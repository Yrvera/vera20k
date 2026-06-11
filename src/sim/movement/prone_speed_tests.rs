//! Prone infantry speed integration tests.

use std::collections::BTreeMap;

use super::locomotor::MovementLayer;
use super::tick_movement_with_grids;
use crate::map::entities::EntityCategory;
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::locomotor_type::SpeedType;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::{Health, MovementTarget};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::{test_intern, test_interner};
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig;
use crate::sim::rng::SimRng;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

fn infantry_rules(crawls: bool) -> RuleSet {
    let rules_ini = IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [E1]\nStrength=100\nArmor=flak\nSpeed=4\nImage=GI\n",
    );
    let mut rules = RuleSet::from_ini(&rules_ini).expect("rules parse");
    let art_ini = IniFile::from_str(&format!(
        "[GI]\nCrawls={}\n",
        if crawls { "yes" } else { "no" }
    ));
    let art = ArtRegistry::from_ini(&art_ini);
    rules.merge_art_data(&art);
    rules
}

fn prone_mover() -> GameEntity {
    let mut entity = GameEntity::new(
        1,
        0,
        0,
        0,
        64,
        test_intern("Americans"),
        Health {
            current: 100,
            max: 100,
        },
        test_intern("E1"),
        EntityCategory::Infantry,
        0,
        5,
        false,
    );
    entity.position.sub_x = SimFixed::from_num(128);
    entity.position.sub_y = SimFixed::from_num(128);
    entity.position.refresh_screen_coords();
    entity.infantry.as_mut().expect("infantry runtime").is_prone = true;
    entity.movement_target = Some(MovementTarget {
        path: vec![(0, 0), (1, 0)],
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(11),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entity
}

fn advance_prone_mover(crawls: bool) -> SimFixed {
    let rules = infantry_rules(crawls);
    let mut entities = EntityStore::new();
    entities.insert(prone_mover());

    let mut rng = SimRng::new(0);
    let mut interner = test_interner();
    let mut occupancy = OccupancyGrid::new();
    let mut sounds = Vec::new();
    let mut next_occupancy_enter_order = crate::sim::world::EnterOrderCounter::new();
    let terrain_costs: BTreeMap<SpeedType, TerrainCostGrid> = BTreeMap::new();

    tick_movement_with_grids(
        &mut entities,
        &[],
        None,
        &terrain_costs,
        &Default::default(),
        &mut occupancy,
        &mut next_occupancy_enter_order,
        &mut rng,
        1000,
        0,
        None,
        None,
        &TerrainSpeedConfig::default(),
        SIM_ZERO,
        9,
        60,
        &mut interner,
        Some(&rules),
        &mut sounds,
    );

    entities.get(1).expect("entity exists").position.sub_x
}

#[test]
fn crawls_yes_prone_movement_uses_ceiling_two_thirds_speed() {
    assert_eq!(advance_prone_mover(true), SimFixed::from_num(136));
}

#[test]
fn crawls_no_prone_movement_uses_speed_plus_half() {
    assert_eq!(advance_prone_mover(false), SimFixed::from_num(144));
}
