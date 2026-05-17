//! FLH (Forward/Lateral/Height) to screen-space transform.
//!
//! Converts FLH lepton offsets into isometric screen-space pixel offsets.
//! The forward/lateral components use the normal ground isometric projection.
//! The height component follows gamemd's `Tactical__AdjustForZ` screen lift.
//!
//! ## Math
//! 1. Rotate the (Forward, Lateral) vector by the facing angle:
//!    - RA2 facing: 0=N, 64=E, 128=S, 192=W
//!    - angle = TAU * (facing / 256.0)
//!    - world_x = Forward * sin(angle) + Lateral * cos(angle)
//!    - world_y = -Forward * cos(angle) + Lateral * sin(angle)
//! 2. Convert world leptons to isometric screen pixels:
//!    - screen_x = (world_x - world_y) * 30.0 / 256.0
//!    - screen_y = (world_x + world_y) * 15.0 / 256.0 - AdjustForZ(Height)
//!
//! ## Dependency rules
//! - Part of util/; no dependencies on other game modules.

/// Pixels per lepton along the isometric X axis (half tile width / leptons per cell).
/// 60px tile width / 2 / 256 leptons = 30/256.
const SCREEN_X_PER_LEPTON: f32 = 30.0 / 256.0;

/// Pixels per lepton along the isometric Y axis (half tile height / leptons per cell).
/// 30px tile height / 2 / 256 leptons = 15/256.
const SCREEN_Y_PER_LEPTON: f32 = 15.0 / 256.0;

/// Height-to-screen multiplier used by gamemd's `Tactical__AdjustForZ`.
///
/// The binary computes approximately:
/// `ftol(z * 0.14348 + (z >= 728 ? 1 : 0) + 0.5)`.
const ADJUST_FOR_Z_MULTIPLIER: f32 = 0.14348;
const ADJUST_FOR_Z_THRESHOLD_LEPTONS: i32 = 728;

/// Convert world Z leptons into gamemd-style screen-Y lift.
pub fn adjust_for_z_leptons(z: i32) -> i32 {
    let threshold_extra = if z >= ADJUST_FOR_Z_THRESHOLD_LEPTONS {
        1.0
    } else {
        0.0
    };
    (z as f32 * ADJUST_FOR_Z_MULTIPLIER + threshold_extra + 0.5).trunc() as i32
}

/// Convert an FLH lepton offset into an isometric screen-space pixel offset.
///
/// `forward`: distance along the unit's facing direction (positive = forward).
/// `lateral`: distance perpendicular to facing (positive = right of facing).
/// `height`: vertical offset (positive = up, produces negative screen Y).
/// `facing`: RA2 facing byte (0=N, 64=E, 128=S, 192=W).
///
/// Returns `(screen_dx, screen_dy)` in pixels, relative to the unit's center.
pub fn flh_to_screen_offset(forward: i32, lateral: i32, height: i32, facing: u8) -> (f32, f32) {
    if forward == 0 && lateral == 0 && height == 0 {
        return (0.0, 0.0);
    }

    // Convert facing (0-255) to radians.
    let angle: f32 = std::f32::consts::TAU * (facing as f32 / 256.0);
    let (sin, cos) = angle.sin_cos();

    let f: f32 = forward as f32;
    let l: f32 = lateral as f32;

    // Rotate (Forward, Lateral) by facing angle into world-space leptons.
    // Forward aligns with the facing direction (sin for X, -cos for Y).
    // Lateral is perpendicular (cos for X, sin for Y).
    let world_x: f32 = f * sin + l * cos;
    let world_y: f32 = -f * cos + l * sin;

    let screen_x: f32 = (world_x - world_y) * SCREEN_X_PER_LEPTON;
    let screen_y: f32 =
        (world_x + world_y) * SCREEN_Y_PER_LEPTON - adjust_for_z_leptons(height) as f32;

    (screen_x, screen_y)
}

/// Convert FLH using the 32-way facing quantization used by gamemd's fire-origin path.
pub fn flh_to_screen_offset_32way(
    forward: i32,
    lateral: i32,
    height: i32,
    facing: u8,
) -> (f32, f32) {
    if forward == 0 && lateral == 0 && height == 0 {
        return (0.0, 0.0);
    }
    let facing_16: u16 = (facing as u16) << 8;
    let bucket: i16 = ((((facing_16 >> 10) + 1) >> 1) & 0x1f) as i16 - 8;
    let quantized_facing: u8 = (((bucket + 8) as u16 * 8) & 0xff) as u8;
    flh_to_screen_offset(forward, lateral, height, quantized_facing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_flh_returns_zero() {
        let (sx, sy) = flh_to_screen_offset(0, 0, 0, 0);
        assert!((sx).abs() < 0.001);
        assert!((sy).abs() < 0.001);
    }

    #[test]
    fn test_forward_only_facing_north() {
        let (sx, sy) = flh_to_screen_offset(100, 0, 0, 0);
        assert!((sx - 11.72).abs() < 0.1, "sx={}", sx);
        assert!((sy - (-5.86)).abs() < 0.1, "sy={}", sy);
    }

    #[test]
    fn test_forward_matches_turret_screen_offset_pattern() {
        let (sx, sy) = flh_to_screen_offset(100, 0, 0, 64);
        assert!((sx - 11.72).abs() < 0.1, "sx={}", sx);
        assert!((sy - 5.86).abs() < 0.1, "sy={}", sy);
    }

    #[test]
    fn test_facing_south_reverses_forward() {
        let (sx, sy) = flh_to_screen_offset(100, 0, 0, 128);
        assert!((sx - (-11.72)).abs() < 0.1, "sx={}", sx);
        assert!((sy - 5.86).abs() < 0.1, "sy={}", sy);
    }

    #[test]
    fn test_lateral_only_facing_north() {
        let (sx, sy) = flh_to_screen_offset(0, 50, 0, 0);
        assert!((sx - 5.86).abs() < 0.1, "sx={}", sx);
        assert!((sy - 2.93).abs() < 0.1, "sy={}", sy);
    }

    #[test]
    fn test_height_only_produces_adjust_for_z_offset() {
        let (sx, sy) = flh_to_screen_offset(0, 0, 100, 0);
        assert!((sx).abs() < 0.001, "sx={}", sx);
        assert!((sy - (-14.0)).abs() < 0.1, "sy={}", sy);
    }

    #[test]
    fn test_combined_flh_east_facing() {
        let (sx, sy) = flh_to_screen_offset(150, 0, 100, 64);
        assert!((sx - 17.58).abs() < 0.1, "sx={}", sx);
        assert!((sy - (-5.21)).abs() < 0.1, "sy={}", sy);
    }

    #[test]
    fn test_negative_lateral() {
        let (sx, _sy) = flh_to_screen_offset(0, -50, 0, 0);
        assert!((sx - (-5.86)).abs() < 0.1, "sx={}", sx);
    }

    #[test]
    fn adjust_for_z_matches_gi_fire_flh_heights() {
        assert_eq!(adjust_for_z_leptons(90), 13);
        assert_eq!(adjust_for_z_leptons(105), 15);
    }

    #[test]
    fn flh_32way_quantizes_small_facing_changes_to_same_offset() {
        let a = flh_to_screen_offset_32way(80, 0, 105, 0);
        let b = flh_to_screen_offset_32way(80, 0, 105, 3);
        assert_eq!(a, b);
    }

    #[test]
    fn flh_32way_changes_after_bucket_boundary() {
        let a = flh_to_screen_offset_32way(80, 0, 105, 0);
        let b = flh_to_screen_offset_32way(80, 0, 105, 8);
        assert_ne!(a, b);
    }

    #[test]
    fn flh_32way_preserves_zero_flh() {
        assert_eq!(flh_to_screen_offset_32way(0, 0, 0, 123), (0.0, 0.0));
    }
}
