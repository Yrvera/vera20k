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

/// Proof that the terminal main-menu slide frame was encoded. The app consumes
/// it only after that frame is submitted and presented.
#[derive(Debug)]
pub(crate) struct MainMenuEntryPresentReceipt {
    _private: (),
}

pub(crate) enum ShellFirstPaintRenderResult {
    NotRendered,
    Rendered {
        main_menu_entry_receipt: Option<MainMenuEntryPresentReceipt>,
    },
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellEntryEffect {
    Unchanged,
    LeftShells,
    Started(ShellSlideKind),
}

#[derive(Debug)]
enum ShellWaveCompletion {
    MainMenu(MainMenuEntryPresentReceipt),
    SinglePlayer,
    Skirmish,
}

/// Render-agnostic reducer for dialog-instance, entry-wave, and title state.
///
/// Production route mutations and the frame driver both use this reducer, so a
/// dialog destroyed between paints cannot be hidden from an edge detector that
/// only remembers the last rendered target.
struct ShellLifecycleReducer<'a> {
    active_shell: &'a mut Option<ShellSlideKind>,
    first_paint_slide: &'a mut Option<ShellFrameWave>,
    title_reveal: &'a mut crate::ui::shell::static_reveal::Kind1StaticReveal,
}

impl<'a> ShellLifecycleReducer<'a> {
    fn from_state(state: &'a mut AppState) -> Self {
        Self {
            active_shell: &mut state.shell_slide_active_shell,
            first_paint_slide: &mut state.shell_first_paint_slide,
            title_reveal: &mut state.main_menu_shell_state.title_reveal,
        }
    }

    fn invalidate_main_menu_dialog_instance(&mut self) {
        self.title_reveal.reset_waiting();
        *self.active_shell = None;
        *self.first_paint_slide = None;
    }

    fn observe_target(&mut self, target: Option<ShellSlideKind>, now: Instant) -> ShellEntryEffect {
        if target == *self.active_shell {
            return ShellEntryEffect::Unchanged;
        }
        if target == Some(ShellSlideKind::MainMenu)
            || *self.active_shell == Some(ShellSlideKind::MainMenu)
        {
            self.title_reveal.reset_waiting();
        }
        *self.active_shell = target;
        match target {
            Some(kind) => {
                *self.first_paint_slide = Some(ShellFrameWave::new_first_paint_slide(
                    kind.slot_count(),
                    now,
                ));
                ShellEntryEffect::Started(kind)
            }
            None => {
                *self.first_paint_slide = None;
                ShellEntryEffect::LeftShells
            }
        }
    }

    fn advance_wave(&mut self, now: Instant) {
        if let Some(wave) = self.first_paint_slide.as_mut() {
            wave.advance(now);
        }
    }

    fn finish_completed_wave(&mut self, kind: ShellSlideKind) -> Option<ShellWaveCompletion> {
        if !self
            .first_paint_slide
            .as_ref()
            .is_some_and(ShellFrameWave::is_complete)
        {
            return None;
        }
        *self.first_paint_slide = None;
        Some(match kind {
            ShellSlideKind::MainMenu => {
                ShellWaveCompletion::MainMenu(MainMenuEntryPresentReceipt { _private: () })
            }
            ShellSlideKind::SinglePlayer => ShellWaveCompletion::SinglePlayer,
            ShellSlideKind::Skirmish => ShellWaveCompletion::Skirmish,
        })
    }

    fn record_main_menu_entry_presented(
        &mut self,
        _receipt: MainMenuEntryPresentReceipt,
        title: &str,
        now: Instant,
    ) -> bool {
        if *self.active_shell != Some(ShellSlideKind::MainMenu) || self.first_paint_slide.is_some()
        {
            return false;
        }
        self.title_reveal.start(title, now)
    }
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

/// Invalidate the destroyed/recreated 0xE2 dialog instance at an actual route
/// boundary, even when the destination never reaches a paint.
///
/// `shell_slide_active_shell` remembers the last target observed by the frame
/// driver. Clearing it here makes a collapsed 0xE2 -> 0x100 -> Back round trip
/// produce a fresh 0xE2 entry edge instead of inheriting the old terminal title.
pub(crate) fn invalidate_main_menu_dialog_instance(state: &mut AppState) {
    ShellLifecycleReducer::from_state(state).invalidate_main_menu_dialog_instance();
}

/// Deliver the kind-1 timer only while the bare 0xE2 dialog owns steady paint.
/// Waiting state is inert, so the terminal slide frame cannot begin the title
/// before its own successful presentation.
pub(crate) fn poll_main_menu_title_reveal(state: &mut AppState) {
    if current_shell_slide_target(state) == Some(ShellSlideKind::MainMenu)
        && state.shell_first_paint_slide.is_none()
    {
        state
            .main_menu_shell_state
            .title_reveal
            .poll_timer(Instant::now());
    }
}

/// Apply the SHOW-completion edge after the terminal slide frame presents.
pub(crate) fn record_main_menu_entry_presented(
    state: &mut AppState,
    receipt: MainMenuEntryPresentReceipt,
) -> bool {
    if current_shell_slide_target(state) != Some(ShellSlideKind::MainMenu) {
        return false;
    }
    let title = crate::app_main_menu_shell_render::main_menu_title_text(state).to_owned();
    ShellLifecycleReducer::from_state(state).record_main_menu_entry_presented(
        receipt,
        &title,
        Instant::now(),
    )
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
    let candidate =
        if state.main_menu_show_native_skirmish_shell || state.dev_skirmish_shell_enabled {
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
    let effect = ShellLifecycleReducer::from_state(state).observe_target(target, Instant::now());
    if let ShellEntryEffect::Started(_) = effect {
        // The slide-in trigger plays GUIMoveInSound (stock MenuSlideIn) at
        // the start of the controls-reveal animation, on each shell entry.
        crate::app::App::play_shell_slide_in_sound(state);
    }
}

/// Render the currently-showing shell while its first-paint slide is live, then
/// advance/complete the wave. Returns `Rendered` when it owned the frame. The shell
/// renderer reads `state.shell_first_paint_slide` and swaps each owner-draw
/// button's SDBTNANM frame index — controls are never repositioned, and the rest
/// of the shell paints exactly as it does steady-state.
pub(crate) fn render_shell_first_paint_slide(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> Result<ShellFirstPaintRenderResult> {
    if state.shell_first_paint_slide.is_none() {
        return Ok(ShellFirstPaintRenderResult::NotRendered);
    }
    let Some(kind) = current_shell_slide_target(state) else {
        // No eligible shell is showing; drop the stale wave and let the normal
        // dispatch paint this frame.
        state.shell_first_paint_slide = None;
        return Ok(ShellFirstPaintRenderResult::NotRendered);
    };

    ShellLifecycleReducer::from_state(state).advance_wave(Instant::now());

    let rendered = match kind {
        ShellSlideKind::Skirmish => {
            crate::app::App::ensure_skirmish_shell_chrome(state);
            if state.skirmish_shell_chrome.is_none() {
                log::warn!("Skirmish shell chrome unavailable; cancelling first-paint slide");
                state.shell_first_paint_slide = None;
                return Ok(ShellFirstPaintRenderResult::NotRendered);
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
        ShellSlideKind::MainMenu => {
            // The wave owns a shared transition target, not the native steady
            // primary-surface boundary finalized by render_main_menu_shell.
            let depth = state.depth_view.clone();
            matches!(
                crate::app_main_menu_shell_render::render_main_menu_shell_to_target(
                    state,
                    encoder,
                    crate::render::shell_transition_pass::ShellRenderTarget {
                        color: target,
                        depth: &depth,
                    },
                )?,
                crate::app_main_menu_shell_render::MainMenuShellRenderResult::Rendered { .. }
            )
        }
    };

    if !rendered {
        // Shell fell back (assets missing): abandon the slide so the normal
        // dispatch can render the fallback path with its egui overlays.
        state.shell_first_paint_slide = None;
        return Ok(ShellFirstPaintRenderResult::NotRendered);
    }

    let completion = ShellLifecycleReducer::from_state(state).finish_completed_wave(kind);
    let mut main_menu_entry_receipt = None;
    match completion {
        // The slide completion edge kicks off the Skirmish right-panel statics'
        // character reveal. Start it here, on the same edge that clears the
        // slide, using the strings the renderer will draw.
        Some(ShellWaveCompletion::Skirmish) => {
            let now = Instant::now();
            let (title, game_type, map_label) =
                crate::app_skirmish_shell_render::skirmish_right_panel_label_strings(state);
            state
                .skirmish_shell_state
                .start_right_panel_static_reveals(&title, &game_type, &map_label, now);
        }
        Some(ShellWaveCompletion::MainMenu(receipt)) => {
            main_menu_entry_receipt = Some(receipt);
        }
        Some(ShellWaveCompletion::SinglePlayer) | None => {}
    }

    Ok(ShellFirstPaintRenderResult::Rendered {
        main_menu_entry_receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::static_reveal::{Kind1PaintWindow, Kind1StaticReveal};
    use std::time::Duration;

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

    #[test]
    fn collapsed_e2_to_100_back_before_paint_rearms_title_and_entry_wave() {
        let start = Instant::now();
        let mut title_reveal = Kind1StaticReveal::default();
        assert!(title_reveal.start("Main Menu", start));
        for count in 1..=17 {
            let Kind1PaintWindow::Due { window, receipt } = title_reveal.paint_window() else {
                panic!("expected dirty title paint {count}");
            };
            assert_eq!(window.count, count);
            assert!(title_reveal.record_presented(receipt));
            if count < 17 {
                assert!(
                    title_reveal.poll_timer(start + Duration::from_millis(30 * u64::from(count)))
                );
            }
        }
        assert!(title_reveal.is_terminal_persistent());

        let mut active_shell = Some(ShellSlideKind::MainMenu);
        let mut first_paint_slide = None;
        let mut slide_sound_edges = Vec::new();

        // Open 0x100: 0xE2 is destroyed, but no 0x100 frame is allowed to run.
        ShellLifecycleReducer {
            active_shell: &mut active_shell,
            first_paint_slide: &mut first_paint_slide,
            title_reveal: &mut title_reveal,
        }
        .invalidate_main_menu_dialog_instance();
        assert_eq!(title_reveal.paint_window(), Kind1PaintWindow::Hidden);
        assert_eq!(active_shell, None);
        assert!(first_paint_slide.is_none());
        assert!(slide_sound_edges.is_empty());

        // Queued Back destroys 0x100 and recreates 0xE2 before the frame driver.
        ShellLifecycleReducer {
            active_shell: &mut active_shell,
            first_paint_slide: &mut first_paint_slide,
            title_reveal: &mut title_reveal,
        }
        .invalidate_main_menu_dialog_instance();
        assert!(first_paint_slide.is_none());
        assert!(slide_sound_edges.is_empty());

        // The next production frame observes only the recreated 0xE2. No 0x100
        // wave or sound ever existed; exactly one fresh 0xE2 entry edge does.
        let effect = ShellLifecycleReducer {
            active_shell: &mut active_shell,
            first_paint_slide: &mut first_paint_slide,
            title_reveal: &mut title_reveal,
        }
        .observe_target(Some(ShellSlideKind::MainMenu), start);
        if let ShellEntryEffect::Started(kind) = effect {
            slide_sound_edges.push(kind);
        }
        assert_eq!(slide_sound_edges, [ShellSlideKind::MainMenu]);
        assert_eq!(title_reveal.paint_window(), Kind1PaintWindow::Hidden);
        assert!(first_paint_slide.is_some());
        assert_eq!(active_shell, Some(ShellSlideKind::MainMenu));

        // Drive the same reducer used by the production renderer through all 15
        // 0xE2 wave ticks, then commit its opaque receipt after presentation.
        let mut entry_receipt = None;
        for tick in 1..=15 {
            let now = start + Duration::from_millis(30 * tick);
            let mut reducer = ShellLifecycleReducer {
                active_shell: &mut active_shell,
                first_paint_slide: &mut first_paint_slide,
                title_reveal: &mut title_reveal,
            };
            reducer.advance_wave(now);
            match reducer.finish_completed_wave(ShellSlideKind::MainMenu) {
                Some(ShellWaveCompletion::MainMenu(receipt)) => {
                    assert_eq!(tick, 15);
                    entry_receipt = Some(receipt);
                }
                Some(_) => panic!("wrong shell completed"),
                None => assert!(tick < 15),
            }
        }
        assert!(first_paint_slide.is_none());
        assert_eq!(title_reveal.paint_window(), Kind1PaintWindow::Hidden);
        assert!(
            ShellLifecycleReducer {
                active_shell: &mut active_shell,
                first_paint_slide: &mut first_paint_slide,
                title_reveal: &mut title_reveal,
            }
            .record_main_menu_entry_presented(
                entry_receipt.expect("terminal 0xE2 receipt"),
                "Main Menu",
                start + Duration::from_millis(450),
            )
        );
        let Kind1PaintWindow::Due { window, .. } = title_reveal.paint_window() else {
            panic!("recreated title did not begin a fresh reveal");
        };
        assert_eq!(window.count, 1);
        assert_eq!(slide_sound_edges, [ShellSlideKind::MainMenu]);
    }
}
