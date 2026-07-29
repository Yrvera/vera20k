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
//! Two mappings carry a deliberate conservative floor, each labelled
//! VERA-internal at its definition: Drive/Ship's owner speed, and Walk's
//! blocked-phase exclusion. Both compensate for a state-lifetime mismatch
//! against native, and both err toward "not moving".
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
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

use super::locomotor::{AirMovePhase, GroundMovePhase, LocomotorState};
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
        LocomotorKind::Walk => Some(walk(entity, locomotor)),
        LocomotorKind::Hover => Some(hover(entity, locomotor)),
        // Fly, Rocket, Mech, DropPod, Parachute have no readiness slot of their
        // own that this gate consults. `None` keeps the conservative answer.
        _ => None,
    }
}

/// IEEE-754 binary64 bit patterns for the only two speed-fraction values the
/// Walk family's native field ever holds.
///
/// Fed to the predicate directly rather than converting a `SimFixed` to float
/// bits: a general fixed→IEEE754 conversion would introduce rounding decisions
/// that must then be reproduced bit-for-bit across every machine in a lockstep
/// match, for no gameplay benefit.
const F64_BITS_ONE: u64 = 0x3FF0_0000_0000_0000;
/// The hover throttle request's third reachable value.
const F64_BITS_HALF: u64 = 0x3FE0_0000_0000_0000;

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

/// Walk's readiness inputs.
///
/// Native predicate: `moving_byte != 0 && applied_speed > 0 && head_to_nonnull`.
///
/// Note the third input reads the locomotor's **head-to** coord — the cell
/// currently being stepped toward — not its final destination, despite the
/// field's name in the predicate. `path[next_index]` is the structural match.
///
/// The blocked-phase exclusion is **VERA-internal**, gamemd equivalent
/// UNCHECKED. It compensates for a lifetime mismatch: we keep a movement target
/// while a walker waits for a repath, whereas the native flag is governed by
/// its own head-to/stop calls. Without it a permanently blocked walker would
/// report moving forever and defer its mission forever — the stall direction,
/// on the largest unit family in the game.
fn walk(entity: &GameEntity, locomotor: &LocomotorState) -> LocomotorReadyState {
    let live_move = entity.movement_target.is_some() && locomotor.phase != GroundMovePhase::Blocked;

    LocomotorReadyState::Walk {
        moving_byte: u8::from(live_move),
        // Native's speed fraction for a walker is only ever 1.0 (move start) or
        // 0.0 (arrival / construction); the Walk locomotor never writes it.
        applied_speed_bits: if live_move { F64_BITS_ONE } else { 0 },
        destination_nonnull: entity
            .movement_target
            .as_ref()
            .is_some_and(|target| target.path.get(target.next_index).is_some()),
    }
}

/// Hover's readiness inputs.
///
/// Native predicate: `slot_moving && speed != 0`. The speed term is the strict
/// one, which makes the conjunction forgiving of an over-inclusive
/// `slot_moving` — so that side errs safely.
///
/// The speed input is the **unramped request**, not `hover_throttle`. The ramp
/// lags the request by up to roughly 27 ticks on the brake side, so reading it
/// would keep reporting "moving" well after a hover unit stopped — the stall
/// direction.
fn hover(entity: &GameEntity, locomotor: &LocomotorState) -> LocomotorReadyState {
    LocomotorReadyState::Hover {
        slot_moving: entity.movement_target.is_some() || entity.navigation.nav_com.is_some(),
        // Forced to zero without a live movement target: the request is only
        // written inside the hover movement branch, so a stopped unit would
        // otherwise keep its last non-zero value indefinitely.
        speed_bits: if entity.movement_target.is_some() {
            hover_request_bits(locomotor.hover_speed_request)
        } else {
            0
        },
    }
}

/// The hover throttle request has exactly three reachable values, so it maps to
/// the native double's bits by table — no float arithmetic in `sim/`.
fn hover_request_bits(request: SimFixed) -> u64 {
    if request == SIM_ZERO {
        0
    } else if request == SimFixed::lit("0.5") {
        F64_BITS_HALF
    } else {
        F64_BITS_ONE
    }
}

#[cfg(test)]
#[path = "ready_producer_tests.rs"]
mod ready_producer_tests;
