//! YR WaveClass beam geometry before tactical projection.

use crate::render::batch::SpriteInstance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveGeometryKind {
    Magnetic,
    NonMagnetic,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WavePoint {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveEndpointPair {
    pub front: WavePoint,
    pub back: WavePoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveGeometryInput {
    pub kind: WaveGeometryKind,
    pub wave_type: u8,
    pub a: WavePoint,
    pub b: WavePoint,
}

const ENDPOINT_SCALE: [f64; 4] = [1.0, 1.049_999_952_316_284_2, 1.049_999_952_316_284_2, 1.0];
const COMMON_OFFSETS: [[(i32, i32); 4]; 3] = [
    [(-30, -100), (-30, 100), (30, -100), (30, 100)],
    [(-34, -44), (-34, 44), (34, -44), (34, 44)],
    [(-27, -34), (-27, 34), (27, -34), (27, 34)],
];

/// YR `WaveClass::Draw_Magnetic` (0x762070) / `Draw_NonMagnetic` (0x761640).
pub fn endpoint_pair(input: WaveGeometryInput) -> WaveEndpointPair {
    let wave_type = usize::from(input.wave_type & 3);
    let scale = ENDPOINT_SCALE[wave_type];
    let inverse = 1.0 - scale;
    let mix = |left: i32, right: i32| (right as f64 * scale + left as f64 * inverse).trunc() as i32;
    let reverse_mix =
        |left: i32, right: i32| (left as f64 * scale + right as f64 * inverse).trunc() as i32;
    let mut front = WavePoint {
        x: mix(input.a.x, input.b.x),
        y: mix(input.a.y, input.b.y),
        z: mix(input.a.z, input.b.z),
    };
    let back = WavePoint {
        x: reverse_mix(input.a.x, input.b.x),
        y: reverse_mix(input.a.y, input.b.y),
        z: reverse_mix(input.a.z, input.b.z),
    };
    if wave_type == 0 || wave_type == 3 {
        front.z += 50;
    }
    WaveEndpointPair { front, back }
}

pub fn world_vertices(input: WaveGeometryInput) -> [WavePoint; 4] {
    let endpoints = endpoint_pair(input);
    let dx = f64::from(endpoints.front.x - endpoints.back.x);
    let dy = f64::from(endpoints.front.y - endpoints.back.y);
    let dz = f64::from(endpoints.front.z - endpoints.back.z);
    let horizontal_length = dx.hypot(dy);
    let z_denominator = match input.kind {
        WaveGeometryKind::Magnetic => horizontal_length.hypot(dz),
        WaveGeometryKind::NonMagnetic => horizontal_length,
    };
    let angle_magnitude = (dx / horizontal_length).clamp(-1.0, 1.0).asin();
    let angle = if endpoints.back.y > endpoints.front.y {
        -angle_magnitude
    } else {
        angle_magnitude
    };
    let (sin, cos) = angle.sin_cos();
    let offsets = local_offsets(input.kind, input.wave_type);

    offsets.map(|(offset_x, offset_y)| {
        let local_x = horizontal_length + f64::from(offset_x);
        let local_y = f64::from(offset_y);
        let local_z = local_x * dz / z_denominator + f64::from(endpoints.back.z);
        WavePoint {
            x: fistp_trunc(f64::from(endpoints.back.x) + local_x * cos - local_y * sin),
            y: fistp_trunc(f64::from(endpoints.back.y) + local_x * sin + local_y * cos),
            z: fistp_trunc(local_z),
        }
    })
}

pub fn draw_order(input: WaveGeometryInput) -> Vec<WavePoint> {
    let vertices = world_vertices(input);
    let endpoints = endpoint_pair(input);
    match input.kind {
        WaveGeometryKind::Magnetic => vec![vertices[0], vertices[1], vertices[3], vertices[2]],
        WaveGeometryKind::NonMagnetic => vec![
            vertices[0],
            endpoints.front,
            vertices[1],
            vertices[3],
            endpoints.back,
            vertices[2],
        ],
    }
}

/// Lower projected WaveClass polygon edges to the existing white-pixel batch primitive.
pub fn build_wave_instances(
    projected_points: &[[f32; 2]],
    tint: [f32; 3],
    alpha: f32,
    depth: f32,
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();
    if projected_points.len() < 2 {
        return instances;
    }
    for edge in projected_points
        .iter()
        .zip(projected_points.iter().cycle().skip(1))
        .take(projected_points.len())
    {
        emit_line(&mut instances, *edge.0, *edge.1, tint, alpha, depth);
    }
    instances
}

fn local_offsets(kind: WaveGeometryKind, wave_type: u8) -> [(i32, i32); 4] {
    let wave_type = usize::from(wave_type & 3);
    if wave_type < 3 {
        COMMON_OFFSETS[wave_type]
    } else if kind == WaveGeometryKind::Magnetic {
        [(0, -50), (0, 50), (0, -50), (0, 50)]
    } else {
        [(-30, -50), (-30, 50), (30, -50), (30, 50)]
    }
}

fn fistp_trunc(value: f64) -> i32 {
    let nearest = value.round();
    if (value - nearest).abs() < 1e-9 {
        nearest as i32
    } else {
        value.trunc() as i32
    }
}

fn emit_line(
    out: &mut Vec<SpriteInstance>,
    start: [f32; 2],
    end: [f32; 2],
    tint: [f32; 3],
    alpha: f32,
    depth: f32,
) {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let steps = dx.abs().max(dy.abs()).ceil() as usize;
    for index in 0..steps.max(1) {
        let fraction = index as f32 / steps.max(1) as f32;
        out.push(SpriteInstance {
            position: [
                (start[0] + dx * fraction).round(),
                (start[1] + dy * fraction).round(),
            ],
            size: [1.0, 1.0],
            uv_size: [1.0, 1.0],
            tint,
            alpha,
            depth,
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(kind: WaveGeometryKind, wave_type: u8) -> WaveGeometryInput {
        WaveGeometryInput {
            kind,
            wave_type,
            a: WavePoint { x: 0, y: 0, z: 0 },
            b: WavePoint { x: 100, y: 0, z: 0 },
        }
    }

    #[test]
    fn executable_endpoint_vectors() {
        assert_eq!(
            endpoint_pair(input(WaveGeometryKind::NonMagnetic, 0)),
            WaveEndpointPair {
                front: WavePoint {
                    x: 100,
                    y: 0,
                    z: 50
                },
                back: WavePoint { x: 0, y: 0, z: 0 }
            }
        );
        assert_eq!(
            endpoint_pair(input(WaveGeometryKind::NonMagnetic, 1)),
            WaveEndpointPair {
                front: WavePoint { x: 104, y: 0, z: 0 },
                back: WavePoint { x: -4, y: 0, z: 0 }
            }
        );
    }

    #[test]
    fn executable_vertex_vectors_keep_wrapper_denominator_difference() {
        assert_eq!(
            world_vertices(input(WaveGeometryKind::Magnetic, 0)),
            [
                WavePoint {
                    x: 100,
                    y: 70,
                    z: 31
                },
                WavePoint {
                    x: -100,
                    y: 70,
                    z: 31
                },
                WavePoint {
                    x: 100,
                    y: 130,
                    z: 58
                },
                WavePoint {
                    x: -100,
                    y: 130,
                    z: 58
                }
            ]
        );
        assert_eq!(
            world_vertices(input(WaveGeometryKind::NonMagnetic, 0)),
            [
                WavePoint {
                    x: 100,
                    y: 70,
                    z: 35
                },
                WavePoint {
                    x: -100,
                    y: 70,
                    z: 35
                },
                WavePoint {
                    x: 100,
                    y: 130,
                    z: 65
                },
                WavePoint {
                    x: -100,
                    y: 130,
                    z: 65
                }
            ]
        );
    }

    #[test]
    fn executable_draw_order_vectors() {
        assert_eq!(
            draw_order(input(WaveGeometryKind::Magnetic, 0)),
            vec![
                WavePoint {
                    x: 100,
                    y: 70,
                    z: 31
                },
                WavePoint {
                    x: -100,
                    y: 70,
                    z: 31
                },
                WavePoint {
                    x: -100,
                    y: 130,
                    z: 58
                },
                WavePoint {
                    x: 100,
                    y: 130,
                    z: 58
                }
            ]
        );
        assert_eq!(draw_order(input(WaveGeometryKind::NonMagnetic, 0)).len(), 6);
    }
}
