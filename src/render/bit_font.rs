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

use crate::assets::fnt_file::{FntFile, FntGlyph};
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;

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

    /// Tint adjustment for missing-glyph fallback rendering -- caller XORs
    /// the input tint to produce the visible "wrong color" effect that
    /// distinguishes missing glyphs at a glance. Faithful 32-bit port of the
    /// original RGB565 `color ^= 0x5555`: decomposing 0x5555 into RGB565
    /// component XOR masks gives R5 ^= 0x0A, G6 ^= 0x2A, B5 ^= 0x15.
    pub fn missing_color_xor(rgb: [f32; 3]) -> [f32; 3] {
        fn xor_565(c: f32, bits: u32, mask: u8) -> f32 {
            let max_val = (1u32 << bits) - 1;
            let quantized = ((c.clamp(0.0, 1.0) * max_val as f32) as u32) as u8;
            let flipped = (quantized ^ mask) & (max_val as u8);
            (flipped as f32) / (max_val as f32)
        }
        [
            xor_565(rgb[0], 5, 0x0A),
            xor_565(rgb[1], 6, 0x2A),
            xor_565(rgb[2], 5, 0x15),
        ]
    }

    pub fn fallback_5x7(gpu: &GpuContext, batch: &BatchRenderer) -> Self {
        const GLYPH_W: u32 = 5;
        const GLYPH_H: u32 = 7;
        const GLYPH_PAD: u32 = 1;
        const ATLAS_COLUMNS: usize = 8;

        let supported = fallback_5x7_glyphs();
        let rows = supported.len().div_ceil(ATLAS_COLUMNS);
        let cell_w = GLYPH_W + GLYPH_PAD * 2;
        let cell_h = GLYPH_H + GLYPH_PAD * 2;
        let atlas_w = (ATLAS_COLUMNS as u32) * cell_w;
        let atlas_h = (rows as u32) * cell_h;
        let mut rgba = vec![0u8; (atlas_w * atlas_h * 4) as usize];
        let mut glyphs = HashMap::new();

        for (idx, (ch, bitmap)) in supported.iter().enumerate() {
            let col = (idx % ATLAS_COLUMNS) as u32;
            let row = (idx / ATLAS_COLUMNS) as u32;
            let origin_x = col * cell_w + GLYPH_PAD;
            let origin_y = row * cell_h + GLYPH_PAD;
            write_5x7_glyph_bitmap(&mut rgba, atlas_w, origin_x, origin_y, bitmap);
            glyphs.insert(
                *ch as u16,
                GlyphEntry {
                    uv_origin: [
                        origin_x as f32 / atlas_w as f32,
                        origin_y as f32 / atlas_h as f32,
                    ],
                    uv_size: [
                        GLYPH_W as f32 / atlas_w as f32,
                        GLYPH_H as f32 / atlas_h as f32,
                    ],
                    pixel_width: GLYPH_W as f32,
                },
            );
        }

        Self {
            atlas_texture: Some(batch.create_texture(gpu, &rgba, atlas_w, atlas_h)),
            glyphs,
            missing_glyph: None,
            cell_height: GLYPH_H + 1,
            bitmap_rows: GLYPH_H,
            space_width: GLYPH_W,
            char_spacing: CHAR_SPACING,
            tab_width: TAB_WIDTH,
            tab_origin: TAB_ORIGIN,
            darken_texture: Some(batch.create_texture(gpu, &[0u8, 0, 0, DARKEN_ALPHA], 1, 1)),
        }
    }

    pub fn from_fnt(gpu: &GpuContext, batch: &BatchRenderer, fnt: &FntFile) -> Self {
        let mut entries: Vec<(u16, &FntGlyph)> = Vec::new();
        for cp in PACKED_CODEPOINT_RANGE {
            if let Some(g) = fnt.glyph(cp) {
                entries.push((cp, g));
            }
        }

        // Synthesize the missing-glyph bitmap: inverted '°' (codepoint 0xB0).
        // Source is white-on-transparent; invert RGB on set pixels so the
        // fallback is visually distinct against tinted destinations.
        let missing_owned: Option<FntGlyph> = fnt.glyph(MISSING_GLYPH_CODEPOINT).map(|src| {
            let mut rgba = src.rgba.clone();
            let mut i = 0;
            while i + 3 < rgba.len() {
                let a = rgba[i + 3];
                rgba[i] = !rgba[i] & a;
                rgba[i + 1] = !rgba[i + 1] & a;
                rgba[i + 2] = !rgba[i + 2] & a;
                i += 4;
            }
            FntGlyph {
                width: src.width,
                rgba,
            }
        });

        if entries.is_empty() && missing_owned.is_none() {
            log::warn!("FNT has no glyphs, falling back to hardcoded font");
            return Self::fallback_5x7(gpu, batch);
        }

        // Reserve a sentinel codepoint for the missing glyph that won't collide
        // with the packed range. u16::MAX is well outside 0x20..0x180.
        const MISSING_SENTINEL: u16 = u16::MAX;
        let mut all_entries: Vec<(u16, &FntGlyph)> = Vec::with_capacity(entries.len() + 1);
        for e in &entries {
            all_entries.push((e.0, e.1));
        }
        if let Some(ref g) = missing_owned {
            all_entries.push((MISSING_SENTINEL, g));
        }

        let row_h = fnt.bitmap_rows;
        let pad = 1u32;
        let max_atlas_w = 512u32;

        struct Placement {
            x: u32,
            y: u32,
        }
        let mut placements: Vec<Placement> = Vec::with_capacity(all_entries.len());
        let mut cursor_x = 0u32;
        let mut cursor_y = 0u32;
        let mut atlas_w = 0u32;

        for (_cp, g) in &all_entries {
            let w = g.width + pad * 2;
            if cursor_x + w > max_atlas_w {
                cursor_x = 0;
                cursor_y += row_h + pad * 2;
            }
            placements.push(Placement {
                x: cursor_x + pad,
                y: cursor_y + pad,
            });
            cursor_x += w;
            if cursor_x > atlas_w {
                atlas_w = cursor_x;
            }
        }
        let atlas_h = cursor_y + row_h + pad * 2;

        let mut rgba = vec![0u8; (atlas_w * atlas_h * 4) as usize];
        let mut glyphs = HashMap::new();
        let mut missing_glyph = None;

        for (idx, (cp, g)) in all_entries.iter().enumerate() {
            let pl = &placements[idx];
            for row in 0..row_h {
                for col in 0..g.width {
                    let src = ((row * g.width + col) * 4) as usize;
                    if src + 3 >= g.rgba.len() {
                        continue;
                    }
                    let dst_x = pl.x + col;
                    let dst_y = pl.y + row;
                    let dst = ((dst_y * atlas_w + dst_x) * 4) as usize;
                    rgba[dst..dst + 4].copy_from_slice(&g.rgba[src..src + 4]);
                }
            }
            let entry = GlyphEntry {
                uv_origin: [pl.x as f32 / atlas_w as f32, pl.y as f32 / atlas_h as f32],
                uv_size: [
                    g.width as f32 / atlas_w as f32,
                    row_h as f32 / atlas_h as f32,
                ],
                pixel_width: g.width as f32,
            };
            if *cp == MISSING_SENTINEL {
                missing_glyph = Some(entry);
            } else {
                glyphs.insert(*cp, entry);
            }
        }

        let space_width = fnt.glyph(0x20).map(|g| g.width).unwrap_or(DEFAULT_SPACE_WIDTH);
        let atlas_texture = batch.create_texture(gpu, &rgba, atlas_w, atlas_h);
        let darken_texture = batch.create_texture(gpu, &[0u8, 0, 0, DARKEN_ALPHA], 1, 1);

        log::info!(
            "BitFont atlas: {}x{} px, {} glyphs (+missing={}), space_width={}",
            atlas_w,
            atlas_h,
            glyphs.len(),
            missing_glyph.is_some(),
            space_width
        );

        Self {
            atlas_texture: Some(atlas_texture),
            glyphs,
            missing_glyph,
            cell_height: fnt.cell_height,
            bitmap_rows: fnt.bitmap_rows,
            space_width,
            char_spacing: CHAR_SPACING,
            tab_width: TAB_WIDTH,
            tab_origin: TAB_ORIGIN,
            darken_texture: Some(darken_texture),
        }
    }
}

fn write_5x7_glyph_bitmap(
    rgba: &mut [u8],
    atlas_w: u32,
    origin_x: u32,
    origin_y: u32,
    rows: &[&str; 7],
) {
    for (y, row) in rows.iter().enumerate() {
        for (x, pixel) in row.as_bytes().iter().enumerate() {
            if *pixel != b'#' {
                continue;
            }
            let idx = (((origin_y + y as u32) * atlas_w + (origin_x + x as u32)) * 4) as usize;
            rgba[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
}

fn fallback_5x7_glyphs() -> Vec<(char, [&'static str; 7])> {
    vec![
        (' ', [".....", ".....", ".....", ".....", ".....", ".....", "....."]),
        ('-', [".....", ".....", ".....", ".###.", ".....", ".....", "....."]),
        (':', [".....", "..#..", ".....", ".....", "..#..", ".....", "....."]),
        ('/', ["....#", "...#.", "..#..", ".#...", "#....", ".....", "....."]),
        ('0', ["#####", "#...#", "#...#", "#...#", "#...#", "#...#", "#####"]),
        ('1', ["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."]),
        ('2', ["#####", "....#", "....#", "#####", "#....", "#....", "#####"]),
        ('3', ["#####", "....#", "..##.", "....#", "....#", "....#", "#####"]),
        ('4', ["#...#", "#...#", "#...#", "#####", "....#", "....#", "....#"]),
        ('5', ["#####", "#....", "#....", "#####", "....#", "....#", "#####"]),
        ('6', ["#####", "#....", "#....", "#####", "#...#", "#...#", "#####"]),
        ('7', ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."]),
        ('8', ["#####", "#...#", "#...#", "#####", "#...#", "#...#", "#####"]),
        ('9', ["#####", "#...#", "#...#", "#####", "....#", "....#", "#####"]),
        ('A', [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
        ('B', ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."]),
        ('C', [".####", "#....", "#....", "#....", "#....", "#....", ".####"]),
        ('D', ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."]),
        ('E', ["#####", "#....", "#....", "####.", "#....", "#....", "#####"]),
        ('F', ["#####", "#....", "#....", "####.", "#....", "#....", "#...."]),
        ('G', [".####", "#....", "#....", "#.###", "#...#", "#...#", ".###."]),
        ('H', ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
        ('I', ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####"]),
        ('J', ["#####", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."]),
        ('K', ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"]),
        ('L', ["#....", "#....", "#....", "#....", "#....", "#....", "#####"]),
        ('M', ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"]),
        ('N', ["#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#"]),
        ('O', [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
        ('P', ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."]),
        ('Q', [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"]),
        ('R', ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"]),
        ('S', [".####", "#....", "#....", ".###.", "....#", "....#", "####."]),
        ('T', ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."]),
        ('U', ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
        ('V', ["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."]),
        ('W', ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#"]),
        ('X', ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"]),
        ('Y', ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."]),
        ('Z', ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"]),
        ('a', [".....", ".....", ".###.", "....#", ".####", "#...#", ".####"]),
        ('b', ["#....", "#....", "####.", "#...#", "#...#", "#...#", "####."]),
        ('c', [".....", ".....", ".####", "#....", "#....", "#....", ".####"]),
        ('d', ["....#", "....#", ".####", "#...#", "#...#", "#...#", ".####"]),
        ('e', [".....", ".....", ".###.", "#...#", "#####", "#....", ".###."]),
        ('f', ["..##.", ".#...", ".#...", "####.", ".#...", ".#...", ".#..."]),
        ('g', [".....", ".####", "#...#", "#...#", ".####", "....#", ".###."]),
        ('h', ["#....", "#....", "####.", "#...#", "#...#", "#...#", "#...#"]),
        ('i', ["..#..", ".....", "..#..", "..#..", "..#..", "..#..", "..#.."]),
        ('j', ["...#.", ".....", "...#.", "...#.", "...#.", "#..#.", ".##.."]),
        ('k', ["#....", "#....", "#..#.", "#.#..", "##...", "#.#..", "#..#."]),
        ('l', [".##..", "..#..", "..#..", "..#..", "..#..", "..#..", ".###."]),
        ('m', [".....", ".....", "##.#.", "#.#.#", "#.#.#", "#...#", "#...#"]),
        ('n', [".....", ".....", "####.", "#...#", "#...#", "#...#", "#...#"]),
        ('o', [".....", ".....", ".###.", "#...#", "#...#", "#...#", ".###."]),
        ('p', [".....", "####.", "#...#", "#...#", "####.", "#....", "#...."]),
        ('q', [".....", ".####", "#...#", "#...#", ".####", "....#", "....#"]),
        ('r', [".....", ".....", ".####", "#....", "#....", "#....", "#...."]),
        ('s', [".....", ".....", ".####", "#....", ".###.", "....#", "####."]),
        ('t', [".#...", ".#...", "####.", ".#...", ".#...", ".#...", "..##."]),
        ('u', [".....", ".....", "#...#", "#...#", "#...#", "#...#", ".####"]),
        ('v', [".....", ".....", "#...#", "#...#", "#...#", ".#.#.", "..#.."]),
        ('w', [".....", ".....", "#...#", "#...#", "#.#.#", "#.#.#", ".#.#."]),
        ('x', [".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#"]),
        ('y', [".....", "#...#", "#...#", ".####", "....#", "...#.", ".##.."]),
        ('z', [".....", ".....", "#####", "...#.", "..#..", ".#...", "#####"]),
    ]
}

