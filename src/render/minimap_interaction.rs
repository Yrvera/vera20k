//! Minimap click inverse and camera-window presentation adapters.

use crate::render::batch::SpriteInstance;

use super::minimap::MinimapRenderer;
use super::minimap_projection::minimap_screen_point_to_camera_top_left;
use super::native_radar_viewport::{
    NativeRadarScreenGeometry, native_viewport_outline_instances, native_viewport_rect,
};

/// The signed-word clamp in the live RadarClass input owner
/// `0x00653D7A..0x00653E66`.
///
/// The four corrections deliberately are not collapsed to `clamp`: native
/// adjusts both packed cell words by the complete delta, in this exact order,
/// and each write wraps as i16 before the next signed comparison.
fn clamp_native_radar_click_cell(
    cell: (i16, i16),
    map_size: (i32, i32),
    tactical_size: (i32, i32),
) -> (i16, i16) {
    let horizontal_cells = tactical_size.0 / 60;
    let horizontal_half = horizontal_cells.wrapping_add(2) / 2;
    let difference_limit = map_size
        .0
        .wrapping_sub(horizontal_half)
        .wrapping_sub(1);
    let vertical_cells = tactical_size.1 / 60;
    let minimum_sum = vertical_cells
        .wrapping_add(map_size.0)
        .wrapping_add(1);
    let maximum_sum = map_size
        .1
        .wrapping_mul(2)
        .wrapping_sub(vertical_cells)
        .wrapping_add(map_size.0)
        .wrapping_sub(1);
    let (mut x, mut y) = cell;

    let y_minus_x = i32::from(y).wrapping_sub(i32::from(x));
    if y_minus_x > difference_limit {
        let delta = y_minus_x.wrapping_sub(difference_limit) as i16;
        y = y.wrapping_sub(delta);
        x = x.wrapping_add(delta);
    }

    let x_minus_y = i32::from(x).wrapping_sub(i32::from(y));
    if x_minus_y > difference_limit.wrapping_sub(1) {
        let delta = x_minus_y
            .wrapping_sub(difference_limit)
            .wrapping_add(1) as i16;
        y = y.wrapping_add(delta);
        x = x.wrapping_sub(delta);
    }

    let sum = i32::from(x).wrapping_add(i32::from(y));
    if sum < minimum_sum {
        let delta = minimum_sum
            .wrapping_sub(i32::from(y))
            .wrapping_sub(i32::from(x)) as i16;
        x = x.wrapping_add(delta);
        y = y.wrapping_add(delta);
    }

    let sum = i32::from(x).wrapping_add(i32::from(y));
    if sum > maximum_sum {
        let delta = sum.wrapping_sub(maximum_sum) as i16;
        x = x.wrapping_sub(delta);
        y = y.wrapping_sub(delta);
    }
    (x, y)
}

fn signed_tracker_or_inverse_cell(
    surface: super::native_radar_surface::NativeRadarSurfaceGeometry,
    pixel: (i32, i32),
    tracker_cell: Option<(i16, i16)>,
) -> (i16, i16) {
    tracker_cell.unwrap_or_else(|| surface.surface_pixel_to_signed_cell(pixel))
}

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
        map_size: (i32, i32),
        tactical_size: (i32, i32),
    ) -> Option<(u16, u16)> {
        let surface = self.native_radar_surface?;
        let pixel = NativeRadarScreenGeometry::new(surface, [rect_x, rect_y, rect_w, rect_h])
            .screen_to_surface_pixel((screen_x, screen_y))?;
        let tracker_cell = if let Some(stable_id) =
            self.radar_tracker.object_at_pixel(pixel.0, pixel.1)
            && let Some(entity) = entities.get(stable_id)
        {
            let leptons = crate::util::lepton::LEPTONS_PER_CELL_I32;
            let x = i32::from(entity.position.rx)
                .wrapping_mul(leptons)
                .wrapping_add(entity.position.sub_x.to_num::<i32>());
            let y = i32::from(entity.position.ry)
                .wrapping_mul(leptons)
                .wrapping_add(entity.position.sub_y.to_num::<i32>());
            Some((
                crate::util::direction_tables::lepton_to_cell(x) as i16,
                crate::util::direction_tables::lepton_to_cell(y) as i16,
            ))
        } else {
            None
        };
        let signed = signed_tracker_or_inverse_cell(surface, pixel, tracker_cell);
        let clamped = clamp_native_radar_click_cell(signed, map_size, tactical_size);
        Some((clamped.0 as u16, clamped.1 as u16))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::native_radar_surface::NativeRadarSurfaceGeometry;

    #[test]
    fn native_click_clamp_preserves_signed_words_and_strict_branch_equality() {
        let map = (100, 100);
        let tactical = (632, 570);
        assert_eq!(clamp_native_radar_click_cell((50, 143), map, tactical), (50, 143));
        assert_eq!(clamp_native_radar_click_cell((50, 144), map, tactical), (51, 143));
        assert_eq!(clamp_native_radar_click_cell((142, 50), map, tactical), (142, 50));
        assert_eq!(clamp_native_radar_click_cell((143, 50), map, tactical), (142, 51));
        assert_eq!(clamp_native_radar_click_cell((55, 55), map, tactical), (55, 55));
        assert_eq!(clamp_native_radar_click_cell((54, 55), map, tactical), (55, 56));
        assert_eq!(clamp_native_radar_click_cell((145, 145), map, tactical), (145, 145));
        assert_eq!(clamp_native_radar_click_cell((145, 146), map, tactical), (144, 145));
    }

    #[test]
    fn native_click_tracker_miss_clamps_negative_inverse_before_unsigned_pack() {
        let surface = NativeRadarSurfaceGeometry::from_raw_rect(30, 20, 300, 180).unwrap();
        let signed = signed_tracker_or_inverse_cell(surface, (0, 0), None);
        assert_eq!(signed, (-4, 25));
        assert_eq!(clamp_native_radar_click_cell(signed, (100, 100), (632, 570)), (85, 114));
    }

    #[test]
    fn native_scaled_screen_adapter_keeps_signed_inverse_until_map_clamp() {
        let surface = NativeRadarSurfaceGeometry::from_raw_rect(30, 20, 300, 180).unwrap();
        let screen = NativeRadarScreenGeometry::new(surface, [100.0, 50.0, 280.0, 216.0]);
        let pixel = screen.screen_to_surface_pixel((100.0, 74.0)).unwrap();
        assert_eq!(pixel, (0, 0));
        let signed = signed_tracker_or_inverse_cell(surface, pixel, None);
        assert_eq!(signed, (-4, 25));
        assert_eq!(clamp_native_radar_click_cell(signed, (100, 100), (632, 570)), (85, 114));
    }

    #[test]
    fn native_click_wide_and_tall_corners_and_edges_clamp_before_packing() {
        let fixtures = [
            (
                NativeRadarSurfaceGeometry::from_raw_rect(30, 20, 300, 180).unwrap(),
                [
                    (0, 0),
                    (139, 0),
                    (0, 82),
                    (139, 82),
                    (70, 0),
                    (70, 82),
                    (0, 41),
                    (139, 41),
                ],
                [
                    ((-4, 25), (85, 114)),
                    ((144, -123), (58, 141)),
                    ((83, 113), (83, 113)),
                    ((232, -35), (57, 140)),
                    ((70, -49), (132, 67)),
                    ((158, 38), (130, 66)),
                    ((39, 69), (41, 71)),
                    ((188, -79), (14, 97)),
                ],
            ),
            (
                NativeRadarSurfaceGeometry::from_raw_rect(30, 20, 180, 300).unwrap(),
                [
                    (0, 0),
                    (63, 0),
                    (0, 107),
                    (63, 107),
                    (32, 0),
                    (32, 107),
                    (0, 54),
                    (63, 54),
                ],
                [
                    ((-4, 25), (85, 114)),
                    ((83, -62), (119, 80)),
                    ((144, 174), (116, 146)),
                    ((231, 86), (151, 112)),
                    ((39, -18), (128, 71)),
                    ((188, 129), (161, 102)),
                    ((70, 100), (70, 100)),
                    ((158, 12), (104, 66)),
                ],
            ),
        ];
        for (surface, pixels, expected) in fixtures {
            let cells = pixels.map(|pixel| {
                let signed = signed_tracker_or_inverse_cell(surface, pixel, None);
                (
                    signed,
                    clamp_native_radar_click_cell(signed, (100, 100), (632, 570)),
                )
            });
            assert_eq!(cells, expected);
        }
    }
}
