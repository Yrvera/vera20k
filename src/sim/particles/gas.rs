//! Gas `BehavesLike` system + particle AI.
//!
//! Per-tick gas logic for both the system (spawning, accumulator) and the
//! individual particles (lifetime, decel, damage countdown). Differs from
//! smoke in two key ways:
//!   - `NextParticle` chains spawn a SINGLE child at the parent's position
//!     (offset by `NextParticleOffset`), not two symmetric children.
//!   - The child copies the parent's velocity AND its drift_x/y/z fields,
//!     so a moving gas plume keeps drifting after a chain step.
//!
//! `move_gas` uses the `GAS_WIND_*` tables (which differ from smoke at SE).
//! Wind apply is gated by odd-frame parity and `tick % (10 / wind_effect)`.
//!
//! ## Deferred (tracked for follow-up tasks)
//! - Damage application to cell occupants when the damage counter hits zero.
//!   Needs `OccupancyGrid` cell-iteration helpers + a Position (cell coords)
//!   ↔ IVec3 leptons converter; lands with Task C6. The "no friend-or-foe
//!   filter" parity rule lives there. C3 only does the countdown bookkeeping.
//! - Bridge collision in `move_gas`.
//! - Wiring `move_gas` into the per-tick path — needs `[General] WindDirection=`
//!   to be parsed first; today `move_gas` is a tested helper waiting for a caller.
//! - Per-particle animation state machine + 1-in-8 even-frame random drift.
//! - Gravity (Z velocity = -2.0 - RulesClass.Gravity).

use super::wind::{GAS_WIND_DX, GAS_WIND_DY};
use super::{Particle, ParticleSystem};
use crate::rules::particle_type::{ParticleType, ParticleTypeId};
use crate::rules::ruleset::RuleSet;
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};
use glam::IVec3;

/// Advance one gas `ParticleSystem` by one tick.
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
    // Single-child finding: each dying parent spawns ONE child at
    // `parent.coords + pt.next_particle_offset`, copying velocity + drift.
    let mut child_specs: Vec<ChildSpec> = Vec::new();
    for p in &sys.particles {
        if !p.marked_for_deletion {
            continue;
        }
        let pt = rules.particle_type(p.type_id);
        let Some(next_id) = pt.next_particle else {
            continue;
        };
        child_specs.push(ChildSpec {
            next_id,
            coords: p.coords + pt.next_particle_offset,
            velocity: p.velocity,
            translucency: p.translucency,
            drift_x: p.drift_x,
            drift_y: p.drift_y,
            drift_z: p.drift_z,
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
                    // PARITY-YELLOW: the original gas-system per-tick AI makes NO
                    // spawn-offset RNG draw here — this periodic-spawn path is
                    // cloned from smoke and has no counterpart in the gas AI, so
                    // these two draws are a phantom scenario-cursor advance vs
                    // gamemd. Converting them to the raw-signed helper preserves
                    // current behavior and keeps the file consistent for the
                    // regression guard; true cursor parity needs the gas spawn
                    // path reworked (tracked: OQ-PARTICLE-RNG-007).
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

    // Phase 4 — accumulator. Slowdown advances the timer toward spawn_cutoff;
    // once the timer crosses the cutoff, the system stops spawning.
    sys.spawn_timer += pst.slowdown;
    if pst.spawn_cutoff < sys.spawn_timer {
        sys.done_spawning = true;
    }
}

/// Per-tick AI for one gas particle. Tier-2 form: state-AI advance →
/// lifetime countdown → velocity decel → damage-counter bookkeeping
/// (reset on zero). Actual damage application to cell occupants is
/// deferred to Task C6 — see module-level notes.
pub(super) fn tick_particle(p: &mut Particle, pt: &ParticleType, image_frame_count: u16) {
    super::system_ai::advance_state(p, pt, image_frame_count);
    p.lifetime_remaining = p.lifetime_remaining.saturating_sub(1);
    if p.lifetime_remaining <= 0 {
        p.marked_for_deletion = true;
    }
    if p.velocity > SIM_ZERO {
        p.velocity = (p.velocity - pt.deacc).max(SIM_ZERO);
    }
    p.damage_counter = p.damage_counter.saturating_sub(1);
    if p.damage_counter <= 0 {
        // C6 will hook the damage-to-cell-occupants iteration here.
        p.damage_counter = pt.max_dc as i16;
    }
}

/// Apply one tick of gas-table wind drift to `p.coords`. Caller supplies the
/// global wind direction (FacingType 0..7) and the current sim tick;
/// `wind_effect` from the particle type acts as a frequency divider, not a
/// magnitude scale (this is the smoke/gas asymmetry).
///
/// Bridge collision is deferred — see module-level notes.
pub(super) fn move_gas(p: &mut Particle, pt: &ParticleType, tick: u64) {
    move_gas_with_wind(p, pt, tick, gas_wind_dir());
}

/// Internal helper exposed for testing — lets a test pin a specific wind
/// direction without touching global rules state.
pub(super) fn move_gas_with_wind(p: &mut Particle, pt: &ParticleType, tick: u64, wind_dir: u8) {
    // Gas movement only progresses on odd frames.
    if tick & 1 == 0 {
        return;
    }
    let we = pt.wind_effect as u64;
    if we > 0 {
        let period = (10 / we).max(1);
        if tick % period == 0 {
            let idx = (wind_dir as usize).min(7);
            p.coords.x += GAS_WIND_DX[idx];
            p.coords.y += GAS_WIND_DY[idx];
        }
    }
    p.coords.x += p.drift_x;
    p.coords.y += p.drift_y;
    p.coords.z += p.drift_z;
}

/// Default gas wind direction. Real value comes from `[General] WindDirection=`
/// once that's parsed; until then everything stays at index 0 (north).
fn gas_wind_dir() -> u8 {
    0
}

struct ChildSpec {
    next_id: ParticleTypeId,
    coords: IVec3,
    velocity: SimFixed,
    translucency: u8,
    drift_x: i32,
    drift_y: i32,
    drift_z: i32,
}

fn make_child(spec: ChildSpec, pt: &ParticleType, rng: &mut SimRng) -> Particle {
    let mut p = make_particle(spec.next_id, spec.coords, spec.coords, pt, rng);
    p.velocity = spec.velocity;
    p.translucency = spec.translucency;
    p.drift_x = spec.drift_x;
    p.drift_y = spec.drift_y;
    p.drift_z = spec.drift_z;
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
    fn next_particle_spawns_one_child_with_velocity_copy() {
        // Gas chain GasA → GasB. Parent dies this tick; system must spawn
        // exactly ONE child carrying parent's velocity + drift.
        let rules = parse(
            "[Particles]\n\
             1=GasA\n\
             2=GasB\n\
             [GasA]\n\
             BehavesLike=Gas\n\
             MaxEC=1\n\
             NextParticle=GasB\n\
             [GasB]\n\
             BehavesLike=Gas\n\
             MaxEC=10\n\
             [ParticleSystems]\n\
             1=Sys\n\
             [Sys]\n\
             BehavesLike=Gas\n\
             HoldsWhat=GasA\n\
             ParticleCap=20\n",
        );
        let mut sim = Simulation::new();
        let mut sys = fake_system(ParticleSystemTypeId(0));
        let pt_a = rules.particle_type(ParticleTypeId(0));
        let mut parent = make_particle(
            ParticleTypeId(0),
            IVec3::new(1000, 2000, 0),
            IVec3::ZERO,
            pt_a,
            sim.particle_rng(),
        );
        parent.lifetime_remaining = 1;
        parent.velocity = SimFixed::from_num(3);
        parent.drift_x = 7;
        parent.drift_y = -4;
        parent.drift_z = 11;
        sys.particles.push(parent);

        tick_system(&mut sys, &mut sim, &rules);

        // Single-child chain: exactly one child, original parent removed.
        assert_eq!(sys.particles.len(), 1, "single-child finding");
        let child = &sys.particles[0];
        assert_eq!(child.type_id, ParticleTypeId(1));
        // Parent's velocity carries to the child.
        assert_eq!(child.velocity, SimFixed::from_num(3));
        // Parent's drift carries too — a moving plume keeps drifting.
        assert_eq!(child.drift_x, 7);
        assert_eq!(child.drift_y, -4);
        assert_eq!(child.drift_z, 11);
        // Child spawns at parent's position (NextParticleOffset defaults to 0,0,0).
        assert_eq!(child.coords, IVec3::new(1000, 2000, 0));
    }

    #[test]
    fn damage_countdown_resets_from_max_dc() {
        // MaxDC=5: ticking exactly 5 frames should land the counter back at 5
        // (decrement each tick, hit zero on tick 5, reset to MaxDC).
        let rules = parse(
            "[Particles]\n\
             1=Gas\n\
             [Gas]\n\
             BehavesLike=Gas\n\
             MaxDC=5\n\
             MaxEC=1000\n\
             Damage=10\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::ZERO,
            IVec3::ZERO,
            pt,
            sim.particle_rng(),
        );
        // make_particle initializes damage_counter = pt.max_dc.
        assert_eq!(p.damage_counter, 5);
        for _ in 0..5 {
            tick_particle(&mut p, pt, 0);
        }
        assert_eq!(
            p.damage_counter, 5,
            "after exactly MaxDC ticks, counter resets to MaxDC"
        );
    }

    #[test]
    fn move_gas_uses_gas_wind_table_se_dx_is_one() {
        // SE wind direction (idx 3): gas DX=1, smoke DX=2. wind_effect=2 gives
        // period = 10/2 = 5; tick=5 is odd AND satisfies tick % 5 == 0, so the
        // wind apply triggers and we read GAS_WIND_DX[3] (== 1), not the smoke
        // value.
        let rules = parse(
            "[Particles]\n\
             1=Gas\n\
             [Gas]\n\
             BehavesLike=Gas\n\
             WindEffect=2\n\
             MaxEC=10\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::ZERO,
            IVec3::ZERO,
            pt,
            sim.particle_rng(),
        );
        move_gas_with_wind(&mut p, pt, 5, 3);
        assert_eq!(p.coords.x, 1, "GAS_WIND_DX[3] == 1");
        assert_eq!(p.coords.y, 2, "GAS_WIND_DY[3] == 2");
    }

    #[test]
    fn gas_spawn_cap_enforced() {
        // Cap=5 — even with aggressive spawning, particle count must stay ≤ 5.
        let rules = parse(
            "[Particles]\n\
             1=Gas\n\
             [Gas]\n\
             BehavesLike=Gas\n\
             MaxEC=1000\n\
             [ParticleSystems]\n\
             1=Sys\n\
             [Sys]\n\
             BehavesLike=Gas\n\
             HoldsWhat=Gas\n\
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
}
