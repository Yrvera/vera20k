//! Overlay-direct bridge destruction walker.
//!
//! Drives full-bridge collapse from a single hit on a cell whose overlay byte
//! is still in the raw body range (per the verification doc, raw-overlay
//! cells route here, not through the state machine). Distinct from the
//! state-machine drivers in `bridge_state/mod.rs`, which handle the
//! late-stage progression after overlays have been transitioned.
//!
//! ## Dependency rules
//! Same as sim/: depends on rules/ + map/; never render / ui / audio / net.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::sim::bridge_state::{BridgeRuntimeState, StateOutcome};

impl BridgeRuntimeState {
    /// Overlay-direct HIGH walker entry. Three responsibilities:
    /// 1. Classify the input cell's overlay byte to pick NS or EW walker.
    /// 2. Pre-walk start-cell shift: read the body-axis neighbors to find
    ///    a stable mid before walking. Multiple hits on different cells of
    ///    the same bridge converge to the same walker start.
    /// 3. Forward the shifted coord to the appropriate walker.
    ///
    /// Returns the walker's outcome (`Collapsed` once Task 7 lands;
    /// `NoChange` for now). When the input overlay is not in the HIGH body
    /// range, the entry returns `NoChange` without touching state.
    pub fn destroy_bridge_high(
        &mut self,
        rx: u16,
        ry: u16,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let overlay = cell.overlay_byte;
        if Self::is_ns_walker_overlay_high(overlay) {
            let (sx, sy) = self.find_walker_start_high_ns(rx, ry);
            return self.destroy_bridge_walker_ns_high(sx, sy, terrain);
        }
        if Self::is_ew_walker_overlay_high(overlay) {
            let (sx, sy) = self.find_walker_start_high_ew(rx, ry);
            return self.destroy_bridge_walker_ew_high(sx, sy, terrain);
        }
        StateOutcome::NoChange
    }

    /// Overlay-direct LOW walker entry. Same shape as `destroy_bridge_high`
    /// with overlay ranges shifted to the LOW body range
    /// (`[0x4A..=0x65]`).
    pub fn destroy_bridge_low(
        &mut self,
        rx: u16,
        ry: u16,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let overlay = cell.overlay_byte;
        if Self::is_ns_walker_overlay_low(overlay) {
            let (sx, sy) = self.find_walker_start_low_ns(rx, ry);
            return self.destroy_bridge_walker_ns_low(sx, sy, terrain);
        }
        if Self::is_ew_walker_overlay_low(overlay) {
            let (sx, sy) = self.find_walker_start_low_ew(rx, ry);
            return self.destroy_bridge_walker_ew_low(sx, sy, terrain);
        }
        StateOutcome::NoChange
    }

    // ----- Pre-walk start-cell shift (mirror of binary's 3-case neighbor
    // check at the walker entries). -----

    fn find_walker_start_high_ns(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0xCD..=0xE8).contains(&o);
        // Body-axis neighbor 1 north of input.
        let north_off = ry == 0
            || self
                .cell(rx, ry - 1)
                .map(|c| !in_range(c.overlay_byte))
                .unwrap_or(true);
        if north_off {
            return (rx, ry.saturating_add(1));
        }
        // Body-axis neighbor 2 north of input.
        let north2_on = ry >= 2
            && self
                .cell(rx, ry - 2)
                .map(|c| in_range(c.overlay_byte))
                .unwrap_or(false);
        if north2_on {
            return (rx, ry - 1);
        }
        (rx, ry)
    }

    fn find_walker_start_high_ew(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0xCD..=0xE8).contains(&o);
        let west_off = rx == 0
            || self
                .cell(rx - 1, ry)
                .map(|c| !in_range(c.overlay_byte))
                .unwrap_or(true);
        if west_off {
            return (rx.saturating_add(1), ry);
        }
        let west2_on = rx >= 2
            && self
                .cell(rx - 2, ry)
                .map(|c| in_range(c.overlay_byte))
                .unwrap_or(false);
        if west2_on {
            return (rx - 1, ry);
        }
        (rx, ry)
    }

    fn find_walker_start_low_ns(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0x4A..=0x65).contains(&o);
        let north_off = ry == 0
            || self
                .cell(rx, ry - 1)
                .map(|c| !in_range(c.overlay_byte))
                .unwrap_or(true);
        if north_off {
            return (rx, ry.saturating_add(1));
        }
        let north2_on = ry >= 2
            && self
                .cell(rx, ry - 2)
                .map(|c| in_range(c.overlay_byte))
                .unwrap_or(false);
        if north2_on {
            return (rx, ry - 1);
        }
        (rx, ry)
    }

    fn find_walker_start_low_ew(&self, rx: u16, ry: u16) -> (u16, u16) {
        let in_range = |o: u8| (0x4A..=0x65).contains(&o);
        let west_off = rx == 0
            || self
                .cell(rx - 1, ry)
                .map(|c| !in_range(c.overlay_byte))
                .unwrap_or(true);
        if west_off {
            return (rx.saturating_add(1), ry);
        }
        let west2_on = rx >= 2
            && self
                .cell(rx - 2, ry)
                .map(|c| in_range(c.overlay_byte))
                .unwrap_or(false);
        if west2_on {
            return (rx - 1, ry);
        }
        (rx, ry)
    }

    // ----- Axis classification by overlay byte. -----

    pub(super) fn is_ns_walker_overlay_high(overlay: u8) -> bool {
        // HIGH NS axis sub-range:
        //   [0xCD..=0xD5] ∪ [0xDF..=0xE2] ∪ {0xE7}
        (0xCD..=0xD5).contains(&overlay) || (0xDF..=0xE2).contains(&overlay) || overlay == 0xE7
    }

    pub(super) fn is_ew_walker_overlay_high(overlay: u8) -> bool {
        // HIGH EW axis sub-range:
        //   [0xD6..=0xDE] ∪ [0xE3..=0xE6] ∪ {0xE8}
        (0xD6..=0xDE).contains(&overlay) || (0xE3..=0xE6).contains(&overlay) || overlay == 0xE8
    }

    pub(super) fn is_ns_walker_overlay_low(overlay: u8) -> bool {
        // LOW NS axis sub-range:
        //   [0x4A..=0x52] ∪ [0x5C..=0x5F] ∪ {0x64}
        (0x4A..=0x52).contains(&overlay) || (0x5C..=0x5F).contains(&overlay) || overlay == 0x64
    }

    pub(super) fn is_ew_walker_overlay_low(overlay: u8) -> bool {
        // LOW EW axis sub-range:
        //   [0x53..=0x5B] ∪ [0x60..=0x63] ∪ {0x65}
        (0x53..=0x5B).contains(&overlay) || (0x60..=0x63).contains(&overlay) || overlay == 0x65
    }

    // ----- Perpendicular neighbor classifiers used by sibling cascade. -----
    //
    // Indexed table is `pick_destruction_overlay(idx, axis, is_high)`, which
    // already ships in `bridge_specs.rs`. NS body cells consult the EW
    // classifier (perpendicular = west/east); EW body cells consult the NS
    // classifier (perpendicular = north/south).

    /// Classify the EW-axis perpendicular pattern at `(rx, ry)`. Reads the
    /// west and east neighbors' overlay bytes and returns a 0..=15 index.
    /// Bit assignment (matches the binary's switch order at the EW
    /// classifier — east-first, then west):
    /// - bit 0 (val 1): east in `{0xD1, 0xD3, 0xD5, 0xE0}`
    /// - bit 1 (val 2): east in `{0xD4, 0xE7}`
    /// - bit 2 (val 4): west in `{0xD2, 0xD3, 0xD4, 0xE2}`
    /// - bit 3 (val 8): west in `{0xD5, 0xE7}`
    pub(super) fn check_bridge_neighbors_ew_high(&self, rx: u16, ry: u16) -> u8 {
        let east = self
            .cell(rx.saturating_add(1), ry)
            .map(|c| c.overlay_byte)
            .unwrap_or(0);
        let west = if rx > 0 {
            self.cell(rx - 1, ry).map(|c| c.overlay_byte).unwrap_or(0)
        } else {
            0
        };
        let mut idx = 0u8;
        match east {
            0xD1 | 0xD3 | 0xD5 | 0xE0 => idx |= 1,
            0xD4 | 0xE7 => idx |= 2,
            _ => {}
        }
        match west {
            0xD2 | 0xD3 | 0xD4 | 0xE2 => idx |= 4,
            0xD5 | 0xE7 => idx |= 8,
            _ => {}
        }
        idx
    }

    /// Classify the NS-axis perpendicular pattern at `(rx, ry)`. Reads the
    /// north and south neighbors' overlay bytes and returns a 0..=15 index.
    /// Bit assignment (matches the binary — north-first, then south):
    /// - bit 0 (val 1): north in `{0xDA, 0xDC, 0xDE, 0xE4}`
    /// - bit 1 (val 2): north in `{0xDD, 0xE8}`
    /// - bit 2 (val 4): south in `{0xDB, 0xDC, 0xDD, 0xE6}`
    /// - bit 3 (val 8): south in `{0xDE, 0xE8}`
    pub(super) fn check_bridge_neighbors_ns_high(&self, rx: u16, ry: u16) -> u8 {
        let north = if ry > 0 {
            self.cell(rx, ry - 1).map(|c| c.overlay_byte).unwrap_or(0)
        } else {
            0
        };
        let south = self
            .cell(rx, ry.saturating_add(1))
            .map(|c| c.overlay_byte)
            .unwrap_or(0);
        let mut idx = 0u8;
        match north {
            0xDA | 0xDC | 0xDE | 0xE4 => idx |= 1,
            0xDD | 0xE8 => idx |= 2,
            _ => {}
        }
        match south {
            0xDB | 0xDC | 0xDD | 0xE6 => idx |= 4,
            0xDE | 0xE8 => idx |= 8,
            _ => {}
        }
        idx
    }

    // ----- Cell-triple iteration helpers. -----

    fn ns_triple(rx: u16, ry: u16) -> [Option<(u16, u16)>; 3] {
        let north = if ry > 0 { Some((rx, ry - 1)) } else { None };
        let south = Some((rx, ry.saturating_add(1)));
        [Some((rx, ry)), north, south]
    }

    fn ew_triple(rx: u16, ry: u16) -> [Option<(u16, u16)>; 3] {
        let west = if rx > 0 { Some((rx - 1, ry)) } else { None };
        let east = Some((rx.saturating_add(1), ry));
        [Some((rx, ry)), west, east]
    }

    // ----- Sibling-cascade leaves (`apply_bridge_destruction_*_high`). -----

    /// Sibling-cascade leaf for the NS body axis. Validates `(rx, ry)` is
    /// in the HIGH overlay range, computes the perpendicular neighbor
    /// pattern via `check_bridge_neighbors_ew_high`, looks up next-overlay
    /// via the shipped `pick_destruction_overlay` table (HIGH NS), and
    /// writes the (this, north, south) length-axis triple. Returns the
    /// list of cells that hit final-collapse (overlay 0xE7) so the caller
    /// can emit BlowUpBridge actions.
    fn apply_bridge_destruction_ns_high(&mut self, rx: u16, ry: u16) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::{Axis, DamageState};

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else {
            return final_cells;
        };
        let cur = cell.overlay_byte;
        // Outer overlay gate: HIGH range.
        if !(0xCD..=0xE8).contains(&cur) {
            return final_cells;
        }

        let idx = self.check_bridge_neighbors_ew_high(rx, ry);
        // idx == 0 means no perpendicular pattern; cascade leaf is a no-op.
        if idx == 0 {
            return final_cells;
        }

        // Two-stage progression: table lookup for cur < 0xDF, then
        // intermediate fixed transitions for 0xDF/0xE1.
        let next = if cur < 0xDF {
            match pick_destruction_overlay(idx, Axis::NS, true) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0xDF {
            0xE0
        } else if cur == 0xE1 {
            0xE2
        } else {
            // 0xE0, 0xE2, 0xE3..0xE8: no further transition at this cell.
            return final_cells;
        };

        for slot in Self::ns_triple(rx, ry) {
            if let Some(pos) = slot {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if next == 0xE7 {
                        c.damage_state = DamageState::Destroyed;
                        final_cells.push(pos);
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }
        final_cells
    }

    /// Sibling-cascade leaf for the EW body axis. Mirror of
    /// `apply_bridge_destruction_ns_high` with EW axis classifier, EW
    /// table, EW intermediates (0xE3 → 0xE4, 0xE5 → 0xE6), and final 0xE8.
    fn apply_bridge_destruction_ew_high(&mut self, rx: u16, ry: u16) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::{Axis, DamageState};

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else {
            return final_cells;
        };
        let cur = cell.overlay_byte;
        if !(0xCD..=0xE8).contains(&cur) {
            return final_cells;
        }

        let idx = self.check_bridge_neighbors_ns_high(rx, ry);
        if idx == 0 {
            return final_cells;
        }

        let next = if cur < 0xE3 {
            match pick_destruction_overlay(idx, Axis::EW, true) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0xE3 {
            0xE4
        } else if cur == 0xE5 {
            0xE6
        } else {
            return final_cells;
        };

        for slot in Self::ew_triple(rx, ry) {
            if let Some(pos) = slot {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if next == 0xE8 {
                        c.damage_state = DamageState::Destroyed;
                        final_cells.push(pos);
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }
        final_cells
    }

    // ----- Walker bodies (HIGH). LOW remains stubbed for Task 8. -----

    /// HIGH NS-axis walker. Reads the input cell's overlay, picks one of
    /// 5 cases:
    /// - `0xDF` → write 0xE0 to (this, north, south); cascade west sibling
    /// - `0xE1` → write 0xE2 to (this, north, south); cascade east sibling
    /// - `< 0xD3` → write 0xD3 to triple; cascade BOTH (rx±1, ry)
    /// - `[0xD3..=0xD5]` → write 0xE7 to triple (FINAL collapse); cascade
    ///   BOTH; mark zones_dirty
    /// - else → no-op
    ///
    /// Returns `Collapsed` when any cell hit final-collapse (0xE7), or
    /// `Absorbed` for an intermediate transition that touched no final
    /// cell. `NoChange` only when the initial overlay byte is outside the
    /// 5-case set.
    pub(super) fn destroy_bridge_walker_ns_high(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{Axis, DamageState, compute_adjacent_bridges_dirty};

        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let cur = cell.overlay_byte;

        // Pick case + sibling-cascade plan.
        let (next, siblings, is_final): (u8, Vec<(u16, u16)>, bool) = if cur == 0xDF {
            (0xE0, vec![(rx.wrapping_sub(1), ry)], false)
        } else if cur == 0xE1 {
            (0xE2, vec![(rx.saturating_add(1), ry)], false)
        } else if cur < 0xD3 {
            (
                0xD3,
                vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)],
                false,
            )
        } else if (0xD3..=0xD5).contains(&cur) {
            (
                0xE7,
                vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)],
                true,
            )
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();

        // Write the (this, north, south) length-axis triple.
        for (slot, opt_pos) in Self::ns_triple(rx, ry).into_iter().enumerate() {
            if let Some(pos) = opt_pos {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if is_final {
                        c.damage_state = DamageState::Destroyed;
                        destroyed.push(pos);
                        actions.push((pos, slot, CellAction::BlowUpBridge));
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }

        // Cascade to perpendicular siblings via the EW classifier-driven leaf.
        for (sx, sy) in siblings {
            if sx == u16::MAX {
                // wrapping_sub overflow at rx == 0 → west neighbor off-map.
                continue;
            }
            let sibling_finals = self.apply_bridge_destruction_ns_high(sx, sy);
            for pos in sibling_finals {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        if !is_final && destroyed.is_empty() {
            // Intermediate transition only — overlay/damage_state changed
            // but no cell hit final. Caller treats this as "absorbed".
            return StateOutcome::Absorbed;
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::NS);
        StateOutcome::Collapsed {
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult { actions },
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
        }
    }

    /// HIGH EW-axis walker. Mirror of `destroy_bridge_walker_ns_high` with:
    /// - `0xE3` → write 0xE4 to (this, west, east); cascade south sibling
    /// - `0xE5` → write 0xE6 to triple; cascade north sibling
    /// - `< 0xDC` → write 0xDC to triple; cascade BOTH (rx, ry±1)
    /// - `[0xDC..=0xDE]` → write 0xE8 to triple (FINAL); cascade BOTH;
    ///   mark zones_dirty
    /// - else → no-op
    pub(super) fn destroy_bridge_walker_ew_high(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{Axis, DamageState, compute_adjacent_bridges_dirty};

        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let cur = cell.overlay_byte;

        let (next, siblings, is_final): (u8, Vec<(u16, u16)>, bool) = if cur == 0xE3 {
            (0xE4, vec![(rx, ry.saturating_add(1))], false)
        } else if cur == 0xE5 {
            (0xE6, vec![(rx, ry.wrapping_sub(1))], false)
        } else if cur < 0xDC {
            (
                0xDC,
                vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))],
                false,
            )
        } else if (0xDC..=0xDE).contains(&cur) {
            (
                0xE8,
                vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))],
                true,
            )
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();

        for (slot, opt_pos) in Self::ew_triple(rx, ry).into_iter().enumerate() {
            if let Some(pos) = opt_pos {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if is_final {
                        c.damage_state = DamageState::Destroyed;
                        destroyed.push(pos);
                        actions.push((pos, slot, CellAction::BlowUpBridge));
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }

        for (sx, sy) in siblings {
            if sy == u16::MAX {
                continue;
            }
            let sibling_finals = self.apply_bridge_destruction_ew_high(sx, sy);
            for pos in sibling_finals {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        if !is_final && destroyed.is_empty() {
            return StateOutcome::Absorbed;
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::EW);
        StateOutcome::Collapsed {
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult { actions },
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
        }
    }

    // ----- LOW perpendicular neighbor classifiers. -----

    /// Classify the EW-axis perpendicular pattern at `(rx, ry)` for LOW
    /// bridges. Bit assignment (matches the binary's switch order at the
    /// LOW EW classifier — east-first, then west):
    /// - bit 0 (val 1): east in `{0x4E, 0x50, 0x52, 0x5D}`
    /// - bit 1 (val 2): east in `{0x51, 0x64}`
    /// - bit 2 (val 4): west in `{0x4F, 0x50, 0x51, 0x5F}`
    /// - bit 3 (val 8): west in `{0x52, 0x64}`
    pub(super) fn check_bridge_neighbors_ew_low(&self, rx: u16, ry: u16) -> u8 {
        let east = self
            .cell(rx.saturating_add(1), ry)
            .map(|c| c.overlay_byte)
            .unwrap_or(0);
        let west = if rx > 0 {
            self.cell(rx - 1, ry).map(|c| c.overlay_byte).unwrap_or(0)
        } else {
            0
        };
        let mut idx = 0u8;
        match east {
            0x4E | 0x50 | 0x52 | 0x5D => idx |= 1,
            0x51 | 0x64 => idx |= 2,
            _ => {}
        }
        match west {
            0x4F | 0x50 | 0x51 | 0x5F => idx |= 4,
            0x52 | 0x64 => idx |= 8,
            _ => {}
        }
        idx
    }

    /// Classify the NS-axis perpendicular pattern at `(rx, ry)` for LOW
    /// bridges. Bit assignment (matches the binary — north-first, then
    /// south):
    /// - bit 0 (val 1): north in `{0x57, 0x59, 0x5B, 0x61}`
    /// - bit 1 (val 2): north in `{0x5A, 0x65}`
    /// - bit 2 (val 4): south in `{0x58, 0x59, 0x5A, 0x63}`
    /// - bit 3 (val 8): south in `{0x5B, 0x65}`
    pub(super) fn check_bridge_neighbors_ns_low(&self, rx: u16, ry: u16) -> u8 {
        let north = if ry > 0 {
            self.cell(rx, ry - 1).map(|c| c.overlay_byte).unwrap_or(0)
        } else {
            0
        };
        let south = self
            .cell(rx, ry.saturating_add(1))
            .map(|c| c.overlay_byte)
            .unwrap_or(0);
        let mut idx = 0u8;
        match north {
            0x57 | 0x59 | 0x5B | 0x61 => idx |= 1,
            0x5A | 0x65 => idx |= 2,
            _ => {}
        }
        match south {
            0x58 | 0x59 | 0x5A | 0x63 => idx |= 4,
            0x5B | 0x65 => idx |= 8,
            _ => {}
        }
        idx
    }

    // ----- LOW sibling-cascade leaves. -----

    /// Sibling-cascade leaf for the LOW NS body axis. Mirror of
    /// `apply_bridge_destruction_ns_high` with LOW outer gate
    /// (`[0x4A..=0x65]`), LOW NS table, LOW intermediates 0x5C/0x5E, and
    /// final 0x64.
    fn apply_bridge_destruction_ns_low(&mut self, rx: u16, ry: u16) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::{Axis, DamageState};

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else {
            return final_cells;
        };
        let cur = cell.overlay_byte;
        if !(0x4A..=0x65).contains(&cur) {
            return final_cells;
        }

        let idx = self.check_bridge_neighbors_ew_low(rx, ry);
        if idx == 0 {
            return final_cells;
        }

        let next = if cur < 0x5C {
            match pick_destruction_overlay(idx, Axis::NS, false) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0x5C {
            0x5D
        } else if cur == 0x5E {
            0x5F
        } else {
            return final_cells;
        };

        for slot in Self::ns_triple(rx, ry) {
            if let Some(pos) = slot {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if next == 0x64 {
                        c.damage_state = DamageState::Destroyed;
                        final_cells.push(pos);
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }
        final_cells
    }

    /// Sibling-cascade leaf for the LOW EW body axis. Intermediates
    /// 0x60/0x62 → 0x61/0x63; final 0x65.
    fn apply_bridge_destruction_ew_low(&mut self, rx: u16, ry: u16) -> Vec<(u16, u16)> {
        use crate::sim::bridge_specs::pick_destruction_overlay;
        use crate::sim::bridge_state::{Axis, DamageState};

        let mut final_cells = Vec::new();
        let Some(cell) = self.cell(rx, ry).copied() else {
            return final_cells;
        };
        let cur = cell.overlay_byte;
        if !(0x4A..=0x65).contains(&cur) {
            return final_cells;
        }

        let idx = self.check_bridge_neighbors_ns_low(rx, ry);
        if idx == 0 {
            return final_cells;
        }

        let next = if cur < 0x60 {
            match pick_destruction_overlay(idx, Axis::EW, false) {
                Some(n) if n != cur => n,
                _ => return final_cells,
            }
        } else if cur == 0x60 {
            0x61
        } else if cur == 0x62 {
            0x63
        } else {
            return final_cells;
        };

        for slot in Self::ew_triple(rx, ry) {
            if let Some(pos) = slot {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if next == 0x65 {
                        c.damage_state = DamageState::Destroyed;
                        final_cells.push(pos);
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }
        final_cells
    }

    // ----- LOW walker bodies. -----

    /// LOW NS-axis walker. Mirror of `destroy_bridge_walker_ns_high` with
    /// LOW case values:
    /// - `0x5C` → write 0x5D to (this, north, south); cascade west sibling
    /// - `0x5E` → write 0x5F to triple; cascade east sibling
    /// - `< 0x50` → write 0x50 to triple; cascade BOTH (rx±1, ry)
    /// - `[0x50..=0x52]` → write 0x64 to triple (FINAL); cascade BOTH;
    ///   mark zones_dirty
    /// - else → no-op
    pub(super) fn destroy_bridge_walker_ns_low(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{Axis, DamageState, compute_adjacent_bridges_dirty};

        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let cur = cell.overlay_byte;

        let (next, siblings, is_final): (u8, Vec<(u16, u16)>, bool) = if cur == 0x5C {
            (0x5D, vec![(rx.wrapping_sub(1), ry)], false)
        } else if cur == 0x5E {
            (0x5F, vec![(rx.saturating_add(1), ry)], false)
        } else if cur < 0x50 {
            (
                0x50,
                vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)],
                false,
            )
        } else if (0x50..=0x52).contains(&cur) {
            (
                0x64,
                vec![(rx.wrapping_sub(1), ry), (rx.saturating_add(1), ry)],
                true,
            )
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();

        for (slot, opt_pos) in Self::ns_triple(rx, ry).into_iter().enumerate() {
            if let Some(pos) = opt_pos {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if is_final {
                        c.damage_state = DamageState::Destroyed;
                        destroyed.push(pos);
                        actions.push((pos, slot, CellAction::BlowUpBridge));
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }

        for (sx, sy) in siblings {
            if sx == u16::MAX {
                continue;
            }
            let sibling_finals = self.apply_bridge_destruction_ns_low(sx, sy);
            for pos in sibling_finals {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        if !is_final && destroyed.is_empty() {
            return StateOutcome::Absorbed;
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::NS);
        StateOutcome::Collapsed {
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult { actions },
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
        }
    }

    /// LOW EW-axis walker. Mirror of NS LOW with EW case values:
    /// - `0x60` → write 0x61 to (this, west, east); cascade south sibling
    /// - `0x62` → write 0x63 to triple; cascade north sibling
    /// - `< 0x59` → write 0x59 to triple; cascade BOTH (rx, ry±1)
    /// - `[0x59..=0x5B]` → write 0x65 to triple (FINAL); cascade BOTH;
    ///   mark zones_dirty
    /// - else → no-op
    pub(super) fn destroy_bridge_walker_ew_low(
        &mut self,
        rx: u16,
        ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        use crate::sim::bridge_specs::{CellAction, SetBridgeDirectionResult};
        use crate::sim::bridge_state::{Axis, DamageState, compute_adjacent_bridges_dirty};

        let Some(cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };
        let cur = cell.overlay_byte;

        let (next, siblings, is_final): (u8, Vec<(u16, u16)>, bool) = if cur == 0x60 {
            (0x61, vec![(rx, ry.saturating_add(1))], false)
        } else if cur == 0x62 {
            (0x63, vec![(rx, ry.wrapping_sub(1))], false)
        } else if cur < 0x59 {
            (
                0x59,
                vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))],
                false,
            )
        } else if (0x59..=0x5B).contains(&cur) {
            (
                0x65,
                vec![(rx, ry.wrapping_sub(1)), (rx, ry.saturating_add(1))],
                true,
            )
        } else {
            return StateOutcome::NoChange;
        };

        let mut destroyed: Vec<(u16, u16)> = Vec::new();
        let mut actions: Vec<((u16, u16), usize, CellAction)> = Vec::new();

        for (slot, opt_pos) in Self::ew_triple(rx, ry).into_iter().enumerate() {
            if let Some(pos) = opt_pos {
                if let Some(c) = self.cell_mut(pos.0, pos.1) {
                    c.overlay_byte = next;
                    if is_final {
                        c.damage_state = DamageState::Destroyed;
                        destroyed.push(pos);
                        actions.push((pos, slot, CellAction::BlowUpBridge));
                    } else {
                        c.damage_state = DamageState::Damaged;
                    }
                }
            }
        }

        for (sx, sy) in siblings {
            if sy == u16::MAX {
                continue;
            }
            let sibling_finals = self.apply_bridge_destruction_ew_low(sx, sy);
            for pos in sibling_finals {
                if !destroyed.contains(&pos) {
                    destroyed.push(pos);
                    actions.push((pos, 0, CellAction::BlowUpBridge));
                }
            }
        }

        if !is_final && destroyed.is_empty() {
            return StateOutcome::Absorbed;
        }

        let adj = compute_adjacent_bridges_dirty(rx, ry, Axis::EW);
        StateOutcome::Collapsed {
            destroyed_cells: destroyed,
            set_bridge_direction: SetBridgeDirectionResult { actions },
            adjacent_bridges_dirty: adj,
            zones_dirty: is_final,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeRuntimeCell, BridgeheadAnchorClass, DamageState,
    };

    fn empty_terrain() -> ResolvedTerrainGrid {
        ResolvedTerrainGrid::from_cells(0, 0, Vec::new())
    }

    fn seed_high_body_cell(state: &mut BridgeRuntimeState, rx: u16, ry: u16, overlay: u8) {
        state.test_seed_cell(
            rx,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: overlay,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
    }

    #[test]
    fn destroy_bridge_high_returns_nochange_for_low_overlay() {
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0x5A);
        let terrain = empty_terrain();
        assert_eq!(
            state.destroy_bridge_high(0, 0, &terrain),
            StateOutcome::NoChange
        );
    }

    #[test]
    fn destroy_bridge_low_returns_nochange_for_high_overlay() {
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xD0);
        let terrain = empty_terrain();
        assert_eq!(
            state.destroy_bridge_low(0, 0, &terrain),
            StateOutcome::NoChange
        );
    }

    #[test]
    fn axis_classifiers_partition_high_range() {
        // Sample a few overlay values from each sub-range.
        for v in [0xCDu8, 0xD0, 0xD5, 0xDF, 0xE0, 0xE2, 0xE7] {
            assert!(BridgeRuntimeState::is_ns_walker_overlay_high(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_high(v));
        }
        for v in [0xD6u8, 0xDA, 0xDE, 0xE3, 0xE5, 0xE6, 0xE8] {
            assert!(BridgeRuntimeState::is_ew_walker_overlay_high(v));
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_high(v));
        }
        // Out-of-range values match neither.
        for v in [0u8, 0x4A, 0x65, 0xCC, 0xE9, 0xFF] {
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_high(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_high(v));
        }
    }

    #[test]
    fn axis_classifiers_partition_low_range() {
        for v in [0x4Au8, 0x4F, 0x52, 0x5C, 0x5F, 0x64] {
            assert!(BridgeRuntimeState::is_ns_walker_overlay_low(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_low(v));
        }
        for v in [0x53u8, 0x57, 0x5B, 0x60, 0x63, 0x65] {
            assert!(BridgeRuntimeState::is_ew_walker_overlay_low(v));
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_low(v));
        }
        for v in [0u8, 0x49, 0x66, 0xCC, 0xD0, 0xFF] {
            assert!(!BridgeRuntimeState::is_ns_walker_overlay_low(v));
            assert!(!BridgeRuntimeState::is_ew_walker_overlay_low(v));
        }
    }

    #[test]
    fn start_shift_high_ns_north_off_steps_south() {
        // Only (2, 0) is a bridge cell. Hitting (2, 0) → north neighbor is
        // off-bridge, so walker starts south at (2, 1).
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 2, 0, 0xD0);
        assert_eq!(state.find_walker_start_high_ns(2, 0), (2, 1));
    }

    #[test]
    fn start_shift_high_ns_north2_on_steps_north() {
        // (2, 0), (2, 1), (2, 2) all on bridge. Hitting (2, 2): north-1
        // (2, 1) on; north-2 (2, 0) on → walker starts at (2, 1).
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_high_body_cell(&mut state, 2, y, 0xD0);
        }
        assert_eq!(state.find_walker_start_high_ns(2, 2), (2, 1));
    }

    #[test]
    fn start_shift_high_ns_stable_mid_no_shift() {
        // 3 bridge cells (2, 0..3). Hitting middle (2, 1): north-1 (2, 0) on,
        // north-2 (2, -1) off-map → no shift.
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_high_body_cell(&mut state, 2, y, 0xD0);
        }
        assert_eq!(state.find_walker_start_high_ns(2, 1), (2, 1));
    }

    #[test]
    fn start_shift_high_ew_west_off_steps_east() {
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 2, 0xD8);
        assert_eq!(state.find_walker_start_high_ew(0, 2), (1, 2));
    }

    #[test]
    fn ns_walker_intermediate_writes_0xd3_to_triple() {
        // 3 NS body cells at (2, 0..3) all overlay 0xD0 (< 0xD3). Hit (2, 1).
        // Expect (2, 0..3) all → 0xD3, damage_state Damaged (not Destroyed).
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_high_body_cell(&mut state, 2, y, 0xD0);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        assert!(
            matches!(outcome, StateOutcome::Absorbed),
            "no perpendicular pattern → no sibling collapse → Absorbed; got {:?}",
            outcome
        );
        for y in 0..3 {
            let c = state.cell(2, y).unwrap();
            assert_eq!(c.overlay_byte, 0xD3, "y={} should transition to 0xD3", y);
            assert_eq!(c.damage_state, DamageState::Damaged);
        }
    }

    #[test]
    fn ns_walker_final_writes_0xe7_marks_destroyed_zones_dirty() {
        // 3 NS body cells at (2, 0..3) all overlay 0xD4 (final-eligible
        // [0xD3..=0xD5]). Hit (2, 1). Expect 0xE7 + Destroyed + zones_dirty.
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_high_body_cell(&mut state, 2, y, 0xD4);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                zones_dirty,
                ..
            } => {
                assert!(zones_dirty, "final-stage collapse must mark zones_dirty");
                for y in 0..3 {
                    let c = state.cell(2, y).unwrap();
                    assert_eq!(c.overlay_byte, 0xE7);
                    assert_eq!(c.damage_state, DamageState::Destroyed);
                    assert!(
                        destroyed_cells.contains(&(2, y)),
                        "(2, {}) missing from destroyed_cells",
                        y
                    );
                }
            }
            other => panic!("expected Collapsed, got {:?}", other),
        }
    }

    #[test]
    fn ns_walker_0xdf_writes_0xe0_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_high_body_cell(&mut state, 2, y, 0xDF);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0xE0);
        }
    }

    #[test]
    fn ns_walker_0xe1_writes_0xe2_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_high_body_cell(&mut state, 2, y, 0xE1);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0xE2);
        }
    }

    #[test]
    fn ns_walker_returns_nochange_for_out_of_case_overlay() {
        // 0xD7 is in EW sub-range (would be routed to EW walker). NS walker
        // hit on it is a no-op.
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 2, 1, 0xD7);
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_high(2, 1, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
        assert_eq!(state.cell(2, 1).unwrap().overlay_byte, 0xD7);
    }

    #[test]
    fn ew_walker_final_writes_0xe8_marks_destroyed_zones_dirty() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            state.test_seed_cell(
                x,
                2,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 5,
                    bridge_group_id: Some(1),
                    damage_state: DamageState::Damaged,
                    axis: Some(Axis::EW),
                    role: BridgeCellRole::Body,
                    anchor_span_id: Some(1),
                    overlay_byte: 0xDD,
                    damaged_variant: false,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
                },
            );
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ew_high(1, 2, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                zones_dirty,
                ..
            } => {
                assert!(zones_dirty);
                for x in 0..3 {
                    let c = state.cell(x, 2).unwrap();
                    assert_eq!(c.overlay_byte, 0xE8);
                    assert_eq!(c.damage_state, DamageState::Destroyed);
                    assert!(destroyed_cells.contains(&(x, 2)));
                }
            }
            other => panic!("expected Collapsed, got {:?}", other),
        }
    }

    #[test]
    fn ew_walker_0xe3_writes_0xe4_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            state.test_seed_cell(
                x,
                2,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 5,
                    bridge_group_id: Some(1),
                    damage_state: DamageState::Damaged,
                    axis: Some(Axis::EW),
                    role: BridgeCellRole::Body,
                    anchor_span_id: Some(1),
                    overlay_byte: 0xE3,
                    damaged_variant: false,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
                },
            );
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ew_high(1, 2, &terrain);
        for x in 0..3 {
            assert_eq!(state.cell(x, 2).unwrap().overlay_byte, 0xE4);
        }
    }

    #[test]
    fn check_bridge_neighbors_ew_high_bit_layout() {
        // Verify all 4 bit-positions resolve correctly. Place a center cell
        // at (1, 0) so we can independently set west=(0,0) and east=(2,0).
        // bit 0 (east in {0xD1,0xD3,0xD5,0xE0}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 1, 0, 0xD0);
        seed_high_body_cell(&mut state, 2, 0, 0xD1);
        assert_eq!(state.check_bridge_neighbors_ew_high(1, 0), 1);
        // bit 1 (east in {0xD4, 0xE7}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 1, 0, 0xD0);
        seed_high_body_cell(&mut state, 2, 0, 0xD4);
        assert_eq!(state.check_bridge_neighbors_ew_high(1, 0), 2);
        // bit 2 (west in {0xD2, 0xD3, 0xD4, 0xE2}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xD2);
        seed_high_body_cell(&mut state, 1, 0, 0xD0);
        assert_eq!(state.check_bridge_neighbors_ew_high(1, 0), 4);
        // bit 3 (west in {0xD5, 0xE7}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xE7);
        seed_high_body_cell(&mut state, 1, 0, 0xD0);
        assert_eq!(state.check_bridge_neighbors_ew_high(1, 0), 8);
    }

    #[test]
    fn check_bridge_neighbors_ns_high_bit_layout() {
        // bit 0 (north in {0xDA, 0xDC, 0xDE, 0xE4}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xDA);
        seed_high_body_cell(&mut state, 0, 1, 0xD0);
        assert_eq!(state.check_bridge_neighbors_ns_high(0, 1), 1);
        // bit 1 (north in {0xDD, 0xE8}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xE8);
        seed_high_body_cell(&mut state, 0, 1, 0xD0);
        assert_eq!(state.check_bridge_neighbors_ns_high(0, 1), 2);
        // bit 2 (south in {0xDB, 0xDC, 0xDD, 0xE6}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xD0);
        seed_high_body_cell(&mut state, 0, 1, 0xE6);
        assert_eq!(state.check_bridge_neighbors_ns_high(0, 0), 4);
        // bit 3 (south in {0xDE, 0xE8}):
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 0, 0, 0xD0);
        seed_high_body_cell(&mut state, 0, 1, 0xDE);
        assert_eq!(state.check_bridge_neighbors_ns_high(0, 0), 8);
    }

    fn seed_low_body_cell(
        state: &mut BridgeRuntimeState,
        rx: u16,
        ry: u16,
        axis: Axis,
        overlay: u8,
    ) {
        state.test_seed_cell(
            rx,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 2,
                bridge_group_id: Some(2),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(axis),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(2),
                overlay_byte: overlay,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
    }

    #[test]
    fn ns_low_walker_intermediate_writes_0x50_to_triple() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x4A);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        assert!(matches!(outcome, StateOutcome::Absorbed));
        for y in 0..3 {
            let c = state.cell(2, y).unwrap();
            assert_eq!(c.overlay_byte, 0x50);
            assert_eq!(c.damage_state, DamageState::Damaged);
        }
    }

    #[test]
    fn ns_low_walker_final_writes_0x64_marks_destroyed_zones_dirty() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x51);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                zones_dirty,
                ..
            } => {
                assert!(zones_dirty);
                for y in 0..3 {
                    let c = state.cell(2, y).unwrap();
                    assert_eq!(c.overlay_byte, 0x64);
                    assert_eq!(c.damage_state, DamageState::Destroyed);
                    assert!(destroyed_cells.contains(&(2, y)));
                }
            }
            other => panic!("expected Collapsed, got {:?}", other),
        }
    }

    #[test]
    fn ns_low_walker_0x5c_writes_0x5d_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x5C);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0x5D);
        }
    }

    #[test]
    fn ns_low_walker_0x5e_writes_0x5f_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x5E);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0x5F);
        }
    }

    #[test]
    fn ns_low_walker_returns_nochange_above_0x52_below_0x5c() {
        // 0x55 is in the LOW EW sub-range (would route to EW walker). NS
        // walker hit on it must be a no-op.
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 2, 1, Axis::NS, 0x55);
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ns_low(2, 1, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
        assert_eq!(state.cell(2, 1).unwrap().overlay_byte, 0x55);
    }

    #[test]
    fn ew_low_walker_final_writes_0x65_marks_destroyed_zones_dirty() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut state, x, 2, Axis::EW, 0x5A);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_walker_ew_low(1, 2, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                zones_dirty,
                ..
            } => {
                assert!(zones_dirty);
                for x in 0..3 {
                    let c = state.cell(x, 2).unwrap();
                    assert_eq!(c.overlay_byte, 0x65);
                    assert_eq!(c.damage_state, DamageState::Destroyed);
                    assert!(destroyed_cells.contains(&(x, 2)));
                }
            }
            other => panic!("expected Collapsed, got {:?}", other),
        }
    }

    #[test]
    fn ew_low_walker_0x60_writes_0x61_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut state, x, 2, Axis::EW, 0x60);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ew_low(1, 2, &terrain);
        for x in 0..3 {
            assert_eq!(state.cell(x, 2).unwrap().overlay_byte, 0x61);
        }
    }

    #[test]
    fn ew_low_walker_0x62_writes_0x63_intermediate() {
        let mut state = BridgeRuntimeState::default();
        for x in 0..3u16 {
            seed_low_body_cell(&mut state, x, 2, Axis::EW, 0x62);
        }
        let terrain = empty_terrain();
        let _ = state.destroy_bridge_walker_ew_low(1, 2, &terrain);
        for x in 0..3 {
            assert_eq!(state.cell(x, 2).unwrap().overlay_byte, 0x63);
        }
    }

    #[test]
    fn check_bridge_neighbors_ew_low_bit_layout() {
        // bit 0 (east in {0x4E, 0x50, 0x52, 0x5D}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        seed_low_body_cell(&mut state, 2, 0, Axis::NS, 0x4E);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 1);
        // bit 1 (east in {0x51, 0x64}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        seed_low_body_cell(&mut state, 2, 0, Axis::NS, 0x64);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 2);
        // bit 2 (west in {0x4F, 0x50, 0x51, 0x5F}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::NS, 0x4F);
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 4);
        // bit 3 (west in {0x52, 0x64}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::NS, 0x52);
        seed_low_body_cell(&mut state, 1, 0, Axis::NS, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ew_low(1, 0), 8);
    }

    #[test]
    fn check_bridge_neighbors_ns_low_bit_layout() {
        // bit 0 (north in {0x57, 0x59, 0x5B, 0x61}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x57);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 1), 1);
        // bit 1 (north in {0x5A, 0x65}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x65);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x4A);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 1), 2);
        // bit 2 (south in {0x58, 0x59, 0x5A, 0x63}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x4A);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x63);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 0), 4);
        // bit 3 (south in {0x5B, 0x65}):
        let mut state = BridgeRuntimeState::default();
        seed_low_body_cell(&mut state, 0, 0, Axis::EW, 0x4A);
        seed_low_body_cell(&mut state, 0, 1, Axis::EW, 0x5B);
        assert_eq!(state.check_bridge_neighbors_ns_low(0, 0), 8);
    }

    #[test]
    fn destroy_bridge_low_classifies_ns_axis_into_walker() {
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_low_body_cell(&mut state, 2, y, Axis::NS, 0x4A);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_low(2, 1, &terrain);
        assert!(matches!(outcome, StateOutcome::Absorbed));
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0x50);
        }
    }

    #[test]
    fn destroy_bridge_high_classifies_ns_axis_into_walker() {
        // 0xD0 routes to NS walker. 0xD0 < 0xD3 → intermediate write 0xD3
        // to triple. Without perpendicular pattern → Absorbed.
        let mut state = BridgeRuntimeState::default();
        for y in 0..3u16 {
            seed_high_body_cell(&mut state, 2, y, 0xD0);
        }
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_high(2, 1, &terrain);
        assert!(
            matches!(outcome, StateOutcome::Absorbed),
            "Task 7 wires real NS walker; expected Absorbed got {:?}",
            outcome,
        );
        for y in 0..3 {
            assert_eq!(state.cell(2, y).unwrap().overlay_byte, 0xD3);
        }
    }
}
