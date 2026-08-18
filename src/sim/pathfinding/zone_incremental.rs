//! Incremental zone updates and exact one-cell topology repair.
//!
//! Terrain-aware mutation owners use the explicit packed-coordinate
//! `AssignOrphaned` / `MergeAdjacent` contract and shared hierarchy patcher.
//! The older PathGrid-only compatibility path still batches a small region:
//! 1. Identifies which zone IDs are affected (have cells in the changed region).
//! 2. Clears those zone IDs everywhere on the map.
//! 3. Re-flood-fills the cleared cells to assign new zone IDs.
//! 4. Rebuilds adjacency and super-zone labels for affected categories.
//!
//! It falls back to full rebuild if too many cells changed, terrain-aware
//! provenance is unavailable, or its legacy IDs approach exhaustion.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/zone_map, sim/zone_build, sim/zone_hierarchy.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::{BTreeMap, BTreeSet};

use super::PathGrid;
use super::terrain_cost::TerrainCostGrid;
use super::zone_build::{
    BridgeRecordFilter, LocalHierarchyPatchResult, build_bridge_redirect, build_zone_hierarchy,
    compute_zone_info, extract_adjacency, flood_fill,
    incremental_rebuild_zone_hierarchy_around_cell, inject_bridge_adjacency, is_passable,
};
use super::zone_hierarchy::SuperZoneMap;
use super::zone_map::{ZONE_INVALID, ZoneGrid, ZoneId};
use crate::map::resolved_terrain::{ResolvedTerrainGrid, zone_class};
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::movement::locomotor::MovementLayer;

/// Maximum changed cells before falling back to full rebuild.
pub(crate) const INCREMENTAL_THRESHOLD: usize = 200;

/// Force full rebuild to compact zone IDs when count approaches u16 max.
const ZONE_ID_COMPACTION_THRESHOLD: u16 = 60_000;

/// Padding around changed cells for the affected bounding box.
const BBOX_PADDING: u16 = 2;

/// Native `CellStruct`: signed X in the low word, signed Y in the high word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PackedZoneCoord(u32);

impl PackedZoneCoord {
    pub(crate) const fn new(x: i16, y: i16) -> Self {
        Self((x as u16 as u32) | ((y as u16 as u32) << 16))
    }

    pub(crate) const fn unpack(self) -> (i16, i16) {
        (self.0 as u16 as i16, (self.0 >> 16) as u16 as i16)
    }
}

/// Provenance selects between the two distinct native one-cell helpers —
/// `MapClass::AssignOrphanedCellZone` @ `0x0056D460` and
/// `MapClass::MergeAdjacentCellZone` @ `0x0056D5A0`. It is never inferred from
/// the target's current class: gamemd's distinction is which helper the caller
/// chose, not anything readable off the cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZoneRepairKind {
    AssignOrphaned,
    // Verified repair arm retained until its terrain mutation owner is wired.
    #[allow(dead_code)]
    MergeAdjacent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZoneRepairOutcome {
    OutsideNoOp,
    SentinelNoOp,
    Adopted { cluster: ZoneId },
    FullRebuild,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaseRepairDecision {
    SentinelNoOp,
    Adopt(ZoneId),
    FullRebuild,
}

/// Apply one verified base-cluster repair and then the shared local hierarchy
/// updater, `MapClass::IncrementalRebuildZoneGraphAroundCell` @ `0x00584550`.
///
/// The ordering is verified exhaustively by callers: all nine native callers of
/// `AssignOrphanedCellZone`/`MergeAdjacentCellZone` call `0x00584550`
/// unconditionally 11-15 bytes later, and every xref is an unconditional call.
/// They are `AnimClass::Middle`, the area-damage helper,
/// `CellClass::DestroyOverlay`, the post-destruction wall cleanup,
/// sell-building-at-cell, `TerrainClass::Limbo`,
/// `BuildingClass::Place_OccupyMap`, `OverlayClass::Mark`, and `FUN_0074E930`
/// (call sites `0x0074EA05` / `0x0074EA10`).
///
/// Current mutation owners wire the explicit provenance in their own Phase-3
/// items; callers without it must keep using a full rebuild.
pub(crate) fn repair_zone_cell(
    zone_grid: &mut ZoneGrid,
    packed_coord: PackedZoneCoord,
    kind: ZoneRepairKind,
    path_grid: &PathGrid,
    _terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    resolved_terrain: &ResolvedTerrainGrid,
    bridge_records: &[crate::sim::bridge_state::BridgeEndpointRecord],
) -> ZoneRepairOutcome {
    let coord = packed_coord.unpack();
    let (x, y) = (i32::from(coord.0), i32::from(coord.1));
    let width = zone_grid.width;
    let height = zone_grid.height;
    if x < 0
        || y < 0
        || x >= i32::from(width)
        || y >= i32::from(height)
        || resolved_terrain
            .cell(x as u16, y as u16)
            .is_none_or(|cell| cell.outside_playfield)
    {
        return ZoneRepairOutcome::OutsideNoOp;
    }
    let index = y as usize * width as usize + x as usize;
    let current_type = resolved_terrain
        .cell(x as u16, y as u16)
        .map_or(zone_class::OUTSIDE, |cell| cell.zone_type);

    let decision = zone_grid
        .base_topology_mut()
        .map(|base| {
            base.movement_classes[index] = current_type;
            decide_base_zone_repair(base, index, x, y, width, height, kind)
        })
        .unwrap_or(BaseRepairDecision::FullRebuild);

    let outcome = match decision {
        BaseRepairDecision::SentinelNoOp => ZoneRepairOutcome::SentinelNoOp,
        BaseRepairDecision::Adopt(cluster) => {
            if let Some(base) = zone_grid.base_topology_mut() {
                base.zone_ids[index] = cluster;
            }
            zone_grid.project_adopted_base_cell(index);
            ZoneRepairOutcome::Adopted { cluster }
        }
        BaseRepairDecision::FullRebuild => {
            zone_grid.rebuild_base_connectivity_preserving_hierarchy(
                path_grid,
                resolved_terrain,
                bridge_records,
            );
            ZoneRepairOutcome::FullRebuild
        }
    };

    let patch_result = zone_grid
        .base_and_hierarchy_mut()
        .map(|(base, hierarchy)| {
            incremental_rebuild_zone_hierarchy_around_cell(
                hierarchy,
                base,
                path_grid,
                resolved_terrain,
                bridge_records,
                coord,
                width,
                height,
            )
        })
        .unwrap_or(LocalHierarchyPatchResult::NeedsFullRebuild);
    if patch_result == LocalHierarchyPatchResult::NeedsFullRebuild
        && let Some(base) = zone_grid.base_topology_mut().map(|base| base.clone())
    {
        zone_grid.replace_hierarchy(build_zone_hierarchy(
            &base,
            path_grid,
            Some(resolved_terrain),
            bridge_records,
            width,
            height,
        ));
    }

    outcome
}

fn decide_base_zone_repair(
    base: &super::zone_build::BaseZoneTopology,
    target_index: usize,
    x: i32,
    y: i32,
    width: u16,
    height: u16,
    kind: ZoneRepairKind,
) -> BaseRepairDecision {
    let target_type = base.movement_classes[target_index];
    if target_type == zone_class::OUTSIDE {
        return BaseRepairDecision::SentinelNoOp;
    }

    let neighbors: [(u8, ZoneId); 8] = std::array::from_fn(|neighbor_index| {
        let (dx, dy, _) = super::zone_build::NEIGHBORS[neighbor_index];
        let nx = x + dx;
        let ny = y + dy;
        if nx < 0 || ny < 0 || nx >= i32::from(width) || ny >= i32::from(height) {
            return (zone_class::OUTSIDE, ZONE_INVALID);
        }
        let index = ny as usize * width as usize + nx as usize;
        (
            base.movement_classes[index],
            base.zone_ids.get(index).copied().unwrap_or(ZONE_INVALID),
        )
    });

    let candidate = neighbors.iter().find(|&&(neighbor_type, _)| match kind {
        ZoneRepairKind::AssignOrphaned => neighbor_type == zone_class::GROUND,
        ZoneRepairKind::MergeAdjacent => neighbor_type == target_type,
    });
    let Some(&(_, candidate_cluster)) = candidate else {
        return BaseRepairDecision::FullRebuild;
    };

    let row0 = &base.raw_zone_ids_by_row[0];
    let mapped = |cluster: ZoneId| {
        if cluster == ZONE_INVALID {
            u16::MAX
        } else {
            row0.get(cluster as usize).copied().unwrap_or(u16::MAX)
        }
    };
    let mut previous_cluster = ZONE_INVALID;
    let mut transitions = 0u8;
    for &(neighbor_type, cluster) in &neighbors {
        if mapped(cluster) != mapped(previous_cluster) && neighbor_type != zone_class::OUTSIDE {
            transitions += 1;
            previous_cluster = cluster;
        }
    }
    if transitions >= 4
        || (kind == ZoneRepairKind::AssignOrphaned && target_type != zone_class::GROUND)
    {
        BaseRepairDecision::FullRebuild
    } else {
        BaseRepairDecision::Adopt(candidate_cluster)
    }
}

/// Attempt an incremental zone update for the given changed cells.
///
/// Returns `true` if the incremental update succeeded. Returns `false` if a
/// full rebuild is needed (too many changes, zone ID exhaustion, etc.).
pub(crate) fn try_incremental_update(
    zone_grid: &mut ZoneGrid,
    changed_cells: &[(u16, u16)],
    path_grid: &PathGrid,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    bridge_records: &[crate::sim::bridge_state::BridgeEndpointRecord],
) -> bool {
    if changed_cells.is_empty() {
        return true;
    }
    if resolved_terrain.is_some() {
        // A batch carries no verified Assign-vs-Merge provenance. Exact
        // terrain-aware callers use `repair_zone_cell`; this generic path must
        // full-rebuild rather than infer the helper from current class.
        return false;
    }
    if changed_cells.len() > INCREMENTAL_THRESHOLD {
        return false;
    }

    let width = zone_grid.width;
    let height = zone_grid.height;

    // Compute bounding box of changed cells with padding.
    let bbox = padded_bbox(changed_cells, width, height);

    // Process each category. We do two passes per category:
    // Pass 1 (mutable): clear + re-flood-fill zone IDs.
    // Pass 2 (immutable reads, then mutable writes): rebuild adjacency/info/super-zones.
    for &mz in MovementZone::all_ground() {
        if !update_category(
            zone_grid,
            mz,
            &bbox,
            changed_cells,
            path_grid,
            terrain_costs,
            resolved_terrain,
            bridge_records,
            width,
            height,
        ) {
            return false;
        }
    }

    true
}

/// Update one movement zone incrementally. Returns false if full rebuild needed.
fn update_category(
    zone_grid: &mut ZoneGrid,
    mz: MovementZone,
    bbox: &(u16, u16, u16, u16),
    changed_cells: &[(u16, u16)],
    path_grid: &PathGrid,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    bridge_records: &[crate::sim::bridge_state::BridgeEndpointRecord],
    width: u16,
    height: u16,
) -> bool {
    let speed_type = mz.speed_type();
    let cost_grid = terrain_costs.get(&speed_type);
    let w = width as usize;
    let (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y) = *bbox;

    let Some(zone_map) = zone_grid.map_mut(mz) else {
        return true;
    };

    // Check zone ID exhaustion.
    if zone_map.zone_count >= ZONE_ID_COMPACTION_THRESHOLD {
        return false;
    }

    // --- Pass 1: Collect affected zones, clear, re-flood-fill ---

    // Collect affected ground zone IDs inside bbox.
    let mut affected_ground: BTreeSet<ZoneId> = BTreeSet::new();
    for ry in bbox_min_y..=bbox_max_y {
        for rx in bbox_min_x..=bbox_max_x {
            let idx = ry as usize * w + rx as usize;
            let zid = zone_map.zone_ids_slice()[idx];
            if zid != ZONE_INVALID {
                affected_ground.insert(zid);
            }
        }
    }

    // If no zones affected, check if newly-passable cells appeared.
    if affected_ground.is_empty() {
        let any_new_passable = changed_cells.iter().any(|&(cx, cy)| {
            is_passable(
                cx,
                cy,
                mz,
                path_grid,
                cost_grid,
                None,
                MovementLayer::Ground,
            )
        });
        if !any_new_passable {
            return true; // nothing to do for this movement zone
        }
    }

    // Clear affected zone IDs everywhere.
    let ground_ids = zone_map.zone_ids_mut();
    for zid in ground_ids.iter_mut() {
        if affected_ground.contains(zid) {
            *zid = ZONE_INVALID;
        }
    }
    // Re-flood-fill cleared ground cells.
    let mut next_zone = zone_map.zone_count + 1;
    let ground_ids = zone_map.zone_ids_mut();
    for ry in 0..height {
        for rx in 0..width {
            let idx = ry as usize * w + rx as usize;
            if ground_ids[idx] != ZONE_INVALID {
                continue;
            }
            if !is_passable(
                rx,
                ry,
                mz,
                path_grid,
                cost_grid,
                None,
                MovementLayer::Ground,
            ) {
                continue;
            }
            flood_fill(
                rx,
                ry,
                next_zone,
                ground_ids,
                width,
                height,
                mz,
                path_grid,
                cost_grid,
                None,
                MovementLayer::Ground,
            );
            next_zone += 1;
        }
    }

    let new_zone_count = next_zone - 1;
    zone_map.set_zone_count(new_zone_count);

    // --- Pass 2: Rebuild adjacency, zone_info, super-zones ---
    let Some(zone_map) = zone_grid.map_for(mz) else {
        return false;
    };
    let ground_slice = zone_map.zone_ids_slice();

    let mut new_adj = extract_adjacency(ground_slice, width, height, new_zone_count);

    // Inject bridge adjacency for ground-capable movement zones.
    if mz.can_use_bridges() {
        inject_bridge_adjacency(
            &mut new_adj,
            ground_slice,
            bridge_records,
            width,
            BridgeRecordFilter::AllActive,
        );
    }

    let new_info = compute_zone_info(ground_slice, width, height, new_zone_count);
    let new_sz = SuperZoneMap::from_adjacency(&new_adj, new_zone_count);

    // Rebuild bridge redirect.
    let bridge_redirect = if mz.can_use_bridges() {
        build_bridge_redirect(path_grid, resolved_terrain, bridge_records, width, height)
    } else {
        None
    };

    // Apply computed results back.
    let Some(zone_map) = zone_grid.map_mut(mz) else {
        return false;
    };
    zone_map.set_zone_info(new_info);
    zone_map.set_bridge_redirect(bridge_redirect);

    if let Some(adj) = zone_grid.adjacency_mut(mz) {
        *adj = new_adj;
    }
    zone_grid.set_super_zone(mz, new_sz);

    true
}

/// Compute the padded bounding box around changed cells, clamped to map bounds.
fn padded_bbox(changed_cells: &[(u16, u16)], width: u16, height: u16) -> (u16, u16, u16, u16) {
    let mut min_x = u16::MAX;
    let mut min_y = u16::MAX;
    let mut max_x = 0u16;
    let mut max_y = 0u16;
    for &(x, y) in changed_cells {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (
        min_x.saturating_sub(BBOX_PADDING),
        min_y.saturating_sub(BBOX_PADDING),
        (max_x + BBOX_PADDING).min(width - 1),
        (max_y + BBOX_PADDING).min(height - 1),
    )
}
