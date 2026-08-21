use super::*;
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits, X87Chop53, X87Ordering};

#[test]
fn fill_terrain_colors_preserves_two_halves_and_clipped_edges() {
    let mut raw = vec![[0; 3]; 5];
    fill_raw_cell(&mut raw, (5, 1), (-1, 0), [1, 2, 3], [11, 12, 13]);
    fill_raw_cell(&mut raw, (5, 1), (1, 0), [21, 22, 23], [31, 32, 33]);
    fill_raw_cell(&mut raw, (5, 1), (4, 0), [41, 42, 43], [51, 52, 53]);
    assert_eq!(
        raw,
        vec![
            [11, 12, 13],
            [21, 22, 23],
            [31, 32, 33],
            [0, 0, 0],
            [41, 42, 43],
        ]
    );
}

#[test]
fn width_constrained_surface_matches_native_weighted_sampler_every_pixel() {
    let geometry = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 173, 113).unwrap();
    assert_eq!(geometry.generated_size(), (140, 91));
    assert_surface_matches_reference((173, 113), geometry.generated_size());
}

#[test]
fn height_constrained_surface_matches_native_weighted_sampler_every_pixel() {
    let geometry = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 137, 149).unwrap();
    assert_eq!(geometry.generated_size(), (99, 108));
    assert_surface_matches_reference((137, 149), geometry.generated_size());
}

#[test]
fn rgb565_pack_and_expand_match_local_active_directdraw_format() {
    assert_eq!(pack_rgb565(255, 255, 255), 0xffff);
    assert_eq!(pack_rgb565(255, 0, 0), 0xf800);
    assert_eq!(pack_rgb565(0, 255, 0), 0x07e0);
    assert_eq!(pack_rgb565(0, 0, 255), 0x001f);
    assert_eq!(unpack_rgb565(pack_rgb565(77, 88, 99)), [72, 88, 96, 255]);
}

#[test]
fn overlay_color_replaces_both_halves_before_weighted_generation() {
    let geometry = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 2, 1).unwrap();
    let surface = NativeRadarTerrainSurface::new(
        geometry,
        vec![NativeRadarCellColors {
            cell: (0, 0),
            left: [10, 20, 30],
            right: [40, 50, 60],
        }],
        BTreeMap::from([((0, 0), [77, 88, 99])]),
    );
    assert_eq!(surface.raw_rgb, vec![[77, 88, 99], [77, 88, 99]]);
    assert!(
        surface
            .generated_rgb565()
            .iter()
            .all(|&pixel| pixel == pack_rgb565(77, 88, 99))
    );
}

fn assert_surface_matches_reference(raw_size: (i32, i32), generated_size: (i32, i32)) {
    let raw = fixture(raw_size);
    let actual = generate_rgb565(&raw, raw_size, generated_size);
    let expected = disassembly_reference(&raw, raw_size, generated_size);
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            actual,
            expected,
            "generated pixel ({},{}) differs",
            index as i32 % generated_size.0,
            index as i32 / generated_size.0,
        );
    }
}

fn fixture(size: (i32, i32)) -> Vec<[u8; 3]> {
    (0..size.1)
        .flat_map(|y| {
            (0..size.0).map(move |x| {
                [
                    x.wrapping_mul(17).wrapping_add(y.wrapping_mul(3)) as u8,
                    x.wrapping_mul(5).wrapping_add(y.wrapping_mul(29)) as u8,
                    x.wrapping_mul(41).wrapping_add(y.wrapping_mul(7)) as u8,
                ]
            })
        })
        .collect()
}

/// Independent rectangular-overlap translation of the instruction ranges
/// `0x00654BE3..0x00654CD2` and `0x00654D29..0x00654DBC`. Production follows
/// the native first/interior/last branches; this oracle intersects the source
/// rectangles directly while retaining every observed f32/x87 store boundary.
fn disassembly_reference(
    raw: &[[u8; 3]],
    raw_size: (i32, i32),
    generated_size: (i32, i32),
) -> Vec<u16> {
    let load = |value: f32| X87Chop53::load_f32(NativeF32Bits::from_bits(value.to_bits())).unwrap();
    let store = |value| f32::from_bits(X87Chop53::store_f32(value).unwrap().bits());
    let ftol = |value| X87Chop53::ftol_i64(value).unwrap() as i32;
    let x_step = store(
        X87Chop53::div(
            X87Chop53::load_i32(raw_size.0),
            X87Chop53::load_i32(generated_size.0),
        )
        .unwrap(),
    );
    let y_step_extended = X87Chop53::div(
        X87Chop53::load_i32(raw_size.1),
        X87Chop53::load_i32(generated_size.1),
    )
    .unwrap();
    let y_step = store(y_step_extended);
    let normalization = store(
        X87Chop53::div(
            X87Chop53::load_f64(NativeF64Bits::ONE).unwrap(),
            X87Chop53::mul(y_step_extended, load(x_step)),
        )
        .unwrap(),
    );
    let mut result = Vec::with_capacity((generated_size.0 * generated_size.1) as usize);
    let mut y0 = 0.0f32;
    for _ in 0..generated_size.1 {
        let y1_extended = X87Chop53::add(load(y0), load(y_step));
        let y1_stored = store(y1_extended);
        let first_y = ftol(load(y0));
        let last_y = (ftol(y1_extended) + 1).min(raw_size.1);
        let mut x0 = 0.0f32;
        for _ in 0..generated_size.0 {
            let x1_extended = X87Chop53::add(load(x0), load(x_step));
            let x1 = store(x1_extended);
            let first_x = ftol(load(x0));
            let last_x = (ftol(x1_extended) + 1).min(raw_size.0);
            let mut accum = [X87Chop53::load_i32(0); 3];
            for sy in first_y..last_y {
                let y_weight = overlap(
                    load(y0),
                    y1_extended,
                    X87Chop53::load_i32(sy),
                    X87Chop53::load_i32(sy + 1),
                );
                for sx in first_x..last_x {
                    let x_weight = overlap(
                        load(x0),
                        load(x1),
                        X87Chop53::load_i32(sx),
                        X87Chop53::load_i32(sx + 1),
                    );
                    let weight =
                        X87Chop53::mul(X87Chop53::mul(x_weight, y_weight), load(normalization));
                    let sample = raw[(sy * raw_size.0 + sx) as usize];
                    for channel in 0..3 {
                        accum[channel] = X87Chop53::add(
                            accum[channel],
                            X87Chop53::mul(X87Chop53::load_i32(sample[channel] as i32), weight),
                        );
                    }
                }
            }
            let convert = |value| {
                ftol(X87Chop53::add(
                    value,
                    X87Chop53::load_f64(NativeF64Bits::HALF).unwrap(),
                ))
                .min(255) as u8
            };
            result.push(pack_rgb565(
                convert(accum[0]),
                convert(accum[1]),
                convert(accum[2]),
            ));
            x0 = x1;
        }
        y0 = y1_stored;
    }
    result
}

fn overlap(
    start: crate::util::native_x87::X87Value,
    end: crate::util::native_x87::X87Value,
    pixel_start: crate::util::native_x87::X87Value,
    pixel_end: crate::util::native_x87::X87Value,
) -> crate::util::native_x87::X87Value {
    let low = if X87Chop53::compare(start, pixel_start) == X87Ordering::Greater {
        start
    } else {
        pixel_start
    };
    let high = if X87Chop53::compare(end, pixel_end) == X87Ordering::Less {
        end
    } else {
        pixel_end
    };
    X87Chop53::sub(high, low)
}
