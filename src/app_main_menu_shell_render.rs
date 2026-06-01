//! Initial main-menu shell render glue for dialog 0xE2.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::app_shell_transition::{ButtonGroup, ShellFrameWave};
use crate::render::batch::SpriteInstance;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_paint::{
    self, ArtFit, ButtonPolicy, PaintButton, PaintLabel, CURSOR_DEPTH, MOVIE_DEPTH,
    PARENT_BACKGROUND_DEPTH, PRESSED_CONTENT_OFFSET_Y, SHELL_TEXT_RGB_ENABLED,
};
use crate::render::shell_transition_pass::ShellRenderTarget;
use crate::ui::main_menu_shell::{
    MainMenuControlId, MainMenuShellLayout, RectPx, compute_layout, csf_key_for_control,
    tooltip_csf_key_for_control,
};

/// Screen-size thresholds above which the centered 800x600 shell is letterboxed
/// (background and chrome offset by ((w-800)/2, (h-600)/2) instead of (0,0)).
const SHELL_LETTERBOX_W_THRESHOLD: i32 = 1023;
const SHELL_LETTERBOX_H_THRESHOLD: i32 = 767;
const SHELL_BASE_W: i32 = 800;
const SHELL_BASE_H: i32 = 600;

/// Dialog 0xE2 owner-draw button policy: native art at the cell top-left, +2 px
/// Y sink on press, no hover flash, no disabled dim (0xE2 has no disabled
/// control). The +1 px text X shift on press is applied in the label builder.
const MAIN_MENU_BUTTON_POLICY: ButtonPolicy = ButtonPolicy {
    art_fit: ArtFit::Native,
    hover_flash: false,
    art_sink_y: PRESSED_CONTENT_OFFSET_Y,
    disabled_dim: false,
};

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

/// Map the layout + shell state into the owner-draw button list for the paint
/// pass. 0xE2 never disables a control, so every button is `enabled: true`;
/// during a first-paint slide each button rides Group A's ramp.
fn main_menu_paint_buttons(
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
    wave: Option<&ShellFrameWave>,
) -> Vec<PaintButton> {
    layout
        .buttons
        .iter()
        .enumerate()
        .map(|(slot, button)| {
            let wave_frame =
                wave.map(|w| w.sdbtnanm_frame(slot as u32, ButtonGroup::A) as usize);
            PaintButton {
                rect: button.rect,
                pressed: pressed_button == Some(button.id),
                hovered: false, // 0xE2 never flashes; hover state is unused on art
                enabled: true,
                wave_frame,
            }
        })
        .collect()
}

fn resolve_csf<'a>(state: &'a AppState, key: &'static str) -> &'a str {
    state
        .csf
        .as_ref()
        .and_then(|csf| csf.get(key))
        .unwrap_or(key)
}

/// Build the owner-draw button labels + statics (title / version / tooltip) as
/// `PaintLabel`s consumed by `shell_paint::paint_labels`. Reproduces the prior
/// `build_text_draws` exactly: button labels h+v-centered in a rect inset by
/// top+=1 / right-=2, shifted +x_offset / +2y on press; statics h-centered
/// top-anchored. 0xE2 text is always #FFFF00 (no disabled control). The `version`
/// string is owned, so the returned labels borrow from `strings`.
fn main_menu_paint_labels<'a>(
    state: &'a AppState,
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
    hovered_button: Option<MainMenuControlId>,
    version_text: &'a str,
) -> Vec<PaintLabel<'a>> {
    use crate::render::shell_text::ShellAlign;
    let mut out = Vec::new();
    let button_align = ShellAlign::H_CENTER | ShellAlign::V_CENTER;
    for button in &layout.buttons {
        let pressed = pressed_button == Some(button.id);
        let x_offset = if pressed {
            layout.pressed_content_offset_x
        } else {
            0
        };
        // gamemd sinks the whole button content (art + label) down +2 px on
        // press. The text Y sink is applied as i32, distinct from the f32 art
        // sink threaded through ButtonPolicy.
        let y_offset = if pressed {
            PRESSED_CONTENT_OFFSET_Y as i32
        } else {
            0
        };
        out.push(PaintLabel {
            text: resolve_csf(state, csf_key_for_control(button.id)),
            rect: RectPx::new(
                button.rect.x + x_offset,
                button.rect.y + 1 + y_offset,
                (button.rect.w - 2).max(0),
                (button.rect.h - 1).max(0),
            ),
            align: button_align,
            rgb: SHELL_TEXT_RGB_ENABLED,
        });
    }
    // Statics: title heading, version, tooltip — top-anchored, h-centered.
    out.push(PaintLabel {
        text: resolve_csf(state, "GUI:MainMenu"),
        rect: layout.title,
        align: ShellAlign::H_CENTER,
        rgb: SHELL_TEXT_RGB_ENABLED,
    });
    out.push(PaintLabel {
        text: version_text,
        rect: layout.version_line,
        align: ShellAlign::H_CENTER,
        rgb: SHELL_TEXT_RGB_ENABLED,
    });
    if let Some(id) = hovered_button {
        out.push(PaintLabel {
            text: resolve_csf(state, tooltip_csf_key_for_control(id)),
            rect: layout.tooltip_line,
            align: ShellAlign::H_CENTER,
            rgb: SHELL_TEXT_RGB_ENABLED,
        });
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

    // 0xE2-only MNSCRN parent background, submitted FIRST (no analog on 0x100).
    let background_instances = build_parent_background_instances(chrome, &layout);
    let movie_instances = build_movie_instances(&layout);
    let chrome_instances =
        shell_paint::paint_chrome(chrome, layout.right_panel, Some(layout.lower_strip), layout.screen.w);
    let buttons = main_menu_paint_buttons(
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
        wave.as_ref(),
    );
    // 0xE2 never flashes, so the hover clock is unused (None) — keep the call
    // shape uniform with 0x100, which threads its hover_started_at.
    let button_instances =
        shell_paint::paint_buttons(chrome, &buttons, MAIN_MENU_BUTTON_POLICY, Instant::now(), None);
    let version_text = format!(
        "{} {}",
        resolve_csf(state, "GUI:Version"),
        state.version_txt
    );
    let labels = main_menu_paint_labels(
        state,
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
        state.main_menu_shell_state.hovered_owner_draw_button,
        &version_text,
    );
    let text_draws = shell_paint::paint_labels(&state.bit_font, &labels);

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

    // The pressed-sink and native-art geometry tests moved to
    // `render::shell_paint` along with the geometry itself (Slice 3).

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
    fn parent_background_renders_behind_movie() {
        // Background depth must be greater (farther back) than the movie's.
        assert!(PARENT_BACKGROUND_DEPTH > MOVIE_DEPTH);
    }
}
