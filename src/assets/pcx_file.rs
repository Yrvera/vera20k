//! Minimal PCX parser for RA2 shell owner-draw art.
//!
//! Supports the retail 8-bit, one-plane, RLE-compressed PCX files used by
//! shell controls. The parser keeps embedded VGA palettes in 8-bit RGB.

use crate::assets::error::AssetError;

#[derive(Debug, Clone)]
pub struct PcxFile {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
    pub palette: [[u8; 3]; 256],
}

impl PcxFile {
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 128 + 769 {
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
        if planes != 1 {
            return Err(pcx_error("Only 1-plane PCX is supported"));
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
        if data[data.len() - 769] != 0x0C {
            return Err(pcx_error("Missing PCX VGA palette"));
        }

        let expected_scan = bytes_per_line * height as usize;
        let encoded = &data[128..data.len() - 769];
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

        let mut pixels = Vec::with_capacity(width as usize * height as usize);
        for row in 0..height as usize {
            let start = row * bytes_per_line;
            pixels.extend_from_slice(&scan[start..start + width as usize]);
        }

        let mut palette = [[0u8; 3]; 256];
        let pal = &data[data.len() - 768..];
        for (idx, rgb) in palette.iter_mut().enumerate() {
            rgb.copy_from_slice(&pal[idx * 3..idx * 3 + 3]);
        }

        Ok(Self {
            width,
            height,
            pixels,
            palette,
        })
    }

    pub fn to_rgba(&self, transparent_index: Option<u8>) -> Vec<u8> {
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

    #[test]
    fn parses_8bit_rle_pcx_with_embedded_palette() {
        let mut data = vec![0u8; 128];
        data[0] = 0x0A;
        data[2] = 1;
        data[3] = 8;
        data[8..10].copy_from_slice(&1u16.to_le_bytes());
        data[10..12].copy_from_slice(&1u16.to_le_bytes());
        data[65] = 1;
        data[66..68].copy_from_slice(&2u16.to_le_bytes());
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
}
