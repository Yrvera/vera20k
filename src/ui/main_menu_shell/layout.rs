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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainMenuShellLayout {
    pub screen: RectPx,
    pub movie_base: MainMenuMovieBase,
    pub movie: RectPx,
    pub title: RectPx,
    pub website_static: RectPx,
    pub buttons: [MainMenuButtonRect; 6],
    pub pressed_content_offset_y: i32,
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

pub fn compute_layout(screen_w: u32, screen_h: u32) -> MainMenuShellLayout {
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;
    let movie_base = movie_base_for_screen_width(screen_w as u32);
    let (movie_x, movie_y) = movie_origin(screen_w, screen_h);
    let (movie_w, movie_h) = match movie_base {
        MainMenuMovieBase::Ra2tsS => (RA2TS_S_W, RA2TS_S_H),
        MainMenuMovieBase::Ra2tsL => (RA2TS_L_W, RA2TS_L_H),
    };
    MainMenuShellLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        movie_base,
        movie: RectPx::new(movie_x, movie_y, movie_w, movie_h),
        title: dlu_rect(425, 1, 108, 10),
        website_static: dlu_rect(447, 29, 61, 33),
        buttons: [
            MainMenuButtonRect {
                id: MainMenuControlId::SinglePlayer0x683,
                rect: dlu_rect(425, 125, 108, 23),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::WwOnline0x684,
                rect: dlu_rect(425, 152, 108, 23),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::Network0x578,
                rect: dlu_rect(425, 179, 108, 23),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::MoviesAndCredits0x686,
                rect: dlu_rect(425, 206, 108, 23),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::Options0x55c,
                rect: dlu_rect(425, 233, 108, 23),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::ExitGame0x3ee,
                rect: dlu_rect(425, 330, 108, 23),
            },
        ],
        pressed_content_offset_y: 2,
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

    MainMenuShellLayout {
        screen: RectPx::new(0, 0, screen_w as i32, screen_h as i32),
        movie_base: movie_base_for_screen_width(screen_w),
        movie: scale_rect(base.movie, scale_x, scale_y),
        title: scale_rect(base.title, scale_x, scale_y),
        website_static: scale_rect(base.website_static, scale_x, scale_y),
        buttons,
        pressed_content_offset_y: ((2.0 * scale_y).round() as i32).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_rects_match_800x600() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsL);
        assert_eq!(layout.movie, RectPx::new(0, 0, 632, 570));
        assert_eq!(layout.buttons[0].rect, RectPx::new(638, 203, 162, 37));
        assert_eq!(layout.buttons[5].rect, RectPx::new(638, 536, 162, 37));
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
        assert_eq!(layout.buttons[0].rect, RectPx::new(638, 203, 162, 37));
    }

    #[test]
    fn responsive_layout_fills_window_by_scaling_base_shell() {
        let layout = compute_responsive_layout(1600, 900);
        assert_eq!(layout.screen, RectPx::new(0, 0, 1600, 900));
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsL);
        assert_eq!(layout.movie, RectPx::new(0, 0, 1264, 855));
        assert_eq!(layout.buttons[0].rect, RectPx::new(1276, 305, 324, 55));
        assert_eq!(layout.buttons[5].rect, RectPx::new(1276, 804, 324, 56));
        assert_eq!(layout.pressed_content_offset_y, 3);
    }

    #[test]
    fn responsive_layout_keeps_640_movie_asset_rule() {
        let layout = compute_responsive_layout(640, 480);
        assert_eq!(layout.movie_base, MainMenuMovieBase::Ra2tsS);
        assert_eq!(layout.movie, RectPx::new(0, 0, 506, 456));
        assert_eq!(layout.buttons[0].rect, RectPx::new(510, 162, 130, 30));
    }
}
