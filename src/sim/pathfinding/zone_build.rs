//! Zone map construction: flood-fill, adjacency extraction, zone info computation.
//!
//! Extracted from zone_map.rs to keep each file under ~400 lines.
//! This module is private to sim/ — public API lives in zone_map.rs.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/pathfinding, sim/terrain_cost, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::VecDeque;

use super::PathGrid;
use super::passability;
use super::terrain_cost::TerrainCostGrid;
use super::zone_map::{ZONE_INVALID, ZoneAdjacency, ZoneId, ZoneInfo, ZoneMap};
use crate::map::resolved_terrain::{ResolvedTerrainGrid, zone_class};
use crate::rules::locomotor_type::MovementZone;
use crate::sim::bridge_state::BridgeEndpointRecord;
use crate::sim::movement::locomotor::MovementLayer;

/// 8-directional neighbor offsets: (dx, dy, is_diagonal).
pub(crate) const NEIGHBORS: [(i32, i32, bool); 8] = [
    (0, -1, false), // N
    (1, -1, true),  // NE
    (1, 0, false),  // E
    (1, 1, true),   // SE
    (0, 1, false),  // S
    (-1, 1, true),  // SW
    (-1, 0, false), // W
    (-1, -1, true), // NW
];

/// Shared persistent topology projected through all 13 MovementZone rows.
pub(crate) struct BaseZoneTopology {
    movement_classes: Vec<u8>,
    zone_ids: Vec<ZoneId>,
    zone_count: ZoneId,
    adjacency: ZoneAdjacency,
}

struct BaseEdgeBuckets {
    buckets: Vec<Vec<(ZoneId, ZoneId)>>,
}

impl BaseEdgeBuckets {
    fn new() -> Self {
        Self {
            buckets: vec![Vec::new(); 256],
        }
    }

    fn register(&mut self, neighbor: ZoneId, current: ZoneId) {
        if neighbor == ZONE_INVALID || current == ZONE_INVALID || neighbor == current {
            return;
        }
        let bucket = (((neighbor & 0x0f) << 4) | (current & 0x0f)) as usize;
        let pair = (neighbor, current);
        if !self.buckets[bucket].contains(&pair) {
            self.buckets[bucket].push(pair);
        }
    }

    fn into_adjacency(self, zone_count: ZoneId) -> ZoneAdjacency {
        let mut adjacency = vec![Vec::new(); zone_count as usize + 1];
        for bucket in self.buckets {
            for (neighbor, current) in bucket {
                add_adjacency(&mut adjacency, neighbor, current);
            }
        }
        ZoneAdjacency::new(adjacency)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BridgeRecordFilter {
    /// `UpdateBridgeZonesHelper @ 0x0056C510` uses active records while adding
    /// bridge zone edges; the verified loop does not read `bridge_kind`.
    AllActive,
    /// `FindBridgeRecord @ 0x0056DA10` skips records where `bridge_kind != 0`.
    HighActiveOnly,
}

fn bridge_record_matches(record: &BridgeEndpointRecord, filter: BridgeRecordFilter) -> bool {
    record.active
        && match filter {
            BridgeRecordFilter::AllActive => true,
            BridgeRecordFilter::HighActiveOnly => record.is_high(),
        }
}

/// Build a zone map and adjacency graph for one MovementZone.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn build_zone_map(
    path_grid: &PathGrid,
    cost_grid: Option<&TerrainCostGrid>,
    mz: MovementZone,
    width: u16,
    height: u16,
) -> (ZoneMap, ZoneAdjacency) {
    build_zone_map_with_terrain(path_grid, cost_grid, None, mz, width, height)
}

/// Build a zone map using shared base-zone semantics when resolved terrain is available.
/// Falls back to the older direct passable-cell flood-fill when resolved terrain is not
/// available (primarily tests and non-terrain-aware call sites).
pub(crate) fn build_zone_map_with_terrain(
    path_grid: &PathGrid,
    cost_grid: Option<&TerrainCostGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    mz: MovementZone,
    width: u16,
    height: u16,
) -> (ZoneMap, ZoneAdjacency) {
    if let Some(terrain) = resolved_terrain {
        let base = build_base_zone_topology(path_grid, terrain, &[], width, height);
        return build_zone_map_from_base_topology(&base, mz, width, height);
    }

    let total = width as usize * height as usize;

    // -- Ground layer flood-fill --
    let mut zone_ids = vec![ZONE_INVALID; total];
    let mut next_zone: ZoneId = 1;

    // Row-major scan for deterministic zone assignment.
    for ry in 0..height {
        for rx in 0..width {
            let idx = ry as usize * width as usize + rx as usize;
            if zone_ids[idx] != ZONE_INVALID {
                continue;
            }
            if !is_passable(
                rx,
                ry,
                mz,
                path_grid,
                cost_grid,
                resolved_terrain,
                MovementLayer::Ground,
            ) {
                continue;
            }
            // BFS flood-fill from this cell.
            flood_fill(
                rx,
                ry,
                next_zone,
                &mut zone_ids,
                width,
                height,
                mz,
                path_grid,
                cost_grid,
                resolved_terrain,
                MovementLayer::Ground,
            );
            next_zone += 1;
        }
    }

    let zone_count = next_zone - 1;

    // -- Extract adjacency (ground only; bridge edges injected by caller) --
    let adj = extract_adjacency(&zone_ids, width, height, zone_count);

    let zone_info = compute_zone_info(&zone_ids, width, height, zone_count);

    let zone_map = ZoneMap::new(
        zone_ids, None, // bridge_redirect set by caller
        width, height, zone_count, zone_info,
    );

    (zone_map, adj)
}

/// Build the one native base topology shared by every MovementZone projection.
pub(crate) fn build_base_zone_topology(
    path_grid: &PathGrid,
    resolved_terrain: &ResolvedTerrainGrid,
    bridge_records: &[BridgeEndpointRecord],
    width: u16,
    height: u16,
) -> BaseZoneTopology {
    let movement_classes: Vec<u8> = (0..height)
        .flat_map(|ry| {
            (0..width).map(move |rx| movement_class_for_cell(path_grid, resolved_terrain, rx, ry))
        })
        .collect();

    let (zone_ids, zone_count, mut edge_buckets) =
        rebuild_node_indices(&movement_classes, path_grid, width, height);
    register_bridge_base_edges(
        &mut edge_buckets,
        &zone_ids,
        bridge_records,
        width,
    );
    let adjacency = edge_buckets.into_adjacency(zone_count);

    BaseZoneTopology {
        movement_classes,
        zone_ids,
        zone_count,
        adjacency,
    }
}

/// Project the shared base topology through one exact MovementZone matrix row.
pub(crate) fn build_zone_map_from_base_topology(
    base: &BaseZoneTopology,
    movement_zone: MovementZone,
    width: u16,
    height: u16,
) -> (ZoneMap, ZoneAdjacency) {
    let derived_by_base = rebuild_zone_ids_for_movement_zone(
        &base.movement_classes,
        &base.zone_ids,
        base.zone_count,
        &base.adjacency.neighbors,
        movement_zone,
    );
    let zone_ids: Vec<ZoneId> = base
        .zone_ids
        .iter()
        .map(|&base_id| {
            if base_id == ZONE_INVALID {
                return ZONE_INVALID;
            }
            let derived = derived_by_base[base_id as usize];
            (derived > 1 && derived != u16::MAX)
                .then_some(derived)
                .unwrap_or(ZONE_INVALID)
        })
        .collect();
    let zone_count = zone_ids.iter().copied().max().unwrap_or(ZONE_INVALID);

    // Exact derived IDs already are the reachability components. Distinct IDs
    // must not be reconnected by a second flat cell-boundary graph.
    let adj = ZoneAdjacency::new(vec![Vec::new(); zone_count as usize + 1]);
    let zone_info = compute_zone_info(&zone_ids, width, height, zone_count);
    let zone_map = ZoneMap::new(
        zone_ids, None, // bridge_redirect set by caller
        width, height, zone_count, zone_info,
    );

    (zone_map, adj)
}

fn movement_class_for_cell(
    path_grid: &PathGrid,
    resolved_terrain: &ResolvedTerrainGrid,
    x: u16,
    y: u16,
) -> u8 {
    let Some(cell) = resolved_terrain.cell(x, y) else {
        return zone_class::OUTSIDE;
    };

    // Buildings are entity-based, not stored on ResolvedTerrainCell.
    // Check PathGrid for building footprints → class 5 (Building).
    // Only override if the cached zone_type isn't already a stronger blocker.
    if cell.zone_type < zone_class::BUILDING && !path_grid.is_walkable(x, y) && !cell.is_water {
        return zone_class::BUILDING;
    }

    cell.zone_type
}

fn rebuild_node_indices(
    movement_classes: &[u8],
    path_grid: &PathGrid,
    width: u16,
    height: u16,
) -> (Vec<u16>, u16, BaseEdgeBuckets) {
    let mut node_indices = vec![0u16; movement_classes.len()];
    let mut next_node: u16 = 1;
    let mut edge_buckets = BaseEdgeBuckets::new();

    for ry in 0..height {
        let mut rx = 0i32;
        while rx < i32::from(width) {
            if rx < 0 {
                rx += 1;
                continue;
            }
            let idx = ry as usize * width as usize + rx as usize;
            if movement_classes[idx] == zone_class::OUTSIDE || node_indices[idx] != 0 {
                rx += 1;
                continue;
            }
            let run_advance = flood_fill_node_index(
                rx as u16,
                ry,
                next_node,
                movement_classes,
                &mut node_indices,
                path_grid,
                width,
                height,
                &mut edge_buckets,
            );
            next_node += 1;
            // Native rechecks the returned rightmost run cell; the ordinary
            // assigned-cell branch then advances past it.
            rx += run_advance;
        }
    }

    (
        node_indices,
        next_node.saturating_sub(1),
        edge_buckets,
    )
}

fn flood_fill_node_index(
    start_x: u16,
    start_y: u16,
    node_id: u16,
    movement_classes: &[u8],
    node_indices: &mut [u16],
    path_grid: &PathGrid,
    width: u16,
    height: u16,
    edge_buckets: &mut BaseEdgeBuckets,
) -> i32 {
    let start_idx = start_y as usize * width as usize + start_x as usize;
    let movement_class = movement_classes[start_idx];
    let seed_x = i32::from(start_x);
    let seed_y = i32::from(start_y);
    let Some(mut carried_level) = base_level_at(path_grid, seed_x, seed_y, width, height)
    else {
        return 0;
    };

    // The native scan first walks left with a carried <=1 height reference.
    // Horizontal scans overwrite prior zone IDs; only vertical recursion tests
    // for an unassigned candidate.
    let mut left = seed_x;
    loop {
        let Some(index) = base_record_index(left, seed_y, width, height) else {
            break;
        };
        if movement_classes[index] != movement_class {
            break;
        }
        let Some(level) = base_level_at(path_grid, left, seed_y, width, height) else {
            break;
        };
        if (i16::from(level) - i16::from(carried_level)).abs() >= 2 {
            break;
        }
        node_indices[index] = node_id;
        carried_level = level;
        left -= 1;
    }
    register_scanline_edge(
        edge_buckets,
        left,
        seed_y,
        left + 1,
        seed_y,
        node_id,
        movement_class,
        node_indices,
        path_grid,
        width,
        height,
    );

    // Right starts at the seed but retains the level carried out of the left
    // scan. The retail comparison is strictly <4.
    let mut right = seed_x;
    loop {
        let Some(index) = base_record_index(right, seed_y, width, height) else {
            break;
        };
        if movement_classes[index] != movement_class {
            break;
        }
        let Some(level) = base_level_at(path_grid, right, seed_y, width, height) else {
            break;
        };
        if (i16::from(level) - i16::from(carried_level)).abs() >= 4 {
            break;
        }
        node_indices[index] = node_id;
        carried_level = level;
        right += 1;
    }
    register_scanline_edge(
        edge_buckets,
        right,
        seed_y,
        right - 1,
        seed_y,
        node_id,
        movement_class,
        node_indices,
        path_grid,
        width,
        height,
    );

    // Both adjacent-row scans include the two flanks [L-1, R+1]. Their
    // current-row reference is one cell right, clamped to R.
    let mut scan_x = left;
    let run_right = right - 1;
    while scan_x <= right {
        let candidate_y = seed_y - 1;
        let reference_x = (scan_x + 1).min(run_right);
        if let (Some(candidate), Some(candidate_level), Some(reference_level)) = (
            base_record_index(scan_x, candidate_y, width, height),
            base_level_at(path_grid, scan_x, candidate_y, width, height),
            base_level_at(path_grid, reference_x, seed_y, width, height),
        ) {
            let neighbor = node_indices[candidate];
            let height_allowed =
                (i16::from(candidate_level) - i16::from(reference_level)).abs() < 2;
            if neighbor == 0 && movement_classes[candidate] == movement_class && height_allowed {
                let child_advance = flood_fill_node_index(
                    scan_x as u16,
                    candidate_y as u16,
                    node_id,
                    movement_classes,
                    node_indices,
                    path_grid,
                    width,
                    height,
                    edge_buckets,
                );
                // Above advances beyond the returned child run.
                scan_x += child_advance + 1;
                continue;
            } else if neighbor != 0
                && neighbor != node_id
                && (height_allowed || movement_class == zone_class::IMPASSABLE)
            {
                edge_buckets.register(neighbor, node_id);
            }
        }
        scan_x += 1;
    }

    // Below uses the same range/reference mapping, but rechecks the returned
    // child run's rightmost cell once before advancing.
    scan_x = left;
    while scan_x <= right {
        let candidate_y = seed_y + 1;
        let reference_x = (scan_x + 1).min(run_right);
        if let (Some(candidate), Some(candidate_level), Some(reference_level)) = (
            base_record_index(scan_x, candidate_y, width, height),
            base_level_at(path_grid, scan_x, candidate_y, width, height),
            base_level_at(path_grid, reference_x, seed_y, width, height),
        ) {
            let neighbor = node_indices[candidate];
            let height_allowed =
                (i16::from(candidate_level) - i16::from(reference_level)).abs() < 2;
            if neighbor == 0 && movement_classes[candidate] == movement_class && height_allowed {
                let child_advance = flood_fill_node_index(
                    scan_x as u16,
                    candidate_y as u16,
                    node_id,
                    movement_classes,
                    node_indices,
                    path_grid,
                    width,
                    height,
                    edge_buckets,
                );
                scan_x += child_advance;
                continue;
            } else if neighbor != 0
                && neighbor != node_id
                && (height_allowed || movement_class == zone_class::IMPASSABLE)
            {
                edge_buckets.register(neighbor, node_id);
            }
        }
        scan_x += 1;
    }

    right - seed_x - 1
}

fn base_record_index(x: i32, y: i32, width: u16, height: u16) -> Option<usize> {
    if x < 0 || y < 0 || x >= i32::from(width) || y >= i32::from(height) {
        return None;
    }
    Some(y as usize * width as usize + x as usize)
}

fn base_level_at(
    path_grid: &PathGrid,
    x: i32,
    y: i32,
    width: u16,
    height: u16,
) -> Option<u8> {
    base_record_index(x, y, width, height)?;
    path_grid
        .cell(x as u16, y as u16)
        .map(|cell| cell.ground_level)
}

#[allow(clippy::too_many_arguments)]
fn register_scanline_edge(
    edge_buckets: &mut BaseEdgeBuckets,
    candidate_x: i32,
    candidate_y: i32,
    reference_x: i32,
    reference_y: i32,
    current_zone: ZoneId,
    captured_class: u8,
    node_indices: &[u16],
    path_grid: &PathGrid,
    width: u16,
    height: u16,
) {
    let Some(candidate) = base_record_index(candidate_x, candidate_y, width, height) else {
        return;
    };
    let neighbor = node_indices[candidate];
    if neighbor == ZONE_INVALID || neighbor == current_zone {
        return;
    }
    let height_allowed = match (
        base_level_at(path_grid, candidate_x, candidate_y, width, height),
        base_level_at(path_grid, reference_x, reference_y, width, height),
    ) {
        (Some(candidate_level), Some(reference_level)) => {
            (i16::from(candidate_level) - i16::from(reference_level)).abs() < 2
        }
        _ => false,
    };
    if height_allowed || captured_class == zone_class::IMPASSABLE {
        edge_buckets.register(neighbor, current_zone);
    }
}

fn rebuild_zone_ids_for_movement_zone(
    movement_classes: &[u8],
    node_indices: &[u16],
    node_count: u16,
    node_adj: &[Vec<u16>],
    movement_zone: MovementZone,
) -> Vec<u16> {
    let mut node_movement_classes = vec![zone_class::OUTSIDE; node_count as usize + 1];
    for (&node, &movement_class) in node_indices.iter().zip(movement_classes.iter()) {
        if node != 0 && node_movement_classes[node as usize] == zone_class::OUTSIDE {
            node_movement_classes[node as usize] = movement_class;
        }
    }

    let Some(row_index) = movement_zone.matrix_row() else {
        return vec![0u16; node_count as usize + 1];
    };
    let row = passability::MOVEMENT_ZONE_PASSABILITY[row_index];
    let mut zone_id_by_node = vec![1u16; node_count as usize + 1];
    for node in 1..=node_count {
        let movement_class = node_movement_classes[node as usize] as usize;
        if row[movement_class] == 1 {
            zone_id_by_node[node as usize] = 0;
        }
    }

    let mut next_label: u16 = 2;
    for start_node in 1..=node_count {
        if zone_id_by_node[start_node as usize] != 0 {
            continue;
        }
        let mut queue = VecDeque::new();
        zone_id_by_node[start_node as usize] = next_label;
        queue.push_back(start_node);

        while let Some(cur) = queue.pop_front() {
            for &neighbor in &node_adj[cur as usize] {
                if zone_id_by_node[neighbor as usize] != 0 {
                    continue;
                }
                zone_id_by_node[neighbor as usize] = next_label;
                queue.push_back(neighbor);
            }
        }

        next_label += 1;
    }

    zone_id_by_node[0] = u16::MAX;
    zone_id_by_node
}

/// Check if a cell is passable for a given MovementZone.
///
/// This helper is still the direct passable-cell check used by the fallback and legacy
/// incremental paths. Terrain-aware full rebuilds now go through `MovementClass8` +
/// `nodeIndex` reconstruction instead.
pub(crate) fn is_passable(
    x: u16,
    y: u16,
    mz: MovementZone,
    path_grid: &PathGrid,
    cost_grid: Option<&TerrainCostGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    _layer: MovementLayer,
) -> bool {
    // Buildings and static obstacles block ground movement regardless of land
    // type. Fly uses matrix row 9 and should not inherit ground PathGrid blocks.
    // Water zones skip this check since water cells are typically blocked in PathGrid.
    if !mz.is_water_mover() && mz != MovementZone::Fly && !path_grid.is_walkable(x, y) {
        return false;
    }

    // Primary check: passability matrix using land_type from resolved terrain.
    // Uses MovementZone (not SpeedType) for the passability lookup — this matches
    // the original engine's Can_Enter_Cell logic where MovementZone determines
    // which cells are passable, while SpeedType only affects movement speed.
    // Critical: SpeedType::Float maps to zone 9 (hover — everything passable),
    // but MovementZone::Water maps to zone 10 (water cells only).
    if let Some(terrain) = resolved_terrain {
        if let Some(cell) = terrain.cell(x, y) {
            if mz.is_water_mover() {
                return super::is_water_surface_cell_passable(cell, mz);
            }
            return passability::is_passable_for_zone(cell.zone_type, mz);
        }
    }

    if mz == MovementZone::Fly {
        return true;
    }

    // Fallback: TerrainCostGrid-based check (pre-matrix behavior).
    if mz.is_water_mover() {
        if let Some(cg) = cost_grid {
            cg.cost_at(x, y) > 0
        } else {
            false
        }
    } else {
        if let Some(cg) = cost_grid {
            cg.cost_at(x, y) > 0
        } else {
            true
        }
    }
}

/// BFS flood-fill on the ground layer.
///
/// Assigns `zone_id` to all passable cells reachable from `(start_x, start_y)`.
/// Height continuity: adjacent cells with ground_level difference > 1 form zone
/// boundaries (matches original engine flood-fill behavior).
pub(crate) fn flood_fill(
    start_x: u16,
    start_y: u16,
    zone_id: ZoneId,
    zone_ids: &mut [ZoneId],
    width: u16,
    height: u16,
    mz: MovementZone,
    path_grid: &PathGrid,
    cost_grid: Option<&TerrainCostGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    layer: MovementLayer,
) {
    let mut queue = VecDeque::new();
    let start_idx = start_y as usize * width as usize + start_x as usize;
    zone_ids[start_idx] = zone_id;
    queue.push_back((start_x, start_y));

    while let Some((cx, cy)) = queue.pop_front() {
        for &(dx, dy, is_diagonal) in &NEIGHBORS {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let nx = nx as u16;
            let ny = ny as u16;
            let n_idx = ny as usize * width as usize + nx as usize;

            if zone_ids[n_idx] != ZONE_INVALID {
                continue;
            }
            if !is_passable(nx, ny, mz, path_grid, cost_grid, resolved_terrain, layer) {
                continue;
            }

            // Diagonal corner-cutting: both adjacent cardinals must be passable.
            if is_diagonal {
                let ax = (cx as i32 + dx) as u16;
                let ay = cy;
                let bx = cx;
                let by = (cy as i32 + dy) as u16;
                if !is_passable(ax, ay, mz, path_grid, cost_grid, resolved_terrain, layer)
                    || !is_passable(bx, by, mz, path_grid, cost_grid, resolved_terrain, layer)
                {
                    continue;
                }
            }

            // Height continuity: original engine enforces abs(h_diff) <= 1 in zone
            // flood-fill. Height jumps > 1 create zone boundaries so the zone system
            // never claims two cells are mutually reachable when A* would fail due to
            // a cliff. Only checked on the ground layer for land-based categories.
            if layer == MovementLayer::Ground {
                if let (Some(cur), Some(nbr)) = (path_grid.cell(cx, cy), path_grid.cell(nx, ny)) {
                    if (cur.ground_level as i16 - nbr.ground_level as i16).abs() > 1 {
                        continue;
                    }
                }
            }

            zone_ids[n_idx] = zone_id;
            queue.push_back((nx, ny));
        }
    }
}

/// Compute per-zone centroid and cell count from the ground zone ID array.
pub(crate) fn compute_zone_info(
    zone_ids: &[ZoneId],
    width: u16,
    _height: u16,
    zone_count: u16,
) -> Vec<ZoneInfo> {
    let mut sums: Vec<(u64, u64, u32)> = vec![(0, 0, 0); zone_count as usize];
    for (idx, &zid) in zone_ids.iter().enumerate() {
        if zid != ZONE_INVALID {
            let x = (idx % width as usize) as u64;
            let y = (idx / width as usize) as u64;
            let entry = &mut sums[zid as usize - 1];
            entry.0 += x;
            entry.1 += y;
            entry.2 += 1;
        }
    }
    sums.iter()
        .map(|&(sx, sy, count)| {
            if count == 0 {
                ZoneInfo::default()
            } else {
                ZoneInfo {
                    center: (
                        u16::try_from(sx / count as u64).unwrap_or(u16::MAX),
                        u16::try_from(sy / count as u64).unwrap_or(u16::MAX),
                    ),
                    cell_count: count,
                }
            }
        })
        .collect()
}

/// Extract adjacency from ground zone ID array (ground-layer only).
///
/// Bridge cross-zone edges are injected separately via `inject_bridge_adjacency`.
pub(crate) fn extract_adjacency(
    ground_zones: &[ZoneId],
    width: u16,
    height: u16,
    zone_count: u16,
) -> ZoneAdjacency {
    let mut adj_sets: Vec<Vec<ZoneId>> = vec![Vec::new(); zone_count as usize + 1];
    let w = width as usize;

    for ry in 0..height {
        for rx in 0..width {
            let idx = ry as usize * w + rx as usize;
            let z = ground_zones[idx];
            if z == ZONE_INVALID {
                continue;
            }
            for &(dx, dy) in &[(1i32, 0i32), (0, 1), (1, 1), (1, -1)] {
                let nx = rx as i32 + dx;
                let ny = ry as i32 + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let n_idx = ny as usize * w + nx as usize;
                let nz = ground_zones[n_idx];
                if nz != ZONE_INVALID && nz != z {
                    add_adjacency(&mut adj_sets, z, nz);
                }
            }
        }
    }

    ZoneAdjacency::new(adj_sets)
}

fn register_bridge_base_edges(
    edge_buckets: &mut BaseEdgeBuckets,
    ground_zones: &[ZoneId],
    bridge_records: &[BridgeEndpointRecord],
    width: u16,
) {
    let w = width as usize;
    for record in bridge_records {
        if !bridge_record_matches(record, BridgeRecordFilter::AllActive) {
            continue;
        }
        let (ax, ay) = record.endpoint_a;
        let (bx, by) = record.endpoint_b;
        let a_idx = ay as usize * w + ax as usize;
        let b_idx = by as usize * w + bx as usize;
        if a_idx >= ground_zones.len() || b_idx >= ground_zones.len() {
            continue;
        }
        edge_buckets.register(ground_zones[a_idx], ground_zones[b_idx]);
    }
}

/// Inject bridge adjacency edges into an existing adjacency graph.
///
/// For each active bridge endpoint record, connects the ground zones at
/// endpoint_a and endpoint_b. This mirrors gamemd.exe AddBridgeZoneEdges
/// (0x005851b0) which adds bidirectional edges to the zone graph.
pub(crate) fn inject_bridge_adjacency(
    adj: &mut ZoneAdjacency,
    ground_zones: &[ZoneId],
    bridge_records: &[BridgeEndpointRecord],
    width: u16,
    filter: BridgeRecordFilter,
) {
    let w = width as usize;
    for record in bridge_records {
        if !bridge_record_matches(record, filter) {
            continue;
        }
        let (ax, ay) = record.endpoint_a;
        let (bx, by) = record.endpoint_b;

        let a_idx = ay as usize * w + ax as usize;
        let b_idx = by as usize * w + bx as usize;

        if a_idx >= ground_zones.len() || b_idx >= ground_zones.len() {
            continue;
        }

        let za = ground_zones[a_idx];
        let zb = ground_zones[b_idx];

        if za != ZONE_INVALID && zb != ZONE_INVALID && za != zb {
            if !adj.neighbors[za as usize].contains(&zb) {
                adj.neighbors[za as usize].push(zb);
            }
            if !adj.neighbors[zb as usize].contains(&za) {
                adj.neighbors[zb as usize].push(za);
            }
        }
    }
}

/// Build per-cell bridge redirect table.
///
/// For each bridge cell (walkable on bridge layer), find the nearest active
/// bridge endpoint and store its ground cell coordinates.
pub(crate) fn build_bridge_redirect(
    path_grid: &PathGrid,
    bridge_records: &[BridgeEndpointRecord],
    width: u16,
    height: u16,
    filter: BridgeRecordFilter,
) -> Option<Vec<Option<(u16, u16)>>> {
    if bridge_records.is_empty() {
        return None;
    }

    let total = width as usize * height as usize;
    let mut redirect: Vec<Option<(u16, u16)>> = vec![None; total];
    let mut any = false;

    for ry in 0..height {
        for rx in 0..width {
            if !path_grid.is_walkable_on_layer(rx, ry, MovementLayer::Bridge) {
                continue;
            }

            let mut best_endpoint: Option<(u16, u16)> = None;
            let mut best_dist = u32::MAX;

            for record in bridge_records {
                if !bridge_record_matches(record, filter) {
                    continue;
                }
                let da = (rx as i32 - record.endpoint_a.0 as i32).unsigned_abs()
                    + (ry as i32 - record.endpoint_a.1 as i32).unsigned_abs();
                let db = (rx as i32 - record.endpoint_b.0 as i32).unsigned_abs()
                    + (ry as i32 - record.endpoint_b.1 as i32).unsigned_abs();
                let closer = if da <= db {
                    (record.endpoint_a, da)
                } else {
                    (record.endpoint_b, db)
                };
                if closer.1 < best_dist {
                    best_dist = closer.1;
                    best_endpoint = Some(closer.0);
                }
            }

            if let Some(ep) = best_endpoint {
                let idx = ry as usize * width as usize + rx as usize;
                redirect[idx] = Some(ep);
                any = true;
            }
        }
    }

    if any { Some(redirect) } else { None }
}

/// Add a bidirectional adjacency edge, preserving first discovery order.
pub(crate) fn add_adjacency(adj: &mut [Vec<ZoneId>], a: ZoneId, b: ZoneId) {
    if !adj[a as usize].contains(&b) {
        adj[a as usize].push(b);
    }
    if !adj[b as usize].contains(&a) {
        adj[b as usize].push(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bridge_state::{BridgeEndpointRecord, BridgeRecordKind};

    fn bridge_record(kind: BridgeRecordKind) -> BridgeEndpointRecord {
        BridgeEndpointRecord {
            endpoint_a: (0, 0),
            endpoint_b: (4, 0),
            group_id: 1,
            active: true,
            bridge_kind: kind,
        }
    }

    #[test]
    fn gsi_04_06_scanline_run_entry_overwrites_prior_vertical_assignment() {
        let mut grid = PathGrid::new(2, 2);
        grid.set_cell_for_test(0, 0, 0, false, false);
        grid.set_cell_for_test(1, 0, 0, false, false);
        grid.set_cell_for_test(0, 1, 2, false, false);
        grid.set_cell_for_test(1, 1, 0, false, false);

        let movement_classes = vec![0; 4];
        let (zone_ids, zone_count, edge_buckets) =
            rebuild_node_indices(&movement_classes, &grid, 2, 2);
        let base = BaseZoneTopology {
            adjacency: edge_buckets.into_adjacency(zone_count),
            movement_classes,
            zone_ids,
            zone_count,
        };

        assert_eq!(base.zone_ids, vec![1, 1, 2, 2]);
        assert_eq!(base.zone_count, 2);
        assert!(base.adjacency.are_adjacent(1, 2));
    }

    #[test]
    fn gsi_04_06_scanline_history_does_not_invent_final_cardinal_edge() {
        let mut grid = PathGrid::new(2, 2);
        for (index, level) in [2, 3, 1, 0].into_iter().enumerate() {
            grid.set_cell_for_test(
                (index % 2) as u16,
                (index / 2) as u16,
                level,
                false,
                false,
            );
        }

        let (zone_ids, zone_count, edge_buckets) =
            rebuild_node_indices(&[0; 4], &grid, 2, 2);
        let adjacency = edge_buckets.into_adjacency(zone_count);

        assert_eq!(zone_ids, vec![1, 1, 2, 2]);
        assert_eq!(zone_count, 2);
        assert!(!adjacency.are_adjacent(1, 2));
    }

    #[test]
    fn gsi_04_06_scanline_storage_fringe_has_one_base_zone() {
        let mut grid = PathGrid::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                grid.set_cell_for_test(x, y, 0, false, false);
            }
        }

        let (zone_ids, zone_count, _) = rebuild_node_indices(
            &[zone_class::GROUND, 7, 7, zone_class::GROUND],
            &grid,
            2,
            2,
        );

        assert_eq!(zone_ids, vec![1, 0, 0, 1]);
        assert_eq!(zone_count, 1);
    }

    #[test]
    fn gsi_04_06_scanline_rejects_right_delta_four() {
        let mut grid = PathGrid::new(2, 1);
        grid.set_cell_for_test(0, 0, 0, false, false);
        grid.set_cell_for_test(1, 0, 4, false, false);

        let (zone_ids, zone_count, _) = rebuild_node_indices(&[0; 2], &grid, 2, 1);

        assert_eq!(zone_ids, vec![1, 2]);
        assert_eq!(zone_count, 2);
    }

    #[test]
    fn bridge_redirect_ignores_low_bridge_records() {
        let mut grid = PathGrid::new(5, 1);
        for rx in 1..=3 {
            grid.set_cell_for_test(rx, 0, 0, true, false);
        }

        let records = [bridge_record(BridgeRecordKind::Low)];
        let redirect =
            build_bridge_redirect(&grid, &records, 5, 1, BridgeRecordFilter::HighActiveOnly);

        assert!(redirect.is_none());
    }

    #[test]
    fn bridge_adjacency_filter_all_active_includes_low_records() {
        let ground_zones = [1, ZONE_INVALID, ZONE_INVALID, ZONE_INVALID, 2];
        let records = [bridge_record(BridgeRecordKind::Low)];
        let mut adj = ZoneAdjacency::new(vec![vec![], vec![], vec![]]);

        inject_bridge_adjacency(
            &mut adj,
            &ground_zones,
            &records,
            5,
            BridgeRecordFilter::AllActive,
        );

        assert!(adj.are_adjacent(1, 2));
    }

    #[test]
    fn bridge_adjacency_filter_high_active_only_skips_low_records() {
        let ground_zones = [1, ZONE_INVALID, ZONE_INVALID, ZONE_INVALID, 2];
        let records = [bridge_record(BridgeRecordKind::Low)];
        let mut adj = ZoneAdjacency::new(vec![vec![], vec![], vec![]]);

        inject_bridge_adjacency(
            &mut adj,
            &ground_zones,
            &records,
            5,
            BridgeRecordFilter::HighActiveOnly,
        );

        assert!(!adj.are_adjacent(1, 2));
    }
}
