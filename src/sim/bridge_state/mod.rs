//! Mutable bridge runtime state layered on top of resolved terrain.
//!
//! Bridges are modeled as terrain, not spawned entities. This module owns the
//! destroyable runtime state used by combat, layered pathing, and bridge-deck
//! fallout handling.
//!
//! TODO(RE): This runtime currently models elevated bridge-deck presence/destruction only.
//! The recovered low-bridge overlay damage progression now lives in
//! `sim::bridge_specs`, but wiring it here still needs mutable overlay state,
//! connected-section selection, and `AtomDamage`/BridgeStrength gate inputs.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use std::collections::{BTreeMap, VecDeque};

/// Bridge body axis. Body cells are stacked along this axis; ramps face
/// perpendicular.
///
/// Mapping: `Axis::EW` ↔ `BridgeDirection::EastWest` ↔ state byte 9–17;
/// `Axis::NS` ↔ `BridgeDirection::NorthSouth` ↔ state byte 0–8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Axis {
    /// Body cells stacked north–south (along Y); ramps face east/west.
    /// State byte range 0–8.
    NS,
    /// Body cells stacked east–west (along X); ramps face north/south.
    /// State byte range 9–17.
    EW,
}

/// Per-cell damage state encoding all 18 state-byte values.
///
/// Body cells transition Healthy → Damaged → Destroyed under repeated
/// damage (per axis). Partial-collapse states are reached only via
/// bridgehead final-step cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DamageState {
    /// Healthy body — `variant` carries the 6-frame jitter (0..=5 per axis,
    /// map-load-deterministic, never advances during gameplay).
    /// Maps to state byte 0–5 (NS) or 9–14 (EW).
    Healthy { variant: u8 },
    /// Damaged body — next hit collapses. State byte 6 (NS) / 15 (EW).
    Damaged,
    /// Partial collapse: ramp B already collapsed; this cell will fire
    /// CollapseA. State byte 7 (NS) / 17 (EW).
    PartialCollapseA,
    /// Partial collapse: ramp A already collapsed; this cell will fire
    /// CollapseB. State byte 8 (NS) / 16 (EW).
    PartialCollapseB,
    /// Fully destroyed.
    Destroyed,
}

impl DamageState {
    /// Encode to binary state byte (`CellClass+0x11E`).
    ///
    /// Per HIGH §3.1 / `apply_ramp_transition` docstring:
    /// - NS axis: Healthy{variant: 0..=5} → 0..=5; Damaged → 6;
    ///   PartialCollapseA → 7; PartialCollapseB → 8; Destroyed → 0.
    /// - EW axis: Healthy{variant: 0..=5} → 9..=14; Damaged → 0xF;
    ///   PartialCollapseA → 0x11; PartialCollapseB → 0x10; Destroyed → 0.
    ///
    /// **Note:** `Destroyed` always maps to byte 0, which is also the encoding
    /// for `Healthy{variant: 0}` initial state. Callers must use context
    /// (phase + prior state) to disambiguate after a `from_state_byte(0)` decode.
    /// `to_state_byte` is unambiguous (every variant has exactly one encoding).
    pub fn to_state_byte(self, axis: Axis) -> u8 {
        let ns_base: u8 = 0;
        let ew_base: u8 = 9;
        let base = match axis { Axis::NS => ns_base, Axis::EW => ew_base };
        match self {
            DamageState::Healthy { variant } => base + variant.min(5),
            DamageState::Damaged => match axis { Axis::NS => 6, Axis::EW => 0xF },
            DamageState::PartialCollapseA => match axis { Axis::NS => 7, Axis::EW => 0x11 },
            DamageState::PartialCollapseB => match axis { Axis::NS => 8, Axis::EW => 0x10 },
            DamageState::Destroyed => 0,
        }
    }

    /// Decode from binary state byte. Returns `None` for bytes outside the
    /// defined ranges (NS: 0..=8; EW: 9..=0x11).
    ///
    /// **State 0 ambiguity:** byte 0 always decodes to `Healthy{variant: 0}`.
    /// Post-collapse `Destroyed` cells also have byte 0 in the binary, but the
    /// caller (body driver) writes `Destroyed` directly without round-tripping
    /// through `from_state_byte`. Test fixtures and snapshot consistency checks
    /// should not rely on this method to recover `Destroyed`.
    pub fn from_state_byte(byte: u8) -> Option<Self> {
        match byte {
            0..=5 => Some(DamageState::Healthy { variant: byte }),
            6 => Some(DamageState::Damaged),
            7 => Some(DamageState::PartialCollapseA),
            8 => Some(DamageState::PartialCollapseB),
            9..=14 => Some(DamageState::Healthy { variant: byte - 9 }),
            0xF => Some(DamageState::Damaged),
            0x10 => Some(DamageState::PartialCollapseB),
            0x11 => Some(DamageState::PartialCollapseA),
            _ => None,
        }
    }
}

/// Cell role within an `AnchorSpan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BridgeCellRole {
    /// Anchor cell: primary cell of an anchor span; carries the canonical state byte.
    Anchor,
    /// Body cell: non-anchor structural cell; follows `anchor_span_id` for state-machine processing.
    Body,
    /// Bridgehead cell: ramp connection-piece off the body.
    Bridgehead,
    /// Tail cell (cell 5 of anchor pattern, walked in `–direction` from anchor).
    Tail,
}

/// Compass-direction enum.
///
/// Discriminant values must match the binary's table indices because
/// `set_bridge_direction` uses them to index into the offsets table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Direction {
    N = 0,
    NE = 1,
    E = 2,
    SE = 3,
    S = 4,
    SW = 5,
    W = 6,
    NW = 7,
}

impl Direction {
    /// Cell-coord offset `(dx, dy)`. Signed because directions can decrement.
    pub const fn offset(self) -> (i32, i32) {
        match self {
            Direction::N => (0, -1),
            Direction::NE => (1, -1),
            Direction::E => (1, 0),
            Direction::SE => (1, 1),
            Direction::S => (0, 1),
            Direction::SW => (-1, 1),
            Direction::W => (-1, 0),
            Direction::NW => (-1, -1),
        }
    }

    /// `(self - 4) & 7` — opposite direction. Used by `set_bridge_direction`
    /// to compute cell 5 (walked in –direction from anchor).
    pub const fn opposite(self) -> Direction {
        match self {
            Direction::N => Direction::S,
            Direction::NE => Direction::SW,
            Direction::E => Direction::W,
            Direction::SE => Direction::NW,
            Direction::S => Direction::N,
            Direction::SW => Direction::NE,
            Direction::W => Direction::E,
            Direction::NW => Direction::SE,
        }
    }
}

/// `apply_ramp_transition` phase. Maps to one of the 16 ramp transition helpers
/// (NS/EW × DamageA/DamageB/CollapseA/CollapseB).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    DamageA,
    DamageB,
    CollapseA,
    CollapseB,
}

/// First-class anchor-span representation. One span per anchor cell.
///
/// Walker pattern: up to 6 cells (anchor + 3 walked +dir + 1 walked –dir +
/// optional fixed-offset cell when direction == W). Per-cell action
/// (BlowUpBridge vs flag-only) is determined by slot index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AnchorSpan {
    /// Stable ID, matches `BridgeRuntimeCell.anchor_span_id`.
    pub id: u16,
    /// The anchor cell. Slot 0.
    pub anchor: (u16, u16),
    /// All cells in walker order:
    /// `[0]=anchor, [1..=3]=+direction × 1/2/3, [4]=-direction × 1, [5]=fixed-offset (only when direction == W)`.
    /// `None` for unused slots when the optional fixed-offset cell isn't present.
    pub cells: [Option<(u16, u16)>; 6],
    /// Body axis (NS or EW). Determined from `bridge_layer.direction`.
    pub axis: Axis,
    /// Walk direction (compass index 0–7). Used to compute walked cells.
    pub direction: Direction,
    /// Mirror of anchor cell's damage state. Convenience for queries.
    pub damage_state: DamageState,
    /// Group ID (existing `BridgeRuntimeState::group_cells`) — preserved for
    /// connectivity queries.
    pub bridge_group_id: u16,
}

impl AnchorSpan {
    /// Cells receiving `BlowUpBridge` on destruction path: slots 0, 1, 2, 4.
    /// Slot 3 (cell 4) and slot 5 (cell 6) are flag-only.
    pub const BLOW_UP_SLOTS: [usize; 4] = [0, 1, 2, 4];

    /// Iterate `(slot, cell)` for present cells (skips `None`).
    pub fn iter_cells(&self) -> impl Iterator<Item = (usize, (u16, u16))> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.map(|cell| (i, cell)))
    }

    /// Cells that get `BlowUpBridge` on destruction. Skips slots 3 and 5
    /// which are flag-only.
    pub fn blow_up_cells(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        Self::BLOW_UP_SLOTS
            .iter()
            .filter_map(|&slot| self.cells[slot])
    }
}

/// Per-cell bridge damage event emitted by combat. World drains via the
/// `bridge_orchestrator` 4-path dispatcher. The Apply_area_damage gate +
/// retry happen in the world orchestrator (not in combat) so the RNG draw
/// order matches the binary's dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BridgeDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: u16,
    /// Interned warhead ID — used for IonCannon identity check (combat
    /// boundary pre-resolves `is_ion_cannon`) and for InfDeath selection in
    /// the C4Warhead ground-kill cascade.
    pub warhead_ref: crate::sim::intern::InternedId,
    /// Pre-resolved at combat: `warhead_ref == rules.ion_cannon_warhead_id()`.
    /// Bypasses the BridgeStrength RNG gate; enables the 3-retry loop on
    /// state-machine paths only (direct-overlay paths are single-shot).
    pub is_ion_cannon: bool,
    /// Explosion z in tile-step level units (signed). Used by the
    /// state-machine Z-height gate: state-machine paths fire only when
    /// `impact_z ∈ [cell.level - 1, cell.level + 1]`. Direct-overlay paths
    /// skip this gate.
    pub impact_z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BridgeStateChange {
    pub destroyed_cells: Vec<(u16, u16)>,
}

/// Outcome of one `body_cell_advance_state` invocation. Mirrors the return
/// codes of binary `ProcessBridgeDamageStateMachine_High @ 0x576BA0` body
/// branch (0 = absorbed, 1 = collapse), with structured fallout for the
/// orchestrator to dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateOutcome {
    /// Damage absorbed — anchor advanced from `Healthy` to `Damaged`. Bridge
    /// still passable. Renderer should redraw.
    Absorbed,
    /// Anchor collapsed — `damage_state` became `Destroyed`. Cascade actions
    /// for orchestrator follow.
    Collapsed {
        /// Cells whose `damage_state` was set to `Destroyed` in this call
        /// (typically just the anchor; perpendicular targets that hit
        /// collapse-final via `update_ramp_perpendicular` also appear here).
        destroyed_cells: Vec<(u16, u16)>,
        /// `BlowUpBridge` cascade actions emitted by `set_bridge_direction`.
        /// Orchestrator dispatches these (kill ground occupants, Limbo
        /// bridge-deck, spawn debris).
        set_bridge_direction:
            crate::sim::bridge_specs::SetBridgeDirectionResult,
        /// Cells where `UpdateAdjacentBridges_High` should run for rim
        /// re-evaluation. Orchestrator (Phase F Task 27) runs the actual
        /// rim helper.
        adjacent_bridges_dirty: Vec<(u16, u16)>,
        /// Whether the zone graph needs rebuild (`InvalidateBridgeZones` →
        /// `UpdateBridgeZonesHelper`). Orchestrator dispatches.
        zones_dirty: bool,
    },
    /// Cell is not a body-bridge cell, anchor span lookup failed, or anchor
    /// is already `Destroyed`. No-op.
    NoChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BridgeRuntimeCell {
    pub deck_present: bool,
    pub destroyable: bool,
    pub deck_level: u8,
    pub bridge_group_id: Option<u16>,

    /// Per-cell damage state. Drives state-machine progression and renderer
    /// display-tile selection. Replaces the old `destroyed: bool`.
    pub damage_state: DamageState,

    /// Bridge body axis (NS or EW). `None` for cells where axis is not
    /// meaningful (orphan body cells, edge cases). Filled by Task 7 anchor walker.
    pub axis: Option<Axis>,

    /// Cell role within its anchor span. Drives state-machine branch dispatch.
    /// Filled by Task 7 anchor walker.
    pub role: BridgeCellRole,

    /// Stable ID of containing `AnchorSpan` (for body cells); `None` for
    /// bridgehead cells.
    pub anchor_span_id: Option<u16>,

    /// Per-cell visible overlay byte (mirrors binary `CellClass+0x44`).
    /// Populated at map-load from `ResolvedTerrainCell.bridge_layer.overlay_id`;
    /// mutated at runtime by the body-cell state machine and (future) perpendicular
    /// overlay-write branch. Renderer queries this to pick the visible tile.
    pub overlay_byte: u8,
}

/// A bridge's ground-level endpoint pair for zone connectivity.
/// Each record connects two ground cells on opposite sides of a bridge.
/// Mirrors gamemd.exe BridgeRecord at MapClass+0x54 (16 bytes each).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BridgeEndpointRecord {
    /// Ground cell on one side of the bridge.
    pub endpoint_a: (u16, u16),
    /// Ground cell on the other side of the bridge.
    pub endpoint_b: (u16, u16),
    /// Which bridge group this record belongs to.
    pub group_id: u16,
    /// Whether the bridge is traversable (false = destroyed).
    pub active: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BridgeRuntimeState {
    width: u16,
    height: u16,
    cells: Vec<Option<BridgeRuntimeCell>>,
    group_cells: BTreeMap<u16, Vec<(u16, u16)>>,
    group_hitpoints: BTreeMap<u16, u16>,
    strength_per_group: u16,
    /// Strength constant from `[CombatDamage] BridgeStrength=` (default 1500).
    /// Used by `apply_area_damage` BridgeStrength RNG gate (Phase F).
    bridge_strength: u16,
    endpoint_records: Vec<BridgeEndpointRecord>,
    /// First-class anchor spans (one per anchor cell). Replaces emergent
    /// flag-bit detection.
    anchor_spans: BTreeMap<u16, AnchorSpan>,
    /// Default per-map override + rules `destroyable_by_default`. Read by
    /// `apply_area_damage` outer gate (Phase F).
    bridge_destroyable_flag: bool,
}

impl BridgeRuntimeState {
    pub fn from_resolved_terrain(
        terrain: &ResolvedTerrainGrid,
        destroyable: bool,
        strength_per_group: u16,
    ) -> Self {
        let width = terrain.width();
        let height = terrain.height();
        let mut cells = vec![None; width as usize * height as usize];
        let mut group_cells: BTreeMap<u16, Vec<(u16, u16)>> = BTreeMap::new();
        let mut anchor_spans: BTreeMap<u16, AnchorSpan> = BTreeMap::new();
        let mut visited = vec![false; cells.len()];
        let mut next_group_id: u16 = 1;
        let mut next_span_id: u16 = 1;

        // Pass 1: BFS-group bridge cells by deck presence (existing
        // group_cells used for endpoint_records + zone connectivity).
        for cell in terrain.iter() {
            let Some(index) = index_of(width, height, cell.rx, cell.ry) else {
                continue;
            };
            if visited[index] || !cell.has_bridge_deck {
                continue;
            }
            let group_id = next_group_id;
            next_group_id = next_group_id.saturating_add(1);
            let mut queue = VecDeque::from([(cell.rx, cell.ry)]);
            let mut members = Vec::new();
            while let Some((rx, ry)) = queue.pop_front() {
                let Some(idx) = index_of(width, height, rx, ry) else {
                    continue;
                };
                if visited[idx] {
                    continue;
                }
                let Some(resolved) = terrain.cell(rx, ry) else {
                    continue;
                };
                if !resolved.has_bridge_deck {
                    continue;
                }
                visited[idx] = true;
                members.push((rx, ry));
                cells[idx] = Some(BridgeRuntimeCell {
                    deck_present: true,
                    destroyable,
                    deck_level: resolved.bridge_deck_level,
                    bridge_group_id: Some(group_id),
                    damage_state: DamageState::Healthy { variant: 0 },
                    axis: bridge_layer_to_axis(resolved.bridge_layer.as_ref()),
                    role: BridgeCellRole::Body, // overwritten in pass 2
                    anchor_span_id: None,
                    overlay_byte: resolved
                        .bridge_layer
                        .as_ref()
                        .map(|bl| bl.overlay_id)
                        .unwrap_or(0),
                });
                for (nx, ny) in cardinal_neighbors(rx, ry, width, height) {
                    if let Some(neighbor) = terrain.cell(nx, ny) {
                        if neighbor.has_bridge_deck {
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
            if !members.is_empty() {
                group_cells.insert(group_id, members);
            }
        }

        // Pass 2: walk anchor patterns. For each cell whose
        // bridge_layer.overlay_id matches an anchor-overlay class, emit one
        // AnchorSpan and tag member cells with role + anchor_span_id.
        for (&group_id, members) in &group_cells {
            for &(rx, ry) in members {
                let Some(resolved) = terrain.cell(rx, ry) else {
                    continue;
                };
                let Some(bl) = resolved.bridge_layer.as_ref() else {
                    continue;
                };
                if !is_anchor_overlay(bl.overlay_id) {
                    continue;
                }
                let axis = bridge_direction_to_axis(bl.direction);
                let direction = anchor_walk_direction(axis);
                let span_id = next_span_id;
                next_span_id = next_span_id.saturating_add(1);
                let span = walk_anchor_pattern(
                    span_id, (rx, ry), axis, direction, group_id, width, height,
                );
                // Tag each cell in span.
                for (slot, cell_pos) in span.iter_cells() {
                    if let Some(idx) = index_of(width, height, cell_pos.0, cell_pos.1) {
                        if let Some(c) = cells[idx].as_mut() {
                            c.role = if slot == 0 {
                                BridgeCellRole::Anchor
                            } else if slot == 4 {
                                BridgeCellRole::Tail
                            } else {
                                BridgeCellRole::Body
                            };
                            c.anchor_span_id = Some(span_id);
                            c.axis = Some(axis);
                        }
                    }
                }
                anchor_spans.insert(span_id, span);
            }
        }

        // Pass 3: classify bridgehead cells (have bridge_layer but not
        // anchor-overlay; not part of an AnchorSpan).
        for cell in terrain.iter() {
            let Some(idx) = index_of(width, height, cell.rx, cell.ry) else {
                continue;
            };
            let Some(resolved) = terrain.cell(cell.rx, cell.ry) else {
                continue;
            };
            let Some(bl) = resolved.bridge_layer.as_ref() else {
                continue;
            };
            if is_anchor_overlay(bl.overlay_id) {
                continue;
            }
            // Bridgehead cells: ramp/connection cells. May not have deck_present
            // if treated purely as ground transition. Mark role only when
            // a BridgeRuntimeCell already exists.
            if let Some(c) = cells[idx].as_mut() {
                c.role = BridgeCellRole::Bridgehead;
                c.anchor_span_id = None;
                c.axis = Some(bridge_direction_to_axis(bl.direction));
            }
        }

        let mut group_hitpoints = BTreeMap::new();
        let strength = strength_per_group.max(1);
        for group_id in group_cells.keys().copied() {
            group_hitpoints.insert(group_id, strength);
        }
        let endpoint_records = compute_bridge_endpoints(&group_cells, terrain, width, height);

        Self {
            width,
            height,
            cells,
            group_cells,
            group_hitpoints,
            strength_per_group: strength,
            bridge_strength: strength, // currently same; Phase F can split if needed
            endpoint_records,
            anchor_spans,
            bridge_destroyable_flag: destroyable,
        }
    }

    /// Look up an anchor span by ID.
    pub fn anchor_span(&self, id: u16) -> Option<&AnchorSpan> {
        self.anchor_spans.get(&id)
    }

    /// All anchor spans, sorted by ID (BTreeMap iteration order).
    pub fn anchor_spans(&self) -> &BTreeMap<u16, AnchorSpan> {
        &self.anchor_spans
    }

    pub fn cell(&self, rx: u16, ry: u16) -> Option<&BridgeRuntimeCell> {
        index_of(self.width, self.height, rx, ry)
            .and_then(|idx| self.cells.get(idx))
            .and_then(|cell| cell.as_ref())
    }

    /// Mutable cell access. Returns `None` if `(rx, ry)` is out of bounds or
    /// the cell is not a bridge runtime cell.
    pub fn cell_mut(&mut self, rx: u16, ry: u16) -> Option<&mut BridgeRuntimeCell> {
        index_of(self.width, self.height, rx, ry)
            .and_then(move |idx| self.cells.get_mut(idx))
            .and_then(|cell| cell.as_mut())
    }

    /// Test-only: insert a `BridgeRuntimeCell` at `(rx, ry)`, growing the
    /// internal `cells` Vec and `width`/`height` to fit if needed. Used by
    /// unit tests that need precise control over cell placement and state
    /// without going through `from_resolved_terrain`.
    #[cfg(test)]
    pub(crate) fn test_seed_cell(&mut self, rx: u16, ry: u16, cell: BridgeRuntimeCell) {
        let needed_w = (rx + 1).max(self.width);
        let needed_h = (ry + 1).max(self.height);
        if needed_w != self.width || needed_h != self.height {
            // Resize while preserving existing (rx, ry) → cell mappings.
            let mut new_cells = vec![None; needed_w as usize * needed_h as usize];
            for old_ry in 0..self.height {
                for old_rx in 0..self.width {
                    let old_idx = old_ry as usize * self.width as usize + old_rx as usize;
                    let new_idx = old_ry as usize * needed_w as usize + old_rx as usize;
                    new_cells[new_idx] = self.cells[old_idx];
                }
            }
            self.cells = new_cells;
            self.width = needed_w;
            self.height = needed_h;
        }
        let idx = ry as usize * self.width as usize + rx as usize;
        self.cells[idx] = Some(cell);
    }

    /// Test-only: insert an `AnchorSpan` directly into the registry.
    #[cfg(test)]
    pub(crate) fn test_seed_anchor_span(&mut self, span: AnchorSpan) {
        self.anchor_spans.insert(span.id, span);
    }

    pub fn is_bridge_walkable(&self, rx: u16, ry: u16) -> bool {
        self.cell(rx, ry).is_some_and(|cell| {
            cell.deck_present && !matches!(cell.damage_state, DamageState::Destroyed)
        })
    }

    pub fn apply_damage(&mut self, event: BridgeDamageEvent) -> Option<BridgeStateChange> {
        if event.damage == 0 {
            return None;
        }
        let cell = self.cell(event.rx, event.ry).copied()?;
        if !cell.deck_present
            || matches!(cell.damage_state, DamageState::Destroyed)
            || !cell.destroyable
        {
            return None;
        }
        let Some(group_id) = cell.bridge_group_id else {
            return None;
        };
        let hp = self
            .group_hitpoints
            .entry(group_id)
            .or_insert(self.strength_per_group);
        *hp = hp.saturating_sub(event.damage);
        if *hp > 0 {
            return None;
        }

        let mut destroyed_cells = self.group_cells.get(&group_id).cloned().unwrap_or_default();
        destroyed_cells.sort_unstable();
        for &(rx, ry) in &destroyed_cells {
            if let Some(idx) = index_of(self.width, self.height, rx, ry) {
                if let Some(cell) = self.cells[idx].as_mut() {
                    cell.damage_state = DamageState::Destroyed;
                }
            }
        }
        for record in &mut self.endpoint_records {
            if record.group_id == group_id {
                record.active = false;
            }
        }
        Some(BridgeStateChange { destroyed_cells })
    }

    /// Body-cell state-machine driver. Mirrors the body branch of binary
    /// `ProcessBridgeDamageStateMachine_High @ 0x576BA0` (HIGH §3.1).
    ///
    /// Receives damage on a body-bridge cell at `(rx, ry)`. Resolves anchor
    /// (follows `anchor_span_id` if input cell is `Body` or `Tail`), reads
    /// anchor's current `damage_state`, transitions per binary switch arms,
    /// fires perpendicular `UpdateRamp_*` writes via `update_ramp_perpendicular`,
    /// and on collapse emits `set_bridge_direction(span, false)` for the
    /// `BlowUpBridge` cascade.
    ///
    /// Returns `StateOutcome::Absorbed` for `Healthy → Damaged`,
    /// `StateOutcome::Collapsed { ... }` for `Damaged → Destroyed` and
    /// partial-collapse → `Destroyed`, and `StateOutcome::NoChange` for
    /// already-destroyed / non-body / unresolvable-anchor inputs.
    ///
    /// `is_high_bridge` is currently unused (state transitions identical for
    /// HIGH and LOW per HIGH §11.1) but kept for API symmetry with the
    /// future overlay-write branch.
    pub fn body_cell_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
    ) -> StateOutcome {
        // 1. Resolve input cell.
        let Some(input_cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };

        // 2. Filter: must be body-bridge (Anchor / Body / Tail). Bridgehead
        //    cells route to Task 14's bridgehead driver (not part of this plan).
        if !matches!(
            input_cell.role,
            BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Tail
        ) {
            return StateOutcome::NoChange;
        }

        // 3. Resolve anchor.
        let anchor_pos = if matches!(input_cell.role, BridgeCellRole::Anchor) {
            (rx, ry)
        } else {
            // Non-anchor body cell: follow anchor_span_id to span.anchor.
            let Some(span_id) = input_cell.anchor_span_id else {
                return StateOutcome::NoChange;
            };
            let Some(span) = self.anchor_span(span_id) else {
                return StateOutcome::NoChange;
            };
            span.anchor
        };

        let Some(anchor_cell) = self.cell(anchor_pos.0, anchor_pos.1).copied() else {
            return StateOutcome::NoChange;
        };
        let Some(axis) = anchor_cell.axis else {
            return StateOutcome::NoChange;
        };
        let span_id = match anchor_cell.anchor_span_id {
            Some(id) => id,
            None => return StateOutcome::NoChange,
        };
        let span_clone = match self.anchor_span(span_id) {
            Some(s) => s.clone(),
            None => return StateOutcome::NoChange,
        };

        // 4. Switch on anchor's damage_state.
        match anchor_cell.damage_state {
            DamageState::Healthy { .. } => {
                // Anchor advances to Damaged.
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Damaged;
                }
                // Fire UpdateRamp_*A and _*B on perpendicular targets.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageB, is_high_bridge,
                );
                StateOutcome::Absorbed
            }
            DamageState::Damaged => {
                // Full collapse — fire CollapseA + CollapseB perpendicular,
                // anchor → Destroyed, set_bridge_direction cascade.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseB, is_high_bridge,
                );
                let mut destroyed = vec![anchor_pos];
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Destroyed;
                }
                // Collect any perpendicular cells that hit collapse-final
                // (became Destroyed via update_ramp_perpendicular).
                for &perp_dir in &[Direction::E, Direction::W, Direction::N, Direction::S] {
                    let (dx, dy) = perp_dir.offset();
                    let nx = anchor_pos.0 as i32 + dx;
                    let ny = anchor_pos.1 as i32 + dy;
                    if nx < 0 || ny < 0 { continue; }
                    let pos = (nx as u16, ny as u16);
                    if let Some(c) = self.cell(pos.0, pos.1) {
                        if matches!(c.damage_state, DamageState::Destroyed)
                            && !destroyed.contains(&pos)
                        {
                            destroyed.push(pos);
                        }
                    }
                }
                let sbd = crate::sim::bridge_specs::set_bridge_direction(&span_clone, false);
                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: destroyed,
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::PartialCollapseA => {
                // Single CollapseA call, then collapse-finalize.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseA, is_high_bridge,
                );
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Destroyed;
                }
                let sbd = crate::sim::bridge_specs::set_bridge_direction(&span_clone, false);
                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: vec![anchor_pos],
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::PartialCollapseB => {
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseB, is_high_bridge,
                );
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Destroyed;
                }
                let sbd = crate::sim::bridge_specs::set_bridge_direction(&span_clone, false);
                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: vec![anchor_pos],
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::Destroyed => StateOutcome::NoChange,
        }
    }

    /// Bridgehead-cell state-machine driver. Mirrors the bridgehead branch of
    /// binary `ProcessBridgeDamageStateMachine_High @ 0x576BA0`.
    ///
    /// Counterpart to `body_cell_advance_state`. Filters on
    /// `role == Bridgehead`, walks to the anchor body cell via
    /// `bridgehead_walk_to_anchor`'s height predicate, then transitions per
    /// the cell's `damage_state`. Fires perpendicular `UpdateRamp_*` writes
    /// via `update_ramp_perpendicular` exactly like the body driver. On
    /// collapse: emits a 3-cell `BlowUpBridge` row (body-axis-aligned via
    /// `bridgehead_blow_up_row`) plus `adjacent_bridges_dirty` flags.
    ///
    /// Critical structural difference vs body branch: does NOT compose
    /// `set_bridge_direction(anchor.span, false)`. Binary leaves the body
    /// span's flag bits untouched on bridgehead destruction; the body span
    /// survives with state byte advanced one tier via the perpendicular
    /// `UpdateRamp_*_Collapse` call. Subsequent damage on body cells
    /// continues the collapse via `body_cell_advance_state`.
    ///
    /// Returns:
    /// - `StateOutcome::Absorbed` on bridgehead `Healthy → Damaged` (any
    ///   cosmetic healthy variant jump-transitions to Damaged in one hit;
    ///   mirrors binary writing overlay slot 2 raw which encodes step 3).
    /// - `StateOutcome::Collapsed { destroyed_cells, set_bridge_direction
    ///   (3-entry BlowUpBridge row — note: field name is reused from body
    ///   driver; bridgehead does NOT call SetBridgeDirection_NESW),
    ///   adjacent_bridges_dirty (perpendiculars of the *bridgehead* coord),
    ///   zones_dirty: true }` on `Damaged → Destroyed`.
    /// - `StateOutcome::NoChange` on non-bridgehead role, anchor walk
    ///   failure (off-map / odd-height intermediate / `cell.axis == None`),
    ///   already `Destroyed`, or `PartialCollapseA/B` (defensive).
    ///
    /// `is_high_bridge` is currently unused (state transitions identical
    /// for HIGH and LOW per HIGH §11.1) but kept for API symmetry with the
    /// future overlay-write branch and the body driver.
    ///
    /// Height-source: `ResolvedTerrainCell.template_height`. Per HIGH §13.5
    /// the binary field `+0x11A` is "bridge-class ID" (a different
    /// abstraction); if parity tests reveal walker drift on real maps, a
    /// derived `bridge_class_id` field on `ResolvedTerrainCell` will replace
    /// the closure source.
    pub fn bridgehead_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> StateOutcome {
        // 1. Resolve input cell.
        let Some(input_cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };

        // 2. Filter: must be a Bridgehead. Body / Anchor / Tail route to the
        //    body driver.
        if !matches!(input_cell.role, BridgeCellRole::Bridgehead) {
            return StateOutcome::NoChange;
        }
        let Some(axis) = input_cell.axis else {
            return StateOutcome::NoChange;
        };

        // 3. Walk to anchor via height predicate.
        let map_w = self.width;
        let map_h = self.height;
        let height_lookup = |pos: (u16, u16)| -> Option<u8> {
            terrain.cell(pos.0, pos.1).map(|c| c.template_height)
        };
        let walk_dir = match axis {
            Axis::NS => Direction::E,
            Axis::EW => Direction::S,
        };
        let Some(anchor_pos) = crate::sim::bridge_specs::bridgehead_walk_to_anchor(
            (rx, ry), axis, walk_dir, height_lookup, map_w, map_h,
        ) else {
            return StateOutcome::NoChange;
        };

        // 4. Switch on bridgehead's damage_state.
        match input_cell.damage_state {
            DamageState::Healthy { .. } => {
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageB, is_high_bridge,
                );
                if let Some(c) = self.cell_mut(rx, ry) {
                    c.damage_state = DamageState::Damaged;
                }
                StateOutcome::Absorbed
            }
            DamageState::Damaged => {
                let anchor_height = terrain
                    .cell(anchor_pos.0, anchor_pos.1)
                    .map(|c| c.template_height)
                    .unwrap_or(0);

                let row = crate::sim::bridge_specs::bridgehead_blow_up_row(
                    anchor_pos, axis, anchor_height, map_w, map_h,
                );

                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseB, is_high_bridge,
                );

                // Bridgehead's own state → Destroyed. NOTE: anchor's
                // damage_state is NOT modified — body span survives with
                // state byte advanced via the perpendicular UpdateRamp call.
                if let Some(c) = self.cell_mut(rx, ry) {
                    c.damage_state = DamageState::Destroyed;
                }

                // Collect any perpendicular cells that hit collapse-final.
                let mut destroyed = vec![(rx, ry)];
                for &perp_dir in
                    &[Direction::E, Direction::W, Direction::N, Direction::S]
                {
                    let (dx, dy) = perp_dir.offset();
                    let nx = anchor_pos.0 as i32 + dx;
                    let ny = anchor_pos.1 as i32 + dy;
                    if nx < 0 || ny < 0 {
                        continue;
                    }
                    let pos = (nx as u16, ny as u16);
                    if let Some(c) = self.cell(pos.0, pos.1) {
                        if matches!(c.damage_state, DamageState::Destroyed)
                            && !destroyed.contains(&pos)
                        {
                            destroyed.push(pos);
                        }
                    }
                }

                // Emit the 3-cell BlowUpBridge row as a SetBridgeDirectionResult.
                // Bridgehead branch does NOT call SetBridgeDirection_NESW;
                // we reuse the result type as a cascade carrier for the
                // orchestrator. Slot index is 0 — bridgehead's row is not
                // part of an AnchorSpan, so the slot has no meaning here.
                let actions: Vec<(
                    (u16, u16),
                    usize,
                    crate::sim::bridge_specs::CellAction,
                )> = row
                    .iter()
                    .filter_map(|c| {
                        c.map(|cell| {
                            (
                                cell,
                                0usize,
                                crate::sim::bridge_specs::CellAction::BlowUpBridge,
                            )
                        })
                    })
                    .collect();
                let sbd = crate::sim::bridge_specs::SetBridgeDirectionResult { actions };

                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: destroyed,
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::PartialCollapseA
            | DamageState::PartialCollapseB
            | DamageState::Destroyed => StateOutcome::NoChange,
        }
    }

    /// Bridge endpoint records for zone connectivity.
    /// Each active record connects ground zones on opposite sides of a bridge.
    pub fn endpoint_records(&self) -> &[BridgeEndpointRecord] {
        &self.endpoint_records
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = ((u16, u16), &BridgeRuntimeCell)> {
        self.cells
            .iter()
            .enumerate()
            .filter_map(move |(idx, cell)| {
                let cell = cell.as_ref()?;
                let rx = (idx % self.width as usize) as u16;
                let ry = (idx / self.width as usize) as u16;
                Some(((rx, ry), cell))
            })
    }
}

fn index_of(width: u16, height: u16, rx: u16, ry: u16) -> Option<usize> {
    (rx < width && ry < height).then_some(ry as usize * width as usize + rx as usize)
}

/// For each bridge group, find the two ground cells on opposite sides.
///
/// Algorithm: collect all ground cells cardinally adjacent to any bridge cell
/// in the group, then pick the pair with maximum Manhattan distance.
fn compute_bridge_endpoints(
    group_cells: &BTreeMap<u16, Vec<(u16, u16)>>,
    terrain: &ResolvedTerrainGrid,
    width: u16,
    height: u16,
) -> Vec<BridgeEndpointRecord> {
    let mut records = Vec::new();

    for (&group_id, members) in group_cells {
        // Collect ground cells adjacent to this bridge group.
        let mut ground_neighbors: Vec<(u16, u16)> = Vec::new();
        for &(bx, by) in members {
            for (nx, ny) in cardinal_neighbors(bx, by, width, height) {
                if members.contains(&(nx, ny)) {
                    continue;
                }
                if let Some(cell) = terrain.cell(nx, ny) {
                    if !cell.ground_walk_blocked && !cell.is_water
                        && !ground_neighbors.contains(&(nx, ny))
                    {
                        ground_neighbors.push((nx, ny));
                    }
                }
            }
        }

        if ground_neighbors.len() < 2 {
            continue;
        }

        // Pick the pair with maximum Manhattan distance.
        let mut best_a = ground_neighbors[0];
        let mut best_b = ground_neighbors[1];
        let mut best_dist: u32 = 0;
        for i in 0..ground_neighbors.len() {
            for j in (i + 1)..ground_neighbors.len() {
                let (ax, ay) = ground_neighbors[i];
                let (bx, by) = ground_neighbors[j];
                let dist = (ax as i32 - bx as i32).unsigned_abs()
                    + (ay as i32 - by as i32).unsigned_abs();
                if dist > best_dist {
                    best_dist = dist;
                    best_a = ground_neighbors[i];
                    best_b = ground_neighbors[j];
                }
            }
        }

        records.push(BridgeEndpointRecord {
            endpoint_a: best_a,
            endpoint_b: best_b,
            group_id,
            active: true,
        });
    }

    records
}

fn cardinal_neighbors(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
) -> impl Iterator<Item = (u16, u16)> {
    const OFFSETS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    OFFSETS.into_iter().filter_map(move |(dx, dy)| {
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;
        (nx >= 0 && ny >= 0 && (nx as u16) < width && (ny as u16) < height)
            .then_some((nx as u16, ny as u16))
    })
}

fn bridge_layer_to_axis(
    layer: Option<&crate::map::resolved_terrain::BridgeLayer>,
) -> Option<Axis> {
    layer.map(|bl| bridge_direction_to_axis(bl.direction))
}

fn bridge_direction_to_axis(d: crate::map::resolved_terrain::BridgeDirection) -> Axis {
    use crate::map::resolved_terrain::BridgeDirection;
    match d {
        BridgeDirection::EastWest => Axis::EW,
        BridgeDirection::NorthSouth => Axis::NS,
        // Low bridges (wood) read bridge_layer separately; treat as NS for now.
        // Phase C may revisit if low needs distinct axis handling.
        BridgeDirection::Low => Axis::NS,
    }
}

/// HIGH bridge anchor overlays = 0x18, 0x19; LOW bridge anchor overlays = 0xED, 0xEE.
///
/// NOTE (Phase B): These overlay IDs are also used to mark every HIGH-bridge
/// deck cell's direction, so under this predicate every HIGH-bridge cell with
/// a bridge_layer becomes an anchor. Phase C anchor-walker correctness tests
/// (Task 27) will tighten this to only true anchor cells.
fn is_anchor_overlay(overlay_id: u8) -> bool {
    matches!(overlay_id, 0x18 | 0x19 | 0xED | 0xEE)
}

/// State-machine convention: NS-axis collapse walks E (dir=2) for ramp A;
/// EW-axis collapse walks S (dir=4) for ramp A. We pick A-direction as the
/// canonical anchor walk direction (cell 5 then walks the opposite from anchor).
fn anchor_walk_direction(axis: Axis) -> Direction {
    match axis {
        Axis::NS => Direction::E,
        Axis::EW => Direction::S,
    }
}

/// Walk the 6-cell anchor pattern. Cells beyond the map edge become `None`.
fn walk_anchor_pattern(
    span_id: u16,
    anchor: (u16, u16),
    axis: Axis,
    direction: Direction,
    bridge_group_id: u16,
    width: u16,
    height: u16,
) -> AnchorSpan {
    let mut cells: [Option<(u16, u16)>; 6] = [None; 6];
    cells[0] = Some(anchor);

    let (dx, dy) = direction.offset();
    // Slot 1, 2, 3: walk +direction × 1, 2, 3.
    for step in 1..=3 {
        let nx = anchor.0 as i32 + dx * step;
        let ny = anchor.1 as i32 + dy * step;
        if nx >= 0 && ny >= 0 && (nx as u16) < width && (ny as u16) < height {
            cells[step as usize] = Some((nx as u16, ny as u16));
        }
    }

    // Slot 4: walk -direction × 1.
    let opp = direction.opposite();
    let (odx, ody) = opp.offset();
    let ox = anchor.0 as i32 + odx;
    let oy = anchor.1 as i32 + ody;
    if ox >= 0 && oy >= 0 && (ox as u16) < width && (oy as u16) < height {
        cells[4] = Some((ox as u16, oy as u16));
    }

    // Slot 5: optional fixed-offset only when direction == W.
    if direction == Direction::W {
        let ex = anchor.0 as i32 + 1;
        let ey = anchor.1 as i32;
        if ex >= 0 && ey >= 0 && (ex as u16) < width && (ey as u16) < height {
            cells[5] = Some((ex as u16, ey as u16));
        }
    }

    AnchorSpan {
        id: span_id,
        anchor,
        cells,
        axis,
        direction,
        damage_state: DamageState::Healthy { variant: 0 },
        bridge_group_id,
    }
}

/// Compute the two perpendicular cells where `UpdateAdjacentBridges_High`
/// should fire after a body-cell collapse. Per binary `0x576BA0`, the call
/// passes the ORIGINAL damaged cell coord (not the anchor); the offsets are
/// directional.
fn compute_adjacent_bridges_dirty(rx: u16, ry: u16, axis: Axis) -> Vec<(u16, u16)> {
    let mut out = Vec::with_capacity(2);
    let perpendiculars: [Direction; 2] = match axis {
        Axis::NS => [Direction::E, Direction::W],
        Axis::EW => [Direction::S, Direction::N],
    };
    for d in perpendiculars {
        let (dx, dy) = d.offset();
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;
        if nx >= 0 && ny >= 0 {
            out.push((nx as u16, ny as u16));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};

    /// 5x1 grid: ground at (0,0), bridge at (1,0)-(3,0), ground at (4,0).
    fn make_bridge_terrain() -> ResolvedTerrainGrid {
        let mut cells = Vec::new();
        for rx in 0..5u16 {
            let on_bridge = (1..=3).contains(&rx);
            cells.push(ResolvedTerrainCell {
                rx,
                ry: 0,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                level: 0,
                filled_clear: false,
                tileset_index: Some(0),
                land_type: 0,
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: TerrainClass::Clear,
                speed_costs: SpeedCostProfile::default(),
                is_water: false,
                is_cliff_like: false,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: on_bridge,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: if on_bridge { 6 } else { 0 },
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                build_blocked: on_bridge,
                has_bridge_deck: on_bridge,
                bridge_walkable: on_bridge,
                bridge_transition: rx == 1 || rx == 3,
                bridge_deck_level: if on_bridge { 4 } else { 0 },
                bridge_layer: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
            });
        }
        ResolvedTerrainGrid::from_cells(5, 1, cells)
    }

    #[test]
    fn bridge_runtime_initializes_intact_groups() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 300);
        let cell = state.cell(1, 0).expect("bridge cell");
        assert!(cell.deck_present);
        assert!(matches!(cell.damage_state, DamageState::Healthy { .. }));
        assert_eq!(cell.deck_level, 4);
        assert_eq!(cell.bridge_group_id, Some(1));
        assert!(state.cell(0, 0).is_none());
    }

    #[test]
    fn destroying_a_bridge_group_marks_all_members_destroyed() {
        let mut state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 50);
        let change = state
            .apply_damage(BridgeDamageEvent {
                rx: 1,
                ry: 0,
                damage: 50,
                warhead_ref: crate::sim::intern::InternedId::default(),
                is_ion_cannon: true,
                impact_z: 4,
            })
            .expect("bridge should be destroyed");
        assert_eq!(change.destroyed_cells, vec![(1, 0), (2, 0), (3, 0)]);
        assert!(!state.is_bridge_walkable(1, 0));
        assert!(!state.is_bridge_walkable(2, 0));
        assert!(!state.is_bridge_walkable(3, 0));
        // Verify damage_state per cell.
        assert_eq!(
            state.cell(1, 0).map(|c| c.damage_state),
            Some(DamageState::Destroyed)
        );
    }

    #[test]
    fn indestructible_bridge_ignores_damage() {
        let mut state =
            BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), false, 50);
        assert!(state
            .apply_damage(BridgeDamageEvent {
                rx: 1,
                ry: 0,
                damage: 50,
                warhead_ref: crate::sim::intern::InternedId::default(),
                is_ion_cannon: true,
                impact_z: 4,
            })
            .is_none());
        assert!(state.is_bridge_walkable(1, 0));
    }

    #[test]
    fn bridge_endpoints_detected() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 300);
        let records = state.endpoint_records();
        assert_eq!(records.len(), 1, "should have exactly one bridge endpoint record");
        let rec = &records[0];
        assert!(rec.active);
        assert_eq!(rec.group_id, 1);
        let endpoints = [rec.endpoint_a, rec.endpoint_b];
        assert!(endpoints.contains(&(0, 0)), "endpoint_a or _b should be (0,0)");
        assert!(endpoints.contains(&(4, 0)), "endpoint_a or _b should be (4,0)");
    }

    #[test]
    fn bridge_destruction_deactivates_endpoints() {
        let mut state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 50);
        state.apply_damage(BridgeDamageEvent {
            rx: 1,
            ry: 0,
            damage: 50,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        });
        let records = state.endpoint_records();
        assert!(!records.is_empty());
        assert!(
            !records[0].active,
            "endpoint should be deactivated after destruction"
        );
    }

    #[test]
    fn direction_offsets_match_compass() {
        assert_eq!(Direction::N.offset(), (0, -1));
        assert_eq!(Direction::E.offset(), (1, 0));
        assert_eq!(Direction::S.offset(), (0, 1));
        assert_eq!(Direction::W.offset(), (-1, 0));
    }

    #[test]
    fn direction_opposite_is_idempotent() {
        for dir in [
            Direction::N, Direction::NE, Direction::E, Direction::SE,
            Direction::S, Direction::SW, Direction::W, Direction::NW,
        ] {
            assert_eq!(dir.opposite().opposite(), dir);
        }
    }

    #[test]
    fn direction_opposite_pairs() {
        assert_eq!(Direction::N.opposite(), Direction::S);
        assert_eq!(Direction::E.opposite(), Direction::W);
        assert_eq!(Direction::NE.opposite(), Direction::SW);
        assert_eq!(Direction::SE.opposite(), Direction::NW);
    }

    fn make_test_span() -> AnchorSpan {
        AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), // slot 0 = anchor
                Some((6, 5)), // slot 1 = +E × 1
                Some((7, 5)), // slot 2 = +E × 2
                Some((8, 5)), // slot 3 = +E × 3 (FLAG ONLY)
                Some((4, 5)), // slot 4 = -E × 1 = +W × 1
                None,         // slot 5 = optional W-direction fixed offset
            ],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Healthy { variant: 0 },
            bridge_group_id: 1,
        }
    }

    #[test]
    fn anchor_span_blow_up_cells_excludes_slot_3() {
        let span = make_test_span();
        let cells: Vec<_> = span.blow_up_cells().collect();
        // Cells 1, 2, 3, 5 in 1-indexed numbering = our slots 0, 1, 2, 4.
        // NOT slot 3 (cell 4, flag-only).
        assert_eq!(cells, vec![(5, 5), (6, 5), (7, 5), (4, 5)]);
    }

    #[test]
    fn anchor_span_iter_cells_skips_none() {
        let span = make_test_span();
        let count = span.iter_cells().count();
        assert_eq!(count, 5); // 6 slots, 1 None
    }

    #[test]
    fn anchor_spans_empty_when_bridge_layer_none() {
        // The default test fixture sets bridge_layer: None, so pass 2 emits no
        // anchor spans. Verifies the constructor still wires everything else.
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 300);
        assert!(state.anchor_spans().is_empty());
        let cell = state.cell(1, 0).expect("bridge cell");
        assert!(cell.deck_present);
        assert!(matches!(cell.damage_state, DamageState::Healthy { .. }));
    }

    #[test]
    fn bridge_runtime_state_snapshot_round_trip() {
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(), true, 1500,
        );
        let json = serde_json::to_string(&state).expect("serialize");
        let restored: BridgeRuntimeState =
            serde_json::from_str(&json).expect("deserialize");
        // Compare cell-by-cell across the full grid.
        for (rx, ry) in [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)] {
            assert_eq!(state.cell(rx, ry), restored.cell(rx, ry), "cell ({rx},{ry})");
        }
        // Compare anchor spans.
        assert_eq!(
            state.anchor_spans().len(),
            restored.anchor_spans().len()
        );
        for (id, span) in state.anchor_spans() {
            assert_eq!(restored.anchor_span(*id), Some(span));
        }
        // is_bridge_walkable behavior parity.
        for (rx, ry) in [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)] {
            assert_eq!(
                state.is_bridge_walkable(rx, ry),
                restored.is_bridge_walkable(rx, ry),
                "walkability ({rx},{ry})"
            );
        }
    }

    #[test]
    fn overlay_byte_populated_at_map_load() {
        // make_bridge_terrain in this file creates a 5x1 strip; the constructor
        // populates overlay_byte from bridge_layer.overlay_id (or 0 if none).
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(),
            true,
            1500,
        );
        // Field is reachable on every populated bridge cell; type is u8.
        for (_, cell) in state.iter_cells() {
            let _byte: u8 = cell.overlay_byte;
        }
    }

    #[test]
    fn overlay_byte_round_trips_via_snapshot() {
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(), true, 1500,
        );
        let json = serde_json::to_string(&state).expect("serialize");
        let restored: BridgeRuntimeState =
            serde_json::from_str(&json).expect("deserialize");
        for ((rx, ry), cell) in state.iter_cells() {
            let r = restored.cell(rx, ry).expect("restored cell present");
            assert_eq!(cell.overlay_byte, r.overlay_byte, "overlay_byte at ({rx},{ry})");
        }
    }

    #[test]
    fn test_seed_cell_grows_grid_to_fit() {
        let mut state = BridgeRuntimeState::default();
        let cell = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x18,
        };
        state.test_seed_cell(5, 5, cell);
        let read = state.cell(5, 5).expect("seeded cell present");
        assert_eq!(read.overlay_byte, 0x18);
        assert_eq!(read.role, BridgeCellRole::Anchor);
    }

    #[test]
    fn cell_mut_writes_visible_through_cell_read() {
        let mut state = BridgeRuntimeState::default();
        let cell = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x18,
        };
        state.test_seed_cell(2, 2, cell);
        state.cell_mut(2, 2).unwrap().overlay_byte = 0xD2;
        assert_eq!(state.cell(2, 2).unwrap().overlay_byte, 0xD2);
    }

    #[test]
    fn damage_state_to_byte_ns_axis() {
        assert_eq!(DamageState::Healthy { variant: 0 }.to_state_byte(Axis::NS), 0);
        assert_eq!(DamageState::Healthy { variant: 3 }.to_state_byte(Axis::NS), 3);
        assert_eq!(DamageState::Healthy { variant: 5 }.to_state_byte(Axis::NS), 5);
        assert_eq!(DamageState::Damaged.to_state_byte(Axis::NS), 6);
        assert_eq!(DamageState::PartialCollapseA.to_state_byte(Axis::NS), 7);
        assert_eq!(DamageState::PartialCollapseB.to_state_byte(Axis::NS), 8);
        assert_eq!(DamageState::Destroyed.to_state_byte(Axis::NS), 0);
    }

    #[test]
    fn damage_state_to_byte_ew_axis() {
        assert_eq!(DamageState::Healthy { variant: 0 }.to_state_byte(Axis::EW), 9);
        assert_eq!(DamageState::Healthy { variant: 5 }.to_state_byte(Axis::EW), 14);
        assert_eq!(DamageState::Damaged.to_state_byte(Axis::EW), 0xF);
        assert_eq!(DamageState::PartialCollapseA.to_state_byte(Axis::EW), 0x11);
        assert_eq!(DamageState::PartialCollapseB.to_state_byte(Axis::EW), 0x10);
        assert_eq!(DamageState::Destroyed.to_state_byte(Axis::EW), 0);
    }

    #[test]
    fn damage_state_to_byte_clamps_healthy_variant() {
        // Variant > 5 is invalid input; should clamp to 5 (max defined healthy).
        assert_eq!(DamageState::Healthy { variant: 7 }.to_state_byte(Axis::NS), 5);
        assert_eq!(DamageState::Healthy { variant: 10 }.to_state_byte(Axis::EW), 14);
    }

    #[test]
    fn damage_state_from_byte_ns_range() {
        assert_eq!(DamageState::from_state_byte(0), Some(DamageState::Healthy { variant: 0 }));
        assert_eq!(DamageState::from_state_byte(3), Some(DamageState::Healthy { variant: 3 }));
        assert_eq!(DamageState::from_state_byte(5), Some(DamageState::Healthy { variant: 5 }));
        assert_eq!(DamageState::from_state_byte(6), Some(DamageState::Damaged));
        assert_eq!(DamageState::from_state_byte(7), Some(DamageState::PartialCollapseA));
        assert_eq!(DamageState::from_state_byte(8), Some(DamageState::PartialCollapseB));
    }

    #[test]
    fn damage_state_from_byte_ew_range() {
        assert_eq!(DamageState::from_state_byte(9), Some(DamageState::Healthy { variant: 0 }));
        assert_eq!(DamageState::from_state_byte(14), Some(DamageState::Healthy { variant: 5 }));
        assert_eq!(DamageState::from_state_byte(0xF), Some(DamageState::Damaged));
        assert_eq!(DamageState::from_state_byte(0x10), Some(DamageState::PartialCollapseB));
        assert_eq!(DamageState::from_state_byte(0x11), Some(DamageState::PartialCollapseA));
    }

    #[test]
    fn damage_state_from_byte_out_of_range_returns_none() {
        assert_eq!(DamageState::from_state_byte(0x12), None);
        assert_eq!(DamageState::from_state_byte(0xFF), None);
    }

    #[test]
    fn damage_state_round_trip_for_each_variant_per_axis() {
        // For every (axis × variant) pair where Destroyed is excluded (it's the
        // ambiguous post-collapse state).
        for axis in [Axis::NS, Axis::EW] {
            for state in [
                DamageState::Healthy { variant: 0 },
                DamageState::Healthy { variant: 5 },
                DamageState::Damaged,
                DamageState::PartialCollapseA,
                DamageState::PartialCollapseB,
            ] {
                let byte = state.to_state_byte(axis);
                let decoded = DamageState::from_state_byte(byte)
                    .expect("decode succeeds for byte produced by encode");
                assert_eq!(decoded, state, "round-trip {state:?} via {axis:?}");
            }
        }
    }

    fn make_body_driver_test_state() -> BridgeRuntimeState {
        // Uses test_seed_cell + test_seed_anchor_span from Task 1 Step 5.
        // Layout for the body-driver tests:
        //   (5,5)  → anchor cell, axis NS, anchor_span_id=1
        //   (4,5), (6,5) → perpendicular anchor partners (axis NS, separate
        //                  span_id) — UpdateRamp_*A walks E, _*B walks W from
        //                  (5,5), so these are the wrappers' targets.
        //   (5,4)  → non-anchor body cell, anchor_span_id=1 — exercises the
        //                  "follow to anchor" path in the driver.
        // Other slots (7,5), (8,5) are referenced by the AnchorSpan but not
        // seeded — body driver doesn't read them, only the partner indirection
        // and the perpendicular cells.
        let mut state = BridgeRuntimeState::default();

        let healthy_template = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x18,
        };

        // Anchor at (5,5).
        state.test_seed_cell(5, 5, healthy_template);

        // Perpendicular anchor partners. They are anchors of their own spans
        // (binary's `+0x80` flag is set), so use anchor_span_id=2.
        let perp = BridgeRuntimeCell {
            anchor_span_id: Some(2),
            ..healthy_template
        };
        state.test_seed_cell(4, 5, perp);
        state.test_seed_cell(6, 5, perp);

        // Non-anchor body cell with anchor_span_id=1 — used by
        // body_driver_non_anchor_body_cell_follows_to_anchor.
        state.test_seed_cell(
            5, 4,
            BridgeRuntimeCell {
                role: BridgeCellRole::Body,
                ..healthy_template
            },
        );

        // AnchorSpan registry entry. The driver looks up by anchor_span_id
        // and reads `span.anchor` to resolve. Slot positions beyond (5,5),
        // (4,5), (6,5) aren't seeded as cells because the driver doesn't
        // touch them in the body-cell branch.
        state.test_seed_anchor_span(AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), Some((6, 5)), Some((7, 5)),
                Some((8, 5)), Some((4, 5)), None,
            ],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Healthy { variant: 0 },
            bridge_group_id: 1,
        });

        state
    }

    #[test]
    fn body_driver_anchor_healthy_advances_to_damaged_returns_absorbed() {
        let mut state = make_body_driver_test_state();
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::Absorbed));
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Damaged);
    }

    #[test]
    fn body_driver_non_anchor_body_cell_follows_to_anchor() {
        let mut state = make_body_driver_test_state();
        // Damage on a body cell, not the anchor.
        let outcome = state.body_cell_advance_state(5, 4, true);
        assert!(matches!(outcome, StateOutcome::Absorbed));
        // Anchor's damage_state advanced, not the input body cell's.
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Damaged);
        assert_eq!(state.cell(5, 4).unwrap().damage_state, DamageState::Healthy { variant: 0 });
    }

    #[test]
    fn body_driver_damaged_anchor_collapses_and_emits_set_bridge_direction() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::Damaged;
        let outcome = state.body_cell_advance_state(5, 5, true);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                set_bridge_direction,
                adjacent_bridges_dirty,
                zones_dirty,
            } => {
                assert!(destroyed_cells.contains(&(5, 5)));
                // 4 BlowUpBridge actions per Task 12 invariant.
                let blow_ups = set_bridge_direction.actions.iter()
                    .filter(|(_, _, a)| matches!(a,
                        crate::sim::bridge_specs::CellAction::BlowUpBridge))
                    .count();
                assert_eq!(blow_ups, 4);
                // 2 perpendicular cells flagged dirty (E and W of (5,5)).
                assert_eq!(adjacent_bridges_dirty.len(), 2);
                assert!(zones_dirty);
            }
            other => panic!("expected Collapsed, got {other:?}"),
        }
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Destroyed);
    }

    #[test]
    fn body_driver_partial_collapse_a_collapses_with_single_ramp_call() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::PartialCollapseA;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::Collapsed { .. }));
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Destroyed);
    }

    #[test]
    fn body_driver_partial_collapse_b_collapses_with_single_ramp_call() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::PartialCollapseB;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::Collapsed { .. }));
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Destroyed);
    }

    #[test]
    fn body_driver_destroyed_anchor_returns_no_change() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::Destroyed;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    #[test]
    fn body_driver_bridgehead_cell_returns_no_change() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().role = BridgeCellRole::Bridgehead;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    #[test]
    fn body_driver_out_of_bounds_returns_no_change() {
        let mut state = make_body_driver_test_state();
        let outcome = state.body_cell_advance_state(99, 99, true);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    /// 5x5 grid; row Y=2 carries the NS bridgehead walk:
    /// (2,2)=8 (bridgehead high-ramp peak), (3,2)=6, (4,2)=4 (anchor body).
    fn make_bridgehead_terrain_ns() -> crate::map::resolved_terrain::ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(25);
        for ry in 0..5u16 {
            for rx in 0..5u16 {
                let template_height: u8 = if ry == 2 {
                    match rx {
                        0 | 1 => 10,
                        2 => 8,
                        3 => 6,
                        4 => 4,
                        _ => 0,
                    }
                } else {
                    0
                };
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    slope_type: 0,
                    template_height,
                    render_offset_x: 0,
                    render_offset_y: 0,
                    terrain_class: TerrainClass::Clear,
                    speed_costs: SpeedCostProfile::default(),
                    is_water: false,
                    is_cliff_like: false,
                    is_cliff_redraw: false,
                    variant: 0,
                    is_rough: false,
                    is_road: false,
                    accepts_smudge: false,
                    has_ramp: false,
                    canonical_ramp: None,
                    ground_walk_blocked: false,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                });
            }
        }
        ResolvedTerrainGrid::from_cells(5, 5, cells)
    }

    /// Bridgehead at (2,2) NS, anchor at (4,2) NS, perpendicular partner
    /// anchors at (3,2) west of anchor and (5,2) east of anchor.
    /// All cells start `Healthy{0}`.
    fn make_bridgehead_state_ns() -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(
            2,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Bridgehead,
                anchor_span_id: None,
                overlay_byte: 0x18,
            },
        );
        state.test_seed_cell(
            4,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0x20,
            },
        );
        state.test_seed_cell(
            5,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0x21,
            },
        );
        state.test_seed_cell(
            3,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0x22,
            },
        );
        // Sentinel to grow state to 5x5 (matches terrain dimensions). The
        // 3-cell BlowUp row reaches Y=3 and Y=1; without this, state's
        // bounds check clamps the row to 2 cells.
        state.test_seed_cell(
            0,
            4,
            BridgeRuntimeCell {
                deck_present: false,
                destroyable: false,
                deck_level: 0,
                bridge_group_id: None,
                damage_state: DamageState::Healthy { variant: 0 },
                axis: None,
                role: BridgeCellRole::Body,
                anchor_span_id: None,
                overlay_byte: 0,
            },
        );
        state
    }

    #[test]
    fn bridgehead_advance_healthy_to_damaged_ns() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::Absorbed);
        assert_eq!(state.cell(2, 2).unwrap().damage_state, DamageState::Damaged);
        // Anchor is NOT modified — only perpendicular partners.
        assert_eq!(
            state.cell(4, 2).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        );
        // East partner — DamageA wrote state byte 4 → Healthy{4}.
        assert_eq!(
            state.cell(5, 2).unwrap().damage_state,
            DamageState::Healthy { variant: 4 }
        );
        // West partner — DamageB wrote state byte 5 → Healthy{5}.
        assert_eq!(
            state.cell(3, 2).unwrap().damage_state,
            DamageState::Healthy { variant: 5 }
        );
    }

    #[test]
    fn bridgehead_advance_damaged_to_destroyed_ns() {
        let mut state = make_bridgehead_state_ns();
        state.cell_mut(2, 2).unwrap().damage_state = DamageState::Damaged;
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                set_bridge_direction,
                adjacent_bridges_dirty,
                zones_dirty,
            } => {
                assert!(destroyed_cells.contains(&(2, 2)));
                assert_eq!(
                    state.cell(2, 2).unwrap().damage_state,
                    DamageState::Destroyed
                );
                // Anchor's damage_state must NOT be modified.
                assert_eq!(
                    state.cell(4, 2).unwrap().damage_state,
                    DamageState::Healthy { variant: 0 },
                    "anchor's damage_state must not be modified by bridgehead collapse"
                );
                // Perpendicular partners advance via CollapseA / CollapseB.
                // CollapseA from anchor walks E to (5,2): state 0 → 7 = PartialCollapseA.
                // CollapseB from anchor walks W to (3,2): state 0 → 8 = PartialCollapseB.
                assert_eq!(
                    state.cell(5, 2).unwrap().damage_state,
                    DamageState::PartialCollapseA
                );
                assert_eq!(
                    state.cell(3, 2).unwrap().damage_state,
                    DamageState::PartialCollapseB
                );
                // 3-cell BlowUp row at anchor (4,2), template_height=4 (even),
                // NS axis → column at X=4, Y in {1, 2, 3}.
                assert_eq!(set_bridge_direction.actions.len(), 3);
                let blow_cells: Vec<(u16, u16)> = set_bridge_direction
                    .actions
                    .iter()
                    .map(|(c, _, _)| *c)
                    .collect();
                assert!(blow_cells.contains(&(4, 1)));
                assert!(blow_cells.contains(&(4, 2)));
                assert!(blow_cells.contains(&(4, 3)));
                // adjacent_bridges_dirty uses bridgehead's coord (2,2), axis NS
                // → perpendiculars E/W → (3,2) and (1,2).
                let adj_set: std::collections::BTreeSet<(u16, u16)> =
                    adjacent_bridges_dirty.iter().copied().collect();
                assert!(adj_set.contains(&(3, 2)));
                assert!(adj_set.contains(&(1, 2)));
                assert!(zones_dirty);
            }
            other => panic!("expected Collapsed, got {:?}", other),
        }
    }

    #[test]
    fn bridgehead_advance_destroyed_no_change() {
        let mut state = make_bridgehead_state_ns();
        state.cell_mut(2, 2).unwrap().damage_state = DamageState::Destroyed;
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }

    #[test]
    fn bridgehead_advance_non_bridgehead_role_no_change() {
        let mut state = make_bridgehead_state_ns();
        state.cell_mut(2, 2).unwrap().role = BridgeCellRole::Body;
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }

    #[test]
    fn bridgehead_advance_anchor_walk_failure_no_change() {
        let mut state = make_bridgehead_state_ns();
        let mut terrain = make_bridgehead_terrain_ns();
        for c in terrain.cells.iter_mut() {
            c.template_height = 10;
        }
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
        assert_eq!(
            state.cell(2, 2).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        );
    }

    #[test]
    fn bridgehead_advance_partial_collapse_states_no_change() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        for partial in [DamageState::PartialCollapseA, DamageState::PartialCollapseB]
        {
            state.cell_mut(2, 2).unwrap().damage_state = partial;
            let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
            assert_eq!(outcome, StateOutcome::NoChange);
        }
    }

    #[test]
    fn bridgehead_advance_off_map_no_change() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(99, 99, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }
}
