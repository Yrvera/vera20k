//! Movement occupancy resolution — deferred cell entry checks for crush, scatter,
//! and infantry sub-cell allocation.
//!
//! When the movement tick detects that the next cell is occupied, it defers the resolution
//! to this module (outside the mutable entity borrow) so that immutable EntityStore lookups
//! can classify blockers and decide between crush, scatter, attack, or wait.

use std::collections::{BTreeMap, BTreeSet};

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
use crate::sim::pathfinding::cell_entry::{
    self, BuildingOccupantEntryDecision, CanEnterLayerContext, CellEntryResult,
    LiveVehicleBuildingEntry, VehicleBuildingEntryBranch,
};
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::{BridgeTraversalInput, PathGrid};
use crate::sim::rng::SimRng;

use super::{
    MovementConfig, MovementTickStats, MoverSnapshot, PATH_STUCK_INIT, PathfindingContext,
    PendingCrushKill,
};

pub(super) type LiveBuildingEntrySkipMap = BTreeMap<(u16, u16), BTreeSet<u64>>;

const RUNTIME_CAN_ENTER_ARG5: i32 = 1;
const BRIDGE_DECK_LEVEL_DELTA: i16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeferredCellCheck {
    Infantry((u16, u16), CanEnterLayerContext),
    Vehicle((u16, u16), CanEnterLayerContext),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeCanEnterCellArgs {
    pub target_cell: (u16, u16),
    pub direction: i8,
    pub height: i16,
    pub parent_current_cell: Option<(u16, u16)>,
    pub arg5: i32,
}

impl RuntimeCanEnterCellArgs {
    pub(super) fn runtime(
        target_cell: (u16, u16),
        direction: i8,
        current_effective_height: i16,
    ) -> Self {
        Self {
            target_cell,
            direction,
            height: current_effective_height,
            parent_current_cell: None,
            arg5: RUNTIME_CAN_ENTER_ARG5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeCanEnterCellEvaluation {
    pub args: RuntimeCanEnterCellArgs,
    pub layers: CanEnterLayerContext,
    pub bridge_traversal_allowed: bool,
}

pub(super) fn runtime_can_enter_direction(current_cell: (u16, u16), target_cell: (u16, u16)) -> i8 {
    let dx = (target_cell.0 as i32 - current_cell.0 as i32).signum();
    let dy = (target_cell.1 as i32 - current_cell.1 as i32).signum();
    match (dx, dy) {
        (0, -1) => 0,
        (1, -1) => 1,
        (1, 0) => 2,
        (1, 1) => 3,
        (0, 1) => 4,
        (-1, 1) => 5,
        (-1, 0) => 6,
        (-1, -1) => 7,
        _ => -1,
    }
}

pub(super) fn runtime_current_effective_height(
    path_grid: Option<&PathGrid>,
    current_cell: (u16, u16),
    on_bridge: bool,
    fallback_z: u8,
) -> i16 {
    path_grid
        .and_then(|grid| grid.cell(current_cell.0, current_cell.1))
        .map_or(fallback_z as i16, |cell| {
            cell.signed_level()
                + if on_bridge {
                    BRIDGE_DECK_LEVEL_DELTA
                } else {
                    0
                }
        })
}

pub(super) fn runtime_can_enter_cell_args(
    path_grid: Option<&PathGrid>,
    current_cell: (u16, u16),
    target_cell: (u16, u16),
    on_bridge: bool,
    fallback_z: u8,
) -> RuntimeCanEnterCellArgs {
    RuntimeCanEnterCellArgs::runtime(
        target_cell,
        runtime_can_enter_direction(current_cell, target_cell),
        runtime_current_effective_height(path_grid, current_cell, on_bridge, fallback_z),
    )
}

pub(super) fn evaluate_runtime_can_enter_cell(
    path_grid: Option<&PathGrid>,
    next_layer: MovementLayer,
    args: RuntimeCanEnterCellArgs,
) -> RuntimeCanEnterCellEvaluation {
    let base = CanEnterLayerContext::single(next_layer);
    let Some(grid) = path_grid else {
        return RuntimeCanEnterCellEvaluation {
            args,
            layers: base,
            bridge_traversal_allowed: true,
        };
    };
    let Some(candidate) = grid.cell(args.target_cell.0, args.target_cell.1) else {
        return RuntimeCanEnterCellEvaluation {
            args,
            layers: base,
            bridge_traversal_allowed: true,
        };
    };
    let explicit_parent = args
        .parent_current_cell
        .and_then(|coord| grid.cell(coord.0, coord.1).map(|cell| (cell, coord)));

    let mut object_list_layer = if candidate.has_structural_bridge()
        && (args.height == -1 || (args.height - candidate.signed_level()).abs() >= 2)
    {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };

    let bridge_traversal = crate::sim::pathfinding::check_bridge_traversal(
        grid,
        BridgeTraversalInput {
            candidate,
            candidate_coord: args.target_cell,
            direction: args.direction,
            path_height: args.height,
            parent: explicit_parent,
        },
    );
    if !bridge_traversal.allowed {
        return RuntimeCanEnterCellEvaluation {
            args,
            layers: base,
            bridge_traversal_allowed: false,
        };
    }
    if bridge_traversal.force_bridge_list {
        object_list_layer = MovementLayer::Bridge;
    }

    let layers = crate::sim::pathfinding::can_enter_layer_context(
        next_layer,
        object_list_layer,
        candidate,
        bridge_traversal.path_height,
    );
    RuntimeCanEnterCellEvaluation {
        args,
        layers,
        bridge_traversal_allowed: true,
    }
}

pub(super) fn detect_deferred_cell_check(
    mover_category: EntityCategory,
    _mover_bypass_grid: bool,
    layer_context: CanEnterLayerContext,
    next_cell: (u16, u16),
    current_cell: (u16, u16),
    current_object_list_layer: MovementLayer,
    occupancy: &OccupancyGrid,
    live_building_entry_skips: &LiveBuildingEntrySkipMap,
) -> Option<DeferredCellCheck> {
    let object_list_layer = layer_context.object_list_layer;
    let occupancy_bits_layer = layer_context.occupancy_bits_layer;
    let is_self_cell = (next_cell.0, next_cell.1, object_list_layer)
        == (current_cell.0, current_cell.1, current_object_list_layer);
    if is_self_cell {
        return None;
    }

    // Static path-grid bypass is not a live-object exception. Only the skip map
    // can suppress specific building occupants such as refinery bib pads and
    // stable-open gates while preserving later blockers in the same cell list.
    let cell_occ = occupancy.get(next_cell.0, next_cell.1);
    if mover_category == EntityCategory::Infantry {
        if cell_occ.is_some_and(|o| {
            has_unignored_blocker_on(o, object_list_layer, next_cell, live_building_entry_skips)
                || has_unignored_blocker_on(
                    o,
                    occupancy_bits_layer,
                    next_cell,
                    live_building_entry_skips,
                )
                || o.infantry(object_list_layer).next().is_some()
                || o.infantry(occupancy_bits_layer).next().is_some()
        }) {
            return Some(DeferredCellCheck::Infantry(next_cell, layer_context));
        }
    } else if cell_occ.is_some_and(|o| {
        has_unignored_blocker_on(o, object_list_layer, next_cell, live_building_entry_skips)
            || o.infantry(object_list_layer).next().is_some()
            || has_unignored_blocker_on(
                o,
                occupancy_bits_layer,
                next_cell,
                live_building_entry_skips,
            )
            || o.infantry(occupancy_bits_layer).next().is_some()
    }) {
        return Some(DeferredCellCheck::Vehicle(next_cell, layer_context));
    }

    None
}

fn has_unignored_blocker_on(
    occ: &crate::sim::occupancy::CellOccupancy,
    layer: MovementLayer,
    cell: (u16, u16),
    live_building_entry_skips: &LiveBuildingEntrySkipMap,
) -> bool {
    let ignored = live_building_entry_skips.get(&cell);
    occ.iter_layer(layer).any(|occupant| {
        occupant.sub_cell.is_none() && !ignored.is_some_and(|ids| ids.contains(&occupant.entity_id))
    })
}

pub(super) fn has_unignored_runtime_occupants_on_layers(
    occupancy: &OccupancyGrid,
    cell: (u16, u16),
    layer_context: CanEnterLayerContext,
    live_building_entry_skips: &LiveBuildingEntrySkipMap,
) -> bool {
    let Some(occ) = occupancy.get(cell.0, cell.1) else {
        return false;
    };
    has_unignored_occupant_on(
        occ,
        layer_context.object_list_layer,
        cell,
        live_building_entry_skips,
    ) || (layer_context.occupancy_bits_layer != layer_context.object_list_layer
        && has_unignored_occupant_on(
            occ,
            layer_context.occupancy_bits_layer,
            cell,
            live_building_entry_skips,
        ))
}

fn has_unignored_occupant_on(
    occ: &crate::sim::occupancy::CellOccupancy,
    layer: MovementLayer,
    cell: (u16, u16),
    live_building_entry_skips: &LiveBuildingEntrySkipMap,
) -> bool {
    let ignored = live_building_entry_skips.get(&cell);
    occ.iter_layer(layer)
        .any(|occupant| !ignored.is_some_and(|ids| ids.contains(&occupant.entity_id)))
}

pub(super) fn build_live_building_entry_skip_map(
    entities: &crate::sim::entity_store::EntityStore,
    mover_id: u64,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> LiveBuildingEntrySkipMap {
    let Some(rules) = rules else {
        return LiveBuildingEntrySkipMap::new();
    };
    let Some(mover) = entities.get(mover_id) else {
        return LiveBuildingEntrySkipMap::new();
    };
    let vehicle_row_helpers = mover.category == EntityCategory::Unit;
    let gate_helpers = matches!(
        mover.category,
        EntityCategory::Unit | EntityCategory::Infantry
    );
    if !vehicle_row_helpers && !gate_helpers {
        return LiveBuildingEntrySkipMap::new();
    }

    let mut skips = LiveBuildingEntrySkipMap::new();
    for building in entities.values() {
        if building.category != EntityCategory::Structure {
            continue;
        }
        let Some(obj) = rules.object(interner.resolve(building.type_ref)) else {
            continue;
        };
        let gate_skip = gate_helpers
            && obj.gate
            && building
                .building_gate
                .is_some_and(|state| state.can_garrison_passable());
        let has_contact = vehicle_row_helpers && mover.has_live_contact_with(building.stable_id);
        let has_vehicle_exception =
            vehicle_row_helpers && (has_contact || obj.unit_repair || obj.bunker || obj.bib);
        if !has_vehicle_exception && !gate_skip {
            continue;
        }
        let is_bunker_occupied = obj.bunker
            && (building.bunker_occupant.is_some()
                || building
                    .passenger_role
                    .cargo()
                    .is_some_and(|cargo| cargo.count() > 0));
        let foundation_cells = crate::sim::production::building_base_foundation_cells(
            building.position.rx,
            building.position.ry,
            &obj.foundation,
        );
        let foundation_cell_set: BTreeSet<(u16, u16)> = foundation_cells.iter().copied().collect();
        for (cx, cy) in foundation_cells {
            let (contact_skip, second_callsite_skip) = if vehicle_row_helpers {
                let input = LiveVehicleBuildingEntry {
                    mover_category: mover.category,
                    branch: VehicleBuildingEntryBranch::RadioContact {
                        mover_has_contact: has_contact,
                    },
                    checked_building_id: building.stable_id,
                    candidate_building_id: Some(building.stable_id),
                    candidate_x: cx,
                    building_origin_x: building.position.rx,
                    number_impassable_rows: obj.number_impassable_rows,
                    is_unit_repair: obj.unit_repair,
                    is_bunker: obj.bunker,
                    bunker_occupied: is_bunker_occupied,
                };
                (
                    cell_entry::decide_live_vehicle_building_entry(input),
                    cell_entry::decide_live_vehicle_building_entry(LiveVehicleBuildingEntry {
                        branch: VehicleBuildingEntryBranch::UnitRepairOrBunker,
                        ..input
                    }),
                )
            } else {
                (
                    BuildingOccupantEntryDecision::KeepBlocker,
                    BuildingOccupantEntryDecision::KeepBlocker,
                )
            };
            let bib_skip = vehicle_row_helpers
                && obj.bib
                && cx
                    .checked_add(1)
                    .is_some_and(|east_x| !foundation_cell_set.contains(&(east_x, cy)));
            if matches!(contact_skip, BuildingOccupantEntryDecision::SkipBlocker)
                || matches!(
                    second_callsite_skip,
                    BuildingOccupantEntryDecision::SkipBlocker
                )
                || bib_skip
                || gate_skip
            {
                skips
                    .entry((cx, cy))
                    .or_default()
                    .insert(building.stable_id);
            }
        }
    }
    skips
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
    live_building_entry_skips: &LiveBuildingEntrySkipMap,
    alliances: &HouseAllianceMap,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    rng: &mut SimRng,
    stats: &mut MovementTickStats,
    finished_entities: &mut Vec<u64>,
    crush_kills: &mut Vec<PendingCrushKill>,
    already_scattered: &mut BTreeSet<u64>,
    blockage_path_delay_ticks: u16,
    sim_tick: u64,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
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
    let crush_capability =
        bump_crush::CrushCapability::new(snap.regular_crusher, snap.omni_crusher);
    let mover_is_crusher = crush_capability.can_crush_units();
    let is_infantry = snap.category == EntityCategory::Infantry;
    if let Some(rules) = rules {
        crate::sim::gate_runtime::request_gate_open_for_cell(
            entities,
            occupancy,
            (nx, ny),
            object_list_layer,
            entity_id,
            interner.resolve(snap.owner),
            rules,
            alliances,
            interner,
        );
    }
    let entry_result = cell_entry::classify_occupied_cell_with_layers_and_ignored(
        (nx, ny),
        layer_context,
        entity_id,
        crush_capability,
        interner.resolve(snap.owner),
        mover_loco_kind,
        snap.bypass_grid,
        live_building_entry_skips.get(&(nx, ny)),
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
        CellEntryResult::Clear => {
            // Locomotor override (JumpJet) can clear a lower native code before
            // we reach this branch.
            if let Some(entity) = entities.get_mut(entity_id) {
                snap_motion_to_cell_center(&mut entity.position, &mut entity.drive_track);
                if let Some(ref mut target) = entity.movement_target {
                    target.blocked_delay = 0;
                    target.path_blocked = false;
                }
            }
        }
        CellEntryResult::ScatterRequired { .. } => {
            // Native code 3 is a soft blocked result for allied gate/building
            // contact. The opener request has already been issued above; the
            // mover must wait/repath instead of entering this occupied cell.
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
                        false,
                    );
                    debug_events.extend(evts);
                }
            }
        }
        CellEntryResult::Crushable { victims } => {
            let crusher_cell = (i32::from(nx), i32::from(ny));
            let crusher_lepton = (i32::from(nx) * 256 + 128, i32::from(ny) * 256 + 128);
            let victims = match bump_crush::classify_drive_crush_phase(
                bump_crush::DriveCrushPhase::FullyInCell,
                &victims,
                entities,
                entity_id,
                alliances,
                interner,
                crusher_lepton,
                crush_capability,
            ) {
                bump_crush::DriveCrushOutcome::Kill { victims } => victims,
                _ => Vec::new(),
            };
            let kill_set: BTreeSet<u64> = victims.iter().copied().collect();
            if let bump_crush::DriveCrushOutcome::Scatter { blockers } =
                bump_crush::classify_drive_crush_phase(
                    bump_crush::DriveCrushPhase::EnteringCell,
                    &victims,
                    entities,
                    entity_id,
                    alliances,
                    interner,
                    crusher_lepton,
                    crush_capability,
                )
            {
                for blocker_id in blockers {
                    if kill_set.contains(&blocker_id) {
                        continue;
                    }
                    if !already_scattered.contains(&blocker_id)
                        && bump_crush::scatter_blocker(
                            entities,
                            blocker_id,
                            path_grid,
                            occupancy,
                            object_list_layer,
                            rng,
                        )
                    {
                        already_scattered.insert(blocker_id);
                        stats.scatter_successes = stats.scatter_successes.saturating_add(1);
                    }
                }
            }
            // Remove crush victims from occupancy immediately (matches gamemd's
            // PerCellProcess which calls RemoveFromGame before continuing).
            for &vid in &victims {
                if let Some(v) = entities.get(vid) {
                    occupancy.remove(v.position.rx, v.position.ry, vid);
                }
            }
            crush_kills.extend(victims.into_iter().map(|victim_id| PendingCrushKill {
                victim_id,
                crusher_id: entity_id,
                crush_coord: crusher_cell,
            }));
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

        let eval = evaluate_runtime_can_enter_cell(
            Some(&grid),
            MovementLayer::Bridge,
            RuntimeCanEnterCellArgs::runtime((1, 0), 2, 0),
        );
        let layers = eval.layers;

        assert!(eval.bridge_traversal_allowed);
        assert_eq!(layers.terrain_layer, MovementLayer::Bridge);
        assert_eq!(layers.object_list_layer, MovementLayer::Bridge);
        assert_eq!(layers.occupancy_bits_layer, MovementLayer::Ground);
    }

    #[test]
    fn runtime_layers_resnapshot_bridge_occupancy_bits_at_deck_height() {
        let mut grid = PathGrid::new(2, 1);
        grid.set_cell_for_test(0, 0, 4, true, false);
        grid.set_cell_for_test(1, 0, 0, true, true);

        let eval = evaluate_runtime_can_enter_cell(
            Some(&grid),
            MovementLayer::Bridge,
            RuntimeCanEnterCellArgs::runtime((1, 0), 2, 4),
        );
        let layers = eval.layers;

        assert!(eval.bridge_traversal_allowed);
        assert_eq!(layers.object_list_layer, MovementLayer::Bridge);
        assert_eq!(layers.occupancy_bits_layer, MovementLayer::Bridge);
    }

    #[test]
    fn runtime_height_uses_current_cell_level_plus_on_bridge() {
        let mut grid = PathGrid::new(1, 1);
        grid.set_cell_for_test(0, 0, 2, false, false);

        assert_eq!(
            runtime_current_effective_height(Some(&grid), (0, 0), true, 99),
            6
        );
        assert_eq!(
            runtime_current_effective_height(Some(&grid), (0, 0), false, 99),
            2
        );
    }

    #[test]
    fn runtime_null_parent_reconstructs_predecessor_from_target_and_direction() {
        let mut grid = PathGrid::new(3, 1);
        grid.set_cell_for_test(0, 0, 0, false, false);
        grid.set_cell_for_test(1, 0, 4, true, false);
        grid.set_cell_for_test(2, 0, 0, true, true);

        let runtime_null_parent = evaluate_runtime_can_enter_cell(
            Some(&grid),
            MovementLayer::Bridge,
            RuntimeCanEnterCellArgs::runtime((2, 0), 2, 0),
        );
        let explicit_wrong_parent = evaluate_runtime_can_enter_cell(
            Some(&grid),
            MovementLayer::Bridge,
            RuntimeCanEnterCellArgs {
                parent_current_cell: Some((0, 0)),
                ..RuntimeCanEnterCellArgs::runtime((2, 0), 2, 0)
            },
        );

        assert!(runtime_null_parent.bridge_traversal_allowed);
        assert_eq!(
            runtime_null_parent.layers.object_list_layer,
            MovementLayer::Bridge
        );
        assert!(explicit_wrong_parent.bridge_traversal_allowed);
        assert_eq!(
            explicit_wrong_parent.layers.object_list_layer,
            MovementLayer::Ground
        );
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
            &LiveBuildingEntrySkipMap::new(),
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
            &LiveBuildingEntrySkipMap::new(),
        );

        assert_eq!(check, Some(DeferredCellCheck::Vehicle((5, 5), layers)));
    }

    #[test]
    fn deferred_detection_ignores_only_live_skipped_building_blocker() {
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            3,
            3,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        let layers = CanEnterLayerContext::single(MovementLayer::Ground);
        let mut skips = LiveBuildingEntrySkipMap::new();
        skips.entry((3, 3)).or_default().insert(10);

        let check = detect_deferred_cell_check(
            EntityCategory::Unit,
            false,
            layers,
            (3, 3),
            (2, 3),
            MovementLayer::Ground,
            &occupancy,
            &skips,
        );

        assert_eq!(check, None);
    }

    #[test]
    fn infantry_deferred_detection_ignores_only_live_skipped_gate_blocker() {
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            3,
            3,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        let layers = CanEnterLayerContext::single(MovementLayer::Ground);
        let mut skips = LiveBuildingEntrySkipMap::new();
        skips.entry((3, 3)).or_default().insert(10);

        let check = detect_deferred_cell_check(
            EntityCategory::Infantry,
            false,
            layers,
            (3, 3),
            (2, 3),
            MovementLayer::Ground,
            &occupancy,
            &skips,
        );

        assert_eq!(check, None);
    }

    #[test]
    fn deferred_detection_bypass_grid_still_checks_unskipped_structure() {
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            3,
            3,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        let layers = CanEnterLayerContext::single(MovementLayer::Ground);

        let check = detect_deferred_cell_check(
            EntityCategory::Unit,
            true,
            layers,
            (3, 3),
            (2, 3),
            MovementLayer::Ground,
            &occupancy,
            &LiveBuildingEntrySkipMap::new(),
        );

        assert_eq!(check, Some(DeferredCellCheck::Vehicle((3, 3), layers)));
    }

    #[test]
    fn deferred_detection_keeps_unrelated_blocker_in_live_skipped_cell() {
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            3,
            3,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        occupancy.add(
            3,
            3,
            20,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let layers = CanEnterLayerContext::single(MovementLayer::Ground);
        let mut skips = LiveBuildingEntrySkipMap::new();
        skips.entry((3, 3)).or_default().insert(10);

        let check = detect_deferred_cell_check(
            EntityCategory::Unit,
            false,
            layers,
            (3, 3),
            (2, 3),
            MovementLayer::Ground,
            &occupancy,
            &skips,
        );

        assert_eq!(check, Some(DeferredCellCheck::Vehicle((3, 3), layers)));
    }

    #[test]
    fn refinery_live_skip_map_opens_bib_east_edge_not_interior() {
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let ini = IniFile::from_str(
            "[VehicleTypes]\n0=HARV\n[BuildingTypes]\n0=GAREFN\n\
             [HARV]\nName=Harvester\nSpeed=4\n\
             [GAREFN]\nName=Refinery\nFoundation=4x3\nBib=yes\nNumberImpassableRows=3\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("refinery rules");
        let mut entities = EntityStore::new();
        let mut mover = GameEntity::test_default(1, "HARV", "Americans", 14, 11);
        mover.category = EntityCategory::Unit;
        entities.insert(mover);
        let mut refinery = GameEntity::test_default(100, "GAREFN", "Americans", 10, 10);
        refinery.category = EntityCategory::Structure;
        entities.insert(refinery);
        let interner = crate::sim::intern::test_interner();

        let skips = build_live_building_entry_skip_map(&entities, 1, &interner, Some(&rules));

        assert!(skips.get(&(13, 11)).is_some_and(|ids| ids.contains(&100)));
        assert!(!skips.get(&(12, 11)).is_some_and(|ids| ids.contains(&100)));
    }

    #[test]
    fn refinery_contact_number_rows_opens_first_clear_column_only() {
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let ini = IniFile::from_str(
            "[VehicleTypes]\n0=HARV\n[BuildingTypes]\n0=GAREFN\n\
             [HARV]\nName=Harvester\nSpeed=4\n\
             [GAREFN]\nName=Refinery\nFoundation=4x3\nBib=no\nNumberImpassableRows=3\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("refinery rules");
        let mut entities = EntityStore::new();
        let mut mover = GameEntity::test_default(1, "HARV", "Americans", 14, 11);
        mover.category = EntityCategory::Unit;
        mover.mark_live_contact_with(100);
        entities.insert(mover);
        let mut refinery = GameEntity::test_default(100, "GAREFN", "Americans", 10, 10);
        refinery.category = EntityCategory::Structure;
        entities.insert(refinery);
        let interner = crate::sim::intern::test_interner();

        let skips = build_live_building_entry_skip_map(&entities, 1, &interner, Some(&rules));

        assert!(skips.get(&(13, 11)).is_some_and(|ids| ids.contains(&100)));
        assert!(!skips.get(&(12, 11)).is_some_and(|ids| ids.contains(&100)));
    }

    #[test]
    fn open_gate_skip_map_requires_mission_18_and_stable_open() {
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::{BuildingGatePhase, BuildingGateRuntime, GameEntity};

        let ini = IniFile::from_str(
            "[VehicleTypes]\n0=MTNK\n[BuildingTypes]\n0=GAGATE_A\n\
             [MTNK]\nName=Tank\nSpeed=4\n\
             [GAGATE_A]\nName=Allied Gate\nFoundation=3x1\nGate=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("gate rules");

        let mut entities = EntityStore::new();
        let mut mover = GameEntity::test_default(1, "MTNK", "Americans", 8, 10);
        mover.category = EntityCategory::Unit;
        entities.insert(mover);
        let mut gate = GameEntity::test_default(100, "GAGATE_A", "Americans", 10, 10);
        gate.category = EntityCategory::Structure;
        gate.building_gate = Some(BuildingGateRuntime {
            mission_18_active: true,
            phase: BuildingGatePhase::OpenStable,
            ..Default::default()
        });
        entities.insert(gate);
        let interner = crate::sim::intern::test_interner();

        let skips = build_live_building_entry_skip_map(&entities, 1, &interner, Some(&rules));
        assert!(skips.get(&(10, 10)).is_some_and(|ids| ids.contains(&100)));
        assert!(skips.get(&(11, 10)).is_some_and(|ids| ids.contains(&100)));
        assert!(skips.get(&(12, 10)).is_some_and(|ids| ids.contains(&100)));

        entities.get_mut(100).unwrap().building_gate = Some(BuildingGateRuntime {
            mission_18_active: true,
            phase: BuildingGatePhase::Opening,
            ..Default::default()
        });
        let skips = build_live_building_entry_skip_map(&entities, 1, &interner, Some(&rules));
        assert!(!skips.get(&(10, 10)).is_some_and(|ids| ids.contains(&100)));

        entities.get_mut(100).unwrap().building_gate = Some(BuildingGateRuntime {
            mission_18_active: false,
            phase: BuildingGatePhase::OpenStable,
            ..Default::default()
        });
        let skips = build_live_building_entry_skip_map(&entities, 1, &interner, Some(&rules));
        assert!(!skips.get(&(10, 10)).is_some_and(|ids| ids.contains(&100)));
    }

    #[test]
    fn infantry_uses_open_gate_skip_without_vehicle_row_helpers() {
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::{BuildingGatePhase, BuildingGateRuntime, GameEntity};

        let ini = IniFile::from_str(
            "[InfantryTypes]\n0=E1\n[BuildingTypes]\n0=GAGATE_A\n\
             [E1]\nName=GI\nSpeed=4\n\
             [GAGATE_A]\nName=Allied Gate\nFoundation=3x1\nGate=yes\nNumberImpassableRows=0\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("gate rules");
        let mut entities = EntityStore::new();
        let mut mover = GameEntity::test_default(1, "E1", "Americans", 8, 10);
        mover.category = EntityCategory::Infantry;
        entities.insert(mover);
        let mut gate = GameEntity::test_default(100, "GAGATE_A", "Americans", 10, 10);
        gate.category = EntityCategory::Structure;
        gate.building_gate = Some(BuildingGateRuntime {
            mission_18_active: true,
            phase: BuildingGatePhase::OpenStable,
            ..Default::default()
        });
        entities.insert(gate);
        let interner = crate::sim::intern::test_interner();

        let skips = build_live_building_entry_skip_map(&entities, 1, &interner, Some(&rules));

        assert!(skips.get(&(10, 10)).is_some_and(|ids| ids.contains(&100)));
    }
}
