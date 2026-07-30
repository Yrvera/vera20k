//! Retail one-shot screenshot completion.
//!
//! The app supplies the already-composited swapchain pixels. This module only
//! converts BGRA8 to the active-retail direct-RGB PCX form, probes
//! the current working directory, and writes the first unused `SCRN%04d.pcx`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, ensure};

pub(crate) const READBACK_TIMEOUT: Duration = Duration::from_secs(15);

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
        rgb.extend_from_slice(&[bgra[2], bgra[1], bgra[0]]);
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
}
