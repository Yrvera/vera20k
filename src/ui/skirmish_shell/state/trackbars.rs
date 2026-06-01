//! Trackbar and skirmish shell option input helpers.

use crate::app_init::MapMenuEntry;
use crate::rules::ini_parser::IniFile;

use super::super::layout::{
    RectPx, SkirmishCheckboxId, SkirmishShellLayout, SkirmishTrackbarId, TRACKBAR_PLAQUE_W,
    TRACKBAR_THUMB_W, checkbox_icon_rect, trackbar_active_width, trackbar_pixel_offset,
    trackbar_thumb_rect,
};
use super::{
    SkirmishShellAction, SkirmishShellState, SkirmishShellUiSound, handle_combo_mouse_down,
    set_open_combo_top_index, top_index_from_thumb_y,
};

pub const TRACKBAR_MOUSE_X_BIAS: i32 = 6;
pub const TRACKBAR_MIN_CLAMP_X: i32 = 1;
pub const GAME_SPEED_MIN: i32 = 0;
pub const GAME_SPEED_MAX: i32 = 6;
pub const GAME_SPEED_STEP: i32 = 1;
pub const CREDITS_MIN: i32 = 5000;
pub const CREDITS_MAX: i32 = 10000;
pub const CREDITS_STEP: i32 = 100;
pub const UNIT_COUNT_MIN: i32 = 0;
pub const UNIT_COUNT_MAX: i32 = 10;
pub const UNIT_COUNT_STEP: i32 = 1;

/// Credits and Unit Count slider ranges for dialog 0x102.
///
/// gamemd reads these from the live Rules instance when it builds the skirmish
/// dialog — Credits from `[MultiplayerDialogSettings]` MinMoney/MaxMoney/
/// MoneyIncrement and Unit Count from MinUnitCount/MaxUnitCount — so a mod that
/// changes those keys shifts the slider extents and snap increment. We seed the
/// same values from the merged rules INI at lobby construction and fall back to
/// the stock-default constants when a key (or the INI) is absent. GameSpeed's
/// range is a hardcoded literal in gamemd too (0..6), so it is not stored here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishTrackbarBounds {
    pub credits_min: i32,
    pub credits_max: i32,
    pub credits_step: i32,
    pub unit_count_min: i32,
    pub unit_count_max: i32,
}

impl Default for SkirmishTrackbarBounds {
    fn default() -> Self {
        Self {
            credits_min: CREDITS_MIN,
            credits_max: CREDITS_MAX,
            credits_step: CREDITS_STEP,
            unit_count_min: UNIT_COUNT_MIN,
            unit_count_max: UNIT_COUNT_MAX,
        }
    }
}

impl SkirmishTrackbarBounds {
    /// Resolve `(min, max, step)` for a trackbar. GameSpeed is a hardcoded range
    /// (matching gamemd's literal 0..6); Credits and Unit Count come from the
    /// seeded bounds. Unit Count has no increment message in gamemd, so its step
    /// stays `UNIT_COUNT_STEP`.
    pub fn range(&self, id: SkirmishTrackbarId) -> (i32, i32, i32) {
        match id {
            SkirmishTrackbarId::GameSpeed0x529 => (GAME_SPEED_MIN, GAME_SPEED_MAX, GAME_SPEED_STEP),
            SkirmishTrackbarId::Credits0x511 => {
                (self.credits_min, self.credits_max, self.credits_step)
            }
            SkirmishTrackbarId::UnitCount0x50c => {
                (self.unit_count_min, self.unit_count_max, UNIT_COUNT_STEP)
            }
        }
    }

    /// Seed Credits/Unit Count bounds from a merged rules INI's
    /// `[MultiplayerDialogSettings]` section, mirroring gamemd's runtime read.
    /// Each missing key keeps its stock-default value.
    pub fn from_multiplayer_dialog_settings(ini: &IniFile) -> Self {
        let mut bounds = Self::default();
        let Some(section) = ini.section("MultiplayerDialogSettings") else {
            return bounds;
        };
        if let Some(value) = section.get_i32("MinMoney") {
            bounds.credits_min = value;
        }
        if let Some(value) = section.get_i32("MaxMoney") {
            bounds.credits_max = value;
        }
        if let Some(value) = section.get_i32("MoneyIncrement") {
            bounds.credits_step = value;
        }
        if let Some(value) = section.get_i32("MinUnitCount") {
            bounds.unit_count_min = value;
        }
        if let Some(value) = section.get_i32("MaxUnitCount") {
            bounds.unit_count_max = value;
        }
        bounds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackbarDragState {
    pub id: SkirmishTrackbarId,
    pub dragging_thumb: bool,
}

pub type SkirmishTrackbarHScrollNotification = (u16, i32, u32);

pub fn trackbar_mouse_allowed_y(rect: RectPx, mouse_y: i32) -> bool {
    mouse_y > rect.y + rect.h - 18 && mouse_y < rect.y + rect.h
}

pub fn trackbar_mouse_value(rect: RectPx, mouse_x: i32, min: i32, max: i32, step: i32) -> i32 {
    let span = max.saturating_sub(min);
    if span == 0 {
        return min;
    }

    let active_width = trackbar_active_width(rect).max(1);
    let max_clamp_x = (rect.w - TRACKBAR_PLAQUE_W - TRACKBAR_THUMB_W).max(TRACKBAR_MIN_CLAMP_X);
    let local_x =
        (mouse_x - rect.x - TRACKBAR_MOUSE_X_BIAS).clamp(TRACKBAR_MIN_CLAMP_X, max_clamp_x);
    let raw = (((local_x - TRACKBAR_MIN_CLAMP_X) * (span + 1)) / active_width).min(span);
    let step = step.max(1);
    min + (raw / step) * step
}

pub fn trackbar_thumb_hit(rect: RectPx, pixel_offset: i32, mouse_x: i32, mouse_y: i32) -> bool {
    trackbar_thumb_rect(rect, pixel_offset).contains(mouse_x, mouse_y)
}

pub const fn game_speed_visual_position(stored_speed: i32) -> i32 {
    GAME_SPEED_MAX - stored_speed
}

pub const fn game_speed_from_visual_position(visual_position: i32) -> i32 {
    GAME_SPEED_MAX - visual_position
}

fn checkbox_value_mut(state: &mut SkirmishShellState, id: SkirmishCheckboxId) -> &mut bool {
    match id {
        SkirmishCheckboxId::ShortGame0x54e => &mut state.short_game,
        SkirmishCheckboxId::McvRepacks0x693 => &mut state.mcv_redeploy,
        SkirmishCheckboxId::CratesAppear0x696 => &mut state.crates,
        SkirmishCheckboxId::SuperWeapons0x69a => &mut state.super_weapons,
        SkirmishCheckboxId::BuildOffAlly0x69d => &mut state.build_off_ally,
    }
}

pub(super) fn trackbar_rect(layout: &SkirmishShellLayout, id: SkirmishTrackbarId) -> RectPx {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => layout.trackbars.game_speed,
        SkirmishTrackbarId::Credits0x511 => layout.trackbars.credits,
        SkirmishTrackbarId::UnitCount0x50c => layout.trackbars.unit_count,
    }
}

pub(super) const fn trackbar_control_id(id: SkirmishTrackbarId) -> u16 {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => 0x529,
        SkirmishTrackbarId::Credits0x511 => 0x511,
        SkirmishTrackbarId::UnitCount0x50c => 0x50c,
    }
}

pub fn trackbar_visual_value(state: &SkirmishShellState, id: SkirmishTrackbarId) -> i32 {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => game_speed_visual_position(state.game_speed),
        SkirmishTrackbarId::Credits0x511 => state.starting_credits,
        SkirmishTrackbarId::UnitCount0x50c => state.unit_count,
    }
}

fn set_trackbar_visual_value(
    state: &mut SkirmishShellState,
    id: SkirmishTrackbarId,
    visual_value: i32,
) {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => {
            state.game_speed = game_speed_from_visual_position(visual_value);
        }
        SkirmishTrackbarId::Credits0x511 => {
            state.starting_credits = visual_value;
        }
        SkirmishTrackbarId::UnitCount0x50c => {
            state.unit_count = visual_value;
        }
    }
}

fn set_trackbar_visual_value_if_changed(
    state: &mut SkirmishShellState,
    id: SkirmishTrackbarId,
    visual_value: i32,
) -> bool {
    if trackbar_visual_value(state, id) == visual_value {
        return false;
    }
    set_trackbar_visual_value(state, id, visual_value);
    true
}

pub(super) fn trackbar_hscroll_wparam(visual_value: i32) -> u32 {
    (((visual_value as u32) & 0xffff) << 16)
        | SkirmishShellState::TRACKBAR_HSCROLL_CHANGED_LOW_WORD as u32
}

fn push_trackbar_changed(
    state: &mut SkirmishShellState,
    id: SkirmishTrackbarId,
    visual_value: i32,
) {
    state.push_trackbar_hscroll(id, visual_value);
    state.push_ui_sound(SkirmishShellUiSound::GenericClick);
}

pub(super) fn trackbar_ids() -> [SkirmishTrackbarId; 3] {
    [
        SkirmishTrackbarId::GameSpeed0x529,
        SkirmishTrackbarId::Credits0x511,
        SkirmishTrackbarId::UnitCount0x50c,
    ]
}

pub fn handle_option_mouse_down(
    state: &mut SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    x: i32,
    y: i32,
) -> SkirmishShellAction {
    state.trackbar_drag = None;

    if handle_combo_mouse_down(state, layout, maps, x, y) {
        return SkirmishShellAction::None;
    }

    for checkbox in layout.checkboxes {
        if checkbox_icon_rect(checkbox.rect).contains(x, y) {
            let value = checkbox_value_mut(state, checkbox.id);
            *value = !*value;
            state.push_ui_sound(SkirmishShellUiSound::GuiCheckboxSound);
            return SkirmishShellAction::None;
        }
    }

    for id in trackbar_ids() {
        let rect = trackbar_rect(layout, id);
        if !trackbar_mouse_allowed_y(rect, y) {
            continue;
        }

        let (min, max, step) = state.trackbar_bounds.range(id);
        let visual_value = trackbar_visual_value(state, id);
        let pixel_offset = trackbar_pixel_offset(visual_value, min, max, step, rect);
        if trackbar_thumb_hit(rect, pixel_offset, x, y) {
            state.trackbar_drag = Some(TrackbarDragState {
                id,
                dragging_thumb: true,
            });
        } else if rect.contains(x, y) {
            state.trackbar_drag = Some(TrackbarDragState {
                id,
                dragging_thumb: false,
            });
            let value = trackbar_mouse_value(rect, x, min, max, step);
            if set_trackbar_visual_value_if_changed(state, id, value) {
                push_trackbar_changed(state, id, value);
            }
        }
        return SkirmishShellAction::None;
    }

    SkirmishShellAction::None
}

pub fn handle_option_mouse_move(
    state: &mut SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    x: i32,
    y: i32,
) -> SkirmishShellAction {
    if let Some(drag) = state.dropdown_scroll_drag {
        if let Some(top_index) =
            top_index_from_thumb_y(state, layout, maps, drag.id, y, drag.grab_offset_y)
        {
            set_open_combo_top_index(state, maps, drag.id, top_index);
        }
        return SkirmishShellAction::None;
    }

    let Some(drag) = state.trackbar_drag else {
        return SkirmishShellAction::None;
    };

    // A rail (non-thumb) press jumps the value once on mouse-down but does not
    // begin a value-tracking drag — only grabbing the thumb follows the cursor.
    if !drag.dragging_thumb {
        return SkirmishShellAction::None;
    }

    let rect = trackbar_rect(layout, drag.id);
    let (min, max, step) = state.trackbar_bounds.range(drag.id);
    let value = trackbar_mouse_value(rect, x, min, max, step);
    if set_trackbar_visual_value_if_changed(state, drag.id, value) {
        push_trackbar_changed(state, drag.id, value);
    }
    SkirmishShellAction::None
}

pub fn handle_option_mouse_up(state: &mut SkirmishShellState) -> SkirmishShellAction {
    state.trackbar_drag = None;
    state.dropdown_scroll_drag = None;
    state.dropdown_scroll_press = None;
    SkirmishShellAction::None
}

/// Mouse-wheel handling for the skirmish shell.
///
/// There is no verified evidence that retail scrolls an open dropdown via the
/// mouse wheel: no wheel handler exists in the owner-draw combo, popup, or
/// scrollbar callbacks. Until a runtime capture proves otherwise, the wheel is
/// inert in the shell and does not consume the event. Do NOT reintroduce a
/// wheel-driven dropdown scroll without that evidence.
pub fn handle_option_mouse_wheel(
    _state: &mut SkirmishShellState,
    _maps: &[MapMenuEntry],
    _lines: f32,
) -> bool {
    false
}

#[cfg(test)]
mod bounds_tests {
    use super::*;

    #[test]
    fn default_bounds_match_stock_constants() {
        let bounds = SkirmishTrackbarBounds::default();
        assert_eq!(
            bounds.range(SkirmishTrackbarId::Credits0x511),
            (CREDITS_MIN, CREDITS_MAX, CREDITS_STEP)
        );
        assert_eq!(
            bounds.range(SkirmishTrackbarId::UnitCount0x50c),
            (UNIT_COUNT_MIN, UNIT_COUNT_MAX, UNIT_COUNT_STEP)
        );
        assert_eq!(
            bounds.range(SkirmishTrackbarId::GameSpeed0x529),
            (GAME_SPEED_MIN, GAME_SPEED_MAX, GAME_SPEED_STEP)
        );
    }

    #[test]
    fn modded_multiplayer_dialog_settings_override_credit_and_unit_bounds() {
        let ini = IniFile::from_str(
            "[MultiplayerDialogSettings]\n\
             MinMoney=2000\nMaxMoney=50000\nMoneyIncrement=250\n\
             MinUnitCount=1\nMaxUnitCount=20\n",
        );
        let bounds = SkirmishTrackbarBounds::from_multiplayer_dialog_settings(&ini);
        assert_eq!(
            bounds.range(SkirmishTrackbarId::Credits0x511),
            (2000, 50000, 250)
        );
        assert_eq!(
            bounds.range(SkirmishTrackbarId::UnitCount0x50c),
            (1, 20, UNIT_COUNT_STEP)
        );
        // GameSpeed has a hardcoded range in gamemd; modded keys never touch it.
        assert_eq!(
            bounds.range(SkirmishTrackbarId::GameSpeed0x529),
            (GAME_SPEED_MIN, GAME_SPEED_MAX, GAME_SPEED_STEP)
        );
    }

    #[test]
    fn absent_keys_keep_stock_defaults() {
        let ini = IniFile::from_str("[MultiplayerDialogSettings]\nMinMoney=3000\n");
        let bounds = SkirmishTrackbarBounds::from_multiplayer_dialog_settings(&ini);
        assert_eq!(
            bounds.range(SkirmishTrackbarId::Credits0x511),
            (3000, CREDITS_MAX, CREDITS_STEP)
        );
        assert_eq!(
            bounds.range(SkirmishTrackbarId::UnitCount0x50c),
            (UNIT_COUNT_MIN, UNIT_COUNT_MAX, UNIT_COUNT_STEP)
        );
    }
}
