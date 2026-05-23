//! Initial main-menu shell render glue for dialog 0xE2.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::render::batch::SpriteInstance;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_text::ShellTextDraw;
use crate::ui::main_menu_shell::{
    MainMenuControlId, MainMenuShellLayout, RIGHT_PANEL_TILE_H, RIGHT_PANEL_WIDTH, RectPx,
    compute_layout, csf_key_for_control, tooltip_csf_key_for_control,
};

const MOVIE_DEPTH: f32 = 0.00095;
const CHROME_DEPTH: f32 = 0.00085;
const BUTTON_DEPTH: f32 = 0.00080;
const TEXT_DEPTH: f32 = 0.00070;
const SHELL_BUTTON_TEXT_RGB_FFFF00: [f32; 3] = [1.0, 1.0, 0.0];

pub(crate) enum MainMenuShellRenderResult {
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
    depth: f32,
) {
    let frame = button_frame(atlas, pressed, hover_highlight);
    // The 156x42 SHP frame sits inside the 168x42 chrome tile right-anchored
    // against the right edge, leaving the 12 px bevel on the left only.
    let scale_x = rect.w as f32 / RIGHT_PANEL_WIDTH as f32;
    let scale_y = rect.h as f32 / RIGHT_PANEL_TILE_H as f32;
    let frame_w = frame.pixel_size[0] * scale_x;
    let frame_h = frame.pixel_size[1] * scale_y;
    let x = rect.x as f32 + (rect.w as f32 - frame_w);
    let y = rect.y as f32 + (rect.h as f32 - frame_h) * 0.5;
    push_entry_sized(out, frame, x, y, [frame_w, frame_h], depth);
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
        SHELL_BUTTON_TEXT_RGB_FFFF00,
        align,
        [0.0, 0.0],
        TEXT_DEPTH,
    ));
}

fn build_button_instances(
    atlas: &MainMenuShellChromeAtlas,
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
    hovered_button: Option<MainMenuControlId>,
    hover_started_at: Option<Instant>,
    now: Instant,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    for button in &layout.buttons {
        let pressed = pressed_button == Some(button.id);
        // gamemd arms a 1000 ms WM_TIMER on hover that toggles the +0xC5
        // flash byte; WM_PAINT picks frame 3 (highlight) when the byte is
        // non-zero, else frame 2 (default). Mirror that locally by deriving
        // the toggle phase from elapsed wall-clock time since hover began.
        let hover_highlight = if !pressed && hovered_button == Some(button.id) {
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
            BUTTON_DEPTH,
        );
    }
    out
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

/// Draw the top `rect.h` rows of `entry` 1:1, cropping the SHP rather than
/// stretching the full image to fit. Used for SDBTM where the SHP is 168x65
/// native but the destination cap region is 23 px tall — gamemd clips, we
/// must too.
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
    layout: &MainMenuShellLayout,
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

fn build_text_draws(
    state: &AppState,
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
    hovered_button: Option<MainMenuControlId>,
) -> Vec<ShellTextDraw> {
    use crate::render::shell_text::ShellAlign;
    let mut out = Vec::new();
    // Owner-draw buttons render h-centered + v-centered within a rect inset
    // by top+=1 and right-=2 from the full button rect, matching gamemd's
    // text-rect construction (`SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT §2`).
    // When pressed the rect's left shifts by +x_offset, producing the net
    // +1 px right text shift gamemd shows on click.
    let button_align = ShellAlign::H_CENTER | ShellAlign::V_CENTER;
    for button in &layout.buttons {
        let text = resolve_csf(state, csf_key_for_control(button.id));
        let x_offset = if pressed_button == Some(button.id) {
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
        push_label(&mut out, state, text, text_rect, button_align);
    }
    // Static text labels (title heading, version, tooltip) render top-anchored
    // h-centered within their rect — they are not vertically centered.
    let title = resolve_csf(state, "GUI:MainMenu");
    push_label(&mut out, state, title, layout.title, ShellAlign::H_CENTER);

    let version_label = resolve_csf(state, "GUI:Version");
    let version_text = format!("{} {}", version_label, state.version_txt);
    push_label(
        &mut out,
        state,
        &version_text,
        layout.version_line,
        ShellAlign::H_CENTER,
    );

    if let Some(id) = hovered_button {
        let tip_key = tooltip_csf_key_for_control(id);
        let tip_text = resolve_csf(state, tip_key);
        push_label(
            &mut out,
            state,
            tip_text,
            layout.tooltip_line,
            ShellAlign::H_CENTER,
        );
    }

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
    let layout = compute_layout(state.gpu.config.width, state.gpu.config.height);
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

    let layout = compute_layout(state.gpu.config.width, state.gpu.config.height);
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
    let chrome_instances = build_chrome_instances(chrome, &layout);
    let button_instances = build_button_instances(
        chrome,
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
        state.main_menu_shell_state.hovered_owner_draw_button,
        state.main_menu_shell_state.hover_started_at,
        Instant::now(),
    );
    let text_draws = build_text_draws(
        state,
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
        state.main_menu_shell_state.hovered_owner_draw_button,
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
    drop(pass);

    Ok(MainMenuShellRenderResult::Rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::main_menu_shell::compute_layout;

    #[test]
    fn movie_instance_uses_layout_movie_rect() {
        let layout = compute_layout(800, 600);
        let instance = movie_instance(&layout);
        assert_eq!(instance.position, [0.0, 0.0]);
        assert_eq!(instance.size, [632.0, 570.0]);
    }
}
