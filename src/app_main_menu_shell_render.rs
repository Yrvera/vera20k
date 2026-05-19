//! Initial main-menu shell render glue for dialog 0xE2.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::render::batch::SpriteInstance;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_text::ShellTextDraw;
use crate::ui::main_menu_shell::{
    MainMenuControlId, MainMenuShellLayout, RectPx, compute_responsive_layout, csf_key_for_control,
};

const MOVIE_DEPTH: f32 = 0.00095;
const BUTTON_DEPTH: f32 = 0.00080;
const TEXT_DEPTH: f32 = 0.00070;
const RETAIL_BUTTON_CLIENT_W: f32 = 162.0;
const RETAIL_BUTTON_CLIENT_H: f32 = 37.0;
const SHELL_BUTTON_TEXT_RGB_FFFF00: [f32; 3] = [1.0, 1.0, 0.0];

pub(crate) enum MainMenuShellRenderResult {
    Rendered,
    Fallback,
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

fn push_entry_sized(
    out: &mut Vec<SpriteInstance>,
    entry: MainMenuShellChromeEntry,
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

fn button_entries(
    atlas: &MainMenuShellChromeAtlas,
    pressed: bool,
) -> (
    MainMenuShellChromeEntry,
    MainMenuShellChromeEntry,
    MainMenuShellChromeEntry,
) {
    if pressed {
        (
            atlas.button_down_left_30,
            atlas.button_down_mid_30,
            atlas.button_down_right_30,
        )
    } else {
        (
            atlas.button_up_left_30,
            atlas.button_up_mid_30,
            atlas.button_up_right_30,
        )
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

fn button_art_y_and_height(rect: RectPx, source_h: f32) -> (f32, f32) {
    let scale_y = (rect.h as f32 / RETAIL_BUTTON_CLIENT_H).max(0.0);
    let art_h = (source_h * scale_y).round().max(1.0);
    let art_y = rect.y as f32 + ((rect.h as f32 - art_h) / 2.0).trunc();
    (art_y, art_h)
}

fn push_button_30(
    out: &mut Vec<SpriteInstance>,
    atlas: &MainMenuShellChromeAtlas,
    rect: RectPx,
    pressed: bool,
    depth: f32,
) {
    let (left, mid, right) = button_entries(atlas, pressed);
    let scale_x = (rect.w as f32 / RETAIL_BUTTON_CLIENT_W).max(0.0);
    let (art_y, art_h) = button_art_y_and_height(rect, left.pixel_size[1]);
    for segment in build_button_segments(
        rect,
        left.pixel_size[0] * scale_x,
        mid.pixel_size[0] * scale_x,
        right.pixel_size[0] * scale_x,
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
        push_entry_sized(out, entry, segment.x, art_y, [segment.width, art_h], depth);
    }
}

fn resolve_csf<'a>(state: &'a AppState, key: &'static str) -> &'a str {
    state
        .csf
        .as_ref()
        .and_then(|csf| csf.get(key))
        .unwrap_or(key)
}

// This wrapper is the only main-menu label path; keep placement fixes here.
fn push_centered_label(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    text: &str,
    rect: RectPx,
    y_offset: i32,
) {
    use crate::render::shell_text::{ShellAlign, TextRect};

    let text_rect = TextRect {
        x: rect.x,
        y: rect.y + y_offset,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    };
    out.push(crate::render::shell_text::draw_in_rect(
        &state.bit_font,
        text,
        text_rect,
        SHELL_BUTTON_TEXT_RGB_FFFF00,
        ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        [0.0, 0.0],
        TEXT_DEPTH,
    ));
}

fn build_button_instances(
    atlas: &MainMenuShellChromeAtlas,
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    for button in &layout.buttons {
        push_button_30(
            &mut out,
            atlas,
            button.rect,
            pressed_button == Some(button.id),
            BUTTON_DEPTH,
        );
    }
    out
}

fn build_text_draws(
    state: &AppState,
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
) -> Vec<ShellTextDraw> {
    let mut out = Vec::new();
    for button in &layout.buttons {
        let text = resolve_csf(state, csf_key_for_control(button.id));
        let y_offset = if pressed_button == Some(button.id) {
            layout.pressed_content_offset_y
        } else {
            0
        };
        push_centered_label(&mut out, state, text, button.rect, y_offset);
    }
    let title = resolve_csf(state, "GUI:MainMenu");
    push_centered_label(&mut out, state, title, layout.title, 0);
    out
}

fn movie_instance(layout: &MainMenuShellLayout) -> SpriteInstance {
    SpriteInstance {
        position: [layout.movie.x as f32, layout.movie.y as f32],
        size: [layout.movie.w as f32, layout.movie.h as f32],
        uv_origin: [0.0, 0.0],
        uv_size: [1.0, 1.0],
        depth: MOVIE_DEPTH,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    }
}

fn build_movie_instances(layout: &MainMenuShellLayout) -> Vec<SpriteInstance> {
    vec![movie_instance(layout)]
}

pub(crate) fn ensure_movie_for_current_layout(state: &mut AppState) -> Result<()> {
    let layout = compute_responsive_layout(state.gpu.config.width, state.gpu.config.height);
    if state.main_menu_movie_base == Some(layout.movie_base) && state.main_menu_movie.is_some() {
        return Ok(());
    }

    let Some(assets) = state.asset_manager.as_ref() else {
        state.main_menu_shell_failed = true;
        return Ok(());
    };
    let asset_name = layout.movie_base.asset_name();
    let Some((bytes, source)) = assets.get_with_source_ref(asset_name) else {
        log::warn!("Missing main-menu RA2TS movie asset {asset_name}");
        state.main_menu_shell_failed = true;
        return Ok(());
    };
    if asset_name.eq_ignore_ascii_case("ra2ts_l.bik")
        && !source.eq_ignore_ascii_case("language.mix")
    {
        log::warn!(
            "ra2ts_l.bik resolved from {source}; retail duplicate priority expected language.mix when both language.mix and langmd.mix contain the file"
        );
    }

    let movie = match crate::render::bink_movie::BinkMovieSurface::from_bytes(
        &state.gpu,
        &state.batch_renderer,
        Arc::<[u8]>::from(bytes),
        source.to_string(),
        true,
    ) {
        Ok(movie) => movie,
        Err(err) => {
            log::warn!("Failed to load main-menu RA2TS movie {asset_name} from {source}: {err:#}");
            state.main_menu_shell_failed = true;
            return Ok(());
        }
    };
    log::info!(
        "Loaded {asset_name} for main menu from {}",
        movie.source_archive()
    );
    state.main_menu_movie = Some(movie);
    state.main_menu_movie_base = Some(layout.movie_base);
    state.main_menu_movie_last_step = Instant::now();
    Ok(())
}

pub(crate) fn render_main_menu_shell(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> Result<MainMenuShellRenderResult> {
    ensure_movie_for_current_layout(state)?;
    if state.main_menu_shell_failed || state.main_menu_shell_chrome.is_none() {
        state.main_menu_shell_failed = true;
        return Ok(MainMenuShellRenderResult::Fallback);
    }

    if let Some(movie) = state.main_menu_movie.as_mut() {
        let now = Instant::now();
        let elapsed = now
            .duration_since(state.main_menu_movie_last_step)
            .as_secs_f64();
        state.main_menu_movie_last_step = now;
        if let Err(err) = movie.step(&state.gpu, elapsed) {
            log::warn!("Failed to step main-menu RA2TS movie: {err:#}");
            state.main_menu_shell_failed = true;
            return Ok(MainMenuShellRenderResult::Fallback);
        }
    }

    let layout = compute_responsive_layout(state.gpu.config.width, state.gpu.config.height);
    let chrome = state
        .main_menu_shell_chrome
        .as_ref()
        .expect("checked before render");
    let movie_texture = state
        .main_menu_movie
        .as_ref()
        .map(|movie| movie.batch_texture())
        .expect("movie loaded before render");

    let movie_instances = build_movie_instances(&layout);
    let button_instances = build_button_instances(
        chrome,
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
    );
    let text_draws = build_text_draws(
        state,
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
    );

    state.batch_renderer.update_camera(
        &state.gpu,
        state.gpu.config.width as f32,
        state.gpu.config.height as f32,
        0.0,
        0.0,
        1.0,
    );
    let movie_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &movie_instances);
    let button_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &button_instances);
    let text_buffers: Vec<_> = text_draws
        .iter()
        .map(|draw| {
            state
                .batch_renderer
                .create_instance_buffer(&state.gpu, &draw.instances)
        })
        .collect();

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Main Menu Shell"),
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
    if let Some((buffer, count)) = movie_buffer.as_ref() {
        state
            .batch_renderer
            .draw_with_buffer_passthrough(&mut pass, movie_texture, buffer, *count);
    }
    if let Some((buffer, count)) = button_buffer.as_ref() {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &chrome.texture,
            buffer,
            *count,
        );
    }
    for (draw, buffer) in text_draws.iter().zip(text_buffers.iter()) {
        let Some((buffer, count)) = buffer.as_ref() else {
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
    pass.set_scissor_rect(0, 0, state.gpu.config.width, state.gpu.config.height);
    drop(pass);

    Ok(MainMenuShellRenderResult::Rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::main_menu_shell::compute_layout;

    #[test]
    fn button_segments_tile_middle_and_keep_caps() {
        let rect = RectPx::new(638, 203, 162, 37);
        let segments = build_button_segments(rect, 10.0, 8.0, 10.0);
        assert_eq!(segments.first().unwrap().piece, ButtonPiece::Left);
        assert_eq!(segments.last().unwrap().piece, ButtonPiece::Right);
        let total: f32 = segments.iter().map(|s| s.width).sum();
        assert_eq!(total.round() as i32, rect.w);
    }

    #[test]
    fn button_instances_center_scaled_30px_art_in_client() {
        let (y, h) = button_art_y_and_height(RectPx::new(638, 203, 162, 37), 30.0);
        assert_eq!(y, 206.0);
        assert_eq!(h, 30.0);

        let (y, h) = button_art_y_and_height(RectPx::new(1276, 305, 324, 55), 30.0);
        assert_eq!(y, 310.0);
        assert_eq!(h, 45.0);
    }

    #[test]
    fn movie_instance_uses_layout_movie_rect() {
        let layout = compute_layout(800, 600);
        let instance = movie_instance(&layout);
        assert_eq!(instance.position, [0.0, 0.0]);
        assert_eq!(instance.size, [632.0, 570.0]);
    }
}
