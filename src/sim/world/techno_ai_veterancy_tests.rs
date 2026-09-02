//! Promotion announcement and self-heal pulse at the per-object AI head.

use super::*;
use crate::map::playfield::PlayfieldBounds;
use crate::rules::ini_parser::IniFile;
use crate::sim::combat::veterancy::{self, LEVEL_ELITE, LEVEL_ROOKIE, LEVEL_UNSAMPLED};
use crate::sim::world::SimSoundEvent;

fn rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[General]\nRepairRate=.016\n\
         [AudioVisual]\nUpgradeVeteranSound=UpgradeVeteran\nUpgradeEliteSound=UpgradeElite\nEliteFlashTimer=150\n\
         [VehicleTypes]\n0=MTNK\n\
         [MTNK]\nStrength=300\nSpeed=6\nCost=700\nVeteranAbilities=STRONGER\nEliteAbilities=SELF_HEAL\n",
    ))
    .expect("veterancy AI rules")
}

fn spawned_grizzly() -> (Simulation, RuleSet, u64) {
    let rules = rules();
    let mut sim = Simulation::with_seed(0x5E7E_2A4C);
    sim.fog.width = 64;
    sim.fog.height = 64;
    sim.playfield_bounds = Some(PlayfieldBounds::from_normalized_local_size(
        64, 2, 2, 56, 52,
    ));
    let bounds = sim.playfield_bounds.unwrap();
    let (rx, ry) = (8u16..56)
        .flat_map(|ry| (8u16..56).map(move |rx| (rx, ry)))
        .find(|&(rx, ry)| bounds.contains_height_aware_packed(rx.into(), ry.into(), 0, 0))
        .expect("interior mode-one cell");
    let id = sim
        .spawn_object_at_height("MTNK", "Soviet", rx, ry, 0, 0, &rules)
        .unwrap();
    (sim, rules, id)
}

/// `AI_Update @ 0x006FA054..0x006FA145`: silent first sample, one cue per
/// crossing carrying the matching `[AudioVisual]` sound, the elite flash seed
/// on the elite crossing only, and the countdown afterwards.
#[test]
fn gsi_08_12_promotion_step_announces_each_crossing_once() {
    let (mut sim, rules, id) = spawned_grizzly();
    assert_eq!(
        sim.substrate.entities.get(id).unwrap().veterancy_rank_cache,
        LEVEL_UNSAMPLED
    );
    veterancy_promotion_step(&mut sim, id, &rules);
    assert!(
        sim.sound_events.is_empty(),
        "the first sample caches silently"
    );
    assert_eq!(
        sim.substrate.entities.get(id).unwrap().veterancy_rank_cache,
        LEVEL_ROOKIE
    );

    veterancy::set_veteran(sim.substrate.entities.get_mut(id).unwrap());
    veterancy_promotion_step(&mut sim, id, &rules);
    let owner = sim.substrate.entities.get(id).unwrap().owner;
    assert!(matches!(
        sim.sound_events.as_slice(),
        [SimSoundEvent::UnitPromoted {
            owner: o,
            sound_id: Some(sound),
            elite: false,
            ..
        }] if *o == owner && sim.interner.resolve(*sound) == "UpgradeVeteran"
    ));
    assert_eq!(
        sim.substrate.entities.get(id).unwrap().elite_flash_frames,
        0
    );
    veterancy_promotion_step(&mut sim, id, &rules);
    assert_eq!(sim.sound_events.len(), 1, "no repeat while the rank holds");

    veterancy::set_elite(sim.substrate.entities.get_mut(id).unwrap());
    veterancy_promotion_step(&mut sim, id, &rules);
    assert!(matches!(
        sim.sound_events.last(),
        Some(SimSoundEvent::UnitPromoted {
            sound_id: Some(sound),
            elite: true,
            ..
        }) if sim.interner.resolve(*sound) == "UpgradeElite"
    ));
    let entity = sim.substrate.entities.get(id).unwrap();
    assert_eq!(entity.veterancy_rank_cache, LEVEL_ELITE);
    assert_eq!(entity.elite_flash_frames, 150);
    veterancy_promotion_step(&mut sim, id, &rules);
    assert_eq!(
        sim.substrate.entities.get(id).unwrap().elite_flash_frames,
        149
    );
}

/// `AI_Update @ 0x006FA743..0x006FA757`: `+1` health on the shared
/// `frame % ftol(RepairRate * 900)` pulse, for an eligible object only.
#[test]
fn gsi_08_12_self_heal_pulses_one_point_on_the_repair_rate_cadence() {
    let (mut sim, rules, id) = spawned_grizzly();
    sim.substrate.entities.get_mut(id).unwrap().health.current = 100;
    sim.session.binary_frame = 14;
    self_heal_step(&mut sim, id, &rules);
    assert_eq!(
        sim.substrate.entities.get(id).unwrap().health.current,
        100,
        "a rookie without SelfHealing= never pulses"
    );

    veterancy::set_elite(sim.substrate.entities.get_mut(id).unwrap());
    self_heal_step(&mut sim, id, &rules);
    assert_eq!(sim.substrate.entities.get(id).unwrap().health.current, 101);
    sim.session.binary_frame = 15;
    self_heal_step(&mut sim, id, &rules);
    assert_eq!(
        sim.substrate.entities.get(id).unwrap().health.current,
        101,
        "off-cadence frames do nothing"
    );
    sim.session.binary_frame = 28;
    self_heal_step(&mut sim, id, &rules);
    assert_eq!(sim.substrate.entities.get(id).unwrap().health.current, 102);
}
