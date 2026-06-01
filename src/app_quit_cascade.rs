//! Graceful main-menu quit cascade (presentation/teardown only).
//!
//! A non-blocking per-frame state machine, modelled on
//! [`crate::app_shell_transition::ShellFrameWave`]: it owns timing only and
//! returns the effects the app applies each frame (lower music volume, hard-stop
//! music, exit). The original runs this teardown as a blocking thread spin; the
//! port reproduces the same observable timing across frames so the winit event
//! loop keeps turning. Reproduces: music fade-out (concurrent with a bounded wait
//! for trailing voices) → music hard-stop → exit. The screen fade-to-black phase
//! is added in sub-step 4b-ii-b.
//!
//! ## Dependency rules
//! - App layer; depends only on std. No render/sim/audio type deps (the app maps
//!   directives onto MusicPlayer/SfxPlayer), so it stays headless-testable.

use std::time::Instant;

/// Music volume-fade rate: full scale (1.0) over 1000 ms, matching the original's
/// theme stop-with-fade (a full-scale volume interpolator over a 1000 ms divisor).
/// A fade from volume `v` therefore reaches silence in `v * 1000` ms.
const MUSIC_FADE_PER_MS: f64 = 1.0 / 1000.0;

/// Safety ceiling on the fade + trailing-voice wait. The original bounds its vox
/// pump-wait at `0xBB8` timer ticks (3000 × 16 ms). Effectively dominated by the
/// ≤1 s music fade, so this is reached only if audio never reports done.
const WAIT_CEILING_MS: u64 = 0xBB8 * 16; // 48_000 ms

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuitPhase {
    /// Music fading out while any trailing EVA/menu voice plays. Ends the instant
    /// the music fade completes OR voices finish OR the ceiling is hit.
    FadeMusicAndWaitVoices,
    /// Terminal — the app exits the event loop.
    Done,
}

/// Per-frame effects the app applies after ticking the cascade.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct QuitCascadeTick {
    /// New music volume to apply this frame (None once the fade is over).
    pub music_volume: Option<f64>,
    /// Hard-stop the music this frame (one-shot, on the fade→teardown edge).
    pub stop_music: bool,
    /// The cascade has finished — exit the event loop.
    pub finished: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct QuitCascade {
    phase: QuitPhase,
    phase_started_at: Instant,
    /// Live music volume captured when the cascade began (the fade start point).
    start_music_volume: f64,
}

impl QuitCascade {
    /// Begin the cascade. `start_music_volume` is the live music volume at quit
    /// time (the caller has already persisted it before calling this).
    pub(crate) fn start(now: Instant, start_music_volume: f64) -> Self {
        Self {
            phase: QuitPhase::FadeMusicAndWaitVoices,
            phase_started_at: now,
            start_music_volume: start_music_volume.clamp(0.0, 1.0),
        }
    }

    /// Advance the cascade and return the effects to apply this frame.
    /// `voices_active` is the live "any EVA/voice still playing" poll.
    pub(crate) fn tick(&mut self, now: Instant, voices_active: bool) -> QuitCascadeTick {
        let elapsed_ms = now.duration_since(self.phase_started_at).as_millis() as u64;
        match self.phase {
            QuitPhase::FadeMusicAndWaitVoices => {
                let faded =
                    (self.start_music_volume - elapsed_ms as f64 * MUSIC_FADE_PER_MS).max(0.0);
                let music_done = faded <= 0.0;
                if music_done || !voices_active || elapsed_ms >= WAIT_CEILING_MS {
                    self.enter(QuitPhase::Done, now);
                    // 4b-ii-a: hard-stop music and exit on the same edge (the
                    // screen fade is inserted between these in 4b-ii-b).
                    return QuitCascadeTick {
                        stop_music: true,
                        finished: true,
                        ..Default::default()
                    };
                }
                QuitCascadeTick {
                    music_volume: Some(faded),
                    ..Default::default()
                }
            }
            QuitPhase::Done => QuitCascadeTick {
                finished: true,
                ..Default::default()
            },
        }
    }

    fn enter(&mut self, phase: QuitPhase, now: Instant) {
        self.phase = phase;
        self.phase_started_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    /// The music volume ramps linearly toward 0 while voices stay active.
    #[test]
    fn music_volume_ramps_down_linearly() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 1.0);
        let mid = c.tick(at(t0, 500), true);
        assert_eq!(mid.music_volume, Some(0.5));
        assert!(!mid.finished && !mid.stop_music);
        let near = c.tick(at(t0, 900), true);
        assert!((near.music_volume.unwrap() - 0.1).abs() < 1e-9);
    }

    /// When the fade reaches silence, the cascade hard-stops music and finishes.
    #[test]
    fn fade_completion_stops_music_and_finishes() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 1.0);
        let end = c.tick(at(t0, 1000), true);
        assert!(end.stop_music && end.finished);
        // Stays finished thereafter.
        assert!(c.tick(at(t0, 1001), true).finished);
    }

    /// A trailing voice finishing early ends the wait before the fade completes.
    #[test]
    fn voices_done_ends_wait_early() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 1.0);
        let end = c.tick(at(t0, 200), false); // voices already done
        assert!(end.stop_music && end.finished);
    }

    /// Default menu volume (0.4) fades to silence in ~400 ms.
    #[test]
    fn default_volume_fades_in_400ms() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 0.4);
        assert_eq!(c.tick(at(t0, 200), true).music_volume, Some(0.2));
        assert!(c.tick(at(t0, 400), true).finished);
    }
}
