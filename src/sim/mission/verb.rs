//! Exact common Mission transitions over private [`MissionCom`] state.
//!
//! These functions model only the verified base Assign, Queue, Commence,
//! Override, and Restore field semantics. Category wrappers own readiness,
//! Aircraft policy, Target, and NavCom ordering. No base transition reads a
//! clock, consumes RNG, allocates, or exposes a production-facing verb.

use super::{MissionCom, MissionId};

const GUARD: MissionId = MissionId::from_raw(5);
const SELLING: MissionId = MissionId::from_raw(19);
const DELIBERATE: MissionId = MissionId::from_raw(28);

/// Whether Queue passed its whole-function guard.
///
/// `Continue` does not imply that Queue's mutation predicate wrote anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QueueContinuation {
    OuterGuardBlocked,
    Continue,
}

/// Apply the common Assign transition.
///
/// Native Assign has only the Deliberate-to-Guard guard; Selling is allowed.
pub(super) fn assign_base(state: &mut MissionCom, requested: MissionId, now: u32) {
    if state.current() == DELIBERATE && requested == GUARD {
        return;
    }
    state.assign_transition(requested, now);
}

/// Apply Queue's guard and conditional queue write.
///
/// Readiness and optional synchronous Commence belong to the category-aware
/// caller and run only after this function returns `Continue`.
pub(super) fn queue_base(state: &mut MissionCom, requested: MissionId) -> QueueContinuation {
    let current = state.current();
    if (current == DELIBERATE && requested == GUARD) || current == SELLING {
        return QueueContinuation::OuterGuardBlocked;
    }
    if requested != MissionId::NONE
        && !(current == requested
            && (state.queued() == requested || state.queued() == MissionId::NONE))
    {
        state.write_queue_and_clear_b8(requested);
    }
    QueueContinuation::Continue
}

/// Promote a queued selector and apply the exact Commence reset set.
pub(super) fn commence_base(state: &mut MissionCom, now: u32) -> bool {
    if state.queued() == MissionId::NONE {
        return false;
    }
    state.promote_queue(now);
    true
}

/// Apply the common Override transition.
///
/// The queued selector, when present, is copied to suspended and remains queued.
pub(super) fn override_base(state: &mut MissionCom, requested: MissionId) {
    let current = state.current();
    if (current == DELIBERATE && requested == GUARD) || current == SELLING {
        return;
    }
    state.override_transition(requested);
}

/// Restore a suspended selector without resetting handler or timing state.
pub(super) fn restore_base(state: &mut MissionCom) -> bool {
    if state.suspended() == MissionId::NONE {
        return false;
    }
    state.restore_transition();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::mission::MissionDispatchTimer;
    use crate::sim::mission::state::MissionTestFixture;

    const ATTACK: MissionId = MissionId::from_raw(1);
    const MOVE: MissionId = MissionId::from_raw(2);
    const HARVEST: MissionId = MissionId::from_raw(10);
    const UNKNOWN_CURRENT: MissionId = MissionId::from_raw(0x1111_1111);
    const UNKNOWN_SUSPENDED: MissionId = MissionId::from_raw(0x2222_2222);
    const UNKNOWN_QUEUED: MissionId = MissionId::from_raw(0x3333_3333);
    const UNKNOWN_REQUEST: MissionId = MissionId::from_raw(0x4444_4444);
    const MOVEMENT_BYPASS_SENTINEL: u8 = 0xa5;
    const HANDLER_STATE_SENTINEL: u32 = 0x1122_3344;
    const MISSION_START_SENTINEL: u32 = 0x5566_7788;
    const AI_COUNTER_SENTINEL: u32 = 0x99aa_bbcc;
    const DISPATCH_START_SENTINEL: i32 = -17;
    const DISPATCH_DELAY_SENTINEL: i32 = -29;

    fn sentinel_fixture(current: MissionId, queued: MissionId) -> MissionTestFixture {
        MissionTestFixture {
            current,
            suspended: UNKNOWN_SUSPENDED,
            queued,
            movement_bypass_latch: MOVEMENT_BYPASS_SENTINEL,
            handler_state: HANDLER_STATE_SENTINEL,
            mission_start_frame: MISSION_START_SENTINEL,
            ai_counter: AI_COUNTER_SENTINEL,
            dispatch_timer: MissionDispatchTimer::from_raw(
                DISPATCH_START_SENTINEL,
                DISPATCH_DELAY_SENTINEL,
            ),
        }
    }

    fn state_from(fixture: MissionTestFixture) -> MissionCom {
        let mut state = MissionCom::at_frame(0);
        state.apply_test_fixture(fixture);
        state
    }

    fn assert_all_fields(context: &str, actual: &MissionCom, expected: &MissionCom) {
        assert_eq!(actual.current(), expected.current(), "{context}: current");
        assert_eq!(
            actual.suspended(),
            expected.suspended(),
            "{context}: suspended"
        );
        assert_eq!(actual.queued(), expected.queued(), "{context}: queued");
        assert_eq!(
            actual.movement_bypass_latch(),
            expected.movement_bypass_latch(),
            "{context}: movement bypass latch"
        );
        assert_eq!(
            actual.handler_state(),
            expected.handler_state(),
            "{context}: handler state"
        );
        assert_eq!(
            actual.mission_start_frame(),
            expected.mission_start_frame(),
            "{context}: mission start frame"
        );
        assert_eq!(
            actual.ai_counter(),
            expected.ai_counter(),
            "{context}: AI counter"
        );
        assert_eq!(
            actual.dispatch_timer().start_frame(),
            expected.dispatch_timer().start_frame(),
            "{context}: dispatch start frame"
        );
        assert_eq!(
            actual.dispatch_timer().delay(),
            expected.dispatch_timer().delay(),
            "{context}: dispatch delay"
        );
    }

    #[test]
    fn mission_assign_base_deliberate_to_guard_is_fieldwise_noop() {
        let fixture = sentinel_fixture(DELIBERATE, UNKNOWN_QUEUED);
        let mut state = state_from(fixture);
        let before = state;

        assign_base(&mut state, GUARD, 0xdead_beef);

        assert_all_fields("guarded Assign", &state, &before);
    }

    #[test]
    fn mission_assign_base_resets_exact_fields_and_preserves_suspended() {
        let now = 0xdead_beef;
        let cases = [
            ("Deliberate to non-Guard", DELIBERATE, ATTACK),
            ("Selling remains assignable", SELLING, MOVE),
            ("ordinary same mission", GUARD, GUARD),
            ("unknown requested dword", MOVE, UNKNOWN_REQUEST),
            (
                "high bits prevent Deliberate guard",
                MissionId::from_raw(0x1000_001c),
                GUARD,
            ),
            (
                "high requested bits prevent Guard target",
                DELIBERATE,
                MissionId::from_raw(0x1000_0005),
            ),
            (
                "None remains a raw assignable selector",
                DELIBERATE,
                MissionId::NONE,
            ),
        ];

        for (name, current, requested) in cases {
            let fixture = sentinel_fixture(current, UNKNOWN_QUEUED);
            let mut state = state_from(fixture);
            let mut expected_fixture = fixture;
            expected_fixture.current = requested;
            expected_fixture.queued = MissionId::NONE;
            expected_fixture.movement_bypass_latch = 0;
            expected_fixture.handler_state = 0;
            expected_fixture.mission_start_frame = now;
            expected_fixture.ai_counter = 0;
            expected_fixture.dispatch_timer = MissionDispatchTimer::at_frame(now);
            let expected = state_from(expected_fixture);

            assign_base(&mut state, requested, now);

            assert_all_fields(name, &state, &expected);
        }
    }

    #[test]
    fn mission_queue_base_outer_guards_are_fieldwise_noops() {
        let cases = [
            ("Deliberate to Guard", DELIBERATE, GUARD),
            ("Selling to Attack", SELLING, ATTACK),
            ("Selling to None", SELLING, MissionId::NONE),
        ];

        for (name, current, requested) in cases {
            let fixture = sentinel_fixture(current, UNKNOWN_QUEUED);
            let mut state = state_from(fixture);
            let before = state;

            let continuation = queue_base(&mut state, requested);

            assert_eq!(
                continuation,
                QueueContinuation::OuterGuardBlocked,
                "{name}: continuation"
            );
            assert_all_fields(name, &state, &before);
        }
    }

    #[test]
    fn mission_queue_base_complete_write_predicate_matrix() {
        let cases = [
            (
                "requested None with unequal current and queue",
                ATTACK,
                MOVE,
                MissionId::NONE,
                false,
            ),
            (
                "requested None with equal current and empty queue",
                MissionId::NONE,
                MissionId::NONE,
                MissionId::NONE,
                false,
            ),
            (
                "requested None with equal current and different queue",
                MissionId::NONE,
                MOVE,
                MissionId::NONE,
                false,
            ),
            (
                "requested None with unequal current and equal queue",
                ATTACK,
                MissionId::NONE,
                MissionId::NONE,
                false,
            ),
            (
                "equal current and empty queue",
                ATTACK,
                MissionId::NONE,
                ATTACK,
                false,
            ),
            (
                "equal current and equal queue",
                ATTACK,
                ATTACK,
                ATTACK,
                false,
            ),
            (
                "equal current and different queue",
                ATTACK,
                MOVE,
                ATTACK,
                true,
            ),
            (
                "unequal current and empty queue",
                ATTACK,
                MissionId::NONE,
                MOVE,
                true,
            ),
            (
                "unequal current and equal queue still clears B8",
                ATTACK,
                MOVE,
                MOVE,
                true,
            ),
            (
                "unequal current and different queue",
                ATTACK,
                HARVEST,
                MOVE,
                true,
            ),
            (
                "unknown dwords compare without normalization",
                UNKNOWN_CURRENT,
                UNKNOWN_QUEUED,
                UNKNOWN_REQUEST,
                true,
            ),
        ];

        for (name, current, queued, requested, writes) in cases {
            let fixture = sentinel_fixture(current, queued);
            let mut state = state_from(fixture);
            let mut expected_fixture = fixture;
            if writes {
                expected_fixture.queued = requested;
                expected_fixture.movement_bypass_latch = 0;
            }
            let expected = state_from(expected_fixture);

            let continuation = queue_base(&mut state, requested);

            assert_eq!(
                continuation,
                QueueContinuation::Continue,
                "{name}: continuation"
            );
            assert_all_fields(name, &state, &expected);
        }
    }

    #[test]
    fn mission_queue_base_uses_full_dword_guard_comparisons() {
        let cases = [
            (
                "high current bits avoid Deliberate guard",
                MissionId::from_raw(0x1000_001c),
                GUARD,
            ),
            (
                "high requested bits avoid Guard target",
                DELIBERATE,
                MissionId::from_raw(0x1000_0005),
            ),
            (
                "high current bits avoid Selling guard",
                MissionId::from_raw(0x1000_0013),
                ATTACK,
            ),
        ];

        for (name, current, requested) in cases {
            let fixture = sentinel_fixture(current, UNKNOWN_QUEUED);
            let mut state = state_from(fixture);
            let mut expected_fixture = fixture;
            expected_fixture.queued = requested;
            expected_fixture.movement_bypass_latch = 0;
            let expected = state_from(expected_fixture);

            let continuation = queue_base(&mut state, requested);

            assert_eq!(continuation, QueueContinuation::Continue, "{name}");
            assert_all_fields(name, &state, &expected);
        }
    }

    #[test]
    fn mission_commence_base_empty_queue_is_fieldwise_noop() {
        let fixture = sentinel_fixture(UNKNOWN_CURRENT, MissionId::NONE);
        let mut state = state_from(fixture);
        let before = state;

        assert!(!commence_base(&mut state, 0xdead_beef));

        assert_all_fields("empty Commence", &state, &before);
    }

    #[test]
    fn mission_commence_base_promotes_raw_queue_and_resets_exact_fields() {
        let now = 0xdead_beef;
        let fixture = sentinel_fixture(UNKNOWN_CURRENT, UNKNOWN_QUEUED);
        let mut state = state_from(fixture);
        let mut expected_fixture = fixture;
        expected_fixture.current = UNKNOWN_QUEUED;
        expected_fixture.queued = MissionId::NONE;
        expected_fixture.movement_bypass_latch = 0;
        expected_fixture.handler_state = 0;
        expected_fixture.mission_start_frame = now;
        expected_fixture.ai_counter = 0;
        expected_fixture.dispatch_timer = MissionDispatchTimer::at_frame(now);
        let expected = state_from(expected_fixture);

        assert!(commence_base(&mut state, now));

        assert_all_fields("successful Commence", &state, &expected);
    }

    #[test]
    fn mission_override_base_outer_guards_are_fieldwise_noops() {
        let cases = [
            ("Deliberate to Guard", DELIBERATE, GUARD),
            ("Selling to Attack", SELLING, ATTACK),
            ("Selling to None", SELLING, MissionId::NONE),
        ];

        for (name, current, requested) in cases {
            let fixture = sentinel_fixture(current, UNKNOWN_QUEUED);
            let mut state = state_from(fixture);
            let before = state;

            override_base(&mut state, requested);

            assert_all_fields(name, &state, &before);
        }
    }

    #[test]
    fn mission_override_base_queue_present_suspends_queue_without_clearing_it() {
        let fixture = sentinel_fixture(UNKNOWN_CURRENT, UNKNOWN_QUEUED);
        let mut state = state_from(fixture);
        let mut expected_fixture = fixture;
        expected_fixture.current = UNKNOWN_REQUEST;
        expected_fixture.suspended = UNKNOWN_QUEUED;
        expected_fixture.movement_bypass_latch = 0;
        let expected = state_from(expected_fixture);

        override_base(&mut state, UNKNOWN_REQUEST);

        assert_all_fields("queued Override", &state, &expected);
    }

    #[test]
    fn mission_override_base_queue_absent_suspends_current_and_accepts_none() {
        let fixture = sentinel_fixture(UNKNOWN_CURRENT, MissionId::NONE);
        let mut state = state_from(fixture);
        let mut expected_fixture = fixture;
        expected_fixture.current = MissionId::NONE;
        expected_fixture.suspended = UNKNOWN_CURRENT;
        expected_fixture.movement_bypass_latch = 0;
        let expected = state_from(expected_fixture);

        override_base(&mut state, MissionId::NONE);

        assert_all_fields("queue-absent Override", &state, &expected);
    }

    #[test]
    fn mission_override_base_uses_full_dword_guard_comparisons() {
        let cases = [
            (
                "high current bits avoid Deliberate guard",
                MissionId::from_raw(0x1000_001c),
                GUARD,
            ),
            (
                "high requested bits avoid Guard target",
                DELIBERATE,
                MissionId::from_raw(0x1000_0005),
            ),
            (
                "high current bits avoid Selling guard",
                MissionId::from_raw(0x1000_0013),
                ATTACK,
            ),
        ];

        for (name, current, requested) in cases {
            let fixture = sentinel_fixture(current, MissionId::NONE);
            let mut state = state_from(fixture);
            let mut expected_fixture = fixture;
            expected_fixture.current = requested;
            expected_fixture.suspended = current;
            expected_fixture.movement_bypass_latch = 0;
            let expected = state_from(expected_fixture);

            override_base(&mut state, requested);

            assert_all_fields(name, &state, &expected);
        }
    }

    #[test]
    fn mission_restore_base_empty_suspended_is_fieldwise_noop() {
        let mut fixture = sentinel_fixture(UNKNOWN_CURRENT, UNKNOWN_QUEUED);
        fixture.suspended = MissionId::NONE;
        let mut state = state_from(fixture);
        let before = state;

        assert!(!restore_base(&mut state));

        assert_all_fields("empty Restore", &state, &before);
    }

    #[test]
    fn mission_restore_base_promotes_raw_suspended_and_preserves_other_fields() {
        let fixture = sentinel_fixture(UNKNOWN_CURRENT, UNKNOWN_QUEUED);
        let mut state = state_from(fixture);
        let mut expected_fixture = fixture;
        expected_fixture.current = UNKNOWN_SUSPENDED;
        expected_fixture.suspended = MissionId::NONE;
        expected_fixture.movement_bypass_latch = 0;
        let expected = state_from(expected_fixture);

        assert!(restore_base(&mut state));

        assert_all_fields("successful Restore", &state, &expected);
    }
}
