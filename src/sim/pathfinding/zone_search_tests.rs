//! Tests for zone-aware pathfinding wrappers.

use super::super::zone_hierarchy::{ZoneEdgeRecord, ZoneHierarchy, ZoneLevelGraph, ZoneRecord};
use super::super::zone_map::{ZoneAdjacency, ZoneGrid, ZoneInfo, ZoneMap};
use super::*;
use crate::map::bridge_facts::{BRIDGE_FLAG_DIRECTION_ZERO, BRIDGE_FLAG_STRUCTURAL};
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::ini_parser::IniFile;
use crate::rules::locomotor_type::MovementZone;
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
use crate::sim::combat::AttackTarget;
use crate::sim::components::{NavTargetRef, OrderIntent};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::test_interner;
use crate::sim::miner::miner_system::{issue_move_if_idle, issue_stock_miner_drive_move};
use crate::sim::miner::{CargoBale, MinerConfig, MinerState, RefineryDockPhase, ResourceType};
use crate::sim::movement::{bump_crush, issue_move_command_with_layered, tick_movement_with_grids};
use crate::sim::occupancy::{CellOccupationGrid, OccupancyGrid};
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig;
use crate::sim::production::{ProductionCategory, STARTING_CREDITS, tick_production};
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;
use crate::util::fixed_math::SimFixed;
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

fn gsi_04_12_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
    let cells = (0..height)
        .flat_map(|ry| {
            (0..width).map(move |rx| ResolvedTerrainCell {
                rx,
                ry,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: false,
                tileset_index: None,
                land_type: LandType::Clear.as_index(),
                yr_cell_land_type: LandType::Clear.as_index(),
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: TerrainClass::Clear,
                speed_costs: SpeedCostProfile::default(),
                is_water: false,
                is_cliff_like: false,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                allows_tiberium: false,
                height_in_pixels: 0,
                variant: 0,
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
                base_land_type: LandType::Clear.as_index(),
                base_yr_cell_land_type: LandType::Clear.as_index(),
                base_terrain_class: TerrainClass::Clear,
                base_speed_costs: SpeedCostProfile::default(),
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
                bridge_layer: None,
                bridge_facts: Default::default(),
                tube_index: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: false,
                bridgehead_anchor_class_at_load: None,
            })
        })
        .collect();
    let mut terrain = ResolvedTerrainGrid::from_cells(width, height, cells);
    terrain.test_set_high_bridge_set_starts(Some(100), None);
    terrain
}

fn gsi_04_12_cell_listed_entity(
    stable_id: u64,
    type_ref: &str,
    owner: &str,
    rx: u16,
    ry: u16,
) -> GameEntity {
    let mut entity = GameEntity::test_default(stable_id, type_ref, owner, rx, ry);
    entity.lifecycle.in_limbo = false;
    entity.lifecycle.cell_marked = true;
    entity
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
    let width = zones.len() as u16;
    level0_hierarchy(zones, width, 1, edges)
}

fn level0_hierarchy(
    zones: Vec<ZoneId>,
    width: u16,
    height: u16,
    edges: &[(ZoneId, ZoneId)],
) -> ZoneHierarchy {
    debug_assert_eq!(zones.len(), width as usize * height as usize);
    let zone_count = zones.iter().copied().max().unwrap_or(0);
    let mut level2 = ZoneLevelGraph::new(1);
    level2.set_record(ZoneRecord::new(1, 0, 0));

    let mut level1 = ZoneLevelGraph::new(1);
    level1.set_record(ZoneRecord::new(1, 1, 0));

    let mut level0 = ZoneLevelGraph::new(zone_count).with_cell_zone_ids(zones, width, height);
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
    zg.set_hierarchy(linear_level0_hierarchy(vec![1, 2, 3], &[(1, 2), (2, 3)]));
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
    zg.set_hierarchy(linear_level0_hierarchy(
        vec![ZONE_INVALID, ZONE_INVALID, ZONE_INVALID],
        &[],
    ));

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
        false,
        Some(&blocker_counts),
    )
    .expect("same-zone ZoneMap fallback should survive incomplete hierarchy cell IDs");

    assert_eq!(path, vec![(0, 0), (1, 0), (2, 0)]);
}

#[test]
fn gsi_04_12_layered_production_precheck_projects_only_hierarchy_coordinates() {
    let mut astar_grid = PathGrid::new(5, 1);
    for x in 1..=3 {
        astar_grid.set_cell_for_test(x, 0, 0, true, true);
    }

    let mut reduced_grid = PathGrid::new(5, 1);
    reduced_grid.set_blocked(2, 0, true);
    let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 5, 1);
    zone_grid.set_hierarchy(linear_level0_hierarchy(vec![1, 3, 1, 4, 2], &[(1, 2)]));
    assert!(
        !zone_grid.can_reach(
            MovementZone::Normal,
            (1, 0),
            MovementLayer::Bridge,
            (3, 0),
            MovementLayer::Bridge,
        ),
        "fixture must prove reduced reachability would abort the layered search"
    );

    let mut terrain = gsi_04_12_terrain(5, 1);
    for x in 1..=3 {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_transition = true;
        cell.bridge_deck_level = 4;
    }
    terrain.cell_mut(0, 0).unwrap().final_tile_index = 100;
    terrain.cell_mut(4, 0).unwrap().final_tile_index = 100;

    let make_entities = || {
        let mut entities = EntityStore::new();
        let mut mover = gsi_04_12_cell_listed_entity(1, "HTNK", "Americans", 1, 0);
        let mut locomotor = crate::sim::movement::locomotor::LocomotorState::for_test_kind(
            crate::rules::locomotor_type::LocomotorKind::Drive,
        );
        locomotor.layer = MovementLayer::Bridge;
        mover.locomotor = Some(locomotor);
        mover.on_bridge = true;
        mover.drive_locomotion = Some(Default::default());
        entities.insert(mover);
        entities.insert(gsi_04_12_cell_listed_entity(2, "HTNK", "Russians", 2, 0));
        entities
    };

    let mut entities = make_entities();
    let interner = test_interner();
    let blocker_counts =
        bump_crush::build_blocker_neighbor_counts(&entities, 5, 1, Some(&terrain), &interner, None);
    let mut cell_occupation = CellOccupationGrid::new();
    assert!(issue_move_command_with_layered(
        &mut entities,
        &astar_grid,
        1,
        (3, 0),
        SimFixed::from_num(128),
        false,
        None,
        None,
        Some(&terrain),
        Some(&zone_grid),
        None,
        false,
        Some(&blocker_counts),
        Some(&mut cell_occupation),
    ));
    let movement = entities
        .get(1)
        .and_then(|entity| entity.movement_target.as_ref())
        .expect("production command should install the projected hierarchy route");

    assert_eq!(
        movement.path.first().copied(),
        Some((1, 0)),
        "projection must not mutate the A* start coordinate or layer"
    );
    assert_eq!(
        movement.path_layers.first().copied(),
        Some(MovementLayer::Bridge),
        "production command must keep the raw A* start layer"
    );
    assert_eq!(
        movement.path.last().copied(),
        Some((3, 0)),
        "projection must not mutate the A* goal or returned path"
    );

    terrain.cell_mut(3, 0).unwrap().bridge_facts.raw_flags = 0;
    let mut entities = make_entities();
    let blocker_counts =
        bump_crush::build_blocker_neighbor_counts(&entities, 5, 1, Some(&terrain), &interner, None);
    assert!(
        !issue_move_command_with_layered(
            &mut entities,
            &astar_grid,
            1,
            (3, 0),
            SimFixed::from_num(128),
            false,
            None,
            None,
            Some(&terrain),
            Some(&zone_grid),
            None,
            false,
            Some(&blocker_counts),
            None,
        ),
        "destination projection must be selected by the destination structural bit"
    );
}

#[test]
fn gsi_04_12_completed_ground_unit_rally_threads_exact_blocker_counts() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n\n\
         [BuildingTypes]\n0=GAWEAP\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\n\n\
         [GAWEAP]\nStrength=1000\nFoundation=1x1\nFactory=UnitType\nExitCoord=256,0,0\n",
    ))
    .expect("ground-unit production rules should parse");

    // The real war-factory exit is ground cell (1,0); the rally is ground cell
    // (5,0), across the intact high bridge at x=2..=4. The dynamic blocker is
    // adjacent at (3,1), so its exact neighbor counts cover the unmarked bridge
    // cells without its live occupation illegally standing on the only route.
    let mut path_grid = PathGrid::new(6, 2);
    for x in 0..6 {
        path_grid.set_blocked(x, 1, true);
    }
    path_grid.set_cell_for_test(1, 0, 4, false, false);
    path_grid.set_cell_for_test(5, 0, 4, false, false);
    for x in 2..=4 {
        path_grid.set_cell_for_test(x, 0, 0, true, true);
    }
    let mut reduced_grid = PathGrid::new(6, 2);
    for x in 0..6 {
        reduced_grid.set_blocked(x, 1, true);
    }
    reduced_grid.set_blocked(3, 0, true);
    let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 6, 2);
    zone_grid.set_hierarchy(level0_hierarchy(
        vec![
            1,
            1,
            3,
            1,
            4,
            2,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
        ],
        6,
        2,
        &[(1, 2)],
    ));
    assert!(
        !zone_grid.can_reach(
            MovementZone::Normal,
            (1, 0),
            MovementLayer::Ground,
            (5, 0),
            MovementLayer::Ground,
        ),
        "fixture must make the old no-count rally path abort in reduced reachability"
    );

    let mut terrain = gsi_04_12_terrain(6, 2);
    for x in 2..=4 {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_transition = true;
        cell.bridge_deck_level = 4;
    }
    terrain.cell_mut(1, 0).unwrap().final_tile_index = 100;
    terrain.cell_mut(1, 0).unwrap().level = 4;
    terrain.cell_mut(5, 0).unwrap().final_tile_index = 100;
    terrain.cell_mut(5, 0).unwrap().level = 4;

    let mut height_map = BTreeMap::new();
    height_map.insert((1, 0), 4);
    let mut sim = Simulation::new();
    sim.resolved_terrain = Some(terrain);
    sim.zone_grid = Some(zone_grid);
    sim.spawn_object("GAWEAP", "Americans", 0, 0, 0, &rules, &height_map)
        .expect("war factory should spawn");
    sim.spawn_object("MTNK", "Russians", 3, 1, 0, &rules, &height_map)
        .expect("dynamic blocker should spawn");

    let owner = sim.interner.intern("Americans");
    let produced_type = sim.interner.intern("MTNK");
    sim.houses.insert(
        owner,
        crate::sim::house_state::HouseState::new(owner, 0, None, true, STARTING_CREDITS, 10),
    );
    sim.houses.get_mut(&owner).unwrap().rally_point = Some((5, 0));
    sim.production.factory_shadow.enqueue(
        owner,
        ProductionCategory::Vehicle,
        produced_type,
        0,
        100,
        0,
    );
    assert!(
        sim.production
            .factory_shadow
            .test_arm_ready(owner, ProductionCategory::Vehicle)
    );

    assert!(
        tick_production(&mut sim, &rules, &height_map, Some(&path_grid)),
        "ready ground-unit production should deliver through the real completion entry"
    );

    let produced = sim
        .substrate
        .entities
        .values()
        .find(|entity| entity.owner == owner && entity.type_ref == produced_type)
        .expect("completed MTNK should exist");
    assert_eq!((produced.position.rx, produced.position.ry), (1, 0));
    let locomotor = produced.locomotor.as_ref().expect("produced locomotor");
    assert_eq!(
        locomotor.kind,
        crate::rules::locomotor_type::LocomotorKind::Drive
    );
    assert_eq!(locomotor.movement_zone, MovementZone::Normal);
    assert!(!produced.on_bridge);
    assert!(!produced.too_big_to_fit_under_bridge);
    let movement = produced
        .movement_target
        .as_ref()
        .expect("completed MTNK should receive the hierarchy-backed rally route");
    assert_eq!(movement.path.first().copied(), Some((1, 0)));
    assert_eq!(movement.path_layers.first(), Some(&MovementLayer::Ground));
    assert!(
        movement
            .path_layers
            .iter()
            .any(|layer| *layer == MovementLayer::Bridge),
        "the rally route must actually traverse the high-bridge layer"
    );
    assert_eq!(movement.path.last().copied(), Some((5, 0)));
    assert!(sim.production.factory_shadow.is_empty());
    assert_eq!(sim.houses.get(&owner).unwrap().rally_point, Some((5, 0)));
}

#[test]
fn gsi_04_12_miner_dock_approach_threads_exact_blocker_counts() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=HARV\n1=BLOCK\n\n\
         [BuildingTypes]\n0=REFN\n\n\
         [HARV]\nStrength=600\nSpeed=4\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\nMovementZone=Normal\nHarvester=yes\nDock=REFN\nStorage=20\n\n\
         [BLOCK]\nStrength=300\nSpeed=4\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\nMovementZone=Normal\n\n\
         [REFN]\nStrength=900\nFoundation=1x1\nRefinery=yes\n",
    ))
    .expect("dock-approach rules should parse");

    // The refinery's 1x1 geometric QueueingCell is (1,0). The live miner
    // starts at (5,0), across the intact high bridge at x=2..=4. The blocker
    // at (3,1) contributes exact neighbor counts without occupying the route.
    let mut path_grid = PathGrid::new(6, 2);
    for x in 0..6 {
        path_grid.set_blocked(x, 1, true);
    }
    path_grid.set_cell_for_test(1, 0, 4, false, false);
    path_grid.set_cell_for_test(5, 0, 4, false, false);
    for x in 2..=4 {
        path_grid.set_cell_for_test(x, 0, 0, true, true);
    }
    let mut reduced_grid = PathGrid::new(6, 2);
    for x in 0..6 {
        reduced_grid.set_blocked(x, 1, true);
    }
    reduced_grid.set_blocked(3, 0, true);
    let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 6, 2);
    zone_grid.set_hierarchy(level0_hierarchy(
        vec![
            1,
            1,
            3,
            1,
            4,
            2,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
        ],
        6,
        2,
        &[(1, 2)],
    ));
    assert!(
        !zone_grid.can_reach(
            MovementZone::Normal,
            (5, 0),
            MovementLayer::Ground,
            (1, 0),
            MovementLayer::Ground,
        ),
        "fixture must make the old no-count dock approach abort in reduced reachability"
    );

    let mut terrain = gsi_04_12_terrain(6, 2);
    for x in 2..=4 {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_transition = true;
        cell.bridge_deck_level = 4;
    }
    for x in [1, 5] {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.final_tile_index = 100;
        cell.level = 4;
    }

    let mut height_map = BTreeMap::new();
    height_map.insert((1, 0), 4);
    height_map.insert((5, 0), 4);
    let mut sim = Simulation::new();
    sim.resolved_terrain = Some(terrain);
    sim.zone_grid = Some(zone_grid);
    let refinery_id = sim
        .spawn_object("REFN", "Americans", 0, 0, 0, &rules, &height_map)
        .expect("refinery should spawn");
    let miner_id = sim
        .spawn_object("HARV", "Americans", 5, 0, 0, &rules, &height_map)
        .expect("harvester should spawn");
    sim.spawn_object("BLOCK", "Russians", 3, 1, 0, &rules, &height_map)
        .expect("dynamic blocker should spawn");
    {
        let entity = sim.substrate.entities.get_mut(miner_id).unwrap();
        let miner = entity.miner.as_mut().expect("HARV should own miner state");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.reserved_refinery = Some(refinery_id);
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.approach_hello_timer = crate::sim::mission::timer::MissionTimer::armed(0, 10);
        entity.mission.set_handler_state(MinerState::Dock.cursor());
    }

    crate::sim::miner::miner_system::tick_miners(
        &mut sim,
        &rules,
        &MinerConfig::default(),
        Some(&path_grid),
    );

    let miner = sim.substrate.entities.get(miner_id).unwrap();
    assert_eq!(miner.miner_state(), Some(MinerState::Dock));
    let miner_state = miner.miner.as_ref().unwrap();
    assert_eq!(miner_state.dock_phase, RefineryDockPhase::Approach);
    assert_eq!(miner_state.reserved_refinery, Some(refinery_id));
    let movement = miner
        .movement_target
        .as_ref()
        .expect("live Approach dispatch should install the hierarchy-backed queue route");
    assert_eq!(movement.path.first().copied(), Some((5, 0)));
    assert!(
        movement
            .path_layers
            .iter()
            .any(|layer| *layer == MovementLayer::Bridge),
        "dock approach must actually traverse the high-bridge layer"
    );
    assert_eq!(movement.path.last().copied(), Some((1, 0)));
}

#[test]
fn gsi_04_12_interaction_order_entry_threads_exact_blocker_counts() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=ENGINEER\n\n\
         [VehicleTypes]\n0=BLOCK\n\n\
         [BuildingTypes]\n0=TARGET\n\n\
         [ENGINEER]\nStrength=75\nSpeed=4\nLocomotor={4A582744-9839-11d1-B709-00A024DDAFD1}\nMovementZone=Infantry\nEngineer=yes\n\n\
         [BLOCK]\nStrength=300\nSpeed=4\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\nMovementZone=Normal\n\n\
         [TARGET]\nStrength=500\nFoundation=1x1\nCapturable=yes\n",
    ))
    .expect("interaction-order rules should parse");

    let mut path_grid = PathGrid::new(6, 2);
    for x in 0..6 {
        path_grid.set_blocked(x, 1, true);
    }
    path_grid.set_cell_for_test(1, 0, 4, false, false);
    path_grid.set_cell_for_test(5, 0, 4, false, false);
    for x in 2..=4 {
        path_grid.set_cell_for_test(x, 0, 0, true, true);
    }
    let mut reduced_grid = PathGrid::new(6, 2);
    for x in 0..6 {
        reduced_grid.set_blocked(x, 1, true);
    }
    reduced_grid.set_blocked(3, 0, true);
    let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 6, 2);
    zone_grid.set_hierarchy(level0_hierarchy(
        vec![
            1,
            1,
            3,
            1,
            4,
            2,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
            ZONE_INVALID,
        ],
        6,
        2,
        &[(1, 2)],
    ));
    assert!(
        !zone_grid.can_reach(
            MovementZone::Infantry,
            (1, 0),
            MovementLayer::Ground,
            (5, 0),
            MovementLayer::Ground,
        ),
        "fixture must make the old no-count interaction path abort in reduced reachability"
    );

    let mut terrain = gsi_04_12_terrain(6, 2);
    for x in 2..=4 {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_transition = true;
        cell.bridge_deck_level = 4;
    }
    for x in [1, 5] {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.final_tile_index = 100;
        cell.level = 4;
    }

    let mut height_map = BTreeMap::new();
    height_map.insert((1, 0), 4);
    height_map.insert((5, 0), 4);
    let mut sim = Simulation::new();
    sim.resolved_terrain = Some(terrain);
    sim.zone_grid = Some(zone_grid);
    let engineer_id = sim
        .spawn_object("ENGINEER", "Americans", 1, 0, 0, &rules, &height_map)
        .expect("engineer should spawn");
    let target_id = sim
        .spawn_object("TARGET", "Russians", 5, 0, 0, &rules, &height_map)
        .expect("capture target should spawn");
    sim.spawn_object("BLOCK", "Russians", 3, 1, 0, &rules, &height_map)
        .expect("dynamic blocker should spawn");

    assert!(sim.apply_command(
        "Americans",
        &crate::sim::command::Command::CaptureBuilding {
            engineer_id,
            target_building_id: target_id,
        },
        Some(&rules),
        Some(&path_grid),
        &height_map,
    ));

    let engineer = sim.substrate.entities.get(engineer_id).unwrap();
    assert_eq!(engineer.capture_target, Some(target_id));
    let movement = engineer
        .movement_target
        .as_ref()
        .expect("live Capture order should install the hierarchy-backed target route");
    assert_eq!(movement.path.first().copied(), Some((1, 0)));
    assert!(
        movement
            .path_layers
            .iter()
            .any(|layer| *layer == MovementLayer::Bridge),
        "interaction approach must actually traverse the high-bridge layer"
    );
    assert_eq!(
        movement.path.last().copied(),
        Some((4, 0)),
        "the concrete Capture path should stop adjacent to the occupied building"
    );
}

#[test]
fn gsi_04_12_attack_pursuit_entry_threads_exact_blocker_counts() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=1\nWarhead=AP\n\n\
         [AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
    ))
    .expect("pursuit rules should parse");

    let mut path_grid = PathGrid::new(5, 1);
    for x in 1..=3 {
        path_grid.set_cell_for_test(x, 0, 0, true, true);
    }

    let mut reduced_grid = PathGrid::new(5, 1);
    reduced_grid.set_blocked(2, 0, true);
    let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 5, 1);
    zone_grid.set_hierarchy(linear_level0_hierarchy(vec![1, 3, 1, 4, 2], &[(1, 2)]));
    assert!(
        !zone_grid.can_reach(
            MovementZone::Normal,
            (1, 0),
            MovementLayer::Bridge,
            (3, 0),
            MovementLayer::Bridge,
        ),
        "fixture must make the no-count pursuit path fail reduced reachability"
    );

    let mut terrain = gsi_04_12_terrain(5, 1);
    for x in 1..=3 {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_transition = true;
        cell.bridge_deck_level = 4;
    }
    terrain.cell_mut(0, 0).unwrap().final_tile_index = 100;
    terrain.cell_mut(4, 0).unwrap().final_tile_index = 100;

    let mut attacker = gsi_04_12_cell_listed_entity(1, "MTNK", "Americans", 1, 0);
    let mut locomotor = crate::sim::movement::locomotor::LocomotorState::for_test_kind(
        crate::rules::locomotor_type::LocomotorKind::Drive,
    );
    locomotor.layer = MovementLayer::Bridge;
    attacker.locomotor = Some(locomotor);
    attacker.on_bridge = true;
    attacker.drive_locomotion = Some(Default::default());
    attacker.attack_target = Some(AttackTarget::for_cell(3, 0));
    let blocker = gsi_04_12_cell_listed_entity(2, "MTNK", "Russians", 2, 0);

    let mut sim = Simulation::new();
    sim.interner = test_interner();
    sim.substrate.entities.insert(attacker);
    sim.substrate.entities.insert(blocker);
    sim.resolved_terrain = Some(terrain);
    sim.zone_grid = Some(zone_grid);

    sim.tick_attack_pursuit(&rules, Some(&path_grid));

    let movement = sim
        .substrate
        .entities
        .get(1)
        .and_then(|entity| entity.movement_target.as_ref())
        .expect("real out-of-range pursuit should reach the projected hierarchy route");
    assert_eq!(movement.path.first().copied(), Some((1, 0)));
    assert_eq!(
        movement.path_layers.first().copied(),
        Some(MovementLayer::Bridge)
    );
    assert_eq!(movement.path.last().copied(), Some((3, 0)));
}

#[test]
fn gsi_04_12_phase_six_order_resume_threads_exact_blocker_counts() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\n",
    ))
    .expect("phase-six movement rules should parse");

    let mut path_grid = PathGrid::new(5, 1);
    for x in 1..=3 {
        path_grid.set_cell_for_test(x, 0, 0, true, true);
    }

    let mut reduced_grid = PathGrid::new(5, 1);
    reduced_grid.set_blocked(2, 0, true);
    let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 5, 1);
    zone_grid.set_hierarchy(linear_level0_hierarchy(vec![1, 3, 1, 4, 2], &[(1, 2)]));
    assert!(
        !zone_grid.can_reach(
            MovementZone::Normal,
            (1, 0),
            MovementLayer::Bridge,
            (3, 0),
            MovementLayer::Bridge,
        ),
        "fixture must make Phase-6 resume fail when blocker counts are absent"
    );

    let mut terrain = gsi_04_12_terrain(5, 1);
    for x in 1..=3 {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_transition = true;
        cell.bridge_deck_level = 4;
    }
    terrain.cell_mut(0, 0).unwrap().final_tile_index = 100;
    terrain.cell_mut(4, 0).unwrap().final_tile_index = 100;

    let mut mover = gsi_04_12_cell_listed_entity(1, "MTNK", "Americans", 1, 0);
    let mut locomotor = crate::sim::movement::locomotor::LocomotorState::for_test_kind(
        crate::rules::locomotor_type::LocomotorKind::Drive,
    );
    locomotor.layer = MovementLayer::Bridge;
    mover.locomotor = Some(locomotor);
    mover.on_bridge = true;
    mover.drive_locomotion = Some(Default::default());
    mover.order_intent = Some(OrderIntent::AttackMove {
        goal_rx: 3,
        goal_ry: 0,
    });
    let blocker = gsi_04_12_cell_listed_entity(2, "MTNK", "Russians", 2, 0);

    let mut sim = Simulation::new();
    sim.interner = test_interner();
    sim.substrate.entities.insert(mover);
    sim.substrate.entities.insert(blocker);
    sim.resolved_terrain = Some(terrain);
    sim.zone_grid = Some(zone_grid);

    sim.tick_order_intents_post_combat(Some(&path_grid), Some(&rules));

    let resumed = sim.substrate.entities.get(1).expect("resumed mover");
    let movement = resumed
        .movement_target
        .as_ref()
        .expect("real Phase-6 resume should reach the projected hierarchy route");
    assert_eq!(movement.path.first().copied(), Some((1, 0)));
    assert_eq!(
        movement.path_layers.first().copied(),
        Some(MovementLayer::Bridge)
    );
    assert_eq!(movement.path.last().copied(), Some((3, 0)));
    assert_eq!(
        resumed.order_intent,
        Some(OrderIntent::AttackMove {
            goal_rx: 3,
            goal_ry: 0,
        }),
        "resume must preserve the continuing AttackMove order"
    );
}

#[test]
fn gsi_04_12_drive_pending_continuation_keeps_hierarchy_context_and_raw_route() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\n",
    ))
    .expect("Drive continuation rules should parse");

    let mut path_grid = PathGrid::new(5, 1);
    for x in 1..=3 {
        path_grid.set_cell_for_test(x, 0, 0, true, true);
    }

    let mut reduced_grid = PathGrid::new(5, 1);
    reduced_grid.set_blocked(2, 0, true);
    let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 5, 1);
    zone_grid.set_hierarchy(linear_level0_hierarchy(vec![1, 3, 1, 4, 2], &[(1, 2)]));
    assert!(
        !zone_grid.can_reach(
            MovementZone::Normal,
            (1, 0),
            MovementLayer::Bridge,
            (3, 0),
            MovementLayer::Bridge,
        ),
        "fixture must reject a continuation that drops the hierarchy context"
    );

    let mut terrain = gsi_04_12_terrain(5, 1);
    for x in 1..=3 {
        let cell = terrain.cell_mut(x, 0).unwrap();
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
        cell.has_bridge_deck = true;
        cell.bridge_walkable = true;
        cell.bridge_transition = true;
        cell.bridge_deck_level = 4;
    }
    terrain.cell_mut(0, 0).unwrap().final_tile_index = 100;
    terrain.cell_mut(4, 0).unwrap().final_tile_index = 100;

    let mut mover = gsi_04_12_cell_listed_entity(1, "MTNK", "Americans", 1, 0);
    let mut locomotor = crate::sim::movement::locomotor::LocomotorState::for_test_kind(
        crate::rules::locomotor_type::LocomotorKind::Drive,
    );
    locomotor.layer = MovementLayer::Bridge;
    mover.locomotor = Some(locomotor);
    mover.on_bridge = true;
    mover.drive_locomotion = Some(Default::default());
    mover.navigation.nav_com = Some(NavTargetRef::cell(3, 0));
    mover.navigation.pending_arrival_clear = true;
    let blocker = gsi_04_12_cell_listed_entity(2, "MTNK", "Russians", 2, 0);

    let mut entities = EntityStore::new();
    entities.insert(mover);
    entities.insert(blocker);
    let mut interner = test_interner();
    let mut occupancy = OccupancyGrid::new();
    let mut cell_occupation = CellOccupationGrid::new();
    let mut raw_cell_occupation = crate::sim::occupancy::RawCellOccupationGrid::new();
    let mut enter_order = crate::sim::world::EnterOrderCounter::new();
    let mut rng = SimRng::new(0);
    let terrain_speed_config = TerrainSpeedConfig::default();
    let terrain_costs = BTreeMap::new();
    let alliances = HouseAllianceMap::new();
    let mut sound_events = Vec::new();
    let mut lifecycle_requests = Vec::new();
    let live_order = [1];

    tick_movement_with_grids(
        &mut entities,
        Some(&live_order),
        Some(&path_grid),
        &terrain_costs,
        &alliances,
        &mut occupancy,
        &mut cell_occupation,
        &mut raw_cell_occupation,
        &mut enter_order,
        &mut rng,
        1,
        1,
        Some(&zone_grid),
        Some(&terrain),
        None,
        &terrain_speed_config,
        SimFixed::from_num(0),
        9,
        60,
        &mut interner,
        Some(&rules),
        &mut sound_events,
        &mut lifecycle_requests,
    );

    let continued = entities.get(1).expect("continued Drive mover");
    assert_eq!(continued.navigation.nav_com, Some(NavTargetRef::cell(3, 0)));
    assert!(!continued.navigation.pending_arrival_clear);
    let movement = continued
        .movement_target
        .as_ref()
        .expect("pending Drive continuation should rebuild the hierarchy route");
    assert_eq!(movement.path.first().copied(), Some((1, 0)));
    assert_eq!(
        movement.path_layers.first().copied(),
        Some(MovementLayer::Bridge)
    );
    assert_eq!(movement.path.last().copied(), Some((3, 0)));
    assert_eq!(movement.final_goal, Some((3, 0)));
}

#[test]
fn gsi_04_12_stock_miner_move_entries_thread_exact_world_context() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=HARV\n\n\
         [HARV]\nStrength=1000\nArmor=heavy\nSpeed=5\nHarvester=yes\n",
    ))
    .expect("stock harvester rules should parse");

    let mut path_grid = PathGrid::new(5, 1);
    for x in 1..=3 {
        path_grid.set_cell_for_test(x, 0, 0, true, true);
    }

    let make_sim = || {
        let mut reduced_grid = PathGrid::new(5, 1);
        reduced_grid.set_blocked(2, 0, true);
        let mut zone_grid = ZoneGrid::build(&reduced_grid, &BTreeMap::new(), 5, 1);
        zone_grid.set_hierarchy(linear_level0_hierarchy(vec![1, 3, 1, 4, 2], &[(1, 2)]));
        assert!(
            !zone_grid.can_reach(
                MovementZone::Normal,
                (1, 0),
                MovementLayer::Bridge,
                (3, 0),
                MovementLayer::Bridge,
            ),
            "fixture must make either miner entry fail if it drops blocker counts"
        );

        let mut terrain = gsi_04_12_terrain(5, 1);
        for x in 1..=3 {
            let cell = terrain.cell_mut(x, 0).unwrap();
            cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DIRECTION_ZERO;
            cell.has_bridge_deck = true;
            cell.bridge_walkable = true;
            cell.bridge_transition = true;
            cell.bridge_deck_level = 4;
        }
        terrain.cell_mut(0, 0).unwrap().final_tile_index = 100;
        terrain.cell_mut(4, 0).unwrap().final_tile_index = 100;

        let mut miner = gsi_04_12_cell_listed_entity(1, "HARV", "Americans", 1, 0);
        let mut locomotor = crate::sim::movement::locomotor::LocomotorState::for_test_kind(
            crate::rules::locomotor_type::LocomotorKind::Drive,
        );
        locomotor.layer = MovementLayer::Bridge;
        miner.locomotor = Some(locomotor);
        miner.on_bridge = true;
        miner.drive_locomotion = Some(Default::default());
        let blocker = gsi_04_12_cell_listed_entity(2, "HARV", "Russians", 2, 0);

        let mut sim = Simulation::new();
        sim.interner = test_interner();
        sim.substrate.entities.insert(miner);
        sim.substrate.entities.insert(blocker);
        sim.resolved_terrain = Some(terrain);
        sim.zone_grid = Some(zone_grid);
        sim
    };

    let mut ore_trip = make_sim();
    assert!(issue_stock_miner_drive_move(
        &mut ore_trip,
        &rules,
        &path_grid,
        1,
        (3, 0),
    ));
    assert_eq!(
        ore_trip
            .substrate
            .entities
            .get(1)
            .and_then(|entity| entity.movement_target.as_ref())
            .and_then(|movement| movement.path.last().copied()),
        Some((3, 0)),
    );

    let mut refinery_return = make_sim();
    issue_move_if_idle(
        &mut refinery_return,
        Some(&rules),
        &path_grid,
        1,
        (3, 0),
        SimFixed::from_num(128),
        None,
    );
    assert_eq!(
        refinery_return
            .substrate
            .entities
            .get(1)
            .and_then(|entity| entity.movement_target.as_ref())
            .and_then(|movement| movement.path.last().copied()),
        Some((3, 0)),
    );
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

/// GSI-06.02 G2: gamemd gates every MovementZone row — `Can_Reach_Zone`
/// short-circuits only on `mzRow == -1`, and the A*-entry precheck reads
/// whatever row `MovementZone=` gives. Stock rulesmd puts every main battle tank
/// in `Destroyer`, every ore miner in `Crusher` and the Battle Fortress in
/// `CrusherAll`, so those rows must reach the reduced precheck.
#[test]
fn gsi_06_02_reduced_zone_precheck_covers_every_land_movement_zone() {
    for mz in [
        MovementZone::Normal,
        MovementZone::Crusher,
        MovementZone::Destroyer,
        MovementZone::AmphibiousDestroyer,
        MovementZone::AmphibiousCrusher,
        MovementZone::Amphibious,
        MovementZone::Infantry,
        MovementZone::InfantryDestroyer,
        MovementZone::Fly,
        MovementZone::CrusherAll,
    ] {
        assert!(
            can_use_reduced_zone_precheck(Some(mz)),
            "{mz:?} must be gated by the reduced zone precheck"
        );
    }

    // `mzRow == -1` returns "reachable" in gamemd, so the gate must not be
    // allowed to refuse the search for it.
    assert!(!can_use_reduced_zone_precheck(Some(MovementZone::Invalid)));

    // VERA-internal residual, gamemd equivalent UNCHECKED: naval surface
    // legality in the zone builder is still coarser than the runtime predicate,
    // so the two water rows stay outside the gate for now.
    assert!(!can_use_reduced_zone_precheck(Some(MovementZone::Water)));
    assert!(!can_use_reduced_zone_precheck(Some(
        MovementZone::WaterBeach
    )));
}
