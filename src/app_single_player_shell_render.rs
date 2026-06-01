//! Single Player intermediate shell render glue for dialog 0x100.

use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::app_shell_transition::{ButtonGroup, ShellFrameWave};
use crate::render::batch::SpriteInstance;
use crate::render::shell_paint::{
    self, ArtFit, ButtonPolicy, PaintButton, PaintLabel, CURSOR_DEPTH, MOVIE_DEPTH,
    SHELL_TEXT_RGB_DISABLED, SHELL_TEXT_RGB_ENABLED,
};
use crate::render::shell_transition_pass::ShellRenderTarget;
use crate::ui::main_menu_shell::RectPx;
use crate::ui::single_player_shell::{
    SinglePlayerControlId, SinglePlayerShellLayout, compute_layout, csf_key_for_control,
};

/// SDBTNANM canvas fit divisors for the 0x100 fit-scaled art (rect.w/168,
/// rect.h/42, then right-anchor + v-center).
const RIGHT_PANEL_WIDTH: f32 = 168.0;
const RIGHT_PANEL_TILE_H: f32 = 42.0;

/// Dialog 0x100 owner-draw button policy: art fit-scaled into the cell and
/// right-anchored / v-centered, ~1 Hz hover flash, NO art Y sink on press
/// (only the text shifts +1 px), and disabled controls dimmed (alpha 0.502 +
/// #9F0000 text). The disabled state fires when no save games exist
/// (LoadSavedGame).
const SP_BUTTON_POLICY: ButtonPolicy = ButtonPolicy {
    art_fit: ArtFit::FitRightAnchored {
        panel_w: RIGHT_PANEL_WIDTH,
        tile_h: RIGHT_PANEL_TILE_H,
    },
    hover_flash: true,
    art_sink_y: 0.0,
    disabled_dim: true,
};

pub(crate) enum SinglePlayerShellRenderResult {
    Rendered,
    Fallback,
}

fn resolve_csf<'a>(state: &'a AppState, key: &'static str) -> &'a str {
    state
        .csf
        .as_ref()
        .and_then(|csf| csf.get(key))
        .unwrap_or(key)
}

/// Map the layout + shell state into the owner-draw button list for the paint
/// pass. A disabled LoadSavedGame control can never paint pressed (frame 4) or
/// hover (frame 3): pressed/hovered are gated on `enabled`, matching the prior
/// emitter. During a first-paint slide each button rides Group A's ramp (a
/// disabled button still slides in, dimmed, via the policy).
fn sp_paint_buttons(
    layout: &SinglePlayerShellLayout,
    pressed_button: Option<SinglePlayerControlId>,
    hovered_button: Option<SinglePlayerControlId>,
    load_saved_game_enabled: bool,
    wave: Option<&ShellFrameWave>,
) -> Vec<PaintButton> {
    layout
        .buttons
        .iter()
        .enumerate()
        .map(|(slot, button)| {
            let enabled = button.id != SinglePlayerControlId::LoadSavedGame0x689
                || load_saved_game_enabled;
            let wave_frame =
                wave.map(|w| w.sdbtnanm_frame(slot as u32, ButtonGroup::A) as usize);
            PaintButton {
                rect: button.rect,
                pressed: enabled && pressed_button == Some(button.id),
                hovered: enabled && hovered_button == Some(button.id),
                enabled,
                wave_frame,
            }
        })
        .collect()
}

/// Build the owner-draw button labels + the single title static. Reproduces the
/// prior `build_text_draws`: button labels h+v-centered in a rect inset by
/// top+=1 / right-=2, shifted +x_offset on press (NO Y sink on 0x100), colored
/// #FFFF00 when enabled / #9F0000 when disabled; the title is h-centered
/// top-anchored #FFFF00. The SP `status_help` / `side_image_static` rects exist
/// in the layout but are NOT drawn — kept that way.
fn sp_paint_labels<'a>(
    state: &'a AppState,
    layout: &SinglePlayerShellLayout,
) -> Vec<PaintLabel<'a>> {
    use crate::render::shell_text::ShellAlign;
    let mut out = Vec::new();
    let button_align = ShellAlign::H_CENTER | ShellAlign::V_CENTER;
    for button in &layout.buttons {
        let enabled = button.id != SinglePlayerControlId::LoadSavedGame0x689
            || state.single_player_shell_state.load_saved_game_enabled;
        let x_offset = if state.single_player_shell_state.pressed_owner_draw_button
            == Some(button.id)
            && enabled
        {
            layout.pressed_content_offset_x
        } else {
            0
        };
        out.push(PaintLabel {
            text: resolve_csf(state, csf_key_for_control(button.id)),
            rect: RectPx::new(
                button.rect.x + x_offset,
                button.rect.y + 1,
                (button.rect.w - 2).max(0),
                (button.rect.h - 1).max(0),
            ),
            align: button_align,
            rgb: if enabled {
                SHELL_TEXT_RGB_ENABLED
            } else {
                SHELL_TEXT_RGB_DISABLED
            },
        });
    }
    out.push(PaintLabel {
        text: resolve_csf(state, "GUI:SinglePlayerMenu"),
        rect: layout.title,
        align: ShellAlign::H_CENTER,
        rgb: SHELL_TEXT_RGB_ENABLED,
    });
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

    // 0x100 has NO parent background; the movie is submitted first.
    let movie_instances = vec![movie_instance(&layout)];
    let chrome_instances =
        shell_paint::paint_chrome(chrome, layout.right_panel, Some(layout.lower_strip), layout.screen.w);
    let buttons = sp_paint_buttons(
        &layout,
        state.single_player_shell_state.pressed_owner_draw_button,
        state.single_player_shell_state.hovered_owner_draw_button,
        state.single_player_shell_state.load_saved_game_enabled,
        wave.as_ref(),
    );
    let button_instances = shell_paint::paint_buttons(
        chrome,
        &buttons,
        SP_BUTTON_POLICY,
        Instant::now(),
        state.single_player_shell_state.hover_started_at,
    );
    let labels = sp_paint_labels(state, &layout);
    let text_draws = shell_paint::paint_labels(&state.bit_font, &labels);

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
