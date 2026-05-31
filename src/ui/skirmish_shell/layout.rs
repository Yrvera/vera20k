//! Dialog 0x102 shell layout recovered from gamemd.exe.

pub use crate::ui::shell::geom::{RectPx, RightPanelRects};
use crate::ui::shell::geom::{
    center_offset, dlu_rect, right_panel_rects, snap_button_biased_truncate,
};

pub const SHELL_BASE_W: i32 = 800;
pub const SHELL_BASE_H: i32 = 600;
pub const RIGHT_PANEL_WIDTH: i32 = crate::ui::shell::geom::RIGHT_PANEL_WIDTH;
pub const SDBTNANM_W: i32 = 156;
pub const SDBTNANM_H: i32 = crate::ui::shell::geom::SDBTNANM_CELL_H;
pub const SDBTNBKGD_H: i32 = crate::ui::shell::geom::RIGHT_PANEL_TILE_H;
pub const SKIRMISH_CHECKBOX_COUNT: usize = 5;
pub const CHECKBOX_ICON_W: i32 = 18;
pub const CHECKBOX_ICON_H: i32 = 18;
pub const CHECKBOX_TEXT_LEFT_OFFSET: i32 = 26;
pub const TRACKBAR_PLAQUE_W: i32 = 50;
pub const TRACKBAR_ACTIVE_WIDTH_SUBTRACT: i32 = 13;
pub const TRACKBAR_THUMB_W: i32 = 12;
pub const SKIRMISH_ROW_COUNT: usize = 8;
pub const SKIRMISH_AI_ROW_COUNT: usize = 7;
pub const COMBO_FACE_H: i32 = 24;
pub const COMBO_DROPDOWN_ROW_H: i32 = 23;
pub const COMBO_DROPDOWN_SCROLLBAR_W: i32 = 20;
pub const COMBO_DROPDOWN_SCROLLBAR_BUTTON_H: i32 = 22;
pub const COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H: i32 = 14;
pub const COMBO_ARROW_RESERVE_W: i32 = 20;
pub const COMBO_ARROW_X_FROM_RIGHT: i32 = 19;
pub const COMBO_ARROW_Y: i32 = 1;
pub const COMBO_TEXT_LEFT_INSET: i32 = 2;
pub const COMBO_SWATCH_INSET: i32 = 2;
pub const PLAYER_NAME_EDIT_CLIENT_INSET: i32 = 1;
pub const PLAYER_NAME_EDIT_TEXT_LEFT_INSET: i32 = 2;
pub const CHOOSE_MAP_MODAL_W: i32 = 533;
pub const CHOOSE_MAP_MODAL_H: i32 = 369;
pub const CHOOSE_MAP_LIST_ROW_H: i32 = 19;
pub const CHOOSE_MAP_LISTBOX_ROW_H: i32 = CHOOSE_MAP_LIST_ROW_H;
pub const CHOOSE_MAP_LISTBOX_SCROLLBAR_W: i32 = 20;
// Validation popup child pixel size. This is the MapDialogRect 6x13 candidate
// derived from the 300x200-DLU template; the exact post-creation client size
// (runtime DLU->pixel conversion) has not been captured, so treat as unconfirmed
// pending a native GetClientRect/screenshot.
pub const VALIDATION_MODAL_W: i32 = 450;
pub const VALIDATION_MODAL_H: i32 = 325;

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
pub enum SkirmishCheckboxId {
    ShortGame0x54e,
    McvRepacks0x693,
    CratesAppear0x696,
    SuperWeapons0x69a,
    BuildOffAlly0x69d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishCheckboxRect {
    pub id: SkirmishCheckboxId,
    pub rect: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishTrackbarId {
    GameSpeed0x529,
    Credits0x511,
    UnitCount0x50c,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishRightPanelTextRects {
    pub title: RectPx,
    pub game_type: RectPx,
    pub map_label: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishTrackbarRects {
    pub game_speed: RectPx,
    pub credits: RectPx,
    pub unit_count: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishTrackbarLabelRects {
    pub game_speed: RectPx,
    pub credits: RectPx,
    pub unit_count: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishColumnLabelRects {
    pub players: RectPx,
    pub side: RectPx,
    pub color: RectPx,
    pub start: RectPx,
    pub team: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishRowRects {
    pub ai_type_combos: [RectPx; SKIRMISH_AI_ROW_COUNT],
    pub side_combos: [RectPx; SKIRMISH_ROW_COUNT],
    pub start_combos: [RectPx; SKIRMISH_ROW_COUNT],
    pub team_combos: [RectPx; SKIRMISH_ROW_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishShellLayout {
    pub screen: RectPx,
    pub right_panel: RightPanelRects,
    pub right_panel_text: SkirmishRightPanelTextRects,
    pub start_button: RectPx,
    pub choose_map_button: RectPx,
    pub back_button: RectPx,
    pub map_preview: RectPx,
    pub column_labels: SkirmishColumnLabelRects,
    pub player_name: RectPx,
    pub rows: SkirmishRowRects,
    pub color_combos: [RectPx; 8],
    pub flags: [RectPx; 8],
    pub trackbars: SkirmishTrackbarRects,
    pub trackbar_labels: SkirmishTrackbarLabelRects,
    pub checkboxes: [SkirmishCheckboxRect; SKIRMISH_CHECKBOX_COUNT],
    pub status_help: RectPx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChooseMapModalButton {
    UseMap0x6c5,
    Cancel0x5c0,
    CreateRandomMap0x583,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChooseMapListboxId {
    Mode0x6eb,
    Map0x553,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseMapModalLayout {
    pub screen: RectPx,
    pub dialog: RectPx,
    pub mode_list: RectPx,
    pub map_list: RectPx,
    pub use_map_button: RectPx,
    pub cancel_button: RectPx,
    pub create_random_map_button: RectPx,
    pub title: RectPx,
    pub select_engagement: RectPx,
    pub game_type_heading: RectPx,
    pub game_map_heading: RectPx,
    pub status_help: RectPx,
    pub preview: RectPx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationModalLayout {
    pub screen: RectPx,
    pub dialog: RectPx,
    pub message: RectPx,
    pub ok_button: RectPx,
}

fn checkbox_dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx {
    dlu_rect(x, y, w, h)
}

const fn offset_rect_x(rect: RectPx, dx: i32) -> RectPx {
    RectPx::new(rect.x + dx, rect.y, rect.w, rect.h)
}

const fn translate_checkbox(rect: SkirmishCheckboxRect, dx: i32, dy: i32) -> SkirmishCheckboxRect {
    SkirmishCheckboxRect {
        id: rect.id,
        rect: rect.rect.translate(dx, dy),
    }
}

fn centered_fixed_shell_offset(screen_w: u32, screen_h: u32) -> (i32, i32) {
    (
        center_offset(screen_w as i32, SHELL_BASE_W),
        center_offset(screen_h as i32, SHELL_BASE_H),
    )
}

pub fn translate_layout(mut layout: SkirmishShellLayout, dx: i32, dy: i32) -> SkirmishShellLayout {
    layout.screen = layout.screen.translate(dx, dy);
    layout.right_panel.top = layout.right_panel.top.translate(dx, dy);
    layout.right_panel.tile = layout.right_panel.tile.translate(dx, dy);
    layout.right_panel.bottom = layout.right_panel.bottom.translate(dx, dy);
    layout.right_panel_text.title = layout.right_panel_text.title.translate(dx, dy);
    layout.right_panel_text.game_type = layout.right_panel_text.game_type.translate(dx, dy);
    layout.right_panel_text.map_label = layout.right_panel_text.map_label.translate(dx, dy);
    layout.start_button = layout.start_button.translate(dx, dy);
    layout.choose_map_button = layout.choose_map_button.translate(dx, dy);
    layout.back_button = layout.back_button.translate(dx, dy);
    layout.map_preview = layout.map_preview.translate(dx, dy);
    layout.column_labels.players = layout.column_labels.players.translate(dx, dy);
    layout.column_labels.side = layout.column_labels.side.translate(dx, dy);
    layout.column_labels.color = layout.column_labels.color.translate(dx, dy);
    layout.column_labels.start = layout.column_labels.start.translate(dx, dy);
    layout.column_labels.team = layout.column_labels.team.translate(dx, dy);
    layout.player_name = layout.player_name.translate(dx, dy);
    for rect in &mut layout.rows.ai_type_combos {
        *rect = rect.translate(dx, dy);
    }
    for rect in &mut layout.rows.side_combos {
        *rect = rect.translate(dx, dy);
    }
    for rect in &mut layout.rows.start_combos {
        *rect = rect.translate(dx, dy);
    }
    for rect in &mut layout.rows.team_combos {
        *rect = rect.translate(dx, dy);
    }
    for rect in &mut layout.color_combos {
        *rect = rect.translate(dx, dy);
    }
    for rect in &mut layout.flags {
        *rect = rect.translate(dx, dy);
    }
    layout.trackbars.game_speed = layout.trackbars.game_speed.translate(dx, dy);
    layout.trackbars.credits = layout.trackbars.credits.translate(dx, dy);
    layout.trackbars.unit_count = layout.trackbars.unit_count.translate(dx, dy);
    layout.trackbar_labels.game_speed = layout.trackbar_labels.game_speed.translate(dx, dy);
    layout.trackbar_labels.credits = layout.trackbar_labels.credits.translate(dx, dy);
    layout.trackbar_labels.unit_count = layout.trackbar_labels.unit_count.translate(dx, dy);
    for rect in &mut layout.checkboxes {
        *rect = translate_checkbox(*rect, dx, dy);
    }
    layout.status_help = layout.status_help.translate(dx, dy);
    layout
}

pub fn compute_fixed_800_layout(screen_w: u32, screen_h: u32) -> SkirmishShellLayout {
    let (dx, dy) = centered_fixed_shell_offset(screen_w, screen_h);
    translate_layout(
        compute_layout(SHELL_BASE_W as u32, SHELL_BASE_H as u32),
        dx,
        dy,
    )
}

pub const fn checkbox_icon_rect(rect: RectPx) -> RectPx {
    RectPx::new(rect.x, rect.y, CHECKBOX_ICON_W, CHECKBOX_ICON_H)
}

pub const fn checkbox_text_rect(rect: RectPx) -> RectPx {
    RectPx::new(
        rect.x + CHECKBOX_TEXT_LEFT_OFFSET,
        rect.y,
        rect.w - CHECKBOX_TEXT_LEFT_OFFSET,
        rect.h,
    )
}

pub const fn player_name_edit_client_rect(rect: RectPx) -> RectPx {
    RectPx::new(
        rect.x + PLAYER_NAME_EDIT_CLIENT_INSET,
        rect.y + PLAYER_NAME_EDIT_CLIENT_INSET,
        rect.w - PLAYER_NAME_EDIT_CLIENT_INSET * 2,
        rect.h - PLAYER_NAME_EDIT_CLIENT_INSET * 2,
    )
}

pub const fn player_name_edit_text_rect(rect: RectPx) -> RectPx {
    let client = player_name_edit_client_rect(rect);
    RectPx::new(
        client.x + PLAYER_NAME_EDIT_TEXT_LEFT_INSET,
        client.y,
        client.w - PLAYER_NAME_EDIT_TEXT_LEFT_INSET,
        client.h,
    )
}

pub const fn trackbar_plaque_rect(rect: RectPx) -> RectPx {
    RectPx::new(
        rect.x + rect.w - TRACKBAR_PLAQUE_W + 1,
        rect.y - 1,
        TRACKBAR_PLAQUE_W,
        rect.h,
    )
}

pub const fn trackbar_value_text_rect(rect: RectPx) -> RectPx {
    RectPx::new(rect.x + rect.w - 49, rect.y, 49, rect.h)
}

pub const fn trackbar_active_width(rect: RectPx) -> i32 {
    rect.w - TRACKBAR_PLAQUE_W - TRACKBAR_ACTIVE_WIDTH_SUBTRACT
}

pub fn trackbar_pixel_offset(value: i32, min: i32, max: i32, step: i32, rect: RectPx) -> i32 {
    let active_width = trackbar_active_width(rect).max(0);
    let span = max.saturating_sub(min);
    if active_width == 0 || span == 0 {
        return 0;
    }

    let step = step.max(1);
    let clamped = value.clamp(min, max);
    let quantized = min + ((clamped - min) / step) * step;
    ((quantized - min) * active_width) / span
}

pub const fn trackbar_thumb_rect(rect: RectPx, pixel_offset: i32) -> RectPx {
    RectPx::new(rect.x + 1 + pixel_offset, rect.y, TRACKBAR_THUMB_W, rect.h)
}

pub const fn combo_face_rect(rect: RectPx) -> RectPx {
    RectPx::new(rect.x, rect.y, rect.w, COMBO_FACE_H)
}

pub const fn combo_arrow_rect(rect: RectPx) -> RectPx {
    RectPx::new(
        rect.x + rect.w - COMBO_ARROW_X_FROM_RIGHT,
        rect.y + COMBO_ARROW_Y,
        0,
        0,
    )
}

pub const fn combo_text_rect(rect: RectPx) -> RectPx {
    let w = rect.w - COMBO_ARROW_RESERVE_W;
    RectPx::new(
        rect.x + COMBO_TEXT_LEFT_INSET,
        rect.y,
        if w > 0 { w } else { 0 },
        COMBO_FACE_H,
    )
}

pub const fn combo_swatch_rect(rect: RectPx) -> RectPx {
    let w = rect.w - COMBO_ARROW_RESERVE_W - COMBO_SWATCH_INSET * 2;
    let h = COMBO_FACE_H - COMBO_SWATCH_INSET * 2;
    RectPx::new(
        rect.x + COMBO_SWATCH_INSET,
        rect.y + COMBO_SWATCH_INSET,
        if w > 0 { w } else { 0 },
        if h > 0 { h } else { 0 },
    )
}

fn dialog_child(dialog: RectPx, local: RectPx) -> RectPx {
    RectPx::new(dialog.x + local.x, dialog.y + local.y, local.w, local.h)
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
    RectPx::new(offset_x + 10, screen_h - offset_y - 21, 615, 20)
}

fn choose_map_status_help_rect(screen_w: i32, screen_h: i32) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    let offset_y = center_offset(screen_h, SHELL_BASE_H);
    RectPx::new(offset_x + 10, screen_h - offset_y - 21, 455, 20)
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
    let right_panel_text = SkirmishRightPanelTextRects {
        title: RectPx::new(panel.top.x + 3, panel.top.y + 3, 162, 16),
        game_type: RectPx::new(panel.top.x + 17, panel.top.y + 167, 135, 16),
        map_label: RectPx::new(panel.top.x + 17, panel.top.y + 189, 135, 33),
    };
    let mut player_name = dlu_rect(38, 36, 100, 14);
    player_name.x += 1;
    player_name.w += 1;
    let mut unit_count_trackbar = dlu_rect(269, 210, 85, 13);
    unit_count_trackbar.y -= 1;

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
    let rows = SkirmishRowRects {
        ai_type_combos: [
            dlu_rect(39, 52, 100, 74),
            dlu_rect(39, 68, 100, 74),
            dlu_rect(39, 84, 100, 74),
            dlu_rect(39, 100, 100, 74),
            dlu_rect(39, 116, 100, 74),
            dlu_rect(39, 132, 100, 74),
            dlu_rect(39, 148, 100, 74),
        ],
        side_combos: [
            dlu_rect(191, 36, 78, 74),
            dlu_rect(191, 52, 78, 74),
            dlu_rect(191, 68, 78, 74),
            dlu_rect(191, 84, 78, 74),
            dlu_rect(191, 100, 78, 74),
            dlu_rect(191, 116, 78, 74),
            dlu_rect(191, 132, 78, 74),
            dlu_rect(191, 148, 78, 74),
        ],
        start_combos: [
            dlu_rect(324, 36, 25, 73),
            dlu_rect(324, 52, 25, 73),
            dlu_rect(324, 68, 25, 73),
            dlu_rect(324, 84, 25, 73),
            dlu_rect(324, 100, 25, 73),
            dlu_rect(324, 116, 25, 73),
            dlu_rect(324, 132, 25, 73),
            dlu_rect(324, 148, 25, 73),
        ],
        team_combos: [
            dlu_rect(364, 36, 25, 73),
            dlu_rect(364, 52, 25, 73),
            dlu_rect(364, 68, 25, 73),
            dlu_rect(364, 84, 25, 73),
            dlu_rect(364, 100, 25, 73),
            dlu_rect(364, 116, 25, 73),
            dlu_rect(364, 132, 25, 73),
            dlu_rect(364, 148, 25, 73),
        ],
    };

    SkirmishShellLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        right_panel: panel,
        right_panel_text,
        start_button: snap_button_biased_truncate(screen_w, screen_h, start_base, panel, SDBTNANM_W),
        choose_map_button: snap_button_biased_truncate(
            screen_w, screen_h, choose_base, panel, SDBTNANM_W,
        ),
        back_button: back_rect(screen_w, panel),
        map_preview: right_anchor(screen_w, screen_h, preview_base),
        column_labels: SkirmishColumnLabelRects {
            players: dlu_rect(39, 21, 97, 10),
            side: dlu_rect(191, 21, 73, 10),
            color: dlu_rect(283, 21, 42, 10),
            start: dlu_rect(325, 21, 34, 10),
            team: dlu_rect(363, 21, 34, 10),
        },
        player_name,
        rows,
        color_combos,
        flags,
        trackbars: SkirmishTrackbarRects {
            game_speed: dlu_rect(269, 176, 85, 13),
            credits: dlu_rect(269, 193, 85, 13),
            unit_count: unit_count_trackbar,
        },
        trackbar_labels: SkirmishTrackbarLabelRects {
            game_speed: RectPx::new(302, 286, 90, 16),
            credits: RectPx::new(302, 314, 90, 16),
            unit_count: RectPx::new(302, 341, 90, 16),
        },
        checkboxes: [
            SkirmishCheckboxRect {
                id: SkirmishCheckboxId::ShortGame0x54e,
                rect: offset_rect_x(checkbox_dlu_rect(48, 176, 100, 10), -1),
            },
            SkirmishCheckboxRect {
                id: SkirmishCheckboxId::McvRepacks0x693,
                rect: offset_rect_x(checkbox_dlu_rect(48, 193, 100, 10), -1),
            },
            SkirmishCheckboxRect {
                id: SkirmishCheckboxId::CratesAppear0x696,
                rect: offset_rect_x(checkbox_dlu_rect(48, 210, 100, 10), -1),
            },
            SkirmishCheckboxRect {
                id: SkirmishCheckboxId::SuperWeapons0x69a,
                rect: offset_rect_x(checkbox_dlu_rect(48, 228, 103, 10), -1),
            },
            SkirmishCheckboxRect {
                id: SkirmishCheckboxId::BuildOffAlly0x69d,
                rect: checkbox_dlu_rect(201, 227, 166, 11),
            },
        ],
        status_help: status_help_rect(screen_w, screen_h),
    }
}

pub fn compute_choose_map_modal_layout(screen_w: u32, screen_h: u32) -> ChooseMapModalLayout {
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;
    let panel = right_panel_rects(screen_w, screen_h);
    let use_map_base = dlu_rect(425, 122, 108, 23);
    let create_random_map_base = dlu_rect(425, 149, 108, 23);
    let preview_base = dlu_rect(428, 23, 96, 69);
    let title_base = dlu_rect(425, 1, 108, 10);

    ChooseMapModalLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        dialog: RectPx::new(0, 0, screen_w, screen_h),
        mode_list: dlu_rect(77, 78, 130, 211),
        map_list: dlu_rect(225, 78, 130, 211),
        use_map_button: snap_button_biased_truncate(
            screen_w, screen_h, use_map_base, panel, SDBTNANM_W,
        ),
        cancel_button: back_rect(screen_w, panel),
        create_random_map_button: snap_button_biased_truncate(
            screen_w,
            screen_h,
            create_random_map_base,
            panel,
            SDBTNANM_W,
        ),
        title: right_anchor(screen_w, screen_h, title_base).translate(0, 1),
        select_engagement: dlu_rect(80, 20, 257, 12),
        game_type_heading: dlu_rect(77, 60, 130, 10),
        game_map_heading: dlu_rect(225, 60, 130, 10),
        status_help: choose_map_status_help_rect(screen_w, screen_h),
        preview: right_anchor(screen_w, screen_h, preview_base),
    }
}

pub fn compute_fixed_800_choose_map_modal_layout(
    screen_w: u32,
    screen_h: u32,
) -> ChooseMapModalLayout {
    compute_choose_map_modal_layout(screen_w, screen_h)
}

pub const fn choose_map_listbox_rect(
    layout: &ChooseMapModalLayout,
    id: ChooseMapListboxId,
) -> RectPx {
    match id {
        ChooseMapListboxId::Mode0x6eb => layout.mode_list,
        ChooseMapListboxId::Map0x553 => layout.map_list,
    }
}

pub fn choose_map_listbox_visible_row_count(rect: RectPx) -> usize {
    (rect.h / CHOOSE_MAP_LISTBOX_ROW_H).max(0) as usize
}

pub fn choose_map_listbox_needs_scrollbar(row_count: usize, rect: RectPx) -> bool {
    row_count > choose_map_listbox_visible_row_count(rect)
}

pub fn choose_map_listbox_scrollbar_rect(row_count: usize, rect: RectPx) -> Option<RectPx> {
    if !choose_map_listbox_needs_scrollbar(row_count, rect) {
        return None;
    }
    Some(RectPx::new(
        rect.x + rect.w - CHOOSE_MAP_LISTBOX_SCROLLBAR_W,
        rect.y,
        CHOOSE_MAP_LISTBOX_SCROLLBAR_W,
        rect.h,
    ))
}

pub fn choose_map_listbox_content_rect(row_count: usize, rect: RectPx) -> RectPx {
    let scrollbar_w = if choose_map_listbox_needs_scrollbar(row_count, rect) {
        CHOOSE_MAP_LISTBOX_SCROLLBAR_W
    } else {
        0
    };
    RectPx::new(rect.x, rect.y, (rect.w - scrollbar_w).max(0), rect.h)
}

pub fn choose_map_listbox_row_rect(content: RectPx, visible_row: usize) -> RectPx {
    let y = content.y + visible_row as i32 * CHOOSE_MAP_LISTBOX_ROW_H;
    RectPx::new(
        content.x,
        y,
        content.w,
        CHOOSE_MAP_LISTBOX_ROW_H
            .min(content.y + content.h - y)
            .max(0),
    )
}

pub fn choose_map_listbox_max_top_index(row_count: usize, rect: RectPx) -> usize {
    row_count.saturating_sub(choose_map_listbox_visible_row_count(rect))
}

pub fn choose_map_listbox_scroll_thumb_rect(
    row_count: usize,
    top_index: usize,
    rect: RectPx,
) -> Option<RectPx> {
    let scrollbar = choose_map_listbox_scrollbar_rect(row_count, rect)?;
    let visible_rows = choose_map_listbox_visible_row_count(rect);
    if row_count == 0 || visible_rows == 0 {
        return None;
    }
    let track_h = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2).max(1);
    let thumb_h = ((track_h * visible_rows as i32) / row_count as i32)
        .max(COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H)
        .min(track_h);
    let max_top = choose_map_listbox_max_top_index(row_count, rect);
    let track_span = (track_h - thumb_h).max(1);
    let thumb_y = scrollbar.y
        + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H
        + if max_top == 0 {
            0
        } else {
            (track_span * top_index.min(max_top) as i32) / max_top as i32
        };
    Some(RectPx::new(scrollbar.x, thumb_y, scrollbar.w, thumb_h))
}

pub fn choose_map_listbox_top_index_from_track_click(
    row_count: usize,
    top_index: usize,
    rect: RectPx,
    mouse_y: i32,
) -> Option<usize> {
    let scrollbar = choose_map_listbox_scrollbar_rect(row_count, rect)?;
    let thumb = choose_map_listbox_scroll_thumb_rect(row_count, top_index, rect)?;
    let max_top = choose_map_listbox_max_top_index(row_count, rect);
    if max_top == 0 {
        return Some(0);
    }
    let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb.h).max(1);
    let thumb_top = (mouse_y - thumb.h / 2).clamp(
        scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
        scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H - thumb.h,
    );
    let local = thumb_top - scrollbar.y - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H;
    Some(((local * max_top as i32 + track_span / 2) / track_span) as usize)
}

pub fn choose_map_modal_button_at(
    layout: &ChooseMapModalLayout,
    x: i32,
    y: i32,
) -> Option<ChooseMapModalButton> {
    if layout.use_map_button.contains(x, y) {
        return Some(ChooseMapModalButton::UseMap0x6c5);
    }
    if layout.cancel_button.contains(x, y) {
        return Some(ChooseMapModalButton::Cancel0x5c0);
    }
    if layout.create_random_map_button.contains(x, y) {
        return Some(ChooseMapModalButton::CreateRandomMap0x583);
    }
    None
}

pub fn choose_map_modal_list_row_at(list: RectPx, x: i32, y: i32) -> Option<usize> {
    if !list.contains(x, y) {
        return None;
    }
    Some(((y - list.y) / CHOOSE_MAP_LISTBOX_ROW_H) as usize)
}

pub fn choose_map_listbox_row_at(
    list: RectPx,
    row_count: usize,
    top_index: usize,
    x: i32,
    y: i32,
) -> Option<usize> {
    let content = choose_map_listbox_content_rect(row_count, list);
    if !content.contains(x, y) {
        return None;
    }
    let idx = top_index + ((y - content.y) / CHOOSE_MAP_LISTBOX_ROW_H) as usize;
    (idx < row_count).then_some(idx)
}

/// Center a native child dialog against the LIVE screen, matching the engine's
/// child-window centering: x/y = max(0, ((screen - child) + 1) / 2). The +1 term
/// biases odd-size centering by one pixel and the max(0) clamps negatives — both
/// observable, so they are reproduced exactly. The validation popup parents to
/// the top-level window, so it centers on the real screen, NOT inside the
/// fixed 800x600 shell box.
fn centered_live_screen_dialog(screen_w: i32, screen_h: i32, w: i32, h: i32) -> RectPx {
    RectPx::new(
        (((screen_w - w) + 1) / 2).max(0),
        (((screen_h - h) + 1) / 2).max(0),
        w,
        h,
    )
}

pub fn compute_validation_modal_layout(screen_w: u32, screen_h: u32) -> ValidationModalLayout {
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;
    let dialog =
        centered_live_screen_dialog(screen_w, screen_h, VALIDATION_MODAL_W, VALIDATION_MODAL_H);

    ValidationModalLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        dialog,
        message: dialog_child(dialog, dlu_rect(40, 40, 220, 50)),
        ok_button: dialog_child(dialog, dlu_rect(207, 175, 83, 15)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CHOOSE_MAP_LIST_ROW_H, COMBO_DROPDOWN_SCROLLBAR_BUTTON_H, ChooseMapModalButton, RectPx,
        SkirmishCheckboxId, checkbox_icon_rect, checkbox_text_rect,
        choose_map_listbox_content_rect, choose_map_listbox_row_at,
        choose_map_listbox_scroll_thumb_rect, choose_map_listbox_scrollbar_rect,
        choose_map_listbox_top_index_from_track_click, choose_map_modal_button_at,
        choose_map_modal_list_row_at, combo_arrow_rect, combo_face_rect, combo_swatch_rect,
        combo_text_rect, compute_choose_map_modal_layout,
        compute_fixed_800_choose_map_modal_layout, compute_fixed_800_layout, compute_layout,
        compute_validation_modal_layout, player_name_edit_client_rect, player_name_edit_text_rect,
        trackbar_active_width, trackbar_pixel_offset, trackbar_plaque_rect, trackbar_thumb_rect,
        trackbar_value_text_rect,
    };

    struct ExpectedRect {
        name: &'static str,
        actual: RectPx,
        expected: RectPx,
    }

    #[test]
    fn key_rects_match_800x600() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.start_button, RectPx::new(644, 241, 156, 42));
        assert_eq!(layout.choose_map_button, RectPx::new(644, 283, 156, 42));
        assert_eq!(layout.map_preview, RectPx::new(644, 37, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(644, 535, 156, 42));
    }

    #[test]
    fn fixed_800_layout_centers_native_shell_without_rescaling() {
        let layout = compute_fixed_800_layout(1024, 768);

        assert_eq!(layout.screen, RectPx::new(112, 84, 800, 600));
        assert_eq!(layout.start_button, RectPx::new(756, 325, 156, 42));
        assert_eq!(layout.choose_map_button, RectPx::new(756, 367, 156, 42));
        assert_eq!(layout.map_preview, RectPx::new(756, 121, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(756, 619, 156, 42));
        assert_eq!(layout.column_labels.players, RectPx::new(171, 118, 146, 16));
        assert_eq!(
            layout.rows.ai_type_combos[0],
            RectPx::new(171, 169, 150, 120)
        );
    }

    #[test]
    fn fixed_800_choose_map_modal_uses_verified_0x6b_shell_helpers() {
        let layout = compute_fixed_800_choose_map_modal_layout(1024, 768);

        assert_eq!(layout.screen, RectPx::new(0, 0, 1024, 768));
        assert_eq!(layout.dialog, RectPx::new(0, 0, 1024, 768));
        assert_eq!(layout.mode_list, RectPx::new(116, 127, 195, 343));
        assert_eq!(layout.map_list, RectPx::new(338, 127, 195, 343));
        assert_eq!(layout.use_map_button, RectPx::new(756, 283, 156, 42));
        assert_eq!(
            layout.create_random_map_button,
            RectPx::new(756, 325, 156, 42)
        );
        assert_eq!(layout.cancel_button, RectPx::new(756, 619, 156, 42));
        assert_eq!(layout.status_help, RectPx::new(122, 663, 455, 20));
    }

    #[test]
    fn key_rects_match_1024x768() {
        let layout = compute_layout(1024, 768);
        let expected = [
            ExpectedRect {
                name: "right_panel.top",
                actual: layout.right_panel.top,
                expected: RectPx::new(744, 84, 168, 199),
            },
            ExpectedRect {
                name: "right_panel.tile",
                actual: layout.right_panel.tile,
                expected: RectPx::new(744, 283, 168, 42),
            },
            ExpectedRect {
                name: "right_panel.bottom",
                actual: layout.right_panel.bottom,
                expected: RectPx::new(744, 661, 168, 23),
            },
            ExpectedRect {
                name: "start_button 0x617",
                actual: layout.start_button,
                expected: RectPx::new(756, 325, 156, 42),
            },
            ExpectedRect {
                name: "choose_map_button 0x5AA",
                actual: layout.choose_map_button,
                expected: RectPx::new(756, 367, 156, 42),
            },
            ExpectedRect {
                name: "map_preview 0x468",
                actual: layout.map_preview,
                expected: RectPx::new(756, 121, 144, 112),
            },
            ExpectedRect {
                name: "back_button 0x5C0",
                actual: layout.back_button,
                expected: RectPx::new(756, 619, 156, 42),
            },
        ];

        for rect in expected {
            assert_eq!(rect.actual, rect.expected, "{}", rect.name);
        }
        assert_eq!(layout.right_panel.tile_count, 9);
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
    fn represented_0102_player_name_one_pixel_fixup_is_applied() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.player_name, RectPx::new(58, 59, 151, 23));
    }

    #[test]
    fn player_name_edit_text_rect_uses_verified_client_and_text_insets() {
        let layout = compute_layout(800, 600);
        assert_eq!(
            player_name_edit_client_rect(layout.player_name),
            RectPx::new(59, 60, 149, 21)
        );
        assert_eq!(
            player_name_edit_text_rect(layout.player_name),
            RectPx::new(61, 60, 147, 21)
        );
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
    fn row_combo_rects_match_800x600_resource_geometry() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.column_labels.players, RectPx::new(59, 34, 146, 16));
        assert_eq!(layout.rows.ai_type_combos[0], RectPx::new(59, 85, 150, 120));
        assert_eq!(layout.rows.side_combos[0], RectPx::new(287, 59, 117, 120));
        assert_eq!(layout.color_combos[0], RectPx::new(423, 59, 44, 119));
        assert_eq!(layout.rows.start_combos[0], RectPx::new(486, 59, 38, 119));
        assert_eq!(layout.rows.team_combos[0], RectPx::new(546, 59, 38, 119));
    }

    #[test]
    fn collapsed_combo_helpers_follow_owner_draw_constants() {
        let rect = RectPx::new(423, 59, 44, 119);

        assert_eq!(combo_face_rect(rect), RectPx::new(423, 59, 44, 24));
        assert_eq!(combo_arrow_rect(rect), RectPx::new(448, 60, 0, 0));
        assert_eq!(combo_text_rect(rect), RectPx::new(425, 59, 24, 24));
        assert_eq!(combo_swatch_rect(rect), RectPx::new(425, 61, 20, 20));
    }

    #[test]
    fn trackbar_rects_match_800x600_final_geometry() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.trackbars.game_speed, RectPx::new(404, 286, 128, 21));
        assert_eq!(layout.trackbars.credits, RectPx::new(404, 314, 128, 21));
        assert_eq!(layout.trackbars.unit_count, RectPx::new(404, 340, 128, 21));
        assert_eq!(
            trackbar_plaque_rect(layout.trackbars.game_speed),
            RectPx::new(483, 285, 50, 21)
        );
        assert_eq!(
            trackbar_plaque_rect(layout.trackbars.credits),
            RectPx::new(483, 313, 50, 21)
        );
        assert_eq!(
            trackbar_plaque_rect(layout.trackbars.unit_count),
            RectPx::new(483, 339, 50, 21)
        );
    }

    #[test]
    fn option_label_static_rects_preserve_resource_positions() {
        for layout in [
            compute_layout(640, 480),
            compute_layout(800, 600),
            compute_layout(1024, 768),
        ] {
            assert_eq!(
                layout.trackbar_labels.game_speed,
                RectPx::new(302, 286, 90, 16)
            );
            assert_eq!(
                layout.trackbar_labels.credits,
                RectPx::new(302, 314, 90, 16)
            );
            assert_eq!(
                layout.trackbar_labels.unit_count,
                RectPx::new(302, 341, 90, 16)
            );
        }
    }

    #[test]
    fn status_help_strip_0x695_bottom_left_rects() {
        assert_eq!(
            compute_layout(640, 480).status_help,
            RectPx::new(10, 459, 615, 20)
        );
        assert_eq!(
            compute_layout(800, 600).status_help,
            RectPx::new(10, 579, 615, 20)
        );
        assert_eq!(
            compute_layout(1024, 768).status_help,
            RectPx::new(122, 663, 615, 20)
        );
    }

    #[test]
    fn skirmish_unit_count_trackbar_applies_0102_fixup_y_minus_one() {
        let layout_800 = compute_layout(800, 600);
        let layout_1024 = compute_layout(1024, 768);
        assert_eq!(layout_1024.trackbars, layout_800.trackbars);
        assert_eq!(
            layout_800.trackbars.unit_count,
            RectPx::new(404, 340, 128, 21)
        );
        assert_eq!(
            layout_1024.trackbars.unit_count,
            RectPx::new(404, 340, 128, 21)
        );
    }

    #[test]
    fn checkbox_rects_match_800x600_final_geometry() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.checkboxes.len(), 5);
        assert_eq!(layout.checkboxes[0].id, SkirmishCheckboxId::ShortGame0x54e);
        assert_eq!(layout.checkboxes[0].rect, RectPx::new(71, 286, 150, 16));
        assert_eq!(layout.checkboxes[1].id, SkirmishCheckboxId::McvRepacks0x693);
        assert_eq!(layout.checkboxes[1].rect, RectPx::new(71, 314, 150, 16));
        assert_eq!(
            layout.checkboxes[2].id,
            SkirmishCheckboxId::CratesAppear0x696
        );
        assert_eq!(layout.checkboxes[2].rect, RectPx::new(71, 341, 150, 16));
        assert_eq!(
            layout.checkboxes[3].id,
            SkirmishCheckboxId::SuperWeapons0x69a
        );
        assert_eq!(layout.checkboxes[3].rect, RectPx::new(71, 371, 155, 16));
        assert_eq!(
            layout.checkboxes[4].id,
            SkirmishCheckboxId::BuildOffAlly0x69d
        );
        assert_eq!(layout.checkboxes[4].rect, RectPx::new(302, 369, 249, 18));
    }

    #[test]
    fn checkbox_rects_match_640x480_final_geometry() {
        let layout = compute_layout(640, 480);
        assert_eq!(layout.checkboxes[0].rect, RectPx::new(71, 286, 150, 16));
        assert_eq!(layout.checkboxes[1].rect, RectPx::new(71, 314, 150, 16));
        assert_eq!(layout.checkboxes[2].rect, RectPx::new(71, 341, 150, 16));
        assert_eq!(layout.checkboxes[3].rect, RectPx::new(71, 371, 155, 16));
        assert_eq!(layout.checkboxes[4].rect, RectPx::new(302, 369, 249, 18));
    }

    #[test]
    fn skirmish_option_checkboxes_apply_0102_fixup_x_minus_one() {
        let layout_800 = compute_layout(800, 600);
        let layout_1024 = compute_layout(1024, 768);

        for layout in [layout_800, layout_1024] {
            assert_eq!(layout.checkboxes[0].rect.x, 71);
            assert_eq!(layout.checkboxes[1].rect.x, 71);
            assert_eq!(layout.checkboxes[2].rect.x, 71);
            assert_eq!(layout.checkboxes[3].rect.x, 71);
            assert_eq!(layout.checkboxes[4].rect.x, 302);
        }
    }

    #[test]
    fn checkbox_icon_and_text_rects_follow_owner_draw_constants() {
        let rect = RectPx::new(71, 286, 150, 16);

        assert_eq!(checkbox_icon_rect(rect), RectPx::new(71, 286, 18, 18));
        assert_eq!(checkbox_text_rect(rect).x, rect.x + 26);
    }

    #[test]
    fn trackbar_geometry_helpers_follow_owner_draw_constants() {
        let rect = RectPx::new(404, 286, 128, 21);

        assert_eq!(trackbar_active_width(rect), 65);
        assert_eq!(
            trackbar_plaque_rect(RectPx::new(0, 0, 128, 21)),
            RectPx::new(79, -1, 50, 21)
        );
        assert_eq!(trackbar_plaque_rect(rect), RectPx::new(483, 285, 50, 21));
        assert_eq!(
            trackbar_value_text_rect(rect),
            RectPx::new(483, 286, 49, 21)
        );
        assert_eq!(trackbar_thumb_rect(rect, 0), RectPx::new(405, 286, 12, 21));
        assert_eq!(trackbar_thumb_rect(rect, 65), RectPx::new(470, 286, 12, 21));
    }

    #[test]
    fn trackbar_pixel_offset_uses_integer_endpoints() {
        let rect = RectPx::new(404, 286, 128, 21);

        assert_eq!(trackbar_pixel_offset(0, 0, 6, 1, rect), 0);
        assert_eq!(trackbar_pixel_offset(6, 0, 6, 1, rect), 65);
        assert_eq!(trackbar_pixel_offset(10000, 5000, 10000, 100, rect), 65);
    }

    #[test]
    fn right_panel_globals_match_research_modes() {
        let a = compute_layout(800, 600);
        assert_eq!(a.right_panel.top, RectPx::new(632, 0, 168, 199));
        assert_eq!(a.right_panel.tile, RectPx::new(632, 199, 168, 42));
        assert_eq!(a.right_panel.tile_count, 9);
        assert_eq!(a.right_panel.bottom, RectPx::new(632, 577, 168, 23));
        assert_eq!(a.right_panel_text.title, RectPx::new(635, 3, 162, 16));
        assert_eq!(a.right_panel_text.game_type, RectPx::new(649, 167, 135, 16));
        assert_eq!(a.right_panel_text.map_label, RectPx::new(649, 189, 135, 33));

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
        assert_eq!(c.right_panel_text.title, RectPx::new(475, 3, 162, 16));
        assert_eq!(c.right_panel_text.game_type, RectPx::new(489, 167, 135, 16));
        assert_eq!(c.right_panel_text.map_label, RectPx::new(489, 189, 135, 33));
    }

    #[test]
    fn large_screen_offsets_without_scaling() {
        let layout = compute_layout(1280, 960);
        assert_eq!(layout.start_button.w, 156);
        assert_eq!(layout.start_button.h, 42);
        assert_eq!(layout.map_preview.w, 144);
        assert_eq!(layout.map_preview.h, 112);
    }

    #[test]
    fn choose_map_modal_layout_matches_verified_0x6b_geometry() {
        let layout = compute_choose_map_modal_layout(800, 600);

        assert_eq!(layout.screen, RectPx::new(0, 0, 800, 600));
        assert_eq!(layout.dialog, RectPx::new(0, 0, 800, 600));
        assert_eq!(layout.mode_list, RectPx::new(116, 127, 195, 343));
        assert_eq!(layout.map_list, RectPx::new(338, 127, 195, 343));
        assert_eq!(layout.use_map_button, RectPx::new(644, 199, 156, 42));
        assert_eq!(
            layout.create_random_map_button,
            RectPx::new(644, 241, 156, 42)
        );
        assert_eq!(layout.cancel_button, RectPx::new(644, 535, 156, 42));
        assert_eq!(layout.title, RectPx::new(635, 3, 162, 16));
        assert_eq!(layout.select_engagement, RectPx::new(120, 33, 386, 20));
        assert_eq!(layout.game_type_heading, RectPx::new(116, 98, 195, 16));
        assert_eq!(layout.game_map_heading, RectPx::new(338, 98, 195, 16));
        assert_eq!(layout.status_help, RectPx::new(10, 579, 455, 20));
        assert_eq!(layout.preview, RectPx::new(644, 37, 144, 112));
    }

    #[test]
    fn choose_map_modal_high_res_preserves_lists_and_offsets_shell_helpers() {
        let layout = compute_choose_map_modal_layout(1024, 768);

        assert_eq!(layout.screen, RectPx::new(0, 0, 1024, 768));
        assert_eq!(layout.dialog, RectPx::new(0, 0, 1024, 768));
        assert_eq!(layout.mode_list, RectPx::new(116, 127, 195, 343));
        assert_eq!(layout.map_list, RectPx::new(338, 127, 195, 343));
        assert_eq!(layout.use_map_button, RectPx::new(756, 283, 156, 42));
        assert_eq!(
            layout.create_random_map_button,
            RectPx::new(756, 325, 156, 42)
        );
        assert_eq!(layout.cancel_button, RectPx::new(756, 619, 156, 42));
        assert_eq!(layout.title, RectPx::new(747, 87, 162, 16));
        assert_eq!(layout.select_engagement, RectPx::new(120, 33, 386, 20));
        assert_eq!(layout.game_type_heading, RectPx::new(116, 98, 195, 16));
        assert_eq!(layout.game_map_heading, RectPx::new(338, 98, 195, 16));
        assert_eq!(layout.status_help, RectPx::new(122, 663, 455, 20));
        assert_eq!(layout.preview, RectPx::new(756, 121, 144, 112));
    }

    #[test]
    fn validation_modal_layout_centers_ok_button() {
        let layout = compute_validation_modal_layout(800, 600);

        // Single-center against the live screen with the +1 odd-size bias:
        // x = (800-450+1)/2 = 175, y = (600-325+1)/2 = 138.
        assert_eq!(layout.dialog, RectPx::new(175, 138, 450, 325));
        assert_eq!(layout.message, RectPx::new(235, 203, 330, 81));
        assert_eq!(layout.ok_button, RectPx::new(486, 422, 125, 24));
    }

    #[test]
    fn validation_modal_centers_against_live_screen_not_800_box() {
        // At 1024x768 the modal centers on the real screen, not inside an 800x600
        // sub-box: x = (1024-450+1)/2 = 287, y = (768-325+1)/2 = 222.
        let layout = compute_validation_modal_layout(1024, 768);
        assert_eq!(layout.dialog, RectPx::new(287, 222, 450, 325));
    }

    #[test]
    fn choose_map_modal_button_hit_test_uses_control_rects() {
        let layout = compute_choose_map_modal_layout(800, 600);

        assert_eq!(
            choose_map_modal_button_at(&layout, layout.use_map_button.x, layout.use_map_button.y),
            Some(ChooseMapModalButton::UseMap0x6c5)
        );
        assert_eq!(
            choose_map_modal_button_at(&layout, layout.cancel_button.x, layout.cancel_button.y),
            Some(ChooseMapModalButton::Cancel0x5c0)
        );
        assert_eq!(
            choose_map_modal_button_at(
                &layout,
                layout.create_random_map_button.x,
                layout.create_random_map_button.y
            ),
            Some(ChooseMapModalButton::CreateRandomMap0x583)
        );
        assert_eq!(
            choose_map_modal_button_at(&layout, layout.dialog.x, layout.dialog.y),
            None
        );
        assert_eq!(
            choose_map_modal_button_at(&layout, layout.dialog.x + 374, layout.dialog.y + 80),
            None
        );
        assert_eq!(
            choose_map_modal_button_at(&layout, layout.dialog.x + 374, layout.dialog.y + 116),
            None
        );
        assert_eq!(
            choose_map_modal_button_at(&layout, layout.dialog.x + 374, layout.dialog.y + 152),
            None
        );
    }

    #[test]
    fn choose_map_modal_list_hit_test_uses_verified_owner_draw_row_height() {
        let layout = compute_choose_map_modal_layout(800, 600);

        assert_eq!(
            choose_map_modal_list_row_at(layout.map_list, layout.map_list.x, layout.map_list.y),
            Some(0)
        );
        assert_eq!(
            choose_map_modal_list_row_at(
                layout.map_list,
                layout.map_list.x,
                layout.map_list.y + CHOOSE_MAP_LIST_ROW_H
            ),
            Some(1)
        );
        assert_eq!(
            choose_map_modal_list_row_at(
                layout.map_list,
                layout.map_list.x,
                layout.map_list.y + layout.map_list.h
            ),
            None
        );
    }

    #[test]
    fn choose_map_modal_listbox_hit_testing_reserves_scrollbar_width() {
        let layout = compute_choose_map_modal_layout(800, 600);
        let rows = 20;
        let scrollbar = choose_map_listbox_scrollbar_rect(rows, layout.map_list).unwrap();
        let content = choose_map_listbox_content_rect(rows, layout.map_list);

        assert_eq!(scrollbar, RectPx::new(513, 127, 20, 343));
        assert_eq!(content, RectPx::new(338, 127, 175, 343));
        assert_eq!(
            choose_map_listbox_row_at(layout.map_list, rows, 5, content.x + 2, content.y),
            Some(5)
        );
        assert_eq!(
            choose_map_listbox_row_at(layout.map_list, rows, 5, scrollbar.x, scrollbar.y),
            None
        );
    }

    #[test]
    fn choose_map_modal_scrollbar_thumb_and_track_map_to_top_index() {
        let layout = compute_choose_map_modal_layout(800, 600);
        let rows = 20;
        let scrollbar = choose_map_listbox_scrollbar_rect(rows, layout.map_list).unwrap();
        let thumb = choose_map_listbox_scroll_thumb_rect(rows, 3, layout.map_list).unwrap();

        assert_eq!(thumb.w, scrollbar.w);
        assert!(thumb.y >= scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H);
        assert!(thumb.y + thumb.h <= scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H);
        assert_eq!(
            choose_map_listbox_top_index_from_track_click(
                rows,
                0,
                layout.map_list,
                scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H
            ),
            Some(0)
        );
        assert_eq!(
            choose_map_listbox_top_index_from_track_click(
                rows,
                0,
                layout.map_list,
                scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H - 1
            ),
            Some(2)
        );
    }
}
