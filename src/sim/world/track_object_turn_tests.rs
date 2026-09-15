use super::*;
use crate::rules::ini_parser::IniFile;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::{
    DriveCoord, DriveLocomotionRuntime, FootPathQueue, MovementTarget, NavTargetRef, TrackProgress,
};
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::drive_track;
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::util::fixed_math::SimFixed;

fn passive_rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str("[VehicleTypes]\n0=MTNK\n[MTNK]\nStrength=500\nSpeed=6\nSensorsSight=1\nWalkRate=1\nIdleRate=0\nPassive=yes\n")).unwrap()
}

fn fixture() -> (Simulation, RuleSet) {
    let rules = RuleSet::from_ini(&IniFile::from_str("[VehicleTypes]\n0=MTNK\n[MTNK]\nStrength=500\nSpeed=6\nSensorsSight=1\nWalkRate=1\nIdleRate=0\n")).unwrap();
    let mut sim = Simulation::new();
    sim.fog.width = 32;
    sim.fog.height = 32;
    sim.session.binary_frame = 10;
    let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 10, 10);
    entity.category = EntityCategory::Unit;
    entity.lifecycle.object_alive = true;
    entity.lifecycle.in_limbo = false;
    entity.lifecycle.cell_marked = true;
    entity.is_voxel = false;
    entity.drive_accelerates = false;
    entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
    entity.drive_locomotion = Some(DriveLocomotionRuntime {
        head_to: Some(DriveCoord::cell(10, 9, 0)),
        track: TrackProgress {
            turn_index: 0,
            cursor: 0,
            reversed: false,
            residual: 0,
        },
        ..Default::default()
    });
    entity.drive_track = drive_track::begin_drive_track_with_head_offset(1, 0, 128, -128, 0);
    entity.movement_target = Some(MovementTarget {
        path: vec![(10, 10), (10, 9)],
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(330),
        current_speed: SimFixed::from_num(330),
        ..Default::default()
    });
    sim.interner = crate::sim::intern::test_interner();
    entity.navigation.path_runtime.start_blocked(10, 9, false);
    sim.substrate.entities.insert(entity);
    sim.substrate.occupancy =
        crate::sim::occupancy::OccupancyGrid::rebuild(&sim.substrate.entities);
    (sim, rules)
}

#[test]
fn ordinary_object_turn_pays_multiple_points_and_runs_prefix_and_shp_once() {
    let (mut sim, rules) = fixture();
    let outcome = sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(outcome.movement.moved_steps >= 2);
    assert!(entity.drive_locomotion.as_ref().unwrap().track.cursor >= 2);
    assert_eq!(
        entity
            .navigation
            .path_runtime
            .blocked_timer
            .remaining(sim.session.binary_frame as i32),
        8
    );
    assert_eq!(entity.body_frame_counter, 1);
    assert_eq!(
        entity.drive_track.as_ref().unwrap().point_index,
        0,
        "legacy geometry is not a second cursor authority"
    );
}

#[test]
fn terminal_sensor_receiver_finishes_before_shp_and_is_not_deferred_to_cell_change() {
    let (mut sim, rules) = fixture();
    // Install a real sensor deposit at the old recorded center, then supply
    // the terminal in the current cell. PerCell2 must remove/add even though
    // this object's cell is unchanged during its movement visit.
    sim.substrate.entities.get_mut(1).unwrap().position.rx = 5;
    sim.add_unit_sensor_after_unlimbo(1, &rules);
    let owner = sim.substrate.entities.get(1).unwrap().owner();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.position.rx = 10;
    entity.navigation.nav_com = Some(NavTargetRef::cell(10, 10));
    let drive = entity.drive_locomotion.as_mut().unwrap();
    drive.head_to = Some(DriveCoord::cell(10, 10, 0));
    drive.destination = drive.head_to;
    drive.track.cursor = drive_track::raw_track_points(1).len() as i32;
    entity.body_frame_counter = 5;
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(entity.sensor_deposit.unwrap().center, (10, 10));
    assert!(!sim.fog.has_sensor_for_house(owner, 5, 10));
    assert!(sim.fog.has_sensor_for_house(owner, 10, 10));
    assert!(entity.navigation.nav_com.is_none());
    assert!(entity.movement_target.is_none());
    assert_eq!(
        entity.body_frame_counter, 5,
        "terminal retired movement before IdleRate=0 SHP admission"
    );
    assert!(entity.foot_occupation_enabled);
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(10, 10),
        0,
        "same-cell terminal performs no extra raw mark"
    );
}

#[test]
fn accepted_chain_runs_sensor_callback_and_consumes_more_paid_points_in_same_object_turn() {
    let (mut sim, _) = fixture();
    let rules = passive_rules();
    sim.add_unit_sensor_after_unlimbo(1, &rules);
    let selection = drive_track::select_drive_track(32, 64, false).unwrap();
    assert!(selection.entry_index > 0);
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.facing = 32;
    entity.navigation.path_replay = FootPathQueue {
        directions: vec![2, 3],
        cursor: 0,
        reference_cell: Some((10, 10)),
    };
    entity.drive_locomotion.as_mut().unwrap().track = TrackProgress {
        turn_index: 1,
        cursor: i32::from(drive_track::raw_track_meta(3).unwrap().chain_index),
        reversed: false,
        residual: 0,
    };
    let outcome = sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    let entity = sim.substrate.entities.get(1).unwrap();
    let track = entity.drive_locomotion.as_ref().unwrap().track;
    assert_eq!(track.turn_index, selection.turn_track_index as i32);
    assert!(entity.drive_locomotion.as_ref().unwrap().track_valid);
    assert!(track.cursor > i32::from(selection.entry_index));
    assert_eq!(entity.navigation.path_replay.cursor, 1);
    // Retail RawTrack[3]7E7A58 -> point37 at7E66B4=(-136,136,32).
    // TurnTrack[1]7E7B34 flags8 leaves XY unchanged in Transform4B4780.
    // With head(2688,2432), PerCell4B1CFD runs in signed cell(9,10).
    assert_eq!(entity.sensor_deposit.unwrap().center, (9, 10));
    assert_ne!((entity.position.rx, entity.position.ry), (9, 10));
    assert!(outcome.movement.moved_steps >= 2);
    assert_eq!(
        entity
            .navigation
            .path_runtime
            .blocked_timer
            .remaining(sim.session.binary_frame as i32),
        8
    );
}

#[test]
fn command_prepared_track_applies_raw_head_once_even_without_a_paid_point_and_after_load() {
    use crate::sim::snapshot::GameSnapshot;
    for kind in [LocomotorKind::Drive, LocomotorKind::Ship] {
        for reload in [false, true] {
            let (mut sim, rules) = fixture();
            let entity = sim.substrate.entities.get_mut(1).unwrap();
            entity.drive_track = None;
            entity.drive_locomotion = None;
            entity.ship_locomotion = None;
            entity.movement_target = None;
            entity.locomotor = Some(LocomotorState::for_test_kind(kind));
            let grid = crate::sim::pathfinding::PathGrid::new(32, 32);
            assert!(crate::sim::movement::issue_move_command(
                &mut sim.substrate.entities,
                &grid,
                1,
                (10, 9),
                SimFixed::from_num(0),
                false,
                None,
                None,
                None,
                false,
                crate::sim::movement::DestinationTiming::new(0, 60),
            ));
            let pending = |entity: &GameEntity| {
                if kind == LocomotorKind::Drive {
                    entity
                        .drive_locomotion
                        .as_ref()
                        .unwrap()
                        .pending_track_occupation
                } else {
                    entity
                        .ship_locomotion
                        .as_ref()
                        .unwrap()
                        .pending_track_occupation
                }
            };
            assert!(pending(sim.substrate.entities.get(1).unwrap()));
            if kind == LocomotorKind::Drive {
                assert!(
                    sim.substrate
                        .entities
                        .get(1)
                        .unwrap()
                        .drive_locomotion
                        .as_ref()
                        .unwrap()
                        .track_valid
                );
            }
            if reload {
                let bytes = GameSnapshot::save(&sim, 0, 0, "pending_apply", 0);
                sim = GameSnapshot::load(&bytes).unwrap().sim;
                assert!(pending(sim.substrate.entities.get(1).unwrap()));
            }
            sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
            let entity = sim.substrate.entities.get(1).unwrap();
            assert!(!pending(entity));
            if kind == LocomotorKind::Drive {
                let drive = entity.drive_locomotion.as_ref().unwrap();
                assert!(drive.track_valid);
                assert_eq!((drive.track.cursor, drive.track.residual), (0, 0));
                assert_eq!(entity.foot_speed.cached_current_speed, 0);
            }
            assert_eq!(
                sim.substrate.raw_cell_occupation.ground_bits(10, 9) & 0x20,
                0x20
            );
            sim.substrate.raw_cell_occupation.clear_ground(10, 9, 0x20);
            sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
            assert_eq!(
                sim.substrate.raw_cell_occupation.ground_bits(10, 9) & 0x20,
                0,
                "cursor0 does not replay Apply1"
            );
        }
    }
}

#[test]
fn terminal_arrival_resets_owner_speed_before_next_accelerating_move() {
    let (mut sim, rules) = fixture();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.navigation.nav_com = Some(NavTargetRef::cell(10, 9));
    let drive = entity.drive_locomotion.as_mut().unwrap();
    drive.destination = drive.head_to;
    drive.track.cursor = drive_track::raw_track_points(1).len() as i32;
    entity.foot_speed.applied_fraction = SimFixed::from_num(1);
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    assert_eq!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .foot_speed
            .applied_fraction,
        SimFixed::from_num(0)
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .foot_speed
            .cached_current_speed,
        0
    );
    let grid = crate::sim::pathfinding::PathGrid::new(32, 32);
    assert!(crate::sim::movement::issue_move_command(
        &mut sim.substrate.entities,
        &grid,
        1,
        (10, 6),
        SimFixed::from_num(330),
        false,
        None,
        None,
        None,
        false,
        crate::sim::movement::DestinationTiming::new(0, 60),
    ));
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.drive_accelerates = true;
    entity.movement_target.as_mut().unwrap().accel_factor = SimFixed::from_num(0.03);
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    assert_eq!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .foot_speed
            .applied_fraction,
        SimFixed::from_num(0.03)
    );
}

#[test]
fn ship_fresh_claim_survives_next_object_visit_and_snapshot_rebuild() {
    use crate::sim::snapshot::GameSnapshot;
    let (mut sim, rules) = fixture();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Ship));
    entity.drive_locomotion = None;
    entity.ship_locomotion = None;
    entity.drive_track = None;
    entity.movement_target = None;
    let grid = crate::sim::pathfinding::PathGrid::new(32, 32);
    assert!(crate::sim::movement::issue_move_command(
        &mut sim.substrate.entities,
        &grid,
        1,
        (10, 7),
        SimFixed::from_num(330),
        false,
        None,
        None,
        None,
        false,
        crate::sim::movement::DestinationTiming::new(0, 60),
    ));
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    let mark = sim
        .substrate
        .entities
        .get(1)
        .unwrap()
        .ship_locomotion
        .as_ref()
        .unwrap()
        .occupation_head_to
        .unwrap();
    assert!(
        !sim.substrate
            .entities
            .get(1)
            .unwrap()
            .foot_occupation_enabled
    );
    sim.substrate
        .entities
        .get_mut(1)
        .unwrap()
        .movement_target
        .as_mut()
        .unwrap()
        .speed = SimFixed::from_num(15);
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    assert!(
        sim.substrate
            .cell_occupation
            .occupied_by_other(mark.rx, mark.ry, mark.layer, 99)
    );
    let bytes = GameSnapshot::save(&sim, 0, 0, "ship_head", 0);
    let loaded = GameSnapshot::load(&bytes).unwrap().sim;
    let rebuilt = crate::sim::occupancy::CellOccupationGrid::rebuild(&loaded.substrate.entities);
    assert!(
        rebuilt.occupied_by_other(mark.rx, mark.ry, mark.layer, 99),
        "follower entry sees the saved Ship reservation"
    );
    assert_eq!(
        loaded
            .substrate
            .raw_cell_occupation
            .ground_bits(mark.rx, mark.ry)
            & 0x20,
        0x20
    );
}

#[test]
fn ordinary_default_passive_false_does_not_accept_a_chain() {
    let (mut sim, rules) = fixture();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.drive_locomotion.as_mut().unwrap().track = TrackProgress {
        turn_index: 1,
        cursor: 37,
        reversed: false,
        residual: 0,
    };
    entity.navigation.path_replay = FootPathQueue {
        directions: vec![2, 3],
        cursor: 0,
        reference_cell: Some((10, 10)),
    };
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        entity.drive_locomotion.as_ref().unwrap().track.turn_index,
        1
    );
    assert_eq!(entity.navigation.path_replay.cursor, 0);
}

#[test]
fn bridge_terminal_uses_owner_height_to_reach_ground_navcom_target() {
    let (mut sim, rules) = fixture();
    let mut grid = crate::sim::pathfinding::PathGrid::new(32, 32);
    grid.set_cell_for_test(10, 9, 0, true, false);
    sim.path_grid = Some(std::sync::Arc::new(grid));
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.on_bridge = true;
    entity.navigation.nav_com = Some(NavTargetRef::cell(10, 9));
    entity.foot_speed.applied_fraction = SimFixed::from_num(1);
    let drive = entity.drive_locomotion.as_mut().unwrap();
    let deck = DriveCoord::cell(10, 9, 416);
    drive.head_to = Some(deck);
    drive.destination = Some(deck);
    drive.track.cursor = drive_track::raw_track_points(1).len() as i32;
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(entity.position.exact_z_leptons, Some(416));
    assert!(entity.navigation.nav_com.is_none());
    assert!(
        entity
            .drive_locomotion
            .as_ref()
            .unwrap()
            .destination
            .is_none()
    );
    assert!(entity.movement_target.is_none());
    assert_eq!(entity.foot_speed.applied_fraction, SimFixed::from_num(0));
}

#[test]
fn terminal_piggyback_end_is_synchronous_and_preserves_owner_speed() {
    let (mut sim, rules) = fixture();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Teleport));
    assert!(crate::sim::movement::locomotor_owner::begin_drive_for_teleporter(entity, 10));
    entity.drive_locomotion = Some(DriveLocomotionRuntime {
        head_to: Some(DriveCoord::cell(10, 9, 0)),
        destination: Some(DriveCoord::cell(10, 9, 0)),
        track: TrackProgress {
            turn_index: 0,
            cursor: drive_track::raw_track_points(1).len() as i32,
            ..Default::default()
        },
        ..Default::default()
    });
    entity.navigation.nav_com = Some(NavTargetRef::cell(10, 9));
    entity.locomotor.as_mut().unwrap().phase =
        crate::sim::movement::locomotor::GroundMovePhase::Cruising;
    entity.foot_speed.applied_fraction = SimFixed::from_num(1);
    sim.advance_live_object_turn(1, Some(&rules), techno_ai::ObjectAiCtx::default());
    let entity = sim.substrate.entities.get(1).unwrap();
    assert_eq!(
        entity.locomotor.as_ref().unwrap().kind,
        LocomotorKind::Teleport
    );
    assert!(entity.drive_locomotion.is_none());
    assert_eq!(entity.foot_speed.applied_fraction, SimFixed::from_num(1));
    assert!(entity.navigation.nav_com.is_none());
}
