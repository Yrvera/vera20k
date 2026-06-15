//! Shell dialog layout pass.
//!
//! Implements contract C7: convert each control's resource rect from DLUs to
//! pixels once, then re-anchor it by its per-control `AnchorRule` for include-set
//! dialogs. Modal-centered dialogs skip the re-anchor (the caller centers them).
//! Render-agnostic; consumes only the descriptor + shared geometry primitives.

use super::descriptor::{AnchorRule, DialogDescriptor, RepositionPolicy};
use super::geom::{self, RectPx, RightPanelRects};

/// Centering base width — the logical shell is authored at 800x600 and
/// horizontally compensated on wider screens. Matches the per-shell helpers.
const SHELL_BASE_W: i32 = 800;

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
                // In-game Options baseline (5a-i): raw DLU->pixel client rect, no
                // re-anchor. The native child-resize helper family (centered
                // offsets for ordinary controls + right-edge button anchoring from
                // the SIDEBTTN canvas) lands in 5a-ii, where the button asset
                // dimensions are known.
                RepositionPolicy::InGameOptions => {
                    geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h)
                }
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

    /// In-game Options (5a-i baseline) is NOT re-anchored — it keeps its DLU->pixel
    /// client rect, identical to the ModalCentered baseline. (5a-ii adds the native
    /// B-helper anchoring and this expectation changes.)
    #[test]
    fn in_game_options_baseline_is_raw_dlu_to_pixel() {
        let desc = DialogDescriptor {
            id: DialogId(0x0BBB),
            bg_kind: BgKind::InGameOptions,
            slide_eligible: false,
            reposition_policy: RepositionPolicy::InGameOptions,
            controls: vec![ctrl(
                0x0529,
                ControlKind::Trackbar,
                RectPx::new(144, 100, 128, 13),
                // Anchor is ignored under InGameOptions baseline.
                AnchorRule::RightAnchor,
            )],
        };
        let laid = layout_pass(&desc, 800, 600);
        assert_eq!(rect_for(&laid, 0x0529), geom::dlu_rect(144, 100, 128, 13));
        assert_eq!(rect_for(&laid, 0x0529), RectPx::new(216, 163, 192, 21));
        // Baseline is screen-size invariant for now (oversized-screen centered
        // offsets are deferred to 5a-ii).
        assert_eq!(layout_pass(&desc, 1024, 768), laid);
    }
}
