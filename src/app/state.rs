//! Process-wide application state owned by the app orchestrator.
//!
//! The top-level `AppState` path remains stable while focused ownership groups
//! are introduced incrementally. Platform lifecycle and pacing are the first
//! extracted group; unrelated presentation, input, and match state stay flat.

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
    UnitAtlas, Waypoint, app_render, frontend::startup_splash,
};

mod platform;

pub(crate) use platform::PlatformState;

/// All initialized state. Created in `resumed()` when the window is available.
/// pub(crate) so app_render.rs can access fields.
pub(crate) struct AppState {
    pub(crate) platform: PlatformState,
    pub(crate) gpu: GpuContext,
    pub(crate) batch_renderer: BatchRenderer,
    pub(crate) combat_light_renderer: crate::render::combat_light::CombatLightRenderer,
    pub(crate) combat_lights: crate::app_combat_lights::CombatLightRuntime,
    /// Reusable GPU instance buffers — avoids per-frame GPU buffer allocation.
    pub(crate) instance_pool: crate::render::batch::InstanceBufferPool,
    pub(crate) tile_atlas: Option<TileAtlas>,
    pub(crate) map_basic: BasicSection,
    /// Exact source whose bytes produced the active parsed map.
    pub(crate) loaded_map_source: Option<crate::app_list_maps::LoadedMapSource>,
    /// Deterministic digest of the parsed source map INI. `None` only for
    /// generated/fallback worlds without an authoritative source-map payload.
    pub(crate) loaded_map_hash: Option<u64>,
    pub(crate) terrain_grid: Option<TerrainGrid>,
    pub(crate) sim_runtime: Option<crate::sim::runtime::SimRuntime>,
    /// App-owned diagnostic recording (F10) — never inside the simulation, so
    /// no load/install path can silently drop an unflushed segment.
    pub(crate) match_diagnostics: crate::app::match_diagnostics::MatchDiagnosticsState,
    pub(crate) unit_atlas: Option<UnitAtlas>,
    pub(crate) vxl_slope_transition_cache:
        RefCell<crate::render::unit_slope_transition_cache::VxlSlopeTransitionCache>,
    /// Palette + per-house RGB ramp GPU resources for the voxel sprite shader.
    pub(crate) palette_set: Option<crate::render::palette_textures::PaletteSet>,
    pub(crate) vxl_compute: Option<crate::render::vxl_compute::VxlComputeRenderer>,
    pub(crate) sprite_atlas: Option<SpriteAtlas>,
    pub(crate) overlay_atlas: Option<OverlayAtlas>,
    pub(crate) bridge_atlas: Option<BridgeAtlas>,
    pub(crate) bridge_railing_atlas: Option<BridgeRailingAtlas>,
    /// Overlay entries from map for per-frame instance generation.
    pub(crate) overlays: crate::app_overlay_index::OverlayRenderIndex,
    /// Terrain objects from map for per-frame instance generation.
    pub(crate) terrain_objects: Vec<TerrainObject>,
    pub(crate) waypoints: HashMap<u32, Waypoint>,
    pub(crate) cell_tags: CellTagMap,
    pub(crate) tags: TagMap,
    /// Overlay ID → type name mapping for atlas lookups at render time.
    pub(crate) overlay_names: BTreeMap<u8, String>,
    /// Precomputed average pixel color for each tiberium overlay (id, frame) pair,
    /// extracted from SHP frames for minimap radar display.
    pub(crate) tiberium_radar_colors: HashMap<(u8, u8), [u8; 3]>,
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
    /// GPU depth texture for back-to-front depth ordering. Recreated on window resize.
    pub(crate) depth_view: wgpu::TextureView,
    /// Encoded-byte RGB565 presentation boundary for stock shell/loading surfaces.
    pub(crate) shell_surface_presenter: crate::render::shell_surface_present::ShellSurfacePresenter,
    /// Optional Catmull-Rom bicubic upscale pass (render at lower res, upscale to window).
    pub(crate) upscale_pass: Option<crate::render::upscale_pass::UpscalePass>,
    pub(crate) camera_x: f32,
    pub(crate) camera_y: f32,
    /// Current zoom level for the game viewport. 1.0 = native pixel scale,
    /// >1.0 = zoomed in (world appears larger), <1.0 = zoomed out (see more map).
    /// Animated each frame toward `zoom_target`.
    pub(crate) zoom_level: f32,
    /// Target zoom level — mouse wheel sets this; `zoom_level` eases toward it.
    pub(crate) zoom_target: f32,
    /// World-space anchor point for zoom animation. The camera adjusts each frame
    /// so this world point stays at `zoom_anchor_screen` during the zoom ease.
    pub(crate) zoom_anchor_world: [f32; 2],
    /// Screen-space position of the zoom anchor (cursor position when wheel fired).
    pub(crate) zoom_anchor_screen: [f32; 2],
    /// Mouse edge auto-scroll ramp state (gamemd's CoastLevel and its 16 ms timer).
    pub(crate) edge_scroll: crate::app_camera::EdgeScrollState,
    /// Tactical mouse capture and right-drag pan anchor.
    pub(crate) tactical_mouse: crate::app_camera::TacticalMouseState,
    /// The four camera bookmarks (View1..4 / SetView1..4).
    pub(crate) view_bookmarks: crate::app_camera::ViewBookmarks,
    pub(crate) cursor_x: f32,
    pub(crate) cursor_y: f32,
    pub(crate) keys_held: HashSet<KeyCode>,
    pub(crate) hotkey_bindings: crate::app_hotkeys::HotkeyBindings,
    pub(crate) hotkey_modifiers: ModifiersState,
    /// Hybrid held/tap state for the retail TypeSelect command.
    pub(crate) type_select: crate::app_types::TypeSelectInputState,
    /// One-shot Shift+S request, consumed at the next render submission.
    pub(crate) retail_screenshot_requested: bool,
    /// Previous presented pre-cursor composition, retained for input-time
    /// screenshot parity.
    pub(crate) retail_screenshot_frame_cache: crate::render::screenshot::PresentedFrameCache,
    /// egui integration — input handling + GPU rendering.
    pub(super) egui: EguiIntegration,
    /// Which screen is currently active (MainMenu, Loading, InGame).
    pub(crate) screen: GameScreen,
    /// Available maps from the RA2 directory for menu selection.
    pub(crate) available_maps: Vec<MapMenuEntry>,
    /// Source-ordered map entries projected from scenario records for the experimental shell.
    pub(crate) skirmish_shell_maps: Vec<MapMenuEntry>,
    /// MPModes rows used by the native Choose Map modal.
    pub(crate) skirmish_modes: Vec<crate::skirmish_modes::SkirmishGameMode>,
    /// Scenario records used by the native Choose Map modal.
    pub(crate) skirmish_scenario_records: Vec<crate::map::skirmish_scenarios::SkirmishScenarioRecord>,
    /// Player-configured skirmish settings (map, country, credits, etc.).
    pub(crate) skirmish_settings: SkirmishSettings,
    pub(crate) loading_session: Option<crate::app_loading::LoadingSession>,
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
    pub(crate) offline_skirmish_runtime: crate::app_skirmish_session::OfflineSkirmishRuntime,
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
        Option<crate::app_skirmish_shell_render::SkirmishPreviewTexture>,
    /// Minimap renderer — created at map load time.
    pub(crate) loading_screen_atlas:
        Option<crate::render::loading_screen_chrome::LoadingScreenAtlas>,
    pub(crate) loading_progress: crate::app_loading::LoadingProgressState,
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
        Option<crate::app_main_menu_shell_render::Ra2tsMovieSessionIdentity>,
    pub(crate) main_menu_movie_last_step: Instant,
    pub(crate) main_menu_shell_failed: bool,
    /// Numeric internal-version string used by the bottom-right main-menu line.
    /// Resolution follows the retail 16-byte/CR-only cached contract.
    pub(crate) version_txt: String,
    pub(crate) main_menu_show_single_player_shell: bool,
    pub(crate) main_menu_show_skirmish_setup: bool,
    pub(crate) main_menu_show_native_skirmish_shell: bool,
    pub(crate) skirmish_shell_return_to_single_player_shell: bool,
    /// Active shell first-paint controls-reveal slide (presentation only). gamemd
    /// plays this on the first paint of every shell dialog (menu / single-player /
    /// skirmish); the wave swaps each owner-draw button's SDBTNANM frame index.
    pub(crate) shell_first_paint_slide: Option<crate::app_shell_transition::ShellFrameWave>,
    /// Which shell dialog the first-paint slide last fired for. Drives per-frame
    /// edge detection so the slide (re)starts on entry into each shell and is
    /// cancelled on leaving all of them.
    pub(crate) shell_slide_active_shell: Option<crate::app_shell_transition::ShellSlideKind>,
    /// Monotonic identity for each newly armed exact Main Menu `0xE2` instance.
    pub(crate) shell_slide_generation: u64,
    /// Active graceful quit cascade (music fade → trailing-voice wait → hard stop
    /// → exit). Some only between Exit-confirm OK and window close; freezes shell
    /// input while it runs.
    pub(crate) quit_cascade: Option<crate::app_quit_cascade::QuitCascade>,
    /// App-owned wall-clock outcome-EVA drain. The deterministic accepted
    /// result and SavourDelay target live in serialized `HouseState`.
    pub(crate) scenario_outcome: Option<crate::app_scenario_exit::ScenarioOutcomeVoiceWait>,
    /// Active running-scenario audio teardown. While present the tactical
    /// frame remains visible but simulation is frozen; its destination is
    /// committed only after the retail fade/voice-wait sequence completes.
    pub(crate) scenario_exit: Option<crate::app_scenario_exit::ScenarioExitCascade>,
    pub(crate) minimap: Option<MinimapRenderer>,
    /// True while left-dragging on minimap (camera pan mode).
    pub(crate) minimap_dragging: bool,
    /// Animated radar chrome — plays 33-frame open/close animation when radar gained/lost.
    pub(crate) radar_anim: Option<crate::render::radar_anim::RadarAnimState>,
    /// Requested-versus-resolved atlas identity used to construct `radar_anim`.
    ///
    /// Kept beside the animation so tactical evidence never reconstructs
    /// provenance from the currently selected sidebar theme.
    pub(crate) radar_animation_source:
        Option<crate::render::sidebar_chrome::ResolvedSidebarChromeIdentity>,
    /// Animated power bar — segment-by-segment transition matching original PowerClass.
    pub(crate) power_bar_anim: crate::sidebar::PowerBarAnimState,
    /// Persistent flash + mode state for in-game sidebar gadgets. Ticked from
    /// `app_sidebar_gadgets::update_sidebar_gadget_state` once per sim tick;
    /// read each frame by the sidebar view builder to pick SHP frame indices.
    pub(crate) sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState,
    /// In-game gadget substrate (study §6.1): retained sidebar button list +
    /// capture/focus state + reusable tick output + the mouse-held record.
    pub(crate) in_game_gadgets: crate::app_gadget_input::InGameGadgets,
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
    /// edges by `app_messages::update`.
    pub(crate) message_clock: crate::ui::messages::PauseAwareClock,
    /// Retained immutable sidebar view plus its per-owner animated credit state.
    /// Consumers read the snapshot; explicit transitions rebuild it.
    pub(crate) sidebar_projection: crate::app::sidebar_projection::SidebarProjectionState,
    /// Content insets [left, top, right, bottom] derived from the transparent opening
    /// in radar.shp frame 0. Used to position the minimap inside the chrome housing.
    /// Unscaled pixels — multiply by `ui_scale` at use site.
    pub(crate) radar_content_insets: Option<[u32; 4]>,
    /// Whether the local player currently has operational radar (power-gated).
    pub(crate) has_radar: bool,
    /// Selection overlay renderer — highlights and drag rectangle.
    pub(crate) selection_overlay: Option<SelectionOverlay>,
    /// Authentic SHROUD.SHP sprite-based shroud edge renderer.
    /// GPU ABuffer — screen-resolution brightness texture for per-pixel shroud darkening.
    /// SHROUD.SHP brightness pixels blitted per-cell, then a full-screen multiply pass
    /// darkens the scene.
    pub(crate) shroud_buffer: Option<crate::render::shroud_buffer::ShroudBuffer>,
    /// Packed cameo art used by the custom build sidebar.
    pub(crate) sidebar_cameo_atlas: Option<SidebarCameoAtlas>,
    /// Original side-mix shell art used to skin the custom sidebar.
    pub(crate) sidebar_chrome: Option<SidebarChromeSet>,
    /// Bitmap font atlas used by the custom sidebar text path.
    pub(crate) bit_font: BitFont,
    /// Asset-backed software cursor shown in-game when available.
    pub(crate) software_cursor: Option<app_render::SoftwareCursor>,
    /// Selection drag state — tracks mouse drag for box-select.
    pub(crate) selection_state: SelectionState,
    /// Player-side `g_CurrentObjects` order. Selection commands update this
    /// immediately; the post-sim reconciliation removes lifecycle departures.
    pub(crate) selection_order: Vec<u64>,
    /// A queued selection command has not yet reached the simulation tick.
    pub(crate) selection_order_pending: bool,
    /// Existing selection paths speak by default; held TypeSelect batches
    /// temporarily suppress and restore this latch.
    pub(crate) selection_voice_enabled: bool,
    /// Optional per-tick parity digest stream, opened only when the environment asks.
    ///
    /// Lives here rather than on `Simulation` so the sim tick performs no file I/O and a
    /// capture run stays identical to an uncaptured one.
    pub(crate) parity_digest_sink: Option<crate::sim::parity_digest::ParityDigestSink>,
    /// Game data from rules.ini — needed by combat system for weapon/warhead lookups.
    /// Startup-shell rules loaded at boot for menu presentation; match paths
    /// read the runtime-bound copy via `rules()`.
    pub(crate) frontend_rules: Option<crate::rules::ruleset::RuleSet>,
    /// CSF string table — localized display names for units, buildings, UI text.
    pub(crate) csf: Option<crate::assets::csf_file::CsfFile>,
    /// Owner name → house color index mapping for atlas key lookups.
    pub(crate) house_color_map: HouseColorMap,
    pub(crate) house_roster: HouseRoster,
    /// End-of-match score presentation, decorated from the sim-owned terminal
    /// snapshot and held until the player leaves the screen. `None` for result
    /// screens with no native score analogue (a load failure, a trigger-driven
    /// campaign end), which keep the non-art fallback.
    pub(crate) score_screen: Option<crate::ui::score_shell::ScoreScreenModel>,
    pub(crate) score_shell_state: crate::ui::score_shell::ScoreShellState,
    /// Number of matches finished this session — the score screen's `Game: n`.
    /// gamemd increments the same counter as it tears the scenario down.
    pub(crate) finished_game_count: u32,
    /// Cell (rx, ry) -> high-bridge facts used by the tactical cursor inverse.
    pub(crate) tactical_bridge_inverse_map:
        BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>,
    /// Cell (rx, ry) -> map lighting bundle. Render paths look up compatibility tints per-frame.
    pub(crate) lighting_grid: CellLightGrid,
    /// Complete source list behind the visible grid. Retained so source
    /// transitions can enumerate only old/new affected areas.
    pub(crate) applied_lighting_sources: Vec<crate::map::lighting::PointLight>,
    /// Exact ScenarioClass profile behind the visible grid.
    pub(crate) applied_lighting_profile: Option<crate::map::lighting::LightingProfileUnits>,
    /// Native detail mask behind the visible grid.
    pub(crate) applied_lighting_detail_level: u32,
    /// YR LightSourceClass-style sampled records. The active grid changes only
    /// after the complete pending refresh has gathered.
    pub(crate) pending_lighting_refresh: Option<crate::map::lighting::DeferredCellLightRefresh>,
    /// Complete derived light-view fingerprint applied to `lighting_grid`.
    /// App view-state only — never serialized or hashed.
    pub(crate) last_lighting_view_fingerprint: Option<u64>,
    /// Parsed map [Lighting] config used to rebuild transient app lighting after load.
    pub(crate) map_lighting_config: LightingConfig,
    /// Active map theater name (e.g., DESERT).
    pub(crate) theater_name: String,
    /// Active map theater extension (e.g., des).
    pub(crate) theater_ext: String,
    /// Match elapsed wall time for the retail score screen. App-local and never
    /// serialized, hashed, or read by deterministic simulation.
    pub(crate) scenario_elapsed_clock: crate::app_frame_pacer::ScenarioElapsedClock,
    /// Target/action lines — colored lines from selected units to command destinations.
    pub(crate) target_lines: crate::app_target_lines::TargetLineState,
    /// Config-sourced input delay — copied to each new Simulation instance at game start.
    pub(crate) configured_input_delay_ticks: u64,
    /// Pending order mode for the next right-click command.
    pub(crate) queued_order_mode: app_render::OrderMode,
    /// Control group slots (0-9) storing stable entity ids.
    pub(crate) control_groups: Vec<Vec<u64>>,
    /// Slot and wall-clock instant of the last plain control-group recall, for
    /// the 800 ms double-tap that centres the camera. Wall clock, never sim
    /// state: the original stamps `timeGetTime()` here and only a recall writes
    /// it — assigning with Ctrl+digit never does.
    pub(crate) last_control_group_press: Option<(usize, std::time::Instant)>,
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
    /// Seeded empty-map sandbox keeps full map visibility while still locking control.
    pub(crate) sandbox_full_visibility: bool,
    /// True when in SpawnPick phase — MCV seeding is deferred until the player picks a waypoint.
    pub(crate) spawn_pick_pending: bool,
    /// Mutually-exclusive cursor-on-tactical-map targeting mode (building
    /// placement OR superweapon). Right-click and Esc clear; arming one
    /// kind clears the other.
    pub(crate) targeting_mode: Option<crate::app_types::TargetingMode>,
    /// Current placement preview for the armed building, if any.
    pub(crate) building_placement_preview: Option<BuildingPlacementPreview>,
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
    /// Parked scroll row per sidebar tab, indexed by `app_input::tab_scroll_slot`.
    /// One entry per `SidebarTab` variant.
    pub(crate) sidebar_scroll_rows_parked: [usize; 4],
    /// Process-wide asset ownership (F11): the one retail MIX manager for
    /// the process, leased to the loading pipeline and always returned.
    pub(crate) process_assets: crate::app::process_assets::ProcessAssets,
    /// Background music player (rodio).
    pub(crate) music_player: Option<MusicPlayer>,
    /// Sound effect player (rodio) — plays one-shot SFX (weapons, voices, UI).
    pub(crate) sfx_player: Option<SfxPlayer>,
    /// sound.ini / soundmd.ini registry mapping IDs to .wav filenames.
    pub(crate) sound_registry: SoundRegistry,
    /// audio.idx/bag indices for bag-based sound lookup (voices, EVA).
    /// Searched in order (YR audiomd first, then base audio).
    pub(crate) audio_indices: Vec<crate::assets::audio_bag::AudioIndex>,
    /// The process-start audio decision persisted for later scenario reloads.
    pub(crate) audio_indices_enabled: bool,
    /// EVA announcement registry from eva.ini / evamd.ini.
    /// Maps EVA event names to per-faction audio.bag sound IDs.
    pub(crate) eva_registry: crate::rules::sound_ini::EvaRegistry,
    /// Fire events from the current sim tick — position data for future muzzle
    /// flash rendering and projectile origin computation. Drained each frame.
    pub(crate) pending_fire_effects: Vec<crate::sim::world::SimFireEvent>,
    /// Active garrison muzzle flash animations. Short-lived one-shot entries
    /// spawned when a garrisoned building fires. Ticked each frame, removed on completion.
    pub(crate) garrison_muzzle_flashes: Vec<crate::sim::components::GarrisonMuzzleFlash>,
    /// Active non-garrison weapon muzzle flash animations spawned from weapon `Anim=`.
    /// App-owned presentation state; combat only emits the fire facts.
    pub(crate) weapon_muzzle_flashes: Vec<crate::sim::components::WeaponMuzzleFlash>,
    /// Active render-only projectile sprites spawned from non-instant weapon fire.
    pub(crate) projectile_visuals: Vec<crate::app_fire_effects::ProjectileVisual>,
    /// Active parachute animations, one per descending paradropped infantry.
    /// Polling-based lifecycle: spawned when an entity gains parachute_state
    /// in the sim, removed on landing or death. Render-only; not snapshotted.
    pub(crate) parachute_anims: Vec<crate::sim::components::ParachuteAnim>,
    /// True when the game is paused (an in-scenario modal is open, sim frozen).
    ///
    /// Derived from `in_game_menu` for every player-driven modal; the debug
    /// pause (dev overlay / hotkey) also sets it without opening a menu.
    pub(crate) paused: bool,
    /// In-scenario modal state — the port of gamemd's in-scenario state
    /// variable. Owns the in-game menu, the abort-mission confirmation and the
    /// parent/child relationship with the `0xBBB` Options dialog.
    pub(crate) in_game_menu: crate::ui::pause_menu::InGameMenuState,
    /// When true, advance exactly one sim tick while paused, then clear.
    pub(crate) debug_frame_step_requested: bool,
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
    /// Global elapsed time for looping terrain overlay animations.
    pub(crate) idle_anim_elapsed_ms: u32,
    /// Logic frame on which each building's slot animations were created, by
    /// entity id.
    ///
    /// gamemd gives every building animation slot its own animation object whose
    /// frame timer is based at the frame it was constructed, so two identical
    /// buildings placed at different times run out of phase with each other.
    /// Presentation-only, so it lives here rather than on the entity.
    ///
    /// DRIFT: gamemd serializes each animation object with its own timer, so a
    /// saved game restores the phases it was saved with. This map is not in the
    /// snapshot, so loading re-stamps every surviving structure at the load
    /// frame and the whole base pulses in unison again — the exact symptom the
    /// per-building phase exists to remove. Fires once per save load, and only
    /// unwinds as those buildings are replaced.
    pub(crate) building_anim_phase_base: std::collections::BTreeMap<u64, u64>,
    /// Debug overlay: show terrain cost / pathgrid overlay. Toggle with P / F9.
    pub(crate) debug_show_pathgrid: bool,
    /// SpeedType for terrain cost overlay. None = auto from selected unit (default Track).
    pub(crate) debug_terrain_cost_speed_type: Option<crate::rules::locomotor_type::SpeedType>,
    /// Debug overlay: show cell grid outlines (blue=terrain, yellow=overlay). Toggle with F8.
    pub(crate) debug_show_cell_grid: bool,
    /// Debug overlay: show height map elevation values. Toggle with H.
    pub(crate) debug_show_heightmap: bool,
    /// Show hotkey reference overlay. Toggle with F1.
    pub(crate) show_hotkey_help: bool,
    /// Debug unit inspector — shows event history for selected entities. Toggle with X.
    pub(crate) debug_unit_inspector: bool,
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
    /// Text-field buffer for the dev overlay's "Save As" name input.
    /// Lives in AppState so the field persists across frames while open.
    pub(crate) dev_overlay_save_name: String,
    /// Save repository, cached listing, and last save/load metadata.
    pub(crate) persistence: crate::app::persistence::PersistenceState,
    /// Rolling FPS / frame-time tracker for the dev overlay readout.
    pub(crate) frame_timer: crate::app_dev_overlay::FrameTimer,
    // -- Reusable per-frame scratch buffers (avoid allocation each frame) --
    /// Overlay instance scratch vec — cleared and refilled each frame.
    pub(crate) cached_overlay_instances: Vec<crate::render::batch::SpriteInstance>,
    /// Unit (voxel) instance scratch vec — cleared and refilled each frame.
    pub(crate) cached_unit_instances: Vec<crate::render::batch::SpriteInstance>,
    /// UnitAtlas texture-page tags aligned with `cached_unit_instances`.
    pub(crate) cached_unit_pages: Vec<usize>,
}

/// Drop app-owned scenario-exit runtime after a successful world replacement.
/// Serialized HouseState remains the sole authority for any loaded SavourDelay;
/// wall waits are reconstructed from its expiry latch without replaying EVA.
pub(crate) fn reset_scenario_exit_runtime(state: &mut AppState) {
    state.scenario_outcome = None;
    state.scenario_exit = None;
    if let Some(player) = state.music_player.as_mut() {
        player.cancel_scenario_theme_request();
        player.set_output_scale(1.0);
    }
    if let Some(player) = state.sfx_player.as_mut() {
        player.set_output_scale(1.0);
    }
}

impl AppState {
    /// Effective render target width — intermediate texture when upscaling, else window.
    pub(crate) fn render_width(&self) -> u32 {
        self.upscale_pass
            .as_ref()
            .map_or(self.gpu.config.width, |u| u.src_width())
    }

    /// Effective render target height — intermediate texture when upscaling, else window.
    pub(crate) fn render_height(&self) -> u32 {
        self.upscale_pass
            .as_ref()
            .map_or(self.gpu.config.height, |u| u.src_height())
    }

    /// Whether the software cursor (mouse.shp) should be active this frame.
    /// Returns false when an egui interactive panel is open so the OS cursor shows.
    pub(crate) fn use_software_cursor(&self) -> bool {
        self.software_cursor.is_some()
            && !self.paused
            && !self.show_save_load_panel
            && !self.main_menu_dialog_open()
    }

    /// Capture-only observation of the exact font and scale inputs consumed by
    /// the most recently completed egui pass.
    pub(crate) fn capture_egui_observation(
        &self,
    ) -> crate::render::egui_integration::EguiCaptureObservation<'_> {
        self.egui.capture_observation(&self.platform.window)
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
        self.targeting_mode
            .as_ref()
            .and_then(crate::app_types::TargetingMode::as_building_placement)
    }

    /// Return the SW section name if the targeting mode is set to
    /// `SuperWeapon`, else `None`.
    pub(crate) fn armed_super_weapon_type(&self) -> Option<&str> {
        self.targeting_mode
            .as_ref()
            .and_then(crate::app_types::TargetingMode::as_super_weapon)
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
