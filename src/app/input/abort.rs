//! B6 physical input. Shared capture owns the pointer; app modal outcomes
//! retain pause/clock and queued EXIT authority (native48CAA8→6471A0).

use crate::app::{App, AppState};
use crate::ui::pause_menu::{AbortConfirmAction, resolve_abort_action};
use crate::ui::shell::abort::{AbortButton, abort_layout};
use winit::event::MouseButton;

fn hit(state: &AppState) -> Option<AbortButton> {
    let (shell, size) =
        crate::app::frontend::skirmish_shell_render::current_in_game_shell_layout(state)?;
    let layout = abort_layout(
        state.renderer.gpu.config.width as i32,
        state.renderer.gpu.config.height as i32,
        shell,
        size,
    );
    let (x, y) = state.window_cursor_position();
    layout.hit(x.round() as i32, y.round() as i32)
}

pub(crate) fn cursor_moved(state: &mut AppState) {
    state.match_state.match_presentation.abort_buttons.hovered = hit(state);
}

pub(crate) fn activate(state: &mut AppState, button: AbortButton) {
    let action = match button {
        AbortButton::Leave => AbortConfirmAction::Leave,
        AbortButton::Resume => AbortConfirmAction::Cancel,
    };
    App::apply_in_game_modal_outcome(state, resolve_abort_action(action));
}

pub(crate) fn mouse(state: &mut AppState, button: MouseButton, pressed: bool) {
    if button != MouseButton::Left {
        return;
    }
    let over = hit(state);
    if pressed {
        state
            .match_state
            .match_presentation
            .abort_buttons
            .press(over);
        if over.is_some() {
            App::play_skirmish_shell_generic_click_sound(state);
        }
    } else if let Some(button) = state
        .match_state
        .match_presentation
        .abort_buttons
        .release(over)
    {
        activate(state, button);
    }
}
