//! App state transitions: map loading into InGame, screen clearing.
//!
//! Extracted from app.rs for file-size limits.

use std::collections::{BTreeMap, HashMap};

use crate::app::loading::init;
use crate::app::presentation::render;
use crate::app::match_runtime::sim_tick;
use crate::map::basic::BasicSection;
use crate::map::houses::HouseRoster;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::trigger_graph::TriggerGraph;
use crate::render::minimap::MinimapRenderer;
use crate::render::selection_overlay::SelectionOverlay;
use crate::sidebar::SidebarTab;
use crate::sim::trigger_runtime::TriggerRuntime;
use crate::ui::game_screen::GameScreen;

use crate::app::AppState;

/// Background clear color for menu screens (dark blue).
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.02,
    g: 0.04,
    b: 0.12,
    a: 1.0,
};

/// Mirror the authoritative speed whenever a complete Simulation replaces the
/// app's live match, including both fresh-map handoff and in-scenario load.
pub(crate) fn sync_in_game_options_speed_from_sim(state: &mut AppState) {
    let Some(game_speed) = state
        .sim_runtime
        .as_ref()
        .map(|rt| &rt.simulation)
        .and_then(crate::sim::world::Simulation::projected_in_game_options_speed)
        .map(u32::from)
    else {
        return;
    };
    state.match_presentation.in_game_options.game_speed = game_speed;
    state.sim_speed_tps = crate::app::types::tps_for_game_speed(game_speed);
}

pub(crate) fn fallback_map_load_result() -> init::MapLoadResult {
    init::MapLoadResult {
        scenario: init::ScenarioLoadInputs {
            startup: crate::match_bootstrap::LoadingStartup::Generic {
                selected_map_file: "fallback".to_string(),
            },
            map_source: crate::app::frontend::list_maps::LoadedMapSource::LegacyFallback {
                label: "fallback".to_string(),
            },
            map_hash: None,
            basic: BasicSection::default(),
            terrain_grid: None,
            resolved_terrain: None,
            simulation: None,
            overlays: Vec::new(),
            terrain_objects: Vec::new(),
            waypoints: HashMap::new(),
            cell_tags: HashMap::new(),
            tags: HashMap::new(),
            triggers: HashMap::new(),
            events: HashMap::new(),
            actions: HashMap::new(),
            trigger_graph: TriggerGraph::default(),
            trigger_runtime: TriggerRuntime::default(),
            overlay_registry: OverlayTypeRegistry::empty(),
            house_roster: HouseRoster::default(),
            height_map: BTreeMap::new(),
            bridge_height_map: BTreeMap::new(),
            tactical_bridge_inverse_map: BTreeMap::new(),
            rules: None,
            map_lighting_config: crate::map::lighting::LightingConfig::default(),
            theater_name: "TEMPERATE".to_string(),
            theater_ext: "tem".to_string(),
            initial_local_owner: None,
            sandbox_full_visibility: false,
            spawn_pick_pending: false,
            camera_anchor_x: 0.0,
            camera_anchor_y: 0.0,
        },
        presentation: init::PresentationLoadAssets {
            tile_atlas: None,
            unit_atlas: None,
            palette_set: None,
            sprite_atlas: None,
            overlay_atlas: None,
            bridge_atlas: None,
            bridge_railing_atlas: None,
            sidebar_cameo_atlas: None,
            sidebar_chrome: None,
            software_cursor: None,
            overlay_names: BTreeMap::new(),
            tiberium_radar_colors: HashMap::new(),
            house_color_map: HashMap::new(),
            lighting_grid: crate::map::lighting::CellLightGrid::new(),
            csf: None,
            fnt_file: None,
        },
        asset_manager: None,
    }
}

pub(crate) fn apply_map_load_result(state: &mut AppState, result: init::MapLoadResult) {
    crate::app::reset_scenario_exit_runtime(state);
    let startup = result.scenario.startup;
    let returns_scenario_rng_to_offline_shell = startup.launch_session().is_some();
    // A loaded world is not timed until the launch handoff actually reaches
    // InGame (SpawnPick remains outside the scenario elapsed span).
    state.scenario_elapsed_clock.reset();
    state.match_presentation.tile_atlas = result.presentation.tile_atlas;
    crate::app::loading::pump::clear_loading_state(state);
    state.map_basic = result.scenario.basic;
    state.loaded_map_source = Some(result.scenario.map_source);
    state.loaded_map_hash = result.scenario.map_hash;
    state.match_presentation.terrain_grid = result.scenario.terrain_grid;
    state.frontend.shell_preview_overlay_registry = Some(result.scenario.overlay_registry.clone());
    // F10 lifecycle: a new match install closes the outgoing diagnostic
    // segment before the runtime slot is overwritten — the old install
    // dropped any unflushed segment silently. A failed close discards
    // rather than retains, so the new timeline cannot append under the
    // old header.
    crate::app::match_runtime::sim_tick::close_replay_segment_for_new_timeline(state);
    let match_rules = result.scenario.rules;
    state.sim_runtime = result.scenario.simulation.zip(match_rules).map(|(simulation, rules)| crate::sim::runtime::SimRuntime {
        simulation,
        resources: crate::sim::runtime::SimResources {
            height_map: result.scenario.height_map,
            bridge_height_map: result.scenario.bridge_height_map,
            overlay_registry: result.scenario.overlay_registry,
            terrain_template: result.scenario.resolved_terrain,
            rules,
            trigger_graph: result.scenario.trigger_graph,
            triggers: result.scenario.triggers,
            events: result.scenario.events,
            actions: result.scenario.actions,
        },
    });
    state.match_presentation.combat_lights.clear();
    sync_in_game_options_speed_from_sim(state);
    if let Some(sim) = state.sim_runtime.as_mut().map(|rt| &mut rt.simulation) {
        sim.set_input_delay_ticks(state.configured_input_delay_ticks);
    }
    state.match_presentation.unit_atlas = result.presentation.unit_atlas;
    state.match_presentation.palette_set = result.presentation.palette_set;
    state.match_presentation.sprite_atlas = result.presentation.sprite_atlas;
    state.match_presentation.overlay_atlas = result.presentation.overlay_atlas;
    state.match_presentation.bridge_atlas = result.presentation.bridge_atlas;
    state.match_presentation.bridge_railing_atlas = result.presentation.bridge_railing_atlas;
    state.match_presentation.sidebar_cameo_atlas = result.presentation.sidebar_cameo_atlas;
    state.match_presentation.sidebar_chrome = result.presentation.sidebar_chrome;
    if let Some(ref fnt) = result.presentation.fnt_file {
        state.renderer.bit_font =
            crate::render::bit_font::BitFont::from_fnt(&state.renderer.gpu, &state.renderer.batch_renderer, fnt);
    }

    // Initialize radar animation from the default (Allied) sidebar chrome atlas.
    // Uses pre-rendered radar.shp frames for the 33-frame open/close animation.
    // Also extract content insets derived from the transparent opening in frame 0.
    let allied_radar = state
        .match_presentation.sidebar_chrome
        .as_ref()
        .and_then(|set| set.resolve_theme(crate::render::sidebar_chrome::SidebarTheme::Allied))
        .map(|resolved| {
            (
                resolved.identity(),
                resolved.atlas.radar_frames.clone(),
                resolved.atlas.radar_frame_size,
                resolved.atlas.radar_content_insets,
            )
        });
    if let Some((identity, frames, [w, h], insets)) = allied_radar {
        state.match_presentation.radar_animation_source = Some(identity);
        state.match_presentation.radar_anim = crate::render::radar_anim::RadarAnimState::new(
            &state.renderer.gpu,
            &state.renderer.batch_renderer,
            frames,
            w,
            h,
        );
        state.match_presentation.radar_content_insets = Some(insets);
    } else {
        state.match_presentation.radar_animation_source = None;
        state.match_presentation.radar_anim = None;
        state.match_presentation.radar_content_insets = None;
    }
    state.match_presentation.has_radar = false;

    state.match_presentation.software_cursor = result.presentation.software_cursor;
    state.match_presentation.overlays.replace_from_source(result.scenario.overlays);
    state.match_presentation.terrain_objects = result.scenario.terrain_objects;
    state.match_presentation.waypoints = result.scenario.waypoints;
    state.match_presentation.cell_tags = result.scenario.cell_tags;
    state.match_presentation.tags = result.scenario.tags;
    if let Some(sim) = state.sim_runtime.as_mut().map(|rt| &mut rt.simulation) {
        sim.install_trigger_runtime(result.scenario.trigger_runtime);
    }
    state.match_presentation.overlay_names = result.presentation.overlay_names;
    state.match_presentation.tiberium_radar_colors = result.presentation.tiberium_radar_colors;
    state.match_presentation.house_color_map = result.presentation.house_color_map;
    state.match_presentation.house_roster = result.scenario.house_roster;
    state.match_presentation.tactical_bridge_inverse_map = result.scenario.tactical_bridge_inverse_map;
    state.match_presentation.lighting_grid = result.presentation.lighting_grid;
    state.match_presentation.applied_lighting_sources.clear();
    state.match_presentation.applied_lighting_profile = None;
    state.match_presentation.applied_lighting_detail_level = state.match_presentation.in_game_options.detail_level.min(2);
    state.match_presentation.pending_lighting_refresh = None;
    state.match_presentation.map_lighting_config = result.scenario.map_lighting_config;
    state.match_presentation.last_lighting_view_fingerprint = None;
    // F04: the app no longer stores a second ArtRegistry; presentation
    // borrows the sole copy owned by RuleSet (state.rules).
    state.csf = result.presentation.csf;
    state.match_presentation.theater_name = result.scenario.theater_name;
    state.match_presentation.theater_ext = result.scenario.theater_ext;

    // The background loader has no access to the live renderer detail option.
    // Re-derive once at handoff so the first visible frame already uses the
    // selected detail mask and its corresponding building-light gate.
    let initial_lighting = match (
        state.terrain_template(),
        state.sim_runtime.as_ref().map(|rt| &rt.simulation),
        state.rules(),
    ) {
        (Some(terrain), Some(sim), Some(rules)) => {
            let view = crate::app::loading::init::derive_lighting_view(
                &state.match_presentation.map_lighting_config,
                Some(sim),
                Some(rules),
                state.match_presentation.in_game_options.detail_level,
            );
            let fingerprint = view.fingerprint;
            let profile = view.profile;
            let detail_level = view.detail_level;
            let point_lights = view.point_lights.clone();
            let grid = crate::app::loading::init::build_lighting_grid_from_view(terrain, &view);
            Some((fingerprint, profile, detail_level, point_lights, grid))
        }
        _ => None,
    };
    if let Some((fingerprint, profile, detail_level, point_lights, grid)) = initial_lighting {
        state.match_presentation.lighting_grid = grid;
        state.match_presentation.last_lighting_view_fingerprint = Some(fingerprint);
        state.match_presentation.applied_lighting_profile = Some(profile);
        state.match_presentation.applied_lighting_detail_level = detail_level;
        state.match_presentation.applied_lighting_sources = point_lights;
    }
    // Map load hands over a world anchor point; the transition applies the
    // active tactical rectangle and live zoom.
    let (tactical_width, tactical_height) =
        crate::app::input::camera::tactical_viewport_size_px(state.render_width(), state.render_height());
    let (camera_x, camera_y) = crate::app::input::camera::tactical_camera_top_left(
        (result.scenario.camera_anchor_x, result.scenario.camera_anchor_y),
        tactical_width as f32,
        tactical_height as f32,
        state.input.zoom_level,
    );
    state.input.camera_x = camera_x;
    state.input.camera_y = camera_y;
    // gamemd's scenario reader fills all four camera bookmarks with the opening
    // view cell, so F1 before any Ctrl+F1 is a valid "go home".
    crate::app::input::camera::seed_view_bookmarks_from_current_view(state);
    // F11 slot: only an actually-carried manager returns (Loading ->
    // Available). The fallback result carries None — the old unconditional
    // assignment wiped the manager the failure path had just restored,
    // discarding the process-sticky MIX cache and theater identity.
    if let Some(manager) = result.asset_manager {
        state.process_assets.return_from_loading(manager);
    }
    state.input.targeting_mode = None;
    state.input.building_placement_preview = None;
    state.match_presentation.active_sidebar_tab = SidebarTab::default_active_tab();
    state.match_presentation.sidebar_scroll_rows = 0;
    state.match_presentation.sidebar_scroll_rows_parked = [0; 4];
    // Re-init the message surface per scenario (the native list is
    // re-initialized at scenario start): drops stale rows from the previous
    // game and any dangling pause span, so a pause→quit→new-map sequence
    // never folds the menu dwell into the new game's frozen-deadline clock.
    // Anchors mirror the AppState ctor; x/width re-sync on first use.
    state.match_presentation.message_list = crate::ui::messages::MessageList::new(
        3,
        0,
        crate::ui::messages::MESSAGE_MAX_VISIBLE_RETAIL,
        0,
    );
    state.match_presentation.message_clock = crate::ui::messages::PauseAwareClock::default();
    let map_title: &str = state.map_basic.name.as_deref().unwrap_or("Unknown Map");
    state.platform.window.set_title(&format!("RA2 - {}", map_title));
    state
        .platform
        .window
        .set_cursor_visible(state.match_presentation.software_cursor.is_none());

    // Create minimap from terrain grid with overlay data.
    if let Some(grid) = &state.match_presentation.terrain_grid {
        let overlay_data: Vec<(
            u16,
            u16,
            crate::render::minimap::OverlayClassification,
            u8,
            Option<[u8; 4]>,
        )> = build_minimap_overlay_data(
            state.match_presentation.overlays.as_slice(),
            &state.match_presentation.terrain_objects,
            &state.match_presentation.overlay_names,
            state.rules(),
            &state.match_presentation.tiberium_radar_colors,
        );
        state.match_presentation.minimap = Some(MinimapRenderer::new(
            &state.renderer.gpu,
            &state.renderer.batch_renderer,
            grid,
            &overlay_data,
            &state.match_presentation.theater_name,
        ));
    }
    state.input.minimap_dragging = false;
    state.input.tactical_mouse = Default::default();
    state.input.keys_held.clear();
    let (tactical_width, tactical_height) =
        crate::app::input::camera::tactical_viewport_size_px(state.render_width(), state.render_height());
    state.input.cursor_x = tactical_width as f32 * 0.5;
    state.input.cursor_y = tactical_height as f32 * 0.5;

    // Create selection overlay for rendering highlights and drag rect.
    // Pass asset_manager so it can load pips.shp for authentic health bar pips.
    state.match_presentation.selection_overlay = Some(SelectionOverlay::new(
        &state.renderer.gpu,
        &state.renderer.batch_renderer,
        state.process_assets.manager(),
    ));

    // Create GPU ABuffer for per-pixel shroud darkening.
    // Loads SHROUD.SHP brightness data and the 256-byte edge LUT.
    if let Some(am) = state.process_assets.manager() {
        if let Some(grid) = state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation)
            .and_then(crate::sim::world::Simulation::path_grid)
        {
            if let Some(shp_data) = am.get_ref("shroud.shp") {
                if let Ok(shp) = crate::assets::shp_file::ShpFile::from_bytes(shp_data) {
                    let (frame_pixels, cw, ch) =
                        crate::render::shroud_buffer::extract_shp_brightness(&shp);
                    state.match_presentation.shroud_buffer = Some(crate::render::shroud_buffer::ShroudBuffer::new(
                        &state.renderer.gpu,
                        state.render_width(),
                        state.render_height(),
                        grid.width(),
                        grid.height(),
                        frame_pixels,
                        cw,
                        ch,
                        crate::render::shroud_buffer::SHROUD_EDGE_LUT,
                    ));
                }
            }
        }
    }

    state.platform.frame_pacer.reset_for_immediate_frame();
    state.input.queued_order_mode = render::OrderMode::Move;
    for group in &mut state.input.control_groups {
        group.clear();
    }
    // Pin the match-scoped local player once at launch. When the launch flow
    // supplies no identity (dev/sandbox), the pin stays None and the legacy
    // override/heuristic path resolves the owner instead.
    state.local_player_owner = result.scenario.initial_local_owner.clone();
    state.local_owner_override = result.scenario.initial_local_owner;
    // F11: reset the whole per-match audio owner. The old reset cleared only
    // three EVA latches — the tick-indexed under-attack suppression window
    // carried into the new match (whose tick counter restarts at 0) and
    // silenced the under-attack EVA line for its first ~30 seconds, and the
    // sound-event queue kept the previous match's undrained events.
    state.match_audio.reset_for_new_match();
    state.sandbox_full_visibility = result.scenario.sandbox_full_visibility;
    state.input.spawn_pick_pending = result.scenario.spawn_pick_pending;

    // Load sound.ini / soundmd.ini for SFX sound ID resolution.
    if let Some(assets) = state.process_assets.manager() {
        state.audio.sound_registry = load_sound_registry(assets);
        state.audio.audio_indices = if crate::app::should_load_audio_indices(state.audio.audio_indices_enabled)
        {
            load_audio_indices(assets)
        } else {
            Vec::new()
        };
        state.audio.eva_registry = load_eva_registry(assets);
    }

    // Start_Scenario resolves `[Basic] Theme` as a theme-section key, then
    // leaves the current shell stream in place while Theme owns its fade and
    // deferred QueueSong/automatic request.
    let music_now_ms = sim_tick::monotonic_frame_pacer_ms(state, std::time::Instant::now());
    if let (Some(player), Some(assets)) = (&mut state.audio.music_player, state.process_assets.manager()) {
        let request = player.resolve_scenario_theme(state.map_basic.theme.as_deref(), assets);
        player.request_scenario_theme(request, music_now_ms);
    }

    if state.input.spawn_pick_pending {
        crate::app::loading::pump::clear_match_startup_state(state);
        state.frontend.screen = GameScreen::SpawnPick;
        if returns_scenario_rng_to_offline_shell {
            state
                .frontend.offline_skirmish_runtime
                .mark_gameplay_rng_return_pending();
        }
        log::info!("Transitioned to SpawnPick — player must choose a start location");
    } else {
        match startup {
            crate::match_bootstrap::LoadingStartup::Accepted(prepared) => {
                let receipt = (|| {
                    let simulation = state
                        .sim_runtime
                        .as_ref()
                        .map(|rt| &rt.simulation)
                        .ok_or_else(|| "accepted map load produced no Simulation".to_string())?;
                    let active_correlation = state.frontend.active_loading_correlation.ok_or_else(|| {
                        "accepted map load lost its active correlation".to_string()
                    })?;
                    crate::match_bootstrap::RustL0Observation {
                        startup: &prepared,
                        simulation,
                        active_correlation,
                        prior_receipt: state.frontend.rust_l0_receipt.as_ref(),
                        screen_is_loading: matches!(state.frontend.screen, GameScreen::Loading),
                        spawn_pick_active: state.input.spawn_pick_pending,
                    }
                    .acknowledge()
                    .map_err(|err| err.to_string())
                })();

                match receipt {
                    Ok(receipt) => {
                        state.frontend.loaded_startup = Some(prepared);
                        state.frontend.rust_l0_receipt = Some(receipt);
                        state.frontend.active_loading_correlation = None;
                        let now_ms = sim_tick::monotonic_frame_pacer_ms(
                            state,
                            std::time::Instant::now(),
                        );
                        state.scenario_elapsed_clock.start(now_ms);
                        state.frontend.screen = GameScreen::InGame;
                        state
                            .frontend.offline_skirmish_runtime
                            .mark_gameplay_rng_return_pending();
                        log::info!("Transitioned to InGame after Rust L0 acknowledgement");
                    }
                    Err(err) => {
                        crate::app::loading::pump::clear_match_startup_state(state);
                        state.frontend.screen = GameScreen::MissionResult {
                            title: "Startup Rejected".to_string(),
                            detail: err.clone(),
                        };
                        log::error!("Accepted startup failed closed at Rust L0: {err}");
                    }
                }
            }
            crate::match_bootstrap::LoadingStartup::UnverifiedLegacy { .. }
            | crate::match_bootstrap::LoadingStartup::Generic { .. } => {
                state.frontend.active_loading_correlation = None;
                state.frontend.loaded_startup = None;
                state.frontend.rust_l0_receipt = None;
                let now_ms =
                    sim_tick::monotonic_frame_pacer_ms(state, std::time::Instant::now());
                state.scenario_elapsed_clock.start(now_ms);
                state.frontend.screen = GameScreen::InGame;
                if returns_scenario_rng_to_offline_shell {
                    state
                        .frontend.offline_skirmish_runtime
                        .mark_gameplay_rng_return_pending();
                }
                log::info!("Transitioned to InGame on noncertifying startup path");
            }
        }
    }
    crate::app::presentation::sidebar_render::refresh_sidebar_projection(state);
}

/// Load sound.ini / soundmd.ini and build a SoundRegistry.
/// YR-first: soundmd.ini takes precedence, sound.ini fills gaps.
pub(crate) fn load_sound_registry(
    assets: &crate::assets::asset_manager::AssetManager,
) -> crate::rules::sound_ini::SoundRegistry {
    use crate::rules::ini_parser::IniFile;
    use crate::rules::sound_ini::SoundRegistry;

    // Try YR sound.ini first (soundmd.ini).
    let mut registry: Option<SoundRegistry> = None;
    for name in ["soundmd.ini", "sound.ini"] {
        if let Some(bytes) = assets.get(name) {
            if let Ok(text) = String::from_utf8(bytes) {
                let ini: IniFile = IniFile::from_str(&text);
                match &mut registry {
                    None => {
                        registry = Some(SoundRegistry::from_ini(&ini));
                        log::info!("Loaded {} for SFX", name);
                    }
                    Some(reg) => {
                        reg.merge_fallback(&ini);
                        log::info!("Merged fallback {} for SFX", name);
                    }
                }
            }
        }
    }
    registry.unwrap_or_default()
}

/// Load audio.idx/bag indices for bag-based sound playback (voices, EVA).
///
/// Tries YR (audiomd) first, then base RA2 (audio). Both are loaded if present
/// so YR sounds take priority but base RA2 sounds are still available.
pub(crate) fn load_audio_indices(
    assets: &crate::assets::asset_manager::AssetManager,
) -> Vec<crate::assets::audio_bag::AudioIndex> {
    use crate::assets::audio_bag::AudioIndex;

    let mut indices = Vec::new();

    // Both AUDIO.MIX and AUDIOMD.MIX contain entries named "audio.idx" and "audio.bag"
    // internally. We need to load each MIX explicitly and extract from within, because
    // the generic first-match lookup would conflate the shared internal filenames.
    // YR (AUDIOMD.MIX) is loaded first so its sounds take priority in the search.
    for mix_name in ["AUDIOMD.MIX", "AUDIO.MIX"] {
        let Some(mix) = assets.archive(mix_name) else {
            continue;
        };
        let idx_data = match mix.get_by_name("audio.idx") {
            Some(d) => d,
            None => {
                log::warn!("{} has no audio.idx entry", mix_name);
                continue;
            }
        };
        let bag_data = match mix.get_by_name("audio.bag") {
            Some(d) => d.to_vec(),
            None => {
                log::warn!("{} has audio.idx but no audio.bag", mix_name);
                continue;
            }
        };
        match AudioIndex::from_idx_bag(idx_data, bag_data) {
            Some(index) => {
                log::info!(
                    "Loaded audio.idx/bag from {}: {} entries",
                    mix_name,
                    index.len()
                );
                indices.push(index);
            }
            None => {
                log::warn!("Failed to parse audio.idx from {}", mix_name);
            }
        }
    }

    if indices.is_empty() {
        log::warn!("No audio.idx/bag found — bag-based sounds (voices, EVA) will be silent");
    }
    indices
}

/// Load eva.ini / evamd.ini and build an EvaRegistry.
/// YR-first: evamd.ini takes precedence, eva.ini fills gaps.
pub(crate) fn load_eva_registry(
    assets: &crate::assets::asset_manager::AssetManager,
) -> crate::rules::sound_ini::EvaRegistry {
    use crate::rules::ini_parser::IniFile;
    use crate::rules::sound_ini::EvaRegistry;

    let mut registry: Option<EvaRegistry> = None;
    for name in ["evamd.ini", "eva.ini"] {
        if let Some(bytes) = assets.get(name) {
            // EVA INI files from MIX archives may contain non-UTF8 bytes (Windows-1252).
            let text = String::from_utf8_lossy(&bytes);
            let ini: IniFile = IniFile::from_str(&text);
            match &mut registry {
                None => {
                    registry = Some(EvaRegistry::from_ini(&ini));
                    log::info!("Loaded {} for EVA", name);
                }
                Some(reg) => {
                    reg.merge_fallback(&ini);
                    log::info!("Merged fallback {} for EVA", name);
                }
            }
        }
    }
    registry.unwrap_or_default()
}

/// Build overlay classification data for the minimap from map overlay entries.
///
/// Classifies each overlay by name pattern (TIB* = ore, GEM* = gem, WALL/FENCE = wall,
/// BRIDGE/BRDG = bridge) and each terrain object as TerrainObject.
fn build_minimap_overlay_data(
    overlays: &[crate::map::overlay::OverlayEntry],
    terrain_objects: &[crate::map::overlay::TerrainObject],
    overlay_names: &BTreeMap<u8, String>,
    _rules: Option<&crate::rules::ruleset::RuleSet>,
    tiberium_colors: &HashMap<(u8, u8), [u8; 3]>,
) -> Vec<(
    u16,
    u16,
    crate::render::minimap::OverlayClassification,
    u8,
    Option<[u8; 4]>,
)> {
    use crate::render::minimap::OverlayClassification;

    let mut data: Vec<(u16, u16, OverlayClassification, u8, Option<[u8; 4]>)> =
        Vec::with_capacity(overlays.len() + terrain_objects.len());

    for entry in overlays {
        let name: &str = match overlay_names.get(&entry.overlay_id) {
            Some(n) => n.as_str(),
            None => continue,
        };
        let upper: String = name.to_ascii_uppercase();
        let classification: OverlayClassification = if upper.starts_with("GEM") {
            OverlayClassification::Gem
        } else if upper.starts_with("TIB") {
            OverlayClassification::Ore
        } else if upper.contains("WALL") || upper.contains("FENCE") {
            OverlayClassification::Wall
        } else if upper.contains("BRIDGE") || upper.contains("BRDG") {
            OverlayClassification::Bridge
        } else {
            OverlayClassification::Other
        };
        // For tiberium overlays (Ore/Gem), look up the precomputed SHP-derived color.
        let precomputed: Option<[u8; 4]> = match classification {
            OverlayClassification::Ore | OverlayClassification::Gem => tiberium_colors
                .get(&(entry.overlay_id, entry.frame))
                .map(|&[r, g, b]| [r, g, b, 255]),
            _ => None,
        };
        data.push((entry.rx, entry.ry, classification, entry.frame, precomputed));
    }

    for obj in terrain_objects {
        data.push((
            obj.rx,
            obj.ry,
            OverlayClassification::TerrainObject,
            0,
            None,
        ));
    }

    data
}

/// Clear the screen to the background color (no depth buffer).
pub(crate) fn clear_screen(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Clear Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
}

#[cfg(test)]
mod game_speed_tests {
    use crate::sim::command::{Command, CommandEnvelope};
    use crate::sim::house_state::HouseState;
    use crate::sim::world::Simulation;

    #[test]
    fn loaded_lobby_speed_projection_includes_due_transition() {
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Local");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, true, 0, 10));
        sim.session.house_order.push(owner);
        sim.queue_command(CommandEnvelope::new(
            owner,
            1,
            Command::SetGameSpeed { speed: 4 },
        ));

        assert_eq!(sim.projected_in_game_options_speed(), Some(4));
    }
}
