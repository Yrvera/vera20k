//! Dialog 0x100 Single Player shell control identity and hit testing.

use std::time::Instant;

use super::layout::SinglePlayerShellLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinglePlayerControlId {
    NewCampaign0x688,
    LoadSavedGame0x689,
    Skirmish0x579,
    MainMenu0x686,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinglePlayerShellAction {
    None,
    NewCampaign,
    LoadSavedGame,
    Skirmish,
    MainMenu,
}

#[derive(Debug, Clone, Default)]
pub struct SinglePlayerShellState {
    pub pressed_owner_draw_button: Option<SinglePlayerControlId>,
    pub hovered_owner_draw_button: Option<SinglePlayerControlId>,
    pub hover_started_at: Option<Instant>,
    pub load_saved_game_enabled: bool,
}

pub fn action_for_control(id: SinglePlayerControlId) -> SinglePlayerShellAction {
    match id {
        SinglePlayerControlId::NewCampaign0x688 => SinglePlayerShellAction::NewCampaign,
        SinglePlayerControlId::LoadSavedGame0x689 => SinglePlayerShellAction::LoadSavedGame,
        SinglePlayerControlId::Skirmish0x579 => SinglePlayerShellAction::Skirmish,
        SinglePlayerControlId::MainMenu0x686 => SinglePlayerShellAction::MainMenu,
    }
}

pub fn return_code_for_action(action: SinglePlayerShellAction) -> Option<i32> {
    match action {
        SinglePlayerShellAction::None => None,
        SinglePlayerShellAction::NewCampaign => Some(8),
        SinglePlayerShellAction::LoadSavedGame => Some(9),
        SinglePlayerShellAction::Skirmish => Some(0x0B),
        SinglePlayerShellAction::MainMenu => Some(0x12),
    }
}

pub fn csf_key_for_control(id: SinglePlayerControlId) -> &'static str {
    match id {
        SinglePlayerControlId::NewCampaign0x688 => "GUI:NewCampaign",
        SinglePlayerControlId::LoadSavedGame0x689 => "GUI:LoadSavedGame",
        SinglePlayerControlId::Skirmish0x579 => "GUI:Skirmish",
        SinglePlayerControlId::MainMenu0x686 => "GUI:MainMenu",
    }
}

pub fn hit_test_owner_draw_button(
    layout: &SinglePlayerShellLayout,
    x: i32,
    y: i32,
) -> Option<SinglePlayerControlId> {
    layout
        .buttons
        .iter()
        .find(|button| button.rect.contains(x, y))
        .map(|button| button.id)
}

pub fn mouse_down(
    state: &mut SinglePlayerShellState,
    layout: &SinglePlayerShellLayout,
    x: i32,
    y: i32,
) {
    let hit = hit_test_owner_draw_button(layout, x, y);
    state.pressed_owner_draw_button = match hit {
        Some(SinglePlayerControlId::LoadSavedGame0x689) if !state.load_saved_game_enabled => None,
        other => other,
    };
}

pub fn mouse_move(
    state: &mut SinglePlayerShellState,
    layout: &SinglePlayerShellLayout,
    x: i32,
    y: i32,
) {
    let new_hover = hit_test_owner_draw_button(layout, x, y);
    if state.hovered_owner_draw_button != new_hover {
        state.hovered_owner_draw_button = new_hover;
        state.hover_started_at = new_hover.map(|_| Instant::now());
    }
}

pub fn mouse_up(
    state: &mut SinglePlayerShellState,
    layout: &SinglePlayerShellLayout,
    x: i32,
    y: i32,
) -> SinglePlayerShellAction {
    let released = hit_test_owner_draw_button(layout, x, y);
    let pressed = state.pressed_owner_draw_button.take();
    if pressed.is_some() && pressed == released {
        let id = released.expect("pressed/released checked above");
        if id == SinglePlayerControlId::LoadSavedGame0x689 && !state.load_saved_game_enabled {
            SinglePlayerShellAction::None
        } else {
            action_for_control(id)
        }
    } else {
        SinglePlayerShellAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::single_player_shell::compute_layout;

    #[test]
    fn command_results_match_dialog_proc_0x52d640() {
        assert_eq!(
            return_code_for_action(action_for_control(SinglePlayerControlId::NewCampaign0x688)),
            Some(8)
        );
        assert_eq!(
            return_code_for_action(action_for_control(
                SinglePlayerControlId::LoadSavedGame0x689
            )),
            Some(9)
        );
        assert_eq!(
            return_code_for_action(action_for_control(SinglePlayerControlId::Skirmish0x579)),
            Some(0x0B)
        );
        assert_eq!(
            return_code_for_action(action_for_control(SinglePlayerControlId::MainMenu0x686)),
            Some(0x12)
        );
    }

    #[test]
    fn hit_test_uses_dialog_0x100_control_identity() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test_owner_draw_button(&layout, 639, 204),
            Some(SinglePlayerControlId::NewCampaign0x688)
        );
        assert_eq!(
            hit_test_owner_draw_button(&layout, 639, 290),
            Some(SinglePlayerControlId::Skirmish0x579)
        );
        assert_eq!(
            hit_test_owner_draw_button(&layout, 639, 540),
            Some(SinglePlayerControlId::MainMenu0x686)
        );
    }

    #[test]
    fn disabled_load_saved_game_does_not_emit_result_9() {
        let layout = compute_layout(800, 600);
        let mut state = SinglePlayerShellState::default();
        mouse_down(&mut state, &layout, 639, 248);
        assert_eq!(
            mouse_up(&mut state, &layout, 639, 248),
            SinglePlayerShellAction::None
        );

        state.load_saved_game_enabled = true;
        mouse_down(&mut state, &layout, 639, 248);
        assert_eq!(
            mouse_up(&mut state, &layout, 639, 248),
            SinglePlayerShellAction::LoadSavedGame
        );
    }
}
