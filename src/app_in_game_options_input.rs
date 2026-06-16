//! Paused-overlay mouse routing for the active in-game Options (`0xBBB`) dialog.
//!
//! Part of the app layer. While `state.paused`, `handle_mouse_input` routes here
//! BEFORE the gadget/tactical dispatch and this consumes the click so it never
//! reaches the tactical viewport (no unit orders behind the overlay). Interaction
//! changes only the visual/stored state — slider thumb + stored value, checkbox
//! check, pressed button frame, and the drag-gated value-label flag. The downstream
//! EFFECTS (sim cadence, target-line gate, INI persist) apply on close only, in
//! `app_options_persist::in_game_options_close` (KD-8).
//!
//! Hit-testing uses the `InGameOptionsAnchor` the overlay render pass cached on
//! `AppState` (KD-6) — the sidebar-anchored Back/Sound/Keyboard button Y is only
//! known at render time, so recomputing the anchor here would be wrong; the cached
//! anchor also guarantees the hit rects exactly match what was drawn.

use winit::event::MouseButton;

use crate::app::AppState;
use crate::ui::shell::descriptor::ControlKind;
use crate::ui::shell::in_game_options::{build_in_game_options_descriptor, control};
use crate::ui::shell::in_game_options_state::{
    InGameOptionsState, OPTIONS_SPEED_MAX, OPTIONS_SPEED_MIN, speed_from_slider_pos,
    trackbar_pos_from_mouse_x,
};
use crate::ui::shell::layout::{LaidOutControl, layout_pass_in_game_options};

/// Which visible `0xBBB` control (if any) is under the cursor. For a trackbar the
/// quantized slider position (0..6) the cursor x maps to is carried alongside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionsHit {
    Button(u16),
    Slider(u16, i32),
    Checkbox(u16),
    None,
}

/// Pure hit-test: the visible interactive control under `cursor`, if any. `laid`
/// must be the `layout_pass_in_game_options` output for the live descriptor (same
/// descriptor order), so control KINDS are recovered by zipping a fresh descriptor.
/// Hidden controls (the VisualDetails triplet) carry `visible: false` and are
/// skipped; statics are not interactive (the cursor falls through them).
pub(crate) fn in_game_options_hit(laid: &[LaidOutControl], cursor: (i32, i32)) -> OptionsHit {
    let (cx, cy) = cursor;
    let desc = build_in_game_options_descriptor();
    for (c, l) in desc.controls.iter().zip(laid.iter()) {
        if !c.visible || !l.rect.contains(cx, cy) {
            continue;
        }
        match c.kind {
            ControlKind::Button => return OptionsHit::Button(c.id),
            ControlKind::Trackbar => {
                let pos = trackbar_pos_from_mouse_x(
                    cx,
                    OPTIONS_SPEED_MIN as i32,
                    OPTIONS_SPEED_MAX as i32,
                    l.rect,
                );
                return OptionsHit::Slider(c.id, pos);
            }
            ControlKind::Checkbox => return OptionsHit::Checkbox(c.id),
            // Statics are not interactive; keep scanning the remaining controls.
            _ => continue,
        }
    }
    OptionsHit::None
}

/// Left-button press/release routing for the paused Options overlay. Non-left
/// buttons are swallowed (no overlay action). No-ops until the overlay has rendered
/// at least once (the anchor cache is `None`).
pub(crate) fn in_game_options_mouse(state: &mut AppState, button: MouseButton, pressed: bool) {
    if button != MouseButton::Left {
        return; // consume non-left while paused; nothing to do
    }
    let Some(anchor) = state.in_game_options_anchor else {
        return; // overlay not rendered yet -> nothing to hit-test
    };
    let screen_w = state.render_width() as i32;
    let screen_h = state.render_height() as i32;
    let desc = build_in_game_options_descriptor();
    let laid = layout_pass_in_game_options(&desc, screen_w, screen_h, anchor);
    let cx = state.cursor_x.round() as i32;
    let cy = state.cursor_y.round() as i32;

    if pressed {
        match in_game_options_hit(&laid, (cx, cy)) {
            OptionsHit::Button(id) => {
                // Buttons (Back/Keyboard/Sound) paint pressed on hold; the action
                // fires on release over the same rect (Back) or no-ops (KD-7 stubs).
                state.in_game_options.pressed_button = Some(id);
            }
            OptionsHit::Slider(id, pos) => begin_slider_press(state, id, pos),
            // Checkbox toggles on press (BS_AUTOCHECKBOX) — visual/stored only (KD-8).
            OptionsHit::Checkbox(id) => toggle_checkbox(&mut state.in_game_options, id),
            OptionsHit::None => {}
        }
    } else {
        // Release: a Back press that ends still over the Back rect closes + applies
        // + persists. Keyboard/Sound release: clear pressed, no action (KD-7).
        let over_back = state.in_game_options.pressed_button == Some(control::BACK)
            && back_rect_contains(&laid, cx, cy);
        state.in_game_options.pressed_button = None;
        state.in_game_options.dragging_slider = None;
        if over_back {
            crate::app_options_persist::in_game_options_close(state);
        }
    }
}

/// Live slider drag while paused: re-quantize the dragged slider's value from the
/// current cursor x against the cached anchor's laid rect. Visual/stored only — the
/// cadence and other effects are deferred to close (KD-8). No-op when no slider is
/// being dragged (so a paused move with no active drag is simply swallowed upstream).
pub(crate) fn in_game_options_drag(state: &mut AppState) {
    let Some(id) = state.in_game_options.dragging_slider else {
        return;
    };
    let Some(anchor) = state.in_game_options_anchor else {
        return;
    };
    let screen_w = state.render_width() as i32;
    let screen_h = state.render_height() as i32;
    let desc = build_in_game_options_descriptor();
    let laid = layout_pass_in_game_options(&desc, screen_w, screen_h, anchor);
    let Some(rect) = laid.iter().find(|l| l.id == id).map(|l| l.rect) else {
        return;
    };
    let cx = state.cursor_x.round() as i32;
    let pos =
        trackbar_pos_from_mouse_x(cx, OPTIONS_SPEED_MIN as i32, OPTIONS_SPEED_MAX as i32, rect);
    store_slider_value(&mut state.in_game_options, id, pos);
}

/// Begin a slider drag on press: mark it dragging, set the per-slider drag flag (so
/// the value label swaps from the template "Faster" to the position word), and
/// store the pressed-at value. Visual/stored only (KD-8).
fn begin_slider_press(state: &mut AppState, id: u16, pos: i32) {
    state.in_game_options.dragging_slider = Some(id);
    match id {
        control::GAME_SPEED => state.in_game_options.game_speed_label_dragged = true,
        control::SCROLL_RATE => state.in_game_options.scroll_rate_label_dragged = true,
        _ => {}
    }
    store_slider_value(&mut state.in_game_options, id, pos);
}

/// Store a slider's new internal value from a slider position (`6 - pos`). Render
/// reads the stored value to draw the thumb + value label; no effect applies here.
fn store_slider_value(opts: &mut InGameOptionsState, id: u16, pos: i32) {
    let value = speed_from_slider_pos(pos.max(0) as u32);
    match id {
        control::GAME_SPEED => opts.game_speed = value,
        control::SCROLL_RATE => opts.scroll_rate = value,
        _ => {}
    }
}

/// Toggle a checkbox's stored bool (the rendered check state only — the downstream
/// effect applies on close, KD-8).
fn toggle_checkbox(opts: &mut InGameOptionsState, id: u16) {
    match id {
        control::TARGET_LINES => opts.unit_action_lines = !opts.unit_action_lines,
        control::SHOW_HIDDEN => opts.show_hidden = !opts.show_hidden,
        control::TOOLTIPS => opts.tooltips = !opts.tooltips,
        _ => {}
    }
}

fn back_rect_contains(laid: &[LaidOutControl], cx: i32, cy: i32) -> bool {
    laid.iter()
        .find(|l| l.id == control::BACK)
        .is_some_and(|l| l.rect.contains(cx, cy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::layout::InGameOptionsAnchor;

    fn test_laid() -> Vec<LaidOutControl> {
        let desc = build_in_game_options_descriptor();
        let anchor = InGameOptionsAnchor {
            button_canvas_w: 125,
            button_canvas_h: 25,
            button_stack_top_y: 200,
            back_button_y: 540,
        };
        layout_pass_in_game_options(&desc, 800, 600, anchor)
    }

    #[test]
    fn hit_test_routes_visible_controls() {
        let laid = test_laid();
        let rect_of = |id: u16| laid.iter().find(|l| l.id == id).unwrap().rect;

        // Back button center -> Button(BACK).
        let back = rect_of(control::BACK);
        assert_eq!(
            in_game_options_hit(&laid, (back.x + back.w / 2, back.y + back.h / 2)),
            OptionsHit::Button(control::BACK)
        );

        // GameSpeed rail far-right edge (inside the rect) -> Slider at the max stop.
        let gs = rect_of(control::GAME_SPEED);
        assert_eq!(
            in_game_options_hit(&laid, (gs.x + gs.w - 1, gs.y + gs.h / 2)),
            OptionsHit::Slider(control::GAME_SPEED, 6)
        );

        // TargetLines checkbox center -> Checkbox(TARGET_LINES).
        let tl = rect_of(control::TARGET_LINES);
        assert_eq!(
            in_game_options_hit(&laid, (tl.x + tl.w / 2, tl.y + tl.h / 2)),
            OptionsHit::Checkbox(control::TARGET_LINES)
        );

        // Empty corner -> None.
        assert_eq!(in_game_options_hit(&laid, (2, 2)), OptionsHit::None);
    }

    #[test]
    fn hidden_visualdetails_trackbar_is_not_hittable() {
        // The VisualDetails trackbar carries visible:false; a cursor over its laid
        // rect must NOT register as a slider hit.
        let laid = test_laid();
        let vd = laid
            .iter()
            .find(|l| l.id == control::VISUAL_DETAILS)
            .unwrap()
            .rect;
        assert_eq!(
            in_game_options_hit(&laid, (vd.x + vd.w / 2, vd.y + vd.h / 2)),
            OptionsHit::None
        );
    }

    #[test]
    fn store_slider_value_inverts_position() {
        let mut opts = InGameOptionsState::default();
        // Far-right slider position 6 -> fastest internal 0.
        store_slider_value(&mut opts, control::GAME_SPEED, 6);
        assert_eq!(opts.game_speed, 0);
        // Far-left position 0 -> slowest internal 6.
        store_slider_value(&mut opts, control::SCROLL_RATE, 0);
        assert_eq!(opts.scroll_rate, 6);
    }

    #[test]
    fn toggle_checkbox_flips_matching_bool_only() {
        let mut opts = InGameOptionsState::default(); // lines on, hidden off, tips on
        toggle_checkbox(&mut opts, control::TARGET_LINES);
        assert!(!opts.unit_action_lines);
        assert!(!opts.show_hidden && opts.tooltips, "others untouched");
        toggle_checkbox(&mut opts, control::SHOW_HIDDEN);
        assert!(opts.show_hidden);
    }
}
