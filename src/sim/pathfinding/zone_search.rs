//! Zone-aware pathfinding — uses zone connectivity for fast unreachability
//! detection and hierarchical corridor-based search space reduction.
//!
//! Current approximation:
//! 1. Look up zone IDs for start and goal.
//! 2. If they are in disconnected zones, return `None` immediately (no A*).
//! 3. Run Dijkstra on the zone adjacency graph to find a coarse corridor.
//! 4. Run cell-level A* restricted to the corridor zones.
//! 5. On failure, retry with per-edge exclusions (up to 5 total attempts).
//!
//! TODO(RE): RA2/YR has distinct regular vs hierarchical entrypoints and a separate
//! allowHS gate. The recovered entrypoint behavior is precise enough to prove those
//! modes exist, but not yet enough to replace this corridor-Dijkstra approximation.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/zone_map, sim/pathfinding, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};

use super::{BlockerNeighborCounts, LayeredEntityBlockMap, SearchMarkerOverlay};

use super::terrain_cost::TerrainCostGrid;
use super::zone_hierarchy::{ZonePrecheckExclusions, ZonePrecheckOutcome, zone_precheck_flat};
use super::zone_map::{ZONE_INVALID, ZoneAdjacency, ZoneGrid, ZoneId, ZoneMap};
use super::{
    LayeredPathStep, PathGrid, find_layered_path_marker, find_path_with_costs_corridor_marker,
    find_path_with_costs_hierarchy_marker_progress, find_path_with_costs_marker,
};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tube_facts::TubeSource;
use crate::rules::locomotor_type::MovementZone;
use crate::sim::movement::locomotor::MovementLayer;

/// Maximum corridor Dijkstra attempts with zone-edge exclusions.
/// The recovered path entry contract uses a default total attempt cap of 5.
const MAX_CORRIDOR_RETRIES: u8 = 5;

#[allow(dead_code)]
const BLOCKED_DESTINATION_ALTERNATE_MARGIN: i32 = 6;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct ZoneEdge {
    a: ZoneId,
    b: ZoneId,
}

impl ZoneEdge {
    pub(super) fn new(a: ZoneId, b: ZoneId) -> Option<Self> {
        if a == ZONE_INVALID || b == ZONE_INVALID || a == b {
            return None;
        }
        Some(if a < b {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        })
    }
}

fn can_use_reduced_zone_precheck(movement_zone: Option<MovementZone>) -> bool {
    match movement_zone {
        None => true,
        Some(
            MovementZone::Normal
            | MovementZone::Amphibious
            | MovementZone::Infantry
            | MovementZone::Fly,
        ) => true,
        // TODO(RE): naval water/beach surface legality in the current terrain-aware zone
        // builder is still coarser than the runtime water-surface predicate, so do not
        // hard-gate those movers on reduced-zone reachability yet.
        Some(_) => false,
    }
}

fn can_reach_same_or_zoned(
    zg: &ZoneGrid,
    mz: MovementZone,
    from: (u16, u16),
    from_layer: MovementLayer,
    to: (u16, u16),
    to_layer: MovementLayer,
) -> bool {
    from == to || zg.can_reach(mz, from, from_layer, to, to_layer)
}

fn can_reach_through_explicit_tube(
    zg: &ZoneGrid,
    mz: MovementZone,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    resolved_terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    let Some(terrain) = resolved_terrain else {
        return false;
    };
    terrain.tube_facts().iter().any(|tube| {
        tube.source == TubeSource::ExplicitMap
            && tube.path_len() > 0
            && tube.exit != (0, 0)
            && can_reach_same_or_zoned(
                zg,
                mz,
                start,
                start_layer,
                tube.entry,
                MovementLayer::Ground,
            )
            && can_reach_same_or_zoned(
                zg,
                mz,
                tube.exit,
                MovementLayer::Ground,
                goal,
                MovementLayer::Ground,
            )
    })
}

fn has_explicit_tube_scenario(resolved_terrain: Option<&ResolvedTerrainGrid>) -> bool {
    let Some(terrain) = resolved_terrain else {
        return false;
    };
    terrain
        .tube_facts()
        .iter()
        .any(|tube| tube.source == TubeSource::ExplicitMap && tube.path_len() > 0)
}

/// Zone-aware path search for flat (ground-only) paths.
///
/// Uses zone reachability plus a corridor-Dijkstra approximation, then runs A*
/// restricted to that corridor. If the bounded hierarchical attempts are
/// exhausted, the search fails rather than running an unrestricted fallback.
///
/// TODO(RE): terrain-aware nodeIndex connectivity can still be a little looser than
/// final movement legality because the recovered node flood-fill is 8-neighbor while
/// the actual step predicate also applies tighter per-move checks. Treat zone gating
/// here as a best-effort reject, not closed parity.
pub fn find_path_zoned(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    urgency: u8,
    mover_is_crusher: bool,
) -> Option<Vec<(u16, u16)>> {
    find_path_zoned_marker(
        grid,
        start,
        goal,
        costs,
        entity_blocks,
        zone_grid,
        mz,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        None,
        None,
        urgency,
        mover_is_crusher,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn find_path_zoned_marker(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
    urgency: u8,
    mover_is_crusher: bool,
) -> Option<Vec<(u16, u16)>> {
    find_path_zoned_marker_inner(
        grid,
        start,
        goal,
        costs,
        entity_blocks,
        zone_grid,
        mz,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        urgency,
        mover_is_crusher,
        blocker_neighbor_counts,
    )
}

#[allow(clippy::too_many_arguments)]
fn find_path_zoned_marker_inner(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    urgency: u8,
    mover_is_crusher: bool,
    blocker_neighbor_counts: Option<&BlockerNeighborCounts>,
) -> Option<Vec<(u16, u16)>> {
    if !can_use_reduced_zone_precheck(movement_zone) {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
        );
    }

    let Some(zg) = zone_grid else {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
        );
    };

    let Some(zone_map) = zg.map_for(mz) else {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
        );
    };
    let start_zone = zone_map.zone_at(start.0, start.1, MovementLayer::Ground);
    let goal_zone = zone_map.zone_at(goal.0, goal.1, MovementLayer::Ground);
    let zones_match = start_zone == goal_zone;

    let hierarchy_counts_available = blocker_neighbor_counts.is_some();
    let explicit_tube_deferred = has_explicit_tube_scenario(resolved_terrain);
    if hierarchy_counts_available
        && !explicit_tube_deferred
        && let Some(hierarchy) = zg.hierarchy_for(mz)
        && let Some(level0_zones) = hierarchy.level(0)
    {
        let hierarchy_start_zone = level0_zones.zone_at(start.0, start.1);
        let hierarchy_goal_zone = level0_zones.zone_at(goal.0, goal.1);
        match zone_precheck_flat(
            hierarchy,
            hierarchy_start_zone,
            hierarchy_goal_zone,
            movement_zone.unwrap_or(mz),
            &ZonePrecheckExclusions::default(),
        ) {
            ZonePrecheckOutcome::Passed(result) => {
                return find_path_with_costs_hierarchy_marker_progress(
                    grid,
                    start,
                    goal,
                    costs,
                    entity_blocks,
                    level0_zones,
                    &result.marked[0],
                    blocker_neighbor_counts.expect("checked above"),
                    &result.paths[0],
                    movement_zone,
                    resolved_terrain,
                    entity_block_map,
                    marker_overlay,
                    urgency,
                    mover_is_crusher,
                )
                .map(|result| result.path);
            }
            ZonePrecheckOutcome::Failed if zones_match => {
                return find_path_with_costs_marker(
                    grid,
                    start,
                    goal,
                    costs,
                    entity_blocks,
                    movement_zone,
                    resolved_terrain,
                    entity_block_map,
                    marker_overlay,
                    urgency,
                    mover_is_crusher,
                );
            }
            ZonePrecheckOutcome::Failed => return None,
        }
    }

    let zone_precheck_passed = zg.can_reach(
        mz,
        start,
        MovementLayer::Ground,
        goal,
        MovementLayer::Ground,
    );

    // Same-zone precheck failures disable hierarchy and still run cell A*.
    if !zone_precheck_passed && zones_match {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
        );
    }

    // Cross-zone precheck failure aborts without cell A*.
    if !zone_precheck_passed {
        if can_reach_through_explicit_tube(
            zg,
            mz,
            start,
            MovementLayer::Ground,
            goal,
            resolved_terrain,
        ) {
            return find_path_with_costs_marker(
                grid,
                start,
                goal,
                costs,
                entity_blocks,
                movement_zone,
                resolved_terrain,
                entity_block_map,
                marker_overlay,
                urgency,
                mover_is_crusher,
            );
        }
        log::trace!(
            "zone_search: unreachable {:?} ({:?}→{:?}), skipping A*",
            mz,
            start,
            goal,
        );
        return None;
    }

    let Some(adjacency) = zg.adjacency_for(mz) else {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
        );
    };

    let start_zone = zone_map.zone_at(start.0, start.1, MovementLayer::Ground);
    let goal_zone = zone_map.zone_at(goal.0, goal.1, MovementLayer::Ground);

    // Same zone — no corridor needed, run A* directly.
    if start_zone == goal_zone && start_zone != ZONE_INVALID {
        return find_path_with_costs_marker(
            grid,
            start,
            goal,
            costs,
            entity_blocks,
            movement_zone,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
        );
    }

    // Try corridor-restricted A* with retry on failure.
    let mut excluded_edges: BTreeSet<ZoneEdge> = BTreeSet::new();
    for attempt in 0..MAX_CORRIDOR_RETRIES {
        if let Some(corridor_zones) =
            find_zone_corridor(zone_map, adjacency, start_zone, goal_zone, &excluded_edges)
        {
            // Expand corridor by one ring of neighbor zones for flexibility.
            let allowed = expand_corridor(&corridor_zones, adjacency);
            if let Some(path) = find_path_with_costs_corridor_marker(
                grid,
                start,
                goal,
                costs,
                entity_blocks,
                zone_map,
                &allowed,
                movement_zone,
                resolved_terrain,
                entity_block_map,
                marker_overlay,
                urgency,
                mover_is_crusher,
            ) {
                return Some(path);
            }
            // Corridor A* failed — exclude all corridor zones and retry.
            log::trace!(
                "zone_search: corridor A* failed attempt {} ({} zones), retrying with exclusions",
                attempt + 1,
                corridor_zones.len(),
            );
            if !exclude_corridor_edges(&corridor_zones, &mut excluded_edges) {
                break;
            }
        } else {
            break; // Dijkstra couldn't find alternative route
        }
    }

    None
}

/// Zone-aware path search for layered (bridge-capable) paths.
///
/// Checks zone connectivity before invoking the layered A* pathfinder.
/// Bridge cells redirect to ground endpoint zones via `zone_at(Bridge)`,
/// so a single ground-layer reachability check covers cross-bridge paths.
pub fn find_layered_path_zoned(
    grid: &PathGrid,
    ground_blocks: Option<&BTreeSet<(u16, u16)>>,
    bridge_blocks: Option<&BTreeSet<(u16, u16)>>,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    terrain_costs: Option<&TerrainCostGrid>,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    urgency: u8,
    mover_is_crusher: bool,
) -> Option<Vec<LayeredPathStep>> {
    find_layered_path_zoned_marker(
        grid,
        ground_blocks,
        bridge_blocks,
        start,
        start_layer,
        goal,
        zone_grid,
        mz,
        terrain_costs,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        None,
        urgency,
        mover_is_crusher,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn find_layered_path_zoned_marker(
    grid: &PathGrid,
    ground_blocks: Option<&BTreeSet<(u16, u16)>>,
    bridge_blocks: Option<&BTreeSet<(u16, u16)>>,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    zone_grid: Option<&ZoneGrid>,
    mz: MovementZone,
    terrain_costs: Option<&TerrainCostGrid>,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    marker_overlay: Option<&SearchMarkerOverlay>,
    urgency: u8,
    mover_is_crusher: bool,
) -> Option<Vec<LayeredPathStep>> {
    // Foundation First deliberately leaves layered bridge routing on the
    // compatibility path. Flat hierarchy precheck/marker gating here does not
    // prove high-bridge route parity or retry producer parity.
    if !can_use_reduced_zone_precheck(movement_zone) {
        return find_layered_path_marker(
            grid,
            ground_blocks,
            bridge_blocks,
            start,
            start_layer,
            goal,
            terrain_costs,
            resolved_terrain,
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
        );
    }

    // Zone pre-check: bridge cells redirect to ground endpoint zones,
    // so a single ground-layer check covers cross-bridge reachability.
    if let Some(zg) = zone_grid {
        if !zg.can_reach(mz, start, start_layer, goal, MovementLayer::Ground) {
            if can_reach_through_explicit_tube(zg, mz, start, start_layer, goal, resolved_terrain) {
                return find_layered_path_marker(
                    grid,
                    ground_blocks,
                    bridge_blocks,
                    start,
                    start_layer,
                    goal,
                    terrain_costs,
                    resolved_terrain,
                    entity_block_map,
                    marker_overlay,
                    urgency,
                    mover_is_crusher,
                );
            }
            log::trace!(
                "zone_search: layered unreachable {:?} ({:?} layer={:?} -> {:?}), skipping A*",
                mz,
                start,
                start_layer,
                goal,
            );
            return None;
        }
    }

    find_layered_path_marker(
        grid,
        ground_blocks,
        bridge_blocks,
        start,
        start_layer,
        goal,
        terrain_costs,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        urgency,
        mover_is_crusher,
    )
}

// ---------------------------------------------------------------------------
// Hierarchical zone Dijkstra
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ZoneQueueEntry {
    cost: i32,
    sequence: u32,
    zone: ZoneId,
}

impl Ord for ZoneQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for ZoneQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Find the cheapest coarse route through the zone adjacency graph.
/// Returns an ordered sequence of zone IDs from start to goal.
///
/// Edge cost still uses Rust's centroid Manhattan approximation, but equal-cost
/// ties follow gamemd.exe `Zone_precheck`: adjacency discovery order wins and
/// `ZoneId` is not a tie key.
pub(super) fn find_zone_corridor(
    zone_map: &ZoneMap,
    adjacency: &ZoneAdjacency,
    start_zone: ZoneId,
    goal_zone: ZoneId,
    excluded_edges: &BTreeSet<ZoneEdge>,
) -> Option<Vec<ZoneId>> {
    if start_zone == ZONE_INVALID || goal_zone == ZONE_INVALID {
        return None;
    }
    if start_zone == goal_zone {
        return Some(vec![start_zone]);
    }

    // Dijkstra on the zone graph with stable insertion-order ties.
    let zone_count = zone_map.zone_count as usize;
    let mut dist: Vec<i32> = vec![i32::MAX; zone_count + 1]; // 1-indexed
    let mut prev: Vec<ZoneId> = vec![ZONE_INVALID; zone_count + 1];
    let mut heap: BinaryHeap<ZoneQueueEntry> = BinaryHeap::new();
    let mut next_sequence: u32 = 1;

    dist[start_zone as usize] = 0;
    heap.push(ZoneQueueEntry {
        cost: 0,
        sequence: 0,
        zone: start_zone,
    });

    while let Some(ZoneQueueEntry { cost, zone, .. }) = heap.pop() {
        if zone == goal_zone {
            // Reconstruct path.
            let mut path = Vec::new();
            let mut cur = goal_zone;
            while cur != ZONE_INVALID {
                path.push(cur);
                cur = prev[cur as usize];
            }
            path.reverse();
            return Some(path);
        }
        if cost > dist[zone as usize] {
            continue; // stale entry
        }
        for &neighbor in adjacency.neighbors_of(zone) {
            if ZoneEdge::new(zone, neighbor).is_some_and(|edge| excluded_edges.contains(&edge)) {
                continue;
            }
            let Some(n_info) = zone_map.info_for(neighbor) else {
                continue;
            };
            // Edge cost: Manhattan distance between zone centers.
            let edge_cost = manhattan(
                zone_map.info_for(zone).map(|i| i.center).unwrap_or((0, 0)),
                n_info.center,
            );
            let new_cost = cost + edge_cost;
            if new_cost < dist[neighbor as usize] {
                dist[neighbor as usize] = new_cost;
                prev[neighbor as usize] = zone;
                heap.push(ZoneQueueEntry {
                    cost: new_cost,
                    sequence: next_sequence,
                    zone: neighbor,
                });
                next_sequence = next_sequence.wrapping_add(1);
            }
        }
    }

    None // No route through zone graph
}

pub(super) fn exclude_corridor_edges(
    corridor: &[ZoneId],
    excluded_edges: &mut BTreeSet<ZoneEdge>,
) -> bool {
    let mut inserted_any = false;
    for pair in corridor.windows(2) {
        if let Some(edge) = ZoneEdge::new(pair[0], pair[1]) {
            inserted_any |= excluded_edges.insert(edge);
        }
    }
    inserted_any
}

/// Manhattan distance between two cell coordinates.
fn manhattan(a: (u16, u16), b: (u16, u16)) -> i32 {
    (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs()
}

#[allow(dead_code)]
fn chebyshev(a: (u16, u16), b: (u16, u16)) -> i32 {
    (a.0 as i32 - b.0 as i32)
        .abs()
        .max((a.1 as i32 - b.1 as i32).abs())
}

#[allow(dead_code)]
pub(crate) fn zone_cost_estimate(
    zg: &ZoneGrid,
    mz: MovementZone,
    start: (u16, u16),
    start_layer: MovementLayer,
    goal: (u16, u16),
    goal_layer: MovementLayer,
) -> i32 {
    if !zg.can_reach(mz, start, start_layer, goal, goal_layer) {
        return i32::MAX;
    }

    let Some(zone_map) = zg.map_for(mz) else {
        return chebyshev(start, goal);
    };
    let start_zone = zone_map.zone_at(start.0, start.1, start_layer);
    let goal_zone = zone_map.zone_at(goal.0, goal.1, goal_layer);
    if start_zone == ZONE_INVALID || goal_zone == ZONE_INVALID {
        return i32::MAX;
    }
    if start_zone == goal_zone {
        return chebyshev(start, goal);
    }

    let Some(adjacency) = zg.adjacency_for(mz) else {
        return chebyshev(start, goal);
    };
    let empty_exclusions = BTreeSet::new();
    let Some(corridor) = find_zone_corridor(
        zone_map,
        adjacency,
        start_zone,
        goal_zone,
        &empty_exclusions,
    ) else {
        return i32::MAX;
    };

    let Some(start_center) = zone_map.info_for(start_zone).map(|info| info.center) else {
        return i32::MAX;
    };
    let Some(goal_center) = zone_map.info_for(goal_zone).map(|info| info.center) else {
        return i32::MAX;
    };

    let mut estimate = chebyshev(start, start_center);
    for pair in corridor.windows(2) {
        let Some(from) = zone_map.info_for(pair[0]).map(|info| info.center) else {
            return i32::MAX;
        };
        let Some(to) = zone_map.info_for(pair[1]).map(|info| info.center) else {
            return i32::MAX;
        };
        estimate = estimate.saturating_add(chebyshev(from, to));
    }
    estimate.saturating_add(chebyshev(goal_center, goal))
}

#[allow(dead_code)]
pub(crate) fn accepts_blocked_destination_alternate(
    helper_result: i32,
    original: (u16, u16),
    alternate: (u16, u16),
) -> bool {
    helper_result != i32::MAX
        && helper_result <= chebyshev(original, alternate) + BLOCKED_DESTINATION_ALTERNATE_MARGIN
}

/// Expand a corridor by adding all 1-hop neighbor zones.
/// This gives A* flexibility to route through cells near corridor boundaries.
fn expand_corridor(corridor: &[ZoneId], adjacency: &ZoneAdjacency) -> BTreeSet<ZoneId> {
    let mut allowed: BTreeSet<ZoneId> = corridor.iter().copied().collect();
    for &zone in corridor {
        for &neighbor in adjacency.neighbors_of(zone) {
            allowed.insert(neighbor);
        }
    }
    allowed
}

#[cfg(test)]
#[path = "zone_search_tests.rs"]
mod zone_search_tests;
