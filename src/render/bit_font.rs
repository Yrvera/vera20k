//! Lower-layer bitmap font: atlas, glyph table, measurement, wrap state
//! machine, missing-glyph fallback. Owned by `AppState.bit_font` and shared
//! by `render::shell_text` (Path A) and `render::sidebar_text` (Path B).
//!
//! Glyph data comes from a parsed [`crate::assets::fnt_file::FntFile`]
//! (GAME.FNT). Falls back to a hardcoded 5x7 path when the FNT is unavailable.
//!
//! Public surface:
//!   - [`BitFont::from_fnt`] / [`BitFont::fallback_5x7`] constructors
//!   - [`BitFont::text_width`] / [`BitFont::wrap_layout`] for measurement
//!   - [`BitFont::build_text`] for sprite-instance emission
//!   - [`BitFont::missing_color_xor`] for caller-side tint adjustment

use std::collections::HashMap;

use crate::render::batch::BatchTexture;

/// Hardcoded inter-glyph spacing.
pub const CHAR_SPACING: u32 = 1;
/// Tab stop width in pixels.
pub const TAB_WIDTH: u32 = 64;
/// Tab origin -- subtracted from x before `% TAB_WIDTH`.
pub const TAB_ORIGIN: u32 = 0;
/// Cell height for GAME.FNT (line advance = bitmap_rows + 1px gap).
pub const CELL_HEIGHT: u32 = 17;
/// Bitmap rows per glyph for GAME.FNT.
pub const BITMAP_ROWS: u32 = 16;
/// Source codepoint for the missing-glyph fallback (CP1252 'deg').
pub const MISSING_GLYPH_CODEPOINT: u16 = 0xB0;
/// Darken-strip alpha for sidebar Ready overlay.
pub const DARKEN_ALPHA: u8 = 175;
/// Default fallback space width when FNT lacks a glyph at 0x20 (defensive).
const DEFAULT_SPACE_WIDTH: u32 = 4;
/// Codepoint range packed into the atlas (ASCII + Latin-1 + Latin Extended-A).
const PACKED_CODEPOINT_RANGE: std::ops::Range<u16> = 0x20..0x0180;

/// UV + pixel-width record for a single glyph in the atlas.
#[derive(Clone, Copy, Debug)]
pub struct GlyphEntry {
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_width: f32,
}

/// One line in a wrap layout -- half-open byte range into the source string.
#[derive(Clone, Copy, Debug)]
pub struct LineSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub width: u32,
}

/// Result of [`BitFont::wrap_layout`] -- total bounds + per-line spans.
#[derive(Clone, Debug, Default)]
pub struct WrapLayout {
    pub width: u32,
    pub height: u32,
    pub lines: Vec<LineSpan>,
}

/// Atlas-backed bitmap font + measurement + missing-glyph fallback.
///
/// Texture fields are `Option<BatchTexture>` so pure-measurement tests can
/// construct a `BitFont` without a GPU context (`atlas`/`darken_texture`
/// accessors `expect` Some -- production callers always populate via
/// `from_fnt`/`fallback_5x7`).
pub struct BitFont {
    pub(crate) atlas_texture: Option<BatchTexture>,
    pub(crate) glyphs: HashMap<u16, GlyphEntry>,
    pub(crate) missing_glyph: Option<GlyphEntry>,
    pub(crate) cell_height: u32,
    pub(crate) bitmap_rows: u32,
    pub(crate) space_width: u32,
    pub(crate) char_spacing: u32,
    pub(crate) tab_width: u32,
    pub(crate) tab_origin: u32,
    pub(crate) darken_texture: Option<BatchTexture>,
}

impl BitFont {
    pub fn atlas(&self) -> &BatchTexture {
        self.atlas_texture
            .as_ref()
            .expect("BitFont atlas not populated (test-only ctor)")
    }
    pub fn darken_texture(&self) -> &BatchTexture {
        self.darken_texture
            .as_ref()
            .expect("BitFont darken_texture not populated (test-only ctor)")
    }
    pub fn glyph_height(&self) -> f32 {
        self.bitmap_rows as f32
    }
    pub fn cell_height(&self) -> f32 {
        self.cell_height as f32
    }
}

