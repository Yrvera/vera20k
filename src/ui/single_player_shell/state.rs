//! Dialog 0x100 Single Player shell control identity and CSF/result lookups.
//!
//! Hit-testing and the press-must-match-release gesture (including the Load Saved
//! Game disabled guard) moved to the shared `ui::shell::controller::DialogController`
//! (substrate Slice 2); this module keeps the control identity, the CSF keys, and
//! the action/result-code tables the controller's activated-control id maps through.

use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinglePlayerControlId {
    NewCampaign0x688,
    LoadSavedGame0x689,
    Skirmish0x579,
    MainMenu0x686,
}

impl SinglePlayerControlId {
    /// The Win32 control resource id this identity stands for. The controller
    /// works in raw resource ids; the app maps back via [`Self::from_resource_id`].
    pub fn resource_id(self) -> u16 {
        match self {
            Self::NewCampaign0x688 => 0x0688,
            Self::LoadSavedGame0x689 => 0x0689,
            Self::Skirmish0x579 => 0x0579,
            Self::MainMenu0x686 => 0x0686,
        }
    }

    /// Inverse of [`Self::resource_id`]; `None` for an unknown id.
    pub fn from_resource_id(id: u16) -> Option<Self> {
        Some(match id {
            0x0688 => Self::NewCampaign0x688,
            0x0689 => Self::LoadSavedGame0x689,
            0x0579 => Self::Skirmish0x579,
            0x0686 => Self::MainMenu0x686,
            _ => return None,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::controller::DialogController;
    use crate::ui::shell::descriptor::DialogId;
    use crate::ui::shell::layout::LaidOutControl;
    use crate::ui::single_player_shell::compute_layout;

    /// Adapt the laid-out single-player buttons into the controller's button feed.
    fn button_feed(
        layout: &crate::ui::single_player_shell::SinglePlayerShellLayout,
    ) -> Vec<LaidOutControl> {
        layout
            .buttons
            .iter()
            .map(|b| LaidOutControl {
                id: b.id.resource_id(),
                rect: b.rect,
            })
            .collect()
    }

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
    fn controller_hits_dialog_0x100_buttons_by_geometry() {
        let layout = compute_layout(800, 600);
        let feed = button_feed(&layout);
        let mut c = DialogController::default();
        c.ensure_active(DialogId(0x0100), false);
        c.on_pointer_down(639, 204, &feed);
        assert_eq!(
            c.pressed(),
            Some(SinglePlayerControlId::NewCampaign0x688.resource_id())
        );
        c.on_pointer_down(639, 290, &feed);
        assert_eq!(
            c.pressed(),
            Some(SinglePlayerControlId::Skirmish0x579.resource_id())
        );
        c.on_pointer_down(639, 540, &feed);
        assert_eq!(
            c.pressed(),
            Some(SinglePlayerControlId::MainMenu0x686.resource_id())
        );
    }

    #[test]
    fn controller_disabled_load_saved_game_suppresses_press_but_still_hovers() {
        let layout = compute_layout(800, 600);
        let feed = button_feed(&layout);
        let load = SinglePlayerControlId::LoadSavedGame0x689.resource_id();
        let mut c = DialogController::default();
        c.ensure_active(DialogId(0x0100), false);
        // Disabled (no saves): press suppressed, no action emitted...
        c.set_disabled(load, true);
        c.on_pointer_down(639, 248, &feed);
        assert_eq!(c.pressed(), None);
        assert_eq!(c.on_pointer_up(639, 248, &feed), None);
        // ...but the disabled button still hover-tracks and arms its timer.
        c.on_pointer_move(639, 248, &feed);
        assert_eq!(c.hovered(), Some(load));
        assert!(c.hover_started_at().is_some());
        // Enabled: press-and-release fires Load Saved Game.
        c.set_disabled(load, false);
        c.on_pointer_down(639, 248, &feed);
        let activated = c.on_pointer_up(639, 248, &feed);
        assert_eq!(
            activated
                .and_then(SinglePlayerControlId::from_resource_id)
                .map(action_for_control),
            Some(SinglePlayerShellAction::LoadSavedGame)
        );
    }
}
