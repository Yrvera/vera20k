//! Pixel-parity Skirmish shell model and layout.
//!
//! This module owns render-agnostic dialog 0x102 geometry, state, and hit
//! testing. Rendering code consumes the computed rects from the app/render
//! layers; this module does not depend on assets or wgpu.

mod layout;
mod state;

pub use layout::{
    ColorComboId, RIGHT_PANEL_WIDTH, RectPx, ShellControlId, SkirmishShellLayout, compute_layout,
};
pub use state::{
    OwnerDrawButton, SkirmishShellAction, SkirmishShellOpponent, SkirmishShellState,
    action_for_owner_draw_button, apply_action, hit_test, hit_test_owner_draw_button,
    launch_settings,
};
