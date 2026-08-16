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
use crate::render::minimap::MinimapRenderer;
use crate::render::selection_overlay::SelectionOverlay;
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
    pub(crate) combat_lights: crate::app::presentation::combat_lights::CombatLightRuntime,
    pub(crate) minimap: Option<MinimapRenderer>,
    /// Animated radar chrome — plays 33-frame open/close animation when radar gained/lost.
    pub(crate) radar_anim: Option<crate::render::radar_anim::RadarAnimState>,
    /// Requested-versus-resolved atlas identity used to construct `radar_anim`.
    ///
    /// Kept beside the animation so tactical evidence never reconstructs
    /// provenance from the currently selected sidebar theme.
    pub(crate) radar_animation_source:
        Option<crate::render::sidebar_chrome::ResolvedSidebarChromeIdentity>,
    /// Content insets [left, top, right, bottom] derived from the transparent opening
    /// in radar.shp frame 0. Used to position the minimap inside the chrome housing.
    /// Unscaled pixels — multiply by `ui_scale` at use site.
    pub(crate) radar_content_insets: Option<[u32; 4]>,
    /// Whether the local player currently has operational radar (power-gated).
    pub(crate) has_radar: bool,
    /// Selection overlay renderer — highlights and drag rectangle.
    pub(crate) selection_overlay: Option<SelectionOverlay>,
    /// Authentic SHROUD.SHP sprite-based shroud edge renderer.
    /// GPU ABuffer — screen-resolution brightness texture for per-pixel shroud darkening.
    /// SHROUD.SHP brightness pixels blitted per-cell, then a full-screen multiply pass
    /// darkens the scene.
    pub(crate) shroud_buffer: Option<crate::render::shroud_buffer::ShroudBuffer>,
    /// Cell (rx, ry) -> high-bridge facts used by the tactical cursor inverse.
    pub(crate) tactical_bridge_inverse_map:
        BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>,
    /// Active map theater name (e.g., DESERT).
    pub(crate) theater_name: String,
    /// Active map theater extension (e.g., des).
    pub(crate) theater_ext: String,
    /// Target/action lines — colored lines from selected units to command destinations.
    pub(crate) target_lines: crate::app::presentation::target_lines::TargetLineState,
    /// Fire events from the current sim tick — position data for future muzzle
    /// flash rendering and projectile origin computation. Drained each frame.
    pub(crate) pending_fire_effects: Vec<crate::sim::world::SimFireEvent>,
    /// Active garrison muzzle flash animations. Short-lived one-shot entries
    /// spawned when a garrisoned building fires. Ticked each frame, removed on completion.
    pub(crate) garrison_muzzle_flashes: Vec<crate::sim::components::GarrisonMuzzleFlash>,
    /// Active non-garrison weapon muzzle flash animations spawned from weapon `Anim=`.
    /// App-owned presentation state; combat only emits the fire facts.
    pub(crate) weapon_muzzle_flashes: Vec<crate::sim::components::WeaponMuzzleFlash>,
    /// Active render-only projectile sprites spawned from non-instant weapon fire.
    pub(crate) projectile_visuals: Vec<crate::app::presentation::fire_effects::ProjectileVisual>,
    /// Active parachute animations, one per descending paradropped infantry.
    /// Polling-based lifecycle: spawned when an entity gains parachute_state
    /// in the sim, removed on landing or death. Render-only; not snapshotted.
    pub(crate) parachute_anims: Vec<crate::sim::components::ParachuteAnim>,
    /// Global elapsed time for looping terrain overlay animations.
    pub(crate) idle_anim_elapsed_ms: u32,
    /// Logic frame on which each building's slot animations were created, by
    /// entity id.
    ///
    /// gamemd gives every building animation slot its own animation object whose
    /// frame timer is based at the frame it was constructed, so two identical
    /// buildings placed at different times run out of phase with each other.
    /// Presentation-only, so it lives here rather than on the entity.
    ///
    /// DRIFT: gamemd serializes each animation object with its own timer, so a
    /// saved game restores the phases it was saved with. This map is not in the
    /// snapshot, so loading re-stamps every surviving structure at the load
    /// frame and the whole base pulses in unison again — the exact symptom the
    /// per-building phase exists to remove. Fires once per save load, and only
    /// unwinds as those buildings are replaced.
    pub(crate) building_anim_phase_base: std::collections::BTreeMap<u64, u64>,
    // -- Reusable per-frame scratch buffers (avoid allocation each frame) --
    /// Overlay instance scratch vec — cleared and refilled each frame.
    pub(crate) cached_overlay_instances: Vec<crate::render::batch::SpriteInstance>,
    /// Unit (voxel) instance scratch vec — cleared and refilled each frame.
    pub(crate) cached_unit_instances: Vec<crate::render::batch::SpriteInstance>,
    /// UnitAtlas texture-page tags aligned with `cached_unit_instances`.
    pub(crate) cached_unit_pages: Vec<usize>,
}
