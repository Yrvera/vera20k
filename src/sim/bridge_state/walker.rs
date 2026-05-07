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
        (0xCD..=0xD5).contains(&overlay)
            || (0xDF..=0xE2).contains(&overlay)
            || overlay == 0xE7
    }

    pub(super) fn is_ew_walker_overlay_high(overlay: u8) -> bool {
        // HIGH EW axis sub-range:
        //   [0xD6..=0xDE] ∪ [0xE3..=0xE6] ∪ {0xE8}
        (0xD6..=0xDE).contains(&overlay)
            || (0xE3..=0xE6).contains(&overlay)
            || overlay == 0xE8
    }

    pub(super) fn is_ns_walker_overlay_low(overlay: u8) -> bool {
        // LOW NS axis sub-range:
        //   [0x4A..=0x52] ∪ [0x5C..=0x5F] ∪ {0x64}
        (0x4A..=0x52).contains(&overlay)
            || (0x5C..=0x5F).contains(&overlay)
            || overlay == 0x64
    }

    pub(super) fn is_ew_walker_overlay_low(overlay: u8) -> bool {
        // LOW EW axis sub-range:
        //   [0x53..=0x5B] ∪ [0x60..=0x63] ∪ {0x65}
        (0x53..=0x5B).contains(&overlay)
            || (0x60..=0x63).contains(&overlay)
            || overlay == 0x65
    }

    // ----- Walker bodies — stubbed in Task 6; real implementations land
    // in Task 7 (HIGH) and Task 8 (LOW). -----

    pub(super) fn destroy_bridge_walker_ns_high(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }

    pub(super) fn destroy_bridge_walker_ew_high(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }

    pub(super) fn destroy_bridge_walker_ns_low(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }

    pub(super) fn destroy_bridge_walker_ew_low(
        &mut self,
        _rx: u16,
        _ry: u16,
        _terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome {
        StateOutcome::NoChange
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeRuntimeCell, DamageState,
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
    fn destroy_bridge_high_classifies_ns_axis_into_walker() {
        // Overlay 0xD0 ∈ NS sub-range. Walker is stubbed → NoChange. Test
        // verifies the entry function classified axis without panic.
        let mut state = BridgeRuntimeState::default();
        seed_high_body_cell(&mut state, 2, 0, 0xD0);
        let terrain = empty_terrain();
        let outcome = state.destroy_bridge_high(2, 0, &terrain);
        assert!(
            matches!(outcome, StateOutcome::NoChange),
            "Task 6 ships with NS/EW walker bodies stubbed",
        );
    }
}
