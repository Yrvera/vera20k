//! App-level bridge for the temporary main-menu -> Skirmish shell shortcut.
//!
//! This module is explicitly bridge/DRIFT code. Verified native YR flow enters
//! Skirmish through an intermediate shell path; this whole-screen compositor
//! only hides the current Rust hard snap until that path exists.

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::app::AppState;
use crate::render::shell_transition_pass::ShellTransitionPass;

pub(crate) const SHELL_BRIDGE_FRAME_MS: u32 = 30;
pub(crate) const SHELL_BRIDGE_FRAME_COUNT: u32 = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellBridgeTarget {
    Skirmish,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellBridgeTransition {
    #[allow(dead_code)]
    pub(crate) started_at: Instant,
    pub(crate) last_step_at: Instant,
    pub(crate) frame_index: u32,
    pub(crate) frame_count: u32,
    pub(crate) frame_ms: u32,
    pub(crate) target: ShellBridgeTarget,
    completion_applied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResizeTransitionResolution {
    ReturnToMainMenu,
    CompleteToSkirmish,
    NoTransition,
}

impl ShellBridgeTransition {
    pub(crate) fn new_main_menu_to_skirmish(started_at: Instant) -> Self {
        Self {
            started_at,
            last_step_at: started_at,
            frame_index: 0,
            frame_count: SHELL_BRIDGE_FRAME_COUNT,
            frame_ms: SHELL_BRIDGE_FRAME_MS,
            target: ShellBridgeTarget::Skirmish,
            completion_applied: false,
        }
    }

    pub(crate) fn progress(&self) -> f32 {
        if self.frame_count == 0 {
            return 1.0;
        }
        (self.frame_index.min(self.frame_count) as f32 / self.frame_count as f32).clamp(0.0, 1.0)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.frame_index >= self.frame_count
    }

    pub(crate) fn advance_to(&mut self, now: Instant) {
        let frame_duration = Duration::from_millis(u64::from(self.frame_ms.max(1)));
        while self.frame_index < self.frame_count
            && now.duration_since(self.last_step_at) >= frame_duration
        {
            self.frame_index += 1;
            self.last_step_at += frame_duration;
        }
    }

    fn mark_completion_applied(&mut self) -> bool {
        if self.completion_applied {
            return false;
        }
        self.completion_applied = true;
        true
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

pub(crate) fn transition_blocks_shell_input(transition: Option<&ShellBridgeTransition>) -> bool {
    transition.is_some()
}

pub(crate) fn resolve_resize(state: &mut AppState) -> ResizeTransitionResolution {
    let Some(transition) = state.main_menu_to_skirmish_transition.take() else {
        return ResizeTransitionResolution::NoTransition;
    };
    state.shell_transition_pass = None;
    if transition.progress() < 0.5 {
        state.main_menu_show_native_skirmish_shell = false;
        ResizeTransitionResolution::ReturnToMainMenu
    } else {
        state.main_menu_show_native_skirmish_shell = true;
        ResizeTransitionResolution::CompleteToSkirmish
    }
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
        log::warn!("Skirmish shell chrome unavailable; cancelling shell bridge transition");
        state.main_menu_to_skirmish_transition = None;
        state.shell_transition_pass = None;
        return Ok(false);
    }

    let width = state.gpu.config.width.max(1);
    let height = state.gpu.config.height.max(1);
    let pass = match state.shell_transition_pass.take() {
        Some(pass) if pass.size_matches(width, height) => pass,
        _ => ShellTransitionPass::new(&state.gpu, width, height),
    };

    match crate::app_main_menu_shell_render::render_main_menu_shell_to_target(
        state,
        encoder,
        pass.source_render_target(),
    )? {
        crate::app_main_menu_shell_render::MainMenuShellRenderResult::Rendered => {}
        crate::app_main_menu_shell_render::MainMenuShellRenderResult::Fallback => {
            state.main_menu_to_skirmish_transition = None;
            state.shell_transition_pass = Some(pass);
            return Ok(false);
        }
    }

    crate::app_skirmish_shell_render::render_skirmish_shell_to_target(
        state,
        encoder,
        pass.destination_render_target(),
        crate::app_skirmish_shell_render::ShellRenderMode::TransitionPreview,
    )?;

    if let Some(transition) = state.main_menu_to_skirmish_transition.as_mut() {
        transition.advance_to(Instant::now());
    }
    let progress = state
        .main_menu_to_skirmish_transition
        .as_ref()
        .map_or(1.0, ShellBridgeTransition::progress);
    pass.draw(&state.gpu, encoder, target, progress);

    if state
        .main_menu_to_skirmish_transition
        .as_ref()
        .is_some_and(ShellBridgeTransition::is_complete)
    {
        complete_main_menu_to_skirmish(state);
    } else {
        state.shell_transition_pass = Some(pass);
    }

    Ok(true)
}

fn complete_main_menu_to_skirmish(state: &mut AppState) {
    let Some(mut transition) = state.main_menu_to_skirmish_transition.take() else {
        return;
    };
    if !transition.mark_completion_applied() {
        return;
    }
    if transition.target == ShellBridgeTarget::Skirmish {
        state.main_menu_show_native_skirmish_shell = true;
        state.shell_transition_pass = None;
        crate::app::App::ensure_skirmish_shell_chrome(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_one_frame_per_thirty_ms_and_clamps_at_completion() {
        let start = Instant::now();
        let mut transition = ShellBridgeTransition::new_main_menu_to_skirmish(start);

        transition.advance_to(start + Duration::from_millis(29));
        assert_eq!(transition.frame_index, 0);

        transition.advance_to(start + Duration::from_millis(30));
        assert_eq!(transition.frame_index, 1);

        transition.advance_to(start + Duration::from_millis(30 * 100));
        assert_eq!(transition.frame_index, SHELL_BRIDGE_FRAME_COUNT);
        assert!(transition.is_complete());
    }

    #[test]
    fn progress_is_monotonic_and_reaches_one_on_final_frame() {
        let start = Instant::now();
        let mut transition = ShellBridgeTransition::new_main_menu_to_skirmish(start);
        let mut previous = transition.progress();
        for frame in 1..=SHELL_BRIDGE_FRAME_COUNT {
            transition.frame_index = frame;
            let progress = transition.progress();
            assert!(progress >= previous);
            previous = progress;
        }
        assert_eq!(transition.progress(), 1.0);
    }

    #[test]
    fn completion_state_can_only_be_applied_once() {
        let start = Instant::now();
        let mut transition = ShellBridgeTransition::new_main_menu_to_skirmish(start);

        assert!(transition.mark_completion_applied());
        assert!(!transition.mark_completion_applied());
    }

    #[test]
    fn input_is_blocked_only_while_bridge_transition_is_active() {
        let start = Instant::now();
        let transition = ShellBridgeTransition::new_main_menu_to_skirmish(start);

        assert!(transition_blocks_shell_input(Some(&transition)));
        assert!(!transition_blocks_shell_input(None));
    }

    #[test]
    fn resize_policy_returns_to_main_menu_before_halfway_and_completes_after() {
        let start = Instant::now();
        let mut transition = ShellBridgeTransition::new_main_menu_to_skirmish(start);
        transition.frame_index = (SHELL_BRIDGE_FRAME_COUNT / 2).saturating_sub(1);
        assert!(transition.progress() < 0.5);

        transition.frame_index = SHELL_BRIDGE_FRAME_COUNT / 2;
        assert!(transition.progress() >= 0.5);
    }
}
