//! Application orchestrator — ties all subsystems together.
//! Implements winit's ApplicationHandler. GPU init deferred to resumed().
//! Helpers: app_init.rs (loading), app_render.rs (rendering).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app_init::MapMenuEntry;
use crate::app_input;
use crate::app_list_maps;
use crate::app_render;
use crate::app_sim_tick;
use crate::app_transitions;
use crate::assets::asset_manager::AssetManager;
use crate::audio::events::SoundEventQueue;
use crate::audio::music::MusicPlayer;
use crate::audio::sfx::SfxPlayer;
use crate::map::actions::ActionMap;
use crate::map::basic::BasicSection;
use crate::map::cell_tags::CellTagMap;
use crate::map::events::EventMap;
use crate::map::houses::{HouseColorMap, HouseRoster};
use crate::map::lighting::{CellLightGrid, LightingConfig};
use crate::map::overlay::{OverlayEntry, TerrainObject};
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::tags::TagMap;
use crate::map::terrain::TerrainGrid;
use crate::map::trigger_graph::TriggerGraph;
use crate::map::triggers::TriggerMap;
use crate::map::waypoints::Waypoint;
use crate::render::batch::BatchRenderer;
use crate::render::bit_font::BitFont;
use crate::render::bridge_atlas::BridgeAtlas;
use crate::render::bridge_railing_atlas::BridgeRailingAtlas;
use crate::render::egui_integration::EguiIntegration;
use crate::render::gpu::GpuContext;
use crate::render::minimap::MinimapRenderer;
use crate::render::overlay_atlas::OverlayAtlas;
use crate::render::selection_overlay::SelectionOverlay;
use crate::render::sidebar_cameo_atlas::SidebarCameoAtlas;
use crate::render::sidebar_chrome::SidebarChromeSet;
use crate::render::sprite_atlas::SpriteAtlas;
use crate::render::tile_atlas::TileAtlas;
use crate::render::unit_atlas::UnitAtlas;
use crate::rules::art_data::ArtRegistry;
use crate::rules::infantry_sequence::InfantrySequenceRegistry;
use crate::rules::sound_ini::SoundRegistry;
use crate::sidebar::{SidebarChromeLayoutSpec, SidebarTab};
use crate::sim::animation::SequenceSet;
use crate::sim::pathfinding::PathGrid;
use crate::sim::production::BuildingPlacementPreview;
use crate::sim::selection::SelectionState;
use crate::sim::world::Simulation;
use crate::ui::game_screen::GameScreen;
use crate::ui::main_menu::{self, SkirmishSettings};
use crate::ui::shell::controller::ShellKey;
use crate::ui::skirmish_shell::{SavedSeedBrowserState, SavedSeedMode};
use crate::util::config::GameConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellFramePreludeStep {
    MaintainIntro,
    ObserveEntry,
}

const MAIN_MENU_SHELL_PRELUDE: &[ShellFramePreludeStep] = &[
    ShellFramePreludeStep::MaintainIntro,
    ShellFramePreludeStep::ObserveEntry,
];

/// The random-map seed file the `.SED` launch branch recognises. Written into
/// the RA2 directory so `map_load` finds it where the original puts it.
const RANDMAP_SED_FILE: &str = "RandMap.Sed";
/// Description the setup dialog stamps onto a randomized configuration; it also
/// becomes the sentinel row's displayed name.
const RANDOM_MAP_DESCRIPTION_KEY: &str = "TXT_RANDOM_MAP_DESCRIPTION";
const RANDOM_MAP_DESCRIPTION_FALLBACK: &str = "Random Map";
/// The players slider is the last of the setup dialog's six option rows, and the
/// dialog gives it a range of 2..8 with a step of one.
const SETUP_PLAYERS_ROW: usize = 5;
const SETUP_PLAYERS_MIN: i32 = 2;
const SETUP_PLAYERS_MAX: i32 = 8;
const SETUP_PLAYERS_STEP: i32 = 1;
/// Matches the rules default the map-load path falls back to; the preview only
/// needs it because terrain resolution takes it, not because cliffs affect the
/// image.
const RANDOM_MAP_PREVIEW_CLIFF_BACK_IMPASSABILITY: u8 = 2;
/// Where the generated preview is written. The chooser's sentinel row reads this
/// back, so writing it is what makes the random-map thumbnail appear there.
const RANDMAP_PREVIEW_FILE: &str = "RandMap.img";

const DEV_SKIRMISH_SHELL_ENV: &str = "RA2_DEV_SKIRMISH_SHELL";
const SHELL_WINDOW_WIDTH: u32 = 800;
const SHELL_WINDOW_HEIGHT: u32 = 600;

/// A random-map generation handed to a worker thread.
///
/// The worker only *generates*; colouring a preview needs the terrain resolver
/// and therefore the asset manager, so the main thread rasterises everything the
/// worker hands it. What matters is that the expensive part is off the UI
/// thread — that is what lets frames render while it runs.
pub(crate) struct RandomMapGenerationJob {
    receiver: std::sync::mpsc::Receiver<RandomMapUpdate>,
    /// Kept back from the worker because rasterising needs it: the resolver
    /// reads theater data to decide each cell's final tile.
    theater: Box<crate::map::theater::TheaterData>,
    terrain_rules: Box<crate::rules::terrain_rules::TerrainRules>,
    /// Set when OK started this generation. Accept cannot run until the map
    /// exists, so it is deferred to whoever collects the result.
    accept_on_finish: bool,
}

/// What the generator worker sends back as it goes.
enum RandomMapUpdate {
    /// The map at one of the boundaries the original redraws its preview at.
    Progress(Box<crate::map::rmg::build::GenerationSnapshot>),
    /// The finished map.
    Finished(Box<crate::map::rmg::GeneratedMap>),
}

/// Whether the original redraws its preview at this generation boundary.
///
/// It draws eight times while generating, reporting 55, 60, 70, 80, 85, 90 and
/// 95 percent on the seven after the first. Two of those pairs have no
/// generation between them at all — only a progress-report helper runs — so the
/// 60 and 85 redraws reproduce the image already on screen and are dropped here:
/// eight calls, six distinct pictures.
///
/// The percentages are the anchor the boundaries below were chosen from; they
/// have no home in the port yet, because the dialog's progress bar is still
/// drawn empty.
fn draws_preview(point: crate::map::rmg::build::GenerationPoint) -> bool {
    use crate::map::rmg::Stage;
    use crate::map::rmg::build::GenerationPoint;
    matches!(
        point,
        // Clears the box before any terrain exists.
        GenerationPoint::Initial
            // 55 (and again at 60): the water is in.
            | GenerationPoint::After(Stage::WaterFinalize)
            // 70: regions, island passes and the green spread.
            | GenerationPoint::After(Stage::RecalcAfterTerrain)
            // 80 (and again at 85): starts, tech buildings and tiberium.
            | GenerationPoint::After(Stage::RecalcAfterTiberium)
            // 90: the hills.
            | GenerationPoint::After(Stage::Hills)
            // 95: LAT patches, trees and rocks.
            | GenerationPoint::After(Stage::Rocks)
    )
}

/// All initialized state. Created in `resumed()` when the window is available.
/// pub(crate) so app_render.rs can access fields.
pub(crate) struct AppState {
    pub(crate) window: Arc<Window>,
    pub(crate) gpu: GpuContext,
    pub(crate) batch_renderer: BatchRenderer,
    /// Reusable GPU instance buffers — avoids per-frame GPU buffer allocation.
    pub(crate) instance_pool: crate::render::batch::InstanceBufferPool,
    pub(crate) tile_atlas: Option<TileAtlas>,
    pub(crate) map_basic: BasicSection,
    /// Exact source whose bytes produced the active parsed map.
    pub(crate) loaded_map_source: Option<crate::app_list_maps::LoadedMapSource>,
    pub(crate) terrain_grid: Option<TerrainGrid>,
    pub(crate) resolved_terrain: Option<ResolvedTerrainGrid>,
    pub(crate) simulation: Option<Simulation>,
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
    pub(crate) overlays: Vec<OverlayEntry>,
    /// Terrain objects from map for per-frame instance generation.
    pub(crate) terrain_objects: Vec<TerrainObject>,
    pub(crate) waypoints: HashMap<u32, Waypoint>,
    pub(crate) cell_tags: CellTagMap,
    pub(crate) tags: TagMap,
    pub(crate) triggers: TriggerMap,
    pub(crate) events: EventMap,
    pub(crate) actions: ActionMap,
    pub(crate) trigger_graph: TriggerGraph,
    /// Overlay ID → type name mapping for atlas lookups at render time.
    pub(crate) overlay_names: BTreeMap<u8, String>,
    /// Precomputed average pixel color for each tiberium overlay (id, frame) pair,
    /// extracted from SHP frames for minimap radar display.
    pub(crate) tiberium_radar_colors: HashMap<(u8, u8), [u8; 3]>,
    /// Registry of overlay types from rules.ini — needed at runtime to look up
    /// overlay_id by name when a wall is placed via production.
    pub(crate) overlay_registry: Option<OverlayTypeRegistry>,
    /// Loaded GameConfig — None when config.toml is missing or invalid.
    /// Read at render time for cosmetic toggles (extra_animations) and other
    /// per-session user preferences. Set in AppState::new() from the existing
    /// GameConfig::load() call; not mutated afterwards.
    pub(crate) game_config: Option<GameConfig>,
    /// GPU depth texture for back-to-front depth ordering. Recreated on window resize.
    pub(crate) depth_view: wgpu::TextureView,
    /// Shell-only encoded-byte RGB565 presentation boundary for dialog 0xE2.
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
    pub(crate) cursor_x: f32,
    pub(crate) cursor_y: f32,
    pub(crate) keys_held: HashSet<KeyCode>,
    /// egui integration — input handling + GPU rendering.
    egui: EguiIntegration,
    /// Which screen is currently active (MainMenu, Loading, InGame).
    pub(crate) screen: GameScreen,
    /// Available maps from the RA2 directory for menu selection.
    pub(crate) available_maps: Vec<MapMenuEntry>,
    /// Source-ordered map entries projected from scenario records for the experimental shell.
    pub(crate) skirmish_shell_maps: Vec<MapMenuEntry>,
    /// MPModes rows used by the native Choose Map modal.
    pub(crate) skirmish_modes: Vec<crate::skirmish_modes::SkirmishGameMode>,
    /// Scenario records used by the native Choose Map modal.
    pub(crate) skirmish_scenario_records: Vec<crate::skirmish_scenarios::SkirmishScenarioRecord>,
    /// Player-configured skirmish settings (map, country, credits, etc.).
    pub(crate) skirmish_settings: SkirmishSettings,
    pub(crate) loading_session: Option<crate::app_loading::LoadingSession>,
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
    /// Contents of `VERSION.TXT` from the retail install, used by the
    /// bottom-right version line on the main menu. Falls back to the
    /// numeric `"1.001TUC"` format when the file is missing.
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
    pub(crate) minimap: Option<MinimapRenderer>,
    /// True while left-dragging on minimap (camera pan mode).
    pub(crate) minimap_dragging: bool,
    /// True while middle-mouse button is held for fast camera panning.
    pub(crate) middle_mouse_panning: bool,
    /// Cursor position when middle-mouse pan started (screen pixels).
    pub(crate) middle_mouse_anchor_x: f32,
    pub(crate) middle_mouse_anchor_y: f32,
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
    /// Smoothly animated credits display per owner — ticks toward actual balance
    /// each frame (step = |diff| / 8, clamped to [1, 143]).
    pub(crate) displayed_credits: HashMap<String, i32>,
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
    /// A* pathfinding grid — walkability data from terrain.
    pub(crate) path_grid: Option<PathGrid>,
    /// Sequence definitions per entity type for animation ticking.
    pub(crate) animation_sequences: BTreeMap<String, SequenceSet>,
    /// Game data from rules.ini — needed by combat system for weapon/warhead lookups.
    pub(crate) rules: Option<crate::rules::ruleset::RuleSet>,
    /// Art.ini registry — needed for building animation overlay lookups at render time.
    pub(crate) art_registry: Option<ArtRegistry>,
    /// Parsed infantry animation sequence definitions from art.ini [*Sequence] sections.
    pub(crate) infantry_sequences: InfantrySequenceRegistry,
    /// CSF string table — localized display names for units, buildings, UI text.
    pub(crate) csf: Option<crate::assets::csf_file::CsfFile>,
    /// Owner name → house color index mapping for atlas key lookups.
    pub(crate) house_color_map: HouseColorMap,
    pub(crate) house_roster: HouseRoster,
    /// Cell (rx, ry) → terrain elevation z for entity/overlay height lookup.
    pub(crate) height_map: BTreeMap<(u16, u16), u8>,
    /// Cell (rx, ry) → bridge deck elevation z. Only bridge cells present.
    /// Used by screen_to_iso to resolve clicks on high bridge surfaces.
    pub(crate) bridge_height_map: BTreeMap<(u16, u16), u8>,
    /// Cell (rx, ry) -> high-bridge facts used by the tactical cursor inverse.
    pub(crate) tactical_bridge_inverse_map:
        BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>,
    /// Cell (rx, ry) -> map lighting bundle. Render paths look up compatibility tints per-frame.
    pub(crate) lighting_grid: CellLightGrid,
    /// Last radiation-glow light epoch applied to `lighting_grid`. The glow is
    /// rebuilt only when this changes (a site stepped on `RadLightDelay`, or the
    /// site set changed). App view-state only — never serialized or hashed.
    pub(crate) last_radiation_light_epoch: u64,
    /// Parsed map [Lighting] config used to rebuild transient app lighting after load.
    pub(crate) map_lighting_config: LightingConfig,
    /// Active map theater name (e.g., DESERT).
    pub(crate) theater_name: String,
    /// Active map theater extension (e.g., des).
    pub(crate) theater_ext: String,
    /// Timestamp of the last in-game update for delta time calculation.
    pub(crate) last_update_time: Instant,
    /// Accumulated real time waiting to be consumed by fixed simulation ticks.
    pub(crate) sim_accumulator_ms: u64,
    /// Target/action lines — colored lines from selected units to command destinations.
    pub(crate) target_lines: crate::app_target_lines::TargetLineState,
    /// Config-sourced input delay — copied to each new Simulation instance at game start.
    pub(crate) configured_input_delay_ticks: u64,
    /// Pending order mode for the next right-click command.
    pub(crate) queued_order_mode: app_render::OrderMode,
    /// Control group slots (0-9) storing stable entity ids.
    pub(crate) control_groups: Vec<Vec<u64>>,
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
    /// EVA edge-detection: local player was in low-power state last frame.
    pub(crate) eva_low_power_active: bool,
    /// EVA edge-detection: a local factory was in an underfunded stall last frame.
    pub(crate) eva_funds_stalled: bool,
    /// EVA edge-detection: local mobile entities whose death has already been
    /// announced (pruned as the corpses despawn).
    pub(crate) eva_announced_dying: std::collections::HashSet<u64>,
    /// Sim tick until which the under-attack EVA voice is suppressed. The
    /// native per-house attack-voice repeat delay is UNVERIFIED-pending-trace;
    /// ~30 s is a conservative interim so sustained fire doesn't spam the line.
    pub(crate) eva_under_attack_block_until_tick: u64,
    /// Seeded empty-map sandbox keeps full map visibility while still locking control.
    pub(crate) sandbox_full_visibility: bool,
    /// When true, computer-controlled players do nothing (no AI commands issued).
    pub(crate) disable_ai: bool,
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
    pub(crate) sidebar_scroll_rows: usize,
    /// Asset manager — kept alive for music track lookups.
    pub(crate) asset_manager: Option<AssetManager>,
    /// Background music player (rodio).
    pub(crate) music_player: Option<MusicPlayer>,
    /// Sound effect player (rodio) — plays one-shot SFX (weapons, voices, UI).
    pub(crate) sfx_player: Option<SfxPlayer>,
    /// sound.ini / soundmd.ini registry mapping IDs to .wav filenames.
    pub(crate) sound_registry: SoundRegistry,
    /// audio.idx/bag indices for bag-based sound lookup (voices, EVA).
    /// Searched in order (YR audiomd first, then base audio).
    pub(crate) audio_indices: Vec<crate::assets::audio_bag::AudioIndex>,
    /// EVA announcement registry from eva.ini / evamd.ini.
    /// Maps EVA event names to per-faction audio.bag sound IDs.
    pub(crate) eva_registry: crate::rules::sound_ini::EvaRegistry,
    /// Pending sound events from the current sim tick, drained each frame.
    pub(crate) sound_events: SoundEventQueue,
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
    /// True when the game is paused (ESC menu visible, sim frozen).
    pub(crate) paused: bool,
    /// When true, advance exactly one sim tick while paused, then clear.
    pub(crate) debug_frame_step_requested: bool,
    /// Effective simulation ticks per second — controls game speed.
    /// Default follows retail/YR skirmish stored game speed 1.
    pub(crate) sim_speed_tps: u32,
    /// Client-side in-game Options (0xBBB) state: the six [Options] values plus
    /// transient interaction flags. `game_speed` is the single source of truth for
    /// `sim_speed_tps` (derived on game start and on Options close). App/ui-level.
    pub(crate) in_game_options: crate::ui::shell::in_game_options_state::InGameOptionsState,
    /// Laid-out 0xBBB anchor cached by the overlay render pass each frame it draws,
    /// so the paused mouse handler hit-tests the exact rects that were rendered
    /// (the sidebar-anchored button Y is render-derived; see KD-6). None until the
    /// overlay first renders.
    pub(crate) in_game_options_anchor: Option<crate::ui::shell::layout::InGameOptionsAnchor>,
    /// Hold the loading splash on screen briefly before showing the client UI.
    pub(crate) startup_splash_until: Option<Instant>,
    /// Global elapsed time for looping IdleAnim overlays (flags, smokestacks, etc.).
    pub(crate) idle_anim_elapsed_ms: u32,
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
    /// Cached save-file listing for the save/load panel (avoids per-frame disk I/O).
    pub(crate) save_list_cache: crate::app_save_load_panel::SaveListCache,
    /// Text-field buffer for the dev overlay's "Save As" name input.
    /// Lives in AppState so the field persists across frames while open.
    pub(crate) dev_overlay_save_name: String,
    /// Tick number recorded by the most recent save this session.
    pub(crate) last_save_tick: Option<u64>,
    /// Wall-clock instant of the most recent save this session.
    pub(crate) last_save_instant: Option<std::time::Instant>,
    /// Path of the most recently loaded save (for "Reload last load").
    pub(crate) last_loaded_save_path: Option<std::path::PathBuf>,
    /// Rolling FPS / frame-time tracker for the dev overlay readout.
    pub(crate) frame_timer: crate::app_dev_overlay::FrameTimer,
    // -- Reusable per-frame scratch buffers (avoid allocation each frame) --
    /// Overlay instance scratch vec — cleared and refilled each frame.
    pub(crate) cached_overlay_instances: Vec<crate::render::batch::SpriteInstance>,
    /// Unit (voxel) instance scratch vec — cleared and refilled each frame.
    pub(crate) cached_unit_instances: Vec<crate::render::batch::SpriteInstance>,
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
        self.egui.capture_observation(&self.window)
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

/// Top-level application. Implements winit's ApplicationHandler.
pub struct App {
    state: Option<AppState>,
    shell_capture: Option<crate::app_shell_capture::ShellCaptureSession>,
    tactical_capture: Option<crate::app_tactical_capture::session::TacticalCaptureSession>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    fn resize_surface_for_window_size(state: &mut AppState, size: PhysicalSize<u32>) {
        state.gpu.resize(size.width, size.height);
        state.depth_view = state.gpu.create_depth_texture();
        state.shell_surface_presenter.resize(&state.gpu);
        // The frame-index wave is driven by wall-clock ticks and repaints every
        // frame, so a mid-flight resize simply lets it finish; no snap/cancel.
        let new_scale = auto_detect_ui_scale(size.width, size.height);
        if (new_scale - state.ui_scale).abs() > f32::EPSILON {
            log::info!("UI scale changed: {}x -> {}x", state.ui_scale, new_scale);
            state.sidebar_layout_spec = state.sidebar_layout_spec_base.with_scale(new_scale);
            state.ui_scale = new_scale;
        }
        Self::invalidate_main_menu_movie_if_base_changed(state);
    }

    pub(crate) fn enter_shell_window_mode(state: &mut AppState) {
        state.window.set_resizable(false);
        let target = PhysicalSize::new(SHELL_WINDOW_WIDTH, SHELL_WINDOW_HEIGHT);
        if state.window.inner_size() == target {
            return;
        }
        if let Some(applied_size) = state.window.request_inner_size(target) {
            Self::resize_surface_for_window_size(state, applied_size);
        }
        state.window.request_redraw();
    }

    fn enter_game_window_mode(state: &AppState) {
        state.window.set_resizable(true);
    }

    fn dev_skirmish_shell_enabled() -> bool {
        std::env::var(DEV_SKIRMISH_SHELL_ENV)
            .ok()
            .is_some_and(|value| {
                let value = value.trim();
                !value.is_empty()
                    && value != "0"
                    && !value.eq_ignore_ascii_case("false")
                    && !value.eq_ignore_ascii_case("off")
                    && !value.eq_ignore_ascii_case("no")
            })
    }

    fn native_skirmish_shell_active(state: &AppState) -> bool {
        state.screen == GameScreen::MainMenu
            && (state.main_menu_show_native_skirmish_shell || state.dev_skirmish_shell_enabled)
    }

    fn single_player_shell_active(state: &AppState) -> bool {
        state.screen == GameScreen::MainMenu && state.main_menu_show_single_player_shell
    }

    fn single_player_shell_layout(
        state: &AppState,
    ) -> crate::ui::single_player_shell::SinglePlayerShellLayout {
        crate::ui::single_player_shell::compute_layout(
            state.gpu.config.width,
            state.gpu.config.height,
        )
    }

    fn skirmish_shell_layout(state: &AppState) -> crate::ui::skirmish_shell::SkirmishShellLayout {
        crate::ui::skirmish_shell::compute_layout(state.render_width(), state.render_height())
    }

    fn skirmish_choose_map_layout(
        state: &AppState,
    ) -> crate::ui::skirmish_shell::ChooseMapModalLayout {
        crate::ui::skirmish_shell::compute_choose_map_modal_layout(
            state.render_width(),
            state.render_height(),
        )
    }

    fn validation_modal_dialog_id() -> crate::ui::shell::descriptor::DialogId {
        crate::ui::shell::descriptor::DialogId(0x00CE)
    }

    fn validation_modal_feed(state: &AppState) -> Vec<crate::ui::shell::layout::LaidOutControl> {
        let layout = crate::ui::skirmish_shell::compute_validation_modal_layout(
            state.render_width(),
            state.render_height(),
        );
        vec![crate::ui::shell::layout::LaidOutControl {
            id: crate::ui::shell::modal::control::OK,
            rect: layout.ok_button,
        }]
    }

    fn shell_key_for_code(code: KeyCode) -> Option<ShellKey> {
        match code {
            KeyCode::Tab => Some(ShellKey::Tab),
            KeyCode::Enter | KeyCode::NumpadEnter => Some(ShellKey::Enter),
            KeyCode::Escape => Some(ShellKey::Escape),
            _ => None,
        }
    }

    fn close_native_skirmish_shell(state: &mut AppState) {
        state.main_menu_show_native_skirmish_shell = false;
        state.shell_first_paint_slide = None;
        state.skirmish_shell_return_to_single_player_shell = false;
        state.dev_skirmish_shell_enabled = false;
        state.skirmish_shell_state.choose_map_modal = None;
        state.skirmish_shell_state.validation_modal = None;
        state.skirmish_shell_state.open_combo_dropdown = None;
        state.skirmish_shell_state.dropdown_scroll_drag = None;
        state.skirmish_shell_state.dropdown_scroll_press = None;
        state.skirmish_shell_state.trackbar_drag = None;
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        crate::ui::skirmish_shell::blur_player_name_edit(&mut state.skirmish_shell_state);
        state.skirmish_shell_last_painted_pressed_button = None;
        state.skirmish_preview_texture = None;
        Self::enter_shell_window_mode(state);
    }

    fn selected_skirmish_mode_is_cooperative(state: &AppState, mode_id: i32) -> bool {
        crate::skirmish_modes::mode_by_id(&state.skirmish_modes, mode_id)
            .is_some_and(|mode| mode.override_file.eq_ignore_ascii_case("MPCoopMD.ini"))
    }

    fn selected_shell_map_file(state: &AppState) -> Option<String> {
        state
            .skirmish_shell_maps
            .get(state.skirmish_shell_state.selected_map_idx)
            .map(|map| map.file_name.clone())
    }

    fn apply_selected_shell_map_file(state: &mut AppState, file_name: &str) -> bool {
        let Some(map_idx) = state
            .skirmish_shell_maps
            .iter()
            .position(|map| map.file_name.eq_ignore_ascii_case(file_name))
        else {
            log::warn!("No loadable Skirmish map entry exists for {file_name}");
            return false;
        };
        crate::ui::skirmish_shell::accept_selected_map(
            &mut state.skirmish_shell_state,
            &state.skirmish_shell_maps,
            map_idx,
        );
        if let Some(legacy_idx) = state
            .available_maps
            .iter()
            .position(|map| map.file_name.eq_ignore_ascii_case(file_name))
        {
            state.skirmish_settings.selected_map_idx = legacy_idx;
        }
        state.skirmish_preview_texture = None;
        true
    }

    fn ensure_active_cooperative_shell_selection(state: &mut AppState) {
        if !Self::selected_skirmish_mode_is_cooperative(
            state,
            state.skirmish_shell_state.selected_mode_id,
        ) {
            return;
        }
        let Some(file_name) = Self::selected_shell_map_file(state) else {
            return;
        };
        let chosen_map = match state
            .offline_skirmish_runtime
            .ensure_cooperative_selection(&file_name, &state.skirmish_shell_maps)
        {
            Ok(chosen_map) => chosen_map,
            Err(err) => {
                log::warn!("Could not bind Cooperative shell progress: {err}");
                None
            }
        };
        if let Some(chosen_map) = chosen_map {
            Self::apply_selected_shell_map_file(state, &chosen_map);
        }
    }

    fn ensure_active_cooperative_modal_selection(state: &mut AppState) {
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_ref() else {
            return;
        };
        let mode_id = modal.selected_mode_id;
        let record_index = modal.selected_record_index();
        if !Self::selected_skirmish_mode_is_cooperative(state, mode_id) {
            return;
        }
        let Some(file_name) = record_index
            .and_then(|index| state.skirmish_scenario_records.get(index))
            .map(|record| record.file_name.clone())
        else {
            return;
        };
        if let Err(err) = state
            .offline_skirmish_runtime
            .ensure_cooperative_selection(&file_name, &state.skirmish_shell_maps)
        {
            log::warn!("Could not bind Cooperative Choose Map progress: {err}");
        }
    }

    fn sync_legacy_skirmish_settings_from_shell(state: &mut AppState) {
        let selected_file = Self::selected_shell_map_file(state);
        let mut settings = crate::ui::skirmish_shell::launch_settings(&state.skirmish_shell_state);
        settings.selected_map_idx = selected_file
            .as_deref()
            .and_then(|file_name| {
                state
                    .available_maps
                    .iter()
                    .position(|map| map.file_name.eq_ignore_ascii_case(file_name))
            })
            .unwrap_or(0);
        state.skirmish_settings = settings;
    }

    fn teardown_skirmish_shell_for_start(state: &mut AppState) {
        state.main_menu_show_native_skirmish_shell = false;
        state.main_menu_show_single_player_shell = false;
        state.skirmish_shell_return_to_single_player_shell = false;
        state.shell_first_paint_slide = None;
        state.skirmish_shell_state.choose_map_modal = None;
        state.skirmish_shell_state.validation_modal = None;
        state.skirmish_shell_state.open_combo_dropdown = None;
        state.skirmish_shell_state.dropdown_scroll_drag = None;
        state.skirmish_shell_state.dropdown_scroll_press = None;
        state.skirmish_shell_state.trackbar_drag = None;
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        crate::ui::skirmish_shell::blur_player_name_edit(&mut state.skirmish_shell_state);
        state.skirmish_shell_last_painted_pressed_button = None;
        state.skirmish_preview_texture = None;
    }

    fn refresh_single_player_load_state(state: &mut AppState) {
        state.save_list_cache.refresh_if_dirty();
        state.single_player_shell_state.load_saved_game_enabled =
            !state.save_list_cache.entries.is_empty();
    }

    fn open_single_player_shell(state: &mut AppState) {
        Self::enter_shell_window_mode(state);
        // Native destroys 0xE2 (including child 0x71A) before constructing
        // 0x100. Invalidate at the route edge rather than waiting for a paint:
        // a queued Back/Escape can otherwise return to 0xE2 before 0x100 draws
        // and incorrectly preserve the old main-menu movie timeline.
        crate::app_main_menu_shell_render::clear_ra2ts_movie_session(state);
        crate::app_shell_transition::invalidate_main_menu_dialog_instance(state);
        state.main_menu_show_single_player_shell = true;
        state.main_menu_show_native_skirmish_shell = false;
        state.skirmish_shell_return_to_single_player_shell = false;
        state.single_player_shell_state.pressed_owner_draw_button = None;
        state.single_player_shell_state.hovered_owner_draw_button = None;
        state.single_player_shell_state.hover_started_at = None;
        Self::refresh_single_player_load_state(state);
    }

    fn close_single_player_shell(state: &mut AppState) {
        // Result 0x12 destroys 0x100 before the main-menu loop constructs a new
        // 0xE2. Clear immediately so a same-event-loop round trip cannot reuse
        // the source dialog's movie session.
        crate::app_main_menu_shell_render::clear_ra2ts_movie_session(state);
        crate::app_shell_transition::invalidate_main_menu_dialog_instance(state);
        state.main_menu_show_single_player_shell = false;
        state.single_player_shell_state.pressed_owner_draw_button = None;
        state.single_player_shell_state.hovered_owner_draw_button = None;
        state.single_player_shell_state.hover_started_at = None;
    }

    fn enter_native_skirmish_from_single_player(state: &mut AppState) {
        // Native destroys dialog 0x100 and its child 0x71A movie handle before
        // constructing 0x102. Drop the hidden Rust session as well so returning
        // to 0x100 cannot continue the pre-Skirmish RA2TS timeline.
        crate::app_main_menu_shell_render::clear_ra2ts_movie_session(state);
        state.main_menu_show_single_player_shell = false;
        state.main_menu_show_native_skirmish_shell = true;
        state.skirmish_shell_return_to_single_player_shell = true;
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        state.skirmish_shell_last_painted_pressed_button = None;
        Self::ensure_skirmish_shell_chrome(state);
        Self::ensure_active_cooperative_shell_selection(state);
        // The skirmish dialog (0x102) slides its controls in on first paint like
        // every shell dialog; the per-frame slide trigger starts that wave once
        // the skirmish shell becomes the showing screen. Clear any stale wave
        // from the source shell here so the trigger restarts cleanly.
        state.shell_first_paint_slide = None;
    }

    fn return_from_skirmish_to_single_player_shell(state: &mut AppState) {
        state.main_menu_show_native_skirmish_shell = false;
        state.shell_first_paint_slide = None;
        state.skirmish_shell_return_to_single_player_shell = false;
        state.skirmish_shell_state.choose_map_modal = None;
        state.skirmish_shell_state.validation_modal = None;
        state.skirmish_shell_state.open_combo_dropdown = None;
        state.skirmish_shell_state.dropdown_scroll_drag = None;
        state.skirmish_shell_state.dropdown_scroll_press = None;
        state.skirmish_shell_state.trackbar_drag = None;
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        crate::ui::skirmish_shell::blur_player_name_edit(&mut state.skirmish_shell_state);
        state.skirmish_shell_last_painted_pressed_button = None;
        state.skirmish_preview_texture = None;
        Self::open_single_player_shell(state);
    }

    fn start_selected_skirmish(state: &mut AppState) {
        let map_name = state
            .available_maps
            .get(state.skirmish_settings.selected_map_idx)
            .map(|m| m.file_name.clone())
            .unwrap_or_else(|| "auto".to_string());
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        state.skirmish_shell_last_painted_pressed_button = None;
        state.main_menu_show_single_player_shell = false;
        state.skirmish_shell_return_to_single_player_shell = false;
        state.shell_first_paint_slide = None;
        let request = crate::app_loading::LoadingRequest::generic_map_load(
            map_name,
            state.skirmish_settings.clone(),
        );
        crate::app_loading::begin_loading(state, request);
        Self::enter_game_window_mode(state);
        state.zoom_level = 1.0;
        state.zoom_target = 1.0;
    }

    fn start_skirmish_session(
        state: &mut AppState,
        session: crate::skirmish_launch::SkirmishLaunchSession,
    ) {
        let request = match crate::match_bootstrap::classify_startup_session(&session) {
            crate::match_bootstrap::StartupSessionClassification::AcceptedExplicitFixedBattle(
                accepted,
            ) => {
                let correlation = match crate::match_bootstrap::allocate_match_correlation(
                    &mut state.next_match_correlation,
                ) {
                    Ok(correlation) => correlation,
                    Err(err) => {
                        log::error!("Cannot start accepted match: {err}");
                        return;
                    }
                };
                let mut clock = crate::match_bootstrap::OrdinaryMatchSeedClock;
                let startup = crate::match_bootstrap::prepare_match_startup(
                    correlation,
                    accepted,
                    &mut clock,
                );
                crate::app_loading::LoadingRequest::accepted_skirmish(
                    startup,
                    state.skirmish_settings.clone(),
                )
            }
            crate::match_bootstrap::StartupSessionClassification::UnverifiedLegacy(reason) => {
                log::warn!("Skirmish startup uses unverified compatibility path: {reason:?}");
                let mut clock = crate::match_bootstrap::OrdinaryMatchSeedClock;
                let seed = crate::match_bootstrap::read_match_seed(&mut clock);
                crate::app_loading::LoadingRequest::unverified_legacy_skirmish(
                    session,
                    seed,
                    state.skirmish_settings.clone(),
                )
            }
        };
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        state.skirmish_shell_last_painted_pressed_button = None;
        state.main_menu_show_single_player_shell = false;
        state.skirmish_shell_return_to_single_player_shell = false;
        state.main_menu_show_native_skirmish_shell = false;
        state.shell_first_paint_slide = None;
        state.skirmish_preview_texture = None;
        crate::app_loading::begin_loading(state, request);
        Self::enter_game_window_mode(state);
        state.zoom_level = 1.0;
        state.zoom_target = 1.0;
    }

    pub(crate) fn ensure_skirmish_shell_chrome(state: &mut AppState) {
        if state.skirmish_shell_chrome.is_some() {
            return;
        }

        let Ok(config) = GameConfig::load() else {
            log::warn!("Could not load game config for development Skirmish shell assets");
            return;
        };
        let Ok(assets) = AssetManager::new(&config.paths.ra2_dir) else {
            log::warn!("Could not load RA2 assets for development Skirmish shell");
            return;
        };

        state.skirmish_shell_chrome =
            crate::render::skirmish_shell_chrome::build_skirmish_shell_chrome_atlas(
                &state.gpu,
                &state.batch_renderer,
                &assets,
            );
    }

    fn build_startup_asset_manager(config: Option<&GameConfig>) -> Option<AssetManager> {
        config.and_then(|cfg| match AssetManager::new(&cfg.paths.ra2_dir) {
            Ok(manager) => Some(manager),
            Err(err) => {
                log::warn!("Could not load startup shell assets: {err:#}");
                None
            }
        })
    }

    fn load_version_txt(config: Option<&GameConfig>) -> String {
        const FALLBACK: &str = "1.001TUC";
        let Some(cfg) = config else {
            return FALLBACK.to_string();
        };
        let path = cfg.paths.ra2_dir.join("VERSION.TXT");
        match std::fs::read_to_string(&path) {
            Ok(s) => s.trim_end_matches(['\r', '\n']).to_string(),
            Err(err) => {
                log::info!(
                    "VERSION.TXT not readable at {}: {err}; using fallback",
                    path.display()
                );
                FALLBACK.to_string()
            }
        }
    }

    fn draw_skirmish_shell_dev_toggle(ctx: &egui::Context, enabled: &mut bool) -> bool {
        let mut changed = false;
        egui::Window::new("Developer")
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-18.0, -18.0))
            .collapsible(true)
            .resizable(false)
            .show(ctx, |ui| {
                changed = ui
                    .checkbox(enabled, "Experimental Skirmish Shell")
                    .on_hover_text("Switches the setup screen to the research shell renderer.")
                    .changed();
            });
        changed
    }

    fn render_egui_main_menu_fallback(
        state: &mut AppState,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        event_loop: &ActiveEventLoop,
    ) -> Result<()> {
        app_transitions::clear_screen(encoder, view);
        state.egui.begin_frame(&state.window);
        let action = main_menu::draw_main_menu_with_maps(
            &state.egui.ctx,
            &state.available_maps,
            &mut state.skirmish_settings,
        );
        let mut dev_shell_enabled = state.dev_skirmish_shell_enabled;
        let dev_shell_changed =
            Self::draw_skirmish_shell_dev_toggle(&state.egui.ctx, &mut dev_shell_enabled);
        if dev_shell_changed {
            state.dev_skirmish_shell_enabled = dev_shell_enabled;
            Self::enter_shell_window_mode(state);
            if state.dev_skirmish_shell_enabled || state.main_menu_show_native_skirmish_shell {
                Self::ensure_skirmish_shell_chrome(state);
            } else {
                state.skirmish_shell_state.pressed_owner_draw_button = None;
            }
        }
        // Confirm modal can be open over the legacy egui menu too; draw it in
        // the same frame so its buttons receive input. This degraded egui path has
        // no SHP shell, so the quit-confirm renders as the egui card here.
        let confirm = Self::draw_main_menu_dialogs(state, true);
        // Degraded fallback (shell chrome failed to load) has no SHP cursor of
        // its own, so keep the OS cursor visible here rather than hiding it and
        // leaving the egui menu with no pointer at all.
        state
            .egui
            .end_frame_and_render(&state.gpu, encoder, view, &state.window, false);
        if confirm {
            event_loop.exit();
            return Ok(());
        }
        Self::handle_main_menu_action(state, action, event_loop);
        Ok(())
    }

    fn handle_main_menu_action(
        state: &mut AppState,
        action: main_menu::MenuAction,
        event_loop: &ActiveEventLoop,
    ) {
        let _ = event_loop;
        match action {
            main_menu::MenuAction::StartSelected => Self::start_selected_skirmish(state),
            // Route through the same confirm message box for consistency with
            // the native shell; the game does not quit on the first click.
            main_menu::MenuAction::Exit => Self::open_exit_confirm_modal(state),
            main_menu::MenuAction::None => {}
        }
    }

    fn handle_skirmish_shell_action(
        state: &mut AppState,
        action: crate::ui::skirmish_shell::SkirmishShellAction,
        event_loop: &ActiveEventLoop,
    ) {
        let action = crate::ui::skirmish_shell::apply_action(
            &mut state.skirmish_shell_state,
            action,
            &state.skirmish_shell_maps,
        );

        match action {
            crate::ui::skirmish_shell::SkirmishShellAction::StartGame => {
                match crate::ui::skirmish_shell::launch_session(
                    &state.skirmish_shell_state,
                    &state.skirmish_shell_maps,
                    &state.skirmish_modes,
                ) {
                    Ok(raw_session) => {
                        match state.offline_skirmish_runtime.close_shell_transaction(
                            &state.skirmish_shell_state,
                            &state.skirmish_shell_maps,
                            &state.skirmish_modes,
                            &raw_session,
                        ) {
                            Ok(resolved_session) => {
                                Self::sync_legacy_skirmish_settings_from_shell(state);
                                Self::teardown_skirmish_shell_for_start(state);
                                state.offline_skirmish_runtime.persist_snapshot();
                                Self::start_skirmish_session(state, resolved_session);
                            }
                            Err(err) => {
                                log::error!(
                                    "Could not resolve Cooperative shell assignments: {err}"
                                );
                                state.window.request_redraw();
                            }
                        }
                    }
                    Err(err) => {
                        if let Some(modal) = Self::skirmish_validation_modal_for_error(state, &err)
                        {
                            Self::show_skirmish_validation_modal(state, modal);
                            state.window.request_redraw();
                        } else {
                            log::warn!("Could not start skirmish shell session: {err:?}");
                        }
                    }
                }
            }
            crate::ui::skirmish_shell::SkirmishShellAction::BackOrExit => {
                match crate::ui::skirmish_shell::pack_launch_session_without_start_validation(
                    &state.skirmish_shell_state,
                    &state.skirmish_shell_maps,
                    &state.skirmish_modes,
                ) {
                    Ok(raw_session) => {
                        if let Err(err) = state.offline_skirmish_runtime.close_shell_transaction(
                            &state.skirmish_shell_state,
                            &state.skirmish_shell_maps,
                            &state.skirmish_modes,
                            &raw_session,
                        ) {
                            // Invalid Cooperative content has no parity-safe
                            // retry cap. Keep Back usable, surface the malformed
                            // data, and retain every draw consumed before error.
                            log::error!("Could not complete Cooperative Back randomization: {err}");
                        }
                    }
                    Err(err) => {
                        log::warn!("Could not pack raw Skirmish Back session: {err:?}");
                    }
                }
                if state.skirmish_shell_return_to_single_player_shell {
                    Self::return_from_skirmish_to_single_player_shell(state);
                } else if Self::native_skirmish_shell_active(state) {
                    Self::close_native_skirmish_shell(state);
                } else {
                    state.offline_skirmish_runtime.persist_snapshot();
                    event_loop.exit();
                    return;
                }
                state.offline_skirmish_runtime.persist_snapshot();
            }
            crate::ui::skirmish_shell::SkirmishShellAction::ChooseMap => {
                Self::open_choose_map_modal(state);
            }
            crate::ui::skirmish_shell::SkirmishShellAction::None
            | crate::ui::skirmish_shell::SkirmishShellAction::SelectColor(_)
            | crate::ui::skirmish_shell::SkirmishShellAction::SelectMap(_) => {}
        }
    }

    fn skirmish_shell_label(state: &AppState, key: &str, fallback: &str) -> String {
        Self::csf_label(state, key, fallback)
    }

    /// Resolve a CSF string key to display text, falling back to the supplied
    /// English string when the table is absent or missing the key.
    fn csf_label(state: &AppState, key: &str, fallback: &str) -> String {
        state
            .csf
            .as_ref()
            .and_then(|csf| csf.get(key))
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| fallback.to_string())
    }

    fn skirmish_validation_modal_for_error(
        state: &AppState,
        err: &crate::skirmish_launch::LaunchValidationError,
    ) -> Option<crate::ui::skirmish_shell::SkirmishValidationModalState> {
        let ok = Self::skirmish_shell_label(state, "TXT_OK", "OK");
        let message = match err {
            crate::skirmish_launch::LaunchValidationError::MapCapacityExceeded {
                capacity, ..
            } => {
                let template = Self::skirmish_shell_label(
                    state,
                    "TXT_SCENARIO_TOO_SMALL",
                    "This map has a %d player max. The max includes human and computer players.",
                );
                template.replace("%d", &capacity.to_string())
            }
            crate::skirmish_launch::LaunchValidationError::NoEnabledOpponent => {
                Self::skirmish_shell_label(
                    state,
                    "TXT_NEED_AT_LEAST_TWO_PLAYERS",
                    "You need at least two players to start the game!",
                )
            }
            crate::skirmish_launch::LaunchValidationError::SameExplicitTeam { .. } => {
                Self::skirmish_shell_label(
                    state,
                    "TXT_CANNOT_ALLY",
                    "Must have more than one team to start a game!",
                )
            }
            _ => return None,
        };
        Some(crate::ui::skirmish_shell::SkirmishValidationModalState::new(message, ok))
    }

    fn show_skirmish_validation_modal(
        state: &mut AppState,
        modal: crate::ui::skirmish_shell::SkirmishValidationModalState,
    ) {
        state.skirmish_shell_state.validation_modal = Some(modal);
        state
            .shell_controller
            .ensure_active(Self::validation_modal_dialog_id(), true);
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        state.skirmish_shell_state.open_combo_dropdown = None;
        state.skirmish_shell_state.dropdown_scroll_drag = None;
        state.skirmish_shell_state.dropdown_scroll_press = None;
        state.skirmish_shell_state.trackbar_drag = None;
        state.skirmish_shell_last_painted_pressed_button = None;
        crate::ui::skirmish_shell::blur_player_name_edit(&mut state.skirmish_shell_state);
    }

    fn open_choose_map_modal(state: &mut AppState) {
        state.skirmish_shell_state.open_combo_dropdown = None;
        state.skirmish_shell_state.dropdown_scroll_drag = None;
        state.skirmish_shell_state.trackbar_drag = None;
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        crate::ui::skirmish_shell::clear_status_help_text(&mut state.skirmish_shell_state);
        let current_record_index = Self::current_choose_map_record_index(state);
        state.skirmish_shell_state.choose_map_modal =
            Some(crate::ui::skirmish_shell::ChooseMapModalState::open(
                state.skirmish_shell_state.selected_mode_id,
                current_record_index,
                &state.skirmish_modes,
                &state.skirmish_scenario_records,
            ));
        Self::ensure_active_cooperative_modal_selection(state);
    }

    fn current_choose_map_record_index(state: &AppState) -> Option<usize> {
        let file_name = state
            .skirmish_shell_maps
            .get(state.skirmish_shell_state.selected_map_idx)?
            .file_name
            .as_str();
        state
            .skirmish_scenario_records
            .iter()
            .position(|record| record.file_name.eq_ignore_ascii_case(file_name))
    }

    fn close_choose_map_modal(state: &mut AppState) {
        if let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() {
            modal.pressed_button = None;
        }
        state.skirmish_shell_state.choose_map_modal = None;
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        state.skirmish_shell_last_painted_pressed_button = None;
    }

    fn commit_choose_map_selection(
        state: &mut AppState,
        selection: crate::ui::skirmish_shell::ChooseMapSelection,
    ) -> bool {
        let Some(record_idx) = selection.record_index else {
            return false;
        };
        let Some(record) = state.skirmish_scenario_records.get(record_idx) else {
            return false;
        };
        let clicked_file_name = record.file_name.clone();
        let selected_file_name =
            if Self::selected_skirmish_mode_is_cooperative(state, selection.mode_id) {
                match state
                    .offline_skirmish_runtime
                    .accept_cooperative_selection(&clicked_file_name, &state.skirmish_shell_maps)
                {
                    Ok(Some(chosen_map)) => chosen_map,
                    Ok(None) => clicked_file_name,
                    Err(err) => {
                        log::warn!("Could not accept Cooperative campaign selection: {err}");
                        return false;
                    }
                }
            } else {
                clicked_file_name
            };
        let Some(map_idx) = state
            .skirmish_shell_maps
            .iter()
            .position(|map| map.file_name.eq_ignore_ascii_case(&selected_file_name))
        else {
            log::warn!(
                "Choose Map selected {}, but no loadable map entry exists yet",
                selected_file_name
            );
            return false;
        };

        state.skirmish_shell_state.selected_mode_id = selection.mode_id;
        crate::ui::skirmish_shell::repair_teams_for_selected_mode(
            &mut state.skirmish_shell_state,
            &state.skirmish_modes,
        );
        crate::ui::skirmish_shell::accept_selected_map(
            &mut state.skirmish_shell_state,
            &state.skirmish_shell_maps,
            map_idx,
        );
        if let Some(legacy_idx) = state
            .available_maps
            .iter()
            .position(|map| map.file_name.eq_ignore_ascii_case(&selected_file_name))
        {
            state.skirmish_settings.selected_map_idx = legacy_idx;
        }
        state.skirmish_preview_texture = None;

        // Native 0x4B2: setting the right-panel game-type / map-label text
        // restarts that static's reveal from the first character. The title is
        // not re-revealed during ordinary setup, so leave it alone. Restart even
        // if a prior reveal had already completed (native restarts regardless).
        let now = Instant::now();
        let (_title, game_type, map_label) =
            crate::app_skirmish_shell_render::skirmish_right_panel_label_strings(state);
        state
            .skirmish_shell_state
            .game_type_reveal
            .start(&game_type, now);
        state
            .skirmish_shell_state
            .map_label_reveal
            .start(&map_label, now);
        true
    }

    fn handle_choose_map_modal_mouse_down(state: &mut AppState) -> bool {
        let layout = Self::skirmish_choose_map_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
            return false;
        };
        if let Some(button) = crate::ui::skirmish_shell::choose_map_modal_button_at(&layout, x, y) {
            modal.pressed_button = Some(button);
            Self::play_main_menu_button_sound(state);
            return true;
        }
        let prior_mode = modal.selected_mode_id;
        if modal.handle_listbox_mouse_down(
            &layout,
            &state.skirmish_modes,
            &state.skirmish_scenario_records,
            x,
            y,
        ) {
            let mode_changed = modal.selected_mode_id != prior_mode;
            let _ = modal;
            if mode_changed {
                Self::ensure_active_cooperative_modal_selection(state);
            }
            return true;
        }
        layout.dialog.contains(x, y)
    }

    fn handle_choose_map_modal_mouse_up(state: &mut AppState) -> bool {
        let layout = Self::skirmish_choose_map_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
            return false;
        };
        let pressed_button = modal.pressed_button.take();
        let released_button = crate::ui::skirmish_shell::choose_map_modal_button_at(&layout, x, y);
        let should_fire = pressed_button.is_some() && pressed_button == released_button;
        if !should_fire {
            return layout.dialog.contains(x, y) || pressed_button.is_some();
        }

        let mut selection_to_commit = None;
        let mut close_modal = false;
        // Copied out inside the arm so the `modal` borrow ends before anything
        // below reborrows `state`. `ChooseMapSelection` is `Copy`.
        let mut open_random_map_setup = None;
        match released_button.expect("checked equal to pressed button") {
            crate::ui::skirmish_shell::ChooseMapModalButton::UseMap0x6c5 => {
                selection_to_commit = modal.accept_selection();
            }
            crate::ui::skirmish_shell::ChooseMapModalButton::Cancel0x5c0 => {
                close_modal = true;
            }
            crate::ui::skirmish_shell::ChooseMapModalButton::CreateRandomMap0x583 => {
                open_random_map_setup = Some(modal.cancel_selection());
            }
        }
        if let Some(selection) = selection_to_commit {
            close_modal = Self::commit_choose_map_selection(state, selection);
        }
        if let Some(previous) = open_random_map_setup {
            // The setup dialog opens OVER the chooser, which stays open behind
            // it so a cancel returns to the untouched selection.
            state.skirmish_shell_state.random_map_setup_modal =
                Some(crate::ui::skirmish_shell::RandomMapSetupModalState::open(
                    crate::map::rmg::RmgOptions::default(),
                    Some(previous),
                    // Saved-seed browsing (0x6C2/0x6C3/0x6C4) is not implemented.
                    false,
                    &mut crate::ui::skirmish_shell::DialogRng::from_entropy(),
                ));
        }
        if close_modal {
            Self::close_choose_map_modal(state);
        }
        true
    }

    /// Kick off generation on a worker and return immediately.
    ///
    /// Everything that needs the asset manager is done here, up front; only
    /// plain data crosses to the worker.
    fn start_random_map_generation(
        state: &mut AppState,
        options: &crate::map::rmg::RmgOptions,
        accept_on_finish: bool,
    ) -> bool {
        let Some(asset_manager) = state.asset_manager.as_mut() else {
            return false;
        };
        let settings = crate::map::rmg::RmgSettings::load(asset_manager);
        let theater_name = crate::map::rmg::emit::theater_name(options.theater);
        let Some(theater) = crate::map::theater::load_theater(asset_manager, theater_name) else {
            log::warn!("random map: theater {theater_name} unavailable");
            return false;
        };
        let terrain_rules = asset_manager
            .get_ref("rulesmd.ini")
            .and_then(|bytes| crate::rules::ini_parser::IniFile::from_bytes(bytes).ok())
            .map(|ini| crate::rules::terrain_rules::TerrainRules::from_ini(&ini))
            .unwrap_or_default();
        let resolved_inputs = crate::map::rmg::build::ResolvedTheaterInputs::from_theater(
            &theater,
            &terrain_rules,
            crate::map::rmg::trig::global().cloned(),
        );
        let blocks =
            crate::map::rmg::theater_blocks::TheaterTileBlocks::build(&theater.lookup, |name| {
                asset_manager.get(name)
            });
        // `[AI] NeutralTechBuildings` plus each type's `Foundation=`, resolved
        // here because only plain data may cross to the worker.
        let tech_types = crate::app_init_helpers::load_neutral_tech_types(asset_manager);

        let (sender, receiver) = std::sync::mpsc::channel();
        let options = options.clone();
        // Generation stays single-threaded and seed-driven; the thread changes
        // only where it runs, never the order it consumes its RNG in.
        let spawned = std::thread::Builder::new()
            .name("random-map-generate".to_string())
            .spawn(move || {
                let generated = crate::map::rmg::build::generate_map_observed(
                    &options,
                    &settings,
                    &resolved_inputs,
                    &blocks,
                    &tech_types,
                    // A closed receiver means the dialog went away; dropping
                    // what we produce is the correct outcome, not an error.
                    &mut |view| {
                        if !draws_preview(view.point()) {
                            return;
                        }
                        let _ = sender.send(RandomMapUpdate::Progress(Box::new(view.snapshot())));
                    },
                );
                if generated.unfilled_start_slots > 0 {
                    log::warn!(
                        "Random map is short of spawns: {} start slot(s) could \
                         not be filled; those players have no start position",
                        generated.unfilled_start_slots
                    );
                }
                let _ = sender.send(RandomMapUpdate::Finished(Box::new(generated)));
            });
        match spawned {
            Ok(_handle) => {
                state.random_map_generation = Some(RandomMapGenerationJob {
                    receiver,
                    theater: Box::new(theater),
                    terrain_rules: Box::new(terrain_rules),
                    accept_on_finish,
                });
                true
            }
            Err(err) => {
                log::warn!("random map: could not spawn the generator thread: {err}");
                false
            }
        }
    }

    /// Collect whatever the generator has produced since the last frame.
    ///
    /// Called every frame while a job is in flight. Returns true when the dialog
    /// changed, so the caller knows to redraw.
    ///
    /// Only the newest of several progress snapshots is rasterised. Colouring is
    /// the expensive half, and an image the worker overtook before a frame was
    /// drawn was never on screen to be seen.
    pub(crate) fn poll_random_map_generation(state: &mut AppState) -> bool {
        if state.random_map_generation.is_some()
            && state.skirmish_shell_state.random_map_setup_modal.is_none()
        {
            // The dialog went away without the job going with it. Drop it here
            // rather than trusting every close path to remember: a job with no
            // dialog has nowhere to deliver, and letting it finish would write
            // a preview file for a map nobody asked for.
            state.random_map_generation = None;
            return false;
        }
        let Some(job) = state.random_map_generation.as_ref() else {
            return false;
        };
        let mut latest_progress = None;
        let mut finished = None;
        let mut died = false;
        loop {
            match job.receiver.try_recv() {
                Ok(RandomMapUpdate::Progress(snapshot)) => latest_progress = Some(snapshot),
                Ok(RandomMapUpdate::Finished(generated)) => {
                    finished = Some(generated);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    died = true;
                    break;
                }
            }
        }

        if let Some(generated) = finished {
            let job = state
                .random_map_generation
                .take()
                .expect("checked present above");
            let preview = Self::rasterise_generated_map(state, &job, &generated);
            if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
                modal.finish_generate(preview);
            }
            if job.accept_on_finish {
                Self::accept_random_map_setup(state);
            }
            return true;
        }

        if died {
            // The worker ended without a result. Clear the job so the dialog
            // does not sit disabled forever waiting on it.
            log::warn!("random map: the generator thread ended without a result");
            state.random_map_generation = None;
            if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
                modal.finish_generate(None);
            }
            return true;
        }

        let Some(snapshot) = latest_progress else {
            return false;
        };
        // Lifted out and put straight back: rasterising reads the job and the
        // rest of the app state at once, and the job lives inside that state.
        let job = state
            .random_map_generation
            .take()
            .expect("checked present above");
        let preview =
            Self::rasterise_map(state, &job, &snapshot.map_file, &snapshot.start_waypoints);
        state.random_map_generation = Some(job);
        if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
            if let Some(preview) = preview {
                modal.show_progress_preview(preview);
            }
        }
        true
    }

    /// Close the setup dialog, abandoning any generation still running for it.
    ///
    /// Dropping the job drops the receiver, so a worker still going finds a
    /// closed channel on its next send and its remaining output goes nowhere.
    /// That matters beyond tidiness: a late finish would otherwise overwrite
    /// `RandMap.img`, changing the chooser's thumbnail to a map the player
    /// walked away from.
    fn close_random_map_setup(state: &mut AppState) {
        state.skirmish_shell_state.random_map_setup_modal = None;
        state.random_map_generation = None;
    }

    /// Commit the dialog's options and close it. Shared by the immediate accept
    /// and the one deferred behind a generation.
    fn accept_random_map_setup(state: &mut AppState) {
        let Some(crate::ui::skirmish_shell::AcceptOutcome::Commit(options)) = state
            .skirmish_shell_state
            .random_map_setup_modal
            .as_ref()
            .map(|modal| modal.accept())
        else {
            return;
        };
        match Self::commit_random_map_setup(state, &options) {
            Ok(()) => Self::close_random_map_setup(state),
            Err(err) => {
                // Staying open is deliberate: a missing seed file makes the
                // launch path fall back to defaults, which would silently
                // start a different map than the one configured.
                log::error!("random map: could not write {RANDMAP_SED_FILE}: {err}");
            }
        }
    }

    /// Rasterise the finished map and persist it as the chooser's thumbnail.
    fn rasterise_generated_map(
        state: &AppState,
        job: &RandomMapGenerationJob,
        generated: &crate::map::rmg::GeneratedMap,
    ) -> Option<crate::map::rmg::preview::PreviewImage> {
        let preview =
            Self::rasterise_map(state, job, &generated.map_file, &generated.start_waypoints)?;
        // Only the finished map is written out: the file is what the chooser
        // row shows later, and a half-built map is not that map.
        Self::write_random_map_preview_file(state, &preview);
        Some(preview)
    }

    /// Colour and rasterise a map. Main thread only: the resolver reads theater
    /// data and the ore/gem colours come out of overlay SHPs.
    ///
    /// Mid-generation snapshots go through here too, so an in-progress preview
    /// is coloured by exactly the path that colours the finished one.
    fn rasterise_map(
        state: &AppState,
        job: &RandomMapGenerationJob,
        map_file: &crate::map::map_file::MapFile,
        start_waypoints: &[(u8, u16, u16)],
    ) -> Option<crate::map::rmg::preview::PreviewImage> {
        // LAT defaults off for runtime maps, so resolve the same way the load
        // path will; a different setting here would colour cells the player
        // never sees.
        let resolved_terrain = crate::map::resolved_terrain::ResolvedTerrainGrid::build(
            map_file,
            Some(&job.theater),
            state.asset_manager.as_ref(),
            Some(&job.terrain_rules),
            None,
            false,
            RANDOM_MAP_PREVIEW_CLIFF_BACK_IMPASSABILITY,
        );
        // Ore and gem cells take their colour from the overlay's own SHP: the
        // growth stage indexes the frame list and the frame header carries the
        // radar triple. The artwork is never sampled for it, so there is no
        // substitute for loading the file.
        let overlay_registry = state.overlay_registry.as_ref();
        let assets = state.asset_manager.as_ref();
        let theater_ext = job.theater.extension;
        let overlay_radar = |overlay_id: u8, stage: u8| -> Option<[u8; 3]> {
            let registry = overlay_registry?;
            // The tiberium flag is the gate: walls, roads and bridges are
            // overlays too, and they keep the terrain colour underneath.
            if !registry.flags(overlay_id)?.tiberium {
                return None;
            }
            // The stage's colour out of the overlay SHP wins; the type's
            // RadarColor= stands in when that comes back essentially black,
            // which is also what happens when the art is missing entirely.
            let from_art = (|| {
                let name = registry.name(overlay_id)?;
                let bytes = crate::map::overlay_types::overlay_shp_candidates(name, theater_ext)
                    .iter()
                    .find_map(|candidate| assets?.get_ref(candidate))?;
                let shp = crate::assets::shp_file::ShpFile::from_bytes(bytes).ok()?;
                Some(shp.frames.get(stage as usize)?.radar_color)
            })()
            .filter(|rgb| *rgb != [0, 0, 0]);
            from_art.or_else(|| registry.flags(overlay_id)?.radar_color)
        };
        let cells = crate::map::rmg::preview::preview_cells_from_map(
            map_file,
            &resolved_terrain,
            &overlay_radar,
        );
        let waypoints = crate::map::rmg::preview::marker_waypoints(start_waypoints);
        crate::map::rmg::preview::render_preview(&cells, &waypoints)
    }

    /// Persist the generated preview so the chooser's random-map row can show it.
    ///
    /// Failure is logged rather than propagated: the dialog's own preview box
    /// draws from memory, so a write failure costs the chooser thumbnail and
    /// nothing else.
    fn write_random_map_preview_file(
        state: &AppState,
        preview: &crate::map::rmg::preview::PreviewImage,
    ) {
        let Some(ra2_dir) = state
            .game_config
            .as_ref()
            .map(|config| config.paths.ra2_dir.clone())
        else {
            return;
        };
        let (Ok(width), Ok(height)) = (u16::try_from(preview.width), u16::try_from(preview.height))
        else {
            log::warn!(
                "random map: preview {}x{} does not fit a PCX header",
                preview.width,
                preview.height
            );
            return;
        };
        let rgb: Vec<u8> = preview
            .rgba
            .chunks_exact(4)
            .flat_map(|px| [px[0], px[1], px[2]])
            .collect();
        match crate::assets::pcx_file::encode_direct_rgb(width, height, &rgb) {
            Ok(encoded) => {
                let path = ra2_dir.join(RANDMAP_PREVIEW_FILE);
                if let Err(err) = std::fs::write(&path, encoded) {
                    log::warn!("random map: could not write {}: {err}", path.display());
                }
            }
            Err(err) => log::warn!("random map: could not encode the preview: {err}"),
        }
    }

    /// Where saved seeds live: the game directory, the same place the dialog's
    /// own working file is written.
    fn saved_seed_dir(state: &AppState) -> Option<std::path::PathBuf> {
        state
            .game_config
            .as_ref()
            .map(|config| config.paths.ra2_dir.clone())
    }

    fn skirmish_saved_seed_layout(
        state: &AppState,
        mode: SavedSeedMode,
    ) -> crate::ui::skirmish_shell::SavedSeedLayout {
        crate::ui::skirmish_shell::compute_saved_seed_layout(
            mode,
            state.render_width(),
            state.render_height(),
        )
    }

    fn handle_saved_seed_browser_mouse_down(state: &mut AppState) -> bool {
        let Some(mode) = state
            .skirmish_shell_state
            .saved_seed_browser
            .as_ref()
            .map(|browser| browser.mode)
        else {
            return false;
        };
        let layout = Self::skirmish_saved_seed_layout(state, mode);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let mut play_sound = false;
        if let Some(browser) = state.skirmish_shell_state.saved_seed_browser.as_mut() {
            match crate::ui::skirmish_shell::saved_seed_control_at(&layout, x, y) {
                Some(crate::ui::skirmish_shell::SavedSeedControl::List) => {
                    if let Some(row) = crate::ui::skirmish_shell::saved_seed_list_row_at(
                        &layout,
                        browser.entries.len(),
                        browser.top_index,
                        x,
                        y,
                    ) {
                        browser.select(row);
                    }
                }
                // The list selects on press; the buttons arm instead, so
                // dragging off one cancels it.
                Some(crate::ui::skirmish_shell::SavedSeedControl::Action)
                | Some(crate::ui::skirmish_shell::SavedSeedControl::Back0x686) => {
                    browser.pressed_control =
                        crate::ui::skirmish_shell::saved_seed_control_at(&layout, x, y);
                    play_sound = true;
                }
                _ => {}
            }
        }
        if play_sound {
            Self::play_main_menu_button_sound(state);
        }
        true
    }

    fn handle_saved_seed_browser_mouse_up(state: &mut AppState) -> bool {
        let Some(mode) = state
            .skirmish_shell_state
            .saved_seed_browser
            .as_ref()
            .map(|browser| browser.mode)
        else {
            return false;
        };
        let layout = Self::skirmish_saved_seed_layout(state, mode);
        let dir = Self::saved_seed_dir(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;

        use crate::ui::skirmish_shell::SavedSeedControl as SeedControl;
        use crate::ui::skirmish_shell::SavedSeedOutcome as Outcome;

        let outcome = {
            let Some(browser) = state.skirmish_shell_state.saved_seed_browser.as_mut() else {
                return false;
            };
            let pressed = browser.pressed_control.take();
            let released = crate::ui::skirmish_shell::saved_seed_control_at(&layout, x, y);
            if pressed.is_none() || pressed != released {
                return true;
            }
            match released {
                Some(SeedControl::Back0x686) => Some(Outcome::Close),
                Some(SeedControl::Action) => browser.action_outcome(),
                _ => None,
            }
        };
        let Some(outcome) = outcome else {
            return true;
        };
        let Some(dir) = dir else {
            state.skirmish_shell_state.saved_seed_browser = None;
            return true;
        };

        match outcome {
            Outcome::Close => state.skirmish_shell_state.saved_seed_browser = None,
            Outcome::Load(file_name) => {
                match crate::map::rmg::saved_seeds::load_saved_seed(&dir.join(&file_name)) {
                    Ok(options) => {
                        // Loading replaces the working options and invalidates
                        // any generated result, exactly as an edit would.
                        if let Some(modal) =
                            state.skirmish_shell_state.random_map_setup_modal.as_mut()
                        {
                            modal.options = options;
                            modal.generated = false;
                            modal.generated_preview = None;
                        }
                        state.skirmish_shell_state.saved_seed_browser = None;
                    }
                    Err(err) => log::warn!("saved seed: could not read {file_name}: {err}"),
                }
            }
            Outcome::Save(name) => {
                let options = state
                    .skirmish_shell_state
                    .random_map_setup_modal
                    .as_ref()
                    .map(|modal| modal.options.clone());
                let path = crate::map::rmg::saved_seeds::seed_path_for_name(&dir, &name);
                match (options, path) {
                    (Some(options), Some(path)) => {
                        if let Err(err) =
                            crate::map::rmg::saved_seeds::save_saved_seed(&path, &options)
                        {
                            log::warn!("saved seed: could not write {name}: {err}");
                        }
                        state.skirmish_shell_state.saved_seed_browser = None;
                    }
                    // A refused name leaves the browser open so the player can
                    // retype rather than silently losing the save.
                    _ => log::warn!("saved seed: {name} is not a usable save name"),
                }
            }
            Outcome::Delete(file_name) => {
                if let Err(err) =
                    crate::map::rmg::saved_seeds::delete_saved_seed(&dir.join(&file_name))
                {
                    log::warn!("saved seed: could not delete {file_name}: {err}");
                }
                // Delete stays open so several can be removed in one visit.
                if let Some(browser) = state.skirmish_shell_state.saved_seed_browser.as_mut() {
                    browser.remove_entry(&file_name);
                }
            }
        }
        true
    }

    /// Persist accepted random-map setup, refresh the sentinel record, and
    /// select it so launch generates from it.
    ///
    /// A failed write is fatal to the commit: `map_load` treats a missing seed
    /// file as "use defaults", so committing anyway would silently start a
    /// different map than the one the player configured.
    fn commit_random_map_setup(
        state: &mut AppState,
        options: &crate::map::rmg::RmgOptions,
    ) -> anyhow::Result<()> {
        let ra2_dir = state
            .game_config
            .as_ref()
            .map(|config| config.paths.ra2_dir.clone())
            .ok_or_else(|| anyhow::anyhow!("no game config; cannot locate the RA2 directory"))?;
        std::fs::write(ra2_dir.join(RANDMAP_SED_FILE), options.to_sed_bytes())?;

        let display = if options.description.is_empty() {
            RANDOM_MAP_DESCRIPTION_FALLBACK
        } else {
            options.description.as_str()
        };
        // Reuse the modal helper: it upserts the single sentinel, honours the
        // mode's random-map admission, and refreshes the filtered record list.
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
            return Ok(());
        };
        let index = modal.create_random_map(
            &mut state.skirmish_scenario_records,
            &state.skirmish_modes,
            display,
            options.num_players,
        );
        let mode_id = modal.selected_mode_id;
        let _ = modal;
        if let Some(index) = index {
            // The scenario record alone is not enough to play: committing a
            // selection resolves it against the loadable map list, which has no
            // entry for a seed file until one is put there.
            let entry = state.skirmish_scenario_records[index].to_map_menu_entry();
            match state
                .skirmish_shell_maps
                .iter()
                .position(|map| map.file_name.eq_ignore_ascii_case(&entry.file_name))
            {
                Some(existing) => state.skirmish_shell_maps[existing] = entry,
                None => state.skirmish_shell_maps.push(entry),
            }
            let selection = crate::ui::skirmish_shell::ChooseMapSelection {
                mode_id,
                record_index: Some(index),
            };
            let _ = Self::commit_choose_map_selection(state, selection);
            Self::close_choose_map_modal(state);
        }
        Ok(())
    }

    fn skirmish_random_map_setup_layout(
        state: &AppState,
    ) -> crate::ui::skirmish_shell::RandomMapSetupLayout {
        crate::ui::skirmish_shell::compute_random_map_setup_layout(
            state.render_width(),
            state.render_height(),
        )
    }

    fn handle_random_map_setup_mouse_down(state: &mut AppState) -> bool {
        let layout = Self::skirmish_random_map_setup_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() else {
            return false;
        };
        // An open list covers the rows under it, so it gets first refusal on the
        // click. Clicking anywhere else closes it without acting on whatever is
        // underneath, the way a dismissed dropdown behaves.
        if let Some(combo) = modal.open_combo {
            let items = crate::ui::skirmish_shell::setup_combo_items(combo);
            let on_list = crate::ui::skirmish_shell::random_map_setup_dropdown_row_at(
                &layout,
                combo.row(),
                items.len(),
                x,
                y,
            )
            .is_some();
            let on_face = crate::ui::skirmish_shell::random_map_setup_control_at(&layout, x, y)
                == Some(combo.control());
            if !on_list && !on_face {
                modal.open_combo = None;
                return true;
            }
            if on_list {
                return true;
            }
        }
        if let Some(control) = crate::ui::skirmish_shell::random_map_setup_control_at(&layout, x, y)
        {
            // The players slider is not a button: it acts on press, not on
            // release, so it never arms a pressed control.
            if control == crate::ui::skirmish_shell::RandomMapSetupControl::Players0x3eb {
                if modal.is_enabled(control) {
                    Self::press_setup_players_trackbar(modal, &layout, x, y);
                }
                return true;
            }
            // A disabled control swallows the click without arming a press, so
            // releasing over it cannot fire.
            if modal.is_enabled(control) {
                modal.pressed_control = Some(control);
                Self::play_main_menu_button_sound(state);
            }
            return true;
        }
        layout.dialog.contains(x, y)
    }

    /// Press behaviour for the players slider, mirroring the shell's other
    /// trackbars: grabbing the thumb starts a tracking drag, while a press on
    /// the rail jumps the value once and tracks nothing.
    fn press_setup_players_trackbar(
        modal: &mut crate::ui::skirmish_shell::RandomMapSetupModalState,
        layout: &crate::ui::skirmish_shell::RandomMapSetupLayout,
        x: i32,
        y: i32,
    ) {
        let rect = layout.control_rects[SETUP_PLAYERS_ROW];
        if !crate::ui::skirmish_shell::trackbar_mouse_allowed_y(rect, y) {
            return;
        }
        let pixel_offset = crate::ui::skirmish_shell::trackbar_pixel_offset(
            modal.options.num_players,
            SETUP_PLAYERS_MIN,
            SETUP_PLAYERS_MAX,
            SETUP_PLAYERS_STEP,
            rect,
        );
        if crate::ui::skirmish_shell::trackbar_thumb_hit(rect, pixel_offset, x, y) {
            modal.dragging_players_thumb = true;
        } else if rect.contains(x, y) {
            modal.set_num_players(crate::ui::skirmish_shell::trackbar_mouse_value(
                rect,
                x,
                SETUP_PLAYERS_MIN,
                SETUP_PLAYERS_MAX,
                SETUP_PLAYERS_STEP,
            ));
        }
    }

    fn handle_random_map_setup_mouse_move(state: &mut AppState) {
        let layout = Self::skirmish_random_map_setup_layout(state);
        let x = state.cursor_x.round() as i32;
        let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() else {
            return;
        };
        if !modal.dragging_players_thumb {
            return;
        }
        let rect = layout.control_rects[SETUP_PLAYERS_ROW];
        modal.set_num_players(crate::ui::skirmish_shell::trackbar_mouse_value(
            rect,
            x,
            SETUP_PLAYERS_MIN,
            SETUP_PLAYERS_MAX,
            SETUP_PLAYERS_STEP,
        ));
        state.window.request_redraw();
    }

    fn handle_random_map_setup_mouse_up(state: &mut AppState) -> bool {
        use crate::ui::skirmish_shell::RandomMapSetupControl as Control;

        let layout = Self::skirmish_random_map_setup_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        // RMGMD.INI drives the randomizer's vegetation bounds; without it the
        // derived vegetation collapses to zero and randomized maps lose trees.
        let settings = state
            .asset_manager
            .as_ref()
            .map(crate::map::rmg::RmgSettings::load)
            .unwrap_or_default();
        let description = state
            .csf
            .as_ref()
            .and_then(|csf| csf.get(RANDOM_MAP_DESCRIPTION_KEY))
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| RANDOM_MAP_DESCRIPTION_FALLBACK.to_string());
        let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() else {
            return false;
        };
        if modal.dragging_players_thumb {
            modal.dragging_players_thumb = false;
            return true;
        }
        // Releasing over an open list commits that entry. The press was never
        // armed for list clicks, so this has to run before the pressed check.
        if let Some(combo) = modal.open_combo {
            let items = crate::ui::skirmish_shell::setup_combo_items(combo);
            if let Some(index) = crate::ui::skirmish_shell::random_map_setup_dropdown_row_at(
                &layout,
                combo.row(),
                items.len(),
                x,
                y,
            ) {
                modal.set_combo_value(combo, items[index].value);
                return true;
            }
        }
        let pressed = modal.pressed_control.take();
        let released = crate::ui::skirmish_shell::random_map_setup_control_at(&layout, x, y);
        if pressed.is_none() || pressed != released {
            return layout.dialog.contains(x, y) || pressed.is_some();
        }

        let mut close_setup = false;
        // Generating needs the whole app state, so it cannot run while the modal
        // is mutably borrowed; the actions below only record what to do.
        let mut generate_requested = false;
        let mut accept_requested = false;
        let mut open_browser: Option<SavedSeedMode> = None;
        match released.expect("checked equal to pressed control") {
            Control::Randomize0x621 => {
                modal.randomize_options(
                    &settings,
                    &mut crate::ui::skirmish_shell::DialogRng::from_entropy(),
                    &description,
                );
            }
            Control::Generate0x620 => {
                modal.begin_generate();
                generate_requested = true;
            }
            Control::Ok0x6c5 => {
                // Accept generates first when nothing has been generated yet,
                // so the committed options always describe a map that exists.
                if matches!(
                    modal.accept(),
                    crate::ui::skirmish_shell::AcceptOutcome::NeedsGenerate
                ) {
                    modal.begin_generate();
                    generate_requested = true;
                }
                accept_requested = true;
            }
            Control::Cancel0x5c0 => {
                // Result 2 in the original: no seed file, no sentinel, no
                // selection change. The chooser underneath is left untouched.
                close_setup = true;
            }
            Control::Load0x6c2 => open_browser = Some(SavedSeedMode::Load),
            Control::Save0x6c3 => open_browser = Some(SavedSeedMode::Save),
            Control::Delete0x6c4 => open_browser = Some(SavedSeedMode::Delete),
            Control::MapType0x405
            | Control::Time0x3ea
            | Control::Theater0x407
            | Control::Size0x406
            | Control::Resources0x408 => {
                if let Some(combo) =
                    crate::ui::skirmish_shell::SetupCombo::from_control(released.expect("matched"))
                {
                    modal.toggle_combo(combo);
                }
            }
            // Dragging the players slider is a separate input mode; clicking the
            // track alone does not move it.
            Control::Players0x3eb => {}
        }

        if let Some(mode) = open_browser {
            let entries = Self::saved_seed_dir(state)
                .map(|dir| crate::map::rmg::saved_seeds::list_saved_seeds(&dir))
                .unwrap_or_default();
            state.skirmish_shell_state.saved_seed_browser =
                Some(SavedSeedBrowserState::open(mode, entries));
            return true;
        }
        if generate_requested {
            let options = state
                .skirmish_shell_state
                .random_map_setup_modal
                .as_ref()
                .map(|modal| modal.options.clone());
            let started = options.is_some_and(|options| {
                Self::start_random_map_generation(state, &options, accept_requested)
            });
            if !started {
                // Nothing will arrive, so the dialog must not be left sitting
                // in its generating state with every control disabled.
                log::warn!("random map: could not start generation for the configured options");
                if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
                    modal.finish_generate(None);
                }
            }
            // Accept, if it was asked for, is now the job's responsibility.
            return true;
        }
        if accept_requested {
            Self::accept_random_map_setup(state);
        }
        if close_setup {
            Self::close_random_map_setup(state);
        }
        true
    }

    fn handle_choose_map_modal_mouse_wheel(state: &mut AppState, lines: f32) -> bool {
        let layout = Self::skirmish_choose_map_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
            return false;
        };
        modal.handle_listbox_wheel(&layout, &state.skirmish_modes, x, y, lines)
    }

    fn sync_player_name_edit_scroll(state: &mut AppState) {
        let layout = Self::skirmish_shell_layout(state);
        let text_rect = crate::ui::skirmish_shell::player_name_edit_text_rect(layout.player_name);
        let prefix_width =
            state
                .bit_font
                .text_width(crate::ui::skirmish_shell::player_name_caret_prefix(
                    &state.skirmish_shell_state,
                ));
        crate::ui::skirmish_shell::update_player_name_scroll_for_caret(
            &mut state.skirmish_shell_state,
            text_rect.w,
            prefix_width,
        );
    }

    fn localized_status_help_text(state: &AppState, key: &str) -> String {
        state
            .csf
            .as_ref()
            .and_then(|csf| csf.get(key))
            .unwrap_or("")
            .to_string()
    }

    fn update_skirmish_shell_status_help(
        state: &mut AppState,
        layout: &crate::ui::skirmish_shell::SkirmishShellLayout,
        x: i32,
        y: i32,
    ) {
        let text = crate::ui::skirmish_shell::hovered_shell_control(
            layout,
            &state.skirmish_shell_state,
            &state.skirmish_shell_maps,
            x,
            y,
        )
        .and_then(crate::ui::skirmish_shell::status_help_key_for_hover)
        .map(|key| Self::localized_status_help_text(state, key))
        .unwrap_or_default();

        if crate::ui::skirmish_shell::set_status_help_text(&mut state.skirmish_shell_state, text) {
            state.window.request_redraw();
        }
    }

    fn localized_choose_map_status_help_text(
        state: &AppState,
        target: crate::ui::skirmish_shell::ChooseMapHoverTarget,
    ) -> String {
        if let crate::ui::skirmish_shell::ChooseMapHoverTarget::ModeListRow0x6eb { mode_index } =
            target
        {
            if let Some(mode) = state.skirmish_modes.get(mode_index) {
                if !mode.tooltip_key.is_empty() {
                    let text = Self::localized_status_help_text(state, &mode.tooltip_key);
                    if !text.is_empty() {
                        return text;
                    }
                }
            }
        }

        crate::ui::skirmish_shell::status_help_key_for_choose_map_hover(target)
            .map(|key| Self::localized_status_help_text(state, key))
            .unwrap_or_default()
    }

    fn update_choose_map_modal_status_help(
        state: &mut AppState,
        layout: &crate::ui::skirmish_shell::ChooseMapModalLayout,
        x: i32,
        y: i32,
    ) {
        let text = state
            .skirmish_shell_state
            .choose_map_modal
            .as_ref()
            .and_then(|modal| {
                crate::ui::skirmish_shell::hovered_choose_map_modal_control(
                    layout,
                    modal,
                    state.skirmish_modes.len(),
                    x,
                    y,
                )
            })
            .map(|target| Self::localized_choose_map_status_help_text(state, target))
            .unwrap_or_default();

        if crate::ui::skirmish_shell::set_status_help_text(&mut state.skirmish_shell_state, text) {
            state.window.request_redraw();
        }
    }

    fn handle_skirmish_shell_key_input(
        state: &mut AppState,
        code: KeyCode,
        text: Option<&str>,
    ) -> bool {
        if !state.skirmish_shell_state.player_name_edit.focused {
            return false;
        }

        let changed = match code {
            KeyCode::Backspace => crate::ui::skirmish_shell::handle_player_name_backspace(
                &mut state.skirmish_shell_state,
            ),
            KeyCode::Delete => crate::ui::skirmish_shell::handle_player_name_delete(
                &mut state.skirmish_shell_state,
            ),
            KeyCode::ArrowLeft => {
                crate::ui::skirmish_shell::handle_player_name_left(&mut state.skirmish_shell_state)
            }
            KeyCode::ArrowRight => {
                crate::ui::skirmish_shell::handle_player_name_right(&mut state.skirmish_shell_state)
            }
            KeyCode::Home => {
                crate::ui::skirmish_shell::handle_player_name_home(&mut state.skirmish_shell_state)
            }
            KeyCode::End => {
                crate::ui::skirmish_shell::handle_player_name_end(&mut state.skirmish_shell_state)
            }
            KeyCode::Tab => {
                crate::ui::skirmish_shell::handle_player_name_tab(&mut state.skirmish_shell_state)
            }
            _ => text.is_some_and(|text| {
                crate::ui::skirmish_shell::insert_player_name_text(
                    &mut state.skirmish_shell_state,
                    text,
                )
            }),
        };

        if changed {
            Self::sync_player_name_edit_scroll(state);
            state.window.request_redraw();
        }
        true
    }

    fn close_validation_modal_from_controller(state: &mut AppState) {
        crate::ui::skirmish_shell::dismiss_validation_modal(&mut state.skirmish_shell_state);
        if state.shell_controller.top_id() == Some(Self::validation_modal_dialog_id()) {
            state.shell_controller.pop();
        }
    }

    fn route_validation_modal_key(state: &mut AppState, key: ShellKey) -> bool {
        if state.skirmish_shell_state.validation_modal.is_none() {
            return false;
        }
        state
            .shell_controller
            .ensure_active(Self::validation_modal_dialog_id(), true);
        if !state.shell_controller.on_key(key) {
            return false;
        }
        Self::close_validation_modal_from_controller(state);
        state.window.request_redraw();
        true
    }

    fn route_validation_modal_mouse_down(state: &mut AppState) -> bool {
        if state.skirmish_shell_state.validation_modal.is_none() {
            return false;
        }
        let feed = Self::validation_modal_feed(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        state
            .shell_controller
            .ensure_active(Self::validation_modal_dialog_id(), true);
        state.shell_controller.on_pointer_down(x, y, &feed);
        state.window.request_redraw();
        true
    }

    fn route_validation_modal_mouse_up(state: &mut AppState) -> bool {
        if state.skirmish_shell_state.validation_modal.is_none() {
            return false;
        }
        let feed = Self::validation_modal_feed(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        state
            .shell_controller
            .ensure_active(Self::validation_modal_dialog_id(), true);
        let activated = state.shell_controller.on_pointer_up(x, y, &feed);
        if activated == Some(crate::ui::shell::modal::control::OK) {
            Self::close_validation_modal_from_controller(state);
        }
        state.window.request_redraw();
        true
    }

    fn handle_skirmish_shell_mouse_down(state: &mut AppState) {
        if Self::route_validation_modal_mouse_down(state) {
            return;
        }
        // The browser sits over the setup dialog, which sits over the
        // chooser, so input is offered in that order.
        if state.skirmish_shell_state.saved_seed_browser.is_some() {
            Self::handle_saved_seed_browser_mouse_down(state);
            return;
        }
        if state.skirmish_shell_state.random_map_setup_modal.is_some() {
            Self::handle_random_map_setup_mouse_down(state);
            return;
        }
        if Self::handle_choose_map_modal_mouse_down(state) {
            return;
        }
        let layout = Self::skirmish_shell_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        if crate::ui::skirmish_shell::player_name_edit_rect_hit(&layout, x, y) {
            crate::ui::skirmish_shell::focus_player_name_edit(&mut state.skirmish_shell_state);
            Self::sync_player_name_edit_scroll(state);
            state.window.request_redraw();
            return;
        }
        if state.skirmish_shell_state.player_name_edit.focused {
            crate::ui::skirmish_shell::blur_player_name_edit(&mut state.skirmish_shell_state);
            state.window.request_redraw();
        }
        if crate::ui::skirmish_shell::combo_dropdown_open(&state.skirmish_shell_state) {
            crate::ui::skirmish_shell::handle_option_mouse_down(
                &mut state.skirmish_shell_state,
                &layout,
                &state.skirmish_shell_maps,
                x,
                y,
            );
            Self::drain_skirmish_shell_ui_sounds(state);
            return;
        }
        state.skirmish_shell_state.pressed_owner_draw_button =
            crate::ui::skirmish_shell::hit_test_owner_draw_button(&layout, x, y);
        if state
            .skirmish_shell_state
            .pressed_owner_draw_button
            .is_some()
        {
            Self::play_main_menu_button_sound(state);
        } else {
            crate::ui::skirmish_shell::handle_option_mouse_down(
                &mut state.skirmish_shell_state,
                &layout,
                &state.skirmish_shell_maps,
                x,
                y,
            );
            Self::drain_skirmish_shell_ui_sounds(state);
        }
    }

    fn handle_skirmish_shell_mouse_up(state: &mut AppState, event_loop: &ActiveEventLoop) {
        if Self::route_validation_modal_mouse_up(state) {
            return;
        }
        // The browser sits over the setup dialog, which sits over the
        // chooser, so input is offered in that order.
        if state.skirmish_shell_state.saved_seed_browser.is_some() {
            Self::handle_saved_seed_browser_mouse_up(state);
            return;
        }
        if state.skirmish_shell_state.random_map_setup_modal.is_some() {
            Self::handle_random_map_setup_mouse_up(state);
            return;
        }
        if state.skirmish_shell_state.choose_map_modal.is_some() {
            Self::handle_choose_map_modal_mouse_up(state);
            return;
        }
        let layout = Self::skirmish_shell_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let released_button = crate::ui::skirmish_shell::hit_test_owner_draw_button(&layout, x, y);
        let pressed_button = state.skirmish_shell_state.pressed_owner_draw_button.take();
        state.skirmish_shell_last_painted_pressed_button = None;
        if pressed_button.is_some() && pressed_button == released_button {
            if let Some(button) = released_button {
                crate::ui::skirmish_shell::handle_option_mouse_up(&mut state.skirmish_shell_state);
                Self::drain_skirmish_shell_ui_sounds(state);
                let action = crate::ui::skirmish_shell::action_for_owner_draw_button(button);
                Self::handle_skirmish_shell_action(state, action, event_loop);
                return;
            }
        }

        crate::ui::skirmish_shell::handle_option_mouse_up(&mut state.skirmish_shell_state);
        Self::drain_skirmish_shell_ui_sounds(state);

        if released_button.is_some() {
            return;
        }

        let action = crate::ui::skirmish_shell::hit_test(&layout, x, y);
        Self::handle_skirmish_shell_action(state, action, event_loop);
    }

    fn handle_skirmish_shell_mouse_move(state: &mut AppState) {
        if state.skirmish_shell_state.random_map_setup_modal.is_some() {
            Self::handle_random_map_setup_mouse_move(state);
            return;
        }
        if state.skirmish_shell_state.choose_map_modal.is_some() {
            let layout = Self::skirmish_choose_map_layout(state);
            let x = state.cursor_x.round() as i32;
            let y = state.cursor_y.round() as i32;
            Self::update_choose_map_modal_status_help(state, &layout, x, y);
            return;
        }
        if state.skirmish_shell_state.validation_modal.is_some() {
            if crate::ui::skirmish_shell::clear_status_help_text(&mut state.skirmish_shell_state) {
                state.window.request_redraw();
            }
            return;
        }
        let layout = Self::skirmish_shell_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        Self::update_skirmish_shell_status_help(state, &layout, x, y);
        crate::ui::skirmish_shell::handle_option_mouse_move(
            &mut state.skirmish_shell_state,
            &layout,
            &state.skirmish_shell_maps,
            x,
            y,
        );
        Self::drain_skirmish_shell_ui_sounds(state);
    }

    fn handle_skirmish_shell_mouse_wheel(state: &mut AppState, lines: f32) -> bool {
        if state.skirmish_shell_state.validation_modal.is_some() {
            return true;
        }
        if state.skirmish_shell_state.choose_map_modal.is_some() {
            return Self::handle_choose_map_modal_mouse_wheel(state, lines);
        }
        let consumed = crate::ui::skirmish_shell::handle_option_mouse_wheel(
            &mut state.skirmish_shell_state,
            &state.skirmish_shell_maps,
            lines,
        );
        Self::drain_skirmish_shell_ui_sounds(state);
        consumed
    }

    /// Adapt the laid-out main-menu buttons into the shared controller's
    /// button-only input feed. Statics (title/website) are deliberately excluded,
    /// so the controller never hit-tests or hover-tracks them.
    fn main_menu_shell_button_feed(
        layout: &crate::ui::main_menu_shell::MainMenuShellLayout,
    ) -> Vec<crate::ui::shell::layout::LaidOutControl> {
        layout
            .buttons
            .iter()
            .map(|b| crate::ui::shell::layout::LaidOutControl {
                id: b.id.resource_id(),
                rect: b.rect,
            })
            .collect()
    }

    fn single_player_shell_button_feed(
        layout: &crate::ui::single_player_shell::SinglePlayerShellLayout,
    ) -> Vec<crate::ui::shell::layout::LaidOutControl> {
        layout
            .buttons
            .iter()
            .map(|b| crate::ui::shell::layout::LaidOutControl {
                id: b.id.resource_id(),
                rect: b.rect,
            })
            .collect()
    }

    /// Mirror the controller's press/hover state into the per-shell struct the
    /// render path reads. Slice-2/Slice-3 boundary: render is retired off these in
    /// Slice 3, after which the controller is the sole authority.
    fn mirror_shell_controller_to_main_menu(state: &mut AppState) {
        state.main_menu_shell_state.pressed_owner_draw_button = state
            .shell_controller
            .pressed()
            .and_then(crate::ui::main_menu_shell::MainMenuControlId::from_resource_id);
        state.main_menu_shell_state.hovered_owner_draw_button = state
            .shell_controller
            .hovered()
            .and_then(crate::ui::main_menu_shell::MainMenuControlId::from_resource_id);
    }

    fn mirror_shell_controller_to_single_player(state: &mut AppState) {
        state.single_player_shell_state.pressed_owner_draw_button = state
            .shell_controller
            .pressed()
            .and_then(crate::ui::single_player_shell::SinglePlayerControlId::from_resource_id);
        state.single_player_shell_state.hovered_owner_draw_button = state
            .shell_controller
            .hovered()
            .and_then(crate::ui::single_player_shell::SinglePlayerControlId::from_resource_id);
        state.single_player_shell_state.hover_started_at =
            state.shell_controller.hover_started_at();
    }

    fn handle_main_menu_shell_mouse_down(state: &mut AppState) {
        let layout = crate::ui::main_menu_shell::compute_layout(
            state.gpu.config.width,
            state.gpu.config.height,
        );
        let feed = Self::main_menu_shell_button_feed(&layout);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x00E2), false);
        state.shell_controller.on_pointer_down(x, y, &feed);
        let pressed = state.shell_controller.pressed().is_some();
        Self::mirror_shell_controller_to_main_menu(state);
        // The original plays the button sound on mouse-DOWN over a button (not on
        // release); `pressed` is button-only by construction, so the website static
        // never triggers it.
        if pressed {
            Self::play_main_menu_button_sound(state);
        }
    }

    fn handle_main_menu_shell_mouse_move(state: &mut AppState) {
        let layout = crate::ui::main_menu_shell::compute_layout(
            state.gpu.config.width,
            state.gpu.config.height,
        );
        let feed = Self::main_menu_shell_button_feed(&layout);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x00E2), false);
        state.shell_controller.on_pointer_move(x, y, &feed);
        Self::mirror_shell_controller_to_main_menu(state);
    }

    fn handle_main_menu_shell_mouse_up(state: &mut AppState, event_loop: &ActiveEventLoop) {
        let layout = crate::ui::main_menu_shell::compute_layout(
            state.gpu.config.width,
            state.gpu.config.height,
        );
        let feed = Self::main_menu_shell_button_feed(&layout);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x00E2), false);
        let activated = state.shell_controller.on_pointer_up(x, y, &feed);
        Self::mirror_shell_controller_to_main_menu(state);
        if let Some(action) = activated
            .and_then(crate::ui::main_menu_shell::MainMenuControlId::from_resource_id)
            .map(crate::ui::main_menu_shell::action_for_control)
        {
            Self::handle_main_menu_shell_action(state, action, event_loop);
        }
    }

    /// The quit-confirm (0x120) modal's OK/Cancel button feed: resource-id'd pixel
    /// rects from the centered modal layout at the live screen size.
    fn exit_confirm_modal_feed(state: &AppState) -> Vec<crate::ui::shell::layout::LaidOutControl> {
        use crate::ui::shell::layout::LaidOutControl;
        use crate::ui::shell::modal;
        let layout = modal::quit_confirm_layout(
            state.gpu.config.width as i32,
            state.gpu.config.height as i32,
        );
        vec![
            LaidOutControl {
                id: modal::control::OK,
                rect: layout.ok,
            },
            LaidOutControl {
                id: modal::control::CANCEL,
                rect: layout.cancel,
            },
        ]
    }

    /// Persist the user-tunable settings the engine currently tracks to
    /// `RA2MD.INI`, preserving the file's other keys and sections. Invoked on
    /// quit-confirm OK strictly BEFORE the app tears down, matching the
    /// original writing options before exit. Today only `[Audio] ScoreVolume`
    /// (the live music volume, already read at boot) round-trips; further
    /// sections are added as the engine grows to model them. A write failure is
    /// logged, never fatal — a quit must not be blocked by a settings error.
    fn persist_settings_on_quit(state: &AppState) {
        let Some(config) = state.game_config.as_ref() else {
            return;
        };
        let Some(player) = state.music_player.as_ref() else {
            return;
        };
        if let Err(err) =
            crate::audio::music::write_score_volume_to_ra2md(&config.paths.ra2_dir, player.volume())
        {
            log::warn!("Failed to persist settings to RA2MD.INI on quit: {err}");
        }
    }

    /// Begin the graceful quit cascade from the main-menu Exit-confirm OK. The
    /// caller persists settings FIRST (so the captured volume is pre-fade), then
    /// calls this instead of exiting immediately; `render_frame` drives it to
    /// completion and then exits the event loop.
    fn start_quit_cascade(state: &mut AppState) {
        let start_volume = state.music_player.as_ref().map_or(0.0, |p| p.volume());
        state.quit_cascade = Some(crate::app_quit_cascade::QuitCascade::start(
            Instant::now(),
            start_volume,
        ));
    }

    fn handle_exit_confirm_modal_mouse_down(state: &mut AppState) {
        let feed = Self::exit_confirm_modal_feed(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        if state.shell_controller.top_id() != Some(crate::ui::shell::descriptor::DialogId(0x0120)) {
            return;
        }
        state.shell_controller.on_pointer_down(x, y, &feed);
    }

    fn handle_exit_confirm_modal_mouse_up(state: &mut AppState) {
        let feed = Self::exit_confirm_modal_feed(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        if state.shell_controller.top_id() != Some(crate::ui::shell::descriptor::DialogId(0x0120)) {
            return;
        }
        let activated = state.shell_controller.on_pointer_up(x, y, &feed);
        match activated {
            // OK -> quit (result 0). Persist settings to RA2MD.INI BEFORE teardown
            // (4b-i), then run the graceful cascade (music fade → trailing-voice
            // wait → hard stop → exit) via render_frame instead of exiting
            // immediately. The screen fade-to-black is sub-step 4b-ii-b.
            Some(id) if id == crate::ui::shell::modal::control::OK => {
                Self::persist_settings_on_quit(state);
                Self::close_exit_confirm_modal_from_controller(state);
                Self::start_quit_cascade(state);
            }
            // Cancel (control 2) -> stay; close the modal via the controller
            // pop (D-B3) so mouse and Esc converge on the same teardown.
            Some(id) if id == crate::ui::shell::modal::control::CANCEL => {
                Self::close_exit_confirm_modal_from_controller(state);
                state.window.request_redraw();
            }
            _ => {}
        }
    }

    fn handle_single_player_shell_mouse_down(state: &mut AppState) {
        let layout = Self::single_player_shell_layout(state);
        let feed = Self::single_player_shell_button_feed(&layout);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let load_enabled = state.single_player_shell_state.load_saved_game_enabled;
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x0100), false);
        // Refresh the Load Saved Game disabled guard before the gesture; the
        // override persists through the matching release (ensure_active only resets
        // on a dialog change, never mid-gesture).
        state.shell_controller.set_disabled(
            crate::ui::single_player_shell::SinglePlayerControlId::LoadSavedGame0x689.resource_id(),
            !load_enabled,
        );
        state.shell_controller.on_pointer_down(x, y, &feed);
        let pressed = state.shell_controller.pressed().is_some();
        Self::mirror_shell_controller_to_single_player(state);
        if pressed {
            Self::play_main_menu_button_sound(state);
        }
    }

    fn handle_single_player_shell_mouse_move(state: &mut AppState) {
        let layout = Self::single_player_shell_layout(state);
        let feed = Self::single_player_shell_button_feed(&layout);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x0100), false);
        // Hover path is enable-UNfiltered: a disabled Load Saved Game still
        // hover-tracks and arms its tooltip timer, exactly as before.
        state.shell_controller.on_pointer_move(x, y, &feed);
        Self::mirror_shell_controller_to_single_player(state);
    }

    fn handle_single_player_shell_mouse_up(state: &mut AppState) {
        let layout = Self::single_player_shell_layout(state);
        let feed = Self::single_player_shell_button_feed(&layout);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let load_enabled = state.single_player_shell_state.load_saved_game_enabled;
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x0100), false);
        state.shell_controller.set_disabled(
            crate::ui::single_player_shell::SinglePlayerControlId::LoadSavedGame0x689.resource_id(),
            !load_enabled,
        );
        let activated = state.shell_controller.on_pointer_up(x, y, &feed);
        Self::mirror_shell_controller_to_single_player(state);
        if let Some(action) = activated
            .and_then(crate::ui::single_player_shell::SinglePlayerControlId::from_resource_id)
            .map(crate::ui::single_player_shell::action_for_control)
        {
            Self::handle_single_player_shell_action(state, action);
        }
    }

    fn play_main_menu_button_sound(state: &mut AppState) {
        let sound_id = state
            .rules
            .as_ref()
            .and_then(|rules| rules.general.gui_main_button_sound.as_deref())
            .map(str::to_string);
        Self::play_shell_ui_sound_by_id(state, sound_id.as_deref());
    }

    /// Play the shell first-paint slide-in cue ([AudioVisual] GUIMoveInSound,
    /// stock `MenuSlideIn`), once at the start of each allow-listed shell
    /// dialog's controls-reveal slide. A no-op when the key is empty/unset.
    pub(crate) fn play_shell_slide_in_sound(state: &mut AppState) {
        let sound_id = state
            .rules
            .as_ref()
            .and_then(|rules| rules.general.gui_move_in_sound.as_deref())
            .map(str::to_string);
        Self::play_shell_ui_sound_by_id(state, sound_id.as_deref());
    }

    /// Active-retail `ShellButtonSlideSound` completion hook. The stock key is
    /// empty in both rules layers, so this named edge intentionally has no
    /// audible output in the exact stock route.
    pub(crate) fn play_shell_slide_completion_sound(_state: &mut AppState) {}

    fn maintain_main_menu_intro(state: &mut AppState) {
        if state.screen != GameScreen::MainMenu || state.quit_cascade.is_some() {
            return;
        }
        if let (Some(player), Some(assets)) = (&mut state.music_player, &state.asset_manager) {
            player.play_menu_theme(assets);
            player.update(assets);
        }
    }

    fn drain_skirmish_shell_ui_sounds(state: &mut AppState) {
        let _trackbar_parent_notifications =
            state.skirmish_shell_state.drain_pending_trackbar_hscrolls();
        for sound in
            crate::ui::skirmish_shell::drain_pending_ui_sounds(&mut state.skirmish_shell_state)
        {
            Self::play_skirmish_shell_ui_sound(state, sound);
        }
    }

    pub(crate) fn play_skirmish_shell_generic_click_sound(state: &mut AppState) {
        Self::play_skirmish_shell_ui_sound(
            state,
            crate::ui::skirmish_shell::SkirmishShellUiSound::GenericClick,
        );
    }

    fn skirmish_shell_ui_sound_id<'a>(
        general: &'a crate::rules::ruleset::GeneralRules,
        sound: crate::ui::skirmish_shell::SkirmishShellUiSound,
    ) -> Option<&'a str> {
        match sound {
            crate::ui::skirmish_shell::SkirmishShellUiSound::GuiCheckboxSound => {
                general.gui_checkbox_sound.as_deref()
            }
            crate::ui::skirmish_shell::SkirmishShellUiSound::GenericClick => {
                general.generic_click_sound.as_deref()
            }
            crate::ui::skirmish_shell::SkirmishShellUiSound::GuiComboOpenSound => {
                general.gui_combo_open_sound.as_deref()
            }
            crate::ui::skirmish_shell::SkirmishShellUiSound::GuiComboCloseSound => {
                general.gui_combo_close_sound.as_deref()
            }
        }
    }

    fn play_skirmish_shell_ui_sound(
        state: &mut AppState,
        sound: crate::ui::skirmish_shell::SkirmishShellUiSound,
    ) {
        let sound_id = state
            .rules
            .as_ref()
            .and_then(|rules| Self::skirmish_shell_ui_sound_id(&rules.general, sound))
            .map(str::to_string);
        Self::play_shell_ui_sound_by_id(state, sound_id.as_deref());
    }

    pub(crate) fn play_shell_ui_sound_by_id(state: &mut AppState, sound_id: Option<&str>) {
        let Some(sound_id) = sound_id else {
            return;
        };
        let (Some(sfx), Some(assets)) = (&mut state.sfx_player, &state.asset_manager) else {
            return;
        };
        sfx.play_sound(
            sound_id,
            &state.sound_registry,
            assets,
            &state.audio_indices,
        );
    }

    fn handle_single_player_shell_action(
        state: &mut AppState,
        action: crate::ui::single_player_shell::SinglePlayerShellAction,
    ) {
        use crate::ui::single_player_shell::SinglePlayerShellAction;

        match action {
            SinglePlayerShellAction::None => {}
            SinglePlayerShellAction::Skirmish => {
                Self::enter_native_skirmish_from_single_player(state);
            }
            SinglePlayerShellAction::MainMenu => {
                Self::close_single_player_shell(state);
            }
            SinglePlayerShellAction::LoadSavedGame => {
                if state.single_player_shell_state.load_saved_game_enabled {
                    state.show_save_load_panel = true;
                    state.save_list_cache.invalidate();
                }
            }
            SinglePlayerShellAction::NewCampaign => {
                // The original opens the campaign selector (Allied/Soviet +
                // difficulty). Open the selector shell; the side/difficulty ->
                // scenario mapping and first-mission launch are not decoded yet.
                state.campaign_select =
                    Some(crate::ui::main_menu_dialogs::CampaignSelectState::default());
            }
        }
    }

    fn handle_main_menu_shell_action(
        state: &mut AppState,
        action: crate::ui::main_menu_shell::MainMenuShellAction,
        event_loop: &ActiveEventLoop,
    ) {
        use crate::ui::main_menu_shell::MainMenuShellAction;

        let _ = event_loop;
        match action {
            MainMenuShellAction::None => {}
            // The original pops a confirm message box here; it does NOT quit on
            // the first Exit click. Quitting happens only on confirm.
            MainMenuShellAction::ExitGame => Self::open_exit_confirm_modal(state),
            MainMenuShellAction::SinglePlayer => {
                Self::open_single_player_shell(state);
            }
            MainMenuShellAction::Options => {
                state.options_dialog =
                    Some(crate::ui::main_menu_dialogs::OptionsDialogState::default());
            }
            MainMenuShellAction::MoviesAndCredits => {
                state.movies_credits_dialog =
                    Some(crate::ui::main_menu_dialogs::MoviesCreditsDialogState::default());
            }
            MainMenuShellAction::WwOnline
            | MainMenuShellAction::Network
            | MainMenuShellAction::YuriWebsite => {
                log::info!(
                    "Main-menu shell action {:?} is preserved but downstream dialog is not implemented yet",
                    action
                );
            }
        }
    }

    /// Open the Exit-Game confirm message box, resolving its labels from CSF.
    fn open_exit_confirm_modal(state: &mut AppState) {
        let csf = |key: &str, fallback: &str| Self::csf_label(state, key, fallback);
        let modal = crate::ui::main_menu_dialogs::ExitConfirmModalState::open(&csf);
        // The SHP modal sources PUDLGBGN/MNBTTN from the skirmish chrome atlas; load
        // it on demand so the quit-confirm renders straight from the main menu.
        Self::ensure_skirmish_shell_chrome(state);
        // Host the modal as a TRUE LIFO push over the active shell (D-B3):
        // teardown pops back to it with focus restored. (ensure_active would
        // reset_to-clobber the stack — the prior "0x120 over 0xE2" comment
        // described behavior that never happened.)
        if state.shell_controller.top_id() != Some(crate::ui::shell::descriptor::DialogId(0x0120)) {
            state
                .shell_controller
                .push(crate::ui::shell::descriptor::DialogId(0x0120), true);
        }
        state.exit_confirm_modal = Some(modal);
    }

    /// Whether any main-menu modal dialog is currently open. Used to route
    /// keyboard/mouse to the modal first.
    pub(crate) fn main_menu_dialog_open(state: &AppState) -> bool {
        state.main_menu_dialog_open()
    }

    /// Close the egui-only main-menu dialogs (options/movies/campaign — never on
    /// the controller stack). The exit-confirm modal closes through
    /// close_exit_confirm_modal_from_controller (D-B3).
    pub(crate) fn close_main_menu_dialogs(state: &mut AppState) {
        state.exit_confirm_modal = None;
        state.options_dialog = None;
        state.movies_credits_dialog = None;
        state.campaign_select = None;
    }

    /// Controller-routed exit-confirm teardown (D-B3): dismiss the modal UI
    /// state, then LIFO-pop its 0x120 instance so focus returns to the shell
    /// beneath. Mirrors `close_validation_modal_from_controller` — every Esc
    /// and mouse close path converges here.
    fn close_exit_confirm_modal_from_controller(state: &mut AppState) {
        state.exit_confirm_modal = None;
        if state.shell_controller.top_id() == Some(crate::ui::shell::descriptor::DialogId(0x0120)) {
            state.shell_controller.pop();
        }
    }

    fn route_exit_confirm_modal_key(state: &mut AppState, key: ShellKey) -> bool {
        if state.exit_confirm_modal.is_none() {
            return false;
        }
        if !state.shell_controller.on_key(key) {
            return false;
        }
        Self::close_exit_confirm_modal_from_controller(state);
        state.window.request_redraw();
        true
    }

    /// Draw whichever main-menu modal dialog is open in the current egui frame
    /// and apply its outcome. Returns `true` when the player has confirmed
    /// quitting, so the caller should exit the event loop.
    /// Draw whichever egui main-menu modal dialog is open. `render_exit_confirm_egui`
    /// is true only on the degraded egui fallback path (where the SHP shell — and
    /// thus the SHP quit-confirm modal — is unavailable); the normal SHP shell path
    /// passes false and renders the quit-confirm as an SHP overlay instead.
    fn draw_main_menu_dialogs(state: &mut AppState, render_exit_confirm_egui: bool) -> bool {
        use crate::ui::main_menu_dialogs as dialogs;

        if render_exit_confirm_egui {
            if let Some(modal) = state.exit_confirm_modal.clone() {
                match dialogs::draw_exit_confirm_modal(&state.egui.ctx, &modal) {
                    dialogs::ExitConfirmAction::Confirm => {
                        // Persist BEFORE teardown (4b-i), then start the graceful
                        // cascade. Return false (not true) so exit is owned by the
                        // cascade; this degraded egui-fallback path runs the audio
                        // phases (the SHP fade overlay is unavailable here).
                        Self::persist_settings_on_quit(state);
                        state.exit_confirm_modal = None;
                        Self::start_quit_cascade(state);
                        return false;
                    }
                    dialogs::ExitConfirmAction::Cancel => {
                        state.exit_confirm_modal = None;
                    }
                    dialogs::ExitConfirmAction::None => {}
                }
                return false;
            }
        }

        if state.options_dialog.is_some() {
            let csf = |key: &str, fallback: &str| Self::csf_label(state, key, fallback);
            if matches!(
                dialogs::draw_options_dialog(&state.egui.ctx, &csf),
                dialogs::OptionsDialogAction::Close
            ) {
                state.options_dialog = None;
            }
            return false;
        }

        if state.movies_credits_dialog.is_some() {
            let csf = |key: &str, fallback: &str| Self::csf_label(state, key, fallback);
            match dialogs::draw_movies_credits_dialog(&state.egui.ctx, &csf) {
                dialogs::MoviesCreditsAction::Back => state.movies_credits_dialog = None,
                // Sneak Preview / Movies / Credits playback is not implemented;
                // the picker would derive entries only from artmd.ini [Movies],
                // which is not parsed yet. No-op for now.
                dialogs::MoviesCreditsAction::SneakPreview
                | dialogs::MoviesCreditsAction::Movies
                | dialogs::MoviesCreditsAction::Credits
                | dialogs::MoviesCreditsAction::None => {}
            }
            return false;
        }

        if let Some(mut campaign) = state.campaign_select.take() {
            let csf = |key: &str, fallback: &str| Self::csf_label(state, key, fallback);
            let action = dialogs::draw_campaign_select(&state.egui.ctx, &csf, &mut campaign);
            match action {
                // The side/difficulty -> scenario mapping and first-mission
                // launch are not decoded; Back returns to the SP shell.
                dialogs::CampaignSelectAction::Back => {}
                dialogs::CampaignSelectAction::None => {
                    state.campaign_select = Some(campaign);
                }
            }
            return false;
        }

        false
    }

    fn invalidate_main_menu_movie_if_base_changed(state: &mut AppState) {
        let movie_base =
            crate::ui::main_menu_shell::movie_base_for_screen_width(state.gpu.config.width);
        if state
            .main_menu_movie_identity
            .is_some_and(|identity| identity.base() != movie_base)
        {
            crate::app_main_menu_shell_render::clear_ra2ts_movie_session(state);
        }
    }

    pub fn new() -> Self {
        Self {
            state: None,
            shell_capture: None,
            tactical_capture: None,
        }
    }

    pub fn new_shell_capture(request: crate::app_shell_capture::ShellCaptureRequest) -> Self {
        Self {
            state: None,
            shell_capture: Some(crate::app_shell_capture::ShellCaptureSession::new(request)),
            tactical_capture: None,
        }
    }

    pub fn new_tactical_capture(request: crate::app_launch::TacticalCaptureRequest) -> Self {
        Self {
            state: None,
            shell_capture: None,
            tactical_capture: Some(
                crate::app_tactical_capture::session::TacticalCaptureSession::new(request),
            ),
        }
    }

    pub fn finish_capture(&mut self) -> Result<()> {
        match (self.shell_capture.as_mut(), self.tactical_capture.as_mut()) {
            (Some(session), None) => session.take_outcome(),
            (None, Some(session)) => session.take_outcome(),
            (None, None) => Ok(()),
            (Some(_), Some(_)) => anyhow::bail!("multiple capture modes were active"),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        log::info!("Application resumed — creating window and GPU context");
        let capture_dimensions = match (self.shell_capture.as_ref(), self.tactical_capture.as_ref())
        {
            (Some(session), None) => Some((session.request().width(), session.request().height())),
            (None, Some(session)) => Some((session.request().width(), session.request().height())),
            (None, None) => None,
            (Some(_), Some(_)) => {
                log::error!("Shell and tactical capture modes are mutually exclusive");
                event_loop.exit();
                return;
            }
        };
        match Self::initialize(event_loop, capture_dimensions) {
            Ok(mut state) => {
                if let Some(session) = self.shell_capture.as_mut() {
                    session.prepare_state(&mut state);
                }
                if let Some(session) = self.tactical_capture.as_mut()
                    && let Err(err) = session.prepare_state(&mut state)
                {
                    log::error!("Tactical capture preparation failed: {err:#}");
                    session.fail(format!("tactical capture preparation failed: {err:#}"));
                    event_loop.exit();
                    return;
                }
                self.state = Some(state);
                log::info!("Initialization complete — showing main menu");
            }
            Err(err) => {
                log::error!("Failed to initialize: {:#}", err);
                if let Some(session) = self.shell_capture.as_mut() {
                    session.fail(format!("shell capture initialization failed: {err:#}"));
                }
                if let Some(session) = self.tactical_capture.as_mut() {
                    session.fail(format!("tactical capture initialization failed: {err:#}"));
                }
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let shell_capture_active = self.shell_capture.is_some();
        let tactical_capture_active = self.tactical_capture.is_some();
        let Some(state) = &mut self.state else { return };

        // This checkpoint is driven entirely from `about_to_wait`. Focused
        // input would contaminate its hidden, no-input production oracle.
        if tactical_capture_active {
            let session = self
                .tactical_capture
                .as_mut()
                .expect("tactical capture session exists");
            match event {
                WindowEvent::CloseRequested => {
                    session.fail("tactical-capture window closed before bundle completion");
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    let expected =
                        PhysicalSize::new(session.request().width(), session.request().height());
                    if size != expected {
                        session.fail(format!(
                            "tactical-capture surface resized to {}x{}, expected {}x{}",
                            size.width, size.height, expected.width, expected.height
                        ));
                        event_loop.exit();
                    } else {
                        Self::resize_surface_for_window_size(state, size);
                    }
                }
                WindowEvent::Focused(true) => {
                    session.record_focus_violation();
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput { .. }
                | WindowEvent::ModifiersChanged(_)
                | WindowEvent::Ime(_)
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::PanGesture { .. }
                | WindowEvent::DoubleTapGesture { .. }
                | WindowEvent::RotationGesture { .. }
                | WindowEvent::TouchpadPressure { .. }
                | WindowEvent::AxisMotion { .. }
                | WindowEvent::Touch(_)
                | WindowEvent::DroppedFile(_)
                | WindowEvent::HoveredFile(_)
                | WindowEvent::HoveredFileCancelled => {
                    session.record_input_violation("window");
                    event_loop.exit();
                }
                _ => {}
            }
            return;
        }

        // A hidden Windows HWND is not guaranteed to receive WM_PAINT, so
        // capture frames are pumped from `about_to_wait`, never window input.
        if shell_capture_active {
            let session = self.shell_capture.as_mut().expect("capture session exists");
            match event {
                WindowEvent::CloseRequested => {
                    log::info!("Shell-capture window close requested");
                    session.fail("shell-capture window closed before bundle completion");
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    let expected =
                        PhysicalSize::new(session.request().width(), session.request().height());
                    if size != expected {
                        session.fail(format!(
                            "shell-capture surface resized to {}x{}, expected {}x{}",
                            size.width, size.height, expected.width, expected.height
                        ));
                        event_loop.exit();
                    } else {
                        Self::resize_surface_for_window_size(state, size);
                    }
                }
                WindowEvent::Focused(true) => {
                    session.fail("shell-capture window unexpectedly received focus");
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput { .. }
                | WindowEvent::ModifiersChanged(_)
                | WindowEvent::Ime(_)
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::PanGesture { .. }
                | WindowEvent::DoubleTapGesture { .. }
                | WindowEvent::RotationGesture { .. }
                | WindowEvent::TouchpadPressure { .. }
                | WindowEvent::AxisMotion { .. }
                | WindowEvent::Touch(_)
                | WindowEvent::DroppedFile(_)
                | WindowEvent::HoveredFile(_)
                | WindowEvent::HoveredFileCancelled => {
                    session.fail("shell-capture received unexpected window input");
                    event_loop.exit();
                }
                _ => {}
            }
            return;
        }

        // Always let egui see the event first for input handling.
        let egui_response: egui_winit::EventResponse =
            state.egui.on_window_event(&state.window, &event);

        // In InGame mode, egui only renders non-interactive overlays
        // (mission banner). The custom sidebar handles its own hit-testing.
        // Ignore egui's `consumed` flag in-game to avoid stale UI state
        // from the Loading screen blocking mouse/keyboard input.
        // Exception: when paused or save/load panel is open, egui renders
        // interactive content.
        let egui_consumed: bool = egui_response.consumed
            && (state.screen != GameScreen::InGame || state.paused || state.show_save_load_panel);

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested");
                if Self::native_skirmish_shell_active(state) {
                    // Pump/quit exits write the last durable snapshot after
                    // teardown, without a fresh control pack or RNG draw.
                    Self::teardown_skirmish_shell_for_start(state);
                    state.offline_skirmish_runtime.persist_snapshot();
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                Self::resize_surface_for_window_size(state, size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    // ESC always reaches the handler when in-game (even when paused)
                    // so the player can toggle pause regardless of egui focus.
                    let is_escape: bool =
                        code == KeyCode::Escape && event.state.is_pressed() && !event.repeat;
                    let in_game: bool = state.screen == GameScreen::InGame;

                    if crate::app_shell_transition::blocks_shell_input(state) {
                        return;
                    }

                    // A main-menu modal dialog (exit confirm, options, movies,
                    // campaign select) takes ESC first: close it and stay,
                    // never propagating to the shell-close handlers below.
                    // The exit-confirm modal routes through the controller
                    // (on_key → LIFO pop, D-B3); the egui-only dialogs are
                    // not on the stack and keep the direct close.
                    if Self::main_menu_dialog_open(state) {
                        if is_escape {
                            if state.exit_confirm_modal.is_some() {
                                if !Self::route_exit_confirm_modal_key(state, ShellKey::Escape) {
                                    // Defensive: on_key only fails with an
                                    // empty route — still close consistently.
                                    Self::close_exit_confirm_modal_from_controller(state);
                                    state.window.request_redraw();
                                }
                            } else {
                                Self::close_main_menu_dialogs(state);
                                state.window.request_redraw();
                            }
                        }
                        return;
                    }

                    if Self::native_skirmish_shell_active(state)
                        && event.state.is_pressed()
                        && !event.repeat
                        && Self::shell_key_for_code(code)
                            .is_some_and(|key| Self::route_validation_modal_key(state, key))
                    {
                        return;
                    }

                    if Self::native_skirmish_shell_active(state) && is_escape {
                        if state.skirmish_shell_state.choose_map_modal.is_some() {
                            // Native chooser `0x6B` has no verified Escape
                            // dismissal. Consume the key without applying the
                            // Cancel transaction or changing its selection.
                            state.window.request_redraw();
                            return;
                        }
                        Self::handle_skirmish_shell_action(
                            state,
                            crate::ui::skirmish_shell::SkirmishShellAction::BackOrExit,
                            event_loop,
                        );
                        state.window.request_redraw();
                        return;
                    }

                    if Self::single_player_shell_active(state) && is_escape {
                        Self::close_single_player_shell(state);
                        state.window.request_redraw();
                        return;
                    }

                    if Self::native_skirmish_shell_active(state)
                        && event.state.is_pressed()
                        && !is_escape
                        && Self::handle_skirmish_shell_key_input(state, code, event.text.as_deref())
                    {
                        return;
                    }

                    if in_game && (is_escape || !egui_consumed) {
                        if event.state.is_pressed() && !event.repeat {
                            if code == KeyCode::KeyN {
                                crate::app_loading::clear_match_startup_state(state);
                            }
                            app_input::handle_hotkey_pressed(state, code);
                        }
                    }
                    // Track held keys only when not paused.
                    if in_game && !egui_consumed {
                        if event.state.is_pressed() {
                            state.keys_held.insert(code);
                        } else {
                            state.keys_held.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // When upscaling, remap window coordinates to render-target coordinates.
                let use_render_source_coords = state.upscale_pass.is_some()
                    && (state.screen == GameScreen::InGame
                        || state.screen == GameScreen::SpawnPick);
                let (sx, sy) = if use_render_source_coords {
                    (
                        state.render_width() as f32 / state.gpu.config.width as f32,
                        state.render_height() as f32 / state.gpu.config.height as f32,
                    )
                } else {
                    (1.0, 1.0)
                };
                state.cursor_x = position.x as f32 * sx;
                state.cursor_y = position.y as f32 * sy;
                // Keep OS cursor hidden whenever the software cursor is active.
                if state.use_software_cursor() {
                    state.window.set_cursor_visible(false);
                }
                // Shared tooltip service: every move restarts the show delay
                // and hides a visible tip (study S1).
                crate::app_tooltips::on_mouse_move(state);
                if crate::app_shell_transition::blocks_shell_input(state) {
                    return;
                }
                if !egui_consumed
                    && (state.screen == GameScreen::InGame || state.screen == GameScreen::SpawnPick)
                {
                    app_input::handle_cursor_moved_in_game(state);
                }
                if !egui_consumed && Self::native_skirmish_shell_active(state) {
                    Self::handle_skirmish_shell_mouse_move(state);
                }
                if !egui_consumed && Self::single_player_shell_active(state) {
                    Self::handle_single_player_shell_mouse_move(state);
                }
                if !egui_consumed
                    && state.screen == GameScreen::MainMenu
                    && !state.main_menu_shell_failed
                    && !state.main_menu_show_skirmish_setup
                    && !Self::single_player_shell_active(state)
                    && !Self::native_skirmish_shell_active(state)
                    // While the SHP quit-confirm modal owns the controller, the menu
                    // move handler must not re-activate 0xE2 and reset the gesture.
                    && state.exit_confirm_modal.is_none()
                {
                    Self::handle_main_menu_shell_mouse_move(state);
                }
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                // Keep OS cursor hidden on click events (not just CursorMoved).
                // Without this, rapid clicks without mouse movement let the OS
                // cursor flash visible between WM_SETCURSOR and the next render.
                if state.use_software_cursor() {
                    state.window.set_cursor_visible(false);
                }
                // Any button press/release kills a visible tooltip + pending
                // timer (all buttons incl. middle — study S1).
                crate::app_tooltips::on_button_event(state);
                if crate::app_shell_transition::blocks_shell_input(state) {
                    return;
                }
                // While a main-menu modal dialog is open, route the click to the
                // SHP quit-confirm modal's OK/Cancel hit-test on the normal shell
                // path; the egui fallback and the other egui dialogs (options/movies/
                // campaign) were already handled by egui above.
                if Self::main_menu_dialog_open(state) {
                    if state.exit_confirm_modal.is_some()
                        && state.screen == GameScreen::MainMenu
                        && !state.main_menu_shell_failed
                        && !state.main_menu_show_skirmish_setup
                        && button == MouseButton::Left
                    {
                        if btn_state.is_pressed() {
                            Self::handle_exit_confirm_modal_mouse_down(state);
                        } else {
                            Self::handle_exit_confirm_modal_mouse_up(state);
                        }
                    }
                    return;
                }
                if Self::native_skirmish_shell_active(state) {
                    if button == MouseButton::Left {
                        if btn_state.is_pressed() {
                            Self::handle_skirmish_shell_mouse_down(state);
                        } else {
                            Self::handle_skirmish_shell_mouse_up(state, event_loop);
                        }
                    }
                } else if Self::single_player_shell_active(state) {
                    if button == MouseButton::Left {
                        if btn_state.is_pressed() {
                            Self::handle_single_player_shell_mouse_down(state);
                        } else {
                            Self::handle_single_player_shell_mouse_up(state);
                        }
                    }
                } else if state.screen == GameScreen::MainMenu
                    && !state.main_menu_shell_failed
                    && !state.main_menu_show_skirmish_setup
                    && !egui_consumed
                {
                    if button == MouseButton::Left {
                        if btn_state.is_pressed() {
                            Self::handle_main_menu_shell_mouse_down(state);
                        } else {
                            Self::handle_main_menu_shell_mouse_up(state, event_loop);
                        }
                    }
                } else if !egui_consumed && state.screen == GameScreen::SpawnPick {
                    if button == MouseButton::Left && btn_state.is_pressed() {
                        crate::app_spawn_pick::handle_spawn_pick_click(state);
                    }
                } else if !egui_consumed && state.screen == GameScreen::InGame {
                    app_input::handle_mouse_input(state, button, btn_state);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y as f32 / 30.0).clamp(-3.0, 3.0),
                };
                if crate::app_shell_transition::blocks_shell_input(state) {
                    return;
                }
                if !egui_consumed
                    && state.screen == GameScreen::MainMenu
                    && Self::native_skirmish_shell_active(state)
                    && Self::handle_skirmish_shell_mouse_wheel(state, lines)
                {
                    state.window.request_redraw();
                    return;
                }
                if !egui_consumed
                    && (state.screen == GameScreen::InGame || state.screen == GameScreen::SpawnPick)
                {
                    // Scroll sidebar when cursor is over the sidebar panel,
                    // otherwise zoom the game viewport (if enabled in settings).
                    if !app_input::try_sidebar_scroll(state, lines)
                        && state.skirmish_settings.zoom_enabled
                    {
                        crate::app_camera::apply_zoom(state, lines);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = Self::render_frame(state, event_loop, None, None) {
                    log::error!("Render: {:#}", err);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let (Some(state), Some(session)) = (self.state.as_mut(), self.tactical_capture.as_mut())
        {
            if let Err(err) = Self::render_frame(state, event_loop, None, Some(&mut *session)) {
                log::error!("Tactical capture render: {err:#}");
                session.fail(format!("tactical capture render failed: {err:#}"));
                event_loop.exit();
                return;
            }
            if !session.is_finished() {
                event_loop.set_control_flow(ControlFlow::WaitUntil(session.next_wake_deadline()));
            }
        } else if let (Some(state), Some(session)) =
            (self.state.as_mut(), self.shell_capture.as_mut())
        {
            if let Err(err) = Self::render_frame(state, event_loop, Some(&mut *session), None) {
                log::error!("Shell capture render: {err:#}");
                session.fail(format!("shell capture render failed: {err:#}"));
                event_loop.exit();
                return;
            }
            if !session.is_finished() {
                let mut deadline = session.next_wake_deadline();
                if let Some(wave_deadline) =
                    crate::app_shell_transition::main_menu_presented_wake_deadline(state)
                {
                    deadline = deadline.max(wave_deadline);
                }
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        } else if let Some(state) = &self.state {
            if crate::app_shell_transition::main_menu_presented_is_poisoned(state) {
                event_loop.set_control_flow(ControlFlow::Wait);
            } else if let Some(deadline) =
                crate::app_shell_transition::main_menu_presented_wake_deadline(state)
            {
                if Instant::now() >= deadline {
                    state.window.request_redraw();
                } else {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                }
            } else {
                state.window.request_redraw();
            }
        }
    }
}

impl App {
    /// Create window, GPU context, and egui integration. Does NOT load a map —
    /// starts in MainMenu state. Map loading is deferred to when the user
    /// clicks "Quick Play".
    fn initialize(
        event_loop: &ActiveEventLoop,
        capture_dimensions: Option<(u32, u32)>,
    ) -> Result<AppState> {
        let (window_width, window_height, window_visible) = capture_dimensions
            .map_or((SHELL_WINDOW_WIDTH, SHELL_WINDOW_HEIGHT, true), |size| {
                (size.0, size.1, false)
            });
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
        let mut bit_font = BitFont::fallback_5x7(&gpu, &batch_renderer);
        let depth_view: wgpu::TextureView = gpu.create_depth_texture();
        let shell_surface_presenter =
            crate::render::shell_surface_present::ShellSurfacePresenter::new(&gpu)?;
        let game_config = GameConfig::load().ok();
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
        let startup_asset_manager = Self::build_startup_asset_manager(game_config.as_ref());
        // Native process startup seeds Scenario before the MPModes loader. The
        // Cooperative factory reached by that loader then advances this cursor
        // before the first shell is shown.
        let mut frontend_seed_clock = crate::match_bootstrap::OrdinaryMatchSeedClock;
        let frontend_seed = crate::match_bootstrap::read_match_seed(&mut frontend_seed_clock);
        let startup_rules = startup_asset_manager
            .as_ref()
            // Startup shell: no mode or map selected yet, so no overrides.
            .and_then(|am| crate::app_init_helpers::load_rules_ini(am, None, None));
        let startup_csf = startup_asset_manager
            .as_ref()
            .and_then(crate::app_init::load_csf);
        let startup_sound_registry = startup_asset_manager
            .as_ref()
            .map(crate::app_transitions::load_sound_registry)
            .unwrap_or_default();
        let startup_audio_indices = startup_asset_manager
            .as_ref()
            .map(crate::app_transitions::load_audio_indices)
            .unwrap_or_default();
        let startup_eva_registry = startup_asset_manager
            .as_ref()
            .map(crate::app_transitions::load_eva_registry)
            .unwrap_or_default();
        if let Some(fnt) = startup_asset_manager.as_ref().and_then(|assets| {
            assets.get_ref("GAME.FNT").and_then(|data| {
                crate::assets::fnt_file::FntFile::from_bytes(data)
                    .map_err(|err| log::warn!("Failed to parse startup GAME.FNT: {err}"))
                    .ok()
            })
        }) {
            bit_font = BitFont::from_fnt(&gpu, &batch_renderer, &fnt);
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
        let version_txt = Self::load_version_txt(game_config.as_ref());
        let available_maps = app_list_maps::list_available_maps().unwrap_or_else(|err| {
            log::warn!("Could not list maps for menu: {:#}", err);
            Vec::new()
        });
        let skirmish_scenario_records =
            app_list_maps::list_skirmish_scenario_records_with_csf(startup_csf.as_ref())
                .unwrap_or_else(|err| {
                    log::warn!("Could not list Skirmish scenario records: {err:#}");
                    Vec::new()
                });
        let skirmish_scenario_records = if skirmish_scenario_records.is_empty() {
            available_maps
                .iter()
                .enumerate()
                .map(|(idx, map)| {
                    crate::skirmish_scenarios::SkirmishScenarioRecord::from_map_menu_entry(idx, map)
                })
                .collect()
        } else {
            skirmish_scenario_records
        };
        let skirmish_shell_maps: Vec<MapMenuEntry> = skirmish_scenario_records
            .iter()
            .map(crate::skirmish_scenarios::SkirmishScenarioRecord::to_map_menu_entry)
            .collect();
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
        if let Some(assets) = startup_asset_manager.as_ref() {
            skirmish_shell_state.trackbar_bounds =
                crate::app_init_helpers::load_skirmish_trackbar_bounds(assets);
            // Seed the per-match option values (Money/UnitCount/TechLevel/
            // GameSpeed and the checkbox toggles) from the merged rules
            // [MultiplayerDialogSettings], so a mod that changes a default opens
            // the dialog on — and launches the match with — its value. Without
            // assets we keep the stock-default values.
            let dialog_options = crate::app_init_helpers::load_skirmish_game_options(assets);
            skirmish_shell_state.apply_multiplayer_dialog_values(&dialog_options);
        }
        let skirmish_defaults =
            crate::app_skirmish_session::skirmish_global_defaults(&skirmish_shell_state);
        let offline_skirmish_runtime =
            crate::app_skirmish_session::OfflineSkirmishRuntime::initialize(
                frontend_seed.value,
                game_config
                    .as_ref()
                    .map(|config| config.paths.ra2_dir.as_path()),
                startup_asset_manager.as_ref(),
                skirmish_defaults,
            );
        offline_skirmish_runtime.hydrate_shell(
            &mut skirmish_shell_state,
            &skirmish_shell_maps,
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
            &skirmish_shell_maps,
        );
        let selected_shell_map = skirmish_shell_maps
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

        let mut state = AppState {
            random_map_generation: None,
            window,
            gpu,
            batch_renderer,
            instance_pool: crate::render::batch::InstanceBufferPool::new(),
            tile_atlas: None,
            map_basic: BasicSection::default(),
            terrain_grid: None,
            resolved_terrain: None,
            simulation: None,
            unit_atlas: None,
            vxl_slope_transition_cache: RefCell::new(Default::default()),
            palette_set: None,
            vxl_compute: Some(vxl_compute),
            sprite_atlas: None,
            overlay_atlas: None,
            bridge_atlas: None,
            bridge_railing_atlas: None,
            overlays: Vec::new(),
            terrain_objects: Vec::new(),
            waypoints: HashMap::new(),
            cell_tags: HashMap::new(),
            tags: HashMap::new(),
            triggers: HashMap::new(),
            events: HashMap::new(),
            actions: HashMap::new(),
            trigger_graph: TriggerGraph::default(),
            overlay_names: BTreeMap::new(),
            tiberium_radar_colors: HashMap::new(),
            overlay_registry: None,
            game_config,
            depth_view,
            shell_surface_presenter,
            upscale_pass,
            camera_x: 0.0,
            camera_y: 0.0,
            zoom_level: 1.0,
            zoom_target: 1.0,
            zoom_anchor_world: [0.0, 0.0],
            zoom_anchor_screen: [0.0, 0.0],
            cursor_x: 0.0,
            cursor_y: 0.0,
            keys_held: HashSet::new(),
            egui,
            screen: GameScreen::default(),
            available_maps,
            skirmish_shell_maps,
            skirmish_modes,
            skirmish_scenario_records,
            skirmish_settings,
            loading_session: None,
            next_match_correlation: 1,
            active_loading_correlation: None,
            loaded_startup: None,
            rust_l0_receipt: None,
            dev_skirmish_shell_enabled,
            skirmish_shell_state,
            offline_skirmish_runtime,
            skirmish_shell_last_painted_pressed_button: None,
            skirmish_shell_chrome,
            skirmish_preview_texture: None,
            loading_screen_atlas: None,
            loading_progress: crate::app_loading::LoadingProgressState::standard_skirmish(),
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
            main_menu_show_single_player_shell: false,
            main_menu_show_skirmish_setup: false,
            main_menu_show_native_skirmish_shell: false,
            skirmish_shell_return_to_single_player_shell: false,
            shell_first_paint_slide: None,
            shell_slide_active_shell: None,
            shell_slide_generation: 0,
            quit_cascade: None,
            minimap: None,
            minimap_dragging: false,
            middle_mouse_panning: false,
            middle_mouse_anchor_x: 0.0,
            middle_mouse_anchor_y: 0.0,
            radar_anim: None,
            radar_animation_source: None,
            power_bar_anim: crate::sidebar::PowerBarAnimState::new(),
            sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState::new(),
            in_game_gadgets: crate::app_gadget_input::InGameGadgets::new(),
            tooltips: crate::ui::tooltips::TooltipService::new(),
            tooltip_epoch: Instant::now(),
            message_list: crate::ui::messages::MessageList::new(
                3,
                0,
                crate::ui::messages::MESSAGE_MAX_VISIBLE_RETAIL,
                0,
            ),
            message_clock: crate::ui::messages::PauseAwareClock::default(),
            radar_content_insets: None,
            has_radar: false,
            selection_overlay: None,
            shroud_buffer: None,
            sidebar_cameo_atlas: None,
            sidebar_chrome: None,
            bit_font,
            software_cursor: startup_software_cursor,
            selection_state: SelectionState::new(),
            loaded_map_source: None,
            path_grid: None,
            animation_sequences: BTreeMap::new(),
            rules: startup_rules,
            art_registry: None,
            infantry_sequences: HashMap::new(),
            csf: startup_csf,
            house_color_map: HashMap::new(),
            house_roster: HouseRoster::default(),
            height_map: BTreeMap::new(),
            bridge_height_map: BTreeMap::new(),
            tactical_bridge_inverse_map: BTreeMap::new(),
            lighting_grid: CellLightGrid::new(),
            last_radiation_light_epoch: 0,
            map_lighting_config: LightingConfig::default(),
            theater_name: "TEMPERATE".to_string(),
            theater_ext: "tem".to_string(),
            last_update_time: Instant::now(),
            sim_accumulator_ms: 0,
            target_lines: crate::app_target_lines::TargetLineState::default(),
            configured_input_delay_ticks: input_delay_ticks,
            queued_order_mode: app_render::OrderMode::Move,
            control_groups: vec![Vec::new(); 10],
            local_player_owner: None,
            local_owner_override: None,
            eva_low_power_active: false,
            eva_funds_stalled: false,
            eva_announced_dying: std::collections::HashSet::new(),
            eva_under_attack_block_until_tick: 0,
            sandbox_full_visibility: false,
            disable_ai: true,
            spawn_pick_pending: false,
            targeting_mode: None,
            building_placement_preview: None,
            active_sidebar_tab: SidebarTab::default_active_tab(),
            sidebar_layout_spec,
            sidebar_layout_spec_base: base_sidebar_layout_spec,
            ui_scale,
            sidebar_scroll_rows: 0,
            asset_manager: startup_asset_manager,
            music_player: MusicPlayer::new(),
            sfx_player: SfxPlayer::new(),
            sound_registry: startup_sound_registry,
            audio_indices: startup_audio_indices,
            eva_registry: startup_eva_registry,
            sound_events: SoundEventQueue::new(),
            pending_fire_effects: Vec::new(),
            garrison_muzzle_flashes: Vec::new(),
            weapon_muzzle_flashes: Vec::new(),
            projectile_visuals: Vec::new(),
            parachute_anims: Vec::new(),
            paused: false,
            debug_frame_step_requested: false,
            // KD-3: unify the two game-speed sources. `in_game_options.game_speed`
            // is the single source of truth; seed it from the skirmish-setup speed
            // (internal 1) and derive `sim_speed_tps` from the same value, so the
            // Options slider reflects the current pace. The resulting tps is
            // unchanged from the prior `default_yr_skirmish_tps()` (= GS1 -> 63).
            sim_speed_tps: crate::app_types::tps_for_game_speed(
                crate::app_types::DEFAULT_YR_SKIRMISH_GAME_SPEED,
            ),
            in_game_options: crate::ui::shell::in_game_options_state::InGameOptionsState {
                game_speed: crate::app_types::DEFAULT_YR_SKIRMISH_GAME_SPEED,
                ..Default::default()
            },
            in_game_options_anchor: None,
            startup_splash_until: None,
            idle_anim_elapsed_ms: 0,
            debug_show_pathgrid: false,
            debug_terrain_cost_speed_type: None,
            debug_show_cell_grid: false,
            debug_show_heightmap: false,
            show_hotkey_help: false,
            debug_unit_inspector: false,
            show_save_load_panel: false,
            exit_confirm_modal: None,
            options_dialog: None,
            movies_credits_dialog: None,
            campaign_select: None,
            save_list_cache: crate::app_save_load_panel::SaveListCache::new(),
            dev_overlay_save_name: String::new(),
            last_save_tick: None,
            last_save_instant: None,
            last_loaded_save_path: None,
            frame_timer: crate::app_dev_overlay::FrameTimer::new(),
            displayed_credits: HashMap::new(),
            cached_overlay_instances: Vec::new(),
            cached_unit_instances: Vec::new(),
        };

        // Seed the live music volume from the user's saved RA2MD.INI
        // [Audio] ScoreVolume, falling back to the engine default when the
        // file/section/key is absent. Matches the original reading this at boot.
        if let Some(player) = state.music_player.as_mut() {
            let saved_volume = state
                .game_config
                .as_ref()
                .and_then(|config| {
                    crate::audio::music::read_score_volume_from_ra2md(&config.paths.ra2_dir)
                })
                .unwrap_or(crate::audio::music::DEFAULT_SCORE_VOLUME);
            player.set_volume(saved_volume);
        }

        if state.dev_skirmish_shell_enabled {
            Self::ensure_active_cooperative_shell_selection(&mut state);
        }

        if std::env::var("RA2_QUICKPLAY").is_ok() {
            let skirmish_settings = state.skirmish_settings.clone();
            let request =
                crate::app_loading::LoadingRequest::generic_map_load("auto", skirmish_settings);
            crate::app_loading::begin_loading(&mut state, request);
        }

        Ok(state)
    }

    /// Dispatch rendering based on current GameScreen state.
    fn render_frame(
        state: &mut AppState,
        event_loop: &ActiveEventLoop,
        mut shell_capture: Option<&mut crate::app_shell_capture::ShellCaptureSession>,
        mut tactical_capture: Option<
            &mut crate::app_tactical_capture::session::TacticalCaptureSession,
        >,
    ) -> Result<()> {
        anyhow::ensure!(
            shell_capture.is_none() || tactical_capture.is_none(),
            "shell and tactical capture cannot share a render"
        );
        if let Some(session) = tactical_capture.as_deref_mut() {
            session.drive_before_render(state)?;
        }
        state.frame_timer.sample(Instant::now());
        crate::app_tooltips::update(state);
        crate::app_messages::update(state);
        if let Some(until) = state.startup_splash_until {
            if Instant::now() < until {
                let output: wgpu::SurfaceTexture = state
                    .gpu
                    .surface
                    .get_current_texture()
                    .map_err(|e| anyhow::anyhow!("Surface texture: {}", e))?;
                let view: wgpu::TextureView = output.texture.create_view(&Default::default());
                let mut encoder: wgpu::CommandEncoder =
                    state
                        .gpu
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Startup Splash Frame"),
                        });
                app_transitions::clear_screen(&mut encoder, &view);
                state.egui.begin_frame(&state.window);
                main_menu::draw_loading_screen(&state.egui.ctx, "Initializing client");
                state.egui.end_frame_and_render(
                    &state.gpu,
                    &mut encoder,
                    &view,
                    &state.window,
                    state.use_software_cursor(),
                );
                state.gpu.queue.submit(std::iter::once(encoder.finish()));
                output.present();
                return Ok(());
            }
            state.startup_splash_until = None;
        }

        // Drive the graceful quit cascade (started on Exit-confirm OK). Compute the
        // voice poll before borrowing the cascade mutably to avoid aliasing.
        if state.quit_cascade.is_some() {
            let now = Instant::now();
            let voices_active = state
                .sfx_player
                .as_ref()
                .is_some_and(|sfx| sfx.voices_active());
            let tick = state
                .quit_cascade
                .as_mut()
                .expect("cascade present")
                .tick(now, voices_active);
            if let (Some(vol), Some(player)) = (tick.music_volume, state.music_player.as_mut()) {
                player.set_volume(vol);
            }
            if tick.stop_music {
                if let Some(player) = state.music_player.as_mut() {
                    player.stop();
                }
            }
            if tick.finished {
                state.quit_cascade = None;
                event_loop.exit();
                return Ok(());
            }
        }

        if tactical_capture.is_none() && matches!(state.screen, GameScreen::InGame) {
            let now = Instant::now();
            let elapsed_ms = app_sim_tick::update_elapsed_ms(state, now);
            app_sim_tick::advance_in_game_runtime(state, elapsed_ms);
        }

        // Native queues/maintains [INTRO] before arming the 0xE2 first-paint
        // owner. An arm stays silent until the matching surface is acquired.
        for step in MAIN_MENU_SHELL_PRELUDE {
            match step {
                ShellFramePreludeStep::MaintainIntro => Self::maintain_main_menu_intro(state),
                ShellFramePreludeStep::ObserveEntry => {
                    crate::app_shell_transition::prepare_main_menu_first_paint_before_acquire(state)
                }
            }
        }
        match crate::app_shell_transition::poll_main_menu_first_paint_before_acquire(
            state,
            Instant::now(),
        )? {
            crate::app_shell_transition::MainMenuFirstPaintPoll::WaitUntil(_) => {
                return Ok(());
            }
            crate::app_shell_transition::MainMenuFirstPaintPoll::Completed => {
                if let Some(session) = shell_capture.as_deref_mut()
                    && session.completion_handoff()
                        == crate::app_shell_capture::ShellCompletionHandoff::
                            FinalizeExitReturnBeforeAcquire
                {
                    session.complete_entry_sequence_after_wave(state)?;
                    event_loop.exit();
                    return Ok(());
                }
            }
            crate::app_shell_transition::MainMenuFirstPaintPoll::Acquire => {}
        }

        let output: wgpu::SurfaceTexture = state
            .gpu
            .surface
            .get_current_texture()
            .map_err(|e| anyhow::anyhow!("Surface texture: {}", e))?;
        let view: wgpu::TextureView = output.texture.create_view(&Default::default());
        let mut encoder: wgpu::CommandEncoder =
            state
                .gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Frame"),
                });
        let mut pending_main_menu_entry_token = None;
        let mut pending_main_menu_title_receipt = None;

        crate::app_shell_transition::activate_shell_first_paint_after_acquire(state);
        // Advance the Skirmish right-panel static text reveals (started at the
        // slide's completion edge). 30 ms-gated internally; a no-op when idle.
        crate::app_shell_transition::advance_shell_static_reveals(state);
        crate::app_shell_transition::poll_main_menu_title_reveal(state);
        let shell_capture_current_frame = match shell_capture.as_deref_mut() {
            Some(session) => session.should_capture_current_frame(state)?,
            None => false,
        };
        let mut game_render_output: Option<crate::app_render::GameRenderOutput> = None;

        match &state.screen {
            GameScreen::MainMenu => {
                if let crate::app_shell_transition::ShellFirstPaintRenderResult::Rendered {
                    main_menu_entry_token,
                } = crate::app_shell_transition::render_shell_first_paint_slide(
                    state,
                    &mut encoder,
                    &output.texture,
                    &view,
                )? {
                    pending_main_menu_entry_token = main_menu_entry_token;
                } else if Self::native_skirmish_shell_active(state) {
                    crate::app_skirmish_shell_render::render_skirmish_shell(
                        state,
                        &mut encoder,
                        &view,
                    )?;
                } else if Self::single_player_shell_active(state) {
                    match crate::app_single_player_shell_render::render_single_player_shell(
                        state,
                        &mut encoder,
                        &view,
                    )? {
                        crate::app_single_player_shell_render::SinglePlayerShellRenderResult::Rendered => {
                            state.egui.begin_frame(&state.window);
                            if state.show_save_load_panel {
                                Self::handle_save_load_panel(state);
                            }
                            // Campaign selector (and any other menu modal) draws
                            // over the SP shell; confirm-quit cannot originate
                            // here, so its return value is ignored.
                            let _ = Self::draw_main_menu_dialogs(state, false);
                            state.egui.end_frame_and_render(
                                &state.gpu,
                                &mut encoder,
                                &view,
                                &state.window,
                                state.use_software_cursor(),
                            );
                        }
                        crate::app_single_player_shell_render::SinglePlayerShellRenderResult::Fallback => {
                            Self::render_egui_main_menu_fallback(
                                state,
                                &mut encoder,
                                &view,
                                event_loop,
                            )?;
                        }
                    }
                } else if !state.main_menu_shell_failed && !state.main_menu_show_skirmish_setup {
                    match crate::app_main_menu_shell_render::render_main_menu_shell(
                        state,
                        &mut encoder,
                        &output.texture,
                    )? {
                        crate::app_main_menu_shell_render::MainMenuShellRenderResult::Rendered {
                            title_receipt,
                        } => {
                            pending_main_menu_title_receipt = title_receipt;
                            state.egui.begin_frame(&state.window);
                            // The SHP shell renders the quit-confirm as an SHP
                            // overlay (and OK exits via its hit-test), so the egui
                            // exit-confirm is suppressed here; campaign/options/
                            // movies egui dialogs still draw. confirm_quit stays false.
                            let confirm_quit = Self::draw_main_menu_dialogs(state, false);
                            state.egui.end_frame_and_render(
                                &state.gpu,
                                &mut encoder,
                                &view,
                                &state.window,
                                state.use_software_cursor(),
                            );
                            if confirm_quit {
                                state.gpu.queue.submit(std::iter::once(encoder.finish()));
                                output.present();
                                if let Some(token) =
                                    pending_main_menu_entry_token.take()
                                {
                                    crate::app_shell_transition::record_main_menu_entry_presented(
                                        state, token,
                                    )?;
                                }
                                if let Some(receipt) =
                                    pending_main_menu_title_receipt.take()
                                {
                                    anyhow::ensure!(
                                        state
                                            .main_menu_shell_state
                                            .title_reveal
                                            .record_presented(receipt),
                                        "main-menu title receipt was stale at present commit"
                                    );
                                }
                                event_loop.exit();
                                return Ok(());
                            }
                        }
                        crate::app_main_menu_shell_render::MainMenuShellRenderResult::Fallback => {
                            Self::render_egui_main_menu_fallback(
                                state,
                                &mut encoder,
                                &view,
                                event_loop,
                            )?;
                        }
                    }
                } else {
                    Self::render_egui_main_menu_fallback(state, &mut encoder, &view, event_loop)?;
                }
            }
            GameScreen::Loading => {
                match crate::app_loading::render_loading_screen(state, &mut encoder, &view) {
                    crate::app_loading::LoadingRenderResult::NativeRendered => {}
                    crate::app_loading::LoadingRenderResult::GenericFallback => {
                        let map_name_display = crate::app_loading::loading_map_name(state)
                            .unwrap_or("auto")
                            .to_string();
                        app_transitions::clear_screen(&mut encoder, &view);
                        state.egui.begin_frame(&state.window);
                        main_menu::draw_loading_screen(&state.egui.ctx, &map_name_display);
                        state.egui.end_frame_and_render(
                            &state.gpu,
                            &mut encoder,
                            &view,
                            &state.window,
                            state.use_software_cursor(),
                        );
                    }
                    crate::app_loading::LoadingRenderResult::NativeFailed(err) => {
                        app_transitions::clear_screen(&mut encoder, &view);
                        log::warn!("Could not render native loading screen: {err:#}");
                        crate::app_loading::clear_loading_state(state);
                        crate::app_loading::clear_match_startup_state(state);
                        state.screen = GameScreen::MissionResult {
                            title: "Loading Failed".to_string(),
                            detail: format!("{err:#}"),
                        };
                    }
                }
            }
            GameScreen::InGame => {
                let game_output = if state.upscale_pass.is_some() {
                    // Render game to intermediate texture, then upscale to swapchain.
                    let up = state.upscale_pass.as_ref().unwrap();
                    let game_view = up.color_view().clone();
                    let game_depth = up.depth_view().clone();
                    let saved_depth = std::mem::replace(&mut state.depth_view, game_depth);
                    let result = app_render::render_game(state, &mut encoder, &game_view);
                    state.depth_view = saved_depth;
                    let render_output = result?;
                    state
                        .upscale_pass
                        .as_ref()
                        .unwrap()
                        .draw(&mut encoder, &view);
                    render_output
                } else {
                    app_render::render_game(state, &mut encoder, &view)?
                };
                let sidebar_view = game_output.sidebar_view.as_ref();
                // Paused: draw the native in-game Options (0xBBB) overlay over the
                // frozen battlefield before egui. The egui pause card is retired;
                // egui below now only carries the sidebar text + dev overlay.
                if state.paused {
                    Self::ensure_skirmish_shell_chrome(state);
                    crate::app_skirmish_shell_render::render_in_game_options_overlay(
                        state,
                        &mut encoder,
                        &view,
                        sidebar_view,
                    )?;
                }
                // Always run egui in-game for sidebar text overlay (Ready labels, credits).
                state.egui.begin_frame(&state.window);
                if let Some(sv) = sidebar_view {
                    crate::app_sidebar_text::draw_sidebar_text_overlay(
                        &state.egui.ctx,
                        sv,
                        state.ui_scale,
                    );
                }
                // Debug panels use a light/.NET theme — push light visuals
                // before rendering, then restore the original after.
                let any_debug_panel = state.debug_show_pathgrid
                    || state.debug_unit_inspector
                    || state.show_hotkey_help;
                let prev_visuals = if any_debug_panel {
                    Some(crate::app_debug_panel::push_debug_light_visuals(
                        &state.egui.ctx,
                    ))
                } else {
                    None
                };
                if state.debug_show_pathgrid {
                    crate::app_debug_panel::draw_debug_panel(&state.egui.ctx, state);
                }
                crate::app_debug_panel::draw_event_history_panel(&state.egui.ctx, state);
                if state.show_hotkey_help {
                    crate::app_debug_panel::draw_hotkey_help(&state.egui.ctx);
                }
                if let Some(prev) = prev_visuals {
                    crate::app_debug_panel::pop_debug_light_visuals(&state.egui.ctx, prev);
                }
                if state.show_save_load_panel {
                    Self::handle_save_load_panel(state);
                }
                if state.paused {
                    // The native 0xBBB overlay (drawn above) replaces the egui
                    // pause card. The dev overlay rides along — push its own light
                    // visuals so its chrome matches the debug panels.
                    let prev = crate::app_debug_panel::push_debug_light_visuals(&state.egui.ctx);
                    Self::handle_dev_overlay(state);
                    crate::app_debug_panel::pop_debug_light_visuals(&state.egui.ctx, prev);
                }
                state.egui.end_frame_and_render(
                    &state.gpu,
                    &mut encoder,
                    &view,
                    &state.window,
                    state.use_software_cursor(),
                );
                game_render_output = Some(game_output);
            }
            GameScreen::MissionResult { title, detail } => {
                app_transitions::clear_screen(&mut encoder, &view);
                state.egui.begin_frame(&state.window);
                if crate::ui::mission_status::draw_mission_result_screen(
                    &state.egui.ctx,
                    title,
                    detail,
                ) {
                    // Persist the replay before the sim is torn down (symmetric
                    // with return_to_main_menu) so a match that ended in
                    // victory/defeat still leaves a replayable log on disk.
                    crate::app_sim_tick::flush_replay_log(state);
                    Self::capture_returned_skirmish_rng(state);
                    crate::app_loading::clear_match_startup_state(state);
                    state.screen = GameScreen::MainMenu;
                    Self::enter_shell_window_mode(state);
                    state.zoom_level = 1.0;
                    state.zoom_target = 1.0;
                }
                state.egui.end_frame_and_render(
                    &state.gpu,
                    &mut encoder,
                    &view,
                    &state.window,
                    state.use_software_cursor(),
                );
            }
            GameScreen::SpawnPick => {
                crate::app_spawn_pick::render_spawn_pick(state, &mut encoder, &view)?;
                state.egui.begin_frame(&state.window);
                crate::app_spawn_pick::draw_spawn_pick_overlay(&state.egui.ctx.clone(), state);
                state.egui.end_frame_and_render(
                    &state.gpu,
                    &mut encoder,
                    &view,
                    &state.window,
                    state.use_software_cursor(),
                );
            }
        }

        let entry_sequence_identity = match shell_capture.as_deref_mut() {
            Some(session) if session.is_entry_sequence() => {
                let token = pending_main_menu_entry_token
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("entry-sequence frame produced no token"))?;
                session.observe_entry_sequence_after_render(state, token)?
            }
            _ => None,
        };
        let pending_entry_sequence = if entry_sequence_identity.is_some() {
            Some(crate::render::frame_readback::PendingBgra8Readback::encode(
                &state.gpu.device,
                &mut encoder,
                &output.texture,
                state.gpu.config.format,
                state.gpu.config.width,
                state.gpu.config.height,
            )?)
        } else {
            None
        };
        let tactical_capture_current_frame =
            match (tactical_capture.as_deref_mut(), game_render_output.as_ref()) {
                (Some(session), Some(render_output)) => {
                    session.observe_after_render(state, render_output)?
                }
                (Some(_), None) | (None, _) => false,
            };
        let capture_current_frame = shell_capture_current_frame || tactical_capture_current_frame;
        let pending_capture = if capture_current_frame {
            Some(crate::render::frame_readback::PendingBgra8Readback::encode(
                &state.gpu.device,
                &mut encoder,
                &output.texture,
                state.gpu.config.format,
                state.gpu.config.width,
                state.gpu.config.height,
            )?)
        } else {
            None
        };
        let capture_timeout = if capture_current_frame {
            Some(if shell_capture_current_frame {
                shell_capture
                    .as_deref()
                    .expect("shell capture session exists when readback is requested")
                    .readback_timeout()?
            } else {
                tactical_capture
                    .as_deref()
                    .expect("tactical capture session exists when readback is requested")
                    .readback_timeout()?
            })
        } else {
            None
        };
        let submission = state.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        if let Some(token) = pending_main_menu_entry_token.take() {
            crate::app_shell_transition::record_main_menu_entry_presented(state, token)?;
        }
        if let (Some(identity), Some(readback)) = (entry_sequence_identity, pending_entry_sequence)
        {
            shell_capture
                .as_deref_mut()
                .expect("entry-sequence session exists for retained readback")
                .record_entry_sequence_submission(identity, readback, submission.clone())?;
        }
        if let Some(receipt) = pending_main_menu_title_receipt.take() {
            anyhow::ensure!(
                state
                    .main_menu_shell_state
                    .title_reveal
                    .record_presented(receipt),
                "main-menu title receipt was stale at present commit"
            );
        }
        if let Some(pending_capture) = pending_capture {
            let pixels = pending_capture.finish(
                &state.gpu.device,
                submission,
                capture_timeout.expect("capture timeout exists with pending readback"),
            )?;
            let surface_format = state.gpu.config.format;
            if shell_capture_current_frame {
                shell_capture
                    .as_deref_mut()
                    .expect("shell capture session exists when readback completes")
                    .complete(state, surface_format, &pixels)?;
            } else {
                tactical_capture
                    .as_deref_mut()
                    .expect("tactical capture session exists when readback completes")
                    .complete_after_readback(state, surface_format, &pixels)?;
            }
            event_loop.exit();
        }

        // Deferred loading: after presenting the Loading screen frame,
        // pump one loading phase. The next patch will continue splitting the
        // remaining legacy load body into smaller phases.
        if matches!(state.screen, GameScreen::Loading) {
            crate::app_loading::loading_screen_presented(state);
            let native_loading = crate::app_loading::is_native_loading_session(state);
            match crate::app_loading::pump_loading_after_present(state) {
                crate::app_loading::LoadingPump::Pending => {
                    state.window.request_redraw();
                }
                crate::app_loading::LoadingPump::Finished(result) => {
                    app_transitions::apply_map_load_result(state, result);
                }
                crate::app_loading::LoadingPump::Failed(err) => {
                    log::warn!("Could not load map: {err:#}");
                    if native_loading {
                        crate::app_loading::clear_loading_state(state);
                        crate::app_loading::clear_match_startup_state(state);
                        state.screen = GameScreen::MissionResult {
                            title: "Loading Failed".to_string(),
                            detail: format!("{err:#}"),
                        };
                    } else {
                        let result = app_transitions::fallback_map_load_result();
                        app_transitions::apply_map_load_result(state, result);
                    }
                }
            }
        }

        Ok(())
    }

    /// Temporary non-chrome quit-to-menu (design §8 Q1): the native `0xBBB`
    /// dialog has no quit button and the egui pause card is retired, so
    /// quit-to-menu survives as a dev-overlay shortcut until the native
    /// Abort-Mission dialog is built (a later 5a step).
    fn capture_returned_skirmish_rng(state: &mut AppState) {
        let gameplay_rng = state
            .simulation
            .as_ref()
            .map(crate::sim::world::Simulation::clone_scenario_rng);
        if let Some(gameplay_rng) = gameplay_rng
            && state
                .offline_skirmish_runtime
                .capture_returned_gameplay_rng(gameplay_rng)
        {
            log::info!("Returned gameplay Scenario cursor to the offline shell");
        }
    }

    fn return_to_main_menu(state: &mut AppState) {
        state.paused = false;
        // Persist the recorded replay before the sim (which owns the log) is
        // torn down — every finished match leaves a replayable log on disk.
        crate::app_sim_tick::flush_replay_log(state);
        Self::capture_returned_skirmish_rng(state);
        crate::app_loading::clear_match_startup_state(state);
        if let Some(ref mut player) = state.music_player {
            player.stop();
        }
        state.screen = GameScreen::MainMenu;
        Self::enter_shell_window_mode(state);
        state.zoom_level = 1.0;
        state.zoom_target = 1.0;
        state.window.set_cursor_visible(true);
        log::info!("Returned to main menu");
    }

    /// Draw the pause menu and handle its actions.
    ///
    /// Retired from the in-game paint path in 5a-ii (the native `0xBBB` overlay
    /// replaces the egui card); kept compiled for reference until 5a-iii wires the
    /// native control input. Quit-to-menu now lives on `return_to_main_menu`.
    #[allow(dead_code)]
    fn handle_pause_menu(state: &mut AppState) {
        use crate::ui::pause_menu::{self, PauseMenuAction, PauseMenuInfo};

        let info = PauseMenuInfo {
            current_track: state.music_player.as_ref().and_then(|p| p.current_track()),
            volume: state.music_player.as_ref().map_or(0.5, |p| p.volume()),
            speed_tps: state.sim_speed_tps,
        };

        let action: PauseMenuAction = pause_menu::draw_pause_menu(&state.egui.ctx, &info);

        match action {
            PauseMenuAction::Resume => {
                state.paused = false;
                // Reset timing to prevent sim accumulator spike from pause duration.
                state.last_update_time = Instant::now();
                state.sim_accumulator_ms = 0;
                // Re-hide OS cursor so the software cursor takes over.
                if state.software_cursor.is_some() {
                    state.window.set_cursor_visible(false);
                }
                log::info!("Game resumed");
            }
            PauseMenuAction::ReturnToMenu => {
                Self::return_to_main_menu(state);
            }
            PauseMenuAction::NextTrack => {
                if let (Some(player), Some(assets)) =
                    (&mut state.music_player, &state.asset_manager)
                {
                    if let Some(name) = player.play_next(assets) {
                        log::info!("Switched to track: {}", name);
                    }
                }
            }
            PauseMenuAction::SetMusicVolume(vol) => {
                if let Some(ref mut player) = state.music_player {
                    player.set_volume(vol);
                }
            }
            PauseMenuAction::SetGameSpeed(tps) => {
                state.sim_speed_tps = tps;
                log::info!("Game speed set to {} tps", tps);
            }
            PauseMenuAction::None => {}
        }
    }

    /// Draw the save/load panel and handle its actions.
    fn handle_save_load_panel(state: &mut AppState) {
        use crate::app_save_load_panel::SaveLoadAction;

        let action = crate::app_save_load_panel::draw_save_load_panel(
            &state.egui.ctx,
            &mut state.save_list_cache,
        );

        match action {
            SaveLoadAction::Load(path) => {
                crate::app_loading::clear_match_startup_state(state);
                app_input::load_save_file(state, &path);
            }
            SaveLoadAction::Delete(path) => {
                if let Err(e) = std::fs::remove_file(&path) {
                    log::error!("Failed to delete save {}: {e}", path.display());
                } else {
                    log::info!("Deleted save: {}", path.display());
                }
                state.save_list_cache.invalidate();
            }
            SaveLoadAction::Close => {
                state.show_save_load_panel = false;
            }
            SaveLoadAction::None => {}
        }
    }

    /// Draw the dev overlay and dispatch its actions. No-op when the
    /// overlay is hidden — caller checks `show_dev_overlay` before
    /// calling.
    fn handle_dev_overlay(state: &mut AppState) {
        use crate::app_dev_overlay::{self, DevOverlayAction, DevOverlayInfo, RecentSaveRow};

        // Build the recent-saves snapshot from the existing cache.
        state.save_list_cache.refresh_if_dirty();
        let recent_saves: Vec<RecentSaveRow> = state
            .save_list_cache
            .entries
            .iter()
            .take(5)
            .map(|e| RecentSaveRow {
                path: e.path.clone(),
                display_name: e
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string(),
                tick: e.header.tick,
                age_str: crate::app_save_load_panel::format_timestamp(e.header.save_timestamp),
            })
            .collect();

        let last_save_age: Option<String> = state.last_save_instant.map(|t| {
            let secs = t.elapsed().as_secs();
            if secs < 60 {
                format!("{secs}s ago")
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else {
                format!("{}h {}m ago", secs / 3600, (secs % 3600) / 60)
            }
        });

        let last_load_available = state
            .last_loaded_save_path
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false);
        let last_load_display = state
            .last_loaded_save_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(str::to_string);

        // Temporarily move the save-name buffer out so it can be borrowed
        // mutably by the info struct without conflicting with state.
        let mut save_name = std::mem::take(&mut state.dev_overlay_save_name);

        let mut info = DevOverlayInfo {
            sim_speed_tps: state.sim_speed_tps,
            paused: state.paused,
            music_volume: state.music_player.as_ref().map_or(0.5, |p| p.volume()),
            sfx_volume: state.sfx_player.as_ref().map_or(0.7, |p| p.volume()),
            show_pathgrid: state.debug_show_pathgrid,
            show_cell_grid: state.debug_show_cell_grid,
            show_heightmap: state.debug_show_heightmap,
            show_unit_inspector: state.debug_unit_inspector,
            reveal_map: state.sandbox_full_visibility,
            fps: state.frame_timer.fps(),
            frame_ms: state.frame_timer.frame_ms_mean(),
            tick_budget_ms: if state.sim_speed_tps == 0 {
                0.0
            } else {
                1000.0 / state.sim_speed_tps as f32
            },
            entity_count: state.simulation.as_ref().map_or(0, |s| s.entities().len()),
            save_name_buf: &mut save_name,
            last_save_tick: state.last_save_tick,
            last_save_age,
            last_load_available,
            last_load_display,
            recent_saves,
        };

        let action = app_dev_overlay::draw_dev_overlay(&state.egui.ctx, &mut info);

        // Restore the (possibly-edited) buffer.
        state.dev_overlay_save_name = save_name;

        match action {
            DevOverlayAction::None => {}
            // Developer-only direct-tps override (fine-grained 1..200, for
            // debugging). It deliberately BYPASSES the 0..6 Options model (KD-3):
            // it writes `sim_speed_tps` without touching `in_game_options.game_speed`,
            // so the next native Options close reasserts the Options speed. The
            // 0..6 bucket model cannot represent these arbitrary tps values.
            DevOverlayAction::SetGameSpeed(tps) => {
                state.sim_speed_tps = tps.max(1);
                log::info!("Game speed: {} tps", state.sim_speed_tps);
            }
            DevOverlayAction::ResetGameSpeed => {
                state.sim_speed_tps = crate::app_types::default_yr_skirmish_tps();
                log::info!("Game speed reset to {} tps", state.sim_speed_tps);
            }
            DevOverlayAction::SetMusicVolume(v) => {
                if let Some(p) = &mut state.music_player {
                    p.set_volume(v);
                }
            }
            DevOverlayAction::SetSfxVolume(v) => {
                if let Some(p) = &mut state.sfx_player {
                    p.set_volume(v);
                }
            }
            DevOverlayAction::TogglePause => {
                app_input::toggle_debug_pause(state);
            }
            DevOverlayAction::ReturnToMenu => {
                Self::return_to_main_menu(state);
            }
            DevOverlayAction::StepOneTick => {
                if state.paused {
                    state.debug_frame_step_requested = true;
                }
            }
            DevOverlayAction::TogglePathGrid => {
                app_input::toggle_pathgrid_overlay(state);
            }
            DevOverlayAction::ToggleCellGrid => {
                state.debug_show_cell_grid = !state.debug_show_cell_grid;
            }
            DevOverlayAction::ToggleHeightmap => {
                state.debug_show_heightmap = !state.debug_show_heightmap;
            }
            DevOverlayAction::ToggleUnitInspector => {
                app_input::toggle_unit_inspector(state);
            }
            DevOverlayAction::ToggleRevealMap => {
                state.sandbox_full_visibility = !state.sandbox_full_visibility;
                log::info!(
                    "Reveal map: {}",
                    if state.sandbox_full_visibility {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
            DevOverlayAction::SaveAs => {
                let name = std::mem::take(&mut state.dev_overlay_save_name);
                app_input::save_with_name(state, &name);
            }
            DevOverlayAction::ReloadLastLoad => {
                if let Some(path) = state.last_loaded_save_path.clone() {
                    if path.exists() {
                        crate::app_loading::clear_match_startup_state(state);
                        app_input::load_save_file(state, &path);
                    } else {
                        log::warn!(
                            "Reload last load: file no longer exists: {}",
                            path.display()
                        );
                    }
                }
            }
            DevOverlayAction::LoadSave(path) => {
                crate::app_loading::clear_match_startup_state(state);
                app_input::load_save_file(state, &path);
            }
        }
    }
}

/// Auto-detect UI scale from screen dimensions.
/// Returns 0.5, 1.0, or 1.5 to keep pixel art crisp at all resolutions.
/// Requires both enough height AND enough width so the sidebar doesn't
/// eat the entire screen at small window sizes.
fn auto_detect_ui_scale(screen_width: u32, screen_height: u32) -> f32 {
    // 1.5x: needs at least 2560×1441 (typical 1440p+ / 4K).
    if screen_width >= 2560 && screen_height > 1440 {
        return 1.5;
    }
    // 1.5x: needs at least 1600×900 so the sidebar leaves enough map view.
    if screen_width >= 1600 && screen_height >= 900 {
        return 1.5;
    }
    0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_intro_precedes_entry_observation() {
        assert_eq!(
            MAIN_MENU_SHELL_PRELUDE,
            [
                ShellFramePreludeStep::MaintainIntro,
                ShellFramePreludeStep::ObserveEntry,
            ]
        );
    }

    #[test]
    fn six_preview_boundaries_cover_the_originals_eight_redraws() {
        use crate::map::rmg::STAGE_ORDER;
        use crate::map::rmg::build::GenerationPoint;

        let drawn: Vec<GenerationPoint> = std::iter::once(GenerationPoint::Initial)
            .chain(
                STAGE_ORDER
                    .iter()
                    .map(|stage| GenerationPoint::After(*stage)),
            )
            .filter(|point| draws_preview(*point))
            .collect();
        // Eight redraws in the original, two of them repeats of the image
        // already on screen.
        assert_eq!(drawn.len(), 6, "{drawn:?}");
        assert_eq!(drawn[0], GenerationPoint::Initial);
        // The last one precedes the final recalc, so the finished map still
        // differs from it and the closing draw is not a repeat either.
        assert_eq!(
            drawn[5],
            GenerationPoint::After(crate::map::rmg::Stage::Rocks)
        );
    }

    #[test]
    fn shell_key_translation_matches_dialog_controller_route() {
        assert_eq!(App::shell_key_for_code(KeyCode::Tab), Some(ShellKey::Tab));
        assert_eq!(
            App::shell_key_for_code(KeyCode::Enter),
            Some(ShellKey::Enter)
        );
        assert_eq!(
            App::shell_key_for_code(KeyCode::NumpadEnter),
            Some(ShellKey::Enter)
        );
        assert_eq!(
            App::shell_key_for_code(KeyCode::Escape),
            Some(ShellKey::Escape)
        );
        assert_eq!(App::shell_key_for_code(KeyCode::Space), None);
    }
}
