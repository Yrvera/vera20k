//! Generic shell first-paint slide (menu / single-player / skirmish).
//!
//! gamemd plays a controls-reveal animation on the *first paint of every shell
//! dialog* — not a menu->skirmish edge transition and not a whole-screen
//! crossfade. Each owner-draw control's chrome-SHP frame index advances on a
//! staggered 30 ms-per-frame schedule; controls are never repositioned. This
//! module reproduces that as a per-dialog wave that any shell renderer can
//! consume. Presentation layer only; `sim/` untouched. Silent in stock YR
//! (`ShellButtonSlideSound=` is empty), so no sound is played.

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::app::AppState;

/// One animation tick per 30 ms, advancing exactly one frame (never skipped).
pub(crate) const WAVE_TICK_MS: u32 = 30;
/// Extra ticks after the last control's entry so the ramp completes.
pub(crate) const WAVE_TAIL_TICKS: u32 = 6;
/// Linear ramp length (delta 0..=5 inclusive => 6 steps).
pub(crate) const WAVE_RAMP_STEPS: i32 = 6;

/// SDBTNANM frame constants per button group, verified from `FUN_006071E0`.
/// Each tuple is (held_before, ramp_base, held_after). With `dir() = -1` on
/// slide-IN, the IN ramp counts DOWN from `base`; the held terminals are distinct
/// constants, not `base`. Slide-OUT uses `dir() = +1` (ramp counts UP).
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
    SlideOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonGroup {
    A,
    B,
}

/// Which shell dialog a first-paint slide belongs to. Every allow-listed shell
/// dialog slides on its own first paint; this identifies the one currently
/// showing so the trigger can detect entry edges and pick the control count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellSlideKind {
    /// Dialog 0xE2 — main menu (6 owner-draw buttons).
    MainMenu,
    /// Dialog 0x100 — single-player shell (4 owner-draw buttons).
    SinglePlayer,
    /// Dialog 0x102 — offline skirmish setup (3 right-panel buttons).
    Skirmish,
}

impl ShellSlideKind {
    /// Number of animated owner-draw button slots, which sets the stagger length.
    fn slot_count(self) -> u32 {
        match self {
            ShellSlideKind::MainMenu => 6,
            ShellSlideKind::SinglePlayer => 4,
            ShellSlideKind::Skirmish => 3,
        }
    }
}

impl WaveDirection {
    /// frame multiplier: -1 on slide-in (frames count DOWN, e.g. 10→5), +1 on
    /// slide-out (frames count UP). Matches gamemd `FUN_006071E0` ramp direction
    /// (-1 on show / +1 on close).
    fn dir(self) -> i32 {
        match self {
            WaveDirection::SlideIn => -1,
            WaveDirection::SlideOut => 1,
        }
    }
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

/// Start a shell dialog's first-paint slide. `slot_count` is the number of
/// animated owner-draw button slots, which sets the stagger length.
pub(crate) fn start_shell_first_paint_slide(state: &mut AppState, slot_count: u32) {
    state.shell_first_paint_slide = Some(ShellFrameWave::new_first_paint_slide(
        slot_count,
        Instant::now(),
    ));
}

/// Advance the Skirmish right-panel static text reveals by one cadence step.
/// Call once per frame: the per-label advance is internally 30 ms-gated and a
/// no-op while no reveal is active, so an unconditional per-frame call never
/// over-advances. This lives outside `render_shell_first_paint_slide` because
/// the reveals start *at* the slide's completion edge (when the slide clears)
/// and keep animating afterwards, when that renderer no longer runs.
pub(crate) fn advance_shell_static_reveals(state: &mut AppState) {
    state
        .skirmish_shell_state
        .advance_right_panel_static_reveals(Instant::now());
}

pub(crate) fn blocks_shell_input(state: &AppState) -> bool {
    // The graceful quit cascade also freezes shell input (the original processes
    // no input during its blocking teardown), so a stray click can't re-enter the
    // menu mid-fade.
    state.quit_cascade.is_some()
        || transition_blocks_shell_input(state.shell_first_paint_slide.as_ref())
}

pub(crate) fn transition_blocks_shell_input(transition: Option<&ShellFrameWave>) -> bool {
    transition.is_some()
}

/// Which allow-listed shell dialog is currently showing, if any. Mirrors the
/// main-menu render dispatch order (skirmish > single-player > bare menu); the
/// egui fallback / skirmish-setup paths are not native shell dialogs and do not
/// slide. Returns `None` off the main menu screen.
pub(crate) fn current_shell_slide_target(state: &AppState) -> Option<ShellSlideKind> {
    use crate::ui::game_screen::GameScreen;
    if state.screen != GameScreen::MainMenu {
        return None;
    }
    if state.main_menu_show_native_skirmish_shell || state.dev_skirmish_shell_enabled {
        Some(ShellSlideKind::Skirmish)
    } else if state.main_menu_show_single_player_shell {
        Some(ShellSlideKind::SinglePlayer)
    } else if !state.main_menu_shell_failed && !state.main_menu_show_skirmish_setup {
        Some(ShellSlideKind::MainMenu)
    } else {
        None
    }
}

/// Detect entry into an allow-listed shell dialog and (re)start its first-paint
/// slide. Run once per frame: when the showing shell changes to an eligible one
/// (launch, navigation, return-from-game) a fresh wave begins; leaving all shells
/// cancels any in-flight wave. Mirrors gamemd, where each dialog is re-created on
/// entry and slides on its own first WM_PAINT.
pub(crate) fn update_shell_first_paint_slide_trigger(state: &mut AppState) {
    let target = current_shell_slide_target(state);
    if target == state.shell_slide_active_shell {
        return;
    }
    state.shell_slide_active_shell = target;
    match target {
        Some(kind) => start_shell_first_paint_slide(state, kind.slot_count()),
        None => state.shell_first_paint_slide = None,
    }
}

/// Render the currently-showing shell while its first-paint slide is live, then
/// advance/complete the wave. Returns `true` when it owned the frame. The shell
/// renderer reads `state.shell_first_paint_slide` and swaps each owner-draw
/// button's SDBTNANM frame index — controls are never repositioned, and the rest
/// of the shell paints exactly as it does steady-state.
pub(crate) fn render_shell_first_paint_slide(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> Result<bool> {
    if state.shell_first_paint_slide.is_none() {
        return Ok(false);
    }
    let Some(kind) = current_shell_slide_target(state) else {
        // No eligible shell is showing; drop the stale wave and let the normal
        // dispatch paint this frame.
        state.shell_first_paint_slide = None;
        return Ok(false);
    };

    if let Some(wave) = state.shell_first_paint_slide.as_mut() {
        wave.advance(Instant::now());
    }

    let rendered = match kind {
        ShellSlideKind::Skirmish => {
            crate::app::App::ensure_skirmish_shell_chrome(state);
            if state.skirmish_shell_chrome.is_none() {
                log::warn!("Skirmish shell chrome unavailable; cancelling first-paint slide");
                state.shell_first_paint_slide = None;
                return Ok(false);
            }
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
            true
        }
        ShellSlideKind::SinglePlayer => matches!(
            crate::app_single_player_shell_render::render_single_player_shell(
                state, encoder, target
            )?,
            crate::app_single_player_shell_render::SinglePlayerShellRenderResult::Rendered
        ),
        ShellSlideKind::MainMenu => matches!(
            crate::app_main_menu_shell_render::render_main_menu_shell(state, encoder, target)?,
            crate::app_main_menu_shell_render::MainMenuShellRenderResult::Rendered
        ),
    };

    if !rendered {
        // Shell fell back (assets missing): abandon the slide so the normal
        // dispatch can render the fallback path with its egui overlays.
        state.shell_first_paint_slide = None;
        return Ok(false);
    }

    // Wave finished at its terminal frame; hand back to the steady idle paint by
    // clearing the slide. The shell is already the active screen.
    if state
        .shell_first_paint_slide
        .as_ref()
        .is_some_and(ShellFrameWave::is_complete)
    {
        // Native sends the 0x4EC at slide completion, kicking off the Skirmish
        // right-panel statics' character reveal. Start it here, on the same edge
        // that clears the slide, using the strings the renderer will draw.
        if matches!(kind, ShellSlideKind::Skirmish) {
            let now = Instant::now();
            let (title, game_type, map_label) =
                crate::app_skirmish_shell_render::skirmish_right_panel_label_strings(state);
            state
                .skirmish_shell_state
                .start_right_panel_static_reveals(&title, &game_type, &map_label, now);
        }
        state.shell_first_paint_slide = None;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_ticks_is_max_schedule_plus_tail() {
        // N=5 buttons => max entry N+2=7, total = 7 + 6 = 13 (≈ N+8).
        let w = ShellFrameWave::new_first_paint_slide(5, Instant::now());
        assert_eq!(w.total_ticks, 5 + 2 + WAVE_TAIL_TICKS);
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
}
