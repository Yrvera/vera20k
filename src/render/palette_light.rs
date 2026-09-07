//! Reference for how the original lights a palette entry, kept next to the
//! shaders that emulate it (`palette_light` in `batch_shader.wgsl`,
//! `sprite_voxel_shader.wgsl`, `zdepth_shader.wgsl`).
//!
//! `LightConvertClass::Constructor` (0x00555DA0) -> `FUN_00556090` builds the
//! per-format table through `FUN_007DE200` (RGB565) and siblings: for every
//! palette entry, each 8-bit channel is multiplied by a 16.16 scale
//! `scale16 = ftol(light_milli * 1000 * 0.065536)` (three `LEA x5` and a `SHL 3`
//! at 0x00556192..0x0055619B, then the double 0.065536 at 0x007ED0B0), i.e.
//! `light_milli * 65536 / 1000` so neutral 1000 is exactly 0x10000. It is
//! shifted right by 16, clamped to 255 when the product overflows a byte, then
//! packed to the display format. The multiply is on the palette bytes, so it
//! is a gamma-space scale.
//!
//! How a brightness such as Ambient 1.0 + ExtraUnitLight 0.2 reaches that
//! multiply is by row selection, not by passing 1200: the constructor builds
//! N rows (0x35 = 53, or 0x1B = 27 when r+g+b < 2000, per 0x00544E70) with
//! `scale_k = k * 2 * scale16 / (N - 1)`, the colour key itself is clamped to
//! 0..1000 by 0x00555AC0, and the draw picks a row for its brightness. So 1.2
//! quantises to row 31 (1.192) or 32 (1.231) of a 53-row table; the row pick
//! is UNCHECKED here. The shaders apply the exact product instead of a row
//! (DRIFT: 3.85% brightness steps, 7.7% on 27-row tables) and do not model
//! the RGB565 packing that follows (DRIFT: up to 3 bits per channel).

/// Native 16.16 scale for a light value in milliunits (1000 = neutral).
pub fn native_scale16(light_milli: i32) -> i64 {
    // ftol truncates toward zero under gamemd's chop control word.
    ((light_milli as f64) * 1000.0 * 0.065536) as i64
}

/// One palette byte lit the native way: `(byte * scale16) >> 16`, clamped to 255.
pub fn native_lit_byte(byte: u8, light_milli: i32) -> u8 {
    let product = byte as i64 * native_scale16(light_milli);
    if product > 0x00FF_FFFF {
        255
    } else {
        (product >> 16) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_light_is_identity() {
        for b in [0u8, 1, 52, 116, 200, 255] {
            assert_eq!(native_lit_byte(b, 1000), b);
        }
    }

    #[test]
    fn table_scale_is_a_byte_multiply() {
        // Native table arithmetic for a 1.2 scale (what the shader now applies;
        // the native row pick would land on 1.192 or 1.231, see the module doc).
        // Retail Iron Gull container top: unittem index 106 red byte 116 reads
        // ~139-144 in a gamemd capture at Ambient 1.0 + ExtraUnitLight 0.2.
        assert_eq!(native_lit_byte(116, 1200), 139);
        // Hull grey 52 (index 56) -> 62.
        assert_eq!(native_lit_byte(52, 1200), 62);
        // A linear-space multiply would give only ~1.09x on these bytes.
    }

    #[test]
    fn overflow_clamps_to_white() {
        assert_eq!(native_lit_byte(255, 1200), 255);
        assert_eq!(native_lit_byte(220, 2000), 255);
    }

    #[test]
    fn shaders_declare_the_same_helper() {
        for src in [
            include_str!("batch_shader.wgsl"),
            include_str!("sprite_voxel_shader.wgsl"),
            include_str!("zdepth_shader.wgsl"),
        ] {
            assert!(src.contains("fn palette_light(rgb_linear: vec3f, tint: vec3f)"));
            assert!(
                src.contains("palette_light(")
                    && !src.contains("rgb * in.tint")
                    && !src.contains("color.rgb * input.tint")
            );
        }
    }
}
