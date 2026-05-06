//! Paradrop Drop_Payload — V-pattern math + per-tick passenger ejection.
//!
//! Each Drop_Payload call ejects one passenger from the carrier's cargo at
//! a 128-lepton offset perpendicular to flight heading. Drops alternate
//! left/right by post-decrement payload-count parity. With initial count=8
//! the visible drop sequence is L, R, L, R, L, R, L, R (first drop LEFT).
//!
//! The 0x3FFF binary-angle quarter-circle in the original collapses to
//! a 64-step facing offset under our 256-facing convention (0x3FFF/0xFFFF
//! ≈ 0.25, and 64/256 = 0.25). The existing 256-entry SIN_TABLE/COS_TABLE
//! in util/facing_table covers all the trig.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on util/facing_table, util/fixed_math.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::util::facing_table::facing_to_movement;
use crate::util::fixed_math::{SimFixed, sim_to_i32};

/// V-pattern lateral radius. From gamemd constant at 0x7E2808 = 128.0 leptons
/// (= 0.5 cell). Each paratrooper lands half a cell to the left or right of
/// the plane's center.
pub const V_PATTERN_RADIUS_LEPTONS: i32 = 128;

/// Reset value for the LandingState mutex (gamemd `aircraft+0x6D3`).
/// Decremented per tick; gates back-to-back drops within 5 ticks of each other.
/// With ROF=130 ticks this is mostly a safety net but is preserved for parity.
pub const LANDING_STATE_RESET: u8 = 5;

/// Default drop interval in ticks if `[ParaDropWeapon] ROF=` lookup fails.
/// Matches the vanilla rulesmd value.
pub const PARADROP_DROP_INTERVAL_TICKS: u16 = 130;

/// Compute the V-pattern lateral offset for the next drop, in leptons.
///
/// `facing`: aircraft body facing 0..=255 (RA2 convention: 0=N, 64=E, 128=S, 192=W).
/// `payload_count_post_dec`: payload count AFTER decrement (matches gamemd's order).
///
/// Returns `(dx, dy)` in leptons. EVEN parity → CW 90° from heading (RIGHT);
/// ODD parity → CCW 90° from heading (LEFT). With initial count=8 the
/// post-decrement sequence 7,6,5,4,3,2,1,0 produces drop sides L,R,L,R,L,R,L,R.
pub fn v_offset(facing: u8, payload_count_post_dec: u8) -> (i32, i32) {
    let drop_facing = if (payload_count_post_dec & 1) == 0 {
        facing.wrapping_add(64) // EVEN → CW 90° (RIGHT of heading)
    } else {
        facing.wrapping_sub(64) // ODD  → CCW 90° (LEFT of heading)
    };
    let radius = SimFixed::from_num(V_PATTERN_RADIUS_LEPTONS);
    let (dx, dy) = facing_to_movement(drop_facing, radius);
    (sim_to_i32(dx), sim_to_i32(dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn magnitude_sq(dx: i32, dy: i32) -> i64 {
        (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64)
    }

    #[test]
    fn test_v_pattern_radius_is_128_for_all_facings() {
        // Magnitude of (dx, dy) should be ~128 leptons regardless of facing.
        // sin/cos LUT is exact at multiples of 64 (cardinal facings) and accurate
        // to <1 lepton elsewhere.
        for facing in 0..=255u8 {
            let (dx, dy) = v_offset(facing, 0); // EVEN parity (RIGHT)
            let mag_sq = magnitude_sq(dx, dy);
            let expected_sq = (V_PATTERN_RADIUS_LEPTONS as i64).pow(2);
            // Allow ±2 leptons of error (LUT discretization at 256 facings).
            let tolerance: i64 = 2 * (V_PATTERN_RADIUS_LEPTONS as i64) * 2 + 4;
            assert!(
                (mag_sq - expected_sq).abs() < tolerance,
                "facing={} produced offset ({},{}), mag²={}, expected ~{}",
                facing, dx, dy, mag_sq, expected_sq,
            );
        }
    }

    #[test]
    fn test_v_pattern_alternates_starting_left() {
        // gamemd: with initial count=8, post-decrement sequence is 7,6,5,4,3,2,1,0.
        // Parity: 7→ODD→LEFT, 6→EVEN→RIGHT, 5→ODD→LEFT, ...
        // Visible drop sequence = L, R, L, R, L, R, L, R (first drop LEFT).
        let facing = 0u8; // North → LEFT = -X (west), RIGHT = +X (east)
        let (dx_first, _) = v_offset(facing, 7); // first drop, payload_post=7 ODD
        let (dx_second, _) = v_offset(facing, 6); // second drop, payload_post=6 EVEN
        assert!(
            dx_first < 0,
            "first drop (count=7, ODD) should be LEFT (-X), got dx={}",
            dx_first,
        );
        assert!(
            dx_second > 0,
            "second drop (count=6, EVEN) should be RIGHT (+X), got dx={}",
            dx_second,
        );
    }

    #[test]
    fn test_v_pattern_facing_north_right_is_east() {
        // Facing 0 (North): RIGHT 90° → facing 64 (East) → +X direction.
        let (dx, dy) = v_offset(0, 0); // EVEN → RIGHT
        assert!(dx > 100, "North-RIGHT should give +X, got dx={}", dx);
        assert!(
            dy.abs() < 30,
            "North-RIGHT should have ~zero Y, got dy={}",
            dy,
        );
    }

    #[test]
    fn test_v_pattern_facing_east_right_is_south() {
        // Facing 64 (East): RIGHT 90° → facing 128 (South) → +Y direction.
        let (dx, dy) = v_offset(64, 0); // EVEN → RIGHT
        assert!(dy > 100, "East-RIGHT should give +Y, got dy={}", dy);
        assert!(
            dx.abs() < 30,
            "East-RIGHT should have ~zero X, got dx={}",
            dx,
        );
    }

    #[test]
    fn test_v_pattern_facing_north_left_is_west() {
        // Facing 0 (North): LEFT 90° → facing 192 (West) → -X direction.
        let (dx, dy) = v_offset(0, 1); // ODD → LEFT
        assert!(dx < -100, "North-LEFT should give -X, got dx={}", dx);
        assert!(
            dy.abs() < 30,
            "North-LEFT should have ~zero Y, got dy={}",
            dy,
        );
    }

    #[test]
    fn test_v_pattern_facing_south_alternates_correctly() {
        // Facing 128 (South): LEFT = facing 64 (East, +X), RIGHT = facing 192 (West, -X).
        let (dx_left, _) = v_offset(128, 1); // ODD → LEFT
        let (dx_right, _) = v_offset(128, 0); // EVEN → RIGHT
        assert!(dx_left > 100, "South-LEFT should be +X (East), got {}", dx_left);
        assert!(dx_right < -100, "South-RIGHT should be -X (West), got {}", dx_right);
    }
}
