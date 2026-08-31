//! Headless retail-scenario loading for cross-engine parity runs.
//!
//! Loads a real retail map with production rules, art, theater and terrain, then
//! constructs a `Simulation` from an explicit seed — no GPU, no window, no atlases.
//! Depends on `assets`/`rules`/`map`/`sim` only; nothing here may reach into `render`,
//! `ui`, `sidebar`, `audio` or `net`.
//!
//! **Scope.** Construction goes through the same GPU-free funnel the app uses
//! (`sim::runtime::construct_scenario`): map-roster houses are created before objects,
//! terrain objects before map entities, and map-placed units/structures spawn with
//! terrain-attached animations. What a headless scenario still lacks versus an app
//! launch is the launch *session* — skirmish player houses, start-position placement,
//! and atlas-derived voxel animation frame counts (a GPU concern).
//!
//! The seed contract mirrors the original engine: one 32-bit word seeds the scenario and
//! main streams identically, fixed before any setup-phase draw.

use std::collections::BTreeMap;
use std::path::Path;

use crate::assets::asset_manager::AssetManager;
use crate::map::map_file::{self, MapFile};
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::theater;
use crate::map::tile_variant_selector::TileVariantSelectorCache;
use crate::map::waypoints;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::scenario_bootstrap::ScenarioBootstrapRng;
use crate::sim::scenario_session::ScenarioDescriptor;
use crate::sim::world::Simulation;

/// Terrain plus the exact setup RNG owner advanced while that terrain loaded.
struct HeadlessTerrainBootstrap {
    resolved: ResolvedTerrainGrid,
    bootstrap_rng: ScenarioBootstrapRng,
}

impl HeadlessTerrainBootstrap {
    #[allow(clippy::too_many_arguments)]
    fn construct_scenario<F>(
        self,
        map: &MapFile,
        theater_name: &str,
        rules: Option<&crate::rules::ruleset::RuleSet>,
        art: Option<&crate::rules::art_data::ArtRegistry>,
        height_map: &BTreeMap<(u16, u16), u8>,
        overlay_registry: Option<&OverlayTypeRegistry>,
        overlay_grid: Option<&OverlayGrid>,
        bridge_destroyability_mode: crate::map::basic::BridgeDestroyabilityMode,
        descriptor: &ScenarioDescriptor,
        initialize_houses_before_objects: F,
    ) -> Simulation
    where
        F: FnOnce(&mut Simulation),
    {
        crate::sim::runtime::construct_scenario(
            map,
            &self.resolved,
            theater_name,
            rules,
            art,
            height_map,
            overlay_registry,
            overlay_grid,
            bridge_destroyability_mode,
            descriptor,
            self.bootstrap_rng,
            initialize_houses_before_objects,
        )
    }
}

/// Build the ordinary-load CellClass population and retain the RNG cursors it
/// advanced for the subsequent sim handoff.
///
/// Active YR `MapClass::Clear @ 0x00565B00` clears the fixed cell table, then
/// `MapClass::Resize @ 0x00565C10` allocates the complete Size diamond before
/// IsoMapPack records overwrite it (allocation loop `0x0056639E..0x00566451`).
#[allow(clippy::too_many_arguments)]
fn build_headless_terrain_bootstrap(
    map: &MapFile,
    theater_data: Option<&crate::map::theater::TheaterData>,
    asset_manager: Option<&AssetManager>,
    terrain_rules: Option<&crate::rules::terrain_rules::TerrainRules>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    cliff_back_impassability: u8,
    seed: u32,
) -> HeadlessTerrainBootstrap {
    let mut bootstrap_rng = ScenarioBootstrapRng::new(seed);
    let (mut scenario_fill_rng, mut variant_main_rng) = bootstrap_rng.terrain_draws();
    let mut scenario_fill_ranged =
        |low, high| scenario_fill_rng.next_range_u32_inclusive(low, high);
    let mut variant_draw = || variant_main_rng.next_u32();
    let mut variant_selector_cache = TileVariantSelectorCache::default();
    let mut variant_selector = variant_selector_cache.begin_load(&mut variant_draw);
    // Each headless parity run owns one process-shaped MapClass identity. The
    // Resize constructor runs before OverlayPack bridge marking, so missing
    // bridge neighbors must already target this handle during resolution.
    let shared_cell_dummy = crate::map::resolved_terrain::SharedCellDummy::fresh();
    shared_cell_dummy.reconstruct_for_map_resize();
    let resolved = ResolvedTerrainGrid::build_with_variant_selector_and_shared_dummy(
        map,
        theater_data,
        asset_manager,
        terrain_rules,
        overlay_registry,
        // Headless terrain-object metadata and LAT remain explicit residuals.
        None,
        false,
        cliff_back_impassability,
        &mut scenario_fill_ranged,
        &mut variant_selector,
        shared_cell_dummy,
        crate::map::resolved_terrain::OverlayLoadSource::Authored,
    );
    drop(variant_selector);
    drop(variant_draw);
    drop(scenario_fill_ranged);
    drop(variant_main_rng);
    drop(scenario_fill_rng);

    HeadlessTerrainBootstrap {
        resolved,
        bootstrap_rng,
    }
}

/// A loaded scenario plus the per-tick inputs `advance_tick` needs.
pub struct HeadlessScenario {
    /// The runtime owner (F09): simulation plus its bound immutable
    /// resources - no independently swappable execution inputs remain.
    pub runtime: crate::sim::runtime::SimRuntime,
    pub map: MapFile,
}

impl HeadlessScenario {
    /// Read access for digest/tooling callers.
    pub fn sim(&self) -> &Simulation {
        &self.runtime.simulation
    }
}

/// Native logic-frame cadence (66 ms). NOTE: this does NOT match the
/// client, which steps the sim at `app::types::SIM_TICK_MS` = 22 ms — a
/// recorded 3x tooling divergence; headless digests are not tick-comparable
/// to client runs until it is resolved.
pub const SIM_TICK_MS: u32 = 1000 / crate::util::fixed_math::RA2_LOGIC_FRAMES_PER_SECOND;

/// Load `map_file_name` from the retail install at `retail_dir` with a pinned seed.
///
/// `map_file_name` is resolved relative to the retail root (e.g. `"Dustbowl.mmx"`).
pub fn load(retail_dir: &Path, map_file_name: &str, seed: u32) -> Result<HeadlessScenario, String> {
    crate::map::retail_trig::install_from_dir(retail_dir);
    if !crate::map::retail_trig::wave_tables_available() {
        return Err(format!(
            "{} does not provide the verified gamemd sine/Acos tables required by stock Sonic Wave simulation",
            retail_dir.join("gamemd.exe").display()
        ));
    }
    let map_path = retail_dir.join(map_file_name);
    let map = map_file::load_from_path(&map_path)
        .map_err(|error| format!("parse {}: {error}", map_path.display()))?;

    let mut assets = AssetManager::new(retail_dir).map_err(|error| {
        format!(
            "open retail MIX archives in {}: {error}",
            retail_dir.display()
        )
    })?;
    let theater = theater::load_theater(&mut assets, &map.header.theater)
        .ok_or_else(|| format!("load theater {}", map.header.theater))?;

    // Production rules layering: RULESMD, then the map's own INI on top.
    let (mut rules, rules_ini, _native_type_construction_trace, art_ini) =
        crate::app::loading::init_helpers::load_rules_with_merged_ini(
            &assets,
            None,
            Some(&map.ini),
        )
        .ok_or_else(|| "load merged rules".to_string())?
        .into_parts();
    let mut art = crate::rules::art_data::ArtRegistry::from_ini(&art_ini);
    rules.merge_art_data(&art);
    rules.general.resolve_art_rates(&art_ini);
    let infantry_sequences =
        crate::rules::infantry_sequence::parse_infantry_sequence_registry(&art_ini);
    let overlay_registry = OverlayTypeRegistry::from_ini(&rules_ini, Some(&art_ini));

    let mut terrain_bootstrap = build_headless_terrain_bootstrap(
        &map,
        Some(&theater),
        Some(&assets),
        Some(&rules.terrain_rules),
        Some(&overlay_registry),
        rules.general.cliff_back_impassability,
        seed,
    );
    // The process-shaped dummy was bound before OverlayPack stamping inside
    // the bootstrap. Every grid/sim clone inside this run now shares it;
    // separate loads remain isolated.
    let scheduler_roots = crate::app::loading::init_helpers::scheduler_anim_roots(
        &rules,
        terrain_bootstrap.resolved.tile_animations(),
    );
    art.bind_scheduler_anim_assets(
        &scheduler_roots,
        &assets,
        theater.extension,
        &map.header.theater,
    )
    .map_err(|error| format!("bind authoritative animation assets: {error}"))?;
    let (populated_smudge_dims, fallback_smudge_dims) =
        art.populate_anim_frame_dims(&assets, theater.extension, &map.header.theater);
    log::info!(
        "Anim frame dims: {} populated, {} fallback (defaults to 30x30)",
        populated_smudge_dims,
        fallback_smudge_dims,
    );
    rules.art_registry = art;
    rules.bind_effect_assets(&assets, theater.extension, &map.header.theater);
    rules.bind_terrain_spawner_assets(&rules_ini, &assets, theater.extension, &map.header.theater);
    rules.bind_animation_sequences(&infantry_sequences);
    let house_roster = crate::map::houses::parse_house_roster(
        &map.ini,
        rules.color_schemes.as_slice(),
        Some(&rules),
    );
    let mut overlay_grid = OverlayGrid::from_overlay_entries(
        &map.overlays,
        terrain_bootstrap.resolved.width(),
        terrain_bootstrap.resolved.height(),
    );
    let cleared_terrain_overlay_cells =
        crate::sim::terrain_spawn::clear_tiberium_source_cells_for_terrain(
            &mut overlay_grid,
            &mut terrain_bootstrap.resolved,
            &map.terrain_objects,
            &rules,
            &overlay_registry,
        );
    if !cleared_terrain_overlay_cells.is_empty() {
        log::info!(
            "Cleared {} same-cell tiberium overlay cell(s) for recognized terrain",
            cleared_terrain_overlay_cells.len(),
        );
    }
    let height_map = terrain_bootstrap.resolved.build_height_map();
    let lighting_profiles = crate::map::lighting::parse_lighting_profiles(&map.ini);

    let descriptor = ScenarioDescriptor {
        seed,
        map_name: map_file_name.to_string(),
        theater: map.header.theater.clone(),
        // A parity run stands in for a skirmish launch, which is a nonzero native mode.
        game_mode_nonzero: true,
        no_damage: false,
        // CANONICAL CELL-ARRAY FRAME, not [Map] Size= — see ScenarioDescriptor.
        map_width: terrain_bootstrap.resolved.width(),
        map_height: terrain_bootstrap.resolved.height(),
        local_left: map.header.local_left as u16,
        local_top: map.header.local_top as u16,
        local_width: map.header.local_width as u16,
        local_height: map.header.local_height as u16,
        mp_start_waypoints: waypoints::multiplayer_start_waypoints(&map.waypoints)
            .into_iter()
            .map(|wp| (wp.index, (wp.rx, wp.ry)))
            .collect(),
        lighting: crate::sim::scenario_session::ScenarioLightingState::new(
            crate::sim::scenario_session::ScenarioLightProfileUnits {
                ambient_percent: lighting_profiles.normal.ambient_percent,
                red_percent: lighting_profiles.normal.red_percent,
                green_percent: lighting_profiles.normal.green_percent,
                blue_percent: lighting_profiles.normal.blue_percent,
                ground_units: lighting_profiles.normal.ground_units,
                level_units: lighting_profiles.normal.level_units,
            },
            crate::sim::scenario_session::ScenarioLightProfileUnits {
                ambient_percent: lighting_profiles.ion.ambient_percent,
                red_percent: lighting_profiles.ion.red_percent,
                green_percent: lighting_profiles.ion.green_percent,
                blue_percent: lighting_profiles.ion.blue_percent,
                ground_units: lighting_profiles.ion.ground_units,
                level_units: lighting_profiles.ion.level_units,
            },
        ),
    };

    // F09: the same GPU-free construction funnel the app uses — bootstrap
    // RNG, map-roster houses before objects, terrain objects before map
    // entities, entity spawn, and terrain-attached animations — then the
    // shared post-funnel finalization (spawner seed, wall owners, overlay
    // grid, smudge grid, post-map). A parity run stands in for a stock
    // skirmish load with bridges destructible, the retail skirmish default.
    let mut sim = terrain_bootstrap.construct_scenario(
        &map,
        &map.header.theater,
        Some(&rules),
        Some(&rules.art_registry),
        &height_map,
        Some(&overlay_registry),
        Some(&overlay_grid),
        crate::map::basic::BridgeDestroyabilityMode::SkirmishOrMultiplayer {
            bridge_destruction: true,
        },
        &descriptor,
        |sim| {
            crate::sim::scenario_bootstrap::initialize_map_roster_houses(
                sim,
                &house_roster,
                Some(&rules),
            );
        },
    );
    // F09: bind HVA-driven voxel animation frame counts through the same
    // GPU-free catalog the app uses; headless previously ran every voxel
    // animation at its 1-frame default.
    let frame_catalog = crate::sim::voxel_frame_catalog::build_voxel_frame_catalog(
        sim.entities(),
        &sim.interner,
        &assets,
        Some(&rules),
        Some(&rules.art_registry),
    );
    sim.update_voxel_anim_frame_counts(&frame_catalog);
    let post_map = crate::sim::runtime::finalize_constructed_scenario(
        &mut sim,
        &map,
        &rules,
        &overlay_registry,
        overlay_grid,
        &house_roster,
        None,
    );
    if !post_map.navigation_published {
        return Err("publish headless post-map navigation".to_string());
    }
    if sim.path_grid().is_none() {
        return Err("headless post-map navigation is unavailable".to_string());
    }
    // The production-side resource-node index is deliberately not seeded: its helper is
    // `#[cfg(test)]`-gated. Map-placed entities now spawn, so a map that pre-places a
    // miner would find no node index — a documented residual until the helper is
    // promoted out of test gating. Ore is still present as map overlays.

    Ok(HeadlessScenario {
        runtime: crate::sim::runtime::SimRuntime {
            simulation: sim,
            resources: crate::sim::runtime::SimResources {
                height_map,
                bridge_height_map: BTreeMap::new(),
                overlay_registry,
                terrain_template: None,
                rules,
                trigger_graph: Default::default(),
                triggers: Default::default(),
                events: Default::default(),
                actions: Default::default(),
                waypoints: map.waypoints.clone(),
            },
        },
        map,
    })
}

impl HeadlessScenario {
    /// Advance one committed simulation frame with no player commands,
    /// through the same bound-resource runtime transaction the app uses.
    pub fn tick(&mut self) {
        let _ = self
            .runtime
            .advance_frame(&[], SIM_TICK_MS, crate::sim::world::TickLane::Ordinary);
    }
}

#[cfg(test)]
mod retail_construction_tests {
    use super::*;
    use crate::sim::rng::SimRng;

    /// One valid LZO chunk whose decompressed bytes are the `(0, 0)`
    /// IsoMapPack terminator, so the parsed map has no explicit cell records.
    const EMPTY_ISO_MAP_PACK: &str = "CAAEABUAAAAAEQAA";

    #[test]
    fn gsi_04_01_headless_sparse_water_load_materializes_and_transfers_rng() {
        let seed = 0x0401_5EED;
        let map_bytes = format!(
            "[Map]\n\
             Theater=TEMPERATE\n\
             Size=0,0,2,1\n\
             LocalSize=0,0,2,1\n\
             Fill=Water\n\
             [IsoMapPack5]\n\
             1={EMPTY_ISO_MAP_PACK}\n"
        );
        let map = MapFile::from_bytes(map_bytes.as_bytes()).expect("parse sparse INI map");
        assert!(
            map.cells.is_empty(),
            "fixture has no explicit terrain cells"
        );

        let terrain_bootstrap =
            build_headless_terrain_bootstrap(&map, None, None, None, None, 2, seed);
        assert_eq!(
            (
                terrain_bootstrap.resolved.width(),
                terrain_bootstrap.resolved.height(),
            ),
            (3, 3),
            "the canonical cell array spans the Size diamond's highest coordinate"
        );
        let mut allocated: Vec<_> = terrain_bootstrap
            .resolved
            .iter()
            .map(|cell| (cell.rx, cell.ry))
            .collect();
        allocated.sort_unstable();
        assert_eq!(allocated, vec![(1, 2), (2, 1), (2, 2)]);

        let height_map = terrain_bootstrap.resolved.build_height_map();
        let descriptor = ScenarioDescriptor {
            seed,
            map_width: terrain_bootstrap.resolved.width(),
            map_height: terrain_bootstrap.resolved.height(),
            local_left: map.header.local_left as u16,
            local_top: map.header.local_top as u16,
            local_width: map.header.local_width as u16,
            local_height: map.header.local_height as u16,
            ..ScenarioDescriptor::default()
        };
        let sim = terrain_bootstrap.construct_scenario(
            &map,
            &map.header.theater,
            None,
            None,
            &height_map,
            None,
            None,
            crate::map::basic::BridgeDestroyabilityMode::SkirmishOrMultiplayer {
                bridge_destruction: true,
            },
            &descriptor,
            |_| {},
        );

        let mut expected_scenario = SimRng::new(u64::from(seed));
        for _ in 0..3 {
            let _ = expected_scenario.next_range_u32_inclusive(0, 3);
        }
        let state = sim.rng_state();
        assert_eq!(state.scenario, expected_scenario.logical_state());
        assert_eq!(
            state.main,
            SimRng::new(u64::from(seed)).logical_state(),
            "no theater TMP selection means the Main cursor stays at its seed"
        );
    }

    /// F09 certification: the shared GPU-free funnel produces a deterministic,
    /// fully populated headless scenario on a retail map. Two loads of the same
    /// map and seed must yield identical parity digests at construction and on
    /// every tick, and the construction gains the funnel promises — map-roster
    /// houses, overlay/smudge/bridge authority, published navigation — must all
    /// be present. App-vs-headless assembly drift is prevented structurally:
    /// both call `construct_scenario` + `finalize_constructed_scenario`.
    #[test]
    #[ignore = "requires RA2_DIR with installed retail RA2/YR assets"]
    fn retail_headless_funnel_construction_is_deterministic_and_populated() {
        let ra2 = std::path::PathBuf::from(
            std::env::var("RA2_DIR").expect("set RA2_DIR to the retail RA2/YR install directory"),
        );
        let seed = 0x00C0_FFEE;
        let mut a = load(&ra2, "Dustbowl.mmx", seed).expect("first headless load");
        let mut b = load(&ra2, "Dustbowl.mmx", seed).expect("second headless load");

        assert_eq!(
            a.sim().parity_digest(),
            b.sim().parity_digest(),
            "same map+seed must produce an identical construction fingerprint"
        );
        assert!(
            !a.sim().houses.is_empty(),
            "map-roster houses must be constructed before objects"
        );
        assert!(
            a.sim().overlay_grid.is_some(),
            "overlay authority must be installed by the shared finalization"
        );
        assert!(
            a.sim().smudge_grid.is_some(),
            "smudge authority must be installed by the shared finalization"
        );
        assert!(
            a.sim().bridge_state.is_some(),
            "bridge runtime state must be constructed by the funnel"
        );

        for tick in 0..30u32 {
            a.tick();
            b.tick();
            assert_eq!(
                a.sim().parity_digest(),
                b.sim().parity_digest(),
                "runtime-backed headless execution diverged at tick {tick}"
            );
        }
    }
}
