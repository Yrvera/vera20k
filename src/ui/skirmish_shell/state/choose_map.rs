//! Choose-map modal state and selection helpers for the skirmish shell.

use crate::skirmish_modes::{SkirmishGameMode, mode_by_id};
use crate::skirmish_scenarios::{
    SkirmishScenarioRecord, filter_records_for_mode, upsert_random_map_sentinel,
};

use super::super::layout::{ChooseMapListboxId, ChooseMapModalButton};

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
    pub pressed_button: Option<ChooseMapModalButton>,
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
            pressed_button: None,
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

    pub fn mode_row_count(&self, modes: &[SkirmishGameMode]) -> usize {
        modes.len()
    }

    pub fn map_row_count(&self) -> usize {
        self.filtered_record_indices.len()
    }

    pub fn top_index(&self, id: ChooseMapListboxId) -> usize {
        match id {
            ChooseMapListboxId::Mode0x6eb => self.mode_top_index,
            ChooseMapListboxId::Map0x553 => self.map_top_index,
        }
    }

    pub fn set_top_index_clamped(
        &mut self,
        id: ChooseMapListboxId,
        row_count: usize,
        visible_rows: usize,
        top_index: usize,
    ) -> bool {
        let max_top = row_count.saturating_sub(visible_rows.max(1));
        let next = top_index.min(max_top);
        let slot = match id {
            ChooseMapListboxId::Mode0x6eb => &mut self.mode_top_index,
            ChooseMapListboxId::Map0x553 => &mut self.map_top_index,
        };
        if *slot == next {
            return false;
        }
        *slot = next;
        true
    }

    pub fn scroll_listbox_by_rows(
        &mut self,
        id: ChooseMapListboxId,
        row_count: usize,
        visible_rows: usize,
        rows: i32,
    ) -> bool {
        let current = self.top_index(id);
        let next = if rows < 0 {
            current.saturating_sub((-rows) as usize)
        } else {
            current.saturating_add(rows as usize)
        };
        self.set_top_index_clamped(id, row_count, visible_rows, next)
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
        let row_count = self.filtered_record_indices.len();
        self.set_top_index_clamped(
            ChooseMapListboxId::Map0x553,
            row_count,
            1,
            self.map_top_index,
        );
    }
}
