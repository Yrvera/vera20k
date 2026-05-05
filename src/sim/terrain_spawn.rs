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

use std::collections::BTreeMap;

use crate::sim::intern::InternedId;
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::rng::SimRng;

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

/// Base ore stock per density level. Matches `ore_growth::ORE_BASE_PER_LEVEL`
/// and `seed_resource_nodes_from_overlays`.
const ORE_BASE_PER_LEVEL: u16 = 120;
/// Maximum ore stock (12 levels × 120). Matches `ore_growth::MAX_ORE_REMAINING`.
const MAX_ORE_REMAINING: u16 = ORE_BASE_PER_LEVEL * 12;
/// Density levels added per spawn. Matches binary's `PlaceTiberium(tib_type, 3)`.
const SPAWN_DENSITY_LEVELS: u16 = 3;
/// Probability roll denominator. Matches binary's `random % 1_000_000`
/// against `AnimationProbability` scaled by 1.0e-6.
const PROBABILITY_DENOMINATOR: u32 = 1_000_000;

/// Tick all terrain spawners.
///
/// Called once per sim tick from `World::advance_tick` (Phase 7), AFTER
/// `tick_ore_growth` so a spawn this tick can't be grown/spread until the
/// next tick.
///
/// Determinism: BTreeMap iteration is sorted by cell. Each spawner draws
/// one RNG value for the probability roll; on a hit, one more for the
/// 8-direction start. Same seed + same map → identical spawn pattern.
pub fn tick_terrain_spawners(
    spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
    path_grid: Option<&PathGrid>,
    rng: &mut SimRng,
) {
    if spawners.is_empty() {
        return;
    }

    let mut overlay_grid = overlay_grid;

    for (&(rx, ry), spawner) in spawners {
        // Defensive: probability=0 means seed inserted a no-spawn entry.
        if spawner.animation_probability_micros == 0 {
            continue;
        }

        let roll = rng.next_range_u32(PROBABILITY_DENOMINATOR);
        if roll >= spawner.animation_probability_micros {
            continue;
        }

        try_spawn_ore(
            (rx, ry),
            resource_nodes,
            overlay_grid.as_deref_mut(),
            default_ore_overlay_id,
            path_grid,
            spawners,
            rng,
        );
    }
}

/// Try to place ore in a random adjacent cell. Mirrors the 8-direction
/// random-start iteration from `ore_growth::try_spread_ore`, but uses
/// the additive density-3 place primitive.
fn try_spawn_ore(
    source: (u16, u16),
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
    path_grid: Option<&PathGrid>,
    spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
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

        if !can_accept_tiberium(cell, resource_nodes, path_grid, spawners) {
            continue;
        }

        place_tiberium_additive(
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
/// Maps onto binary's `CanAcceptTiberium` checks:
/// - Cell walkable (rejects buildings/cliffs/water — approximate match for
///   "no living building" + "passable land type")
/// - No other SpawnsTiberium TerrainClass on the cell
/// - Cell does not already hold a non-ore resource (gems). Rejecting here
///   (rather than silently no-op'ing inside the place fn) lets the 8-direction
///   loop continue past gems to find an empty cell.
///
/// Existing ORE on the cell is NOT a rejection reason — place is additive.
fn can_accept_tiberium(
    cell: (u16, u16),
    resource_nodes: &BTreeMap<(u16, u16), ResourceNode>,
    path_grid: Option<&PathGrid>,
    spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
) -> bool {
    if let Some(grid) = path_grid {
        if !grid.is_walkable(cell.0, cell.1) {
            return false;
        }
    }
    if spawners.contains_key(&cell) {
        return false;
    }
    if let Some(existing) = resource_nodes.get(&cell) {
        if existing.resource_type != ResourceType::Ore {
            return false;
        }
    }
    true
}

/// Place ore at `cell` with density `SPAWN_DENSITY_LEVELS`, additive on existing.
///
/// Caller must have already checked `can_accept_tiberium`, which guarantees
/// the cell is either empty or holds ore (not gems).
fn place_tiberium_additive(
    cell: (u16, u16),
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    default_ore_overlay_id: Option<u8>,
) {
    let density_stock: u16 = ORE_BASE_PER_LEVEL * SPAWN_DENSITY_LEVELS;

    let (new_remaining, was_empty) = match resource_nodes.get(&cell) {
        Some(existing) => {
            debug_assert_eq!(existing.resource_type, ResourceType::Ore);
            let r = existing.remaining.saturating_add(density_stock);
            (r.min(MAX_ORE_REMAINING), false)
        }
        None => (density_stock, true),
    };

    resource_nodes.insert(
        cell,
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: new_remaining,
        },
    );

    if let Some(grid) = overlay_grid {
        let target_frame: u8 = (new_remaining / ORE_BASE_PER_LEVEL)
            .saturating_sub(1)
            .min(11) as u8;
        if was_empty {
            if let Some(id) = default_ore_overlay_id {
                grid.place_overlay(cell.0, cell.1, id, target_frame);
            }
        } else {
            grid.set_overlay_data(cell.0, cell.1, target_frame);
        }
    }
}

/// Populate `production.terrain_spawners` from the map's terrain objects.
///
/// For each `TerrainObject` whose name matches a TerrainObjectType with
/// `spawns_tiberium = true && is_animated = true`, insert a spawner state
/// keyed by cell with `animation_probability_micros` cached from rules.
/// Returns the count seeded.
///
/// Also resolves `production.default_ore_overlay_id` from `overlay_names`
/// (first entry whose uppercase name starts with "TIB"). Used as the fallback
/// overlay_id when TIBTRE spawns ore on a previously empty cell.
pub fn seed_terrain_spawners(
    sim: &mut crate::sim::world::Simulation,
    terrain_objects: &[crate::map::overlay::TerrainObject],
    rules: &crate::rules::ruleset::RuleSet,
    overlay_names: &BTreeMap<u8, String>,
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
        let type_ref = sim.interner.intern(&obj.name);
        sim.production.terrain_spawners.insert(
            (obj.rx, obj.ry),
            TerrainSpawnerState {
                type_ref,
                animation_probability_micros: t.animation_probability_micros,
            },
        );
        seeded += 1;
    }
    seeded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::intern::StringInterner;

    fn spawner(interner: &mut StringInterner, name: &str, prob_micros: u32) -> TerrainSpawnerState {
        TerrainSpawnerState {
            type_ref: interner.intern(name),
            animation_probability_micros: prob_micros,
        }
    }

    #[test]
    fn probability_one_always_spawns_within_one_tick() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        assert_eq!(resource_nodes.len(), 1);
        let &cell = resource_nodes.keys().next().unwrap();
        let dx = (cell.0 as i32 - 10).abs();
        let dy = (cell.1 as i32 - 10).abs();
        assert!(dx <= 1 && dy <= 1 && (dx + dy) > 0);
    }

    #[test]
    fn probability_zero_never_spawns() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE_NEVER", 0));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        for _ in 0..1000 {
            tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        }
        assert!(resource_nodes.is_empty());
    }

    #[test]
    fn spawn_on_empty_cell_creates_density_3_ore() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        let mut rng = SimRng::new(7);

        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        let node = resource_nodes.values().next().unwrap();
        assert_eq!(node.resource_type, ResourceType::Ore);
        assert_eq!(node.remaining, 360);
    }

    #[test]
    fn spawn_is_additive_on_existing_ore() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Ore,
                    remaining: 240,
                },
            );
        }
        let mut rng = SimRng::new(7);

        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        let added = resource_nodes
            .values()
            .filter(|n| n.remaining == 600)
            .count();
        assert_eq!(added, 1, "exactly one neighbor should grow by 360 stock");
    }

    #[test]
    fn spawn_never_overwrites_gem_cells() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Gem,
                    remaining: 360,
                },
            );
        }
        let mut rng = SimRng::new(7);
        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        for n in resource_nodes.values() {
            assert_eq!(n.resource_type, ResourceType::Gem);
            assert_eq!(n.remaining, 360);
        }
    }

    #[test]
    fn spawn_skips_gem_neighbors_and_picks_empty_cell() {
        // Regression test for the gem-cell asymmetry caught in /review-plan:
        // when the random-start direction lands on a gem cell, try_spawn_ore
        // must continue iterating to find an empty neighbor, not consume its
        // single placement chance silently.
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        // 7 of 8 neighbors are gems; the SE cell at offset (1, 1) is empty.
        for &(dx, dy) in &ADJACENT_OFFSETS {
            if (dx, dy) == (1, 1) {
                continue;
            }
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Gem,
                    remaining: 360,
                },
            );
        }
        let mut rng = SimRng::new(7);
        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        let se = resource_nodes
            .get(&(11, 11))
            .expect("SE neighbor should exist after spawn");
        assert_eq!(se.resource_type, ResourceType::Ore);
        assert_eq!(se.remaining, 360);
        for &(dx, dy) in &ADJACENT_OFFSETS {
            if (dx, dy) == (1, 1) {
                continue;
            }
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            let n = resource_nodes.get(&cell).expect("gem still present");
            assert_eq!(n.resource_type, ResourceType::Gem);
            assert_eq!(n.remaining, 360);
        }
    }

    #[test]
    fn spawn_caps_at_max_remaining() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE01", 1_000_000));
        let mut resource_nodes = BTreeMap::new();
        for &(dx, dy) in &ADJACENT_OFFSETS {
            let cell = ((10 + dx) as u16, (10 + dy) as u16);
            resource_nodes.insert(
                cell,
                ResourceNode {
                    resource_type: ResourceType::Ore,
                    remaining: 1320,
                },
            );
        }
        let mut rng = SimRng::new(7);
        tick_terrain_spawners(&spawners, &mut resource_nodes, None, None, None, &mut rng);
        let capped = resource_nodes
            .values()
            .filter(|n| n.remaining == MAX_ORE_REMAINING)
            .count();
        assert_eq!(capped, 1);
    }

    #[test]
    fn seed_filters_to_spawning_animated_types_and_caches_probability() {
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
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\nAnimationProbability=.003\n\
             [TREE01]\nSpawnsTiberium=no\nIsAnimated=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let mut sim = Simulation::new();
        let mut overlay_names = BTreeMap::new();
        overlay_names.insert(2u8, "TIB1".to_string());
        overlay_names.insert(7u8, "RUBBLE".to_string());

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
        let seeded = seed_terrain_spawners(&mut sim, &objs, &rules, &overlay_names);
        assert_eq!(seeded, 1);
        let placed = sim
            .production
            .terrain_spawners
            .get(&(5, 6))
            .expect("TIBTRE01 seeded at (5,6)");
        assert_eq!(placed.animation_probability_micros, 3000);
        assert_eq!(sim.production.default_ore_overlay_id, Some(2));
    }

    #[test]
    fn full_pipeline_seeds_then_ticks_until_spawn() {
        use crate::map::overlay::TerrainObject;
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        use crate::sim::world::Simulation;

        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TerrainTypes]\n1=TIBTRE01\n\
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\n\
             AnimationRate=3\nAnimationProbability=.5\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let mut sim = Simulation::new();
        let objs = vec![TerrainObject {
            rx: 20,
            ry: 20,
            name: "TIBTRE01".into(),
        }];
        let mut overlay_names = BTreeMap::new();
        overlay_names.insert(102u8, "TIB1".to_string());
        seed_terrain_spawners(&mut sim, &objs, &rules, &overlay_names);
        assert_eq!(sim.production.terrain_spawners.len(), 1);
        let placed = sim.production.terrain_spawners.get(&(20, 20)).unwrap();
        assert_eq!(placed.animation_probability_micros, 500_000);

        let mut rng = SimRng::new(99);
        let mut spawned = false;
        for _ in 0..50 {
            tick_terrain_spawners(
                &sim.production.terrain_spawners,
                &mut sim.production.resource_nodes,
                None,
                sim.production.default_ore_overlay_id,
                None,
                &mut rng,
            );
            if !sim.production.resource_nodes.is_empty() {
                spawned = true;
                break;
            }
        }
        assert!(spawned, "TIBTRE should spawn within 50 ticks at p=0.5");
    }

    #[test]
    fn deterministic_same_seed_same_pattern() {
        let mut interner = StringInterner::default();
        let mut spawners = BTreeMap::new();
        spawners.insert((10, 10), spawner(&mut interner, "TIBTRE_HALF", 500_000));

        fn run(
            spawners: &BTreeMap<(u16, u16), TerrainSpawnerState>,
            seed: u64,
        ) -> BTreeMap<(u16, u16), ResourceNode> {
            let mut nodes = BTreeMap::new();
            let mut rng = SimRng::new(seed);
            for _ in 0..200 {
                tick_terrain_spawners(spawners, &mut nodes, None, None, None, &mut rng);
            }
            nodes
        }

        let a = run(&spawners, 42);
        let b = run(&spawners, 42);
        assert_eq!(a, b, "same seed must produce identical state");
    }
}
