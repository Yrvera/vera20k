//! Match presentation owner (F12 `MatchPresentationState`), part 1: the
//! per-match GPU atlas set and the software cursor.
//!
//! Rebuilt on every map load from `PresentationLoadAssets`; process-lifetime
//! GPU objects live in `app::renderer_state::RendererState` instead.

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
}
