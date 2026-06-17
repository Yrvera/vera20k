//! Integration test: spawn one BigGreySmokeSys, advance ticks, verify the
//! state-AI advance progresses animation_state on at least one live particle.

use glam::IVec3;
use vera20k::rules::ini_parser::IniFile;
use vera20k::rules::particle_system_type::ParticleSystemTypeId;
use vera20k::rules::ruleset::RuleSet;
use vera20k::sim::particles::system_ai::tick_particle_systems;
use vera20k::sim::world::Simulation;

/// Minimal RuleSet with one Smoke system + type, mimicking BigGreySmokeSys.
fn build_smoke_rules() -> RuleSet {
    let ini_text = "[Particles]\n\
1=LargeGreySmoke\n\
[LargeGreySmoke]\n\
BehavesLike=Smoke\n\
Image=LGRYSMK1\n\
MaxEC=80\n\
Translucency=50\n\
EndStateAI=20\n\
StateAIAdvance=4\n\
DeleteOnStateLimit=yes\n\
[ParticleSystems]\n\
1=BigGreySmokeSys\n\
[BigGreySmokeSys]\n\
BehavesLike=Smoke\n\
HoldsWhat=LargeGreySmoke\n\
Spawns=yes\n\
SpawnFrames=10\n\
ParticleCap=15\n";
    let ini = IniFile::from_str(ini_text);
    RuleSet::from_ini(&ini).expect("rules parse")
}

#[test]
fn smoke_animation_state_advances_over_ticks() {
    let rules = build_smoke_rules();
    let mut sim = Simulation::new();

    // Pre-populate effect_frame_counts as if the atlas had registered
    // LGRYSMK1 with 21 frames (matches stock YR).
    let id = sim.intern("LGRYSMK1");
    sim.effect_frame_counts.insert(id, 21);

    let _sys_id = sim
        .spawn_particle_system(
            ParticleSystemTypeId(0),
            IVec3::new(1024, 1024, 0),
            None,
            None,
            IVec3::ZERO,
            None,
            &rules,
        )
        .expect("spawn");

    // Advance enough ticks to spawn at least one particle and let
    // animation_state advance from 0.
    // SpawnFrames=10 → first particle at tick 10.
    // image_frame_count=21 (odd) → denom = (1+1) + 4 = 6.
    for _ in 0..30 {
        tick_particle_systems(&mut sim, &rules);
        sim.session.tick += 1;
    }

    // The system was fed `_sys_id`, but tick_particle_systems uses
    // remove → tick → reinsert; pull whichever system survives by walking
    // the store (single system in this test).
    let sys = sim
        .particle_systems
        .iter()
        .next()
        .map(|(_id, sys)| sys)
        .expect("system alive");
    assert!(!sys.particles.is_empty(), "should have spawned particles");
    let p = &sys.particles[0];
    assert!(p.animation_state > 0, "animation_state should advance");
}
