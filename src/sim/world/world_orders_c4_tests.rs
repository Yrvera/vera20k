//! Integration tests for the C4 plant lifecycle (walk-up, claim,
//! detonation, scatter). Covers the parity-critical behaviors from the
//! design doc: happy path, attacker death, Iron Curtain, two attackers,
//! target death, Stop cancellation, rejection of CanC4=no targets,
//! rejection of non-C4 attackers, plus a determinism regression test.

use super::*;
use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::{C4PlantState, Health, PendingC4Detonation};
use crate::sim::game_entity::GameEntity;
use std::collections::BTreeMap;

fn c4_test_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=GHOST\n1=TANY\n2=E1\n\n\
         [VehicleTypes]\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GAPILE\n1=CAMISC01\n\n\
         [GHOST]\nStrength=125\nArmor=flak\nSpeed=4\nC4=yes\nPrimary=Pistol\n\n\
         [TANY]\nStrength=125\nArmor=flak\nSpeed=4\nC4=yes\nPrimary=Pistol\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [GAPILE]\nStrength=600\nArmor=wood\nFoundation=2x2\n\n\
         [CAMISC01]\nStrength=600\nArmor=concrete\nFoundation=1x1\nCanC4=no\n\n\
         [Pistol]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [CombatDamage]\nC4Warhead=SA\n\n\
         [SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    RuleSet::from_ini(&ini).expect("c4 test rules should parse")
}

fn build_sim_with_c4_rules() -> (Simulation, RuleSet, BTreeMap<(u16, u16), u8>) {
    let mut sim = Simulation::new();
    let mut rules = c4_test_rules();
    // Required: tick_c4_plants calls rules.c4_warhead_id() which panics
    // unless this resolver has run.
    rules.resolve_bridge_warheads(&mut sim.interner);
    (sim, rules, BTreeMap::new())
}

fn spawn_infantry(sim: &mut Simulation, type_str: &str, owner: &str, rx: u16, ry: u16) -> u64 {
    let owner_id = sim.interner.intern(owner);
    let type_id = sim.interner.intern(type_str);
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner_id,
        Health {
            current: 125,
            max: 125,
        },
        type_id,
        EntityCategory::Infantry,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

fn spawn_building(sim: &mut Simulation, type_str: &str, owner: &str, rx: u16, ry: u16) -> u64 {
    let owner_id = sim.interner.intern(owner);
    let type_id = sim.interner.intern(type_str);
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner_id,
        Health {
            current: 600,
            max: 600,
        },
        type_id,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

/// Advance one tick, draining any pending commands first (mirrors the
/// production app_sim_tick loop).
fn step(sim: &mut Simulation, rules: &RuleSet, heights: &BTreeMap<(u16, u16), u8>) {
    let due = sim.take_due_commands();
    sim.advance_tick(&due, Some(rules), heights, None, None, 67);
}

// ---------- Test 1: happy path ----------

#[test]
fn c4_plant_happy_path_kills_building_and_seal_survives() {
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    // Spawn SEAL adjacent (Chebyshev-1) to the building so the plant claims
    // on the first tick — skips the pathfinding walk-up which is tested
    // elsewhere. tick_c4_plants Phase 1's adjacency check is what we're
    // verifying here.
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 11);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    let owner = sim.interner.intern("Americans");
    sim.queue_command(CommandEnvelope::new(
        owner,
        sim.tick + 1,
        Command::PlantC4 {
            attacker_id: seal,
            target_building_id: bld,
        },
    ));

    // First advance: command dispatch sets c4_plant; tick_c4_plants Phase 1
    // sees adjacency and claims.
    step(&mut sim, &rules, &heights);
    let pending = sim
        .entities
        .get(bld)
        .unwrap()
        .pending_c4_detonation
        .expect("plant must be claimed on adjacency");
    let plant_start = pending.plant_start_tick;

    // Advance until detonation tick fires. Phase 2 fires when
    // `sim.tick - plant_start >= delay`. The current sim.tick is already
    // past plant_start by 1 (advance_tick increments at the end), so
    // `delay + 1` more advances are enough.
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..(delay + 1) {
        step(&mut sim, &rules, &heights);
    }

    assert!(
        sim.entities
            .get(bld)
            .map_or(true, |b| b.dying || b.health.current == 0),
        "building must be destroyed at plant_start + c4_delay (plant_start={plant_start}, sim.tick={})",
        sim.tick
    );
    assert!(
        sim.entities.get(seal).is_some(),
        "SEAL must survive the plant"
    );
    assert!(
        !sim.entities.get(seal).unwrap().dying,
        "SEAL must not be dying"
    );
}

// ---------- Test 2: attacker death mid-plant ----------

#[test]
fn c4_attacker_death_does_not_abort_detonation() {
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 11);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    // Manually claim the plant (skip walk-up).
    sim.entities.get_mut(bld).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    // Mid-plant: kill the SEAL outright.
    sim.entities.get_mut(seal).unwrap().health.current = 0;
    sim.entities.get_mut(seal).unwrap().dying = true;
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities.get(seal).is_none() || sim.entities.get(seal).unwrap().dying,
        "SEAL must be despawned or dying after kill"
    );

    // Advance through C4Delay. Building MUST still die — the +0x6df marker
    // is never cleared in the C4 path, so detonation fires regardless.
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..(delay + 2) {
        step(&mut sim, &rules, &heights);
    }
    assert!(
        sim.entities
            .get(bld)
            .map_or(true, |b| b.dying || b.health.current == 0),
        "PARITY (OQ2): detonation must fire even after attacker death"
    );
}

// ---------- Test 3: Iron Curtain blocks then expires ----------

#[test]
fn c4_iron_curtain_blocks_until_expiry_then_kills() {
    use crate::sim::superweapon::invulnerability::{InvulnKind, InvulnerabilityState};
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 11);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    // Claim the plant, then IC the building. IC duration must outlast
    // C4Delay (27) so the first detonation attempt is nullified.
    sim.entities.get_mut(bld).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });
    sim.entities.get_mut(bld).unwrap().invulnerability = Some(InvulnerabilityState {
        start_frame: sim.tick as u32,
        duration_frames: 40,
        kind: InvulnKind::IronCurtain,
    });

    // Advance through C4Delay + 5. Building must STILL be alive (IC nullifies).
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..(delay + 5) {
        step(&mut sim, &rules, &heights);
    }
    assert!(
        sim.entities
            .get(bld)
            .is_some_and(|b| !b.dying && b.health.current > 0),
        "IC must block C4 damage while active"
    );

    // Advance past IC duration. The next damage tick kills the building.
    for _ in 0..40 {
        step(&mut sim, &rules, &heights);
    }
    assert!(
        sim.entities
            .get(bld)
            .map_or(true, |b| b.dying || b.health.current == 0),
        "PARITY: building must die after IC expires (damage retries every tick)"
    );
}

// ---------- Test 4: second SEAL on already-claimed target ----------

#[test]
fn second_c4_attacker_does_not_overwrite_plant() {
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    // Both adjacent to the building. seal_a gets the lower stable_id (sorted
    // iteration order in tick_c4_plants makes them deterministic).
    let seal_a = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 11);
    let seal_b = spawn_infantry(&mut sim, "TANY", "Americans", 11, 10);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    sim.entities.get_mut(seal_a).unwrap().c4_plant = Some(C4PlantState {
        target_building_id: bld,
    });
    sim.entities.get_mut(seal_b).unwrap().c4_plant = Some(C4PlantState {
        target_building_id: bld,
    });

    // First tick: A claims (lower stable_id, sorted order). B sees the claim and hovers.
    step(&mut sim, &rules, &heights);
    let pending = sim
        .entities
        .get(bld)
        .unwrap()
        .pending_c4_detonation
        .expect("first attacker claims");
    assert_eq!(
        pending.attacker_id, seal_a,
        "first attacker (lower stable_id) wins the claim — deterministic by sorted iteration"
    );

    // Another tick: pending must NOT have been overwritten by B.
    step(&mut sim, &rules, &heights);
    let pending_after = sim
        .entities
        .get(bld)
        .unwrap()
        .pending_c4_detonation
        .unwrap();
    assert_eq!(
        pending_after.plant_start_tick, pending.plant_start_tick,
        "pending plant_start_tick must not be overwritten by second attacker"
    );
    assert_eq!(
        pending_after.attacker_id, seal_a,
        "pending attacker must not be overwritten by second attacker"
    );
}

// ---------- Test 5: target death clears c4_plant ----------

#[test]
fn target_death_clears_c4_plant_on_attacker() {
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    sim.entities.get_mut(seal).unwrap().c4_plant = Some(C4PlantState {
        target_building_id: bld,
    });

    // Kill the building via direct mutation (simulate another weapon).
    sim.entities.get_mut(bld).unwrap().health.current = 0;
    sim.entities.get_mut(bld).unwrap().dying = true;

    step(&mut sim, &rules, &heights);

    assert!(
        sim.entities.get(seal).unwrap().c4_plant.is_none(),
        "c4_plant must clear when target dies"
    );
}

// ---------- Test 6: Stop cancels walk-up but not already-claimed plant ----------

#[test]
fn stop_cancels_walkup_but_not_already_claimed_plant() {
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    let owner = sim.interner.intern("Americans");

    // Case A: Stop during walk-up clears c4_plant.
    sim.entities.get_mut(seal).unwrap().c4_plant = Some(C4PlantState {
        target_building_id: bld,
    });
    sim.queue_command(CommandEnvelope::new(
        owner,
        sim.tick + 1,
        Command::Stop { entity_id: seal },
    ));
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities.get(seal).unwrap().c4_plant.is_none(),
        "Stop must clear c4_plant during walk-up"
    );
    assert!(
        sim.entities
            .get(bld)
            .unwrap()
            .pending_c4_detonation
            .is_none(),
        "no plant was claimed, building stays clean"
    );

    // Case B: Stop AFTER plant is claimed does NOT clear pending_c4_detonation.
    let plant_start = sim.tick;
    sim.entities.get_mut(bld).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: plant_start,
        attacker_id: seal,
    });
    sim.queue_command(CommandEnvelope::new(
        owner,
        sim.tick + 1,
        Command::Stop { entity_id: seal },
    ));
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities
            .get(bld)
            .unwrap()
            .pending_c4_detonation
            .is_some(),
        "PARITY: claimed plant survives Stop on attacker"
    );

    // And the building still detonates on schedule.
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..(delay + 2) {
        step(&mut sim, &rules, &heights);
    }
    assert!(
        sim.entities
            .get(bld)
            .map_or(true, |b| b.dying || b.health.current == 0),
        "claimed plant detonates on schedule even after Stop on attacker"
    );
}

// ---------- Test 7: CanC4=no building rejects PlantC4 ----------

#[test]
fn cannot_c4_building_rejects_plant_command() {
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
    let oil = spawn_building(&mut sim, "CAMISC01", "Soviets", 10, 10);

    let owner = sim.interner.intern("Americans");
    sim.queue_command(CommandEnvelope::new(
        owner,
        sim.tick + 1,
        Command::PlantC4 {
            attacker_id: seal,
            target_building_id: oil,
        },
    ));
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities.get(seal).unwrap().c4_plant.is_none(),
        "PlantC4 must be silently rejected for CanC4=no buildings"
    );
    assert!(
        sim.entities
            .get(oil)
            .unwrap()
            .pending_c4_detonation
            .is_none(),
        "rejected PlantC4 must not set pending_c4_detonation on the target"
    );
}

// ---------- Test 8: non-C4 attacker rejects PlantC4 ----------

#[test]
fn non_c4_unit_rejects_plant_command() {
    let (mut sim, rules, heights) = build_sim_with_c4_rules();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 5, 5);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    let owner = sim.interner.intern("Americans");
    sim.queue_command(CommandEnvelope::new(
        owner,
        sim.tick + 1,
        Command::PlantC4 {
            attacker_id: gi,
            target_building_id: bld,
        },
    ));
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities.get(gi).unwrap().c4_plant.is_none(),
        "PlantC4 must be silently rejected for non-C4 attackers"
    );
    assert!(
        sim.entities
            .get(bld)
            .unwrap()
            .pending_c4_detonation
            .is_none(),
        "rejected PlantC4 must not set pending_c4_detonation on the target"
    );
}

// ---------- Task 14: Determinism / replay regression ----------

#[test]
fn c4_lifecycle_is_deterministic() {
    fn run() -> Vec<u64> {
        let (mut sim, rules, heights) = build_sim_with_c4_rules();
        let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 11);
        let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);
        let owner = sim.interner.intern("Americans");
        sim.queue_command(CommandEnvelope::new(
            owner,
            sim.tick + 1,
            Command::PlantC4 {
                attacker_id: seal,
                target_building_id: bld,
            },
        ));
        let mut hashes = Vec::new();
        for _ in 0..100 {
            step(&mut sim, &rules, &heights);
            hashes.push(sim.state_hash());
        }
        hashes
    }

    let h1 = run();
    let h2 = run();
    assert_eq!(h1, h2, "C4 lifecycle must be deterministic across runs");
}
