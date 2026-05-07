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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BridgeDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BridgeStateChange {
    pub destroyed_cells: Vec<(u16, u16)>,
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
    /// bridgehead cells (which use `bridgehead_step` instead).
    pub anchor_span_id: Option<u16>,

    /// Bridgehead 4-step progression counter (0..=3). Only meaningful when
    /// `role == BridgeCellRole::Bridgehead`.
    pub bridgehead_step: u8,
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
                    bridgehead_step: 0,
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
                c.bridgehead_step = 0;
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
}
