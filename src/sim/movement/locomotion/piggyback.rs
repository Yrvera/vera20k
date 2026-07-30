//! The one piggyback mechanism: a locomotor temporarily displacing another.
//!
//! ## The native protocol
//!
//! A locomotor that takes over stores the displaced one *inside itself*, in a
//! single stash slot, and the protocol around that slot is short and strict:
//!
//! - **BEGIN** takes the outgoing locomotor. It fails with a pointer error on a
//!   null argument, and — the load-bearing part — **fails outright if the stash
//!   slot is already occupied**. Only then does it store the pointer and take a
//!   reference. Nesting is refused by the engine, not merely avoided by callers.
//! - **END** hands the stashed pointer back out and clears the slot, returning a
//!   distinct "nothing was stashed" result when the slot is empty. Ownership
//!   moves to the caller: BEGIN takes a reference, END does not drop one.
//! - **The end gate** is dominated by movement. Drive's reads
//!   `!Is_Moving() && stash_present && <own flag> && !host_flag`, with the
//!   `Is_Moving` call first — a moving unit can never unwind its piggyback.
//!
//! Two things follow for the Rust model, and both are why this module replaces
//! two earlier ones rather than joining them:
//!
//! 1. **There is one slot, so there is one mechanism.** The previous
//!    `PiggybackLocomotor` stashed only a kind and a layer while
//!    `OverrideLocomotor` stashed a whole boxed clone, and the two had separate
//!    begin/end APIs for the same native slot. A unit could hold both at once,
//!    which the engine's E_FAIL makes impossible.
//! 2. **Non-nesting is enforced by the type here**, not by a runtime check that
//!    a caller might skip: [`StashedLocomotor`] holds the restorable state and
//!    has no stash field of its own, so a stash cannot contain a stash.
//!
//! ## What this module does NOT yet model
//!
//! Per-class end gates beyond the movement clause. Drive's gate reads its own
//! flag byte and one host flag; Jumpjet's host clause is an OR rather than a
//! single test, and Teleport's gate is a multi-valued phase index rather than a
//! boolean — a single shared boolean would make Teleport's gate constant-true
//! and let a piggyback unwind mid-warp. [`is_ok_to_end`] therefore models only
//! the movement clause and the stash-present clause, both of which are shared by
//! every class, and the per-class clauses are **UNCHECKED** and unimplemented.
//! The gate is conservative in the direction that matters: it never permits an
//! end that the movement clause would refuse.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ and sibling movement state only.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::locomotor_type::{LocomotorKind, SpeedType};
use crate::util::fixed_math::SimFixed;

use super::super::locomotor::{AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer};

/// The displaced locomotor, held by the one that took over.
///
/// Deliberately a flat record and not a boxed `LocomotorState`: it carries the
/// state that restoring must put back and **cannot itself hold a stash**, which
/// is what makes nesting unrepresentable rather than merely rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StashedLocomotor {
    pub kind: LocomotorKind,
    pub layer: MovementLayer,
    pub air_phase: AirMovePhase,
    pub speed_multiplier: SimFixed,
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
}

impl StashedLocomotor {
    /// Capture the currently-driving locomotor so it can be restored later.
    pub fn capture(state: &LocomotorState) -> Self {
        Self {
            kind: state.kind,
            layer: state.layer,
            air_phase: state.air_phase,
            speed_multiplier: state.speed_multiplier,
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
        }
    }
}

/// The outcome of a BEGIN, mirroring the native tri-state return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeginOutcome {
    /// The displaced locomotor was stashed and the new one is now driving.
    Installed,
    /// The stash slot was already occupied. Native returns `E_FAIL` here and
    /// changes nothing; so does this. Callers that must succeed have to END
    /// first — which is what the engine's own end-first BEGIN sites do.
    RefusedNested,
}

/// Stash the currently-driving locomotor and install `kind`/`layer` over it.
///
/// Refuses — changing nothing — when a stash is already present.
pub fn begin(
    state: &mut LocomotorState,
    kind: LocomotorKind,
    layer: MovementLayer,
) -> BeginOutcome {
    if state.piggyback.is_some() {
        return BeginOutcome::RefusedNested;
    }
    state.piggyback = Some(StashedLocomotor::capture(state));
    state.kind = kind;
    state.layer = layer;
    state.phase = GroundMovePhase::Idle;
    state.air_phase = AirMovePhase::Landed;
    BeginOutcome::Installed
}

/// Pop the stash and restore the locomotor it holds.
///
/// Returns the restored record, or `None` when nothing was stashed — the native
/// `S_FALSE` case. The slot is cleared before the restore is applied, so the
/// stash is observably empty at the moment the previous locomotor takes over
/// again, matching the native null window.
pub fn end(state: &mut LocomotorState) -> Option<StashedLocomotor> {
    let stashed = state.piggyback.take()?;
    state.kind = stashed.kind;
    state.layer = stashed.layer;
    state.phase = GroundMovePhase::Idle;
    state.air_phase = stashed.air_phase;
    state.speed_multiplier = stashed.speed_multiplier;
    state.altitude = stashed.altitude;
    state.target_altitude = stashed.target_altitude;
    state.climb_rate = stashed.climb_rate;
    state.jumpjet_speed = stashed.jumpjet_speed;
    state.jumpjet_accel = stashed.jumpjet_accel;
    state.jumpjet_current_speed = stashed.jumpjet_current_speed;
    state.jumpjet_deviation = stashed.jumpjet_deviation;
    state.jumpjet_crash_speed = stashed.jumpjet_crash_speed;
    state.jumpjet_turn_rate = stashed.jumpjet_turn_rate;
    state.balloon_hover = stashed.balloon_hover;
    state.hover_attack = stashed.hover_attack;
    state.speed_type = stashed.speed_type;
    Some(stashed)
}

/// Whether the active piggyback may be unwound now.
///
/// Models the two clauses every class shares: the unit must not be moving, and
/// something must actually be stashed. The per-class clauses are UNCHECKED and
/// absent — see the module docs.
pub fn is_ok_to_end(state: &LocomotorState, is_moving: bool) -> bool {
    !is_moving && state.piggyback.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::locomotor_type::LocomotorKind;

    fn teleporter() -> LocomotorState {
        LocomotorState::for_test_kind(LocomotorKind::Teleport)
    }

    /// Native BEGIN returns E_FAIL when the stash slot is occupied and changes
    /// nothing. This is the whole reason there is one mechanism and not two:
    /// with two independent stashes a unit could hold both at once, which the
    /// engine makes impossible.
    #[test]
    fn begin_refuses_to_nest() {
        let mut state = teleporter();
        assert_eq!(
            begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground),
            BeginOutcome::Installed
        );
        let before = state.clone();

        assert_eq!(
            begin(&mut state, LocomotorKind::Hover, MovementLayer::Ground),
            BeginOutcome::RefusedNested
        );
        assert_eq!(
            state.kind, before.kind,
            "a refused BEGIN must not install its locomotor"
        );
        assert_eq!(
            state.piggyback, before.piggyback,
            "a refused BEGIN must not disturb the existing stash"
        );
    }

    /// The end-first shape: two of the native BEGIN sites END before they BEGIN,
    /// precisely because BEGIN cannot nest. Ending first makes the second BEGIN
    /// succeed.
    #[test]
    fn ending_first_lets_a_second_begin_succeed() {
        let mut state = teleporter();
        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);

        assert!(end(&mut state).is_some());
        assert_eq!(
            begin(&mut state, LocomotorKind::Hover, MovementLayer::Ground),
            BeginOutcome::Installed
        );
        assert_eq!(state.kind, LocomotorKind::Hover);
    }

    #[test]
    fn end_restores_the_stashed_locomotor_and_clears_the_slot() {
        let mut state = teleporter();
        let original_kind = state.kind;
        let original_layer = state.layer;

        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);
        assert_eq!(state.kind, LocomotorKind::Drive);
        assert!(state.piggyback.is_some());

        let restored = end(&mut state).expect("a stash was present");
        assert_eq!(restored.kind, original_kind);
        assert_eq!(state.kind, original_kind);
        assert_eq!(state.layer, original_layer);
        assert!(
            state.piggyback.is_none(),
            "the slot must be empty after END — the native null window"
        );
    }

    /// Native END returns S_FALSE rather than an error when nothing is stashed.
    #[test]
    fn end_without_a_stash_reports_nothing_to_pop() {
        let mut state = teleporter();
        assert!(end(&mut state).is_none());
        assert!(state.piggyback.is_none());
    }

    /// The see-through identity: a Chrono Miner driving on a piggybacked Drive
    /// still *is* a Teleport unit. The installed slot answers that, while `kind`
    /// answers "what is driving right now" — mission logic asks the former and
    /// the destination path asks the latter, so both must stay distinguishable.
    #[test]
    fn effective_class_sees_through_stash() {
        let mut state = teleporter();
        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);

        assert_eq!(state.kind, LocomotorKind::Drive, "Drive is driving");
        assert_eq!(
            state.effective_kind(),
            LocomotorKind::Teleport,
            "but the unit is still a Teleport unit"
        );
    }

    /// The movement clause dominates: a moving unit can never unwind.
    #[test]
    fn is_ok_to_end_is_dominated_by_movement() {
        let mut state = teleporter();
        assert!(
            !is_ok_to_end(&state, false),
            "nothing stashed — nothing to end"
        );

        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);
        assert!(!is_ok_to_end(&state, true), "moving must refuse the end");
        assert!(is_ok_to_end(&state, false));
    }

    /// A stash cannot hold a stash — enforced by `StashedLocomotor` having no
    /// stash field at all, so this is a compile-time property the test merely
    /// records.
    #[test]
    fn a_stash_holds_no_stash_of_its_own() {
        let mut state = teleporter();
        begin(&mut state, LocomotorKind::Drive, MovementLayer::Ground);
        let stashed = state.piggyback.expect("stash present");
        // `stashed` is a StashedLocomotor: it exposes kind/layer/flight state and
        // no piggyback field. If one were ever added, this test stops compiling.
        assert_eq!(stashed.kind, LocomotorKind::Teleport);
    }
}
