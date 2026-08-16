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
use crate::map::waypoints;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::scenario_session::ScenarioDescriptor;
use crate::sim::world::Simulation;

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

/// Matches the client's simulation cadence so tick numbering is comparable.
pub const SIM_TICK_MS: u32 = 1000 / 15;

/// Load `map_file_name` from the retail install at `retail_dir` with a pinned seed.
///
/// `map_file_name` is resolved relative to the retail root (e.g. `"Dustbowl.mmx"`).
pub fn load(retail_dir: &Path, map_file_name: &str, seed: u32) -> Result<HeadlessScenario, String> {
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
    let (mut rules, rules_ini) =
        crate::app_init_helpers::load_rules_with_merged_ini(&assets, None, Some(&map.ini))
            .ok_or_else(|| "load merged rules".to_string())?
            .into_parts();
    let (mut art, art_ini) = crate::app_init_helpers::load_art_ini(&assets)
        .ok_or_else(|| "load merged art".to_string())?;
    rules.merge_art_data(&art);
    rules.general.resolve_art_rates(&art_ini);
    let infantry_sequences =
        crate::rules::infantry_sequence::parse_infantry_sequence_registry(&art_ini);
    let overlay_registry = OverlayTypeRegistry::from_ini(&rules_ini, Some(&art_ini));

    let mut resolved = ResolvedTerrainGrid::build(
        &map,
        Some(&theater),
        Some(&assets),
        Some(&rules.terrain_rules),
        Some(&overlay_registry),
        false,
        rules.general.cliff_back_impassability,
    );
    let scheduler_roots =
        crate::app_init_helpers::scheduler_anim_roots(&rules, resolved.tile_animations());
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
    let house_roster =
        crate::map::houses::parse_house_roster(&map.ini, rules.color_schemes.as_slice());
    let mut overlay_grid =
        OverlayGrid::from_overlay_entries(&map.overlays, resolved.width(), resolved.height());
    let cleared_terrain_overlay_cells =
        crate::sim::terrain_spawn::clear_tiberium_source_cells_for_terrain(
            &mut overlay_grid,
            &mut resolved,
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
    let height_map = resolved.build_height_map();
    let lighting_profiles = crate::map::lighting::parse_lighting_profiles(&map.ini);

    let descriptor = ScenarioDescriptor {
        seed,
        map_name: map_file_name.to_string(),
        theater: map.header.theater.clone(),
        // A parity run stands in for a skirmish launch, which is a nonzero native mode.
        game_mode_nonzero: true,
        no_damage: false,
        // CANONICAL CELL-ARRAY FRAME, not [Map] Size= — see ScenarioDescriptor.
        map_width: resolved.width(),
        map_height: resolved.height(),
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
    let mut sim = crate::sim::runtime::construct_scenario(
        &map,
        &resolved,
        &map.header.theater,
        Some(&rules),
        Some(&rules.art_registry),
        &height_map,
        crate::map::basic::BridgeDestroyabilityMode::SkirmishOrMultiplayer {
            bridge_destruction: true,
        },
        &descriptor,
        crate::sim::scenario_bootstrap::ScenarioBootstrapRng::new(seed),
        |sim| {
            crate::sim::scenario_bootstrap::initialize_map_roster_houses(
                sim,
                &house_roster,
                Some(&rules),
            );
        },
    );
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
