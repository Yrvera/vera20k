//! Scenario-menu entry metadata shared by the shell map selector and the
//! skirmish catalog. Map-owned (F06): every field derives from map-file
//! parsing; app initialization only constructs entries.

use crate::map::briefing::BriefingSection;
use crate::map::preview::{PreviewSection, PreviewSourceBounds};
use crate::map::waypoints::Waypoint;

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
