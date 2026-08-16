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

use crate::map::houses::{HouseAllianceMap, are_houses_friendly};
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::pathfinding::PathGrid;

/// Bit flag: cell has been seen at least once (persists across ticks).
const FLAG_REVEALED: u8 = 0x01;
/// Bit flag: cell is currently in line of sight (rebuilt each tick).
const FLAG_VISIBLE: u8 = 0x02;
/// Bit flag: cell is covered by an enemy gap generator (rebuilt each tick).
/// Entities on gap-covered cells are hidden from the local player; terrain
/// renders black (treated as unrevealed).
const FLAG_GAP_COVERED: u8 = 0x04;
/// Bit flag: cell is covered by a friendly (own/allied) gap generator (rebuilt
/// each tick). Terrain renders half-bright fog rather than black.
const FLAG_GAP_FOG: u8 = 0x08;

/// Serialized CellClass visibility fields for one owner/cell projection.
///
/// The renderer continues to consume the compact `OwnerVisibility::cells`
/// bitmap. This state preserves the native transition contract underneath it:
/// signed shroud counters, the split CellClass flag words, and the two signed
/// occlusion caches. It is kept per owner because VERA's visibility authority
/// is per house, while the retail CellClass helpers read the current player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellVisibilityRuntime {
    /// CellClass +0x130. `-1` is the native inactive sentinel.
    pub shroud_counter: i32,
    /// CellClass +0x134 upper clamp for `shroud_counter`.
    pub gap_shroud_counter: i32,
    /// CellClass +0x12C: ground visible (`0x08`) and ground cache open (`0x10`).
    pub alt_flags: u8,
    /// CellClass +0x140 visibility/fog transition flags.
    pub flags: u32,
    /// CellClass +0x120 ground occlusion cache.
    pub visibility: i8,
    /// CellClass +0x121 fog/air occlusion cache.
    pub foggedness: i8,
}

impl Default for CellVisibilityRuntime {
    fn default() -> Self {
        Self {
            shroud_counter: -1,
            gap_shroud_counter: i32::MAX,
            alt_flags: 0,
            flags: 0,
            visibility: 0,
            foggedness: 0,
        }
    }
}

/// Ordered side-effect boundary of the native map-cell visibility update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellVisibilityEvent {
    /// TacticalClass::RegisterCellAsVisible: the redraw invalidation boundary.
    RegisterCellAsVisible,
    /// MapClass::RevealCheck, always before an eligible clean-fog operation.
    RevealCheck,
    /// CellClass::CleanFog on the first mapped/fog-display transition only.
    CleanFog,
}

impl CellVisibilityRuntime {
    const ALT_GROUND_VISIBLE: u8 = 0x08;
    const ALT_GROUND_OPEN: u8 = 0x10;
    const FLAG_FOG_OPEN: u32 = 0x01;
    const FLAG_MAPPED: u32 = 0x02;
    const FLAG_SHROUD_POSITIVE: u32 = 0x20;
    const FLAG_TRANSIENT: u32 = 0x40;
    const FLAG_FOGGED_OBJECT_SNAPSHOT: u32 = 0x400000;

    fn set_fogged_object_snapshot(&mut self, present: bool) {
        if present {
            self.flags |= Self::FLAG_FOGGED_OBJECT_SNAPSHOT;
        } else {
            self.flags &= !Self::FLAG_FOGGED_OBJECT_SNAPSHOT;
        }
    }

    /// Native `CellClass::IncreaseShroudCounter`: no redraw side effect.
    pub fn increase_shroud_counter(&mut self) {
        let old = self.shroud_counter;
        if self.shroud_counter == -1 {
            self.shroud_counter = 0;
        }
        self.shroud_counter = self.shroud_counter.saturating_add(1);
        self.shroud_counter = self.shroud_counter.min(self.gap_shroud_counter);
        if old <= 0 && self.shroud_counter > 0 {
            self.flags |= Self::FLAG_SHROUD_POSITIVE;
        }
    }

    /// Native `CellClass::ReduceShroudCounter`, including the `1 -> -1` edge.
    /// Counter mutation itself deliberately emits no redraw event.
    pub fn reduce_shroud_counter(&mut self) {
        if self.shroud_counter == 1 {
            self.shroud_counter = 0;
        }
        self.shroud_counter = self.shroud_counter.saturating_sub(1);
        if self.shroud_counter > 0 {
            return;
        }
        if self.alt_flags & (Self::ALT_GROUND_VISIBLE | Self::ALT_GROUND_OPEN)
            == (Self::ALT_GROUND_VISIBLE | Self::ALT_GROUND_OPEN)
        {
            self.flags &= !Self::FLAG_SHROUD_POSITIVE;
        } else {
            self.alt_flags |= Self::ALT_GROUND_VISIBLE | Self::ALT_GROUND_OPEN;
        }
    }

    /// Native `CellClass::Unshroud` flag projection; it is not a traversal.
    pub fn unshroud(&mut self) {
        self.alt_flags |= Self::ALT_GROUND_VISIBLE | Self::ALT_GROUND_OPEN;
        if self.shroud_counter > 0 {
            self.flags |= Self::FLAG_SHROUD_POSITIVE;
        }
    }

    /// Apply the full MapCell-style projection. The event callback makes the
    /// `RevealCheck`-before-`CleanFog` order explicit without coupling sim to
    /// renderer invalidation or the unported fogged-object render records.
    pub fn map_visible(&mut self, fog_of_war: bool, mut emit: impl FnMut(CellVisibilityEvent)) {
        let had_mapped = self.flags & Self::FLAG_MAPPED != 0;
        let before = *self;

        self.flags = (self.flags & !(Self::FLAG_MAPPED | Self::FLAG_TRANSIENT)) | Self::FLAG_MAPPED;
        self.increase_shroud_counter();
        self.alt_flags |= Self::ALT_GROUND_VISIBLE | Self::ALT_GROUND_OPEN;
        self.visibility = -1;
        self.flags |= Self::FLAG_FOG_OPEN;
        self.foggedness = -1;

        if *self != before {
            emit(CellVisibilityEvent::RegisterCellAsVisible);
            emit(CellVisibilityEvent::RevealCheck);
        }
        if !had_mapped && fog_of_war {
            // CellClass::CleanFog clears the snapshot bit before freeing the
            // shared footprint records; the record store is intentionally not
            // represented until its owner/link lifetime has a Rust authority.
            self.flags &= !Self::FLAG_FOGGED_OBJECT_SNAPSHOT;
            emit(CellVisibilityEvent::CleanFog);
        }
    }
}

/// RA2 hard-caps effective sight at 10 cells. Going past 10 was a crash
/// in the original engine — we clamp to this limit for compatibility.
pub const MAX_SIGHT_RANGE: u16 = 10;

/// World height of one terrain level, in leptons. Same retail value the
/// coordinate-Z evaluator uses (`util::lepton::LEPTONS_PER_LEVEL`); kept local
/// because the reveal kernel's input is a height, not a bridge or range query.
const LEPTONS_PER_HEIGHT_LEVEL: i32 = 104;

/// Screen height of one isometric cell, in pixels. The reveal centre is pushed
/// toward isometric north by however many whole cells of screen lift the
/// viewer's height buys, so the revealed disc sits under the sprite rather than
/// under its ground shadow.
const CELL_HEIGHT_PX: i32 = crate::map::terrain::TILE_HEIGHT as i32;

/// Percentage of base sight added per elevation step.
///
/// Original: `TechnoClass::UpdateReveal` derives the step count by dividing the
/// object's world Z in leptons by `[General] LeptonsPerSightIncrease`, scales it
/// by ten, then computes `Sight * (1 + 0.01 * that)` — so the combination is
/// multiplicative off world Z, not an additive per-terrain-level bonus, and one
/// step is +10%. Verified 2026-08-04.
const ELEVATION_SIGHT_PERCENT_PER_STEP: i32 = 10;

/// Screen lift, in whole pixels, for an object at `height_leptons` above the
/// map plane. Reproduces the engine's height→screen conversion including its
/// extra-pixel threshold and the `+0.5` that precedes a truncating float→int.
fn height_lift_px(height_leptons: i32) -> i32 {
    crate::util::native_x87::adjust_for_z_standard(height_leptons)
}

/// Cells the reveal spiral's centre is shifted toward isometric north.
///
/// For a ground object standing on terrain level `L` this is `L / 2`, matching
/// the shorthand this used to be written as; for an airborne object it is
/// driven by its lepton altitude instead, which at stock `FlightLevel=1500`
/// works out to 7 cells — the same distance its sprite is drawn above its
/// ground cell.
fn iso_height_shift_cells(height_leptons: i32) -> i32 {
    height_lift_px(height_leptons) / CELL_HEIGHT_PX
}

/// Height above the map plane, in leptons, for one entity.
///
/// The engine keeps a single 3-D world coordinate per object and feeds its Z to
/// both the reveal-centre shift and the line-of-sight viewer level, so terrain
/// elevation and flight altitude are one quantity here too. The precedence
/// between the three things that can hold an object up mirrors
/// `render::locomotor_visual` exactly, so the shroud and the sprite cannot
/// disagree about where the object is.
fn entity_height_leptons(entity: &crate::sim::game_entity::GameEntity) -> i32 {
    use crate::rules::locomotor_type::LocomotorKind;
    use crate::sim::movement::locomotor::MovementLayer;

    let terrain: i32 = i32::from(entity.position.z) * LEPTONS_PER_HEIGHT_LEVEL;
    let above_ground: i32 = if let Some(state) = entity.parachute_state.as_ref() {
        state.altitude.to_num::<i32>()
    } else if let Some(state) = entity.rocket_state.as_ref() {
        state.altitude.to_num::<i32>()
    } else {
        match entity.locomotor.as_ref() {
            Some(loco)
                if loco.layer == MovementLayer::Air && loco.kind != LocomotorKind::Rocket =>
            {
                loco.altitude.to_num::<i32>()
            }
            _ => 0,
        }
    };
    terrain + above_ground
}

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
    /// CellClass-like transition state aligned with `cells`. Old snapshots
    /// deserialize this as empty and are expanded lazily before the next tick.
    #[serde(default)]
    cell_runtime: Vec<CellVisibilityRuntime>,
    /// Number of current-frame visibility contributors per cell. This lets the
    /// next recompute apply the same number of native counter reductions that
    /// this frame admitted; it is serialized because a snapshot can occur
    /// between visibility rebuilds.
    #[serde(default)]
    visibility_marks: Vec<u16>,
}

impl Default for OwnerVisibility {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            width: 0,
            height: 0,
            cell_runtime: Vec::new(),
            visibility_marks: Vec::new(),
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
            cell_runtime: vec![CellVisibilityRuntime::default(); len],
            visibility_marks: vec![0; len],
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

    /// Returns true if the cell is covered by a friendly gap generator this tick.
    pub fn is_gap_fog(&self, rx: u16, ry: u16) -> bool {
        self.index(rx, ry)
            .map_or(false, |i| self.cells[i] & FLAG_GAP_FOG != 0)
    }

    /// Mark a cell as both visible and revealed.
    pub fn mark_visible(&mut self, rx: u16, ry: u16) {
        self.mark_visible_with_fog_of_war(rx, ry, true);
    }

    /// Same as [`Self::mark_visible`], with the scenario fog rule carried to
    /// the CellClass first-map transition.
    pub fn mark_visible_with_fog_of_war(&mut self, rx: u16, ry: u16, fog_of_war: bool) {
        if let Some(i) = self.index(rx, ry) {
            self.ensure_cell_runtime();
            self.cell_runtime[i].map_visible(fog_of_war, |_| {});
            self.visibility_marks[i] = self.visibility_marks[i].saturating_add(1);
            self.cells[i] |= FLAG_VISIBLE | FLAG_REVEALED;
        }
    }

    /// Clear all visible flags while preserving revealed flags.
    /// Called each tick by `recompute_owner_visibility_in_place` so existing
    /// grids can be reused without reallocation.
    pub fn clear_all_visible(&mut self) {
        self.ensure_cell_runtime();
        for ((cell, runtime), marks) in self
            .cells
            .iter_mut()
            .zip(&mut self.cell_runtime)
            .zip(&mut self.visibility_marks)
        {
            if *cell & FLAG_VISIBLE != 0 {
                for _ in 0..(*marks).max(1) {
                    runtime.reduce_shroud_counter();
                }
            }
            *marks = 0;
            *cell &= !(FLAG_VISIBLE | FLAG_GAP_COVERED | FLAG_GAP_FOG);
        }
    }

    /// Clear only transient Gap Generator flags while preserving line of sight
    /// and persisted map knowledge.
    fn clear_gap_flags(&mut self) {
        for cell in &mut self.cells {
            *cell &= !(FLAG_GAP_COVERED | FLAG_GAP_FOG);
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

    /// Serialized CellClass-style visibility state, in the same row-major
    /// order as `cells`, for deterministic hashing and snapshot inspection.
    pub fn cell_runtime_raw(&self) -> &[CellVisibilityRuntime] {
        &self.cell_runtime
    }

    /// Current-frame counter contributions aligned with [`Self::cells_raw`].
    pub fn visibility_marks_raw(&self) -> &[u16] {
        &self.visibility_marks
    }

    fn set_fogged_object_snapshot(&mut self, rx: u16, ry: u16, present: bool) {
        let Some(index) = self.index(rx, ry) else {
            return;
        };
        self.ensure_cell_runtime();
        self.cell_runtime[index].set_fogged_object_snapshot(present);
    }

    fn ensure_cell_runtime(&mut self) {
        if self.cell_runtime.len() != self.cells.len() {
            self.cell_runtime
                .resize(self.cells.len(), CellVisibilityRuntime::default());
        }
        if self.visibility_marks.len() != self.cells.len() {
            self.visibility_marks.resize(self.cells.len(), 0);
        }
    }

    #[cfg(test)]
    fn resized_preserving_state(&self, width: u16, height: u16) -> Self {
        let mut expanded = Self::new(width, height);
        for ry in 0..self.height.min(height) {
            for rx in 0..self.width.min(width) {
                let old = ry as usize * self.width as usize + rx as usize;
                let new = ry as usize * width as usize + rx as usize;
                expanded.cells[new] = self.cells[old];
                if let Some(runtime) = self.cell_runtime.get(old) {
                    expanded.cell_runtime[new] = *runtime;
                }
                if let Some(marks) = self.visibility_marks.get(old) {
                    expanded.visibility_marks[new] = *marks;
                }
            }
        }
        expanded
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

/// Stable Rust identity for one native FoggedObjectClass allocation.
pub type FoggedObjectId = u64;

/// Shared frozen-building footprint ownership. Rendering payload is
/// deliberately absent until `FreezeInFog`'s draw-record fields are closed;
/// this record only represents the proven cross-cell lifetime contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoggedObjectFootprintRecord {
    pub id: FoggedObjectId,
    /// VERA's per-house projection of native `CurrentPlayer` fog state.
    pub viewer: InternedId,
    pub source_entity_id: u64,
    /// Occupy-list order, with the anchor already applied and invalid cells
    /// omitted by the caller that owns map bounds.
    pub occupied_cells: Vec<(u16, u16)>,
}

/// Nonserialized merged-visibility cache for one owner (F10 `FogViewCache`).
///
/// Presentation-only: discarded by every snapshot load (serde skip) and
/// rebuilt before the first tactical render; never part of save bytes or the
/// state hash, so building it any number of times cannot affect determinism.
#[derive(Debug, Clone, Default)]
pub(crate) struct FogViewCache {
    /// The owner the merged grid was built for, plus the merged grid. All
    /// alliance-aware queries (is_cell_visible, edge masks) use this for
    /// O(1) lookups instead of iterating all owners per cell.
    pub(crate) merged: Option<(InternedId, OwnerVisibility)>,
    /// Bumps on every rebuild. The fog mask renderer and minimap dirty-gate
    /// on this runtime counter, never on the serialized wire shadow.
    pub(crate) generation: u64,
}

/// Global fog/shroud state keyed by owner name.
///
/// Stores per-owner visibility grids plus a lazily-computed merged view cache
/// for fast alliance-aware queries. The cache is built via
/// `build_merged_for()` and then used by `is_cell_visible`, edge masks, etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FogState {
    pub width: u16,
    pub height: u16,
    pub by_owner: BTreeMap<InternedId, OwnerVisibility>,
    pub alliances: HouseAllianceMap,
    /// The merged local-owner view (F10): nonserialized presentation cache.
    #[serde(skip)]
    pub(crate) view_cache: FogViewCache,
    /// Version-81 wire-compatibility shadow (F10): the exact serialized `u64`
    /// slot the pre-split view generation occupied, still updated in lockstep
    /// with the cache so round-trip bytes stay identical. Render consumes
    /// `view_generation()`, never this field. Do not bump SNAPSHOT_VERSION
    /// for this; retiring the slot waits for the next planned bump.
    pub generation_wire_shadow: u64,
    /// Native CellClass::FoggedObjects vectors, keyed by viewer and cell. IDs
    /// may be shared across every cell in one building footprint.
    #[serde(default)]
    pub fogged_object_cells: BTreeMap<(InternedId, u16, u16), Vec<FoggedObjectId>>,
    /// Owning allocation table for shared fogged-object IDs.
    #[serde(default)]
    pub fogged_objects: BTreeMap<FoggedObjectId, FoggedObjectFootprintRecord>,
    /// Allocation cursor for the Rust-stable counterpart of native pointers.
    #[serde(default)]
    pub next_fogged_object_id: FoggedObjectId,
    /// Native signed-word SensorsOfHouses counters, row-major per house.
    #[serde(default)]
    pub sensors_by_house: BTreeMap<InternedId, Vec<i16>>,
    /// Native CellClass::CloakedByHouses words, row-major by cell. House
    /// selection uses the original x86-masked bit index (`h & 31`).
    #[serde(default)]
    pub cloaked_by_houses: Vec<u32>,
}

impl FogState {
    /// Insert one shared frozen-building footprint record. Named location:
    /// `BuildingClass::FreezeInFog` installs the same FoggedObjectClass pointer
    /// into every occupy-list cell, not one allocation per cell.
    pub fn insert_fogged_object_footprint(
        &mut self,
        viewer: InternedId,
        receiver_cell: (u16, u16),
        source_entity_id: u64,
        occupied_cells: Vec<(u16, u16)>,
    ) -> FoggedObjectId {
        let id = self.next_fogged_object_id.max(1);
        self.next_fogged_object_id = id.wrapping_add(1);
        let record = FoggedObjectFootprintRecord {
            id,
            viewer,
            source_entity_id,
            occupied_cells,
        };
        for &(rx, ry) in &record.occupied_cells {
            self.fogged_object_cells
                .entry((viewer, rx, ry))
                .or_default()
                .push(id);
        }
        self.fogged_objects.insert(id, record);
        if self.width > 0 && self.height > 0 {
            self.by_owner
                .entry(viewer)
                .or_insert_with(|| OwnerVisibility::new(self.width, self.height))
                .set_fogged_object_snapshot(receiver_cell.0, receiver_cell.1, true);
        }
        id
    }

    /// Clear one cell's vector in native reverse order. Each shared record is
    /// first unlinked by exact ID from all footprint cells, then returned to
    /// the caller in destruction/invalidation order. Other empty vectors stay
    /// allocated, matching `CellClass::ClearFoggedObjects @ 0x004802D0`.
    pub fn clear_fogged_objects_at(
        &mut self,
        viewer: InternedId,
        rx: u16,
        ry: u16,
    ) -> Vec<FoggedObjectFootprintRecord> {
        if let Some(vis) = self.by_owner.get_mut(&viewer) {
            vis.set_fogged_object_snapshot(rx, ry, false);
        }
        let Some(mut ids) = self.fogged_object_cells.remove(&(viewer, rx, ry)) else {
            return Vec::new();
        };
        let mut removed = Vec::with_capacity(ids.len());
        while let Some(id) = ids.pop() {
            let Some(record) = self.fogged_objects.remove(&id) else {
                continue;
            };
            for &(record_rx, record_ry) in &record.occupied_cells {
                if (record_rx, record_ry) == (rx, ry) {
                    continue;
                }
                if let Some(cell_ids) = self
                    .fogged_object_cells
                    .get_mut(&(viewer, record_rx, record_ry))
                {
                    cell_ids.retain(|candidate| *candidate != id);
                }
            }
            removed.push(record);
        }
        removed
    }

    /// Recreate CellClass sensor storage for current map bounds. Sensor
    /// producers can then use add/remove calls without owning visibility maps.
    pub fn reset_sensor_counts(&mut self) {
        self.sensors_by_house.clear();
    }

    /// Recreate the serialized CellClass cloak-owner words for current map
    /// bounds without inventing a cloak-generator mask producer.
    pub fn reset_cloaked_by_houses(&mut self) {
        self.cloaked_by_houses.clear();
        self.cloaked_by_houses
            .resize(usize::from(self.width) * usize::from(self.height), 0);
    }

    /// Native `CellClass::SetCloakedByHouse @ 0x00487110`.
    pub fn set_cloaked_by_house(&mut self, house_index: u8, rx: u16, ry: u16) -> bool {
        let Some(word) = self.cloak_word_mut(rx, ry) else {
            return false;
        };
        let bit = 1_u32 << (u32::from(house_index) & 31);
        let changed = *word & bit == 0;
        *word |= bit;
        changed
    }

    /// Native `CellClass::ClearCloakedByHouse @ 0x00487130`.
    pub fn clear_cloaked_by_house(&mut self, house_index: u8, rx: u16, ry: u16) -> bool {
        let Some(word) = self.cloak_word_mut(rx, ry) else {
            return false;
        };
        let bit = 1_u32 << (u32::from(house_index) & 31);
        let changed = *word & bit != 0;
        *word &= !bit;
        changed
    }

    /// Native `CellClass::IsSensedByHouse @ 0x004870B0`; the pinned label is
    /// stale and this tests cloak-generator ownership, not sensor coverage.
    pub fn is_cloaked_by_house(&self, house_index: u8, rx: u16, ry: u16) -> bool {
        let Some(word) = self.cloak_word(rx, ry) else {
            return false;
        };
        let bit = 1_u32 << (u32::from(house_index) & 31);
        word & bit != 0
    }

    fn cloak_word(&self, rx: u16, ry: u16) -> Option<u32> {
        if rx >= self.width || ry >= self.height {
            return None;
        }
        let index = usize::from(ry) * usize::from(self.width) + usize::from(rx);
        self.cloaked_by_houses.get(index).copied()
    }

    fn cloak_word_mut(&mut self, rx: u16, ry: u16) -> Option<&mut u32> {
        if rx >= self.width || ry >= self.height {
            return None;
        }
        let cell_count = usize::from(self.width) * usize::from(self.height);
        if self.cloaked_by_houses.len() != cell_count {
            self.reset_cloaked_by_houses();
        }
        let index = usize::from(ry) * usize::from(self.width) + usize::from(rx);
        self.cloaked_by_houses.get_mut(index)
    }

    /// Native `FootClass::Sensors_AddAt @ 0x004DE7B0`: outer-Y/inner-X strict
    /// circle and signed-word increment. Returned cells are the exact ordered
    /// boundary where native forces resident objects through virtual `+0x420`.
    pub fn sensors_add_at(
        &mut self,
        house: InternedId,
        center: (u16, u16),
        radius: u16,
    ) -> Vec<(u16, u16)> {
        self.update_sensor_circle(house, center, radius, true)
    }

    /// Paired `FootClass::Sensors_RemoveAt @ 0x004DE940` decrement walk.
    pub fn sensors_remove_at(
        &mut self,
        house: InternedId,
        center: (u16, u16),
        radius: u16,
    ) -> Vec<(u16, u16)> {
        self.update_sensor_circle(house, center, radius, false)
    }

    fn update_sensor_circle(
        &mut self,
        house: InternedId,
        center: (u16, u16),
        radius: u16,
        add: bool,
    ) -> Vec<(u16, u16)> {
        let radius = i32::from(radius);
        if radius <= 0 || self.width == 0 || self.height == 0 {
            return Vec::new();
        }
        let cell_count = usize::from(self.width) * usize::from(self.height);
        let counters = self
            .sensors_by_house
            .entry(house)
            .or_insert_with(|| vec![0; cell_count]);
        if counters.len() != cell_count {
            counters.clear();
            counters.resize(cell_count, 0);
        }
        let mut touched = Vec::new();
        for dy in -radius..radius {
            for dx in -radius..radius {
                if dx * dx + dy * dy >= radius * radius {
                    continue;
                }
                let cell_x = i32::from(center.0) + dx;
                let cell_y = i32::from(center.1) + dy;
                if cell_x < 0
                    || cell_y < 0
                    || cell_x >= i32::from(self.width)
                    || cell_y >= i32::from(self.height)
                {
                    continue;
                }
                let rx = cell_x as u16;
                let ry = cell_y as u16;
                let index = usize::from(ry) * usize::from(self.width) + usize::from(rx);
                if add {
                    counters[index] = counters[index].wrapping_add(1);
                } else if counters[index] > 0 {
                    counters[index] = counters[index].wrapping_sub(1);
                } else {
                    continue;
                }
                touched.push((rx, ry));
            }
        }
        touched
    }

    pub fn has_sensor_for_house(&self, house: InternedId, rx: u16, ry: u16) -> bool {
        if rx >= self.width || ry >= self.height {
            return false;
        }
        let index = usize::from(ry) * usize::from(self.width) + usize::from(rx);
        self.sensors_by_house
            .get(&house)
            .and_then(|counters| counters.get(index))
            .is_some_and(|count| *count > 0)
    }

    /// Native `CellClass::DrawObjectsCloaked`: no observer-mode bypass.
    pub fn draw_objects_cloaked(
        &self,
        current_player: Option<InternedId>,
        object_owner: InternedId,
        object_owner_index: u8,
        rx: u16,
        ry: u16,
    ) -> bool {
        let Some(current_player) = current_player else {
            return false;
        };
        if !self.is_cloaked_by_house(object_owner_index, rx, ry) {
            return false;
        }
        current_player == object_owner || !self.has_sensor_for_house(current_player, rx, ry)
    }

    /// Build a merged visibility grid for the given owner and all their allies.
    /// Call once per tick (or when the local owner changes). Subsequent calls
    /// to `is_cell_visible`, `is_cell_revealed`, and edge mask methods will
    /// use this merged grid for O(1) lookups.
    ///
    /// Reuses the previous merged buffer when dimensions haven't changed to
    /// avoid per-tick allocation.
    pub fn build_merged_for(&mut self, owner: InternedId, interner: &StringInterner) {
        // Reuse existing buffer if dimensions match; otherwise allocate.
        let mut merged = match self.view_cache.merged.take() {
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
        self.view_cache.merged = Some((owner, merged));
        self.view_cache.generation = self.view_cache.generation.wrapping_add(1);
        // Kept in lockstep purely for v81 byte compatibility (see field doc).
        self.generation_wire_shadow = self.generation_wire_shadow.wrapping_add(1);
    }

    /// The runtime view-cache generation render dirty-gates on (F10). Resets
    /// with the cache on every load; never the serialized wire shadow.
    pub fn view_generation(&self) -> u64 {
        self.view_cache.generation
    }

    /// Get the merged visibility grid, falling back to iterating all owners
    /// if no merged grid is available for this owner.
    fn merged_vis(&self, owner: InternedId) -> Option<&OwnerVisibility> {
        if let Some((cached_owner, ref vis)) = self.view_cache.merged {
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

    /// Returns true if the cell is covered by a friendly gap generator for this owner.
    pub fn is_cell_gap_fog(&self, owner: InternedId, rx: u16, ry: u16) -> bool {
        if let Some(vis) = self.merged_vis(owner) {
            return vis.is_gap_fog(rx, ry);
        }
        self.by_owner
            .get(&owner)
            .is_some_and(|s| s.is_gap_fog(rx, ry))
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

    /// Restore shroud after a house loses its last SpySat provider.
    ///
    /// Historical exploration is discarded, but the current Phase-3 techno
    /// sight survives the House rung and remains explored in the same frame.
    /// Transient Gap flags are replaced by the final SpySat -> Gap pass.
    pub(crate) fn restore_shroud_after_spy_sat_loss(&mut self, owner: InternedId) {
        if let Some(visibility) = self.by_owner.get_mut(&owner) {
            for cell in &mut visibility.cells {
                *cell = if *cell & FLAG_VISIBLE != 0 {
                    FLAG_VISIBLE | FLAG_REVEALED
                } else {
                    0
                };
            }
        }
    }

    /// Clear the transient enemy/friendly Gap result on every viewer plane.
    pub fn clear_gap_flags(&mut self) {
        for visibility in self.by_owner.values_mut() {
            visibility.clear_gap_flags();
        }
    }

    /// Lift unexplored shroud for one viewer without granting current sight.
    pub fn reveal_all_for_owner(&mut self, owner: InternedId) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let vis = self
            .by_owner
            .entry(owner)
            .or_insert_with(|| OwnerVisibility::new(self.width, self.height));
        for cell in &mut vis.cells {
            *cell |= FLAG_REVEALED;
        }
    }

    /// Lift unexplored shroud only on the supplied allocated map cells.
    pub fn reveal_cells_for_owner<I>(&mut self, owner: InternedId, cells: I)
    where
        I: IntoIterator<Item = (u16, u16)>,
    {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let vis = self
            .by_owner
            .entry(owner)
            .or_insert_with(|| OwnerVisibility::new(self.width, self.height));
        for (rx, ry) in cells {
            if let Some(index) = vis.index(rx, ry) {
                vis.cells[index] |= FLAG_REVEALED;
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
            *state = state.resized_preserving_state(w, h);
        }
        state.mark_visible(rx, ry);
    }
}

/// Configuration for visibility computation, passed to `recompute_owner_visibility`.
pub struct VisionConfig {
    /// Additive sight bonus for veteran+ units (from [General] VeteranSight=).
    /// Default 0 (vanilla RA2 gives no sight bonus from veterancy).
    pub veteran_sight_bonus: i32,
    /// Leptons of elevation per +1 sight cell (from [General] LeptonsPerSightIncrease=).
    /// 256 leptons = 1 z-level. 0 disables the elevation bonus.
    pub leptons_per_sight_increase: i32,
    /// Height-based LOS obstruction (from [General] RevealByHeight=).
    /// When true, terrain 4+ levels above the viewer at the midpoint blocks sight.
    /// Default true (the standard RA2/YR setting).
    pub reveal_by_height: bool,
    /// Scenario `FogOfWar=` governs the first-map `CleanFog` transition. It
    /// does not change the compact visibility bitmap's existing semantics.
    pub fog_of_war: bool,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            veteran_sight_bonus: 0,
            leptons_per_sight_increase: 0,
            reveal_by_height: true,
            fog_of_war: false,
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
    interner: &crate::sim::intern::StringInterner,
) -> FogState {
    let mut fog = FogState::default();
    recompute_owner_visibility_in_place(
        &mut fog, entities, path_grid, alliances, config, None, interner,
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
    height_grid: Option<&[u8]>,
    _interner: &crate::sim::intern::StringInterner,
) {
    // Construction-seeded session bounds are authoritative; the lazy
    // derivation stays only as the fallback for fixture sims built without a
    // descriptor (zero-dim fog).
    let (width, height) = if fog.width > 0 && fog.height > 0 {
        (fog.width, fog.height)
    } else {
        resolve_bounds(entities, path_grid)
    };
    if width == 0 || height == 0 {
        *fog = FogState::default();
        return;
    }

    // First tick or dimension change: recreate all grids (cold path).
    if fog.width != width || fog.height != height {
        fog.by_owner.clear();
        fog.fogged_object_cells.clear();
        fog.fogged_objects.clear();
        fog.next_fogged_object_id = 0;
        fog.reset_sensor_counts();
        fog.width = width;
        fog.height = height;
    } else {
        // Hot path: clear visible flags, preserve revealed.
        for vis in fog.by_owner.values_mut() {
            vis.clear_all_visible();
        }
    }

    fog.alliances = alliances.clone();
    fog.view_cache.merged = None;

    // Batch entities by owner to avoid repeated BTreeMap lookups and String allocations.
    // Each unique owner's grid is looked up once, then all their entities reveal into it.
    for entity in entities.values() {
        // Dying corpses (uninit'd this tick, awaiting the end-of-tick drain)
        // provide no vision — gamemd conceals on death.
        if entity.dying {
            continue;
        }
        // Skip entities inside a transport — they don't provide vision.
        if entity.passenger_role.is_inside_transport() {
            continue;
        }

        let vis = fog
            .by_owner
            .entry(entity.owner)
            .or_insert_with(|| OwnerVisibility::new(width, height));

        let height_leptons: i32 = entity_height_leptons(entity);

        // Elevation raises sight MULTIPLICATIVELY, off the object's world Z in
        // leptons, not additively off its terrain level:
        //   sight = trunc(Sight * (1 + 0.10 * trunc(Z_leptons / LeptonsPerSightIncrease)))
        // At the stock LeptonsPerSightIncrease=2000 no reachable height — not a
        // level-15 plateau, not stock FlightLevel — produces a single step, so
        // this is inert in an ordinary match. It is written as the engine's
        // mechanism rather than folded to a constant because a map or mode INI
        // can lower the key. Guarded against a zero divisor, which the engine
        // does not do (VERA-internal; gamemd equivalent UNCHECKED).
        let base_range: i32 = entity.vision_range as i32;
        let elev_steps: i32 = if config.leptons_per_sight_increase > 0 {
            height_leptons / config.leptons_per_sight_increase
        } else {
            0
        };
        let with_elevation: i32 =
            (base_range * (100 + ELEVATION_SIGHT_PERCENT_PER_STEP * elev_steps)) / 100;
        // Veterancy is multiplicative in the engine and gated on the type owning
        // the sight promotion ability; VERA carries an additive stand-in because
        // the parsed rules value is an integer. Both are inert at the stock
        // `VeteranSight=0.0`, so this only diverges under a mod/map override.
        let vet_bonus: i32 = if entity.veterancy >= 100 {
            config.veteran_sight_bonus
        } else {
            0
        };
        let effective: u16 = ((with_elevation + vet_bonus).max(0) as u16).min(MAX_SIGHT_RANGE);

        reveal_radius_into(
            vis,
            entity.position.rx,
            entity.position.ry,
            effective,
            height_leptons,
            config.reveal_by_height,
            config.fog_of_war,
            height_grid,
            width,
            height,
        );
    }
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

/// Mark all cells within `range` of `(center_rx, center_ry)` as visible+revealed.
///
/// Iterates the engine's reveal spiral table — `REVEAL_SPIRAL[0 .. RING_SIZES[sight]]`
/// — with no special case at any radius. Every entry, ring 10 included, passes
/// through the same height line-of-sight gate.
///
/// ## Elevation Z-shift
/// The spiral is centered on the viewer's *screen* cell, not its raw foot cell.
/// A raised object's sprite renders toward isometric north, so the engine shifts
/// the reveal center by the same whole number of cells to keep the revealed
/// footprint under the sprite. Without this an elevated unit over-reveals toward
/// isometric south, and an aircraft lifts shroud under its shadow instead of
/// under itself. The shift is applied unconditionally (independent of
/// `reveal_by_height`).
///
/// The height-LOS obstruction check is *not* affected by the shift: in the
/// engine the shift cancels out of the obstruction-cell math, leaving it
/// relative to the raw foot cell. We reproduce that by adding `z_shift` back when
/// computing the obstruction cell below.
///
/// `viewer_height_leptons` is the viewer's world Z — terrain elevation plus any
/// flight altitude — because that is the single quantity the engine feeds to
/// both the shift and the LOS viewer level.
fn reveal_radius_into(
    vis: &mut OwnerVisibility,
    center_rx: u16,
    center_ry: u16,
    range: u16,
    viewer_height_leptons: i32,
    reveal_by_height: bool,
    fog_of_war: bool,
    height_grid: Option<&[u8]>,
    width: u16,
    height: u16,
) {
    // Sight 0 reveals nothing at all — not even the viewer's own cell. The
    // engine's reveal kernel returns before the spiral, and its per-object entry
    // point returns earlier still, so the 36 stock `Sight=0` types (fences, map
    // lamps, spy/cargo/paradrop planes) never open a hole in the shroud.
    if range == 0 {
        return;
    }

    let viewer_level = viewer_height_leptons / LEPTONS_PER_HEIGHT_LEVEL;
    let z_shift = iso_height_shift_cells(viewer_height_leptons);
    let cx = i32::from(center_rx) - z_shift;
    let cy = i32::from(center_ry) - z_shift;
    let w = i32::from(width);
    let h = i32::from(height);

    // Clamp range to MAX_SIGHT_RANGE (the original also clamps to 10).
    let clamped = (range as usize).min(MAX_SIGHT_RANGE as usize);
    let spiral_end = REVEAL_RING_SIZES[clamped];

    for i in 0..spiral_end {
        let (dx, dy) = REVEAL_SPIRAL[i];
        let rx = cx + dx as i32;
        let ry = cy + dy as i32;
        if rx >= 0 && rx < w && ry >= 0 && ry < h {
            // Height-based LOS: check whether terrain at the obstruction cell
            // blocks sight. The original engine samples the cell at
            // `foot_target + mirror[i] + (2, 2)` — the per-entry mirror steps one
            // cell back toward the viewer, plus a fixed +2 on each axis baked into
            // the original's obstruction math. The obstruction is relative to the
            // raw foot cell, so we add `z_shift` back to undo the spiral's Z-shift
            // (in the original this cancellation is implicit). If that cell's Level
            // exceeds viewer_level + 3, the target is not revealed (LOS blocked).
            if reveal_by_height {
                if let Some(hg) = height_grid {
                    let (mdx, mdy) = REVEAL_MIRROR[i];
                    let obs_x = rx + mdx as i32 + 2 + z_shift;
                    let obs_y = ry + mdy as i32 + 2 + z_shift;
                    if obs_x >= 0 && obs_x < w && obs_y >= 0 && obs_y < h {
                        let obs_level = hg[(obs_y * w + obs_x) as usize] as i32;
                        if viewer_level + 3 < obs_level {
                            continue; // terrain blocks LOS
                        }
                    }
                }
            }
            vis.mark_visible_with_fog_of_war(rx as u16, ry as u16, fog_of_war);
        }
    }
}

/// Reveal spiral table extracted from the original engine.
/// Each (dx, dy) is a cell offset from the revealing unit's position.
/// Entries are ordered in expanding rings by sight radius.
///
/// Recovered whole from the engine's table initialiser, which writes every
/// entry as a literal `(dy << 16) | dx` store or a two-argument coordinate
/// call — the table itself lives in zero-initialised data, so reading the
/// image gives nothing. Ring membership is exactly
/// `max(|dx|,|dy|) + min(|dx|,|dy|)/2 == r` (truncating division), which
/// independently reproduces all twelve cumulative counts in
/// [`REVEAL_RING_SIZES`].
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
    (4, -8), (5, -8), (-7, -7), (-6, -7), (6, -7), (7, -7), (-7, -6), (7, -6), (-8, -5), (8, -5),
    (-8, -4), (8, -4), (-9, -3), (9, -3), (-9, -2), (9, -2), (-10, -1), (10, -1), (-10, 0),
    (10, 0), (-10, 1), (10, 1), (-9, 2), (9, 2), (-9, 3), (9, 3), (-8, 4), (8, 4), (-8, 5),
    (8, 5), (-7, 6), (7, 6), (-7, 7), (-6, 7), (6, 7), (7, 7), (-5, 8), (-4, 8), (4, 8), (5, 8),
    (-3, 9), (-2, 9), (2, 9), (3, 9), (-1, 10), (0, 10), (1, 10),
];

/// Cumulative entry count for each sight radius 0–10, read from the engine's
/// read-only data. To reveal cells for sight N, iterate
/// `REVEAL_SPIRAL[0..REVEAL_RING_SIZES[N]]`.
///
/// The table continues past this with 369 for sight 11, which the kernel's
/// clamp to 10 makes unreachable for object reveals.
const REVEAL_RING_SIZES: [usize; 11] = [1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309];

/// Mirror/direction table for height-based LOS checks (RevealByHeight).
///
/// Each entry corresponds to the same index in `REVEAL_SPIRAL`. The (mdx, mdy)
/// offset is added to the target cell position to find the obstruction cell — the
/// cell one step closer to the viewer along the line of sight. If that cell's
/// terrain Level exceeds `viewer_level + 3`, LOS is blocked.
///
/// Recovered from the engine's mirror-table initialiser the same way as
/// [`REVEAL_SPIRAL`]. That table stops at 309 entries — one per spiral entry
/// the sight clamp can reach — which is why this one does too.
#[rustfmt::skip]
const REVEAL_MIRROR: [(i8, i8); 309] = [
    // Sight 0: 1 entry
    (0, 0),
    // Sight 1: entries 1..9 (8 new)
    (-1, 1), (0, 1), (1, 1), (1, 0), (-1, 0), (1, -1), (0, -1), (-1, -1),
    // Sight 2: entries 9..21 (12 new)
    (1, 1), (0, 1), (-1, 1), (1, 1), (-1, 1), (1, 0), (-1, 0), (1, -1), (-1, -1), (1, -1),
    (0, -1), (-1, -1),
    // Sight 3: entries 21..37 (16 new)
    (0, 1), (0, 1), (0, 1), (1, 1), (-1, 1), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0),
    (-1, 0), (1, -1), (-1, -1), (0, -1), (0, -1), (0, -1),
    // Sight 4: entries 37..61 (24 new)
    (0, 1), (0, 1), (0, 1), (1, 1), (1, 1), (-1, 1), (-1, 1), (1, 1), (-1, 1), (1, 0),
    (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, -1), (-1, -1), (1, -1), (1, -1), (-1, -1),
    (-1, -1), (0, -1), (0, -1), (0, -1),
    // Sight 5: entries 61..89 (28 new)
    (0, 1), (0, 1), (0, 1), (1, 1), (1, 1), (-1, 1), (-1, 1), (1, 1), (-1, 1), (1, 1),
    (-1, 1), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, -1), (-1, -1), (1, -1),
    (-1, -1), (1, -1), (1, -1), (-1, -1), (-1, -1), (0, -1), (0, -1), (0, -1),
    // Sight 6: entries 89..121 (32 new)
    (0, 1), (0, 1), (0, 1), (1, 1), (0, 1), (0, 1), (-1, 1), (1, 1), (-1, 1), (1, 1),
    (-1, 1), (1, 0), (1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0),
    (-1, 0), (1, -1), (-1, -1), (1, -1), (-1, -1), (1, -1), (0, -1), (0, -1), (-1, -1), (0, -1),
    (0, -1), (0, -1),
    // Sight 7: entries 121..161 (40 new)
    (0, 1), (0, 1), (0, 1), (1, 1), (0, 1), (0, 1), (-1, 1), (1, 1), (1, 1), (-1, 1),
    (-1, 1), (1, 1), (-1, 1), (1, 1), (-1, 1), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0),
    (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, -1), (-1, -1), (1, -1), (-1, -1), (1, -1),
    (1, -1), (-1, -1), (-1, -1), (1, -1), (0, -1), (0, -1), (-1, -1), (0, -1), (0, -1), (0, -1),
    // Sight 8: entries 161..205 (44 new)
    (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (1, 1), (1, 1), (-1, 1),
    (-1, 1), (1, 1), (-1, 1), (1, 1), (-1, 1), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0),
    (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, -1),
    (-1, -1), (1, -1), (-1, -1), (1, -1), (1, -1), (-1, -1), (-1, -1), (0, -1), (0, -1), (0, -1),
    (0, -1), (0, -1), (0, -1), (0, -1),
    // Sight 9: entries 205..253 (48 new)
    (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (1, 1), (1, 1), (-1, 1),
    (-1, 1), (1, 1), (-1, 1), (1, 1), (-1, 1), (1, 1), (-1, 1), (1, 0), (-1, 0), (1, 0),
    (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0),
    (-1, 0), (1, -1), (-1, -1), (1, -1), (-1, -1), (1, -1), (-1, -1), (1, -1), (1, -1), (-1, -1),
    (-1, -1), (0, -1), (0, -1), (0, -1), (0, -1), (0, -1), (0, -1), (0, -1),
    // Sight 10: entries 253..309 (56 new)
    (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (1, 1), (1, 1), (-1, 1),
    (-1, 1), (1, 1), (1, 1), (-1, 1), (-1, 1), (1, 1), (-1, 1), (1, 1), (-1, 1), (1, 1),
    (-1, 1), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, 0),
    (-1, 0), (1, 0), (-1, 0), (1, 0), (-1, 0), (1, -1), (-1, -1), (1, -1), (-1, -1), (1, -1),
    (-1, -1), (1, -1), (1, -1), (-1, -1), (-1, -1), (1, -1), (1, -1), (-1, -1), (-1, -1),
    (0, -1), (0, -1), (0, -1), (0, -1), (0, -1), (0, -1), (0, -1),
];

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
    // Fire-reveal events don't use height-based LOS (matches gamemd), and the
    // event carries no height of its own, so the centre is unshifted.
    reveal_radius_into(
        vis, center_rx, center_ry, range, 0, false, true, None, width, height,
    );
}

/// Materialize active SpySat house latches by marking every synthetic-grid cell
/// **revealed** for those owners. Production world code routes the same write
/// through the native allocated-cell iterator instead. Call after normal vision.
///
/// gamemd's whole-map reveal sets only the explored bit on every cell. It does
/// not create a "currently in sight" state — with `FogOfWar=no` no such per-cell
/// state exists at all — so the uplink lifts the shroud and nothing more. It
/// must not mark cells *visible* here: that layer is VERA-internal and gates
/// combat target acquisition, so writing it map-wide would let every unit
/// acquire across the whole map, which the engine never does.
///
/// Writing it repeatedly is idempotent. The persisted per-house aggregate latch
/// decides activation and last-provider loss; this helper only materializes the
/// active owners and never infers a transition from an absent list entry.
///
/// Takes the owner names whose persisted SpySat latch is active.
pub fn apply_spy_sat(
    fog: &mut FogState,
    spy_sat_owners: &[InternedId],
    _interner: &StringInterner,
) {
    for &owner_id in spy_sat_owners {
        fog.reveal_all_for_owner(owner_id);
    }
}

/// Apply Gap Generator coverage for one tick. Each generator carries its own
/// `GapRadiusInCells`. For every cell in the strict circular footprint
/// `dx*dx + dy*dy < (radius+1)*(radius+1)`:
///   - enemy viewers: clear FLAG_VISIBLE and FLAG_REVEALED, then set
///     FLAG_GAP_COVERED; the cell renders black and its persisted map knowledge
///     stays erased until local sight or a whole-map reveal reaches it again;
///   - friendly viewers (owner + allies): set FLAG_GAP_FOG — the cell renders
///     half-bright fog while keeping the owner's own vision.
/// Call AFTER spy_sat so gap wins in contested areas.
///
/// Takes a list of (owner_name, rx, ry, radius) for each gap generator.
pub fn apply_gap_generators(
    fog: &mut FogState,
    gap_generators: &[(InternedId, u16, u16, i32)],
    interner: &StringInterner,
) {
    let width = fog.width;
    let height = fog.height;
    if width == 0 || height == 0 {
        return;
    }
    for &(gap_owner_id, center_rx, center_ry, radius) in gap_generators {
        if radius <= 0 {
            continue;
        }
        let gap_owner = interner.resolve(gap_owner_id);
        let cx = i32::from(center_rx);
        let cy = i32::from(center_ry);
        // Strict native footprint: accept a cell when dx*dx + dy*dy < (radius+1)^2.
        let threshold = (radius + 1) * (radius + 1);
        let min_x = (cx - radius).max(0);
        let max_x = (cx + radius).min(i32::from(width) - 1);
        let min_y = (cy - radius).max(0);
        let max_y = (cy + radius).min(i32::from(height) - 1);

        for (viewer_id, vis) in fog.by_owner.iter_mut() {
            let viewer = interner.resolve(*viewer_id);
            let friendly = are_houses_friendly(&fog.alliances, gap_owner, viewer);
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let dx = x - cx;
                    let dy = y - cy;
                    if dx * dx + dy * dy >= threshold {
                        continue;
                    }
                    if let Some(i) = vis.index(x as u16, y as u16) {
                        if friendly {
                            vis.cells[i] |= FLAG_GAP_FOG;
                        } else {
                            vis.cells[i] &= !(FLAG_VISIBLE | FLAG_REVEALED);
                            vis.cells[i] |= FLAG_GAP_COVERED;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod vision_tests;

#[cfg(test)]
mod adjust_for_z_tests {
    use super::{height_lift_px, iso_height_shift_cells};

    #[test]
    fn adjust_for_z_reveal_shift_uses_retail_integer_lift() {
        assert_eq!(height_lift_px(104), 15);
        assert_eq!(height_lift_px(256), 37);
        assert_eq!(height_lift_px(727), 104);
        assert_eq!(height_lift_px(728), 105);
        assert_eq!(height_lift_px(1_500), 216);
        assert_eq!(height_lift_px(-400), -56);
        assert_eq!(iso_height_shift_cells(1_500), 7);
        assert_eq!(iso_height_shift_cells(-400), -1);
    }
}
