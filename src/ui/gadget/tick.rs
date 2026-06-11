//! The per-tick dispatch authority: hit-test (G14), per-gadget filter (G15),
//! and the three-tier event tick (G5-G13). Deterministic: all inputs arrive in
//! `GadgetInput`; NO wall-clock access.

use super::button::dispatch_action;
use super::focus::FocusState;
use super::list::GadgetList;
use super::{
    FLAG_KEYBOARD, FLAG_LEFT_HELD, FLAG_LEFT_PRESS, FLAG_LEFT_RELEASE, FLAG_LEFT_UP,
    FLAG_RIGHT_HELD, FLAG_RIGHT_PRESS, FLAG_RIGHT_RELEASE, FLAG_RIGHT_UP, GadgetHandle,
    HIT_SEED_AREA, KEY_LMB_DOWN, KEY_LMB_UP, KEY_RMB_DOWN, KEY_RMB_UP,
};

/// One tick's input snapshot, built by the app driver.
#[derive(Debug, Clone, Copy, Default)]
pub struct GadgetInput {
    /// Queued event: 0 = idle tick; KEY_* mouse codes; any other non-zero
    /// value = keyboard event (G8).
    pub queued_key: u16,
    /// Coordinates latched when the event was queued (G6: used iff the key's
    /// low byte is 1 or 2).
    pub event_x: i32,
    pub event_y: i32,
    /// Live cursor position (G6 idle/keyboard source; G22 inside-test source).
    pub mouse_x: i32,
    pub mouse_y: i32,
    /// Live button state (G8 held bits — idle ticks only).
    pub left_held: bool,
    pub right_held: bool,
    /// Modifier keys, polled fresh (G9).
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// One emitted paint (G11/G12/G19 cadence record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawCmd {
    pub handle: GadgetHandle,
    pub forced: bool,
}

/// One Action dispatch (post-G15-masking) — test/observability record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchRecord {
    pub handle: GadgetHandle,
    pub masked_flags: u16,
    /// G9 modifier word: SHIFT=1 | CTRL=2 | ALT=4 (0 outside the broadcast tier).
    pub modifier: u8,
}

/// Tick results. Owned by the driver and reused across ticks (buffers are
/// cleared, not reallocated — no per-frame allocation in the input path).
#[derive(Debug, Default)]
pub struct TickOutput {
    pub draws: Vec<DrawCmd>,
    pub dispatches: Vec<DispatchRecord>,
    /// Hover transition this tick (G7): old gadget left / new gadget entered.
    pub hover_left: Option<GadgetHandle>,
    pub hover_entered: Option<GadgetHandle>,
    /// The broadcast-walk consumer (None for sticky/keyboard-tier ticks).
    pub consumed_by: Option<GadgetHandle>,
}

impl TickOutput {
    pub fn clear(&mut self) {
        self.draws.clear();
        self.dispatches.clear();
        self.hover_left = None;
        self.hover_entered = None;
        self.consumed_by = None;
    }
}

/// G14 — hit test: forward walk, skip disabled, HALF-OPEN rects, smallest
/// area wins with a signed `<=` tie-break (equal area ⇒ the LATER gadget
/// wins), seeded with the fixed 786,432 px² constant — a gadget larger than
/// the seed can never win.
pub fn hit_test(list: &GadgetList, mx: i32, my: i32) -> Option<GadgetHandle> {
    let mut best: Option<GadgetHandle> = None;
    let mut best_area: i32 = HIT_SEED_AREA;
    for g in list.iter() {
        if g.is_disabled || !g.rect.contains(mx, my) {
            continue;
        }
        let area = g.rect.area();
        if area <= best_area {
            best = Some(g.handle);
            best_area = area;
        }
    }
    best
}

/// G15 — clicked-on filter: mask FIRST, then dispatch iff the gadget is the sticky
/// holder (even masked-0), OR masked flags contain the keyboard bit (bounds
/// bypassed), OR masked flags are non-zero AND the point is inside the
/// half-open rect. Returns the Action result (non-zero = consumed).
#[allow(clippy::too_many_arguments)]
pub(crate) fn clicked_on(
    list: &mut GadgetList,
    handle: GadgetHandle,
    key: &mut u16,
    raw_flags: u16,
    x: i32,
    y: i32,
    modifier: u8,
    live: (i32, i32),
    focus: &mut FocusState,
    out: &mut TickOutput,
) -> u32 {
    let is_sticky_holder = focus.sticky == Some(handle);
    let Some(g) = list.get_mut(handle) else {
        return 0;
    };
    let masked = raw_flags & g.flags;
    if !is_sticky_holder
        && (masked & FLAG_KEYBOARD) == 0
        && (masked == 0 || !g.rect.contains(x, y))
    {
        return 0;
    }
    out.dispatches.push(DispatchRecord {
        handle,
        masked_flags: masked,
        modifier,
    });
    dispatch_action(g, masked, key, live, focus)
}

/// G19 — draw(forced): paints iff forced or dirty, then clears the dirty
/// byte; the paint is emitted as a `DrawCmd`.
pub(crate) fn draw_one(
    list: &mut GadgetList,
    handle: GadgetHandle,
    forced: bool,
    out: &mut TickOutput,
) {
    if let Some(g) = list.get_mut(handle) {
        if forced || g.is_to_redraw {
            g.is_to_redraw = false;
            out.draws.push(DrawCmd { handle, forced });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::gadget::list::{GadgetBehavior, GadgetSpec};
    use crate::ui::gadget::{GadgetRect, ListId};

    fn spec(rect: GadgetRect, flags: u16) -> GadgetSpec {
        GadgetSpec::new(rect, flags, false)
    }

    #[test]
    fn g14_smallest_area_wins() {
        let mut l = GadgetList::new(ListId(1));
        let big = l.add_tail(spec(GadgetRect::new(0, 0, 100, 100), 0xFF));
        let small = l.add_tail(spec(GadgetRect::new(10, 10, 20, 20), 0xFF));
        assert_eq!(hit_test(&l, 15, 15), Some(small), "smaller wins inside both");
        assert_eq!(hit_test(&l, 90, 90), Some(big), "only the big one contains");
        assert_eq!(hit_test(&l, 200, 200), None);
    }

    #[test]
    fn g14_equal_area_later_wins() {
        let mut l = GadgetList::new(ListId(1));
        let _first = l.add_tail(spec(GadgetRect::new(0, 0, 20, 20), 0xFF));
        let second = l.add_tail(spec(GadgetRect::new(0, 0, 20, 20), 0xFF));
        assert_eq!(
            hit_test(&l, 5, 5),
            Some(second),
            "signed <= tie-break: later-in-list wins on equal area"
        );
    }

    #[test]
    fn g14_disabled_invisible_and_seed_caps_area() {
        let mut l = GadgetList::new(ListId(1));
        let mut d = spec(GadgetRect::new(0, 0, 10, 10), 0xFF);
        d.disabled = true;
        l.add_tail(d);
        assert_eq!(hit_test(&l, 5, 5), None, "disabled gadgets are invisible");
        // Area 1024*768 ties the seed (signed <=) and CAN win; one px more cannot.
        let mut l2 = GadgetList::new(ListId(2));
        let exact = l2.add_tail(spec(GadgetRect::new(0, 0, 1024, 768), 0xFF));
        assert_eq!(hit_test(&l2, 5, 5), Some(exact), "area == seed ties via <=");
        let mut l3 = GadgetList::new(ListId(3));
        l3.add_tail(spec(GadgetRect::new(0, 0, 1024, 769), 0xFF));
        assert_eq!(hit_test(&l3, 5, 5), None, "area > 786,432 can never win");
    }

    #[test]
    fn g14_half_open_boundary() {
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(spec(GadgetRect::new(10, 10, 5, 5), 0xFF));
        assert_eq!(hit_test(&l, 10, 10), Some(a));
        assert_eq!(hit_test(&l, 14, 14), Some(a));
        assert_eq!(hit_test(&l, 15, 10), None, "right edge out");
        assert_eq!(hit_test(&l, 10, 15), None, "bottom edge out");
    }

    #[test]
    fn g15_mask_first_filter() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let mut s = spec(GadgetRect::new(0, 0, 10, 10), 0x05);
        s.id = 0x65;
        s.behavior = GadgetBehavior::Control;
        let a = l.add_tail(s);
        let mut key: u16 = 0;
        // Raw flags 0x40 (right release) mask to 0 against 0x05 → early-out
        // even though the point is inside.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, 0x40, 5, 5, 0, (5, 5), &mut f, &mut out),
            0
        );
        assert!(out.dispatches.is_empty(), "filtered before dispatch");
        // Masked non-zero but outside the rect → early-out.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, 0x01, 50, 50, 0, (50, 50), &mut f, &mut out),
            0
        );
        // Masked non-zero and inside → dispatches.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, 0x01, 5, 5, 0, (5, 5), &mut f, &mut out),
            1
        );
        assert_eq!(out.dispatches.len(), 1);
        assert_eq!(out.dispatches[0].masked_flags, 0x01);
    }

    #[test]
    fn g15_sticky_holder_bypasses_even_masked_0() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::button(
            GadgetRect::new(0, 0, 10, 10),
            0x65,
            crate::ui::gadget::list::ToggleKind::Flip,
        ));
        f.sticky = Some(a);
        let mut key: u16 = 0;
        // Raw held flags mask to 0 against mask 5; the holder is dispatched
        // anyway (masked-0 hover-track path).
        clicked_on(&mut l, a, &mut key, 0x82, 50, 50, 0, (50, 50), &mut f, &mut out);
        assert_eq!(out.dispatches.len(), 1);
        assert_eq!(out.dispatches[0].masked_flags, 0);
    }

    #[test]
    fn g15_keyboard_flag_bypasses_bounds() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let mut s = spec(GadgetRect::new(0, 0, 10, 10), 0x05 | FLAG_KEYBOARD);
        s.id = 0x42;
        s.behavior = GadgetBehavior::Control;
        let a = l.add_tail(s);
        let mut key: u16 = 0;
        // Keyboard flag, point far outside → still dispatches.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, FLAG_KEYBOARD, 500, 500, 0, (500, 500), &mut f, &mut out),
            1
        );
        assert_eq!(key, 0x42 | 0x8000);
    }

    #[test]
    fn draw_one_dirty_gate_g19() {
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(spec(GadgetRect::new(0, 0, 10, 10), 0));
        draw_one(&mut l, a, false, &mut out);
        assert!(out.draws.is_empty(), "clean + unforced = no paint");
        l.get_mut(a).unwrap().is_to_redraw = true;
        draw_one(&mut l, a, false, &mut out);
        assert_eq!(out.draws.len(), 1);
        assert!(!l.get(a).unwrap().is_to_redraw, "paint clears the dirty byte");
        draw_one(&mut l, a, true, &mut out);
        assert_eq!(out.draws.len(), 2, "forced always paints");
    }
}
