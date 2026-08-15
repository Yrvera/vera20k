//! Headless retail-scenario loading for cross-engine parity runs.
//!
//! Loads a real retail map with production rules, art, theater and terrain, then
//! constructs a `Simulation` from an explicit seed — no GPU, no window, no atlases.
//! Depends on `assets`/`rules`/`map`/`sim` only; nothing here may reach into `render`,
//! `ui`, `sidebar`, `audio` or `net`.
//!
//! **Scope.** This packages the *load* half of a match. Map-placed units and structures
//! are not spawned: `app_init_helpers::spawn_entities` builds the sim and the voxel/SHP
//! atlases in one pass and needs a `GpuContext`, so the sim half cannot be called from a
//! headless binary until that function is split. Houses are likewise a launch-session
//! concern, not a load concern. A scenario loaded here therefore has real terrain, real
//! ore, real rules and a pinned RNG — and no combatants.
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
use crate::rules::ruleset::RuleSet;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::scenario_session::ScenarioDescriptor;
use crate::sim::world::Simulation;

/// A loaded scenario plus the per-tick inputs `advance_tick` needs.
pub struct HeadlessScenario {
    pub sim: Simulation,
    pub rules: RuleSet,
    pub map: MapFile,
    pub height_map: BTreeMap<(u16, u16), u8>,
    pub path_grid: PathGrid,
    pub overlay_registry: OverlayTypeRegistry,
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

    let resolved = ResolvedTerrainGrid::build(
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
    rules.art_registry = art;
    rules.bind_effect_assets(&assets, theater.extension, &map.header.theater);
    rules.bind_animation_sequences(&infantry_sequences);
    let height_map = resolved.build_height_map();
    let path_grid = PathGrid::from_resolved_terrain(&resolved);
    let overlay_grid =
        OverlayGrid::from_overlay_entries(&map.overlays, resolved.width(), resolved.height());
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

    let mut sim = Simulation::from_descriptor(&descriptor);
    sim.terrain_speed_config =
        crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig::from_general(
            rules.general.tracked_uphill,
            rules.general.tracked_downhill,
            rules.general.wheeled_uphill,
            rules.general.wheeled_downhill,
        );
    sim.playfield_bounds = Some(crate::sim::cell_rect::PlayfieldBounds {
        base: map.header.width as i32,
        off_fc: map.header.local_left as i32,
        off_100: map.header.local_top as i32,
        off_104: map.header.local_width as i32,
        off_108: map.header.local_height as i32,
    });

    sim.resolved_terrain = Some(resolved);
    sim.overlay_grid = Some(overlay_grid);
    // The production-side resource-node index is deliberately not seeded: its helper is
    // `#[cfg(test)]`-gated, and nothing reads that index without miners, which this
    // scenario has none of. Ore is still present as map overlays. Seed it here when unit
    // spawning lands.

    Ok(HeadlessScenario {
        sim,
        rules,
        map,
        height_map,
        path_grid,
        overlay_registry,
    })
}

impl HeadlessScenario {
    /// Advance one committed simulation frame with no player commands.
    pub fn tick(&mut self) {
        self.sim.advance_tick(
            &[],
            Some(&self.rules),
            &self.height_map,
            Some(&self.path_grid),
            None,
            SIM_TICK_MS,
        );
    }
}
