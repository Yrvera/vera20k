//! Per-gadget Action implementations: sticky capture protocol (G17), base
//! consume (G16), control result posting (G13), and the toggle-button
//! machine (G22/G23). Pure functions over `Gadget` + `FocusState`; the tick
//! (tick.rs) routes into `dispatch_action` after G15 filtering.

use super::focus::FocusState;
use super::list::{Gadget, GadgetBehavior};
use super::{FLAG_RIGHT_PRESS, FLAG_RIGHT_RELEASE, PRESS_BITS, RELEASE_BITS, RESULT_BUTTON, RESULT_RIGHT};

/// G17 — sticky capture protocol: press bits acquire capture iff the gadget is sticky;
/// release bits release holder-only (an acquire+release in one call both run).
pub(crate) fn sticky_process(g: &Gadget, masked: u16, focus: &mut FocusState) {
    if g.is_sticky && (masked & PRESS_BITS) != 0 {
        focus.sticky = Some(g.handle);
    } else if focus.sticky != Some(g.handle) {
        return;
    }
    if (masked & RELEASE_BITS) != 0 {
        focus.sticky = None;
    }
}

/// G16 — base Action: masked-0 consumes nothing; anything else dirties the
/// gadget, runs the capture protocol, and consumes.
pub(crate) fn base_action(g: &mut Gadget, masked: u16, focus: &mut FocusState) -> u32 {
    if masked == 0 {
        return 0;
    }
    g.is_to_redraw = true;
    sticky_process(g, masked, focus);
    1
}

/// G13 — control action: post `id|0x8000` (`|0x4000` iff a right-release
/// fired AND the gadget's mask includes right-press), then chain to base.
/// (The live in-game population has no peer links — peer callbacks are not
/// modeled.)
pub(crate) fn control_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    focus: &mut FocusState,
) -> u32 {
    if masked != 0 {
        *key = if g.id == 0 { 0 } else { g.id | RESULT_BUTTON };
        if (masked & FLAG_RIGHT_RELEASE) != 0 && (g.flags & FLAG_RIGHT_PRESS) != 0 {
            *key = g.id | RESULT_BUTTON | RESULT_RIGHT;
        }
    }
    base_action(g, masked, focus)
}

/// Behavior router called by `clicked_on` after G15 masking. `live` is the
/// LIVE cursor position (G22's inside-test source — never the event coords).
pub(crate) fn dispatch_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    live: (i32, i32),
    focus: &mut FocusState,
) -> u32 {
    match g.behavior {
        GadgetBehavior::Plain => base_action(g, masked, focus),
        GadgetBehavior::Control => control_action(g, masked, key, focus),
        GadgetBehavior::Button(_) => toggle_action(g, masked, key, live, focus),
    }
}

/// Placeholder routing until Task 5 lands the G22 machine.
fn toggle_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    _live: (i32, i32),
    focus: &mut FocusState,
) -> u32 {
    control_action(g, masked, key, focus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::gadget::list::{GadgetList, GadgetSpec};
    use crate::ui::gadget::{GadgetRect, ListId};

    fn one_gadget(spec: GadgetSpec) -> (GadgetList, crate::ui::gadget::GadgetHandle) {
        let mut l = GadgetList::new(ListId(1));
        let h = l.add_tail(spec);
        (l, h)
    }

    #[test]
    fn sticky_acquire_and_holder_only_release_g17() {
        let mut f = FocusState::new();
        // Same list so a/b carry DISTINCT handles (handles are per-list).
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true));
        let b = l.add_tail(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true));
        // Press acquires.
        sticky_process(l.get(a).unwrap(), 0x01, &mut f);
        assert_eq!(f.sticky, Some(a));
        // A non-holder's release does NOT release.
        sticky_process(l.get(b).unwrap(), 0x04, &mut f);
        assert_eq!(f.sticky, Some(a), "holder-only release");
        // Holder's release releases.
        sticky_process(l.get(a).unwrap(), 0x04, &mut f);
        assert_eq!(f.sticky, None);
        // Press+release in one call acquires then releases.
        sticky_process(l.get(a).unwrap(), 0x05, &mut f);
        assert_eq!(f.sticky, None);
        // Non-sticky gadget never acquires.
        let (l3, c) = one_gadget(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, false));
        sticky_process(l3.get(c).unwrap(), 0x01, &mut f);
        assert_eq!(f.sticky, None);
        let _ = c;
    }

    #[test]
    fn base_action_g16() {
        let mut f = FocusState::new();
        let (mut l, a) = one_gadget(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0xFF, true));
        let g = l.get_mut(a).unwrap();
        assert_eq!(base_action(g, 0, &mut f), 0, "masked-0 consumes nothing");
        assert!(!g.is_to_redraw);
        assert_eq!(base_action(g, 0x08, &mut f), 1, "any masked bits consume");
        assert!(g.is_to_redraw);
    }

    #[test]
    fn control_action_result_protocol_g13() {
        let mut f = FocusState::new();
        // Mask includes right press (0x10) → right-release posts |0x4000.
        let mut spec = GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x55, true);
        spec.id = 0xC9;
        spec.behavior = GadgetBehavior::Control;
        let (mut l, a) = one_gadget(spec);
        let mut key: u16 = 0;
        let g = l.get_mut(a).unwrap();
        assert_eq!(control_action(g, 0x04, &mut key, &mut f), 1);
        assert_eq!(key, 0xC9 | 0x8000, "left release posts ID|0x8000");
        key = 0;
        assert_eq!(control_action(g, 0x40, &mut key, &mut f), 1);
        assert_eq!(key, 0xC9 | 0xC000, "right release + masked 0x10 posts ID|0xC000");
        // Mask WITHOUT right press: right-release does not add 0x4000.
        let mut spec2 = GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true);
        spec2.id = 0x65;
        spec2.behavior = GadgetBehavior::Control;
        let (mut l2, b) = one_gadget(spec2);
        key = 0;
        assert_eq!(control_action(l2.get_mut(b).unwrap(), 0x40, &mut key, &mut f), 1);
        assert_eq!(key, 0x65 | 0x8000);
        // id 0 posts 0 on the plain branch.
        let mut spec3 = GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true);
        spec3.behavior = GadgetBehavior::Control;
        let (mut l3, c) = one_gadget(spec3);
        key = 0x1234;
        assert_eq!(control_action(l3.get_mut(c).unwrap(), 0x04, &mut key, &mut f), 1);
        assert_eq!(key, 0, "ID==0 posts 0");
        // masked-0 leaves the key untouched and consumes nothing.
        key = 0x1234;
        assert_eq!(control_action(l3.get_mut(c).unwrap(), 0, &mut key, &mut f), 0);
        assert_eq!(key, 0x1234);
    }
}
