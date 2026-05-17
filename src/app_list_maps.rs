//! Map file discovery and loading utilities.
//!
//! Scans the RA2 directory for available maps and loads them from disk.
//! Extracted from app_init_helpers.rs for file-size limits.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::app_init::MapMenuEntry;
use crate::assets::mix_archive::MixArchive;
use crate::map::briefing::BriefingSection;
use crate::map::map_file::{self, MapFile};
use crate::map::preview::{PreviewSection, PreviewSourceBounds};
use crate::map::waypoints::{multiplayer_start_waypoints, parse_waypoints};
use crate::rules::ini_parser::IniFile;
use crate::util::config::GameConfig;

/// List available maps in the RA2 directory for the main-menu map selector.
///
/// Includes `.mmx`, `.map`, and `.mpr` files (case-insensitive), with light
/// metadata extracted from `[Basic]` when available.
pub fn list_available_maps() -> Result<Vec<MapMenuEntry>> {
    let config: GameConfig = GameConfig::load()?;
    let ra2_dir: PathBuf = config.paths.ra2_dir;
    let mut maps: Vec<MapMenuEntry> = Vec::new();
    for entry in std::fs::read_dir(&ra2_dir)? {
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

fn read_map_menu_entry_from_ini(ini: &IniFile, file_name: &str) -> MapMenuEntry {
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

fn preview_source_bounds_from_verified_source(_ini: &IniFile) -> Option<PreviewSourceBounds> {
    // Live Ghidra verifies the four source-bound fields consumed by
    // DrawStartPositions, but this plan does not yet verify that map
    // `[Map] LocalSize=` is their exact menu-preview source. Leave empty until
    // that mapping is checked against retail.
    None
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
}
