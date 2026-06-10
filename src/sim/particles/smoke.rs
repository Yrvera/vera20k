//! Smoke `BehavesLike` system + particle AI.
//!
//! Per-tick smoke logic for both the system (spawning, accumulator) and the
//! individual particles (lifetime, decel). Includes the two-child
//! `NextParticle` finding: when a smoke particle dies, the system spawns
//! TWO children at symmetric `(+dx, +dy)` and `(-dx, -dy)` offsets — not one.
//! Getting this wrong silently halves smoke density and changes the
//! visual signature of damaged buildings + refinery vents.
//!
//! ## Deferred (tracked for follow-up tasks)
//! - Entity-attached following (needs a `Position → IVec3 leptons` helper).
//! - Translucency cutoff fade in the spawn path.
//! - Spawn velocity reduction formula (accumulator delta × 0.025).
//! - Bridge collision in `move_smoke`.
//! - Wiring `move_smoke` into the per-tick path — needs `[General] WindDirection=`
//!   to be parsed first; today `move_smoke` is a tested helper waiting for a caller.
//! - Per-particle animation state machine + 25%-on-even-frame random drift.

use super::wind::{SMOKE_WIND_DX, SMOKE_WIND_DY};
use super::{Particle, ParticleSystem};
use crate::rules::particle_type::{ParticleType, ParticleTypeId};
use crate::rules::ruleset::RuleSet;
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};
use glam::IVec3;

/// Advance one smoke `ParticleSystem` by one tick.
pub(super) fn tick_system(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    let pst = rules.particle_system_type(sys.type_id);
    let cap = pst.particle_cap as usize;
    let tick = sim.session.tick;

    // Phase 1 — tick existing particles.
    for p in &mut sys.particles {
        let pt = rules.particle_type(p.type_id);
        let frame_count = super::system_ai::resolve_image_frame_count(sim, pt);
        tick_particle(p, pt, frame_count);
    }

    // Phase 2 — collect NextParticle children for any dying particle, then prune.
    // Two-child finding: each dying parent spawns at +(dx,dy) AND -(dx,dy).
    let mut child_specs: Vec<ChildSpec> = Vec::new();
    for p in &sys.particles {
        if !p.marked_for_deletion {
            continue;
        }
        let pt = rules.particle_type(p.type_id);
        let Some(next_id) = pt.next_particle else {
            continue;
        };
        let r = pt.radius >> 3; // type's Radius / 8
        let dx = symmetric_offset(r, sim.particle_rng());
        let dy = symmetric_offset(r, sim.particle_rng());
        let parent_coords = p.coords;
        let parent_velocity = p.velocity;
        let parent_translucency = p.translucency;
        child_specs.push(ChildSpec {
            next_id,
            coords: parent_coords + IVec3::new(dx, dy, 0),
            velocity: parent_velocity,
            translucency: parent_translucency,
        });
        child_specs.push(ChildSpec {
            next_id,
            coords: parent_coords + IVec3::new(-dx, -dy, 0),
            velocity: parent_velocity,
            translucency: parent_translucency,
        });
    }
    sys.particles.retain(|p| !p.marked_for_deletion);
    for spec in child_specs {
        if sys.particles.len() >= cap {
            break;
        }
        let pt = rules.particle_type(spec.next_id);
        sys.particles.push(make_child(spec, pt, sim.particle_rng()));
    }

    // Phase 3 — spawn a new particle if conditions allow.
    if !sys.done_spawning && pst.spawns {
        let timer_int = sys.spawn_timer.to_num::<i32>().max(1) as u64;
        if tick % timer_int == 0 {
            if let Some(holds) = pst.holds_what {
                if sys.particles.len() < cap {
                    let pt = rules.particle_type(holds);
                    let r = pst.spawn_radius.max(0) as u32;
                    let off_x = sim.particle_rng().next_raw_modulo_signed(r + 1);
                    let off_y = sim.particle_rng().next_raw_modulo_signed(r + 1);
                    let spawn_pos = IVec3::new(
                        sys.coords.x + off_x,
                        sys.coords.y + off_y,
                        sys.coords.z + 10,
                    );
                    sys.particles.push(make_particle(
                        holds,
                        spawn_pos,
                        spawn_pos,
                        pt,
                        sim.particle_rng(),
                    ));
                }
            }
        }
    }

    // Phase 4 — accumulator. Per the binary, slowdown advances the timer toward
    // spawn_cutoff; once the timer crosses the cutoff, the system stops spawning.
    sys.spawn_timer += pst.slowdown;
    if pst.spawn_cutoff < sys.spawn_timer {
        sys.done_spawning = true;
    }
}

/// Per-tick AI for one smoke particle. Minimal Tier-2 form: state-AI advance
/// (animation_state + translucency-state byte writes) → lifetime countdown
/// → velocity decel. Drift lands later.
pub(super) fn tick_particle(p: &mut Particle, pt: &ParticleType, image_frame_count: u16) {
    super::system_ai::advance_state(p, pt, image_frame_count);
    p.lifetime_remaining = p.lifetime_remaining.saturating_sub(1);
    if p.lifetime_remaining <= 0 {
        p.marked_for_deletion = true;
    }
    if p.velocity > SIM_ZERO {
        p.velocity = (p.velocity - pt.deacc).max(SIM_ZERO);
    }
}

/// Apply one tick of smoke-table wind drift to `p.coords`. Caller supplies the
/// global wind direction (FacingType 0..7); `wind_effect` from the particle
/// type scales the magnitude. Out-of-range wind directions are clamped to 0.
///
/// Bridge collision and odd-frame gating are deferred — see module-level notes.
pub(super) fn move_smoke(p: &mut Particle, pt: &ParticleType) {
    move_smoke_with_wind(p, pt, smoke_wind_dir());
}

/// Internal helper exposed for testing — lets a test pin a specific wind
/// direction without touching global rules state.
pub(super) fn move_smoke_with_wind(p: &mut Particle, pt: &ParticleType, wind_dir: u8) {
    let idx = (wind_dir as usize).min(7);
    let scale = pt.wind_effect as i32;
    p.coords.x += SMOKE_WIND_DX[idx] * scale;
    p.coords.y += SMOKE_WIND_DY[idx] * scale;
    p.coords.x += p.drift_x;
    p.coords.y += p.drift_y;
    p.coords.z += p.drift_z;
}

/// Default smoke wind direction. Real value comes from `[General] WindDirection=`
/// once that's parsed; until then everything stays at index 0 (north).
fn smoke_wind_dir() -> u8 {
    0
}

struct ChildSpec {
    next_id: ParticleTypeId,
    coords: IVec3,
    velocity: SimFixed,
    translucency: u8,
}

fn make_child(spec: ChildSpec, pt: &ParticleType, rng: &mut SimRng) -> Particle {
    let mut p = make_particle(spec.next_id, spec.coords, spec.coords, pt, rng);
    p.velocity = spec.velocity;
    p.translucency = spec.translucency;
    p
}

fn make_particle(
    type_id: ParticleTypeId,
    coords: IVec3,
    spawn_origin: IVec3,
    pt: &ParticleType,
    rng: &mut SimRng,
) -> Particle {
    let base = (pt.max_ec as u32).max(1);
    let lifetime_extra = rng.next_raw_abs_modulo(base) as i16;
    let lifetime_remaining = (pt.max_ec as i16).saturating_add(lifetime_extra);
    Particle {
        type_id,
        coords,
        previous_coords: spawn_origin,
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
        current_color: [0; 3],
        color_index: 0,
        color_accumulator: SimFixed::from_num(0),
        prev_delta: [SIM_ZERO; 3],
        state_advance_counter: 0,
    }
}

/// Symmetric random offset around `r`. With `r > 0`, draws a signed remainder
/// `raw` in `[-(r-1), r-1]` (one raw draw, no abs):
///   - `raw <= 0` returns `raw - r` (a value in `[-(2r-1), -r]`)
///   - `raw >= 1` returns `raw + r` (a value in `[r+1, 2r-1]`)
/// The sign-split matches the original two-child spawn path.
fn symmetric_offset(r: i32, rng: &mut SimRng) -> i32 {
    if r <= 0 {
        return 0;
    }
    let raw = rng.next_raw_modulo_signed(r as u32);
    if raw < 1 { raw - r } else { raw + r }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::particle_system_type::ParticleSystemTypeId;
    use crate::sim::particles::ParticleSystem;
    use glam::IVec3;

    fn fake_system(type_id: ParticleSystemTypeId) -> ParticleSystem {
        ParticleSystem {
            stable_id: 0,
            type_id,
            coords: IVec3::ZERO,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(1),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1D,
            marked_for_deletion: false,
            directionless: false,
            attached_entity: None,
            owner_entity: None,
            target_coords: IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        }
    }

    fn parse(ini_text: &str) -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(ini_text)).expect("rules parse")
    }

    #[test]
    fn next_particle_spawns_two_children_at_symmetric_offsets() {
        // Smoke chain: SmkA → SmkB. Radius=8 so r=8>>3=1, giving deterministic
        // offsets of either +/-1 from symmetric_offset.
        let rules = parse(
            "[Particles]\n\
             1=SmkA\n\
             2=SmkB\n\
             [SmkA]\n\
             BehavesLike=Smoke\n\
             MaxEC=1\n\
             Radius=8\n\
             NextParticle=SmkB\n\
             [SmkB]\n\
             BehavesLike=Smoke\n\
             MaxEC=10\n\
             [ParticleSystems]\n\
             1=Sys\n\
             [Sys]\n\
             BehavesLike=Smoke\n\
             HoldsWhat=SmkA\n\
             ParticleCap=20\n",
        );
        let mut sim = Simulation::new();
        let mut sys = fake_system(ParticleSystemTypeId(0));
        // Seed one SmkA particle with lifetime=1 so it dies this tick.
        let pt_a = rules.particle_type(ParticleTypeId(0));
        let mut parent = make_particle(
            ParticleTypeId(0),
            IVec3::new(1000, 2000, 0),
            IVec3::ZERO,
            pt_a,
            sim.particle_rng(),
        );
        parent.lifetime_remaining = 1;
        sys.particles.push(parent);

        tick_system(&mut sys, &mut sim, &rules);

        // Parent died → two SmkB children. Original parent removed.
        assert_eq!(sys.particles.len(), 2, "two-child finding");
        assert!(sys.particles.iter().all(|p| p.type_id == ParticleTypeId(1)));
        // The two children must be symmetric around the parent's position.
        let a = sys.particles[0].coords;
        let b = sys.particles[1].coords;
        let parent_pos = IVec3::new(1000, 2000, 0);
        let offset_a = a - parent_pos;
        let offset_b = b - parent_pos;
        assert_eq!(offset_a, -offset_b, "children symmetric around parent");
        assert_ne!(offset_a, IVec3::ZERO, "offsets must be non-zero");
    }

    #[test]
    fn symmetric_offset_uses_signed_remainder() {
        // The signed helper can yield a negative remainder, so the raw<1 branch
        // reaches [-(2r-1), -r] for any negative draw — a region the old
        // non-negative next_range_u32 (min offset -r only at raw==0) could never
        // produce. seed=1 third raw draw is negative (0xDA63B931 = -630_998_735);
        // with r=100: -630_998_735 % 100 = -35 -> raw<1 -> raw - r = -135.
        let mut rng = SimRng::new(1);
        rng.next_u32(); // skip draw 1
        rng.next_u32(); // skip draw 2 -> next (3rd) draw is the negative 0xDA63B931
        assert_eq!(symmetric_offset(100, &mut rng), -135);
    }

    #[test]
    fn spawn_cap_enforced() {
        // Cap=5 — even with aggressive spawning, particle count must stay ≤ 5.
        let rules = parse(
            "[Particles]\n\
             1=Smk\n\
             [Smk]\n\
             BehavesLike=Smoke\n\
             MaxEC=1000\n\
             [ParticleSystems]\n\
             1=Sys\n\
             [Sys]\n\
             BehavesLike=Smoke\n\
             HoldsWhat=Smk\n\
             ParticleCap=5\n\
             SpawnFrames=1\n\
             Spawns=yes\n",
        );
        let mut sim = Simulation::new();
        let mut sys = fake_system(ParticleSystemTypeId(0));
        for _ in 0..50 {
            tick_system(&mut sys, &mut sim, &rules);
            sim.session.tick += 1;
        }
        assert!(
            sys.particles.len() <= 5,
            "cap exceeded: {}",
            sys.particles.len()
        );
    }

    #[test]
    fn done_spawning_when_accumulator_exceeds_cutoff() {
        // Slowdown=1 per tick, SpawnCutoff=2 → after 3 ticks accumulator > cutoff.
        let rules = parse(
            "[ParticleSystems]\n\
             1=Sys\n\
             [Sys]\n\
             BehavesLike=Smoke\n\
             ParticleCap=10\n\
             Slowdown=1.0\n\
             SpawnCutoff=2.0\n",
        );
        let mut sim = Simulation::new();
        let mut sys = fake_system(ParticleSystemTypeId(0));
        // spawn_timer starts at 1. After tick 1: 1+1=2, cutoff (2) < timer (2) is FALSE.
        // After tick 2: 2+1=3, cutoff (2) < timer (3) is TRUE → done_spawning.
        for _ in 0..2 {
            tick_system(&mut sys, &mut sim, &rules);
            sim.session.tick += 1;
        }
        assert!(
            sys.done_spawning,
            "done_spawning should be set after cutoff"
        );
    }

    #[test]
    fn move_smoke_uses_smoke_table_at_se() {
        // SE wind direction (idx 3): smoke=2, gas=1.
        let rules = parse(
            "[Particles]\n\
             1=Smk\n\
             [Smk]\n\
             BehavesLike=Smoke\n\
             WindEffect=1\n\
             MaxEC=10\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::new(0, 0, 0),
            IVec3::ZERO,
            pt,
            sim.particle_rng(),
        );
        move_smoke_with_wind(&mut p, pt, 3);
        // SMOKE_WIND_DX[3] = 2 → coords.x advanced by +2 (smoke table, not gas).
        assert_eq!(p.coords.x, 2);
        // SMOKE_WIND_DY[3] = 2.
        assert_eq!(p.coords.y, 2);
    }
}
