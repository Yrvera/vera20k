//! Parser for `[Preview]` and `[PreviewPack]` map sections.
//!
//! This records preview metadata and decodes the packed thumbnail pixels used
//! by the skirmish shell on demand. GPU upload remains in the app/render layer.

use std::fmt;

use crate::rules::ini_parser::IniFile;
use crate::util::base64::base64_decode;
use crate::util::lzo::{LzoError, decompress_chunks};

/// CPU RGBA thumbnail decoded from `[PreviewPack]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPreview {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Parsed preview-related metadata from a map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewSection {
    /// Size metadata from `[Preview] Size=` when present.
    pub size: Option<(u32, u32)>,
    /// True if `[PreviewPack]` exists and contains data.
    pub has_packed_preview: bool,
    /// Decoded CPU-side preview thumbnail when the map carries a valid pack.
    pub decoded: Option<DecodedPreview>,
}

/// Projected start point from `[Header] WaypointN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewStartPoint {
    pub x: i32,
    pub y: i32,
}

/// Source rectangle inputs needed by the original preview marker projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSourceBounds {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
    pub start_points: Vec<PreviewStartPoint>,
}

/// Errors produced while decoding `[PreviewPack]` image data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewDecodeError {
    MissingPreviewPack,
    InvalidBase64(String),
    Lzo(String),
    PixelByteCount { expected: usize, actual: usize },
    PixelBufferTooLarge,
}

impl fmt::Display for PreviewDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPreviewPack => write!(f, "missing or empty [PreviewPack]"),
            Self::InvalidBase64(err) => write!(f, "invalid PreviewPack base64: {err}"),
            Self::Lzo(err) => write!(f, "invalid PreviewPack LZO stream: {err}"),
            Self::PixelByteCount { expected, actual } => {
                write!(
                    f,
                    "PreviewPack byte count {actual} did not match expected {expected}"
                )
            }
            Self::PixelBufferTooLarge => write!(f, "PreviewPack dimensions overflowed"),
        }
    }
}

impl std::error::Error for PreviewDecodeError {}

impl From<LzoError> for PreviewDecodeError {
    fn from(value: LzoError) -> Self {
        Self::Lzo(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewChannelOrder {
    Rgb,
    #[allow(dead_code)]
    Bgr,
}

const PREVIEW_CHANNEL_ORDER: PreviewChannelOrder = PreviewChannelOrder::Rgb;

/// Parse preview metadata from map INI sections.
pub fn parse_preview_section(ini: &IniFile) -> PreviewSection {
    let size = ini
        .section("Preview")
        .and_then(|section| section.get("Size"))
        .and_then(parse_preview_size);

    let has_packed_preview = preview_pack_text(ini).is_some();

    PreviewSection {
        size,
        has_packed_preview,
        decoded: None,
    }
}

fn parse_preview_size(value: &str) -> Option<(u32, u32)> {
    let parts: Vec<u32> = value
        .split(',')
        .map(str::trim)
        .map(str::parse::<u32>)
        .collect::<Result<Vec<u32>, _>>()
        .ok()?;
    match parts.as_slice() {
        [width, height] => Some((*width, *height)),
        [_, _, width, height, ..] => Some((*width, *height)),
        _ => None,
    }
}

fn preview_pack_text(ini: &IniFile) -> Option<String> {
    let section = ini.section("PreviewPack")?;
    let values = section.get_values();
    if values.iter().all(|value| value.trim().is_empty()) {
        return None;
    }
    Some(values.concat())
}

fn expected_preview_rgb_len(width: u32, height: u32) -> Result<usize, PreviewDecodeError> {
    let pixels = width
        .checked_mul(height)
        .ok_or(PreviewDecodeError::PixelBufferTooLarge)?;
    let bytes = pixels
        .checked_mul(3)
        .ok_or(PreviewDecodeError::PixelBufferTooLarge)?;
    usize::try_from(bytes).map_err(|_| PreviewDecodeError::PixelBufferTooLarge)
}

fn expected_preview_rgba_len(width: u32, height: u32) -> Result<usize, PreviewDecodeError> {
    let pixels = width
        .checked_mul(height)
        .ok_or(PreviewDecodeError::PixelBufferTooLarge)?;
    let bytes = pixels
        .checked_mul(4)
        .ok_or(PreviewDecodeError::PixelBufferTooLarge)?;
    usize::try_from(bytes).map_err(|_| PreviewDecodeError::PixelBufferTooLarge)
}

fn push_rgba_from_preview_pixel(out: &mut Vec<u8>, pixel: &[u8]) {
    match PREVIEW_CHANNEL_ORDER {
        PreviewChannelOrder::Rgb => {
            out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
        }
        PreviewChannelOrder::Bgr => {
            out.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
        }
    }
}

/// Decode row-major 3-byte `[PreviewPack]` pixels into RGBA.
pub fn decode_preview_pack(
    pack_text: &str,
    width: u32,
    height: u32,
) -> Result<DecodedPreview, PreviewDecodeError> {
    let encoded = pack_text.trim();
    if encoded.is_empty() {
        return Err(PreviewDecodeError::MissingPreviewPack);
    }

    let compressed = base64_decode(encoded).map_err(PreviewDecodeError::InvalidBase64)?;
    let rgb = decompress_chunks(&compressed)?;
    let expected = expected_preview_rgb_len(width, height)?;
    if rgb.len() != expected {
        return Err(PreviewDecodeError::PixelByteCount {
            expected,
            actual: rgb.len(),
        });
    }

    let mut rgba = Vec::with_capacity(expected_preview_rgba_len(width, height)?);
    for pixel in rgb.chunks_exact(3) {
        push_rgba_from_preview_pixel(&mut rgba, pixel);
    }

    Ok(DecodedPreview {
        width,
        height,
        rgba,
    })
}

/// Decode the preview image from a full map INI.
///
/// This is intentionally separate from `parse_preview_section` so menu map
/// discovery can stay cheap even when the RA2 directory has many custom maps.
pub fn decode_preview_image_from_ini(
    ini: &IniFile,
) -> Result<Option<DecodedPreview>, PreviewDecodeError> {
    let Some((width, height)) = ini
        .section("Preview")
        .and_then(|section| section.get("Size"))
        .and_then(parse_preview_size)
    else {
        return Ok(None);
    };
    let Some(pack_text) = preview_pack_text(ini) else {
        return Ok(None);
    };
    decode_preview_pack(&pack_text, width, height).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn parse_preview_metadata() {
        let ini = IniFile::from_str("[Preview]\nSize=80,50\n[PreviewPack]\n1=ABC\n2=DEF\n");
        let preview = parse_preview_section(&ini);
        assert_eq!(preview.size, Some((80, 50)));
        assert!(preview.has_packed_preview);
        assert_eq!(preview.decoded, None);
    }

    #[test]
    fn empty_preview_pack_is_not_counted() {
        let ini = IniFile::from_str("[PreviewPack]\n1=\n");
        let preview = parse_preview_section(&ini);
        assert_eq!(preview.size, None);
        assert!(!preview.has_packed_preview);
        assert_eq!(preview.decoded, None);
    }

    #[test]
    fn parse_preview_size_uses_rect_dimensions() {
        let ini = IniFile::from_str("[Preview]\nSize=0,0,138,75\n");
        let preview = parse_preview_section(&ini);
        assert_eq!(preview.size, Some((138, 75)));
    }

    #[test]
    fn parse_preview_size_rejects_single_value() {
        let ini = IniFile::from_str("[Preview]\nSize=138\n");
        let preview = parse_preview_section(&ini);
        assert_eq!(preview.size, None);
    }

    #[test]
    fn preview_pack_text_uses_numeric_key_order() {
        let ini = IniFile::from_str("[PreviewPack]\n2=BBB\n10=CCC\n1=AAA\n");
        assert_eq!(preview_pack_text(&ini).as_deref(), Some("AAABBBCCC"));
    }

    #[test]
    fn preview_pack_text_rejects_empty_numbered_values() {
        let ini = IniFile::from_str("[PreviewPack]\n1=\n2=\n");
        assert_eq!(preview_pack_text(&ini), None);
    }

    #[test]
    fn decode_preview_pack_literal_chunk_to_rgba() {
        let preview = decode_preview_pack("CgAGABcBAgMEBQYRAAA=", 2, 1).expect("valid preview");
        assert_eq!(preview.width, 2);
        assert_eq!(preview.height, 1);
        assert_eq!(preview.rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn decode_preview_pack_rejects_wrong_byte_count() {
        let err = decode_preview_pack("CgAGABcBAgMEBQYRAAA=", 1, 1).unwrap_err();
        assert_eq!(
            err,
            PreviewDecodeError::PixelByteCount {
                expected: 3,
                actual: 6,
            }
        );
    }

    #[test]
    fn parse_preview_section_leaves_valid_pack_lazy() {
        let ini =
            IniFile::from_str("[Preview]\nSize=0,0,2,1\n[PreviewPack]\n1=CgAGABcBAgMEBQYRAAA=\n");
        let preview = parse_preview_section(&ini);
        assert_eq!(preview.size, Some((2, 1)));
        assert!(preview.has_packed_preview);
        assert_eq!(preview.decoded, None);
    }

    #[test]
    fn decode_preview_image_from_ini_decodes_valid_pack() {
        let ini =
            IniFile::from_str("[Preview]\nSize=0,0,2,1\n[PreviewPack]\n1=CgAGABcBAgMEBQYRAAA=\n");
        let decoded = decode_preview_image_from_ini(&ini)
            .expect("valid preview decode")
            .expect("decoded preview");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn parse_preview_section_keeps_invalid_pack_nonfatal() {
        let ini = IniFile::from_str("[Preview]\nSize=2,1\n[PreviewPack]\n1=not valid base64!\n");
        let preview = parse_preview_section(&ini);
        assert_eq!(preview.size, Some((2, 1)));
        assert!(preview.has_packed_preview);
        assert_eq!(preview.decoded, None);
    }
}
