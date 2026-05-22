//! Dialog 0x102 shell layout recovered from gamemd.exe.

pub const SHELL_BASE_W: i32 = 800;
pub const SHELL_BASE_H: i32 = 600;
pub const RIGHT_PANEL_WIDTH: i32 = 168;
pub const SDBTNANM_W: i32 = 156;
pub const SDBTNANM_H: i32 = 42;
pub const SDBTNBKGD_H: i32 = 42;

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
pub enum ShellControlId {
    StartGame0x617,
    ChooseMap0x5aa,
    Back0x5c0,
    MapPreview0x468,
    PlayerName0x6a0,
    PlayerColor0x6a2,
    AiColor0x522,
    AiColor0x523,
    AiColor0x524,
    AiColor0x525,
    AiColor0x526,
    AiColor0x527,
    AiColor0x528,
    Flag0x6da,
    Flag0x6db,
    Flag0x6dc,
    Flag0x6dd,
    Flag0x6de,
    Flag0x6df,
    Flag0x6e0,
    Flag0x6e1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorComboId {
    Player,
    Ai(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RightPanelRects {
    pub top: RectPx,
    pub tile: RectPx,
    pub tile_count: i32,
    pub bottom: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishTrackbarRects {
    pub game_speed: RectPx,
    pub credits: RectPx,
    pub unit_count: RectPx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishShellLayout {
    pub screen: RectPx,
    pub right_panel: RightPanelRects,
    pub start_button: RectPx,
    pub choose_map_button: RectPx,
    pub back_button: RectPx,
    pub map_preview: RectPx,
    pub player_name: RectPx,
    pub color_combos: [RectPx; 8],
    pub flags: [RectPx; 8],
    pub trackbars: SkirmishTrackbarRects,
}

const BASE_X: i32 = 6;
const BASE_Y: i32 = 13;

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
    ((screen - base) / 2).max(0)
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

fn owner_draw_button_snap(
    screen_w: i32,
    screen_h: i32,
    panel: RightPanelRects,
    original: RectPx,
) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    let offset_y = center_offset(screen_h, SHELL_BASE_H);
    let row = ((original.y + offset_y - panel.tile.y) / SDBTNBKGD_H).max(0);
    RectPx::new(
        screen_w - offset_x - SDBTNANM_W,
        panel.tile.y + row * SDBTNBKGD_H,
        SDBTNANM_W,
        SDBTNANM_H,
    )
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
    let top = RectPx::new(effective_right - RIGHT_PANEL_WIDTH, top_margin, 168, 199);
    let tile = RectPx::new(top.x, top.y + top.h, 168, SDBTNBKGD_H);
    let effective_h = if screen_h > 767 {
        screen_h - top_margin * 2
    } else {
        screen_h
    };
    let remaining = (effective_h - top.h).max(0);
    let tile_count = (remaining / SDBTNBKGD_H).min(9);
    let bottom_y = tile.y + tile_count * SDBTNBKGD_H;
    let bottom_h = screen_h - top_margin - bottom_y;
    RightPanelRects {
        top,
        tile,
        tile_count,
        bottom: RectPx::new(top.x, bottom_y, 168, bottom_h.max(0)),
    }
}

fn back_rect(screen_w: i32, panel: RightPanelRects) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    RectPx::new(
        screen_w - offset_x - SDBTNANM_W,
        panel.tile.y + (panel.tile_count - 1).max(0) * SDBTNBKGD_H,
        SDBTNANM_W,
        SDBTNANM_H,
    )
}

pub fn compute_layout(screen_w: u32, screen_h: u32) -> SkirmishShellLayout {
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;

    let start_base = dlu_rect(425, 149, 108, 23);
    let choose_base = dlu_rect(425, 176, 108, 23);
    let preview_base = dlu_rect(429, 23, 96, 69);
    let panel = right_panel_rects(screen_w, screen_h);
    let mut player_name = dlu_rect(38, 36, 100, 14);
    player_name.x += 1;
    player_name.w += 1;
    let mut unit_count_trackbar = dlu_rect(269, 210, 85, 13);
    unit_count_trackbar.y -= 1;
    // Other verified 0x102 one-pixel fixups target controls not represented here yet.

    let color_combos = [
        dlu_rect(282, 36, 29, 73),
        dlu_rect(282, 52, 29, 73),
        dlu_rect(282, 68, 29, 73),
        dlu_rect(282, 84, 29, 73),
        dlu_rect(282, 100, 29, 73),
        dlu_rect(282, 116, 29, 73),
        dlu_rect(282, 132, 29, 73),
        dlu_rect(282, 148, 29, 73),
    ];
    let flags = [
        dlu_rect(150, 36, 32, 12),
        dlu_rect(150, 52, 32, 12),
        dlu_rect(150, 68, 32, 12),
        dlu_rect(150, 84, 32, 12),
        dlu_rect(150, 100, 32, 12),
        dlu_rect(150, 116, 32, 12),
        dlu_rect(150, 132, 32, 12),
        dlu_rect(150, 148, 32, 12),
    ];

    SkirmishShellLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        right_panel: panel,
        start_button: owner_draw_button_snap(screen_w, screen_h, panel, start_base),
        choose_map_button: owner_draw_button_snap(screen_w, screen_h, panel, choose_base),
        back_button: back_rect(screen_w, panel),
        map_preview: right_anchor(screen_w, screen_h, preview_base),
        player_name,
        color_combos,
        flags,
        trackbars: SkirmishTrackbarRects {
            game_speed: dlu_rect(269, 176, 85, 13),
            credits: dlu_rect(269, 193, 85, 13),
            unit_count: unit_count_trackbar,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_layout, RectPx};

    #[test]
    fn key_rects_match_800x600() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.start_button, RectPx::new(644, 241, 156, 42));
        assert_eq!(layout.choose_map_button, RectPx::new(644, 283, 156, 42));
        assert_eq!(layout.map_preview, RectPx::new(644, 37, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(644, 535, 156, 42));
    }

    #[test]
    fn key_rects_match_1024x768() {
        let layout = compute_layout(1024, 768);
        assert_eq!(layout.start_button, RectPx::new(756, 325, 156, 42));
        assert_eq!(layout.choose_map_button, RectPx::new(756, 367, 156, 42));
        assert_eq!(layout.map_preview, RectPx::new(756, 121, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(756, 619, 156, 42));
    }

    #[test]
    fn key_rects_match_640x480_formula() {
        let layout = compute_layout(640, 480);
        assert_eq!(layout.start_button, RectPx::new(484, 241, 156, 42));
        assert_eq!(layout.choose_map_button, RectPx::new(484, 283, 156, 42));
        assert_eq!(layout.map_preview, RectPx::new(484, 37, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(484, 409, 156, 42));
    }

    #[test]
    fn represented_0102_one_pixel_fixups_are_applied() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.player_name, RectPx::new(58, 59, 151, 23));
        assert_eq!(layout.trackbars.unit_count, RectPx::new(404, 340, 128, 21));
    }

    #[test]
    fn color_combos_and_flags_do_not_right_anchor() {
        let layout_800 = compute_layout(800, 600);
        let layout_1024 = compute_layout(1024, 768);
        assert_eq!(layout_800.color_combos, layout_1024.color_combos);
        assert_eq!(layout_800.flags, layout_1024.flags);
        assert_eq!(layout_800.color_combos[0], RectPx::new(423, 59, 44, 119));
        assert_eq!(layout_800.flags[0], RectPx::new(225, 59, 48, 20));
    }

    #[test]
    fn trackbar_rects_match_800x600_final_geometry() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.trackbars.game_speed, RectPx::new(404, 286, 128, 21));
        assert_eq!(layout.trackbars.credits, RectPx::new(404, 314, 128, 21));
        assert_eq!(layout.trackbars.unit_count, RectPx::new(404, 340, 128, 21));
    }

    #[test]
    fn trackbars_preserve_ordinary_child_rects_at_1024() {
        let layout_800 = compute_layout(800, 600);
        let layout_1024 = compute_layout(1024, 768);
        assert_eq!(layout_1024.trackbars, layout_800.trackbars);
    }

    #[test]
    fn right_panel_globals_match_research_modes() {
        let a = compute_layout(800, 600);
        assert_eq!(a.right_panel.top, RectPx::new(632, 0, 168, 199));
        assert_eq!(a.right_panel.tile, RectPx::new(632, 199, 168, 42));
        assert_eq!(a.right_panel.tile_count, 9);
        assert_eq!(a.right_panel.bottom, RectPx::new(632, 577, 168, 23));

        let b = compute_layout(1024, 768);
        assert_eq!(b.right_panel.top, RectPx::new(744, 84, 168, 199));
        assert_eq!(b.right_panel.tile, RectPx::new(744, 283, 168, 42));
        assert_eq!(b.right_panel.tile_count, 9);
        assert_eq!(b.right_panel.bottom, RectPx::new(744, 661, 168, 23));

        let c = compute_layout(640, 480);
        assert_eq!(c.right_panel.top, RectPx::new(472, 0, 168, 199));
        assert_eq!(c.right_panel.tile, RectPx::new(472, 199, 168, 42));
        assert_eq!(c.right_panel.tile_count, 6);
        assert_eq!(c.right_panel.bottom, RectPx::new(472, 451, 168, 29));
    }

    #[test]
    fn large_screen_offsets_without_scaling() {
        let layout = compute_layout(1280, 960);
        assert_eq!(layout.start_button.w, 156);
        assert_eq!(layout.start_button.h, 42);
        assert_eq!(layout.map_preview.w, 144);
        assert_eq!(layout.map_preview.h, 112);
    }
}
