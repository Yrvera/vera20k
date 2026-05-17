//! Path B upper wrapper for sidebar text. Single-line emission with optional
//! selected-unit fade (first N characters tinted from side-highlight color
//! toward the base text color). Side-color highlight table per side
//! (Allied / Soviet / Yuri).
//!
//! Most sidebar callers use the plain pass-through fns; only the Ready cameo
//! text needs `build_text_with_fade`.

use crate::render::batch::{BatchTexture, SpriteInstance};
use crate::render::bit_font::BitFont;
use crate::render::sidebar_chrome::SidebarTheme;

/// Side highlight colors used as fade endpoint for selected-unit text effect.
const HIGHLIGHT_ALLIED: [f32; 3] = [164.0 / 255.0, 210.0 / 255.0, 1.0];
const HIGHLIGHT_SOVIET: [f32; 3] = [1.0, 1.0, 0.0];
const HIGHLIGHT_YURI: [f32; 3] = [1.0, 1.0, 0.0];

pub fn side_highlight_color(theme: SidebarTheme) -> [f32; 3] {
    match theme {
        SidebarTheme::Allied => HIGHLIGHT_ALLIED,
        SidebarTheme::Soviet => HIGHLIGHT_SOVIET,
        SidebarTheme::Yuri => HIGHLIGHT_YURI,
    }
}

// --- Plain pass-throughs preserved for existing single-color callers ---

pub fn text_width(font: &BitFont, text: &str) -> f32 {
    font.text_width(text) as f32
}

pub fn glyph_height(font: &BitFont) -> f32 {
    font.glyph_height()
}

pub fn darken_texture(font: &BitFont) -> &BatchTexture {
    font.darken_texture()
}

pub fn texture(font: &BitFont) -> &BatchTexture {
    font.atlas()
}

pub fn build_text(
    font: &BitFont,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    depth: f32,
    tint: [f32; 3],
    camera_offset: [f32; 2],
) -> Vec<SpriteInstance> {
    font.build_text(text, x, y, scale, depth, tint, camera_offset)
}

/// Selected-unit fade. First `fade_param` characters (capped at 8) tint from
/// `side_highlight` toward `base_color`; subsequent characters use
/// `base_color`. `fade_param == 0` => no fade (equivalent to `build_text`).
pub fn build_text_with_fade(
    font: &BitFont,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    depth: f32,
    base_color: [f32; 3],
    side_highlight: [f32; 3],
    fade_param: u32,
    camera_offset: [f32; 2],
) -> Vec<SpriteInstance> {
    if fade_param == 0 {
        return font.build_text(text, x, y, scale, depth, base_color, camera_offset);
    }
    let chars_to_fade = fade_param.min(8);
    let mut line_offset: u32 = 9u32.saturating_sub(fade_param) * 0x1F;
    let mut out: Vec<SpriteInstance> = Vec::with_capacity(text.len());
    let mut cursor_x = x;
    let spacing = scale; // CHAR_SPACING = 1
    let mut emitted = 0u32;

    for (char_idx, ch) in text.chars().enumerate() {
        if ch == '\r' || ch == '\n' {
            continue;
        }
        let tint = if (char_idx as u32) < chars_to_fade {
            // Fade from highlight back to the normal text color: line_offset
            // starts small ((9-fade_param)*0x1F) and grows by 0x1F per char,
            // so early chars are near highlight, later chars near base.
            let t = (line_offset.min(255) as f32) / 255.0;
            lerp_rgb(side_highlight, base_color, t)
        } else {
            base_color
        };
        if (char_idx as u32) < chars_to_fade {
            line_offset = line_offset.saturating_add(0x1F);
        }
        if ch == ' ' {
            if emitted > 0 {
                cursor_x += spacing;
            }
            cursor_x += font.text_width(" ") as f32 * scale;
            emitted += 1;
            continue;
        }
        let mut single = font.build_text(
            &ch.to_string(),
            cursor_x,
            y,
            scale,
            depth,
            tint,
            camera_offset,
        );
        if let Some(inst) = single.first() {
            let w = inst.size[0];
            if emitted > 0 {
                for s in &mut single {
                    s.position[0] += spacing;
                }
                cursor_x += spacing;
            }
            cursor_x += w;
        }
        out.append(&mut single);
        emitted += 1;
    }
    out
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
