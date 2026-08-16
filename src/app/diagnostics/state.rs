//! Process diagnostics owner (F12 `DiagnosticsState`): debug overlay toggles,
//! the frame stepper, the parity digest sink, and dev-overlay bookkeeping.
//!
//! Match-lifetime diagnostic replay lives in `app::match_diagnostics`; this
//! owner is process-scoped tooling state.

pub(crate) struct DiagnosticsState {
    /// One-shot: advance a single sim tick while paused (dev overlay).
    pub(crate) debug_frame_step_requested: bool,
    /// PathGrid walkability overlay toggle (P / F9).
    pub(crate) debug_show_pathgrid: bool,
    /// Per-overlay SpeedType override for the terrain-cost overlay; `None`
    /// derives it from the selected unit.
    pub(crate) debug_terrain_cost_speed_type:
        Option<crate::rules::locomotor_type::SpeedType>,
    /// Cell grid overlay toggle.
    pub(crate) debug_show_cell_grid: bool,
    /// Heightmap overlay toggle.
    pub(crate) debug_show_heightmap: bool,
    /// Debug unit inspector (X): mirrors sim-side per-entity event logging.
    pub(crate) debug_unit_inspector: bool,
    /// Optional per-tick parity digest capture (diagnostics; never perturbs
    /// the run being measured).
    pub(crate) parity_digest_sink: Option<crate::sim::parity_digest::ParityDigestSink>,
    /// Save-name text field in the dev overlay.
    pub(crate) dev_overlay_save_name: String,
    /// Rolling frame-time statistics for the dev overlay.
    pub(crate) frame_timer: crate::app::diagnostics::dev_overlay::FrameTimer,
}
