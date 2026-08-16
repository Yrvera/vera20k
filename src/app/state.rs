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
    /// App-owned wall-clock outcome-EVA drain. The deterministic accepted
    /// result and SavourDelay target live in serialized `HouseState`.
    pub(crate) scenario_outcome: Option<crate::app::match_runtime::scenario_exit::ScenarioOutcomeVoiceWait>,
    /// Active running-scenario audio teardown. While present the tactical
    /// frame remains visible but simulation is frozen; its destination is
    /// committed only after the retail fade/voice-wait sequence completes.
    pub(crate) scenario_exit: Option<crate::app::match_runtime::scenario_exit::ScenarioExitCascade>,
    /// Game data from rules.ini — needed by combat system for weapon/warhead lookups.
    /// Startup-shell rules loaded at boot for menu presentation; match paths
    /// read the runtime-bound copy via `rules()`.
    pub(crate) frontend_rules: Option<crate::rules::ruleset::RuleSet>,
    /// CSF string table — localized display names for units, buildings, UI text.
    pub(crate) csf: Option<crate::assets::csf_file::CsfFile>,
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
    /// Frontend owner (F12): shell/menu/score/dialog flow state.
    pub(crate) frontend: crate::app::frontend::state::FrontendState,
    /// Seeded empty-map sandbox keeps full map visibility while still locking control.
    pub(crate) sandbox_full_visibility: bool,
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
    /// Effective simulation ticks per second — controls game speed.
    /// Default follows retail/YR skirmish stored game speed 1.
    pub(crate) sim_speed_tps: u32,
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
            && !self.match_presentation.show_save_load_panel
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
        self.frontend.exit_confirm_modal.is_some()
            || self.frontend.options_dialog.is_some()
            || self.frontend.movies_credits_dialog.is_some()
            || self.frontend.campaign_select.is_some()
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
