//! TIBTRE-style terrain object ore spawning.
//!
//! Per-cell sim state for terrain objects with `SpawnsTiberium=yes`. Each tick,
//! every spawner rolls its `AnimationProbability`; on success it places ore
//! in a random adjacent walkable cell at density 3, additive on existing ore.
//!
//! ## Animation model
//! Single-phase: roll succeeds → spawn immediately. The two-phase model
//! (roll → animation midpoint countdown → spawn) is collapsed because the
//! animation visual is render-only and the spawn-rate average is identical.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/overlay_grid, sim/pathfinding,
//!   sim/rng, sim/miner (ResourceNode/ResourceType).
//! - The tick function does NOT depend on rules/ — config is baked into
//!   TerrainSpawnerState at seed time (mirrors OreGrowthConfig pattern).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::intern::InternedId;

/// Per-instance state for one TIBTRE-style spawner placed on the map.
///
/// Keyed by cell in `ProductionState::terrain_spawners`. The spawner doesn't
/// move and isn't destroyable (`Immune=yes` on TIBTRE), so the only
/// lifecycle is "exists from map load to game end".
///
/// `animation_probability_micros` is cached at seed time so the tick function
/// doesn't need to look up rules — same pattern as `OreGrowthConfig` baking
/// `growth_rate_seconds` from rules at map load.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TerrainSpawnerState {
    /// Interned name of the TerrainObjectType (e.g. "TIBTRE01"). Kept for
    /// debug logging and future render-side visual lookup; NOT used by the
    /// tick function.
    pub type_ref: InternedId,
    /// Cached `AnimationProbability * 1_000_000` from rules at seed time.
    /// The tick rolls `rng.next_range_u32(1_000_000) < this` directly.
    /// 0 = never spawns (defensive; seed function won't insert these).
    pub animation_probability_micros: u32,
}
