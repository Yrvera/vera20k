//! Map-edge passable cell finder.
//!
//! Picks a ground-walkable cell along the requested map edge (N/E/S/W),
//! biased toward a target cell. Used by the paradrop SW launch handler
//! to choose the carrier aircraft's spawn edge cell and the opposite-edge
//! exit cell.
//!
//! Modes 0/1/3 (N/E/W) use a simple linear scan + closest-to-target tiebreak.
//! Mode 2 (South) collects a candidate list of up to 10 valid cells and picks
//! closest-to-target — preserving gamemd FUN_004AA440's mode-2 asymmetry.
//! (gamemd's RNG branch only fires when the alternate cell is sentinel; for
//! paradrop we always pass target as the alternate, so we always hit the
//! deterministic closest-to-target path.)
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on sim/pathfinding.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::pathfinding::PathGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    North,
    East,
    South,
    West,
}

impl Edge {
    pub fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Edge::North),
            1 => Some(Edge::East),
            2 => Some(Edge::South),
            3 => Some(Edge::West),
            _ => None,
        }
    }
}

/// Find a passable cell along the given map edge, biased toward `target`.
/// Returns `None` if no passable cell exists along that edge.
pub fn find_passable_at_edge(
    path_grid: &PathGrid,
    map_width: u16,
    map_height: u16,
    edge: Edge,
    target: (u16, u16),
) -> Option<(u16, u16)> {
    match edge {
        Edge::North | Edge::East | Edge::West => {
            scan_linear(path_grid, edge, map_width, map_height, target)
        }
        Edge::South => scan_candidates_closest(path_grid, map_width, map_height, target),
    }
}

fn scan_linear(
    path_grid: &PathGrid,
    edge: Edge,
    map_width: u16,
    map_height: u16,
    target: (u16, u16),
) -> Option<(u16, u16)> {
    let cells: Vec<(u16, u16)> = match edge {
        Edge::North => (0..map_width).map(|x| (x, 0)).collect(),
        Edge::East => (0..map_height)
            .map(|y| (map_width.saturating_sub(1), y))
            .collect(),
        Edge::West => (0..map_height).map(|y| (0, y)).collect(),
        Edge::South => unreachable!("south uses scan_candidates_closest"),
    };

    cells
        .into_iter()
        .filter(|&(rx, ry)| path_grid.is_walkable(rx, ry))
        .min_by_key(|&(rx, ry)| {
            let dx = rx as i32 - target.0 as i32;
            let dy = ry as i32 - target.1 as i32;
            dx * dx + dy * dy
        })
}

fn scan_candidates_closest(
    path_grid: &PathGrid,
    map_width: u16,
    map_height: u16,
    target: (u16, u16),
) -> Option<(u16, u16)> {
    let south_y = map_height.saturating_sub(1);
    let mut candidates: Vec<(u16, u16)> = Vec::with_capacity(10);
    for x in 0..map_width {
        if candidates.len() >= 10 {
            break;
        }
        if path_grid.is_walkable(x, south_y) {
            candidates.push((x, south_y));
        }
    }
    candidates.into_iter().min_by_key(|&(rx, ry)| {
        let dx = rx as i32 - target.0 as i32;
        let dy = ry as i32 - target.1 as i32;
        dx * dx + dy * dy
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_north_edge_picks_closest_to_target_x() {
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::North, (42, 50)).unwrap();
        assert_eq!(cell.1, 0);
        assert_eq!(cell.0, 42);
    }

    #[test]
    fn test_west_edge_picks_closest_to_target_y() {
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::West, (50, 70)).unwrap();
        assert_eq!(cell.0, 0);
        assert_eq!(cell.1, 70);
    }

    #[test]
    fn test_east_edge_picks_closest_to_target_y() {
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::East, (50, 30)).unwrap();
        assert_eq!(cell.0, 99);
        assert_eq!(cell.1, 30);
    }

    #[test]
    fn test_south_edge_picks_closest_to_target_x_within_first_10() {
        // Mode 2 only collects the first 10 walkable cells (x=0..10),
        // then picks closest to target.x. Target x=5 → cell x=5.
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::South, (5, 50)).unwrap();
        assert_eq!(cell, (5, 99));
    }

    #[test]
    fn test_south_edge_target_outside_candidate_window_picks_nearest_candidate() {
        // Target x=80 — outside the 0..10 candidate window. Closest candidate is x=9.
        let grid = PathGrid::test_all_passable(100, 100);
        let cell = find_passable_at_edge(&grid, 100, 100, Edge::South, (80, 50)).unwrap();
        assert_eq!(cell, (9, 99));
    }

    #[test]
    fn test_no_passable_returns_none() {
        let grid = PathGrid::test_all_blocked(100, 100);
        assert_eq!(
            find_passable_at_edge(&grid, 100, 100, Edge::North, (50, 50)),
            None
        );
        assert_eq!(
            find_passable_at_edge(&grid, 100, 100, Edge::South, (50, 50)),
            None
        );
    }

    #[test]
    fn test_edge_from_index() {
        assert_eq!(Edge::from_index(0), Some(Edge::North));
        assert_eq!(Edge::from_index(1), Some(Edge::East));
        assert_eq!(Edge::from_index(2), Some(Edge::South));
        assert_eq!(Edge::from_index(3), Some(Edge::West));
        assert_eq!(Edge::from_index(4), None);
    }
}
