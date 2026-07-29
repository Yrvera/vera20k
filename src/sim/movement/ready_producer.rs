//! Per-tick producers for `LocomotorState::mission_ready_state`.
//!
//! The readiness *predicate* lives in [`super::locomotor_ready`] and is
//! exhaustively tested against the native comparison. This module supplies its
//! *inputs* from live entity state, so the Mission readiness gate stops
//! substituting a constant "not moving".
//!
//! ## Why this is a separate module
//! `locomotor_ready` is destined for `sim::substrate::locomotion`, whose
//! dependency floor is rules/util only. Producing the inputs requires reading
//! `GameEntity`, so it stays here in `sim::movement`.
//!
//! ## Error direction is the safety property
//! Before this module existed the gate always answered "not moving", so the
//! moving-defer branch never fired. A producer that wrongly answers **moving**
//! makes missions defer and can stall a unit permanently; a producer that
//! wrongly answers **not moving** is no worse than the previous behaviour.
//! Every mapping below is therefore written to fail toward "not moving" when
//! its state is absent, and families without a faithful mapping return `None`
//! rather than a guess.
//!
//! ## Parity status
//! The predicate is VERIFIED (exhaustive over its input space). Every mapping
//! here is **UNCHECKED**: each was traced to its native field, but no
//! gamemd-derived executable check compares the two, so this is a
//! well-provenanced correspondence and not proof.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/ movement and entity state only.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;

use super::locomotor::{AirMovePhase, LocomotorState};
use super::locomotor_ready::LocomotorReadyState;
use super::teleport_movement::TeleportPhase;

/// Refresh every entity's readiness inputs from its current locomotor state.
///
/// Runs once at the end of the movement phase. The Mission gate reads the
/// result during the *next* tick's per-object dispatch, which runs before that
/// object moves again — so the value it sees is the state as of the last
/// completed movement, which is the same thing the native live virtual call
/// observes at that point in the tick.
pub(crate) fn refresh_mission_ready_states(entities: &mut EntityStore, binary_frame: u32) {
    for entity in entities.values_mut() {
        let state = ready_state_for(entity, binary_frame);
        if let Some(locomotor) = entity.locomotor.as_mut() {
            locomotor.mission_ready_state = state;
        }
    }
}

/// Readiness inputs for one entity, or `None` when this family has no faithful
/// producer yet (the gate then keeps its conservative "not moving" answer).
fn ready_state_for(entity: &GameEntity, binary_frame: u32) -> Option<LocomotorReadyState> {
    let locomotor = entity.locomotor.as_ref()?;
    match locomotor.active_kind() {
        LocomotorKind::Drive => Some(drive_family(entity, binary_frame, DriveFamily::Drive)),
        LocomotorKind::Ship => Some(drive_family(entity, binary_frame, DriveFamily::Ship)),
        LocomotorKind::Teleport => Some(teleport(entity)),
        LocomotorKind::Jumpjet => Some(jumpjet(entity, locomotor)),
        // Walk and Hover are deliberately absent — see the module-level note in
        // the S2 mapping section of the locomotion substrate design. Walk needs
        // its head-to field (the native slot reads head-to, not destination),
        // and Hover needs the speed request persisted rather than consumed
        // inline. Returning `None` keeps their previous, safe answer.
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum DriveFamily {
    Drive,
    Ship,
}

/// Drive and Ship read the same four inputs through separate native slots.
///
/// Native predicate: `turning_active || (slot_moving && head_to_nonnull && owner_speed > 0)`.
fn drive_family(
    entity: &GameEntity,
    binary_frame: u32,
    family: DriveFamily,
) -> LocomotorReadyState {
    let turning_active = entity
        .body_facing
        .as_ref()
        .is_some_and(|facing| facing.is_rotating(binary_frame));

    let drive = entity.drive_locomotion.as_ref();
    let head_to = drive.and_then(|drive| drive.head_to);
    let head_to_nonnull = head_to.is_some();

    // Native compares the head-to point against the owner's world coordinate in
    // leptons, and deliberately ignores Z.
    let current_x = i32::from(entity.position.rx) * 256 + entity.position.sub_x.to_num::<i32>();
    let current_y = i32::from(entity.position.ry) * 256 + entity.position.sub_y.to_num::<i32>();
    let slot_moving = drive.and_then(|drive| drive.destination).is_some()
        || head_to.is_some_and(|point| (point.x, point.y) != (current_x, current_y));

    // Native reads the owner's applied speed, which its locomotor drives to
    // exactly zero once the unit comes fully to rest. We have no separate
    // owner-side fraction, so this reads the movement target's ramped speed and
    // is zero whenever there is no movement target — the conservative direction.
    let owner_speed = entity
        .movement_target
        .as_ref()
        .map_or(0, |target| target.current_speed.to_num::<i32>());

    match family {
        DriveFamily::Drive => LocomotorReadyState::Drive {
            turning_active,
            slot_moving,
            head_to_nonnull,
            owner_speed,
        },
        DriveFamily::Ship => LocomotorReadyState::Ship {
            turning_active,
            slot_moving,
            head_to_nonnull,
            owner_speed,
        },
    }
}

/// Teleport's readiness input is a private one-shot flag, not the warp phase
/// counter.
///
/// It is true only for the relocation tick itself. It is NOT true during the
/// post-warp chrono delay — treating the whole teleport state as "moving" would
/// defer a warped unit's missions for the entire delay.
fn teleport(entity: &GameEntity) -> LocomotorReadyState {
    LocomotorReadyState::Teleport {
        state: u8::from(matches!(
            entity.teleport_state.as_ref().map(|state| state.phase),
            Some(TeleportPhase::Relocate)
        )),
    }
}

/// Jumpjet's readiness input is a flight-phase enum; the predicate treats 0
/// (grounded) and 2 (holding station) as not moving.
///
/// Native separates "hovering in place" from "hovering while translating"; our
/// `AirMovePhase` collapses both into `Hovering`, so the presence of a movement
/// target discriminates them.
fn jumpjet(entity: &GameEntity, locomotor: &LocomotorState) -> LocomotorReadyState {
    let state = match locomotor.air_phase {
        AirMovePhase::Landed => 0,
        AirMovePhase::Ascending => 1,
        AirMovePhase::Hovering => {
            if entity.movement_target.is_some() {
                3
            } else {
                2
            }
        }
        AirMovePhase::Cruising => 3,
        AirMovePhase::Descending => 4,
    };
    LocomotorReadyState::Jumpjet { state }
}

#[cfg(test)]
#[path = "ready_producer_tests.rs"]
mod ready_producer_tests;
