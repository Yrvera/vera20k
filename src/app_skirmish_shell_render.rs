//! Skirmish shell sprite construction and render pass.
//!
//! Part of the app layer: may depend on ui and render modules. Keeps the
//! `GameScreen::MainMenu` branch in `app.rs` small.

use std::sync::Once;

use crate::app::AppState;
use crate::app_init::MapMenuEntry;
use crate::map::preview::PreviewSourceBounds;
use crate::render::batch::{BatchTexture, SpriteInstance};
use crate::render::shell_text::{self, ShellAlign, ShellTextDraw, TextRect};
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::rules::house_colors::{HouseColorIndex, house_color_ramp};
use crate::ui::main_menu::SkirmishCountry;
use crate::ui::skirmish_shell::{
    CHOOSE_MAP_LIST_ROW_H, COMBO_DROPDOWN_ROW_H, COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
    ChooseMapModalLayout, DropdownScrollbarPart, OwnerDrawButton, RectPx, SkirmishAiRowType,
    SkirmishCheckboxId, SkirmishComboId, SkirmishComboItem, SkirmishCountryChoice,
    SkirmishShellAction, SkirmishShellLayout, SkirmishShellState, SkirmishTrackbarId,
    checkbox_icon_rect, checkbox_text_rect, combo_arrow_rect, combo_dropdown_content_rect,
    combo_dropdown_needs_scrollbar, combo_dropdown_rect, combo_dropdown_scroll_thumb_rect,
    combo_dropdown_scrollbar_rect, combo_dropdown_visible_row_count, combo_face_rect, combo_items,
    combo_swatch_rect, combo_text_rect, compute_choose_map_modal_layout, compute_layout,
    selected_combo_item_index, trackbar_pixel_offset, trackbar_plaque_rect, trackbar_thumb_rect,
    trackbar_value_text_rect, trackbar_visual_value,
};

static HIGH_RES_PARENT_BACKGROUND_LOG: Once = Once::new();

const PRESSED_BUTTON_CONTENT_OFFSET_Y: i32 = 2;
const START_MARKER_OFFSET_X: i32 = -9;
const START_MARKER_OFFSET_Y: i32 = -6;
const BUTTON_DISABLED_ALPHA: f32 = 0x80 as f32 / 255.0;
const SHELL_PARENT_BACKGROUND_DEPTH: f32 = 0.00090;
const SHELL_LOWER_STRIP_DEPTH: f32 = 0.00077;
const SHELL_PREVIEW_SURFACE_DEPTH: f32 = 0.00058;
const SHELL_CONTROL_DEPTH: f32 = 0.00055;
const SHELL_CONTROL_TEXT_DEPTH: f32 = 0.00039;
const SHELL_SWATCH_DEPTH: f32 = 0.00054;
const SHELL_DROPDOWN_DEPTH: f32 = 0.00034;
const SHELL_DROPDOWN_TEXT_DEPTH: f32 = 0.00029;
// Owner-draw button dark text color 0x00000C05 decoded as RGB.
const SHELL_BUTTON_TEXT_RGB_00000C05: [f32; 3] = [5.0 / 255.0, 12.0 / 255.0, 0.0];
const SHELL_LABEL_TEXT_RGB: [f32; 3] = [1.0, 1.0, 0.0];
const SHELL_DROPDOWN_BG_RGB: [f32; 3] = [0.015, 0.024, 0.018];
const SHELL_DROPDOWN_BORDER_RGB: [f32; 3] = [0.60, 0.52, 0.24];
const SHELL_DROPDOWN_SELECTED_RGB: [f32; 3] = [0.16, 0.24, 0.15];
const SHELL_SCROLLBAR_TRACK_RGB: [f32; 3] = [0.035, 0.042, 0.034];
const SHELL_MODAL_BG_RGB: [f32; 3] = [0.020, 0.032, 0.025];
const SHELL_MODAL_PANEL_RGB: [f32; 3] = [0.050, 0.060, 0.044];

pub(crate) struct SkirmishPreviewTexture {
    pub selected_map_idx: usize,
    pub texture: BatchTexture,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentBackgroundRole {
    Mnscrns640,
    CoopGameSetup800,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LowerStripRole {
    Lwscrns640,
    LwscrnlLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonPiece {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ButtonSegment {
    piece: ButtonPiece,
    x: f32,
    width: f32,
    uv_width_ratio: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishShellDrawRole {
    ParentBackgroundMnscrns640,
    ParentBackgroundCoopGameSetup800,
    RightPanelTopSdtp,
    RightPanelTileSdbtnbkgd,
    RightPanelOverlaySdbtnanmFrame10,
    RightPanelBottomSdbtm,
    LowerSideLwscrns,
    LowerSideLwscrnl,
    OwnerDrawButton,
    Flag,
    PreviewSurface,
    StartMarker,
    StartMarkerLabel,
}

fn push_entry(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    out.push(SpriteInstance {
        position: [rect.x as f32, rect.y as f32],
        size: [rect.w as f32, rect.h as f32],
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    });
}

fn push_entry_sized(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    x: f32,
    y: f32,
    size: [f32; 2],
    depth: f32,
) {
    out.push(SpriteInstance {
        position: [x, y],
        size,
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    });
}

fn push_entry_top_clipped_native(
    out: &mut Vec<SpriteInstance>,
    mut entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let src_w = entry.pixel_size[0].round();
    let src_h = entry.pixel_size[1].round();
    if src_w <= 0.0 || src_h <= 0.0 || rect.w <= 0 || rect.h <= 0 {
        return;
    }

    let draw_w = (rect.w as f32).min(src_w);
    let draw_h = (rect.h as f32).min(src_h);
    entry.uv_size[0] *= draw_w / src_w;
    entry.uv_size[1] *= draw_h / src_h;
    push_entry_sized(
        out,
        entry,
        rect.x as f32,
        rect.y as f32,
        [draw_w, draw_h],
        depth,
    );
}

fn push_entry_native(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    x: i32,
    y: i32,
    depth: f32,
) {
    push_entry_sized(out, entry, x as f32, y as f32, entry.pixel_size, depth);
}

fn push_entry_sized_alpha(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    x: f32,
    y: f32,
    size: [f32; 2],
    depth: f32,
    alpha: f32,
) {
    out.push(SpriteInstance {
        position: [x, y],
        size,
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha,
        ..Default::default()
    });
}

fn push_entry_fit(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let [src_w, src_h] = entry.pixel_size;
    if src_w <= 0.0 || src_h <= 0.0 || rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let scale = (rect.w as f32 / src_w).min(rect.h as f32 / src_h);
    let w = (src_w * scale).round();
    let h = (src_h * scale).round();
    let x = rect.x as f32 + ((rect.w as f32 - w) * 0.5).round();
    let y = rect.y as f32 + ((rect.h as f32 - h) * 0.5).round();
    push_entry_sized(out, entry, x, y, [w, h], depth);
}

fn push_flag_entry_native_clipped_centered(
    out: &mut Vec<SpriteInstance>,
    mut entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let src_w_px = entry.pixel_size[0].round() as i32;
    let src_h_px = entry.pixel_size[1].round() as i32;
    if src_w_px <= 0 || src_h_px <= 0 || rect.w <= 0 || rect.h <= 0 {
        return;
    }

    let src_w = src_w_px as f32;
    let src_h = src_h_px as f32;
    let rect_w = rect.w as f32;
    let rect_h = rect.h as f32;
    let draw_w = src_w.min(rect_w);
    let draw_h = src_h.min(rect_h);
    let x = if src_w_px < rect.w {
        (rect.x + (rect.w - src_w_px) / 2) as f32
    } else {
        rect.x as f32
    };
    let y = if src_h_px < rect.h {
        (rect.y + (rect.h - src_h_px) / 2) as f32
    } else {
        rect.y as f32
    };

    entry.uv_size[0] *= draw_w / src_w;
    entry.uv_size[1] *= draw_h / src_h;
    push_entry_sized(out, entry, x, y, [draw_w, draw_h], depth);
}

fn button_entries(
    atlas: &SkirmishShellChromeAtlas,
    pressed: bool,
) -> Option<(
    SkirmishShellChromeEntry,
    SkirmishShellChromeEntry,
    SkirmishShellChromeEntry,
)> {
    if pressed {
        Some((
            atlas.button_down_left_30?,
            atlas.button_down_mid_30?,
            atlas.button_down_right_30?,
        ))
    } else {
        Some((
            atlas.button_up_left_30?,
            atlas.button_up_mid_30?,
            atlas.button_up_right_30?,
        ))
    }
}

#[cfg(test)]
fn button_piece_asset_names(pressed: bool) -> (&'static str, &'static str, &'static str) {
    if pressed {
        ("bde_li30.pcx", "bde_mi30.pcx", "bde_ri30.pcx")
    } else {
        ("bue_li30.pcx", "bue_mi30.pcx", "bue_ri30.pcx")
    }
}

fn build_button_segments(
    rect: RectPx,
    left_w: f32,
    mid_w: f32,
    right_w: f32,
) -> Vec<ButtonSegment> {
    if rect.w <= 0 {
        return Vec::new();
    }
    let rect_w = rect.w as f32;
    let left_w = left_w.round().max(1.0).min(rect_w);
    let right_w = right_w.round().max(1.0).min(rect_w);
    let mid_w = mid_w.round().max(1.0);
    let mut segments = vec![ButtonSegment {
        piece: ButtonPiece::Left,
        x: rect.x as f32,
        width: left_w,
        uv_width_ratio: 1.0,
    }];

    let middle_start = rect.x as f32 + left_w;
    let middle_dest_w = (rect_w - right_w).max(0.0);
    let mut covered = 0.0;
    while covered < middle_dest_w - f32::EPSILON {
        let width = (middle_dest_w - covered).min(mid_w);
        segments.push(ButtonSegment {
            piece: ButtonPiece::Middle,
            x: middle_start + covered,
            width,
            uv_width_ratio: width / mid_w,
        });
        covered += width;
    }

    segments.push(ButtonSegment {
        piece: ButtonPiece::Right,
        x: rect.x as f32 + rect_w - right_w,
        width: right_w,
        uv_width_ratio: 1.0,
    });
    segments
}

fn button_segment_sprite_size(entry: SkirmishShellChromeEntry, segment: ButtonSegment) -> [f32; 2] {
    [segment.width, entry.pixel_size[1]]
}

fn button_art_y(rect: RectPx, art_h: f32, pressed: bool, disabled: bool) -> f32 {
    let art_h = art_h.round() as i32;
    let pressed_offset = if pressed && !disabled {
        PRESSED_BUTTON_CONTENT_OFFSET_Y
    } else {
        0
    };
    (rect.y + (rect.h - art_h) / 2 + pressed_offset) as f32
}

#[cfg(test)]
fn shell_text_origin(
    rect: RectPx,
    text_w: u32,
    text_h: u32,
    flags: ShellAlign,
    y_offset: i32,
) -> (i32, i32) {
    let mut x = rect.x;
    if flags.contains(ShellAlign::H_CENTER) && text_w < rect.w as u32 {
        x += ((rect.w as u32 - text_w) / 2) as i32;
    } else if flags.contains(ShellAlign::H_RIGHT) && text_w < rect.w as u32 {
        x += (rect.w as u32 - text_w) as i32;
    }
    let mut y = rect.y + y_offset;
    if flags.contains(ShellAlign::V_CENTER) && text_h < rect.h as u32 {
        y += ((rect.h as u32 - text_h) / 2) as i32;
    }
    (x, y)
}

fn push_button_30(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    pressed: bool,
    disabled: bool,
    depth: f32,
) {
    let Some((left, mid, right)) = button_entries(atlas, pressed && !disabled) else {
        return;
    };
    let alpha = if disabled { BUTTON_DISABLED_ALPHA } else { 1.0 };
    for segment in build_button_segments(
        rect,
        left.pixel_size[0],
        mid.pixel_size[0],
        right.pixel_size[0],
    ) {
        let mut entry = match segment.piece {
            ButtonPiece::Left => left,
            ButtonPiece::Middle => mid,
            ButtonPiece::Right => right,
        };
        if segment.piece == ButtonPiece::Middle && segment.uv_width_ratio < 1.0 {
            entry.uv_size[0] *= segment.uv_width_ratio;
            entry.pixel_size[0] = segment.width;
        }
        push_entry_sized_alpha(
            out,
            entry,
            segment.x,
            button_art_y(rect, entry.pixel_size[1], pressed, disabled),
            button_segment_sprite_size(entry, segment),
            depth,
            alpha,
        );
    }
}

fn push_start_marker_sprites(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    projected_positions: &[(i32, i32)],
    depth: f32,
) {
    let Some(marker) = atlas.start_marker else {
        return;
    };
    for &(x, y) in projected_positions {
        let (marker_x, marker_y) = start_marker_top_left(x, y);
        push_entry_native(out, marker, marker_x, marker_y, depth);
    }
}

fn start_marker_top_left(anchor_x: i32, anchor_y: i32) -> (i32, i32) {
    (
        anchor_x + START_MARKER_OFFSET_X,
        anchor_y + START_MARKER_OFFSET_Y,
    )
}

fn project_preview_start_positions(
    bounds: &PreviewSourceBounds,
    fitted_preview_rect: RectPx,
) -> Vec<(i32, i32)> {
    if fitted_preview_rect.w <= 0
        || fitted_preview_rect.h <= 0
        || bounds.width == 0
        || bounds.height == 0
    {
        return Vec::new();
    }

    bounds
        .start_points
        .iter()
        .take(8)
        .map(|point| {
            let x_per_mille = ((point.x - bounds.origin_x) as i64 * 1000) / bounds.width as i64;
            let y_per_mille = ((point.y - bounds.origin_y) as i64 * 1000) / bounds.height as i64;
            let x = fitted_preview_rect.x
                + ((x_per_mille * fitted_preview_rect.w as i64) / 1000) as i32;
            let y = fitted_preview_rect.y
                + ((y_per_mille * fitted_preview_rect.h as i64) / 1000) as i32;
            (x, y)
        })
        .collect()
}

fn build_start_marker_instances(
    atlas: &SkirmishShellChromeAtlas,
    projected_positions: &[(i32, i32)],
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();
    push_start_marker_sprites(&mut instances, atlas, projected_positions, 0.00056);
    instances
}

fn side_item_data_for_country(country: SkirmishCountry) -> i32 {
    match country {
        SkirmishCountry::America => 0,
        SkirmishCountry::Korea => 1,
        SkirmishCountry::France => 2,
        SkirmishCountry::Germany => 3,
        SkirmishCountry::GreatBritain => 4,
        SkirmishCountry::Libya => 5,
        SkirmishCountry::Iraq => 6,
        SkirmishCountry::Cuba => 7,
        SkirmishCountry::Russia => 8,
        SkirmishCountry::Yuri => 9,
    }
}

fn flag_pcx_for_side_item_data(item_data: i32) -> Option<&'static str> {
    match item_data {
        -3 => Some("obsi.pcx"),
        -2 => Some("rani.pcx"),
        0 => Some("usai.pcx"),
        1 => Some("japi.pcx"),
        2 => Some("frai.pcx"),
        3 => Some("geri.pcx"),
        4 => Some("gbri.pcx"),
        5 => Some("djbi.pcx"),
        6 => Some("arbi.pcx"),
        7 => Some("lati.pcx"),
        8 => Some("rusi.pcx"),
        9 => Some("yrii.pcx"),
        _ => None,
    }
}

fn flag_name_for_country(country: SkirmishCountry) -> Option<&'static str> {
    flag_pcx_for_side_item_data(side_item_data_for_country(country))
}

fn flag_name_for_country_choice(random: bool, country: SkirmishCountry) -> Option<&'static str> {
    if random {
        flag_pcx_for_side_item_data(-2)
    } else {
        flag_name_for_country(country)
    }
}

fn flag_entry(atlas: &SkirmishShellChromeAtlas, label: &str) -> Option<SkirmishShellChromeEntry> {
    atlas
        .flags
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(label))
        .map(|(_, entry)| *entry)
}

fn checkbox_entry(
    atlas: &SkirmishShellChromeAtlas,
    checked: bool,
) -> Option<SkirmishShellChromeEntry> {
    if checked {
        atlas.checkbox_checked_cce_i
    } else {
        atlas.checkbox_unchecked_cue_i
    }
}

fn checkbox_checked(shell: &SkirmishShellState, id: SkirmishCheckboxId) -> bool {
    match id {
        SkirmishCheckboxId::ShortGame0x54e => shell.short_game,
        SkirmishCheckboxId::McvRepacks0x693 => shell.mcv_redeploy,
        SkirmishCheckboxId::CratesAppear0x696 => shell.crates,
        SkirmishCheckboxId::SuperWeapons0x69a => shell.super_weapons,
        SkirmishCheckboxId::BuildOffAlly0x69d => shell.build_off_ally,
    }
}

fn combo_face_entry(
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
) -> Option<SkirmishShellChromeEntry> {
    match rect.w {
        150 => atlas.combo_face_150,
        117 => atlas.combo_face_117,
        44 => atlas.combo_face_44,
        38 => atlas.combo_face_38,
        _ => None,
    }
}

fn push_tinted_entry(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    out.push(SpriteInstance {
        position: [rect.x as f32, rect.y as f32],
        size: [rect.w as f32, rect.h as f32],
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint,
        alpha: 1.0,
        ..Default::default()
    });
}

fn push_solid_rect(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    let Some(pixel) = atlas.white_pixel else {
        return;
    };
    push_tinted_entry(out, pixel, rect, tint, depth);
}

fn push_rect_outline(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y, rect.w, 1),
        tint,
        depth,
    );
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y + rect.h - 1, rect.w, 1),
        tint,
        depth,
    );
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y, 1, rect.h),
        tint,
        depth,
    );
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x + rect.w - 1, rect.y, 1, rect.h),
        tint,
        depth,
    );
}

fn push_scrollbar_thumb(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    depth: f32,
) {
    let top_h = atlas
        .scrollbar_thumb_top
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    let bottom_h = atlas
        .scrollbar_thumb_bottom
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    if let Some(top) = atlas.scrollbar_thumb_top {
        push_entry_native(out, top, rect.x, rect.y, depth);
    }
    if let Some(bottom) = atlas.scrollbar_thumb_bottom {
        push_entry_native(out, bottom, rect.x, rect.y + rect.h - bottom_h, depth);
    }
    if let Some(mid) = atlas.scrollbar_thumb_mid {
        let mid_y = rect.y + top_h;
        let mid_h = rect.h - top_h - bottom_h;
        if mid_h > 0 {
            push_entry(out, mid, RectPx::new(rect.x, mid_y, rect.w, mid_h), depth);
        }
    }
}

fn push_dropdown_scrollbar_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    scrollbar: RectPx,
    thumb: RectPx,
    pressed_part: Option<DropdownScrollbarPart>,
) {
    push_solid_rect(
        out,
        atlas,
        scrollbar,
        SHELL_SCROLLBAR_TRACK_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00004,
    );
    let up_entry = scrollbar_arrow_entry(
        atlas.scrollbar_arrow_up_released,
        atlas.scrollbar_arrow_up_pressed,
        pressed_part == Some(DropdownScrollbarPart::UpArrow),
    );
    if let Some(up) = up_entry {
        push_entry_native(
            out,
            up,
            scrollbar.x,
            scrollbar.y,
            SHELL_DROPDOWN_DEPTH - 0.00005,
        );
    }
    let down_entry = scrollbar_arrow_entry(
        atlas.scrollbar_arrow_down_released,
        atlas.scrollbar_arrow_down_pressed,
        pressed_part == Some(DropdownScrollbarPart::DownArrow),
    );
    if let Some(down) = down_entry {
        push_entry_native(
            out,
            down,
            scrollbar.x,
            scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
            SHELL_DROPDOWN_DEPTH - 0.00005,
        );
    }
    push_scrollbar_thumb(out, atlas, thumb, SHELL_DROPDOWN_DEPTH - 0.00006);
    push_rect_outline(
        out,
        atlas,
        scrollbar,
        SHELL_DROPDOWN_BORDER_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00007,
    );
}

fn scrollbar_arrow_entry(
    released: Option<SkirmishShellChromeEntry>,
    pressed: Option<SkirmishShellChromeEntry>,
    is_pressed: bool,
) -> Option<SkirmishShellChromeEntry> {
    if is_pressed {
        pressed.or(released)
    } else {
        released
    }
}

fn house_color_tint(index: usize) -> [f32; 3] {
    let ramp = house_color_ramp(HouseColorIndex(index.min(7) as u8));
    let color = ramp[0];
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    ]
}

fn push_combo_face(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    color_index: Option<usize>,
    open: bool,
    disabled: bool,
    depth: f32,
) {
    if let Some(face) = combo_face_entry(atlas, rect) {
        push_entry(out, face, combo_face_rect(rect), depth);
    }
    if let (Some(color_index), Some(white)) = (color_index, atlas.white_pixel) {
        push_tinted_entry(
            out,
            white,
            combo_swatch_rect(rect),
            house_color_tint(color_index),
            SHELL_SWATCH_DEPTH,
        );
    }
    let arrow = match (disabled, open) {
        (true, true) => atlas.combo_arrow_down_gray_pressed,
        (true, false) => atlas.combo_arrow_down_gray_released,
        (false, true) => atlas.combo_arrow_down_pressed,
        (false, false) => atlas.combo_arrow_down_released,
    };
    if let Some(arrow) = arrow {
        let arrow_rect = combo_arrow_rect(rect);
        push_entry_native(out, arrow, arrow_rect.x, arrow_rect.y, depth - 0.00001);
    }
}

fn push_trackbar_plaque(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    depth: f32,
) {
    let plaque = trackbar_plaque_rect(rect);
    if let Some(mid) = atlas.trackbar_plaque_mid_trofm {
        push_entry(out, mid, plaque, depth);
    }
    if let Some(left) = atlas.trackbar_plaque_left_trofl {
        push_entry_native(out, left, plaque.x, plaque.y, depth - 0.00001);
    }
    if let Some(right) = atlas.trackbar_plaque_right_trofr {
        let w = right.pixel_size[0].round() as i32;
        push_entry_native(
            out,
            right,
            plaque.x + plaque.w - w,
            plaque.y,
            depth - 0.00001,
        );
    }
}

fn trackbar_rect_for_id(layout: &SkirmishShellLayout, id: SkirmishTrackbarId) -> RectPx {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => layout.trackbars.game_speed,
        SkirmishTrackbarId::Credits0x511 => layout.trackbars.credits,
        SkirmishTrackbarId::UnitCount0x50c => layout.trackbars.unit_count,
    }
}

fn push_checkbox_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    for checkbox in layout.checkboxes {
        let Some(entry) = checkbox_entry(atlas, checkbox_checked(shell, checkbox.id)) else {
            continue;
        };
        push_entry(
            out,
            entry,
            checkbox_icon_rect(checkbox.rect),
            SHELL_CONTROL_DEPTH,
        );
    }
}

fn push_trackbar_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    for id in [
        SkirmishTrackbarId::GameSpeed0x529,
        SkirmishTrackbarId::Credits0x511,
        SkirmishTrackbarId::UnitCount0x50c,
    ] {
        let rect = trackbar_rect_for_id(layout, id);
        if let Some(rail) = atlas.trackbar_rail {
            push_entry_native(out, rail, rect.x, rect.y, SHELL_CONTROL_DEPTH);
        }
        push_trackbar_plaque(out, atlas, rect, SHELL_CONTROL_DEPTH);

        let value = trackbar_visual_value(shell, id);
        let (min, max, step) = match id {
            SkirmishTrackbarId::GameSpeed0x529 => (0, 6, 1),
            SkirmishTrackbarId::Credits0x511 => (5000, 10000, 100),
            SkirmishTrackbarId::UnitCount0x50c => (0, 10, 1),
        };
        let px = trackbar_pixel_offset(value, min, max, step, rect);
        if let Some(thumb) = atlas.trackbar_thumb_trakgrip {
            let thumb_rect = trackbar_thumb_rect(rect, px);
            push_entry(out, thumb, thumb_rect, SHELL_CONTROL_DEPTH - 0.00002);
        }
    }
}

fn push_combo_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
    let open = shell.open_combo_dropdown.map(|dropdown| dropdown.id);
    push_combo_face(
        out,
        atlas,
        layout.rows.side_combos[0],
        None,
        open == Some(SkirmishComboId::Side(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );
    push_combo_face(
        out,
        atlas,
        layout.color_combos[0],
        Some(shell.player_color_index),
        open == Some(SkirmishComboId::Color(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );
    push_combo_face(
        out,
        atlas,
        layout.rows.start_combos[0],
        None,
        open == Some(SkirmishComboId::Start(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );
    push_combo_face(
        out,
        atlas,
        layout.rows.team_combos[0],
        None,
        open == Some(SkirmishComboId::Team(0)),
        false,
        SHELL_CONTROL_DEPTH,
    );

    for (idx, opponent) in shell.opponents.iter().enumerate() {
        if idx >= layout.rows.ai_type_combos.len() {
            break;
        }
        let row = idx + 1;
        push_combo_face(
            out,
            atlas,
            layout.rows.ai_type_combos[idx],
            None,
            open == Some(SkirmishComboId::AiType(idx)),
            false,
            SHELL_CONTROL_DEPTH,
        );
        let sibling_disabled = !opponent.is_active();
        push_combo_face(
            out,
            atlas,
            layout.rows.side_combos[row],
            None,
            open == Some(SkirmishComboId::Side(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
        push_combo_face(
            out,
            atlas,
            layout.color_combos[row],
            (!sibling_disabled).then_some(opponent.color_index),
            open == Some(SkirmishComboId::Color(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
        push_combo_face(
            out,
            atlas,
            layout.rows.start_combos[row],
            None,
            open == Some(SkirmishComboId::Start(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
        push_combo_face(
            out,
            atlas,
            layout.rows.team_combos[row],
            None,
            open == Some(SkirmishComboId::Team(row)),
            sibling_disabled,
            SHELL_CONTROL_DEPTH,
        );
    }
}

fn push_dropdown_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
) {
    let Some(open) = shell.open_combo_dropdown else {
        return;
    };
    let Some(dropdown) = combo_dropdown_rect(shell, layout, maps, open.id) else {
        return;
    };
    let content = combo_dropdown_content_rect(shell, layout, maps, open.id).unwrap_or(dropdown);
    let needs_scrollbar = combo_dropdown_needs_scrollbar(shell, maps, open.id);
    push_solid_rect(
        out,
        atlas,
        dropdown,
        SHELL_DROPDOWN_BG_RGB,
        SHELL_DROPDOWN_DEPTH,
    );
    let visible_rows = combo_dropdown_visible_row_count(shell, maps, open.id);
    if let Some(selected_rect) =
        dropdown_selected_row_rect(shell, maps, open.id, open.top_index, content)
    {
        push_solid_rect(
            out,
            atlas,
            selected_rect,
            SHELL_DROPDOWN_SELECTED_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00001,
        );
    }
    for (idx, item) in combo_items(shell, maps, open.id)
        .into_iter()
        .skip(open.top_index)
        .take(visible_rows)
        .enumerate()
    {
        if let SkirmishComboItem::Color(color_index) = item {
            let row_y = content.y + idx as i32 * COMBO_DROPDOWN_ROW_H;
            let swatch = RectPx::new(content.x + 2, row_y + 2, content.w - 4, 19);
            push_solid_rect(
                out,
                atlas,
                swatch,
                house_color_tint(color_index),
                SHELL_DROPDOWN_DEPTH - 0.00002,
            );
        }
    }
    if needs_scrollbar {
        if let (Some(scrollbar), Some(thumb)) = (
            combo_dropdown_scrollbar_rect(shell, layout, maps, open.id),
            combo_dropdown_scroll_thumb_rect(shell, layout, maps, open.id),
        ) {
            let pressed_part = shell
                .dropdown_scroll_press
                .filter(|pressed| pressed.id == open.id)
                .map(|pressed| pressed.part);
            push_dropdown_scrollbar_instances(out, atlas, scrollbar, thumb, pressed_part);
        }
    }
    push_rect_outline(
        out,
        atlas,
        dropdown,
        SHELL_DROPDOWN_BORDER_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00003,
    );
}

fn push_choose_map_listbox_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    list: RectPx,
    selected_visible_row: Option<usize>,
    depth: f32,
) {
    push_solid_rect(out, atlas, list, SHELL_DROPDOWN_BG_RGB, depth);
    if let Some(row) = selected_visible_row {
        let y = list.y + row as i32 * CHOOSE_MAP_LIST_ROW_H;
        if y < list.y + list.h {
            push_solid_rect(
                out,
                atlas,
                RectPx::new(
                    list.x,
                    y,
                    list.w,
                    CHOOSE_MAP_LIST_ROW_H.min(list.y + list.h - y),
                ),
                SHELL_DROPDOWN_SELECTED_RGB,
                depth - 0.00001,
            );
        }
    }
    push_rect_outline(out, atlas, list, SHELL_DROPDOWN_BORDER_RGB, depth - 0.00002);
}

fn push_choose_map_modal_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &ChooseMapModalLayout,
    shell: &SkirmishShellState,
) {
    let Some(modal) = shell.choose_map_modal.as_ref() else {
        return;
    };
    push_solid_rect(
        out,
        atlas,
        layout.dialog,
        SHELL_MODAL_BG_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00008,
    );
    push_rect_outline(
        out,
        atlas,
        layout.dialog,
        SHELL_DROPDOWN_BORDER_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00009,
    );
    push_choose_map_listbox_instances(
        out,
        atlas,
        layout.mode_list,
        modal
            .selected_mode_id
            .checked_sub(1)
            .map(|idx| idx as usize)
            .filter(|idx| *idx >= modal.mode_top_index)
            .map(|idx| idx - modal.mode_top_index),
        SHELL_DROPDOWN_DEPTH - 0.00010,
    );
    push_choose_map_listbox_instances(
        out,
        atlas,
        layout.map_list,
        modal
            .highlighted_filtered_index
            .filter(|idx| *idx >= modal.map_top_index)
            .map(|idx| idx - modal.map_top_index),
        SHELL_DROPDOWN_DEPTH - 0.00010,
    );
    for button in [
        layout.use_map_button,
        layout.cancel_button,
        layout.create_random_map_button,
    ] {
        push_solid_rect(
            out,
            atlas,
            button,
            SHELL_MODAL_PANEL_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00011,
        );
        push_rect_outline(
            out,
            atlas,
            button,
            SHELL_DROPDOWN_BORDER_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00012,
        );
    }
    push_rect_outline(
        out,
        atlas,
        layout.preview,
        SHELL_DROPDOWN_BORDER_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00012,
    );
}

fn dropdown_selected_row_rect(
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    top_index: usize,
    content: RectPx,
) -> Option<RectPx> {
    let visible_rows = combo_dropdown_visible_row_count(shell, maps, id);
    let selected = selected_combo_item_index(shell, maps, id)?;
    if selected < top_index || selected >= top_index + visible_rows {
        return None;
    }
    Some(RectPx::new(
        content.x,
        content.y + (selected - top_index) as i32 * COMBO_DROPDOWN_ROW_H,
        content.w,
        COMBO_DROPDOWN_ROW_H,
    ))
}

fn parent_background_role(layout: &SkirmishShellLayout) -> Option<ParentBackgroundRole> {
    match layout.screen.w {
        640 => Some(ParentBackgroundRole::Mnscrns640),
        800 => Some(ParentBackgroundRole::CoopGameSetup800),
        width => {
            if width > 800 {
                HIGH_RES_PARENT_BACKGROUND_LOG.call_once(|| {
                    log::info!(
                        "Skirmish shell parent background skipped for {width}px width; Ghidra verifies no fresh >800 parent substitution"
                    );
                });
            }
            None
        }
    }
}

fn parent_background_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> Option<SkirmishShellChromeEntry> {
    match parent_background_role(layout)? {
        ParentBackgroundRole::Mnscrns640 => atlas.background_640_mnscrns,
        ParentBackgroundRole::CoopGameSetup800 => atlas.background_800_coop_game_setup,
    }
}

fn lower_strip_role(layout: &SkirmishShellLayout) -> LowerStripRole {
    match layout.screen.w {
        640 => LowerStripRole::Lwscrns640,
        _ => LowerStripRole::LwscrnlLarge,
    }
}

fn lower_strip_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> Option<SkirmishShellChromeEntry> {
    match lower_strip_role(layout) {
        LowerStripRole::Lwscrns640 => atlas.lower_side_640_lwscrns,
        LowerStripRole::LwscrnlLarge => atlas.lower_side_large_lwscrnl,
    }
}

fn common_shell_origin(layout: &SkirmishShellLayout) -> (i32, i32) {
    let x = if layout.screen.w > 1023 {
        (layout.screen.w - 800) / 2
    } else {
        0
    };
    let y = if layout.screen.h > 767 {
        (layout.screen.h - 600) / 2
    } else {
        0
    };
    (x, y)
}

fn lower_strip_rect(layout: &SkirmishShellLayout, entry: SkirmishShellChromeEntry) -> RectPx {
    let w = entry.pixel_size[0].round() as i32;
    let h = entry.pixel_size[1].round() as i32;
    let (origin_x, origin_y) = common_shell_origin(layout);
    let shell_h = if layout.screen.h > 767 {
        600
    } else {
        layout.screen.h
    };
    RectPx::new(origin_x, origin_y + shell_h - h, w, h)
}

fn right_panel_overlay_rect(
    layout: &SkirmishShellLayout,
    row: i32,
    entry: SkirmishShellChromeEntry,
) -> RectPx {
    let w = entry.pixel_size[0].round() as i32;
    let h = entry.pixel_size[1].round() as i32;
    let x = layout.right_panel.tile.x + layout.right_panel.tile.w - w;
    RectPx::new(x, layout.right_panel.tile.y + row * h, w, h)
}

fn right_panel_frame10_overlay_active(_shell: &SkirmishShellState) -> bool {
    // Verified standard offline Skirmish first paint leaves the dialog gate at
    // zero, and that gate makes RightPanel__Draw skip the frame-10 overlay.
    false
}

pub fn skirmish_shell_semantic_draw_order(
    layout: &SkirmishShellLayout,
    overlay_frame10_active: bool,
    preview_surface_available: bool,
    start_marker_overlay_available: bool,
    flag_count: usize,
) -> Vec<SkirmishShellDrawRole> {
    let mut roles = Vec::new();
    roles.push(SkirmishShellDrawRole::RightPanelTopSdtp);
    roles.extend(
        std::iter::repeat(SkirmishShellDrawRole::RightPanelTileSdbtnbkgd)
            .take(layout.right_panel.tile_count.max(0) as usize),
    );
    if overlay_frame10_active {
        roles.extend(
            std::iter::repeat(SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10)
                .take(layout.right_panel.tile_count.max(0) as usize),
        );
    }
    roles.push(SkirmishShellDrawRole::RightPanelBottomSdbtm);
    roles.push(match lower_strip_role(layout) {
        LowerStripRole::Lwscrns640 => SkirmishShellDrawRole::LowerSideLwscrns,
        LowerStripRole::LwscrnlLarge => SkirmishShellDrawRole::LowerSideLwscrnl,
    });
    if let Some(role) = parent_background_role(layout) {
        roles.push(match role {
            ParentBackgroundRole::Mnscrns640 => SkirmishShellDrawRole::ParentBackgroundMnscrns640,
            ParentBackgroundRole::CoopGameSetup800 => {
                SkirmishShellDrawRole::ParentBackgroundCoopGameSetup800
            }
        });
    }
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::OwnerDrawButton).take(3));
    if preview_surface_available {
        roles.push(SkirmishShellDrawRole::PreviewSurface);
    }
    if start_marker_overlay_available {
        roles.push(SkirmishShellDrawRole::StartMarker);
        roles.push(SkirmishShellDrawRole::StartMarkerLabel);
    }
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::Flag).take(flag_count));
    roles
}

pub fn build_skirmish_shell_instances(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    choose_map_layout: Option<&ChooseMapModalLayout>,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();

    if let Some(top) = atlas.right_panel_top_sdtp {
        push_entry(&mut instances, top, layout.right_panel.top, 0.00080);
    }

    if let Some(tile) = atlas.right_panel_tile_sdbtnbkgd {
        for row in 0..layout.right_panel.tile_count {
            let rect = RectPx::new(
                layout.right_panel.tile.x,
                layout.right_panel.tile.y + row * layout.right_panel.tile.h,
                layout.right_panel.tile.w,
                layout.right_panel.tile.h,
            );
            push_entry(&mut instances, tile, rect, 0.00079);
        }
    }

    if right_panel_frame10_overlay_active(shell) {
        if let Some(overlay) = atlas.right_panel_overlay_sdbtnanm_frame10 {
            for row in 0..layout.right_panel.tile_count {
                push_entry(
                    &mut instances,
                    overlay,
                    right_panel_overlay_rect(layout, row, overlay),
                    0.000785,
                );
            }
        }
    }

    if let Some(bottom) = atlas.right_panel_bottom_sdbtm {
        push_entry_top_clipped_native(&mut instances, bottom, layout.right_panel.bottom, 0.00078);
    }

    if let Some(lower_strip) = lower_strip_entry(atlas, layout) {
        push_entry(
            &mut instances,
            lower_strip,
            lower_strip_rect(layout, lower_strip),
            SHELL_LOWER_STRIP_DEPTH,
        );
    }

    if let Some(background) = parent_background_entry(atlas, layout) {
        push_entry_native(
            &mut instances,
            background,
            layout.screen.x,
            layout.screen.y,
            SHELL_PARENT_BACKGROUND_DEPTH,
        );
    }

    push_button_30(
        &mut instances,
        atlas,
        layout.start_button,
        shell.pressed_owner_draw_button == Some(OwnerDrawButton::StartGame0x617),
        false,
        0.00059,
    );
    push_button_30(
        &mut instances,
        atlas,
        layout.back_button,
        shell.pressed_owner_draw_button == Some(OwnerDrawButton::Back0x5c0),
        false,
        0.00059,
    );
    push_button_30(
        &mut instances,
        atlas,
        layout.choose_map_button,
        shell.pressed_owner_draw_button == Some(OwnerDrawButton::ChooseMap0x5aa),
        false,
        0.00059,
    );

    push_combo_instances(&mut instances, atlas, layout, shell);
    push_checkbox_instances(&mut instances, atlas, layout, shell);
    push_trackbar_instances(&mut instances, atlas, layout, shell);

    if let Some(flag) =
        flag_name_for_country_choice(shell.player_country_random, shell.player_country)
            .and_then(|name| flag_entry(atlas, name))
    {
        push_flag_entry_native_clipped_centered(&mut instances, flag, layout.flags[0], 0.00057);
    }
    for idx in 1..layout.flags.len() {
        let entry = shell
            .opponents
            .get(idx - 1)
            .filter(|opponent| opponent.is_active())
            .and_then(|opponent| {
                flag_name_for_country_choice(opponent.country_random, opponent.country)
            })
            .and_then(|name| flag_entry(atlas, name));
        if let Some(flag) = entry {
            push_flag_entry_native_clipped_centered(
                &mut instances,
                flag,
                layout.flags[idx],
                0.00057,
            );
        }
    }

    push_dropdown_instances(&mut instances, atlas, layout, shell, maps);
    if let Some(choose_map_layout) = choose_map_layout {
        push_choose_map_modal_instances(&mut instances, atlas, choose_map_layout, shell);
    }

    instances
}

fn localized_label(state: &AppState, key: &str, fallback: &str) -> String {
    state
        .csf
        .as_ref()
        .and_then(|csf| csf.get(key))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback.to_string())
}

fn checkbox_label(id: SkirmishCheckboxId) -> (&'static str, &'static str) {
    match id {
        SkirmishCheckboxId::ShortGame0x54e => ("GUI:ShortGame", "Short Game"),
        SkirmishCheckboxId::McvRepacks0x693 => ("GUI:MCVRepacks", "MCV Repacks"),
        SkirmishCheckboxId::CratesAppear0x696 => ("GUI:CratesAppear", "Crates Appear"),
        SkirmishCheckboxId::SuperWeapons0x69a => ("GUI:SuperWeaponsAllowed", "Super Weapons"),
        SkirmishCheckboxId::BuildOffAlly0x69d => ("GUI:BuildOffAlly", "Build Off Ally"),
    }
}

fn start_position_label(pos: crate::ui::main_menu::StartPosition) -> String {
    match pos {
        crate::ui::main_menu::StartPosition::Auto => "Random".to_string(),
        crate::ui::main_menu::StartPosition::Position(idx) => (idx + 1).to_string(),
    }
}

fn team_label(team: i32) -> String {
    match team {
        0 => "A".to_string(),
        1 => "B".to_string(),
        2 => "C".to_string(),
        3 => "D".to_string(),
        _ => "None".to_string(),
    }
}

fn combo_item_label(state: &AppState, item: SkirmishComboItem) -> String {
    match item {
        SkirmishComboItem::AiType(row_type) => {
            let (key, fallback) = row_type_label(row_type);
            localized_label(state, key, fallback)
        }
        SkirmishComboItem::Country(SkirmishCountryChoice::Random) => {
            localized_label(state, "GUI:RandomAsSymbols", "Random")
        }
        SkirmishComboItem::Country(SkirmishCountryChoice::Country(country)) => {
            country.label().to_string()
        }
        SkirmishComboItem::ColorSentinel(_) => String::new(),
        SkirmishComboItem::Color(_) => String::new(),
        SkirmishComboItem::Start(start) => match start {
            crate::ui::main_menu::StartPosition::Auto => {
                localized_label(state, "GUI:RandomAsSymbols", "Random")
            }
            crate::ui::main_menu::StartPosition::Position(idx) => (idx + 1).to_string(),
        },
        SkirmishComboItem::Team(team) => {
            if team == 0 {
                localized_label(state, "GUI:None", "None")
            } else {
                team_label(team)
            }
        }
    }
}

fn country_choice_label(state: &AppState, random: bool, country: SkirmishCountry) -> String {
    if random {
        localized_label(state, "GUI:RandomAsSymbols", "Random")
    } else {
        country.label().to_string()
    }
}

fn trackbar_display_value(shell: &SkirmishShellState, id: SkirmishTrackbarId) -> String {
    match id {
        SkirmishTrackbarId::GameSpeed0x529 => trackbar_visual_value(shell, id).to_string(),
        SkirmishTrackbarId::Credits0x511 => shell.starting_credits.to_string(),
        SkirmishTrackbarId::UnitCount0x50c => shell.unit_count.to_string(),
    }
}

fn row_type_label(row_type: SkirmishAiRowType) -> (&'static str, &'static str) {
    row_type.label()
}

fn push_button_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    pressed: bool,
    depth: f32,
) {
    let text_rect = button_text_rect(rect, pressed);
    push_text_draw(
        out,
        state,
        label,
        text_rect,
        SHELL_LABEL_TEXT_RGB,
        ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        depth,
    );
}

fn button_text_rect(rect: RectPx, pressed: bool) -> TextRect {
    let mut x = rect.x;
    let y = if pressed {
        x += 2;
        rect.y + 5
    } else {
        rect.y + 1
    };
    TextRect {
        x,
        y,
        w: (rect.x + rect.w - 2 - x).max(0) as u32,
        h: (rect.y + rect.h - y).max(0) as u32,
    }
}

fn push_text_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    text_rect: TextRect,
    color: [f32; 3],
    align: ShellAlign,
    depth: f32,
) {
    let draw = shell_text::draw_in_rect(
        &state.bit_font,
        label,
        text_rect,
        color,
        align,
        [0.0, 0.0],
        depth,
    );
    out.push(draw);
}

fn rect_to_text_rect(rect: RectPx) -> TextRect {
    TextRect {
        x: rect.x,
        y: rect.y,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    }
}

fn push_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    depth: f32,
) {
    let text_rect = TextRect {
        x: rect.x,
        y: rect.y,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    };
    push_text_draw(
        out,
        state,
        label,
        text_rect,
        SHELL_LABEL_TEXT_RGB,
        ShellAlign::V_CENTER,
        depth,
    );
}

fn push_static_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    align: ShellAlign,
    depth: f32,
) {
    push_text_draw(
        out,
        state,
        label,
        rect_to_text_rect(rect),
        SHELL_LABEL_TEXT_RGB,
        align,
        depth,
    );
}

fn build_shell_text_draws(
    state: &AppState,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
) -> (Vec<ShellTextDraw>, Vec<SpriteInstance>) {
    let mut shell_draws: Vec<ShellTextDraw> = Vec::new();
    let bare_instances: Vec<SpriteInstance> = Vec::new();

    let start = localized_label(state, "GUI:StartGame", "Start Game");
    let choose = localized_label(state, "GUI:ChooseMap", "Choose Map");
    let back = localized_label(state, "GUI:Back", "Back");

    for (label, rect, button) in [
        (
            start.as_str(),
            layout.start_button,
            OwnerDrawButton::StartGame0x617,
        ),
        (
            choose.as_str(),
            layout.choose_map_button,
            OwnerDrawButton::ChooseMap0x5aa,
        ),
        (
            back.as_str(),
            layout.back_button,
            OwnerDrawButton::Back0x5c0,
        ),
    ] {
        push_button_label_draw(
            &mut shell_draws,
            state,
            label,
            rect,
            shell.pressed_owner_draw_button == Some(button),
            0.00041,
        );
    }

    for (key, fallback, rect) in [
        ("GUI:Players", "Players", layout.column_labels.players),
        ("GUI:Side", "Side", layout.column_labels.side),
        ("GUI:Color", "Color", layout.column_labels.color),
        ("GUI:StartPosition", "Start", layout.column_labels.start),
        ("GUI:Team", "Team", layout.column_labels.team),
    ] {
        let label = localized_label(state, key, fallback);
        push_label_draw(
            &mut shell_draws,
            state,
            &label,
            rect,
            SHELL_CONTROL_TEXT_DEPTH,
        );
    }

    let title = localized_label(state, "GUI:SkirmishGame", "Skirmish Game");
    push_static_label_draw(
        &mut shell_draws,
        state,
        &title,
        layout.right_panel_text.title,
        ShellAlign::H_CENTER,
        SHELL_CONTROL_TEXT_DEPTH,
    );
    let game_type = state
        .skirmish_modes
        .iter()
        .find(|mode| mode.id == shell.selected_mode_id)
        .map(|mode| localized_label(state, &mode.ui_name_key, &mode.ui_name_key))
        .unwrap_or_else(|| localized_label(state, "GUI:Battle", "Battle"));
    push_static_label_draw(
        &mut shell_draws,
        state,
        &game_type,
        layout.right_panel_text.game_type,
        ShellAlign::H_CENTER,
        SHELL_CONTROL_TEXT_DEPTH,
    );
    let map_label = maps
        .get(shell.selected_map_idx)
        .map(|map| map.display_name.as_str())
        .unwrap_or("None");
    push_static_label_draw(
        &mut shell_draws,
        state,
        map_label,
        layout.right_panel_text.map_label,
        ShellAlign::H_CENTER,
        SHELL_CONTROL_TEXT_DEPTH,
    );

    push_label_draw(
        &mut shell_draws,
        state,
        "Player",
        layout.player_name,
        SHELL_CONTROL_TEXT_DEPTH,
    );

    for checkbox in layout.checkboxes {
        let (key, fallback) = checkbox_label(checkbox.id);
        let label = localized_label(state, key, fallback);
        push_label_draw(
            &mut shell_draws,
            state,
            &label,
            checkbox_text_rect(checkbox.rect),
            SHELL_CONTROL_TEXT_DEPTH,
        );
    }

    for id in [
        SkirmishTrackbarId::GameSpeed0x529,
        SkirmishTrackbarId::Credits0x511,
        SkirmishTrackbarId::UnitCount0x50c,
    ] {
        let value = trackbar_display_value(shell, id);
        push_text_draw(
            &mut shell_draws,
            state,
            &value,
            rect_to_text_rect(trackbar_value_text_rect(trackbar_rect_for_id(layout, id))),
            SHELL_BUTTON_TEXT_RGB_00000C05,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
            SHELL_CONTROL_TEXT_DEPTH,
        );
    }

    push_label_draw(
        &mut shell_draws,
        state,
        &country_choice_label(state, shell.player_country_random, shell.player_country),
        combo_text_rect(layout.rows.side_combos[0]),
        SHELL_CONTROL_TEXT_DEPTH,
    );
    push_label_draw(
        &mut shell_draws,
        state,
        &start_position_label(shell.player_start_position),
        combo_text_rect(layout.rows.start_combos[0]),
        SHELL_CONTROL_TEXT_DEPTH,
    );
    push_label_draw(
        &mut shell_draws,
        state,
        &team_label(shell.player_team),
        combo_text_rect(layout.rows.team_combos[0]),
        SHELL_CONTROL_TEXT_DEPTH,
    );

    for (idx, opponent) in shell.opponents.iter().enumerate() {
        if idx >= layout.rows.ai_type_combos.len() {
            break;
        }
        let row = idx + 1;
        let (key, fallback) = row_type_label(opponent.row_type);
        let row_type = localized_label(state, key, fallback);
        push_label_draw(
            &mut shell_draws,
            state,
            &row_type,
            combo_text_rect(layout.rows.ai_type_combos[idx]),
            SHELL_CONTROL_TEXT_DEPTH,
        );
        if opponent.is_active() {
            push_label_draw(
                &mut shell_draws,
                state,
                &country_choice_label(state, opponent.country_random, opponent.country),
                combo_text_rect(layout.rows.side_combos[row]),
                SHELL_CONTROL_TEXT_DEPTH,
            );
            push_label_draw(
                &mut shell_draws,
                state,
                &start_position_label(opponent.start_position),
                combo_text_rect(layout.rows.start_combos[row]),
                SHELL_CONTROL_TEXT_DEPTH,
            );
            push_label_draw(
                &mut shell_draws,
                state,
                &team_label(opponent.team),
                combo_text_rect(layout.rows.team_combos[row]),
                SHELL_CONTROL_TEXT_DEPTH,
            );
        }
    }

    if let Some(open) = shell.open_combo_dropdown {
        if let Some(dropdown) = combo_dropdown_rect(shell, layout, maps, open.id) {
            let content =
                combo_dropdown_content_rect(shell, layout, maps, open.id).unwrap_or(dropdown);
            let visible_rows = combo_dropdown_visible_row_count(shell, maps, open.id);
            let text_w = content.w - 3;
            for (idx, item) in combo_items(shell, maps, open.id)
                .into_iter()
                .skip(open.top_index)
                .take(visible_rows)
                .enumerate()
            {
                let label = combo_item_label(state, item);
                if label.is_empty() {
                    continue;
                }
                let rect = RectPx::new(
                    content.x + 3,
                    content.y + idx as i32 * COMBO_DROPDOWN_ROW_H,
                    text_w.max(0),
                    COMBO_DROPDOWN_ROW_H,
                );
                push_text_draw(
                    &mut shell_draws,
                    state,
                    &label,
                    rect_to_text_rect(rect),
                    SHELL_LABEL_TEXT_RGB,
                    ShellAlign::V_CENTER,
                    SHELL_DROPDOWN_TEXT_DEPTH,
                );
            }
        }
    }

    (shell_draws, bare_instances)
}

fn push_choose_map_modal_text_draws(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    layout: &ChooseMapModalLayout,
) {
    let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_ref() else {
        return;
    };

    let title = localized_label(state, "GUI:ChooseMap", "Choose Map");
    push_text_draw(
        out,
        state,
        &title,
        rect_to_text_rect(layout.title),
        SHELL_LABEL_TEXT_RGB,
        ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        SHELL_DROPDOWN_TEXT_DEPTH - 0.00008,
    );

    for (label, rect) in [
        (
            localized_label(state, "GUI:UseMap", "Use Map"),
            layout.use_map_button,
        ),
        (
            localized_label(state, "GUI:Cancel", "Cancel"),
            layout.cancel_button,
        ),
        (
            localized_label(state, "GUI:CreateRandomMap", "Create Random Map"),
            layout.create_random_map_button,
        ),
    ] {
        push_text_draw(
            out,
            state,
            &label,
            rect_to_text_rect(rect),
            SHELL_LABEL_TEXT_RGB,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.00009,
        );
    }

    let visible_mode_rows = (layout.mode_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (visible_row, mode) in state
        .skirmish_modes
        .iter()
        .skip(modal.mode_top_index)
        .take(visible_mode_rows)
        .enumerate()
    {
        let label = localized_label(state, &mode.ui_name_key, &mode.ui_name_key);
        let rect = RectPx::new(
            layout.mode_list.x + 2,
            layout.mode_list.y + visible_row as i32 * CHOOSE_MAP_LIST_ROW_H,
            layout.mode_list.w - 4,
            CHOOSE_MAP_LIST_ROW_H,
        );
        push_text_draw(
            out,
            state,
            &label,
            rect_to_text_rect(rect),
            SHELL_LABEL_TEXT_RGB,
            ShellAlign::V_CENTER,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.00009,
        );
    }

    let visible_map_rows = (layout.map_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (visible_row, record_idx) in modal
        .filtered_record_indices
        .iter()
        .skip(modal.map_top_index)
        .take(visible_map_rows)
        .enumerate()
    {
        let Some(record) = state.skirmish_scenario_records.get(*record_idx) else {
            continue;
        };
        let rect = RectPx::new(
            layout.map_list.x + 2,
            layout.map_list.y + visible_row as i32 * CHOOSE_MAP_LIST_ROW_H,
            layout.map_list.w - 4,
            CHOOSE_MAP_LIST_ROW_H,
        );
        push_text_draw(
            out,
            state,
            &record.display_name,
            rect_to_text_rect(rect),
            SHELL_LABEL_TEXT_RGB,
            ShellAlign::V_CENTER,
            SHELL_DROPDOWN_TEXT_DEPTH - 0.00009,
        );
    }
}

fn push_start_marker_labels(
    out: &mut Vec<SpriteInstance>,
    state: &AppState,
    projected_positions: &[(i32, i32)],
    depth: f32,
) {
    for (idx, &(x, y)) in projected_positions.iter().enumerate() {
        let label = (idx + 1).to_string();
        let (label_x, label_y) = start_marker_label_origin(x, y);
        out.extend(state.bit_font.build_text(
            &label,
            label_x as f32,
            label_y as f32,
            1.0,
            depth,
            start_marker_label_color(),
            [0.0, 0.0],
        ));
    }
}

fn start_marker_label_origin(anchor_x: i32, anchor_y: i32) -> (i32, i32) {
    (anchor_x - 2, anchor_y - 6)
}

fn start_marker_label_color() -> [f32; 3] {
    SHELL_LABEL_TEXT_RGB
}

fn build_start_marker_label_instances(
    state: &AppState,
    projected_positions: &[(i32, i32)],
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();
    push_start_marker_labels(&mut instances, state, projected_positions, 0.00040);
    instances
}

fn selected_preview_texture_is_current(state: &AppState, selected_map_idx: usize) -> bool {
    state
        .skirmish_preview_texture
        .as_ref()
        .is_some_and(|cached| cached.selected_map_idx == selected_map_idx)
}

fn decode_preview_for_map_entry(
    entry: &MapMenuEntry,
) -> Option<crate::map::preview::DecodedPreview> {
    if let Some(decoded) = entry.preview.decoded.as_ref() {
        return Some(decoded.clone());
    }

    let config = crate::util::config::GameConfig::load().ok()?;
    let ini_path = config.paths.ra2_dir.join(&entry.file_name);
    let ini = crate::app_list_maps::read_map_ini_for_metadata(&ini_path)?;
    match crate::map::preview::decode_preview_image_from_ini(&ini) {
        Ok(preview) => preview,
        Err(err) => {
            log::warn!(
                "Failed to lazily decode map PreviewPack for {}: {err}",
                entry.file_name
            );
            None
        }
    }
}

fn ensure_selected_preview_texture(state: &mut AppState) {
    let selected_map_idx = state.skirmish_shell_state.selected_map_idx;
    if selected_preview_texture_is_current(state, selected_map_idx) {
        return;
    }

    let decoded = state
        .skirmish_shell_maps
        .get(selected_map_idx)
        .and_then(decode_preview_for_map_entry);

    let Some(decoded) = decoded.as_ref() else {
        state.skirmish_preview_texture = None;
        return;
    };

    let texture = state.batch_renderer.create_texture(
        &state.gpu,
        &decoded.rgba,
        decoded.width,
        decoded.height,
    );
    state.skirmish_preview_texture = Some(SkirmishPreviewTexture {
        selected_map_idx,
        texture,
        width: decoded.width,
        height: decoded.height,
    });
}

fn aspect_fit_rect(dst: RectPx, src_w: u32, src_h: u32) -> RectPx {
    if dst.w <= 0 || dst.h <= 0 || src_w == 0 || src_h == 0 {
        return RectPx::new(dst.x, dst.y, 0, 0);
    }

    let src_w = src_w as i32;
    let src_h = src_h as i32;
    let scale_w = dst.w * 1000 / src_w;
    let scale_h = dst.h * 1000 / src_h;
    let scale = scale_w.min(scale_h);
    let fitted_w = src_w * scale / 1000;
    let fitted_h = src_h * scale / 1000;
    RectPx::new(
        dst.x + dst.w / 2 - (src_w * scale) / 2000,
        dst.y + dst.h / 2 - (src_h * scale) / 2000,
        fitted_w,
        fitted_h,
    )
}

fn build_preview_surface_instance(
    dst: RectPx,
    preview_width: u32,
    preview_height: u32,
) -> Option<SpriteInstance> {
    let fitted = aspect_fit_rect(dst, preview_width, preview_height);
    if fitted.w <= 0 || fitted.h <= 0 {
        return None;
    }

    Some(SpriteInstance {
        position: [fitted.x as f32, fitted.y as f32],
        size: [fitted.w as f32, fitted.h as f32],
        uv_origin: [0.0, 0.0],
        uv_size: [1.0, 1.0],
        depth: SHELL_PREVIEW_SURFACE_DEPTH,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    })
}

fn update_owner_draw_button_paint_sound(state: &mut AppState) {
    let pressed = state.skirmish_shell_state.pressed_owner_draw_button;
    if pressed.is_some() && state.skirmish_shell_last_painted_pressed_button.is_none() {
        crate::app::App::play_skirmish_shell_generic_click_sound(state);
    }
    state.skirmish_shell_last_painted_pressed_button = pressed;
}

pub(crate) fn render_skirmish_shell(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> anyhow::Result<SkirmishShellAction> {
    let chrome = state.skirmish_shell_chrome.take();
    let result = render_skirmish_shell_with_atlas(state, encoder, target, chrome.as_ref());
    state.skirmish_shell_chrome = chrome;
    result
}

fn render_skirmish_shell_with_atlas(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    atlas: Option<&SkirmishShellChromeAtlas>,
) -> anyhow::Result<SkirmishShellAction> {
    let layout = compute_layout(state.render_width(), state.render_height());
    let choose_map_layout = state
        .skirmish_shell_state
        .choose_map_modal
        .as_ref()
        .map(|_| compute_choose_map_modal_layout(state.render_width(), state.render_height()));
    let action = SkirmishShellAction::None;

    let Some(atlas) = atlas else {
        clear_shell_target(state, encoder, target);
        return Ok(action);
    };

    update_owner_draw_button_paint_sound(state);
    ensure_selected_preview_texture(state);
    let selected_preview_bounds = state
        .skirmish_shell_maps
        .get(state.skirmish_shell_state.selected_map_idx)
        .and_then(|entry| entry.preview_source_bounds.as_ref());
    let preview_rect = choose_map_layout
        .as_ref()
        .map(|layout| layout.preview)
        .unwrap_or(layout.map_preview);
    let fitted_preview_rect = state
        .skirmish_preview_texture
        .as_ref()
        .map(|preview| aspect_fit_rect(preview_rect, preview.width, preview.height));
    let projected_start_positions = selected_preview_bounds
        .zip(fitted_preview_rect)
        .map(|(bounds, rect)| project_preview_start_positions(bounds, rect))
        .unwrap_or_default();

    let preview_instance = state.skirmish_preview_texture.as_ref().and_then(|preview| {
        build_preview_surface_instance(preview_rect, preview.width, preview.height)
    });
    let preview_buffer = preview_instance.as_ref().and_then(|instance| {
        state
            .batch_renderer
            .create_instance_buffer(&state.gpu, &[*instance])
    });
    let marker_instances = fitted_preview_rect
        .filter(|_| !projected_start_positions.is_empty())
        .map(|_| build_start_marker_instances(atlas, &projected_start_positions))
        .unwrap_or_default();
    let marker_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &marker_instances);

    let instances = build_skirmish_shell_instances(
        atlas,
        &layout,
        choose_map_layout.as_ref(),
        &state.skirmish_shell_state,
        &state.skirmish_shell_maps,
    );
    let (mut shell_draws, bare_text_instances) = build_shell_text_draws(
        state,
        &layout,
        &state.skirmish_shell_state,
        &state.skirmish_shell_maps,
    );
    if let Some(choose_map_layout) = choose_map_layout.as_ref() {
        push_choose_map_modal_text_draws(&mut shell_draws, state, choose_map_layout);
    }
    let marker_label_instances = fitted_preview_rect
        .filter(|_| !projected_start_positions.is_empty())
        .map(|_| build_start_marker_label_instances(state, &projected_start_positions))
        .unwrap_or_default();
    let marker_label_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &marker_label_instances);
    state.batch_renderer.update_camera(
        &state.gpu,
        state.render_width() as f32,
        state.render_height() as f32,
        0.0,
        0.0,
        1.0,
    );

    let Some((buffer, count)) = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &instances)
    else {
        clear_shell_target(state, encoder, target);
        return Ok(action);
    };
    let bare_text_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &bare_text_instances);
    let scissored_text_buffers: Vec<_> = shell_draws
        .iter()
        .map(|d| {
            state
                .batch_renderer
                .create_instance_buffer(&state.gpu, &d.instances)
        })
        .collect();

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Skirmish Shell"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app_types::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &state.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    state
        .batch_renderer
        .draw_with_buffer_passthrough(&mut pass, &atlas.texture, &buffer, count);
    if let (Some(preview), Some((buffer, count))) = (
        state.skirmish_preview_texture.as_ref(),
        preview_buffer.as_ref(),
    ) {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &preview.texture,
            buffer,
            *count,
        );
    }
    if let Some((buffer, count)) = marker_buffer.as_ref() {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &atlas.texture,
            buffer,
            *count,
        );
    }
    if let Some((buffer, count)) = marker_label_buffer.as_ref() {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            state.bit_font.atlas(),
            buffer,
            *count,
        );
    }
    if let Some((buffer, count)) = bare_text_buffer.as_ref() {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            state.bit_font.atlas(),
            buffer,
            *count,
        );
    }
    for (draw, buf) in shell_draws.iter().zip(scissored_text_buffers.iter()) {
        let Some((buffer, count)) = buf.as_ref() else {
            continue;
        };
        pass.set_scissor_rect(
            draw.scissor.x,
            draw.scissor.y,
            draw.scissor.w,
            draw.scissor.h,
        );
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            state.bit_font.atlas(),
            buffer,
            *count,
        );
    }
    // Reset scissor to full render so any subsequent draws / passes aren't clipped.
    pass.set_scissor_rect(0, 0, state.render_width(), state.render_height());
    drop(pass);

    Ok(action)
}

fn clear_shell_target(
    state: &AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Skirmish Shell Clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app_types::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &state.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entry(width: f32, height: f32) -> SkirmishShellChromeEntry {
        SkirmishShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [width, height],
        }
    }

    #[test]
    fn scrollbar_arrow_entry_uses_pressed_art_only_while_pressed() {
        let released = test_entry(20.0, 22.0);
        let pressed = test_entry(21.0, 23.0);

        assert_eq!(
            scrollbar_arrow_entry(Some(released), Some(pressed), false)
                .unwrap()
                .pixel_size,
            released.pixel_size
        );
        assert_eq!(
            scrollbar_arrow_entry(Some(released), Some(pressed), true)
                .unwrap()
                .pixel_size,
            pressed.pixel_size
        );
        assert_eq!(
            scrollbar_arrow_entry(Some(released), None, true)
                .unwrap()
                .pixel_size,
            released.pixel_size
        );
    }

    #[test]
    fn button_segments_tile_middle_and_keep_caps() {
        let rect = RectPx::new(644, 241, 156, 42);
        let segments = build_button_segments(rect, 7.0, 64.0, 10.0);
        let expected = [
            (ButtonPiece::Left, 644.0, 7.0),
            (ButtonPiece::Middle, 651.0, 64.0),
            (ButtonPiece::Middle, 715.0, 64.0),
            (ButtonPiece::Middle, 779.0, 18.0),
            (ButtonPiece::Right, 790.0, 10.0),
        ];

        assert_eq!(segments.len(), expected.len());
        for (segment, (piece, x, width)) in segments.iter().zip(expected) {
            assert_eq!(segment.piece, piece);
            assert_eq!(segment.x, x);
            assert_eq!(segment.width, width);
        }
    }

    #[test]
    fn final_middle_segment_clips_when_width_is_not_tile_multiple() {
        let rect = RectPx::new(0, 0, 162, 37);
        let segments = build_button_segments(rect, 8.0, 60.0, 8.0);
        let middle_segments: Vec<_> = segments
            .iter()
            .filter(|s| s.piece == ButtonPiece::Middle)
            .collect();
        assert!(middle_segments.len() > 1);
        let last_middle = middle_segments.last().unwrap();
        assert!(last_middle.width < 60.0);
        assert!(last_middle.uv_width_ratio < 1.0);
    }

    #[test]
    fn button_segment_sprites_keep_native_art_height() {
        let entry = SkirmishShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [8.0, 30.0],
        };
        let segment = ButtonSegment {
            piece: ButtonPiece::Left,
            x: 635.0,
            width: 8.0,
            uv_width_ratio: 1.0,
        };

        assert_eq!(button_segment_sprite_size(entry, segment), [8.0, 30.0]);
    }

    #[test]
    fn button_art_y_centers_native_height_and_offsets_pressed_enabled() {
        let rect = RectPx::new(644, 241, 156, 42);

        assert_eq!(button_art_y(rect, 30.0, false, false), 247.0);
        assert_eq!(button_art_y(rect, 30.0, true, false), 249.0);
        assert_eq!(button_art_y(rect, 30.0, true, true), 247.0);
    }

    #[test]
    fn pressed_buttons_select_down_skin_assets() {
        assert_eq!(
            button_piece_asset_names(false),
            ("bue_li30.pcx", "bue_mi30.pcx", "bue_ri30.pcx")
        );
        assert_eq!(
            button_piece_asset_names(true),
            ("bde_li30.pcx", "bde_mi30.pcx", "bde_ri30.pcx")
        );
    }

    fn assert_f32_close(left: f32, right: f32) {
        assert!((left - right).abs() < 0.00001, "{left} != {right}");
    }

    #[test]
    fn flag_entry_draws_native_size_centered_when_smaller_than_rect() {
        let entry = SkirmishShellChromeEntry {
            uv_origin: [0.25, 0.5],
            uv_size: [0.5, 0.25],
            pixel_size: [20.0, 10.0],
        };
        let mut out = Vec::new();

        push_flag_entry_native_clipped_centered(
            &mut out,
            entry,
            RectPx::new(100, 50, 30, 16),
            0.00057,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position, [105.0, 53.0]);
        assert_eq!(out[0].size, [20.0, 10.0]);
        assert_eq!(out[0].uv_origin, entry.uv_origin);
        assert_eq!(out[0].uv_size, entry.uv_size);
    }

    #[test]
    fn flag_entry_centers_odd_delta_with_integer_truncation() {
        let entry = SkirmishShellChromeEntry {
            uv_origin: [0.25, 0.5],
            uv_size: [0.5, 0.25],
            pixel_size: [47.0, 14.0],
        };
        let mut out = Vec::new();

        push_flag_entry_native_clipped_centered(
            &mut out,
            entry,
            RectPx::new(100, 50, 48, 16),
            0.00057,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position, [100.0, 51.0]);
        assert_eq!(out[0].size, [47.0, 14.0]);
        assert_eq!(out[0].uv_origin, entry.uv_origin);
        assert_eq!(out[0].uv_size, entry.uv_size);
    }

    #[test]
    fn flag_entry_clips_uvs_without_fit_scaling_when_larger_than_rect() {
        let entry = SkirmishShellChromeEntry {
            uv_origin: [0.25, 0.5],
            uv_size: [0.5, 0.25],
            pixel_size: [20.0, 10.0],
        };
        let mut out = Vec::new();

        push_flag_entry_native_clipped_centered(
            &mut out,
            entry,
            RectPx::new(100, 50, 12, 6),
            0.00057,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position, [100.0, 50.0]);
        assert_eq!(out[0].size, [12.0, 6.0]);
        assert_eq!(out[0].uv_origin, entry.uv_origin);
        assert_f32_close(out[0].uv_size[0], 0.3);
        assert_f32_close(out[0].uv_size[1], 0.15);
    }

    #[test]
    fn sdbtm_bottom_cap_clips_top_source_rows_without_scaling() {
        let entry = SkirmishShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [0.84, 0.65],
            pixel_size: [168.0, 65.0],
        };
        let mut out = Vec::new();

        push_entry_top_clipped_native(&mut out, entry, RectPx::new(632, 577, 168, 23), 0.00078);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position, [632.0, 577.0]);
        assert_eq!(out[0].size, [168.0, 23.0]);
        assert_eq!(out[0].uv_origin, entry.uv_origin);
        assert_f32_close(out[0].uv_size[0], 0.84);
        assert_f32_close(out[0].uv_size[1], 0.23);
    }

    #[test]
    fn side_item_data_maps_to_verified_flag_pcxs() {
        assert_eq!(flag_pcx_for_side_item_data(-3), Some("obsi.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(-2), Some("rani.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(0), Some("usai.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(1), Some("japi.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(2), Some("frai.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(3), Some("geri.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(4), Some("gbri.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(5), Some("djbi.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(6), Some("arbi.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(7), Some("lati.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(8), Some("rusi.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(9), Some("yrii.pcx"));
        assert_eq!(flag_pcx_for_side_item_data(10), None);
    }

    #[test]
    fn country_flags_preserve_side_item_data_order() {
        assert_eq!(side_item_data_for_country(SkirmishCountry::Korea), 1);
        assert_eq!(
            flag_name_for_country(SkirmishCountry::Korea),
            Some("japi.pcx")
        );
        assert_eq!(side_item_data_for_country(SkirmishCountry::GreatBritain), 4);
        assert_eq!(
            flag_name_for_country(SkirmishCountry::GreatBritain),
            Some("gbri.pcx")
        );
        assert_eq!(side_item_data_for_country(SkirmishCountry::Cuba), 7);
        assert_eq!(
            flag_name_for_country(SkirmishCountry::Cuba),
            Some("lati.pcx")
        );
    }

    #[test]
    fn parent_background_role_uses_only_verified_widths() {
        assert_eq!(
            parent_background_role(&compute_layout(640, 480)),
            Some(ParentBackgroundRole::Mnscrns640)
        );
        assert_eq!(
            parent_background_role(&compute_layout(800, 600)),
            Some(ParentBackgroundRole::CoopGameSetup800)
        );
        assert_eq!(parent_background_role(&compute_layout(1024, 768)), None);
    }

    #[test]
    fn lower_strip_role_uses_only_verified_widths() {
        assert_eq!(
            lower_strip_role(&compute_layout(640, 480)),
            LowerStripRole::Lwscrns640
        );
        assert_eq!(
            lower_strip_role(&compute_layout(800, 600)),
            LowerStripRole::LwscrnlLarge
        );
        assert_eq!(
            lower_strip_role(&compute_layout(1024, 768)),
            LowerStripRole::LwscrnlLarge
        );
    }

    #[test]
    fn lower_strip_rect_uses_native_asset_size_at_screen_bottom() {
        let small = SkirmishShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [472.0, 32.0],
        };
        let large = SkirmishShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [632.0, 32.0],
        };

        assert_eq!(
            lower_strip_rect(&compute_layout(640, 480), small),
            RectPx::new(0, 448, 472, 32)
        );
        assert_eq!(
            lower_strip_rect(&compute_layout(800, 600), large),
            RectPx::new(0, 568, 632, 32)
        );
        assert_eq!(
            lower_strip_rect(&compute_layout(1024, 768), large),
            RectPx::new(112, 652, 632, 32)
        );
    }

    #[test]
    fn semantic_draw_order_records_verified_right_panel_sequence() {
        let layout = compute_layout(800, 600);
        let order = skirmish_shell_semantic_draw_order(&layout, true, false, false, 0);
        assert_eq!(order[0], SkirmishShellDrawRole::RightPanelTopSdtp);
        assert_eq!(
            &order[1..10],
            [SkirmishShellDrawRole::RightPanelTileSdbtnbkgd; 9]
        );
        assert_eq!(
            &order[10..19],
            [SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10; 9]
        );
        assert_eq!(order[19], SkirmishShellDrawRole::RightPanelBottomSdbtm);
        assert_eq!(order[20], SkirmishShellDrawRole::LowerSideLwscrnl);
        assert_eq!(
            order[21],
            SkirmishShellDrawRole::ParentBackgroundCoopGameSetup800
        );
        assert_eq!(&order[22..25], [SkirmishShellDrawRole::OwnerDrawButton; 3]);
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarker));
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarkerLabel));
    }

    #[test]
    fn standard_offline_first_paint_skips_sdbtnanm_frame10_overlay() {
        let shell = SkirmishShellState::default();
        let layout = compute_layout(800, 600);
        let order = skirmish_shell_semantic_draw_order(
            &layout,
            right_panel_frame10_overlay_active(&shell),
            false,
            false,
            0,
        );

        assert!(!right_panel_frame10_overlay_active(&shell));
        assert!(!order.contains(&SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10));
        assert!(order.contains(&SkirmishShellDrawRole::RightPanelBottomSdbtm));
    }

    #[test]
    fn semantic_draw_order_keeps_1024_parent_blank_but_large_lower_strip() {
        let order =
            skirmish_shell_semantic_draw_order(&compute_layout(1024, 768), false, false, false, 0);
        assert_eq!(order[0], SkirmishShellDrawRole::RightPanelTopSdtp);
        assert!(order.contains(&SkirmishShellDrawRole::LowerSideLwscrnl));
        assert!(!order.contains(&SkirmishShellDrawRole::ParentBackgroundMnscrns640));
        assert!(!order.contains(&SkirmishShellDrawRole::ParentBackgroundCoopGameSetup800));
    }

    #[test]
    fn preview_markers_require_real_preview_surface() {
        let order =
            skirmish_shell_semantic_draw_order(&compute_layout(800, 600), false, false, false, 0);
        assert!(!order.contains(&SkirmishShellDrawRole::PreviewSurface));
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarker));
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarkerLabel));

        let order =
            skirmish_shell_semantic_draw_order(&compute_layout(800, 600), false, true, true, 0);
        assert!(order.contains(&SkirmishShellDrawRole::PreviewSurface));
        assert!(order.contains(&SkirmishShellDrawRole::StartMarker));
        assert!(order.contains(&SkirmishShellDrawRole::StartMarkerLabel));
    }

    #[test]
    fn decoded_preview_surface_does_not_imply_start_marker_overlays() {
        let order =
            skirmish_shell_semantic_draw_order(&compute_layout(800, 600), false, true, false, 0);
        assert!(order.contains(&SkirmishShellDrawRole::PreviewSurface));
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarker));
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarkerLabel));
    }

    #[test]
    fn skirmish_preview_aspect_fit_uses_gamemd_integer_per_mille_truncation() {
        let layout = compute_layout(800, 600);
        let fitted = aspect_fit_rect(layout.map_preview, 138, 75);
        assert_eq!(fitted, RectPx::new(645, 54, 143, 78));
    }

    #[test]
    fn header_preview_points_project_with_integer_per_mille_math() {
        let bounds = PreviewSourceBounds {
            origin_x: 10,
            origin_y: 20,
            width: 100,
            height: 80,
            start_points: vec![
                crate::map::preview::PreviewStartPoint { x: 10, y: 20 },
                crate::map::preview::PreviewStartPoint { x: 60, y: 60 },
                crate::map::preview::PreviewStartPoint { x: 109, y: 99 },
            ],
        };
        let projected = project_preview_start_positions(&bounds, RectPx::new(644, 54, 144, 78));
        assert_eq!(projected, vec![(644, 54), (716, 93), (786, 130)]);
    }

    #[test]
    fn start_marker_instances_apply_verified_shape_offsets() {
        assert_eq!(start_marker_top_left(120, 70), (111, 64));
    }

    #[test]
    fn start_marker_overlays_use_destination_surface_clip_not_preview_rect() {
        let bounds = PreviewSourceBounds {
            origin_x: 0,
            origin_y: 0,
            width: 100,
            height: 100,
            start_points: vec![crate::map::preview::PreviewStartPoint { x: 140, y: 50 }],
        };
        let preview = RectPx::new(645, 54, 143, 78);

        let projected = project_preview_start_positions(&bounds, preview);

        assert_eq!(projected, vec![(845, 93)]);
        assert!(!preview.contains(projected[0].0, projected[0].1));
        assert_eq!(
            start_marker_top_left(projected[0].0, projected[0].1),
            (836, 87)
        );
        assert_eq!(
            start_marker_label_origin(projected[0].0, projected[0].1),
            (843, 87)
        );
    }

    #[test]
    fn start_marker_labels_use_startbut_overlay_origin_and_yellow_color() {
        assert_eq!(start_marker_label_origin(120, 70), (118, 64));
        assert_eq!(start_marker_label_color(), SHELL_LABEL_TEXT_RGB);
        assert_ne!(start_marker_label_color(), SHELL_BUTTON_TEXT_RGB_00000C05);
    }

    #[test]
    fn skirmish_dropdown_selected_row_fill_is_full_row() {
        let layout = compute_layout(800, 600);
        let shell = SkirmishShellState::default();
        let maps: [MapMenuEntry; 0] = [];
        let content =
            combo_dropdown_content_rect(&shell, &layout, &maps, SkirmishComboId::Color(0)).unwrap();

        let selected =
            dropdown_selected_row_rect(&shell, &maps, SkirmishComboId::Color(0), 0, content)
                .unwrap();

        assert_eq!(
            selected,
            RectPx::new(
                content.x,
                content.y + COMBO_DROPDOWN_ROW_H,
                content.w,
                COMBO_DROPDOWN_ROW_H
            )
        );
    }

    #[test]
    fn text_origin_centers_and_applies_pressed_offset() {
        let rect = RectPx::new(10, 20, 100, 30);
        assert_eq!(
            shell_text_origin(rect, 40, 10, ShellAlign::H_CENTER | ShellAlign::V_CENTER, 0),
            (40, 30)
        );
        assert_eq!(
            shell_text_origin(
                rect,
                40,
                10,
                ShellAlign::H_CENTER | ShellAlign::V_CENTER,
                PRESSED_BUTTON_CONTENT_OFFSET_Y
            ),
            (40, 32)
        );
    }

    #[test]
    fn button_text_rect_follows_owner_draw_caller_contract() {
        let rect = RectPx::new(644, 241, 156, 42);
        let released = button_text_rect(rect, false);
        let pressed = button_text_rect(rect, true);

        assert_eq!(
            (released.x, released.y, released.w, released.h),
            (644, 242, 154, 41)
        );
        assert_eq!(
            (pressed.x, pressed.y, pressed.w, pressed.h),
            (646, 246, 152, 37)
        );
    }

    #[test]
    fn shell_text_colors_follow_verified_owner_draw_sources() {
        assert_eq!(
            SHELL_BUTTON_TEXT_RGB_00000C05,
            [5.0 / 255.0, 12.0 / 255.0, 0.0]
        );
        assert_eq!(SHELL_LABEL_TEXT_RGB, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn text_origin_supports_left_and_right_alignment_flags() {
        let rect = RectPx::new(10, 20, 100, 30);
        assert_eq!(
            shell_text_origin(rect, 40, 10, ShellAlign::NONE, 0),
            (10, 20)
        );
        assert_eq!(
            shell_text_origin(rect, 40, 10, ShellAlign::H_RIGHT, 0),
            (70, 20)
        );
    }
}
