//! App-local admission for gameplay frames.
//!
//! The deterministic simulation never reads wall time. This pacer only decides
//! whether one outer event-loop iteration may admit one gameplay frame.

use std::time::Instant;

const FRAME_BUCKET_SHIFT: u32 = 4;
const FRAME_BUCKET_MS: u64 = 1 << FRAME_BUCKET_SHIFT;
const MIN_TIMED_GAME_SPEED: u8 = 1;
const MAX_TIMED_GAME_SPEED: u8 = 6;

#[cfg(windows)]
#[link(name = "winmm")]
unsafe extern "system" {
    fn timeGetTime() -> u32;
}

/// Return the app-local pacing clock.
///
/// On Windows this is the same wrapping millisecond authority used by the
/// retail executable. Other targets retain a monotonic development fallback.
pub(crate) fn wall_clock_ms(fallback_epoch: Instant, now: Instant) -> u64 {
    #[cfg(windows)]
    {
        let _ = (fallback_epoch, now);
        // SAFETY: timeGetTime takes no arguments and returns the process-wide
        // Windows uptime counter as an unsigned 32-bit millisecond word.
        u64::from(unsafe { timeGetTime() })
    }
    #[cfg(not(windows))]
    {
        now.duration_since(fallback_epoch).as_millis() as u64
    }
}

#[derive(Debug, Default)]
pub(crate) struct LocalFramePacer {
    last_frame_start_bucket: Option<u32>,
}

impl LocalFramePacer {
    pub(crate) const fn new() -> Self {
        Self {
            last_frame_start_bucket: None,
        }
    }

    pub(crate) fn should_admit(&self, now_ms: u64, game_speed: u8, paused: bool) -> bool {
        if paused {
            return false;
        }
        if game_speed == 0 {
            return true;
        }
        let Some(last_bucket) = self.last_frame_start_bucket else {
            return true;
        };
        let required_buckets =
            u32::from(game_speed.clamp(MIN_TIMED_GAME_SPEED, MAX_TIMED_GAME_SPEED));
        let elapsed = frame_bucket(now_ms).wrapping_sub(last_bucket) as i32;
        elapsed >= required_buckets as i32
    }

    pub(crate) fn record_admitted_frame(&mut self, frame_start_ms: u64) {
        self.last_frame_start_bucket = Some(frame_bucket(frame_start_ms));
    }

    /// Forget the prior pacing window so the next unpaused frame runs now.
    pub(crate) fn reset_for_immediate_frame(&mut self) {
        self.last_frame_start_bucket = None;
    }

    /// Suppress another automatic frame until one full pacing window passes.
    ///
    /// This is for explicit single-step/capture work. Normal match entry and
    /// modal resume use `reset_for_immediate_frame`.
    pub(crate) fn reanchor(&mut self, now_ms: u64) {
        self.last_frame_start_bucket = Some(frame_bucket(now_ms));
    }

    #[cfg(test)]
    fn next_deadline_ms(&self, game_speed: u8) -> Option<u64> {
        if game_speed == 0 {
            return None;
        }
        let last_bucket = self.last_frame_start_bucket?;
        let required_buckets =
            u32::from(game_speed.clamp(MIN_TIMED_GAME_SPEED, MAX_TIMED_GAME_SPEED));
        Some(
            u64::from(last_bucket)
                .saturating_add(u64::from(required_buckets))
                .saturating_mul(FRAME_BUCKET_MS),
        )
    }
}

const fn frame_bucket(now_ms: u64) -> u32 {
    (now_ms as u32) >> FRAME_BUCKET_SHIFT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_unpaused_iteration_is_immediately_eligible() {
        assert!(LocalFramePacer::new().should_admit(123, 1, false));
    }

    #[test]
    fn timed_speeds_require_the_exact_bucket_distance() {
        for speed in MIN_TIMED_GAME_SPEED..=MAX_TIMED_GAME_SPEED {
            let mut pacer = LocalFramePacer::new();
            pacer.record_admitted_frame(32);
            let deadline = 32 + u64::from(speed) * FRAME_BUCKET_MS;
            assert!(!pacer.should_admit(deadline - 1, speed, false));
            assert!(pacer.should_admit(deadline, speed, false));
            assert_eq!(pacer.next_deadline_ms(speed), Some(deadline));
        }
    }

    #[test]
    fn speed_zero_is_uncapped() {
        let mut pacer = LocalFramePacer::new();
        pacer.record_admitted_frame(160);
        assert!(pacer.should_admit(160, 0, false));
        assert_eq!(pacer.next_deadline_ms(0), None);
    }

    #[test]
    fn pause_does_not_consume_an_eligible_frame() {
        let mut pacer = LocalFramePacer::new();
        pacer.record_admitted_frame(160);
        assert!(!pacer.should_admit(176, 1, true));
        assert!(pacer.should_admit(176, 1, false));
    }

    #[test]
    fn a_long_stall_still_admits_only_one_frame() {
        let mut pacer = LocalFramePacer::new();
        pacer.record_admitted_frame(0);
        assert!(pacer.should_admit(10_000, 1, false));
        pacer.record_admitted_frame(10_000);
        assert!(!pacer.should_admit(10_000, 1, false));
    }

    #[test]
    fn reanchor_discards_elapsed_wall_time() {
        let mut pacer = LocalFramePacer::new();
        pacer.record_admitted_frame(0);
        assert!(pacer.should_admit(10_000, 1, false));
        pacer.reanchor(10_000);
        assert!(!pacer.should_admit(10_000, 1, false));
        assert!(pacer.should_admit(10_016, 1, false));
    }

    #[test]
    fn out_of_range_speed_clamps_to_six_buckets() {
        let mut pacer = LocalFramePacer::new();
        pacer.record_admitted_frame(0);
        assert!(!pacer.should_admit(95, u8::MAX, false));
        assert!(pacer.should_admit(96, u8::MAX, false));
    }

    #[test]
    fn native_signed_bucket_subtraction_stalls_at_uptime_rollover() {
        let mut pacer = LocalFramePacer::new();
        pacer.record_admitted_frame(u64::from(u32::MAX - 7));

        assert!(!pacer.should_admit(u64::from(u32::MAX) + 1, 1, false));
    }
}
