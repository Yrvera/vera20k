//! Behavior-3 Spark system AI — the burst spawner that owns `spark.rs`'s
//! per-particle kernel.
//!
//! `spark.rs` has held the verified per-particle arithmetic and `spark_world.rs`
//! the collision inputs; what was missing was the system that creates the
//! particles and runs them. This module is that system: the burst gate, the
//! cap-derived burst size, the velocity draws, the countdown, and the facing
//! walk, in the order `ParticleSystemClass::AI_Spark @ 0x0062E840` commits them.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ and util/ only.

use glam::IVec3;

use crate::rules::particle_type::ParticleType;
use crate::rules::ruleset::RuleSet;
use crate::sim::particles::spark::{
    SparkKernelError, begin_particle_tick, finish_particle_tick, gravity_as_stored_f32,
};
use crate::sim::particles::spark_world::SparkCollisionWorld;
use crate::sim::particles::{Particle, ParticleSystem, SparkRuntimeState};
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;
use crate::util::fixed_math::SIM_ZERO;
use crate::util::native_x87::{
    NativeF32Bits, NativeF64Bits, X87Chop53, X87Ordering, X87Value, sqrt_approx_f32,
};

/// `Random__RandomRanged(0, 0x7ffffffe)` — the inclusive high bound the spark
/// probability and facing draws share with the per-particle color draw.
const RANGED_DRAW_MAX: u32 = 0x7fff_fffe;

/// `(double)sample * 4.656612877414201e-10`, i.e. the `1 / 2^31` the probability
/// comparisons scale their raw sample by.
const RANGED_DRAW_RECIPROCAL: NativeF64Bits = NativeF64Bits::from_bits(0x3e00_0000_0040_0000);

/// The two facing walk thresholds at the tail of `AI_Spark`.
const FACING_STEP_DOWN_THRESHOLD: f64 = 0.3;
const FACING_STEP_UP_THRESHOLD: f64 = 0.6;

/// `0x11`/`0x29` — the clamps the facing walk holds the system between.
const FACING_MIN: i32 = 0x11;
const FACING_MAX: i32 = 0x29;

/// Advance one Spark `ParticleSystem` by one tick.
pub(super) fn tick_system(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    if sys.spark_spawn_frames > 0 {
        run_spawn_frame(sys, sim, rules);

        // The countdown is decremented after the optional burst, and reaching
        // zero is what retires the system — its `lifetime` is not consulted.
        sys.spark_spawn_frames -= 1;
        if sys.spark_spawn_frames < 1 {
            sys.marked_for_deletion = true;
        }

        walk_facing(sys, sim.particle_rng());
    }

    tick_particles(sys, sim, rules);
}

/// The burst half: `0x0062E85E`..`0x0062ECC9`.
fn run_spawn_frame(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    let pst = rules.particle_system_type(sys.type_id);
    let Some(pt_id) = pst.holds_what else {
        // `pstype+0x294 == -1` still runs the countdown and the facing walk; it
        // just has no ParticleType to construct.
        return;
    };
    let cap = pst.particle_cap as i32;
    let spawn_chance = pst.spawn_spark_percentage;
    let spawn_direction = pst.spawn_direction;
    let directionless = sys.directionless;

    // Final frame spawns unconditionally and draws nothing; every earlier frame
    // pays one ranged draw for the probability test.
    if sys.spark_spawn_frames != 1 && !ranged_probability_hit(sim.particle_rng(), spawn_chance) {
        return;
    }

    let half = cap / 2;
    if half <= 0 {
        return;
    }
    let count = sim.particle_rng().next_raw_abs_modulo(half as u32) as i32 + half;

    // Three draws are taken once per burst, before the loop, and only the
    // directionless arm consumes them — but they are drawn either way.
    let pt = rules.particle_type(pt_id).clone();
    let shared_second = sim
        .particle_rng()
        .next_raw_modulo_signed(pt.y_velocity.max(1) as u32);
    let shared_first = sim
        .particle_rng()
        .next_raw_modulo_signed(pt.x_velocity.max(1) as u32);
    let shared_third = sim
        .particle_rng()
        .next_raw_modulo_signed(pt.z_velocity_range.max(1) as u32);
    // The first draw feeds Y and the second feeds X; the native locals are
    // assigned across each other at `0x0062E8C4`..`0x0062E8D2`.
    let shared_x = shared_second;
    let shared_y = shared_first;
    let shared_z = shared_third;

    for _ in 0..count {
        let Some(mut particle) = construct_particle(sys.coords, pt_id, &pt, sim.particle_rng())
        else {
            continue;
        };
        let vx = sim
            .particle_rng()
            .next_raw_modulo_signed(pt.x_velocity.max(1) as u32);
        let vy = sim
            .particle_rng()
            .next_raw_modulo_signed(pt.y_velocity.max(1) as u32);
        let vz = sim
            .particle_rng()
            .next_raw_abs_modulo(pt.z_velocity_range.max(1) as u32) as i32
            + pt.min_z_velocity;

        let offset = if directionless {
            IVec3::new(shared_x, shared_y, shared_z)
        } else {
            spawn_direction
        };
        if let Some(spark) = particle.spark.as_mut() {
            match blend_launch_velocity(IVec3::new(vx, vy, vz), offset) {
                Ok(velocity) => {
                    spark.velocity_x = velocity[0];
                    spark.velocity_y = velocity[1];
                    spark.velocity_z = velocity[2];
                }
                Err(_) => continue,
            }
        }
        sys.particles.push(particle);
    }
}

/// `ParticleClass::Constructor @ 0x0062B5E0` as reached from the Spark burst:
/// both coordinate arguments are the system's own position, so the constructor's
/// own direction vector is zero and the burst overwrites it below. The draws it
/// consumes are what matter here — one for lifetime, and one more for the color
/// endpoints when the type authors them.
fn construct_particle(
    coords: IVec3,
    pt_id: crate::rules::particle_type::ParticleTypeId,
    pt: &ParticleType,
    rng: &mut SimRng,
) -> Option<Particle> {
    let max_ec = pt.max_ec.max(1);
    let lifetime_extra = rng.next_raw_abs_modulo(u32::from(max_ec)) as i16;
    let lifetime_remaining = (pt.max_ec as i16).saturating_add(lifetime_extra);

    let start_rgb = start_color(pt, rng);

    Some(Particle {
        type_id: pt_id,
        coords,
        previous_coords: coords,
        origin: coords,
        direction: [SIM_ZERO; 3],
        velocity: pt.velocity,
        lifetime_remaining,
        damage_counter: pt.max_dc as i16,
        state_ai_advance: pt.state_ai_advance,
        animation_state: pt.start_state_ai,
        translucency: pt.translucency,
        hit_ground: false,
        marked_for_deletion: false,
        drift_x: 0,
        drift_y: 0,
        drift_z: 0,
        current_color: start_rgb,
        color_index: 0,
        color_accumulator: SIM_ZERO,
        spark: Some(SparkRuntimeState {
            velocity_x: NativeF32Bits::from_bits(0),
            velocity_y: NativeF32Bits::from_bits(0),
            velocity_z: NativeF32Bits::from_bits(0),
            start_rgb,
            color_index: 0,
            color_accumulator: NativeF64Bits::POSITIVE_ZERO,
        }),
        prev_delta: [SIM_ZERO; 3],
        state_advance_counter: 0,
    })
}

/// `0x0062B7C4`..`0x0062B82A`: a type with no authored `ColorList` keeps its
/// stored color; a type whose two start colors are all-zero takes the list head
/// with no draw; anything else pays one ranged draw and interpolates.
///
/// RESIDUAL (GSI-05.13) — the interpolation is `FUN_00661020 @ 0x00661020`,
/// which computes `ftol(a * (1 - t) + b * t)` per component. Its two clamp arms
/// decompile degenerately (both tails run the same `ftol` store), so the
/// out-of-range behavior is UNCHECKED. Trigger: a Spark type whose
/// `StartColor1`/`StartColor2` interpolate outside `0..=255`. Player effect: a
/// wrong launch tint on such a spark. Frequency: none in stock — every
/// `[Particles]` Spark entry authors both endpoints inside the byte range.
/// Downstream risk: none; the draw is consumed either way, so RNG order holds.
fn start_color(pt: &ParticleType, rng: &mut SimRng) -> [u8; 3] {
    if pt.color_list.is_empty() {
        return [0; 3];
    }
    let a = pt.start_color_1;
    let b = pt.start_color_2;
    if a == [0; 3] && b == [0; 3] {
        return pt.color_list[0];
    }
    let sample = rng.next_range_u32_inclusive(0, RANGED_DRAW_MAX);
    let t = scale_ranged_sample(sample);
    let mut out = [0u8; 3];
    for index in 0..3 {
        let lhs = f64::from(a[index]) * (1.0 - t);
        let rhs = f64::from(b[index]) * t;
        out[index] = (lhs + rhs).clamp(0.0, 255.0) as u8;
    }
    out
}

/// The launch vector: measure the raw draw's magnitude, add the spawn offset,
/// renormalize, then scale back to the original magnitude.
fn blend_launch_velocity(
    raw: IVec3,
    offset: IVec3,
) -> Result<[NativeF32Bits; 3], SparkKernelError> {
    let raw = [stored_f32(raw.x)?, stored_f32(raw.y)?, stored_f32(raw.z)?];
    let magnitude = magnitude_of(raw)?;

    let blended = [
        add_stored(raw[0], stored_f32(offset.x)?)?,
        add_stored(raw[1], stored_f32(offset.y)?)?,
        add_stored(raw[2], stored_f32(offset.z)?)?,
    ];
    let blended_magnitude = magnitude_of(blended)?;

    let zero = X87Chop53::load_i32(0);
    let normalized = if X87Chop53::compare(X87Chop53::load_f32(blended_magnitude)?, zero)
        == X87Ordering::Equal
    {
        blended
    } else {
        let divisor = X87Chop53::load_f32(blended_magnitude)?;
        [
            X87Chop53::store_f32(X87Chop53::div(X87Chop53::load_f32(blended[0])?, divisor)?)?,
            X87Chop53::store_f32(X87Chop53::div(X87Chop53::load_f32(blended[1])?, divisor)?)?,
            X87Chop53::store_f32(X87Chop53::div(X87Chop53::load_f32(blended[2])?, divisor)?)?,
        ]
    };

    let scale = X87Chop53::load_f32(magnitude)?;
    Ok([
        X87Chop53::store_f32(X87Chop53::mul(scale, X87Chop53::load_f32(normalized[0])?))?,
        X87Chop53::store_f32(X87Chop53::mul(scale, X87Chop53::load_f32(normalized[1])?))?,
        X87Chop53::store_f32(X87Chop53::mul(scale, X87Chop53::load_f32(normalized[2])?))?,
    ])
}

/// `Sqrt_Approx(z*z + y*y + x*x)` — the summation order is native's.
fn magnitude_of(components: [NativeF32Bits; 3]) -> Result<NativeF32Bits, SparkKernelError> {
    let x = X87Chop53::load_f32(components[0])?;
    let y = X87Chop53::load_f32(components[1])?;
    let z = X87Chop53::load_f32(components[2])?;
    let sum = X87Chop53::add(
        X87Chop53::add(X87Chop53::mul(z, z), X87Chop53::mul(y, y)),
        X87Chop53::mul(x, x),
    );
    sqrt_approx_f32(sum).map_err(SparkKernelError::from)
}

fn stored_f32(value: i32) -> Result<NativeF32Bits, SparkKernelError> {
    X87Chop53::store_f32(X87Chop53::load_i32(value)).map_err(SparkKernelError::from)
}

fn add_stored(lhs: NativeF32Bits, rhs: NativeF32Bits) -> Result<NativeF32Bits, SparkKernelError> {
    let sum = X87Chop53::add(X87Chop53::load_f32(lhs)?, X87Chop53::load_f32(rhs)?);
    X87Chop53::store_f32(sum).map_err(SparkKernelError::from)
}

/// `(double)sample * 2^-31`, the scaling both probability comparisons use.
fn scale_ranged_sample(sample: u32) -> f64 {
    let scaled = X87Chop53::mul(
        X87Chop53::load_i32(sample as i32),
        X87Chop53::load_f64(RANGED_DRAW_RECIPROCAL).unwrap_or_else(|_| X87Chop53::load_i32(0)),
    );
    x87_to_f64(scaled)
}

fn x87_to_f64(value: X87Value) -> f64 {
    X87Chop53::store_f64(value)
        .map(|bits| f64::from_bits(bits.bits()))
        .unwrap_or(0.0)
}

fn ranged_probability_hit(rng: &mut SimRng, chance: crate::util::fixed_math::SimFixed) -> bool {
    let sample = rng.next_range_u32_inclusive(0, RANGED_DRAW_MAX);
    scale_ranged_sample(sample) <= chance.to_num::<f64>()
}

/// The tail walk at `0x0062ECAE`..`0x0062ECE7`: one draw picks down, up, or hold.
fn walk_facing(sys: &mut ParticleSystem, rng: &mut SimRng) {
    let sample = rng.next_range_u32_inclusive(0, RANGED_DRAW_MAX);
    if let Some(next) = stepped_facing(i32::from(sys.facing), scale_ranged_sample(sample)) {
        sys.facing = next as u8;
    }
}

/// The walk's three arms. `None` is the third arm, which jumps past the store
/// and therefore leaves the facing alone.
fn stepped_facing(facing: i32, roll: f64) -> Option<i32> {
    if roll < FACING_STEP_DOWN_THRESHOLD {
        let stepped = facing - 3;
        Some(if stepped < FACING_MIN + 1 {
            FACING_MIN
        } else {
            stepped
        })
    } else if roll < FACING_STEP_UP_THRESHOLD {
        let stepped = facing + 3;
        Some(if stepped > FACING_MAX - 1 {
            FACING_MAX
        } else {
            stepped
        })
    } else {
        None
    }
}

/// The dispatch and compacting-removal tail: every particle is advanced in
/// creation order, then the vector is walked backwards destroying the marked
/// ones.
fn tick_particles(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    if sys.particles.is_empty() {
        return;
    }
    let Ok(gravity) = gravity_as_stored_f32(rules.general.gravity) else {
        return;
    };

    let mut motions = Vec::with_capacity(sys.particles.len());
    for particle in &mut sys.particles {
        match begin_particle_tick(particle, gravity) {
            Ok(motion) => motions.push(Some(motion)),
            Err(_) => motions.push(None),
        }
    }

    // Collision facts are gathered under an immutable borrow, before the
    // per-particle RNG is taken; no draw happens inside this window.
    let mut facts = Vec::with_capacity(motions.len());
    {
        let Ok(world) = SparkCollisionWorld::new(sim, rules) else {
            return;
        };
        for motion in &motions {
            facts.push(motion.and_then(|motion| world.query(motion).ok()));
        }
    }

    for ((particle, motion), fact) in sys
        .particles
        .iter_mut()
        .zip(motions.into_iter())
        .zip(facts.into_iter())
    {
        let (Some(motion), Some(fact)) = (motion, fact) else {
            continue;
        };
        let pt = rules.particle_type(particle.type_id);
        let color_speed = pt.color_speed;
        let color_count = pt.color_list.len();
        let _ = finish_particle_tick(
            particle,
            motion,
            fact,
            color_speed,
            color_count,
            sim.particle_rng(),
        );
    }

    sys.particles
        .retain(|particle| !particle.marked_for_deletion);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::particle_system_type::ParticleSystemTypeId;

    /// `SparkSys`-shaped rules: `ParticleCap=6` so the burst is 3..=5, and a
    /// spark ParticleType carrying the four velocity keys the burst reads.
    fn spark_rules(spark_spawn_frames: u32, spawn_spark_percentage: &str) -> RuleSet {
        let ini_text = format!(
            "[Particles]\n\
             1=Spk\n\
             [ParticleSystems]\n\
             1=Sys\n\
             [Spk]\n\
             BehavesLike=Spark\n\
             MaxEC=10\n\
             MaxDC=4\n\
             StartStateAI=0\n\
             EndStateAI=10\n\
             StateAIAdvance=4\n\
             Translucency=0\n\
             XVelocity=20\n\
             YVelocity=20\n\
             MinZVelocity=1\n\
             ZVelocityRange=10\n\
             [Sys]\n\
             BehavesLike=Spark\n\
             HoldsWhat=Spk\n\
             ParticleCap=6\n\
             SpawnFrames=1\n\
             Lifetime=200\n\
             SparkSpawnFrames={spark_spawn_frames}\n\
             SpawnSparkPercentage={spawn_spark_percentage}\n",
        );
        let ini = IniFile::from_str(&ini_text);
        RuleSet::from_ini(&ini).expect("rules parse")
    }

    fn spark_system(sim: &mut Simulation, rules: &RuleSet) -> u64 {
        sim.spawn_particle_system(
            ParticleSystemTypeId(0),
            IVec3::new(8 * 256, 8 * 256, 0),
            None,
            None,
            IVec3::ZERO,
            None,
            rules,
        )
        .expect("spark systems are admitted")
    }

    /// `0x0062E85E`: the last spawn frame skips the probability draw entirely,
    /// and `0x0062E88E` sizes the burst as `|Next()| % (cap / 2) + cap / 2`.
    #[test]
    fn final_spawn_frame_bursts_without_a_probability_draw() {
        let rules = spark_rules(1, "0.0");
        let mut sim = Simulation::new();
        let id = spark_system(&mut sim, &rules);

        let mut sys = sim
            .particle_systems_mut()
            .take_for_tick(id)
            .expect("system present");
        super::tick_system(&mut sys, &mut sim, &rules);

        // ParticleCap = 6, so half = 3 and the burst is 3..=5 even though
        // SpawnSparkPercentage is zero.
        assert!(
            (3..=5).contains(&sys.particles.len()),
            "burst size {} outside cap-derived 3..=5",
            sys.particles.len()
        );
        assert!(sys.particles.iter().all(|p| p.spark.is_some()));
        assert_eq!(sys.spark_spawn_frames, 0);
        assert!(
            sys.marked_for_deletion,
            "the countdown reaching zero retires the system"
        );
    }

    /// A zero `SpawnSparkPercentage` on a non-final frame loses the probability
    /// test, so the frame spends its draw and spawns nothing.
    #[test]
    fn non_final_frame_with_zero_percentage_spawns_nothing_but_still_counts_down() {
        let rules = spark_rules(4, "0.0");
        let mut sim = Simulation::new();
        let id = spark_system(&mut sim, &rules);

        let mut sys = sim
            .particle_systems_mut()
            .take_for_tick(id)
            .expect("system present");
        super::tick_system(&mut sys, &mut sim, &rules);

        assert!(sys.particles.is_empty());
        assert_eq!(sys.spark_spawn_frames, 3);
        assert!(!sys.marked_for_deletion);
    }

    /// `0x0062ECAE`: one draw picks step-down, step-up, or hold, and the two
    /// stepping arms clamp to `0x11` and `0x29`.
    #[test]
    fn facing_walk_steps_by_three_and_clamps_to_the_native_bounds() {
        // Hold arm: at or above 0.6 the store is jumped past.
        assert_eq!(stepped_facing(0x1D, 0.6), None);
        assert_eq!(stepped_facing(0x1D, 0.99), None);
        // Step-down arm below 0.3, step-up arm in between.
        assert_eq!(stepped_facing(0x1D, 0.0), Some(0x1D - 3));
        assert_eq!(stepped_facing(0x1D, 0.299), Some(0x1D - 3));
        assert_eq!(stepped_facing(0x1D, 0.3), Some(0x1D + 3));
        assert_eq!(stepped_facing(0x1D, 0.599), Some(0x1D + 3));
        // Both clamps, including the native off-by-one shape: the low arm
        // clamps when the step lands below 0x12, the high arm when it lands
        // above 0x28.
        assert_eq!(stepped_facing(FACING_MIN + 3, 0.0), Some(FACING_MIN));
        assert_eq!(stepped_facing(FACING_MIN + 4, 0.0), Some(FACING_MIN + 1));
        assert_eq!(stepped_facing(FACING_MAX - 3, 0.5), Some(FACING_MAX));
        assert_eq!(stepped_facing(FACING_MAX - 4, 0.5), Some(FACING_MAX - 1));
    }

    /// The live walk stays inside the clamps and only ever moves by three.
    #[test]
    fn live_facing_walk_stays_inside_the_clamped_band() {
        let rules = spark_rules(4, "0.0");
        let mut sim = Simulation::new();
        let id = spark_system(&mut sim, &rules);
        let mut sys = sim
            .particle_systems_mut()
            .take_for_tick(id)
            .expect("system present");
        assert_eq!(sys.facing, 0x1D, "constructor facing");

        for _ in 0..256 {
            let before = i32::from(sys.facing);
            walk_facing(&mut sys, sim.particle_rng());
            let after = i32::from(sys.facing);
            assert!((FACING_MIN..=FACING_MAX).contains(&after));
            let delta = after - before;
            assert!(
                delta == 0 || delta.abs() == 3 || after == FACING_MIN || after == FACING_MAX,
                "facing moved by {delta} without hitting a clamp"
            );
        }
    }
}
