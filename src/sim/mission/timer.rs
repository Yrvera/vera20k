//! Frame-anchored timer primitives used by Mission-related systems.
//!
//! `MissionTimer` retains the existing unsigned timing model shared by docking,
//! gates, and miners. `MissionDispatchTimer` separately models native Mission
//! dispatch's signed dwords and wrapping comparisons. Both snapshot the global
//! frame counter rather than decrementing per tick. sim/ only.
use serde::{Deserialize, Serialize};

/// "Unarmed / always due" (the -1 start). `u32::MAX`: the live counter starts at
/// 0 and would take ~3.3 years at 15fps to reach it, so it is never a live value.
pub const SENTINEL: u32 = u32::MAX;

const DISPATCH_ALWAYS_DUE_START: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionTimer {
    pub start_frame: u32,
    pub duration: u32,
}

/// Native Mission-dispatch timer state.
///
/// Both fields are signed dwords. The simulation's unsigned frame counter is
/// reinterpreted as the same 32 raw bits before signed wrapping arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionDispatchTimer {
    start_frame: i32,
    delay: i32,
}

impl MissionDispatchTimer {
    /// Construct a timer anchored at the supplied simulation frame and due
    /// immediately because its delay is zero.
    #[inline]
    pub const fn at_frame(frame: u32) -> Self {
        Self {
            start_frame: frame as i32,
            delay: 0,
        }
    }

    /// Preserve raw native timer dwords without normalization.
    #[inline]
    pub const fn from_raw(start_frame: i32, delay: i32) -> Self {
        Self { start_frame, delay }
    }

    /// Return the raw signed start-frame dword.
    #[inline]
    pub const fn start_frame(self) -> i32 {
        self.start_frame
    }

    /// Return the raw signed delay dword.
    #[inline]
    pub const fn delay(self) -> i32 {
        self.delay
    }

    /// Test native Mission-dispatch readiness.
    ///
    /// A `-1` start is always due. Otherwise elapsed time is a signed wrapping
    /// subtraction and the due boundary is inclusive.
    #[inline]
    pub fn due(self, now: u32) -> bool {
        self.start_frame == DISPATCH_ALWAYS_DUE_START
            || (now as i32).wrapping_sub(self.start_frame) >= self.delay
    }

    /// Return the signed wrapping remainder only while dispatch is pending.
    ///
    /// Native dispatch does not saturate this subtraction.
    #[inline]
    pub fn remaining_if_pending(self, now: u32) -> Option<i32> {
        if self.due(now) {
            return None;
        }
        let elapsed = (now as i32).wrapping_sub(self.start_frame);
        Some(self.delay.wrapping_sub(elapsed))
    }
}

impl Default for MissionTimer {
    fn default() -> Self {
        Self {
            start_frame: SENTINEL,
            duration: 0,
        }
    }
}

impl MissionTimer {
    /// Construct armed at `start_frame` for `duration` frames.
    #[inline]
    pub fn armed(start_frame: u32, duration: u32) -> Self {
        Self {
            start_frame,
            duration,
        }
    }

    /// `true` once `duration` frames have elapsed since `start_frame` (inclusive),
    /// or always when unarmed.
    #[inline]
    pub fn due(self, now: u32) -> bool {
        self.start_frame == SENTINEL || now.wrapping_sub(self.start_frame) >= self.duration
    }

    /// Re-anchor at `now` for `n` frames.
    #[inline]
    pub fn defer(&mut self, now: u32, n: u32) {
        self.start_frame = now;
        self.duration = n;
    }

    /// Alias of [`MissionTimer::defer`].
    #[inline]
    pub fn arm(&mut self, now: u32, n: u32) {
        self.defer(now, n);
    }

    /// Alias of `defer(now, 0)` — due again on the next check.
    #[inline]
    pub fn reset(&mut self, now: u32) {
        self.defer(now, 0);
    }

    /// Disarm → always due.
    #[inline]
    pub fn clear(&mut self) {
        self.start_frame = SENTINEL;
        self.duration = 0;
    }

    /// `true` while a live frame anchor is set.
    #[inline]
    pub fn is_armed(self) -> bool {
        self.start_frame != SENTINEL
    }

    /// Frames since the anchor (0 when unarmed).
    #[inline]
    pub fn elapsed(self, now: u32) -> u32 {
        if self.start_frame == SENTINEL {
            0
        } else {
            now.wrapping_sub(self.start_frame)
        }
    }

    /// Frames left before due (0 when unarmed or already due; saturating).
    #[inline]
    pub fn remaining(self, now: u32) -> u32 {
        if self.start_frame == SENTINEL {
            0
        } else {
            self.duration
                .saturating_sub(now.wrapping_sub(self.start_frame))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_dispatch_raw_fields_and_frame_bits_round_trip() {
        let raw = MissionDispatchTimer::from_raw(i32::MIN, i32::MAX);
        assert_eq!(raw.start_frame(), i32::MIN);
        assert_eq!(raw.delay(), i32::MAX);

        let high_bit_frame = MissionDispatchTimer::at_frame(0x8000_0000);
        assert_eq!(high_bit_frame.start_frame(), i32::MIN);
        assert_eq!(high_bit_frame.delay(), 0);
        assert!(high_bit_frame.due(0x8000_0000));
    }

    #[test]
    fn signed_dispatch_minus_one_start_is_always_due() {
        let timer = MissionDispatchTimer::from_raw(-1, i32::MAX);

        assert!(timer.due(0));
        assert!(timer.due(u32::MAX));
        assert_eq!(timer.remaining_if_pending(0), None);
    }

    #[test]
    fn signed_dispatch_negative_delay_is_due_at_anchor() {
        let timer = MissionDispatchTimer::from_raw(10, -1);

        assert!(timer.due(10));
        assert_eq!(timer.remaining_if_pending(10), None);
    }

    #[test]
    fn signed_dispatch_exact_boundary_is_inclusive() {
        let timer = MissionDispatchTimer::from_raw(10, 5);

        assert!(!timer.due(14));
        assert_eq!(timer.remaining_if_pending(14), Some(1));
        assert!(timer.due(15));
        assert_eq!(timer.remaining_if_pending(15), None);
    }

    #[test]
    fn signed_dispatch_wraparound_uses_signed_elapsed_comparison() {
        let start = i32::MAX - 1;
        let now = (i32::MIN + 1) as u32;

        let pending = MissionDispatchTimer::from_raw(start, 4);
        assert!(!pending.due(now));
        assert_eq!(pending.remaining_if_pending(now), Some(1));

        let due = MissionDispatchTimer::from_raw(start, 3);
        assert!(due.due(now));
        assert_eq!(due.remaining_if_pending(now), None);
    }

    #[test]
    fn signed_dispatch_remaining_uses_wrapping_subtraction() {
        let timer = MissionDispatchTimer::from_raw(10, i32::MAX);
        let now = (10i32.wrapping_add(i32::MIN)) as u32;

        assert!(!timer.due(now));
        assert_eq!(timer.remaining_if_pending(now), Some(-1));
    }

    #[test]
    fn unarmed_is_always_due() {
        let t = MissionTimer::default();
        assert!(!t.is_armed());
        assert!(t.due(0));
        assert!(t.due(12_345));
    }

    #[test]
    fn inclusive_due_boundary() {
        let t = MissionTimer::armed(10, 5);
        assert!(t.is_armed());
        assert!(!t.due(14)); // 14 - 10 = 4 < 5
        assert!(t.due(15)); // 15 - 10 = 5 >= 5 (inclusive)
        assert!(t.due(16));
    }

    #[test]
    fn defer_zero_is_due_next_check() {
        let mut t = MissionTimer::armed(100, 50);
        t.defer(200, 0);
        assert_eq!((t.start_frame, t.duration), (200, 0));
        assert!(t.due(200)); // 0 >= 0
    }

    #[test]
    fn arm_and_reset_alias_defer() {
        let mut a = MissionTimer::default();
        a.arm(10, 7);
        assert_eq!((a.start_frame, a.duration), (10, 7));
        let mut b = MissionTimer::default();
        b.reset(10);
        assert_eq!((b.start_frame, b.duration), (10, 0));
    }

    #[test]
    fn clear_makes_due_again() {
        let mut t = MissionTimer::armed(10, 100);
        assert!(!t.due(20));
        t.clear();
        assert!(!t.is_armed());
        assert!(t.due(20));
    }

    #[test]
    fn elapsed_and_remaining() {
        let t = MissionTimer::armed(10, 5);
        assert_eq!(t.elapsed(13), 3);
        assert_eq!(t.remaining(13), 2);
        assert_eq!(t.remaining(15), 0);
        assert_eq!(t.remaining(99), 0); // saturating, never underflows
        let s = MissionTimer::default();
        assert_eq!(s.elapsed(50), 0);
        assert_eq!(s.remaining(50), 0);
    }

    #[test]
    fn reversal_arithmetic_matches_gate_reverse() {
        // Mirrors the building gate's direction-reversal math: a transition armed
        // at frame 100 for 39 ticks, reversed at frame 110 against a nominal total
        // of 39, yields duration 10 with the start-frame baseline preserved.
        let mut t = MissionTimer::armed(100, 39);
        let total = 39u32;
        let elapsed = t.elapsed(110);
        let live_remaining = t.duration.saturating_sub(elapsed);
        t.duration = total.saturating_sub(live_remaining);
        assert_eq!(t.duration, 10);
        assert_eq!(t.start_frame, 100);
    }

    #[test]
    fn wraparound_delta_is_correct() {
        // Anchor near the top of u32; `now` has wrapped past 0.
        let t = MissionTimer::armed(u32::MAX - 2, 5);
        assert!(t.due(2)); // 2 - (MAX-2) wraps to 5 -> due (5 >= 5)
        assert!(!t.due(1)); // wraps to 4 -> not due
    }
}
