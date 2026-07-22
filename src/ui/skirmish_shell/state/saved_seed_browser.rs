//! Saved-seed browser state: the Load / Save / Delete lists the setup dialog
//! opens over itself.
//!
//! Render-agnostic. Owns the listing, the selection, and — in Save mode — the
//! typed file name. Performing the file operation belongs to the caller; this
//! model only decides what was asked for.

use crate::map::rmg::saved_seeds::SavedSeed;

use super::super::layout::SavedSeedMode;

/// Longest file name the Save field accepts, matching the other shell edits'
/// habit of bounding input rather than letting a name grow unbounded.
pub const SAVED_SEED_NAME_MAX_CHARS: usize = 32;

/// What closing the browser asks the caller to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedSeedOutcome {
    /// Load this file's options into the dialog.
    Load(String),
    /// Write the current options under this typed name.
    Save(String),
    /// Delete this file, then stay open so more can be removed.
    Delete(String),
    /// Dismiss without doing anything.
    Close,
}

#[derive(Debug, Clone)]
pub struct SavedSeedBrowserState {
    pub mode: SavedSeedMode,
    pub entries: Vec<SavedSeed>,
    /// Highlighted row, or `None` when the list is empty.
    pub selected: Option<usize>,
    /// First visible row.
    pub top_index: usize,
    /// The Save field's contents. Seeded from the selected entry so clicking a
    /// row then Save overwrites it, which is how the player expects a save list
    /// to behave.
    pub typed_name: String,
    pub pressed_control: Option<super::super::layout::SavedSeedControl>,
}

impl SavedSeedBrowserState {
    /// Open a browser over an existing listing.
    ///
    /// The first entry starts selected so Load and Delete are immediately
    /// actionable; Save additionally mirrors it into the name field.
    pub fn open(mode: SavedSeedMode, entries: Vec<SavedSeed>) -> Self {
        let selected = (!entries.is_empty()).then_some(0);
        let typed_name = match (mode, entries.first()) {
            (SavedSeedMode::Save, Some(entry)) => entry.display_name.clone(),
            _ => String::new(),
        };
        Self {
            mode,
            entries,
            selected,
            top_index: 0,
            typed_name,
            pressed_control: None,
        }
    }

    /// The highlighted entry, if any.
    pub fn selected_entry(&self) -> Option<&SavedSeed> {
        self.selected.and_then(|index| self.entries.get(index))
    }

    /// Highlight a row, mirroring its name into the Save field.
    pub fn select(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        self.selected = Some(index);
        if self.mode == SavedSeedMode::Save {
            self.typed_name = self.entries[index].display_name.clone();
        }
    }

    /// Whether the action button can fire.
    ///
    /// Load and Delete need a selection; Save needs a non-blank name, which is
    /// why typing one works even with an empty list.
    pub fn action_enabled(&self) -> bool {
        match self.mode {
            SavedSeedMode::Save => !self.typed_name.trim().is_empty(),
            SavedSeedMode::Load | SavedSeedMode::Delete => self.selected_entry().is_some(),
        }
    }

    /// What the action button asks for, or `None` when it is disabled.
    pub fn action_outcome(&self) -> Option<SavedSeedOutcome> {
        if !self.action_enabled() {
            return None;
        }
        match self.mode {
            SavedSeedMode::Save => Some(SavedSeedOutcome::Save(self.typed_name.trim().to_string())),
            SavedSeedMode::Load => Some(SavedSeedOutcome::Load(
                self.selected_entry()?.file_name.clone(),
            )),
            SavedSeedMode::Delete => Some(SavedSeedOutcome::Delete(
                self.selected_entry()?.file_name.clone(),
            )),
        }
    }

    /// Drop an entry after it was deleted, keeping the highlight on the row that
    /// slid into its place rather than jumping to the top.
    pub fn remove_entry(&mut self, file_name: &str) {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.file_name == file_name)
        else {
            return;
        };
        self.entries.remove(index);
        self.selected = if self.entries.is_empty() {
            None
        } else {
            Some(index.min(self.entries.len() - 1))
        };
        self.top_index = self.top_index.min(self.entries.len().saturating_sub(1));
        if self.mode == SavedSeedMode::Save {
            self.typed_name = self
                .selected_entry()
                .map(|entry| entry.display_name.clone())
                .unwrap_or_default();
        }
    }

    /// Append a character to the Save field.
    pub fn insert_name_char(&mut self, ch: char) {
        if self.mode != SavedSeedMode::Save || ch.is_control() {
            return;
        }
        if self.typed_name.chars().count() >= SAVED_SEED_NAME_MAX_CHARS {
            return;
        }
        // A path separator would take the save out of the browser's folder, so
        // it is refused at the keystroke rather than at write time.
        if matches!(ch, '/' | '\\' | ':') {
            return;
        }
        self.typed_name.push(ch);
    }

    /// Remove the last character of the Save field.
    pub fn backspace_name(&mut self) {
        if self.mode == SavedSeedMode::Save {
            self.typed_name.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(name: &str) -> SavedSeed {
        SavedSeed {
            file_name: format!("{name}.sed"),
            display_name: name.to_string(),
        }
    }

    fn entries() -> Vec<SavedSeed> {
        vec![seed("Alpha"), seed("Beta"), seed("Gamma")]
    }

    #[test]
    fn opening_selects_the_first_entry() {
        let state = SavedSeedBrowserState::open(SavedSeedMode::Load, entries());
        assert_eq!(state.selected, Some(0));
        assert_eq!(
            state.selected_entry().map(|e| e.file_name.as_str()),
            Some("Alpha.sed")
        );
    }

    #[test]
    fn save_mirrors_the_selection_into_the_name_field() {
        let mut state = SavedSeedBrowserState::open(SavedSeedMode::Save, entries());
        assert_eq!(state.typed_name, "Alpha", "seeded from the first entry");
        state.select(2);
        assert_eq!(state.typed_name, "Gamma", "clicking a row targets it");
    }

    #[test]
    fn load_and_delete_need_a_selection_but_save_needs_only_a_name() {
        let empty = SavedSeedBrowserState::open(SavedSeedMode::Load, Vec::new());
        assert!(!empty.action_enabled(), "nothing to load");

        let mut save = SavedSeedBrowserState::open(SavedSeedMode::Save, Vec::new());
        assert!(!save.action_enabled(), "a blank name saves nothing");
        save.insert_name_char('X');
        assert!(
            save.action_enabled(),
            "typing a name is enough with no list"
        );
    }

    #[test]
    fn a_whitespace_only_name_does_not_count_as_typed() {
        let mut state = SavedSeedBrowserState::open(SavedSeedMode::Save, Vec::new());
        for ch in "   ".chars() {
            state.insert_name_char(ch);
        }
        assert!(!state.action_enabled());
    }

    #[test]
    fn the_action_reports_the_file_for_load_and_delete_and_the_name_for_save() {
        let load = SavedSeedBrowserState::open(SavedSeedMode::Load, entries());
        assert_eq!(
            load.action_outcome(),
            Some(SavedSeedOutcome::Load("Alpha.sed".to_string()))
        );
        let del = SavedSeedBrowserState::open(SavedSeedMode::Delete, entries());
        assert_eq!(
            del.action_outcome(),
            Some(SavedSeedOutcome::Delete("Alpha.sed".to_string()))
        );
        let mut save = SavedSeedBrowserState::open(SavedSeedMode::Save, entries());
        save.typed_name = "  Fresh  ".to_string();
        assert_eq!(
            save.action_outcome(),
            Some(SavedSeedOutcome::Save("Fresh".to_string())),
            "the name is trimmed"
        );
    }

    #[test]
    fn removing_an_entry_keeps_the_highlight_in_place() {
        let mut state = SavedSeedBrowserState::open(SavedSeedMode::Delete, entries());
        state.select(1);
        state.remove_entry("Beta.sed");
        assert_eq!(state.entries.len(), 2);
        assert_eq!(
            state.selected_entry().map(|e| e.display_name.as_str()),
            Some("Gamma"),
            "the row that slid up is highlighted, not the top"
        );
    }

    #[test]
    fn removing_the_last_entry_clears_the_selection() {
        let mut state = SavedSeedBrowserState::open(SavedSeedMode::Delete, vec![seed("Only")]);
        state.remove_entry("Only.sed");
        assert!(state.entries.is_empty());
        assert_eq!(state.selected, None);
        assert!(!state.action_enabled());
    }

    #[test]
    fn the_name_field_refuses_separators_controls_and_overlong_input() {
        let mut state = SavedSeedBrowserState::open(SavedSeedMode::Save, Vec::new());
        for ch in ['/', '\\', ':', '\n', '\t'] {
            state.insert_name_char(ch);
        }
        assert!(
            state.typed_name.is_empty(),
            "none of those are name characters"
        );

        for _ in 0..(SAVED_SEED_NAME_MAX_CHARS + 10) {
            state.insert_name_char('a');
        }
        assert_eq!(state.typed_name.chars().count(), SAVED_SEED_NAME_MAX_CHARS);

        state.backspace_name();
        assert_eq!(
            state.typed_name.chars().count(),
            SAVED_SEED_NAME_MAX_CHARS - 1
        );
    }

    #[test]
    fn load_mode_ignores_typing() {
        let mut state = SavedSeedBrowserState::open(SavedSeedMode::Load, entries());
        state.insert_name_char('X');
        assert!(state.typed_name.is_empty(), "only Save has a name field");
    }
}
