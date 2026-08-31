//! App-level loading screen state and parity primitives.
//!
//! This module sits above simulation and owns loading-screen progress behavior
//! verified from gamemd.exe. It also owns the request/session boundary used by
//! the app loop before map-load phases are split into a fully pumpable job.

use crate::app::AppState;
use crate::app::loading::init::{self, MapLoadInitial, MapLoadResult};
use crate::app::loading::fresh_scenario::FreshScenarioLoadContextDescriptor;
use crate::app::loading::composition::{
    LoadingCompositionSnapshot, LoadingParticipantId, LoadingStartAssignment, MmpbRegionRect,
    RANDOM_MAP_PREVIEW_FILE, build_loading_composition, build_random_map_loading_composition,
    loading_base_origin,
};
use crate::app::loading::progress_row::{
    LoadingProgressRowLayout, LoadingProgressRowSnapshot, layout_standard_skirmish_progress_row,
};
use crate::sim::scenario_bootstrap::StockOfflinePrefixProjection;
use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Color;
use crate::assets::pcx_file::PcxFile;
use crate::map::preview::DecodedPreview;
use crate::match_bootstrap::{LoadingStartup, PreparedMatchStartup};
use crate::render::batch::{BatchRenderer, SpriteInstance};
use crate::render::bit_font::BitFont;
use crate::render::draw_state::DrawState;
use crate::render::gpu::GpuContext;
use crate::render::loading_screen_chrome::{
    LoadingArtVariant, LoadingScreenAtlas, LoadingScreenCompositionAtlasInput, LoadingScreenEntry,
    LoadingScreenWidth, MmpbMarkerRemap, PreparedLoadingPreviewRgba,
    build_loading_screen_atlas_with_composition,
};
use crate::render::shell_surface_present::ShellSurfacePresenter;
use crate::render::shell_text::{ScissorRect, ShellAlign, ShellTextDraw, TextRect, draw_in_rect};
use crate::rules::color_scheme::{
    ColorSchemeEntry, hsv_to_rgb, scheme_entry_by_name, scheme_entry_for_priority,
    scheme_hsv_by_entry,
};
use crate::rules::house_colors::{HouseColorIndex, HouseColorRamps};
use crate::skirmish_launch::{LaunchCountry, SkirmishLaunchSession};
use crate::ui::game_screen::GameScreen;
use crate::ui::main_menu::SkirmishSettings;
use std::path::{Path, PathBuf};

const STANDARD_SKIRMISH_PROGRESS_MAX: f64 = 100.0;
const PROGRESS_PERCENT_SCALE: f64 = 0.01;
const PERCENT_DISPLAY_SCALE: f64 = 100.0;
const FTOL_EPSILON: f64 = 0.000_001;
const BACKGROUND_DEPTH: f32 = 0.90;
const PREVIEW_DEPTH: f32 = 0.80;
const MARKER_DEPTH: f32 = 0.70;
const TEXT_BACKING_DEPTH: f32 = 0.60;
const TEXT_DEPTH: f32 = 0.50;
const TEXT_BACKING_ALPHA: f32 = 159.0 / 255.0;
const TEXT_BACKING_PADDING: f32 = 2.0;
/// Solid backing fill (G3) sits just behind the bar so the bar draws over it.
const SOLID_FILL_DEPTH: f32 = 0.20;
/// Static loading colors used only when rules `[Colors]` are unavailable
/// (missing assets / headless); replaced by `resolve_player_colors` once rules
/// load.
const FALLBACK_BACKING_RGB: [f32; 3] = [0.22, 0.22, 0.22];
const FALLBACK_PROGRESS_RAMP: [Color; 16] = [Color::rgb(180, 180, 180); 16];
const PROGRESS_DEPTH: f32 = 0.10;
/// Side icon (G4) draws above the background, at the bar's depth.
const SIDE_ICON_DEPTH: f32 = 0.10;
/// Row label follows the bar and country insignia.
const ROW_LABEL_DEPTH: f32 = 0.05;

/// Effective selected-map standard offline Skirmish milestones after first LS draw.
///
/// The theater ramp between 13 and 25 is dynamic in gamemd.exe, so it is modeled
/// by `theater_ramp_changed_values` rather than hardcoded as every integer.
pub const STANDARD_SKIRMISH_SELECTED_MAP_MILESTONES_AFTER_FIRST_RENDER: &[u32] = &[
    3, 8, 12, 25, 30, 31, 35, 45, 50, 55, 58, 60, 63, 65, 67, 68, 69, 70, 72, 74, 76, 78, 82, 86,
    90, 93, 96, 98, 100,
];

/// Raw native calls that should not create a redraw in the normal selected-map path.
pub const STANDARD_SKIRMISH_NONADVANCING_RAW_CALLS: &[u32] = &[6, 58, 60, 25];

#[derive(Debug, Clone, PartialEq)]
pub struct LoadingProgressState {
    max_value: f64,
    current_value: f64,
}

impl LoadingProgressState {
    pub fn standard_skirmish() -> Self {
        Self {
            max_value: STANDARD_SKIRMISH_PROGRESS_MAX,
            current_value: 0.0,
        }
    }

    pub fn max_value(&self) -> f64 {
        self.max_value
    }

    pub fn current_value(&self) -> f64 {
        self.current_value
    }

    pub fn current_percent(&self) -> f64 {
        self.current_value / self.max_value * PERCENT_DISPLAY_SCALE
    }

    /// Apply gamemd's loading milestone callback gate plus ProgressClass setter.
    ///
    /// Only strictly advancing callback percentages reach the setter. The setter
    /// stores `max * 0.01 * percent`, clamps only above max, and returns `false`
    /// when the stored value did not change.
    pub fn advance_progress(&mut self, percent: u32) -> bool {
        let requested = f64::from(percent);
        if self.current_percent() >= requested {
            return false;
        }

        self.set_percent(requested)
    }

    fn set_percent(&mut self, percent: f64) -> bool {
        let mut new_value = self.max_value * PROGRESS_PERCENT_SCALE * percent;
        if new_value > self.max_value {
            new_value = self.max_value;
        }
        if self.current_value == new_value {
            return false;
        }
        self.current_value = new_value;
        true
    }

    pub fn fill_width_gamemd_ftol_positive_domain(&self, frame0_width: u32) -> u32 {
        gamemd_ftol_positive_domain(f64::from(frame0_width) * self.current_value / self.max_value)
            .max(0) as u32
    }
}

/// Receives a raw native loading milestone at a real load-phase boundary.
///
/// Implementors apply the monotonic gate (via [`LoadingProgressState::advance_progress`])
/// and may synchronously repaint the loading screen on an advancing milestone,
/// mirroring gamemd's per-milestone synchronous hidden-to-primary blit.
/// Selected maps use raw values through 100; random-map seed loads halve raw
/// values and finish at raw 200.
pub(crate) trait LoadingProgressSink {
    fn milestone(&mut self, percent: u32);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeLoadingProgressCadence {
    SelectedMap,
    RandomMapHalved,
}

impl NativeLoadingProgressCadence {
    fn for_selected_map_file(selected_map_file: &str) -> Self {
        if crate::map::rmg::is_seed_selection(selected_map_file) {
            Self::RandomMapHalved
        } else {
            Self::SelectedMap
        }
    }

    fn effective_percent(self, raw_percent: u32) -> u32 {
        match self {
            Self::SelectedMap => raw_percent,
            // Native integer division truncates raw random-map milestones.
            Self::RandomMapHalved => raw_percent / 2,
        }
    }

    fn terminal_raw_percent(self) -> u32 {
        match self {
            Self::SelectedMap => 100,
            Self::RandomMapHalved => 200,
        }
    }

    /// Whether native has resolved Scenario inputs before the first loading frame.
    ///
    /// Selected maps also need their parsed preview. Random maps take pixels
    /// from `RandMap.img`, but Full Init has already regenerated the `.SED`,
    /// copied accepted start staging, and run both selected-mode callbacks
    /// before DrawLoadingScreen. Both branches therefore swallow the loader's
    /// raw 8 here and hand it to the first post-frame pump.
    fn prepares_scenario_before_first_frame(self) -> bool {
        match self {
            Self::SelectedMap | Self::RandomMapHalved => true,
        }
    }
}

/// A sink that only advances the gated progress state, with no repaint. Used at
/// the pump call sites before the render-triggering sink is constructed, and as
/// the base behavior shared by all sinks.
struct GatedProgressSink<'a> {
    progress: &'a mut LoadingProgressState,
    cadence: NativeLoadingProgressCadence,
}

impl LoadingProgressSink for GatedProgressSink<'_> {
    fn milestone(&mut self, raw_percent: u32) {
        self.progress
            .advance_progress(self.cadence.effective_percent(raw_percent));
    }
}

/// Sink for the generic (non-native) map load, which has no progress bar.
struct NoopProgressSink;

impl LoadingProgressSink for NoopProgressSink {
    fn milestone(&mut self, _percent: u32) {}
}

enum FreshScenarioLoadState {
    Pending,
    Ready(FreshScenarioLoadContextDescriptor),
    Transferred,
}

pub(crate) struct LoadingRequest {
    /// `None` is an internal terminal-transfer marker only. Every live request
    /// owns exactly one explicit startup variant.
    startup: Option<LoadingStartup>,
    fresh_scenario_load: FreshScenarioLoadState,
    /// Accepted setup start staging is small, provenance-bearing gameplay
    /// input. It is never reconstructed from the presentation preview.
    accepted_rmg_start_staging: Option<crate::app::shell_random_map::AcceptedRmgStartStaging>,
    /// Setup-generated preview retained only as a loading-composition fallback.
    /// It never supplies gameplay map data, RNG continuation, or constructors.
    random_map_preview: Option<crate::map::rmg::GeneratedMap>,
    presentation: LoadingPresentation,
    fallback_skirmish_settings: SkirmishSettings,
}

impl LoadingRequest {
    pub(crate) fn accepted_skirmish(
        startup: PreparedMatchStartup,
        fallback_skirmish_settings: SkirmishSettings,
    ) -> Self {
        Self {
            startup: Some(LoadingStartup::Accepted(startup)),
            fresh_scenario_load: FreshScenarioLoadState::Pending,
            accepted_rmg_start_staging: None,
            random_map_preview: None,
            presentation: LoadingPresentation::NativeSelectedSkirmish,
            fallback_skirmish_settings,
        }
    }

    pub(crate) fn unverified_legacy_skirmish(
        skirmish_launch_session: SkirmishLaunchSession,
        seed: crate::match_bootstrap::MatchSeed,
        fallback_skirmish_settings: SkirmishSettings,
    ) -> Self {
        Self {
            startup: Some(LoadingStartup::UnverifiedLegacy {
                session: skirmish_launch_session,
                seed,
            }),
            fresh_scenario_load: FreshScenarioLoadState::Pending,
            accepted_rmg_start_staging: None,
            random_map_preview: None,
            presentation: LoadingPresentation::NativeSelectedSkirmish,
            fallback_skirmish_settings,
        }
    }

    pub(crate) fn generic_map_load(
        selected_map_file: impl Into<String>,
        fallback_skirmish_settings: SkirmishSettings,
    ) -> Self {
        Self {
            startup: Some(LoadingStartup::Generic {
                selected_map_file: selected_map_file.into(),
            }),
            fresh_scenario_load: FreshScenarioLoadState::Pending,
            accepted_rmg_start_staging: None,
            random_map_preview: None,
            presentation: LoadingPresentation::GenericMapLoad,
            fallback_skirmish_settings,
        }
    }

    pub(crate) fn selected_map_file(&self) -> &str {
        self.startup().selected_map_file()
    }

    /// Run the request's authoritative initial-map entry. The retained random
    /// preview is intentionally not an argument: active retail's `.SED` reader
    /// regenerates gameplay after Start, while the preview remains a loading-
    /// composition fallback only.
    pub(crate) fn load_initial_with_assets(
        &self,
        ra2_dir: std::path::PathBuf,
        asset_manager: &mut AssetManager,
        progress: &mut dyn LoadingProgressSink,
    ) -> anyhow::Result<MapLoadInitial> {
        init::load_map_initial_with_assets(
            ra2_dir,
            asset_manager,
            Some(self.selected_map_file()),
            progress,
        )
    }

    fn skirmish_launch_session(&self) -> Option<&SkirmishLaunchSession> {
        self.startup().launch_session()
    }

    fn startup(&self) -> &LoadingStartup {
        self.startup
            .as_ref()
            .expect("live loading request must retain startup authority")
    }

    /// Attach the setup preview for presentation fallback only. Scenario read
    /// 0x00684620 regenerates the accepted `.SED`; that launch result, not this
    /// preview, resolves the stock Scenario prefix and initializes gameplay.
    #[cfg(test)]
    pub(crate) fn with_random_map_preview(
        mut self,
        random_map_preview: Option<crate::map::rmg::GeneratedMap>,
    ) -> Self {
        self.random_map_preview = random_map_preview;
        self
    }

    /// Transfer one accepted setup transaction into loading. Preview terrain,
    /// MapGen continuation, and construction trace remain presentation-only;
    /// the separately extracted staging becomes the active Scenario waypoint
    /// authority for Gather, loading markers, and the live session snapshot.
    pub(crate) fn with_accepted_random_map(
        mut self,
        accepted: Option<crate::app::shell_random_map::AcceptedRandomMapLaunch>,
    ) -> Self {
        if let Some(accepted) = accepted {
            let (preview, start_staging) = accepted.into_parts();
            self.random_map_preview = Some(preview);
            self.accepted_rmg_start_staging = Some(start_staging);
        }
        self
    }

    #[cfg(test)]
    fn random_map_preview(&self) -> Option<&crate::map::rmg::GeneratedMap> {
        self.random_map_preview.as_ref()
    }

    pub(crate) fn prepare_fresh_scenario_load_context(
        &mut self,
        initial: &MapLoadInitial,
    ) -> anyhow::Result<()> {
        match &self.fresh_scenario_load {
            FreshScenarioLoadState::Ready(_) => return Ok(()),
            FreshScenarioLoadState::Transferred => {
                anyhow::bail!("fresh scenario context was already transferred")
            }
            FreshScenarioLoadState::Pending => {}
        }
        self.fresh_scenario_load = FreshScenarioLoadState::Ready(
            FreshScenarioLoadContextDescriptor::admit_stock_offline(
                self.startup
                    .as_ref()
                    .expect("live loading request retains startup authority"),
                initial.map_data(),
                initial.map_source(),
                &mut self.accepted_rmg_start_staging,
            )?,
        );
        Ok(())
    }

    pub(crate) fn fresh_scenario_load_context(
        &self,
    ) -> Option<&FreshScenarioLoadContextDescriptor> {
        match &self.fresh_scenario_load {
            FreshScenarioLoadState::Ready(context) => Some(context),
            FreshScenarioLoadState::Pending | FreshScenarioLoadState::Transferred => None,
        }
    }

    pub(crate) fn take_fresh_scenario_load_context(
        &mut self,
    ) -> anyhow::Result<FreshScenarioLoadContextDescriptor> {
        match &self.fresh_scenario_load {
            FreshScenarioLoadState::Pending => {
                anyhow::bail!("loading transfer attempted before fresh scenario admission")
            }
            FreshScenarioLoadState::Transferred => {
                anyhow::bail!("fresh scenario context transfers exactly once")
            }
            FreshScenarioLoadState::Ready(_) => {}
        }
        match std::mem::replace(
            &mut self.fresh_scenario_load,
            FreshScenarioLoadState::Transferred,
        ) {
            FreshScenarioLoadState::Ready(context) => Ok(context),
            FreshScenarioLoadState::Pending | FreshScenarioLoadState::Transferred => {
                unreachable!("state was checked before terminal move")
            }
        }
    }

    fn take_startup(&mut self) -> LoadingStartup {
        self.startup
            .take()
            .expect("terminal loading phase transfers startup authority once")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoadingPresentation {
    NativeSelectedSkirmish,
    GenericMapLoad,
}

pub(crate) struct NativeLoadingScreenState {
    pub variant: LoadingArtVariant,
    local_side_index: u8,
    /// Local player's MP color scheme — source of the G3 solid backing fill and
    /// bar remap. Derived from the launch session, not the country variant.
    pub color_index: HouseColorIndex,
    /// Empty-bar backing fill color (normalized RGB), resolved from the player's
    /// `[Colors]` scheme HSV. Falls back to a static shade when `[Colors]` is
    /// unavailable (e.g. assets missing).
    pub backing_rgb: [f32; 3],
    /// GAME.FNT loading-copy color from the named AlliedLoad/SovietLoad scheme.
    pub text_rgb: [f32; 3],
    /// Full `[Colors]` band copied into PROGBARM palette indices 16..31 when
    /// the session-local loading atlas is decoded.
    pub progress_ramp: [Color; 16],
    pub progress: LoadingProgressState,
    pub progress_row: LoadingProgressRowSnapshot,
    pub atlas: Option<LoadingScreenAtlas>,
    pub composition: Option<LoadingCompositionSnapshot>,
    pub first_renderer_ready: bool,
    /// Native constructs two runtime ColorScheme objects per current `[Colors]`
    /// entry. Capture that pre-load count before the later rules reset.
    runtime_color_scheme_count: usize,
    progress_cadence: NativeLoadingProgressCadence,
}

impl NativeLoadingScreenState {
    fn standard_skirmish(
        variant: LoadingArtVariant,
        local_side_index: u8,
        color_index: HouseColorIndex,
        progress_row: LoadingProgressRowSnapshot,
        progress_cadence: NativeLoadingProgressCadence,
    ) -> Self {
        Self {
            variant,
            local_side_index,
            color_index,
            // Static placeholders; replaced by `resolve_player_colors` once rules load.
            backing_rgb: FALLBACK_BACKING_RGB,
            text_rgb: [1.0, 1.0, 1.0],
            progress_ramp: FALLBACK_PROGRESS_RAMP,
            progress: LoadingProgressState::standard_skirmish(),
            progress_row,
            atlas: None,
            composition: None,
            first_renderer_ready: false,
            runtime_color_scheme_count: 0,
            progress_cadence,
        }
    }

    /// Resolve the backing fill + progress ramp from the rules `[Colors]` data. The
    /// player's `color_index` is a `[Colors]` entry index: the backing is that
    /// entry's HSV→RGB, and PROGBARM uses all 16 shades of that entry's ramp. Leaves the static
    /// fallbacks in place when the color list is empty/unmatched.
    fn resolve_player_colors(&mut self, schemes: &[ColorSchemeEntry], ramps: &HouseColorRamps) {
        self.runtime_color_scheme_count = schemes.len() * 2;
        let entry = self.color_index.0 as usize;
        if let Some(hsv) = scheme_hsv_by_entry(schemes, entry) {
            self.backing_rgb = normalize_rgb(hsv_to_rgb(hsv));
        }
        let text_scheme = if self.local_side_index == 0 {
            "AlliedLoad"
        } else {
            "SovietLoad"
        };
        if let Some(entry) = scheme_entry_by_name(schemes, text_scheme)
            && let Some(hsv) = scheme_hsv_by_entry(schemes, entry)
        {
            self.text_rgb = normalize_rgb(hsv_to_rgb(hsv));
        } else {
            log::warn!("Missing loading text color scheme {text_scheme}; using white");
        }
        self.progress_ramp = *ramps.ramp(self.color_index);
    }
}

pub(crate) struct LoadingSession {
    pub request: LoadingRequest,
    pub native: Option<NativeLoadingScreenState>,
    job: LoadingJob,
    pub first_frame_presented: bool,
}

impl LoadingSession {
    /// The pump's native admission gate (F07 characterization): a native
    /// session may not pump loader work until its first loading frame
    /// composition is ready to present. `first_renderer_ready` flips only when
    /// the session-local atlas decode completes, and the frame loop calls the
    /// pump strictly after `loading_screen_presented`.
    pub(crate) fn native_pump_blocked(&self) -> bool {
        self.native
            .as_ref()
            .is_some_and(|native| !native.first_renderer_ready)
    }

    fn from_request(request: LoadingRequest) -> Self {
        let native = match (&request.presentation, request.skirmish_launch_session()) {
            (LoadingPresentation::NativeSelectedSkirmish, Some(skirmish_launch_session)) => {
                let variant =
                    loading_art_variant_from_launch_country(skirmish_launch_session.local.country);
                // `local.color_index` is the gamemd color priority; resolve to a
                // `[Colors]` entry index (priority LUT + /2).
                let color_index = HouseColorIndex(scheme_entry_for_priority(
                    skirmish_launch_session.local.color_index as i32,
                ) as u8);
                let progress_cadence = NativeLoadingProgressCadence::for_selected_map_file(
                    request.selected_map_file(),
                );
                Some(NativeLoadingScreenState::standard_skirmish(
                    variant,
                    skirmish_launch_session.local.country.side_index(),
                    color_index,
                    LoadingProgressRowSnapshot::from_launch_session(skirmish_launch_session),
                    progress_cadence,
                ))
            }
            (LoadingPresentation::GenericMapLoad, None) => None,
            (LoadingPresentation::NativeSelectedSkirmish, None)
            | (LoadingPresentation::GenericMapLoad, Some(_)) => {
                debug_assert!(
                    false,
                    "LoadingRequest constructor created mismatched launch/presentation modes"
                );
                None
            }
        };
        Self {
            request,
            native,
            job: LoadingJob::new(),
            first_frame_presented: false,
        }
    }
}

pub(crate) enum LoadingPump {
    Pending,
    Finished(MapLoadResult),
    Failed(anyhow::Error),
}

pub(crate) enum LoadingRenderResult {
    NativeRendered,
    GenericFallback,
    NativeFailed(anyhow::Error),
}

enum LoadingJobPhase {
    InitialMapSelection,
    RemainingLegacyLoad(Option<MapLoadInitial>),
}

struct LoadingJob {
    phase: LoadingJobPhase,
    ra2_dir: Option<PathBuf>,
    asset_manager: Option<AssetManager>,
}

impl LoadingJob {
    fn new() -> Self {
        Self {
            phase: LoadingJobPhase::InitialMapSelection,
            ra2_dir: None,
            asset_manager: None,
        }
    }
}

pub(crate) fn begin_loading(state: &mut AppState, request: LoadingRequest) {
    let next_active = request
        .startup()
        .accepted()
        .map(|startup| startup.correlation);
    replace_match_startup_slots(
        &mut state.frontend.active_loading_correlation,
        &mut state.frontend.loaded_startup,
        &mut state.frontend.rust_l0_receipt,
        next_active,
    );
    clear_loading_state(state);
    let mut session = LoadingSession::from_request(request);
    session.job.ra2_dir = state
        .platform.game_config
        .as_ref()
        .map(|config| config.paths.ra2_dir.clone());
    // Retail has one process-global MIX list and LoadFileFromMIX cache. Lease
    // that same manager through the loading job instead of reconstructing it
    // at the shell -> scenario boundary (F11 slot: Available -> Loading).
    session.job.asset_manager = state.process_assets.lease_for_loading();
    // Resolve the backing fill from the live rules `[Colors]` schemes now that
    // `state.rules` is reachable (the native ctor only sees the launch session).
    if let (Some(native), Some(rules)) = (session.native.as_mut(), state.rules()) {
        native.resolve_player_colors(&rules.color_schemes, &rules.house_color_ramps);
    }
    state.frontend.loading_session = Some(session);
    state.frontend.screen = GameScreen::Loading;
}

pub(crate) fn loading_map_name(state: &AppState) -> Option<&str> {
    state
        .frontend.loading_session
        .as_ref()
        .map(|session| session.request.selected_map_file())
}

pub(crate) fn clear_loading_state(state: &mut AppState) {
    // F11 slot: the rescue is unconditional — the leased manager (with its
    // sticky CRC cache and theater identity) always comes home. The old code
    // rescued only when the state slot was empty and otherwise dropped it.
    if let Some(mut session) = state.frontend.loading_session.take() {
        if let Some(manager) = session.job.asset_manager.take() {
            state.process_assets.return_from_loading(manager);
        }
    }
    state.frontend.loading_screen_atlas = None;
    state.frontend.loading_progress = LoadingProgressState::standard_skirmish();
}

/// Close any prior/in-flight match startup without resetting the process-wide
/// monotonically increasing correlation allocator.
pub(crate) fn clear_match_startup_state(state: &mut AppState) {
    replace_match_startup_slots(
        &mut state.frontend.active_loading_correlation,
        &mut state.frontend.loaded_startup,
        &mut state.frontend.rust_l0_receipt,
        None,
    );
}

fn replace_match_startup_slots(
    active: &mut Option<crate::match_bootstrap::MatchCorrelationId>,
    loaded: &mut Option<crate::match_bootstrap::PreparedMatchStartup>,
    receipt: &mut Option<crate::match_bootstrap::RustL0Receipt>,
    next_active: Option<crate::match_bootstrap::MatchCorrelationId>,
) {
    *active = next_active;
    *loaded = None;
    *receipt = None;
}

/// The handle the player launched this skirmish under, as shown on the loading
/// screen's progress row. `None` outside a skirmish launch.
pub(crate) fn launch_player_name(state: &AppState) -> Option<String> {
    state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.request.skirmish_launch_session())
        .map(|launch| launch.player_name.clone())
}

pub(crate) fn is_native_loading_session(state: &AppState) -> bool {
    state
        .frontend.loading_session
        .as_ref()
        .is_some_and(|session| session.native.is_some())
}

pub(crate) fn pump_loading_after_present(state: &mut AppState) -> LoadingPump {
    let Some(mut session) = state.frontend.loading_session.take() else {
        return LoadingPump::Pending;
    };
    if session.native_pump_blocked() {
        restore_job_asset_manager(state, &mut session);
        return LoadingPump::Failed(anyhow::anyhow!(
            "native Skirmish loading renderer was not ready before the first loading pump"
        ));
    }

    let phase = std::mem::replace(&mut session.job.phase, LoadingJobPhase::InitialMapSelection);
    let result = match phase {
        LoadingJobPhase::InitialMapSelection => {
            let initial = match ensure_session_job_asset_manager(state, &mut session) {
                Ok(()) => {
                    let ra2_dir = session
                        .job
                        .ra2_dir
                        .clone()
                        .expect("asset-manager setup stores the RA2 directory");
                    let asset_manager = session
                        .job
                        .asset_manager
                        .as_mut()
                        .expect("asset-manager setup stores the manager");
                    match session.native.as_mut() {
                        // The map-parse milestone (8) is emitted inside the loader.
                        Some(native) => {
                            let cadence = native.progress_cadence;
                            let mut sink = GatedProgressSink {
                                progress: &mut native.progress,
                                cadence,
                            };
                            session.request.load_initial_with_assets(
                                ra2_dir,
                                asset_manager,
                                &mut sink,
                            )
                        }
                        None => session.request.load_initial_with_assets(
                            ra2_dir,
                            asset_manager,
                            &mut NoopProgressSink,
                        ),
                    }
                }
                Err(err) => Err(err),
            };
            match initial {
                Ok(initial) => match session
                    .request
                    .prepare_fresh_scenario_load_context(&initial)
                {
                    Ok(()) => {
                        session.job.phase = LoadingJobPhase::RemainingLegacyLoad(Some(initial));
                        LoadingPump::Pending
                    }
                    Err(err) => LoadingPump::Failed(err),
                },
                Err(err) => LoadingPump::Failed(err),
            }
        }
        LoadingJobPhase::RemainingLegacyLoad(mut initial) => {
            let Some(initial) = initial.take() else {
                restore_job_asset_manager(state, &mut session);
                return LoadingPump::Failed(anyhow::anyhow!(
                    "loading job had no initial map state"
                ));
            };
            let native_theater_cache_mismatch = theater_cache_mismatch(
                state.match_state.loaded_map_source.is_some(),
                &state.match_state.match_presentation.theater_name,
                initial.theater_name(),
            );
            // All mid-load milestones (12..98) are emitted inside the loader; the
            // pump emits the terminal 100 once the result is ready. For the native
            // case we drive a RenderingProgressSink that synchronously repaints the
            // loading screen on each advancing milestone (gamemd's per-milestone
            // hidden-to-primary blit), so the bar visibly sweeps instead of
            // snapping once.
            //
            // Pre-copy the by-value pieces before borrowing so the disjoint
            // split-borrows (gpu/depth_view/batch shared, vxl_compute &mut,
            // native.progress &mut, native.atlas shared, request shared) all
            // hold simultaneously.
            let render_size = [state.renderer.gpu.config.width, state.renderer.gpu.config.height];
            // The pre-parse swallowed the loader's raw 8 so it could not present
            // before the first frame; hand it over now for either native cadence.
            if let Some(native) = session.native.as_mut()
                && native
                    .progress_cadence
                    .prepares_scenario_before_first_frame()
            {
                advance_and_present_native_progress(
                    &state.renderer.gpu,
                    &state.renderer.shell_surface_presenter,
                    &state.renderer.depth_view,
                    &state.renderer.batch_renderer,
                    &state.renderer.bit_font,
                    native,
                    8,
                    render_size,
                );
            }
            // `session.native` and `session.request` are disjoint fields, so the
            // launch-session/settings borrows below coexist with the native split.
            let fresh_scenario_context = match session
                .request
                .take_fresh_scenario_load_context()
            {
                Ok(context) => context,
                Err(err) => {
                    restore_job_asset_manager(state, &mut session);
                    return LoadingPump::Failed(err);
                }
            };
            let startup = session.request.take_startup();
            if !state.process_assets.has_native_rules() {
                restore_job_asset_manager(state, &mut session);
                return LoadingPump::Failed(anyhow::anyhow!(
                    "loading requires the process-resident native Rules owner"
                ));
            }
            let Some(asset_manager) = session.job.asset_manager.as_mut() else {
                restore_job_asset_manager(state, &mut session);
                return LoadingPump::Failed(anyhow::anyhow!(
                    "loading job lost its process asset manager"
                ));
            };
            let shared_cell_dummy = state.process_assets.shared_cell_dummy.clone();
            let (native_rules_owner, tile_variant_selector_cache) =
                state.process_assets.native_rules_mut_with_tile_cache();
            let native_rules_owner = native_rules_owner
                .expect("native Rules availability checked before split borrow");
            let load_result = match session.native.as_mut() {
                // Only repaint when the atlas is present; without it the bar
                // cannot draw, so fall back to the gate-only sink.
                Some(native) if native.atlas.is_some() => {
                    let backing_rgb = native.backing_rgb;
                    let text_rgb = native.text_rgb;
                    let cadence = native.progress_cadence;
                    let runtime_color_scheme_count = native.runtime_color_scheme_count;
                    let atlas = native.atlas.as_ref().expect("atlas present checked above");
                    let composition = native.composition.as_ref();
                    let mut sink = RenderingProgressSink {
                        gpu: &state.renderer.gpu,
                        presenter: &state.renderer.shell_surface_presenter,
                        depth_view: &state.renderer.depth_view,
                        batch: &state.renderer.batch_renderer,
                        font: &state.renderer.bit_font,
                        progress: &mut native.progress,
                        progress_row: &native.progress_row,
                        atlas,
                        composition,
                        backing_rgb,
                        text_rgb,
                        render_size,
                        cadence,
                    };
                    init::load_map_from_initial(
                        &state.renderer.gpu,
                        &state.renderer.batch_renderer,
                        asset_manager,
                        initial,
                        startup,
                        fresh_scenario_context,
                        &session.request.fallback_skirmish_settings,
                        native_theater_cache_mismatch,
                        runtime_color_scheme_count,
                        state.renderer.vxl_compute.as_mut(),
                        native_rules_owner,
                        shared_cell_dummy.clone(),
                        tile_variant_selector_cache,
                        &mut sink,
                    )
                }
                Some(native) => {
                    let cadence = native.progress_cadence;
                    let runtime_color_scheme_count = native.runtime_color_scheme_count;
                    let mut sink = GatedProgressSink {
                        progress: &mut native.progress,
                        cadence,
                    };
                    init::load_map_from_initial(
                        &state.renderer.gpu,
                        &state.renderer.batch_renderer,
                        asset_manager,
                        initial,
                        startup,
                        fresh_scenario_context,
                        &session.request.fallback_skirmish_settings,
                        native_theater_cache_mismatch,
                        runtime_color_scheme_count,
                        state.renderer.vxl_compute.as_mut(),
                        native_rules_owner,
                        shared_cell_dummy.clone(),
                        tile_variant_selector_cache,
                        &mut sink,
                    )
                }
                None => init::load_map_from_initial(
                    &state.renderer.gpu,
                    &state.renderer.batch_renderer,
                    asset_manager,
                    initial,
                    startup,
                    fresh_scenario_context,
                    &session.request.fallback_skirmish_settings,
                    false,
                    0,
                    state.renderer.vxl_compute.as_mut(),
                    native_rules_owner,
                    shared_cell_dummy,
                    tile_variant_selector_cache,
                    &mut NoopProgressSink,
                ),
            };
            match load_result {
                Ok(mut result) => {
                    if let Some(native) = session.native.as_mut() {
                        let terminal_raw_percent = native.progress_cadence.terminal_raw_percent();
                        advance_and_present_native_progress(
                            &state.renderer.gpu,
                            &state.renderer.shell_surface_presenter,
                            &state.renderer.depth_view,
                            &state.renderer.batch_renderer,
                            &state.renderer.bit_font,
                            native,
                            terminal_raw_percent,
                            render_size,
                        );
                    }
                    result.asset_manager = session.job.asset_manager.take();
                    LoadingPump::Finished(result)
                }
                Err(err) => LoadingPump::Failed(err),
            }
        }
    };

    if matches!(result, LoadingPump::Pending) {
        state.frontend.loading_session = Some(session);
    } else if matches!(result, LoadingPump::Failed(_)) {
        restore_job_asset_manager(state, &mut session);
    }
    result
}

fn ensure_job_asset_manager(state: &mut AppState) -> anyhow::Result<()> {
    let Some(mut session) = state.frontend.loading_session.take() else {
        return Ok(());
    };
    let result = ensure_session_job_asset_manager(state, &mut session);
    state.frontend.loading_session = Some(session);
    result
}

fn loading_asset_manager(session: &LoadingSession) -> Option<&AssetManager> {
    session.job.asset_manager.as_ref()
}

fn ensure_session_job_asset_manager(
    state: &mut AppState,
    session: &mut LoadingSession,
) -> anyhow::Result<()> {
    if session.job.ra2_dir.is_none() {
        session.job.ra2_dir = Some(
            state
                .platform.game_config
                .as_ref()
                .map(|config| config.paths.ra2_dir.clone())
                .ok_or_else(|| anyhow::anyhow!("missing game config for loading job assets"))?,
        );
    }
    if session.job.asset_manager.is_none() {
        let asset_manager = if let Some(asset_manager) = state.process_assets.lease_for_loading() {
            asset_manager
        } else {
            // Warn only when a manager actually existed and its lease was
            // lost — reconstructing then loses the sticky CRC cache and
            // theater identity. An asset-less startup (no retail archives)
            // has nothing to lose and stays quiet.
            if state.process_assets.is_leased() {
                log::warn!(
                    "loading job reconstructs an AssetManager; process-sticky \
                     MIX cache and theater identity restart"
                );
            }
            state.process_assets.note_lease_ended_without_return();
            AssetManager::new(
                session
                    .job
                    .ra2_dir
                    .as_deref()
                    .expect("RA2 directory was initialized above"),
            )?
        };
        session.job.asset_manager = Some(asset_manager);
    }
    Ok(())
}

fn restore_job_asset_manager(state: &mut AppState, session: &mut LoadingSession) {
    // F11 slot: unconditional return (Loading -> Available); a double return
    // keeps the resident manager and logs inside the slot.
    if let Some(manager) = session.job.asset_manager.take() {
        state.process_assets.return_from_loading(manager);
    }
}

/// Resolve native Scenario inputs before constructing the first loading frame.
///
/// Full Init runs both selected-mode callbacks before DrawLoadingScreen. Fixed
/// maps additionally derive their preview from the parsed scenario; random maps
/// use `RandMap.img` pixels but still require regenerated header data and the
/// accepted staged starts. The loader's raw 8 is intentionally swallowed here
/// and visibly handed off only after the first frame has been presented.
fn prepare_scenario_initial_before_first_frame(state: &mut AppState) -> anyhow::Result<()> {
    let should_prepare = state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .is_some_and(|native| {
            native
                .progress_cadence
                .prepares_scenario_before_first_frame()
        })
        && state.frontend.loading_session.as_ref().is_some_and(|session| {
            matches!(session.job.phase, LoadingJobPhase::InitialMapSelection)
        });
    if !should_prepare {
        return Ok(());
    }

    let Some(mut session) = state.frontend.loading_session.take() else {
        return Ok(());
    };
    let result = ensure_session_job_asset_manager(state, &mut session).and_then(|()| {
        let ra2_dir = session
            .job
            .ra2_dir
            .clone()
            .expect("asset-manager setup stores the RA2 directory");
        let asset_manager = session
            .job
            .asset_manager
            .as_mut()
            .expect("asset-manager setup stores the manager");
        session
            .request
            .load_initial_with_assets(ra2_dir, asset_manager, &mut NoopProgressSink)
    });
    match result {
        Ok(initial) => {
            if let Err(err) = session
                .request
                .prepare_fresh_scenario_load_context(&initial)
            {
                state.frontend.loading_session = Some(session);
                return Err(err);
            }
            session.job.phase = LoadingJobPhase::RemainingLegacyLoad(Some(initial));
            state.frontend.loading_session = Some(session);
            Ok(())
        }
        Err(err) => {
            state.frontend.loading_session = Some(session);
            Err(err)
        }
    }
}

/// Decode the random-map preview bitmap written by the random-map setup dialog.
///
/// This is the whole preview source for a random-map load: gamemd reads the same
/// file rather than deriving a preview from the scenario. A missing or unreadable
/// file omits the preview layer; every other layer still draws.
fn decode_random_map_loading_preview(ra2_dir: &Path) -> Option<DecodedPreview> {
    let path = ra2_dir.join(RANDOM_MAP_PREVIEW_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            log::warn!(
                "Loading screen: no random-map preview at {} ({err})",
                path.display()
            );
            return None;
        }
    };
    let pcx = match PcxFile::from_bytes(&bytes) {
        Ok(pcx) => pcx,
        Err(err) => {
            log::warn!(
                "Loading screen: cannot decode random-map preview {} ({err})",
                path.display()
            );
            return None;
        }
    };
    Some(DecodedPreview {
        width: u32::from(pcx.width),
        height: u32::from(pcx.height),
        rgba: pcx.to_rgba(None),
    })
}

/// Resolve the assigned start waypoints the marker layer draws for a selected map.
pub(crate) fn selected_map_start_assignments(
    launch_session: &SkirmishLaunchSession,
    projection: Option<&StockOfflinePrefixProjection>,
) -> Vec<LoadingStartAssignment> {
    let Some(projection) = projection else {
        return Vec::new();
    };
    projection
        .start_table()
        .iter()
        .enumerate()
        .filter_map(|(start_index, participant_index)| {
            let participant_index = (*participant_index)?;
            projection.final_gathered_starts().get(start_index)?;
            let (participant, color_priority) = if participant_index == 0 {
                (
                    LoadingParticipantId::Local,
                    launch_session.local.color_index,
                )
            } else {
                let opponent_index = participant_index - 1;
                let opponent = launch_session.opponents.get(opponent_index)?;
                (
                    LoadingParticipantId::Opponent(opponent_index),
                    opponent.color_index,
                )
            };
            Some(LoadingStartAssignment {
                start_index: u32::try_from(start_index).ok()?,
                participant,
                color_priority,
            })
        })
        .collect()
}

/// Build the loading composition for either map kind.
///
/// gamemd branches only the preview holder on the random-map flag; the four text
/// layers sit after that branch and are drawn for both kinds. Splitting the
/// snapshot on the branch (as this used to) dropped the country name, the
/// special-unit line, the briefing and "Loading..." from every random-map load.
fn ensure_loading_composition_snapshot(state: &mut AppState) {
    let snapshot = {
        let Some(session) = state.frontend.loading_session.as_ref() else {
            return;
        };
        let Some(native) = session.native.as_ref() else {
            return;
        };
        if native.composition.is_some() {
            return;
        }
        let Some(context) = session.request.fresh_scenario_load_context() else {
            return;
        };
        let launch_session = context.stock_offline_launch().session();
        let projection = context.stock_offline_projection();
        let render_size = [state.renderer.gpu.config.width, state.renderer.gpu.config.height];
        match native.progress_cadence {
            NativeLoadingProgressCadence::SelectedMap => {
                let LoadingJobPhase::RemainingLegacyLoad(Some(initial)) = &session.job.phase else {
                    return;
                };
                let assignments =
                    selected_map_start_assignments(launch_session, Some(projection));
                build_loading_composition(
                    initial.map_data(),
                    launch_session,
                    state.process_assets.csf.as_ref(),
                    render_size,
                    &assignments,
                )
            }
            NativeLoadingProgressCadence::RandomMapHalved => {
                let LoadingJobPhase::RemainingLegacyLoad(Some(initial)) = &session.job.phase else {
                    return;
                };
                let preview = session
                    .job
                    .ra2_dir
                    .as_deref()
                    .and_then(decode_random_map_loading_preview);
                let assignments =
                    selected_map_start_assignments(launch_session, Some(projection));
                build_random_map_loading_composition(
                    launch_session,
                    state.process_assets.csf.as_ref(),
                    render_size,
                    preview,
                    initial.map_data(),
                    projection.active_scenario_waypoints(),
                    &assignments,
                )
            }
        }
    };

    if let Some(native) = state
        .frontend.loading_session
        .as_mut()
        .and_then(|session| session.native.as_mut())
    {
        native.composition = Some(snapshot);
    }
}

fn theater_cache_mismatch(
    has_successfully_loaded_map: bool,
    cached_theater: &str,
    requested_theater: &str,
) -> bool {
    !has_successfully_loaded_map || !cached_theater.eq_ignore_ascii_case(requested_theater)
}

/// Derive the verified dynamic theater values from the native runtime scheme count.
///
/// Native relies on `N == 0 || N >= 13`; counts 1..12 would divide by zero.
/// Rust treats those malformed, non-stock counts like an empty dynamic loop.
pub(crate) fn theater_ramp_changed_values(runtime_color_scheme_count: usize) -> Vec<u32> {
    if runtime_color_scheme_count < 13 {
        return Vec::new();
    }

    let quotient = runtime_color_scheme_count / 13;
    let mut previous = 12;
    let mut emitted = Vec::new();
    for scheme_index in 0..runtime_color_scheme_count {
        let candidate = ((scheme_index / quotient).min(13) + 12) as u32;
        if candidate != previous {
            emitted.push(candidate);
            previous = candidate;
        }
    }
    emitted
}

pub(crate) fn ensure_native_loading_atlas(state: &mut AppState) -> anyhow::Result<()> {
    let Some(variant) = selected_loading_art_variant(state) else {
        return Ok(());
    };
    if state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .and_then(|native| native.atlas.as_ref())
        .is_some()
    {
        return Ok(());
    }
    if state
        .frontend.loading_session
        .as_ref()
        .and_then(loading_asset_manager)
        .is_none()
    {
        ensure_job_asset_manager(state)?;
    }
    let loading_archives_ready = state
        .frontend.loading_session
        .as_mut()
        .and_then(|session| session.job.asset_manager.as_mut())
        .ok_or_else(|| {
            anyhow::anyhow!("native loading job has no asset manager after initialization")
        })?
        .register_loading_archives()?;
    if !loading_archives_ready {
        return Err(anyhow::anyhow!(
            "native loading archives LOADMD.MIX and LOAD.MIX are unavailable"
        ));
    }
    prepare_scenario_initial_before_first_frame(state)?;
    ensure_loading_composition_snapshot(state);
    let Some(assets) = state
        .frontend.loading_session
        .as_ref()
        .and_then(loading_asset_manager)
    else {
        return Err(anyhow::anyhow!(
            "native loading job has no asset manager after initialization"
        ));
    };
    let width = LoadingScreenWidth::for_render_width(state.renderer.gpu.config.width);
    let progress_ramp = state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .map(|native| native.progress_ramp)
        .ok_or_else(|| anyhow::anyhow!("native loading session lost its progress ramp"))?;
    let prepared_preview = state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .and_then(|native| native.composition.as_ref())
        .and_then(|composition| composition.preview.as_ref())
        .map(|preview| PreparedLoadingPreviewRgba {
            width: preview.image.width,
            height: preview.image.height,
            rgba: preview.image.rgba.clone(),
        });
    let marker_remaps = state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .and_then(|native| native.composition.as_ref())
        .zip(state.rules())
        .map(|(composition, rules)| {
            composition
                .markers
                .iter()
                .map(|marker| {
                    let color_key =
                        scheme_entry_for_priority(i32::from(marker.color_priority)) as u8;
                    MmpbMarkerRemap {
                        color_key,
                        ramp: *rules.house_color_ramps.ramp(HouseColorIndex(color_key)),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let composition_input =
        prepared_preview
            .as_ref()
            .map(|preview| LoadingScreenCompositionAtlasInput {
                preview,
                marker_remaps: &marker_remaps,
            });
    let atlas = build_loading_screen_atlas_with_composition(
        &state.renderer.gpu,
        &state.renderer.batch_renderer,
        &assets,
        variant,
        width,
        &progress_ramp,
        composition_input,
    );
    if let Some(native) = state
        .frontend.loading_session
        .as_mut()
        .and_then(|session| session.native.as_mut())
    {
        native.first_renderer_ready = atlas.is_some();
        native.atlas = atlas;
    }
    if state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .is_some_and(|native| native.first_renderer_ready)
    {
        log::info!("Native standard Skirmish loading atlas ready: {variant:?} {width:?}");
        Ok(())
    } else {
        log::warn!("Native standard Skirmish loading atlas failed: {variant:?} {width:?}");
        Err(anyhow::anyhow!(
            "native standard Skirmish loading atlas failed: {variant:?} {width:?}"
        ))
    }
}

pub(crate) fn render_loading_screen(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    destination: &wgpu::Texture,
) -> LoadingRenderResult {
    if !is_native_loading_session(state) {
        return LoadingRenderResult::GenericFallback;
    }
    if let Err(err) = ensure_native_loading_atlas(state) {
        return LoadingRenderResult::NativeFailed(err);
    }
    if let Some(session) = state.frontend.loading_session.as_mut()
        && !session.first_frame_presented
        && let Some(native) = session.native.as_mut()
    {
        // Retail composes the LS country surface first, then ProgressClass
        // supplies the first displayed value: selected maps show raw 3, while
        // random-map seed loads halve it to 1.
        native
            .progress
            .advance_progress(native.progress_cadence.effective_percent(3));
    }
    let Some(native) = state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
    else {
        return LoadingRenderResult::GenericFallback;
    };
    let Some(atlas) = native.atlas.as_ref() else {
        return LoadingRenderResult::NativeFailed(anyhow::anyhow!(
            "native Skirmish loading atlas was not available for render"
        ));
    };
    let target = state.renderer.shell_surface_presenter.source_render_view();

    let frame_plan = build_native_loading_frame_plan(
        &state.renderer.bit_font,
        atlas,
        native.composition.as_ref(),
        &native.progress_row,
        &native.progress,
        native.backing_rgb,
        native.text_rgb,
        [state.renderer.gpu.config.width, state.renderer.gpu.config.height],
    );
    let instances = frame_plan.instances;
    let text_draws = frame_plan.text_draws;

    state.renderer.batch_renderer.update_camera(
        &state.renderer.gpu,
        state.renderer.gpu.config.width as f32,
        state.renderer.gpu.config.height as f32,
        0.0,
        0.0,
        1.0,
    );
    let Some((buffer, count)) = state
        .renderer.batch_renderer
        .create_instance_buffer(&state.renderer.gpu, &instances)
    else {
        return LoadingRenderResult::NativeFailed(anyhow::anyhow!(
            "native Skirmish loading instances could not be uploaded"
        ));
    };
    let backing_buffers = text_draws
        .iter()
        .map(|draw| {
            state
                .renderer.batch_renderer
                .create_instance_buffer(&state.renderer.gpu, &draw.backing)
        })
        .collect::<Vec<_>>();
    let text_buffers = text_draws
        .iter()
        .map(|draw| {
            state
                .renderer.batch_renderer
                .create_instance_buffer(&state.renderer.gpu, &draw.text.instances)
        })
        .collect::<Vec<_>>();

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Native Loading Screen"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app::types::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &state.renderer.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    state
        .renderer.batch_renderer
        .draw_with_buffer_passthrough(&mut pass, &atlas.texture, &buffer, count);
    for ((draw, backing_buffer), text_buffer) in text_draws
        .iter()
        .zip(backing_buffers.iter())
        .zip(text_buffers.iter())
    {
        if let Some((buffer, count)) = backing_buffer.as_ref() {
            state.renderer.batch_renderer.draw_with_buffer_passthrough(
                &mut pass,
                &atlas.texture,
                buffer,
                *count,
            );
        }
        let Some((buffer, count)) = text_buffer.as_ref() else {
            continue;
        };
        let Some(scissor) = clamp_loading_scissor(
            draw.text.scissor,
            state.renderer.gpu.config.width,
            state.renderer.gpu.config.height,
        ) else {
            continue;
        };
        pass.set_scissor_rect(scissor.x, scissor.y, scissor.w, scissor.h);
        state.renderer.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            state.renderer.bit_font.atlas(),
            buffer,
            *count,
        );
    }
    pass.set_scissor_rect(0, 0, state.renderer.gpu.config.width, state.renderer.gpu.config.height);
    drop(pass);
    state
        .renderer.shell_surface_presenter
        .encode_present(encoder, destination);
    LoadingRenderResult::NativeRendered
}

pub(crate) fn loading_screen_presented(state: &mut AppState) {
    let Some(session) = state.frontend.loading_session.as_mut() else {
        state.frontend.loading_progress.advance_progress(3);
        return;
    };
    session.first_frame_presented = true;
}

fn selected_loading_art_variant(state: &AppState) -> Option<LoadingArtVariant> {
    if !matches!(state.frontend.screen, GameScreen::Loading) {
        return None;
    }
    state
        .frontend.loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .map(|native| native.variant)
}

fn loading_art_variant_from_launch_country(country: LaunchCountry) -> LoadingArtVariant {
    match country {
        LaunchCountry::America => LoadingArtVariant::Americans,
        LaunchCountry::Korea => LoadingArtVariant::Alliance,
        LaunchCountry::France => LoadingArtVariant::French,
        LaunchCountry::Germany => LoadingArtVariant::Germans,
        LaunchCountry::GreatBritain => LoadingArtVariant::British,
        LaunchCountry::Libya => LoadingArtVariant::Africans,
        LaunchCountry::Iraq => LoadingArtVariant::Arabs,
        LaunchCountry::Cuba => LoadingArtVariant::Confederation,
        LaunchCountry::Russia => LoadingArtVariant::Russians,
        LaunchCountry::Yuri => LoadingArtVariant::Yuri,
    }
}

/// Normalize an 8-bit RGB triple to 0..1.
fn normalize_rgb(rgb: [u8; 3]) -> [f32; 3] {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
    ]
}

struct NativeLoadingTextDraw {
    backing: Vec<SpriteInstance>,
    text: ShellTextDraw,
}

struct NativeLoadingFramePlan {
    instances: Vec<SpriteInstance>,
    text_draws: Vec<NativeLoadingTextDraw>,
}

fn native_loading_row_layout(
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    progress: &LoadingProgressState,
    render_size: [u32; 2],
) -> Option<LoadingProgressRowLayout> {
    if progress.current_value() == 0.0 {
        return None;
    }
    Some(layout_standard_skirmish_progress_row(
        render_size,
        [
            atlas.progress_frame0.pixel_size[0] as i32,
            atlas.progress_frame0.pixel_size[1] as i32,
        ],
        atlas
            .side_icon
            .map(|icon| [icon.pixel_size[0] as i32, icon.pixel_size[1] as i32]),
        font.cell_height() as i32,
    ))
}

fn build_native_loading_text_draws(
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    composition: Option<&LoadingCompositionSnapshot>,
    row: &LoadingProgressRowSnapshot,
    row_layout: Option<&LoadingProgressRowLayout>,
    text_rgb: [f32; 3],
    row_rgb: [f32; 3],
) -> Vec<NativeLoadingTextDraw> {
    let mut draws = Vec::with_capacity(5);
    if let Some(composition) = composition {
        if let Some(text) = composition.text.country_name.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.country_name,
                text_rgb,
                ShellAlign::H_RIGHT,
                true,
                TEXT_DEPTH,
            ));
        }
        if let Some(text) = composition.text.special_unit.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.special_unit,
                [0.0, 0.0, 0.0],
                ShellAlign::NONE,
                false,
                TEXT_DEPTH,
            ));
        }
        if let Some(text) = composition.text.load_brief.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.load_brief,
                text_rgb,
                ShellAlign::NONE,
                true,
                TEXT_DEPTH,
            ));
        }
        if let Some(text) = composition.text.loading.as_deref() {
            draws.push(build_native_loading_text_draw(
                font,
                atlas,
                text,
                composition.text_rects.loading,
                text_rgb,
                ShellAlign::NONE,
                true,
                TEXT_DEPTH,
            ));
        }
    }
    if let Some(layout) = row_layout
        && !row.label.is_empty()
        && layout.label_rect.w > 0
        && layout.label_rect.h > 0
    {
        draws.push(build_native_loading_text_draw(
            font,
            atlas,
            &row.label,
            layout.label_rect,
            row_rgb,
            ShellAlign::NONE,
            false,
            ROW_LABEL_DEPTH,
        ));
    }
    draws
}

#[allow(clippy::too_many_arguments)]
fn build_native_loading_text_draw(
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    text: &str,
    rect: crate::ui::shell::geom::RectPx,
    color: [f32; 3],
    align: ShellAlign,
    with_backing: bool,
    depth: f32,
) -> NativeLoadingTextDraw {
    let width = rect.w.max(0) as u32;
    let height = rect.h.max(0) as u32;
    let text_rect = TextRect {
        x: rect.x,
        y: rect.y,
        w: width,
        h: height,
    };
    let text_draw = draw_in_rect(font, text, text_rect, color, align, [0.0, 0.0], depth, None);
    let mut backing = Vec::new();
    if with_backing && !text_draw.instances.is_empty() {
        let layout = font.wrap_layout(text, width);
        let aligned_x = if align.contains(ShellAlign::H_RIGHT) && layout.width < width {
            rect.x + (width - layout.width) as i32
        } else if align.contains(ShellAlign::H_CENTER) && layout.width < width {
            rect.x + ((width - layout.width) / 2) as i32
        } else {
            rect.x
        };
        push_entry_tinted(
            &mut backing,
            atlas.solid_texel,
            [
                aligned_x as f32 - TEXT_BACKING_PADDING,
                rect.y as f32 - TEXT_BACKING_PADDING,
            ],
            [
                layout.width as f32 + TEXT_BACKING_PADDING * 2.0,
                layout.height.min(height) as f32 + TEXT_BACKING_PADDING * 2.0,
            ],
            TEXT_BACKING_DEPTH,
            [0.0, 0.0, 0.0],
        );
        if let Some(instance) = backing.last_mut() {
            instance.alpha = TEXT_BACKING_ALPHA;
        }
    }
    NativeLoadingTextDraw {
        backing,
        text: text_draw,
    }
}

fn clamp_loading_scissor(
    scissor: ScissorRect,
    render_width: u32,
    render_height: u32,
) -> Option<ScissorRect> {
    let x = scissor.x.min(render_width);
    let y = scissor.y.min(render_height);
    let w = scissor.w.min(render_width.saturating_sub(x));
    let h = scissor.h.min(render_height.saturating_sub(y));
    (w > 0 && h > 0).then_some(ScissorRect { x, y, w, h })
}

/// Build the full native loading-screen instance list (background, solid backing
/// fill, clipped progress bar, side icon) shared by the per-frame render path and
/// the synchronous-repaint sink.
fn build_native_loading_instances(
    atlas: &LoadingScreenAtlas,
    composition: Option<&LoadingCompositionSnapshot>,
    progress: &LoadingProgressState,
    backing_rgb: [f32; 3],
    row_layout: Option<&LoadingProgressRowLayout>,
    base_origin: [i32; 2],
) -> Vec<SpriteInstance> {
    let mut instances = Vec::with_capacity(12);
    // The art hangs off the same base origin as the progress row and the text
    // layers, so an oversized window centers all three together.
    push_entry(
        &mut instances,
        atlas.background,
        [base_origin[0] as f32, base_origin[1] as f32],
        BACKGROUND_DEPTH,
    );

    if let Some(composition) = composition {
        if let (Some(prepared), Some(preview_entry)) = (composition.preview.as_ref(), atlas.preview)
        {
            // gamemd blits the source preview into the fitted destination rect,
            // resampling the whole image. The aspect fit has already chosen a
            // destination that preserves the source ratio, so both axes scale;
            // clipping either one here would cut the map's edge off.
            push_entry_scaled(
                &mut instances,
                preview_entry,
                [
                    (prepared.region.x + prepared.fit.pad_x) as f32,
                    (prepared.region.y + prepared.fit.pad_y) as f32,
                ],
                [prepared.fit.width as f32, prepared.fit.height as f32],
                PREVIEW_DEPTH,
                [1.0; 3],
            );
        }
        // Markers only exist alongside a preview, and they are cropped to that
        // preview's region for the same reason gamemd composes them into a
        // region-sized surface before blitting it.
        if let Some(prepared) = composition.preview.as_ref() {
            for marker in &composition.markers {
                let color_key = scheme_entry_for_priority(i32::from(marker.color_priority)) as u8;
                let Some(entry) = atlas.mmpb_markers.get(&color_key).copied() else {
                    continue;
                };
                push_entry_clipped(
                    &mut instances,
                    entry,
                    [marker.anchor.screen_x, marker.anchor.screen_y],
                    MARKER_DEPTH,
                    prepared.region,
                );
            }
        }
    }

    // The LS renderer's compose-only state owns no progress row. The selected-map
    // path advances to 3 before the first confirmed display blit.
    if progress.current_value() == 0.0 {
        return instances;
    }

    let Some(row_layout) = row_layout else {
        return instances;
    };
    let bar_w = atlas.progress_frame0.pixel_size[0];
    let bar_h = atlas.progress_frame0.pixel_size[1];
    let bar_origin = [
        row_layout.bar_origin[0] as f32,
        row_layout.bar_origin[1] as f32,
    ];

    // G3: solid backing fill — full bar frame rect (W x H), filled with the
    // player scheme's `[Colors]` HSV→RGB color, drawn BEFORE the clipped bar so
    // the bar covers it.
    push_entry_tinted(
        &mut instances,
        atlas.solid_texel,
        bar_origin,
        [bar_w, bar_h],
        SOLID_FILL_DEPTH,
        backing_rgb,
    );

    // G2: clipped progress span. The session atlas already contains the
    // player's 16-shade remap, so preserve its per-pixel colors.
    let progress_width = progress.fill_width_gamemd_ftol_positive_domain(bar_w as u32);
    if progress_width > 0 {
        push_progress_fill(
            &mut instances,
            atlas.progress_frame0,
            bar_origin,
            progress_width as f32,
            PROGRESS_DEPTH,
        );
    }

    // Country insignia follows the progress span. The atlas has already applied
    // the verified RGB-magenta key.
    if let (Some(icon), Some(icon_origin)) = (atlas.side_icon, row_layout.icon_origin) {
        push_entry(
            &mut instances,
            icon,
            [icon_origin[0] as f32, icon_origin[1] as f32],
            SIDE_ICON_DEPTH,
        );
    }

    instances
}

#[allow(clippy::too_many_arguments)]
fn build_native_loading_frame_plan(
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    composition: Option<&LoadingCompositionSnapshot>,
    progress_row: &LoadingProgressRowSnapshot,
    progress: &LoadingProgressState,
    backing_rgb: [f32; 3],
    text_rgb: [f32; 3],
    render_size: [u32; 2],
) -> NativeLoadingFramePlan {
    let row_layout = native_loading_row_layout(font, atlas, progress, render_size);
    let instances = build_native_loading_instances(
        atlas,
        composition,
        progress,
        backing_rgb,
        row_layout.as_ref(),
        loading_base_origin(render_size),
    );
    let text_draws = build_native_loading_text_draws(
        font,
        atlas,
        composition,
        progress_row,
        row_layout.as_ref(),
        text_rgb,
        backing_rgb,
    );
    NativeLoadingFramePlan {
        instances,
        text_draws,
    }
}

/// Acquire a surface frame, render the native loading screen, and present it.
///
/// Used by the synchronous-repaint sink to mirror gamemd's per-milestone
/// `WM_PAINT`. All wgpu ops take `&self`, so only shared references are needed.
/// Returns an error on acquire/upload failure; the caller treats it as non-fatal.
fn present_native_loading(
    gpu: &GpuContext,
    presenter: &ShellSurfacePresenter,
    depth_view: &wgpu::TextureView,
    batch: &BatchRenderer,
    font: &BitFont,
    atlas: &LoadingScreenAtlas,
    composition: Option<&LoadingCompositionSnapshot>,
    progress_row: &LoadingProgressRowSnapshot,
    progress: &LoadingProgressState,
    backing_rgb: [f32; 3],
    text_rgb: [f32; 3],
    render_size: [u32; 2],
) -> anyhow::Result<()> {
    let output = gpu
        .surface
        .get_current_texture()
        .map_err(|e| anyhow::anyhow!("loading repaint surface texture: {e}"))?;
    let view = presenter.source_render_view();
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Native Loading Repaint"),
        });

    let frame_plan = build_native_loading_frame_plan(
        font,
        atlas,
        composition,
        progress_row,
        progress,
        backing_rgb,
        text_rgb,
        render_size,
    );
    let instances = frame_plan.instances;
    let text_draws = frame_plan.text_draws;
    batch.update_camera(
        gpu,
        gpu.config.width as f32,
        gpu.config.height as f32,
        0.0,
        0.0,
        1.0,
    );
    let Some((buffer, count)) = batch.create_instance_buffer(gpu, &instances) else {
        return Err(anyhow::anyhow!(
            "loading repaint instances could not be uploaded"
        ));
    };
    let backing_buffers = text_draws
        .iter()
        .map(|draw| batch.create_instance_buffer(gpu, &draw.backing))
        .collect::<Vec<_>>();
    let text_buffers = text_draws
        .iter()
        .map(|draw| batch.create_instance_buffer(gpu, &draw.text.instances))
        .collect::<Vec<_>>();

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Native Loading Repaint"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(crate::app::types::CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        batch.draw_with_buffer_passthrough(&mut pass, &atlas.texture, &buffer, count);
        for ((draw, backing_buffer), text_buffer) in text_draws
            .iter()
            .zip(backing_buffers.iter())
            .zip(text_buffers.iter())
        {
            if let Some((buffer, count)) = backing_buffer.as_ref() {
                batch.draw_with_buffer_passthrough(&mut pass, &atlas.texture, buffer, *count);
            }
            let Some((buffer, count)) = text_buffer.as_ref() else {
                continue;
            };
            let Some(scissor) =
                clamp_loading_scissor(draw.text.scissor, gpu.config.width, gpu.config.height)
            else {
                continue;
            };
            pass.set_scissor_rect(scissor.x, scissor.y, scissor.w, scissor.h);
            batch.draw_with_buffer_passthrough(&mut pass, font.atlas(), buffer, *count);
        }
        pass.set_scissor_rect(0, 0, gpu.config.width, gpu.config.height);
    }

    presenter.encode_present(&mut encoder, &output.texture);
    gpu.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    Ok(())
}

fn advance_and_present_native_progress(
    gpu: &GpuContext,
    presenter: &ShellSurfacePresenter,
    depth_view: &wgpu::TextureView,
    batch: &BatchRenderer,
    font: &BitFont,
    native: &mut NativeLoadingScreenState,
    raw_percent: u32,
    render_size: [u32; 2],
) {
    let effective_percent = native.progress_cadence.effective_percent(raw_percent);
    if !native.progress.advance_progress(effective_percent) {
        return;
    }
    let Some(atlas) = native.atlas.as_ref() else {
        return;
    };
    if let Err(err) = present_native_loading(
        gpu,
        presenter,
        depth_view,
        batch,
        font,
        atlas,
        native.composition.as_ref(),
        &native.progress_row,
        &native.progress,
        native.backing_rgb,
        native.text_rgb,
        render_size,
    ) {
        log::warn!(
            "Native loading repaint at raw milestone {raw_percent} \
             (effective {effective_percent}) failed: {err:#}"
        );
    }
}

/// Sink that synchronously re-renders and presents the loading screen on each
/// advancing milestone, implementing gamemd's per-milestone visible handoff
/// through wgpu. Render or surface-acquire failures are logged and swallowed so
/// they never abort the map load.
struct RenderingProgressSink<'a> {
    gpu: &'a GpuContext,
    presenter: &'a ShellSurfacePresenter,
    depth_view: &'a wgpu::TextureView,
    batch: &'a BatchRenderer,
    font: &'a BitFont,
    progress: &'a mut LoadingProgressState,
    progress_row: &'a LoadingProgressRowSnapshot,
    atlas: &'a LoadingScreenAtlas,
    composition: Option<&'a LoadingCompositionSnapshot>,
    backing_rgb: [f32; 3],
    text_rgb: [f32; 3],
    render_size: [u32; 2],
    cadence: NativeLoadingProgressCadence,
}

impl LoadingProgressSink for RenderingProgressSink<'_> {
    fn milestone(&mut self, raw_percent: u32) {
        let effective_percent = self.cadence.effective_percent(raw_percent);
        if self.progress.advance_progress(effective_percent) {
            if let Err(err) = present_native_loading(
                self.gpu,
                self.presenter,
                self.depth_view,
                self.batch,
                self.font,
                self.atlas,
                self.composition,
                self.progress_row,
                self.progress,
                self.backing_rgb,
                self.text_rgb,
                self.render_size,
            ) {
                log::warn!(
                    "Native loading repaint at raw milestone {raw_percent} \
                     (effective {effective_percent}) failed: {err:#}"
                );
            }
        }
    }
}

fn push_entry(
    out: &mut Vec<SpriteInstance>,
    entry: LoadingScreenEntry,
    position: [f32; 2],
    depth: f32,
) {
    push_entry_scaled(out, entry, position, entry.pixel_size, depth, [1.0; 3]);
}

/// Push a quad that resamples the whole source into `size`.
///
/// This is the ordinary scaling blit: the full atlas slot is sampled across the
/// destination rect, so a destination smaller than the source squashes the image
/// instead of cutting pieces off it.
fn push_entry_scaled(
    out: &mut Vec<SpriteInstance>,
    entry: LoadingScreenEntry,
    position: [f32; 2],
    size: [f32; 2],
    depth: f32,
    tint: [f32; 3],
) {
    out.push(SpriteInstance {
        position,
        size,
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint,
        alpha: 1.0,
        draw_state: DrawState::default(),
    });
}

/// Push a quad cropped to the preview region, dropping it when nothing is left.
///
/// gamemd composes the start markers into a surface exactly the size of the
/// preview region and blits that surface, so a marker whose nudge pushes it past
/// an edge is cut off there instead of spilling onto the loading art.
fn push_entry_clipped(
    out: &mut Vec<SpriteInstance>,
    entry: LoadingScreenEntry,
    position: [i32; 2],
    depth: f32,
    clip: MmpbRegionRect,
) {
    let width = entry.pixel_size[0] as i32;
    let height = entry.pixel_size[1] as i32;
    if width <= 0 || height <= 0 {
        return;
    }
    let left = position[0].max(clip.x);
    let top = position[1].max(clip.y);
    let right = (position[0] + width).min(clip.x + clip.width);
    let bottom = (position[1] + height).min(clip.y + clip.height);
    if right <= left || bottom <= top {
        return;
    }

    let visible = [(right - left) as f32, (bottom - top) as f32];
    let cropped = [(left - position[0]) as f32, (top - position[1]) as f32];
    out.push(SpriteInstance {
        position: [left as f32, top as f32],
        size: visible,
        uv_origin: [
            entry.uv_origin[0] + entry.uv_size[0] * cropped[0] / entry.pixel_size[0],
            entry.uv_origin[1] + entry.uv_size[1] * cropped[1] / entry.pixel_size[1],
        ],
        uv_size: [
            entry.uv_size[0] * visible[0] / entry.pixel_size[0],
            entry.uv_size[1] * visible[1] / entry.pixel_size[1],
        ],
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        draw_state: DrawState::default(),
    });
}

fn push_entry_tinted(
    out: &mut Vec<SpriteInstance>,
    entry: LoadingScreenEntry,
    position: [f32; 2],
    size: [f32; 2],
    depth: f32,
    tint: [f32; 3],
) {
    push_entry_scaled(out, entry, position, size, depth, tint);
}

/// Push the progress bar's filled span: `PROGBARM.SHP` frame 0 revealed from the
/// left, full height.
///
/// This is the one loading-screen layer that is *clipped* rather than scaled —
/// the bar sweeps by uncovering more of the same frame, so the U axis is cut at
/// the fill width while the V axis stays whole. Every other layer scales.
fn push_progress_fill(
    out: &mut Vec<SpriteInstance>,
    entry: LoadingScreenEntry,
    position: [f32; 2],
    fill_width: f32,
    depth: f32,
) {
    out.push(SpriteInstance {
        position,
        size: [fill_width, entry.pixel_size[1]],
        uv_origin: entry.uv_origin,
        uv_size: [
            entry.uv_size[0] * (fill_width / entry.pixel_size[0]).clamp(0.0, 1.0),
            entry.uv_size[1],
        ],
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        draw_state: DrawState::default(),
    });
}

fn gamemd_ftol_positive_domain(value: f64) -> i32 {
    debug_assert!(value >= 0.0);
    let nearest = value.round();
    if (value - nearest).abs() <= FTOL_EPSILON {
        return nearest as i32;
    }

    // Exact fractional x87 control-word behavior remains a narrow follow-up.
    value as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skirmish_launch::{
        AiDifficulty, LaunchCountry, LaunchStartPosition, LaunchTeam, SkirmishAiSlot,
        SkirmishLaunchMode, SkirmishLaunchOptions, SkirmishLocalSlot,
    };

    /// Test sink that applies the monotonic gate and records every milestone
    /// that actually advanced the bar, so the emit sequence can be asserted.
    struct RecordingProgressSink {
        progress: LoadingProgressState,
        emitted: Vec<u32>,
    }

    impl RecordingProgressSink {
        fn standard() -> Self {
            Self {
                progress: LoadingProgressState::standard_skirmish(),
                emitted: Vec::new(),
            }
        }
    }

    impl LoadingProgressSink for RecordingProgressSink {
        fn milestone(&mut self, percent: u32) {
            if self.progress.advance_progress(percent) {
                self.emitted.push(percent);
            }
        }
    }

    pub(super) fn test_launch_session(country: LaunchCountry) -> SkirmishLaunchSession {
        SkirmishLaunchSession {
            mode: SkirmishLaunchMode {
                id: 1,
                ui_name_key: "GUI:Battle".to_string(),
                tooltip_key: "STT:ModeBattle".to_string(),
                override_file: "MPBattleMD.ini".to_string(),
                map_filter: "standard".to_string(),
                random_maps_allowed: true,
                allies_allowed: true,
                must_ally: false,
            },
            selected_map_file: Some("mp01t4.map".to_string()),
            player_name: "Player".to_string(),
            local: SkirmishLocalSlot {
                country,
                country_random: false,
                color_index: 0,
                color_random: false,
                start_position: LaunchStartPosition::Position(0),
                team: LaunchTeam::None,
            },
            opponents: vec![SkirmishAiSlot {
                country: LaunchCountry::Russia,
                country_random: false,
                color_index: 1,
                color_random: false,
                start_position: LaunchStartPosition::Position(1),
                team: LaunchTeam::None,
                difficulty: AiDifficulty::Easy,
            }],
            pre_fill_house_roster:
                crate::skirmish_launch::PreFillHouseRoster::from_compact_skirmish(1),
            options: SkirmishLaunchOptions::default(),
        }
    }

    struct TestClock(u32);

    impl crate::match_bootstrap::MatchSeedClock for TestClock {
        fn low_u32(&mut self) -> u32 {
            self.0
        }

        fn source(&self) -> crate::match_bootstrap::MatchSeedSource {
            crate::match_bootstrap::MatchSeedSource::Controlled
        }

        fn seed_authority_certifying(&self) -> bool {
            true
        }
    }

    fn prepared_startup(next: &mut u64, seed: u32) -> crate::match_bootstrap::PreparedMatchStartup {
        let launch = test_launch_session(LaunchCountry::America);
        let accepted = match crate::match_bootstrap::classify_startup_session(&launch) {
            crate::match_bootstrap::StartupSessionClassification::AcceptedExplicitFixedBattle(
                accepted,
            ) => accepted,
            other => panic!("fixture was not accepted: {other:?}"),
        };
        let correlation = crate::match_bootstrap::allocate_match_correlation(next).unwrap();
        crate::match_bootstrap::prepare_match_startup(correlation, accepted, &mut TestClock(seed))
    }

    pub(super) fn unverified_seed(value: u32) -> crate::match_bootstrap::MatchSeed {
        crate::match_bootstrap::MatchSeed {
            value,
            source: crate::match_bootstrap::MatchSeedSource::Controlled,
            seed_authority_certifying: false,
        }
    }

    fn prefix_test_map(starts: &[(u8, u16, u16)]) -> crate::map::map_file::MapFile {
        let mut map =
            crate::map::rmg::emit::empty_map_file(&crate::map::rmg::RmgOptions::default(), 32, 32);
        map.waypoints.extend(starts.iter().map(|&(slot, rx, ry)| {
            (
                u32::from(slot),
                crate::map::waypoints::Waypoint {
                    index: u32::from(slot),
                    rx,
                    ry,
                },
            )
        }));
        map
    }

    fn generated_preview_with_starts(
        seed: u16,
        starts: &[(u8, u16, u16)],
    ) -> crate::map::rmg::GeneratedMap {
        crate::map::rmg::GeneratedMap {
            map_file: prefix_test_map(starts),
            mapgen_continuation: crate::map::rmg::RmgRng::new(seed).into_continuation(),
            construction_trace: crate::map::rmg::RmgConstructionTrace::default(),
            start_waypoints: starts.to_vec(),
            stages_run: Vec::new(),
            unfilled_start_slots: 0,
        }
    }

    fn accepted_random_map_with_starts(
        selected_map_file: &str,
        seed: u16,
        preview_waypoints: &[(u8, u16, u16)],
        staged_starts: &[(u8, u16, u16)],
    ) -> crate::app::shell_random_map::AcceptedRandomMapLaunch {
        let mut retention = crate::app::shell_random_map::RandomMapGenerationRetention::default();
        let mut generated = generated_preview_with_starts(seed, staged_starts);
        generated.map_file = prefix_test_map(preview_waypoints);
        retention.finish_generation(generated);
        retention.accept_setup(selected_map_file);
        retention
            .take_acceptance_for_loading(Some(selected_map_file))
            .expect("matching accepted random-map fixture")
    }

    fn receipt_for(
        startup: &crate::match_bootstrap::PreparedMatchStartup,
    ) -> crate::match_bootstrap::RustL0Receipt {
        let simulation = crate::sim::world::Simulation::with_seed(u64::from(startup.seed.value));
        crate::match_bootstrap::RustL0Observation {
            startup,
            simulation: &simulation,
            active_correlation: startup.correlation,
            prior_receipt: None,
            screen_is_loading: true,
            spawn_pick_active: false,
        }
        .acknowledge()
        .expect("valid test startup must acknowledge")
    }

    #[test]
    fn loading_side_comes_from_first_launch_node_country() {
        let session = LoadingSession::from_request(LoadingRequest::unverified_legacy_skirmish(
            test_launch_session(LaunchCountry::Korea),
            unverified_seed(1),
            SkirmishSettings::default(),
        ));

        assert_eq!(
            session.native.as_ref().map(|native| native.variant),
            Some(LoadingArtVariant::Alliance)
        );
    }

    #[test]
    fn loading_progress_row_snapshots_the_launch_player_name() {
        let mut launch = test_launch_session(LaunchCountry::America);
        launch.player_name = "Commander".to_owned();
        let session = LoadingSession::from_request(LoadingRequest::unverified_legacy_skirmish(
            launch,
            unverified_seed(22),
            SkirmishSettings::default(),
        ));

        assert_eq!(
            session
                .native
                .as_ref()
                .map(|native| native.progress_row.label.as_str()),
            Some("Commander"),
        );
    }

    #[test]
    fn loading_session_preserves_selected_map_filename() {
        let session = LoadingSession::from_request(LoadingRequest::unverified_legacy_skirmish(
            test_launch_session(LaunchCountry::Yuri),
            unverified_seed(2),
            SkirmishSettings::default(),
        ));

        assert_eq!(session.request.selected_map_file(), "mp01t4.map");
        assert_eq!(
            session
                .request
                .skirmish_launch_session()
                .and_then(|launch| launch.selected_map_file.as_deref()),
            Some("mp01t4.map")
        );
    }

    #[test]
    fn loading_session_selects_native_progress_cadence_from_map_kind() {
        let selected = LoadingSession::from_request(LoadingRequest::unverified_legacy_skirmish(
            test_launch_session(LaunchCountry::America),
            unverified_seed(20),
            SkirmishSettings::default(),
        ));
        assert_eq!(
            selected
                .native
                .as_ref()
                .map(|native| native.progress_cadence),
            Some(NativeLoadingProgressCadence::SelectedMap)
        );

        let mut random_map = test_launch_session(LaunchCountry::America);
        random_map.selected_map_file = Some("RandMap.Sed".to_string());
        let random = LoadingSession::from_request(LoadingRequest::unverified_legacy_skirmish(
            random_map,
            unverified_seed(21),
            SkirmishSettings::default(),
        ));
        assert_eq!(
            random.native.as_ref().map(|native| native.progress_cadence),
            Some(NativeLoadingProgressCadence::RandomMapHalved)
        );
    }

    #[test]
    fn gsi_04_12_loading_request_preview_is_presentation_only() {
        use crate::map::waypoints::Waypoint;

        let mut launch = test_launch_session(LaunchCountry::America);
        launch.selected_map_file = Some("RandMap.Sed".to_string());
        let mut map =
            crate::map::rmg::emit::empty_map_file(&crate::map::rmg::RmgOptions::default(), 32, 32);
        map.waypoints.insert(
            0,
            Waypoint {
                index: 0,
                rx: 10,
                ry: 20,
            },
        );
        map.waypoints.insert(
            1,
            Waypoint {
                index: 1,
                rx: 30,
                ry: 40,
            },
        );
        let generated = crate::map::rmg::GeneratedMap {
            map_file: map,
            mapgen_continuation:
                crate::map::rmg::RmgRng::new(0x1234).into_continuation(),
            construction_trace: crate::map::rmg::RmgConstructionTrace::default(),
            start_waypoints: vec![(0, 10, 20), (1, 30, 40)],
            stages_run: Vec::new(),
            unfilled_start_slots: 0,
        };
        let request = LoadingRequest::unverified_legacy_skirmish(
            launch.clone(),
            unverified_seed(0x1234),
            SkirmishSettings::default(),
        )
        .with_random_map_preview(Some(generated));

        assert_eq!(
            request
                .random_map_preview()
                .expect("loading request owns setup preview")
                .start_waypoints,
            vec![(0, 10, 20), (1, 30, 40)]
        );
        let assignments = selected_map_start_assignments(&launch, None);
        assert!(
            assignments.is_empty(),
            "preview waypoints cannot resolve gameplay starts"
        );
    }

    #[test]
    fn gsi_04_12_generated_prefix_uses_accepted_staging_once() {
        use crate::map::map_file::MapCell;

        let selected = "RandMap.Sed";
        let staged = [(0, 70, 70)];
        let preview_waypoints = [(0, 110, 110), (1, 130, 110)];
        let regenerated = [(0, 70, 90), (1, 90, 90)];
        let mut launch = test_launch_session(LaunchCountry::America);
        launch.selected_map_file = Some(selected.to_string());
        let accepted =
            accepted_random_map_with_starts(selected, 0x1212, &preview_waypoints, &staged);
        let mut regenerated_map = crate::map::rmg::emit::empty_map_file(
            &crate::map::rmg::RmgOptions::default(),
            100,
            100,
        );
        regenerated_map
            .waypoints
            .extend(regenerated.iter().map(|&(slot, rx, ry)| {
                (
                    u32::from(slot),
                    crate::map::waypoints::Waypoint {
                        index: u32::from(slot),
                        rx,
                        ry,
                    },
                )
            }));
        regenerated_map.cells = [(60, 60), (60, 140), (140, 60), (140, 140)]
            .into_iter()
            .map(|(rx, ry)| MapCell {
                rx,
                ry,
                tile_index: 0,
                sub_tile: 0,
                z: 0,
            })
            .collect();
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            regenerated_map,
            crate::app::frontend::list_maps::LoadedMapSource::Generated {
                seed_name: selected.to_ascii_lowercase(),
            },
        );
        let mut request = LoadingRequest::unverified_legacy_skirmish(
            launch.clone(),
            unverified_seed(0x1212),
            SkirmishSettings::default(),
        )
        .with_accepted_random_map(Some(accepted));

        request
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap();
        let context = request
            .fresh_scenario_load_context()
            .expect("generated launch prepares a required typed context");
        let projection = context.stock_offline_projection();
        let final_starts = projection
            .final_gathered_starts()
            .iter()
            .map(|waypoint| (waypoint.index as u8, waypoint.rx, waypoint.ry))
            .collect::<Vec<_>>();
        assert_eq!(final_starts.len(), 2);
        assert_eq!(final_starts[0], staged[0]);
        let gathered_fallback = final_starts[1];
        assert!(
            !staged.contains(&gathered_fallback)
                && !preview_waypoints.contains(&gathered_fallback)
                && !regenerated.contains(&gathered_fallback),
            "deficient Gather must add a distinct temporary start"
        );
        assert_ne!(final_starts, preview_waypoints);
        assert_ne!(final_starts, regenerated);
        let active_starts = crate::map::waypoints::multiplayer_start_waypoints(
            projection.active_scenario_waypoints(),
        )
        .into_iter()
        .map(|waypoint| (waypoint.index as u8, waypoint.rx, waypoint.ry))
        .collect::<Vec<_>>();
        assert_eq!(active_starts, staged);

        let assignments = selected_map_start_assignments(&launch, Some(projection));
        let composition = build_random_map_loading_composition(
            &launch,
            None,
            [800, 600],
            Some(DecodedPreview {
                width: 300,
                height: 100,
                rgba: vec![255; 300 * 100 * 4],
            }),
            initial.map_data(),
            projection.active_scenario_waypoints(),
            &assignments,
        );
        assert_eq!(
            composition
                .markers
                .iter()
                .map(|marker| {
                    (
                        marker.waypoint.index as u8,
                        marker.waypoint.rx,
                        marker.waypoint.ry,
                    )
                })
                .collect::<Vec<_>>(),
            staged,
            "random loading markers read the active Scenario staging copy"
        );

        let staged_session = crate::app::loading::init::scenario_start_waypoints_for_load(
            initial.map_data(),
            Some(projection),
        );
        let regenerated_session = crate::app::loading::init::scenario_start_waypoints_for_load(
            initial.map_data(),
            None,
        );
        assert_eq!(
            staged_session
                .iter()
                .map(|(&index, &(rx, ry))| (index as u8, rx, ry))
                .collect::<Vec<_>>(),
            staged
        );
        assert_ne!(staged_session, regenerated_session);
        let gathered_session = final_starts
            .iter()
            .map(|&(index, rx, ry)| (u32::from(index), (rx, ry)))
            .collect();
        assert_ne!(
            staged_session, gathered_session,
            "the active Scenario table excludes Gather-only fallback starts"
        );
        let staged_sim = crate::sim::world::Simulation::from_descriptor(
            &crate::sim::scenario_session::ScenarioDescriptor {
                seed: 0x1212,
                map_width: 256,
                map_height: 256,
                mp_start_waypoints: staged_session,
                ..Default::default()
            },
        );
        let regenerated_sim = crate::sim::world::Simulation::from_descriptor(
            &crate::sim::scenario_session::ScenarioDescriptor {
                seed: 0x1212,
                map_width: 256,
                map_height: 256,
                mp_start_waypoints: regenerated_session,
                ..Default::default()
            },
        );
        let gathered_sim = crate::sim::world::Simulation::from_descriptor(
            &crate::sim::scenario_session::ScenarioDescriptor {
                seed: 0x1212,
                map_width: 256,
                map_height: 256,
                mp_start_waypoints: gathered_session,
                ..Default::default()
            },
        );
        assert_eq!(
            staged_sim
                .session
                .mp_start_waypoints
                .iter()
                .map(|(&index, &(rx, ry))| (index as u8, rx, ry))
                .collect::<Vec<_>>(),
            staged
        );
        assert_ne!(staged_sim.state_hash(), gathered_sim.state_hash());
        assert_ne!(staged_sim.state_hash(), regenerated_sim.state_hash());
        assert!(
            request.accepted_rmg_start_staging.is_none(),
            "accepted setup staging transfers exactly once"
        );
        assert!(
            request.random_map_preview().is_some(),
            "presentation preview remains available to loading composition"
        );
        request
            .prepare_fresh_scenario_load_context(&initial)
            .expect("re-entry observes the already prepared plan without another transfer");
        assert!(request.accepted_rmg_start_staging.is_none());
    }

    #[test]
    fn load_descriptor_source_family_format_matrix() {
        use crate::app::loading::fresh_scenario::{
            FreshMapMaterialization, FreshScenarioFamily, FreshStartupProvenance,
        };

        let starts = [(0, 20, 24), (1, 42, 46)];
        let signed_formats = [
            (None, 0, false),
            (Some(1), 1, false),
            (Some(2), 2, true),
            (Some(4), 4, true),
            (Some(-7), -7, false),
        ];
        for source in [
            crate::app::frontend::list_maps::LoadedMapSource::Loose {
                path: std::path::PathBuf::from("mp01t4.map"),
                payload_len: 17,
            },
            crate::app::frontend::list_maps::LoadedMapSource::Mix {
                logical_name: "mp01t4.map".to_string(),
                source_archive: "mapsmd03.mix".to_string(),
                entry_id: 0x1234,
                payload_len: 19,
            },
        ] {
            for (format, expected_signed, expected_pack_gate) in signed_formats {
                let mut map = prefix_test_map(&starts);
                map.basic.new_ini_format = format;
                let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
                    map,
                    source.clone(),
                );
                let mut request = LoadingRequest::unverified_legacy_skirmish(
                    test_launch_session(LaunchCountry::America),
                    unverified_seed(0x1A2B_3C4D),
                    SkirmishSettings::default(),
                );
                request
                    .prepare_fresh_scenario_load_context(&initial)
                    .expect("authored Loose/MIX stock context");
                let context = request.fresh_scenario_load_context().unwrap();
                assert_eq!(context.physical_source(), &source);
                assert_eq!(context.materialization(), FreshMapMaterialization::Authored);
                assert_eq!(context.family(), FreshScenarioFamily::StockOffline);
                assert_eq!(
                    context.startup_provenance(),
                    FreshStartupProvenance::ResolvedLegacy
                );
                assert_eq!(context.match_seed(), 0x1A2B_3C4D);
                assert_eq!(context.signed_new_ini_format(), expected_signed);
                context
                    .validate_terminal_transfer(request.startup(), &source, expected_signed)
                    .expect("the independently moved terminal owners still agree");
                assert_eq!(
                    context.authored_pack_bodies_enabled(),
                    expected_pack_gate,
                    "only signed NewINIFormat > 1 gates authored pack bodies"
                );
            }
        }

        for mode_id in [1, 2] {
            let selected = format!("Accepted{mode_id}.SED");
            let mut launch = test_launch_session(LaunchCountry::America);
            launch.selected_map_file = Some(selected.clone());
            if mode_id == 2 {
                launch.mode = SkirmishLaunchMode {
                    id: 2,
                    ui_name_key: "GUI:FreeForAll".to_string(),
                    tooltip_key: "STT:ModeFreeForAll".to_string(),
                    override_file: "MPFreeForAllMD.ini".to_string(),
                    map_filter: "standard".to_string(),
                    random_maps_allowed: true,
                    allies_allowed: false,
                    must_ally: false,
                };
            }
            let accepted = accepted_random_map_with_starts(
                &selected,
                0x2345,
                &starts,
                &starts,
            );
            let mut map = prefix_test_map(&starts);
            map.basic.new_ini_format = Some(4);
            let source = crate::app::frontend::list_maps::LoadedMapSource::Generated {
                seed_name: selected.to_ascii_lowercase(),
            };
            let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
                map,
                source.clone(),
            );
            let mut request = LoadingRequest::unverified_legacy_skirmish(
                launch,
                unverified_seed(0x2345),
                SkirmishSettings::default(),
            )
            .with_accepted_random_map(Some(accepted));
            request
                .prepare_fresh_scenario_load_context(&initial)
                .expect("accepted Battle/FFA generated context");
            let context = request.fresh_scenario_load_context().unwrap();
            assert_eq!(context.physical_source(), &source);
            assert_eq!(
                context.materialization(),
                FreshMapMaterialization::AcceptedGenerated
            );
            assert_eq!(context.signed_new_ini_format(), 4);
            context
                .validate_terminal_transfer(request.startup(), &source, 4)
                .expect("generated terminal owners still agree");
            assert!(
                !context.authored_pack_bodies_enabled(),
                "serialized format cannot turn generated materialization into authored Mark"
            );
        }
    }

    #[test]
    fn accepted_and_resolved_legacy_share_one_stock_cursor_shape() {
        use crate::app::loading::fresh_scenario::{
            FreshScenarioFamily, FreshStartupProvenance,
        };

        let starts = [(0, 20, 24), (1, 42, 46)];
        let source = crate::app::frontend::list_maps::LoadedMapSource::Loose {
            path: std::path::PathBuf::from("mp01t4.map"),
            payload_len: 23,
        };
        let mut map = prefix_test_map(&starts);
        map.basic.new_ini_format = Some(4);
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            map,
            source,
        );
        let seed = 0x3456_789A;
        let mut next = 1;
        let prepared = prepared_startup(&mut next, seed);
        let legacy_session = prepared.session.launch_session().clone();
        let mut accepted =
            LoadingRequest::accepted_skirmish(prepared, SkirmishSettings::default());
        let mut resolved_legacy = LoadingRequest::unverified_legacy_skirmish(
            legacy_session,
            unverified_seed(seed),
            SkirmishSettings::default(),
        );
        accepted
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap();
        resolved_legacy
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap();
        let accepted_context = accepted.fresh_scenario_load_context().unwrap();
        let legacy_context = resolved_legacy.fresh_scenario_load_context().unwrap();
        assert_eq!(accepted_context.family(), FreshScenarioFamily::StockOffline);
        assert_eq!(accepted_context.family(), legacy_context.family());
        assert_eq!(accepted_context.match_seed(), legacy_context.match_seed());
        assert_eq!(
            accepted_context.stock_offline_projection().final_gathered_starts(),
            legacy_context
                .stock_offline_projection()
                .final_gathered_starts()
        );
        assert_eq!(
            accepted_context.stock_offline_projection().start_table(),
            legacy_context.stock_offline_projection().start_table()
        );
        assert_eq!(
            accepted_context.startup_provenance(),
            FreshStartupProvenance::Accepted
        );
        assert_eq!(
            legacy_context.startup_provenance(),
            FreshStartupProvenance::ResolvedLegacy
        );

        let accepted_parts = accepted
            .take_fresh_scenario_load_context()
            .unwrap()
            .into_stock_offline_parts();
        let legacy_parts = resolved_legacy
            .take_fresh_scenario_load_context()
            .unwrap()
            .into_stock_offline_parts();
        let mut accepted_owner = crate::sim::scenario_bootstrap::ScenarioBootstrapRng::new(seed);
        let mut legacy_owner = crate::sim::scenario_bootstrap::ScenarioBootstrapRng::new(seed);
        let _ = accepted_owner
            .install_pre_fill_scenario_prefix_plan(accepted_parts.scenario_prefix)
            .unwrap();
        let _ = legacy_owner
            .install_pre_fill_scenario_prefix_plan(legacy_parts.scenario_prefix)
            .unwrap();
        assert_eq!(
            accepted_owner.logical_states_for_test(),
            legacy_owner.logical_states_for_test(),
            "startup provenance cannot change the verified stock prefix cursor"
        );
    }

    #[test]
    fn generic_manual_and_unresolved_legacy_reject_before_receipt_or_staging() {
        let selected = "Rejected.SED";
        let starts = [(0, 20, 24), (1, 42, 46)];
        let source = crate::app::frontend::list_maps::LoadedMapSource::Generated {
            seed_name: selected.to_string(),
        };
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&starts),
            source,
        );
        let accepted = accepted_random_map_with_starts(selected, 0x4567, &starts, &starts);
        let mut generic = LoadingRequest::generic_map_load(
            selected,
            SkirmishSettings::default(),
        )
        .with_accepted_random_map(Some(accepted));
        let generic_err = generic
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(format!("{generic_err:#}").contains("Generic startup"));
        assert!(generic.fresh_scenario_load_context().is_none());
        assert!(generic.accepted_rmg_start_staging.is_some());

        let accepted = accepted_random_map_with_starts(selected, 0x4567, &starts, &starts);
        let mut unresolved_session = test_launch_session(LaunchCountry::America);
        unresolved_session.selected_map_file = Some(selected.to_string());
        unresolved_session.local.country_random = true;
        let mut unresolved = LoadingRequest::unverified_legacy_skirmish(
            unresolved_session,
            unverified_seed(0x4567),
            SkirmishSettings::default(),
        )
        .with_accepted_random_map(Some(accepted));
        let unresolved_err = unresolved
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(
            format!("{unresolved_err:#}").contains("local slot still has a random country")
        );
        assert!(unresolved.fresh_scenario_load_context().is_none());
        assert!(unresolved.accepted_rmg_start_staging.is_some());

        let authored = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&starts),
            crate::app::frontend::list_maps::LoadedMapSource::Loose {
                path: std::path::PathBuf::from("manual.map"),
                payload_len: 1,
            },
        );
        let mut manual_session = test_launch_session(LaunchCountry::America);
        manual_session.selected_map_file = Some(" auto ".to_string());
        let mut manual = LoadingRequest::unverified_legacy_skirmish(
            manual_session,
            unverified_seed(0x4567),
            SkirmishSettings::default(),
        );
        let manual_err = manual
            .prepare_fresh_scenario_load_context(&authored)
            .unwrap_err();
        assert!(format!("{manual_err:#}").contains("no exact selected map record"));
        assert!(manual.fresh_scenario_load_context().is_none());
    }

    #[test]
    fn loading_assignments_keep_gather_table_slots_separate_from_sparse_waypoint_indices() {
        use crate::app::loading::composition::{
            PreviewAspectFit, ProjectedPlayfieldBounds, build_mmpb_marker_records,
            native_loading_waypoint_prefix,
        };

        let mut launch = test_launch_session(LaunchCountry::America);
        let mut second_opponent = launch.opponents[0].clone();
        second_opponent.color_index = 2;
        second_opponent.start_position = LaunchStartPosition::Position(2);
        launch.opponents.push(second_opponent);
        launch.pre_fill_house_roster =
            crate::skirmish_launch::PreFillHouseRoster::from_compact_skirmish(2);

        // Gather target 3 retains raw slots 0 and 2, then appends a fallback
        // at vector position 2. The retained slot and fallback consequently
        // carry the same Waypoint::index even though the assignment table has
        // three distinct positions.
        let map = prefix_test_map(&[(0, 20, 24), (2, 42, 46), (3, 54, 58)]);
        let descriptor = crate::sim::scenario_bootstrap::MatchLaunchDescriptor::from_resolved(
            launch.clone(),
        )
        .unwrap();
        let plan = crate::sim::scenario_bootstrap::prepare_stock_offline_scenario_prefix_plan(
            &descriptor,
            &map,
            &map.waypoints,
            0x1223,
        )
        .unwrap();
        assert_eq!(
            plan.final_gathered_starts()
                .iter()
                .map(|waypoint| waypoint.index)
                .collect::<Vec<_>>(),
            vec![0, 2, 2]
        );

        let assignments = selected_map_start_assignments(&launch, Some(plan.projection()));
        assert_eq!(
            assignments
                .iter()
                .map(|assignment| (assignment.start_index, assignment.participant))
                .collect::<Vec<_>>(),
            vec![
                (0, LoadingParticipantId::Local),
                (1, LoadingParticipantId::Opponent(0)),
                (2, LoadingParticipantId::Opponent(1)),
            ],
            "loading assignments are keyed by Scenario start-table position"
        );

        // The compositor still reads original Scenario geometry. Native's odd
        // sparse-prefix rule visits raw waypoint 2 and colors it from table[2],
        // not from the retained Gather vector entry at position 1.
        let raw_prefix = native_loading_waypoint_prefix(&map.waypoints);
        let markers = build_mmpb_marker_records(
            &raw_prefix,
            &assignments,
            ProjectedPlayfieldBounds {
                min_x: 0,
                min_y: 0,
                extent_x: 1_000,
                extent_y: 1_000,
            },
            MmpbRegionRect {
                x: 0,
                y: 0,
                width: 200,
                height: 200,
            },
            PreviewAspectFit {
                scale_1000: 1_000,
                width: 200,
                height: 200,
                pad_x: 0,
                pad_y: 0,
            },
        );
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[1].waypoint.index, 2);
        assert_eq!(
            markers[1].participant,
            LoadingParticipantId::Opponent(1)
        );
    }

    #[test]
    fn gsi_04_12_generated_prefix_rejects_presentation_only_preview() {
        let selected = "RandMap.Sed";
        let starts = [(0, 20, 24), (1, 42, 46)];
        let mut launch = test_launch_session(LaunchCountry::America);
        launch.selected_map_file = Some(selected.to_string());
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&starts),
            crate::app::frontend::list_maps::LoadedMapSource::Generated {
                seed_name: selected.to_string(),
            },
        );
        let mut request = LoadingRequest::unverified_legacy_skirmish(
            launch,
            unverified_seed(0x1313),
            SkirmishSettings::default(),
        )
        .with_random_map_preview(Some(generated_preview_with_starts(0x1313, &starts)));

        let err = request
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("no accepted setup start staging"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn generated_prefix_rejects_mismatched_source_name() {
        let selected = "RandMap.Sed";
        let starts = [(0, 20, 24), (1, 42, 46)];
        let mut launch = test_launch_session(LaunchCountry::America);
        launch.selected_map_file = Some(selected.to_string());
        let accepted = accepted_random_map_with_starts(selected, 0x1414, &starts, &starts);
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&starts),
            crate::app::frontend::list_maps::LoadedMapSource::Generated {
                seed_name: "Other.Sed".to_string(),
            },
        );
        let mut request = LoadingRequest::unverified_legacy_skirmish(
            launch,
            unverified_seed(0x1414),
            SkirmishSettings::default(),
        )
        .with_accepted_random_map(Some(accepted));

        let err = request
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("does not match selected record"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn generated_prefix_rejects_cooperative_mode() {
        let selected = "RandMap.Sed";
        let starts = [(0, 20, 24), (1, 42, 46)];
        let mut launch = test_launch_session(LaunchCountry::America);
        launch.selected_map_file = Some(selected.to_string());
        let cooperative = crate::skirmish_modes::stock_skirmish_modes()
            .into_iter()
            .find(|mode| mode.id == 3)
            .expect("retail Cooperative row");
        launch.mode = SkirmishLaunchMode::from_game_mode(&cooperative);
        let accepted = accepted_random_map_with_starts(selected, 0x1515, &starts, &starts);
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&starts),
            crate::app::frontend::list_maps::LoadedMapSource::Generated {
                seed_name: selected.to_string(),
            },
        );
        let mut request = LoadingRequest::unverified_legacy_skirmish(
            launch,
            unverified_seed(0x1515),
            SkirmishSettings::default(),
        )
        .with_accepted_random_map(Some(accepted));

        let err = request
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("unsupported for stock mode id 3"),
            "unexpected error: {err:#}"
        );
        assert!(
            request.accepted_rmg_start_staging.is_some(),
            "unsupported family rejects before consuming accepted staging"
        );
    }

    #[test]
    fn generated_prefix_rejects_a_spoofed_stock_row_before_consuming_staging() {
        let selected = "RandMap.SED";
        let starts = [(0, 20, 24), (1, 42, 46)];
        let mut launch = test_launch_session(LaunchCountry::America);
        launch.selected_map_file = Some(selected.to_string());
        launch.mode.tooltip_key = "STT:SpoofedBattle".to_string();
        let accepted = accepted_random_map_with_starts(selected, 0x1516, &starts, &starts);
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&starts),
            crate::app::frontend::list_maps::LoadedMapSource::Generated {
                seed_name: selected.to_string(),
            },
        );
        let mut request = LoadingRequest::unverified_legacy_skirmish(
            launch,
            unverified_seed(0x1516),
            SkirmishSettings::default(),
        )
        .with_accepted_random_map(Some(accepted));

        let err = request
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(format!("{err:#}").contains("not the validated active-retail stock row"));
        assert!(request.accepted_rmg_start_staging.is_some());
        assert!(request.fresh_scenario_load_context().is_none());
    }

    #[test]
    fn authored_prefix_rejects_random_map_staging_for_loose_and_mix_sources() {
        let selected = "mp01t4.map";
        let starts = [(0, 20, 24), (1, 42, 46)];
        let sources = [
            crate::app::frontend::list_maps::LoadedMapSource::Loose {
                path: std::path::PathBuf::from(selected),
                payload_len: 1,
            },
            crate::app::frontend::list_maps::LoadedMapSource::Mix {
                logical_name: selected.to_string(),
                source_archive: "mapsmd03.mix".to_string(),
                entry_id: 7,
                payload_len: 1,
            },
        ];
        for source in sources {
            let mut launch = test_launch_session(LaunchCountry::America);
            launch.selected_map_file = Some(selected.to_string());
            let accepted = accepted_random_map_with_starts(selected, 0x1616, &starts, &starts);
            let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
                prefix_test_map(&starts),
                source,
            );
            let mut request = LoadingRequest::unverified_legacy_skirmish(
                launch,
                unverified_seed(0x1616),
                SkirmishSettings::default(),
            )
            .with_accepted_random_map(Some(accepted));

            let err = request
                .prepare_fresh_scenario_load_context(&initial)
                .unwrap_err();
            assert!(
                format!("{err:#}").contains("cannot attach to an authored map source"),
                "unexpected error: {err:#}"
            );
        }
    }

    #[test]
    fn stock_prefix_rejects_legacy_fallback_source() {
        let starts = [(0, 20, 24), (1, 42, 46)];
        let launch = test_launch_session(LaunchCountry::America);
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&starts),
            crate::app::frontend::list_maps::LoadedMapSource::LegacyFallback {
                label: "fixture".to_string(),
            },
        );
        let mut request = LoadingRequest::unverified_legacy_skirmish(
            launch,
            unverified_seed(0x1717),
            SkirmishSettings::default(),
        );

        let err = request
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("requires an exact Loose, MIX, or accepted generated source"),
            "unexpected error: {err:#}"
        );

        let mut next = 1;
        let mut accepted = LoadingRequest::accepted_skirmish(
            prepared_startup(&mut next, 0x1717),
            SkirmishSettings::default(),
        );
        let accepted_err = accepted
            .prepare_fresh_scenario_load_context(&initial)
            .unwrap_err();
        assert!(
            format!("{accepted_err:#}")
                .contains("requires an exact Loose, MIX, or accepted generated source"),
            "unexpected error: {accepted_err:#}"
        );
    }

    #[test]
    fn stock_launch_terminal_transfer_requires_ready_prefix() {
        let mut pending = LoadingRequest::unverified_legacy_skirmish(
            test_launch_session(LaunchCountry::America),
            unverified_seed(0x1818),
            SkirmishSettings::default(),
        );
        let pending_err = pending
            .take_fresh_scenario_load_context()
            .unwrap_err();
        assert!(
            format!("{pending_err:#}").contains("before fresh scenario admission"),
            "unexpected error: {pending_err:#}"
        );
        let initial = crate::app::loading::init::MapLoadInitial::from_test_map_source(
            prefix_test_map(&[(0, 20, 24), (1, 42, 46)]),
            crate::app::frontend::list_maps::LoadedMapSource::Loose {
                path: std::path::PathBuf::from("mp01t4.map"),
                payload_len: 1,
            },
        );
        pending
            .prepare_fresh_scenario_load_context(&initial)
            .expect("resolved authored stock launch admits once");
        let _context = pending
            .take_fresh_scenario_load_context()
            .expect("ready context transfers once");
        let transferred_err = pending
            .take_fresh_scenario_load_context()
            .unwrap_err();
        assert!(
            format!("{transferred_err:#}").contains("transfers exactly once"),
            "unexpected error: {transferred_err:#}"
        );
    }

    #[test]
    fn gsi_04_12_stock_ffa_preview_can_only_supply_loading_fallback_pixels() {
        use crate::map::map_file::MapCell;
        use crate::map::waypoints::Waypoint;

        let mut launch = test_launch_session(LaunchCountry::America);
        launch.mode = SkirmishLaunchMode {
            id: 2,
            ui_name_key: "GUI:FreeForAll".to_string(),
            tooltip_key: "STT:ModeFreeForAll".to_string(),
            override_file: "MPFreeForAllMD.ini".to_string(),
            map_filter: "standard".to_string(),
            random_maps_allowed: true,
            allies_allowed: false,
            must_ally: false,
        };
        launch.selected_map_file = Some("RandMap.Sed".to_string());

        let mut map = crate::map::rmg::emit::empty_map_file(
            &crate::map::rmg::RmgOptions::default(),
            100,
            100,
        );
        map.cells = vec![
            MapCell {
                rx: 60,
                ry: 60,
                tile_index: 0,
                sub_tile: 0,
                z: 0,
            },
            MapCell {
                rx: 60,
                ry: 140,
                tile_index: 0,
                sub_tile: 0,
                z: 0,
            },
            MapCell {
                rx: 140,
                ry: 60,
                tile_index: 0,
                sub_tile: 0,
                z: 0,
            },
        ];
        map.waypoints.insert(
            0,
            Waypoint {
                index: 0,
                rx: 70,
                ry: 70,
            },
        );
        map.waypoints.insert(
            1,
            Waypoint {
                index: 1,
                rx: 90,
                ry: 70,
            },
        );
        map.waypoints.insert(
            2,
            Waypoint {
                index: 2,
                rx: 70,
                ry: 90,
            },
        );
        let mut launch_map = crate::map::rmg::emit::empty_map_file(
            &crate::map::rmg::RmgOptions::default(),
            100,
            100,
        );
        launch_map.cells = map.cells.clone();
        let active_scenario_waypoints = map.waypoints.clone();
        let generated = crate::map::rmg::GeneratedMap {
            map_file: map,
            mapgen_continuation:
                crate::map::rmg::RmgRng::new(0x4567).into_continuation(),
            construction_trace: crate::map::rmg::RmgConstructionTrace::default(),
            start_waypoints: vec![(0, 70, 70), (1, 90, 70), (2, 70, 90)],
            stages_run: Vec::new(),
            unfilled_start_slots: 0,
        };
        let _request = LoadingRequest::unverified_legacy_skirmish(
            launch.clone(),
            unverified_seed(0x4567),
            SkirmishSettings::default(),
        )
        .with_random_map_preview(Some(generated));
        let assignments = selected_map_start_assignments(&launch, None);
        assert!(
            assignments.is_empty(),
            "only the launch-time `.SED` regeneration may resolve participants"
        );
        let preview = DecodedPreview {
            width: 300,
            height: 100,
            rgba: vec![255; 300 * 100 * 4],
        };
        let composition = build_random_map_loading_composition(
            &launch,
            None,
            [800, 600],
            Some(preview),
            &launch_map,
            &active_scenario_waypoints,
            &assignments,
        );
        let prepared = composition.preview.expect("FFA retained preview");
        assert_eq!(
            prepared
                .image
                .rgba
                .chunks_exact(4)
                .filter(|pixel| *pixel == [0, 0, 0, 255])
                .count(),
            3 * 4 * 4,
            "all three valid FFA starts are burned black"
        );
        assert!(composition.markers.is_empty());
    }

    #[test]
    fn random_map_progress_uses_native_integer_halving_and_raw_200_terminal() {
        let cadence = NativeLoadingProgressCadence::RandomMapHalved;

        assert_eq!(cadence.effective_percent(3), 1);
        assert_eq!(cadence.effective_percent(90), 45);
        assert_eq!(
            cadence.effective_percent(cadence.terminal_raw_percent()),
            100
        );
        assert_eq!(
            NativeLoadingProgressCadence::SelectedMap.effective_percent(3),
            3
        );
        assert_eq!(
            NativeLoadingProgressCadence::SelectedMap.terminal_raw_percent(),
            100
        );
    }

    #[test]
    fn loading_session_falls_back_without_native_session_only_outside_parity_path() {
        let session = LoadingSession::from_request(LoadingRequest::generic_map_load(
            "auto",
            SkirmishSettings::default(),
        ));

        assert!(session.native.is_none());
        assert!(session.request.skirmish_launch_session().is_none());
        assert_eq!(session.request.selected_map_file(), "auto");
    }

    #[test]
    fn loading_session_starts_at_initial_map_selection_phase() {
        let session = LoadingSession::from_request(LoadingRequest::unverified_legacy_skirmish(
            test_launch_session(LaunchCountry::America),
            unverified_seed(3),
            SkirmishSettings::default(),
        ));

        assert!(matches!(
            session.job.phase,
            LoadingJobPhase::InitialMapSelection
        ));
    }

    #[test]
    fn loading_request_moves_exact_startup_authority_once() {
        struct Clock;
        impl crate::match_bootstrap::MatchSeedClock for Clock {
            fn low_u32(&mut self) -> u32 {
                0x1234_5678
            }

            fn source(&self) -> crate::match_bootstrap::MatchSeedSource {
                crate::match_bootstrap::MatchSeedSource::Controlled
            }

            fn seed_authority_certifying(&self) -> bool {
                true
            }
        }

        let session = test_launch_session(LaunchCountry::America);
        let accepted = match crate::match_bootstrap::classify_startup_session(&session) {
            crate::match_bootstrap::StartupSessionClassification::AcceptedExplicitFixedBattle(
                accepted,
            ) => accepted,
            other => panic!("fixture was not accepted: {other:?}"),
        };
        let mut next = 1;
        let correlation = crate::match_bootstrap::allocate_match_correlation(&mut next).unwrap();
        let mut clock = Clock;
        let prepared =
            crate::match_bootstrap::prepare_match_startup(correlation, accepted, &mut clock);
        let mut request =
            LoadingRequest::accepted_skirmish(prepared.clone(), SkirmishSettings::default());

        assert_eq!(request.startup().accepted(), Some(&prepared));
        assert_eq!(request.take_startup(), LoadingStartup::Accepted(prepared));
        assert!(
            request.startup.is_none(),
            "authority transfers exactly once"
        );
    }

    #[test]
    fn replacing_loading_startup_clears_prior_loaded_startup_and_receipt_then_registers_new_correlation()
     {
        let mut next = 1;
        let prior = prepared_startup(&mut next, 0x1111_2222);
        let replacement = prepared_startup(&mut next, 0x3333_4444);
        let prior_receipt = receipt_for(&prior);
        let prior_correlation = prior.correlation;
        let replacement_correlation = replacement.correlation;
        let mut active = Some(prior_correlation);
        let mut loaded = Some(prior);
        let mut receipt = Some(prior_receipt);

        replace_match_startup_slots(
            &mut active,
            &mut loaded,
            &mut receipt,
            Some(replacement_correlation),
        );

        assert_eq!(active, Some(replacement_correlation));
        assert!(loaded.is_none());
        assert!(receipt.is_none());
    }

    #[test]
    fn failed_startup_cleanup_clears_all_three_slots() {
        let mut next = 1;
        let prior = prepared_startup(&mut next, 0x5555_6666);
        let prior_receipt = receipt_for(&prior);
        let mut active = Some(prior.correlation);
        let mut loaded = Some(prior);
        let mut receipt = Some(prior_receipt);

        replace_match_startup_slots(&mut active, &mut loaded, &mut receipt, None);

        assert!(active.is_none());
        assert!(loaded.is_none());
        assert!(receipt.is_none());
    }

    #[test]
    fn loading_progress_standard_skirmish_initializes_one_lane_max_100() {
        let progress = LoadingProgressState::standard_skirmish();

        assert_eq!(progress.max_value(), 100.0);
        assert_eq!(progress.current_value(), 0.0);
        assert_eq!(progress.current_percent(), 0.0);
    }

    #[test]
    fn loading_progress_duplicate_milestones_do_not_redraw() {
        let mut progress = LoadingProgressState::standard_skirmish();

        assert!(progress.advance_progress(3));
        assert!(!progress.advance_progress(3));
        assert_eq!(progress.current_value(), 3.0);
    }

    #[test]
    fn loading_progress_lower_milestone_does_not_redraw() {
        let mut progress = LoadingProgressState::standard_skirmish();

        assert!(progress.advance_progress(8));
        assert!(!progress.advance_progress(6));
        assert_eq!(progress.current_value(), 8.0);
    }

    #[test]
    fn loading_progress_advancing_milestone_requests_redraw() {
        let mut progress = LoadingProgressState::standard_skirmish();

        assert!(progress.advance_progress(3));
        assert!(progress.advance_progress(8));
        assert_eq!(progress.current_value(), 8.0);
    }

    #[test]
    fn loading_progress_clipped_width_matches_native_formula_for_exact_values() {
        let mut progress = LoadingProgressState::standard_skirmish();

        assert_eq!(progress.fill_width_gamemd_ftol_positive_domain(326), 0);
        assert!(progress.advance_progress(50));
        assert_eq!(progress.fill_width_gamemd_ftol_positive_domain(326), 163);
        assert!(progress.advance_progress(100));
        assert_eq!(progress.fill_width_gamemd_ftol_positive_domain(326), 326);
    }

    #[test]
    fn loading_progress_fill_width_uses_gamemd_ftol_positive_domain() {
        let mut progress = LoadingProgressState::standard_skirmish();

        assert!(progress.advance_progress(25));
        assert_eq!(progress.fill_width_gamemd_ftol_positive_domain(400), 100);
    }

    #[test]
    fn loading_progress_suppresses_nonadvancing_raw_native_calls() {
        let mut progress = LoadingProgressState::standard_skirmish();

        assert!(progress.advance_progress(8));
        assert!(!progress.advance_progress(6));
        assert!(progress.advance_progress(60));
        assert!(!progress.advance_progress(58));
        assert!(!progress.advance_progress(60));
    }

    #[test]
    fn recording_sink_emits_full_monotonic_ledger_in_our_execution_order() {
        // The values the loaders emit, in the order our pipeline crosses them
        // (3 at present, 100 at Finished are emitted by the pump, not the loaders).
        let mut loader_emits = vec![8, 12];
        loader_emits.extend(theater_ramp_changed_values(42));
        loader_emits.extend([
            30, 31, 35, 45, 50, 55, 58, 60, 63, 65, 67, 68, 69, 70, 72, 74, 76, 78, 82, 86, 90, 93,
            96, 98,
        ]);
        let mut sink = RecordingProgressSink::standard();
        sink.milestone(3); // present handoff
        for v in loader_emits.iter().copied() {
            sink.milestone(v);
        }
        sink.milestone(100); // pump Finished

        // Every emitted value advanced the bar (no suppressed/duplicate values),
        // starts at 3, ends at 100, and is strictly monotonic.
        assert_eq!(sink.emitted.first(), Some(&3));
        assert_eq!(sink.emitted.last(), Some(&100));
        assert_eq!(sink.emitted.len(), loader_emits.len() + 2);
        assert!(sink.emitted.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn recording_sink_suppresses_nonadvancing_and_duplicate_milestones() {
        let mut sink = RecordingProgressSink::standard();
        // A stray lower/raw value (e.g. gamemd's raw 6 after 8) and duplicates
        // must not advance the bar.
        for v in [8, 6, 8, 12, 12, 30] {
            sink.milestone(v);
        }
        assert_eq!(sink.emitted, vec![8, 12, 30]);
    }

    #[test]
    fn loading_progress_standard_skirmish_selected_map_emits_verified_milestone_ledger() {
        let ramp = theater_ramp_changed_values(42);
        let mut expected = Vec::from([3, 8, 12]);
        expected.extend(ramp);
        expected.extend([
            30, 31, 35, 45, 50, 55, 58, 60, 63, 65, 67, 68, 69, 70, 72, 74, 76, 78, 82, 86, 90, 93,
            96, 98, 100,
        ]);

        let mut progress = LoadingProgressState::standard_skirmish();
        let emitted: Vec<u32> = expected
            .iter()
            .copied()
            .filter(|value| progress.advance_progress(*value))
            .collect();

        assert_eq!(emitted, expected);
    }

    #[test]
    fn loading_progress_theater_ramp_stock_rulesmd_count_emits_13_through_25() {
        let emitted = theater_ramp_changed_values(42);

        assert_eq!(emitted, (13..=25).collect::<Vec<_>>());
    }

    #[test]
    fn loading_progress_theater_ramp_nonmultiple_base_count_uses_native_quotient() {
        let emitted = theater_ramp_changed_values(38);

        assert_eq!(emitted, (13..=25).collect::<Vec<_>>());
    }

    #[test]
    fn loading_progress_theater_ramp_zero_or_invalid_small_count_has_no_dynamic_values() {
        assert!(theater_ramp_changed_values(0).is_empty());
        assert!(theater_ramp_changed_values(12).is_empty());
    }

    #[test]
    fn loading_theater_cache_mismatch_covers_first_same_and_changed_cases() {
        assert!(theater_cache_mismatch(false, "TEMPERATE", "TEMPERATE"));
        assert!(!theater_cache_mismatch(true, "TEMPERATE", "temperate"));
        assert!(theater_cache_mismatch(true, "TEMPERATE", "SNOW"));
    }

    #[test]
    fn loading_progress_read_ini_basic_milestones_precede_map_pack_milestones() {
        let sequence = STANDARD_SKIRMISH_SELECTED_MAP_MILESTONES_AFTER_FIRST_RENDER;
        let pos_55 = sequence.iter().position(|value| *value == 55).unwrap();
        let pos_60 = sequence.iter().position(|value| *value == 60).unwrap();
        let pos_63 = sequence.iter().position(|value| *value == 63).unwrap();

        assert!(pos_55 < pos_63);
        assert!(pos_60 < pos_63);
    }

    #[test]
    fn loading_progress_standard_skirmish_presents_on_advancing_milestones() {
        let mut progress = LoadingProgressState::standard_skirmish();

        let presents = [3, 8, 12, 25, 30]
            .into_iter()
            .filter(|value| progress.advance_progress(*value))
            .count();

        assert_eq!(presents, 5);
    }

    #[test]
    fn native_loading_state_keeps_the_full_player_progress_ramp() {
        let mk = |name: &str, hsv: [u8; 3]| ColorSchemeEntry {
            name: name.into(),
            hsv,
        };
        let ramps = HouseColorRamps::from_schemes(&[
            mk("Gold", [43, 239, 255]),
            mk("DarkBlue", [153, 214, 212]),
        ]);
        let mut native = NativeLoadingScreenState::standard_skirmish(
            LoadingArtVariant::Americans,
            0,
            HouseColorIndex(1),
            LoadingProgressRowSnapshot {
                label: "Player".to_owned(),
            },
            NativeLoadingProgressCadence::SelectedMap,
        );
        native.resolve_player_colors(
            &[mk("Gold", [43, 239, 255]), mk("DarkBlue", [153, 214, 212])],
            &ramps,
        );

        assert_eq!(native.runtime_color_scheme_count, 4);
        assert_eq!(native.progress_ramp, *ramps.ramp(HouseColorIndex(1)));
        assert_ne!(native.progress_ramp[0], native.progress_ramp[15]);
        assert!(native.progress_ramp[0].b > native.progress_ramp[0].r);
    }

    #[test]
    fn progress_row_starts_at_the_first_advancing_milestone() {
        let mut progress = LoadingProgressState::standard_skirmish();
        assert_eq!(progress.current_value(), 0.0);
        assert!(progress.advance_progress(3));
        assert!(progress.current_value() > 0.0);
    }

    /// Synthetic `mmpb.shp` frame-0 atlas slot: 12x12 pixels somewhere inside a
    /// shared atlas, so cropping has to move both the UV origin and the UV size.
    fn marker_entry() -> LoadingScreenEntry {
        LoadingScreenEntry {
            uv_origin: [0.25, 0.5],
            uv_size: [0.1, 0.2],
            pixel_size: [12.0, 12.0],
        }
    }

    #[test]
    fn a_preview_wider_than_its_region_is_squashed_whole_not_cropped() {
        use crate::app::loading::composition::{aspect_fit_preview, mmpb_region_rect};

        // A stock map whose projected preview overruns the 800-wide region: the
        // fit picks a destination narrower than the source, which is exactly the
        // case the bar's left-to-right U clamp used to silently crop.
        let region = mmpb_region_rect(800);
        let fit = aspect_fit_preview(region, 400, 200).expect("valid fit");
        assert!(fit.width < 400, "fixture must exercise a downscale");

        let entry = LoadingScreenEntry {
            uv_origin: [0.5, 0.25],
            uv_size: [0.4, 0.2],
            pixel_size: [400.0, 200.0],
        };
        let mut instances = Vec::new();
        push_entry_scaled(
            &mut instances,
            entry,
            [(region.x + fit.pad_x) as f32, (region.y + fit.pad_y) as f32],
            [fit.width as f32, fit.height as f32],
            PREVIEW_DEPTH,
            [1.0; 3],
        );

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].size, [fit.width as f32, fit.height as f32]);
        // The whole source is sampled on both axes; nothing is cut off.
        assert_eq!(instances[0].uv_origin, entry.uv_origin);
        assert_eq!(instances[0].uv_size, entry.uv_size);
    }

    #[test]
    fn the_progress_bar_is_the_only_layer_revealed_by_clipping_u() {
        let entry = LoadingScreenEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [0.4, 0.05],
            pixel_size: [400.0, 10.0],
        };
        let mut instances = Vec::new();

        push_progress_fill(&mut instances, entry, [24.0, 332.0], 100.0, PROGRESS_DEPTH);

        assert_eq!(instances.len(), 1);
        // A quarter of the frame is uncovered: a quarter of U, all of V.
        assert_eq!(instances[0].size, [100.0, 10.0]);
        assert_eq!(instances[0].uv_size, [0.4_f32 * 0.25, 0.05]);
    }

    #[test]
    fn both_native_loading_cadences_prepare_scenario_before_first_frame() {
        assert!(NativeLoadingProgressCadence::SelectedMap.prepares_scenario_before_first_frame());
        assert!(
            NativeLoadingProgressCadence::RandomMapHalved.prepares_scenario_before_first_frame()
        );
    }

    #[test]
    fn markers_inside_the_preview_region_draw_uncropped() {
        let clip = MmpbRegionRect::new(499, 379, 216, 166);
        let mut instances = Vec::new();

        push_entry_clipped(
            &mut instances,
            marker_entry(),
            [520, 400],
            MARKER_DEPTH,
            clip,
        );

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].position, [520.0, 400.0]);
        assert_eq!(instances[0].size, [12.0, 12.0]);
        assert_eq!(instances[0].uv_origin, [0.25, 0.5]);
        assert_eq!(instances[0].uv_size, [0.1, 0.2]);
    }

    #[test]
    fn markers_overhanging_the_preview_region_are_cut_off_at_its_edge() {
        let clip = MmpbRegionRect::new(499, 379, 216, 166);
        let mut instances = Vec::new();

        // 4 px past the region's right edge and 3 px above its top edge.
        push_entry_clipped(
            &mut instances,
            marker_entry(),
            [707, 376],
            MARKER_DEPTH,
            clip,
        );

        assert_eq!(instances.len(), 1);
        let instance = instances[0];
        assert_eq!(instance.position, [707.0, 379.0]);
        assert_eq!(instance.size, [8.0, 9.0]);
        // The three cropped top rows advance the UV origin; the four cropped
        // right columns are simply never sampled.
        assert_eq!(instance.uv_origin, [0.25_f32, 0.5 + 0.2 * 3.0 / 12.0]);
        assert_eq!(instance.uv_size, [0.1_f32 * 8.0 / 12.0, 0.2 * 9.0 / 12.0]);
    }

    #[test]
    fn markers_entirely_outside_the_preview_region_are_dropped() {
        let clip = MmpbRegionRect::new(499, 379, 216, 166);
        let mut instances = Vec::new();

        push_entry_clipped(
            &mut instances,
            marker_entry(),
            [715, 400],
            MARKER_DEPTH,
            clip,
        );
        push_entry_clipped(
            &mut instances,
            marker_entry(),
            [520, 367],
            MARKER_DEPTH,
            clip,
        );

        assert!(instances.is_empty());
    }

    #[test]
    fn loading_progress_duplicate_or_lower_milestones_do_not_present() {
        let mut progress = LoadingProgressState::standard_skirmish();

        let presents = [8, 6, 8, 12, 12]
            .into_iter()
            .filter(|value| progress.advance_progress(*value))
            .count();

        assert_eq!(presents, 2);
    }
}

#[cfg(test)]
mod pump_gate_tests {
    use super::*;
    use crate::skirmish_launch::LaunchCountry;

    /// F07 characterization: deferred loader work begins only after the first
    /// native loading frame can present. A fresh native session is blocked
    /// (composition has not produced the first renderer), and readiness alone
    /// unblocks it; the frame loop calls the pump strictly after
    /// `loading_screen_presented` inside the Loading-screen branch.
    #[test]
    fn loading_pump_starts_only_after_present() {
        let request = LoadingRequest::unverified_legacy_skirmish(
            tests::test_launch_session(LaunchCountry::America),
            tests::unverified_seed(7),
            SkirmishSettings::default(),
        );
        let mut session = LoadingSession::from_request(request);
        assert!(
            session.native.is_some(),
            "native skirmish presentation must build a native loading session"
        );
        assert!(!session.first_frame_presented);
        assert!(
            session.native_pump_blocked(),
            "the pump must refuse native sessions before the first loading frame is ready"
        );

        if let Some(native) = session.native.as_mut() {
            native.first_renderer_ready = true;
        }
        assert!(!session.native_pump_blocked());
    }
}
