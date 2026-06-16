//! Active in-game Options (`0xBBB`) overlay sprite construction.
//!
//! Part of the app layer: turns the render-agnostic `0xBBB` descriptor +
//! natively-anchored layout into the owner-draw control draw list, composited as
//! an overlay over the frozen battlefield. 5a-ii is render-only — it paints the
//! released SIDEBTTN buttons, the two visible trackbars, and the three checkboxes
//! at the populate defaults; input (drag/toggle/Back) and the persisted values
//! land in 5a-iii.
//!
//! The visible text statics (title `0x694`, captions `0x714`/`0x715`, value
//! labels `0x671`/`0x672`, footer `0x695`) are emitted as BitFont glyphs from
//! their CSF caption keys (see `build_in_game_options_text_instances`); their
//! rects + keys are transcribed verbatim from the `0xBBB` DLGTEMPLATE. The
//! hidden VisualDetails caption/label (`0x716`/`0x673`) carry `visible: false`
//! and are skipped. The value labels paint their template default (`GUI:Faster`);
//! the slider-position-driven label swap (TXT_SLOWEST..TXT_FASTEST) is 5a-iii.

use crate::assets::csf_file::CsfFile;
use crate::render::batch::SpriteInstance;
use crate::render::bit_font::BitFont;
use crate::render::shell_text::{self, ShellAlign, TextRect};
use crate::render::skirmish_shell_chrome::{ControlChrome, SkirmishShellChromeAtlas};
use crate::sidebar::SidebarView;
use crate::ui::shell::descriptor::ControlKind;
use crate::ui::shell::geom;
use crate::ui::shell::in_game_options::{build_in_game_options_descriptor, control};
use crate::ui::shell::layout::{InGameOptionsAnchor, layout_pass_in_game_options};
use crate::ui::skirmish_shell::trackbar_pixel_offset;

use super::{SHELL_CONTROL_TEXT_DEPTH, SHELL_LABEL_TEXT_RGB};

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

/// Build the BitFont glyph instances for the active in-game Options overlay's
/// visible text statics (title/captions/value-labels/footer), resolved from their
/// CSF caption keys. Hidden statics (the VisualDetails caption/label) carry
/// `visible: false` and are skipped; `GUI:Blank` (the footer) resolves to empty
/// and emits nothing. These glyphs sample the BitFont atlas — a different texture
/// from the owner-draw chrome atlas — so they are returned as their own instance
/// list and the caller draws them in a separate pass-through call.
///
/// Camera offset is NOT applied here: the caller pre-offsets every overlay
/// instance (chrome + text + cursor) by the rounded camera scroll uniformly, the
/// same convention as `build_in_game_options_instances`.
pub(crate) fn build_in_game_options_text_instances(
    font: &BitFont,
    csf: Option<&CsfFile>,
    screen_w: i32,
    screen_h: i32,
    anchor: InGameOptionsAnchor,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    for draw in in_game_options_static_draws(csf, screen_w, screen_h, anchor) {
        let rect = TextRect {
            x: draw.rect.x,
            y: draw.rect.y,
            w: draw.rect.w.max(0) as u32,
            h: draw.rect.h.max(0) as u32,
        };
        let text_draw = shell_text::draw_in_rect(
            font,
            &draw.text,
            rect,
            SHELL_LABEL_TEXT_RGB,
            draw.align,
            // Caller applies the camera pre-offset uniformly (see doc comment).
            [0.0, 0.0],
            SHELL_CONTROL_TEXT_DEPTH,
            None,
        );
        out.extend(text_draw.instances);
    }
    out
}

/// One resolved static text draw: its laid screen rect, alignment, and display
/// text. Produced for every VISIBLE static-class control with non-empty resolved
/// text (so the hidden VisualDetails statics and the empty `GUI:Blank` footer
/// drop out here). Split from the glyph emission so the selection/rect/align/text
/// logic is unit-testable without a GPU font.
struct OptionsStaticDraw {
    #[cfg_attr(not(test), allow(dead_code))]
    id: u16,
    rect: geom::RectPx,
    align: ShellAlign,
    text: String,
}

fn in_game_options_static_draws(
    csf: Option<&CsfFile>,
    screen_w: i32,
    screen_h: i32,
    anchor: InGameOptionsAnchor,
) -> Vec<OptionsStaticDraw> {
    let desc = build_in_game_options_descriptor();
    let laid = layout_pass_in_game_options(&desc, screen_w, screen_h, anchor);
    let mut out = Vec::new();
    for (c, l) in desc.controls.iter().zip(laid.iter()) {
        if !c.visible || c.kind != ControlKind::Static {
            continue;
        }
        let Some(key) = c.csf_key else {
            continue;
        };
        let text = resolve_static_text(csf, key);
        if text.is_empty() {
            continue;
        }
        out.push(OptionsStaticDraw {
            id: c.id,
            rect: l.rect,
            align: options_static_align(c.id),
            text,
        });
    }
    out
}

/// Resolve a static's CSF caption key to display text, falling back to an English
/// string when the CSF table is unavailable or the key is missing. `GUI:Blank`
/// (the footer) and unknown keys fall back to empty, so the footer paints nothing.
fn resolve_static_text(csf: Option<&CsfFile>, key: &str) -> String {
    csf.and_then(|c| c.get(key))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| options_static_fallback(key).to_string())
}

fn options_static_fallback(key: &str) -> &'static str {
    match key {
        "GUI:GameOptions" => "Game Options",
        "GUI:GameSpeed" => "Game Speed",
        "GUI:ScrollRate" => "Scroll Rate",
        "GUI:Faster" => "Faster",
        // GUI:Blank (footer) + anything else -> no text.
        _ => "",
    }
}

/// Per-static text alignment, derived from the `0xBBB` template `SS_*` style bits:
/// the title `0x694` is SS_CENTER (and is the same control id/rect as the skirmish
/// right-panel title `0x694`, which renders H-centered + top-anchored, no
/// V_CENTER); the GameSpeed/ScrollRate captions are SS_RIGHT; the value labels and
/// footer are SS_LEFT. Captions/labels are vertically centered against their
/// slider row (the trackbar value-label convention); the title is top-anchored.
/// Vertical placement is a manual-visual-gate item (confirm vs gamemd side-by-side).
fn options_static_align(id: u16) -> ShellAlign {
    match id {
        control::TITLE => ShellAlign::H_CENTER,
        control::GAME_SPEED_CAPTION | control::SCROLL_RATE_CAPTION => {
            ShellAlign::H_RIGHT | ShellAlign::V_CENTER
        }
        // Value labels (0x671/0x672) + footer (0x695): SS_LEFT, vertically centered.
        _ => ShellAlign::V_CENTER,
    }
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

    #[test]
    fn static_draws_select_visible_statics_resolve_text_and_align() {
        // csf=None -> English fallbacks. The footer (GUI:Blank) falls back to
        // empty and drops out; the two hidden VisualDetails statics (visible:false)
        // drop out. Leaves title + 2 captions + 2 value labels = 5.
        let draws = in_game_options_static_draws(None, 800, 600, test_anchor());
        let ids: Vec<u16> = draws.iter().map(|d| d.id).collect();
        assert_eq!(draws.len(), 5, "title + 2 captions + 2 value labels");
        for id in [
            control::TITLE,
            control::GAME_SPEED_CAPTION,
            control::SCROLL_RATE_CAPTION,
            control::GAME_SPEED_VALUE,
            control::SCROLL_RATE_VALUE,
        ] {
            assert!(ids.contains(&id), "missing static {id:#06x}");
        }
        // Hidden VisualDetails caption/label + the blank footer are NOT drawn.
        assert!(!ids.contains(&control::VISUAL_DETAILS_CAPTION));
        assert!(!ids.contains(&control::VISUAL_DETAILS_VALUE));
        assert!(!ids.contains(&control::FOOTER));

        let find = |id: u16| draws.iter().find(|d| d.id == id).unwrap();
        // Alignment from the template SS_* bits.
        assert_eq!(find(control::TITLE).align, ShellAlign::H_CENTER);
        assert_eq!(
            find(control::GAME_SPEED_CAPTION).align,
            ShellAlign::H_RIGHT | ShellAlign::V_CENTER
        );
        assert_eq!(find(control::GAME_SPEED_VALUE).align, ShellAlign::V_CENTER);
        // Laid rect == projected DLU rect (centered offset is 0 at the 800x600 base).
        assert_eq!(find(control::TITLE).rect, geom::dlu_rect(425, 1, 108, 10));
        assert_eq!(
            find(control::GAME_SPEED_CAPTION).rect,
            geom::dlu_rect(61, 99, 78, 15)
        );
        // The value labels paint the template default ("Faster") for 5a-ii; the
        // slider-position swap is 5a-iii.
        assert_eq!(find(control::GAME_SPEED_VALUE).text, "Faster");
    }

    #[test]
    fn text_instances_emit_glyphs_from_bit_font_atlas_path() {
        // Smoke test the glyph delegation: a font carrying the letters of "Faster"
        // (the value-label fallback) yields glyph instances. The detailed layout
        // math lives in shell_text/bit_font unit tests.
        use crate::render::bit_font::tests::make_test_font;
        let font = make_test_font(
            &[
                (b'F' as u16, 6),
                (b'a' as u16, 6),
                (b's' as u16, 6),
                (b't' as u16, 6),
                (b'e' as u16, 6),
                (b'r' as u16, 6),
            ],
            8,
        );
        let out = build_in_game_options_text_instances(&font, None, 800, 600, test_anchor());
        assert!(
            !out.is_empty(),
            "value labels emit glyphs from the font atlas"
        );
    }
}
