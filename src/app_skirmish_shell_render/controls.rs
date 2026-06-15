//! Owner-draw control sprite helpers for the skirmish shell renderer.
//!
//! Contains player-name edit chrome, checkboxes, trackbars, combo faces,
//! and ComboDropWin popup sprite construction.

use crate::app_init::MapMenuEntry;
use crate::render::batch::SpriteInstance;
use crate::render::bit_font::BitFont;
use crate::render::skirmish_shell_chrome::{
    ControlChrome, SkirmishShellChromeAtlas, SkirmishShellChromeEntry,
};
use crate::rules::color_scheme::{ColorSchemeEntry, hsv_to_rgb, scheme_for_priority};
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
    push_entry, push_entry_native, push_ownerdraw_two_pixel_bevel_frame,
    push_ownerdraw_two_pixel_bevel_frame_px, push_solid_rect, push_solid_rect_px,
    push_tinted_entry,
};
use super::{
    OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF, SHELL_CONTROL_DEPTH,
    SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE, SHELL_DROPDOWN_DEPTH,
    SHELL_EDIT_CARET_DEPTH, SHELL_EDIT_FRAME_DEPTH, SHELL_EDIT_SELECTION_DEPTH,
    SHELL_LABEL_TEXT_RGB, SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
    SHELL_SWATCH_DEPTH,
};

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
    chrome: &ControlChrome,
    rect: RectPx,
) -> Option<SkirmishShellChromeEntry> {
    match rect.w {
        150 => chrome.combo_face_150,
        117 => chrome.combo_face_117,
        44 => chrome.combo_face_44,
        38 => chrome.combo_face_38,
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
    chrome: &ControlChrome,
    rect: RectPx,
    depth: f32,
) {
    let top_h = chrome
        .scrollbar_thumb_top
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    let bottom_h = chrome
        .scrollbar_thumb_bottom
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    if let Some(top) = chrome.scrollbar_thumb_top {
        push_entry_native(out, top, rect.x, rect.y, depth);
    }
    if let Some(bottom) = chrome.scrollbar_thumb_bottom {
        push_entry_native(out, bottom, rect.x, rect.y + rect.h - bottom_h, depth);
    }
    if let Some(mid) = chrome.scrollbar_thumb_mid {
        let mid_y = rect.y + top_h;
        let mid_h = rect.h - top_h - bottom_h;
        if mid_h > 0 {
            push_entry(out, mid, RectPx::new(rect.x, mid_y, rect.w, mid_h), depth);
        }
    }
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
    // No [Colors] loaded yet (rules not ready): neutral grey so the swatch still
    // draws rather than going black. A populated lobby always hits the path above.
    [0.5, 0.5, 0.5]
}

pub(super) fn paint_trackbar_plaque(
    out: &mut Vec<SpriteInstance>,
    chrome: &ControlChrome,
    rect: RectPx,
    depth: f32,
) {
    let plaque = trackbar_plaque_rect(rect);
    if let Some(mid) = chrome.trackbar_plaque_mid_trofm {
        push_entry(out, mid, plaque, depth);
    }
    if let Some(left) = chrome.trackbar_plaque_left_trofl {
        push_entry_native(out, left, plaque.x, plaque.y, depth - 0.00001);
    }
    if let Some(right) = chrome.trackbar_plaque_right_trofr {
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

/// Render-side per-control paint state for the Slice 4 dispatch seam (O1: a
/// data-enum match like `ButtonPolicy`, NOT a trait). One variant per painted
/// control kind (mirrors `ui::shell::ControlKind`); each carries only the RESOLVED
/// per-control STATE (a checkbox's checked flag, a trackbar's thumb pixel offset) +
/// its rect — `paint_control` looks the glyphs up itself from the `ControlChrome`
/// subset (the atlas-taking seam shape). App-layer because the emitters + skirmish
/// layout live here (`render/` must not depend on the app layer, so the seam can't
/// sit in `render/shell_paint.rs` as the original draft assumed). 4A lands
/// `Checkbox`; 4B adds `Trackbar`; 4C adds the collapsed `Combo` face; 4D adds the
/// shared `ScrollBar` (combo dropdown + choose-map listbox); later steps add listbox arms.
pub(super) enum ControlPaint {
    Checkbox {
        checked: bool,
        rect: RectPx,
    },
    Trackbar {
        rect: RectPx,
        thumb_px: i32,
    },
    Combo {
        rect: RectPx,
        swatch: Option<[f32; 3]>,
        open: bool,
        disabled: bool,
    },
    ScrollBar {
        scrollbar: RectPx,
        thumb: RectPx,
        pressed_part: Option<DropdownScrollbarPart>,
    },
    /// Active in-game Options (0xBBB) owner-draw button — SIDEBTTN type 2, drawn at
    /// the SHP native 125x25 size. `rect` is the already-anchored top-left from the
    /// layout pass; `frame` is the resolved SIDEBTTN frame (0 released / 1 pressed /
    /// 2 flash).
    Button {
        rect: RectPx,
        frame: u8,
    },
}

/// Slice 4 paint seam: emit one control, selecting the emitter by `ControlPaint`
/// variant (the per-control-kind dispatch) and resolving its glyphs from the
/// `ControlChrome` subset. Re-homes what were direct per-family emitter calls into
/// one match; emission ORDER (entry / rect / depth) is byte-identical to the
/// pre-seam code.
pub(super) fn paint_control(
    out: &mut Vec<SpriteInstance>,
    chrome: &ControlChrome,
    paint: ControlPaint,
) {
    match paint {
        ControlPaint::Checkbox { checked, rect } => {
            let icon = if checked {
                chrome.checkbox_checked_cce_i
            } else {
                chrome.checkbox_unchecked_cue_i
            };
            if let Some(entry) = icon {
                push_entry(out, entry, checkbox_icon_rect(rect), SHELL_CONTROL_DEPTH);
            }
        }
        ControlPaint::Trackbar { rect, thumb_px } => {
            // Emission order preserved byte-for-byte: rail → plaque(mid, left,
            // right) → thumb, at the same depths as the pre-seam emitter.
            if let Some(rail) = chrome.trackbar_rail {
                push_entry_native(out, rail, rect.x, rect.y, SHELL_CONTROL_DEPTH);
            }
            paint_trackbar_plaque(out, chrome, rect, SHELL_CONTROL_DEPTH);
            if let Some(thumb) = chrome.trackbar_thumb_trakgrip {
                let thumb_rect = trackbar_thumb_rect(rect, thumb_px);
                push_entry(out, thumb, thumb_rect, SHELL_CONTROL_DEPTH - 0.00002);
            }
        }
        ControlPaint::Combo {
            rect,
            swatch,
            open,
            disabled,
        } => {
            // Emission order preserved byte-for-byte from the pre-seam
            // push_combo_face: face glyph → swatch (color faces only) → arrow, at
            // the same depths. The caller resolves the swatch RGB so the arm stays
            // chrome-only.
            if let Some(face) = combo_face_entry(chrome, rect) {
                push_entry(out, face, combo_face_rect(rect), SHELL_CONTROL_DEPTH);
            }
            if let (Some(tint), Some(white)) = (swatch, chrome.white_pixel) {
                push_tinted_entry(
                    out,
                    white,
                    combo_swatch_rect(rect),
                    tint,
                    SHELL_SWATCH_DEPTH,
                );
            }
            let arrow = match (disabled, open) {
                (true, true) => chrome.combo_arrow_down_gray_pressed,
                (true, false) => chrome.combo_arrow_down_gray_released,
                (false, true) => chrome.combo_arrow_down_pressed,
                (false, false) => chrome.combo_arrow_down_released,
            };
            if let Some(arrow) = arrow {
                let arrow_rect = combo_arrow_rect(rect);
                push_entry_native(
                    out,
                    arrow,
                    arrow_rect.x,
                    arrow_rect.y,
                    SHELL_CONTROL_DEPTH - 0.00001,
                );
            }
        }
        ControlPaint::ScrollBar {
            scrollbar,
            thumb,
            pressed_part,
        } => {
            // Byte-for-byte the pre-seam push_dropdown_scrollbar_instances: track fill
            // → up arrow → down arrow → thumb(top/mid/bottom) → bevel frame, at the
            // hardcoded SHELL_DROPDOWN_DEPTH offsets the shared emitter used for BOTH
            // the combo popup and the choose-map listbox.
            push_solid_rect_px(
                out,
                chrome.white_pixel,
                scrollbar,
                SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
                SHELL_DROPDOWN_DEPTH - 0.00004,
            );
            let up_entry = scrollbar_arrow_entry(
                chrome.scrollbar_arrow_up_released,
                chrome.scrollbar_arrow_up_pressed,
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
                chrome.scrollbar_arrow_down_released,
                chrome.scrollbar_arrow_down_pressed,
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
            push_scrollbar_thumb(out, chrome, thumb, SHELL_DROPDOWN_DEPTH - 0.00006);
            push_ownerdraw_two_pixel_bevel_frame_px(
                out,
                chrome.white_pixel,
                scrollbar,
                SHELL_DROPDOWN_DEPTH - 0.00007,
            );
        }
        ControlPaint::Button { rect, frame } => {
            // Active 0xBBB owner-draw button: one SIDEBTTN glyph at its native
            // 125x25 size, at the anchored top-left. `entry` is `Copy` and
            // push_entry_native takes it by value (match the other arms).
            let entry = match frame {
                1 => chrome.options_button_sidebttn_frame1,
                2 => chrome.options_button_sidebttn_frame2,
                _ => chrome.options_button_sidebttn_frame0,
            };
            if let Some(entry) = entry {
                push_entry_native(out, entry, rect.x, rect.y, SHELL_CONTROL_DEPTH);
            }
        }
    }
}

/// SIDEBTTN type-2 button frame: released 0, pressed 1. Frame 2 is the
/// timer-driven flash/checked state (record `0xC5`), deferred until the
/// button-flash timer is modeled — 5a-ii renders released/pressed only.
pub(super) fn options_button_sidebttn_frame_index(pressed: bool) -> u8 {
    if pressed { 1 } else { 0 }
}

pub(super) fn push_checkbox_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    // Slice 4A/4B: each checkbox paints through the per-control-kind seam, which
    // resolves the checked/unchecked icon from the `ControlChrome` subset. The
    // iteration order (layout.checkboxes), the icon rect, and SHELL_CONTROL_DEPTH
    // are unchanged from the pre-seam loop — byte-identical emission (see the
    // draw-list test).
    let chrome = atlas.control_chrome();
    for checkbox in layout.checkboxes {
        paint_control(
            out,
            &chrome,
            ControlPaint::Checkbox {
                checked: checkbox_checked(shell, checkbox.id),
                rect: checkbox.rect,
            },
        );
    }
}

pub(super) fn push_trackbar_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    // Slice 4B: each trackbar paints through the per-control-kind seam. The
    // value→pixel quantization (reading bounds) stays here in the skirmish layer;
    // only the resolved `thumb_px` + rect cross the seam, and `paint_control`
    // resolves the rail/plaque/thumb glyphs from the `ControlChrome` subset.
    let chrome = atlas.control_chrome();
    for id in [
        SkirmishTrackbarId::GameSpeed0x529,
        SkirmishTrackbarId::Credits0x511,
        SkirmishTrackbarId::UnitCount0x50c,
    ] {
        let rect = trackbar_rect_for_id(layout, id);
        let value = trackbar_visual_value(shell, id);
        let (min, max, step) = shell.trackbar_bounds.range(id);
        let px = trackbar_pixel_offset(value, min, max, step, rect);
        paint_control(out, &chrome, ControlPaint::Trackbar { rect, thumb_px: px });
    }
}

pub(super) fn push_combo_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    color_schemes: &[ColorSchemeEntry],
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    // Slice 4C: each collapsed combo face paints through the per-control-kind seam,
    // which resolves the face glyph / swatch white-pixel / arrow variant from the
    // `ControlChrome` subset. The skirmish layer keeps the per-face color-index
    // resolution and pre-resolves it to a swatch RGB so the arm stays chrome-only;
    // iteration order, rects, and SHELL_CONTROL_DEPTH are byte-identical to the
    // pre-seam push_combo_face loop (see the draw-list test). The open dropdown
    // popup + shared scrollbar stay on `atlas` (4D).
    let chrome = atlas.control_chrome();
    let open = shell.open_combo_dropdown.map(|dropdown| dropdown.id);
    let swatch_for =
        |color_index: Option<usize>| color_index.map(|ci| house_color_tint(color_schemes, ci));
    paint_control(
        out,
        &chrome,
        ControlPaint::Combo {
            rect: layout.rows.side_combos[0],
            swatch: None,
            open: open == Some(SkirmishComboId::Side(0)),
            disabled: false,
        },
    );
    paint_control(
        out,
        &chrome,
        ControlPaint::Combo {
            rect: layout.color_combos[0],
            swatch: swatch_for(Some(shell.player_color_index)),
            open: open == Some(SkirmishComboId::Color(0)),
            disabled: false,
        },
    );
    paint_control(
        out,
        &chrome,
        ControlPaint::Combo {
            rect: layout.rows.start_combos[0],
            swatch: None,
            open: open == Some(SkirmishComboId::Start(0)),
            disabled: false,
        },
    );
    paint_control(
        out,
        &chrome,
        ControlPaint::Combo {
            rect: layout.rows.team_combos[0],
            swatch: None,
            open: open == Some(SkirmishComboId::Team(0)),
            disabled: false,
        },
    );

    for (idx, opponent) in shell.opponents.iter().enumerate() {
        if idx >= layout.rows.ai_type_combos.len() {
            break;
        }
        let row = idx + 1;
        paint_control(
            out,
            &chrome,
            ControlPaint::Combo {
                rect: layout.rows.ai_type_combos[idx],
                swatch: None,
                open: open == Some(SkirmishComboId::AiType(idx)),
                disabled: false,
            },
        );
        let sibling_disabled = !opponent.is_active();
        paint_control(
            out,
            &chrome,
            ControlPaint::Combo {
                rect: layout.rows.side_combos[row],
                swatch: None,
                open: open == Some(SkirmishComboId::Side(row)),
                disabled: sibling_disabled,
            },
        );
        paint_control(
            out,
            &chrome,
            ControlPaint::Combo {
                rect: layout.color_combos[row],
                swatch: swatch_for((!sibling_disabled).then_some(opponent.color_index)),
                open: open == Some(SkirmishComboId::Color(row)),
                disabled: sibling_disabled,
            },
        );
        paint_control(
            out,
            &chrome,
            ControlPaint::Combo {
                rect: layout.rows.start_combos[row],
                swatch: None,
                open: open == Some(SkirmishComboId::Start(row)),
                disabled: sibling_disabled,
            },
        );
        paint_control(
            out,
            &chrome,
            ControlPaint::Combo {
                rect: layout.rows.team_combos[row],
                swatch: None,
                open: open == Some(SkirmishComboId::Team(row)),
                disabled: sibling_disabled,
            },
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
            let chrome = atlas.control_chrome();
            paint_control(
                out,
                &chrome,
                ControlPaint::ScrollBar {
                    scrollbar,
                    thumb,
                    pressed_part,
                },
            );
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
        // (blue-dominant), resolved from the [Colors] HSV via the priority LUT.
        let schemes = retail_schemes();
        let red = house_color_tint(&schemes, 1);
        assert!(
            red[0] > red[1] && red[0] > red[2],
            "slot 1 red-dominant: {red:?}"
        );
        let blue = house_color_tint(&schemes, 2);
        assert!(
            blue[2] > blue[0] && blue[2] > blue[1],
            "slot 2 blue-dominant: {blue:?}"
        );
    }

    #[test]
    fn empty_schemes_fall_back_to_neutral_grey() {
        // Defensive path: with no [Colors] loaded the swatch still renders a color.
        let tint = house_color_tint(&[], 0);
        assert!(tint.iter().any(|&channel| channel > 0.0));
    }

    #[test]
    fn checkbox_paint_seam_emits_icon_at_icon_rect_with_control_depth() {
        // Draw-list assertion (Slice 4 §1.4): the per-control-kind seam emits
        // exactly one instance, at the 18x18 icon rect, at SHELL_CONTROL_DEPTH,
        // carrying the resolved checked-icon entry's uv — byte-identical to the
        // pre-seam push_checkbox_instances emission. The seam resolves the icon
        // from the ControlChrome subset itself (the atlas-taking seam shape).
        let entry = SkirmishShellChromeEntry {
            uv_origin: [0.1, 0.2],
            uv_size: [0.3, 0.4],
            pixel_size: [18.0, 18.0],
        };
        let chrome = ControlChrome {
            checkbox_checked_cce_i: Some(entry),
            ..Default::default()
        };
        let rect = RectPx::new(71, 286, 150, 16);
        let icon = checkbox_icon_rect(rect);

        let mut out = Vec::new();
        paint_control(
            &mut out,
            &chrome,
            ControlPaint::Checkbox {
                checked: true,
                rect,
            },
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position, [icon.x as f32, icon.y as f32]);
        assert_eq!(out[0].size, [icon.w as f32, icon.h as f32]);
        assert_eq!(out[0].uv_origin, entry.uv_origin);
        assert_eq!(out[0].uv_size, entry.uv_size);
        assert_eq!(out[0].depth, SHELL_CONTROL_DEPTH);

        // Missing icon → no instance (matches the pre-seam `else continue`).
        let mut empty = Vec::new();
        paint_control(
            &mut empty,
            &ControlChrome::default(),
            ControlPaint::Checkbox {
                checked: true,
                rect,
            },
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn options_button_frame_index_released_pressed() {
        assert_eq!(options_button_sidebttn_frame_index(false), 0);
        assert_eq!(options_button_sidebttn_frame_index(true), 1);
    }

    #[test]
    fn options_button_paint_seam_emits_sidebttn_frame_at_rect() {
        // Draw-list assertion: the 0xBBB owner-draw button arm emits exactly one
        // SIDEBTTN glyph at its native size, at the anchored top-left, at
        // SHELL_CONTROL_DEPTH. Frame selects by the resolved index (0/1/2).
        let entry = SkirmishShellChromeEntry {
            uv_origin: [0.10, 0.20],
            uv_size: [0.30, 0.40],
            pixel_size: [125.0, 25.0],
        };
        let chrome = ControlChrome {
            options_button_sidebttn_frame0: Some(entry),
            ..Default::default()
        };
        let rect = RectPx::new(653, 198, 125, 25);

        let mut out = Vec::new();
        paint_control(&mut out, &chrome, ControlPaint::Button { rect, frame: 0 });
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position, [653.0, 198.0]);
        assert_eq!(out[0].size, entry.pixel_size);
        assert_eq!(out[0].uv_origin, entry.uv_origin);
        assert_eq!(out[0].uv_size, entry.uv_size);
        assert_eq!(out[0].depth, SHELL_CONTROL_DEPTH);

        // A frame whose entry isn't loaded → no instance (only frame 0 is set here).
        let mut missing = Vec::new();
        paint_control(
            &mut missing,
            &chrome,
            ControlPaint::Button { rect, frame: 1 },
        );
        assert!(missing.is_empty());

        // Empty chrome → no instance.
        let mut empty = Vec::new();
        paint_control(
            &mut empty,
            &ControlChrome::default(),
            ControlPaint::Button { rect, frame: 0 },
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn trackbar_paint_seam_emits_rail_plaque_thumb_in_native_order() {
        // Draw-list assertion (Slice 4 §1.4): the trackbar seam emits rail →
        // plaque(mid, left, right) → thumb in the exact order/positions/depths of
        // the pre-seam push_trackbar_instances emitter, across min/mid/max thumb
        // offsets. Geometry is pinned via the same layout helpers the emitter uses,
        // so the assertion tracks the helpers rather than hardcoding plaque math.
        let rail = SkirmishShellChromeEntry {
            uv_origin: [0.01, 0.02],
            uv_size: [0.03, 0.04],
            pixel_size: [200.0, 18.0],
        };
        let mid = SkirmishShellChromeEntry {
            uv_origin: [0.11, 0.12],
            uv_size: [0.13, 0.14],
            pixel_size: [10.0, 16.0],
        };
        let left = SkirmishShellChromeEntry {
            uv_origin: [0.21, 0.22],
            uv_size: [0.23, 0.24],
            pixel_size: [6.0, 16.0],
        };
        let right = SkirmishShellChromeEntry {
            uv_origin: [0.31, 0.32],
            uv_size: [0.33, 0.34],
            pixel_size: [7.0, 16.0],
        };
        let thumb = SkirmishShellChromeEntry {
            uv_origin: [0.41, 0.42],
            uv_size: [0.43, 0.44],
            pixel_size: [12.0, 18.0],
        };
        let chrome = ControlChrome {
            trackbar_rail: Some(rail),
            trackbar_plaque_mid_trofm: Some(mid),
            trackbar_plaque_left_trofl: Some(left),
            trackbar_plaque_right_trofr: Some(right),
            trackbar_thumb_trakgrip: Some(thumb),
            ..Default::default()
        };
        let rect = RectPx::new(176, 168, 200, 24);
        let plaque = trackbar_plaque_rect(rect);

        for thumb_px in [0, 68, 137] {
            let mut out = Vec::new();
            paint_control(&mut out, &chrome, ControlPaint::Trackbar { rect, thumb_px });
            assert_eq!(out.len(), 5, "rail + plaque(mid, left, right) + thumb");

            // 0: rail — native at the control origin, SHELL_CONTROL_DEPTH.
            assert_eq!(out[0].position, [rect.x as f32, rect.y as f32]);
            assert_eq!(out[0].size, rail.pixel_size);
            assert_eq!(out[0].uv_origin, rail.uv_origin);
            assert_eq!(out[0].depth, SHELL_CONTROL_DEPTH);

            // 1: plaque mid — scaled to the plaque rect, same depth.
            assert_eq!(out[1].position, [plaque.x as f32, plaque.y as f32]);
            assert_eq!(out[1].size, [plaque.w as f32, plaque.h as f32]);
            assert_eq!(out[1].uv_origin, mid.uv_origin);
            assert_eq!(out[1].depth, SHELL_CONTROL_DEPTH);

            // 2: plaque left — native at the plaque origin, one tick under.
            assert_eq!(out[2].position, [plaque.x as f32, plaque.y as f32]);
            assert_eq!(out[2].size, left.pixel_size);
            assert_eq!(out[2].uv_origin, left.uv_origin);
            assert_eq!(out[2].depth, SHELL_CONTROL_DEPTH - 0.00001);

            // 3: plaque right — native, right-aligned in the plaque, one tick under.
            let right_w = right.pixel_size[0].round() as i32;
            assert_eq!(
                out[3].position,
                [(plaque.x + plaque.w - right_w) as f32, plaque.y as f32]
            );
            assert_eq!(out[3].size, right.pixel_size);
            assert_eq!(out[3].uv_origin, right.uv_origin);
            assert_eq!(out[3].depth, SHELL_CONTROL_DEPTH - 0.00001);

            // 4: thumb — follows thumb_px via trackbar_thumb_rect, two ticks under.
            let thumb_rect = trackbar_thumb_rect(rect, thumb_px);
            assert_eq!(out[4].position, [thumb_rect.x as f32, thumb_rect.y as f32]);
            assert_eq!(out[4].size, [thumb_rect.w as f32, thumb_rect.h as f32]);
            assert_eq!(out[4].uv_origin, thumb.uv_origin);
            assert_eq!(out[4].depth, SHELL_CONTROL_DEPTH - 0.00002);
        }

        // Missing entries → nothing emitted (matches the pre-seam `if let` guards).
        let mut empty = Vec::new();
        paint_control(
            &mut empty,
            &ControlChrome::default(),
            ControlPaint::Trackbar { rect, thumb_px: 0 },
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn combo_face_paint_seam_emits_face_swatch_arrow() {
        // Draw-list assertion (Slice 4 §1.4): the collapsed combo seam emits face
        // glyph → swatch (color faces only) → arrow variant, in the exact
        // order/positions/depths of the pre-seam push_combo_face emitter, across the
        // four face shapes — (a) color open, (b) color closed, (c) plain, (d)
        // disabled. Geometry is pinned via the same layout helpers the emitter uses.
        let face = SkirmishShellChromeEntry {
            uv_origin: [0.01, 0.02],
            uv_size: [0.03, 0.04],
            pixel_size: [150.0, 24.0],
        };
        let white = SkirmishShellChromeEntry {
            uv_origin: [0.11, 0.12],
            uv_size: [0.13, 0.14],
            pixel_size: [1.0, 1.0],
        };
        let arrow_released = SkirmishShellChromeEntry {
            uv_origin: [0.21, 0.22],
            uv_size: [0.23, 0.24],
            pixel_size: [16.0, 16.0],
        };
        let arrow_pressed = SkirmishShellChromeEntry {
            uv_origin: [0.31, 0.32],
            uv_size: [0.33, 0.34],
            pixel_size: [16.0, 16.0],
        };
        let arrow_gray_released = SkirmishShellChromeEntry {
            uv_origin: [0.41, 0.42],
            uv_size: [0.43, 0.44],
            pixel_size: [16.0, 16.0],
        };
        let arrow_gray_pressed = SkirmishShellChromeEntry {
            uv_origin: [0.51, 0.52],
            uv_size: [0.53, 0.54],
            pixel_size: [16.0, 16.0],
        };
        let chrome = ControlChrome {
            combo_face_150: Some(face),
            white_pixel: Some(white),
            combo_arrow_down_released: Some(arrow_released),
            combo_arrow_down_pressed: Some(arrow_pressed),
            combo_arrow_down_gray_released: Some(arrow_gray_released),
            combo_arrow_down_gray_pressed: Some(arrow_gray_pressed),
            ..Default::default()
        };
        // w=150 selects combo_face_150 via combo_face_entry.
        let rect = RectPx::new(176, 100, 150, 24);
        let face_rect = combo_face_rect(rect);
        let swatch_rect = combo_swatch_rect(rect);
        let arrow_rect = combo_arrow_rect(rect);
        let tint = [0.25, 0.5, 0.75];

        let assert_face = |inst: &SpriteInstance| {
            assert_eq!(inst.position, [face_rect.x as f32, face_rect.y as f32]);
            assert_eq!(inst.size, [face_rect.w as f32, face_rect.h as f32]);
            assert_eq!(inst.uv_origin, face.uv_origin);
            assert_eq!(inst.depth, SHELL_CONTROL_DEPTH);
        };
        let assert_arrow = |inst: &SpriteInstance, entry: SkirmishShellChromeEntry| {
            assert_eq!(inst.position, [arrow_rect.x as f32, arrow_rect.y as f32]);
            assert_eq!(inst.size, entry.pixel_size);
            assert_eq!(inst.uv_origin, entry.uv_origin);
            assert_eq!(inst.depth, SHELL_CONTROL_DEPTH - 0.00001);
        };

        // (a) color face open: face → swatch → pressed arrow.
        let mut a = Vec::new();
        paint_control(
            &mut a,
            &chrome,
            ControlPaint::Combo {
                rect,
                swatch: Some(tint),
                open: true,
                disabled: false,
            },
        );
        assert_eq!(a.len(), 3, "face + swatch + arrow");
        assert_face(&a[0]);
        assert_eq!(a[1].position, [swatch_rect.x as f32, swatch_rect.y as f32]);
        assert_eq!(a[1].size, [swatch_rect.w as f32, swatch_rect.h as f32]);
        assert_eq!(a[1].uv_origin, white.uv_origin);
        assert_eq!(a[1].tint, tint);
        assert_eq!(a[1].depth, SHELL_SWATCH_DEPTH);
        assert_arrow(&a[2], arrow_pressed);

        // (b) color face closed: face → swatch → released arrow.
        let mut b = Vec::new();
        paint_control(
            &mut b,
            &chrome,
            ControlPaint::Combo {
                rect,
                swatch: Some(tint),
                open: false,
                disabled: false,
            },
        );
        assert_eq!(b.len(), 3);
        assert_face(&b[0]);
        assert_eq!(b[1].tint, tint);
        assert_eq!(b[1].depth, SHELL_SWATCH_DEPTH);
        assert_arrow(&b[2], arrow_released);

        // (c) plain face (no swatch): face → released arrow.
        let mut c = Vec::new();
        paint_control(
            &mut c,
            &chrome,
            ControlPaint::Combo {
                rect,
                swatch: None,
                open: false,
                disabled: false,
            },
        );
        assert_eq!(c.len(), 2, "face + arrow, no swatch");
        assert_face(&c[0]);
        assert_arrow(&c[1], arrow_released);

        // (d) disabled face (no swatch): face → gray released arrow.
        let mut d = Vec::new();
        paint_control(
            &mut d,
            &chrome,
            ControlPaint::Combo {
                rect,
                swatch: None,
                open: false,
                disabled: true,
            },
        );
        assert_eq!(d.len(), 2);
        assert_face(&d[0]);
        assert_arrow(&d[1], arrow_gray_released);

        // Empty chrome (no glyphs) → nothing emitted.
        let mut empty = Vec::new();
        paint_control(
            &mut empty,
            &ControlChrome::default(),
            ControlPaint::Combo {
                rect,
                swatch: Some(tint),
                open: true,
                disabled: false,
            },
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn scrollbar_paint_seam_emits_track_arrows_thumb_bevel() {
        // Draw-list assertion (Slice 4 §1.4): the ScrollBar arm reproduces the
        // pre-seam push_dropdown_scrollbar_instances sequence — track fill →
        // up arrow → down arrow → thumb(top/bottom/mid) → 2-ring bevel frame — at
        // the hardcoded SHELL_DROPDOWN_DEPTH offsets, for both pressed states.
        let white = SkirmishShellChromeEntry {
            uv_origin: [0.01, 0.02],
            uv_size: [0.03, 0.04],
            pixel_size: [1.0, 1.0],
        };
        let up_r = SkirmishShellChromeEntry {
            uv_origin: [0.11, 0.12],
            uv_size: [0.13, 0.14],
            pixel_size: [20.0, 22.0],
        };
        let up_p = SkirmishShellChromeEntry {
            uv_origin: [0.21, 0.22],
            uv_size: [0.23, 0.24],
            pixel_size: [20.0, 22.0],
        };
        let dn_r = SkirmishShellChromeEntry {
            uv_origin: [0.31, 0.32],
            uv_size: [0.33, 0.34],
            pixel_size: [20.0, 22.0],
        };
        let dn_p = SkirmishShellChromeEntry {
            uv_origin: [0.41, 0.42],
            uv_size: [0.43, 0.44],
            pixel_size: [20.0, 22.0],
        };
        let th_t = SkirmishShellChromeEntry {
            uv_origin: [0.51, 0.52],
            uv_size: [0.53, 0.54],
            pixel_size: [20.0, 6.0],
        };
        let th_m = SkirmishShellChromeEntry {
            uv_origin: [0.61, 0.62],
            uv_size: [0.63, 0.64],
            pixel_size: [20.0, 4.0],
        };
        let th_b = SkirmishShellChromeEntry {
            uv_origin: [0.71, 0.72],
            uv_size: [0.73, 0.74],
            pixel_size: [20.0, 6.0],
        };
        let chrome = ControlChrome {
            white_pixel: Some(white),
            scrollbar_arrow_up_released: Some(up_r),
            scrollbar_arrow_up_pressed: Some(up_p),
            scrollbar_arrow_down_released: Some(dn_r),
            scrollbar_arrow_down_pressed: Some(dn_p),
            scrollbar_thumb_top: Some(th_t),
            scrollbar_thumb_mid: Some(th_m),
            scrollbar_thumb_bottom: Some(th_b),
            ..Default::default()
        };
        // scrollbar tall enough for top+bottom thumb caps + a positive mid span.
        let scrollbar = RectPx::new(300, 100, 20, 120);
        let thumb = RectPx::new(300, 144, 20, 30);

        // Default (no pressed part): both arrows show the RELEASED glyph.
        let mut out = Vec::new();
        paint_control(
            &mut out,
            &chrome,
            ControlPaint::ScrollBar {
                scrollbar,
                thumb,
                pressed_part: None,
            },
        );
        // track(1) + up(1) + down(1) + thumb top/bottom/mid(3) + 2 bevel rings × 4
        // edges each (each ring: top/left/bottom/right solid rects) = 8.
        assert_eq!(out.len(), 14, "track + 2 arrows + 3 thumb + 8 bevel edges");

        // 0: track fill — white pixel tinted, scrollbar rect, DEPTH-0.00004.
        assert_eq!(out[0].position, [scrollbar.x as f32, scrollbar.y as f32]);
        assert_eq!(out[0].size, [scrollbar.w as f32, scrollbar.h as f32]);
        assert_eq!(out[0].uv_origin, white.uv_origin);
        assert_eq!(out[0].depth, SHELL_DROPDOWN_DEPTH - 0.00004);

        // 1: up arrow — released, native at scrollbar origin, DEPTH-0.00005.
        assert_eq!(out[1].position, [scrollbar.x as f32, scrollbar.y as f32]);
        assert_eq!(out[1].uv_origin, up_r.uv_origin);
        assert_eq!(out[1].depth, SHELL_DROPDOWN_DEPTH - 0.00005);

        // 2: down arrow — released, native at the bottom button slot, DEPTH-0.00005.
        assert_eq!(
            out[2].position,
            [
                scrollbar.x as f32,
                (scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H) as f32
            ]
        );
        assert_eq!(out[2].uv_origin, dn_r.uv_origin);
        assert_eq!(out[2].depth, SHELL_DROPDOWN_DEPTH - 0.00005);

        // 3: thumb top — native at thumb origin, DEPTH-0.00006.
        assert_eq!(out[3].position, [thumb.x as f32, thumb.y as f32]);
        assert_eq!(out[3].uv_origin, th_t.uv_origin);
        assert_eq!(out[3].depth, SHELL_DROPDOWN_DEPTH - 0.00006);

        // 4: thumb bottom — native, bottom-aligned in the thumb rect.
        let bottom_h = th_b.pixel_size[1].round() as i32;
        assert_eq!(
            out[4].position,
            [thumb.x as f32, (thumb.y + thumb.h - bottom_h) as f32]
        );
        assert_eq!(out[4].uv_origin, th_b.uv_origin);

        // 5: thumb mid — stretched between caps.
        let top_h = th_t.pixel_size[1].round() as i32;
        assert_eq!(out[5].position, [thumb.x as f32, (thumb.y + top_h) as f32]);
        assert_eq!(
            out[5].size,
            [thumb.w as f32, (thumb.h - top_h - bottom_h) as f32]
        );
        assert_eq!(out[5].uv_origin, th_m.uv_origin);

        // 6..10: outer bevel ring (4 edges) at DEPTH-0.00007; 10..14: inner ring at -0.00008.
        for inst in &out[6..10] {
            assert_eq!(
                inst.depth,
                SHELL_DROPDOWN_DEPTH - 0.00007,
                "outer bevel ring depth"
            );
        }
        for inst in &out[10..14] {
            assert_eq!(
                inst.depth,
                SHELL_DROPDOWN_DEPTH - 0.00007 - 0.00001,
                "inner bevel ring depth"
            );
        }

        // Pressed up-arrow swaps ONLY the up glyph to the pressed uv.
        let mut pressed = Vec::new();
        paint_control(
            &mut pressed,
            &chrome,
            ControlPaint::ScrollBar {
                scrollbar,
                thumb,
                pressed_part: Some(DropdownScrollbarPart::UpArrow),
            },
        );
        assert_eq!(pressed[1].uv_origin, up_p.uv_origin);
        assert_eq!(pressed[2].uv_origin, dn_r.uv_origin);

        // Empty chrome → nothing emitted.
        let mut empty = Vec::new();
        paint_control(
            &mut empty,
            &ControlChrome::default(),
            ControlPaint::ScrollBar {
                scrollbar,
                thumb,
                pressed_part: None,
            },
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn checkbox_icon_rect_right_edge_is_half_open() {
        // The toggle keys off the 18x18 icon rect via RectPx::contains
        // (half-open): the last interior px (icon.x+17) HITS, the right edge
        // (icon.x+18) MISSES — the boundary the input toggle branch relies on.
        let rect = RectPx::new(71, 286, 150, 16);
        let icon = checkbox_icon_rect(rect);
        assert_eq!(icon.w, 18, "C-Checkbox icon width is 18px");
        assert!(
            icon.contains(icon.x + icon.w - 1, icon.y),
            "last interior px hits"
        );
        assert!(
            !icon.contains(icon.x + icon.w, icon.y),
            "right edge (x+18) misses"
        );
    }
}
