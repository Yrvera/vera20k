//! `MissionTimer` — the single frame-anchored deferral primitive.
//!
//! The mission throttle snapshots the global frame counter and tests a delta —
//! it never decrements, so skipped ticks never drift the cadence. This
//! generalizes the building-gate's already-correct `(last_frame, ticks_remaining)`
//! model. Pure integer `u32`, wrapping arithmetic, no float. sim/ only.
use serde::{Deserialize, Serialize};

/// "Unarmed / always due" (the -1 start). `u32::MAX`: the live counter starts at
/// 0 and would take ~3.3 years at 15fps to reach it, so it is never a live value.
pub const SENTINEL: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionTimer {
    pub start_frame: u32,
    pub duration: u32,
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
            self.duration.saturating_sub(now.wrapping_sub(self.start_frame))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn wraparound_delta_is_correct() {
        // Anchor near the top of u32; `now` has wrapped past 0.
        let t = MissionTimer::armed(u32::MAX - 2, 5);
        assert!(t.due(2)); // 2 - (MAX-2) wraps to 5 -> due (5 >= 5)
        assert!(!t.due(1)); // wraps to 4 -> not due
    }
}
