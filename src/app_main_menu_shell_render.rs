//! Initial main-menu shell render glue for dialog 0xE2.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::app_shell_transition::{ButtonGroup, ShellFrameWave};
use crate::render::batch::SpriteInstance;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_text::ShellTextDraw;
use crate::render::shell_transition_pass::ShellRenderTarget;
use crate::ui::main_menu_shell::{
    MainMenuControlId, MainMenuShellLayout, RectPx, compute_layout, csf_key_for_control,
    tooltip_csf_key_for_control,
};

/// Parent background sits behind the movie in the Z stack. Greater depth =
/// farther back, so this must exceed MOVIE_DEPTH.
const PARENT_BACKGROUND_DEPTH: f32 = 0.00098;
const MOVIE_DEPTH: f32 = 0.00095;
const CHROME_DEPTH: f32 = 0.00085;
/// Screen-size thresholds above which the centered 800x600 shell is letterboxed
/// (background and chrome offset by ((w-800)/2, (h-600)/2) instead of (0,0)).
const SHELL_LETTERBOX_W_THRESHOLD: i32 = 1023;
const SHELL_LETTERBOX_H_THRESHOLD: i32 = 767;
const SHELL_BASE_W: i32 = 800;
const SHELL_BASE_H: i32 = 600;
const BUTTON_DEPTH: f32 = 0.00080;
const TEXT_DEPTH: f32 = 0.00070;
/// The software cursor draws on top of everything else on the menu (smallest
/// depth). The original hides the OS cursor and blits the cursor SHP last.
const CURSOR_DEPTH: f32 = 0.00001;
const SHELL_BUTTON_TEXT_RGB_FFFF00: [f32; 3] = [1.0, 1.0, 0.0];
/// On press, gamemd's owner-draw button sinks the whole button content down by
/// +2 px in Y (in addition to the +1 px right shift from
/// `pressed_content_offset_x`). Both the button art and its label move together.
/// Y+ is downward in this screen-space render path.
const PRESSED_CONTENT_OFFSET_Y: f32 = 2.0;

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

fn button_frame(atlas: &MainMenuShellChromeAtlas, pressed: bool) -> MainMenuShellChromeEntry {
    // Dialog 0xE2 buttons only ever show frame 2 (default) or frame 4
    // (pressed). The frame-3 focus-flash is never reached on this dialog, so
    // `button_hover` stays loaded in the atlas but is not selected here.
    if pressed {
        atlas.button_pressed
    } else {
        atlas.button_default
    }
}

/// Vertical content sink applied to a button (art + label) while pressed.
/// Returns +2 px (downward) when pressed, 0 otherwise.
fn pressed_content_offset_y(pressed: bool) -> f32 {
    if pressed {
        PRESSED_CONTENT_OFFSET_Y
    } else {
        0.0
    }
}

fn push_button_shp(
    out: &mut Vec<SpriteInstance>,
    atlas: &MainMenuShellChromeAtlas,
    rect: RectPx,
    pressed: bool,
    depth: f32,
) {
    let frame = button_frame(atlas, pressed);
    // The SDBTNANM frame is drawn at its NATIVE pixel size at the button
    // client rect's top-left — no stretch-to-tile and no centering/right
    // anchor. The frame's own internal x/y offset (baked into the rendered
    // atlas entry) handles the small inset of the 156-wide art inside the
    // 162-wide client rect. Pressed art sinks +2 px in Y only.
    let x = rect.x as f32;
    let y = rect.y as f32 + pressed_content_offset_y(pressed);
    push_entry_sized(out, frame, x, y, frame.pixel_size, depth);
}

/// Draw a main-menu button at a first-paint slide frame: native size at the
/// button rect top-left, same geometry as the steady frame. Clamps down one
/// frame if the exact index is missing, and holds (draws nothing) if neither is
/// baked — never panics on a short SHP.
fn push_button_wave_frame(
    out: &mut Vec<SpriteInstance>,
    atlas: &MainMenuShellChromeAtlas,
    rect: RectPx,
    frame: usize,
    depth: f32,
) {
    let wave_frame = |idx: usize| atlas.button_wave_frames.get(idx).copied().flatten();
    if let Some(entry) = wave_frame(frame).or_else(|| wave_frame(frame.saturating_sub(1))) {
        push_entry_sized(
            out,
            entry,
            rect.x as f32,
            rect.y as f32,
            entry.pixel_size,
            depth,
        );
    }
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
        None,
    ));
}

fn build_button_instances(
    atlas: &MainMenuShellChromeAtlas,
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
    wave: Option<&ShellFrameWave>,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    for (slot, button) in layout.buttons.iter().enumerate() {
        match wave {
            // First-paint slide: every button is an enabled main button => Group A.
            Some(wave) => {
                let frame = wave.sdbtnanm_frame(slot as u32, ButtonGroup::A);
                push_button_wave_frame(&mut out, atlas, button.rect, frame, BUTTON_DEPTH);
            }
            None => {
                let pressed = pressed_button == Some(button.id);
                push_button_shp(&mut out, atlas, button.rect, pressed, BUTTON_DEPTH);
            }
        }
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
        let pressed = pressed_button == Some(button.id);
        let x_offset = if pressed {
            layout.pressed_content_offset_x
        } else {
            0
        };
        // gamemd sinks the whole button content (art + label) down +2 px on
        // press; mirror the SHP shift here on the label's text rect.
        let y_offset = pressed_content_offset_y(pressed) as i32;
        let text_rect = RectPx::new(
            button.rect.x + x_offset,
            button.rect.y + 1 + y_offset,
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

/// Top-left origin of the centered 800x600 shell within the swapchain.
///
/// (0,0) at screen sizes up to the letterbox thresholds; otherwise the shell
/// is centered, offsetting by ((w-800)/2, (h-600)/2). The parent background is
/// painted at this origin at its native SHP canvas size.
fn shell_origin(layout: &MainMenuShellLayout) -> (i32, i32) {
    let x = if layout.screen.w > SHELL_LETTERBOX_W_THRESHOLD {
        (layout.screen.w - SHELL_BASE_W) / 2
    } else {
        0
    };
    let y = if layout.screen.h > SHELL_LETTERBOX_H_THRESHOLD {
        (layout.screen.h - SHELL_BASE_H) / 2
    } else {
        0
    };
    (x, y)
}

/// Select the parent-background SHP: MNSCRNS only at exactly 640 wide, else
/// MNSCRNL (mirrors gamemd's `g_ScreenWidth == 640` switch).
fn select_parent_background(
    screen_w: i32,
    mnscrns_640: Option<MainMenuShellChromeEntry>,
    mnscrnl_large: Option<MainMenuShellChromeEntry>,
) -> Option<MainMenuShellChromeEntry> {
    if screen_w == 640 {
        mnscrns_640
    } else {
        mnscrnl_large
    }
}

fn parent_background_entry(
    atlas: &MainMenuShellChromeAtlas,
    layout: &MainMenuShellLayout,
) -> Option<MainMenuShellChromeEntry> {
    select_parent_background(
        layout.screen.w,
        atlas.parent_background_640_mnscrns,
        atlas.parent_background_large_mnscrnl,
    )
}

/// Build the parent-background instance drawn behind the movie and chrome.
/// Drawn at native SHP canvas size at the centered shell origin.
fn build_parent_background_instances(
    atlas: &MainMenuShellChromeAtlas,
    layout: &MainMenuShellLayout,
) -> Vec<SpriteInstance> {
    let Some(entry) = parent_background_entry(atlas, layout) else {
        return Vec::new();
    };
    let (x, y) = shell_origin(layout);
    let mut out = Vec::new();
    push_entry_sized(
        &mut out,
        entry,
        x as f32,
        y as f32,
        entry.pixel_size,
        PARENT_BACKGROUND_DEPTH,
    );
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

/// Build the menu software-cursor sprite instance in screen space.
///
/// The menu always shows the default arrow (no hover/feedback variants), frame
/// 0, hotspot (0,0). Returns None when no software cursor is loaded. The menu
/// render uses a camera offset of (0,0), so the cursor sits at the raw screen
/// pointer position minus the hotspot.
fn menu_cursor_instance(state: &AppState) -> Option<SpriteInstance> {
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

pub(crate) fn render_main_menu_shell(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> Result<MainMenuShellRenderResult> {
    let depth = state.depth_view.clone();
    render_main_menu_shell_to_target(
        state,
        encoder,
        ShellRenderTarget {
            color: target,
            depth: &depth,
        },
    )
}

pub(crate) fn render_main_menu_shell_to_target(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: ShellRenderTarget<'_>,
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

    let background_instances = build_parent_background_instances(chrome, &layout);
    let movie_instances = build_movie_instances(&layout);
    let chrome_instances = build_chrome_instances(chrome, &layout);
    let button_instances = build_button_instances(
        chrome,
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
        wave.as_ref(),
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
    let background_buffer = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &background_instances);
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
    let cursor_instances: Vec<SpriteInstance> = menu_cursor_instance(state).into_iter().collect();
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
        label: Some("Main Menu Shell"),
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
    if let Some((buffer, count)) = background_buffer.as_ref() {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &chrome.texture,
            buffer,
            *count,
        );
    }
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
    if let (Some((buffer, count)), Some(texture)) = (cursor_buffer.as_ref(), cursor_texture) {
        state
            .batch_renderer
            .draw_with_buffer_passthrough(&mut pass, texture, buffer, *count);
    }
    drop(pass);

    Ok(MainMenuShellRenderResult::Rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::main_menu_shell::compute_layout;

    #[test]
    fn pressed_button_sinks_content_two_px_down() {
        // Unpressed adds no vertical offset; pressed sinks content +2 px (down).
        assert_eq!(pressed_content_offset_y(false), 0.0);
        assert_eq!(pressed_content_offset_y(true), 2.0);
        assert_eq!(
            pressed_content_offset_y(true) - pressed_content_offset_y(false),
            PRESSED_CONTENT_OFFSET_Y
        );
    }

    #[test]
    fn movie_instance_uses_layout_movie_rect() {
        let layout = compute_layout(800, 600);
        let instance = movie_instance(&layout);
        assert_eq!(instance.position, [0.0, 0.0]);
        assert_eq!(instance.size, [632.0, 570.0]);
    }

    fn fake_entry(w: f32, h: f32) -> MainMenuShellChromeEntry {
        MainMenuShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [w, h],
        }
    }

    #[test]
    fn parent_background_selects_mnscrns_only_at_width_640() {
        let mnscrns = fake_entry(472.0, 450.0);
        let mnscrnl = fake_entry(632.0, 570.0);
        // Exactly 640 wide -> MNSCRNS.
        assert_eq!(
            select_parent_background(640, Some(mnscrns), Some(mnscrnl)),
            Some(mnscrns)
        );
        // Any other width -> MNSCRNL.
        for w in [800, 1024, 1600] {
            assert_eq!(
                select_parent_background(w, Some(mnscrns), Some(mnscrnl)),
                Some(mnscrnl)
            );
        }
    }

    #[test]
    fn shell_origin_letterboxes_only_above_thresholds() {
        assert_eq!(shell_origin(&compute_layout(800, 600)), (0, 0));
        assert_eq!(shell_origin(&compute_layout(1024, 768)), (112, 84));
    }

    #[test]
    fn button_shp_draws_native_size_at_rect_top_left() {
        let frame = fake_entry(156.0, 42.0);
        let mut out = Vec::new();
        let rect = RectPx::new(635, 203, 162, 37);
        // Mirror push_button_shp's geometry directly (avoids needing an atlas):
        // native pixel size, top-left position, +2 px Y sink when pressed.
        let x = rect.x as f32;
        let y_unpressed = rect.y as f32 + pressed_content_offset_y(false);
        push_entry_sized(
            &mut out,
            frame,
            x,
            y_unpressed,
            frame.pixel_size,
            BUTTON_DEPTH,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].size, [156.0, 42.0]);
        assert_eq!(out[0].position, [635.0, 203.0]);

        out.clear();
        let y_pressed = rect.y as f32 + pressed_content_offset_y(true);
        push_entry_sized(
            &mut out,
            frame,
            x,
            y_pressed,
            frame.pixel_size,
            BUTTON_DEPTH,
        );
        // Pressed: native size, same X, +2 px Y, no horizontal shift.
        assert_eq!(out[0].size, [156.0, 42.0]);
        assert_eq!(out[0].position, [635.0, 205.0]);
    }

    #[test]
    fn parent_background_renders_behind_movie() {
        // Background depth must be greater (farther back) than the movie's.
        assert!(PARENT_BACKGROUND_DEPTH > MOVIE_DEPTH);
    }
}
