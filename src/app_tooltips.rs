//! Tooltip service driver (study S1): the ONLY wall-clock reader for tooltip
//! timing. Feeds cursor moves + button kills into `ui::tooltips`, re-syncs the
//! in-game sidebar/cameo region set per frame, and builds the in-game tooltip
//! draw instances. Pregame shell status lines use their dialog hover state
//! directly because they are not the delayed native tooltip mechanism.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::app_sidebar_render::current_sidebar_view;
use crate::render::batch::SpriteInstance;
use crate::ui::game_screen::GameScreen;
use crate::ui::tooltips::{TipRect, TipRegion};

/// In-game tip ids mirror the gamemd id space: button ids as-is, cameo slots
/// at 1000+.
pub(crate) const CAMEO_TIP_ID_BASE: u32 = 1000;

/// Interim cameo tip shape: localized name, newline, $cost. gamemd formats
/// through CSF#0xC6E (label unmapped — plan deferred item); name+cost args
/// are the verified content.
const CAMEO_TIP_COST_PREFIX: &str = "$";

/// Box placement: cursor offset + screen clamp (the native placement math is
/// undecoded — plan deferred item).
pub(crate) const TIP_CURSOR_OFFSET: [i32; 2] = [12, 16];
/// Box padding around the measured text (doc-inherited +4/+3 box metrics).
pub(crate) const TIP_PAD: [f32; 2] = [4.0, 3.0];
/// Line stride for multi-line tips (GAME.FNT cell height).
pub(crate) const TIP_LINE_HEIGHT: f32 = 17.0;
/// Tip text tint (interim: shell yellow; native scheme unverified — deferred).
pub(crate) const TIP_TEXT_RGB: [f32; 3] = [1.0, 1.0, 0.0];

pub(crate) fn now_ms(state: &AppState) -> u64 {
    state.tooltip_epoch.elapsed().as_millis() as u64
}

/// CursorMoved feed (all screens).
pub(crate) fn on_mouse_move(state: &mut AppState) {
    let now = now_ms(state);
    let (x, y) = (state.cursor_x.round() as i32, state.cursor_y.round() as i32);
    state.tooltips.on_mouse_move(x, y, now);
}

/// MouseInput feed — ANY button, press or release, kills tip + timer.
pub(crate) fn on_button_event(state: &mut AppState) {
    let now = now_ms(state);
    state.tooltips.on_button(now);
}

/// Per-frame update: refresh regions for the live in-game surface, then pump
/// the delayed-tooltip timer.
pub(crate) fn update(state: &mut AppState) {
    let now = now_ms(state);
    if state.screen == GameScreen::InGame {
        sync_in_game_regions(state);
    } else {
        state.tooltips.sync_regions(&[]);
    }
    state.tooltips.poll(now);
}

fn tip_rect(r: crate::sidebar::Rect) -> TipRect {
    TipRect::new(
        r.x.round() as i32,
        r.y.round() as i32,
        r.w.round() as i32,
        r.h.round() as i32,
    )
}

fn csf_text(state: &AppState, key: &str) -> String {
    state
        .csf
        .as_ref()
        .map(|csf| csf.text(key).into_owned())
        .unwrap_or_default()
}

/// Sidebar regions, mirroring the native registration set (contract lane §3.6):
/// tabs + scroll (EMPTY text until the CSF numeric-id mapping pass — no tip
/// shows, matching the native NULL-text outcome), repair/sell (direct CSF
/// keys), cameos (name + cost, interim format).
fn sync_in_game_regions(state: &mut AppState) {
    let Some(view) = current_sidebar_view(state) else {
        state.tooltips.sync_regions(&[]);
        return;
    };
    let mut regions: Vec<TipRegion> = Vec::with_capacity(8 + view.items.len());
    for (i, tab) in view.tabs.iter().enumerate() {
        regions.push(TipRegion {
            id: crate::app_gadget_input::ID_TAB_BASE as u32 + i as u32,
            rect: tip_rect(tab.rect),
            text: String::new(), // CSF#0x13DB..0x13E1 labels unmapped (deferred)
        });
    }
    regions.push(TipRegion {
        id: crate::app_gadget_input::ID_REPAIR as u32,
        rect: tip_rect(view.repair_button.rect),
        text: csf_text(state, "TXT_REPAIR_MODE"),
    });
    regions.push(TipRegion {
        id: crate::app_gadget_input::ID_SELL as u32,
        rect: tip_rect(view.sell_button.rect),
        text: csf_text(state, "TXT_SELL_MODE"),
    });
    {
        let (down_size, up_size) = {
            let atlas = crate::app_sidebar_render::current_sidebar_chrome(state);
            let sz = |e: Option<&crate::render::sidebar_chrome::SidebarChromeEntry>| {
                e.map(|e| {
                    [
                        e.pixel_size[0] * state.ui_scale,
                        e.pixel_size[1] * state.ui_scale,
                    ]
                })
            };
            match atlas {
                Some(a) => (
                    sz(a.scroll_down_frames[0].as_ref()),
                    sz(a.scroll_up_frames[0].as_ref()),
                ),
                None => (None, None),
            }
        };
        let (down_rect, up_rect) = crate::sidebar::scroll_button_rects(
            &view.layout,
            state.sidebar_layout_spec.sidebar_width,
            down_size,
            up_size,
        );
        regions.push(TipRegion {
            id: crate::app_gadget_input::ID_SCROLL_DOWN as u32,
            rect: tip_rect(down_rect),
            text: String::new(), // CSF#0x13D3 unmapped (deferred)
        });
        regions.push(TipRegion {
            id: crate::app_gadget_input::ID_SCROLL_UP as u32,
            rect: tip_rect(up_rect),
            text: String::new(), // CSF#0x13CD unmapped (deferred)
        });
    }
    for (slot, item) in view.items.iter().enumerate() {
        let text = if item.is_superweapon {
            // SW tips: the SW UIName directly (no cost). Localized SW UIName
            // parse is a deferred follow-up; display_name today.
            item.display_name.clone()
        } else {
            let name = state
                .rules
                .as_ref()
                .and_then(|r| r.object(&item.type_id))
                .and_then(|o| o.ui_name.as_deref())
                .and_then(|key| state.csf.as_ref().map(|csf| csf.text(key).into_owned()))
                .unwrap_or_else(|| item.display_name.clone());
            match item.cost {
                Some(cost) => format!("{name}\n{CAMEO_TIP_COST_PREFIX}{cost}"),
                None => name,
            }
        };
        regions.push(TipRegion {
            id: CAMEO_TIP_ID_BASE + slot as u32,
            rect: tip_rect(item.rect),
            text,
        });
    }
    state.tooltips.sync_regions(&regions);
}

/// In-game tooltip draw: (fill instances on the darken texture, text
/// instances on the GAME.FNT atlas), drawn between the chat overlay and the
/// software cursor (study O10).
pub(crate) fn build_tooltip_instances(
    state: &AppState,
) -> (Vec<SpriteInstance>, Vec<SpriteInstance>) {
    let Some(tip) = state.tooltips.active() else {
        return (Vec::new(), Vec::new());
    };
    if state.screen != GameScreen::InGame {
        return (Vec::new(), Vec::new());
    }
    let font = &state.bit_font;
    let lines: Vec<&str> = tip.text.split('\n').collect();
    let text_w = lines
        .iter()
        .map(|l| font.text_width(l) as f32)
        .fold(0.0_f32, f32::max);
    let box_w = text_w + TIP_PAD[0] * 2.0;
    let box_h = lines.len() as f32 * TIP_LINE_HEIGHT + TIP_PAD[1] * 2.0;
    // Cursor offset, clamped on-screen (placement math deferred).
    let max_x = state.render_width() as f32 - box_w;
    let max_y = state.render_height() as f32 - box_h;
    let bx = ((tip.x + TIP_CURSOR_OFFSET[0]) as f32).clamp(0.0, max_x.max(0.0));
    let by = ((tip.y + TIP_CURSOR_OFFSET[1]) as f32).clamp(0.0, max_y.max(0.0));
    let mut fill = Vec::with_capacity(1);
    if state.bit_font.darken_texture().is_some() {
        fill.push(SpriteInstance {
            position: [bx, by],
            size: [box_w, box_h],
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            depth: 0.00021,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });
    }
    let mut text = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        text.extend(crate::render::sidebar_text::build_text(
            font,
            line,
            bx + TIP_PAD[0],
            by + TIP_PAD[1] + i as f32 * TIP_LINE_HEIGHT,
            1.0,
            0.00020,
            TIP_TEXT_RGB,
            [0.0, 0.0],
        ));
    }
    (fill, text)
}
