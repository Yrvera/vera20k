//! App-level loading screen state and parity primitives.
//!
//! This module sits above simulation and owns loading-screen progress behavior
//! verified from gamemd.exe. It also owns the request/session boundary used by
//! the app loop before map-load phases are split into a fully pumpable job.

use crate::app::AppState;
use crate::app_init::{self, MapLoadInitial, MapLoadResult};
use crate::assets::asset_manager::AssetManager;
use crate::render::batch::{BatchRenderer, SpriteInstance};
use crate::render::gpu::GpuContext;
use crate::render::loading_screen_chrome::{
    LoadingArtVariant, LoadingScreenAtlas, LoadingScreenEntry, LoadingScreenWidth,
};
use crate::rules::house_colors::{self, HouseColorIndex};
use crate::skirmish_launch::{LaunchCountry, SkirmishLaunchSession};
use crate::ui::game_screen::GameScreen;
use crate::ui::main_menu::SkirmishSettings;
use std::path::PathBuf;

const STANDARD_SKIRMISH_PROGRESS_MAX: f64 = 100.0;
const PROGRESS_PERCENT_SCALE: f64 = 0.01;
const PERCENT_DISPLAY_SCALE: f64 = 100.0;
const FTOL_EPSILON: f64 = 0.000_001;
const BACKGROUND_DEPTH: f32 = 0.90;
/// Solid backing fill (G3) sits just behind the bar so the bar draws over it.
const SOLID_FILL_DEPTH: f32 = 0.20;
/// Player-ramp shade indices (0 brightest, 15 darkest). The filled bar uses the
/// brightest shade; the empty backing/track a darker one so progress is visible.
/// Exact gamemd +0x308/+0x30C shades are a value-precision follow-up.
const BAR_FILL_RAMP_SHADE: usize = 0;
const BACKING_RAMP_SHADE: usize = 11;
const PROGRESS_DEPTH: f32 = 0.10;
/// Side icon (G4) draws above the background, at the bar's depth.
const SIDE_ICON_DEPTH: f32 = 0.10;

/// Bar helper offset (+5) plus inset (+3) from the row origin x.
/// Verified gamemd `0x00643720` + `0x00643400`.
const BAR_X_HELPER_INSET: f32 = 5.0 + 3.0;
/// Inset (+3) added to the vertically-centered bar y.
const BAR_Y_INSET: f32 = 3.0;
/// Padding added when computing the row height (`row_h = max(...) + 4`).
const ROW_PADDING: f32 = 4.0;
/// Vertical slack added to the bar height for the centering band (`H + 6`).
const BAR_HEIGHT_BAND: f32 = 6.0;
/// Horizontal gap between the bar's right edge and the side icon (`0x15`).
const SIDE_ICON_GAP: f32 = 0x15 as f32;

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

/// Receives a native loading milestone (0..=100) at a real load-phase boundary.
///
/// Implementors apply the monotonic gate (via [`LoadingProgressState::advance_progress`])
/// and may synchronously repaint the loading screen on an advancing milestone,
/// mirroring gamemd's per-milestone synchronous `WM_PAINT`.
pub(crate) trait LoadingProgressSink {
    fn milestone(&mut self, percent: u32);
}

/// A sink that only advances the gated progress state, with no repaint. Used at
/// the pump call sites before the render-triggering sink is constructed, and as
/// the base behavior shared by all sinks.
struct GatedProgressSink<'a> {
    progress: &'a mut LoadingProgressState,
}

impl LoadingProgressSink for GatedProgressSink<'_> {
    fn milestone(&mut self, percent: u32) {
        self.progress.advance_progress(percent);
    }
}

/// Sink for the generic (non-native) map load, which has no progress bar.
struct NoopProgressSink;

impl LoadingProgressSink for NoopProgressSink {
    fn milestone(&mut self, _percent: u32) {}
}

#[derive(Clone)]
pub(crate) struct LoadingRequest {
    selected_map_file: String,
    launch: LoadingLaunch,
    presentation: LoadingPresentation,
    fallback_skirmish_settings: SkirmishSettings,
}

impl LoadingRequest {
    pub(crate) fn native_selected_skirmish(
        selected_map_file: String,
        skirmish_launch_session: SkirmishLaunchSession,
        fallback_skirmish_settings: SkirmishSettings,
    ) -> Self {
        Self {
            selected_map_file,
            launch: LoadingLaunch::Skirmish(skirmish_launch_session),
            presentation: LoadingPresentation::NativeSelectedSkirmish,
            fallback_skirmish_settings,
        }
    }

    pub(crate) fn generic_map_load(
        selected_map_file: impl Into<String>,
        fallback_skirmish_settings: SkirmishSettings,
    ) -> Self {
        Self {
            selected_map_file: selected_map_file.into(),
            launch: LoadingLaunch::Generic,
            presentation: LoadingPresentation::GenericMapLoad,
            fallback_skirmish_settings,
        }
    }

    pub(crate) fn selected_map_file(&self) -> &str {
        &self.selected_map_file
    }

    fn skirmish_launch_session(&self) -> Option<&SkirmishLaunchSession> {
        match &self.launch {
            LoadingLaunch::Skirmish(session) => Some(session),
            LoadingLaunch::Generic => None,
        }
    }
}

#[derive(Clone)]
pub(crate) enum LoadingLaunch {
    Skirmish(SkirmishLaunchSession),
    Generic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoadingPresentation {
    NativeSelectedSkirmish,
    GenericMapLoad,
}

pub(crate) struct NativeLoadingScreenState {
    pub variant: LoadingArtVariant,
    /// Local player's MP color scheme — source of the G3 solid backing fill and
    /// bar remap. Derived from the launch session, not the country variant.
    pub color_index: HouseColorIndex,
    pub progress: LoadingProgressState,
    pub atlas: Option<LoadingScreenAtlas>,
    pub first_renderer_ready: bool,
}

impl NativeLoadingScreenState {
    fn standard_skirmish(variant: LoadingArtVariant, color_index: HouseColorIndex) -> Self {
        Self {
            variant,
            color_index,
            progress: LoadingProgressState::standard_skirmish(),
            atlas: None,
            first_renderer_ready: false,
        }
    }
}

pub(crate) struct LoadingSession {
    pub request: LoadingRequest,
    pub native: Option<NativeLoadingScreenState>,
    job: LoadingJob,
    pub first_frame_presented: bool,
}

impl LoadingSession {
    fn from_request(request: LoadingRequest) -> Self {
        let native = match (&request.presentation, &request.launch) {
            (
                LoadingPresentation::NativeSelectedSkirmish,
                LoadingLaunch::Skirmish(skirmish_launch_session),
            ) => {
                let variant =
                    loading_art_variant_from_launch_country(skirmish_launch_session.local.country);
                let color_index =
                    HouseColorIndex(skirmish_launch_session.local.color_index);
                Some(NativeLoadingScreenState::standard_skirmish(
                    variant,
                    color_index,
                ))
            }
            (LoadingPresentation::GenericMapLoad, LoadingLaunch::Generic) => None,
            (LoadingPresentation::NativeSelectedSkirmish, LoadingLaunch::Generic)
            | (LoadingPresentation::GenericMapLoad, LoadingLaunch::Skirmish(_)) => {
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
    clear_loading_state(state);
    state.loading_session = Some(LoadingSession::from_request(request));
    state.screen = GameScreen::Loading;
}

pub(crate) fn loading_map_name(state: &AppState) -> Option<&str> {
    state
        .loading_session
        .as_ref()
        .map(|session| session.request.selected_map_file())
}

pub(crate) fn clear_loading_state(state: &mut AppState) {
    state.loading_session = None;
    state.loading_screen_atlas = None;
    state.loading_progress = LoadingProgressState::standard_skirmish();
}

pub(crate) fn is_native_loading_session(state: &AppState) -> bool {
    state
        .loading_session
        .as_ref()
        .is_some_and(|session| session.native.is_some())
}

pub(crate) fn pump_loading_after_present(state: &mut AppState) -> LoadingPump {
    let Some(mut session) = state.loading_session.take() else {
        return LoadingPump::Pending;
    };
    if session
        .native
        .as_ref()
        .is_some_and(|native| !native.first_renderer_ready)
    {
        return LoadingPump::Failed(anyhow::anyhow!(
            "native Skirmish loading renderer was not ready before the first loading pump"
        ));
    }

    let phase = std::mem::replace(&mut session.job.phase, LoadingJobPhase::InitialMapSelection);
    let result = match phase {
        LoadingJobPhase::InitialMapSelection => {
            let requested_map_file = session.request.selected_map_file().to_string();
            let requested_map = Some(requested_map_file.as_str());
            let initial = match take_job_assets_for_initial_load(state, &mut session) {
                Ok((ra2_dir, asset_manager)) => match session.native.as_mut() {
                    // The map-parse milestone (8) is emitted inside the loader.
                    Some(native) => {
                        let mut sink = GatedProgressSink {
                            progress: &mut native.progress,
                        };
                        app_init::load_map_initial_with_assets(
                            ra2_dir,
                            asset_manager,
                            requested_map,
                            &mut sink,
                        )
                    }
                    None => app_init::load_map_initial_with_assets(
                        ra2_dir,
                        asset_manager,
                        requested_map,
                        &mut NoopProgressSink,
                    ),
                },
                Err(err) => Err(err),
            };
            match initial {
                Ok(initial) => {
                    session.job.phase = LoadingJobPhase::RemainingLegacyLoad(Some(initial));
                    LoadingPump::Pending
                }
                Err(err) => LoadingPump::Failed(err),
            }
        }
        LoadingJobPhase::RemainingLegacyLoad(mut initial) => {
            let Some(initial) = initial.take() else {
                return LoadingPump::Failed(anyhow::anyhow!(
                    "loading job had no initial map state"
                ));
            };
            // All mid-load milestones (12..98) are emitted inside the loader; the
            // pump emits the terminal 100 once the result is ready. For the native
            // case we drive a RenderingProgressSink that synchronously repaints the
            // loading screen on each advancing milestone (gamemd's per-milestone
            // WM_PAINT), so the bar visibly sweeps instead of snapping once.
            //
            // Pre-copy the by-value pieces before borrowing so the disjoint
            // split-borrows (gpu/depth_view/batch shared, vxl_compute &mut,
            // native.progress &mut, native.atlas shared, request shared) all
            // hold simultaneously.
            let render_width = state.gpu.config.width;
            // `session.native` and `session.request` are disjoint fields, so the
            // launch-session/settings borrows below coexist with the native split.
            let load_result = match session.native.as_mut() {
                // Only repaint when the atlas is present; without it the bar
                // cannot draw, so fall back to the gate-only sink.
                Some(native) if native.atlas.is_some() => {
                    let color_index = native.color_index;
                    let atlas = native.atlas.as_ref().expect("atlas present checked above");
                    let mut sink = RenderingProgressSink {
                        gpu: &state.gpu,
                        depth_view: &state.depth_view,
                        batch: &state.batch_renderer,
                        progress: &mut native.progress,
                        atlas,
                        color_index,
                        render_width,
                    };
                    app_init::load_map_from_initial(
                        &state.gpu,
                        &state.batch_renderer,
                        initial,
                        session.request.skirmish_launch_session(),
                        &session.request.fallback_skirmish_settings,
                        state.vxl_compute.as_mut(),
                        &mut sink,
                    )
                }
                Some(native) => {
                    let mut sink = GatedProgressSink {
                        progress: &mut native.progress,
                    };
                    app_init::load_map_from_initial(
                        &state.gpu,
                        &state.batch_renderer,
                        initial,
                        session.request.skirmish_launch_session(),
                        &session.request.fallback_skirmish_settings,
                        state.vxl_compute.as_mut(),
                        &mut sink,
                    )
                }
                None => app_init::load_map_from_initial(
                    &state.gpu,
                    &state.batch_renderer,
                    initial,
                    session.request.skirmish_launch_session(),
                    &session.request.fallback_skirmish_settings,
                    state.vxl_compute.as_mut(),
                    &mut NoopProgressSink,
                ),
            };
            match load_result {
                Ok(result) => {
                    advance_native_progress(&mut session, 100);
                    LoadingPump::Finished(result)
                }
                Err(err) => LoadingPump::Failed(err),
            }
        }
    };

    if matches!(result, LoadingPump::Pending) {
        state.loading_session = Some(session);
    }
    result
}

fn ensure_job_asset_manager(state: &mut AppState) -> anyhow::Result<()> {
    let Some(session) = state.loading_session.as_ref() else {
        return Ok(());
    };
    if session.job.asset_manager.is_some() {
        return Ok(());
    }

    let ra2_dir = state
        .game_config
        .as_ref()
        .map(|config| config.paths.ra2_dir.clone())
        .ok_or_else(|| anyhow::anyhow!("missing game config for loading job assets"))?;
    let asset_manager = AssetManager::new(&ra2_dir)?;

    let Some(session) = state.loading_session.as_mut() else {
        return Ok(());
    };
    session.job.ra2_dir = Some(ra2_dir);
    session.job.asset_manager = Some(asset_manager);
    Ok(())
}

fn take_job_assets_for_initial_load(
    state: &AppState,
    session: &mut LoadingSession,
) -> anyhow::Result<(PathBuf, AssetManager)> {
    if session.job.asset_manager.is_none() {
        let ra2_dir = state
            .game_config
            .as_ref()
            .map(|config| config.paths.ra2_dir.clone())
            .ok_or_else(|| anyhow::anyhow!("missing game config for loading job assets"))?;
        let asset_manager = AssetManager::new(&ra2_dir)?;
        session.job.ra2_dir = Some(ra2_dir);
        session.job.asset_manager = Some(asset_manager);
    }

    let ra2_dir = session
        .job
        .ra2_dir
        .take()
        .ok_or_else(|| anyhow::anyhow!("loading job missing RA2 directory"))?;
    let asset_manager = session
        .job
        .asset_manager
        .take()
        .ok_or_else(|| anyhow::anyhow!("loading job missing asset manager"))?;
    Ok((ra2_dir, asset_manager))
}

fn advance_native_progress(session: &mut LoadingSession, percent: u32) {
    if let Some(native) = session.native.as_mut() {
        native.progress.advance_progress(percent);
    }
}

/// Build the verified dynamic theater ramp values that actually change progress.
pub fn theater_ramp_changed_values(values: impl IntoIterator<Item = u32>) -> Vec<u32> {
    let mut state = LoadingProgressState::standard_skirmish();
    let mut emitted = Vec::new();
    for value in [3, 8, 12].into_iter().chain(values) {
        if state.advance_progress(value) && (13..=25).contains(&value) {
            emitted.push(value);
        }
    }
    emitted
}

pub(crate) fn ensure_native_loading_atlas(state: &mut AppState) -> anyhow::Result<()> {
    let Some(variant) = selected_loading_art_variant(state) else {
        return Ok(());
    };
    if state
        .loading_session
        .as_ref()
        .and_then(|session| session.native.as_ref())
        .and_then(|native| native.atlas.as_ref())
        .is_some()
    {
        return Ok(());
    }
    ensure_job_asset_manager(state)?;
    let Some(assets) = state
        .loading_session
        .as_ref()
        .and_then(|session| session.job.asset_manager.as_ref())
    else {
        return Err(anyhow::anyhow!(
            "native loading job has no asset manager after initialization"
        ));
    };
    let width = LoadingScreenWidth::for_render_width(state.gpu.config.width);
    let atlas = crate::render::loading_screen_chrome::build_loading_screen_atlas(
        &state.gpu,
        &state.batch_renderer,
        &assets,
        variant,
        width,
    );
    if let Some(native) = state
        .loading_session
        .as_mut()
        .and_then(|session| session.native.as_mut())
    {
        native.first_renderer_ready = atlas.is_some();
        native.atlas = atlas;
    }
    if state
        .loading_session
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
    target: &wgpu::TextureView,
) -> LoadingRenderResult {
    if !is_native_loading_session(state) {
        return LoadingRenderResult::GenericFallback;
    }
    if let Err(err) = ensure_native_loading_atlas(state) {
        return LoadingRenderResult::NativeFailed(err);
    }
    let Some(native) = state
        .loading_session
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

    let instances = build_native_loading_instances(
        atlas,
        &native.progress,
        native.color_index,
        state.gpu.config.width,
    );

    state.batch_renderer.update_camera(
        &state.gpu,
        state.gpu.config.width as f32,
        state.gpu.config.height as f32,
        0.0,
        0.0,
        1.0,
    );
    let Some((buffer, count)) = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &instances)
    else {
        return LoadingRenderResult::NativeFailed(anyhow::anyhow!(
            "native Skirmish loading instances could not be uploaded"
        ));
    };

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Native Loading Screen"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app_types::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &state.depth_view,
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
        .batch_renderer
        .draw_with_buffer_passthrough(&mut pass, &atlas.texture, &buffer, count);
    LoadingRenderResult::NativeRendered
}

pub(crate) fn loading_screen_presented(state: &mut AppState) {
    let Some(session) = state.loading_session.as_mut() else {
        state.loading_progress.advance_progress(3);
        return;
    };
    session.first_frame_presented = true;
    if let Some(native) = session.native.as_mut() {
        native.progress.advance_progress(3);
    }
}

fn selected_loading_art_variant(state: &AppState) -> Option<LoadingArtVariant> {
    if !matches!(state.screen, GameScreen::Loading) {
        return None;
    }
    state
        .loading_session
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

/// Row-draw origin (the NW corner the bar helper offsets from), before any
/// helper/inset additions. Verified gamemd standard-skirmish ProgressClass
/// position: (0x10, 0x141) at >=800px, (0x0C, 0x100) below.
fn standard_skirmish_row_origin(render_width: u32) -> [f32; 2] {
    if render_width >= 800 {
        [0x10 as f32, 0x141 as f32]
    } else {
        [0x0C as f32, 0x100 as f32]
    }
}

/// Row height = max(side_icon_h, H + 6, font_h) + 4.
///
/// `font_h` (the loading font height) is not readily available at this layer, so
/// `H` stands in for it; since `H + 6 > H`, the `H + 6` term dominates whenever
/// the font is no taller than the bar. Exact font-height precision is a small
/// vertical follow-up.
fn standard_skirmish_row_height(bar_height: f32, side_icon_height: f32) -> f32 {
    let font_h = bar_height; // stand-in for the loading font height (follow-up).
    side_icon_height
        .max(bar_height + BAR_HEIGHT_BAND)
        .max(font_h)
        + ROW_PADDING
}

/// Bar fill origin: x = base_x + 5 + 3 (helper + inset); y = base_y +
/// ((row_h - (H + 6)) / 2) + 3 (vertical row-centering + inset).
fn standard_skirmish_progress_position(
    render_width: u32,
    bar_height: f32,
    row_height: f32,
) -> [f32; 2] {
    let [base_x, base_y] = standard_skirmish_row_origin(render_width);
    let centered_y = base_y + (row_height - (bar_height + BAR_HEIGHT_BAND)) / 2.0 + BAR_Y_INSET;
    [base_x + BAR_X_HELPER_INSET, centered_y]
}

/// Side-icon origin: x = base_x + W + 0x15; y vertically centered against row_h.
fn standard_skirmish_side_icon_position(
    render_width: u32,
    bar_width: f32,
    row_height: f32,
    icon_height: f32,
) -> [f32; 2] {
    let [base_x, base_y] = standard_skirmish_row_origin(render_width);
    let icon_x = base_x + bar_width + SIDE_ICON_GAP;
    let icon_y = base_y + (row_height - icon_height) / 2.0;
    [icon_x, icon_y]
}

/// A shade of the player's color scheme ramp, normalized to 0..1.
/// Ramp index 0 is the brightest shade, 15 the darkest.
fn player_scheme_shade_rgb(color_index: HouseColorIndex, shade: usize) -> [f32; 3] {
    let ramp = house_colors::house_color_ramp(color_index);
    let c = ramp[shade.min(ramp.len() - 1)];
    [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0]
}

/// Empty-bar backing color (gamemd ColorScheme+0x308). Drawn darker than the
/// fill so the filled span reads as distinct — gamemd's track sits darker than
/// the remapped bar. Exact +0x308 shade is a value-precision follow-up; a
/// darker ramp shade is used until the precise HSV is matched.
fn player_scheme_backing_rgb(color_index: HouseColorIndex) -> [f32; 3] {
    player_scheme_shade_rgb(color_index, BACKING_RAMP_SHADE)
}

/// Filled-bar color (gamemd ColorScheme+0x30C remap, approximated as a tint on
/// the PROGBARM frame). Uses the brightest ramp shade so the fill reads brighter
/// than the backing. A true palette remap of the frame is a deferred follow-up.
fn player_scheme_bar_rgb(color_index: HouseColorIndex) -> [f32; 3] {
    player_scheme_shade_rgb(color_index, BAR_FILL_RAMP_SHADE)
}

/// Build the full native loading-screen instance list (background, solid backing
/// fill, clipped progress bar, side icon) shared by the per-frame render path and
/// the synchronous-repaint sink.
fn build_native_loading_instances(
    atlas: &LoadingScreenAtlas,
    progress: &LoadingProgressState,
    color_index: HouseColorIndex,
    render_width: u32,
) -> Vec<SpriteInstance> {
    let mut instances = Vec::with_capacity(4);
    push_entry(
        &mut instances,
        atlas.background,
        [0.0, 0.0],
        BACKGROUND_DEPTH,
    );

    let bar_w = atlas.progress_frame0.pixel_size[0];
    let bar_h = atlas.progress_frame0.pixel_size[1];
    let side_icon_h = atlas
        .side_icon
        .map(|icon| icon.pixel_size[1])
        .unwrap_or(0.0);
    let row_h = standard_skirmish_row_height(bar_h, side_icon_h);
    let bar_origin = standard_skirmish_progress_position(render_width, bar_h, row_h);

    // G3: solid backing fill — full bar frame rect (W x H), tinted with the
    // player color scheme, drawn BEFORE the clipped bar so the bar covers it.
    push_entry_tinted(
        &mut instances,
        atlas.solid_texel,
        bar_origin,
        [bar_w, bar_h],
        SOLID_FILL_DEPTH,
        player_scheme_backing_rgb(color_index),
    );

    // G2: clipped progress span. The bar frame is remapped to the player color
    // via tint as a follow-up; full palette remap of the baked frame is deferred.
    let progress_width = progress.fill_width_gamemd_ftol_positive_domain(bar_w as u32);
    if progress_width > 0 {
        push_entry_tinted(
            &mut instances,
            atlas.progress_frame0,
            bar_origin,
            [progress_width as f32, bar_h],
            PROGRESS_DEPTH,
            player_scheme_bar_rgb(color_index),
        );
    }

    // G4: country side icon to the right of the bar, vertically centered. No
    // label is drawn (skirmish text pointer is 0).
    if let Some(icon) = atlas.side_icon {
        let icon_pos = standard_skirmish_side_icon_position(
            render_width,
            bar_w,
            row_h,
            icon.pixel_size[1],
        );
        push_entry(&mut instances, icon, icon_pos, SIDE_ICON_DEPTH);
    }

    instances
}

/// Acquire a surface frame, render the native loading screen, and present it.
///
/// Used by the synchronous-repaint sink to mirror gamemd's per-milestone
/// `WM_PAINT`. All wgpu ops take `&self`, so only shared references are needed.
/// Returns an error on acquire/upload failure; the caller treats it as non-fatal.
fn present_native_loading(
    gpu: &GpuContext,
    depth_view: &wgpu::TextureView,
    batch: &BatchRenderer,
    atlas: &LoadingScreenAtlas,
    progress: &LoadingProgressState,
    color_index: HouseColorIndex,
    render_width: u32,
) -> anyhow::Result<()> {
    let output = gpu
        .surface
        .get_current_texture()
        .map_err(|e| anyhow::anyhow!("loading repaint surface texture: {e}"))?;
    let view = output.texture.create_view(&Default::default());
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Native Loading Repaint"),
        });

    let instances = build_native_loading_instances(atlas, progress, color_index, render_width);
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

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Native Loading Repaint"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(crate::app_types::CLEAR_COLOR),
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
    }

    gpu.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    Ok(())
}

/// Sink that synchronously re-renders and presents the loading screen on each
/// advancing milestone, mirroring gamemd's per-milestone `WM_PAINT`. Render or
/// surface-acquire failures are logged and swallowed so they never abort the
/// map load.
struct RenderingProgressSink<'a> {
    gpu: &'a GpuContext,
    depth_view: &'a wgpu::TextureView,
    batch: &'a BatchRenderer,
    progress: &'a mut LoadingProgressState,
    atlas: &'a LoadingScreenAtlas,
    color_index: HouseColorIndex,
    render_width: u32,
}

impl LoadingProgressSink for RenderingProgressSink<'_> {
    fn milestone(&mut self, percent: u32) {
        if self.progress.advance_progress(percent) {
            if let Err(err) = present_native_loading(
                self.gpu,
                self.depth_view,
                self.batch,
                self.atlas,
                self.progress,
                self.color_index,
                self.render_width,
            ) {
                log::warn!("Native loading repaint at milestone {percent} failed: {err:#}");
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
    push_entry_sized(out, entry, position, entry.pixel_size, depth);
}

fn push_entry_sized(
    out: &mut Vec<SpriteInstance>,
    entry: LoadingScreenEntry,
    position: [f32; 2],
    size: [f32; 2],
    depth: f32,
) {
    push_entry_tinted(out, entry, position, size, depth, [1.0, 1.0, 1.0]);
}

fn push_entry_tinted(
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
        uv_size: [
            entry.uv_size[0] * (size[0] / entry.pixel_size[0]).clamp(0.0, 1.0),
            entry.uv_size[1],
        ],
        depth,
        tint,
        alpha: 1.0,
        house_color_idx: 0,
        fx_flags: 0,
        fx_params: [0.0; 4],
        ic_tint: [0.0; 4],
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

    fn test_launch_session(country: LaunchCountry) -> SkirmishLaunchSession {
        SkirmishLaunchSession {
            mode: SkirmishLaunchMode {
                id: 1,
                ui_name_key: "GUI:Battle".to_string(),
                tooltip_key: "STT:ModeBattle".to_string(),
                override_file: "battlemd.ini".to_string(),
                map_filter: "standard".to_string(),
                random_maps_allowed: false,
                allies_allowed: true,
                must_ally: false,
            },
            selected_map_file: Some("mp01t4.map".to_string()),
            player_name: "Player".to_string(),
            local: SkirmishLocalSlot {
                country,
                country_random: false,
                color_index: 0,
                start_position: LaunchStartPosition::Position(0),
                team: LaunchTeam::None,
            },
            opponents: vec![SkirmishAiSlot {
                country: LaunchCountry::Russia,
                country_random: false,
                color_index: 1,
                start_position: LaunchStartPosition::Position(1),
                team: LaunchTeam::None,
                difficulty: AiDifficulty::Easy,
            }],
            options: SkirmishLaunchOptions::default(),
        }
    }

    #[test]
    fn loading_side_comes_from_first_launch_node_country() {
        let session = LoadingSession::from_request(LoadingRequest::native_selected_skirmish(
            "mp01t4.map".to_string(),
            test_launch_session(LaunchCountry::Korea),
            SkirmishSettings::default(),
        ));

        assert_eq!(
            session.native.as_ref().map(|native| native.variant),
            Some(LoadingArtVariant::Alliance)
        );
    }

    #[test]
    fn loading_session_preserves_selected_map_filename() {
        let session = LoadingSession::from_request(LoadingRequest::native_selected_skirmish(
            "mp02t2.map".to_string(),
            test_launch_session(LaunchCountry::Yuri),
            SkirmishSettings::default(),
        ));

        assert_eq!(session.request.selected_map_file(), "mp02t2.map");
        assert_eq!(
            session
                .request
                .skirmish_launch_session()
                .and_then(|launch| launch.selected_map_file.as_deref()),
            Some("mp01t4.map")
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
        let session = LoadingSession::from_request(LoadingRequest::native_selected_skirmish(
            "mp01t4.map".to_string(),
            test_launch_session(LaunchCountry::America),
            SkirmishSettings::default(),
        ));

        assert!(matches!(
            session.job.phase,
            LoadingJobPhase::InitialMapSelection
        ));
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
        let loader_emits: [u32; 25] = [
            8, 12, 30, 31, 35, 45, 50, 55, 58, 60, 63, 65, 67, 68, 69, 70, 72, 74, 76, 78, 82, 86,
            93, 96, 98,
        ];
        let mut sink = RecordingProgressSink::standard();
        sink.milestone(3); // present handoff
        for v in loader_emits {
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
        let ramp = [13, 15, 18, 22, 25];
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
    fn loading_progress_theater_ramp_emits_only_changed_dynamic_values() {
        let emitted = theater_ramp_changed_values([13, 13, 14, 14, 18, 25, 25]);

        assert_eq!(emitted, vec![13, 14, 18, 25]);
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
    fn bar_origin_uses_helper_offset_and_row_centering() {
        // Helper +5 plus inset +3 => x = base_x + 8 at both widths.
        let bar_h = 14.0;
        let side_icon_h = 0.0;
        let row_h = standard_skirmish_row_height(bar_h, side_icon_h);

        let pos_800 = standard_skirmish_progress_position(800, bar_h, row_h);
        assert_eq!(pos_800[0], 0x10 as f32 + 8.0);

        let pos_640 = standard_skirmish_progress_position(640, bar_h, row_h);
        assert_eq!(pos_640[0], 0x0C as f32 + 8.0);

        // Vertical centering: y = base_y + (row_h - (H + 6)) / 2 + 3.
        // With no side icon and font_h == H, row_h = (H + 6) + 4, so the
        // centering term is (row_h - (H + 6)) / 2 = 2; total inset = 2 + 3 = 5.
        let centering = (row_h - (bar_h + BAR_HEIGHT_BAND)) / 2.0;
        assert_eq!(centering, 2.0);
        assert_eq!(pos_800[1], 0x141 as f32 + centering + BAR_Y_INSET);
        assert_eq!(pos_640[1], 0x100 as f32 + centering + BAR_Y_INSET);
    }

    #[test]
    fn row_height_is_dominated_by_tallest_of_band_icon_font_plus_padding() {
        // Tall icon dominates.
        assert_eq!(standard_skirmish_row_height(14.0, 40.0), 40.0 + ROW_PADDING);
        // Otherwise the H + 6 band dominates (font_h == H stand-in).
        assert_eq!(
            standard_skirmish_row_height(14.0, 0.0),
            14.0 + BAR_HEIGHT_BAND + ROW_PADDING
        );
    }

    #[test]
    fn side_icon_sits_one_gap_right_of_bar_and_is_vertically_centered() {
        let bar_w = 326.0;
        let row_h = 60.0;
        let icon_h = 40.0;

        let pos_800 = standard_skirmish_side_icon_position(800, bar_w, row_h, icon_h);
        assert_eq!(pos_800[0], 0x10 as f32 + bar_w + 0x15 as f32);
        assert_eq!(pos_800[1], 0x141 as f32 + (row_h - icon_h) / 2.0);

        let pos_640 = standard_skirmish_side_icon_position(640, bar_w, row_h, icon_h);
        assert_eq!(pos_640[0], 0x0C as f32 + bar_w + 0x15 as f32);
        assert_eq!(pos_640[1], 0x100 as f32 + (row_h - icon_h) / 2.0);
    }

    #[test]
    fn player_scheme_colors_are_normalized_and_backing_is_darker_than_bar() {
        let bar = player_scheme_bar_rgb(HouseColorIndex(0));
        let backing = player_scheme_backing_rgb(HouseColorIndex(0));
        assert!(bar.iter().all(|c| (0.0..=1.0).contains(c)));
        assert!(backing.iter().all(|c| (0.0..=1.0).contains(c)));
        // The empty-bar backing must read darker than the filled bar so progress
        // is visible (sum of channels as a brightness proxy).
        let lum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(lum(backing) < lum(bar));
        // DarkBlue scheme's bar shade must be more blue than red.
        let blue = player_scheme_bar_rgb(HouseColorIndex(1));
        assert!(blue[2] > blue[0]);
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
