//! Dialog 0x100 Single Player shell layout recovered from gamemd.exe.

use super::state::SinglePlayerControlId;
use crate::ui::main_menu_shell::{MainMenuMovieBase, RectPx, movie_base_for_screen_width};

const SHELL_BASE_W: i32 = 800;
const SHELL_BASE_H: i32 = 600;
const BASE_X: i32 = 6;
const BASE_Y: i32 = 13;
const RIGHT_PANEL_WIDTH: i32 = 168;
const RIGHT_PANEL_TOP_H: i32 = 199;
const RIGHT_PANEL_TILE_H: i32 = 42;
const RIGHT_PANEL_TILE_COUNT_BASE: i32 = 9;
const LOWER_STRIP_H: i32 = 32;
const SDBTNANM_W: i32 = 168;
const SDBTNANM_H: i32 = 42;
const RA2TS_L_W: i32 = 632;
const RA2TS_L_H: i32 = 570;
const RA2TS_S_W: i32 = 472;
const RA2TS_S_H: i32 = 450;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SinglePlayerButtonRect {
    pub id: SinglePlayerControlId,
    pub rect: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RightPanelRects {
    pub top: RectPx,
    pub tile: RectPx,
    pub tile_count: i32,
    pub bottom: RectPx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinglePlayerShellLayout {
    pub screen: RectPx,
    pub movie_base: MainMenuMovieBase,
    pub movie: RectPx,
    pub title: RectPx,
    pub status_help: RectPx,
    pub side_image_static: RectPx,
    pub buttons: [SinglePlayerButtonRect; 4],
    pub pressed_content_offset_x: i32,
    pub right_panel: RightPanelRects,
    pub lower_strip: RectPx,
}

fn mul_div_round(n: i32, numer: i32, denom: i32) -> i32 {
    let value = n * numer;
    if value >= 0 {
        (value + denom / 2) / denom
    } else {
        (value - denom / 2) / denom
    }
}

fn dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx {
    RectPx::new(
        mul_div_round(x, BASE_X, 4),
        mul_div_round(y, BASE_Y, 8),
        mul_div_round(w, BASE_X, 4),
        mul_div_round(h, BASE_Y, 8),
    )
}

fn center_offset(screen: i32, base: i32) -> i32 {
    if screen > base {
        (screen - base) / 2
    } else {
        0
    }
}

fn movie_origin(screen_w: i32, screen_h: i32) -> (i32, i32) {
    let x = if screen_w <= SHELL_BASE_W {
        0
    } else {
        (screen_w - SHELL_BASE_W) / 2
    };
    let y = if screen_h <= SHELL_BASE_H {
        0
    } else {
        (screen_h - SHELL_BASE_H) / 2
    };
    (x, y)
}

fn right_panel_rects(screen_w: i32, screen_h: i32) -> RightPanelRects {
    let left_margin = if screen_w > 1023 {
        (screen_w - SHELL_BASE_W) / 2
    } else {
        0
    };
    let top_margin = if screen_h > 767 {
        (screen_h - SHELL_BASE_H) / 2
    } else {
        0
    };
    let effective_right = screen_w - left_margin;
    let top = RectPx::new(
        effective_right - RIGHT_PANEL_WIDTH,
        top_margin,
        RIGHT_PANEL_WIDTH,
        RIGHT_PANEL_TOP_H,
    );
    let tile = RectPx::new(top.x, top.y + top.h, RIGHT_PANEL_WIDTH, RIGHT_PANEL_TILE_H);
    let effective_h = if screen_h > 767 {
        screen_h - top_margin * 2
    } else {
        screen_h
    };
    let remaining = (effective_h - top.h).max(0);
    let tile_count = (remaining / RIGHT_PANEL_TILE_H).min(RIGHT_PANEL_TILE_COUNT_BASE);
    let bottom_y = tile.y + tile_count * RIGHT_PANEL_TILE_H;
    let bottom_h = (screen_h - top_margin - bottom_y).max(0);
    RightPanelRects {
        top,
        tile,
        tile_count,
        bottom: RectPx::new(top.x, bottom_y, RIGHT_PANEL_WIDTH, bottom_h),
    }
}

fn lower_strip_rect(screen_w: i32, screen_h: i32) -> RectPx {
    let left_margin = if screen_w > 1023 {
        (screen_w - SHELL_BASE_W) / 2
    } else {
        0
    };
    let top_margin = if screen_h > 767 {
        (screen_h - SHELL_BASE_H) / 2
    } else {
        0
    };
    let shell_h = if screen_h > 767 {
        SHELL_BASE_H
    } else {
        screen_h
    };
    let w = if screen_w == 640 { 472 } else { 632 };
    RectPx::new(
        left_margin,
        top_margin + shell_h - LOWER_STRIP_H,
        w,
        LOWER_STRIP_H,
    )
}

fn right_anchor(screen_w: i32, screen_h: i32, original: RectPx) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    let offset_y = center_offset(screen_h, SHELL_BASE_H);
    let inset = (RIGHT_PANEL_WIDTH - original.w) / 2;
    RectPx::new(
        screen_w - offset_x - original.w - inset,
        original.y + offset_y,
        original.w,
        original.h,
    )
}

fn status_help_rect(screen_w: i32, screen_h: i32) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    let offset_y = center_offset(screen_h, SHELL_BASE_H);
    RectPx::new(offset_x + 10, screen_h - offset_y - 21, 455, 20)
}

fn owner_draw_button_snap_rect(
    screen_w: i32,
    screen_h: i32,
    source: RectPx,
    panel: RightPanelRects,
) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    let source_y = source.y + center_offset(screen_h, SHELL_BASE_H);
    let tile_h = panel.tile.h.max(1);
    let tile_index = ((source_y - panel.tile.y + tile_h / 2) / tile_h).max(0);
    RectPx::new(
        screen_w - offset_x - SDBTNANM_W,
        panel.tile.y + tile_index * tile_h,
        SDBTNANM_W,
        SDBTNANM_H,
    )
}

fn back_rect(screen_w: i32, panel: RightPanelRects) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    RectPx::new(
        screen_w - offset_x - SDBTNANM_W,
        panel.tile.y + (panel.tile_count - 1).max(0) * RIGHT_PANEL_TILE_H,
        SDBTNANM_W,
        SDBTNANM_H,
    )
}

pub fn compute_layout(screen_w: u32, screen_h: u32) -> SinglePlayerShellLayout {
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;
    let movie_base = movie_base_for_screen_width(screen_w as u32);
    let (movie_x, movie_y) = movie_origin(screen_w, screen_h);
    let (movie_w, movie_h) = match movie_base {
        MainMenuMovieBase::Ra2tsS => (RA2TS_S_W, RA2TS_S_H),
        MainMenuMovieBase::Ra2tsL => (RA2TS_L_W, RA2TS_L_H),
    };
    let panel = right_panel_rects(screen_w, screen_h);
    let title_base = dlu_rect(425, 1, 108, 10);
    let side_image_base = dlu_rect(446, 29, 61, 33);

    SinglePlayerShellLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        movie_base,
        movie: RectPx::new(movie_x, movie_y, movie_w, movie_h),
        title: {
            let title = right_anchor(screen_w, screen_h, title_base);
            RectPx::new(title.x, title.y + 1, title.w, title.h)
        },
        status_help: status_help_rect(screen_w, screen_h),
        side_image_static: right_anchor(screen_w, screen_h, side_image_base),
        buttons: [
            SinglePlayerButtonRect {
                id: SinglePlayerControlId::NewCampaign0x688,
                rect: owner_draw_button_snap_rect(
                    screen_w,
                    screen_h,
                    dlu_rect(425, 122, 108, 23),
                    panel,
                ),
            },
            SinglePlayerButtonRect {
                id: SinglePlayerControlId::LoadSavedGame0x689,
                rect: owner_draw_button_snap_rect(
                    screen_w,
                    screen_h,
                    dlu_rect(425, 149, 108, 23),
                    panel,
                ),
            },
            SinglePlayerButtonRect {
                id: SinglePlayerControlId::Skirmish0x579,
                rect: owner_draw_button_snap_rect(
                    screen_w,
                    screen_h,
                    dlu_rect(425, 176, 108, 23),
                    panel,
                ),
            },
            SinglePlayerButtonRect {
                id: SinglePlayerControlId::MainMenu0x686,
                rect: back_rect(screen_w, panel),
            },
        ],
        pressed_content_offset_x: 1,
        right_panel: panel,
        lower_strip: lower_strip_rect(screen_w, screen_h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_rects_match_dialog_0x100_rows_at_800x600() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsL);
        assert_eq!(layout.movie, RectPx::new(0, 0, 632, 570));
        assert_eq!(layout.title, RectPx::new(635, 3, 162, 16));
        assert_eq!(layout.buttons[0].rect, RectPx::new(632, 199, 168, 42));
        assert_eq!(layout.buttons[1].rect, RectPx::new(632, 241, 168, 42));
        assert_eq!(layout.buttons[2].rect, RectPx::new(632, 283, 168, 42));
        assert_eq!(layout.buttons[3].rect, RectPx::new(632, 535, 168, 42));
        assert_eq!(layout.status_help, RectPx::new(10, 579, 455, 20));
    }

    #[test]
    fn large_screen_keeps_native_shell_unscaled_and_centered() {
        let layout = compute_layout(1024, 768);
        assert_eq!(layout.movie, RectPx::new(112, 84, 632, 570));
        assert_eq!(layout.right_panel.top, RectPx::new(744, 84, 168, 199));
        assert_eq!(layout.buttons[2].rect, RectPx::new(744, 367, 168, 42));
        assert_eq!(layout.buttons[3].rect, RectPx::new(744, 619, 168, 42));
        assert_eq!(layout.status_help, RectPx::new(122, 663, 455, 20));
    }
}
