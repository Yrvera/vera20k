//! Focused regression tests for ordered lifecycle authority.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::anim_class::{AnimObject, AnimRuntime, AnimWorldCoord};
use crate::sim::animation::{Animation, SequenceKind};
use crate::sim::combat::{AttackTarget, PendingInfantryFire, TargetKind};
use crate::sim::components::{C4PlantState, Health, NavTargetRef};
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::mission::state::MissionTestFixture;
use crate::sim::mission::{MissionDispatchTimer, MissionId, MissionType};
use crate::sim::movement::homing_movement::{HomingTarget, attach_homing_state};
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
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
