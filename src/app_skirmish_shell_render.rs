//! Skirmish shell sprite construction and render pass.
//!
//! Part of the app layer: may depend on ui and render modules. Keeps the
//! `GameScreen::MainMenu` branch in `app.rs` small.

use std::sync::Once;

use crate::app::AppState;
use crate::app_init::MapMenuEntry;
use crate::render::batch::{BatchTexture, SpriteInstance};
use crate::render::shell_text::{self, ShellAlign, ShellTextDraw, TextRect};
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::ui::main_menu::SkirmishCountry;
use crate::ui::skirmish_shell::{
    OwnerDrawButton, RectPx, SkirmishShellAction, SkirmishShellLayout, SkirmishShellState,
    compute_layout,
};

static PREVIEW_MARKER_WAIT_LOG: Once = Once::new();
static HIGH_RES_PARENT_BACKGROUND_LOG: Once = Once::new();

const PRESSED_BUTTON_CONTENT_OFFSET_Y: i32 = 2;
const START_MARKER_OFFSET_X: i32 = -9;
const START_MARKER_OFFSET_Y: i32 = -6;
const BUTTON_DISABLED_ALPHA: f32 = 0x80 as f32 / 255.0;
const SHELL_PARENT_BACKGROUND_DEPTH: f32 = 0.00090;
const SHELL_LOWER_STRIP_DEPTH: f32 = 0.00077;
const SHELL_PREVIEW_SURFACE_DEPTH: f32 = 0.00058;
// Live Ghidra recovered button text color 0x00000C05 before the original
// wrapper converted it to the active 16-bit display format; final RGB parity
// still needs screenshot comparison against retail YR.
const SHELL_BUTTON_TEXT_RGB_00000C05: [f32; 3] = [0.0, 12.0 / 255.0, 5.0 / 255.0];

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
    let right_w = right_w.round().max(1.0).min((rect_w - left_w).max(0.0));
    let mid_w = mid_w.round().max(1.0);
    let mut segments = vec![ButtonSegment {
        piece: ButtonPiece::Left,
        x: rect.x as f32,
        width: left_w,
        uv_width_ratio: 1.0,
    }];

    let mut x = rect.x as f32 + left_w;
    let end = rect.x as f32 + rect_w - right_w;
    while x < end - f32::EPSILON {
        let width = (end - x).min(mid_w);
        segments.push(ButtonSegment {
            piece: ButtonPiece::Middle,
            x,
            width,
            uv_width_ratio: width / mid_w,
        });
        x += width;
    }

    segments.push(ButtonSegment {
        piece: ButtonPiece::Right,
        x: rect.x as f32 + rect_w - right_w,
        width: right_w,
        uv_width_ratio: 1.0,
    });
    segments
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
            rect.y as f32,
            [segment.width, rect.h as f32],
            depth,
            alpha,
        );
    }
}

fn push_start_marker_sprites(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    preview_rect: RectPx,
    projected_positions: &[(i32, i32)],
    real_preview_surface_available: bool,
    depth: f32,
) {
    if !real_preview_surface_available {
        PREVIEW_MARKER_WAIT_LOG.call_once(|| {
            log::info!(
                "Skirmish shell STARTBUT.SHP markers skipped until real preview surface decode and verified source bounds are available"
            );
        });
        return;
    }
    let Some(marker) = atlas.start_marker else {
        return;
    };
    for &(x, y) in projected_positions {
        let marker_x = x + START_MARKER_OFFSET_X;
        let marker_y = y + START_MARKER_OFFSET_Y;
        if !preview_rect.contains(x, y) {
            continue;
        }
        push_entry_native(out, marker, marker_x, marker_y, depth);
    }
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

fn flag_entry(atlas: &SkirmishShellChromeAtlas, label: &str) -> Option<SkirmishShellChromeEntry> {
    atlas
        .flags
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(label))
        .map(|(_, entry)| *entry)
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
    // The binary branch is verified, but the dialog state byte that toggles it
    // is still unnamed. Keep the decision isolated for the next Ghidra pass.
    true
}

pub fn skirmish_shell_semantic_draw_order(
    layout: &SkirmishShellLayout,
    overlay_frame10_active: bool,
    preview_surface_available: bool,
    start_marker_overlay_available: bool,
    flag_count: usize,
) -> Vec<SkirmishShellDrawRole> {
    let mut roles = Vec::new();
    if let Some(role) = parent_background_role(layout) {
        roles.push(match role {
            ParentBackgroundRole::Mnscrns640 => SkirmishShellDrawRole::ParentBackgroundMnscrns640,
            ParentBackgroundRole::CoopGameSetup800 => {
                SkirmishShellDrawRole::ParentBackgroundCoopGameSetup800
            }
        });
    }
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
    shell: &SkirmishShellState,
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();

    if let Some(background) = parent_background_entry(atlas, layout) {
        push_entry_native(
            &mut instances,
            background,
            layout.screen.x,
            layout.screen.y,
            SHELL_PARENT_BACKGROUND_DEPTH,
        );
    }

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
        push_entry(&mut instances, bottom, layout.right_panel.bottom, 0.00078);
    }

    if let Some(lower_strip) = lower_strip_entry(atlas, layout) {
        push_entry(
            &mut instances,
            lower_strip,
            lower_strip_rect(layout, lower_strip),
            SHELL_LOWER_STRIP_DEPTH,
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

    push_start_marker_sprites(
        &mut instances,
        atlas,
        layout.map_preview,
        &[],
        false,
        0.00056,
    );

    if let Some(flag) =
        flag_name_for_country(shell.player_country).and_then(|name| flag_entry(atlas, name))
    {
        push_entry_fit(&mut instances, flag, layout.flags[0], 0.00057);
    }
    for idx in 1..layout.flags.len() {
        let entry = shell
            .opponents
            .get(idx - 1)
            .filter(|opponent| opponent.enabled)
            .and_then(|opponent| flag_name_for_country(opponent.country))
            .and_then(|name| flag_entry(atlas, name));
        if let Some(flag) = entry {
            push_entry_fit(&mut instances, flag, layout.flags[idx], 0.00057);
        }
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

fn push_button_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    y_offset: i32,
    depth: f32,
) {
    let text_rect = TextRect {
        x: rect.x,
        y: rect.y + y_offset,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    };
    let draw = shell_text::draw_in_rect(
        &state.bit_font,
        label,
        text_rect,
        SHELL_BUTTON_TEXT_RGB_00000C05,
        ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        [0.0, 0.0],
        depth,
    );
    out.push(draw);
}

fn build_shell_text_draws(
    state: &AppState,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) -> (Vec<ShellTextDraw>, Vec<SpriteInstance>) {
    let mut shell_draws: Vec<ShellTextDraw> = Vec::new();
    let mut bare_instances: Vec<SpriteInstance> = Vec::new();

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
        let y_off = if shell.pressed_owner_draw_button == Some(button) {
            PRESSED_BUTTON_CONTENT_OFFSET_Y
        } else {
            0
        };
        push_button_label_draw(&mut shell_draws, state, label, rect, y_off, 0.00041);
    }

    push_start_marker_labels(
        &mut bare_instances,
        state,
        layout.map_preview,
        &[],
        false,
        0.00040,
    );

    (shell_draws, bare_instances)
}

fn push_start_marker_labels(
    out: &mut Vec<SpriteInstance>,
    state: &AppState,
    preview_rect: RectPx,
    projected_positions: &[(i32, i32)],
    real_preview_surface_available: bool,
    depth: f32,
) {
    if !real_preview_surface_available {
        return;
    }
    for (idx, &(x, y)) in projected_positions.iter().enumerate() {
        if !preview_rect.contains(x, y) {
            continue;
        }
        let label = (idx + 1).to_string();
        out.extend(state.bit_font.build_text(
            &label,
            x as f32,
            y as f32,
            1.0,
            depth,
            SHELL_BUTTON_TEXT_RGB_00000C05,
            [0.0, 0.0],
        ));
    }
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
        .available_maps
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

    let scale_w = dst.w as f32 / src_w as f32;
    let scale_h = dst.h as f32 / src_h as f32;
    let scale = scale_w.min(scale_h);
    let fitted_w = (src_w as f32 * scale).round() as i32;
    let fitted_h = (src_h as f32 * scale).round() as i32;
    RectPx::new(
        dst.x + ((dst.w - fitted_w) / 2),
        dst.y + ((dst.h - fitted_h) / 2),
        fitted_w,
        fitted_h,
    )
}

fn build_preview_surface_instance(
    layout: &SkirmishShellLayout,
    preview_width: u32,
    preview_height: u32,
) -> Option<SpriteInstance> {
    let fitted = aspect_fit_rect(layout.map_preview, preview_width, preview_height);
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
    let action = SkirmishShellAction::None;

    let Some(atlas) = atlas else {
        clear_shell_target(state, encoder, target);
        return Ok(action);
    };

    ensure_selected_preview_texture(state);
    let preview_instance = state
        .skirmish_preview_texture
        .as_ref()
        .and_then(|preview| build_preview_surface_instance(&layout, preview.width, preview.height));
    let preview_buffer = preview_instance.as_ref().and_then(|instance| {
        state
            .batch_renderer
            .create_instance_buffer(&state.gpu, &[*instance])
    });

    let instances = build_skirmish_shell_instances(atlas, &layout, &state.skirmish_shell_state);
    let (shell_draws, bare_text_instances) =
        build_shell_text_draws(state, &layout, &state.skirmish_shell_state);
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

    #[test]
    fn button_segments_tile_middle_and_keep_caps() {
        let rect = RectPx::new(635, 242, 162, 37);
        let segments = build_button_segments(rect, 8.0, 64.0, 8.0);
        assert_eq!(segments.first().unwrap().piece, ButtonPiece::Left);
        assert_eq!(segments.last().unwrap().piece, ButtonPiece::Right);
        assert!(segments.iter().any(|s| s.piece == ButtonPiece::Middle));
        let total_width: f32 = segments.iter().map(|s| s.width).sum();
        assert_eq!(total_width, 162.0);
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
        assert_eq!(
            order[0],
            SkirmishShellDrawRole::ParentBackgroundCoopGameSetup800
        );
        assert_eq!(order[1], SkirmishShellDrawRole::RightPanelTopSdtp);
        assert_eq!(
            &order[2..11],
            [SkirmishShellDrawRole::RightPanelTileSdbtnbkgd; 9]
        );
        assert_eq!(
            &order[11..20],
            [SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10; 9]
        );
        assert_eq!(order[20], SkirmishShellDrawRole::RightPanelBottomSdbtm);
        assert_eq!(order[21], SkirmishShellDrawRole::LowerSideLwscrnl);
        assert_eq!(&order[22..25], [SkirmishShellDrawRole::OwnerDrawButton; 3]);
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarker));
        assert!(!order.contains(&SkirmishShellDrawRole::StartMarkerLabel));
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
    fn preview_surface_aspect_fits_dustbowl_thumbnail() {
        let layout = compute_layout(800, 600);
        let fitted = aspect_fit_rect(layout.map_preview, 138, 75);
        assert_eq!(fitted.w, 144);
        assert_eq!(fitted.h, 78);
        assert_eq!(fitted.x, layout.map_preview.x);
        assert_eq!(fitted.y, layout.map_preview.y + 17);
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
