//! App initialization helpers — map loading, entity spawning, asset loading.
//!
//! Extracted from app.rs to keep the main orchestrator under the 400-line limit.
//! These functions run once at startup (not per-frame).
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::app_init_helpers::{
    build_entity_atlases, build_sidebar_cameo_atlas, build_tile_atlas, load_art_ini,
    load_rules_ini, log_trigger_graph_diagnostics, parse_debug_spawn_units_env, spawn_entities,
    theater_ext_for,
};
use crate::app_list_maps::{load_map_by_name_or_path_with_assets, try_load_mmx};
use crate::app_skirmish::{
    apply_skirmish_launch_session, build_overlay_atlas_from_map,
    house_color_map_for_launch_session, seed_skirmish_opening_if_needed,
};

use crate::assets::asset_manager::AssetManager;
use crate::map::actions::ActionMap;
use crate::map::basic::{BasicSection, BridgeDestroyabilityMode};
use crate::map::briefing::BriefingSection;
use crate::map::cell_tags::CellTagMap;
use crate::map::events::EventMap;
use crate::map::houses::{self, HouseColorMap, HouseRoster};
use crate::map::lighting::{self, CellLightGrid, LightingConfig, PointLight};
use crate::map::map_file::MapFile;
use crate::map::overlay::{OverlayEntry, TerrainObject};
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::preview::{PreviewSection, PreviewSourceBounds};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tags::TagMap;
use crate::map::terrain::{self, LocalBounds, TerrainGrid};
use crate::map::theater;
use crate::map::trigger_graph::TriggerGraph;
use crate::map::triggers::TriggerMap;
use crate::map::waypoints::{self, Waypoint};
use crate::render::batch::BatchRenderer;
use crate::render::bridge_atlas::BridgeAtlas;
use crate::render::bridge_railing_atlas::{BridgeRailingAtlas, BridgeRailingTileBases};
use crate::render::cursor_atlas;
use crate::render::gpu::GpuContext;
use crate::render::overlay_atlas::OverlayAtlas;
use crate::render::sidebar_cameo_atlas::SidebarCameoAtlas;
use crate::render::sidebar_chrome::SidebarChromeSet;
use crate::render::sprite_atlas::SpriteAtlas;
use crate::render::tile_atlas::TileAtlas;
use crate::render::unit_atlas::UnitAtlas;
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::{GeneralRules, RuleSet};
use crate::sim::pathfinding::PathGrid;
use crate::sim::production;
use crate::sim::trigger_runtime::TriggerRuntime;
use crate::sim::world::Simulation;
use crate::skirmish_launch::SkirmishLaunchSession;

/// All data produced by loading a map: terrain, tile atlas, entities, and camera.
pub struct MapLoadResult {
    pub basic: BasicSection,
    pub tile_atlas: Option<TileAtlas>,
    pub terrain_grid: Option<TerrainGrid>,
    pub resolved_terrain: Option<ResolvedTerrainGrid>,
    pub simulation: Option<Simulation>,
    pub unit_atlas: Option<UnitAtlas>,
    /// Palette + per-house RGB ramp GPU resources for the voxel sprite shader.
    /// None when no theater palette is available (rare).
    pub palette_set: Option<crate::render::palette_textures::PaletteSet>,
    pub sprite_atlas: Option<SpriteAtlas>,
    pub overlay_atlas: Option<OverlayAtlas>,
    pub bridge_atlas: Option<BridgeAtlas>,
    pub bridge_railing_atlas: Option<BridgeRailingAtlas>,
    pub sidebar_cameo_atlas: Option<SidebarCameoAtlas>,
    pub sidebar_chrome: Option<SidebarChromeSet>,
    pub(crate) software_cursor: Option<crate::app_render::SoftwareCursor>,
    /// Overlay entries for per-frame instance generation.
    pub overlays: Vec<OverlayEntry>,
    /// Terrain objects for per-frame instance generation.
    pub terrain_objects: Vec<TerrainObject>,
    pub waypoints: HashMap<u32, Waypoint>,
    pub cell_tags: CellTagMap,
    pub tags: TagMap,
    pub triggers: TriggerMap,
    pub events: EventMap,
    pub actions: ActionMap,
    pub trigger_graph: TriggerGraph,
    pub trigger_runtime: TriggerRuntime,
    /// Overlay ID → type name mapping (from rules.ini [OverlayTypes]).
    pub overlay_names: BTreeMap<u8, String>,
    /// Precomputed average pixel color for each tiberium overlay (id, frame) pair,
    /// extracted from SHP frames for minimap radar display.
    pub tiberium_radar_colors: HashMap<(u8, u8), [u8; 3]>,
    /// Overlay type registry — kept so wall placement can look up overlay_id by name.
    pub overlay_registry: OverlayTypeRegistry,
    /// Owner name → house color index mapping (from map [Houses] sections).
    pub house_color_map: HouseColorMap,
    pub house_roster: HouseRoster,
    /// Cell (rx, ry) → terrain elevation z for overlay/entity height lookup.
    pub height_map: BTreeMap<(u16, u16), u8>,
    /// Cell (rx, ry) → bridge deck elevation z. Only bridge cells present.
    pub bridge_height_map: BTreeMap<(u16, u16), u8>,
    pub tactical_bridge_inverse_map: BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>,
    /// Pre-built pathfinding grid with water/cliff/building walkability.
    pub path_grid: Option<PathGrid>,
    /// Parsed rules.ini data — kept for combat system weapon/warhead lookups.
    pub rules: Option<RuleSet>,
    /// Art.ini registry — kept for building animation overlay lookups at render time.
    pub art_registry: Option<ArtRegistry>,
    /// Parsed infantry animation sequence definitions from art.ini [*Sequence] sections.
    pub infantry_sequences: crate::rules::infantry_sequence::InfantrySequenceRegistry,
    /// CSF string table — localized display names loaded from language MIX.
    pub csf: Option<crate::assets::csf_file::CsfFile>,
    /// Parsed GAME.FNT bitmap font for authentic sidebar text rendering.
    pub fnt_file: Option<crate::assets::fnt_file::FntFile>,
    /// Per-cell RGB tint from map [Lighting] section.
    pub lighting_grid: CellLightGrid,
    /// Parsed [Lighting] config used for transient lighting rebuilds.
    pub map_lighting_config: LightingConfig,
    /// Current map theater name (e.g., "DESERT", "TEMPERATE").
    pub theater_name: String,
    /// Current theater extension (e.g., "des", "tem").
    pub theater_ext: String,
    /// Preferred initial local owner when the loader seeded a sandbox opening.
    pub initial_local_owner: Option<String>,
    /// Keep full map visibility for the empty-map sandbox opening.
    pub sandbox_full_visibility: bool,
    /// True when MCV seeding was deferred for spawn-pick phase.
    /// The map has 2+ multiplayer start waypoints and the player should pick one.
    pub spawn_pick_pending: bool,
    pub camera_x: f32,
    pub camera_y: f32,
    /// Asset manager — kept alive for music/audio lookups after map load.
    pub asset_manager: Option<AssetManager>,
}

pub(crate) struct MapLoadInitial {
    asset_manager: AssetManager,
    map_data: MapFile,
}

pub(crate) fn load_csf(asset_manager: &AssetManager) -> Option<crate::assets::csf_file::CsfFile> {
    for name in [
        "ra2md.csf",
        "ra2.csf",
        "stringtablemd.csf",
        "stringtable.csf",
    ] {
        let Some(bytes) = asset_manager.get_ref(name) else {
            continue;
        };
        match crate::assets::csf_file::CsfFile::from_bytes(bytes) {
            Ok(csf) => {
                log::info!("Loaded CSF string table: {name}");
                return Some(csf);
            }
            Err(err) => {
                log::warn!("Failed to parse CSF {name}: {err:#}");
            }
        }
    }
    None
}

/// Rebuild transient app lighting from base map light plus the current live entities.
pub(crate) fn rebuild_lighting_grid_from_sim(
    resolved_terrain: &ResolvedTerrainGrid,
    lighting_config: &LightingConfig,
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
) -> CellLightGrid {
    let mut lighting_grid = lighting::build_cell_light_grid_from_heights(
        resolved_terrain
            .iter()
            .map(|cell| ((cell.rx, cell.ry), cell.level)),
        lighting_config,
    );
    let point_lights = collect_live_building_lights(simulation, rules);
    lighting::accumulate_point_lights(&mut lighting_grid, &point_lights);
    lighting_grid
}

fn collect_live_building_lights(
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
) -> Vec<PointLight> {
    let (Some(sim), Some(rules)) = (simulation, rules) else {
        return Vec::new();
    };
    sim.entities()
        .values()
        .filter(|entity| {
            entity.category == crate::map::entities::EntityCategory::Structure
                && !entity.dying
                && entity.health.current > 0
        })
        .filter_map(|entity| {
            let type_id = sim.interner.resolve(entity.type_ref);
            let obj = rules.object(type_id)?;
            lighting::point_light_from_object(
                entity.position.rx,
                entity.position.ry,
                obj.light_visibility,
                obj.light_intensity,
                [
                    obj.light_red_tint,
                    obj.light_green_tint,
                    obj.light_blue_tint,
                ],
            )
        })
        .collect()
}

fn clear_tiberium_source_cells_for_spawning_terrain(
    sim: &mut Simulation,
    resolved_terrain: &mut ResolvedTerrainGrid,
    terrain_objects: &[TerrainObject],
    rules: &RuleSet,
    overlay_registry: &OverlayTypeRegistry,
) -> BTreeSet<(u16, u16)> {
    let source_cells: BTreeSet<(u16, u16)> = terrain_objects
        .iter()
        .filter(|obj| {
            rules
                .terrain_object_type_case_insensitive(&obj.name)
                .is_some_and(|terrain_type| terrain_type.spawns_tiberium)
        })
        .map(|obj| (obj.rx, obj.ry))
        .collect();
    if source_cells.is_empty() {
        return BTreeSet::new();
    }

    let mut cleared_cells = BTreeSet::new();
    for &cell in &source_cells {
        if sim.production.resource_nodes.remove(&cell).is_some() {
            cleared_cells.insert(cell);
        }
    }

    let mut overlay_cleared = Vec::new();
    if let Some(grid) = sim.overlay_grid.as_mut() {
        for &(rx, ry) in &source_cells {
            let cell = *grid.cell(rx, ry);
            let Some(overlay_id) = cell.overlay_id else {
                continue;
            };
            if !overlay_registry
                .flags(overlay_id)
                .is_some_and(|flags| flags.tiberium)
            {
                continue;
            }
            grid.clear_overlay(rx, ry);
            overlay_cleared.push((rx, ry));
            cleared_cells.insert((rx, ry));
        }
    }

    if let Some(grid) = sim.overlay_grid.as_ref() {
        for &(rx, ry) in &overlay_cleared {
            crate::sim::overlay_grid::recalc_overlay_passability(
                grid,
                resolved_terrain,
                overlay_registry,
                rx,
                ry,
            );
        }
        if let Some(sim_terrain) = sim.resolved_terrain.as_mut() {
            for &(rx, ry) in &overlay_cleared {
                crate::sim::overlay_grid::recalc_overlay_passability(
                    grid,
                    sim_terrain,
                    overlay_registry,
                    rx,
                    ry,
                );
            }
        }
    }

    cleared_cells
}

/// Lightweight metadata used by the main-menu map selector.
#[derive(Debug, Clone)]
pub struct MapMenuEntry {
    /// Actual file name/path token used to load the map later.
    pub file_name: String,
    /// Human-facing label derived from `[Basic] Name` when available.
    pub display_name: String,
    /// Optional author text from `[Basic]`.
    pub author: Option<String>,
    /// Ordered mission briefing lines from `[Briefing]`.
    pub briefing: BriefingSection,
    /// Lightweight preview metadata from `[Preview]` / `[PreviewPack]`.
    pub preview: PreviewSection,
    /// Multiplayer start waypoints 0..=7, sorted by waypoint index.
    pub multiplayer_start_waypoints: Vec<Waypoint>,
    /// Verified source bounds for projecting starts onto the preview surface.
    pub preview_source_bounds: Option<PreviewSourceBounds>,
}

pub(crate) fn load_map_initial_with_assets(
    ra2_dir: PathBuf,
    asset_manager: AssetManager,
    requested_map: Option<&str>,
    progress: &mut dyn crate::app_loading::LoadingProgressSink,
) -> Result<MapLoadInitial> {
    // Check RA2_QUICKPLAY env var: if it names a .map/.mpr file, load that directly.
    // UI-selected map name/path (requested_map) takes precedence.
    // Default: try testmap1.map in the project directory first, then fall back to .mmx files.
    let quickplay_map: Option<String> = std::env::var("RA2_QUICKPLAY")
        .ok()
        .filter(|v| v.ends_with(".map") || v.ends_with(".mpr") || v.ends_with(".mmx"));

    let map_data: MapFile =
        if let Some(map_name) = requested_map.filter(|m| !m.eq_ignore_ascii_case("auto")) {
            load_map_by_name_or_path_with_assets(&ra2_dir, map_name, &asset_manager)?
        } else if let Some(ref map_name) = quickplay_map {
            load_map_by_name_or_path_with_assets(&ra2_dir, map_name, &asset_manager)?
        } else if Path::new("testmap1.map").exists() {
            let bytes: Vec<u8> = std::fs::read("testmap1.map")?;
            log::info!("Loading default map: testmap1.map");
            MapFile::from_bytes(&bytes)?
        } else {
            let mmx_names: &[&str] = &[
                "Dustbowl.mmx",
                "Barrel.mmx",
                "GoldSt.mmx",
                "Kaliforn.mmx",
                "Hills.mmx",
                "Grinder.mmx",
                "Break.mmx",
                "Potomac.mmx",
                "Arena.mmx",
                "Lostlake.mmx",
                "Oceansid.mmx",
                "Pacific.mmx",
            ];
            try_load_mmx(&ra2_dir, mmx_names)?
        };
    log::info!(
        "Map loaded: title={:?}, theater={}, {}x{}, {} cells, {} entities",
        map_data.basic.name,
        map_data.header.theater,
        map_data.header.width,
        map_data.header.height,
        map_data.cells.len(),
        map_data.entities.len()
    );
    log_trigger_graph_diagnostics(&map_data);

    // First post-render-handoff milestone: the selected map has been opened and
    // parsed. (gamemd emits 8 at theater-init entry; in our pipeline the map is
    // parsed first, so 8 marks the end of this initial phase.)
    progress.milestone(8);

    Ok(MapLoadInitial {
        asset_manager,
        map_data,
    })
}

pub(crate) fn load_map_from_initial(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    initial: MapLoadInitial,
    skirmish_launch_session: Option<&SkirmishLaunchSession>,
    skirmish_settings: &crate::ui::main_menu::SkirmishSettings,
    mut vxl_compute: Option<&mut crate::render::vxl_compute::VxlComputeRenderer>,
    progress: &mut dyn crate::app_loading::LoadingProgressSink,
) -> Result<MapLoadResult> {
    let MapLoadInitial {
        mut asset_manager,
        map_data,
    } = initial;

    // Load theater INI for tileset lookup, palette, and LAT configuration.
    // Also loads theater-specific MIX archives (e.g., isotemmd.mix) at highest priority.
    let theater_result: Option<theater::TheaterData> =
        theater::load_theater(&mut asset_manager, &map_data.header.theater);
    let theater_ext: &'static str = match &theater_result {
        Some(td) => td.extension,
        None => theater_ext_for(&map_data.header.theater),
    };
    // Theater archives + palettes loaded (gamemd Init_Theater milestones).
    progress.milestone(12);
    progress.milestone(30);

    let parse_bool_env = |key: &str| -> Option<bool> {
        std::env::var(key).ok().map(|v| {
            let n = v.trim().to_ascii_lowercase();
            n == "1" || n == "true" || n == "yes" || n == "on"
        })
    };
    // Default for runtime maps: keep original authored transitions.
    // Set RA2_ENABLE_LAT=1 to opt in to auto-LAT generation.
    // RA2_DISABLE_LAT=1 always forces LAT off.
    let enable_lat: bool = parse_bool_env("RA2_ENABLE_LAT").unwrap_or(false);
    let force_disable_lat: bool = parse_bool_env("RA2_DISABLE_LAT").unwrap_or(false);
    let lat_enabled: bool = !force_disable_lat && enable_lat;
    if force_disable_lat {
        log::warn!("LAT disabled by RA2_DISABLE_LAT");
    } else if !lat_enabled {
        log::info!("LAT disabled by default (set RA2_ENABLE_LAT=1 to enable)");
    }

    // Load rules.ini and art.ini before building resolved terrain so overlay
    // semantics and art-foundation data are available to the pipeline.
    let mut rules: Option<RuleSet> = load_rules_ini(&asset_manager);
    let art_result: Option<(ArtRegistry, IniFile)> = load_art_ini(&asset_manager);
    let (mut art, art_ini): (Option<ArtRegistry>, Option<IniFile>) = match art_result {
        Some((reg, ini)) => (Some(reg), Some(ini)),
        None => (None, None),
    };
    if let (Some(r), Some(a)) = (rules.as_mut(), art.as_mut()) {
        r.merge_art_data(a);
        // Eagerly populate per-anim SHP frame dimensions so the smudge
        // dispatcher can size-filter without falling back to the (30, 30)
        // default that always loses the threshold check.
        let (populated, fallback) =
            a.populate_anim_frame_dims(&asset_manager, theater_ext, &map_data.header.theater);
        log::info!(
            "Anim frame dims: {} populated, {} fallback (defaults to 30x30)",
            populated,
            fallback,
        );
        // Retain the art registry on RuleSet so dispatchers (e.g. smudge
        // spawning) can read per-anim spawn flags via &RuleSet alone.
        // Cloned because downstream consumers in this function still read
        // through the `art` Option (lighting, sidebar, sim spawn, etc.).
        r.art_registry = a.clone();
    }
    // Resolve warp animation rates from art.ini sections (e.g., [WARPOUT] Rate=120).
    if let (Some(r), Some(art_ini_file)) = (&mut rules, &art_ini) {
        r.general.resolve_art_rates(art_ini_file);
    }
    // Parse infantry animation sequence definitions from art.ini [*Sequence] sections.
    let infantry_sequences = if let Some(ref art_ini_file) = art_ini {
        crate::rules::infantry_sequence::parse_infantry_sequence_registry(art_ini_file)
    } else {
        HashMap::new()
    };
    // Rules + art parsed, merged, and processed (gamemd command-bar/CD/rules
    // milestones).
    progress.milestone(31);
    progress.milestone(35);
    progress.milestone(45);
    let csf: Option<crate::assets::csf_file::CsfFile> = load_csf(&asset_manager);
    let rules_ini: IniFile = asset_manager
        .get_with_source("rulesmd.ini")
        .or_else(|| asset_manager.get_with_source("rules.ini"))
        .and_then(|(d, source)| {
            log::info!("Raw rules INI from: {}", source);
            IniFile::from_bytes(&d).ok()
        })
        .unwrap_or_else(|| IniFile::from_str(""));
    let overlay_registry: OverlayTypeRegistry =
        OverlayTypeRegistry::from_ini(&rules_ini, art_ini.as_ref());

    // Compute playable area bounds from LocalSize (border filler hidden by shroud).
    let local_bounds: Option<LocalBounds> = Some(LocalBounds::from_header(&map_data.header));

    let cliff_back = rules
        .as_ref()
        .map(|r| r.general.cliff_back_impassability)
        .unwrap_or(2);
    let mut resolved_terrain = ResolvedTerrainGrid::build(
        &map_data,
        theater_result.as_ref(),
        Some(&asset_manager),
        rules.as_ref().map(|r| &r.terrain_rules),
        Some(&overlay_registry),
        lat_enabled,
        cliff_back,
    );
    let anchor_variant_table = theater_result
        .as_ref()
        .and_then(crate::map::theater::BridgeAnchorVariantTable::from_theater);
    let grid: TerrainGrid = terrain::build_terrain_grid_from_resolved(
        &resolved_terrain,
        local_bounds,
        anchor_variant_table,
    );
    // Side/house mix + resolved terrain grid ready.
    progress.milestone(50);
    progress.milestone(55);

    // Build per-cell lighting from map [Lighting] section.
    let lighting_config = lighting::parse_lighting(&map_data.ini);
    // [Basic]/lighting read complete (gamemd Read_INI_Basic milestones).
    progress.milestone(58);
    progress.milestone(60);

    let tile_atlas: Option<TileAtlas> = match &theater_result {
        Some(td) => build_tile_atlas(
            &asset_manager,
            &td.lookup,
            &td.iso_palette,
            td.extension,
            &grid,
            gpu,
            batch,
        ),
        None => None,
    };

    // Theater tileset / map-section surfaces built (gamemd map-section milestones).
    progress.milestone(63);
    progress.milestone(65);

    let art_fallback: ArtRegistry = ArtRegistry::empty();

    // Parse house color assignments from map INI ([Houses] + per-house Color=).
    // Color=<name> resolves against the rules `[Colors]` list (entry index).
    let color_schemes: &[crate::rules::color_scheme::ColorSchemeEntry] = rules
        .as_ref()
        .map(|r| r.color_schemes.as_slice())
        .unwrap_or(&[]);
    let house_roster: HouseRoster = houses::parse_house_roster(&map_data.ini, color_schemes);
    let house_color_map: HouseColorMap = skirmish_launch_session.map_or_else(
        || house_roster.color_map(),
        |session| house_color_map_for_launch_session(session, &house_roster),
    );
    progress.milestone(67);

    // Build height lookup for entity/overlay elevation (shared between subsystems).
    let height_map: BTreeMap<(u16, u16), u8> = resolved_terrain.build_height_map();
    let bridge_height_map: BTreeMap<(u16, u16), u8> = resolved_terrain.build_bridge_height_map();
    let tactical_bridge_inverse_map: BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell> =
        resolved_terrain.build_tactical_bridge_inverse_map();

    let bridge_railing_tile_bases = theater_result
        .as_ref()
        .and_then(|td| td.bridge_railing_slope_starts())
        .map(
            |(slope_set_pieces_start, slope_set_pieces2_start)| BridgeRailingTileBases {
                slope_set_pieces_start,
                slope_set_pieces2_start,
            },
        );

    // Extract theater palettes for entity/overlay rendering.
    // Move palettes out of TheaterData (no longer needed after tile atlas is built).
    let (unit_palette, overlay_iso_palette, overlay_tiberium_palette) = match theater_result {
        Some(td) => (
            Some(td.unit_palette),
            Some(td.iso_palette),
            Some(td.tiberium_palette),
        ),
        None => (None, None, None),
    };
    // Map/overlay prelude ready (gamemd IsoMapPack/overlay milestones).
    progress.milestone(68);
    progress.milestone(69);
    progress.milestone(70);

    let bridge_destroyability_mode =
        skirmish_launch_session.map_or(BridgeDestroyabilityMode::CampaignOrEditor, |session| {
            BridgeDestroyabilityMode::SkirmishOrMultiplayer {
                bridge_destruction: session.options.bridges_destroyable,
            }
        });

    // One negotiated per-match seed, fixed before the sim exists and before
    // any setup-phase draw (random country/color resolution included). Logged
    // as the repro handle until replay save/load UI exists.
    let scenario_descriptor = crate::sim::scenario_session::ScenarioDescriptor {
        seed: crate::app_init_helpers::generate_match_seed(),
    };
    log::info!("Match seed: 0x{:08X}", scenario_descriptor.seed);

    let (simulation, mut unit_atlas, mut sprite_atlas, mut palette_set) = spawn_entities(
        &map_data,
        &resolved_terrain,
        &asset_manager,
        gpu,
        batch,
        theater_ext,
        &map_data.header.theater,
        rules.as_ref(),
        art.as_ref(),
        &house_color_map,
        &height_map,
        unit_palette.as_ref(),
        &infantry_sequences,
        vxl_compute.as_deref_mut(),
        bridge_destroyability_mode,
        &scenario_descriptor,
    );
    // Terrain/tiberium + units/infantry/buildings created from the map
    // (gamemd terrain/units/objects/buildings milestones).
    progress.milestone(72);
    progress.milestone(74);
    progress.milestone(76);
    progress.milestone(78);
    let mut simulation = simulation;
    if let Some(sim) = &mut simulation {
        if skirmish_launch_session.is_none() {
            sim.house_alliances = house_roster.alliance_map();
            // Populate per-player HouseState from the map's house roster.
            for house in &house_roster.houses {
                let side_idx = crate::sim::house_state::side_index_from_name(house.side.as_deref());
                let is_human = house.player_control == Some(true);
                let name_id = sim.interner.intern(&house.name);
                let country_id = house.country.as_deref().map(|c| sim.interner.intern(c));
                sim.houses.insert(
                    name_id,
                    crate::sim::house_state::HouseState::new(
                        name_id,
                        side_idx,
                        country_id,
                        is_human,
                        sim.game_options.starting_credits,
                        sim.game_options.tech_level,
                    ),
                );
            }
        }
    }
    // Pre-intern all rule type IDs so that build_option_for_owner can resolve
    // InternedIds for types that haven't been spawned yet (e.g. GAPOWR).
    // Without this, sidebar cameo lookups fail because unspawned types get
    // InternedId(0) and resolve to the wrong string.
    if let (Some(sim), Some(ruleset)) = (&mut simulation, rules.as_ref()) {
        ruleset.intern_all_ids(&mut sim.interner);
        // One-hop type resolution: build the handle table now that every type id
        // is interned. Mirrors the bridge-warhead pre-resolve below.
        sim.resolve_type_handles(ruleset);
    }
    // Pre-resolve `[CombatDamage] IonCannonWarhead=` and `C4Warhead=` against
    // the simulation interner. Combat reads these via accessors during the
    // bridge-damage path; resolution must happen before any combat tick.
    if let (Some(sim), Some(ruleset)) = (&mut simulation, rules.as_mut()) {
        ruleset.resolve_bridge_warheads(&mut sim.interner);
    }

    // SpawnPick phase is disabled — MCV always spawns directly at the chosen position.
    let spawn_pick_pending: bool = false;

    let mut initial_local_owner: Option<String> = None;
    if !spawn_pick_pending {
        if let (Some(sim), Some(ruleset)) = (&mut simulation, rules.as_ref()) {
            let should_rebuild_entity_atlases = if let Some(session) = skirmish_launch_session {
                let result = apply_skirmish_launch_session(
                    sim,
                    &map_data,
                    &house_roster,
                    ruleset,
                    &height_map,
                    &resolved_terrain,
                    session,
                );
                initial_local_owner = result.local_owner;
                result.spawned_mcvs > 0
            } else {
                initial_local_owner = seed_skirmish_opening_if_needed(
                    sim,
                    &map_data,
                    &house_roster,
                    ruleset,
                    &height_map,
                    skirmish_settings,
                );
                // Set up AI players: all playable houses except the local (first) player.
                if let Some(ref local_owner) = initial_local_owner {
                    setup_ai_players(sim, &house_roster, local_owner);
                }
                initial_local_owner.is_some()
            };

            if should_rebuild_entity_atlases {
                let (new_unit_atlas, new_sprite_atlas, new_palette_set) = build_entity_atlases(
                    sim,
                    &asset_manager,
                    gpu,
                    batch,
                    theater_ext,
                    &map_data.header.theater,
                    rules.as_ref(),
                    art.as_ref(),
                    &house_color_map,
                    unit_palette.as_ref(),
                    &infantry_sequences,
                    vxl_compute.as_deref_mut(),
                );
                unit_atlas = new_unit_atlas;
                sprite_atlas = new_sprite_atlas;
                palette_set = new_palette_set;
            }
        }
    }

    // Copy world-effect SHP frame counts from the sprite atlas into the simulation
    // so sim systems (chrono-teleport) can spawn effects with the correct frame count.
    if let (Some(sim), Some(atlas)) = (&mut simulation, &sprite_atlas) {
        for (name, &count) in &atlas.active_anim_frame_counts {
            let name_id = sim.interner.intern(name);
            sim.effect_frame_counts.insert(name_id, count);
        }
    }

    // Optional debug spawn list for render testing.
    // Examples:
    //   RA2_DEBUG_SPAWN_UNITS=1                  -> default list (HTNK,MTNK,E1)
    //   RA2_DEBUG_SPAWN_UNITS=HTNK,MTNK,APOC
    if let (Some(sim), Some(ruleset), Some(debug_units)) = (
        &mut simulation,
        rules.as_ref(),
        parse_debug_spawn_units_env(),
    ) {
        let owner: String = house_color_map
            .keys()
            .find(|h| {
                let up = h.to_ascii_uppercase();
                up != "NEUTRAL" && up != "SPECIAL"
            })
            .cloned()
            .unwrap_or_else(|| "Americans".to_string());

        let (anchor_rx, anchor_ry): (u16, u16) = map_data
            .entities
            .iter()
            .find(|e| {
                e.category == crate::map::entities::EntityCategory::Structure
                    && e.owner.eq_ignore_ascii_case(&owner)
            })
            .map(|e| (e.cell_x, e.cell_y))
            .or_else(|| {
                waypoints::first_multiplayer_start(&map_data.waypoints).map(|wp| (wp.rx, wp.ry))
            })
            .or_else(|| map_data.cells.first().map(|c| (c.rx, c.ry)))
            .unwrap_or((50, 50));

        let offsets: &[(i32, i32)] = &[
            (2, 2),
            (4, 2),
            (6, 2),
            (2, 4),
            (4, 4),
            (6, 4),
            (2, 6),
            (4, 6),
        ];
        let mut spawned: u32 = 0;
        for (i, type_id) in debug_units.iter().enumerate() {
            let (ox, oy) = offsets[i % offsets.len()];
            let rx = (anchor_rx as i32 + ox).max(0) as u16;
            let ry = (anchor_ry as i32 + oy).max(0) as u16;
            if sim
                .spawn_object(type_id, &owner, rx, ry, 64, ruleset, &height_map)
                .is_some()
            {
                spawned += 1;
            } else {
                log::warn!("Debug spawn failed for '{}'", type_id);
            }
        }
        if spawned > 0 {
            log::info!(
                "Debug-spawned {} unit(s) for owner={} near ({},{}): {:?}",
                spawned,
                owner,
                anchor_rx,
                anchor_ry,
                debug_units
            );
        }
    }

    let (
        overlay_atlas,
        bridge_atlas,
        bridge_railing_atlas,
        overlay_names,
        mut overlays_connected,
        tiberium_radar_colors,
    ) = build_overlay_atlas_from_map(
        &map_data,
        &asset_manager,
        gpu,
        batch,
        theater_ext,
        &rules_ini,
        art.as_ref().unwrap_or(&art_fallback),
        overlay_iso_palette.as_ref(),
        unit_palette.as_ref(),
        overlay_tiberium_palette.as_ref(),
        rules.as_ref().map(|r| &r.smudge_types),
        bridge_railing_tile_bases,
    );

    let mut terrain_frame_counts = BTreeMap::new();
    if let Some(atlas) = overlay_atlas.as_ref() {
        for obj in &map_data.terrain_objects {
            if let Some(frame_count) = atlas.terrain_anim_frame_count(&obj.name) {
                terrain_frame_counts.insert(obj.name.clone(), u16::from(frame_count));
                terrain_frame_counts.insert(obj.name.to_ascii_uppercase(), u16::from(frame_count));
            }
        }
    }

    if let Some(sim) = &mut simulation {
        let seeded =
            production::seed_resource_nodes_from_overlays(sim, &map_data.overlays, &overlay_names);
        if seeded > 0 {
            log::info!("Seeded {} resource node cells for economy loop", seeded);
        }
        // Seed TIBTRE-style ore-spawning terrain objects. Skip gracefully if
        // rules failed to load (matches the ore_growth_config pattern below).
        if let Some(rules_for_terrain) = rules.as_ref() {
            let seeded_terrain = crate::sim::terrain_spawn::seed_terrain_spawners(
                sim,
                &map_data.terrain_objects,
                rules_for_terrain,
                &overlay_names,
                &terrain_frame_counts,
                map_data.header.theater.eq_ignore_ascii_case("SNOW"),
            );
            if seeded_terrain > 0 {
                log::info!(
                    "Seeded {} ore-spawning terrain objects (TIBTRE)",
                    seeded_terrain,
                );
            }
        } else {
            log::warn!("No rules loaded — skipping terrain spawner seeding");
        }
        // Seed mutable overlay grid from map overlay data.
        if let Some(rt) = &sim.resolved_terrain {
            let grid_width = rt.width();
            let grid_height = rt.height();
            sim.overlay_grid = Some(crate::sim::overlay_grid::OverlayGrid::from_overlay_entries(
                &map_data.overlays,
                grid_width,
                grid_height,
            ));
            log::info!(
                "Overlay grid initialized: {}x{}, {} entries",
                grid_width,
                grid_height,
                map_data.overlays.len(),
            );
        }
        if let Some(rules_for_terrain) = rules.as_ref() {
            let cleared_cells = clear_tiberium_source_cells_for_spawning_terrain(
                sim,
                &mut resolved_terrain,
                &map_data.terrain_objects,
                rules_for_terrain,
                &overlay_registry,
            );
            if !cleared_cells.is_empty() {
                overlays_connected.retain(|entry| {
                    !cleared_cells.contains(&(entry.rx, entry.ry))
                        || !overlay_registry
                            .flags(entry.overlay_id)
                            .is_some_and(|flags| flags.tiberium)
                });
                log::info!(
                    "Cleared {} same-cell tiberium overlay/resource source cell(s) for spawning terrain",
                    cleared_cells.len(),
                );
            }
        }
        // Seed smudge grid from map [Smudge] entries. Requires terrain +
        // overlay grids built above so placement gates (slope, overlay,
        // accepts_smudge) can reject invalid map entries at load.
        if let (Some(rt), Some(overlay), Some(rules_for_smudge)) = (
            sim.resolved_terrain.as_ref(),
            sim.overlay_grid.as_ref(),
            rules.as_ref(),
        ) {
            let grid_width = rt.width();
            let grid_height = rt.height();
            sim.smudge_grid = Some(crate::sim::smudge_grid::SmudgeGrid::from_map_entries(
                &map_data.smudges,
                &rules_for_smudge.smudge_types,
                rt,
                overlay,
                grid_width,
                grid_height,
            ));
            log::info!(
                "Smudge grid initialized: {}x{}, {} entries",
                grid_width,
                grid_height,
                map_data.smudges.len(),
            );
        }
        // Initialize ore growth/spread config from merged INI sources.
        let general_default = GeneralRules::default();
        let general_rules = rules.as_ref().map_or(&general_default, |r| &r.general);
        let ore_config = crate::sim::ore_growth::OreGrowthConfig::from_ini(
            general_rules,
            &map_data.basic,
            &map_data.special_flags,
        );
        let map_w = map_data.header.width as u16;
        let map_h = map_data.header.height as u16;
        sim.production.ore_growth_config = ore_config;
        sim.production.ore_growth_state = crate::sim::ore_growth::OreGrowthState::new(map_w, map_h);
        if let (Some(rules_for_tiberium), Some(overlay_grid)) =
            (rules.as_ref(), sim.overlay_grid.as_ref())
        {
            let source_object_cells: BTreeSet<(u16, u16)> = sim
                .production
                .terrain_object_cells
                .keys()
                .copied()
                .collect();
            let stats = sim
                .production
                .ore_growth_state
                .rebuild_native_tiberium_queues_from_overlays(
                    overlay_grid,
                    &overlay_registry,
                    &rules_for_tiberium.tiberium_types,
                    sim.resolved_terrain.as_ref(),
                    &source_object_cells,
                    map_data.basic.tiberium_growth_enabled.unwrap_or(true),
                    general_rules.tiberium_spreads
                        && map_data.special_flags.tiberium_spreads.unwrap_or(true),
                    sim.binary_frame,
                );
            log::info!(
                "Native tiberium queues rebuilt: {} growth entries, {} spread entries",
                stats.growth_entries,
                stats.spread_entries,
            );
        } else {
            sim.production
                .ore_growth_state
                .reset_native_tiberium_classes(0, sim.binary_frame);
        }
    }

    // Build PathGrid with terrain walkability derived from resolved terrain:
    // terrain/object/overlay blocking plus dynamic structure occupancy.
    let path_grid: Option<PathGrid> = {
        let mut grid: PathGrid = PathGrid::from_resolved_terrain(&resolved_terrain);

        // Block building footprints using foundation sizes from rules.ini.
        for ent in &map_data.entities {
            if ent.category == crate::map::entities::EntityCategory::Structure {
                let obj = rules.as_ref().and_then(|r| r.object(&ent.type_id));
                let foundation: &str = obj.map(|o| o.foundation.as_str()).unwrap_or("1x1");
                let has_bib: bool = obj.map(|o| o.bib).unwrap_or(false);
                grid.block_building_movement_cells(ent.cell_x, ent.cell_y, foundation, has_bib);
            }
        }

        // Block cells occupied by terrain objects (trees, rocks, light posts, etc.).
        for obj in &map_data.terrain_objects {
            grid.set_blocked(obj.rx, obj.ry, true);
        }

        // Build per-SpeedType terrain cost grids for cost-aware pathfinding.
        // Units look up their SpeedType to pick the right grid at move time.
        {
            use crate::rules::locomotor_type::SpeedType;
            use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
            let speed_types = [
                SpeedType::Foot,
                SpeedType::Track,
                SpeedType::Wheel,
                SpeedType::Float,
                SpeedType::Amphibious,
                SpeedType::Hover,
                SpeedType::FloatBeach,
            ];
            let mut terrain_costs: BTreeMap<SpeedType, TerrainCostGrid> = BTreeMap::new();
            for &st in &speed_types {
                let cost_grid = TerrainCostGrid::from_resolved_terrain(&resolved_terrain, st);
                terrain_costs.insert(st, cost_grid);
            }
            if let Some(sim) = &mut simulation {
                sim.terrain_costs = terrain_costs;
            }
            // Winged units ignore terrain — no need for a Winged cost grid
            // (find_path_with_costs falls back to find_path when no grid found).
            log::info!(
                "Built {} terrain cost grids for cost-aware pathfinding",
                speed_types.len()
            );
        }

        Some(grid)
    };

    // Prefer the first multiplayer start waypoint as the initial anchor when
    // present. Otherwise, center on the playable area / terrain grid.
    let sw: f32 = gpu.config.width as f32;
    let sh: f32 = gpu.config.height as f32;
    let (camera_x, camera_y): (f32, f32) =
        if let Some(start_wp) = waypoints::first_multiplayer_start(&map_data.waypoints) {
            let wp_z = height_map
                .get(&(start_wp.rx, start_wp.ry))
                .copied()
                .unwrap_or(0);
            let (sx, sy) = terrain::iso_to_screen(start_wp.rx, start_wp.ry, wp_z);
            (sx - sw / 2.0, sy - sh / 2.0)
        } else {
            let (area_x, area_y, area_w, area_h) = match local_bounds {
                Some(b) => (b.pixel_x, b.pixel_y, b.pixel_w, b.pixel_h),
                None => (
                    grid.origin_x,
                    grid.origin_y,
                    grid.world_width,
                    grid.world_height,
                ),
            };
            (area_x + (area_w - sw) / 2.0, area_y + (area_h - sh) / 2.0)
        };
    // Load cameo MIX archives so that *ICON.SHP files are findable.
    // These nested MIXes live inside local.mix/localmd.mix and aren't
    // auto-extracted by the two-level brute-force pass.
    for cameo_mix in ["cameomd.mix", "cameo.mix"] {
        match asset_manager.load_nested(cameo_mix) {
            Ok(()) => log::info!("Loaded nested {cameo_mix} for sidebar cameo icons"),
            Err(_) => log::debug!("{cameo_mix} not found (optional)"),
        }
    }
    let sidebar_cameo_atlas =
        build_sidebar_cameo_atlas(gpu, batch, &asset_manager, rules.as_ref(), art.as_ref());
    let sidebar_chrome =
        crate::render::sidebar_chrome::build_sidebar_chrome_set(gpu, batch, &asset_manager);
    let fnt_file = asset_manager.get_ref("GAME.FNT").and_then(|data| {
        crate::assets::fnt_file::FntFile::from_bytes(data)
            .map_err(|e| log::warn!("Failed to parse GAME.FNT: {e}"))
            .ok()
    });
    let software_cursor = cursor_atlas::build_software_cursor(gpu, batch, &asset_manager);
    if software_cursor.is_some() {
        log::info!("Software cursor loaded from mouse.sha — OS cursor will be hidden");
    } else {
        log::warn!("Software cursor NOT loaded (mouse.sha missing?) — using OS cursor");
    }
    let trigger_runtime = TriggerRuntime::from_map(&map_data.triggers, &map_data.local_variables);
    let lighting_grid = rebuild_lighting_grid_from_sim(
        &resolved_terrain,
        &lighting_config,
        simulation.as_ref(),
        rules.as_ref(),
    );
    // Final post-map-init milestones (gamemd cell-attributes, beacon-art [90
    // coalesced here, no distinct Rust step], post-map-init, tactical cleanup,
    // final pre-render refresh). 100 is emitted by the pump at Finished.
    progress.milestone(82);
    progress.milestone(86);
    progress.milestone(93);
    progress.milestone(96);
    progress.milestone(98);

    // Move fields out of map_data (last use) instead of cloning.
    let theater_name = map_data.header.theater;
    Ok(MapLoadResult {
        basic: map_data.basic,
        tile_atlas,
        terrain_grid: Some(grid),
        resolved_terrain: Some(resolved_terrain),
        simulation,
        unit_atlas,
        palette_set,
        sprite_atlas,
        overlay_atlas,
        bridge_atlas,
        bridge_railing_atlas,
        sidebar_cameo_atlas,
        sidebar_chrome,
        software_cursor,
        overlays: overlays_connected,
        terrain_objects: map_data.terrain_objects,
        waypoints: map_data.waypoints,
        cell_tags: map_data.cell_tags,
        tags: map_data.tags,
        triggers: map_data.triggers,
        events: map_data.events,
        actions: map_data.actions,
        trigger_graph: map_data.trigger_graph,
        trigger_runtime,
        overlay_names,
        tiberium_radar_colors,
        overlay_registry,
        house_color_map,
        house_roster,
        height_map,
        bridge_height_map,
        tactical_bridge_inverse_map,
        path_grid,
        rules,
        art_registry: art,
        infantry_sequences,
        csf,
        fnt_file,
        lighting_grid,
        map_lighting_config: lighting_config,
        theater_name,
        theater_ext: theater_ext.to_string(),
        sandbox_full_visibility: false,
        spawn_pick_pending,
        initial_local_owner,
        camera_x,
        camera_y,
        asset_manager: Some(asset_manager),
    })
}

/// Register non-local playable houses as AI opponents.
fn setup_ai_players(
    sim: &mut crate::sim::world::Simulation,
    house_roster: &HouseRoster,
    local_owner: &str,
) {
    use crate::sim::ai::AiPlayerState;

    for house in &house_roster.houses {
        // Skip neutral/civilian/special houses.
        let up = house.name.to_ascii_uppercase();
        if matches!(
            up.as_str(),
            "NEUTRAL" | "SPECIAL" | "CIVILIAN" | "GOODGUY" | "BADGUY" | "JP"
        ) {
            continue;
        }
        // Skip the local player.
        if house.name.eq_ignore_ascii_case(local_owner) {
            continue;
        }
        sim.ai_players
            .push(AiPlayerState::new(sim.interner.intern(&house.name)));
        log::info!("AI player registered: {}", house.name);
    }
}
