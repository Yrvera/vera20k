//! Single Player -> Skirmish shell frame-index wave transition.
//!
//! Animates each shell control's chrome-SHP frame index on a staggered schedule.
//! No positional slide, no crossfade. Presentation layer only; `sim/` untouched.

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::app::AppState;

/// One animation tick per 30 ms, advancing exactly one frame (never skipped).
pub(crate) const WAVE_TICK_MS: u32 = 30;
/// Extra ticks after the last control's entry so the ramp completes.
pub(crate) const WAVE_TAIL_TICKS: u32 = 6;
/// Linear ramp length (delta 0..=5 inclusive => 6 steps).
pub(crate) const WAVE_RAMP_STEPS: i32 = 6;

/// SDBTNANM frame constants per button group, verified from the binary frame schedule.
/// Each tuple is (held_before, ramp_base, held_after) for slide-IN; slide-OUT swaps
/// held_before<->held_after and negates the ramp direction.
/// Group A = enabled "active" buttons; Group B = the remaining buttons.
pub(crate) struct WaveFrames {
    pub before: i32,
    pub base: i32,
    pub after: i32,
}
pub(crate) const GROUP_A_IN: WaveFrames = WaveFrames { before: 1, base: 5, after: 10 };
pub(crate) const GROUP_A_OUT: WaveFrames = WaveFrames { before: 10, base: 10, after: 1 };
pub(crate) const GROUP_B_IN: WaveFrames = WaveFrames { before: 0, base: 11, after: 10 };
pub(crate) const GROUP_B_OUT: WaveFrames = WaveFrames { before: 10, base: 16, after: 0 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveDirection {
    SlideIn,
    SlideOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonGroup {
    A,
    B,
}

impl WaveDirection {
    /// frame multiplier: +1 on slide-in, -1 on slide-out.
    fn dir(self) -> i32 {
        match self {
            WaveDirection::SlideIn => 1,
            WaveDirection::SlideOut => -1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResizeTransitionResolution {
    ReturnToMainMenu,
    CompleteToSkirmish,
    NoTransition,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellFrameWave {
    last_step_at: Instant,
    /// 0-based current tick.
    tick: u32,
    /// number of animated control slots (N).
    #[allow(dead_code)]
    slot_count: u32,
    /// inclusive max tick = max(entry ticks) + tail.
    total_ticks: u32,
    direction: WaveDirection,
    completion_applied: bool,
}

impl ShellFrameWave {
    pub(crate) fn new_skirmish_slide_in(slot_count: u32, now: Instant) -> Self {
        Self {
            last_step_at: now,
            tick: 0,
            slot_count,
            total_ticks: Self::total_ticks_for(slot_count),
            direction: WaveDirection::SlideIn,
            completion_applied: false,
        }
    }

    /// Replicates the binary schedule-array build: button slots get entry ticks
    /// 1..=N+1, plus anchor slots; total animation = max(schedule) + WAVE_TAIL_TICKS.
    /// For N animated buttons the max entry is N+2 (the SDMPBTN/radar anchor successor),
    /// so total = N + 2 + 6 = N + 8. Computed explicitly rather than approximated.
    fn total_ticks_for(slot_count: u32) -> u32 {
        // schedule entries: 1..=(slot_count+1) for the button column,
        // plus the anchor successor at (slot_count+1)+1; anchors at 0 do not raise the max.
        let max_entry = slot_count + 2;
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

    pub(crate) fn mark_completion_applied(&mut self) -> bool {
        if self.completion_applied {
            return false;
        }
        self.completion_applied = true;
        true
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

pub(crate) fn start_main_menu_to_skirmish(state: &mut AppState) {
    start_main_menu_to_skirmish_at(state, Instant::now());
}

pub(crate) fn start_main_menu_to_skirmish_at(state: &mut AppState, now: Instant) {
    state.main_menu_show_skirmish_setup = false;
    state.main_menu_show_native_skirmish_shell = false;
    state.main_menu_to_skirmish_transition =
        Some(ShellBridgeTransition::new_main_menu_to_skirmish(now));
}

pub(crate) fn blocks_shell_input(state: &AppState) -> bool {
    transition_blocks_shell_input(state.main_menu_to_skirmish_transition.as_ref())
}

pub(crate) fn transition_blocks_shell_input(transition: Option<&ShellFrameWave>) -> bool {
    transition.is_some()
}

pub(crate) fn render_main_menu_to_skirmish_transition(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> Result<bool> {
    if state.main_menu_to_skirmish_transition.is_none() {
        return Ok(false);
    }

    crate::app::App::ensure_skirmish_shell_chrome(state);
    if state.skirmish_shell_chrome.is_none() {
        log::warn!("Skirmish shell chrome unavailable; cancelling shell frame wave");
        state.main_menu_to_skirmish_transition = None;
        return Ok(false);
    }

    if let Some(wave) = state.main_menu_to_skirmish_transition.as_mut() {
        wave.advance(Instant::now());
    }

    // The destination skirmish shell paints itself directly; the wave supplies
    // each right-panel button's SDBTNANM frame for the duration of the slide-in.
    let depth = state.depth_view.clone();
    crate::app_skirmish_shell_render::render_skirmish_shell_to_target(
        state,
        encoder,
        crate::render::shell_transition_pass::ShellRenderTarget {
            color: target,
            depth: &depth,
        },
        crate::app_skirmish_shell_render::ShellRenderMode::TransitionPreview,
    )?;

    if state
        .main_menu_to_skirmish_transition
        .as_ref()
        .is_some_and(ShellFrameWave::is_complete)
    {
        complete_skirmish_slide_in(state);
    }

    Ok(true)
}

fn complete_skirmish_slide_in(state: &mut AppState) {
    let Some(mut wave) = state.main_menu_to_skirmish_transition.take() else {
        return;
    };
    if !wave.mark_completion_applied() {
        return;
    }
    // Wave finished at terminal frame 10; hand back to the normal idle paint
    // (frame2/frame4) by leaving the native shell as the active screen.
    state.main_menu_show_native_skirmish_shell = true;
    crate::app::App::ensure_skirmish_shell_chrome(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_ticks_is_max_schedule_plus_tail() {
        // N=5 buttons => max entry N+2=7, total = 7 + 6 = 13 (≈ N+8).
        let w = ShellFrameWave::new_skirmish_slide_in(5, Instant::now());
        assert_eq!(w.total_ticks, 5 + 2 + WAVE_TAIL_TICKS);
    }

    #[test]
    fn advance_steps_one_frame_per_30ms_and_never_collapses() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_skirmish_slide_in(4, t0);
        w.advance(t0 + Duration::from_millis(29));
        assert_eq!(w.tick, 0);
        w.advance(t0 + Duration::from_millis(30));
        assert_eq!(w.tick, 1);
        // A 1-second gap must still advance only ONE index (no catch-up).
        w.advance(t0 + Duration::from_millis(1030));
        assert_eq!(w.tick, 2);
    }

    #[test]
    fn group_a_slide_in_holds_1_ramps_5_to_10_then_holds_10() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_skirmish_slide_in(3, t0);
        // slot 1 enters at tick 2; before that it holds at the "before" terminal = 1.
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 1);
        for _ in 0..2 {
            w.tick += 1;
        } // tick = 2 => delta 0 => base 5
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 5);
        w.tick += 5; // delta 5 => 5 + 5 = 10 (last ramp step)
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
        w.tick += 3; // delta >= 6 => held "after" terminal = 10
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
    }

    #[test]
    fn group_b_slide_in_holds_0_ramps_11_to_16_then_holds_10() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_skirmish_slide_in(3, t0);
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 0); // before-entry (slot 0 enters tick 1)
        w.tick += 1; // delta 0 => base 11
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 11);
        w.tick += 5; // delta 5 => 16
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 16);
        w.tick += 3; // delta >= 6 => held "after" = 10
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 10);
    }
}
