//! Text draw helpers for the skirmish shell renderer.
//!
//! Owns localized labels, text rect conversion, modal/listbox text,
//! status help text, and start-marker number labels.

use crate::app::AppState;
use crate::app_init::MapMenuEntry;
use crate::render::batch::SpriteInstance;
use crate::render::bit_font::BitFont;
use crate::render::shell_text::{self, ShellAlign, ShellTextDraw, TextRect};
use crate::ui::main_menu::SkirmishCountry;
use crate::ui::skirmish_shell::{
    COMBO_DROPDOWN_ROW_H, ChooseMapModalLayout, OwnerDrawButton, RectPx, SkirmishAiRowType,
    SkirmishCheckboxId, SkirmishComboItem, SkirmishCountryChoice, SkirmishShellLayout,
    SkirmishShellOpponent, SkirmishShellState, SkirmishTrackbarId, ValidationModalLayout,
    checkbox_text_rect, choose_map_listbox_content_rect, choose_map_listbox_row_rect,
    choose_map_listbox_visible_row_count, combo_dropdown_content_rect, combo_dropdown_rect,
    combo_dropdown_visible_row_count, combo_items, combo_text_rect, player_name_edit_text_rect,
    trackbar_value_text_rect, trackbar_visual_value,
};

use super::controls::trackbar_rect_for_id;
use super::{
    COMBODROPWIN_TEXT_INSET_X, COMBODROPWIN_TEXT_TRUNCATION_SCROLLBAR_RESERVE_PX,
    SHELL_CONTROL_TEXT_DEPTH, SHELL_DISABLED_TEXT_RGB_FROM_PACKED_0000009F,
    SHELL_DROPDOWN_TEXT_DEPTH, SHELL_LABEL_TEXT_RGB,
};

pub(super) fn localized_label(state: &AppState, key: &str, fallback: &str) -> String {
    state
        .csf
        .as_ref()
        .and_then(|csf| csf.get(key))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback.to_string())
}

pub(super) fn checkbox_label(id: SkirmishCheckboxId) -> (&'static str, &'static str) {
    match id {
        SkirmishCheckboxId::ShortGame0x54e => ("GUI:ShortGame", "Short Game"),
        SkirmishCheckboxId::McvRepacks0x693 => ("GUI:MCVRepacks", "MCV Repacks"),
        SkirmishCheckboxId::CratesAppear0x696 => ("GUI:CratesAppear", "Crates Appear"),
        SkirmishCheckboxId::SuperWeapons0x69a => ("GUI:SuperWeaponsAllowed", "Super Weapons"),
        SkirmishCheckboxId::BuildOffAlly0x69d => ("GUI:BuildOffAlly", "Build Off Ally"),
    }
}

pub(super) fn start_position_label(pos: crate::ui::main_menu::StartPosition) -> String {
    match pos {
        crate::ui::main_menu::StartPosition::Auto => "Random".to_string(),
        crate::ui::main_menu::StartPosition::Position(idx) => (idx + 1).to_string(),
    }
}

pub(super) fn team_label(team: i32) -> String {
    match team {
        0 => "A".to_string(),
        1 => "B".to_string(),
        2 => "C".to_string(),
        3 => "D".to_string(),
        _ => "None".to_string(),
    }
}

pub(super) fn combo_item_label(state: &AppState, item: SkirmishComboItem) -> String {
    match item {
        SkirmishComboItem::AiType(row_type) => {
            let (key, fallback) = row_type_label(row_type);
            localized_label(state, key, fallback)
        }
        SkirmishComboItem::Country(SkirmishCountryChoice::Random) => {
            localized_label(state, "GUI:RandomAsSymbols", "Random")
        }
        SkirmishComboItem::Country(SkirmishCountryChoice::Country(country)) => {
            country.label().to_string()
        }
        SkirmishComboItem::ColorSentinel(_) => String::new(),
        SkirmishComboItem::Color(_) => String::new(),
        SkirmishComboItem::Start(start) => match start {
            crate::ui::main_menu::StartPosition::Auto => {
                localized_label(state, "GUI:RandomAsSymbols", "Random")
            }
            crate::ui::main_menu::StartPosition::Position(idx) => (idx + 1).to_string(),
        },
        SkirmishComboItem::Team(team) => team_label(team),
    }
}

pub(super) fn country_choice_label(
    state: &AppState,
    random: bool,
    country: SkirmishCountry,
) -> String {
    if random {
        localized_label(state, "GUI:RandomAsSymbols", "Random")
    } else {
        country.label().to_string()
    }
}

pub(super) fn trackbar_display_value(shell: &SkirmishShellState, id: SkirmishTrackbarId) -> String {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => trackbar_visual_value(shell, id).to_string(),
        SkirmishTrackbarId::Credits0x511 => shell.starting_credits.to_string(),
        SkirmishTrackbarId::UnitCount0x50c => shell.unit_count.to_string(),
    }
}

pub(super) fn trackbar_value_text_color() -> [f32; 3] {
    SHELL_LABEL_TEXT_RGB
}

pub(super) fn trackbar_label(state: &AppState, id: SkirmishTrackbarId) -> String {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => localized_label(state, "GUI:GameSpeed", "Game Speed"),
        SkirmishTrackbarId::Credits0x511 => localized_label(state, "GUI:Credits", "Credits"),
        SkirmishTrackbarId::UnitCount0x50c => localized_label(state, "GUI:UnitCount", "Unit Count"),
    }
}

pub(super) fn trackbar_label_rect_for_id(
    layout: &SkirmishShellLayout,
    id: SkirmishTrackbarId,
) -> RectPx {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => layout.trackbar_labels.game_speed,
        SkirmishTrackbarId::Credits0x511 => layout.trackbar_labels.credits,
        SkirmishTrackbarId::UnitCount0x50c => layout.trackbar_labels.unit_count,
    }
}

pub(super) fn row_type_label(row_type: SkirmishAiRowType) -> (&'static str, &'static str) {
    row_type.label()
}

pub(super) fn push_button_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    pressed: bool,
    depth: f32,
) {
    let text_rect = button_text_rect(rect, pressed);
    push_text_draw(
        out,
        state,
        label,
        text_rect,
        button_label_color(),
        ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        depth,
    );
}

pub(super) fn button_label_color() -> [f32; 3] {
    SHELL_LABEL_TEXT_RGB
}

pub(super) fn combo_face_text_color(disabled: bool) -> [f32; 3] {
    if disabled {
        SHELL_DISABLED_TEXT_RGB_FROM_PACKED_0000009F
    } else {
        SHELL_LABEL_TEXT_RGB
    }
}

fn opponent_sibling_combo_text_color(opponent: &SkirmishShellOpponent) -> [f32; 3] {
    combo_face_text_color(!opponent.is_active())
}

pub(super) fn button_text_rect(rect: RectPx, pressed: bool) -> TextRect {
    let mut x = rect.x;
    let y = if pressed {
        x += 2;
        rect.y + 5
    } else {
        rect.y + 1
    };
    TextRect {
        x,
        y,
        w: (rect.x + rect.w - 2 - x).max(0) as u32,
        h: (rect.y + rect.h - y).max(0) as u32,
    }
}

pub(super) const fn validation_modal_body_text_align() -> ShellAlign {
    ShellAlign::NONE
}

pub(super) fn push_text_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    text_rect: TextRect,
    color: [f32; 3],
    align: ShellAlign,
    depth: f32,
) {
    let draw = shell_text::draw_in_rect(
        &state.bit_font,
        label,
        text_rect,
        color,
        align,
        [0.0, 0.0],
        depth,
    );
    out.push(draw);
}

pub(super) fn rect_to_text_rect(rect: RectPx) -> TextRect {
    TextRect {
        x: rect.x,
        y: rect.y,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    }
}

pub(super) fn combo_dropdown_text_rect_for_current_renderer(
    content: RectPx,
    visible_row: usize,
) -> TextRect {
    // `ComboDropWin` draws from x+3 to the row right edge. The separate
    // client_width-20 value is only the caller-side fit limit.
    TextRect {
        x: content.x + COMBODROPWIN_TEXT_INSET_X,
        y: content.y + visible_row as i32 * COMBO_DROPDOWN_ROW_H,
        w: (content.w - COMBODROPWIN_TEXT_INSET_X).max(0) as u32,
        h: COMBO_DROPDOWN_ROW_H.max(0) as u32,
    }
}

pub(super) fn combo_dropdown_text_fit_width(content: RectPx) -> u32 {
    (content.w - COMBODROPWIN_TEXT_TRUNCATION_SCROLLBAR_RESERVE_PX).max(0) as u32
}

pub(super) fn combo_face_text_fit_width(rect: RectPx) -> u32 {
    combo_text_rect(rect).w.max(0) as u32
}

pub(super) fn truncate_owner_draw_label<'a>(
    font: &BitFont,
    label: &'a str,
    fit_width: u32,
) -> std::borrow::Cow<'a, str> {
    if font.text_width(label) <= fit_width {
        return std::borrow::Cow::Borrowed(label);
    }

    let mut truncated = label.to_string();
    while !truncated.is_empty() && font.text_width(&truncated) > fit_width {
        truncated.pop();
    }
    std::borrow::Cow::Owned(truncated)
}

pub(super) fn truncate_combo_dropdown_label<'a>(
    font: &BitFont,
    label: &'a str,
    fit_width: u32,
) -> std::borrow::Cow<'a, str> {
    truncate_owner_draw_label(font, label, fit_width)
}

pub(super) fn push_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    depth: f32,
) {
    let text_rect = TextRect {
        x: rect.x,
        y: rect.y,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    };
    push_text_draw(
        out,
        state,
        label,
        text_rect,
        SHELL_LABEL_TEXT_RGB,
        ShellAlign::V_CENTER,
        depth,
    );
}

fn rects_intersect(a: RectPx, b: RectPx) -> bool {
    a.w > 0
        && a.h > 0
        && b.w > 0
        && b.h > 0
        && a.x < b.x + b.w
        && a.x + a.w > b.x
        && a.y < b.y + b.h
        && a.y + a.h > b.y
}

fn text_covered_by_overlay(rect: RectPx, overlays: &[RectPx]) -> bool {
    overlays
        .iter()
        .any(|overlay| rects_intersect(rect, *overlay))
}

fn push_combo_face_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    covering_overlays: &[RectPx],
) {
    push_combo_face_label_draw_with_color(
        out,
        state,
        label,
        rect,
        SHELL_LABEL_TEXT_RGB,
        covering_overlays,
    );
}

fn push_combo_face_label_draw_with_color(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    color: [f32; 3],
    covering_overlays: &[RectPx],
) {
    let text_rect = combo_text_rect(rect);
    if text_covered_by_overlay(text_rect, covering_overlays) {
        return;
    }
    let fit_width = combo_face_text_fit_width(rect);
    let label = truncate_owner_draw_label(&state.bit_font, label, fit_width);
    push_text_draw(
        out,
        state,
        label.as_ref(),
        rect_to_text_rect(text_rect),
        color,
        ShellAlign::V_CENTER,
        SHELL_CONTROL_TEXT_DEPTH,
    );
}

pub(super) fn push_static_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    align: ShellAlign,
    depth: f32,
) {
    push_text_draw(
        out,
        state,
        label,
        rect_to_text_rect(rect),
        SHELL_LABEL_TEXT_RGB,
        align,
        depth,
    );
}

pub(super) fn push_player_name_edit_text_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    shell: &SkirmishShellState,
    layout: &SkirmishShellLayout,
) {
    let rect = player_name_edit_text_rect(layout.player_name);
    let scissor = shell_text::ScissorRect {
        x: rect.x.max(0) as u32,
        y: rect.y.max(0) as u32,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    };
    if shell.player_name_edit.text.is_empty() {
        out.push(ShellTextDraw {
            instances: Vec::new(),
            scissor,
        });
        return;
    }

    let line_y =
        rect.y as f32 + ((rect.h as f32 - state.bit_font.glyph_height()).max(0.0) / 2.0).floor();
    let instances = state.bit_font.build_text(
        &shell.player_name_edit.text,
        (rect.x - shell.player_name_edit.scroll_x.max(0)) as f32,
        line_y,
        1.0,
        SHELL_CONTROL_TEXT_DEPTH,
        SHELL_LABEL_TEXT_RGB,
        [0.0, 0.0],
    );
    out.push(ShellTextDraw { instances, scissor });
}

pub(super) fn build_shell_text_draws(
    state: &AppState,
    layout: &SkirmishShellLayout,
    validation_layout: Option<&ValidationModalLayout>,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
) -> (Vec<ShellTextDraw>, Vec<SpriteInstance>) {
    let mut shell_draws: Vec<ShellTextDraw> = Vec::new();
    let bare_instances: Vec<SpriteInstance> = Vec::new();
    let mut covering_overlays = Vec::new();
    if let Some(dropdown) = shell
        .open_combo_dropdown
        .and_then(|open| combo_dropdown_rect(shell, layout, maps, open.id))
    {
        covering_overlays.push(dropdown);
    }
    if let Some(validation_layout) = validation_layout {
        covering_overlays.push(validation_layout.dialog);
    }

    let start = localized_label(state, "GUI:StartGame", "Start Game");
    let choose = localized_label(state, "GUI:ChooseMap", "Choose Map");
    let back = localized_label(state, "GUI:Back", "Back");

    for (label, rect, button) in [
        (
            start.as_str(),
            layout.start_button,
            OwnerDrawButton::StartGame0x617,
        ),
        (
            choose.as_str(),
            layout.choose_map_button,
            OwnerDrawButton::ChooseMap0x5aa,
        ),
        (
            back.as_str(),
            layout.back_button,
            OwnerDrawButton::Back0x5c0,
        ),
    ] {
        push_button_label_draw(
            &mut shell_draws,
            state,
            label,
            rect,
            shell.pressed_owner_draw_button == Some(button),
            0.00041,
        );
    }

    for (key, fallback, rect) in [
        ("GUI:Players", "Players", layout.column_labels.players),
        ("GUI:Side", "Side", layout.column_labels.side),
        ("GUI:Color", "Color", layout.column_labels.color),
        ("GUI:StartPosition", "Start", layout.column_labels.start),
        ("GUI:Team", "Team", layout.column_labels.team),
    ] {
        let label = localized_label(state, key, fallback);
        push_label_draw(
            &mut shell_draws,
            state,
            &label,
            rect,
            SHELL_CONTROL_TEXT_DEPTH,
        );
    }

    let title = localized_label(state, "GUI:SkirmishGame", "Skirmish Game");
    push_static_label_draw(
        &mut shell_draws,
        state,
        &title,
        layout.right_panel_text.title,
        ShellAlign::H_CENTER,
        SHELL_CONTROL_TEXT_DEPTH,
    );
    let game_type = state
        .skirmish_modes
        .iter()
        .find(|mode| mode.id == shell.selected_mode_id)
        .map(|mode| localized_label(state, &mode.ui_name_key, &mode.ui_name_key))
        .unwrap_or_else(|| localized_label(state, "GUI:Battle", "Battle"));
    push_static_label_draw(
        &mut shell_draws,
        state,
        &game_type,
        layout.right_panel_text.game_type,
        ShellAlign::H_CENTER,
        SHELL_CONTROL_TEXT_DEPTH,
    );
    let map_label = maps
        .get(shell.selected_map_idx)
        .map(|map| map.display_name.as_str())
        .unwrap_or("None");
    push_static_label_draw(
        &mut shell_draws,
        state,
        map_label,
        layout.right_panel_text.map_label,
        ShellAlign::H_CENTER,
        SHELL_CONTROL_TEXT_DEPTH,
    );

    push_player_name_edit_text_draw(&mut shell_draws, state, shell, layout);

    for checkbox in layout.checkboxes {
        let (key, fallback) = checkbox_label(checkbox.id);
        let label = localized_label(state, key, fallback);
        push_label_draw(
            &mut shell_draws,
            state,
            &label,
            checkbox_text_rect(checkbox.rect),
            SHELL_CONTROL_TEXT_DEPTH,
        );
    }

    for id in [
        SkirmishTrackbarId::GameSpeed0x529,
        SkirmishTrackbarId::Credits0x511,
        SkirmishTrackbarId::UnitCount0x50c,
    ] {
        let label = trackbar_label(state, id);
        push_label_draw(
            &mut shell_draws,
            state,
            &label,
            trackbar_label_rect_for_id(layout, id),
            SHELL_CONTROL_TEXT_DEPTH,
        );

        let value = trackbar_display_value(shell, id);
        push_text_draw(
            &mut shell_draws,
            state,
            &value,
            rect_to_text_rect(trackbar_value_text_rect(trackbar_rect_for_id(layout, id))),
            trackbar_value_text_color(),
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
            SHELL_CONTROL_TEXT_DEPTH,
        );
    }

    if let Some(status_help_text) = parent_shell_status_help_text(shell) {
        push_label_draw(
            &mut shell_draws,
            state,
            status_help_text,
            layout.status_help,
            SHELL_CONTROL_TEXT_DEPTH,
        );
    }

    push_combo_face_label_draw(
        &mut shell_draws,
        state,
        &country_choice_label(state, shell.player_country_random, shell.player_country),
        layout.rows.side_combos[0],
        &covering_overlays,
    );
    push_combo_face_label_draw(
        &mut shell_draws,
        state,
        &start_position_label(shell.player_start_position),
        layout.rows.start_combos[0],
        &covering_overlays,
    );
    push_combo_face_label_draw(
        &mut shell_draws,
        state,
        &team_label(shell.player_team),
        layout.rows.team_combos[0],
        &covering_overlays,
    );

    for (idx, opponent) in shell.opponents.iter().enumerate() {
        if idx >= layout.rows.ai_type_combos.len() {
            break;
        }
        let row = idx + 1;
        let (key, fallback) = row_type_label(opponent.row_type);
        let row_type = localized_label(state, key, fallback);
        push_combo_face_label_draw(
            &mut shell_draws,
            state,
            &row_type,
            layout.rows.ai_type_combos[idx],
            &covering_overlays,
        );
        let sibling_text_color = opponent_sibling_combo_text_color(opponent);
        push_combo_face_label_draw_with_color(
            &mut shell_draws,
            state,
            &country_choice_label(state, opponent.country_random, opponent.country),
            layout.rows.side_combos[row],
            sibling_text_color,
            &covering_overlays,
        );
        push_combo_face_label_draw_with_color(
            &mut shell_draws,
            state,
            &start_position_label(opponent.start_position),
            layout.rows.start_combos[row],
            sibling_text_color,
            &covering_overlays,
        );
        push_combo_face_label_draw_with_color(
            &mut shell_draws,
            state,
            &team_label(opponent.team),
            layout.rows.team_combos[row],
            sibling_text_color,
            &covering_overlays,
        );
    }

    if let Some(open) = shell.open_combo_dropdown {
        if let Some(dropdown) = combo_dropdown_rect(shell, layout, maps, open.id) {
            let content =
                combo_dropdown_content_rect(shell, layout, maps, open.id).unwrap_or(dropdown);
            let visible_rows = combo_dropdown_visible_row_count(shell, maps, open.id);
            for (idx, item) in combo_items(shell, maps, open.id)
                .into_iter()
                .skip(open.top_index)
                .take(visible_rows)
                .enumerate()
            {
                let label = combo_item_label(state, item);
                if label.is_empty() {
                    continue;
                }
                let rect = combo_dropdown_text_rect_for_current_renderer(content, idx);
                let fit_width = combo_dropdown_text_fit_width(content);
                let label = truncate_combo_dropdown_label(&state.bit_font, &label, fit_width);
                push_text_draw(
                    &mut shell_draws,
                    state,
                    label.as_ref(),
                    rect,
                    SHELL_LABEL_TEXT_RGB,
                    ShellAlign::V_CENTER,
                    SHELL_DROPDOWN_TEXT_DEPTH,
                );
            }
        }
    }

    (shell_draws, bare_instances)
}

pub(super) fn parent_shell_status_help_text(shell: &SkirmishShellState) -> Option<&str> {
    (!shell.status_help_text.is_empty()).then_some(shell.status_help_text.as_str())
}

#[cfg(test)]
pub(super) fn choose_map_modal_parent_status_help_text(
    _shell: &SkirmishShellState,
) -> Option<&str> {
    None
}

pub(super) fn choose_map_modal_status_help_text(shell: &SkirmishShellState) -> Option<&str> {
    (!shell.status_help_text.is_empty()).then_some(shell.status_help_text.as_str())
}

pub(super) fn push_choose_map_modal_text_draws(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    layout: &ChooseMapModalLayout,
) {
    let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_ref() else {
        return;
    };

    for (key, fallback, rect, align) in [
        (
            "GUI:ChooseMap",
            "Choose Map",
            layout.title,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        ),
        (
            "GUI:SelectEngagement",
            "Select Engagement",
            layout.select_engagement,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        ),
        (
            "GUI:GameType",
            "Game Type",
            layout.game_type_heading,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        ),
        (
            "GUI:GameMap",
            "Game Map",
            layout.game_map_heading,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        ),
    ] {
        let label = localized_label(state, key, fallback);
        push_text_draw(
            out,
            state,
            &label,
            rect_to_text_rect(rect),
            SHELL_LABEL_TEXT_RGB,
            align,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.00008,
        );
    }

    for (label, rect) in [
        (
            localized_label(state, "GUI:UseMap", "Use Map"),
            layout.use_map_button,
        ),
        (
            localized_label(state, "GUI:Cancel", "Cancel"),
            layout.cancel_button,
        ),
        (
            localized_label(state, "GUI:CreateRandomMap", "Create Random Map"),
            layout.create_random_map_button,
        ),
    ] {
        push_text_draw(
            out,
            state,
            &label,
            rect_to_text_rect(rect),
            SHELL_LABEL_TEXT_RGB,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.00009,
        );
    }

    if let Some(status_help_text) = choose_map_modal_status_help_text(&state.skirmish_shell_state) {
        push_label_draw(
            out,
            state,
            status_help_text,
            layout.status_help,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.0001,
        );
    }

    let mode_count = modal.mode_row_count(&state.skirmish_modes);
    let mode_content = choose_map_listbox_content_rect(mode_count, layout.mode_list);
    let visible_mode_rows = choose_map_listbox_visible_row_count(layout.mode_list);
    for (visible_row, mode) in state
        .skirmish_modes
        .iter()
        .skip(modal.mode_top_index)
        .take(visible_mode_rows)
        .enumerate()
    {
        let label = localized_label(state, &mode.ui_name_key, &mode.ui_name_key);
        let row_rect = choose_map_listbox_row_rect(mode_content, visible_row);
        let rect = RectPx::new(row_rect.x + 2, row_rect.y, row_rect.w - 2, row_rect.h);
        push_text_draw(
            out,
            state,
            &label,
            rect_to_text_rect(rect),
            SHELL_LABEL_TEXT_RGB,
            ShellAlign::V_CENTER,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.00009,
        );
    }

    let map_count = modal.map_row_count();
    let map_content = choose_map_listbox_content_rect(map_count, layout.map_list);
    let visible_map_rows = choose_map_listbox_visible_row_count(layout.map_list);
    for (visible_row, record_idx) in modal
        .filtered_record_indices
        .iter()
        .skip(modal.map_top_index)
        .take(visible_map_rows)
        .enumerate()
    {
        let Some(record) = state.skirmish_scenario_records.get(*record_idx) else {
            continue;
        };
        let row_rect = choose_map_listbox_row_rect(map_content, visible_row);
        let rect = RectPx::new(row_rect.x + 2, row_rect.y, row_rect.w - 2, row_rect.h);
        push_text_draw(
            out,
            state,
            &record.display_name,
            rect_to_text_rect(rect),
            SHELL_LABEL_TEXT_RGB,
            ShellAlign::V_CENTER,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.00009,
        );
    }
}

pub(super) fn push_validation_modal_text_draws(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    layout: &ValidationModalLayout,
) {
    let Some(modal) = state.skirmish_shell_state.validation_modal.as_ref() else {
        return;
    };
    push_text_draw(
        out,
        state,
        &modal.message,
        rect_to_text_rect(layout.message),
        SHELL_LABEL_TEXT_RGB,
        validation_modal_body_text_align(),
        SHELL_DROPDOWN_TEXT_DEPTH - 0.00012,
    );
    push_text_draw(
        out,
        state,
        &modal.ok_button,
        button_text_rect(layout.ok_button, modal.ok_button_pressed),
        SHELL_LABEL_TEXT_RGB,
        ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        SHELL_DROPDOWN_TEXT_DEPTH - 0.00013,
    );
}

pub(super) fn push_start_marker_labels(
    out: &mut Vec<SpriteInstance>,
    state: &AppState,
    projected_positions: &[(i32, i32)],
    depth: f32,
) {
    for (idx, &(x, y)) in projected_positions.iter().enumerate() {
        let label = (idx + 1).to_string();
        let (label_x, label_y) = start_marker_label_origin(x, y);
        out.extend(state.bit_font.build_text(
            &label,
            label_x as f32,
            label_y as f32,
            1.0,
            depth,
            start_marker_label_color(),
            [0.0, 0.0],
        ));
    }
}

pub(super) fn start_marker_label_origin(anchor_x: i32, anchor_y: i32) -> (i32, i32) {
    (anchor_x - 2, anchor_y - 6)
}

pub(super) fn start_marker_label_color() -> [f32; 3] {
    SHELL_LABEL_TEXT_RGB
}

pub(super) fn build_start_marker_label_instances(
    state: &AppState,
    projected_positions: &[(i32, i32)],
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();
    push_start_marker_labels(&mut instances, state, projected_positions, 0.00040);
    instances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combo_face_text_under_open_dropdown_is_suppressed() {
        let dropdown = RectPx::new(23, 96, 149, 92);
        let covered_combo_text = RectPx::new(25, 123, 129, 24);
        let clear_combo_text = RectPx::new(25, 71, 129, 24);

        assert!(text_covered_by_overlay(covered_combo_text, &[dropdown]));
        assert!(!text_covered_by_overlay(clear_combo_text, &[dropdown]));
        assert!(!text_covered_by_overlay(covered_combo_text, &[]));
    }

    #[test]
    fn text_under_validation_dialog_is_suppressed_like_dropdown_text() {
        let dialog = RectPx::new(220, 239, 360, 122);
        let covered_combo_text = RectPx::new(260, 260, 100, 24);
        let clear_combo_text = RectPx::new(25, 71, 129, 24);

        assert!(text_covered_by_overlay(covered_combo_text, &[dialog]));
        assert!(!text_covered_by_overlay(clear_combo_text, &[dialog]));
    }

    #[test]
    fn adjacent_rects_do_not_count_as_dropdown_coverage() {
        let dropdown = RectPx::new(10, 20, 100, 50);
        let above = RectPx::new(10, 0, 100, 20);
        let below = RectPx::new(10, 70, 100, 20);

        assert!(!text_covered_by_overlay(above, &[dropdown]));
        assert!(!text_covered_by_overlay(below, &[dropdown]));
    }

    #[test]
    fn team_label_uses_native_sentinel_values() {
        assert_eq!(team_label(-2), "None");
        assert_eq!(team_label(0), "A");
        assert_eq!(team_label(1), "B");
        assert_eq!(team_label(2), "C");
        assert_eq!(team_label(3), "D");
    }

    #[test]
    fn trackbar_value_text_uses_normal_shell_yellow_source() {
        assert_eq!(trackbar_value_text_color(), SHELL_LABEL_TEXT_RGB);
    }

    #[test]
    fn combo_face_text_color_uses_disabled_source_for_inactive_siblings() {
        let shell = SkirmishShellState::default();

        assert_eq!(combo_face_text_color(false), SHELL_LABEL_TEXT_RGB);
        assert_eq!(
            combo_face_text_color(true),
            SHELL_DISABLED_TEXT_RGB_FROM_PACKED_0000009F
        );
        assert_eq!(
            opponent_sibling_combo_text_color(&shell.opponents[0]),
            SHELL_LABEL_TEXT_RGB
        );
        assert_eq!(
            opponent_sibling_combo_text_color(&shell.opponents[1]),
            SHELL_DISABLED_TEXT_RGB_FROM_PACKED_0000009F
        );
    }

    #[test]
    fn combodropwin_row_text_pretruncates_without_using_clip_width_as_rect_width() {
        let font = crate::render::bit_font::tests::make_test_font(
            &[
                (b'a' as u16, 6),
                (b'b' as u16, 6),
                (b'c' as u16, 6),
                (b'd' as u16, 6),
                (b' ' as u16, 4),
            ],
            4,
        );
        let content = RectPx::new(100, 50, 40, COMBO_DROPDOWN_ROW_H);

        let rect = combo_dropdown_text_rect_for_current_renderer(content, 0);
        assert_eq!(rect.x, 103);
        assert_eq!(rect.w, 37);
        assert_eq!(combo_dropdown_text_fit_width(content), 20);

        let label = truncate_combo_dropdown_label(&font, "ab cd", 20);
        assert_eq!(label.as_ref(), "ab ");
        assert!(font.text_width(label.as_ref()) <= 20);
    }

    #[test]
    fn collapsed_combo_face_text_pretruncates_to_arrow_reserved_width() {
        let font = crate::render::bit_font::tests::make_test_font(
            &[
                (b'a' as u16, 6),
                (b'b' as u16, 6),
                (b'c' as u16, 6),
                (b'd' as u16, 6),
                (b' ' as u16, 4),
            ],
            4,
        );
        let combo = RectPx::new(423, 59, 44, 24);

        assert_eq!(combo_face_text_fit_width(combo), 24);
        let label = truncate_owner_draw_label(&font, "ab cd", combo_face_text_fit_width(combo));

        assert_eq!(label.as_ref(), "ab ");
        assert!(font.text_width(label.as_ref()) <= combo_face_text_fit_width(combo));
    }
}
