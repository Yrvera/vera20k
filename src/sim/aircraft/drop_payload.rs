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

use crate::rules::ruleset::RuleSet;
use crate::sim::movement::parachute_descent::begin_parachute_descent;
use crate::sim::passenger::PassengerRole;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::facing_table::facing_to_movement;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_to_i32};

/// V-pattern lateral radius. From gamemd constant at 0x7E2808 = 128.0 leptons
/// (= 0.5 cell). Each paratrooper lands half a cell to the left or right of
/// the plane's center.
pub const V_PATTERN_RADIUS_LEPTONS: i32 = 128;

/// Reset value for the LandingState mutex (gamemd `aircraft+0x6D3`).
/// Decremented per tick; gates back-to-back drops.
pub const LANDING_STATE_RESET: u8 = 5;

/// Drop interval in sim ticks between consecutive drops.
///
/// Hardcoded in gamemd's `Mission_Rescue` (0x00415960): every code path returns
/// 5, meaning the rescue mission re-fires every 5 game frames while in range
/// and drops one passenger per call. This is NOT driven by `[ParaDropWeapon]
/// ROF=` (that weapon is a dummy — its rules.ini comment says so).
///
/// Our sim runs at 45 Hz vs gamemd's 15 fps, so 5 game frames = 15 sim ticks.
pub const PARADROP_DROP_INTERVAL_TICKS: u16 = 15;

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

/// Outcome of a single Drop_Payload attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropResult {
    /// Passenger placed, parachute descent attached. Caller resets cooldown
    /// to ROF, sets landing_state=5, decrements payload_count.
    Success,
    /// Drop cell impassable. Passenger was re-inserted at cargo HEAD; caller
    /// leaves drop_cooldown unchanged so we retry next tick.
    ImpassableRetry,
    /// begin_parachute_descent returned false (entity missing or attach failed).
    /// Same retry semantics as ImpassableRetry.
    AttachFailedRetry,
    /// Cargo was empty (caller should have gated on cargo_empty already).
    NoCargo,
}

/// Attempt to drop one passenger from the carrier aircraft's cargo.
///
/// Pre-conditions (caller-enforced):
///   - aircraft entity exists and has PassengerRole::Transport with non-empty cargo
///   - drop_cooldown == 0 && landing_state == 0
///
/// `path_grid`: Some when threaded from advance_tick; None in headless tests
/// (passability defaults to "always passable" in that case).
/// `rules`: needed to look up passenger ObjectType.size for cargo accounting.
pub fn try_drop(
    sim: &mut Simulation,
    rules: &RuleSet,
    aircraft_id: u64,
    payload_count_pre_dec: u8,
    path_grid: Option<&PathGrid>,
) -> DropResult {
    // 1. Snapshot aircraft state (release borrow before mutating).
    // Capture the aircraft's full lepton position (cell + sub-cell) so the
    // V-pattern offset can apply at lepton precision. With cell-only math the
    // ±128 lateral offset truncates to 0 and every drop lands on the same cell.
    let (facing, altitude, aircraft_x_lep, aircraft_y_lep) = match sim.entities.get(aircraft_id) {
        Some(a) => {
            let alt = a.locomotor.as_ref().map(|l| l.altitude).unwrap_or(SIM_ZERO);
            let x_lep = a.position.rx as i32 * 256 + sim_to_i32(a.position.sub_x);
            let y_lep = a.position.ry as i32 * 256 + sim_to_i32(a.position.sub_y);
            (a.facing, alt, x_lep, y_lep)
        }
        None => return DropResult::NoCargo,
    };

    // 2. Pop FIFO passenger from cargo.
    let passenger_id = match sim
        .entities
        .get_mut(aircraft_id)
        .and_then(|a| a.passenger_role.cargo_mut())
        .and_then(|c| c.unload_first())
    {
        Some(id) => id,
        None => return DropResult::NoCargo,
    };

    // Look up passenger size now (needed to correct cargo.total_size on success).
    // PassengerCargo::unload_first does NOT decrement total_size — caller's job.
    let pax_size: u32 = sim
        .entities
        .get(passenger_id)
        .and_then(|p| {
            let type_str = sim.interner.resolve(p.type_ref);
            rules.object(type_str).map(|o| o.size)
        })
        .unwrap_or(1);

    // 3. Compute V-offset in leptons, then split into (cell, sub-cell).
    // Using `div_euclid`/`rem_euclid` so negative offsets cross cell
    // boundaries correctly (left-side drops walk one cell west when the
    // aircraft is in the western half of its cell).
    let payload_count_post = payload_count_pre_dec.saturating_sub(1);
    let (dx, dy) = v_offset(facing, payload_count_post);
    let drop_x_lep = aircraft_x_lep + dx;
    let drop_y_lep = aircraft_y_lep + dy;
    let drop_rx = drop_x_lep.div_euclid(256).clamp(0, u16::MAX as i32) as u16;
    let drop_ry = drop_y_lep.div_euclid(256).clamp(0, u16::MAX as i32) as u16;
    let drop_sub_x = SimFixed::from_num(drop_x_lep.rem_euclid(256));
    let drop_sub_y = SimFixed::from_num(drop_y_lep.rem_euclid(256));

    // 4. Passability check via threaded path_grid.
    let passable = path_grid.map_or(true, |g| g.is_walkable(drop_rx, drop_ry));
    if !passable {
        if let Some(cargo) = sim
            .entities
            .get_mut(aircraft_id)
            .and_then(|a| a.passenger_role.cargo_mut())
        {
            cargo.passengers.insert(0, passenger_id);
        }
        return DropResult::ImpassableRetry;
    }

    // 5. Position passenger at drop cell; un-limbo. Do NOT touch
    // `loco.altitude` here — `begin_parachute_descent` uses `OverrideKind`
    // to gate the descent and tracks altitude in `ParachuteDescentState`,
    // and the renderer reads loco.altitude as a visual offset. If we set
    // loco.altitude=plane_altitude here, it gets snapshotted by
    // `begin_override` and `end_override` restores it on landing — the
    // unit then renders ~3 tiles up indefinitely.
    if let Some(passenger) = sim.entities.get_mut(passenger_id) {
        passenger.position.rx = drop_rx;
        passenger.position.ry = drop_ry;
        passenger.position.sub_x = drop_sub_x;
        passenger.position.sub_y = drop_sub_y;
        // Update cached screen coords now so the first frame of descent
        // doesn't briefly render the GI at the carrier's old position.
        passenger.position.refresh_screen_coords();
        passenger.passenger_role = PassengerRole::None;
    }

    // 6. Attach parachute descent.
    if !begin_parachute_descent(&mut sim.entities, passenger_id, altitude) {
        // L17 deviation: revert passenger_role and re-insert at cargo HEAD; retry.
        if let Some(passenger) = sim.entities.get_mut(passenger_id) {
            passenger.passenger_role = PassengerRole::Inside {
                transport_id: aircraft_id,
            };
        }
        if let Some(cargo) = sim
            .entities
            .get_mut(aircraft_id)
            .and_then(|a| a.passenger_role.cargo_mut())
        {
            cargo.passengers.insert(0, passenger_id);
        }
        return DropResult::AttachFailedRetry;
    }

    // 7. ChuteSound at drop cell.
    sim.sound_events.push(SimSoundEvent::ChuteSound {
        rx: drop_rx,
        ry: drop_ry,
    });

    // 8. Decrement cargo.total_size on success (unload_first left it stale).
    if let Some(cargo) = sim
        .entities
        .get_mut(aircraft_id)
        .and_then(|a| a.passenger_role.cargo_mut())
    {
        cargo.total_size = cargo.total_size.saturating_sub(pax_size);
    }

    DropResult::Success
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
                facing,
                dx,
                dy,
                mag_sq,
                expected_sq,
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
        assert!(
            dx_left > 100,
            "South-LEFT should be +X (East), got {}",
            dx_left
        );
        assert!(
            dx_right < -100,
            "South-RIGHT should be -X (West), got {}",
            dx_right
        );
    }
}
