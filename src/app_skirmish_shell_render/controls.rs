//! Owner-draw control sprite helpers for the skirmish shell renderer.
//!
//! Contains player-name edit chrome, checkboxes, trackbars, combo faces,
//! and ComboDropWin popup sprite construction.

use crate::app_init::MapMenuEntry;
use crate::render::batch::SpriteInstance;
use crate::render::bit_font::BitFont;
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::rules::color_scheme::{ColorSchemeEntry, hsv_to_rgb, scheme_for_priority};
use crate::rules::house_colors::{HouseColorIndex, house_color_ramp};
use crate::ui::skirmish_shell::{
    COMBO_DROPDOWN_ROW_H, COMBO_DROPDOWN_SCROLLBAR_BUTTON_H, DropdownScrollbarPart, RectPx,
    SkirmishCheckboxId, SkirmishComboId, SkirmishComboItem, SkirmishShellLayout,
    SkirmishShellState, SkirmishTrackbarId, checkbox_icon_rect, combo_arrow_rect,
    combo_dropdown_content_rect, combo_dropdown_needs_scrollbar, combo_dropdown_rect,
    combo_dropdown_scroll_thumb_rect, combo_dropdown_scrollbar_rect,
    combo_dropdown_visible_row_count, combo_face_rect, combo_items, combo_swatch_rect,
    player_name_edit_text_rect, selected_combo_item_index, trackbar_pixel_offset,
    trackbar_plaque_rect, trackbar_thumb_rect, trackbar_visual_value,
};

use super::chrome::{
    push_entry, push_entry_native, push_ownerdraw_two_pixel_bevel_frame, push_solid_rect,
    push_tinted_entry,
};
use super::{
    OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF, SHELL_CONTROL_DEPTH,
    SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE, SHELL_DROPDOWN_DEPTH,
    SHELL_EDIT_CARET_DEPTH, SHELL_EDIT_FRAME_DEPTH, SHELL_EDIT_SELECTION_DEPTH,
    SHELL_LABEL_TEXT_RGB, SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
    SHELL_SWATCH_DEPTH,
};

pub(super) fn checkbox_entry(
    atlas: &SkirmishShellChromeAtlas,
    checked: bool,
) -> Option<SkirmishShellChromeEntry> {
    if checked {
        atlas.checkbox_checked_cce_i
    } else {
        atlas.checkbox_unchecked_cue_i
    }
}

pub(super) fn checkbox_checked(shell: &SkirmishShellState, id: SkirmishCheckboxId) -> bool {
    match id {
        SkirmishCheckboxId::ShortGame0x54e => shell.short_game,
        SkirmishCheckboxId::McvRepacks0x693 => shell.mcv_redeploy,
        SkirmishCheckboxId::CratesAppear0x696 => shell.crates,
        SkirmishCheckboxId::SuperWeapons0x69a => shell.super_weapons,
        SkirmishCheckboxId::BuildOffAlly0x69d => shell.build_off_ally,
    }
}

pub(super) fn combo_face_entry(
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
) -> Option<SkirmishShellChromeEntry> {
    match rect.w {
        150 => atlas.combo_face_150,
        117 => atlas.combo_face_117,
        44 => atlas.combo_face_44,
        38 => atlas.combo_face_38,
        _ => None,
    }
}

pub(super) fn char_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

pub(super) fn player_name_caret_x_from_prefix_width(
    text_rect: RectPx,
    scroll_x: i32,
    prefix_width: u32,
) -> i32 {
    text_rect.x - scroll_x.max(0) + prefix_width as i32
}

pub(super) fn player_name_caret_x(
    font: &BitFont,
    shell: &SkirmishShellState,
    text_rect: RectPx,
) -> i32 {
    let edit = &shell.player_name_edit;
    let prefix = &edit.text[..char_byte_index(&edit.text, edit.caret)];
    player_name_caret_x_from_prefix_width(text_rect, edit.scroll_x, font.text_width(prefix))
}

pub(super) fn push_player_name_edit_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    font: &BitFont,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    push_ownerdraw_two_pixel_bevel_frame(out, atlas, layout.player_name, SHELL_EDIT_FRAME_DEPTH);

    let text_rect = player_name_edit_text_rect(layout.player_name);
    if shell.player_name_edit.focused {
        if let Some((start, end)) = shell.player_name_edit.selection {
            let (start, end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            if start != end {
                let start_byte = char_byte_index(&shell.player_name_edit.text, start);
                let end_byte = char_byte_index(&shell.player_name_edit.text, end);
                let prefix = &shell.player_name_edit.text[..start_byte];
                let selected = &shell.player_name_edit.text[start_byte..end_byte];
                let x = player_name_caret_x_from_prefix_width(
                    text_rect,
                    shell.player_name_edit.scroll_x,
                    font.text_width(prefix),
                );
                let w = font.text_width(selected) as i32;
                push_solid_rect(
                    out,
                    atlas,
                    RectPx::new(x, text_rect.y + 2, w.max(1), (text_rect.h - 4).max(1)),
                    OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF,
                    SHELL_EDIT_SELECTION_DEPTH,
                );
            }
        } else {
            let x = player_name_caret_x(font, shell, text_rect);
            push_solid_rect(
                out,
                atlas,
                RectPx::new(x, text_rect.y + 2, 2, (text_rect.h - 4).max(1)),
                SHELL_LABEL_TEXT_RGB,
                SHELL_EDIT_CARET_DEPTH,
            );
        }
    }
}

pub(super) fn push_scrollbar_thumb(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    depth: f32,
) {
    let top_h = atlas
        .scrollbar_thumb_top
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    let bottom_h = atlas
        .scrollbar_thumb_bottom
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    if let Some(top) = atlas.scrollbar_thumb_top {
        push_entry_native(out, top, rect.x, rect.y, depth);
    }
    if let Some(bottom) = atlas.scrollbar_thumb_bottom {
        push_entry_native(out, bottom, rect.x, rect.y + rect.h - bottom_h, depth);
    }
    if let Some(mid) = atlas.scrollbar_thumb_mid {
        let mid_y = rect.y + top_h;
        let mid_h = rect.h - top_h - bottom_h;
        if mid_h > 0 {
            push_entry(out, mid, RectPx::new(rect.x, mid_y, rect.w, mid_h), depth);
        }
    }
}

pub(super) fn push_dropdown_scrollbar_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    scrollbar: RectPx,
    thumb: RectPx,
    pressed_part: Option<DropdownScrollbarPart>,
) {
    push_solid_rect(
        out,
        atlas,
        scrollbar,
        SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
        SHELL_DROPDOWN_DEPTH - 0.00004,
    );
    let up_entry = scrollbar_arrow_entry(
        atlas.scrollbar_arrow_up_released,
        atlas.scrollbar_arrow_up_pressed,
        pressed_part == Some(DropdownScrollbarPart::UpArrow),
    );
    if let Some(up) = up_entry {
        push_entry_native(
            out,
            up,
            scrollbar.x,
            scrollbar.y,
            SHELL_DROPDOWN_DEPTH - 0.00005,
        );
    }
    let down_entry = scrollbar_arrow_entry(
        atlas.scrollbar_arrow_down_released,
        atlas.scrollbar_arrow_down_pressed,
        pressed_part == Some(DropdownScrollbarPart::DownArrow),
    );
    if let Some(down) = down_entry {
        push_entry_native(
            out,
            down,
            scrollbar.x,
            scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
            SHELL_DROPDOWN_DEPTH - 0.00005,
        );
    }
    push_scrollbar_thumb(out, atlas, thumb, SHELL_DROPDOWN_DEPTH - 0.00006);
    push_ownerdraw_two_pixel_bevel_frame(out, atlas, scrollbar, SHELL_DROPDOWN_DEPTH - 0.00007);
}

pub(super) fn scrollbar_arrow_entry(
    released: Option<SkirmishShellChromeEntry>,
    pressed: Option<SkirmishShellChromeEntry>,
    is_pressed: bool,
) -> Option<SkirmishShellChromeEntry> {
    if is_pressed {
        pressed.or(released)
    } else {
        released
    }
}

/// Resolve a lobby color slot (0..=7) to its swatch RGB.
///
/// The 8 lobby color slots present the `[Colors]` schemes in priority order: the
/// slot index IS the color priority. `scheme_for_priority` applies the priority
/// LUT + scheme-doubling, then `hsv_to_rgb` runs the same 6-sextant integer
/// conversion the loading-screen backing uses — so a lobby swatch and the loading
/// backing match for a given slot.
///
/// Falls back to the legacy synthesized ramp only when the `[Colors]` list is empty
/// (rules not yet loaded), so the swatch still renders rather than going black; in a
/// normal skirmish lobby the scheme list is always populated.
pub(super) fn house_color_tint(color_schemes: &[ColorSchemeEntry], index: usize) -> [f32; 3] {
    if let Some(scheme) = scheme_for_priority(color_schemes, index as i32) {
        let [r, g, b] = hsv_to_rgb(scheme.hsv);
        return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
    }
    let ramp = house_color_ramp(HouseColorIndex(index.min(7) as u8));
    let color = ramp[0];
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    ]
}

pub(super) fn push_combo_face(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    color_schemes: &[ColorSchemeEntry],
    rect: RectPx,
    color_index: Option<usize>,
    open: bool,
    disabled: bool,
    depth: f32,
) {
    if let Some(face) = combo_face_entry(atlas, rect) {
        push_entry(out, face, combo_face_rect(rect), depth);
    }
    if let (Some(color_index), Some(white)) = (color_index, atlas.white_pixel) {
        push_tinted_entry(
            out,
            white,
            combo_swatch_rect(rect),
            house_color_tint(color_schemes, color_index),
            SHELL_SWATCH_DEPTH,
        );
    }
    let arrow = match (disabled, open) {
        (true, true) => atlas.combo_arrow_down_gray_pressed,
        (true, false) => atlas.combo_arrow_down_gray_released,
        (false, true) => atlas.combo_arrow_down_pressed,
        (false, false) => atlas.combo_arrow_down_released,
    };
    if let Some(arrow) = arrow {
        let arrow_rect = combo_arrow_rect(rect);
        push_entry_native(out, arrow, arrow_rect.x, arrow_rect.y, depth - 0.00001);
    }
}

pub(super) fn push_trackbar_plaque(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    depth: f32,
) {
    let plaque = trackbar_plaque_rect(rect);
    if let Some(mid) = atlas.trackbar_plaque_mid_trofm {
        push_entry(out, mid, plaque, depth);
    }
    if let Some(left) = atlas.trackbar_plaque_left_trofl {
        push_entry_native(out, left, plaque.x, plaque.y, depth - 0.00001);
    }
    if let Some(right) = atlas.trackbar_plaque_right_trofr {
        let w = right.pixel_size[0].round() as i32;
        push_entry_native(
            out,
            right,
            plaque.x + plaque.w - w,
            plaque.y,
            depth - 0.00001,
        );
    }
}

pub(super) fn trackbar_rect_for_id(layout: &SkirmishShellLayout, id: SkirmishTrackbarId) -> RectPx {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => layout.trackbars.game_speed,
        SkirmishTrackbarId::Credits0x511 => layout.trackbars.credits,
        SkirmishTrackbarId::UnitCount0x50c => layout.trackbars.unit_count,
    }
}

pub(super) fn push_checkbox_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    for checkbox in layout.checkboxes {
        let Some(entry) = checkbox_entry(atlas, checkbox_checked(shell, checkbox.id)) else {
            continue;
        };
        push_entry(
            out,
            entry,
            checkbox_icon_rect(checkbox.rect),
            SHELL_CONTROL_DEPTH,
        );
    }
}

pub(super) fn push_trackbar_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    for id in [
        SkirmishTrackbarId::GameSpeed0x529,
        SkirmishTrackbarId::Credits0x511,
        SkirmishTrackbarId::UnitCount0x50c,
    ] {
        let rect = trackbar_rect_for_id(layout, id);
        if let Some(rail) = atlas.trackbar_rail {
            push_entry_native(out, rail, rect.x, rect.y, SHELL_CONTROL_DEPTH);
        }
        push_trackbar_plaque(out, atlas, rect, SHELL_CONTROL_DEPTH);

        let value = trackbar_visual_value(shell, id);
        let (min, max, step) = match id {
            SkirmishTrackbarId::GameSpeed0x529 => (0, 6, 1),
            SkirmishTrackbarId::Credits0x511 => (5000, 10000, 100),
            SkirmishTrackbarId::UnitCount0x50c => (0, 10, 1),
        };
        let px = trackbar_pixel_offset(value, min, max, step, rect);
        if let Some(thumb) = atlas.trackbar_thumb_trakgrip {
            let thumb_rect = trackbar_thumb_rect(rect, px);
            push_entry(out, thumb, thumb_rect, SHELL_CONTROL_DEPTH - 0.00002);
        }
    }
}

pub(super) fn push_combo_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    color_schemes: &[ColorSchemeEntry],
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    let open = shell.open_combo_dropdown.map(|dropdown| dropdown.id);
    push_combo_face(
        out,
        atlas,
        color_schemes,
        layout.rows.side_combos[0],
        None,
        open == Some(SkirmishComboId::Side(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );
    push_combo_face(
        out,
        atlas,
        color_schemes,
        layout.color_combos[0],
        Some(shell.player_color_index),
        open == Some(SkirmishComboId::Color(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );
    push_combo_face(
        out,
        atlas,
        color_schemes,
        layout.rows.start_combos[0],
        None,
        open == Some(SkirmishComboId::Start(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );
    push_combo_face(
        out,
        atlas,
        color_schemes,
        layout.rows.team_combos[0],
        None,
        open == Some(SkirmishComboId::Team(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );

    for (idx, opponent) in shell.opponents.iter().enumerate() {
        if idx >= layout.rows.ai_type_combos.len() {
            break;
        }
        let row = idx + 1;
        push_combo_face(
            out,
            atlas,
            color_schemes,
            layout.rows.ai_type_combos[idx],
            None,
            open == Some(SkirmishComboId::AiType(idx)),
            false,
            SHELL_CONTROL_DEPTH,
        );
        let sibling_disabled = !opponent.is_active();
        push_combo_face(
            out,
            atlas,
            color_schemes,
            layout.rows.side_combos[row],
            None,
            open == Some(SkirmishComboId::Side(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
        push_combo_face(
            out,
            atlas,
            color_schemes,
            layout.color_combos[row],
            (!sibling_disabled).then_some(opponent.color_index),
            open == Some(SkirmishComboId::Color(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
        push_combo_face(
            out,
            atlas,
            color_schemes,
            layout.rows.start_combos[row],
            None,
            open == Some(SkirmishComboId::Start(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
        push_combo_face(
            out,
            atlas,
            color_schemes,
            layout.rows.team_combos[row],
            None,
            open == Some(SkirmishComboId::Team(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
    }
}

pub(super) fn push_dropdown_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    color_schemes: &[ColorSchemeEntry],
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
) {
    let Some(open) = shell.open_combo_dropdown else {
        return;
    };
    let Some(dropdown) = combo_dropdown_rect(shell, layout, maps, open.id) else {
        return;
    };
    let content = combo_dropdown_content_rect(shell, layout, maps, open.id).unwrap_or(dropdown);
    let needs_scrollbar = combo_dropdown_needs_scrollbar(shell, maps, open.id);
    push_solid_rect(
        out,
        atlas,
        dropdown,
        SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE,
        SHELL_DROPDOWN_DEPTH,
    );
    let visible_rows = combo_dropdown_visible_row_count(shell, maps, open.id);
    if let Some(selected_rect) =
        dropdown_selected_row_rect(shell, maps, open.id, open.top_index, content)
    {
        push_solid_rect(
            out,
            atlas,
            selected_rect,
            OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF,
            SHELL_DROPDOWN_DEPTH - 0.00001,
        );
    }
    for (idx, item) in combo_items(shell, maps, open.id)
        .into_iter()
        .skip(open.top_index)
        .take(visible_rows)
        .enumerate()
    {
        if let SkirmishComboItem::Color(color_index) = item {
            let row_y = content.y + idx as i32 * COMBO_DROPDOWN_ROW_H;
            let swatch = RectPx::new(content.x + 2, row_y + 2, content.w - 4, 19);
            push_solid_rect(
                out,
                atlas,
                swatch,
                house_color_tint(color_schemes, color_index),
                SHELL_DROPDOWN_DEPTH - 0.00002,
            );
        }
    }
    if needs_scrollbar {
        if let (Some(scrollbar), Some(thumb)) = (
            combo_dropdown_scrollbar_rect(shell, layout, maps, open.id),
            combo_dropdown_scroll_thumb_rect(shell, layout, maps, open.id),
        ) {
            let pressed_part = shell
                .dropdown_scroll_press
                .filter(|pressed| pressed.id == open.id)
                .map(|pressed| pressed.part);
            push_dropdown_scrollbar_instances(out, atlas, scrollbar, thumb, pressed_part);
        }
    }
    push_ownerdraw_two_pixel_bevel_frame(out, atlas, dropdown, SHELL_DROPDOWN_DEPTH - 0.00003);
}

pub(super) fn dropdown_selected_row_rect(
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    top_index: usize,
    content: RectPx,
) -> Option<RectPx> {
    let visible_rows = combo_dropdown_visible_row_count(shell, maps, id);
    let selected = selected_combo_item_index(shell, maps, id)?;
    if selected < top_index || selected >= top_index + visible_rows {
        return None;
    }
    Some(RectPx::new(
        content.x,
        content.y + (selected - top_index) as i32 * COMBO_DROPDOWN_ROW_H,
        content.w,
        COMBO_DROPDOWN_ROW_H,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::color_scheme::backing_rgb_for_priority;

    /// The reachable entries of the retail rulesmd `[Colors]` list, in order — same
    /// fixture rationale as the color_scheme.rs tests. Slot/priority indices land on
    /// the scattered scheme entries via the priority LUT + doubling.
    fn retail_schemes() -> Vec<ColorSchemeEntry> {
        let raw: &[(&str, [u8; 3])] = &[
            ("LightGold", [25, 255, 255]),
            ("Gold", [43, 239, 255]),
            ("LightGrey", [0, 0, 240]),
            ("Grey", [0, 0, 131]),
            ("Red", [20, 255, 184]),
            ("DarkRed", [0, 230, 255]),
            ("Orange", [25, 230, 255]),
            ("Magenta", [221, 102, 255]),
            ("Purple", [201, 201, 189]),
            ("LightBlue", [119, 143, 255]),
            ("DarkBlue", [153, 214, 212]),
            ("NeonBlue", [185, 156, 238]),
            ("DarkSky", [131, 200, 230]),
            ("Green", [104, 241, 195]),
            ("DarkGreen", [81, 200, 210]),
        ];
        raw.iter()
            .map(|(name, hsv)| ColorSchemeEntry {
                name: name.to_string(),
                hsv: *hsv,
            })
            .collect()
    }

    #[test]
    fn swatch_matches_loading_backing_for_every_slot() {
        // The lobby swatch and the loading-screen progress-bar backing must agree
        // color-for-color for each of the 8 slots — this is the parity that broke.
        let schemes = retail_schemes();
        for slot in 0..8usize {
            let rgb = backing_rgb_for_priority(&schemes, slot as i32).unwrap();
            let expected = [
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
            ];
            assert_eq!(house_color_tint(&schemes, slot), expected, "slot {slot}");
        }
    }

    #[test]
    fn slot_one_is_red_slot_two_is_blue() {
        // Priority order: slot 1 = DarkRed (red-dominant), slot 2 = DarkBlue
        // (blue-dominant) — the reverse of the old SCHEME_BASES ordering.
        let schemes = retail_schemes();
        let red = house_color_tint(&schemes, 1);
        assert!(red[0] > red[1] && red[0] > red[2], "slot 1 red-dominant: {red:?}");
        let blue = house_color_tint(&schemes, 2);
        assert!(blue[2] > blue[0] && blue[2] > blue[1], "slot 2 blue-dominant: {blue:?}");
    }

    #[test]
    fn empty_schemes_fall_back_to_legacy_ramp() {
        // Defensive path: with no [Colors] loaded the swatch still renders a color.
        let tint = house_color_tint(&[], 0);
        assert!(tint.iter().any(|&channel| channel > 0.0));
    }
}
