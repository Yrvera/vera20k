//! Tests for zone-aware pathfinding wrappers.

use super::super::zone_hierarchy::{ZoneEdgeRecord, ZoneHierarchy, ZoneLevelGraph, ZoneRecord};
use super::super::zone_map::{ZoneAdjacency, ZoneGrid, ZoneInfo, ZoneMap};
use super::*;
use crate::rules::locomotor_type::MovementZone;
use crate::sim::pathfinding::PathGrid;
use std::collections::{BTreeMap, BTreeSet};

fn grid_from_str(s: &str) -> PathGrid {
    let lines: Vec<&str> = s.trim().lines().map(|l| l.trim()).collect();
    let h = lines.len() as u16;
    let w = lines[0].len() as u16;
    let mut grid = PathGrid::new(w, h);
    for (ry, line) in lines.iter().enumerate() {
        for (rx, ch) in line.chars().enumerate() {
            if ch == '#' {
                grid.set_blocked(rx as u16, ry as u16, true);
            }
        }
    }
    grid
}

#[test]
fn zoned_path_reachable_returns_path() {
    let grid = grid_from_str(
        "
        .....
        .....
        .....
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 3);
    let path = find_path_zoned(
        &grid,
        (0, 0),
        (4, 2),
        None,
        None,
        Some(&zg),
        MovementZone::Normal,
        None,
        None,
        None,
        0,
        false,
    );
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(*path.first().unwrap(), (0, 0));
    assert_eq!(*path.last().unwrap(), (4, 2));
}

#[test]
fn zoned_path_unreachable_returns_none_instantly() {
    let grid = grid_from_str(
        "
        ..#..
        ..#..
        ..#..
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 3);
    // (0,0) and (4,0) are in different disconnected zones.
    let path = find_path_zoned(
        &grid,
        (0, 0),
        (4, 0),
        None,
        None,
        Some(&zg),
        MovementZone::Normal,
        None,
        None,
        None,
        0,
        false,
    );
    assert!(path.is_none());
}

#[test]
fn zoned_path_no_zone_grid_falls_through() {
    let grid = grid_from_str(
        "
        .....
        .....
    ",
    );
    // Without zone grid, should just run normal A*.
    let path = find_path_zoned(
        &grid,
        (0, 0),
        (4, 1),
        None,
        None,
        None,
        MovementZone::Normal,
        None,
        None,
        None,
        0,
        false,
    );
    assert!(path.is_some());
}

#[test]
fn zoned_path_same_cell() {
    let grid = grid_from_str(
        "
        .....
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 1);
    let path = find_path_zoned(
        &grid,
        (2, 0),
        (2, 0),
        None,
        None,
        Some(&zg),
        MovementZone::Normal,
        None,
        None,
        None,
        0,
        false,
    );
    assert!(path.is_some());
    assert_eq!(path.unwrap(), vec![(2, 0)]);
}

#[test]
fn zoned_path_entity_blocks_respected() {
    let grid = grid_from_str(
        "
        ...
        ...
        ...
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 3, 3);
    // Block the direct path with entities.
    let mut blocks = BTreeSet::new();
    blocks.insert((1, 0));
    blocks.insert((1, 1));
    blocks.insert((1, 2));
    // Zone says reachable (static terrain is connected), but entities block.
    // A* should still find no path since the wall of entities cuts off (2,x).
    let path = find_path_zoned(
        &grid,
        (0, 0),
        (2, 0),
        None,
        Some(&blocks),
        Some(&zg),
        MovementZone::Normal,
        None,
        None,
        None,
        0,
        false,
    );
    // Path exists because goal cell is always reachable even if entity-blocked.
    // But the path would need to go around — with a 3x3 grid fully blocked
    // in column 1, there's no way around.
    assert!(path.is_none());
}

fn test_zone_map() -> (ZoneMap, ZoneAdjacency) {
    let zone_map = ZoneMap::new(
        vec![1, 2, 3, 4],
        None,
        4,
        1,
        4,
        vec![
            ZoneInfo {
                center: (0, 0),
                cell_count: 1,
            },
            ZoneInfo {
                center: (1, 0),
                cell_count: 1,
            },
            ZoneInfo {
                center: (0, 1),
                cell_count: 1,
            },
            ZoneInfo {
                center: (2, 0),
                cell_count: 1,
            },
        ],
    );
    let adjacency =
        ZoneAdjacency::new(vec![vec![], vec![2, 3], vec![1, 3, 4], vec![1, 2], vec![2]]);
    (zone_map, adjacency)
}

fn equal_cost_zone_map(adjacency_order: Vec<ZoneId>) -> (ZoneMap, ZoneAdjacency) {
    let zone_map = ZoneMap::new(
        vec![1, 2, 3, 4, 5],
        None,
        5,
        1,
        5,
        vec![
            ZoneInfo {
                center: (0, 0),
                cell_count: 1,
            },
            ZoneInfo {
                center: (1, 0),
                cell_count: 1,
            },
            ZoneInfo {
                center: (1, 0),
                cell_count: 1,
            },
            ZoneInfo {
                center: (0, 1),
                cell_count: 1,
            },
            ZoneInfo {
                center: (2, 0),
                cell_count: 1,
            },
        ],
    );
    let adjacency = ZoneAdjacency::new(vec![
        vec![],
        adjacency_order,
        vec![1, 5],
        vec![1, 5],
        vec![],
        vec![2, 3],
    ]);
    (zone_map, adjacency)
}

fn linear_level0_hierarchy(zones: Vec<ZoneId>, edges: &[(ZoneId, ZoneId)]) -> ZoneHierarchy {
    let zone_count = zones.iter().copied().max().unwrap_or(0);
    let width = zones.len() as u16;
    let mut level2 = ZoneLevelGraph::new(1);
    level2.set_record(ZoneRecord::new(1, 0, 0));

    let mut level1 = ZoneLevelGraph::new(1);
    level1.set_record(ZoneRecord::new(1, 1, 0));

    let mut level0 = ZoneLevelGraph::new(zone_count).with_cell_zone_ids(zones, width, 1);
    for zone in 1..=zone_count {
        level0.set_record(ZoneRecord::new(zone, 1, 0));
    }
    for &(a, b) in edges {
        level0.push_edge(a, ZoneEdgeRecord::new(b, 0));
        level0.push_edge(b, ZoneEdgeRecord::new(a, 0));
    }

    ZoneHierarchy::new(level0, level1, level2)
}

#[test]
fn zone_precheck_hierarchy_path_bypasses_reduced_superzone_abort() {
    let astar_grid = PathGrid::new(3, 1);
    let mut reduced_grid = PathGrid::new(3, 1);
    reduced_grid.set_blocked(1, 0, true);
    let mut zg = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 3, 1);
    zg.set_hierarchy(
        MovementZone::Normal,
        linear_level0_hierarchy(vec![1, 2, 3], &[(1, 2), (2, 3)]),
    );
    assert!(
        !zg.can_reach(
            MovementZone::Normal,
            (0, 0),
            MovementLayer::Ground,
            (2, 0),
            MovementLayer::Ground
        ),
        "fixture must prove the old reduced SuperZoneMap would abort"
    );

    let blocker_counts = BlockerNeighborCounts::new(3, 1);
    let path = find_path_zoned_marker_inner(
        &astar_grid,
        (0, 0),
        (2, 0),
        None,
        None,
        Some(&zg),
        MovementZone::Normal,
        Some(MovementZone::Normal),
        None,
        None,
        None,
        0,
        false,
        Some(&blocker_counts),
    )
    .expect("eligible hierarchy precheck should not be preempted by reduced reachability");

    assert_eq!(path, vec![(0, 0), (1, 0), (2, 0)]);
}

#[test]
fn zone_precheck_failed_hierarchy_keeps_zone_map_same_zone_fallback() {
    let astar_grid = PathGrid::new(3, 1);
    let mut zg = ZoneGrid::build(&astar_grid, &BTreeMap::new(), 3, 1);
    zg.set_hierarchy(
        MovementZone::Normal,
        linear_level0_hierarchy(vec![ZONE_INVALID, ZONE_INVALID, ZONE_INVALID], &[]),
    );

    let blocker_counts = BlockerNeighborCounts::new(3, 1);
    let path = find_path_zoned_marker_inner(
        &astar_grid,
        (0, 0),
        (2, 0),
        None,
        None,
        Some(&zg),
        MovementZone::Normal,
        Some(MovementZone::Normal),
        None,
        None,
        None,
        0,
        false,
        Some(&blocker_counts),
    )
    .expect("same-zone ZoneMap fallback should survive incomplete hierarchy cell IDs");

    assert_eq!(path, vec![(0, 0), (1, 0), (2, 0)]);
}

#[test]
fn zone_corridor_equal_cost_ties_keep_adjacency_order() {
    let (zone_map, adjacency) = equal_cost_zone_map(vec![3, 2]);
    let excluded_edges = BTreeSet::new();

    let corridor = find_zone_corridor(&zone_map, &adjacency, 1, 5, &excluded_edges)
        .expect("equal-cost corridor should exist");

    assert_eq!(
        corridor,
        vec![1, 3, 5],
        "equal-cost zone ties must keep adjacency discovery order, not lower ZoneId"
    );
}

#[test]
fn zone_corridor_equal_cost_ties_follow_reversed_adjacency_order() {
    let (zone_map, adjacency) = equal_cost_zone_map(vec![2, 3]);
    let excluded_edges = BTreeSet::new();

    let corridor = find_zone_corridor(&zone_map, &adjacency, 1, 5, &excluded_edges)
        .expect("equal-cost corridor should exist");

    assert_eq!(corridor, vec![1, 2, 5]);
}

#[test]
fn zone_corridor_retry_excludes_edges_not_zones() {
    let (zone_map, adjacency) = test_zone_map();
    let mut excluded_edges = BTreeSet::new();

    let first =
        find_zone_corridor(&zone_map, &adjacency, 1, 4, &excluded_edges).expect("initial corridor");
    assert_eq!(first, vec![1, 2, 4]);

    excluded_edges.insert(ZoneEdge::new(1, 2).unwrap());
    let second = find_zone_corridor(&zone_map, &adjacency, 1, 4, &excluded_edges)
        .expect("alternate corridor should reuse zone 2 through another edge");
    assert_eq!(second, vec![1, 3, 2, 4]);
}

#[test]
fn zone_edge_exclusions_are_undirected() {
    let zone_map = ZoneMap::new(
        vec![1, 2],
        None,
        2,
        1,
        2,
        vec![
            ZoneInfo {
                center: (0, 0),
                cell_count: 1,
            },
            ZoneInfo {
                center: (1, 0),
                cell_count: 1,
            },
        ],
    );
    let adjacency = ZoneAdjacency::new(vec![vec![], vec![2], vec![1]]);
    let mut excluded_edges = BTreeSet::new();
    excluded_edges.insert(ZoneEdge::new(1, 2).unwrap());

    assert!(find_zone_corridor(&zone_map, &adjacency, 2, 1, &excluded_edges).is_none());
}

#[test]
fn zone_cost_estimate_matches_precheck_and_alternate_margin() {
    let grid = grid_from_str(
        "
        .....
        .....
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 2);

    let estimate = zone_cost_estimate(
        &zg,
        MovementZone::Normal,
        (0, 0),
        crate::sim::movement::locomotor::MovementLayer::Ground,
        (4, 1),
        crate::sim::movement::locomotor::MovementLayer::Ground,
    );
    assert_eq!(estimate, 4);
    assert!(accepts_blocked_destination_alternate(
        estimate,
        (4, 1),
        (0, 1)
    ));
    assert!(!accepts_blocked_destination_alternate(
        i32::MAX,
        (4, 1),
        (0, 1)
    ));

    let blocked_grid = grid_from_str(
        "
        ..#..
        ..#..
    ",
    );
    let blocked_zg = ZoneGrid::build(&blocked_grid, &BTreeMap::new(), 5, 2);
    assert_eq!(
        zone_cost_estimate(
            &blocked_zg,
            MovementZone::Normal,
            (0, 0),
            crate::sim::movement::locomotor::MovementLayer::Ground,
            (4, 0),
            crate::sim::movement::locomotor::MovementLayer::Ground,
        ),
        i32::MAX
    );
}
