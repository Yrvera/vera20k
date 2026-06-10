//! Building placement tests — verifies foundation overlap detection, placement validity,
//! and per-owner placement pool management for the production system.

use std::collections::{BTreeMap, VecDeque};

use super::{
    BuildingPlacementError, ProductionCategory,
    cancel_last_for_owner, credits_for_owner, cycle_active_producer_for_owner_category,
    find_spawn_cell_for_owner, place_ready_building, placement_preview_for_owner,
    producer_candidates_for_owner_category, ready_buildings_for_owner, sell_building,
    tick_production,
};
use crate::map::bridge_facts::BRIDGE_FLAG_DESTROYED_OR_RAMP;
use crate::map::resolved_terrain::{RampDirection, ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::ini_parser::IniFile;
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

// Re-use test helpers from the main production_tests module.
use super::tests::{
    basic_multi_queue_rules, build_catalog_rules, factory_rules, placement_radius_rules,
    sell_rules, spawn_structure,
};

fn resolved_clear_grid_with_override(
    width: u16,
    height: u16,
    mut override_cell: impl FnMut(&mut ResolvedTerrainCell),
) -> ResolvedTerrainGrid {
    let mut cells = Vec::with_capacity((width as usize) * (height as usize));
    for ry in 0..height {
        for rx in 0..width {
            let mut cell = ResolvedTerrainCell {
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
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                allows_tiberium: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
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
            };
            override_cell(&mut cell);
            cells.push(cell);
        }
    }
    ResolvedTerrainGrid::from_cells(width, height, cells)
}

fn naval_yard_placement_rules() -> RuleSet {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=GAYARD\n\
         [GACNST]\n\
         Strength=1000\n\
         Armor=wood\n\
         Foundation=2x2\n\
         BaseNormal=yes\n\
         Adjacent=12\n\
         [GAYARD]\n\
         Strength=1500\n\
         Armor=concrete\n\
         Foundation=1x1\n\
         WaterBound=yes\n\
         Naval=yes\n\
         Adjacent=12\n",
    );
    RuleSet::from_ini(&ini).expect("naval yard placement rules should parse")
}

fn build_off_ally_rules() -> RuleSet {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=GAPOWR\n\
         [GACNST]\n\
         Strength=1000\n\
         Armor=wood\n\
         Foundation=2x2\n\
         BaseNormal=yes\n\
         EligibileForAllyBuilding=yes\n\
         [GAPOWR]\n\
         Strength=750\n\
         Armor=wood\n\
         Foundation=2x2\n\
         Adjacent=0\n",
    );
    RuleSet::from_ini(&ini).expect("BuildOffAlly placement rules should parse")
}

fn mark_allied(sim: &mut Simulation, a: &str, b: &str) {
    let a = a.to_ascii_uppercase();
    let b = b.to_ascii_uppercase();
    sim.house_alliances
        .entry(a.clone())
        .or_default()
        .insert(b.clone());
    sim.house_alliances.entry(b).or_default().insert(a);
}

fn ready_building(sim: &mut Simulation, owner: &str, type_id: &str) {
    let owner_id = sim.interner.intern(owner);
    let type_id = sim.interner.intern(type_id);
    sim.production
        .ready_by_owner
        .insert(owner_id, VecDeque::from([type_id]));
}

#[test]
fn completed_building_moves_into_ready_placement_pool() {
    let mut sim = Simulation::new();
    let rules = build_catalog_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gacnst = sim.interner.intern("GACNST");
    // P5d: arm the Building build directly in the registry (queue-of-record), then force it
    // to the completed-held state so `tick_production` moves it into the ready-placement pool.
    super::tests::arm_build_via(
        &mut sim,
        &rules,
        "Americans",
        "GACNST",
        ProductionCategory::Building,
        100,
        1,
    );
    assert!(sim
        .production
        .factory_shadow
        .test_arm_ready(americans, ProductionCategory::Building));

    let spawned = tick_production(&mut sim, &rules, &height_map, None, 700);
    assert!(!spawned, "completed building should wait for placement");
    assert!(sim.production.factory_shadow.is_empty());
    assert_eq!(
        ready_buildings_for_owner(&sim, &rules, "Americans")
            .into_iter()
            .map(|item| item.type_id)
            .collect::<Vec<_>>(),
        vec![gacnst]
    );
}

#[test]
fn place_ready_building_spawns_and_consumes_ready_item() {
    let mut sim = Simulation::new();
    let rules = build_catalog_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 18, 18);

    let americans = sim.interner.intern("Americans");
    let gacnst = sim.interner.intern("GACNST");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gacnst]));

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GACNST",
        20,
        20,
        Some(&grid),
        &height_map,
    ));
    assert!(ready_buildings_for_owner(&sim, &rules, "Americans").is_empty());

    let structures = sim
        .substrate.entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.owner)
                .eq_ignore_ascii_case("Americans")
                && sim
                    .interner
                    .resolve(e.type_ref)
                    .eq_ignore_ascii_case("GACNST")
                && e.position.rx == 20
                && e.position.ry == 20
                && e.category == crate::map::entities::EntityCategory::Structure
        })
        .count();
    assert_eq!(structures, 1);
}

#[test]
fn place_ready_building_accepts_clear_mixed_height_footprint() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let mut height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);

    for (cell, z) in [((12, 10), 0), ((13, 10), 1), ((12, 11), 2), ((13, 11), 3)] {
        height_map.insert(cell, z);
    }

    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));

    let preview = placement_preview_for_owner(
        &sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    )
    .expect("preview should exist");
    assert!(
        preview.valid,
        "mixed clear heights should not reject placement"
    );
    assert!(
        preview.cell_valid.iter().all(|valid| *valid),
        "all otherwise-clear mixed-height cells should be individually valid"
    );

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));

    assert!(sim.substrate.entities.values().any(|e| {
        sim.interner
            .resolve(e.type_ref)
            .eq_ignore_ascii_case("GAPOWR")
            && e.position.rx == 12
            && e.position.ry == 10
    }));
}

#[test]
fn place_ready_building_rejects_blocked_cell_inside_mixed_height_footprint() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let mut height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    grid.set_blocked(13, 11, true);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);

    for (cell, z) in [((12, 10), 0), ((13, 10), 1), ((12, 11), 2), ((13, 11), 3)] {
        height_map.insert(cell, z);
    }

    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));
    assert_eq!(
        ready_buildings_for_owner(&sim, &rules, "Americans").len(),
        1,
        "blocked placement must not consume the ready building"
    );
}

#[test]
fn refinery_placement_spawns_one_starter_harvester() {
    let mut sim = Simulation::new();
    let rules = build_catalog_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 18, 18);
    let americans = sim.interner.intern("Americans");
    let garefn = sim.interner.intern("GAREFN");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([garefn]));

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAREFN",
        20,
        20,
        Some(&grid),
        &height_map,
    ));

    let harvesters: Vec<(u16, u16)> = sim
        .substrate.entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.owner)
                .eq_ignore_ascii_case("Americans")
                && sim
                    .interner
                    .resolve(e.type_ref)
                    .eq_ignore_ascii_case("HARV")
                && e.category == crate::map::entities::EntityCategory::Unit
        })
        .map(|e| (e.position.rx, e.position.ry))
        .collect();
    assert_eq!(harvesters.len(), 1);
    let (harv_rx, harv_ry) = harvesters[0];
    assert!(
        !(20..=22).contains(&harv_rx) || !(20..=22).contains(&harv_ry),
        "starter harvester spawned inside refinery footprint at ({harv_rx},{harv_ry})"
    );
}

#[test]
fn modded_refinery_placement_uses_free_unit_from_rules() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=MODHARV\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=MODPROC\n\
         [GACNST]\n\
         Foundation=2x2\n\
         [MODPROC]\n\
         Refinery=yes\n\
         FreeUnit=MODHARV\n\
         Foundation=3x3\n\
         [MODHARV]\n\
         Harvester=yes\n\
         Dock=MODPROC\n\
         Speed=4\n",
    ))
    .expect("rules should parse");
    let mut sim = Simulation::new();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 18, 18);
    let americans = sim.interner.intern("Americans");
    let modproc = sim.interner.intern("MODPROC");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([modproc]));

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "MODPROC",
        20,
        20,
        Some(&grid),
        &height_map,
    ));

    let harvesters = sim
        .substrate.entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.owner)
                .eq_ignore_ascii_case("Americans")
                && sim
                    .interner
                    .resolve(e.type_ref)
                    .eq_ignore_ascii_case("MODHARV")
                && e.category == crate::map::entities::EntityCategory::Unit
        })
        .count();
    assert_eq!(harvesters, 1);
}

#[test]
fn refinery_without_free_unit_spawns_nothing() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=MODHARV\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=MODPROC\n\
         [GACNST]\n\
         Foundation=2x2\n\
         [MODPROC]\n\
         Refinery=yes\n\
         Foundation=3x3\n\
         [MODHARV]\n\
         Harvester=yes\n\
         Dock=MODPROC\n\
         Speed=4\n",
    ))
    .expect("rules should parse");
    let mut sim = Simulation::new();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 18, 18);
    let americans = sim.interner.intern("Americans");
    let modproc = sim.interner.intern("MODPROC");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([modproc]));

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "MODPROC",
        20,
        20,
        Some(&grid),
        &height_map,
    ));

    let harvesters = sim
        .substrate.entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.owner)
                .eq_ignore_ascii_case("Americans")
                && sim
                    .interner
                    .resolve(e.type_ref)
                    .eq_ignore_ascii_case("MODHARV")
                && e.category == crate::map::entities::EntityCategory::Unit
        })
        .count();
    assert_eq!(harvesters, 0);
}

#[test]
fn place_ready_building_rejects_blocked_or_overlapping_cells() {
    let mut sim = Simulation::new();
    let rules = build_catalog_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    grid.set_blocked(31, 31, true);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 30, 30);
    spawn_structure(&mut sim, 2, "Americans", "GACNST", 40, 40);

    let americans = sim.interner.intern("Americans");
    let gacnst = sim.interner.intern("GACNST");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gacnst, gacnst]));

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GACNST",
        31,
        31,
        Some(&grid),
        &height_map,
    ));
    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GACNST",
        40,
        40,
        Some(&grid),
        &height_map,
    ));
    assert_eq!(
        ready_buildings_for_owner(&sim, &rules, "Americans").len(),
        2,
        "invalid placement must not consume the ready building"
    );
}

#[test]
fn place_ready_building_requires_base_normal_provider_within_adjacent_range() {
    let rules = placement_radius_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    let mut sim = Simulation::new();
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));

    let mut far_sim = Simulation::new();
    spawn_structure(&mut far_sim, 1, "Americans", "GACNST", 10, 10);
    let far_americans = far_sim.interner.intern("Americans");
    let far_gapowr = far_sim.interner.intern("GAPOWR");
    far_sim
        .production
        .ready_by_owner
        .insert(far_americans, VecDeque::from([far_gapowr]));
    // GACNST has Adjacent=6 (default), foundation 2x2 at (10,10).
    // Expanded zone: max_x = 10+2-1+7 = 18, so (20,10) is out of range.
    assert!(!place_ready_building(
        &mut far_sim,
        &rules,
        "Americans",
        "GAPOWR",
        20,
        10,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn base_normal_false_structures_do_not_extend_build_area() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GAGAP", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn build_off_ally_enabled_accepts_allied_eligible_provider() {
    let mut sim = Simulation::new();
    let rules = build_off_ally_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Alliance", "GACNST", 10, 10);
    mark_allied(&mut sim, "Americans", "Alliance");
    ready_building(&mut sim, "Americans", "GAPOWR");

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn build_off_ally_disabled_rejects_allied_eligible_provider() {
    let mut sim = Simulation::new();
    let rules = build_off_ally_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    sim.session.game_options.build_off_ally = false;
    spawn_structure(&mut sim, 1, "Alliance", "GACNST", 10, 10);
    mark_allied(&mut sim, "Americans", "Alliance");
    ready_building(&mut sim, "Americans", "GAPOWR");

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn build_off_ally_requires_eligibile_for_ally_building() {
    let mut sim = Simulation::new();
    let rules = build_off_ally_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Alliance", "GAPOWR", 10, 10);
    mark_allied(&mut sim, "Americans", "Alliance");
    ready_building(&mut sim, "Americans", "GAPOWR");

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn build_off_ally_off_keeps_own_base_provider() {
    let mut sim = Simulation::new();
    let rules = build_off_ally_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    sim.session.game_options.build_off_ally = false;
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    ready_building(&mut sim, "Americans", "GAPOWR");

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn placement_preview_reports_out_of_build_area() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));

    let preview = placement_preview_for_owner(
        &sim,
        &rules,
        "Americans",
        "GAPOWR",
        20,
        20,
        Some(&grid),
        &BTreeMap::new(),
    )
    .expect("preview should exist");
    assert!(!preview.valid);
    assert_eq!(preview.reason, Some(BuildingPlacementError::OutOfBuildArea));
}

#[test]
fn placement_preview_reports_blocked_terrain() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let mut grid = PathGrid::new(64, 64);
    grid.set_blocked(12, 10, true);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));

    let preview = placement_preview_for_owner(
        &sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &BTreeMap::new(),
    )
    .expect("preview should exist");
    assert!(!preview.valid);
    assert_eq!(preview.reason, Some(BuildingPlacementError::BlockedTerrain));
}

#[test]
fn place_ready_building_rejects_bridge_deck_cells() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));
    sim.resolved_terrain = Some(resolved_clear_grid_with_override(64, 64, |cell| {
        if cell.rx == 12 && cell.ry == 10 {
            cell.build_blocked = true;
            cell.has_bridge_deck = true;
            cell.bridge_walkable = true;
            cell.bridge_transition = true;
            cell.bridge_deck_level = 3;
        }
    }));

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));

    let preview = placement_preview_for_owner(
        &sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    )
    .expect("preview should exist");
    assert_eq!(preview.reason, Some(BuildingPlacementError::BlockedTerrain));
}

#[test]
fn place_ready_building_rejects_bridge_0x400_marker_cells() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));
    sim.resolved_terrain = Some(resolved_clear_grid_with_override(64, 64, |cell| {
        if cell.rx == 12 && cell.ry == 10 {
            cell.bridge_facts.raw_flags |= BRIDGE_FLAG_DESTROYED_OR_RAMP;
        }
    }));

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));

    let preview = placement_preview_for_owner(
        &sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    )
    .expect("preview should exist");
    assert_eq!(preview.reason, Some(BuildingPlacementError::BlockedTerrain));
    assert!(
        !preview.cell_valid[0],
        "binary CellClass+0x140 bit 0x400 blocks placement even without live bridge deck flags"
    );
}

#[test]
fn place_ready_building_rejects_canonical_ramp_cells() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));
    sim.resolved_terrain = Some(resolved_clear_grid_with_override(64, 64, |cell| {
        if cell.rx == 12 && cell.ry == 10 {
            cell.has_ramp = true;
            cell.canonical_ramp = Some(RampDirection::West);
            cell.slope_type = 1;
            cell.ground_walk_blocked = false;
            cell.build_blocked = true;
        }
    }));

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));

    let preview = placement_preview_for_owner(
        &sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    )
    .expect("preview should exist");
    assert_eq!(preview.reason, Some(BuildingPlacementError::BlockedTerrain));
    assert!(
        sim.resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(12, 10))
            .is_some_and(|cell| !cell.ground_walk_blocked && cell.build_blocked),
        "canonical ramp fixture should stay movement-passable while rejecting placement"
    );
}

#[test]
fn place_ready_building_rejects_destroyed_bridge_over_blocked_ground() {
    let mut sim = Simulation::new();
    let rules = placement_radius_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));
    let resolved = resolved_clear_grid_with_override(64, 64, |cell| {
        if cell.rx == 12 && cell.ry == 10 {
            cell.ground_walk_blocked = true;
            cell.is_water = true;
            cell.base_build_blocked = true;
            cell.build_blocked = true;
            cell.has_bridge_deck = true;
            cell.bridge_walkable = true;
            cell.bridge_transition = true;
            cell.bridge_deck_level = 3;
        }
    });
    sim.bridge_state = Some(
        crate::sim::bridge_state::BridgeRuntimeState::from_resolved_terrain(&resolved, true, 5),
    );
    sim.resolved_terrain = Some(resolved);
    if let Some(state) = sim.bridge_state.as_mut() {
        // Direct mutation replaces the legacy `apply_damage`. The placement
        // gate reads `is_bridge_walkable`, which fails on `DamageState::Destroyed`.
        if let Some(c) = state.cell_mut(12, 10) {
            c.damage_state = crate::sim::bridge_state::DamageState::Destroyed;
        }
    }

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAPOWR",
        12,
        10,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn water_bound_building_rejects_beach_like_water_cells() {
    let mut sim = Simulation::new();
    let rules = naval_yard_placement_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gayard = sim.interner.intern("GAYARD");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gayard]));
    sim.resolved_terrain = Some(resolved_clear_grid_with_override(64, 64, |cell| {
        if cell.rx == 20 && cell.ry == 20 {
            cell.is_water = true;
            cell.land_type = 3; // Beach/shallow shore: amphibious OK, ships blocked.
            cell.terrain_class = TerrainClass::Water;
            cell.base_build_blocked = true;
            cell.build_blocked = true;
        }
    }));
    let grid =
        PathGrid::from_resolved_terrain(sim.resolved_terrain.as_ref().expect("resolved terrain"));

    assert!(!place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAYARD",
        20,
        20,
        Some(&grid),
        &height_map,
    ));

    let preview = placement_preview_for_owner(
        &sim,
        &rules,
        "Americans",
        "GAYARD",
        20,
        20,
        Some(&grid),
        &height_map,
    )
    .expect("preview should exist");
    assert_eq!(preview.reason, Some(BuildingPlacementError::BlockedTerrain));
}

#[test]
fn water_bound_building_accepts_true_ship_water_cells() {
    let mut sim = Simulation::new();
    let rules = naval_yard_placement_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    let americans = sim.interner.intern("Americans");
    let gayard = sim.interner.intern("GAYARD");
    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gayard]));
    sim.resolved_terrain = Some(resolved_clear_grid_with_override(64, 64, |cell| {
        if cell.rx == 20 && cell.ry == 20 {
            cell.is_water = true;
            cell.land_type = 4; // Water
            cell.terrain_class = TerrainClass::Water;
            cell.base_build_blocked = true;
            cell.build_blocked = true;
        }
    }));
    let grid =
        PathGrid::from_resolved_terrain(sim.resolved_terrain.as_ref().expect("resolved terrain"));

    assert!(place_ready_building(
        &mut sim,
        &rules,
        "Americans",
        "GAYARD",
        20,
        20,
        Some(&grid),
        &height_map,
    ));
}

#[test]
fn producer_candidates_are_sorted_by_stable_id() {
    let mut sim = Simulation::new();
    let rules = factory_rules();

    spawn_structure(&mut sim, 9, "Americans", "GAWEAP", 20, 20);
    spawn_structure(&mut sim, 3, "Americans", "GAWEAP", 10, 10);
    spawn_structure(&mut sim, 5, "Americans", "GAWEAP", 15, 15);

    let candidates = producer_candidates_for_owner_category(
        &sim.substrate.entities,
        &rules,
        "Americans",
        ProductionCategory::Vehicle,
        true,
        &sim.interner,
    );
    let ids: Vec<u64> = candidates.into_iter().map(|entry| entry.0).collect();
    assert_eq!(ids, vec![3, 5, 9]);
}

#[test]
fn cycle_active_producer_rotates_matching_factories() {
    let mut sim = Simulation::new();
    let rules = factory_rules();

    spawn_structure(&mut sim, 3, "Americans", "GAWEAP", 10, 10);
    spawn_structure(&mut sim, 5, "Americans", "GAWEAP", 15, 15);
    spawn_structure(&mut sim, 9, "Americans", "GAWEAP", 20, 20);

    assert!(cycle_active_producer_for_owner_category(
        &mut sim,
        &rules,
        "Americans",
        ProductionCategory::Vehicle,
    ));
    assert_eq!(
        sim.production
            .active_producer_by_owner
            .get(&sim.interner.intern("Americans"))
            .and_then(|categories| categories.get(&ProductionCategory::Vehicle))
            .copied(),
        Some(3)
    );
    assert!(cycle_active_producer_for_owner_category(
        &mut sim,
        &rules,
        "Americans",
        ProductionCategory::Vehicle,
    ));
    assert_eq!(
        sim.production
            .active_producer_by_owner
            .get(&sim.interner.intern("Americans"))
            .and_then(|categories| categories.get(&ProductionCategory::Vehicle))
            .copied(),
        Some(5)
    );
}

#[test]
fn blocked_active_war_factory_does_not_spawn_from_second_factory() {
    let mut sim = Simulation::new();
    let rules = factory_rules();
    let mut grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GAWEAP", 10, 10);
    spawn_structure(&mut sim, 2, "Americans", "GAWEAP", 30, 30);
    let americans = sim.interner.intern("Americans");
    sim.production.active_producer_by_owner.insert(
        americans,
        BTreeMap::from([(ProductionCategory::Vehicle, 1)]),
    );

    grid.set_blocked(12, 11, true);

    let spawn = find_spawn_cell_for_owner(
        &mut sim,
        &rules,
        "Americans",
        ObjectCategory::Vehicle,
        Some(&grid),
        false,
    );

    assert!(
        spawn.is_none(),
        "blocked active war factory must not route the completed vehicle through the second factory, got {:?}",
        spawn
    );
}

#[test]
fn stock_war_factory_initial_exit_has_no_nearest_cell_fallback() {
    let mut sim = Simulation::new();
    let rules = factory_rules();
    let mut grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GAWEAP", 10, 10);
    grid.set_blocked(12, 11, true);

    let spawn = find_spawn_cell_for_owner(
        &mut sim,
        &rules,
        "Americans",
        ObjectCategory::Vehicle,
        Some(&grid),
        false,
    );

    assert!(
        spawn.is_none(),
        "blocked ExitCoord must fail initial war-factory delivery instead of probing neighboring cells, got {:?}",
        spawn
    );
}

#[test]
fn stock_war_factory_clear_exitcoord_succeeds() {
    let mut sim = Simulation::new();
    let rules = factory_rules();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GAWEAP", 10, 10);

    let spawn = find_spawn_cell_for_owner(
        &mut sim,
        &rules,
        "Americans",
        ObjectCategory::Vehicle,
        Some(&grid),
        false,
    )
    .expect("clear ExitCoord should accept the stock war-factory spawn cell");

    assert_eq!(
        spawn,
        (12, 11),
        "stock land war factory initial spawn uses ExitCoord=512,256,0"
    );
}

#[test]
fn spawn_routing_prefers_active_producer_when_available() {
    let mut sim = Simulation::new();
    let rules = factory_rules();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 3, "Americans", "GAWEAP", 10, 10);
    spawn_structure(&mut sim, 5, "Americans", "GAWEAP", 30, 30);
    let americans = sim.interner.intern("Americans");
    sim.production.active_producer_by_owner.insert(
        americans,
        BTreeMap::from([(ProductionCategory::Vehicle, 5)]),
    );

    let spawn = find_spawn_cell_for_owner(
        &mut sim,
        &rules,
        "Americans",
        ObjectCategory::Vehicle,
        Some(&grid),
        false,
    )
    .expect("active producer should provide a valid exit");

    assert!(
        spawn.0 >= 31 && spawn.0 <= 33 && spawn.1 >= 30 && spawn.1 <= 32,
        "spawn should prefer the active war factory, got {:?}",
        spawn
    );
}

#[test]
fn cancel_last_for_owner_cancels_latest_item_across_categories() {
    let mut sim = Simulation::new();
    let rules = basic_multi_queue_rules();

    *super::credits_entry_for_owner(&mut sim, "Americans") = 1000;
    let americans = sim.interner.intern("Americans");
    // P5d: arm two registry builds (E1 order 1, MTNK order 2 = the latest). No upfront
    // charge, so credits stay 1000 until the cancel refund.
    super::tests::arm_build_via(
        &mut sim,
        &rules,
        "Americans",
        "E1",
        ProductionCategory::Infantry,
        100,
        1,
    );
    super::tests::arm_build_via(
        &mut sim,
        &rules,
        "Americans",
        "MTNK",
        ProductionCategory::Vehicle,
        100,
        2,
    );

    // Simulate a partly-charged MTNK (the latest item) so the abandon refunds its SPENT
    // portion (700-300=400), not the full cost — the legacy full-refund of a partly-charged
    // build is the retired DRIFT.
    {
        let f = sim
            .production
            .factory_shadow
            .test_factory_mut(americans, ProductionCategory::Vehicle)
            .expect("vehicle factory armed");
        f.progress = 20;
        f.balance = 300;
        f.original_balance = 700;
    }

    let canceled = cancel_last_for_owner(&mut sim, &rules, "Americans");
    assert!(canceled);
    assert_eq!(
        credits_for_owner(&sim, "Americans"),
        1400,
        "partial refund of the spent portion (700-300=400), not the full cost"
    );

    // The latest (MTNK / Vehicle) build is cancelled + pruned; the Infantry build remains.
    assert!(
        sim.production
            .factory_shadow
            .view(americans, ProductionCategory::Infantry)
            .is_some(),
        "the Infantry build remains"
    );
    assert!(
        sim.production
            .factory_shadow
            .view(americans, ProductionCategory::Vehicle)
            .is_none(),
        "the cancelled Vehicle build is pruned"
    );
}

#[test]
fn sell_building_refunds_half_current_value_and_ejects_allied_infantry() {
    let mut sim = Simulation::new();
    let rules = sell_rules();
    *super::credits_entry_for_owner(&mut sim, "Americans") = 1000;

    // Use spawn_structure for dual-write, then reduce health for the test.
    spawn_structure(&mut sim, 1, "Americans", "GAPOWR", 20, 20);
    if let Some(ge) = sim.substrate.entities.get_mut(1) {
        ge.health = Health {
            current: 375,
            max: 750,
        };
        ge.mark_live_contact_with(99);
    }
    let mut peer = GameEntity::test_default(99, "MTNK", "Americans", 22, 20);
    peer.owner = sim.interner.intern("Americans");
    peer.type_ref = sim.interner.intern("MTNK");
    peer.mark_live_contact_with(1);
    sim.substrate.entities.insert(peer);

    assert!(sell_building(&mut sim, &rules, 1));
    assert_eq!(credits_for_owner(&sim, "Americans"), 1200);

    let survivors: Vec<(String, u16, u16)> = sim
        .substrate.entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.owner)
                .eq_ignore_ascii_case("Americans")
                && sim.interner.resolve(e.type_ref).eq_ignore_ascii_case("E1")
        })
        .map(|e| ("E1".to_string(), e.position.rx, e.position.ry))
        .collect();
    // RA2 formula: refund = 800 * 50% * 50% = 200, survivors = 200 / 500 = 0.
    // Cheap Allied buildings at half health don't eject survivors.
    assert_eq!(
        survivors.len(),
        0,
        "800-cost Allied building at half health: refund 200 / divisor 500 = 0 survivors"
    );
    // Deferred-delete: sell_building enqueues; drain at end-of-tick to free the slot.
    sim.flush_pending_delete();
    assert!(
        !sim.substrate.entities.contains(1),
        "sold building should be removed from the store"
    );
    assert!(
        !sim.substrate.entities.get(99).unwrap().has_live_contact_with(1),
        "selling a building should clear peer radio contacts to it"
    );
}

#[test]
fn sell_building_uses_owner_appropriate_survivor_type_and_caps_count() {
    let mut sim = Simulation::new();
    let rules = sell_rules();
    // Soviet house: side_index=1 so the sell system picks E2 survivor type.
    let russians_key = sim.interner.intern("RUSSIANS");
    let russians_display = sim.interner.intern("Russians");
    sim.houses.insert(
        russians_key,
        crate::sim::house_state::HouseState::new(russians_display, 1, None, false, 1000, 10),
    );

    spawn_structure(&mut sim, 2, "Russians", "NAHAND", 30, 30);
    if let Some(ge) = sim.substrate.entities.get_mut(2) {
        ge.health = Health {
            current: 500,
            max: 500,
        };
    }

    assert!(sell_building(&mut sim, &rules, 2));
    assert_eq!(credits_for_owner(&sim, "Russians"), 1250);

    let conscripts = sim
        .substrate.entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.owner)
                .eq_ignore_ascii_case("Russians")
                && sim.interner.resolve(e.type_ref).eq_ignore_ascii_case("E2")
        })
        .count();
    // RA2 formula: refund = 500 * 50% * 100% = 250, survivors = 250 / 250 = 1.
    assert_eq!(
        conscripts, 1,
        "500-cost Soviet building at full health: refund 250 / divisor 250 = 1 survivor"
    );
}

#[test]
#[ignore = "WIP: captured-civilian sell-revert not yet landed"]
fn sell_captured_civilian_ejects_reverts_and_keeps_building() {
    use crate::sim::passenger::{PassengerCargo, PassengerRole};
    let mut sim = Simulation::new();
    let rules = sell_rules();
    *super::credits_entry_for_owner(&mut sim, "Americans") = 1000;

    // Spawn a CanBeOccupied building owned by Americans, with
    // garrison_original_owner = Some(Neutral).
    spawn_structure(&mut sim, 10, "Americans", "CAGAS01", 20, 20);
    let neutral_id = sim.interner.intern("Neutral");
    if let Some(t) = sim.substrate.entities.get_mut(10) {
        t.garrison_original_owner = Some(neutral_id);
        t.passenger_role = PassengerRole::Transport {
            cargo: PassengerCargo::new(5, 1),
        };
    }
    // Two occupants inside the cargo.
    let amer_id = sim.interner.intern("Americans");
    let e1_id = sim.interner.intern("E1");
    for &pid in &[11u64, 12u64] {
        let mut pax =
            crate::sim::game_entity::GameEntity::test_default(pid, "E1", "Americans", 19, 20);
        pax.owner = amer_id;
        pax.type_ref = e1_id;
        pax.passenger_role = PassengerRole::Inside { transport_id: 10 };
        sim.substrate.entities.insert(pax);
    }
    if let Some(t) = sim.substrate.entities.get_mut(10) {
        if let Some(c) = t.passenger_role.cargo_mut() {
            c.board(11, 1);
            c.board(12, 1);
        }
    }

    assert!(sell_building(&mut sim, &rules, 10));

    // Building still in store, owner reverted, cargo cleared.
    let bldg = sim.substrate.entities.get(10).expect("building should still exist");
    assert_eq!(sim.interner.resolve(bldg.owner), "Neutral");
    assert!(
        bldg.garrison_original_owner.is_none(),
        "original_owner should have been consumed"
    );
    let cargo = bldg.passenger_role.cargo().expect("cargo");
    assert!(cargo.is_empty(), "cargo should be cleared");

    // Both occupants alive on the map, role=None, not dying.
    for &pid in &[11u64, 12u64] {
        let pax = sim.substrate.entities.get(pid).expect("occupant exists");
        assert!(!pax.dying, "occupant {pid} should not be dying");
        assert!(pax.health.current > 0, "occupant {pid} should be alive");
        assert!(
            matches!(pax.passenger_role, PassengerRole::None),
            "occupant {pid} role should be None"
        );
    }

    // No refund credited.
    assert_eq!(
        credits_for_owner(&sim, "Americans"),
        1000,
        "captured-civilian sell pays no refund"
    );
}

#[test]
#[ignore = "WIP: captured-civilian sell-revert not yet landed"]
fn sell_captured_civilian_emits_structure_abandoned_with_pre_revert_owner() {
    use crate::sim::passenger::{PassengerCargo, PassengerRole};
    use crate::sim::world::SimSoundEvent;
    let mut sim = Simulation::new();
    let rules = sell_rules();
    spawn_structure(&mut sim, 20, "Americans", "CAGAS01", 30, 30);
    let neutral_id = sim.interner.intern("Neutral");
    let amer_id = sim.interner.intern("Americans");
    let e1_id = sim.interner.intern("E1");
    if let Some(t) = sim.substrate.entities.get_mut(20) {
        t.garrison_original_owner = Some(neutral_id);
        t.passenger_role = PassengerRole::Transport {
            cargo: PassengerCargo::new(5, 1),
        };
    }
    let mut pax = crate::sim::game_entity::GameEntity::test_default(21, "E1", "Americans", 29, 30);
    pax.owner = amer_id;
    pax.type_ref = e1_id;
    pax.passenger_role = PassengerRole::Inside { transport_id: 20 };
    sim.substrate.entities.insert(pax);
    if let Some(t) = sim.substrate.entities.get_mut(20) {
        if let Some(c) = t.passenger_role.cargo_mut() {
            c.board(21, 1);
        }
    }

    assert!(sell_building(&mut sim, &rules, 20));

    let mut found = false;
    for evt in &sim.sound_events {
        if let SimSoundEvent::StructureAbandoned { owner } = evt {
            assert_eq!(
                sim.interner.resolve(*owner),
                "Americans",
                "StructureAbandoned should carry pre-revert owner (Americans), not post-revert civilian"
            );
            found = true;
        }
    }
    assert!(
        found,
        "expected StructureAbandoned event after captured-civilian sell"
    );
}

#[test]
fn sell_player_built_garrisoned_building_demolishes_and_ejects_alive() {
    use crate::sim::passenger::{PassengerCargo, PassengerRole};
    let mut sim = Simulation::new();
    let rules = sell_rules();
    *super::credits_entry_for_owner(&mut sim, "Americans") = 0;

    // Spawn a CanBeOccupied building OWNED by Americans with NO original_owner
    // (player-built, not captured). CAGAS01 in sell_rules has Cost=0 so the
    // refund is 0 — this test pins the demolition path (entities.remove fired)
    // and the alive-eject of the occupant, not the refund magnitude.
    spawn_structure(&mut sim, 30, "Americans", "CAGAS01", 40, 40);
    let amer_id = sim.interner.intern("Americans");
    let e1_id = sim.interner.intern("E1");
    if let Some(t) = sim.substrate.entities.get_mut(30) {
        // garrison_original_owner stays None — player-built path.
        t.passenger_role = PassengerRole::Transport {
            cargo: PassengerCargo::new(5, 1),
        };
    }
    let mut pax = crate::sim::game_entity::GameEntity::test_default(31, "E1", "Americans", 39, 40);
    pax.owner = amer_id;
    pax.type_ref = e1_id;
    pax.passenger_role = PassengerRole::Inside { transport_id: 30 };
    sim.substrate.entities.insert(pax);
    if let Some(t) = sim.substrate.entities.get_mut(30) {
        if let Some(c) = t.passenger_role.cargo_mut() {
            c.board(31, 1);
        }
    }

    assert!(sell_building(&mut sim, &rules, 30));

    // Building removed (deferred-delete: drain at end-of-tick to free the slot).
    sim.flush_pending_delete();
    assert!(
        !sim.substrate.entities.contains(30),
        "player-built garrison should be demolished on sell"
    );
    // Occupant placed on the map alive.
    let pax = sim.substrate.entities.get(31).expect("occupant exists");
    assert!(!pax.dying, "occupant should not be dying");
    assert!(pax.health.current > 0, "occupant should be alive");
    assert!(
        matches!(pax.passenger_role, PassengerRole::None),
        "occupant role should be None"
    );
}
