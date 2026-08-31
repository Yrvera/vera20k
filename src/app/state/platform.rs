//! Process-local platform lifecycle and frame-pacing state.

use std::sync::Arc;
use std::time::Instant;

use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::app::match_runtime::frame_pacer::LocalFramePacer;

/// Window lifecycle and wall-clock pacing owned by the platform layer.
///
/// This state is process-local: it is never serialized, hashed, or read by the
/// deterministic simulation.
pub(crate) struct PlatformState {
    pub(crate) window: Arc<Window>,
    /// Whether this application currently owns the foreground.
    ///
    /// gamemd tracks the same edge-triggered byte from `WM_ACTIVATEAPP` and
    /// parks its main tick in a sleep-and-network-only loop while it is clear:
    /// the frame counter, input, AI, map logic and per-tick update all stop.
    /// Only the message pump keeps running. Starts true — a window that never
    /// reports an activation edge must keep running.
    pub(crate) window_active: bool,
    /// Whether the window has no visible surface — minimised, or occluded on
    /// the platforms that report occlusion.
    ///
    /// Windows never emits `WindowEvent::Occluded` (winit only raises it from
    /// the iOS, X11, macOS and Web backends); a minimise arrives there as a
    /// zero-sized `Resized` instead. Both signals feed this flag, so the redraw
    /// loop parks on every platform. Presentation-only.
    pub(crate) window_hidden: bool,
    /// Monotonic epoch used only by the app-local gameplay-frame pacer.
    pub(crate) frame_pacer_epoch: Instant,
    /// Local wall-clock admission state. Never serialized or read by the sim.
    pub(crate) frame_pacer: LocalFramePacer,
    /// Loaded GameConfig — missing config.toml falls back to the executable
    /// root; `None` only when config loading or executable-root discovery
    /// fails. Set once at process start from `GameConfig::load()`; not
    /// mutated afterwards.
    pub(crate) game_config: Option<crate::util::config::GameConfig>,
    /// Effective shell client size for this process. Interactive launches use
    /// the resolved retail profile pair; sealed captures retain their explicit
    /// dimensions as the higher-priority automation projection.
    pub(crate) shell_client_size: PhysicalSize<u32>,
}

impl PlatformState {
    pub(crate) fn new(
        window: Arc<Window>,
        game_config: Option<crate::util::config::GameConfig>,
        shell_client_size: PhysicalSize<u32>,
    ) -> Self {
        Self {
            window,
            window_active: true,
            window_hidden: false,
            frame_pacer_epoch: Instant::now(),
            frame_pacer: LocalFramePacer::new(),
            game_config,
            shell_client_size,
        }
    }
}
