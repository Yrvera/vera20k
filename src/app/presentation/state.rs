//! Match presentation owner (F12 `MatchPresentationState`): the per-match GPU
//! atlas set, the software cursor, and the map-view data the render paths read
//! each frame (terrain/overlay/waypoint/tag projections, house colors, and the
//! transient lighting view).
//!
//! Rebuilt on every map load from `PresentationLoadAssets` and the parsed map;
//! process-lifetime GPU objects live in `app::renderer_state::RendererState`.

use std::collections::{BTreeMap, HashMap};

use crate::map::cell_tags::CellTagMap;
use crate::map::houses::{HouseColorMap, HouseRoster};
use crate::map::lighting::{CellLightGrid, LightingConfig};
use crate::map::overlay::TerrainObject;
use crate::map::tags::TagMap;
use crate::map::terrain::TerrainGrid;
use crate::map::waypoints::Waypoint;
use crate::render::bridge_atlas::BridgeAtlas;
use crate::render::bridge_railing_atlas::BridgeRailingAtlas;
use crate::render::overlay_atlas::OverlayAtlas;
use crate::render::sidebar_cameo_atlas::SidebarCameoAtlas;
use crate::render::sidebar_chrome::SidebarChromeSet;
use crate::render::sprite_atlas::SpriteAtlas;
use crate::render::tile_atlas::TileAtlas;
use crate::render::unit_atlas::UnitAtlas;

pub(crate) struct MatchPresentationState {
    pub(crate) tile_atlas: Option<TileAtlas>,
    pub(crate) unit_atlas: Option<UnitAtlas>,
    /// Palette + per-house RGB ramp GPU resources for the voxel sprite shader.
    pub(crate) palette_set: Option<crate::render::palette_textures::PaletteSet>,
    pub(crate) sprite_atlas: Option<SpriteAtlas>,
    pub(crate) overlay_atlas: Option<OverlayAtlas>,
    pub(crate) bridge_atlas: Option<BridgeAtlas>,
    pub(crate) bridge_railing_atlas: Option<BridgeRailingAtlas>,
    pub(crate) sidebar_cameo_atlas: Option<SidebarCameoAtlas>,
    pub(crate) sidebar_chrome: Option<SidebarChromeSet>,
    pub(crate) software_cursor: Option<crate::app::presentation::render::SoftwareCursor>,
    pub(crate) terrain_grid: Option<TerrainGrid>,
    /// Overlay entries from map for per-frame instance generation.
    pub(crate) overlays: crate::app::presentation::overlay_index::OverlayRenderIndex,
    /// Terrain objects from map for per-frame instance generation.
    pub(crate) terrain_objects: Vec<TerrainObject>,
    pub(crate) waypoints: HashMap<u32, Waypoint>,
    pub(crate) cell_tags: CellTagMap,
    pub(crate) tags: TagMap,
    /// Overlay ID → type name mapping for atlas lookups at render time.
    pub(crate) overlay_names: BTreeMap<u8, String>,
    /// Precomputed average pixel color for each tiberium overlay (id, frame) pair,
    /// extracted from SHP frames for minimap radar display.
    pub(crate) tiberium_radar_colors: HashMap<(u8, u8), [u8; 3]>,
    /// Owner name → house color index mapping for atlas key lookups.
    pub(crate) house_color_map: HouseColorMap,
    pub(crate) house_roster: HouseRoster,
    /// Cell (rx, ry) -> map lighting bundle. Render paths look up compatibility tints per-frame.
    pub(crate) lighting_grid: CellLightGrid,
    /// Complete source list behind the visible grid. Retained so source
    /// transitions can enumerate only old/new affected areas.
    pub(crate) applied_lighting_sources: Vec<crate::map::lighting::PointLight>,
    /// Exact ScenarioClass profile behind the visible grid.
    pub(crate) applied_lighting_profile: Option<crate::map::lighting::LightingProfileUnits>,
    /// Native detail mask behind the visible grid.
    pub(crate) applied_lighting_detail_level: u32,
    /// YR LightSourceClass-style sampled records. The active grid changes only
    /// after the complete pending refresh has gathered.
    pub(crate) pending_lighting_refresh: Option<crate::map::lighting::DeferredCellLightRefresh>,
    /// Complete derived light-view fingerprint applied to `lighting_grid`.
    /// App view-state only — never serialized or hashed.
    pub(crate) last_lighting_view_fingerprint: Option<u64>,
    /// Parsed map [Lighting] config used to rebuild transient app lighting after load.
    pub(crate) map_lighting_config: LightingConfig,
}
