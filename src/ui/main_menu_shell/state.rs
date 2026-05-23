//! Initial main-menu shell control identity and hit testing.

use std::time::Instant;

use super::layout::MainMenuShellLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuControlId {
    SinglePlayer0x683,
    WwOnline0x684,
    Network0x578,
    MoviesAndCredits0x686,
    Options0x55c,
    ExitGame0x3ee,
    YuriWebsite0x71b,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuShellAction {
    None,
    SinglePlayer,
    WwOnline,
    Network,
    MoviesAndCredits,
    Options,
    ExitGame,
    YuriWebsite,
}

#[derive(Debug, Clone, Default)]
pub struct MainMenuShellState {
    pub pressed_owner_draw_button: Option<MainMenuControlId>,
    /// Control under the cursor right now, if any. Drives the bottom-left
    /// tooltip status line and the SDBTNANM focus-flash frame selection.
    pub hovered_owner_draw_button: Option<MainMenuControlId>,
    /// When the current hover started, so the renderer can compute the
    /// 1 Hz frame 2 ↔ frame 3 toggle gamemd arms via `SetTimer(hwnd, 0,
    /// 1000, NULL)` in the WM_LBUTTONDOWN/hover-mutator path. Cleared on
    /// unhover; only updated when the hovered button identity changes
    /// (matches gamemd's "do not re-arm if already armed" guard).
    pub hover_started_at: Option<Instant>,
}

pub fn action_for_control(id: MainMenuControlId) -> MainMenuShellAction {
    match id {
        MainMenuControlId::SinglePlayer0x683 => MainMenuShellAction::SinglePlayer,
        MainMenuControlId::WwOnline0x684 => MainMenuShellAction::WwOnline,
        MainMenuControlId::Network0x578 => MainMenuShellAction::Network,
        MainMenuControlId::MoviesAndCredits0x686 => MainMenuShellAction::MoviesAndCredits,
        MainMenuControlId::Options0x55c => MainMenuShellAction::Options,
        MainMenuControlId::ExitGame0x3ee => MainMenuShellAction::ExitGame,
        MainMenuControlId::YuriWebsite0x71b => MainMenuShellAction::YuriWebsite,
    }
}

pub fn return_code_for_action(action: MainMenuShellAction) -> Option<i32> {
    match action {
        MainMenuShellAction::None => None,
        MainMenuShellAction::SinglePlayer => Some(1),
        MainMenuShellAction::WwOnline => Some(2),
        MainMenuShellAction::Network => Some(3),
        MainMenuShellAction::MoviesAndCredits => Some(4),
        MainMenuShellAction::Options => Some(5),
        MainMenuShellAction::ExitGame => Some(6),
        MainMenuShellAction::YuriWebsite => None,
    }
}

pub fn csf_key_for_control(id: MainMenuControlId) -> &'static str {
    match id {
        MainMenuControlId::SinglePlayer0x683 => "GUI:SinglePlayer",
        MainMenuControlId::WwOnline0x684 => "GUI:WWOnline",
        MainMenuControlId::Network0x578 => "GUI:Network",
        MainMenuControlId::MoviesAndCredits0x686 => "GUI:MoviesAndCredits",
        MainMenuControlId::Options0x55c => "GUI:Options",
        MainMenuControlId::ExitGame0x3ee => "GUI:ExitGame",
        MainMenuControlId::YuriWebsite0x71b => "TXT_YURI_WEBSITE",
    }
}

/// CSF key for the bottom-left hover-tooltip status line, looked up per
/// control when the cursor is over a main-menu owner-draw button.
pub fn tooltip_csf_key_for_control(id: MainMenuControlId) -> &'static str {
    match id {
        MainMenuControlId::SinglePlayer0x683 => "STT:MainButtonSinglePlayer",
        MainMenuControlId::WwOnline0x684 => "STT:MainButtonWWOnline",
        MainMenuControlId::Network0x578 => "STT:MainButtonNetwork",
        MainMenuControlId::MoviesAndCredits0x686 => "STT:MainButtonMovies",
        MainMenuControlId::Options0x55c => "STT:MainButtonOptions",
        MainMenuControlId::ExitGame0x3ee => "STT:MainButtonExitGamemd",
        MainMenuControlId::YuriWebsite0x71b => "STT:MainButtonYuriWebSite",
    }
}

pub fn hit_test_owner_draw_button(
    layout: &MainMenuShellLayout,
    x: i32,
    y: i32,
) -> Option<MainMenuControlId> {
    layout
        .buttons
        .iter()
        .find(|button| button.rect.contains(x, y))
        .map(|button| button.id)
}

pub fn mouse_down(state: &mut MainMenuShellState, layout: &MainMenuShellLayout, x: i32, y: i32) {
    state.pressed_owner_draw_button = hit_test_owner_draw_button(layout, x, y);
}

/// Update the hover tracking from a cursor position. Called per mouse move.
///
/// Only resets `hover_started_at` when the hovered control identity
/// transitions — cursor moves within the same button do not re-arm the
/// timer (matches gamemd's `piVar17[0x31] == '\0'` guard before SetTimer).
pub fn mouse_move(state: &mut MainMenuShellState, layout: &MainMenuShellLayout, x: i32, y: i32) {
    let new_hover = hit_test_owner_draw_button(layout, x, y);
    if state.hovered_owner_draw_button != new_hover {
        state.hovered_owner_draw_button = new_hover;
        state.hover_started_at = new_hover.map(|_| Instant::now());
    }
}

pub fn mouse_up(
    state: &mut MainMenuShellState,
    layout: &MainMenuShellLayout,
    x: i32,
    y: i32,
) -> MainMenuShellAction {
    let released = hit_test_owner_draw_button(layout, x, y);
    let pressed = state.pressed_owner_draw_button.take();
    if pressed.is_some() && pressed == released {
        released
            .map(action_for_control)
            .unwrap_or(MainMenuShellAction::None)
    } else {
        MainMenuShellAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::main_menu_shell::compute_layout;

    #[test]
    fn button_actions_preserve_return_codes() {
        assert_eq!(
            return_code_for_action(action_for_control(MainMenuControlId::SinglePlayer0x683)),
            Some(1)
        );
        assert_eq!(
            return_code_for_action(action_for_control(MainMenuControlId::WwOnline0x684)),
            Some(2)
        );
        assert_eq!(
            return_code_for_action(action_for_control(MainMenuControlId::Network0x578)),
            Some(3)
        );
        assert_eq!(
            return_code_for_action(action_for_control(MainMenuControlId::MoviesAndCredits0x686)),
            Some(4)
        );
        assert_eq!(
            return_code_for_action(action_for_control(MainMenuControlId::Options0x55c)),
            Some(5)
        );
        assert_eq!(
            return_code_for_action(action_for_control(MainMenuControlId::ExitGame0x3ee)),
            Some(6)
        );
    }

    #[test]
    fn hit_test_uses_owner_draw_button_identity() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test_owner_draw_button(&layout, 639, 204),
            Some(MainMenuControlId::SinglePlayer0x683)
        );
        assert_eq!(
            hit_test_owner_draw_button(&layout, 639, 537),
            Some(MainMenuControlId::ExitGame0x3ee)
        );
        assert_eq!(hit_test_owner_draw_button(&layout, 800, 203), None);
    }

    #[test]
    fn hit_test_uses_unscaled_large_screen_button_rects() {
        let layout = compute_layout(1024, 768);
        assert_eq!(
            hit_test_owner_draw_button(&layout, 760, 300),
            Some(MainMenuControlId::SinglePlayer0x683)
        );
        assert_eq!(hit_test_owner_draw_button(&layout, 809, 255), None);
    }

    #[test]
    fn mouse_release_must_match_pressed_button() {
        let layout = compute_layout(800, 600);
        let mut state = MainMenuShellState::default();
        mouse_down(&mut state, &layout, 639, 204);
        assert_eq!(
            mouse_up(&mut state, &layout, 639, 247),
            MainMenuShellAction::None
        );
        mouse_down(&mut state, &layout, 639, 204);
        assert_eq!(
            mouse_up(&mut state, &layout, 639, 204),
            MainMenuShellAction::SinglePlayer
        );
    }
}
