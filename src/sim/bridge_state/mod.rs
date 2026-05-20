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

pub mod walker;

use crate::map::resolved_terrain::{BridgeDirection, ResolvedTerrainGrid};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// 8-neighbor direction offsets used by `apply_damaged_variant_flood_fill`.
/// Order: N, NE, E, SE, S, SW, W, NW (standard RA2 8-facing convention).
///
/// The order does not affect the final bit state — the flood-fill is bool-idempotent
/// via its early-return guard — but it is fixed for deterministic recursion order
/// across lockstep clients.
const EIGHT_NEIGHBOR_OFFSETS: [(i32, i32); 8] = [
    (0, -1),  // N
    (1, -1),  // NE
    (1, 0),   // E
    (1, 1),   // SE
    (0, 1),   // S
    (-1, 1),  // SW
    (-1, 0),  // W
    (-1, -1), // NW
];

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
        let base = match axis {
            Axis::NS => ns_base,
            Axis::EW => ew_base,
        };
        match self {
            DamageState::Healthy { variant } => base + variant.min(5),
            DamageState::Damaged => match axis {
                Axis::NS => 6,
                Axis::EW => 0xF,
            },
            DamageState::PartialCollapseA => match axis {
                Axis::NS => 7,
                Axis::EW => 0x11,
            },
            DamageState::PartialCollapseB => match axis {
                Axis::NS => 8,
                Axis::EW => 0x10,
            },
            DamageState::Destroyed => 0,
        }
    }

    /// Render-side state byte. Returns the *base* byte for `Healthy { variant }`
    /// (`0` for NS, `9` for EW) regardless of the stored variant. The renderer
    /// re-derives Latin-square jitter from cell `(x, y)` per the binary
    /// `DrawOverlay_Body` path (RE doc §3.3.1, ledger #4).
    pub fn render_state_byte(self, axis: Axis) -> u8 {
        match self {
            DamageState::Healthy { .. } => match axis {
                Axis::NS => 0,
                Axis::EW => 9,
            },
            other => other.to_state_byte(axis),
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

/// Per-cell anchor tile-class for bridgehead-adjacent cells.
///
/// Mirrors the four `IsoTileTypeIndex` slots used by the bridgehead state
/// machine. Each value corresponds to a BridgeSet-relative tile_id offset
/// (slot 0..3); the actual tile_ids are theater-portable via
/// `BridgeMiddle1` / `BridgeMiddle2`.
///
/// - `Variant0` — pristine bridgehead (map-load default for cells with no
///   author-damaged anchor placement).
/// - `Variant1` — first DamageB intermediate. Reached only via neighbor
///   `UpdateRamp_*_DamageB` progression on a Variant0 target.
/// - `Damaged` — second DamageB intermediate. Reached only via neighbor
///   `UpdateRamp_*_DamageB` progression on a Variant1 target. Also written
///   by Collapse* paths advancing any non-AboutToFall variant.
/// - `AboutToFall` — most-damaged variant. Two reach paths:
///   1. **Direct hit on a bridgehead cell** — the bridgehead state machine
///      writes the anchor straight to this slot (skipping Variant1/Damaged).
///   2. **Map-load author-damaged anchor** — maps may place this tile_id
///      directly; the renderer reflects it from frame 1.
///
/// Meaningful only when `BridgeRuntimeCell.role` is `Anchor` or
/// `Bridgehead`; the renderer ignores it on other roles.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum BridgeheadAnchorClass {
    #[default]
    Variant0,
    Variant1,
    Damaged,
    AboutToFall,
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

/// Per-event context passed from world orchestrator to `BridgeRuntimeState`
/// for the 4-path dispatcher. Carries the pre-resolved IonCannon flag, the
/// impact z (for state-machine Z-height gate), and the interner-resolved
/// warhead reference. The orchestrator owns the `&mut SimRng` and does the
/// actual RNG draws — drivers themselves are pure of RNG.
#[derive(Debug, Clone, Copy)]
pub struct BridgeDamageContext {
    pub damage: u16,
    pub warhead_ref: crate::sim::intern::InternedId,
    pub is_ion_cannon: bool,
    pub bridge_strength: u16,
    /// Tile-step level units (signed for safety). State-machine Z-gate fires
    /// when `impact_z ∈ [cell.level - 1, cell.level + 1]` (3-level window).
    pub impact_z: i32,
}

/// Path discriminator for the bridge-damage 4-path dispatcher.
/// Order matches the binary's outer-dispatch evaluation order
/// (HighSM → LowSM → LowDirect → HighDirect).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchPath {
    /// HIGH state-machine: anchor / body / tail / bridgehead cell whose
    /// overlay byte has already transitioned out of the raw body range.
    /// Includes Z-height range gate.
    HighStateMachine,
    /// LOW state-machine: same shape as `HighStateMachine` for low bridges.
    /// Includes Z-height range gate.
    LowStateMachine,
    /// LOW direct-overlay: `cell.overlay_byte ∈ [0x4A..=0x63]`. Single-shot;
    /// no Z-gate.
    LowDirect,
    /// HIGH direct-overlay: `cell.overlay_byte ∈ [0xCD..=0xE6]`. Single-shot;
    /// no Z-gate.
    HighDirect,
}

impl DispatchPath {
    /// State-machine paths support the IonCannon 3-retry loop. Direct-overlay
    /// paths are single-shot regardless of warhead.
    pub fn is_state_machine(self) -> bool {
        matches!(
            self,
            DispatchPath::HighStateMachine | DispatchPath::LowStateMachine
        )
    }
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
        set_bridge_direction: crate::sim::bridge_specs::SetBridgeDirectionResult,
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

/// Outcome of a single `body_cell_repair_state` call. Carries the
/// side-effects the caller must fire AFTER state mutation.
///
/// Side-effect gating mirrors the original engine's repair-walker semantics:
///   - `zones_dirty`: rebuild PathGrid + zone grid. Set only when a
///     **main-deck damaged or destroyed** cell was repaired —
///     bridgehead-only repairs do NOT trigger zones rebuild.
///   - `radar_cells`: mark these cells dirty in the minimap. Set only
///     for cells that transitioned **from `Destroyed`** to `Healthy`.
///   - `repaired_cells`: total mutated cell count for caller's
///     `bridge_state_changed` decision and metrics.
#[derive(Debug, Clone, Default)]
pub struct RepairOutcome {
    pub zones_dirty: bool,
    pub radar_cells: Vec<(u16, u16)>,
    pub repaired_cells: u32,
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

    /// Selects damaged-vs-undamaged sub-tile art for the deck TMP at draw time.
    /// Written only by the `ToggleBridgePavement`-equivalent path (deferred
    /// follow-up); defaults to `false` at map load.
    pub damaged_variant: bool,

    /// Anchor tile-class mirror written by the bridgehead state machine when
    /// damage lands on a bridgehead-class cell. Carries the visual variant
    /// of the anchor (or neighbor bridgehead progressed via `DamageB`).
    /// Defaults to `Variant0` at map load. The renderer follow-up will read
    /// this to pick the anchor's TMP tile variant; G3 lands the sim-side
    /// write only.
    #[serde(default)]
    pub bridgehead_anchor_class: BridgeheadAnchorClass,
}

/// Binary bridge record kind (`BridgeRecord+0x0C`).
///
/// Verified against `MapClass__ComputeBridgeZones @ 0x0056D6E0`:
/// high bridges write `0`, low bridges write `1`. `MapClass__FindBridgeRecord`
/// skips non-zero kinds, so callers must choose high-only vs all-record use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BridgeRecordKind {
    High,
    Low,
}

impl Default for BridgeRecordKind {
    fn default() -> Self {
        Self::High
    }
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
    /// High vs low bridge record kind.
    #[serde(default)]
    pub bridge_kind: BridgeRecordKind,
}

impl BridgeEndpointRecord {
    pub fn is_high(&self) -> bool {
        self.bridge_kind == BridgeRecordKind::High
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BridgeRuntimeState {
    width: u16,
    height: u16,
    cells: Vec<Option<BridgeRuntimeCell>>,
    group_cells: BTreeMap<u16, Vec<(u16, u16)>>,
    /// Strength constant from `[CombatDamage] BridgeStrength=` (default 1500).
    /// Used by the dispatcher's per-path BridgeStrength RNG gate.
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
        bridge_strength: u16,
    ) -> Self {
        let width = terrain.width();
        let height = terrain.height();
        let mut cells = vec![None; width as usize * height as usize];
        let mut group_cells: BTreeMap<u16, Vec<(u16, u16)>> = BTreeMap::new();
        let mut anchor_spans: BTreeMap<u16, AnchorSpan> = BTreeMap::new();
        let mut visited = vec![false; cells.len()];
        let mut next_group_id: u16 = 1;
        let mut next_span_id: u16 = 1;

        // Pass 1: BFS-group structural bridge cells. High bridges use the
        // authoritative SetBridgeDirection-equivalent facts; low bridges and
        // existing test fixtures keep the legacy deck fallback.
        for cell in terrain.iter() {
            let Some(index) = index_of(width, height, cell.rx, cell.ry) else {
                continue;
            };
            if visited[index] || !resolved_cell_has_runtime_deck(cell) {
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
                if !resolved_cell_has_runtime_deck(resolved) {
                    continue;
                }
                visited[idx] = true;
                members.push((rx, ry));
                cells[idx] = Some(BridgeRuntimeCell {
                    deck_present: true,
                    destroyable,
                    deck_level: resolved.bridge_deck_level,
                    bridge_group_id: Some(group_id),
                    damage_state: initial_bridge_damage_state(resolved),
                    axis: bridge_fact_axis(resolved)
                        .or_else(|| bridge_layer_to_axis(resolved.bridge_layer.as_ref())),
                    role: BridgeCellRole::Body, // overwritten in pass 2
                    anchor_span_id: None,
                    overlay_byte: resolved
                        .bridge_facts
                        .overlay_id
                        .or_else(|| resolved.bridge_layer.as_ref().map(|bl| bl.overlay_id))
                        .unwrap_or(0),
                    damaged_variant: false,
                    bridgehead_anchor_class: resolved
                        .bridgehead_anchor_class_at_load
                        .unwrap_or(BridgeheadAnchorClass::Variant0),
                });
                for (nx, ny) in cardinal_neighbors(rx, ry, width, height) {
                    if let Some(neighbor) = terrain.cell(nx, ny) {
                        if resolved_cell_has_runtime_deck(neighbor) {
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
            if !members.is_empty() {
                group_cells.insert(group_id, members);
            }
        }

        // Pass 2: walk anchor patterns. High bridges trust the 0x80 anchor
        // fact. Low/legacy bridges keep the previous bridge_layer fallback.
        for (&group_id, members) in &group_cells {
            for &(rx, ry) in members {
                let Some(resolved) = terrain.cell(rx, ry) else {
                    continue;
                };
                let fact_anchor = resolved.bridge_facts.is_anchor_self();
                let legacy_anchor = resolved.bridge_facts.family
                    == crate::map::bridge_facts::BridgeStampFamily::None
                    && resolved
                        .bridge_layer
                        .as_ref()
                        .is_some_and(|bl| is_anchor_overlay(bl.overlay_id));
                if !fact_anchor && !legacy_anchor {
                    continue;
                }
                let (axis, direction) = if fact_anchor {
                    let stamp_direction = resolved.bridge_facts.direction.unwrap_or(0);
                    (
                        bridge_stamp_direction_to_axis(stamp_direction),
                        bridge_stamp_direction_to_direction(stamp_direction),
                    )
                } else {
                    let bl = resolved
                        .bridge_layer
                        .as_ref()
                        .expect("legacy anchor checked bridge_layer above");
                    let axis = bridge_direction_to_axis(bl.direction);
                    (axis, anchor_walk_direction(axis))
                };
                let span_id = next_span_id;
                next_span_id = next_span_id.saturating_add(1);
                let span = walk_anchor_pattern(
                    span_id,
                    (rx, ry),
                    axis,
                    direction,
                    group_id,
                    width,
                    height,
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

        // Pass 4: register bridgehead cells. ResolvedTerrainCell sets
        // bridge_walkable=true and has_bridge_deck=false at every bridgehead
        // (see resolved_terrain.rs bridgehead pass). Bridgeheads are NOT
        // created in pass 1 (no deck) and NOT touched by pass 3 (no
        // bridge_layer). Without this pass the rebuild silently flips
        // PathCell.bridge_walkable to false on every rebuild_dynamic_path_grid.
        //
        // Contract: deck_present=true permanently, damage_state=Healthy
        // permanently, bridge_group_id=None, anchor_span_id=None, axis=None,
        // overlay_byte=0. The dispatcher (path_matches_cell HighSM/LowSM)
        // rejects Bridgehead+axis.is_none() so no damage-event RNG fires on
        // these cells. Pass-3 bridgeheads (axis=Some) stay in the allowed set.
        for cell in terrain.iter() {
            if !cell.bridge_walkable || cell.has_bridge_deck {
                continue;
            }
            let Some(idx) = index_of(width, height, cell.rx, cell.ry) else {
                continue;
            };
            if cells[idx].is_some() {
                // Defensive: pass 1 already registered a cell here. The
                // condition (bw && !has_deck) should be mutually exclusive
                // with pass 1's has_deck, so this branch is unreachable
                // unless the resolved terrain is internally inconsistent.
                continue;
            }
            cells[idx] = Some(BridgeRuntimeCell {
                deck_present: true,
                destroyable,
                deck_level: cell.bridge_deck_level,
                bridge_group_id: None,
                damage_state: DamageState::Healthy { variant: 0 },
                axis: None,
                role: BridgeCellRole::Bridgehead,
                anchor_span_id: None,
                overlay_byte: 0,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            });
        }

        let endpoint_records =
            compute_bridge_endpoints(&group_cells, terrain, width, height, &cells);

        Self {
            width,
            height,
            cells,
            group_cells,
            bridge_strength: bridge_strength.max(1),
            endpoint_records,
            anchor_spans,
            bridge_destroyable_flag: destroyable,
        }
    }

    /// Look up an anchor span by ID.
    pub fn anchor_span(&self, id: u16) -> Option<&AnchorSpan> {
        self.anchor_spans.get(&id)
    }

    /// Mutable counterpart to `anchor_span`. Used by `body_cell_repair_state`
    /// to sync the span's mirror `damage_state` field after per-cell repair.
    pub fn anchor_span_mut(&mut self, id: u16) -> Option<&mut AnchorSpan> {
        self.anchor_spans.get_mut(&id)
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

    /// Map width in cells. Needed by walker code in the `walker` submodule
    /// (Rust privacy: child modules can't read parent's private fields
    /// without a getter or `pub(super)`).
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Map height in cells. See `width()` rationale.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// `[CombatDamage] BridgeStrength=` value used by the per-path RNG gate
    /// in the bridge-damage dispatcher. Read-only; set at construction.
    pub fn bridge_strength(&self) -> u16 {
        self.bridge_strength
    }

    /// Whether the global `SpecialFlags::DestroyableBridges` is set. Outer
    /// gate of the bridge-damage dispatcher; if false, bridges are immune.
    pub fn is_destroyable(&self) -> bool {
        self.bridge_destroyable_flag
    }

    /// Per-path entry-condition classifier for the world orchestrator. Pure
    /// function; no mutation. Returns true iff the cell at `(rx, ry)`
    /// matches the entry conditions for `path` under `ctx`.
    ///
    /// Mirrors the binary's per-path entry checks:
    /// - HighStateMachine / LowStateMachine: cell is bridge-structural and
    ///   has transitioned out of the raw body overlay range. Z-gate
    ///   restricts `impact_z` to `[cell.level - 1, cell.level + 1]`.
    ///   Raw-overlay cells are routed through the walker, NOT the state
    ///   machine, so this path explicitly REJECTS cells whose `overlay_byte`
    ///   is still in the body range.
    /// - HighDirect: `overlay_byte ∈ [0xCD..=0xE6]`. Single-shot, no Z-gate.
    /// - LowDirect:  `overlay_byte ∈ [0x4A..=0x63]`. Single-shot, no Z-gate.
    pub(crate) fn path_matches_cell(
        &self,
        path: DispatchPath,
        rx: u16,
        ry: u16,
        ctx: &BridgeDamageContext,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> bool {
        let Some(cell) = self.cell(rx, ry) else {
            return false;
        };
        match path {
            DispatchPath::HighDirect => (0xCD..=0xE6).contains(&cell.overlay_byte),
            DispatchPath::LowDirect => (0x4A..=0x63).contains(&cell.overlay_byte),
            DispatchPath::HighStateMachine | DispatchPath::LowStateMachine => {
                // Raw-overlay cells route to the walker, NOT the state
                // machine. State-machine fires only after the overlay has
                // been transitioned out of the body range.
                if matches!(path, DispatchPath::HighStateMachine)
                    && (0xCD..=0xE6).contains(&cell.overlay_byte)
                {
                    return false;
                }
                if matches!(path, DispatchPath::LowStateMachine)
                    && (0x4A..=0x63).contains(&cell.overlay_byte)
                {
                    return false;
                }
                if !matches!(
                    cell.role,
                    BridgeCellRole::Anchor
                        | BridgeCellRole::Body
                        | BridgeCellRole::Tail
                        | BridgeCellRole::Bridgehead
                ) {
                    return false;
                }
                // Pass-4 bridgeheads (registered by `from_resolved_terrain`'s
                // bridgehead pass) have axis=None and would cause
                // `bridgehead_advance_state` to return NoChange — but only
                // after the per-path BridgeStrength RNG roll already burned a
                // draw. Reject them here so the dispatcher never rolls RNG
                // for a pass-4-targeted event (lockstep). Pass-3 bridgeheads
                // (axis=Some, registered from `bridge_layer.direction`) keep
                // their existing routing into the bridgehead state machine.
                if matches!(cell.role, BridgeCellRole::Bridgehead) && cell.axis.is_none() {
                    return false;
                }
                // High vs low discriminator: deck_level >= 4 is "high"
                // (matches binary's tile-step gate). Bridgehead cells share
                // the same axis classification.
                let is_high = cell.deck_level >= 4;
                let want_high = matches!(path, DispatchPath::HighStateMachine);
                if is_high != want_high {
                    return false;
                }
                // Z-height range gate: pass when `impact_z` is within one
                // level above or below the bridge deck level. Direct-overlay
                // paths skip this gate.
                let level_i32 = terrain
                    .cell(rx, ry)
                    .map(|c| c.level as i32)
                    .unwrap_or(cell.deck_level as i32);
                if ctx.impact_z < level_i32 - 1 || ctx.impact_z > level_i32 + 1 {
                    return false;
                }
                true
            }
        }
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

    #[cfg(test)]
    pub(crate) fn test_set_endpoint_records(&mut self, records: Vec<BridgeEndpointRecord>) {
        self.endpoint_records = records;
    }

    pub fn is_bridge_walkable(&self, rx: u16, ry: u16) -> bool {
        self.cell(rx, ry).is_some_and(|cell| {
            cell.deck_present && !matches!(cell.damage_state, DamageState::Destroyed)
        })
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
        terrain: &ResolvedTerrainGrid,
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
                    self,
                    anchor_pos,
                    axis,
                    Phase::DamageA,
                    is_high_bridge,
                    terrain,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self,
                    anchor_pos,
                    axis,
                    Phase::DamageB,
                    is_high_bridge,
                    terrain,
                );
                StateOutcome::Absorbed
            }
            DamageState::Damaged => {
                // Full collapse — fire CollapseA + CollapseB perpendicular,
                // anchor → Destroyed, set_bridge_direction cascade.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self,
                    anchor_pos,
                    axis,
                    Phase::CollapseA,
                    is_high_bridge,
                    terrain,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self,
                    anchor_pos,
                    axis,
                    Phase::CollapseB,
                    is_high_bridge,
                    terrain,
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
                    self,
                    anchor_pos,
                    axis,
                    Phase::CollapseA,
                    is_high_bridge,
                    terrain,
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
                    self,
                    anchor_pos,
                    axis,
                    Phase::CollapseB,
                    is_high_bridge,
                    terrain,
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

    /// Propagate the damaged-variant bit across an 8-neighbor region bounded
    /// by underlying-terrain `final_tile_index` equality. The kickoff call
    /// gates on the seed cell's `has_damaged_data` flag; recursive calls skip
    /// the gate (cells sharing a tile_index share the gate flag, since they're
    /// rendered from the same TMP).
    ///
    /// `state == true` flips the bit on (damage / collapse caller path).
    /// `state == false` flips it off (repair walker path).
    ///
    /// Idempotent: cells already in the target state return early without
    /// recursing.
    ///
    /// Returns the count of cells mutated.
    pub fn apply_damaged_variant_flood_fill(
        &mut self,
        rx: u16,
        ry: u16,
        state: bool,
        terrain: &ResolvedTerrainGrid,
    ) -> u32 {
        self.apply_damaged_variant_flood_fill_internal(rx, ry, state, terrain, true)
    }

    fn apply_damaged_variant_flood_fill_internal(
        &mut self,
        rx: u16,
        ry: u16,
        state: bool,
        terrain: &ResolvedTerrainGrid,
        kickoff: bool,
    ) -> u32 {
        let cell_state = match self.cell(rx, ry) {
            Some(c) => c.damaged_variant,
            None => return 0,
        };
        if cell_state == state {
            return 0;
        }

        let resolved = match terrain.cell(rx, ry) {
            Some(c) => c,
            None => return 0,
        };
        let seed_tile_id = resolved.final_tile_index;
        if seed_tile_id == 0xFFFF || seed_tile_id < 0 {
            return 0;
        }

        if kickoff && !resolved.has_damaged_data {
            return 0;
        }

        if let Some(c) = self.cell_mut(rx, ry) {
            c.damaged_variant = state;
        }
        let mut count: u32 = 1;

        for (dx, dy) in EIGHT_NEIGHBOR_OFFSETS {
            let nx_i = rx as i32 + dx;
            let ny_i = ry as i32 + dy;
            if nx_i < 0 || ny_i < 0 {
                continue;
            }
            let nx = nx_i as u16;
            let ny = ny_i as u16;
            if let Some(n_resolved) = terrain.cell(nx, ny) {
                if n_resolved.final_tile_index == seed_tile_id {
                    count += self
                        .apply_damaged_variant_flood_fill_internal(nx, ny, state, terrain, false);
                }
            }
        }

        count
    }

    /// Reverse counterpart to `body_cell_advance_state`. Repairs cells found
    /// in `scan_cells`: collects unique `anchor_span_id`s, iterates each
    /// span's cells (slots 0..6), and transitions
    /// `Damaged`/`Destroyed`/`PartialCollapse{A,B}` → `Healthy { variant }`.
    ///
    /// The Rust model uses anchor-span iteration in place of the binary's
    /// 3-cell-perpendicular-strip walker — the cell-state mutations are
    /// equivalent; the binary's RNG draw count differs (per-strip vs
    /// per-cell), locked across our Rust clients by the iteration-order pin
    /// test.
    ///
    /// **Side-effect gating:**
    ///   - `outcome.zones_dirty = true` iff at least one **main-deck**
    ///     (Anchor/Body/Tail role) damaged or destroyed cell was repaired.
    ///     Bridgehead-only repairs do NOT set this flag.
    ///   - `outcome.radar_cells` contains cells whose prior state was
    ///     `Destroyed`. Cells transitioning from `Damaged` or
    ///     `PartialCollapse{A,B}` are NOT added.
    ///
    /// **RNG draws** (locked for lockstep across Rust clients):
    ///   - Main-deck damaged/destroyed/partial-collapse → 1 draw per cell
    ///     (`rng.next_range_u32(4)` → variant `0..=3`). MUST stay in `0..=3`
    ///     because variants 4/5 are RESERVED for `update_ramp_perpendicular`
    ///     to encode NS DamageA/B (they would render as damage-progression
    ///     SHP frames).
    ///   - Bridgehead damaged → write `Healthy { variant: 0 }`, **0 draws**.
    ///   - Already-`Healthy` or non-bridge cells → skip, **0 draws**.
    ///
    /// **Iteration order** (parity-critical, locked by test):
    ///   1. Anchor spans collected into `BTreeSet<u16>` for sorted iteration.
    ///   2. Within each span, cells iterated in slot order 0..=5.
    ///   3. `None` slots skipped.
    pub fn body_cell_repair_state(
        &mut self,
        scan_cells: &[(u16, u16)],
        rng: &mut crate::sim::rng::SimRng,
        terrain: &ResolvedTerrainGrid,
    ) -> RepairOutcome {
        let mut outcome = RepairOutcome::default();

        // Step 1: Collect unique anchor spans from scan cells.
        let mut spans: BTreeSet<u16> = BTreeSet::new();
        for &(rx, ry) in scan_cells {
            if let Some(cell) = self.cell(rx, ry) {
                if let Some(span_id) = cell.anchor_span_id {
                    spans.insert(span_id);
                }
            }
        }

        // Step 2: Iterate each span; for each cell, transition damage_state.
        for span_id in spans {
            // Clone span cell list to avoid borrow conflict.
            let cells_list: [Option<(u16, u16)>; 6] = match self.anchor_span(span_id) {
                Some(span) => span.cells,
                None => continue,
            };

            for slot in 0..6 {
                let Some(cell_pos) = cells_list[slot] else {
                    continue;
                };
                let Some(prior_state) = self.cell(cell_pos.0, cell_pos.1).map(|c| c.damage_state)
                else {
                    continue;
                };
                let Some(role) = self.cell(cell_pos.0, cell_pos.1).map(|c| c.role) else {
                    continue;
                };

                let new_state: DamageState = match (role, prior_state) {
                    // Already healthy: skip, no RNG draw.
                    (_, DamageState::Healthy { .. }) => continue,

                    // Bridgehead: fixed variant, no RNG.
                    (BridgeCellRole::Bridgehead, _) => DamageState::Healthy { variant: 0 },

                    // Main-deck (Anchor/Body/Tail) damaged/destroyed/partial: RNG variant.
                    (
                        BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Tail,
                        DamageState::Damaged
                        | DamageState::Destroyed
                        | DamageState::PartialCollapseA
                        | DamageState::PartialCollapseB,
                    ) => {
                        // Variant range MUST be 0..=3 (rng.next_range_u32(4));
                        // variants 4/5 encode NS DamageA/B in our render model
                        // and would draw damage-progression SHP frames.
                        let variant = rng.next_range_u32(4) as u8;
                        DamageState::Healthy { variant }
                    }
                };

                if let Some(cell) = self.cell_mut(cell_pos.0, cell_pos.1) {
                    cell.damage_state = new_state;
                }
                let _ =
                    self.apply_damaged_variant_flood_fill(cell_pos.0, cell_pos.1, false, terrain);
                outcome.repaired_cells += 1;

                let is_main_deck = matches!(
                    role,
                    BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Tail
                );
                if is_main_deck {
                    outcome.zones_dirty = true;
                }
                if matches!(prior_state, DamageState::Destroyed) {
                    outcome.radar_cells.push(cell_pos);
                }
            }

            // Step 3: Sync the AnchorSpan's mirror `damage_state` field with
            // the anchor cell's new state (the span struct caches this for
            // queries; existing forward state machine does the same).
            let anchor_pos = self.anchor_span(span_id).map(|s| s.anchor);
            if let Some((arx, ary)) = anchor_pos {
                let new_anchor_state = self.cell(arx, ary).map(|c| c.damage_state);
                if let (Some(state), Some(span)) = (new_anchor_state, self.anchor_span_mut(span_id))
                {
                    span.damage_state = state;
                }
            }
        }

        outcome
    }

    /// Bridgehead-cell state-machine driver.
    ///
    /// Sparse-by-design: most bridgehead cells absorb damage via the per-axis
    /// start-cell gate inside `bridgehead_walk_to_anchor` (NS rejects odd
    /// heights; EW rejects heights > 4). Only the small subset that passes
    /// the gate reaches the anchor-write path.
    ///
    /// On a successful walk:
    /// - Writes `bridgehead_anchor_class = AboutToFall` on the anchor cell.
    ///   This is the **most-damaged variant** (4th slot in the enum, matching
    ///   the reference engine's anchor-tile write target). The write is
    ///   idempotent — repeat hits leave the anchor at AboutToFall.
    /// - Fires `update_ramp_perpendicular(DamageA)` and `DamageB` on the
    ///   anchor's perpendicular neighbors. These do both the existing
    ///   state-byte bump (on Anchor targets) AND the asymmetric A/B
    ///   tile-class progression (on Anchor and Bridgehead targets) —
    ///   `Variant0 → Variant1 → Damaged` via DamageB; DamageA preserves.
    /// - The hit bridgehead cell's own `damage_state` is NEVER modified.
    ///
    /// Returns:
    /// - `StateOutcome::Absorbed` on a successful walk + anchor write.
    /// - `StateOutcome::NoChange` on role mismatch, missing axis, gated
    ///   start cell, or walk-off-map.
    /// - **Never** returns `Collapsed`. Sustained bridgehead direct fire
    ///   cannot collapse a bridge on this path; the body-cell cascade
    ///   (via `body_cell_advance_state`) is the only collapse route.
    ///
    /// `is_high_bridge` is currently unused (state transitions identical
    /// for HIGH and LOW per HIGH §11.1) but kept for API symmetry.
    ///
    /// Height-source: `ResolvedTerrainCell.template_height`.
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

        // 3. Walk to anchor via the height-based predicate. The helper
        //    computes walk direction internally per the start cell's height
        //    and applies the per-axis start-cell gate. Failures (odd-h NS,
        //    h>4 EW, off-map) yield None — the damage is absorbed without
        //    state change.
        let map_w = self.width;
        let map_h = self.height;
        let height_lookup = |pos: (u16, u16)| -> Option<u8> {
            terrain.cell(pos.0, pos.1).map(|c| c.template_height)
        };
        let Some(anchor_pos) = crate::sim::bridge_specs::bridgehead_walk_to_anchor(
            (rx, ry),
            axis,
            height_lookup,
            map_w,
            map_h,
        ) else {
            return StateOutcome::NoChange;
        };

        // 4. Write the anchor's bridgehead_anchor_class to AboutToFall
        //    (the most-damaged variant, 4th enum slot). Matches the
        //    reference engine's first-hit write to the anchor's tile-class
        //    field. The write is idempotent on repeat hits (AboutToFall
        //    stays AboutToFall). The hit bridgehead cell's own
        //    damage_state is never touched.
        if let Some(anchor_cell) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
            anchor_cell.bridgehead_anchor_class = BridgeheadAnchorClass::AboutToFall;
        }

        // 5. Fire the perpendicular DamageA + DamageB writes. These do the
        //    state-byte bump on Anchor targets and the asymmetric A/B
        //    tile-class progression on both Anchor and Bridgehead targets.
        let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
            self,
            anchor_pos,
            axis,
            Phase::DamageA,
            is_high_bridge,
            terrain,
        );
        let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
            self,
            anchor_pos,
            axis,
            Phase::DamageB,
            is_high_bridge,
            terrain,
        );

        StateOutcome::Absorbed
    }

    /// Bridge endpoint records for zone connectivity.
    /// Each active record connects ground zones on opposite sides of a bridge.
    pub fn endpoint_records(&self) -> &[BridgeEndpointRecord] {
        &self.endpoint_records
    }

    /// Recompute `endpoint_records[*].active` flags from current cell damage
    /// state. Once any cell of a bridge group enters `DamageState::Destroyed`,
    /// the bridge can no longer carry traffic across the span — the record
    /// is deactivated so the zone graph (`zone_build`) stops treating its
    /// endpoint pair as connected.
    ///
    /// Deactivation is one-way (no re-activation). The legacy single-shot
    /// `apply_damage` flipped `active = false` when the entire group was
    /// destroyed in one hit; the orchestrator's state-machine collapses
    /// cells individually, so the first destroyed cell already severs the
    /// bridge and its zone connection.
    pub fn refresh_endpoint_active_flags(&mut self) {
        let mut destroyed_groups: BTreeSet<u16> = BTreeSet::new();
        for cell_opt in &self.cells {
            if let Some(cell) = cell_opt {
                if matches!(cell.damage_state, DamageState::Destroyed) {
                    if let Some(gid) = cell.bridge_group_id {
                        destroyed_groups.insert(gid);
                    }
                }
            }
        }
        for record in &mut self.endpoint_records {
            if destroyed_groups.contains(&record.group_id) {
                record.active = false;
            }
        }
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

/// Enumerate the 25 cells in a 5×5 inclusive `[-2..=+2]` scan around
/// `center`. Yields cell coordinates clamped to non-negative `(u16, u16)`
/// (cells with negative computed coords are skipped — they're off-map).
///
/// Used by the engineer-repair trigger and the hut-destruction collapse
/// dispatch. Inclusive bounds `-2..=+2` produce exactly 25 cells when the
/// center is interior; off-map negative cells are silently dropped.
pub fn cells_in_5x5_scan(center: (u16, u16)) -> impl Iterator<Item = (u16, u16)> {
    let (cx, cy) = (center.0 as i32, center.1 as i32);
    (-2..=2i32).flat_map(move |dy| {
        (-2..=2i32).filter_map(move |dx| {
            let nx = cx + dx;
            let ny = cy + dy;
            if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
                None
            } else {
                Some((nx as u16, ny as u16))
            }
        })
    })
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
    runtime_cells: &[Option<BridgeRuntimeCell>],
) -> Vec<BridgeEndpointRecord> {
    let mut records = Vec::new();

    for (&group_id, members) in group_cells {
        if bridge_record_kind_for_group(members, terrain) == BridgeRecordKind::Low {
            continue;
        }
        // Collect ground cells adjacent to this bridge group.
        let mut ground_neighbors: Vec<(u16, u16)> = Vec::new();
        for &(bx, by) in members {
            for (nx, ny) in cardinal_neighbors(bx, by, width, height) {
                if members.contains(&(nx, ny)) {
                    continue;
                }
                if let Some(cell) = terrain.cell(nx, ny) {
                    if !cell.ground_walk_blocked
                        && !cell.is_water
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
                let dist =
                    (ax as i32 - bx as i32).unsigned_abs() + (ay as i32 - by as i32).unsigned_abs();
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
            bridge_kind: BridgeRecordKind::High,
        });
    }

    records.extend(compute_low_bridge_tube_endpoints(
        terrain,
        width,
        height,
        runtime_cells,
    ));

    records
}

fn compute_low_bridge_tube_endpoints(
    terrain: &ResolvedTerrainGrid,
    width: u16,
    height: u16,
    runtime_cells: &[Option<BridgeRuntimeCell>],
) -> Vec<BridgeEndpointRecord> {
    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for cell in terrain.iter() {
        if !cell.is_low_bridge_tube_cell()
            || !has_opposite_low_bridge_tube_neighbors(terrain, cell.rx, cell.ry)
        {
            continue;
        }
        let Some(tube) = terrain.tube_at_cell(cell.rx, cell.ry) else {
            continue;
        };
        let Some(idx) = index_of(width, height, cell.rx, cell.ry) else {
            continue;
        };
        let Some(group_id) = runtime_cells
            .get(idx)
            .and_then(|runtime| runtime.as_ref())
            .and_then(|runtime| runtime.bridge_group_id)
        else {
            continue;
        };
        let endpoint_a = (cell.rx, cell.ry);
        let endpoint_b = tube.exit;
        let key = ordered_endpoint_key(endpoint_a, endpoint_b);
        if !seen.insert(key) {
            continue;
        }
        records.push(BridgeEndpointRecord {
            endpoint_a,
            endpoint_b,
            group_id,
            active: true,
            bridge_kind: BridgeRecordKind::Low,
        });
    }
    records
}

fn has_opposite_low_bridge_tube_neighbors(terrain: &ResolvedTerrainGrid, rx: u16, ry: u16) -> bool {
    (neighbor_is_low_bridge_tube(terrain, rx, ry, Direction::E)
        && neighbor_is_low_bridge_tube(terrain, rx, ry, Direction::W))
        || (neighbor_is_low_bridge_tube(terrain, rx, ry, Direction::S)
            && neighbor_is_low_bridge_tube(terrain, rx, ry, Direction::N))
}

fn neighbor_is_low_bridge_tube(
    terrain: &ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
    direction: Direction,
) -> bool {
    let (dx, dy) = direction.offset();
    let nx = rx as i32 + dx;
    let ny = ry as i32 + dy;
    if nx < 0 || ny < 0 || nx >= terrain.width() as i32 || ny >= terrain.height() as i32 {
        return false;
    }
    terrain
        .cell(nx as u16, ny as u16)
        .is_some_and(|cell| cell.is_low_bridge_tube_cell())
}

fn ordered_endpoint_key(a: (u16, u16), b: (u16, u16)) -> ((u16, u16), (u16, u16)) {
    if a <= b { (a, b) } else { (b, a) }
}

fn bridge_record_kind_for_group(
    members: &[(u16, u16)],
    terrain: &ResolvedTerrainGrid,
) -> BridgeRecordKind {
    let has_explicit_low_layer = members.iter().any(|&(rx, ry)| {
        terrain.cell(rx, ry).is_some_and(|cell| {
            cell.bridge_layer
                .as_ref()
                .is_some_and(|layer| layer.direction == BridgeDirection::Low)
        })
    });

    if has_explicit_low_layer {
        BridgeRecordKind::Low
    } else {
        BridgeRecordKind::High
    }
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

fn bridge_layer_to_axis(layer: Option<&crate::map::resolved_terrain::BridgeLayer>) -> Option<Axis> {
    layer.map(|bl| bridge_direction_to_axis(bl.direction))
}

fn resolved_cell_has_runtime_deck(
    cell: &crate::map::resolved_terrain::ResolvedTerrainCell,
) -> bool {
    cell.bridge_facts.has_structural_bridge()
        || (cell.has_bridge_deck
            && cell.bridge_facts.family == crate::map::bridge_facts::BridgeStampFamily::None)
}

fn bridge_fact_axis(cell: &crate::map::resolved_terrain::ResolvedTerrainCell) -> Option<Axis> {
    cell.bridge_facts
        .direction
        .map(bridge_stamp_direction_to_axis)
}

fn initial_bridge_damage_state(
    cell: &crate::map::resolved_terrain::ResolvedTerrainCell,
) -> DamageState {
    if cell.bridge_facts.family != crate::map::bridge_facts::BridgeStampFamily::None {
        DamageState::from_state_byte(cell.bridge_facts.state_byte)
            .unwrap_or(DamageState::Healthy { variant: 0 })
    } else {
        DamageState::Healthy { variant: 0 }
    }
}

fn bridge_stamp_direction_to_axis(direction: u8) -> Axis {
    match direction & 7 {
        2 | 6 => Axis::EW,
        _ => Axis::NS,
    }
}

fn bridge_stamp_direction_to_direction(direction: u8) -> Direction {
    match direction & 7 {
        0 => Direction::N,
        1 => Direction::NE,
        2 => Direction::E,
        3 => Direction::SE,
        4 => Direction::S,
        5 => Direction::SW,
        6 => Direction::W,
        _ => Direction::NW,
    }
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
    use crate::map::resolved_terrain::{
        BridgeDirection, BridgeLayer, ResolvedTerrainCell, ResolvedTerrainGrid, YR_CELL_LAND_TUNNEL,
    };
    use crate::map::tube_facts::{TubeFact, TubeId};
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
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: false,
                tileset_index: Some(0),
                land_type: 0,
                yr_cell_land_type: 0,
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
                base_land_type: 0,
                base_yr_cell_land_type: 0,
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: on_bridge,
                has_bridge_deck: on_bridge,
                bridge_walkable: on_bridge,
                bridge_transition: rx == 1 || rx == 3,
                bridge_deck_level: if on_bridge { 4 } else { 0 },
                bridge_layer: None,
                bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                tube_index: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: false,
                bridgehead_anchor_class_at_load: None,
            });
        }
        ResolvedTerrainGrid::from_cells(5, 1, cells)
    }

    fn make_low_bridge_terrain() -> ResolvedTerrainGrid {
        let mut tubes = Vec::new();
        let cells = make_bridge_terrain()
            .iter()
            .cloned()
            .map(|mut cell| {
                if (1..=3).contains(&cell.rx) {
                    cell.bridge_deck_level = cell.level;
                    cell.bridge_layer = Some(BridgeLayer {
                        overlay_id: 0x4a,
                        overlay_name: "LOBRDG01".to_string(),
                        deck_level: cell.level,
                        direction: BridgeDirection::Low,
                    });
                    cell.yr_cell_land_type = YR_CELL_LAND_TUNNEL;
                    let tube_id = TubeId(tubes.len() as u16);
                    tubes.push(TubeFact::auto_low_bridge((cell.rx, cell.ry), 2));
                    cell.tube_index = Some(tube_id);
                }
                cell
            })
            .collect();
        ResolvedTerrainGrid::from_cells_with_tubes(5, 1, cells, tubes)
    }

    /// 5x1 grid: ground(0,0), bridgehead(1,0), body(2,0), bridgehead(3,0),
    /// ground(4,0). Bridgeheads carry realistic resolved-terrain shape:
    /// bridge_walkable=true, has_bridge_deck=false, transition=true,
    /// bridge_deck_level=4. Body at (2,0) has has_bridge_deck=true.
    fn make_bridge_with_bridgeheads_terrain() -> ResolvedTerrainGrid {
        let mut cells = Vec::new();
        for rx in 0..5u16 {
            let is_body = rx == 2;
            let is_head = rx == 1 || rx == 3;
            cells.push(ResolvedTerrainCell {
                rx,
                ry: 0,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: false,
                tileset_index: Some(0),
                land_type: 0,
                yr_cell_land_type: 0,
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
                ground_walk_blocked: is_body,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                base_land_type: 0,
                base_yr_cell_land_type: 0,
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: is_body,
                has_bridge_deck: is_body,
                bridge_walkable: is_body || is_head,
                bridge_transition: is_head,
                bridge_deck_level: if is_body || is_head { 4 } else { 0 },
                bridge_layer: None,
                bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                tube_index: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: false,
                bridgehead_anchor_class_at_load: None,
            });
        }
        ResolvedTerrainGrid::from_cells(5, 1, cells)
    }

    #[test]
    fn bridgeheads_registered_with_bridgehead_role() {
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_with_bridgeheads_terrain(),
            true,
            300,
        );
        for rx in [1u16, 3] {
            let cell = state.cell(rx, 0).expect("bridgehead cell must register");
            assert!(matches!(cell.role, BridgeCellRole::Bridgehead));
            assert!(cell.deck_present, "bridgeheads carry deck_present=true");
            assert!(matches!(
                cell.damage_state,
                DamageState::Healthy { variant: 0 }
            ));
            assert!(cell.bridge_group_id.is_none());
            assert!(cell.anchor_span_id.is_none());
            assert!(cell.axis.is_none());
            assert_eq!(cell.deck_level, 4);
        }
    }

    #[test]
    fn bridgehead_is_bridge_walkable_returns_true() {
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_with_bridgeheads_terrain(),
            true,
            300,
        );
        assert!(state.is_bridge_walkable(1, 0));
        assert!(state.is_bridge_walkable(3, 0));
    }

    #[test]
    fn bridgehead_survives_body_cell_collapse() {
        let mut state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_with_bridgeheads_terrain(),
            true,
            50,
        );
        if let Some(c) = state.cell_mut(2, 0) {
            c.damage_state = DamageState::Destroyed;
        }
        assert!(state.is_bridge_walkable(1, 0));
        assert!(state.is_bridge_walkable(3, 0));
        assert!(matches!(
            state.cell(1, 0).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        ));
        assert!(matches!(
            state.cell(3, 0).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        ));
        assert!(!state.is_bridge_walkable(2, 0));
    }

    #[test]
    fn ns_walker_triple_skips_bridgehead_neighbors() {
        // 3x4 NS bridge: head(2,1), body(2,2), head(2,3). The walker
        // triple-writes (this, north=(2,1), south=(2,3)) which would
        // corrupt the bridgeheads if Task 2's role skip were missing.
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(
            2,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 4,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0xD3,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        for ry in [1u16, 3] {
            state.test_seed_cell(
                2,
                ry,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 4,
                    bridge_group_id: None,
                    damage_state: DamageState::Healthy { variant: 0 },
                    axis: None,
                    role: BridgeCellRole::Bridgehead,
                    anchor_span_id: None,
                    overlay_byte: 0,
                    damaged_variant: false,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
                },
            );
        }
        let terrain =
            crate::map::resolved_terrain::ResolvedTerrainGrid::from_cells(3, 4, Vec::new());

        let _ = state.destroy_bridge_walker_ns_high(2, 2, &terrain);

        for ry in [1u16, 3] {
            let head = state.cell(2, ry).expect("bridgehead survives walker");
            assert_eq!(head.overlay_byte, 0, "bridgehead overlay_byte untouched");
            assert!(matches!(
                head.damage_state,
                DamageState::Healthy { variant: 0 }
            ));
            assert!(matches!(head.role, BridgeCellRole::Bridgehead));
        }
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
    fn marking_group_cells_destroyed_makes_them_unwalkable() {
        // Direct mutation replaces the legacy `apply_damage`. The
        // orchestrator's walker performs the per-cell damage-state
        // transitions through `body_cell_advance_state`; this lower-
        // level test just asserts the read paths (is_bridge_walkable)
        // honor `DamageState::Destroyed`.
        let mut state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 50);
        for (rx, ry) in [(1u16, 0u16), (2, 0), (3, 0)] {
            if let Some(cell) = state.cell_mut(rx, ry) {
                cell.damage_state = DamageState::Destroyed;
            }
        }
        assert!(!state.is_bridge_walkable(1, 0));
        assert!(!state.is_bridge_walkable(2, 0));
        assert!(!state.is_bridge_walkable(3, 0));
        assert_eq!(
            state.cell(1, 0).map(|c| c.damage_state),
            Some(DamageState::Destroyed)
        );
    }

    #[test]
    fn indestructible_bridge_outer_gate_is_clear() {
        // The orchestrator's outer gate is `is_destroyable()`. When a
        // bridge runtime is built with `destroyable=false`, the gate
        // closes and the dispatcher bails before any path fires.
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), false, 50);
        assert!(!state.is_destroyable());
        assert!(state.is_bridge_walkable(1, 0));
    }

    #[test]
    fn bridge_endpoints_detected() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 300);
        let records = state.endpoint_records();
        assert_eq!(
            records.len(),
            1,
            "should have exactly one bridge endpoint record"
        );
        let rec = &records[0];
        assert!(rec.active);
        assert_eq!(rec.group_id, 1);
        assert_eq!(rec.bridge_kind, BridgeRecordKind::High);
        let endpoints = [rec.endpoint_a, rec.endpoint_b];
        assert!(
            endpoints.contains(&(0, 0)),
            "endpoint_a or _b should be (0,0)"
        );
        assert!(
            endpoints.contains(&(4, 0)),
            "endpoint_a or _b should be (4,0)"
        );
    }

    #[test]
    fn bridge_endpoint_records_mark_low_groups_low() {
        let state =
            BridgeRuntimeState::from_resolved_terrain(&make_low_bridge_terrain(), true, 300);
        let records = state.endpoint_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].bridge_kind, BridgeRecordKind::Low);
        assert!(!records[0].is_high());
    }

    #[test]
    fn low_bridge_tube_record_requires_opposite_neighbors() {
        let mut terrain = make_low_bridge_terrain();
        let cell = terrain.cell_mut(3, 0).expect("right low bridge cell");
        cell.tube_index = None;
        cell.yr_cell_land_type = 0;

        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 300);
        assert!(
            state.endpoint_records().is_empty(),
            "low bridge records require the verified opposite low-neighbor pattern"
        );
    }

    #[test]
    fn bridge_destruction_deactivates_endpoints() {
        // Endpoint deactivation is now driven by the orchestrator's
        // `refresh_bridge_zones_if_dirty`, which calls
        // `refresh_endpoint_active_flags` whenever a walker / state-machine
        // collapse marks `zones_dirty`. This in-module test exercises the
        // deactivation logic in isolation: mutate cells to Destroyed (the
        // dispatcher's terminal effect) and call the refresh helper
        // directly. The full pipeline is covered by world-level integration
        // tests in `world_tests.rs`.
        let mut state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 50);
        // Pre-condition: endpoint exists and is active.
        let records = state.endpoint_records();
        assert_eq!(records.len(), 1);
        assert!(records[0].active);
        let group_id = records[0].group_id;

        // Mark every cell of the group destroyed (simulates a final-stage
        // walker cascade landing on the entire group).
        for (rx, ry) in [(1u16, 0u16), (2, 0), (3, 0)] {
            if let Some(c) = state.cell_mut(rx, ry) {
                c.damage_state = DamageState::Destroyed;
            }
        }
        state.refresh_endpoint_active_flags();

        let records = state.endpoint_records();
        assert!(
            !records[0].active,
            "endpoint of destroyed group {group_id} must deactivate"
        );
        assert_eq!(records[0].bridge_kind, BridgeRecordKind::High);
    }

    #[test]
    fn refresh_endpoint_active_flags_deactivates_on_first_destroyed_cell() {
        // Per the new state-machine semantic: a single destroyed cell in a
        // group severs the bridge — the endpoint flips inactive immediately,
        // not just when the entire group is destroyed.
        let mut state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 50);
        assert!(state.endpoint_records()[0].active);

        // Destroy only ONE cell of the 3-cell group.
        if let Some(c) = state.cell_mut(2, 0) {
            c.damage_state = DamageState::Destroyed;
        }
        state.refresh_endpoint_active_flags();

        assert!(
            !state.endpoint_records()[0].active,
            "first destroyed cell must already deactivate the endpoint"
        );
    }

    #[test]
    fn refresh_endpoint_active_flags_leaves_intact_groups_active() {
        // No destroyed cells anywhere — refresh must not flip anything.
        let mut state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 50);
        state.refresh_endpoint_active_flags();
        assert!(state.endpoint_records()[0].active);
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
            Direction::N,
            Direction::NE,
            Direction::E,
            Direction::SE,
            Direction::S,
            Direction::SW,
            Direction::W,
            Direction::NW,
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
    fn stamped_high_bridge_facts_create_anchor_span_without_bridge_layer() {
        use crate::map::bridge_facts::{
            BridgeCellFacts, BridgeStampFamily, stamp_set_bridge_direction,
        };

        let width = 10u16;
        let height = 10u16;
        let mut facts = vec![BridgeCellFacts::default(); width as usize * height as usize];
        stamp_set_bridge_direction(
            &mut facts,
            width,
            height,
            (5, 5),
            BridgeStampFamily::Nesw,
            0,
            true,
        );
        facts[5usize * width as usize + 5].overlay_id = Some(0x18);

        let mut cells = Vec::new();
        for ry in 0..height {
            for rx in 0..width {
                let idx = ry as usize * width as usize + rx as usize;
                let structural = facts[idx].has_structural_bridge();
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    ground_walk_blocked: structural,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: structural,
                    has_bridge_deck: false,
                    bridge_walkable: structural,
                    bridge_transition: facts[idx].has_transition_flag(),
                    bridge_deck_level: if structural { 4 } else { 0 },
                    bridge_layer: None,
                    bridge_facts: facts[idx],
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }

        let terrain = ResolvedTerrainGrid::from_cells(width, height, cells);
        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);

        assert_eq!(state.anchor_spans().len(), 1);
        let span = state.anchor_spans().values().next().expect("span");
        assert_eq!(span.anchor, (5, 5));
        assert_eq!(span.axis, Axis::NS);
        assert_eq!(span.direction, Direction::N);
        assert_eq!(
            span.cells,
            [
                Some((5, 5)),
                Some((5, 4)),
                Some((5, 3)),
                Some((5, 2)),
                Some((5, 6)),
                None,
            ]
        );
        assert_eq!(state.cell(5, 5).expect("anchor").overlay_byte, 0x18);
        assert!(matches!(
            state.cell(5, 5).expect("anchor").role,
            BridgeCellRole::Anchor
        ));
        assert!(state.cell(5, 4).is_some());
        assert!(state.cell(5, 3).is_some());
        assert!(state.cell(5, 6).is_some());
        assert!(
            state.cell(5, 2).is_none(),
            "slot 3 is flag-only and must not create a runtime bridge cell"
        );
    }

    #[test]
    fn bridge_runtime_state_snapshot_round_trip() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 1500);
        let json = serde_json::to_string(&state).expect("serialize");
        let restored: BridgeRuntimeState = serde_json::from_str(&json).expect("deserialize");
        // Compare cell-by-cell across the full grid.
        for (rx, ry) in [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)] {
            assert_eq!(
                state.cell(rx, ry),
                restored.cell(rx, ry),
                "cell ({rx},{ry})"
            );
        }
        // Compare anchor spans.
        assert_eq!(state.anchor_spans().len(), restored.anchor_spans().len());
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
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 1500);
        // Field is reachable on every populated bridge cell; type is u8.
        for (_, cell) in state.iter_cells() {
            let _byte: u8 = cell.overlay_byte;
        }
    }

    #[test]
    fn overlay_byte_round_trips_via_snapshot() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 1500);
        let json = serde_json::to_string(&state).expect("serialize");
        let restored: BridgeRuntimeState = serde_json::from_str(&json).expect("deserialize");
        for ((rx, ry), cell) in state.iter_cells() {
            let r = restored.cell(rx, ry).expect("restored cell present");
            assert_eq!(
                cell.overlay_byte, r.overlay_byte,
                "overlay_byte at ({rx},{ry})"
            );
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
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
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
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        };
        state.test_seed_cell(2, 2, cell);
        state.cell_mut(2, 2).unwrap().overlay_byte = 0xD2;
        assert_eq!(state.cell(2, 2).unwrap().overlay_byte, 0xD2);
    }

    #[test]
    fn damage_state_to_byte_ns_axis() {
        assert_eq!(
            DamageState::Healthy { variant: 0 }.to_state_byte(Axis::NS),
            0
        );
        assert_eq!(
            DamageState::Healthy { variant: 3 }.to_state_byte(Axis::NS),
            3
        );
        assert_eq!(
            DamageState::Healthy { variant: 5 }.to_state_byte(Axis::NS),
            5
        );
        assert_eq!(DamageState::Damaged.to_state_byte(Axis::NS), 6);
        assert_eq!(DamageState::PartialCollapseA.to_state_byte(Axis::NS), 7);
        assert_eq!(DamageState::PartialCollapseB.to_state_byte(Axis::NS), 8);
        assert_eq!(DamageState::Destroyed.to_state_byte(Axis::NS), 0);
    }

    #[test]
    fn damage_state_to_byte_ew_axis() {
        assert_eq!(
            DamageState::Healthy { variant: 0 }.to_state_byte(Axis::EW),
            9
        );
        assert_eq!(
            DamageState::Healthy { variant: 5 }.to_state_byte(Axis::EW),
            14
        );
        assert_eq!(DamageState::Damaged.to_state_byte(Axis::EW), 0xF);
        assert_eq!(DamageState::PartialCollapseA.to_state_byte(Axis::EW), 0x11);
        assert_eq!(DamageState::PartialCollapseB.to_state_byte(Axis::EW), 0x10);
        assert_eq!(DamageState::Destroyed.to_state_byte(Axis::EW), 0);
    }

    #[test]
    fn damage_state_to_byte_clamps_healthy_variant() {
        // Variant > 5 is invalid input; should clamp to 5 (max defined healthy).
        assert_eq!(
            DamageState::Healthy { variant: 7 }.to_state_byte(Axis::NS),
            5
        );
        assert_eq!(
            DamageState::Healthy { variant: 10 }.to_state_byte(Axis::EW),
            14
        );
    }

    #[test]
    fn damage_state_from_byte_ns_range() {
        assert_eq!(
            DamageState::from_state_byte(0),
            Some(DamageState::Healthy { variant: 0 })
        );
        assert_eq!(
            DamageState::from_state_byte(3),
            Some(DamageState::Healthy { variant: 3 })
        );
        assert_eq!(
            DamageState::from_state_byte(5),
            Some(DamageState::Healthy { variant: 5 })
        );
        assert_eq!(DamageState::from_state_byte(6), Some(DamageState::Damaged));
        assert_eq!(
            DamageState::from_state_byte(7),
            Some(DamageState::PartialCollapseA)
        );
        assert_eq!(
            DamageState::from_state_byte(8),
            Some(DamageState::PartialCollapseB)
        );
    }

    #[test]
    fn damage_state_from_byte_ew_range() {
        assert_eq!(
            DamageState::from_state_byte(9),
            Some(DamageState::Healthy { variant: 0 })
        );
        assert_eq!(
            DamageState::from_state_byte(14),
            Some(DamageState::Healthy { variant: 5 })
        );
        assert_eq!(
            DamageState::from_state_byte(0xF),
            Some(DamageState::Damaged)
        );
        assert_eq!(
            DamageState::from_state_byte(0x10),
            Some(DamageState::PartialCollapseB)
        );
        assert_eq!(
            DamageState::from_state_byte(0x11),
            Some(DamageState::PartialCollapseA)
        );
    }

    #[test]
    fn damage_state_from_byte_out_of_range_returns_none() {
        assert_eq!(DamageState::from_state_byte(0x12), None);
        assert_eq!(DamageState::from_state_byte(0xFF), None);
    }

    #[test]
    fn render_state_byte_strips_healthy_variant() {
        assert_eq!(
            DamageState::Healthy { variant: 0 }.render_state_byte(Axis::NS),
            0
        );
        assert_eq!(
            DamageState::Healthy { variant: 5 }.render_state_byte(Axis::NS),
            0
        );
        assert_eq!(
            DamageState::Healthy { variant: 0 }.render_state_byte(Axis::EW),
            9
        );
        assert_eq!(
            DamageState::Healthy { variant: 5 }.render_state_byte(Axis::EW),
            9
        );
        assert_eq!(DamageState::Damaged.render_state_byte(Axis::NS), 6);
        assert_eq!(DamageState::Damaged.render_state_byte(Axis::EW), 0xF);
        assert_eq!(DamageState::Destroyed.render_state_byte(Axis::NS), 0);
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
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
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
            5,
            4,
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
                Some((5, 5)),
                Some((6, 5)),
                Some((7, 5)),
                Some((8, 5)),
                Some((4, 5)),
                None,
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
        let outcome = state.body_cell_advance_state(5, 5, true, &flood_fill_terrain(20, 20, 0));
        assert!(matches!(outcome, StateOutcome::Absorbed));
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Damaged);
    }

    #[test]
    fn body_driver_non_anchor_body_cell_follows_to_anchor() {
        let mut state = make_body_driver_test_state();
        // Damage on a body cell, not the anchor.
        let outcome = state.body_cell_advance_state(5, 4, true, &flood_fill_terrain(20, 20, 0));
        assert!(matches!(outcome, StateOutcome::Absorbed));
        // Anchor's damage_state advanced, not the input body cell's.
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Damaged);
        assert_eq!(
            state.cell(5, 4).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        );
    }

    #[test]
    fn body_driver_damaged_anchor_collapses_and_emits_set_bridge_direction() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::Damaged;
        let outcome = state.body_cell_advance_state(5, 5, true, &flood_fill_terrain(20, 20, 0));
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                set_bridge_direction,
                adjacent_bridges_dirty,
                zones_dirty,
            } => {
                assert!(destroyed_cells.contains(&(5, 5)));
                // 4 BlowUpBridge actions per Task 12 invariant.
                let blow_ups = set_bridge_direction
                    .actions
                    .iter()
                    .filter(|(_, _, a)| {
                        matches!(a, crate::sim::bridge_specs::CellAction::BlowUpBridge)
                    })
                    .count();
                assert_eq!(blow_ups, 4);
                // 2 perpendicular cells flagged dirty (E and W of (5,5)).
                assert_eq!(adjacent_bridges_dirty.len(), 2);
                assert!(zones_dirty);
            }
            other => panic!("expected Collapsed, got {other:?}"),
        }
        assert_eq!(
            state.cell(5, 5).unwrap().damage_state,
            DamageState::Destroyed
        );
    }

    #[test]
    fn body_driver_partial_collapse_a_collapses_with_single_ramp_call() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::PartialCollapseA;
        let outcome = state.body_cell_advance_state(5, 5, true, &flood_fill_terrain(20, 20, 0));
        assert!(matches!(outcome, StateOutcome::Collapsed { .. }));
        assert_eq!(
            state.cell(5, 5).unwrap().damage_state,
            DamageState::Destroyed
        );
    }

    #[test]
    fn body_driver_partial_collapse_b_collapses_with_single_ramp_call() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::PartialCollapseB;
        let outcome = state.body_cell_advance_state(5, 5, true, &flood_fill_terrain(20, 20, 0));
        assert!(matches!(outcome, StateOutcome::Collapsed { .. }));
        assert_eq!(
            state.cell(5, 5).unwrap().damage_state,
            DamageState::Destroyed
        );
    }

    #[test]
    fn body_driver_destroyed_anchor_returns_no_change() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::Destroyed;
        let outcome = state.body_cell_advance_state(5, 5, true, &flood_fill_terrain(20, 20, 0));
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    #[test]
    fn body_driver_bridgehead_cell_returns_no_change() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().role = BridgeCellRole::Bridgehead;
        let outcome = state.body_cell_advance_state(5, 5, true, &flood_fill_terrain(20, 20, 0));
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    #[test]
    fn body_driver_out_of_bounds_returns_no_change() {
        let mut state = make_body_driver_test_state();
        let outcome = state.body_cell_advance_state(99, 99, true, &flood_fill_terrain(20, 20, 0));
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    /// 5x5 grid; column X=2 carries the NS bridgehead walk:
    /// (2,4)=8 (bridgehead high-ramp peak), (2,3)=6, (2,2)=4 (anchor body),
    /// (2,1)=0, (2,0)=0. Walk N from (2,4) terminates at (2,2).
    fn make_bridgehead_terrain_ns() -> crate::map::resolved_terrain::ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(25);
        for ry in 0..5u16 {
            for rx in 0..5u16 {
                let template_height: u8 = if rx == 2 {
                    match ry {
                        4 => 8,
                        3 => 6,
                        2 => 4,
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
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(5, 5, cells)
    }

    /// Bridgehead at (2,4) NS, anchor at (2,2) NS, perpendicular partner
    /// anchors at (1,2) west and (3,2) east. All cells start `Healthy{0}`.
    /// Walk N from (2,4) h=8 → (2,3) h=6 → (2,2) h=4 (anchor).
    fn make_bridgehead_state_ns() -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        // Bridgehead at (2, 4).
        state.test_seed_cell(
            2,
            4,
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
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        // Anchor at (2, 2).
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
                role: BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0x20,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        // DamageA neighbor (east of anchor) at (3, 2).
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
                overlay_byte: 0x21,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        // DamageB neighbor (west of anchor) at (1, 2).
        state.test_seed_cell(
            1,
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
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        // Sentinel to grow state to 5x5 (matches terrain dimensions).
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
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        state
    }

    #[test]
    fn bridgehead_advance_first_hit_writes_anchor_damaged() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        let pre_hit_bridgehead = *state.cell(2, 4).unwrap();
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::Variant0
        );

        let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
        assert_eq!(outcome, StateOutcome::Absorbed);

        // Bridgehead's own damage_state is NOT modified.
        let post_bridgehead = *state.cell(2, 4).unwrap();
        assert_eq!(
            post_bridgehead.damage_state,
            pre_hit_bridgehead.damage_state
        );

        // Anchor's bridgehead_anchor_class becomes AboutToFall (4th slot —
        // first-hit writes the most-damaged variant directly, skipping
        // intermediate slots).
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::AboutToFall
        );

        // East perpendicular partner (DamageA) — state byte 0 → 4 → Healthy{4}.
        assert_eq!(
            state.cell(3, 2).unwrap().damage_state,
            DamageState::Healthy { variant: 4 }
        );
        // West perpendicular partner (DamageB) — state byte 0 → 5 → Healthy{5}.
        assert_eq!(
            state.cell(1, 2).unwrap().damage_state,
            DamageState::Healthy { variant: 5 }
        );
    }

    #[test]
    fn bridgehead_advance_repeat_hits_stay_damaged_no_collapse() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        for _ in 0..100 {
            let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
            assert_eq!(
                outcome,
                StateOutcome::Absorbed,
                "every hit must return Absorbed, never Collapsed",
            );
        }
        // Anchor's tile class stays AboutToFall (idempotent across hits).
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::AboutToFall
        );
        // Bridgehead cell's own damage_state never changes.
        assert!(matches!(
            state.cell(2, 4).unwrap().damage_state,
            DamageState::Healthy { .. }
        ));
    }

    #[test]
    fn bridgehead_advance_odd_h_ns_absorbs_with_no_change() {
        // Bridgehead at h=5 (odd NS ramp): parity gate fires.
        let mut state = make_bridgehead_state_ns();
        let mut terrain = make_bridgehead_terrain_ns();
        // Override (2, 4) height to 5 — odd, parity-gated.
        if let Some(cell) = terrain.cells.get_mut(4 * 5 + 2) {
            cell.template_height = 5;
        }
        let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
        // Anchor's tile class unchanged.
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::Variant0
        );
    }

    #[test]
    fn bridgehead_advance_h_gt_4_ew_absorbs_with_no_change() {
        // Bridgehead at h=0xC (EW high-ramp peak): upper-bound gate fires.
        // Use a fresh setup since the shared fixture is NS-axis.
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
                axis: Some(Axis::EW),
                role: BridgeCellRole::Bridgehead,
                anchor_span_id: None,
                overlay_byte: 0x18,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        // 3x3 terrain with cell (2,2) h=0xC.
        let mut cells = Vec::with_capacity(9);
        for ry in 0..3u16 {
            for rx in 0..3u16 {
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
                    slope_type: 0,
                    template_height: if rx == 2 && ry == 2 { 0x0C } else { 0 },
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
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        let terrain = ResolvedTerrainGrid::from_cells(3, 3, cells);
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }

    #[test]
    fn bridgehead_advance_walks_through_odd_intermediate() {
        // Mid-walk parity tolerance: walk passes through an odd h=5
        // intermediate between h=8 start and h=4 anchor.
        let mut state = make_bridgehead_state_ns();
        let mut terrain = make_bridgehead_terrain_ns();
        // Patch the walk path: (2,4)=8, (2,3)=5 (odd!), (2,2)=4.
        if let Some(c) = terrain.cells.get_mut(3 * 5 + 2) {
            c.template_height = 5;
        }
        let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
        assert_eq!(outcome, StateOutcome::Absorbed);
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::AboutToFall,
            "walk must pass through odd-h intermediate and damage the anchor",
        );
    }

    #[test]
    fn bridgehead_advance_non_bridgehead_role_no_change() {
        let mut state = make_bridgehead_state_ns();
        state.cell_mut(2, 4).unwrap().role = BridgeCellRole::Body;
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }

    #[test]
    fn bridgehead_advance_anchor_walk_failure_no_change() {
        // All heights = 10: start cell is even (passes parity gate) but walk
        // never converges to h=4 within the 16-iter cap (heights stay 10
        // along the column / walking off-map).
        let mut state = make_bridgehead_state_ns();
        let mut terrain = make_bridgehead_terrain_ns();
        for c in terrain.cells.iter_mut() {
            c.template_height = 10;
        }
        let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
        // Anchor's tile class unchanged.
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::Variant0
        );
    }

    #[test]
    fn bridgehead_advance_off_map_no_change() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(99, 99, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }

    /// Build a 1x1 ResolvedTerrainGrid with a single cell at (rx, ry) and
    /// the given `level`. Used by `path_matches_cell` Z-gate tests.
    fn make_terrain_at_level(rx: u16, ry: u16, level: u8) -> ResolvedTerrainGrid {
        let w = rx + 1;
        let h = ry + 1;
        let mut cells = Vec::with_capacity(w as usize * h as usize);
        for cy in 0..h {
            for cx in 0..w {
                cells.push(ResolvedTerrainCell {
                    rx: cx,
                    ry: cy,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: if cx == rx && cy == ry { level } else { 0 },
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    ground_walk_blocked: false,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(w, h, cells)
    }

    fn dispatch_test_ctx(damage: u16, impact_z: i32) -> BridgeDamageContext {
        BridgeDamageContext {
            damage,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: false,
            bridge_strength: 1500,
            impact_z,
        }
    }

    #[test]
    fn path_matches_high_direct_for_raw_body_overlay() {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(
            2,
            0,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::EW),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0xD0,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        let terrain = make_terrain_at_level(2, 0, 5);
        let ctx = dispatch_test_ctx(100, 5);
        assert!(state.path_matches_cell(DispatchPath::HighDirect, 2, 0, &ctx, &terrain));
        assert!(
            !state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx, &terrain),
            "raw-overlay cell must NOT match HighStateMachine"
        );
    }

    #[test]
    fn path_matches_low_direct_for_raw_low_overlay() {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(
            2,
            0,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 2,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0x4F,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        let terrain = make_terrain_at_level(2, 0, 2);
        let ctx = dispatch_test_ctx(100, 2);
        assert!(state.path_matches_cell(DispatchPath::LowDirect, 2, 0, &ctx, &terrain));
        assert!(
            !state.path_matches_cell(DispatchPath::LowStateMachine, 2, 0, &ctx, &terrain),
            "raw-overlay cell must NOT match LowStateMachine"
        );
        assert!(
            !state.path_matches_cell(DispatchPath::HighDirect, 2, 0, &ctx, &terrain),
            "low overlay must not match HighDirect range"
        );
    }

    #[test]
    fn path_matches_high_sm_z_gate_includes_window_excludes_outside() {
        let mut state = BridgeRuntimeState::default();
        // Anchor cell with overlay transitioned out of body range so the
        // state-machine path is reachable.
        state.test_seed_cell(
            2,
            0,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Damaged,
                axis: Some(Axis::EW),
                role: BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0x6,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        let terrain = make_terrain_at_level(2, 0, 5);
        // impact_z=8 is +3 above level → outside window
        let ctx_far = dispatch_test_ctx(100, 8);
        assert!(!state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx_far, &terrain));
        // impact_z=5 is at level → passes
        let ctx_at = dispatch_test_ctx(100, 5);
        assert!(state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx_at, &terrain));
        // impact_z=6 is +1 → boundary inclusive
        let ctx_plus = dispatch_test_ctx(100, 6);
        assert!(state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx_plus, &terrain));
        // impact_z=4 is -1 → boundary inclusive
        let ctx_minus = dispatch_test_ctx(100, 4);
        assert!(state.path_matches_cell(
            DispatchPath::HighStateMachine,
            2,
            0,
            &ctx_minus,
            &terrain
        ));
        // impact_z=3 is -2 → outside window
        let ctx_below = dispatch_test_ctx(100, 3);
        assert!(!state.path_matches_cell(
            DispatchPath::HighStateMachine,
            2,
            0,
            &ctx_below,
            &terrain
        ));
    }

    #[test]
    fn path_matches_low_sm_excludes_high_deck() {
        let mut state = BridgeRuntimeState::default();
        // Cell is "high" (deck_level >= 4) — LowStateMachine must reject it
        // even though overlay/role/Z all otherwise match.
        state.test_seed_cell(
            2,
            0,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 5,
                bridge_group_id: Some(1),
                damage_state: DamageState::Damaged,
                axis: Some(Axis::NS),
                role: BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0x6,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        let terrain = make_terrain_at_level(2, 0, 5);
        let ctx = dispatch_test_ctx(100, 5);
        assert!(!state.path_matches_cell(DispatchPath::LowStateMachine, 2, 0, &ctx, &terrain));
        assert!(state.path_matches_cell(DispatchPath::HighStateMachine, 2, 0, &ctx, &terrain));
    }

    #[test]
    fn path_matches_returns_false_for_missing_cell() {
        let state = BridgeRuntimeState::default();
        let terrain = make_terrain_at_level(0, 0, 0);
        let ctx = dispatch_test_ctx(100, 0);
        for path in [
            DispatchPath::HighStateMachine,
            DispatchPath::LowStateMachine,
            DispatchPath::HighDirect,
            DispatchPath::LowDirect,
        ] {
            assert!(!state.path_matches_cell(path, 5, 5, &ctx, &terrain));
        }
    }

    #[test]
    fn dispatch_path_is_state_machine() {
        assert!(DispatchPath::HighStateMachine.is_state_machine());
        assert!(DispatchPath::LowStateMachine.is_state_machine());
        assert!(!DispatchPath::HighDirect.is_state_machine());
        assert!(!DispatchPath::LowDirect.is_state_machine());
    }

    #[test]
    fn bridge_state_getters_return_construction_values() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 1500);
        assert!(state.is_destroyable());
        assert_eq!(state.bridge_strength(), 1500);
        assert!(state.width() >= 5);
        assert!(state.height() >= 1);
    }

    #[test]
    fn bridge_state_destroyable_flag_disabled() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), false, 800);
        assert!(!state.is_destroyable());
        assert_eq!(state.bridge_strength(), 800);
    }

    // ---- G4 damaged-variant flood-fill tests ------------------------------------

    /// Build a flat `width × height` `ResolvedTerrainGrid` where every cell
    /// shares `final_tile_index = tile_id`, `has_damaged_data = true`, and
    /// all other fields are zero/default. Suitable for flood-fill unit tests
    /// that only care about tile_id equality + has_damaged_data gating.
    fn flood_fill_terrain(width: u16, height: u16, tile_id: i32) -> ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(width as usize * height as usize);
        for ry in 0..height {
            for rx in 0..width {
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: tile_id,
                    source_sub_tile: 0,
                    final_tile_index: tile_id,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    ground_walk_blocked: false,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: true,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    /// Build a `BridgeRuntimeState` with healthy body cells at the given coords.
    fn flood_fill_bridge_state(coords: &[(u16, u16)]) -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        for &(rx, ry) in coords {
            state.test_seed_cell(
                rx,
                ry,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 0,
                    bridge_group_id: Some(1),
                    damage_state: DamageState::Healthy { variant: 0 },
                    axis: Some(Axis::NS),
                    role: BridgeCellRole::Body,
                    anchor_span_id: Some(1),
                    overlay_byte: 0,
                    damaged_variant: false,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
                },
            );
        }
        state
    }

    #[test]
    fn flood_fill_kickoff_skips_when_no_damaged_data() {
        let mut bs = flood_fill_bridge_state(&[(5, 5), (5, 6)]);
        let mut terrain = flood_fill_terrain(10, 10, 42);
        if let Some(c) = terrain.cell_mut(5, 5) {
            c.has_damaged_data = false;
        }
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 0);
        assert!(!bs.cell(5, 5).unwrap().damaged_variant);
        assert!(!bs.cell(5, 6).unwrap().damaged_variant);
    }

    #[test]
    fn flood_fill_propagates_to_same_tile_id_neighbors() {
        let coords = [
            (4, 4),
            (5, 4),
            (6, 4),
            (4, 5),
            (5, 5),
            (6, 5),
            (4, 6),
            (5, 6),
            (6, 6),
        ];
        let mut bs = flood_fill_bridge_state(&coords);
        let terrain = flood_fill_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 9);
        for &(rx, ry) in &coords {
            assert!(
                bs.cell(rx, ry).unwrap().damaged_variant,
                "cell ({},{}) should be damaged",
                rx,
                ry
            );
        }
    }

    #[test]
    fn flood_fill_stops_at_different_tile_id_boundary() {
        let mut bs = flood_fill_bridge_state(&[(5, 5), (5, 6), (5, 7)]);
        let mut terrain = flood_fill_terrain(10, 10, 42);
        if let Some(c) = terrain.cell_mut(5, 6) {
            c.final_tile_index = 99;
        }
        let _ = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert!(bs.cell(5, 5).unwrap().damaged_variant);
        assert!(
            !bs.cell(5, 6).unwrap().damaged_variant,
            "boundary cell stays pristine"
        );
        assert!(
            !bs.cell(5, 7).unwrap().damaged_variant,
            "downstream cell stays pristine"
        );
    }

    #[test]
    fn flood_fill_idempotent_when_already_in_target_state() {
        let mut bs = flood_fill_bridge_state(&[(5, 5)]);
        if let Some(c) = bs.cell_mut(5, 5) {
            c.damaged_variant = true;
        }
        let terrain = flood_fill_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 0, "no mutation when already in target state");
    }

    #[test]
    fn flood_fill_eight_directions_includes_diagonals() {
        let coords = [
            (4, 4),
            (5, 4),
            (6, 4),
            (4, 5),
            (5, 5),
            (6, 5),
            (4, 6),
            (5, 6),
            (6, 6),
        ];
        let mut bs = flood_fill_bridge_state(&coords);
        let terrain = flood_fill_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 9);
        assert!(bs.cell(4, 4).unwrap().damaged_variant, "NW diagonal hit");
        assert!(bs.cell(6, 4).unwrap().damaged_variant, "NE diagonal hit");
        assert!(bs.cell(4, 6).unwrap().damaged_variant, "SW diagonal hit");
        assert!(bs.cell(6, 6).unwrap().damaged_variant, "SE diagonal hit");
    }

    #[test]
    fn flood_fill_clear_propagates_state_false() {
        let coords = [(5u16, 5u16), (5, 6), (5, 7)];
        let mut bs = flood_fill_bridge_state(&coords);
        for &(rx, ry) in &coords {
            if let Some(c) = bs.cell_mut(rx, ry) {
                c.damaged_variant = true;
            }
        }
        let terrain = flood_fill_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, false, &terrain);
        assert_eq!(count, 3);
        for &(rx, ry) in &coords {
            assert!(!bs.cell(rx, ry).unwrap().damaged_variant);
        }
    }

    #[test]
    fn flood_fill_off_map_returns_zero() {
        let mut bs = flood_fill_bridge_state(&[(5, 5)]);
        let terrain = flood_fill_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(99, 99, true, &terrain);
        assert_eq!(count, 0);
    }

    #[test]
    fn flood_fill_sentinel_tile_id_returns_zero() {
        let mut bs = flood_fill_bridge_state(&[(5, 5)]);
        let mut terrain = flood_fill_terrain(10, 10, 42);
        if let Some(c) = terrain.cell_mut(5, 5) {
            c.final_tile_index = 0xFFFF;
        }
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 0);
    }

    /// Synthetic 3x3 grid with a single bridge anchor cell at (1,1).
    /// `pre_class` is written to that cell's bridgehead_anchor_class_at_load.
    fn make_pre_class_terrain(pre_class: Option<BridgeheadAnchorClass>) -> ResolvedTerrainGrid {
        use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
        let mut cells = Vec::with_capacity(9);
        for ry in 0..3u16 {
            for rx in 0..3u16 {
                let is_anchor = rx == 1 && ry == 1;
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: None,
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    ground_walk_blocked: false,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: false,
                    has_bridge_deck: is_anchor,
                    bridge_walkable: is_anchor,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: if is_anchor { pre_class } else { None },
                });
            }
        }
        ResolvedTerrainGrid::from_cells(3, 3, cells)
    }

    #[test]
    fn from_resolved_terrain_copies_pre_damaged_anchor_class() {
        let terrain = make_pre_class_terrain(Some(BridgeheadAnchorClass::AboutToFall));
        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);
        let cell = state.cell(1, 1).expect("bridge cell exists");
        assert_eq!(
            cell.bridgehead_anchor_class,
            BridgeheadAnchorClass::AboutToFall
        );
    }

    #[test]
    fn from_resolved_terrain_defaults_to_variant0_when_pre_class_is_none() {
        let terrain = make_pre_class_terrain(None);
        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);
        let cell = state.cell(1, 1).expect("bridge cell exists");
        assert_eq!(
            cell.bridgehead_anchor_class,
            BridgeheadAnchorClass::Variant0
        );
    }
}

#[cfg(test)]
mod scan_tests {
    use super::cells_in_5x5_scan;

    #[test]
    fn cells_in_5x5_scan_interior_yields_25_cells() {
        let cells: Vec<(u16, u16)> = cells_in_5x5_scan((10, 10)).collect();
        assert_eq!(cells.len(), 25);
        assert!(cells.contains(&(8, 8)));
        assert!(cells.contains(&(12, 12)));
        assert!(cells.contains(&(10, 10)));
    }

    #[test]
    fn cells_in_5x5_scan_at_origin_clamps_negative() {
        let cells: Vec<(u16, u16)> = cells_in_5x5_scan((0, 0)).collect();
        // Only (0..=2, 0..=2) range — 3×3 = 9 cells.
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(0, 0)));
        assert!(cells.contains(&(2, 2)));
        assert!(!cells.iter().any(|(x, _)| *x > 2));
    }

    #[test]
    fn cells_in_5x5_scan_at_edge_clamps_one_side() {
        let cells: Vec<(u16, u16)> = cells_in_5x5_scan((1, 5)).collect();
        // X range: [0..=3] = 4. Y range: [3..=7] = 5. Total = 20.
        assert_eq!(cells.len(), 20);
    }
}

#[cfg(test)]
mod repair_tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::rng::SimRng;

    fn seeded_rng() -> SimRng {
        SimRng::new(0x4242_4242_4242_4242)
    }

    /// Minimal flat 20x20 terrain grid: all cells share tile_id=0,
    /// has_damaged_data=false. Sufficient context for repair tests — the
    /// flood-fill writer's gate fails on has_damaged_data=false, so repair
    /// flood-fill calls become no-ops (terrain is required by signature only).
    fn repair_test_terrain() -> ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(20 * 20);
        for ry in 0..20u16 {
            for rx in 0..20u16 {
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    ground_walk_blocked: false,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(20, 20, cells)
    }

    /// Build a 5-cell test span along Y (NS bridge) with all cells seeded to
    /// `state`. Anchor at (10,10); body cells at (10,11), (10,12), (10,13);
    /// "−direction" cell at (10,9). Slot 5 = None.
    fn build_single_ns_span(state: DamageState) -> BridgeRuntimeState {
        let mut bs = BridgeRuntimeState::default();
        let span = AnchorSpan {
            id: 1,
            anchor: (10, 10),
            cells: [
                Some((10, 10)),
                Some((10, 11)),
                Some((10, 12)),
                Some((10, 13)),
                Some((10, 9)),
                None,
            ],
            axis: Axis::NS,
            direction: Direction::S,
            damage_state: state,
            bridge_group_id: 1,
        };
        bs.test_seed_anchor_span(span);

        for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
            let role = if (rx, ry) == (10, 10) {
                BridgeCellRole::Anchor
            } else {
                BridgeCellRole::Body
            };
            bs.test_seed_cell(
                rx,
                ry,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 0,
                    bridge_group_id: Some(1),
                    damage_state: state,
                    axis: Some(Axis::NS),
                    role,
                    anchor_span_id: Some(1),
                    overlay_byte: 0,
                    damaged_variant: false,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
                },
            );
        }
        bs
    }

    #[test]
    fn repair_destroyed_main_deck_sets_zones_dirty_and_radar() {
        let mut bs = build_single_ns_span(DamageState::Destroyed);
        let mut rng = seeded_rng();
        let scan = vec![(10, 10)];
        let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
        assert!(outcome.zones_dirty, "main-deck repair must set zones_dirty");
        assert_eq!(outcome.radar_cells.len(), 5);
        assert_eq!(outcome.repaired_cells, 5);
        for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
            let s = bs.cell(rx, ry).unwrap().damage_state;
            assert!(
                matches!(s, DamageState::Healthy { .. }),
                "cell ({rx},{ry}) state = {s:?}"
            );
        }
    }

    #[test]
    fn repair_damaged_main_deck_zones_dirty_but_no_radar() {
        let mut bs = build_single_ns_span(DamageState::Damaged);
        let mut rng = seeded_rng();
        let scan = vec![(10, 10)];
        let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
        assert!(outcome.zones_dirty);
        assert!(
            outcome.radar_cells.is_empty(),
            "Damaged → Healthy does NOT mark radar dirty"
        );
        assert_eq!(outcome.repaired_cells, 5);
    }

    #[test]
    fn repair_bridgehead_no_rng_no_zones_no_radar() {
        let mut bs = build_single_ns_span(DamageState::Damaged);
        for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
            bs.cell_mut(rx, ry).unwrap().role = BridgeCellRole::Bridgehead;
        }
        let mut rng = seeded_rng();
        let rng_state_before = rng.state();
        let scan = vec![(10, 10)];
        let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
        assert!(
            !outcome.zones_dirty,
            "bridgehead-only repair must NOT set zones_dirty"
        );
        assert!(outcome.radar_cells.is_empty());
        assert_eq!(outcome.repaired_cells, 5);
        assert_eq!(
            rng.state(),
            rng_state_before,
            "bridgehead repair must not draw RNG"
        );
        for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
            assert!(matches!(
                bs.cell(rx, ry).unwrap().damage_state,
                DamageState::Healthy { variant: 0 }
            ));
        }
    }

    #[test]
    fn repair_healthy_cell_is_noop() {
        let mut bs = build_single_ns_span(DamageState::Healthy { variant: 3 });
        let mut rng = seeded_rng();
        let rng_state_before = rng.state();
        let scan = vec![(10, 10)];
        let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
        assert!(!outcome.zones_dirty);
        assert!(outcome.radar_cells.is_empty());
        assert_eq!(outcome.repaired_cells, 0);
        assert_eq!(
            rng.state(),
            rng_state_before,
            "healthy cells must not draw RNG"
        );
        assert!(matches!(
            bs.cell(10, 10).unwrap().damage_state,
            DamageState::Healthy { variant: 3 }
        ));
    }

    #[test]
    fn repair_partial_collapse_to_healthy() {
        let mut bs = build_single_ns_span(DamageState::PartialCollapseA);
        let mut rng = seeded_rng();
        let scan = vec![(10, 10)];
        let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
        assert!(outcome.zones_dirty);
        assert!(
            outcome.radar_cells.is_empty(),
            "PartialCollapse → Healthy does NOT mark radar dirty"
        );
        assert_eq!(outcome.repaired_cells, 5);
    }

    #[test]
    fn repair_no_bridge_in_scan_empty_outcome() {
        let mut bs = BridgeRuntimeState::default();
        let mut rng = seeded_rng();
        let rng_state_before = rng.state();
        let scan: Vec<(u16, u16)> = (0..25).map(|i| (i, 0)).collect();
        let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
        assert!(!outcome.zones_dirty);
        assert!(outcome.radar_cells.is_empty());
        assert_eq!(outcome.repaired_cells, 0);
        assert_eq!(rng.state(), rng_state_before);
    }

    #[test]
    fn repair_determinism_same_seed_same_variants() {
        let mut bs_a = build_single_ns_span(DamageState::Destroyed);
        let mut bs_b = build_single_ns_span(DamageState::Destroyed);
        let mut rng_a = seeded_rng();
        let mut rng_b = seeded_rng();
        let scan = vec![(10, 10)];
        bs_a.body_cell_repair_state(&scan, &mut rng_a, &repair_test_terrain());
        bs_b.body_cell_repair_state(&scan, &mut rng_b, &repair_test_terrain());
        for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
            let va = bs_a.cell(rx, ry).unwrap().damage_state;
            let vb = bs_b.cell(rx, ry).unwrap().damage_state;
            assert_eq!(va, vb, "variant divergence at ({rx},{ry})");
        }
    }

    /// Re-derive the pinned variants by replaying the exact RNG draw
    /// sequence: 5 cells, all main-deck damaged → 5 sequential
    /// `next_range_u32(4)` calls (matches `body_cell_repair_state`'s draw).
    fn compute_pinned_variants(rng: &mut SimRng) -> Vec<u8> {
        (0..5).map(|_| rng.next_range_u32(4) as u8).collect()
    }

    #[test]
    fn repair_strip_iteration_order_pin() {
        // Locks the RNG-draw sequence for a known 5-cell destroyed span.
        // If anyone reorders `AnchorSpan.cells` or changes the iteration
        // pattern, this test fails with diff-friendly output.
        let mut bs = build_single_ns_span(DamageState::Destroyed);
        let mut rng = seeded_rng();
        let scan = vec![(10, 10)];
        bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());

        // Variants captured in slot order from the span definition:
        //   slot 0 = (10,10), slot 1 = (10,11), slot 2 = (10,12),
        //   slot 3 = (10,13), slot 4 = (10,9), slot 5 = None.
        let variants: Vec<u8> = [(10, 10), (10, 11), (10, 12), (10, 13), (10, 9)]
            .iter()
            .map(|&(rx, ry)| match bs.cell(rx, ry).unwrap().damage_state {
                DamageState::Healthy { variant } => variant,
                other => panic!("non-Healthy after repair: {other:?}"),
            })
            .collect();

        let expected = compute_pinned_variants(&mut seeded_rng());
        assert_eq!(
            variants, expected,
            "RNG-draw iteration order changed — verify span.cells slot order"
        );

        for v in &variants {
            assert!(
                *v <= 3,
                "repair walker wrote variant {v} — must be 0..=3 (healthy)"
            );
        }
    }

    #[test]
    fn repair_two_overlapping_spans_processed_in_btreeset_order() {
        // Span 1 already present from build_single_ns_span. Span 2 at
        // (9..=13, 11) with anchor (10,11). They share cell (10,11): span 1's
        // body cell + span 2's anchor.
        let mut bs = build_single_ns_span(DamageState::Destroyed);
        let span2 = AnchorSpan {
            id: 2,
            anchor: (10, 11),
            cells: [
                Some((10, 11)),
                Some((11, 11)),
                Some((12, 11)),
                Some((13, 11)),
                Some((9, 11)),
                None,
            ],
            axis: Axis::NS,
            direction: Direction::S,
            damage_state: DamageState::Destroyed,
            bridge_group_id: 1,
        };
        bs.test_seed_anchor_span(span2);
        for &(rx, ry) in &[(9, 11), (11, 11), (12, 11), (13, 11)] {
            bs.test_seed_cell(
                rx,
                ry,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 0,
                    bridge_group_id: Some(1),
                    damage_state: DamageState::Destroyed,
                    axis: Some(Axis::NS),
                    role: BridgeCellRole::Body,
                    anchor_span_id: Some(2),
                    overlay_byte: 0,
                    damaged_variant: false,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
                },
            );
        }
        let mut rng = seeded_rng();
        let scan: Vec<(u16, u16)> = cells_in_5x5_scan((10, 10)).collect();
        let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
        assert!(outcome.zones_dirty);
        // Span 1: 5 cells; Span 2: 5 cells minus the shared (10,11) cell
        // (already Healthy after span 1's pass) = 4. Total 9.
        assert_eq!(
            outcome.repaired_cells, 9,
            "overlap cell (10,11) repaired once by span 1, skipped by span 2"
        );
    }
}
