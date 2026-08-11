//! Per-SpeedType terrain cost grid for variable-cost A* pathfinding.
//!
//! Each cell has a speed modifier (0 = blocked, 100 = normal, <100 = slow terrain).
//! The A* pathfinder multiplies its step cost by `100 / cost_at(x,y)` so that
//! slower terrain costs more and the planner routes around it.
//!
//! ## Design
//! `TerrainCostGrid` is built from map data + a `SpeedType` and provides the
//! cost lookup that `find_path_with_costs()` uses. It is separate from `PathGrid`
//! (which is boolean walkability) to keep the fast path working for simple queries.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on map/ (MapCell, TilesetLookup).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::SpeedType;

/// Normal terrain speed — no bonus, no penalty.
const COST_NORMAL: u8 = 100;
/// Rough terrain — slower movement for tracked/wheeled.
const COST_ROUGH: u8 = 75;
/// Blocked / impassable for this SpeedType.
const COST_BLOCKED: u8 = 0;

/// Per-cell speed modifier grid for one SpeedType.
///
/// Values: 0 = blocked, 100 = normal speed, <100 = slow terrain.
/// Built once per SpeedType from map data. The A* planner reads this to weight
/// step costs, making units avoid rough terrain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainCostGrid {
    costs: Vec<u8>,
    width: u16,
    height: u16,
}

/// Build every ground/naval terrain-cost row used by the simulation.
/// Winged movement deliberately has no grid because it ignores terrain.
pub(crate) fn build_canonical_terrain_cost_grids(
    terrain: &ResolvedTerrainGrid,
) -> BTreeMap<SpeedType, TerrainCostGrid> {
    SpeedType::ALL_WITH_COSTS
        .iter()
        .copied()
        .map(|speed_type| {
            (
                speed_type,
                TerrainCostGrid::from_resolved_terrain(terrain, speed_type),
            )
        })
        .collect()
}

impl TerrainCostGrid {
    /// Build a terrain cost grid from resolved terrain metadata.
    ///
    /// Uses the cell's INI-backed speed profile as the terrain substrate. When a
    /// profile is unavailable, the existing coarse terrain-cost classifier remains
    /// the weighting fallback; reduced-zone matrix rows are not SpeedType data.
    pub fn from_resolved_terrain(terrain: &ResolvedTerrainGrid, speed_type: SpeedType) -> Self {
        let size: usize = terrain.width() as usize * terrain.height() as usize;
        let mut costs: Vec<u8> = vec![COST_BLOCKED; size];

        for cell in terrain.iter() {
            let idx: usize = cell.ry as usize * terrain.width() as usize + cell.rx as usize;
            if idx >= costs.len() {
                continue;
            }
            let ramp_passable = cell.canonical_ramp.is_some();
            // Retail terrain-object occupation is a sub-cell mask on the ground
            // occupation plane, and only the infantry entry gate reads it: the
            // cell closes to infantry only when every functional sub-cell bit is
            // set. Vehicles keep the whole-cell block, so the relaxation is
            // scoped to the Foot row.
            let terrain_object_blocked = if speed_type == SpeedType::Foot {
                cell.terrain_object_occupation.is_some_and(|ini_bits| {
                    super::core::terrain_object_blocks_infantry(
                        super::core::terrain_object_cell_bits_from_ini(ini_bits),
                    )
                })
            } else {
                cell.terrain_object_blocks
            };
            let hard_blocked = (cell.is_cliff_like && !ramp_passable)
                || cell.overlay_blocks
                || terrain_object_blocked;
            // Bridge deck overrides underlying terrain (water/cliff) for ground units.
            // Units walk on the bridge surface, not the terrain below.
            let cost = if cell.is_elevated_bridge_cell() && !cell.overlay_blocks {
                COST_NORMAL
            } else if hard_blocked {
                COST_BLOCKED
            } else if ramp_passable {
                COST_NORMAL
            } else if let Some(resolved) = cell.speed_costs.cost_for_speed_type(speed_type) {
                // INI speed costs are the primary source — they come from rules.ini
                // [Clear], [Rough], [Tiberium], etc. sections and encode the actual
                // speed percentage per SpeedType. 0 = blocked, >0 = passable.
                resolved
            } else {
                classify_terrain_cost(
                    speed_type,
                    cell.is_water,
                    // `ground_walk_blocked` folds in the terrain object; for the
                    // Foot row the sub-cell rule above already decided that.
                    if speed_type == SpeedType::Foot {
                        cell.base_ground_walk_blocked || cell.overlay_blocks
                    } else {
                        cell.ground_walk_blocked
                    },
                    cell.is_rough,
                    cell.is_road,
                )
            };
            costs[idx] = cost;
        }

        Self {
            costs,
            width: terrain.width(),
            height: terrain.height(),
        }
    }

    /// Get the speed modifier for a cell (0 = blocked, 100 = normal).
    pub fn cost_at(&self, x: u16, y: u16) -> u8 {
        if x >= self.width || y >= self.height {
            return COST_BLOCKED;
        }
        self.costs[y as usize * self.width as usize + x as usize]
    }

    /// Map width in cells.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Map height in cells.
    pub fn height(&self) -> u16 {
        self.height
    }
}

/// Determine the terrain cost for a cell given its SpeedType and tile classification.
///
/// Roads use uniform cost (same as clear terrain), matching the original engine's
/// A* behavior where all passable cells have equal pathfinding weight.
fn classify_terrain_cost(
    speed_type: SpeedType,
    is_water: bool,
    is_cliff: bool,
    is_rough: bool,
    _is_road: bool,
) -> u8 {
    match speed_type {
        SpeedType::Foot => {
            if is_water || is_cliff {
                COST_BLOCKED
            } else if is_rough {
                // Infantry handle rough terrain better than vehicles.
                90
            } else {
                COST_NORMAL
            }
        }
        SpeedType::Track => {
            if is_water || is_cliff {
                COST_BLOCKED
            } else if is_rough {
                COST_ROUGH
            } else {
                COST_NORMAL
            }
        }
        SpeedType::Wheel => {
            if is_water || is_cliff {
                COST_BLOCKED
            } else if is_rough {
                // Wheeled vehicles are even slower on rough terrain.
                60
            } else {
                COST_NORMAL
            }
        }
        SpeedType::Float | SpeedType::FloatBeach | SpeedType::Hover => {
            // Hover/float units can cross water.
            if is_cliff { COST_BLOCKED } else { COST_NORMAL }
        }
        SpeedType::Amphibious => {
            // Amphibious units cross both land and water.
            if is_cliff { COST_BLOCKED } else { COST_NORMAL }
        }
        SpeedType::Winged => {
            // Aircraft ignore all terrain.
            COST_NORMAL
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    use crate::map::resolved_terrain::ResolvedTerrainGrid;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};

    #[test]
    fn test_track_on_water_is_blocked() {
        let cost = classify_terrain_cost(SpeedType::Track, true, false, false, false);
        assert_eq!(cost, COST_BLOCKED);
    }

    #[test]
    fn test_track_on_road_is_normal() {
        let cost = classify_terrain_cost(SpeedType::Track, false, false, false, true);
        assert_eq!(cost, COST_NORMAL);
    }

    #[test]
    fn test_track_on_rough_is_slow() {
        let cost = classify_terrain_cost(SpeedType::Track, false, false, true, false);
        assert_eq!(cost, COST_ROUGH);
    }

    #[test]
    fn test_float_on_water_is_passable() {
        let cost = classify_terrain_cost(SpeedType::Float, true, false, false, false);
        assert_eq!(cost, COST_NORMAL);
    }

    #[test]
    fn test_winged_ignores_terrain() {
        assert_eq!(
            classify_terrain_cost(SpeedType::Winged, true, true, true, false),
            COST_NORMAL
        );
    }

    #[test]
    fn test_foot_on_rough_is_less_penalized() {
        let foot = classify_terrain_cost(SpeedType::Foot, false, false, true, false);
        let track = classify_terrain_cost(SpeedType::Track, false, false, true, false);
        assert!(
            foot > track,
            "Infantry handle rough terrain better than vehicles"
        );
    }

    #[test]
    fn test_wheel_on_rough_is_most_penalized() {
        let wheel = classify_terrain_cost(SpeedType::Wheel, false, false, true, false);
        let track = classify_terrain_cost(SpeedType::Track, false, false, true, false);
        assert!(
            wheel < track,
            "Wheeled vehicles suffer more on rough terrain"
        );
    }

    #[test]
    fn test_from_resolved_terrain_uses_resolved_surface_classes() {
        use crate::sim::pathfinding::passability::LandType;
        let terrain = ResolvedTerrainGrid::from_cells(
            2,
            2,
            vec![
                ResolvedTerrainCell {
                    is_road: true,
                    land_type: LandType::Road.as_index(),
                    speed_costs: SpeedCostProfile {
                        track: Some(120),
                        ..SpeedCostProfile::default()
                    },
                    ..make_resolved_cell(0, 0)
                },
                ResolvedTerrainCell {
                    is_rough: true,
                    land_type: LandType::Rough.as_index(),
                    speed_costs: SpeedCostProfile {
                        track: Some(75),
                        wheel: Some(60),
                        foot: Some(90),
                        ..SpeedCostProfile::default()
                    },
                    ..make_resolved_cell(1, 0)
                },
                ResolvedTerrainCell {
                    is_water: true,
                    land_type: LandType::Water.as_index(),
                    ground_walk_blocked: true,
                    speed_costs: SpeedCostProfile {
                        track: Some(0),
                        hover: Some(100),
                        ..SpeedCostProfile::default()
                    },
                    ..make_resolved_cell(0, 1)
                },
                ResolvedTerrainCell {
                    is_cliff_like: true,
                    land_type: LandType::Rock.as_index(),
                    ground_walk_blocked: true,
                    ..make_resolved_cell(1, 1)
                },
            ],
        );
        let track = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Track);
        let hover = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Hover);
        // Road cell uses INI speed cost (Track=120) — no road bonus in A*.
        assert_eq!(track.cost_at(0, 0), 120);
        assert_eq!(track.cost_at(1, 0), COST_ROUGH);
        assert_eq!(track.cost_at(0, 1), COST_BLOCKED);
        assert_eq!(hover.cost_at(0, 1), COST_NORMAL);
        assert_eq!(hover.cost_at(1, 1), COST_BLOCKED);
    }

    #[test]
    fn gsi_04_13_synthetic_low_bridge_uses_ground_surface_cost_not_deck_override() {
        use crate::sim::pathfinding::passability::LandType;

        let custom_surface = SpeedCostProfile {
            track: Some(55),
            ..SpeedCostProfile::default()
        };
        let terrain = ResolvedTerrainGrid::from_cells(
            2,
            1,
            vec![
                ResolvedTerrainCell {
                    level: 2,
                    has_bridge_deck: true,
                    bridge_walkable: false,
                    bridge_deck_level: 2,
                    land_type: LandType::Rough.as_index(),
                    is_rough: true,
                    speed_costs: custom_surface,
                    ..make_resolved_cell(0, 0)
                },
                ResolvedTerrainCell {
                    level: 0,
                    has_bridge_deck: true,
                    bridge_walkable: true,
                    bridge_deck_level: 4,
                    land_type: LandType::Rough.as_index(),
                    is_rough: true,
                    speed_costs: custom_surface,
                    ..make_resolved_cell(1, 0)
                },
            ],
        );
        let track = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Track);

        assert_eq!(track.cost_at(0, 0), 55, "ground-level bridge uses TMP Land");
        assert_eq!(
            track.cost_at(1, 0),
            COST_NORMAL,
            "elevated deck overrides TMP Land"
        );
    }

    #[test]
    fn test_canonical_ramp_is_not_blocked_by_cliff_like_rock_land() {
        use crate::map::resolved_terrain::RampDirection;
        use crate::sim::pathfinding::passability::LandType;
        let terrain = ResolvedTerrainGrid::from_cells(
            1,
            1,
            vec![ResolvedTerrainCell {
                is_cliff_like: true,
                land_type: LandType::Rock.as_index(),
                canonical_ramp: Some(RampDirection::North),
                ground_walk_blocked: false,
                build_blocked: true,
                ..make_resolved_cell(0, 0)
            }],
        );
        let track = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Track);

        assert_eq!(track.cost_at(0, 0), COST_NORMAL);
    }

    /// A terrain object with `TemperateOccupationBits` short of the full
    /// sub-cell mask blocks vehicles but not infantry. 56 of 60 stock temperate
    /// terrain types are `=4`, i.e. a single sub-cell bit.
    fn tree_cell(rx: u16, ry: u16, bits: u8) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            terrain_object_occupation: Some(bits),
            terrain_object_blocks: bits != 0,
            // resolved terrain folds the terrain object into ground_walk_blocked;
            // base_ground_walk_blocked is the value without it.
            ground_walk_blocked: bits != 0,
            base_ground_walk_blocked: false,
            ..make_resolved_cell(rx, ry)
        }
    }

    #[test]
    fn partial_terrain_occupation_blocks_vehicles_but_not_infantry() {
        use crate::sim::pathfinding::PathGrid;
        let terrain = ResolvedTerrainGrid::from_cells(
            2,
            1,
            vec![tree_cell(0, 0, 4), make_resolved_cell(1, 0)],
        );
        let foot = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Foot);
        let track = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Track);
        assert_ne!(foot.cost_at(0, 0), COST_BLOCKED);
        assert_eq!(track.cost_at(0, 0), COST_BLOCKED);

        let grid = PathGrid::from_resolved_terrain(&terrain);
        assert!(grid.is_walkable_for_infantry(0, 0));
        assert!(!grid.is_walkable(0, 0));
        assert_eq!(grid.terrain_object_cell_bits_at(0, 0), 0x10);
    }

    #[test]
    fn full_terrain_occupation_blocks_infantry_too() {
        use crate::sim::pathfinding::PathGrid;
        let terrain = ResolvedTerrainGrid::from_cells(1, 1, vec![tree_cell(0, 0, 7)]);
        let foot = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Foot);
        assert_eq!(foot.cost_at(0, 0), COST_BLOCKED);

        let grid = PathGrid::from_resolved_terrain(&terrain);
        assert!(!grid.is_walkable_for_infantry(0, 0));
        assert!(!grid.is_walkable(0, 0));
        assert_eq!(grid.terrain_object_cell_bits_at(0, 0), 0x1C);
    }

    /// The relaxed Foot cost row is worthless unless the search is told the
    /// mover is infantry: the neighbour gate short-circuits on `is_walkable`,
    /// which is false for a partially-occupied tree cell, before the cost grid
    /// is ever consulted. This drives the production entry point end to end on a
    /// 3x1 corridor whose only middle cell is a `bits=4` tree — an infantryman
    /// must walk through it, a vehicle must fail outright.
    #[test]
    fn infantry_astar_routes_through_a_partially_occupied_tree_cell() {
        use crate::sim::pathfinding::{PathGrid, find_path_with_costs};
        let terrain = ResolvedTerrainGrid::from_cells(
            3,
            1,
            vec![
                make_resolved_cell(0, 0),
                tree_cell(1, 0, 4),
                make_resolved_cell(2, 0),
            ],
        );
        let grid = PathGrid::from_resolved_terrain(&terrain);
        let foot = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Foot);
        let track = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Track);

        let infantry_path = find_path_with_costs(
            &grid,
            (0, 0),
            (2, 0),
            Some(&foot),
            None,
            None,
            Some(&terrain),
            None,
            0,
            false,
            true,
        );
        assert_eq!(
            infantry_path,
            Some(vec![(0, 0), (1, 0), (2, 0)]),
            "infantry must path through a single-sub-cell tree",
        );

        let vehicle_path = find_path_with_costs(
            &grid,
            (0, 0),
            (2, 0),
            Some(&track),
            None,
            None,
            Some(&terrain),
            None,
            0,
            false,
            false,
        );
        assert_eq!(
            vehicle_path, None,
            "a vehicle has no route: the tree closes the whole cell",
        );
    }

    #[test]
    fn terrain_occupation_five_and_six_leave_one_subcell_for_infantry() {
        use crate::sim::pathfinding::PathGrid;
        for bits in [5u8, 6u8] {
            let terrain = ResolvedTerrainGrid::from_cells(1, 1, vec![tree_cell(0, 0, bits)]);
            let foot = TerrainCostGrid::from_resolved_terrain(&terrain, SpeedType::Foot);
            assert_ne!(foot.cost_at(0, 0), COST_BLOCKED, "bits={bits}");
            let grid = PathGrid::from_resolved_terrain(&terrain);
            assert!(grid.is_walkable_for_infantry(0, 0), "bits={bits}");
            assert!(!grid.is_walkable(0, 0), "bits={bits}");
        }
    }

    #[test]
    fn terrain_relaxation_never_reopens_a_cliff_cell() {
        use crate::sim::pathfinding::PathGrid;
        let cliff_with_tree = ResolvedTerrainCell {
            is_cliff_like: true,
            base_ground_walk_blocked: true,
            ..tree_cell(0, 0, 4)
        };
        let terrain = ResolvedTerrainGrid::from_cells(1, 1, vec![cliff_with_tree]);
        let grid = PathGrid::from_resolved_terrain(&terrain);
        assert!(!grid.is_walkable_for_infantry(0, 0));
    }

    fn make_resolved_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            height_in_pixels: 0,
            variant: 0,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: Default::default(),
            base_speed_costs: Default::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }
}
