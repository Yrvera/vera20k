//! Scenario-menu entry metadata shared by the shell map selector and the
//! skirmish catalog. Map-owned (F06): every field derives from map-file
//! parsing; app initialization only constructs entries.

use crate::map::briefing::BriefingSection;
use crate::map::preview::{PreviewSection, PreviewSourceBounds, PreviewStartPoint};
use crate::map::waypoints::{Waypoint, multiplayer_start_waypoints, parse_waypoints, skirmish_player_capacity};
use crate::rules::ini_parser::IniFile;

/// Lightweight metadata used by the main-menu map selector.
#[derive(Debug, Clone)]
pub struct MapMenuEntry {
    /// Actual file name/path token used to load the map later.
    pub file_name: String,
    /// Human-facing label derived from `[Basic] Name` when available.
    pub display_name: String,
    /// Optional author text from `[Basic]`.
    pub author: Option<String>,
    /// Ordered mission briefing lines from `[Briefing]`.
    pub briefing: BriefingSection,
    /// Lightweight preview metadata from `[Preview]` / `[PreviewPack]`.
    pub preview: PreviewSection,
    /// Multiplayer start waypoints 0..=7, sorted by waypoint index.
    pub multiplayer_start_waypoints: Vec<Waypoint>,
    /// Setup-shell player capacity from native waypoint counting, including
    /// the `[RandomMap] NumPlayers` / eight-player fallback path.
    pub player_capacity: i32,
    /// Verified source bounds for projecting starts onto the preview surface.
    pub preview_source_bounds: Option<PreviewSourceBounds>,
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
        player_capacity: skirmish_player_capacity(ini),
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
