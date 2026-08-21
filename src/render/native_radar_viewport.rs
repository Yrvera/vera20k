//! Native radar screen mapping and tactical camera-window overlay.
//!
//! This presentation-only authority composes the generated primary surface
//! established by `RebuildRadarSurfaces @ 0x00654650` with the ordinary
//! sidebar aperture. It owns both the click inverse (`0x006539D0/0x00656750`)
//! and the viewport outline (`RadarClass::Update @ 0x00656EC0`) so those paths
//! cannot drift into independent 200x200 transforms.

use crate::render::batch::SpriteInstance;
use crate::util::native_x87::{NativeF32Bits, X87Chop53, X87Value};

use super::minimap_helpers::MINIMAP_DEPTH;
use super::native_radar_surface::{
    NativeRadarSurfaceGeometry, RADAR_APERTURE_HEIGHT, RADAR_APERTURE_WIDTH,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NativeRadarRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Generated-primary placement inside one screen-space 140x108 aperture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct NativeRadarScreenGeometry {
    surface: NativeRadarSurfaceGeometry,
    aperture: [f32; 4],
}

impl NativeRadarScreenGeometry {
    pub const fn new(surface: NativeRadarSurfaceGeometry, aperture: [f32; 4]) -> Self {
        Self { surface, aperture }
    }

    pub fn content_rect(self) -> [f32; 4] {
        let scale = self.surface_scale();
        let offset = self.surface.aperture_offset();
        let size = self.surface.generated_size();
        [
            self.aperture[0] + offset.0 as f32 * scale.0,
            self.aperture[1] + offset.1 as f32 * scale.1,
            size.0 as f32 * scale.0,
            size.1 as f32 * scale.1,
        ]
    }

    /// Native screen coordinates are integral and the live surface bounds are
    /// lower-inclusive/upper-exclusive. VERA's optional UI scale is an outer
    /// adapter; rounded destination pixels are distributed monotonically over
    /// the unchanged native generated coordinate range.
    pub fn screen_to_surface_pixel(self, screen: (f32, f32)) -> Option<(i32, i32)> {
        let rect = self.content_rect();
        let left = rect[0].round() as i32;
        let top = rect[1].round() as i32;
        let width = rect[2].round() as i32;
        let height = rect[3].round() as i32;
        let x = screen.0.round() as i32;
        let y = screen.1.round() as i32;
        let dx = x.wrapping_sub(left);
        let dy = y.wrapping_sub(top);
        if (dx as u32) >= width as u32 || (dy as u32) >= height as u32 {
            return None;
        }
        let size = self.surface.generated_size();
        Some((
            dx.wrapping_mul(size.0) / width.max(1),
            dy.wrapping_mul(size.1) / height.max(1),
        ))
    }

    pub fn surface_rect_to_screen(self, rect: NativeRadarRect) -> [f32; 4] {
        let content = self.content_rect();
        let scale = self.surface_scale();
        [
            content[0] + rect.x as f32 * scale.0,
            content[1] + rect.y as f32 * scale.1,
            rect.w as f32 * scale.0,
            rect.h as f32 * scale.1,
        ]
    }

    pub fn surface_scale(self) -> (f32, f32) {
        (
            self.aperture[2] / RADAR_APERTURE_WIDTH as f32,
            self.aperture[3] / RADAR_APERTURE_HEIGHT as f32,
        )
    }
}

/// `RadarClass::Update @ 0x00656F5E..0x00657117` camera-window rectangle in
/// generated-primary coordinates. The x origin-minus-one fix lives in
/// `cell_to_surface_pixel`; sizes preserve the native x87 operation order.
pub(super) fn native_viewport_rect(
    surface: NativeRadarSurfaceGeometry,
    center_cell: (u16, u16),
    tactical_width: i32,
    tactical_height: i32,
) -> NativeRadarRect {
    let center = surface.cell_to_surface_pixel(center_cell);
    let zoom = load_f32(surface.zoom());
    let denom_x = div(load_constant(60.0), zoom);
    let denom_y = div(load_constant(30.0), zoom);
    let width_twice = add(load_i32(tactical_width), load_i32(tactical_width));
    let height_twice = add(load_i32(tactical_height), load_i32(tactical_height));
    let w = native_ftol(add(div(width_twice, denom_x), load_constant(1.0)));
    let h_float = div(height_twice, denom_y);
    let h = native_ftol(h_float);
    let mut x = center
        .0
        .wrapping_sub(native_ftol(div(load_i32(tactical_width), denom_x)));
    let mut y = center.1.wrapping_sub(native_ftol(mul(h_float, load_constant(0.5))));
    let size = surface.generated_size();

    if x < 0 {
        x = 0;
    } else if size.0 <= x.wrapping_add(w) {
        x = size.0.wrapping_sub(1).wrapping_sub(w);
    }
    if y < 0 {
        y = 0;
    } else if size.1 <= y.wrapping_add(h) {
        y = size.1.wrapping_sub(1).wrapping_sub(h);
    }
    NativeRadarRect { x, y, w, h }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeRadarViewportTransition {
    pub previous: NativeRadarRect,
    pub current: NativeRadarRect,
    pub dirty_previous_border: Vec<(i32, i32)>,
}

/// Client-local current/previous `RadarClass+0x14DC..0x14F8` lifecycle.
///
/// VERA clears and redraws the GPU target every frame, so old border pixels are
/// already output-equivalently restored. The exact native dirty visit list is
/// nevertheless retained here to pin chronology and avoid losing it if the
/// sidebar becomes a retained software surface later.
#[derive(Debug, Clone, Default)]
pub(super) struct NativeRadarViewportState {
    current: Option<NativeRadarRect>,
    previous: Option<NativeRadarRect>,
    force_redraw: bool,
}

impl NativeRadarViewportState {
    pub fn reset_for_rebuild(&mut self) {
        self.current = None;
        self.previous = None;
        self.force_redraw = true;
    }

    pub fn update(&mut self, current: NativeRadarRect) -> NativeRadarViewportTransition {
        let previous = if self.force_redraw {
            current
        } else {
            self.previous.unwrap_or(current)
        };
        let dirty_previous_border = if previous == current {
            Vec::new()
        } else {
            native_previous_border_visits(previous)
        };
        self.current = Some(current);
        // Update copies +0x14DC..+0x14E8 over +0x14EC..+0x14F8 after draw.
        self.previous = Some(current);
        self.force_redraw = false;
        NativeRadarViewportTransition {
            previous,
            current,
            dirty_previous_border,
        }
    }
}

fn native_previous_border_visits(rect: NativeRadarRect) -> Vec<(i32, i32)> {
    let mut visits = Vec::with_capacity((rect.w.wrapping_add(rect.h).wrapping_mul(2)).max(0) as usize);
    for y in 0..rect.h.max(0) {
        visits.push((rect.x, rect.y.wrapping_add(y)));
        visits.push((rect.x.wrapping_add(rect.w).wrapping_sub(1), rect.y.wrapping_add(y)));
    }
    for x in 0..rect.w.max(0) {
        visits.push((rect.x.wrapping_add(x), rect.y));
        visits.push((rect.x.wrapping_add(x), rect.y.wrapping_add(rect.h).wrapping_sub(1)));
    }
    visits
}

pub(super) fn native_viewport_outline_instances(
    camera: (f32, f32),
    screen: NativeRadarScreenGeometry,
    rect: NativeRadarRect,
    tint: [f32; 3],
) -> Vec<SpriteInstance> {
    let [x, y, w, h] = screen.surface_rect_to_screen(rect);
    let (pixel_w, pixel_h) = screen.surface_scale();
    let make = |position: [f32; 2], size: [f32; 2]| SpriteInstance {
        position,
        size,
        uv_origin: [0.0, 0.0],
        uv_size: [1.0, 1.0],
        depth: MINIMAP_DEPTH,
        tint,
        alpha: 1.0,
        ..Default::default()
    };
    vec![
        make([camera.0 + x, camera.1 + y], [w, pixel_h]),
        make(
            [camera.0 + x, camera.1 + y + h - pixel_h],
            [w, pixel_h],
        ),
        make([camera.0 + x, camera.1 + y], [pixel_w, h]),
        make(
            [camera.0 + x + w - pixel_w, camera.1 + y],
            [pixel_w, h],
        ),
    ]
}

fn load_i32(value: i32) -> X87Value {
    X87Chop53::load_i32(value)
}

fn load_constant(value: f32) -> X87Value {
    load_f32(value)
}

fn load_f32(value: f32) -> X87Value {
    X87Chop53::load_f32(NativeF32Bits::from_bits(value.to_bits())).expect("finite radar value")
}

fn add(lhs: X87Value, rhs: X87Value) -> X87Value {
    X87Chop53::add(lhs, rhs)
}

fn mul(lhs: X87Value, rhs: X87Value) -> X87Value {
    X87Chop53::mul(lhs, rhs)
}

fn div(lhs: X87Value, rhs: X87Value) -> X87Value {
    X87Chop53::div(lhs, rhs).expect("positive radar divisor")
}

fn native_ftol(value: X87Value) -> i32 {
    X87Chop53::ftol_i64(value).expect("radar viewport value fits i32") as i32
}

#[cfg(test)]
#[path = "native_radar_viewport_tests.rs"]
mod tests;
