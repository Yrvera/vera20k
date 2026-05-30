//! gamemd kind-1 static text reveal for the Skirmish 0x102 right-panel labels.
//! Pure presentation state: a per-label character cursor advancing one step per
//! 30 ms, started by the shell first-paint slide completion (the 0x4EC->0x4EE
//! event). Renders the first `count` characters (v1: hard left-to-right wipe;
//! the trailing highlight gradient is deferred). No sim coupling.

use std::time::{Duration, Instant};

/// Native timer interval for kind-1 reveal (0x1E ms).
pub const REVEAL_TICK_MS: u32 = 30;
/// Native reveal step added per advance.
pub const REVEAL_STEP: u32 = 1;
/// Native trailing highlight window (range) for these statics.
pub const REVEAL_RANGE: u32 = 8;

/// Per-label reveal cursor. Default = inactive (renders full text).
#[derive(Debug, Clone)]
pub struct StaticReveal {
    /// `None` => inactive: render the whole string (steady state).
    /// `Some(_)` => running or completed reveal with a character cursor.
    running: Option<RunState>,
}

#[derive(Debug, Clone)]
struct RunState {
    /// Native +0x80 reveal count; starts at 1.
    count: u32,
    /// Inclusive target = char_len + 1 + REVEAL_RANGE; advance stops at/after this.
    target: u32,
    last_step_at: Instant,
}

impl Default for StaticReveal {
    fn default() -> Self {
        // Inactive by default so labels render full text until a reveal starts.
        Self { running: None }
    }
}

impl StaticReveal {
    /// Begin (or restart) the reveal for `text` at the given instant.
    /// target = char count + 1 + range (native wcslen(text)+1+range).
    pub fn start(&mut self, text: &str, now: Instant) {
        let target = text.chars().count() as u32 + 1 + REVEAL_RANGE;
        self.running = Some(RunState {
            count: 1,
            target,
            last_step_at: now,
        });
    }

    /// Advance at most ONE step per call, only once >= 30 ms elapsed since the
    /// last step. Never collapses multiple steps (faithful to one-per-Sleep).
    pub fn advance(&mut self, now: Instant) {
        let step = Duration::from_millis(u64::from(REVEAL_TICK_MS));
        if let Some(run) = self.running.as_mut() {
            if run.count < run.target && now.duration_since(run.last_step_at) >= step {
                run.count += REVEAL_STEP;
                run.last_step_at += step;
            }
        }
    }

    /// Reveal window to hand the renderer, or `None` when inactive (full text).
    /// Once `count >= target` the reveal is complete; we return `None` so the
    /// renderer draws the full string (native leaves full text drawn after stop).
    pub fn window(&self) -> Option<RevealWindow> {
        match &self.running {
            Some(run) if run.count < run.target => Some(RevealWindow {
                count: run.count,
                range: REVEAL_RANGE,
            }),
            _ => None,
        }
    }
}

/// Character reveal window passed to the text renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealWindow {
    /// Number of leading characters drawn (chars >= count are hidden).
    pub count: u32,
    /// Trailing highlight-gradient width.
    pub range: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_renders_full_text() {
        // Inactive default => no reveal window => renderer draws full string.
        assert_eq!(StaticReveal::default().window(), None);
    }

    #[test]
    fn start_sets_count_one_and_target_len_plus_9() {
        let t0 = Instant::now();
        let mut r = StaticReveal::default();
        r.start("ABCDE", t0); // 5 chars => target 5+1+8 = 14
        assert_eq!(r.window(), Some(RevealWindow { count: 1, range: 8 }));
    }

    #[test]
    fn advance_one_step_per_30ms_no_catchup_and_stops_at_target() {
        let t0 = Instant::now();
        let mut r = StaticReveal::default();
        r.start("AB", t0); // target = 2+1+8 = 11
        r.advance(t0 + Duration::from_millis(29));
        assert_eq!(r.window().unwrap().count, 1); // not yet
        r.advance(t0 + Duration::from_millis(30));
        assert_eq!(r.window().unwrap().count, 2);
        // A 1-second gap advances only ONE step (no catch-up).
        r.advance(t0 + Duration::from_millis(1030));
        assert_eq!(r.window().unwrap().count, 3);
        // Drive to target: count reaches 11 => window() becomes None (full text).
        let mut t = t0 + Duration::from_millis(1030);
        for _ in 0..20 {
            t += Duration::from_millis(30);
            r.advance(t);
        }
        assert_eq!(r.window(), None);
    }

    #[test]
    fn restart_resets_count_to_one() {
        let t0 = Instant::now();
        let mut r = StaticReveal::default();
        r.start("ABCDEFGHIJ", t0);
        for i in 1..=5 {
            r.advance(t0 + Duration::from_millis(30 * i));
        }
        r.start("NEWMAP", t0 + Duration::from_millis(500)); // text changed mid-reveal
        assert_eq!(r.window().unwrap().count, 1);
    }
}
