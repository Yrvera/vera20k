//! Retail process-start splash composition and presentation.
//!
//! This is intentionally separate from `app_loading`: the native GLS splash is
//! process initialization chrome, not selected-scenario loading presentation.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::assets::asset_manager::AssetManager;
use crate::assets::csf_file::CsfFile;
use crate::assets::fnt_file::FntFile;
use crate::assets::pal_file::Palette;
use crate::assets::shp_file::ShpFile;
use crate::render::batch::{BatchRenderer, BatchTexture, SpriteInstance};
use crate::render::gpu::GpuContext;
use crate::render::shell_surface_present::ShellSurfacePresenter;

// Startup splash art. The `MD` suffix marks the Yuri's Revenge variants
// (`GLSSMD`/`GLSLMD`/`GLSMD`, in `langmd.mix`); these unsuffixed names are the
// Red Alert 2 originals, in `ra2.mix -> local.mix`. Both sets exist in a retail
// install and both are 640x480 / 800x600 single-frame SHPs with a matching
// palette, so this is a straight substitution.
//
// Deliberate presentation choice, not a parity claim -- the project otherwise
// takes YR over RA2. Restoring the Yuri splash is re-adding the `MD` suffixes.
//
// Note the strings drawn over the splash still come from the YR string table,
// so the trademark line names Yuri's Revenge. Changing that means sourcing the
// whole UI string table from RA2, which is a much larger change than swapping
// three asset names.
const SMALL_SPLASH_SHP: &str = "GLSS.SHP";
const LARGE_SPLASH_SHP: &str = "GLSL.SHP";
const SPLASH_PALETTE: &str = "GLS.PAL";
const MINIMUM_VISIBLE_TIME: Duration = Duration::from_millis(5000);
const TEXT_COLOR: [u8; 4] = [255, 255, 255, 255];

const COPYRIGHT_KEY: &str = "TXT_COPYRIGHT";
const COPYRIGHT_FALLBACK: &str = "© 2000, 2001 ELECTRONIC ARTS INC. ALL RIGHTS RESERVED";
const BRAND_KEY: &str = "GUI:WWBrand";
const BRAND_FALLBACK: &str = "WESTWOOD STUDIOS\u{99} IS AN ELECTRONIC ARTS\u{99} BRAND";
const LOADING_KEY: &str = "GUI:LoadingEx";
const LOADING_FALLBACK: &str = "Loading...";
const TRADEMARK_TOP_KEY: &str = "GUI:TradeMarkTop";
const TRADEMARK_TOP_FALLBACK: &str =
    "Command & Conquer and Yuri's Revenge are trademarks or registered";
const TRADEMARK_BOTTOM_KEY: &str = "GUI:TradeMarkBottom";
const TRADEMARK_BOTTOM_FALLBACK: &str =
    "trademarks of Electronic Arts Inc. in the U.S. and/or other countries.";

/// The minimum on-screen hold, anchored at the first successful present.
///
/// Native anchors its timestamp *after* the blit and lets the rules/type
/// initialization that follows run inside that window, so the hold is
/// `max(work-after-present, MINIMUM_VISIBLE_TIME)` and never an added sleep.
/// Arming exactly once is what makes that true: the per-frame re-present would
/// otherwise push the deadline forward on every frame and hold the splash up
/// forever.
#[derive(Debug, Default)]
struct VisibleHold {
    deadline: Option<Instant>,
}

impl VisibleHold {
    fn mark_presented(&mut self, now: Instant) {
        if self.deadline.is_none() {
            self.deadline = Some(now + MINIMUM_VISIBLE_TIME);
        }
    }

    /// An unarmed hold is still active, so a first present that failed on a
    /// transient surface acquisition is retried instead of skipped.
    fn is_active(&self, now: Instant) -> bool {
        self.deadline.is_none_or(|deadline| now < deadline)
    }
}

pub(crate) struct StartupSplashPresentation {
    texture: BatchTexture,
    instance_buffer: wgpu::Buffer,
    hold: VisibleHold,
}

impl StartupSplashPresentation {
    pub(crate) fn build(
        gpu: &GpuContext,
        batch: &BatchRenderer,
        assets: &AssetManager,
        csf: Option<&CsfFile>,
        font: &FntFile,
        client_width: u32,
        client_height: u32,
    ) -> Result<Self> {
        anyhow::ensure!(
            client_width > 0 && client_height > 0,
            "startup splash requires a non-zero client size"
        );

        let rgba = compose_startup_splash(
            assets,
            csf,
            font,
            client_width as usize,
            client_height as usize,
        )?;
        let texture = batch.create_texture(gpu, &rgba, client_width, client_height);
        let instance = SpriteInstance {
            position: [0.0, 0.0],
            size: [client_width as f32, client_height as f32],
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            depth: 0.0,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        };
        let (instance_buffer, count) = batch
            .create_instance_buffer(gpu, &[instance])
            .context("create startup splash instance buffer")?;
        debug_assert_eq!(count, 1);

        Ok(Self {
            texture,
            instance_buffer,
            hold: VisibleHold::default(),
        })
    }

    pub(crate) fn mark_presented(&mut self, now: Instant) {
        self.hold.mark_presented(now);
    }

    pub(crate) fn is_active(&self, now: Instant) -> bool {
        self.hold.is_active(now)
    }
}

pub(crate) fn render_and_present(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    presenter: &ShellSurfacePresenter,
    depth: &wgpu::TextureView,
    splash: &StartupSplashPresentation,
) -> Result<()> {
    batch.update_camera(
        gpu,
        gpu.config.width as f32,
        gpu.config.height as f32,
        0.0,
        0.0,
        1.0,
    );

    let output = gpu
        .surface
        .get_current_texture()
        .context("acquire startup splash surface")?;
    let source_view = presenter.source_render_view();
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Retail Startup Splash"),
        });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Retail Startup Splash Composition"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &source_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                // The shared passthrough batch pipeline declares Depth32Float
                // even though it compares Always and does not write depth.
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        batch.draw_with_buffer_ui_passthrough(
            &mut pass,
            &splash.texture,
            &splash.instance_buffer,
            1,
        );
    }
    presenter.encode_present(&mut encoder, &output.texture);
    gpu.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    Ok(())
}

fn compose_startup_splash(
    assets: &AssetManager,
    csf: Option<&CsfFile>,
    font: &FntFile,
    client_width: usize,
    client_height: usize,
) -> Result<Vec<u8>> {
    let mut canvas = vec![0u8; client_width * client_height * 4];
    for pixel in canvas.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    if let (Some(palette_bytes), Some(shp_load)) = (
        assets.get_ref(SPLASH_PALETTE),
        assets.load_file_from_mix(splash_shp_for_width(client_width as u32)),
    ) {
        match (
            Palette::from_bytes_gamemd_ui(palette_bytes),
            ShpFile::from_bytes(&shp_load.bytes),
        ) {
            (Ok(palette), Ok(shp)) => {
                if let Some(frame) = shp.frames.first() {
                    let frame_rgba = shp
                        .frame_to_rgba_ui(0, &palette)
                        .context("decode startup splash SHP frame 0")?;
                    let origin_x = centered_offset(client_width as i32, shp.width as i32)
                        + frame.frame_x as i32;
                    let origin_y = centered_offset(client_height as i32, shp.height as i32)
                        + frame.frame_y as i32;
                    blit_rgba_clipped(
                        &mut canvas,
                        client_width,
                        client_height,
                        &frame_rgba,
                        frame.frame_width as usize,
                        frame.frame_height as usize,
                        origin_x,
                        origin_y,
                    );
                }
            }
            (Err(err), _) => log::warn!("Could not parse {SPLASH_PALETTE}: {err}"),
            (_, Err(err)) => log::warn!(
                "Could not parse {}: {err}",
                splash_shp_for_width(client_width as u32)
            ),
        }
    } else {
        log::warn!(
            "Retail startup splash art unavailable (palette={}, shp={}); drawing text over black",
            SPLASH_PALETTE,
            splash_shp_for_width(client_width as u32)
        );
    }

    let copyright = csf_text(csf, COPYRIGHT_KEY, COPYRIGHT_FALLBACK);
    let brand = csf_text(csf, BRAND_KEY, BRAND_FALLBACK);
    let loading = csf_text(csf, LOADING_KEY, LOADING_FALLBACK);
    let trademark_top = csf_text(csf, TRADEMARK_TOP_KEY, TRADEMARK_TOP_FALLBACK);
    let trademark_bottom = csf_text(csf, TRADEMARK_BOTTOM_KEY, TRADEMARK_BOTTOM_FALLBACK);

    let first_bottom_y = client_height as i32 - 40;
    let second_bottom_y = first_bottom_y + 3 + font.cell_height as i32;

    draw_text(
        &mut canvas,
        client_width,
        client_height,
        font,
        &copyright,
        client_width as i32 - font.text_width(&copyright) as i32 - 10,
        first_bottom_y,
    );
    draw_text(
        &mut canvas,
        client_width,
        client_height,
        font,
        &brand,
        client_width as i32 - font.text_width(&brand) as i32 - 10,
        second_bottom_y,
    );
    draw_text(
        &mut canvas,
        client_width,
        client_height,
        font,
        &loading,
        10,
        10,
    );
    draw_text(
        &mut canvas,
        client_width,
        client_height,
        font,
        &trademark_top,
        10,
        first_bottom_y,
    );
    draw_text(
        &mut canvas,
        client_width,
        client_height,
        font,
        &trademark_bottom,
        10,
        second_bottom_y,
    );

    Ok(canvas)
}

fn splash_shp_for_width(client_width: u32) -> &'static str {
    if client_width == 640 {
        SMALL_SPLASH_SHP
    } else {
        LARGE_SPLASH_SHP
    }
}

fn centered_offset(client_extent: i32, art_extent: i32) -> i32 {
    (client_extent - art_extent) / 2
}

fn csf_text<'a>(csf: Option<&'a CsfFile>, key: &str, fallback: &'a str) -> String {
    match csf {
        Some(table) => table.text(key).into_owned(),
        None => fallback.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_rgba_clipped(
    destination: &mut [u8],
    destination_width: usize,
    destination_height: usize,
    source: &[u8],
    source_width: usize,
    source_height: usize,
    destination_x: i32,
    destination_y: i32,
) {
    for source_y in 0..source_height {
        let target_y = destination_y + source_y as i32;
        if !(0..destination_height as i32).contains(&target_y) {
            continue;
        }
        for source_x in 0..source_width {
            let target_x = destination_x + source_x as i32;
            if !(0..destination_width as i32).contains(&target_x) {
                continue;
            }
            let source_offset = (source_y * source_width + source_x) * 4;
            let alpha = source.get(source_offset + 3).copied().unwrap_or(0);
            if alpha == 0 {
                continue;
            }
            let destination_offset =
                (target_y as usize * destination_width + target_x as usize) * 4;
            destination[destination_offset..destination_offset + 4]
                .copy_from_slice(&source[source_offset..source_offset + 4]);
        }
    }
}

fn draw_text(
    canvas: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    font: &FntFile,
    text: &str,
    x: i32,
    y: i32,
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        let codepoint = ch as u32;
        if codepoint > u16::MAX as u32 {
            continue;
        }
        let Some(glyph) = font.glyph(codepoint as u16) else {
            continue;
        };
        for glyph_y in 0..font.bitmap_rows as usize {
            let target_y = y + glyph_y as i32;
            if !(0..canvas_height as i32).contains(&target_y) {
                continue;
            }
            for glyph_x in 0..glyph.width as usize {
                let target_x = cursor_x + glyph_x as i32;
                if !(0..canvas_width as i32).contains(&target_x) {
                    continue;
                }
                let glyph_offset = (glyph_y * glyph.width as usize + glyph_x) * 4;
                if glyph.rgba.get(glyph_offset + 3).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let canvas_offset = (target_y as usize * canvas_width + target_x as usize) * 4;
                canvas[canvas_offset..canvas_offset + 4].copy_from_slice(&TEXT_COLOR);
            }
        }
        cursor_x += glyph.width as i32 + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialized_empty_csf() -> CsfFile {
        let mut bytes = Vec::new();
        for value in [0x4353_4620_u32, 3, 1, 1, 0, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        CsfFile::from_bytes(&bytes).expect("valid initialized CSF header")
    }

    #[test]
    fn exact_640_selects_small_every_other_width_selects_large() {
        assert_eq!(splash_shp_for_width(640), SMALL_SPLASH_SHP);
        assert_eq!(splash_shp_for_width(639), LARGE_SPLASH_SHP);
        assert_eq!(splash_shp_for_width(800), LARGE_SPLASH_SHP);
        assert_eq!(splash_shp_for_width(1920), LARGE_SPLASH_SHP);
    }

    #[test]
    fn centered_offset_uses_signed_truncation_toward_zero() {
        assert_eq!(centered_offset(800, 800), 0);
        assert_eq!(centered_offset(801, 800), 0);
        assert_eq!(centered_offset(799, 800), 0);
        assert_eq!(centered_offset(798, 800), -1);
        assert_eq!(centered_offset(806, 800), 3);
    }

    #[test]
    fn initialized_csf_exposes_missing_label_instead_of_english_fallback() {
        let csf = initialized_empty_csf();
        assert_eq!(
            csf_text(Some(&csf), COPYRIGHT_KEY, COPYRIGHT_FALLBACK),
            format!("MISSING:'{COPYRIGHT_KEY}'")
        );
        assert_eq!(
            csf_text(None, COPYRIGHT_KEY, COPYRIGHT_FALLBACK),
            COPYRIGHT_FALLBACK
        );
    }

    #[test]
    fn clipped_blit_preserves_black_border_and_skips_transparent_pixels() {
        let mut destination = vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255];
        let source = vec![
            10, 20, 30, 255, 40, 50, 60, 0, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        blit_rgba_clipped(&mut destination, 2, 2, &source, 2, 2, -1, 0);
        assert_eq!(&destination[0..4], &[0, 0, 0, 255]);
        assert_eq!(&destination[8..12], &[100, 110, 120, 255]);
    }

    #[test]
    fn the_hold_anchors_at_the_first_present_and_never_rearms() {
        let start = Instant::now();
        let mut hold = VisibleHold::default();
        // Unarmed: still active, so a transient first-present failure retries
        // rather than dropping the splash.
        assert!(hold.is_active(start + Duration::from_secs(600)));

        hold.mark_presented(start);
        // The per-frame re-present calls this again on every frame; if it
        // re-armed, the splash would never expire.
        hold.mark_presented(start + Duration::from_secs(3));
        hold.mark_presented(start + MINIMUM_VISIBLE_TIME);

        assert!(hold.is_active(start));
        assert!(hold.is_active(start + MINIMUM_VISIBLE_TIME - Duration::from_millis(1)));
        // The hold is measured from the present, so initialization work that
        // runs after it is spent inside the five seconds, not added to them.
        assert!(!hold.is_active(start + MINIMUM_VISIBLE_TIME));
        assert!(!hold.is_active(start + MINIMUM_VISIBLE_TIME + Duration::from_secs(1)));
    }
}
