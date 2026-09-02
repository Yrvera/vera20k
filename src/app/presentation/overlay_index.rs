//! `OverlayRenderIndex` — the app's explicit presentation index over dynamic
//! overlay cells (F08).
//!
//! Frozen contract:
//! - source-map coordinates retain source order;
//! - an update to an existing coordinate retains its slot;
//! - the first dynamic appearance appends in `SimFrameOutput.overlay_updates`
//!   order;
//! - clearing leaves a coordinate tombstone that draws nothing because the
//!   live `OverlayGrid` is authoritative and empty there;
//! - reoccupation with a different ID reuses the slot and reads the new live
//!   value;
//! - full restore appends only missing occupied coordinates in restore-output
//!   order (an existing coordinate additionally refreshes its source-seed
//!   identity, which is invisible in-match because the live grid wins);
//!
//! Residual (recorded, not silent): entries still carry the source-seed
//! `overlay_id`/`frame` as the pre-runtime display fallback (shell/loading
//! render without a simulation); in-match, live `OverlayGrid` identity always
//! wins in `overlay_render_identity`.

use crate::map::overlay::OverlayEntry;

#[derive(Default)]
pub(crate) struct OverlayRenderIndex {
    entries: Vec<OverlayEntry>,
}

impl OverlayRenderIndex {
    /// Install the source-map overlay list, in source order.
    pub(crate) fn replace_from_source(&mut self, entries: Vec<OverlayEntry>) {
        self.entries = entries;
    }

    /// Upsert authoritative occupied cells by coordinate: existing
    /// coordinates keep their slot (only the seed identity refreshes); new
    /// coordinates append in candidate order. Returns inserted-or-changed.
    pub(crate) fn upsert_occupied(&mut self, candidates: Vec<OverlayEntry>) -> usize {
        let mut by_coordinate: std::collections::HashMap<(u16, u16), usize> = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| ((entry.rx, entry.ry), index))
            .collect();
        let mut synced = 0;
        for candidate in candidates {
            let coordinate = (candidate.rx, candidate.ry);
            if let Some(&index) = by_coordinate.get(&coordinate) {
                let entry = &mut self.entries[index];
                if entry.overlay_id != candidate.overlay_id || entry.frame != candidate.frame {
                    entry.overlay_id = candidate.overlay_id;
                    entry.frame = candidate.frame;
                    synced += 1;
                }
            } else {
                by_coordinate.insert(coordinate, self.entries.len());
                self.entries.push(candidate);
                synced += 1;
            }
        }
        synced
    }

    /// Drop the render entries for cells whose overlay identity was erased.
    ///
    /// [`OverlayRenderIndex::upsert_occupied`] only ever adds or rewrites an
    /// occupied cell, so a removal has to arrive on its own channel. Returns the
    /// number of entries actually dropped.
    pub(crate) fn remove_cells(&mut self, cells: &[(u16, u16)]) -> usize {
        if cells.is_empty() {
            return 0;
        }
        let removed: std::collections::HashSet<(u16, u16)> = cells.iter().copied().collect();
        let before = self.entries.len();
        self.entries
            .retain(|entry| !removed.contains(&(entry.rx, entry.ry)));
        before - self.entries.len()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, OverlayEntry> {
        self.entries.iter()
    }

    pub(crate) fn as_slice(&self) -> &[OverlayEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rx: u16, ry: u16, id: u8, frame: u8) -> OverlayEntry {
        OverlayEntry {
            rx,
            ry,
            overlay_id: id,
            frame,
        }
    }

    /// F08 matrix: source order, dynamic-append order, slot retention on
    /// update and reoccupation, and restore appending only missing
    /// coordinates in restore order.
    #[test]
    fn overlay_render_index_preserves_source_dynamic_tombstone_and_restore_order() {
        let mut index = OverlayRenderIndex::default();
        // Source install keeps source order.
        index.replace_from_source(vec![entry(5, 5, 10, 0), entry(3, 3, 11, 0)]);
        let coords: Vec<_> = index.iter().map(|e| (e.rx, e.ry)).collect();
        assert_eq!(coords, vec![(5, 5), (3, 3)]);

        // First dynamic appearances append in update order.
        assert_eq!(
            index.upsert_occupied(vec![entry(9, 9, 20, 1), entry(1, 1, 21, 2)]),
            2
        );
        let coords: Vec<_> = index.iter().map(|e| (e.rx, e.ry)).collect();
        assert_eq!(coords, vec![(5, 5), (3, 3), (9, 9), (1, 1)]);

        // Updating an existing coordinate retains its slot.
        assert_eq!(index.upsert_occupied(vec![entry(3, 3, 12, 4)]), 1);
        let third = &index.as_slice()[1];
        assert_eq!(
            (third.rx, third.ry, third.overlay_id, third.frame),
            (3, 3, 12, 4)
        );
        assert_eq!(index.as_slice().len(), 4);

        // Reoccupation with a different ID reuses the slot (tombstoned
        // coordinates draw nothing in-match because the live grid is empty;
        // the index itself never removes the coordinate).
        assert_eq!(index.upsert_occupied(vec![entry(9, 9, 30, 0)]), 1);
        assert_eq!(index.as_slice()[2].overlay_id, 30);

        // Restore appends only missing occupied coordinates, in restore order.
        let restored = vec![entry(5, 5, 10, 0), entry(7, 7, 40, 0), entry(2, 2, 41, 0)];
        index.upsert_occupied(restored);
        let coords: Vec<_> = index.iter().map(|e| (e.rx, e.ry)).collect();
        assert_eq!(coords, vec![(5, 5), (3, 3), (9, 9), (1, 1), (7, 7), (2, 2)]);
    }
}
