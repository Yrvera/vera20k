//! Building placement tests — verifies foundation overlap detection, placement validity,
//! and per-owner placement pool management for the production system.

use std::collections::{BTreeMap, VecDeque};

use super::{
    BuildingPlacementError, ProductionCategory, cancel_last_for_owner, credits_for_owner,
    cycle_active_producer_for_owner_category, find_spawn_cell_for_owner, foundation_dimensions,
    place_ready_building, placement_preview_for_owner, producer_candidates_for_owner_category,
    ready_buildings_for_owner, sell_building, tick_production,
};
use crate::map::bridge_facts::BRIDGE_FLAG_DESTROYED_OR_RAMP;
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::{
    RampDirection, ResolvedTerrainCell, ResolvedTerrainGrid, zone_class,
};
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::{BuildingUp, Health};
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::MissionType;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::power_system::has_active_radar;
use crate::sim::world::Simulation;

// Re-use test helpers from the main production_tests module.
use super::tests::{
    basic_multi_queue_rules, build_catalog_rules, factory_rules, placement_radius_rules,
    sell_rules, spawn_structure,
};

fn stock_refinery_completion_rules() -> RuleSet {
    let mut rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=CMIN\n\
         1=HARV\n\
         2=BLOCKER\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=NACNST\n\
         2=GAREFN\n\
         3=NAREFN\n\
         [GACNST]\n\
         Factory=BuildingType\n\
         [NACNST]\n\
         Factory=BuildingType\n\
         [GAREFN]\n\
         Refinery=yes\n\
         FreeUnit=CMIN\n\
         [NAREFN]\n\
         Refinery=yes\n\
         FreeUnit=HARV\n\
         [CMIN]\n\
         Harvester=yes\n\
         Dock=GAREFN\n\
         Cost=1400\n\
         Speed=4\n\
         Storage=20\n\
         [HARV]\n\
         Harvester=yes\n\
         Dock=NAREFN\n\
         Cost=1400\n\
         Speed=4\n\
         Storage=20\n\
         [BLOCKER]\n\
         Cost=1\n\
         Speed=0\n",
    ))
    .expect("stock refinery completion rules should parse");
    let art = ArtRegistry::from_ini(&IniFile::from_str(
        "[GACNST]\n\
         Foundation=4x4\n\
         [NACNST]\n\
         Foundation=4x4\n\
         [GAREFN]\n\
         Foundation=4x3\n\
         [NAREFN]\n\
         Foundation=4x3\n",
    ));
    rules.merge_art_data(&art);
    rules
}

fn ready_and_place(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
    rx: u16,
    ry: u16,
    path_grid: &PathGrid,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> u64 {
    let owner_id = sim.interner.intern(owner);
    let type_ref = sim.interner.intern(type_id);
    sim.production
        .ready_by_owner
        .entry(owner_id)
        .or_default()
        .push_back(type_ref);
    assert!(place_ready_building(
        sim,
        rules,
        owner,
        type_id,
        rx,
        ry,
        Some(path_grid),
        height_map,
    ));
    sim.substrate
        .entities
        .values()
        .find(|entity| {
            entity.owner == owner_id
                && entity.type_ref == type_ref
                && entity.position.rx == rx
                && entity.position.ry == ry
                && entity.category == EntityCategory::Structure
        })
        .map(|entity| entity.stable_id)
        .expect("placed building should exist")
}

fn set_ticks_until_completion(sim: &mut Simulation, stable_id: u64, ticks: u16) {
    assert!(ticks > 0);
    let building_up = sim
        .substrate
        .entities
        .get_mut(stable_id)
        .and_then(|entity| entity.building_up.as_mut())
        .expect("placed building should have BuildingUp");
    assert!(ticks <= building_up.total_ticks);
    building_up.elapsed_ticks = building_up.total_ticks - ticks;
}

fn block_building_foundation(
    path_grid: &mut PathGrid,
    rules: &RuleSet,
    type_id: &str,
    rx: u16,
    ry: u16,
) {
    let foundation = &rules
        .object(type_id)
        .expect("building rules should exist")
        .foundation;
    let (width, height) = foundation_dimensions(foundation);
    for y in ry..ry + height {
        for x in rx..rx + width {
            path_grid.set_blocked(x, y, true);
        }
    }
}

fn unit_ids(sim: &Simulation, owner: &str, type_id: &str) -> Vec<u64> {
    sim.substrate
        .entities
        .values()
        .filter(|entity| {
            entity.category == EntityCategory::Unit
                && sim
                    .interner
                    .resolve(entity.owner)
                    .eq_ignore_ascii_case(owner)
                && sim
                    .interner
                    .resolve(entity.type_ref)
                    .eq_ignore_ascii_case(type_id)
        })
        .map(|entity| entity.stable_id)
        .collect()
}

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

fn ground_occupant_placement_rules() -> RuleSet {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         0=E1\n\
         [VehicleTypes]\n\
         0=MTNK\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=GAPOWR\n\
         2=GAWALL\n\
         [E1]\n\
         Strength=100\n\
         Armor=flak\n\
         [MTNK]\n\
         Strength=300\n\
         Armor=heavy\n\
         [GACNST]\n\
         Strength=1000\n\
         Armor=wood\n\
         Foundation=2x2\n\
         BaseNormal=yes\n\
         [GAPOWR]\n\
         Strength=750\n\
         Armor=wood\n\
         Foundation=2x2\n\
         Adjacent=0\n\
         [GAWALL]\n\
         Strength=300\n\
         Armor=concrete\n\
         Foundation=1x1\n\
         Adjacent=0\n\
         Wall=yes\n",
    );
    RuleSet::from_ini(&ini).expect("ground-occupant placement rules should parse")
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

fn stock_power_contract_rules() -> RuleSet {
    let fixture = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=GAPOWR\n\
         2=AMRADR\n\
         [GACNST]\n\
         Strength=1000\n\
         Armor=concrete\n\
         Adjacent=2\n\
         Power=0\n\
         [GAPOWR]\n\
         BuildCat=Power\n\
         Strength=750\n\
         Armor=wood\n\
         Adjacent=2\n\
         Power=200\n\
         [AMRADR]\n\
         BuildCat=Tech\n\
         Strength=600\n\
         Armor=steel\n\
         Adjacent=2\n\
         Power=-50\n\
         Radar=yes\n",
    );
    RuleSet::from_ini(&fixture).expect("stock power-contract fixture should parse")
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
    assert!(
        sim.production
            .factory_shadow
            .test_arm_ready(americans, ProductionCategory::Building)
    );

    let spawned = tick_production(&mut sim, &rules, &height_map, None);
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
        .substrate
        .entities
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
fn stock_gapowr_placement_restores_power_and_radar_during_buildup() {
    let mut sim = Simulation::new();
    let rules = stock_power_contract_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
    spawn_structure(&mut sim, 2, "Americans", "AMRADR", 10, 14);

    let americans = sim.interner.intern("Americans");
    let gapowr = sim.interner.intern("GAPOWR");

    sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    let outage = sim
        .power_states
        .get(&americans)
        .expect("stock radar house should have derived power state");
    assert_eq!(outage.total_output, 0);
    assert_eq!(outage.total_drain, 50);
    assert!(outage.is_low_power);
    assert!(
        !has_active_radar(
            &sim.substrate.entities,
            &sim.power_states,
            &rules,
            americans,
            &sim.interner,
        ),
        "stock American radar must be offline during house low power"
    );

    sim.production
        .ready_by_owner
        .insert(americans, VecDeque::from([gapowr]));
    let place = CommandEnvelope::new(
        americans,
        sim.session.tick + 1,
        Command::PlaceReadyBuilding {
            owner: americans,
            type_id: gapowr,
            rx: 12,
            ry: 10,
        },
    );

    let tick = sim.advance_tick(&[place], Some(&rules), &height_map, Some(&grid), None, 67);
    assert_eq!(tick.executed_commands, 1);

    let placed = sim
        .substrate
        .entities
        .values()
        .find(|entity| {
            entity.owner == americans
                && entity.type_ref == gapowr
                && entity.position.rx == 12
                && entity.position.ry == 10
        })
        .expect("production command should place stock GAPOWR");
    assert!(
        placed.building_up.is_some(),
        "power recovery must occur while the placement buildup is still active"
    );

    // Native Unlimbo requests a house power reassessment, but exact event-vs-
    // House update ordering within the placement command frame remains
    // unverified. Advance to the next guaranteed assessment while buildup is
    // still active rather than certifying an unsupported one-frame claim.
    sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(
        sim.substrate.entities.values().any(|entity| {
            entity.owner == americans && entity.type_ref == gapowr && entity.building_up.is_some()
        }),
        "GAPOWR must still be in its visible buildup during reassessment"
    );

    let recovered = sim
        .power_states
        .get(&americans)
        .expect("post-placement tick should reassess stock power");
    assert_eq!(recovered.total_output, 200);
    assert_eq!(recovered.total_drain, 50);
    assert!(!recovered.is_low_power);
    assert!(
        has_active_radar(
            &sim.substrate.entities,
            &sim.power_states,
            &rules,
            americans,
            &sim.interner,
        ),
        "existing stock American radar should recover while GAPOWR is building up"
    );
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
fn stock_refinery_free_unit_spawns_on_building_up_completion_once() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, &rules, "GAREFN", 20, 20);
    set_ticks_until_completion(&mut sim, refinery_id, 2);

    assert!(unit_ids(&sim, "Americans", "CMIN").is_empty());
    let before_completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(!before_completion.spawned_entities);
    assert!(
        sim.substrate
            .entities
            .get(refinery_id)
            .is_some_and(|entity| entity.building_up.is_some())
    );
    assert!(unit_ids(&sim, "Americans", "CMIN").is_empty());

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(completion.spawned_entities);
    assert!(
        sim.substrate
            .entities
            .get(refinery_id)
            .is_some_and(|entity| entity.building_up.is_none())
    );
    assert_eq!(unit_ids(&sim, "Americans", "CMIN").len(), 1);

    let later = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(!later.spawned_entities);
    assert_eq!(unit_ids(&sim, "Americans", "CMIN").len(), 1);
}

#[test]
fn stock_4x3_refinery_free_unit_uses_native_primary_cell() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, &rules, "GAREFN", 20, 20);
    set_ticks_until_completion(&mut sim, refinery_id, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(completion.spawned_entities);
    let miner = sim
        .substrate
        .entities
        .values()
        .find(|entity| {
            entity.category == EntityCategory::Unit
                && entity.position.rx == 22
                && entity.position.ry == 22
                && sim
                    .interner
                    .resolve(entity.owner)
                    .eq_ignore_ascii_case("Americans")
                && sim
                    .interner
                    .resolve(entity.type_ref)
                    .eq_ignore_ascii_case("CMIN")
        })
        .expect("stock Allied FreeUnit should use the 4x3 native primary cell");
    assert_eq!(miner.facing, 0xC0);
    assert_eq!(miner.mission.current().known(), Some(MissionType::Harvest));
    assert!(
        !sim.substrate.entities.values().any(|entity| {
            entity.category == EntityCategory::Unit
                && entity.position.rx == 22
                && entity.position.ry == 23
                && sim
                    .interner
                    .resolve(entity.type_ref)
                    .eq_ignore_ascii_case("CMIN")
        }),
        "FreeUnit must not use the old south-edge heuristic"
    );
}

#[test]
fn occupied_primary_bay_uses_one_fallback_without_overlap() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, &rules, "GAREFN", 20, 20);
    let blocker_id = sim
        .spawn_object("BLOCKER", "Russians", 22, 22, 0, &rules, &height_map)
        .expect("dynamic primary blocker should spawn");
    assert!(sim.substrate.occupancy.contains_entity(22, 22, refinery_id));
    assert!(sim.substrate.occupancy.contains_entity(22, 22, blocker_id));
    set_ticks_until_completion(&mut sim, refinery_id, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(completion.spawned_entities);
    let cmin_ids = unit_ids(&sim, "Americans", "CMIN");
    assert_eq!(
        cmin_ids.len(),
        1,
        "fallback must reuse one constructed unit"
    );
    let miner = sim
        .substrate
        .entities
        .get(cmin_ids[0])
        .expect("fallback miner should remain alive");
    assert_ne!(
        (miner.position.rx, miner.position.ry),
        (22, 22),
        "FreeUnit must not overlap an independent primary-bay blocker"
    );
    assert_eq!(miner.facing, 0xA0);
    assert!(sim.substrate.occupancy.contains_entity(22, 22, blocker_id));
    assert!(
        !sim.substrate
            .occupancy
            .contains_entity(22, 22, miner.stable_id),
        "failed primary Reveal must not mark the miner into occupancy"
    );
}

#[test]
fn occupied_first_fallback_retries_same_unit_at_second_candidate() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, &rules, "GAREFN", 20, 20);
    sim.spawn_object("BLOCKER", "Russians", 22, 22, 0, &rules, &height_map)
        .expect("dynamic primary blocker should spawn");
    sim.spawn_object("BLOCKER", "Russians", 19, 19, 0, &rules, &height_map)
        .expect("first compatibility fallback blocker should spawn");
    set_ticks_until_completion(&mut sim, refinery_id, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(completion.spawned_entities);
    let cmin_ids = unit_ids(&sim, "Americans", "CMIN");
    assert_eq!(
        cmin_ids.len(),
        1,
        "fallback retries must not allocate another FreeUnit"
    );
    let miner = sim
        .substrate
        .entities
        .get(cmin_ids[0])
        .expect("second-fallback miner should remain alive");
    assert_eq!((miner.position.rx, miner.position.ry), (20, 19));
    assert_eq!(miner.facing, 0xA0);
    assert!(
        !sim.substrate
            .occupancy
            .contains_entity(19, 19, miner.stable_id),
        "failed first fallback must leave no occupancy residue"
    );
}

#[test]
fn free_unit_total_placement_failure_refunds_once_and_leaves_no_entity() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    sim.spawn_object("BLOCKER", "Russians", 22, 22, 0, &rules, &height_map)
        .expect("dynamic primary blocker should spawn");
    for ry in 0..64 {
        for rx in 0..64 {
            grid.set_blocked(rx, ry, true);
        }
    }
    *super::credits_entry_for_owner(&mut sim, "Americans") = 100;
    let americans = sim.interner.get("Americans").expect("owner should exist");
    let owned_units_before = sim
        .houses
        .get(&americans)
        .expect("house should exist")
        .owned_unit_count;
    set_ticks_until_completion(&mut sim, refinery_id, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(
        !completion.spawned_entities,
        "a constructed-then-destroyed FreeUnit is not a successful spawn"
    );
    assert!(unit_ids(&sim, "Americans", "CMIN").is_empty());
    assert!(
        !sim.substrate.entities.values().any(|entity| sim
            .interner
            .resolve(entity.type_ref)
            .eq_ignore_ascii_case("CMIN")),
        "same-tick pending-delete drain must leave no living or limbo CMIN"
    );
    assert_eq!(credits_for_owner(&sim, "Americans"), 1500);
    assert_eq!(
        sim.houses
            .get(&americans)
            .expect("house should remain")
            .owned_unit_count,
        owned_units_before,
        "constructed FreeUnit owner count must be released exactly once"
    );

    let later = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(!later.spawned_entities);
    assert_eq!(
        credits_for_owner(&sim, "Americans"),
        1500,
        "consumed BuildingUp transition must not refund twice"
    );
}

#[test]
fn stock_soviet_refinery_completion_spawns_harv() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Russians", "NACNST", 14, 20);
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Russians",
        "NAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, &rules, "NAREFN", 20, 20);
    set_ticks_until_completion(&mut sim, refinery_id, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(completion.spawned_entities);
    let harvester_id = unit_ids(&sim, "Russians", "HARV")
        .into_iter()
        .next()
        .expect("stock Soviet refinery should spawn HARV");
    let harvester = sim
        .substrate
        .entities
        .get(harvester_id)
        .expect("spawned HARV should exist");
    assert_eq!((harvester.position.rx, harvester.position.ry), (22, 22));
    assert_eq!(harvester.facing, 0xC0);
    assert_eq!(
        harvester.mission.current().known(),
        Some(MissionType::Harvest)
    );
    assert!(unit_ids(&sim, "Russians", "CMIN").is_empty());
}

#[test]
fn non_refinery_completion_has_no_free_unit_or_credit_side_effect() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let construction_yard_id = sim
        .spawn_object("GACNST", "Americans", 20, 20, 0, &rules, &height_map)
        .expect("construction yard should spawn");
    sim.substrate
        .entities
        .get_mut(construction_yard_id)
        .expect("construction yard should exist")
        .building_up = Some(BuildingUp {
        elapsed_ticks: 0,
        total_ticks: 1,
    });
    let credits_before = credits_for_owner(&sim, "Americans");

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(!completion.spawned_entities);
    assert!(unit_ids(&sim, "Americans", "CMIN").is_empty());
    assert!(unit_ids(&sim, "Americans", "HARV").is_empty());
    assert_eq!(credits_for_owner(&sim, "Americans"), credits_before);
}

#[test]
fn simultaneous_refinery_completions_preserve_stable_id_order() {
    let mut sim = Simulation::new();
    let rules = stock_refinery_completion_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    spawn_structure(&mut sim, 2, "Russians", "NACNST", 14, 35);
    let allied_refinery = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    let soviet_refinery = ready_and_place(
        &mut sim,
        &rules,
        "Russians",
        "NAREFN",
        20,
        35,
        &grid,
        &height_map,
    );
    assert!(allied_refinery < soviet_refinery);
    block_building_foundation(&mut grid, &rules, "GAREFN", 20, 20);
    block_building_foundation(&mut grid, &rules, "NAREFN", 20, 35);
    set_ticks_until_completion(&mut sim, allied_refinery, 1);
    set_ticks_until_completion(&mut sim, soviet_refinery, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(completion.spawned_entities);
    let cmin = unit_ids(&sim, "Americans", "CMIN");
    let harv = unit_ids(&sim, "Russians", "HARV");
    assert_eq!(cmin.len(), 1);
    assert_eq!(harv.len(), 1);
    assert!(
        cmin[0] < harv[0],
        "FreeUnits must allocate in completed-building stable-ID order"
    );
}

#[test]
fn modded_refinery_completion_uses_free_unit_from_rules() {
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
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "MODPROC",
        20,
        20,
        &grid,
        &height_map,
    );
    assert!(unit_ids(&sim, "Americans", "MODHARV").is_empty());
    set_ticks_until_completion(&mut sim, refinery_id, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(completion.spawned_entities);
    assert_eq!(unit_ids(&sim, "Americans", "MODHARV").len(), 1);
}

#[test]
fn refinery_without_free_unit_spawns_nothing_on_completion() {
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
    let refinery_id = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "MODPROC",
        20,
        20,
        &grid,
        &height_map,
    );
    set_ticks_until_completion(&mut sim, refinery_id, 1);

    let completion = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 67);
    assert!(!completion.spawned_entities);
    assert!(unit_ids(&sim, "Americans", "MODHARV").is_empty());
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
fn placement_command_rejects_marked_ground_mobiles_until_they_are_unmarked() {
    let rules = ground_occupant_placement_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    for blocker_type in ["MTNK", "E1"] {
        let mut sim = Simulation::new();
        spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);
        let blocker_id = sim
            .spawn_object(blocker_type, "Americans", 13, 11, 0, &rules, &height_map)
            .expect("real spawn/unlimbo should mark the mobile occupant");
        assert!(
            sim.substrate.occupancy.contains_entity(13, 11, blocker_id),
            "{blocker_type} must enter the authoritative Ground object list"
        );

        // A dying infantry remains visible and marked until the app completes
        // its death animation and calls UnInit. Native placement reads the cell
        // object list, so this state must continue to block.
        if blocker_type == "E1" {
            sim.substrate
                .entities
                .get_mut(blocker_id)
                .expect("spawned infantry")
                .dying = true;
        }

        ready_building(&mut sim, "Americans", "GAPOWR");
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
        .expect("ready building should have a preview");
        assert!(!preview.valid, "{blocker_type} must reject the preview");
        assert_eq!(
            preview.cell_valid,
            vec![true, true, true, false],
            "only the non-origin foundation cell occupied by {blocker_type} should reject"
        );

        let americans = sim.interner.get("Americans").expect("owner interned");
        let gapowr = sim.interner.get("GAPOWR").expect("type interned");
        let entities_before = sim.substrate.entities.len();
        let next_id_before = sim.substrate.next_stable_object_id;
        let occupancy_generation_before = sim.substrate.occupancy.generation();
        let rejected = CommandEnvelope::new(
            americans,
            sim.session.tick + 1,
            Command::PlaceReadyBuilding {
                owner: americans,
                type_id: gapowr,
                rx: 12,
                ry: 10,
            },
        );
        let tick = sim.advance_tick(
            &[rejected],
            Some(&rules),
            &height_map,
            Some(&grid),
            None,
            67,
        );

        assert_eq!(tick.executed_commands, 1);
        assert!(
            !tick.spawned_entities,
            "rejected placement must not report a spawned entity"
        );
        assert_eq!(sim.substrate.entities.len(), entities_before);
        assert_eq!(sim.substrate.next_stable_object_id, next_id_before);
        assert_eq!(
            sim.substrate.occupancy.generation(),
            occupancy_generation_before,
            "rejected placement must not mutate CellClass-style membership"
        );
        assert_eq!(
            ready_buildings_for_owner(&sim, &rules, "Americans").len(),
            1,
            "rejected placement must preserve the ready building"
        );

        let _ = sim.conceal(blocker_id);
        assert!(
            !sim.substrate.occupancy.contains_entity(13, 11, blocker_id),
            "Conceal must remove the blocker before placement becomes legal"
        );
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
        .expect("ready building should retain its preview after rejection");
        assert!(
            preview.valid,
            "the same foundation must become legal after {blocker_type} is unmarked"
        );

        let accepted = CommandEnvelope::new(
            americans,
            sim.session.tick + 1,
            Command::PlaceReadyBuilding {
                owner: americans,
                type_id: gapowr,
                rx: 12,
                ry: 10,
            },
        );
        let tick = sim.advance_tick(
            &[accepted],
            Some(&rules),
            &height_map,
            Some(&grid),
            None,
            67,
        );
        assert!(tick.spawned_entities);
        assert!(ready_buildings_for_owner(&sim, &rules, "Americans").is_empty());
        assert!(sim.substrate.entities.values().any(|entity| {
            entity.type_ref == gapowr
                && entity.position.rx == 12
                && entity.position.ry == 10
                && entity.building_up.is_some()
        }));
    }
}

#[test]
fn placement_command_rejects_nonblocking_overlay_and_preserves_ready_building() {
    let mut sim = Simulation::new();
    let rules = ground_occupant_placement_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 10, 10);

    let mut overlay_grid = OverlayGrid::new(64, 64);
    overlay_grid.place_overlay(13, 11, 7, 4);
    sim.overlay_grid = Some(overlay_grid);
    ready_building(&mut sim, "Americans", "GAPOWR");

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
    .expect("ready building should have a preview");
    assert!(!preview.valid, "any ordinary nonempty overlay must reject");
    assert_eq!(preview.cell_valid, vec![true, true, true, false]);

    let americans = sim.interner.get("Americans").expect("owner interned");
    let gapowr = sim.interner.get("GAPOWR").expect("type interned");
    let entities_before = sim.substrate.entities.len();
    let next_id_before = sim.substrate.next_stable_object_id;
    let occupancy_generation_before = sim.substrate.occupancy.generation();
    let rejected = CommandEnvelope::new(
        americans,
        sim.session.tick + 1,
        Command::PlaceReadyBuilding {
            owner: americans,
            type_id: gapowr,
            rx: 12,
            ry: 10,
        },
    );
    let tick = sim.advance_tick(
        &[rejected],
        Some(&rules),
        &height_map,
        Some(&grid),
        None,
        67,
    );

    assert_eq!(tick.executed_commands, 1);
    assert!(!tick.spawned_entities);
    assert_eq!(sim.substrate.entities.len(), entities_before);
    assert_eq!(sim.substrate.next_stable_object_id, next_id_before);
    assert_eq!(
        sim.substrate.occupancy.generation(),
        occupancy_generation_before
    );
    assert_eq!(
        ready_buildings_for_owner(&sim, &rules, "Americans").len(),
        1,
        "rejected overlay placement must preserve the ready building"
    );
    let overlay = sim
        .overlay_grid
        .as_ref()
        .expect("overlay grid retained")
        .cell(13, 11);
    assert_eq!((overlay.overlay_id, overlay.overlay_data), (Some(7), 4));

    sim.overlay_grid
        .as_mut()
        .expect("overlay grid retained")
        .clear_overlay(13, 11);
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
    .expect("ready building should retain its preview after rejection");
    assert!(
        preview.valid,
        "the same foundation must become legal after the overlay is cleared"
    );

    let accepted = CommandEnvelope::new(
        americans,
        sim.session.tick + 1,
        Command::PlaceReadyBuilding {
            owner: americans,
            type_id: gapowr,
            rx: 12,
            ry: 10,
        },
    );
    let tick = sim.advance_tick(
        &[accepted],
        Some(&rules),
        &height_map,
        Some(&grid),
        None,
        67,
    );
    assert!(tick.spawned_entities);
    assert!(ready_buildings_for_owner(&sim, &rules, "Americans").is_empty());
}

#[test]
fn empty_cell_wall_placement_still_works_but_wall_on_overlay_rejects() {
    let rules = ground_occupant_placement_rules();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);

    let mut clear_sim = Simulation::new();
    spawn_structure(&mut clear_sim, 1, "Americans", "GACNST", 10, 10);
    ready_building(&mut clear_sim, "Americans", "GAWALL");
    let preview = placement_preview_for_owner(
        &clear_sim,
        &rules,
        "Americans",
        "GAWALL",
        12,
        10,
        Some(&grid),
        &height_map,
    )
    .expect("ready wall should have a preview");
    assert!(
        preview.valid,
        "the ordinary empty-cell wall preview must remain accepted: {:?}",
        preview.reason
    );
    assert!(
        place_ready_building(
            &mut clear_sim,
            &rules,
            "Americans",
            "GAWALL",
            12,
            10,
            Some(&grid),
            &height_map,
        ),
        "the ordinary empty-cell wall commit must remain accepted"
    );

    let mut overlay_sim = Simulation::new();
    spawn_structure(&mut overlay_sim, 1, "Americans", "GACNST", 10, 10);
    let mut overlay_grid = OverlayGrid::new(64, 64);
    overlay_grid.place_overlay(12, 10, 7, 4);
    overlay_sim.overlay_grid = Some(overlay_grid);
    ready_building(&mut overlay_sim, "Americans", "GAWALL");
    let preview = placement_preview_for_owner(
        &overlay_sim,
        &rules,
        "Americans",
        "GAWALL",
        12,
        10,
        Some(&grid),
        &height_map,
    )
    .expect("ready wall should have a preview");
    assert!(!preview.valid, "an ordinary wall must not replace ore");
    assert_eq!(
        ready_buildings_for_owner(&overlay_sim, &rules, "Americans").len(),
        1
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
fn gsi_04_04_water_bound_building_rejects_beach_zone() {
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
            cell.land_type = LandType::Beach.as_index();
            cell.zone_type = zone_class::BEACH;
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
fn gsi_04_04_water_bound_building_accepts_water_zone() {
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
            cell.land_type = LandType::Water.as_index();
            cell.zone_type = zone_class::WATER;
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
    assert!(matches!(
        sim.reveal(1),
        crate::sim::world::RevealOutcome::Revealed { .. }
    ));
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
        .substrate
        .entities
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
        !sim.substrate
            .entities
            .get(99)
            .unwrap()
            .has_live_contact_with(1),
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
        .substrate
        .entities
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
    let bldg = sim
        .substrate
        .entities
        .get(10)
        .expect("building should still exist");
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
