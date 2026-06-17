//! Client-side in-game Options (0xBBB) state: the six [Options] values plus the
//! transient interaction state the overlay needs. App/ui-level only — never sim/.
//!
//! Values are stored in gamemd's INTERNAL representation: GameSpeed/ScrollRate are
//! 0..6 with 0 = fastest (the dialog slider position is `6 - value`); DetailLevel
//! is 0..2 direct. Defaults match gamemd OptionsClass::SetDefaults.

use crate::ui::skirmish_shell::{RectPx, trackbar_active_width};

/// GameSpeed/ScrollRate internal range (0 = fastest .. 6 = slowest).
pub const OPTIONS_SPEED_MIN: u32 = 0;
pub const OPTIONS_SPEED_MAX: u32 = 6;
/// DetailLevel range (0 = low .. 2 = high), direct (not inverted).
pub const OPTIONS_DETAIL_MAX: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameOptionsState {
    /// Internal GameSpeed 0..6 (0 = fastest). Slider position is `6 - game_speed`.
    pub game_speed: u32,
    /// Internal ScrollRate 0..6 (0 = fastest). Slider position is `6 - scroll_rate`.
    pub scroll_rate: u32,
    /// DetailLevel 0..2 (direct). Hidden in 0xBBB but carried for persistence.
    pub detail_level: u32,
    pub unit_action_lines: bool,
    pub show_hidden: bool,
    pub tooltips: bool,
    /// Transient: which owner-draw button is held (for the pressed frame).
    pub pressed_button: Option<u16>,
    /// Transient: control id of the slider currently being dragged, if any.
    pub dragging_slider: Option<u16>,
    /// Transient per-slider "dragged since this open" — gates the label swap from
    /// the template default ("Faster") to the position CSF text (gamemd quirk).
    pub game_speed_label_dragged: bool,
    pub scroll_rate_label_dragged: bool,
}

impl Default for InGameOptionsState {
    fn default() -> Self {
        // gamemd OptionsClass::SetDefaults: GameSpeed 3, ScrollRate 3,
        // DetailLevel 2, UnitActionLines 1, ShowHidden 0, ToolTips 1.
        Self {
            game_speed: 3,
            scroll_rate: 3,
            detail_level: 2,
            unit_action_lines: true,
            show_hidden: false,
            tooltips: true,
            pressed_button: None,
            dragging_slider: None,
            game_speed_label_dragged: false,
            scroll_rate_label_dragged: false,
        }
    }
}

impl InGameOptionsState {
    /// Reset the transient interaction flags when the overlay (re)opens — gamemd
    /// recreates the dialog, so the label-dragged quirk resets each open.
    pub fn on_open(&mut self) {
        self.pressed_button = None;
        self.dragging_slider = None;
        self.game_speed_label_dragged = false;
        self.scroll_rate_label_dragged = false;
    }
}

/// Slider position (0..6) shown for an internal speed value: `6 - value`.
/// GameSpeed/ScrollRate only (DetailLevel is direct).
pub fn speed_slider_pos(internal: u32) -> u32 {
    OPTIONS_SPEED_MAX - internal.min(OPTIONS_SPEED_MAX)
}

/// Internal speed value from a slider position (0..6): `6 - pos`.
pub fn speed_from_slider_pos(pos: u32) -> u32 {
    OPTIONS_SPEED_MAX - pos.min(OPTIONS_SPEED_MAX)
}

/// Quantized slider POSITION (0..max) for a mouse x over a laid trackbar `rect`.
/// Inverse of `trackbar_pixel_offset` (which maps value -> pixel as
/// `(value-min)*active_width/span`, thumb drawn at `rect.x + 1 + offset`).
pub fn trackbar_pos_from_mouse_x(mouse_x: i32, min: i32, max: i32, rect: RectPx) -> i32 {
    let active_width = trackbar_active_width(rect).max(1);
    let span = (max - min).max(1);
    let rel = (mouse_x - (rect.x + 1)).clamp(0, active_width);
    // round to nearest stop
    min + ((rel * span * 2 + active_width) / (active_width * 2)).clamp(0, span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_gamemd_setdefaults() {
        let s = InGameOptionsState::default();
        assert_eq!((s.game_speed, s.scroll_rate, s.detail_level), (3, 3, 2));
        assert!(s.unit_action_lines && !s.show_hidden && s.tooltips);
    }

    #[test]
    fn slider_pos_inverts_speed_round_trip() {
        for v in 0..=OPTIONS_SPEED_MAX {
            assert_eq!(speed_from_slider_pos(speed_slider_pos(v)), v);
        }
        // Internal 3 (default) sits at the midpoint slider position 3.
        assert_eq!(speed_slider_pos(3), 3);
        // Internal 0 (fastest) is the far slider position 6.
        assert_eq!(speed_slider_pos(0), 6);
    }

    #[test]
    fn on_open_clears_transient_flags() {
        let mut s = InGameOptionsState {
            game_speed_label_dragged: true,
            pressed_button: Some(0x686),
            ..Default::default()
        };
        s.on_open();
        assert!(!s.game_speed_label_dragged && s.pressed_button.is_none());
    }

    #[test]
    fn mouse_x_maps_back_to_slider_stop() {
        use crate::ui::skirmish_shell::{RectPx, trackbar_pixel_offset};
        let rect = RectPx::new(216, 163, 192, 21); // GameSpeed laid rect @ 800x600
        for pos in 0..=6 {
            let px = trackbar_pixel_offset(pos, 0, 6, 1, rect);
            let thumb_center_x = rect.x + 1 + px;
            assert_eq!(
                trackbar_pos_from_mouse_x(thumb_center_x, 0, 6, rect),
                pos,
                "pos {pos}"
            );
        }
        // Clamps past the ends.
        assert_eq!(trackbar_pos_from_mouse_x(rect.x - 50, 0, 6, rect), 0);
        assert_eq!(trackbar_pos_from_mouse_x(rect.x + 9999, 0, 6, rect), 6);
    }
}
