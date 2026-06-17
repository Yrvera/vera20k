//! Unified scrollbar geometry for the skirmish shell's two scrollable lists — the
//! combo dropdown (Model A) and the choose-map listbox (Model B). Both lists share
//! identical thumb-height, thumb-position, and pointer→top_index math; the points
//! where they legitimately differ — row height, how the visible-row count is
//! derived, and what an empty list does to the thumb — are explicit parameters on
//! `ScrollModel`. The other three model differences (drag, wheel, cursor storage)
//! are caller behavior and are recorded here only for documentation; they change no
//! value this module computes.
//!
//! Depends only on `RectPx` and the scrollbar constants from `layout`; holds no
//! state, render, or UI dependency (pure integer pixel geometry).

use super::layout::{
    CHOOSE_MAP_LISTBOX_ROW_H, COMBO_DROPDOWN_ROW_H, COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
    COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H, RectPx,
};

/// Where a model derives its visible-row count. Combo dropdowns cap visible rows
/// per control (Side = 7, Color/Start = 9, AiType/Team = unbounded); the choose-map
/// listbox derives them geometrically from its rect height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleRowSource {
    /// `item_count.min(cap)`, or unbounded (`item_count`) when `cap == 0`.
    PerControlCap(i32),
    /// `(rect_h / row_h).max(0)`.
    GeometricFromRect,
}

/// What `thumb_height` returns for an empty/degenerate list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyThumbPath {
    /// `track_h.max(MIN_THUMB_H)` — combo Model A (unreachable under its scrollbar gate).
    MaxThumb,
    /// `None` — listbox Model B (reachable when the rect is shorter than one row).
    NoThumb,
}

/// The six divergence axes between the two scroll models, parameterized. The first
/// three drive the geometry below; `thumb_drag_enabled`, `wheel_active`, and the
/// cursor storage (which lives in the caller, not here) are honored by the call
/// sites — the combo wires drag + treats the wheel as inert + stores a fused
/// `Option<OpenComboDropdown>`; the listbox skips drag + handles the wheel + stores
/// two bare `usize`. They are recorded so one struct documents all six.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollModel {
    pub row_h: i32,
    pub visible_row_source: VisibleRowSource,
    pub thumb_drag_enabled: bool,
    pub wheel_active: bool,
    pub empty_path: EmptyThumbPath,
}

impl ScrollModel {
    /// The combo dropdown (Model A): per-control cap, drag on, wheel inert,
    /// full-track thumb on empty. `cap` is the per-combo max-visible (0 = unbounded).
    pub const fn combo(cap: i32) -> Self {
        Self {
            row_h: COMBO_DROPDOWN_ROW_H,
            visible_row_source: VisibleRowSource::PerControlCap(cap),
            thumb_drag_enabled: true,
            wheel_active: false,
            empty_path: EmptyThumbPath::MaxThumb,
        }
    }

    /// The choose-map listbox (Model B): geometric visible rows, no drag, wheel
    /// active, no thumb on empty.
    pub const fn listbox() -> Self {
        Self {
            row_h: CHOOSE_MAP_LISTBOX_ROW_H,
            visible_row_source: VisibleRowSource::GeometricFromRect,
            thumb_drag_enabled: false,
            wheel_active: true,
            empty_path: EmptyThumbPath::NoThumb,
        }
    }

    /// Visible-row count. `PerControlCap(0)` is unbounded (`item_count`); the
    /// geometric source ignores `item_count`.
    pub fn visible_rows(&self, item_count: usize, rect_h: i32) -> usize {
        match self.visible_row_source {
            VisibleRowSource::PerControlCap(cap) => {
                if cap > 0 {
                    item_count.min(cap as usize)
                } else {
                    item_count
                }
            }
            VisibleRowSource::GeometricFromRect => (rect_h / self.row_h).max(0) as usize,
        }
    }

    /// `item_count − visible_rows`, saturating. Matches both legacy `max_top`.
    pub fn max_top_index(&self, item_count: usize, visible_rows: usize) -> usize {
        item_count.saturating_sub(visible_rows)
    }

    /// Thumb height in pixels, or `None` when the list is empty/degenerate under
    /// `empty_path`. `scrollbar_h` is the full track (scrollbar rect) height.
    pub fn thumb_height(
        &self,
        visible_rows: usize,
        item_count: usize,
        scrollbar_h: i32,
    ) -> Option<i32> {
        let track_h = (scrollbar_h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2).max(1);
        if item_count == 0 || visible_rows == 0 {
            return match self.empty_path {
                EmptyThumbPath::MaxThumb => Some(track_h.max(COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H)),
                EmptyThumbPath::NoThumb => None,
            };
        }
        Some(
            ((track_h * visible_rows as i32) / item_count as i32)
                .max(COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H)
                .min(track_h),
        )
    }

    /// Thumb top-Y inside `scrollbar` for `top_index`. `thumb_h` from `thumb_height`;
    /// `max_top` from `max_top_index`.
    pub fn thumb_y(
        &self,
        scrollbar: RectPx,
        thumb_h: i32,
        top_index: usize,
        max_top: usize,
    ) -> i32 {
        let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb_h).max(1);
        scrollbar.y
            + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H
            + if max_top == 0 {
                0
            } else {
                (track_span * top_index.min(max_top) as i32) / max_top as i32
            }
    }

    /// Pointer→top_index — the shared core for BOTH a track click and a thumb drag.
    /// `thumb_top_candidate` is `mouse_y − thumb_h/2` for a track click, or
    /// `mouse_y − grab_offset_y` for a drag.
    pub fn top_index_from_thumb_top(
        &self,
        scrollbar: RectPx,
        thumb_h: i32,
        max_top: usize,
        thumb_top_candidate: i32,
    ) -> usize {
        if max_top == 0 {
            return 0;
        }
        let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb_h).max(1);
        let thumb_top = thumb_top_candidate.clamp(
            scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
            scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H - thumb_h,
        );
        let local = thumb_top - scrollbar.y - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H;
        ((local * max_top as i32 + track_span / 2) / track_span) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::super::layout::{
        COMBO_DROPDOWN_SCROLLBAR_BUTTON_H as BUTTON_H,
        COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H as MIN_THUMB_H,
    };
    use super::*;

    // ---- Verbatim reference copies of the pre-4E legacy math (the executable spec).
    //      These NEVER change; the unified primitive is proven equal to them. ----

    /// Model A: `combo_dropdown_thumb_height` (combos.rs pre-4E).
    fn legacy_combo_thumb_height(visible_rows: usize, item_count: usize, scrollbar_h: i32) -> i32 {
        let track_h = (scrollbar_h - BUTTON_H * 2).max(1);
        if item_count == 0 {
            return track_h.max(MIN_THUMB_H);
        }
        ((track_h * visible_rows as i32) / item_count as i32)
            .max(MIN_THUMB_H)
            .min(track_h)
    }

    /// Model B: thumb-height portion of `choose_map_listbox_scroll_thumb_rect` (layout.rs pre-4E).
    fn legacy_listbox_thumb_height(
        visible_rows: usize,
        row_count: usize,
        scrollbar_h: i32,
    ) -> Option<i32> {
        if row_count == 0 || visible_rows == 0 {
            return None;
        }
        let track_h = (scrollbar_h - BUTTON_H * 2).max(1);
        Some(
            ((track_h * visible_rows as i32) / row_count as i32)
                .max(MIN_THUMB_H)
                .min(track_h),
        )
    }

    /// Model A `thumb_y` (combo_dropdown_scroll_thumb_rect pre-4E).
    fn legacy_combo_thumb_y(
        scrollbar: RectPx,
        thumb_h: i32,
        top_index: usize,
        max_top: usize,
    ) -> i32 {
        let track_span = (scrollbar.h - BUTTON_H * 2 - thumb_h).max(1);
        scrollbar.y
            + BUTTON_H
            + if max_top == 0 {
                0
            } else {
                (track_span * top_index.min(max_top) as i32) / max_top as i32
            }
    }

    /// Model B `thumb_y` (choose_map_listbox_scroll_thumb_rect pre-4E) — track_h-based form.
    fn legacy_listbox_thumb_y(
        scrollbar: RectPx,
        thumb_h: i32,
        top_index: usize,
        max_top: usize,
    ) -> i32 {
        let track_h = (scrollbar.h - BUTTON_H * 2).max(1);
        let track_span = (track_h - thumb_h).max(1);
        scrollbar.y
            + BUTTON_H
            + if max_top == 0 {
                0
            } else {
                (track_span * top_index.min(max_top) as i32) / max_top as i32
            }
    }

    /// Shared pointer→top_index core (identical in A track-click, A drag, B track-click pre-4E).
    fn legacy_pointer_to_top(
        scrollbar: RectPx,
        thumb_h: i32,
        max_top: usize,
        candidate: i32,
    ) -> usize {
        if max_top == 0 {
            return 0;
        }
        let track_span = (scrollbar.h - BUTTON_H * 2 - thumb_h).max(1);
        let thumb_top = candidate.clamp(
            scrollbar.y + BUTTON_H,
            scrollbar.y + scrollbar.h - BUTTON_H - thumb_h,
        );
        let local = thumb_top - scrollbar.y - BUTTON_H;
        ((local * max_top as i32 + track_span / 2) / track_span) as usize
    }

    /// Representative scrollbars incl. degenerate `track_h`-clamp geometries.
    fn scrollbars() -> Vec<RectPx> {
        vec![
            RectPx::new(513, 127, 20, 343), // the frozen choose-map listbox geometry
            RectPx::new(100, 50, 20, 23 * 7), // a Side combo dropdown (cap 7, row 23)
            RectPx::new(100, 50, 20, 23 * 9), // Color/Start dropdown (cap 9)
            RectPx::new(0, 0, 20, 44),      // degenerate: scrollbar.h - 44 == 0  -> track_h clamp
            RectPx::new(0, 0, 20, 45),      // degenerate: track_h == 1
            RectPx::new(0, 0, 20, 46),
        ]
    }

    const N: usize = 24; // boundary count ceiling; bump if a stock combo can exceed it

    #[test]
    fn unbounded_combo_never_needs_a_scrollbar() {
        // PerControlCap(0): visible_rows == item_count => item > visible is always false.
        let m = ScrollModel::combo(0);
        for n in 0..=N {
            assert_eq!(m.visible_rows(n, 9_999), n);
        }
    }

    #[test]
    fn unified_matches_combo_model_over_boundaries() {
        for &cap in &[7i32, 9] {
            let model = ScrollModel::combo(cap);
            for sb in scrollbars() {
                // item_count==0 is unreachable under the gate (see reachability test); start at 1.
                for item_count in 1..=N {
                    let visible_rows = model.visible_rows(item_count, sb.h);
                    if visible_rows == 0 {
                        continue; // unreachable for a combo (visible==0 <=> item==0)
                    }
                    if item_count <= visible_rows {
                        continue; // no scrollbar => thumb never built
                    }
                    let thumb_h = model.thumb_height(visible_rows, item_count, sb.h).unwrap();
                    assert_eq!(
                        thumb_h,
                        legacy_combo_thumb_height(visible_rows, item_count, sb.h),
                        "thumb_h cap={cap} sb={sb:?} n={item_count}"
                    );
                    let max_top = model.max_top_index(item_count, visible_rows);
                    assert_eq!(max_top, item_count.saturating_sub(visible_rows));
                    for top_index in 0..=max_top {
                        assert_eq!(
                            model.thumb_y(sb, thumb_h, top_index, max_top),
                            legacy_combo_thumb_y(sb, thumb_h, top_index, max_top),
                            "thumb_y cap={cap} sb={sb:?} n={item_count} top={top_index}"
                        );
                    }
                    // The pointer->index clamp requires lo <= hi
                    // (scrollbar.h >= 2*BUTTON_H + thumb_h). Real combo scrollbars are
                    // >=161px so always satisfy it; the degenerate track_h-clamp
                    // fixtures do not, and both unified + legacy clamp identically
                    // (they would panic identically), so only compare where it is
                    // well-formed.
                    if sb.h >= 2 * BUTTON_H + thumb_h {
                        for my in (sb.y - 5)..=(sb.y + sb.h + 5) {
                            // track click anchor:
                            let track_anchor = my - thumb_h / 2;
                            assert_eq!(
                                model.top_index_from_thumb_top(sb, thumb_h, max_top, track_anchor),
                                legacy_pointer_to_top(sb, thumb_h, max_top, track_anchor),
                                "track cap={cap} sb={sb:?} n={item_count} my={my}"
                            );
                            // drag anchor (combo-only) — grab offset of 3px from thumb top:
                            let drag_anchor = my - 3;
                            assert_eq!(
                                model.top_index_from_thumb_top(sb, thumb_h, max_top, drag_anchor),
                                legacy_pointer_to_top(sb, thumb_h, max_top, drag_anchor),
                                "drag cap={cap} sb={sb:?} n={item_count} my={my}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn unified_matches_listbox_model_over_boundaries() {
        let model = ScrollModel::listbox();
        for sb in scrollbars() {
            let visible_rows = model.visible_rows(0, sb.h); // geometric: item_count ignored
            for row_count in 0..=N {
                // thumb_height Option must match B EXACTLY, incl. the reachable
                // visible_rows==0 (rect shorter than one row) -> None case.
                assert_eq!(
                    model.thumb_height(visible_rows, row_count, sb.h),
                    legacy_listbox_thumb_height(visible_rows, row_count, sb.h),
                    "listbox thumb_h sb={sb:?} rows={row_count} vis={visible_rows}"
                );
                if row_count == 0 || visible_rows == 0 || row_count <= visible_rows {
                    continue; // no thumb / no scrollbar
                }
                let thumb_h = model.thumb_height(visible_rows, row_count, sb.h).unwrap();
                let max_top = model.max_top_index(row_count, visible_rows);
                assert_eq!(max_top, row_count.saturating_sub(visible_rows));
                for top_index in 0..=max_top {
                    assert_eq!(
                        model.thumb_y(sb, thumb_h, top_index, max_top),
                        legacy_listbox_thumb_y(sb, thumb_h, top_index, max_top),
                        "listbox thumb_y sb={sb:?} rows={row_count} top={top_index}"
                    );
                }
                // See the combo loop: the pointer->index clamp needs lo <= hi; real
                // listbox scrollbars are >=343px, the degenerate fixtures are not.
                if sb.h >= 2 * BUTTON_H + thumb_h {
                    for my in (sb.y - 5)..=(sb.y + sb.h + 5) {
                        let anchor = my - thumb_h / 2;
                        assert_eq!(
                            model.top_index_from_thumb_top(sb, thumb_h, max_top, anchor),
                            legacy_pointer_to_top(sb, thumb_h, max_top, anchor),
                            "listbox track sb={sb:?} rows={row_count} my={my}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn empty_paths_are_reachability_gated() {
        // Step-1 proof, executable: A's empty trigger (item_count==0) is UNREACHABLE
        // under the combo scrollbar gate because visible_rows==0 <=> item_count==0 =>
        // `item > visible` (0 > 0) is false => no scrollbar. B's visible_rows==0 (rect
        // shorter than one row) IS reachable and yields no thumb.
        let combo = ScrollModel::combo(7);
        assert_eq!(
            combo.visible_rows(0, 9_999),
            0,
            "combo empty => 0 visible => gate closed"
        );

        let listbox = ScrollModel::listbox();
        let short_h = CHOOSE_MAP_LISTBOX_ROW_H - 1; // rect shorter than one row
        assert_eq!(listbox.visible_rows(0, short_h), 0);
        // With rows present and a tall-enough scrollbar the gate is open, but the
        // thumb is None under NoThumb:
        assert_eq!(listbox.thumb_height(0, 5, short_h + 200), None);

        // The combo's MaxThumb path, if it WERE reached, returns a full-track thumb —
        // pinned so a future change to the (dead) branch is visible:
        let track_h = (100 - BUTTON_H * 2).max(1);
        assert_eq!(
            combo.thumb_height(0, 0, 100),
            Some(track_h.max(MIN_THUMB_H))
        );
    }
}
