//! Map file discovery and loading utilities.
//!
//! Scans the RA2 directory for available maps and loads them from disk.
//! Extracted from app_init_helpers.rs for file-size limits.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::app_init::MapMenuEntry;
use crate::assets::asset_manager::AssetManager;
use crate::assets::csf_file::CsfFile;
use crate::assets::mix_archive::MixArchive;
use crate::map::briefing::BriefingSection;
use crate::map::map_file::{self, MapFile};
use crate::map::preview::{PreviewSection, PreviewSourceBounds, PreviewStartPoint};
use crate::map::waypoints::{multiplayer_start_waypoints, parse_waypoints};
use crate::rules::ini_parser::IniFile;
use crate::skirmish_scenarios::{SkirmishScenarioRecord, SkirmishScenarioSource};
use crate::util::config::GameConfig;

/// List available maps in the RA2 directory for the main-menu map selector.
///
/// Includes `.mmx`, `.map`, and `.mpr` files (case-insensitive), with light
/// metadata extracted from `[Basic]` when available.
pub fn list_available_maps() -> Result<Vec<MapMenuEntry>> {
    let config: GameConfig = GameConfig::load()?;
    let ra2_dir: PathBuf = config.paths.ra2_dir;
    let mut maps: Vec<MapMenuEntry> = Vec::new();
    for entry in std::fs::read_dir(ra2_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let ext = ext.to_ascii_lowercase();
        if matches!(ext.as_str(), "mmx" | "yro" | "map" | "mpr" | "yrm") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                maps.push(read_map_menu_entry(&path, name));
            }
        }
    }
    maps.sort_by_key(|m| m.display_name.to_ascii_lowercase());
    Ok(maps)
}

pub fn list_skirmish_scenario_records() -> Result<Vec<SkirmishScenarioRecord>> {
    list_skirmish_scenario_records_with_csf(None)
}

pub fn list_skirmish_scenario_records_with_csf(
    csf: Option<&CsfFile>,
) -> Result<Vec<SkirmishScenarioRecord>> {
    let config: GameConfig = GameConfig::load()?;
    let ra2_dir: PathBuf = config.paths.ra2_dir;
    let assets = AssetManager::new(&ra2_dir).ok();
    let mut records = Vec::new();

    if let Some(assets) = assets.as_ref() {
        if let Some(pkt) = assets
            .get_ref("MISSIONSMD.PKT")
            .and_then(|bytes| IniFile::from_bytes(bytes).ok())
        {
            append_pkt_records(
                &mut records,
                &pkt,
                SkirmishScenarioSource::MissionsMdPkt,
                csf,
                |file_name| {
                    assets
                        .get_ref(file_name)
                        .and_then(|bytes| IniFile::from_bytes(bytes).ok())
                },
            );
        }
    }

    append_loose_pkt_records(&mut records, &ra2_dir, assets.as_ref(), csf)?;
    append_loose_yro_records(&mut records, &ra2_dir, assets.as_ref(), csf)?;
    append_loose_yrm_records(&mut records, &ra2_dir)?;

    Ok(records)
}

pub fn list_loose_skirmish_scenario_records() -> Result<Vec<SkirmishScenarioRecord>> {
    let config: GameConfig = GameConfig::load()?;
    let ra2_dir: PathBuf = config.paths.ra2_dir;
    let mut records = Vec::new();
    append_loose_yro_records(&mut records, &ra2_dir, None, None)?;
    append_loose_yrm_records(&mut records, &ra2_dir)?;
    Ok(records)
}

fn append_loose_pkt_records(
    records: &mut Vec<SkirmishScenarioRecord>,
    ra2_dir: &Path,
    assets: Option<&AssetManager>,
    csf: Option<&CsfFile>,
) -> Result<()> {
    for (path, file_name) in loose_files_with_extension(ra2_dir, "pkt")? {
        let Some(pkt) = read_ini_file(&path) else {
            continue;
        };
        append_pkt_records(
            records,
            &pkt,
            SkirmishScenarioSource::LoosePkt(file_name),
            csf,
            |map_file| {
                read_map_ini_for_metadata(&ra2_dir.join(map_file)).or_else(|| {
                    assets
                        .and_then(|assets| assets.get_ref(map_file))
                        .and_then(|bytes| IniFile::from_bytes(bytes).ok())
                })
            },
        );
    }
    Ok(())
}

fn append_loose_yro_records(
    records: &mut Vec<SkirmishScenarioRecord>,
    ra2_dir: &Path,
    assets: Option<&AssetManager>,
    csf: Option<&CsfFile>,
) -> Result<()> {
    for (path, file_name) in loose_files_with_extension(ra2_dir, "yro")? {
        let archive = match MixArchive::load(&path) {
            Ok(archive) => archive,
            Err(_) => continue,
        };
        let pkt_name = Path::new(&file_name)
            .with_extension("PKT")
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string);
        let Some(pkt) = pkt_name
            .as_deref()
            .and_then(|name| archive.get_by_name(name))
            .and_then(|bytes| IniFile::from_bytes(bytes).ok())
        else {
            continue;
        };
        append_pkt_records(
            records,
            &pkt,
            SkirmishScenarioSource::LooseYro(file_name),
            csf,
            |map_file| {
                archive
                    .get_by_name(map_file)
                    .and_then(|bytes| IniFile::from_bytes(bytes).ok())
                    .or_else(|| {
                        assets
                            .and_then(|assets| assets.get_ref(map_file))
                            .and_then(|bytes| IniFile::from_bytes(bytes).ok())
                    })
                    .or_else(|| read_map_ini_for_metadata(&ra2_dir.join(map_file)))
            },
        );
    }
    Ok(())
}

fn append_loose_yrm_records(
    records: &mut Vec<SkirmishScenarioRecord>,
    ra2_dir: &Path,
) -> Result<()> {
    for (path, file_name) in loose_files_with_extension(ra2_dir, "yrm")? {
        let Some(ini) = read_map_ini_for_metadata(&path) else {
            continue;
        };
        records.push(SkirmishScenarioRecord::concrete_from_ini(
            records.len(),
            SkirmishScenarioSource::LooseYrm(file_name),
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
            &ini,
        ));
    }
    Ok(())
}

fn loose_files_with_extension(ra2_dir: &Path, extension: &str) -> Result<Vec<(PathBuf, String)>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(&ra2_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        match ext.to_ascii_lowercase().as_str() {
            ext if ext.eq_ignore_ascii_case(extension) => files.push((path, file_name)),
            _ => {}
        }
    }
    Ok(files)
}

fn append_pkt_records<F>(
    records: &mut Vec<SkirmishScenarioRecord>,
    pkt: &IniFile,
    source: SkirmishScenarioSource,
    csf: Option<&CsfFile>,
    mut map_ini: F,
) where
    F: FnMut(&str) -> Option<IniFile>,
{
    let Some(multimaps) = pkt.section("MultiMaps") else {
        return;
    };
    for map_stem in multimaps.get_values() {
        let map_stem = map_stem.trim();
        if map_stem.is_empty() {
            continue;
        }
        let file_name = format!("{map_stem}.MAP");
        let Some(map_ini) = map_ini(&file_name) else {
            continue;
        };
        let display_name = pkt_display_name(pkt, map_stem, csf)
            .unwrap_or_else(|| display_name_from_basic_or_file(&map_ini, &file_name));
        records.push(SkirmishScenarioRecord::pkt_from_ini(
            records.len(),
            source.clone(),
            &file_name,
            &map_ini,
            display_name,
        ));
    }
}

fn pkt_display_name(pkt: &IniFile, map_stem: &str, csf: Option<&CsfFile>) -> Option<String> {
    let section = pkt.section(map_stem)?;
    if let Some(value) = section
        .get("DescriptionText")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(value.to_string());
    }

    section
        .get("Description")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            csf.and_then(|csf| csf.get(value))
                .unwrap_or(value)
                .to_string()
        })
}

fn display_name_from_basic_or_file(ini: &IniFile, file_name: &str) -> String {
    crate::map::basic::parse_basic_section(ini)
        .name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| file_name.to_string())
}

fn read_ini_file(path: &Path) -> Option<IniFile> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| IniFile::from_bytes(&bytes).ok())
}

pub(crate) fn read_map_menu_entry(path: &Path, file_name: &str) -> MapMenuEntry {
    let fallback = || MapMenuEntry {
        file_name: file_name.to_string(),
        display_name: file_name.to_string(),
        author: None,
        briefing: BriefingSection::default(),
        preview: PreviewSection::default(),
        multiplayer_start_waypoints: Vec::new(),
        preview_source_bounds: None,
    };

    let ini = match read_map_ini_for_metadata(path) {
        Some(ini) => ini,
        None => return fallback(),
    };

    read_map_menu_entry_from_ini(&ini, file_name)
}

pub(crate) fn read_map_menu_entry_from_ini(ini: &IniFile, file_name: &str) -> MapMenuEntry {
    let basic = crate::map::basic::parse_basic_section(ini);
    let display_name = basic
        .name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| file_name.to_string());

    MapMenuEntry {
        file_name: file_name.to_string(),
        display_name,
        author: basic.author,
        briefing: crate::map::briefing::parse_briefing_section(&ini),
        preview: crate::map::preview::parse_preview_section(&ini),
        multiplayer_start_waypoints: multiplayer_start_waypoints(&parse_waypoints(ini)),
        preview_source_bounds: preview_source_bounds_from_verified_source(ini),
    }
}

fn preview_source_bounds_from_verified_source(ini: &IniFile) -> Option<PreviewSourceBounds> {
    let header = ini.section("Header")?;
    let origin_x = header.get_i32("StartX")?;
    let origin_y = header.get_i32("StartY")?;
    let width = header.get_i32("Width")?;
    let height = header.get_i32("Height")?;
    let count = header.get_i32("NumberStartingPoints")?;

    if width <= 0 || height <= 0 || count <= 0 || count >= 9 {
        return None;
    }

    let start_points = (1..=count)
        .map(|idx| {
            header
                .get(&format!("Waypoint{idx}"))
                .and_then(parse_preview_start_point)
                .unwrap_or(PreviewStartPoint { x: 0, y: 0 })
        })
        .collect();

    Some(PreviewSourceBounds {
        origin_x,
        origin_y,
        width: width as u32,
        height: height as u32,
        start_points,
    })
}

fn parse_preview_start_point(value: &str) -> Option<PreviewStartPoint> {
    let mut parts = value.split(',').map(str::trim);
    let x = parts.next()?.parse::<i32>().ok()?;
    let y = parts.next()?.parse::<i32>().ok()?;
    Some(PreviewStartPoint { x, y })
}

pub(crate) fn read_map_ini_for_metadata(path: &Path) -> Option<IniFile> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() >= 2 && bytes[0] == 0 && bytes[1] == 0 {
        // MIX-wrapped: pick the entry that parses as INI with a [Map] section
        // (retail map MIXes also contain a tiny [MultiMaps] description stub).
        let archive = MixArchive::load(path).ok()?;
        let mut entries = archive.entries().to_vec();
        entries.sort_by(|a, b| b.size.cmp(&a.size));
        for entry in &entries {
            let data = archive.get_by_id(entry.id)?;
            if let Ok(ini) = IniFile::from_bytes(data) {
                if ini.section("Map").is_some() {
                    return Some(ini);
                }
            }
        }
        None
    } else {
        IniFile::from_bytes(&bytes).ok()
    }
}

pub(crate) fn load_map_by_name_or_path(ra2_dir: &Path, map_name: &str) -> Result<MapFile> {
    let direct: PathBuf = PathBuf::from(map_name);
    if direct.exists() {
        return load_map_from_path(&direct);
    }

    let in_ra2: PathBuf = ra2_dir.join(map_name);
    if in_ra2.exists() {
        return load_map_from_path(&in_ra2);
    }

    for ext in ["mmx", "yro", "map", "mpr", "yrm"] {
        let candidate = ra2_dir.join(format!("{}.{}", map_name, ext));
        if candidate.exists() {
            return load_map_from_path(&candidate);
        }
    }

    Err(anyhow::anyhow!(
        "Map '{}' not found (checked cwd, RA2 dir, and .mmx/.yro/.map/.mpr/.yrm variants)",
        map_name
    ))
}

pub(crate) fn load_map_by_name_or_path_with_assets(
    ra2_dir: &Path,
    map_name: &str,
    assets: &AssetManager,
) -> Result<MapFile> {
    match load_map_by_name_or_path(ra2_dir, map_name) {
        Ok(map) => return Ok(map),
        Err(local_err) => {
            for candidate in asset_map_candidates(map_name) {
                if let Some(bytes) = assets.get_ref(&candidate) {
                    log::info!("Loaded map {candidate} from MIX assets");
                    return MapFile::from_bytes(bytes).map_err(Into::into);
                }
            }
            Err(local_err)
        }
    }
}

pub(crate) fn asset_map_candidates(map_name: &str) -> Vec<String> {
    let mut names = Vec::new();
    names.push(map_name.to_string());
    let has_extension = Path::new(map_name).extension().is_some();
    if !has_extension {
        for ext in ["mmx", "yro", "map", "mpr", "yrm"] {
            names.push(format!("{map_name}.{ext}"));
        }
    }
    names
}

pub(crate) fn load_map_from_path(path: &Path) -> Result<MapFile> {
    map_file::load_from_path(path).map_err(Into::into)
}

/// Try loading .mmx map files from a list of candidates.
pub(crate) fn try_load_mmx(ra2_dir: &Path, names: &[&str]) -> Result<MapFile> {
    for &name in names {
        let path: PathBuf = ra2_dir.join(name);
        if path.exists() {
            match map_file::load_mmx(&path) {
                Ok(mf) => {
                    log::info!("Loaded map from {}", name);
                    return Ok(mf);
                }
                Err(err) => {
                    log::warn!("Failed to load {}: {:#}", name, err);
                }
            }
        }
    }
    Err(anyhow::anyhow!(
        "No .mmx map files found in {}",
        ra2_dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_entry_exposes_sorted_multiplayer_start_waypoints() {
        let ini = IniFile::from_str(
            "[Basic]\nName=Waypoint Test\nNewINIFormat=5\n[Waypoints]\n7=120034\n0=100011\n99=55098\n3=110022\n",
        );
        let entry = read_map_menu_entry_from_ini(&ini, "test.map");
        let indices: Vec<u32> = entry
            .multiplayer_start_waypoints
            .iter()
            .map(|wp| wp.index)
            .collect();
        assert_eq!(indices, vec![0, 3, 7]);
        assert_eq!(entry.multiplayer_start_waypoints[0].rx, 11);
        assert_eq!(entry.multiplayer_start_waypoints[0].ry, 100);
        assert_eq!(entry.preview_source_bounds, None);
    }

    #[test]
    fn menu_entry_exposes_header_preview_start_bounds() {
        let ini = IniFile::from_str(
            "[Basic]\nName=Header Starts\nNewINIFormat=5\n\
             [Header]\nStartX=10\nStartY=20\nWidth=100\nHeight=80\n\
             NumberStartingPoints=2\nWaypoint1=25,30\nWaypoint2=90,70\n",
        );
        let entry = read_map_menu_entry_from_ini(&ini, "test.map");
        assert_eq!(
            entry.preview_source_bounds,
            Some(PreviewSourceBounds {
                origin_x: 10,
                origin_y: 20,
                width: 100,
                height: 80,
                start_points: vec![
                    PreviewStartPoint { x: 25, y: 30 },
                    PreviewStartPoint { x: 90, y: 70 },
                ],
            })
        );
    }

    #[test]
    fn invalid_header_start_count_disables_live_preview_markers() {
        let ini = IniFile::from_str(
            "[Header]\nStartX=0\nStartY=0\nWidth=100\nHeight=80\nNumberStartingPoints=9\n",
        );
        let entry = read_map_menu_entry_from_ini(&ini, "test.map");
        assert_eq!(entry.preview_source_bounds, None);
    }

    #[test]
    fn asset_map_candidates_adds_retail_map_extensions_for_stems() {
        assert_eq!(
            asset_map_candidates("mp01t2"),
            vec![
                "mp01t2".to_string(),
                "mp01t2.mmx".to_string(),
                "mp01t2.yro".to_string(),
                "mp01t2.map".to_string(),
                "mp01t2.mpr".to_string(),
                "mp01t2.yrm".to_string(),
            ]
        );
    }

    #[test]
    fn asset_map_candidates_keeps_explicit_map_names_exact() {
        assert_eq!(asset_map_candidates("MP01T2.MAP"), vec!["MP01T2.MAP"]);
    }

    fn encode_csf_string(s: &str) -> Vec<u8> {
        s.encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .map(|b| !b)
            .collect()
    }

    fn build_test_csf(entries: &[(&str, &str)]) -> CsfFile {
        let mut data = Vec::new();
        data.extend_from_slice(&0x4353_4620u32.to_le_bytes());
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);

        for (label, value) in entries {
            let encoded_value = encode_csf_string(value);
            data.extend_from_slice(&0x4C42_4C20u32.to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
            data.extend_from_slice(&(label.len() as u32).to_le_bytes());
            data.extend_from_slice(label.as_bytes());
            data.extend_from_slice(&0x5354_5220u32.to_le_bytes());
            data.extend_from_slice(&(value.encode_utf16().count() as u32).to_le_bytes());
            data.extend_from_slice(&encoded_value);
        }

        CsfFile::from_bytes(&data).expect("test CSF should parse")
    }

    #[test]
    fn pkt_records_preserve_multimaps_source_order_and_pkt_names() {
        let pkt = IniFile::from_str(
            "[MultiMaps]\n1=Zoo\n2=Alpha\n3=Raw\n\
             [Zoo]\nDescriptionText=Zoo Display\n\
             [Alpha]\nDescription=GUI:AlphaName\n",
        );
        let csf = build_test_csf(&[("GUI:AlphaName", "Localized Alpha")]);

        let mut records = Vec::new();
        append_pkt_records(
            &mut records,
            &pkt,
            SkirmishScenarioSource::MissionsMdPkt,
            Some(&csf),
            |file_name| match file_name {
                "Zoo.MAP" => Some(IniFile::from_str(
                    "[Basic]\nName=Basic Zoo\nGameModes=standard\n",
                )),
                "Alpha.MAP" => Some(IniFile::from_str(
                    "[Basic]\nName=Basic Alpha\nGameModes=standard\n",
                )),
                "Raw.MAP" => Some(IniFile::from_str(
                    "[Basic]\nName=Basic Raw\nGameModes=standard\n",
                )),
                _ => None,
            },
        );

        let names: Vec<&str> = records
            .iter()
            .map(|record| record.display_name.as_str())
            .collect();
        assert_eq!(names, vec!["Zoo Display", "Localized Alpha", "Basic Raw"]);
        assert_eq!(records[0].file_name, "Zoo.MAP");
        assert_eq!(records[1].file_name, "Alpha.MAP");
        assert_eq!(records[2].file_name, "Raw.MAP");
    }
}
