//! Movement step helpers — cell transition mechanics, vehicle rotation, lepton advancement,
//! and cell boundary crossing detection.
//!
//! Contains the inner-loop logic extracted from `tick_movement_with_grids`: how a mover
//! rotates in place, advances sub-cell position, detects cell boundary crossings, and
//! performs the actual cell transition with occupancy/terrain checks.

use std::collections::BTreeSet;

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::{
    DriveCoord, DriveLocomotionRuntime, DriveOccupationFootprint, MovementTarget, Position,
    ShipLocomotionRuntime,
};
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::movement::bump_crush;
use crate::sim::movement::drive_locomotion as drive_locomotion_helpers;
use crate::sim::movement::drive_track::{self, DriveTrackState};
use crate::sim::movement::locomotor::{GroundMovePhase, LocomotorState, MovementLayer};
use crate::sim::movement::movement_blocked::handle_blocked_tick;
use crate::sim::movement::movement_bridge::resolve_cell_transition_bridge_state;
use crate::sim::movement::movement_occupancy::{
    DeferredCellCheck, LiveBuildingEntrySkipMap, detect_deferred_cell_check,
    evaluate_runtime_can_enter_cell, naval_terrain_diag, runtime_can_enter_cell_args,
};
use crate::sim::movement::movement_reservation::reserve_destination_after_transition;
use crate::sim::occupancy::{CellListInsertion, CellOccupationGrid, OccupancyGrid};
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::rng::SimRng;
use crate::sim::world::EnterOrderCounter;
use crate::util::fixed_math::{
    SIM_HALF, SIM_ONE, SIM_ZERO, SimFixed, facing_from_delta_int as facing_from_delta,
    fixed_distance,
};
use crate::util::lepton::CELL_CENTER_LEPTON;

use super::{
    CLIFF_HEIGHT_THRESHOLD, MovementConfig, MovementTickStats, MoverSnapshot, PATH_STUCK_INIT,
    PathfindingContext,
};

fn shared_track_kind(locomotor: &Option<LocomotorState>) -> Option<LocomotorKind> {
    locomotor
        .as_ref()
        .map(|locomotor| locomotor.kind)
        .filter(|kind| matches!(kind, LocomotorKind::Drive | LocomotorKind::Ship))
}

fn resolved_track_endpoint(
    path_grid: Option<&PathGrid>,
    cell: (u16, u16),
    layer: MovementLayer,
    fallback_z: u8,
) -> DriveCoord {
    let z = path_grid
        .and_then(|grid| grid.cell(cell.0, cell.1))
        .map_or(fallback_z, |path_cell| {
            path_cell.effective_cell_z_for_layer(layer)
        });
    DriveCoord::cell(cell.0, cell.1, i32::from(z as i8))
}

#[allow(clippy::too_many_arguments)]
fn accept_shared_track(
    kind: LocomotorKind,
    drive_locomotion: &mut Option<DriveLocomotionRuntime>,
    ship_locomotion: &mut Option<ShipLocomotionRuntime>,
    endpoint: (i16, i16),
    endpoint_coord: DriveCoord,
    consumed_directions: usize,
) {
    match kind {
        LocomotorKind::Drive => {
            if let Some(drive) = drive_locomotion.as_mut() {
                super::path_markers::accept_path_replay(
                    &mut drive.path,
                    endpoint,
                    consumed_directions,
                );
            }
        }
        LocomotorKind::Ship => {
            if let Some(ship) = ship_locomotion.as_mut() {
                super::path_markers::accept_path_replay(
                    &mut ship.path,
                    endpoint,
                    consumed_directions,
                );
                ship.head_to = Some(endpoint_coord);
            }
        }
        _ => {}
    }
}

pub(super) fn movement_frame_budget_from_current_speed(current_speed_per_second: SimFixed) -> i32 {
    (current_speed_per_second / SimFixed::from_num(15u8)).to_num::<i32>()
}

fn scaled_frame_delta(
    direction_component: SimFixed,
    direction_length: SimFixed,
    frame_budget: i32,
) -> SimFixed {
    if direction_component == SIM_ZERO || direction_length <= SIM_ZERO || frame_budget == 0 {
        return SIM_ZERO;
    }

    const FRACTION_SCALE: i64 = 1 << 16;
    let numerator =
        i64::from(direction_component.to_bits()) * i64::from(frame_budget) * FRACTION_SCALE;
    let delta_bits = numerator / i64::from(direction_length.to_bits());
    SimFixed::from_bits(
        i32::try_from(delta_bits).expect("movement component exceeds SimFixed range"),
    )
}

fn whole_lepton_subcell_after_delta(cell: u16, subcell: SimFixed, delta: SimFixed) -> SimFixed {
    const FRACTION_SCALE: i64 = 1 << 16;
    const LEPTONS_PER_CELL: i64 = 256;
    let cell_origin = i64::from(cell) * LEPTONS_PER_CELL;
    let absolute_bits =
        cell_origin * FRACTION_SCALE + i64::from(subcell.to_bits()) + i64::from(delta.to_bits());
    let absolute_leptons = absolute_bits / FRACTION_SCALE;
    let subcell_leptons = absolute_leptons - cell_origin;
    SimFixed::from_num(
        i32::try_from(subcell_leptons).expect("sub-cell movement exceeds SimFixed range"),
    )
}

fn advance_by_frame_budget(target: &MovementTarget, position: &mut Position, frame_budget: i32) {
    let delta_x = scaled_frame_delta(target.move_dir_x, target.move_dir_len, frame_budget);
    let delta_y = scaled_frame_delta(target.move_dir_y, target.move_dir_len, frame_budget);
    position.sub_x = whole_lepton_subcell_after_delta(position.rx, position.sub_x, delta_x);
    position.sub_y = whole_lepton_subcell_after_delta(position.ry, position.sub_y, delta_y);
}

fn advance_straight_position(
    target: &MovementTarget,
    position: &mut Position,
    frame_budget: i32,
    fraction: SimFixed,
    whole_lepton_result: bool,
) {
    if whole_lepton_result {
        advance_by_frame_budget(target, position, frame_budget);
    } else {
        position.sub_x += target.move_dir_x * fraction;
        position.sub_y += target.move_dir_y * fraction;
    }
}

pub(super) fn apply_cell_transition_remainder(
    target: &mut MovementTarget,
    position: &mut Position,
    dx_cell: i32,
    dy_cell: i32,
    nx: u16,
    ny: u16,
    is_infantry: bool,
) {
    // Infantry: clear blocking state on each cell arrival (fresh grace period).
    // Vehicles: keep both flags — once blocked, urgency escalates permanently.
    if is_infantry {
        target.blocked_delay = 0;
        target.path_blocked = false;
    }
    if dx_cell > 0 {
        position.sub_x -= crate::util::lepton::LEPTONS_PER_CELL;
    } else if dx_cell < 0 {
        position.sub_x += crate::util::lepton::LEPTONS_PER_CELL;
    }
    if dy_cell > 0 {
        position.sub_y -= crate::util::lepton::LEPTONS_PER_CELL;
    } else if dy_cell < 0 {
        position.sub_y += crate::util::lepton::LEPTONS_PER_CELL;
    }
    position.rx = nx;
    position.ry = ny;
}

pub(super) fn configure_motion_after_transition(
    target: &mut MovementTarget,
    locomotor: &Option<LocomotorState>,
    drive_track: &mut Option<DriveTrackState>,
    drive_locomotion: &mut Option<DriveLocomotionRuntime>,
    ship_locomotion: &mut Option<ShipLocomotionRuntime>,
    facing: &mut u8,
    facing_target: &mut Option<u8>,
    category: EntityCategory,
    mover_rot: i32,
    current_cell: (u16, u16),
    current_sub: (SimFixed, SimFixed),
    path_grid: Option<&PathGrid>,
    current_z: u8,
) {
    target.next_index += 1;
    if target.next_index < target.path.len() {
        let next = target.path[target.next_index];
        let ndx = next.0 as i32 - current_cell.0 as i32;
        let ndy = next.1 as i32 - current_cell.1 as i32;

        let new_face = facing_from_delta(ndx, ndy);
        let shared_kind = shared_track_kind(locomotor);
        let uses_drive_tracks = shared_kind.is_some();
        let is_ship = shared_kind == Some(LocomotorKind::Ship);
        let mut substituted_delta: Option<(i32, i32)> = None;
        let track_initiated = if uses_drive_tracks {
            if let Some(sel) = drive_track::select_shared_ordinary_track(*facing, new_face, is_ship)
            {
                *drive_track = drive_track::begin_drive_track(
                    sel.raw_track_index,
                    sel.flags,
                    ndx,
                    ndy,
                    sel.target_facing,
                );
                drive_track.is_some()
            } else if let Some(fb) = drive_track::build_shared_sharp_turn_fallback(*facing, is_ship)
            {
                // Sharp-turn substitute: no precomputed curve exists for this
                // turn angle, so drive straight ahead in the current facing for
                // one cell instead of curving.
                //
                // This consumes exactly ONE path node — the same single node any
                // straight step consumes, already accounted for by the advance at
                // the top of this function. The substitute is not a special case
                // in the node-consumption sense: native selection substitutes the
                // straight track and then falls into the shared no-cell-crossing
                // tail, which shifts the path queue by one like every other
                // non-crossing step. Dropping a second node here strands the unit
                // one waypoint further off-route on every sharp turn.
                // See docs/research/DRIVE_SHARP_TURN_FALLBACK_RE.md §3.2 — and note
                // its Q2 wording describes that shared shift as though it were
                // specific to the substitute, which is what this code read as a
                // licence to drop an extra node.
                let (cdx, cdy) = crate::util::fixed_math::dir_to_cell_delta(*facing);
                *drive_track = drive_track::begin_drive_track(
                    fb.raw_track_index,
                    fb.flags,
                    cdx,
                    cdy,
                    fb.target_facing,
                );
                if drive_track.is_some() {
                    // Movement direction follows the straight-ahead delta, not the
                    // unreachable node's delta; node bookkeeping is unchanged.
                    substituted_delta = Some((cdx, cdy));
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            *drive_track = None;
            false
        };

        if track_initiated {
            *facing_target = None;
            if let Some(kind) = shared_kind {
                let (accepted_dx, accepted_dy) = substituted_delta.unwrap_or((ndx, ndy));
                let endpoint = (
                    (i32::from(current_cell.0) + accepted_dx) as i16,
                    (i32::from(current_cell.1) + accepted_dy) as i16,
                );
                let endpoint_cell = (endpoint.0 as u16, endpoint.1 as u16);
                let endpoint_layer = if substituted_delta.is_some() {
                    locomotor
                        .as_ref()
                        .map_or(MovementLayer::Ground, |locomotor| locomotor.layer)
                } else {
                    target.layer_at(target.next_index)
                };
                accept_shared_track(
                    kind,
                    drive_locomotion,
                    ship_locomotion,
                    endpoint,
                    resolved_track_endpoint(path_grid, endpoint_cell, endpoint_layer, current_z),
                    1,
                );
            }
        } else {
            if is_ship && let Some(ship) = ship_locomotion.as_mut() {
                ship.head_to = None;
            }
            if category == EntityCategory::Infantry || mover_rot <= 0 {
                *facing = new_face;
            } else {
                *facing_target = Some(new_face);
            }
        }

        if category == EntityCategory::Infantry {
            // Infantry: direction from current sub-cell toward next cell's subcell position.
            // Use the allocated subcell offset to maintain visual spread during movement,
            // matching the WalkLocomotionClass which walks to FindSubCellDest result.
            let (sc_x, sc_y) = locomotor
                .as_ref()
                .and_then(|l| l.subcell_dest)
                .unwrap_or((CELL_CENTER_LEPTON, CELL_CENTER_LEPTON));
            let dest_x = SimFixed::from_num(ndx * 256) + sc_x;
            let dest_y = SimFixed::from_num(ndy * 256) + sc_y;
            let dx = dest_x - current_sub.0;
            let dy = dest_y - current_sub.1;
            target.move_dir_x = dx;
            target.move_dir_y = dy;
            target.move_dir_len = fixed_distance(dx, dy);
        } else {
            let (eff_dx, eff_dy) = substituted_delta.unwrap_or((ndx, ndy));
            let (d_x, d_y, d_len) = crate::util::lepton::cell_delta_to_lepton_dir(eff_dx, eff_dy);
            target.move_dir_x = d_x;
            target.move_dir_y = d_y;
            target.move_dir_len = d_len;
        }
    } else if let Some(loco) = locomotor {
        if let Some((dest_x, dest_y)) = loco.subcell_dest {
            let dx = dest_x - current_sub.0;
            let dy = dest_y - current_sub.1;
            target.move_dir_x = dx;
            target.move_dir_y = dy;
            let len: SimFixed = fixed_distance(dx, dy);
            target.move_dir_len = if len > SIM_HALF { len } else { SIM_ONE };
        }
    }
}

/// Per-tick hover steering: turn the body facing toward the current one-cell
/// waypoint through the native-frame `FacingClass` at the unit's rules ROT, and
/// point `move_dir` along the resulting hull heading (unit vector, len = 1) so
/// the shared lepton advancement produces facing-lagged curved motion.
///
/// Returns `true` while the required turn exceeds 45° (the turn-stall): the
/// caller brakes the throttle (request 0) and holds position for the tick.
/// Holding position during the hard-turn phase is a disclosed approximation —
/// the original translates along the stale heading while braking, but the
/// path-directed crossing loop cannot absorb a sideways cell exit; the drift
/// this suppresses is bounded by the brake-decay tail (see the P2b plan doc).
///
/// Hover never uses the stop-rotate-go path (`handle_vehicle_rotation`); any
/// `facing_target` left by shared path plumbing is cleared here.
pub(super) fn hover_steer(
    facing: &mut u8,
    facing_target: &mut Option<u8>,
    body_facing: &mut Option<super::facing_class::FacingClass>,
    position: &Position,
    target: &mut MovementTarget,
    rot: i32,
    native_frame: u32,
) -> bool {
    use crate::util::lepton::CELL_CENTER_LEPTON;

    *facing_target = None;
    let (wx, wy): (u16, u16) = if target.next_index < target.path.len() {
        target.path[target.next_index]
    } else {
        target.final_goal.unwrap_or((position.rx, position.ry))
    };
    let dxl: SimFixed = SimFixed::from_num((wx as i32 - position.rx as i32) * 256)
        + (CELL_CENTER_LEPTON - position.sub_x);
    let dyl: SimFixed = SimFixed::from_num((wy as i32 - position.ry as i32) * 256)
        + (CELL_CENTER_LEPTON - position.sub_y);
    if dxl == SIM_ZERO && dyl == SIM_ZERO {
        // Already exactly on the waypoint — nothing to steer toward.
        *body_facing = None;
        return false;
    }

    let desired16: u16 = super::hover::hover_desired_facing16(dxl, dyl);
    let rot_byte: u8 = rot.clamp(0, 0x7F) as u8;
    let bf = body_facing.get_or_insert_with(|| {
        super::facing_class::FacingClass::new((*facing as u16) << 8, rot_byte)
    });
    bf.set(desired16, native_frame);
    let current16: u16 = bf.current(native_frame);
    *facing = (current16 >> 8) as u8;

    let (mx, my) = super::hover::hover_move_dir(current16);
    target.move_dir_x = mx;
    target.move_dir_y = my;
    target.move_dir_len = SIM_ONE;

    super::hover::hover_turning_hard(current16, desired16)
}

/// Result of vehicle rotation — tells the caller whether to skip this tick.
pub(super) enum RotationResult {
    /// Still rotating in place — caller should `continue` (skip lepton advancement).
    StillRotating {
        debug_events: Vec<(u32, DebugEventKind)>,
    },
    /// Rotation complete or not needed — proceed with movement.
    ReadyToMove,
}

/// Handle vehicle in-place rotation before movement begins.
///
/// Vehicles rotate toward `facing_target` before advancing. When `ROT > 0` the
/// hull turns through a native-frame `FacingClass` at the unit's rules ROT —
/// gamemd's `DriveLocomotionClass::Do_Turn` on the body PrimaryFacing, whose
/// turn duration is `abs(delta_8bit) / ROT` native frames (frame-count based,
/// NOT millisecond based). `ROT = 0` means instant snap. Infantry are excluded
/// by the caller (they always turn instantly without this function).
///
/// `facing` stays the authoritative rendered/logic heading; each tick it is
/// refreshed from the interpolator's current value (top byte of the 16-bit
/// facing). `body_facing` holds the interpolator and lives only while a turn is
/// in progress — it is cleared as soon as there is no active rotation.
///
/// Takes individual fields to avoid borrow conflicts with `entity.movement_target`.
pub(super) fn handle_vehicle_rotation(
    facing: &mut u8,
    facing_target: &mut Option<u8>,
    body_facing: &mut Option<super::facing_class::FacingClass>,
    position: &mut Position,
    locomotor: &mut Option<LocomotorState>,
    rot: i32,
    native_frame: u32,
    sim_tick: u64,
) -> RotationResult {
    let Some(target_facing) = *facing_target else {
        // No in-place rotation in progress — drop any stale interpolator so the
        // next turn starts fresh from the then-current heading.
        *body_facing = None;
        return RotationResult::ReadyToMove;
    };
    if rot <= 0 {
        // ROT=0 — instant turn, no gradual rotation.
        *facing = target_facing;
        *facing_target = None;
        *body_facing = None;
        return RotationResult::ReadyToMove;
    }

    // Rules ROT drives the hull FacingClass. `set` is a no-op once already aimed
    // at the target, so calling it each tick is safe and yields smooth retargets
    // (it snapshots the live animated value into the rotation origin).
    let rot_byte = rot.min(0x7F) as u8;
    let bf = body_facing.get_or_insert_with(|| {
        super::facing_class::FacingClass::new((*facing as u16) << 8, rot_byte)
    });
    bf.set((target_facing as u16) << 8, native_frame);
    *facing = (bf.current(native_frame) >> 8) as u8;

    if bf.is_rotating(native_frame) {
        // Still rotating in place — advance facing but don't move.
        let mut debug_events = Vec::new();
        if let Some(loco) = locomotor {
            let old_phase = loco.phase;
            loco.phase = GroundMovePhase::Accelerating;
            if old_phase != GroundMovePhase::Accelerating {
                debug_events.push((
                    sim_tick as u32,
                    DebugEventKind::PhaseChange {
                        from: format!("{:?}", old_phase),
                        to: "Accelerating".into(),
                        reason: "movement started".into(),
                    },
                ));
            }
        }
        RotationResult::StillRotating { debug_events }
    } else {
        // Rotation complete — snap to the exact target and start moving.
        *facing = target_facing;
        *facing_target = None;
        *body_facing = None;
        RotationResult::ReadyToMove
    }
}

/// Result of lepton position advancement.
pub(super) enum AdvanceResult {
    /// Drive track is active — caller should `continue` (skip cell crossings).
    DriveTrackActive,
    /// Drive track crossed a cell boundary — caller must handle the cell
    /// transition (update rx/ry, advance next_index, reserve destination),
    /// then continue the track on the next tick.
    DriveTrackCellJump,
    /// Drive track reached the chain_index — caller should attempt to chain
    /// into a follow-on track curve (check passability of the next-next cell,
    /// select new track if OK). If chaining fails, the current track continues
    /// on the next tick.
    DriveTrackChainReady,
    /// Normal advancement done — caller should proceed to cell crossings.
    ReadyForCrossings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::locomotor_type::LocomotorKind;
    use crate::sim::movement::locomotor::LocomotorState;

    /// Body/hull in-place turn duration = abs(delta_8bit) / ROT native frames
    /// (gamemd DriveLocomotionClass::Do_Turn on the hull FacingClass at the
    /// unit's rules ROT). Verified in
    /// docs/research/BODY_FACING_DRIVE_LOCOMOTOR_ROT_GHIDRA_REPORT.md: for ROT=5
    /// a 90° (0x40) turn is 12 frames and a 180° (0x80) turn is 25 frames — the
    /// values gamemd produces, and the whole point of the frame-based model
    /// (the old ms-integrated path was tick-rate-dependent and ~2× too fast).
    #[test]
    fn test_body_rotation_matches_native_frame_duration() {
        // Drive the in-place rotation frame by frame, returning the native-frame
        // count at which it completes (ReadyToMove with the exact target reached).
        fn frames_to_turn(from: u8, to: u8, rot: i32) -> u32 {
            let mut facing = from;
            let mut facing_target = Some(to);
            let mut body_facing = None;
            let mut position = Position {
                rx: 5,
                ry: 5,
                z: 0,
                sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
                sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
            };
            let mut locomotor = None;
            for frame in 0..1000u32 {
                match handle_vehicle_rotation(
                    &mut facing,
                    &mut facing_target,
                    &mut body_facing,
                    &mut position,
                    &mut locomotor,
                    rot,
                    frame,
                    0,
                ) {
                    RotationResult::ReadyToMove => {
                        assert_eq!(facing, to, "rotation must land exactly on the target");
                        assert!(body_facing.is_none(), "interpolator cleared on completion");
                        return frame;
                    }
                    RotationResult::StillRotating { .. } => {}
                }
            }
            panic!("rotation did not complete within 1000 frames");
        }

        // ROT=5 (MTNK/AMCV/HTNK/…): 0x40 = 16384/1280 = 12 frames; 0x80 = 25.
        assert_eq!(
            frames_to_turn(0x00, 0x40, 5),
            12,
            "90° at ROT=5 = 12 frames"
        );
        assert_eq!(
            frames_to_turn(0x00, 0x80, 5),
            25,
            "180° at ROT=5 = 25 frames"
        );
        // Counter-clockwise 90° (shortest arc) is the same duration.
        assert_eq!(
            frames_to_turn(0x40, 0x00, 5),
            12,
            "CCW 90° at ROT=5 = 12 frames"
        );
        // ROT=0 snaps instantly (no gradual rotation).
        assert_eq!(frames_to_turn(0x00, 0x40, 0), 0, "ROT=0 turns instantly");
    }

    #[test]
    fn drive_track_completion_retries_new_track_with_residual_only() {
        let mut target = MovementTarget {
            path: vec![(0, 0), (1, 0)],
            path_layers: vec![MovementLayer::Ground; 2],
            next_index: 1,
            speed: SimFixed::from_num(300),
            current_speed: SimFixed::from_num(300),
            move_dir_x: SimFixed::from_num(256),
            move_dir_y: SIM_ZERO,
            move_dir_len: SimFixed::from_num(256),
            final_goal: Some((1, 0)),
            ..Default::default()
        };
        let mut position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        };
        let mut facing = 0;
        let mut facing_target = None;
        let mut drive_track_state =
            Some(drive_track::begin_drive_track(15, 0, 0, 0, 0xC0).unwrap());
        let last_index = drive_track::raw_track_meta(15).unwrap().points_count - 1;
        drive_track_state.as_mut().unwrap().point_index = last_index - 1;
        let mut drive_locomotion = Some(DriveLocomotionRuntime::default());
        let mut ship_locomotion = None;
        let mut locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));

        let result = advance_lepton_position(
            &mut target,
            &mut position,
            &mut facing,
            &mut facing_target,
            &mut drive_track_state,
            &mut drive_locomotion,
            &mut ship_locomotion,
            &mut locomotor,
            EntityCategory::Unit,
            SimFixed::from_num(300),
            movement_frame_budget_from_current_speed(SimFixed::from_num(300)),
            SimFixed::from_num(1) / SimFixed::from_num(15),
            1,
            None,
            MovementLayer::Ground,
            None,
        );

        assert!(matches!(result, AdvanceResult::DriveTrackActive));
        let drive = drive_locomotion.as_ref().expect("drive runtime");
        assert_eq!(drive.residual_budget, 6);
        let track = drive_track_state.as_ref().expect("new track installed");
        assert_eq!(track.residual, 6);
        assert_eq!(drive.point_index, track.point_index);
    }

    #[test]
    fn drive_track_first_native_frame_uses_native_frame_budget() {
        let mut target = MovementTarget {
            path: vec![(0, 0), (1, 0)],
            path_layers: vec![MovementLayer::Ground; 2],
            next_index: 1,
            speed: SimFixed::from_num(255),
            current_speed: SimFixed::from_num(255) * SimFixed::lit("0.7"),
            move_dir_x: SimFixed::from_num(256),
            move_dir_y: SIM_ZERO,
            move_dir_len: SimFixed::from_num(256),
            final_goal: Some((1, 0)),
            ..Default::default()
        };
        let mut position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        };
        let mut facing = 0;
        let mut facing_target = None;
        let mut drive_track_state =
            Some(drive_track::begin_drive_track_with_head_offset(1, 0, 0, 0, 0).unwrap());
        let start_index = drive_track_state.as_ref().unwrap().point_index;
        let mut drive_locomotion = Some(DriveLocomotionRuntime::default());
        let mut ship_locomotion = None;
        let mut locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        let current_speed = target.current_speed;

        let result = advance_lepton_position(
            &mut target,
            &mut position,
            &mut facing,
            &mut facing_target,
            &mut drive_track_state,
            &mut drive_locomotion,
            &mut ship_locomotion,
            &mut locomotor,
            EntityCategory::Unit,
            current_speed,
            movement_frame_budget_from_current_speed(current_speed),
            SimFixed::from_num(1) / SimFixed::from_num(15),
            1,
            None,
            MovementLayer::Ground,
            None,
        );

        assert!(matches!(result, AdvanceResult::DriveTrackActive));
        let drive = drive_locomotion.as_ref().unwrap();
        let track = drive_track_state.as_ref().unwrap();
        assert_eq!(track.point_index, start_index + 1);
        assert_eq!(drive.residual_budget, 4);
        assert_eq!(track.residual, 4);
    }

    #[test]
    fn gsi_04_05_paid_track_point_clears_current_before_same_cell_coordinate_commit() {
        let mut target = MovementTarget {
            path: vec![(0, 0), (1, 0)],
            path_layers: vec![MovementLayer::Ground; 2],
            next_index: 1,
            speed: SimFixed::from_num(255),
            current_speed: SimFixed::from_num(255),
            move_dir_x: SimFixed::from_num(256),
            move_dir_y: SIM_ZERO,
            move_dir_len: SimFixed::from_num(256),
            ..Default::default()
        };
        let mut position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        };
        let mut facing = 0;
        let mut facing_target = None;
        let mut drive_track_state =
            Some(drive_track::begin_drive_track_with_head_offset(1, 0, 0, 0, 0).unwrap());
        let mut drive_locomotion = Some(DriveLocomotionRuntime {
            occupation_head_to: Some(DriveOccupationFootprint {
                rx: 1,
                ry: 0,
                layer: MovementLayer::Ground,
            }),
            ..Default::default()
        });
        let mut ship_locomotion = None;
        let mut locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        let mut bits = CellOccupationGrid::new();
        bits.mark_vehicle_on_layer(0, 0, 1, MovementLayer::Ground);
        bits.mark_vehicle_on_layer(1, 0, 1, MovementLayer::Ground);

        let result = advance_lepton_position(
            &mut target,
            &mut position,
            &mut facing,
            &mut facing_target,
            &mut drive_track_state,
            &mut drive_locomotion,
            &mut ship_locomotion,
            &mut locomotor,
            EntityCategory::Unit,
            SimFixed::from_num(255),
            movement_frame_budget_from_current_speed(SimFixed::from_num(255)),
            SimFixed::from_num(1) / SimFixed::from_num(15),
            1,
            Some(&mut bits),
            MovementLayer::Ground,
            None,
        );

        assert!(matches!(result, AdvanceResult::DriveTrackActive));
        assert_eq!((position.rx, position.ry), (0, 0));
        assert_eq!(bits.vehicle_bits(0, 0, MovementLayer::Ground), 0);
        assert_eq!(bits.vehicle_bits(1, 0, MovementLayer::Ground), 0x20);
        assert!(
            drive_locomotion
                .as_ref()
                .unwrap()
                .current_occupation_cleared
        );
    }

    #[test]
    fn drive_track_each_call_consumes_fresh_native_frame_budget() {
        let mut target = MovementTarget {
            path: vec![(0, 0), (1, 0)],
            path_layers: vec![MovementLayer::Ground; 2],
            next_index: 1,
            speed: SimFixed::from_num(255),
            current_speed: SimFixed::from_num(255) * SimFixed::lit("0.7"),
            move_dir_x: SimFixed::from_num(256),
            move_dir_y: SIM_ZERO,
            move_dir_len: SimFixed::from_num(256),
            final_goal: Some((1, 0)),
            ..Default::default()
        };
        let mut position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        };
        let mut facing = 0;
        let mut facing_target = None;
        let mut drive_track_state =
            Some(drive_track::begin_drive_track_with_head_offset(1, 0, 0, 0, 0).unwrap());
        let mut drive_locomotion = Some(DriveLocomotionRuntime::default());
        let mut ship_locomotion = None;
        let mut locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        let dt = SimFixed::from_num(1) / SimFixed::from_num(15);
        let current_speed = target.current_speed;

        let _ = advance_lepton_position(
            &mut target,
            &mut position,
            &mut facing,
            &mut facing_target,
            &mut drive_track_state,
            &mut drive_locomotion,
            &mut ship_locomotion,
            &mut locomotor,
            EntityCategory::Unit,
            current_speed,
            movement_frame_budget_from_current_speed(current_speed),
            dt,
            1,
            None,
            MovementLayer::Ground,
            None,
        );
        let index_after_native_frame = drive_track_state.as_ref().unwrap().point_index;

        let _ = advance_lepton_position(
            &mut target,
            &mut position,
            &mut facing,
            &mut facing_target,
            &mut drive_track_state,
            &mut drive_locomotion,
            &mut ship_locomotion,
            &mut locomotor,
            EntityCategory::Unit,
            current_speed,
            movement_frame_budget_from_current_speed(current_speed),
            dt,
            1,
            None,
            MovementLayer::Ground,
            None,
        );
        assert_eq!(
            drive_track_state.as_ref().unwrap().point_index,
            index_after_native_frame + 2,
            "every explicit call must consume a fresh reached-frame budget"
        );
    }

    fn advance_straight_walk(effective_speed: SimFixed, frame_budget: i32) -> Position {
        let mut target = MovementTarget {
            path: vec![(0, 0), (1, 0)],
            path_layers: vec![MovementLayer::Ground; 2],
            next_index: 1,
            speed: effective_speed,
            current_speed: effective_speed,
            move_dir_x: SimFixed::from_num(256),
            move_dir_y: SIM_ZERO,
            move_dir_len: SimFixed::from_num(256),
            final_goal: Some((1, 0)),
            ..Default::default()
        };
        let mut position = Position {
            rx: 0,
            ry: 0,
            z: 0,
            sub_x: SimFixed::from_num(128),
            sub_y: SimFixed::from_num(128),
        };
        let mut facing = 0;
        let mut facing_target = None;
        let mut drive_track_state = None;
        let mut drive_locomotion = None;
        let mut ship_locomotion = None;
        let mut locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Walk));

        let result = advance_lepton_position(
            &mut target,
            &mut position,
            &mut facing,
            &mut facing_target,
            &mut drive_track_state,
            &mut drive_locomotion,
            &mut ship_locomotion,
            &mut locomotor,
            EntityCategory::Infantry,
            effective_speed,
            frame_budget,
            SimFixed::from_num(1) / SimFixed::from_num(15),
            1,
            None,
            MovementLayer::Ground,
            None,
        );
        assert!(matches!(result, AdvanceResult::ReadyForCrossings));
        position
    }

    #[test]
    fn straight_walk_commits_the_whole_frame_budget() {
        let effective_speed = SimFixed::from_num(150);
        let position = advance_straight_walk(
            effective_speed,
            movement_frame_budget_from_current_speed(effective_speed),
        );

        assert_eq!(position.sub_x, SimFixed::from_num(138));
        assert_eq!(position.sub_y, SimFixed::from_num(128));
    }

    #[test]
    fn straight_walk_zero_budget_leaves_position_unchanged() {
        let position = advance_straight_walk(SIM_ZERO, 0);

        assert_eq!(position.sub_x, SimFixed::from_num(128));
        assert_eq!(position.sub_y, SimFixed::from_num(128));
    }
}

fn install_drive_head_to_occupation(
    drive_locomotion: &mut Option<DriveLocomotionRuntime>,
    cell_occupation: &mut Option<&mut CellOccupationGrid>,
    entity_id: u64,
    current_cell: (u16, u16),
    current_layer: MovementLayer,
    next: Option<DriveOccupationFootprint>,
) {
    let Some(drive) = drive_locomotion.as_mut() else {
        return;
    };
    match (next, cell_occupation.as_deref_mut()) {
        (Some(next), Some(occupation)) => {
            crate::sim::occupancy::replace_drive_head_to_occupation(
                drive,
                occupation,
                entity_id,
                current_cell,
                current_layer,
                next,
            );
        }
        (Some(next), None) => drive.occupation_head_to = Some(next),
        (None, Some(occupation)) => {
            crate::sim::occupancy::clear_drive_head_to_occupation_for_replacement(
                drive,
                occupation,
                entity_id,
                current_cell,
                current_layer,
            );
        }
        (None, None) => drive.occupation_head_to = None,
    }
}

fn advance_drive_track_retry_after_selection(
    target: &mut MovementTarget,
    position: &mut Position,
    facing: &mut u8,
    facing_target: &mut Option<u8>,
    drive_track_state: &mut Option<DriveTrackState>,
    drive_locomotion: &mut Option<DriveLocomotionRuntime>,
    cell_occupation: &mut Option<&mut CellOccupationGrid>,
    entity_id: u64,
    current_occupation_layer: MovementLayer,
) -> AdvanceResult {
    let Some(track_state) = drive_track_state else {
        return AdvanceResult::ReadyForCrossings;
    };
    let prior_point_index = track_state.point_index;
    let advance = if let Some(drive) = drive_locomotion.as_mut() {
        let advance = drive_track::advance_drive_track_with_budget(
            track_state,
            0,
            &mut drive.residual_budget,
        );
        drive.point_index = track_state.point_index;
        drive.track_valid = !advance.finished;
        advance
    } else {
        drive_track::advance_drive_track(track_state, SIM_ZERO, SIM_ONE)
    };
    if track_state.point_index != prior_point_index
        && let (Some(drive), Some(occupation)) =
            (drive_locomotion.as_mut(), cell_occupation.as_deref_mut())
    {
        crate::sim::occupancy::clear_current_drive_occupation_for_paid_point(
            drive,
            occupation,
            entity_id,
            (position.rx, position.ry),
            current_occupation_layer,
        );
    }
    *facing = advance.facing;
    *facing_target = None;

    if advance.cell_jump && target.next_index < target.path.len() {
        position.sub_x = advance.sub_x;
        position.sub_y = advance.sub_y;
        return AdvanceResult::DriveTrackCellJump;
    }

    if advance.chain_ready && target.next_index < target.path.len() {
        position.sub_x = advance.sub_x;
        position.sub_y = advance.sub_y;
        return AdvanceResult::DriveTrackChainReady;
    }

    if advance.finished {
        *drive_track_state = None;
        position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
        position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
        return AdvanceResult::ReadyForCrossings;
    }

    position.sub_x = advance.sub_x;
    position.sub_y = advance.sub_y;
    if let Some(track_state) = drive_track_state.as_ref() {
        if let Some(interp) = drive_track::interp_sub_step(
            advance.sub_x,
            advance.sub_y,
            advance.next_step_delta_x,
            advance.next_step_delta_y,
            track_state.residual,
            advance.had_next_step,
        ) {
            position.sub_x = interp.sub_x;
            position.sub_y = interp.sub_y;
        }
    }
    AdvanceResult::DriveTrackActive
}

/// Advance sub_x/sub_y toward the next cell — either via drive track (smooth
/// curve) or straight-line lepton vector. Includes infantry wobble seeding.
///
/// Takes individual entity fields to avoid borrow conflicts with
/// `entity.movement_target` (which the caller holds as `ref mut target`).
pub(super) fn advance_lepton_position(
    target: &mut MovementTarget,
    position: &mut Position,
    facing: &mut u8,
    facing_target: &mut Option<u8>,
    drive_track_state: &mut Option<DriveTrackState>,
    drive_locomotion: &mut Option<DriveLocomotionRuntime>,
    ship_locomotion: &mut Option<ShipLocomotionRuntime>,
    locomotor: &mut Option<LocomotorState>,
    category: EntityCategory,
    effective_speed: SimFixed,
    frame_budget: i32,
    dt: SimFixed,
    entity_id: u64,
    mut cell_occupation: Option<&mut CellOccupationGrid>,
    current_occupation_layer: MovementLayer,
    path_grid: Option<&PathGrid>,
) -> AdvanceResult {
    if let Some(track_state) = drive_track_state {
        // Drive track advancement: step through pre-computed curve points.
        // The track handles position AND facing, producing smooth turns.
        let prior_point_index = track_state.point_index;
        let advance = if let Some(drive) = drive_locomotion.as_mut() {
            let fresh_budget = movement_frame_budget_from_current_speed(effective_speed);
            let advance = drive_track::advance_drive_track_with_budget(
                track_state,
                fresh_budget,
                &mut drive.residual_budget,
            );
            drive.point_index = track_state.point_index;
            drive.track_valid = !advance.finished;
            advance
        } else {
            drive_track::advance_drive_track(track_state, effective_speed, dt)
        };
        if track_state.point_index != prior_point_index
            && let (Some(drive), Some(occupation)) =
                (drive_locomotion.as_mut(), cell_occupation.as_deref_mut())
        {
            crate::sim::occupancy::clear_current_drive_occupation_for_paid_point(
                drive,
                occupation,
                entity_id,
                (position.rx, position.ry),
                current_occupation_layer,
            );
        }
        *facing = advance.facing;
        *facing_target = None; // track handles facing

        if advance.cell_jump && target.next_index < target.path.len() {
            // Coordinate-based cell crossing detected — the transformed track
            // point position landed in a different cell. The cell_offset was
            // already adjusted inside advance_drive_track. Update visual position.
            position.sub_x = advance.sub_x;
            position.sub_y = advance.sub_y;
            // Signal the caller to handle the actual cell transition
            // (update rx/ry, reserve destination, bridge state, etc.).
            return AdvanceResult::DriveTrackCellJump;
        }

        if advance.chain_ready && target.next_index < target.path.len() {
            // Track reached chain_index — signal caller to attempt chaining.
            // The caller will check Can_Enter_Cell on the next-next cell and
            // either replace the track state or let the current track continue.
            position.sub_x = advance.sub_x;
            position.sub_y = advance.sub_y;
            return AdvanceResult::DriveTrackChainReady;
        }

        if advance.finished {
            *drive_track_state = None;
            position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
            position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
            let shared_kind = shared_track_kind(locomotor);
            let uses_drive_tracks = shared_kind.is_some();
            let is_ship = shared_kind == Some(LocomotorKind::Ship);
            if uses_drive_tracks
                && category != EntityCategory::Infantry
                && let Some(next) = target.path.get(target.next_index).copied()
            {
                let ndx = next.0 as i32 - position.rx as i32;
                let ndy = next.1 as i32 - position.ry as i32;
                let next_face = facing_from_delta(ndx, ndy);
                let mut substituted_delta: Option<(i32, i32)> = None;
                let new_track = if let Some(sel) =
                    drive_track::select_shared_ordinary_track(*facing, next_face, is_ship)
                {
                    drive_track::begin_drive_track(
                        sel.raw_track_index,
                        sel.flags,
                        ndx,
                        ndy,
                        sel.target_facing,
                    )
                } else if let Some(fb) =
                    drive_track::build_shared_sharp_turn_fallback(*facing, is_ship)
                {
                    let (cdx, cdy) = crate::util::fixed_math::dir_to_cell_delta(*facing);
                    substituted_delta = Some((cdx, cdy));
                    drive_track::begin_drive_track(
                        fb.raw_track_index,
                        fb.flags,
                        cdx,
                        cdy,
                        fb.target_facing,
                    )
                } else {
                    None
                };
                if let Some(new_track) = new_track {
                    if substituted_delta.is_some() {
                        target.next_index += 1;
                    }
                    let (eff_dx, eff_dy) = substituted_delta.unwrap_or((ndx, ndy));
                    let (d_x, d_y, d_len) =
                        crate::util::lepton::cell_delta_to_lepton_dir(eff_dx, eff_dy);
                    target.move_dir_x = d_x;
                    target.move_dir_y = d_y;
                    target.move_dir_len = d_len;
                    *drive_track_state = Some(new_track);
                    *facing_target = None;
                    let endpoint = (
                        (i32::from(position.rx) + eff_dx) as i16,
                        (i32::from(position.ry) + eff_dy) as i16,
                    );
                    let endpoint_cell = (endpoint.0 as u16, endpoint.1 as u16);
                    let endpoint_layer = if substituted_delta.is_some() {
                        current_occupation_layer
                    } else {
                        target.layer_at(target.next_index)
                    };
                    accept_shared_track(
                        shared_kind.expect("shared track kind"),
                        drive_locomotion,
                        ship_locomotion,
                        endpoint,
                        resolved_track_endpoint(
                            path_grid,
                            endpoint_cell,
                            endpoint_layer,
                            position.z,
                        ),
                        1,
                    );
                    let next_occupation = if substituted_delta.is_some() {
                        substituted_delta.and_then(|(dx, dy)| {
                            if current_occupation_layer != MovementLayer::Ground {
                                return None;
                            }
                            let rx = u16::try_from(i32::from(position.rx) + dx).ok()?;
                            let ry = u16::try_from(i32::from(position.ry) + dy).ok()?;
                            Some(DriveOccupationFootprint {
                                rx,
                                ry,
                                layer: MovementLayer::Ground,
                            })
                        })
                    } else {
                        (target.layer_at(target.next_index) == MovementLayer::Ground).then_some(
                            DriveOccupationFootprint {
                                rx: next.0,
                                ry: next.1,
                                layer: MovementLayer::Ground,
                            },
                        )
                    };
                    install_drive_head_to_occupation(
                        drive_locomotion,
                        &mut cell_occupation,
                        entity_id,
                        (position.rx, position.ry),
                        current_occupation_layer,
                        next_occupation,
                    );
                    return advance_drive_track_retry_after_selection(
                        target,
                        position,
                        facing,
                        facing_target,
                        drive_track_state,
                        drive_locomotion,
                        &mut cell_occupation,
                        entity_id,
                        current_occupation_layer,
                    );
                }
                if is_ship && let Some(ship) = ship_locomotion.as_mut() {
                    ship.head_to = None;
                }
            }
            // Track complete — snap to cell center so standard movement resumes.
            position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
            position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
            // Fall through to ReadyForCrossings — normal movement takes over.
        } else {
            // Mid-track, no events — apply discrete-step pos, then layer
            // sub-step interp on top using the residual budget. The interp
            // helper enforces the L4 cell-validity safety gate; cell occupancy
            // never changes mid-step (rx/ry unchanged on this path).
            position.sub_x = advance.sub_x;
            position.sub_y = advance.sub_y;
            if let Some(interp) = drive_track::interp_sub_step(
                advance.sub_x,
                advance.sub_y,
                advance.next_step_delta_x,
                advance.next_step_delta_y,
                track_state.residual,
                advance.had_next_step,
            ) {
                position.sub_x = interp.sub_x;
                position.sub_y = interp.sub_y;
            }
            return AdvanceResult::DriveTrackActive;
        }
    } else {
        let needs_drive_native_step = drive_locomotion
            .as_ref()
            .is_some_and(drive_locomotion_helpers::drive_requires_native_step);
        let shared_kind = shared_track_kind(locomotor);
        let is_ship = shared_kind == Some(LocomotorKind::Ship);
        let needs_ship_native_step = is_ship
            && ship_locomotion
                .as_ref()
                .is_some_and(|ship| !ship.path.directions.is_empty());
        if needs_drive_native_step || needs_ship_native_step {
            if target.next_index >= target.path.len() {
                if let Some(drive) = drive_locomotion.as_mut() {
                    drive.residual_budget = 0;
                    drive.path.cursor = drive.path.directions.len().min(u16::MAX as usize) as u16;
                    drive.path.directions.clear();
                }
                return AdvanceResult::ReadyForCrossings;
            }
            let uses_drive_tracks = shared_kind.is_some();
            if uses_drive_tracks
                && category != EntityCategory::Infantry
                && let Some(next) = target.path.get(target.next_index).copied()
            {
                let ndx = next.0 as i32 - position.rx as i32;
                let ndy = next.1 as i32 - position.ry as i32;
                let next_face = facing_from_delta(ndx, ndy);
                let mut substituted_delta: Option<(i32, i32)> = None;
                let new_track = if let Some(sel) =
                    drive_track::select_shared_ordinary_track(*facing, next_face, is_ship)
                {
                    drive_track::begin_drive_track(
                        sel.raw_track_index,
                        sel.flags,
                        ndx,
                        ndy,
                        sel.target_facing,
                    )
                } else if let Some(fb) =
                    drive_track::build_shared_sharp_turn_fallback(*facing, is_ship)
                {
                    let (cdx, cdy) = crate::util::fixed_math::dir_to_cell_delta(*facing);
                    substituted_delta = Some((cdx, cdy));
                    drive_track::begin_drive_track(
                        fb.raw_track_index,
                        fb.flags,
                        cdx,
                        cdy,
                        fb.target_facing,
                    )
                } else {
                    None
                };
                if let Some(new_track) = new_track {
                    if substituted_delta.is_some() {
                        target.next_index += 1;
                    }
                    let (eff_dx, eff_dy) = substituted_delta.unwrap_or((ndx, ndy));
                    let (d_x, d_y, d_len) =
                        crate::util::lepton::cell_delta_to_lepton_dir(eff_dx, eff_dy);
                    target.move_dir_x = d_x;
                    target.move_dir_y = d_y;
                    target.move_dir_len = d_len;
                    *drive_track_state = Some(new_track);
                    *facing_target = None;
                    let endpoint = (
                        (i32::from(position.rx) + eff_dx) as i16,
                        (i32::from(position.ry) + eff_dy) as i16,
                    );
                    let endpoint_cell = (endpoint.0 as u16, endpoint.1 as u16);
                    let endpoint_layer = if substituted_delta.is_some() {
                        current_occupation_layer
                    } else {
                        target.layer_at(target.next_index)
                    };
                    accept_shared_track(
                        shared_kind.expect("shared track kind"),
                        drive_locomotion,
                        ship_locomotion,
                        endpoint,
                        resolved_track_endpoint(
                            path_grid,
                            endpoint_cell,
                            endpoint_layer,
                            position.z,
                        ),
                        1,
                    );
                    let next_occupation = if substituted_delta.is_some() {
                        substituted_delta.and_then(|(dx, dy)| {
                            if current_occupation_layer != MovementLayer::Ground {
                                return None;
                            }
                            let rx = u16::try_from(i32::from(position.rx) + dx).ok()?;
                            let ry = u16::try_from(i32::from(position.ry) + dy).ok()?;
                            Some(DriveOccupationFootprint {
                                rx,
                                ry,
                                layer: MovementLayer::Ground,
                            })
                        })
                    } else {
                        (target.layer_at(target.next_index) == MovementLayer::Ground).then_some(
                            DriveOccupationFootprint {
                                rx: next.0,
                                ry: next.1,
                                layer: MovementLayer::Ground,
                            },
                        )
                    };
                    install_drive_head_to_occupation(
                        drive_locomotion,
                        &mut cell_occupation,
                        entity_id,
                        (position.rx, position.ry),
                        current_occupation_layer,
                        next_occupation,
                    );
                    return advance_drive_track_retry_after_selection(
                        target,
                        position,
                        facing,
                        facing_target,
                        drive_track_state,
                        drive_locomotion,
                        &mut cell_occupation,
                        entity_id,
                        current_occupation_layer,
                    );
                }
                if is_ship && let Some(ship) = ship_locomotion.as_mut() {
                    ship.head_to = None;
                }
            }
            return AdvanceResult::DriveTrackActive;
        }
        let whole_lepton_result = locomotor
            .as_ref()
            .is_some_and(|locomotor| locomotor.kind == LocomotorKind::Walk);
        let lepton_step = if whole_lepton_result {
            SimFixed::from_num(frame_budget)
        } else {
            effective_speed * dt
        };
        if target.move_dir_len > SIM_ZERO {
            let frac: SimFixed = lepton_step / target.move_dir_len;
            // When walking to subcell_dest (path exhausted), clamp so we
            // don't overshoot. Without this, frac > 1.0 makes the infantry
            // walk past the destination and off the cell.
            if frac >= SIM_ONE {
                if let Some(loco) = locomotor {
                    if let Some((dest_x, dest_y)) = loco.subcell_dest {
                        if target.next_index >= target.path.len() {
                            // Snap to destination — we'd overshoot this tick.
                            position.sub_x = dest_x;
                            position.sub_y = dest_y;
                            // Fall through below.
                            // The post-loop check will detect arrival and finish.
                        }
                    }
                }
                // For cell-to-cell movement, frac > 1.0 is normal — it means
                // the entity crossed a cell boundary, handled by the crossing loop.
                if target.next_index < target.path.len()
                    || locomotor.as_ref().and_then(|l| l.subcell_dest).is_none()
                {
                    advance_straight_position(
                        target,
                        position,
                        frame_budget,
                        frac,
                        whole_lepton_result,
                    );
                }
            } else {
                advance_straight_position(
                    target,
                    position,
                    frame_budget,
                    frac,
                    whole_lepton_result,
                );
            }
        }

        // Advance infantry wobble phase while walking.
        // Original engine: WalkLocomotionClass accumulates wobble each tick
        // via `wobble += 3.0 / (wobbleRate / turnRate)`.
        if category == EntityCategory::Infantry {
            if let Some(loco) = locomotor {
                // Seed phase from entity ID on first tick so group members
                // don't bob in sync — each starts at a different phase.
                if loco.infantry_wobble_phase == 0.0 {
                    loco.infantry_wobble_phase =
                        (entity_id.wrapping_mul(2654435761) & 0xFFFF) as f32 / 0xFFFF as f32
                            * std::f32::consts::TAU;
                }
                let dt_f32: f32 = dt.to_num::<f32>();
                loco.infantry_wobble_phase += super::INFANTRY_WOBBLE_RATE * dt_f32;
            }
        }
    }

    AdvanceResult::ReadyForCrossings
}

/// Output from the cell boundary crossing loop.
pub(super) struct CrossingOutput {
    /// If set, the caller must handle deferred occupancy outside the entity borrow.
    pub deferred_cell_check: Option<DeferredCellCheck>,
    /// Bridge render state to apply after the loop. Predicate-driven; see movement_bridge.rs.
    pub pending_bridge_update: super::movement_bridge::BridgeStateUpdate,
    /// The resolved movement layer after all crossings.
    pub active_layer: MovementLayer,
    /// Debug events accumulated during crossing checks.
    pub debug_events: Vec<(u32, DebugEventKind)>,
    /// Whether the entity was marked as stuck and should abort.
    pub aborted_for_stuck: bool,
}

/// Process cell boundary crossings — the inner loop that checks whether
/// sub_x/sub_y have crossed cell boundaries, validates terrain walkability,
/// cliff height, and occupancy, then performs cell transitions with lepton
/// remainder carry-over.
///
/// Takes individual entity fields to avoid borrow conflicts with
/// `entity.movement_target` (which the caller holds as `ref mut target`).
#[allow(clippy::too_many_arguments)]
pub(super) fn process_cell_crossings(
    target: &mut MovementTarget,
    position: &mut Position,
    facing: &mut u8,
    facing_target: &mut Option<u8>,
    body_facing: Option<super::FacingClass>,
    locomotor: &mut Option<LocomotorState>,
    drive_track_state: &mut Option<DriveTrackState>,
    drive_locomotion: &mut Option<DriveLocomotionRuntime>,
    ship_locomotion: &mut Option<ShipLocomotionRuntime>,
    sub_cell: &mut Option<u8>,
    category: EntityCategory,
    entity_id: u64,
    mut active_layer: MovementLayer,
    snap: &MoverSnapshot,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_cost_grid: Option<&TerrainCostGrid>,
    mover_entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    mover_entity_block_map: Option<&crate::sim::pathfinding::LayeredEntityBlockMap>,
    live_building_entry_skips: &LiveBuildingEntrySkipMap,
    occupancy: &mut OccupancyGrid,
    cell_occupation: &mut CellOccupationGrid,
    occupancy_enter_order: &mut u64,
    next_occupancy_enter_order: &mut EnterOrderCounter,
    stats: &mut MovementTickStats,
    finished_entities: &mut Vec<u64>,
    rng: &mut SimRng,
    ctx: PathfindingContext<'_>,
    mcfg: MovementConfig,
    sim_tick: u64,
    marker_context: Option<super::path_markers::BridgeMarkerContext<'_>>,
) -> CrossingOutput {
    let mut debug_events: Vec<(u32, DebugEventKind)> = Vec::new();
    let mut deferred_cell_check: Option<DeferredCellCheck> = None;
    let mut pending_bridge_update: super::movement_bridge::BridgeStateUpdate =
        super::movement_bridge::BridgeStateUpdate::Unchanged;
    let mut projected_on_bridge_state = snap.on_bridge;
    let mut aborted_for_stuck: bool = false;

    loop {
        if target.next_index >= target.path.len() {
            break;
        }
        let old_rx = position.rx;
        let old_ry = position.ry;
        let (nx, ny): (u16, u16) = target.path[target.next_index];
        let dx_cell: i32 = nx as i32 - position.rx as i32;
        let dy_cell: i32 = ny as i32 - position.ry as i32;

        // Check if sub_x/sub_y have crossed cell boundaries on each axis.
        let crossed_x: bool = match dx_cell.signum() {
            1 => position.sub_x >= crate::util::lepton::LEPTONS_PER_CELL,
            -1 => position.sub_x <= SIM_ZERO,
            _ => true, // No X movement needed for this step.
        };
        let crossed_y: bool = match dy_cell.signum() {
            1 => position.sub_y >= crate::util::lepton::LEPTONS_PER_CELL,
            -1 => position.sub_y <= SIM_ZERO,
            _ => true,
        };
        if !(crossed_x && crossed_y) {
            break;
        }

        let next_layer = target.layer_at(target.next_index);
        let runtime_entry = evaluate_runtime_can_enter_cell(
            path_grid,
            next_layer,
            runtime_can_enter_cell_args(
                path_grid,
                (position.rx, position.ry),
                (nx, ny),
                projected_on_bridge_state,
                position.z,
            ),
        );
        let layer_context = runtime_entry.layers;
        let mut layer_grid_ok: Option<bool> = None;
        let mut layer_terrain_ok: Option<bool> = None;

        if !runtime_entry.bridge_traversal_allowed {
            position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
            position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
            *drive_track_state = None;
            target.movement_delay = 0;
            let mover_is_crusher = snap.regular_crusher || snap.omni_crusher;
            let evts = handle_blocked_tick(
                target,
                facing,
                body_facing,
                &snap.locomotor,
                drive_locomotion,
                ship_locomotion,
                entity_id,
                (position.rx, position.ry),
                active_layer,
                snap.on_bridge,
                stats,
                finished_entities,
                &mut aborted_for_stuck,
                ctx,
                entity_cost_grid,
                mover_entity_blocks,
                mover_entity_block_map,
                snap.too_big_to_fit_under_bridge,
                mcfg,
                rng,
                sim_tick,
                PATH_STUCK_INIT,
                mover_is_crusher,
                category == EntityCategory::Infantry,
                true,
                marker_context,
                occupancy,
            );
            debug_events.extend(evts);
            break;
        }

        // --- Terrain walkability check (static map data) ---
        let layer_walkable = match layer_context.terrain_layer {
            MovementLayer::Ground => {
                // Water movers (ships) bypass PathGrid — water cells are
                // marked non-walkable for land units but ships need them.
                // Use passability matrix directly, same as the pathfinder.
                let cost_grid = if target.ignore_terrain_cost {
                    None
                } else {
                    entity_cost_grid
                };
                let grid_ok: bool = match path_grid {
                    Some(grid) => crate::sim::pathfinding::is_cell_passable_for_mover_with_speed(
                        grid,
                        nx,
                        ny,
                        Some(snap.movement_zone),
                        snap.speed_type,
                        resolved_terrain,
                        cost_grid,
                        target.bypass_grid,
                        crate::sim::pathfinding::cell_entry::TerrainEntryMode::RuntimeTransition,
                    ),
                    None => true,
                };
                let terrain_ok: bool = true;
                layer_grid_ok = Some(grid_ok);
                layer_terrain_ok = Some(terrain_ok);
                grid_ok && terrain_ok
            }
            MovementLayer::Bridge => path_grid.is_some_and(|grid| {
                if !grid.is_walkable_on_layer(nx, ny, MovementLayer::Bridge) {
                    return false;
                }
                true
            }),
            MovementLayer::Air | MovementLayer::Underground => false,
        };
        if !layer_walkable {
            if snap.movement_zone.is_water_mover() {
                log::info!(
                    "NAVAL transition blocked: entity={} cur=({},{}) next=({},{}) layer={:?} grid_ok={:?} terrain_ok={:?} blocked_delay={} path_blocked={} {}",
                    entity_id,
                    position.rx,
                    position.ry,
                    nx,
                    ny,
                    next_layer,
                    layer_grid_ok,
                    layer_terrain_ok,
                    target.blocked_delay,
                    target.path_blocked,
                    naval_terrain_diag(resolved_terrain, (nx, ny)),
                );
            }
            // Undo lepton advancement — entity stays at cell center.
            position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
            position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
            *drive_track_state = None;
            // Terrain-blocked (building/cliff) — the path is stale.
            // Force immediate repath by clearing movement_delay.
            target.movement_delay = 0;
            let mover_is_crusher = snap.regular_crusher || snap.omni_crusher;
            let evts = handle_blocked_tick(
                target,
                facing,
                body_facing,
                &snap.locomotor,
                drive_locomotion,
                ship_locomotion,
                entity_id,
                (position.rx, position.ry),
                active_layer,
                snap.on_bridge,
                stats,
                finished_entities,
                &mut aborted_for_stuck,
                ctx,
                entity_cost_grid,
                mover_entity_blocks,
                mover_entity_block_map,
                snap.too_big_to_fit_under_bridge,
                mcfg,
                rng,
                sim_tick,
                PATH_STUCK_INIT,
                mover_is_crusher,
                category == EntityCategory::Infantry,
                true, // terrain block: skip code-2 grace period
                marker_context,
                occupancy,
            );
            debug_events.extend(evts);
            break;
        }

        // --- Cliff detection ---
        // Original engine: if height difference >= 3 levels and not a
        // bridge ramp, treat as cliff. Catches stale paths after terrain
        // changes, bump/scatter toward cliff edges, etc.
        if let Some(pg) = path_grid {
            if let Some(next_cell) = pg.cell(nx, ny) {
                let next_level = next_cell.effective_cell_z_for_layer(next_layer);
                let diff = (position.z as i16 - next_level as i16).unsigned_abs();
                let is_bridge_ramp =
                    next_cell.is_bridge_transition_cell() || next_cell.is_elevated_bridge_cell();
                if diff >= CLIFF_HEIGHT_THRESHOLD && !is_bridge_ramp {
                    position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
                    position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
                    *drive_track_state = None;
                    target.movement_delay = 0;
                    let mover_is_crusher = snap.regular_crusher || snap.omni_crusher;
                    let evts = handle_blocked_tick(
                        target,
                        facing,
                        body_facing,
                        &snap.locomotor,
                        drive_locomotion,
                        ship_locomotion,
                        entity_id,
                        (position.rx, position.ry),
                        active_layer,
                        snap.on_bridge,
                        stats,
                        finished_entities,
                        &mut aborted_for_stuck,
                        ctx,
                        entity_cost_grid,
                        mover_entity_blocks,
                        mover_entity_block_map,
                        snap.too_big_to_fit_under_bridge,
                        mcfg,
                        rng,
                        sim_tick,
                        PATH_STUCK_INIT,
                        mover_is_crusher,
                        category == EntityCategory::Infantry,
                        true, // cliff block: skip code-2 grace period
                        marker_context,
                        occupancy,
                    );
                    debug_events.extend(evts);
                    break;
                }
            }
        }

        // --- Occupancy check (entity-aware: sub-cell, crush, bump) ---
        // Occupancy check: vehicles defer to crush/bump/attack handler,
        // infantry defer to sub-cell/attack handler. Both break out of the
        // loop to release the mutable entity borrow for blocker lookups.
        let current_object_list_layer = if projected_on_bridge_state {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        if let Some(check) = detect_deferred_cell_check(
            snap.category,
            entity_id,
            target.bypass_grid,
            layer_context,
            (nx, ny),
            (position.rx, position.ry),
            current_object_list_layer,
            occupancy,
            cell_occupation,
            live_building_entry_skips,
        ) {
            deferred_cell_check = Some(check);
            break;
        }

        // --- Cell transition: carry over lepton remainder ---
        // Only adjust the axes that actually crossed a boundary.
        // Do NOT snap the perpendicular axis to center — that causes
        // a visible position jump when transitioning from diagonal
        // to cardinal movement (e.g., sub_x=51 → 128 = ~9px snap).
        apply_cell_transition_remainder(
            target,
            position,
            dx_cell,
            dy_cell,
            nx,
            ny,
            category == EntityCategory::Infantry,
        );
        // GATE A2 verified order: the object-list layer is selected by the
        // occupant's OnBridge byte sampled at each call site. Capture the OLD
        // (pre-transition) layer BEFORE evaluating the bridge transition, so the
        // old-cell removal walks the old layer and the new-cell insertion the new
        // layer (the two halves may differ when stepping on/off the deck).
        let old_occupancy_layer = if projected_on_bridge_state {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        let bridge_update = resolve_cell_transition_bridge_state(
            position,
            path_grid,
            (old_rx, old_ry),
            (nx, ny),
            next_layer,
        );
        projected_on_bridge_state =
            super::movement_bridge::projected_on_bridge(projected_on_bridge_state, bridge_update);
        if !matches!(
            bridge_update,
            super::movement_bridge::BridgeStateUpdate::Unchanged
        ) {
            pending_bridge_update = bridge_update;
        }
        let new_occupancy_layer = if projected_on_bridge_state {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        // Update occupancy grid: move entity from old cell to new cell, removing
        // on the OLD layer and inserting on the NEW layer (verified two-layer
        // order). Uses current sub_cell (from old cell). For infantry,
        // reserve_destination below may allocate a new sub-cell and correct it
        // via update_sub_cell.
        let insertion = CellListInsertion::from_category(category);
        let order = next_occupancy_enter_order.next();
        *occupancy_enter_order = order;
        occupancy.move_entity_layered(
            old_rx,
            old_ry,
            nx,
            ny,
            entity_id,
            old_occupancy_layer,
            new_occupancy_layer,
            *sub_cell,
            insertion,
        );
        if category == EntityCategory::Unit
            && let Some(drive) = drive_locomotion.as_mut()
        {
            crate::sim::occupancy::mark_current_drive_occupation_after_crossing(
                drive,
                cell_occupation,
                entity_id,
                (nx, ny),
                new_occupancy_layer,
            );
        }
        active_layer = next_layer;
        if let Some(loco) = locomotor {
            loco.layer = next_layer;
        }
        if !reserve_destination_after_transition(
            category,
            locomotor,
            drive_track_state,
            position,
            sub_cell,
            target,
            next_layer,
            nx,
            ny,
            occupancy,
            rng,
        ) {
            break;
        }
        // After reservation, infantry sub_cell may have changed.
        if category == EntityCategory::Infantry {
            occupancy.update_sub_cell(nx, ny, entity_id, *sub_cell);
        }
        stats.moved_steps = stats.moved_steps.saturating_add(1);

        configure_motion_after_transition(
            target,
            locomotor,
            drive_track_state,
            drive_locomotion,
            ship_locomotion,
            facing,
            facing_target,
            category,
            snap.rot,
            (nx, ny),
            (position.sub_x, position.sub_y),
            path_grid,
            position.z,
        );

        // Pre-allocate subcell in the NEXT path cell for infantry direction targeting.
        // FindSubCellDest reserves a subcell in the destination cell before walking,
        // so each infantry targets its own subcell position based on the destination
        // cell's occupancy rather than carrying the current cell's.
        if category == EntityCategory::Infantry && target.next_index < target.path.len() {
            let next_cell = target.path[target.next_index];
            if let Some(pre_sub) = bump_crush::allocate_sub_cell_with_preference(
                occupancy.get(next_cell.0, next_cell.1),
                active_layer,
                None,
                position.sub_x,
                position.sub_y,
                rng,
            ) {
                let (sc_x, sc_y) = crate::util::lepton::subcell_lepton_offset(Some(pre_sub));
                if let Some(loco) = locomotor {
                    loco.subcell_dest = Some((sc_x, sc_y));
                }
                // Recompute direction toward the destination cell's subcell.
                let ndx = next_cell.0 as i32 - nx as i32;
                let ndy = next_cell.1 as i32 - ny as i32;
                let dest_x = SimFixed::from_num(ndx * 256) + sc_x;
                let dest_y = SimFixed::from_num(ndy * 256) + sc_y;
                let dx = dest_x - position.sub_x;
                let dy = dest_y - position.sub_y;
                target.move_dir_x = dx;
                target.move_dir_y = dy;
                target.move_dir_len = fixed_distance(dx, dy);
            }
        }
    }

    CrossingOutput {
        deferred_cell_check,
        pending_bridge_update,
        active_layer,
        debug_events,
        aborted_for_stuck,
    }
}
