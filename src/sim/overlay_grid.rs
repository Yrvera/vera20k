//! Per-cell mutable overlay state — runtime fork of the immutable map OverlayPack.
//!
//! Mirrors CellClass +0x44 (OverlayTypeIndex) and +0x11E (OverlayData) from gamemd.exe.
//! Seeded from map data at init, mutated during gameplay by ore growth, wall damage,
//! and bridge overlay damage.
//!
//! Dependency rules: depends on map/overlay (OverlayEntry for seeding).
//! Never depends on render/, ui/, sidebar/, audio/, net/.

use crate::map::overlay::{OverlayDataPack, OverlayEntry};
use crate::map::overlay_types::{
    OverlayTypeRegistry, clears_tiberium_on_slope, is_bridge_overlay_index, retained_overlay_land,
};
use crate::map::resolved_terrain::{
    ResolvedTerrainGrid, overlay_reduced_zone_type, recalc_zone_type,
};
use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
use crate::sim::intern::InternedId;

/// Per-cell mutable overlay state — mirrors CellClass +0x44 / +0x11E.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OverlayCell {
    /// Overlay type index into OverlayTypeRegistry. None = no overlay.
    pub overlay_id: Option<u8>,
    /// Multi-purpose data byte:
    /// - Ore/gems (Tiberium=true): density 0-11 (SHP frame index)
    /// - Walls (Wall=true): (damage_level << 4) | connectivity_bitmask
    ///   Connectivity: N=1, E=2, S=4, W=8
    /// - Bridges: damage state 0-17 (EW 0-8, NS 9-17)
    /// - Other: raw frame index
    pub overlay_data: u8,
    /// Owning house for a placed wall. Map-pack overlays start unowned.
    ///
    /// A cleared GAWALL/NAWALL cell may retain this value after native's
    /// isolated-neighbor cleanup. It is inert while `overlay_id` is `None` and
    /// is overwritten by the next placement.
    pub wall_owner: Option<InternedId>,
}

impl Default for OverlayCell {
    fn default() -> Self {
        Self {
            overlay_id: None,
            overlay_data: 0,
            wall_owner: None,
        }
    }
}

/// Mutable overlay state grid — seeded from map [OverlayPack] at init,
/// mutated during gameplay by ore growth, wall damage, bridge damage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OverlayGrid {
    width: u16,
    height: u16,
    cells: Vec<OverlayCell>,
    /// Cells mutated this tick — drained by the app layer to trigger
    /// `recalc_overlay_passability`. Not part of game state; never serialized.
    /// Always empty at tick boundaries (drained every tick after `advance_tick`).
    #[serde(skip, default)]
    dirty_cells: Vec<(u16, u16)>,
    /// A synchronous sim-side recalc already observed a passability change for
    /// one of the dirty cells. The app drain must preserve that first result
    /// even though recalculating the now-current terrain returns `false`.
    #[serde(skip, default)]
    synchronous_passability_changed: bool,
}

impl OverlayGrid {
    /// Create an empty grid with no overlays.
    pub fn new(width: u16, height: u16) -> Self {
        let count = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![OverlayCell::default(); count],
            dirty_cells: Vec::new(),
            synchronous_passability_changed: false,
        }
    }

    /// Seed from parsed map overlay entries.
    ///
    /// Bridge overlays are intentionally excluded: bridge body/bridgehead
    /// overlay bytes are owned by `BridgeRuntimeState`, while this grid owns
    /// mutable non-bridge overlay bytes such as ore and walls.
    pub fn from_overlay_entries(entries: &[OverlayEntry], width: u16, height: u16) -> Self {
        let mut grid = Self::new(width, height);
        for entry in entries {
            if is_bridge_overlay_index(entry.overlay_id) {
                continue;
            }
            if let Some(idx) = index_of(width, height, entry.rx, entry.ry) {
                grid.cells[idx] = OverlayCell {
                    overlay_id: Some(entry.overlay_id),
                    overlay_data: entry.frame,
                    wall_owner: None,
                };
            }
        }
        grid
    }

    /// Seed overlay identities from `[OverlayPack]`, then apply the raw
    /// `[OverlayDataPack]` byte to every cell when that pack is present.
    ///
    /// The native map reader performs the data-pack overwrite after stamping
    /// overlay identities, including cells whose overlay identity is empty.
    /// Initialization writes directly so it creates no runtime dirtiness.
    pub fn from_overlay_packs(
        entries: &[OverlayEntry],
        data: &OverlayDataPack,
        width: u16,
        height: u16,
    ) -> Self {
        let mut grid = Self::from_overlay_entries(entries, width, height);
        if data.is_present() {
            for ry in 0..height {
                for rx in 0..width {
                    let idx = ry as usize * width as usize + rx as usize;
                    grid.cells[idx].overlay_data = data.byte_at(rx, ry);
                }
            }
        }
        grid
    }

    /// Read cell at (rx, ry). Returns default (no overlay) for out-of-bounds.
    pub fn cell(&self, rx: u16, ry: u16) -> &OverlayCell {
        match index_of(self.width, self.height, rx, ry) {
            Some(idx) => &self.cells[idx],
            None => &DEFAULT_CELL,
        }
    }

    /// Mutable access to cell. Panics if out-of-bounds.
    pub fn cell_mut(&mut self, rx: u16, ry: u16) -> &mut OverlayCell {
        let idx =
            index_of(self.width, self.height, rx, ry).expect("OverlayGrid::cell_mut out of bounds");
        &mut self.cells[idx]
    }

    /// Remove overlay from cell entirely. Returns previous overlay_id if any.
    pub fn clear_overlay(&mut self, rx: u16, ry: u16) -> Option<u8> {
        let idx = index_of(self.width, self.height, rx, ry)?;
        let prev = self.cells[idx].overlay_id;
        self.cells[idx] = OverlayCell::default();
        self.dirty_cells.push((rx, ry));
        prev
    }

    /// Place overlay at cell.
    pub fn place_overlay(&mut self, rx: u16, ry: u16, overlay_id: u8, data: u8) {
        if let Some(idx) = index_of(self.width, self.height, rx, ry) {
            self.cells[idx] = OverlayCell {
                overlay_id: Some(overlay_id),
                overlay_data: data,
                wall_owner: None,
            };
            self.dirty_cells.push((rx, ry));
        }
    }

    /// Stamp a player-owned wall overlay at a cell.
    pub fn place_owned_wall(
        &mut self,
        rx: u16,
        ry: u16,
        overlay_id: u8,
        data: u8,
        owner: InternedId,
    ) {
        if let Some(idx) = index_of(self.width, self.height, rx, ry) {
            self.cells[idx] = OverlayCell {
                overlay_id: Some(overlay_id),
                overlay_data: data,
                wall_owner: Some(owner),
            };
            self.dirty_cells.push((rx, ry));
        }
    }

    /// Clear type/data while retaining the wall owner.
    fn clear_overlay_preserving_owner(&mut self, rx: u16, ry: u16) -> Option<u8> {
        let idx = index_of(self.width, self.height, rx, ry)?;
        let prev = self.cells[idx].overlay_id;
        self.cells[idx].overlay_id = None;
        self.cells[idx].overlay_data = 0;
        self.dirty_cells.push((rx, ry));
        prev
    }

    /// Update overlay_data in place (density change, damage increment).
    /// No-op if out-of-bounds or cell has no overlay.
    pub fn set_overlay_data(&mut self, rx: u16, ry: u16, data: u8) {
        if let Some(idx) = index_of(self.width, self.height, rx, ry) {
            if self.cells[idx].overlay_id.is_some() {
                self.cells[idx].overlay_data = data;
                self.dirty_cells.push((rx, ry));
            }
        }
    }

    /// Count ore/gem neighbors (8-dir) on demand.
    pub fn count_ore_neighbors(&self, rx: u16, ry: u16, registry: &OverlayTypeRegistry) -> u8 {
        let mut count: u8 = 0;
        for (dx, dy) in ADJACENT_8 {
            let nx = rx as i32 + dx;
            let ny = ry as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let cell = self.cell(nx as u16, ny as u16);
            if let Some(id) = cell.overlay_id {
                if registry.flags(id).is_some_and(|f| f.tiberium) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Iterate all cells that have an overlay (for hashing).
    pub fn iter_occupied(&self) -> impl Iterator<Item = (u16, u16, &OverlayCell)> {
        self.cells
            .iter()
            .enumerate()
            .filter_map(move |(idx, cell)| {
                if cell.overlay_id.is_some() {
                    let rx = (idx % self.width as usize) as u16;
                    let ry = (idx / self.width as usize) as u16;
                    Some((rx, ry, cell))
                } else {
                    None
                }
            })
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// Drain the list of cells mutated since last call. Consumer (app layer)
    /// calls `recalc_overlay_passability` for each and may trigger a zone rebuild.
    ///
    /// MUST be called every tick to keep the list empty at snapshot boundaries.
    pub fn take_dirty_cells(&mut self) -> Vec<(u16, u16)> {
        self.take_dirty_cells_with_passability_signal().0
    }

    /// Preserve a passability-change result from a required synchronous recalc
    /// until the normal app-side dirty drain can rebuild paths and zones.
    pub(crate) fn record_synchronous_passability_change(&mut self, changed: bool) {
        self.synchronous_passability_changed |= changed;
    }

    /// Drain overlay dirtiness and any already-observed passability change as
    /// one runtime-only result. Neither component is serialized or hashed.
    pub(crate) fn take_dirty_cells_with_passability_signal(
        &mut self,
    ) -> (Vec<(u16, u16)>, bool) {
        (
            std::mem::take(&mut self.dirty_cells),
            std::mem::take(&mut self.synchronous_passability_changed),
        )
    }
}

/// Recompute overlay-driven fields on ResolvedTerrainCell after an overlay mutation.
///
/// Reads overlay_id from the grid, checks registry flags for reduced ZoneType,
/// computes new overlay_blocks / zone_type / land_type / speed_costs values, writes
/// them to resolved_terrain. Returns true if any passability- or zone-relevant value
/// changed (caller should trigger zone rebuild).
///
/// Mirrors gamemd.exe RecalcAttributes stage 3a, scoped to overlay->passability +
/// overlay->LandType (+0xEC).
pub fn recalc_overlay_passability(
    overlay_grid: &mut OverlayGrid,
    resolved_terrain: &mut ResolvedTerrainGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
) -> bool {
    use crate::map::resolved_terrain::zone_class;

    let Some(slope_type) = resolved_terrain.cell(rx, ry).map(|cell| cell.slope_type) else {
        return false;
    };
    let mut overlay_id = overlay_grid.cell(rx, ry).overlay_id;
    let mut cleared_resource_for_slope = false;
    if overlay_id
        .and_then(|id| registry.flags(id))
        .is_some_and(|flags| clears_tiberium_on_slope(flags, slope_type))
    {
        *overlay_grid.cell_mut(rx, ry) = OverlayCell::default();
        overlay_id = None;
        cleared_resource_for_slope = true;
    }
    let flags = overlay_id.and_then(|id| registry.flags(id));
    let overlay_zone = overlay_reduced_zone_type(flags);
    let new_blocks = matches!(
        overlay_zone,
        Some(zone_class::WALL) | Some(zone_class::IMPASSABLE)
    );

    let Some(terrain_cell) = resolved_terrain.cell_mut(rx, ry) else {
        return false;
    };

    let old_overlay_zone_type = terrain_cell.overlay_zone_type;
    let old = (
        terrain_cell.overlay_blocks,
        terrain_cell.zone_type,
        terrain_cell.land_type,
        terrain_cell.yr_cell_land_type,
        terrain_cell.terrain_class,
        terrain_cell.speed_costs,
        terrain_cell.is_water,
        terrain_cell.is_cliff_like,
        terrain_cell.is_rough,
        terrain_cell.is_road,
        terrain_cell.ground_walk_blocked,
        terrain_cell.build_blocked,
    );

    restore_pristine_land(terrain_cell);
    if let Some(flags) = flags {
        if let Some(land) = retained_overlay_land(flags, slope_type) {
            apply_overlay_land(terrain_cell, land, flags.land_speed_costs);
        }
    }

    terrain_cell.overlay_blocks = new_blocks;
    terrain_cell.overlay_zone_type = overlay_zone;
    terrain_cell.ground_walk_blocked =
        terrain_cell.base_ground_walk_blocked || terrain_cell.terrain_object_blocks || new_blocks;
    terrain_cell.build_blocked = terrain_cell.base_build_blocked
        || terrain_cell.terrain_object_blocks
        || new_blocks
        || terrain_cell.has_bridge_deck;
    terrain_cell.zone_type = recalc_zone_type(
        terrain_cell.outside_playfield,
        terrain_cell.overlay_zone_type,
        terrain_cell.land_type,
        terrain_cell.speed_costs.wheel,
        terrain_cell.terrain_object_occupation,
    );

    let new = (
        terrain_cell.overlay_blocks,
        terrain_cell.zone_type,
        terrain_cell.land_type,
        terrain_cell.yr_cell_land_type,
        terrain_cell.terrain_class,
        terrain_cell.speed_costs,
        terrain_cell.is_water,
        terrain_cell.is_cliff_like,
        terrain_cell.is_rough,
        terrain_cell.is_road,
        terrain_cell.ground_walk_blocked,
        terrain_cell.build_blocked,
    );
    cleared_resource_for_slope
        || old_overlay_zone_type != terrain_cell.overlay_zone_type
        || old != new
}

fn restore_pristine_land(cell: &mut crate::map::resolved_terrain::ResolvedTerrainCell) {
    cell.land_type = cell.base_land_type;
    cell.yr_cell_land_type = cell.base_yr_cell_land_type;
    cell.terrain_class = cell.base_terrain_class;
    cell.speed_costs = cell.base_speed_costs;
    let base_land = LandType::from_index(cell.base_land_type);
    cell.is_water = base_land.is_some_and(LandType::is_water);
    cell.is_cliff_like = base_land.is_some_and(LandType::is_cliff_like)
        || matches!(cell.base_terrain_class, TerrainClass::Cliff);
    cell.is_rough = base_land.is_some_and(LandType::is_rough);
    cell.is_road = base_land.is_some_and(LandType::is_road);
}

fn apply_overlay_land(
    cell: &mut crate::map::resolved_terrain::ResolvedTerrainCell,
    land: LandType,
    speed_costs: Option<SpeedCostProfile>,
) {
    cell.land_type = land.as_index();
    cell.yr_cell_land_type = land.as_index();
    cell.terrain_class = land.terrain_class();
    cell.speed_costs = speed_costs.unwrap_or_default();
    cell.is_water = land.is_water();
    cell.is_cliff_like = land.is_cliff_like();
    cell.is_rough = land.is_rough();
    cell.is_road = land.is_road();
}

/// A combat-emitted request to damage a wall overlay at a specific cell.
///
/// Sentinel value `damage == u16::MAX` represents forced destruction (bypasses
/// the probabilistic gate inside `damage_wall_overlay`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WallDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: u16,
}

/// Result of a wall damage attempt.
#[derive(Debug, Clone, Default)]
pub struct WallDamageResult {
    /// Cells where overlay_data changed (need re-render).
    pub changed_cells: Vec<(u16, u16)>,
    /// Cells where wall was fully destroyed (need zone rebuild + render removal).
    pub destroyed_cells: Vec<(u16, u16)>,
}

/// Damage a wall overlay, matching gamemd.exe CellClass::DestroyOverlay (0x00480CB0).
///
/// 1. Random damage check against Strength
/// 2. Increment damage level (upper nibble of overlay_data)
/// 3. At penultimate damage level: chain-damage cardinal neighbors
/// 4. At full destruction: clear overlay, add to destroyed list
///
/// `damage == u16::MAX` bypasses the random check (forced destruction).
pub fn damage_wall_overlay(
    overlay_grid: &mut OverlayGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
    damage: u16,
    rng: &mut crate::sim::rng::SimRng,
) -> WallDamageResult {
    let mut result = WallDamageResult::default();
    damage_wall_recursive(overlay_grid, registry, rx, ry, damage, rng, &mut result);
    result
}

fn damage_wall_recursive(
    grid: &mut OverlayGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
    damage: u16,
    rng: &mut crate::sim::rng::SimRng,
    result: &mut WallDamageResult,
) {
    let cell = *grid.cell(rx, ry);
    let Some(overlay_id) = cell.overlay_id else {
        return;
    };
    let Some(flags) = registry.flags(overlay_id) else {
        return;
    };
    if !flags.wall {
        return;
    }

    // Random damage check (gamemd): when damage < Strength, draw
    // RandomRanged(0, Strength) — INCLUSIVE on the high end, range [0, Strength]
    // — and apply damage only when roll < damage; otherwise no effect. The
    // engine uses `< damage` (so roll == damage is a no-op) and the inclusive
    // top, both of which differ from an exclusive `[0, Strength-1]` / `>` test.
    if damage != u16::MAX && flags.strength > 0 && damage < flags.strength {
        let roll = rng.next_range_u32_inclusive(0, flags.strength as u32) as u16;
        if roll >= damage {
            return;
        }
    }

    // Increment damage level (upper nibble).
    let new_data = cell.overlay_data.wrapping_add(0x10);
    let damage_level = new_data >> 4;

    // At penultimate damage level: chain-damage cardinal neighbors.
    if flags.damage_levels > 2 && damage_level == (flags.damage_levels as u8).saturating_sub(1) {
        const CARDINAL: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        for (dx, dy) in CARDINAL {
            let nx = rx as i32 + dx;
            let ny = ry as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let (nx, ny) = (nx as u16, ny as u16);
            let neighbor = *grid.cell(nx, ny);
            if neighbor.overlay_id == Some(overlay_id) && (neighbor.overlay_data >> 4) == 0 {
                damage_wall_recursive(grid, registry, nx, ny, 200, rng, result);
            }
        }
    }

    // Check if fully destroyed.
    let penultimate = flags.damage_levels.saturating_sub(1);
    let connected = new_data & 0x0F != 0;
    let retain = u16::from(damage_level) < penultimate
        || (u16::from(damage_level) == penultimate && connected);
    if damage != u16::MAX && retain {
        // Not fully destroyed — just update damage level.
        grid.set_overlay_data(rx, ry, new_data);
        result.changed_cells.push((rx, ry));
        return;
    }

    // Full destruction.
    grid.clear_overlay(rx, ry);
    result.destroyed_cells.push((rx, ry));
}

/// Outcome of `recompute_wall_connectivity_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecomputeResult {
    /// Cell was not a wall, or no nibble change.
    NoChange,
    /// Connectivity nibble changed; cell remains.
    Updated,
    /// Auto-destruct threshold tripped; cell cleared.
    Destroyed,
}

/// Per-overlay-type byte-value thresholds at which neighbor cleanup destroys an
/// already-damaged isolated wall.
fn auto_destruct_threshold(overlay_id: u8, full_byte: u8) -> bool {
    match overlay_id {
        0x00 => matches!(full_byte, 0x10 | 0x20), // GASAND
        0x02 => matches!(full_byte, 0x20 | 0x30), // GAWALL
        0x1A => matches!(full_byte, 0x20 | 0x30), // NAWALL
        _ => false,
    }
}

/// Refresh one cell's connectivity nibble against its 4 cardinal neighbors,
/// then apply the per-type auto-destruct safety net.
///
/// Same-type-only matching.
pub fn recompute_wall_connectivity_at(
    grid: &mut OverlayGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
) -> RecomputeResult {
    let cell = *grid.cell(rx, ry);
    let Some(overlay_id) = cell.overlay_id else {
        return RecomputeResult::NoChange;
    };
    let Some(flags) = registry.flags(overlay_id) else {
        return RecomputeResult::NoChange;
    };
    if !flags.wall {
        return RecomputeResult::NoChange;
    }

    // Cardinal neighbor connectivity scan. Bit assignment matches existing
    // compute_wall_connectivity: N=0, E=1, S=2, W=3.
    const CARDINAL: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut connectivity: u8 = 0;
    for (bit, (dx, dy)) in CARDINAL.iter().enumerate() {
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;
        if nx < 0 || ny < 0 {
            continue;
        }
        let neighbor = grid.cell(nx as u16, ny as u16);
        if neighbor.overlay_id == Some(overlay_id) {
            connectivity |= 1 << bit;
        }
    }

    let damage_nibble = cell.overlay_data & 0xF0;
    let new_byte = damage_nibble | connectivity;

    // Auto-destruct threshold fires whenever cleanup runs over an already-
    // damaged isolated wall, even if the connectivity nibble itself is
    // unchanged. Mirrors PostDestructionWallCleanup §5.2.
    if auto_destruct_threshold(overlay_id, new_byte) {
        if matches!(overlay_id, 0x02 | 0x1A) {
            grid.clear_overlay_preserving_owner(rx, ry);
        } else {
            grid.clear_overlay(rx, ry);
        }
        return RecomputeResult::Destroyed;
    }

    if new_byte == cell.overlay_data {
        return RecomputeResult::NoChange;
    }

    grid.set_overlay_data(rx, ry, new_byte);
    RecomputeResult::Updated
}

/// Refresh a newly stamped wall and its four cardinal neighbors only.
pub fn refresh_wall_connectivity_after_placement(
    grid: &mut OverlayGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
) {
    const CARDINAL: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let _ = recompute_wall_connectivity_at(grid, registry, rx, ry);
    for (dx, dy) in CARDINAL {
        let nx = i32::from(rx) + dx;
        let ny = i32::from(ry) + dy;
        if nx < 0 || ny < 0 {
            continue;
        }
        let _ = recompute_wall_connectivity_at(grid, registry, nx as u16, ny as u16);
    }
}

/// Reproduce the fixed cleanup fan-out after direct destruction.
///
/// Each cardinal neighbor receives a N/E/S/W/self cross update in table order.
/// Cleanup removals do not recursively expand this fixed scope.
pub fn cleanup_wall_neighbors(
    grid: &mut OverlayGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
) -> Vec<(u16, u16)> {
    const CARDINAL: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    const CROSS: [(i32, i32); 5] = [(0, -1), (1, 0), (0, 1), (-1, 0), (0, 0)];

    let mut destroyed: Vec<(u16, u16)> = Vec::new();
    for (outer_dx, outer_dy) in CARDINAL {
        let cx = i32::from(rx) + outer_dx;
        let cy = i32::from(ry) + outer_dy;
        for (dx, dy) in CROSS {
            let nx = cx + dx;
            let ny = cy + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let nx = nx as u16;
            let ny = ny as u16;
            if nx >= grid.width() || ny >= grid.height() {
                continue;
            }
            if let RecomputeResult::Destroyed =
                recompute_wall_connectivity_at(grid, registry, nx, ny)
            {
                if !destroyed.contains(&(nx, ny)) {
                    destroyed.push((nx, ny));
                }
            }
        }
    }

    destroyed
}

/// 8-direction offsets: N, NE, E, SE, S, SW, W, NW.
const ADJACENT_8: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Static default for out-of-bounds reads.
const DEFAULT_CELL: OverlayCell = OverlayCell {
    overlay_id: None,
    overlay_data: 0,
    wall_owner: None,
};

fn index_of(width: u16, height: u16, rx: u16, ry: u16) -> Option<usize> {
    (rx < width && ry < height).then_some(ry as usize * width as usize + rx as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_is_empty() {
        let grid = OverlayGrid::new(4, 4);
        assert_eq!(grid.cell(0, 0).overlay_id, None);
        assert_eq!(grid.cell(3, 3).overlay_id, None);
    }

    #[test]
    fn gsi_04_07_map_seeded_overlay_is_unowned() {
        let grid = OverlayGrid::from_overlay_entries(
            &[OverlayEntry {
                rx: 1,
                ry: 2,
                overlay_id: 2,
                frame: 0x18,
            }],
            4,
            4,
        );
        let cell = grid.cell(1, 2);
        assert_eq!(cell.overlay_id, Some(2));
        assert_eq!(cell.overlay_data, 0x18);
        assert_eq!(cell.wall_owner, None);
    }

    #[test]
    fn from_overlay_entries_seeds_cells() {
        let entries = vec![
            OverlayEntry {
                rx: 1,
                ry: 2,
                overlay_id: 5,
                frame: 7,
            },
            OverlayEntry {
                rx: 3,
                ry: 0,
                overlay_id: 10,
                frame: 0,
            },
        ];
        let grid = OverlayGrid::from_overlay_entries(&entries, 4, 4);
        assert_eq!(grid.cell(1, 2).overlay_id, Some(5));
        assert_eq!(grid.cell(1, 2).overlay_data, 7);
        assert_eq!(grid.cell(3, 0).overlay_id, Some(10));
        assert_eq!(grid.cell(0, 0).overlay_id, None);
    }

    #[test]
    fn from_overlay_entries_skips_bridge_overlay_bytes() {
        let entries = vec![
            OverlayEntry {
                rx: 1,
                ry: 1,
                overlay_id: 24,
                frame: 3,
            },
            OverlayEntry {
                rx: 2,
                ry: 1,
                overlay_id: 5,
                frame: 7,
            },
        ];

        let grid = OverlayGrid::from_overlay_entries(&entries, 4, 4);

        assert_eq!(
            grid.cell(1, 1).overlay_id,
            None,
            "bridge overlay byte is owned by BridgeRuntimeState"
        );
        assert_eq!(grid.cell(2, 1).overlay_id, Some(5));
    }

    #[test]
    fn gsi_04_09_overlay_pack_init_preserves_all_raw_data_without_dirtying() {
        let entries = [OverlayEntry {
            rx: 1,
            ry: 1,
            overlay_id: 5,
            frame: 7,
        }];
        let data = OverlayDataPack::from_cells([(0, 0, 42), (1, 1, 9)]);
        let mut present = OverlayGrid::from_overlay_packs(&entries, &data, 2, 2);

        assert_eq!(present.cell(0, 0).overlay_id, None);
        assert_eq!(present.cell(0, 0).overlay_data, 42);
        assert_eq!(present.cell(1, 1).overlay_id, Some(5));
        assert_eq!(
            present.cell(1, 1).overlay_data,
            9,
            "raw data pack overwrites the frame stamped with OverlayPack"
        );
        assert!(present.take_dirty_cells().is_empty());

        let mut missing = OverlayGrid::from_overlay_packs(
            &entries,
            &OverlayDataPack::missing(),
            2,
            2,
        );
        assert_eq!(missing.cell(0, 0).overlay_id, None);
        assert_eq!(missing.cell(0, 0).overlay_data, 0);
        assert_eq!(missing.cell(1, 1).overlay_id, Some(5));
        assert_eq!(missing.cell(1, 1).overlay_data, 7);
        assert!(missing.take_dirty_cells().is_empty());
    }

    #[test]
    fn place_and_clear_overlay() {
        let mut grid = OverlayGrid::new(4, 4);
        grid.place_overlay(2, 2, 42, 11);
        assert_eq!(grid.cell(2, 2).overlay_id, Some(42));
        assert_eq!(grid.cell(2, 2).overlay_data, 11);

        let prev = grid.clear_overlay(2, 2);
        assert_eq!(prev, Some(42));
        assert_eq!(grid.cell(2, 2).overlay_id, None);
    }

    #[test]
    fn set_overlay_data_updates_existing() {
        let mut grid = OverlayGrid::new(4, 4);
        grid.place_overlay(1, 1, 5, 3);
        grid.set_overlay_data(1, 1, 9);
        assert_eq!(grid.cell(1, 1).overlay_data, 9);
        assert_eq!(grid.cell(1, 1).overlay_id, Some(5));
    }

    #[test]
    fn set_overlay_data_noop_on_empty_cell() {
        let mut grid = OverlayGrid::new(4, 4);
        grid.set_overlay_data(1, 1, 9);
        assert_eq!(grid.cell(1, 1).overlay_id, None);
        assert_eq!(grid.cell(1, 1).overlay_data, 0);
    }

    #[test]
    fn out_of_bounds_returns_default() {
        let grid = OverlayGrid::new(2, 2);
        assert_eq!(grid.cell(5, 5).overlay_id, None);
    }

    #[test]
    fn iter_occupied_skips_empty() {
        let mut grid = OverlayGrid::new(3, 3);
        grid.place_overlay(0, 0, 1, 0);
        grid.place_overlay(2, 2, 2, 5);
        let occupied: Vec<_> = grid.iter_occupied().collect();
        assert_eq!(occupied.len(), 2);
        assert_eq!(occupied[0].0, 0); // rx
        assert_eq!(occupied[0].1, 0); // ry
        assert_eq!(occupied[1].0, 2);
        assert_eq!(occupied[1].1, 2);
    }

    #[test]
    fn place_overlay_pushes_dirty() {
        let mut grid = OverlayGrid::new(10, 10);
        assert!(grid.dirty_cells.is_empty());
        grid.place_overlay(3, 4, 7, 0);
        assert_eq!(grid.dirty_cells, vec![(3, 4)]);
    }

    #[test]
    fn clear_overlay_pushes_dirty_when_in_bounds() {
        let mut grid = OverlayGrid::new(10, 10);
        grid.place_overlay(2, 2, 5, 0);
        grid.dirty_cells.clear();
        let prev = grid.clear_overlay(2, 2);
        assert_eq!(prev, Some(5));
        assert_eq!(grid.dirty_cells, vec![(2, 2)]);
    }

    #[test]
    fn clear_overlay_no_push_when_out_of_bounds() {
        let mut grid = OverlayGrid::new(10, 10);
        let prev = grid.clear_overlay(100, 100);
        assert_eq!(prev, None);
        assert!(grid.dirty_cells.is_empty());
    }

    #[test]
    fn set_overlay_data_pushes_only_when_cell_has_overlay() {
        let mut grid = OverlayGrid::new(10, 10);
        // No overlay → no push.
        grid.set_overlay_data(1, 1, 42);
        assert!(grid.dirty_cells.is_empty());
        // With overlay → push.
        grid.place_overlay(1, 1, 9, 0);
        grid.dirty_cells.clear();
        grid.set_overlay_data(1, 1, 42);
        assert_eq!(grid.dirty_cells, vec![(1, 1)]);
    }

    #[test]
    fn take_dirty_cells_returns_and_clears() {
        let mut grid = OverlayGrid::new(10, 10);
        grid.place_overlay(0, 0, 1, 0);
        grid.place_overlay(1, 1, 2, 0);
        let drained = grid.take_dirty_cells();
        assert_eq!(drained, vec![(0, 0), (1, 1)]);
        assert!(grid.dirty_cells.is_empty());
        // Second take returns empty.
        assert!(grid.take_dirty_cells().is_empty());
    }

    #[test]
    fn dirty_cells_preserve_push_order() {
        // Determinism: drain must iterate in push order.
        let mut grid = OverlayGrid::new(10, 10);
        grid.place_overlay(5, 5, 1, 0);
        grid.place_overlay(0, 0, 2, 0);
        grid.place_overlay(9, 3, 3, 0);
        assert_eq!(grid.take_dirty_cells(), vec![(5, 5), (0, 0), (9, 3)]);
    }

    fn single_cell_terrain(
        base_land_type: u8,
        speed_costs: crate::rules::terrain_rules::SpeedCostProfile,
        is_water: bool,
        base_ground_walk_blocked: bool,
    ) -> crate::map::resolved_terrain::ResolvedTerrainGrid {
        use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
        use crate::rules::terrain_rules::TerrainClass;

        let terrain_class = if is_water {
            TerrainClass::Water
        } else {
            TerrainClass::Clear
        };
        ResolvedTerrainGrid::from_cells(
            1,
            1,
            vec![ResolvedTerrainCell {
                rx: 0,
                ry: 0,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: true,
                tileset_index: None,
                land_type: base_land_type,
                yr_cell_land_type: base_land_type,
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class,
                speed_costs,
                is_water,
                is_cliff_like: base_ground_walk_blocked,
                is_rough: false,
                is_road: false,
                accepts_smudge: true,
                allows_tiberium: false,
                height_in_pixels: 0,
                variant: 0,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: base_ground_walk_blocked,
                terrain_object_blocks: false,
                terrain_object_occupation: None,
                overlay_blocks: false,
                overlay_zone_type: None,
                outside_playfield: false,
                zone_type: zone_class::GROUND,
                base_ground_walk_blocked,
                base_build_blocked: false,
                base_land_type,
                base_yr_cell_land_type: base_land_type,
                base_terrain_class: terrain_class,
                base_speed_costs: speed_costs,
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
                bridge_layer: None,
                bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                tube_index: None,
                radar_left: [0; 3],
                radar_right: [0; 3],
                has_damaged_data: false,
                bridgehead_anchor_class_at_load: None,
            }],
        )
    }

    fn gsi_04_04_registry() -> OverlayTypeRegistry {
        use crate::rules::ini_parser::IniFile;

        let ini = IniFile::from_str(
            "\
[OverlayTypes]
0=CRUSHWALL
1=WALLFLAG
2=ZEROLAND
3=ROCKFLAG
4=RUBBLE
5=GATEOVL
6=ONELAND
7=ROADKEEP
8=ROADRESTORE
9=WALLLAND
10=RAILLAND
11=ORE
12=WATERKEEP
13=ROUGHKEEP
14=STOCKORE
15=WALLORE
16=RAILORE
[Clear]
Wheel=100%
[Road]
Foot=80%
Track=70%
Wheel=55%
[Water]
Float=66%
Wheel=44%
[Rock]
Wheel=0%
[Wall]
Wheel=40%
[Tiberium]
Foot=90%
Track=70%
Wheel=80%
[Rough]
Foot=77%
Wheel=33%
[Ice]
Wheel=1%
[Railroad]
Wheel=60%
[CRUSHWALL]
Crushable=yes
Wall=yes
NoUseTileLandType=no
[WALLFLAG]
Wall=yes
NoUseTileLandType=no
[ZEROLAND]
Land=Rock
NoUseTileLandType=no
[ROCKFLAG]
IsARock=yes
NoUseTileLandType=no
[RUBBLE]
IsRubble=yes
NoUseTileLandType=no
[GATEOVL]
Gate=yes
NoUseTileLandType=no
[ONELAND]
Land=Ice
NoUseTileLandType=no
[ROADKEEP]
Land=Road
NoUseTileLandType=yes
[ROADRESTORE]
Land=Road
NoUseTileLandType=no
[WALLLAND]
Land=Wall
NoUseTileLandType=no
[RAILLAND]
Land=Railroad
NoUseTileLandType=no
[ORE]
Tiberium=yes
NoUseTileLandType=no
[WATERKEEP]
Land=Water
NoUseTileLandType=yes
[ROUGHKEEP]
Land=Rough
NoUseTileLandType=yes
[STOCKORE]
Tiberium=yes
[WALLORE]
Tiberium=yes
Land=Wall
NoUseTileLandType=no
[RAILORE]
Tiberium=yes
Land=Railroad
NoUseTileLandType=no
",
        );
        OverlayTypeRegistry::from_ini(&ini, None)
    }

    #[test]
    fn gsi_04_04_recalc_zone_type_overlay_priority_matches_gamemd() {
        use crate::map::resolved_terrain::zone_class;
        use crate::rules::ini_parser::IniFile;
        use crate::rules::terrain_rules::LandType;
        use crate::rules::terrain_rules::SpeedCostProfile;

        let ini = IniFile::from_str(
            "\
[OverlayTypes]
0=SANDBAG
1=HARDWALL
2=ROCKOVL
3=RUBBLE
[Clear]
Wheel=100%
[Rock]
Wheel=0%
[SANDBAG]
Crushable=yes
Wall=yes
Land=Clear
[HARDWALL]
Wall=yes
Land=Clear
[ROCKOVL]
Land=Rock
[RUBBLE]
IsRubble=yes
",
        );
        let registry = OverlayTypeRegistry::from_ini(&ini, None);
        let clear = LandType::Clear.as_index();

        let cases = [
            (0, zone_class::CRUSHABLE, false),
            (1, zone_class::WALL, true),
            (2, zone_class::IMPASSABLE, true),
            (3, zone_class::GROUND, false),
        ];
        for (overlay_id, expected_zone, expected_blocks) in cases {
            let mut overlay_grid = OverlayGrid::new(1, 1);
            overlay_grid.place_overlay(0, 0, overlay_id, 0);
            let mut terrain = single_cell_terrain(clear, SpeedCostProfile::default(), false, false);
            recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 0, 0);
            let cell = terrain.cell(0, 0).expect("cell");
            assert_eq!(cell.zone_type, expected_zone, "overlay id {overlay_id}");
            assert_eq!(
                cell.overlay_blocks, expected_blocks,
                "overlay id {overlay_id}"
            );
        }
    }

    #[test]
    fn gsi_04_04_recalc_zone_type_water_and_beach_precede_speed_threshold() {
        use crate::map::resolved_terrain::zone_class;
        use crate::rules::terrain_rules::LandType;
        use crate::rules::terrain_rules::SpeedCostProfile;

        let registry = OverlayTypeRegistry::empty();
        let mut overlay_grid = OverlayGrid::new(1, 1);
        let mut zero_wheel = SpeedCostProfile::default();
        zero_wheel.wheel = Some(0);

        let mut water = single_cell_terrain(LandType::Water.as_index(), zero_wheel, true, true);
        recalc_overlay_passability(&mut overlay_grid, &mut water, &registry, 0, 0);
        assert_eq!(water.cell(0, 0).unwrap().zone_type, zone_class::WATER);

        let mut beach = single_cell_terrain(LandType::Beach.as_index(), zero_wheel, false, false);
        recalc_overlay_passability(&mut overlay_grid, &mut beach, &registry, 0, 0);
        assert_eq!(beach.cell(0, 0).unwrap().zone_type, zone_class::BEACH);

        let mut rock = single_cell_terrain(LandType::Rock.as_index(), zero_wheel, false, true);
        recalc_overlay_passability(&mut overlay_grid, &mut rock, &registry, 0, 0);
        assert_eq!(rock.cell(0, 0).unwrap().zone_type, zone_class::IMPASSABLE);
    }

    #[test]
    fn gsi_04_04_recalc_zone_exact_priority_thresholds_and_non_predicates() {
        use crate::map::resolved_terrain::zone_class;
        use crate::rules::terrain_rules::{LandType, SpeedCostProfile};

        let registry = gsi_04_04_registry();
        let clear = LandType::Clear.as_index();
        let mut clear_speed = SpeedCostProfile::default();
        clear_speed.wheel = Some(100);

        for (overlay_id, expected_zone, expected_blocks) in [
            (0, zone_class::CRUSHABLE, false),
            (1, zone_class::WALL, true),
            (2, zone_class::IMPASSABLE, true),
            (3, zone_class::IMPASSABLE, true),
            (4, zone_class::GROUND, false),
        ] {
            let mut overlay_grid = OverlayGrid::new(1, 1);
            overlay_grid.place_overlay(0, 0, overlay_id, 0);
            let mut terrain = single_cell_terrain(clear, clear_speed, false, false);
            terrain.cell_mut(0, 0).unwrap().terrain_object_occupation = Some(7);
            terrain.cell_mut(0, 0).unwrap().terrain_object_blocks = true;
            recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 0, 0);
            let cell = terrain.cell(0, 0).expect("priority cell");
            assert_eq!(cell.zone_type, expected_zone, "overlay {overlay_id}");
            assert_eq!(cell.overlay_blocks, expected_blocks, "overlay {overlay_id}");
        }

        let mut zero_wheel = SpeedCostProfile::default();
        zero_wheel.wheel = Some(0);
        let mut overlay_grid = OverlayGrid::new(1, 1);
        overlay_grid.place_overlay(0, 0, 4, 0);
        let mut rubble_on_water =
            single_cell_terrain(LandType::Water.as_index(), zero_wheel, true, false);
        recalc_overlay_passability(&mut overlay_grid, &mut rubble_on_water, &registry, 0, 0);
        assert_eq!(
            rubble_on_water.cell(0, 0).unwrap().zone_type,
            zone_class::GROUND,
            "rubble is a terminal overlay result, not absence"
        );

        overlay_grid.place_overlay(0, 0, 5, 0);
        let mut gate_on_water =
            single_cell_terrain(LandType::Water.as_index(), zero_wheel, true, false);
        recalc_overlay_passability(&mut overlay_grid, &mut gate_on_water, &registry, 0, 0);
        assert_eq!(
            gate_on_water.cell(0, 0).unwrap().zone_type,
            zone_class::WATER
        );

        overlay_grid.place_overlay(0, 0, 6, 0);
        let mut exact_one_overlay = single_cell_terrain(clear, clear_speed, false, false);
        recalc_overlay_passability(&mut overlay_grid, &mut exact_one_overlay, &registry, 0, 0);
        assert_eq!(
            exact_one_overlay.cell(0, 0).unwrap().zone_type,
            zone_class::GROUND,
            "overlay Land wheel=1 is not the exact-zero overlay predicate"
        );

        let mut empty_grid = OverlayGrid::new(1, 1);
        for (wheel, expected) in [
            (0, zone_class::IMPASSABLE),
            (1, zone_class::IMPASSABLE),
            (2, zone_class::GROUND),
        ] {
            let mut speed = SpeedCostProfile::default();
            speed.wheel = Some(wheel);
            let mut terrain = single_cell_terrain(clear, speed, false, false);
            recalc_overlay_passability(&mut empty_grid, &mut terrain, &registry, 0, 0);
            assert_eq!(
                terrain.cell(0, 0).unwrap().zone_type,
                expected,
                "wheel={wheel}"
            );
        }

        for (occupation, expected_zone, expected_blocks) in [
            (7, zone_class::WALL, true),
            (4, zone_class::BUILDING, true),
            (0, zone_class::BUILDING, false),
        ] {
            let mut object = single_cell_terrain(clear, clear_speed, false, false);
            let object_cell = object.cell_mut(0, 0).unwrap();
            object_cell.terrain_object_occupation = Some(occupation);
            object_cell.terrain_object_blocks = expected_blocks;
            recalc_overlay_passability(&mut empty_grid, &mut object, &registry, 0, 0);
            assert_eq!(object.cell(0, 0).unwrap().zone_type, expected_zone);
        }

        let mut blocked_base = single_cell_terrain(clear, clear_speed, false, true);
        recalc_overlay_passability(&mut empty_grid, &mut blocked_base, &registry, 0, 0);
        assert_eq!(
            blocked_base.cell(0, 0).unwrap().zone_type,
            zone_class::GROUND,
            "base_ground_walk_blocked is not a RecalcZoneType predicate"
        );
    }

    #[test]
    fn gsi_04_04_runtime_land_writer_retains_profiles_and_restores_pristine_tile() {
        use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};

        let registry = gsi_04_04_registry();
        let mut base_speed = SpeedCostProfile::default();
        base_speed.wheel = Some(100);

        for (overlay_id, land, class) in [
            (7, LandType::Road, TerrainClass::Road),
            (9, LandType::Wall, TerrainClass::Wall),
            (10, LandType::Railroad, TerrainClass::Railroad),
            (12, LandType::Water, TerrainClass::Water),
            (13, LandType::Rough, TerrainClass::Rough),
        ] {
            let mut overlay_grid = OverlayGrid::new(1, 1);
            overlay_grid.place_overlay(0, 0, overlay_id, 0);
            let mut terrain =
                single_cell_terrain(LandType::Clear.as_index(), base_speed, false, false);
            recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 0, 0);
            let cell = terrain.cell(0, 0).expect("retained land cell");
            assert_eq!(cell.land_type, land.as_index(), "overlay {overlay_id}");
            assert_eq!(cell.terrain_class, class, "overlay {overlay_id}");
            assert_eq!(
                cell.speed_costs,
                registry
                    .flags(overlay_id)
                    .unwrap()
                    .land_speed_costs
                    .unwrap(),
                "overlay {overlay_id} profile"
            );
            assert_eq!(cell.is_water, land == LandType::Water);
            assert_eq!(cell.is_road, land == LandType::Road);
            assert_eq!(cell.is_rough, land == LandType::Rough);
        }

        let mut overlay_grid = OverlayGrid::new(1, 1);
        overlay_grid.place_overlay(0, 0, 8, 0);
        let mut terrain = single_cell_terrain(LandType::Clear.as_index(), base_speed, false, false);
        recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 0, 0);
        assert_eq!(
            terrain.cell(0, 0).unwrap().land_type,
            LandType::Clear.as_index()
        );

        overlay_grid.place_overlay(0, 0, 7, 0);
        recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 0, 0);
        assert_eq!(
            terrain.cell(0, 0).unwrap().land_type,
            LandType::Road.as_index()
        );
        overlay_grid.clear_overlay(0, 0);
        recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 0, 0);
        let restored = terrain.cell(0, 0).unwrap();
        assert_eq!(restored.land_type, LandType::Clear.as_index());
        assert_eq!(restored.terrain_class, TerrainClass::Clear);
        assert_eq!(restored.speed_costs, base_speed);
        assert!(!restored.is_water);
        assert!(!restored.is_road);
        assert!(!restored.is_rough);
    }

    #[test]
    fn gsi_04_04_runtime_tiberium_slope_branches_clear_exact_overlays() {
        use crate::rules::terrain_rules::{LandType, SpeedCostProfile};

        let registry = gsi_04_04_registry();
        let mut base_speed = SpeedCostProfile::default();
        base_speed.wheel = Some(100);

        for slope in [0, 4, 5] {
            let mut overlay_grid = OverlayGrid::new(1, 1);
            overlay_grid.place_overlay(0, 0, 11, 7);
            let mut terrain =
                single_cell_terrain(LandType::Clear.as_index(), base_speed, false, false);
            terrain.cell_mut(0, 0).unwrap().slope_type = slope;

            assert!(recalc_overlay_passability(
                &mut overlay_grid,
                &mut terrain,
                &registry,
                0,
                0
            ));

            let cell = terrain.cell(0, 0).unwrap();
            if slope < 5 {
                assert_eq!(overlay_grid.cell(0, 0).overlay_id, Some(11));
                assert_eq!(cell.land_type, LandType::Tiberium.as_index());
            } else {
                assert_eq!(overlay_grid.cell(0, 0), &OverlayCell::default());
                assert_eq!(cell.land_type, LandType::Clear.as_index());
            }
        }

        for (overlay_id, land) in [
            (14, LandType::Tiberium),
            (15, LandType::Wall),
            (16, LandType::Railroad),
        ] {
            for slope in [0, 1, 4, 5] {
                let mut overlay_grid = OverlayGrid::new(1, 1);
                overlay_grid.place_overlay(0, 0, overlay_id, 7);
                let mut terrain =
                    single_cell_terrain(LandType::Clear.as_index(), base_speed, false, false);
                terrain.cell_mut(0, 0).unwrap().slope_type = slope;

                assert!(recalc_overlay_passability(
                    &mut overlay_grid,
                    &mut terrain,
                    &registry,
                    0,
                    0
                ));
                if slope == 0 {
                    assert_eq!(overlay_grid.cell(0, 0).overlay_id, Some(overlay_id));
                    assert_eq!(terrain.cell(0, 0).unwrap().land_type, land.as_index());
                } else {
                    assert_eq!(overlay_grid.cell(0, 0), &OverlayCell::default());
                    assert_eq!(
                        terrain.cell(0, 0).unwrap().land_type,
                        LandType::Clear.as_index()
                    );
                }
            }
        }
    }

    /// Round-trip on tiberium overlay add/remove: a fresh-spread or TIBTRE-spawned
    /// ore cell must inherit Tiberium-mode `land_type` / `terrain_class` /
    /// `speed_costs`, and a harvested-to-zero ore cell must revert to the
    /// underlying terrain values. Regression test for the §10 parity bug where
    /// runtime ore placement bypassed RecalcAttributes-equivalent logic.
    #[test]
    fn gsi_04_04_tiberium_overlay_round_trip_updates_terrain_metadata() {
        use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
        use crate::rules::ini_parser::IniFile;
        use crate::rules::terrain_rules::LandType;
        use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};

        // Registry with overlay id=0 marked Tiberium=yes (mirrors stock TIB01).
        let ini = IniFile::from_str(
            "[OverlayTypes]\n0=TIB01\n[Tiberium]\nTrack=70%\nFoot=90%\n[TIB01]\nTiberium=yes\n",
        );
        let registry = OverlayTypeRegistry::from_ini(&ini, None);

        // A single Clear-base cell at (5, 5). Underlying values mirror what
        // `resolved_terrain::build()` would produce on a clear-grass tile.
        let clear_lt = LandType::Clear.as_index();
        let base_speed = SpeedCostProfile::default();
        let mut cells = Vec::with_capacity(100);
        for ry in 0..10u16 {
            for rx in 0..10u16 {
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: true,
                    tileset_index: None,
                    land_type: clear_lt,
                    yr_cell_land_type: clear_lt,
                    slope_type: 0,
                    template_height: 0,
                    render_offset_x: 0,
                    render_offset_y: 0,
                    terrain_class: TerrainClass::Clear,
                    speed_costs: base_speed,
                    is_water: false,
                    is_cliff_like: false,
                    is_rough: false,
                    is_road: false,
                    accepts_smudge: true,
                    allows_tiberium: false,
                    height_in_pixels: 0,
                    variant: 0,
                    has_ramp: false,
                    canonical_ramp: None,
                    ground_walk_blocked: false,
                    terrain_object_blocks: false,
                    terrain_object_occupation: None,
                    overlay_blocks: false,
                    overlay_zone_type: None,
                    outside_playfield: false,
                    zone_type: zone_class::GROUND,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    base_land_type: clear_lt,
                    base_yr_cell_land_type: clear_lt,
                    base_terrain_class: TerrainClass::Clear,
                    base_speed_costs: base_speed,
                    build_blocked: false,
                    has_bridge_deck: false,
                    bridge_walkable: false,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0; 3],
                    radar_right: [0; 3],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        let mut terrain = ResolvedTerrainGrid::from_cells(10, 10, cells);

        // Install a distinct Tiberium-mode speed profile so we can prove the
        // round-trip actually copies it (not the same default).
        let tib_speed = SpeedCostProfile {
            foot: Some(90),
            track: Some(70),
            wheel: Some(100),
            float: Some(100),
            amphibious: Some(100),
            float_beach: Some(100),
            hover: Some(100),
        };
        let mut overlay_grid = OverlayGrid::new(10, 10);
        let tib_lt = LandType::Tiberium.as_index();

        // 1. Place ore overlay → recalc must flip cell to Tiberium-mode.
        overlay_grid.place_overlay(5, 5, 0, 3);
        let changed = recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 5, 5);
        assert!(changed, "tiberium placement must report a change");
        let idx = 5 * 10 + 5;
        let cell = &terrain.cells[idx];
        assert_eq!(cell.land_type, tib_lt, "land_type → Tiberium");
        assert_eq!(cell.yr_cell_land_type, tib_lt, "yr_cell_land_type → 5");
        assert_eq!(cell.terrain_class, TerrainClass::Tiberium);
        assert_eq!(
            cell.speed_costs, tib_speed,
            "speed_costs sourced from [Tiberium]"
        );
        assert_eq!(cell.zone_type, zone_class::GROUND);
        assert!(!cell.overlay_blocks);

        // 2. Remove ore overlay (harvested to zero) → recalc must restore base values.
        overlay_grid.clear_overlay(5, 5);
        let changed = recalc_overlay_passability(&mut overlay_grid, &mut terrain, &registry, 5, 5);
        assert!(changed, "tiberium removal must report a change");
        let cell = &terrain.cells[idx];
        assert_eq!(cell.land_type, clear_lt, "land_type → underlying Clear");
        assert_eq!(cell.yr_cell_land_type, clear_lt);
        assert_eq!(cell.terrain_class, TerrainClass::Clear);
        assert_eq!(
            cell.speed_costs, base_speed,
            "speed_costs → underlying default"
        );
        assert_eq!(cell.zone_type, zone_class::GROUND);
        assert!(!cell.overlay_blocks);
    }
}

#[cfg(test)]
mod recompute_tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    /// Build a registry whose overlay_id=2 maps to GAWALL (for auto-destruct
    /// threshold matching). Filler entries at id 0 and 1 get Wall=yes too so
    /// tests can exercise other ids if needed.
    fn make_wall_registry() -> OverlayTypeRegistry {
        let text = "\
[OverlayTypes]
0=GASAND
1=CYCL
2=GAWALL
[GASAND]
Wall=yes
Strength=400
[CYCL]
Wall=yes
Strength=400
[GAWALL]
Wall=yes
Strength=400
";
        let ini = IniFile::from_str(text);
        OverlayTypeRegistry::from_ini(&ini, None)
    }

    fn make_retail_stage_registry() -> OverlayTypeRegistry {
        let rules = IniFile::from_str(
            "[OverlayTypes]\n\
             0=GASAND\n\
             1=CYCL\n\
             2=GAWALL\n\
             [GASAND]\n\
             Wall=yes\n\
             Armor=wood\n\
             Strength=100\n\
             [CYCL]\n\
             [GAWALL]\n\
             Wall=yes\n\
             Armor=concrete\n\
             Strength=100\n",
        );
        let art = IniFile::from_str(
            "[GASAND]\n\
             DamageLevels=2\n\
             [GAWALL]\n\
             DamageLevels=3\n",
        );
        OverlayTypeRegistry::from_ini(&rules, Some(&art))
    }

    #[test]
    fn gsi_04_07_damage_stage_connection_forced_and_chain_order() {
        let registry = make_retail_stage_registry();
        let mut rng = crate::sim::rng::SimRng::new(7);

        let mut isolated = OverlayGrid::new(12, 12);
        isolated.place_overlay(5, 5, 0, 0);
        let result = damage_wall_overlay(&mut isolated, &registry, 5, 5, 100, &mut rng);
        assert_eq!(result.destroyed_cells, vec![(5, 5)]);

        let mut connected = OverlayGrid::new(12, 12);
        connected.place_overlay(5, 5, 0, 0x02);
        connected.place_overlay(6, 5, 0, 0x08);
        let result = damage_wall_overlay(&mut connected, &registry, 5, 5, 100, &mut rng);
        assert!(result.destroyed_cells.is_empty());
        assert_eq!(connected.cell(5, 5).overlay_data, 0x12);
        let result = damage_wall_overlay(&mut connected, &registry, 5, 5, 100, &mut rng);
        assert_eq!(result.destroyed_cells, vec![(5, 5)]);

        let mut forced_chain = OverlayGrid::new(12, 12);
        forced_chain.place_overlay(5, 5, 2, 0x1A);
        forced_chain.place_overlay(6, 5, 2, 0x08);
        let result = damage_wall_overlay(&mut forced_chain, &registry, 5, 5, u16::MAX, &mut rng);
        assert!(result.destroyed_cells.contains(&(5, 5)));
        assert_eq!(
            forced_chain.cell(6, 5).overlay_data & 0xF0,
            0x10,
            "forced terminal removal chains into pristine neighbors first"
        );
    }

    #[test]
    fn gsi_04_07_cleanup_has_fixed_scope_and_preserves_concrete_owner_quirk() {
        let registry = make_retail_stage_registry();
        let owner = crate::sim::intern::InternedId::from_index(9);
        let mut grid = OverlayGrid::new(16, 8);
        grid.place_owned_wall(4, 3, 2, 0x20, owner);
        grid.place_owned_wall(6, 3, 2, 0x20, owner);

        let destroyed = cleanup_wall_neighbors(&mut grid, &registry, 2, 3);
        assert_eq!(destroyed, vec![(4, 3)]);
        assert_eq!(grid.cell(4, 3).overlay_id, None);
        assert_eq!(grid.cell(4, 3).overlay_data, 0);
        assert_eq!(
            grid.cell(4, 3).wall_owner,
            Some(owner),
            "GAWALL isolated cleanup leaves its stale owner"
        );
        assert_eq!(
            grid.cell(6, 3).overlay_id,
            Some(2),
            "cleanup does not recursively flood beyond the fixed cross fan-out"
        );

        grid.place_owned_wall(4, 4, 0, 0x10, owner);
        let _ = cleanup_wall_neighbors(&mut grid, &registry, 2, 4);
        assert_eq!(grid.cell(4, 4).overlay_id, None);
        assert_eq!(grid.cell(4, 4).wall_owner, None);
    }

    #[test]
    fn recompute_no_op_for_non_wall_cell() {
        let mut grid = OverlayGrid::new(10, 10);
        let reg = OverlayTypeRegistry::empty();
        let r = recompute_wall_connectivity_at(&mut grid, &reg, 5, 5);
        assert_eq!(r, RecomputeResult::NoChange);
    }

    #[test]
    fn wall_damage_inclusive_range_and_ge_boundary() {
        // RandomRanged(0, 400) on seed=1: span=400, mask=0x1FF; first masked
        // sample 0x78B76ED5 & 0x1FF = 0xD5 = 213 (<= 400 -> accepted). gamemd
        // applies damage only when roll < damage, so with roll == 213:
        //   damage=213 -> roll >= damage -> NO-OP (the old `>` test applied here)
        //   damage=214 -> roll <  damage -> damage applied
        let reg = make_wall_registry();

        // No-op exactly at roll == damage (the key old-vs-new discriminator).
        let mut grid = OverlayGrid::new(10, 10);
        grid.place_overlay(5, 5, 2, 0);
        let mut rng = crate::sim::rng::SimRng::new(1);
        let r = damage_wall_overlay(&mut grid, &reg, 5, 5, 213, &mut rng);
        assert!(
            r.changed_cells.is_empty() && r.destroyed_cells.is_empty(),
            "roll == damage must be a no-op"
        );
        assert_eq!(
            grid.cell(5, 5).overlay_data,
            0,
            "no nibble change at roll == damage"
        );
        assert_eq!(
            grid.cell(5, 5).overlay_id,
            Some(2),
            "wall intact at roll == damage"
        );

        // Damage applied just below the boundary (same seed -> same roll 213).
        // (This registry leaves DamageLevels at its low default, so a single hit
        // fully destroys the wall rather than only bumping the nibble — either
        // outcome proves damage was applied, which is what the boundary tests.)
        let mut grid = OverlayGrid::new(10, 10);
        grid.place_overlay(5, 5, 2, 0);
        let mut rng = crate::sim::rng::SimRng::new(1);
        let r = damage_wall_overlay(&mut grid, &reg, 5, 5, 214, &mut rng);
        assert!(
            !r.changed_cells.is_empty() || !r.destroyed_cells.is_empty(),
            "roll < damage must apply damage"
        );
        let cell = grid.cell(5, 5);
        assert!(
            cell.overlay_id != Some(2) || cell.overlay_data != 0,
            "wall must be destroyed or have its damage nibble advanced"
        );
    }

    #[test]
    fn recompute_updates_nibble_when_neighbor_changes() {
        let mut grid = OverlayGrid::new(10, 10);
        let reg = make_wall_registry();
        // Place two adjacent GAWALL at (5,5) and (6,5). Initialize (5,5) with
        // stale connectivity (0b0001 = N) so the recompute changes it to
        // 0b0010 (E neighbor only).
        grid.place_overlay(5, 5, 2, 0b0001);
        grid.place_overlay(6, 5, 2, 0);
        let r = recompute_wall_connectivity_at(&mut grid, &reg, 5, 5);
        assert_eq!(r, RecomputeResult::Updated);
        assert_eq!(grid.cell(5, 5).overlay_data, 0b0010);
    }

    #[test]
    fn recompute_destroys_isolated_max_damage_gawall() {
        let mut grid = OverlayGrid::new(10, 10);
        let reg = make_wall_registry();
        // Isolated GAWALL with damage stage 3 (= 0x30) and connectivity 0 → 0x30
        // matches the auto-destruct threshold.
        grid.place_overlay(5, 5, 2, 0x30);
        let r = recompute_wall_connectivity_at(&mut grid, &reg, 5, 5);
        assert_eq!(r, RecomputeResult::Destroyed);
        assert_eq!(grid.cell(5, 5).overlay_id, None);
    }

    #[test]
    fn recompute_keeps_max_damage_wall_when_connected() {
        let mut grid = OverlayGrid::new(10, 10);
        let reg = make_wall_registry();
        // GAWALL at (5,5) with damage stage 3 PLUS connection to E neighbor.
        // Connected → byte = 0x30 | 0b0010 = 0x32 → not in auto-destruct set → kept.
        grid.place_overlay(5, 5, 2, 0x30);
        grid.place_overlay(6, 5, 2, 0);
        let r = recompute_wall_connectivity_at(&mut grid, &reg, 5, 5);
        assert_eq!(r, RecomputeResult::Updated);
        assert_eq!(grid.cell(5, 5).overlay_data, 0x32);
        assert!(grid.cell(5, 5).overlay_id.is_some());
    }

    #[test]
    fn cleanup_chain_does_not_destroy_intact_segment() {
        let mut grid = OverlayGrid::new(10, 10);
        let reg = make_wall_registry();
        // Row of 3 GAWALL at (4,5), (5,5), (6,5), all at max damage stage 3.
        // Connectivity nibbles: (4,5)=0b0010 (E only), (5,5)=0b1010 (E+W),
        // (6,5)=0b1000 (W only). Full bytes: 0x32, 0x3A, 0x38.
        grid.place_overlay(4, 5, 2, 0x32);
        grid.place_overlay(5, 5, 2, 0x3A);
        grid.place_overlay(6, 5, 2, 0x38);
        // Destroy the leftmost cell directly (simulating damage_wall_overlay's destroy).
        grid.clear_overlay(4, 5);
        // Run cleanup starting from (4, 5).
        let destroyed = cleanup_wall_neighbors(&mut grid, &reg, 4, 5);
        // (5,5) loses W connection → byte = 0x32 → not in {0x20, 0x30} → keeps.
        // (6,5) keeps W connection (5,5 still present) → byte = 0x38 → keeps.
        assert!(destroyed.is_empty());
        assert!(grid.cell(5, 5).overlay_id.is_some());
        assert!(grid.cell(6, 5).overlay_id.is_some());
    }

    #[test]
    fn cleanup_chain_terminates_via_visited_set() {
        let mut grid = OverlayGrid::new(10, 10);
        let reg = make_wall_registry();
        // 5-cell row of GAWALL at max damage; recompute initial connectivity
        // via the helper to set up coherent state, then destroy the leftmost
        // cell and run cleanup. Test passes if it returns in finite time.
        let cells: [(u16, u16); 5] = [(2, 5), (3, 5), (4, 5), (5, 5), (6, 5)];
        for &(rx, ry) in &cells {
            grid.place_overlay(rx, ry, 2, 0x30);
        }
        for &(rx, ry) in &cells {
            let _ = recompute_wall_connectivity_at(&mut grid, &reg, rx, ry);
        }
        // After initial recompute, cells with neighbors survive; isolated ones
        // would already be gone. The endpoints had only one neighbor → byte
        // 0x32 or 0x38, both kept. Middle three: 0x3A, kept.
        // Now destroy (2,5) and trigger cleanup.
        grid.clear_overlay(2, 5);
        let _ = cleanup_wall_neighbors(&mut grid, &reg, 2, 5);
        // No assertion: termination is the test.
    }

    #[test]
    fn cleanup_handles_oob_neighbors() {
        let mut grid = OverlayGrid::new(10, 10);
        let reg = make_wall_registry();
        grid.place_overlay(0, 0, 2, 0x30);
        grid.clear_overlay(0, 0);
        // Cleanup at (0,0) — neighbors include (-1, 0) and (0, -1) which are OOB.
        let _ = cleanup_wall_neighbors(&mut grid, &reg, 0, 0);
        // No panic = pass.
    }
}
