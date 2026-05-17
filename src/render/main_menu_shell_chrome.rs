//! Initial main-menu shell chrome atlas for dialog 0xE2 owner-draw buttons.

use crate::assets::asset_manager::AssetManager;
use crate::assets::pcx_file::PcxFile;
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;

const ATLAS_PADDING: u32 = 2;

#[derive(Debug, Clone, Copy)]
pub struct MainMenuShellChromeEntry {
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_size: [f32; 2],
}

pub struct MainMenuShellChromeAtlas {
    pub texture: BatchTexture,
    pub button_up_left_30: MainMenuShellChromeEntry,
    pub button_up_mid_30: MainMenuShellChromeEntry,
    pub button_up_right_30: MainMenuShellChromeEntry,
    pub button_down_left_30: MainMenuShellChromeEntry,
    pub button_down_mid_30: MainMenuShellChromeEntry,
    pub button_down_right_30: MainMenuShellChromeEntry,
}

const BUTTON_PCX_NAMES: [&str; 6] = [
    "bue_li30.pcx",
    "bue_mi30.pcx",
    "bue_ri30.pcx",
    "bde_li30.pcx",
    "bde_mi30.pcx",
    "bde_ri30.pcx",
];

struct RenderedChromeEntry {
    label: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

pub fn build_main_menu_shell_chrome_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    assets: &AssetManager,
) -> Option<MainMenuShellChromeAtlas> {
    let mut rendered = Vec::new();
    for name in BUTTON_PCX_NAMES {
        let entry = render_pcx_entry(assets, name).or_else(|| {
            log::warn!("Missing mandatory main-menu shell asset {name}");
            None
        })?;
        rendered.push(entry);
    }
    let (texture, packed) = pack_entries(gpu, batch, &rendered)?;
    log::info!("Main-menu shell chrome atlas loaded");
    Some(MainMenuShellChromeAtlas {
        texture,
        button_up_left_30: packed[0],
        button_up_mid_30: packed[1],
        button_up_right_30: packed[2],
        button_down_left_30: packed[3],
        button_down_mid_30: packed[4],
        button_down_right_30: packed[5],
    })
}

fn render_pcx_entry(assets: &AssetManager, name: &str) -> Option<RenderedChromeEntry> {
    let bytes = assets.get_ref(name)?;
    let pcx = PcxFile::from_bytes(bytes)
        .map_err(|err| log::warn!("Failed to parse {name}: {err}"))
        .ok()?;
    Some(RenderedChromeEntry {
        label: name.to_ascii_lowercase(),
        width: pcx.width as u32,
        height: pcx.height as u32,
        rgba: pcx.to_rgba(Some(0)),
    })
}

fn pack_entries(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    entries: &[RenderedChromeEntry],
) -> Option<(BatchTexture, Vec<MainMenuShellChromeEntry>)> {
    if entries.is_empty() {
        return None;
    }

    let atlas_width = entries
        .iter()
        .map(|entry| entry.width)
        .max()
        .unwrap_or(1)
        .max(1024);
    let mut x = 0u32;
    let mut y = 0u32;
    let mut row_h = 0u32;
    let mut placements = Vec::with_capacity(entries.len());

    for entry in entries {
        if x > 0 && x + entry.width + ATLAS_PADDING > atlas_width {
            x = 0;
            y += row_h + ATLAS_PADDING;
            row_h = 0;
        }
        placements.push((x, y));
        x += entry.width + ATLAS_PADDING;
        row_h = row_h.max(entry.height);
    }

    let atlas_height = (y + row_h).next_power_of_two().max(1);
    let mut rgba = vec![0u8; (atlas_width * atlas_height * 4) as usize];

    for (entry, (px, py)) in entries.iter().zip(placements.iter().copied()) {
        for row in 0..entry.height {
            let src = (row * entry.width * 4) as usize;
            let dst = (((py + row) * atlas_width + px) * 4) as usize;
            let len = (entry.width * 4) as usize;
            rgba[dst..dst + len].copy_from_slice(&entry.rgba[src..src + len]);
        }
    }

    log::info!(
        "Main-menu shell chrome atlas: {}x{} px, {} pieces",
        atlas_width,
        atlas_height,
        entries.len()
    );
    for entry in entries {
        log::info!("  {}: {}x{}", entry.label, entry.width, entry.height);
    }

    let texture = batch.create_texture(gpu, &rgba, atlas_width, atlas_height);
    let atlas_entries = entries
        .iter()
        .zip(placements)
        .map(|(entry, (px, py))| MainMenuShellChromeEntry {
            uv_origin: [
                px as f32 / atlas_width as f32,
                py as f32 / atlas_height as f32,
            ],
            uv_size: [
                entry.width as f32 / atlas_width as f32,
                entry.height as f32 / atlas_height as f32,
            ],
            pixel_size: [entry.width as f32, entry.height as f32],
        })
        .collect();
    Some((texture, atlas_entries))
}

#[cfg(test)]
mod tests {
    use super::BUTTON_PCX_NAMES;

    #[test]
    fn mandatory_owner_draw_assets_match_dialog_0xe2() {
        assert_eq!(
            BUTTON_PCX_NAMES,
            [
                "bue_li30.pcx",
                "bue_mi30.pcx",
                "bue_ri30.pcx",
                "bde_li30.pcx",
                "bde_mi30.pcx",
                "bde_ri30.pcx",
            ]
        );
    }
}
