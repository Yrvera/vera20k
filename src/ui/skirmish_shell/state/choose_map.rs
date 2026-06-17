//! Choose-map modal state and selection helpers for the skirmish shell.

use crate::skirmish_modes::{SkirmishGameMode, mode_by_id};
use crate::skirmish_scenarios::{
    SkirmishScenarioRecord, filter_records_for_mode, upsert_random_map_sentinel,
};

use super::super::layout::{
    COMBO_DROPDOWN_SCROLLBAR_BUTTON_H, ChooseMapListboxId, ChooseMapModalButton,
    ChooseMapModalLayout, RectPx, choose_map_listbox_rect, choose_map_listbox_row_at,
    choose_map_listbox_scroll_thumb_rect, choose_map_listbox_scrollbar_rect,
    choose_map_listbox_top_index_from_track_click, choose_map_listbox_visible_row_count,
};

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

    /// One listbox's scrollbar mouse-down: up/down arrow buttons step ±1; a thumb
    /// hit consumes without dragging (no drag-follow — the divergence from the combo
    /// model); a track click jumps to the rounded top_index. Returns true if the
    /// click landed inside this listbox's scrollbar.
    fn listbox_scrollbar_mouse_down(
        &mut self,
        id: ChooseMapListboxId,
        list: RectPx,
        row_count: usize,
        x: i32,
        y: i32,
    ) -> bool {
        let Some(scrollbar) = choose_map_listbox_scrollbar_rect(row_count, list) else {
            return false;
        };
        if !scrollbar.contains(x, y) {
            return false;
        }
        let visible_rows = choose_map_listbox_visible_row_count(list);
        if y < scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H {
            self.scroll_listbox_by_rows(id, row_count, visible_rows, -1);
            return true;
        }
        if y >= scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H {
            self.scroll_listbox_by_rows(id, row_count, visible_rows, 1);
            return true;
        }
        if let Some(thumb) =
            choose_map_listbox_scroll_thumb_rect(row_count, self.top_index(id), list)
        {
            if thumb.contains(x, y) {
                return true;
            }
            if let Some(top_index) = choose_map_listbox_top_index_from_track_click(
                row_count,
                self.top_index(id),
                list,
                y,
            ) {
                self.set_top_index_clamped(id, row_count, visible_rows, top_index);
            }
        }
        true
    }

    /// Dispatch a modal mouse-down that already missed the OK/Cancel/Random buttons:
    /// listbox scrollbars (mode, then map), then a mode-row click (re-filters), then a
    /// map-row click. Returns true if consumed. The button hit-test + dialog-contains
    /// fallthrough stay app-side (4D defers the modal chrome/buttons).
    pub fn handle_listbox_mouse_down(
        &mut self,
        layout: &ChooseMapModalLayout,
        modes: &[SkirmishGameMode],
        records: &[SkirmishScenarioRecord],
        x: i32,
        y: i32,
    ) -> bool {
        let mode_row_count = self.mode_row_count(modes);
        let map_row_count = self.map_row_count();
        if self.listbox_scrollbar_mouse_down(
            ChooseMapListboxId::Mode0x6eb,
            layout.mode_list,
            mode_row_count,
            x,
            y,
        ) {
            return true;
        }
        if self.listbox_scrollbar_mouse_down(
            ChooseMapListboxId::Map0x553,
            layout.map_list,
            map_row_count,
            x,
            y,
        ) {
            return true;
        }
        if let Some(mode_idx) =
            choose_map_listbox_row_at(layout.mode_list, mode_row_count, self.mode_top_index, x, y)
        {
            if let Some(mode) = modes.get(mode_idx) {
                self.select_mode(mode.id, modes, records);
            }
            return true;
        }
        if let Some(filtered_idx) =
            choose_map_listbox_row_at(layout.map_list, map_row_count, self.map_top_index, x, y)
        {
            self.select_map_filtered_row(filtered_idx);
            return true;
        }
        false
    }

    /// Mouse wheel over the modal. Four branches preserved byte-for-byte: cursor over
    /// map_list (checked FIRST) → map, else mode_list → mode, else consume; lines==0 →
    /// consume/no-scroll; lines>0 → up by ceil(|lines|); lines<0 → down by ceil(|lines|).
    pub fn handle_listbox_wheel(
        &mut self,
        layout: &ChooseMapModalLayout,
        modes: &[SkirmishGameMode],
        x: i32,
        y: i32,
        lines: f32,
    ) -> bool {
        let id = if layout.map_list.contains(x, y) {
            ChooseMapListboxId::Map0x553
        } else if layout.mode_list.contains(x, y) {
            ChooseMapListboxId::Mode0x6eb
        } else {
            return true;
        };
        if lines == 0.0 {
            return true;
        }
        let rows = if lines > 0.0 {
            -(lines.abs().ceil().max(1.0) as i32)
        } else {
            lines.abs().ceil().max(1.0) as i32
        };
        let list = choose_map_listbox_rect(layout, id);
        let visible_rows = choose_map_listbox_visible_row_count(list);
        let row_count = match id {
            ChooseMapListboxId::Mode0x6eb => self.mode_row_count(modes),
            ChooseMapListboxId::Map0x553 => self.map_row_count(),
        };
        self.scroll_listbox_by_rows(id, row_count, visible_rows, rows);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skirmish_modes::stock_skirmish_modes;
    use crate::ui::skirmish_shell::{
        CHOOSE_MAP_LISTBOX_ROW_H, choose_map_listbox_content_rect, compute_choose_map_modal_layout,
    };

    /// A modal whose map list overflows by `overflow` rows past what fits, so the map
    /// scrollbar exists and `map_top_index` can reach at least `overflow`. The mode
    /// list keeps the stock modes. Map rows are synthesized directly (no scenario
    /// records needed) since the scroll/wheel paths read only `map_row_count`.
    fn modal_with_map_overflow(
        overflow: usize,
        modes: &[SkirmishGameMode],
        layout: &ChooseMapModalLayout,
    ) -> (ChooseMapModalState, usize) {
        let visible = choose_map_listbox_visible_row_count(layout.map_list);
        let row_count = visible + overflow;
        let mut modal = ChooseMapModalState::open(modes[0].id, None, modes, &[]);
        modal.filtered_record_indices = (0..row_count).collect();
        modal.highlighted_filtered_index = Some(0);
        modal.map_top_index = 0;
        (modal, row_count)
    }

    #[test]
    fn wheel_over_map_list_scrolls_down_on_negative_lines() {
        // lines<0 → +ceil(|lines|) rows; cursor over map_list routes to the map list.
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let (mut modal, _) = modal_with_map_overflow(10, &modes, &layout);
        let (x, y) = (layout.map_list.x + 1, layout.map_list.y + 1);
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, -1.5));
        assert_eq!(modal.map_top_index, 2, "ceil(1.5)=2 rows down");
        assert_eq!(modal.mode_top_index, 0, "map wheel leaves mode list alone");
    }

    #[test]
    fn wheel_positive_lines_scroll_up_by_ceil() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let (mut modal, _) = modal_with_map_overflow(10, &modes, &layout);
        modal.map_top_index = 5;
        let (x, y) = (layout.map_list.x + 1, layout.map_list.y + 1);
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, 2.0));
        assert_eq!(modal.map_top_index, 3, "ceil(2.0)=2 rows up from 5");
    }

    #[test]
    fn wheel_zero_lines_consumes_without_scrolling() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let (mut modal, _) = modal_with_map_overflow(10, &modes, &layout);
        modal.map_top_index = 4;
        let (x, y) = (layout.map_list.x + 1, layout.map_list.y + 1);
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, 0.0));
        assert_eq!(modal.map_top_index, 4, "zero lines: consumed, no scroll");
    }

    #[test]
    fn wheel_over_mode_list_leaves_map_index_untouched() {
        // Routing branch: cursor over mode_list must NOT scroll the map list.
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let (mut modal, _) = modal_with_map_overflow(10, &modes, &layout);
        modal.map_top_index = 4;
        let (x, y) = (layout.mode_list.x + 1, layout.mode_list.y + 1);
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, -2.0));
        assert_eq!(
            modal.map_top_index, 4,
            "mode wheel does not move the map list"
        );
    }

    #[test]
    fn wheel_outside_both_lists_consumes_without_scrolling() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let (mut modal, _) = modal_with_map_overflow(10, &modes, &layout);
        modal.map_top_index = 4;
        // (0,0) is the screen corner, outside both listbox rects.
        assert!(modal.handle_listbox_wheel(&layout, &modes, 0, 0, -3.0));
        assert_eq!(modal.map_top_index, 4, "outside lists: consumed, no scroll");
        assert_eq!(modal.mode_top_index, 0);
    }

    #[test]
    fn mouse_down_scrollbar_down_arrow_steps_map_top_index() {
        // Click the map listbox's down-arrow button → +1 row.
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let (mut modal, row_count) = modal_with_map_overflow(10, &modes, &layout);
        let scrollbar = choose_map_listbox_scrollbar_rect(row_count, layout.map_list)
            .expect("overflowing map list has a scrollbar");
        let x = scrollbar.x + 1;
        let y = scrollbar.y + scrollbar.h - 1; // inside the bottom arrow button
        assert!(modal.handle_listbox_mouse_down(&layout, &modes, &[], x, y));
        assert_eq!(modal.map_top_index, 1);
    }

    #[test]
    fn mouse_down_map_row_selects_filtered_row() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let (mut modal, row_count) = modal_with_map_overflow(10, &modes, &layout);
        modal.highlighted_filtered_index = None;
        let content = choose_map_listbox_content_rect(row_count, layout.map_list);
        // Second visible row (row index 1) at top_index 0.
        let (x, y) = (content.x + 1, content.y + CHOOSE_MAP_LISTBOX_ROW_H + 1);
        assert!(modal.handle_listbox_mouse_down(&layout, &modes, &[], x, y));
        assert_eq!(modal.highlighted_filtered_index, Some(1));
    }
}
