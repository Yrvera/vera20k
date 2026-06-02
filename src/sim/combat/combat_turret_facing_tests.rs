//! Integration tests for turret rotation + fire decision parity.
//!
//! Verifies the FacingClass-driven combat behavior end-to-end through
//! `Simulation::advance_tick`, covering 1-tick acquisition latency,
//! mid-rotation retarget, slow vs fast ROT alignment timing, and the
//! flipped Phase 5 tick order.

use std::collections::BTreeMap;

use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::AttackTarget;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::FacingClass;
use crate::sim::movement::turret::body_facing_to_turret;
use crate::sim::world::Simulation;

fn empty_height_map() -> BTreeMap<(u16, u16), u8> {
    BTreeMap::new()
}

/// Minimal rules with MTNK at the given ROT byte. tick_turret_rotation
/// re-applies this each tick via barrel.set_rot, so it drives the
/// per-test rotation rate.
fn rules_with_mtnk_rot(rot: u32) -> RuleSet {
    let ini_str: String = format!(
        "[VehicleTypes]\n0=MTNK\n\n\
[InfantryTypes]\n0=ENGI\n\n\
[BuildingTypes]\n0=GAPILE\n\n\
[AircraftTypes]\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\nROT={rot}\n\n\
[ENGI]\nStrength=75\nArmor=none\nSpeed=4\n\n\
[GAPILE]\nStrength=300\nArmor=heavy\n\n\
[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n"
    );
    let ini: IniFile = IniFile::from_str(&ini_str);
    RuleSet::from_ini(&ini).expect("rules_with_mtnk_rot should parse")
}

/// Spawn a turreted attacker at (rx, ry) facing north (0) with the given ROT byte.
fn spawn_turreted(sim: &mut Simulation, stable_id: u64, rx: u16, ry: u16, rot_byte: u8) {
    let mut entity = GameEntity::test_default(stable_id, "MTNK", "Americans", rx, ry);
    entity.barrel_facing = Some(FacingClass::new(body_facing_to_turret(0), rot_byte));
    sim.substrate.entities.insert(entity);
}

/// Spawn a passive target at (rx, ry).
fn spawn_target(sim: &mut Simulation, stable_id: u64, rx: u16, ry: u16) {
    let entity = GameEntity::test_default(stable_id, "GAPILE", "Soviet", rx, ry);
    sim.substrate.entities.insert(entity);
}

/// Replace sim's interner with the thread-local test interner so entity
/// type_ref / owner IDs from `GameEntity::test_default` (which uses
/// `test_intern`) resolve correctly inside sim functions.
fn use_test_interner(sim: &mut Simulation) {
    sim.interner = crate::sim::intern::test_interner();
}

#[test]
fn one_tick_acquisition_latency_first_tick_no_fire() {
    // After issuing an attack, the binary takes 1+ frames to rotate the turret
    // before firing (combat reads last-frame's facing). Even with ROT large
    // enough to fully rotate in 1 frame, the FIRST tick after target-set
    // produces no fire because combat ran BEFORE turret_rotation.
    let mut sim = Simulation::new();
    spawn_turreted(&mut sim, 1, 5, 5, 100); // ROT=100 → rot_per_frame=25600
    spawn_target(&mut sim, 2, 8, 5);
    use_test_interner(&mut sim);
    let rules = rules_with_mtnk_rot(100);

    // Attach attack_target so combat will try to fire on the next tick.
    if let Some(e) = sim.substrate.entities.get_mut(1) {
        e.attack_target = Some(AttackTarget::new(2));
    }

    let initial_target_health = sim.substrate.entities.get(2).unwrap().health.current;
    sim.advance_tick(&[], Some(&rules), &empty_height_map(), None, None, 67);

    // Target should still be alive — combat ran before turret rotation, so
    // turret was at facing 0 (body), not aligned with target.
    let target_health_after_one_tick = sim.substrate.entities.get(2).unwrap().health.current;
    assert_eq!(
        target_health_after_one_tick, initial_target_health,
        "First tick after acquisition should not fire (1-tick latency)"
    );
}

#[test]
fn slow_rot_takes_more_frames_to_align_than_fast_rot() {
    // ROT=1 vs ROT=10: same acquisition geometry, the slow turret takes
    // proportionally more binary frames to align. Fixes the current
    // is_turret_aligned_u16 flat-tolerance bug.
    let mut sim_slow = Simulation::new();
    let mut sim_fast = Simulation::new();
    spawn_turreted(&mut sim_slow, 1, 5, 5, 1); // ROT=1 → rot_per_frame=256
    spawn_turreted(&mut sim_fast, 1, 5, 5, 10); // ROT=10 → rot_per_frame=2560
    spawn_target(&mut sim_slow, 2, 5, 8); // 3 cells south
    spawn_target(&mut sim_fast, 2, 5, 8);
    use_test_interner(&mut sim_slow);
    use_test_interner(&mut sim_fast);
    let rules_slow = rules_with_mtnk_rot(1);
    let rules_fast = rules_with_mtnk_rot(10);

    // Attach attack_target on both.
    sim_slow.substrate.entities.get_mut(1).unwrap().attack_target = Some(AttackTarget::new(2));
    sim_fast.substrate.entities.get_mut(1).unwrap().attack_target = Some(AttackTarget::new(2));

    // Compute the expected duration: from facing 0 (north, after body_facing_to_turret(0))
    // to facing south (~32768). Diff = 32768. ROT=1: duration = 32768/256 = 128 frames.
    // ROT=10: duration = 32768/2560 = 12 frames.
    // Run 13 binary frames worth of ticks. Fast turret should be done; slow not.

    // Each 67ms tick advances binary_frame by ~1.
    for _ in 0..13 {
        sim_slow.advance_tick(&[], Some(&rules_slow), &empty_height_map(), None, None, 67);
        sim_fast.advance_tick(&[], Some(&rules_fast), &empty_height_map(), None, None, 67);
    }

    let slow_rotating = sim_slow
        .substrate.entities
        .get(1)
        .unwrap()
        .barrel_facing
        .as_ref()
        .map(|f| f.is_rotating(sim_slow.binary_frame))
        .unwrap_or(false);
    let fast_rotating = sim_fast
        .substrate.entities
        .get(1)
        .unwrap()
        .barrel_facing
        .as_ref()
        .map(|f| f.is_rotating(sim_fast.binary_frame))
        .unwrap_or(false);

    assert!(
        slow_rotating,
        "ROT=1 turret should still be rotating after 13 frames"
    );
    assert!(
        !fast_rotating,
        "ROT=10 turret should be done rotating after 13 frames"
    );
}

#[test]
fn idle_turret_returns_to_body_facing() {
    // No attack_target, body facing east (64) — turret should rotate to match.
    let mut sim = Simulation::new();
    let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
    entity.facing = 64; // body east
    entity.barrel_facing = Some(FacingClass::new(body_facing_to_turret(0), 100));
    // ROT=100 → rot_per_frame=25600. Diff from 0 (north turret) to body_facing_to_turret(64) =
    // 64*256 = 16384. Duration = 16384/25600 = 0 → snaps in 1 frame.
    sim.substrate.entities.insert(entity);
    use_test_interner(&mut sim);
    let rules = rules_with_mtnk_rot(100);

    // Run 2 ticks to ensure turret_rotation has had a chance to act.
    sim.advance_tick(&[], Some(&rules), &empty_height_map(), None, None, 67);
    sim.advance_tick(&[], Some(&rules), &empty_height_map(), None, None, 67);

    let barrel = sim.substrate.entities.get(1).unwrap().barrel_facing.as_ref().unwrap();
    assert_eq!(
        barrel.destination(),
        body_facing_to_turret(64),
        "Idle turret should target body facing"
    );
}

#[test]
fn mid_rotation_retarget_snapshots_into_prev() {
    // Start a rotation, advance partway, set a new target. The new prev
    // should equal the animated value at the moment of the new set (not the
    // original prev) — visible smoothness of mid-rotation retarget.
    let mut fc = FacingClass::new(0, 5);
    fc.set(12800, 0); // rotation 0 → 12800 over 10 frames.
    let animated_at_5 = fc.current(5);
    fc.set(25600, 5); // retarget mid-rotation.

    // After re-set, prev should equal the animated value at frame 5, NOT 0.
    assert_eq!(
        fc.current(5),
        animated_at_5,
        "Animated value immediately after re-set should equal pre-set animated value (no jump)"
    );
}
