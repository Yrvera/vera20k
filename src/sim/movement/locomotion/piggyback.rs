//! The one piggyback mechanism: a locomotor temporarily displacing another.
//!
//! `WalkLocomotionClass::IPiggyback::{BeginPiggyback,EndPiggyback}` owns one
//! suspended `ILocomotion` reference. BEGIN takes that complete object, END
//! transfers the same object back, and Walk save/load serializes it as a nested
//! COM object. The Rust seam mirrors ownership, not COM refcounts.

use std::ops::Deref;

use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::util::fixed_math::SimFixed;

use super::super::locomotor::{AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer};

/// Runtime state shared by every locomotor object, independent of its installed
/// class identity. This is what moves as one object through the piggyback slot.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocomotorCommonRuntime {
    pub powered: bool,
    pub phase: GroundMovePhase,
    pub air_phase: AirMovePhase,
    pub speed_multiplier: SimFixed,
    pub speed_fraction: SimFixed,
    pub fly_current_speed: SimFixed,
    pub altitude: SimFixed,
    pub target_altitude: SimFixed,
    pub climb_rate: SimFixed,
    pub jumpjet_speed: SimFixed,
    pub jumpjet_accel: SimFixed,
    pub jumpjet_current_speed: SimFixed,
    pub jumpjet_deviation: i32,
    pub jumpjet_crash_speed: SimFixed,
    pub jumpjet_turn_rate: i32,
    pub balloon_hover: bool,
    pub hover_attack: bool,
    pub speed_type: SpeedType,
    pub movement_zone: MovementZone,
    pub rot: i32,
    pub air_progress: SimFixed,
    pub infantry_wobble_phase: f32,
    pub subcell_dest: Option<(SimFixed, SimFixed)>,
    pub hover_throttle: SimFixed,
    pub hover_speed_request: SimFixed,
    pub hover_bob_offset: SimFixed,
}

/// Class-local state that travels with the locomotor object.
///
/// The current movement implementation has not yet moved the special process
/// fields from their entity adapters, so their payloads are deliberately typed
/// placeholders. They prevent another flat stash from silently losing a special
/// locomotor's identity when those fields move here in the next wave.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LocomotorRuntimePayload {
    Drive,
    Walk,
    Teleport { phase: u8 },
    Tunnel { state: u8 },
    Rocket { state: u8 },
    DropPod { descent_ticks: u32 },
    Hover,
    Mech,
    Ship,
    Fly,
    Jumpjet,
    Parachute,
}

impl LocomotorRuntimePayload {
    fn for_kind(kind: LocomotorKind) -> Self {
        match kind {
            LocomotorKind::Drive => Self::Drive,
            LocomotorKind::Walk => Self::Walk,
            LocomotorKind::Teleport => Self::Teleport { phase: 0 },
            LocomotorKind::Tunnel => Self::Tunnel { state: 0 },
            LocomotorKind::Rocket => Self::Rocket { state: 0 },
            LocomotorKind::DropPod => Self::DropPod { descent_ticks: 0 },
            LocomotorKind::Hover => Self::Hover,
            LocomotorKind::Mech => Self::Mech,
            LocomotorKind::Ship => Self::Ship,
            LocomotorKind::Fly => Self::Fly,
            LocomotorKind::Jumpjet => Self::Jumpjet,
            LocomotorKind::Parachute => Self::Parachute,
        }
    }
}

/// One complete movable locomotor instance. Installed identity remains on the
/// host `LocomotorState`; this object is the active or suspended implementation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocomotorRuntime {
    pub kind: LocomotorKind,
    pub layer: MovementLayer,
    pub common: LocomotorCommonRuntime,
    pub payload: LocomotorRuntimePayload,
}

impl LocomotorRuntime {
    /// Capture all active runtime-owned state without copying the host's
    /// installed slot or piggyback pointer.
    pub fn capture(state: &LocomotorState) -> Self {
        Self {
            kind: state.kind,
            layer: state.layer,
            common: LocomotorCommonRuntime {
                powered: state.powered,
                phase: state.phase,
                air_phase: state.air_phase,
                speed_multiplier: state.speed_multiplier,
                speed_fraction: state.speed_fraction,
                fly_current_speed: state.fly_current_speed,
                altitude: state.altitude,
                target_altitude: state.target_altitude,
                climb_rate: state.climb_rate,
                jumpjet_speed: state.jumpjet_speed,
                jumpjet_accel: state.jumpjet_accel,
                jumpjet_current_speed: state.jumpjet_current_speed,
                jumpjet_deviation: state.jumpjet_deviation,
                jumpjet_crash_speed: state.jumpjet_crash_speed,
                jumpjet_turn_rate: state.jumpjet_turn_rate,
                balloon_hover: state.balloon_hover,
                hover_attack: state.hover_attack,
                speed_type: state.speed_type,
                movement_zone: state.movement_zone,
                rot: state.rot,
                air_progress: state.air_progress,
                infantry_wobble_phase: state.infantry_wobble_phase,
                subcell_dest: state.subcell_dest,
                hover_throttle: state.hover_throttle,
                hover_speed_request: state.hover_speed_request,
                hover_bob_offset: state.hover_bob_offset,
            },
            payload: LocomotorRuntimePayload::for_kind(state.kind),
        }
    }

    /// Build an incoming runtime from the current host defaults. New callers
    /// should prefer `begin_with_runtime` when they already own an instance.
    pub fn replacement_from(state: &LocomotorState, kind: LocomotorKind, layer: MovementLayer) -> Self {
        let mut runtime = Self::capture(state);
        runtime.kind = kind;
        runtime.layer = layer;
        runtime.common.phase = GroundMovePhase::Idle;
        runtime.common.air_phase = AirMovePhase::Landed;
        runtime.payload = LocomotorRuntimePayload::for_kind(kind);
        runtime
    }

    fn install_into(self, state: &mut LocomotorState) {
        state.kind = self.kind;
        state.layer = self.layer;
        state.powered = self.common.powered;
        state.phase = self.common.phase;
        state.air_phase = self.common.air_phase;
        state.speed_multiplier = self.common.speed_multiplier;
        state.speed_fraction = self.common.speed_fraction;
        state.fly_current_speed = self.common.fly_current_speed;
        state.altitude = self.common.altitude;
        state.target_altitude = self.common.target_altitude;
        state.climb_rate = self.common.climb_rate;
        state.jumpjet_speed = self.common.jumpjet_speed;
        state.jumpjet_accel = self.common.jumpjet_accel;
        state.jumpjet_current_speed = self.common.jumpjet_current_speed;
        state.jumpjet_deviation = self.common.jumpjet_deviation;
        state.jumpjet_crash_speed = self.common.jumpjet_crash_speed;
        state.jumpjet_turn_rate = self.common.jumpjet_turn_rate;
        state.balloon_hover = self.common.balloon_hover;
        state.hover_attack = self.common.hover_attack;
        state.speed_type = self.common.speed_type;
        state.movement_zone = self.common.movement_zone;
        state.rot = self.common.rot;
        state.air_progress = self.common.air_progress;
        state.infantry_wobble_phase = self.common.infantry_wobble_phase;
        state.subcell_dest = self.common.subcell_dest;
        state.hover_throttle = self.common.hover_throttle;
        state.hover_speed_request = self.common.hover_speed_request;
        state.hover_bob_offset = self.common.hover_bob_offset;
    }
}

/// A boxed suspended locomotor instance. The box corresponds to the single
/// nested COM object persisted by WalkLocomotionClass::Save.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StashedLocomotor(Box<LocomotorRuntime>);

impl StashedLocomotor {
    pub fn capture(state: &LocomotorState) -> Self {
        Self(Box::new(LocomotorRuntime::capture(state)))
    }

    pub fn from_runtime(runtime: LocomotorRuntime) -> Self {
        Self(Box::new(runtime))
    }

    pub fn into_runtime(self) -> LocomotorRuntime {
        *self.0
    }
}

impl Deref for StashedLocomotor {
    type Target = LocomotorRuntime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The outcome of a BEGIN, mirroring the native tri-state return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeginOutcome {
    Installed,
    RefusedNull,
    RefusedNested,
}

/// The ownership result of END. `Empty` is native `S_FALSE`; `RefusedNull`
/// represents the native null output pointer, which Rust callers avoid by using
/// `end` or supplying a real destination to `end_into`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndOutcome {
    Restored,
    Empty,
    RefusedNull,
}

/// Stash the active complete runtime and install `incoming` as the new active
/// locomotor. An occupied slot is an atomic E_FAIL-style refusal.
pub fn begin_with_runtime(state: &mut LocomotorState, incoming: Option<LocomotorRuntime>) -> BeginOutcome {
    let Some(incoming) = incoming else {
        return BeginOutcome::RefusedNull;
    };
    if state.piggyback.is_some() {
        return BeginOutcome::RefusedNested;
    }

    state.piggyback = Some(StashedLocomotor::capture(state));
    incoming.install_into(state);
    BeginOutcome::Installed
}

/// Compatibility adapter for callers that name only a replacement class. It
/// still transfers one complete current runtime into the suspended slot.
pub fn begin(state: &mut LocomotorState, kind: LocomotorKind, layer: MovementLayer) -> BeginOutcome {
    let incoming = LocomotorRuntime::replacement_from(state, kind, layer);
    begin_with_runtime(state, Some(incoming))
}

/// Transfer the suspended runtime into an explicit output location. The active
/// state is unchanged because native END transfers an interface; the caller
/// decides when to install it.
pub fn end_into(state: &mut LocomotorState, output: Option<&mut Option<LocomotorRuntime>>) -> EndOutcome {
    let Some(output) = output else {
        return EndOutcome::RefusedNull;
    };
    let Some(stashed) = state.piggyback.take() else {
        return EndOutcome::Empty;
    };
    *output = Some(stashed.into_runtime());
    EndOutcome::Restored
}

/// Transfer the suspended runtime back to the host and make it active.
pub fn end(state: &mut LocomotorState) -> Option<StashedLocomotor> {
    let stashed = state.piggyback.take()?;
    let restored = stashed.clone();
    stashed.into_runtime().install_into(state);
    Some(restored)
}

/// The nested-runtime save marker used by the clean-room snapshot seam.
/// Serde persists the following boxed runtime only when this returns one.
pub fn serialized_presence(state: &LocomotorState) -> u8 {
    u8::from(state.piggyback.is_some())
}

/// Inputs to the class-local `IsOKToEnd` gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndGateContext {
    pub owner_moving: bool,
    pub owner_teleporting: bool,
    pub owner_deploying: bool,
}

/// Whether the active piggyback may be unwound now.
///
/// The movement and populated-slot checks are common. Walk's named-location
/// gate additionally requires its local state and linked owner bytes clear;
/// mapped here to an idle active phase and clear owner transition flags. Special
/// locomotors stay conservative until their Process state machines supply their
/// own completed phase to the caller.
pub fn is_ok_to_end(state: &LocomotorState, context: EndGateContext) -> bool {
    if context.owner_moving || state.piggyback.is_none() {
        return false;
    }

    match state.kind {
        LocomotorKind::Drive
        | LocomotorKind::Walk
        | LocomotorKind::Hover
        | LocomotorKind::Mech
        | LocomotorKind::Ship => {
            state.phase == GroundMovePhase::Idle
                && !context.owner_teleporting
                && !context.owner_deploying
        }
        LocomotorKind::Fly | LocomotorKind::Jumpjet | LocomotorKind::Parachute => {
            state.air_phase == AirMovePhase::Landed
                && !context.owner_teleporting
                && !context.owner_deploying
        }
        LocomotorKind::Teleport
        | LocomotorKind::Tunnel
        | LocomotorKind::Rocket
        | LocomotorKind::DropPod => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn teleporter() -> LocomotorState {
        LocomotorState::for_test_kind(LocomotorKind::Teleport)
    }

    #[test]
    fn begin_rejects_null_and_nested_runtime_without_mutation() {
        let mut state = teleporter();
        assert_eq!(begin_with_runtime(&mut state, None), BeginOutcome::RefusedNull);
        let before = state.clone();

        assert_eq!(begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground), BeginOutcome::Installed);
        let nested_before = state.clone();
        assert_eq!(
            begin(&mut state, LocomotorKind::Ship, MovementLayer::Ground),
            BeginOutcome::RefusedNested
        );
        assert_eq!(state.piggyback, nested_before.piggyback);
        assert_eq!(before.kind, LocomotorKind::Teleport);
    }

    #[test]
    fn begin_and_end_transfer_complete_runtime_without_touching_installed_slot() {
        let mut state = teleporter();
        state.altitude = SimFixed::from_num(123);
        state.hover_speed_request = SimFixed::from_num(1);
        let installed = state.slot;

        assert_eq!(begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground), BeginOutcome::Installed);
        assert_eq!(state.kind, LocomotorKind::Drive);
        assert_eq!(state.slot, installed);
        assert_eq!(
            state.piggyback.as_deref().expect("stash").common.altitude,
            SimFixed::from_num(123)
        );

        let restored = end(&mut state).expect("suspended runtime");
        assert_eq!(restored.kind, LocomotorKind::Teleport);
        assert_eq!(state.kind, LocomotorKind::Teleport);
        assert_eq!(state.altitude, SimFixed::from_num(123));
        assert_eq!(state.slot, installed);
        assert!(state.piggyback.is_none());
    }

    #[test]
    fn end_into_reports_null_empty_and_transfers_without_installing() {
        let mut state = teleporter();
        assert_eq!(end_into(&mut state, None), EndOutcome::RefusedNull);
        let mut output = None;
        assert_eq!(end_into(&mut state, Some(&mut output)), EndOutcome::Empty);

        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);
        assert_eq!(end_into(&mut state, Some(&mut output)), EndOutcome::Restored);
        assert_eq!(state.kind, LocomotorKind::Drive);
        assert_eq!(output.expect("transferred runtime").kind, LocomotorKind::Teleport);
        assert!(state.piggyback.is_none());
    }

    #[test]
    fn ordinary_and_special_end_gates_are_distinct() {
        let mut state = teleporter();
        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);
        let ready = EndGateContext {
            owner_moving: false,
            owner_teleporting: false,
            owner_deploying: false,
        };
        assert!(is_ok_to_end(&state, ready));

        state.kind = LocomotorKind::Teleport;
        assert!(!is_ok_to_end(&state, ready));
        state.kind = LocomotorKind::Drive;
        state.phase = GroundMovePhase::Cruising;
        assert!(!is_ok_to_end(&state, ready));
    }

    #[test]
    fn serialized_presence_matches_the_single_suspended_runtime() {
        let mut state = teleporter();
        assert_eq!(serialized_presence(&state), 0);
        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);
        assert_eq!(serialized_presence(&state), 1);
    }
}
