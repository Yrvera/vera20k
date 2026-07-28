//! CPU-side standard-Skirmish loading progress-row identity and geometry.
//!
//! Depends only on immutable launch data and integer shell geometry. GPU asset
//! decoding remains in `render::loading_screen_chrome`; draw submission remains
//! in `app_loading`.

use crate::skirmish_launch::SkirmishLaunchSession;
use crate::ui::shell::geom::RectPx;

const WIDE_BREAKPOINT: u32 = 800;
const NARROW_BASE_X: i32 = 0x0C;
const NARROW_BASE_Y: i32 = 0x100;
const NARROW_ROW_WIDTH: i32 = 0x146;
const WIDE_BASE_X: i32 = 0x10;
const WIDE_BASE_Y: i32 = 0x141;
const WIDE_ROW_WIDTH: i32 = 0x196;
const BAR_X_HELPER_INSET: i32 = 5 + 3;
const BAR_Y_INSET: i32 = 3;
const BAR_HEIGHT_BAND: i32 = 6;
const ROW_PADDING: i32 = 4;
const SIDE_ICON_GAP: i32 = 0x15;
const LABEL_GAP_AFTER_ICON: i32 = 10;
const LABEL_RIGHT_INSET: i32 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadingProgressRowSnapshot {
    pub label: String,
}

impl LoadingProgressRowSnapshot {
    pub fn from_launch_session(session: &SkirmishLaunchSession) -> Self {
        Self {
            label: session.player_name.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LoadingProgressRowLayout {
    pub bar_origin: [i32; 2],
    pub icon_origin: Option<[i32; 2]>,
    pub label_rect: RectPx,
}

pub(crate) fn layout_standard_skirmish_progress_row(
    render_width: u32,
    bar_size: [i32; 2],
    side_icon_size: Option<[i32; 2]>,
    font_height: i32,
) -> LoadingProgressRowLayout {
    let [base_x, base_y, row_width] = if render_width >= WIDE_BREAKPOINT {
        [WIDE_BASE_X, WIDE_BASE_Y, WIDE_ROW_WIDTH]
    } else {
        [NARROW_BASE_X, NARROW_BASE_Y, NARROW_ROW_WIDTH]
    };
    let bar_width = bar_size[0].max(0);
    let bar_height = bar_size[1].max(0);
    let font_height = font_height.max(0);
    let side_icon_size = side_icon_size.filter(|size| size[0] > 0 && size[1] > 0);
    let icon_height = side_icon_size.map_or(0, |size| size[1]);
    let row_height = icon_height
        .max(bar_height + BAR_HEIGHT_BAND)
        .max(font_height)
        + ROW_PADDING;

    let bar_origin = [
        base_x + BAR_X_HELPER_INSET,
        base_y + (row_height - (bar_height + BAR_HEIGHT_BAND)) / 2 + BAR_Y_INSET,
    ];
    let icon_x = base_x + bar_width + SIDE_ICON_GAP;
    let icon_origin = side_icon_size.map(|size| [icon_x, base_y + (row_height - size[1]) / 2]);
    let label_x = side_icon_size.map_or(icon_x, |size| icon_x + size[0] + LABEL_GAP_AFTER_ICON);
    let label_y = base_y + (row_height - font_height) / 2;
    let label_right = base_x + row_width - LABEL_RIGHT_INSET;
    let label_rect = RectPx::new(
        label_x,
        label_y,
        (label_right - label_x).max(0),
        font_height,
    );

    LoadingProgressRowLayout {
        bar_origin,
        icon_origin,
        label_rect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_progress_row_layout_matches_native_640_fixture() {
        let layout = layout_standard_skirmish_progress_row(640, [80, 5], Some([47, 23]), 12);

        assert_eq!(layout.bar_origin, [20, 267]);
        assert_eq!(layout.icon_origin, Some([113, 258]));
        assert_eq!(layout.label_rect, RectPx::new(170, 263, 165, 12));
    }

    #[test]
    fn loading_progress_row_layout_matches_native_800_fixture() {
        let layout = layout_standard_skirmish_progress_row(800, [80, 5], Some([47, 23]), 12);

        assert_eq!(layout.bar_origin, [24, 332]);
        assert_eq!(layout.icon_origin, Some([117, 323]));
        assert_eq!(layout.label_rect, RectPx::new(174, 328, 245, 12));
    }

    #[test]
    fn missing_icon_uses_would_be_icon_anchor_for_label() {
        let layout = layout_standard_skirmish_progress_row(640, [80, 5], None, 12);

        assert_eq!(layout.bar_origin, [20, 261]);
        assert_eq!(layout.icon_origin, None);
        assert_eq!(layout.label_rect, RectPx::new(113, 258, 222, 12));
    }

    #[test]
    fn actual_font_height_can_dominate_the_row() {
        let layout = layout_standard_skirmish_progress_row(640, [80, 5], Some([20, 10]), 30);

        assert_eq!(layout.label_rect.y, 258);
        assert_eq!(layout.label_rect.h, 30);
    }
}
