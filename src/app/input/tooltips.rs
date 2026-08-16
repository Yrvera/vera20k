//! Tooltip service driver (study S1): the ONLY wall-clock reader for tooltip
//! timing. Feeds cursor moves + button kills into `ui::tooltips`, re-syncs the
//! in-game sidebar/cameo region set per frame, and builds the in-game tooltip
//! draw instances. Pregame shell status lines use their dialog hover state
//! directly because they are not the delayed native tooltip mechanism.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::app::presentation::sidebar_render::{current_sidebar_theme, current_sidebar_view};
use crate::assets::csf_file::{CsfArg, format_csf};
use crate::render::batch::SpriteInstance;
use crate::ui::game_screen::GameScreen;
use crate::ui::tooltips::{TipRect, TipRegion};

/// In-game tip ids mirror the gamemd id space: button ids as-is, cameo slots
/// at 1000+.
pub(crate) const CAMEO_TIP_ID_BASE: u32 = 1000;

/// CSF label carrying the cameo tooltip money format. Retail English value is
/// `"%s $%d"` — name, space, currency sign, cost.
///
/// gamemd has a second branch that would use `TXT_MONEY_FORMAT_1` (`"$%d"`,
/// cost only), selected by a global flag. That flag is dead in stock YR: it
/// has exactly two read sites (the cameo tooltip and the cameo status text)
/// and no write site anywhere in the image, and its stored value is 0. The
/// cost-only form is therefore unreachable and is deliberately not modelled.
const CAMEO_TIP_MONEY_FORMAT: &str = "TXT_MONEY_FORMAT_2";

/// CSF labels for the sidebar gadget tips. gamemd dispatches these by the same
/// gadget ids we use (`gadget_input::ID_*`), passing the label in ECX; the
/// numbers previously mistaken for CSF ids at these call sites are the
/// engine's `__LINE__` values.
const TIP_LABEL_SCROLL_UP: &str = "Tip:ScrollUp";
const TIP_LABEL_SCROLL_DOWN: &str = "Tip:ScrollDown";
const TIP_LABELS_TAB: [&str; 4] = ["Tip:Tab1", "Tip:Tab2", "Tip:Tab3", "Tip:Tab4"];
/// CSF label for the parenthesised suffix on a disabled tab/scroll tip.
const TIP_LABEL_DISABLED: &str = "Tip:Disabled";
/// Suffix shape for a disabled tab/scroll tip. Unlike the words, this format
/// is a wide literal compiled into the engine rather than a table entry, so it
/// is not localizable and is reproduced here.
const TIP_DISABLED_FORMAT: &str = "%s\n(%s)";

/// Tip id gamemd uses for the sidebar power meter. It is asked before the
/// sidebar gadget ids, so the region is registered first.
pub(crate) const ID_POWER_TIP: u32 = 999;
/// CSF label for the power readout. Retail English is
/// `"Power = %d\nDrain = %d"`.
const TIP_LABEL_POWER_DRAIN: &str = "TXT_POWER_DRAIN";

/// Box placement: cursor offset + screen clamp (the native placement math is
/// undecoded — plan deferred item).
pub(crate) const TIP_CURSOR_OFFSET: [i32; 2] = [12, 16];
/// Popup box size = measured text plus this much in **total** (not per side):
/// gamemd adds 4 to the measured width and 3 to the measured height.
pub(crate) const TIP_BOX_PAD: [f32; 2] = [4.0, 3.0];
/// Text draw origin inside the popup box: `+2` horizontal, `+4` vertical.
/// With the FNT cell height carrying a 1 px gap under the glyph rows, a
/// one-line tip lands exactly flush with the box bottom.
pub(crate) const TIP_TEXT_INSET: [f32; 2] = [2.0, 4.0];

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

/// gamemd re-formats a tab/scroll tip as `"<tip>\n(<Tip:Disabled>)"` when the
/// gadget's enable test fails. Cameo tips return before this step, and the
/// repair/sell ids never reach it. `disabled == false` returns the tip
/// unchanged, so this is the native branch, not an added gate.
fn with_disabled_suffix(state: &AppState, tip: String, disabled: bool) -> String {
    if !disabled || tip.is_empty() {
        return tip;
    }
    let suffix = csf_text(state, TIP_LABEL_DISABLED);
    format_csf(
        TIP_DISABLED_FORMAT,
        &[CsfArg::Str(&tip), CsfArg::Str(&suffix)],
    )
}

/// Cameo tooltip text for a buildable slot.
///
/// gamemd formats the localized `UIName` and the item cost through CSF
/// `TXT_MONEY_FORMAT_2`, then walks the formatted buffer and rewrites **every**
/// space as a line feed. That includes the space the format string itself puts
/// between the name and the price, so a Grizzly Battle Tank cameo tip is four
/// stacked lines — `Grizzly` / `Battle` / `Tank` / `$700` — not one line and
/// not a name/price pair.
fn cameo_tip_text(money_format: Option<&str>, name: &str, cost: Option<i32>) -> String {
    let formatted = match (money_format, cost) {
        (Some(fmt), Some(cost)) => {
            format_csf(fmt, &[CsfArg::Str(name), CsfArg::Int(i64::from(cost))])
        }
        // VERA-internal: gamemd has neither case — a failed CSF load is fatal
        // there and every buildable cameo carries a cost — so this only keeps
        // an assetless dev run readable. gamemd equivalent UNCHECKED.
        _ => name.to_string(),
    };
    formatted.replace(' ', "\n")
}

/// Sidebar regions, mirroring the native registration set: tabs and scroll
/// arrows from their `Tip:*` labels (with the disabled suffix), repair/sell
/// from direct CSF keys, cameos through the money format.
fn sync_in_game_regions(state: &mut AppState) {
    let Some(view) = current_sidebar_view(state).cloned() else {
        state.tooltips.sync_regions(&[]);
        return;
    };
    let mut regions: Vec<TipRegion> = Vec::with_capacity(9 + view.items.len());
    // Power meter first: gamemd asks the power bar for a tip before the
    // sidebar gadget ids, and registration order decides the hit here.
    let power_rect = crate::sidebar::power_bar_rect(&view.layout, state.sidebar_layout_spec);
    let power_text = state
        .csf
        .as_ref()
        .map(|csf| {
            format_csf(
                csf.text(TIP_LABEL_POWER_DRAIN).as_ref(),
                &[
                    CsfArg::Int(i64::from(view.power_produced)),
                    CsfArg::Int(i64::from(view.power_drained)),
                ],
            )
        })
        .unwrap_or_default();
    regions.push(TipRegion {
        id: ID_POWER_TIP,
        rect: tip_rect(power_rect),
        text: power_text,
    });
    for (i, tab) in view.tabs.iter().enumerate() {
        let label = TIP_LABELS_TAB
            .get(i)
            .copied()
            .unwrap_or(TIP_LABELS_TAB[TIP_LABELS_TAB.len() - 1]);
        let text = with_disabled_suffix(state, csf_text(state, label), tab.disabled);
        regions.push(TipRegion {
            id: crate::app::input::gadget_input::ID_TAB_BASE as u32 + i as u32,
            rect: tip_rect(tab.rect),
            text,
        });
    }
    regions.push(TipRegion {
        id: crate::app::input::gadget_input::ID_REPAIR as u32,
        rect: tip_rect(view.repair_button.rect),
        text: csf_text(state, "TXT_REPAIR_MODE"),
    });
    regions.push(TipRegion {
        id: crate::app::input::gadget_input::ID_SELL as u32,
        rect: tip_rect(view.sell_button.rect),
        text: csf_text(state, "TXT_SELL_MODE"),
    });
    {
        // Our scroll gadget model carries no disabled state yet, so the
        // `Tip:Disabled` branch is unreachable for this pair; the labels
        // themselves are the native ones.
        regions.push(TipRegion {
            id: crate::app::input::gadget_input::ID_SCROLL_DOWN as u32,
            rect: tip_rect(view.scroll_down_button.rect),
            text: csf_text(state, TIP_LABEL_SCROLL_DOWN),
        });
        regions.push(TipRegion {
            id: crate::app::input::gadget_input::ID_SCROLL_UP as u32,
            rect: tip_rect(view.scroll_up_button.rect),
            text: csf_text(state, TIP_LABEL_SCROLL_UP),
        });
    }
    for (slot, item) in view.items.iter().enumerate() {
        let text = if item.is_superweapon {
            // Superweapon slots return early in gamemd: the localized UIName
            // verbatim, with no cost and no space-to-line-feed rewrite. The
            // section name is the fallback only when rules or the string table
            // are absent (assetless dev run).
            item.super_weapon_section
                .as_deref()
                .and_then(|section| state.rules()?.super_weapon(section))
                .and_then(|sw| sw.ui_name.as_deref())
                .and_then(|key| state.csf.as_ref().map(|csf| csf.text(key).into_owned()))
                .unwrap_or_else(|| item.display_name.clone())
        } else {
            let name = state.rules()
                .and_then(|r| r.object(&item.type_id))
                .and_then(|o| o.ui_name.as_deref())
                .and_then(|key| state.csf.as_ref().map(|csf| csf.text(key).into_owned()))
                .unwrap_or_else(|| item.display_name.clone());
            let money_format = state
                .csf
                .as_ref()
                .map(|csf| csf.text(CAMEO_TIP_MONEY_FORMAT));
            cameo_tip_text(money_format.as_deref(), &name, item.cost)
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
    // gamemd draws the tip in the current sidebar text colour, so the same
    // side-dependent colour the cameo labels use — not a fixed yellow.
    let tint = crate::render::sidebar_text::side_highlight_color(current_sidebar_theme(state));
    tooltip_quads(
        &state.bit_font,
        &tip.text,
        [tip.x, tip.y],
        [state.render_width() as f32, state.render_height() as f32],
        tint,
        // Both tooltip lanes are drawn by `draw_pooled_ui`, which binds the UI
        // camera. That uniform still carries the rounded *world* camera
        // position and the shader subtracts it, so every screen-space UI lane
        // has to add it back — the sidebar text lane and the software cursor
        // both do. `tip.x/y` are cursor coordinates, already screen space, so
        // without this the popup is displaced by the whole camera offset and
        // leaves the screen as soon as the player scrolls.
        [state.camera_x, state.camera_y],
        state.bit_font.darken_texture().is_some(),
    )
}

/// Geometry half of [`build_tooltip_instances`], split out so the box metrics
/// and the UI-camera compensation are reachable from a unit test.
///
/// Returns `(fill quads, text quads)`. `camera_offset` must be the live world
/// camera: the UI pipeline subtracts it in the shader, so passing zero here
/// walks the popup off screen.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tooltip_quads(
    font: &crate::render::bit_font::BitFont,
    text: &str,
    tip_xy: [i32; 2],
    screen: [f32; 2],
    tint: [f32; 3],
    camera_offset: [f32; 2],
    with_fill: bool,
) -> (Vec<SpriteInstance>, Vec<SpriteInstance>) {
    // Region the popup is sized and clamped against. gamemd measures with the
    // selected region's width; which region record it picks is UNCHECKED, so
    // the visible screen is used here.
    let (layout, [box_w, box_h]) = size_tip_box(font, text, screen[0] as u32);
    // Cursor offset, clamped on-screen (placement math deferred).
    let bx = ((tip_xy[0] + TIP_CURSOR_OFFSET[0]) as f32).clamp(0.0, (screen[0] - box_w).max(0.0));
    let by = ((tip_xy[1] + TIP_CURSOR_OFFSET[1]) as f32).clamp(0.0, (screen[1] - box_h).max(0.0));

    let mut fill = Vec::with_capacity(1);
    if with_fill {
        fill.push(SpriteInstance {
            position: [bx + camera_offset[0], by + camera_offset[1]],
            size: [box_w, box_h],
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            depth: 0.00021,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });
    }
    let line_advance = font.cell_height();
    let mut out = Vec::new();
    for (i, span) in layout.lines.iter().enumerate() {
        let line = &text[span.start_byte..span.end_byte];
        out.extend(crate::render::sidebar_text::build_text(
            font,
            line,
            bx + TIP_TEXT_INSET[0],
            by + TIP_TEXT_INSET[1] + i as f32 * line_advance,
            1.0,
            0.00020,
            tint,
            camera_offset,
        ));
    }
    (fill, out)
}

/// Native popup sizing: measure the tip with the region width as the wrap
/// limit, add `+4` width and `+3` height, and if that box is not narrower than
/// the region, measure again at `region_width - 4` and re-pad. Returns the
/// layout the draw pass must use together with the box size.
pub(crate) fn size_tip_box(
    font: &crate::render::bit_font::BitFont,
    text: &str,
    region_w: u32,
) -> (crate::render::bit_font::WrapLayout, [f32; 2]) {
    let mut layout = font.wrap_layout(text, region_w);
    let mut box_w = layout.width as f32 + TIP_BOX_PAD[0];
    if region_w > 0 && box_w >= region_w as f32 {
        layout = font.wrap_layout(text, region_w.saturating_sub(TIP_BOX_PAD[0] as u32));
        box_w = layout.width as f32 + TIP_BOX_PAD[0];
    }
    let box_h = layout.height as f32 + TIP_BOX_PAD[1];
    (layout, [box_w, box_h])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::bit_font::tests::make_test_font;

    /// Retail English `TXT_MONEY_FORMAT_2`.
    const RETAIL_MONEY_FORMAT_2: &str = "%s $%d";

    #[test]
    fn cameo_tip_splits_every_space_including_the_formats_own() {
        // gamemd rewrites every 0x20 in the formatted buffer as 0x0A, so the
        // name breaks per word AND the price lands on its own line.
        assert_eq!(
            cameo_tip_text(
                Some(RETAIL_MONEY_FORMAT_2),
                "Grizzly Battle Tank",
                Some(700)
            ),
            "Grizzly\nBattle\nTank\n$700"
        );
        assert_eq!(
            cameo_tip_text(Some(RETAIL_MONEY_FORMAT_2), "Tesla Reactor", Some(600)),
            "Tesla\nReactor\n$600"
        );
    }

    #[test]
    fn cameo_tip_currency_text_comes_from_the_csf_format_not_rust() {
        // A localized table that puts the currency word after the amount must
        // survive verbatim; nothing in the Rust may re-add a '$'.
        let localized = "%s %d kr";
        assert_eq!(
            cameo_tip_text(Some(localized), "Grizzly Tank", Some(700)),
            "Grizzly\nTank\n700\nkr"
        );
        assert!(!cameo_tip_text(Some(localized), "Grizzly", Some(700)).contains('$'));
    }

    #[test]
    fn cameo_tip_without_cost_is_the_name_alone() {
        assert_eq!(
            cameo_tip_text(Some(RETAIL_MONEY_FORMAT_2), "Grizzly Battle Tank", None),
            "Grizzly\nBattle\nTank"
        );
    }

    #[test]
    fn disabled_tip_appends_the_parenthesised_csf_word() {
        assert_eq!(
            format_csf(
                TIP_DISABLED_FORMAT,
                &[CsfArg::Str("Structures Tab"), CsfArg::Str("Disabled")]
            ),
            "Structures Tab\n(Disabled)"
        );
    }

    #[test]
    fn tip_box_is_measured_text_plus_four_by_three() {
        let font = make_test_font(&[(b'x' as u16, 6), (b'y' as u16, 6)], 4);
        // Two lines: measured height is 2 * cell_height.
        let (layout, size) = size_tip_box(&font, "xx\nxy", 0);
        assert_eq!(layout.lines.len(), 2);
        let measured_w = layout.width as f32;
        let measured_h = layout.height as f32;
        assert_eq!(measured_h, font.cell_height() * 2.0);
        assert_eq!(size[0], measured_w + 4.0, "width pad is +4 total");
        assert_eq!(size[1], measured_h + 3.0, "height pad is +3 total");
    }

    #[test]
    fn tip_box_remeasures_at_region_width_minus_four_when_too_wide() {
        // Each 'x' measures 6 px of ink plus 1 px trailing spacing = 7.
        let font = make_test_font(&[(b'x' as u16, 6)], 4);
        let region_w = 31u32;
        // First pass at the region width wraps after 4 glyphs (28 px); with the
        // +4 padding that box is not narrower than the 31 px region, so gamemd
        // measures again at region_width - 4 = 27 and re-pads.
        assert_eq!(font.wrap_layout("xxxxx", region_w).width, 28);
        let (layout, size) = size_tip_box(&font, "xxxxx", region_w);
        assert_eq!(layout.width, 21, "second measure used region_width - 4");
        assert_eq!(size[0], 25.0, "box is the re-measured width + 4");
        assert!(size[0] < region_w as f32);
    }

    #[test]
    fn tip_box_keeps_the_first_measure_when_it_already_fits() {
        let font = make_test_font(&[(b'x' as u16, 6)], 4);
        // 3 glyphs = 21 px, box 25 px, comfortably inside a 100 px region.
        let (layout, size) = size_tip_box(&font, "xxx", 100);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(size[0], 25.0);
        assert_eq!(size[1], font.cell_height() + 3.0);
    }

    /// The UI pipeline binds a camera uniform carrying the live world camera
    /// and the shader subtracts it, so screen-space lanes must add it back.
    /// Passing zero here put the whole popup off screen at any normal camera
    /// position — the regression this pins.
    #[test]
    fn tooltip_quads_compensate_for_the_ui_camera() {
        let font = make_test_font(&[(b'x' as u16, 6)], 4);
        let screen = [800.0, 600.0];
        let at_origin = tooltip_quads(
            &font,
            "xxx",
            [100, 100],
            screen,
            [1.0, 1.0, 0.0],
            [0.0, 0.0],
            true,
        );
        let panned = tooltip_quads(
            &font,
            "xxx",
            [100, 100],
            screen,
            [1.0, 1.0, 0.0],
            [640.0, 480.0],
            true,
        );

        assert_eq!(at_origin.0.len(), 1);
        assert_eq!(at_origin.1.len(), 3);
        assert_eq!(panned.0.len(), at_origin.0.len());
        assert_eq!(panned.1.len(), at_origin.1.len());

        // Every quad — fill and text — shifts by exactly the camera offset the
        // shader will subtract, leaving the popup at the cursor on screen.
        for (a, b) in at_origin.0.iter().zip(panned.0.iter()) {
            assert_eq!(b.position[0] - a.position[0], 640.0);
            assert_eq!(b.position[1] - a.position[1], 480.0);
        }
        for (a, b) in at_origin.1.iter().zip(panned.1.iter()) {
            assert_eq!(b.position[0] - a.position[0], 640.0);
            assert_eq!(b.position[1] - a.position[1], 480.0);
        }

        // And with no pan the box sits at cursor + the native cursor offset,
        // with the text at the +2/+4 inset inside it.
        assert_eq!(
            at_origin.0[0].position,
            [
                100.0 + TIP_CURSOR_OFFSET[0] as f32,
                100.0 + TIP_CURSOR_OFFSET[1] as f32
            ]
        );
        assert_eq!(
            at_origin.1[0].position,
            [
                100.0 + TIP_CURSOR_OFFSET[0] as f32 + 2.0,
                100.0 + TIP_CURSOR_OFFSET[1] as f32 + 4.0
            ]
        );
    }

    #[test]
    fn tip_text_inset_is_two_by_four() {
        // Pins the native draw origin inside the popup box; the previous VERA
        // values were (4, 3).
        assert_eq!(TIP_TEXT_INSET, [2.0, 4.0]);
        assert_eq!(TIP_BOX_PAD, [4.0, 3.0]);
    }

    #[test]
    fn sidebar_gadget_tip_labels_are_the_native_ones() {
        // The ids are gamemd's, so the label table must line up with them:
        // tabs 0xCB..0xCE are Tab1..Tab4 in sidebar tab order.
        assert_eq!(crate::app::input::gadget_input::ID_TAB_BASE, 0x00CB);
        assert_eq!(crate::app::input::gadget_input::ID_SCROLL_UP, 0x00C8);
        assert_eq!(crate::app::input::gadget_input::ID_SCROLL_DOWN, 0x00C9);
        assert_eq!(
            TIP_LABELS_TAB,
            ["Tip:Tab1", "Tip:Tab2", "Tip:Tab3", "Tip:Tab4"]
        );
        assert_eq!(TIP_LABEL_SCROLL_UP, "Tip:ScrollUp");
        assert_eq!(TIP_LABEL_SCROLL_DOWN, "Tip:ScrollDown");
        assert_eq!(TIP_LABEL_DISABLED, "Tip:Disabled");
    }
}
