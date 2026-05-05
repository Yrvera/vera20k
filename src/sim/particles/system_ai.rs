//! Per-tick particle-system AI dispatch.
//!
//! Drives every `ParticleSystem` in the store forward by one tick: pulls each
//! system out of the store, runs its per-`BehavesLike` AI, decrements lifetime,
//! then either drops or reinserts. Smoke (C2), Gas (C3), and Fire (C4) have
//! full bodies; the Tier-3 variants (Spark, Railgun) are still no-ops.
//!
//! Pulling each system out before ticking lets the inner AI take `&mut Simulation`
//! freely — needed for spawning child systems and applying damage to entities —
//! without aliasing the store. `reinsert` puts the system back at its original
//! `stable_id` so deterministic iteration order is preserved.

use crate::rules::particle_system_type::ParticleSystemBehavesLike;
use crate::rules::ruleset::RuleSet;
use crate::sim::particles::ParticleSystem;
use crate::sim::world::Simulation;

pub fn tick_particle_systems(sim: &mut Simulation, rules: &RuleSet) {
    let ids = sim.particle_systems.ids();
    for id in ids {
        let Some(mut sys) = sim.particle_systems.remove(id) else {
            continue;
        };

        tick_one_system(&mut sys, sim, rules);

        sys.lifetime -= 1;
        if sys.lifetime == 0 {
            sys.marked_for_deletion = true;
        }

        if sys.marked_for_deletion && sys.particles.is_empty() {
            // Dropped — already removed from store above.
            continue;
        }
        sim.particle_systems.reinsert(sys);
    }
}

fn tick_one_system(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    let behaves_like = rules.particle_system_type(sys.type_id).behaves_like;
    match behaves_like {
        ParticleSystemBehavesLike::Smoke => tick_smoke(sys, sim, rules),
        ParticleSystemBehavesLike::Gas => tick_gas(sys, sim, rules),
        ParticleSystemBehavesLike::Fire => tick_fire(sys, sim, rules),
        ParticleSystemBehavesLike::Spark | ParticleSystemBehavesLike::Railgun => {
            // Tier 3 — no-op.
        }
    }
}

fn tick_smoke(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    super::smoke::tick_system(sys, sim, rules);
}

fn tick_gas(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    super::gas::tick_system(sys, sim, rules);
}

fn tick_fire(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    super::fire::tick_system(sys, sim, rules);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::particle_system_type::ParticleSystemTypeId;
    use crate::sim::particles::ParticleSystem;
    use crate::util::fixed_math::SimFixed;
    use glam::IVec3;

    fn empty_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str("")).expect("empty rules parse")
    }

    fn build_rules(behaves_like: &str, lifetime: i32) -> RuleSet {
        let ini_text = format!(
            "[ParticleSystems]\n\
             1=Sys\n\
             [Sys]\n\
             BehavesLike={behaves_like}\n\
             Lifetime={lifetime}\n",
        );
        RuleSet::from_ini(&IniFile::from_str(&ini_text)).expect("rules parse")
    }

    fn fake_system(type_id: ParticleSystemTypeId, lifetime: i32) -> ParticleSystem {
        ParticleSystem {
            stable_id: 0,
            type_id,
            coords: IVec3::ZERO,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(0),
            lifetime,
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

    #[test]
    fn empty_store_tick_is_a_no_op() {
        let mut sim = Simulation::new();
        let rules = empty_rules();
        tick_particle_systems(&mut sim, &rules);
        assert_eq!(sim.particle_systems.len(), 0);
    }

    #[test]
    fn finite_lifetime_drops_when_countdown_hits_zero() {
        let rules = build_rules("Smoke", 3);
        let mut sim = Simulation::new();
        sim.particle_systems
            .insert(fake_system(ParticleSystemTypeId(0), 1));
        // Tick once: lifetime 1 → 0, marked, particles empty → dropped.
        tick_particle_systems(&mut sim, &rules);
        assert_eq!(sim.particle_systems.len(), 0);
    }

    #[test]
    fn infinite_lifetime_survives_when_empty() {
        let rules = build_rules("Smoke", -1);
        let mut sim = Simulation::new();
        sim.particle_systems
            .insert(fake_system(ParticleSystemTypeId(0), -1));
        // -1 → -2, never == 0; marked stays false; survives.
        tick_particle_systems(&mut sim, &rules);
        assert_eq!(sim.particle_systems.len(), 1);
    }

    #[test]
    fn reinsert_preserves_stable_id() {
        let rules = build_rules("Smoke", 100);
        let mut sim = Simulation::new();
        let id = sim
            .particle_systems
            .insert(fake_system(ParticleSystemTypeId(0), 100));
        tick_particle_systems(&mut sim, &rules);
        assert!(sim.particle_systems.get(id).is_some());
        assert_eq!(sim.particle_systems.get(id).unwrap().lifetime, 99);
    }

    #[test]
    fn spark_and_railgun_dispatch_is_a_no_op() {
        for behaves in ["Spark", "Railgun"] {
            let rules = build_rules(behaves, 100);
            let mut sim = Simulation::new();
            sim.particle_systems
                .insert(fake_system(ParticleSystemTypeId(0), 100));
            tick_particle_systems(&mut sim, &rules);
            assert_eq!(sim.particle_systems.len(), 1, "{behaves} should survive");
        }
    }
}
