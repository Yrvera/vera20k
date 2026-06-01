//! Player-name edit state and input helpers for the skirmish shell.

use crate::sim::game_options::GameOptions;
use crate::skirmish_launch::SkirmishLaunchOptions;
use crate::skirmish_modes::{SkirmishGameMode, mode_by_id};
use crate::ui::main_menu::{SkirmishCountry, SkirmishSettings, StartPosition};

use super::super::layout::{SkirmishShellLayout, SkirmishTrackbarId};
use super::super::static_reveal::StaticReveal;
use super::trackbars::{SkirmishTrackbarBounds, trackbar_control_id, trackbar_hscroll_wparam};
use super::{
    ChooseMapModalState, DropdownScrollDragState, DropdownScrollbarPressState, OpenComboDropdown,
    OwnerDrawButton, SkirmishShellOpponent, SkirmishShellUiSound,
    SkirmishTrackbarHScrollNotification, SkirmishValidationModalState, TrackbarDragState,
    default_opponents,
};

pub const PLAYER_NAME_DEFAULT: &str = "Player";
pub const PLAYER_NAME_MAX_CHARS: usize = 19;
pub const PLAYER_NAME_CARET_MARGIN_PX: i32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerNameEditState {
    pub text: String,
    pub focused: bool,
    pub selection: Option<(usize, usize)>,
    pub caret: usize,
    pub scroll_x: i32,
}

impl Default for PlayerNameEditState {
    fn default() -> Self {
        Self::with_name(PLAYER_NAME_DEFAULT)
    }
}

impl PlayerNameEditState {
    /// Build an edit state seeded with a starting name, capped to the field's
    /// 19-character limit. Used to pre-fill the field from the persistent
    /// player profile instead of a hardcoded literal.
    pub fn with_name(name: &str) -> Self {
        let text: String = name.chars().take(PLAYER_NAME_MAX_CHARS).collect();
        let caret = text.chars().count();
        Self {
            text,
            focused: false,
            selection: None,
            caret,
            scroll_x: 0,
        }
    }
}

impl PlayerNameEditState {
    fn char_len(&self) -> usize {
        self.text.chars().count()
    }

    fn clamp_position(&self, position: usize) -> usize {
        position.min(self.char_len())
    }

    fn byte_index(&self, char_index: usize) -> usize {
        if char_index == 0 {
            return 0;
        }
        self.text
            .char_indices()
            .nth(char_index)
            .map(|(idx, _)| idx)
            .unwrap_or(self.text.len())
    }

    fn normalized_selection(&self) -> Option<(usize, usize)> {
        let (start, end) = self.selection?;
        let start = self.clamp_position(start);
        let end = self.clamp_position(end);
        if start == end {
            None
        } else if start < end {
            Some((start, end))
        } else {
            Some((end, start))
        }
    }

    fn delete_range(&mut self, start: usize, end: usize) {
        let start_byte = self.byte_index(start);
        let end_byte = self.byte_index(end);
        self.text.replace_range(start_byte..end_byte, "");
        self.caret = start;
        self.selection = None;
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.normalized_selection() else {
            return false;
        };
        self.delete_range(start, end);
        true
    }

    fn clamp_state(&mut self) {
        let len = self.char_len();
        self.caret = self.caret.min(len);
        self.selection = self.selection.and_then(|(start, end)| {
            let start = start.min(len);
            let end = end.min(len);
            (start != end).then_some((start, end))
        });
        self.scroll_x = self.scroll_x.max(0);
    }

    pub fn focus_select_all(&mut self) -> bool {
        let changed = !self.focused
            || self.selection != Some((0, self.char_len()))
            || self.caret != self.char_len();
        self.focused = true;
        let len = self.char_len();
        self.selection = (len > 0).then_some((0, len));
        self.caret = len;
        self.scroll_x = 0;
        changed
    }

    pub fn blur(&mut self) -> bool {
        let changed = self.focused || self.selection.is_some();
        self.focused = false;
        self.selection = None;
        self.clamp_state();
        changed
    }

    pub fn insert_text(&mut self, text: &str) -> bool {
        let filtered: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if filtered.is_empty() {
            return false;
        }

        self.delete_selection();
        let capacity = PLAYER_NAME_MAX_CHARS.saturating_sub(self.char_len());
        if capacity == 0 {
            return false;
        }
        let inserted: String = filtered.chars().take(capacity).collect();
        if inserted.is_empty() {
            return false;
        }

        let byte = self.byte_index(self.caret);
        self.text.insert_str(byte, &inserted);
        self.caret += inserted.chars().count();
        self.selection = None;
        self.clamp_state();
        true
    }

    pub fn backspace(&mut self) -> bool {
        if self.delete_selection() {
            self.clamp_state();
            return true;
        }
        if self.caret == 0 {
            return false;
        }
        let end = self.caret;
        self.delete_range(end - 1, end);
        self.clamp_state();
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.delete_selection() {
            self.clamp_state();
            return true;
        }
        let len = self.char_len();
        if self.caret >= len {
            return false;
        }
        self.delete_range(self.caret, self.caret + 1);
        self.clamp_state();
        true
    }

    pub fn move_left(&mut self) -> bool {
        let old = (self.caret, self.selection);
        self.selection = None;
        self.caret = self.caret.saturating_sub(1);
        old != (self.caret, self.selection)
    }

    pub fn move_right(&mut self) -> bool {
        let old = (self.caret, self.selection);
        self.selection = None;
        self.caret = (self.caret + 1).min(self.char_len());
        old != (self.caret, self.selection)
    }

    pub fn move_home(&mut self) -> bool {
        let old = (self.caret, self.selection);
        self.selection = None;
        self.caret = 0;
        old != (self.caret, self.selection)
    }

    pub fn move_end(&mut self) -> bool {
        let old = (self.caret, self.selection);
        self.selection = None;
        self.caret = self.char_len();
        old != (self.caret, self.selection)
    }

    pub fn caret_prefix(&self) -> &str {
        &self.text[..self.byte_index(self.caret)]
    }
}

#[derive(Debug, Clone)]
pub struct SkirmishShellState {
    pub selected_map_idx: usize,
    pub selected_mode_id: i32,
    pub player_name_edit: PlayerNameEditState,
    pub player_country: SkirmishCountry,
    pub player_country_random: bool,
    pub player_color_index: usize,
    pub player_color_claimed: bool,
    pub player_start_position: StartPosition,
    pub player_team: i32,
    pub starting_credits: i32,
    pub game_speed: i32,
    pub unit_count: i32,
    /// Credits/Unit Count slider ranges, seeded from `[MultiplayerDialogSettings]`
    /// at lobby construction (stock-default constants until then).
    pub trackbar_bounds: SkirmishTrackbarBounds,
    /// Per-match option base, seeded from `[MultiplayerDialogSettings]` at lobby
    /// construction (stock defaults until then). The launch path overrides the
    /// fields exposed as widgets from the live state above; this base carries
    /// the non-widget toggles (tech level, bases, shroud, …) into the match.
    pub launch_options_base: SkirmishLaunchOptions,
    pub short_game: bool,
    pub super_weapons: bool,
    pub build_off_ally: bool,
    pub crates: bool,
    pub mcv_redeploy: bool,
    pub zoom_enabled: bool,
    pub opponents: Vec<SkirmishShellOpponent>,
    pub selected_mode_allies_allowed: bool,
    pub selected_mode_must_ally: bool,
    pub pressed_owner_draw_button: Option<OwnerDrawButton>,
    pub trackbar_drag: Option<TrackbarDragState>,
    pub dropdown_scroll_drag: Option<DropdownScrollDragState>,
    pub dropdown_scroll_press: Option<DropdownScrollbarPressState>,
    pub open_combo_dropdown: Option<OpenComboDropdown>,
    pub choose_map_modal: Option<ChooseMapModalState>,
    pub validation_modal: Option<SkirmishValidationModalState>,
    pub status_help_text: String,
    pub pending_trackbar_hscrolls: Vec<SkirmishTrackbarHScrollNotification>,
    pub pending_ui_sounds: Vec<SkirmishShellUiSound>,
    /// Title static 0x694 reveal cursor.
    pub title_reveal: StaticReveal,
    /// Game-type static 0x6EC reveal cursor.
    pub game_type_reveal: StaticReveal,
    /// Map-label static 0x5A8 reveal cursor.
    pub map_label_reveal: StaticReveal,
}

impl Default for SkirmishShellState {
    fn default() -> Self {
        let settings = SkirmishSettings::default();
        let options = SkirmishLaunchOptions::default();
        Self {
            selected_map_idx: settings.selected_map_idx,
            selected_mode_id: 1,
            player_name_edit: PlayerNameEditState::default(),
            player_country: settings.player_country,
            player_country_random: false,
            player_color_index: 0,
            player_color_claimed: true,
            player_start_position: settings.start_position,
            player_team: -2,
            starting_credits: options.starting_credits,
            game_speed: options.game_speed,
            unit_count: options.unit_count,
            trackbar_bounds: SkirmishTrackbarBounds::default(),
            launch_options_base: SkirmishLaunchOptions::default(),
            short_game: options.short_game,
            super_weapons: options.super_weapons,
            build_off_ally: options.build_off_ally,
            crates: options.crates,
            mcv_redeploy: options.mcv_redeploy,
            zoom_enabled: settings.zoom_enabled,
            opponents: default_opponents(settings.ai_country),
            selected_mode_allies_allowed: true,
            selected_mode_must_ally: false,
            pressed_owner_draw_button: None,
            trackbar_drag: None,
            dropdown_scroll_drag: None,
            dropdown_scroll_press: None,
            open_combo_dropdown: None,
            choose_map_modal: None,
            validation_modal: None,
            status_help_text: String::new(),
            pending_trackbar_hscrolls: Vec::new(),
            pending_ui_sounds: Vec::new(),
            title_reveal: StaticReveal::default(),
            game_type_reveal: StaticReveal::default(),
            map_label_reveal: StaticReveal::default(),
        }
    }
}

impl SkirmishShellState {
    pub const TRACKBAR_WM_HSCROLL_MESSAGE: u32 = 0x114;
    pub const TRACKBAR_HSCROLL_CHANGED_LOW_WORD: u16 = 5;

    /// Seed the lobby from per-match options parsed from
    /// `[MultiplayerDialogSettings]`. The values the setup dialog exposes as
    /// widgets are copied into the live fields so each control opens on the
    /// configured value; the full set is also retained as the launch base so
    /// the non-widget toggles reach the match unchanged. `GameSpeed` is stored
    /// as parsed — the trackbar inverts it only for display.
    pub fn apply_multiplayer_dialog_values(&mut self, options: &GameOptions) {
        self.starting_credits = options.starting_credits;
        self.unit_count = options.unit_count;
        self.game_speed = options.game_speed;
        self.short_game = options.short_game;
        self.super_weapons = options.super_weapons;
        self.build_off_ally = options.build_off_ally;
        self.crates = options.crates;
        self.mcv_redeploy = options.mcv_redeploy;
        self.launch_options_base = SkirmishLaunchOptions::from_game_options(options);
    }

    pub(super) fn push_ui_sound(&mut self, sound: SkirmishShellUiSound) {
        self.pending_ui_sounds.push(sound);
    }

    pub(super) fn push_trackbar_hscroll(&mut self, id: SkirmishTrackbarId, visual_value: i32) {
        self.pending_trackbar_hscrolls.push((
            trackbar_control_id(id),
            visual_value,
            trackbar_hscroll_wparam(visual_value),
        ));
    }

    pub fn pending_trackbar_hscrolls(&self) -> &[SkirmishTrackbarHScrollNotification] {
        &self.pending_trackbar_hscrolls
    }

    pub fn drain_pending_trackbar_hscrolls(&mut self) -> Vec<SkirmishTrackbarHScrollNotification> {
        std::mem::take(&mut self.pending_trackbar_hscrolls)
    }

    pub fn pending_ui_sounds(&self) -> &[SkirmishShellUiSound] {
        &self.pending_ui_sounds
    }

    pub fn drain_pending_ui_sounds(&mut self) -> Vec<SkirmishShellUiSound> {
        std::mem::take(&mut self.pending_ui_sounds)
    }

    /// Start the reveal for all three right-panel statics using their current
    /// text. Called at shell first-paint slide completion (the 0x4EC->0x4EE
    /// event).
    pub fn start_right_panel_static_reveals(
        &mut self,
        title: &str,
        game_type: &str,
        map_label: &str,
        now: std::time::Instant,
    ) {
        self.title_reveal.start(title, now);
        self.game_type_reveal.start(game_type, now);
        self.map_label_reveal.start(map_label, now);
    }

    /// Advance all three reveals one cadence step (each is internally 30 ms-gated).
    pub fn advance_right_panel_static_reveals(&mut self, now: std::time::Instant) {
        self.title_reveal.advance(now);
        self.game_type_reveal.advance(now);
        self.map_label_reveal.advance(now);
    }
}

pub fn set_status_help_text(state: &mut SkirmishShellState, text: impl Into<String>) -> bool {
    let text = text.into();
    if state.status_help_text == text {
        return false;
    }
    state.status_help_text = text;
    true
}

pub fn clear_status_help_text(state: &mut SkirmishShellState) -> bool {
    set_status_help_text(state, String::new())
}

pub fn dismiss_validation_modal(state: &mut SkirmishShellState) -> bool {
    state.validation_modal.take().is_some()
}

pub fn drain_pending_ui_sounds(state: &mut SkirmishShellState) -> Vec<SkirmishShellUiSound> {
    state.drain_pending_ui_sounds()
}

pub(super) fn inactive_ai_team_default(state: &SkirmishShellState) -> i32 {
    if state.selected_mode_allies_allowed {
        3
    } else {
        -2
    }
}

#[cfg(test)]
mod multiplayer_dialog_value_tests {
    use super::*;

    #[test]
    fn default_shell_launch_base_matches_hardcoded_defaults() {
        // The safety-net default path must stay on the hardcoded fallback so a
        // default()-constructed shell is byte-identical without an INI.
        let shell = SkirmishShellState::default();
        assert_eq!(shell.launch_options_base, SkirmishLaunchOptions::default());
    }

    #[test]
    fn apply_multiplayer_dialog_values_seeds_widgets_and_launch_base() {
        let mut shell = SkirmishShellState::default();
        let options = GameOptions {
            starting_credits: 7400,
            unit_count: 4,
            game_speed: 4,
            short_game: false,
            super_weapons: false,
            build_off_ally: false,
            crates: false,
            mcv_redeploy: false,
            tech_level: 3,
            bases: false,
            shroud: false,
            ..GameOptions::default()
        };

        shell.apply_multiplayer_dialog_values(&options);

        // Widget-backed fields are seeded into the live state.
        assert_eq!(shell.starting_credits, 7400);
        assert_eq!(shell.unit_count, 4);
        // GameSpeed is stored as parsed; the trackbar handles display inversion.
        assert_eq!(shell.game_speed, 4);
        assert!(!shell.short_game);
        assert!(!shell.super_weapons);
        assert!(!shell.build_off_ally);
        assert!(!shell.crates);
        assert!(!shell.mcv_redeploy);
        // Non-widget toggles ride on the launch base into the match.
        assert_eq!(shell.launch_options_base.tech_level, 3);
        assert!(!shell.launch_options_base.bases);
        assert!(!shell.launch_options_base.shroud);
    }
}

pub fn repair_teams_for_selected_mode(state: &mut SkirmishShellState, modes: &[SkirmishGameMode]) {
    let mode = mode_by_id(modes, state.selected_mode_id)
        .or_else(|| mode_by_id(modes, 1))
        .or_else(|| modes.first());
    if let Some(mode) = mode {
        state.selected_mode_allies_allowed = mode.allies_allowed;
        state.selected_mode_must_ally = mode.must_ally;
    }

    let inactive_default = inactive_ai_team_default(state);
    if state.selected_mode_must_ally && state.player_team == -2 {
        state.player_team = 0;
    }
    for opponent in &mut state.opponents {
        if !opponent.is_active() {
            opponent.team = inactive_default;
        } else if state.selected_mode_must_ally && opponent.team == -2 {
            opponent.team = 3;
        }
    }
}

pub fn combo_dropdown_open(state: &SkirmishShellState) -> bool {
    state.open_combo_dropdown.is_some()
}

pub fn player_name_edit_rect_hit(layout: &SkirmishShellLayout, x: i32, y: i32) -> bool {
    layout.player_name.contains(x, y)
}

pub fn focus_player_name_edit(state: &mut SkirmishShellState) -> bool {
    state.open_combo_dropdown = None;
    state.dropdown_scroll_drag = None;
    state.dropdown_scroll_press = None;
    state.trackbar_drag = None;
    state.pressed_owner_draw_button = None;
    state.player_name_edit.focus_select_all()
}

pub fn blur_player_name_edit(state: &mut SkirmishShellState) -> bool {
    state.player_name_edit.blur()
}

pub fn insert_player_name_text(state: &mut SkirmishShellState, text: &str) -> bool {
    state.player_name_edit.insert_text(text)
}

pub fn handle_player_name_backspace(state: &mut SkirmishShellState) -> bool {
    state.player_name_edit.backspace()
}

pub fn handle_player_name_delete(state: &mut SkirmishShellState) -> bool {
    state.player_name_edit.delete()
}

pub fn handle_player_name_left(state: &mut SkirmishShellState) -> bool {
    state.player_name_edit.move_left()
}

pub fn handle_player_name_right(state: &mut SkirmishShellState) -> bool {
    state.player_name_edit.move_right()
}

pub fn handle_player_name_home(state: &mut SkirmishShellState) -> bool {
    state.player_name_edit.move_home()
}

pub fn handle_player_name_end(state: &mut SkirmishShellState) -> bool {
    state.player_name_edit.move_end()
}

pub fn player_name_caret_prefix(state: &SkirmishShellState) -> &str {
    state.player_name_edit.caret_prefix()
}

pub fn update_player_name_scroll_for_caret(
    state: &mut SkirmishShellState,
    visible_width: i32,
    caret_prefix_width: u32,
) -> bool {
    let visible_width = visible_width.max(0);
    let caret_x = caret_prefix_width as i32;
    let old = state.player_name_edit.scroll_x;
    let mut scroll = old.max(0);

    if visible_width <= PLAYER_NAME_CARET_MARGIN_PX * 2 {
        scroll = caret_x;
    } else {
        let right_limit = scroll + visible_width - PLAYER_NAME_CARET_MARGIN_PX;
        if caret_x > right_limit {
            scroll = caret_x - visible_width + PLAYER_NAME_CARET_MARGIN_PX;
        }
        let left_limit = scroll + PLAYER_NAME_CARET_MARGIN_PX;
        if caret_x < left_limit {
            scroll = (caret_x - PLAYER_NAME_CARET_MARGIN_PX).max(0);
        }
    }

    state.player_name_edit.scroll_x = scroll.max(0);
    state.player_name_edit.scroll_x != old
}

/// Handle Tab while the player-name edit has focus. The original moves keyboard
/// focus to the next dialog tab-stop control; the skirmish shell currently
/// models keyboard focus only for this edit, so the observable effect we can
/// reproduce is that focus leaves the edit (its caret/selection clear). Full
/// focus advancement to a specific next control awaits a shell-wide keyboard
/// focus/tab-order model. Returns true when focus state changed.
pub fn handle_player_name_tab(state: &mut SkirmishShellState) -> bool {
    blur_player_name_edit(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn map_change_restarts_map_label_reveal_from_count_one() {
        let now = Instant::now();
        let mut s = SkirmishShellState::default();
        s.map_label_reveal.start("OLD MAP", now);
        for i in 1..=4 {
            s.map_label_reveal.advance(now + Duration::from_millis(30 * i));
        }
        // Selecting a new map restarts the reveal with the new text (the 0x4B2
        // text-update path the use-map handler drives), from the first character.
        s.map_label_reveal.start("NEW MAP", now + Duration::from_millis(500));
        assert_eq!(s.map_label_reveal.window().unwrap().count, 1);
    }
}
