//! Active in-game Options (`0xBBB`) overlay sprite construction.
//!
//! Part of the app layer: turns the render-agnostic `0xBBB` descriptor +
//! natively-anchored layout into the owner-draw control draw list, composited as
//! an overlay over the frozen battlefield. 5a-ii is render-only — it paints the
//! released SIDEBTTN buttons, the two visible trackbars, and the three checkboxes
//! at the populate defaults; input (drag/toggle/Back) and the persisted values
//! land in 5a-iii.
//!
//! The static text controls (title/captions/value-labels/footer) carry no
//! verified DLU rects in-repo (the `0xBBB` design table marks them `—`), so they
//! are NOT emitted here — fabricating their rects would violate the parity bar.
//! Grounding the static rects from the `0xBBB` resource template is a follow-up.

use crate::render::batch::SpriteInstance;
use crate::render::skirmish_shell_chrome::{ControlChrome, SkirmishShellChromeAtlas};
use crate::sidebar::SidebarView;
use crate::ui::shell::descriptor::ControlKind;
use crate::ui::shell::geom;
use crate::ui::shell::in_game_options::{build_in_game_options_descriptor, control};
use crate::ui::shell::layout::{InGameOptionsAnchor, layout_pass_in_game_options};
use crate::ui::skirmish_shell::trackbar_pixel_offset;

use super::controls::{ControlPaint, options_button_sidebttn_frame_index, paint_control};

/// SIDEBTTN SHP canvas fallback (header dims), used only if the atlas entry is
/// missing. The retail SIDEBTTN.SHP is 125x25, 3 frames.
const SIDEBTTN_CANVAS_W: i32 = 125;
const SIDEBTTN_CANVAS_H: i32 = 25;

/// The `0xBBB` GameSpeed/ScrollRate trackbars are range 0..6 (the value is
/// inverted `6 - pos` at apply time; for the rendered thumb only the slider
/// position matters).
const OPTIONS_TRACKBAR_MIN: i32 = 0;
const OPTIONS_TRACKBAR_MAX: i32 = 6;
const OPTIONS_TRACKBAR_STEP: i32 = 1;
/// Populate-default slider position (middle) for both sliders until 5a-iii wires
/// the persisted value.
const OPTIONS_TRACKBAR_DEFAULT_POSITION: i32 = 3;

/// Build the owner-draw control draw list for the active in-game Options overlay
/// at the current screen size, using the app-supplied `anchor` (SIDEBTTN canvas +
/// sidebar-bound button column Y). Hidden controls (the VisualDetails triplet)
/// are skipped; only buttons/trackbars/checkboxes are emitted (statics deferred).
pub(crate) fn build_in_game_options_instances(
    chrome: &ControlChrome,
    screen_w: i32,
    screen_h: i32,
    anchor: InGameOptionsAnchor,
) -> Vec<SpriteInstance> {
    let desc = build_in_game_options_descriptor();
    let laid = layout_pass_in_game_options(&desc, screen_w, screen_h, anchor);
    let mut out = Vec::new();
    for (c, l) in desc.controls.iter().zip(laid.iter()) {
        if !c.visible {
            continue;
        }
        let rect = l.rect;
        match c.kind {
            ControlKind::Button => {
                // Render-only: no input yet, so every button paints its released
                // frame (0). Pressed/flash frames arrive with 5a-iii input.
                let frame = options_button_sidebttn_frame_index(false);
                paint_control(&mut out, chrome, ControlPaint::Button { rect, frame });
            }
            ControlKind::Trackbar => {
                let thumb_px = trackbar_pixel_offset(
                    OPTIONS_TRACKBAR_DEFAULT_POSITION,
                    OPTIONS_TRACKBAR_MIN,
                    OPTIONS_TRACKBAR_MAX,
                    OPTIONS_TRACKBAR_STEP,
                    rect,
                );
                paint_control(&mut out, chrome, ControlPaint::Trackbar { rect, thumb_px });
            }
            ControlKind::Checkbox => {
                let checked = options_checkbox_default_checked(c.id);
                paint_control(&mut out, chrome, ControlPaint::Checkbox { checked, rect });
            }
            // The active 0xBBB set carries only buttons/trackbars/checkboxes.
            _ => {}
        }
    }
    out
}

/// Populate-default checkbox state for the `0xBBB` checkboxes (TargetLines on,
/// ShowHidden off, Tooltips on) until 5a-iii reads the persisted Options values.
fn options_checkbox_default_checked(id: u16) -> bool {
    match id {
        control::TARGET_LINES | control::TOOLTIPS => true,
        control::SHOW_HIDDEN => false,
        _ => false,
    }
}

/// Resolve the app-supplied anchoring inputs for the overlay: the SIDEBTTN canvas
/// (from the loaded atlas entry) and the owner-draw button column Y.
///
/// FLAGGED (KD-4): the button column Y is bound to the in-game sidebar panel —
/// Sound/Keyboard stack one row below the sidebar top, Back one row up from its
/// bottom. The exact gamemd sidebar-global Y is unverified; the manual visual
/// gate (Task 9) confirms/tunes this against gamemd side-by-side. When the
/// sidebar geometry is unavailable (headless/edge frames) it falls back to the
/// dialog's own DLU button tops (a flagged DLU-Y fallback, not the final bind).
pub(crate) fn in_game_options_anchor(
    atlas: &SkirmishShellChromeAtlas,
    sidebar_view: Option<&SidebarView>,
) -> InGameOptionsAnchor {
    let (button_canvas_w, button_canvas_h) = atlas
        .options_button_sidebttn_frame0
        .map(|e| {
            (
                e.pixel_size[0].round() as i32,
                e.pixel_size[1].round() as i32,
            )
        })
        .unwrap_or((SIDEBTTN_CANVAS_W, SIDEBTTN_CANVAS_H));
    let (button_stack_top_y, back_button_y) = match sidebar_view {
        Some(sv) => {
            let top = sv.panel_rect.y.round() as i32;
            let bottom = (sv.panel_rect.y + sv.panel_rect.h).round() as i32;
            (top + button_canvas_h, bottom - button_canvas_h)
        }
        None => {
            let desc = build_in_game_options_descriptor();
            let dlu_top = |id: u16| {
                desc.controls
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| {
                        geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h).y
                    })
                    .unwrap_or(0)
            };
            (dlu_top(control::SOUND), dlu_top(control::BACK))
        }
    };
    InGameOptionsAnchor {
        button_canvas_w,
        button_canvas_h,
        button_stack_top_y,
        back_button_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::skirmish_shell_chrome::SkirmishShellChromeEntry;

    fn entry(w: f32, h: f32) -> SkirmishShellChromeEntry {
        SkirmishShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [w, h],
        }
    }

    fn test_anchor() -> InGameOptionsAnchor {
        InGameOptionsAnchor {
            button_canvas_w: 125,
            button_canvas_h: 25,
            button_stack_top_y: 200,
            back_button_y: 540,
        }
    }

    #[test]
    fn ingame_options_emitter_emits_visible_controls_skips_visualdetails() {
        // Buttons-only chrome: exactly the 3 visible owner-draw buttons
        // (Back/Keyboard/Sound) emit one SIDEBTTN glyph each, all right-edge
        // anchored at screen_w - 147 at the native 125x25 canvas. The hidden
        // VisualDetails control contributes nothing.
        let buttons_only = ControlChrome {
            options_button_sidebttn_frame0: Some(entry(125.0, 25.0)),
            ..Default::default()
        };
        let out = build_in_game_options_instances(&buttons_only, 800, 600, test_anchor());
        assert_eq!(out.len(), 3, "3 owner-draw buttons");
        for inst in &out {
            assert_eq!(inst.position[0], (800 - 147) as f32);
            assert_eq!(inst.size, [125.0, 25.0]);
        }

        // Checkbox-icon-only chrome: the 3 visible checkboxes emit one icon each.
        let checks_only = ControlChrome {
            checkbox_checked_cce_i: Some(entry(18.0, 18.0)),
            checkbox_unchecked_cue_i: Some(entry(18.0, 18.0)),
            ..Default::default()
        };
        let out = build_in_game_options_instances(&checks_only, 800, 600, test_anchor());
        assert_eq!(out.len(), 3, "3 checkboxes");

        // Rail-only chrome: only the 2 VISIBLE trackbars (GameSpeed/ScrollRate)
        // emit a rail; the hidden VisualDetails trackbar is skipped.
        let rail_only = ControlChrome {
            trackbar_rail: Some(entry(200.0, 18.0)),
            ..Default::default()
        };
        let out = build_in_game_options_instances(&rail_only, 800, 600, test_anchor());
        assert_eq!(out.len(), 2, "2 visible trackbars (VisualDetails hidden)");

        // Empty chrome: no glyphs loaded -> nothing emitted.
        let out =
            build_in_game_options_instances(&ControlChrome::default(), 800, 600, test_anchor());
        assert!(out.is_empty());
    }
}
