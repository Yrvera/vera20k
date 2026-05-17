//! Skirmish shell chrome atlas.
//!
//! Loads verified offline Skirmish dialog `0x102` shell art, then packs it
//! into a GPU texture for batched drawing. Assets without direct active
//! Skirmish evidence are research candidates and must not be rendered by the
//! default shell path.

use std::collections::HashMap;

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::assets::pcx_file::PcxFile;
use crate::assets::shp_file::ShpFile;
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;

const ATLAS_PADDING: u32 = 2;

#[derive(Debug, Clone, Copy)]
pub struct SkirmishShellChromeEntry {
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_size: [f32; 2],
}

pub struct SkirmishShellChromeAtlas {
    pub texture: BatchTexture,
    pub sd_top: Option<SkirmishShellChromeEntry>,
    pub sd_tile: Option<SkirmishShellChromeEntry>,
    pub sd_bottom: Option<SkirmishShellChromeEntry>,
    pub sd_button_anim: Option<SkirmishShellChromeEntry>,
    pub sd_map_button: Option<SkirmishShellChromeEntry>,
    pub background_640_mnscrnl: Option<SkirmishShellChromeEntry>,
    pub background_800_coop_game_setup: Option<SkirmishShellChromeEntry>,
    pub button_up_left_30: Option<SkirmishShellChromeEntry>,
    pub button_up_mid_30: Option<SkirmishShellChromeEntry>,
    pub button_up_right_30: Option<SkirmishShellChromeEntry>,
    pub button_down_left_30: Option<SkirmishShellChromeEntry>,
    pub button_down_mid_30: Option<SkirmishShellChromeEntry>,
    pub button_down_right_30: Option<SkirmishShellChromeEntry>,
    pub start_marker: Option<SkirmishShellChromeEntry>,
    pub assigned_player_marker_mmpb: Option<SkirmishShellChromeEntry>,
    pub flags: Vec<(String, SkirmishShellChromeEntry)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
enum ShellAssetRole {
    VerifiedParentBackground,
    VerifiedOfflineStartMarker,
    AssignedPlayerMarker,
    RightPanelChrome,
    VerifiedOwnerDrawButton,
    VerifiedFlag,
    ResearchCandidate,
    Other,
}

struct RenderedShellEntry {
    label: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

pub fn build_skirmish_shell_chrome_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    assets: &AssetManager,
) -> Option<SkirmishShellChromeAtlas> {
    let right_panel_palette = load_unresolved_shell_shp_palette(assets)?;
    let parent_background_palette = load_parent_background_palette(assets);

    let mut rendered = Vec::new();
    for name in [
        "SDTP.SHP",
        "SDBTNBKGD.SHP",
        "SDBTM.SHP",
        "SDBTNANM.SHP",
        "SDMPBTN.SHP",
    ] {
        let entry = render_shp_entry(assets, name, &right_panel_palette, 0).or_else(|| {
            log::warn!("Missing mandatory Skirmish shell asset {name}");
            None
        })?;
        rendered.push(entry);
    }

    if let Some(parent_background_palette) = parent_background_palette.as_ref() {
        for name in ["MNSCRNL.SHP", "MnScrnLCoopGameSetup.shp"] {
            push_optional(
                &mut rendered,
                render_shp_entry(assets, name, parent_background_palette, 0),
                name,
            );
        }
    } else {
        log::warn!(
            "Skipping verified Skirmish parent backgrounds because MnScrnLCoopGameSetup.PAL is missing or invalid"
        );
    }

    for name in ["STARTBUT.SHP", "mmpb.shp"] {
        push_optional(
            &mut rendered,
            render_shp_entry(assets, name, &right_panel_palette, 0),
            name,
        );
    }

    for name in [
        "bue_li30.pcx",
        "bue_mi30.pcx",
        "bue_ri30.pcx",
        "bde_li30.pcx",
        "bde_mi30.pcx",
        "bde_ri30.pcx",
        "usai.pcx",
        "japi.pcx",
        "frai.pcx",
        "geri.pcx",
        "gbri.pcx",
        "djbi.pcx",
        "arbi.pcx",
        "lati.pcx",
        "rusi.pcx",
        "yrii.pcx",
        "obsi.pcx",
        "rani.pcx",
    ] {
        push_optional(&mut rendered, render_pcx_entry(assets, name, Some(0)), name);
    }

    let (texture, packed) = pack_entries(gpu, batch, &rendered)?;
    let by_label: HashMap<String, SkirmishShellChromeEntry> = rendered
        .iter()
        .map(|entry| entry.label.clone())
        .zip(packed)
        .collect();
    let flags = [
        "usai.pcx", "japi.pcx", "frai.pcx", "geri.pcx", "gbri.pcx", "djbi.pcx", "arbi.pcx",
        "lati.pcx", "rusi.pcx", "yrii.pcx", "obsi.pcx", "rani.pcx",
    ]
    .into_iter()
    .filter_map(|name| {
        by_label
            .get(name)
            .copied()
            .map(|entry| (name.to_string(), entry))
    })
    .collect();

    Some(SkirmishShellChromeAtlas {
        texture,
        sd_top: by_label.get("sdtp.shp").copied(),
        sd_tile: by_label.get("sdbtnbkgd.shp").copied(),
        sd_bottom: by_label.get("sdbtm.shp").copied(),
        sd_button_anim: by_label.get("sdbtnanm.shp").copied(),
        sd_map_button: by_label.get("sdmpbtn.shp").copied(),
        background_640_mnscrnl: by_label.get("mnscrnl.shp").copied(),
        background_800_coop_game_setup: by_label.get("mnscrnlcoopgamesetup.shp").copied(),
        button_up_left_30: by_label.get("bue_li30.pcx").copied(),
        button_up_mid_30: by_label.get("bue_mi30.pcx").copied(),
        button_up_right_30: by_label.get("bue_ri30.pcx").copied(),
        button_down_left_30: by_label.get("bde_li30.pcx").copied(),
        button_down_mid_30: by_label.get("bde_mi30.pcx").copied(),
        button_down_right_30: by_label.get("bde_ri30.pcx").copied(),
        start_marker: by_label.get("startbut.shp").copied(),
        assigned_player_marker_mmpb: by_label.get("mmpb.shp").copied(),
        flags,
    })
}

fn load_parent_background_palette(assets: &AssetManager) -> Option<Palette> {
    let Some(palette_bytes) = assets.get_ref("MnScrnLCoopGameSetup.PAL") else {
        log::warn!("Missing verified Skirmish parent-background palette MnScrnLCoopGameSetup.PAL");
        return None;
    };
    Palette::from_bytes(palette_bytes)
        .map_err(|err| {
            log::warn!(
                "Could not parse verified Skirmish parent-background palette MnScrnLCoopGameSetup.PAL: {err:#}"
            );
            err
        })
        .ok()
}

fn load_unresolved_shell_shp_palette(assets: &AssetManager) -> Option<Palette> {
    // Exact shell SHP palette binding is still open pending live Ghidra.
    // PCX owner-draw assets do not use this path; they carry embedded palettes.
    let palette_bytes = assets
        .get_ref("SDBTNANM.PAL")
        .or_else(|| assets.get_ref("SHELL.PAL"))
        .or_else(|| assets.get_ref("DIALOG.PAL"))?;
    Palette::from_bytes(palette_bytes).ok()
}

#[cfg(test)]
fn classify_shell_asset(name: &str) -> ShellAssetRole {
    match name.to_ascii_lowercase().as_str() {
        "mnscrnl.shp" | "mnscrnlcoopgamesetup.shp" => ShellAssetRole::VerifiedParentBackground,
        "startbut.shp" => ShellAssetRole::VerifiedOfflineStartMarker,
        "mmpb.shp" => ShellAssetRole::AssignedPlayerMarker,
        "sdtp.shp" | "sdbtnbkgd.shp" | "sdbtm.shp" | "sdbtnanm.shp" | "sdmpbtn.shp" => {
            ShellAssetRole::RightPanelChrome
        }
        "bue_li30.pcx" | "bue_mi30.pcx" | "bue_ri30.pcx" | "bde_li30.pcx" | "bde_mi30.pcx"
        | "bde_ri30.pcx" => ShellAssetRole::VerifiedOwnerDrawButton,
        "usai.pcx" | "japi.pcx" | "frai.pcx" | "geri.pcx" | "gbri.pcx" | "djbi.pcx"
        | "arbi.pcx" | "lati.pcx" | "rusi.pcx" | "yrii.pcx" | "obsi.pcx" | "rani.pcx" => {
            ShellAssetRole::VerifiedFlag
        }
        "mnscrns.shp"
        | "mnscrnlcustomizebattle.shp"
        | "dbak6440.pcx"
        | "dlgsysa.pcx"
        | "dlgsysi.pcx" => ShellAssetRole::ResearchCandidate,
        _ => ShellAssetRole::Other,
    }
}

fn push_optional(
    entries: &mut Vec<RenderedShellEntry>,
    entry: Option<RenderedShellEntry>,
    name: &str,
) {
    if let Some(entry) = entry {
        entries.push(entry);
    } else {
        log::warn!("Missing optional Skirmish shell asset {name}");
    }
}

fn render_shp_entry(
    assets: &AssetManager,
    file_name: &str,
    palette: &Palette,
    frame: usize,
) -> Option<RenderedShellEntry> {
    let bytes = assets.get_ref(file_name)?;
    let shp = ShpFile::from_bytes(bytes).ok()?;
    if frame >= shp.frames.len() {
        return None;
    }
    let frame_rgba = shp.frame_to_rgba(frame, palette).ok()?;
    let canvas_w = shp.width as u32;
    let canvas_h = shp.height as u32;
    let shp_frame = &shp.frames[frame];
    let frame_w = shp_frame.frame_width as u32;
    let frame_h = shp_frame.frame_height as u32;
    let frame_x = shp_frame.frame_x as u32;
    let frame_y = shp_frame.frame_y as u32;
    let rgba = if frame_w == canvas_w && frame_h == canvas_h && frame_x == 0 && frame_y == 0 {
        frame_rgba
    } else {
        let mut canvas = vec![0u8; (canvas_w * canvas_h * 4) as usize];
        for row in 0..frame_h {
            let src = (row * frame_w * 4) as usize;
            let dst = (((frame_y + row) * canvas_w + frame_x) * 4) as usize;
            let len = (frame_w * 4) as usize;
            if src + len <= frame_rgba.len() && dst + len <= canvas.len() {
                canvas[dst..dst + len].copy_from_slice(&frame_rgba[src..src + len]);
            }
        }
        canvas
    };
    Some(RenderedShellEntry {
        label: file_name.to_ascii_lowercase(),
        width: canvas_w,
        height: canvas_h,
        rgba,
    })
}

fn render_pcx_entry(
    assets: &AssetManager,
    file_name: &str,
    transparent_index: Option<u8>,
) -> Option<RenderedShellEntry> {
    let bytes = assets.get_ref(file_name)?;
    let pcx = PcxFile::from_bytes(bytes).ok()?;
    Some(RenderedShellEntry {
        label: file_name.to_ascii_lowercase(),
        width: pcx.width as u32,
        height: pcx.height as u32,
        rgba: pcx.to_rgba(transparent_index),
    })
}

fn pack_entries(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    entries: &[RenderedShellEntry],
) -> Option<(BatchTexture, Vec<SkirmishShellChromeEntry>)> {
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
        "Skirmish shell chrome atlas: {}x{} px, {} pieces",
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
        .map(|(entry, (px, py))| SkirmishShellChromeEntry {
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
    use super::{
        AssetManager, ShellAssetRole, classify_shell_asset, load_parent_background_palette,
        load_unresolved_shell_shp_palette, render_shp_entry,
    };

    #[test]
    fn skirmish_shell_asset_classification_matches_live_render_path() {
        assert_eq!(
            classify_shell_asset("MNSCRNL.SHP"),
            ShellAssetRole::VerifiedParentBackground
        );
        assert_eq!(
            classify_shell_asset("MnScrnLCoopGameSetup.shp"),
            ShellAssetRole::VerifiedParentBackground
        );
        assert_eq!(
            classify_shell_asset("STARTBUT.SHP"),
            ShellAssetRole::VerifiedOfflineStartMarker
        );
        assert_eq!(
            classify_shell_asset("bue_li30.pcx"),
            ShellAssetRole::VerifiedOwnerDrawButton
        );
        assert_eq!(
            classify_shell_asset("mmpb.shp"),
            ShellAssetRole::AssignedPlayerMarker
        );
        assert_ne!(
            classify_shell_asset("mmpb.shp"),
            ShellAssetRole::VerifiedOfflineStartMarker
        );
        assert_ne!(
            classify_shell_asset("mmpb.shp"),
            ShellAssetRole::VerifiedOfflineStartMarker
        );
        assert_eq!(
            classify_shell_asset("SDMPBTN.SHP"),
            ShellAssetRole::RightPanelChrome
        );
        assert_ne!(
            classify_shell_asset("SDMPBTN.SHP"),
            ShellAssetRole::VerifiedOfflineStartMarker
        );
        assert_eq!(
            classify_shell_asset("MnScrnLCustomizeBattle.shp"),
            ShellAssetRole::ResearchCandidate
        );
        assert_ne!(
            classify_shell_asset("MnScrnLCustomizeBattle.shp"),
            ShellAssetRole::VerifiedOwnerDrawButton
        );
        assert_ne!(
            classify_shell_asset("sidebar.pal"),
            ShellAssetRole::VerifiedOwnerDrawButton
        );
    }

    #[test]
    #[ignore]
    fn retail_shell_shp_dimensions_match_research() {
        let config = crate::util::config::GameConfig::load().expect("game config");
        let assets = AssetManager::new(&config.paths.ra2_dir).expect("asset manager");
        let palette = load_unresolved_shell_shp_palette(&assets).expect("shell SHP palette");
        let sdbtn = render_shp_entry(&assets, "SDBTNANM.SHP", &palette, 0).expect("SDBTNANM");
        assert_eq!((sdbtn.width, sdbtn.height), (156, 42));
    }

    #[test]
    #[ignore]
    fn retail_parent_backgrounds_decode_with_verified_palette() {
        let config = crate::util::config::GameConfig::load().expect("game config");
        let assets = AssetManager::new(&config.paths.ra2_dir).expect("asset manager");
        let palette = load_parent_background_palette(&assets).expect("parent palette");
        render_shp_entry(&assets, "MNSCRNL.SHP", &palette, 0).expect("MNSCRNL");
        render_shp_entry(&assets, "MnScrnLCoopGameSetup.shp", &palette, 0)
            .expect("MnScrnLCoopGameSetup");
    }
}
