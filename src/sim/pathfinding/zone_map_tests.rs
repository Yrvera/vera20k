//! Tests for zone map flood-fill and adjacency extraction.

use super::zone_build::{
    LocalHierarchyPatchResult, build_zone_map, incremental_rebuild_zone_hierarchy_around_cell,
};
use super::zone_hierarchy::{ZoneEdgeRecord, ZoneHierarchy, ZoneLevelGraph, ZoneRecord};
use super::zone_incremental::{
    PackedZoneCoord, ZoneRepairKind, ZoneRepairOutcome, repair_zone_cell,
};
use super::zone_map::*;
use crate::map::resolved_terrain::{
    BridgeDirection, BridgeLayer, ResolvedTerrainCell, ResolvedTerrainGrid, YR_CELL_LAND_TUNNEL,
    zone_class,
};
use crate::map::tube_facts::{TubeFact, TubeId, TubeSource};
use crate::rules::locomotor_type::MovementZone;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::bridge_state::{BridgeEndpointRecord, BridgeRecordKind, BridgeRuntimeState};
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::pathfinding::{PathCell, PathGrid};
use std::collections::BTreeMap;

// Helper: build a PathGrid from a string map where '.' = walkable, '#' = blocked.
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

// Helper: build zones for Normal movement zone with no cost grid (PathGrid only).
fn land_zones(grid: &PathGrid) -> (ZoneMap, ZoneAdjacency) {
    build_zone_map(
        grid,
        None,
        MovementZone::Normal,
        grid.width(),
        grid.height(),
    )
}

fn tiny_hierarchy() -> ZoneHierarchy {
    let mut level2 = ZoneLevelGraph::new(1);
    level2.set_record(ZoneRecord::new(1, 0, 0));
    let mut level1 = ZoneLevelGraph::new(1);
    level1.set_record(ZoneRecord::new(1, 1, 0));
    let mut level0 = ZoneLevelGraph::new(1);
    level0.set_record(ZoneRecord::new(1, 1, 0));
    level0.push_edge(1, ZoneEdgeRecord::new(1, 0));
    ZoneHierarchy::new(level0, level1, level2)
}

fn water_row_terrain(width: u16) -> ResolvedTerrainGrid {
    let cells = (0..width)
        .map(|rx| ResolvedTerrainCell {
            rx,
            ry: 0,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: crate::sim::pathfinding::passability::LandType::Water.as_index(),
            yr_cell_land_type: crate::sim::pathfinding::passability::LandType::Water.as_index(),
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Water,
            speed_costs: SpeedCostProfile::default(),
            is_water: true,
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
            zone_type: 4,
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
        })
        .collect();
    ResolvedTerrainGrid::from_cells(width, 1, cells)
}

fn clear_beach_water_row_terrain() -> ResolvedTerrainGrid {
    let land_types = [
        crate::sim::pathfinding::passability::LandType::Clear.as_index(),
        crate::sim::pathfinding::passability::LandType::Beach.as_index(),
        crate::sim::pathfinding::passability::LandType::Water.as_index(),
    ];
    let cells = land_types
        .into_iter()
        .enumerate()
        .map(|(rx, land_type)| ResolvedTerrainCell {
            rx: rx as u16,
            ry: 0,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type,
            yr_cell_land_type: land_type,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: match land_type {
                x if x == crate::sim::pathfinding::passability::LandType::Water.as_index() => {
                    TerrainClass::Water
                }
                x if x == crate::sim::pathfinding::passability::LandType::Beach.as_index() => {
                    TerrainClass::Beach
                }
                _ => TerrainClass::Clear,
            },
            speed_costs: SpeedCostProfile::default(),
            is_water: land_type == crate::sim::pathfinding::passability::LandType::Water.as_index(),
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
            zone_type: match land_type {
                x if x == crate::sim::pathfinding::passability::LandType::Water.as_index() => 4,
                x if x == crate::sim::pathfinding::passability::LandType::Beach.as_index() => 3,
                _ => 0,
            },
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
        })
        .collect();
    ResolvedTerrainGrid::from_cells(3, 1, cells)
}

fn stock_low_bridge_auto_shell_terrain() -> ResolvedTerrainGrid {
    let mut cells = Vec::new();
    let mut tubes = Vec::new();
    for rx in 0..5u16 {
        let is_low_bridge = (1..=3).contains(&rx);
        let tube_index = if is_low_bridge {
            let tube_id = TubeId(tubes.len() as u16);
            tubes.push(TubeFact::auto_low_bridge((rx, 0), 2));
            Some(tube_id)
        } else {
            None
        };
        cells.push(ResolvedTerrainCell {
            rx,
            ry: 0,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: crate::sim::pathfinding::passability::LandType::Clear.as_index(),
            yr_cell_land_type: if is_low_bridge {
                YR_CELL_LAND_TUNNEL
            } else {
                0
            },
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: if is_low_bridge {
                TerrainClass::Tunnel
            } else {
                TerrainClass::Clear
            },
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
            zone_type: zone_class::GROUND,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: Default::default(),
            base_speed_costs: Default::default(),
            build_blocked: false,
            has_bridge_deck: is_low_bridge,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: is_low_bridge.then(|| BridgeLayer {
                overlay_id: 0x4a,
                overlay_name: "LOBRDG01".to_string(),
                deck_level: 0,
                direction: BridgeDirection::Low,
            }),
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        });
    }
    ResolvedTerrainGrid::from_cells_with_tubes(5, 1, cells, tubes)
}

fn terrain_from_zone_classes(
    width: u16,
    height: u16,
    classes: &[u8],
    levels: &[u8],
) -> ResolvedTerrainGrid {
    assert_eq!(classes.len(), width as usize * height as usize);
    assert_eq!(levels.len(), classes.len());
    let prototype = water_row_terrain(1).cells.into_iter().next().unwrap();
    let cells = classes
        .iter()
        .zip(levels)
        .enumerate()
        .map(|(index, (&zone_type, &level))| {
            let mut cell = prototype.clone();
            cell.rx = (index % width as usize) as u16;
            cell.ry = (index / width as usize) as u16;
            cell.level = level;
            cell.zone_type = zone_type;
            cell.outside_playfield = zone_type == zone_class::OUTSIDE;
            cell.is_water = zone_type == zone_class::WATER;
            cell.land_type = if cell.is_water {
                crate::sim::pathfinding::passability::LandType::Water.as_index()
            } else if zone_type == zone_class::BEACH {
                crate::sim::pathfinding::passability::LandType::Beach.as_index()
            } else {
                crate::sim::pathfinding::passability::LandType::Clear.as_index()
            };
            cell.yr_cell_land_type = cell.land_type;
            cell.terrain_class = if cell.is_water {
                TerrainClass::Water
            } else if zone_type == zone_class::BEACH {
                TerrainClass::Beach
            } else {
                TerrainClass::Clear
            };
            cell.ground_walk_blocked = false;
            cell.terrain_object_blocks = false;
            cell.overlay_blocks = false;
            cell
        })
        .collect();
    ResolvedTerrainGrid::from_cells(width, height, cells)
}

fn native_nonbridge_zone_fixture(width: u16, height: u16) -> ZoneGrid {
    let cell_count = usize::from(width) * usize::from(height);
    let terrain = terrain_from_zone_classes(
        width,
        height,
        &vec![zone_class::GROUND; cell_count],
        &vec![0; cell_count],
    );
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    ZoneGrid::build_with_terrain(
        &path_grid,
        &BTreeMap::new(),
        Some(&terrain),
        &[],
        width,
        height,
    )
}

#[test]
fn gsi_04_01_nonbridge_getzoneid_uses_padded_square_clamps_and_raw_rows() {
    let mut zones = native_nonbridge_zone_fixture(2, 2);
    {
        let base = zones.base_topology_mut().unwrap();
        base.movement_classes = vec![0; 4];
        base.zone_ids = vec![2, 3, 4, 5];
        base.zone_count = 5;
        for row in &mut base.raw_zone_ids_by_row {
            row.resize(6, 0);
        }
        base.raw_zone_ids_by_row[MovementZone::Normal.matrix_row().unwrap()][0] = 41;
        base.raw_zone_ids_by_row[MovementZone::Normal.matrix_row().unwrap()][2] = 42;
        base.raw_zone_ids_by_row[MovementZone::Crusher.matrix_row().unwrap()][2] = 1;
        base.raw_zone_ids_by_row[MovementZone::Destroyer.matrix_row().unwrap()][2] = u16::MAX;
        base.raw_zone_ids_by_row[MovementZone::Amphibious.matrix_row().unwrap()][2] = 73;
    }

    assert_eq!(
        zones.get_zone_id_nonbridge_native((2, 0), MovementZone::Normal),
        Some(41),
        "the native W+1 padded final column owns base cluster 0"
    );
    assert_eq!(
        zones.get_zone_id_nonbridge_native((0, 2), MovementZone::Normal),
        Some(41),
        "the native W+1 padded final row owns base cluster 0"
    );
    assert_eq!(
        zones.get_zone_id_nonbridge_native((-1, 0), MovementZone::Normal),
        Some(42),
        "a negative linear index clamps to the first real base entry"
    );
    assert_eq!(
        zones.get_zone_id_nonbridge_native((u16::MAX.into(), 0), MovementZone::Normal),
        Some(42),
        "coordinate components truncate to their packed signed-i16 words"
    );
    assert_eq!(
        zones
            .get_zone_id_nonbridge_native((i16::MAX.into(), i16::MAX.into()), MovementZone::Normal),
        Some(41),
        "an oversized positive linear index clamps to the final padded entry"
    );

    assert_eq!(
        zones.get_zone_id_nonbridge_native((0, 0), MovementZone::Normal),
        Some(42)
    );
    assert_eq!(
        zones.get_zone_id_nonbridge_native((0, 0), MovementZone::Crusher),
        Some(1),
        "raw reserved label 1 is not flattened"
    );
    assert_eq!(
        zones.get_zone_id_nonbridge_native((0, 0), MovementZone::Destroyer),
        Some(u16::MAX),
        "raw 0xffff is not flattened"
    );
    assert_eq!(
        zones.get_zone_id_nonbridge_native((0, 0), MovementZone::Amphibious),
        Some(73),
        "the same base cluster projects through the selected movement row"
    );
}

#[test]
fn gsi_04_01_nonbridge_getzoneid_rejects_non_native_topology_metadata() {
    let path_grid = PathGrid::new(2, 2);
    let compatibility_only = ZoneGrid::build(&path_grid, &BTreeMap::new(), 2, 2);
    assert_eq!(
        compatibility_only.get_zone_id_nonbridge_native((0, 0), MovementZone::Normal),
        None,
        "flattened compatibility maps have no native base topology authority"
    );

    let nonsquare = native_nonbridge_zone_fixture(2, 1);
    assert_eq!(
        nonsquare.get_zone_id_nonbridge_native((0, 0), MovementZone::Normal),
        None
    );

    let mut inconsistent = native_nonbridge_zone_fixture(2, 2);
    inconsistent.base_topology_mut().unwrap().zone_ids.pop();
    assert_eq!(
        inconsistent.get_zone_id_nonbridge_native((0, 0), MovementZone::Normal),
        None
    );

    let mut missing_raw_cluster = native_nonbridge_zone_fixture(2, 2);
    {
        let base = missing_raw_cluster.base_topology_mut().unwrap();
        base.zone_ids[0] = 500;
        base.raw_zone_ids_by_row[MovementZone::Normal.matrix_row().unwrap()].truncate(2);
    }
    assert_eq!(
        missing_raw_cluster.get_zone_id_nonbridge_native((0, 0), MovementZone::Normal),
        None
    );
    assert_eq!(
        missing_raw_cluster.get_zone_id_nonbridge_native((0, 0), MovementZone::Invalid),
        None,
        "native's unchecked invalid row is an explicit safe failure in Rust"
    );
}

fn base_defense_reachability_fixture() -> ZoneGrid {
    let mut zones = native_nonbridge_zone_fixture(4, 4);
    let base = zones.base_topology_mut().unwrap();
    base.movement_classes = vec![0; 16];
    base.zone_ids = (2..18).collect();
    base.zone_count = 17;
    let row = MovementZone::Normal.matrix_row().unwrap();
    base.raw_zone_ids_by_row[row] = (0..18).map(|cluster| 100 + cluster).collect();
    zones
}

#[test]
fn gsi_04_05_base_defense_reachability_preserves_bypass_fringe_and_raw_equality() {
    let zones = base_defense_reachability_fixture();
    assert!(zones.can_reach_base_defense_response(
        None,
        (0, 0),
        (3, 3),
        false,
        true,
        4,
        4,
    ));

    assert!(zones.can_reach_base_defense_response(
        Some(MovementZone::Normal),
        (3, 2),
        (0, 0),
        false,
        false,
        4,
        4,
    ));
    assert!(!zones.can_reach_base_defense_response(
        Some(MovementZone::Normal),
        (3, 2),
        (0, 0),
        false,
        true,
        4,
        4,
    ));

    assert!(zones.can_reach_base_defense_response(
        Some(MovementZone::Normal),
        (4, 0),
        (0, 4),
        false,
        true,
        4,
        4,
    ), "two raw padded-cluster-zero labels compare equal");
}

#[test]
fn gsi_04_05_base_defense_reachability_redirects_only_the_candidate_bridge_side() {
    let mut zones = base_defense_reachability_fixture();
    let mut redirect = vec![None; 16];
    redirect[5] = Some((3, 3));
    zones
        .map_mut(MovementZone::Normal)
        .unwrap()
        .set_bridge_redirect(Some(redirect));

    assert!(zones.can_reach_base_defense_response(
        Some(MovementZone::Normal),
        (1, 1),
        (3, 3),
        true,
        true,
        4,
        4,
    ));
    assert!(!zones.can_reach_base_defense_response(
        Some(MovementZone::Normal),
        (1, 1),
        (3, 3),
        false,
        true,
        4,
        4,
    ));
}

#[test]
fn gsi_04_06_scanline_storage_fringe_merges_isometric_cardinal_cells() {
    let terrain = terrain_from_zone_classes(
        2,
        2,
        &[zone_class::GROUND, 7, 7, zone_class::GROUND],
        &[0; 4],
    );
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 2, 2);
    let normal = zones.map_for(MovementZone::Normal).unwrap();

    assert_eq!(normal.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(normal.zone_at(1, 1, MovementLayer::Ground), 2);
}

#[test]
fn gsi_04_06_scanline_fringe_edge_merges_amphibious_class_transition() {
    let terrain = terrain_from_zone_classes(
        2,
        2,
        &[zone_class::GROUND, 7, 7, zone_class::BEACH],
        &[0; 4],
    );
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 2, 2);

    let normal = zones.map_for(MovementZone::Normal).unwrap();
    assert_eq!(normal.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(normal.zone_at(1, 1, MovementLayer::Ground), ZONE_INVALID);

    let amphibious = zones.map_for(MovementZone::Amphibious).unwrap();
    assert_eq!(amphibious.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(amphibious.zone_at(1, 1, MovementLayer::Ground), 2);
}

#[test]
fn gsi_04_06_base_fill_allows_right_delta_three_but_not_vertical() {
    let east = terrain_from_zone_classes(2, 1, &[0, 0], &[0, 3]);
    let east_path = PathGrid::from_resolved_terrain(&east);
    let east_zones =
        ZoneGrid::build_with_terrain(&east_path, &BTreeMap::new(), Some(&east), &[], 2, 1);
    let east_normal = east_zones.map_for(MovementZone::Normal).unwrap();
    assert_eq!(
        east_normal.zone_at(0, 0, MovementLayer::Ground),
        east_normal.zone_at(1, 0, MovementLayer::Ground)
    );

    let vertical = terrain_from_zone_classes(1, 2, &[0, 0], &[0, 3]);
    let vertical_path = PathGrid::from_resolved_terrain(&vertical);
    let vertical_zones =
        ZoneGrid::build_with_terrain(&vertical_path, &BTreeMap::new(), Some(&vertical), &[], 1, 2);
    let vertical_normal = vertical_zones.map_for(MovementZone::Normal).unwrap();
    assert_eq!(vertical_normal.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(vertical_normal.zone_at(0, 1, MovementLayer::Ground), 3);
}

#[test]
fn gsi_04_06_class_transition_merges_for_amphibious_not_normal() {
    let terrain =
        terrain_from_zone_classes(2, 1, &[zone_class::GROUND, zone_class::BEACH], &[0; 2]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 2, 1);

    let amphibious = zones.map_for(MovementZone::Amphibious).unwrap();
    assert_eq!(amphibious.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(amphibious.zone_at(1, 0, MovementLayer::Ground), 2);

    let normal = zones.map_for(MovementZone::Normal).unwrap();
    assert_eq!(normal.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(normal.zone_at(1, 0, MovementLayer::Ground), ZONE_INVALID);
}

#[test]
fn gsi_04_06_class_six_boundary_bypasses_height_for_subterranean_row() {
    let terrain =
        terrain_from_zone_classes(2, 1, &[zone_class::GROUND, zone_class::IMPASSABLE], &[0, 7]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 2, 1);
    let subterranean = zones.map_for(MovementZone::Subterranean).unwrap();

    assert_eq!(subterranean.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(subterranean.zone_at(1, 0, MovementLayer::Ground), 2);
}

#[test]
fn gsi_04_06_active_bridge_edge_merges_base_zones_before_projection() {
    let terrain = terrain_from_zone_classes(5, 1, &[0, 7, 7, 7, 0], &[0; 5]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let without_bridge =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 5, 1);
    let normal = without_bridge.map_for(MovementZone::Normal).unwrap();
    assert_eq!(normal.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(normal.zone_at(4, 0, MovementLayer::Ground), 3);

    let records = [BridgeEndpointRecord {
        endpoint_a: (0, 0),
        endpoint_b: (4, 0),
        group_id: 1,
        active: true,
        bridge_kind: BridgeRecordKind::High,
    }];
    let with_bridge =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &records, 5, 1);
    let normal = with_bridge.map_for(MovementZone::Normal).unwrap();
    assert_eq!(normal.zone_at(0, 0, MovementLayer::Ground), 2);
    assert_eq!(normal.zone_at(4, 0, MovementLayer::Ground), 2);
}

#[test]
fn gsi_04_06_all_thirteen_rows_preserve_native_derived_labels() {
    let classes = [0, 7, 1, 7, 2, 7, 3, 7, 4, 7, 5, 7, 6];
    let terrain = terrain_from_zone_classes(13, 1, &classes, &[0; 13]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 13, 1);

    assert_eq!(MovementZone::all_ground().len(), 13);
    for &movement_zone in MovementZone::all_ground() {
        let row = crate::sim::pathfinding::passability::MOVEMENT_ZONE_PASSABILITY
            [movement_zone.matrix_row().unwrap()];
        let map = zones.map_for(movement_zone).unwrap();
        let mut next_label = 2;
        for class in 0..=6u8 {
            let x = u16::from(class) * 2;
            let expected = if row[class as usize] == 1 {
                let label = next_label;
                next_label += 1;
                label
            } else {
                ZONE_INVALID
            };
            assert_eq!(
                map.zone_at(x, 0, MovementLayer::Ground),
                expected,
                "{movement_zone:?} class {class}"
            );
        }
        assert_eq!(map.zone_count, next_label - 1, "{movement_zone:?}");
        assert_eq!(map.zone_at(1, 0, MovementLayer::Ground), ZONE_INVALID);
    }
}

#[test]
fn gsi_04_06_simulation_rebuild_initializes_zone_grid() {
    let terrain = terrain_from_zone_classes(1, 1, &[zone_class::GROUND], &[0]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let mut sim = crate::sim::world::Simulation::new();
    sim.resolved_terrain = Some(terrain);

    sim.rebuild_zone_grid(&path_grid);

    let normal = sim
        .zone_grid
        .as_ref()
        .and_then(|zones| zones.map_for(MovementZone::Normal))
        .unwrap();
    assert_eq!(normal.zone_at(0, 0, MovementLayer::Ground), 2);
}

#[test]
fn gsi_04_06_pathgrid_blocking_does_not_rewrite_cell_owned_reduced_class() {
    let terrain = terrain_from_zone_classes(
        3,
        1,
        &[zone_class::GROUND, zone_class::CRUSHABLE, zone_class::WALL],
        &[0; 3],
    );
    let mut path_grid = PathGrid::from_resolved_terrain(&terrain);
    for x in 0..3 {
        path_grid.set_blocked(x, 0, true);
    }
    let zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 3, 1);

    assert_ne!(
        zones
            .map_for(MovementZone::Normal)
            .unwrap()
            .zone_at(0, 0, MovementLayer::Ground),
        ZONE_INVALID,
        "a PathGrid bit cannot turn Ground into Building"
    );
    assert_ne!(
        zones
            .map_for(MovementZone::Crusher)
            .unwrap()
            .zone_at(1, 0, MovementLayer::Ground),
        ZONE_INVALID,
        "Crusher must retain the Crushable column"
    );
    assert_eq!(
        zones
            .map_for(MovementZone::Infantry)
            .unwrap()
            .zone_at(2, 0, MovementLayer::Ground),
        ZONE_INVALID,
        "Wall must not be coerced to Infantry-passable Building"
    );
}

#[test]
fn gsi_04_06_simulation_detects_class_only_change_with_identical_pathgrid() {
    let terrain = terrain_from_zone_classes(1, 1, &[zone_class::GROUND], &[0]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let mut sim = crate::sim::world::Simulation::new();
    sim.resolved_terrain = Some(terrain);
    sim.rebuild_zone_grid(&path_grid);
    assert_ne!(
        sim.zone_grid
            .as_ref()
            .unwrap()
            .map_for(MovementZone::Normal)
            .unwrap()
            .zone_at(0, 0, MovementLayer::Ground),
        ZONE_INVALID
    );

    sim.resolved_terrain.as_mut().unwrap().cells[0].zone_type = zone_class::CRUSHABLE;
    sim.rebuild_zone_grid(&path_grid);

    let zones = sim.zone_grid.as_ref().unwrap();
    assert_eq!(
        zones
            .map_for(MovementZone::Normal)
            .unwrap()
            .zone_at(0, 0, MovementLayer::Ground),
        ZONE_INVALID
    );
    assert_ne!(
        zones
            .map_for(MovementZone::Crusher)
            .unwrap()
            .zone_at(0, 0, MovementLayer::Ground),
        ZONE_INVALID
    );
}

fn base_repair_fixture(
    classes: [u8; 9],
    clusters: [ZoneId; 9],
) -> (ResolvedTerrainGrid, PathGrid, ZoneGrid) {
    let terrain = terrain_from_zone_classes(3, 3, &classes, &[0; 9]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let mut zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 3, 3);
    let base = zones.base_topology_mut().unwrap();
    base.movement_classes = classes.to_vec();
    base.zone_ids = clusters.to_vec();
    base.zone_count = clusters.iter().copied().max().unwrap_or(0);
    base.adjacency
        .neighbors
        .resize(base.zone_count as usize + 1, Vec::new());
    for row in &mut base.raw_zone_ids_by_row {
        row.resize(base.zone_count as usize + 1, 1);
        row[0] = u16::MAX;
        for cluster in 1..=base.zone_count {
            row[cluster as usize] = cluster + 1;
        }
    }
    let flat_ids: Vec<ZoneId> = clusters
        .iter()
        .map(|&cluster| {
            (cluster != ZONE_INVALID)
                .then_some(cluster + 1)
                .unwrap_or(ZONE_INVALID)
        })
        .collect();
    let flat_zone_count = base.zone_count + 1;
    for &movement_zone in MovementZone::all_ground() {
        let map = zones.map_mut(movement_zone).unwrap();
        *map.zone_ids_mut() = flat_ids.clone();
        map.set_zone_count(flat_zone_count);
        map.set_zone_info(super::zone_build::compute_zone_info(
            map.zone_ids_slice(),
            3,
            3,
            map.zone_count,
        ));
    }
    (terrain, path_grid, zones)
}

#[test]
fn gsi_04_06_base_repair_uses_transition_count_first_candidate_and_preserves_tables() {
    // Neighbor order from center: N=A, NE=B, E=A, then sentinels. This is
    // exactly three row-0 mapping transitions, so the first candidate wins.
    let classes = [7, 0, 2, 7, 0, 0, 7, 7, 7];
    let clusters = [0, 1, 2, 0, 3, 3, 0, 0, 0];
    let (terrain, path_grid, mut zones) = base_repair_fixture(classes, clusters);
    for &movement_zone in MovementZone::all_ground() {
        let map = zones.map_for(movement_zone).unwrap();
        assert_eq!(map.zone_at(1, 1, MovementLayer::Ground), 4);
        assert_eq!(map.info_for(2).unwrap().cell_count, 1);
        assert_eq!(map.info_for(2).unwrap().center, (1, 0));
        assert_eq!(map.info_for(4).unwrap().cell_count, 2);
        assert_eq!(map.info_for(4).unwrap().center, (1, 1));
    }
    let (before_count, before_adj, before_raw, before_clusters) = {
        let base = zones.base_topology_mut().unwrap();
        (
            base.zone_count,
            base.adjacency.neighbors.clone(),
            base.raw_zone_ids_by_row.clone(),
            base.zone_ids.clone(),
        )
    };
    let before_maps: Vec<Vec<ZoneId>> = MovementZone::all_ground()
        .iter()
        .map(|&mz| zones.map_for(mz).unwrap().zone_ids_slice().to_vec())
        .collect();

    let outcome = repair_zone_cell(
        &mut zones,
        PackedZoneCoord::new(1, 1),
        ZoneRepairKind::AssignOrphaned,
        &path_grid,
        &BTreeMap::new(),
        &terrain,
        &[],
    );
    assert_eq!(outcome, ZoneRepairOutcome::Adopted { cluster: 1 });

    {
        let base = zones.base_topology_mut().unwrap();
        assert_eq!(base.zone_count, before_count);
        assert_eq!(base.adjacency.neighbors, before_adj);
        assert_eq!(base.raw_zone_ids_by_row, before_raw);
        assert_eq!(base.zone_ids[4], 1);
        for (index, (&before, &after)) in before_clusters.iter().zip(&base.zone_ids).enumerate() {
            if index != 4 {
                assert_eq!(after, before, "unrelated base cell {index}");
            }
        }
    }
    for (row, &movement_zone) in MovementZone::all_ground().iter().enumerate() {
        let after = zones.map_for(movement_zone).unwrap().zone_ids_slice();
        for index in 0..after.len() {
            if index != 4 {
                assert_eq!(
                    after[index], before_maps[row][index],
                    "row {row} cell {index}"
                );
            }
        }
        assert_eq!(before_maps[row][4], 4);
        assert_eq!(after[4], 2, "target inherits candidate cluster mapping");
        let adopted = zones.map_for(movement_zone).unwrap().info_for(2).unwrap();
        assert_eq!(adopted.cell_count, 2);
        assert_eq!(adopted.center, (1, 0));
        let vacated = zones.map_for(movement_zone).unwrap().info_for(4).unwrap();
        assert_eq!(vacated.cell_count, 1);
        assert_eq!(vacated.center, (2, 1));
    }
}

#[test]
fn gsi_04_06_base_repair_fallbacks_and_explicit_merge_provenance() {
    let alternating_classes = [7, 0, 2, 7, 0, 0, 7, 7, 2];
    let alternating_clusters = [0, 1, 2, 0, 3, 1, 0, 0, 2];
    let (terrain, path_grid, mut zones) =
        base_repair_fixture(alternating_classes, alternating_clusters);
    assert_eq!(
        repair_zone_cell(
            &mut zones,
            PackedZoneCoord::new(1, 1),
            ZoneRepairKind::AssignOrphaned,
            &path_grid,
            &BTreeMap::new(),
            &terrain,
            &[],
        ),
        ZoneRepairOutcome::FullRebuild,
        "A/B/A/B reaches four transitions and must rebuild"
    );

    let non_ground_assign_classes = [7, 0, 2, 7, 2, 7, 7, 7, 7];
    let non_ground_assign_clusters = [0, 1, 2, 0, 2, 0, 0, 0, 0];
    let (terrain, path_grid, mut zones) =
        base_repair_fixture(non_ground_assign_classes, non_ground_assign_clusters);
    assert_eq!(
        repair_zone_cell(
            &mut zones,
            PackedZoneCoord::new(1, 1),
            ZoneRepairKind::AssignOrphaned,
            &path_grid,
            &BTreeMap::new(),
            &terrain,
            &[],
        ),
        ZoneRepairOutcome::FullRebuild,
        "AssignOrphaned cannot adopt when the target is non-ground"
    );

    let merge_classes = [7, 2, 0, 7, 2, 2, 7, 7, 7];
    let merge_clusters = [0, 2, 1, 0, 3, 2, 0, 0, 0];
    let (terrain, path_grid, mut zones) = base_repair_fixture(merge_classes, merge_clusters);
    assert_eq!(
        repair_zone_cell(
            &mut zones,
            PackedZoneCoord::new(1, 1),
            ZoneRepairKind::MergeAdjacent,
            &path_grid,
            &BTreeMap::new(),
            &terrain,
            &[],
        ),
        ZoneRepairOutcome::Adopted { cluster: 2 },
        "Merge adopts the first same-type non-ground neighbor"
    );

    let mut sentinel = terrain_from_zone_classes(1, 1, &[zone_class::OUTSIDE], &[0]);
    sentinel.cells[0].outside_playfield = false;
    let sentinel_path = PathGrid::from_resolved_terrain(&sentinel);
    let mut sentinel_zones =
        ZoneGrid::build_with_terrain(&sentinel_path, &BTreeMap::new(), Some(&sentinel), &[], 1, 1);
    assert_eq!(
        repair_zone_cell(
            &mut sentinel_zones,
            PackedZoneCoord::new(0, 0),
            ZoneRepairKind::MergeAdjacent,
            &sentinel_path,
            &BTreeMap::new(),
            &sentinel,
            &[],
        ),
        ZoneRepairOutcome::SentinelNoOp
    );
    assert_eq!(
        repair_zone_cell(
            &mut sentinel_zones,
            PackedZoneCoord::new(-1, 0),
            ZoneRepairKind::MergeAdjacent,
            &sentinel_path,
            &BTreeMap::new(),
            &sentinel,
            &[],
        ),
        ZoneRepairOutcome::OutsideNoOp
    );
}

#[derive(Debug, PartialEq, Eq)]
struct HierarchyRegionSnapshot {
    cell_ids: Vec<ZoneId>,
    records: Vec<(ZoneId, Option<ZoneRecord>, Vec<ZoneEdgeRecord>)>,
}

fn hierarchy_region_snapshot(
    hierarchy: &ZoneHierarchy,
    width: u16,
    height: u16,
    x_min: u16,
) -> Vec<HierarchyRegionSnapshot> {
    (0..3)
        .map(|level| {
            let graph = hierarchy.level(level).unwrap();
            let mut cell_ids = Vec::new();
            let mut record_ids = Vec::new();
            for y in 0..height {
                for x in x_min..width {
                    let zone = graph.zone_at(x, y);
                    cell_ids.push(zone);
                    if zone != ZONE_INVALID && !record_ids.contains(&zone) {
                        record_ids.push(zone);
                    }
                }
            }
            let records = record_ids
                .into_iter()
                .map(|zone| (zone, graph.record(zone), graph.edges(zone).to_vec()))
                .collect();
            HierarchyRegionSnapshot { cell_ids, records }
        })
        .collect()
}

#[test]
fn gsi_04_06_fallback_rebuilds_base_without_resetting_hierarchy_high_water() {
    let width = 16;
    let height = 8;
    let classes: Vec<u8> = (0..height)
        .flat_map(|_| {
            (0..width).map(|x| {
                if x == 7 {
                    zone_class::OUTSIDE
                } else if x >= 12 {
                    zone_class::WALL
                } else {
                    zone_class::GROUND
                }
            })
        })
        .collect();
    let terrain = terrain_from_zone_classes(width, height, &classes, &vec![0; classes.len()]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let mut zones = ZoneGrid::build_with_terrain(
        &path_grid,
        &BTreeMap::new(),
        Some(&terrain),
        &[],
        width,
        height,
    );
    let mut expected = ZoneGrid::build_with_terrain(
        &path_grid,
        &BTreeMap::new(),
        Some(&terrain),
        &[],
        width,
        height,
    );

    let (initial_slots, initial_right_ids) = {
        let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
        (
            std::array::from_fn::<_, 3, _>(|level| {
                hierarchy.level(level).unwrap().record_slot_count()
            }),
            std::array::from_fn::<_, 3, _>(|level| hierarchy.level(level).unwrap().zone_at(10, 2)),
        )
    };
    {
        let (base, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
        assert_eq!(
            incremental_rebuild_zone_hierarchy_around_cell(
                hierarchy,
                base,
                &path_grid,
                &terrain,
                &[],
                (10, 2),
                width,
                height,
            ),
            LocalHierarchyPatchResult::Patched
        );
    }

    let prior_high_water = {
        let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
        std::array::from_fn::<_, 3, _>(|level| {
            let graph = hierarchy.level(level).unwrap();
            assert!(graph.record_slot_count() > initial_slots[level]);
            assert!(graph.record(initial_right_ids[level]).is_some());
            assert!(graph.edges(initial_right_ids[level]).is_empty());
            graph.record_slot_count()
        })
    };
    let right_before = {
        let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
        hierarchy_region_snapshot(hierarchy, width, height, 8)
    };
    assert!(
        right_before
            .iter()
            .any(|level| { level.records.iter().any(|(_, _, edges)| !edges.is_empty()) })
    );

    let expected_base = expected.base_topology_mut().unwrap().clone();
    let expected_rows: Vec<(MovementZone, Vec<ZoneId>, Vec<Vec<ZoneId>>)> =
        MovementZone::all_ground()
            .iter()
            .map(|&movement_zone| {
                (
                    movement_zone,
                    expected
                        .map_for(movement_zone)
                        .unwrap()
                        .zone_ids_slice()
                        .to_vec(),
                    expected
                        .adjacency_for(movement_zone)
                        .unwrap()
                        .neighbors
                        .clone(),
                )
            })
            .collect();

    let index = |x: usize, y: usize| y * width as usize + x;
    {
        let base = zones.base_topology_mut().unwrap();
        for &(x, y, zone_type, cluster) in &[
            (2, 1, zone_class::GROUND, 1),
            (3, 1, zone_class::WALL, 2),
            (3, 2, zone_class::GROUND, 1),
            (3, 3, zone_class::WALL, 2),
            (2, 3, zone_class::OUTSIDE, 0),
            (1, 3, zone_class::OUTSIDE, 0),
            (1, 2, zone_class::OUTSIDE, 0),
            (1, 1, zone_class::OUTSIDE, 0),
        ] {
            base.movement_classes[index(x, y)] = zone_type;
            base.zone_ids[index(x, y)] = cluster;
        }
        base.movement_classes[index(2, 2)] = zone_class::GROUND;
        base.zone_ids[index(2, 2)] = 1;
        base.raw_zone_ids_by_row[0][0] = u16::MAX;
        base.raw_zone_ids_by_row[0][1] = 2;
        base.raw_zone_ids_by_row[0][2] = 3;

        // A distant, deliberately stale base/projection entry proves the
        // fallback refresh is global even though the hierarchy patch is local.
        base.movement_classes[index(14, 2)] = zone_class::GROUND;
        base.zone_ids[index(14, 2)] = 1;
    }
    zones.project_adopted_base_cell(index(14, 2));
    assert_ne!(
        zones
            .map_for(MovementZone::Normal)
            .unwrap()
            .zone_at(14, 2, MovementLayer::Ground),
        expected
            .map_for(MovementZone::Normal)
            .unwrap()
            .zone_at(14, 2, MovementLayer::Ground)
    );

    assert_eq!(
        repair_zone_cell(
            &mut zones,
            PackedZoneCoord::new(2, 2),
            ZoneRepairKind::AssignOrphaned,
            &path_grid,
            &BTreeMap::new(),
            &terrain,
            &[],
        ),
        ZoneRepairOutcome::FullRebuild,
        "the ordered A/B/A/B neighborhood has exactly four transitions"
    );

    {
        let actual = zones.base_topology_mut().unwrap();
        assert_eq!(actual.movement_classes, expected_base.movement_classes);
        assert_eq!(actual.zone_ids, expected_base.zone_ids);
        assert_eq!(actual.zone_count, expected_base.zone_count);
        assert_eq!(
            actual.adjacency.neighbors,
            expected_base.adjacency.neighbors
        );
        assert_eq!(
            actual.raw_zone_ids_by_row,
            expected_base.raw_zone_ids_by_row
        );
    }
    for (movement_zone, expected_ids, expected_adjacency) in expected_rows {
        assert_eq!(
            zones.map_for(movement_zone).unwrap().zone_ids_slice(),
            expected_ids
        );
        assert_eq!(
            zones.adjacency_for(movement_zone).unwrap().neighbors,
            expected_adjacency
        );
    }

    let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
    assert_eq!(
        hierarchy_region_snapshot(hierarchy, width, height, 8),
        right_before,
        "the isolated hierarchy block retains IDs, metadata, and edge order"
    );
    for (level, &high_water) in prior_high_water.iter().enumerate() {
        let graph = hierarchy.level(level).unwrap();
        assert_eq!(graph.zone_at(2, 2), high_water as ZoneId);
        assert!(graph.record_slot_count() > high_water);
    }
}

#[test]
fn gsi_04_06_local_hierarchy_patch_keeps_stale_holes_and_appends_edges_stably() {
    let terrain = terrain_from_zone_classes(6, 1, &[zone_class::GROUND; 6], &[0; 6]);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    let mut zones =
        ZoneGrid::build_with_terrain(&path_grid, &BTreeMap::new(), Some(&terrain), &[], 6, 1);
    let (old, middle, right, old_slots) = {
        let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
        let level0 = hierarchy.level(0).unwrap();
        (
            level0.zone_at(0, 0),
            level0.zone_at(2, 0),
            level0.zone_at(5, 0),
            level0.record_slot_count(),
        )
    };

    assert_eq!(
        repair_zone_cell(
            &mut zones,
            PackedZoneCoord::new(-1, 0),
            ZoneRepairKind::MergeAdjacent,
            &path_grid,
            &BTreeMap::new(),
            &terrain,
            &[],
        ),
        ZoneRepairOutcome::OutsideNoOp
    );
    {
        let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
        let level0 = hierarchy.level(0).unwrap();
        assert_eq!(level0.zone_at(0, 0), old);
        assert_eq!(level0.zone_at(2, 0), middle);
        assert_eq!(level0.zone_at(5, 0), right);
        assert_eq!(level0.record_slot_count(), old_slots);
    }

    assert_eq!(
        repair_zone_cell(
            &mut zones,
            PackedZoneCoord::new(0, 0),
            ZoneRepairKind::MergeAdjacent,
            &path_grid,
            &BTreeMap::new(),
            &terrain,
            &[],
        ),
        ZoneRepairOutcome::Adopted { cluster: 1 }
    );

    let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
    let level0 = hierarchy.level(0).unwrap();
    let replacement = level0.zone_at(0, 0);
    assert_ne!(replacement, old);
    assert_eq!(level0.zone_at(2, 0), middle, "2x2 alignment boundary");
    assert_eq!(level0.zone_at(5, 0), right, "unrelated level-0 block");
    assert!(level0.record(old).is_some(), "old slot remains allocated");
    assert!(level0.edges(old).is_empty(), "stale slot edges are cleared");
    assert!(level0.record_slot_count() > old_slots);
    let middle_edges: Vec<ZoneId> = level0
        .edges(middle)
        .iter()
        .map(|edge| edge.neighbor)
        .collect();
    assert!(!middle_edges.contains(&old));
    assert_eq!(middle_edges.first().copied(), Some(right));
    assert_eq!(middle_edges.last().copied(), Some(replacement));
    assert_eq!(
        level0.record(replacement).unwrap().parent,
        hierarchy.level(1).unwrap().zone_at(0, 0),
        "8x8 parent refresh links the replacement to the rebuilt coarse level"
    );
}

#[test]
fn single_open_area_one_zone() {
    let grid = grid_from_str(
        "
        .....
        .....
        .....
    ",
    );
    let (zm, adj) = land_zones(&grid);
    assert_eq!(zm.zone_count, 1);
    // All cells should be zone 1.
    for ry in 0..3u16 {
        for rx in 0..5u16 {
            assert_eq!(zm.zone_at(rx, ry, MovementLayer::Ground), 1);
        }
    }
    // No adjacency (only one zone).
    assert!(adj.neighbors_of(1).is_empty());
}

#[test]
fn zone_grid_hierarchy_accessors_clear_on_mutation() {
    let terrain = terrain_from_zone_classes(1, 1, &[zone_class::GROUND], &[0]);
    let grid = PathGrid::from_resolved_terrain(&terrain);
    let terrain_costs = BTreeMap::new();
    let mut zg = ZoneGrid::build_with_terrain(&grid, &terrain_costs, Some(&terrain), &[], 1, 1);

    let normal = zg.hierarchy_for(MovementZone::Normal).unwrap();
    let water = zg.hierarchy_for(MovementZone::Water).unwrap();
    assert!(std::ptr::eq(normal, water));
    assert!(zg.hierarchy_for(MovementZone::Invalid).is_none());

    assert!(zg.map_mut(MovementZone::Water).is_some());
    assert!(zg.hierarchy_for(MovementZone::Normal).is_none());
    assert!(zg.hierarchy_for(MovementZone::Water).is_none());

    zg.set_hierarchy(tiny_hierarchy());
    assert!(zg.adjacency_mut(MovementZone::Normal).is_some());
    assert!(zg.hierarchy_for(MovementZone::Water).is_none());

    zg.set_hierarchy(tiny_hierarchy());
    let sz = super::zone_hierarchy::SuperZoneMap::from_adjacency(
        zg.adjacency_for(MovementZone::Water).unwrap(),
        zg.map_for(MovementZone::Water).unwrap().zone_count,
    );
    zg.set_super_zone(MovementZone::Water, sz);
    assert!(zg.hierarchy_for(MovementZone::Normal).is_none());

    let rebuilt_terrain = terrain_from_zone_classes(3, 1, &[zone_class::GROUND; 3], &[0; 3]);
    let rebuilt_grid = PathGrid::from_resolved_terrain(&rebuilt_terrain);
    let rebuilt = ZoneGrid::build_with_terrain(
        &rebuilt_grid,
        &terrain_costs,
        Some(&rebuilt_terrain),
        &[],
        3,
        1,
    );
    let rebuilt_normal = rebuilt.hierarchy_for(MovementZone::Normal).unwrap();
    let rebuilt_water = rebuilt.hierarchy_for(MovementZone::Water).unwrap();
    assert!(std::ptr::eq(rebuilt_normal, rebuilt_water));
    let rebuilt_level0 = rebuilt_normal.level(0).unwrap();
    assert_eq!(rebuilt_level0.zone_count(), 2);
    assert_eq!(rebuilt_level0.zone_at(0, 0), 1);
    assert_eq!(rebuilt_level0.zone_at(2, 0), 2);
}

#[test]
fn wall_splits_into_two_zones() {
    let grid = grid_from_str(
        "
        ..#..
        ..#..
        ..#..
    ",
    );
    let (zm, _adj) = land_zones(&grid);
    assert_eq!(zm.zone_count, 2);
    // Left side should be zone 1, right side zone 2.
    let z_left = zm.zone_at(0, 0, MovementLayer::Ground);
    let z_right = zm.zone_at(3, 0, MovementLayer::Ground);
    assert_ne!(z_left, ZONE_INVALID);
    assert_ne!(z_right, ZONE_INVALID);
    assert_ne!(z_left, z_right);
    // Wall cells should be ZONE_INVALID.
    assert_eq!(zm.zone_at(2, 0, MovementLayer::Ground), ZONE_INVALID);
}

#[test]
fn blocked_cells_are_invalid() {
    let grid = grid_from_str(
        "
        .#.
        ###
        .#.
    ",
    );
    let (zm, _adj) = land_zones(&grid);
    // Each corner is isolated (diagonal would need both cardinals passable).
    // (0,0) is passable, (2,0) is passable, but they can't connect diagonally
    // through (1,0)=blocked and (0,1)=blocked.
    let z00 = zm.zone_at(0, 0, MovementLayer::Ground);
    let z20 = zm.zone_at(2, 0, MovementLayer::Ground);
    let z02 = zm.zone_at(0, 2, MovementLayer::Ground);
    let z22 = zm.zone_at(2, 2, MovementLayer::Ground);
    assert_ne!(z00, ZONE_INVALID);
    assert_ne!(z20, ZONE_INVALID);
    // All four corners should be different zones (isolated by wall).
    assert_ne!(z00, z20);
    assert_ne!(z00, z02);
    assert_ne!(z00, z22);
}

#[test]
fn diagonal_connectivity_requires_cardinal_passable() {
    // Two cells diagonally adjacent but one cardinal blocked → different zones.
    let grid = grid_from_str(
        "
        .#
        #.
    ",
    );
    let (zm, _adj) = land_zones(&grid);
    // (0,0) and (1,1) are diagonally adjacent but (1,0)=# and (0,1)=# block the diagonal.
    let z00 = zm.zone_at(0, 0, MovementLayer::Ground);
    let z11 = zm.zone_at(1, 1, MovementLayer::Ground);
    assert_ne!(z00, z11);
}

#[test]
fn diagonal_connectivity_with_both_cardinals() {
    // Two cells diagonally adjacent with both cardinals passable → same zone.
    let grid = grid_from_str(
        "
        ..
        ..
    ",
    );
    let (zm, _adj) = land_zones(&grid);
    assert_eq!(zm.zone_count, 1);
}

#[test]
fn adjacency_between_zones() {
    // Two zones separated by a gap that has adjacent cells.
    // Zones are adjacent when their cells are 8-connected neighbors.
    let grid = grid_from_str(
        "
        ..#..
        .....
        ..#..
    ",
    );
    let (zm, _adj) = land_zones(&grid);
    // The gap at (2,1) connects everything into one zone.
    assert_eq!(zm.zone_count, 1);

    // Now create a true split with adjacency:
    let grid2 = grid_from_str(
        "
        ...##
        ...##
        .....
        ##...
        ##...
    ",
    );
    let (zm2, _adj2) = land_zones(&grid2);
    // Check that zones exist and might be adjacent via the connecting corridor.
    assert!(zm2.zone_count >= 1);
}

#[test]
fn same_zone_check() {
    let grid = grid_from_str(
        "
        .....
        .....
    ",
    );
    let (zm, _adj) = land_zones(&grid);
    assert!(zm.same_zone((0, 0), (4, 1), MovementLayer::Ground));
}

#[test]
fn different_zones_same_zone_check() {
    let grid = grid_from_str(
        "
        ..#..
        ..#..
    ",
    );
    let (zm, _adj) = land_zones(&grid);
    assert!(!zm.same_zone((0, 0), (3, 0), MovementLayer::Ground));
}

#[test]
fn deterministic_zone_ids() {
    let grid = grid_from_str(
        "
        ..#..
        ..#..
        ..#..
    ",
    );
    let (zm1, _) = land_zones(&grid);
    let (zm2, _) = land_zones(&grid);
    // Zone IDs must be identical across runs.
    for ry in 0..3u16 {
        for rx in 0..5u16 {
            assert_eq!(
                zm1.zone_at(rx, ry, MovementLayer::Ground),
                zm2.zone_at(rx, ry, MovementLayer::Ground),
                "Non-deterministic zone at ({}, {})",
                rx,
                ry,
            );
        }
    }
}

#[test]
fn zone_grid_can_reach_same_zone() {
    let grid = grid_from_str(
        "
        .....
        .....
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 2);
    assert!(zg.can_reach(
        MovementZone::Normal,
        (0, 0),
        MovementLayer::Ground,
        (4, 1),
        MovementLayer::Ground,
    ));
}

#[test]
fn zone_grid_cannot_reach_disconnected() {
    let grid = grid_from_str(
        "
        ..#..
        ..#..
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 2);
    assert!(!zg.can_reach(
        MovementZone::Normal,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));
}

#[test]
fn zone_grid_fly_always_reachable() {
    let grid = grid_from_str(
        "
        ..#..
        ..#..
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 2);
    assert!(zg.can_reach(
        MovementZone::Fly,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));
}

#[test]
fn water_zone_grid_uses_resolved_land_type_directly() {
    let terrain = water_row_terrain(5);
    let grid = PathGrid::from_resolved_terrain(&terrain);
    let zg = ZoneGrid::build_with_terrain(&grid, &BTreeMap::new(), Some(&terrain), &[], 5, 1);
    assert!(zg.can_reach(
        MovementZone::Water,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));
}

#[test]
fn waterbeach_zone_grid_connects_beach_to_water_with_resolved_terrain() {
    let terrain = clear_beach_water_row_terrain();
    let grid = PathGrid::from_resolved_terrain(&terrain);
    let zg = ZoneGrid::build_with_terrain(&grid, &BTreeMap::new(), Some(&terrain), &[], 3, 1);
    assert!(zg.can_reach(
        MovementZone::WaterBeach,
        (1, 0),
        MovementLayer::Ground,
        (2, 0),
        MovementLayer::Ground,
    ));
    assert!(zg.can_reach(
        MovementZone::Amphibious,
        (0, 0),
        MovementLayer::Ground,
        (2, 0),
        MovementLayer::Ground,
    ));
}

#[test]
fn stock_low_bridge_auto_shell_zone_grid_uses_low_records_without_explicit_tubes() {
    let terrain = stock_low_bridge_auto_shell_terrain();
    assert!(
        terrain
            .tube_facts()
            .iter()
            .all(|tube| tube.source == TubeSource::AutoLowBridge && tube.path_len() == 0)
    );

    let bridge_state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 300);
    let records = bridge_state.endpoint_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].bridge_kind, BridgeRecordKind::Low);
    assert_eq!(records[0].endpoint_a, (0, 0));
    assert_eq!(records[0].endpoint_b, (4, 0));

    let grid = PathGrid::from_resolved_terrain(&terrain);
    let zg = ZoneGrid::build_with_terrain(&grid, &BTreeMap::new(), Some(&terrain), records, 5, 1);
    assert!(zg.can_reach(
        MovementZone::Normal,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));
    assert!(zg.can_reach(
        MovementZone::Infantry,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));

    let normal_map = zg.map_for(MovementZone::Normal).expect("normal zone map");
    assert_eq!(
        normal_map.zone_at(2, 0, MovementLayer::Bridge),
        ZONE_INVALID,
        "low records are all-active zone data, not high-bridge redirect records"
    );
}

// ---------------------------------------------------------------------------
// Height continuity tests
// ---------------------------------------------------------------------------

/// Build a PathGrid from a height array. All cells ground-walkable.
fn path_grid_from_heights(heights: &[u8], width: u16, height: u16) -> PathGrid {
    assert_eq!(heights.len(), width as usize * height as usize);
    let cells: Vec<PathCell> = heights
        .iter()
        .map(|&h| PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            bridge_structural: false,
            bridge_marker_0x80: false,
            transition: false,
            ground_level: h,
            bridge_deck_level: 0,
            slope_type: 0,
            tube_index: None,
            low_bridge_tube_cell: false,
        })
        .collect();
    PathGrid::from_cells(cells, width, height)
}

/// Build zones for Land category with height data (from PathGrid cells).
fn land_zones_with_height(grid: &PathGrid) -> (ZoneMap, ZoneAdjacency) {
    build_zone_map(
        grid,
        None,
        MovementZone::Normal,
        grid.width(),
        grid.height(),
    )
}

#[test]
fn height_cliff_splits_zone() {
    // All cells walkable, but heights jump from 0 to 3 — should split into 2 zones.
    let grid = path_grid_from_heights(&[0, 0, 0, 3, 3, 3], 6, 1);
    let (zm, _adj) = land_zones_with_height(&grid);
    assert_eq!(
        zm.zone_count, 2,
        "Height cliff (0→3) should split into two zones"
    );
    let z_left = zm.zone_at(0, 0, MovementLayer::Ground);
    let z_right = zm.zone_at(5, 0, MovementLayer::Ground);
    assert_ne!(z_left, ZONE_INVALID);
    assert_ne!(z_right, ZONE_INVALID);
    assert_ne!(z_left, z_right);
}

#[test]
fn height_ramp_stays_one_zone() {
    // Heights [0,1,2,3,4] — each adjacent pair differs by exactly 1.
    let grid = path_grid_from_heights(&[0, 1, 2, 3, 4], 5, 1);
    let (zm, _adj) = land_zones_with_height(&grid);
    assert_eq!(zm.zone_count, 1, "Gradual ramp (step=1) should be one zone");
}

#[test]
fn height_check_skipped_when_all_level_zero() {
    // When all cells have ground_level=0, heights don't split zones — all passable cells merge.
    let grid = grid_from_str("......");
    let (zm, _adj) = land_zones(&grid);
    assert_eq!(
        zm.zone_count, 1,
        "All level-zero cells should merge into one zone"
    );
}

#[test]
fn height_2d_plateau_isolated() {
    // 3x3 grid: center cell at height 5, rest at height 0.
    // Center should be isolated (h_diff > 1 in all directions).
    #[rustfmt::skip]
    let grid = path_grid_from_heights(&[
        0, 0, 0,
        0, 5, 0,
        0, 0, 0,
    ], 3, 3);
    let (zm, _adj) = land_zones_with_height(&grid);
    let z_corner = zm.zone_at(0, 0, MovementLayer::Ground);
    let z_center = zm.zone_at(1, 1, MovementLayer::Ground);
    assert_ne!(z_corner, ZONE_INVALID);
    assert_ne!(z_center, ZONE_INVALID);
    assert_ne!(
        z_corner, z_center,
        "Height-5 plateau should be isolated from height-0 surround"
    );
}

#[test]
fn height_step_of_two_splits() {
    // Heights [0, 2, 4] — each step is 2, exceeding the threshold of 1.
    let grid = path_grid_from_heights(&[0, 2, 4], 3, 1);
    let (zm, _adj) = land_zones_with_height(&grid);
    assert_eq!(zm.zone_count, 3, "Each cell is its own zone when step=2");
}

// ---------------------------------------------------------------------------
// Incremental zone update tests
// ---------------------------------------------------------------------------

/// Verify that incremental update produces correct reachability after blocking a cell.
#[test]
fn incremental_block_cell_splits_zone() {
    // 5x1 grid: all walkable → one zone.
    let grid = grid_from_str(".....");
    let mut zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 1);
    assert!(zg.can_reach(
        MovementZone::Normal,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));

    // Block center cell (2,0) → should split into two zones.
    let mut grid2 = grid.clone();
    grid2.set_blocked(2, 0, true);
    let changed = grid.diff_cells(&grid2).unwrap();
    assert_eq!(changed.len(), 1);

    let result = crate::sim::pathfinding::zone_incremental::try_incremental_update(
        &mut zg,
        &changed,
        &grid2,
        &BTreeMap::new(),
        None,
        &[],
    );
    assert!(result, "Incremental update should succeed");
    assert!(
        !zg.can_reach(
            MovementZone::Normal,
            (0, 0),
            MovementLayer::Ground,
            (4, 0),
            MovementLayer::Ground,
        ),
        "After blocking center, left and right should be disconnected"
    );
    // Left side still connected within itself.
    assert!(zg.can_reach(
        MovementZone::Normal,
        (0, 0),
        MovementLayer::Ground,
        (1, 0),
        MovementLayer::Ground,
    ));
    // Right side still connected within itself.
    assert!(zg.can_reach(
        MovementZone::Normal,
        (3, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));
}

/// Verify that unblocking a cell reconnects zones.
#[test]
fn incremental_unblock_cell_merges_zones() {
    // Start with wall in center.
    let grid1 = grid_from_str("..#..");
    let mut zg = ZoneGrid::build(&grid1, &BTreeMap::new(), 5, 1);
    assert!(!zg.can_reach(
        MovementZone::Normal,
        (0, 0),
        MovementLayer::Ground,
        (4, 0),
        MovementLayer::Ground,
    ));

    // Remove wall → should reconnect.
    let grid2 = grid_from_str(".....");
    let changed = grid1.diff_cells(&grid2).unwrap();
    assert_eq!(changed.len(), 1);

    let result = crate::sim::pathfinding::zone_incremental::try_incremental_update(
        &mut zg,
        &changed,
        &grid2,
        &BTreeMap::new(),
        None,
        &[],
    );
    assert!(result);
    assert!(
        zg.can_reach(
            MovementZone::Normal,
            (0, 0),
            MovementLayer::Ground,
            (4, 0),
            MovementLayer::Ground,
        ),
        "After removing wall, zones should reconnect"
    );
}

/// Large number of changed cells should trigger fallback (return false).
#[test]
fn incremental_fallback_on_large_change() {
    let grid = grid_from_str(".....");
    let mut zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 1);

    // Simulate > INCREMENTAL_THRESHOLD changed cells.
    let many_changes: Vec<(u16, u16)> = (0..201).map(|i| (i % 5, 0)).collect();
    let result = crate::sim::pathfinding::zone_incremental::try_incremental_update(
        &mut zg,
        &many_changes,
        &grid,
        &BTreeMap::new(),
        None,
        &[],
    );
    assert!(
        !result,
        "Should fall back to full rebuild on > threshold changes"
    );
}

/// Empty changeset is a no-op.
#[test]
fn incremental_no_change_is_noop() {
    let grid = grid_from_str(".....");
    let mut zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 1);
    let original_zone_count = zg.map_for(MovementZone::Normal).unwrap().zone_count;

    let result = crate::sim::pathfinding::zone_incremental::try_incremental_update(
        &mut zg,
        &[],
        &grid,
        &BTreeMap::new(),
        None,
        &[],
    );
    assert!(result);
    assert_eq!(
        zg.map_for(MovementZone::Normal).unwrap().zone_count,
        original_zone_count,
    );
}

#[test]
fn incremental_with_resolved_terrain_falls_back_to_full_rebuild_path() {
    let terrain = water_row_terrain(3);
    let grid = PathGrid::from_resolved_terrain(&terrain);
    let mut zg = ZoneGrid::build_with_terrain(&grid, &BTreeMap::new(), Some(&terrain), &[], 3, 1);
    let mut grid2 = grid.clone();
    grid2.set_blocked(1, 0, true);
    let changed = grid.diff_cells(&grid2).unwrap();

    let result = crate::sim::pathfinding::zone_incremental::try_incremental_update(
        &mut zg,
        &changed,
        &grid2,
        &BTreeMap::new(),
        Some(&terrain),
        &[],
    );
    assert!(!result);
}

#[test]
fn terrain_aware_incremental_update_requests_full_rebuild() {
    let terrain = water_row_terrain(5);
    let grid = PathGrid::from_resolved_terrain(&terrain);
    let mut zg = ZoneGrid::build_with_terrain(&grid, &BTreeMap::new(), Some(&terrain), &[], 5, 1);

    let result = crate::sim::pathfinding::zone_incremental::try_incremental_update(
        &mut zg,
        &[(0, 0)],
        &grid,
        &BTreeMap::new(),
        Some(&terrain),
        &[],
    );
    assert!(
        !result,
        "terrain-aware zoning should currently force a full rebuild on dynamic updates"
    );
}

#[test]
fn per_movement_zone_grids_are_separate() {
    // Verify that the ZoneCategory collapse is truly gone —
    // each MovementZone variant gets its own independent zone grid.
    let grid = grid_from_str(
        "
        .....
        .....
    ",
    );
    let zg = ZoneGrid::build(&grid, &BTreeMap::new(), 5, 2);
    // Normal and Crusher should each have their own zone map.
    assert!(
        zg.map_for(MovementZone::Normal).is_some(),
        "Normal should have a zone map"
    );
    assert!(
        zg.map_for(MovementZone::Crusher).is_some(),
        "Crusher should have a zone map"
    );
    assert!(
        zg.map_for(MovementZone::Infantry).is_some(),
        "Infantry should have a zone map"
    );
    assert!(
        zg.map_for(MovementZone::Water).is_some(),
        "Water should have a zone map"
    );
    // The binary rebuild loop covers all 13 matrix rows, including Fly.
    assert!(
        zg.map_for(MovementZone::Fly).is_some(),
        "Fly should have a zone map"
    );
    // All matrix-backed movement zones should have maps.
    for &mz in MovementZone::all_ground() {
        assert!(
            zg.map_for(mz).is_some(),
            "MovementZone {:?} should have a zone map",
            mz
        );
    }
}
