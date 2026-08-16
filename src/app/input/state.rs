//! Match input owner (F12 `MatchInputState`): camera, zoom, cursor, held
//! keys, hotkey bindings, and the TypeSelect input machine.
//!
//! These are the app-side input/viewport facts for the running match (and the
//! shells that reuse the same cursor/key plumbing). Sim-authoritative input —
//! queued commands — lives in the simulation, never here.

use std::collections::HashSet;
use winit::keyboard::{KeyCode, ModifiersState};

pub(crate) struct MatchInputState {
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
    pub(crate) edge_scroll: crate::app::input::camera::EdgeScrollState,
    /// Tactical mouse capture and right-drag pan anchor.
    pub(crate) tactical_mouse: crate::app::input::camera::TacticalMouseState,
    /// The four camera bookmarks (View1..4 / SetView1..4).
    pub(crate) view_bookmarks: crate::app::input::camera::ViewBookmarks,
    pub(crate) cursor_x: f32,
    pub(crate) cursor_y: f32,
    pub(crate) keys_held: HashSet<KeyCode>,
    pub(crate) hotkey_bindings: crate::app::input::hotkeys::HotkeyBindings,
    pub(crate) hotkey_modifiers: ModifiersState,
    /// Hybrid held/tap state for the retail TypeSelect command.
    pub(crate) type_select: crate::app::types::TypeSelectInputState,
    /// One-shot Shift+S request, consumed at the next render submission.
    pub(crate) retail_screenshot_requested: bool,
}
