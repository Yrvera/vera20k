//! Skirmish shell state and hit testing.

use crate::app_init::MapMenuEntry;
use crate::skirmish_launch::{
    AiDifficulty, HOUSE_COLOR_COUNT, LaunchCountry, LaunchStartPosition, LaunchTeam,
    LaunchValidationError, SKIRMISH_PLAYER_SLOT_COUNT, SkirmishAiSlot, SkirmishLaunchMode,
    SkirmishLaunchOptions, SkirmishLaunchSession, SkirmishLocalSlot,
};
use crate::skirmish_modes::{SkirmishGameMode, mode_by_id};
use crate::skirmish_scenarios::{
    SkirmishScenarioRecord, filter_records_for_mode, upsert_random_map_sentinel,
};
use crate::ui::main_menu::{SkirmishCountry, SkirmishSettings, StartPosition};
use std::time::{SystemTime, UNIX_EPOCH};

use super::layout::{
    COMBO_ARROW_RESERVE_W, COMBO_DROPDOWN_ROW_H, COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
    COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H, COMBO_DROPDOWN_SCROLLBAR_W, COMBO_FACE_H, ColorComboId,
    RectPx, SkirmishCheckboxId, SkirmishShellLayout, SkirmishTrackbarId, TRACKBAR_PLAQUE_W,
    TRACKBAR_THUMB_W, checkbox_icon_rect, combo_face_rect, trackbar_active_width,
    trackbar_pixel_offset, trackbar_thumb_rect,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishShellUiSound {
    GuiCheckboxSound,
    GenericClick,
    GuiComboOpenSound,
    GuiComboCloseSound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishComboId {
    AiType(usize),
    Side(usize),
    Color(usize),
    Start(usize),
    Team(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishCountryChoice {
    Random,
    Country(SkirmishCountry),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishComboItem {
    AiType(SkirmishAiRowType),
    Country(SkirmishCountryChoice),
    ColorSentinel(i32),
    Color(usize),
    Start(StartPosition),
    Team(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenComboDropdown {
    pub id: SkirmishComboId,
    pub top_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropdownScrollDragState {
    pub id: SkirmishComboId,
    pub grab_offset_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropdownScrollbarPart {
    UpArrow,
    DownArrow,
    Thumb,
    Track,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropdownScrollbarPressState {
    pub id: SkirmishComboId,
    pub part: DropdownScrollbarPart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackbarDragState {
    pub id: SkirmishTrackbarId,
    pub dragging_thumb: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseMapSelection {
    pub mode_id: i32,
    pub record_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseMapModalState {
    pub saved_selection: ChooseMapSelection,
    pub selected_mode_id: i32,
    pub filtered_record_indices: Vec<usize>,
    pub highlighted_filtered_index: Option<usize>,
    pub mode_top_index: usize,
    pub map_top_index: usize,
}

impl ChooseMapModalState {
    pub fn open(
        current_mode_id: i32,
        current_record_index: Option<usize>,
        modes: &[SkirmishGameMode],
        records: &[SkirmishScenarioRecord],
    ) -> Self {
        let selected_mode_id = mode_by_id(modes, current_mode_id)
            .or_else(|| modes.first())
            .map(|mode| mode.id)
            .unwrap_or(current_mode_id);
        let mut state = Self {
            saved_selection: ChooseMapSelection {
                mode_id: current_mode_id,
                record_index: current_record_index,
            },
            selected_mode_id,
            filtered_record_indices: Vec::new(),
            highlighted_filtered_index: None,
            mode_top_index: 0,
            map_top_index: 0,
        };
        state.refresh_records(modes, records, current_record_index);
        state
    }

    pub fn selected_record_index(&self) -> Option<usize> {
        self.highlighted_filtered_index
            .and_then(|idx| self.filtered_record_indices.get(idx).copied())
    }

    pub fn select_mode(
        &mut self,
        mode_id: i32,
        modes: &[SkirmishGameMode],
        records: &[SkirmishScenarioRecord],
    ) -> bool {
        if mode_by_id(modes, mode_id).is_none() {
            return false;
        }
        self.selected_mode_id = mode_id;
        self.map_top_index = 0;
        self.refresh_records(modes, records, None);
        true
    }

    pub fn select_map_filtered_row(&mut self, row: usize) -> bool {
        if row >= self.filtered_record_indices.len() {
            return false;
        }
        self.highlighted_filtered_index = Some(row);
        true
    }

    pub fn accept_selection(&self) -> Option<ChooseMapSelection> {
        Some(ChooseMapSelection {
            mode_id: self.selected_mode_id,
            record_index: Some(self.selected_record_index()?),
        })
    }

    pub const fn cancel_selection(&self) -> ChooseMapSelection {
        self.saved_selection
    }

    pub fn create_random_map(
        &mut self,
        records: &mut Vec<SkirmishScenarioRecord>,
        modes: &[SkirmishGameMode],
        display_name: impl Into<String>,
    ) -> Option<usize> {
        let mode = mode_by_id(modes, self.selected_mode_id)?;
        if !mode.random_maps_allowed {
            return None;
        }

        let record_index = upsert_random_map_sentinel(records, display_name);
        self.refresh_records(modes, records, Some(record_index));
        Some(record_index)
    }

    fn refresh_records(
        &mut self,
        modes: &[SkirmishGameMode],
        records: &[SkirmishScenarioRecord],
        preferred_record_index: Option<usize>,
    ) {
        self.filtered_record_indices = mode_by_id(modes, self.selected_mode_id)
            .map(|mode| filter_records_for_mode(records, mode))
            .unwrap_or_default();
        self.highlighted_filtered_index = preferred_record_index
            .and_then(|record_idx| {
                self.filtered_record_indices
                    .iter()
                    .position(|idx| *idx == record_idx)
            })
            .or_else(|| (!self.filtered_record_indices.is_empty()).then_some(0));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishAiRowType {
    None,
    Easy,
    Normal,
    Hard,
}

impl SkirmishAiRowType {
    pub const fn item_data(self) -> i32 {
        match self {
            Self::None => -1,
            Self::Easy => 2,
            Self::Normal => 1,
            Self::Hard => 0,
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Easy | Self::Normal | Self::Hard)
    }

    pub const fn difficulty(self) -> Option<AiDifficulty> {
        match self {
            Self::None => None,
            Self::Easy => Some(AiDifficulty::Easy),
            Self::Normal => Some(AiDifficulty::Normal),
            Self::Hard => Some(AiDifficulty::Hard),
        }
    }

    pub const fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::None => ("GUI:None", "None"),
            Self::Easy => ("GUI:AIEasy", "Easy"),
            Self::Normal => ("GUI:AINormal", "Normal"),
            Self::Hard => ("GUI:AIHard", "Hard"),
        }
    }
}

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

fn trackbar_rect(layout: &SkirmishShellLayout, id: SkirmishTrackbarId) -> RectPx {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => layout.trackbars.game_speed,
        SkirmishTrackbarId::Credits0x511 => layout.trackbars.credits,
        SkirmishTrackbarId::UnitCount0x50c => layout.trackbars.unit_count,
    }
}

fn trackbar_range(id: SkirmishTrackbarId) -> (i32, i32, i32) {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => (GAME_SPEED_MIN, GAME_SPEED_MAX, GAME_SPEED_STEP),
        SkirmishTrackbarId::Credits0x511 => (CREDITS_MIN, CREDITS_MAX, CREDITS_STEP),
        SkirmishTrackbarId::UnitCount0x50c => (UNIT_COUNT_MIN, UNIT_COUNT_MAX, UNIT_COUNT_STEP),
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

fn trackbar_ids() -> [SkirmishTrackbarId; 3] {
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

        let (min, max, step) = trackbar_range(id);
        let visual_value = trackbar_visual_value(state, id);
        let pixel_offset = trackbar_pixel_offset(visual_value, min, max, step, rect);
        if trackbar_thumb_hit(rect, pixel_offset, x, y) {
            state.trackbar_drag = Some(TrackbarDragState {
                id,
                dragging_thumb: true,
            });
        } else if rect.contains(x, y) {
            let value = trackbar_mouse_value(rect, x, min, max, step);
            if set_trackbar_visual_value_if_changed(state, id, value) {
                state.push_ui_sound(SkirmishShellUiSound::GenericClick);
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
    if !drag.dragging_thumb {
        return SkirmishShellAction::None;
    }

    let rect = trackbar_rect(layout, drag.id);
    let (min, max, step) = trackbar_range(drag.id);
    let value = trackbar_mouse_value(rect, x, min, max, step);
    if set_trackbar_visual_value_if_changed(state, drag.id, value) {
        state.push_ui_sound(SkirmishShellUiSound::GenericClick);
    }
    SkirmishShellAction::None
}

pub fn handle_option_mouse_up(state: &mut SkirmishShellState) -> SkirmishShellAction {
    state.trackbar_drag = None;
    state.dropdown_scroll_drag = None;
    state.dropdown_scroll_press = None;
    SkirmishShellAction::None
}

pub fn handle_option_mouse_wheel(
    state: &mut SkirmishShellState,
    maps: &[MapMenuEntry],
    lines: f32,
) -> bool {
    let Some(open) = state.open_combo_dropdown else {
        return false;
    };
    if lines == 0.0 {
        return true;
    }
    let step = lines.abs().ceil().max(1.0) as i32;
    let rows = if lines > 0.0 { -step } else { step };
    scroll_open_combo_by_rows(state, maps, open.id, rows);
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishShellOpponent {
    pub enabled: bool,
    pub row_type: SkirmishAiRowType,
    pub country: SkirmishCountry,
    pub country_random: bool,
    pub color_index: usize,
    pub start_position: StartPosition,
    pub team: i32,
    pub difficulty: AiDifficulty,
}

impl SkirmishShellOpponent {
    pub const fn is_active(&self) -> bool {
        self.row_type.is_active()
    }
}

#[derive(Debug, Clone)]
pub struct SkirmishShellState {
    pub selected_map_idx: usize,
    pub selected_mode_id: i32,
    pub player_country: SkirmishCountry,
    pub player_country_random: bool,
    pub player_color_index: usize,
    pub player_start_position: StartPosition,
    pub player_team: i32,
    pub starting_credits: i32,
    pub game_speed: i32,
    pub unit_count: i32,
    pub short_game: bool,
    pub super_weapons: bool,
    pub build_off_ally: bool,
    pub crates: bool,
    pub mcv_redeploy: bool,
    pub zoom_enabled: bool,
    pub opponents: Vec<SkirmishShellOpponent>,
    pub pressed_owner_draw_button: Option<OwnerDrawButton>,
    pub trackbar_drag: Option<TrackbarDragState>,
    pub dropdown_scroll_drag: Option<DropdownScrollDragState>,
    pub dropdown_scroll_press: Option<DropdownScrollbarPressState>,
    pub open_combo_dropdown: Option<OpenComboDropdown>,
    pub choose_map_modal: Option<ChooseMapModalState>,
    pub pending_ui_sounds: Vec<SkirmishShellUiSound>,
}

impl Default for SkirmishShellState {
    fn default() -> Self {
        let settings = SkirmishSettings::default();
        let options = SkirmishLaunchOptions::default();
        Self {
            selected_map_idx: settings.selected_map_idx,
            selected_mode_id: 1,
            player_country: settings.player_country,
            player_country_random: false,
            player_color_index: 0,
            player_start_position: settings.start_position,
            player_team: -2,
            starting_credits: options.starting_credits,
            game_speed: options.game_speed,
            unit_count: options.unit_count,
            short_game: options.short_game,
            super_weapons: options.super_weapons,
            build_off_ally: options.build_off_ally,
            crates: options.crates,
            mcv_redeploy: options.mcv_redeploy,
            zoom_enabled: settings.zoom_enabled,
            opponents: default_opponents(settings.ai_country),
            pressed_owner_draw_button: None,
            trackbar_drag: None,
            dropdown_scroll_drag: None,
            dropdown_scroll_press: None,
            open_combo_dropdown: None,
            choose_map_modal: None,
            pending_ui_sounds: Vec::new(),
        }
    }
}

impl SkirmishShellState {
    fn push_ui_sound(&mut self, sound: SkirmishShellUiSound) {
        self.pending_ui_sounds.push(sound);
    }

    pub fn pending_ui_sounds(&self) -> &[SkirmishShellUiSound] {
        &self.pending_ui_sounds
    }

    pub fn drain_pending_ui_sounds(&mut self) -> Vec<SkirmishShellUiSound> {
        std::mem::take(&mut self.pending_ui_sounds)
    }
}

pub fn drain_pending_ui_sounds(state: &mut SkirmishShellState) -> Vec<SkirmishShellUiSound> {
    state.drain_pending_ui_sounds()
}

pub fn combo_dropdown_open(state: &SkirmishShellState) -> bool {
    state.open_combo_dropdown.is_some()
}

pub fn combo_rect(layout: &SkirmishShellLayout, id: SkirmishComboId) -> Option<RectPx> {
    match id {
        SkirmishComboId::AiType(idx) => layout.rows.ai_type_combos.get(idx).copied(),
        SkirmishComboId::Side(row) => layout.rows.side_combos.get(row).copied(),
        SkirmishComboId::Color(row) => layout.color_combos.get(row).copied(),
        SkirmishComboId::Start(row) => layout.rows.start_combos.get(row).copied(),
        SkirmishComboId::Team(row) => layout.rows.team_combos.get(row).copied(),
    }
}

pub fn combo_dropdown_rect(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> Option<RectPx> {
    let rect = combo_rect(layout, id)?;
    let item_count = combo_items(state, maps, id).len() as i32;
    if item_count == 0 {
        return None;
    }
    let max_rows = combo_dropdown_max_visible_rows(id);
    let visible_rows = if max_rows > 0 {
        item_count.min(max_rows)
    } else {
        item_count
    };
    Some(RectPx::new(
        rect.x,
        rect.y + COMBO_FACE_H + 1,
        rect.w,
        visible_rows * COMBO_DROPDOWN_ROW_H,
    ))
}

pub const fn combo_dropdown_max_visible_rows(id: SkirmishComboId) -> i32 {
    match id {
        SkirmishComboId::Side(_) => 7,
        SkirmishComboId::Color(_) | SkirmishComboId::Start(_) => 9,
        SkirmishComboId::AiType(_) | SkirmishComboId::Team(_) => 0,
    }
}

pub fn combo_dropdown_visible_row_count(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> usize {
    let item_count = combo_items(state, maps, id).len();
    let max_rows = combo_dropdown_max_visible_rows(id);
    if max_rows > 0 {
        item_count.min(max_rows as usize)
    } else {
        item_count
    }
}

fn combo_dropdown_item_count(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> usize {
    combo_items(state, maps, id).len()
}

pub fn combo_dropdown_needs_scrollbar(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> bool {
    combo_dropdown_item_count(state, maps, id) > combo_dropdown_visible_row_count(state, maps, id)
}

fn combo_dropdown_max_top_index(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> usize {
    combo_dropdown_item_count(state, maps, id)
        .saturating_sub(combo_dropdown_visible_row_count(state, maps, id))
}

fn normal_color_index(color_index: usize) -> usize {
    color_index.min(HOUSE_COLOR_COUNT - 1)
}

pub fn combo_dropdown_scrollbar_rect(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> Option<RectPx> {
    if !combo_dropdown_needs_scrollbar(state, maps, id) {
        return None;
    }
    let dropdown = combo_dropdown_rect(state, layout, maps, id)?;
    Some(RectPx::new(
        dropdown.x + dropdown.w - COMBO_DROPDOWN_SCROLLBAR_W,
        dropdown.y,
        COMBO_DROPDOWN_SCROLLBAR_W,
        dropdown.h,
    ))
}

pub fn combo_dropdown_content_rect(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> Option<RectPx> {
    let dropdown = combo_dropdown_rect(state, layout, maps, id)?;
    let width = if combo_dropdown_needs_scrollbar(state, maps, id) {
        dropdown.w - COMBO_DROPDOWN_SCROLLBAR_W
    } else {
        dropdown.w
    };
    Some(RectPx::new(
        dropdown.x,
        dropdown.y,
        width.max(0),
        dropdown.h,
    ))
}

fn combo_dropdown_thumb_height(visible_rows: usize, item_count: usize, scrollbar_h: i32) -> i32 {
    let track_h = (scrollbar_h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2).max(1);
    if item_count == 0 {
        return track_h.max(COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H);
    }
    ((track_h * visible_rows as i32) / item_count as i32)
        .max(COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H)
        .min(track_h)
}

pub fn combo_dropdown_scroll_thumb_rect(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> Option<RectPx> {
    let scrollbar = combo_dropdown_scrollbar_rect(state, layout, maps, id)?;
    let visible_rows = combo_dropdown_visible_row_count(state, maps, id);
    let item_count = combo_dropdown_item_count(state, maps, id);
    let thumb_h = combo_dropdown_thumb_height(visible_rows, item_count, scrollbar.h);
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    let open_top = state
        .open_combo_dropdown
        .filter(|open| open.id == id)
        .map(|open| open.top_index.min(max_top))
        .unwrap_or(0);
    let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb_h).max(1);
    let thumb_y = scrollbar.y
        + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H
        + if max_top == 0 {
            0
        } else {
            (track_span * open_top as i32) / max_top as i32
        };
    Some(RectPx::new(scrollbar.x, thumb_y, scrollbar.w, thumb_h))
}

fn set_open_combo_top_index(
    state: &mut SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    top_index: usize,
) {
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    if let Some(open) = state
        .open_combo_dropdown
        .as_mut()
        .filter(|open| open.id == id)
    {
        open.top_index = top_index.min(max_top);
    }
}

fn scroll_open_combo_by_rows(
    state: &mut SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    rows: i32,
) -> bool {
    let Some(open) = state.open_combo_dropdown.filter(|open| open.id == id) else {
        return false;
    };
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    let next = if rows < 0 {
        open.top_index.saturating_sub((-rows) as usize)
    } else {
        (open.top_index + rows as usize).min(max_top)
    };
    set_open_combo_top_index(state, maps, id, next);
    next != open.top_index
}

fn top_index_from_thumb_y(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    mouse_y: i32,
    grab_offset_y: i32,
) -> Option<usize> {
    let scrollbar = combo_dropdown_scrollbar_rect(state, layout, maps, id)?;
    let thumb = combo_dropdown_scroll_thumb_rect(state, layout, maps, id)?;
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    if max_top == 0 {
        return Some(0);
    }
    let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb.h).max(1);
    let thumb_top = (mouse_y - grab_offset_y).clamp(
        scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
        scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H - thumb.h,
    );
    let local = thumb_top - scrollbar.y - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H;
    Some(((local * max_top as i32 + track_span / 2) / track_span) as usize)
}

fn top_index_from_scrollbar_track_click(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    mouse_y: i32,
) -> Option<usize> {
    let scrollbar = combo_dropdown_scrollbar_rect(state, layout, maps, id)?;
    let thumb = combo_dropdown_scroll_thumb_rect(state, layout, maps, id)?;
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    if max_top == 0 {
        return Some(0);
    }
    let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb.h).max(1);
    let thumb_top = (mouse_y - thumb.h / 2).clamp(
        scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
        scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H - thumb.h,
    );
    let local = thumb_top - scrollbar.y - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H;
    Some(((local * max_top as i32 + track_span / 2) / track_span) as usize)
}

pub fn combo_items(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> Vec<SkirmishComboItem> {
    match id {
        SkirmishComboId::AiType(_) => [
            SkirmishAiRowType::None,
            SkirmishAiRowType::Easy,
            SkirmishAiRowType::Normal,
            SkirmishAiRowType::Hard,
        ]
        .into_iter()
        .map(SkirmishComboItem::AiType)
        .collect(),
        SkirmishComboId::Side(_) => std::iter::once(SkirmishCountryChoice::Random)
            .chain(
                SkirmishCountry::ALL
                    .into_iter()
                    .map(SkirmishCountryChoice::Country),
            )
            .map(SkirmishComboItem::Country)
            .collect(),
        SkirmishComboId::Color(_) => std::iter::once(SkirmishComboItem::ColorSentinel(-2))
            .chain((0..HOUSE_COLOR_COUNT).map(SkirmishComboItem::Color))
            .collect(),
        SkirmishComboId::Start(row) => {
            let capacity = maps
                .get(state.selected_map_idx)
                .map(|map| map.multiplayer_start_waypoints.len())
                .unwrap_or(SKIRMISH_PLAYER_SLOT_COUNT)
                .min(SKIRMISH_PLAYER_SLOT_COUNT);
            let selected = selected_start_position(state, row);
            let mut items = vec![SkirmishComboItem::Start(StartPosition::Auto)];
            for position in 0..capacity {
                let start = StartPosition::Position(position as u8);
                if selected == Some(start) || !start_position_taken_by_other_row(state, row, start)
                {
                    items.push(SkirmishComboItem::Start(start));
                }
            }
            items
        }
        SkirmishComboId::Team(_) => [-2, 0, 1, 2, 3]
            .into_iter()
            .map(SkirmishComboItem::Team)
            .collect(),
    }
}

pub fn selected_combo_item(
    state: &SkirmishShellState,
    id: SkirmishComboId,
) -> Option<SkirmishComboItem> {
    match id {
        SkirmishComboId::AiType(idx) => state
            .opponents
            .get(idx)
            .map(|opponent| SkirmishComboItem::AiType(opponent.row_type)),
        SkirmishComboId::Side(0) => {
            Some(SkirmishComboItem::Country(if state.player_country_random {
                SkirmishCountryChoice::Random
            } else {
                SkirmishCountryChoice::Country(state.player_country)
            }))
        }
        SkirmishComboId::Side(row) => state.opponents.get(row - 1).map(|opponent| {
            SkirmishComboItem::Country(if opponent.country_random {
                SkirmishCountryChoice::Random
            } else {
                SkirmishCountryChoice::Country(opponent.country)
            })
        }),
        SkirmishComboId::Color(0) => Some(SkirmishComboItem::Color(normal_color_index(
            state.player_color_index,
        ))),
        SkirmishComboId::Color(row) => state
            .opponents
            .get(row - 1)
            .map(|opponent| SkirmishComboItem::Color(normal_color_index(opponent.color_index))),
        SkirmishComboId::Start(row) => {
            selected_start_position(state, row).map(SkirmishComboItem::Start)
        }
        SkirmishComboId::Team(0) => Some(SkirmishComboItem::Team(state.player_team)),
        SkirmishComboId::Team(row) => state
            .opponents
            .get(row - 1)
            .map(|opponent| SkirmishComboItem::Team(opponent.team)),
    }
}

pub fn selected_combo_item_index(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> Option<usize> {
    let selected = selected_combo_item(state, id)?;
    combo_items(state, maps, id)
        .iter()
        .position(|item| *item == selected)
}

pub fn combo_enabled(state: &SkirmishShellState, id: SkirmishComboId) -> bool {
    match id {
        SkirmishComboId::AiType(_) => true,
        SkirmishComboId::Side(0)
        | SkirmishComboId::Color(0)
        | SkirmishComboId::Start(0)
        | SkirmishComboId::Team(0) => true,
        SkirmishComboId::Side(row)
        | SkirmishComboId::Color(row)
        | SkirmishComboId::Start(row)
        | SkirmishComboId::Team(row) => state
            .opponents
            .get(row.saturating_sub(1))
            .is_some_and(SkirmishShellOpponent::is_active),
    }
}

fn selected_start_position(state: &SkirmishShellState, row: usize) -> Option<StartPosition> {
    if row == 0 {
        Some(state.player_start_position)
    } else {
        state
            .opponents
            .get(row - 1)
            .map(|opponent| opponent.start_position)
    }
}

fn start_position_taken_by_other_row(
    state: &SkirmishShellState,
    row: usize,
    start: StartPosition,
) -> bool {
    if state.player_start_position == start && row != 0 {
        return true;
    }
    state
        .opponents
        .iter()
        .enumerate()
        .any(|(idx, opponent)| idx + 1 != row && opponent.start_position == start)
}

fn arrow_hit_rect(rect: RectPx) -> RectPx {
    let face = combo_face_rect(rect);
    RectPx::new(
        face.x + face.w - COMBO_ARROW_RESERVE_W,
        face.y,
        COMBO_ARROW_RESERVE_W,
        face.h,
    )
}

fn combo_hit_order() -> Vec<SkirmishComboId> {
    let mut ids = Vec::new();
    for row in (0..SKIRMISH_PLAYER_SLOT_COUNT).rev() {
        ids.push(SkirmishComboId::Team(row));
        ids.push(SkirmishComboId::Start(row));
        ids.push(SkirmishComboId::Color(row));
        ids.push(SkirmishComboId::Side(row));
        if row > 0 {
            ids.push(SkirmishComboId::AiType(row - 1));
        }
    }
    ids
}

fn combo_arrow_at(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    x: i32,
    y: i32,
) -> Option<SkirmishComboId> {
    combo_hit_order().into_iter().find(|id| {
        combo_enabled(state, *id)
            && combo_rect(layout, *id)
                .map(|rect| arrow_hit_rect(rect).contains(x, y))
                .unwrap_or(false)
    })
}

fn apply_combo_selection(
    state: &mut SkirmishShellState,
    id: SkirmishComboId,
    item: SkirmishComboItem,
) {
    match (id, item) {
        (SkirmishComboId::AiType(idx), SkirmishComboItem::AiType(row_type)) => {
            if let Some(opponent) = state.opponents.get_mut(idx) {
                opponent.row_type = row_type;
                opponent.enabled = row_type.is_active();
                if let Some(difficulty) = row_type.difficulty() {
                    opponent.difficulty = difficulty;
                }
            }
        }
        (SkirmishComboId::Side(0), SkirmishComboItem::Country(choice)) => match choice {
            SkirmishCountryChoice::Random => {
                state.player_country_random = true;
            }
            SkirmishCountryChoice::Country(country) => {
                state.player_country_random = false;
                state.player_country = country;
            }
        },
        (SkirmishComboId::Side(row), SkirmishComboItem::Country(choice)) => {
            if let Some(opponent) = state.opponents.get_mut(row - 1) {
                match choice {
                    SkirmishCountryChoice::Random => {
                        opponent.country_random = true;
                    }
                    SkirmishCountryChoice::Country(country) => {
                        opponent.country_random = false;
                        opponent.country = country;
                    }
                }
            }
        }
        (SkirmishComboId::Color(0), SkirmishComboItem::Color(color)) => {
            state.player_color_index = color.min(HOUSE_COLOR_COUNT - 1);
        }
        (SkirmishComboId::Color(row), SkirmishComboItem::Color(color)) => {
            if let Some(opponent) = state.opponents.get_mut(row - 1) {
                opponent.color_index = color.min(HOUSE_COLOR_COUNT - 1);
            }
        }
        (SkirmishComboId::Color(_), SkirmishComboItem::ColorSentinel(_)) => {}
        (SkirmishComboId::Start(0), SkirmishComboItem::Start(start)) => {
            state.player_start_position = start;
        }
        (SkirmishComboId::Start(row), SkirmishComboItem::Start(start)) => {
            if let Some(opponent) = state.opponents.get_mut(row - 1) {
                opponent.start_position = start;
            }
        }
        (SkirmishComboId::Team(0), SkirmishComboItem::Team(team)) => {
            state.player_team = team;
        }
        (SkirmishComboId::Team(row), SkirmishComboItem::Team(team)) => {
            if let Some(opponent) = state.opponents.get_mut(row - 1) {
                opponent.team = team;
            }
        }
        _ => {}
    }
}

fn handle_combo_mouse_down(
    state: &mut SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    x: i32,
    y: i32,
) -> bool {
    if let Some(open) = state.open_combo_dropdown {
        if let Some(scrollbar) = combo_dropdown_scrollbar_rect(state, layout, maps, open.id) {
            if scrollbar.contains(x, y) {
                if y < scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H {
                    state.dropdown_scroll_press = Some(DropdownScrollbarPressState {
                        id: open.id,
                        part: DropdownScrollbarPart::UpArrow,
                    });
                    scroll_open_combo_by_rows(state, maps, open.id, -1);
                } else if y >= scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H {
                    state.dropdown_scroll_press = Some(DropdownScrollbarPressState {
                        id: open.id,
                        part: DropdownScrollbarPart::DownArrow,
                    });
                    scroll_open_combo_by_rows(state, maps, open.id, 1);
                } else if let Some(thumb) =
                    combo_dropdown_scroll_thumb_rect(state, layout, maps, open.id)
                {
                    if thumb.contains(x, y) {
                        state.dropdown_scroll_press = Some(DropdownScrollbarPressState {
                            id: open.id,
                            part: DropdownScrollbarPart::Thumb,
                        });
                        state.dropdown_scroll_drag = Some(DropdownScrollDragState {
                            id: open.id,
                            grab_offset_y: y - thumb.y,
                        });
                    } else if let Some(top_index) =
                        top_index_from_scrollbar_track_click(state, layout, maps, open.id, y)
                    {
                        state.dropdown_scroll_press = Some(DropdownScrollbarPressState {
                            id: open.id,
                            part: DropdownScrollbarPart::Track,
                        });
                        set_open_combo_top_index(state, maps, open.id, top_index);
                    }
                }
                return true;
            }
        }
        if let Some(dropdown) = combo_dropdown_rect(state, layout, maps, open.id) {
            if let Some(content) = combo_dropdown_content_rect(state, layout, maps, open.id) {
                if content.contains(x, y) {
                    let row = open.top_index + ((y - content.y) / COMBO_DROPDOWN_ROW_H) as usize;
                    if let Some(item) = combo_items(state, maps, open.id).get(row).copied() {
                        apply_combo_selection(state, open.id, item);
                    }
                    state.open_combo_dropdown = None;
                    state.dropdown_scroll_drag = None;
                    state.dropdown_scroll_press = None;
                    state.push_ui_sound(SkirmishShellUiSound::GuiComboCloseSound);
                    return true;
                }
            }
            if dropdown.contains(x, y) {
                // The click landed in popup chrome rather than a row; native
                // ComboDropWin consumes it without closing the popup.
                return true;
            }
        }
        if let Some(id) = combo_arrow_at(state, layout, x, y) {
            state.open_combo_dropdown = if id == open.id {
                state.push_ui_sound(SkirmishShellUiSound::GuiComboCloseSound);
                None
            } else {
                state.push_ui_sound(SkirmishShellUiSound::GuiComboCloseSound);
                state.push_ui_sound(SkirmishShellUiSound::GuiComboOpenSound);
                Some(OpenComboDropdown { id, top_index: 0 })
            };
            state.dropdown_scroll_drag = None;
            state.dropdown_scroll_press = None;
            return true;
        }
        state.open_combo_dropdown = None;
        state.dropdown_scroll_drag = None;
        state.dropdown_scroll_press = None;
        state.push_ui_sound(SkirmishShellUiSound::GuiComboCloseSound);
        return true;
    }

    if let Some(id) = combo_arrow_at(state, layout, x, y) {
        state.open_combo_dropdown = Some(OpenComboDropdown { id, top_index: 0 });
        state.dropdown_scroll_drag = None;
        state.dropdown_scroll_press = None;
        state.push_ui_sound(SkirmishShellUiSound::GuiComboOpenSound);
        return true;
    }

    false
}

fn default_opponents(first_country: SkirmishCountry) -> Vec<SkirmishShellOpponent> {
    let countries = [
        first_country,
        SkirmishCountry::Cuba,
        SkirmishCountry::Libya,
        SkirmishCountry::Iraq,
        SkirmishCountry::America,
        SkirmishCountry::Korea,
        SkirmishCountry::Germany,
    ];

    countries
        .into_iter()
        .enumerate()
        .map(|(idx, country)| SkirmishShellOpponent {
            enabled: idx == 0,
            row_type: if idx == 0 {
                SkirmishAiRowType::Easy
            } else {
                SkirmishAiRowType::None
            },
            country,
            country_random: false,
            color_index: (idx + 1) % HOUSE_COLOR_COUNT,
            start_position: StartPosition::Auto,
            team: -2,
            difficulty: AiDifficulty::Easy,
        })
        .collect()
}

fn random_country_for_slot(state: &SkirmishShellState, slot: usize) -> SkirmishCountry {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as usize)
        .unwrap_or(0);
    let idx = nanos
        .wrapping_add(state.selected_map_idx)
        .wrapping_add(slot * 17)
        % SkirmishCountry::ALL.len();
    SkirmishCountry::ALL[idx]
}

fn launch_country_from_menu(
    state: &SkirmishShellState,
    slot: usize,
    country: SkirmishCountry,
    random: bool,
) -> LaunchCountry {
    let country = if random {
        random_country_for_slot(state, slot)
    } else {
        country
    };
    match country {
        SkirmishCountry::America => LaunchCountry::America,
        SkirmishCountry::Korea => LaunchCountry::Korea,
        SkirmishCountry::France => LaunchCountry::France,
        SkirmishCountry::Germany => LaunchCountry::Germany,
        SkirmishCountry::GreatBritain => LaunchCountry::GreatBritain,
        SkirmishCountry::Libya => LaunchCountry::Libya,
        SkirmishCountry::Iraq => LaunchCountry::Iraq,
        SkirmishCountry::Cuba => LaunchCountry::Cuba,
        SkirmishCountry::Russia => LaunchCountry::Russia,
        SkirmishCountry::Yuri => LaunchCountry::Yuri,
    }
}

fn launch_start_position(
    slot: usize,
    start_position: StartPosition,
) -> Result<LaunchStartPosition, LaunchValidationError> {
    match start_position {
        StartPosition::Auto => Ok(LaunchStartPosition::Auto),
        StartPosition::Position(position) if position < SKIRMISH_PLAYER_SLOT_COUNT as u8 => {
            Ok(LaunchStartPosition::Position(position))
        }
        StartPosition::Position(position) => {
            Err(LaunchValidationError::InvalidStartPosition { slot, position })
        }
    }
}

fn launch_color_index(slot: usize, color_index: usize) -> Result<u8, LaunchValidationError> {
    if color_index < HOUSE_COLOR_COUNT {
        Ok(color_index as u8)
    } else {
        Err(LaunchValidationError::InvalidColorIndex { slot, color_index })
    }
}

pub fn launch_settings(state: &SkirmishShellState) -> SkirmishSettings {
    let ai_country = state
        .opponents
        .iter()
        .find(|opponent| opponent.is_active())
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

pub fn launch_session(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
) -> Result<SkirmishLaunchSession, LaunchValidationError> {
    let selected_map = maps
        .get(state.selected_map_idx)
        .ok_or(LaunchValidationError::NoSelectedMap)?;

    let active_count = state
        .opponents
        .iter()
        .filter(|opponent| opponent.is_active())
        .count();
    let requested_players = active_count + 1;
    let capacity = selected_map.multiplayer_start_waypoints.len();
    if capacity < requested_players {
        return Err(LaunchValidationError::MapCapacityExceeded {
            capacity,
            requested_players,
        });
    }
    if active_count == 0 {
        return Err(LaunchValidationError::NoEnabledOpponent);
    }

    if state.player_team >= 0 {
        let local_team = state.player_team as u8;
        let all_active_ai_same_team = state
            .opponents
            .iter()
            .filter(|opponent| opponent.is_active())
            .all(|opponent| {
                LaunchTeam::from_shell_value(opponent.team) == LaunchTeam::Team(local_team)
            });
        if all_active_ai_same_team {
            return Err(LaunchValidationError::SameExplicitTeam { team: local_team });
        }
    }

    let local = SkirmishLocalSlot {
        country: launch_country_from_menu(
            state,
            0,
            state.player_country,
            state.player_country_random,
        ),
        color_index: launch_color_index(0, state.player_color_index)?,
        start_position: launch_start_position(0, state.player_start_position)?,
        team: LaunchTeam::from_shell_value(state.player_team),
    };

    let mut opponents = Vec::new();
    for (idx, opponent) in state.opponents.iter().enumerate() {
        let Some(difficulty) = opponent.row_type.difficulty() else {
            continue;
        };
        let slot = idx + 1;
        opponents.push(SkirmishAiSlot {
            country: launch_country_from_menu(
                state,
                slot,
                opponent.country,
                opponent.country_random,
            ),
            color_index: launch_color_index(slot, opponent.color_index)?,
            start_position: launch_start_position(slot, opponent.start_position)?,
            team: LaunchTeam::from_shell_value(opponent.team),
            difficulty,
        });
    }

    let mut options = SkirmishLaunchOptions::default();
    options.starting_credits = state.starting_credits;
    options.unit_count = state.unit_count;
    options.game_speed = state.game_speed;
    options.short_game = state.short_game;
    options.super_weapons = state.super_weapons;
    options.build_off_ally = state.build_off_ally;
    options.crates = state.crates;
    options.mcv_redeploy = state.mcv_redeploy;

    Ok(SkirmishLaunchSession {
        mode: SkirmishLaunchMode::Battle,
        selected_map_file: Some(selected_map.file_name.clone()),
        local,
        opponents,
        options,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::briefing::BriefingSection;
    use crate::map::preview::PreviewSection;
    use crate::map::waypoints::Waypoint;
    use crate::rules::ini_parser::IniFile;
    use crate::skirmish_launch::SKIRMISH_AI_SLOT_COUNT;
    use crate::skirmish_modes::stock_skirmish_modes;
    use crate::skirmish_scenarios::{SkirmishScenarioKind, SkirmishScenarioSource};
    use crate::ui::skirmish_shell::{
        COMBO_DROPDOWN_ROW_H, COMBO_DROPDOWN_SCROLLBAR_W, checkbox_text_rect, compute_layout,
    };

    fn test_map_entry(name: &str) -> MapMenuEntry {
        test_map_entry_with_starts(name, 8)
    }

    fn test_map_entry_with_starts(name: &str, start_count: usize) -> MapMenuEntry {
        MapMenuEntry {
            file_name: name.to_string(),
            display_name: name.to_string(),
            author: None,
            briefing: BriefingSection::default(),
            preview: PreviewSection::default(),
            multiplayer_start_waypoints: (0..start_count)
                .map(|idx| Waypoint {
                    index: idx as u32,
                    rx: idx as u16,
                    ry: idx as u16,
                })
                .collect(),
            preview_source_bounds: None,
        }
    }

    fn test_scenario_record(
        source_ordinal: usize,
        name: &str,
        game_modes: &str,
    ) -> SkirmishScenarioRecord {
        let ini = IniFile::from_str(&format!("[Basic]\nName={name}\nGameModes={game_modes}\n"));
        SkirmishScenarioRecord::concrete_from_ini(
            source_ordinal,
            SkirmishScenarioSource::LooseYrm(format!("{name}.yrm")),
            &format!("{name}.yrm"),
            &ini,
        )
    }

    #[test]
    fn hit_test_start_choose_and_back() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test(
                &layout,
                layout.start_button.x + 1,
                layout.start_button.y + 1
            ),
            SkirmishShellAction::StartGame
        );
        assert_eq!(
            hit_test(
                &layout,
                layout.choose_map_button.x + 1,
                layout.choose_map_button.y + 1
            ),
            SkirmishShellAction::ChooseMap
        );
        assert_eq!(
            hit_test(&layout, layout.back_button.x + 1, layout.back_button.y + 1),
            SkirmishShellAction::BackOrExit
        );
    }

    #[test]
    fn hit_test_uses_exclusive_bottom_right_edges() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test(
                &layout,
                layout.start_button.x + layout.start_button.w,
                layout.start_button.y
            ),
            SkirmishShellAction::None
        );
        assert_eq!(
            hit_test(
                &layout,
                layout.back_button.x,
                layout.back_button.y + layout.back_button.h
            ),
            SkirmishShellAction::None
        );
    }

    #[test]
    fn choose_map_action_bubbles_without_cycling_selected_map() {
        let maps = [test_map_entry("a.mmx"), test_map_entry("b.mmx")];
        let mut shell = SkirmishShellState::default();
        shell.selected_map_idx = 0;

        assert_eq!(
            apply_action(&mut shell, SkirmishShellAction::ChooseMap, &maps),
            SkirmishShellAction::ChooseMap
        );
        assert_eq!(shell.selected_map_idx, 0);
    }

    #[test]
    fn choose_map_modal_open_filters_and_highlights_current_record() {
        let modes = stock_skirmish_modes();
        let records = vec![
            test_scenario_record(0, "first", "standard"),
            test_scenario_record(1, "second", "standard"),
            test_scenario_record(2, "team", "teamgame"),
        ];

        let modal = ChooseMapModalState::open(1, Some(1), &modes, &records);

        assert_eq!(modal.selected_mode_id, 1);
        assert_eq!(modal.filtered_record_indices, vec![0, 1]);
        assert_eq!(modal.highlighted_filtered_index, Some(1));
        assert_eq!(modal.selected_record_index(), Some(1));
    }

    #[test]
    fn choose_map_modal_select_mode_rebuilds_map_list_by_filter() {
        let modes = stock_skirmish_modes();
        let records = vec![
            test_scenario_record(0, "battle", "standard"),
            test_scenario_record(1, "team", "teamgame"),
            test_scenario_record(2, "duel", "duel"),
        ];
        let mut modal = ChooseMapModalState::open(1, Some(0), &modes, &records);

        assert!(modal.select_mode(9, &modes, &records));

        assert_eq!(modal.selected_mode_id, 9);
        assert_eq!(modal.filtered_record_indices, vec![1]);
        assert_eq!(modal.selected_record_index(), Some(1));
        assert_eq!(modal.map_top_index, 0);
    }

    #[test]
    fn choose_map_modal_cancel_restores_saved_selection_accept_uses_highlight() {
        let modes = stock_skirmish_modes();
        let records = vec![
            test_scenario_record(0, "first", "standard"),
            test_scenario_record(1, "second", "standard"),
        ];
        let mut modal = ChooseMapModalState::open(1, Some(0), &modes, &records);

        assert!(modal.select_map_filtered_row(1));

        assert_eq!(
            modal.accept_selection(),
            Some(ChooseMapSelection {
                mode_id: 1,
                record_index: Some(1),
            })
        );
        assert_eq!(
            modal.cancel_selection(),
            ChooseMapSelection {
                mode_id: 1,
                record_index: Some(0),
            }
        );
    }

    #[test]
    fn choose_map_modal_random_map_command_is_mode_gated_and_single_record() {
        let modes = stock_skirmish_modes();
        let mut records = vec![test_scenario_record(0, "battle", "standard")];
        let mut modal = ChooseMapModalState::open(1, Some(0), &modes, &records);

        let first_random = modal
            .create_random_map(&mut records, &modes, "Random Map")
            .expect("battle allows random maps");
        let second_random = modal
            .create_random_map(&mut records, &modes, "Random Map")
            .expect("battle still allows random maps");

        assert_eq!(first_random, second_random);
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[first_random].kind,
            SkirmishScenarioKind::RandomMapSentinel
        );
        assert_eq!(modal.selected_record_index(), Some(first_random));

        assert!(modal.select_mode(9, &modes, &records));
        assert_eq!(modal.create_random_map(&mut records, &modes, "Nope"), None);
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn owner_draw_button_hit_test_returns_control_identity() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test_owner_draw_button(
                &layout,
                layout.start_button.x + 1,
                layout.start_button.y + 1
            ),
            Some(OwnerDrawButton::StartGame0x617)
        );
        assert_eq!(
            hit_test_owner_draw_button(
                &layout,
                layout.choose_map_button.x + 1,
                layout.choose_map_button.y + 1
            ),
            Some(OwnerDrawButton::ChooseMap0x5aa)
        );
        assert_eq!(
            hit_test_owner_draw_button(&layout, layout.back_button.x + 1, layout.back_button.y + 1),
            Some(OwnerDrawButton::Back0x5c0)
        );
        assert_eq!(
            hit_test_owner_draw_button(
                &layout,
                layout.start_button.x + layout.start_button.w,
                layout.start_button.y
            ),
            None
        );
    }

    #[test]
    fn trackbar_mouse_y_gate_rejects_top_four_pixels() {
        let rect = RectPx::new(404, 286, 128, 21);

        assert!(!trackbar_mouse_allowed_y(rect, rect.y));
        assert!(!trackbar_mouse_allowed_y(rect, rect.y + 3));
        assert!(trackbar_mouse_allowed_y(rect, rect.y + 4));
        assert!(!trackbar_mouse_allowed_y(rect, rect.y + rect.h));
    }

    #[test]
    fn trackbar_thumb_hit_uses_exclusive_twelve_pixel_interval() {
        let rect = RectPx::new(404, 286, 128, 21);
        let thumb_x = rect.x + 1 + 10;
        let y = rect.y + 4;

        assert!(trackbar_thumb_hit(rect, 10, thumb_x, y));
        assert!(trackbar_thumb_hit(rect, 10, thumb_x + 11, y));
        assert!(!trackbar_thumb_hit(rect, 10, thumb_x + 12, y));
    }

    #[test]
    fn trackbar_mouse_x_clamps_below_and_above_range() {
        let rect = RectPx::new(404, 286, 128, 21);

        assert_eq!(trackbar_mouse_value(rect, rect.x - 100, 0, 6, 1), 0);
        assert_eq!(trackbar_mouse_value(rect, rect.x + 1000, 0, 6, 1), 6);
    }

    #[test]
    fn trackbar_mouse_value_snaps_credits_and_unit_count() {
        let rect = RectPx::new(404, 314, 128, 21);

        assert_eq!(
            trackbar_mouse_value(rect, rect.x + 39, 5000, 10000, 100),
            7400
        );
        assert_eq!(trackbar_mouse_value(rect, rect.x + 39, 0, 10, 1), 5);
    }

    #[test]
    fn default_shell_options_use_launch_defaults() {
        let shell = SkirmishShellState::default();
        let options = SkirmishLaunchOptions::default();

        assert_eq!(shell.starting_credits, options.starting_credits);
        assert_eq!(shell.game_speed, options.game_speed);
        assert_eq!(shell.unit_count, options.unit_count);
        assert_eq!(shell.short_game, options.short_game);
        assert_eq!(shell.super_weapons, options.super_weapons);
        assert_eq!(shell.build_off_ally, options.build_off_ally);
        assert_eq!(shell.crates, options.crates);
        assert_eq!(shell.mcv_redeploy, options.mcv_redeploy);
    }

    #[test]
    fn checkbox_icon_click_toggles_but_label_click_does_not() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let checkbox = layout.checkboxes[0];
        let initial = shell.short_game;

        assert_eq!(
            handle_option_mouse_down(&mut shell, &layout, &[], checkbox.rect.x, checkbox.rect.y),
            SkirmishShellAction::None
        );
        assert_eq!(shell.short_game, !initial);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiCheckboxSound]
        );

        let label = checkbox_text_rect(checkbox.rect);
        handle_option_mouse_down(&mut shell, &layout, &[], label.x, label.y + 1);
        assert_eq!(shell.short_game, !initial);
        assert!(shell.drain_pending_ui_sounds().is_empty());
    }

    #[test]
    fn trackbar_top_edge_does_not_change_value() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let rect = layout.trackbars.credits;

        handle_option_mouse_down(&mut shell, &layout, &[], rect.x, rect.y);
        assert_eq!(
            shell.starting_credits,
            SkirmishLaunchOptions::default().starting_credits
        );
        assert!(shell.drain_pending_ui_sounds().is_empty());
    }

    #[test]
    fn trackbar_outside_thumb_click_remaps_value() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let rect = layout.trackbars.credits;

        handle_option_mouse_down(&mut shell, &layout, &[], rect.x + 39, rect.y + 4);
        assert_eq!(shell.starting_credits, 7400);
        assert_eq!(shell.trackbar_drag, None);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GenericClick]
        );
    }

    #[test]
    fn trackbar_thumb_hit_starts_drag() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let rect = layout.trackbars.credits;
        let pixel_offset = trackbar_pixel_offset(
            shell.starting_credits,
            CREDITS_MIN,
            CREDITS_MAX,
            CREDITS_STEP,
            rect,
        );
        let thumb = trackbar_thumb_rect(rect, pixel_offset);

        handle_option_mouse_down(&mut shell, &layout, &[], thumb.x, thumb.y + 4);
        assert_eq!(
            shell.trackbar_drag,
            Some(TrackbarDragState {
                id: SkirmishTrackbarId::Credits0x511,
                dragging_thumb: true,
            })
        );
        assert!(shell.drain_pending_ui_sounds().is_empty());
    }

    #[test]
    fn trackbar_mouse_move_updates_only_while_dragging() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let rect = layout.trackbars.credits;
        let pixel_offset = trackbar_pixel_offset(
            shell.starting_credits,
            CREDITS_MIN,
            CREDITS_MAX,
            CREDITS_STEP,
            rect,
        );
        let thumb = trackbar_thumb_rect(rect, pixel_offset);

        handle_option_mouse_down(&mut shell, &layout, &[], thumb.x, thumb.y + 4);
        handle_option_mouse_move(&mut shell, &layout, &[], rect.x - 100, rect.y + 4);
        assert_eq!(shell.starting_credits, CREDITS_MIN);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GenericClick]
        );

        handle_option_mouse_up(&mut shell);
        handle_option_mouse_move(&mut shell, &layout, &[], rect.x + 1000, rect.y + 4);
        assert_eq!(shell.starting_credits, CREDITS_MIN);
        assert_eq!(shell.trackbar_drag, None);
        assert!(shell.drain_pending_ui_sounds().is_empty());
    }

    #[test]
    fn trackbar_repeated_drag_same_value_is_silent() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let rect = layout.trackbars.credits;
        let pixel_offset = trackbar_pixel_offset(
            shell.starting_credits,
            CREDITS_MIN,
            CREDITS_MAX,
            CREDITS_STEP,
            rect,
        );
        let thumb = trackbar_thumb_rect(rect, pixel_offset);

        let max_value_x = thumb.x + TRACKBAR_THUMB_W / 2;
        handle_option_mouse_down(&mut shell, &layout, &[], max_value_x, thumb.y + 4);
        handle_option_mouse_move(&mut shell, &layout, &[], max_value_x, thumb.y + 4);

        assert_eq!(
            shell.starting_credits,
            SkirmishLaunchOptions::default().starting_credits
        );
        assert!(shell.drain_pending_ui_sounds().is_empty());
    }

    #[test]
    fn game_speed_visual_position_inverts_stored_value() {
        assert_eq!(game_speed_visual_position(1), 5);
        assert_eq!(game_speed_from_visual_position(5), 1);
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
    fn default_shell_tracks_native_slot_count() {
        let shell = SkirmishShellState::default();
        assert_eq!(shell.opponents.len(), SKIRMISH_AI_SLOT_COUNT);
        assert_eq!(shell.opponents.iter().filter(|o| o.is_active()).count(), 1);
        assert_eq!(shell.player_team, -2);
        assert_eq!(shell.opponents[0].team, -2);
    }

    #[test]
    fn ai_row_type_uses_verified_item_data_order() {
        assert_eq!(SkirmishAiRowType::None.item_data(), -1);
        assert_eq!(SkirmishAiRowType::Easy.item_data(), 2);
        assert_eq!(SkirmishAiRowType::Normal.item_data(), 1);
        assert_eq!(SkirmishAiRowType::Hard.item_data(), 0);
        assert!(!SkirmishAiRowType::None.is_active());
        assert!(SkirmishAiRowType::Hard.is_active());
    }

    #[test]
    fn launch_session_packs_selected_map_and_enabled_slots() {
        let mut shell = SkirmishShellState::default();
        shell.selected_map_idx = 1;
        shell.player_country = SkirmishCountry::Korea;
        shell.player_color_index = 3;
        shell.player_start_position = StartPosition::Position(2);
        shell.player_team = 0;
        shell.starting_credits = 7400;
        shell.unit_count = 4;
        shell.game_speed = game_speed_from_visual_position(3);
        shell.short_game = false;
        shell.super_weapons = false;
        shell.build_off_ally = false;
        shell.crates = false;
        shell.mcv_redeploy = false;
        shell.opponents[0].country = SkirmishCountry::Yuri;
        shell.opponents[0].color_index = 6;
        shell.opponents[0].start_position = StartPosition::Position(4);
        shell.opponents[0].team = 1;
        shell.opponents[0].row_type = SkirmishAiRowType::Hard;

        let maps = [test_map_entry("first.mmx"), test_map_entry("second.mmx")];
        let session = launch_session(&shell, &maps).expect("session");

        assert_eq!(session.mode, SkirmishLaunchMode::Battle);
        assert_eq!(session.selected_map_file.as_deref(), Some("second.mmx"));
        assert_eq!(session.local.country, LaunchCountry::Korea);
        assert_eq!(session.local.color_index, 3);
        assert_eq!(
            session.local.start_position,
            LaunchStartPosition::Position(2)
        );
        assert_eq!(session.local.team, LaunchTeam::Team(0));
        assert_eq!(session.opponents.len(), 1);
        assert_eq!(session.opponents[0].country, LaunchCountry::Yuri);
        assert_eq!(session.opponents[0].color_index, 6);
        assert_eq!(session.opponents[0].difficulty, AiDifficulty::Hard);
        assert_eq!(session.options.starting_credits, shell.starting_credits);
        assert_eq!(session.options.unit_count, shell.unit_count);
        assert_eq!(session.options.game_speed, shell.game_speed);
        assert_eq!(session.options.short_game, shell.short_game);
        assert_eq!(session.options.super_weapons, shell.super_weapons);
        assert_eq!(session.options.build_off_ally, shell.build_off_ally);
        assert_eq!(session.options.crates, shell.crates);
        assert_eq!(session.options.mcv_redeploy, shell.mcv_redeploy);
    }

    #[test]
    fn launch_session_preserves_build_off_ally_default() {
        let shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let session = launch_session(&shell, &maps).expect("session");

        assert!(session.options.build_off_ally);
    }

    #[test]
    fn launch_session_rejects_missing_map_and_bad_color() {
        let mut shell = SkirmishShellState::default();
        assert_eq!(
            launch_session(&shell, &[]).unwrap_err(),
            LaunchValidationError::NoSelectedMap
        );

        shell.player_color_index = HOUSE_COLOR_COUNT;
        let maps = [test_map_entry("map.mmx")];
        assert_eq!(
            launch_session(&shell, &maps).unwrap_err(),
            LaunchValidationError::InvalidColorIndex {
                slot: 0,
                color_index: HOUSE_COLOR_COUNT,
            }
        );
    }

    #[test]
    fn launch_session_rejects_map_capacity_overflow() {
        let mut shell = SkirmishShellState::default();
        shell.opponents[1].row_type = SkirmishAiRowType::Normal;
        let maps = [test_map_entry_with_starts("tiny.mmx", 2)];

        assert_eq!(
            launch_session(&shell, &maps).unwrap_err(),
            LaunchValidationError::MapCapacityExceeded {
                capacity: 2,
                requested_players: 3,
            }
        );
    }

    #[test]
    fn launch_session_rejects_no_active_opponents() {
        let mut shell = SkirmishShellState::default();
        for opponent in &mut shell.opponents {
            opponent.row_type = SkirmishAiRowType::None;
        }
        let maps = [test_map_entry("map.mmx")];

        assert_eq!(
            launch_session(&shell, &maps).unwrap_err(),
            LaunchValidationError::NoEnabledOpponent
        );
    }

    #[test]
    fn launch_session_rejects_same_explicit_team() {
        let mut shell = SkirmishShellState::default();
        shell.player_team = 0;
        shell.opponents[0].team = 0;
        let maps = [test_map_entry("map.mmx")];

        assert_eq!(
            launch_session(&shell, &maps).unwrap_err(),
            LaunchValidationError::SameExplicitTeam { team: 0 }
        );
    }

    #[test]
    fn team_combo_uses_verified_item_data_values() {
        let shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];

        assert_eq!(
            combo_items(&shell, &maps, SkirmishComboId::Team(0)),
            vec![
                SkirmishComboItem::Team(-2),
                SkirmishComboItem::Team(0),
                SkirmishComboItem::Team(1),
                SkirmishComboItem::Team(2),
                SkirmishComboItem::Team(3),
            ]
        );
    }

    #[test]
    fn side_combo_exposes_random_country_and_verified_dropdown_cap() {
        let layout = compute_layout(800, 600);
        let shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let items = combo_items(&shell, &maps, SkirmishComboId::Side(0));

        assert_eq!(
            items.first().copied(),
            Some(SkirmishComboItem::Country(SkirmishCountryChoice::Random))
        );
        assert_eq!(items.len(), SkirmishCountry::ALL.len() + 1);

        let dropdown =
            combo_dropdown_rect(&shell, &layout, &maps, SkirmishComboId::Side(0)).unwrap();
        assert_eq!(dropdown.y, layout.rows.side_combos[0].y + COMBO_FACE_H + 1);
        assert_eq!(dropdown.h, 7 * COMBO_DROPDOWN_ROW_H);
        assert_eq!(
            combo_dropdown_visible_row_count(&shell, &maps, SkirmishComboId::Side(0)),
            7
        );
        let content =
            combo_dropdown_content_rect(&shell, &layout, &maps, SkirmishComboId::Side(0)).unwrap();
        let scrollbar =
            combo_dropdown_scrollbar_rect(&shell, &layout, &maps, SkirmishComboId::Side(0))
                .unwrap();
        assert_eq!(content.w, dropdown.w - COMBO_DROPDOWN_SCROLLBAR_W);
        assert_eq!(scrollbar.x, dropdown.x + content.w);
        assert_eq!(scrollbar.h, dropdown.h);
    }

    #[test]
    fn dropdown_wheel_and_hit_test_use_top_index() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.rows.side_combos[0];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboOpenSound]
        );
        assert!(handle_option_mouse_wheel(&mut shell, &maps, -1.0));
        assert_eq!(shell.open_combo_dropdown.unwrap().top_index, 1);
        assert!(shell.drain_pending_ui_sounds().is_empty());

        let content =
            combo_dropdown_content_rect(&shell, &layout, &maps, SkirmishComboId::Side(0)).unwrap();
        handle_option_mouse_down(&mut shell, &layout, &maps, content.x + 2, content.y + 1);

        assert_eq!(
            selected_combo_item(&shell, SkirmishComboId::Side(0)),
            combo_items(&shell, &maps, SkirmishComboId::Side(0))
                .get(1)
                .copied()
        );
        assert_eq!(shell.open_combo_dropdown, None);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboCloseSound]
        );
    }

    #[test]
    fn dropdown_scrollbar_arrows_step_and_drag_clamp_top_index() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.rows.side_combos[0];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboOpenSound]
        );
        let scrollbar =
            combo_dropdown_scrollbar_rect(&shell, &layout, &maps, SkirmishComboId::Side(0))
                .unwrap();
        handle_option_mouse_down(
            &mut shell,
            &layout,
            &maps,
            scrollbar.x + 1,
            scrollbar.y + scrollbar.h - 1,
        );
        assert_eq!(shell.open_combo_dropdown.unwrap().top_index, 1);
        assert_eq!(
            shell.dropdown_scroll_press,
            Some(DropdownScrollbarPressState {
                id: SkirmishComboId::Side(0),
                part: DropdownScrollbarPart::DownArrow,
            })
        );
        assert!(shell.drain_pending_ui_sounds().is_empty());

        let thumb =
            combo_dropdown_scroll_thumb_rect(&shell, &layout, &maps, SkirmishComboId::Side(0))
                .unwrap();
        handle_option_mouse_down(&mut shell, &layout, &maps, thumb.x + 1, thumb.y + 1);
        assert_eq!(
            shell.dropdown_scroll_drag,
            Some(DropdownScrollDragState {
                id: SkirmishComboId::Side(0),
                grab_offset_y: 1,
            })
        );
        assert_eq!(
            shell.dropdown_scroll_press,
            Some(DropdownScrollbarPressState {
                id: SkirmishComboId::Side(0),
                part: DropdownScrollbarPart::Thumb,
            })
        );
        assert!(shell.drain_pending_ui_sounds().is_empty());
        handle_option_mouse_move(
            &mut shell,
            &layout,
            &maps,
            thumb.x + 1,
            scrollbar.y + scrollbar.h,
        );
        assert_eq!(
            shell.open_combo_dropdown.unwrap().top_index,
            combo_dropdown_max_top_index(&shell, &maps, SkirmishComboId::Side(0))
        );
        assert!(shell.drain_pending_ui_sounds().is_empty());
        handle_option_mouse_up(&mut shell);
        assert_eq!(shell.dropdown_scroll_drag, None);
        assert_eq!(shell.dropdown_scroll_press, None);
        assert!(shell.drain_pending_ui_sounds().is_empty());
    }

    #[test]
    fn skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.rows.side_combos[0];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);
        shell.drain_pending_ui_sounds();
        let scrollbar =
            combo_dropdown_scrollbar_rect(&shell, &layout, &maps, SkirmishComboId::Side(0))
                .unwrap();
        let max_top = combo_dropdown_max_top_index(&shell, &maps, SkirmishComboId::Side(0));
        let click_y = scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H - 1;
        let selected_before = selected_combo_item(&shell, SkirmishComboId::Side(0));
        let expected = top_index_from_scrollbar_track_click(
            &shell,
            &layout,
            &maps,
            SkirmishComboId::Side(0),
            click_y,
        )
        .unwrap();

        handle_option_mouse_down(&mut shell, &layout, &maps, scrollbar.x + 1, click_y);

        assert!(expected > 1);
        assert!(expected <= max_top);
        assert_eq!(shell.open_combo_dropdown.unwrap().top_index, expected);
        assert_eq!(
            selected_combo_item(&shell, SkirmishComboId::Side(0)),
            selected_before
        );
        assert_eq!(
            shell.open_combo_dropdown.unwrap().id,
            SkirmishComboId::Side(0)
        );
        assert_eq!(
            shell.dropdown_scroll_press,
            Some(DropdownScrollbarPressState {
                id: SkirmishComboId::Side(0),
                part: DropdownScrollbarPart::Track,
            })
        );
        assert!(shell.drain_pending_ui_sounds().is_empty());
    }

    #[test]
    fn selecting_random_country_updates_shell_choice_state() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.rows.side_combos[0];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboOpenSound]
        );
        let dropdown =
            combo_dropdown_rect(&shell, &layout, &maps, SkirmishComboId::Side(0)).unwrap();
        handle_option_mouse_down(&mut shell, &layout, &maps, dropdown.x + 2, dropdown.y + 1);

        assert!(shell.player_country_random);
        assert_eq!(
            selected_combo_item(&shell, SkirmishComboId::Side(0)),
            Some(SkirmishComboItem::Country(SkirmishCountryChoice::Random))
        );
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboCloseSound]
        );
    }

    #[test]
    fn hit_test_ignores_combo_faces_after_owner_draw_buttons() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            hit_test(&layout, layout.color_combos[0].x, layout.color_combos[0].y),
            SkirmishShellAction::None
        );
        assert_eq!(
            hit_test(&layout, layout.color_combos[1].x, layout.color_combos[1].y),
            SkirmishShellAction::None
        );
    }

    #[test]
    fn combo_arrow_opens_dropdown_and_selects_color_row() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.color_combos[0];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboOpenSound]
        );

        assert_eq!(
            shell.open_combo_dropdown,
            Some(OpenComboDropdown {
                id: SkirmishComboId::Color(0),
                top_index: 0
            })
        );

        let dropdown =
            combo_dropdown_rect(&shell, &layout, &maps, SkirmishComboId::Color(0)).unwrap();
        handle_option_mouse_down(
            &mut shell,
            &layout,
            &maps,
            dropdown.x + 2,
            dropdown.y + COMBO_DROPDOWN_ROW_H * 4 + 1,
        );

        assert_eq!(shell.player_color_index, 3);
        assert_eq!(shell.open_combo_dropdown, None);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboCloseSound]
        );
    }

    #[test]
    fn skirmish_color_dropdown_normal_population_omits_initialized_row_8() {
        let mut shell = SkirmishShellState::default();
        shell.player_color_index = 8;
        let maps = [test_map_entry("map.mmx")];

        let items = combo_items(&shell, &maps, SkirmishComboId::Color(0));

        assert_eq!(items.first(), Some(&SkirmishComboItem::ColorSentinel(-2)));
        assert_eq!(
            &items[1..],
            &[
                SkirmishComboItem::Color(0),
                SkirmishComboItem::Color(1),
                SkirmishComboItem::Color(2),
                SkirmishComboItem::Color(3),
                SkirmishComboItem::Color(4),
                SkirmishComboItem::Color(5),
                SkirmishComboItem::Color(6),
                SkirmishComboItem::Color(7),
            ]
        );
        assert!(!items.contains(&SkirmishComboItem::Color(8)));
        assert_eq!(
            selected_combo_item(&shell, SkirmishComboId::Color(0)),
            Some(SkirmishComboItem::Color(7))
        );
        assert_eq!(
            selected_combo_item_index(&shell, &maps, SkirmishComboId::Color(0)),
            Some(8)
        );
    }

    #[test]
    fn combo_outside_click_closes_with_close_sound() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.rows.side_combos[0];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboOpenSound]
        );

        handle_option_mouse_down(&mut shell, &layout, &maps, 0, 0);

        assert_eq!(shell.open_combo_dropdown, None);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboCloseSound]
        );
    }

    #[test]
    fn start_dropdown_omits_starts_reserved_by_other_rows() {
        let mut shell = SkirmishShellState::default();
        shell.player_start_position = StartPosition::Position(0);
        shell.opponents[0].start_position = StartPosition::Position(1);
        let maps = [test_map_entry_with_starts("map.mmx", 4)];

        let items = combo_items(&shell, &maps, SkirmishComboId::Start(2));

        assert!(items.contains(&SkirmishComboItem::Start(StartPosition::Auto)));
        assert!(!items.contains(&SkirmishComboItem::Start(StartPosition::Position(0))));
        assert!(!items.contains(&SkirmishComboItem::Start(StartPosition::Position(1))));
        assert!(items.contains(&SkirmishComboItem::Start(StartPosition::Position(2))));
        assert!(items.contains(&SkirmishComboItem::Start(StartPosition::Position(3))));
    }

    #[test]
    fn ai_type_dropdown_updates_active_row_state() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.rows.ai_type_combos[1];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboOpenSound]
        );
        let dropdown =
            combo_dropdown_rect(&shell, &layout, &maps, SkirmishComboId::AiType(1)).unwrap();
        handle_option_mouse_down(
            &mut shell,
            &layout,
            &maps,
            dropdown.x + 2,
            dropdown.y + COMBO_DROPDOWN_ROW_H * 2 + 1,
        );

        assert_eq!(shell.opponents[1].row_type, SkirmishAiRowType::Normal);
        assert!(shell.opponents[1].enabled);
        assert_eq!(shell.opponents[1].difficulty, AiDifficulty::Normal);
        assert_eq!(
            shell.drain_pending_ui_sounds(),
            vec![SkirmishShellUiSound::GuiComboCloseSound]
        );
    }

    #[test]
    fn inactive_ai_sibling_combo_does_not_open() {
        let layout = compute_layout(800, 600);
        let mut shell = SkirmishShellState::default();
        let maps = [test_map_entry("map.mmx")];
        let rect = layout.rows.side_combos[2];

        handle_option_mouse_down(&mut shell, &layout, &maps, rect.x + rect.w - 1, rect.y + 1);

        assert_eq!(shell.open_combo_dropdown, None);
    }
}
