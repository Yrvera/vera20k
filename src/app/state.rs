//! Process-wide application state owned by the app orchestrator.
//!
//! The top-level `AppState` path remains stable while focused ownership groups
//! are introduced incrementally. Platform lifecycle and pacing are the first
//! extracted group; unrelated presentation, input, and match state stay flat.

use super::presentation::render;
use super::{
    BTreeMap, BasicSection, BatchRenderer, BitFont,
    BridgeAtlas, BridgeRailingAtlas, BuildingPlacementPreview, CellLightGrid, CellTagMap,
    EguiIntegration, GameConfig, GameScreen, GpuContext, HashMap, HashSet, HouseColorMap,
    HouseRoster, Instant, KeyCode, LightingConfig, MapMenuEntry,
    MinimapRenderer, ModifiersState, MusicPlayer, OverlayAtlas, OverlayTypeRegistry,
    RandomMapGenerationJob, RandomMapGenerationRetention, RefCell, ResolvedTerrainGrid,
    SelectionOverlay, SelectionState, SfxPlayer, SidebarCameoAtlas,
    SidebarChromeLayoutSpec, SidebarChromeSet, SidebarTab, SkirmishSettings,
    SoundRegistry, SpriteAtlas, TagMap, TerrainGrid, TerrainObject, TileAtlas,
    UnitAtlas, Waypoint, frontend::startup_splash,
};

mod platform;

pub(crate) use platform::PlatformState;

/// All initialized state. Created in `resumed()` when the window is available.
/// pub(crate) so app_render.rs can access fields.
pub(crate) struct AppState {
    pub(crate) platform: PlatformState,
    /// Process-wide renderer owner (F12): GPU context, batch renderer,
    /// pools, passes, egui, fonts, and rendering caches.
    pub(crate) renderer: crate::app::renderer_state::RendererState,
    /// Match input owner (F12): camera, zoom, cursor, keys, hotkeys.
    pub(crate) input: crate::app::input::state::MatchInputState,
    /// Match presentation owner (F12), part 1: per-match atlases + cursor.
    pub(crate) match_presentation: crate::app::presentation::state::MatchPresentationState,
    pub(crate) map_basic: BasicSection,
    /// Exact source whose bytes produced the active parsed map.
    pub(crate) loaded_map_source: Option<crate::app::frontend::list_maps::LoadedMapSource>,
    /// Deterministic digest of the parsed source map INI. `None` only for
    /// generated/fallback worlds without an authoritative source-map payload.
    pub(crate) loaded_map_hash: Option<u64>,
    pub(crate) sim_runtime: Option<crate::sim::runtime::SimRuntime>,
    /// App-owned diagnostic recording (F10) — never inside the simulation, so
    /// no load/install path can silently drop an unflushed segment.
    pub(crate) match_diagnostics: crate::app::match_diagnostics::MatchDiagnosticsState,
    /// Shell-retained overlay registry for the random-map preview: keeps the
    /// last-loaded match registry across scenario exit, matching the pre-F07
    /// persistence of the old app field. Match paths read the runtime-bound
    /// copy via `overlay_registry()`.
    pub(crate) shell_preview_overlay_registry: Option<OverlayTypeRegistry>,
    /// Loaded GameConfig — missing config.toml falls back to the executable root;
    /// None only when config loading or executable-root discovery fails.
    /// Read at render time for cosmetic toggles (extra_animations) and other
    /// per-session user preferences. Set in AppState::new() from the existing
    /// GameConfig::load() call; not mutated afterwards.
    pub(crate) game_config: Option<GameConfig>,
    /// Which screen is currently active (MainMenu, Loading, InGame).
    pub(crate) screen: GameScreen,
    /// Available maps from the RA2 directory for menu selection.
    pub(crate) available_maps: Vec<MapMenuEntry>,
    /// Scenario records + their projected shell map entries (F11): one owner,
    /// projection re-derived on every mutation so indices cannot drift.
    pub(crate) scenario_catalog: crate::app::scenario_catalog::ScenarioCatalog,
    /// MPModes rows used by the native Choose Map modal.
    pub(crate) skirmish_modes: Vec<crate::skirmish_modes::SkirmishGameMode>,
    /// Player-configured skirmish settings (map, country, credits, etc.).
    pub(crate) skirmish_settings: SkirmishSettings,
    pub(crate) loading_session: Option<crate::app::loading::pump::LoadingSession>,
    /// Process-persistent terrain-load cache. Scenario teardown, failed loads,
    /// reseeds, and save transitions must not clear it.
    pub(crate) tile_variant_selector_cache:
        crate::map::tile_variant_selector::TileVariantSelectorCache,
    /// Process-owned front-end Main stream. RMG dialog actions and the first
    /// preview selector reach share this cursor; accepted matches reseed their
    /// own Main stream instead of inheriting it.
    pub(crate) frontend_main_rng: crate::sim::rng::SimRng,
    /// Process-lifetime monotonic identity source; zero is permanently reserved.
    pub(crate) next_match_correlation: u64,
    /// Correlation owned by the currently loading accepted attempt.
    pub(crate) active_loading_correlation: Option<crate::match_bootstrap::MatchCorrelationId>,
    /// Accepted startup authority retained after successful installation.
    pub(crate) loaded_startup: Option<crate::match_bootstrap::PreparedMatchStartup>,
    /// Immutable pre-first-tick evidence for the loaded accepted startup.
    pub(crate) rust_l0_receipt: Option<crate::match_bootstrap::RustL0Receipt>,
    /// Opt-in research shell path. Defaults off so the egui Skirmish setup is visible.
    pub(crate) dev_skirmish_shell_enabled: bool,
    pub(crate) skirmish_shell_state: crate::ui::skirmish_shell::SkirmishShellState,
    /// Process-lifetime offline shell snapshot, Scenario cursor, and
    /// Cooperative progress authority.
    pub(crate) offline_skirmish_runtime: crate::app::frontend::skirmish_session::OfflineSkirmishRuntime,
    /// Last owner-draw Skirmish button state observed by the native render path.
    /// Used for the retail GenericClick paint-transition sound.
    pub(crate) skirmish_shell_last_painted_pressed_button:
        Option<crate::ui::skirmish_shell::OwnerDrawButton>,
    pub(crate) skirmish_shell_chrome:
        Option<crate::render::skirmish_shell_chrome::SkirmishShellChromeAtlas>,
    /// Generation running on a worker, if any. Generating a map takes long
    /// enough to freeze the window if done inline, which also means the
    /// dialog's "Working / Please Wait" never gets a frame to appear in.
    pub(crate) random_map_generation: Option<RandomMapGenerationJob>,
    /// Exact setup-generated map retained through OK until loading owns it.
    pub(crate) random_map_retention: RandomMapGenerationRetention,
    pub(crate) skirmish_preview_texture:
        Option<crate::app::frontend::skirmish_shell_render::SkirmishPreviewTexture>,
    /// Minimap renderer — created at map load time.
    pub(crate) loading_screen_atlas:
        Option<crate::render::loading_screen_chrome::LoadingScreenAtlas>,
    pub(crate) loading_progress: crate::app::loading::pump::LoadingProgressState,
    pub(crate) main_menu_shell_state: crate::ui::main_menu_shell::MainMenuShellState,
    pub(crate) single_player_shell_state: crate::ui::single_player_shell::SinglePlayerShellState,
    /// Shared descriptor-driven input authority for the front-end shell dialogs
    /// (0xE2 main menu, 0x100 single player). Owns hit-test + press-must-match;
    /// its press/hover state is mirrored back into the per-shell structs above for
    /// the render path (substrate Slice 2).
    pub(crate) shell_controller: crate::ui::shell::controller::DialogController,
    pub(crate) main_menu_shell_chrome:
        Option<crate::render::main_menu_shell_chrome::MainMenuShellChromeAtlas>,
    pub(crate) main_menu_movie: Option<crate::render::bink_movie::BinkMovieSurface>,
    pub(crate) main_menu_movie_identity:
        Option<crate::app::frontend::main_menu_shell_render::Ra2tsMovieSessionIdentity>,
    pub(crate) main_menu_movie_last_step: Instant,
    pub(crate) main_menu_shell_failed: bool,
    /// Numeric internal-version string used by the bottom-right main-menu line.
    /// Resolution follows the retail 16-byte/CR-only cached contract.
    pub(crate) version_txt: String,
    /// Which shell surface owns the MainMenu screen (F11): structural
    /// exclusivity replaces the old boolean triple.
    pub(crate) shell_route: crate::app::shell_route::ShellRoute,
    /// Active shell first-paint controls-reveal slide (presentation only). gamemd
    /// plays this on the first paint of every shell dialog (menu / single-player /
    /// skirmish); the wave swaps each owner-draw button's SDBTNANM frame index.
    pub(crate) shell_first_paint_slide: Option<crate::app::frontend::shell_transition::ShellFrameWave>,
    /// Which shell dialog the first-paint slide last fired for. Drives per-frame
    /// edge detection so the slide (re)starts on entry into each shell and is
    /// cancelled on leaving all of them.
    pub(crate) shell_slide_active_shell: Option<crate::app::frontend::shell_transition::ShellSlideKind>,
    /// Monotonic identity for each newly armed exact Main Menu `0xE2` instance.
    pub(crate) shell_slide_generation: u64,
    /// Active graceful quit cascade (music fade → trailing-voice wait → hard stop
    /// → exit). Some only between Exit-confirm OK and window close; freezes shell
    /// input while it runs.
    pub(crate) quit_cascade: Option<crate::app::frontend::quit_cascade::QuitCascade>,
    /// App-owned wall-clock outcome-EVA drain. The deterministic accepted
    /// result and SavourDelay target live in serialized `HouseState`.
    pub(crate) scenario_outcome: Option<crate::app::match_runtime::scenario_exit::ScenarioOutcomeVoiceWait>,
    /// Active running-scenario audio teardown. While present the tactical
    /// frame remains visible but simulation is frozen; its destination is
    /// committed only after the retail fade/voice-wait sequence completes.
    pub(crate) scenario_exit: Option<crate::app::match_runtime::scenario_exit::ScenarioExitCascade>,
    /// Animated power bar — segment-by-segment transition matching original PowerClass.
    pub(crate) power_bar_anim: crate::sidebar::PowerBarAnimState,
    /// Persistent flash + mode state for in-game sidebar gadgets. Ticked from
    /// `sidebar_gadgets::update_sidebar_gadget_state` once per sim tick;
    /// read each frame by the sidebar view builder to pick SHP frame indices.
    pub(crate) sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState,
    /// In-game gadget substrate (study §6.1): retained sidebar button list +
    /// capture/focus state + reusable tick output + the mouse-held record.
    pub(crate) in_game_gadgets: crate::app::input::gadget_input::InGameGadgets,
    /// Shared tooltip service (study S1) — the model is clock-injected; only
    /// `app_tooltips` reads the wall clock.
    pub(crate) tooltips: crate::ui::tooltips::TooltipService,
    /// Epoch for the tooltip/message wall-clock (`now_ms` = elapsed since
    /// app construction).
    pub(crate) tooltip_epoch: Instant,
    /// In-game chat/system message surface (study §3.1) — re-anchored to the
    /// tactical viewport per frame by `app_messages`.
    pub(crate) message_list: crate::ui::messages::MessageList,
    /// Pause-adjusted clock for message deadlines (contract §4.2 step 8 /
    /// §4.3: the native composite timer freezes during pause). Fed pause
    /// edges by `messages::update`.
    pub(crate) message_clock: crate::ui::messages::PauseAwareClock,
    /// Retained immutable sidebar view plus its per-owner animated credit state.
    /// Consumers read the snapshot; explicit transitions rebuild it.
    pub(crate) sidebar_projection: crate::app::sidebar_projection::SidebarProjectionState,
    /// Game data from rules.ini — needed by combat system for weapon/warhead lookups.
    /// Startup-shell rules loaded at boot for menu presentation; match paths
    /// read the runtime-bound copy via `rules()`.
    pub(crate) frontend_rules: Option<crate::rules::ruleset::RuleSet>,
    /// CSF string table — localized display names for units, buildings, UI text.
    pub(crate) csf: Option<crate::assets::csf_file::CsfFile>,
    /// End-of-match score presentation, decorated from the sim-owned terminal
    /// snapshot and held until the player leaves the screen. `None` for result
    /// screens with no native score analogue (a load failure, a trigger-driven
    /// campaign end), which keep the non-art fallback.
    pub(crate) score_screen: Option<crate::ui::score_shell::ScoreScreenModel>,
    pub(crate) score_shell_state: crate::ui::score_shell::ScoreShellState,
    /// Number of matches finished this session — the score screen's `Game: n`.
    /// gamemd increments the same counter as it tears the scenario down.
    pub(crate) finished_game_count: u32,
    /// Match elapsed wall time for the retail score screen. App-local and never
    /// serialized, hashed, or read by deterministic simulation.
    pub(crate) scenario_elapsed_clock: crate::app::match_runtime::frame_pacer::ScenarioElapsedClock,
    /// Config-sourced input delay — copied to each new Simulation instance at game start.
    pub(crate) configured_input_delay_ticks: u64,
    /// Match-scoped local player identity, pinned ONCE at match launch
    /// (skirmish session / spawn-pick) and never rewritten mid-match. All
    /// command/HUD owner resolution reads this first — selection must never
    /// repoint the local player (lockstep: each client issues commands as its
    /// fixed house). `None` only in dev/sandbox flows with no launch identity,
    /// where the legacy heuristic + debug override below take over.
    pub(crate) local_player_owner: Option<String>,
    /// Explicit local owner preference for HUD/commands (set by debug actions).
    /// Only consulted when `local_player_owner` is `None` (sandbox/dev flows).
    pub(crate) local_owner_override: Option<String>,
    /// Per-match audio owner (F11): sound event queue + EVA latches; resets
    /// on every match install and on leaving a match for the shell.
    pub(crate) match_audio: crate::app::match_audio::MatchAudioState,
    /// Process diagnostics owner (F12): debug toggles, frame stepper,
    /// parity digest sink, dev-overlay bookkeeping.
    pub(crate) diag: crate::app::diagnostics::state::DiagnosticsState,
    /// Seeded empty-map sandbox keeps full map visibility while still locking control.
    pub(crate) sandbox_full_visibility: bool,
    /// Active tab for the custom in-game sidebar.
    pub(crate) active_sidebar_tab: SidebarTab,
    /// Optional local override for chrome positioning loaded from sidebar_layout.ron.
    /// This is the SCALED version — multiply base by ui_scale at init/resize.
    pub(crate) sidebar_layout_spec: SidebarChromeLayoutSpec,
    /// Unscaled base layout spec (from file or stock). Kept for re-scaling on resize.
    pub(crate) sidebar_layout_spec_base: SidebarChromeLayoutSpec,
    /// Integer UI scale factor (1, 2, or 3). Auto-detected from screen height.
    /// Sidebar, minimap, and other UI elements are scaled by this factor.
    pub(crate) ui_scale: f32,
    /// Scroll offset for the current sidebar tab's item list.
    ///
    /// gamemd's sidebar keeps this row per build strip, not one shared value —
    /// its scroll command indexes the strip by column. This holds the live row
    /// for the active tab; the parked rows for the other tabs live in
    /// `sidebar_scroll_rows_parked` and swap in and out on a tab change, which
    /// keeps every consumer reading one field while the position stops bleeding
    /// across tabs.
    pub(crate) sidebar_scroll_rows: usize,
    /// Parked scroll row per sidebar tab, indexed by `input::dispatch::tab_scroll_slot`.
    /// One entry per `SidebarTab` variant.
    pub(crate) sidebar_scroll_rows_parked: [usize; 4],
    /// Process-wide asset ownership (F11): the one retail MIX manager for
    /// the process, leased to the loading pipeline and always returned.
    pub(crate) process_assets: crate::app::process_assets::ProcessAssets,
    /// Process-wide audio owner (F12): players and registries.
    pub(crate) audio: crate::app::audio_runtime::AppAudioRuntime,
    /// True when the game is paused (an in-scenario modal is open, sim frozen).
    ///
    /// Derived from `in_game_menu` for every player-driven modal; the debug
    /// pause (dev overlay / hotkey) also sets it without opening a menu.
    pub(crate) paused: bool,
    /// In-scenario modal state — the port of gamemd's in-scenario state
    /// variable. Owns the in-game menu, the abort-mission confirmation and the
    /// parent/child relationship with the `0xBBB` Options dialog.
    pub(crate) in_game_menu: crate::ui::pause_menu::InGameMenuState,
    /// Effective simulation ticks per second — controls game speed.
    /// Default follows retail/YR skirmish stored game speed 1.
    pub(crate) sim_speed_tps: u32,
    /// Client-side in-game Options (0xBBB) state: the six [Options] values plus
    /// transient interaction flags. `game_speed` mirrors the launched sim and
    /// queues an authoritative transition on close; `sim_speed_tps` is its local
    /// presentation readout. App/ui-level.
    pub(crate) in_game_options: crate::ui::shell::in_game_options_state::InGameOptionsState,
    /// Laid-out 0xBBB anchor cached by the overlay render pass each frame it draws,
    /// so the paused mouse handler hit-tests the exact rects that were rendered
    /// (the sidebar-anchored button Y is render-derived; see KD-6). None until the
    /// overlay first renders.
    pub(crate) in_game_options_anchor: Option<crate::ui::shell::layout::InGameOptionsAnchor>,
    /// Retail process-start splash, held until its post-present deadline.
    pub(crate) startup_splash: Option<startup_splash::StartupSplashPresentation>,
    /// Show hotkey reference overlay. Toggle with F1.
    pub(crate) show_hotkey_help: bool,
    /// Save/load panel visible. Toggle with F5.
    pub(crate) show_save_load_panel: bool,
    /// Exit-Game confirm message box, open while the player is being asked to
    /// confirm quitting. The app only exits on confirm, never on the first
    /// Exit click.
    pub(crate) exit_confirm_modal: Option<crate::ui::main_menu_dialogs::ExitConfirmModalState>,
    /// Options launcher dialog (open-level shell; real widgets not decoded).
    pub(crate) options_dialog: Option<crate::ui::main_menu_dialogs::OptionsDialogState>,
    /// Movies & Credits sub-panel (open-level shell; playback not implemented).
    pub(crate) movies_credits_dialog:
        Option<crate::ui::main_menu_dialogs::MoviesCreditsDialogState>,
    /// Campaign selector dialog (Single Player -> New Campaign; launch mapping
    /// not decoded).
    pub(crate) campaign_select: Option<crate::ui::main_menu_dialogs::CampaignSelectState>,
    /// Save repository, cached listing, and last save/load metadata.
    pub(crate) persistence: crate::app::persistence::PersistenceState,
}

/// Drop app-owned scenario-exit runtime after a successful world replacement.
/// Serialized HouseState remains the sole authority for any loaded SavourDelay;
/// wall waits are reconstructed from its expiry latch without replaying EVA.
pub(crate) fn reset_scenario_exit_runtime(state: &mut AppState) {
    state.scenario_outcome = None;
    state.scenario_exit = None;
    if let Some(player) = state.audio.music_player.as_mut() {
        player.cancel_scenario_theme_request();
        player.set_output_scale(1.0);
    }
    if let Some(player) = state.audio.sfx_player.as_mut() {
        player.set_output_scale(1.0);
    }
}

impl AppState {
    /// Effective render target width — intermediate texture when upscaling, else window.
    pub(crate) fn render_width(&self) -> u32 {
        self.renderer.upscale_pass
            .as_ref()
            .map_or(self.renderer.gpu.config.width, |u| u.src_width())
    }

    /// Effective render target height — intermediate texture when upscaling, else window.
    pub(crate) fn render_height(&self) -> u32 {
        self.renderer.upscale_pass
            .as_ref()
            .map_or(self.renderer.gpu.config.height, |u| u.src_height())
    }

    /// Whether the software cursor (mouse.shp) should be active this frame.
    /// Returns false when an egui interactive panel is open so the OS cursor shows.
    pub(crate) fn use_software_cursor(&self) -> bool {
        self.match_presentation.software_cursor.is_some()
            && !self.paused
            && !self.show_save_load_panel
            && !self.main_menu_dialog_open()
    }

    /// Capture-only observation of the exact font and scale inputs consumed by
    /// the most recently completed egui pass.
    pub(crate) fn capture_egui_observation(
        &self,
    ) -> crate::render::egui_integration::EguiCaptureObservation<'_> {
        self.renderer.egui.capture_observation(&self.platform.window)
    }

    /// Whether any main-menu modal dialog (exit confirm, options, movies,
    /// campaign select) is currently open.
    pub(crate) fn main_menu_dialog_open(&self) -> bool {
        self.exit_confirm_modal.is_some()
            || self.options_dialog.is_some()
            || self.movies_credits_dialog.is_some()
            || self.campaign_select.is_some()
    }

    /// Return the building-placement section name if the targeting mode
    /// is set to `BuildingPlacement`, else `None`.
    pub(crate) fn armed_building_type(&self) -> Option<&str> {
        self.input.targeting_mode
            .as_ref()
            .and_then(crate::app::types::TargetingMode::as_building_placement)
    }

    /// Return the SW section name if the targeting mode is set to
    /// `SuperWeapon`, else `None`.
    pub(crate) fn armed_super_weapon_type(&self) -> Option<&str> {
        self.input.targeting_mode
            .as_ref()
            .and_then(crate::app::types::TargetingMode::as_super_weapon)
    }
}

impl AppState {
    /// Immutable view of the running simulation (F10): the read boundary
    /// presentation cones consume. `None` outside a match. Sites that also
    /// hold `&mut` app fields keep the `sim_runtime` field chain and call
    /// `rt.view()` directly for split borrows.
    pub(crate) fn sim_view(&self) -> Option<crate::sim::runtime::SimView<'_>> {
        self.sim_runtime.as_ref().map(|rt| rt.view())
    }

    /// Fixed per-cell terrain heights for the active match, or the empty map
    /// when no runtime exists — matching the pre-F07 always-present field.
    pub(crate) fn height_map(&self) -> &BTreeMap<(u16, u16), u8> {
        static EMPTY: std::sync::OnceLock<BTreeMap<(u16, u16), u8>> = std::sync::OnceLock::new();
        self.sim_runtime
            .as_ref()
            .map(|rt| &rt.resources.height_map)
            .unwrap_or_else(|| EMPTY.get_or_init(BTreeMap::new))
    }

    /// Bridge-deck heights for the active match (see `height_map`).
    pub(crate) fn bridge_height_map(&self) -> &BTreeMap<(u16, u16), u8> {
        static EMPTY: std::sync::OnceLock<BTreeMap<(u16, u16), u8>> = std::sync::OnceLock::new();
        self.sim_runtime
            .as_ref()
            .map(|rt| &rt.resources.bridge_height_map)
            .unwrap_or_else(|| EMPTY.get_or_init(BTreeMap::new))
    }
}

impl AppState {
    /// The overlay registry: runtime-bound during a match, shell-retained
    /// (last loaded) otherwise — exactly the old field's lifecycle.
    pub(crate) fn overlay_registry(&self) -> Option<&OverlayTypeRegistry> {
        self.sim_runtime
            .as_ref()
            .map(|rt| &rt.resources.overlay_registry)
            .or(self.shell_preview_overlay_registry.as_ref())
    }
}

impl AppState {
    /// The active rules: runtime-bound during a match, startup-shell rules
    /// otherwise. Matches the old field's Option shape at every consumer.
    pub(crate) fn rules(&self) -> Option<&crate::rules::ruleset::RuleSet> {
        self.sim_runtime
            .as_ref()
            .map(|rt| &rt.resources.rules)
            .or(self.frontend_rules.as_ref())
    }
}

impl AppState {
    /// The immutable base resolved-terrain template for the active match
    /// (static rendering + restore); never the live sim grid.
    pub(crate) fn terrain_template(&self) -> Option<&ResolvedTerrainGrid> {
        self.sim_runtime
            .as_ref()
            .and_then(|rt| rt.resources.terrain_template.as_ref())
    }
}
