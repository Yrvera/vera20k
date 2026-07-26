//! Initial main-menu shell render glue for dialog 0xE2.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;

use crate::app::AppState;
use crate::app_shell_transition::{ButtonGroup, ShellFrameWave};
use crate::render::batch::SpriteInstance;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_paint::{
    self, ArtFit, ButtonPolicy, CURSOR_DEPTH, MOVIE_DEPTH, PARENT_BACKGROUND_DEPTH, PaintButton,
    PaintLabel, SHELL_TEXT_RGB_ENABLED,
};
use crate::render::shell_text::ShellAlign;
use crate::render::shell_text_reveal::PathAReveal;
use crate::render::shell_transition_pass::ShellRenderTarget;
use crate::ui::main_menu_shell::{
    MainMenuControlId, MainMenuMovieBase, MainMenuShellLayout, RectPx, compute_layout,
    csf_key_for_control, tooltip_csf_key_for_control,
};
use crate::ui::shell::static_reveal::{Kind1PaintWindow, Kind1RevealReceipt, Kind1RevealWindow};

/// Screen-size thresholds above which the centered 800x600 shell is letterboxed
/// (background and chrome offset by ((w-800)/2, (h-600)/2) instead of (0,0)).
const SHELL_LETTERBOX_W_THRESHOLD: i32 = 1023;
const SHELL_LETTERBOX_H_THRESHOLD: i32 = 767;
const SHELL_BASE_W: i32 = 800;
const SHELL_BASE_H: i32 = 600;

/// Dialog 0xE2 owner-draw button policy: native art remains at the cell top-left
/// while frame selection changes on press. The dialog has no hover flash or
/// disabled owner-draw button.
const MAIN_MENU_BUTTON_POLICY: ButtonPolicy = ButtonPolicy {
    art_fit: ArtFit::Native,
    hover_flash: false,
    art_sink_y: 0.0,
    disabled_dim: false,
};

/// Native static `0x695` is left-aligned and vertically centered.
const MAIN_MENU_STATUS_ALIGN: ShellAlign = ShellAlign::V_CENTER;

pub(crate) enum MainMenuShellRenderResult {
    Rendered {
        title_receipt: Option<Kind1RevealReceipt>,
    },
    Fallback,
}

/// Dialog whose child static `0x71A` owns the active RA2TS movie handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ra2tsDialogOwner {
    MainMenu0xE2,
    SinglePlayer0x100,
}

/// Identity of the one RA2TS session installed for a movie-bearing dialog.
///
/// Owner and asset base change atomically because neither alone is sufficient
/// to make the decoder/texture reusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ra2tsMovieSessionIdentity {
    owner: Ra2tsDialogOwner,
    base: MainMenuMovieBase,
}

impl Ra2tsMovieSessionIdentity {
    const fn new(owner: Ra2tsDialogOwner, base: MainMenuMovieBase) -> Self {
        Self { owner, base }
    }

    pub(crate) const fn owner(self) -> Ra2tsDialogOwner {
        self.owner
    }

    pub(crate) const fn base(self) -> MainMenuMovieBase {
        self.base
    }
}

fn ra2ts_movie_session_is_reusable(
    movie_loaded: bool,
    active_identity: Option<Ra2tsMovieSessionIdentity>,
    requested_identity: Ra2tsMovieSessionIdentity,
) -> bool {
    movie_loaded && active_identity == Some(requested_identity)
}

/// Drop the one active RA2TS decoder/texture and all of its cache identity.
///
/// This models destruction of the owning dialog's child static `0x71A`. The
/// next movie-bearing dialog reconstructs its own session lazily on first paint.
pub(crate) fn clear_ra2ts_movie_session(state: &mut AppState) {
    state.main_menu_movie = None;
    state.main_menu_movie_identity = None;
    state.main_menu_movie_last_step = Instant::now();
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
            let wave_frame = wave.map(|w| w.sdbtnanm_frame(slot as u32, ButtonGroup::A) as usize);
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

pub(crate) fn main_menu_title_text(state: &AppState) -> &str {
    resolve_csf(state, "GUI:MainMenu")
}

fn main_menu_status_csf_key(hovered_button: Option<MainMenuControlId>) -> Option<&'static str> {
    hovered_button.map(tooltip_csf_key_for_control)
}

fn main_menu_title_path_a(window: Kind1RevealWindow) -> PathAReveal {
    PathAReveal {
        count: window.count,
        range: window.range,
        base_rgb: [255, 255, 0],
        highlight_rgb: [255, 255, 255],
    }
}

/// Build the owner-draw button labels and dialog statics consumed by
/// `shell_paint::paint_labels`. Button labels use the exact native normal/pressed
/// clipping rectangles. Status static `0x695` reads the dialog's immediate hover
/// state rather than the delayed in-game tooltip service.
fn main_menu_paint_labels<'a>(
    state: &'a AppState,
    layout: &MainMenuShellLayout,
    pressed_button: Option<MainMenuControlId>,
    hovered_button: Option<MainMenuControlId>,
    version_text: &'a str,
    title_window: Option<Kind1RevealWindow>,
) -> Vec<PaintLabel<'a>> {
    use crate::render::shell_text::ShellAlign;
    let mut out = Vec::new();
    let button_align = ShellAlign::H_CENTER | ShellAlign::V_CENTER;
    for button in &layout.buttons {
        let pressed = pressed_button == Some(button.id);
        out.push(PaintLabel {
            text: resolve_csf(state, csf_key_for_control(button.id)),
            rect: owner_draw_button_label_rect(button.rect, pressed),
            align: button_align,
            rgb: SHELL_TEXT_RGB_ENABLED,
            path_a_reveal: None,
        });
    }
    // Statics: title heading, version, tooltip — top-anchored, h-centered.
    if let Some(window) = title_window {
        out.push(PaintLabel {
            text: main_menu_title_text(state),
            rect: layout.title,
            align: ShellAlign::H_CENTER,
            rgb: SHELL_TEXT_RGB_ENABLED,
            path_a_reveal: Some(main_menu_title_path_a(window)),
        });
    }
    out.push(PaintLabel {
        text: version_text,
        rect: layout.version_line,
        align: ShellAlign::H_CENTER,
        rgb: SHELL_TEXT_RGB_ENABLED,
        path_a_reveal: None,
    });
    if let Some(key) = main_menu_status_csf_key(hovered_button) {
        out.push(PaintLabel {
            text: resolve_csf(state, key),
            rect: layout.tooltip_line,
            align: MAIN_MENU_STATUS_ALIGN,
            rgb: SHELL_TEXT_RGB_ENABLED,
            path_a_reveal: None,
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

pub(crate) fn ensure_movie_for_current_layout(
    state: &mut AppState,
    requested_owner: Ra2tsDialogOwner,
) -> Result<()> {
    let layout = compute_layout(state.gpu.config.width, state.gpu.config.height);
    let requested_identity = Ra2tsMovieSessionIdentity::new(requested_owner, layout.movie_base);
    if ra2ts_movie_session_is_reusable(
        state.main_menu_movie.is_some(),
        state.main_menu_movie_identity,
        requested_identity,
    ) {
        return Ok(());
    }
    clear_ra2ts_movie_session(state);

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
    state.main_menu_movie_identity = Some(requested_identity);
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

/// Paint the normal 0xE2 route through its active-retail presentation boundary.
///
/// First-paint transition callers intentionally use
/// [`render_main_menu_shell_to_target`] so their shared transition target is
/// not post-processed as a steady native primary surface.
pub(crate) fn render_main_menu_shell(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    destination: &wgpu::Texture,
) -> Result<MainMenuShellRenderResult> {
    let (title_window, title_receipt) =
        match state.main_menu_shell_state.title_reveal.paint_window() {
            Kind1PaintWindow::Hidden => (None, None),
            Kind1PaintWindow::Retained(window) => (Some(window), None),
            Kind1PaintWindow::Due { window, receipt } => (Some(window), Some(receipt)),
        };
    let color = state.shell_surface_presenter.source_render_view();
    let depth = state.depth_view.clone();
    let result = render_main_menu_shell_to_target_inner(
        state,
        encoder,
        ShellRenderTarget {
            color: &color,
            depth: &depth,
        },
        title_window,
    )?;
    match result {
        MainMenuShellRenderResult::Rendered { .. } => {
            state
                .shell_surface_presenter
                .encode_present(encoder, destination);
            Ok(MainMenuShellRenderResult::Rendered { title_receipt })
        }
        MainMenuShellRenderResult::Fallback => Ok(MainMenuShellRenderResult::Fallback),
    }
}

pub(crate) fn render_main_menu_shell_to_target(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: ShellRenderTarget<'_>,
) -> Result<MainMenuShellRenderResult> {
    render_main_menu_shell_to_target_inner(state, encoder, target, None)
}

fn render_main_menu_shell_to_target_inner(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: ShellRenderTarget<'_>,
    title_window: Option<Kind1RevealWindow>,
) -> Result<MainMenuShellRenderResult> {
    ensure_movie_for_current_layout(state, Ra2tsDialogOwner::MainMenu0xE2)?;
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
    let chrome_instances = shell_paint::paint_chrome(
        chrome,
        layout.right_panel,
        Some(layout.lower_strip),
        layout.screen.w,
    );
    let buttons = main_menu_paint_buttons(
        &layout,
        state.main_menu_shell_state.pressed_owner_draw_button,
        wave.as_ref(),
    );
    // 0xE2 never flashes, so the hover clock is unused (None) — keep the call
    // shape uniform with 0x100, which threads its hover_started_at.
    let button_instances = shell_paint::paint_buttons(
        chrome,
        &buttons,
        MAIN_MENU_BUTTON_POLICY,
        Instant::now(),
        None,
    );
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
        title_window,
    );
    let text_draws = shell_paint::paint_labels(&state.bit_font, &labels);

    // Quit-confirm SHP modal overlay (blocking; drawn over the menu, under the
    // cursor). `None` when the modal is closed or the skirmish atlas (which holds
    // PUDLGBGN/MNBTTN) is not loaded.
    let modal_overlay = build_exit_confirm_modal_overlay(state);
    let skirmish_chrome = state.skirmish_shell_chrome.as_ref();

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
    let modal_sprite_buffer = modal_overlay.as_ref().and_then(|m| {
        state
            .batch_renderer
            .create_instance_buffer(&state.gpu, &m.sprites)
    });
    let modal_text_buffers: Vec<_> = modal_overlay
        .as_ref()
        .map(|m| {
            m.text
                .iter()
                .map(|draw| {
                    state
                        .batch_renderer
                        .create_instance_buffer(&state.gpu, &draw.instances)
                })
                .collect()
        })
        .unwrap_or_default();
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

    // Quit-cascade fade-to-black: a full-screen black quad over EVERYTHING (incl.
    // the cursor), alpha ramped 0→1 by the cascade. Reuses the 1×1 opaque
    // white_pixel + the ALPHA_BLENDING passthrough pipeline; no new shader. Built
    // here (like every other layer) so it outlives the render pass. Only present on
    // the SHP path (the atlas is unavailable on the egui fallback).
    let fade_alpha = state
        .quit_cascade
        .as_ref()
        .map_or(0.0, |cascade| cascade.overlay_alpha());
    let fade_buffer = if fade_alpha > 0.0 {
        skirmish_chrome
            .and_then(|sk| sk.white_pixel)
            .and_then(|white| {
                let quad = [crate::render::batch::SpriteInstance {
                    position: [0.0, 0.0],
                    size: [
                        state.gpu.config.width as f32,
                        state.gpu.config.height as f32,
                    ],
                    uv_origin: white.uv_origin,
                    uv_size: white.uv_size,
                    // Passthrough compares depth Always and this draws last, so any
                    // depth sits on top; the frontmost value is used for clarity.
                    depth: 0.0,
                    tint: [0.0, 0.0, 0.0],
                    alpha: fade_alpha,
                    ..Default::default()
                }];
                state
                    .batch_renderer
                    .create_instance_buffer(&state.gpu, &quad)
            })
    } else {
        None
    };

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
    // Quit-confirm modal overlay: SHP panel + buttons (skirmish atlas texture),
    // then labels (font atlas), above the menu but below the cursor.
    if let (Some(overlay), Some(sk_chrome)) = (modal_overlay.as_ref(), skirmish_chrome) {
        if let Some((buffer, count)) = modal_sprite_buffer.as_ref() {
            state.batch_renderer.draw_with_buffer_passthrough(
                &mut pass,
                &sk_chrome.texture,
                buffer,
                *count,
            );
        }
        for (draw, buffer) in overlay.text.iter().zip(modal_text_buffers.iter()) {
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
    }
    if let (Some((buffer, count)), Some(texture)) = (cursor_buffer.as_ref(), cursor_texture) {
        state
            .batch_renderer
            .draw_with_buffer_passthrough(&mut pass, texture, buffer, *count);
    }
    // Quit-cascade fade-to-black overlay, drawn LAST so it blackens everything
    // including the cursor (the original's palette fade affects the whole frame).
    if let (Some((buffer, count)), Some(sk_chrome)) = (fade_buffer.as_ref(), skirmish_chrome) {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &sk_chrome.texture,
            buffer,
            *count,
        );
    }
    drop(pass);

    Ok(MainMenuShellRenderResult::Rendered {
        title_receipt: None,
    })
}

/// Back-to-front depths for the quit-confirm modal overlay. They sit in the clear
/// band between the menu's front-most text (`TEXT_DEPTH`) and the cursor
/// (`CURSOR_DEPTH`), so the modal blocks the menu while the cursor stays on top.
const EXIT_CONFIRM_MODAL_DEPTHS: shell_paint::ModalDepths = shell_paint::ModalDepths {
    background: 0.00050,
    button: 0.00045,
    text: 0.00040,
};

/// Build the quit-confirm (0x120) SHP modal overlay when it is open: the centered
/// PUDLGBGN panel + MNBTTN OK/Cancel buttons + body/OK/Cancel labels, sourced from
/// the skirmish chrome atlas (the only atlas that loads PUDLGBGN/MNBTTN). The
/// pressed button is read from the shared shell controller (its top dialog is the
/// modal while open). Returns `None` when the modal is closed or its art is absent.
fn build_exit_confirm_modal_overlay(state: &AppState) -> Option<shell_paint::ModalDraw> {
    use crate::ui::shell::modal;
    let modal_state = state.exit_confirm_modal.as_ref()?;
    let atlas = state.skirmish_shell_chrome.as_ref()?;
    let layout = modal::quit_confirm_layout(
        state.gpu.config.width as i32,
        state.gpu.config.height as i32,
    );
    let pressed = state.shell_controller.pressed();
    let ok_pressed = pressed == Some(modal::control::OK);
    let cancel_pressed = pressed == Some(modal::control::CANCEL);
    let frames = shell_paint::ModalButtonFrames {
        up: atlas.modal_button_mnbttn_frame0,
        disabled: atlas.modal_button_mnbttn_frame1,
        pressed: atlas.modal_button_mnbttn_frame2,
    };
    let buttons = [
        shell_paint::ModalButton {
            rect: layout.ok,
            pressed: ok_pressed,
            enabled: true,
        },
        shell_paint::ModalButton {
            rect: layout.cancel,
            pressed: cancel_pressed,
            enabled: true,
        },
    ];
    // Body left-top wrapped; OK/Cancel centered on their buttons with the MNBTTN
    // press sink — same conventions as the skirmish validation modal.
    let labels = [
        PaintLabel {
            text: &modal_state.title,
            rect: layout.body,
            align: ShellAlign::NONE,
            rgb: SHELL_TEXT_RGB_ENABLED,
            path_a_reveal: None,
        },
        PaintLabel {
            text: &modal_state.confirm,
            rect: owner_draw_button_label_rect(layout.ok, ok_pressed),
            align: ShellAlign::H_CENTER | ShellAlign::V_CENTER,
            rgb: SHELL_TEXT_RGB_ENABLED,
            path_a_reveal: None,
        },
        PaintLabel {
            text: &modal_state.cancel,
            rect: owner_draw_button_label_rect(layout.cancel, cancel_pressed),
            align: ShellAlign::H_CENTER | ShellAlign::V_CENTER,
            rgb: SHELL_TEXT_RGB_ENABLED,
            path_a_reveal: None,
        },
    ];
    Some(shell_paint::paint_modal_shp(
        &state.bit_font,
        atlas.validation_modal_background_pudlgbgn,
        frames,
        layout.dialog,
        &buttons,
        &labels,
        EXIT_CONFIRM_MODAL_DEPTHS,
    ))
}

/// Exact owner-draw button label clipping rectangle: unpressed
/// `+0x/+1y/-2w/-1h`, pressed `+2x/+5y/-4w/-5h`.
fn owner_draw_button_label_rect(rect: RectPx, pressed: bool) -> RectPx {
    let (dx, dy) = if pressed { (2, 5) } else { (0, 1) };
    RectPx::new(
        rect.x + dx,
        rect.y + dy,
        (rect.w - 2 - dx).max(0),
        (rect.h - dy).max(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::main_menu_shell::compute_layout;

    #[test]
    fn owner_draw_label_rect_matches_native_boundaries() {
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
    fn status_key_follows_immediate_hover_state() {
        assert_eq!(main_menu_status_csf_key(None), None);
        assert_eq!(
            main_menu_status_csf_key(Some(MainMenuControlId::SinglePlayer0x683)),
            Some("STT:MainButtonSinglePlayer")
        );
        assert_eq!(
            main_menu_status_csf_key(Some(MainMenuControlId::ExitGame0x3ee)),
            Some("STT:MainButtonExitGamemd")
        );
    }

    #[test]
    fn status_static_is_left_aligned_and_vertically_centered() {
        assert_eq!(MAIN_MENU_STATUS_ALIGN, ShellAlign::V_CENTER);
        assert!(!MAIN_MENU_STATUS_ALIGN.contains(ShellAlign::H_CENTER));
    }

    #[test]
    fn terminal_title_metadata_is_content_agnostic_path_a() {
        let reveal = main_menu_title_path_a(Kind1RevealWindow {
            count: 17,
            range: 8,
        });
        assert_eq!(
            reveal,
            PathAReveal {
                count: 17,
                range: 8,
                base_rgb: [255, 255, 0],
                highlight_rgb: [255, 255, 255],
            }
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
    fn parent_background_renders_behind_movie() {
        // Background depth must be greater (farther back) than the movie's.
        assert!(PARENT_BACKGROUND_DEPTH > MOVIE_DEPTH);
    }

    #[test]
    fn changing_dialog_owner_restarts_ra2ts_even_when_asset_base_matches() {
        assert!(!ra2ts_movie_session_is_reusable(
            true,
            Some(Ra2tsMovieSessionIdentity::new(
                Ra2tsDialogOwner::MainMenu0xE2,
                MainMenuMovieBase::Ra2tsL,
            )),
            Ra2tsMovieSessionIdentity::new(
                Ra2tsDialogOwner::SinglePlayer0x100,
                MainMenuMovieBase::Ra2tsL,
            ),
        ));
        assert!(!ra2ts_movie_session_is_reusable(
            true,
            Some(Ra2tsMovieSessionIdentity::new(
                Ra2tsDialogOwner::SinglePlayer0x100,
                MainMenuMovieBase::Ra2tsL,
            )),
            Ra2tsMovieSessionIdentity::new(
                Ra2tsDialogOwner::MainMenu0xE2,
                MainMenuMovieBase::Ra2tsL,
            ),
        ));
    }

    #[test]
    fn matching_owner_and_base_reuses_only_a_loaded_ra2ts_session() {
        let identity = Ra2tsMovieSessionIdentity::new(
            Ra2tsDialogOwner::MainMenu0xE2,
            MainMenuMovieBase::Ra2tsL,
        );
        assert!(ra2ts_movie_session_is_reusable(
            true,
            Some(identity),
            identity,
        ));
        assert!(!ra2ts_movie_session_is_reusable(
            false,
            Some(identity),
            identity,
        ));
        assert!(!ra2ts_movie_session_is_reusable(
            true,
            Some(Ra2tsMovieSessionIdentity::new(
                Ra2tsDialogOwner::MainMenu0xE2,
                MainMenuMovieBase::Ra2tsS,
            )),
            identity,
        ));
    }

    #[test]
    fn cleared_identity_blocks_collapsed_dialog_round_trip_reuse() {
        for owner in [
            Ra2tsDialogOwner::MainMenu0xE2,
            Ra2tsDialogOwner::SinglePlayer0x100,
        ] {
            let identity = Ra2tsMovieSessionIdentity::new(owner, MainMenuMovieBase::Ra2tsL);
            let mut active_identity = Some(identity);
            assert!(ra2ts_movie_session_is_reusable(
                true,
                active_identity,
                identity,
            ));

            // The source dialog is destroyed and the destination never paints.
            // Keep movie_loaded true deliberately to model a stale GPU surface.
            active_identity = None;
            assert!(!ra2ts_movie_session_is_reusable(
                true,
                active_identity,
                identity,
            ));

            active_identity = Some(identity);
            assert!(ra2ts_movie_session_is_reusable(
                true,
                active_identity,
                identity,
            ));
        }
    }
}
