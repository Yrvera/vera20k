//! Client-local radar-event animation and Spacebar history.
//!
//! Native `RadarClass` owns this beside its generated radar surfaces. It reads
//! `g_PlayerPtr` visibility, so it is deliberately presentation state: never
//! snapshot or world-hash authority. The production writer closed here is the
//! type-5 `EnemyObjectSensed` call at `TechnoClass::IdleAnimDispatch`
//! `0x0070DAD7`; other event producers remain on the older sim queue.

use std::time::{Duration, Instant};

use crate::rules::radar_event_config::RadarEventConfig;
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits, X87Chop53, X87Ordering};

use super::native_radar_surface::native_event_initial_radius;

const TYPE5_DEDUP_DISTANCE: i32 = 6;
const TYPE5_VISIBLE_FRAMES: u32 = 200;
const TYPE5_LIFETIME_FRAMES: u32 = 400;
const CYCLE_RING_LEN: usize = 8;
const CYCLE_RESTART: Duration = Duration::from_millis(1600);
const TYPE5_BRIGHT: [u8; 4] = [0, 255, 255, 255];
const TYPE5_DIM: [u8; 4] = [0, 128, 128, 255];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EnemySensedSource {
    pub cell: (u16, u16),
    pub radar_pixel: (i32, i32),
}

#[derive(Debug, Clone)]
pub(super) struct ClientRadarEvent {
    source: EnemySensedSource,
    created_frame: u64,
    radius: f32,
    rotation: f32,
    rotation_speed: f32,
    fade: f32,
    fade_speed: f32,
    expanding: bool,
    needs_draw: bool,
    phase_started_frame: Option<u64>,
}

impl ClientRadarEvent {
    fn new(
        source: EnemySensedSource,
        current_frame: u64,
        surface_size: (i32, i32),
        config: &RadarEventConfig,
    ) -> Self {
        let initial_radius = native_event_initial_radius(source.radar_pixel, surface_size);
        Self {
            source,
            created_frame: current_frame,
            radius: initial_radius as f32,
            rotation: f32::from_bits(0x3f49_0fdb),
            rotation_speed: config.rotation_speed,
            fade: 0.0,
            fade_speed: config.color_speed,
            expanding: true,
            needs_draw: true,
            phase_started_frame: None,
        }
    }

    fn tick(&mut self, current_frame: u64, config: &RadarEventConfig) {
        if !self.needs_draw {
            return;
        }
        if !self.expanding
            && self.phase_started_frame.is_some_and(|started| {
                current_frame.wrapping_sub(started) >= u64::from(TYPE5_VISIBLE_FRAMES)
            })
        {
            self.needs_draw = false;
        }

        let min_radius = config.min_radius as i32;
        let radius = X87Chop53::sub(
            load_f32(self.radius),
            load_native_f32(config.native_scalars.speed),
        );
        let min_radius_x87 = X87Chop53::load_i32(min_radius);
        self.radius = store_f32(if X87Chop53::compare(radius, min_radius_x87)
            == X87Ordering::Greater
        {
            radius
        } else {
            min_radius_x87
        });
        let snap_offset = native_rotation_remainder(self.rotation);
        if self.expanding {
            let radius_difference = X87Chop53::sub(load_f32(self.radius), min_radius_x87);
            let epsilon = X87Chop53::load_f64(NativeF64Bits::from_bits(
                0x3f84_7ae1_47ae_147b,
            ))
            .expect("native 0.01 is finite");
            let absolute_difference = if X87Chop53::compare(
                radius_difference,
                X87Chop53::load_i32(0),
            ) == X87Ordering::Less
            {
                X87Chop53::neg(radius_difference)
            } else {
                radius_difference
            };
            if X87Chop53::compare(absolute_difference, epsilon) != X87Ordering::Less {
                self.rotation = native_add_stored_f32(self.rotation, self.rotation_speed);
            } else if snap_offset < self.rotation_speed {
                self.rotation = native_add_stored_f32(self.rotation, snap_offset);
                self.expanding = false;
                self.phase_started_frame = Some(current_frame);
            } else {
                self.rotation = native_add_stored_f32(self.rotation, self.rotation_speed);
                self.rotation_speed = native_decelerated_rotation_speed(
                    self.rotation_speed,
                    config.native_scalars.rotation_speed,
                );
            }
        }
        let tau = X87Chop53::load_f64(NativeF64Bits::from_bits(0x4019_21fb_5444_2d18))
            .expect("native two-pi is finite");
        let rotation = load_f32(self.rotation);
        if X87Chop53::compare(rotation, tau) == X87Ordering::Greater {
            self.rotation = store_f32(X87Chop53::sub(rotation, tau));
        }
        self.tick_fade();
    }

    fn tick_fade(&mut self) {
        self.fade = native_add_stored_f32(self.fade, self.fade_speed);
        if self.fade < 0.0 && self.fade_speed < 0.0 {
            self.fade_speed = -self.fade_speed;
            self.fade = 0.0;
        } else if self.fade > 1.0 && self.fade_speed > 0.0 {
            self.fade_speed = -self.fade_speed;
            self.fade = 1.0;
        }
    }

    fn expired(&self, current_frame: u64) -> bool {
        !self.expanding
            && self.phase_started_frame.is_some_and(|started| {
                current_frame.wrapping_sub(started) >= u64::from(TYPE5_LIFETIME_FRAMES)
            })
    }

    fn corners(&self) -> [(i32, i32); 4] {
        let x = (self.radius * self.rotation.cos()) as i32;
        let y = (self.radius * self.rotation.sin()) as i32;
        let (cx, cy) = self.source.radar_pixel;
        [
            (cx.wrapping_add(x), cy.wrapping_add(y)),
            (cx.wrapping_sub(y), cy.wrapping_add(x)),
            (cx.wrapping_sub(x), cy.wrapping_sub(y)),
            (cx.wrapping_add(y), cy.wrapping_sub(x)),
        ]
    }
}

fn load_f32(value: f32) -> crate::util::native_x87::X87Value {
    X87Chop53::load_f32(NativeF32Bits::from_bits(value.to_bits()))
        .expect("radar-event state is finite")
}

fn load_native_f32(value: NativeF32Bits) -> crate::util::native_x87::X87Value {
    X87Chop53::load_f32(value).expect("radar-event rule scalar is finite")
}

fn store_f32(value: crate::util::native_x87::X87Value) -> f32 {
    f32::from_bits(
        X87Chop53::store_f32(value)
            .expect("radar-event state remains finite f32")
            .bits(),
    )
}

fn native_add_stored_f32(lhs: f32, rhs: f32) -> f32 {
    store_f32(X87Chop53::add(load_f32(lhs), load_f32(rhs)))
}

fn native_rotation_remainder(rotation: f32) -> f32 {
    // `TickRadarEvent @ 0x0065FE69..0x0065FE98`: x87 computes
    // `(angle + pi/4) - trunc((angle + pi/4) * 2/pi) * pi/2`, then stores
    // the remainder to f32 before comparing it with the f32 rotation speed.
    let quarter_turn = X87Chop53::load_f64(NativeF64Bits::from_bits(
        0x3fe9_21fb_5444_2d18,
    ))
    .expect("native pi/4 is finite");
    let two_over_pi = X87Chop53::load_f64(NativeF64Bits::from_bits(
        0x3fe4_5f30_6dc9_c883,
    ))
    .expect("native two-over-pi is finite");
    let half_turn = X87Chop53::load_f64(NativeF64Bits::from_bits(
        0x3ff9_21fb_5444_2d18,
    ))
    .expect("native pi/2 is finite");
    let shifted = X87Chop53::add(load_f32(rotation), quarter_turn);
    let turns = X87Chop53::ftol_i64(X87Chop53::mul(shifted, two_over_pi))
        .expect("radar-event rotation quotient fits i64") as i32;
    store_f32(X87Chop53::sub(
        shifted,
        X87Chop53::mul(X87Chop53::load_i32(turns), half_turn),
    ))
}

fn native_decelerated_rotation_speed(current: f32, base: NativeF32Bits) -> f32 {
    // `0x0065FF23..0x0065FF58` keeps the floor extended for the compare but
    // rounds the subtraction to the native f32 local before selecting.
    let base = load_native_f32(base);
    let floor = X87Chop53::mul(
        base,
        load_f32(f32::from_bits(0x3eaa_aaab)),
    );
    let step = X87Chop53::mul(base, load_f32(f32::from_bits(0x3ca3_d70a)));
    let decelerated = X87Chop53::load_f32(
        X87Chop53::store_f32(X87Chop53::sub(load_f32(current), step))
            .expect("decelerated rotation speed remains finite"),
    )
    .expect("stored rotation speed reloads");
    store_f32(if X87Chop53::compare(floor, decelerated) == X87Ordering::Greater {
        floor
    } else {
        decelerated
    })
}

/// Exact type-5 live array plus the independent eight-cell review ring from
/// `InitRadarEvent @ 0x0065FB80`. The live array is intentionally uncapped.
#[derive(Debug)]
pub(super) struct ClientRadarEvents {
    events: Vec<ClientRadarEvent>,
    cycle_cells: [Option<(u16, u16)>; CYCLE_RING_LEN],
    newest_ring_index: Option<usize>,
    cycle_index: Option<usize>,
    last_cycle_at: Option<Instant>,
    last_advanced_frame: Option<u64>,
    suppress_until_baseline: bool,
}

impl Default for ClientRadarEvents {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            cycle_cells: [None; CYCLE_RING_LEN],
            newest_ring_index: None,
            cycle_index: None,
            last_cycle_at: None,
            last_advanced_frame: None,
            suppress_until_baseline: true,
        }
    }
}

impl ClientRadarEvents {
    pub fn reset_for_load_or_view(&mut self) {
        *self = Self::default();
    }

    pub fn finish_baseline(&mut self) {
        self.suppress_until_baseline = false;
    }

    /// `CreateRadarEvent @ 0x0065FA70`: unique type-5 events scan the entire
    /// live array and suppress only when truncated integer Euclidean distance
    /// is less than six. Equality is accepted. Only accepted events enter the
    /// eight-cell review ring (`0x0065FC6E..0x0065FC99`).
    pub fn create_enemy_sensed(
        &mut self,
        source: EnemySensedSource,
        current_frame: u64,
        surface_size: (i32, i32),
        config: &RadarEventConfig,
    ) -> bool {
        if self.suppress_until_baseline {
            return false;
        }
        let duplicate = self.events.iter().any(|event| {
            let dx = i32::from(event.source.cell.0).wrapping_sub(i32::from(source.cell.0));
            let dy = i32::from(event.source.cell.1).wrapping_sub(i32::from(source.cell.1));
            dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy))
                < TYPE5_DEDUP_DISTANCE * TYPE5_DEDUP_DISTANCE
        });
        if duplicate {
            return false;
        }

        self.events.push(ClientRadarEvent::new(
            source,
            current_frame,
            surface_size,
            config,
        ));
        let index = self.newest_ring_index.map_or(0, |index| (index + 1) % CYCLE_RING_LEN);
        self.cycle_cells[index] = Some(source.cell);
        self.newest_ring_index = Some(index);
        self.cycle_index = Some(index);
        true
    }

    /// `RadarClass::Draw @ 0x0065336D` ticks before `RadarClass::Update` draws.
    /// A newly created event therefore receives its first state step in its
    /// creation frame. Missed presentation frames are caught up in native
    /// frame order; an event never ticks before its own creation frame.
    pub fn advance_to_frame(&mut self, current_frame: u64, config: &RadarEventConfig) {
        let first = self
            .last_advanced_frame
            .map_or(current_frame, |frame| frame.wrapping_add(1));
        if first <= current_frame {
            for frame in first..=current_frame {
                for event in &mut self.events {
                    if event.created_frame <= frame {
                        event.tick(frame, config);
                    }
                }
                self.events.retain(|event| !event.expired(frame));
            }
        }
        self.last_advanced_frame = Some(current_frame);
    }

    pub fn draw_type5(
        &self,
        rgba: &mut [u8],
        stride_width: u32,
        canvas_height: u32,
        surface_size: (i32, i32),
    ) {
        // `TickAndDrawRadarEvents @ 0x00660000` walks ascending insertion order.
        let clip_width = surface_size.0.max(0) as u32;
        let clip_height = surface_size.1.max(0) as u32;
        for event in self.events.iter().filter(|event| event.needs_draw) {
            let corners = event.corners();
            let color = blend_color(TYPE5_DIM, TYPE5_BRIGHT, event.fade);
            for edge in 0..4 {
                draw_line(
                    rgba,
                    stride_width,
                    canvas_height,
                    clip_width,
                    clip_height,
                    corners[edge],
                    corners[(edge + 1) % 4],
                    color,
                );
            }
        }
    }

    pub fn cycle_cell(&mut self, now: Instant) -> Option<(u16, u16)> {
        let newest = self.newest_ring_index?;
        let restart = self
            .last_cycle_at
            .is_none_or(|last| now.saturating_duration_since(last) >= CYCLE_RESTART);
        let index = if restart {
            newest
        } else {
            let current = self.cycle_index.unwrap_or(newest);
            let previous = current.checked_sub(1).unwrap_or(CYCLE_RING_LEN - 1);
            if self.cycle_cells[previous].is_some() {
                previous
            } else {
                newest
            }
        };
        self.cycle_index = Some(index);
        self.last_cycle_at = Some(now);
        self.cycle_cells[index]
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.events.len()
    }
}

fn blend_color(dim: [u8; 4], bright: [u8; 4], fade: f32) -> [u8; 4] {
    let fade = fade.clamp(0.0, 1.0);
    [
        (dim[0] as f32 + (bright[0] as f32 - dim[0] as f32) * fade) as u8,
        (dim[1] as f32 + (bright[1] as f32 - dim[1] as f32) * fade) as u8,
        (dim[2] as f32 + (bright[2] as f32 - dim[2] as f32) * fade) as u8,
        255,
    ]
}

fn draw_line(
    rgba: &mut [u8],
    stride_width: u32,
    canvas_height: u32,
    clip_width: u32,
    clip_height: u32,
    start: (i32, i32),
    end: (i32, i32),
    color: [u8; 4],
) {
    let (mut x0, mut y0) = start;
    let (x1, y1) = end;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        // Radar-event raster leaves the final endpoint to the next edge.
        if x0 == x1 && y0 == y1 {
            break;
        }
        if x0 >= 0
            && y0 >= 0
            && x0 < clip_width as i32
            && y0 < clip_height as i32
            && x0 < stride_width as i32
            && y0 < canvas_height as i32
        {
            let offset = ((y0 as u32 * stride_width + x0 as u32) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
        let twice = error * 2;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

#[cfg(test)]
#[path = "radar_events_tests.rs"]
mod tests;
