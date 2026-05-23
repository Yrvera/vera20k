//! Dialog 0xE2 shell layout recovered from gamemd.exe.

use super::state::MainMenuControlId;

pub const SHELL_BASE_W: i32 = 800;
pub const SHELL_BASE_H: i32 = 600;
const BASE_X: i32 = 6;
const BASE_Y: i32 = 13;
pub const RA2TS_L_W: i32 = 632;
pub const RA2TS_L_H: i32 = 570;
pub const RA2TS_S_W: i32 = 472;
pub const RA2TS_S_H: i32 = 450;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectPx {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectPx {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuMovieBase {
    Ra2tsS,
    Ra2tsL,
}

impl MainMenuMovieBase {
    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::Ra2tsS => "ra2ts_s.bik",
            Self::Ra2tsL => "ra2ts_l.bik",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuButtonRect {
    pub id: MainMenuControlId,
    pub rect: RectPx,
}

/// Right-panel chrome rects (SDTP top, SDBTNBKGD repeated tile, SDBTM bottom)
/// drawn behind the owner-draw buttons by the parent WM_PAINT_Handler in
/// gamemd. Mirrors the skirmish shell layout — both shells share the same
/// right-panel chrome stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RightPanelRects {
    pub top: RectPx,
    pub tile: RectPx,
    pub tile_count: i32,
    pub bottom: RectPx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainMenuShellLayout {
    pub screen: RectPx,
    pub movie_base: MainMenuMovieBase,
    pub movie: RectPx,
    pub title: RectPx,
    /// Bottom-right version/status static. Text is `"<GUI:Version> <VERSION.TXT>"`.
    /// Sits inside the SDBTM cap, sidebar-inset from the right edge.
    pub version_line: RectPx,
    /// Bottom-left hover tooltip/status static. Receives the CSF tooltip for the
    /// control under the cursor and is otherwise blank.
    pub tooltip_line: RectPx,
    pub website_static: RectPx,
    pub buttons: [MainMenuButtonRect; 6],
    pub pressed_content_offset_x: i32,
    pub right_panel: RightPanelRects,
    pub lower_strip: RectPx,
}

/// Native dimensions of the SDBTNBKGD tile and SDTP/SDBTM caps at 800x600.
/// These mirror retail RA2 SHP sizes verified in the skirmish shell research.
pub const RIGHT_PANEL_WIDTH: i32 = 168;
pub const RIGHT_PANEL_TOP_H: i32 = 199;
pub const RIGHT_PANEL_TILE_H: i32 = 42;
pub const RIGHT_PANEL_TILE_COUNT_BASE: i32 = 9;
pub const RIGHT_PANEL_BOTTOM_H: i32 = 23;
pub const LOWER_STRIP_H: i32 = 32;

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

pub fn movie_base_for_screen_width(screen_w: u32) -> MainMenuMovieBase {
    if screen_w == 640 {
        MainMenuMovieBase::Ra2tsS
    } else {
        MainMenuMovieBase::Ra2tsL
    }
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
    // LWSCRNS at 640w is 472 wide; LWSCRNL at >=800w is 632 wide. The lower
    // strip sits flush against the screen bottom (or centered shell bottom).
    let w = if screen_w == 640 { 472 } else { 632 };
    RectPx::new(
        left_margin,
        top_margin + shell_h - LOWER_STRIP_H,
        w,
        LOWER_STRIP_H,
    )
}

/// Bottom-left tooltip/status rect anchored to the screen bottom-left.
///
/// X is offset `+10` px from the centering margin; Y places the control's
/// bottom edge one pixel above the screen bottom (or above the centered
/// shell's bottom on oversized screens).
fn tooltip_line_rect(screen_w: i32, screen_h: i32) -> RectPx {
    let base = dlu_rect(2, 355, 303, 12);
    let ctrl_w = base.w;
    let ctrl_h = base.h;
    let delta_x = if screen_w > SHELL_BASE_W {
        (screen_w - SHELL_BASE_W) / 2
    } else {
        0
    };
    let delta_y = if screen_h > SHELL_BASE_H {
        (screen_h - SHELL_BASE_H) / 2
    } else {
        0
    };
    let x = delta_x + 10;
    let y = screen_h - ctrl_h - delta_y - 1;
    RectPx::new(x, y, ctrl_w, ctrl_h)
}

/// Bottom-right version-line rect anchored to the SDBTM lower-cap bottom edge.
///
/// The retail layout pass uses a sidebar inset of `(168 - ctrl_w) / 2` on the
/// X axis (3 px at the standard 162 px control width), shifted left by
/// `max(0, (screen_w - 800) / 2)` on widescreens. Y anchors the control's
/// bottom edge to the bottom edge of the right-panel lower cap.
fn version_line_rect(screen_w: i32, right_panel: RightPanelRects) -> RectPx {
    let base = dlu_rect(425, 357, 108, 10);
    let ctrl_w = base.w;
    let ctrl_h = base.h;
    let inset = (RIGHT_PANEL_WIDTH - ctrl_w) / 2;
    let delta_x = if screen_w > SHELL_BASE_W {
        (screen_w - SHELL_BASE_W) / 2
    } else {
        0
    };
    let x = screen_w - inset - ctrl_w - delta_x;
    let cap_bottom = right_panel.bottom.y + right_panel.bottom.h;
    let y = cap_bottom - ctrl_h;
    RectPx::new(x, y, ctrl_w, ctrl_h)
}

/// Position a right-panel child control with the same sidebar inset and
/// oversized-screen horizontal compensation used by the retail right-anchor
/// helper.
fn right_anchor_rect(screen_w: i32, right_panel: RightPanelRects, rect: RectPx) -> RectPx {
    let inset = (RIGHT_PANEL_WIDTH - rect.w) / 2;
    let delta_x = if screen_w > SHELL_BASE_W {
        (screen_w - SHELL_BASE_W) / 2
    } else {
        0
    };
    RectPx::new(
        screen_w - inset - rect.w - delta_x,
        right_panel.top.y + rect.y,
        rect.w,
        rect.h,
    )
}

fn title_rect(screen_w: i32, right_panel: RightPanelRects) -> RectPx {
    let anchored = right_anchor_rect(screen_w, right_panel, dlu_rect(425, 1, 108, 10));
    // Retail special-cases the 0x694 heading after the right-anchor pass:
    // top += 7, height += 1 in FUN_0060B950 for main menu dialog 0xE2.
    RectPx::new(anchored.x, anchored.y + 7, anchored.w, anchored.h + 1)
}

/// Snap a DLU-derived button Y to the nearest chrome tile row.
///
/// Each button sits in one SDBTNBKGD tile slot in the right-panel chrome.
/// The DLU positions in the dialog template land within ±4 px of a tile
/// boundary, so we round to the nearest tile and use its rect directly.
/// This makes the button rect == the chrome tile rect, so the SDBTNANM
/// button artwork centers exactly inside it.
fn button_rect_for_dlu_y(dlu_y: i32, right_panel: RightPanelRects) -> RectPx {
    let raw_y = mul_div_round(dlu_y, BASE_Y, 8) + right_panel.top.y;
    let tile_y = right_panel.tile.y;
    let tile_h = right_panel.tile.h.max(1);
    let tile_index = ((raw_y - tile_y + tile_h / 2) / tile_h).max(0);
    let y = tile_y + tile_index * tile_h;
    RectPx::new(right_panel.tile.x, y, right_panel.tile.w, tile_h)
}

pub fn compute_layout(screen_w: u32, screen_h: u32) -> MainMenuShellLayout {
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;
    let movie_base = movie_base_for_screen_width(screen_w as u32);
    let (movie_x, movie_y) = movie_origin(screen_w, screen_h);
    let (movie_w, movie_h) = match movie_base {
        MainMenuMovieBase::Ra2tsS => (RA2TS_S_W, RA2TS_S_H),
        MainMenuMovieBase::Ra2tsL => (RA2TS_L_W, RA2TS_L_H),
    };
    let right_panel = right_panel_rects(screen_w, screen_h);
    let lower_strip = lower_strip_rect(screen_w, screen_h);
    let version_line = version_line_rect(screen_w, right_panel);
    let tooltip_line = tooltip_line_rect(screen_w, screen_h);
    let website_base = dlu_rect(447, 29, 61, 33);
    MainMenuShellLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        movie_base,
        movie: RectPx::new(movie_x, movie_y, movie_w, movie_h),
        title: title_rect(screen_w, right_panel),
        version_line,
        tooltip_line,
        website_static: right_anchor_rect(screen_w, right_panel, website_base),
        buttons: [
            MainMenuButtonRect {
                id: MainMenuControlId::SinglePlayer0x683,
                rect: button_rect_for_dlu_y(125, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::WwOnline0x684,
                rect: button_rect_for_dlu_y(152, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::Network0x578,
                rect: button_rect_for_dlu_y(179, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::MoviesAndCredits0x686,
                rect: button_rect_for_dlu_y(206, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::Options0x55c,
                rect: button_rect_for_dlu_y(233, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::ExitGame0x3ee,
                rect: button_rect_for_dlu_y(330, right_panel),
            },
        ],
        pressed_content_offset_x: 1,
        right_panel,
        lower_strip,
    }
}

fn scale_rect(rect: RectPx, scale_x: f32, scale_y: f32) -> RectPx {
    let x0 = (rect.x as f32 * scale_x).round() as i32;
    let y0 = (rect.y as f32 * scale_y).round() as i32;
    let x1 = ((rect.x + rect.w) as f32 * scale_x).round() as i32;
    let y1 = ((rect.y + rect.h) as f32 * scale_y).round() as i32;
    RectPx::new(x0, y0, (x1 - x0).max(0), (y1 - y0).max(0))
}

/// Compute the modern responsive shell layout.
///
/// The verified 800x600 dialog is kept as the logical coordinate space, then
/// stretched independently on X/Y to fill the swapchain. This intentionally
/// drifts from retail pixel parity, but keeps render and input coordinates in
/// the same window-pixel space.
pub fn compute_responsive_layout(screen_w: u32, screen_h: u32) -> MainMenuShellLayout {
    let base = compute_layout(SHELL_BASE_W as u32, SHELL_BASE_H as u32);
    let scale_x = screen_w as f32 / SHELL_BASE_W as f32;
    let scale_y = screen_h as f32 / SHELL_BASE_H as f32;
    let mut buttons = base.buttons;
    for button in &mut buttons {
        button.rect = scale_rect(button.rect, scale_x, scale_y);
    }
    let right_panel = RightPanelRects {
        top: scale_rect(base.right_panel.top, scale_x, scale_y),
        tile: scale_rect(base.right_panel.tile, scale_x, scale_y),
        tile_count: base.right_panel.tile_count,
        bottom: scale_rect(base.right_panel.bottom, scale_x, scale_y),
    };
    let lower_strip = scale_rect(base.lower_strip, scale_x, scale_y);

    MainMenuShellLayout {
        screen: RectPx::new(0, 0, screen_w as i32, screen_h as i32),
        movie_base: movie_base_for_screen_width(screen_w),
        movie: scale_rect(base.movie, scale_x, scale_y),
        title: scale_rect(base.title, scale_x, scale_y),
        version_line: scale_rect(base.version_line, scale_x, scale_y),
        tooltip_line: scale_rect(base.tooltip_line, scale_x, scale_y),
        website_static: scale_rect(base.website_static, scale_x, scale_y),
        buttons,
        pressed_content_offset_x: ((1.0 * scale_x).round() as i32).max(1),
        right_panel,
        lower_strip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_rects_match_800x600() {
        // Buttons snap to chrome tile rows (168x42). Tile 0 starts at the
        // bottom of SDTP (y=199). SP→tile 0, Exit→tile 8 (y=535) at 800x600.
        let layout = compute_layout(800, 600);
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsL);
        assert_eq!(layout.movie, RectPx::new(0, 0, 632, 570));
        assert_eq!(layout.buttons[0].rect, RectPx::new(632, 199, 168, 42));
        assert_eq!(layout.buttons[5].rect, RectPx::new(632, 535, 168, 42));
    }

    #[test]
    fn title_rect_matches_dlu_at_800x600() {
        // Right-anchor helper then main-menu heading nudge: top += 7, height += 1.
        let layout = compute_layout(800, 600);
        assert_eq!(layout.title, RectPx::new(635, 9, 162, 17));
    }

    #[test]
    fn tooltip_line_anchors_bottom_left_with_10_px_inset() {
        // DLU (2, 355, 303, 12) at 800x600 → pixel (3, 577, 455, 20).
        // Bottom-left layout pass: X = 0 + 10 = 10, Y = 600 - 20 - 0 - 1 = 579.
        let layout = compute_layout(800, 600);
        assert_eq!(layout.tooltip_line, RectPx::new(10, 579, 455, 20));
    }

    #[test]
    fn version_line_uses_sidebar_inset_and_bottom_cap_anchor() {
        // DLU (425, 357, 108, 10) at 800x600 → pixel (638, 580, 162, 16) raw.
        // Sidebar inset = (168 - 162) / 2 = 3 → final X = 800 - 3 - 162 = 635.
        // Bottom-cap anchor: right_panel.bottom = (632, 577, 168, 23);
        // bottom_edge = 600. Y = 600 - 16 = 584.
        let layout = compute_layout(800, 600);
        assert_eq!(layout.version_line, RectPx::new(635, 584, 162, 16));
    }

    #[test]
    fn key_rects_match_640x480_movie_choice() {
        let layout = compute_layout(640, 480);
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsS);
        assert_eq!(layout.movie, RectPx::new(0, 0, 472, 450));
    }

    #[test]
    fn large_screen_offsets_movie_without_scaling() {
        let layout = compute_layout(1024, 768);
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsL);
        assert_eq!(layout.movie, RectPx::new(112, 84, 632, 570));
        assert_eq!(layout.right_panel.top, RectPx::new(744, 84, 168, 199));
        assert_eq!(layout.right_panel.tile, RectPx::new(744, 283, 168, 42));
        assert_eq!(layout.right_panel.bottom, RectPx::new(744, 661, 168, 23));
        assert_eq!(layout.lower_strip, RectPx::new(112, 652, 632, 32));
        assert_eq!(layout.title, RectPx::new(747, 93, 162, 17));
    }

    #[test]
    fn large_screen_buttons_snap_to_centered_right_panel_tiles() {
        let layout = compute_layout(1024, 768);
        let expected_y = [283, 325, 367, 409, 451, 619];
        for (button, y) in layout.buttons.iter().zip(expected_y) {
            assert_eq!(button.rect, RectPx::new(744, y, 168, 42));
        }
    }

    #[test]
    fn responsive_layout_fills_window_by_scaling_base_shell() {
        let layout = compute_responsive_layout(1600, 900);
        assert_eq!(layout.screen, RectPx::new(0, 0, 1600, 900));
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsL);
        assert_eq!(layout.movie, RectPx::new(0, 0, 1264, 855));
        // Base buttons (632, 199, 168, 42) → at 2x/1.5x: (1264, 299, 336, 63).
        assert_eq!(layout.buttons[0].rect, RectPx::new(1264, 299, 336, 63));
        assert_eq!(layout.buttons[5].rect, RectPx::new(1264, 803, 336, 63));
        assert_eq!(layout.pressed_content_offset_x, 2);
    }

    #[test]
    fn responsive_layout_keeps_640_movie_asset_rule() {
        let layout = compute_responsive_layout(640, 480);
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsS);
        assert_eq!(layout.movie, RectPx::new(0, 0, 506, 456));
        // Base (632, 199, 168, 42) scaled by 0.8x/0.8x → (506, 159, 134, 34).
        assert_eq!(layout.buttons[0].rect, RectPx::new(506, 159, 134, 34));
    }
}
