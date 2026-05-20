//! Movement occupancy resolution — deferred cell entry checks for crush, scatter,
//! and infantry sub-cell allocation.
//!
//! When the movement tick detects that the next cell is occupied, it defers the resolution
//! to this module (outside the mutable entity borrow) so that immutable EntityStore lookups
//! can classify blockers and decide between crush, scatter, attack, or wait.

use std::collections::BTreeSet;

use crate::map::entities::EntityCategory;
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::combat::AttackTarget;
use crate::sim::components::Position;
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::bump_crush;
use crate::sim::movement::drive_track::DriveTrackState;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::movement::movement_blocked::handle_blocked_tick;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::cell_entry::{self, CanEnterLayerContext, CellEntryResult};
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::{BridgeTraversalInput, PathGrid};
use crate::sim::rng::SimRng;

use super::{
    MovementConfig, MovementTickStats, MoverSnapshot, PATH_STUCK_INIT, PathfindingContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeferredCellCheck {
    Infantry((u16, u16), CanEnterLayerContext),
    Vehicle((u16, u16), CanEnterLayerContext),
}

pub(super) fn resolve_runtime_can_enter_layers(
    path_grid: Option<&PathGrid>,
    current_cell: (u16, u16),
    next_cell: (u16, u16),
    next_layer: MovementLayer,
    path_height: u8,
) -> CanEnterLayerContext {
    let base = CanEnterLayerContext::single(next_layer);
    let Some(grid) = path_grid else {
        return base;
    };
    let (Some(parent), Some(candidate)) = (
        grid.cell(current_cell.0, current_cell.1),
        grid.cell(next_cell.0, next_cell.1),
    ) else {
        return base;
    };

    let needs_bridge_traversal = candidate.has_bridgehead_transition()
        || !candidate.has_structural_bridge()
        || !parent.has_structural_bridge();
    if !needs_bridge_traversal {
        return base;
    }

    let bridge_traversal = crate::sim::pathfinding::check_bridge_traversal(
        grid,
        BridgeTraversalInput {
            candidate,
            candidate_coord: next_cell,
            // Explicit parent is supplied, so the direction is not used for
            // predecessor reconstruction. It only needs to avoid the -1 seed path.
            direction: 0,
            path_height: path_height as i16,
            parent: Some((parent, current_cell)),
        },
    );
    if !bridge_traversal.allowed {
        return base;
    }

    crate::sim::pathfinding::can_enter_layer_context(
        next_layer,
        if bridge_traversal.force_bridge_list {
            MovementLayer::Bridge
        } else {
            base.object_list_layer
        },
        candidate,
        bridge_traversal.path_height,
    )
}

pub(super) fn detect_deferred_cell_check(
    mover_category: EntityCategory,
    mover_bypass_grid: bool,
    layer_context: CanEnterLayerContext,
    next_cell: (u16, u16),
    current_cell: (u16, u16),
    current_object_list_layer: MovementLayer,
    occupancy: &OccupancyGrid,
) -> Option<DeferredCellCheck> {
    let object_list_layer = layer_context.object_list_layer;
    let occupancy_bits_layer = layer_context.occupancy_bits_layer;
    let is_self_cell = (next_cell.0, next_cell.1, object_list_layer)
        == (current_cell.0, current_cell.1, current_object_list_layer);
    if is_self_cell {
        return None;
    }

    // bypass_grid is only set during scripted choreographed drives (the
    // harvester dock-into-foundation path). Skip the deferred occupancy
    // check entirely for these — the cell transition completes inline,
    // and dock_reservations already prevents multi-mover contention on
    // the pad cell. Without this short-circuit, structure occupants in
    // foundation cells trigger a deferred check that breaks the cell
    // transition loop, then the Clear arm snaps the mover back to cell
    // center → sub-cell oscillation, mover never advances.
    if mover_bypass_grid {
        return None;
    }

    let cell_occ = occupancy.get(next_cell.0, next_cell.1);
    if mover_category == EntityCategory::Infantry {
        if bump_crush::allocate_sub_cell_with_reserved(cell_occ, occupancy_bits_layer, None)
            .is_none()
            || cell_occ.is_some_and(|o| {
                o.has_blockers_on(object_list_layer)
                    || o.infantry(object_list_layer).next().is_some()
            })
        {
            return Some(DeferredCellCheck::Infantry(next_cell, layer_context));
        }
    } else if cell_occ.is_some_and(|o| {
        o.has_blockers_on(object_list_layer)
            || o.infantry(object_list_layer).next().is_some()
            || o.has_blockers_on(occupancy_bits_layer)
            || o.infantry(occupancy_bits_layer).next().is_some()
    }) {
        return Some(DeferredCellCheck::Vehicle(next_cell, layer_context));
    }

    None
}

pub(super) fn snap_motion_to_cell_center(
    position: &mut Position,
    drive_track: &mut Option<DriveTrackState>,
) {
    position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
    position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
    *drive_track = None;
}

pub(super) fn naval_terrain_diag(
    terrain: Option<&ResolvedTerrainGrid>,
    cell: (u16, u16),
) -> String {
    let Some(terrain) = terrain else {
        return "terrain=<none>".into();
    };
    let Some(t) = terrain.cell(cell.0, cell.1) else {
        return format!("terrain=OOB({},{})", cell.0, cell.1);
    };
    format!(
        "terrain[water={} land_type={} cliff={} overlay_blocks={} terrain_blocks={} bridge_walkable={} bridge_deck={} level={}]",
        t.is_water,
        t.land_type,
        t.is_cliff_like,
        t.overlay_blocks,
        t.terrain_object_blocks,
        t.bridge_walkable,
        t.bridge_deck_level,
        t.level,
    )
}

pub(super) fn naval_occ_diag(
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    cell: (u16, u16),
) -> String {
    match occupancy.get(cell.0, cell.1) {
        Some(occ) => format!(
            "occ[blockers={} infantry={}]",
            occ.blockers(layer).count(),
            occ.infantry(layer).count(),
        ),
        None => "occ[empty]".into(),
    }
}

/// Handle the deferred occupancy check — runs outside the mutable entity borrow
/// so `classify_occupied_cell()` can do immutable EntityStore lookups for blocker
/// properties. Returns debug events to be pushed onto the entity.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_deferred_occupancy(
    entities: &mut EntityStore,
    check: DeferredCellCheck,
    entity_id: u64,
    snap: &MoverSnapshot,
    active_layer: MovementLayer,
    ctx: PathfindingContext<'_>,
    mcfg: MovementConfig,
    entity_cost_grid: Option<&TerrainCostGrid>,
    mover_entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    mover_entity_block_map: Option<&crate::sim::pathfinding::LayeredEntityBlockMap>,
    occupancy: &mut OccupancyGrid,
    alliances: &HouseAllianceMap,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    rng: &mut SimRng,
    stats: &mut MovementTickStats,
    finished_entities: &mut Vec<u64>,
    crush_kills: &mut Vec<u64>,
    already_scattered: &mut BTreeSet<u64>,
    blockage_path_delay_ticks: u16,
    sim_tick: u64,
    interner: &crate::sim::intern::StringInterner,
) -> Vec<(u32, DebugEventKind)> {
    let mut debug_events: Vec<(u32, DebugEventKind)> = Vec::new();
    let (nx, ny, layer_context) = match check {
        DeferredCellCheck::Infantry((nx, ny), layers)
        | DeferredCellCheck::Vehicle((nx, ny), layers) => (nx, ny, layers),
    };
    let object_list_layer = layer_context.object_list_layer;
    let occupancy_bits_layer = layer_context.occupancy_bits_layer;
    let mover_loco_kind = snap
        .locomotor
        .as_ref()
        .map_or(LocomotorKind::Drive, |l| l.kind);
    let mover_is_crusher = snap.omni_crusher
        || matches!(
            snap.locomotor.as_ref().map(|l| l.movement_zone),
            Some(
                crate::rules::locomotor_type::MovementZone::Crusher
                    | crate::rules::locomotor_type::MovementZone::AmphibiousCrusher
                    | crate::rules::locomotor_type::MovementZone::CrusherAll
            )
        );
    let is_infantry = snap.category == EntityCategory::Infantry;
    let entry_result = cell_entry::classify_occupied_cell_with_layers(
        (nx, ny),
        layer_context,
        entity_id,
        snap.movement_zone,
        snap.omni_crusher,
        interner.resolve(snap.owner),
        mover_loco_kind,
        snap.bypass_grid,
        occupancy,
        entities,
        alliances,
        interner,
    );
    if snap.movement_zone.is_water_mover() {
        log::info!(
            "NAVAL occupancy block: entity={} cur=({},{}) next=({},{}) object_layer={:?} occupancy_layer={:?} result={:?} blocked_delay={} path_blocked={} {} {}",
            entity_id,
            entities.get(entity_id).map(|e| e.position.rx).unwrap_or(nx),
            entities.get(entity_id).map(|e| e.position.ry).unwrap_or(ny),
            nx,
            ny,
            object_list_layer,
            occupancy_bits_layer,
            entry_result,
            entities
                .get(entity_id)
                .and_then(|e| e.movement_target.as_ref())
                .map(|mt| mt.blocked_delay)
                .unwrap_or(0),
            entities
                .get(entity_id)
                .and_then(|e| e.movement_target.as_ref())
                .map(|mt| mt.path_blocked)
                .unwrap_or(false),
            naval_terrain_diag(resolved_terrain, (nx, ny)),
            naval_occ_diag(occupancy, occupancy_bits_layer, (nx, ny)),
        );
    }

    match entry_result {
        CellEntryResult::Clear | CellEntryResult::ScatterRequired { .. } => {
            // Locomotor override (JumpJet) cleared the block. Code 3 is kept
            // soft until the dedicated building scatter producer is ported.
            if let Some(entity) = entities.get_mut(entity_id) {
                snap_motion_to_cell_center(&mut entity.position, &mut entity.drive_track);
                if let Some(ref mut target) = entity.movement_target {
                    target.blocked_delay = 0;
                    target.path_blocked = false;
                }
            }
        }
        CellEntryResult::Crushable { victims } => {
            // Remove crush victims from occupancy immediately (matches gamemd's
            // PerCellProcess which calls RemoveFromGame before continuing).
            for &vid in &victims {
                if let Some(v) = entities.get(vid) {
                    occupancy.remove(v.position.rx, v.position.ry, vid);
                }
            }
            crush_kills.extend(victims);
            if let Some(entity) = entities.get_mut(entity_id) {
                snap_motion_to_cell_center(&mut entity.position, &mut entity.drive_track);
                if let Some(ref mut target) = entity.movement_target {
                    target.blocked_delay = 0;
                    target.path_blocked = false;
                }
            }
        }
        CellEntryResult::FriendlyStationary { blocker_id } => {
            // Scatter the stationary friendly blocker out of the way.
            // Matches original engine: CellClass::Scatter_Objects with force=1
            // tells the BLOCKER to move, not the mover. The blocker receives a
            // movement command to walk to an adjacent cell.
            let mut scattered = false;
            if !already_scattered.contains(&blocker_id) {
                scattered = bump_crush::scatter_blocker(
                    entities,
                    blocker_id,
                    path_grid,
                    occupancy,
                    object_list_layer,
                    rng,
                );
                if scattered {
                    already_scattered.insert(blocker_id);
                    stats.scatter_successes = stats.scatter_successes.saturating_add(1);
                }
            }
            // Mover waits — blocker is walking away. If scatter failed,
            // fall through to handle_blocked_tick for repath.
            if let Some(entity) = entities.get_mut(entity_id) {
                snap_motion_to_cell_center(&mut entity.position, &mut entity.drive_track);
                let cur_pos = (entity.position.rx, entity.position.ry);
                if let Some(ref mut target) = entity.movement_target {
                    if scattered {
                        // Blocker is moving — treat as temporary block, start
                        // a short wait before repath so the blocker has time to clear.
                        if !target.path_blocked {
                            target.path_blocked = true;
                            target.blocked_delay = blockage_path_delay_ticks;
                        }
                    } else {
                        let mut aborted_for_stuck = false;
                        let evts = handle_blocked_tick(
                            target,
                            &mut entity.facing,
                            &snap.locomotor,
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
                            mover_is_crusher,
                            is_infantry,
                            false, // friendly stationary: keep code-2 grace
                        );
                        debug_events.extend(evts);
                    }
                }
            }
        }
        CellEntryResult::OccupiedEnemy { blocker_id } => {
            // Code 5: Attack blocker while waiting.
            if let Some(entity) = entities.get_mut(entity_id) {
                snap_motion_to_cell_center(&mut entity.position, &mut entity.drive_track);
                if entity.attack_target.is_none() {
                    entity.attack_target = Some(AttackTarget::new(blocker_id));
                }
                let cur_pos = (entity.position.rx, entity.position.ry);
                if let Some(ref mut target) = entity.movement_target {
                    let mut aborted_for_stuck = false;
                    let evts = handle_blocked_tick(
                        target,
                        &mut entity.facing,
                        &snap.locomotor,
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
                        mover_is_crusher,
                        is_infantry,
                        false, // enemy blocker (code-5): keep code-2-style grace
                    );
                    debug_events.extend(evts);
                }
            }
        }
        CellEntryResult::TemporaryBlock { blocker_id } => {
            // Moving friendly — wait, then scatter the BLOCKER.
            // Original engine: locomotor calls CellClass::Scatter_Objects with
            // force=1 regardless of whether blocker is moving or stationary.
            // The blocker is told to scatter; the mover waits.
            if let Some(entity) = entities.get_mut(entity_id) {
                snap_motion_to_cell_center(&mut entity.position, &mut entity.drive_track);
                if let Some(ref mut target) = entity.movement_target {
                    if !target.path_blocked {
                        target.path_blocked = true;
                        target.blocked_delay = blockage_path_delay_ticks;
                    }
                    if target.blocked_delay > 0 {
                        // Still waiting — do nothing this tick.
                    } else {
                        // Wait expired — try scattering the blocker, then repath.
                        if !already_scattered.contains(&blocker_id) {
                            let scattered = bump_crush::scatter_blocker(
                                entities,
                                blocker_id,
                                path_grid,
                                occupancy,
                                object_list_layer,
                                rng,
                            );
                            if scattered {
                                already_scattered.insert(blocker_id);
                                stats.scatter_successes = stats.scatter_successes.saturating_add(1);
                            }
                        }
                        // Whether scatter succeeded or not, repath the mover.
                        // Re-borrow the mover since scatter_blocker released it.
                        if let Some(entity) = entities.get_mut(entity_id) {
                            let cur_pos = (entity.position.rx, entity.position.ry);
                            if let Some(ref mut target) = entity.movement_target {
                                let mut aborted_for_stuck = false;
                                let evts = handle_blocked_tick(
                                    target,
                                    &mut entity.facing,
                                    &snap.locomotor,
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
                                    mover_is_crusher,
                                    is_infantry,
                                    false, // temp block (moving friendly): keep grace
                                );
                                debug_events.extend(evts);
                            }
                        }
                    }
                }
            }
        }
        CellEntryResult::FriendlyWall | CellEntryResult::Impassable => {
            // Shouldn't reach here from NeedsBlockerCheck, but handle gracefully.
            if let Some(entity) = entities.get_mut(entity_id) {
                snap_motion_to_cell_center(&mut entity.position, &mut entity.drive_track);
                let cur_pos = (entity.position.rx, entity.position.ry);
                if let Some(ref mut target) = entity.movement_target {
                    let mut aborted_for_stuck = false;
                    let evts = handle_blocked_tick(
                        target,
                        &mut entity.facing,
                        &snap.locomotor,
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
                        mover_is_crusher,
                        is_infantry,
                        true, // wall/impassable (code-7): skip grace
                    );
                    debug_events.extend(evts);
                }
            }
        }
    }

    debug_events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::occupancy::CellListInsertion;

    #[test]
    fn runtime_layers_keep_bridge_object_list_with_ground_occupancy_bits() {
        let mut grid = PathGrid::new(2, 1);
        grid.set_cell_for_test(0, 0, 4, true, false);
        grid.set_cell_for_test(1, 0, 0, true, true);

        let layers =
            resolve_runtime_can_enter_layers(Some(&grid), (0, 0), (1, 0), MovementLayer::Bridge, 0);

        assert_eq!(layers.terrain_layer, MovementLayer::Bridge);
        assert_eq!(layers.object_list_layer, MovementLayer::Bridge);
        assert_eq!(layers.occupancy_bits_layer, MovementLayer::Ground);
    }

    #[test]
    fn runtime_layers_resnapshot_bridge_occupancy_bits_at_deck_height() {
        let mut grid = PathGrid::new(2, 1);
        grid.set_cell_for_test(0, 0, 4, true, false);
        grid.set_cell_for_test(1, 0, 0, true, true);

        let layers =
            resolve_runtime_can_enter_layers(Some(&grid), (0, 0), (1, 0), MovementLayer::Bridge, 4);

        assert_eq!(layers.object_list_layer, MovementLayer::Bridge);
        assert_eq!(layers.occupancy_bits_layer, MovementLayer::Bridge);
    }

    #[test]
    fn deferred_detection_uses_occupancy_bits_layer_not_object_list_layer() {
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            1,
            0,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let layers = CanEnterLayerContext {
            terrain_layer: MovementLayer::Bridge,
            object_list_layer: MovementLayer::Bridge,
            occupancy_bits_layer: MovementLayer::Ground,
        };
        let check = detect_deferred_cell_check(
            EntityCategory::Unit,
            false,
            layers,
            (1, 0),
            (0, 0),
            MovementLayer::Ground,
            &occupancy,
        );

        assert_eq!(check, Some(DeferredCellCheck::Vehicle((1, 0), layers)));
    }

    #[test]
    fn deferred_detection_uses_object_list_layer_for_selected_blockers() {
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            5,
            5,
            10,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let layers = CanEnterLayerContext {
            terrain_layer: MovementLayer::Bridge,
            object_list_layer: MovementLayer::Bridge,
            occupancy_bits_layer: MovementLayer::Ground,
        };
        let check = detect_deferred_cell_check(
            EntityCategory::Unit,
            false,
            layers,
            (5, 5),
            (4, 5),
            MovementLayer::Ground,
            &occupancy,
        );

        assert_eq!(check, Some(DeferredCellCheck::Vehicle((5, 5), layers)));
    }
}
