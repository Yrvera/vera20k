//! Pixel-parity Skirmish shell model and layout.
//!
//! This module owns render-agnostic dialog 0x102 geometry, state, and hit
//! testing. Rendering code consumes the computed rects from the app/render
//! layers; this module does not depend on assets or wgpu.

mod layout;
mod state;

pub use layout::{
    compute_layout, ColorComboId, RectPx, ShellControlId, SkirmishShellLayout,
    SkirmishTrackbarRects, RIGHT_PANEL_WIDTH,
};
pub use state::{
    action_for_owner_draw_button, apply_action, hit_test, hit_test_owner_draw_button,
    launch_settings, OwnerDrawButton, SkirmishShellAction, SkirmishShellOpponent,
    SkirmishShellState,
};
