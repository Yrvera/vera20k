//! Ground movement tick — the per-tick state machine for all ground/bridge entities.
//!
//! Contains the main `tick_movement_with_grids()` function which processes every
//! entity that has a `MovementTarget`: rotation, speed ramping, drive tracks,
//! cell boundary crossings, bridge transitions, deferred occupancy checks,
//! formation sync, and bump/crush resolution.
//!
//! This is the largest single function in the codebase (~1,300 lines) because
//! ground movement is irreducibly complex — the borrow checker constrains how
//! the per-entity loop can be decomposed, and the function already delegates to
//! 6 private submodules (movement_path, movement_blocked, movement_bridge,
//! movement_step, movement_reservation, movement_occupancy).
//!
//! ## Dependency rules
//! - Internal to sim/movement — called via re-export in mod.rs.

use std::collections::{BTreeMap, BTreeSet};

use crate::map::entities::EntityCategory;
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::cell_rect::PlayfieldBounds;
use crate::sim::components::{DriveOccupationFootprint, MovementTarget, NavTargetRef, Position};
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::infantry;
use crate::sim::lifecycle_request::{LifecycleRequest, UninitReason};
use crate::sim::movement::movement_blocked::handle_blocked_tick;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::cell_entry::{self, CellEntryResult, TerrainEntryMode};
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig;
use crate::sim::pathfinding::zone_map::ZoneGrid;
use crate::sim::rng::SimRng;
use crate::sim::world::EnterOrderCounter;
use crate::util::fixed_math::{
    SIM_HALF, SIM_ONE, SIM_ZERO, SimFixed, fixed_distance, isqrt_i64,
    native_movement_frame_fraction,
};

use super::bump_crush;
use super::drive_locomotion;
use super::locomotor::{GroundMovePhase, MovementLayer};
use super::movement_bridge::{
    BRIDGE_Z_OFFSET, BridgeStateUpdate, apply_pending_bridge_render_state,
};
use super::movement_occupancy::{
    DeferredCellCheck, build_live_building_entry_skip_map,
    evaluate_runtime_can_enter_cell_with_transition, handle_deferred_occupancy,
    has_unignored_runtime_occupants_on_layers, runtime_can_enter_direction,
    runtime_current_effective_height,
};
use super::movement_path::{find_move_path, supports_layered_bridge_pathing};
use super::movement_step;
use super::path_markers::{BridgeMarkerContext, snapshot_bridge_marker_peers};
use super::tube_movement;
use super::{
    MIN_BRAKE_FRACTION, MovementConfig, MovementTickStats, MoverSnapshot, PATH_STUCK_INIT,
    PathfindingContext, PendingCrushKill, facing_from_delta, walking_to_subcell_dest,
};
use crate::sim::occupancy::{
    CellListInsertion, CellOccupationGrid, OccupancyGrid, RawCellOccupationGrid,
};

fn tick_forced_drive_tracks(
    entities: &mut EntityStore,
    entity_order: &[u64],
    tube_processed: &BTreeSet<u64>,
    occupancy: &mut OccupancyGrid,
    cell_occupation: &mut CellOccupationGrid,
    next_occupancy_enter_order: &mut EnterOrderCounter,
    dt: SimFixed,
    stats: &mut MovementTickStats,
) -> BTreeSet<u64> {
    let mut processed: BTreeSet<u64> = BTreeSet::new();
    for &entity_id in entity_order {
        let Some(entity) = entities.get_mut(entity_id) else {
            continue;
        };
        if tube_processed.contains(&entity_id)
            || entity.forced_drive_track.is_none()
            || entity.low_bridge_tube_state.is_some()
        {
            continue;
        }
        if entity.movement_layer_or_ground() != MovementLayer::Ground {
            continue;
        }

        let (advance, residual, point_index, paid_point) = {
            let (forced_track, drive_locomotion) =
                (&mut entity.forced_drive_track, &mut entity.drive_locomotion);
            let forced = forced_track.as_mut().expect("checked forced_drive_track");
            let prior_point_index = forced.track.point_index;
            let advance = if let Some(drive) = drive_locomotion.as_mut() {
                super::drive_track::advance_forced_drive_track(
                    forced,
                    dt,
                    &mut drive.residual_budget,
                )
            } else {
                // Defensive compatibility for old/test snapshots without the
                // owner runtime. Active Force_Track installers always create it.
                let mut residual = forced.track.residual;
                super::drive_track::advance_forced_drive_track(forced, dt, &mut residual)
            };
            let residual = drive_locomotion
                .as_ref()
                .map_or(forced.track.residual, |drive| drive.residual_budget);
            (
                advance,
                residual,
                forced.track.point_index,
                forced.track.point_index != prior_point_index,
            )
        };

        let current_cell = (entity.position.rx, entity.position.ry);
        if paid_point {
            // Forward progress on a forced curve clears the impatience flag
            // exactly as it does on an ordinary track — gamemd's paid-point
            // block is shared by both.
            if let Some(target) = entity.movement_target.as_mut() {
                target.path_blocked = false;
            }
            if let Some(drive) = entity.drive_locomotion.as_mut() {
                crate::sim::occupancy::clear_current_drive_occupation_for_paid_point(
                    drive,
                    cell_occupation,
                    entity_id,
                    current_cell,
                    MovementLayer::Ground,
                );
            }
        }
        if let Some(drive) = entity.drive_locomotion.as_mut() {
            drive.point_index = point_index;
            drive.track_valid = true;
        }
        entity.facing = advance.facing;
        entity.facing_target = None;
        let prior_subcell = (entity.position.sub_x, entity.position.sub_y);
        entity.position.sub_x = advance.sub_x;
        entity.position.sub_y = advance.sub_y;
        if !advance.finished
            && let Some(interp) = super::drive_track::interp_sub_step(
                advance.sub_x,
                advance.sub_y,
                advance.next_step_delta_x,
                advance.next_step_delta_y,
                residual,
                advance.had_next_step,
            )
        {
            entity.position.sub_x = interp.sub_x;
            entity.position.sub_y = interp.sub_y;
        }
        if (entity.position.sub_x, entity.position.sub_y) != prior_subcell {
            entity.position.exact_z_leptons = None;
        }
        processed.insert(entity_id);
        stats.movers_total = stats.movers_total.saturating_add(1);
        stats.moved_steps = stats.moved_steps.saturating_add(1);

        if advance.finished {
            // Track termination is gamemd's second unconditional reset of the
            // impatience flag, written before the terminal cell commit.
            if let Some(target) = entity.movement_target.as_mut() {
                target.path_blocked = false;
            }
            let head = entity
                .drive_locomotion
                .as_ref()
                .and_then(|drive| drive.head_to);
            if let Some(head) = head {
                let target_rx = head.x.div_euclid(256);
                let target_ry = head.y.div_euclid(256);
                if let (Ok(target_rx), Ok(target_ry)) =
                    (u16::try_from(target_rx), u16::try_from(target_ry))
                {
                    let old_cell = (entity.position.rx, entity.position.ry);
                    let target_cell = (target_rx, target_ry);
                    let cell_changed = old_cell != target_cell;
                    if cell_changed && entity.lifecycle.cell_marked {
                        // Native terminal order: Exit old cell, commit the exact
                        // head coordinates, then Enter the target list.
                        occupancy.remove_on_layer(
                            old_cell.0,
                            old_cell.1,
                            entity_id,
                            MovementLayer::Ground,
                        );
                        cell_occupation.clear_vehicle_on_layer(
                            old_cell.0,
                            old_cell.1,
                            entity_id,
                            MovementLayer::Ground,
                        );
                    }

                    entity.position.rx = target_rx;
                    entity.position.ry = target_ry;
                    entity.position.sub_x = SimFixed::from_num(head.x.rem_euclid(256));
                    entity.position.sub_y = SimFixed::from_num(head.y.rem_euclid(256));
                    entity.position.exact_z_leptons = None;
                    if let Ok(z) = u8::try_from(head.z) {
                        entity.position.z = z;
                    }

                    if cell_changed && entity.lifecycle.cell_marked {
                        entity.occupancy_enter_order = next_occupancy_enter_order.next();
                        occupancy.add(
                            target_rx,
                            target_ry,
                            entity_id,
                            MovementLayer::Ground,
                            None,
                            CellListInsertion::from_category(entity.category),
                        );
                    }
                    if let Some(drive) = entity.drive_locomotion.as_mut() {
                        crate::sim::occupancy::finish_drive_head_to_occupation(
                            drive,
                            cell_occupation,
                            entity_id,
                            target_cell,
                            MovementLayer::Ground,
                        );
                        drive.head_to = None;
                        drive.track_valid = false;
                        drive.track_index = -1;
                        drive.point_index = 0;
                    }
                } else {
                    log::warn!(
                        "forced drive track entity={} has out-of-map terminal head ({}, {})",
                        entity_id,
                        head.x,
                        head.y
                    );
                }
            }
            entity.forced_drive_track = None;
        } else if advance.cell_jump || advance.chain_ready {
            // Forced advancement deliberately does not synthesize ordinary
            // cell jumps. Keep this guard for malformed/unsupported metadata.
            log::warn!(
                "forced drive track entity={} ended on unsupported event: cell_jump={} chain_ready={}",
                entity_id,
                advance.cell_jump,
                advance.chain_ready
            );
            entity.forced_drive_track = None;
        }
    }
    processed
}

// Naval diagnostic functions moved to movement_occupancy.rs

/// 2D Euclidean distance in leptons from `pos` to the center of cell `goal`.
///
/// Used by the drive-track speed ramp to decide when to start braking. Cell
/// center is `cell * 256 + 128` leptons. i64 widening keeps `dx² + dy²` safe
/// on large maps (max sum-of-squares ~10^11 for a 64k-cell diagonal).
fn distance_to_goal_leptons(pos: &Position, goal: (u16, u16)) -> SimFixed {
    let unit_x: i64 = pos.rx as i64 * 256 + pos.sub_x.to_num::<i64>();
    let unit_y: i64 = pos.ry as i64 * 256 + pos.sub_y.to_num::<i64>();
    let goal_x: i64 = goal.0 as i64 * 256 + 128;
    let goal_y: i64 = goal.1 as i64 * 256 + 128;
    let dx = unit_x - goal_x;
    let dy = unit_y - goal_y;
    SimFixed::from_num(isqrt_i64(dx * dx + dy * dy) as i32)
}

/// Build a read-only snapshot of the mover's properties before entering the
/// inner movement loop. This avoids repeated `entities.get()` calls and keeps
/// the data available across the mutable/immutable borrow boundary.
fn snapshot_mover(
    entities: &EntityStore,
    entity_id: u64,
    playfield_bounds: Option<crate::sim::cell_rect::PlayfieldBounds>,
) -> Option<MoverSnapshot> {
    let e = entities.get(entity_id)?;
    Some(MoverSnapshot {
        category: e.category,
        speed_type: e.locomotor.as_ref().map(|l| l.speed_type),
        movement_zone: e
            .locomotor
            .as_ref()
            .map(|l| l.movement_zone)
            .unwrap_or(MovementZone::Normal),
        omni_crusher: e.omni_crusher,
        regular_crusher: e.regular_crusher,
        drive_accelerates: e.drive_accelerates,
        owner: e.owner,
        too_big_to_fit_under_bridge: e.too_big_to_fit_under_bridge,
        on_bridge: e.on_bridge,
        runtime_bridge_transition: e.runtime_bridge_transition,
        locomotor: e.locomotor.clone(),
        rot: e.locomotor.as_ref().map(|l| l.rot).unwrap_or(0),
        bypass_grid: e
            .movement_target
            .as_ref()
            .map(|mt| mt.bypass_grid)
            .unwrap_or(false),
        sub_cell_priority_mission: SUB_CELL_PRIORITY_MISSIONS.contains(&e.mission.current().raw()),
        nav_com_cell: e
            .navigation
            .nav_com
            .as_ref()
            .and_then(|nav| nav_target_object_cell(entities, nav)),
        allow_zone_hierarchy: playfield_bounds.is_none() || e.in_playfield,
    })
}

/// Missions whose sub-cell placement bypasses the occupancy, blocker and
/// garrison checks in the original engine: Enter (7), Capture (8), Eaten (9),
/// Area Guard (11), Patrol (25). Anything outside this set takes the ordinary
/// gated placement.
const SUB_CELL_PRIORITY_MISSIONS: [i32; 5] = [7, 8, 9, 11, 25];

/// Resolve the cell of a nav target that is an **object**, for the priority
/// sub-cell placement test only.
///
/// The original reads its destination field as an object pointer and asks the
/// object what it is; priority is granted only when that pointer is live and
/// names a unit-like or building type. A bare destination *cell* is not an
/// object there and never grants priority, so `Cell` resolves to `None` here —
/// otherwise Area Guard and Patrol infantry, which routinely hold cell
/// destinations, would get the occupancy-free, blocker-free, garrison-free
/// placement the original denies them.
fn nav_target_object_cell(entities: &EntityStore, nav: &NavTargetRef) -> Option<(u16, u16)> {
    match *nav {
        NavTargetRef::Cell { .. } => None,
        NavTargetRef::Entity { id }
        | NavTargetRef::Object { id }
        | NavTargetRef::Building { id } => entities.get(id).map(|t| (t.position.rx, t.position.ry)),
    }
}

/// Rebuild one owner's pathfinding entity-block snapshot iff occupancy has
/// mutated since that snapshot was last built. Returns whether a rebuild ran.
///
/// The movement tick builds these snapshots once before the mover loop, but
/// gamemd processes movers in live object order — a mover that repaths after an
/// earlier mover committed a move this tick must see the new position. Gating on
/// the occupancy generation refreshes the snapshot to the live state at repath
/// time (bit-equivalent to per-neighbor live classification for a synchronous A*
/// search) while skipping the no-op case where nothing moved.
#[allow(clippy::too_many_arguments)]
fn refresh_owner_block_set_if_stale(
    entity_block_sets: &mut BTreeMap<
        crate::sim::intern::InternedId,
        (
            BTreeSet<(u16, u16)>,
            crate::sim::pathfinding::LayeredEntityBlockMap,
        ),
    >,
    built_at_gen: &mut BTreeMap<crate::sim::intern::InternedId, u64>,
    owner: crate::sim::intern::InternedId,
    current_gen: u64,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> bool {
    if built_at_gen.get(&owner).copied() == Some(current_gen) {
        return false;
    }
    let owner_str = interner.resolve(owner);
    let pair = bump_crush::build_entity_block_set(entities, owner_str, alliances, interner, rules);
    entity_block_sets.insert(owner, pair);
    built_at_gen.insert(owner, current_gen);
    true
}

/// Result of path exhaustion check — tells the caller how to proceed.
enum PathExhaustionResult {
    /// Path is not yet exhausted — continue to rotation/movement.
    NotExhausted,
    /// Entity was repathed to the next segment — continue to rotation/movement.
    Repathed(Vec<(u32, DebugEventKind)>),
    /// Entity finished its path — caller should `continue` to next entity.
    Finished,
}

/// Check if the current path segment is exhausted and either repath to the next
/// 24-step segment toward the final goal, or mark the entity as finished.
///
/// Also handles the subcell redirect: when the path is exhausted but infantry is
/// still walking toward subcell_dest, redirects move_dir toward the destination.
///
/// Takes individual entity fields to avoid borrow conflicts.
#[allow(clippy::too_many_arguments)]
fn handle_path_exhaustion(
    target: &mut MovementTarget,
    locomotor: &Option<super::locomotor::LocomotorState>,
    drive_locomotion: &mut Option<crate::sim::components::DriveLocomotionRuntime>,
    ship_locomotion: &mut Option<crate::sim::components::ShipLocomotionRuntime>,
    active_ordinary_track: bool,
    position: &super::super::components::Position,
    category: EntityCategory,
    facing: &mut u8,
    facing_target: &mut Option<u8>,
    _entity_id: u64,
    active_layer: MovementLayer,
    snap: &MoverSnapshot,
    ctx: PathfindingContext<'_>,
    entity_cost_grid: Option<&TerrainCostGrid>,
    mover_entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    mover_entity_block_map: Option<&crate::sim::pathfinding::LayeredEntityBlockMap>,
    path_delay_ticks: u16,
    sim_tick: u64,
) -> PathExhaustionResult {
    if target.next_index < target.path.len() || active_ordinary_track {
        // Path not yet exhausted — check subcell redirect case and return.
        return PathExhaustionResult::NotExhausted;
    }

    // Path exhausted — check if at final goal.
    let at_final_goal: bool = target
        .final_goal
        .map_or(true, |fg| (position.rx, position.ry) == fg);
    if !at_final_goal {
        // Auto-repath: compute next 24-step segment toward final_goal.
        let fg = target.final_goal.unwrap(); // safe: at_final_goal was false
        let cur = (position.rx, position.ry);
        let layered_pathing_for_seg = snap
            .locomotor
            .as_ref()
            .zip(ctx.path_grid)
            .is_some_and(|(loco, pg)| supports_layered_bridge_pathing(loco, pg, snap.on_bridge));
        // DIAGNOSTIC: log segment repath when on bridge layer
        if active_layer == MovementLayer::Bridge {
            log::warn!(
                "BRIDGE_DIAG entity={}: path segment exhausted ON BRIDGE at ({},{}) z={} \
                 layered_pathing={} goal=({},{})",
                _entity_id,
                cur.0,
                cur.1,
                position.z,
                layered_pathing_for_seg,
                fg.0,
                fg.1,
            );
        }
        let seg_zone_mz = snap
            .locomotor
            .as_ref()
            .map(|l| l.movement_zone)
            .unwrap_or(MovementZone::Normal);
        if ctx.path_grid.is_some() {
            if let Some((new_path, new_layers)) = find_move_path(
                ctx,
                layered_pathing_for_seg,
                cur,
                active_layer,
                fg,
                entity_cost_grid,
                // Pass the merged entity_blocks set to both layered slots so the
                // layered A* sees building footprints regardless of which layer
                // it expands. Mirrors the try_repath_after_block fix.
                mover_entity_blocks,
                mover_entity_blocks,
                mover_entity_blocks,
                seg_zone_mz,
                Some(snap.movement_zone),
                snap.too_big_to_fit_under_bridge,
                mover_entity_block_map,
                0, // urgency=0: proactive segment repath, no block escalation
                snap.omni_crusher
                    || matches!(
                        snap.locomotor.as_ref().map(|l| l.movement_zone),
                        Some(
                            MovementZone::Crusher
                                | MovementZone::AmphibiousCrusher
                                | MovementZone::CrusherAll
                        )
                    ),
                snap.category == EntityCategory::Infantry,
                snap.allow_zone_hierarchy,
            ) {
                if new_path.len() >= 2 {
                    // DIAGNOSTIC: detect layer mismatch after repath
                    if active_layer == MovementLayer::Bridge {
                        let has_bridge_step =
                            new_layers.iter().any(|l| *l == MovementLayer::Bridge);
                        if !has_bridge_step {
                            log::warn!(
                                "BRIDGE_DIAG entity={}: segment repath produced ALL-GROUND path \
                                 while on bridge! path_len={} — unit will fall through",
                                _entity_id,
                                new_path.len(),
                            );
                        } else {
                            let first_layer =
                                new_layers.get(1).copied().unwrap_or(MovementLayer::Ground);
                            log::info!(
                                "BRIDGE_DIAG entity={}: segment repath OK, first_layer={:?} path_len={}",
                                _entity_id,
                                first_layer,
                                new_path.len(),
                            );
                        }
                    }
                    let saved_speed = target.speed;
                    let saved_goal = target.final_goal;
                    let next = new_path[1];
                    let dx = next.0 as i32 - cur.0 as i32;
                    let dy = next.1 as i32 - cur.1 as i32;
                    let (d_x, d_y, d_len) = crate::util::lepton::cell_delta_to_lepton_dir(dx, dy);
                    // Preserve speed ramping state across segment repath —
                    // the unit is already moving, don't reset to zero.
                    let saved_current = target.current_speed;
                    let saved_accel = target.accel_factor;
                    let saved_decel = target.decel_factor;
                    let saved_slowdown = target.slowdown_distance;
                    let saved_group = target.group_id;
                    *target = MovementTarget {
                        path: new_path,
                        path_layers: new_layers,
                        next_index: 1,
                        speed: saved_speed,
                        current_speed: saved_current,
                        accel_factor: saved_accel,
                        decel_factor: saved_decel,
                        slowdown_distance: saved_slowdown,
                        move_dir_x: d_x,
                        move_dir_y: d_y,
                        move_dir_len: d_len,
                        movement_delay: path_delay_ticks,
                        blocked_delay: 0,
                        path_blocked: false,
                        path_stuck_counter: PATH_STUCK_INIT,
                        final_goal: saved_goal,
                        group_id: saved_group,
                        ignore_terrain_cost: false,
                        bypass_grid: false,
                    };
                    match locomotor.as_ref().map(|locomotor| locomotor.kind) {
                        Some(crate::rules::locomotor_type::LocomotorKind::Drive) => {
                            if let Some(drive) = drive_locomotion.as_mut() {
                                super::path_markers::install_path_replay(
                                    &mut drive.path,
                                    cur,
                                    &target.path,
                                    target.next_index,
                                );
                            }
                        }
                        Some(crate::rules::locomotor_type::LocomotorKind::Ship) => {
                            if let Some(ship) = ship_locomotion.as_mut() {
                                super::path_markers::install_path_replay(
                                    &mut ship.path,
                                    cur,
                                    &target.path,
                                    target.next_index,
                                );
                            }
                        }
                        _ => {}
                    }
                    debug_assert_eq!(
                        target.path.len(),
                        target.path_layers.len(),
                        "path/path_layers desync after segment repath"
                    );
                    // Update facing toward next cell.
                    let new_face: u8 = facing_from_delta(dx, dy);
                    if category == EntityCategory::Infantry || snap.rot <= 0 {
                        *facing = new_face;
                    } else {
                        *facing_target = Some(new_face);
                    }
                    // Continue processing this entity on the new segment.
                    let mut debug_events = Vec::new();
                    debug_events.push((
                        sim_tick as u32,
                        DebugEventKind::Repath {
                            reason: "path segment exhausted".into(),
                            new_path_len: target.path.len(),
                        },
                    ));
                    // After repath, also apply subcell redirect if path is now exhausted
                    // (shouldn't happen with len>=2, but be safe).
                    apply_subcell_redirect(target, locomotor, position);
                    return PathExhaustionResult::Repathed(debug_events);
                } else if !walking_to_subcell_dest(locomotor, position.sub_x, position.sub_y) {
                    return PathExhaustionResult::Finished;
                }
            } else if !walking_to_subcell_dest(locomotor, position.sub_x, position.sub_y) {
                return PathExhaustionResult::Finished;
            }
        } else if !walking_to_subcell_dest(locomotor, position.sub_x, position.sub_y) {
            return PathExhaustionResult::Finished;
        }
    } else if !walking_to_subcell_dest(locomotor, position.sub_x, position.sub_y) {
        return PathExhaustionResult::Finished;
    }

    // Path exhausted but subcell walk still active — redirect move_dir.
    apply_subcell_redirect(target, locomotor, position);
    PathExhaustionResult::NotExhausted
}

/// If path is exhausted but infantry is walking to subcell_dest, redirect
/// move_dir toward the destination so the lepton advancement walks the
/// right direction.
fn apply_subcell_redirect(
    target: &mut MovementTarget,
    locomotor: &Option<super::locomotor::LocomotorState>,
    position: &super::super::components::Position,
) {
    if target.next_index >= target.path.len() {
        if let Some(loco) = locomotor {
            if let Some((dest_x, dest_y)) = loco.subcell_dest {
                let dx: SimFixed = dest_x - position.sub_x;
                let dy: SimFixed = dest_y - position.sub_y;
                target.move_dir_x = dx;
                target.move_dir_y = dy;
                let len: SimFixed = fixed_distance(dx, dy);
                target.move_dir_len = if len > SIM_HALF { len } else { SIM_ONE };
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_pending_drive_arrivals(
    entities: &mut EntityStore,
    entity_order: &[u64],
    ctx: PathfindingContext<'_>,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    entity_block_sets: &BTreeMap<
        crate::sim::intern::InternedId,
        (
            BTreeSet<(u16, u16)>,
            crate::sim::pathfinding::LayeredEntityBlockMap,
        ),
    >,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    cell_occupation: &mut CellOccupationGrid,
) {
    let Some(grid) = ctx.path_grid else {
        super::navcom::process_pending_empty_drive_arrivals_in_order(entities, entity_order);
        return;
    };
    for &entity_id in entity_order {
        let Some(entity) = entities.get_mut(entity_id) else {
            continue;
        };
        if !entity.navigation.pending_arrival_clear {
            continue;
        }
        if entity.movement_target.is_some() || entity.drive_track.is_some() {
            continue;
        }
        // Process-entry rebuild: an owner destination that survived the
        // end-of-track resolution (off-destination finish, or a queued
        // waypoint advanced at arrival) gets a fresh path toward it here —
        // the drive locomotor's no-track process-entry position. Entities
        // deferred with a non-cell owner target fall back to the queue
        // advance, then to the owner-null clear.
        let (rx, ry) = if let Some(NavTargetRef::Cell { rx, ry }) = entity.navigation.nav_com {
            (rx, ry)
        } else if let Some(NavTargetRef::Cell { rx, ry }) =
            entity.navigation.nav_queue.first().copied()
        {
            entity.navigation.nav_queue.remove(0);
            (rx, ry)
        } else {
            super::navcom::set_destination_internal_null(entity);
            continue;
        };
        super::navcom::foot_stop_moving(entity);
        super::navcom::set_destination_internal_cell(entity, (rx, ry), ctx.resolved_terrain);

        let current = (entity.position.rx, entity.position.ry);
        let current_layer = entity.movement_layer_or_ground();
        let Some(loco) = entity.locomotor.as_ref() else {
            // VERA-internal retry policy: `set_destination_internal_cell`
            // above cleared the deferred flag, so bailing out here would
            // strand a live owner destination with no path, no movement, and
            // no retry — a permanent dead-end (callers gate on
            // `nav_com.is_some()`). Re-arm the flag so the next tick retries.
            // The gamemd fallback on a failed process-entry repath is
            // UNCHECKED.
            entity.navigation.pending_arrival_clear = true;
            continue;
        };
        let layered_pathing = supports_layered_bridge_pathing(loco, grid, entity.on_bridge);
        let movement_zone = Some(loco.movement_zone);
        let terrain_cost = terrain_costs.get(&loco.speed_type);
        let (entity_blocks, entity_block_map) = entity_block_sets
            .get(&entity.owner)
            .map(|(b, m)| (Some(b), Some(m)))
            .unwrap_or((None, None));
        let mut occupied_blocks = entity_blocks.cloned().unwrap_or_default();
        occupied_blocks
            .extend(cell_occupation.occupied_cells_ignoring(MovementLayer::Ground, entity_id));
        let occupied_blocks_ref = (!occupied_blocks.is_empty()).then_some(&occupied_blocks);
        let Some((path, path_layers)) = find_move_path(
            ctx,
            layered_pathing,
            current,
            current_layer,
            (rx, ry),
            terrain_cost,
            occupied_blocks_ref,
            occupied_blocks_ref,
            occupied_blocks_ref,
            loco.movement_zone,
            movement_zone,
            entity.too_big_to_fit_under_bridge,
            entity_block_map,
            0,
            entity.omni_crusher
                || matches!(
                    loco.movement_zone,
                    MovementZone::Crusher
                        | MovementZone::AmphibiousCrusher
                        | MovementZone::CrusherAll
                ),
            entity.category == EntityCategory::Infantry,
            ctx.playfield_bounds.is_none() || entity.in_playfield,
        ) else {
            // VERA-internal retry policy: pathfinding failed, so re-arm the
            // deferred flag (cleared by `set_destination_internal_cell`
            // above) and retry next tick toward the surviving owner
            // destination instead of stranding it as a permanent dead-end.
            // The gamemd fallback on a failed process-entry repath is
            // UNCHECKED.
            entity.navigation.pending_arrival_clear = true;
            continue;
        };
        if path.len() < 2 {
            // VERA-internal retry policy, same as the pathfinding-failure
            // branch above; the gamemd equivalent is UNCHECKED.
            entity.navigation.pending_arrival_clear = true;
            continue;
        }
        let obj = rules.and_then(|r| r.object(interner.resolve(entity.type_ref)));
        let speed_multiplier = loco.speed_multiplier;
        // Copy out (Copy type) before `loco`'s borrow of `entity` ends: only
        // Drive-kind movers ride drive-track curve tables below — hover (and
        // any other straight-line mover) must not pick one up on repath.
        let loco_kind = loco.kind;
        // `FootClass::GetCurrentSpeed @ 0x004DB1A0`: the FASTER multiply sits
        // on the truncated per-frame type speed, before the locomotor's own
        // fraction — see `veterancy::veteran_speed_leptons_per_second`.
        let veteran_speed = rules.map_or(1.0, |r| r.general.veteran_speed);
        let speed = (crate::sim::combat::veterancy::mover_speed_leptons_per_second(
            obj.map_or(4, |o| o.speed),
            Some(loco_kind),
            crate::sim::combat::veterancy::rank_of(entity.veterancy_raw),
            obj,
            veteran_speed,
        ) * speed_multiplier)
            .max(SimFixed::lit("25"));
        let dx = path[1].0 as i32 - path[0].0 as i32;
        let dy = path[1].1 as i32 - path[0].1 as i32;
        let (move_dir_x, move_dir_y, move_dir_len) =
            crate::util::lepton::cell_delta_to_lepton_dir(dx, dy);
        let movement = MovementTarget {
            path,
            path_layers,
            next_index: 1,
            speed,
            current_speed: speed,
            accel_factor: obj.map_or(SIM_ZERO, |o| o.accel_factor),
            decel_factor: obj.map_or(SIM_ZERO, |o| o.decel_factor),
            slowdown_distance: obj.map_or(SIM_ZERO, |o| SimFixed::from_num(o.slowdown_distance)),
            move_dir_x,
            move_dir_y,
            move_dir_len,
            final_goal: Some((rx, ry)),
            ..Default::default()
        };
        let mut track_occupation_target = None;
        let mut accepted_path_reference = None;
        let mut accepted_path_nodes: usize = 1;
        if matches!(
            loco_kind,
            crate::rules::locomotor_type::LocomotorKind::Drive
        ) {
            let to_delta =
                movement
                    .path
                    .get(1)
                    .zip(movement.path.get(2))
                    .map(|(&(hx, hy), &(ax, ay))| {
                        (i32::from(ax) - i32::from(hx), i32::from(ay) - i32::from(hy))
                    });
            match super::drive_track::plan_drive_track_from_path(
                entity.facing,
                (dx, dy),
                to_delta,
                false,
            ) {
                super::drive_track::DriveTrackDecision::TurnFirst { desired_facing } => {
                    // Exact-facing precondition: no curve and no step until the
                    // body reaches the head node's octant.
                    entity.drive_track = None;
                    entity.facing_target = Some(desired_facing);
                }
                super::drive_track::DriveTrackDecision::Select(plan) => {
                    entity.drive_track = super::drive_track::begin_selected_drive_track(&plan);
                    if entity.drive_track.is_some() {
                        entity.facing_target = None;
                        accepted_path_nodes = plan.nodes;
                        // `next_index` is 1 here, so the head node index equals
                        // the number of nodes the curve spans.
                        let head_index = plan.nodes;
                        let head_rx = i32::from(entity.position.rx) + plan.head_dx;
                        let head_ry = i32::from(entity.position.ry) + plan.head_dy;
                        accepted_path_reference = Some((head_rx as i16, head_ry as i16));
                        if movement.layer_at(head_index) == MovementLayer::Ground
                            && let (Ok(rx), Ok(ry)) =
                                (u16::try_from(head_rx), u16::try_from(head_ry))
                        {
                            track_occupation_target = Some(DriveOccupationFootprint {
                                rx,
                                ry,
                                layer: MovementLayer::Ground,
                            });
                        }
                    }
                }
                super::drive_track::DriveTrackDecision::Unavailable => {}
            }
        }
        if let Some(drive) = entity.drive_locomotion.as_mut() {
            super::path_markers::install_path_replay(&mut drive.path, current, &movement.path, 1);
            if let Some(reference) = accepted_path_reference {
                super::path_markers::accept_path_replay(
                    &mut drive.path,
                    reference,
                    accepted_path_nodes,
                );
            }
            match track_occupation_target {
                Some(next) => crate::sim::occupancy::replace_drive_head_to_occupation(
                    drive,
                    cell_occupation,
                    entity_id,
                    current,
                    current_layer,
                    next,
                ),
                None => crate::sim::occupancy::clear_drive_head_to_occupation_for_replacement(
                    drive,
                    cell_occupation,
                    entity_id,
                    current,
                    current_layer,
                ),
            }
        }
        entity.movement_target = Some(movement);
    }
}

/// `Can_Enter_Cell` code for an allied body that is standing still in the cell.
/// The one code the selection gate can produce that does NOT share the entry at
/// 0x004B3607: `CMP EDX,0x6 / JNZ 0x004B3944` at 0x004B36F4 splits it out.
const CODE_FRIENDLY_STATIONARY: u8 = 6;

/// Dispatch for a fresh Drive curve refused with a code that shares gamemd's
/// entry at 0x004B3607 — a bare occupation-mask claim (code 2), a moving
/// friendly (2), or an enemy (5). **Code 6 does not come here**; it has its own
/// retail arm with a scatter in it and is routed to the classifying lane
/// instead, at the call site.
///
/// For the mask there is exactly one arm and nothing to classify. The mask is
/// the last-resort arm of `Can_Enter_Cell` — `TEST EBP,EBP; JNZ 0x0073FD37` at
/// 0x0073FC24 skips it whenever the object-list walk already produced a code —
/// and when its vehicle bit is what refuses, the answer is the literal 2 written
/// at 0x0073FD32. So the code the gate decided with is the code this consumes;
/// no second predicate exists to disagree with it.
///
/// Code 5 belongs here rather than in the crossing lane's attack-the-blocker
/// arm: for a Drive mover gamemd has no blocking-object arm at all. Every code
/// that is not 1, 3 or 6 falls through `0x004B3944 CMP EDX,0x1 /
/// JNZ 0x004B3607` into this same entry, and the ten call sites of the
/// mission-override vtable slot give the *walk* locomotor the only
/// blocking-object one. A blocked vehicle repaths; it does not stop and shoot.
///
/// gamemd's code-2 arm, read end to end from dispatch site B (`Can_Enter_Cell`
/// call at 0x004B34C0):
///
/// * 0x004B36F7 `JNZ 0x004B3944`, then 0x004B3947 `JNZ 0x004B3607` — codes 2, 4,
///   5 and 7 share this entry;
/// * 0x004B3607-0x004B3646 nulls the Drive head-to coordinate from the null
///   triple at 0x008A0790 (VERA does this inside the gate, before returning);
/// * 0x004B364D `CMP EAX,0x2 / JNZ 0x004B3A97` selects the code-2 arm;
/// * 0x004B3659-0x004B368D raises the mover's blocked flag `Foot+0x6B7` ONCE and,
///   only on that first pass, arms its escalation timer with `Rules+0x1768`;
/// * 0x004B3690-0x004B36B6 returns early while the movement-delay timer
///   (`Foot+0x640` start / `+0x648` duration) is still running;
/// * 0x004B36BC-0x004B36EF sets `BL` from whether the escalation timer expired,
///   0x004B39FB `TEST BL,BL / SETNZ DL / INC EDX` turns that into 1 or 2, and
///   0x004B3A0E calls `FootClass::Find_Path(cell, 0, urgency)`.
///
/// **There is no `Scatter_Objects` call anywhere in the arm.** The movement
/// body's cell scatters all sit in code-6 ladders — 0x004B3225 in the blocked
/// block, and 0x004B393A in the code-6 dispatch arm itself. A previous revision
/// of this file claimed the arm contained no CALL at all; that is true only of
/// the range 0x004B3656-0x004B36EF, and the arm exits that range via
/// `JMP 0x004B39D3` into the `Find_Path` call above.
///
/// `handle_blocked_tick` already IS that ladder: it arms `blocked_delay` once on
/// the first blocked pass and never re-arms it, so urgency reads 1 while
/// `BlockagePathDelay` runs and 2 thereafter, and it repaths on every pass the
/// movement-delay limiter allows. Nothing else belongs here — in particular no
/// scatter, and therefore no scenario-RNG draw, on the most common event this
/// gate produces.
#[allow(clippy::too_many_arguments)]
fn handle_deferred_drive_selection_block(
    entities: &mut EntityStore,
    entity_id: u64,
    snap: &MoverSnapshot,
    active_layer: MovementLayer,
    ctx: PathfindingContext<'_>,
    mcfg: MovementConfig,
    entity_cost_grid: Option<&TerrainCostGrid>,
    mover_entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    mover_entity_block_map: Option<&crate::sim::pathfinding::LayeredEntityBlockMap>,
    occupancy: &OccupancyGrid,
    rng: &mut SimRng,
    stats: &mut MovementTickStats,
    finished_entities: &mut Vec<u64>,
    sim_tick: u64,
    marker_context: Option<crate::sim::movement::path_markers::BridgeMarkerContext<'_>>,
) -> Vec<(u32, DebugEventKind)> {
    let Some(entity) = entities.get_mut(entity_id) else {
        return Vec::new();
    };
    let cur_pos = (entity.position.rx, entity.position.ry);
    let body_facing = entity.body_facing;
    let Some(ref mut target) = entity.movement_target else {
        return Vec::new();
    };
    let mut aborted_for_stuck = false;
    handle_blocked_tick(
        target,
        &mut entity.facing,
        body_facing,
        &snap.locomotor,
        &mut entity.drive_locomotion,
        &mut entity.ship_locomotion,
        entity_id,
        cur_pos,
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
        bump_crush::CrushCapability::new(snap.regular_crusher, snap.omni_crusher).can_crush_units(),
        snap.category == EntityCategory::Infantry,
        snap.allow_zone_hierarchy,
        // Code 2 keeps its grace span; the escalation timer is what selects the
        // repath urgency (0x004B36BC-0x004B36EF).
        false,
        // All five `Rules+0x1718` give-up compares in the movement body sit
        // outside the code-2 dispatch.
        false,
        marker_context,
        occupancy,
    )
}

#[derive(Debug, Clone, Copy)]
struct DeferredDriveTrackChain {
    target_cell: (u16, u16),
    layers: cell_entry::CanEnterLayerContext,
    bridge_traversal_allowed: bool,
    cur_face: u8,
    next_face: u8,
}

#[allow(clippy::too_many_arguments)]
fn classify_drive_track_chain_entry(
    chain: DeferredDriveTrackChain,
    entity_id: u64,
    snap: &MoverSnapshot,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_cost_grid: Option<&TerrainCostGrid>,
    occupancy: &OccupancyGrid,
    cell_occupation: &CellOccupationGrid,
    live_building_entry_skips: &super::movement_occupancy::LiveBuildingEntrySkipMap,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    if !chain.bridge_traversal_allowed {
        return CellEntryResult::Impassable;
    }

    let (x, y) = chain.target_cell;
    let terrain_clear = match chain.layers.terrain_layer {
        // Category-aware for the same reason as the plain runtime crossing: the
        // search and the step-in gate must be one predicate.
        MovementLayer::Ground => path_grid.map_or(true, |grid| {
            crate::sim::pathfinding::is_cell_passable_for_category_on_layer(
                grid,
                x,
                y,
                MovementLayer::Ground,
                Some(snap.movement_zone),
                snap.speed_type,
                resolved_terrain,
                entity_cost_grid,
                snap.bypass_grid,
                TerrainEntryMode::RuntimeTransition,
                snap.category == EntityCategory::Infantry,
            )
        }),
        MovementLayer::Bridge => path_grid.is_some_and(|grid| {
            crate::sim::pathfinding::is_cell_passable_for_mover_on_layer_with_speed(
                grid,
                x,
                y,
                MovementLayer::Bridge,
                Some(snap.movement_zone),
                snap.speed_type,
                resolved_terrain,
                entity_cost_grid,
                snap.bypass_grid,
                TerrainEntryMode::RuntimeTransition,
            )
        }),
        MovementLayer::Air | MovementLayer::Underground => false,
    };
    if !terrain_clear {
        return CellEntryResult::Impassable;
    }

    if !has_unignored_runtime_occupants_on_layers(
        occupancy,
        chain.target_cell,
        chain.layers,
        live_building_entry_skips,
    ) && !cell_occupation.occupied_by_other(
        chain.target_cell.0,
        chain.target_cell.1,
        chain.layers.occupancy_bits_layer,
        entity_id,
    ) {
        return CellEntryResult::Clear;
    }

    let mover_loco_kind = snap
        .locomotor
        .as_ref()
        .map_or(crate::rules::locomotor_type::LocomotorKind::Drive, |l| {
            l.kind
        });
    cell_entry::classify_occupied_cell_with_layers_and_ignored_and_occupation(
        chain.target_cell,
        chain.layers,
        entity_id,
        bump_crush::CrushCapability::new(snap.regular_crusher, snap.omni_crusher),
        interner.resolve(snap.owner),
        mover_loco_kind,
        snap.bypass_grid,
        live_building_entry_skips.get(&chain.target_cell),
        occupancy,
        cell_occupation,
        entities,
        alliances,
        interner,
    )
}

/// May the chained curve be installed for this cell-entry answer?
///
/// Only code 0 (and the crush case, which resolves to code 0 once the victims
/// are killed) admits a step. Every non-zero code takes a per-code branch in the
/// movement body instead — `CMP EAX,0x3 / CMP EAX,0x6` at 0x004B2FFF-0x004B3027,
/// with all remaining non-zero codes falling into the shared wait tail at
/// 0x004B3282 — and none of those branches commits a crossing.
///
/// The two temporary codes used to be admitted here, and that is the defect this
/// gate closes: `TemporaryOccupation` is the mask arm's code 2 (another mover
/// has stamped this cell), and `TemporaryBlock` is a body still standing in the
/// cell. gamemd refuses both for a vehicle occupant. Its object-list walk reads
/// the occupant's `Foot+0x6B6` at 0x0073FA30 and branches
/// (0x0073FA30-0x0073FA7C):
///
/// * `+0x6B6 == 0` -> query the occupant's locomotor slot `+0xA4`; false skips
///   the occupant entirely (0x0073FA6B falls to 0x0073FA7C), true raises the
///   code;
/// * `+0x6B6 != 0` and RTTI (`vtable+0x2C`) **is** `0x0F` -> the SAME locomotor
///   query, so those occupants can still be skipped (RTTI-to-class binding
///   UNCHECKED);
/// * `+0x6B6 != 0` and RTTI is anything else -> straight to the raise.
///
/// The raise at 0x0073FA74 is a running MAX, not an unconditional store: it is
/// guarded by `CMP [ESP+0x18],0x2 / JGE 0x0073FA7C` at 0x0073FA6D, so it only
/// lifts an accumulator that is still below 2.
///
/// Admitting the chain on those codes let a follower install a curve into, and
/// then drive onto, a cell another vehicle was standing in, so refusing is the
/// right direction. But VERA refuses on BOTH branches, and gamemd does not.
///
/// **CONFIRMED DRIFT — VERA is stricter than gamemd for an occupant in
/// transit.** An earlier revision of this comment argued that a vehicle always
/// reaches the raise because `FootClass`'s constructor initialises `+0x6B6` to 1
/// (`MOV byte ptr [ESI+0x6B6],0x1` at 0x004D344A). The constructor store is
/// real, but it is the IDLE value, not an invariant: an exhaustive writer search
/// finds each locomotor clearing and re-setting the same byte around its own
/// motion — `DriveLocomotionClass__Process_Drive_Track @ 0x004B0F20` writes 0 at
/// 0x004B161A and 1 at 0x004B1FEF, `ShipLocomotionClass__Process_Drive_Track`
/// 0x006A0CDA / 0x006A1632, `HoverLocomotionClass__Move` 0x005147D5 /
/// 0x0051451E. A moving vehicle therefore carries 0 and takes the very same
/// locomotor `+0xA4` question at 0x0073FA46; when that answers false, gamemd
/// skips the occupant and admits the follower.
///
/// So on the arrival phase of a group move, where the cell ahead is held by a
/// peer that is itself still moving, gamemd can chain through and VERA cannot.
/// Player effect: columns space out slightly more than retail and take marginally
/// longer to close up. Frequency: every multi-unit move. VERA models neither
/// `+0x6B6` nor the locomotor `+0xA4` slot, so closing this needs both; gamemd
/// equivalent of the `+0xA4` predicate itself is UNCHECKED.
fn drive_track_chain_entry_allows_track_install(entry_result: &CellEntryResult) -> bool {
    matches!(
        entry_result,
        CellEntryResult::Clear | CellEntryResult::Crushable { .. }
    )
}

fn drive_track_chain_check_crushable_obstacle(
    entities: &mut EntityStore,
    occupancy: &OccupancyGrid,
    chain: DeferredDriveTrackChain,
    entity_id: u64,
    snap: &MoverSnapshot,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> bool {
    let Some(rules) = rules else {
        return false;
    };
    crate::sim::gate_runtime::request_gate_open_for_cell(
        entities,
        occupancy,
        chain.target_cell,
        chain.layers.object_list_layer,
        entity_id,
        interner.resolve(snap.owner),
        rules,
        alliances,
        interner,
    )
}

#[allow(clippy::too_many_arguments)]
fn handle_deferred_drive_track_chain(
    entities: &mut EntityStore,
    entity_id: u64,
    snap: &MoverSnapshot,
    chain: DeferredDriveTrackChain,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_cost_grid: Option<&TerrainCostGrid>,
    occupancy: &mut OccupancyGrid,
    cell_occupation: &mut CellOccupationGrid,
    live_building_entry_skips: &super::movement_occupancy::LiveBuildingEntrySkipMap,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    rng: &mut SimRng,
    stats: &mut MovementTickStats,
    crush_kills: &mut Vec<PendingCrushKill>,
    already_scattered: &mut BTreeSet<u64>,
    sim_tick: u64,
) -> bool {
    let entry_result = classify_drive_track_chain_entry(
        chain,
        entity_id,
        snap,
        path_grid,
        resolved_terrain,
        entity_cost_grid,
        occupancy,
        cell_occupation,
        live_building_entry_skips,
        entities,
        alliances,
        interner,
    );
    let install_chain = drive_track_chain_entry_allows_track_install(&entry_result);

    match entry_result {
        CellEntryResult::Clear
        | CellEntryResult::TemporaryBlock { .. }
        | CellEntryResult::TemporaryOccupation => {}
        CellEntryResult::ScatterRequired { .. } => {
            drive_track_chain_check_crushable_obstacle(
                entities, occupancy, chain, entity_id, snap, rules, alliances, interner,
            );
        }
        CellEntryResult::Crushable { victims } => {
            let crusher_cell = (
                i32::from(chain.target_cell.0),
                i32::from(chain.target_cell.1),
            );
            let crusher_lepton = (
                i32::from(chain.target_cell.0) * 256 + 128,
                i32::from(chain.target_cell.1) * 256 + 128,
            );
            let victims = match bump_crush::classify_drive_crush_phase(
                bump_crush::DriveCrushPhase::FullyInCell,
                &victims,
                entities,
                entity_id,
                alliances,
                interner,
                crusher_lepton,
                bump_crush::CrushCapability::new(snap.regular_crusher, snap.omni_crusher),
                bump_crush::ScatterEligibility::from_rules(rules),
                sim_tick as u32,
            ) {
                bump_crush::DriveCrushOutcome::Kill { victims } => victims,
                _ => Vec::new(),
            };
            for &victim_id in &victims {
                if let Some((rx, ry)) = entities
                    .get(victim_id)
                    .map(|victim| (victim.position.rx, victim.position.ry))
                {
                    occupancy.remove(rx, ry, victim_id);
                }
                if let Some(victim) = entities.get_mut(victim_id) {
                    // This forced-drive path shares the same UNCHECKED
                    // pre-UnInit unmark timing as the ordinary crush path.
                    if let Some(drive) = victim.drive_locomotion.as_mut() {
                        crate::sim::occupancy::clear_drive_head_to_occupation_for_remove(
                            drive,
                            cell_occupation,
                            victim_id,
                        );
                    }
                    victim.lifecycle.cell_marked = false;
                    cell_occupation.reconcile_entity(victim);
                }
            }
            crush_kills.extend(victims.into_iter().map(|victim_id| PendingCrushKill {
                victim_id,
                crusher_id: entity_id,
                crush_coord: crusher_cell,
            }));
        }
        CellEntryResult::FriendlyStationary { blocker_id } => {
            let blocker_fraidycat =
                bump_crush::blocker_is_fraidycat(entities, blocker_id, rules, interner);
            if !already_scattered.contains(&blocker_id)
                && bump_crush::scatter_blocker(
                    entities,
                    blocker_id,
                    path_grid,
                    occupancy,
                    chain.layers.object_list_layer,
                    rng,
                    rules.map(|r| &r.mission_control),
                    blocker_fraidycat,
                )
            {
                already_scattered.insert(blocker_id);
                stats.scatter_successes = stats.scatter_successes.saturating_add(1);
            }
        }
        CellEntryResult::FriendlyWall
        | CellEntryResult::OccupiedEnemy { .. }
        | CellEntryResult::Impassable => {
            return false;
        }
    }
    if !install_chain {
        return false;
    }

    let Some(sel) = super::drive_track::select_drive_track(chain.cur_face, chain.next_face, false)
    else {
        return false;
    };
    let Some(entity) = entities.get_mut(entity_id) else {
        return false;
    };
    let chain_dx = chain.target_cell.0 as i32 - entity.position.rx as i32;
    let chain_dy = chain.target_cell.1 as i32 - entity.position.ry as i32;
    let Some(new_track) = super::drive_track::begin_drive_track(
        sel.raw_track_index,
        sel.flags,
        chain_dx,
        chain_dy,
        sel.target_facing,
    ) else {
        return false;
    };
    let current_cell = (entity.position.rx, entity.position.ry);
    // Same pair of claims the fresh selection installs: the forward RawTrack
    // handoff cell this curve passes through, then its head cell.
    //
    // The pair stays on ONE plane — the head mark's, resolved just below from
    // `chain.layers.occupancy_bits_layer` — rather than pinning the handoff to
    // Ground while the head follows the layer context. `Apply_Track_Occupation_Mode`
    // applies one mode to both coordinates and its mark helper picks the plane
    // from each coordinate's own height, which VERA has no equivalent of here;
    // splitting them would let a mover on a bridge deck claim a ground cell it
    // is not on and block a ground mover underneath. Deck equivalent UNCHECKED.
    let handoff = (chain.layers.occupancy_bits_layer == MovementLayer::Ground)
        .then(|| {
            super::drive_track::is_at_coord_track_cells(&new_track, current_cell, true)
                .0
                .and_then(|(hx, hy)| {
                    Some(DriveOccupationFootprint {
                        rx: u16::try_from(hx).ok()?,
                        ry: u16::try_from(hy).ok()?,
                        layer: MovementLayer::Ground,
                    })
                })
        })
        .flatten();
    entity.drive_track = Some(new_track);
    let current_layer = entity
        .occupancy_list_layer()
        .unwrap_or(MovementLayer::Ground);
    if let Some(drive) = entity.drive_locomotion.as_mut() {
        super::path_markers::accept_path_replay(
            &mut drive.path,
            (chain.target_cell.0 as i16, chain.target_cell.1 as i16),
            1,
        );
        let next = (chain.layers.occupancy_bits_layer == MovementLayer::Ground).then_some(
            DriveOccupationFootprint {
                rx: chain.target_cell.0,
                ry: chain.target_cell.1,
                layer: MovementLayer::Ground,
            },
        );
        match next {
            Some(next) => crate::sim::occupancy::replace_drive_head_to_occupation(
                drive,
                cell_occupation,
                entity_id,
                current_cell,
                current_layer,
                next,
            ),
            None => crate::sim::occupancy::clear_drive_head_to_occupation_for_replacement(
                drive,
                cell_occupation,
                entity_id,
                current_cell,
                current_layer,
            ),
        }
        crate::sim::occupancy::replace_drive_handoff_occupation(
            drive,
            cell_occupation,
            entity_id,
            current_cell,
            current_layer,
            handoff,
        );
    }
    true
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn tick_movement_with_grids(
    entities: &mut EntityStore,
    live_order: Option<&[u64]>,
    path_grid: Option<&PathGrid>,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    alliances: &HouseAllianceMap,
    occupancy: &mut OccupancyGrid,
    cell_occupation: &mut CellOccupationGrid,
    raw_cell_occupation: &mut RawCellOccupationGrid,
    next_occupancy_enter_order: &mut EnterOrderCounter,
    rng: &mut SimRng,
    sim_tick: u64,
    native_frame: u32,
    zone_grid: Option<&ZoneGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    playfield_bounds: Option<PlayfieldBounds>,
    terrain_speed_config: &TerrainSpeedConfig,
    close_enough: SimFixed,
    path_delay_ticks: u16,
    blockage_path_delay_ticks: u16,
    interner: &mut crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
    lifecycle_requests: &mut Vec<LifecycleRequest>,
) -> MovementTickStats {
    tick_movement_with_grids_scoped(
        entities,
        live_order,
        path_grid,
        terrain_costs,
        alliances,
        occupancy,
        cell_occupation,
        raw_cell_occupation,
        next_occupancy_enter_order,
        rng,
        sim_tick,
        native_frame,
        zone_grid,
        resolved_terrain,
        None,
        None,
        playfield_bounds,
        terrain_speed_config,
        close_enough,
        path_delay_ticks,
        blockage_path_delay_ticks,
        interner,
        rules,
        sound_events,
        lifecycle_requests,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn tick_movement_object_with_grids(
    entities: &mut EntityStore,
    entity_id: u64,
    path_grid: Option<&PathGrid>,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    alliances: &HouseAllianceMap,
    occupancy: &mut OccupancyGrid,
    cell_occupation: &mut CellOccupationGrid,
    raw_cell_occupation: &mut RawCellOccupationGrid,
    next_occupancy_enter_order: &mut EnterOrderCounter,
    rng: &mut SimRng,
    sim_tick: u64,
    native_frame: u32,
    zone_grid: Option<&ZoneGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    overlay_grid: Option<&crate::sim::overlay_grid::OverlayGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    playfield_bounds: Option<PlayfieldBounds>,
    terrain_speed_config: &TerrainSpeedConfig,
    close_enough: SimFixed,
    path_delay_ticks: u16,
    blockage_path_delay_ticks: u16,
    interner: &mut crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
    lifecycle_requests: &mut Vec<LifecycleRequest>,
) -> MovementTickStats {
    tick_movement_with_grids_scoped(
        entities,
        Some(std::slice::from_ref(&entity_id)),
        path_grid,
        terrain_costs,
        alliances,
        occupancy,
        cell_occupation,
        raw_cell_occupation,
        next_occupancy_enter_order,
        rng,
        sim_tick,
        native_frame,
        zone_grid,
        resolved_terrain,
        overlay_grid,
        overlay_registry,
        playfield_bounds,
        terrain_speed_config,
        close_enough,
        path_delay_ticks,
        blockage_path_delay_ticks,
        interner,
        rules,
        sound_events,
        lifecycle_requests,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn tick_movement_with_grids_scoped(
    entities: &mut EntityStore,
    live_order: Option<&[u64]>,
    path_grid: Option<&PathGrid>,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    alliances: &HouseAllianceMap,
    occupancy: &mut OccupancyGrid,
    cell_occupation: &mut CellOccupationGrid,
    raw_cell_occupation: &mut RawCellOccupationGrid,
    next_occupancy_enter_order: &mut EnterOrderCounter,
    rng: &mut SimRng,
    sim_tick: u64,
    native_frame: u32,
    zone_grid: Option<&ZoneGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    overlay_grid: Option<&crate::sim::overlay_grid::OverlayGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    playfield_bounds: Option<PlayfieldBounds>,
    terrain_speed_config: &TerrainSpeedConfig,
    close_enough: SimFixed,
    path_delay_ticks: u16,
    blockage_path_delay_ticks: u16,
    interner: &mut crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
    lifecycle_requests: &mut Vec<LifecycleRequest>,
    single_object: bool,
) -> MovementTickStats {
    let mut stats = MovementTickStats::default();
    if live_order.is_some_and(|order| order.is_empty()) {
        // An explicitly supplied empty LogicVector is authoritative. It is not
        // the test-wrapper signal for deriving stable-id order from storage.
        return stats;
    }
    let blocker_neighbor_counts = path_grid.map(|grid| {
        bump_crush::build_blocker_neighbor_counts_with_overlays(
            entities,
            grid.width(),
            grid.height(),
            resolved_terrain,
            overlay_grid,
            overlay_registry,
            interner,
            rules,
        )
    });
    let ctx = PathfindingContext {
        path_grid,
        zone_grid,
        resolved_terrain,
        playfield_bounds,
        blocker_neighbor_counts: blocker_neighbor_counts.as_ref(),
    };
    let mcfg = MovementConfig {
        close_enough,
        path_delay_ticks,
        blockage_path_delay_ticks,
    };
    let dt = native_movement_frame_fraction();
    let fallback_order;
    let entity_order: &[u64] = match live_order {
        Some(order) => order,
        None => {
            fallback_order = entities.keys_sorted();
            &fallback_order
        }
    };
    for &entity_id in entity_order {
        if let Some(entity) = entities.get(entity_id) {
            cell_occupation.reconcile_entity(entity);
        }
    }
    // Collect entities that have finished their paths (need movement_target removal after loop).
    let mut finished_entities: Vec<u64> = Vec::new();
    // Deferred effects — applied after the movement loop to avoid borrow conflicts.
    let mut crush_kills: Vec<PendingCrushKill> = Vec::new();
    // Track which blockers have already been told to scatter this tick,
    // preventing duplicate scatter commands from multiple movers.
    let mut already_scattered: BTreeSet<u64> = BTreeSet::new();

    // Active TubeMovement owns the entire object turn. Capture this before any
    // helper can mutate navigation state, because a successful final clears
    // the payload but still must not resume ordinary processing this tick.
    let tube_active_at_start: BTreeSet<u64> = entity_order
        .iter()
        .copied()
        .filter(|&entity_id| {
            entities
                .get(entity_id)
                .is_some_and(|entity| entity.low_bridge_tube_state.is_some())
        })
        .collect();

    // DriveLocomotionClass::Process @ 0x004B0500 samples CellClass+0x11C at
    // 0x004B050B..0x004B0557 before its first track/movement branch;
    // ShipLocomotionClass::Process @ 0x0069FC10 does the same at
    // 0x0069FC1B..0x0069FC67. Entry-active Tube owns the whole object turn and
    // is the only exclusion here. Stationary eligible objects still Process.
    if let Some(terrain) = resolved_terrain {
        for &entity_id in entity_order {
            if tube_active_at_start.contains(&entity_id) {
                continue;
            }
            let sampled_slope = entities.get(entity_id).and_then(|entity| {
                terrain
                    .cell(entity.position.rx, entity.position.ry)
                    .map(|cell| cell.slope_type)
            });
            if let Some(sampled_slope) = sampled_slope
                && let Some(entity) = entities.get_mut(entity_id)
            {
                super::slope_transition::sample_process_entry(entity, sampled_slope, native_frame);
            }
        }
    }

    let drive_reaims: Vec<(u64, crate::sim::components::DriveCoord)> =
        drive_locomotion::drive_entity_nav_targets(entities)
            .into_iter()
            .filter(|(mover_id, _)| entity_order.contains(mover_id))
            .filter(|(mover_id, _)| !tube_active_at_start.contains(mover_id))
            .filter_map(|(mover_id, target)| {
                super::navcom::resolve_entity_nav_target_drive_coord(target, entities)
                    .map(|coord| (mover_id, coord))
            })
            .collect();
    for (mover_id, coord) in drive_reaims {
        if let Some(entity) = entities.get_mut(mover_id) {
            drive_locomotion::refresh_drive_head_to_coord(entity, coord);
        }
    }

    let mut tube_processed = tube_active_at_start;
    if let Some(terrain) = resolved_terrain {
        for &entity_id in entity_order {
            if tube_movement::tick_active_tube_object(
                entities,
                entity_id,
                terrain,
                path_grid,
                occupancy,
                cell_occupation,
                raw_cell_occupation,
                next_occupancy_enter_order,
                rules,
                interner,
                rng,
                native_frame,
            ) {
                tube_processed.insert(entity_id);
                stats.movers_total = stats.movers_total.saturating_add(1);
            }
        }
    }
    let forced_drive_processed = tick_forced_drive_tracks(
        entities,
        entity_order,
        &tube_processed,
        occupancy,
        cell_occupation,
        next_occupancy_enter_order,
        dt,
        &mut stats,
    );
    let ordinary_entry_order: Vec<u64> = entity_order
        .iter()
        .copied()
        .filter(|entity_id| !tube_processed.contains(entity_id))
        .collect();

    // Collect movers in live object order: ground/bridge entities with a movement_target.
    let mut movers: Vec<u64> = Vec::new();
    let mut mover_owners: BTreeSet<crate::sim::intern::InternedId> = BTreeSet::new();
    for &id in entity_order {
        if let Some(entity) = entities.get(id) {
            let _ = drive_locomotion::process_drive_locomotion_shell(entity);
            if entity.navigation.pending_arrival_clear {
                mover_owners.insert(entity.owner);
            }
            if forced_drive_processed.contains(&id)
                || tube_processed.contains(&id)
                || entity.movement_target.is_none()
                || entity.low_bridge_tube_state.is_some()
            {
                continue;
            }
            let layer = entity.movement_layer_or_ground();
            if !matches!(layer, MovementLayer::Air | MovementLayer::Underground) {
                movers.push(id);
                mover_owners.insert(entity.owner);
            }
        }
    }
    // Pre-build entity block sets per owner for friendly-passable pathfinding during repath.
    // RA2 optimization: moving friendly units are passable (code-2 dynamic cost);
    // only stationary/enemy units hard-block. InternedId is Copy, so keys are cheap.
    let mut entity_block_sets: BTreeMap<
        crate::sim::intern::InternedId,
        (
            BTreeSet<(u16, u16)>,
            crate::sim::pathfinding::LayeredEntityBlockMap,
        ),
    > = mover_owners
        .iter()
        .map(|&owner_id| {
            let owner_str = interner.resolve(owner_id);
            let pair =
                bump_crush::build_entity_block_set(entities, owner_str, alliances, interner, rules);
            (owner_id, pair)
        })
        .collect();
    // Occupancy generation these snapshots reflect. Captured before
    // process_pending_drive_arrivals so any move it makes advances the generation
    // and forces the first consuming mover to rebuild. Each owner's snapshot is
    // lazily refreshed in the mover loop below whenever occupancy changed since it
    // was last built (gamemd processes movers in live object order).
    let block_set_build_gen = occupancy.generation();
    let mut block_set_built_at_gen: BTreeMap<crate::sim::intern::InternedId, u64> =
        entity_block_sets
            .keys()
            .map(|&owner| (owner, block_set_build_gen))
            .collect();

    process_pending_drive_arrivals(
        entities,
        &ordinary_entry_order,
        ctx,
        terrain_costs,
        &entity_block_sets,
        interner,
        rules,
        cell_occupation,
    );
    movers.clear();
    for &id in entity_order {
        if let Some(entity) = entities.get(id) {
            if forced_drive_processed.contains(&id)
                || tube_processed.contains(&id)
                || entity.movement_target.is_none()
                || entity.low_bridge_tube_state.is_some()
            {
                continue;
            }
            let layer = entity.movement_layer_or_ground();
            if !matches!(layer, MovementLayer::Air | MovementLayer::Underground) {
                movers.push(id);
            }
        }
    }

    for entity_id in movers {
        if contains_crush_victim(&crush_kills, entity_id) {
            continue;
        }
        stats.movers_total = stats.movers_total.saturating_add(1);

        // Snapshot mover data before entering the inner loop so we can release the
        // mutable borrow on `entities` when needed for crush/bump immutable lookups.
        let Some(snap) = snapshot_mover(entities, entity_id, playfield_bounds) else {
            continue;
        };
        let prone_crawls = entities.get(entity_id).and_then(|entity| {
            if !infantry::is_prone_for_damage(entity) {
                return None;
            }
            let rules = rules?;
            let obj = rules.object(interner.resolve(entity.type_ref))?;
            Some(obj.crawls)
        });
        let entity_cost_grid: Option<&TerrainCostGrid> =
            snap.speed_type.and_then(|st| terrain_costs.get(&st));
        // Slice 6: refresh this owner's pathfinding snapshot if occupancy changed
        // since it was built (e.g. an earlier mover committed a move this tick).
        // Matches gamemd's live-order processing; no-op when nothing moved. Must run
        // before the immutable refs below borrow `entity_block_sets`.
        refresh_owner_block_set_if_stale(
            &mut entity_block_sets,
            &mut block_set_built_at_gen,
            snap.owner,
            occupancy.generation(),
            entities,
            alliances,
            interner,
            rules,
        );
        let (mover_entity_blocks, mover_entity_block_map): (
            Option<&BTreeSet<(u16, u16)>>,
            Option<&crate::sim::pathfinding::LayeredEntityBlockMap>,
        ) = entity_block_sets
            .get(&snap.owner)
            .map(|(b, m)| (Some(b), Some(m)))
            .unwrap_or((None, None));
        let live_building_entry_skips =
            build_live_building_entry_skip_map(entities, entity_id, interner, rules);
        let marker_peers = snapshot_bridge_marker_peers(entities, rules, interner);
        let marker_context;

        let mut aborted_for_stuck: bool = false;
        let mut active_layer: MovementLayer;
        let mut debug_events: Vec<(u32, DebugEventKind)> = Vec::new();
        let mut pending_bridge_update: BridgeStateUpdate = BridgeStateUpdate::Unchanged;
        // Vehicle crush/bump needs immutable EntityStore access, which conflicts
        // with the mutable entity borrow. When detected, we save the target cell
        // and layer, break out of the while loop, release the borrow, then handle
        // the check in a separate scope below.
        let mut deferred_cell_check: Option<DeferredCellCheck> = None;
        let mut deferred_drive_track_chain: Option<DeferredDriveTrackChain> = None;
        let mut deferred_drive_selection_block: Option<movement_step::DriveSelectionRefusal> = None;
        let mut already_finished: bool = false;

        // Scoped mutable borrow of the entity — released at block end so the
        // vehicle crush/bump check below can do immutable EntityStore lookups.
        {
            let Some(entity) = entities.get_mut(entity_id) else {
                continue;
            };
            // S4a (Option B): the per-object mission dispatch (`+0xC4` tick
            // counter + `derived_mission` commit) was relocated to the object-AI
            // host stage (pre-movement, LogicVector order), so it no longer
            // happens here. The arrival-tick value is preserved: the host commits
            // `Move` before this loop clears the target on arrival.
            active_layer = entity.movement_layer_or_ground();
            let marker_body_facing = entity.body_facing;
            let Some(ref mut target) = entity.movement_target else {
                continue;
            };
            target.movement_delay = target.movement_delay.saturating_sub(1);
            target.blocked_delay = target.blocked_delay.saturating_sub(1);

            match handle_path_exhaustion(
                target,
                &entity.locomotor,
                &mut entity.drive_locomotion,
                &mut entity.ship_locomotion,
                entity.drive_track.is_some(),
                &entity.position,
                entity.category,
                &mut entity.facing,
                &mut entity.facing_target,
                entity_id,
                active_layer,
                &snap,
                ctx,
                entity_cost_grid,
                mover_entity_blocks,
                mover_entity_block_map,
                path_delay_ticks,
                sim_tick,
            ) {
                PathExhaustionResult::Finished => {
                    finished_entities.push(entity_id);
                    continue;
                }
                PathExhaustionResult::Repathed(evts) => {
                    debug_events.extend(evts);
                }
                PathExhaustionResult::NotExhausted => {}
            }

            if let Some(tube_id) = tube_movement::pending_path_tube_id(
                target,
                &entity.position,
                active_layer,
                resolved_terrain,
            ) {
                let exit = target.path.get(target.next_index).copied();
                let terrain = resolved_terrain.expect("tube admission resolved terrain");
                if tube_movement::begin_path_tube_step(
                    entity_id,
                    entity.category,
                    &mut entity.position,
                    &mut entity.drive_locomotion,
                    &mut entity.low_bridge_tube_state,
                    target,
                    &mut entity.lifecycle.cell_marked,
                    tube_id,
                    terrain,
                    occupancy,
                    cell_occupation,
                    raw_cell_occupation,
                )
                .is_ok()
                {
                    if let Some(exit) = exit
                        && let Some(drive) = entity.drive_locomotion.as_mut()
                    {
                        super::path_markers::accept_path_replay(
                            &mut drive.path,
                            (exit.0 as i16, exit.1 as i16),
                            1,
                        );
                    }
                    tube_processed.insert(entity_id);
                    continue;
                }
            }

            marker_context = path_grid.map(|grid| BridgeMarkerContext {
                // PathfinderClass+0x03 is initialized to one by the
                // process-static constructor and has no active writer that
                // clears it.
                enabled: true,
                peers: &marker_peers,
                raw_occupation: raw_cell_occupation,
                grid,
                terrain: resolved_terrain,
                playfield_bounds,
                native_frame,
            });

            // Steering / rotation. Hover steers continuously toward the current
            // waypoint (facing-lagged curves, turn-stall braking) and never
            // stop-rotates; everything else keeps the rotate-in-place-then-move
            // behavior. ROT=0 means instant turn in both models.
            let uses_hover_locomotor = snap.locomotor.as_ref().is_some_and(|loco| {
                matches!(
                    loco.kind,
                    crate::rules::locomotor_type::LocomotorKind::Hover
                )
            });
            let mut hover_stall = false;
            if snap.category != EntityCategory::Infantry {
                if uses_hover_locomotor {
                    hover_stall = movement_step::hover_steer(
                        &mut entity.facing,
                        &mut entity.facing_target,
                        &mut entity.body_facing,
                        &entity.position,
                        target,
                        snap.rot,
                        native_frame,
                    );
                } else {
                    match movement_step::handle_vehicle_rotation(
                        &mut entity.facing,
                        &mut entity.facing_target,
                        &mut entity.body_facing,
                        &mut entity.position,
                        &mut entity.locomotor,
                        snap.rot,
                        native_frame,
                        sim_tick,
                    ) {
                        movement_step::RotationResult::StillRotating { debug_events: evts } => {
                            debug_events.extend(evts);
                            continue;
                        }
                        movement_step::RotationResult::ReadyToMove => {}
                    }
                }
            }

            // Per-cell speed modifier: terrain type × slope × damaged-mover.
            // Computed from the unit's current cell and next path step. Gamemd
            // builds this fraction inside Drive/Ship Process_Movement only, so
            // the helper returns 1.0 for every other locomotor.
            let below_condition_yellow = rules.is_some_and(|r| {
                crate::sim::pathfinding::terrain_speed::is_at_or_below_condition_yellow(
                    entity.health.current as i64,
                    entity.health.max as i64,
                    r.general.condition_yellow_x1000,
                )
            });
            let cell_speed_mod: SimFixed = {
                let next_cell = target.path.get(target.next_index).copied();
                match (
                    resolved_terrain,
                    snap.speed_type,
                    &snap.locomotor,
                    next_cell,
                ) {
                    (Some(terrain), Some(st), Some(loco), Some(nc)) => {
                        super::drive_locomotion::compute_drive_target_speed_fraction(
                            st,
                            loco.kind,
                            (entity.position.rx, entity.position.ry),
                            nc,
                            terrain,
                            terrain_speed_config,
                            below_condition_yellow,
                        )
                    }
                    _ => SIM_ONE,
                }
            };
            let uses_drive_locomotor = snap.locomotor.as_ref().is_some_and(|loco| {
                matches!(
                    loco.kind,
                    crate::rules::locomotor_type::LocomotorKind::Drive
                )
            });
            let uses_ship_locomotor = snap.locomotor.as_ref().is_some_and(|loco| {
                matches!(loco.kind, crate::rules::locomotor_type::LocomotorKind::Ship)
            });
            // Speed ramping: acceleration toward max speed, deceleration near goal.
            // Matches the Drive/Ship Process_Drive_Track fraction computation.
            if uses_drive_locomotor || uses_ship_locomotor {
                let goal = target.final_goal.unwrap_or_else(|| {
                    target
                        .path
                        .last()
                        .copied()
                        .unwrap_or((entity.position.rx, entity.position.ry))
                });
                let mut dist = distance_to_goal_leptons(&entity.position, goal);

                if snap.movement_zone.is_water_mover() {
                    if let Some(cell) =
                        path_grid.and_then(|pg| pg.cell(entity.position.rx, entity.position.ry))
                    {
                        if cell.bridge_deck_level_if_any().is_some() {
                            dist += BRIDGE_Z_OFFSET;
                        }
                    }
                }

                if uses_drive_locomotor {
                    let raw_speed_per_frame = target.speed / SimFixed::from_num(15);
                    if let Some(drive) = entity.drive_locomotion.as_mut() {
                        super::drive_locomotion::update_drive_speed_fraction(
                            drive,
                            cell_speed_mod,
                            snap.drive_accelerates,
                            raw_speed_per_frame,
                            target.accel_factor,
                            target.decel_factor,
                            target.slowdown_distance,
                            dist,
                        );
                        target.current_speed = target.speed * drive.current_speed_fraction;
                        drive.owner_current_speed =
                            super::drive_locomotion::owner_current_speed_from_fraction(
                                target.speed,
                                drive.current_speed_fraction,
                            );
                    } else {
                        target.current_speed = target.speed * cell_speed_mod;
                    }
                } else if let Some(ship) = entity.ship_locomotion.as_mut() {
                    let raw_speed_per_frame = target.speed / SimFixed::from_num(15);
                    let requested_fraction =
                        super::drive_locomotion::ship_process_target_speed_fraction(
                            ship,
                            cell_speed_mod,
                        );
                    super::drive_locomotion::update_ship_speed_fraction(
                        ship,
                        requested_fraction,
                        snap.drive_accelerates,
                        raw_speed_per_frame,
                        target.accel_factor,
                        target.decel_factor,
                        target.slowdown_distance,
                        dist,
                    );
                    target.current_speed = target.speed * ship.current_speed_fraction;
                    ship.owner_current_speed =
                        super::drive_locomotion::owner_current_speed_from_fraction(
                            target.speed,
                            ship.current_speed_fraction,
                        );
                } else {
                    target.current_speed = target.speed * cell_speed_mod;
                }
            } else if uses_hover_locomotor {
                // Hover throttle (the hover locomotor's SpeedUpdate model, see
                // sim/movement/hover.rs): a [0,1] fraction of base Speed ramped
                // at the HoverAcceleration/HoverBrake minute rates. Request: 0
                // while turning hard (steering above), 0.5 on arrival slow-in /
                // departure slow-out (~1 cell of goal / path start), else 1.0.
                // HoverBoost multiplies the request when the next two queued
                // steps share a direction; the post-boost clamp to 1.0 makes it
                // a cruise no-op. Throttle persists on the locomotor across
                // repaths.
                let goal = target.final_goal.unwrap_or_else(|| {
                    target
                        .path
                        .last()
                        .copied()
                        .unwrap_or((entity.position.rx, entity.position.ry))
                });
                let dist_goal = distance_to_goal_leptons(&entity.position, goal);
                let start = target.path.first().copied().unwrap_or(goal);
                let dist_start = distance_to_goal_leptons(&entity.position, start);
                // Straightaway when the step INTO the current waypoint and the
                // step OUT of it share a direction (the two queued same-facing
                // path entries of the boost condition).
                let straightaway = if target.next_index + 1 < target.path.len() {
                    let a = target.path[target.next_index];
                    let b = target.path[target.next_index + 1];
                    let dir_in = facing_from_delta(
                        a.0 as i32 - entity.position.rx as i32,
                        a.1 as i32 - entity.position.ry as i32,
                    );
                    let dir_out =
                        facing_from_delta(b.0 as i32 - a.0 as i32, b.1 as i32 - a.1 as i32);
                    dir_in == dir_out
                } else {
                    false
                };
                let (accel_min, brake_min, boost) = rules
                    .map(|r| {
                        (
                            r.general.hover_acceleration,
                            r.general.hover_brake,
                            r.general.hover_boost,
                        )
                    })
                    .unwrap_or((
                        super::hover::HOVER_ACCELERATION_DEFAULT_MINUTES,
                        super::hover::HOVER_BRAKE_DEFAULT_MINUTES,
                        SimFixed::lit("1.5"),
                    ));
                let request = super::hover::hover_speed_request(hover_stall, dist_goal, dist_start);
                let boost_mult = if straightaway { boost } else { SIM_ONE };
                let throttle = snap
                    .locomotor
                    .as_ref()
                    .map(|l| l.hover_throttle)
                    .unwrap_or(SIM_ONE);
                let new_throttle = super::hover::hover_tick_throttle(
                    throttle, request, boost_mult, accel_min, brake_min,
                );
                if let Some(ref mut loco) = entity.locomotor {
                    loco.hover_throttle = new_throttle;
                    // The readiness producer reads the request, not the ramp.
                    loco.hover_speed_request = request;
                }
                target.current_speed = target.speed * new_throttle;
            } else if target.accel_factor > SIM_ZERO || target.decel_factor > SIM_ZERO {
                let goal = target.final_goal.unwrap_or_else(|| {
                    target
                        .path
                        .last()
                        .copied()
                        .unwrap_or((entity.position.rx, entity.position.ry))
                });
                // 2D Euclidean lepton distance — diagonal arrivals brake ~41%
                // earlier than the prior Chebyshev metric. Bridge Z offset added
                // below for water movers.
                let mut dist = distance_to_goal_leptons(&entity.position, goal);

                // Ships under bridges: inflate distance by bridge Z clearance to prevent
                // premature braking.
                if snap.movement_zone.is_water_mover() {
                    if let Some(cell) =
                        path_grid.and_then(|pg| pg.cell(entity.position.rx, entity.position.ry))
                    {
                        if cell.bridge_deck_level_if_any().is_some() {
                            dist += BRIDGE_Z_OFFSET;
                        }
                    }
                }

                if dist < target.slowdown_distance && target.slowdown_distance > SIM_ZERO {
                    // Within braking distance: decelerate, floor at 30% of max speed.
                    target.current_speed -= target.decel_factor;
                    let floor = target.speed * MIN_BRAKE_FRACTION;
                    if target.current_speed < floor {
                        target.current_speed = floor;
                    }
                } else if target.current_speed < target.speed {
                    // Below max speed: accelerate.
                    target.current_speed += target.accel_factor;
                    if target.current_speed > target.speed {
                        target.current_speed = target.speed;
                    }
                }
                // Clamp to non-negative.
                if target.current_speed < SIM_ZERO {
                    target.current_speed = SIM_ZERO;
                }
            } else {
                // No ramping data — constant speed fallback.
                target.current_speed = target.speed;
            }
            let mut effective_speed: SimFixed = if uses_drive_locomotor || uses_ship_locomotor {
                target.current_speed
            } else {
                target.current_speed * cell_speed_mod
            };
            let mut frame_budget =
                movement_step::movement_frame_budget_from_current_speed(effective_speed);
            if let Some(crawls) = prone_crawls {
                frame_budget =
                    infantry::apply_prone_speed(SimFixed::from_num(frame_budget), crawls)
                        .to_num::<i32>();
            }
            // Hover turn-stall: hold position while the body swings through a
            // >45° turn (the throttle keeps braking above). See hover_steer's
            // doc for why translation is suppressed rather than decayed.
            if hover_stall {
                effective_speed = SIM_ZERO;
                frame_budget = 0;
            }

            // Advance sub_x/sub_y toward the next cell — either via drive track
            // (smooth curve) or straight-line lepton vector.
            let mut skip_cell_crossings_after_chain_ready = false;
            let current_occupation_layer = if entity.on_bridge {
                MovementLayer::Bridge
            } else {
                MovementLayer::Ground
            };
            let prior_position = (
                entity.position.rx,
                entity.position.ry,
                entity.position.sub_x,
                entity.position.sub_y,
            );
            let advance_result = movement_step::advance_lepton_position(
                target,
                &mut entity.position,
                &mut entity.facing,
                &mut entity.facing_target,
                &mut entity.drive_track,
                &mut entity.drive_locomotion,
                &mut entity.ship_locomotion,
                &mut entity.locomotor,
                entity.category,
                effective_speed,
                frame_budget,
                dt,
                entity_id,
                Some(&mut *cell_occupation),
                // The object-list arm, from this owner's blocker snapshot.
                // It is refreshed above whenever occupancy changed, so it
                // reflects every mover that already committed this tick.
                movement_step::DriveCellAdmission {
                    units: mover_entity_block_map,
                },
                current_occupation_layer,
                path_grid,
            );
            if (
                entity.position.rx,
                entity.position.ry,
                entity.position.sub_x,
                entity.position.sub_y,
            ) != prior_position
            {
                entity.position.exact_z_leptons = None;
            }
            match advance_result {
                movement_step::AdvanceResult::DriveTrackActive => continue,
                movement_step::AdvanceResult::DriveTrackCellJump { cell_dx, cell_dy } => {
                    // Drive track coordinates crossed a cell boundary.
                    // Perform the cell transition: move rx/ry by the delta the
                    // coordinate actually applied, reserve destination, handle
                    // bridge state, and consume the queued path node only once
                    // the mover's cell has reached it.
                    if target.next_index < target.path.len() {
                        // gamemd keeps ONE absolute coordinate per object and
                        // derives its cell from that coordinate, so the cell and
                        // the sub-cell offset always move by the same delta and
                        // the rendered position stays continuous. Taking the
                        // cell from the path node instead lets the two disagree:
                        // the straight NE/SW curves pass exactly through a cell
                        // corner and cross one axis per point, which under the
                        // old code moved the mover a whole cell (256 leptons)
                        // sideways in a single frame and consumed two path nodes
                        // for one diagonal step.
                        let old_rx = entity.position.rx;
                        let old_ry = entity.position.ry;
                        let nx = old_rx.saturating_add_signed(cell_dx as i16);
                        let ny = old_ry.saturating_add_signed(cell_dy as i16);
                        // The queued node is reached by this crossing when the
                        // cell we land in IS that node. The same-cell path step
                        // below is the one exception: A* can emit a bridge-ramp
                        // node that repeats the current cell on a different
                        // layer, which no coordinate crossing can ever equal.
                        // gamemd's queue holds direction octants and cannot
                        // express a same-cell step at all, so this arm is
                        // VERA-internal with the gamemd equivalent UNCHECKED; it
                        // preserves the pre-existing consumption of such a node.
                        let queued = target.path[target.next_index];
                        let same_cell_path_step = queued == (old_rx, old_ry);
                        let reaches_queued_node = queued == (nx, ny) || same_cell_path_step;
                        // DIAGNOSTIC: detect same-cell layer transition in drive track path
                        if same_cell_path_step {
                            let next_layer = target.layer_at(target.next_index);
                            log::warn!(
                                "BRIDGE_DIAG entity={}: DriveTrackCellJump same-cell step! \
                                 cell=({},{}) path_layer={:?} active_layer={:?} z={} \
                                 next_index={}/{}",
                                entity_id,
                                old_rx,
                                old_ry,
                                next_layer,
                                active_layer,
                                entity.position.z,
                                target.next_index,
                                target.path.len(),
                            );
                        }
                        // Update cell coordinates.
                        entity.position.rx = nx;
                        entity.position.ry = ny;
                        // GATE A2 verified order: capture the OLD (pre-transition)
                        // object-list layer first; the bridge predicate below may
                        // flip on_bridge, giving a different NEW layer.
                        let old_occupancy_layer = if entity.on_bridge {
                            MovementLayer::Bridge
                        } else {
                            MovementLayer::Ground
                        };
                        let mut new_occupancy_layer = old_occupancy_layer;
                        // Bridge state resolution: apply the on_bridge cell-flag predicate.
                        // loco.layer follows A*'s path_layer (next_layer). on_bridge is
                        // updated by apply_pending_bridge_render_state from bridge_update below
                        // — driven by the predicate, NOT the layer match.
                        if let Some(pg) = path_grid {
                            let next_layer = target.layer_at(target.next_index);
                            let bridge_update =
                                super::movement_bridge::resolve_cell_transition_bridge_state(
                                    &mut entity.position,
                                    Some(pg),
                                    (old_rx, old_ry),
                                    (nx, ny),
                                    entity.on_bridge,
                                );
                            pending_bridge_update = bridge_update;
                            let new_on_bridge = super::movement_bridge::projected_on_bridge(
                                entity.on_bridge,
                                bridge_update,
                            );
                            new_occupancy_layer = if new_on_bridge {
                                MovementLayer::Bridge
                            } else {
                                MovementLayer::Ground
                            };
                            active_layer = next_layer;
                            if let Some(ref mut loco) = entity.locomotor {
                                loco.layer = next_layer;
                            }
                        }
                        // Update occupancy grid: move entity from old cell to new
                        // cell, removing on the OLD layer and inserting on the NEW
                        // layer (verified two-layer order).
                        let order = next_occupancy_enter_order.next();
                        entity.occupancy_enter_order = order;
                        occupancy.move_entity_layered(
                            old_rx,
                            old_ry,
                            nx,
                            ny,
                            entity_id,
                            old_occupancy_layer,
                            new_occupancy_layer,
                            entity.sub_cell,
                            CellListInsertion::from_category(entity.category),
                        );
                        if entity.category == EntityCategory::Unit
                            && let Some(drive) = entity.drive_locomotion.as_mut()
                        {
                            crate::sim::occupancy::mark_current_drive_occupation_after_crossing(
                                drive,
                                cell_occupation,
                                entity_id,
                                (nx, ny),
                                new_occupancy_layer,
                            );
                        }
                        // Reserve destination cell.
                        super::movement_reservation::reserve_destination_after_transition(
                            entity.category,
                            entity_id,
                            &mut entity.locomotor,
                            &mut entity.position,
                            &mut entity.sub_cell,
                            active_layer,
                            nx,
                            ny,
                            occupancy,
                            snap.sub_cell_priority_mission && snap.nav_com_cell == Some((nx, ny)),
                        );
                        // After reservation, infantry sub_cell may have changed.
                        if entity.category == EntityCategory::Infantry {
                            occupancy.update_sub_cell(nx, ny, entity_id, entity.sub_cell);
                        }
                        stats.moved_steps = stats.moved_steps.saturating_add(1);
                        // Consume the queued path node only when the mover's own
                        // cell has actually reached it. A curve that crosses one
                        // axis at a time passes through an intermediate cell that
                        // is not on the path; that crossing is a real object-list
                        // move (gamemd performs it too) but it is not an arrival.
                        // Update move_dir for after the track finishes. Don't
                        // initiate a new drive track — current one is still active.
                        if reaches_queued_node {
                            target.next_index += 1;
                            if target.next_index < target.path.len() {
                                let next = target.path[target.next_index];
                                let ndx = next.0 as i32 - nx as i32;
                                let ndy = next.1 as i32 - ny as i32;
                                let (d_x, d_y, d_len) =
                                    crate::util::lepton::cell_delta_to_lepton_dir(ndx, ndy);
                                target.move_dir_x = d_x;
                                target.move_dir_y = d_y;
                                target.move_dir_len = d_len;
                            }
                        }
                    }
                    // Apply bridge state and screen coords, then continue to next tick.
                    super::movement_bridge::apply_pending_bridge_render_state(
                        &mut entity.locomotor,
                        &mut entity.bridge_occupancy,
                        &mut entity.on_bridge,
                        active_layer,
                        pending_bridge_update,
                        entity_id,
                    );
                    continue;
                }
                movement_step::AdvanceResult::DriveTrackChainReady => {
                    // Track reached chain_index — attempt to chain into a
                    // follow-on track curve. Check passability of the next
                    // cell in the path, select a new track if the direction
                    // changes, and replace the drive track state.
                    // If chaining fails, the current track continues normally.
                    if target.next_index < target.path.len() {
                        // The chain window is (current curve's exit direction,
                        // the queue head's direction) — gamemd's
                        // `path[0]_dir + octant(target_facing) * 8`, taken from
                        // the mover's own cell, not one node further on. Its
                        // eligibility test is the same inequality, which can
                        // only be true once the curve has already consumed the
                        // node it turns into.
                        let cur_cell = (entity.position.rx, entity.position.ry);
                        let head_cell = target.path[target.next_index];
                        let ndx = head_cell.0 as i32 - cur_cell.0 as i32;
                        let ndy = head_cell.1 as i32 - cur_cell.1 as i32;
                        // gamemd's queue head is octant-adjacent by
                        // construction (the path queue stores direction
                        // octants), so a chain is only planned against an
                        // adjacent node. A two-cell head occurs in VERA only
                        // while a curve kept across a mid-flight re-order still
                        // has its own two-node head queued; chaining against it
                        // would anchor the follow-on curve on the wrong cell.
                        if (ndx != 0 || ndy != 0) && ndx.abs() <= 1 && ndy.abs() <= 1 {
                            let next_face = super::facing_from_delta(ndx, ndy);
                            // Use the active track's post-turn facing as the
                            // chain "from-dir." By the time the chain attempt
                            // fires (at chain_index of the current track),
                            // entity.facing is mid-rotation along the curve;
                            // the binary uses the TurnTrack entry's
                            // target_facing here. The unwrap_or is defensive:
                            // DriveTrackChainReady is only produced inside an
                            // active track.
                            let cur_face = entity
                                .drive_track
                                .as_ref()
                                .map(|t| t.target_facing)
                                .unwrap_or(entity.facing);
                            // Only chain if the direction changes (otherwise
                            // the current track finishes into straight movement).
                            // The comparison is between direction octants, as in
                            // the binary: a computed step facing is not always
                            // the exact octant byte (east is 63, not 64), so
                            // comparing raw bytes chains a curve into its own
                            // continuation.
                            if crate::util::direction::direction_from_facing(next_face)
                                != crate::util::direction::direction_from_facing(cur_face)
                            {
                                // Runtime Can_Enter_Cell tuple for the chained
                                // lookahead: target, direction, current height,
                                // null parent, arg5=1.
                                let next_layer = target.layer_at(target.next_index);
                                let runtime_entry = evaluate_runtime_can_enter_cell_with_transition(
                                    path_grid,
                                    next_layer,
                                    &mut entity.runtime_bridge_transition,
                                    entity.on_bridge,
                                    super::movement_occupancy::RuntimeCanEnterCellArgs::runtime(
                                        head_cell,
                                        runtime_can_enter_direction(cur_cell, head_cell),
                                        runtime_current_effective_height(
                                            path_grid,
                                            (entity.position.rx, entity.position.ry),
                                            entity.on_bridge,
                                            entity.position.z,
                                        ),
                                    ),
                                );
                                deferred_drive_track_chain = Some(DeferredDriveTrackChain {
                                    target_cell: head_cell,
                                    layers: runtime_entry.layers,
                                    bridge_traversal_allowed: runtime_entry
                                        .bridge_traversal_allowed,
                                    cur_face,
                                    next_face,
                                });
                            }
                        }
                    }
                    // Whether chaining succeeded or not, continue to next tick.
                    // If chaining failed, the current track continues from
                    // where it was (point_index stays at chain_index).
                    skip_cell_crossings_after_chain_ready = true;
                }
                movement_step::AdvanceResult::DriveTrackFreshBlocked(refusal) => {
                    // gamemd asks `Can_Enter_Cell` before it commits a curve and
                    // dispatches on the CODE it returns — the codes do not share
                    // one arm. Nothing was installed, nothing was reserved, and
                    // the mover has not moved, so no crossing follows; the only
                    // thing carried out is the refusal and its code.
                    //
                    // Code 6 — an allied body sitting still in the cell — takes
                    // its own arm at 0x004B36FD (`CMP EDX,0x6 / JNZ 0x004B3944`
                    // at 0x004B36F4-0x004B36F7 is what separates it) and that arm
                    // ends in `CellClass__Scatter_Objects @ 0x00481670`, called
                    // at 0x004B393A, before it falls into the shared entry via
                    // `JMP 0x004B3607`. So the parked blocker gets told to move.
                    // Routing it into the code-2 entry instead leaves it parked
                    // forever and the mover repathing around a cell that never
                    // clears.
                    //
                    // `handle_deferred_occupancy`'s `FriendlyStationary` arm IS
                    // that ladder — scatter the blocker, then take the wait — so
                    // the refusal is handed to it rather than reimplemented here.
                    // Nothing else in the tick sets `deferred_cell_check` on this
                    // path: `process_cell_crossings`, its only other producer, is
                    // skipped one line below.
                    //
                    // VERA-internal, gamemd equivalent UNCHECKED: the retail arm
                    // only reaches its scatter through one of three tests
                    // (0x004B37C4, 0x004B37F9, 0x004B3829 — a magnitude compare
                    // against a Rules field, a height compare, and a cell-kind
                    // compare); VERA scatters unconditionally, as its crossing
                    // lane already did before this gate existed.
                    //
                    // The layer context is `single(refusal.layer)` — the three
                    // layers collapsed onto the plane the claim was found on —
                    // where the crossing lane resolves them separately through
                    // `evaluate_runtime_can_enter_cell_with_transition`. The two
                    // agree off a bridge, which is the only regime with
                    // fixtures; the deck equivalent is UNCHECKED, as it is for
                    // the handoff mark.
                    if refusal.cost_code == Some(CODE_FRIENDLY_STATIONARY) {
                        deferred_cell_check = Some(DeferredCellCheck::Vehicle(
                            refusal.cell,
                            cell_entry::CanEnterLayerContext::single(refusal.layer),
                        ));
                    }
                    deferred_drive_selection_block = Some(refusal);
                    skip_cell_crossings_after_chain_ready = true;
                }
                movement_step::AdvanceResult::ReadyForCrossings => {}
            }

            if !skip_cell_crossings_after_chain_ready {
                // Check for cell boundary crossings and handle cell transitions.
                let crossing = movement_step::process_cell_crossings(
                    target,
                    &mut entity.position,
                    &mut entity.facing,
                    &mut entity.facing_target,
                    marker_body_facing,
                    &mut entity.locomotor,
                    &mut entity.drive_track,
                    &mut entity.drive_locomotion,
                    &mut entity.ship_locomotion,
                    &mut entity.sub_cell,
                    entity.category,
                    entity_id,
                    active_layer,
                    &snap,
                    path_grid,
                    resolved_terrain,
                    entity_cost_grid,
                    mover_entity_blocks,
                    mover_entity_block_map,
                    &live_building_entry_skips,
                    occupancy,
                    cell_occupation,
                    &mut entity.occupancy_enter_order,
                    next_occupancy_enter_order,
                    &mut stats,
                    &mut finished_entities,
                    rng,
                    ctx,
                    mcfg,
                    sim_tick,
                    marker_context,
                );
                deferred_cell_check = crossing.deferred_cell_check;
                pending_bridge_update = crossing.pending_bridge_update;
                active_layer = crossing.active_layer;
                debug_events.extend(crossing.debug_events);
                aborted_for_stuck = crossing.aborted_for_stuck;
                entity.runtime_bridge_transition = crossing.runtime_bridge_transition;

                // Apply bridge layer state BEFORE computing screen position, so that
                // the render frame always sees consistent state. Without this, there's
                // a one-frame window where the unit is in the bridge cell but
                // bridge_occupancy is still None, causing the renderer to use ground
                // height interpolation and briefly dip the unit to water level.
                if !aborted_for_stuck
                    && !matches!(deferred_cell_check, Some(DeferredCellCheck::Vehicle(_, _)))
                {
                    apply_pending_bridge_render_state(
                        &mut entity.locomotor,
                        &mut entity.bridge_occupancy,
                        &mut entity.on_bridge,
                        active_layer,
                        pending_bridge_update,
                        entity_id,
                    );
                }

                // (Removed apply_bridge_lookahead_if_needed call: anticipatory layer
                // change was a workaround for the broken reactive heuristic. The
                // cell-flag predicate now makes the layer transition at the cell
                // boundary exactly, never anticipatorily — see movement_bridge.rs.)

                // DIAGNOSTIC: detect unexpected z-drop on bridge cells.
                // If bridge_occupancy is set but z is at ground level, something
                // cleared z without clearing bridge_occupancy (or vice versa).
                if let Some(ref bocc) = entity.bridge_occupancy {
                    if entity.position.z + 2 < bocc.deck_level {
                        log::error!(
                            "BRIDGE_DIAG entity={}: Z BELOW DECK! z={} deck={} \
                         cell=({},{}) layer={:?} bridge_occ={:?}",
                            entity_id,
                            entity.position.z,
                            bocc.deck_level,
                            entity.position.rx,
                            entity.position.ry,
                            active_layer,
                            entity.bridge_occupancy,
                        );
                    }
                }

                // Update screen position from lepton coordinates every tick.

                // Z handling: Z snaps discretely at cell boundaries via
                // entity.position.z (set earlier in this tick). The original engine
                // does NOT interpolate Z during sub-cell movement; track delta Z is
                // explicitly zeroed.
                // Visual smoothness on slopes comes from the body tilt system (pitch/roll),
                // not from Z interpolation. Removing the Z lerp that was here fixes a bug
                // where units on bridges visually fell to water level every cell transition
                // (the lookahead read ground_level instead of bridge_deck_level).

                // Post-loop finalization (still inside mutable borrow scope).
                if !aborted_for_stuck
                    && !matches!(deferred_cell_check, Some(DeferredCellCheck::Vehicle(_, _)))
                {
                    if target.next_index >= target.path.len() {
                        let at_final: bool = target
                            .final_goal
                            .map_or(true, |fg| (entity.position.rx, entity.position.ry) == fg);
                        if at_final
                            && !walking_to_subcell_dest(
                                &entity.locomotor,
                                entity.position.sub_x,
                                entity.position.sub_y,
                            )
                        {
                            finished_entities.push(entity_id);
                            already_finished = true;
                        }
                    }
                }
            }
        } // mutable entity borrow released here

        if aborted_for_stuck || already_finished {
            continue;
        }

        if let Some(chain) = deferred_drive_track_chain {
            handle_deferred_drive_track_chain(
                entities,
                entity_id,
                &snap,
                chain,
                path_grid,
                resolved_terrain,
                entity_cost_grid,
                occupancy,
                cell_occupation,
                &live_building_entry_skips,
                alliances,
                interner,
                rules,
                rng,
                &mut stats,
                &mut crush_kills,
                &mut already_scattered,
                sim_tick,
            );
        }

        if let Some(refusal) = deferred_drive_selection_block {
            stats.selection_admission_refusals =
                stats.selection_admission_refusals.saturating_add(1);
            log::trace!(
                "SELECTION_REFUSAL entity={entity_id} cell={:?} layer={:?} arm={:?} code={:?}",
                refusal.cell,
                refusal.layer,
                refusal.arm,
                refusal.cost_code,
            );
            // Code 6 was routed to the classifying lane above, which owns the
            // scatter ladder AND the wait/repath fallback when the scatter
            // fails. Running both would dispatch the same refusal twice.
            if refusal.cost_code != Some(CODE_FRIENDLY_STATIONARY) {
                let evts = handle_deferred_drive_selection_block(
                    entities,
                    entity_id,
                    &snap,
                    active_layer,
                    ctx,
                    mcfg,
                    entity_cost_grid,
                    mover_entity_blocks,
                    mover_entity_block_map,
                    occupancy,
                    rng,
                    &mut stats,
                    &mut finished_entities,
                    sim_tick,
                    marker_context,
                );
                debug_events.extend(evts);
            }
        }

        // --- Deferred occupancy check (unified vehicle + infantry) ---
        // Runs outside the mutable entity borrow so classify_occupied_cell()
        // can do immutable EntityStore lookups for blocker properties.
        if let Some(check) = deferred_cell_check {
            let occ_evts = handle_deferred_occupancy(
                entities,
                check,
                entity_id,
                &snap,
                active_layer,
                ctx,
                mcfg,
                entity_cost_grid,
                mover_entity_blocks,
                mover_entity_block_map,
                occupancy,
                cell_occupation,
                &live_building_entry_skips,
                alliances,
                path_grid,
                resolved_terrain,
                rng,
                &mut stats,
                &mut finished_entities,
                &mut crush_kills,
                &mut already_scattered,
                sim_tick,
                interner,
                rules,
                marker_context,
            );
            debug_events.extend(occ_evts);
        }

        // Push deferred debug events onto the entity now that all borrows are released.
        if !debug_events.is_empty() {
            if let Some(entity) = entities.get_mut(entity_id) {
                for (tick, kind) in debug_events.drain(..) {
                    entity.push_debug_event(tick, kind);
                }
            }
        }
    }

    if !single_object {
        sync_formation_speeds_after_live_pass(entities);
    }

    // Apply the immediate crush effects, then hand teardown to the lifecycle
    // authority. Occupancy entries were already removed in
    // handle_deferred_occupancy; that early timing remains UNCHECKED until the
    // unified per-object scheduler owns the whole locomotor/PerCellProcess pass.
    crush_kills.sort_by_key(|kill| (kill.victim_id, kill.crusher_id));
    crush_kills.dedup_by_key(|kill| kill.victim_id);
    for kill in &crush_kills {
        let victim_id = kill.victim_id;
        // Emit sounds BEFORE entity mutation/removal so position + type_ref
        // are still valid on the victim.
        if let Some(rules) = rules {
            if let Some(victim) = entities.get(victim_id) {
                bump_crush::emit_crush_kill_sounds_at(
                    victim,
                    kill.crush_coord,
                    rules,
                    interner,
                    sound_events,
                );
            }
        }
        if entities.get(victim_id).is_some() {
            // Crushing is a lethal path that never produces a damage event, so
            // the score-screen kill credit is captured here, against the same
            // shared helper the damage loop uses. Running infantry over is
            // routine, so without this the Kills column reads visibly low.
            let crusher_owner = entities.get(kill.crusher_id).map(|crusher| crusher.owner);
            if let Some(victim) = entities.get_mut(victim_id) {
                victim.health.current = 0;
                if let Some(rules) = rules {
                    crate::sim::combat::capture_kill_credit(victim, crusher_owner, rules, interner);
                }
            }
            // A crush runs the same `Record_The_Kill @ 0x00702D40` the damage
            // path does, so the crusher earns the victim's experience too.
            if let Some(rules) = rules {
                crate::sim::combat::award_kill_experience(
                    entities,
                    rules,
                    interner,
                    alliances,
                    kill.crusher_id,
                    victim_id,
                );
            }
            lifecycle_requests.push(LifecycleRequest::Uninit {
                stable_id: victim_id,
                reason: UninitReason::Crush,
            });
            stats.crush_kills = stats.crush_kills.saturating_add(1);
        }
    }

    finalize_finished_entities(
        entities,
        &finished_entities,
        &crush_kills,
        sim_tick,
        resolved_terrain,
        cell_occupation,
    );
    let ordinary_tail_order: Vec<u64> = entity_order
        .iter()
        .copied()
        .filter(|entity_id| !tube_processed.contains(entity_id))
        .collect();
    update_locomotor_phases(entities, &ordinary_tail_order, &crush_kills, sim_tick);

    // Hover vertical controller — every hover unit, moving OR parked (idle
    // units still float at cruise height and bob). Runs after the XY stage so
    // the per-tick order matches the original locomotor (step, then vertical).
    let (vh_height, vh_bob, vh_dampen, vh_gravity) = rules
        .map(|r| {
            (
                r.general.hover_height,
                r.general.hover_bob,
                r.general.hover_dampen,
                r.general.gravity,
            )
        })
        .unwrap_or((
            120,
            SimFixed::from_num(0.04),
            SimFixed::from_num(0.4),
            3, // engine code default; stock [AudioVisual] overrides to 6
        ));
    for &entity_id in &ordinary_tail_order {
        if contains_crush_victim(&crush_kills, entity_id) {
            continue;
        }
        let Some(entity) = entities.get_mut(entity_id) else {
            continue;
        };
        let is_hover = entity
            .locomotor
            .as_ref()
            .is_some_and(|l| matches!(l.kind, crate::rules::locomotor_type::LocomotorKind::Hover));
        if !is_hover || !entity.is_active() {
            continue;
        }
        let moving = entity.movement_target.is_some();
        // Climbing: the next path cell's ground is higher than the current
        // cell's — the height deficit is measured against the uphill slope.
        let climbing = moving
            && path_grid.is_some_and(|pg| {
                entity.movement_target.as_ref().is_some_and(|t| {
                    t.path.get(t.next_index).is_some_and(|&(nx, ny)| {
                        match (
                            pg.cell(nx, ny),
                            pg.cell(entity.position.rx, entity.position.ry),
                        ) {
                            (Some(next), Some(cur)) => next.ground_level > cur.ground_level,
                            _ => false,
                        }
                    })
                })
            });
        if let Some(ref mut loco) = entity.locomotor {
            // Hover is the one family with an observable response to power:
            // unpowered, it stops producing lift and sinks.
            let powered = loco.powered;
            let (new_height, new_offset) = super::hover::hover_vertical_tick(
                loco.altitude,
                loco.hover_bob_offset,
                native_frame,
                moving,
                climbing,
                powered,
                vh_height,
                vh_bob,
                vh_dampen,
                vh_gravity,
            );
            loco.altitude = new_height;
            loco.hover_bob_offset = new_offset;
        }
    }

    stats
}

// ---------------------------------------------------------------------------
// Post-loop helpers — extracted from tick_movement_with_grids
// ---------------------------------------------------------------------------

/// Formation speed sync (deep_113 lines 451-456).
/// Cap grouped units to the slowest member's max speed so formations stay
/// together instead of faster units pulling ahead.
pub(crate) fn sync_formation_speeds_after_live_pass(entities: &mut EntityStore) {
    let mut group_min_speed: BTreeMap<u32, SimFixed> = BTreeMap::new();
    for entity in entities.values() {
        // A Dying corpse keeps its movement_target but won't move; it must not
        // drag a living formation's speed down to its (possibly slower) value.
        if entity.dying {
            continue;
        }
        if let Some(ref mt) = entity.movement_target {
            if let Some(gid) = mt.group_id {
                let entry = group_min_speed.entry(gid).or_insert(mt.speed);
                if mt.speed < *entry {
                    *entry = mt.speed;
                }
            }
        }
    }
    if !group_min_speed.is_empty() {
        for entity in entities.values_mut() {
            if entity.dying {
                continue;
            }
            if let Some(ref mut mt) = entity.movement_target {
                if let Some(gid) = mt.group_id {
                    if let Some(&min_spd) = group_min_speed.get(&gid) {
                        if mt.speed > min_spd {
                            mt.speed = min_spd;
                        }
                    }
                }
            }
        }
    }
}

/// Remove movement targets from finished entities, reset sub-cell to final
/// position, and transition locomotor to Idle.
fn contains_crush_victim(crush_kills: &[PendingCrushKill], stable_id: u64) -> bool {
    // The mover loop consults this before the deferred kill list is sorted.
    // Preserve native live-order visibility with a linear membership check;
    // post-loop sorting remains only for deterministic request emission.
    crush_kills.iter().any(|kill| kill.victim_id == stable_id)
}

fn finalize_finished_entities(
    entities: &mut EntityStore,
    finished: &[u64],
    crush_kills: &[PendingCrushKill],
    sim_tick: u64,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    cell_occupation: &mut CellOccupationGrid,
) {
    for &entity_id in finished {
        if contains_crush_victim(crush_kills, entity_id) {
            continue;
        }
        if let Some(entity) = entities.get_mut(entity_id) {
            let current_cell = (entity.position.rx, entity.position.ry);
            let current_layer = entity
                .occupancy_list_layer()
                .unwrap_or(MovementLayer::Ground);
            if entity.category == EntityCategory::Unit
                && let Some(drive) = entity.drive_locomotion.as_mut()
            {
                crate::sim::occupancy::finish_drive_head_to_occupation(
                    drive,
                    cell_occupation,
                    entity_id,
                    current_cell,
                    current_layer,
                );
            }
            super::navcom::finish_drive_navigation(entity, resolved_terrain);
            entity.movement_target = None;
            entity.drive_track = None; // clear any active drive track curve
            entity.body_facing = None; // steering/turn interpolator ends with the move
            // Snap sub-cell leptons to final position. Use the locomotor's
            // subcell_dest if available (set during cell entry), otherwise fall
            // back to computing from sub_cell index. Vehicles snap to center.
            let (snap_x, snap_y) = entity
                .locomotor
                .as_ref()
                .and_then(|l| l.subcell_dest)
                .unwrap_or_else(|| crate::util::lepton::subcell_lepton_offset(entity.sub_cell));
            let prior_subcell = (entity.position.sub_x, entity.position.sub_y);
            entity.position.sub_x = snap_x;
            entity.position.sub_y = snap_y;
            if (snap_x, snap_y) != prior_subcell {
                entity.position.exact_z_leptons = None;
            }
            let old_phase = entity.locomotor.as_ref().map(|l| l.phase);
            if let Some(ref mut loco) = entity.locomotor {
                loco.phase = GroundMovePhase::Idle;
                loco.infantry_wobble_phase = 0.0;
                loco.subcell_dest = None;
                // Full stop zeroes the hover throttle (the hover locomotor's
                // arrival cleanup) so the next order spins up from rest.
                loco.hover_throttle = crate::util::fixed_math::SIM_ZERO;
            }
            if let Some(old) = old_phase {
                if old != GroundMovePhase::Idle {
                    entity.push_debug_event(
                        sim_tick as u32,
                        DebugEventKind::PhaseChange {
                            from: format!("{:?}", old),
                            to: "Idle".into(),
                            reason: "movement complete".into(),
                        },
                    );
                }
            }
        }
    }
}

/// Update locomotor phases for all active movers — 7-state mapping.
/// Maps the current movement state to the appropriate WalkLocomotionClass state.
fn update_locomotor_phases(
    entities: &mut EntityStore,
    entity_order: &[u64],
    crush_kills: &[PendingCrushKill],
    sim_tick: u64,
) {
    for &id in entity_order {
        if contains_crush_victim(crush_kills, id) {
            continue;
        }
        if let Some(entity) = entities.get_mut(id) {
            // Compute new phase and capture old phase in a scoped block to release
            // borrows before calling push_debug_event.
            let phase_change: Option<(GroundMovePhase, GroundMovePhase, &'static str)> = {
                if let (Some(target), Some(loco)) = (&entity.movement_target, &mut entity.locomotor)
                {
                    let old_phase = loco.phase;
                    let (new_phase, reason) = if target.path_blocked {
                        (GroundMovePhase::Blocked, "cell blocked")
                    } else if target.current_speed <= SIM_ZERO {
                        // Speed is zero but path remains — stopping or waiting to start.
                        (GroundMovePhase::Stopping, "decelerating to stop")
                    } else if target.current_speed < target.speed * MIN_BRAKE_FRACTION {
                        // Below 30% of max speed — still accelerating from rest.
                        (GroundMovePhase::Accelerating, "reached cruise speed")
                    } else if target.current_speed >= target.speed {
                        // At or above max speed — cruising.
                        (GroundMovePhase::Cruising, "reached cruise speed")
                    } else {
                        // Between 30% and max — path following with speed ramping.
                        (GroundMovePhase::PathFollow, "approaching next cell")
                    };
                    loco.phase = new_phase;
                    if old_phase != new_phase {
                        Some((old_phase, new_phase, reason))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some((old, new, reason)) = phase_change {
                entity.push_debug_event(
                    sim_tick as u32,
                    DebugEventKind::PhaseChange {
                        from: format!("{:?}", old),
                        to: format!("{:?}", new),
                        reason: reason.into(),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod distance_tests {
    use super::*;
    use crate::util::lepton::CELL_CENTER_LEPTON;

    fn pos_at(rx: u16, ry: u16) -> Position {
        Position {
            rx,
            ry,
            z: 0,
            exact_z_leptons: None,
            sub_x: CELL_CENTER_LEPTON,
            sub_y: CELL_CENTER_LEPTON,
        }
    }

    #[test]
    fn distance_same_cell_center_is_zero() {
        let d = distance_to_goal_leptons(&pos_at(10, 10), (10, 10));
        assert_eq!(d, SIM_ZERO);
    }

    #[test]
    fn distance_one_cell_cardinal_is_256_leptons() {
        let d = distance_to_goal_leptons(&pos_at(10, 10), (11, 10));
        assert_eq!(d, SimFixed::from_num(256));
    }

    #[test]
    fn distance_one_cell_diagonal_is_sqrt2_times_256() {
        // Euclidean 1-cell diagonal: sqrt(256² + 256²) = 256·sqrt(2) ≈ 362.
        // isqrt_i64(131072) = 362 (truncated). Prior Chebyshev metric returned 256.
        let d = distance_to_goal_leptons(&pos_at(10, 10), (11, 11));
        assert_eq!(d, SimFixed::from_num(362));
    }

    #[test]
    fn distance_two_cell_diagonal_brakes_at_500_threshold() {
        // 2-cell diagonal ≈ 724 leptons; default SlowdownDistance=500 → not braking yet.
        let d = distance_to_goal_leptons(&pos_at(10, 10), (12, 12));
        assert!(d > SimFixed::from_num(500));
        // 1-cell diagonal ≈ 362 → would now trigger braking, where Chebyshev (256) also did.
        let d = distance_to_goal_leptons(&pos_at(10, 10), (11, 11));
        assert!(d < SimFixed::from_num(500));
    }
}

#[cfg(test)]
mod drive_track_chain_tests {
    use super::*;
    use crate::rules::locomotor_type::LocomotorKind;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern::{test_intern, test_interner};
    use crate::sim::movement::locomotor::LocomotorState;

    // Slice 6 acceptance: a snapshot rebuilt at repath time reflects same-tick
    // moves — observably equivalent to live per-neighbor Can_Enter_Cell for a
    // synchronous search (study CELLCLASS_MAPCLASS..._SERVICE_STUDY §8 Slice 6).
    #[test]
    fn owner_block_set_refreshes_when_occupancy_generation_advances() {
        let alliances = HouseAllianceMap::new();
        let mut entities = EntityStore::new();
        let mut blocker = GameEntity::test_default(10, "HTNK", "Americans", 5, 5);
        blocker.category = EntityCategory::Unit;
        blocker.lifecycle.in_limbo = false;
        blocker.lifecycle.cell_marked = true;
        entities.insert(blocker);
        // Clone the test interner AFTER the entity is created so it can resolve
        // the just-interned owner string.
        let interner = test_interner();
        let owner = test_intern("Americans");

        // Initial snapshot at gen 0: friendly stationary unit -> soft-block at (5,5).
        let mut sets = BTreeMap::new();
        sets.insert(
            owner,
            bump_crush::build_entity_block_set(&entities, "Americans", &alliances, &interner, None),
        );
        let mut built_at: BTreeMap<crate::sim::intern::InternedId, u64> = BTreeMap::new();
        built_at.insert(owner, 0);
        assert!(sets[&owner].1.contains_key(MovementLayer::Ground, &(5, 5)));
        assert!(!sets[&owner].1.contains_key(MovementLayer::Ground, &(6, 6)));

        // Same-tick move of the blocker to (6,6); occupancy generation advances.
        {
            let b = entities.get_mut(10).unwrap();
            b.position.rx = 6;
            b.position.ry = 6;
        }
        let rebuilt = refresh_owner_block_set_if_stale(
            &mut sets,
            &mut built_at,
            owner,
            7,
            &entities,
            &alliances,
            &interner,
            None,
        );
        assert!(
            rebuilt,
            "stale snapshot must rebuild when generation advances"
        );
        assert!(
            !sets[&owner].1.contains_key(MovementLayer::Ground, &(5, 5)),
            "old cell freed"
        );
        assert!(
            sets[&owner].1.contains_key(MovementLayer::Ground, &(6, 6)),
            "new cell blocked"
        );
    }

    #[test]
    fn owner_block_set_not_rebuilt_when_generation_unchanged() {
        let alliances = HouseAllianceMap::new();
        let mut entities = EntityStore::new();
        let mut blocker = GameEntity::test_default(10, "HTNK", "Americans", 5, 5);
        blocker.category = EntityCategory::Unit;
        blocker.lifecycle.in_limbo = false;
        blocker.lifecycle.cell_marked = true;
        entities.insert(blocker);
        let interner = test_interner();
        let owner = test_intern("Americans");

        let mut sets = BTreeMap::new();
        sets.insert(
            owner,
            bump_crush::build_entity_block_set(&entities, "Americans", &alliances, &interner, None),
        );
        let mut built_at: BTreeMap<crate::sim::intern::InternedId, u64> = BTreeMap::new();
        built_at.insert(owner, 4);

        // Generation matches the recorded build gen -> no rebuild, even though the
        // entity moved underneath us.
        entities.get_mut(10).unwrap().position.rx = 6;
        let rebuilt = refresh_owner_block_set_if_stale(
            &mut sets,
            &mut built_at,
            owner,
            4,
            &entities,
            &alliances,
            &interner,
            None,
        );
        assert!(!rebuilt, "no rebuild when generation is unchanged");
        assert!(
            sets[&owner].1.contains_key(MovementLayer::Ground, &(5, 5)),
            "snapshot left untouched"
        );
    }

    fn drive_snapshot() -> MoverSnapshot {
        let locomotor = LocomotorState::for_test_kind(LocomotorKind::Drive);
        MoverSnapshot {
            category: EntityCategory::Unit,
            speed_type: Some(SpeedType::Track),
            movement_zone: MovementZone::Normal,
            omni_crusher: false,
            regular_crusher: false,
            drive_accelerates: false,
            owner: test_intern("Americans"),
            too_big_to_fit_under_bridge: false,
            on_bridge: false,
            runtime_bridge_transition: Default::default(),
            locomotor: Some(locomotor),
            rot: 5,
            bypass_grid: false,
            sub_cell_priority_mission: false,
            nav_com_cell: None,
            allow_zone_hierarchy: true,
        }
    }

    fn chain_to_east_cell() -> DeferredDriveTrackChain {
        DeferredDriveTrackChain {
            target_cell: (11, 10),
            layers: cell_entry::CanEnterLayerContext::single(MovementLayer::Ground),
            bridge_traversal_allowed: true,
            cur_face: 0,
            next_face: 32,
        }
    }

    fn run_chain_with_blocker(blocker_moving: bool) -> (bool, EntityStore, MovementTickStats) {
        let mut entities = EntityStore::new();
        let mut mover = GameEntity::test_default(1, "MTNK", "Americans", 10, 10);
        mover.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        mover.lifecycle.in_limbo = false;
        mover.lifecycle.cell_marked = true;
        entities.insert(mover);

        let mut blocker = GameEntity::test_default(2, "MTNK", "Americans", 11, 10);
        blocker.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        blocker.lifecycle.in_limbo = false;
        blocker.lifecycle.cell_marked = true;
        if blocker_moving {
            blocker.movement_target = Some(MovementTarget::default());
        }
        entities.insert(blocker);

        let mut occupancy = OccupancyGrid::rebuild(&entities);
        let mut cell_occupation = CellOccupationGrid::new();
        let snap = drive_snapshot();
        let chain = chain_to_east_cell();
        let live_building_entry_skips: BTreeMap<(u16, u16), BTreeSet<u64>> = BTreeMap::new();
        let alliances = HouseAllianceMap::new();
        let interner = test_interner();
        let mut rng = SimRng::new(0);
        let mut stats = MovementTickStats::default();
        let mut crush_kills = Vec::new();
        let mut already_scattered = BTreeSet::new();

        let installed = handle_deferred_drive_track_chain(
            &mut entities,
            1,
            &snap,
            chain,
            None,
            None,
            None,
            &mut occupancy,
            &mut cell_occupation,
            &live_building_entry_skips,
            &alliances,
            &interner,
            None,
            &mut rng,
            &mut stats,
            &mut crush_kills,
            &mut already_scattered,
            0,
        );
        (installed, entities, stats)
    }

    #[test]
    fn drive_track_chain_install_gate_matches_gamemd_codes() {
        assert!(drive_track_chain_entry_allows_track_install(
            &CellEntryResult::Clear
        ));
        assert!(drive_track_chain_entry_allows_track_install(
            &CellEntryResult::Crushable { victims: vec![1] }
        ));
        // Code 2, both variants. A body still standing in the cell and a bare
        // mask claim are both refused here. That is stricter than gamemd for an
        // occupant in TRANSIT, which carries `Foot+0x6B6 == 0` (0x004B161A) and
        // therefore takes the locomotor `+0xA4` question at 0x0073FA46 that can
        // skip it — a recorded DRIFT, written up on
        // `drive_track_chain_entry_allows_track_install`. Only a parked
        // occupant (`+0x6B6 != 0`, RTTI != 0x0F) reaches the raise directly via
        // 0x0073FA38-0x0073FA44.
        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::TemporaryBlock { blocker_id: 1 }
        ));
        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::TemporaryOccupation
        ));
        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::ScatterRequired {
                blocker_id: Some(1),
            }
        ));
        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::FriendlyStationary { blocker_id: 1 }
        ));
        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::FriendlyWall
        ));
        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::OccupiedEnemy { blocker_id: 1 }
        ));
        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::Impassable
        ));
    }

    #[test]
    fn drive_track_chain_code3_requests_gate_open_without_install_permission() {
        let ini = crate::rules::ini_parser::IniFile::from_str(
            "[VehicleTypes]\n0=MTNK\n[BuildingTypes]\n0=GAGATE_A\n\
             [MTNK]\nName=Tank\nSpeed=4\n\
             [GAGATE_A]\nName=Allied Gate\nFoundation=3x1\nGate=yes\nDeployTime=.044\nGateCloseDelay=.2\n",
        );
        let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("gate rules");
        let mut entities = EntityStore::new();
        let mut mover = GameEntity::test_default(1, "MTNK", "Americans", 10, 10);
        mover.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        mover.lifecycle.in_limbo = false;
        mover.lifecycle.cell_marked = true;
        entities.insert(mover);

        let mut gate = GameEntity::test_default(100, "GAGATE_A", "Americans", 11, 10);
        gate.category = EntityCategory::Structure;
        gate.building_gate = Some(crate::sim::game_entity::BuildingGateRuntime::default());
        gate.lifecycle.in_limbo = false;
        gate.lifecycle.cell_marked = true;
        entities.insert(gate);

        let occupancy = OccupancyGrid::rebuild(&entities);
        let alliances = HouseAllianceMap::new();
        let interner = test_interner();
        let requested = drive_track_chain_check_crushable_obstacle(
            &mut entities,
            &occupancy,
            chain_to_east_cell(),
            1,
            &drive_snapshot(),
            Some(&rules),
            &alliances,
            &interner,
        );

        assert!(!drive_track_chain_entry_allows_track_install(
            &CellEntryResult::ScatterRequired {
                blocker_id: Some(100),
            }
        ));
        assert!(requested);
        let gate = entities.get(100).unwrap().building_gate.unwrap();
        assert!(gate.mission_18_active);
        assert_eq!(
            gate.mission_state,
            crate::sim::game_entity::BuildingGateMissionState::Setup
        );
    }

    #[test]
    fn drive_track_chain_code6_scatters_without_installing_track() {
        let (installed, entities, stats) = run_chain_with_blocker(false);

        assert!(!installed);
        assert!(entities.get(1).unwrap().drive_track.is_none());
        assert!(entities.get(2).unwrap().movement_target.is_some());
        assert_eq!(stats.scatter_successes, 1);
    }

    /// Code 2 refuses the chain and, unlike code 6, consumes no scatter.
    ///
    /// This asserted the opposite until 2026-08-05. A moving friendly vehicle is
    /// still a body in the cell, so refusing is the right direction — but this
    /// is a RATCHET on a recorded DRIFT, not a parity assertion, and it must not
    /// be read as one. gamemd would ask the occupant's locomotor `+0xA4` at
    /// 0x0073FA46 (`Foot+0x6B6` reads 0 while that occupant is in transit —
    /// 0x004B161A clears it, 0x004B1FEF re-sets it) and, on a false answer,
    /// admit the follower. VERA has neither field and refuses unconditionally.
    /// Full write-up on `drive_track_chain_entry_allows_track_install`.
    ///
    /// The no-scatter half is the load-bearing half for determinism, and it
    /// still holds, but not for the reason recorded here earlier: the Drive
    /// code-2 arm is NOT call-free. It runs 0x004B3607 -> 0x004B3656 ->
    /// `JMP 0x004B39D3` -> `CALL 0x004D3920` (`FootClass::Find_Path`) at
    /// 0x004B3A0E with urgency `(BL != 0) + 1`. What it has none of is a
    /// `Scatter_Objects` call; every cell scatter in the movement body sits in a
    /// code-6 ladder — 0x004B3225 in the blocked block, 0x004B393A in the code-6
    /// dispatch arm.
    #[test]
    fn drive_track_chain_code2_refuses_track_without_scattering() {
        let (installed, entities, stats) = run_chain_with_blocker(true);

        assert!(!installed);
        assert!(entities.get(1).unwrap().drive_track.is_none());
        assert_eq!(stats.scatter_successes, 0);
    }

    #[test]
    fn drive_track_crush_clears_cell_mark_before_lifecycle_handoff() {
        let mut entities = EntityStore::new();
        let mut mover = GameEntity::test_default(1, "MTNK", "Americans", 10, 10);
        mover.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        mover.lifecycle.in_limbo = false;
        mover.lifecycle.cell_marked = true;
        entities.insert(mover);

        let mut victim = GameEntity::test_default(2, "E1", "Soviets", 11, 10);
        victim.category = EntityCategory::Infantry;
        victim.crushable = true;
        victim.lifecycle.in_limbo = false;
        victim.lifecycle.cell_marked = true;
        entities.insert(victim);

        let mut occupancy = OccupancyGrid::rebuild(&entities);
        let mut cell_occupation = CellOccupationGrid::new();
        let mut snap = drive_snapshot();
        snap.regular_crusher = true;
        let live_building_entry_skips = BTreeMap::new();
        let alliances = HouseAllianceMap::new();
        let interner = test_interner();
        let mut rng = SimRng::new(0);
        let mut stats = MovementTickStats::default();
        let mut crush_kills = Vec::new();
        let mut already_scattered = BTreeSet::new();

        let installed = handle_deferred_drive_track_chain(
            &mut entities,
            1,
            &snap,
            chain_to_east_cell(),
            None,
            None,
            None,
            &mut occupancy,
            &mut cell_occupation,
            &live_building_entry_skips,
            &alliances,
            &interner,
            None,
            &mut rng,
            &mut stats,
            &mut crush_kills,
            &mut already_scattered,
            0,
        );

        assert!(installed);
        assert_eq!(crush_kills.len(), 1);
        assert_eq!(crush_kills[0].victim_id, 2);
        assert!(!entities.get(2).unwrap().lifecycle.cell_marked);
        assert!(!occupancy.contains_entity(11, 10, 2));
    }
}
