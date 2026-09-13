//! Rust production-route regressions for the instance lifetime documented in
//! `locomotor_owner`. These supplied states do not certify native command or
//! warp behavior beyond the constructor/transfer/reuse boundaries cited there.

use std::collections::BTreeMap;

use super::*;
use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::command::Command;
use crate::sim::components::{DriveCoord, Health};
use crate::sim::movement::locomotion::{LocomotorRuntimePayload, LocomotorSlot};
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::movement::{self, teleport_movement};
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

fn fixture() -> (Simulation, RuleSet) {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=CMIN\n[CMIN]\nStrength=400\nSpeed=4\n\
         Harvester=yes\nTeleporter=yes\nMovementZone=Normal\n\
         Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}\n",
    ))
    .expect("CMIN fixture rules");
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    let type_id = sim.interner.intern("CMIN");
    let mut entity = GameEntity::new_at_frame_zero_for_test(
        1,
        8,
        8,
        0,
        0,
        owner,
        Health {
            current: 400,
            max: 400,
        },
        type_id,
        EntityCategory::Unit,
        0,
        5,
        true,
    );
    entity.locomotor = Some(LocomotorState::from_object_type(
        rules.object("CMIN").expect("CMIN type"),
        rules.general.flight_level,
        0,
    ));
    entity.lifecycle.in_limbo = false;
    sim.substrate.entities.insert(entity);
    (sim, rules)
}

fn curve() -> DriveTrackState {
    DriveTrackState {
        raw_track_index: 1,
        point_index: 3,
        residual: 5,
        transform_flags: 0,
        head_offset_x: 128,
        head_offset_y: 128,
        cell_offset_x: 0,
        cell_offset_y: 0,
        target_facing: 0,
    }
}

fn supply_drive_state(entity: &mut GameEntity) {
    entity.drive_locomotion = Some(DriveLocomotionRuntime {
        // Is_Moving compares exact XY only. Retained Z deliberately differs
        // from the owner's height, so retirement cannot depend on full XYZ.
        head_to: Some(DriveCoord::cell(8, 8, 731)),
        track: crate::sim::components::TrackProgress {
            turn_index: 1,
            cursor: 3,
            residual: 971,
            ..Default::default()
        },
        current_speed_fraction: SimFixed::lit("0.5"),
        owner_current_speed: 11,
        ..Default::default()
    });
    entity.drive_track = Some(curve());
}

fn activate_drive(entity: &mut GameEntity) {
    assert!(begin_drive_for_teleporter(entity, 19));
    supply_drive_state(entity);
}

fn owned_state(entity: &GameEntity) -> serde_json::Value {
    serde_json::to_value((
        &entity.locomotor,
        &entity.drive_locomotion,
        &entity.drive_track,
        &entity.forced_drive_track,
    ))
    .expect("serialized locomotor and external instance state")
}

fn assert_retired(entity: &GameEntity) {
    let locomotor = entity.locomotor.as_ref().expect("restored locomotor");
    assert_eq!(locomotor.active_kind(), LocomotorKind::Teleport);
    assert!(locomotor.piggyback.is_none());
    assert!(entity.drive_locomotion.is_none());
    assert!(entity.drive_track.is_none());
    assert!(entity.forced_drive_track.is_none());
}

fn destination(sim: &mut Simulation, rules: &RuleSet, building: bool) -> bool {
    let grid = PathGrid::test_all_passable(16, 16);
    movement::set_destination_for_teleporter_entity(
        &mut sim.substrate.entities,
        Some(&grid),
        1,
        (12, 8),
        SimFixed::from_num(6),
        false,
        None,
        None,
        None,
        None,
        None,
        false,
        &rules.general,
        true,
        true,
        building,
        None,
        37,
    )
}

#[test]
fn empty_destination_retires_drive_before_starting_primary_teleport() {
    let (mut sim, rules) = fixture();
    activate_drive(sim.substrate.entities.get_mut(1).unwrap());

    assert!(destination(&mut sim, &rules, false));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert_retired(entity);
    assert!(entity.teleport_state.is_some());
}

#[test]
fn foot_ai_restore_retires_same_drive_fields_as_direct_destination() {
    let (mut sim, _) = fixture();
    activate_drive(sim.substrate.entities.get_mut(1).unwrap());

    assert!(movement::tick_locomotor_piggyback_restore_one(
        &mut sim.substrate.entities,
        1
    ));
    assert_retired(sim.substrate.entities.get(1).unwrap());
    assert!(!movement::tick_locomotor_piggyback_restore_one(
        &mut sim.substrate.entities,
        1
    ));
}

#[test]
fn refused_restore_keeps_live_head_and_forced_segment() {
    for forced in [false, true] {
        let (mut sim, rules) = fixture();
        let entity = sim.substrate.entities.get_mut(1).unwrap();
        activate_drive(entity);
        if forced {
            entity.forced_drive_track = Some(ForcedDriveTrackState {
                turn_track_index: 0x47,
                track: curve(),
                speed: SimFixed::from_num(6),
            });
        } else {
            entity.drive_locomotion.as_mut().unwrap().head_to = Some(DriveCoord::cell(9, 8, 731));
        }
        let before = owned_state(entity);

        assert!(!destination(&mut sim, &rules, false));
        assert!(!movement::tick_locomotor_piggyback_restore_one(
            &mut sim.substrate.entities,
            1
        ));
        let entity = sim.substrate.entities.get(1).unwrap();
        assert_eq!(owned_state(entity), before);
        assert!(entity.teleport_state.is_none());
    }
}

#[test]
fn building_destination_installs_fresh_drive_without_previous_instance_state() {
    let (mut sim, rules) = fixture();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    // A saved state produced by the old direct-END path could leave these
    // fields attached to primary Teleport. They must not become a new Drive.
    supply_drive_state(entity);
    entity.forced_drive_track = Some(ForcedDriveTrackState {
        turn_track_index: 0x47,
        track: curve(),
        speed: SimFixed::from_num(6),
    });

    assert!(destination(&mut sim, &rules, true));

    let entity = sim.substrate.entities.get(1).unwrap();
    let locomotor = entity.locomotor.as_ref().unwrap();
    assert_eq!(locomotor.active_kind(), LocomotorKind::Drive);
    assert_eq!(locomotor.effective_kind(), LocomotorKind::Teleport);
    assert!(entity.movement_target.is_some());
    assert!(entity.forced_drive_track.is_none());
    let drive = entity.drive_locomotion.as_ref().unwrap();
    assert_eq!(drive.track.residual, 0);
    assert_eq!(drive.current_speed_fraction, SIM_ZERO);
    assert_eq!(drive.owner_current_speed, 0);
    if let Some(track) = &entity.drive_track {
        assert_eq!(track.residual, 0);
    }
}

#[test]
fn reusing_active_drive_keeps_complete_instance_including_forced_track() {
    let (mut sim, _) = fixture();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    activate_drive(entity);
    entity.forced_drive_track = Some(ForcedDriveTrackState {
        turn_track_index: 0x47,
        track: curve(),
        speed: SimFixed::from_num(6),
    });
    let before = owned_state(entity);

    assert!(begin_drive_for_teleporter(entity, 900));
    assert_eq!(owned_state(entity), before);
}

#[test]
fn refused_installation_and_absent_stash_do_not_retire_external_state() {
    for state in [
        None,
        Some(LocomotorState::for_test_kind(LocomotorKind::Drive)),
        {
            let mut incoherent = LocomotorState::for_test_kind(LocomotorKind::Drive);
            incoherent.slot = LocomotorSlot::from_kind(LocomotorKind::Teleport);
            Some(incoherent)
        },
    ] {
        let (mut sim, _) = fixture();
        let entity = sim.substrate.entities.get_mut(1).unwrap();
        entity.locomotor = state;
        supply_drive_state(entity);
        let before = owned_state(entity);

        assert!(!begin_drive_for_teleporter(entity, 37));
        assert!(!try_restore_primary(entity));
        assert!(!restore_admitted_primary(entity));
        assert_eq!(owned_state(entity), before);
    }
}

#[test]
fn stop_command_retires_only_the_drive_admitted_by_its_existing_gate() {
    // Stop's existing swap policy is VERA-specific. This checks the changed
    // transfer lifetime and preserves its prior admission timing.
    for head_ahead in [false, true] {
        let (mut sim, rules) = fixture();
        let entity = sim.substrate.entities.get_mut(1).unwrap();
        activate_drive(entity);
        if head_ahead {
            entity.drive_locomotion.as_mut().unwrap().head_to = Some(DriveCoord::cell(9, 8, 731));
        }
        assert!(sim.apply_command(
            "Americans",
            &Command::Stop { entity_id: 1 },
            Some(&rules),
            None,
            &BTreeMap::new(),
        ));

        let entity = sim.substrate.entities.get(1).unwrap();
        if head_ahead {
            assert_eq!(
                entity.locomotor.as_ref().unwrap().active_kind(),
                LocomotorKind::Drive
            );
            assert_eq!(
                entity.drive_locomotion.as_ref().unwrap().head_to,
                Some(DriveCoord::cell(9, 8, 731))
            );
            assert_eq!(entity.drive_track.as_ref().unwrap().residual, 5);
        } else {
            assert_retired(entity);
        }
    }
}

#[test]
fn finished_teleport_restores_suspended_drive_without_retiring_its_state() {
    let (mut sim, rules) = fixture();
    let entity = sim.substrate.entities.get_mut(1).unwrap();
    entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
    supply_drive_state(entity);
    let before = owned_state(entity);

    assert!(teleport_movement::issue_teleport_command(
        &mut sim.substrate.entities,
        1,
        (12, 8),
        &rules.general,
        true,
        37,
    ));
    teleport_movement::tick_teleport_movement(
        &mut sim.substrate.entities,
        &mut sim.substrate.occupancy,
        &[1],
        1,
        None,
    );

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(entity.teleport_state.is_none());
    assert_eq!(
        entity.locomotor.as_ref().unwrap().layer,
        MovementLayer::Ground
    );
    assert_eq!(owned_state(entity), before);
}

#[test]
fn failed_miner_path_restores_full_payload_and_external_instance_state() {
    for stale_fields in [false, true] {
        let (mut sim, rules) = fixture();
        let entity = sim.substrate.entities.get_mut(1).unwrap();
        if stale_fields {
            supply_drive_state(entity);
        }
        let before = owned_state(entity);
        let mut grid = PathGrid::test_all_blocked(16, 16);
        grid.set_blocked(8, 8, false);
        grid.set_blocked(12, 8, false);

        assert!(
            !crate::sim::miner::miner_system::issue_stock_miner_drive_move(
                &mut sim,
                &rules,
                &grid,
                1,
                (12, 8),
            )
        );

        let entity = sim.substrate.entities.get(1).unwrap();
        assert_eq!(owned_state(entity), before);
        assert!(matches!(
            entity.locomotor.as_ref().unwrap().runtime_payload,
            LocomotorRuntimePayload::Teleport(None)
        ));
        assert!(entity.movement_target.is_none());
    }
}
