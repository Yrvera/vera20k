//! Hermetic stock-contract production oracles for miner outbound Drive commands.

use crate::sim::movement::locomotion::LocomotorSlot;

use std::collections::BTreeMap;

use crate::map::bridge_facts::BridgeCellFacts;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::components::{DriveCoord, NavTargetRef};
use crate::sim::miner::{
    CargoBale, MinerConfig, MinerKind, MinerState, ResourceNode, ResourceType,
};
use crate::sim::movement::locomotor::{GroundMovePhase, MovementLayer, PiggybackLocomotor};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::passability::LandType;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::world::Simulation;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, ra2_speed_to_leptons_per_second};

const GRID_SIZE: u16 = 64;
const START: (u16, u16) = (32, 32);
const ONE_ORE_LEVEL: u16 = 120;

struct OutboundContractOracle {
    rules: RuleSet,
    overlays: OverlayTypeRegistry,
    tib01: u8,
    clear_speed_costs: SpeedCostProfile,
    tiberium_speed_costs: SpeedCostProfile,
}

fn outbound_contract_oracle() -> OutboundContractOracle {
    let rules_ini = IniFile::from_str(include_str!(
        "../../../tests/fixtures/ini/miner_outbound_rules_contract.ini"
    ));
    let mut rules = RuleSet::from_ini(&rules_ini).expect("outbound contract rules");
    let art_ini = IniFile::from_str(include_str!(
        "../../../tests/fixtures/ini/miner_outbound_art_contract.ini"
    ));
    rules.merge_art_data(&ArtRegistry::from_ini(&art_ini));
    let overlays = OverlayTypeRegistry::from_ini(&rules_ini, None);
    let tib01 = overlays.id_for_name("TIB01").expect("retail TIB01");
    let clear_speed_costs = rules
        .terrain_rules
        .semantics_by_name("Clear")
        .expect("merged [Clear]")
        .speed_costs;
    let tiberium_speed_costs = rules
        .terrain_rules
        .semantics_by_name("Tiberium")
        .expect("merged [Tiberium]")
        .speed_costs;

    assert_eq!(tiberium_speed_costs.track, Some(70));
    assert!(overlays.flags(tib01).is_some_and(|flags| flags.tiberium));
    for (type_id, expected_locomotor, teleporter) in [
        ("HARV", LocomotorKind::Drive, false),
        ("CMIN", LocomotorKind::Teleport, true),
    ] {
        let object = rules
            .object(type_id)
            .unwrap_or_else(|| panic!("retail {type_id}"));
        assert!(object.harvester, "{type_id} Harvester=yes");
        assert_eq!(object.speed, 4, "{type_id} Speed=4");
        assert_eq!(object.turret_rot, 5, "{type_id} ROT=5");
        assert!(object.crusher, "{type_id} Crusher=yes");
        assert_eq!(object.movement_zone, MovementZone::Crusher);
        assert_eq!(object.speed_type, SpeedType::Track);
        assert_eq!(object.locomotor, expected_locomotor);
        assert_eq!(object.teleporter, teleporter);
        assert!(object.accelerates);
        assert_eq!(object.accel_factor, SimFixed::lit("0.03"));
        assert_eq!(object.decel_factor, SimFixed::lit("0.002"));
        assert_eq!(object.slowdown_distance, 500);
    }

    OutboundContractOracle {
        rules,
        overlays,
        tib01,
        clear_speed_costs,
        tiberium_speed_costs,
    }
}

fn production_sim(seed: u64, oracle: &OutboundContractOracle) -> Simulation {
    let mut sim = Simulation::with_seed(seed);
    oracle.rules.intern_all_ids(&mut sim.interner);
    sim.resolve_type_handles(&oracle.rules);
    sim
}

fn resolved_cell(
    rx: u16,
    ry: u16,
    terrain_class: TerrainClass,
    land_type: u8,
    speed_costs: SpeedCostProfile,
) -> ResolvedTerrainCell {
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
        land_type,
        yr_cell_land_type: land_type,
        slope_type: 0,
        template_height: 0,
        render_offset_x: 0,
        render_offset_y: 0,
        terrain_class,
        speed_costs,
        is_water: false,
        is_cliff_like: false,
        is_rough: false,
        is_road: false,
        accepts_smudge: false,
        allows_tiberium: false,
        is_cliff_redraw: false,
        variant: 0,
        has_ramp: false,
        canonical_ramp: None,
        ground_walk_blocked: false,
        terrain_object_blocks: false,
        overlay_blocks: false,
        zone_type: zone_class::GROUND,
        base_ground_walk_blocked: false,
        base_build_blocked: false,
        base_land_type: LandType::Clear.as_index(),
        base_yr_cell_land_type: LandType::Clear.as_index(),
        base_terrain_class: TerrainClass::Clear,
        base_speed_costs: speed_costs,
        build_blocked: false,
        has_bridge_deck: false,
        bridge_walkable: false,
        bridge_transition: false,
        bridge_deck_level: 0,
        bridge_layer: None,
        bridge_facts: BridgeCellFacts::default(),
        tube_index: None,
        radar_left: [0, 0, 0],
        radar_right: [0, 0, 0],
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    }
}

fn staged_terrain(
    oracle: &OutboundContractOracle,
    ore_cells: &[(u16, u16)],
) -> ResolvedTerrainGrid {
    let clear_land_type = LandType::Clear.as_index();
    let mut terrain = ResolvedTerrainGrid::from_cells(
        GRID_SIZE,
        GRID_SIZE,
        (0..GRID_SIZE)
            .flat_map(|ry| {
                (0..GRID_SIZE).map(move |rx| {
                    resolved_cell(
                        rx,
                        ry,
                        TerrainClass::Clear,
                        clear_land_type,
                        oracle.clear_speed_costs,
                    )
                })
            })
            .collect(),
    );
    for &(rx, ry) in ore_cells {
        let cell = terrain.cell_mut(rx, ry).expect("staged ore cell");
        let tiberium_land_type = LandType::Tiberium.as_index();
        cell.land_type = tiberium_land_type;
        cell.yr_cell_land_type = tiberium_land_type;
        cell.terrain_class = TerrainClass::Tiberium;
        cell.speed_costs = oracle.tiberium_speed_costs;
        cell.allows_tiberium = true;
    }
    terrain
}

fn install_world(
    sim: &mut Simulation,
    oracle: &OutboundContractOracle,
    grid: &PathGrid,
    ore_cells: &[(u16, u16)],
    nodes: &[(u16, u16)],
    install_zones: bool,
) {
    let terrain = staged_terrain(oracle, ore_cells);
    sim.terrain_costs = SpeedType::ALL_WITH_COSTS
        .iter()
        .copied()
        .map(|speed_type| {
            (
                speed_type,
                TerrainCostGrid::from_resolved_terrain(&terrain, speed_type),
            )
        })
        .collect();
    sim.resolved_terrain = Some(terrain);
    sim.overlay_grid = Some(OverlayGrid::new(GRID_SIZE, GRID_SIZE));
    for &(rx, ry) in ore_cells {
        sim.overlay_grid
            .as_mut()
            .expect("overlay grid")
            .place_overlay(rx, ry, oracle.tib01, 0);
    }
    for &cell in nodes {
        sim.production.resource_nodes.insert(
            cell,
            ResourceNode {
                resource_type: ResourceType::Ore,
                remaining: ONE_ORE_LEVEL,
            },
        );
    }
    if install_zones {
        sim.rebuild_zone_grid(grid);
        assert!(sim.zone_grid.is_some());
    } else {
        sim.zone_grid = None;
    }
    for &(rx, ry) in ore_cells {
        assert_eq!(
            sim.terrain_costs
                .get(&SpeedType::Track)
                .expect("Track terrain costs")
                .cost_at(rx, ry),
            70,
        );
    }
}

fn spawn_stock_miner(
    sim: &mut Simulation,
    oracle: &OutboundContractOracle,
    type_id: &str,
    expected_kind: MinerKind,
) -> u64 {
    let id = sim
        .spawn_object(
            type_id,
            "Americans",
            START.0,
            START.1,
            0,
            &oracle.rules,
            &BTreeMap::new(),
        )
        .unwrap_or_else(|| panic!("spawn {type_id}"));
    let entity = sim.substrate.entities.get(id).expect("spawned miner");
    assert!(entity.lifecycle.object_alive);
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(entity.in_logic_vector);
    assert!(sim.live_object_order_snapshot().contains(&id));
    assert_eq!(
        entity.miner.as_ref().expect("miner component").kind,
        expected_kind,
    );
    assert_eq!(
        entity
            .locomotor
            .as_ref()
            .expect("stock locomotor")
            .movement_zone,
        MovementZone::Crusher,
    );
    id
}

fn spawn_stock_refinery(
    sim: &mut Simulation,
    oracle: &OutboundContractOracle,
    anchor: (u16, u16),
) -> u64 {
    let id = sim
        .spawn_object(
            "GAREFN",
            "Americans",
            anchor.0,
            anchor.1,
            0,
            &oracle.rules,
            &BTreeMap::new(),
        )
        .expect("spawn GAREFN");
    let entity = sim.substrate.entities.get(id).expect("spawned refinery");
    assert!(entity.lifecycle.object_alive);
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(entity.in_logic_vector);
    assert!(sim.live_object_order_snapshot().contains(&id));
    id
}

fn arm_full_ore_return(sim: &mut Simulation, entity_id: u64, config: &MinerConfig) {
    let entity = sim
        .substrate
        .entities
        .get_mut(entity_id)
        .expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.cargo = (0..miner.capacity_bales)
        .map(|_| CargoBale {
            resource_type: ResourceType::Ore,
            value: config.ore_bale_value,
        })
        .collect();
    miner.reserved_refinery = None;
    entity
        .mission
        .set_handler_state(MinerState::ReturnToRefinery.cursor());
}

fn arm_search(sim: &mut Simulation, entity_id: u64) {
    let entity = sim
        .substrate
        .entities
        .get_mut(entity_id)
        .expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    entity
        .mission
        .set_handler_state(MinerState::SearchOre.cursor());
    miner.target_ore_cell = None;
    miner.harvest_timer.clear();
}

fn advance(sim: &mut Simulation, oracle: &OutboundContractOracle, grid: &PathGrid) {
    let _ = sim.advance_tick(
        &[],
        Some(&oracle.rules),
        &BTreeMap::new(),
        Some(grid),
        Some(&oracle.overlays),
        67,
    );
}

fn position_tuple(sim: &Simulation, entity_id: u64) -> (u16, u16, SimFixed, SimFixed) {
    let position = &sim
        .substrate
        .entities
        .get(entity_id)
        .expect("entity")
        .position;
    (position.rx, position.ry, position.sub_x, position.sub_y)
}

fn assert_ore_intact(sim: &Simulation, oracle: &OutboundContractOracle, target: (u16, u16)) {
    let node = sim
        .production
        .resource_nodes
        .get(&target)
        .expect("positive ore node");
    assert_eq!(node.resource_type, ResourceType::Ore);
    assert_eq!(node.remaining, ONE_ORE_LEVEL);
    let overlay = sim
        .overlay_grid
        .as_ref()
        .expect("overlay grid")
        .cell(target.0, target.1);
    assert_eq!(overlay.overlay_id, Some(oracle.tib01));
    assert_eq!(overlay.overlay_data, 0);
}

fn assert_command_state(
    sim: &Simulation,
    oracle: &OutboundContractOracle,
    entity_id: u64,
    type_id: &str,
    target: (u16, u16),
) {
    let object = oracle.rules.object(type_id).expect("retail miner type");
    let entity = sim.substrate.entities.get(entity_id).expect("miner entity");
    let movement = entity.movement_target.as_ref().expect("movement target");
    assert_eq!(movement.path.first().copied(), Some(START));
    assert_eq!(movement.path.last().copied(), Some(target));
    assert_eq!(movement.final_goal, Some(target));
    assert_eq!(
        movement.speed,
        ra2_speed_to_leptons_per_second(object.speed),
    );
    assert_eq!(movement.accel_factor, object.accel_factor);
    assert_eq!(movement.decel_factor, object.decel_factor);
    assert_eq!(
        movement.slowdown_distance,
        SimFixed::from_num(object.slowdown_distance),
    );
    assert!(!movement.ignore_terrain_cost);
    assert!(!movement.bypass_grid);
    assert_eq!(
        entity.navigation.nav_com,
        Some(NavTargetRef::cell(target.0, target.1)),
    );
    let expected_coord = DriveCoord::cell(target.0, target.1, 0);
    let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
    assert_eq!(drive.destination, Some(expected_coord));
    assert_eq!(drive.head_to, Some(expected_coord));
    assert_eq!(
        drive.path.directions.len(),
        movement.path.len().saturating_sub(1),
    );
    assert!(!drive.path.directions.is_empty());
    // The Harvest handler dispatches BEFORE Phase-1 ground movement (the
    // native handler→locomotion order), so by observation time the drive has
    // already begun accelerating in the same tick the command was issued.
    assert!(drive.current_speed_fraction > SIM_ZERO);
    assert_eq!(
        entity.locomotor.as_ref().expect("active locomotor").kind,
        LocomotorKind::Drive,
    );
}

fn locomotor_tuple(
    sim: &Simulation,
    entity_id: u64,
) -> (
    LocomotorKind,
    LocomotorSlot,
    Option<PiggybackLocomotor>,
    MovementLayer,
    GroundMovePhase,
) {
    let locomotor = sim
        .substrate
        .entities
        .get(entity_id)
        .and_then(|entity| entity.locomotor.as_ref())
        .expect("locomotor");
    (
        locomotor.kind,
        locomotor.slot,
        locomotor.piggyback,
        locomotor.layer,
        locomotor.phase,
    )
}

#[test]
fn production_stock_miners_use_drive_command_for_adjacent_ore() {
    let oracle = outbound_contract_oracle();
    let target = (32, 31);
    for (type_id, kind) in [("HARV", MinerKind::War), ("CMIN", MinerKind::Chrono)] {
        let mut sim = production_sim(0x0715_D001, &oracle);
        let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
        install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
        let entity_id = spawn_stock_miner(&mut sim, &oracle, type_id, kind);
        let start_position = position_tuple(&sim, entity_id);
        arm_search(&mut sim, entity_id);
        let rng_before_search = sim.rng_state();

        advance(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before_search, "{type_id} search RNG");
        let entity = sim.substrate.entities.get(entity_id).expect("miner");
        let miner = entity.miner.as_ref().expect("miner");
        assert_eq!(entity.miner_state().unwrap(), MinerState::MoveToOre);
        assert_eq!(miner.target_ore_cell, Some(target));

        if type_id == "CMIN" {
            let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(entity.navigation.nav_com, None);
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(
                locomotor.slot,
                LocomotorSlot::from_kind(LocomotorKind::Teleport)
            );
            assert_eq!(locomotor.piggyback, None);
        }

        let rng_before_issue = sim.rng_state();
        advance(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before_issue, "{type_id} issue RNG");
        assert_command_state(&sim, &oracle, entity_id, type_id, target);
        {
            let entity = sim.substrate.entities.get(entity_id).expect("miner");
            let locomotor = entity.locomotor.as_ref().expect("locomotor");
            if type_id == "CMIN" {
                assert_eq!(
                    locomotor.slot,
                    LocomotorSlot::from_kind(LocomotorKind::Teleport)
                );
                assert_eq!(
                    locomotor.piggyback.expect("CMIN Drive piggyback").kind,
                    LocomotorKind::Teleport,
                );
            } else {
                assert_eq!(
                    locomotor.slot,
                    LocomotorSlot::from_kind(LocomotorKind::Drive)
                );
                assert_eq!(locomotor.piggyback, None);
            }
            assert!(entity.teleport_state.is_none());
        }

        advance(&mut sim, &oracle, &grid);
        {
            let entity = sim.substrate.entities.get(entity_id).expect("miner");
            let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
            let movement = entity.movement_target.as_ref().expect("movement");
            assert_eq!(drive.current_speed_fraction, SimFixed::lit("0.3"));
            assert_eq!(
                movement.current_speed,
                movement.speed * SimFixed::lit("0.3"),
            );
        }

        let mut physically_departed = position_tuple(&sim, entity_id) != start_position;
        let mut reached_harvest = false;
        for _ in 0..128 {
            advance(&mut sim, &oracle, &grid);
            let entity = sim.substrate.entities.get(entity_id).expect("miner");
            physically_departed |= position_tuple(&sim, entity_id) != start_position;
            reached_harvest |= entity.miner_state().expect("miner") == MinerState::Harvest;
            assert!(entity.teleport_state.is_none());
            if type_id == "CMIN" && entity.movement_target.is_some() {
                let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
                assert_eq!(locomotor.kind, LocomotorKind::Drive);
                assert_eq!(
                    locomotor.slot,
                    LocomotorSlot::from_kind(LocomotorKind::Teleport)
                );
                assert!(locomotor.piggyback.is_some());
            }
            if reached_harvest {
                break;
            }
        }
        assert!(physically_departed, "{type_id} must leave {START:?}");
        assert!(reached_harvest, "{type_id} must reach Harvest");
        let entity = sim.substrate.entities.get(entity_id).expect("miner");
        assert_eq!(entity.navigation.nav_com, None);
        assert!(!entity.navigation.pending_arrival_clear);
        if type_id == "CMIN" {
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(
                locomotor.slot,
                LocomotorSlot::from_kind(LocomotorKind::Teleport)
            );
            assert_eq!(locomotor.piggyback, None);
            assert!(
                entity.drive_locomotion.is_none(),
                "native FootClass::AI releases retired Drive"
            );
        }
        // The outbound leg legitimately consumes scenario RNG now: each
        // still-driving dispatch exits through the default Rate epilogue,
        // drawing one RandomRanged(0,2). Only the non-scenario streams stay
        // untouched across the leg.
        let rng_after = sim.rng_state();
        assert_eq!(rng_after.main, rng_before_search.main, "{type_id} main RNG");
        assert_eq!(
            rng_after.mapgen, rng_before_search.mapgen,
            "{type_id} mapgen RNG"
        );
        assert_ore_intact(&sim, &oracle, target);
    }
}

#[test]
fn production_harv_outbound_drive_uses_rule_profile() {
    let oracle = outbound_contract_oracle();
    let target = (32, 29);
    let mut sim = production_sim(0x0715_D002, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_search(&mut sim, entity_id);
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    // Search and issue consume no scenario RNG.
    assert_eq!(sim.rng_state(), rng_before);
    assert_command_state(&sim, &oracle, entity_id, "HARV", target);
    let harv = oracle.rules.object("HARV").expect("HARV");
    assert!(3 * 256 > harv.slowdown_distance);
    let acceleration = harv.accel_factor;

    // The next dispatch sees the non-null owner destination and exits through
    // the default Rate epilogue: exactly one RandomRanged(0,2) draw.
    let mut probe = sim.miner_jitter_rng().clone();
    let _ = probe.next_range_u32_inclusive(0, 2);
    let expected_scenario_after_draw = probe.logical_state();

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
    let movement = entity.movement_target.as_ref().expect("movement");
    // Dispatch precedes movement: the issuing tick already accelerated once,
    // so this (second post-issue) tick sits at two acceleration steps.
    assert_eq!(drive.current_speed_fraction, acceleration + acceleration);
    assert_eq!(
        movement.current_speed,
        movement.speed * (acceleration + acceleration)
    );
    assert!(movement.current_speed > SIM_ZERO);
    let rng_after = sim.rng_state();
    assert_eq!(rng_after.scenario, expected_scenario_after_draw);
    assert_eq!(rng_after.main, rng_before.main);
    assert_eq!(rng_after.mapgen, rng_before.mapgen);
}

#[test]
fn production_stock_harv_far_return_drive_uses_rule_profile() {
    let oracle = outbound_contract_oracle();
    let config = MinerConfig::from_rules(&oracle.rules);
    let refinery_anchor = (10, 10);
    let refinery_type = oracle.rules.object("GAREFN").expect("GAREFN");
    let queueing = refinery_type.queueing_cell.expect("stock QueueingCell");
    let staging = (
        refinery_anchor.0 + queueing.0,
        refinery_anchor.1 + queueing.1,
    );
    let accepted_dock = (refinery_anchor.0 + 3, refinery_anchor.1 + 1);
    assert_eq!(queueing, (4, 1));
    assert_ne!(staging, accepted_dock);

    let dx = u32::from(START.0.abs_diff(refinery_anchor.0));
    let dy = u32::from(START.1.abs_diff(refinery_anchor.1));
    let threshold = u32::from(config.too_far_threshold_standard);
    assert!(dx * dx + dy * dy > threshold * threshold);

    let mut sim = production_sim(0x0715_D008, &oracle);
    let mut grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    grid.block_building_movement_cells(
        refinery_anchor.0,
        refinery_anchor.1,
        &refinery_type.foundation,
        refinery_type.bib,
    );
    assert!(!grid.is_walkable(refinery_anchor.0, refinery_anchor.1));
    assert!(grid.is_walkable(staging.0, staging.1));
    install_world(&mut sim, &oracle, &grid, &[], &[], true);
    let refinery_id = spawn_stock_refinery(&mut sim, &oracle, refinery_anchor);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_full_ore_return(&mut sim, entity_id, &config);

    let start = position_tuple(&sim, entity_id);
    // Mission_Harvest's shared delay-jitter RNG tail remains a separate scheduler residual.
    advance(&mut sim, &oracle, &grid);

    let harv = oracle.rules.object("HARV").expect("HARV");
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    let miner = entity.miner.as_ref().expect("miner");
    let movement = entity.movement_target.as_ref().expect("movement target");
    let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
    assert_eq!(entity.miner_state().unwrap(), MinerState::ReturnToRefinery);
    assert_eq!(miner.reserved_refinery, Some(refinery_id));
    assert_eq!(movement.final_goal, Some(staging));
    assert_eq!(
        entity.navigation.nav_com,
        Some(NavTargetRef::cell(staging.0, staging.1)),
    );
    assert_eq!(
        drive.destination,
        Some(DriveCoord::cell(staging.0, staging.1, 0)),
    );
    assert_eq!(movement.speed, ra2_speed_to_leptons_per_second(harv.speed),);
    assert_eq!(movement.accel_factor, harv.accel_factor);
    assert_eq!(movement.decel_factor, harv.decel_factor);
    assert_eq!(
        movement.slowdown_distance,
        SimFixed::from_num(harv.slowdown_distance),
    );
    // Handler dispatch precedes Phase-1 movement, so the issuing tick already
    // ran the drive's first acceleration step.
    assert_eq!(drive.current_speed_fraction, harv.accel_factor);

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
    let movement = entity.movement_target.as_ref().expect("movement target");
    assert_eq!(
        drive.current_speed_fraction,
        harv.accel_factor + harv.accel_factor
    );
    assert_eq!(
        movement.current_speed,
        movement.speed * (harv.accel_factor + harv.accel_factor),
    );
    assert!(movement.current_speed > SIM_ZERO);

    let mut departed = position_tuple(&sim, entity_id) != start;
    for _ in 0..96 {
        if departed {
            break;
        }
        advance(&mut sim, &oracle, &grid);
        departed = position_tuple(&sim, entity_id) != start;
    }
    assert!(departed, "stock HARV must physically leave {start:?}");
}

#[test]
fn production_stock_harv_far_return_preserves_existing_navcom_owner() {
    let oracle = outbound_contract_oracle();
    let config = MinerConfig::from_rules(&oracle.rules);
    let refinery_anchor = (10, 10);
    let original = (32, 29);
    let refinery_type = oracle.rules.object("GAREFN").expect("GAREFN");
    let queueing = refinery_type.queueing_cell.expect("stock QueueingCell");
    let staging = (
        refinery_anchor.0 + queueing.0,
        refinery_anchor.1 + queueing.1,
    );
    let accepted_dock = (refinery_anchor.0 + 3, refinery_anchor.1 + 1);
    assert_eq!(queueing, (4, 1));
    assert_ne!(staging, accepted_dock);
    let mut sim = production_sim(0x0715_D009, &oracle);
    let mut grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    grid.block_building_movement_cells(
        refinery_anchor.0,
        refinery_anchor.1,
        &refinery_type.foundation,
        refinery_type.bib,
    );
    assert!(!grid.is_walkable(refinery_anchor.0, refinery_anchor.1));
    assert!(grid.is_walkable(staging.0, staging.1));
    install_world(&mut sim, &oracle, &grid, &[original], &[original], true);
    let refinery_id = spawn_stock_refinery(&mut sim, &oracle, refinery_anchor);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_search(&mut sim, entity_id);

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    {
        let entity = sim.substrate.entities.get(entity_id).expect("HARV");
        assert_eq!(
            entity.navigation.nav_com,
            Some(NavTargetRef::cell(original.0, original.1)),
        );
        assert_eq!(
            entity
                .drive_locomotion
                .as_ref()
                .expect("Drive runtime")
                .destination,
            Some(DriveCoord::cell(original.0, original.1, 0)),
        );
    }

    {
        let entity = sim.substrate.entities.get_mut(entity_id).expect("HARV");
        entity.movement_target = None;
    }
    arm_full_ore_return(&mut sim, entity_id, &config);

    let (
        nav_before,
        drive_before,
        target_before,
        cargo_before,
        timers_before,
        miner_contacts_before,
        dock_entered_before,
    ) = {
        let entity = sim.substrate.entities.get(entity_id).expect("HARV");
        let miner = entity.miner.as_ref().expect("miner");
        assert_eq!(miner.reserved_refinery, None);
        (
            entity.navigation.nav_com,
            entity
                .drive_locomotion
                .as_ref()
                .expect("Drive runtime")
                .clone(),
            miner.target_ore_cell,
            miner.cargo.clone(),
            (
                miner.harvest_timer,
                miner.rescan_cooldown,
                miner.dock_enter_retry,
                miner.approach_hello_timer,
                miner.mission_deploy_timer,
                miner.unload_cluster_timer,
            ),
            entity.radio_contacts.clone(),
            entity.dock_entered_with,
        )
    };
    let refinery_contacts_before = sim
        .substrate
        .entities
        .get(refinery_id)
        .expect("GAREFN")
        .radio_contacts
        .clone();
    let sound_count_before = sim.sound_events.len();
    assert!(!sim.production.dock_reservations.is_occupied(refinery_id));

    // This oracle covers the state-2 owner gate, not the shared mission-delay RNG tail.
    advance(&mut sim, &oracle, &grid);

    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    let miner = entity.miner.as_ref().expect("miner");
    let timers_after = (
        miner.harvest_timer,
        miner.rescan_cooldown,
        miner.dock_enter_retry,
        miner.approach_hello_timer,
        miner.mission_deploy_timer,
        miner.unload_cluster_timer,
    );
    assert_eq!(entity.miner_state().unwrap(), MinerState::ReturnToRefinery);
    assert_eq!(miner.reserved_refinery, None);
    assert_eq!(entity.navigation.nav_com, nav_before);
    assert_eq!(entity.drive_locomotion.as_ref(), Some(&drive_before));
    assert!(entity.movement_target.is_none());
    assert_eq!(miner.target_ore_cell, target_before);
    assert_eq!(miner.cargo, cargo_before);
    assert_eq!(timers_after, timers_before);
    assert_eq!(entity.radio_contacts, miner_contacts_before);
    assert_eq!(entity.dock_entered_with, dock_entered_before);
    assert_eq!(
        sim.substrate
            .entities
            .get(refinery_id)
            .expect("GAREFN")
            .radio_contacts,
        refinery_contacts_before,
    );
    assert!(!sim.production.dock_reservations.is_occupied(refinery_id));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact(refinery_id, entity_id),
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(refinery_id, entity_id),
    );
    assert!(
        !sim.production
            .dock_reservations
            .is_on_pad(refinery_id, entity_id),
    );
    assert_eq!(sim.sound_events.len(), sound_count_before);
}

#[test]
fn production_cmin_outbound_drive_keeps_teleport_primary() {
    let oracle = outbound_contract_oracle();
    let target = (32, 29);
    let mut sim = production_sim(0x0715_D003, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_search(&mut sim, entity_id);
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    {
        let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
        let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
        assert_eq!(entity.navigation.nav_com, None);
        assert_eq!(locomotor.kind, LocomotorKind::Teleport);
        assert_eq!(
            locomotor.slot,
            LocomotorSlot::from_kind(LocomotorKind::Teleport)
        );
        assert_eq!(locomotor.piggyback, None);
    }

    advance(&mut sim, &oracle, &grid);
    assert_command_state(&sim, &oracle, entity_id, "CMIN", target);
    {
        let locomotor = sim
            .substrate
            .entities
            .get(entity_id)
            .and_then(|entity| entity.locomotor.as_ref())
            .expect("CMIN locomotor");
        assert_eq!(
            locomotor.slot,
            LocomotorSlot::from_kind(LocomotorKind::Teleport)
        );
        assert!(locomotor.piggyback.is_some());
    }

    let mut reached_harvest = false;
    for _ in 0..240 {
        advance(&mut sim, &oracle, &grid);
        let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
        assert!(entity.teleport_state.is_none());
        if entity.movement_target.is_some() {
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Drive);
            assert_eq!(
                locomotor.slot,
                LocomotorSlot::from_kind(LocomotorKind::Teleport)
            );
            assert!(locomotor.piggyback.is_some());
        }
        if entity.miner_state().expect("miner") == MinerState::Harvest {
            assert!(entity.movement_target.is_none());
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(
                locomotor.slot,
                LocomotorSlot::from_kind(LocomotorKind::Teleport)
            );
            assert_eq!(locomotor.piggyback, None);
            assert_eq!(entity.navigation.nav_com, None);
            assert!(!entity.navigation.pending_arrival_clear);
            assert!(entity.drive_locomotion.is_none());
            reached_harvest = true;
            break;
        }
    }
    assert!(reached_harvest);
    // The still-driving dispatches draw one scenario RandomRanged(0,2) each
    // (the default Rate epilogue); only the non-scenario streams are
    // untouched across the outbound leg.
    let rng_after = sim.rng_state();
    assert_eq!(rng_after.main, rng_before.main);
    assert_eq!(rng_after.mapgen, rng_before.mapgen);
}

#[test]
fn production_cmin_failed_outbound_issue_restores_locomotor_exactly() {
    let oracle = outbound_contract_oracle();
    let target = (32, 29);
    let mut sim = production_sim(0x0715_D004, &oracle);
    let mut grid = PathGrid::test_all_blocked(GRID_SIZE, GRID_SIZE);
    grid.set_blocked(START.0, START.1, false);
    grid.set_blocked(target.0, target.1, false);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], false);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_search(&mut sim, entity_id);
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    let before = locomotor_tuple(&sim, entity_id);
    assert_eq!(before.0, LocomotorKind::Teleport);
    assert_eq!(before.1, LocomotorSlot::from_kind(LocomotorKind::Teleport));
    assert_eq!(before.2, None);
    assert_eq!(
        sim.substrate
            .entities
            .get(entity_id)
            .expect("CMIN")
            .navigation
            .nav_com,
        None,
    );

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
    assert!(entity.movement_target.is_none());
    assert_eq!(entity.navigation.nav_com, None);
    assert_eq!(locomotor_tuple(&sim, entity_id), before);
    assert_eq!(sim.rng_state(), rng_before);
}

#[test]
fn production_harv_navcom_without_movement_target_is_not_reissued() {
    let oracle = outbound_contract_oracle();
    let original = (32, 29);
    let preferable = (32, 31);
    let mut sim = production_sim(0x0715_D005, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(
        &mut sim,
        &oracle,
        &grid,
        &[original, preferable],
        &[original],
        true,
    );
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_search(&mut sim, entity_id);

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    assert_eq!(
        sim.substrate
            .entities
            .get(entity_id)
            .expect("HARV")
            .navigation
            .nav_com,
        Some(NavTargetRef::cell(original.0, original.1)),
    );

    {
        let entity = sim.substrate.entities.get_mut(entity_id).expect("HARV");
        entity.movement_target = None;
    }
    sim.production.resource_nodes.insert(
        preferable,
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: ONE_ORE_LEVEL,
        },
    );
    let rng_before = sim.rng_state();
    // The still-driving dispatch (non-null NavCom) exits through the default
    // Rate epilogue: exactly one scenario RandomRanged(0,2) draw, no scan.
    let mut probe = sim.miner_jitter_rng().clone();
    let _ = probe.next_range_u32_inclusive(0, 2);
    let expected_scenario_after_draw = probe.logical_state();

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    assert_eq!(
        entity.miner.as_ref().expect("miner").target_ore_cell,
        Some(original),
    );
    assert_eq!(
        entity.navigation.nav_com,
        Some(NavTargetRef::cell(original.0, original.1)),
    );
    assert!(
        entity.movement_target.is_none(),
        "non-null NavCom must suppress scan and command reissue",
    );
    let rng_after = sim.rng_state();
    assert_eq!(rng_after.scenario, expected_scenario_after_draw);
    assert_eq!(rng_after.main, rng_before.main);
    assert_eq!(rng_after.mapgen, rng_before.mapgen);
}

#[test]
fn production_harv_navcom_defers_removed_target_revalidation() {
    let oracle = outbound_contract_oracle();
    let original = (32, 29);
    let replacement = (32, 31);
    let mut sim = production_sim(0x0715_D006, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(
        &mut sim,
        &oracle,
        &grid,
        &[original, replacement],
        &[original],
        true,
    );
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_search(&mut sim, entity_id);

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    assert_command_state(&sim, &oracle, entity_id, "HARV", original);

    {
        let entity = sim.substrate.entities.get_mut(entity_id).expect("HARV");
        entity.movement_target = None;
        assert_eq!(
            entity.navigation.nav_com,
            Some(NavTargetRef::cell(original.0, original.1)),
            "fixture must isolate the native NavCom owner gate",
        );
    }
    sim.production.resource_nodes.remove(&original);
    sim.production.resource_nodes.insert(
        replacement,
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: ONE_ORE_LEVEL,
        },
    );
    let rng_before = sim.rng_state();
    // The still-driving dispatch (non-null NavCom) exits through the default
    // Rate epilogue: exactly one scenario RandomRanged(0,2) draw, no scan.
    let mut probe = sim.miner_jitter_rng().clone();
    let _ = probe.next_range_u32_inclusive(0, 2);
    let expected_scenario_after_draw = probe.logical_state();

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    let miner = entity.miner.as_ref().expect("miner");
    assert_eq!(entity.miner_state().unwrap(), MinerState::MoveToOre);
    assert_eq!(miner.target_ore_cell, Some(original));
    assert_eq!(
        entity.navigation.nav_com,
        Some(NavTargetRef::cell(original.0, original.1)),
    );
    assert!(
        entity.movement_target.is_none(),
        "non-null NavCom must defer depletion validation and command reissue",
    );
    let rng_after = sim.rng_state();
    assert_eq!(rng_after.scenario, expected_scenario_after_draw);
    assert_eq!(rng_after.main, rng_before.main);
    assert_eq!(rng_after.mapgen, rng_before.mapgen);
}

#[test]
fn production_cmin_arrival_clears_navcom_same_tick_and_releases_drive() {
    let oracle = outbound_contract_oracle();
    let target = (32, 31);
    let mut sim = production_sim(0x0715_D007, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_search(&mut sim, entity_id);

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    assert_command_state(&sim, &oracle, entity_id, "CMIN", target);

    // Drive leg → arrival tick. A track that ends at the owner destination
    // clears the owner NavCom pair on the SAME tick (no deferred interval),
    // and the same tick's piggyback-restore pass releases the retired Drive.
    let mut arrived = false;
    for _ in 0..128 {
        advance(&mut sim, &oracle, &grid);
        let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
        assert!(entity.teleport_state.is_none());
        if (entity.position.rx, entity.position.ry) == target && entity.movement_target.is_none() {
            arrived = true;
            assert_eq!(entity.navigation.nav_com, None);
            assert!(!entity.navigation.pending_arrival_clear);
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(
                locomotor.slot,
                LocomotorSlot::from_kind(LocomotorKind::Teleport)
            );
            assert_eq!(locomotor.piggyback, None);
            assert!(
                entity.drive_locomotion.is_none(),
                "restoring primary Teleport must release retired Drive runtime",
            );
            break;
        }
    }
    assert!(arrived, "CMIN must complete the outbound drive leg");

    // The MoveToOre→Harvest transition lands on the next due dispatch; the
    // still-driving dispatches are paced at the Rate-epilogue cadence, so the
    // next due dispatch can be up to ~16 frames after physical arrival.
    let mut reached_harvest = false;
    for _ in 0..32 {
        advance(&mut sim, &oracle, &grid);
        let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
        if entity.miner_state().expect("miner") == MinerState::Harvest {
            reached_harvest = true;
            assert_eq!(entity.navigation.nav_com, None);
            assert!(!entity.navigation.pending_arrival_clear);
            assert!(entity.drive_locomotion.is_none());
            break;
        }
    }
    assert!(
        reached_harvest,
        "MoveToOre→Harvest lands on the next due dispatch"
    );
    assert_ore_intact(&sim, &oracle, target);
}

/// Regression: the low-bridge tube gate must not classify the dock pad-entry
/// direct move (a deliberate multi-cell, bypass-grid straight-line segment)
/// as a failed tube traversal. It did, which stranded every miner whose
/// accepted CAN_DOCK entry began more than one cell from the pad — the
/// player-visible "chrono miner fills up and never returns" stall in every
/// ordinary skirmish with an Allied refinery.
#[test]
fn cmin_full_close_return_docks_and_deposits() {
    use crate::sim::miner::RefineryDockPhase;
    let oracle = outbound_contract_oracle();
    let config = MinerConfig::from_rules(&oracle.rules);
    let refinery_anchor = (10, 10);
    let mut sim = production_sim(0xDEB6, &oracle);
    let mut grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    let refinery_type = oracle.rules.object("GAREFN").expect("GAREFN");
    grid.block_building_movement_cells(
        refinery_anchor.0,
        refinery_anchor.1,
        &refinery_type.foundation,
        refinery_type.bib,
    );
    install_world(&mut sim, &oracle, &grid, &[], &[], true);
    let _refinery_id = spawn_stock_refinery(&mut sim, &oracle, refinery_anchor);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_full_ore_return(&mut sim, entity_id, &config);

    let mut deposited_at = None;
    for tick in 0..3000u32 {
        advance(&mut sim, &oracle, &grid);
        let e = sim.substrate.entities.get(entity_id).expect("miner");
        if e.miner.as_ref().expect("miner comp").cargo.is_empty() {
            deposited_at = Some(tick);
            break;
        }
    }
    let deposited_at = deposited_at.expect(
        "full CMIN must complete the close-return dock cycle and deposit \
         (stall = the pad-entry direct move died; see tube gate)",
    );

    // Let the state-4 departure and the delayed Harvest redispatch run.
    for _ in 0..100u32 {
        advance(&mut sim, &oracle, &grid);
    }

    // The cycle physically completed: the miner entered the refinery pad,
    // cargo empty, dock bookkeeping released, and the handler resumed
    // harvest scheduling.
    let e = sim.substrate.entities.get(entity_id).expect("miner");
    let m = e.miner.as_ref().expect("miner comp");
    assert!(m.cargo.is_empty());
    assert_eq!(m.dock_phase, RefineryDockPhase::Approach);
    assert!(
        m.reserved_refinery.is_none(),
        "reservation released at exit"
    );
    assert!(
        matches!(
            e.miner_state().expect("cursor"),
            MinerState::SearchOre | MinerState::WaitNoOre
        ),
        "post-deposit cursor resumes harvest scheduling, got {:?}",
        e.miner_state(),
    );
    assert!(
        deposited_at > 0,
        "deposit happened during the run (tick {deposited_at})"
    );
}

/// Regression: the second harvest cycle must leave the refinery pad. At dock
/// departure the miner stands on the pad still facing the refinery, so the
/// outbound move begins with a sharp (>=135°) turn whose fallback drive track
/// consumes the first path node — leaving the "next" node two cells out. The
/// tube gate classified that non-adjacent step on a plain (non-tube) cell as
/// a failed tube traversal and killed the move on its issue tick, freezing
/// the miner on the pad in a dispatch/kill loop after every deposit.
#[test]
fn cmin_second_cycle_leaves_the_pad_and_reharvests() {
    let oracle = outbound_contract_oracle();
    let config = MinerConfig::from_rules(&oracle.rules);
    let refinery_anchor = (10, 10);
    let ore: &[(u16, u16)] = &[(22, 18), (23, 18), (22, 19), (23, 19)];
    let mut sim = production_sim(0xDEB7, &oracle);
    let mut grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    let refinery_type = oracle.rules.object("GAREFN").expect("GAREFN");
    grid.block_building_movement_cells(
        refinery_anchor.0,
        refinery_anchor.1,
        &refinery_type.foundation,
        refinery_type.bib,
    );
    install_world(&mut sim, &oracle, &grid, ore, ore, true);
    // install_world places overlays at density 0 (the outbound suite never
    // extracts). Reduce_Tiberium reads the overlay density byte, so give the
    // patch real density or harvesting yields zero bales.
    for &(rx, ry) in ore {
        sim.overlay_grid
            .as_mut()
            .expect("overlay grid")
            .set_overlay_data(rx, ry, 11);
        sim.production
            .resource_nodes
            .get_mut(&(rx, ry))
            .expect("ore node")
            .remaining = 11 * ONE_ORE_LEVEL;
    }
    let _refinery_id = spawn_stock_refinery(&mut sim, &oracle, refinery_anchor);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_full_ore_return(&mut sim, entity_id, &config);

    let mut deposited_at = None;
    for tick in 0..3000u32 {
        advance(&mut sim, &oracle, &grid);
        let e = sim.substrate.entities.get(entity_id).expect("miner");
        if e.miner.as_ref().expect("miner comp").cargo.is_empty() {
            deposited_at = Some(tick);
            break;
        }
    }
    deposited_at.expect("cycle 1 deposit completes");

    let mut reharvested = false;
    for _ in 0..2000u32 {
        advance(&mut sim, &oracle, &grid);
        let e = sim.substrate.entities.get(entity_id).expect("miner");
        if !e.miner.as_ref().expect("miner comp").cargo.is_empty() {
            reharvested = true;
            break;
        }
    }
    assert!(
        reharvested,
        "miner must drive back out and harvest again after the first deposit \
         (stall = sharp-turn fallback path killed by the tube gate)",
    );
}
