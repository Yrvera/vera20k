//! Particle spawn helpers.
//!
//! Three entry points:
//!   - `Simulation::spawn_particle_system` — public API for combat / refinery /
//!     gap-gen / area-damage to create a new system at a world coord.
//!   - `spawn_particle` — append one particle to a system's vector, capped by
//!     `ParticleSystemType::particle_cap`. Used by per-tick system AI.
//!   - `spawn_particle_with_insert` — Fire-only variant: append, then random-
//!     shuffle within the last `insert_range` slots so the visual stream has
//!     variety instead of strict FIFO.
//!
//! Tier 3 system types (`Spark`, `Railgun`) are accepted by the public entry
//! point but logged + skipped — runtime spawn returns `None`.

use super::{Particle, ParticleSystem};
use crate::rules::particle_system_type::{ParticleSystemBehavesLike, ParticleSystemTypeId};
use crate::rules::particle_type::ParticleBehavesLike;
use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;
use crate::util::fixed_math::{SIM_ONE, SIM_ZERO, SimFixed};
use fixed::types::I48F16;
use glam::IVec3;

impl Simulation {
    /// Spawn a new particle system. Returns the new system's stable id, or
    /// `None` if the type is `Spark` or `Railgun` (Tier 3 — not implemented).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_particle_system(
        &mut self,
        type_id: ParticleSystemTypeId,
        coords: IVec3,
        attached_entity: Option<u64>,
        owner_entity: Option<u64>,
        target_coords: IVec3,
        owner_house: Option<InternedId>,
        rules: &RuleSet,
    ) -> Option<u64> {
        let pst = rules.particle_system_type(type_id);
        match pst.behaves_like {
            ParticleSystemBehavesLike::Spark | ParticleSystemBehavesLike::Railgun => {
                log::warn!(
                    "particles: Tier 3 PSC type {:?} requested at {:?} — skipped",
                    pst.behaves_like,
                    coords,
                );
                return None;
            }
            _ => {}
        }
        let directionless = pst.spawn_direction == IVec3::ZERO;
        let sys = ParticleSystem {
            stable_id: 0,
            type_id,
            coords,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(pst.spawn_frames as i32),
            lifetime: pst.lifetime,
            spark_spawn_frames: pst.spark_spawn_frames as i32,
            facing: 0x1D,
            marked_for_deletion: false,
            directionless,
            attached_entity,
            owner_entity,
            target_coords,
            owner_house,
            done_spawning: false,
        };
        Some(self.particle_systems.insert(sys))
    }
}

/// Append one particle to `sys.particles`. Returns `false` when the system's
/// type has no `HoldsWhat` set or its particle cap is already reached.
pub(super) fn spawn_particle(
    sys: &mut ParticleSystem,
    coords: IVec3,
    spawn_origin: IVec3,
    rules: &RuleSet,
    rng: &mut SimRng,
) -> bool {
    let pst = rules.particle_system_type(sys.type_id);
    let Some(pt_id) = pst.holds_what else {
        return false;
    };
    if sys.particles.len() >= pst.particle_cap as usize {
        return false;
    }
    let pt = rules.particle_type(pt_id);
    let direction = normalized_direction(coords, sys.target_coords);
    let state_ai_advance = spawn_state_ai_advance(pt, coords, sys.target_coords, direction);

    let lifetime_extra = if pt.behaves_like == ParticleBehavesLike::Railgun {
        rng.next_range_u32(10) as i16
    } else {
        let base = (pt.max_ec as u32).max(1);
        rng.next_range_u32(base) as i16
    };
    let lifetime_remaining = (pt.max_ec as i16).saturating_add(lifetime_extra);

    sys.particles.push(Particle {
        type_id: pt_id,
        coords,
        previous_coords: spawn_origin,
        origin: coords,
        direction,
        velocity: pt.velocity,
        lifetime_remaining,
        damage_counter: pt.max_dc as i16,
        state_ai_advance,
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
        prev_delta: [SimFixed::from_num(0); 3],
        state_advance_counter: 0,
    });
    true
}

fn normalized_direction(source: IVec3, target: IVec3) -> [SimFixed; 3] {
    let dx = target.x - source.x;
    let dy = target.y - source.y;
    let dz = target.z - source.z;
    if dx == 0 && dy == 0 && dz == 0 {
        return [SIM_ZERO; 3];
    }

    let dx_w = I48F16::from_num(dx);
    let dy_w = I48F16::from_num(dy);
    let dz_w = I48F16::from_num(dz);
    let dist = sqrt_i48(dx_w * dx_w + dy_w * dy_w + dz_w * dz_w);
    if dist <= I48F16::ZERO {
        return [SIM_ZERO; 3];
    }

    [
        i48_to_sim(dx_w / dist),
        i48_to_sim(dy_w / dist),
        i48_to_sim(dz_w / dist),
    ]
}

fn spawn_state_ai_advance(
    pt: &crate::rules::particle_type::ParticleType,
    source: IVec3,
    target: IVec3,
    direction: [SimFixed; 3],
) -> u8 {
    if !pt.normalized {
        return pt.state_ai_advance;
    }

    let step_x = ftol_chop(direction[0] * pt.velocity).abs();
    let step_y = ftol_chop(direction[1] * pt.velocity).abs();
    let mut best_ticks = SimFixed::from_num(9999);

    if step_x > 0 {
        best_ticks = SimFixed::from_num((source.x - target.x).abs()) / SimFixed::from_num(step_x);
    }
    if step_y > 0 {
        let y_ticks = SimFixed::from_num((source.y - target.y).abs()) / SimFixed::from_num(step_y);
        if best_ticks >= y_ticks {
            best_ticks = y_ticks;
        }
    }

    let divisor = SimFixed::from_num(u16::from(pt.final_damage_state) + 1);
    let advance: i32 = ftol_chop(best_ticks / divisor + SIM_ONE);
    advance as u8
}

fn ftol_chop(val: SimFixed) -> i32 {
    let bits = i64::from(val.to_bits());
    if bits >= 0 {
        (bits >> 16) as i32
    } else {
        -((-bits) >> 16) as i32
    }
}

fn sqrt_i48(val: I48F16) -> I48F16 {
    if val <= I48F16::ZERO {
        return I48F16::ZERO;
    }
    let two = I48F16::from_num(2);
    let mut guess = if val < two { val } else { val / two };
    for _ in 0..16 {
        if guess <= I48F16::ZERO {
            return I48F16::ZERO;
        }
        guess = (guess + val / guess) / two;
    }
    guess
}

fn i48_to_sim(val: I48F16) -> SimFixed {
    let bits = val.to_bits().clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    SimFixed::from_bits(bits)
}

/// Fire-only variant: spawn one particle, then random-shuffle it into the last
/// `insert_range` slots so the stream looks varied. Returns `false` if the
/// underlying `spawn_particle` failed (cap reached or no `HoldsWhat`).
pub(super) fn spawn_particle_with_insert(
    sys: &mut ParticleSystem,
    coords: IVec3,
    spawn_origin: IVec3,
    insert_range: usize,
    rules: &RuleSet,
    rng: &mut SimRng,
) -> bool {
    if insert_range == 0 || !spawn_particle(sys, coords, spawn_origin, rules, rng) {
        return false;
    }
    let count = sys.particles.len();
    if count < 2 {
        return true;
    }
    let actual_range = insert_range.min(count);
    let random_offset = rng.next_range_u32(actual_range as u32) as usize;
    let insert_pos = count.saturating_sub(2).saturating_sub(random_offset);
    if insert_pos + 1 >= count {
        return true;
    }
    let p = sys.particles.pop().unwrap();
    sys.particles.insert(insert_pos + 1, p);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    /// Build a tiny RuleSet with one ParticleType + one ParticleSystemType.
    /// `behaves_like` selects which BehavesLike to assign on the system.
    /// `particle_cap` lets each test pin its own cap independently of the default.
    fn build_rules(behaves_like: &str, particle_cap: u32) -> RuleSet {
        let ini_text = format!(
            "[Particles]\n\
             1=Smk\n\
             [ParticleSystems]\n\
             1=Sys\n\
             [Smk]\n\
             BehavesLike=Smoke\n\
             MaxEC=10\n\
             MaxDC=4\n\
             StartStateAI=0\n\
             EndStateAI=10\n\
             StateAIAdvance=4\n\
             Translucency=0\n\
             [Sys]\n\
             BehavesLike={behaves_like}\n\
             HoldsWhat=Smk\n\
             ParticleCap={particle_cap}\n\
             SpawnFrames=1\n\
             Lifetime=200\n",
        );
        let ini = IniFile::from_str(&ini_text);
        RuleSet::from_ini(&ini).expect("rules parse")
    }

    #[test]
    fn spawn_returns_none_for_spark_at_tier_2() {
        let rules = build_rules("Spark", 50);
        let mut sim = Simulation::new();
        let result = sim.spawn_particle_system(
            ParticleSystemTypeId(0),
            IVec3::ZERO,
            None,
            None,
            IVec3::ZERO,
            None,
            &rules,
        );
        assert!(result.is_none());
        assert_eq!(sim.particle_systems.len(), 0);
    }

    #[test]
    fn spawn_returns_none_for_railgun_at_tier_2() {
        let rules = build_rules("Railgun", 50);
        let mut sim = Simulation::new();
        let result = sim.spawn_particle_system(
            ParticleSystemTypeId(0),
            IVec3::ZERO,
            None,
            None,
            IVec3::ZERO,
            None,
            &rules,
        );
        assert!(result.is_none());
    }

    #[test]
    fn spawn_returns_some_for_smoke() {
        let rules = build_rules("Smoke", 50);
        let mut sim = Simulation::new();
        let id = sim.spawn_particle_system(
            ParticleSystemTypeId(0),
            IVec3::new(100, 100, 0),
            None,
            None,
            IVec3::ZERO,
            None,
            &rules,
        );
        assert!(id.is_some());
        assert_eq!(sim.particle_systems.len(), 1);
        let sys = sim.particle_systems.get(id.unwrap()).unwrap();
        assert_eq!(sys.coords, IVec3::new(100, 100, 0));
        assert_eq!(sys.lifetime, 200);
        assert_eq!(sys.facing, 0x1D);
        assert!(sys.directionless);
    }

    #[test]
    fn spawn_particle_respects_particle_cap() {
        let rules = build_rules("Smoke", 3);
        let mut sim = Simulation::new();
        let sys_id = sim
            .spawn_particle_system(
                ParticleSystemTypeId(0),
                IVec3::ZERO,
                None,
                None,
                IVec3::ZERO,
                None,
                &rules,
            )
            .unwrap();
        let mut rng = SimRng::new(1);
        let sys = sim.particle_systems.get_mut(sys_id).unwrap();
        for _ in 0..10 {
            spawn_particle(sys, IVec3::ZERO, IVec3::ZERO, &rules, &mut rng);
        }
        assert_eq!(sys.particles.len(), 3);
    }

    #[test]
    fn spawn_with_insert_does_not_exceed_cap() {
        let rules = build_rules("Fire", 5);
        let mut sim = Simulation::new();
        let sys_id = sim
            .spawn_particle_system(
                ParticleSystemTypeId(0),
                IVec3::ZERO,
                None,
                None,
                IVec3::ZERO,
                None,
                &rules,
            )
            .unwrap();
        let mut rng = SimRng::new(1);
        let sys = sim.particle_systems.get_mut(sys_id).unwrap();
        for _ in 0..10 {
            spawn_particle_with_insert(sys, IVec3::ZERO, IVec3::ZERO, 3, &rules, &mut rng);
        }
        assert_eq!(sys.particles.len(), 5);
    }

    #[test]
    fn spawn_particle_returns_false_when_holds_what_unset() {
        // [Sys] without HoldsWhat — minimal INI to leave holds_what = None.
        let ini_text = "[ParticleSystems]\n\
                        1=NoHold\n\
                        [NoHold]\n\
                        BehavesLike=Smoke\n\
                        ParticleCap=10\n";
        let ini = IniFile::from_str(ini_text);
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let mut sim = Simulation::new();
        let sys_id = sim
            .spawn_particle_system(
                ParticleSystemTypeId(0),
                IVec3::ZERO,
                None,
                None,
                IVec3::ZERO,
                None,
                &rules,
            )
            .unwrap();
        let mut rng = SimRng::new(1);
        let sys = sim.particle_systems.get_mut(sys_id).unwrap();
        assert!(!spawn_particle(
            sys,
            IVec3::ZERO,
            IVec3::ZERO,
            &rules,
            &mut rng
        ));
        assert!(sys.particles.is_empty());
    }

    #[test]
    fn normalized_particle_rewrites_state_ai_advance_from_xy_travel_time() {
        let ini_text = "[Particles]\n\
                        1=FireStream\n\
                        [FireStream]\n\
                        BehavesLike=Fire\n\
                        MaxEC=10\n\
                        Velocity=28.0\n\
                        StateAIAdvance=6\n\
                        StartStateAI=1\n\
                        EndStateAI=19\n\
                        FinalDamageState=14\n\
                        Normalized=yes\n\
                        [ParticleSystems]\n\
                        1=FireSys\n\
                        [FireSys]\n\
                        BehavesLike=Fire\n\
                        HoldsWhat=FireStream\n\
                        ParticleCap=5\n";
        let ini = IniFile::from_str(ini_text);
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let mut sim = Simulation::new();
        let sys_id = sim
            .spawn_particle_system(
                ParticleSystemTypeId(0),
                IVec3::new(0, 0, 0),
                None,
                None,
                IVec3::new(420, 0, 0),
                None,
                &rules,
            )
            .unwrap();
        let mut rng = SimRng::new(1);
        let sys = sim.particle_systems.get_mut(sys_id).unwrap();

        assert!(spawn_particle(
            sys,
            IVec3::new(0, 0, 0),
            IVec3::new(0, 0, 0),
            &rules,
            &mut rng
        ));

        let particle = &sys.particles[0];
        assert_eq!(particle.direction, [SIM_ONE, SIM_ZERO, SIM_ZERO]);
        // step_x=trunc(1*28)=28; best_ticks=420/28=15;
        // advance=trunc(15/(FinalDamageState+1) + 1)=trunc(2)=2.
        assert_eq!(particle.state_ai_advance, 2);
    }

    #[test]
    fn normalized_particle_uses_3d_direction_but_only_xy_tick_candidates() {
        let ini_text = "[Particles]\n\
                        1=FireStream\n\
                        [FireStream]\n\
                        BehavesLike=Fire\n\
                        MaxEC=10\n\
                        Velocity=28.0\n\
                        StateAIAdvance=6\n\
                        FinalDamageState=14\n\
                        Normalized=yes\n\
                        [ParticleSystems]\n\
                        1=FireSys\n\
                        [FireSys]\n\
                        BehavesLike=Fire\n\
                        HoldsWhat=FireStream\n\
                        ParticleCap=5\n";
        let ini = IniFile::from_str(ini_text);
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let mut sim = Simulation::new();
        let sys_id = sim
            .spawn_particle_system(
                ParticleSystemTypeId(0),
                IVec3::new(0, 0, 0),
                None,
                None,
                IVec3::new(300, 400, 1200),
                None,
                &rules,
            )
            .unwrap();
        let mut rng = SimRng::new(1);
        let sys = sim.particle_systems.get_mut(sys_id).unwrap();

        assert!(spawn_particle(
            sys,
            IVec3::new(0, 0, 0),
            IVec3::ZERO,
            &rules,
            &mut rng
        ));

        let particle = &sys.particles[0];
        assert!(particle.direction[2] > SIM_ZERO);
        // Full 3D normalization makes x/y component steps small:
        // trunc(300/1300*28)=6, trunc(400/1300*28)=8.
        // X and Y candidates are both 50 ticks; Z is not considered.
        // advance=trunc(50/15 + 1)=4, replacing INI StateAIAdvance=6.
        assert_eq!(particle.state_ai_advance, 4);
    }

    #[test]
    fn normalized_particle_stores_low_byte_of_rewritten_advance() {
        let ini_text = "[Particles]\n\
                        1=FireStream\n\
                        [FireStream]\n\
                        BehavesLike=Fire\n\
                        MaxEC=10\n\
                        Velocity=1.0\n\
                        StateAIAdvance=6\n\
                        FinalDamageState=0\n\
                        Normalized=yes\n\
                        [ParticleSystems]\n\
                        1=FireSys\n\
                        [FireSys]\n\
                        BehavesLike=Fire\n\
                        HoldsWhat=FireStream\n\
                        ParticleCap=5\n";
        let ini = IniFile::from_str(ini_text);
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let mut sim = Simulation::new();
        let sys_id = sim
            .spawn_particle_system(
                ParticleSystemTypeId(0),
                IVec3::ZERO,
                None,
                None,
                IVec3::new(300, 0, 0),
                None,
                &rules,
            )
            .unwrap();
        let mut rng = SimRng::new(1);
        let sys = sim.particle_systems.get_mut(sys_id).unwrap();

        assert!(spawn_particle(
            sys,
            IVec3::ZERO,
            IVec3::ZERO,
            &rules,
            &mut rng
        ));

        // advance=trunc(300/1/(0+1)+1)=301; byte store keeps 45.
        assert_eq!(sys.particles[0].state_ai_advance, 45);
    }
}
