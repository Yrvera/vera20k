//! Focused regression tests for ordered lifecycle authority.

use std::collections::BTreeMap;

use crate::map::bridge_facts::{
    BRIDGE_FLAG_ANCHOR_SELF, BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts, BridgeStampFamily,
};
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::locomotor_type::LocomotorKind;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::anim_class::{AnimObject, AnimRuntime, AnimWorldCoord};
use crate::sim::animation::{Animation, SequenceKind};
use crate::sim::bridge_state::StateOutcome;
use crate::sim::combat::{AttackTarget, PendingInfantryFire, TargetKind};
use crate::sim::components::{
    C4PlantState, DriveLocomotionRuntime, DriveOccupationFootprint, Health, NavTargetRef,
};
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::mission::state::MissionTestFixture;
use crate::sim::mission::{MissionDispatchTimer, MissionId, MissionType};
use crate::sim::movement::homing_movement::{HomingTarget, attach_homing_state};
use crate::sim::movement::locomotor::{AirMovePhase, LocomotorState, MovementLayer};
use crate::sim::occupancy::CellListInsertion;
use crate::sim::particles::ParticleSystem;
use crate::sim::passenger::{PassengerCargo, PassengerRole};
use crate::util::fixed_math::SimFixed;
use glam::IVec3;

use super::{
    LifecycleOutput, LifecycleTestEvent, PlacementEvidence, RevealFailure, RevealOutcome,
    RevealPosition, RevealRequest, Simulation,
};

fn insert_entity(sim: &mut Simulation, stable_id: u64, category: EntityCategory) {
    let owner = sim.interner.intern("Americans");
    let type_ref = sim.interner.intern("TEST");
    let mut entity = GameEntity::new_at_frame_zero_for_test(
        stable_id,
        2,
        3,
        0,
        0,
        owner,
        Health {
            current: 100,
            max: 100,
        },
        type_ref,
        category,
        0,
        5,
        category != EntityCategory::Infantry,
    );
    entity.category = category;
    sim.substrate.entities.insert(entity);
}

fn request(rx: u16, ry: u16, placement: PlacementEvidence) -> RevealRequest {
    RevealRequest {
        position: RevealPosition {
            rx,
            ry,
            z: 2,
            sub_x: SimFixed::from_num(128),
            sub_y: SimFixed::from_num(64),
        },
        placement,
        logic_eligible: true,
    }
}

fn common_raw_request(rx: u16, ry: u16, z: u8, sub_x: i32, sub_y: i32) -> RevealRequest {
    RevealRequest {
        position: RevealPosition {
            rx,
            ry,
            z,
            sub_x: SimFixed::from_num(sub_x),
            sub_y: SimFixed::from_num(sub_y),
        },
        placement: PlacementEvidence::MarkSucceeded,
        logic_eligible: true,
    }
}

fn common_raw_terrain_cell(
    rx: u16,
    ry: u16,
    level: u8,
    has_bridge_deck: bool,
) -> ResolvedTerrainCell {
    let bridge_facts = if has_bridge_deck {
        BridgeCellFacts {
            raw_flags: BRIDGE_FLAG_ANCHOR_SELF | BRIDGE_FLAG_STRUCTURAL,
            family: BridgeStampFamily::Nesw,
            direction: Some(0),
            ..BridgeCellFacts::default()
        }
    } else {
        BridgeCellFacts::default()
    };
    ResolvedTerrainCell {
        rx,
        ry,
        source_tile_index: 0,
        source_sub_tile: 0,
        final_tile_index: 0,
        final_sub_tile: 0,
        is_wood_bridge_repair_tile: false,
        level,
        filled_clear: false,
        tileset_index: Some(0),
        land_type: 0,
        yr_cell_land_type: 0,
        slope_type: 0,
        template_height: level,
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
        is_cliff_redraw: false,
        variant: 0,
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
        base_terrain_class: TerrainClass::Clear,
        base_speed_costs: SpeedCostProfile::default(),
        build_blocked: false,
        has_bridge_deck,
        bridge_walkable: has_bridge_deck,
        bridge_transition: false,
        bridge_deck_level: if has_bridge_deck {
            level.saturating_add(4)
        } else {
            level
        },
        bridge_layer: None,
        bridge_facts,
        tube_index: None,
        radar_left: [0, 0, 0],
        radar_right: [0, 0, 0],
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    }
}

fn install_common_raw_terrain(
    sim: &mut Simulation,
    width: u16,
    height: u16,
    level: u8,
    bridge_cell: Option<(u16, u16)>,
) {
    let cells = (0..height)
        .flat_map(|ry| {
            (0..width).map(move |rx| {
                common_raw_terrain_cell(rx, ry, level, bridge_cell == Some((rx, ry)))
            })
        })
        .collect();
    let terrain = ResolvedTerrainGrid::from_cells(width, height, cells);
    sim.bridge_state = Some(
        crate::sim::bridge_state::BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500),
    );
    sim.resolved_terrain = Some(terrain);
}

fn install_fly_aircraft(sim: &mut Simulation, stable_id: u64, altitude: SimFixed) {
    insert_entity(sim, stable_id, EntityCategory::Aircraft);
    let mut locomotor = LocomotorState::for_test_kind(LocomotorKind::Fly);
    locomotor.altitude = altitude;
    locomotor.air_phase = if altitude <= SimFixed::from_num(0) {
        AirMovePhase::Landed
    } else {
        AirMovePhase::Cruising
    };
    sim.substrate
        .entities
        .get_mut(stable_id)
        .expect("aircraft")
        .locomotor = Some(locomotor);
}

#[test]
fn gsi_04_12_common_raw_occupation_ground_unit_links_marks_then_unlinks_clears() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0, 128, 128));

    assert!(sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x20);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
    let linked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListLinked)
        .expect("object list link event");
    let marked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationMarked)
        .expect("raw mark event");
    assert!(
        linked < marked,
        "selected object list must link before raw mark"
    );

    sim.lifecycle_test_events.clear();
    let _ = sim.object_conceal(1);

    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    let unlinked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .expect("object list unlink event");
    let cleared = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationCleared)
        .expect("raw clear event");
    assert!(
        unlinked < cleared,
        "selected object list must unlink before raw clear"
    );
}

#[test]
fn gsi_04_12_common_raw_occupation_structural_deck_unit_tracks_production_collapse() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 2, Some((3, 4)));
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 6, 128, 128));

    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0x20);
    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(3, 4, MovementLayer::Ground),
        1,
        "raw deck selection must not reuse the OnBridge object-list selector"
    );

    let _ = sim.object_conceal(1);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);

    {
        let terrain = sim.resolved_terrain.as_ref().expect("resolved terrain");
        let bridge_state = sim.bridge_state.as_mut().expect("bridge runtime state");
        assert!(matches!(
            bridge_state.body_cell_advance_state(3, 4, true, terrain),
            StateOutcome::Absorbed
        ));
        assert!(matches!(
            bridge_state.body_cell_advance_state(3, 4, true, terrain),
            StateOutcome::Collapsed { .. }
        ));
        assert!(
            bridge_state
                .cell(3, 4)
                .expect("collapsed bridge cell")
                .deck_present,
            "collapse leaves the structural deck record present"
        );
        assert!(!bridge_state.is_bridge_walkable(3, 4));
    }

    insert_entity(&mut sim, 2, EntityCategory::Unit);
    let _ = sim.try_reveal_entity(2, common_raw_request(3, 4, 6, 128, 128));

    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x20);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);

    let _ = sim.object_conceal(2);

    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(3, 4),
        0x20,
        "height-only clear still targets deck after the collapsed mark used ground"
    );
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
}

#[test]
fn gsi_04_12_common_raw_occupation_signed_z_marks_ground_on_live_bridge() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 0, Some((3, 4)));
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0x80, 128, 128));

    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x20);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);

    let _ = sim.object_conceal(1);
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);

    insert_entity(&mut sim, 2, EntityCategory::Infantry);
    let _ = sim.try_reveal_entity(2, common_raw_request(3, 4, 0x80, 192, 64));

    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x04);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
}

#[test]
fn gsi_04_12_common_raw_occupation_signed_ground_pins_mark_and_height_clear() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 0xFE, Some((3, 4)));
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 2, 128, 128));

    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0x20);

    let _ = sim.object_conceal(1);

    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
}

#[test]
fn gsi_04_12_common_raw_occupation_high_nonstructural_unit_keeps_native_stale_ground_bit() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 2, None);
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 6, 128, 128));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x20);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);

    let _ = sim.object_conceal(1);

    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(3, 4),
        0x20,
        "height-only clear targets deck even though nonstructural mark used ground"
    );
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
}

#[test]
fn gsi_04_12_common_raw_occupation_infantry_masks_follow_coordinates_and_never_bit_02() {
    let mut sim = Simulation::new();
    let cases = [
        (1, 1, 1, 128, 128, 0x01),
        (2, 2, 1, 64, 64, 0x01),
        (3, 3, 1, 192, 64, 0x04),
        (4, 4, 1, 64, 192, 0x08),
        (5, 5, 1, 192, 192, 0x10),
        (6, 6, 1, 128, 64, 0x01),
        (7, 7, 1, 192, 128, 0x04),
        (8, 8, 1, 128, 192, 0x08),
        (9, 9, 1, 64, 128, 0x01),
    ];

    for &(stable_id, rx, ry, sub_x, sub_y, expected_mask) in &cases {
        insert_entity(&mut sim, stable_id, EntityCategory::Infantry);
        let _ = sim.try_reveal_entity(stable_id, common_raw_request(rx, ry, 0, sub_x, sub_y));
        let bits = sim.substrate.raw_cell_occupation.ground_bits(rx, ry);
        assert_eq!(bits, expected_mask, "coordinate ({sub_x},{sub_y})");
        assert_eq!(bits & 0x02, 0, "GetSubCell never produces raw bit 0x02");
    }
}

#[test]
fn gsi_04_12_common_raw_occupation_infantry_marks_after_link_but_conceal_leaves_stale_bit() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0, 192, 64));
    let linked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListLinked)
        .expect("Infantry list link");
    let marked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationMarked)
        .expect("Infantry raw mark");
    assert!(linked < marked);
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x04);

    sim.lifecycle_test_events.clear();
    let _ = sim.object_conceal(1);

    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x04);
    assert!(
        sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationListUnlinked)
    );
    assert!(
        !sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationCleared)
    );
}

#[test]
fn gsi_04_12_common_raw_occupation_infantry_above_deck_height_links_without_marking() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 1, Some((3, 4)));
    insert_entity(&mut sim, 1, EntityCategory::Infantry);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 6, 192, 64));

    assert!(sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
    assert!(
        sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationListLinked)
    );
    assert!(
        !sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationMarked)
    );
}

#[test]
fn gsi_04_12_common_raw_occupation_building_foundation_is_ground_only() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Structure);
    {
        let building = sim.substrate.entities.get_mut(1).expect("building");
        building.foundation = "2x2".to_string();
        building.on_bridge = true;
    }

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 9, 128, 128));

    for (rx, ry) in [(3, 4), (3, 5), (4, 4), (4, 5)] {
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(rx, ry), 0x80);
        assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(rx, ry), 0);
        assert_eq!(
            sim.substrate
                .occupancy
                .count_on_layer(rx, ry, MovementLayer::Bridge),
            1
        );
    }

    let _ = sim.object_conceal(1);
    for (rx, ry) in [(3, 4), (3, 5), (4, 4), (4, 5)] {
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(rx, ry), 0);
        assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(rx, ry), 0);
    }
}

#[test]
fn gsi_04_12_common_raw_occupation_skips_transport_and_airborne_entities() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate.entities.get_mut(1).unwrap().passenger_role =
        PassengerRole::Inside { transport_id: 99 };
    let _ = sim.try_reveal_entity(1, common_raw_request(2, 3, 0, 128, 128));
    assert!(!sim.substrate.occupancy.contains_entity(2, 3, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(2, 3), 0);

    insert_entity(&mut sim, 2, EntityCategory::Unit);
    let mut locomotor = LocomotorState::for_test_kind(LocomotorKind::Drive);
    locomotor.layer = MovementLayer::Air;
    sim.substrate.entities.get_mut(2).unwrap().locomotor = Some(locomotor);
    let _ = sim.try_reveal_entity(2, common_raw_request(5, 6, 0, 128, 128));
    assert!(!sim.substrate.occupancy.contains_entity(5, 6, 2));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(5, 6), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(5, 6), 0);
}

#[test]
fn gsi_04_12_object_raw_occupation_landed_fly_reveal_and_conceal_are_ordered() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 0, None);
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(0));

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0, 128, 128));

    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(3, 4, MovementLayer::Ground),
        1
    );
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x40);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
    let linked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListLinked)
        .expect("landed Fly list link");
    let marked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationMarked)
        .expect("landed Fly raw mark");
    assert!(linked < marked);

    sim.lifecycle_test_events.clear();
    let _ = sim.object_conceal(1);

    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    let unlinked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .expect("landed Fly list unlink");
    let cleared = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationCleared)
        .expect("landed Fly raw clear");
    assert!(unlinked < cleared);
}

#[test]
fn gsi_04_12_object_raw_occupation_airborne_and_non_fly_aircraft_are_excluded() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 0, None);
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(1));
    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0, 128, 128));

    insert_entity(&mut sim, 2, EntityCategory::Aircraft);
    sim.substrate.entities.get_mut(2).unwrap().locomotor =
        Some(LocomotorState::for_test_kind(LocomotorKind::Rocket));
    let _ = sim.try_reveal_entity(2, common_raw_request(5, 6, 0, 128, 128));

    for (stable_id, rx, ry) in [(1, 3, 4), (2, 5, 6)] {
        assert!(!sim.substrate.occupancy.contains_entity(rx, ry, stable_id));
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(rx, ry), 0);
        assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(rx, ry), 0);
    }
}

#[test]
fn gsi_04_12_object_raw_occupation_deck_clear_rechecks_live_structural_state() {
    let mut sim = Simulation::new();
    // Signed level -2 makes z=2 exactly four normalized levels above ground.
    install_common_raw_terrain(&mut sim, 8, 8, 0xFE, Some((3, 4)));
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(0));
    sim.substrate.entities.get_mut(1).unwrap().on_bridge = true;

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 2, 128, 128));
    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(3, 4, MovementLayer::Bridge),
        1
    );
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0x40);

    let _ = sim.object_conceal(1);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);

    install_fly_aircraft(&mut sim, 2, SimFixed::from_num(0));
    sim.substrate.entities.get_mut(2).unwrap().on_bridge = true;
    let _ = sim.try_reveal_entity(2, common_raw_request(3, 4, 2, 128, 128));
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0x40);

    {
        let terrain = sim.resolved_terrain.as_ref().expect("resolved terrain");
        let bridge_state = sim.bridge_state.as_mut().expect("bridge runtime state");
        assert!(matches!(
            bridge_state.body_cell_advance_state(3, 4, true, terrain),
            StateOutcome::Absorbed
        ));
        assert!(matches!(
            bridge_state.body_cell_advance_state(3, 4, true, terrain),
            StateOutcome::Collapsed { .. }
        ));
        assert!(!bridge_state.is_bridge_walkable(3, 4));
    }

    let _ = sim.object_conceal(2);
    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 2));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(
        sim.substrate.raw_cell_occupation.deck_bits(3, 4),
        0x40,
        "generic clear retargets ground after collapse and leaves the deck bit stale"
    );
}

#[test]
fn gsi_04_12_object_raw_occupation_production_fly_tick_unmarks_takeoff_and_marks_landing() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 0, None);
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(0));
    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0, 128, 128));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x40);

    {
        let locomotor = sim
            .substrate
            .entities
            .get_mut(1)
            .unwrap()
            .locomotor
            .as_mut()
            .unwrap();
        locomotor.air_phase = AirMovePhase::Ascending;
        locomotor.target_altitude = SimFixed::from_num(600);
        locomotor.climb_rate = SimFixed::from_num(1500);
    }
    sim.tick_air_movement_with_cell_lists_one(1);

    let aircraft = sim.substrate.entities.get(1).unwrap();
    assert!(aircraft.locomotor.as_ref().unwrap().altitude > SimFixed::from_num(0));
    assert!(
        aircraft.lifecycle.cell_marked,
        "the post-process Mark transaction completed"
    );
    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);

    {
        let locomotor = sim
            .substrate
            .entities
            .get_mut(1)
            .unwrap()
            .locomotor
            .as_mut()
            .unwrap();
        locomotor.air_phase = AirMovePhase::Descending;
        locomotor.altitude = SimFixed::from_num(1);
        locomotor.target_altitude = SimFixed::from_num(0);
        locomotor.climb_rate = SimFixed::from_num(1500);
    }
    sim.tick_air_movement_with_cell_lists_one(1);

    let aircraft = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        aircraft.locomotor.as_ref().unwrap().altitude,
        SimFixed::from_num(0)
    );
    assert!(aircraft.lifecycle.cell_marked);
    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(3, 4, MovementLayer::Ground),
        1
    );
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x40);
}

fn insert_anim(sim: &mut Simulation, stable_id: u64, inactive: bool) {
    let type_id = sim.interner.intern("TESTANIM");
    let anim = AnimObject {
        stable_id,
        native_unique_id: stable_id as i32,
        type_id,
        world_coord: AnimWorldCoord { x: 0, y: 0, z: 0 },
        draw_flags: 0,
        z_adjust: 0,
        effective_end: 1,
        effective_loop_end: 1,
        runtime: AnimRuntime {
            current_frame: 0,
            frame_step: 1,
            delay_remaining: 0,
            rate_reload: 1,
            frame_timer: crate::sim::timer::CdTimer::started(0, 1),
            loop_remaining: 0,
            first_ai_guard: false,
            constructor_reverse: false,
            inactive,
        },
        in_logic_vector: false,
        owner_entity: None,
        start_sound_active: false,
        stop_sound_id: None,
    };
    assert!(sim.substrate.anims.insert(anim).is_none());
}

fn insert_particle_system(sim: &mut Simulation, stable_id: u64) {
    sim.substrate.particle_systems.insert(ParticleSystem {
        stable_id,
        in_logic_vector: false,
        type_id: crate::rules::particle_system_type::ParticleSystemTypeId(0),
        coords: IVec3::ZERO,
        offset: IVec3::ZERO,
        particles: Vec::new(),
        spawn_timer: SimFixed::from_num(0),
        lifetime: -1,
        spark_spawn_frames: 0,
        facing: 0,
        marked_for_deletion: false,
        directionless: true,
        attached_entity: None,
        owner_entity: None,
        target_coords: IVec3::ZERO,
        owner_house: None,
        done_spawning: false,
    });
}

#[test]
fn lifecycle_authority_reveal_commits_coords_then_marks_then_registers() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    assert_eq!(
        sim.try_reveal_entity(1, request(10, 20, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed {
            logic_registered: true
        }
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        (entity.position.rx, entity.position.ry, entity.position.z),
        (10, 20, 2)
    );
    assert_eq!(entity.position.sub_x, SimFixed::from_num(128));
    assert_eq!(entity.position.sub_y, SimFixed::from_num(64));
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(entity.in_logic_vector);
    assert!(sim.substrate.occupancy.contains_entity(10, 20, 1));
    assert_eq!(sim.live_object_order_snapshot(), vec![1]);

    let expected = [
        LifecycleTestEvent::RevealLimboCleared,
        LifecycleTestEvent::RevealCoordinatesCommitted,
        LifecycleTestEvent::MarkPut,
        LifecycleTestEvent::RawOccupationListLinked,
        LifecycleTestEvent::RawOccupationMarked,
        LifecycleTestEvent::CellMarked,
        LifecycleTestEvent::RevealDisplayBoundary,
        LifecycleTestEvent::LogicAppended,
        LifecycleTestEvent::LogicMembershipSet,
    ];
    assert_eq!(sim.lifecycle_test_events.as_slice(), expected.as_slice());
}

#[test]
fn reveal_stops_after_mark_when_object_is_not_alive() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .lifecycle
        .object_alive = false;

    assert_eq!(
        sim.try_reveal_entity(1, request(10, 20, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed {
            logic_registered: false
        }
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);
    assert!(sim.lifecycle_outputs.is_empty());
}

#[test]
fn lifecycle_authority_reveal_mark_failure_keeps_adjusted_coords_alive_limbo() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    assert_eq!(
        sim.try_reveal_entity(1, request(10, 20, PlacementEvidence::MarkFailed)),
        RevealOutcome::Failed(RevealFailure::MarkFailed)
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        (entity.position.rx, entity.position.ry, entity.position.z),
        (10, 20, 2)
    );
    assert!(entity.lifecycle.object_alive);
    assert!(entity.lifecycle.in_limbo);
    assert!(!entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::RevealLimboCleared,
            LifecycleTestEvent::RevealCoordinatesCommitted,
            LifecycleTestEvent::MarkPut,
        ]
    );
}

#[test]
fn lifecycle_authority_reveal_early_reject_commits_nothing() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let before = sim.substrate.entities.get(1).unwrap().position.clone();

    assert_eq!(
        sim.try_reveal_entity(1, request(10, 20, PlacementEvidence::RejectedEarly)),
        RevealOutcome::Failed(RevealFailure::RejectedEarly)
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (before.rx, before.ry)
    );
    assert!(entity.lifecycle.in_limbo);
    assert!(sim.lifecycle_test_events.is_empty());
}

#[test]
fn lifecycle_authority_reveal_rejects_an_already_marked_limbo_object() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let before = sim.substrate.entities.get(1).unwrap().position.clone();
    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .lifecycle
        .cell_marked = true;

    assert_eq!(
        sim.try_reveal_entity(1, request(10, 20, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Failed(RevealFailure::RejectedEarly)
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (before.rx, before.ry)
    );
    assert!(entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(sim.lifecycle_test_events.is_empty());
}

#[test]
fn lifecycle_authority_reveal_logic_failure_keeps_successful_mark() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate.logic.force_next_insert_failure_for_test();

    assert_eq!(
        sim.try_reveal_entity(1, request(10, 20, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed {
            logic_registered: false
        }
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);
    assert!(sim.substrate.occupancy.contains_entity(10, 20, 1));
}

#[test]
fn lifecycle_authority_logic_flag_sets_after_append() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);

    sim.register_live_object(1);
    assert_eq!(sim.live_object_order_snapshot(), vec![1]);
    assert!(sim.substrate.entities.get(1).unwrap().in_logic_vector);
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::LogicAppended,
            LifecycleTestEvent::LogicMembershipSet,
        ]
    );
}

#[test]
fn lifecycle_authority_logic_append_failure_leaves_flag_clear() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate.logic.force_next_insert_failure_for_test();

    sim.register_live_object(1);
    assert!(sim.live_object_order_snapshot().is_empty());
    assert!(!sim.substrate.entities.get(1).unwrap().in_logic_vector);
    assert!(sim.lifecycle_test_events.is_empty());
}

#[test]
fn lifecycle_authority_logic_remove_compacts_then_clears_flag() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    insert_entity(&mut sim, 3, EntityCategory::Unit);
    sim.register_live_object(1);
    sim.register_live_object(2);
    sim.register_live_object(3);

    sim.unregister_live_object(2);
    assert_eq!(sim.live_object_order_snapshot(), vec![1, 3]);
    assert!(sim.substrate.entities.get(1).unwrap().in_logic_vector);
    assert!(!sim.substrate.entities.get(2).unwrap().in_logic_vector);
    assert!(sim.substrate.entities.get(3).unwrap().in_logic_vector);
}

#[test]
fn lifecycle_authority_flagged_missing_remove_still_clears_flag() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate.entities.get_mut(1).unwrap().in_logic_vector = true;
    assert!(sim.live_object_order_snapshot().is_empty());

    sim.unregister_live_object(1);
    assert!(sim.live_object_order_snapshot().is_empty());
    assert!(!sim.substrate.entities.get(1).unwrap().in_logic_vector);
}

#[test]
fn particle_logic_membership_uses_the_object_local_guard_and_rebuilds_it() {
    let mut sim = Simulation::new();
    insert_particle_system(&mut sim, 7);

    assert!(sim.reveal_particle_system(7));
    assert!(
        sim.substrate
            .particle_systems
            .get(7)
            .unwrap()
            .in_logic_vector
    );
    assert_eq!(sim.live_object_order_snapshot(), vec![7]);

    assert!(sim.reveal_particle_system(7));
    assert_eq!(sim.live_object_order_snapshot(), vec![7]);

    sim.substrate
        .particle_systems
        .get_mut(7)
        .unwrap()
        .in_logic_vector = false;
    sim.rebuild_logic_membership();
    assert!(
        sim.substrate
            .particle_systems
            .get(7)
            .unwrap()
            .in_logic_vector
    );
    sim.debug_assert_logic_membership_consistent();

    assert!(sim.conceal_particle_system(7));
    assert!(
        !sim.substrate
            .particle_systems
            .get(7)
            .unwrap()
            .in_logic_vector
    );
    assert!(sim.live_object_order_snapshot().is_empty());
}

#[test]
fn open_topped_direct_registration_keeps_hidden_passenger_live() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    let _ = sim.reveal(1);
    let _ = sim.reveal(2);

    assert_eq!(sim.techno_limbo(1), super::ConcealOutcome::Concealed);
    sim.substrate.entities.get_mut(1).unwrap().passenger_role =
        PassengerRole::Inside { transport_id: 99 };

    assert!(sim.register_open_topped_passenger(1));
    let passenger = sim.substrate.entities.get(1).unwrap();
    assert!(passenger.lifecycle.object_alive);
    assert!(passenger.lifecycle.in_limbo);
    assert!(!passenger.lifecycle.cell_marked);
    assert!(passenger.in_logic_vector);
    assert_eq!(sim.live_object_order_snapshot(), vec![2, 1]);

    assert!(sim.register_open_topped_passenger(1));
    assert_eq!(sim.live_object_order_snapshot(), vec![2, 1]);
}

#[test]
fn lifecycle_authority_second_reveal_is_idempotent() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    assert_eq!(
        sim.try_reveal_entity(1, request(10, 20, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed {
            logic_registered: true
        }
    );
    let first_position = sim.substrate.entities.get(1).unwrap().position.clone();
    let first_enter_order = sim.substrate.entities.get(1).unwrap().occupancy_enter_order;
    sim.lifecycle_outputs.clear();
    sim.lifecycle_test_events.clear();

    assert_eq!(
        sim.try_reveal_entity(1, request(30, 40, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::AlreadyRevealed
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        (
            entity.position.rx,
            entity.position.ry,
            entity.position.z,
            entity.position.sub_x,
            entity.position.sub_y,
        ),
        (
            first_position.rx,
            first_position.ry,
            first_position.z,
            first_position.sub_x,
            first_position.sub_y,
        )
    );
    assert_eq!(entity.occupancy_enter_order, first_enter_order);
    assert!(sim.substrate.occupancy.contains_entity(10, 20, 1));
    assert!(!sim.substrate.occupancy.contains_entity(30, 40, 1));
    assert_eq!(sim.live_object_order_snapshot(), vec![1]);
    assert!(sim.lifecycle_outputs.is_empty());
    assert!(sim.lifecycle_test_events.is_empty());
}

#[test]
fn lifecycle_authority_conceal_without_dirty_eligibility_still_clears_drawn_state() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);
    sim.lifecycle_outputs.clear();

    let _ = sim.object_conceal(1);
    assert_eq!(
        sim.lifecycle_outputs,
        vec![
            LifecycleOutput::DisplayRemove { stable_id: 1 },
            LifecycleOutput::DetachAttachedAnims { stable_id: 1 },
            LifecycleOutput::StopVoc { stable_id: 1 },
            LifecycleOutput::ClearDrawnState { stable_id: 1 },
            LifecycleOutput::ClearRedraw { stable_id: 1 },
        ]
    );
}

#[test]
fn lifecycle_authority_conceal_dirty_eligibility_emits_dirty_before_drawn_clear() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .dirty_rect_eligible = true;
    let _ = sim.reveal(1);
    sim.lifecycle_outputs.clear();

    let _ = sim.object_conceal(1);
    assert_eq!(
        &sim.lifecycle_outputs[3..],
        &[
            LifecycleOutput::DirtyTacticalRect { stable_id: 1 },
            LifecycleOutput::ClearDrawnState { stable_id: 1 },
            LifecycleOutput::ClearRedraw { stable_id: 1 },
        ]
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(entity.lifecycle.object_alive);
    assert!(entity.lifecycle.in_limbo);
    assert!(!entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);
}

#[test]
fn lifecycle_authority_conceal_deselects_unmarks_unregisters_then_sets_limbo() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.selected = true;
    entity.dirty_rect_eligible = true;
    sim.lifecycle_test_events.clear();

    let _ = sim.object_conceal(1);
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::ConcealDeselected,
            LifecycleTestEvent::RawOccupationListUnlinked,
            LifecycleTestEvent::RawOccupationCleared,
            LifecycleTestEvent::ConcealUnmarked,
            LifecycleTestEvent::ConcealDisplayBoundary,
            LifecycleTestEvent::ConcealAnimBoundary,
            LifecycleTestEvent::ConcealVocBoundary,
            LifecycleTestEvent::ConcealLogicRemoved,
            LifecycleTestEvent::ConcealDirtyTacticalRectBoundary,
            LifecycleTestEvent::ConcealClearDrawnStateBoundary,
            LifecycleTestEvent::ConcealLimboSet,
            LifecycleTestEvent::ConcealClearRedrawBoundary,
        ]
    );
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(!entity.selected);
    assert!(!entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);
    assert!(entity.lifecycle.in_limbo);
}

#[test]
fn gsi_04_05_conceal_removes_only_selected_object_list_layer() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);

    let (rx, ry) = {
        let entity = sim.substrate.entities.get(1).unwrap();
        (entity.position.rx, entity.position.ry)
    };
    sim.substrate.occupancy.add(
        rx,
        ry,
        1,
        MovementLayer::Bridge,
        None,
        CellListInsertion::PrependNonBuilding,
    );
    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(rx, ry, MovementLayer::Ground),
        1
    );
    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(rx, ry, MovementLayer::Bridge),
        1
    );

    let _ = sim.object_conceal(1);

    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(rx, ry, MovementLayer::Ground),
        0,
        "conceal must unlink the current ground-list entry"
    );
    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(rx, ry, MovementLayer::Bridge),
        1,
        "conceal must not cross-scan the bridge object list"
    );
    assert!(!sim.substrate.entities.get(1).unwrap().lifecycle.cell_marked);
}

#[test]
fn gsi_04_05_hard_limbo_clears_pending_then_current_vehicle_occupation() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);
    let current = {
        let entity = sim.substrate.entities.get(1).unwrap();
        (entity.position.rx, entity.position.ry)
    };
    let head = (current.0 + 1, current.1);
    {
        let entity = sim.substrate.entities.get_mut(1).unwrap();
        entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
        entity.drive_locomotion = Some(DriveLocomotionRuntime {
            occupation_head_to: Some(DriveOccupationFootprint {
                rx: head.0,
                ry: head.1,
                layer: MovementLayer::Ground,
            }),
            ..Default::default()
        });
    }
    sim.substrate
        .cell_occupation
        .mark_vehicle_on_layer(head.0, head.1, 1, MovementLayer::Ground);

    let _ = sim.object_conceal(1);

    assert_eq!(
        sim.substrate
            .cell_occupation
            .vehicle_bits(head.0, head.1, MovementLayer::Ground),
        0
    );
    assert_eq!(
        sim.substrate
            .cell_occupation
            .vehicle_bits(current.0, current.1, MovementLayer::Ground),
        0
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .drive_locomotion
            .as_ref()
            .unwrap()
            .occupation_head_to,
        None
    );
}

#[test]
fn lifecycle_authority_conceal_outputs_match_release_order() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .dirty_rect_eligible = true;
    let _ = sim.reveal(1);
    sim.lifecycle_outputs.clear();

    let _ = sim.object_conceal(1);
    assert_eq!(
        sim.lifecycle_outputs,
        vec![
            LifecycleOutput::DisplayRemove { stable_id: 1 },
            LifecycleOutput::DetachAttachedAnims { stable_id: 1 },
            LifecycleOutput::StopVoc { stable_id: 1 },
            LifecycleOutput::DirtyTacticalRect { stable_id: 1 },
            LifecycleOutput::ClearDrawnState { stable_id: 1 },
            LifecycleOutput::ClearRedraw { stable_id: 1 },
        ]
    );
}

#[test]
fn lifecycle_authority_conceal_keeps_object_alive() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);

    let _ = sim.object_conceal(1);
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(entity.lifecycle.object_alive);
    assert!(entity.lifecycle.in_limbo);
    assert!(!entity.dying);
    assert!(!sim.substrate.pending_delete.contains(&1));
}

#[test]
fn lifecycle_authority_techno_limbo_breaks_contacts_before_common_conceal() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    let _ = sim.reveal(1);
    let _ = sim.reveal(2);
    assert_eq!(
        sim.substrate
            .entities
            .get_mut(1)
            .unwrap()
            .radio_contacts
            .insert(2),
        Some(0)
    );
    assert_eq!(
        sim.substrate
            .entities
            .get_mut(2)
            .unwrap()
            .radio_contacts
            .insert(1),
        Some(0)
    );
    sim.lifecycle_test_events.clear();

    let _ = sim.techno_limbo(1);
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::BreakSlot {
                slot: 0,
                target: Some(2),
            },
            LifecycleTestEvent::BreakSenderCleared { target: 2 },
            LifecycleTestEvent::BreakReceiverClassEffect { target: 2 },
            LifecycleTestEvent::BreakReceiverCleared { target: 2 },
            LifecycleTestEvent::ConcealDeselected,
            LifecycleTestEvent::RawOccupationListUnlinked,
            LifecycleTestEvent::RawOccupationCleared,
            LifecycleTestEvent::ConcealUnmarked,
            LifecycleTestEvent::ConcealDisplayBoundary,
            LifecycleTestEvent::ConcealAnimBoundary,
            LifecycleTestEvent::ConcealVocBoundary,
            LifecycleTestEvent::ConcealLogicRemoved,
            LifecycleTestEvent::ConcealClearDrawnStateBoundary,
            LifecycleTestEvent::ConcealLimboSet,
            LifecycleTestEvent::ConcealClearRedrawBoundary,
        ]
    );
    assert!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .radio_contacts
            .is_empty()
    );
    assert!(
        sim.substrate
            .entities
            .get(2)
            .unwrap()
            .radio_contacts
            .is_empty()
    );
}

#[test]
fn lifecycle_authority_uninit_limbos_while_alive_then_clears_alive_then_queues() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);
    sim.lifecycle_test_events.clear();

    sim.uninit(1);
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(!entity.lifecycle.object_alive);
    assert!(entity.lifecycle.in_limbo);
    assert!(!entity.lifecycle.cell_marked);
    assert!(entity.dying);
    assert_eq!(sim.substrate.pending_delete, vec![1]);
    let limbo = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::ConcealLimboSet)
        .unwrap();
    let alive_clear = sim
        .lifecycle_test_events
        .iter()
        .position(|event| {
            matches!(
                *event,
                LifecycleTestEvent::UninitAliveCleared { stable_id: 1 }
            )
        })
        .unwrap();
    let queued = sim
        .lifecycle_test_events
        .iter()
        .position(|event| {
            matches!(
                *event,
                LifecycleTestEvent::PendingDeleteQueued { stable_id: 1 }
            )
        })
        .unwrap();
    assert!(limbo < alive_clear && alive_clear < queued);
}

#[test]
fn lifecycle_authority_uninit_removal_boundary_sees_alive_marked_target() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);
    sim.lifecycle_test_events.clear();

    sim.uninit(1);
    let removal = sim
        .lifecycle_test_events
        .iter()
        .position(|event| {
            matches!(
                *event,
                LifecycleTestEvent::UninitRemovalNotifyBoundary {
                    stable_id: 1,
                    object_alive: true,
                    cell_marked: true,
                }
            )
        })
        .expect("removal boundary");
    let conceal = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::ConcealDeselected)
        .expect("common Conceal");
    assert!(removal < conceal);
}

#[test]
fn lifecycle_authority_uninit_dead_limbo_remains_resolvable_until_drain() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);

    sim.uninit(1);
    let entity = sim
        .substrate
        .entities
        .get(1)
        .expect("dead object remains stored until the ordinary drain");
    assert!(!entity.lifecycle.object_alive);
    assert!(entity.lifecycle.in_limbo);
    assert!(!entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);
    assert_eq!(sim.substrate.pending_delete, vec![1]);

    sim.process_pending_delete();
    assert!(!sim.substrate.entities.contains(1));
    assert!(sim.substrate.pending_delete.is_empty());
}

#[test]
fn lifecycle_authority_transport_uninits_passengers_in_cargo_order_before_carrier_notify() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Infantry);
    insert_entity(&mut sim, 3, EntityCategory::Infantry);
    let _ = sim.reveal(1);
    let mut cargo = PassengerCargo::new(5, 1);
    cargo.board_forced(2, 1);
    cargo.board_forced(3, 1);
    sim.substrate.entities.get_mut(1).unwrap().passenger_role = PassengerRole::Transport { cargo };
    sim.substrate.entities.get_mut(2).unwrap().passenger_role =
        PassengerRole::Inside { transport_id: 1 };
    sim.substrate.entities.get_mut(3).unwrap().passenger_role =
        PassengerRole::Inside { transport_id: 1 };
    sim.lifecycle_test_events.clear();

    sim.uninit(1);
    assert_eq!(sim.substrate.pending_delete, vec![2, 3, 1]);
    let notify_order = sim
        .lifecycle_test_events
        .iter()
        .filter_map(|event| match event {
            LifecycleTestEvent::UninitRemovalNotifyBoundary { stable_id, .. } => Some(*stable_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(notify_order, vec![2, 3, 1]);
    assert!(matches!(
        sim.substrate.entities.get(2).unwrap().passenger_role,
        PassengerRole::None
    ));
    assert!(matches!(
        sim.substrate.entities.get(3).unwrap().passenger_role,
        PassengerRole::None
    ));
    let cargo = sim
        .substrate
        .entities
        .get(1)
        .unwrap()
        .passenger_role
        .cargo()
        .unwrap();
    assert!(cargo.passengers.is_empty());
    assert!(cargo.passenger_sizes.is_empty());
    assert_eq!(cargo.total_size, 0);
    assert_eq!(cargo.garrison_fire_index, 0);
}

#[test]
fn uninit_pointer_expiry_walks_global_object_order_before_break_and_conceal() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_anim(&mut sim, 2, false);
    insert_particle_system(&mut sim, 3);
    insert_entity(&mut sim, 4, EntityCategory::Unit);
    let _ = sim.reveal(1);
    let _ = sim.reveal(4);
    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .radio_contacts
        .insert(4);
    sim.substrate
        .entities
        .get_mut(4)
        .unwrap()
        .radio_contacts
        .insert(1);
    sim.lifecycle_test_events.clear();

    sim.uninit(4);

    let visited = sim
        .lifecycle_test_events
        .iter()
        .filter_map(|event| match event {
            LifecycleTestEvent::UninitRemovalListenerVisited {
                expired_id: 4,
                listener_id,
                target_alive,
                target_in_limbo,
            } => Some((*listener_id, *target_alive, *target_in_limbo)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visited,
        vec![
            (1, true, false),
            (2, true, false),
            (3, true, false),
            (4, true, false),
        ]
    );

    let last_listener = sim
        .lifecycle_test_events
        .iter()
        .rposition(|event| {
            matches!(
                event,
                LifecycleTestEvent::UninitRemovalListenerVisited { expired_id: 4, .. }
            )
        })
        .unwrap();
    let break_slot = sim
        .lifecycle_test_events
        .iter()
        .position(|event| {
            matches!(
                event,
                LifecycleTestEvent::BreakSlot {
                    slot: 0,
                    target: Some(1)
                }
            )
        })
        .unwrap();
    let conceal = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::ConcealDeselected)
        .unwrap();
    assert!(last_listener < break_slot && break_slot < conceal);
}

#[test]
fn pointer_expiry_clears_live_refs_and_preserves_retaliation_attacker() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    let _ = sim.try_reveal_entity(2, request(9, 11, PlacementEvidence::MarkSucceeded));

    let listener = sim.substrate.entities.get_mut(1).unwrap();
    listener.attack_target = Some(AttackTarget {
        target: TargetKind::Entity(2),
        cooldown_ticks: 0,
        burst_remaining: 0,
        burst_delay_ticks: 0,
        pending_infantry_fire: None,
    });
    listener.suspended_attack_target = Some(TargetKind::Entity(2));
    listener.navigation.suspended_nav_com = Some(NavTargetRef::Entity { id: 2 });
    listener.navigation.nav_com_aux = Some(NavTargetRef::Object { id: 2 });
    listener.navigation.nav_com = Some(NavTargetRef::Building { id: 2 });
    listener.navigation.nav_queue = vec![
        NavTargetRef::Entity { id: 2 },
        NavTargetRef::Cell { rx: 7, ry: 8 },
        NavTargetRef::Object { id: 2 },
    ];
    listener.capture_target = Some(2);
    listener.c4_plant = Some(C4PlantState {
        target_building_id: 2,
    });
    listener.last_attacker_id = Some(2);

    sim.uninit(2);

    let listener = sim.substrate.entities.get(1).unwrap();
    assert!(listener.attack_target.is_none());
    assert!(listener.suspended_attack_target.is_none());
    assert!(listener.navigation.suspended_nav_com.is_none());
    assert!(listener.navigation.nav_com_aux.is_none());
    assert!(listener.navigation.nav_com.is_none());
    assert_eq!(
        listener.navigation.nav_queue,
        vec![NavTargetRef::Cell { rx: 7, ry: 8 }]
    );
    assert!(listener.capture_target.is_none());
    assert!(listener.c4_plant.is_none());
    assert_eq!(listener.last_attacker_id, Some(2));
}

#[test]
fn occupier_capture_retains_live_non_selling_nav_target() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Structure);
    let _ = sim.try_reveal_entity(2, request(9, 11, PlacementEvidence::MarkSucceeded));
    sim.mission_assign_exact(
        1,
        MissionId::from_known(MissionType::Capture),
        sim.session.binary_frame,
    )
    .unwrap();

    let listener = sim.substrate.entities.get_mut(1).unwrap();
    listener.occupier = true;
    listener.navigation.suspended_nav_com = Some(NavTargetRef::Building { id: 2 });
    listener.navigation.nav_com_aux = Some(NavTargetRef::Building { id: 2 });
    listener.navigation.nav_com = Some(NavTargetRef::Building { id: 2 });
    listener.navigation.nav_queue = vec![NavTargetRef::Building { id: 2 }];

    sim.uninit(2);

    let listener = sim.substrate.entities.get(1).unwrap();
    assert!(listener.navigation.suspended_nav_com.is_none());
    assert_eq!(
        listener.navigation.nav_com_aux,
        Some(NavTargetRef::Building { id: 2 })
    );
    assert_eq!(
        listener.navigation.nav_com,
        Some(NavTargetRef::Building { id: 2 })
    );
    assert!(listener.navigation.nav_queue.is_empty());
}

#[test]
fn selling_target_disables_occupier_capture_nav_retention() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Structure);
    let _ = sim.try_reveal_entity(2, request(9, 11, PlacementEvidence::MarkSucceeded));
    sim.mission_assign_exact(
        1,
        MissionId::from_known(MissionType::Capture),
        sim.session.binary_frame,
    )
    .unwrap();
    sim.mission_assign_exact(
        2,
        MissionId::from_known(MissionType::Selling),
        sim.session.binary_frame,
    )
    .unwrap();

    let listener = sim.substrate.entities.get_mut(1).unwrap();
    listener.occupier = true;
    listener.navigation.nav_com_aux = Some(NavTargetRef::Building { id: 2 });
    listener.navigation.nav_com = Some(NavTargetRef::Building { id: 2 });

    sim.uninit(2);

    let listener = sim.substrate.entities.get(1).unwrap();
    assert!(listener.navigation.nav_com_aux.is_none());
    assert!(listener.navigation.nav_com.is_none());
}

#[test]
fn target_expiry_rearms_passive_scan_and_restores_suspended_mission() {
    let mut sim = Simulation::with_seed(0x1234);
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    insert_entity(&mut sim, 3, EntityCategory::Unit);
    let _ = sim.try_reveal_entity(2, request(9, 11, PlacementEvidence::MarkSucceeded));
    sim.session.binary_frame = 105;

    let listener = sim.substrate.entities.get_mut(1).unwrap();
    listener.attack_target = Some(AttackTarget::new(2));
    listener.suspended_attack_target = Some(TargetKind::Entity(3));
    listener.navigation.suspended_nav_com = Some(NavTargetRef::Cell { rx: 7, ry: 8 });
    listener.passive_scan_timer.arm(100, 20);
    listener.mission.apply_test_fixture(MissionTestFixture {
        current: MissionId::from_known(MissionType::Attack),
        suspended: MissionId::from_known(MissionType::Guard),
        queued: MissionId::NONE,
        movement_bypass_latch: 0,
        handler_state: 0,
        mission_start_frame: 100,
        ai_counter: 0,
        dispatch_timer: MissionDispatchTimer::at_frame(100),
    });

    let mut expected_rng = sim.scenario_rng.clone();
    let delay = expected_rng.next_range_u32_inclusive(4, 8);
    sim.uninit(2);

    let listener = sim.substrate.entities.get(1).unwrap();
    assert_eq!(listener.passive_scan_timer.start_frame, 105);
    assert_eq!(listener.passive_scan_timer.duration, delay);
    assert_eq!(
        listener.mission.current(),
        MissionId::from_known(MissionType::Guard)
    );
    assert_eq!(listener.mission.suspended(), MissionId::NONE);
    assert_eq!(
        listener.attack_target.as_ref().map(|target| target.target),
        Some(TargetKind::Entity(3))
    );
    assert_eq!(
        listener.navigation.nav_com,
        Some(NavTargetRef::Cell { rx: 7, ry: 8 })
    );
    assert_eq!(
        sim.scenario_rng.logical_state(),
        expected_rng.logical_state()
    );
}

#[test]
fn infantry_target_expiry_clears_firing_action_before_target() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    let _ = sim.try_reveal_entity(2, request(9, 11, PlacementEvidence::MarkSucceeded));

    let listener = sim.substrate.entities.get_mut(1).unwrap();
    listener.attack_target = Some(AttackTarget {
        target: TargetKind::Entity(2),
        cooldown_ticks: 9,
        burst_remaining: 3,
        burst_delay_ticks: 2,
        pending_infantry_fire: Some(PendingInfantryFire {
            sequence: SequenceKind::Attack,
            fire_frame: 4,
        }),
    });
    listener.animation = Some(Animation::new(SequenceKind::Attack));
    listener.mission_leaf = crate::sim::mission::MissionLeafState::infantry_raw_for_test(7, 12);

    sim.uninit(2);

    let listener = sim.substrate.entities.get(1).unwrap();
    assert!(listener.attack_target.is_none());
    assert_eq!(
        listener.animation.as_ref().unwrap().sequence,
        SequenceKind::Stand
    );
    let leaf = listener.mission_leaf.as_infantry().unwrap();
    assert_eq!(leaf.firing_sequence_latch(), 0);
    assert_eq!(leaf.doing(), -1);
}

#[test]
fn pointer_expiry_clears_particle_owner_and_deletes_attached_system() {
    let mut sim = Simulation::new();
    insert_particle_system(&mut sim, 1);
    insert_particle_system(&mut sim, 2);
    insert_entity(&mut sim, 3, EntityCategory::Unit);
    let _ = sim.reveal(3);
    sim.substrate
        .particle_systems
        .get_mut(1)
        .unwrap()
        .owner_entity = Some(3);
    let attached = sim.substrate.particle_systems.get_mut(2).unwrap();
    attached.owner_entity = Some(3);
    attached.attached_entity = Some(3);

    sim.uninit(3);

    let owned = sim.substrate.particle_systems.get(1).unwrap();
    assert!(owned.owner_entity.is_none());
    assert!(!owned.marked_for_deletion);
    let attached = sim.substrate.particle_systems.get(2).unwrap();
    assert!(attached.owner_entity.is_none());
    assert!(attached.attached_entity.is_none());
    assert!(attached.marked_for_deletion);
}

#[test]
fn homing_target_expiry_uses_ground_cell_but_nulls_high_flying_target() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    insert_entity(&mut sim, 3, EntityCategory::Unit);
    insert_entity(&mut sim, 4, EntityCategory::Aircraft);
    let _ = sim.try_reveal_entity(2, request(9, 11, PlacementEvidence::MarkSucceeded));
    let _ = sim.try_reveal_entity(4, request(13, 15, PlacementEvidence::MarkSucceeded));
    let mut high_locomotor = LocomotorState::for_test_kind(LocomotorKind::Fly);
    high_locomotor.altitude = SimFixed::from_num(208);
    sim.substrate.entities.get_mut(4).unwrap().locomotor = Some(high_locomotor);
    assert!(attach_homing_state(
        &mut sim.substrate.entities,
        1,
        (2, 3),
        2,
        (9, 11),
        SimFixed::from_num(10),
        5,
        0,
        false,
        false,
        SimFixed::from_num(1),
    ));
    assert!(attach_homing_state(
        &mut sim.substrate.entities,
        3,
        (2, 3),
        4,
        (13, 15),
        SimFixed::from_num(10),
        5,
        0,
        false,
        false,
        SimFixed::from_num(1),
    ));

    sim.uninit(2);
    sim.uninit(4);

    assert_eq!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .homing_state
            .as_ref()
            .unwrap()
            .target,
        Some(HomingTarget::Cell { rx: 9, ry: 11 })
    );
    assert!(
        sim.substrate
            .entities
            .get(3)
            .unwrap()
            .homing_state
            .as_ref()
            .unwrap()
            .target
            .is_none()
    );
}

#[test]
fn expiring_mixed_size_passenger_updates_transport_total_exactly() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Infantry);
    insert_entity(&mut sim, 3, EntityCategory::Infantry);
    let mut cargo = PassengerCargo::new(5, 0);
    cargo.board_forced(2, 3);
    cargo.board_forced(3, 1);
    sim.substrate.entities.get_mut(1).unwrap().passenger_role = PassengerRole::Transport { cargo };
    sim.substrate.entities.get_mut(2).unwrap().passenger_role =
        PassengerRole::Inside { transport_id: 1 };
    sim.substrate.entities.get_mut(3).unwrap().passenger_role =
        PassengerRole::Inside { transport_id: 1 };

    sim.uninit(2);

    let cargo = sim
        .substrate
        .entities
        .get(1)
        .unwrap()
        .passenger_role
        .cargo()
        .unwrap();
    assert_eq!(cargo.passengers, vec![3]);
    assert_eq!(cargo.passenger_sizes, vec![1]);
    assert_eq!(cargo.total_size, 1);
}

#[test]
fn lifecycle_authority_duplicate_uninit_does_not_double_release_owned_count() {
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 0, 10));
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.houses.get_mut(&owner).unwrap().owned_unit_count = 2;

    sim.uninit(1);
    sim.uninit(1);
    assert_eq!(sim.houses.get(&owner).unwrap().owned_unit_count, 1);
    assert!(sim.substrate.entities.get(1).unwrap().owned_count_released);
    assert_eq!(sim.substrate.pending_delete, vec![1, 1]);
    assert_eq!(
        sim.lifecycle_test_events
            .iter()
            .filter(|event| matches!(
                event,
                LifecycleTestEvent::UninitRemovalNotifyBoundary { stable_id: 1, .. }
            ))
            .count(),
        2
    );

    sim.lifecycle_test_events.clear();
    sim.process_pending_delete();
    assert!(!sim.substrate.entities.contains(1));
    assert!(sim.substrate.pending_delete.is_empty());
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::PendingDeleteDrainStarted,
            LifecycleTestEvent::FinalizedCommon { stable_id: 1 },
        ]
    );
}

#[test]
fn lifecycle_authority_immediate_uninit_releases_owned_count_once() {
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 0, 10));
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.houses.get_mut(&owner).unwrap().owned_unit_count = 2;

    sim.uninit(1);
    assert_eq!(sim.houses.get(&owner).unwrap().owned_unit_count, 1);
    assert!(sim.substrate.entities.get(1).unwrap().owned_count_released);
}

#[test]
fn score_stats_credit_the_killer_and_charge_the_victim_once() {
    let mut sim = Simulation::new();
    let victim_owner = sim.interner.intern("Americans");
    let killer_owner = sim.interner.intern("Russians");
    sim.houses.insert(
        victim_owner,
        HouseState::new(victim_owner, 0, None, true, 0, 10),
    );
    sim.houses.insert(
        killer_owner,
        HouseState::new(killer_owner, 1, None, false, 0, 10),
    );
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let victim = sim.substrate.entities.get_mut(1).unwrap();
    victim.health.current = 0;
    victim.killed_by = Some(killer_owner);
    // A stock Rhino (HTNK) is Cost=900, and the award is the victim's cost.
    victim.kill_award_points = 900;

    // A repeated uninit must not double-count: the exactly-once owned-count
    // guard covers the statistics too.
    sim.uninit(1);
    sim.uninit(1);

    assert_eq!(sim.houses.get(&victim_owner).unwrap().stats.losses(), 1);
    assert_eq!(sim.houses.get(&victim_owner).unwrap().stats.kills(), 0);
    let killer = sim.houses.get(&killer_owner).unwrap();
    assert_eq!(killer.stats.kills(), 1);
    assert_eq!(
        killer.stats.units_killed, 1,
        "a unit victim must land in the unit bucket, not the building one"
    );
    assert_eq!(killer.stats.score_points, 900);
}

#[test]
fn score_stats_survive_the_retaliation_pass_clearing_last_attacker() {
    // Dying infantry linger in the logic vector, so the retaliation pass wipes
    // `last_attacker_id` before they are removed. The kill record is captured at
    // the instant of destruction and must not depend on that field.
    let mut sim = Simulation::new();
    let victim_owner = sim.interner.intern("Americans");
    let killer_owner = sim.interner.intern("Russians");
    sim.houses.insert(
        victim_owner,
        HouseState::new(victim_owner, 0, None, true, 0, 10),
    );
    sim.houses.insert(
        killer_owner,
        HouseState::new(killer_owner, 1, None, false, 0, 10),
    );
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    let victim = sim.substrate.entities.get_mut(1).unwrap();
    victim.health.current = 0;
    victim.killed_by = Some(killer_owner);
    // A stock GI (E1) is Cost=200.
    victim.kill_award_points = 200;
    victim.last_attacker_id = None;

    sim.uninit(1);

    assert_eq!(sim.houses.get(&killer_owner).unwrap().stats.kills(), 1);
    assert_eq!(
        sim.houses.get(&killer_owner).unwrap().stats.score_points,
        200
    );
}

#[test]
fn score_stats_ignore_a_removal_that_was_not_a_destruction() {
    // Selling or otherwise despawning a healthy object is not a loss.
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 0, 10));
    insert_entity(&mut sim, 1, EntityCategory::Structure);
    sim.substrate.entities.get_mut(1).unwrap().health.current = 400;

    sim.uninit(1);

    assert_eq!(sim.houses.get(&owner).unwrap().stats.losses(), 0);
}

#[test]
fn score_stats_count_a_self_inflicted_kill_but_award_no_points() {
    // Self-inflicted destruction (own death weapon, own splash) is both a loss
    // and a kill for the house â€” native increments the kill table regardless of
    // relation and suppresses only the points.
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 0, 10));
    insert_entity(&mut sim, 1, EntityCategory::Structure);
    let victim = sim.substrate.entities.get_mut(1).unwrap();
    victim.health.current = 0;
    victim.killed_by = Some(owner);
    victim.kill_award_points = 800;

    sim.uninit(1);

    let house = sim.houses.get(&owner).unwrap();
    assert_eq!(house.stats.buildings_lost, 1);
    assert_eq!(house.stats.buildings_killed, 1);
    assert_eq!(
        house.stats.score_points, 0,
        "an allied or self-inflicted victim is worth no score"
    );
}

#[test]
fn dont_score_victims_book_no_kill_no_loss_and_no_points() {
    // Stock `DontScore=yes` covers SLAV and the three spawner missiles
    // (V3ROCKET/DMISL/CMISL). Slaves die and respawn all match and AA intercepts
    // are routine, so a victim that native ignores entirely must not show up in
    // any of the three columns.
    let mut sim = Simulation::new();
    let victim_owner = sim.interner.intern("Americans");
    let killer_owner = sim.interner.intern("Russians");
    sim.houses.insert(
        victim_owner,
        HouseState::new(victim_owner, 0, None, true, 0, 10),
    );
    sim.houses.insert(
        killer_owner,
        HouseState::new(killer_owner, 1, None, false, 0, 10),
    );
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    let victim = sim.substrate.entities.get_mut(1).unwrap();
    victim.health.current = 0;
    victim.dont_score = true;
    // Even with a credit already recorded, the loss half stays suppressed.
    victim.killed_by = Some(killer_owner);
    victim.kill_award_points = 500;

    sim.uninit(1);

    let victim_house = sim.houses.get(&victim_owner).unwrap();
    assert_eq!(victim_house.stats.losses(), 0, "no phantom loss");
    let killer = sim.houses.get(&killer_owner).unwrap();
    assert_eq!(killer.stats.kills(), 0, "no phantom kill");
    assert_eq!(killer.stats.score_points, 0, "no phantom points");
}

#[test]
fn score_column_sums_the_harvest_and_kill_feeders() {
    // The native score field has two feeders. Drive the kill half through the
    // real award helper on stock costs rather than a hand-picked total: one
    // Rhino (Cost=900) plus one veteran GI (Cost=200, doubled).
    use crate::rules::ini_parser::IniFile;
    use crate::rules::object_type::{ObjectCategory, ObjectType};
    use crate::sim::combat::score_award_for_victim;
    use crate::sim::house_state::MatchStatistics;

    let of = |body: &str, category| {
        let ini = IniFile::from_str(&format!(
            "[T]
{body}"
        ));
        ObjectType::from_ini_section("T", ini.section("T").expect("section"), category)
    };
    let rhino = of(
        "Cost=900
",
        ObjectCategory::Vehicle,
    );
    let gi = of(
        "Cost=200
",
        ObjectCategory::Infantry,
    );

    let kill_half =
        score_award_for_victim(Some(&rhino), 0) + score_award_for_victim(Some(&gi), 100);
    assert_eq!(kill_half, 1_300);

    let stats = MatchStatistics {
        score_points: kill_half,
        ..Default::default()
    };
    // Harvest half: 240 bales deposited at the x5.0 statistics rate.
    assert_eq!(stats.score(1_200), 2_500);
}

#[test]
fn lifecycle_authority_animated_death_stays_represented_until_uninit() {
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 0, 10));
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.houses.get_mut(&owner).unwrap().owned_unit_count = 2;
    let _ = sim.reveal(1);

    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.health.current = 0;
    entity.dying = true;
    assert_eq!(sim.houses.get(&owner).unwrap().owned_unit_count, 2);
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(!entity.owned_count_released);
    assert!(entity.lifecycle.object_alive);
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(entity.in_logic_vector);

    sim.uninit(1);
    assert_eq!(sim.houses.get(&owner).unwrap().owned_unit_count, 1);
    assert!(
        !sim.substrate
            .entities
            .get(1)
            .unwrap()
            .lifecycle
            .object_alive
    );
    assert!(!sim.substrate.entities.get(1).unwrap().in_logic_vector);
}

#[test]
fn lifecycle_authority_duplicate_queue_finalizes_once() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.uninit(1);
    sim.uninit(1);
    assert_eq!(sim.substrate.pending_delete, vec![1, 1]);
    sim.lifecycle_test_events.clear();

    sim.process_pending_delete();
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::PendingDeleteDrainStarted,
            LifecycleTestEvent::FinalizedCommon { stable_id: 1 },
        ]
    );
    assert!(!sim.substrate.entities.contains(1));
    assert!(sim.substrate.pending_delete.is_empty());
}

#[test]
fn lifecycle_authority_entity_and_anim_share_ordered_drain() {
    let mut sim = Simulation::new();
    insert_anim(&mut sim, 2, true);
    sim.substrate.pending_delete.push(2);
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.uninit(1);
    assert_eq!(sim.substrate.pending_delete, vec![2, 1]);
    sim.lifecycle_test_events.clear();

    sim.process_pending_delete();
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::PendingDeleteDrainStarted,
            LifecycleTestEvent::FinalizedCommon { stable_id: 2 },
            LifecycleTestEvent::FinalizedCommon { stable_id: 1 },
        ]
    );
    assert!(!sim.substrate.anims.contains_key(2));
    assert!(!sim.substrate.entities.contains(1));
    assert!(sim.substrate.pending_delete.is_empty());
}

#[test]
fn lifecycle_authority_late_tail_commits_frame_before_drain() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.uninit(1);
    sim.lifecycle_test_events.clear();

    let height_map = BTreeMap::new();
    let _ = sim.advance_tick(&[], None, &height_map, None, None, 67);
    assert_eq!(
        sim.lifecycle_test_events,
        vec![
            LifecycleTestEvent::BinaryFrameCommitted,
            LifecycleTestEvent::PendingDeleteDrainStarted,
            LifecycleTestEvent::FinalizedCommon { stable_id: 1 },
        ]
    );
    assert_eq!(sim.session.binary_frame, 1);
    assert!(!sim.substrate.entities.contains(1));
}

#[test]
fn lifecycle_authority_set_logic_order_for_test_synchronizes_all_membership_flags() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    insert_anim(&mut sim, 3, false);
    insert_particle_system(&mut sim, 4);

    sim.set_logic_order_for_test(vec![3, 4, 1]);
    assert_eq!(sim.live_object_order_snapshot(), vec![3, 4, 1]);
    assert!(sim.substrate.anims.get(3).unwrap().in_logic_vector);
    assert!(
        sim.substrate
            .particle_systems
            .get(4)
            .unwrap()
            .in_logic_vector
    );
    assert!(sim.substrate.entities.get(1).unwrap().in_logic_vector);
    assert!(!sim.substrate.entities.get(2).unwrap().in_logic_vector);
    sim.debug_assert_logic_membership_consistent();

    sim.set_logic_order_for_test(vec![2]);
    assert_eq!(sim.live_object_order_snapshot(), vec![2]);
    assert!(!sim.substrate.anims.get(3).unwrap().in_logic_vector);
    assert!(
        !sim.substrate
            .particle_systems
            .get(4)
            .unwrap()
            .in_logic_vector
    );
    assert!(!sim.substrate.entities.get(1).unwrap().in_logic_vector);
    assert!(sim.substrate.entities.get(2).unwrap().in_logic_vector);
    sim.debug_assert_logic_membership_consistent();
}

#[test]
fn lifecycle_authority_alive_queued_object_remains_queued() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.substrate.pending_delete.push(1);

    sim.process_pending_delete();
    assert!(sim.substrate.entities.contains(1));
    assert_eq!(sim.substrate.pending_delete, vec![1]);
}

fn attack_fixture(current: MissionType, suspended: MissionId) -> MissionTestFixture {
    MissionTestFixture {
        current: MissionId::from_known(current),
        suspended,
        queued: MissionId::NONE,
        movement_bypass_latch: 0,
        handler_state: 0,
        mission_start_frame: 0,
        ai_counter: 0,
        dispatch_timer: MissionDispatchTimer::at_frame(0),
    }
}

/// Two attackers share one target that then leaves play ALIVE â€” sold, captured,
/// mind-controlled or teleported. Nothing dies, so the pointer-expiry broadcast
/// never runs and this sweep is the only thing that releases them.
///
/// Pins the three clauses that make the detach sweep different from the expiry
/// one: the Restore runs before the target clear, the clear is skipped when the
/// Restore replaced the target, and the walk is descending.
#[test]
fn detach_sweep_restores_before_clearing_target_in_descending_id_order() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Infantry);
    insert_entity(&mut sim, 3, EntityCategory::Structure);
    insert_entity(&mut sim, 4, EntityCategory::Unit);

    for attacker in [1, 2] {
        let entity = sim.substrate.entities.get_mut(attacker).unwrap();
        entity.attack_target = Some(AttackTarget::new(3));
        entity.suspended_attack_target = Some(TargetKind::Entity(4));
        entity.navigation.nav_com = None;
        entity.navigation.suspended_nav_com = Some(NavTargetRef::Cell { rx: 7, ry: 8 });
        entity.mission.apply_test_fixture(attack_fixture(
            MissionType::Attack,
            MissionId::from_known(MissionType::Move),
        ));
    }
    sim.lifecycle_test_events.clear();

    sim.stop_all_targeting_on_detach(3);

    // The detaching object is untouched and still present: this is not removal.
    assert!(sim.substrate.entities.contains(3));

    for attacker in [1, 2] {
        let entity = sim.substrate.entities.get(attacker).unwrap();
        assert_eq!(
            entity.mission.current(),
            MissionId::from_known(MissionType::Move),
            "attacker {attacker} restored its suspended mission"
        );
        assert_eq!(entity.mission.suspended(), MissionId::NONE);
        // Restore-then-clear: the archived target is reinstalled and the
        // null-out is skipped because it no longer matches the detaching object.
        assert_eq!(
            entity.attack_target.as_ref().map(|target| target.target),
            Some(TargetKind::Entity(4)),
            "attacker {attacker} kept the target the Restore installed"
        );
        assert_eq!(
            entity.navigation.nav_com,
            Some(NavTargetRef::Cell { rx: 7, ry: 8 }),
            "attacker {attacker} got its destination back"
        );
    }

    let visits: Vec<(u64, bool, bool)> = sim
        .lifecycle_test_events
        .iter()
        .filter_map(|event| match event {
            LifecycleTestEvent::DetachTargetingSweepVisited {
                detach_id: 3,
                listener_id,
                restored,
                target_cleared,
            } => Some((*listener_id, *restored, *target_cleared)),
            _ => None,
        })
        .collect();
    assert_eq!(
        visits,
        vec![(2, true, false), (1, true, false)],
        "the sweep walks descending stable-ID order"
    );
}

/// The other half of the sweep: an attacker with no suspended mission. The
/// Restore writes nothing, so the conditional null-out is the clause that fires
/// and the attacker is left with no target â€” which is what hands it to the
/// Attack handler's idle exit on its next dispatch.
#[test]
fn detach_sweep_clears_target_when_no_mission_was_suspended() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Structure);

    let attacker = sim.substrate.entities.get_mut(1).unwrap();
    attacker.attack_target = Some(AttackTarget::new(2));
    attacker.passively_acquired_target = true;
    attacker
        .mission
        .apply_test_fixture(attack_fixture(MissionType::Attack, MissionId::NONE));
    sim.lifecycle_test_events.clear();

    sim.stop_all_targeting_on_detach(2);

    let attacker = sim.substrate.entities.get(1).unwrap();
    assert!(attacker.attack_target.is_none());
    assert!(!attacker.passively_acquired_target);
    assert_eq!(
        attacker.mission.current(),
        MissionId::from_known(MissionType::Attack),
        "a Restore with nothing suspended writes no mission field"
    );
    assert_eq!(attacker.mission.suspended(), MissionId::NONE);
    assert!(matches!(
        sim.lifecycle_test_events.as_slice(),
        [LifecycleTestEvent::DetachTargetingSweepVisited {
            detach_id: 2,
            listener_id: 1,
            restored: false,
            target_cleared: true,
        }]
    ));
}

/// A cell target is not an object pointer, so neither detach sweep can see it.
/// This is the recorded hole behind the wall arm of the blocked-step Override:
/// an object overridden onto a wall has no Restore route from here.
#[test]
fn detach_sweep_never_matches_a_cell_target() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Structure);

    let attacker = sim.substrate.entities.get_mut(1).unwrap();
    attacker.attack_target = Some(AttackTarget::for_cell(9, 11));
    attacker.mission.apply_test_fixture(attack_fixture(
        MissionType::Attack,
        MissionId::from_known(MissionType::Move),
    ));

    sim.stop_all_targeting_on_detach(2);

    let attacker = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        attacker.attack_target.as_ref().map(|target| target.target),
        Some(TargetKind::Cell(9, 11))
    );
    assert_eq!(
        attacker.mission.current(),
        MissionId::from_known(MissionType::Attack),
        "a cell-targeted object is never restored by the detach sweep"
    );
}

/// The whole loop end to end, in the shape the deferred item names: an
/// infantryman is overridden onto a blocker, the blocker LEAVES ALIVE (owner
/// change â€” engineer capture, Yuri, Psychic Beacon), and the infantryman comes
/// back to what it was doing instead of sitting on Attack forever.
#[test]
fn overridden_attacker_is_released_when_its_blocker_leaves_alive() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    let soviets = sim.interner.intern("Soviets");
    sim.substrate.entities.get_mut(2).unwrap().owner = soviets;

    let mover = sim.substrate.entities.get_mut(1).unwrap();
    mover.navigation.nav_com = Some(NavTargetRef::Cell { rx: 20, ry: 21 });
    mover
        .mission
        .apply_test_fixture(attack_fixture(MissionType::Move, MissionId::NONE));

    assert!(sim.mission_override_blocked_by_object(1, 2));

    let mover = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        mover.mission.current(),
        MissionId::from_known(MissionType::Attack)
    );
    assert_eq!(
        mover.mission.suspended(),
        MissionId::from_known(MissionType::Move)
    );
    assert_eq!(
        mover.attack_target.as_ref().map(|target| target.target),
        Some(TargetKind::Entity(2))
    );
    assert!(mover.navigation.nav_com.is_none(), "the mover stops");
    assert_eq!(
        mover.navigation.suspended_nav_com,
        Some(NavTargetRef::Cell { rx: 20, ry: 21 })
    );

    // The blocker changes hands and stays alive.
    let americans = sim.interner.intern("Americans");
    sim.change_owner(2, americans);

    assert!(sim.substrate.entities.contains(2), "the blocker is alive");
    let mover = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        mover.mission.current(),
        MissionId::from_known(MissionType::Move),
        "the mover goes back to its original order"
    );
    assert_eq!(mover.mission.suspended(), MissionId::NONE);
    assert_eq!(
        mover.navigation.nav_com,
        Some(NavTargetRef::Cell { rx: 20, ry: 21 }),
        "and re-paths to the destination it was archived with"
    );
    assert!(
        mover.attack_target.is_none(),
        "nothing was archived, so the conditional null-out fires"
    );
}

/// The ally arm: the blocked-step Override does not fire on a friendly blocker,
/// and it writes none of the five fields.
#[test]
fn blocked_step_override_does_not_fire_on_an_allied_blocker() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Unit);

    let mover = sim.substrate.entities.get_mut(1).unwrap();
    mover.navigation.nav_com = Some(NavTargetRef::Cell { rx: 20, ry: 21 });
    mover
        .mission
        .apply_test_fixture(attack_fixture(MissionType::Move, MissionId::NONE));

    // Same house: always an ally.
    assert!(!sim.mission_override_blocked_by_object(1, 2));

    let mover = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        mover.mission.current(),
        MissionId::from_known(MissionType::Move)
    );
    assert_eq!(mover.mission.suspended(), MissionId::NONE);
    assert!(mover.attack_target.is_none());
    assert_eq!(
        mover.navigation.nav_com,
        Some(NavTargetRef::Cell { rx: 20, ry: 21 })
    );
    assert!(mover.navigation.suspended_nav_com.is_none());
}

/// Two blocked steps against two different blockers with no Restore between
/// them: the second Override archives the FIRST override's mission, so a later
/// Restore lands the object back on Attack and the original order is lost.
/// Native, and it must survive the wiring â€” no caller-side clobber guard.
#[test]
fn second_blocked_step_override_clobbers_the_archived_mission() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Infantry);
    insert_entity(&mut sim, 2, EntityCategory::Unit);
    insert_entity(&mut sim, 3, EntityCategory::Unit);
    let soviets = sim.interner.intern("Soviets");
    for blocker in [2, 3] {
        sim.substrate.entities.get_mut(blocker).unwrap().owner = soviets;
    }
    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .mission
        .apply_test_fixture(attack_fixture(MissionType::Move, MissionId::NONE));

    assert!(sim.mission_override_blocked_by_object(1, 2));
    assert!(sim.mission_override_blocked_by_object(1, 3));

    let mover = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        mover.mission.suspended(),
        MissionId::from_known(MissionType::Attack),
        "the second Override archived the first Override's mission"
    );
    assert_eq!(
        mover.attack_target.as_ref().map(|target| target.target),
        Some(TargetKind::Entity(3))
    );
}
