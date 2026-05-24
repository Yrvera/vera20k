//! TIBTRE-style terrain object ore spawning.
//!
//! Per-cell sim state for terrain objects with `SpawnsTiberium=yes`. Idle
//! spawners roll their native-shaped `AnimationProbability`; a hit starts the
//! terrain animation, and ore placement is delayed until the animation midpoint.
//!
//! ## Animation model
//! Two-phase: roll succeeds -> start at frame 0 -> advance one frame every
//! `AnimationRate` ticks -> reset to idle at midpoint -> forced tiberium spread.
//!
//! ## Dependency rules
//! - Part of sim/ - depends on sim/overlay_grid, sim/pathfinding, sim/rng, and
//!   sim/miner (ResourceNode/ResourceType).
//! - The tick function does NOT depend on rules/ - config is baked into
//!   TerrainSpawnerState at seed time (mirrors OreGrowthConfig pattern).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::{BTreeMap, BTreeSet};

use crate::map::bridge_facts::{BRIDGE_FLAG_DESTROYED_OR_RAMP, BRIDGE_FLAG_STRUCTURAL};
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::sim::intern::InternedId;
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::rng::SimRng;

/// Probability roll denominator. Matches binary's `random % 1_000_000`
/// against `AnimationProbability` scaled by 1.0e-6.
const PROBABILITY_DENOMINATOR: u32 = 1_000_000;
const PROBABILITY_SCALE: f64 = 1.0e-6;

/// Base ore stock per density level. Matches `ore_growth::ORE_BASE_PER_LEVEL`
/// and `seed_resource_nodes_from_overlays`.
const ORE_BASE_PER_LEVEL: u16 = 120;
/// Density levels placed per spawn. Matches binary's `PlaceTiberium(tib_type, 3)`.
const SPAWN_DENSITY_LEVELS: u16 = 3;

/// 8 adjacent directions: N, NE, E, SE, S, SW, W, NW.
/// Matches `ore_growth::ADJACENT_OFFSETS` ordering.
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

/// Exact fixed representation for `AnimationProbability`.
///
/// The binary rolls raw `Random::Next`, treats it as signed, takes abs, mods
/// by 1,000,000, scales by 1e-6 as a double, then uses strict `<`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TerrainSpawnProbability {
    pub micros: u32,
}

impl TerrainSpawnProbability {
    pub fn from_micros(micros: u32) -> Self {
        Self {
            micros: micros.min(PROBABILITY_DENOMINATOR),
        }
    }

    pub fn roll_succeeds(self, rng: &mut SimRng) -> bool {
        raw_probability_sample(rng.next_u32()) < self.as_f64()
    }

    fn as_f64(self) -> f64 {
        f64::from(self.micros) * PROBABILITY_SCALE
    }
}

/// Native-shaped probability sample from one raw RNG word.
pub fn raw_probability_sample(raw: u32) -> f64 {
    let signed = raw as i32;
    let abs = if signed < 0 {
        signed.wrapping_neg() as u32
    } else {
        signed as u32
    };
    f64::from(abs % PROBABILITY_DENOMINATOR) * PROBABILITY_SCALE
}

/// Persisted animation state for one terrain spawner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TerrainSpawnerPhase {
    Idle,
    Active {
        current_frame: u16,
        ticks_until_next_frame: u16,
    },
}

/// Per-instance state for one TIBTRE-style spawner placed on the map.
///
/// Keyed by cell in `ProductionState::terrain_spawners`. The spawner doesn't
/// move and isn't destroyable (`Immune=yes` on TIBTRE), so the only lifecycle is
/// "exists from map load to game end".
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TerrainSpawnerState {
    /// Interned name of the TerrainObjectType (e.g. "TIBTRE01"). Kept for
    /// debug logging and future render-side visual lookup; NOT used by the
    /// tick function.
    pub type_ref: InternedId,
    /// Compatibility mirror for existing integration/hash code.
    pub animation_probability_micros: u32,
    /// Native-shaped fixed probability used by the state-machine tick.
    pub animation_probability: TerrainSpawnProbability,
    /// `AnimationRate=` in logic ticks per animation frame.
    pub animation_rate_ticks: u16,
    /// Loaded terrain SHP frame count. App integration must supply this; tests
    /// may use stock 22, but production logic must not assume it.
    pub frame_count: u16,
    /// Frame at which the binary resets active state and calls SpreadTiberium.
    pub midpoint_frame: u16,
    /// Idle or currently playing terrain animation.
    pub phase: TerrainSpawnerPhase,
}

impl TerrainSpawnerState {
    pub fn new(
        type_ref: InternedId,
        animation_probability_micros: u32,
        animation_rate_ticks: u16,
        frame_count: u16,
    ) -> Self {
        let micros = animation_probability_micros.min(PROBABILITY_DENOMINATOR);
        Self {
            type_ref,
            animation_probability_micros: micros,
            animation_probability: TerrainSpawnProbability::from_micros(micros),
            animation_rate_ticks,
            frame_count,
            midpoint_frame: frame_count / 2,
            phase: TerrainSpawnerPhase::Idle,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.phase, TerrainSpawnerPhase::Active { .. })
    }

    fn can_animate(&self) -> bool {
        self.animation_rate_ticks > 0 && self.frame_count > 0
    }

    fn tick(&mut self, rng: &mut SimRng) -> TerrainSpawnerTick {
        match self.phase {
            TerrainSpawnerPhase::Idle => {
                if self.animation_probability_micros == 0 || !self.can_animate() {
                    return TerrainSpawnerTick::Idle;
                }
                if self.animation_probability.roll_succeeds(rng) {
                    self.phase = TerrainSpawnerPhase::Active {
                        current_frame: 0,
                        ticks_until_next_frame: self.animation_rate_ticks,
                    };
                    return TerrainSpawnerTick::AnimationStarted;
                }
                TerrainSpawnerTick::Idle
            }
            TerrainSpawnerPhase::Active {
                current_frame,
                ticks_until_next_frame,
            } => {
                let next_timer = ticks_until_next_frame.saturating_sub(1);
                if next_timer > 0 {
                    self.phase = TerrainSpawnerPhase::Active {
                        current_frame,
                        ticks_until_next_frame: next_timer,
                    };
                    return TerrainSpawnerTick::Active;
                }

                let next_frame = current_frame.saturating_add(1);
                if next_frame == self.midpoint_frame {
                    self.phase = TerrainSpawnerPhase::Idle;
                    TerrainSpawnerTick::SpawnDue
                } else {
                    self.phase = TerrainSpawnerPhase::Active {
                        current_frame: next_frame,
                        ticks_until_next_frame: self.animation_rate_ticks,
                    };
                    TerrainSpawnerTick::Active
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainSpawnerTick {
    Idle,
    AnimationStarted,
    Active,
    SpawnDue,
}

/// Short-lived mutation context for the stateful terrain spawner tick.
pub struct TerrainSpawnContext<'a> {
    pub resource_nodes: &'a mut BTreeMap<(u16, u16), ResourceNode>,
    pub overlay_grid: Option<&'a mut OverlayGrid>,
    pub default_ore_overlay_id: Option<u8>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub overlay_registry: Option<&'a crate::map::overlay_types::OverlayTypeRegistry>,
    pub path_grid: Option<&'a PathGrid>,
    pub rng: &'a mut SimRng,
}

impl<'a> TerrainSpawnContext<'a> {
    pub fn new(
        resource_nodes: &'a mut BTreeMap<(u16, u16), ResourceNode>,
        overlay_grid: Option<&'a mut OverlayGrid>,
        default_ore_overlay_id: Option<u8>,
        rng: &'a mut SimRng,
    ) -> Self {
        Self {
            resource_nodes,
            overlay_grid,
            default_ore_overlay_id,
            resolved_terrain: None,
            overlay_registry: None,
            path_grid: None,
            rng,
        }
    }

    pub fn with_validation_context(
        mut self,
        resolved_terrain: Option<&'a ResolvedTerrainGrid>,
        overlay_registry: Option<&'a crate::map::overlay_types::OverlayTypeRegistry>,
        path_grid: Option<&'a PathGrid>,
    ) -> Self {
        self.resolved_terrain = resolved_terrain;
        self.overlay_registry = overlay_registry;
        self.path_grid = path_grid;
        self
    }
}

/// Tick all terrain spawners using the verified delayed animation state machine.
///
/// Contract:
/// - idle spawners roll probability from raw `rng.next_u32()`;
/// - a hit starts frame 0 and never spawns on the same tick;
/// - active spawners do not roll probability;
/// - midpoint resets active state to idle before the forced spread attempt;
/// - placement only targets empty cells owned by this file's generic gates.
pub fn tick_terrain_spawners_stateful(
    spawners: &mut BTreeMap<(u16, u16), TerrainSpawnerState>,
    mut ctx: TerrainSpawnContext<'_>,
) {
    if spawners.is_empty() {
        return;
    }

    let spawner_cells: BTreeSet<(u16, u16)> = spawners.keys().copied().collect();
    for &cell in &spawner_cells {
        let Some(spawner) = spawners.get_mut(&cell) else {
            continue;
        };
        if spawner.tick(ctx.rng) != TerrainSpawnerTick::SpawnDue {
            continue;
        }

        try_spawn_ore(
            cell,
            ctx.resource_nodes,
            ctx.overlay_grid.as_deref_mut(),
            ctx.default_ore_overlay_id,
            &spawner_cells,
            ctx.resolved_terrain,
            ctx.path_grid,
            ctx.rng,
        );
    }
}

/// Compatibility shim for current world integration.
///
/// The verified state machine requires mutable `TerrainSpawnerState`; `World`
/// must switch to `tick_terrain_spawners_stateful` to enable TIBTRE spawning
/// again.
pub fn tick_terrain_spawners(
    _spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
    _resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    _overlay_grid: Option<&mut OverlayGrid>,
    _default_ore_overlay_id: Option<u8>,
    _path_grid: Option<&PathGrid>,
    _rng: &mut SimRng,
) {
}

/// Try to place ore in a random adjacent cell. Mirrors the 8-direction
/// random-start iteration from `ore_growth::try_spread_ore`, but accepts only
/// empty targets and creates a density-3 cell.
fn try_spawn_ore(
    source: (u16, u16),
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
    spawner_cells: &BTreeSet<(u16, u16)>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    path_grid: Option<&PathGrid>,
    rng: &mut SimRng,
) {
    let start_dir = rng.next_range_u32(8) as usize;

    for i in 0..8 {
        let dir = (start_dir + i) % 8;
        let (dx, dy) = ADJACENT_OFFSETS[dir];
        let nx = source.0 as i32 + dx;
        let ny = source.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
            continue;
        }
        let cell = (nx as u16, ny as u16);

        if !can_accept_tiberium(
            cell,
            resource_nodes,
            overlay_grid.as_deref(),
            spawner_cells,
            resolved_terrain,
            path_grid,
        ) {
            continue;
        }

        place_tiberium_empty(
            cell,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            default_ore_overlay_id,
        );
        return;
    }
}

/// Whether a cell can receive new ore from a terrain spawner.
///
/// Checks the verified stock placement gates available in sim state: target is
/// in bounds, has no ore/overlay, is not another spawning terrain object, is on
/// a flat buildable tile, is not a bridge deck/ramp, and the current resolved
/// tile type has `AllowTiberium=yes`.
fn can_accept_tiberium(
    cell: (u16, u16),
    resource_nodes: &BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&OverlayGrid>,
    spawner_cells: &BTreeSet<(u16, u16)>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    path_grid: Option<&PathGrid>,
) -> bool {
    if spawner_cells.contains(&cell) {
        return false;
    }
    if resource_nodes.contains_key(&cell) {
        return false;
    }
    if let Some(grid) = overlay_grid {
        if grid.cell(cell.0, cell.1).overlay_id.is_some() {
            return false;
        }
    }
    if let Some(grid) = resolved_terrain {
        let Some(terrain_cell) = grid.cell(cell.0, cell.1) else {
            return false;
        };
        if !resolved_cell_accepts_tiberium(terrain_cell) {
            return false;
        }
    } else if let Some(grid) = path_grid {
        if grid.cell(cell.0, cell.1).is_none() {
            return false;
        }
    }
    true
}

fn resolved_cell_accepts_tiberium(cell: &ResolvedTerrainCell) -> bool {
    if !cell.allows_tiberium {
        return false;
    }
    if cell.slope_type != 0 {
        return false;
    }
    if cell.base_build_blocked {
        return false;
    }
    cell.bridge_flags() & (BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DESTROYED_OR_RAMP) == 0
}

/// Place ore at `cell` with density `SPAWN_DENSITY_LEVELS`.
///
/// Caller must have already checked `can_accept_tiberium`, which guarantees the
/// cell is empty for the generic stores owned here.
fn place_tiberium_empty(
    cell: (u16, u16),
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
) {
    if overlay_grid.is_some() && default_ore_overlay_id.is_none() {
        return;
    }

    resource_nodes.insert(
        cell,
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: ORE_BASE_PER_LEVEL * SPAWN_DENSITY_LEVELS,
        },
    );

    if let Some(grid) = overlay_grid {
        if let Some(id) = default_ore_overlay_id {
            grid.place_overlay(cell.0, cell.1, id, SPAWN_DENSITY_LEVELS as u8);
        }
    }
}

/// Populate `production.terrain_spawners` from the map's terrain objects.
///
/// Terrain SHP frame counts are supplied by app/render-side asset loading before
/// entering sim. Sim stores only the numeric count, preserving the layering
/// boundary.
pub fn seed_terrain_spawners(
    sim: &mut crate::sim::world::Simulation,
    terrain_objects: &[crate::map::overlay::TerrainObject],
    rules: &crate::rules::ruleset::RuleSet,
    overlay_names: &BTreeMap<u8, String>,
    terrain_frame_counts: &BTreeMap<String, u16>,
) -> usize {
    sim.production.default_ore_overlay_id = overlay_names
        .iter()
        .find(|(_id, name)| name.to_ascii_uppercase().starts_with("TIB"))
        .map(|(id, _)| *id);

    let mut seeded = 0usize;
    for obj in terrain_objects {
        let Some(t) = rules.terrain_object_type_case_insensitive(&obj.name) else {
            continue;
        };
        if !t.spawns_tiberium || !t.is_animated {
            continue;
        }
        let frame_count = terrain_frame_counts
            .get(&obj.name)
            .or_else(|| terrain_frame_counts.get(&obj.name.to_ascii_uppercase()))
            .copied()
            .unwrap_or(0);
        let type_ref = sim.interner.intern(&obj.name);
        sim.production.terrain_spawners.insert(
            (obj.rx, obj.ry),
            TerrainSpawnerState::new(
                type_ref,
                t.animation_probability_micros,
                u16::from(t.animation_rate),
                frame_count,
            ),
        );
        seeded += 1;
    }
    seeded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::intern::StringInterner;

    const STOCK_FRAME_COUNT: u16 = 22;
    const STOCK_RATE: u16 = 3;

    fn resolved_cell() -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx: 0,
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
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: true,
            is_cliff_redraw: false,
            variant: 0,
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
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: SpeedCostProfile::default(),
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
        }
    }

    fn spawner(interner: &mut StringInterner, name: &str, prob_micros: u32) -> TerrainSpawnerState {
        TerrainSpawnerState::new(
            interner.intern(name),
            prob_micros,
            STOCK_RATE,
            STOCK_FRAME_COUNT,
        )
    }

    fn tick(
        spawners: &mut BTreeMap<(u16, u16), TerrainSpawnerState>,
        resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        rng: &mut SimRng,
    ) {
        tick_terrain_spawners_stateful(
            spawners,
            TerrainSpawnContext::new(resource_nodes, None, None, rng),
        );
    }

    #[test]
    fn raw_probability_sample_uses_signed_abs_mod_and_double_scale() {
        assert_eq!(raw_probability_sample(0), 0.0);
        assert_eq!(raw_probability_sample(0xFFFF_FFFF), 0.000001);
        assert_eq!(raw_probability_sample(1_000_001), 0.000001);
    }

    #[test]
    fn probability_uses_strict_less_boundary() {
        let p = TerrainSpawnProbability::from_micros(1);
        assert!(raw_probability_sample(0) < p.as_f64());
        assert!(!(raw_probability_sample(0xFFFF_FFFF) < p.as_f64()));
    }

    #[test]
    fn resolved_cell_gate_requires_flat_buildable_allow_tiberium_non_bridge() {
        let cell = resolved_cell();
        assert!(resolved_cell_accepts_tiberium(&cell));

        let mut no_allow = cell.clone();
        no_allow.allows_tiberium = false;
        assert!(!resolved_cell_accepts_tiberium(&no_allow));

        let mut sloped = cell.clone();
        sloped.slope_type = 1;
        assert!(!resolved_cell_accepts_tiberium(&sloped));

        let mut blocked = cell.clone();
        blocked.base_build_blocked = true;
        assert!(!resolved_cell_accepts_tiberium(&blocked));

        let mut bridge = cell;
        bridge.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        assert!(!resolved_cell_accepts_tiberium(&bridge));
    }

    #[test]
    fn probability_hit_does_not_spawn_same_tick() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        tick(&mut spawners, &mut resource_nodes, &mut rng);

        assert!(resource_nodes.is_empty());
        assert_eq!(
            spawners.get(&(10, 10)).unwrap().phase,
            TerrainSpawnerPhase::Active {
                current_frame: 0,
                ticks_until_next_frame: STOCK_RATE,
            }
        );
    }

    #[test]
    fn stock_rate3_spawns_33_ticks_after_probability_hit() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        tick(&mut spawners, &mut resource_nodes, &mut rng);
        for _ in 0..32 {
            tick(&mut spawners, &mut resource_nodes, &mut rng);
            assert!(resource_nodes.is_empty());
        }

        tick(&mut spawners, &mut resource_nodes, &mut rng);
        assert_eq!(resource_nodes.len(), 1);
        assert_eq!(
            spawners.get(&(10, 10)).unwrap().phase,
            TerrainSpawnerPhase::Idle
        );
    }

    #[test]
    fn active_animation_suppresses_probability_rolls() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        let mut state = spawner(&mut interner, "TIBTRE01", 1_000_000);
        state.phase = TerrainSpawnerPhase::Active {
            current_frame: 0,
            ticks_until_next_frame: STOCK_RATE,
        };
        spawners.insert((10, 10), state);
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(123);
        let before = rng.state();

        tick(&mut spawners, &mut resource_nodes, &mut rng);

        assert_eq!(
            rng.state(),
            before,
            "active non-midpoint tick consumes no RNG"
        );
        assert!(resource_nodes.is_empty());
    }

    #[test]
    fn probability_zero_never_starts_animation() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE_NEVER", 0));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        for _ in 0..1000 {
            tick(&mut spawners, &mut resource_nodes, &mut rng);
        }
        assert!(resource_nodes.is_empty());
        assert_eq!(
            spawners.get(&(10, 10)).unwrap().phase,
            TerrainSpawnerPhase::Idle
        );
    }

    #[test]
    fn spawn_on_empty_cell_creates_density_3_ore() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        for _ in 0..34 {
            tick(&mut spawners, &mut resource_nodes, &mut rng);
        }

        let node = resource_nodes.values().next().unwrap();
        assert_eq!(node.resource_type, ResourceType::Ore);
        assert_eq!(node.remaining, 360);
    }

    #[test]
    fn spawn_skips_existing_ore_neighbors_instead_of_growing_them() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        for &(dx, dy) in &ADJACENT_OFFSETS {
            if (dx, dy) == (1, 1) {
                continue;
            }
            resource_nodes.insert(
                ((10 + dx) as u16, (10 + dy) as u16),
                ResourceNode {
                    resource_type: ResourceType::Ore,
                    remaining: 240,
                },
            );
        }
        let mut rng = SimRng::new(7);

        for _ in 0..34 {
            tick(&mut spawners, &mut resource_nodes, &mut rng);
        }

        assert_eq!(resource_nodes.get(&(11, 11)).unwrap().remaining, 360);
        let grown_existing = resource_nodes
            .values()
            .filter(|n| n.remaining > 360)
            .count();
        assert_eq!(grown_existing, 0, "existing ore must not be additive-grown");
    }

    #[test]
    fn spawn_places_nothing_when_all_neighbors_have_resources() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        for &(dx, dy) in &ADJACENT_OFFSETS {
            resource_nodes.insert(
                ((10 + dx) as u16, (10 + dy) as u16),
                ResourceNode {
                    resource_type: ResourceType::Ore,
                    remaining: 240,
                },
            );
        }
        let mut rng = SimRng::new(7);

        for _ in 0..34 {
            tick(&mut spawners, &mut resource_nodes, &mut rng);
        }

        assert_eq!(resource_nodes.len(), 8);
        assert!(resource_nodes.values().all(|n| n.remaining == 240));
    }

    #[test]
    fn spawn_places_nothing_when_all_neighbors_have_overlays() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut overlay_grid = OverlayGrid::new(32, 32);
        for &(dx, dy) in &ADJACENT_OFFSETS {
            overlay_grid.place_overlay((10 + dx) as u16, (10 + dy) as u16, 5, 0);
        }
        let mut rng = SimRng::new(7);

        for _ in 0..34 {
            tick_terrain_spawners_stateful(
                &mut spawners,
                TerrainSpawnContext::new(
                    &mut resource_nodes,
                    Some(&mut overlay_grid),
                    Some(2),
                    &mut rng,
                ),
            );
        }

        assert!(resource_nodes.is_empty());
    }

    #[test]
    fn new_cell_overlay_data_is_three() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut overlay_grid = OverlayGrid::new(32, 32);
        let mut rng = SimRng::new(7);

        for _ in 0..34 {
            tick_terrain_spawners_stateful(
                &mut spawners,
                TerrainSpawnContext::new(
                    &mut resource_nodes,
                    Some(&mut overlay_grid),
                    Some(2),
                    &mut rng,
                ),
            );
        }

        let &(rx, ry) = resource_nodes.keys().next().unwrap();
        let overlay = overlay_grid.cell(rx, ry);
        assert_eq!(overlay.overlay_id, Some(2));
        assert_eq!(overlay.overlay_data, 3);
    }

    #[test]
    fn deterministic_same_seed_same_pattern() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE_HALF", 500_000));

        fn run(
            source: &BTreeMap<(u16, u16), TerrainSpawnerState>,
            seed: u64,
        ) -> BTreeMap<(u16, u16), ResourceNode> {
            let mut spawners = source.clone();
            let mut nodes = BTreeMap::new();
            let mut rng = SimRng::new(seed);
            for _ in 0..200 {
                tick(&mut spawners, &mut nodes, &mut rng);
            }
            nodes
        }

        let a = run(&spawners, 42);
        let b = run(&spawners, 42);
        assert_eq!(a, b, "same seed must produce identical state");
    }

    #[test]
    fn seed_filters_to_spawning_animated_types_and_caches_probability_and_rate() {
        use crate::map::overlay::TerrainObject;
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        use crate::sim::world::Simulation;

        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TerrainTypes]\n1=TIBTRE01\n2=TREE01\n\
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\n\
             AnimationRate=3\nAnimationProbability=.003\n\
             [TREE01]\nSpawnsTiberium=no\nIsAnimated=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let mut sim = Simulation::new();
        let mut overlay_names = BTreeMap::new();
        overlay_names.insert(2u8, "TIB1".to_string());
        overlay_names.insert(7u8, "RUBBLE".to_string());
        let mut terrain_frame_counts = BTreeMap::new();
        terrain_frame_counts.insert("TIBTRE01".to_string(), STOCK_FRAME_COUNT);

        let objs = vec![
            TerrainObject {
                rx: 5,
                ry: 6,
                name: "TIBTRE01".to_string(),
            },
            TerrainObject {
                rx: 8,
                ry: 9,
                name: "TREE01".to_string(),
            },
            TerrainObject {
                rx: 1,
                ry: 2,
                name: "UNKNOWN".to_string(),
            },
        ];
        let seeded = seed_terrain_spawners(
            &mut sim,
            &objs,
            &rules,
            &overlay_names,
            &terrain_frame_counts,
        );
        assert_eq!(seeded, 1);
        let placed = sim
            .production
            .terrain_spawners
            .get(&(5, 6))
            .expect("TIBTRE01 seeded at (5,6)");
        assert_eq!(placed.animation_probability_micros, 3000);
        assert_eq!(
            placed.animation_probability,
            TerrainSpawnProbability::from_micros(3000)
        );
        assert_eq!(placed.animation_rate_ticks, 3);
        assert_eq!(placed.frame_count, STOCK_FRAME_COUNT);
        assert_eq!(placed.midpoint_frame, STOCK_FRAME_COUNT / 2);
        assert_eq!(sim.production.default_ore_overlay_id, Some(2));
    }
}
