//! Focused tests for sidebar chrome asset authority and retail provenance.

use std::path::PathBuf;

use super::{
    ALLIED_SIDE_ARCHIVE_ORDER, SIDE_TWO_ARCHIVE_ORDER, SidebarSideRoute, build_gclock_cpu_atlas,
    is_generic_sidebar_shp_name, resolve_sidebar_asset_in_order,
};
use crate::app_sidebar_build::build_gclock_instance;
use crate::assets::asset_manager::AssetManager;
use crate::assets::mix_archive::MixArchive;
use crate::assets::mix_hash::mix_hash;
use crate::assets::pal_file::Palette;
use crate::assets::shp_file::ShpFile;
use crate::render::sidebar_chrome::SidebarTheme;
use crate::sidebar::Rect;
use crate::util::config::GameConfig;

fn retail_ra2_dir() -> PathBuf {
    std::env::var_os("RA2_DIR")
        .map(PathBuf::from)
        .or_else(|| GameConfig::load().ok().map(|config| config.paths.ra2_dir))
        .expect("set RA2_DIR or provide config.toml for the ignored retail test")
}

fn test_mix(name: &str, body: &[u8]) -> MixArchive {
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(&mix_hash(name).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(body);
    MixArchive::from_bytes(data).expect("synthetic MIX")
}

#[test]
fn side_route_prefers_md_then_base_then_neutral() {
    let md = test_mix("piece.shp", b"md");
    let base = test_mix("piece.shp", b"base");
    let neutral = test_mix("piece.shp", b"neutral");
    let resolved = resolve_sidebar_asset_in_order(
        [
            ("sidec02md.mix", &md),
            ("sidec02.mix", &base),
            ("sidenc02.mix", &neutral),
        ],
        "piece.shp",
    )
    .expect("MD winner");
    assert_eq!(resolved.archive_name, "sidec02md.mix");
    assert_eq!(resolved.bytes, b"md");

    let unrelated_md = test_mix("other.shp", b"other");
    let resolved = resolve_sidebar_asset_in_order(
        [
            ("sidec02md.mix", &unrelated_md),
            ("sidec02.mix", &base),
            ("sidenc02.mix", &neutral),
        ],
        "piece.shp",
    )
    .expect("base fallback");
    assert_eq!(resolved.archive_name, "sidec02.mix");
    assert_eq!(resolved.bytes, b"base");

    let unrelated_base = test_mix("other.shp", b"other");
    let resolved = resolve_sidebar_asset_in_order(
        [
            ("sidec02md.mix", &unrelated_md),
            ("sidec02.mix", &unrelated_base),
            ("sidenc02.mix", &neutral),
        ],
        "piece.shp",
    )
    .expect("neutral fallback");
    assert_eq!(resolved.archive_name, "sidenc02.mix");
    assert_eq!(resolved.bytes, b"neutral");
}

#[test]
fn generic_route_is_bounded_away_from_yuri_theme_art() {
    for name in [
        "SIDE1.SHP",
        "SIDE2.SHP",
        "SIDE3.SHP",
        "TAB00.SHP",
        "TAB01.SHP",
        "TAB02.SHP",
        "TAB03.SHP",
        "REPAIR.SHP",
        "SELL.SHP",
        "R-UP.SHP",
        "R-DN.SHP",
        "POWERP.SHP",
        "GCLOCK2.SHP",
    ] {
        assert!(is_generic_sidebar_shp_name(name), "{name}");
    }

    for name in [
        "RADAR.SHP",
        "RADARY.SHP",
        "BKGDLGY.SHP",
        "BKGDMDY.SHP",
        "BKGDSMY.SHP",
        "TABS.SHP",
        "POWER.SHP",
        "ADDON.SHP",
        "UNKNOWN.SHP",
    ] {
        assert!(!is_generic_sidebar_shp_name(name), "{name}");
    }
}

#[test]
fn theme_routes_match_active_side_mapping() {
    assert_eq!(
        ALLIED_SIDE_ARCHIVE_ORDER,
        &["sidec01md.mix", "sidec01.mix", "sidenc01.mix"]
    );
    assert_eq!(
        SIDE_TWO_ARCHIVE_ORDER,
        &["sidec02md.mix", "sidec02.mix", "sidenc02.mix"]
    );
}

fn resolved_shp(route: SidebarSideRoute<'_>, name: &str) -> (&'static str, ShpFile) {
    let resolved = route.resolve_generic_shp(name).expect(name);
    let shp = ShpFile::from_bytes(resolved.bytes).expect(name);
    (resolved.archive_name, shp)
}

fn atlas_cell_has_alpha(
    atlas: &super::CpuGclockAtlas,
    frame_index: usize,
    cell_width: u32,
    cell_height: u32,
) -> bool {
    let cell_x = (frame_index as u32 % 8) * cell_width;
    let cell_y = (frame_index as u32 / 8) * cell_height;
    (cell_y..cell_y + cell_height).any(|y| {
        (cell_x..cell_x + cell_width).any(|x| {
            let alpha = ((y * atlas.width + x) * 4 + 3) as usize;
            atlas.rgba.get(alpha).copied().unwrap_or_default() != 0
        })
    })
}

#[test]
#[ignore = "requires the configured stock retail RA2/YR install"]
fn retail_yuri_generic_route_uses_side_two_and_builds_production_clock() {
    let assets = AssetManager::new(&retail_ra2_dir()).expect("load retail asset stack");
    let allied = SidebarSideRoute::for_theme(&assets, SidebarTheme::Allied);
    let soviet = SidebarSideRoute::for_theme(&assets, SidebarTheme::Soviet);
    let yuri = SidebarSideRoute::for_theme(&assets, SidebarTheme::Yuri);

    let (allied_tab_source, allied_tab) = resolved_shp(allied, "TAB00.SHP");
    let (soviet_tab_source, soviet_tab) = resolved_shp(soviet, "TAB00.SHP");
    let (yuri_tab_source, yuri_tab) = resolved_shp(yuri, "TAB00.SHP");
    assert_eq!(allied_tab_source, "sidec01.mix");
    assert_eq!(soviet_tab_source, "sidec02.mix");
    assert_eq!(yuri_tab_source, "sidec02.mix");
    assert_eq!((allied_tab.width, allied_tab.height), (28, 27));
    assert_eq!((soviet_tab.width, soviet_tab.height), (32, 28));
    assert_eq!((yuri_tab.width, yuri_tab.height), (32, 28));

    for (name, dimensions) in [
        ("REPAIR.SHP", (52, 32)),
        ("SELL.SHP", (52, 32)),
        ("R-UP.SHP", (46, 27)),
        ("R-DN.SHP", (46, 27)),
        ("POWERP.SHP", (16, 2)),
    ] {
        let (source, shp) = resolved_shp(yuri, name);
        assert_eq!(source, "sidec02.mix", "{name}");
        assert_eq!((shp.width, shp.height), dimensions, "{name}");
    }

    let generic_palette_source = yuri.resolve("SIDEBAR.PAL").expect("generic palette");
    assert_eq!(generic_palette_source.archive_name, "sidec02.mix");
    let generic_palette = Palette::from_bytes(generic_palette_source.bytes).expect("SIDEBAR.PAL");

    let yuri_theme = assets.archive("sidec02md.mix").expect("Yuri theme archive");
    assert!(yuri_theme.get_by_name("RADARYURI.PAL").is_some());
    assert!(yuri_theme.get_by_name("RADARY.SHP").is_some());
    assert!(yuri_theme.get_by_name("BKGDLGY.SHP").is_some());
    assert!(!is_generic_sidebar_shp_name("RADARY.SHP"));
    assert!(!is_generic_sidebar_shp_name("BKGDLGY.SHP"));

    let atlas = build_gclock_cpu_atlas(yuri, &generic_palette).expect("Yuri GCLOCK2 atlas");
    assert_eq!(atlas.source_archive, "sidec02.mix");
    assert_eq!((atlas.width, atlas.height), (480, 336));
    assert_eq!(atlas.frames.len(), 55);
    assert!(!atlas_cell_has_alpha(&atlas, 0, 60, 48));
    assert!(atlas_cell_has_alpha(&atlas, 1, 60, 48));
    assert!(atlas_cell_has_alpha(&atlas, 54, 60, 48));

    let instance = build_gclock_instance(
        &atlas.frames,
        0.5,
        Rect {
            x: 10.0,
            y: 20.0,
            w: 60.0,
            h: 48.0,
        },
        [0.0, 0.0],
    )
    .expect("production GCLOCK instance");
    assert_eq!(instance.position, [10.0, 20.0]);
    assert_eq!(instance.size, [60.0, 48.0]);
    assert_eq!(instance.uv_origin, atlas.frames[28].uv_origin);
    assert_eq!(instance.uv_size, atlas.frames[28].uv_size);
}
