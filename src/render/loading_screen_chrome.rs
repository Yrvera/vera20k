//! Native standard Skirmish loading-screen chrome.
//!
//! Loads verified `0x00552D60` LS country art and the `PROGBARM.SHP` frame-0
//! progress source into a batch texture. Blocked marker/text layers are not
//! substituted here.

use std::collections::HashMap;

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::assets::shp_file::ShpFile;
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;

const ATLAS_PADDING: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingScreenWidth {
    W640,
    W800,
}

impl LoadingScreenWidth {
    pub fn for_render_width(width: u32) -> Self {
        if width >= 800 { Self::W800 } else { Self::W640 }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::W640 => "640",
            Self::W800 => "800",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingArtVariant {
    Yuri,
    Observer,
    Americans,
    Russians,
    Africans,
    Alliance,
    Arabs,
    Germans,
    French,
    Confederation,
    British,
}

impl LoadingArtVariant {
    pub fn from_country_name(country: &str) -> Option<Self> {
        match country.to_ascii_lowercase().as_str() {
            "yuricountry" | "yuri" => Some(Self::Yuri),
            "observer" | "obs" => Some(Self::Observer),
            "americans" | "usa" | "ustates" => Some(Self::Americans),
            "russians" | "russia" => Some(Self::Russians),
            "africans" | "libya" | "lybia" => Some(Self::Africans),
            "alliance" | "korea" => Some(Self::Alliance),
            "arabs" | "iraq" => Some(Self::Arabs),
            "germans" | "germany" => Some(Self::Germans),
            "french" | "france" => Some(Self::French),
            "confederation" | "cuba" => Some(Self::Confederation),
            "british" | "ukingdom" => Some(Self::British),
            _ => None,
        }
    }

    pub fn manifest(self) -> LoadingArtManifest {
        match self {
            Self::Yuri => LoadingArtManifest::new("yuri", "MPYLS.PAL"),
            Self::Observer => LoadingArtManifest::new("obs", "MPLSOBS.PAL"),
            Self::Americans => LoadingArtManifest::new("ustates", "MPLSU.PAL"),
            Self::Russians => LoadingArtManifest::new("russia", "MPLSR.PAL"),
            Self::Africans => LoadingArtManifest::new("libya", "MPLSL.PAL"),
            Self::Alliance => LoadingArtManifest::new("korea", "MPLSK.PAL"),
            Self::Arabs => LoadingArtManifest::new("iraq", "MPLSI.PAL"),
            Self::Germans => LoadingArtManifest::new("germany", "MPLSG.PAL"),
            Self::French => LoadingArtManifest::new("france", "MPLSF.PAL"),
            Self::Confederation => LoadingArtManifest::new("cuba", "MPLSC.PAL"),
            Self::British => LoadingArtManifest::new("ukingdom", "MPLSUK.PAL"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingArtManifest {
    pub country_token: &'static str,
    pub palette_name: &'static str,
}

impl LoadingArtManifest {
    const fn new(country_token: &'static str, palette_name: &'static str) -> Self {
        Self {
            country_token,
            palette_name,
        }
    }

    pub fn background_asset(self, width: LoadingScreenWidth) -> String {
        format!("ls{}{}.shp", width.prefix(), self.country_token)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoadingScreenEntry {
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_size: [f32; 2],
}

pub struct LoadingScreenAtlas {
    pub texture: BatchTexture,
    pub background: LoadingScreenEntry,
    pub progress_frame0: LoadingScreenEntry,
}

struct RenderedLoadingEntry {
    label: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

pub fn build_loading_screen_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    assets: &AssetManager,
    variant: LoadingArtVariant,
    width: LoadingScreenWidth,
) -> Option<LoadingScreenAtlas> {
    let manifest = variant.manifest();
    let background_name = manifest.background_asset(width);
    let background_palette = load_named_ui_palette(assets, manifest.palette_name)?;
    let progress_palette = load_named_ui_palette(assets, "MPLS.PAL")
        .or_else(|| load_named_ui_palette(assets, manifest.palette_name))?;

    let rendered = vec![
        mandatory_shp(
            assets,
            &background_name,
            &background_palette,
            0,
            manifest.palette_name,
        )?,
        mandatory_shp(assets, "PROGBARM.SHP", &progress_palette, 0, "MPLS.PAL")?,
    ];

    let (texture, packed) = pack_entries(gpu, batch, &rendered)?;
    let by_label: HashMap<String, LoadingScreenEntry> = rendered
        .iter()
        .map(|entry| entry.label.clone())
        .zip(packed)
        .collect();
    Some(LoadingScreenAtlas {
        texture,
        background: *by_label.get(&background_name.to_ascii_lowercase())?,
        progress_frame0: *by_label.get("progbarm.shp")?,
    })
}

fn load_named_ui_palette(assets: &AssetManager, name: &str) -> Option<Palette> {
    let Some(bytes) = assets.get_ref(name) else {
        log::warn!("Missing standard Skirmish loading palette {name}");
        return None;
    };
    Palette::from_bytes_gamemd_ui(bytes)
        .map_err(|err| {
            log::warn!("Could not parse standard Skirmish loading palette {name}: {err:#}");
            err
        })
        .ok()
}

fn mandatory_shp(
    assets: &AssetManager,
    file_name: &str,
    palette: &Palette,
    frame: usize,
    palette_name: &str,
) -> Option<RenderedLoadingEntry> {
    render_shp_entry(assets, file_name, palette, frame).or_else(|| {
        log::warn!(
            "Missing mandatory standard Skirmish loading asset {file_name} frame {frame} decoded with {palette_name}"
        );
        None
    })
}

fn render_shp_entry(
    assets: &AssetManager,
    file_name: &str,
    palette: &Palette,
    frame: usize,
) -> Option<RenderedLoadingEntry> {
    let Some(bytes) = assets.get_ref(file_name) else {
        log::warn!("Missing standard Skirmish loading SHP {file_name}");
        return None;
    };
    let shp = ShpFile::from_bytes(bytes)
        .map_err(|err| {
            log::warn!("Could not parse standard Skirmish loading SHP {file_name}: {err:#}");
            err
        })
        .ok()?;
    if frame >= shp.frames.len() {
        log::warn!(
            "Standard Skirmish loading SHP {file_name} missing frame {frame}; frame count {}",
            shp.frames.len()
        );
        return None;
    }
    let frame_rgba = shp
        .frame_to_rgba_ui(frame, palette)
        .map_err(|err| {
            log::warn!(
                "Could not decode standard Skirmish loading SHP {file_name} frame {frame}: {err:#}"
            );
            err
        })
        .ok()?;
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
    Some(RenderedLoadingEntry {
        label: file_name.to_ascii_lowercase(),
        width: canvas_w,
        height: canvas_h,
        rgba,
    })
}

fn pack_entries(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    entries: &[RenderedLoadingEntry],
) -> Option<(BatchTexture, Vec<LoadingScreenEntry>)> {
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
        "Standard Skirmish loading atlas: {}x{} px, {} pieces",
        atlas_width,
        atlas_height,
        entries.len()
    );

    let texture = batch.create_texture(gpu, &rgba, atlas_width, atlas_height);
    let atlas_entries = entries
        .iter()
        .zip(placements)
        .map(|(entry, (px, py))| LoadingScreenEntry {
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
    use super::{LoadingArtVariant, LoadingScreenWidth, render_shp_entry};

    #[test]
    fn loading_art_manifest_uses_verified_binary_string_table_names() {
        let cases = [
            (
                LoadingArtVariant::Yuri,
                "ls640yuri.shp",
                "ls800yuri.shp",
                "MPYLS.PAL",
            ),
            (
                LoadingArtVariant::Observer,
                "ls640obs.shp",
                "ls800obs.shp",
                "MPLSOBS.PAL",
            ),
            (
                LoadingArtVariant::Americans,
                "ls640ustates.shp",
                "ls800ustates.shp",
                "MPLSU.PAL",
            ),
            (
                LoadingArtVariant::Russians,
                "ls640russia.shp",
                "ls800russia.shp",
                "MPLSR.PAL",
            ),
            (
                LoadingArtVariant::Africans,
                "ls640libya.shp",
                "ls800libya.shp",
                "MPLSL.PAL",
            ),
            (
                LoadingArtVariant::Alliance,
                "ls640korea.shp",
                "ls800korea.shp",
                "MPLSK.PAL",
            ),
            (
                LoadingArtVariant::Arabs,
                "ls640iraq.shp",
                "ls800iraq.shp",
                "MPLSI.PAL",
            ),
            (
                LoadingArtVariant::Germans,
                "ls640germany.shp",
                "ls800germany.shp",
                "MPLSG.PAL",
            ),
            (
                LoadingArtVariant::French,
                "ls640france.shp",
                "ls800france.shp",
                "MPLSF.PAL",
            ),
            (
                LoadingArtVariant::Confederation,
                "ls640cuba.shp",
                "ls800cuba.shp",
                "MPLSC.PAL",
            ),
            (
                LoadingArtVariant::British,
                "ls640ukingdom.shp",
                "ls800ukingdom.shp",
                "MPLSUK.PAL",
            ),
        ];

        for (variant, expected_640, expected_800, expected_palette) in cases {
            let manifest = variant.manifest();
            assert_eq!(
                manifest.background_asset(LoadingScreenWidth::W640),
                expected_640
            );
            assert_eq!(
                manifest.background_asset(LoadingScreenWidth::W800),
                expected_800
            );
            assert_eq!(manifest.palette_name, expected_palette);
        }
    }

    #[test]
    fn loading_art_manifest_does_not_use_mode2_or_campaign_assets() {
        for variant in [
            LoadingArtVariant::Yuri,
            LoadingArtVariant::Observer,
            LoadingArtVariant::Americans,
            LoadingArtVariant::Russians,
            LoadingArtVariant::Africans,
            LoadingArtVariant::Alliance,
            LoadingArtVariant::Arabs,
            LoadingArtVariant::Germans,
            LoadingArtVariant::French,
            LoadingArtVariant::Confederation,
            LoadingArtVariant::British,
        ] {
            let manifest = variant.manifest();
            let name = manifest.background_asset(LoadingScreenWidth::W800);
            assert!(!name.to_ascii_lowercase().contains("pudlgbg"));
            assert_ne!(name.to_ascii_lowercase(), "spldbr.shp");
        }
    }

    #[test]
    fn loading_art_variant_resolves_rules_country_names() {
        assert_eq!(
            LoadingArtVariant::from_country_name("Americans"),
            Some(LoadingArtVariant::Americans)
        );
        assert_eq!(
            LoadingArtVariant::from_country_name("Alliance"),
            Some(LoadingArtVariant::Alliance)
        );
        assert_eq!(
            LoadingArtVariant::from_country_name("YuriCountry"),
            Some(LoadingArtVariant::Yuri)
        );
    }

    #[test]
    fn loading_art_manifest_assets_resolve_and_decode_from_configured_install_when_available() {
        let config = match crate::util::config::GameConfig::load() {
            Ok(config) => config,
            Err(_) => return,
        };
        if !config.paths.ra2_dir.exists() {
            return;
        }

        let assets = crate::assets::asset_manager::AssetManager::new(&config.paths.ra2_dir)
            .expect("configured RA2 install should load");

        assert!(
            assets.get_ref("PROGBARM.SHP").is_some(),
            "PROGBARM.SHP should resolve from configured RA2 install"
        );
        assert!(
            assets.get_ref("MPLS.PAL").is_some(),
            "MPLS.PAL should resolve from configured RA2 install"
        );

        for variant in [
            LoadingArtVariant::Yuri,
            LoadingArtVariant::Observer,
            LoadingArtVariant::Americans,
            LoadingArtVariant::Russians,
            LoadingArtVariant::Africans,
            LoadingArtVariant::Alliance,
            LoadingArtVariant::Arabs,
            LoadingArtVariant::Germans,
            LoadingArtVariant::French,
            LoadingArtVariant::Confederation,
            LoadingArtVariant::British,
        ] {
            let manifest = variant.manifest();
            let palette_bytes = assets
                .get_ref(manifest.palette_name)
                .unwrap_or_else(|| panic!("{variant:?} loading palette should resolve"));
            let palette = crate::assets::pal_file::Palette::from_bytes_gamemd_ui(palette_bytes)
                .unwrap_or_else(|err| panic!("{variant:?} loading palette should decode: {err:#}"));
            assert!(
                assets
                    .get_ref(&manifest.background_asset(LoadingScreenWidth::W800))
                    .is_some(),
                "{variant:?} 800px loading background should resolve"
            );
            let decoded = render_shp_entry(
                &assets,
                &manifest.background_asset(LoadingScreenWidth::W800),
                &palette,
                0,
            )
            .unwrap_or_else(|| panic!("{variant:?} 800px loading background should decode"));
            assert_eq!(decoded.width, 800);
            assert_eq!(decoded.height, 600);
        }
    }
}
