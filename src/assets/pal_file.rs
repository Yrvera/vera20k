//! Parser for RA2 .pal palette files.
//!
//! A .pal file is exactly 768 bytes: 256 entries of 3 bytes each (R, G, B).
//! Color values are in the VGA 6-bit range (0–63). gamemd decodes each component
//! with `raw << 2`, so the maximum stored RGBA component is 252.
//!
//! ## House Color Remapping
//! Palette indices 16–31 (16 entries) are reserved for "house colors" — the player's
//! team color. When rendering a unit, these 16 indices get replaced with the owning
//! player's color scheme. This is how RA2 distinguishes player units visually.
//!
//! ## Index 0 = Transparent
//! By convention, palette index 0 is fully transparent. Sprites use index 0
//! for pixels that should show the background behind them.
//!
//! ## Dependency rules
//! - Part of assets/ — no dependencies on game modules.

use std::path::Path;

use crate::assets::error::AssetError;

/// Number of colors in an RA2 palette (always 256 — one per possible byte value).
const PALETTE_COLOR_COUNT: usize = 256;

/// Size of a .pal file in bytes: 256 colors * 3 bytes (R, G, B) each.
const PAL_FILE_SIZE: usize = PALETTE_COLOR_COUNT * 3;

/// First palette index reserved for house (player) colors.
const HOUSE_COLOR_START: usize = 16;

/// Number of palette indices used for house colors (16 through 31 inclusive).
const HOUSE_COLOR_COUNT: usize = 16;

/// A single RGBA color with 8-bit channels.
///
/// Stored as RGBA (not just RGB) because index 0 needs alpha=0 for transparency,
/// and GPU textures expect 4 bytes per pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create a fully opaque color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a fully transparent color (used for palette index 0).
    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
}

/// A 256-color palette loaded from a .pal file.
///
/// Colors are decoded from VGA 6-bit components with the native left shift.
/// Index 0 is always transparent. Indices 16–31 are house colors
/// that can be remapped per player.
#[derive(Debug, Clone)]
pub struct Palette {
    /// The 256 colors. Index 0 has alpha=0 (transparent).
    pub colors: [Color; PALETTE_COLOR_COUNT],
}

impl Palette {
    /// Parse a palette from raw bytes.
    ///
    /// Input must be exactly 768 bytes (256 colors * 3 bytes RGB).
    /// Each color component is in VGA 6-bit range (0–63) and is decoded as
    /// `raw << 2`.
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() != PAL_FILE_SIZE {
            return Err(AssetError::InvalidPalSize {
                expected: PAL_FILE_SIZE,
                actual: data.len(),
            });
        }

        // Start with all-black, fully opaque colors.
        let mut colors: [Color; PALETTE_COLOR_COUNT] = [Color::rgb(0, 0, 0); PALETTE_COLOR_COUNT];

        for (i, color) in colors.iter_mut().enumerate() {
            let base: usize = i * 3;
            let raw = [data[base], data[base + 1], data[base + 2]];
            let [r, g, b] = decode_gamemd_rgb(raw);
            // Index 0 is transparent by convention in all Westwood games.
            // The renderer adapter also treats the exact raw magenta triplet as a
            // chroma key; checking before conversion preserves that alpha policy.
            let is_transparent: bool = i == 0 || raw == [63, 0, 63];
            *color = Color {
                r,
                g,
                b,
                a: if is_transparent { 0 } else { 255 },
            };
        }

        Ok(Palette { colors })
    }

    /// Parse a palette using gamemd's UI/loading conversion.
    ///
    /// The native UI path shifts 6-bit VGA components left by two bits, so the
    /// maximum component `63` becomes `252`. This conversion does not assign
    /// transparency or chroma-key alpha; callers that render SHP frames must apply
    /// any frame-specific transparency policy separately.
    pub fn from_bytes_gamemd_ui(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() != PAL_FILE_SIZE {
            return Err(AssetError::InvalidPalSize {
                expected: PAL_FILE_SIZE,
                actual: data.len(),
            });
        }

        let mut colors: [Color; PALETTE_COLOR_COUNT] = [Color::rgb(0, 0, 0); PALETTE_COLOR_COUNT];

        for (i, color) in colors.iter_mut().enumerate() {
            let base: usize = i * 3;
            let [r, g, b] = decode_gamemd_rgb([data[base], data[base + 1], data[base + 2]]);
            *color = Color::rgb(r, g, b);
        }

        Ok(Palette { colors })
    }

    /// Load a palette from a .pal file on disk.
    ///
    /// This is a convenience wrapper around from_bytes() for loading loose files.
    /// In production, palettes are extracted from .mix archives and parsed via from_bytes().
    pub fn load(path: &Path) -> Result<Self, AssetError> {
        let data: Vec<u8> = std::fs::read(path)?;
        Self::from_bytes(&data)
    }

    /// Create a copy of this palette with house colors (indices 16–31) replaced.
    ///
    /// Each player has a unique set of 16 colors. When rendering a unit owned by
    /// that player, we swap palette indices 16–31 with the player's house colors.
    /// This is how RA2 makes allied units blue, soviet units red, etc.
    pub fn with_house_colors(&self, house_colors: &[Color; HOUSE_COLOR_COUNT]) -> Palette {
        let mut remapped: Palette = self.clone();
        remapped.colors[HOUSE_COLOR_START..HOUSE_COLOR_START + HOUSE_COLOR_COUNT]
            .copy_from_slice(house_colors);
        remapped
    }

    /// Convert this palette's colors to a flat RGBA byte array (1024 bytes).
    ///
    /// Useful for uploading the palette as a 256x1 GPU texture.
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(PALETTE_COLOR_COUNT * 4);
        for color in &self.colors {
            bytes.push(color.r);
            bytes.push(color.g);
            bytes.push(color.b);
            bytes.push(color.a);
        }
        bytes
    }
}

fn decode_gamemd_rgb(raw: [u8; 3]) -> [u8; 3] {
    [raw[0] << 2, raw[1] << 2, raw[2] << 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pal_file_gamemd_shift_left_two_maps_63_to_252() {
        let mut data: Vec<u8> = vec![0u8; PAL_FILE_SIZE];
        data[3] = 63;
        data[4] = 31;
        data[5] = 1;

        let pal: Palette = Palette::from_bytes_gamemd_ui(&data).expect("Should parse");

        assert_eq!(pal.colors[1].r, 252);
        assert_eq!(pal.colors[1].g, 124);
        assert_eq!(pal.colors[1].b, 4);
    }

    #[test]
    fn pal_file_gamemd_ui_does_not_assign_alpha_in_palette_conversion() {
        let mut data: Vec<u8> = vec![0u8; PAL_FILE_SIZE];
        data[0] = 63;
        data[1] = 0;
        data[2] = 63;

        let pal: Palette = Palette::from_bytes_gamemd_ui(&data).expect("Should parse");

        assert_eq!(pal.colors[0], Color::rgb(252, 0, 252));
        assert_eq!(pal.colors[0].a, 255);
    }

    #[test]
    fn gsi_02_13_general_palette_uses_native_component_shift() {
        let mut data: Vec<u8> = vec![0u8; PAL_FILE_SIZE];
        data[3] = 63;
        data[4] = 31;
        data[5] = 1;

        let pal: Palette = Palette::from_bytes(&data).expect("Should parse valid palette");

        assert_eq!(pal.colors[0].a, 0);
        assert_eq!(pal.colors[1].r, 252);
        assert_eq!(pal.colors[1].g, 124);
        assert_eq!(pal.colors[1].b, 4);
        assert_eq!(pal.colors[1].a, 255);
    }

    #[test]
    fn gsi_02_13_general_palette_preserves_raw_magenta_chroma_alpha() {
        let mut data: Vec<u8> = vec![0u8; PAL_FILE_SIZE];
        data[3..6].copy_from_slice(&[63, 0, 63]);

        let pal: Palette = Palette::from_bytes(&data).expect("Should parse valid palette");

        assert_eq!(
            pal.colors[1],
            Color {
                r: 252,
                g: 0,
                b: 252,
                a: 0,
            }
        );
    }

    #[test]
    fn test_reject_wrong_size() {
        let data: Vec<u8> = vec![0u8; 100]; // Too small
        let result = Palette::from_bytes(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_rgba_bytes_length() {
        let data: Vec<u8> = vec![0u8; PAL_FILE_SIZE];
        let pal: Palette = Palette::from_bytes(&data).expect("Should parse");
        let rgba: Vec<u8> = pal.to_rgba_bytes();
        // 256 colors * 4 bytes (RGBA) each = 1024 bytes
        assert_eq!(rgba.len(), 1024);
    }

    #[test]
    fn test_house_color_remap() {
        let data: Vec<u8> = vec![0u8; PAL_FILE_SIZE];
        let pal: Palette = Palette::from_bytes(&data).expect("Should parse");

        // Create red house colors.
        let red_house: [Color; HOUSE_COLOR_COUNT] = [Color::rgb(255, 0, 0); HOUSE_COLOR_COUNT];
        let remapped: Palette = pal.with_house_colors(&red_house);

        // Indices 16–31 should now be red.
        for i in HOUSE_COLOR_START..(HOUSE_COLOR_START + HOUSE_COLOR_COUNT) {
            assert_eq!(remapped.colors[i].r, 255);
            assert_eq!(remapped.colors[i].g, 0);
            assert_eq!(remapped.colors[i].b, 0);
        }

        // Other indices should be unchanged (still black from the zero input).
        assert_eq!(remapped.colors[0].r, 0);
        assert_eq!(remapped.colors[32].r, 0);
    }
}
