//! Skirmish shell sprite construction and render pass.
//!
//! Part of the app layer: may depend on ui and render modules. Keeps the
//! `GameScreen::MainMenu` branch in `app.rs` small.

use std::sync::Once;

use crate::app::AppState;
use crate::render::batch::SpriteInstance;
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::ui::main_menu::SkirmishCountry;
use crate::ui::skirmish_shell::{
    compute_layout, OwnerDrawButton, RectPx, SkirmishShellAction, SkirmishShellLayout,
    SkirmishShellState,
};

static PREVIEW_MARKER_WAIT_LOG: Once = Once::new();

const PRESSED_BUTTON_CONTENT_OFFSET_Y: i32 = 2;
const START_MARKER_OFFSET_X: i32 = -9;
const START_MARKER_OFFSET_Y: i32 = -6;
const BUTTON_DISABLED_ALPHA: f32 = 0x80 as f32 / 255.0;
const SHELL_PARENT_BACKGROUND_DEPTH: f32 = 0.00090;
// Live Ghidra recovered button text color 0x00000C05 before the original
// wrapper converted it to the active 16-bit display format; final RGB parity
// still needs screenshot comparison against retail YR.
const SHELL_BUTTON_TEXT_RGB_00000C05: [f32; 3] = [0.0, 12.0 / 255.0, 5.0 / 255.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentBackgroundRole {
    Mnscrnl640,
    CoopGameSetup800,
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
#[allow(dead_code)]
enum ShellTextAlign {
    Left,
    Center,
    Right,
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
        640 => Some(ParentBackgroundRole::Mnscrnl640),
        800 => Some(ParentBackgroundRole::CoopGameSetup800),
        _ => None,
    }
}

fn parent_background_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> Option<SkirmishShellChromeEntry> {
    match parent_background_role(layout)? {
        ParentBackgroundRole::Mnscrnl640 => atlas.background_640_mnscrnl,
        ParentBackgroundRole::CoopGameSetup800 => atlas.background_800_coop_game_setup,
    }
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

    if let Some(top) = atlas.sd_top {
        push_entry(&mut instances, top, layout.right_panel.top, 0.00080);
    }

    if let Some(tile) = atlas.sd_tile {
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

    if let Some(bottom) = atlas.sd_bottom {
        push_entry(&mut instances, bottom, layout.right_panel.bottom, 0.00078);
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

fn push_centered_text(
    out: &mut Vec<SpriteInstance>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    align: ShellTextAlign,
    vertical_center: bool,
    y_offset: i32,
    depth: f32,
) {
    let scale = 1.0;
    let text_w = state.sidebar_text.text_width(label) * scale;
    let text_h = state.sidebar_text.glyph_height() * scale;
    let (x, y) = shell_text_origin(text_w, text_h, rect, align, vertical_center, y_offset);
    // The current sprite-font path does not clip individual glyphs. Keep the
    // button labels inside the known owner-draw rects until the exact shell
    // bitfont clipping wrapper is ported.
    if text_w > rect.w as f32 || text_h > rect.h as f32 {
        return;
    }
    out.extend(state.sidebar_text.build_text(
        label,
        x,
        y,
        scale,
        depth,
        SHELL_BUTTON_TEXT_RGB_00000C05,
        [0.0, 0.0],
    ));
}

fn build_shell_text_instances(
    state: &AppState,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();
    let start = localized_label(state, "GUI:StartGame", "Start Game");
    let choose = localized_label(state, "GUI:ChooseMap", "Choose Map");
    let back = localized_label(state, "GUI:Back", "Back");
    push_centered_text(
        &mut instances,
        state,
        &start,
        layout.start_button,
        ShellTextAlign::Center,
        true,
        if shell.pressed_owner_draw_button == Some(OwnerDrawButton::StartGame0x617) {
            PRESSED_BUTTON_CONTENT_OFFSET_Y
        } else {
            0
        },
        0.00041,
    );
    push_centered_text(
        &mut instances,
        state,
        &choose,
        layout.choose_map_button,
        ShellTextAlign::Center,
        true,
        if shell.pressed_owner_draw_button == Some(OwnerDrawButton::ChooseMap0x5aa) {
            PRESSED_BUTTON_CONTENT_OFFSET_Y
        } else {
            0
        },
        0.00041,
    );
    push_centered_text(
        &mut instances,
        state,
        &back,
        layout.back_button,
        ShellTextAlign::Center,
        true,
        if shell.pressed_owner_draw_button == Some(OwnerDrawButton::Back0x5c0) {
            PRESSED_BUTTON_CONTENT_OFFSET_Y
        } else {
            0
        },
        0.00041,
    );
    push_start_marker_labels(
        &mut instances,
        state,
        layout.map_preview,
        &[],
        false,
        0.00040,
    );
    instances
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
        out.extend(state.sidebar_text.build_text(
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

fn shell_text_origin(
    text_w: f32,
    text_h: f32,
    rect: RectPx,
    align: ShellTextAlign,
    vertical_center: bool,
    y_offset: i32,
) -> (f32, f32) {
    let x = match align {
        ShellTextAlign::Left => rect.x as f32,
        ShellTextAlign::Center => rect.x as f32 + ((rect.w as f32 - text_w) * 0.5).round(),
        ShellTextAlign::Right => rect.x as f32 + (rect.w as f32 - text_w).round(),
    };
    let y = if vertical_center {
        rect.y as f32 + ((rect.h as f32 - text_h) * 0.5).round()
    } else {
        rect.y as f32
    };
    (x, y + y_offset as f32)
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

    let instances = build_skirmish_shell_instances(atlas, &layout, &state.skirmish_shell_state);
    let text_instances = build_shell_text_instances(state, &layout, &state.skirmish_shell_state);
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
    let text_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &text_instances);

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
    if let Some((buffer, count)) = text_buffer.as_ref() {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            state.sidebar_text.texture(),
            buffer,
            *count,
        );
    }
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
            Some(ParentBackgroundRole::Mnscrnl640)
        );
        assert_eq!(
            parent_background_role(&compute_layout(800, 600)),
            Some(ParentBackgroundRole::CoopGameSetup800)
        );
        assert_eq!(parent_background_role(&compute_layout(1024, 768)), None);
    }

    #[test]
    fn text_origin_centers_and_applies_pressed_offset() {
        let rect = RectPx::new(100, 50, 162, 37);
        let (x, y) = shell_text_origin(80.0, 10.0, rect, ShellTextAlign::Center, true, 0);
        assert_eq!(x, 141.0);
        assert_eq!(y, 64.0);
        let (_, pressed_y) = shell_text_origin(
            80.0,
            10.0,
            rect,
            ShellTextAlign::Center,
            true,
            PRESSED_BUTTON_CONTENT_OFFSET_Y,
        );
        assert_eq!(pressed_y, y + 2.0);
    }

    #[test]
    fn text_origin_supports_left_and_right_alignment_flags() {
        let rect = RectPx::new(100, 50, 162, 37);
        assert_eq!(
            shell_text_origin(80.0, 10.0, rect, ShellTextAlign::Left, false, 0),
            (100.0, 50.0)
        );
        assert_eq!(
            shell_text_origin(80.0, 10.0, rect, ShellTextAlign::Right, false, 0),
            (182.0, 50.0)
        );
    }
}
