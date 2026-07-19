//! Magic-byte format sniffer for retail MIX entries.
//!
//! Identifies leaf-asset formats by structural signatures (no dependency on
//! XCC's filename database). Shared by the audit-assets coverage binary and
//! the retail_goldens certification suite so both walk the identical corpus.
//!
//! ## Dependency rules
//! - Part of assets/ — depends only on assets::mix_archive.

use crate::assets::mix_archive::MixArchive;

/// Detect the format of an asset by magic bytes / structural signatures.
///
/// Returns `None` for things we don't audit (nested MIX, INI text, BIK video,
/// VQA, audio bags, raw binary blobs we don't parse, etc.).
pub fn detect_format(data: &[u8]) -> Option<&'static str> {
    if data.len() < 4 {
        return None;
    }

    // 1. Strong magic matches (unique 4+ byte signatures).
    if &data[0..4] == b" FSC" {
        return Some("csf");
    }
    if &data[0..4] == b"fonT" {
        return Some("fnt");
    }
    if data.len() >= 16 && &data[0..16] == b"Voxel Animation\0" {
        return Some("vxl");
    }
    // BIK video — skip (separate handling).
    if &data[0..3] == b"BIK" {
        return None;
    }
    // VQA — skip.
    if &data[0..4] == b"FORM" {
        return None;
    }
    // PCX: 0x0A magic, version <= 5, RLE encoding flag 1, sane bit depth.
    if data.len() >= 128
        && data[0] == 0x0A
        && data[1] <= 5
        && data[2] == 1
        && matches!(data[3], 1 | 2 | 4 | 8)
    {
        return Some("pcx");
    }

    // 2. Nested MIX archive — skip (it's a container, not a leaf asset).
    if data[0] == 0 && data[1] == 0 && MixArchive::looks_like_mix(data) {
        return None;
    }

    // 3. INI / text — skip.
    if matches!(data[0], b'[' | b';') {
        return None;
    }
    // Plain ASCII letter at start with no nulls in first 32 bytes is probably text.
    if data[0].is_ascii_alphabetic() && data.iter().take(32).all(|b| *b != 0) {
        return None;
    }

    // 4. TMP: tile_width=60, tile_height=30 at offsets 8/12 (RA2 isometric).
    if data.len() >= 16 {
        let tile_w = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let tile_h = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        if tile_w == 60 && tile_h == 30 {
            return Some("tmp");
        }
    }

    // 5. SHP: bytes[0..2] = 0, reasonable width/height/frame_count.
    if data.len() >= 8 && data[0] == 0 && data[1] == 0 {
        let w = u16::from_le_bytes([data[2], data[3]]);
        let h = u16::from_le_bytes([data[4], data[5]]);
        let frames = u16::from_le_bytes([data[6], data[7]]);
        if (1..=2048).contains(&w)
            && (1..=2048).contains(&h)
            && (1..=10000).contains(&frames)
            && data.len() >= 8 + (frames as usize) * 24
        {
            return Some("shp");
        }
    }

    // 6. AUD: byte 11 is the format byte (1 = WS_ADPCM, 99 = IMA_ADPCM).
    //    Also require a reasonable sample rate.
    if data.len() >= 12 {
        let format_byte = data[11];
        let sample_rate = u16::from_le_bytes([data[0], data[1]]);
        if (format_byte == 1 || format_byte == 99)
            && matches!(sample_rate, 8000 | 11025 | 22050 | 44100 | 48000)
        {
            return Some("aud");
        }
    }

    // 7. VPL: header has first_remap, last_remap, num_sections; total size is
    //    16 + 768 (palette) + num_sections * 256.
    if data.len() >= 16 + 768 {
        let first_remap = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let last_remap = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let num_sections = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        if first_remap < 256
            && last_remap < 256
            && first_remap <= last_remap
            && (16..=512).contains(&num_sections)
            && data.len() == 16 + 768 + (num_sections as usize) * 256
        {
            return Some("vpl");
        }
    }

    // 8. PAL: exactly 768 bytes (256 RGB triplets, 6-bit values).
    if data.len() == 768 {
        return Some("pal");
    }

    // 9. HVA: no magic, but offset 16 = frame_count, offset 20 = section_count.
    //    Total size = 24 + section_count*16 + frame_count*section_count*48.
    if data.len() >= 24 {
        let frame_count = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let section_count = u32::from_le_bytes([data[20], data[21], data[22], data[23]]) as usize;
        if frame_count > 0
            && (1..=200).contains(&section_count)
            && frame_count <= 1000
            && data.len() == 24 + section_count * 16 + frame_count * section_count * 48
        {
            return Some("hva");
        }
    }

    None
}
