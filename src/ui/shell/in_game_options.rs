//! In-game Options dialog (0xBBB active) descriptor.
//!
//! Render-agnostic data for the native in-game Options dialog the screen ESC
//! opens during an active game. Depends only on the shared shell descriptor +
//! geometry types (no sim/render/assets), honoring the ui/ layering rule.
//!
//! Scope of this sub-step (5a-i): the ACTIVE `0xBBB` set of nine interactive
//! controls with their verified resource DLU rects, plus the raw DLU->pixel
//! baseline (see `layout::layout_pass`). The text statics (title/captions/value
//! labels/footer), the native child-resize anchoring, the owner-draw paint,
//! input, and INI persistence land in later 5a sub-steps. The shell variant
//! `0xF5` is a follow-on (its full control rects are not all yet verified).
//!
//! Control set verified by transcribing the live `0xBBB` dialog resource
//! template: 17 controls total = 3 owner-draw buttons (Back/Keyboard/Sound) +
//! 3 trackbars (GameSpeed/ScrollRate/VisualDetails) + 3 auto-checkboxes
//! (TargetLines/Tooltips/ShowHidden) + 8 text statics (title `0x694`, captions
//! `0x714`/`0x715`/`0x716`, value labels `0x671`/`0x672`/`0x673`, footer
//! `0x695`). The two formerly-unidentified controls are the ScrollRate caption
//! `0x715` and the VisualDetails caption `0x716` — both STATIC, neither a hidden
//! button. The statics are render-only and land with the paint sub-step.

use super::descriptor::{
    AnchorRule, BgKind, ControlDescriptor, ControlKind, DialogDescriptor, DialogId,
    RepositionPolicy,
};
use super::geom::RectPx;

/// Resource control ids for the active-game Options dialog (`0xBBB`).
pub mod control {
    /// Back button -> own-proc result 1 (close + persist), unconditional.
    pub const BACK: u16 = 0x0686;
    /// Keyboard button -> sub-dialog; STUB until a later 5a step.
    pub const KEYBOARD: u16 = 0x052C;
    /// Sound button -> sub-dialog; STUB until a later 5a step. Runtime-conditional
    /// enable (the proc may disable it when no sound device is present).
    pub const SOUND: u16 = 0x052D;
    /// Game Speed trackbar (range 0..6; value inverted `6 - pos` at apply time).
    pub const GAME_SPEED: u16 = 0x0529;
    /// Scroll Rate trackbar (range 0..6; value inverted `6 - pos` at apply time).
    pub const SCROLL_RATE: u16 = 0x052A;
    /// Visual Details trackbar (range 0..2; direct value). Ships DISABLED in the
    /// active-game template (see `build_in_game_options_descriptor`).
    pub const VISUAL_DETAILS: u16 = 0x052B;
    /// Target Lines checkbox -> Options UnitActionLines.
    pub const TARGET_LINES: u16 = 0x0601;
    /// Show Hidden checkbox -> Options ShowHidden.
    pub const SHOW_HIDDEN: u16 = 0x0604;
    /// Tooltips checkbox -> Options ToolTips.
    pub const TOOLTIPS: u16 = 0x0602;
}

/// RT_DIALOG resource id of the active-game Options dialog.
const DIALOG_0BBB: u16 = 0x0BBB;

/// Build the render-agnostic descriptor for the ACTIVE in-game Options dialog
/// (`0xBBB`): the nine interactive controls with their verified resource DLU
/// rects. Background composites as an overlay over the frozen battlefield.
/// Reposition uses the `InGameOptions` baseline (raw DLU->pixel in 5a-i; native
/// anchoring in 5a-ii).
///
/// `enabled` carries the **resource-template default** (per `ControlDescriptor`):
/// VisualDetails (`0x52B`) is created disabled in the active-game template and
/// the Options proc's populate path sets its range/position but never enables it,
/// so it is stored `enabled: false`. The Sound button is template-enabled here
/// (its runtime no-device disable layers over this in the controller, 5a-iii).
pub fn build_in_game_options_descriptor() -> DialogDescriptor {
    DialogDescriptor {
        id: DialogId(DIALOG_0BBB),
        controls: vec![
            options_control(
                control::BACK,
                ControlKind::Button,
                RectPx::new(425, 346, 108, 23),
                true,
            ),
            options_control(
                control::KEYBOARD,
                ControlKind::Button,
                RectPx::new(425, 149, 108, 23),
                true,
            ),
            options_control(
                control::SOUND,
                ControlKind::Button,
                RectPx::new(425, 122, 108, 23),
                true,
            ),
            options_control(
                control::GAME_SPEED,
                ControlKind::Trackbar,
                RectPx::new(144, 100, 128, 13),
                true,
            ),
            options_control(
                control::SCROLL_RATE,
                ControlKind::Trackbar,
                RectPx::new(144, 131, 128, 13),
                true,
            ),
            // Disabled in the active-game template; the populate path never enables it.
            options_control(
                control::VISUAL_DETAILS,
                ControlKind::Trackbar,
                RectPx::new(144, 162, 128, 13),
                false,
            ),
            options_control(
                control::TARGET_LINES,
                ControlKind::Checkbox,
                RectPx::new(89, 206, 119, 10),
                true,
            ),
            options_control(
                control::SHOW_HIDDEN,
                ControlKind::Checkbox,
                RectPx::new(89, 224, 119, 10),
                true,
            ),
            options_control(
                control::TOOLTIPS,
                ControlKind::Checkbox,
                RectPx::new(214, 206, 127, 10),
                true,
            ),
        ],
        bg_kind: BgKind::InGameOptions,
        slide_eligible: false,
        reposition_policy: RepositionPolicy::InGameOptions,
    }
}

/// One Options control descriptor. The `anchor` field is unused under
/// `RepositionPolicy::InGameOptions` (the native child-resize helpers key off
/// control id/kind, not a per-control anchor enum), so a benign value is stored;
/// CSF captions/labels are attached with the paint sub-step (5a-ii). `enabled`
/// records the resource-template default (runtime enable/disable layers over it
/// in the controller).
fn options_control(
    id: u16,
    kind: ControlKind,
    dlu_rect: RectPx,
    enabled: bool,
) -> ControlDescriptor {
    ControlDescriptor {
        id,
        kind,
        dlu_rect,
        anchor: AnchorRule::RightAnchor,
        csf_key: None,
        tooltip_key: None,
        group: 0,
        enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::geom;
    use crate::ui::shell::layout::layout_pass;

    fn control_ids(d: &DialogDescriptor) -> Vec<u16> {
        d.controls.iter().map(|c| c.id).collect()
    }
    fn kind_of(d: &DialogDescriptor, id: u16) -> ControlKind {
        d.controls
            .iter()
            .find(|c| c.id == id)
            .expect("control id")
            .kind
    }

    #[test]
    fn descriptor_carries_the_nine_active_0bbb_controls() {
        let d = build_in_game_options_descriptor();
        assert_eq!(d.id, DialogId(0x0BBB));
        assert_eq!(d.bg_kind, BgKind::InGameOptions);
        assert_eq!(d.reposition_policy, RepositionPolicy::InGameOptions);
        assert!(!d.slide_eligible);
        let ids = control_ids(&d);
        assert_eq!(ids.len(), 9);
        for id in [
            control::BACK,
            control::KEYBOARD,
            control::SOUND,
            control::GAME_SPEED,
            control::SCROLL_RATE,
            control::VISUAL_DETAILS,
            control::TARGET_LINES,
            control::SHOW_HIDDEN,
            control::TOOLTIPS,
        ] {
            assert!(ids.contains(&id), "missing control {id:#06x}");
        }
    }

    #[test]
    fn control_kinds_match_template() {
        let d = build_in_game_options_descriptor();
        assert_eq!(kind_of(&d, control::BACK), ControlKind::Button);
        assert_eq!(kind_of(&d, control::KEYBOARD), ControlKind::Button);
        assert_eq!(kind_of(&d, control::SOUND), ControlKind::Button);
        assert_eq!(kind_of(&d, control::GAME_SPEED), ControlKind::Trackbar);
        assert_eq!(kind_of(&d, control::SCROLL_RATE), ControlKind::Trackbar);
        assert_eq!(kind_of(&d, control::VISUAL_DETAILS), ControlKind::Trackbar);
        assert_eq!(kind_of(&d, control::TARGET_LINES), ControlKind::Checkbox);
        assert_eq!(kind_of(&d, control::SHOW_HIDDEN), ControlKind::Checkbox);
        assert_eq!(kind_of(&d, control::TOOLTIPS), ControlKind::Checkbox);
    }

    #[test]
    fn descriptor_dlu_rects_match_verified_template() {
        // Verbatim from the live 0xBBB dialog resource template. A 1-DLU drift
        // here shifts the control every time in-game Options opens.
        let d = build_in_game_options_descriptor();
        let dlu = |id: u16| d.controls.iter().find(|c| c.id == id).unwrap().dlu_rect;
        assert_eq!(dlu(control::BACK), RectPx::new(425, 346, 108, 23));
        assert_eq!(dlu(control::KEYBOARD), RectPx::new(425, 149, 108, 23));
        assert_eq!(dlu(control::SOUND), RectPx::new(425, 122, 108, 23));
        assert_eq!(dlu(control::GAME_SPEED), RectPx::new(144, 100, 128, 13));
        assert_eq!(dlu(control::SCROLL_RATE), RectPx::new(144, 131, 128, 13));
        assert_eq!(dlu(control::VISUAL_DETAILS), RectPx::new(144, 162, 128, 13));
        assert_eq!(dlu(control::TARGET_LINES), RectPx::new(89, 206, 119, 10));
        assert_eq!(dlu(control::SHOW_HIDDEN), RectPx::new(89, 224, 119, 10));
        assert_eq!(dlu(control::TOOLTIPS), RectPx::new(214, 206, 127, 10));
    }

    #[test]
    fn enabled_state_matches_template_default() {
        // Resource-template default: VisualDetails is the only control created
        // disabled in the active-game template; the rest are template-enabled.
        let d = build_in_game_options_descriptor();
        let enabled = |id: u16| d.controls.iter().find(|c| c.id == id).unwrap().enabled;
        assert!(
            !enabled(control::VISUAL_DETAILS),
            "VisualDetails ships disabled"
        );
        for id in [
            control::BACK,
            control::KEYBOARD,
            control::SOUND,
            control::GAME_SPEED,
            control::SCROLL_RATE,
            control::TARGET_LINES,
            control::SHOW_HIDDEN,
            control::TOOLTIPS,
        ] {
            assert!(enabled(id), "control {id:#06x} should be template-enabled");
        }
    }

    #[test]
    fn baseline_layout_is_raw_dlu_to_pixel_per_control() {
        // 5a-i baseline: every control == its raw DLU->pixel rect (no anchor).
        let d = build_in_game_options_descriptor();
        let laid = layout_pass(&d, 800, 600);
        let rect_for = |id: u16| laid.iter().find(|c| c.id == id).unwrap().rect;
        for c in &d.controls {
            let expected = geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h);
            assert_eq!(rect_for(c.id), expected, "control {:#06x}", c.id);
        }
        // Concrete spot-checks (round-half-up DLU factor x*6/4, y*13/8).
        assert_eq!(rect_for(control::BACK), RectPx::new(638, 562, 162, 37));
        assert_eq!(
            rect_for(control::GAME_SPEED),
            RectPx::new(216, 163, 192, 21)
        );
        assert_eq!(rect_for(control::TOOLTIPS), RectPx::new(321, 335, 191, 16));
    }
}
