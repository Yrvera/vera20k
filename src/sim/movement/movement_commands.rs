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
use crate::sim::components::{DriveCoord, DriveOccupationFootprint, MovementTarget};
use crate::sim::entity_store::EntityStore;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::zone_map::ZoneGrid;
use crate::sim::pathfinding::{BlockerNeighborCounts, LayeredEntityBlockMap};
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

use super::movement_path::{
    find_move_path, merge_path_blocks, resolve_reachable_move_goal, resolve_requested_move_goal,
    supports_layered_bridge_pathing,
};
use super::{PathfindingContext, facing_from_delta};
use crate::rules::locomotor_type::MovementZone;
use crate::sim::components::OrderIntent;
use crate::sim::game_entity::GameEntity;

use super::drive_track;
use super::teleport_movement;

fn resolved_track_endpoint(
    grid: &PathGrid,
    cell: (u16, u16),
    layer: crate::sim::movement::locomotor::MovementLayer,
    fallback_z: u8,
) -> DriveCoord {
    let z = grid.cell(cell.0, cell.1).map_or(fallback_z, |path_cell| {
        path_cell.effective_cell_z_for_layer(layer)
    });
    DriveCoord::cell(cell.0, cell.1, i32::from(z as i8))
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
        None,
        None,
        None,
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
    playfield_bounds: Option<crate::sim::cell_rect::PlayfieldBounds>,
    binary_frame: u32,
) -> bool {
    let Some(entity) = entities.get(entity_id) else {
        return false;
    };
    if !can_accept_destination(entity) {
        return false;
    }
    let has_teleport_locomotor = entity.locomotor.as_ref().is_some_and(|loco| {
        loco.effective_kind() == LocomotorKind::Teleport
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
            None,
            playfield_bounds,
            None,
        );
    }

    if destination_has_building {
        let Some(grid) = grid else {
            return false;
        };
        if let Some(entity) = entities.get_mut(entity_id)
            && let Some(ref mut loco) = entity.locomotor
        {
            loco.begin_drive_piggyback_for_teleporter(binary_frame);
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
            None,
            playfield_bounds,
            None,
        );
    }

    if let Some(entity) = entities.get(entity_id) {
        let should_restore = entity.locomotor.as_ref().is_some_and(|loco| {
            loco.effective_kind() == LocomotorKind::Teleport
                && loco.active_kind() != LocomotorKind::Teleport
        });
        // `TechnoClass::Set_Destination` @ `0x00741970` unwinds through the
        // same gated protocol `FootClass::AI` uses — `Is_Ok_To_End` (`+0x14`)
        // first, transfer only when it returns true — at `0x00742587` and
        // `0x00742681`. Its third END, `0x00742A7C`, and the war-factory-exit
        // fragment at `0x0044E014` are gated on `Is_Piggybacking` (`+0x1C`)
        // alone, so "no ungated END" would be too strong; every native END is
        // nevertheless part of a *swap*. Here the gated form is the right one:
        // a Chrono Miner still driving keeps Drive installed and the per-tick
        // restore picks it up on the frame the drive actually stops.
        let gate = super::locomotor_end_gate_context(entity);
        let may_end = entity.locomotor.as_ref().is_some_and(|loco| {
            loco.can_restore_primary_from_piggyback(
                gate.owner_moving,
                gate.owner_teleporting,
                gate.owner_deploying,
            )
        });
        if should_restore
            && may_end
            && let Some(entity) = entities.get_mut(entity_id)
            && let Some(ref mut loco) = entity.locomotor
        {
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

pub(crate) fn issue_move_command_with_layered(
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
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
    playfield_bounds: Option<crate::sim::cell_rect::PlayfieldBounds>,
    mut cell_occupation: Option<&mut crate::sim::occupancy::CellOccupationGrid>,
) -> bool {
    // Read the entity's current position and locomotor state.
    let Some(entity) = entities.get(entity_id) else {
        log::warn!("issue_move_command: entity {} not found", entity_id);
        return false;
    };
    if !can_accept_destination(entity) {
        return false;
    }
    // The original engine dispatches its cell-entry predicate by object class,
    // so terrain-object occupation is read at sub-cell granularity for infantry
    // and whole-cell for everything else. The search has to know which.
    let is_infantry: bool = entity.category == EntityCategory::Infantry;
    // `AStar @ 0x0042CAD6` uses hierarchy only for a mover whose stored
    // TechnoClass+0x3D5 byte is true. Authority is explicit: resolved terrain
    // and MapClass bounds have independent lifetimes in headless fixtures and
    // during staged startup, so neither can stand in for the other.
    let allow_zone_hierarchy = playfield_bounds.is_none() || entity.in_playfield;
    let locomotor_kind = entity.locomotor.as_ref().map(|locomotor| locomotor.kind);
    let uses_drive_locomotor = locomotor_kind == Some(LocomotorKind::Drive);
    let uses_ship_locomotor = locomotor_kind == Some(LocomotorKind::Ship);
    let uses_shared_tracks = uses_drive_locomotor || uses_ship_locomotor;
    // A new destination never rewinds a curve already in flight.
    // `TechnoClass::Set_Destination` @ `0x00741970` only records the target —
    // NavCom in `FootClass::Set_Destination_Internal` @ `0x004D94B0`, the
    // coordinate in Drive `Head_To_Coord` @ `0x004AFD40` — and never touches
    // the Drive track cursor, so the new path takes effect at the curve's next
    // node. Installing a fresh curve here re-read the body position from the
    // new curve's lead-in point (the current cell centre) and visibly snapped
    // the vehicle backward, up to half a cell, on every mid-drive re-order.
    // Keep the curve and anchor the new path at its committed head cell.
    let current_cell = (entity.position.rx, entity.position.ry);
    let in_flight_curve_head: Option<(u16, u16)> = if uses_shared_tracks {
        entity.drive_track.as_ref().and_then(|track| {
            let (_, head) = drive_track::is_at_coord_track_cells(track, current_cell, false);
            u16::try_from(head.0).ok().zip(u16::try_from(head.1).ok())
        })
    } else {
        None
    };
    let keep_in_flight_curve = in_flight_curve_head.is_some();
    let (start_rx, start_ry) = in_flight_curve_head.unwrap_or(current_cell);
    let current_layer = match in_flight_curve_head {
        // The layer the body will be on at the curve head — from the accepted
        // path while it still lists that node, else the current layer.
        Some(head) => entity
            .movement_target
            .as_ref()
            .and_then(|target| {
                target
                    .path
                    .iter()
                    .position(|&cell| cell == head)
                    .map(|index| target.layer_at(index))
            })
            .unwrap_or_else(|| entity.movement_layer_or_ground()),
        None => entity.movement_layer_or_ground(),
    };
    // Derive movement_zone from the entity's locomotor — no parameter needed.
    let movement_zone: Option<MovementZone> = entity.locomotor.as_ref().map(|l| l.movement_zone);
    let speed_type = entity.locomotor.as_ref().map(|l| l.speed_type);
    let too_big_to_fit_under_bridge = entity.too_big_to_fit_under_bridge;
    let layered_pathing = entity
        .locomotor
        .as_ref()
        .is_some_and(|loco| supports_layered_bridge_pathing(loco, grid, entity.on_bridge));
    // Accepting a destination re-powers the locomotor — the player-facing
    // recovery edge. Native's Set_Destination powers it on before installing the
    // destination, so a unit that was powered down can always be ordered to move
    // again. Placed after the immutable reads above so the borrow is free.
    if let Some(loco) = entities
        .get_mut(entity_id)
        .and_then(|entity| entity.locomotor.as_mut())
    {
        loco.power_on();
    }
    let mut merged_entity_blocks = merge_path_blocks(
        entity_blocks,
        resolved_terrain,
        movement_zone,
        too_big_to_fit_under_bridge,
    );
    if let Some(occupation) = cell_occupation.as_deref() {
        merged_entity_blocks.extend(occupation.occupied_cells_ignoring(
            crate::sim::movement::locomotor::MovementLayer::Ground,
            entity_id,
        ));
    }
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

    if queue && !uses_shared_tracks {
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
                let Some((appended, appended_layers)) = find_move_path(
                    PathfindingContext {
                        path_grid: Some(grid),
                        zone_grid,
                        resolved_terrain,
                        playfield_bounds,
                        blocker_neighbor_counts,
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
                    0, // urgency=0: initial move command
                    mover_is_crusher,
                    is_infantry,
                    allow_zone_hierarchy,
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
    let ctx = PathfindingContext {
        path_grid: Some(grid),
        zone_grid,
        resolved_terrain,
        playfield_bounds,
        blocker_neighbor_counts,
    };
    let search = |goal: (u16, u16)| {
        find_move_path(
            ctx,
            layered_pathing,
            (start_rx, start_ry),
            current_layer,
            goal,
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
            0, // urgency=0: initial move command
            mover_is_crusher,
            is_infantry,
            allow_zone_hierarchy,
        )
    };
    // Order-time reachability recovery. Gamemd runs `Can_Reach_Zone` before it
    // installs the mission and, when it fails, retargets to the nearest cell in
    // the mover's OWN zone rather than dropping the order — the unit drives to
    // the near bank. VERA evaluates it only after the search has already failed:
    // the reduced per-row zone map alone under-reports reachability relative to
    // the hierarchy-backed layered search (a high-bridge route the search finds
    // can read as cross-zone), so gating ahead of the search would refuse
    // destinations gamemd accepts. The recovered set is the same; only the
    // evaluation order differs.
    let mut effective_target = effective_target;
    let mut found = search(effective_target);
    if found.is_none()
        && let Some(st) = speed_type
    {
        match resolve_reachable_move_goal(
            grid,
            zone_grid,
            resolved_terrain,
            (start_rx, start_ry),
            current_layer,
            effective_target,
            zone_mz,
            st,
        ) {
            Some(near_bank) if near_bank != effective_target => {
                log::info!(
                    "Move: ({},{}) is out of the mover's zone, retargeting to ({},{})",
                    effective_target.0,
                    effective_target.1,
                    near_bank.0,
                    near_bank.1,
                );
                effective_target = near_bank;
                found = search(effective_target);
            }
            _ => {}
        }
    }
    let Some((path, path_layers)) = found else {
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

    // A kept curve's head cell is a future node the body has not crossed into
    // yet: the queue cursor starts ON it so the coordinate crossing consumes
    // it, exactly as it would have consumed that node under the replaced path.
    let head_not_yet_reached = keep_in_flight_curve && (start_rx, start_ry) != current_cell;
    let first_target_index = if head_not_yet_reached { 0 } else { 1 };

    // Compute initial direction vector toward the first path step.
    // No carry-forward needed — sub_x/sub_y already encode the entity's
    // exact lepton position, so it continues from wherever it is.
    let (dir_x, dir_y, dir_len) = if head_not_yet_reached {
        // The first vector target is the kept curve's head itself — up to two
        // cells out for a two-node curve — so use the Euclidean form (as in
        // `issue_direct_move`) in case the curve is torn down early and the
        // vector step has to cover the multi-cell delta.
        let dx = i32::from(start_rx) - i32::from(current_cell.0);
        let dy = i32::from(start_ry) - i32::from(current_cell.1);
        let dir_x = SimFixed::from_num(dx * 256);
        let dir_y = SimFixed::from_num(dy * 256);
        let dir_len = crate::util::fixed_math::fixed_distance(dir_x, dir_y);
        (dir_x, dir_y, dir_len)
    } else if path.len() >= 2 {
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
    let movement: MovementTarget = MovementTarget {
        path,
        path_layers,
        // Index 0 is the path anchor: the current position normally (first
        // target is 1), the kept curve's still-unreached head when re-ordered
        // mid-curve (the head itself is the first queued node).
        next_index: first_target_index,
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
    if let Some(entity_mut) = entities.get_mut(entity_id) {
        let locomotor_kind = entity_mut
            .locomotor
            .as_ref()
            .map(|locomotor| locomotor.kind);
        let uses_drive_locomotor = locomotor_kind == Some(LocomotorKind::Drive);
        let uses_ship_locomotor = locomotor_kind == Some(LocomotorKind::Ship);
        let uses_shared_tracks = uses_drive_locomotor || uses_ship_locomotor;
        if uses_shared_tracks {
            super::navcom::set_destination_internal_cell(
                entity_mut,
                effective_target,
                resolved_terrain,
            );
            entity_mut.navigation.nav_queue.clear();
        }
        if uses_drive_locomotor {
            let drive = entity_mut
                .drive_locomotion
                .get_or_insert_with(Default::default);
            super::path_markers::install_path_replay(
                &mut drive.path,
                (start_rx, start_ry),
                &movement.path,
                1,
            );
            drive.turn.target_direction = drive.path.directions.first().copied();
            drive.turn.target_facing_16 = initial_step_delta
                .map(|(dx, dy)| crate::util::fixed_math::facing_from_delta_int_u16(dx, dy));
            drive.turn.rate_timer = 0;
            drive.turn.first_movement_allowed = false;
            // Native path acceptance installs path/turn/destination only.
            // Process_Movement/Track owns the later target/current qword writes.
        } else if uses_ship_locomotor {
            let ship = entity_mut
                .ship_locomotion
                .get_or_insert_with(Default::default);
            super::path_markers::install_path_replay(
                &mut ship.path,
                (start_rx, start_ry),
                &movement.path,
                1,
            );
        }
        let mut drive_track_started = false;
        let mut track_occupation_target: Option<DriveOccupationFootprint> = None;
        let mut accepted_path_reference: Option<(i16, i16)> = None;
        let mut accepted_path_nodes: usize = 1;
        // Set when the body is not yet on the head path node's octant: gamemd
        // commands that turn and installs no curve until the body reaches it.
        let mut turn_first: Option<u8> = None;
        // A kept in-flight curve owns facing and position until it completes;
        // the fresh-curve selection below runs only from a standstill anchor.
        if !keep_in_flight_curve && let Some(f) = new_facing {
            if entity_mut.category != EntityCategory::Infantry
                && uses_shared_tracks
                && let Some((dx, dy)) = initial_step_delta
            {
                let to_delta =
                    movement
                        .path
                        .get(1)
                        .zip(movement.path.get(2))
                        .map(|(&(hx, hy), &(ax, ay))| {
                            (i32::from(ax) - i32::from(hx), i32::from(ay) - i32::from(hy))
                        });
                match drive_track::plan_drive_track_from_path(
                    entity_mut.facing,
                    (dx, dy),
                    to_delta,
                    uses_ship_locomotor,
                ) {
                    drive_track::DriveTrackDecision::TurnFirst { desired_facing } => {
                        turn_first = Some(desired_facing);
                    }
                    drive_track::DriveTrackDecision::Select(plan) => {
                        entity_mut.drive_track = drive_track::begin_selected_drive_track(&plan);
                        drive_track_started = entity_mut.drive_track.is_some();
                        if drive_track_started {
                            // `next_index` starts at 1, so the head node index is
                            // exactly the number of nodes the curve spans.
                            let head_index = plan.nodes;
                            accepted_path_nodes = plan.nodes;
                            let head_rx = i32::from(entity_mut.position.rx) + plan.head_dx;
                            let head_ry = i32::from(entity_mut.position.ry) + plan.head_dy;
                            accepted_path_reference = Some((head_rx as i16, head_ry as i16));
                            if movement.layer_at(head_index)
                                == crate::sim::movement::locomotor::MovementLayer::Ground
                                && let (Ok(rx), Ok(ry)) =
                                    (u16::try_from(head_rx), u16::try_from(head_ry))
                            {
                                track_occupation_target = Some(DriveOccupationFootprint {
                                    rx,
                                    ry,
                                    layer: crate::sim::movement::locomotor::MovementLayer::Ground,
                                });
                            }
                        }
                    }
                    drive_track::DriveTrackDecision::Unavailable => {}
                }
            }

            if drive_track_started {
                entity_mut.facing_target = None;
            } else if uses_shared_tracks {
                entity_mut.drive_track = None;
                entity_mut.facing_target = turn_first;
                if let Some(ship) = entity_mut.ship_locomotion.as_mut() {
                    ship.head_to = None;
                }
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
        // A kept curve's head-to and handoff occupation claims stay with it —
        // the body is still physically driving into the claimed cells. The
        // clear/replace arms below are for curves this order replaces.
        if uses_drive_locomotor && !keep_in_flight_curve {
            let current_cell = (entity_mut.position.rx, entity_mut.position.ry);
            let current_layer = entity_mut
                .occupancy_list_layer()
                .unwrap_or(crate::sim::movement::locomotor::MovementLayer::Ground);
            if let Some(drive) = entity_mut.drive_locomotion.as_mut() {
                if let Some(reference) = accepted_path_reference {
                    super::path_markers::accept_path_replay(
                        &mut drive.path,
                        reference,
                        accepted_path_nodes,
                    );
                }
                match (track_occupation_target, cell_occupation.as_deref_mut()) {
                    (Some(next), Some(occupation)) => {
                        crate::sim::occupancy::replace_drive_head_to_occupation(
                            drive,
                            occupation,
                            entity_id,
                            current_cell,
                            current_layer,
                            next,
                        );
                        // The curve this order replaces takes its forward
                        // handoff claim with it. Leaving it behind strands a
                        // cell nothing occupies, and every later mover is
                        // refused entry to it for the rest of the match.
                        crate::sim::occupancy::drop_drive_handoff_occupation(
                            drive,
                            occupation,
                            entity_id,
                            current_cell,
                            current_layer,
                        );
                    }
                    (Some(next), None) => {
                        drive.occupation_head_to = Some(next);
                        drive.occupation_handoff = None;
                    }
                    (None, Some(occupation)) => {
                        crate::sim::occupancy::clear_drive_head_to_occupation_for_replacement(
                            drive,
                            occupation,
                            entity_id,
                            current_cell,
                            current_layer,
                        );
                        crate::sim::occupancy::drop_drive_handoff_occupation(
                            drive,
                            occupation,
                            entity_id,
                            current_cell,
                            current_layer,
                        );
                    }
                    (None, None) => {
                        drive.occupation_head_to = None;
                        drive.occupation_handoff = None;
                    }
                }
            }
        } else if uses_ship_locomotor && !keep_in_flight_curve {
            let fallback_z = entity_mut.position.z;
            if let Some(ship) = entity_mut.ship_locomotion.as_mut() {
                if let Some(reference) = accepted_path_reference {
                    super::path_markers::accept_path_replay(
                        &mut ship.path,
                        reference,
                        accepted_path_nodes,
                    );
                    let endpoint = (reference.0 as u16, reference.1 as u16);
                    let layer = movement
                        .path
                        .iter()
                        .position(|&cell| cell == endpoint)
                        .map_or(current_layer, |index| movement.layer_at(index));
                    ship.head_to = Some(resolved_track_endpoint(grid, endpoint, layer, fallback_z));
                } else {
                    ship.head_to = None;
                }
            }
        }
        entity_mut.movement_target = Some(movement);
    }

    true
}
