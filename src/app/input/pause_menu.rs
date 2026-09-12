//! Offline B5 input. Native callback4F11B0 owns six commands; rendering and
//! hit-testing share the loaded side-art geometry (72FC60,60B000,60B350).

use crate::app::{App, AppState};
use crate::ui::pause_menu::InGameMenuState;
use crate::ui::shell::pause_menu::{PauseMenuButton, PauseMenuButtonState, pause_menu_layout};
use crate::ui::skirmish_shell::SavedSeedMode;
use winit::event::MouseButton;

pub(crate) fn button_states(state: &AppState) -> [PauseMenuButtonState; 6] {
    let interaction = state.match_state.match_presentation.pause_menu_interaction;
    // Native4F17D9..4F1832 applies559C20 eligibility to Load/Delete only.
    // VERA uses its own compatible snapshot headers; Save remains available.
    let has_saves = state.match_state.match_presentation.pause_menu_has_saves;
    PauseMenuButton::ALL.map(|button| PauseMenuButtonState {
        pressed: interaction.is_pressed(button),
        // C5 timer highlighting is separate from pointer hover.
        highlighted: false,
        enabled: has_saves || !matches!(button, PauseMenuButton::Load | PauseMenuButton::Delete),
    })
}

fn hit(state: &AppState) -> Option<PauseMenuButton> {
    let (shell, size) =
        crate::app::frontend::skirmish_shell_render::current_in_game_shell_layout(state)?;
    let layout = pause_menu_layout(
        state.renderer.gpu.config.width as i32,
        state.renderer.gpu.config.height as i32,
        shell,
        size,
    );
    let (x, y) = state.window_cursor_position();
    let states = button_states(state);
    PauseMenuButton::ALL.into_iter().find(|button| {
        states[*button as usize].enabled
            && layout.buttons[*button as usize].contains(x.round() as i32, y.round() as i32)
    })
}

pub(crate) fn cursor_moved(state: &mut AppState) {
    state
        .match_state
        .match_presentation
        .pause_menu_interaction
        .hovered = hit(state);
}

pub(crate) fn mouse(state: &mut AppState, button: MouseButton, pressed: bool) {
    if button != MouseButton::Left {
        return;
    }
    let over = hit(state);
    state
        .match_state
        .match_presentation
        .pause_menu_interaction
        .hovered = over;
    if pressed {
        state
            .match_state
            .match_presentation
            .pause_menu_interaction
            .press(over);
        if over.is_some() {
            // Common type2 button61374B..613771: generic cue at button-down.
            App::play_skirmish_shell_generic_click_sound(state);
        }
    } else {
        let held = state
            .match_state
            .match_presentation
            .pause_menu_interaction
            .release(over);
        if let Some(button) = held {
            activate(state, button);
        }
    }
}

fn activate(state: &mut AppState, button: PauseMenuButton) {
    match button {
        PauseMenuButton::GameControls => {
            App::enter_in_game_menu_state(state, InGameMenuState::Options)
        }
        PauseMenuButton::Resume => App::enter_in_game_menu_state(state, InGameMenuState::Closed),
        PauseMenuButton::Abort => {
            App::enter_in_game_menu_state(state, InGameMenuState::AbortConfirm)
        }
        PauseMenuButton::Load => App::open_saved_game_browser(state, SavedSeedMode::Load),
        PauseMenuButton::Save => App::open_saved_game_browser(state, SavedSeedMode::Save),
        PauseMenuButton::Delete => App::open_saved_game_browser(state, SavedSeedMode::Delete),
    }
}
