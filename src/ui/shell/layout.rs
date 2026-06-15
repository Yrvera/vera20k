//! Shell dialog layout pass.
//!
//! Implements contract C7: convert each control's resource rect from DLUs to
//! pixels once, then re-anchor it by its per-control `AnchorRule` for include-set
//! dialogs. Modal-centered dialogs skip the re-anchor (the caller centers them).
//! Render-agnostic; consumes only the descriptor + shared geometry primitives.

use super::descriptor::{AnchorRule, ControlKind, DialogDescriptor, RepositionPolicy};
use super::geom::{self, RectPx, RightPanelRects};

/// Centering base width — the logical shell is authored at 800x600 and
/// horizontally compensated on wider screens. Matches the per-shell helpers.
const SHELL_BASE_W: i32 = 800;
/// Centering base height — the in-game Options dialog is authored at 800x600 and
/// its ordinary controls take the centered vertical offset above this.
const SHELL_BASE_H: i32 = 600;
/// Active in-game Options (`0xBBB`) owner-draw button right-edge inset:
/// `x = screen_w - 147`. A literal pixel inset the native child-resize helper
/// applies, not a struct field or canvas-size read.
const IN_GAME_OPTIONS_BUTTON_RIGHT_INSET: i32 = 147;
/// 25-px row pitch for the upper button stack (Sound at +0, Keyboard at +25).
const IN_GAME_OPTIONS_BUTTON_ROW_PITCH: i32 = 25;

/// One control's resolved pixel rect, keyed by its resource id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaidOutControl {
    pub id: u16,
    pub rect: RectPx,
}

/// Run the shell layout pass over a dialog descriptor (contract C7). Returns one
/// resolved pixel rect per control, in descriptor order.
pub fn layout_pass(desc: &DialogDescriptor, screen_w: i32, screen_h: i32) -> Vec<LaidOutControl> {
    let panel = geom::right_panel_rects(screen_w, screen_h);
    desc.controls
        .iter()
        .map(|c| {
            let rect = match desc.reposition_policy {
                RepositionPolicy::IncludeSetReanchor => {
                    apply_anchor(c.anchor, c.dlu_rect, screen_w, panel)
                }
                // Modal-centered dialogs keep their DLU-derived client rect; the
                // caller positions the modal panel (no fullscreen re-anchor).
                RepositionPolicy::ModalCentered => {
                    geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h)
                }
                // Active in-game Options (`0xBBB`). Ordinary controls
                // (trackbars/checkboxes/statics) take the screen-centered offset
                // (zero at the 800x600 base, +112/+84 at 1024x768); owner-draw
                // buttons additionally pin to the right edge at the SIDEBTTN canvas
                // with a sidebar-anchored row Y. That button anchoring needs the
                // runtime SIDEBTTN size + in-game sidebar geometry (above this
                // layer), so the production overlay resolves the full layout via
                // `layout_pass_in_game_options`; this bare `layout_pass` applies
                // only the centered offset every child shares.
                RepositionPolicy::InGameOptions => {
                    let (dx, dy) = in_game_options_centered_offset(screen_w, screen_h);
                    geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h)
                        .translate(dx, dy)
                }
            };
            LaidOutControl { id: c.id, rect }
        })
        .collect()
}

/// Active-only centered offset for the in-game Options (`0xBBB`) ordinary
/// controls: `((screen-base)/2).max(0)` per axis (0 at the 800x600 base,
/// +112/+84 at 1024x768). Owner-draw buttons do NOT take this — they right-edge
/// anchor instead.
fn in_game_options_centered_offset(screen_w: i32, screen_h: i32) -> (i32, i32) {
    (
        geom::center_offset(screen_w, SHELL_BASE_W),
        geom::center_offset(screen_h, SHELL_BASE_H),
    )
}

/// App-supplied anchoring inputs for the active in-game Options (`0xBBB`) overlay
/// that `ui/shell` cannot compute itself. The native child-resize helper anchors
/// the owner-draw buttons to the in-game SIDEBAR geometry (a 25-px row stack) and
/// sizes them to the loaded SIDEBTTN canvas; both live above this layer, so the
/// render/app layer fills these in and `layout_pass_in_game_options` consumes
/// them — keeping `ui/shell` render-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InGameOptionsAnchor {
    /// SIDEBTTN SHP canvas width (read from the loaded SHP header; 125 px).
    pub button_canvas_w: i32,
    /// SIDEBTTN SHP canvas height (read from the loaded SHP header; 25 px).
    pub button_canvas_h: i32,
    /// Top Y of the upper button stack: Sound at +0, Keyboard at +25. Bound to
    /// the in-game sidebar's button-column anchor (KD-4 — verify at the gate).
    pub button_stack_top_y: i32,
    /// Top Y of the bottom-anchored Back button row (sidebar's bottom anchor).
    pub back_button_y: i32,
}

/// Resolve the active in-game Options (`0xBBB`) layout with the app-supplied
/// `anchor`. Ordinary controls (trackbars/checkboxes/statics) take the
/// screen-centered offset; owner-draw buttons (Back/Keyboard/Sound) render at the
/// SIDEBTTN canvas size, right-edge anchored at `screen_w - 147`, with a 25-px
/// row-stack Y bound to the in-game sidebar. `!visible` controls are still laid
/// out in descriptor order (the emitter skips them); their rect is harmless.
pub fn layout_pass_in_game_options(
    desc: &DialogDescriptor,
    screen_w: i32,
    screen_h: i32,
    anchor: InGameOptionsAnchor,
) -> Vec<LaidOutControl> {
    use super::in_game_options::control;
    let (dx, dy) = in_game_options_centered_offset(screen_w, screen_h);
    desc.controls
        .iter()
        .map(|c| {
            let rect = if c.kind == ControlKind::Button {
                let y = match c.id {
                    control::BACK => anchor.back_button_y,
                    control::KEYBOARD => {
                        anchor.button_stack_top_y + IN_GAME_OPTIONS_BUTTON_ROW_PITCH
                    }
                    // Sound tops the stack; any other button shares its anchor.
                    _ => anchor.button_stack_top_y,
                };
                RectPx::new(
                    screen_w - IN_GAME_OPTIONS_BUTTON_RIGHT_INSET,
                    y,
                    anchor.button_canvas_w,
                    anchor.button_canvas_h,
                )
            } else {
                geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h)
                    .translate(dx, dy)
            };
            LaidOutControl { id: c.id, rect }
        })
        .collect()
}

/// Resolve one control's pixel rect by re-anchor rule. `dlu` is the raw DLU
/// resource rect; the owner-draw button rules use only `dlu.y`.
fn apply_anchor(rule: AnchorRule, dlu: RectPx, screen_w: i32, panel: RightPanelRects) -> RectPx {
    match rule {
        AnchorRule::OwnerDrawButtonSnap { cell_w } => {
            geom::snap_button_round_half_up(dlu.y, panel, cell_w)
        }
        AnchorRule::OwnerDrawButtonRawTop { cell_w } => {
            let y = geom::mul_div_round(dlu.y, geom::DLU_BASE_Y, 8) + panel.top.y;
            let x = panel.top.x + (geom::RIGHT_PANEL_WIDTH - cell_w);
            RectPx::new(x, y, cell_w, geom::SDBTNANM_CELL_H)
        }
        AnchorRule::RightAnchor => {
            right_anchor(screen_w, panel, geom::dlu_rect(dlu.x, dlu.y, dlu.w, dlu.h))
        }
        AnchorRule::RightAnchorNudge { dy, dh } => {
            let a = right_anchor(screen_w, panel, geom::dlu_rect(dlu.x, dlu.y, dlu.w, dlu.h));
            RectPx::new(a.x, a.y + dy, a.w, a.h + dh)
        }
    }
}

/// Right-panel child anchor: sidebar-inset + oversized-screen horizontal
/// compensation, anchored to `panel.top.y + rect.y`. Port of the per-shell
/// `right_anchor` helper; `rect` is the already-DLU->pixel-converted client rect.
fn right_anchor(screen_w: i32, panel: RightPanelRects, rect: RectPx) -> RectPx {
    let inset = (geom::RIGHT_PANEL_WIDTH - rect.w) / 2;
    let delta_x = geom::center_offset(screen_w, SHELL_BASE_W);
    RectPx::new(
        screen_w - inset - rect.w - delta_x,
        panel.top.y + rect.y,
        rect.w,
        rect.h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::descriptor::{
        BgKind, ControlDescriptor, ControlKind, DialogDescriptor, DialogId,
    };
    use crate::ui::shell::geom::SDBTNANM_CELL_W_NARROW;
    use crate::ui::shell::in_game_options::{build_in_game_options_descriptor, control};

    fn ctrl(id: u16, kind: ControlKind, dlu: RectPx, anchor: AnchorRule) -> ControlDescriptor {
        ControlDescriptor {
            id,
            kind,
            dlu_rect: dlu,
            anchor,
            csf_key: None,
            tooltip_key: None,
            group: 0,
            enabled: true,
            visible: true,
        }
    }

    fn rect_for(laid: &[LaidOutControl], id: u16) -> RectPx {
        laid.iter().find(|c| c.id == id).expect("control id").rect
    }

    /// All four anchor rules reproduce the 0xE2 helper outputs at 800x600.
    #[test]
    fn anchor_rules_reproduce_main_menu_rects_800x600() {
        let desc = DialogDescriptor {
            id: DialogId(0x00E2),
            bg_kind: BgKind::RightPanelShell,
            slide_eligible: true,
            reposition_policy: RepositionPolicy::IncludeSetReanchor,
            controls: vec![
                ctrl(
                    0x0683,
                    ControlKind::Button,
                    RectPx::new(425, 125, 108, 23),
                    AnchorRule::OwnerDrawButtonSnap {
                        cell_w: SDBTNANM_CELL_W_NARROW,
                    },
                ),
                ctrl(
                    0x03EE,
                    ControlKind::Button,
                    RectPx::new(425, 330, 108, 23),
                    AnchorRule::OwnerDrawButtonRawTop {
                        cell_w: SDBTNANM_CELL_W_NARROW,
                    },
                ),
                ctrl(
                    0x0694,
                    ControlKind::Static,
                    RectPx::new(425, 1, 108, 10),
                    AnchorRule::RightAnchorNudge { dy: 7, dh: 1 },
                ),
                ctrl(
                    0x071B,
                    ControlKind::Static,
                    RectPx::new(447, 29, 61, 33),
                    AnchorRule::RightAnchor,
                ),
            ],
        };
        let laid = layout_pass(&desc, 800, 600);
        // Snap button: flush-right 156-cell, first row at y=199.
        assert_eq!(rect_for(&laid, 0x0683), RectPx::new(644, 199, 156, 42));
        // Raw-top Exit: same column, un-snapped low at y=536.
        assert_eq!(rect_for(&laid, 0x03EE), RectPx::new(644, 536, 156, 42));
        // Title: right-anchor (635,2,162,16) then +7y/+1h.
        assert_eq!(rect_for(&laid, 0x0694), RectPx::new(635, 9, 162, 17));
        // Website static: right-anchor of (671,47,92,54) -> x=800-38-92.
        assert_eq!(rect_for(&laid, 0x071B), RectPx::new(670, 47, 92, 54));
    }

    /// Modal-centered dialogs are NOT re-anchored — they keep their DLU->pixel
    /// client rect (contract C7 include-set gating; 0x120/0xCE are excluded).
    #[test]
    fn modal_centered_policy_skips_reanchor() {
        let desc = DialogDescriptor {
            id: DialogId(0x0120),
            bg_kind: BgKind::ModalShp,
            slide_eligible: false,
            reposition_policy: RepositionPolicy::ModalCentered,
            controls: vec![ctrl(
                0x0001,
                ControlKind::Button,
                RectPx::new(425, 125, 108, 23),
                // Anchor rule is ignored under ModalCentered.
                AnchorRule::OwnerDrawButtonSnap {
                    cell_w: SDBTNANM_CELL_W_NARROW,
                },
            )],
        };
        let laid = layout_pass(&desc, 800, 600);
        // Plain DLU->pixel: (425,125,108,23) -> (638,203,162,37). No snap/anchor.
        assert_eq!(rect_for(&laid, 0x0001), geom::dlu_rect(425, 125, 108, 23));
        assert_eq!(rect_for(&laid, 0x0001), RectPx::new(638, 203, 162, 37));
    }

    /// Re-anchor is oversized-screen aware: 1024x768 reproduces the 0xE2 cells.
    #[test]
    fn snap_and_raw_top_track_oversized_screen() {
        let desc = DialogDescriptor {
            id: DialogId(0x00E2),
            bg_kind: BgKind::RightPanelShell,
            slide_eligible: true,
            reposition_policy: RepositionPolicy::IncludeSetReanchor,
            controls: vec![
                ctrl(
                    0x0683,
                    ControlKind::Button,
                    RectPx::new(425, 125, 108, 23),
                    AnchorRule::OwnerDrawButtonSnap {
                        cell_w: SDBTNANM_CELL_W_NARROW,
                    },
                ),
                ctrl(
                    0x03EE,
                    ControlKind::Button,
                    RectPx::new(425, 330, 108, 23),
                    AnchorRule::OwnerDrawButtonRawTop {
                        cell_w: SDBTNANM_CELL_W_NARROW,
                    },
                ),
            ],
        };
        let laid = layout_pass(&desc, 1024, 768);
        assert_eq!(rect_for(&laid, 0x0683), RectPx::new(756, 283, 156, 42));
        assert_eq!(rect_for(&laid, 0x03EE), RectPx::new(756, 620, 156, 42));
    }

    /// Test anchor with deterministic sidebar-derived button Y values so the
    /// owner-draw button placement is fixed regardless of the runtime sidebar.
    fn options_test_anchor() -> InGameOptionsAnchor {
        InGameOptionsAnchor {
            button_canvas_w: 125,
            button_canvas_h: 25,
            button_stack_top_y: 200,
            back_button_y: 540,
        }
    }

    /// 5a-ii: ordinary in-game Options controls take the screen-centered offset —
    /// zero at the 800x600 base (== raw DLU), +112/+84 at 1024x768. (Replaces the
    /// superseded 5a-i screen-invariant raw-DLU baseline.)
    #[test]
    fn in_game_options_ordinary_controls_centered_offset() {
        let desc = build_in_game_options_descriptor();
        let at800 = layout_pass_in_game_options(&desc, 800, 600, options_test_anchor());
        // GameSpeed trackbar 0x529 (144,100,128,13): no shift at the base size.
        assert_eq!(
            rect_for(&at800, control::GAME_SPEED),
            geom::dlu_rect(144, 100, 128, 13)
        );
        assert_eq!(
            rect_for(&at800, control::GAME_SPEED),
            RectPx::new(216, 163, 192, 21)
        );
        let at1024 = layout_pass_in_game_options(&desc, 1024, 768, options_test_anchor());
        let base = geom::dlu_rect(144, 100, 128, 13);
        assert_eq!(
            rect_for(&at1024, control::GAME_SPEED),
            RectPx::new(base.x + 112, base.y + 84, base.w, base.h)
        );
    }

    /// 5a-ii: owner-draw buttons render at the SIDEBTTN 125x25 canvas, right-edge
    /// anchored at `screen_w - 147` (NOT their DLU rect), with the sidebar-derived
    /// 25-px row stack (Sound top, Keyboard +25, Back bottom-anchored).
    #[test]
    fn in_game_options_buttons_right_edge_sidebttn_size() {
        let desc = build_in_game_options_descriptor();
        let at800 = layout_pass_in_game_options(&desc, 800, 600, options_test_anchor());
        let back = rect_for(&at800, control::BACK);
        assert_eq!((back.x, back.y, back.w, back.h), (800 - 147, 540, 125, 25));
        let sound = rect_for(&at800, control::SOUND);
        assert_eq!((sound.x, sound.y), (800 - 147, 200));
        let keyboard = rect_for(&at800, control::KEYBOARD);
        assert_eq!((keyboard.x, keyboard.y), (800 - 147, 225));
        // Buttons track the right edge on a wider screen; the centered ordinary
        // offset never moves them.
        let at1024 = layout_pass_in_game_options(&desc, 1024, 768, options_test_anchor());
        assert_eq!(rect_for(&at1024, control::BACK).x, 1024 - 147);
    }
}
