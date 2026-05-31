//! Single Player intermediate shell render glue for dialog 0x100.

use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::app_shell_transition::{ButtonGroup, ShellFrameWave};
use crate::render::batch::SpriteInstance;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_text::ShellTextDraw;
use crate::render::shell_transition_pass::ShellRenderTarget;
use crate::ui::main_menu_shell::RectPx;
use crate::ui::single_player_shell::{
    SinglePlayerControlId, SinglePlayerShellLayout, compute_layout, csf_key_for_control,
};

const MOVIE_DEPTH: f32 = 0.00095;
const CHROME_DEPTH: f32 = 0.00085;
const BUTTON_DEPTH: f32 = 0.00080;
const TEXT_DEPTH: f32 = 0.00070;
/// Software cursor draws on top of everything else on the shell (smallest
/// depth). The original hides the OS cursor and blits the cursor SHP last.
const CURSOR_DEPTH: f32 = 0.00001;
const RIGHT_PANEL_WIDTH: i32 = 168;
const RIGHT_PANEL_TILE_H: i32 = 42;
const SHELL_BUTTON_TEXT_RGB_FFFF00: [f32; 3] = [1.0, 1.0, 0.0];
const SHELL_DISABLED_TEXT_RGB_FROM_PACKED_0000009F: [f32; 3] = [0x9F as f32 / 255.0, 0.0, 0.0];
const BUTTON_DISABLED_ALPHA: f32 = 0x80 as f32 / 255.0;

pub(crate) enum SinglePlayerShellRenderResult {
    Rendered,
    Fallback,
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

fn button_frame(
    atlas: &MainMenuShellChromeAtlas,
    pressed: bool,
    hover_highlight: bool,
) -> MainMenuShellChromeEntry {
    if pressed {
        atlas.button_pressed
    } else if hover_highlight {
        atlas.button_hover
    } else {
        atlas.button_default
    }
}

fn push_button_shp(
    out: &mut Vec<SpriteInstance>,
    atlas: &MainMenuShellChromeAtlas,
    rect: RectPx,
    pressed: bool,
    hover_highlight: bool,
    enabled: bool,
) {
    let frame = button_frame(atlas, pressed, hover_highlight);
    let scale_x = rect.w as f32 / RIGHT_PANEL_WIDTH as f32;
    let scale_y = rect.h as f32 / RIGHT_PANEL_TILE_H as f32;
    let frame_w = frame.pixel_size[0] * scale_x;
    let frame_h = frame.pixel_size[1] * scale_y;
    let x = rect.x as f32 + (rect.w as f32 - frame_w);
    let y = rect.y as f32 + (rect.h as f32 - frame_h) * 0.5;
    out.push(SpriteInstance {
        position: [x, y],
        size: [frame_w, frame_h],
        uv_origin: frame.uv_origin,
        uv_size: frame.uv_size,
        depth: BUTTON_DEPTH,
        tint: [1.0, 1.0, 1.0],
        alpha: if enabled { 1.0 } else { BUTTON_DISABLED_ALPHA },
        ..Default::default()
    });
}

fn resolve_csf<'a>(state: &'a AppState, key: &'static str) -> &'a str {
    state
        .csf
        .as_ref()
        .and_then(|csf| csf.get(key))
        .unwrap_or(key)
}

fn push_label(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    text: &str,
    rect: RectPx,
    align: crate::render::shell_text::ShellAlign,
    rgb: [f32; 3],
) {
    use crate::render::shell_text::TextRect;

    let text_rect = TextRect {
        x: rect.x,
        y: rect.y,
        w: rect.w.max(0) as u32,
        h: rect.h.max(0) as u32,
    };
    out.push(crate::render::shell_text::draw_in_rect(
        &state.bit_font,
        text,
        text_rect,
        rgb,
        align,
        [0.0, 0.0],
        TEXT_DEPTH,
        None,
    ));
}

#[allow(clippy::too_many_arguments)]
fn build_button_instances(
    atlas: &MainMenuShellChromeAtlas,
    layout: &SinglePlayerShellLayout,
    pressed_button: Option<SinglePlayerControlId>,
    hovered_button: Option<SinglePlayerControlId>,
    hover_started_at: Option<Instant>,
    load_saved_game_enabled: bool,
    now: Instant,
    wave: Option<&ShellFrameWave>,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    for (slot, button) in layout.buttons.iter().enumerate() {
        let enabled =
            button.id != SinglePlayerControlId::LoadSavedGame0x689 || load_saved_game_enabled;
        if let Some(wave) = wave {
            // First-paint slide: animate every button through Group A's ramp.
            // Disabled buttons still slide in, dimmed, matching the steady alpha.
            let frame = wave.sdbtnanm_frame(slot as u32, ButtonGroup::A);
            push_button_wave_frame(&mut out, atlas, button.rect, frame, enabled);
            continue;
        }
        let pressed = enabled && pressed_button == Some(button.id);
        let hover_highlight = if enabled && !pressed && hovered_button == Some(button.id) {
            hover_started_at
                .map(|start| now.duration_since(start).as_millis() / 1000 % 2 == 1)
                .unwrap_or(false)
        } else {
            false
        };
        push_button_shp(
            &mut out,
            atlas,
            button.rect,
            pressed,
            hover_highlight,
            enabled,
        );
    }
    out
}

/// Draw a single-player button at a first-paint slide frame, using the same
/// fit-scale and right-anchor as the steady button art. Clamps down one frame if
/// the index is missing; holds (draws nothing) if neither is baked.
fn push_button_wave_frame(
    out: &mut Vec<SpriteInstance>,
    atlas: &MainMenuShellChromeAtlas,
    rect: RectPx,
    frame_idx: usize,
    enabled: bool,
) {
    let wave_frame = |idx: usize| atlas.button_wave_frames.get(idx).copied().flatten();
    let Some(frame) = wave_frame(frame_idx).or_else(|| wave_frame(frame_idx.saturating_sub(1)))
    else {
        return;
    };
    let scale_x = rect.w as f32 / RIGHT_PANEL_WIDTH as f32;
    let scale_y = rect.h as f32 / RIGHT_PANEL_TILE_H as f32;
    let frame_w = frame.pixel_size[0] * scale_x;
    let frame_h = frame.pixel_size[1] * scale_y;
    let x = rect.x as f32 + (rect.w as f32 - frame_w);
    let y = rect.y as f32 + (rect.h as f32 - frame_h) * 0.5;
    out.push(SpriteInstance {
        position: [x, y],
        size: [frame_w, frame_h],
        uv_origin: frame.uv_origin,
        uv_size: frame.uv_size,
        depth: BUTTON_DEPTH,
        tint: [1.0, 1.0, 1.0],
        alpha: if enabled { 1.0 } else { BUTTON_DISABLED_ALPHA },
        ..Default::default()
    });
}

fn push_entry_rect(
    out: &mut Vec<SpriteInstance>,
    entry: MainMenuShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    push_entry_sized(
        out,
        entry,
        rect.x as f32,
        rect.y as f32,
        [rect.w as f32, rect.h as f32],
        depth,
    );
}

fn push_clipped_top(
    out: &mut Vec<SpriteInstance>,
    entry: MainMenuShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let native_h = entry.pixel_size[1].max(1.0);
    let visible_h = (rect.h as f32).min(native_h);
    let uv_h = entry.uv_size[1] * (visible_h / native_h);
    out.push(SpriteInstance {
        position: [rect.x as f32, rect.y as f32],
        size: [rect.w as f32, visible_h],
        uv_origin: entry.uv_origin,
        uv_size: [entry.uv_size[0], uv_h],
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    });
}

fn build_chrome_instances(
    atlas: &MainMenuShellChromeAtlas,
    layout: &SinglePlayerShellLayout,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    if let Some(top) = atlas.right_panel_top_sdtp {
        push_entry_rect(&mut out, top, layout.right_panel.top, CHROME_DEPTH);
    }
    if let Some(tile) = atlas.right_panel_tile_sdbtnbkgd {
        for row in 0..layout.right_panel.tile_count {
            let rect = RectPx::new(
                layout.right_panel.tile.x,
                layout.right_panel.tile.y + row * layout.right_panel.tile.h,
                layout.right_panel.tile.w,
                layout.right_panel.tile.h,
            );
            push_entry_rect(&mut out, tile, rect, CHROME_DEPTH);
        }
    }
    if let Some(bottom) = atlas.right_panel_bottom_sdbtm {
        push_clipped_top(&mut out, bottom, layout.right_panel.bottom, CHROME_DEPTH);
    }
    let lower_strip_entry = if layout.screen.w == 640 {
        atlas.lower_side_640_lwscrns
    } else {
        atlas.lower_side_large_lwscrnl
    };
    if let Some(strip) = lower_strip_entry {
        push_entry_rect(&mut out, strip, layout.lower_strip, CHROME_DEPTH);
    }
    out
}

fn build_text_draws(state: &AppState, layout: &SinglePlayerShellLayout) -> Vec<ShellTextDraw> {
    use crate::render::shell_text::ShellAlign;

    let mut out = Vec::new();
    let button_align = ShellAlign::H_CENTER | ShellAlign::V_CENTER;
    for button in &layout.buttons {
        let enabled = button.id != SinglePlayerControlId::LoadSavedGame0x689
            || state.single_player_shell_state.load_saved_game_enabled;
        let text = resolve_csf(state, csf_key_for_control(button.id));
        let x_offset = if state.single_player_shell_state.pressed_owner_draw_button
            == Some(button.id)
            && enabled
        {
            layout.pressed_content_offset_x
        } else {
            0
        };
        let text_rect = RectPx::new(
            button.rect.x + x_offset,
            button.rect.y + 1,
            (button.rect.w - 2).max(0),
            (button.rect.h - 1).max(0),
        );
        push_label(
            &mut out,
            state,
            text,
            text_rect,
            button_align,
            if enabled {
                SHELL_BUTTON_TEXT_RGB_FFFF00
            } else {
                SHELL_DISABLED_TEXT_RGB_FROM_PACKED_0000009F
            },
        );
    }

    let title = resolve_csf(state, "GUI:SinglePlayerMenu");
    push_label(
        &mut out,
        state,
        title,
        layout.title,
        ShellAlign::H_CENTER,
        SHELL_BUTTON_TEXT_RGB_FFFF00,
    );

    out
}

fn movie_instance(layout: &SinglePlayerShellLayout) -> SpriteInstance {
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

/// Build the software-cursor sprite for the single-player shell.
///
/// The shell renders in screen space with the camera at (0,0), so the cursor
/// sits at the raw pointer position minus its hotspot — same convention as the
/// main-menu shell. Returns None when no software cursor is loaded; the OS
/// cursor is hidden process-wide, so without this the shell shows no pointer.
fn shell_cursor_instance(state: &AppState) -> Option<SpriteInstance> {
    let cursor = state.software_cursor.as_ref()?;
    let sequence = cursor.get(crate::app_types::CursorId::Default)?;
    let frame = crate::app_cursor::current_software_cursor_frame(sequence)?;
    Some(SpriteInstance {
        position: [
            state.cursor_x - sequence.hotspot[0],
            state.cursor_y - sequence.hotspot[1],
        ],
        size: [frame.width, frame.height],
        uv_origin: [0.0, 0.0],
        uv_size: [1.0, 1.0],
        depth: CURSOR_DEPTH,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    })
}

pub(crate) fn render_single_player_shell(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> Result<SinglePlayerShellRenderResult> {
    crate::app_main_menu_shell_render::ensure_movie_for_current_layout(state)?;
    if state.main_menu_shell_failed || state.main_menu_shell_chrome.is_none() {
        state.main_menu_shell_failed = true;
        return Ok(SinglePlayerShellRenderResult::Fallback);
    }

    if let Some(movie) = state.main_menu_movie.as_mut() {
        let now = Instant::now();
        let elapsed = now
            .duration_since(state.main_menu_movie_last_step)
            .as_secs_f64();
        state.main_menu_movie_last_step = now;
        if let Err(err) = movie.step(&state.gpu, elapsed) {
            log::warn!("Failed to step single-player RA2TS movie: {err:#}");
            state.main_menu_shell_failed = true;
            return Ok(SinglePlayerShellRenderResult::Fallback);
        }
    }

    let depth = state.depth_view.clone();
    let target = ShellRenderTarget {
        color: target,
        depth: &depth,
    };
    let layout = compute_layout(state.gpu.config.width, state.gpu.config.height);
    // While a first-paint slide is live the buttons animate through their
    // SDBTNANM ramp frames; off-slide this is None and they paint steady-state.
    let wave = state.shell_first_paint_slide.clone();
    let chrome = state
        .main_menu_shell_chrome
        .as_ref()
        .expect("checked before render");
    let movie_texture = state
        .main_menu_movie
        .as_ref()
        .map(|movie| movie.batch_texture())
        .expect("movie loaded before render");

    let movie_instances = vec![movie_instance(&layout)];
    let chrome_instances = build_chrome_instances(chrome, &layout);
    let button_instances = build_button_instances(
        chrome,
        &layout,
        state.single_player_shell_state.pressed_owner_draw_button,
        state.single_player_shell_state.hovered_owner_draw_button,
        state.single_player_shell_state.hover_started_at,
        state.single_player_shell_state.load_saved_game_enabled,
        Instant::now(),
        wave.as_ref(),
    );
    let text_draws = build_text_draws(state, &layout);

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
    let chrome_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &chrome_instances);
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
    let cursor_instances: Vec<SpriteInstance> =
        shell_cursor_instance(state).into_iter().collect();
    let cursor_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &cursor_instances);
    // Default-cursor frame-0 texture, borrowed for the duration of the pass.
    let cursor_texture = state
        .software_cursor
        .as_ref()
        .and_then(|cursor| cursor.get(crate::app_types::CursorId::Default))
        .and_then(|sequence| sequence.frames.first())
        .map(|frame| &frame.texture);

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Single Player Shell"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target.color,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app_types::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: target.depth,
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
    if let Some((buffer, count)) = chrome_buffer.as_ref() {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &chrome.texture,
            buffer,
            *count,
        );
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
    // Software cursor draws last, on top of all chrome/controls.
    if let (Some((buffer, count)), Some(texture)) = (cursor_buffer.as_ref(), cursor_texture) {
        state
            .batch_renderer
            .draw_with_buffer_passthrough(&mut pass, texture, buffer, *count);
    }
    drop(pass);

    Ok(SinglePlayerShellRenderResult::Rendered)
}
