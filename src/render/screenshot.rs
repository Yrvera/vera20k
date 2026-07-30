//! Retail one-shot screenshot completion.
//!
//! The app retains the last presented client surface so a hotkey captures the
//! pixels already visible when the input is dispatched. This module queues that
//! readback, expands the active RGB565 surface into direct-RGB PCX bytes, probes
//! the current working directory, and writes the first unused `SCRN%04d.pcx`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, ensure};

pub(crate) const READBACK_TIMEOUT: Duration = Duration::from_secs(15);

/// Last complete client surface committed to presentation.
#[derive(Default)]
pub(crate) struct PresentedFrameCache {
    texture: Option<wgpu::Texture>,
    width: u32,
    height: u32,
    format: Option<wgpu::TextureFormat>,
    valid: bool,
}

impl PresentedFrameCache {
    /// Queue a readback of the previously presented surface when requested,
    /// then retain the current surface for the next input-dispatch boundary.
    pub(crate) fn capture_previous_and_remember_current(
        &mut self,
        requested: bool,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        current: &wgpu::Texture,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<Option<crate::render::frame_readback::PendingBgra8Readback>> {
        self.ensure_texture(device, format, width, height);
        let retained = self
            .texture
            .as_ref()
            .expect("presented-frame cache texture was just ensured");
        let readback_source = if self.valid { retained } else { current };
        let pending = requested
            .then(|| {
                crate::render::frame_readback::PendingBgra8Readback::encode(
                    device,
                    encoder,
                    readback_source,
                    format,
                    width,
                    height,
                )
            })
            .transpose()?;

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: current,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: retained,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.valid = true;
        Ok(pending)
    }

    fn ensure_texture(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) {
        if self.texture.is_some()
            && self.width == width
            && self.height == height
            && self.format == Some(format)
        {
            return;
        }

        self.texture = Some(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Last presented client frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }));
        self.width = width;
        self.height = height;
        self.format = Some(format);
        self.valid = false;
    }
}

/// Write one final client-frame readback using the active retail naming and PCX
/// contract. `pixels` must be tightly packed, top-left-origin BGRA8.
pub(crate) fn write_retail_screenshot(
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    pixels: &[u8],
) -> Result<PathBuf> {
    ensure!(
        matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ),
        "retail screenshot requires a BGRA8 client surface, got {format:?}"
    );

    let pixel_count = width
        .checked_mul(height)
        .context("screenshot pixel count overflowed")?;
    let expected_len = usize::try_from(
        pixel_count
            .checked_mul(4)
            .context("screenshot byte count overflowed")?,
    )
    .context("screenshot byte count does not fit this platform")?;
    ensure!(
        pixels.len() == expected_len,
        "screenshot readback has {} bytes, expected {expected_len}",
        pixels.len()
    );

    let pcx_width = u16::try_from(width).context("screenshot width does not fit PCX")?;
    let pcx_height = u16::try_from(height).context("screenshot height does not fit PCX")?;
    let mut rgb = Vec::with_capacity(
        usize::try_from(
            pixel_count
                .checked_mul(3)
                .context("screenshot RGB byte count overflowed")?,
        )
        .context("screenshot RGB byte count does not fit this platform")?,
    );
    for bgra in pixels.chunks_exact(4) {
        rgb.extend_from_slice(&active_retail_rgb565_expansion(bgra));
    }

    let encoded = crate::assets::pcx_file::encode_direct_rgb(pcx_width, pcx_height, &rgb)
        .map_err(anyhow::Error::new)
        .context("could not encode retail screenshot PCX")?;
    let path = first_unused_screenshot_path(Path::new("."))
        .context("the SCRN screenshot filename space is exhausted")?;
    std::fs::write(&path, encoded)
        .with_context(|| format!("could not write screenshot {}", path.display()))?;
    Ok(path)
}

fn active_retail_rgb565_expansion(bgra: &[u8]) -> [u8; 3] {
    [bgra[2] & 0xF8, bgra[1] & 0xFC, bgra[0] & 0xF8]
}

fn first_unused_screenshot_path(directory: &Path) -> Option<PathBuf> {
    first_unused_screenshot_path_with(directory, Path::exists)
}

fn first_unused_screenshot_path_with(
    directory: &Path,
    mut exists: impl FnMut(&Path) -> bool,
) -> Option<PathBuf> {
    (0..=i32::MAX)
        .map(|index| directory.join(format!("SCRN{index:04}.pcx")))
        .find(|candidate| !exists(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_probe_starts_at_zero_and_uses_first_gap() {
        let occupied = ["SCRN0000.pcx", "SCRN0001.pcx", "SCRN0003.pcx"];
        let path = first_unused_screenshot_path_with(Path::new("."), |candidate| {
            candidate
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| occupied.contains(&name))
        })
        .expect("unused screenshot path");

        assert_eq!(path.file_name().unwrap(), "SCRN0002.pcx");
    }

    #[test]
    fn filename_width_is_a_minimum_not_a_four_digit_cap() {
        let path = first_unused_screenshot_path_with(Path::new("."), |candidate| {
            candidate.file_name().unwrap() != "SCRN10000.pcx"
        })
        .expect("unused screenshot path");
        assert_eq!(path.file_name().unwrap(), "SCRN10000.pcx");
    }

    #[test]
    fn active_surface_expansion_preserves_only_rgb565_channel_bits() {
        assert_eq!(
            active_retail_rgb565_expansion(&[0xFF, 0xAB, 0x7F, 0x42]),
            [0x78, 0xA8, 0xF8]
        );
    }
}
