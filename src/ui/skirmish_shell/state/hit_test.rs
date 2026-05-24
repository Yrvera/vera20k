//! Hit testing, hover targets, status-help keys, and action application for the skirmish shell.

use crate::app_init::MapMenuEntry;
use crate::skirmish_launch::SKIRMISH_PLAYER_SLOT_COUNT;

use super::super::layout::{
    COMBO_DROPDOWN_ROW_H, ChooseMapModalButton, ChooseMapModalLayout, ColorComboId, RectPx,
    SKIRMISH_AI_ROW_COUNT, SkirmishCheckboxId, SkirmishShellLayout, SkirmishTrackbarId,
    choose_map_listbox_row_at, combo_face_rect,
};
use super::trackbars::{trackbar_ids, trackbar_rect};
use super::{
    ChooseMapHoverTarget, ChooseMapModalState, OwnerDrawButton, SkirmishAiRowType, SkirmishComboId,
    SkirmishComboItem, SkirmishHoverTarget, SkirmishShellAction, SkirmishShellState,
    combo_dropdown_content_rect, combo_dropdown_visible_row_count, combo_items, combo_rect,
};

fn hover_open_combo_item(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    x: i32,
    y: i32,
) -> Option<SkirmishHoverTarget> {
    let open = state.open_combo_dropdown?;
    let content = combo_dropdown_content_rect(state, layout, maps, open.id)?;
    if !content.contains(x, y) {
        return None;
    }

    let visible_row = ((y - content.y) / COMBO_DROPDOWN_ROW_H).max(0) as usize;
    if visible_row >= combo_dropdown_visible_row_count(state, maps, open.id) {
        return None;
    }
    let item_index = open.top_index + visible_row;
    let item = combo_items(state, maps, open.id).get(item_index).copied()?;
    Some(SkirmishHoverTarget::ComboItem { id: open.id, item })
}

pub fn hovered_shell_control(
    layout: &SkirmishShellLayout,
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    x: i32,
    y: i32,
) -> Option<SkirmishHoverTarget> {
    if state.choose_map_modal.is_some() || state.validation_modal.is_some() {
        return None;
    }

    if let Some(target) = hover_open_combo_item(state, layout, maps, x, y) {
        return Some(target);
    }

    if layout.status_help.contains(x, y) {
        return Some(SkirmishHoverTarget::StatusHelp0x695);
    }
    if layout.player_name.contains(x, y) {
        return Some(SkirmishHoverTarget::PlayerName0x6a0);
    }
    if let Some(button) = hit_test_owner_draw_button(layout, x, y) {
        return Some(SkirmishHoverTarget::OwnerDrawButton(button));
    }
    if layout.map_preview.contains(x, y) {
        return Some(SkirmishHoverTarget::MapPreview0x468);
    }
    for checkbox in layout.checkboxes {
        if checkbox.rect.contains(x, y) {
            return Some(SkirmishHoverTarget::Checkbox(checkbox.id));
        }
    }
    for id in trackbar_ids() {
        if trackbar_rect(layout, id).contains(x, y) {
            return Some(SkirmishHoverTarget::Trackbar(id));
        }
    }
    for row in 0..SKIRMISH_PLAYER_SLOT_COUNT {
        for id in [
            SkirmishComboId::Side(row),
            SkirmishComboId::Color(row),
            SkirmishComboId::Start(row),
            SkirmishComboId::Team(row),
        ] {
            if combo_rect(layout, id).is_some_and(|rect| combo_face_rect(rect).contains(x, y)) {
                return Some(SkirmishHoverTarget::ComboFace(id));
            }
        }
    }
    for row in 0..SKIRMISH_AI_ROW_COUNT {
        let id = SkirmishComboId::AiType(row);
        if combo_rect(layout, id).is_some_and(|rect| combo_face_rect(rect).contains(x, y)) {
            return Some(SkirmishHoverTarget::ComboFace(id));
        }
    }

    None
}

pub fn status_help_key_for_hover(target: SkirmishHoverTarget) -> Option<&'static str> {
    match target {
        SkirmishHoverTarget::StatusHelp0x695 => None,
        SkirmishHoverTarget::PlayerName0x6a0 => Some("STT:SkirmishEditPlayer"),
        SkirmishHoverTarget::MapPreview0x468 => Some("STT:SkirmishMapThumbnail"),
        SkirmishHoverTarget::OwnerDrawButton(OwnerDrawButton::StartGame0x617) => {
            Some("STT:SkirmishButtonStartGame")
        }
        SkirmishHoverTarget::OwnerDrawButton(OwnerDrawButton::ChooseMap0x5aa) => {
            Some("STT:SkirmishButtonChooseMap")
        }
        SkirmishHoverTarget::OwnerDrawButton(OwnerDrawButton::Back0x5c0) => {
            Some("STT:SkirmishButtonBack")
        }
        SkirmishHoverTarget::Checkbox(SkirmishCheckboxId::ShortGame0x54e) => {
            Some("STT:SkirmishCBoxShortGame")
        }
        SkirmishHoverTarget::Checkbox(SkirmishCheckboxId::McvRepacks0x693) => {
            Some("STT:SkirmishCBoxRedeploys")
        }
        SkirmishHoverTarget::Checkbox(SkirmishCheckboxId::CratesAppear0x696) => {
            Some("STT:SkirmishCBoxCrates")
        }
        SkirmishHoverTarget::Checkbox(SkirmishCheckboxId::SuperWeapons0x69a) => {
            Some("STT:SkirmishCBoxSWAllowed")
        }
        SkirmishHoverTarget::Checkbox(SkirmishCheckboxId::BuildOffAlly0x69d) => {
            Some("STT:SkirmishCBoxBuildOffAlly")
        }
        SkirmishHoverTarget::Trackbar(SkirmishTrackbarId::GameSpeed0x529) => {
            Some("STT:SkirmishSliderSpeed")
        }
        SkirmishHoverTarget::Trackbar(SkirmishTrackbarId::Credits0x511) => {
            Some("STT:SkirmishSliderCredits")
        }
        SkirmishHoverTarget::Trackbar(SkirmishTrackbarId::UnitCount0x50c) => {
            Some("STT:SkirmishSliderUnit")
        }
        SkirmishHoverTarget::ComboFace(id) => status_help_key_for_combo(id),
        SkirmishHoverTarget::ComboItem {
            id: SkirmishComboId::AiType(_),
            item: SkirmishComboItem::AiType(row_type),
        } => status_help_key_for_ai_row_type(row_type),
        SkirmishHoverTarget::ComboItem { id, .. } => status_help_key_for_combo(id),
    }
}

pub fn hovered_choose_map_modal_control(
    layout: &ChooseMapModalLayout,
    modal: &ChooseMapModalState,
    mode_count: usize,
    x: i32,
    y: i32,
) -> Option<ChooseMapHoverTarget> {
    if layout.status_help.contains(x, y) {
        return Some(ChooseMapHoverTarget::StatusHelp0x695);
    }
    if let Some(button) = super::super::layout::choose_map_modal_button_at(layout, x, y) {
        return Some(ChooseMapHoverTarget::Button(button));
    }
    if layout.preview.contains(x, y) {
        return Some(ChooseMapHoverTarget::Preview0x468);
    }
    if layout.mode_list.contains(x, y) {
        if let Some(mode_index) =
            choose_map_listbox_row_at(layout.mode_list, mode_count, modal.mode_top_index, x, y)
        {
            return Some(ChooseMapHoverTarget::ModeListRow0x6eb { mode_index });
        }
        return Some(ChooseMapHoverTarget::ModeList0x6eb);
    }
    if layout.map_list.contains(x, y) {
        return Some(ChooseMapHoverTarget::MapList0x553);
    }
    None
}

pub fn status_help_key_for_choose_map_hover(target: ChooseMapHoverTarget) -> Option<&'static str> {
    match target {
        ChooseMapHoverTarget::StatusHelp0x695 => None,
        ChooseMapHoverTarget::ModeList0x6eb | ChooseMapHoverTarget::ModeListRow0x6eb { .. } => {
            Some("STT:ScenarioListGameType")
        }
        ChooseMapHoverTarget::MapList0x553 => Some("STT:ScenarioListMaps"),
        ChooseMapHoverTarget::Preview0x468 => Some("STT:ScenarioMapThumbnail"),
        ChooseMapHoverTarget::Button(ChooseMapModalButton::UseMap0x6c5) => {
            Some("STT:ScenarioButtonUseMap")
        }
        ChooseMapHoverTarget::Button(ChooseMapModalButton::CreateRandomMap0x583) => {
            Some("STT:ScenarioButtonRandom")
        }
        ChooseMapHoverTarget::Button(ChooseMapModalButton::Cancel0x5c0) => {
            Some("STT:ScenarioButtonCancel")
        }
    }
}

fn status_help_key_for_combo(id: SkirmishComboId) -> Option<&'static str> {
    match id {
        SkirmishComboId::AiType(_) => Some("STT:SkirmishComboAIPlayer"),
        SkirmishComboId::Side(_) => Some("STT:SkirmishComboCountry"),
        SkirmishComboId::Color(_) => Some("STT:SkirmishComboColor"),
        SkirmishComboId::Start(_) => Some("STT:HostComboStart"),
        SkirmishComboId::Team(_) => Some("STT:HostComboTeam"),
    }
}

fn status_help_key_for_ai_row_type(row_type: SkirmishAiRowType) -> Option<&'static str> {
    match row_type {
        SkirmishAiRowType::None => Some("STT:PlayerNone"),
        SkirmishAiRowType::Easy => Some("STT:PlayerDumbAI"),
        SkirmishAiRowType::Normal => Some("STT:PlayerSmartAI"),
        SkirmishAiRowType::Hard => Some("STT:PlayerGeniusAI"),
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
        SkirmishShellAction::ChooseMap => SkirmishShellAction::ChooseMap,
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
