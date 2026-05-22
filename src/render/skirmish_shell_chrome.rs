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
const OWNER_DRAW_FLAG_TRANSPARENT_RGB: [u8; 3] = [255, 0, 255];
const PRIMITIVE_BEVEL_COLOR_A_RGB: [u8; 3] = [0xC5, 0xBE, 0xA7];
const PRIMITIVE_BEVEL_COLOR_B_RGB: [u8; 3] = [0x80, 0x7A, 0x68];
const SKIRMISH_FLAG_PCX_NAMES: [&str; 12] = [
    "usai.pcx", "japi.pcx", "frai.pcx", "geri.pcx", "gbri.pcx", "djbi.pcx", "arbi.pcx", "lati.pcx",
    "rusi.pcx", "yrii.pcx", "obsi.pcx", "rani.pcx",
];

#[derive(Debug, Clone, Copy)]
pub struct SkirmishShellChromeEntry {
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_size: [f32; 2],
}

pub struct SkirmishShellChromeAtlas {
    pub texture: BatchTexture,
    pub right_panel_top_sdtp: Option<SkirmishShellChromeEntry>,
    pub right_panel_tile_sdbtnbkgd: Option<SkirmishShellChromeEntry>,
    pub right_panel_overlay_sdbtnanm_frame10: Option<SkirmishShellChromeEntry>,
    pub right_panel_bottom_sdbtm: Option<SkirmishShellChromeEntry>,
    pub sd_map_button: Option<SkirmishShellChromeEntry>,
    pub background_640_mnscrns: Option<SkirmishShellChromeEntry>,
    pub background_800_coop_game_setup: Option<SkirmishShellChromeEntry>,
    pub lower_side_640_lwscrns: Option<SkirmishShellChromeEntry>,
    pub lower_side_large_lwscrnl: Option<SkirmishShellChromeEntry>,
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
    let shell_palette = load_shell_palette(assets)?;
    let shell2_palette = load_named_palette(assets, "SHELL2.PAL")?;
    let sdbtnanm_palette = load_named_palette(assets, "SDBTNANM.PAL");
    let parent_background_palette = load_parent_background_palette(assets);

    let mut rendered = Vec::new();
    rendered.push(mandatory_shp(
        assets,
        "SDTP.SHP",
        &shell_palette,
        0,
        "SHELL.PAL",
    )?);
    rendered.push(mandatory_shp(
        assets,
        "SDBTNBKGD.SHP",
        &shell2_palette,
        0,
        "SHELL2.PAL",
    )?);
    rendered.push(mandatory_shp(
        assets,
        "SDBTM.SHP",
        &shell_palette,
        0,
        "SHELL.PAL",
    )?);
    if let Some(sdbtnanm_palette) = sdbtnanm_palette.as_ref() {
        match render_shp_entry_labeled(
            assets,
            "SDBTNANM.SHP",
            "sdbtnanm.shp#10",
            sdbtnanm_palette,
            10,
        ) {
            Some(entry) => rendered.push(entry),
            None => log::warn!(
                "Missing optional Skirmish shell asset SDBTNANM.SHP frame 10; not substituting frame 0"
            ),
        }
    }
    rendered.push(mandatory_shp(
        assets,
        "SDMPBTN.SHP",
        &shell_palette,
        0,
        "SHELL.PAL",
    )?);
    rendered.push(mandatory_shp(
        assets,
        "LWSCRNS.SHP",
        &shell_palette,
        0,
        "SHELL.PAL",
    )?);
    rendered.push(mandatory_shp(
        assets,
        "LWSCRNL.SHP",
        &shell_palette,
        0,
        "SHELL.PAL",
    )?);

    if let Some(parent_background_palette) = parent_background_palette.as_ref() {
        for name in ["MNSCRNS.SHP", "MnScrnLCoopGameSetup.shp"] {
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
            render_shp_entry(assets, name, &shell_palette, 0),
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
    ] {
        push_optional(&mut rendered, render_pcx_entry(assets, name, Some(0)), name);
    }

    for name in SKIRMISH_FLAG_PCX_NAMES {
        push_optional(&mut rendered, render_flag_pcx_entry(assets, name), name);
    }

    let (texture, packed) = pack_entries(gpu, batch, &rendered)?;
    let by_label: HashMap<String, SkirmishShellChromeEntry> = rendered
        .iter()
        .map(|entry| entry.label.clone())
        .zip(packed)
        .collect();
    let flags = SKIRMISH_FLAG_PCX_NAMES
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
        right_panel_top_sdtp: by_label.get("sdtp.shp").copied(),
        right_panel_tile_sdbtnbkgd: by_label.get("sdbtnbkgd.shp").copied(),
        right_panel_overlay_sdbtnanm_frame10: by_label.get("sdbtnanm.shp#10").copied(),
        right_panel_bottom_sdbtm: by_label.get("sdbtm.shp").copied(),
        sd_map_button: by_label.get("sdmpbtn.shp").copied(),
        background_640_mnscrns: by_label.get("mnscrns.shp").copied(),
        background_800_coop_game_setup: by_label.get("mnscrnlcoopgamesetup.shp").copied(),
        lower_side_640_lwscrns: by_label.get("lwscrns.shp").copied(),
        lower_side_large_lwscrnl: by_label.get("lwscrnl.shp").copied(),
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

fn load_shell_palette(assets: &AssetManager) -> Option<Palette> {
    load_named_palette(assets, "SHELL.PAL")
}

fn load_named_palette(assets: &AssetManager, name: &str) -> Option<Palette> {
    let Some(palette_bytes) = assets.get_ref(name) else {
        log::warn!("Missing verified Skirmish shell palette {name}");
        return None;
    };
    Palette::from_bytes(palette_bytes)
        .map_err(|err| {
            log::warn!("Could not parse verified Skirmish shell palette {name}: {err:#}");
            err
        })
        .ok()
}

#[cfg(test)]
fn classify_shell_asset(name: &str) -> ShellAssetRole {
    match name.to_ascii_lowercase().as_str() {
        "mnscrns.shp" | "mnscrnlcoopgamesetup.shp" => ShellAssetRole::VerifiedParentBackground,
        "startbut.shp" => ShellAssetRole::VerifiedOfflineStartMarker,
        "mmpb.shp" => ShellAssetRole::AssignedPlayerMarker,
        "sdtp.shp" | "sdbtnbkgd.shp" | "sdbtm.shp" | "sdbtnanm.shp" | "sdmpbtn.shp"
        | "lwscrns.shp" | "lwscrnl.shp" => ShellAssetRole::RightPanelChrome,
        "bue_li30.pcx" | "bue_mi30.pcx" | "bue_ri30.pcx" | "bde_li30.pcx" | "bde_mi30.pcx"
        | "bde_ri30.pcx" => ShellAssetRole::VerifiedOwnerDrawButton,
        "usai.pcx" | "japi.pcx" | "frai.pcx" | "geri.pcx" | "gbri.pcx" | "djbi.pcx"
        | "arbi.pcx" | "lati.pcx" | "rusi.pcx" | "yrii.pcx" | "obsi.pcx" | "rani.pcx" => {
            ShellAssetRole::VerifiedFlag
        }
        "mnscrnl.shp"
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

fn mandatory_shp(
    assets: &AssetManager,
    file_name: &str,
    palette: &Palette,
    frame: usize,
    palette_name: &str,
) -> Option<RenderedShellEntry> {
    render_shp_entry(assets, file_name, palette, frame).or_else(|| {
        log::warn!(
            "Missing mandatory Skirmish shell asset {file_name} frame {frame} decoded with {palette_name}"
        );
        None
    })
}

fn render_shp_entry(
    assets: &AssetManager,
    file_name: &str,
    palette: &Palette,
    frame: usize,
) -> Option<RenderedShellEntry> {
    render_shp_entry_labeled(
        assets,
        file_name,
        &file_name.to_ascii_lowercase(),
        palette,
        frame,
    )
}

fn render_shp_entry_labeled(
    assets: &AssetManager,
    file_name: &str,
    label: &str,
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
        label: label.to_ascii_lowercase(),
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

fn render_flag_pcx_entry(assets: &AssetManager, file_name: &str) -> Option<RenderedShellEntry> {
    let bytes = assets.get_ref(file_name)?;
    let pcx = PcxFile::from_bytes(bytes).ok()?;
    Some(RenderedShellEntry {
        label: file_name.to_ascii_lowercase(),
        width: pcx.width as u32,
        height: pcx.height as u32,
        rgba: pcx.to_rgba_with_color_key(OWNER_DRAW_FLAG_TRANSPARENT_RGB),
    })
}

#[allow(dead_code)]
fn render_primitive_bevel_entry(
    label: &str,
    width: u32,
    height: u32,
    box_xywh: [i32; 4],
    border: i32,
) -> RenderedShellEntry {
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    if border <= 0 || box_xywh[2] <= 0 || box_xywh[3] <= 0 {
        return RenderedShellEntry {
            label: label.to_ascii_lowercase(),
            width,
            height,
            rgba,
        };
    }

    let left0 = box_xywh[0] - border;
    let top0 = box_xywh[1] - border;
    let right0 = box_xywh[0] + box_xywh[2] + border - 1;
    let bottom0 = box_xywh[1] + box_xywh[3] + border - 1;
    let color_a = rgba_color(PRIMITIVE_BEVEL_COLOR_A_RGB);
    let color_b = rgba_color(PRIMITIVE_BEVEL_COLOR_B_RGB);
    let mixed = rgba_color(average_rgb(
        PRIMITIVE_BEVEL_COLOR_A_RGB,
        PRIMITIVE_BEVEL_COLOR_B_RGB,
    ));

    for ring in 0..border {
        let left = left0 + ring;
        let top = top0 + ring;
        let right = right0 - ring;
        let bottom = bottom0 - ring;
        let (top_left_color, bottom_right_color) = if border == 2 && ring == 1 {
            (color_b, color_a)
        } else {
            (color_a, color_b)
        };

        draw_axis_line_inclusive_clipped(
            &mut rgba,
            width,
            height,
            (left, top),
            (right - 1, top),
            top_left_color,
        );
        draw_axis_line_inclusive_clipped(
            &mut rgba,
            width,
            height,
            (left, top + 1),
            (left, bottom),
            top_left_color,
        );
        draw_axis_line_inclusive_clipped(
            &mut rgba,
            width,
            height,
            (right, bottom),
            (left, bottom),
            bottom_right_color,
        );
        draw_axis_line_inclusive_clipped(
            &mut rgba,
            width,
            height,
            (right, bottom - 1),
            (right, top + 1),
            bottom_right_color,
        );

        if border == 2 {
            put_pixel_clipped(&mut rgba, width, height, right, top, mixed);
            put_pixel_clipped(&mut rgba, width, height, left, bottom, mixed);
        }
    }

    RenderedShellEntry {
        label: label.to_ascii_lowercase(),
        width,
        height,
        rgba,
    }
}

fn draw_axis_line_inclusive_clipped(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    start: (i32, i32),
    end: (i32, i32),
    color: [u8; 4],
) {
    if width == 0 || height == 0 {
        return;
    }

    let max_x = width.saturating_sub(1) as i32;
    let max_y = height.saturating_sub(1) as i32;
    if start.1 == end.1 {
        let y = start.1;
        if y < 0 || y > max_y {
            return;
        }
        let from = start.0.min(end.0).max(0);
        let to = start.0.max(end.0).min(max_x);
        if from > to {
            return;
        }
        for x in from..=to {
            put_pixel_clipped(rgba, width, height, x, y, color);
        }
    } else if start.0 == end.0 {
        let x = start.0;
        if x < 0 || x > max_x {
            return;
        }
        let from = start.1.min(end.1).max(0);
        let to = start.1.max(end.1).min(max_y);
        if from > to {
            return;
        }
        for y in from..=to {
            put_pixel_clipped(rgba, width, height, x, y, color);
        }
    }
}

fn put_pixel_clipped(rgba: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let offset = ((y as u32 * width + x as u32) * 4) as usize;
    if offset + 4 <= rgba.len() {
        rgba[offset..offset + 4].copy_from_slice(&color);
    }
}

fn rgba_color(rgb: [u8; 3]) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], 255]
}

fn average_rgb(a: [u8; 3], b: [u8; 3]) -> [u8; 3] {
    [
        ((u16::from(a[0]) + u16::from(b[0])) / 2) as u8,
        ((u16::from(a[1]) + u16::from(b[1])) / 2) as u8,
        ((u16::from(a[2]) + u16::from(b[2])) / 2) as u8,
    ]
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
        average_rgb, classify_shell_asset, draw_axis_line_inclusive_clipped, load_named_palette,
        load_parent_background_palette, render_primitive_bevel_entry, render_shp_entry, rgba_color,
        AssetManager, RenderedShellEntry, ShellAssetRole, OWNER_DRAW_FLAG_TRANSPARENT_RGB,
        PRIMITIVE_BEVEL_COLOR_A_RGB, PRIMITIVE_BEVEL_COLOR_B_RGB,
    };

    fn pixel(entry: &RenderedShellEntry, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * entry.width + x) * 4) as usize;
        [
            entry.rgba[offset],
            entry.rgba[offset + 1],
            entry.rgba[offset + 2],
            entry.rgba[offset + 3],
        ]
    }

    #[test]
    fn skirmish_shell_asset_classification_matches_live_render_path() {
        assert_eq!(
            classify_shell_asset("MNSCRNS.SHP"),
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
        assert_eq!(
            classify_shell_asset("SDMPBTN.SHP"),
            ShellAssetRole::RightPanelChrome
        );
        assert_eq!(
            classify_shell_asset("LWSCRNS.SHP"),
            ShellAssetRole::RightPanelChrome
        );
        assert_eq!(
            classify_shell_asset("LWSCRNL.SHP"),
            ShellAssetRole::RightPanelChrome
        );
        assert_ne!(
            classify_shell_asset("SDMPBTN.SHP"),
            ShellAssetRole::VerifiedOfflineStartMarker
        );
        assert_eq!(
            classify_shell_asset("MNSCRNL.SHP"),
            ShellAssetRole::ResearchCandidate
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
    fn verified_flag_pcxs_use_magenta_color_key() {
        assert_eq!(OWNER_DRAW_FLAG_TRANSPARENT_RGB, [255, 0, 255]);
    }

    #[test]
    fn primitive_bevel_line_spans_include_both_endpoints() {
        let color = [1, 2, 3, 255];
        let mut rgba = vec![0u8; 5 * 3 * 4];

        draw_axis_line_inclusive_clipped(&mut rgba, 5, 3, (1, 1), (3, 1), color);
        let entry = RenderedShellEntry {
            label: "line".to_string(),
            width: 5,
            height: 3,
            rgba,
        };

        assert_eq!(pixel(&entry, 0, 1), [0, 0, 0, 0]);
        assert_eq!(pixel(&entry, 1, 1), color);
        assert_eq!(pixel(&entry, 2, 1), color);
        assert_eq!(pixel(&entry, 3, 1), color);
        assert_eq!(pixel(&entry, 4, 1), [0, 0, 0, 0]);
    }

    #[test]
    fn primitive_bevel_line_spans_clip_to_destination_extents() {
        let color = [4, 5, 6, 255];
        let mut rgba = vec![0u8; 3 * 3 * 4];

        draw_axis_line_inclusive_clipped(&mut rgba, 3, 3, (-2, 1), (5, 1), color);
        draw_axis_line_inclusive_clipped(&mut rgba, 3, 3, (2, -3), (2, 8), color);
        let entry = RenderedShellEntry {
            label: "clip".to_string(),
            width: 3,
            height: 3,
            rgba,
        };

        assert_eq!(pixel(&entry, 0, 1), color);
        assert_eq!(pixel(&entry, 1, 1), color);
        assert_eq!(pixel(&entry, 2, 1), color);
        assert_eq!(pixel(&entry, 2, 0), color);
        assert_eq!(pixel(&entry, 2, 2), color);
        assert_eq!(pixel(&entry, 0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn primitive_bevel_border_two_swaps_outer_and_inner_ring_colors() {
        let entry = render_primitive_bevel_entry("bevel", 8, 8, [2, 2, 2, 2], 2);
        let color_a = rgba_color(PRIMITIVE_BEVEL_COLOR_A_RGB);
        let color_b = rgba_color(PRIMITIVE_BEVEL_COLOR_B_RGB);

        assert_eq!(pixel(&entry, 0, 0), color_a);
        assert_eq!(pixel(&entry, 0, 1), color_a);
        assert_eq!(pixel(&entry, 5, 1), color_b);
        assert_eq!(pixel(&entry, 2, 5), color_b);

        assert_eq!(pixel(&entry, 1, 1), color_b);
        assert_eq!(pixel(&entry, 1, 2), color_b);
        assert_eq!(pixel(&entry, 4, 2), color_a);
        assert_eq!(pixel(&entry, 2, 4), color_a);
    }

    #[test]
    fn primitive_bevel_border_two_averages_mixed_corners() {
        let entry = render_primitive_bevel_entry("bevel", 8, 8, [2, 2, 2, 2], 2);
        let mixed = rgba_color(average_rgb(
            PRIMITIVE_BEVEL_COLOR_A_RGB,
            PRIMITIVE_BEVEL_COLOR_B_RGB,
        ));

        assert_eq!(mixed, [0xA2, 0x9C, 0x87, 255]);
        assert_eq!(pixel(&entry, 5, 0), mixed);
        assert_eq!(pixel(&entry, 0, 5), mixed);
        assert_eq!(pixel(&entry, 4, 1), mixed);
        assert_eq!(pixel(&entry, 1, 4), mixed);
    }

    #[test]
    #[ignore]
    fn retail_shell_shp_dimensions_match_research() {
        let config = crate::util::config::GameConfig::load().expect("game config");
        let assets = AssetManager::new(&config.paths.ra2_dir).expect("asset manager");
        let shell_palette = load_named_palette(&assets, "SHELL.PAL").expect("SHELL.PAL");
        let anim_palette = load_named_palette(&assets, "SDBTNANM.PAL").expect("SDBTNANM.PAL");
        let sdbtn = render_shp_entry(&assets, "SDBTNANM.SHP", &anim_palette, 10)
            .expect("SDBTNANM frame 10");
        let lwscrns = render_shp_entry(&assets, "LWSCRNS.SHP", &shell_palette, 0).expect("LWSCRNS");
        let lwscrnl = render_shp_entry(&assets, "LWSCRNL.SHP", &shell_palette, 0).expect("LWSCRNL");
        assert_eq!((sdbtn.width, sdbtn.height), (156, 42));
        assert_eq!((lwscrns.width, lwscrns.height), (472, 32));
        assert_eq!((lwscrnl.width, lwscrnl.height), (632, 32));
    }

    #[test]
    #[ignore]
    fn retail_parent_backgrounds_decode_with_verified_palette() {
        let config = crate::util::config::GameConfig::load().expect("game config");
        let assets = AssetManager::new(&config.paths.ra2_dir).expect("asset manager");
        let palette = load_parent_background_palette(&assets).expect("parent palette");
        let mnscrns = render_shp_entry(&assets, "MNSCRNS.SHP", &palette, 0).expect("MNSCRNS");
        let coop = render_shp_entry(&assets, "MnScrnLCoopGameSetup.shp", &palette, 0)
            .expect("MnScrnLCoopGameSetup");
        assert_eq!((mnscrns.width, mnscrns.height), (472, 448));
        assert_eq!((coop.width, coop.height), (632, 568));
    }
}
