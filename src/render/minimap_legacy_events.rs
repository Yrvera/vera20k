//! Legacy renderer for sim-owned radar-event types not migrated to the
//! client-local RadarClass authority. Type 5 is explicitly excluded.

use crate::map::playfield::PlayfieldBounds;
use crate::rules::radar_event_config::RadarEventConfig;
use crate::sim::intern::InternedId;
use crate::sim::radar::{RadarEventQueue, RadarEventType};

use super::minimap_helpers::{dim_color, draw_line, world_to_minimap_pixel};
use super::radar_tracker::RadarProjectionFacts;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_legacy_sim_radar_events(
    rgba: &mut [u8],
    size: u32,
    events: &RadarEventQueue,
    config: Option<&RadarEventConfig>,
    playfield_bounds: Option<PlayfieldBounds>,
    visibility_owner: Option<InternedId>,
    projection: RadarProjectionFacts,
) {
    for event in events.iter() {
        if event.event_type == RadarEventType::EnemyObjectSensed
            || !event.event_type.draws_on_minimap()
        {
            continue;
        }
        if !playfield_bounds.is_some_and(|bounds| {
            bounds.contains_geometry_packed(i32::from(event.rx), i32::from(event.ry))
        }) {
            continue;
        }
        if let Some(ev_owner) = event.owner {
            if visibility_owner.is_some_and(|vo| vo != ev_owner) {
                continue;
            }
        }
        let (sx, sy) = crate::map::terrain::iso_to_screen(event.rx, event.ry, 0);
        let (cx, cy) = world_to_minimap_pixel(
            sx,
            sy,
            projection.world_origin_x,
            projection.world_origin_y,
            projection.world_width,
            projection.world_height,
            projection.map_offset_x,
            projection.map_offset_y,
            projection.map_pixel_w,
            projection.map_pixel_h,
        );
        let progress = event.progress();
        let min_radius = config.map_or(8.0, |c| c.min_radius);
        let start_radius = min_radius * 4.0;
        let radius = start_radius + (min_radius - start_radius) * progress;
        let color_speed = config.map_or(0.1, |c| c.color_speed);
        let pulse = 0.6 + 0.4 * (event.age_frames as f32 * color_speed).sin().abs();
        let base_color = event.event_type.color();
        let color = [
            (base_color[0] as f32 * pulse).min(255.0) as u8,
            (base_color[1] as f32 * pulse).min(255.0) as u8,
            (base_color[2] as f32 * pulse).min(255.0) as u8,
            255,
        ];
        let rotation = event.rotation_radians();
        let cos_a = rotation.cos();
        let sin_a = rotation.sin();
        let cxi = cx as i32;
        let cyi = cy as i32;
        let corners = [
            (cxi + (radius * cos_a) as i32, cyi + (radius * sin_a) as i32),
            (cxi - (radius * sin_a) as i32, cyi + (radius * cos_a) as i32),
            (cxi - (radius * cos_a) as i32, cyi - (radius * sin_a) as i32),
            (cxi + (radius * sin_a) as i32, cyi - (radius * cos_a) as i32),
        ];
        for edge in 0..4 {
            draw_line(
                rgba,
                size,
                corners[edge].0,
                corners[edge].1,
                corners[(edge + 1) % 4].0,
                corners[(edge + 1) % 4].1,
                color,
            );
        }
        let inner_radius = radius * 0.7;
        if inner_radius >= 1.0 {
            let inner = [
                (cxi + (inner_radius * cos_a) as i32, cyi + (inner_radius * sin_a) as i32),
                (cxi - (inner_radius * sin_a) as i32, cyi + (inner_radius * cos_a) as i32),
                (cxi - (inner_radius * cos_a) as i32, cyi - (inner_radius * sin_a) as i32),
                (cxi + (inner_radius * sin_a) as i32, cyi - (inner_radius * cos_a) as i32),
            ];
            let inner_color = dim_color(color, 0.5);
            for edge in 0..4 {
                draw_line(
                    rgba,
                    size,
                    inner[edge].0,
                    inner[edge].1,
                    inner[(edge + 1) % 4].0,
                    inner[(edge + 1) % 4].1,
                    inner_color,
                );
            }
        }
    }
}
