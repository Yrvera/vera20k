//! Move command issuing — A* pathfinding and MovementTarget attachment.
//!
//! Entry points for issuing move commands to entities. These are called from
//! `world_commands.rs`, `miner_system.rs`, and `production_queue.rs` — not
//! from the per-tick movement loop.
//!
//! ## Dependency rules
//! - Internal to sim/movement — called via re-exports in mod.rs.

use std::collections::BTreeSet;

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::rules::ruleset::GeneralRules;
use crate::sim::components::MovementTarget;
use crate::sim::entity_store::EntityStore;
use crate::sim::pathfinding::LayeredEntityBlockMap;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::zone_map::ZoneGrid;
use crate::util::direction::{DIRECTION_DELTAS, TUBE_STEP_DIRECTION};
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

use super::movement_path::{
    find_move_path_with_marker, merge_path_blocks, resolve_requested_move_goal,
    supports_layered_bridge_pathing,
};
use super::path_markers::build_peer_search_marker_overlay;
use super::{PathfindingContext, facing_from_delta};
use crate::rules::locomotor_type::MovementZone;
use crate::sim::components::OrderIntent;
use crate::sim::game_entity::GameEntity;

use super::drive_track;
use super::droppod_movement::DropPodPhase;
use super::teleport_movement;

fn drive_direction_from_step(from: (u16, u16), to: (u16, u16)) -> Option<u8> {
    let dx = to.0 as i32 - from.0 as i32;
    let dy = to.1 as i32 - from.1 as i32;
    if dx.abs() > 1 || dy.abs() > 1 {
        return Some(TUBE_STEP_DIRECTION);
    }
    DIRECTION_DELTAS
        .iter()
        .position(|&delta| delta == (dx, dy))
        .map(|idx| idx as u8)
}

fn drive_directions_from_path(path: &[(u16, u16)]) -> Vec<u8> {
    path.windows(2)
        .filter_map(|step| drive_direction_from_step(step[0], step[1]))
        .collect()
}

/// Check if an entity can accept a new movement destination.
///
/// Prevents destination changes during special states: dying, deploying,
/// undeploying, falling, and unloading passengers.
fn can_accept_destination(entity: &GameEntity) -> bool {
    if entity.dying {
        return false;
    }
    if entity.building_up.is_some() || entity.building_down.is_some() {
        return false;
    }
    if entity
        .droppod_state
        .as_ref()
        .is_some_and(|s| s.phase == DropPodPhase::Falling)
    {
        return false;
    }
    if matches!(entity.order_intent, Some(OrderIntent::Unloading)) {
        return false;
    }
    true
}

/// Clear owner navigation and queued endpoint state through the native-shaped
/// null-destination path.
pub fn clear_navigation_for_entity(entity: &mut GameEntity) {
    super::navcom::set_destination_internal_null(entity);
    entity.navigation.nav_queue.clear();
}

/// Issue a move command: compute an A* path and attach a MovementTarget to the entity.
///
/// Returns `true` if a valid path was found and the entity is now moving.
/// Returns `false` if the entity doesn't exist, has no Position, or no path exists.
///
/// `speed` is the movement speed in cells per second (from rules.ini Speed= value).
pub fn issue_move_command(
    entities: &mut EntityStore,
    grid: &PathGrid,
    entity_id: u64,
    target: (u16, u16),
    speed: SimFixed,
    queue: bool,
    terrain_costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    mover_is_crusher: bool,
) -> bool {
    issue_move_command_with_layered(
        entities,
        grid,
        entity_id,
        target,
        speed,
        queue,
        terrain_costs,
        entity_blocks,
        None, // resolved_terrain — per-tick repath has it
        None, // zone_grid — basic entrypoint has no Simulation context
        entity_block_map,
        mover_is_crusher,
    )
}

/// Gamemd-shaped Set_Destination bridge for Teleporter units.
///
/// `LocomotorState.kind` remains the active locomotor. If the target cell is a
/// building cell, a Teleport-primary unit activates Drive piggyback and receives
/// a normal ground movement target. If the target cell is empty and active
/// Teleport is available, Teleport receives Head_To_Coord and starts the warp.
#[allow(clippy::too_many_arguments)]
pub fn set_destination_for_teleporter_entity(
    entities: &mut EntityStore,
    grid: Option<&PathGrid>,
    entity_id: u64,
    target: (u16, u16),
    speed: SimFixed,
    queue: bool,
    terrain_costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    zone_grid: Option<&ZoneGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    mover_is_crusher: bool,
    rules: &GeneralRules,
    is_harvester: bool,
    is_teleporter: bool,
    destination_has_building: bool,
) -> bool {
    let Some(entity) = entities.get(entity_id) else {
        return false;
    };
    if !can_accept_destination(entity) {
        return false;
    }
    let has_teleport_locomotor = entity.locomotor.as_ref().is_some_and(|loco| {
        loco.primary_kind() == LocomotorKind::Teleport
            || loco.active_kind() == LocomotorKind::Teleport
    });
    if !is_teleporter || !has_teleport_locomotor {
        let Some(grid) = grid else {
            return false;
        };
        return issue_move_command_with_layered(
            entities,
            grid,
            entity_id,
            target,
            speed,
            queue,
            terrain_costs,
            entity_blocks,
            resolved_terrain,
            zone_grid,
            entity_block_map,
            mover_is_crusher,
        );
    }

    if destination_has_building {
        let Some(grid) = grid else {
            return false;
        };
        if let Some(entity) = entities.get_mut(entity_id)
            && let Some(ref mut loco) = entity.locomotor
        {
            loco.begin_drive_piggyback_for_teleporter();
        }
        return issue_move_command_with_layered(
            entities,
            grid,
            entity_id,
            target,
            speed,
            queue,
            terrain_costs,
            entity_blocks,
            resolved_terrain,
            zone_grid,
            entity_block_map,
            mover_is_crusher,
        );
    }

    if let Some(entity) = entities.get_mut(entity_id) {
        let should_restore = entity.locomotor.as_ref().is_some_and(|loco| {
            loco.primary_kind() == LocomotorKind::Teleport
                && loco.active_kind() != LocomotorKind::Teleport
        });
        if should_restore && let Some(ref mut loco) = entity.locomotor {
            loco.restore_primary_from_piggyback();
        }
    }

    teleport_movement::issue_active_teleport_head_to_coord(
        entities,
        entity_id,
        target,
        rules,
        is_harvester,
    )
}

/// Issue a direct move to a single cell without A* pathfinding.
///
/// Used for scripted movement into/out of building footprints where the target
/// cell is not pathfindable (e.g. refinery pad inside the foundation). Creates
/// a 2-cell `MovementTarget` `[start, target]` with a Euclidean direction
/// vector that handles multi-cell deltas correctly. Each step bypasses A*;
/// callers that also need to bypass `path_grid` walkability (e.g. foundation
/// traversal) should set `bypass_grid = true` on the resulting `MovementTarget`.
///
/// Returns `true` if the entity was found and the move was issued.
pub fn issue_direct_move(
    entities: &mut EntityStore,
    entity_id: u64,
    target: (u16, u16),
    speed: SimFixed,
) -> bool {
    let Some(entity) = entities.get(entity_id) else {
        return false;
    };
    if !can_accept_destination(entity) {
        return false;
    }
    let start = (entity.position.rx, entity.position.ry);
    if start == target {
        return true; // Already there.
    }
    let current_layer = entity.movement_layer_or_ground();

    let dx = target.0 as i32 - start.0 as i32;
    let dy = target.1 as i32 - start.1 as i32;
    let new_facing = facing_from_delta(dx, dy);
    // Compute direction vector with EUCLIDEAN length so multi-cell deltas
    // (e.g. pad→exit_cell may be (-2, +1)) advance at the correct speed.
    // `cell_delta_to_lepton_dir` only handles unit deltas — for multi-cell
    // deltas its length is wrong, causing the dual-axis crossing check in
    // movement_step to never satisfy.
    let dir_x: SimFixed = SimFixed::from_num(dx * 256);
    let dir_y: SimFixed = SimFixed::from_num(dy * 256);
    let dir_len: SimFixed = crate::util::fixed_math::fixed_distance(dir_x, dir_y);

    let movement = MovementTarget {
        path: vec![start, target],
        path_layers: vec![current_layer, current_layer],
        next_index: 1,
        speed,
        current_speed: speed,
        move_dir_x: dir_x,
        move_dir_y: dir_y,
        move_dir_len: dir_len,
        ignore_terrain_cost: true,
        ..Default::default()
    };

    if let Some(entity_mut) = entities.get_mut(entity_id) {
        entity_mut.movement_target = Some(movement);
        let has_rot = entity_mut.locomotor.as_ref().is_some_and(|l| l.rot > 0);
        if entity_mut.category != EntityCategory::Infantry && has_rot {
            entity_mut.facing_target = Some(new_facing);
        } else {
            entity_mut.facing = new_facing;
        }
    }
    true
}

pub fn issue_move_command_with_layered(
    entities: &mut EntityStore,
    grid: &PathGrid,
    entity_id: u64,
    target: (u16, u16),
    speed: SimFixed,
    queue: bool,
    terrain_costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    zone_grid: Option<&ZoneGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    mover_is_crusher: bool,
) -> bool {
    // Read the entity's current position and locomotor state.
    let Some(entity) = entities.get(entity_id) else {
        log::warn!("issue_move_command: entity {} not found", entity_id);
        return false;
    };
    if !can_accept_destination(entity) {
        return false;
    }
    let start_rx: u16 = entity.position.rx;
    let start_ry: u16 = entity.position.ry;
    let current_layer = entity.movement_layer_or_ground();
    let uses_drive_locomotor = entity
        .locomotor
        .as_ref()
        .is_some_and(|l| matches!(l.kind, LocomotorKind::Drive));
    // Derive movement_zone from the entity's locomotor — no parameter needed.
    let movement_zone: Option<MovementZone> = entity.locomotor.as_ref().map(|l| l.movement_zone);
    let too_big_to_fit_under_bridge = entity.too_big_to_fit_under_bridge;
    let layered_pathing = entity
        .locomotor
        .as_ref()
        .is_some_and(|loco| supports_layered_bridge_pathing(loco, grid, entity.on_bridge));
    let marker_request_start = if queue && !uses_drive_locomotor {
        entity
            .movement_target
            .as_ref()
            .and_then(|movement| movement.path.last().copied())
            .unwrap_or((start_rx, start_ry))
    } else {
        (start_rx, start_ry)
    };
    let merged_entity_blocks = merge_path_blocks(
        entity_blocks,
        resolved_terrain,
        movement_zone,
        too_big_to_fit_under_bridge,
    );
    let merged_entity_blocks_ref =
        (!merged_entity_blocks.is_empty()).then_some(&merged_entity_blocks);
    let Some(effective_target) = resolve_requested_move_goal(
        grid,
        target,
        merged_entity_blocks_ref,
        movement_zone,
        resolved_terrain,
        10,
    ) else {
        log::warn!(
            "No walkable cell near ({},{}) - cannot issue move",
            target.0,
            target.1,
        );
        return false;
    };
    if effective_target != target {
        log::info!(
            "Move: goal ({},{}) blocked, redirecting to ({},{})",
            target.0,
            target.1,
            effective_target.0,
            effective_target.1,
        );
    }

    let marker_overlay =
        build_peer_search_marker_overlay(entities, entity_id, marker_request_start);
    let marker_overlay_ref = (!marker_overlay.is_empty()).then_some(&marker_overlay);

    if queue && !uses_drive_locomotor {
        // Check if entity already has a movement target to append to. Drive
        // commands reissue the destination instead; standard YR player/team/
        // trigger paths do not append to Foot NavQueue.
        let entity_mut = entities.get_mut(entity_id);
        if let Some(entity_mut) = entity_mut {
            if let Some(ref mut movement) = entity_mut.movement_target {
                let append_start = movement
                    .path
                    .last()
                    .copied()
                    .unwrap_or((start_rx, start_ry));
                let append_layer = movement
                    .path_layers
                    .last()
                    .copied()
                    .unwrap_or(current_layer);
                let zone_mz = movement_zone.unwrap_or(MovementZone::Normal);
                let Some((appended, appended_layers)) = find_move_path_with_marker(
                    PathfindingContext {
                        path_grid: Some(grid),
                        zone_grid,
                        resolved_terrain,
                        blocker_neighbor_counts: None,
                    },
                    layered_pathing,
                    append_start,
                    append_layer,
                    effective_target,
                    terrain_costs,
                    // Pass the merged entity_blocks set to both layered slots so
                    // the layered A* sees building footprints regardless of which
                    // layer it expands. Mirrors the try_repath_after_block fix.
                    merged_entity_blocks_ref,
                    merged_entity_blocks_ref,
                    merged_entity_blocks_ref,
                    zone_mz,
                    movement_zone,
                    too_big_to_fit_under_bridge,
                    entity_block_map,
                    marker_overlay_ref,
                    0, // urgency=0: initial move command
                    mover_is_crusher,
                ) else {
                    return false;
                };
                if appended.len() >= 2 {
                    movement.path.extend_from_slice(&appended[1..]);
                    movement
                        .path_layers
                        .extend_from_slice(&appended_layers[1..]);
                    movement.speed = speed;
                    movement.blocked_delay = 0;
                    movement.path_blocked = false;
                    debug_assert_eq!(
                        movement.path.len(),
                        movement.path_layers.len(),
                        "path/path_layers desync after queue append"
                    );
                }
                return true;
            }
        }
    }
    let zone_mz = movement_zone.unwrap_or(MovementZone::Normal);
    let Some((path, path_layers)) = find_move_path_with_marker(
        PathfindingContext {
            path_grid: Some(grid),
            zone_grid,
            resolved_terrain,
            blocker_neighbor_counts: None,
        },
        layered_pathing,
        (start_rx, start_ry),
        current_layer,
        effective_target,
        terrain_costs,
        // Pass the merged entity_blocks set to both layered slots so the
        // layered A* sees building footprints regardless of which layer
        // it expands. Mirrors the try_repath_after_block fix.
        merged_entity_blocks_ref,
        merged_entity_blocks_ref,
        merged_entity_blocks_ref,
        zone_mz,
        movement_zone,
        too_big_to_fit_under_bridge,
        entity_block_map,
        marker_overlay_ref,
        0, // urgency=0: initial move command
        mover_is_crusher,
    ) else {
        let eb_count = merged_entity_blocks_ref.map_or(0, |s| s.len());
        log::warn!(
            "No path from ({},{}) to ({},{}) [entity_blocks={}, start_walkable={}, goal_walkable={}]",
            start_rx,
            start_ry,
            effective_target.0,
            effective_target.1,
            eb_count,
            grid.is_walkable(start_rx, start_ry),
            grid.is_walkable(effective_target.0, effective_target.1),
        );
        return false;
    };

    // Log path with walkability check for each cell — helps diagnose paths
    // that go through blocked cells (indicates PathGrid mismatch).
    let path_desc: String = path
        .iter()
        .map(|&(px, py)| {
            let w = grid.is_walkable(px, py);
            if w {
                format!("({},{})", px, py)
            } else {
                format!("({},{})!BLOCKED", px, py)
            }
        })
        .collect::<Vec<_>>()
        .join("→");
    log::info!(
        "Path: grid={}x{} entity_blocks={} {}",
        grid.width(),
        grid.height(),
        merged_entity_blocks_ref.map_or(0, |s| s.len()),
        path_desc,
    );

    // Compute initial facing toward the first movement cell (path[1], since path[0] = start).
    let mut new_facing: Option<u8> = None;
    if path.len() >= 2 {
        let next: (u16, u16) = path[1];
        let dx: i32 = next.0 as i32 - start_rx as i32;
        let dy: i32 = next.1 as i32 - start_ry as i32;
        new_facing = Some(facing_from_delta(dx, dy));
    }

    // Compute initial direction vector toward the first path step.
    // No carry-forward needed — sub_x/sub_y already encode the entity's
    // exact lepton position, so it continues from wherever it is.
    let (dir_x, dir_y, dir_len) = if path.len() >= 2 {
        crate::util::lepton::cell_delta_to_lepton_dir(
            path[1].0 as i32 - path[0].0 as i32,
            path[1].1 as i32 - path[0].1 as i32,
        )
    } else {
        (SIM_ZERO, SIM_ZERO, SIM_ZERO)
    };
    let initial_step_delta = if path.len() >= 2 {
        Some((
            path[1].0 as i32 - path[0].0 as i32,
            path[1].1 as i32 - path[0].1 as i32,
        ))
    } else {
        None
    };

    // Attach the MovementTarget and update facing on the entity.
    // All units start at full speed — acceleration/deceleration is disabled.
    let mut movement: MovementTarget = MovementTarget {
        path,
        path_layers,
        next_index: 1, // Index 0 is the current position, 1 is the first target.
        speed,
        current_speed: speed,
        move_dir_x: dir_x,
        move_dir_y: dir_y,
        move_dir_len: dir_len,
        final_goal: Some(effective_target),
        ..Default::default()
    };
    debug_assert_eq!(
        movement.path.len(),
        movement.path_layers.len(),
        "path/path_layers desync in initial MovementTarget"
    );
    let drive_path_directions = drive_directions_from_path(&movement.path);

    if let Some(entity_mut) = entities.get_mut(entity_id) {
        let uses_drive_locomotor = entity_mut
            .locomotor
            .as_ref()
            .is_some_and(|l| matches!(l.kind, LocomotorKind::Drive));
        if uses_drive_locomotor {
            super::navcom::set_destination_internal_cell(
                entity_mut,
                effective_target,
                resolved_terrain,
            );
            entity_mut.navigation.nav_queue.clear();
            let drive = entity_mut
                .drive_locomotion
                .get_or_insert_with(Default::default);
            drive.path.directions = drive_path_directions;
            drive.path.cursor = 0;
            drive.turn.target_direction = drive.path.directions.first().copied();
            drive.turn.target_facing_16 = initial_step_delta
                .map(|(dx, dy)| crate::util::fixed_math::facing_from_delta_int_u16(dx, dy));
            drive.turn.rate_timer = 0;
            drive.turn.first_movement_allowed = false;
            super::drive_locomotion::update_drive_speed_fraction(
                drive,
                crate::util::fixed_math::SIM_ONE,
                entity_mut.drive_accelerates,
                SIM_ZERO,
                SIM_ZERO,
                SIM_ZERO,
                SIM_ZERO,
                SIM_ZERO,
            );
        }
        let mut drive_track_started = false;
        if let Some(f) = new_facing {
            if entity_mut.category != EntityCategory::Infantry
                && uses_drive_locomotor
                && let Some((dx, dy)) = initial_step_delta
            {
                if let Some(sel) = drive_track::select_drive_track(entity_mut.facing, f, false) {
                    entity_mut.drive_track = drive_track::begin_drive_track(
                        sel.raw_track_index,
                        sel.flags,
                        dx,
                        dy,
                        sel.target_facing,
                    );
                    drive_track_started = entity_mut.drive_track.is_some();
                } else if let Some(fb) = drive_track::build_sharp_turn_fallback(entity_mut.facing) {
                    let (cdx, cdy) = crate::util::fixed_math::dir_to_cell_delta(entity_mut.facing);
                    entity_mut.drive_track = drive_track::begin_drive_track(
                        fb.raw_track_index,
                        fb.flags,
                        cdx,
                        cdy,
                        fb.target_facing,
                    );
                    if entity_mut.drive_track.is_some() {
                        movement.next_index += 1;
                        let (d_x, d_y, d_len) =
                            crate::util::lepton::cell_delta_to_lepton_dir(cdx, cdy);
                        movement.move_dir_x = d_x;
                        movement.move_dir_y = d_y;
                        movement.move_dir_len = d_len;
                        drive_track_started = true;
                    }
                }
            }

            if drive_track_started {
                entity_mut.facing_target = None;
            } else if uses_drive_locomotor {
                entity_mut.drive_track = None;
                entity_mut.facing_target = None;
            } else {
                entity_mut.drive_track = None;
                // Infantry always turn instantly (RA2 behavior).
                // Vehicles with ROT>0 set facing_target for gradual rotation.
                let has_rot: bool = entity_mut.locomotor.as_ref().is_some_and(|l| l.rot > 0);
                if entity_mut.category != EntityCategory::Infantry && has_rot {
                    entity_mut.facing_target = Some(f);
                } else {
                    entity_mut.facing = f;
                }
            }
        }
        entity_mut.movement_target = Some(movement);
    }

    true
}
