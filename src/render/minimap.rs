//! Minimap (radar view) — tiny overhead view of the entire map.
//!
//! Shows terrain as colored pixels and entity positions as colored dots
//! in the native 140x108 radar aperture.
//! A white rectangle indicates the current camera viewport.
//!
//! ## Implementation
//! - Terrain image is generated at map load and whenever mutable MapClass
//!   LocalSize authority changes.
//! - Unit dots are overlaid on demand by copying the base image and stamping dots.
//! - The combined image is only re-uploaded when sim/fog state changes.
//! - A separate 2x2 white pixel texture is used for the viewport rectangle lines.
//!
//! ## Screen-space rendering trick
//! The batch shader subtracts camera_pos from world positions. To render UI elements
//! at fixed screen positions, we add camera_pos back:
//!   `instance.position = screen_pos + camera_offset`
//! This cancels out in the shader: `clip_pos = (screen_pos + cam - cam) / screen_size`.
//!
//! ## Dependency rules
//! - Part of render/ — depends on render/batch, render/gpu, map/terrain, sim/components.
//! - Reads from sim/ via EntityStore iteration (GameEntity.position, .owner) but NEVER mutates sim state.

use crate::map::entities::EntityCategory;
use crate::map::houses::HouseColorMap;
use crate::map::playfield::PlayfieldBounds;
use crate::map::terrain::TerrainGrid;
use crate::render::batch::{BatchRenderer, BatchTexture, SpriteInstance};
use crate::render::gpu::GpuContext;
use crate::rules::house_colors::HouseColorRamps;
use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::vision::FogState;
use std::collections::BTreeMap;

use super::minimap_helpers::{
    COLOR_SHROUD, MINIMAP_DEPTH, MINIMAP_HEIGHT, MINIMAP_WIDTH, VIEWPORT_LINE_THICKNESS,
    RadarSurfacePixel, cell_visibility_color, dim_color, set_pixel, surface_visibility_color,
};
use super::radar_tracker::{
    RadarProjectionFacts, RetainedRadarTracker, radar_entity_owner_color,
    radar_pixel_candidate_eligible,
};
use super::radar_visibility::build_radar_object_update;
#[cfg(test)]
use super::radar_tracker::RadarTrackerEntry;
pub use super::minimap_helpers::{OverlayClassification, default_minimap_rect};
use super::minimap_helpers::{OverlayPixel, TerrainPixel};
use super::minimap_legacy_events::draw_legacy_sim_radar_events;
use super::minimap_projection::{
    MinimapPlayfieldProjection, aperture_pixel, generated_primary_copy_frame,
    minimap_screen_point_to_camera_top_left, rasterize_native_terrain,
};
use super::native_radar_surface::NativeRadarSurfaceGeometry;
use super::native_radar_terrain::NativeRadarTerrainSurface;
use super::radar_events::{ClientRadarEvents, EnemySensedSource};

pub(crate) type MinimapOverlayDatum = (u16, u16, OverlayClassification, u8, Option<[u8; 4]>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlayfieldAuthorityStamp {
    bounds: Option<PlayfieldBounds>,
    revision: u64,
}

fn playfield_authority_needs_reconcile(
    installed: Option<PlayfieldAuthorityStamp>,
    bounds: Option<PlayfieldBounds>,
    revision: u64,
) -> bool {
    installed != Some(PlayfieldAuthorityStamp { bounds, revision })
}

/// Minimap renderer — manages terrain image, unit overlay, and viewport rectangle.
pub struct MinimapRenderer {
    /// Base terrain image in the fixed native 140x108 radar aperture, rebuilt
    /// on a playfield-authority revision.
    base_terrain_rgba: Vec<u8>,
    /// GPU texture containing the current minimap image (terrain + unit dots).
    map_texture: BatchTexture,
    /// Raw GPU texture handle for `write_texture()` reuse (avoids per-frame alloc).
    map_texture_raw: wgpu::Texture,
    /// Reusable RGBA scratch buffer for rebuilding the minimap texture.
    rgba_scratch: Vec<u8>,
    /// Tiny 2x2 white pixel texture for drawing the viewport rectangle lines.
    white_texture: BatchTexture,
    /// Cached world bounds for coordinate mapping.
    world_origin_x: f32,
    world_origin_y: f32,
    world_width: f32,
    world_height: f32,
    terrain_pixels: Vec<TerrainPixel>,
    /// Exact generated-primary pixels, including each pixel's native inverse
    /// shroud/fog cell and packed terrain color.
    surface_pixels: Vec<RadarSurfacePixel>,
    /// Per-cell overlay metadata. Authoritative radar surfaces bake these into
    /// raw RGB before sampling; only the mapless fallback stamps them later.
    overlay_pixels: Vec<OverlayPixel>,
    /// Legacy mapless aspect-fit sub-region within the native aperture.
    map_offset_x: f32,
    map_offset_y: f32,
    map_pixel_w: f32,
    map_pixel_h: f32,
    /// Exact generated primary-surface geometry used by native radar events.
    native_radar_surface: Option<NativeRadarSurfaceGeometry>,
    native_radar_terrain: Option<NativeRadarTerrainSurface>,
    /// Last simulation tick used to refresh the texture.
    last_sim_tick: u64,
    /// Last fog generation used to refresh the texture.
    last_fog_generation: u64,
    /// Last local owner used for fog-aware refresh.
    last_visibility_owner: Option<InternedId>,
    last_radar_terrain_dirty_generation: u64,
    playfield_bounds: Option<PlayfieldBounds>,
    installed_playfield_authority: Option<PlayfieldAuthorityStamp>,
    /// Client-local +0x423/discovery cache; never snapshot/hash authority.
    radar_tracker: RetainedRadarTracker,
    /// Client-local type-5 live array and accepted-cell review ring. Native
    /// owns both beside RadarClass surfaces, never in serialized world state.
    radar_events: ClientRadarEvents,
}

impl MinimapRenderer {
    /// Create a new MinimapRenderer, generating the initial terrain image.
    ///
    /// Each terrain cell is mapped to a minimap pixel based on its normalized
    /// position within the world. Color is chosen by TMP radar colors when
    /// available, falling back to tile classification (water/land/elevated).
    /// Overlay data is pre-classified by the caller to avoid render/ depending
    /// on map/overlay_types.
    pub fn new(
        gpu: &GpuContext,
        batch: &BatchRenderer,
        grid: &TerrainGrid,
        resolved_terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
        overlay_data: &[MinimapOverlayDatum],
        theater_name: &str,
        playfield_bounds: Option<PlayfieldBounds>,
        playfield_revision: u64,
    ) -> Self {
        let pixel_count: usize = (MINIMAP_WIDTH * MINIMAP_HEIGHT * 4) as usize;
        let projection = MinimapPlayfieldProjection::derive(
            grid,
            resolved_terrain,
            overlay_data,
            theater_name,
            playfield_bounds,
        );

        let (map_texture_raw, map_texture) =
            batch.create_updatable_texture(
                gpu,
                &projection.base_rgba,
                MINIMAP_WIDTH,
                MINIMAP_HEIGHT,
            );
        let white_texture: BatchTexture = create_white_texture(gpu, batch);
        let rgba_scratch: Vec<u8> = vec![0u8; pixel_count];

        log::info!(
            "Minimap created: {}x{} px, {} terrain cells, {} overlay pixels",
            MINIMAP_WIDTH,
            MINIMAP_HEIGHT,
            projection.terrain_pixels.len(),
            projection.overlay_pixels.len(),
        );

        Self {
            base_terrain_rgba: projection.base_rgba,
            map_texture,
            map_texture_raw,
            rgba_scratch,
            white_texture,
            world_origin_x: projection.world_origin_x,
            world_origin_y: projection.world_origin_y,
            world_width: projection.world_width,
            world_height: projection.world_height,
            terrain_pixels: projection.terrain_pixels,
            surface_pixels: projection.surface_pixels,
            overlay_pixels: projection.overlay_pixels,
            map_offset_x: projection.map_offset_x,
            map_offset_y: projection.map_offset_y,
            map_pixel_w: projection.map_pixel_w,
            map_pixel_h: projection.map_pixel_h,
            native_radar_surface: projection.native_radar_surface,
            native_radar_terrain: projection.native_radar_terrain,
            last_sim_tick: u64::MAX,
            last_fog_generation: u64::MAX,
            last_visibility_owner: None,
            last_radar_terrain_dirty_generation: u64::MAX,
            playfield_bounds,
            installed_playfield_authority: Some(PlayfieldAuthorityStamp {
                bounds: playfield_bounds,
                revision: playfield_revision,
            }),
            radar_tracker: RetainedRadarTracker::default(),
            radar_events: ClientRadarEvents::default(),
        }
    }

    /// Synchronously install a changed normalized LocalSize authority and
    /// rebuild the radar surface/mapping. The revision is part of the gate so
    /// a repeated writer still performs native's full RefreshRadar path.
    pub(crate) fn reconcile_playfield(
        &mut self,
        gpu: &GpuContext,
        grid: &TerrainGrid,
        resolved_terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
        overlay_data: &[MinimapOverlayDatum],
        theater_name: &str,
        playfield_bounds: Option<PlayfieldBounds>,
        playfield_revision: u64,
    ) -> bool {
        let authority = PlayfieldAuthorityStamp {
            bounds: playfield_bounds,
            revision: playfield_revision,
        };
        if !playfield_authority_needs_reconcile(
            self.installed_playfield_authority,
            playfield_bounds,
            playfield_revision,
        ) {
            return false;
        }

        let action40_rebuild = self.installed_playfield_authority.is_some();
        let projection = MinimapPlayfieldProjection::derive(
            grid,
            resolved_terrain,
            overlay_data,
            theater_name,
            playfield_bounds,
        );
        self.base_terrain_rgba = projection.base_rgba;
        self.world_origin_x = projection.world_origin_x;
        self.world_origin_y = projection.world_origin_y;
        self.world_width = projection.world_width;
        self.world_height = projection.world_height;
        self.terrain_pixels = projection.terrain_pixels;
        self.surface_pixels = projection.surface_pixels;
        self.overlay_pixels = projection.overlay_pixels;
        self.map_offset_x = projection.map_offset_x;
        self.map_offset_y = projection.map_offset_y;
        self.map_pixel_w = projection.map_pixel_w;
        self.map_pixel_h = projection.map_pixel_h;
        self.native_radar_surface = projection.native_radar_surface;
        self.native_radar_terrain = projection.native_radar_terrain;
        self.playfield_bounds = playfield_bounds;
        self.installed_playfield_authority = Some(authority);
        self.rgba_scratch.resize(self.base_terrain_rgba.len(), 0);
        self.last_sim_tick = u64::MAX;
        self.last_fog_generation = u64::MAX;
        self.last_radar_terrain_dirty_generation = u64::MAX;
        if action40_rebuild {
            // FUN_00655990 clears +0x423; FUN_006E21E0 then forces Buildings.
            self.radar_tracker.reset_for_action40();
        } else {
            // A stale/restore reconcile is not itself an action-40 firing.
            self.radar_tracker.reset_for_load_or_view();
            self.radar_events.reset_for_load_or_view();
        }

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.map_texture_raw,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.base_terrain_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(MINIMAP_WIDTH * 4),
                rows_per_image: Some(MINIMAP_HEIGHT),
            },
            wgpu::Extent3d {
                width: MINIMAP_WIDTH,
                height: MINIMAP_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        true
    }

    pub(crate) fn needs_playfield_reconcile(
        &self,
        playfield_bounds: Option<PlayfieldBounds>,
        playfield_revision: u64,
    ) -> bool {
        playfield_authority_needs_reconcile(
            self.installed_playfield_authority,
            playfield_bounds,
            playfield_revision,
        )
    }

    /// Invalidate the dirty-gate so the next update redraws regardless of
    /// counters (F10): after a load the fog view generation restarts and the
    /// restored tick may equal the pre-load tick, so equal counters no longer
    /// prove an unchanged view.
    pub fn mark_stale(&mut self) {
        self.last_sim_tick = u64::MAX;
        self.last_fog_generation = u64::MAX;
        // The restored sim's radar-terrain dirty generation restarts too.
        // Known residual: base-terrain pixels patched by the abandoned
        // timeline (destroyed-bridge cells) are not re-derived here because
        // the restored sim's dirty list is empty; see the load-path notes.
        self.last_radar_terrain_dirty_generation = u64::MAX;
        self.installed_playfield_authority = None;
        self.radar_tracker.reset_for_load_or_view();
        self.radar_events.reset_for_load_or_view();
    }

    /// Update the minimap texture with unit dot overlays from the ECS world.
    ///
    /// Copies the base terrain image, stamps overlay pixels (ore, gems, walls,
    /// bridges, trees), then stamps a colored dot for each entity with
    /// Position + Owner, and re-uploads to the GPU. If sim tick, owner, and
    /// fog generation are unchanged, the existing texture is reused.
    pub fn update_unit_dots(
        &mut self,
        gpu: &GpuContext,
        _batch: &BatchRenderer,
        entities: &crate::sim::entity_store::EntityStore,
        logic_order: &[u64],
        houses: &BTreeMap<InternedId, crate::sim::house_state::HouseState>,
        house_colors: &HouseColorMap,
        sim_tick: u64,
        local_owner: Option<InternedId>,
        fog: &FogState,
        full_visibility: bool,
        game_mode_nonzero: bool,
        rules: Option<&RuleSet>,
        radar_events: Option<&crate::sim::radar::RadarEventQueue>,
        interner: Option<&crate::sim::intern::StringInterner>,
        bridge_state: Option<&crate::sim::bridge_state::BridgeRuntimeState>,
        resolved_terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
        radar_terrain_dirty_cells: &[(u16, u16)],
        radar_terrain_dirty_generation: u64,
    ) {
        let fog_generation = if full_visibility { 0 } else { fog.view_generation() };
        let visibility_owner = local_owner;
        if sim_tick == self.last_sim_tick
            && fog_generation == self.last_fog_generation
            && visibility_owner == self.last_visibility_owner
            && radar_terrain_dirty_generation == self.last_radar_terrain_dirty_generation
        {
            return;
        }
        if radar_terrain_dirty_generation != self.last_radar_terrain_dirty_generation {
            self.apply_bridge_terrain_dirty_cells(bridge_state, radar_terrain_dirty_cells);
        }
        if visibility_owner != self.last_visibility_owner && self.last_sim_tick != u64::MAX {
            // g_PlayerPtr controls bucket-front insertion and visibility. A
            // view-owner switch must rebuild, not reuse the other client's
            // ordered tracker.
            self.radar_tracker.reset_for_load_or_view();
            self.radar_events.reset_for_load_or_view();
        }
        self.last_sim_tick = sim_tick;
        self.last_fog_generation = fog_generation;
        self.last_visibility_owner = visibility_owner;
        self.last_radar_terrain_dirty_generation = radar_terrain_dirty_generation;

        let size: u32 = MINIMAP_WIDTH;
        {
            let rgba: &mut Vec<u8> = &mut self.rgba_scratch;

            // Fill scratch buffer: either shroud + fog-aware terrain, or base terrain copy.
            if let Some(local_owner) = local_owner.filter(|_| !full_visibility) {
                for pixel in rgba.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&COLOR_SHROUD);
                }
                let pixels: &[RadarSurfacePixel] = if self.native_radar_terrain.is_some() {
                    &self.surface_pixels
                } else {
                    // The fallback has no authoritative generated surface.
                    // It retains the legacy per-cell presentation path.
                    &[]
                };
                for terrain_pixel in pixels {
                    let color = match surface_visibility_color(local_owner, fog, terrain_pixel) {
                        Some(color) => color,
                        None => continue,
                    };
                    if let Some((x, y)) = aperture_pixel(
                        self.native_radar_surface,
                        (terrain_pixel.px, terrain_pixel.py),
                    ) {
                        set_pixel(rgba, size, x, y, color);
                    }
                }
                if self.native_radar_terrain.is_none() {
                    for terrain_pixel in &self.terrain_pixels {
                        let color = match cell_visibility_color(local_owner, fog, terrain_pixel) {
                            Some(color) => color,
                            None => continue,
                        };
                        if let Some((x, y)) =
                            aperture_pixel(None, (terrain_pixel.px, terrain_pixel.py))
                        {
                            set_pixel(rgba, size, x, y, color);
                        }
                    }
                }
            } else {
                rgba.copy_from_slice(&self.base_terrain_rgba);
            }

            // With authoritative native terrain, CellClass::GetRadarColor has
            // already applied overlay precedence in the raw RGB buffer before
            // weighted sampling. Retain the old stamp only for mapless fallback.
            for overlay in self
                .overlay_pixels
                .iter()
                .filter(|_| self.native_radar_terrain.is_none())
            {
                if let Some(local_owner) = local_owner.filter(|_| !full_visibility) {
                    if !fog.is_cell_revealed(local_owner, overlay.rx, overlay.ry) {
                        continue;
                    }
                    let mut color: [u8; 4] = overlay.color;
                    if overlay.classification == OverlayClassification::Bridge {
                        color = dim_color(color, 0.5);
                    }
                    if let Some((x, y)) =
                        aperture_pixel(self.native_radar_surface, (overlay.px, overlay.py))
                    {
                        set_pixel(rgba, size, x, y, color);
                    }
                } else {
                    let mut color = overlay.color;
                    if overlay.classification == OverlayClassification::Bridge {
                        color = dim_color(color, 0.5);
                    }
                    if let Some((x, y)) =
                        aperture_pixel(self.native_radar_surface, (overlay.px, overlay.py))
                    {
                        set_pixel(rgba, size, x, y, color);
                    }
                }
            }
        }

        // Trigger/action precedes LogicClass: action 40 clears all, forces the
        // reverse Building tail, then mobiles return on ordinary +0x4A0 visits.
        let projection = self.radar_projection_facts();
        self.radar_tracker.remove_absent_or_ineligible(entities);
        if self.radar_tracker.take_action40_building_tail_pending() {
            for stable_id in entities.keys_sorted().into_iter().rev() {
                let Some(entity) = entities.get(stable_id) else {
                    continue;
                };
                if entity.category != EntityCategory::Structure {
                    continue;
                }
                if !entity.lifecycle.object_alive || entity.lifecycle.in_limbo {
                    continue;
                }
                let update = build_radar_object_update(
                    entity,
                    houses,
                    local_owner,
                    fog,
                    full_visibility,
                    game_mode_nonzero,
                    rules,
                    interner,
                    projection,
                    self.playfield_bounds,
                    resolved_terrain,
                );
                if let Some(event) = self.radar_tracker.update_object(update, true) {
                    self.create_enemy_sensed_event(event, sim_tick, rules);
                }
            }
        }
        for &stable_id in logic_order {
            let Some(entity) = entities.get(stable_id) else {
                continue;
            };
            let update = build_radar_object_update(
                entity,
                houses,
                local_owner,
                fog,
                full_visibility,
                game_mode_nonzero,
                rules,
                interner,
                projection,
                self.playfield_bounds,
                resolved_terrain,
            );
            if let Some(event) = self.radar_tracker.update_object(update, false) {
                self.create_enemy_sensed_event(event, sim_tick, rules);
            }
        }
        self.radar_events.finish_baseline();
        let default_event_config = crate::rules::radar_event_config::RadarEventConfig::default();
        let event_config = rules
            .map(|rules| &rules.radar_event_config)
            .unwrap_or(&default_event_config);
        self.radar_events.advance_to_frame(sim_tick, event_config);

        // RenderCellPixel @ 0x00655C50 scans each ordered bucket forward and
        // the first eligible exact-coordinate object supplies the owner color.
        // Resolve the per-house ramp table once (the default empty table only
        // when rules are absent).
        let default_ramps = HouseColorRamps::default();
        let ramps: &HouseColorRamps = rules
            .map(|r| &r.house_color_ramps)
            .unwrap_or(&default_ramps);
        let projection = self.radar_projection_facts();
        let winners = self.radar_tracker.visible_winners(|entry| {
            radar_pixel_candidate_eligible(
                entry,
                entities,
                houses,
                local_owner,
                fog,
                full_visibility,
                game_mode_nonzero,
                rules,
                interner,
                projection,
            )
        });
        let rgba: &mut Vec<u8> = &mut self.rgba_scratch;
        for entry in winners {
            let Some(entity) = entities.get(entry.stable_id) else {
                continue;
            };
            let color = radar_entity_owner_color(entity, interner, house_colors, ramps);
            if entry.x >= 0 && entry.y >= 0 {
                if let Some((x, y)) = aperture_pixel(
                    self.native_radar_surface,
                    (entry.x as u32, entry.y as u32),
                ) {
                    set_pixel(rgba, size, x, y, color);
                }
            }
        }

        // Preserve not-yet-migrated event types on their historical renderer.
        if let Some(events) = radar_events {
            draw_legacy_sim_radar_events(
                rgba,
                size,
                events,
                rules.map(|rules| &rules.radar_event_config),
                self.playfield_bounds,
                visibility_owner,
                projection,
            );
        }
        // `RadarClass::Update @ 0x00656EC0` composes object pixels, then the
        // ascending radar-event array, then SpySatellite vision. Rust's SpySat
        // materialization is already present in the fog/base inputs; the type-5
        // outline must nevertheless remain after object pixels here.
        if let Some(surface) = self.native_radar_surface {
            self.radar_events.draw_type5(
                rgba,
                MINIMAP_WIDTH,
                MINIMAP_HEIGHT,
                surface.generated_size(),
                surface.aperture_offset(),
            );
        }

        // Rewrite existing GPU texture instead of creating a new one.
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.map_texture_raw,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(MINIMAP_HEIGHT),
            },
            wgpu::Extent3d {
                width: size,
                height: MINIMAP_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
    }

    fn radar_projection_facts(&self) -> RadarProjectionFacts {
        RadarProjectionFacts {
            native_surface: self.native_radar_surface,
            world_origin_x: self.world_origin_x,
            world_origin_y: self.world_origin_y,
            world_width: self.world_width,
            world_height: self.world_height,
            map_offset_x: self.map_offset_x,
            map_offset_y: self.map_offset_y,
            map_pixel_w: self.map_pixel_w,
            map_pixel_h: self.map_pixel_h,
        }
    }

    fn create_enemy_sensed_event(
        &mut self,
        event: super::radar_tracker::RadarSensedPresentationEvent,
        sim_tick: u64,
        rules: Option<&RuleSet>,
    ) {
        let Some(surface) = self.native_radar_surface else {
            return;
        };
        // `InitRadarEvent @ 0x0065FB80` consumes CellToRadarPixel's point in
        // the generated primary surface, not the Rust 200x200 texture frame.
        let pixel = surface.cell_to_surface_pixel(event.cell);
        let default_config = crate::rules::radar_event_config::RadarEventConfig::default();
        let config = rules
            .map(|rules| &rules.radar_event_config)
            .unwrap_or(&default_config);
        self.radar_events.create_enemy_sensed(
            EnemySensedSource {
                cell: event.cell,
                radar_pixel: pixel,
            },
            sim_tick,
            surface.generated_size(),
            config,
        );
    }

    pub(crate) fn cycle_enemy_sensed_event(
        &mut self,
        now: std::time::Instant,
    ) -> Option<(u16, u16)> {
        self.radar_events.cycle_cell(now)
    }

    fn apply_bridge_terrain_dirty_cells(
        &mut self,
        bridge_state: Option<&crate::sim::bridge_state::BridgeRuntimeState>,
        cells: &[(u16, u16)],
    ) {
        let bridge_color = OverlayClassification::Bridge
            .color(0)
            .map(|c| dim_color(c, 0.5));
        let mut native_changes = Vec::new();
        for &(rx, ry) in cells {
            let Some(terrain_pixel) = self
                .terrain_pixels
                .iter()
                .find(|pixel| pixel.rx == rx && pixel.ry == ry)
                .copied()
            else {
                continue;
            };
            if let Some((x, y)) = aperture_pixel(
                self.native_radar_surface,
                (terrain_pixel.px, terrain_pixel.py),
            ) {
                set_pixel(
                    &mut self.base_terrain_rgba,
                    MINIMAP_WIDTH,
                    x,
                    y,
                    terrain_pixel.color,
                );
            }

            let bridge_visible = bridge_state
                .and_then(|state| state.cell(rx, ry))
                .is_some_and(|cell| {
                    cell.deck_present
                        && crate::sim::bridge_state::BridgeRuntimeState::effective_render_state(
                            cell,
                        )
                        .is_some()
                });

            if bridge_visible {
                let Some(color) = bridge_color else { continue };
                native_changes.push(((rx, ry), Some([color[0], color[1], color[2]])));
                if let Some(existing) = self
                    .overlay_pixels
                    .iter_mut()
                    .find(|pixel| pixel.rx == rx && pixel.ry == ry)
                {
                    existing.px = terrain_pixel.px;
                    existing.py = terrain_pixel.py;
                    existing.color = color;
                    existing.classification = OverlayClassification::Bridge;
                } else {
                    self.overlay_pixels.push(OverlayPixel {
                        rx,
                        ry,
                        px: terrain_pixel.px,
                        py: terrain_pixel.py,
                        color,
                        classification: OverlayClassification::Bridge,
                    });
                }
            } else {
                native_changes.push(((rx, ry), None));
                self.overlay_pixels
                    .retain(|pixel| !(pixel.rx == rx && pixel.ry == ry));
            }
        }
        if let Some(surface) = &mut self.native_radar_terrain
            && surface.set_cell_overrides(native_changes)
        {
            let (rgba, pixels) = rasterize_native_terrain(surface);
            self.base_terrain_rgba = rgba;
            self.surface_pixels = pixels;
        }
    }

    /// Build a SpriteInstance that fills the given screen rect with the minimap.
    ///
    /// Native's already-generated primary is copied at its own dimensions and
    /// centered in the aperture; only the no-authority fallback fills the rect.
    pub fn build_minimap_instance_in_rect(
        &self,
        camera_x: f32,
        camera_y: f32,
        screen_x: f32,
        screen_y: f32,
        width: f32,
        height: f32,
    ) -> SpriteInstance {
        let (position, size, uv_origin, uv_size) = self.native_radar_surface.map_or(
            (
                [camera_x + screen_x, camera_y + screen_y],
                [width, height],
                [0.0, 0.0],
                [1.0, 1.0],
            ),
            |surface| {
                let copy = generated_primary_copy_frame(surface, width, height);
                (
                    [
                        camera_x + screen_x + copy.offset[0],
                        camera_y + screen_y + copy.offset[1],
                    ],
                    copy.size,
                    copy.uv_origin,
                    copy.uv_size,
                )
            },
        );
        SpriteInstance {
            position,
            size,
            uv_origin,
            uv_size,
            depth: MINIMAP_DEPTH,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        }
    }

    /// Return true if a screen-space cursor position is inside the minimap rectangle.
    pub fn contains_screen_point_in_rect(
        &self,
        screen_x: f32,
        screen_y: f32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
    ) -> bool {
        screen_x >= rect_x
            && screen_x <= rect_x + rect_w
            && screen_y >= rect_y
            && screen_y <= rect_y + rect_h
    }

    /// Convert a screen-space minimap click/drag point to camera top-left world position.
    pub fn camera_top_left_for_screen_point_in_rect(
        &self,
        screen_x: f32,
        screen_y: f32,
        screen_w: f32,
        screen_h: f32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
    ) -> (f32, f32) {
        minimap_screen_point_to_camera_top_left(
            screen_x,
            screen_y,
            screen_w,
            screen_h,
            rect_x,
            rect_y,
            rect_w,
            rect_h,
            self.world_origin_x,
            self.world_origin_y,
            self.world_width,
            self.world_height,
            self.map_offset_x,
            self.map_offset_y,
            self.map_pixel_w,
            self.map_pixel_h,
        )
    }

    /// Build SpriteInstances for the camera viewport rectangle on the minimap.
    pub fn build_viewport_rect_in_rect(
        &self,
        camera_x: f32,
        camera_y: f32,
        screen_w: f32,
        screen_h: f32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
    ) -> Vec<SpriteInstance> {
        let width = MINIMAP_WIDTH as f32;
        let height = MINIMAP_HEIGHT as f32;
        let nx_left = (camera_x - self.world_origin_x) / self.world_width;
        let ny_top = (camera_y - self.world_origin_y) / self.world_height;
        let nx_right = (camera_x + screen_w - self.world_origin_x) / self.world_width;
        let ny_bottom = (camera_y + screen_h - self.world_origin_y) / self.world_height;
        let left = ((nx_left * self.map_pixel_w + self.map_offset_x) / width * rect_w)
            .clamp(0.0, rect_w);
        let top = ((ny_top * self.map_pixel_h + self.map_offset_y) / height * rect_h)
            .clamp(0.0, rect_h);
        let right = ((nx_right * self.map_pixel_w + self.map_offset_x) / width * rect_w)
            .clamp(0.0, rect_w);
        let bottom = ((ny_bottom * self.map_pixel_h + self.map_offset_y) / height * rect_h)
            .clamp(0.0, rect_h);

        let vp_w: f32 = right - left;
        let vp_h: f32 = bottom - top;
        let t: f32 = VIEWPORT_LINE_THICKNESS;

        let mut lines: Vec<SpriteInstance> = Vec::with_capacity(4);

        // Top edge.
        lines.push(SpriteInstance {
            position: [camera_x + rect_x + left, camera_y + rect_y + top],
            size: [vp_w, t],
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            depth: MINIMAP_DEPTH,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });
        // Bottom edge.
        lines.push(SpriteInstance {
            position: [camera_x + rect_x + left, camera_y + rect_y + bottom - t],
            size: [vp_w, t],
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            depth: MINIMAP_DEPTH,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });
        // Left edge.
        lines.push(SpriteInstance {
            position: [camera_x + rect_x + left, camera_y + rect_y + top],
            size: [t, vp_h],
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            depth: MINIMAP_DEPTH,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });
        // Right edge.
        lines.push(SpriteInstance {
            position: [camera_x + rect_x + right - t, camera_y + rect_y + top],
            size: [t, vp_h],
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            depth: MINIMAP_DEPTH,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });

        lines
    }

    /// Get a reference to the minimap texture for drawing.
    pub fn map_texture(&self) -> &BatchTexture {
        &self.map_texture
    }

    /// Get a reference to the white texture for drawing viewport lines.
    pub fn white_texture(&self) -> &BatchTexture {
        &self.white_texture
    }
}

fn minimap_entity_in_playfield(
    playfield_authority_configured: bool,
    entity: &crate::sim::game_entity::GameEntity,
) -> bool {
    !playfield_authority_configured || entity.in_playfield
}

/// Create a 2x2 solid white texture for drawing lines and rectangles.
fn create_white_texture(gpu: &GpuContext, batch: &BatchRenderer) -> BatchTexture {
    let white_rgba: [u8; 16] = [
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    ];
    batch.create_texture(gpu, &white_rgba, 2, 2)
}

#[cfg(test)]
#[path = "minimap_tests.rs"]
mod tests;
