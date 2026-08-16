//! Single Player intermediate shell render glue for dialog 0x100.

use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::app::frontend::shell_transition::{ButtonGroup, ShellFrameWave};
use crate::render::batch::SpriteInstance;
use crate::render::shell_paint::{
    self, ArtFit, ButtonPolicy, CURSOR_DEPTH, MOVIE_DEPTH, PaintButton, PaintLabel,
    SHELL_TEXT_RGB_DISABLED, SHELL_TEXT_RGB_ENABLED,
};
use crate::render::shell_text::ShellAlign;
use crate::render::shell_transition_pass::ShellRenderTarget;
use crate::ui::main_menu_shell::RectPx;
use crate::ui::single_player_shell::{
    SinglePlayerControlId, SinglePlayerShellLayout, compute_layout, csf_key_for_control,
};

/// Dialog 0x100 paints the native 156x42 SDBTNANM frame at the control origin.
/// Mouse hover updates static 0x695 but does not select frame 3. Press selects
/// frame 4 without moving the art; disabled LoadSavedGame remains dimmed.
const SP_BUTTON_POLICY: ButtonPolicy = ButtonPolicy {
    art_fit: ArtFit::Native,
    hover_flash: false,
    art_sink_y: 0.0,
    disabled_dim: true,
};
const SP_BUTTON_ALIGN: ShellAlign = ShellAlign(ShellAlign::H_CENTER.0 | ShellAlign::V_CENTER.0);
const SP_STATUS_ALIGN: ShellAlign = ShellAlign::V_CENTER;

pub(crate) enum SinglePlayerShellRenderResult {
    Rendered,
    Fallback,
}

fn resolve_csf<'a>(state: &'a AppState, key: &'static str) -> std::borrow::Cow<'a, str> {
    state
        .csf
        .as_ref()
        .map(|csf| csf.text(key))
        .unwrap_or(std::borrow::Cow::Borrowed(key))
}

/// Map the layout + shell state into the owner-draw button list for the paint
/// pass. A disabled LoadSavedGame control can never paint pressed; the raw
/// controller hover remains available separately for static 0x695. During a
/// first-paint slide each button rides Group A's ramp (a disabled button still
/// slides in, dimmed, via the policy).
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
            let enabled =
                button.id != SinglePlayerControlId::LoadSavedGame0x689 || load_saved_game_enabled;
            let wave_frame = wave.map(|w| w.sdbtnanm_frame(slot as u32, ButtonGroup::A) as usize);
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

/// Native owner-draw label clip: unpressed `(x, y+1, w-2, h-1)`, pressed
/// `(x+2, y+5, w-4, h-5)`.
fn owner_draw_button_label_rect(rect: RectPx, pressed: bool) -> RectPx {
    let (dx, dy) = if pressed { (2, 5) } else { (0, 1) };
    RectPx::new(
        rect.x + dx,
        rect.y + dy,
        (rect.w - 2 - dx).max(0),
        (rect.h - dy).max(0),
    )
}

fn single_player_status_csf_key(hovered: Option<SinglePlayerControlId>) -> Option<&'static str> {
    hovered.map(SinglePlayerControlId::tooltip_csf_key)
}

/// Build the owner-draw button labels, title, and immediate 0x695 hover-help
/// static. Button labels use the disabled color for unavailable LoadSavedGame;
/// status help remains enabled yellow when that disabled control is hovered.
fn sp_paint_labels<'a>(
    state: &'a AppState,
    layout: &SinglePlayerShellLayout,
) -> Vec<PaintLabel<'a>> {
    let mut out = Vec::with_capacity(layout.buttons.len() + 2);
    for button in &layout.buttons {
        let enabled = button.id != SinglePlayerControlId::LoadSavedGame0x689
            || state.single_player_shell_state.load_saved_game_enabled;
        let pressed =
            enabled && state.single_player_shell_state.pressed_owner_draw_button == Some(button.id);
        out.push(PaintLabel {
            text: resolve_csf(state, csf_key_for_control(button.id)),
            rect: owner_draw_button_label_rect(button.rect, pressed),
            align: SP_BUTTON_ALIGN,
            rgb: if enabled {
                SHELL_TEXT_RGB_ENABLED
            } else {
                SHELL_TEXT_RGB_DISABLED
            },
            path_a_reveal: None,
        });
    }
    out.push(PaintLabel {
        text: resolve_csf(state, "GUI:SinglePlayerMenu"),
        rect: layout.title,
        align: ShellAlign::H_CENTER,
        rgb: SHELL_TEXT_RGB_ENABLED,
        path_a_reveal: None,
    });
    if let Some(key) =
        single_player_status_csf_key(state.single_player_shell_state.hovered_owner_draw_button)
    {
        out.push(PaintLabel {
            text: resolve_csf(state, key),
            rect: layout.status_help,
            align: SP_STATUS_ALIGN,
            rgb: SHELL_TEXT_RGB_ENABLED,
            path_a_reveal: None,
        });
    }
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
    let sequence = cursor.get(crate::app::types::CursorId::Default)?;
    let frame = crate::app::input::cursor::current_software_cursor_frame(sequence)?;
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
    destination: &wgpu::Texture,
) -> Result<SinglePlayerShellRenderResult> {
    crate::app::frontend::main_menu_shell_render::ensure_movie_for_current_layout(
        state,
        crate::app::frontend::main_menu_shell_render::Ra2tsDialogOwner::SinglePlayer0x100,
    )?;
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
        if let Err(err) = movie.step(&state.renderer.gpu, elapsed) {
            log::warn!("Failed to step single-player RA2TS movie: {err:#}");
            state.main_menu_shell_failed = true;
            return Ok(SinglePlayerShellRenderResult::Fallback);
        }
    }

    let color = state.renderer.shell_surface_presenter.source_render_view();
    let depth = state.renderer.depth_view.clone();
    let target = ShellRenderTarget {
        color: &color,
        depth: &depth,
    };
    let layout = compute_layout(state.renderer.gpu.config.width, state.renderer.gpu.config.height);
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
    let chrome_instances = shell_paint::paint_chrome(
        chrome,
        layout.right_panel,
        Some(layout.lower_strip),
        layout.screen.w,
    );
    let buttons = sp_paint_buttons(
        &layout,
        state.single_player_shell_state.pressed_owner_draw_button,
        state.single_player_shell_state.hovered_owner_draw_button,
        state.single_player_shell_state.load_saved_game_enabled,
        wave.as_ref(),
    );
    let button_instances =
        shell_paint::paint_buttons(chrome, &buttons, SP_BUTTON_POLICY, Instant::now(), None);
    let labels = sp_paint_labels(state, &layout);
    let text_draws = shell_paint::paint_labels(&state.renderer.bit_font, &labels);

    state.renderer.batch_renderer.update_camera(
        &state.renderer.gpu,
        state.renderer.gpu.config.width as f32,
        state.renderer.gpu.config.height as f32,
        0.0,
        0.0,
        1.0,
    );
    let movie_buffer = state
        .renderer.batch_renderer
        .create_instance_buffer(&state.renderer.gpu, &movie_instances);
    let chrome_buffer = state
        .renderer.batch_renderer
        .create_instance_buffer(&state.renderer.gpu, &chrome_instances);
    let button_buffer = state
        .renderer.batch_renderer
        .create_instance_buffer(&state.renderer.gpu, &button_instances);
    let text_buffers: Vec<_> = text_draws
        .iter()
        .map(|draw| {
            state
                .renderer.batch_renderer
                .create_instance_buffer(&state.renderer.gpu, &draw.instances)
        })
        .collect();
    let cursor_instances: Vec<SpriteInstance> = shell_cursor_instance(state).into_iter().collect();
    let cursor_buffer = state
        .renderer.batch_renderer
        .create_instance_buffer(&state.renderer.gpu, &cursor_instances);
    // Default-cursor frame-0 texture, borrowed for the duration of the pass.
    let cursor_texture = state
        .software_cursor
        .as_ref()
        .and_then(|cursor| cursor.get(crate::app::types::CursorId::Default))
        .and_then(|sequence| sequence.frames.first())
        .map(|frame| &frame.texture);

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Single Player Shell"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target.color,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app::types::CLEAR_COLOR),
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
            .renderer.batch_renderer
            .draw_with_buffer_passthrough(&mut pass, movie_texture, buffer, *count);
    }
    if let Some((buffer, count)) = chrome_buffer.as_ref() {
        state.renderer.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &chrome.texture,
            buffer,
            *count,
        );
    }
    if let Some((buffer, count)) = button_buffer.as_ref() {
        state.renderer.batch_renderer.draw_with_buffer_passthrough(
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
        state.renderer.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            state.renderer.bit_font.atlas(),
            buffer,
            *count,
        );
    }
    pass.set_scissor_rect(0, 0, state.renderer.gpu.config.width, state.renderer.gpu.config.height);
    // Software cursor draws last, on top of all chrome/controls.
    if let (Some((buffer, count)), Some(texture)) = (cursor_buffer.as_ref(), cursor_texture) {
        state
            .renderer.batch_renderer
            .draw_with_buffer_passthrough(&mut pass, texture, buffer, *count);
    }
    drop(pass);
    state
        .renderer.shell_surface_presenter
        .encode_present(encoder, destination);

    Ok(SinglePlayerShellRenderResult::Rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_player_policy_uses_native_art_without_mouse_hover_flash() {
        assert!(matches!(SP_BUTTON_POLICY.art_fit, ArtFit::Native));
        assert!(!SP_BUTTON_POLICY.hover_flash);
        assert_eq!(SP_BUTTON_POLICY.art_sink_y, 0.0);
        assert!(SP_BUTTON_POLICY.disabled_dim);
    }

    #[test]
    fn owner_draw_label_clip_matches_native_up_and_pressed_rects() {
        let button = RectPx::new(644, 199, 156, 42);
        assert_eq!(
            owner_draw_button_label_rect(button, false),
            RectPx::new(644, 200, 154, 41)
        );
        assert_eq!(
            owner_draw_button_label_rect(button, true),
            RectPx::new(646, 204, 152, 37)
        );
    }

    #[test]
    fn status_help_is_immediate_and_includes_disabled_load() {
        assert_eq!(single_player_status_csf_key(None), None);
        assert_eq!(
            single_player_status_csf_key(Some(SinglePlayerControlId::LoadSavedGame0x689)),
            Some("STT:SingleButtonLoadSavedGame")
        );
        assert!(SP_STATUS_ALIGN.contains(ShellAlign::V_CENTER));
        assert!(!SP_STATUS_ALIGN.contains(ShellAlign::H_CENTER));
    }

    #[test]
    fn gsi_13_26_single_player_steady_frame_uses_rgb565_presenter_after_full_composition() {
        let source = include_str!("single_player_shell_render.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .expect("test module follows production renderer")
            .0;
        let renderer = &production[production
            .find("pub(crate) fn render_single_player_shell")
            .expect("production renderer")..];

        assert!(renderer.contains("destination: &wgpu::Texture"));
        assert!(!renderer.contains("target: &wgpu::TextureView"));
        let source_view = renderer
            .find("shell_surface_presenter.source_render_view()")
            .expect("RGB565 presenter source view");
        let fallback_returns: Vec<_> = renderer
            .match_indices("return Ok(SinglePlayerShellRenderResult::Fallback)")
            .map(|(index, _)| index)
            .collect();
        let render_pass = renderer
            .find("encoder.begin_render_pass")
            .expect("complete shell render pass");
        let cursor = renderer
            .find("Software cursor draws last")
            .expect("software cursor submission");
        let pass_end = renderer.find("drop(pass);").expect("render pass end");
        let present = renderer
            .find(".encode_present(encoder, destination);")
            .expect("RGB565 encode/present");

        assert_eq!(fallback_returns.len(), 2);
        assert!(fallback_returns.iter().all(|&index| index < source_view));
        assert!(source_view < render_pass);
        assert!(render_pass < cursor);
        assert!(cursor < pass_end);
        assert!(pass_end < present);

        let app_source = include_str!("../frame.rs");
        let dispatch = &app_source[app_source
            .find("else if Self::single_player_shell_active(state)")
            .expect("single-player steady dispatch")..];
        let shell_call = dispatch
            .find("render_single_player_shell")
            .expect("single-player renderer call");
        let overlay = dispatch
            .find("state.renderer.egui.end_frame_and_render")
            .expect("post-shell egui overlay");
        assert!(dispatch[shell_call..overlay].contains("&output.texture"));
        assert!(dispatch[overlay..].starts_with("state.renderer.egui.end_frame_and_render"));
        assert!(dispatch[overlay..].contains("&view"));
    }
}
