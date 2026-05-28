//! Ore growth and spread system — data-driven from rules.ini and map INI.
//!
//! Ports the proven RA1 algorithm (MapClass::Logic + CellClass::Grow/Spread_Tiberium)
//! into the RA2 engine's ResourceNode model. All tuning comes from INI files:
//! - rules.ini [General]: GrowthRate, TiberiumGrows, TiberiumSpreads
//! - map INI [Basic]: TiberiumGrowthEnabled
//! - map INI [SpecialFlags]: TiberiumGrows, TiberiumSpreads
//!
//! ## Algorithm (matching RA1 MapClass::Logic)
//! 1. Incremental scan: each tick processes a fraction of the map
//! 2. Collect growth/spread candidates via reservoir sampling
//! 3. When full scan completes: execute growth, then spread
//! 4. Growth = increase ore remaining by one richness level (ore only, not gems)
//! 5. Spread = spawn new ore in a random adjacent empty+walkable cell
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/miner (ResourceNode, ResourceType),
//!   sim/pathfinding (PathGrid), sim/rng (SimRng), rules/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use crate::map::basic::{BasicSection, SpecialFlagsSection};
use crate::map::bridge_facts::{BRIDGE_FLAG_DESTROYED_OR_RAMP, BRIDGE_FLAG_STRUCTURAL};
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::GeneralRules;
use crate::rules::tiberium_type::{TiberiumTypeId, TiberiumTypeRegistry};
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::rng::SimRng;
use crate::util::fixed_math::{SIM_TICK_HZ, SimFixed};

/// Base ore stock per richness level — matches seed_resource_nodes_from_overlays().
const ORE_BASE_PER_LEVEL: u16 = 120;
/// Maximum ore richness = 12 levels (OverlayData 0-11 in RA1).
const MAX_ORE_LEVELS: u16 = 12;
/// Maximum ore `remaining` value (12 levels * 120 per level).
const MAX_ORE_REMAINING: u16 = ORE_BASE_PER_LEVEL * MAX_ORE_LEVELS;
/// Ore must be above this threshold to spread (>6 levels, matching RA1 OverlayData > 6).
const SPREAD_THRESHOLD: u16 = ORE_BASE_PER_LEVEL * 6;
/// Max candidates collected per scan cycle (bounded like RA1's fixed-size arrays).
const MAX_CANDIDATES: usize = 50;
/// Native AddToGrowthQueue priority jitter span.
const GROWTH_QUEUE_PRIORITY_WINDOW: u32 = 50;
const PERCENT_PPM: i64 = 1_000_000;
const GROWTH_BATCH_MIN: u32 = 5;
const GROWTH_BATCH_MAX: u32 = 50;
const SPREAD_BATCH_MIN: u32 = 5;
const SPREAD_BATCH_MAX: u32 = 25;
const TIMER_MULTIPLIER_PPM: u32 = 1_000_000;
const GEM_BASE_PER_LEVEL: u16 = 180;
const SPREAD_GERMINATION_DENSITY: u8 = 3;

/// 8 adjacent directions for spread: N, NE, E, SE, S, SW, W, NW.
const ADJACENT_OFFSETS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Effective ore growth configuration resolved from merged INI sources.
///
/// Constructed once at map load. The resolution order is:
/// map [SpecialFlags] > map [Basic] > rules.ini [General]
/// All flags must be true for growth/spread to be active.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OreGrowthConfig {
    /// Whether ore cells grow denser over time.
    pub grows: bool,
    /// Whether rich ore spreads to adjacent empty cells.
    pub spreads: bool,
    /// Seconds per full map growth scan cycle (from GrowthRate= in minutes, converted
    /// to integer seconds at config construction to avoid f32 in the tick path).
    pub growth_rate_seconds: u32,
}

impl OreGrowthConfig {
    /// Resolve effective config from rules.ini [General] + map [Basic] + map [SpecialFlags].
    ///
    /// Resolution: each flag must be true at ALL levels to be enabled.
    /// GrowthRate comes only from rules.ini (not overridable per-map).
    pub fn from_ini(
        general: &GeneralRules,
        basic: &BasicSection,
        special_flags: &SpecialFlagsSection,
    ) -> Self {
        let grows = general.tiberium_grows
            && basic.tiberium_growth_enabled.unwrap_or(true)
            && special_flags.tiberium_grows.unwrap_or(true);
        let spreads = general.tiberium_spreads && special_flags.tiberium_spreads.unwrap_or(true);
        let growth_rate_minutes = general.growth_rate_minutes.max(0.01);
        // Convert f32 minutes → integer seconds at the INI boundary via
        // fixed-point to avoid platform-dependent f32 multiplication rounding.
        let rate_fixed = SimFixed::saturating_from_num(growth_rate_minutes);
        let growth_rate_seconds =
            (rate_fixed * SimFixed::from_num(60)).to_num::<i32>().max(1) as u32;

        log::info!(
            "OreGrowthConfig: grows={}, spreads={}, rate={}s",
            grows,
            spreads,
            growth_rate_seconds,
        );

        Self {
            grows,
            spreads,
            growth_rate_seconds,
        }
    }

    /// Disabled config — no growth or spread.
    pub fn disabled() -> Self {
        Self {
            grows: false,
            spreads: false,
            growth_rate_seconds: 300, // 5 minutes
        }
    }
}

/// Queued ore growth cell inserted by native-style AddToGrowthQueue callers.
///
/// Native stores queue priority as a float. This keeps the same observable
/// priority shape while leaving execution to an explicit future queue processor.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OreGrowthQueueEntry {
    pub rx: u16,
    pub ry: u16,
    pub priority: f32,
}

/// Native-style spread queue entry inserted by `Reduce_Tiberium` full removal.
///
/// The full queue processor is still being ported; this state captures the
/// deterministic membership/reseed side effect so depletion no longer drops it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OreSpreadQueueEntry {
    pub resource_type: ResourceType,
    pub rx: u16,
    pub ry: u16,
}

/// Native `TiberiumClass` queue/timer state shell.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NativeTiberiumState {
    pub classes: Vec<NativeTiberiumClassState>,
}

/// Per-type native growth/spread scheduler state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NativeTiberiumClassState {
    pub growth_timer: NativeTiberiumTimer,
    pub spread_timer: NativeTiberiumTimer,
    pub growth_heap: Vec<NativeTiberiumQueueEntry>,
    pub spread_heap: Vec<NativeTiberiumQueueEntry>,
    pub growth_bitmap: BTreeSet<(u16, u16)>,
    pub spread_bitmap: BTreeSet<(u16, u16)>,
}

/// CDTimer-shaped fields used by native tiberium drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NativeTiberiumTimer {
    pub start_frame: u32,
    pub interval: u32,
}

/// Heap entry shell for native growth/spread queues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NativeTiberiumQueueEntry {
    pub rx: u16,
    pub ry: u16,
    /// Raw IEEE bits for GameMD's float priority.
    pub priority_bits: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeTiberiumRebuildStats {
    pub growth_entries: usize,
    pub spread_entries: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeGrowthProcessStats {
    pub processor_calls: u32,
    pub attempt_rng_draws: u32,
    pub requested_attempts: u32,
    pub popped_entries: u32,
    pub stale_entries: u32,
    pub grown_entries: u32,
    pub reinserted_entries: u32,
    pub full_clears: u32,
    pub spread_feed_calls: u32,
    pub spread_enqueued_entries: u32,
}

impl NativeGrowthProcessStats {
    fn add(&mut self, other: Self) {
        self.processor_calls += other.processor_calls;
        self.attempt_rng_draws += other.attempt_rng_draws;
        self.requested_attempts += other.requested_attempts;
        self.popped_entries += other.popped_entries;
        self.stale_entries += other.stale_entries;
        self.grown_entries += other.grown_entries;
        self.reinserted_entries += other.reinserted_entries;
        self.full_clears += other.full_clears;
        self.spread_feed_calls += other.spread_feed_calls;
        self.spread_enqueued_entries += other.spread_enqueued_entries;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeSpreadProcessStats {
    pub processor_calls: u32,
    pub budget_rng_draws: u32,
    pub requested_budget: u32,
    pub popped_entries: u32,
    pub zero_target_entries: u32,
    pub spread_calls: u32,
    pub placed_entries: u32,
    pub reinserted_entries: u32,
    pub bitmap_clears: u32,
}

impl NativeSpreadProcessStats {
    fn add(&mut self, other: Self) {
        self.processor_calls += other.processor_calls;
        self.budget_rng_draws += other.budget_rng_draws;
        self.requested_budget += other.requested_budget;
        self.popped_entries += other.popped_entries;
        self.zero_target_entries += other.zero_target_entries;
        self.spread_calls += other.spread_calls;
        self.placed_entries += other.placed_entries;
        self.reinserted_entries += other.reinserted_entries;
        self.bitmap_clears += other.bitmap_clears;
    }
}

impl NativeTiberiumClassState {
    pub fn new_due(current_frame: u32) -> Self {
        Self {
            growth_timer: NativeTiberiumTimer::due(current_frame),
            spread_timer: NativeTiberiumTimer::due(current_frame),
            growth_heap: Vec::new(),
            spread_heap: Vec::new(),
            growth_bitmap: BTreeSet::new(),
            spread_bitmap: BTreeSet::new(),
        }
    }
}

impl NativeTiberiumTimer {
    pub fn due(current_frame: u32) -> Self {
        Self {
            start_frame: current_frame,
            interval: 0,
        }
    }
}

/// Persistent state for the incremental map scanner.
///
/// Lives in ProductionState. The scanner processes a fraction of the map each
/// tick and collects candidates via reservoir sampling (fair random selection
/// from a stream of unknown length, bounded to MAX_CANDIDATES).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OreGrowthState {
    /// Current position in the cell iteration (wraps to 0 after full scan).
    scan_cursor: usize,
    /// Total number of cells to scan (map_width * map_height).
    total_cells: usize,
    /// Map dimensions for cell coordinate conversion.
    map_width: u16,
    /// Map height for native neighbor bounds checks.
    #[serde(default)]
    map_height: u16,
    /// Cells eligible for growth this scan cycle.
    growth_candidates: Vec<(u16, u16)>,
    /// Cells eligible for spread this scan cycle.
    spread_candidates: Vec<(u16, u16)>,
    /// Reservoir sampling counter for growth (total candidates seen).
    growth_seen: usize,
    /// Reservoir sampling counter for spread (total candidates seen).
    spread_seen: usize,
    /// Native AddToGrowthQueue-style entries inserted by explicit placement paths.
    #[serde(default)]
    growth_queue: Vec<OreGrowthQueueEntry>,
    /// Native AddToSpreadQueue-style entries inserted by explicit cell events.
    #[serde(default)]
    spread_queue: Vec<OreSpreadQueueEntry>,
    /// Deterministic membership guard for `spread_queue`.
    #[serde(default)]
    spread_membership: BTreeSet<(ResourceType, u16, u16)>,
    /// Native per-`TiberiumClass` state shell for the YR queue model.
    #[serde(default)]
    native_tiberium: NativeTiberiumState,
}

impl OreGrowthState {
    /// Create a new scanner for a map of the given dimensions.
    pub fn new(map_width: u16, map_height: u16) -> Self {
        Self {
            scan_cursor: 0,
            total_cells: map_width as usize * map_height as usize,
            map_width,
            map_height,
            growth_candidates: Vec::with_capacity(MAX_CANDIDATES),
            spread_candidates: Vec::with_capacity(MAX_CANDIDATES),
            growth_seen: 0,
            spread_seen: 0,
            growth_queue: Vec::new(),
            spread_queue: Vec::new(),
            spread_membership: BTreeSet::new(),
            native_tiberium: NativeTiberiumState::default(),
        }
    }

    /// Allocate native per-type tiberium state with due timers.
    pub fn reset_native_tiberium_classes(&mut self, type_count: usize, current_frame: u32) {
        self.native_tiberium.classes = (0..type_count)
            .map(|_| NativeTiberiumClassState::new_due(current_frame))
            .collect();
    }

    /// Native per-type tiberium queue/timer shell.
    pub fn native_tiberium_state(&self) -> &NativeTiberiumState {
        &self.native_tiberium
    }

    /// Native-shaped `AddToGrowthQueue`: no dedupe, density-gated, one RNG on insert.
    pub fn add_native_growth_queue_cell(
        &mut self,
        overlay_grid: &OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        rx: u16,
        ry: u16,
        binary_frame: u32,
        rng: &mut SimRng,
    ) -> Option<NativeTiberiumQueueEntry> {
        let cell = overlay_grid.cell(rx, ry);
        let overlay_id = cell.overlay_id?;
        let mapping = overlay_registry.tiberium_overlay_mapping(tiberium_types, overlay_id)?;
        let ty = tiberium_types.get(mapping.tiberium_type)?;
        if cell.overlay_data >= ty.max_density.saturating_sub(1) {
            return None;
        }
        let class = self
            .native_tiberium
            .classes
            .get_mut(mapping.tiberium_type.0 as usize)?;
        let entry = NativeTiberiumQueueEntry {
            rx,
            ry,
            priority_bits: growth_queue_priority(binary_frame, rng.next_u32()).to_bits(),
        };
        class.growth_heap.push(entry);
        class.growth_bitmap.insert((rx, ry));
        Some(entry)
    }

    /// Native-shaped `AddToSpreadQueue`: source-gated, bitmap-deduped, one RNG on insert.
    pub fn add_native_spread_queue_cell(
        &mut self,
        overlay_grid: &OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        rx: u16,
        ry: u16,
        binary_frame: u32,
        spread_enabled: bool,
        rng: &mut SimRng,
    ) -> Option<NativeTiberiumQueueEntry> {
        let type_id =
            current_tiberium_type(overlay_grid, overlay_registry, tiberium_types, rx, ry)?;
        self.add_native_spread_queue_cell_for_type(
            type_id,
            overlay_grid,
            overlay_registry,
            tiberium_types,
            resolved_terrain,
            source_object_cells,
            rx,
            ry,
            binary_frame,
            spread_enabled,
            rng,
        )
    }

    fn add_native_spread_queue_cell_for_type(
        &mut self,
        type_id: TiberiumTypeId,
        overlay_grid: &OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        rx: u16,
        ry: u16,
        binary_frame: u32,
        spread_enabled: bool,
        rng: &mut SimRng,
    ) -> Option<NativeTiberiumQueueEntry> {
        if !source_can_spread_tiberium(
            type_id,
            overlay_grid,
            overlay_registry,
            tiberium_types,
            resolved_terrain,
            source_object_cells,
            rx,
            ry,
            spread_enabled,
        ) {
            return None;
        }
        let class = self.native_tiberium.classes.get_mut(type_id.0 as usize)?;
        if class.spread_bitmap.contains(&(rx, ry)) {
            return None;
        }
        let entry = NativeTiberiumQueueEntry {
            rx,
            ry,
            priority_bits: growth_queue_priority(binary_frame, rng.next_u32()).to_bits(),
        };
        class.spread_heap.push(entry);
        class.spread_bitmap.insert((rx, ry));
        Some(entry)
    }

    /// Process all due native growth queues. Spread feed is counted but not executed yet.
    pub fn tick_native_growth_driver(
        &mut self,
        overlay_grid: &mut OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        rng: &mut SimRng,
        current_frame: u32,
        growth_enabled: bool,
        spread_enabled: bool,
    ) -> NativeGrowthProcessStats {
        if !growth_enabled {
            return NativeGrowthProcessStats::default();
        }
        let due_ids: Vec<TiberiumTypeId> = self
            .native_tiberium
            .classes
            .iter()
            .enumerate()
            .filter_map(|(idx, class)| {
                native_timer_due(class.growth_timer, current_frame)
                    .then(|| u8::try_from(idx).ok().map(TiberiumTypeId))
                    .flatten()
            })
            .collect();
        let mut stats = NativeGrowthProcessStats::default();
        for type_id in due_ids {
            stats.add(self.process_native_growth_for_type(
                type_id,
                overlay_grid,
                overlay_registry,
                tiberium_types,
                resolved_terrain,
                source_object_cells,
                resource_nodes,
                rng,
                current_frame,
                spread_enabled,
            ));
            if let (Some(class), Some(ty)) = (
                self.native_tiberium.classes.get_mut(type_id.0 as usize),
                tiberium_types.get(type_id),
            ) {
                class.growth_timer = NativeTiberiumTimer {
                    start_frame: current_frame,
                    interval: scaled_timer_interval(ty.growth, TIMER_MULTIPLIER_PPM),
                };
            }
        }
        stats
    }

    /// Native `GrowthProcessor` for one tiberium type.
    pub fn process_native_growth_for_type(
        &mut self,
        type_id: TiberiumTypeId,
        overlay_grid: &mut OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        rng: &mut SimRng,
        current_frame: u32,
        spread_enabled: bool,
    ) -> NativeGrowthProcessStats {
        let Some(ty) = tiberium_types.get(type_id) else {
            return NativeGrowthProcessStats::default();
        };
        let Some(class) = self.native_tiberium.classes.get_mut(type_id.0 as usize) else {
            return NativeGrowthProcessStats::default();
        };
        if class.growth_heap.is_empty() || ty.growth_percentage_ppm <= 0 {
            return NativeGrowthProcessStats::default();
        }

        class
            .growth_heap
            .sort_by(|a, b| priority_f32(a).total_cmp(&priority_f32(b)));
        let batch = growth_batch_size(class.growth_heap.len(), ty.growth_percentage_ppm);
        let actual_attempts = signed_abs_mod_plus_one(rng.next_u32(), batch);
        let mut stats = NativeGrowthProcessStats {
            processor_calls: 1,
            attempt_rng_draws: 1,
            requested_attempts: actual_attempts,
            ..NativeGrowthProcessStats::default()
        };

        for _ in 0..actual_attempts {
            if class.growth_heap.is_empty() {
                break;
            }
            let entry = class.growth_heap.remove(0);
            stats.popped_entries += 1;
            let current_type = current_tiberium_type(
                overlay_grid,
                overlay_registry,
                tiberium_types,
                entry.rx,
                entry.ry,
            );
            if current_type != Some(type_id) {
                stats.stale_entries += 1;
                continue;
            }

            grow_existing_tiberium_cell(overlay_grid, resource_nodes, ty, entry.rx, entry.ry);
            stats.grown_entries += 1;

            let post_data = overlay_grid.cell(entry.rx, entry.ry).overlay_data;
            if post_data < ty.max_density.saturating_sub(1) {
                let replacement = NativeTiberiumQueueEntry {
                    rx: entry.rx,
                    ry: entry.ry,
                    priority_bits: growth_queue_priority(current_frame, rng.next_u32()).to_bits(),
                };
                class.growth_heap.push(replacement);
                class
                    .growth_heap
                    .sort_by(|a, b| priority_f32(a).total_cmp(&priority_f32(b)));
                class.growth_bitmap.insert((entry.rx, entry.ry));
                stats.reinserted_entries += 1;
                stats.spread_feed_calls += 1;
                if source_can_spread_tiberium(
                    type_id,
                    overlay_grid,
                    overlay_registry,
                    tiberium_types,
                    resolved_terrain,
                    source_object_cells,
                    entry.rx,
                    entry.ry,
                    spread_enabled,
                ) && !class.spread_bitmap.contains(&(entry.rx, entry.ry))
                {
                    let spread_entry = NativeTiberiumQueueEntry {
                        rx: entry.rx,
                        ry: entry.ry,
                        priority_bits: growth_queue_priority(current_frame, rng.next_u32())
                            .to_bits(),
                    };
                    class.spread_heap.push(spread_entry);
                    class.spread_bitmap.insert((entry.rx, entry.ry));
                    stats.spread_enqueued_entries += 1;
                }
            } else {
                class.growth_bitmap.remove(&(entry.rx, entry.ry));
                stats.full_clears += 1;
            }
        }

        stats
    }

    /// Process all due native spread queues.
    pub fn tick_native_spread_driver(
        &mut self,
        overlay_grid: &mut OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        path_grid: Option<&PathGrid>,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        rng: &mut SimRng,
        current_frame: u32,
        growth_enabled: bool,
        spread_enabled: bool,
    ) -> NativeSpreadProcessStats {
        if !growth_enabled || !spread_enabled {
            return NativeSpreadProcessStats::default();
        }
        let due_ids: Vec<TiberiumTypeId> = self
            .native_tiberium
            .classes
            .iter()
            .enumerate()
            .filter_map(|(idx, class)| {
                native_timer_due(class.spread_timer, current_frame)
                    .then(|| u8::try_from(idx).ok().map(TiberiumTypeId))
                    .flatten()
            })
            .collect();
        let mut stats = NativeSpreadProcessStats::default();
        for type_id in due_ids {
            stats.add(self.process_native_spread_for_type(
                type_id,
                overlay_grid,
                overlay_registry,
                tiberium_types,
                resource_nodes,
                path_grid,
                resolved_terrain,
                source_object_cells,
                rng,
                current_frame,
                spread_enabled,
            ));
            if let (Some(class), Some(ty)) = (
                self.native_tiberium.classes.get_mut(type_id.0 as usize),
                tiberium_types.get(type_id),
            ) {
                class.spread_timer = NativeTiberiumTimer {
                    start_frame: current_frame,
                    interval: scaled_timer_interval(ty.spread, TIMER_MULTIPLIER_PPM),
                };
            }
        }
        stats
    }

    /// Native `SpreadProcessor` for one tiberium type.
    pub fn process_native_spread_for_type(
        &mut self,
        type_id: TiberiumTypeId,
        overlay_grid: &mut OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        path_grid: Option<&PathGrid>,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        rng: &mut SimRng,
        current_frame: u32,
        spread_enabled: bool,
    ) -> NativeSpreadProcessStats {
        let Some(ty) = tiberium_types.get(type_id) else {
            return NativeSpreadProcessStats::default();
        };
        let class_idx = type_id.0 as usize;
        if self.native_tiberium.classes.get(class_idx).is_none() {
            return NativeSpreadProcessStats::default();
        };
        if self.native_tiberium.classes[class_idx]
            .spread_heap
            .is_empty()
            || ty.spread_percentage_ppm <= 0
        {
            return NativeSpreadProcessStats::default();
        }

        self.native_tiberium.classes[class_idx]
            .spread_heap
            .sort_by(|a, b| priority_f32(a).total_cmp(&priority_f32(b)));
        let batch = spread_batch_size(
            self.native_tiberium.classes[class_idx].spread_heap.len(),
            ty.spread_percentage_ppm,
        );
        let budget = signed_abs_mod_plus_one(rng.next_u32(), batch);
        let mut stats = NativeSpreadProcessStats {
            processor_calls: 1,
            budget_rng_draws: 1,
            requested_budget: budget,
            ..NativeSpreadProcessStats::default()
        };

        let mut processed_sources = 0;
        while processed_sources < budget {
            let Some(entry) = self.native_tiberium.classes[class_idx]
                .spread_heap
                .first()
                .copied()
            else {
                break;
            };
            self.native_tiberium.classes[class_idx]
                .spread_heap
                .remove(0);
            stats.popped_entries += 1;
            let valid_targets = count_native_spread_targets(
                resource_nodes,
                overlay_grid,
                path_grid,
                resolved_terrain,
                source_object_cells,
                entry.rx,
                entry.ry,
                self.map_width,
                self.effective_map_height(),
            );
            if valid_targets == 0 {
                self.native_tiberium.classes[class_idx]
                    .spread_bitmap
                    .remove(&(entry.rx, entry.ry));
                stats.zero_target_entries += 1;
                stats.bitmap_clears += 1;
                continue;
            }

            stats.spread_calls += 1;
            processed_sources += 1;
            if let Some(placed_cell) = spread_tiberium_from_source(
                type_id,
                overlay_grid,
                overlay_registry,
                tiberium_types,
                resource_nodes,
                path_grid,
                resolved_terrain,
                source_object_cells,
                entry.rx,
                entry.ry,
                self.map_width,
                self.effective_map_height(),
                spread_enabled,
                rng,
            ) {
                stats.placed_entries += 1;
                self.add_native_growth_queue_cell(
                    overlay_grid,
                    overlay_registry,
                    tiberium_types,
                    placed_cell.0,
                    placed_cell.1,
                    current_frame,
                    rng,
                );
            }

            if valid_targets > 1 {
                let class = &mut self.native_tiberium.classes[class_idx];
                class.spread_heap.push(NativeTiberiumQueueEntry {
                    rx: entry.rx,
                    ry: entry.ry,
                    priority_bits: 0.0f32.to_bits(),
                });
                class
                    .spread_heap
                    .sort_by(|a, b| priority_f32(a).total_cmp(&priority_f32(b)));
                class.spread_bitmap.insert((entry.rx, entry.ry));
                stats.reinserted_entries += 1;
            }
        }

        stats
    }

    /// Rebuild native growth then spread queues from current post-load cells.
    pub fn rebuild_native_tiberium_queues_from_overlays(
        &mut self,
        overlay_grid: &OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        basic_growth_enabled: bool,
        tiberium_spreads_enabled: bool,
        current_frame: u32,
    ) -> NativeTiberiumRebuildStats {
        self.reset_native_tiberium_classes(tiberium_types.len(), current_frame);
        let zero_priority = 0.0f32.to_bits();
        let mut stats = NativeTiberiumRebuildStats::default();

        for (rx, ry, cell) in overlay_grid.iter_occupied() {
            let Some(overlay_id) = cell.overlay_id else {
                continue;
            };
            let Some(mapping) =
                overlay_registry.tiberium_overlay_mapping(tiberium_types, overlay_id)
            else {
                continue;
            };
            let Some(ty) = tiberium_types.get(mapping.tiberium_type) else {
                continue;
            };
            let Some(class) = self
                .native_tiberium
                .classes
                .get_mut(mapping.tiberium_type.0 as usize)
            else {
                continue;
            };
            if !cell_is_flat(resolved_terrain, rx, ry) {
                continue;
            }

            if basic_growth_enabled
                && ty.growth_percentage_ppm >= 0
                && cell.overlay_data < ty.max_density.saturating_sub(1)
            {
                class.growth_heap.push(NativeTiberiumQueueEntry {
                    rx,
                    ry,
                    priority_bits: zero_priority,
                });
                class.growth_bitmap.insert((rx, ry));
                stats.growth_entries += 1;
            }

            if tiberium_spreads_enabled
                && ty.spread_percentage_ppm >= 0
                && cell.overlay_data > mapping.tiberium_type.0 / 2
                && !source_object_cells.contains(&(rx, ry))
            {
                class.spread_heap.push(NativeTiberiumQueueEntry {
                    rx,
                    ry,
                    priority_bits: zero_priority,
                });
                class.spread_bitmap.insert((rx, ry));
                stats.spread_entries += 1;
            }
        }

        stats
    }

    /// Enqueue a newly placed ore cell with native AddToGrowthQueue priority.
    ///
    /// Verified TIBTRE placement consumes one raw Random::Next word and stores
    /// priority as `currentFrame + (signed_abs(raw) % 50)`.
    pub fn enqueue_growth_queue_cell(
        &mut self,
        rx: u16,
        ry: u16,
        binary_frame: u32,
        rng: &mut SimRng,
    ) -> OreGrowthQueueEntry {
        let priority = growth_queue_priority(binary_frame, rng.next_u32());
        let entry = OreGrowthQueueEntry { rx, ry, priority };
        self.growth_queue.push(entry);
        entry
    }

    /// Native-style growth queue entries waiting for an explicit processor.
    pub fn growth_queue_entries(&self) -> &[OreGrowthQueueEntry] {
        &self.growth_queue
    }

    /// Native-style spread queue entries waiting for a future queue processor.
    pub fn spread_queue_entries(&self) -> &[OreSpreadQueueEntry] {
        &self.spread_queue
    }

    /// Clear all spread memberships for a removed cell across tiberium types.
    pub fn clear_spread_memberships_for_cell(&mut self, rx: u16, ry: u16) {
        self.spread_membership
            .retain(|&(_, cell_rx, cell_ry)| cell_rx != rx || cell_ry != ry);
        self.spread_queue
            .retain(|entry| entry.rx != rx || entry.ry != ry);
    }

    /// Add one cell to the per-type spread queue if it is not already queued.
    pub fn enqueue_spread_queue_cell(
        &mut self,
        resource_type: ResourceType,
        rx: u16,
        ry: u16,
    ) -> bool {
        if !self.spread_membership.insert((resource_type, rx, ry)) {
            return false;
        }
        self.spread_queue.push(OreSpreadQueueEntry {
            resource_type,
            rx,
            ry,
        });
        true
    }

    /// Reseed same-type resource neighbors around a just-depleted cell.
    pub fn reseed_spread_neighbors_after_reduction(
        &mut self,
        resource_type: ResourceType,
        cell: (u16, u16),
        resource_nodes: &BTreeMap<(u16, u16), ResourceNode>,
    ) {
        self.clear_spread_memberships_for_cell(cell.0, cell.1);
        let map_height = self.effective_map_height();
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let nx = cell.0 as i32 + dx;
            let ny = cell.1 as i32 + dy;
            if nx < 0 || ny < 0 || nx >= self.map_width as i32 || ny >= map_height as i32 {
                continue;
            }
            let neighbor = (nx as u16, ny as u16);
            let Some(node) = resource_nodes.get(&neighbor) else {
                continue;
            };
            if node.resource_type == resource_type && node.remaining > 0 {
                self.enqueue_spread_queue_cell(resource_type, neighbor.0, neighbor.1);
            }
        }
    }

    /// Native `Reduce_Tiberium` full-removal spread reseed.
    ///
    /// Clears this removed cell's spread bitmap bit for every tiberium class,
    /// then calls the removed cell's type `AddToSpreadQueue` for each eligible
    /// neighboring source. Existing heap entries are intentionally left stale.
    #[allow(clippy::too_many_arguments)]
    pub fn reseed_native_spread_neighbors_after_reduction(
        &mut self,
        removed_type: TiberiumTypeId,
        overlay_grid: &OverlayGrid,
        overlay_registry: &OverlayTypeRegistry,
        tiberium_types: &TiberiumTypeRegistry,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        source_object_cells: &BTreeSet<(u16, u16)>,
        removed_cell: (u16, u16),
        binary_frame: u32,
        spread_enabled: bool,
        rng: &mut SimRng,
    ) -> usize {
        self.clear_spread_memberships_for_cell(removed_cell.0, removed_cell.1);
        for class in &mut self.native_tiberium.classes {
            class.spread_bitmap.remove(&removed_cell);
        }

        let map_height = self.effective_map_height();
        let mut inserted = 0usize;
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let nx = removed_cell.0 as i32 + dx;
            let ny = removed_cell.1 as i32 + dy;
            if nx < 0 || ny < 0 || nx >= self.map_width as i32 || ny >= map_height as i32 {
                continue;
            }
            if self
                .add_native_spread_queue_cell_for_type(
                    removed_type,
                    overlay_grid,
                    overlay_registry,
                    tiberium_types,
                    resolved_terrain,
                    source_object_cells,
                    nx as u16,
                    ny as u16,
                    binary_frame,
                    spread_enabled,
                    rng,
                )
                .is_some()
            {
                inserted += 1;
            }
        }
        inserted
    }

    fn effective_map_height(&self) -> u16 {
        if self.map_height != 0 || self.map_width == 0 {
            return self.map_height;
        }
        (self.total_cells / self.map_width as usize) as u16
    }

    /// Hash persistent ore-growth scheduler state for replay/desync checks.
    pub fn hash_state(&self, hasher: &mut impl Hasher) {
        self.scan_cursor.hash(hasher);
        self.total_cells.hash(hasher);
        self.map_width.hash(hasher);
        self.effective_map_height().hash(hasher);
        self.growth_candidates.hash(hasher);
        self.spread_candidates.hash(hasher);
        self.growth_seen.hash(hasher);
        self.spread_seen.hash(hasher);
        for entry in &self.growth_queue {
            entry.rx.hash(hasher);
            entry.ry.hash(hasher);
            entry.priority.to_bits().hash(hasher);
        }
        for entry in &self.spread_queue {
            entry.resource_type.hash(hasher);
            entry.rx.hash(hasher);
            entry.ry.hash(hasher);
        }
        for &(resource_type, rx, ry) in &self.spread_membership {
            resource_type.hash(hasher);
            rx.hash(hasher);
            ry.hash(hasher);
        }
        self.native_tiberium.classes.len().hash(hasher);
        for class in &self.native_tiberium.classes {
            class.growth_timer.start_frame.hash(hasher);
            class.growth_timer.interval.hash(hasher);
            class.spread_timer.start_frame.hash(hasher);
            class.spread_timer.interval.hash(hasher);
            for entry in &class.growth_heap {
                entry.rx.hash(hasher);
                entry.ry.hash(hasher);
                entry.priority_bits.hash(hasher);
            }
            for entry in &class.spread_heap {
                entry.rx.hash(hasher);
                entry.ry.hash(hasher);
                entry.priority_bits.hash(hasher);
            }
            class.growth_bitmap.hash(hasher);
            class.spread_bitmap.hash(hasher);
        }
    }
}

fn cell_is_flat(resolved_terrain: Option<&ResolvedTerrainGrid>, rx: u16, ry: u16) -> bool {
    resolved_terrain
        .and_then(|grid| grid.cell(rx, ry))
        .map_or(true, |cell| cell.slope_type == 0)
}

fn native_timer_due(timer: NativeTiberiumTimer, current_frame: u32) -> bool {
    current_frame.wrapping_sub(timer.start_frame) >= timer.interval
}

fn scaled_timer_interval(base: u32, multiplier_ppm: u32) -> u32 {
    ((u64::from(base) * u64::from(multiplier_ppm)) / TIMER_MULTIPLIER_PPM as u64)
        .min(u64::from(u32::MAX)) as u32
}

fn priority_f32(entry: &NativeTiberiumQueueEntry) -> f32 {
    f32::from_bits(entry.priority_bits)
}

fn growth_batch_size(heap_count: usize, growth_percentage_ppm: i32) -> u32 {
    let scaled =
        ((heap_count as i64 * i64::from(growth_percentage_ppm)) + (PERCENT_PPM / 2)) / PERCENT_PPM;
    scaled.clamp(i64::from(GROWTH_BATCH_MIN), i64::from(GROWTH_BATCH_MAX)) as u32
}

fn spread_batch_size(heap_count: usize, spread_percentage_ppm: i32) -> u32 {
    let scaled =
        ((heap_count as i64 * i64::from(spread_percentage_ppm)) + (PERCENT_PPM / 2)) / PERCENT_PPM;
    scaled.clamp(i64::from(SPREAD_BATCH_MIN), i64::from(SPREAD_BATCH_MAX)) as u32
}

fn signed_abs_mod_plus_one(raw: u32, modulus: u32) -> u32 {
    debug_assert!(modulus > 0);
    let signed = raw as i32;
    let abs = if signed < 0 {
        signed.wrapping_neg() as u32
    } else {
        signed as u32
    };
    abs % modulus + 1
}

fn current_tiberium_type(
    overlay_grid: &OverlayGrid,
    overlay_registry: &OverlayTypeRegistry,
    tiberium_types: &TiberiumTypeRegistry,
    rx: u16,
    ry: u16,
) -> Option<TiberiumTypeId> {
    let overlay_id = overlay_grid.cell(rx, ry).overlay_id?;
    overlay_registry
        .tiberium_overlay_mapping(tiberium_types, overlay_id)
        .map(|mapping| mapping.tiberium_type)
}

fn source_can_spread_tiberium(
    type_id: TiberiumTypeId,
    overlay_grid: &OverlayGrid,
    overlay_registry: &OverlayTypeRegistry,
    tiberium_types: &TiberiumTypeRegistry,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    source_object_cells: &BTreeSet<(u16, u16)>,
    rx: u16,
    ry: u16,
    spread_enabled: bool,
) -> bool {
    if !spread_enabled || source_object_cells.contains(&(rx, ry)) {
        return false;
    }
    if !cell_is_flat(resolved_terrain, rx, ry) {
        return false;
    }
    if current_tiberium_type(overlay_grid, overlay_registry, tiberium_types, rx, ry)
        != Some(type_id)
    {
        return false;
    }
    let Some(ty) = tiberium_types.get(type_id) else {
        return false;
    };
    let cell = overlay_grid.cell(rx, ry);
    if cell.overlay_data <= type_id.0 / 2 {
        return false;
    }
    if ty.spread_percentage_ppm < 0 {
        return false;
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn count_native_spread_targets(
    resource_nodes: &BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: &OverlayGrid,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    source_object_cells: &BTreeSet<(u16, u16)>,
    rx: u16,
    ry: u16,
    map_width: u16,
    map_height: u16,
) -> u8 {
    let mut count = 0u8;
    for &(dx, dy) in &ADJACENT_OFFSETS {
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;
        if nx < 0 || ny < 0 || nx >= map_width as i32 || ny >= map_height as i32 {
            continue;
        }
        if can_place_native_tiberium_target(
            resource_nodes,
            overlay_grid,
            path_grid,
            resolved_terrain,
            source_object_cells,
            nx as u16,
            ny as u16,
        ) {
            count = count.saturating_add(1);
        }
    }
    count
}

#[allow(clippy::too_many_arguments)]
fn spread_tiberium_from_source(
    type_id: TiberiumTypeId,
    overlay_grid: &mut OverlayGrid,
    overlay_registry: &OverlayTypeRegistry,
    tiberium_types: &TiberiumTypeRegistry,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    source_object_cells: &BTreeSet<(u16, u16)>,
    rx: u16,
    ry: u16,
    map_width: u16,
    map_height: u16,
    spread_enabled: bool,
    rng: &mut SimRng,
) -> Option<(u16, u16)> {
    if !source_can_spread_tiberium(
        type_id,
        overlay_grid,
        overlay_registry,
        tiberium_types,
        resolved_terrain,
        source_object_cells,
        rx,
        ry,
        spread_enabled,
    ) {
        return None;
    }
    let start_dir = rng.next_range_u32(8) as usize;
    for i in 0..8 {
        let dir = (start_dir + i) % 8;
        let (dx, dy) = ADJACENT_OFFSETS[dir];
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;
        if nx < 0 || ny < 0 || nx >= map_width as i32 || ny >= map_height as i32 {
            continue;
        }
        let target = (nx as u16, ny as u16);
        if !can_place_native_tiberium_target(
            resource_nodes,
            overlay_grid,
            path_grid,
            resolved_terrain,
            source_object_cells,
            target.0,
            target.1,
        ) {
            continue;
        }
        place_native_spread_tiberium(
            type_id,
            target,
            overlay_grid,
            overlay_registry,
            tiberium_types,
            resource_nodes,
            rng,
        )?;
        return Some(target);
    }
    None
}

fn can_place_native_tiberium_target(
    resource_nodes: &BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: &OverlayGrid,
    path_grid: Option<&PathGrid>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    source_object_cells: &BTreeSet<(u16, u16)>,
    rx: u16,
    ry: u16,
) -> bool {
    if resource_nodes.contains_key(&(rx, ry)) || source_object_cells.contains(&(rx, ry)) {
        return false;
    }
    if overlay_grid.cell(rx, ry).overlay_id.is_some() {
        return false;
    }
    if let Some(grid) = resolved_terrain {
        let Some(cell) = grid.cell(rx, ry) else {
            return false;
        };
        if !cell.allows_tiberium || cell.slope_type != 0 || cell.base_build_blocked {
            return false;
        }
        if cell.bridge_flags() & (BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DESTROYED_OR_RAMP) != 0 {
            return false;
        }
    } else if let Some(grid) = path_grid {
        if grid.cell(rx, ry).is_none() {
            return false;
        }
    }
    true
}

fn place_native_spread_tiberium(
    type_id: TiberiumTypeId,
    target: (u16, u16),
    overlay_grid: &mut OverlayGrid,
    overlay_registry: &OverlayTypeRegistry,
    tiberium_types: &TiberiumTypeRegistry,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    rng: &mut SimRng,
) -> Option<()> {
    let ty = tiberium_types.get(type_id)?;
    if SPREAD_GERMINATION_DENSITY >= ty.max_density {
        return None;
    }
    let variants = overlay_registry.flat_tiberium_variant_ids(ty)?;
    let overlay_id = variants[rng.next_range_u32(variants.len() as u32) as usize];
    overlay_grid.place_overlay(target.0, target.1, overlay_id, SPREAD_GERMINATION_DENSITY);
    let resource_type = resource_type_for_tiberium_image(ty.image);
    let base = stock_per_density_for_tiberium_image(ty.image);
    resource_nodes.insert(
        target,
        ResourceNode {
            resource_type,
            remaining: base.saturating_mul(u16::from(SPREAD_GERMINATION_DENSITY)),
        },
    );
    Some(())
}

fn grow_existing_tiberium_cell(
    overlay_grid: &mut OverlayGrid,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    ty: &crate::rules::tiberium_type::TiberiumType,
    rx: u16,
    ry: u16,
) {
    let current_data = overlay_grid.cell(rx, ry).overlay_data;
    let new_data = current_data
        .saturating_add(1)
        .min(ty.max_density.saturating_sub(1));
    overlay_grid.set_overlay_data(rx, ry, new_data);
    let resource_type = resource_type_for_tiberium_image(ty.image);
    let base = stock_per_density_for_tiberium_image(ty.image);
    resource_nodes
        .entry((rx, ry))
        .and_modify(|node| {
            node.resource_type = resource_type;
            node.remaining = node.remaining.saturating_add(base);
        })
        .or_insert(ResourceNode {
            resource_type,
            remaining: base,
        });
}

fn resource_type_for_tiberium_image(image: u8) -> ResourceType {
    if image == 2 {
        ResourceType::Gem
    } else {
        ResourceType::Ore
    }
}

fn stock_per_density_for_tiberium_image(image: u8) -> u16 {
    if image == 2 {
        GEM_BASE_PER_LEVEL
    } else {
        ORE_BASE_PER_LEVEL
    }
}

/// Advance ore growth/spread by one sim tick.
///
/// This is the main entry point called from advance_tick(). It scans a fraction
/// of the map each tick and executes growth/spread when a full cycle completes.
pub fn tick_ore_growth(
    config: &OreGrowthConfig,
    state: &mut OreGrowthState,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    path_grid: Option<&PathGrid>,
    mut overlay_grid: Option<&mut crate::sim::overlay_grid::OverlayGrid>,
    rng: &mut SimRng,
) {
    if !config.grows && !config.spreads {
        return;
    }
    if state.total_cells == 0 {
        return;
    }

    // How many cells to scan this tick: total_cells / (rate_seconds * tick_hz).
    // This ensures one full scan completes every `growth_rate_seconds` seconds.
    let rate_seconds: u32 = config.growth_rate_seconds.max(1);
    let ticks_per_cycle: u32 = rate_seconds.saturating_mul(SIM_TICK_HZ).max(1);
    let cells_per_tick: usize =
        (state.total_cells as u32).div_ceil(ticks_per_cycle).max(1) as usize;

    // Scan a chunk of cells from the cursor position.
    let scan_end = (state.scan_cursor + cells_per_tick).min(state.total_cells);

    // We iterate over resource_nodes rather than all cells — much more efficient
    // since only a small fraction of cells have ore. We filter by coordinate range
    // corresponding to the current scan chunk.
    for (&(rx, ry), node) in resource_nodes.iter() {
        let cell_index = ry as usize * state.map_width as usize + rx as usize;
        if cell_index < state.scan_cursor || cell_index >= scan_end {
            continue;
        }

        // Only ore grows/spreads (not gems), matching RA1 behavior.
        if node.resource_type != ResourceType::Ore {
            continue;
        }

        // Can this cell grow? (ore present, below max richness)
        if config.grows && node.remaining < MAX_ORE_REMAINING {
            reservoir_sample(
                &mut state.growth_candidates,
                &mut state.growth_seen,
                (rx, ry),
                rng,
            );
        }

        // Can this cell spread? (ore present, above spread threshold)
        if config.spreads && node.remaining > SPREAD_THRESHOLD {
            reservoir_sample(
                &mut state.spread_candidates,
                &mut state.spread_seen,
                (rx, ry),
                rng,
            );
        }
    }

    state.scan_cursor = scan_end;

    // When full scan completes, execute collected growth and spread actions.
    if state.scan_cursor >= state.total_cells {
        // Phase 1: Growth — increase remaining by one richness level.
        if config.grows {
            for &(rx, ry) in &state.growth_candidates {
                if let Some(node) = resource_nodes.get_mut(&(rx, ry)) {
                    if node.resource_type == ResourceType::Ore && node.remaining < MAX_ORE_REMAINING
                    {
                        let new_remaining = node.remaining + ORE_BASE_PER_LEVEL;
                        node.remaining = new_remaining.min(MAX_ORE_REMAINING);
                        // Sync overlay frame to match new density.
                        if let Some(grid) = overlay_grid.as_deref_mut() {
                            let frame = (node.remaining / ORE_BASE_PER_LEVEL)
                                .saturating_sub(1)
                                .min(11) as u8;
                            grid.set_overlay_data(rx, ry, frame);
                        }
                    }
                }
            }
        }

        // Phase 2: Spread — spawn new ore in a random adjacent empty cell.
        if config.spreads {
            for &(rx, ry) in &state.spread_candidates {
                try_spread_ore(
                    resource_nodes,
                    path_grid,
                    overlay_grid.as_deref_mut(),
                    rng,
                    rx,
                    ry,
                    state.map_width,
                );
            }
        }

        // Reset for next cycle.
        state.scan_cursor = 0;
        state.growth_candidates.clear();
        state.spread_candidates.clear();
        state.growth_seen = 0;
        state.spread_seen = 0;

        let node_count = resource_nodes.len();
        log::debug!(
            "Ore growth cycle complete: {} resource nodes on map",
            node_count
        );
    }
}

/// Reservoir sampling: maintain a bounded random sample from a stream.
///
/// Ensures each candidate has an equal probability of being in the final sample,
/// regardless of the total stream length. Matches RA1's MapClass::Logic approach.
fn reservoir_sample(
    candidates: &mut Vec<(u16, u16)>,
    seen: &mut usize,
    cell: (u16, u16),
    rng: &mut SimRng,
) {
    *seen += 1;
    if candidates.len() < MAX_CANDIDATES {
        candidates.push(cell);
    } else {
        // Replace a random existing candidate with probability MAX_CANDIDATES / seen.
        let r = rng.next_range_u32(*seen as u32) as usize;
        if r < MAX_CANDIDATES {
            candidates[r] = cell;
        }
    }
}

/// Native-shaped AddToGrowthQueue priority from one raw RNG word.
fn growth_queue_priority(binary_frame: u32, raw: u32) -> f32 {
    binary_frame.wrapping_add(growth_queue_priority_delay(raw)) as f32
}

fn growth_queue_priority_delay(raw: u32) -> u32 {
    let signed = raw as i32;
    let abs = if signed < 0 {
        signed.wrapping_neg() as u32
    } else {
        signed as u32
    };
    abs % GROWTH_QUEUE_PRIORITY_WINDOW
}

/// Try to spread ore from (rx, ry) to a random adjacent cell.
///
/// Picks a random starting direction and checks all 8 neighbors. The first
/// cell that passes `can_germinate()` gets a new ore node at level 1.
fn try_spread_ore(
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    path_grid: Option<&PathGrid>,
    overlay_grid: Option<&mut crate::sim::overlay_grid::OverlayGrid>,
    rng: &mut SimRng,
    rx: u16,
    ry: u16,
    map_width: u16,
) {
    // Random starting direction for fairness (matching RA1 Random_Pick(FACING_N, FACING_NW)).
    let start_dir = rng.next_range_u32(8) as usize;

    for i in 0..8 {
        let dir = (start_dir + i) % 8;
        let (dx, dy) = ADJACENT_OFFSETS[dir];
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;

        // Bounds check.
        if nx < 0 || ny < 0 || nx >= map_width as i32 {
            continue;
        }
        let nx = nx as u16;
        let ny = ny as u16;

        if can_germinate(resource_nodes, path_grid, nx, ny) {
            resource_nodes.insert(
                (nx, ny),
                ResourceNode {
                    resource_type: ResourceType::Ore,
                    remaining: ORE_BASE_PER_LEVEL,
                },
            );
            // New ore at level 1 -> frame 0. Copy overlay_id from source cell.
            if let Some(grid) = overlay_grid {
                if let Some(source_id) = grid.cell(rx, ry).overlay_id {
                    grid.place_overlay(nx, ny, source_id, 0);
                }
            }
            return;
        }
    }
}

/// Whether a cell can receive new ore via spread.
///
/// Matches RA1 CellClass::Can_Tiberium_Germinate:
/// - No existing resource node on the cell
/// - Cell is within map bounds
/// - Cell is walkable (not water, cliff, or building footprint)
fn can_germinate(
    resource_nodes: &BTreeMap<(u16, u16), ResourceNode>,
    path_grid: Option<&PathGrid>,
    rx: u16,
    ry: u16,
) -> bool {
    // Already has a resource node — can't place another.
    if resource_nodes.contains_key(&(rx, ry)) {
        return false;
    }

    // Must be walkable terrain (not water, cliff, or building).
    if let Some(grid) = path_grid {
        if !grid.is_walkable(rx, ry) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::overlay::OverlayEntry;
    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::tiberium_type::{TiberiumTypeId, TiberiumTypeRegistry};
    use crate::sim::miner::{ResourceNode, ResourceType};
    use crate::sim::overlay_grid::OverlayGrid;
    use crate::sim::rng::SimRng;

    fn make_config(grows: bool, spreads: bool) -> OreGrowthConfig {
        OreGrowthConfig {
            grows,
            spreads,
            growth_rate_seconds: 1, // Very fast for testing
        }
    }

    fn make_state(width: u16, height: u16) -> OreGrowthState {
        OreGrowthState::new(width, height)
    }

    fn ore_node(remaining: u16) -> ResourceNode {
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining,
        }
    }

    fn gem_node(remaining: u16) -> ResourceNode {
        ResourceNode {
            resource_type: ResourceType::Gem,
            remaining,
        }
    }

    fn tiberium_rebuild_fixture() -> (IniFile, OverlayTypeRegistry, TiberiumTypeRegistry) {
        let mut text = String::from(
            "\
[Tiberiums]
0=Riparius
1=Cruentus

[Riparius]
Image=1
Value=25
Growth=2200
GrowthPercentage=.06
Spread=2200
SpreadPercentage=.06

[Cruentus]
Image=2
Value=50
Growth=10000
GrowthPercentage=0
Spread=10000
SpreadPercentage=0

[OverlayTypes]
",
        );
        let mut key = 1;
        for prefix in ["TIB", "GEM"] {
            for variant in 1..=12 {
                text.push_str(&format!(
                    "{}={}{}\n",
                    key,
                    prefix,
                    format!("{:02}", variant)
                ));
                key += 1;
            }
        }
        for prefix in ["TIB", "GEM"] {
            for variant in 1..=12 {
                text.push_str(&format!("[{}{:02}]\nTiberium=yes\n", prefix, variant));
            }
        }
        let ini = IniFile::from_str(&text);
        let overlay_registry = OverlayTypeRegistry::from_ini(&ini, None);
        let tiberium_types = TiberiumTypeRegistry::from_ini(&ini);
        (ini, overlay_registry, tiberium_types)
    }

    /// Run enough ticks to complete one full scan cycle.
    fn run_full_cycle(
        config: &OreGrowthConfig,
        state: &mut OreGrowthState,
        nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        rng: &mut SimRng,
    ) {
        for _ in 0..10000 {
            tick_ore_growth(config, state, nodes, None, None, rng);
            if state.scan_cursor == 0 {
                return;
            }
        }
        panic!("Full cycle did not complete within 10000 ticks");
    }

    #[test]
    fn growth_increments_ore_remaining() {
        let config = make_config(true, false);
        let mut state = make_state(10, 10);
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(120)); // Level 1
        let mut rng = SimRng::new(42);

        run_full_cycle(&config, &mut state, &mut nodes, &mut rng);

        let node = nodes.get(&(5, 5)).expect("node still exists");
        assert_eq!(node.remaining, 240, "Should grow by one level (120)");
    }

    #[test]
    fn growth_caps_at_max_remaining() {
        let config = make_config(true, false);
        let mut state = make_state(10, 10);
        let mut nodes = BTreeMap::new();
        nodes.insert((3, 3), ore_node(MAX_ORE_REMAINING - 10)); // Near max
        let mut rng = SimRng::new(42);

        run_full_cycle(&config, &mut state, &mut nodes, &mut rng);

        let node = nodes.get(&(3, 3)).expect("node still exists");
        assert_eq!(node.remaining, MAX_ORE_REMAINING, "Should cap at max");
    }

    #[test]
    fn gems_do_not_grow_or_spread() {
        let config = make_config(true, true);
        let mut state = make_state(10, 10);
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), gem_node(900)); // Rich gems — above spread threshold
        let mut rng = SimRng::new(42);

        run_full_cycle(&config, &mut state, &mut nodes, &mut rng);

        let node = nodes.get(&(5, 5)).expect("node still exists");
        assert_eq!(node.remaining, 900, "Gems should not grow");
        // Only the original gem node should exist (no spread).
        assert_eq!(nodes.len(), 1, "Gems should not spread");
    }

    #[test]
    fn spread_creates_new_ore_node() {
        let config = make_config(false, true);
        let mut state = make_state(10, 10);
        let mut nodes = BTreeMap::new();
        // Rich ore above spread threshold.
        nodes.insert((5, 5), ore_node(SPREAD_THRESHOLD + 120));
        let mut rng = SimRng::new(42);

        run_full_cycle(&config, &mut state, &mut nodes, &mut rng);

        assert!(
            nodes.len() > 1,
            "Should have spread to at least one adjacent cell"
        );
        // New node should be ore at base level.
        for (&(rx, ry), node) in &nodes {
            if rx == 5 && ry == 5 {
                continue;
            }
            assert_eq!(node.resource_type, ResourceType::Ore);
            assert_eq!(node.remaining, ORE_BASE_PER_LEVEL);
            // Must be adjacent to (5,5).
            let dx = (rx as i32 - 5).unsigned_abs();
            let dy = (ry as i32 - 5).unsigned_abs();
            assert!(dx <= 1 && dy <= 1, "Spread node must be adjacent");
        }
    }

    #[test]
    fn ore_below_threshold_does_not_spread() {
        let config = make_config(false, true);
        let mut state = make_state(10, 10);
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(SPREAD_THRESHOLD - 1)); // Below threshold
        let mut rng = SimRng::new(42);

        run_full_cycle(&config, &mut state, &mut nodes, &mut rng);

        assert_eq!(nodes.len(), 1, "Low ore should not spread");
    }

    #[test]
    fn disabled_flags_prevent_all_activity() {
        let config = make_config(false, false);
        let mut state = make_state(10, 10);
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(120));
        let mut rng = SimRng::new(42);

        // Run many ticks — nothing should change.
        for _ in 0..100 {
            tick_ore_growth(&config, &mut state, &mut nodes, None, None, &mut rng);
        }

        let node = nodes.get(&(5, 5)).expect("node still exists");
        assert_eq!(node.remaining, 120, "Nothing should change when disabled");
    }

    #[test]
    fn cannot_germinate_on_existing_node() {
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(120));

        assert!(!can_germinate(&nodes, None, 5, 5));
        assert!(can_germinate(&nodes, None, 5, 6));
    }

    #[test]
    fn reservoir_sampling_stays_bounded() {
        let mut candidates: Vec<(u16, u16)> = Vec::new();
        let mut seen: usize = 0;
        let mut rng = SimRng::new(99);

        for i in 0..500 {
            reservoir_sample(&mut candidates, &mut seen, (i, 0), &mut rng);
        }

        assert_eq!(seen, 500);
        assert!(
            candidates.len() <= MAX_CANDIDATES,
            "Candidates should not exceed MAX_CANDIDATES"
        );
    }

    #[test]
    fn growth_queue_priority_uses_signed_abs_raw_modulo() {
        assert_eq!(growth_queue_priority_delay(0), 0);
        assert_eq!(growth_queue_priority_delay(0xFFFF_FFFF), 1);
        assert_eq!(growth_queue_priority_delay(51), 1);
        assert_eq!(growth_queue_priority_delay(0x8000_0000), 48);
    }

    #[test]
    fn enqueue_growth_queue_cell_consumes_one_raw_draw_and_stores_priority() {
        let mut state = make_state(20, 20);
        let mut rng = SimRng::new(1);
        let before = rng.state();

        let entry = state.enqueue_growth_queue_cell(4, 7, 1234, &mut rng);

        assert_ne!(rng.state(), before, "queue insertion consumes one raw draw");
        assert_eq!(entry.rx, 4);
        assert_eq!(entry.ry, 7);
        assert_eq!(
            entry.priority,
            growth_queue_priority(1234, 0x78B7_6ED5),
            "first raw draw for seed 1 should set native-style priority"
        );
        assert_eq!(state.growth_queue_entries(), &[entry]);
    }

    #[test]
    fn native_tiberium_shell_allocates_per_type_due_timers() {
        let mut state = make_state(20, 20);

        state.reset_native_tiberium_classes(4, 1234);

        let native = state.native_tiberium_state();
        assert_eq!(native.classes.len(), 4);
        for class in &native.classes {
            assert_eq!(class.growth_timer, NativeTiberiumTimer::due(1234));
            assert_eq!(class.spread_timer, NativeTiberiumTimer::due(1234));
            assert!(class.growth_heap.is_empty());
            assert!(class.spread_heap.is_empty());
            assert!(class.growth_bitmap.is_empty());
            assert!(class.spread_bitmap.is_empty());
        }
    }

    #[test]
    fn native_tiberium_shell_hashes_timers_heaps_and_bitmaps() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut base = make_state(20, 20);
        base.reset_native_tiberium_classes(1, 10);

        let mut changed = base.clone();
        let class = &mut changed.native_tiberium.classes[0];
        class.growth_timer.interval = 2200;
        class.growth_heap.push(NativeTiberiumQueueEntry {
            rx: 4,
            ry: 7,
            priority_bits: 0.0f32.to_bits(),
        });
        class.spread_bitmap.insert((5, 8));

        let mut base_hasher = DefaultHasher::new();
        base.hash_state(&mut base_hasher);
        let mut changed_hasher = DefaultHasher::new();
        changed.hash_state(&mut changed_hasher);

        assert_ne!(base_hasher.finish(), changed_hasher.finish());
    }

    #[test]
    fn native_tiberium_rebuild_seeds_growth_and_spread_from_overlay_cells() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let tib02 = overlay_registry.id_for_name("TIB02").expect("TIB02");
        let gem01 = overlay_registry.id_for_name("GEM01").expect("GEM01");
        let gem02 = overlay_registry.id_for_name("GEM02").expect("GEM02");
        let overlay_grid = OverlayGrid::from_overlay_entries(
            &[
                OverlayEntry {
                    rx: 1,
                    ry: 1,
                    overlay_id: tib01,
                    frame: 10,
                },
                OverlayEntry {
                    rx: 2,
                    ry: 1,
                    overlay_id: tib02,
                    frame: 11,
                },
                OverlayEntry {
                    rx: 1,
                    ry: 2,
                    overlay_id: gem01,
                    frame: 0,
                },
                OverlayEntry {
                    rx: 2,
                    ry: 2,
                    overlay_id: gem02,
                    frame: 1,
                },
            ],
            8,
            8,
        );
        let mut state = make_state(8, 8);

        let stats = state.rebuild_native_tiberium_queues_from_overlays(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            true,
            true,
            77,
        );

        assert_eq!(
            stats,
            NativeTiberiumRebuildStats {
                growth_entries: 3,
                spread_entries: 3,
            }
        );
        let native = state.native_tiberium_state();
        let riparius = &native.classes[TiberiumTypeId(0).0 as usize];
        let cruentus = &native.classes[TiberiumTypeId(1).0 as usize];
        assert_eq!(riparius.growth_heap.len(), 1, "data 11 does not grow");
        assert_eq!(riparius.spread_heap.len(), 2, "data 10 and 11 spread");
        assert_eq!(
            cruentus.growth_heap.len(),
            2,
            "zero percentage still seeds rebuild membership"
        );
        assert_eq!(cruentus.spread_heap.len(), 1, "type 1 needs data > 0");
        assert!(
            riparius
                .growth_heap
                .iter()
                .all(|entry| entry.priority_bits == 0.0f32.to_bits())
        );
        assert_eq!(riparius.growth_bitmap.len(), riparius.growth_heap.len());
        assert_eq!(cruentus.spread_bitmap.len(), cruentus.spread_heap.len());
    }

    #[test]
    fn native_tiberium_rebuild_respects_basic_growth_and_source_object_gates() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let tib02 = overlay_registry.id_for_name("TIB02").expect("TIB02");
        let overlay_grid = OverlayGrid::from_overlay_entries(
            &[
                OverlayEntry {
                    rx: 1,
                    ry: 1,
                    overlay_id: tib01,
                    frame: 10,
                },
                OverlayEntry {
                    rx: 2,
                    ry: 1,
                    overlay_id: tib02,
                    frame: 10,
                },
            ],
            8,
            8,
        );
        let mut source_object_cells = BTreeSet::new();
        source_object_cells.insert((1, 1));
        let mut state = make_state(8, 8);

        let stats = state.rebuild_native_tiberium_queues_from_overlays(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &source_object_cells,
            false,
            true,
            99,
        );

        assert_eq!(
            stats,
            NativeTiberiumRebuildStats {
                growth_entries: 0,
                spread_entries: 1,
            }
        );
        let riparius = &state.native_tiberium_state().classes[0];
        assert!(riparius.growth_heap.is_empty());
        assert_eq!(riparius.spread_heap.len(), 1);
        assert_eq!(
            (riparius.spread_heap[0].rx, riparius.spread_heap[0].ry),
            (2, 1)
        );
    }

    #[test]
    fn native_tiberium_rebuild_clears_previous_native_queue_state() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let populated = OverlayGrid::from_overlay_entries(
            &[OverlayEntry {
                rx: 1,
                ry: 1,
                overlay_id: tib01,
                frame: 10,
            }],
            8,
            8,
        );
        let empty = OverlayGrid::new(8, 8);
        let mut state = make_state(8, 8);
        state.rebuild_native_tiberium_queues_from_overlays(
            &populated,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            true,
            true,
            1,
        );
        assert!(
            !state.native_tiberium_state().classes[0]
                .growth_heap
                .is_empty()
        );

        let stats = state.rebuild_native_tiberium_queues_from_overlays(
            &empty,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            true,
            true,
            2,
        );

        assert_eq!(stats, NativeTiberiumRebuildStats::default());
        for class in &state.native_tiberium_state().classes {
            assert!(class.growth_heap.is_empty());
            assert!(class.spread_heap.is_empty());
            assert!(class.growth_bitmap.is_empty());
            assert!(class.spread_bitmap.is_empty());
            assert_eq!(class.growth_timer, NativeTiberiumTimer::due(2));
            assert_eq!(class.spread_timer, NativeTiberiumTimer::due(2));
        }
    }

    #[test]
    fn native_add_to_growth_queue_allows_duplicates_and_rejects_density_11_without_rng() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let mut overlay_grid = OverlayGrid::new(8, 8);
        overlay_grid.place_overlay(1, 1, tib01, 3);
        overlay_grid.place_overlay(2, 1, tib01, 11);
        let mut state = make_state(8, 8);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        let mut rng = SimRng::new(1);

        let first = state.add_native_growth_queue_cell(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            1,
            1,
            100,
            &mut rng,
        );
        let second = state.add_native_growth_queue_cell(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            1,
            1,
            100,
            &mut rng,
        );
        let before_reject = rng.state();
        let rejected = state.add_native_growth_queue_cell(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            2,
            1,
            100,
            &mut rng,
        );

        assert!(first.is_some());
        assert!(second.is_some());
        assert_eq!(
            state.native_tiberium_state().classes[0].growth_heap.len(),
            2,
            "growth inserts are not deduped by bitmap"
        );
        assert_eq!(
            state.native_tiberium_state().classes[0].growth_bitmap.len(),
            1
        );
        assert_eq!(rejected, None);
        assert_eq!(
            rng.state(),
            before_reject,
            "density-11 rejection consumes no RNG"
        );
    }

    #[test]
    fn native_growth_processor_zero_percentage_exits_without_rng() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let gem01 = overlay_registry.id_for_name("GEM01").expect("GEM01");
        let mut overlay_grid = OverlayGrid::new(8, 8);
        overlay_grid.place_overlay(1, 1, gem01, 1);
        let mut state = make_state(8, 8);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        state.native_tiberium.classes[1]
            .growth_heap
            .push(NativeTiberiumQueueEntry {
                rx: 1,
                ry: 1,
                priority_bits: 0.0f32.to_bits(),
            });
        let mut nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);
        let before = rng.state();

        let stats = state.process_native_growth_for_type(
            TiberiumTypeId(1),
            &mut overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            &mut nodes,
            &mut rng,
            100,
            true,
        );

        assert_eq!(stats, NativeGrowthProcessStats::default());
        assert_eq!(rng.state(), before);
        assert_eq!(
            state.native_tiberium_state().classes[1].growth_heap.len(),
            1
        );
    }

    #[test]
    fn native_growth_processor_drops_stale_entry_without_clearing_bitmap() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let mut overlay_grid = OverlayGrid::new(8, 8);
        let mut state = make_state(8, 8);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        state.native_tiberium.classes[0]
            .growth_heap
            .push(NativeTiberiumQueueEntry {
                rx: 1,
                ry: 1,
                priority_bits: 0.0f32.to_bits(),
            });
        state.native_tiberium.classes[0]
            .growth_bitmap
            .insert((1, 1));
        let mut nodes = BTreeMap::new();
        let mut rng = SimRng::new(3);

        let stats = state.process_native_growth_for_type(
            TiberiumTypeId(0),
            &mut overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            &mut nodes,
            &mut rng,
            100,
            true,
        );

        assert_eq!(stats.processor_calls, 1);
        assert_eq!(stats.attempt_rng_draws, 1);
        assert_eq!(stats.popped_entries, 1);
        assert_eq!(stats.stale_entries, 1);
        assert!(
            state.native_tiberium_state().classes[0]
                .growth_heap
                .is_empty()
        );
        assert!(
            state.native_tiberium_state().classes[0]
                .growth_bitmap
                .contains(&(1, 1)),
            "stale pop does not clear the growth bitmap"
        );
    }

    #[test]
    fn native_growth_processor_grows_then_clears_full_density_cell() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let mut overlay_grid = OverlayGrid::new(8, 8);
        overlay_grid.place_overlay(1, 1, tib01, 10);
        let mut state = make_state(8, 8);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        state.native_tiberium.classes[0]
            .growth_heap
            .push(NativeTiberiumQueueEntry {
                rx: 1,
                ry: 1,
                priority_bits: 0.0f32.to_bits(),
            });
        state.native_tiberium.classes[0]
            .growth_bitmap
            .insert((1, 1));
        let mut nodes = BTreeMap::new();
        nodes.insert((1, 1), ore_node(10 * ORE_BASE_PER_LEVEL));
        let mut rng = SimRng::new(5);

        let stats = state.process_native_growth_for_type(
            TiberiumTypeId(0),
            &mut overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            &mut nodes,
            &mut rng,
            100,
            true,
        );

        assert_eq!(overlay_grid.cell(1, 1).overlay_data, 11);
        assert_eq!(stats.grown_entries, 1);
        assert_eq!(stats.full_clears, 1);
        assert!(
            state.native_tiberium_state().classes[0]
                .growth_heap
                .is_empty()
        );
        assert!(
            !state.native_tiberium_state().classes[0]
                .growth_bitmap
                .contains(&(1, 1))
        );
    }

    #[test]
    fn native_growth_processor_reinserts_submax_cell_and_counts_spread_feed() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let mut overlay_grid = OverlayGrid::new(8, 8);
        overlay_grid.place_overlay(1, 1, tib01, 3);
        let mut state = make_state(8, 8);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        state.native_tiberium.classes[0]
            .growth_heap
            .push(NativeTiberiumQueueEntry {
                rx: 1,
                ry: 1,
                priority_bits: 0.0f32.to_bits(),
            });
        let mut nodes = BTreeMap::new();
        nodes.insert((1, 1), ore_node(3 * ORE_BASE_PER_LEVEL));
        let mut rng = SimRng::new(9);

        let stats = state.process_native_growth_for_type(
            TiberiumTypeId(0),
            &mut overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            &mut nodes,
            &mut rng,
            100,
            true,
        );

        assert_eq!(overlay_grid.cell(1, 1).overlay_data, 4);
        assert_eq!(stats.grown_entries, 1);
        assert_eq!(stats.reinserted_entries, 1);
        assert_eq!(stats.spread_feed_calls, 1);
        assert_eq!(
            state.native_tiberium_state().classes[0].growth_heap.len(),
            1
        );
        assert!(
            state.native_tiberium_state().classes[0]
                .growth_bitmap
                .contains(&(1, 1))
        );
    }

    #[test]
    fn native_add_to_spread_queue_dedupes_and_rejects_without_rng() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let mut overlay_grid = OverlayGrid::new(8, 8);
        overlay_grid.place_overlay(1, 1, tib01, 3);
        let mut state = make_state(8, 8);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        let mut rng = SimRng::new(11);

        let first = state.add_native_spread_queue_cell(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            1,
            1,
            100,
            true,
            &mut rng,
        );
        let before_dedupe = rng.state();
        let second = state.add_native_spread_queue_cell(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            1,
            1,
            100,
            true,
            &mut rng,
        );
        let disabled = state.add_native_spread_queue_cell(
            &overlay_grid,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            1,
            1,
            100,
            false,
            &mut rng,
        );

        assert!(first.is_some());
        assert_eq!(second, None);
        assert_eq!(disabled, None);
        assert_eq!(rng.state(), before_dedupe);
        assert_eq!(
            state.native_tiberium_state().classes[0].spread_heap.len(),
            1
        );
        assert!(
            state.native_tiberium_state().classes[0]
                .spread_bitmap
                .contains(&(1, 1))
        );
    }

    fn block_all_neighbors_except(
        overlay_grid: &mut OverlayGrid,
        blocker_id: u8,
        source: (u16, u16),
        open: Option<(u16, u16)>,
    ) {
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let cell = ((source.0 as i32 + dx) as u16, (source.1 as i32 + dy) as u16);
            if Some(cell) != open {
                overlay_grid.place_overlay(cell.0, cell.1, blocker_id, 0);
            }
        }
    }

    #[test]
    fn native_spread_processor_zero_target_entries_do_not_spend_budget() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let blocker = overlay_registry.id_for_name("GEM01").expect("GEM01");
        let mut overlay_grid = OverlayGrid::new(10, 10);
        overlay_grid.place_overlay(2, 2, tib01, 3);
        overlay_grid.place_overlay(7, 7, tib01, 3);
        block_all_neighbors_except(&mut overlay_grid, blocker, (2, 2), None);
        block_all_neighbors_except(&mut overlay_grid, blocker, (7, 7), Some((8, 7)));
        let mut state = make_state(10, 10);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        state.native_tiberium.classes[0]
            .spread_heap
            .push(NativeTiberiumQueueEntry {
                rx: 2,
                ry: 2,
                priority_bits: 0.0f32.to_bits(),
            });
        state.native_tiberium.classes[0]
            .spread_heap
            .push(NativeTiberiumQueueEntry {
                rx: 7,
                ry: 7,
                priority_bits: 1.0f32.to_bits(),
            });
        state.native_tiberium.classes[0]
            .spread_bitmap
            .insert((2, 2));
        state.native_tiberium.classes[0]
            .spread_bitmap
            .insert((7, 7));
        let mut nodes = BTreeMap::new();
        let mut rng = SimRng::new(12);

        let stats = state.process_native_spread_for_type(
            TiberiumTypeId(0),
            &mut overlay_grid,
            &overlay_registry,
            &tiberium_types,
            &mut nodes,
            None,
            None,
            &BTreeSet::new(),
            &mut rng,
            200,
            true,
        );

        assert_eq!(stats.processor_calls, 1);
        assert_eq!(stats.zero_target_entries, 1);
        assert_eq!(stats.spread_calls, 1);
        assert_eq!(stats.popped_entries, 2);
        assert!(
            !state.native_tiberium_state().classes[0]
                .spread_bitmap
                .contains(&(2, 2))
        );
    }

    #[test]
    fn native_spread_processor_one_target_leaves_bitmap_without_reinsert() {
        let (_ini, overlay_registry, tiberium_types) = tiberium_rebuild_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let blocker = overlay_registry.id_for_name("GEM01").expect("GEM01");
        let mut overlay_grid = OverlayGrid::new(8, 8);
        overlay_grid.place_overlay(3, 3, tib01, 3);
        block_all_neighbors_except(&mut overlay_grid, blocker, (3, 3), Some((4, 3)));
        let mut state = make_state(8, 8);
        state.reset_native_tiberium_classes(tiberium_types.len(), 10);
        state.native_tiberium.classes[0]
            .spread_heap
            .push(NativeTiberiumQueueEntry {
                rx: 3,
                ry: 3,
                priority_bits: 0.0f32.to_bits(),
            });
        state.native_tiberium.classes[0]
            .spread_bitmap
            .insert((3, 3));
        let mut nodes = BTreeMap::new();
        let mut rng = SimRng::new(13);

        let stats = state.process_native_spread_for_type(
            TiberiumTypeId(0),
            &mut overlay_grid,
            &overlay_registry,
            &tiberium_types,
            &mut nodes,
            None,
            None,
            &BTreeSet::new(),
            &mut rng,
            200,
            true,
        );

        assert_eq!(stats.spread_calls, 1);
        assert_eq!(stats.reinserted_entries, 0);
        assert!(
            state.native_tiberium_state().classes[0]
                .spread_heap
                .is_empty()
        );
        assert!(
            state.native_tiberium_state().classes[0]
                .spread_bitmap
                .contains(&(3, 3))
        );
        assert_eq!(
            overlay_grid.cell(4, 3).overlay_data,
            SPREAD_GERMINATION_DENSITY
        );
        assert_eq!(
            nodes.get(&(4, 3)).map(|node| node.remaining),
            Some(ORE_BASE_PER_LEVEL * u16::from(SPREAD_GERMINATION_DENSITY))
        );
        assert_eq!(
            state.native_tiberium_state().classes[0].growth_heap.len(),
            1
        );
    }

    #[test]
    fn full_scan_cycle_resets_cursor() {
        let config = make_config(true, false);
        let mut state = make_state(5, 5); // 25 cells — very small
        let mut nodes = BTreeMap::new();
        nodes.insert((2, 2), ore_node(120));
        let mut rng = SimRng::new(42);

        // Run ticks until cursor wraps.
        let mut wrapped = false;
        for _ in 0..1000 {
            tick_ore_growth(&config, &mut state, &mut nodes, None, None, &mut rng);
            if state.scan_cursor == 0 {
                wrapped = true;
                break;
            }
        }

        assert!(wrapped, "Scan cursor should wrap to 0 after full cycle");
    }

    #[test]
    fn growth_rate_controls_scan_speed() {
        // Fast rate: 0.01 minutes → scans many cells per tick.
        let fast = make_config(true, false);
        let mut state_fast = make_state(100, 100); // 10000 cells
        let mut nodes_fast = BTreeMap::new();
        nodes_fast.insert((50, 50), ore_node(120));
        let mut rng = SimRng::new(42);

        tick_ore_growth(
            &fast,
            &mut state_fast,
            &mut nodes_fast,
            None,
            None,
            &mut rng,
        );
        let fast_progress = state_fast.scan_cursor;

        // Slow rate: 100 minutes → scans very few cells per tick.
        let slow = OreGrowthConfig {
            grows: true,
            spreads: false,
            growth_rate_seconds: 6000, // 100 minutes
        };
        let mut state_slow = make_state(100, 100);
        let mut nodes_slow = BTreeMap::new();
        nodes_slow.insert((50, 50), ore_node(120));
        let mut rng2 = SimRng::new(42);

        tick_ore_growth(
            &slow,
            &mut state_slow,
            &mut nodes_slow,
            None,
            None,
            &mut rng2,
        );
        let slow_progress = state_slow.scan_cursor;

        assert!(
            fast_progress > slow_progress,
            "Fast rate ({}) should scan more cells per tick than slow rate ({})",
            fast_progress,
            slow_progress,
        );
    }

    #[test]
    fn spread_does_not_overwrite_existing_nodes() {
        let config = make_config(false, true);
        let mut state = make_state(10, 10);
        let mut nodes = BTreeMap::new();
        // Rich source at center.
        nodes.insert((5, 5), ore_node(SPREAD_THRESHOLD + 120));
        // Surround with existing gem nodes — spread should not overwrite them.
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let nx = (5 + dx) as u16;
            let ny = (5 + dy) as u16;
            nodes.insert((nx, ny), gem_node(500));
        }
        let mut rng = SimRng::new(42);

        run_full_cycle(&config, &mut state, &mut nodes, &mut rng);

        // Should still have exactly 9 nodes (center + 8 neighbors).
        assert_eq!(nodes.len(), 9, "No new nodes should appear when surrounded");
        // All neighbors should still be gems.
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let nx = (5 + dx) as u16;
            let ny = (5 + dy) as u16;
            let node = nodes.get(&(nx, ny)).expect("neighbor exists");
            assert_eq!(
                node.resource_type,
                ResourceType::Gem,
                "Neighbors should be unchanged gems"
            );
        }
    }
}
