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
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
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

#[path = "app_startup_splash.rs"]
mod app_startup_splash;
mod frame;
mod handler;
mod initialize;
mod state;

pub(crate) use state::{AppState, reset_scenario_exit_runtime};

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
/// Caption gamemd loads onto the abort-mission confirmation's action button.
/// Its two mode-dependent siblings (`GUI:Restart` in campaign, `GUI:Observe` in
/// multiplayer) sit on a second button that offline skirmish hides outright.
const ABORT_CONFIRM_LEAVE_KEY: &str = "GUI:Leave";
/// The shipped English table resolves `GUI:Leave` to "Quit"; the fallback only
/// applies when the string table is missing entirely, so it says the same.
const ABORT_CONFIRM_LEAVE_FALLBACK: &str = "Quit";
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

/// Generated-map ownership across the setup dialog and loading handoff.
///
/// The candidate belongs only to the open setup dialog. The accepted map is
/// retained until the matching `.SED` launch transfers it to LoadingRequest.
/// gamemd provenance: random-map setup runner FUN_00595BC0 and accepted caller
/// 0x005E8590 retain the generated scenario consumed by Scenario initialization.
#[derive(Default)]
pub(crate) struct RandomMapGenerationRetention {
    candidate: Option<crate::map::rmg::GeneratedMap>,
    accepted: Option<(String, crate::map::rmg::GeneratedMap)>,
}

impl RandomMapGenerationRetention {
    fn begin_generation(&mut self) {
        self.candidate = None;
        self.accepted = None;
    }

    fn finish_generation(&mut self, generated: crate::map::rmg::GeneratedMap) {
        self.candidate = Some(generated);
    }

    fn cancel_setup(&mut self) {
        self.candidate = None;
        self.accepted = None;
    }

    fn accept_setup(&mut self, selected_map_file: &str) {
        self.accepted = self
            .candidate
            .take()
            .map(|generated| (selected_map_file.to_owned(), generated));
    }

    fn select_map(&mut self, selected_map_file: &str) {
        if self.accepted.as_ref().is_some_and(|(accepted_file, _)| {
            !accepted_file.eq_ignore_ascii_case(selected_map_file)
        }) {
            self.accepted = None;
        }
    }

    fn take_for_loading(
        &mut self,
        selected_map_file: Option<&str>,
    ) -> Option<crate::map::rmg::GeneratedMap> {
        let (accepted_file, generated) = self.accepted.take()?;
        selected_map_file
            .is_some_and(|selected| accepted_file.eq_ignore_ascii_case(selected))
            .then_some(generated)
    }
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

/// Top-level application. Implements winit's ApplicationHandler.
pub struct App {
    state: Option<AppState>,
    shell_capture: Option<crate::app_shell_capture::ShellCaptureSession>,
    tactical_capture: Option<crate::app_tactical_capture::session::TacticalCaptureSession>,
    startup_audio: StartupAudioDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StartupAudioDisposition {
    initialize_music_output: bool,
    initialize_sfx_output: bool,
    load_audio_indices: bool,
}

impl StartupAudioDisposition {
    /// gamemd-derived: `FUN_0052F620 @ 0x0052F620` clears the audio global for
    /// `-NOAUDIO`; active `Init_Game @ 0x0052BA60` passes that value to
    /// `AudioSystem::Init @ 0x00406B10`, whose false branch skips output and
    /// audio MIX/index construction while the later bookkeeping remains live.
    const fn for_audio_enabled(audio_enabled: bool) -> Self {
        Self {
            initialize_music_output: audio_enabled,
            initialize_sfx_output: audio_enabled,
            load_audio_indices: audio_enabled,
        }
    }
}

/// Keep initial and later scenario audio-index loads on the same process-start
/// decision. This deliberately does not inspect the optional output players:
/// an enabled DirectSound/output initialization may independently fail.
pub(crate) const fn should_load_audio_indices(audio_indices_enabled: bool) -> bool {
    audio_indices_enabled
}

impl Default for StartupAudioDisposition {
    fn default() -> Self {
        Self::for_audio_enabled(true)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(crate::app_startup_options::RetailStartupOptions::default())
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

    /// The end-of-match score screen owns input whenever it has both a resolved
    /// model and the shell chrome to draw it with; without either, the result
    /// screen falls back to its egui form and egui keeps the input.
    fn score_shell_active(state: &AppState) -> bool {
        matches!(state.screen, GameScreen::MissionResult { .. })
            && state.score_screen.is_some()
            && state.main_menu_shell_chrome.is_some()
    }

    fn score_shell_layout(state: &AppState) -> crate::ui::score_shell::ScoreShellLayout {
        crate::ui::score_shell::compute_layout(state.gpu.config.width, state.gpu.config.height)
    }

    fn handle_score_shell_mouse_move(state: &mut AppState) {
        let layout = Self::score_shell_layout(state);
        state.score_shell_state.continue_hovered =
            layout.hit_continue(state.cursor_x.round() as i32, state.cursor_y.round() as i32);
    }

    fn handle_score_shell_mouse_down(state: &mut AppState) {
        let layout = Self::score_shell_layout(state);
        let inside =
            layout.hit_continue(state.cursor_x.round() as i32, state.cursor_y.round() as i32);
        state.score_shell_state.continue_pressed = inside;
        if inside {
            Self::play_main_menu_button_sound(state);
        }
    }

    /// Release inside the button leaves the score screen. This is the only exit:
    /// the native dialog is modal with one Continue button and dismisses to the
    /// shell, so there is no cancel path to mirror.
    fn handle_score_shell_mouse_up(state: &mut AppState) {
        let layout = Self::score_shell_layout(state);
        let inside =
            layout.hit_continue(state.cursor_x.round() as i32, state.cursor_y.round() as i32);
        let activated = state.score_shell_state.continue_pressed && inside;
        state.score_shell_state.continue_pressed = false;
        if activated {
            Self::leave_mission_result_screen(state);
        }
    }

    /// Shared teardown for both result-screen forms: flush the deterministic log
    /// while its simulation is still alive, hand the scenario stream back to the
    /// offline shell, then drop the match.
    fn leave_mission_result_screen(state: &mut AppState) {
        crate::app_sim_tick::flush_replay_log(state);
        Self::capture_returned_skirmish_rng(state);
        crate::app_loading::clear_match_startup_state(state);
        state.scenario_elapsed_clock.reset();
        state.score_screen = None;
        state.score_shell_state = Default::default();
        state.screen = GameScreen::MainMenu;
        Self::enter_shell_window_mode(state);
        state.zoom_level = 1.0;
        state.zoom_target = 1.0;
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
        Self::apply_selected_shell_map_index(state, map_idx)
    }

    /// One production mutation point for shell selection plus retained-RMG
    /// identity invalidation, whether selection came from Cooperative repair or
    /// the Choose Map dialog.
    fn apply_selected_shell_map_index(state: &mut AppState, map_idx: usize) -> bool {
        let Some(file_name) = state
            .skirmish_shell_maps
            .get(map_idx)
            .map(|map| map.file_name.clone())
        else {
            return false;
        };
        crate::ui::skirmish_shell::accept_selected_map(
            &mut state.skirmish_shell_state,
            &state.skirmish_shell_maps,
            map_idx,
        );
        state.random_map_retention.select_map(&file_name);
        if let Some(legacy_idx) = state
            .available_maps
            .iter()
            .position(|map| map.file_name.eq_ignore_ascii_case(&file_name))
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
        // Prepare dialog 0x102 from the process-lifetime MIX list before
        // destroying its 0x100 source. Active YR's FUN_00534E50 registers the
        // neutral pair on that shared list before the shell SHPs are loaded.
        if !Self::ensure_skirmish_shell_chrome(state) {
            log::warn!("Skirmish shell chrome unavailable; retaining the Single Player shell");
            return;
        }

        // Native destroys dialog 0x100 and its child 0x71A movie handle before
        // constructing 0x102. Drop the hidden Rust session as well so returning
        // to 0x100 cannot continue the pre-Skirmish RA2TS timeline.
        crate::app_main_menu_shell_render::clear_ra2ts_movie_session(state);
        state.main_menu_show_single_player_shell = false;
        state.main_menu_show_native_skirmish_shell = true;
        state.skirmish_shell_return_to_single_player_shell = true;
        state.skirmish_shell_state.pressed_owner_draw_button = None;
        state.skirmish_shell_last_painted_pressed_button = None;
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
        let retained_random_map = state
            .random_map_retention
            .take_for_loading(session.selected_map_file.as_deref());
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
        }
        .with_retained_random_map(retained_random_map);
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

    pub(crate) fn ensure_skirmish_shell_chrome(state: &mut AppState) -> bool {
        if state.skirmish_shell_chrome.is_some() {
            return true;
        }

        let Some(assets) = state.asset_manager.as_ref() else {
            log::warn!(
                "Could not prepare Skirmish shell chrome: process asset manager is unavailable"
            );
            return false;
        };

        state.skirmish_shell_chrome =
            crate::render::skirmish_shell_chrome::build_skirmish_shell_chrome_atlas(
                &state.gpu,
                &state.batch_renderer,
                assets,
            );
        let ready = state.skirmish_shell_chrome.is_some();
        if !ready {
            log::warn!(
                "Could not prepare Skirmish shell chrome from the registered retail archives"
            );
        }
        ready
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

    fn load_version_txt() -> String {
        crate::util::version::retail_internal_version().to_owned()
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
            Self::enter_shell_window_mode(state);
            if dev_shell_enabled {
                if Self::ensure_skirmish_shell_chrome(state) {
                    state.dev_skirmish_shell_enabled = true;
                } else {
                    state.dev_skirmish_shell_enabled = false;
                    log::warn!(
                        "Development Skirmish shell unavailable; retaining the current shell"
                    );
                }
            } else {
                state.dev_skirmish_shell_enabled = false;
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
            .map(|csf| csf.text(key).into_owned())
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
        let applied = Self::apply_selected_shell_map_index(state, map_idx);
        debug_assert!(applied, "validated chooser map index must remain loadable");

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
            let armed = modal.press_button(button, &state.skirmish_modes);
            let _ = modal;
            if armed {
                Self::play_main_menu_button_sound(state);
            }
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
        let released_button = crate::ui::skirmish_shell::choose_map_modal_button_at(&layout, x, y);
        let (had_pressed_button, fired_button) =
            modal.release_button(released_button, &state.skirmish_modes);
        let Some(fired_button) = fired_button else {
            return layout.dialog.contains(x, y) || had_pressed_button;
        };

        let mut selection_to_commit = None;
        let mut close_modal = false;
        // Copied out inside the arm so the `modal` borrow ends before anything
        // below reborrows `state`. `ChooseMapSelection` is `Copy`.
        let mut open_random_map_setup = None;
        match fired_button {
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
                    &mut state.frontend_main_rng,
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
        // A second Generate makes the previous dialog result stale immediately,
        // even when setup cannot progress far enough to spawn the worker.
        state.random_map_retention.begin_generation();
        let Some(asset_manager) = state.asset_manager.as_mut() else {
            return false;
        };
        let settings = crate::map::rmg::RmgSettings::load(asset_manager);
        let theater_name = crate::map::rmg::emit::theater_name(options.theater);
        let Some(theater) = crate::map::theater::load_theater(asset_manager, theater_name) else {
            log::warn!("random map: theater {theater_name} unavailable");
            return false;
        };
        // Stock RMG preview publishes its resolved theater registry before the
        // later ordinary map load, even if generation subsequently fails.
        state
            .tile_variant_selector_cache
            .complete_theater_registry_load(
                theater.rmg_tiles.clear_tile,
                theater.rmg_tiles.water_set,
            );
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
            state.random_map_retention.finish_generation(*generated);
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

    /// Remove the setup dialog and any in-flight worker without changing the
    /// retention disposition already chosen by accept or cancel.
    fn dismiss_random_map_setup(state: &mut AppState) {
        state.skirmish_shell_state.random_map_setup_modal = None;
        state.random_map_generation = None;
    }

    /// Cancel the setup dialog, abandoning any generation and every retained
    /// result associated with this random-map selection.
    ///
    /// Dropping the job drops the receiver, so a worker still going finds a
    /// closed channel on its next send and its remaining output goes nowhere.
    /// That matters beyond tidiness: a late finish would otherwise overwrite
    /// `RandMap.img`, changing the chooser's thumbnail to a map the player
    /// walked away from.
    fn cancel_random_map_setup(state: &mut AppState) {
        Self::dismiss_random_map_setup(state);
        state.random_map_retention.cancel_setup();
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
            Ok(()) => {
                state.random_map_retention.accept_setup(RANDMAP_SED_FILE);
                // Successful OK already chose the retained result; dialog
                // teardown must not run the cancellation invalidation path.
                Self::dismiss_random_map_setup(state);
            }
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
        state: &mut AppState,
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
        state: &mut AppState,
        job: &RandomMapGenerationJob,
        map_file: &crate::map::map_file::MapFile,
        start_waypoints: &[(u8, u16, u16)],
    ) -> Option<crate::map::rmg::preview::PreviewImage> {
        // LAT defaults off for runtime maps, so resolve the same way the load
        // path will; a different setting here would colour cells the player
        // never sees.
        let resolved_terrain = {
            let frontend_main_rng = &mut state.frontend_main_rng;
            let selector_cache = &mut state.tile_variant_selector_cache;
            let asset_manager = state.asset_manager.as_ref();
            let mut raw_draw = || frontend_main_rng.next_u32();
            let mut selector = selector_cache.begin_load(&mut raw_draw);
            // RMG InitMap supplies explicit Clear cells. Its preview never
            // borrows a Scenario cursor; equal-bound Fill remains zero-cost.
            let mut scenario_fill_ranged = |low, high| {
                debug_assert_eq!((low, high), (0, 0));
                0
            };
            crate::map::resolved_terrain::ResolvedTerrainGrid::build_with_variant_selector(
                map_file,
                Some(&job.theater),
                asset_manager,
                Some(&job.terrain_rules),
                None,
                None,
                false,
                RANDOM_MAP_PREVIEW_CLIFF_BACK_IMPASSABILITY,
                &mut scenario_fill_ranged,
                &mut selector,
            )
        };
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
            .flat_map(|px| {
                crate::render::native_surface_format::ACTIVE_RETAIL_RGB565_PRESENTATION
                    .storage_roundtrip_rgb8([px[0], px[1], px[2]])
            })
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
            .map(|csf| csf.text(RANDOM_MAP_DESCRIPTION_KEY).into_owned())
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
                modal.randomize_options(&settings, &mut state.frontend_main_rng, &description);
            }
            Control::Generate0x620 => {
                modal.reroll_derived_for_generate(&settings, &mut state.frontend_main_rng);
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
                    modal.reroll_derived_for_generate(&settings, &mut state.frontend_main_rng);
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
            Self::cancel_random_map_setup(state);
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
            .map(|csf| csf.text(key).into_owned())
            .unwrap_or_default()
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

    fn drive_scenario_exit(state: &mut AppState, wall_ms: u64) {
        if state.scenario_exit.is_none() {
            return;
        }
        let poll_voices = state
            .scenario_exit
            .as_ref()
            .is_some_and(|exit| exit.needs_voice_poll(wall_ms));
        let voices_active = poll_voices
            && state
                .sfx_player
                .as_mut()
                .is_some_and(|sfx| sfx.pump_and_check_voices());
        let tick = state
            .scenario_exit
            .as_mut()
            .expect("scenario exit remains present")
            .tick(wall_ms, voices_active);

        if let Some(scale) = tick.music_output_scale {
            if let Some(player) = state.music_player.as_mut() {
                player.set_output_scale(scale);
            }
        }
        if let Some(scale) = tick.sfx_output_scale {
            if let Some(player) = state.sfx_player.as_mut() {
                player.set_output_scale(scale);
            }
        }
        if tick.stop_audio {
            if let Some(player) = state.music_player.as_mut() {
                player.stop();
                player.set_output_scale(1.0);
            }
            if let Some(player) = state.sfx_player.as_mut() {
                player.stop_all();
                player.set_output_scale(1.0);
            }
        }
        // ScoreDialog__WndProc @ 0x005C9B10 resolves the literal SCORE theme
        // and starts it immediately on WM_INITDIALOG. Keep this after the
        // hard stop and output-scale restoration so ScoreX begins audible.
        if let Some(crate::app_scenario_exit::ScenarioExitAudioAction::PlayTheme(theme)) =
            tick.after_stop
            && let (Some(player), Some(assets)) = (&mut state.music_player, &state.asset_manager)
        {
            let _ = player.play_track(theme, assets);
        }
        if !tick.finished {
            return;
        }

        let destination = state
            .scenario_exit
            .as_mut()
            .and_then(crate::app_scenario_exit::ScenarioExitCascade::take_destination);
        state.scenario_exit = None;
        match destination {
            Some(crate::app_scenario_exit::ScenarioExitDestination::Score {
                title,
                detail,
                model,
            }) => {
                state.score_screen = Some(model);
                state.score_shell_state = Default::default();
                state.screen = GameScreen::MissionResult { title, detail };
            }
            Some(crate::app_scenario_exit::ScenarioExitDestination::MainMenu) => {
                Self::return_to_main_menu(state);
            }
            None => log::error!("Scenario exit finished without a destination"),
        }
    }

    fn apply_scenario_exit_voice_action(
        state: &mut AppState,
        action: crate::app_scenario_exit::ScenarioExitVoiceAction,
    ) {
        match action {
            crate::app_scenario_exit::ScenarioExitVoiceAction::InterruptBattleControlTerminated => {
                let Some(owner) = state.local_player_owner.as_deref() else {
                    log::warn!("Battle-control termination EVA has no pinned local owner");
                    return;
                };
                let faction = crate::app_building_anim::eva_faction_key(owner, &state.house_roster);
                let fallback = match faction {
                    "Russian" => "csof015",
                    "Yuri" => "cyur015",
                    _ => "ceva015",
                };
                let sound_id = state
                    .eva_registry
                    .get("EVA_BattleControlTerminated", faction)
                    .unwrap_or(fallback)
                    .to_string();
                if let (Some(sfx), Some(assets)) = (&mut state.sfx_player, &state.asset_manager) {
                    let _ = sfx.interrupt_eva_sound(
                        &sound_id,
                        &state.sound_registry,
                        assets,
                        &state.audio_indices,
                    );
                }
            }
        }
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
        let now_ms = crate::app_sim_tick::monotonic_frame_pacer_ms(state, Instant::now());
        if let (Some(player), Some(assets)) = (&mut state.music_player, &state.asset_manager) {
            player.play_menu_theme(assets);
            player.update(assets, now_ms);
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
        let _ = Self::ensure_skirmish_shell_chrome(state);
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

    pub fn new(startup_options: crate::app_startup_options::RetailStartupOptions) -> Self {
        Self {
            state: None,
            shell_capture: None,
            tactical_capture: None,
            startup_audio: StartupAudioDisposition::for_audio_enabled(
                startup_options.audio_enabled,
            ),
        }
    }

    pub fn new_shell_capture(request: crate::app_shell_capture::ShellCaptureRequest) -> Self {
        Self {
            state: None,
            shell_capture: Some(crate::app_shell_capture::ShellCaptureSession::new(request)),
            tactical_capture: None,
            startup_audio: StartupAudioDisposition::default(),
        }
    }

    pub fn new_tactical_capture(request: crate::app_launch::TacticalCaptureRequest) -> Self {
        Self {
            state: None,
            shell_capture: None,
            tactical_capture: Some(
                crate::app_tactical_capture::session::TacticalCaptureSession::new(request),
            ),
            startup_audio: StartupAudioDisposition::default(),
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

impl App {
    /// Hand the scenario RNG cursor back to the offline shell when a match ends.
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
        state.in_game_menu = crate::ui::pause_menu::InGameMenuState::Closed;
        state.in_game_options_anchor = None;
        // Persist the deterministic diagnostic log before its owning sim is
        // torn down.
        crate::app_sim_tick::flush_replay_log(state);
        Self::capture_returned_skirmish_rng(state);
        crate::app_loading::clear_match_startup_state(state);
        state.scenario_elapsed_clock.reset();
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

    /// Apply one foreground-activation edge.
    ///
    /// gamemd handles `WM_ACTIVATEAPP` edge-triggered — it compares the new
    /// value against the stored one and does nothing when it is unchanged — and
    /// runs a focus-restore on the regain edge that flushes the recorded
    /// keyboard and mouse state. Held keys are dropped on both edges here: the
    /// modifier that performed an Alt+Tab never delivers its release to this
    /// window, so it would otherwise stay latched and suppress force-fire and
    /// the ordinary cursor for the rest of the match.
    fn set_window_active(state: &mut AppState, active: bool) {
        if state.window_active == active {
            return;
        }
        state.window_active = active;
        state.keys_held.clear();
        state.hotkey_modifiers = ModifiersState::empty();
        state.type_select.clear_held();
        // gamemd-derived: the `WM_ACTIVATEAPP` changed edge at 0x007778AC
        // stops/restores the primary DirectSound output through 0x00407020 /
        // 0x00407040 while secondary playback cursors continue. Keep this on
        // the same edge as the main-loop gate rather than pausing each stream.
        if let Some(player) = state.music_player.as_mut() {
            player.set_focus_output_active(active);
        }
        if let Some(player) = state.sfx_player.as_mut() {
            player.set_focus_output_active(active);
        }
        if active {
            // The deactivated span must not buy a catch-up frame: forget the
            // pacing window so exactly one frame runs immediately, then normal
            // pacing resumes.
            state.frame_pacer.reset_for_immediate_frame();
            state.window.request_redraw();
        }
        log::info!(
            "Window {}",
            if active { "activated" } else { "deactivated" }
        );
    }

    /// Apply one window-visibility edge. Waking on the un-hide edge is what
    /// gets the parked redraw loop turning again.
    fn set_window_hidden(state: &mut AppState, hidden: bool) {
        if state.window_hidden == hidden {
            return;
        }
        state.window_hidden = hidden;
        if !hidden {
            state.window.request_redraw();
        }
    }

    /// Does the in-scenario modal machine own this Escape press?
    ///
    /// gamemd reaches the in-game menu from the sidebar menu control; this port
    /// has no such control yet, so Escape stands in for it. Both the binding and
    /// this precedence are VERA-internal — gamemd's keyboard route into the menu
    /// is UNCHECKED. Escape keeps its in-world cancel duties: while no modal is
    /// open and a placement/targeting or repair/sell mode is armed, Escape
    /// cancels that instead and the machine stays out of it.
    fn in_game_menu_owns_escape(state: &AppState) -> bool {
        let in_world_mode_armed = state.targeting_mode.is_some()
            || state.sidebar_gadget_state.repair_mode_on
            || state.sidebar_gadget_state.sell_mode_on;
        crate::ui::pause_menu::escape_belongs_to_modal_machine(
            state.in_game_menu,
            in_world_mode_armed,
        )
    }

    /// Route one Escape press through the in-scenario modal machine.
    fn route_in_game_menu_escape(state: &mut AppState) {
        use crate::ui::pause_menu::InGameMenuState;

        // Backing out of Options takes the same exit its Back control does —
        // apply and persist the touched `[Options]` values — so the two ways of
        // leaving the dialog cannot disagree about what was saved.
        if state.in_game_menu == InGameMenuState::Options {
            crate::app_options_persist::in_game_options_close(state);
        }
        let next = state.in_game_menu.on_escape();
        Self::enter_in_game_menu_state(state, next);
    }

    /// Commit an in-scenario modal transition and the app-layer effects that
    /// ride on it.
    ///
    /// The simulation freezes for every non-zero state: gamemd's modal pump
    /// never reaches its main tick in offline campaign or skirmish, so no
    /// per-tick update, frame-counter step or tactical recomposition happens
    /// while a dialog is up. Freezing does not skip ticks — the tick simply
    /// stops advancing and resumes from the same number, so the tick stream is
    /// unchanged and a replay of the match still reproduces.
    fn enter_in_game_menu_state(
        state: &mut AppState,
        next: crate::ui::pause_menu::InGameMenuState,
    ) {
        use crate::ui::pause_menu::InGameMenuState;

        let previous = state.in_game_menu;
        if previous == next {
            return;
        }
        let was_open = previous.is_open();
        let will_be_open = next.is_open();
        if was_open != will_be_open
            && !crate::app_sim_tick::current_session_mode(state).is_network()
        {
            let now_ms = crate::app_sim_tick::monotonic_frame_pacer_ms(state, Instant::now());
            if will_be_open {
                state.scenario_elapsed_clock.pause(now_ms);
            } else {
                state.scenario_elapsed_clock.resume(now_ms);
            }
        }
        state.in_game_menu = next;

        // Leaving Options: drop the cached `0xBBB` hit-test anchor so the
        // overlay's own mouse handler cannot claim clicks aimed at the menu.
        if previous == InGameMenuState::Options {
            state.in_game_options_anchor = None;
        }
        if next == InGameMenuState::Options {
            // Reset the transient interaction flags so the drag-gated
            // value-label quirk resets on every open.
            state.in_game_options.on_open();
        }

        state.paused = next.is_open();
        if next.is_open() {
            // Show the OS cursor so the modal is clickable.
            if state.software_cursor.is_some() {
                state.window.set_cursor_visible(true);
            }
        } else {
            // Elapsed modal time must not cause a catch-up frame.
            state.frame_pacer.reset_for_immediate_frame();
            if state.software_cursor.is_some() {
                state.window.set_cursor_visible(false);
            }
        }
        state.window.request_redraw();
    }

    /// Draw whichever in-scenario modal card is open and commit its route.
    ///
    /// Options is the native `0xBBB` overlay, drawn earlier in the frame; this
    /// only reconciles the machine when that overlay closes itself.
    fn handle_in_game_menu(state: &mut AppState) {
        use crate::ui::pause_menu::{self, InGameMenuState, ModalOutcome};

        let outcome = match state.in_game_menu {
            InGameMenuState::Closed => ModalOutcome::Stay,
            InGameMenuState::Menu => {
                pause_menu::resolve_menu_action(pause_menu::draw_in_game_menu(&state.egui.ctx))
            }
            InGameMenuState::AbortConfirm => {
                let leave_label =
                    Self::csf_label(state, ABORT_CONFIRM_LEAVE_KEY, ABORT_CONFIRM_LEAVE_FALLBACK);
                pause_menu::resolve_abort_action(pause_menu::draw_abort_confirm(
                    &state.egui.ctx,
                    &leave_label,
                ))
            }
            // Options is the native `0xBBB` overlay, drawn earlier in the frame
            // and reconciled by `sync_in_game_menu_with_options_overlay`.
            InGameMenuState::Options => ModalOutcome::Stay,
        };

        match outcome {
            ModalOutcome::Stay => {}
            ModalOutcome::Enter(next) => Self::enter_in_game_menu_state(state, next),
            ModalOutcome::LeaveMatch => Self::exit_match_to_shell(state),
        }
    }

    /// Re-enter the in-game menu when the Options overlay closes itself.
    ///
    /// gamemd's Options dialog is a **child** of the in-game menu: when it
    /// returns, the state machine writes state 1, so closing Options puts the
    /// player back on the menu rather than back into the mission. The `0xBBB`
    /// overlay in this port owns its own Back handler and clears `paused`
    /// there, so the parent state is re-asserted here — before the frame's
    /// simulation advance, so the child's close cannot leak a stray tick.
    fn sync_in_game_menu_with_options_overlay(state: &mut AppState) {
        use crate::ui::pause_menu::InGameMenuState;

        if state.in_game_menu == InGameMenuState::Options && !state.paused {
            Self::enter_in_game_menu_state(state, InGameMenuState::Menu);
        }
    }

    /// Queue the running match's native graceful-exit event.
    ///
    /// gamemd's confirmed Abort queues an EXIT event for the local player; when
    /// that event executes it raises the graceful-exit session flag, and the
    /// session-end router tears the session down **without** the victory or
    /// defeat teardown — no outcome announcement, no result screen, straight
    /// back to the shell. Confirmation itself only constructs and queues the
    /// event; teardown begins after the event-tail dispatcher executes it.
    fn exit_match_to_shell(state: &mut AppState) {
        log::info!("Abort Mission confirmed — queueing EXIT event");
        if state.scenario_exit.is_some() {
            return;
        }
        let Some(owner) = crate::app_commands::preferred_local_owner(state) else {
            log::warn!("Abort Mission confirmation has no local command owner");
            return;
        };
        if crate::app_commands::try_schedule_command(
            state,
            &owner,
            crate::sim::command::Command::ExitMatch,
        )
        .is_none()
        {
            log::warn!("Abort Mission EXIT event could not be queued for '{owner}'");
            return;
        }
        Self::enter_in_game_menu_state(state, crate::ui::pause_menu::InGameMenuState::Closed);
    }

    /// Consume EventClass opcode `0x13`'s executed edge and enter the existing
    /// battle-abort teardown. A repeated drain without another dispatch is a
    /// no-op because the simulation edge is taken.
    fn consume_executed_abort_exit(state: &mut AppState, wall_ms: u64) {
        let local_owner = crate::app_commands::preferred_local_owner(state);
        let local_owner_id = local_owner.as_deref().and_then(|owner| {
            state
                .simulation
                .as_ref()
                .and_then(|sim| sim.interner.get(owner))
        });
        let local_outcome_exit_ready = local_owner_id.is_some_and(|owner| {
            state
                .simulation
                .as_ref()
                .and_then(|sim| sim.houses.get(&owner))
                .and_then(|house| house.outcome_state)
                .is_some_and(|outcome| outcome.exit_ready)
        });
        let executed_owner = state
            .simulation
            .as_mut()
            .and_then(|sim| sim.take_executed_exit_owner());
        let Some(executed_owner) = executed_owner else {
            return;
        };
        if Some(executed_owner) != local_owner_id {
            log::warn!("Ignoring executed EXIT event not owned by the local player");
            return;
        }
        if matches!(
            crate::app_scenario_exit::arbitrate_executed_exit(local_outcome_exit_ready),
            crate::app_scenario_exit::ExecutedExitDisposition::Outcome
        ) {
            // Main_Game observes the ready victory/loss route first. The EXIT
            // edge is still consumed, but cannot clear or replace that route.
            return;
        }
        if state.scenario_exit.is_some() {
            return;
        }

        // Abort is the independent EXIT-event route: it never inherits the
        // victory/defeat HouseClass SavourDelay or its outcome-voice wait.
        state.scenario_outcome = None;
        let _ = state.scenario_elapsed_clock.stop(wall_ms);
        // gamemd provenance: battle abort teardown; verified
        // GameExit__BattleControlTerminated @ 0x00686570 starts Theme's fade,
        // then fades the independent audio master, bounds its voice pump to
        // 300 timer buckets, and finally hard-stops audio.
        let mut scenario_exit = crate::app_scenario_exit::ScenarioExitCascade::start(
            wall_ms,
            crate::app_scenario_exit::ScenarioExitDestination::MainMenu,
        );
        // `0x00686570` requests EVA_BattleControlTerminated as an INTERRUPT
        // immediately before waiting for the two simultaneous audio fades.
        if let Some(action) = scenario_exit.take_start_voice_action() {
            Self::apply_scenario_exit_voice_action(state, action);
        }
        state.scenario_exit = Some(scenario_exit);
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
    fn gsi_01_01_noaudio_suppresses_music_and_sfx_output() {
        let disposition = StartupAudioDisposition::for_audio_enabled(false);

        assert!(!disposition.initialize_music_output);
        assert!(!disposition.initialize_sfx_output);
        assert!(!disposition.load_audio_indices);
    }

    #[test]
    fn gsi_01_01_default_startup_enables_music_and_sfx_output() {
        let disposition = StartupAudioDisposition::default();

        assert!(disposition.initialize_music_output);
        assert!(disposition.initialize_sfx_output);
        assert!(disposition.load_audio_indices);
    }

    #[test]
    fn gsi_01_01_audio_index_decision_survives_scenario_transitions() {
        for audio_enabled in [false, true] {
            let startup = StartupAudioDisposition::for_audio_enabled(audio_enabled);
            let persisted_in_state = startup.load_audio_indices;

            assert_eq!(
                should_load_audio_indices(startup.load_audio_indices),
                audio_enabled
            );
            assert_eq!(should_load_audio_indices(persisted_in_state), audio_enabled);
        }
    }

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

    fn retained_map(seed: i32, start_x: u16) -> crate::map::rmg::GeneratedMap {
        let mut options = crate::map::rmg::RmgOptions::default();
        options.seed = seed;
        crate::map::rmg::GeneratedMap {
            map_file: crate::map::rmg::emit::empty_map_file(&options, 32, 32),
            start_waypoints: vec![(0, start_x, 20)],
            stages_run: Vec::new(),
            unfilled_start_slots: 0,
        }
    }

    #[test]
    fn gsi_03_09_random_map_retention_invalidates_and_transfers_exactly_once() {
        let mut regenerated = RandomMapGenerationRetention::default();
        regenerated.finish_generation(retained_map(11, 10));
        regenerated.accept_setup("RandMap.Sed");
        regenerated.begin_generation();
        assert!(
            regenerated.take_for_loading(Some("RandMap.Sed")).is_none(),
            "starting a genuine regeneration invalidates accepted map A"
        );

        let mut reopened_then_cancelled_without_generate = RandomMapGenerationRetention::default();
        reopened_then_cancelled_without_generate.finish_generation(retained_map(12, 11));
        reopened_then_cancelled_without_generate.accept_setup("RandMap.Sed");
        reopened_then_cancelled_without_generate.cancel_setup();
        assert!(
            reopened_then_cancelled_without_generate
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "a genuine setup Cancel invalidates accepted map A"
        );

        let mut reopened_then_cancelled = RandomMapGenerationRetention::default();
        reopened_then_cancelled.finish_generation(retained_map(13, 12));
        reopened_then_cancelled.accept_setup("RandMap.Sed");
        reopened_then_cancelled.begin_generation();
        reopened_then_cancelled.finish_generation(retained_map(14, 13));
        reopened_then_cancelled.cancel_setup();
        assert!(
            reopened_then_cancelled
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "reopen, regenerate, then Cancel cannot resurrect accepted map A"
        );

        let mut cancelled = RandomMapGenerationRetention::default();
        cancelled.finish_generation(retained_map(22, 20));
        cancelled.cancel_setup();
        cancelled.accept_setup("RandMap.Sed");
        assert!(cancelled.take_for_loading(Some("RandMap.Sed")).is_none());

        let mut selected_elsewhere = RandomMapGenerationRetention::default();
        selected_elsewhere.finish_generation(retained_map(33, 30));
        selected_elsewhere.accept_setup("RandMap.Sed");
        selected_elsewhere.select_map("mp01t4.map");
        assert!(
            selected_elsewhere
                .take_for_loading(Some("RandMap.Sed"))
                .is_none()
        );

        let mut alternate_seed = RandomMapGenerationRetention::default();
        alternate_seed.finish_generation(retained_map(34, 35));
        alternate_seed.accept_setup("RandMap.Sed");
        // This is the retention authority used by apply_selected_shell_map_file:
        // another seed selection must invalidate, not merely refuse this call.
        alternate_seed.select_map("Other.Sed");
        assert!(
            alternate_seed
                .take_for_loading(Some("RandMap.Sed"))
                .is_none()
        );
        assert!(alternate_seed.take_for_loading(Some("Other.Sed")).is_none());

        let mut mismatched_launch = RandomMapGenerationRetention::default();
        mismatched_launch.finish_generation(retained_map(35, 36));
        mismatched_launch.accept_setup("RandMap.Sed");
        assert!(
            mismatched_launch
                .take_for_loading(Some("Other.Sed"))
                .is_none()
        );
        assert!(
            mismatched_launch
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "a refused nonmatching launch invalidates the prior accepted map"
        );

        let mut accepted_after_successful_close = RandomMapGenerationRetention::default();
        accepted_after_successful_close.finish_generation(retained_map(44, 40));
        accepted_after_successful_close.accept_setup("RandMap.Sed");
        // App::dismiss_random_map_setup has no retention side effect after OK.
        accepted_after_successful_close.select_map("RANDMAP.SED");
        let transferred = accepted_after_successful_close
            .take_for_loading(Some("randmap.sed"))
            .expect("successful accept close preserves the newly accepted map");
        assert_eq!(transferred.start_waypoints, vec![(0, 40, 20)]);
        assert!(
            accepted_after_successful_close
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "successful accept still transfers exactly once"
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
