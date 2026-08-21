//! Minimap click inverse and camera-window presentation adapters.

use crate::render::batch::SpriteInstance;

use super::minimap::MinimapRenderer;
use super::minimap_projection::minimap_screen_point_to_camera_top_left;
use super::native_radar_viewport::{
    NativeRadarScreenGeometry, native_viewport_outline_instances, native_viewport_rect,
};

impl MinimapRenderer {
    /// Return true only inside the generated primary when native authority is
    /// installed. Native `0x006539D0` rejects the centered letterbox margins.
    pub fn contains_screen_point_in_rect(
        &self,
        screen_x: f32,
        screen_y: f32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
    ) -> bool {
        if let Some(surface) = self.native_radar_surface {
            return NativeRadarScreenGeometry::new(surface, [rect_x, rect_y, rect_w, rect_h])
                .screen_to_surface_pixel((screen_x, screen_y))
                .is_some();
        }
        if self.playfield_bounds.is_some() {
            return false;
        }
        screen_x >= rect_x
            && screen_x < rect_x + rect_w
            && screen_y >= rect_y
            && screen_y < rect_y + rect_h
    }

    /// Screen-space generated-primary rectangle. `None` under configured
    /// MapClass authority means fail closed; only mapless fixtures retain the
    /// full-aperture adapter.
    pub(crate) fn content_screen_rect_in_rect(
        &self,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
    ) -> Option<[f32; 4]> {
        self.native_radar_surface
            .map(|surface| {
                NativeRadarScreenGeometry::new(surface, [rect_x, rect_y, rect_w, rect_h])
                    .content_rect()
            })
            .or_else(|| self.playfield_bounds.is_none().then_some([rect_x, rect_y, rect_w, rect_h]))
    }

    /// `RadarClass::GetObjectAtRadarPixel @ 0x00656750`: screen to generated
    /// pixel, reverse tracker lookup, then exact x87 cell inverse fallback.
    pub(crate) fn native_click_cell_in_rect(
        &self,
        screen_x: f32,
        screen_y: f32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
        entities: &crate::sim::entity_store::EntityStore,
    ) -> Option<(u16, u16)> {
        let surface = self.native_radar_surface?;
        let pixel = NativeRadarScreenGeometry::new(surface, [rect_x, rect_y, rect_w, rect_h])
            .screen_to_surface_pixel((screen_x, screen_y))?;
        if let Some(stable_id) = self.radar_tracker.object_at_pixel(pixel.0, pixel.1)
            && let Some(entity) = entities.get(stable_id)
        {
            let leptons = crate::util::lepton::LEPTONS_PER_CELL_I32;
            let x = i32::from(entity.position.rx)
                .wrapping_mul(leptons)
                .wrapping_add(entity.position.sub_x.to_num::<i32>());
            let y = i32::from(entity.position.ry)
                .wrapping_mul(leptons)
                .wrapping_add(entity.position.sub_y.to_num::<i32>());
            return Some((
                crate::util::direction_tables::lepton_to_cell(x) as i16 as u16,
                crate::util::direction_tables::lepton_to_cell(y) as i16 as u16,
            ));
        }
        surface.surface_pixel_to_cell(pixel)
    }

    pub(crate) const fn has_playfield_authority(&self) -> bool {
        self.playfield_bounds.is_some()
    }

    /// Mapless/headless adapter retained only when no MapClass authority exists.
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

    /// Build the native camera-window rectangle after the generated content
    /// copy. The active side's exact sidebar text color is supplied by app
    /// composition; each edge is one generated-surface pixel thick.
    pub fn build_viewport_rect_in_rect(
        &mut self,
        camera_x: f32,
        camera_y: f32,
        center_cell: (u16, u16),
        tactical_width: i32,
        tactical_height: i32,
        rect_x: f32,
        rect_y: f32,
        rect_w: f32,
        rect_h: f32,
        sidebar_color: [f32; 3],
    ) -> Vec<SpriteInstance> {
        let Some(surface) = self.native_radar_surface else {
            // No rectangular fallback is permitted once MapClass authority is
            // configured. Mapless renderer fixtures have no native geometry.
            return Vec::new();
        };
        let rect = native_viewport_rect(surface, center_cell, tactical_width, tactical_height);
        let transition = self.viewport_state.update(rect);
        let _native_old_border_visits = transition.dirty_previous_border;
        native_viewport_outline_instances(
            (camera_x, camera_y),
            NativeRadarScreenGeometry::new(surface, [rect_x, rect_y, rect_w, rect_h]),
            transition.current,
            sidebar_color,
        )
    }
}
