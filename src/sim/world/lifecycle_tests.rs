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
use crate::sim::projectile::{
    ProjectileCollisionPolicy, ProjectileCoord, ProjectileGuidance, ProjectilePayload,
    ProjectileSpawn, ProjectileTarget, ProjectileTrajectory, ProjectileVelocity,
    ProjectileVisualState, TargetExpiryPolicy, cell_target_coord,
};
use crate::sim::snapshot::{GameSnapshot, SnapshotRestoreError};
use crate::sim::terrain_object::{TerrainObjectLifecycle, TerrainObjectState};
use crate::sim::wave::{Wave, WaveDamagePayload, WaveRecordedCell};
use crate::util::fixed_math::SimFixed;
use glam::IVec3;

use super::techno_ai::ObjectAiCtx;
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

fn insert_reservation_building(
    sim: &mut Simulation,
    stable_id: u64,
    owner_name: &str,
    rx: u16,
    ry: u16,
    foundation: &str,
    spacing: i32,
) -> crate::sim::intern::InternedId {
    let owner = sim.interner.intern(owner_name);
    let type_ref = sim.interner.intern(&format!("BUILDING{stable_id}"));
    let mut entity = GameEntity::new_at_frame_zero_for_test(
        stable_id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 100,
            max: 100,
        },
        type_ref,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    entity.foundation = foundation.to_string();
    entity.base_reservation_spacing = Some(spacing);
    sim.substrate.entities.insert(entity);
    owner
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

#[test]
fn gsi_04_11_structure_mark_clears_full_smudge_footprints_and_refinery_hole() {
    let mut sim = Simulation::with_seed(1);
    let stable_id = 41;
    insert_reservation_building(&mut sim, stable_id, "Americans", 5, 5, "3x3Refinery", 0);

    let mut grid = crate::sim::smudge_grid::SmudgeGrid::new(16, 16);
    for (rx, ry, frame_offset) in [(4, 5, 0), (5, 5, 1), (4, 6, 2), (5, 6, 3)] {
        grid.test_force_set(
            rx,
            ry,
            crate::sim::smudge_grid::SmudgeCell {
                type_id: Some(1),
                footprint_origin: Some((4, 5)),
                frame_offset,
            },
        );
    }
    grid.test_force_set(
        7,
        6,
        crate::sim::smudge_grid::SmudgeCell {
            type_id: Some(2),
            footprint_origin: Some((7, 6)),
            frame_offset: 0,
        },
    );
    grid.test_force_set(
        9,
        9,
        crate::sim::smudge_grid::SmudgeCell {
            type_id: Some(3),
            footprint_origin: Some((9, 9)),
            frame_offset: 0,
        },
    );
    let _ = grid.drain_dirty();
    sim.smudge_grid = Some(grid);

    assert!(matches!(
        sim.try_reveal_entity(stable_id, request(5, 5, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed { .. }
    ));

    let grid = sim.smudge_grid.as_ref().unwrap();
    for cell in [(4, 5), (5, 5), (4, 6), (5, 6), (7, 6)] {
        assert!(grid.cell(cell.0, cell.1).type_id.is_none(), "cell {cell:?}");
    }
    assert!(
        grid.cell(9, 9).type_id.is_some(),
        "a footprint outside the full refinery rectangle survives"
    );
    let expected = vec![(4, 5), (5, 5), (4, 6), (5, 6), (7, 6)];
    assert_eq!(sim.tactical_dirty_cells, expected);
    assert_eq!(sim.radar_terrain_dirty_cells, expected);
    assert_eq!(sim.radar_terrain_dirty_generation, 5);
}

fn gsi_04_16_edge_gate_rules() -> crate::rules::ruleset::RuleSet {
    let ini = crate::rules::ini_parser::IniFile::from_str(
        "[BuildingTypes]\n0=GACNST\n1=CAOILD\n\
         [GACNST]\nStrength=1000\nFactory=BuildingType\n\
         [CAOILD]\nStrength=1000\n",
    );
    crate::rules::ruleset::RuleSet::from_ini(&ini).expect("edge-gate fixture rules")
}

fn gsi_04_16_dustbowl_bounds() -> crate::sim::cell_rect::PlayfieldBounds {
    crate::sim::cell_rect::PlayfieldBounds {
        base: 70,
        off_fc: 2,
        off_100: 8,
        off_104: 65,
        off_108: 62,
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
fn gsi_04_12_common_raw_occupation_infantry_marks_after_link_then_clears() {
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
    assert_eq!(
        sim.substrate
            .raw_cell_occupation
            .ground_infantry_owner(3, 4),
        Some(1)
    );

    sim.lifecycle_test_events.clear();
    let _ = sim.object_conceal(1);

    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(
        sim.substrate
            .raw_cell_occupation
            .ground_infantry_owner(3, 4),
        None
    );
    assert!(
        sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationListUnlinked),
        "object-list unlink precedes the independent raw occupation clear"
    );
    assert!(
        sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationCleared)
    );
}

#[test]
fn gsi_04_12_common_raw_occupation_infantry_above_deck_height_marks_and_clears_deck() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 1, Some((3, 4)));
    insert_entity(&mut sim, 1, EntityCategory::Infantry);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 6, 192, 64));

    assert!(sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0x04);
    assert_eq!(
        sim.substrate.raw_cell_occupation.deck_infantry_owner(3, 4),
        Some(1)
    );
    assert!(
        sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationListLinked)
    );
    assert!(
        sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::RawOccupationMarked)
    );

    let _ = sim.object_conceal(1);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
    assert_eq!(
        sim.substrate.raw_cell_occupation.deck_infantry_owner(3, 4),
        None
    );
}

#[test]
fn gsi_04_12_common_raw_occupation_high_nonstructural_infantry_retains_mark_plane_asymmetry() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 2, None);
    insert_entity(&mut sim, 1, EntityCategory::Infantry);

    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 6, 192, 64));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x04);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);

    let _ = sim.object_conceal(1);
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(3, 4),
        0x04,
        "InfantryClass::Unmark selects the high plane by Z without retesting Flags&0x100"
    );
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
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
fn gsi_04_05_hidden_lifecycle_follows_base_lists_without_expanding_them() {
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    insert_entity(&mut sim, 1, EntityCategory::Structure);
    {
        let building = sim.substrate.entities.get_mut(1).expect("building");
        building.foundation = "2x2".to_string();
        let profile = building
            .building_hidden_occupancy
            .as_mut()
            .expect("structure constructor profile");
        profile.add_occupy[0] = Some((-1, 0));
        profile.remove_occupy[0] = Some((1, 1));
    }

    let _ = sim.try_reveal_entity(1, common_raw_request(4, 4, 0, 128, 128));
    assert!(sim.substrate.occupancy.contains_entity(5, 5, 1));
    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.hidden_occupation.count(3, 4), 1);
    assert_eq!(sim.substrate.hidden_occupation.count(5, 5), 0);
    let linked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListLinked)
        .expect("base lists linked");
    let hidden = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::HiddenOccupationEntered)
        .expect("hidden counters entered");
    assert!(linked < hidden);

    sim.lifecycle_test_events.clear();
    let _ = sim.object_conceal(1);
    assert!(!sim.substrate.occupancy.contains_entity(5, 5, 1));
    assert_eq!(sim.substrate.hidden_occupation.entry_count(), 0);
    let unlinked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .expect("base lists unlinked");
    let hidden = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::HiddenOccupationExited)
        .expect("hidden counters exited");
    assert!(unlinked < hidden);
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

#[test]
fn gsi_05_05_fly_takeoff_commits_absolute_z_after_remove_process() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 2, None);
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(0));
    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 2, 128, 128));
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
    let altitude = aircraft
        .locomotor
        .as_ref()
        .unwrap()
        .altitude
        .to_num::<i32>();
    assert!(altitude > 0);
    assert_eq!(aircraft.position.exact_z_leptons, Some(208 + altitude));
    assert!(!sim.substrate.occupancy.contains_entity(3, 4, 1));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
}

#[test]
fn gsi_05_05_fly_landing_on_bridge_uses_absolute_z_for_deck_put() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 0, Some((3, 4)));
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(1));
    sim.substrate.entities.get_mut(1).unwrap().on_bridge = true;
    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0, 128, 128));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);

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
        locomotor.target_altitude = SimFixed::from_num(0);
        locomotor.climb_rate = SimFixed::from_num(1500);
    }
    sim.tick_air_movement_with_cell_lists_one(1);

    let aircraft = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        aircraft.locomotor.as_ref().unwrap().altitude,
        SimFixed::from_num(0)
    );
    assert_eq!(aircraft.position.exact_z_leptons, Some(416));
    assert_eq!(
        sim.substrate
            .occupancy
            .count_on_layer(3, 4, MovementLayer::Bridge),
        1
    );
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0x40);
}

#[test]
fn gsi_05_05_object_raw_occupation_uses_exact_sloped_ground_z_for_put_and_remove() {
    let mut sim = Simulation::new();
    install_common_raw_terrain(&mut sim, 8, 8, 1, Some((3, 4)));
    sim.resolved_terrain
        .as_mut()
        .unwrap()
        .cell_mut(3, 4)
        .unwrap()
        .slope_type = 1;
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(0));
    sim.substrate.entities.get_mut(1).unwrap().on_bridge = true;
    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 0, 64, 192));
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x40);
    sim.remove_entity_occupancy(1);
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);

    // Level 1 plus slope 1 at local X=64 produces exact ground Z 130.
    // Keep coarse Z deliberately above the deck so only exact Object Z can
    // distinguish the two sides of the inclusive 416-lepton threshold.
    {
        let aircraft = sim.substrate.entities.get_mut(1).unwrap();
        aircraft.position.z = 0x7f;
        aircraft.position.exact_z_leptons = Some(130 + 415);
    }
    sim.add_entity_occupancy(1);
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0x40);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
    sim.remove_entity_occupancy(1);
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);

    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .position
        .exact_z_leptons = Some(130 + 416);
    sim.add_entity_occupancy(1);
    assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0x40);
    sim.remove_entity_occupancy(1);
    assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(3, 4), 0);
}

#[test]
fn gsi_05_05_mapless_fly_uses_dummy_ground_then_bridge_height() {
    let mut sim = Simulation::new();
    install_fly_aircraft(&mut sim, 1, SimFixed::from_num(100));
    {
        let aircraft = sim.substrate.entities.get_mut(1).unwrap();
        aircraft.on_bridge = true;
        aircraft.locomotor.as_mut().unwrap().target_altitude = SimFixed::from_num(100);
    }
    let _ = sim.try_reveal_entity(1, common_raw_request(3, 4, 2, 128, 128));

    sim.tick_air_movement_with_cell_lists_one(1);

    let aircraft = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        aircraft.position.exact_z_leptons,
        Some(416 + 100),
        "missing terrain uses native dummy-cell ground zero before OnBridge"
    );
}

#[test]
fn gsi_04_07_damage_air_spatial_entry_crossing_and_exit_keep_vector_order() {
    let mut sim = Simulation::new();
    sim.session.map_width = 40;
    sim.session.map_height = 40;
    install_common_raw_terrain(&mut sim, 40, 40, 0, None);
    install_fly_aircraft(&mut sim, 20, SimFixed::from_num(4));
    install_fly_aircraft(&mut sim, 10, SimFixed::from_num(4));

    let _ = sim.try_reveal_entity(20, common_raw_request(2, 4, 4, 128, 128));
    let _ = sim.try_reveal_entity(10, common_raw_request(3, 4, 4, 128, 128));
    let first = sim.substrate.entities.get(20).unwrap();
    let second = sim.substrate.entities.get(10).unwrap();
    assert_eq!(first.air_spatial_bucket, second.air_spatial_bucket);
    assert!(
        first.air_spatial_enter_order < second.air_spatial_enter_order,
        "same-bucket vector retains append order, not stable-ID order"
    );
    let first_order = first.air_spatial_enter_order;
    let shared_bucket = second.air_spatial_bucket;
    let second_order = second.air_spatial_enter_order;

    sim.tick_air_movement_with_cell_lists_one(20);
    assert_eq!(
        sim.substrate
            .entities
            .get(20)
            .unwrap()
            .air_spatial_enter_order,
        first_order,
        "Fly's temporary cell-list transaction is not an air-vector re-entry"
    );

    sim.substrate.entities.get_mut(20).unwrap().position.rx = 12;
    sim.tick_air_movement_with_cell_lists_one(20);
    let crossed = sim.substrate.entities.get(20).unwrap();
    assert_ne!(crossed.air_spatial_bucket, shared_bucket);
    assert!(crossed.air_spatial_enter_order > second_order);

    let _ = sim.object_conceal(20);
    let exited = sim.substrate.entities.get(20).unwrap();
    assert_eq!(exited.air_spatial_bucket, None);
    assert_eq!(exited.air_spatial_enter_order, 0);
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
        draw_runtime: crate::sim::anim_class::AnimDrawRuntime::default(),
        use_cell_drawer: false,
        terrain_attached: false,
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
            LifecycleTestEvent::ConcealDestroyNotifyBoundary {
                stable_id: 1,
                object_alive: true,
                cell_marked: true,
                resolvable: true,
            },
            LifecycleTestEvent::UninitRemovalListenerVisited {
                expired_id: 1,
                listener_id: 1,
                target_alive: true,
                target_in_limbo: false,
            },
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
            LifecycleTestEvent::ConcealDestroyNotifyBoundary {
                stable_id: 1,
                object_alive: true,
                cell_marked: true,
                resolvable: true,
            },
            LifecycleTestEvent::UninitRemovalListenerVisited {
                expired_id: 1,
                listener_id: 1,
                target_alive: true,
                target_in_limbo: false,
            },
            LifecycleTestEvent::UninitRemovalListenerVisited {
                expired_id: 1,
                listener_id: 2,
                target_alive: true,
                target_in_limbo: false,
            },
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
                    resolvable: true,
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
fn gsi_05_03_conceal_destroy_and_uninit_broadcast_before_unmark() {
    let mut conceal_only = Simulation::new();
    insert_entity(&mut conceal_only, 1, EntityCategory::Unit);
    let _ = conceal_only.reveal(1);
    conceal_only.lifecycle_test_events.clear();

    assert_eq!(
        conceal_only.object_conceal(1),
        super::ConcealOutcome::Concealed
    );
    let conceal_boundary = conceal_only
        .lifecycle_test_events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::ConcealDestroyNotifyBoundary {
                    stable_id: 1,
                    object_alive: true,
                    cell_marked: true,
                    resolvable: true,
                }
        })
        .expect("Conceal Destroy notification boundary");
    let conceal_unmark = conceal_only
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .expect("Conceal unmark");
    assert!(conceal_boundary < conceal_unmark);
    assert!(conceal_only.substrate.entities.contains(1));
    assert!(
        conceal_only
            .substrate
            .entities
            .get(1)
            .unwrap()
            .lifecycle
            .object_alive
    );

    let mut uninit = Simulation::new();
    insert_entity(&mut uninit, 1, EntityCategory::Unit);
    insert_entity(&mut uninit, 2, EntityCategory::Unit);
    let _ = uninit.reveal(2);
    uninit.lifecycle_test_events.clear();
    uninit.uninit(2);

    let direct = uninit
        .lifecycle_test_events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::UninitRemovalNotifyBoundary {
                    stable_id: 2,
                    object_alive: true,
                    cell_marked: true,
                    resolvable: true,
                }
        })
        .expect("direct UnInit expiry boundary");
    let destroy = uninit
        .lifecycle_test_events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::ConcealDestroyNotifyBoundary {
                    stable_id: 2,
                    object_alive: true,
                    cell_marked: true,
                    resolvable: true,
                }
        })
        .expect("virtual Conceal Destroy expiry boundary");
    let unmark = uninit
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .expect("UnInit Conceal unmark");
    let alive_clear = uninit
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::UninitAliveCleared { stable_id: 2 })
        .expect("UnInit alive clear");
    assert!(direct < destroy && destroy < unmark && unmark < alive_clear);
    assert_eq!(
        uninit
            .lifecycle_test_events
            .iter()
            .filter(|event| matches!(
                event,
                LifecycleTestEvent::UninitRemovalListenerVisited { expired_id: 2, .. }
            ))
            .count(),
        4,
        "two listeners observe both expiry broadcasts"
    );
}

#[test]
fn gsi_05_03_dead_nonlimbo_stored_object_still_runs_conceal() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);
    {
        let entity = sim.substrate.entities.get_mut(1).unwrap();
        entity.lifecycle.object_alive = false;
        assert!(!entity.lifecycle.in_limbo);
        assert!(entity.lifecycle.cell_marked);
    }
    sim.lifecycle_test_events.clear();

    assert_eq!(sim.techno_limbo(1), super::ConcealOutcome::Concealed);
    let destroy = sim
        .lifecycle_test_events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::ConcealDestroyNotifyBoundary {
                    stable_id: 1,
                    object_alive: false,
                    cell_marked: true,
                    resolvable: true,
                }
        })
        .expect("dead non-limbo object reached Conceal's Destroy boundary");
    let unmark = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .expect("dead non-limbo object reached Conceal's Mark(REMOVE)");
    let limbo_set = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::ConcealLimboSet)
        .expect("dead non-limbo object completed Conceal");
    assert!(destroy < unmark && unmark < limbo_set);

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(!entity.lifecycle.object_alive);
    assert!(entity.lifecycle.in_limbo);
    assert!(!entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);
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
            (1, true, false),
            (2, true, false),
            (3, true, false),
            (4, true, false),
        ]
    );

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
    let direct_last_listener = sim.lifecycle_test_events[..break_slot]
        .iter()
        .rposition(|event| {
            matches!(
                event,
                LifecycleTestEvent::UninitRemovalListenerVisited { expired_id: 4, .. }
            )
        })
        .unwrap();
    let conceal_notify = sim
        .lifecycle_test_events
        .iter()
        .position(|event| {
            matches!(
                event,
                LifecycleTestEvent::ConcealDestroyNotifyBoundary { stable_id: 4, .. }
            )
        })
        .unwrap();
    let second_last_listener = sim
        .lifecycle_test_events
        .iter()
        .rposition(|event| {
            matches!(
                event,
                LifecycleTestEvent::UninitRemovalListenerVisited { expired_id: 4, .. }
            )
        })
        .unwrap();
    let unmark = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .unwrap();
    assert!(direct_last_listener < break_slot && break_slot < conceal);
    assert!(conceal < conceal_notify && conceal_notify < second_last_listener);
    assert!(second_last_listener < unmark);
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
fn gsi_05_03_duplicate_uninit_repeats_direct_expiry_and_queue_but_finalizes_once() {
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 0, 10));
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    sim.houses.get_mut(&owner).unwrap().owned_unit_count = 2;
    let _ = sim.reveal(1);

    sim.lifecycle_test_events.clear();
    sim.uninit(1);
    assert_eq!(
        sim.lifecycle_test_events
            .iter()
            .filter(|event| matches!(
                event,
                LifecycleTestEvent::UninitRemovalNotifyBoundary { stable_id: 1, .. }
            ))
            .count(),
        1
    );
    assert_eq!(
        sim.lifecycle_test_events
            .iter()
            .filter(|event| matches!(
                event,
                LifecycleTestEvent::ConcealDestroyNotifyBoundary { stable_id: 1, .. }
            ))
            .count(),
        1
    );
    assert_eq!(
        sim.lifecycle_test_events
            .iter()
            .filter(|event| matches!(
                event,
                LifecycleTestEvent::UninitRemovalListenerVisited { expired_id: 1, .. }
            ))
            .count(),
        2,
        "the first active UnInit performs its direct dispatch and Conceal's Destroy dispatch"
    );

    let repeat_start = sim.lifecycle_test_events.len();
    sim.uninit(1);
    let repeated_events = &sim.lifecycle_test_events[repeat_start..];
    assert_eq!(sim.houses.get(&owner).unwrap().owned_unit_count, 1);
    assert!(sim.substrate.entities.get(1).unwrap().owned_count_released);
    assert_eq!(sim.substrate.pending_delete, vec![1, 1]);
    let direct = repeated_events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::UninitRemovalNotifyBoundary {
                    stable_id: 1,
                    object_alive: false,
                    cell_marked: false,
                    resolvable: true,
                }
        })
        .expect("repeat UnInit direct expiry boundary");
    let direct_listener = repeated_events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::UninitRemovalListenerVisited {
                    expired_id: 1,
                    listener_id: 1,
                    target_alive: false,
                    target_in_limbo: true,
                }
        })
        .expect("repeat UnInit direct expiry dispatch");
    let conceal_return = repeated_events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::ConcealAlreadyLimboReturn {
                    stable_id: 1,
                    object_alive: false,
                    resolvable: true,
                }
        })
        .expect("repeat Limbo reached Conceal's InLimbo return");
    assert!(direct < direct_listener && direct_listener < conceal_return);
    assert_eq!(
        repeated_events
            .iter()
            .filter(|event| matches!(
                event,
                LifecycleTestEvent::UninitRemovalListenerVisited { expired_id: 1, .. }
            ))
            .count(),
        1,
        "the repeated UnInit adds exactly one direct expiry dispatch"
    );
    assert!(
        !repeated_events.iter().any(|event| matches!(
            event,
            LifecycleTestEvent::ConcealDestroyNotifyBoundary { stable_id: 1, .. }
        )),
        "Conceal's InLimbo return occurs before Destroy"
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
fn gsi_04_05_reservation_successful_reveal_marks_expanded_rect_after_lists() {
    let mut sim = Simulation::new();
    let owner = insert_reservation_building(&mut sim, 101, "Americans", 10, 20, "2x3", 1);
    sim.session.house_order.push(owner);

    assert!(matches!(
        sim.try_reveal_entity(101, request(10, 20, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed { .. }
    ));
    for x in 9..=12 {
        for y in 19..=23 {
            assert_eq!(sim.substrate.base_reservations.raw_mask(None, x, y), 1);
        }
    }
    assert_eq!(sim.substrate.base_reservations.raw_mask(None, 8, 20), 0);
    assert_eq!(sim.substrate.base_reservations.raw_mask(None, 13, 20), 0);

    let cell_marked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::CellMarked)
        .unwrap();
    let reservation_marked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::BaseReservationMarked)
        .unwrap();
    let display = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RevealDisplayBoundary)
        .unwrap();
    assert!(cell_marked < reservation_marked && reservation_marked < display);
}

#[test]
fn gsi_04_16_dustbowl_conyard_unlimbo_uses_local_size_edge() {
    let mut sim = Simulation::new();
    sim.playfield_bounds = Some(gsi_04_16_dustbowl_bounds());
    let rules = gsi_04_16_edge_gate_rules();
    let owner = sim.interner.intern("Americans");
    let mut house = HouseState::new(owner, 0, None, true, 0, 10);
    house.base_center = Some((70, 116));
    sim.houses.insert(owner, house);

    let conyard = sim
        .spawn_object("GACNST", "Americans", 69, 115, 0, &rules, &BTreeMap::new())
        .expect("GACNST reveals");
    assert!(
        sim.substrate
            .entities
            .get(conyard)
            .unwrap()
            .determines_waypoint_edge,
        "Factory=BuildingType must freeze onto the entity before Reveal"
    );
    let house = sim.houses.get(&owner).expect("launch house");
    assert_eq!(house.base_center, Some((70, 116)));
    assert_eq!(house.waypoint_edge, 2);
}

#[test]
fn gsi_04_16_committed_structure_owner_change_refreshes_new_house_edge() {
    let mut sim = Simulation::new();
    sim.playfield_bounds = Some(gsi_04_16_dustbowl_bounds());
    let rules = gsi_04_16_edge_gate_rules();
    let old_owner = sim.interner.intern("Americans");
    let new_owner = sim.interner.intern("Soviets");
    sim.houses
        .insert(old_owner, HouseState::new(old_owner, 0, None, true, 0, 10));
    let mut new_house = HouseState::new(new_owner, 1, None, false, 0, 10);
    new_house.base_center = Some((68, 114));
    sim.houses.insert(new_owner, new_house);
    let conyard = sim
        .spawn_object("GACNST", "Americans", 69, 115, 0, &rules, &BTreeMap::new())
        .expect("GACNST reveals");

    sim.change_owner(conyard, new_owner);

    let new_house = sim.houses.get(&new_owner).expect("new owner house");
    assert_eq!(new_house.base_center, Some((68, 114)));
    assert_eq!(new_house.waypoint_edge, 2);
}

#[test]
fn gsi_04_16_caoild_reveal_and_owner_change_preserve_waypoint_edges() {
    let mut sim = Simulation::new();
    sim.playfield_bounds = Some(gsi_04_16_dustbowl_bounds());
    let rules = gsi_04_16_edge_gate_rules();
    let old_owner = sim.interner.intern("Americans");
    let new_owner = sim.interner.intern("Soviets");
    let mut old_house = HouseState::new(old_owner, 0, None, true, 0, 10);
    old_house.waypoint_edge = 1;
    sim.houses.insert(old_owner, old_house);
    let mut new_house = HouseState::new(new_owner, 1, None, false, 0, 10);
    new_house.waypoint_edge = 3;
    sim.houses.insert(new_owner, new_house);

    let oil = sim
        .spawn_object("CAOILD", "Americans", 69, 115, 0, &rules, &BTreeMap::new())
        .expect("CAOILD reveals");
    assert!(
        !sim.substrate
            .entities
            .get(oil)
            .unwrap()
            .determines_waypoint_edge,
        "missing Factory= must freeze as an ineligible edge profile"
    );
    assert_eq!(sim.houses.get(&old_owner).unwrap().waypoint_edge, 1);

    sim.change_owner(oil, new_owner);

    assert_eq!(sim.houses.get(&new_owner).unwrap().waypoint_edge, 3);
}

#[test]
fn gsi_04_05_reservation_failed_reveal_never_writes() {
    let mut sim = Simulation::new();
    let owner = insert_reservation_building(&mut sim, 102, "Americans", 10, 20, "2x2", 1);
    sim.session.house_order.push(owner);

    assert_eq!(
        sim.try_reveal_entity(102, request(10, 20, PlacementEvidence::MarkFailed)),
        RevealOutcome::Failed(RevealFailure::MarkFailed)
    );
    assert_eq!(sim.substrate.base_reservations.entries().count(), 0);
    assert_eq!(sim.substrate.base_reservations.dummy_mask(), 0);
    assert!(
        !sim.lifecycle_test_events
            .contains(&LifecycleTestEvent::BaseReservationMarked)
    );
}

#[test]
fn gsi_04_05_reservation_limbo_clears_before_unlink_repairs_overlap_and_preserves_other_house() {
    let mut sim = Simulation::new();
    let owner = insert_reservation_building(&mut sim, 103, "Americans", 10, 10, "1x1", 1);
    let other = sim.interner.intern("Russians");
    sim.session.house_order.extend([owner, other]);
    insert_reservation_building(&mut sim, 104, "Americans", 12, 10, "1x1", 1);
    let _ = sim.try_reveal_entity(103, request(10, 10, PlacementEvidence::MarkSucceeded));
    let _ = sim.try_reveal_entity(104, request(12, 10, PlacementEvidence::MarkSucceeded));
    sim.substrate.base_reservations.reserve(None, 9, 10, 1);

    sim.lifecycle_test_events.clear();
    assert_eq!(sim.techno_limbo(103), super::ConcealOutcome::Concealed);

    assert_eq!(
        sim.substrate.base_reservations.raw_mask(None, 9, 10),
        1 << 1,
        "AND-not clears only the leaving owner's bit"
    );
    assert_eq!(
        sim.substrate.base_reservations.raw_mask(None, 11, 10) & 1,
        1,
        "neighbor repair restores the same-house overlap"
    );
    let cleared = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::BaseReservationCleared)
        .unwrap();
    let unlinked = sim
        .lifecycle_test_events
        .iter()
        .position(|event| *event == LifecycleTestEvent::RawOccupationListUnlinked)
        .unwrap();
    assert!(cleared < unlinked);
}

#[test]
fn gsi_04_05_reservation_repair_scan_reaches_asymmetric_high_edge() {
    assert_eq!(
        super::lifecycle::building_base_reservation_repair_rect(10, 20, 4, 4, 1),
        crate::sim::cell_rect::CellRect::new(8, 18, 9, 9),
        "half-open repair bounds are [8,17) x [18,27)"
    );
    assert_eq!(
        super::lifecycle::building_base_reservation_repair_rect(10, 20, 6, 6, -1),
        crate::sim::cell_rect::CellRect::new(12, 22, 1, 1),
        "signed negative spacing yields [12,13) x [22,23)"
    );

    let mut sim = Simulation::new();
    let owner = insert_reservation_building(&mut sim, 107, "Americans", 10, 20, "4x4", 1);
    sim.session.house_order.push(owner);
    insert_reservation_building(&mut sim, 108, "Americans", 16, 20, "1x1", 2);
    let _ = sim.try_reveal_entity(107, request(10, 20, PlacementEvidence::MarkSucceeded));
    let _ = sim.try_reveal_entity(108, request(16, 20, PlacementEvidence::MarkSucceeded));
    assert_eq!(sim.substrate.base_reservations.raw_mask(None, 14, 20), 1);

    assert_eq!(sim.techno_limbo(107), super::ConcealOutcome::Concealed);
    assert_eq!(
        sim.substrate.base_reservations.raw_mask(None, 14, 20),
        1,
        "the x=16 neighbor, omitted by the old [8,16) scan, re-marks its overlap"
    );
}

#[test]
fn gsi_04_05_reservation_art_foundation_recomputes_writer_before_lifecycle_mark() {
    use crate::rules::art_data::ArtRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;

    let mut rules = RuleSet::from_ini(&IniFile::from_str(
        "[AI]\nAIBaseSpacing=2\n\
         [BuildingTypes]\n0=GACNST\n1=ONECNST\n\
         [GACNST]\nUndeploysInto=AMCV\n\
         [ONECNST]\nUndeploysInto=AMCV\n",
    ))
    .expect("split rules-side construction-yard data");
    assert_eq!(
        rules.object("GACNST").unwrap().base_reservation_spacing,
        None,
        "the provisional Rules-only 1x1 foundation is ineligible"
    );

    rules.merge_art_data(&ArtRegistry::from_ini(&IniFile::from_str(
        "[GACNST]\nFoundation=4x4\n\
         [ONECNST]\nFoundation=1x1\n",
    )));
    let gacnst = rules.object("GACNST").unwrap();
    assert_eq!(gacnst.foundation, "4x4");
    assert_eq!(gacnst.base_reservation_spacing, Some(2));
    assert_eq!(
        rules.object("ONECNST").unwrap().base_reservation_spacing,
        None,
        "an effective ART 1x1 undeployer remains ineligible"
    );
    let foundation = gacnst.foundation.clone();
    let spacing = gacnst.base_reservation_spacing.unwrap();

    let mut sim = Simulation::new();
    let owner =
        insert_reservation_building(&mut sim, 106, "Americans", 30, 40, &foundation, spacing);
    sim.session.house_order.push(owner);
    assert!(matches!(
        sim.try_reveal_entity(106, request(30, 40, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed { .. }
    ));
    assert_eq!(sim.substrate.base_reservations.raw_mask(None, 28, 38), 1);
    assert_eq!(sim.substrate.base_reservations.raw_mask(None, 35, 45), 1);
    assert_eq!(sim.substrate.base_reservations.raw_mask(None, 36, 45), 0);
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

fn gsi_05_02_projectile(source_id: u64, fuse_frames: Option<u16>) -> ProjectileSpawn {
    ProjectileSpawn {
        source_id,
        origin: ProjectileCoord::new(0, 0, 0),
        target: ProjectileTarget::Cell { rx: 16, ry: 0 },
        initial_target_position: ProjectileCoord::new(4096, 0, 0),
        payload: ProjectilePayload {
            base_damage: 1,
            warhead: crate::sim::intern::InternedId::from_index(0),
            weapon: crate::sim::intern::InternedId::from_index(0),
            owner: crate::sim::intern::InternedId::from_index(0),
        },
        speed_leptons_per_frame: 64,
        velocity: ProjectileVelocity::new(64, 0, 0),
        trajectory: ProjectileTrajectory::Straight,
        guidance: None,
        visual: ProjectileVisualState::new(0, 0, 0),
        arm_frames: 0,
        fuse_frames,
        ranged_fuse: false,
        tracks_target: false,
        target_expiry: TargetExpiryPolicy::Expire,
        collision: ProjectileCollisionPolicy::NONE,
    }
}

fn gsi_05_04_guided_projectile(
    source_id: u64,
    target: ProjectileTarget,
    initial_target_position: ProjectileCoord,
) -> ProjectileSpawn {
    let mut spawn = gsi_05_02_projectile(source_id, None);
    spawn.target = target;
    spawn.initial_target_position = initial_target_position;
    spawn.guidance = Some(ProjectileGuidance {
        rot: 60,
        missile_rot_var: SimFixed::lit("0.25"),
        course_lock_frames: 0,
        sidewinder_phase: 0,
        airburst: false,
        very_high: false,
        level: false,
        pitch_bam: 0,
        frames_elapsed: 0,
    });
    spawn.tracks_target = true;
    spawn.target_expiry = TargetExpiryPolicy::DetonateAtLastKnown;
    spawn
}

fn gsi_05_02_mixed_fixture() -> (Simulation, [u64; 6]) {
    let mut sim = Simulation::new();

    let entity_id = sim.allocate_stable_id();
    insert_entity(&mut sim, entity_id, EntityCategory::Unit);
    sim.register_live_object(entity_id);

    let anim_id = sim.allocate_stable_id();
    insert_anim(&mut sim, anim_id, true);
    sim.register_live_object(anim_id);

    let particle_id = sim.allocate_stable_id();
    insert_particle_system(&mut sim, particle_id);
    sim.register_live_object(particle_id);

    let terrain_id = sim.allocate_stable_id();
    sim.production.terrain_objects.insert(
        terrain_id,
        TerrainObjectState {
            stable_id: terrain_id,
            in_logic_vector: false,
            type_ref: sim.interner.intern("TREE01"),
            rx: 8,
            ry: 9,
            health: 10,
            max_health: 10,
            occupation_bits: 0,
            lifecycle: TerrainObjectLifecycle::Live,
        },
    );
    assert!(sim.register_terrain_object(terrain_id));

    let projectile_id = sim.allocate_stable_id();
    sim.admit_projectile(projectile_id, gsi_05_02_projectile(entity_id, None));

    let wave_id = sim.allocate_stable_id();
    sim.admit_wave(
        wave_id,
        Wave::new(
            3,
            ProjectileCoord::new(0, 0, 0),
            ProjectileCoord::new(256, 0, 0),
        ),
    );

    let mixed = [
        terrain_id,
        entity_id,
        wave_id,
        anim_id,
        projectile_id,
        particle_id,
    ];
    sim.set_logic_order_for_test(mixed.to_vec());
    (sim, mixed)
}

#[test]
fn gsi_05_02_mixed_six_family_order_roundtrips_and_dispatches_every_slot() {
    let (sim, mixed) = gsi_05_02_mixed_fixture();
    let bytes = GameSnapshot::save(&sim, 0, 0, "logic-membership", 0);
    let mut restored = GameSnapshot::load(&bytes)
        .expect("mixed Logic snapshot")
        .sim;
    restored
        .restore_after_snapshot_load()
        .expect("mixed Logic identities restore");

    assert_eq!(restored.live_object_order_snapshot(), mixed);
    assert!(
        restored
            .production
            .terrain_objects
            .get(&mixed[0])
            .unwrap()
            .in_logic_vector
    );
    assert!(
        restored
            .substrate
            .entities
            .get(mixed[1])
            .unwrap()
            .in_logic_vector
    );
    assert!(restored.waves.get(mixed[2]).unwrap().in_logic_vector);
    assert!(
        restored
            .substrate
            .anims
            .get(mixed[3])
            .unwrap()
            .in_logic_vector
    );
    assert!(restored.projectiles.get(mixed[4]).unwrap().in_logic_vector);
    assert!(
        restored
            .substrate
            .particle_systems
            .get(mixed[5])
            .unwrap()
            .in_logic_vector
    );

    let mut visited = Vec::new();
    restored.for_each_live_object(|sim, id| {
        if sim.object_ai_visit_one(id, None, ObjectAiCtx::default()) {
            visited.push(id);
        }
    });
    assert_eq!(visited, mixed);
    restored.debug_assert_logic_membership_consistent();
}

#[test]
fn gsi_05_02_tail_appends_run_same_pass_and_terminal_current_skips_successor() {
    let mut sim = Simulation::new();
    let entity_id = sim.allocate_stable_id();
    insert_entity(&mut sim, entity_id, EntityCategory::Unit);
    sim.register_live_object(entity_id);

    let mut appended = None;
    let mut visited = Vec::new();
    sim.for_each_live_object(|sim, id| {
        visited.push(id);
        let _ = sim.object_ai_visit_one(id, None, ObjectAiCtx::default());
        if id == entity_id {
            let projectile_id = sim.allocate_stable_id();
            sim.admit_projectile(projectile_id, gsi_05_02_projectile(entity_id, None));
            let wave_id = sim.allocate_stable_id();
            sim.admit_wave(
                wave_id,
                Wave::new(
                    3,
                    ProjectileCoord::new(0, 0, 0),
                    ProjectileCoord::new(256, 0, 0),
                ),
            );
            appended = Some((projectile_id, wave_id));
        }
    });
    let (projectile_id, wave_id) = appended.expect("tail objects appended");
    assert_eq!(visited, vec![entity_id, projectile_id, wave_id]);
    assert_eq!(
        sim.projectiles.get(projectile_id).unwrap().position,
        // CellClass target coordinates resolve from the live cell center on
        // every visit, so Cell(16, 0) contributes a small positive Y step.
        ProjectileCoord::new(64, 1, 0)
    );
    assert_eq!(sim.waves.get(wave_id).unwrap().lifetime, 99);

    let mut removal = Simulation::new();
    let terminal_id = removal.allocate_stable_id();
    removal.admit_projectile(terminal_id, gsi_05_02_projectile(0, Some(0)));
    let successor_id = removal.allocate_stable_id();
    removal.admit_wave(
        successor_id,
        Wave::new(
            3,
            ProjectileCoord::new(0, 0, 0),
            ProjectileCoord::new(256, 0, 0),
        ),
    );
    let mut removal_visited = Vec::new();
    removal.for_each_live_object(|sim, id| {
        removal_visited.push(id);
        let _ = sim.object_ai_visit_one(id, None, ObjectAiCtx::default());
    });
    assert_eq!(removal_visited, vec![terminal_id]);
    assert_eq!(removal.live_object_order_snapshot(), vec![successor_id]);
    assert!(removal.projectiles.get(terminal_id).is_some());
    assert_eq!(removal.substrate.pending_delete, vec![terminal_id]);
    assert_eq!(removal.waves.get(successor_id).unwrap().lifetime, 100);
    removal.debug_assert_logic_membership_consistent();
}

#[test]
fn gsi_05_02_terrain_map_construction_registers_before_later_techno() {
    let rules =
        crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
            "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [TerrainTypes]\n0=TREE01\n[TREE01]\nStrength=10\n",
        ))
        .expect("terrain fixture rules");
    let mut sim = Simulation::new();
    assert_eq!(
        crate::sim::terrain_spawn::construct_terrain_objects(
            &mut sim,
            &[crate::map::overlay::TerrainObject {
                rx: 3,
                ry: 4,
                name: "TREE01".to_owned(),
            }],
            &rules,
            false,
        ),
        1
    );
    let terrain_id = *sim.production.terrain_objects.keys().next().unwrap();

    let entity_id = sim.allocate_stable_id();
    insert_entity(&mut sim, entity_id, EntityCategory::Unit);
    sim.register_live_object(entity_id);
    assert_eq!(
        sim.live_object_order_snapshot(),
        vec![terrain_id, entity_id]
    );
}

#[test]
fn gsi_05_02_lethal_terrain_unregisters_and_inactive_slot_cannot_roundtrip() {
    let rules =
        crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
            "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [TerrainTypes]\n0=TREE01\n[Warheads]\n0=WOODWH\n\
             [TREE01]\nStrength=10\nArmor=wood\nImmune=no\n\
             [WOODWH]\nWood=yes\nCellSpread=1\nPercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
        ))
        .expect("lethal terrain rules");
    let mut sim = Simulation::new();
    let terrain_id = sim.allocate_stable_id();
    let type_ref = sim.interner.intern("TREE01");
    let warhead_ref = sim.interner.intern("WOODWH");
    sim.production.terrain_objects.insert(
        terrain_id,
        TerrainObjectState {
            stable_id: terrain_id,
            in_logic_vector: false,
            type_ref,
            rx: 5,
            ry: 6,
            health: 10,
            max_health: 10,
            occupation_bits: 0,
            lifecycle: TerrainObjectLifecycle::Live,
        },
    );
    sim.production
        .terrain_object_cells
        .insert((5, 6), terrain_id);
    assert!(sim.register_terrain_object(terrain_id));

    sim.commit_noncombat_aoe_receivers(
        &rules,
        None,
        &[crate::sim::combat::combat_aoe::AreaDamageReceiver::Terrain(
            crate::sim::combat::TerrainDamageEvent {
                stable_id: terrain_id,
                rx: 5,
                ry: 6,
                damage: 10,
                distance_leptons: 0,
                warhead_ref,
                near_center_ic_isolation_eligible: false,
            },
        )],
    );
    let terrain = &sim.production.terrain_objects[&terrain_id];
    assert_eq!(terrain.lifecycle, TerrainObjectLifecycle::Destroyed);
    assert!(!terrain.in_logic_vector);
    assert!(sim.live_object_order_snapshot().is_empty());
    assert_eq!(sim.substrate.pending_delete, vec![terrain_id]);

    let bytes = GameSnapshot::save(&sim, 0, 0, "inactive-terrain", 0);
    let mut restored = GameSnapshot::load(&bytes)
        .expect("inactive terrain snapshot")
        .sim;
    restored
        .restore_after_snapshot_load()
        .expect("inactive terrain remains outside Logic");
    assert!(restored.live_object_order_snapshot().is_empty());

    restored.set_logic_order_for_test(vec![terrain_id]);
    assert_eq!(
        restored.restore_after_snapshot_load(),
        Err(SnapshotRestoreError::InactiveLogicIdentity {
            registry: "TerrainObjectStore",
            object_id: terrain_id,
        })
    );
}

#[test]
fn gsi_05_03_terminal_non_entities_remain_resolvable_until_common_drain() {
    let rules =
        crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
            "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [TerrainTypes]\n0=TREE01\n[Warheads]\n0=WOODWH\n\
             [TREE01]\nStrength=10\nArmor=wood\nImmune=no\n\
             [WOODWH]\nWood=yes\nCellSpread=1\nPercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
        ))
        .expect("terminal non-entity rules");
    let mut sim = Simulation::new();

    let terrain_id = sim.allocate_stable_id();
    let terrain_type = sim.interner.intern("TREE01");
    let warhead_ref = sim.interner.intern("WOODWH");
    sim.production.terrain_objects.insert(
        terrain_id,
        TerrainObjectState {
            stable_id: terrain_id,
            in_logic_vector: false,
            type_ref: terrain_type,
            rx: 5,
            ry: 6,
            health: 10,
            max_health: 10,
            occupation_bits: 0,
            lifecycle: TerrainObjectLifecycle::Live,
        },
    );
    sim.production
        .terrain_object_cells
        .insert((5, 6), terrain_id);
    assert!(sim.register_terrain_object(terrain_id));
    sim.commit_noncombat_aoe_receivers(
        &rules,
        None,
        &[crate::sim::combat::combat_aoe::AreaDamageReceiver::Terrain(
            crate::sim::combat::TerrainDamageEvent {
                stable_id: terrain_id,
                rx: 5,
                ry: 6,
                damage: 10,
                distance_leptons: 0,
                warhead_ref,
                near_center_ic_isolation_eligible: false,
            },
        )],
    );

    let projectile_id = sim.allocate_stable_id();
    sim.admit_projectile(projectile_id, gsi_05_02_projectile(0, Some(0)));
    assert!(sim.object_ai_visit_one(projectile_id, None, ObjectAiCtx::default()));

    let wave_id = sim.allocate_stable_id();
    let mut terminal_wave = Wave::new(
        3,
        ProjectileCoord::new(0, 0, 0),
        ProjectileCoord::new(256, 0, 0),
    );
    terminal_wave.lifetime = 0;
    sim.admit_wave(wave_id, terminal_wave);
    assert!(sim.object_ai_visit_one(wave_id, None, ObjectAiCtx::default()));

    assert_eq!(
        sim.production.terrain_objects[&terrain_id].lifecycle,
        TerrainObjectLifecycle::Destroyed
    );
    assert!(sim.production.terrain_objects.contains_key(&terrain_id));
    assert!(sim.projectiles.get(projectile_id).is_some());
    assert!(sim.waves.get(wave_id).is_some());
    assert!(!sim.production.terrain_objects[&terrain_id].in_logic_vector);
    assert!(!sim.projectiles.get(projectile_id).unwrap().in_logic_vector);
    assert!(!sim.waves.get(wave_id).unwrap().in_logic_vector);
    assert_eq!(
        sim.substrate.pending_delete,
        vec![terrain_id, projectile_id, wave_id]
    );
    assert!(sim.live_object_order_snapshot().is_empty());

    let bytes = GameSnapshot::save(&sim, 0, 0, "terminal-non-entities", 0);
    let mut restored = GameSnapshot::load(&bytes)
        .expect("terminal non-entity snapshot")
        .sim;
    restored
        .restore_after_snapshot_load()
        .expect("deferred non-entities restore outside Logic");
    assert_eq!(
        restored.substrate.pending_delete,
        vec![terrain_id, projectile_id, wave_id]
    );
    assert!(
        restored
            .production
            .terrain_objects
            .contains_key(&terrain_id)
    );
    assert!(restored.projectiles.get(projectile_id).is_some());
    assert!(restored.waves.get(wave_id).is_some());

    restored.process_pending_delete();
    assert!(
        !restored
            .production
            .terrain_objects
            .contains_key(&terrain_id)
    );
    assert!(restored.projectiles.get(projectile_id).is_none());
    assert!(restored.waves.get(wave_id).is_none());
    assert!(restored.substrate.pending_delete.is_empty());
}

#[test]
fn gsi_05_04_ground_source_and_target_retarget_before_removal_without_expiry() {
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    install_common_raw_terrain(&mut sim, 16, 16, 2, None);

    let target_id = sim.allocate_stable_id();
    insert_entity(&mut sim, target_id, EntityCategory::Unit);
    assert!(matches!(
        sim.try_reveal_entity(target_id, request(9, 11, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed { .. }
    ));
    let target_position = ProjectileCoord::new(
        9 * 256 + 128,
        11 * 256 + 64,
        2 * crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS,
    );
    let projectile_id = sim.allocate_stable_id();
    sim.admit_projectile(
        projectile_id,
        gsi_05_04_guided_projectile(
            target_id,
            ProjectileTarget::Entity(target_id),
            target_position,
        ),
    );
    sim.lifecycle_test_events.clear();

    sim.uninit(target_id);

    let cell_target = ProjectileTarget::Cell { rx: 9, ry: 11 };
    let projectile = sim
        .projectiles
        .get(projectile_id)
        .expect("Bullet remains stored throughout pointer cleanup");
    assert_eq!(projectile.source_id, crate::sim::combat::RAD_NO_ATTACKER);
    assert_eq!(projectile.target, cell_target);
    assert!(projectile.in_logic_vector);
    assert!(sim.substrate.entities.contains(target_id));
    assert!(sim.lifecycle_test_events.iter().any(|event| {
        *event
            == LifecycleTestEvent::ProjectilePointerExpiredVisited {
                expired_id: target_id,
                projectile_id,
                expired_resolvable: true,
                projectile_resolvable: true,
                source_id: crate::sim::combat::RAD_NO_ATTACKER,
                target: cell_target,
            }
    }));

    assert!(sim.object_ai_visit_one(projectile_id, None, ObjectAiCtx::default()));
    assert!(sim.pending_projectile_detonations.is_empty());
    assert!(sim.projectiles.get(projectile_id).is_some());
    assert_eq!(
        sim.projectiles.get(projectile_id).unwrap().target,
        cell_target
    );
}

#[test]
fn gsi_05_04_building_get_coords_uses_foundation_center_cell() {
    let mut sim = Simulation::new();
    sim.session.map_width = 20;
    sim.session.map_height = 20;
    install_common_raw_terrain(&mut sim, 20, 20, 0, None);

    let target_id = sim.allocate_stable_id();
    insert_entity(&mut sim, target_id, EntityCategory::Structure);
    let gapowr = sim.interner.intern("GAPOWR");
    {
        let target = sim.substrate.entities.get_mut(target_id).unwrap();
        target.type_ref = gapowr;
        target.foundation = "2x2".to_string();
    }
    assert!(matches!(
        sim.try_reveal_entity(target_id, common_raw_request(9, 11, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));
    let projectile_id = sim.allocate_stable_id();
    sim.admit_projectile(
        projectile_id,
        gsi_05_04_guided_projectile(
            crate::sim::combat::RAD_NO_ATTACKER,
            ProjectileTarget::Entity(target_id),
            ProjectileCoord::new(9 * 256 + 128, 11 * 256 + 128, 0),
        ),
    );

    sim.uninit(target_id);

    assert_eq!(
        sim.projectiles.get(projectile_id).unwrap().target,
        ProjectileTarget::Cell { rx: 10, ry: 12 },
        "BuildingClass GetCoords shifts GAPOWR's NW anchor by 128 leptons per 2x2 axis before truncation"
    );
}

#[test]
fn gsi_05_04_cell_target_tracks_production_bridge_collapse() {
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    install_common_raw_terrain(&mut sim, 16, 16, 0, Some((6, 7)));

    let target_id = sim.allocate_stable_id();
    insert_entity(&mut sim, target_id, EntityCategory::Unit);
    assert!(matches!(
        sim.try_reveal_entity(target_id, common_raw_request(6, 7, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));
    let projectile_id = sim.allocate_stable_id();
    let bridge_z = crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS;
    let center = ProjectileCoord::new(6 * 256 + 128, 7 * 256 + 128, bridge_z);
    let mut spawn = gsi_05_04_guided_projectile(
        crate::sim::combat::RAD_NO_ATTACKER,
        ProjectileTarget::Entity(target_id),
        center,
    );
    spawn.origin = center;
    spawn.guidance = None;
    spawn.tracks_target = false;
    sim.admit_projectile(projectile_id, spawn);

    sim.uninit(target_id);
    assert_eq!(
        sim.projectiles.get(projectile_id).unwrap().target,
        ProjectileTarget::Cell { rx: 6, ry: 7 }
    );
    assert_eq!(
        cell_target_coord(
            sim.resolved_terrain.as_ref(),
            sim.bridge_state.as_ref(),
            6,
            7
        ),
        center
    );
    {
        let terrain = sim.resolved_terrain.as_ref().expect("resolved terrain");
        let bridge_state = sim.bridge_state.as_mut().expect("bridge runtime state");
        assert!(matches!(
            bridge_state.body_cell_advance_state(6, 7, true, terrain),
            StateOutcome::Absorbed
        ));
        assert!(matches!(
            bridge_state.body_cell_advance_state(6, 7, true, terrain),
            StateOutcome::Collapsed { .. }
        ));
        assert!(!bridge_state.is_bridge_walkable(6, 7));
    }
    assert_eq!(
        cell_target_coord(
            sim.resolved_terrain.as_ref(),
            sim.bridge_state.as_ref(),
            6,
            7
        ),
        ProjectileCoord::new(6 * 256 + 128, 7 * 256 + 128, 0)
    );

    assert!(sim.object_ai_visit_one(projectile_id, None, ObjectAiCtx::default()));

    assert!(sim.pending_projectile_detonations.is_empty());
    let projectile = sim.projectiles.get(projectile_id).expect("Bullet survives");
    assert_eq!(projectile.velocity.z, -64);
    assert_eq!(projectile.position.z, bridge_z - 64);
}

#[test]
fn gsi_05_04_intact_bridge_cell_target_reaches_shrapnel_consumer() {
    let rules =
        crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=MTNK\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Warheads]\n0=WH\n\
             [MTNK]\nStrength=100\nArmor=heavy\nPrimary=PARENT\n\
             [PARENT]\nDamage=0\nROF=10\nRange=6\nSpeed=30\nProjectile=PARENTPROJ\nWarhead=WH\n\
             [PARENTPROJ]\nAirburst=yes\nShrapnelWeapon=CHILD\nShrapnelCount=-2\n\
             [CHILD]\nDamage=5\nROF=10\nRange=3\nSpeed=40\nProjectile=CHILDPROJ\nWarhead=WH\n\
             [CHILDPROJ]\nSubjectToWalls=yes\n\
             [WH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("bridge shrapnel rules");
    let mut sim = Simulation::with_seed(0x46_a310);
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    install_common_raw_terrain(&mut sim, 16, 16, 0, Some((6, 7)));

    let source_id = sim.allocate_stable_id();
    insert_entity(&mut sim, source_id, EntityCategory::Unit);
    let source_type = sim.interner.intern("MTNK");
    sim.substrate.entities.get_mut(source_id).unwrap().type_ref = source_type;
    let projectile_id = sim.allocate_stable_id();
    let detonation = crate::sim::projectile::ProjectileDetonation {
        projectile_id,
        source_id,
        target: ProjectileTarget::Cell { rx: 6, ry: 7 },
        impact: ProjectileCoord::new(6 * 256 + 128, 7 * 256 + 128, 0),
        payload: ProjectilePayload {
            base_damage: 0,
            warhead: sim.interner.intern("WH"),
            weapon: sim.interner.intern("PARENT"),
            owner: sim.interner.intern("Americans"),
        },
        reason: crate::sim::projectile::ProjectileDetonationReason::ReachedTarget,
    };

    assert!(
        sim.bridge_state
            .as_ref()
            .is_some_and(|state| state.is_bridge_walkable(6, 7))
    );
    let result = sim.tick_combat_with_fatal_lifecycle(
        &rules,
        None,
        100,
        &[],
        &std::collections::BTreeSet::new(),
        &[detonation],
        &[],
    );

    assert_eq!(
        result.projectile_spawns.len(),
        1,
        "ShrapnelCount=-2 subtracts the intact deck target's one-cell vertical distance; suppressing the live +416 deck term would emit two children"
    );
}

#[test]
fn gsi_05_04_combat_fatal_expiry_keeps_authoritative_cell_target() {
    let rules =
        crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=VICTIM\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Warheads]\n0=KILLWH\n\
             [VICTIM]\nStrength=10\nArmor=light\n\
             [KILLWH]\nCellSpread=0\nPercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("combat-fatal expiry rules");
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    install_common_raw_terrain(&mut sim, 16, 16, 0, None);

    let victim_id = sim.allocate_stable_id();
    insert_entity(&mut sim, victim_id, EntityCategory::Unit);
    let victim_type = sim.interner.intern("VICTIM");
    {
        let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
        victim.type_ref = victim_type;
        victim.health.current = 10;
        victim.health.max = 10;
    }
    assert!(matches!(
        sim.try_reveal_entity(victim_id, common_raw_request(5, 6, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));

    let listener_id = sim.allocate_stable_id();
    sim.admit_projectile(
        listener_id,
        gsi_05_04_guided_projectile(
            crate::sim::combat::RAD_NO_ATTACKER,
            ProjectileTarget::Entity(victim_id),
            ProjectileCoord::new(5 * 256 + 128, 6 * 256 + 128, 0),
        ),
    );
    let detonation = crate::sim::projectile::ProjectileDetonation {
        projectile_id: 9000,
        source_id: crate::sim::combat::RAD_NO_ATTACKER,
        target: ProjectileTarget::Entity(victim_id),
        impact: ProjectileCoord::new(5 * 256 + 128, 6 * 256 + 128, 0),
        payload: ProjectilePayload {
            base_damage: 10,
            warhead: sim.interner.intern("KILLWH"),
            weapon: sim.interner.intern("MISSINGWEAPON"),
            owner: sim.interner.intern("Americans"),
        },
        reason: crate::sim::projectile::ProjectileDetonationReason::ReachedTarget,
    };
    let logic_order = sim.live_object_order_snapshot();

    let _ = sim.tick_combat_with_fatal_lifecycle(
        &rules,
        None,
        100,
        &logic_order,
        &std::collections::BTreeSet::new(),
        &[detonation],
        &[],
    );

    let victim = sim
        .substrate
        .entities
        .get(victim_id)
        .expect("fatal target remains resolvable before the common drain");
    assert!(!victim.lifecycle.object_alive);
    assert!(victim.lifecycle.in_limbo);
    assert!(sim.substrate.pending_delete.contains(&victim_id));
    assert_eq!(
        sim.projectiles.get(listener_id).unwrap().target,
        ProjectileTarget::Cell { rx: 5, ry: 6 },
        "the synchronous fatal UnInit must read the same allocated CellClass terrain authority as combat before late deletion"
    );
}

#[test]
fn gsi_05_04_combat_fatal_garrison_recursion_keeps_cell_target() {
    let rules =
        crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
            "[InfantryTypes]\n0=OCCUPANT\n\
             [VehicleTypes]\n0=BLOCKER\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n0=GARRISON\n\
             [Warheads]\n0=KILLWH\n\
             [OCCUPANT]\nStrength=10\nArmor=none\n\
             [BLOCKER]\nStrength=100\nArmor=heavy\n\
             [GARRISON]\nStrength=10\nArmor=concrete\nFoundation=2x2\nCanBeOccupied=yes\n\
             [KILLWH]\nCellSpread=0\nPercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("fatal garrison recursion rules");
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    install_common_raw_terrain(&mut sim, 16, 16, 0, None);

    let building_id = sim.allocate_stable_id();
    insert_entity(&mut sim, building_id, EntityCategory::Structure);
    let passenger_id = sim.allocate_stable_id();
    insert_entity(&mut sim, passenger_id, EntityCategory::Infantry);
    let building_type = sim.interner.intern("GARRISON");
    let passenger_type = sim.interner.intern("OCCUPANT");
    {
        let passenger = sim.substrate.entities.get_mut(passenger_id).unwrap();
        passenger.type_ref = passenger_type;
        passenger.passenger_role = PassengerRole::Inside {
            transport_id: building_id,
        };
    }
    {
        let building = sim.substrate.entities.get_mut(building_id).unwrap();
        building.type_ref = building_type;
        building.foundation = "2x2".to_string();
        building.health.current = 10;
        building.health.max = 10;
        let mut cargo = PassengerCargo::new(5, 1);
        assert!(cargo.board(passenger_id, 1));
        building.passenger_role = PassengerRole::Transport { cargo };
    }
    assert!(matches!(
        sim.try_reveal_entity(building_id, common_raw_request(8, 8, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));

    // Destruction's native no-exit arm is selected only when every perimeter
    // probe rejects. Occupied blockers establish that production condition.
    let blocker_type = sim.interner.intern("BLOCKER");
    for (rx, ry) in [
        (10, 10),
        (10, 9),
        (10, 8),
        (10, 7),
        (9, 10),
        (8, 10),
        (7, 10),
        (8, 7),
        (9, 7),
        (7, 8),
        (7, 9),
    ] {
        let blocker_id = sim.allocate_stable_id();
        insert_entity(&mut sim, blocker_id, EntityCategory::Unit);
        sim.substrate.entities.get_mut(blocker_id).unwrap().type_ref = blocker_type;
        assert!(matches!(
            sim.try_reveal_entity(blocker_id, common_raw_request(rx, ry, 0, 128, 128)),
            RevealOutcome::Revealed { .. }
        ));
    }

    let listener_id = sim.allocate_stable_id();
    sim.admit_projectile(
        listener_id,
        gsi_05_04_guided_projectile(
            crate::sim::combat::RAD_NO_ATTACKER,
            ProjectileTarget::Entity(passenger_id),
            ProjectileCoord::new(2 * 256, 3 * 256, 0),
        ),
    );
    let detonation = crate::sim::projectile::ProjectileDetonation {
        projectile_id: 9001,
        source_id: crate::sim::combat::RAD_NO_ATTACKER,
        target: ProjectileTarget::Entity(building_id),
        impact: ProjectileCoord::new(8 * 256 + 128, 8 * 256 + 128, 0),
        payload: ProjectilePayload {
            base_damage: 10,
            warhead: sim.interner.intern("KILLWH"),
            weapon: sim.interner.intern("MISSINGWEAPON"),
            owner: sim.interner.intern("Americans"),
        },
        reason: crate::sim::projectile::ProjectileDetonationReason::ReachedTarget,
    };
    let logic_order = sim.live_object_order_snapshot();

    let _ = sim.tick_combat_with_fatal_lifecycle(
        &rules,
        None,
        100,
        &logic_order,
        &std::collections::BTreeSet::new(),
        &[detonation],
        &[],
    );

    let passenger = sim
        .substrate
        .entities
        .get(passenger_id)
        .expect("no-exit occupant remains resolvable before common drain");
    assert!(!passenger.lifecycle.object_alive);
    assert!(passenger.lifecycle.in_limbo);
    assert!(sim.substrate.pending_delete.contains(&passenger_id));
    assert_eq!(
        sim.projectiles.get(listener_id).unwrap().target,
        ProjectileTarget::Cell { rx: 2, ry: 3 },
        "recursive no-exit passenger UnInit inherits combat's allocated CellClass authority"
    );
}

#[test]
fn gsi_05_04_in_rectangle_native_unallocated_cell_becomes_explicit_null() {
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;
    install_common_raw_terrain(&mut sim, 16, 16, 0, None);

    let target_id = sim.allocate_stable_id();
    insert_entity(&mut sim, target_id, EntityCategory::Unit);
    assert!(matches!(
        sim.try_reveal_entity(target_id, request(8, 8, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed { .. }
    ));

    let allocated = (0..16)
        .flat_map(|ry| (0..16).map(move |rx| (rx, ry)))
        .filter(|&cell| cell != (8, 8))
        .collect::<Vec<_>>();
    sim.resolved_terrain
        .as_mut()
        .expect("terrain")
        .test_set_native_allocated_cells(&allocated);

    let projectile_id = sim.allocate_stable_id();
    sim.admit_projectile(
        projectile_id,
        gsi_05_04_guided_projectile(
            crate::sim::combat::RAD_NO_ATTACKER,
            ProjectileTarget::Entity(target_id),
            ProjectileCoord::new(8 * 256 + 128, 8 * 256 + 128, 0),
        ),
    );

    sim.uninit(target_id);

    assert_eq!(
        sim.projectiles.get(projectile_id).unwrap().target,
        ProjectileTarget::None,
        "the rectangular coordinate has no native-allocated CellClass target"
    );
    assert!(sim.projectiles.get(projectile_id).unwrap().in_logic_vector);
}

#[test]
fn gsi_05_04_high_flying_source_and_target_become_explicit_null() {
    let mut sim = Simulation::new();
    sim.session.map_width = 32;
    sim.session.map_height = 32;

    let target_id = sim.allocate_stable_id();
    install_fly_aircraft(&mut sim, target_id, SimFixed::from_num(208));
    assert!(matches!(
        sim.try_reveal_entity(target_id, request(13, 15, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed { .. }
    ));
    let projectile_id = sim.allocate_stable_id();
    let mut spawn = gsi_05_04_guided_projectile(
        target_id,
        ProjectileTarget::Entity(target_id),
        ProjectileCoord::new(13 * 256 + 128, 15 * 256 + 64, 208),
    );
    spawn.origin = ProjectileCoord::new(1024, 1024, 0);
    sim.admit_projectile(projectile_id, spawn);
    sim.lifecycle_test_events.clear();

    sim.uninit(target_id);

    let projectile = sim.projectiles.get(projectile_id).unwrap();
    assert_eq!(projectile.source_id, crate::sim::combat::RAD_NO_ATTACKER);
    assert_eq!(projectile.target, ProjectileTarget::None);
    assert!(projectile.in_logic_vector);
    assert!(sim.lifecycle_test_events.iter().any(|event| {
        *event
            == LifecycleTestEvent::ProjectilePointerExpiredVisited {
                expired_id: target_id,
                projectile_id,
                expired_resolvable: true,
                projectile_resolvable: true,
                source_id: crate::sim::combat::RAD_NO_ATTACKER,
                target: ProjectileTarget::None,
            }
    }));

    let bytes = GameSnapshot::save(&sim, 0, 0, "null-bullet-target", 0);
    let mut restored = GameSnapshot::load(&bytes)
        .expect("null Bullet target snapshot")
        .sim;
    restored
        .restore_after_snapshot_load()
        .expect("null Bullet target remains valid on restore");
    assert_eq!(
        restored.projectiles.get(projectile_id).unwrap().target,
        ProjectileTarget::None
    );
    assert!(restored.object_ai_visit_one(projectile_id, None, ObjectAiCtx::default()));
    assert!(restored.pending_projectile_detonations.is_empty());
    let advanced = restored.projectiles.get(projectile_id).unwrap();
    assert_eq!(advanced.target, ProjectileTarget::None);
    assert!(
        advanced.velocity.y < 0,
        "guided Bullet AI steers toward native null's zero CoordStruct, not its cached target"
    );
}

#[test]
fn gsi_05_04_off_map_null_cleanup_is_idempotent_across_duplicate_uninit() {
    let mut sim = Simulation::new();
    sim.session.map_width = 8;
    sim.session.map_height = 8;

    let target_id = sim.allocate_stable_id();
    insert_entity(&mut sim, target_id, EntityCategory::Unit);
    assert!(matches!(
        sim.try_reveal_entity(target_id, request(9, 9, PlacementEvidence::MarkSucceeded)),
        RevealOutcome::Revealed { .. }
    ));
    let projectile_id = sim.allocate_stable_id();
    sim.admit_projectile(
        projectile_id,
        gsi_05_04_guided_projectile(
            crate::sim::combat::RAD_NO_ATTACKER,
            ProjectileTarget::Entity(target_id),
            ProjectileCoord::new(9 * 256 + 128, 9 * 256 + 64, 0),
        ),
    );

    sim.uninit(target_id);
    let after_first = sim.projectiles.get(projectile_id).unwrap().clone();
    assert_eq!(after_first.target, ProjectileTarget::None);
    sim.lifecycle_test_events.clear();

    sim.uninit(target_id);

    assert_eq!(sim.projectiles.get(projectile_id).unwrap(), &after_first);
    assert_eq!(sim.substrate.pending_delete, vec![target_id, target_id]);
    assert_eq!(
        sim.lifecycle_test_events
            .iter()
            .filter(|event| matches!(
                event,
                LifecycleTestEvent::ProjectilePointerExpiredVisited {
                    expired_id,
                    projectile_id: visited_id,
                    source_id: crate::sim::combat::RAD_NO_ATTACKER,
                    target: ProjectileTarget::None,
                    ..
                } if *expired_id == target_id && *visited_id == projectile_id
            ))
            .count(),
        1,
        "the repeated UnInit performs one direct, idempotent Bullet callback"
    );
}

#[test]
fn gsi_05_04_projectile_listener_keeps_mixed_object_construction_order() {
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;

    let entity_id = sim.allocate_stable_id();
    insert_entity(&mut sim, entity_id, EntityCategory::Unit);

    let projectile_id = sim.allocate_stable_id();
    let target_id = 5;
    sim.admit_projectile(
        projectile_id,
        gsi_05_04_guided_projectile(
            target_id,
            ProjectileTarget::Entity(target_id),
            ProjectileCoord::new(2 * 256, 3 * 256, 0),
        ),
    );

    let anim_id = sim.allocate_stable_id();
    insert_anim(&mut sim, anim_id, false);
    let particle_id = sim.allocate_stable_id();
    insert_particle_system(&mut sim, particle_id);
    assert_eq!(sim.allocate_stable_id(), target_id);
    insert_entity(&mut sim, target_id, EntityCategory::Unit);
    let _ = sim.reveal(target_id);
    sim.lifecycle_test_events.clear();

    sim.uninit(target_id);

    let visited = sim
        .lifecycle_test_events
        .iter()
        .filter_map(|event| match event {
            LifecycleTestEvent::UninitRemovalListenerVisited {
                expired_id,
                listener_id,
                ..
            } if *expired_id == target_id => Some(*listener_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visited,
        vec![
            entity_id,
            projectile_id,
            anim_id,
            particle_id,
            target_id,
            entity_id,
            projectile_id,
            anim_id,
            particle_id,
            target_id,
        ]
    );
    assert!(sim.lifecycle_test_events.iter().any(|event| matches!(
        event,
        LifecycleTestEvent::ProjectilePointerExpiredVisited {
            expired_id,
            projectile_id: visited_id,
            expired_resolvable: true,
            projectile_resolvable: true,
            ..
        } if *expired_id == target_id && *visited_id == projectile_id
    )));
    assert!(sim.projectiles.get(projectile_id).is_some());
    assert!(sim.projectiles.get(projectile_id).unwrap().in_logic_vector);
}

fn gsi_01_05_damage_rules() -> crate::rules::ruleset::RuleSet {
    crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
        "[InfantryTypes]\n\
             [VehicleTypes]\n0=VICTIM\n1=SUCCESSOR\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n0=DUPBLDG\n\
             [Warheads]\n0=KILLWH\n\
             [VICTIM]\nStrength=30\nArmor=light\n\
             [SUCCESSOR]\nStrength=30\nArmor=light\n\
             [DUPBLDG]\nStrength=10\nArmor=concrete\nFoundation=2x1\n\
             [KILLWH]\nCellSpread=0\nPercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("Logic-slot damage fixture rules")
}

#[test]
fn gsi_01_05_lethal_bullet_commits_receiver_before_retirement_and_double_compaction() {
    let rules = gsi_01_05_damage_rules();
    let mut sim = Simulation::new();
    let victim_id = sim.allocate_stable_id();
    insert_entity(&mut sim, victim_id, EntityCategory::Unit);
    let victim_type = sim.interner.intern("VICTIM");
    {
        let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
        victim.type_ref = victim_type;
        victim.health.current = 10;
        victim.health.max = 10;
    }
    assert!(matches!(
        sim.try_reveal_entity(victim_id, common_raw_request(5, 6, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));

    let successor_id = sim.allocate_stable_id();
    insert_entity(&mut sim, successor_id, EntityCategory::Unit);
    let successor_type = sim.interner.intern("SUCCESSOR");
    sim.substrate
        .entities
        .get_mut(successor_id)
        .unwrap()
        .type_ref = successor_type;
    assert!(matches!(
        sim.try_reveal_entity(successor_id, common_raw_request(8, 9, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));

    let projectile_id = sim.allocate_stable_id();
    let impact = ProjectileCoord::new(5 * 256 + 128, 6 * 256 + 128, 0);
    let mut spawn = gsi_05_02_projectile(crate::sim::combat::RAD_NO_ATTACKER, Some(0));
    spawn.origin = impact;
    spawn.target = ProjectileTarget::Entity(victim_id);
    spawn.initial_target_position = impact;
    spawn.payload = ProjectilePayload {
        base_damage: 10,
        warhead: sim.interner.intern("KILLWH"),
        weapon: sim.interner.intern("MISSINGWEAPON"),
        owner: sim.interner.intern("Americans"),
    };
    sim.admit_projectile(projectile_id, spawn);
    sim.set_logic_order_for_test(vec![projectile_id, victim_id, successor_id]);
    sim.lifecycle_test_events.clear();

    let mut visited = Vec::new();
    sim.for_each_live_object(|sim, id| {
        visited.push(id);
        assert!(sim.object_ai_visit_one(id, Some(&rules), ObjectAiCtx::default()));
    });

    assert_eq!(visited, vec![projectile_id]);
    assert_eq!(sim.live_object_order_snapshot(), vec![successor_id]);
    let victim = sim
        .substrate
        .entities
        .get(victim_id)
        .expect("fatal receiver remains resolvable until common drain");
    assert!(!victim.lifecycle.object_alive);
    assert!(victim.lifecycle.in_limbo);
    assert_eq!(victim.mission.ai_counter(), 0);
    assert_eq!(
        sim.substrate
            .entities
            .get(successor_id)
            .unwrap()
            .mission
            .ai_counter(),
        0,
        "receiver removal followed by current Bullet removal skips the shifted successor"
    );
    assert_eq!(sim.substrate.pending_delete, vec![victim_id, projectile_id]);
    assert!(sim.pending_projectile_detonations.is_empty());
    assert!(sim.projectiles.get(projectile_id).is_some());
    assert!(!sim.projectiles.get(projectile_id).unwrap().in_logic_vector);

    let queued = sim
        .lifecycle_test_events
        .iter()
        .filter_map(|event| match event {
            LifecycleTestEvent::PendingDeleteQueued { stable_id } => Some(*stable_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        queued,
        vec![victim_id, projectile_id],
        "fatal receiver UnInit completes before Bullet UnInit/retirement"
    );
}

#[test]
fn gsi_01_05_terminal_wave_damages_once_before_single_current_removal() {
    let rules = gsi_01_05_damage_rules();
    let mut sim = Simulation::new();
    let victim_id = sim.allocate_stable_id();
    insert_entity(&mut sim, victim_id, EntityCategory::Unit);
    let victim_type = sim.interner.intern("VICTIM");
    sim.substrate.entities.get_mut(victim_id).unwrap().type_ref = victim_type;
    sim.substrate.entities.get_mut(victim_id).unwrap().health = Health {
        current: 30,
        max: 30,
    };
    assert!(matches!(
        sim.try_reveal_entity(victim_id, common_raw_request(4, 5, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));

    let successor_id = sim.allocate_stable_id();
    insert_entity(&mut sim, successor_id, EntityCategory::Unit);
    let successor_type = sim.interner.intern("SUCCESSOR");
    sim.substrate
        .entities
        .get_mut(successor_id)
        .unwrap()
        .type_ref = successor_type;
    assert!(matches!(
        sim.try_reveal_entity(successor_id, common_raw_request(8, 9, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));

    let wave_id = sim.allocate_stable_id();
    let mut wave = Wave::new(
        0,
        ProjectileCoord::new(4 * 256, 5 * 256, 0),
        ProjectileCoord::new(5 * 256, 5 * 256, 0),
    )
    .with_damage_payload(WaveDamagePayload {
        firer_id: crate::sim::combat::RAD_NO_ATTACKER,
        base_damage: 10,
        warhead: sim.interner.intern("KILLWH"),
    });
    wave.lifetime = 0;
    wave.replace_recorded_cells(vec![WaveRecordedCell { rx: 4, ry: 5 }]);
    sim.admit_wave(wave_id, wave);
    sim.set_logic_order_for_test(vec![wave_id, victim_id, successor_id]);
    sim.lifecycle_test_events.clear();

    let mut visited = Vec::new();
    sim.for_each_live_object(|sim, id| {
        visited.push(id);
        assert!(sim.object_ai_visit_one(id, Some(&rules), ObjectAiCtx::default()));
    });

    assert_eq!(visited, vec![wave_id, successor_id]);
    assert_eq!(
        sim.substrate
            .entities
            .get(victim_id)
            .unwrap()
            .health
            .current,
        20,
        "Wave receiver fires once at its Logic slot and is not replayed later"
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(victim_id)
            .unwrap()
            .mission
            .ai_counter(),
        0,
        "retiring the current Wave shifts and skips the victim slot once"
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(successor_id)
            .unwrap()
            .mission
            .ai_counter(),
        1
    );
    assert!(sim.pending_wave_damage_requests.is_empty());
    assert_eq!(sim.substrate.pending_delete, vec![wave_id]);
    assert_eq!(
        sim.live_object_order_snapshot(),
        vec![victim_id, successor_id]
    );
    assert!(sim.waves.get(wave_id).is_some());
    assert!(!sim.waves.get(wave_id).unwrap().in_logic_vector);
    assert_eq!(
        sim.lifecycle_test_events
            .iter()
            .filter(|event| {
                **event == (LifecycleTestEvent::PendingDeleteQueued { stable_id: wave_id })
            })
            .count(),
        1
    );
}

#[test]
fn gsi_01_05_wave_reselects_live_cell_list_after_fatal_receiver_unmark() {
    let rules = gsi_01_05_damage_rules();
    let mut sim = Simulation::new();
    sim.session.map_width = 16;
    sim.session.map_height = 16;

    let building_id = sim.allocate_stable_id();
    insert_entity(&mut sim, building_id, EntityCategory::Structure);
    let building_type = sim.interner.intern("DUPBLDG");
    {
        let building = sim.substrate.entities.get_mut(building_id).unwrap();
        building.type_ref = building_type;
        building.foundation = "2x1".to_string();
        building.health.current = 10;
        building.health.max = 10;
    }
    assert!(matches!(
        sim.try_reveal_entity(building_id, common_raw_request(4, 5, 0, 128, 128)),
        RevealOutcome::Revealed { .. }
    ));
    assert!(sim.substrate.occupancy.contains_entity(4, 5, building_id));
    assert!(sim.substrate.occupancy.contains_entity(5, 5, building_id));

    let wave_id = sim.allocate_stable_id();
    let mut wave = Wave::new(
        0,
        ProjectileCoord::new(4 * 256, 5 * 256, 0),
        ProjectileCoord::new(6 * 256, 5 * 256, 0),
    )
    .with_damage_payload(WaveDamagePayload {
        firer_id: crate::sim::combat::RAD_NO_ATTACKER,
        base_damage: 10,
        warhead: sim.interner.intern("KILLWH"),
    });
    wave.lifetime = 0;
    wave.replace_recorded_cells(vec![
        WaveRecordedCell { rx: 4, ry: 5 },
        WaveRecordedCell { rx: 5, ry: 5 },
    ]);
    sim.admit_wave(wave_id, wave);
    sim.set_logic_order_for_test(vec![wave_id, building_id]);
    sim.lifecycle_test_events.clear();

    assert!(sim.object_ai_visit_one(wave_id, Some(&rules), ObjectAiCtx::default()));

    let selected = sim
        .lifecycle_test_events
        .iter()
        .filter_map(|event| match event {
            LifecycleTestEvent::WaveDamageReceiverSelected {
                wave_id: selected_wave,
                target_id,
            } if *selected_wave == wave_id => Some(*target_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        selected,
        vec![building_id],
        "fatal UnInit from the first cell removes the 2x1 foundation entry before the second recorded cell selects its current occupants"
    );
    let building = sim.substrate.entities.get(building_id).unwrap();
    assert!(!building.lifecycle.object_alive);
    assert!(building.lifecycle.in_limbo);
    assert!(!sim.substrate.occupancy.contains_entity(4, 5, building_id));
    assert!(!sim.substrate.occupancy.contains_entity(5, 5, building_id));
    assert_eq!(sim.substrate.pending_delete, vec![building_id, wave_id]);
    assert!(sim.pending_wave_damage_requests.is_empty());
}

#[test]
fn gsi_05_02_restore_rejects_each_live_modeled_family_missing_from_logic() {
    let mut terrain = Simulation::new();
    let terrain_id = terrain.allocate_stable_id();
    terrain.production.terrain_objects.insert(
        terrain_id,
        TerrainObjectState {
            stable_id: terrain_id,
            in_logic_vector: false,
            type_ref: terrain.interner.intern("TREE01"),
            rx: 1,
            ry: 1,
            health: 1,
            max_health: 1,
            occupation_bits: 0,
            lifecycle: TerrainObjectLifecycle::Live,
        },
    );
    assert_eq!(
        terrain.restore_after_snapshot_load(),
        Err(SnapshotRestoreError::MissingRequiredLogicIdentity {
            registry: "TerrainObjectStore",
            object_id: terrain_id,
        })
    );

    let mut projectile = Simulation::new();
    let projectile_id = projectile.allocate_stable_id();
    projectile
        .projectiles
        .spawn(projectile_id, gsi_05_02_projectile(0, None));
    assert_eq!(
        projectile.restore_after_snapshot_load(),
        Err(SnapshotRestoreError::MissingRequiredLogicIdentity {
            registry: "ProjectileStore",
            object_id: projectile_id,
        })
    );

    let mut wave = Simulation::new();
    let wave_id = wave.allocate_stable_id();
    wave.waves.spawn(
        wave_id,
        Wave::new(
            3,
            ProjectileCoord::new(0, 0, 0),
            ProjectileCoord::new(1, 0, 0),
        ),
    );
    assert_eq!(
        wave.restore_after_snapshot_load(),
        Err(SnapshotRestoreError::MissingRequiredLogicIdentity {
            registry: "WaveStore",
            object_id: wave_id,
        })
    );
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
