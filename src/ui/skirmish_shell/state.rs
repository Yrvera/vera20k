//! Skirmish shell state and hit testing.

use crate::app_init::MapMenuEntry;
use crate::ui::main_menu::{SkirmishCountry, SkirmishSettings, StartPosition};

use super::layout::{ColorComboId, RectPx, SkirmishShellLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerDrawButton {
    StartGame0x617,
    ChooseMap0x5aa,
    Back0x5c0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishShellAction {
    None,
    StartGame,
    BackOrExit,
    ChooseMap,
    SelectColor(ColorComboId),
    SelectMap(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishShellOpponent {
    pub enabled: bool,
    pub country: SkirmishCountry,
    pub color_index: usize,
    pub start_position: StartPosition,
    pub team: i32,
}

#[derive(Debug, Clone)]
pub struct SkirmishShellState {
    pub selected_map_idx: usize,
    pub player_country: SkirmishCountry,
    pub player_color_index: usize,
    pub player_start_position: StartPosition,
    pub starting_credits: i32,
    pub short_game: bool,
    pub zoom_enabled: bool,
    pub opponents: Vec<SkirmishShellOpponent>,
    pub pressed_owner_draw_button: Option<OwnerDrawButton>,
}

impl Default for SkirmishShellState {
    fn default() -> Self {
        let settings = SkirmishSettings::default();
        Self {
            selected_map_idx: settings.selected_map_idx,
            player_country: settings.player_country,
            player_color_index: 0,
            player_start_position: settings.start_position,
            starting_credits: settings.starting_credits,
            short_game: settings.short_game,
            zoom_enabled: settings.zoom_enabled,
            opponents: vec![SkirmishShellOpponent {
                enabled: true,
                country: settings.ai_country,
                color_index: 5,
                start_position: StartPosition::Auto,
                team: 0,
            }],
            pressed_owner_draw_button: None,
        }
    }
}

pub fn launch_settings(state: &SkirmishShellState) -> SkirmishSettings {
    let ai_country = state
        .opponents
        .iter()
        .find(|opponent| opponent.enabled)
        .map(|opponent| opponent.country)
        .unwrap_or(SkirmishCountry::Russia);

    SkirmishSettings {
        selected_map_idx: state.selected_map_idx,
        player_country: state.player_country,
        ai_country,
        starting_credits: state.starting_credits,
        start_position: state.player_start_position,
        short_game: state.short_game,
        zoom_enabled: state.zoom_enabled,
    }
}

fn hit_rect(rect: RectPx, x: i32, y: i32, action: SkirmishShellAction) -> SkirmishShellAction {
    if rect.contains(x, y) {
        action
    } else {
        SkirmishShellAction::None
    }
}

pub fn action_for_owner_draw_button(button: OwnerDrawButton) -> SkirmishShellAction {
    match button {
        OwnerDrawButton::StartGame0x617 => SkirmishShellAction::StartGame,
        OwnerDrawButton::ChooseMap0x5aa => SkirmishShellAction::ChooseMap,
        OwnerDrawButton::Back0x5c0 => SkirmishShellAction::BackOrExit,
    }
}

pub fn hit_test_owner_draw_button(
    layout: &SkirmishShellLayout,
    x: i32,
    y: i32,
) -> Option<OwnerDrawButton> {
    if layout.start_button.contains(x, y) {
        return Some(OwnerDrawButton::StartGame0x617);
    }
    if layout.choose_map_button.contains(x, y) {
        return Some(OwnerDrawButton::ChooseMap0x5aa);
    }
    if layout.back_button.contains(x, y) {
        return Some(OwnerDrawButton::Back0x5c0);
    }
    None
}

pub fn hit_test(layout: &SkirmishShellLayout, x: i32, y: i32) -> SkirmishShellAction {
    let start = hit_rect(layout.start_button, x, y, SkirmishShellAction::StartGame);
    if start != SkirmishShellAction::None {
        return start;
    }

    let choose = hit_rect(
        layout.choose_map_button,
        x,
        y,
        SkirmishShellAction::ChooseMap,
    );
    if choose != SkirmishShellAction::None {
        return choose;
    }

    let back = hit_rect(layout.back_button, x, y, SkirmishShellAction::BackOrExit);
    if back != SkirmishShellAction::None {
        return back;
    }

    for (idx, rect) in layout.color_combos.iter().copied().enumerate().rev() {
        if rect.contains(x, y) {
            return if idx == 0 {
                SkirmishShellAction::SelectColor(ColorComboId::Player)
            } else {
                SkirmishShellAction::SelectColor(ColorComboId::Ai(idx - 1))
            };
        }
    }

    SkirmishShellAction::None
}

pub fn apply_action(
    state: &mut SkirmishShellState,
    action: SkirmishShellAction,
    maps: &[MapMenuEntry],
) -> SkirmishShellAction {
    match action {
        SkirmishShellAction::None => SkirmishShellAction::None,
        SkirmishShellAction::StartGame => SkirmishShellAction::StartGame,
        SkirmishShellAction::BackOrExit => SkirmishShellAction::BackOrExit,
        SkirmishShellAction::ChooseMap => {
            if !maps.is_empty() {
                state.selected_map_idx = (state.selected_map_idx + 1) % maps.len();
            }
            SkirmishShellAction::None
        }
        SkirmishShellAction::SelectMap(idx) => {
            if idx < maps.len() {
                state.selected_map_idx = idx;
            }
            SkirmishShellAction::None
        }
        SkirmishShellAction::SelectColor(target) => {
            match target {
                ColorComboId::Player => {
                    state.player_color_index = (state.player_color_index + 1) % 8;
                }
                ColorComboId::Ai(idx) => {
                    if let Some(opponent) = state.opponents.get_mut(idx) {
                        opponent.color_index = (opponent.color_index + 1) % 8;
                    }
                }
            }
            SkirmishShellAction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::skirmish_shell::compute_layout;

    #[test]
    fn hit_test_start_choose_and_back() {
        let layout = compute_layout(800, 600);
        assert_eq!(hit_test(&layout, 636, 243), SkirmishShellAction::StartGame);
        assert_eq!(hit_test(&layout, 636, 287), SkirmishShellAction::ChooseMap);
        assert_eq!(hit_test(&layout, 645, 536), SkirmishShellAction::BackOrExit);
    }

    #[test]
    fn hit_test_uses_exclusive_bottom_right_edges() {
        let layout = compute_layout(800, 600);
        assert_eq!(hit_test(&layout, 635 + 162, 242), SkirmishShellAction::None);
        assert_eq!(hit_test(&layout, 635, 242 + 37), SkirmishShellAction::None);
    }

    #[test]
    fn owner_draw_button_hit_test_returns_control_identity() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test_owner_draw_button(&layout, 636, 243),
            Some(OwnerDrawButton::StartGame0x617)
        );
        assert_eq!(
            hit_test_owner_draw_button(&layout, 636, 287),
            Some(OwnerDrawButton::ChooseMap0x5aa)
        );
        assert_eq!(
            hit_test_owner_draw_button(&layout, 645, 536),
            Some(OwnerDrawButton::Back0x5c0)
        );
        assert_eq!(hit_test_owner_draw_button(&layout, 635 + 162, 242), None);
    }

    #[test]
    fn launch_settings_preserves_current_load_contract() {
        let shell = SkirmishShellState::default();
        let settings = launch_settings(&shell);
        assert_eq!(settings.selected_map_idx, shell.selected_map_idx);
        assert_eq!(settings.starting_credits, shell.starting_credits);
        assert_eq!(settings.short_game, shell.short_game);
    }

    #[test]
    fn hit_test_color_combos() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test(&layout, layout.color_combos[0].x, layout.color_combos[0].y),
            SkirmishShellAction::SelectColor(ColorComboId::Player)
        );
        assert_eq!(
            hit_test(&layout, layout.color_combos[1].x, layout.color_combos[1].y),
            SkirmishShellAction::SelectColor(ColorComboId::Ai(0))
        );
    }
}
