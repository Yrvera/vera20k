//! GeneticConverter superweapon launch handler.
//!
//! Mutates infantry in target area into Brutes. Two code paths depending on
//! Rules->MutateExplosion: either AoE via MutateExplosionWarhead, or per-cell
//! legacy mutation applied to marked infantry in a 3-by-3 grid. After either
//! mutation path, attempts a Brute replacement at each selected death position.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/combat/combat_aoe, sim/superweapon/cell_grid,
//!   sim/game_entity, sim/components, sim/world.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

mod mutation;

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::WorldEffect;
use crate::sim::intern::InternedId;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Launch GeneticConverter at (target_rx, target_ry). Mutates infantry in area.
pub fn launch(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
    sw_type: InternedId,
    overlay_registry: Option<&OverlayTypeRegistry>,
) -> bool {
    // Spawn the launch animation before mutation.
    spawn_invoke_anim(sim, rules, "IONBLAST", target_rx, target_ry);

    let kill_count = mutation::execute(sim, rules, owner, target_rx, target_ry, overlay_registry);

    // Publish the launch sound after replacement admission.
    sim.sound_events.push(SimSoundEvent::SuperWeaponLaunched {
        owner,
        sw_type,
        rx: target_rx,
        ry: target_ry,
    });

    log::info!(
        "GeneticConverter launched at ({}, {}) by '{}', {} infantry mutated",
        target_rx,
        target_ry,
        sim.interner.resolve(owner),
        kill_count
    );

    true
}

fn spawn_invoke_anim(sim: &mut Simulation, rules: &RuleSet, anim_name: &str, rx: u16, ry: u16) {
    let frames = rules.effect_frame_count(anim_name).unwrap_or(20);
    let iid = sim.interner.intern(anim_name);
    sim.world_effects.push(WorldEffect {
        anim_spawn: None,
        shp_name: iid,
        rx,
        ry,
        sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
        sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        z: 5,
        frame: 0,
        total_frames: frames,
        frame_delay: 1,
        elapsed_frames: 0,
        translucent: false,
        delay_frames: 0,
        start_sound_id: None,
        start_sound_emitted: false,
    });
}
