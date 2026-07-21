//! Focused regression tests for ordered lifecycle authority.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::sim::anim_class::{AnimObject, AnimRuntime, AnimWorldCoord};
use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::passenger::{PassengerCargo, PassengerRole};
use crate::util::fixed_math::SimFixed;

use super::{
    LifecycleOutput, LifecycleTestEvent, PlacementEvidence, RevealFailure, RevealOutcome,
    RevealPosition, RevealRequest, Simulation,
};

fn insert_entity(sim: &mut Simulation, stable_id: u64, category: EntityCategory) {
    let owner = sim.interner.intern("Americans");
    let type_ref = sim.interner.intern("TEST");
    let mut entity = GameEntity::new(
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
            rate_elapsed: 0,
            loop_remaining: 0,
            first_ai_guard: false,
            constructor_reverse: false,
            inactive,
        },
        in_logic_vector: false,
        start_sound_active: false,
        stop_sound_id: None,
    };
    assert!(sim.substrate.anims.insert(anim).is_none());
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
fn lifecycle_authority_legacy_app_death_handoff_changes_only_logic_membership() {
    let mut sim = Simulation::new();
    insert_entity(&mut sim, 1, EntityCategory::Unit);
    let _ = sim.reveal(1);
    sim.substrate.entities.get_mut(1).unwrap().selected = true;
    let before = sim.substrate.entities.get(1).unwrap();
    let lifecycle = before.lifecycle;
    let position = before.position.clone();
    let health = before.health;
    let selected = before.selected;
    let dying = before.dying;
    let owned_count_released = before.owned_count_released;

    sim.legacy_unregister_logic_only_for_app_death(1);
    let after = sim.substrate.entities.get(1).unwrap();
    assert!(!after.in_logic_vector);
    assert_eq!(sim.live_object_order_snapshot(), Vec::<u64>::new());
    assert_eq!(after.lifecycle, lifecycle);
    assert_eq!(
        (
            after.position.rx,
            after.position.ry,
            after.position.z,
            after.position.sub_x,
            after.position.sub_y,
            after.position.screen_x,
            after.position.screen_y,
        ),
        (
            position.rx,
            position.ry,
            position.z,
            position.sub_x,
            position.sub_y,
            position.screen_x,
            position.screen_y,
        )
    );
    assert_eq!(
        (after.health.current, after.health.max),
        (health.current, health.max)
    );
    assert_eq!(after.selected, selected);
    assert_eq!(after.dying, dying);
    assert_eq!(after.owned_count_released, owned_count_released);
    assert!(sim.substrate.occupancy.contains_entity(2, 3, 1));
    assert!(sim.substrate.pending_delete.is_empty());
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
            entity.position.screen_x,
            entity.position.screen_y,
        ),
        (
            first_position.rx,
            first_position.ry,
            first_position.z,
            first_position.sub_x,
            first_position.sub_y,
            first_position.screen_x,
            first_position.screen_y,
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
    assert_eq!(cargo.total_size, 0);
    assert_eq!(cargo.garrison_fire_index, 0);
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
fn lifecycle_authority_deferred_animation_then_uninit_releases_owned_count_once() {
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
    sim.release_owned_count_once(1);
    sim.legacy_unregister_logic_only_for_app_death(1);
    assert_eq!(sim.houses.get(&owner).unwrap().owned_unit_count, 1);
    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(entity.owned_count_released);
    assert!(entity.lifecycle.object_alive);
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(!entity.in_logic_vector);

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

    sim.set_logic_order_for_test(vec![3, 1]);
    assert_eq!(sim.live_object_order_snapshot(), vec![3, 1]);
    assert!(sim.substrate.anims.get(3).unwrap().in_logic_vector);
    assert!(sim.substrate.entities.get(1).unwrap().in_logic_vector);
    assert!(!sim.substrate.entities.get(2).unwrap().in_logic_vector);
    sim.debug_assert_logic_membership_consistent();

    sim.set_logic_order_for_test(vec![2]);
    assert_eq!(sim.live_object_order_snapshot(), vec![2]);
    assert!(!sim.substrate.anims.get(3).unwrap().in_logic_vector);
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
