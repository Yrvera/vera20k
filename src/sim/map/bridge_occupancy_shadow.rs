//! SHADOW two-layer per-cell object-list occupancy (GATE A2).
//!
//! Models the gamemd-native bridge occupancy LIST representation verified in
//! `GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md`: each cell holds
//! TWO independent object lists — a ground list (`CellClass+0xE4` FirstObject) and
//! a bridge/deck list (`CellClass+0xE8` AltObject). An occupant lives on exactly
//! ONE of the two, selected by the occupant's persistent `OnBridge` byte
//! (`Object+0x8C`), sampled at the add/remove call site. AddContent prepends
//! non-structures to the selected head and appends structures (`WhatAmI()==6`) to
//! the selected tail; RemoveContent walks only the selected list. The cell also
//! tracks a per-list occupant COUNT.
//!
//! ## SHADOW ONLY — NOT authoritative.
//! The authoritative occupancy storage is `sim::occupancy::OccupancyGrid` (a single
//! layer-tagged list). This module is a parallel, gamemd-faithful representation
//! used to validate the two-layer/order/transition contract via tests; it is NOT
//! wired into the tick, NOT serialized, and NOT part of the state hash. The cutover
//! that makes this the authoritative store (and bumps `SNAPSHOT_VERSION`) is a
//! separate reviewed step — see the deferred-cutover note in the bridge plan (P5).
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on sim/map (`ListLayer`) and std.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::sim::map::bridge_topology::ListLayer;

/// One occupant of a cell list. `is_structure` selects prepend-vs-append on add
/// (gamemd `WhatAmI()==6` buildings append; everything else prepends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowOccupant {
    pub entity_id: u64,
    pub is_structure: bool,
}

/// A single cell's two object lists, each in gamemd insertion order (head first).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShadowCellLists {
    /// Ground list (`FirstObject`, +0xE4).
    ground: Vec<u64>,
    /// Bridge/deck list (`AltObject`, +0xE8).
    bridge: Vec<u64>,
}

impl ShadowCellLists {
    /// Read one layer's list in head-first order.
    pub fn layer(&self, layer: ListLayer) -> &[u64] {
        match layer {
            ListLayer::Ground => &self.ground,
            ListLayer::Bridge => &self.bridge,
        }
    }

    /// Occupant count on one layer (mirrors the per-list count field).
    pub fn count(&self, layer: ListLayer) -> usize {
        self.layer(layer).len()
    }

    fn list_mut(&mut self, layer: ListLayer) -> &mut Vec<u64> {
        match layer {
            ListLayer::Ground => &mut self.ground,
            ListLayer::Bridge => &mut self.bridge,
        }
    }

    fn is_empty(&self) -> bool {
        self.ground.is_empty() && self.bridge.is_empty()
    }
}

/// SHADOW per-cell two-layer object-list grid. Deterministic `BTreeMap` keyed by
/// cell coord for replay-stable iteration (mirrors the authoritative grid's
/// determinism guarantee).
#[derive(Debug, Clone, Default)]
pub struct ShadowBridgeOccupancy {
    cells: BTreeMap<(u16, u16), ShadowCellLists>,
}

impl ShadowBridgeOccupancy {
    pub fn new() -> Self {
        Self::default()
    }

    /// AddContent: insert an occupant into the layer selected by its `on_bridge`
    /// byte. Non-structures prepend to the head; structures append to the tail.
    pub fn add(&mut self, rx: u16, ry: u16, occupant: ShadowOccupant, on_bridge: bool) {
        let layer = layer_for(on_bridge);
        let list = self.cells.entry((rx, ry)).or_default().list_mut(layer);
        if occupant.is_structure {
            list.push(occupant.entity_id);
        } else {
            list.insert(0, occupant.entity_id);
        }
    }

    /// RemoveContent: remove an occupant from the layer selected by its `on_bridge`
    /// byte (walks ONLY the selected list, matching the binary). No-op if absent.
    pub fn remove(&mut self, rx: u16, ry: u16, entity_id: u64, on_bridge: bool) {
        let layer = layer_for(on_bridge);
        if let Some(cell) = self.cells.get_mut(&(rx, ry)) {
            cell.list_mut(layer).retain(|&id| id != entity_id);
            if cell.is_empty() {
                self.cells.remove(&(rx, ry));
            }
        }
    }

    /// Cell crossing with the verified GATE A2 ordering:
    /// 1. remove from the OLD cell using the OLD `on_bridge` layer,
    /// 2. (caller has already evaluated the transition and supplies the NEW byte),
    /// 3. add to the NEW cell using the NEW `on_bridge` layer.
    ///
    /// Old-cell removal observes the pre-transition layer; new-cell insertion
    /// observes the post-transition layer. This is the load-bearing asymmetry: the
    /// two halves may target different layers when the occupant stepped on/off the
    /// deck during the crossing.
    pub fn cross(
        &mut self,
        old_rx: u16,
        old_ry: u16,
        old_on_bridge: bool,
        new_rx: u16,
        new_ry: u16,
        new_on_bridge: bool,
        occupant: ShadowOccupant,
    ) {
        self.remove(old_rx, old_ry, occupant.entity_id, old_on_bridge);
        self.add(new_rx, new_ry, occupant, new_on_bridge);
    }

    /// Read a cell's two lists, if any occupant is present.
    pub fn get(&self, rx: u16, ry: u16) -> Option<&ShadowCellLists> {
        self.cells.get(&(rx, ry))
    }

    /// Occupant count on one layer of a cell.
    pub fn count_on(&self, rx: u16, ry: u16, layer: ListLayer) -> usize {
        self.cells.get(&(rx, ry)).map_or(0, |c| c.count(layer))
    }
}

/// The object-LIST layer for an occupant is selected by its persistent `on_bridge`
/// byte alone (NOT the locomotor/path layer, NOT the Z-height bit-layer selector).
#[inline]
fn layer_for(on_bridge: bool) -> ListLayer {
    if on_bridge {
        ListLayer::Bridge
    } else {
        ListLayer::Ground
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: u64) -> ShadowOccupant {
        ShadowOccupant {
            entity_id: id,
            is_structure: false,
        }
    }
    fn structure(id: u64) -> ShadowOccupant {
        ShadowOccupant {
            entity_id: id,
            is_structure: true,
        }
    }

    #[test]
    fn list_layer_selected_by_on_bridge_byte() {
        // GATE A2 (a): the same cell holds two independent lists; the occupant's
        // on_bridge byte alone decides which one it joins.
        let mut occ = ShadowBridgeOccupancy::new();
        occ.add(5, 5, unit(1), /* on_bridge */ false);
        occ.add(5, 5, unit(2), /* on_bridge */ true);

        assert_eq!(occ.get(5, 5).unwrap().layer(ListLayer::Ground), &[1]);
        assert_eq!(occ.get(5, 5).unwrap().layer(ListLayer::Bridge), &[2]);
        assert_eq!(occ.count_on(5, 5, ListLayer::Ground), 1);
        assert_eq!(occ.count_on(5, 5, ListLayer::Bridge), 1);
    }

    #[test]
    fn add_order_nonstructures_prepend_structures_append_within_layer() {
        // GATE A2 (b): AddContent prepends non-structures to the head and appends
        // WhatAmI()==6 buildings to the tail of the SELECTED layer.
        let mut occ = ShadowBridgeOccupancy::new();
        occ.add(3, 3, unit(1), false);
        occ.add(3, 3, structure(100), false);
        occ.add(3, 3, unit(2), false);
        // unit 2 prepended ahead of unit 1; structure 100 stays at the tail.
        assert_eq!(occ.get(3, 3).unwrap().layer(ListLayer::Ground), &[2, 1, 100]);
        // Bridge layer untouched.
        assert_eq!(occ.count_on(3, 3, ListLayer::Bridge), 0);
    }

    #[test]
    fn remove_walks_only_the_selected_layer() {
        // GATE A2 (a): RemoveContent walks ONLY the list selected by on_bridge.
        // Removing id=1 with the WRONG layer byte must NOT find it.
        let mut occ = ShadowBridgeOccupancy::new();
        occ.add(7, 7, unit(1), /* ground */ false);
        occ.add(7, 7, unit(1), /* bridge */ true); // same id on both lists

        // Remove from the bridge layer only.
        occ.remove(7, 7, 1, /* on_bridge */ true);
        assert_eq!(occ.get(7, 7).unwrap().layer(ListLayer::Ground), &[1]);
        assert_eq!(occ.count_on(7, 7, ListLayer::Bridge), 0);
    }

    #[test]
    fn cross_removes_old_layer_then_adds_new_layer() {
        // GATE A2 (c): step-onto-deck crossing — remove from the OLD (ground)
        // layer of the old cell, add to the NEW (bridge) layer of the new cell.
        let mut occ = ShadowBridgeOccupancy::new();
        occ.add(1, 1, unit(9), /* on_bridge */ false);

        occ.cross(
            1, 1, /* old_on_bridge */ false, // ground at the old cell
            2, 2, /* new_on_bridge */ true,  // deck at the new cell
            unit(9),
        );

        // Old cell empty; new cell has the occupant on the BRIDGE list.
        assert!(occ.get(1, 1).is_none());
        assert_eq!(occ.get(2, 2).unwrap().layer(ListLayer::Bridge), &[9]);
        assert_eq!(occ.count_on(2, 2, ListLayer::Ground), 0);
    }

    #[test]
    fn cross_step_off_deck_uses_old_bridge_layer_for_removal() {
        // GATE A2 (c): step-off crossing — the OLD-cell removal must use the old
        // (bridge) byte; if it used the new (ground) byte it would miss the
        // occupant and leak a stale bridge-list entry.
        let mut occ = ShadowBridgeOccupancy::new();
        occ.add(4, 4, unit(7), /* on_bridge */ true); // on the deck

        occ.cross(
            4, 4, /* old_on_bridge */ true, // deck at the old cell
            5, 5, /* new_on_bridge */ false, // ground at the new cell
            unit(7),
        );

        // No leaked bridge-list entry at the old cell.
        assert!(occ.get(4, 4).is_none());
        assert_eq!(occ.get(5, 5).unwrap().layer(ListLayer::Ground), &[7]);
    }

    #[test]
    fn remove_cleans_up_fully_empty_cell() {
        let mut occ = ShadowBridgeOccupancy::new();
        occ.add(6, 6, unit(1), false);
        occ.remove(6, 6, 1, false);
        assert!(occ.get(6, 6).is_none());
    }
}
