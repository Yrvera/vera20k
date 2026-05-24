//! Minimal PCX parser for RA2 shell owner-draw art.
//!
//! Supports the retail 8-bit, one-plane, RLE-compressed PCX files used by
//! shell controls, plus the runtime-written 3-plane direct RGB preview form.
//! The parser keeps embedded VGA palettes in 8-bit RGB.

use crate::assets::error::AssetError;

#[derive(Debug, Clone)]
pub struct PcxFile {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
    pub palette: [[u8; 3]; 256],
    direct_rgb: bool,
}

impl PcxFile {
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 128 {
            return Err(pcx_error("PCX too short"));
        }
        if data[0] != 0x0A || data[2] != 1 || data[3] != 8 {
            return Err(pcx_error("Unsupported PCX header"));
        }
        let x_min = u16::from_le_bytes([data[4], data[5]]);
        let y_min = u16::from_le_bytes([data[6], data[7]]);
        let x_max = u16::from_le_bytes([data[8], data[9]]);
        let y_max = u16::from_le_bytes([data[10], data[11]]);
        let planes = data[65];
        if planes != 1 && planes != 3 {
            return Err(pcx_error("Only 1-plane or 3-plane PCX is supported"));
        }
        let bytes_per_line = u16::from_le_bytes([data[66], data[67]]) as usize;
        let width = x_max
            .checked_sub(x_min)
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| pcx_error("Invalid PCX width"))?;
        let height = y_max
            .checked_sub(y_min)
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| pcx_error("Invalid PCX height"))?;
        if bytes_per_line < width as usize {
            return Err(pcx_error("Invalid PCX bytes per line"));
        }

        let expected_scan = bytes_per_line
            .checked_mul(planes as usize)
            .and_then(|v| v.checked_mul(height as usize))
            .ok_or_else(|| pcx_error("PCX scan data too large"))?;
        let mut palette = [[0u8; 3]; 256];
        let has_trailing_palette = data.len() >= 128 + 769 && data[data.len() - 769] == 0x0C;
        let encoded = if planes == 1 {
            if !has_trailing_palette {
                return Err(pcx_error("Missing PCX VGA palette"));
            }
            let pal = &data[data.len() - 768..];
            for (idx, rgb) in palette.iter_mut().enumerate() {
                rgb.copy_from_slice(&pal[idx * 3..idx * 3 + 3]);
            }
            &data[128..data.len() - 769]
        } else if has_trailing_palette {
            &data[128..data.len() - 769]
        } else {
            &data[128..]
        };
        let scan = decode_pcx_rle(encoded, expected_scan)?;
        let pixels = if planes == 1 {
            trim_paletted_scanlines(&scan, width, height, bytes_per_line)
        } else {
            decode_direct_rgb_scanlines(&scan, width, height, bytes_per_line)
        };

        Ok(Self {
            width,
            height,
            pixels,
            palette,
            direct_rgb: planes == 3,
        })
    }

    pub fn to_rgba(&self, transparent_index: Option<u8>) -> Vec<u8> {
        if self.is_direct_rgb() {
            let mut rgba = Vec::with_capacity(self.pixels.len() / 3 * 4);
            for rgb in self.pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
            return rgba;
        }

        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for &idx in &self.pixels {
            let [r, g, b] = self.palette[idx as usize];
            let a = if transparent_index == Some(idx) {
                0
            } else {
                255
            };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
        rgba
    }

    pub fn to_rgba_with_color_key(&self, transparent_rgb: [u8; 3]) -> Vec<u8> {
        if self.is_direct_rgb() {
            let mut rgba = Vec::with_capacity(self.pixels.len() / 3 * 4);
            for rgb in self.pixels.chunks_exact(3) {
                let a = if rgb == transparent_rgb { 0 } else { 255 };
                rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
            }
            return rgba;
        }

        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for &idx in &self.pixels {
            let [r, g, b] = self.palette[idx as usize];
            let a = if [r, g, b] == transparent_rgb { 0 } else { 255 };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
        rgba
    }

    fn is_direct_rgb(&self) -> bool {
        self.direct_rgb
    }
}

fn decode_pcx_rle(encoded: &[u8], expected_scan: usize) -> Result<Vec<u8>, AssetError> {
    let mut scan = Vec::with_capacity(expected_scan);
    let mut i = 0usize;
    while i < encoded.len() && scan.len() < expected_scan {
        let byte = encoded[i];
        i += 1;
        if byte & 0xC0 == 0xC0 {
            if i >= encoded.len() {
                return Err(pcx_error("Truncated PCX RLE run"));
            }
            let count = (byte & 0x3F) as usize;
            let value = encoded[i];
            i += 1;
            scan.extend(std::iter::repeat(value).take(count));
        } else {
            scan.push(byte);
        }
    }
    if scan.len() < expected_scan {
        return Err(pcx_error("PCX RLE stream ended early"));
    }
    scan.truncate(expected_scan);
    Ok(scan)
}

fn trim_paletted_scanlines(scan: &[u8], width: u16, height: u16, bytes_per_line: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for row in 0..height as usize {
        let start = row * bytes_per_line;
        pixels.extend_from_slice(&scan[start..start + width as usize]);
    }
    pixels
}

fn decode_direct_rgb_scanlines(
    scan: &[u8],
    width: u16,
    height: u16,
    bytes_per_line: usize,
) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let row_stride = bytes_per_line * 3;
    let mut pixels = Vec::with_capacity(width * height * 3);
    for row in 0..height {
        let row_start = row * row_stride;
        let red_start = row_start;
        let green_start = red_start + bytes_per_line;
        let blue_start = green_start + bytes_per_line;
        for col in 0..width {
            pixels.extend_from_slice(&[
                scan[red_start + col],
                scan[green_start + col],
                scan[blue_start + col],
            ]);
        }
    }
    pixels
}

fn pcx_error(detail: &str) -> AssetError {
    AssetError::ParseError {
        format: "PCX".to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::PcxFile;

    fn pcx_header(
        x_min: u16,
        y_min: u16,
        x_max: u16,
        y_max: u16,
        planes: u8,
        bytes_per_line: u16,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 128];
        data[0] = 0x0A;
        data[2] = 1;
        data[3] = 8;
        data[4..6].copy_from_slice(&x_min.to_le_bytes());
        data[6..8].copy_from_slice(&y_min.to_le_bytes());
        data[8..10].copy_from_slice(&x_max.to_le_bytes());
        data[10..12].copy_from_slice(&y_max.to_le_bytes());
        data[65] = planes;
        data[66..68].copy_from_slice(&bytes_per_line.to_le_bytes());
        data
    }

    #[test]
    fn parses_8bit_rle_pcx_with_embedded_palette() {
        let mut data = pcx_header(0, 0, 1, 1, 1, 2);
        data.extend_from_slice(&[0xC4, 1]);
        data.push(0x0C);
        let mut pal = vec![0u8; 768];
        pal[3] = 10;
        pal[4] = 20;
        pal[5] = 30;
        data.extend_from_slice(&pal);

        let pcx = PcxFile::from_bytes(&data).expect("pcx");
        assert_eq!((pcx.width, pcx.height), (2, 2));
        assert_eq!(pcx.pixels, vec![1, 1, 1, 1]);
        assert_eq!(pcx.palette[1], [10, 20, 30]);
        assert_eq!(pcx.to_rgba(None)[0..4], [10, 20, 30, 255]);
    }

    #[test]
    fn parses_3plane_direct_rgb_pcx_with_bounds_dimensions() {
        let mut data = pcx_header(10, 20, 11, 21, 3, 4);
        data.extend_from_slice(&[
            1, 2, 99, 99, // row 0 red plane plus padding
            10, 20, 99, 99, // row 0 green plane plus padding
            30, 40, 99, 99, // row 0 blue plane plus padding
            3, 4, 99, 99, // row 1 red plane plus padding
            50, 60, 99, 99, // row 1 green plane plus padding
            70, 80, 99, 99, // row 1 blue plane plus padding
        ]);

        let pcx = PcxFile::from_bytes(&data).expect("pcx");

        assert_eq!((pcx.width, pcx.height), (2, 2));
        assert_eq!(
            pcx.pixels,
            vec![
                1, 10, 30, //
                2, 20, 40, //
                3, 50, 70, //
                4, 60, 80,
            ]
        );
        assert_eq!(
            pcx.to_rgba(None),
            vec![
                1, 10, 30, 255, //
                2, 20, 40, 255, //
                3, 50, 70, 255, //
                4, 60, 80, 255,
            ]
        );
    }

    #[test]
    fn direct_rgb_color_key_applies_after_rgb_conversion() {
        let mut data = pcx_header(0, 0, 1, 0, 3, 2);
        data.extend_from_slice(&[
            0xC1, 255, 1, // red
            0, 2, // green
            0xC1, 255, 3, // blue
        ]);

        let pcx = PcxFile::from_bytes(&data).expect("pcx");

        assert_eq!(
            pcx.to_rgba_with_color_key([255, 0, 255]),
            vec![
                255, 0, 255, 0, //
                1, 2, 3, 255,
            ]
        );
    }

    #[test]
    fn rgba_color_key_applies_after_embedded_palette_conversion() {
        let pcx = PcxFile {
            width: 3,
            height: 1,
            pixels: vec![1, 2, 3],
            palette: {
                let mut palette = [[0u8; 3]; 256];
                palette[1] = [255, 0, 255];
                palette[2] = [0, 0, 0];
                palette[3] = [255, 0, 255];
                palette
            },
            direct_rgb: false,
        };

        assert_eq!(
            pcx.to_rgba_with_color_key([255, 0, 255]),
            vec![
                255, 0, 255, 0, //
                0, 0, 0, 255, //
                255, 0, 255, 0,
            ]
        );
    }
}
