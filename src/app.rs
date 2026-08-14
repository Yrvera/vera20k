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
mod in_game;
mod shell_random_map;
mod state;

pub(crate) use shell_random_map::{
    RandomMapGenerationJob, RandomMapGenerationRetention,
};
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

/// Caption gamemd loads onto the abort-mission confirmation's action button.
/// Its two mode-dependent siblings (`GUI:Restart` in campaign, `GUI:Observe` in
/// multiplayer) sit on a second button that offline skirmish hides outright.
const ABORT_CONFIRM_LEAVE_KEY: &str = "GUI:Leave";
/// The shipped English table resolves `GUI:Leave` to "Quit"; the fallback only
/// applies when the string table is missing entirely, so it says the same.
const ABORT_CONFIRM_LEAVE_FALLBACK: &str = "Quit";
const DEV_SKIRMISH_SHELL_ENV: &str = "RA2_DEV_SKIRMISH_SHELL";
const SHELL_WINDOW_WIDTH: u32 = 800;
const SHELL_WINDOW_HEIGHT: u32 = 600;

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
