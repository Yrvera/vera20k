//! Behavior-preserving Mission writers for the legacy Rust state machines.
//!
//! These operations make the remaining reduced-state owners enumerable while
//! exact Mission authority stays dormant. They deliberately preserve the
//! existing Rust writes and must not delegate to native-exact Mission verbs.

use super::{MissionCom, MissionType};

/// Start a fresh legacy order, clearing the reduced interrupt stack and
/// re-anchoring the final dispatch-timer fields to preserve the old callsite's
/// behavior during the authority migration.
pub(crate) fn legacy_full_retask(state: &mut MissionCom, requested: MissionType, now: u32) {
    state.legacy_full_retask(requested, now);
}

/// Replace only the legacy current selector.
pub(crate) fn legacy_current_only_retask(state: &mut MissionCom, requested: MissionType) {
    state.legacy_current_only_retask(requested);
}

/// Project a Unit's legacy machines at its per-object host point.
pub(crate) fn legacy_unit_host_projection(
    state: &mut MissionCom,
    current: MissionType,
    substate: u8,
) {
    state.legacy_projection(current, substate);
}

/// Project legacy machines at the deterministic tick tail.
pub(crate) fn legacy_tick_tail_projection(
    state: &mut MissionCom,
    current: MissionType,
    substate: u8,
) {
    state.legacy_projection(current, substate);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::mission::state::MissionTestFixture;
    use crate::sim::mission::{MissionDispatchTimer, MissionId};

    fn populated_state() -> MissionCom {
        let mut state = MissionCom::at_frame(0);
        state.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Guard),
            suspended: MissionId::from_known(MissionType::Move),
            queued: MissionId::from_known(MissionType::Attack),
            movement_bypass_latch: 0xa5,
            handler_state: 7,
            mission_start_frame: 13,
            ai_counter: u32::MAX,
            dispatch_timer: MissionDispatchTimer::from_raw(5, 99),
        });
        state
    }

    #[test]
    fn legacy_full_retask_preserves_reduced_behavior() {
        let mut state = populated_state();

        legacy_full_retask(&mut state, MissionType::Harvest, 42);

        assert_eq!(state.current(), MissionId::from_known(MissionType::Harvest));
        assert_eq!(state.queued(), MissionId::NONE);
        assert_eq!(state.suspended(), MissionId::NONE);
        assert_eq!(state.handler_state(), 0);
        assert_eq!(state.dispatch_timer(), MissionDispatchTimer::at_frame(42));
        assert_eq!(state.movement_bypass_latch(), 0xa5);
        assert_eq!(state.mission_start_frame(), 13);
        assert_eq!(state.ai_counter(), u32::MAX);
    }

    #[test]
    fn legacy_current_only_retask_changes_only_current() {
        let mut state = populated_state();
        let mut expected = state;
        expected.legacy_current_only_retask(MissionType::AttackMove);

        legacy_current_only_retask(&mut state, MissionType::AttackMove);

        assert_eq!(state, expected);
    }

    #[test]
    fn legacy_unit_host_projection_writes_only_projection_fields() {
        let mut state = populated_state();
        let queued = state.queued();
        let suspended = state.suspended();
        let timer = state.dispatch_timer();

        legacy_unit_host_projection(&mut state, MissionType::Enter, 3);

        assert_eq!(state.current(), MissionId::from_known(MissionType::Enter));
        assert_eq!(state.handler_state(), 3);
        assert_eq!(state.ai_counter(), 0, "counter wraps like the prior write");
        assert_eq!(state.queued(), queued);
        assert_eq!(state.suspended(), suspended);
        assert_eq!(state.dispatch_timer(), timer);
    }

    #[test]
    fn legacy_tick_tail_projection_writes_only_projection_fields() {
        let mut state = populated_state();
        let queued = state.queued();
        let suspended = state.suspended();
        let timer = state.dispatch_timer();

        legacy_tick_tail_projection(&mut state, MissionType::Selling, 11);

        assert_eq!(state.current(), MissionId::from_known(MissionType::Selling));
        assert_eq!(state.handler_state(), 11);
        assert_eq!(state.ai_counter(), 0, "counter wraps like the prior write");
        assert_eq!(state.queued(), queued);
        assert_eq!(state.suspended(), suspended);
        assert_eq!(state.dispatch_timer(), timer);
    }
}
