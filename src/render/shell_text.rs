//! Path A upper wrapper for shell controls. Bit-flag alignment, per-pixel
//! scissor clip, vertical center via measure-then-offset, per-line horizontal
//! alignment, `max_height` cutoff. Calls into `bit_font::BitFont` for glyph
//! data and wrap layout.

use crate::render::batch::SpriteInstance;
use crate::render::bit_font::BitFont;

/// Alignment flag set for `draw_in_rect`.
/// 0x01 = h-center, 0x02 = h-right, 0x04 = v-center.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ShellAlign(pub u8);

impl ShellAlign {
    pub const NONE: ShellAlign = ShellAlign(0);
    pub const H_CENTER: ShellAlign = ShellAlign(0x01);
    pub const H_RIGHT: ShellAlign = ShellAlign(0x02);
    pub const V_CENTER: ShellAlign = ShellAlign(0x04);

    pub fn contains(self, flag: ShellAlign) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl std::ops::BitOr for ShellAlign {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        ShellAlign(self.0 | rhs.0)
    }
}

/// Pixel-coordinate scissor rect. Apply via `wgpu::RenderPass::set_scissor_rect`.
#[derive(Copy, Clone, Debug, Default)]
pub struct ScissorRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Output of `draw_in_rect`: sprite instances plus the scissor the caller
/// must set on its render pass before drawing them.
pub struct ShellTextDraw {
    pub instances: Vec<SpriteInstance>,
    pub scissor: ScissorRect,
}

/// Pixel rect input to `draw_in_rect` -- width/height in screen pixels.
#[derive(Copy, Clone, Debug)]
pub struct TextRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub fn draw_in_rect(
    font: &BitFont,
    text: &str,
    rect: TextRect,
    color: [f32; 3],
    flags: ShellAlign,
    cam_offset: [f32; 2],
    depth: f32,
) -> ShellTextDraw {
    let scissor = ScissorRect {
        x: rect.x.max(0) as u32,
        y: rect.y.max(0) as u32,
        w: rect.w,
        h: rect.h,
    };
    if text.is_empty() {
        return ShellTextDraw {
            instances: Vec::new(),
            scissor,
        };
    }
    let layout = font.wrap_layout(text, rect.w);
    let base_x = rect.x as f32;
    let mut line_y = rect.y as f32;
    if flags.contains(ShellAlign::V_CENTER) && layout.height < rect.h {
        line_y += ((rect.h - layout.height) / 2) as f32;
    }
    let line_advance = font.cell_height();

    let mut instances: Vec<SpriteInstance> = Vec::with_capacity(text.len());
    for span in &layout.lines {
        if (line_y + font.glyph_height()) > (rect.y as f32 + rect.h as f32) {
            break;
        }
        let line_x_offset = if flags.contains(ShellAlign::H_CENTER) && span.width < rect.w {
            ((rect.w - span.width) / 2) as f32
        } else if flags.contains(ShellAlign::H_RIGHT) && span.width < rect.w {
            (rect.w - span.width) as f32
        } else {
            0.0
        };
        let segment = &text[span.start_byte..span.end_byte];
        let mut line_instances = font.build_text(
            segment,
            base_x + line_x_offset,
            line_y,
            1.0,
            depth,
            color,
            cam_offset,
        );
        instances.append(&mut line_instances);
        line_y += line_advance;
    }
    ShellTextDraw { instances, scissor }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::bit_font::tests::make_test_font;

    fn test_font() -> BitFont {
        make_test_font(&[(b'x' as u16, 6), (b'a' as u16, 6), (b'b' as u16, 6)], 4)
    }

    #[test]
    fn scissor_equals_rect() {
        let font = test_font();
        let draw = draw_in_rect(
            &font,
            "x",
            TextRect {
                x: 10,
                y: 20,
                w: 100,
                h: 30,
            },
            [1.0, 1.0, 1.0],
            ShellAlign::NONE,
            [0.0, 0.0],
            0.5,
        );
        assert_eq!(draw.scissor.x, 10);
        assert_eq!(draw.scissor.y, 20);
        assert_eq!(draw.scissor.w, 100);
        assert_eq!(draw.scissor.h, 30);
    }

    #[test]
    fn empty_text_returns_empty_instances() {
        let font = test_font();
        let draw = draw_in_rect(
            &font,
            "",
            TextRect {
                x: 0,
                y: 0,
                w: 100,
                h: 30,
            },
            [1.0, 1.0, 1.0],
            ShellAlign::V_CENTER | ShellAlign::H_CENTER,
            [0.0, 0.0],
            0.5,
        );
        assert!(draw.instances.is_empty());
    }

    #[test]
    fn align_combines_with_bitor() {
        let combined = ShellAlign::H_CENTER | ShellAlign::V_CENTER;
        assert!(combined.contains(ShellAlign::H_CENTER));
        assert!(combined.contains(ShellAlign::V_CENTER));
        assert!(!combined.contains(ShellAlign::H_RIGHT));
    }

    #[test]
    fn vcenter_offsets_correctly() {
        let font = test_font();
        let draw = draw_in_rect(
            &font,
            "x",
            TextRect {
                x: 0,
                y: 0,
                w: 100,
                h: 40,
            },
            [1.0, 1.0, 1.0],
            ShellAlign::V_CENTER,
            [0.0, 0.0],
            0.5,
        );
        assert_eq!(draw.instances.len(), 1);
        let expected_y = ((40 - 17) / 2) as f32;
        assert!(
            (draw.instances[0].position[1] - expected_y).abs() < 0.01,
            "y = {}",
            draw.instances[0].position[1]
        );
    }

    #[test]
    fn align_center_single_line() {
        let font = test_font();
        let draw = draw_in_rect(
            &font,
            "x",
            TextRect {
                x: 0,
                y: 0,
                w: 100,
                h: 30,
            },
            [1.0, 1.0, 1.0],
            ShellAlign::H_CENTER,
            [0.0, 0.0],
            0.5,
        );
        assert_eq!(draw.instances.len(), 1);
        // Single 'x' measured width per gamemd = 6 + 1*char_spacing = 7.
        let expected_x = ((100 - 7) / 2) as f32;
        assert!(
            (draw.instances[0].position[0] - expected_x).abs() < 0.01,
            "x = {}",
            draw.instances[0].position[0]
        );
    }

    #[test]
    fn align_right_single_line() {
        let font = test_font();
        let draw = draw_in_rect(
            &font,
            "x",
            TextRect {
                x: 0,
                y: 0,
                w: 100,
                h: 30,
            },
            [1.0, 1.0, 1.0],
            ShellAlign::H_RIGHT,
            [0.0, 0.0],
            0.5,
        );
        assert_eq!(draw.instances.len(), 1);
        let expected_x = (100 - 7) as f32;
        assert!(
            (draw.instances[0].position[0] - expected_x).abs() < 0.01,
            "x = {}",
            draw.instances[0].position[0]
        );
    }
}
