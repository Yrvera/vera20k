//! Producer tests for the Mission gate's readiness inputs.
//!
//! These assert the mapping direction, not native parity. The predicate they
//! feed is separately proven exhaustively in `locomotor_ready.rs`.

use super::*;
use crate::sim::components::{
    DriveCoord, DriveLocomotionRuntime, MovementTarget, ShipLocomotionRuntime,
};
use crate::sim::movement::teleport_movement::TeleportState;
use crate::util::fixed_math::{SIM_ONE, SimFixed};

fn entity_with(kind: LocomotorKind) -> GameEntity {
    let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
    entity.locomotor = Some(LocomotorState::for_test_kind(kind));
    entity
}

fn moving_target(speed: i32) -> MovementTarget {
    MovementTarget {
        path: vec![(5, 5), (6, 5)],
        next_index: 1,
        current_speed: SimFixed::from_num(speed),
        ..MovementTarget::default()
    }
}

/// A parked vehicle must report "not moving". This is the direction that
/// matters: a false "moving" makes the mission gate defer every tick, which
/// stalls the unit outright.
#[test]
fn parked_drive_unit_reports_not_moving() {
    let entity = entity_with(LocomotorKind::Drive);
    let state = ready_state_for(&entity, 100).expect("Drive has a producer");
    assert!(
        !state.is_moving_now(),
        "a parked tank must not report moving"
    );
}

/// A vehicle with a class-owned destination/head and positive owner-applied
/// speed reports moving without a path-execution adapter.
#[test]
fn driving_unit_reports_moving() {
    let mut entity = entity_with(LocomotorKind::Drive);
    let head = DriveCoord {
        x: 6 * 256 + 128,
        y: 5 * 256 + 128,
        z: 0,
    };
    entity.drive_locomotion = Some(DriveLocomotionRuntime {
        destination: Some(head),
        head_to: Some(head),
        current_speed_fraction: SIM_ONE,
        owner_current_speed: 25,
        ..DriveLocomotionRuntime::default()
    });

    let state = ready_state_for(&entity, 100).expect("Drive has a producer");
    assert!(state.is_moving_now(), "a driving tank must report moving");
}

/// Standing exactly on the stale head-to point reads not-moving. This is native
/// behaviour, not a workaround: the native slot compares head-to against the
/// owner's coordinate and ignores Z.
#[test]
fn unit_parked_on_its_head_to_reports_not_moving() {
    let mut entity = entity_with(LocomotorKind::Drive);
    entity.drive_locomotion = Some(DriveLocomotionRuntime {
        head_to: Some(DriveCoord {
            x: 5 * 256 + i32::from(entity.position.sub_x.to_num::<i32>() as i16),
            y: 5 * 256 + i32::from(entity.position.sub_y.to_num::<i32>() as i16),
            z: 0,
        }),
        ..DriveLocomotionRuntime::default()
    });

    let state = ready_state_for(&entity, 100).expect("Drive has a producer");
    assert!(!state.is_moving_now());
}

/// Ship uses the same four inputs through its own native slot, so it must
/// behave identically to Drive on identical state — but keep its own variant.
#[test]
fn ship_mirrors_drive_but_keeps_its_own_variant() {
    let mut entity = entity_with(LocomotorKind::Ship);
    let head = DriveCoord::cell(6, 5, 0);
    entity.ship_locomotion = Some(ShipLocomotionRuntime {
        destination: Some(head),
        head_to: Some(head),
        current_speed_fraction: SIM_ONE,
        owner_current_speed: 20,
        ..Default::default()
    });
    let state = ready_state_for(&entity, 100).expect("Ship has a producer");
    assert!(matches!(state, LocomotorReadyState::Ship { .. }));
    assert!(state.is_moving_now());
}

/// The trap this producer exists to avoid: a warped unit sitting out its chrono
/// delay is NOT moving. Treating the whole teleport state as "moving" would
/// defer its missions for the entire delay.
#[test]
fn teleport_chrono_delay_reports_not_moving() {
    let mut entity = entity_with(LocomotorKind::Teleport);
    entity.teleport_state = Some(TeleportState {
        phase: TeleportPhase::ChronoDelay,
        target_rx: 20,
        target_ry: 20,
        being_warped_ticks: 10,
    });

    let state = ready_state_for(&entity, 100).expect("Teleport has a producer");
    assert!(
        !state.is_moving_now(),
        "chrono delay must not defer the unit's missions"
    );
}

/// The relocation tick itself is the one moment the native flag is set.
#[test]
fn teleport_relocate_reports_moving() {
    let mut entity = entity_with(LocomotorKind::Teleport);
    entity.teleport_state = Some(TeleportState {
        phase: TeleportPhase::Relocate,
        target_rx: 20,
        target_ry: 20,
        being_warped_ticks: 16,
    });

    let state = ready_state_for(&entity, 100).expect("Teleport has a producer");
    assert!(state.is_moving_now());
}

/// Native separates hovering-in-place from hovering-while-translating; our
/// single `Hovering` phase is split by the presence of a movement target.
#[test]
fn jumpjet_hovering_split_by_movement_target() {
    let mut entity = entity_with(LocomotorKind::Jumpjet);
    if let Some(locomotor) = entity.locomotor.as_mut() {
        locomotor.air_phase = AirMovePhase::Hovering;
    }

    let holding = ready_state_for(&entity, 100).expect("Jumpjet has a producer");
    assert!(
        !holding.is_moving_now(),
        "holding station is native state 2 — not moving"
    );

    entity.movement_target = Some(moving_target(20));
    let translating = ready_state_for(&entity, 100).expect("Jumpjet has a producer");
    assert!(
        translating.is_moving_now(),
        "translating is native state 3 — moving"
    );
}

/// A grounded jumpjet is native state 0.
#[test]
fn jumpjet_landed_reports_not_moving() {
    let mut entity = entity_with(LocomotorKind::Jumpjet);
    if let Some(locomotor) = entity.locomotor.as_mut() {
        locomotor.air_phase = AirMovePhase::Landed;
    }
    let state = ready_state_for(&entity, 100).expect("Jumpjet has a producer");
    assert!(!state.is_moving_now());
}

/// A standing infantryman is not moving.
#[test]
fn idle_walker_reports_not_moving() {
    let entity = entity_with(LocomotorKind::Walk);
    let state = ready_state_for(&entity, 100).expect("Walk has a producer");
    assert!(!state.is_moving_now());
}

/// A walker stepping toward the next cell is moving.
#[test]
fn walking_infantry_reports_moving() {
    let mut entity = entity_with(LocomotorKind::Walk);
    entity.movement_target = Some(moving_target(10));
    let state = ready_state_for(&entity, 100).expect("Walk has a producer");
    assert!(state.is_moving_now());
}

/// The stall case this family's conservative floor exists for: a walker that is
/// blocked keeps its movement target while it waits for a repath. Reporting it
/// as moving would defer its mission for as long as it stays blocked.
#[test]
fn blocked_walker_reports_not_moving() {
    let mut entity = entity_with(LocomotorKind::Walk);
    entity.movement_target = Some(moving_target(10));
    if let Some(locomotor) = entity.locomotor.as_mut() {
        locomotor.phase = GroundMovePhase::Blocked;
    }
    let state = ready_state_for(&entity, 100).expect("Walk has a producer");
    assert!(
        !state.is_moving_now(),
        "a blocked walker must not defer its mission indefinitely"
    );
}

/// A hover unit with no movement work is not moving, and a stale speed request
/// cannot resurrect it.
#[test]
fn stopped_hover_reports_not_moving_despite_stale_request() {
    let mut entity = entity_with(LocomotorKind::Hover);
    if let Some(locomotor) = entity.locomotor.as_mut() {
        // Left over from the last leg — the producer must not trust it.
        locomotor.hover_speed_request = SIM_ONE;
    }
    let state = ready_state_for(&entity, 100).expect("Hover has a producer");
    assert!(!state.is_moving_now());
}

/// A hover unit under way with a non-zero throttle request is moving.
#[test]
fn hover_under_way_reports_moving() {
    let mut entity = entity_with(LocomotorKind::Hover);
    entity.movement_target = Some(moving_target(15));
    if let Some(locomotor) = entity.locomotor.as_mut() {
        locomotor.hover_speed_request = SIM_ONE;
    }
    let state = ready_state_for(&entity, 100).expect("Hover has a producer");
    assert!(state.is_moving_now());
}

/// A zero throttle request — the turn-stall case — reads not moving even with a
/// live movement target, because the native predicate's speed term is strict.
#[test]
fn hover_turn_stall_reports_not_moving() {
    let mut entity = entity_with(LocomotorKind::Hover);
    entity.movement_target = Some(moving_target(15));
    if let Some(locomotor) = entity.locomotor.as_mut() {
        locomotor.hover_speed_request = SIM_ZERO;
    }
    let state = ready_state_for(&entity, 100).expect("Hover has a producer");
    assert!(!state.is_moving_now());
}

/// The three reachable throttle requests map onto the native double's bits.
#[test]
fn hover_request_maps_to_native_double_bits() {
    assert_eq!(hover_request_bits(SIM_ZERO), 0);
    assert_eq!(hover_request_bits(SimFixed::lit("0.5")), F64_BITS_HALF);
    assert_eq!(hover_request_bits(SIM_ONE), F64_BITS_ONE);
}

/// Families with no readiness slot this gate consults must yield `None`, which
/// leaves the mission gate on its conservative answer rather than a guess.
#[test]
fn unmapped_families_yield_no_producer() {
    for kind in [LocomotorKind::Fly, LocomotorKind::Rocket] {
        let entity = entity_with(kind);
        assert!(
            ready_state_for(&entity, 100).is_none(),
            "{kind:?} has no faithful producer and must not be guessed"
        );
    }
}

/// An entity with no locomotor at all yields nothing.
#[test]
fn entity_without_locomotor_yields_no_producer() {
    let entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
    assert!(ready_state_for(&entity, 100).is_none());
}
