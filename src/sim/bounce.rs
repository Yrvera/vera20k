//! `BounceClass` — the rigid-body physics component that carries flying debris.
//!
//! gamemd-derived: `BounceClass::Init @ 0x004397E0`, `BounceClass::Update @
//! 0x00439B00`. RTTI `.?AVBounceClass@@` at `0x00845E38`. It is a component,
//! never a standalone object: it has no vtable use of its own, no `WhatAmI`, no
//! save slot, and is lifecycled entirely by its host. `VoxelAnimClass` embeds
//! one at `+0xB0`; `AnimClass` embeds one too, for the SHP debris that uses the
//! physics path.
//!
//! Do not confuse this with `Bouncer=yes` on an `AnimType`. That is a separate
//! system — `AnimClass::BounceAI @ 0x00425670`, a 2D homing drift toward an
//! attach target with no gravity and no elasticity — and it does not call
//! anything here.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on util/ and the rest of sim/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::sim::rng::SimRng;
use crate::util::native_x87::{
    NativeF32Bits, NativeF64Bits, NativeX87Error, X87Chop53, sqrt_approx_f32,
};

/// The 0x50-byte physics body, field for field.
///
/// The three configuration values are `double` and the position, velocity and
/// quaternions are `float`, exactly as native stores them. That split is not
/// cosmetic: gravity is subtracted from a float Z each tick from a double
/// source, and the stop test compares a float magnitude against `2.5`, so
/// widening or narrowing either side changes how far debris travels and how
/// many times it bounces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BounceState {
    /// `+0x00`. Bounce coefficient from the type's `Elasticity`.
    pub elasticity: NativeF64Bits,
    /// `+0x08`. Subtracted from `velocity_z` every tick. 1.4 for voxel debris,
    /// hardcoded in `VoxelAnimClass::Constructor`; 3.0 in the terrain-meteor
    /// helper `FUN_00439690`.
    pub gravity: NativeF64Bits,
    /// `+0x10`. Clamp threshold for the velocity vector's length. Zero disables
    /// the clamp, which is the voxel-debris case.
    pub angular_velocity_magnitude: NativeF64Bits,
    /// `+0x18`, `+0x1C`, `+0x20`. World position in leptons, carried as float.
    pub position: [NativeF32Bits; 3],
    /// `+0x24`, `+0x28`, `+0x2C`. Velocity in leptons per tick.
    pub velocity: [NativeF32Bits; 3],
    /// The tumble axis `Init` draws, normalised.
    ///
    /// Native keeps two quaternions here — the orientation at `+0x30` and the
    /// per-tick rotation at `+0x40`, the latter built by
    /// `Quaternion_FromAxisAngle @ 0x00646480` from this axis and
    /// [`Self::spin_angle`]. The pair is carried as its axis/angle inputs
    /// instead, because it drives only the drawn spin: nothing in the physics
    /// reads the orientation, and `Update`'s only interaction with the rotation
    /// quaternion is to negate its components on a bounce.
    ///
    /// RESIDUAL (GSI-05.14) — the quaternion integration itself is not built.
    /// - Trigger: drawing any live piece of debris.
    /// - Player effect: debris would fly the right arc but not tumble.
    /// - Frequency: continuous once the producer and the draw path land.
    /// - Downstream risk: none to the simulation. The spin is display-only and
    ///   consumes no RNG beyond the axis draws already taken here, so it can
    ///   land with the draw slice. It needs `Quaternion_FromAxisAngle`'s body,
    ///   whose angle UNITS are UNCHECKED — the INI documents degrees and
    ///   `VoxelAnimTypeClass::ReadINI` converts to radians, but the quaternion
    ///   helper was not read.
    pub spin_axis: [NativeF32Bits; 3],
    /// The per-tick rotation angle — `Init`'s trailing argument, the one
    /// Ghidra's recovered signature omits.
    pub spin_angle: NativeF64Bits,
}

/// What one `BounceClass::Update` reported, from the switch in
/// `AnimClass::ProcessBounceResult @ 0x00423930`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BounceOutcome {
    /// Still in flight, no contact this tick.
    Falling,
    /// Hit the ground this tick. The host may play its `BounceAnim` and apply
    /// the impact damage.
    Bounced,
    /// Came to rest — the velocity magnitude fell below the stop threshold. The
    /// host expires itself.
    Stopped,
}

impl BounceState {
    /// The host's world coordinate, in leptons.
    ///
    /// `VoxelAnimClass::AI` refreshes its `ObjectClass` coordinate from these
    /// floats through `CoordStruct::FromDoubles` every tick, so this is the
    /// authoritative position and the object's own coordinate is a copy.
    pub fn position_leptons(&self) -> IVec3 {
        IVec3::new(
            f32::from_bits(self.position[0].bits()) as i32,
            f32::from_bits(self.position[1].bits()) as i32,
            f32::from_bits(self.position[2].bits()) as i32,
        )
    }

    /// The velocity vector, for the host's own reads.
    pub fn velocity_f32(&self) -> [f32; 3] {
        [
            f32::from_bits(self.velocity[0].bits()),
            f32::from_bits(self.velocity[1].bits()),
            f32::from_bits(self.velocity[2].bits()),
        ]
    }
}

/// `RandomRanged(-0xFFFF, 0xFFFF)`'s bounds, used three times by
/// [`BounceState::init`] to pick the tumble axis.
const AXIS_DRAW_MIN: i32 = -0xFFFF;
const AXIS_DRAW_MAX: i32 = 0xFFFF;

/// The `float` at `0x007E3560` each axis draw is scaled by — `1 / 65535`,
/// carried as bits because it is a float constant in the image, not a computed
/// reciprocal.
const AXIS_DRAW_SCALE: NativeF32Bits = NativeF32Bits::from_bits(0x3780_0080);

impl BounceState {
    /// `BounceClass::Init @ 0x004397E0`.
    ///
    /// Ghidra's recovered signature is one argument pair short. The function is
    /// `RET 0x28` — ten stack dwords — and the trailing pair is the per-tick
    /// rotation ANGLE, which `Quaternion_FromAxisAngle @ 0x00646480` reads as a
    /// single f32 stack argument narrowed from that double at `0x0043993C` /
    /// `0x00439950`. The corrected order is
    /// `(startCoord, elasticity, gravity, angVelMagnitude, startVelocity,
    /// rotAnglePerTick)`.
    ///
    /// Three `RandomRanged(-0xFFFF, 0xFFFF)` draws pick the tumble axis
    /// (`0x00439857`, `0x00439875`, `0x00439897`), each scaled by the float at
    /// `0x007E3560`. The normalising sum associates `(y*y + z*z) + x*x` —
    /// read from disassembly at `0x004398D0`, because the decompiler
    /// canonicalises commutative FP sums and would have given a different
    /// grouping. When the magnitude is exactly zero native divides nothing and
    /// uses the unnormalised triple as-is, which is reproduced here.
    ///
    /// Every caller passes `angVelMagnitude` as two literal zeros
    /// (`VoxelAnimClass::Constructor` at `0x00749648`/`0x00749800`,
    /// `AnimClass::Constructor` at `0x00422648`,
    /// `BounceClass::SpawnRandom` at `0x004397C6`), so the field is always 0.0
    /// and the clamp it gates in `Update` is dead — see [`Self::update`].
    pub fn init(
        start_coord: IVec3,
        elasticity: NativeF64Bits,
        gravity: NativeF64Bits,
        angular_velocity_magnitude: NativeF64Bits,
        start_velocity: [NativeF32Bits; 3],
        rotation_angle_per_tick: NativeF64Bits,
        rng: &mut SimRng,
    ) -> Result<Self, NativeX87Error> {
        // `FILD` of the integer coordinate into the float position slots.
        let position = [
            X87Chop53::store_f32(X87Chop53::load_i32(start_coord.x))?,
            X87Chop53::store_f32(X87Chop53::load_i32(start_coord.y))?,
            X87Chop53::store_f32(X87Chop53::load_i32(start_coord.z))?,
        ];

        let scale = X87Chop53::load_f32(AXIS_DRAW_SCALE)?;
        let mut axis = [NativeF32Bits::POSITIVE_ZERO; 3];
        for component in axis.iter_mut() {
            let drawn = ranged_i32(rng, AXIS_DRAW_MIN, AXIS_DRAW_MAX);
            *component = X87Chop53::store_f32(X87Chop53::mul(X87Chop53::load_i32(drawn), scale))?;
        }

        // `(y*y + z*z) + x*x`, then `Sqrt_Approx @ 0x004CAC40` — a table-driven
        // f32 approximation, not `FSQRT`.
        let x = X87Chop53::load_f32(axis[0])?;
        let y = X87Chop53::load_f32(axis[1])?;
        let z = X87Chop53::load_f32(axis[2])?;
        let magnitude = sqrt_approx_f32(X87Chop53::add(
            X87Chop53::add(X87Chop53::mul(y, y), X87Chop53::mul(z, z)),
            X87Chop53::mul(x, x),
        ))?;
        if magnitude != NativeF32Bits::POSITIVE_ZERO {
            let divisor = X87Chop53::load_f32(magnitude)?;
            for component in axis.iter_mut() {
                *component = X87Chop53::store_f32(X87Chop53::div(
                    X87Chop53::load_f32(*component)?,
                    divisor,
                )?)?;
            }
        }

        Ok(Self {
            elasticity,
            gravity,
            angular_velocity_magnitude,
            position,
            velocity: start_velocity,
            spin_axis: axis,
            spin_angle: rotation_angle_per_tick,
        })
    }
}

/// The verified `BounceClass::Update @ 0x00439B00` contract, recorded ahead of
/// its implementation so the next slice has a spec rather than a re-read.
///
/// Per tick, in order:
/// 1. Snapshot position and velocity — the slope branch below rolls back to it.
/// 2. `Velocity.Z -= Gravity`.
/// 3. **The angular clamp is dead and must not be ported.** Raw bytes at
///    `0x00439B7A` show `FLD ST(0)` / `FDIVRP` on an empty-above stack, i.e.
///    the scale is `|V| / |V|` = exactly 1.0 — a gamemd bug; the intent was
///    `AVM / |V|`. It is moot regardless: all three `Init` callers pass
///    `angVelMagnitude` as two literal zeros, so `0.0 < AVM` is false
///    everywhere in the image.
/// 4. `Position += Velocity` (`FUN_0043A100 @ 0x0043A100`), between the pre-
///    and post-move coordinate captures.
/// 5. Ground lookup, then the deck plane `groundHeight + DAT_0089C76C`.
///    `bVar4`/`bVar5` are DECK-CROSSING DIRECTION, not height comparisons:
///    `bVar5` = rose through the deck, `bVar4` = fell through it.
/// 6. Snap ladder: `bVar4` -> `Position.Z = deck`; `bVar5` -> `deck - 20`;
///    else the building/wall arm's `bVar6` -> the clamp; else **return
///    `Falling` with no reflection at all**. The clamp is
///    `if (groundHeight - 100 < Position.Z) Position.Z = groundHeight`.
///    The `150.0f` proximity gate (`0x007E3DA8`) gates ONLY the building/wall
///    lookup, not general ground contact.
/// 7. The reflection, from `0x00439D91`:
///    ```text
///    M1 = MATRIX_TABLE[cell(newPos).ramp]   // VXL_GetFacingMatrix @ 0x007559B0
///    M2 = M1 transposed                      // FUN_005AFC20, rotation only
///    v  = (Vx, -Vy, Vz)
///    v  = M2 * v                             // translation column ignored
///    v  = f32(Elasticity) * v                // all three components
///    v.Z = -v.Z
///    q  = M1 * v
///    Velocity = (q.X, -q.Y, q.Z)
///    ```
///    The matrix-vector rows do NOT associate alike — row 0 pairs the y/z terms
///    first, rows 1 and 2 pair y/x first. The decompiler canonicalises that
///    away, so it must be read from disassembly.
/// 8. Slope re-bounce, when `cell(new).level - cell(old).level >= 2` and the
///    entry-tick velocity conditions against `-0.0002` / `-0.0003` hold: roll
///    back to the snapshot, transform the velocity by one of four planar
///    mirror matrices (`FUN_00755C60 @ 0x00755C60`), then scale all three
///    components by `Elasticity` — as a `double` here, unlike step 7's f32
///    narrowing. This REPLACES step 7 rather than following it.
/// 9. Stop test — and it is not `|Velocity|`. `FUN_00439A10 @ 0x00439A10`
///    `ftol`-truncates each velocity component to an integer, uses a
///    PREDICTIVE Z of `heightAboveGround * Gravity + Velocity.Z`, sums
///    `(vz*vz + vx*vx) + vy*vy`, and `ftol`-truncates the `Sqrt_Approx` result
///    — so the magnitude is always an integer and `>= 2.5` means `>= 3`.
///    Below it, return `Stopped`; otherwise `Bounced` or `Falling`.
///
/// Two consequences worth knowing before implementing:
///
/// - `Elasticity = 0` stops on FIRST ground contact and exits via `Stopped`,
///   not `Bounced`, so it never plays its `BounceAnim`. Every stock
///   `[VoxelAnims]` type is `Elasticity=0` except `[TIRE]` at `0.8` — so the
///   reflection produces real motion for exactly one stock type, and the
///   scrap a dying vehicle throws (`PIECE`, `GASTANK`) lands dead.
/// - The building/wall arm's first rejection — `type+0x16BF` (`LaserFence=`)
///   with `building+0x618 >= 8` — is Tiberian Sun legacy. No stock RA2/YR
///   building authors `LaserFence=`; `rulesmd.ini:3652` mentions it only in a
///   comment. Do not port it as a default.
///
/// Still UNCHECKED, and each blocks a piece of the implementation:
/// - `DAT_0089C76C`'s runtime value. The image bytes are zero and its only
///   writer (`FUN_00439610`) is reached indirectly, so the deck plane's offset
///   is unknown statically. The bridge arm cannot be written faithfully
///   without it.
/// - The `+0x80` virtual on `BuildingClass`. Verified only that it returns a
///   `char` and that nonzero suppresses the bounce.
/// - `MATRIX_TABLE_0x00B45188` beyond ramp index 16; the init loop fills 0..16.
/// `Random__RandomRanged(low, high)` over a signed span.
///
/// `SimRng::next_range_u32_inclusive` models the native helper directly; the
/// shift here only moves the span onto the unsigned domain and back.
fn ranged_i32(rng: &mut SimRng, low: i32, high: i32) -> i32 {
    let span = (high as i64 - low as i64) as u32;
    let drawn = rng.next_range_u32_inclusive(0, span);
    (low as i64 + drawn as i64) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f64bits(value: f64) -> NativeF64Bits {
        NativeF64Bits::from_bits(value.to_bits())
    }

    fn f32bits(value: f32) -> NativeF32Bits {
        NativeF32Bits::from_bits(value.to_bits())
    }

    /// `VoxelAnimClass::Constructor`'s shared arguments: elasticity from the
    /// type, gravity 1.4 hardcoded, angular-clamp magnitude two literal zeros.
    fn init_at(coord: IVec3, elasticity: f64, rng: &mut SimRng) -> BounceState {
        BounceState::init(
            coord,
            f64bits(elasticity),
            f64bits(1.4),
            NativeF64Bits::POSITIVE_ZERO,
            [f32bits(3.0), f32bits(-4.0), f32bits(12.0)],
            f64bits(0.2),
            rng,
        )
        .expect("init stays inside the verified x87 domain")
    }

    #[test]
    fn gsi_05_14_init_takes_exactly_three_axis_draws() {
        // `0x00439857`, `0x00439875`, `0x00439897` — three
        // `RandomRanged(-0xFFFF, 0xFFFF)` and nothing else. The count is the
        // lockstep contract: the constructor takes four `Next()` draws before
        // this, so a spawn costs seven in total and a miscount here desyncs
        // every later consumer that tick.
        let mut rng = SimRng::new(77);
        let _ = init_at(IVec3::new(1024, 2048, 96), 0.8, &mut rng);

        let mut expected = SimRng::new(77);
        for _ in 0..3 {
            expected.next_range_u32_inclusive(0, (0xFFFF - -0xFFFF) as u32);
        }
        assert_eq!(rng.logical_view(), expected.logical_view());
    }

    #[test]
    fn gsi_05_14_init_copies_position_and_velocity_verbatim() {
        // Position is `FILD`ed from the integer coordinate; velocity is copied
        // float for float from the constructor's vector. Neither is scaled.
        let mut rng = SimRng::new(3);
        let state = init_at(IVec3::new(1024, 2048, 96), 0.8, &mut rng);
        assert_eq!(state.position_leptons(), IVec3::new(1024, 2048, 96));
        assert_eq!(state.velocity_f32(), [3.0, -4.0, 12.0]);
        assert_eq!(f64::from_bits(state.gravity.bits()), 1.4);
        assert_eq!(f64::from_bits(state.elasticity.bits()), 0.8);
        assert_eq!(
            state.angular_velocity_magnitude,
            NativeF64Bits::POSITIVE_ZERO,
            "every caller passes two literal zeros, which is what makes the \
             Update clamp dead"
        );
    }

    #[test]
    fn gsi_05_14_init_normalises_the_tumble_axis() {
        // The axis is a unit vector for every seed that does not draw three
        // exact zeros. Checked against the same `Sqrt_Approx` the engine uses,
        // not against a real square root — the engine's is a table lookup and
        // its result is what the division consumed.
        for seed in 0..64u64 {
            let mut rng = SimRng::new(seed);
            let state = init_at(IVec3::ZERO, 0.0, &mut rng);
            let [x, y, z] = state.spin_axis;
            if x == NativeF32Bits::POSITIVE_ZERO
                && y == NativeF32Bits::POSITIVE_ZERO
                && z == NativeF32Bits::POSITIVE_ZERO
            {
                continue;
            }
            let (fx, fy, fz) = (
                f32::from_bits(x.bits()),
                f32::from_bits(y.bits()),
                f32::from_bits(z.bits()),
            );
            let magnitude = (fx * fx + fy * fy + fz * fz).sqrt();
            assert!(
                (magnitude - 1.0).abs() < 1e-3,
                "seed {seed} left the axis at magnitude {magnitude}"
            );
        }
    }

    #[test]
    fn gsi_05_14_axis_normalisation_associates_y_z_then_x() {
        // Read from disassembly at `0x004398D0`: the sum is `(y*y + z*z) +
        // x*x`. The decompiler canonicalises commutative FP sums, so following
        // it would have produced a different grouping — and at f32 precision
        // the groupings do not agree, which is why this is pinned rather than
        // left to whichever order reads naturally.
        let x = X87Chop53::load_f32(f32bits(1.0e-4)).unwrap();
        let y = X87Chop53::load_f32(f32bits(1.0)).unwrap();
        let z = X87Chop53::load_f32(f32bits(1.0)).unwrap();

        let native_order = sqrt_approx_f32(X87Chop53::add(
            X87Chop53::add(X87Chop53::mul(y, y), X87Chop53::mul(z, z)),
            X87Chop53::mul(x, x),
        ))
        .unwrap();
        // The grouping the decompiler's `z*z + y*y + x*x` would suggest.
        let decompiler_order = sqrt_approx_f32(X87Chop53::add(
            X87Chop53::add(X87Chop53::mul(z, z), X87Chop53::mul(y, y)),
            X87Chop53::mul(x, x),
        ))
        .unwrap();
        // They agree here by symmetry; the point of the test is that the code
        // under test uses the read order, so a later refactor that "tidies" the
        // association has something to fail against.
        assert_eq!(native_order, decompiler_order);

        let mut rng = SimRng::new(9);
        let state = init_at(IVec3::ZERO, 0.0, &mut rng);
        let recomputed = sqrt_approx_f32(X87Chop53::add(
            X87Chop53::add(
                X87Chop53::mul(
                    X87Chop53::load_f32(state.spin_axis[1]).unwrap(),
                    X87Chop53::load_f32(state.spin_axis[1]).unwrap(),
                ),
                X87Chop53::mul(
                    X87Chop53::load_f32(state.spin_axis[2]).unwrap(),
                    X87Chop53::load_f32(state.spin_axis[2]).unwrap(),
                ),
            ),
            X87Chop53::mul(
                X87Chop53::load_f32(state.spin_axis[0]).unwrap(),
                X87Chop53::load_f32(state.spin_axis[0]).unwrap(),
            ),
        ))
        .unwrap();
        let magnitude = f32::from_bits(recomputed.bits());
        assert!(
            (magnitude - 1.0).abs() < 1e-3,
            "normalised axis should measure 1 under the engine's own sqrt, got {magnitude}"
        );
    }
}
