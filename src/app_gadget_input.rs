//! In-game Framework-A gadget driver: owns the retained sidebar button list
//! (study §6.1 `ui::gadget`), builds/synchronizes it from the live
//! `SidebarView`, feeds it mouse-edge events plus one idle tick per frame,
//! applies fired button IDs onto existing app actions, and publishes the
//! transient pressed bits for the 5-frame visuals.
//!
//! Replaces fire-on-mouse-DOWN for tabs / repair / sell (study G22) and adds
//! the strip-scroll pair (mask 0x55 ⇒ no hold-repeat, one page per click, G23).
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use winit::event::MouseButton;

use crate::app::AppState;
use crate::app_sidebar_render::current_sidebar_view;
use crate::sidebar::{self, SidebarAction, SidebarTab, SidebarView};
use crate::ui::gadget::focus::FocusState;
use crate::ui::gadget::list::{GadgetBehavior, GadgetList, GadgetSpec, ToggleKind};
use crate::ui::gadget::tick::{GadgetInput, TickOutput, tick};
use crate::ui::gadget::{
    GadgetHandle, GadgetRect, KEY_LMB_DOWN, KEY_LMB_UP, KEY_RMB_DOWN, KEY_RMB_UP, ListId,
    RESULT_BUTTON, RESULT_RIGHT,
};

/// gamemd sidebar button IDs (study §2.5 live-population table; Kind/mask
/// identities VERIFIED-LIVE — decompile citation in the plan Sources section).
pub(crate) const ID_TAB_BASE: u16 = 0x00CB; // tabs 0xCB..=0xCE, Kind 2 latch-ON
pub(crate) const ID_REPAIR: u16 = 0x0065; // Kind 1 flip
pub(crate) const ID_SELL: u16 = 0x0066; // Kind 1 flip
pub(crate) const ID_SCROLL_DOWN: u16 = 0x00C9; // +1 page, Kind 0
pub(crate) const ID_SCROLL_UP: u16 = 0x00C8; // −1 page, Kind 0
/// Scroll mask: presses + releases for BOTH buttons, no held bits — no
/// hold-repeat (G23); right-release fires `ID|0xC000`, consumer masks it off.
const SCROLL_FLAGS: u16 = 0x0055;
/// The single in-game gadget list (ListId uniqueness is app-owned).
const IN_GAME_LIST: ListId = ListId(1);

/// Stable handles of the 8 sidebar buttons, in retained order.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarButtonHandles {
    pub tabs: [GadgetHandle; 4],
    pub repair: GadgetHandle,
    pub sell: GadgetHandle,
    pub scroll_down: GadgetHandle,
    pub scroll_up: GadgetHandle,
}

/// Persistent driver state on `AppState`.
#[derive(Debug)]
pub(crate) struct InGameGadgets {
    pub list: GadgetList,
    pub focus: FocusState,
    /// Reused tick output (buffers cleared per tick, never reallocated).
    pub out: TickOutput,
    /// Live held record (G8 idle-tick held-bit source — nothing else in the
    /// app tracks left/right held state).
    pub left_held: bool,
    pub right_held: bool,
    pub handles: Option<SidebarButtonHandles>,
}

impl InGameGadgets {
    pub fn new() -> Self {
        Self {
            list: GadgetList::new(IN_GAME_LIST),
            focus: FocusState::new(),
            out: TickOutput::default(),
            left_held: false,
            right_held: false,
            handles: None,
        }
    }
}

fn rect_px(r: sidebar::Rect) -> GadgetRect {
    GadgetRect::new(
        r.x.round() as i32,
        r.y.round() as i32,
        r.w.round() as i32,
        r.h.round() as i32,
    )
}

/// Atlas frame-0 sizes for the scroll pair, ×ui_scale (same convention as the
/// repair/sell view rects — zero size when the atlas is missing).
fn scroll_sizes(state: &AppState) -> (Option<[f32; 2]>, Option<[f32; 2]>) {
    let Some(atlas) = crate::app_sidebar_render::current_sidebar_chrome(state) else {
        return (None, None);
    };
    let sz = |e: Option<&crate::render::sidebar_chrome::SidebarChromeEntry>| {
        e.map(|e| [e.pixel_size[0] * state.ui_scale, e.pixel_size[1] * state.ui_scale])
    };
    (
        sz(atlas.scroll_down_frames[0].as_ref()),
        sz(atlas.scroll_up_frames[0].as_ref()),
    )
}

/// Build the list once (retained order = tabs 0..3, repair, sell, scroll-down,
/// scroll-up; rects are disjoint so relative order is unobservable today, and
/// this pins ONE order for hit priority + draw, study O7/G20), then re-sync
/// every tick: rects from the live view, disabled bits + is_on from app state
/// (the native external latch-on/latch-off equivalent — tabs are externally
/// driven, study §2.1).
fn sync_gadgets(state: &mut AppState, view: &SidebarView) {
    let (down_size, up_size) = scroll_sizes(state);
    let (down_rect, up_rect) = sidebar::scroll_button_rects(
        &view.layout,
        state.sidebar_layout_spec.sidebar_width,
        down_size,
        up_size,
    );
    let tab_rects: Vec<GadgetRect> = view.tabs.iter().map(|t| rect_px(t.rect)).collect();
    let tab_active: Vec<bool> = view.tabs.iter().map(|t| t.active).collect();
    let repair_rect = rect_px(view.repair_button.rect);
    let sell_rect = rect_px(view.sell_button.rect);
    let gs = state.sidebar_gadget_state.clone();

    let gadgets = &mut state.in_game_gadgets;
    if gadgets.handles.is_none() {
        let list = &mut gadgets.list;
        let zero = GadgetRect::new(0, 0, 0, 0);
        let tabs = [0u16, 1, 2, 3].map(|i| {
            list.add_tail(GadgetSpec::button(zero, ID_TAB_BASE + i, ToggleKind::LatchOn))
        });
        let repair = list.add_tail(GadgetSpec::button(zero, ID_REPAIR, ToggleKind::Flip));
        let sell = list.add_tail(GadgetSpec::button(zero, ID_SELL, ToggleKind::Flip));
        let scroll_down = list.add_tail(
            GadgetSpec::button(zero, ID_SCROLL_DOWN, ToggleKind::Plain).with_flags(SCROLL_FLAGS),
        );
        let scroll_up = list.add_tail(
            GadgetSpec::button(zero, ID_SCROLL_UP, ToggleKind::Plain).with_flags(SCROLL_FLAGS),
        );
        gadgets.handles = Some(SidebarButtonHandles {
            tabs,
            repair,
            sell,
            scroll_down,
            scroll_up,
        });
    }
    let handles = gadgets.handles.expect("built above");
    let sync = |list: &mut GadgetList, h: GadgetHandle, rect, disabled, is_on: Option<bool>| {
        if let Some(g) = list.get_mut(h) {
            g.rect = rect;
            g.is_disabled = disabled;
            if let (Some(on), GadgetBehavior::Button(b)) = (is_on, &mut g.behavior) {
                b.is_on = on;
            }
        }
    };
    for i in 0..4 {
        let rect = tab_rects.get(i).copied().unwrap_or(GadgetRect::new(0, 0, 0, 0));
        let active = tab_active.get(i).copied().unwrap_or(false);
        sync(&mut gadgets.list, handles.tabs[i], rect, gs.tab_disabled[i], Some(active));
    }
    sync(&mut gadgets.list, handles.repair, repair_rect, gs.repair_disabled, Some(gs.repair_mode_on));
    sync(&mut gadgets.list, handles.sell, sell_rect, gs.sell_disabled, Some(gs.sell_mode_on));
    sync(&mut gadgets.list, handles.scroll_down, rect_px(down_rect), false, None);
    sync(&mut gadgets.list, handles.scroll_up, rect_px(up_rect), false, None);
}
