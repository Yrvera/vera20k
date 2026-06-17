//! Generic shell first-paint slide driver (menu / single-player / skirmish).
//!
//! The original plays a controls-reveal animation on the *first paint of every
//! allow-listed shell dialog* — not a menu->skirmish edge transition and not a
//! whole-screen crossfade. Each owner-draw control's chrome-SHP frame index
//! advances on a staggered 30 ms-per-frame schedule; controls are never
//! repositioned.
//!
//! The render-agnostic data + schedule live in [`crate::ui::shell::slide`] (the
//! dialog-id allow-list, per-dialog animated-slot count, and the [`ShellFrameWave`]
//! frame sweep). This module is the app/render glue: it maps the currently-showing
//! screen to a shell dialog, (re)starts/advances the wave on entry edges, plays
//! the slide-in start cue, and dispatches the per-frame shell repaint while the
//! wave is live. The slide-in start cue is `GUIMoveInSound` (stock `MenuSlideIn`);
//! the stock-empty end cue (`ShellButtonSlideSound`) stays silent.

use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::ui::shell::descriptor::DialogId;

// Re-export the render-agnostic schedule types from the shared substrate so the
// shell renderers (and the `AppState` field) keep their existing import paths.
pub(crate) use crate::ui::shell::slide::{ButtonGroup, ShellFrameWave};

/// Which shell dialog a first-paint slide belongs to. Every allow-listed shell
/// dialog slides on its own first paint; this identifies the one currently
/// showing so the trigger can detect entry edges and look up the control count.
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
    /// The Win32 dialog resource id this shell maps to. The slide's eligibility
    /// and animated-slot count are looked up from this id in the data-driven
    /// `slide` table (no hardcoded per-kind counts here).
    pub(crate) fn dialog_id(self) -> DialogId {
        DialogId(match self {
            ShellSlideKind::MainMenu => 0x00E2,
            ShellSlideKind::SinglePlayer => 0x0100,
            ShellSlideKind::Skirmish => 0x0102,
        })
    }

    /// Number of animated owner-draw button slots, which sets the stagger length.
    /// Sourced from the data-driven `slide` table; every rendered shell has an
    /// entry, so a miss is a programming error.
    fn slot_count(self) -> u32 {
        crate::ui::shell::slide::slot_count_for(self.dialog_id())
            .expect("rendered shell dialog must have a slide slot count")
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
/// slide. The candidate is gated through the data-driven slide allow-list, so a
/// dialog only slides when its id is eligible. Returns `None` off the main menu
/// screen.
pub(crate) fn current_shell_slide_target(state: &AppState) -> Option<ShellSlideKind> {
    use crate::ui::game_screen::GameScreen;
    if state.screen != GameScreen::MainMenu {
        return None;
    }
    let candidate = if state.main_menu_show_native_skirmish_shell || state.dev_skirmish_shell_enabled
    {
        ShellSlideKind::Skirmish
    } else if state.main_menu_show_single_player_shell {
        ShellSlideKind::SinglePlayer
    } else if !state.main_menu_shell_failed && !state.main_menu_show_skirmish_setup {
        ShellSlideKind::MainMenu
    } else {
        return None;
    };
    crate::ui::shell::slide::is_slide_eligible(candidate.dialog_id()).then_some(candidate)
}

/// Detect entry into an allow-listed shell dialog and (re)start its first-paint
/// slide. Run once per frame: when the showing shell changes to an eligible one
/// (launch, navigation, return-from-game) a fresh wave begins and the slide-in
/// start cue plays; leaving all shells cancels any in-flight wave. Mirrors the
/// original, where each dialog is re-created on entry and slides on its own first
/// WM_PAINT, with `GUIMoveInSound` played at the start of that slide.
pub(crate) fn update_shell_first_paint_slide_trigger(state: &mut AppState) {
    let target = current_shell_slide_target(state);
    if target == state.shell_slide_active_shell {
        return;
    }
    state.shell_slide_active_shell = target;
    match target {
        Some(kind) => {
            start_shell_first_paint_slide(state, kind.slot_count());
            // The slide-in trigger plays GUIMoveInSound (stock MenuSlideIn) at
            // the start of the controls-reveal animation, on each shell entry.
            crate::app::App::play_shell_slide_in_sound(state);
        }
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
        // The slide completion edge kicks off the Skirmish right-panel statics'
        // character reveal. Start it here, on the same edge that clears the
        // slide, using the strings the renderer will draw.
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
    fn shell_kinds_map_to_their_dialog_ids() {
        assert_eq!(ShellSlideKind::MainMenu.dialog_id(), DialogId(0x00E2));
        assert_eq!(ShellSlideKind::SinglePlayer.dialog_id(), DialogId(0x0100));
        assert_eq!(ShellSlideKind::Skirmish.dialog_id(), DialogId(0x0102));
    }

    #[test]
    fn shell_kinds_resolve_data_driven_slot_counts() {
        assert_eq!(ShellSlideKind::MainMenu.slot_count(), 6);
        assert_eq!(ShellSlideKind::SinglePlayer.slot_count(), 4);
        assert_eq!(ShellSlideKind::Skirmish.slot_count(), 3);
    }
}
