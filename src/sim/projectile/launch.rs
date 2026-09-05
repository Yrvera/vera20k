//! Native FireAt launch math, with the stores and lookup boundaries retained.
//!
//! Owners: 6FE8EE..6FF014, 48A8D0/48A9D0, 4CAE30/4CB3D0,
//! 4CADB0 and 4CACB0/4CAD00. This module does not resolve world receivers.

use crate::map::retail_trig::{AcosTable, TrigTable};
use crate::util::native_x87::{
    NativeF32Bits, NativeF64Bits, X87Chop53 as X, X87Ordering, X87Value,
};

use super::{ProjectileCoord, ProjectileVelocity};

const PI_HALF: NativeF64Bits = NativeF64Bits::from_bits(0x3ff9_21fb_5444_2d18);
const WORD_SCALE: NativeF64Bits = NativeF64Bits::from_bits(0xc0c4_5f07_af68_ecef);
const RADIAN_SCALE: NativeF64Bits = NativeF64Bits::from_bits(0xbf19_222d_989f_5e57);
const POSITIVE_HEIGHT_ANGLE: NativeF64Bits = NativeF64Bits::from_bits(0x3fe9_21d9_f4d3_7c12);
const TRIG_SCALE: NativeF32Bits = NativeF32Bits::from_bits(0x4522_f983);

fn d(bits: NativeF64Bits) -> X87Value {
    X::load_f64(bits).expect("finite native launch input")
}

fn f(bits: NativeF32Bits) -> X87Value {
    X::load_f32(bits).expect("finite native launch table value")
}

fn store(value: X87Value) -> NativeF64Bits {
    X::store_f64(value).expect("finite stored native launch value")
}

fn round(value: X87Value) -> X87Value {
    d(store(value))
}

fn int(value: X87Value) -> i32 {
    X::ftol_i64(value).expect("native launch conversion fits signed i64") as i32
}

fn sqrt(value: X87Value) -> X87Value {
    f(crate::util::native_x87::sqrt_approx_f32(round(value))
        .expect("native launch squared value fits the finite lookup domain"))
}

fn less(a: X87Value, b: X87Value) -> bool {
    X::compare(a, b) == X87Ordering::Less
}

fn radians(word: u16) -> NativeF64Bits {
    store(X::mul(
        X::load_i32(i32::from(word as i16) - 0x3fff),
        d(RADIAN_SCALE),
    ))
}

fn angle_word(angle: X87Value) -> u16 {
    int(X::mul(X::sub(angle, d(PI_HALF)), d(WORD_SCALE))) as u16
}

fn sin(table: &TrigTable, angle: NativeF64Bits) -> X87Value {
    let units = int(X::mul(d(angle), f(TRIG_SCALE)));
    f(NativeF32Bits::from_bits(table.sin(units).to_bits()))
}

fn cos(table: &TrigTable, angle: NativeF64Bits) -> X87Value {
    let units = int(X::mul(d(angle), f(TRIG_SCALE)));
    f(NativeF32Bits::from_bits(table.cos(units).to_bits()))
}

fn atan(y: X87Value, x: X87Value) -> X87Value {
    crate::util::direction_tables::native_atan2_f32(
        X::store_f32(y).expect("finite launch atan numerator"),
        X::store_f32(x).expect("finite launch atan denominator"),
    )
}

fn acos(table: &AcosTable, argument: X87Value) -> X87Value {
    // 4CADB0 uses a NEGATIVE scale and subtracts the signed offset from
    // 859094. Within the active table domain this advances into 4097 entries.
    let scaled = X::mul(
        X::add(d(store(argument)), X::load_i32(1)),
        X::load_i32(-2048),
    );
    let index = int(scaled).wrapping_neg() as usize;
    X::sub(
        d(PI_HALF),
        f(NativeF32Bits::from_bits(table.entry(index).to_bits())),
    )
}

fn acos_nan(table: &AcosTable) -> X87Value {
    // A native 0/0 reaches ftol's indefinite 80000000 low word. SHL 2
    // wraps it to zero, so 4CADCF reads entry zero rather than rejecting it.
    X::sub(
        d(PI_HALF),
        f(NativeF32Bits::from_bits(table.entry(0).to_bits())),
    )
}

/// Original 48A9D0. Height >0 bypasses every arithmetic failure predicate.
fn arc_angle(
    table: &AcosTable,
    range: i32,
    height: i32,
    speed: i32,
    gravity: NativeF64Bits,
    high_root: bool,
) -> Option<NativeF64Bits> {
    if height > 0 {
        return Some(POSITIVE_HEIGHT_ANGLE);
    }
    let zero = X::load_i32(0);
    let h = X::load_i32(height);
    let s = X::load_i32(speed);
    let g = d(gravity);
    let r = X::load_i32(range);
    let speed2 = X::mul(s, s);
    let base = round(X::sub(speed2, X::mul(h, g)));
    let range2 = round(X::mul(r, r));
    let speed4 = X::mul(X::mul(speed2, s), s);
    let cross = X::mul(X::mul(speed2, h), g);
    let discriminant = round(X::sub(
        X::sub(speed4, X::add(cross, cross)),
        X::mul(X::mul(g, g), range2),
    ));
    if less(discriminant, zero) {
        return None;
    }
    if range == 0 {
        // The original division yields NaN for 0/0, and +Inf for H²/0.
        // Its ordered predicates reject the former. The latter gives a
        // signed-zero quotient; sqrt returns +0, then speed zero gives NaN.
        if height == 0 {
            return None;
        }
        return Some(store(if speed == 0 {
            acos_nan(table)
        } else {
            acos(table, zero)
        }));
    }
    let ratio = X::div(X::mul(h, h), range2).expect("nonzero squared range");
    let denominator_half = X::add(ratio, X::load_i32(1));
    let denominator = round(X::add(denominator_half, denominator_half));
    let root = sqrt(discriminant);
    let numerator = if high_root {
        X::sub(base, root)
    } else {
        X::add(root, base)
    };
    let quotient = round(X::div(numerator, denominator).expect("positive finite denominator"));
    if less(quotient, zero) {
        return None;
    }
    if speed == 0 {
        return Some(store(acos_nan(table)));
    }
    let argument = round(X::div(sqrt(quotient), s).expect("nonzero launch speed"));
    Some(store(acos(table, argument)))
}

/// 48A8D0 as reached by FireAt 6FED21. A failed second probe still selects
/// the sign: 70D590's return address occupies the residue's high DWORD.
/// Every valid Acos-table angle exceeds every possible such tiny positive
/// residue (minimum angle 7.549789948768648e-8, maximum residue <1.5e-306).
fn fireat_arc_pitch(
    table: &AcosTable,
    range: i32,
    height: i32,
    speed: i32,
    gravity: NativeF64Bits,
    high_root: bool,
) -> Option<u16> {
    let first = d(arc_angle(table, range, height, speed, gravity, high_root)?);
    let invert = !high_root
        && arc_angle(table, range.wrapping_add(1), height, speed, gravity, false)
            .is_none_or(|second| less(d(second), first));
    Some(angle_word(if invert { X::neg(first) } else { first }))
}

/// World receiver results are resolved by the firing owner, before this scalar
/// kernel. `heading` is native DirStruct (north=0), not math BAM.
pub(crate) struct FireAtLaunch {
    pub delta: ProjectileCoord,
    pub speed: i32,
    pub vertical: bool,
    pub heading: Option<u16>,
    pub arcing: bool,
    pub gravity: NativeF64Bits,
    pub high_root: bool,
    pub voxel_downward: Option<bool>,
    /// BuildingType+EF4*200 minus the live source virtual+300 result Z.
    pub building_pitch_height: Option<i32>,
}

pub(crate) struct FireAtLaunchResult {
    pub velocity: ProjectileVelocity,
    pub speed: i32,
}

/// Native 70D590, after the firing owner resolves source raw Location and the
/// current target's virtual+48 coordinate. Its second target read is equivalent
/// for these stable Techno coordinates; cell dummy receivers remain upstream.
pub(crate) fn high_arc_root(
    lobber: bool,
    source: ProjectileCoord,
    target: Option<ProjectileCoord>,
) -> bool {
    if lobber {
        return true;
    }
    let Some(target) = target else {
        return false;
    };
    let height = target.z.wrapping_sub(source.z);
    if height <= 0 {
        return false;
    }
    let x = X::load_i32(source.x.wrapping_sub(target.x));
    let y = X::load_i32(source.y.wrapping_sub(target.y));
    int(sqrt(X::add(X::mul(x, x), X::mul(y, y)))) < height
}

/// FireAt 6FE8EE..6FF014 for ordinary ROT<=0 and stock RadialFireSegments=0.
/// The returned doubles are the six DWORDs copied by Bullet::Fire 468691..A0.
pub(crate) fn fireat_launch(input: FireAtLaunch) -> Option<FireAtLaunchResult> {
    let (trig, acos_table) = crate::map::retail_trig::required_math_tables();
    fireat_launch_with_tables(input, trig, acos_table)
}

fn fireat_launch_with_tables(
    input: FireAtLaunch,
    trig: &TrigTable,
    acos_table: &AcosTable,
) -> Option<FireAtLaunchResult> {
    let x = X::load_i32(input.delta.x);
    let y = X::load_i32(input.delta.y);
    let z = X::load_i32(input.delta.z);
    let x2 = round(X::mul(x, x));
    let distance = int(sqrt(X::add(X::add(X::mul(z, z), X::mul(y, y)), x2)));
    let mut speed = input.speed.min(distance / 2);
    if input.vertical {
        speed = 1;
    }
    let heading = input
        .heading
        .unwrap_or_else(|| angle_word(atan(X::load_i32(input.delta.y.wrapping_neg()), x)));

    // Scale the native (100,0,0) seed, retaining both normalization lookups.
    let scale =
        X::div(X::load_i32(speed), sqrt(X::load_i32(10000))).expect("positive native seed norm");
    let mut seed_x = round(X::mul(scale, X::load_i32(100)));
    let seed_y = round(X::mul(scale, X::load_i32(0)));
    if X::compare(seed_x, X::load_i32(0)) == X87Ordering::Equal
        && X::compare(seed_y, X::load_i32(0)) == X87Ordering::Equal
    {
        seed_x = X::load_i32(100);
    }
    let horizontal_seed = round(sqrt(X::add(X::mul(seed_x, seed_x), X::mul(seed_y, seed_y))));
    let heading_radians = radians(heading);
    let mut vx = round(X::mul(cos(trig, heading_radians), horizontal_seed));
    let mut vy = round(X::neg(X::mul(sin(trig, heading_radians), horizontal_seed)));
    let vz = X::load_i32(0);
    let horizontal_range = || int(sqrt(X::add(X::mul(y, y), x2)));
    let pitch = if input.arcing {
        fireat_arc_pitch(
            acos_table,
            horizontal_range(),
            input.delta.z,
            speed,
            input.gravity,
            input.high_root,
        )?
    } else if let Some(downward) = input.voxel_downward {
        if downward { 0x8000 } else { 0x4000 }
    } else if input.delta.z.wrapping_abs() > 200 {
        let height = input.building_pitch_height.unwrap_or(input.delta.z);
        let absolute_height = height.wrapping_abs();
        let bias = if input.building_pitch_height.is_some() && absolute_height < 20 {
            0
        } else {
            20
        };
        let range = X::load_i32(horizontal_range());
        let minimum = d(NativeF64Bits::from_bits(0x3fa9_9999_9999_999a));
        let denominator = if less(range, minimum) { minimum } else { range };
        let angle = atan(
            X::sub(X::load_i32(absolute_height), X::load_i32(bias)),
            denominator,
        );
        angle_word(if height < 0 { X::neg(angle) } else { angle })
    } else {
        0x3fff
    };

    let existing_pitch = radians(angle_word(atan(
        vz,
        sqrt(X::add(X::mul(vx, vx), X::mul(vy, vy))),
    )));
    let magnitude = round(sqrt(X::add(
        X::add(X::mul(vx, vx), X::mul(vy, vy)),
        X::mul(vz, vz),
    )));
    if X::compare(d(existing_pitch), X::load_i32(0)) != X87Ordering::Equal {
        vx = round(X::div(vx, cos(trig, existing_pitch)).expect("nonzero initial pitch cosine"));
        vy = round(X::div(vy, cos(trig, existing_pitch)).expect("nonzero initial pitch cosine"));
    }
    let pitch_radians = radians(pitch);
    Some(FireAtLaunchResult {
        velocity: ProjectileVelocity::from_native([
            store(X::mul(cos(trig, pitch_radians), vx)),
            store(X::mul(cos(trig, pitch_radians), vy)),
            store(X::mul(sin(trig, pitch_radians), magnitude)),
        ]),
        speed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn tables() -> (&'static TrigTable, &'static AcosTable) {
        let tables = crate::map::retail_trig::required_math_tables();
        assert!(
            tables.0.matches_retail() && tables.1.matches_retail(),
            "native launch comparison requires RA2_DIR with verified gamemd.exe"
        );
        tables
    }

    #[test]
    #[ignore = "requires RA2_DIR with verified gamemd.exe math tables"]
    fn original_fireat_and_fire_preserve_exact_launch_bits() {
        let (trig, acos) = tables();
        for (name, data) in [
            (
                "ordinary",
                include_str!("../../../tools/projectile_oracle/fireat_launch.json"),
            ),
            (
                "voxel",
                include_str!("../../../tools/projectile_oracle/voxel_launch.json"),
            ),
            (
                "second probe",
                include_str!("../../../tools/projectile_oracle/arc_second_probe.json"),
            ),
            (
                "building",
                include_str!("../../../tools/projectile_oracle/building_pitch.json"),
            ),
            (
                "directed",
                include_str!("../../../tools/projectile_oracle/directed_launch.json"),
            ),
        ] {
            let rows: Vec<Value> = serde_json::from_str(data).unwrap();
            for (index, row) in rows.iter().enumerate() {
                let delta = &row["delta"];
                let delta = ProjectileCoord::new(
                    delta[0].as_i64().unwrap() as i32,
                    delta[1].as_i64().unwrap() as i32,
                    delta[2].as_i64().unwrap() as i32,
                );
                let arcing = row["arcing"].as_bool().unwrap_or(name == "second probe");
                let input = FireAtLaunch {
                    delta,
                    speed: row["speed"].as_i64().unwrap() as i32,
                    vertical: row["vertical"].as_bool().unwrap_or(false),
                    heading: row["hull"].as_u64().map(|hull| {
                        if row["turret"].as_bool().unwrap() {
                            row["barrel"].as_u64().unwrap() as u16
                        } else {
                            hull as u16
                        }
                    }),
                    arcing,
                    gravity: super::super::projectile_gravity(
                        6,
                        row["floater"].as_bool().unwrap_or(false),
                    ),
                    high_root: high_arc_root(
                        row["lobber"].as_bool().unwrap_or(false),
                        ProjectileCoord::new(0, 0, 0),
                        Some(delta),
                    ),
                    voxel_downward: row["voxel"]
                        .as_bool()
                        .unwrap_or(false)
                        .then_some(delta.z < 0),
                    building_pitch_height: row["building_height"].as_i64().map(|height| {
                        (height as i32)
                            .wrapping_mul(200)
                            .wrapping_sub(row["source_z"].as_i64().unwrap() as i32)
                    }),
                };
                let result = fireat_launch_with_tables(input, trig, acos);
                assert_eq!(
                    result.is_some(),
                    row["success"].as_u64().unwrap() != 0,
                    "{name} native row {index}: {row}"
                );
                if let Some(result) = result {
                    let actual = result
                        .velocity
                        .native()
                        .map(|v| format!("{:016x}", v.bits()));
                    let expected: Vec<_> = row["bits"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap())
                        .collect();
                    assert_eq!(
                        actual.as_slice(),
                        expected.as_slice(),
                        "{name} native row {index}: {row}"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "requires RA2_DIR with verified gamemd.exe math tables"]
    fn original_arc_solver_domain_and_failure_predicates() {
        let (_, table) = tables();
        let rows: Vec<Value> = serde_json::from_str(include_str!(
            "../../../tools/projectile_oracle/arc_domain.json"
        ))
        .unwrap();
        for (index, row) in rows.iter().enumerate() {
            let result = arc_angle(
                table,
                row["range"].as_i64().unwrap() as i32,
                row["height"].as_i64().unwrap() as i32,
                row["speed"].as_i64().unwrap() as i32,
                NativeF64Bits::from_bits(row["gravity"].as_f64().unwrap().to_bits()),
                row["mode"].as_u64().unwrap() != 0,
            );
            assert_eq!(
                result.is_some(),
                row["angle_ok"].as_u64().unwrap() != 0,
                "native solver row {index}: {row}"
            );
            if let Some(result) = result {
                let raw = row["angle_raw"].as_str().unwrap();
                let bytes: [u8; 8] = std::array::from_fn(|i| {
                    u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).unwrap()
                });
                assert_eq!(
                    result.bits(),
                    u64::from_le_bytes(bytes),
                    "native solver row {index}: {row}"
                );
            }
        }
    }
}
