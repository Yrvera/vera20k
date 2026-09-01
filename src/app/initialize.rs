//! Process, window, GPU, frontend, and initial app-state construction.

use std::path::Path;

use crate::app::frontend::startup_options::{RetailStartupOptions, ScreenSize};
use crate::app::persistence::options_profile::{RetailOptionsLoad, RetailOptionsProfile};

use super::presentation::render;
use super::frontend::list_maps;
use super::{
    ActiveEventLoop, App, AppState, Arc, AssetManager, BTreeMap, BasicSection, BatchRenderer,
    BitFont, CellLightGrid, DEV_SKIRMISH_SHELL_ENV, EguiIntegration, GameConfig, GameScreen,
    GpuContext, HashMap, HashSet, HouseRoster, Instant, LightingConfig,
    ModifiersState, MusicPlayer, PhysicalSize, PlatformState, RandomMapGenerationRetention,
    Result,
    SelectionState, SfxPlayer, SidebarChromeLayoutSpec,
    SidebarTab, StartupAudioDisposition, Window, WindowAttributes,
    auto_detect_ui_scale, frontend::startup_splash,
    should_load_audio_indices,
};

fn startup_window_projection(
    profile_screen: ScreenSize,
    capture_dimensions: Option<(u32, u32)>,
) -> (u32, u32, bool) {
    capture_dimensions.map_or(
        (profile_screen.width, profile_screen.height, true),
        |(width, height)| (width, height, false),
    )
}

fn select_startup_options_load<LoadProfile>(
    capture_dimensions: Option<(u32, u32)>,
    startup_options: &RetailStartupOptions,
    ra2_dir: Option<&Path>,
    load_profile: LoadProfile,
) -> RetailOptionsLoad
where
    LoadProfile: FnOnce(&Path, &RetailStartupOptions) -> RetailOptionsLoad,
{
    if capture_dimensions.is_some() {
        RetailOptionsLoad::without_ra2md(&RetailStartupOptions::default())
    } else if let Some(ra2_dir) = ra2_dir {
        load_profile(ra2_dir, startup_options)
    } else {
        RetailOptionsLoad::without_ra2md(startup_options)
    }
}

trait StartupAudioProfileOperations {
    fn set_sound_volume(&mut self, volume: f64);
    fn set_voice_volume(&mut self, volume: f64);
    fn set_score_volume(&mut self, volume: f64);
}

impl StartupAudioProfileOperations for crate::app::audio_runtime::AppAudioRuntime {
    fn set_sound_volume(&mut self, volume: f64) {
        if let Some(player) = self.sfx_player.as_mut() {
            player.set_sound_volume(volume);
        }
    }

    fn set_voice_volume(&mut self, volume: f64) {
        if let Some(player) = self.sfx_player.as_mut() {
            player.set_voice_volume(volume);
        }
    }

    fn set_score_volume(&mut self, volume: f64) {
        if let Some(player) = self.music_player.as_mut() {
            player.set_volume(volume);
        }
    }
}

fn apply_startup_audio_profile(
    profile: &RetailOptionsProfile,
    operations: &mut impl StartupAudioProfileOperations,
) {
    // gamemd-derived: the Audio tail of `OptionsClass__ReadFromINI @ 0x005FA620`
    // applies SoundVolume, VoiceVolume, then ScoreVolume through distinct owners.
    operations.set_sound_volume(f64::from(profile.sound_volume));
    operations.set_voice_volume(f64::from(profile.voice_volume));
    operations.set_score_volume(f64::from(profile.score_volume));
}

impl App {
    fn build_startup_asset_manager(config: Option<&GameConfig>) -> Option<AssetManager> {
        config.and_then(|cfg| match AssetManager::new(&cfg.paths.ra2_dir) {
            Ok(manager) => Some(manager),
            Err(err) => {
                log::warn!("Could not load startup shell assets: {err:#}");
                None
            }
        })
    }

    fn load_version_txt() -> String {
        crate::util::version::retail_internal_version().to_owned()
    }

    /// Create window, GPU context, and egui integration. Does NOT load a map —
    /// starts in MainMenu state. Map loading is deferred to when the user
    /// clicks "Quick Play".
    pub(super) fn initialize(
        event_loop: &ActiveEventLoop,
        capture_dimensions: Option<(u32, u32)>,
        startup_options: RetailStartupOptions,
    ) -> Result<AppState> {
        // One parsed retail profile snapshot yields two ordered products: the
        // early Video/fallback pair that selects the window, and the later
        // full-read profile retained by persistence. Capture is a sealed
        // automation lane, so it uses exact defaults and its explicit
        // dimensions instead of ingesting operator argv/profile screen state.
        let game_config = GameConfig::load().ok();
        let options_load = select_startup_options_load(
            capture_dimensions,
            &startup_options,
            game_config
                .as_ref()
                .map(|config| config.paths.ra2_dir.as_path()),
            RetailOptionsLoad::from_ra2md,
        );
        let profile_screen = options_load.startup_screen;
        let options_profile = options_load.retained_profile;
        let (window_width, window_height, window_visible) =
            startup_window_projection(profile_screen, capture_dimensions);
        let shell_client_size = PhysicalSize::new(window_width, window_height);
        let startup_audio =
            StartupAudioDisposition::for_audio_enabled(startup_options.audio_enabled);
        let window_attrs: WindowAttributes = WindowAttributes::default()
            .with_title("RA2 Engine")
            .with_inner_size(PhysicalSize::new(window_width, window_height))
            .with_resizable(false)
            .with_visible(window_visible)
            .with_active(window_visible);
        let window: Arc<Window> = Arc::new(event_loop.create_window(window_attrs)?);
        let gpu: GpuContext = GpuContext::new(window.clone())?;
        let egui: EguiIntegration = EguiIntegration::new(&gpu, &window);
        let batch_renderer: BatchRenderer = BatchRenderer::new(&gpu);
        let combat_light_renderer = crate::render::combat_light::CombatLightRenderer::new(&gpu);
        let mut bit_font = BitFont::fallback_5x7(&gpu, &batch_renderer);
        let depth_view: wgpu::TextureView = gpu.create_depth_texture();
        let shell_surface_presenter =
            crate::render::shell_surface_present::ShellSurfacePresenter::new(&gpu)?;
        let input_delay_ticks: u64 = game_config
            .as_ref()
            .map(|cfg| cfg.gameplay.input_delay_ticks.max(1) as u64)
            .unwrap_or(2);
        let upscale_pass = game_config
            .as_ref()
            .filter(|cfg| cfg.graphics.upscale)
            .map(|cfg| {
                let rw = cfg.graphics.render_width();
                let rh = cfg.graphics.render_height();
                log::info!(
                    "Upscale pass enabled: render at {}x{}, upscale to window",
                    rw,
                    rh,
                );
                crate::render::upscale_pass::UpscalePass::new(&gpu, rw, rh)
            });
        let base_sidebar_layout_spec = SidebarChromeLayoutSpec::load_optional_default()
            .map(|spec| spec.unwrap_or_else(SidebarChromeLayoutSpec::stock))
            .unwrap_or_else(|err| {
                log::warn!("Could not load sidebar layout override: {:#}", err);
                SidebarChromeLayoutSpec::stock()
            });
        // Auto-detect integer UI scale from window size.
        let screen_w = window.inner_size().width;
        let screen_h = window.inner_size().height;
        let ui_scale: f32 = auto_detect_ui_scale(screen_w, screen_h);
        log::info!("UI scale: {}x ({}x{})", ui_scale, screen_w, screen_h);
        let sidebar_layout_spec = base_sidebar_layout_spec.with_scale(ui_scale);
        let vxl_compute = crate::render::vxl_compute::VxlComputeRenderer::new(&gpu.device);
        let dev_skirmish_shell_enabled = Self::dev_skirmish_shell_enabled();
        if dev_skirmish_shell_enabled {
            log::info!(
                "Development Skirmish shell enabled via {}",
                DEV_SKIRMISH_SHELL_ENV
            );
        }
        let mut startup_asset_manager = Self::build_startup_asset_manager(game_config.as_ref());
        // Native process startup seeds Scenario before the MPModes loader. The
        // Cooperative factory reached by that loader then advances this cursor
        // before the first shell is shown.
        let mut frontend_seed_clock = crate::match_bootstrap::OrdinaryMatchSeedClock;
        let frontend_seed = crate::match_bootstrap::read_match_seed(&mut frontend_seed_clock);
        // The splash goes up as soon as the archives are mounted and the two
        // things it draws with are available: native presents it immediately
        // after the mix mount and lets the rules/type initialization run under
        // the artwork, padding out the remaining hold only if that work
        // finished early. The string-table load has to stay ahead of the
        // present — all five text layers fall back to English literals when the
        // CSF is absent.
        //
        // What makes the move safe is that the archive stack is identical at
        // both positions: the only registration that changes it sits after the
        // splash in the old ordering as well, so first-winner resolution for
        // the splash palette and SHP cannot differ. (The steps that moved below
        // do share the asset manager mutably in effect — its mix cache is
        // interior-mutable behind a lock — but caching a lookup does not change
        // which archive wins it.)
        let startup_csf = startup_asset_manager
            .as_ref()
            .map(crate::app::loading::init::load_csf)
            .transpose()?;
        let startup_fnt = startup_asset_manager.as_ref().and_then(|assets| {
            assets.get_ref("GAME.FNT").and_then(|data| {
                crate::assets::fnt_file::FntFile::from_bytes(data)
                    .map_err(|err| log::warn!("Failed to parse startup GAME.FNT: {err}"))
                    .ok()
            })
        });
        if let Some(fnt) = startup_fnt.as_ref() {
            bit_font = BitFont::from_fnt(&gpu, &batch_renderer, &fnt);
        }
        let mut startup_splash = if capture_dimensions.is_none() {
            startup_asset_manager
                .as_ref()
                .zip(startup_fnt.as_ref())
                .and_then(|(assets, fnt)| {
                    startup_splash::StartupSplashPresentation::build(
                        &gpu,
                        &batch_renderer,
                        assets,
                        startup_csf.as_ref(),
                        fnt,
                        gpu.config.width,
                        gpu.config.height,
                    )
                    .map_err(|err| log::warn!("Could not build retail startup splash: {err:#}"))
                    .ok()
                })
        } else {
            None
        };
        if let Some(splash) = startup_splash.as_mut() {
            match startup_splash::render_and_present(
                &gpu,
                &batch_renderer,
                &shell_surface_presenter,
                &depth_view,
                splash,
            ) {
                Ok(()) => splash.mark_presented(Instant::now()),
                Err(err) => {
                    // Surface acquisition can be transient before the first
                    // event-loop redraw. Keep the unarmed splash for retry.
                    log::warn!("Initial retail startup splash present deferred: {err:#}");
                }
            }
        }
        // Everything below runs with the splash already on screen: the
        // presented swapchain frame stays composited while this thread blocks,
        // and the hold armed above is measured from that present, so a slow
        // load is spent inside the five seconds instead of before them.
        let (startup_rules, startup_rules_projection, startup_native_rules) =
            startup_asset_manager
                .as_ref()
                .and_then(crate::app::loading::init_helpers::load_startup_rules)
                .map(crate::app::loading::init_helpers::StartupRulesLoad::into_parts)
                .map(|(rules, projection, owner)| (rules, projection, Some(owner)))
                .unwrap_or((None, None, None));
        let startup_sound_registry = startup_asset_manager
            .as_ref()
            .map(crate::app::loading::transitions::load_sound_registry)
            .unwrap_or_default();
        let startup_audio_indices = if should_load_audio_indices(startup_audio.load_audio_indices) {
            startup_asset_manager
                .as_ref()
                .map(crate::app::loading::transitions::load_audio_indices)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let startup_eva_registry = startup_asset_manager
            .as_ref()
            .map(crate::app::loading::transitions::load_eva_registry)
            .unwrap_or_default();
        if let Some(assets) = startup_asset_manager.as_mut() {
            match assets.register_neutral_archives() {
                Ok(true) => {
                    log::info!("Registered retail neutral shell archives");
                }
                Ok(false) => {
                    log::warn!(
                        "Retail neutral shell archives are unavailable; shell presentation may fall back"
                    );
                }
                Err(err) => {
                    log::warn!("Could not register retail neutral shell archives: {err:#}");
                }
            }
        }
        let skirmish_shell_chrome = if dev_skirmish_shell_enabled {
            startup_asset_manager.as_ref().and_then(|assets| {
                crate::render::skirmish_shell_chrome::build_skirmish_shell_chrome_atlas(
                    &gpu,
                    &batch_renderer,
                    assets,
                )
            })
        } else {
            None
        };
        let main_menu_shell_chrome = startup_asset_manager.as_ref().and_then(|assets| {
            crate::render::main_menu_shell_chrome::build_main_menu_shell_chrome_atlas(
                &gpu,
                &batch_renderer,
                assets,
            )
        });
        let main_menu_shell_failed =
            startup_asset_manager.is_none() || main_menu_shell_chrome.is_none();
        let version_txt = Self::load_version_txt();
        let available_maps = list_maps::list_available_maps().unwrap_or_else(|err| {
            log::warn!("Could not list maps for menu: {:#}", err);
            Vec::new()
        });
        let skirmish_scenario_records =
            match (startup_asset_manager.as_mut(), game_config.as_ref()) {
                (Some(assets), Some(config)) => {
                    list_maps::list_skirmish_scenario_records_with_assets(
                        &config.paths.ra2_dir,
                        assets,
                        startup_csf.as_ref(),
                    )
                }
                _ => Ok(Vec::new()),
            }
            .unwrap_or_else(|err| {
                log::warn!("Could not list Skirmish scenario records: {err:#}");
                Vec::new()
            });
        let skirmish_scenario_records = if skirmish_scenario_records.is_empty() {
            available_maps
                .iter()
                .enumerate()
                .map(|(idx, map)| {
                    crate::map::skirmish_scenarios::SkirmishScenarioRecord::from_map_menu_entry(idx, map)
                })
                .collect()
        } else {
            skirmish_scenario_records
        };
        // F11: the catalog owns the records and derives the shell-map
        // projection internally; nothing re-projects by hand anymore.
        let scenario_catalog =
            crate::app::scenario_catalog::ScenarioCatalog::from_records(skirmish_scenario_records);
        let skirmish_modes = startup_asset_manager
            .as_ref()
            .and_then(
                |assets| match crate::skirmish_modes::skirmish_modes_from_assets(assets) {
                    Ok(modes) => Some(modes),
                    Err(err) => {
                        log::warn!("Could not load Skirmish mode roster: {err}");
                        None
                    }
                },
            )
            .unwrap_or_default();
        let mut skirmish_shell_state = crate::ui::skirmish_shell::SkirmishShellState::default();
        // Seed the Credits/Unit Count slider ranges from rulesmd's
        // [MultiplayerDialogSettings] so a mod that changes the money/unit bounds
        // shifts the slider extents like gamemd does (it reads them from Rules at
        // dialog-build time); without assets we keep the stock-default ranges.
        if let Some(rules_projection) = startup_rules_projection.as_ref() {
            skirmish_shell_state.trackbar_bounds =
                crate::ui::skirmish_shell::SkirmishTrackbarBounds::from_multiplayer_dialog_settings(
                    rules_projection,
                );
            // Seed the per-match option values (Money/UnitCount/TechLevel/
            // GameSpeed and the checkbox toggles) from the merged rules
            // [MultiplayerDialogSettings], so a mod that changes a default opens
            // the dialog on — and launches the match with — its value. Without
            // assets we keep the stock-default values.
            let dialog_options =
                crate::sim::game_options::GameOptions::from_multiplayer_dialog_settings(
                    rules_projection,
                );
            skirmish_shell_state.apply_multiplayer_dialog_values(&dialog_options);
        }
        let skirmish_defaults =
            crate::app::frontend::skirmish_session::skirmish_global_defaults(&skirmish_shell_state);
        let offline_skirmish_runtime =
            crate::app::frontend::skirmish_session::OfflineSkirmishRuntime::initialize(
                frontend_seed.value,
                game_config
                    .as_ref()
                    .map(|config| config.paths.ra2_dir.as_path()),
                startup_asset_manager.as_ref(),
                startup_rules_projection.as_ref(),
                skirmish_defaults,
            );
        offline_skirmish_runtime.hydrate_shell(
            &mut skirmish_shell_state,
            scenario_catalog.shell_maps(),
            &skirmish_modes,
        );
        // Pre-fill the player-name field from the persistent profile name when
        // configured, mirroring the original seeding the field from a profile
        // source rather than always showing a fixed default.
        if let Some(profile_name) = game_config
            .as_ref()
            .and_then(|config| config.profile.player_name())
        {
            skirmish_shell_state.player_name_edit =
                crate::ui::skirmish_shell::PlayerNameEditState::with_name(profile_name);
        }
        crate::ui::skirmish_shell::repair_teams_for_selected_mode(
            &mut skirmish_shell_state,
            &skirmish_modes,
        );
        crate::ui::skirmish_shell::initialize_rows_for_selected_map(
            &mut skirmish_shell_state,
            scenario_catalog.shell_maps(),
        );
        let selected_shell_map = scenario_catalog
            .shell_maps()
            .get(skirmish_shell_state.selected_map_idx)
            .map(|map| map.file_name.as_str());
        let mut skirmish_settings =
            crate::ui::skirmish_shell::launch_settings(&skirmish_shell_state);
        skirmish_settings.selected_map_idx = selected_shell_map
            .and_then(|file_name| {
                available_maps
                    .iter()
                    .position(|map| map.file_name.eq_ignore_ascii_case(file_name))
            })
            .unwrap_or(0);

        // Build the software cursor at startup so the main menu draws the SHP
        // arrow and hides the OS cursor, matching the original which hides the
        // OS cursor for the whole process and blits the cursor SHP every frame.
        let startup_software_cursor = startup_asset_manager.as_ref().and_then(|assets| {
            crate::render::cursor_atlas::build_software_cursor(&gpu, &batch_renderer, assets)
        });
        let hotkey_bindings =
            crate::app::input::hotkeys::HotkeyBindings::load(startup_asset_manager.as_ref());
        let startup_in_game_options =
            crate::app::persistence::options::in_game_options_from_profile(&options_profile);
        let mut startup_tooltips = crate::ui::tooltips::TooltipService::new();
        let mut startup_target_lines =
            crate::app::presentation::target_lines::TargetLineState::default();
        crate::app::persistence::options::apply_presentation_option_gates(
            &mut startup_target_lines,
            &mut startup_tooltips,
            &startup_in_game_options,
        );

        let music_player = startup_audio
            .initialize_music_output
            .then(MusicPlayer::new)
            .flatten();
        let sfx_player = startup_audio
            .initialize_sfx_output
            .then(SfxPlayer::new)
            .flatten();
        let launcher_audio_available =
            crate::app::audio_runtime::derive_launcher_audio_available(
                startup_options.audio_enabled,
                music_player.is_some(),
                sfx_player.is_some(),
            );
        let mut startup_audio_runtime = crate::app::audio_runtime::AppAudioRuntime {
            theme: crate::audio::theme::ThemeRuntime::default(),
            music_player,
            sfx_player,
            sound_registry: startup_sound_registry,
            audio_indices: startup_audio_indices,
            audio_indices_enabled: startup_audio.load_audio_indices,
            launcher_audio_available,
            theme_startup_suppressed: false,
            eva_registry: startup_eva_registry,
        };
        if let Some(assets) = startup_asset_manager.as_ref() {
            startup_audio_runtime.initialize_theme(assets);
        }

        let mut state = AppState {
            platform: PlatformState::new(window, game_config, shell_client_size),
            match_state: crate::app::match_runtime::state::MatchState {
            sim_runtime: None,
            input: crate::app::input::state::MatchInputState {
            minimap_dragging: false,
            selection_state: SelectionState::new(),
            selection_order: Vec::new(),
            selection_order_pending: false,
            selection_voice_enabled: true,
            queued_order_mode: render::OrderMode::Move,
            control_groups: vec![Vec::new(); 10],
            last_control_group_press: None,
            follow_target: None,
            spawn_pick_pending: false,
            targeting_mode: None,
            building_placement_preview: None,
                camera_x: 0.0,
                camera_y: 0.0,
                zoom_level: 1.0,
                zoom_target: 1.0,
                zoom_anchor_world: [0.0, 0.0],
                zoom_anchor_screen: [0.0, 0.0],
                edge_scroll: crate::app::input::camera::EdgeScrollState::default(),
                tactical_mouse: crate::app::input::camera::TacticalMouseState::default(),
                view_bookmarks: crate::app::input::camera::ViewBookmarks::default(),
                cursor_x: 0.0,
                cursor_y: 0.0,
                keys_held: HashSet::new(),
                hotkey_bindings,
                hotkey_modifiers: ModifiersState::empty(),
                type_select: crate::app::types::TypeSelectInputState::default(),
                retail_screenshot_requested: false,
            },
            match_presentation: crate::app::presentation::state::MatchPresentationState {
            power_bar_anim: crate::sidebar::PowerBarAnimState::new(),
            sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState::new(),
            in_game_gadgets: crate::app::input::gadget_input::InGameGadgets::new(),
            sidebar_projection: Default::default(),
            active_sidebar_tab: SidebarTab::default_active_tab(),
            sidebar_layout_spec,
            sidebar_layout_spec_base: base_sidebar_layout_spec,
            ui_scale,
            sidebar_scroll_rows: 0,
            sidebar_scroll_rows_parked: [0; 4],
            tooltips: startup_tooltips,
            tooltip_epoch: Instant::now(),
            message_list: crate::ui::messages::MessageList::new(
                3,
                0,
                crate::ui::messages::MESSAGE_MAX_VISIBLE_RETAIL,
                0,
            ),
            message_clock: crate::ui::messages::PauseAwareClock::default(),
            in_game_menu: crate::ui::pause_menu::InGameMenuState::default(),
            in_game_options: startup_in_game_options,
            in_game_options_anchor: None,
            show_hotkey_help: false,
            show_save_load_panel: false,
            combat_lights: Default::default(),
            minimap: None,
            radar_anim: None,
            radar_animation_source: None,
            radar_content_insets: None,
            has_radar: false,
            selection_overlay: None,
            shroud_buffer: None,
            tactical_bridge_inverse_map: BTreeMap::new(),
            theater_name: "TEMPERATE".to_string(),
            theater_ext: "tem".to_string(),
            target_lines: startup_target_lines,
            pending_fire_effects: Vec::new(),
            garrison_muzzle_flashes: Vec::new(),
            weapon_muzzle_flashes: Vec::new(),
            projectile_visuals: Vec::new(),
            parachute_anims: Vec::new(),
            idle_anim_elapsed_ms: 0,
            building_anim_phase_base: std::collections::BTreeMap::new(),
            cached_overlay_instances: Vec::new(),
            cached_unit_instances: Vec::new(),
            cached_unit_pages: Vec::new(),
                terrain_grid: None,
                installed_playfield_authority: None,
                overlays: Default::default(),
                terrain_objects: Vec::new(),
                waypoints: HashMap::new(),
                cell_tags: HashMap::new(),
                tags: HashMap::new(),
                overlay_names: BTreeMap::new(),
                overlay_radar_colors: HashMap::new(),
                house_color_map: HashMap::new(),
                house_roster: HouseRoster::default(),
                lighting_grid: CellLightGrid::new(),
                applied_lighting_sources: Vec::new(),
                applied_lighting_profile: None,
                applied_lighting_detail_level: 2,
                pending_lighting_refresh: None,
                last_lighting_view_fingerprint: None,
                map_lighting_config: LightingConfig::default(),
                tile_atlas: None,
                unit_atlas: None,
                palette_set: None,
                sprite_atlas: None,
                overlay_atlas: None,
                bridge_atlas: None,
                bridge_railing_atlas: None,
                sidebar_cameo_atlas: None,
                sidebar_chrome: None,
                software_cursor: startup_software_cursor,
            },
            match_audio: Default::default(),
            match_diagnostics: Default::default(),
            map_basic: BasicSection::default(),
            loaded_map_source: None,
            loaded_map_hash: None,
            scenario_outcome: None,
            scenario_exit: None,
            scenario_elapsed_clock: crate::app::match_runtime::frame_pacer::ScenarioElapsedClock::new(),
            configured_input_delay_ticks: input_delay_ticks,
            local_player_owner: None,
            local_owner_override: None,
            sandbox_full_visibility: false,
            paused: false,
            // KD-3: unify the two game-speed sources. `in_game_options.game_speed`
            // (in the presentation owner) is the single source of truth; seed it
            // from the skirmish-setup speed (internal 1) and derive
            // `sim_speed_tps` from the same value, so the Options slider
            // reflects the current pace. The resulting tps is unchanged from
            // the prior `default_yr_skirmish_tps()` (= GS1 -> 63).
            sim_speed_tps: crate::app::types::tps_for_game_speed(
                crate::app::types::DEFAULT_YR_SKIRMISH_GAME_SPEED,
            ),
            },
            frontend: crate::app::frontend::state::FrontendState {
            screen: GameScreen::default(),
            available_maps,
            scenario_catalog,
            skirmish_modes,
            skirmish_settings,
            loading_session: None,
            frontend_main_rng: crate::sim::rng::SimRng::new(u64::from(frontend_seed.value)),
            next_match_correlation: 1,
            active_loading_correlation: None,
            loaded_startup: None,
            rust_l0_receipt: None,
            random_map_generation: None,
            random_map_retention: RandomMapGenerationRetention::default(),
            skirmish_preview_texture: None,
            loading_screen_atlas: None,
            loading_progress: crate::app::loading::pump::LoadingProgressState::standard_skirmish(),
            frontend_rules: startup_rules,
            shell_preview_overlay_registry: None,
            dev_skirmish_shell_enabled,
            skirmish_shell_state,
            offline_skirmish_runtime,
            skirmish_shell_last_painted_pressed_button: None,
            skirmish_shell_chrome,
            main_menu_shell_state: crate::ui::main_menu_shell::MainMenuShellState::default(),
            single_player_shell_state:
                crate::ui::single_player_shell::SinglePlayerShellState::default(),
            shell_controller: crate::ui::shell::controller::DialogController::default(),
            main_menu_shell_chrome,
            main_menu_movie: None,
            main_menu_movie_identity: None,
            main_menu_movie_last_step: Instant::now(),
            main_menu_shell_failed,
            version_txt,
            shell_first_paint_slide: None,
            shell_slide_active_shell: None,
            shell_slide_generation: 0,
            quit_cascade: None,
            startup_splash,
            exit_confirm_modal: None,
            options_dialog: None,
            movies_credits_dialog: None,
            campaign_select: None,
            score_screen: None,
            score_shell_state: Default::default(),
            finished_game_count: 0,
            shell_route: Default::default(),
            },
            renderer: crate::app::renderer_state::RendererState {
                gpu,
                batch_renderer,
                combat_light_renderer,
                instance_pool: crate::render::batch::InstanceBufferPool::new(),
                depth_view,
                shell_surface_presenter,
                upscale_pass,
                egui,
                vxl_compute: Some(vxl_compute),
                bit_font,
                vxl_slope_transition_cache: std::cell::RefCell::new(Default::default()),
                retail_screenshot_frame_cache: Default::default(),
            },
            process_assets: crate::app::process_assets::ProcessAssets::from_startup(
                startup_asset_manager,
                startup_csf,
                startup_native_rules,
                startup_rules_projection,
            ),
            audio: startup_audio_runtime,
            persistence: crate::app::persistence::PersistenceState::new(options_profile),
            diag: crate::app::diagnostics::state::DiagnosticsState {
                debug_frame_step_requested: false,
                debug_show_pathgrid: false,
                debug_terrain_cost_speed_type: None,
                debug_show_cell_grid: false,
                debug_show_heightmap: false,
                debug_unit_inspector: false,
                parity_digest_sink: match crate::sim::parity_digest::ParityDigestSink::from_env() {
                    Ok(sink) => {
                        if let Some(sink) = sink.as_ref() {
                            log::info!("parity digest capture -> {}", sink.path().display());
                        }
                        sink
                    }
                    Err(error) => {
                        log::error!("parity digest sink could not be opened: {error}");
                        None
                    }
                },
                dev_overlay_save_name: String::new(),
                frame_timer: crate::app::diagnostics::dev_overlay::FrameTimer::new(),
            },
        };

        // Project all three retained profile gains before any player can start
        // an audible source. The profile keeps native negative values for
        // round-trip; the output owners provide the documented safe clamp.
        apply_startup_audio_profile(&state.persistence.options_profile, &mut state.audio);

        if state.frontend.dev_skirmish_shell_enabled {
            Self::ensure_active_cooperative_shell_selection(&mut state);
        }

        if std::env::var("RA2_QUICKPLAY").is_ok() {
            let skirmish_settings = state.frontend.skirmish_settings.clone();
            let request =
                crate::app::loading::pump::LoadingRequest::generic_map_load("auto", skirmish_settings);
            crate::app::loading::pump::begin_loading(&mut state, request);
        }

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum StartupAudioCall {
        Sound(f64),
        Voice(f64),
        Score(f64),
    }

    #[derive(Default)]
    struct RecordingStartupAudioOperations {
        calls: Vec<StartupAudioCall>,
    }

    impl StartupAudioProfileOperations for RecordingStartupAudioOperations {
        fn set_sound_volume(&mut self, volume: f64) {
            self.calls.push(StartupAudioCall::Sound(volume));
        }

        fn set_voice_volume(&mut self, volume: f64) {
            self.calls.push(StartupAudioCall::Voice(volume));
        }

        fn set_score_volume(&mut self, volume: f64) {
            self.calls.push(StartupAudioCall::Score(volume));
        }
    }

    #[test]
    fn startup_audio_profile_applies_exact_values_in_native_order() {
        let profile = RetailOptionsProfile {
            sound_volume: 0.125,
            voice_volume: 0.5,
            score_volume: 0.875,
            ..Default::default()
        };
        let mut operations = RecordingStartupAudioOperations::default();

        apply_startup_audio_profile(&profile, &mut operations);

        assert_eq!(
            operations.calls,
            vec![
                StartupAudioCall::Sound(0.125),
                StartupAudioCall::Voice(0.5),
                StartupAudioCall::Score(0.875),
            ]
        );
    }

    #[test]
    fn capture_dimensions_override_profile_and_remain_hidden() {
        let capture_dimensions = Some((1024, 768));
        let startup_options = RetailStartupOptions {
            audio_enabled: false,
            screen_width: 320,
            screen_height: 200,
            ..RetailStartupOptions::default()
        };
        let conflicting_ra2md_load = RetailOptionsLoad {
            retained_profile: crate::app::persistence::options_profile::RetailOptionsProfile {
                detail_level: 0,
                unit_action_lines: false,
                tooltips: false,
                screen_width: 640,
                screen_height: 480,
                sound_volume: 0.1,
                voice_volume: 0.2,
                score_volume: 0.3,
                ..Default::default()
            },
            startup_screen: ScreenSize {
                width: 640,
                height: 480,
            },
        };
        let profile_loader_called = std::cell::Cell::new(false);
        let options_load = select_startup_options_load(
            capture_dimensions,
            &startup_options,
            Some(Path::new("C:/operator-profile")),
            |_, _| {
                profile_loader_called.set(true);
                conflicting_ra2md_load
            },
        );

        assert!(!profile_loader_called.get());
        assert_eq!(
            options_load,
            RetailOptionsLoad::without_ra2md(&RetailStartupOptions::default())
        );
        assert_eq!(
            startup_window_projection(options_load.startup_screen, capture_dimensions),
            (1024, 768, false)
        );
        assert_eq!(
            options_load.startup_screen,
            ScreenSize {
                width: 800,
                height: 600,
            }
        );

        let profile = &options_load.retained_profile;
        assert_eq!((profile.screen_width, profile.screen_height), (800, 600));
        assert_eq!(profile.detail_level, 2);
        assert!(profile.unit_action_lines);
        assert!(profile.tooltips);
        assert_eq!(
            (
                profile.sound_volume,
                profile.voice_volume,
                profile.score_volume,
            ),
            (0.7, 0.7, 0.4)
        );

        let options_projection =
            crate::app::persistence::options::in_game_options_from_profile(profile);
        assert_eq!(options_projection.detail_level, 2);
        assert!(options_projection.unit_action_lines);
        assert!(options_projection.tooltips);
    }
}
