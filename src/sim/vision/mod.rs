//! Deterministic fog/shroud visibility state computed from unit vision radii.
//!
//! Each cell has two independent flags: "revealed" (seen at least once) and
//! "visible" (currently in line of sight). State is stored in a flat Vec<u8>
//! grid per owner for O(1) lookup.
//!
//! ## Performance
//! Alliance-aware queries (`is_cell_visible`, edge masks) use a pre-merged
//! visibility grid so each cell lookup is O(1) instead of iterating all owners.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::map::houses::{are_houses_friendly, HouseAllianceMap};
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::pathfinding::PathGrid;

/// Bit flag: cell has been seen at least once (persists across ticks).
const FLAG_REVEALED: u8 = 0x01;
/// Bit flag: cell is currently in line of sight (rebuilt each tick).
const FLAG_VISIBLE: u8 = 0x02;
/// Bit flag: cell is covered by an enemy gap generator (rebuilt each tick).
/// Entities on gap-covered cells are hidden from the local player.
const FLAG_GAP_COVERED: u8 = 0x04;

/// RA2 hard-caps effective sight at 10 cells. Going past 10 was a crash
/// in the original engine — we clamp to this limit for compatibility.
pub const MAX_SIGHT_RANGE: u16 = 10;

const LEPTONS_PER_CELL_I32: i32 = 256;
const REVEAL_AREA2_Z_TO_LEVEL_DIVISOR: i32 = 104;
const REVEAL_AREA2_Z_SHIFT_SLOPE: f64 = 0.14350360082660085;
const REVEAL_AREA2_Z_SHIFT_HIGH_Z_THRESHOLD: i32 = 728;
const REVEAL_AREA2_Z_SHIFT_CELL_DIVISOR: i32 = 30;

/// Per-owner visibility stored as a flat grid of flag bytes.
///
/// Indexed by `ry * width + rx`. Each byte holds FLAG_REVEALED and/or
/// FLAG_VISIBLE bits. This gives O(1) per-cell lookups instead of O(log n)
/// with the previous BTreeSet design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerVisibility {
    cells: Vec<u8>,
    width: u16,
    height: u16,
}

impl Default for OwnerVisibility {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            width: 0,
            height: 0,
        }
    }
}

impl OwnerVisibility {
    /// Create a new zeroed visibility grid of the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let len: usize = (width as usize) * (height as usize);
        Self {
            cells: vec![0u8; len],
            width,
            height,
        }
    }

    /// Index into the flat grid, or None if out of bounds.
    fn index(&self, rx: u16, ry: u16) -> Option<usize> {
        if rx < self.width && ry < self.height {
            Some((ry as usize) * (self.width as usize) + (rx as usize))
        } else {
            None
        }
    }

    /// Returns true if the cell is currently visible (in line of sight).
    pub fn is_visible(&self, rx: u16, ry: u16) -> bool {
        self.index(rx, ry)
            .map_or(false, |i| self.cells[i] & FLAG_VISIBLE != 0)
    }

    /// Returns true if the cell has been revealed at least once.
    pub fn is_revealed(&self, rx: u16, ry: u16) -> bool {
        self.index(rx, ry)
            .map_or(false, |i| self.cells[i] & FLAG_REVEALED != 0)
    }

    /// Returns true if the cell is covered by an enemy gap generator this tick.
    pub fn is_gap_covered(&self, rx: u16, ry: u16) -> bool {
        self.index(rx, ry)
            .map_or(false, |i| self.cells[i] & FLAG_GAP_COVERED != 0)
    }

    /// Mark a cell as both visible and revealed.
    pub fn mark_visible(&mut self, rx: u16, ry: u16) {
        if let Some(i) = self.index(rx, ry) {
            self.cells[i] |= FLAG_VISIBLE | FLAG_REVEALED;
        }
    }

    /// Clear all visible flags while preserving revealed flags.
    /// Called each tick by `recompute_owner_visibility_in_place` so existing
    /// grids can be reused without reallocation.
    pub fn clear_all_visible(&mut self) {
        for cell in &mut self.cells {
            *cell &= !(FLAG_VISIBLE | FLAG_GAP_COVERED);
        }
    }

    /// Zero all flags (visible + revealed). Used when reusing the merged
    /// grid buffer in `build_merged_for`.
    fn clear_all(&mut self) {
        for cell in &mut self.cells {
            *cell = 0;
        }
    }

    /// Return the raw cells slice for deterministic hashing.
    pub fn cells_raw(&self) -> &[u8] {
        &self.cells
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// Merge revealed bits from a previous tick's grid into this one.
    /// Cells that were revealed before stay revealed even if no unit sees them now.
    pub fn merge_revealed_from(&mut self, other: &OwnerVisibility) {
        // If dimensions differ, fall back to per-cell copy for the overlapping region.
        if self.width == other.width && self.height == other.height {
            for (dst, src) in self.cells.iter_mut().zip(other.cells.iter()) {
                *dst |= *src & FLAG_REVEALED;
            }
        } else {
            let overlap_w: u16 = self.width.min(other.width);
            let overlap_h: u16 = self.height.min(other.height);
            for ry in 0..overlap_h {
                for rx in 0..overlap_w {
                    if other.is_revealed(rx, ry) {
                        if let Some(i) = self.index(rx, ry) {
                            self.cells[i] |= FLAG_REVEALED;
                        }
                    }
                }
            }
        }
    }

    /// Merge all flags (revealed + visible) from another grid into this one.
    /// Used to build a combined allied visibility view.
    pub fn merge_all_flags_from(&mut self, other: &OwnerVisibility) {
        if self.width == other.width && self.height == other.height {
            for (dst, src) in self.cells.iter_mut().zip(other.cells.iter()) {
                *dst |= *src;
            }
        } else {
            let overlap_w: u16 = self.width.min(other.width);
            let overlap_h: u16 = self.height.min(other.height);
            for ry in 0..overlap_h {
                for rx in 0..overlap_w {
                    if let (Some(si), Some(di)) = (other.index(rx, ry), self.index(rx, ry)) {
                        self.cells[di] |= other.cells[si];
                    }
                }
            }
        }
    }
}

/// Global fog/shroud state keyed by owner name.
///
/// Stores per-owner visibility grids plus a lazily-computed merged grid for
/// fast alliance-aware queries. The merged grid is built once via
/// `build_merged_for()` and then used by `is_cell_visible`, edge masks, etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FogState {
    pub width: u16,
    pub height: u16,
    pub by_owner: BTreeMap<InternedId, OwnerVisibility>,
    pub alliances: HouseAllianceMap,
    /// Pre-merged visibility grid for a specific owner + their allies.
    /// Built once per tick via `build_merged_for()`. All alliance-aware
    /// queries (is_cell_visible, edge masks) use this for O(1) lookups
    /// instead of iterating all owners per cell.
    #[serde(skip)]
    pub(crate) merged: Option<(InternedId, OwnerVisibility)>,
    /// Monotonically increasing counter bumped whenever visibility changes
    /// (after each `build_merged_for()` call). Used by the fog mask renderer
    /// and minimap to skip redundant updates when fog hasn't changed.
    pub generation: u64,
}

impl FogState {
    /// Build a merged visibility grid for the given owner and all their allies.
    /// Call once per tick (or when the local owner changes). Subsequent calls
    /// to `is_cell_visible`, `is_cell_revealed`, and edge mask methods will
    /// use this merged grid for O(1) lookups.
    ///
    /// Reuses the previous merged buffer when dimensions haven't changed to
    /// avoid per-tick allocation.
    pub fn build_merged_for(&mut self, owner: InternedId, interner: &StringInterner) {
        // Reuse existing buffer if dimensions match; otherwise allocate.
        let mut merged = match self.merged.take() {
            Some((_, mut vis)) if vis.width == self.width && vis.height == self.height => {
                vis.clear_all();
                vis
            }
            _ => OwnerVisibility::new(self.width, self.height),
        };
        let owner_str = interner.resolve(owner);
        for (viewer_id, state) in &self.by_owner {
            let viewer_str = interner.resolve(*viewer_id);
            if are_houses_friendly(&self.alliances, owner_str, viewer_str) {
                merged.merge_all_flags_from(state);
            }
        }
        self.merged = Some((owner, merged));
        self.generation = self.generation.wrapping_add(1);
    }

    /// Return a reference to the raw merged visibility cells (if built).
    /// Used by the snapshot system to diff visibility transitions cheaply
    /// without cloning the entire FogState.
    pub fn merged_cells(&self) -> Option<&[u8]> {
        self.merged.as_ref().map(|(_, vis)| vis.cells_raw())
    }

    /// Get the merged visibility grid, falling back to iterating all owners
    /// if no merged grid is available for this owner.
    fn merged_vis(&self, owner: InternedId) -> Option<&OwnerVisibility> {
        if let Some((cached_owner, ref vis)) = self.merged {
            if cached_owner == owner {
                return Some(vis);
            }
        }
        None
    }

    /// Returns true if the owner (or a friendly ally) currently sees the cell.
    pub fn is_cell_visible(&self, owner: InternedId, rx: u16, ry: u16) -> bool {
        // Fast path: use pre-merged grid.
        if let Some(vis) = self.merged_vis(owner) {
            return vis.is_visible(rx, ry);
        }
        // Slow fallback: iterate all owners (used in tests or when merged not built).
        // Only valid if by_owner is empty or merged not yet built.
        self.by_owner
            .get(&owner)
            .is_some_and(|s| s.is_visible(rx, ry))
    }

    /// Returns true if the owner (or a friendly ally) has revealed the cell.
    pub fn is_cell_revealed(&self, owner: InternedId, rx: u16, ry: u16) -> bool {
        if let Some(vis) = self.merged_vis(owner) {
            return vis.is_revealed(rx, ry);
        }
        self.by_owner
            .get(&owner)
            .is_some_and(|s| s.is_revealed(rx, ry))
    }

    /// Returns true if the cell is covered by an enemy gap generator for this owner.
    pub fn is_cell_gap_covered(&self, owner: InternedId, rx: u16, ry: u16) -> bool {
        if let Some(vis) = self.merged_vis(owner) {
            return vis.is_gap_covered(rx, ry);
        }
        self.by_owner
            .get(&owner)
            .is_some_and(|s| s.is_gap_covered(rx, ry))
    }

    /// Returns true if two owners should be treated as friendly.
    pub fn is_friendly(&self, a: &str, b: &str) -> bool {
        are_houses_friendly(&self.alliances, a, b)
    }

    /// Returns true if two interned owners should be treated as friendly.
    pub fn is_friendly_id(&self, a: InternedId, b: InternedId, interner: &StringInterner) -> bool {
        a == b || are_houses_friendly(&self.alliances, interner.resolve(a), interner.resolve(b))
    }

    /// Clear all explored/revealed state for the given owner.
    /// Used by spy infiltration to reset an enemy's map knowledge.
    pub fn reset_explored_for_owner(&mut self, owner: InternedId) {
        if let Some(vis) = self.by_owner.get_mut(&owner) {
            for cell in &mut vis.cells {
                *cell = 0;
            }
        }
    }

    /// 4-bit neighbor mask for shroud edge rendering.
    ///
    /// Returns a mask where each bit indicates that the corresponding iso
    /// edge-sharing neighbor is ALSO shrouded (never revealed). A set bit means
    /// the neighbor is in the same state (shrouded), so no edge fade is needed
    /// on that side.
    ///
    /// Bit layout matches the diamond's 4 edges (same as LAT adjacency):
    /// Bit 0 = NE (rx, ry-1), Bit 1 = SE (rx+1, ry), Bit 2 = SW (rx, ry+1),
    /// Bit 3 = NW (rx-1, ry).
    ///
    /// Out-of-bounds neighbors are treated as shrouded (bit set).
    pub fn shroud_edge_mask(&self, owner: InternedId, rx: u16, ry: u16) -> u8 {
        let mut mask: u8 = 0;
        if ry == 0 || !self.is_cell_revealed(owner, rx, ry - 1) {
            mask |= 0x01;
        }
        if !self.is_cell_revealed(owner, rx + 1, ry) {
            mask |= 0x02;
        }
        if !self.is_cell_revealed(owner, rx, ry + 1) {
            mask |= 0x04;
        }
        if rx == 0 || !self.is_cell_revealed(owner, rx - 1, ry) {
            mask |= 0x08;
        }
        mask
    }

    /// 8-bit neighbor mask for SHROUD.SHP edge rendering.
    ///
    /// Each bit is SET when that neighbor IS shrouded (unexplored).
    /// The 8-bit value indexes directly into the 256-byte frame lookup table
    /// to select which SHROUD.SHP frame to render.
    ///
    /// Only meaningful for cells that ARE revealed — call on explored cells only.
    ///
    /// Bit layout (cell-relative dx,dy):
    /// ```text
    ///   NW(-1,-1)=bit6   N(0,-1)=bit7   NE(+1,-1)=bit0
    ///   W(-1, 0)=bit5       *            E(+1, 0)=bit1
    ///   SW(-1,+1)=bit4   S(0,+1)=bit3   SE(+1,+1)=bit2
    /// ```
    ///
    /// Out-of-bounds neighbors are treated as shrouded (bit set).
    pub fn shroud_edge_mask_8bit(&self, owner: InternedId, rx: u16, ry: u16) -> u8 {
        let mut mask: u8 = 0;
        // bit 0 = NE (+1, -1)
        if ry == 0 || !self.is_cell_revealed(owner, rx + 1, ry - 1) {
            mask |= 0x01;
        }
        // bit 1 = E (+1, 0)
        if !self.is_cell_revealed(owner, rx + 1, ry) {
            mask |= 0x02;
        }
        // bit 2 = SE (+1, +1)
        if !self.is_cell_revealed(owner, rx + 1, ry + 1) {
            mask |= 0x04;
        }
        // bit 3 = S (0, +1)
        if !self.is_cell_revealed(owner, rx, ry + 1) {
            mask |= 0x08;
        }
        // bit 4 = SW (-1, +1)
        if rx == 0 || !self.is_cell_revealed(owner, rx - 1, ry + 1) {
            mask |= 0x10;
        }
        // bit 5 = W (-1, 0)
        if rx == 0 || !self.is_cell_revealed(owner, rx - 1, ry) {
            mask |= 0x20;
        }
        // bit 6 = NW (-1, -1)
        if rx == 0 || ry == 0 || !self.is_cell_revealed(owner, rx - 1, ry - 1) {
            mask |= 0x40;
        }
        // bit 7 = N (0, -1)
        if ry == 0 || !self.is_cell_revealed(owner, rx, ry - 1) {
            mask |= 0x80;
        }
        mask
    }

    /// Test helper: mark a cell visible for the given owner.
    /// Auto-expands the grid dimensions if needed so tests don't need to
    /// pre-set width/height.
    #[cfg(test)]
    pub fn mark_visible_for_owner(&mut self, owner: InternedId, rx: u16, ry: u16) {
        let needed_w: u16 = rx.saturating_add(1);
        let needed_h: u16 = ry.saturating_add(1);
        if self.width < needed_w {
            self.width = needed_w;
        }
        if self.height < needed_h {
            self.height = needed_h;
        }
        let w = self.width;
        let h = self.height;
        let state = self
            .by_owner
            .entry(owner)
            .or_insert_with(|| OwnerVisibility::new(w, h));
        if state.width() < w || state.height() < h {
            let mut expanded = OwnerVisibility::new(w, h);
            expanded.merge_all_flags_from(state);
            *state = expanded;
        }
        state.mark_visible(rx, ry);
    }
}

/// Configuration for visibility computation, passed to `recompute_owner_visibility`.
pub struct VisionConfig {
    /// Multiplicative sight scalar applied when a veterancy SIGHT ability gate passes.
    /// Parsed from [General] `VeteranSight=`. Default 0.0 disables the multiplier.
    pub veteran_sight_scalar: f32,
    /// Leptons of elevation per +1 sight cell (from [General] LeptonsPerSightIncrease=).
    /// 256 leptons = 1 z-level. 0 disables the elevation bonus.
    pub leptons_per_sight_increase: i32,
    /// Height-based LOS obstruction (from [General] RevealByHeight=).
    /// When true, terrain 4+ levels above the viewer at the midpoint blocks sight.
    /// Default true (the standard RA2/YR setting).
    pub reveal_by_height: bool,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            veteran_sight_scalar: 0.0,
            leptons_per_sight_increase: 0,
            reveal_by_height: true,
        }
    }
}

/// Recompute deterministic fog/shroud state for all owners (allocating variant).
///
/// Creates a fresh `FogState` and populates it. Used by tests; production code
/// calls `recompute_owner_visibility_in_place` to avoid per-tick allocation.
pub fn recompute_owner_visibility(
    entities: &EntityStore,
    path_grid: Option<&PathGrid>,
    alliances: &HouseAllianceMap,
    config: &VisionConfig,
    rules: Option<&RuleSet>,
    interner: &crate::sim::intern::StringInterner,
) -> FogState {
    let mut fog = FogState::default();
    recompute_owner_visibility_in_place(
        &mut fog, entities, path_grid, alliances, config, rules, None, interner,
    );
    fog
}

/// Recompute deterministic fog/shroud visibility in-place, reusing existing grids.
///
/// Clears `FLAG_VISIBLE` on all existing owner grids (preserving `FLAG_REVEALED`),
/// then re-reveals from entity positions. New owners get a fresh grid; dead owners
/// keep their revealed state with no visible cells.
///
/// This avoids the per-tick allocation of `Vec<u8>` grids and the subsequent
/// `merge_revealed_from` pass — revealed bits are never destroyed.
pub fn recompute_owner_visibility_in_place(
    fog: &mut FogState,
    entities: &EntityStore,
    path_grid: Option<&PathGrid>,
    alliances: &HouseAllianceMap,
    config: &VisionConfig,
    rules: Option<&RuleSet>,
    height_grid: Option<&[u8]>,
    interner: &crate::sim::intern::StringInterner,
) {
    let (width, height) = resolve_bounds(entities, path_grid);
    if width == 0 || height == 0 {
        *fog = FogState::default();
        return;
    }

    // First tick or dimension change: recreate all grids (cold path).
    if fog.width != width || fog.height != height {
        fog.by_owner.clear();
        fog.width = width;
        fog.height = height;
    } else {
        // Hot path: clear visible flags, preserve revealed.
        for vis in fog.by_owner.values_mut() {
            vis.clear_all_visible();
        }
    }

    fog.alliances = alliances.clone();
    fog.merged = None;

    // Batch entities by owner to avoid repeated BTreeMap lookups and String allocations.
    // Each unique owner's grid is looked up once, then all their entities reveal into it.
    for entity in entities.values() {
        // Skip entities inside a transport — they don't provide vision.
        if entity.passenger_role.is_inside_transport() {
            continue;
        }

        let vis = fog
            .by_owner
            .entry(entity.owner)
            .or_insert_with(|| OwnerVisibility::new(width, height));

        let effective = effective_sight_range(entity, config, rules, interner);
        let origin_x_leptons = i32::from(entity.position.rx) * LEPTONS_PER_CELL_I32
            + entity.position.sub_x.to_num::<i32>();
        let origin_y_leptons = i32::from(entity.position.ry) * LEPTONS_PER_CELL_I32
            + entity.position.sub_y.to_num::<i32>();
        let origin_z_leptons = i32::from(entity.position.z) * REVEAL_AREA2_Z_TO_LEVEL_DIVISOR;

        reveal_radius_into(
            vis,
            origin_x_leptons,
            origin_y_leptons,
            origin_z_leptons,
            effective,
            config.reveal_by_height,
            height_grid,
            width,
            height,
        );
    }
}

fn effective_sight_range(
    entity: &crate::sim::game_entity::GameEntity,
    config: &VisionConfig,
    rules: Option<&RuleSet>,
    interner: &crate::sim::intern::StringInterner,
) -> u16 {
    let base_range = entity.vision_range as i32;
    let height_leptons = entity.position.z as i32 * 256;
    let pre_veteran_range = if config.leptons_per_sight_increase > 0 {
        let elevation_step = (height_leptons / config.leptons_per_sight_increase) as f32 * 10.0;
        (base_range as f32 * (1.0 + elevation_step * 0.01)) as i32
    } else {
        base_range
    };

    let sight_scaled_range = rules
        .and_then(|rules| rules.object(interner.resolve(entity.type_ref)))
        .and_then(|obj| {
            let has_sight_ability = if entity.veterancy >= 200 {
                obj.veteran_sight_ability || obj.elite_sight_ability
            } else if entity.veterancy >= 100 {
                obj.veteran_sight_ability
            } else {
                false
            };
            if has_sight_ability && config.veteran_sight_scalar != 0.0 {
                Some((pre_veteran_range as f32 * config.veteran_sight_scalar) as i32)
            } else {
                None
            }
        })
        .unwrap_or(pre_veteran_range);

    sight_scaled_range.clamp(0, MAX_SIGHT_RANGE as i32) as u16
}

fn resolve_bounds(entities: &EntityStore, path_grid: Option<&PathGrid>) -> (u16, u16) {
    if let Some(grid) = path_grid {
        return (grid.width(), grid.height());
    }

    let mut max_x = 0u16;
    let mut max_y = 0u16;
    let mut found = false;
    for entity in entities.values() {
        found = true;
        max_x = max_x.max(entity.position.rx);
        max_y = max_y.max(entity.position.ry);
    }
    if found {
        (max_x.saturating_add(1), max_y.saturating_add(1))
    } else {
        (0, 0)
    }
}

/// Mark all cells within `range` of the projected RA2/YR origin as visible+revealed.
///
/// Uses the recovered `RevealArea2` per-cell gate: project the origin by Z,
/// truncate Euclidean distance, and reject candidate cells above origin level + 3
/// when RevealByHeight is active. Candidate enumeration uses the recovered
/// AoE offset table through the RA2 hard cap of sight 10.
fn reveal_radius_into(
    vis: &mut OwnerVisibility,
    origin_x_leptons: i32,
    origin_y_leptons: i32,
    origin_z_leptons: i32,
    range: u16,
    reveal_by_height: bool,
    height_grid: Option<&[u8]>,
    width: u16,
    height: u16,
) {
    let (cx, cy) =
        reveal_area2_projected_origin_cell(origin_x_leptons, origin_y_leptons, origin_z_leptons);
    let w = i32::from(width);
    let h = i32::from(height);

    // Clamp range to MAX_SIGHT_RANGE (the original also clamps to 10).
    let clamped = (range as usize).min(MAX_SIGHT_RANGE as usize);

    let spiral_end = REVEAL_RING_SIZES[clamped];

    for i in 0..spiral_end {
        let (dx, dy) = REVEAL_SPIRAL[i];
        let rx = cx + dx as i32;
        let ry = cy + dy as i32;
        if reveal_area2_candidate_passes(
            rx,
            ry,
            cx,
            cy,
            clamped as i32,
            origin_z_leptons,
            reveal_by_height,
            height_grid,
            w,
            h,
        ) {
            vis.mark_visible(rx as u16, ry as u16);
        }
    }
}

fn reveal_area2_projected_origin_cell(
    origin_x_leptons: i32,
    origin_y_leptons: i32,
    origin_z_leptons: i32,
) -> (i32, i32) {
    let shift = reveal_area2_origin_cell_shift_yr(origin_z_leptons);
    (
        origin_x_leptons / LEPTONS_PER_CELL_I32 + shift,
        origin_y_leptons / LEPTONS_PER_CELL_I32 + shift,
    )
}

fn reveal_area2_origin_cell_shift_yr(z_leptons: i32) -> i32 {
    -(reveal_area2_adjust_for_z_yr(z_leptons) / REVEAL_AREA2_Z_SHIFT_CELL_DIVISOR)
}

fn reveal_area2_adjust_for_z_yr(z_leptons: i32) -> i32 {
    let high_z_bias = if z_leptons >= REVEAL_AREA2_Z_SHIFT_HIGH_Z_THRESHOLD {
        1.0
    } else {
        0.0
    };
    (z_leptons as f64 * REVEAL_AREA2_Z_SHIFT_SLOPE + high_z_bias + 0.5).trunc() as i32
}

fn reveal_area2_candidate_passes(
    rx: i32,
    ry: i32,
    cx: i32,
    cy: i32,
    range: i32,
    origin_z_leptons: i32,
    reveal_by_height: bool,
    height_grid: Option<&[u8]>,
    width: i32,
    height: i32,
) -> bool {
    if rx < 0 || rx >= width || ry < 0 || ry >= height {
        return false;
    }

    let dx = rx - cx;
    let dy = ry - cy;
    let distance = ((dx * dx + dy * dy) as f64).sqrt().trunc() as i32;
    if distance > range {
        return false;
    }

    let Some(hg) = height_grid.filter(|_| reveal_by_height) else {
        return true;
    };

    let candidate_level = hg[(ry * width + rx) as usize] as i32;
    let origin_level = origin_z_leptons / REVEAL_AREA2_Z_TO_LEVEL_DIVISOR;
    candidate_level <= origin_level + 3
}

/// Reveal spiral table extracted from the original engine.
/// Each (dx, dy) is a cell offset from the revealing unit's position.
/// Entries are ordered in expanding rings by sight radius.
#[rustfmt::skip]
const REVEAL_SPIRAL: [(i8, i8); 309] = [
    // Sight 0: 1 entry
    (0, 0),
    // Sight 1: entries 1..9 (8 new)
    (1, -1), (0, -1), (-1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1),
    // Sight 2: entries 9..21 (12 new)
    (-1, -2), (0, -2), (1, -2), (-2, -1), (2, -1), (-2, 0), (2, 0), (-2, 1), (2, 1), (-1, 2),
    (0, 2), (1, 2),
    // Sight 3: entries 21..37 (16 new)
    (-1, -3), (0, -3), (1, -3), (-2, -2), (2, -2), (-3, -1), (3, -1), (-3, 0), (3, 0), (-3, 1),
    (3, 1), (-2, 2), (2, 2), (-1, 3), (0, 3), (1, 3),
    // Sight 4: entries 37..61 (24 new)
    (-1, -4), (0, -4), (1, -4), (-3, -3), (-2, -3), (2, -3), (3, -3), (-3, -2), (3, -2),
    (-4, -1), (4, -1), (-4, 0), (4, 0), (-4, 1), (4, 1), (-3, 2), (3, 2), (-3, 3), (-2, 3),
    (2, 3), (3, 3), (-1, 4), (0, 4), (1, 4),
    // Sight 5: entries 61..89 (28 new)
    (-1, -5), (0, -5), (1, -5), (-3, -4), (-2, -4), (2, -4), (3, -4), (-4, -3), (4, -3),
    (-4, -2), (4, -2), (-5, -1), (5, -1), (-5, 0), (5, 0), (-5, 1), (5, 1), (-4, 2), (4, 2),
    (-4, 3), (4, 3), (-3, 4), (-2, 4), (2, 4), (3, 4), (-1, 5), (0, 5), (1, 5),
    // Sight 6: entries 89..121 (32 new)
    (-1, -6), (0, -6), (1, -6), (-3, -5), (-2, -5), (2, -5), (3, -5), (-4, -4), (4, -4),
    (-5, -3), (5, -3), (-5, -2), (5, -2), (-6, -1), (6, -1), (-6, 0), (6, 0), (-6, 1), (6, 1),
    (-5, 2), (5, 2), (-5, 3), (5, 3), (-4, 4), (4, 4), (-3, 5), (-2, 5), (2, 5), (3, 5),
    (-1, 6), (0, 6), (1, 6),
    // Sight 7: entries 121..161 (40 new)
    (-1, -7), (0, -7), (1, -7), (-3, -6), (-2, -6), (2, -6), (3, -6), (-5, -5), (-4, -5),
    (4, -5), (5, -5), (-5, -4), (5, -4), (-6, -3), (6, -3), (-6, -2), (6, -2), (-7, -1), (7, -1),
    (-7, 0), (7, 0), (-7, 1), (7, 1), (-6, 2), (6, 2), (-6, 3), (6, 3), (-5, 4), (5, 4),
    (-5, 5), (-4, 5), (4, 5), (5, 5), (-3, 6), (-2, 6), (2, 6), (3, 6), (-1, 7), (0, 7), (1, 7),
    // Sight 8: entries 161..205 (44 new)
    (-1, -8), (0, -8), (1, -8), (-3, -7), (-2, -7), (2, -7), (3, -7), (-5, -6), (-4, -6),
    (4, -6), (5, -6), (-6, -5), (6, -5), (-6, -4), (6, -4), (-7, -3), (7, -3), (-7, -2), (7, -2),
    (-8, -1), (8, -1), (-8, 0), (8, 0), (-8, 1), (8, 1), (-7, 2), (7, 2), (-7, 3), (7, 3),
    (-6, 4), (6, 4), (-6, 5), (6, 5), (-5, 6), (-4, 6), (4, 6), (5, 6), (-3, 7), (-2, 7),
    (2, 7), (3, 7), (-1, 8), (0, 8), (1, 8),
    // Sight 9: entries 205..253 (48 new)
    (-1, -9), (0, -9), (1, -9), (-3, -8), (-2, -8), (2, -8), (3, -8), (-5, -7), (-4, -7),
    (4, -7), (5, -7), (-6, -6), (6, -6), (-7, -5), (7, -5), (-7, -4), (7, -4), (-8, -3), (8, -3),
    (-8, -2), (8, -2), (-9, -1), (9, -1), (-9, 0), (9, 0), (-9, 1), (9, 1), (-8, 2), (8, 2),
    (-8, 3), (8, 3), (-7, 4), (7, 4), (-7, 5), (7, 5), (-6, 6), (6, 6), (-5, 7), (-4, 7),
    (4, 7), (5, 7), (-3, 8), (-2, 8), (2, 8), (3, 8), (-1, 9), (0, 9), (1, 9),
    // Sight 10: entries 253..309 (56 new)
    (-1, -10), (0, -10), (1, -10), (-3, -9), (-2, -9), (2, -9), (3, -9), (-5, -8), (-4, -8),
    (4, -8), (5, -8), (-7, -7), (-6, -7), (6, -7), (7, -7), (-7, -6), (7, -6), (-8, -5),
    (8, -5), (-8, -4), (8, -4), (-9, -3), (9, -3), (-9, -2), (9, -2), (-10, -1), (10, -1),
    (-10, 0), (10, 0), (-10, 1), (10, 1), (-9, 2), (9, 2), (-9, 3), (9, 3), (-8, 4),
    (8, 4), (-8, 5), (8, 5), (-7, 6), (7, 6), (-7, 7), (-6, 7), (6, 7), (7, 7),
    (-5, 8), (-4, 8), (4, 8), (5, 8), (-3, 9), (-2, 9), (2, 9), (3, 9), (-1, 10),
    (0, 10), (1, 10),
];

/// Cumulative AoE offset count for each sight radius 0-10.
/// Radius 10 entries 253-308 are from
/// `evidence/derived/aoe_radius10_11_dxdy_parity_static_recovered_20260503.tsv`.
const REVEAL_RING_SIZES: [usize; 11] = [1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309];

/// Public version of reveal_radius for use by external systems (e.g., RevealOnFire).
pub fn reveal_radius(
    fog: &mut FogState,
    owner: InternedId,
    center_rx: u16,
    center_ry: u16,
    range: u16,
) {
    let width = fog.width;
    let height = fog.height;
    if width == 0 || height == 0 {
        return;
    }
    let vis = fog
        .by_owner
        .entry(owner)
        .or_insert_with(|| OwnerVisibility::new(width, height));
    // Fire-reveal events don't use height-based LOS (matches gamemd).
    reveal_radius_into(
        vis,
        i32::from(center_rx) * LEPTONS_PER_CELL_I32,
        i32::from(center_ry) * LEPTONS_PER_CELL_I32,
        0,
        range,
        false,
        None,
        width,
        height,
    );
}

/// Apply SpySat full-map reveal: if any alive SpySat building exists for an owner,
/// mark ALL cells visible+revealed for that owner. Call after normal vision recompute.
///
/// Takes a list of owner names for each powered SpySat building currently alive.
/// Power state filtering is done by the caller (see `refresh_fog`).
pub fn apply_spy_sat(
    fog: &mut FogState,
    spy_sat_owners: &[InternedId],
    _interner: &StringInterner,
) {
    let width = fog.width;
    let height = fog.height;
    for &owner_id in spy_sat_owners {
        let vis = fog
            .by_owner
            .entry(owner_id)
            .or_insert_with(|| OwnerVisibility::new(width, height));
        for cell in &mut vis.cells {
            *cell |= FLAG_VISIBLE | FLAG_REVEALED;
        }
    }
}

/// Apply Gap Generator suppression: for each gap generator, clear FLAG_VISIBLE
/// on all enemy owners' cells within `gap_radius`. This turns Visible → Revealed
/// (fogged) for enemies in the gap field. Call AFTER spy_sat so gap wins.
///
/// Takes a list of (owner_name, rx, ry) for each gap generator building/unit.
pub fn apply_gap_generators(
    fog: &mut FogState,
    gap_generators: &[(InternedId, u16, u16)],
    gap_radius: i32,
    interner: &StringInterner,
) {
    let width = fog.width;
    let height = fog.height;
    if width == 0 || height == 0 {
        return;
    }
    let rr = gap_radius;
    for &(gap_owner_id, center_rx, center_ry) in gap_generators {
        let gap_owner = interner.resolve(gap_owner_id);
        let cx = i32::from(center_rx);
        let cy = i32::from(center_ry);
        let min_x = (cx - rr).max(0);
        let max_x = (cx + rr).min(i32::from(width) - 1);
        let min_y = (cy - rr).max(0);
        let max_y = (cy + rr).min(i32::from(height) - 1);
        let radius_sq = rr * rr;

        // Clear visibility for all enemy owners in the gap radius.
        for (viewer_id, vis) in fog.by_owner.iter_mut() {
            let viewer = interner.resolve(*viewer_id);
            if are_houses_friendly(&fog.alliances, gap_owner, viewer) {
                continue; // Don't suppress friendly vision.
            }
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let dx = x - cx;
                    let dy = y - cy;
                    if dx * dx + dy * dy > radius_sq {
                        continue;
                    }
                    if let Some(i) = vis.index(x as u16, y as u16) {
                        vis.cells[i] &= !FLAG_VISIBLE;
                        vis.cells[i] |= FLAG_GAP_COVERED;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod vision_tests;
