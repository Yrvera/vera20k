//! Data-driven first-paint slide eligibility + frame schedule (contract C11).
//!
//! Every allow-listed front-end shell dialog (main menu `0xE2`, single player
//! `0x100`, skirmish setup `0x102`, …) plays a one-shot controls-reveal slide on
//! its OWN first paint — not a screen-edge crossfade. Each owner-draw control's
//! chrome-SHP (SDBTNANM) frame index advances on a staggered 30 ms-per-frame
//! schedule; controls are never repositioned. This module owns the two
//! render-agnostic halves of that behaviour:
//!   * **eligibility data** — the dialog-id allow-list (`is_slide_eligible`) and
//!     per-dialog animated owner-draw-button count (`slot_count_for`), and
//!   * **the frame schedule** — [`ShellFrameWave`], the SDBTNANM frame sweep with
//!     the verified tick cadence and loop bound.
//!
//! Render-agnostic: depends only on [`DialogId`] + `std` (no sim/render/assets),
//! honouring the `ui/` layering rule. The app layer (`app_shell_transition`) maps
//! the showing screen to a `DialogId`, drives the wave each frame, and plays the
//! start cue (`GUIMoveInSound`, stock `MenuSlideIn`); the stock-empty end cue
//! (`ShellButtonSlideSound`) stays silent.

use std::time::{Duration, Instant};

use super::descriptor::DialogId;

/// One animation tick per 30 ms, advancing exactly one frame (never skipped).
pub(crate) const WAVE_TICK_MS: u32 = 30;
/// Extra ticks after the last schedule entry so the ramp completes. The loop
/// bound is `max(schedule entry) + WAVE_TAIL_TICKS`.
pub(crate) const WAVE_TAIL_TICKS: u32 = 6;
/// Linear ramp length (delta 0..=5 inclusive => 6 steps).
pub(crate) const WAVE_RAMP_STEPS: i32 = 6;

/// SDBTNANM frame constants per button group. Each tuple is
/// `(held_before, ramp_base, held_after)`. With `dir() = -1` on slide-IN, the IN
/// ramp counts DOWN from `base`; the held terminals are distinct constants, not
/// `base`. Slide-OUT uses `dir() = +1` (ramp counts UP).
/// Group A = regular owner-draw button cell (SDBTNANM 10→5, settle 1).
/// Group B = the "second cell group" (SDBTNANM 16→11, settle 0) — not yet wired
/// by any consumer.
pub(crate) struct WaveFrames {
    pub before: i32,
    pub base: i32,
    pub after: i32,
}
/// SHOW: hold 10 → ramp 10,9,8,7,6,5 → settle 1.
pub(crate) const GROUP_A_IN: WaveFrames = WaveFrames {
    before: 10,
    base: 10,
    after: 1,
};
/// CLOSE: hold 1 → ramp 5,6,7,8,9,10 → settle 10.
pub(crate) const GROUP_A_OUT: WaveFrames = WaveFrames {
    before: 1,
    base: 5,
    after: 10,
};
/// SHOW: hold 10 → ramp 16,15,14,13,12,11 → settle 0.
pub(crate) const GROUP_B_IN: WaveFrames = WaveFrames {
    before: 10,
    base: 16,
    after: 0,
};
/// CLOSE: hold 0 → ramp 11,12,13,14,15,16 → settle 10.
pub(crate) const GROUP_B_OUT: WaveFrames = WaveFrames {
    before: 0,
    base: 11,
    after: 10,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveDirection {
    SlideIn,
    /// Slide-OUT (close) ramp. Modeled for completeness — the OUT frame
    /// constants are part of the faithful schedule — but the first-paint driver
    /// only ever runs slide-IN, so no consumer constructs this yet.
    #[allow(dead_code)]
    SlideOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonGroup {
    A,
    /// The "second cell group" (SDBTNANM 16→11). Modeled for completeness; not
    /// yet wired by any shell renderer (all current shells use group A).
    #[allow(dead_code)]
    B,
}

impl WaveDirection {
    /// Frame multiplier: -1 on slide-in (frames count DOWN, e.g. 10→5), +1 on
    /// slide-out (frames count UP). Matches the original ramp direction (-1 on
    /// show / +1 on close).
    fn dir(self) -> i32 {
        match self {
            WaveDirection::SlideIn => -1,
            WaveDirection::SlideOut => 1,
        }
    }
}

// --- Data-driven slide eligibility + per-dialog slot count (allow-list) -------

/// A slide-eligible front-end shell dialog that has a renderer here, plus its
/// animated owner-draw button count `N`. `N` = the dialog's visible/enabled
/// owner-draw button children (the SDBTNANM cells) — statics/headings do not
/// animate. `N` sets the per-cell stagger length and therefore the loop bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellSlideSpec {
    pub dialog_id: u16,
    pub slot_count: u32,
}

/// Front-end shell dialogs we render today, with their animated slot counts.
/// `0xE2` main menu: Single Player / WW Online / Network / Movies / Options /
/// Exit (6). `0x100` single player: 4 owner-draw buttons. `0x102` skirmish
/// setup: Start Game / Choose Map / Back (3 right-panel buttons).
pub(crate) const RENDERED_SHELL_SLIDES: &[ShellSlideSpec] = &[
    ShellSlideSpec {
        dialog_id: 0x00E2,
        slot_count: 6,
    },
    ShellSlideSpec {
        dialog_id: 0x0100,
        slot_count: 4,
    },
    ShellSlideSpec {
        dialog_id: 0x0102,
        slot_count: 3,
    },
];

/// Front-end shell dialog ids that slide on first paint (the eligibility
/// allow-list, scoped to the front-end shells). The three rendered shells
/// (`RENDERED_SHELL_SLIDES`) plus the front-end dialogs documented as
/// allow-listed but not yet rendered here (`0x94`/`0x6B`/`0x101` per
/// `docs/research/skirmish-ui/SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`
/// §3); those slide automatically once a renderer maps to them and gains a
/// `RENDERED_SHELL_SLIDES` slot count. The original's full allow-list is wider
/// (~58 ids, mostly network/WOL setup dialogs that are out of scope here).
/// Excluded: modal dialogs (`0x120` confirm, `0xCE` body-ok) and the in-game
/// Options dialog (`0xBBB`), all of which carry `slide_eligible = false`.
pub(crate) const SHELL_SLIDE_ALLOW_LIST: &[u16] =
    &[0x00E2, 0x0094, 0x006B, 0x0100, 0x0101, 0x0102];

/// Whether a dialog plays the first-paint controls-reveal slide.
pub(crate) fn is_slide_eligible(id: DialogId) -> bool {
    SHELL_SLIDE_ALLOW_LIST.contains(&id.0)
}

/// Animated owner-draw button count for a rendered shell dialog (`N`, the stagger
/// length). `None` for an allow-listed dialog that has no renderer here yet — the
/// app layer only ever drives the slide for dialogs it actually paints.
pub(crate) fn slot_count_for(id: DialogId) -> Option<u32> {
    RENDERED_SHELL_SLIDES
        .iter()
        .find(|s| s.dialog_id == id.0)
        .map(|s| s.slot_count)
}

// --- Frame schedule (the wave) -----------------------------------------------

/// Per-dialog first-paint slide state. `None` (Idle) → `Some(running)` →
/// `Some(complete)` mirrors the original's `1→2→3` slide state machine: the wave
/// is created on the dialog's entry edge, advances one frame per 30 ms tick, and
/// signals completion via [`ShellFrameWave::is_complete`].
#[derive(Debug, Clone)]
pub(crate) struct ShellFrameWave {
    last_step_at: Instant,
    /// 0-based current tick.
    tick: u32,
    /// number of animated control slots (N).
    #[allow(dead_code)]
    slot_count: u32,
    /// inclusive loop bound = max(schedule entry) + WAVE_TAIL_TICKS.
    total_ticks: u32,
    direction: WaveDirection,
}

impl ShellFrameWave {
    pub(crate) fn new_first_paint_slide(slot_count: u32, now: Instant) -> Self {
        Self {
            last_step_at: now,
            tick: 0,
            slot_count,
            total_ticks: Self::total_ticks_for(slot_count),
            direction: WaveDirection::SlideIn,
        }
    }

    /// Loop bound (total frames) for `N` animated button slots. Replicates the
    /// schedule-array build: the N button slots take entry ticks `1..=N` (index
    /// `s` ← `s+1`), and a fixed radar-open anchor slot is written at entry tick
    /// `N+3`, which the max-scan always picks as the largest schedule entry. The
    /// loop then runs `max + WAVE_TAIL_TICKS` ticks, so total = `N + 3 + 6 =
    /// N + 9`. (The radar group itself only DRAWS when its `+0xD5`-family gate is
    /// set, but its schedule slot is written unconditionally, so the bound is a
    /// pure function of N for every shell.)
    fn total_ticks_for(slot_count: u32) -> u32 {
        let max_entry = slot_count + 3;
        max_entry + WAVE_TAIL_TICKS
    }

    /// Entry tick for a control slot (the stagger): slot 0 enters at tick 1.
    fn entry_tick(slot: u32) -> i32 {
        slot as i32 + 1
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.tick >= self.total_ticks
    }

    /// Advance at most ONE tick per call, only once >= 30 ms has elapsed.
    /// Never collapses multiple indices (faithful to one-frame-per-Sleep).
    pub(crate) fn advance(&mut self, now: Instant) {
        let step = Duration::from_millis(u64::from(WAVE_TICK_MS));
        if self.tick < self.total_ticks && now.duration_since(self.last_step_at) >= step {
            self.tick += 1;
            self.last_step_at += step;
        }
    }

    /// Frame index for an SDBTNANM button at the current tick.
    /// 4-case: held-before / linear ramp (base + delta*dir) / held-after.
    /// Terminal frames are DISTINCT constants, not `base` (verified from binary).
    pub(crate) fn sdbtnanm_frame(&self, slot: u32, group: ButtonGroup) -> usize {
        let f = match (group, self.direction) {
            (ButtonGroup::A, WaveDirection::SlideIn) => GROUP_A_IN,
            (ButtonGroup::A, WaveDirection::SlideOut) => GROUP_A_OUT,
            (ButtonGroup::B, WaveDirection::SlideIn) => GROUP_B_IN,
            (ButtonGroup::B, WaveDirection::SlideOut) => GROUP_B_OUT,
        };
        let dir = self.direction.dir();
        let delta = self.tick as i32 - Self::entry_tick(slot);
        let frame = if delta < 0 {
            f.before // held at the group's "before" terminal
        } else if delta < WAVE_RAMP_STEPS {
            f.base + delta * dir // 6-step ramp
        } else {
            f.after // held at the group's "after" terminal
        };
        frame.max(0) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_ticks_is_max_schedule_plus_tail() {
        // N=5 buttons => max entry N+3=8, total = 8 + 6 = 14 (= N+9). The
        // radar-open anchor slot (entry tick N+3) is the max, not the last
        // button (entry tick N).
        let w = ShellFrameWave::new_first_paint_slide(5, Instant::now());
        assert_eq!(w.total_ticks, 5 + 3 + WAVE_TAIL_TICKS);
        assert_eq!(w.total_ticks, 5 + 9);
    }

    #[test]
    fn total_ticks_matches_native_table() {
        // Binary loop bound per N (= total frames): N=3→12, N=4→13, N=6→15.
        for (n, total) in [(3, 12), (4, 13), (6, 15)] {
            let w = ShellFrameWave::new_first_paint_slide(n, Instant::now());
            assert_eq!(w.total_ticks, total, "N={n}");
        }
    }

    #[test]
    fn advance_steps_one_frame_per_30ms_and_never_collapses() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_first_paint_slide(4, t0);
        w.advance(t0 + Duration::from_millis(29));
        assert_eq!(w.tick, 0);
        w.advance(t0 + Duration::from_millis(30));
        assert_eq!(w.tick, 1);
        // A 1-second gap must still advance only ONE index (no catch-up).
        w.advance(t0 + Duration::from_millis(1030));
        assert_eq!(w.tick, 2);
    }

    #[test]
    fn group_a_slide_in_holds_10_ramps_10_to_5_then_holds_1() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_first_paint_slide(3, t0);
        // slot 1 enters at tick 2; before that it holds at the "before" terminal = 10.
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
        for _ in 0..2 {
            w.tick += 1;
        } // tick = 2 => delta 0 => base 10
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
        w.tick += 5; // delta 5 => 10 + 5*-1 = 5 (last ramp step)
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 5);
        w.tick += 3; // delta >= 6 => held "after" terminal = 1
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 1);
    }

    #[test]
    fn group_b_slide_in_holds_10_ramps_16_to_11_then_holds_0() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_first_paint_slide(3, t0);
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 10); // before-entry (slot 0 enters tick 1)
        w.tick += 1; // delta 0 => base 16
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 16);
        w.tick += 5; // delta 5 => 16 + 5*-1 = 11
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 11);
        w.tick += 3; // delta >= 6 => held "after" = 0
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 0);
    }

    #[test]
    fn rendered_shells_are_all_eligible() {
        for spec in RENDERED_SHELL_SLIDES {
            assert!(
                is_slide_eligible(DialogId(spec.dialog_id)),
                "rendered shell {:#06x} must be on the allow-list",
                spec.dialog_id
            );
            assert_eq!(
                slot_count_for(DialogId(spec.dialog_id)),
                Some(spec.slot_count)
            );
        }
    }

    #[test]
    fn modal_and_in_game_dialogs_do_not_slide() {
        // Modal confirm/body-ok and the in-game Options dialog are excluded.
        for id in [0x0120u16, 0x00CE, 0x0BBB] {
            assert!(!is_slide_eligible(DialogId(id)), "{id:#06x} must not slide");
            assert_eq!(slot_count_for(DialogId(id)), None);
        }
    }

    #[test]
    fn allow_listed_but_unrendered_dialogs_have_no_slot_count() {
        // 0x94/0x6B/0x101 are eligible per research but have no renderer yet, so
        // the app layer never drives them; slot count is therefore unknown.
        for id in [0x0094u16, 0x006B, 0x0101] {
            assert!(is_slide_eligible(DialogId(id)));
            assert_eq!(slot_count_for(DialogId(id)), None);
        }
    }
}
