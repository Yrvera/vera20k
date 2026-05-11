//! Acceptance tests for the miner (harvester) state machine system.
//!
//! Tests exercise the miner_system::tick_miners() pipeline with a minimal
//! EntityStore: miner entity + refinery structure + resource nodes. Verifies
//! payout math, dock queuing, Chrono teleport rules, incremental unloading,
//! local continuation, pip display, and refinery rebinding.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;
use crate::sim::miner::{
    CargoBale, Miner, MinerConfig, MinerKind, MinerState, RefineryDockPhase, ResourceNode,
    ResourceType,
};
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::production::credits_for_owner;
use crate::sim::world::Simulation;

/// Minimal rules that know about HARV, CMIN, and GAREFN.
fn miner_rules() -> RuleSet {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=HARV\n\
         1=CMIN\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GAREFN\n\
         [HARV]\n\
         Name=War Miner\n\
         Cost=1400\n\
         Strength=600\n\
         Armor=heavy\n\
         Speed=4\n\
         ROT=5\n\
         Sight=5\n\
         TechLevel=1\n\
         Owner=Americans\n\
         Harvester=yes\n\
         Dock=GAREFN\n\
         [CMIN]\n\
         Name=Chrono Miner\n\
         Cost=1400\n\
         Strength=400\n\
         Armor=light\n\
         Speed=4\n\
         Sight=5\n\
         TechLevel=1\n\
         Owner=Americans\n\
         Harvester=yes\n\
         Teleporter=yes\n\
         ChronoInSound=ChronoMinerTeleport\n\
         ChronoOutSound=ChronoMinerTeleport\n\
         Dock=GAREFN\n\
         [GAREFN]\n\
         Name=Ore Refinery\n\
         Cost=2000\n\
         Strength=900\n\
         Armor=wood\n\
         TechLevel=1\n\
         Owner=Americans\n\
         Foundation=4x3\n\
         Refinery=yes\n\
         FreeUnit=CMIN\n",
    );
    RuleSet::from_ini(&ini).expect("miner rules")
}

fn dock_rules() -> RuleSet {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=MODHARV\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=MODPROC\n\
         1=OTHERPROC\n\
         [MODHARV]\n\
         Name=Mod Harvester\n\
         Harvester=yes\n\
         Dock=MODPROC\n\
         Speed=4\n\
         [MODPROC]\n\
         Name=Mod Refinery\n\
         Foundation=4x3\n\
         Refinery=yes\n\
         [OTHERPROC]\n\
         Name=Other Refinery\n\
         Foundation=4x3\n\
         Refinery=yes\n",
    );
    RuleSet::from_ini(&ini).expect("dock rules")
}

/// Spawn a miner entity at (rx, ry), returning its stable_id.
fn spawn_miner(sim: &mut Simulation, sid: u64, kind: MinerKind, rx: u16, ry: u16) -> u64 {
    let type_id = match kind {
        MinerKind::War => "HARV",
        MinerKind::Chrono => "CMIN",
        MinerKind::Slave => "SMIN",
    };
    let health_val: u16 = match kind {
        MinerKind::War => 600,
        MinerKind::Chrono => 400,
        MinerKind::Slave => 2000,
    };
    let owner_id = sim.interner.intern("Americans");
    let type_id_interned = sim.interner.intern(type_id);
    let mut ge = GameEntity::new(
        sid,
        rx,
        ry,
        0,
        0,
        owner_id,
        Health {
            current: health_val,
            max: health_val,
        },
        type_id_interned,
        EntityCategory::Unit,
        0,
        5,
        true,
    );
    ge.miner = Some(Miner::new(kind, &MinerConfig::default(), 0));
    sim.entities.insert(ge);
    // Update next_stable_entity_id if needed so allocate_stable_entity_id doesn't collide.
    if sim.next_stable_entity_id <= sid {
        sim.next_stable_entity_id = sid + 1;
    }
    sid
}

/// Spawn a refinery structure at (rx, ry) with a given stable_id.
fn spawn_refinery(sim: &mut Simulation, sid: u64, rx: u16, ry: u16) {
    let owner_id = sim.interner.intern("Americans");
    let type_id = sim.interner.intern("GAREFN");
    let ge = GameEntity::new(
        sid,
        rx,
        ry,
        0,
        0,
        owner_id,
        Health {
            current: 900,
            max: 900,
        },
        type_id,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    sim.entities.insert(ge);
    if sim.next_stable_entity_id <= sid {
        sim.next_stable_entity_id = sid + 1;
    }
}

fn spawn_structure(sim: &mut Simulation, sid: u64, type_id: &str, rx: u16, ry: u16) {
    let owner_id = sim.interner.intern("Americans");
    let type_id_interned = sim.interner.intern(type_id);
    let ge = GameEntity::new(
        sid,
        rx,
        ry,
        0,
        0,
        owner_id,
        Health {
            current: 900,
            max: 900,
        },
        type_id_interned,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    sim.entities.insert(ge);
    if sim.next_stable_entity_id <= sid {
        sim.next_stable_entity_id = sid + 1;
    }
}

/// Place ore resource nodes at a cell with a given amount.
fn place_ore(sim: &mut Simulation, rx: u16, ry: u16, amount: u16) {
    sim.production.resource_nodes.insert(
        (rx, ry),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: amount,
        },
    );
}

/// Place gem resource nodes at a cell with a given amount.
#[allow(dead_code)]
fn place_gems(sim: &mut Simulation, rx: u16, ry: u16, amount: u16) {
    sim.production.resource_nodes.insert(
        (rx, ry),
        ResourceNode {
            resource_type: ResourceType::Gem,
            remaining: amount,
        },
    );
}

/// Tick the miner system `n` times.
///
/// Matches advance_tick ordering: teleport (Phase 2) → miners (Phase 7) →
/// ground movement. Teleport must run before miners so that Relocate/ChronoDelay
/// updates are visible to the miner snapshot.
fn tick_miners_n(sim: &mut Simulation, rules: &RuleSet, n: usize) {
    let config = MinerConfig::default();
    let grid = PathGrid::new(64, 64);
    for _ in 0..n {
        crate::sim::movement::teleport_movement::tick_teleport_movement(
            &mut sim.entities,
            &mut OccupancyGrid::new(),
            67,
            sim.tick,
        );
        super::miner_system::tick_miners(sim, rules, &config, Some(&grid));
        // Also tick movement so issue_direct_move targets are consumed
        // (Linked/Departing wait for movement_target to be None).
        crate::sim::movement::tick_movement(&mut sim.entities, 67, &mut sim.interner);
        sim.tick += 1;
    }
}

/// Read the Miner component from an entity by stable_id.
fn get_miner(sim: &Simulation, entity_id: u64) -> Miner {
    sim.entities
        .get(entity_id)
        .and_then(|e| e.miner.as_ref())
        .cloned()
        .expect("miner component should exist")
}

// ==========================================================================
// Test 1: War Miner full ore load = 1000 credits
// ==========================================================================
#[test]
fn war_miner_full_ore_payout_is_1000() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Miner at dock cell, refinery at (10, 10) with 4x3 foundation.
    // Dock cell = (rx + width, ry + height/2) = (10 + 4, 10 + 1) = (14, 11) — east platform.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    // Pre-load cargo: 40 ore bales.
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..40 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        // Put miner in Dock state so it proceeds to Unload.
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
    }

    let before = credits_for_owner(&sim, "Americans");
    // Tick enough times to fully unload: 40 bales * unload_interval=57 = 2280 ticks.
    tick_miners_n(&mut sim, &rules, 2400);

    let after = credits_for_owner(&sim, "Americans");
    assert_eq!(after - before, 1000, "War Miner full ore = 1000 credits");
}

// ==========================================================================
// Test 2: War Miner full gem load = 2000 credits
// ==========================================================================
#[test]
fn war_miner_full_gem_payout_is_2000() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..40 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Gem,
                value: 50,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
    }

    let before = credits_for_owner(&sim, "Americans");
    tick_miners_n(&mut sim, &rules, 2400);
    let after = credits_for_owner(&sim, "Americans");
    assert_eq!(after - before, 2000, "War Miner full gems = 2000 credits");
}

// ==========================================================================
// Test 3: Chrono Miner full ore load = 500 credits
// ==========================================================================
#[test]
fn chrono_miner_full_ore_payout_is_500() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..20 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
    }

    let before = credits_for_owner(&sim, "Americans");
    // 20 bales * unload_interval=57 = 1140 ticks.
    tick_miners_n(&mut sim, &rules, 1200);
    let after = credits_for_owner(&sim, "Americans");
    assert_eq!(after - before, 500, "Chrono Miner full ore = 500 credits");
}

// ==========================================================================
// Test 4: Chrono Miner full gem load = 1000 credits
// ==========================================================================
#[test]
fn chrono_miner_full_gem_payout_is_1000() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..20 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Gem,
                value: 50,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
    }

    let before = credits_for_owner(&sim, "Americans");
    tick_miners_n(&mut sim, &rules, 1200);
    let after = credits_for_owner(&sim, "Americans");
    assert_eq!(
        after - before,
        1000,
        "Chrono Miner full gems = 1000 credits"
    );
}

// ==========================================================================
// Test 5: Chrono Miner teleports on return (position snaps to dock)
// ==========================================================================
#[test]
fn chrono_miner_teleports_to_refinery_on_return() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Miner at ore far from refinery. Must be > ChronoHarvTooFarDistance (50 cells)
    // from dock cell (14, 11) so the chrono teleport triggers.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 80, 80);
    spawn_refinery(&mut sim, 2, 10, 10);
    // Dock cell for 4x3 at (10,10) = (14, 11) — east platform.

    // Give it some cargo so it wants to return.
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
        // No reserved_refinery yet — the system should find one and teleport.
    }

    // Tick 1: miner finds refinery and issues teleport command.
    // tick_teleport_movement already ran this iteration (no-op), so Relocate
    // hasn't executed yet — teleport_state is set but position is unchanged.
    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_some(),
        "Chrono Miner should have an active teleport after first tick"
    );

    // Tick 2: tick_teleport_movement runs Relocate → position snaps to queue cell.
    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (14, 11),
        "Position should be at queue cell after Relocate"
    );

    // Run enough ticks for the chrono delay to expire and dock sequence to complete.
    // Distance ~95 cells → delay ≈ 95*256/48 ≈ 509 ticks. After the delay the
    // miner enters the 4-state dock FSM (Approach → Linked → Unloading →
    // Departing) and ends up at the exit cell. For the 4×3 refinery at (10, 10)
    // the gamemd-formula exit cell is (11, 12).
    tick_miners_n(&mut sim, &rules, 600);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_none(),
        "Teleport should be complete"
    );
    // After teleport + dock sequence, miner exits at the refinery exit cell.
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (11, 12),
        "Chrono Miner should be at exit cell after completing dock sequence"
    );
}

// ==========================================================================
// Test 6: War Miner does NOT teleport (stays where it is on first return tick)
// ==========================================================================
#[test]
fn war_miner_does_not_teleport() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 30, 30);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let pos = &sim.entities.get(miner_id).expect("entity").position;
    // War miner should NOT have teleported — still at (30, 30).
    assert_eq!((pos.rx, pos.ry), (30, 30), "War Miner should not teleport");
}

// ==========================================================================
// Test 7: Dock queuing — only one miner at a refinery at a time
// ==========================================================================
#[test]
fn dock_queuing_one_at_a_time() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Two miners at the dock cell, both ready to unload.
    let m1 = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    let m2 = spawn_miner(&mut sim, 3, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    // Pre-load both with cargo, put in Dock Approach state (poll-and-link).
    for entity_id in [m1, m2] {
        let entity = sim.entities.get_mut(entity_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.reserved_refinery = Some(2);
    }

    // First tick: one should get the dock, other should wait.
    tick_miners_n(&mut sim, &rules, 1);

    let m1_miner = get_miner(&sim, m1);
    let m2_miner = get_miner(&sim, m2);

    // Miner with lower stable_id (1) processes first, wins the reservation,
    // and transitions to Linked. m2 fails the reservation poll and stays
    // in Approach.
    assert_eq!(
        m1_miner.state,
        MinerState::Dock,
        "First miner should still be docking"
    );
    assert_eq!(
        m1_miner.dock_phase,
        RefineryDockPhase::Linked,
        "First miner should advance to Linked once reservation is granted"
    );
    assert_eq!(
        m2_miner.state,
        MinerState::Dock,
        "Second miner should still be docking"
    );
    assert_eq!(
        m2_miner.dock_phase,
        RefineryDockPhase::Approach,
        "Second miner should still be in Approach polling for the dock"
    );
}

// ==========================================================================
// Test 8: Credits arrive incrementally during unload (not instant)
// ==========================================================================
#[test]
fn credits_arrive_incrementally_during_unload() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    // Load 10 bales (250 credits total).
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..10 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
    }

    let before = credits_for_owner(&sim, "Americans");

    // After 1 tick: dock grants, transition to Unload, first bale popped immediately.
    tick_miners_n(&mut sim, &rules, 1);
    let after_1 = credits_for_owner(&sim, "Americans");
    // First unload_timer is 0, so first bale pops on first unload tick.
    assert!(
        after_1 - before <= 25,
        "Should have at most 1 bale worth after first tick"
    );

    // After a few more ticks, should have more but NOT all.
    // unload_tick_interval=57, so 10 bails need ~570 ticks total.
    tick_miners_n(&mut sim, &rules, 50);
    let after_51 = credits_for_owner(&sim, "Americans");
    assert!(
        after_51 - before < 250,
        "Credits should not be fully delivered after only 51 ticks (need ~570 ticks for 10 bails)"
    );

    // After enough ticks, all 250 delivered.
    tick_miners_n(&mut sim, &rules, 600);
    let after_all = credits_for_owner(&sim, "Americans");
    assert_eq!(
        after_all - before,
        250,
        "All 10 bales = 250 credits should be delivered"
    );
}

// ==========================================================================
// Test 9: After ore cell empties, miner searches for more (local continuation)
// ==========================================================================
#[test]
fn local_continuation_after_cell_depletes() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Miner at (20, 20). Two ore cells: one small (will deplete), one nearby.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    place_ore(&mut sim, 20, 20, 2); // Only 2 bales worth
    place_ore(&mut sim, 22, 20, 100); // Nearby ore within local radius (6 cells)

    // Put miner in Harvest state at its position.
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    // Tick enough to deplete the small cell and search for the next.
    // harvest_tick_interval=8, so 2 bales takes ~17 ticks, then search triggers.
    tick_miners_n(&mut sim, &rules, 30);

    let miner = get_miner(&sim, miner_id);
    // After (20,20) depletes, the short-scan continuation must pick (22,20)
    // and the miner transitions to MoveToOre / Harvest (gamemd State 1
    // depletion path: stay harvesting, move to new cell within
    // TiberiumShortScan radius).
    assert_eq!(
        miner.target_ore_cell,
        Some((22, 20)),
        "Short-scan continuation should pick the nearby ore at (22, 20)"
    );
    assert!(
        matches!(miner.state, MinerState::MoveToOre | MinerState::Harvest),
        "Miner should be moving to / harvesting the new cell; state was {:?}",
        miner.state,
    );
}

// ==========================================================================
// Test 9a: Cell depletes with PARTIAL cargo → miner continues to nearby ore
//          (the short-scan-before-return behavior, gamemd State 1)
// ==========================================================================
#[test]
fn harvest_continues_to_nearby_ore_when_cell_depletes_partial_cargo() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // Cell at miner's position: depletes after 2 bales.
    place_ore(&mut sim, 20, 20, 2);
    // Nearby ore well within TiberiumShortScan (radius 6 cells).
    place_ore(&mut sim, 23, 20, 100);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    // Tick enough to deplete (20,20) and trigger the continuation scan.
    tick_miners_n(&mut sim, &rules, 30);

    let miner = get_miner(&sim, miner_id);
    assert!(
        !miner.cargo.is_empty(),
        "Miner should have extracted bales before cell depleted"
    );
    assert_eq!(
        miner.target_ore_cell,
        Some((23, 20)),
        "After cell depleted, miner should pick the nearby ore via short scan"
    );
    assert!(
        matches!(miner.state, MinerState::MoveToOre | MinerState::Harvest),
        "Miner should move to / be harvesting the new ore cell, not return-to-refinery; \
         state was {:?}",
        miner.state,
    );
    assert!(
        !matches!(miner.state, MinerState::ReturnToRefinery | MinerState::Dock),
        "Miner with ore nearby must NOT head to refinery on partial cargo"
    );
}

// ==========================================================================
// Test 9b: Cell depletes with PARTIAL cargo + no ore nearby → miner returns
// ==========================================================================
#[test]
fn harvest_returns_when_no_ore_within_short_scan() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // Only the miner's cell has ore. Nothing within the short-scan radius
    // (default 6 cells). The further ore patch is well outside.
    place_ore(&mut sim, 20, 20, 2);
    place_ore(&mut sim, 50, 50, 100); // far outside local_continuation_radius

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    tick_miners_n(&mut sim, &rules, 30);

    let miner = get_miner(&sim, miner_id);
    assert!(
        !miner.cargo.is_empty(),
        "Miner should have extracted bales before depletion"
    );
    assert!(
        matches!(miner.state, MinerState::ReturnToRefinery | MinerState::Dock),
        "With cargo but no nearby ore, miner must head to refinery; state was {:?}",
        miner.state,
    );
}

// ==========================================================================
// Test 9c: EMPTY-cargo cell depletion falls back to SearchOre 4-stage cascade
//          (regression guard: ensures the empty-cargo path keeps working)
// ==========================================================================
#[test]
fn empty_cargo_cell_depletion_falls_back_to_full_search() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // No ore on the miner's cell when Harvest state runs.
    // Nothing within short-scan radius (6 cells).
    // Ore exists within long-scan radius (default 48).
    place_ore(&mut sim, 40, 20, 100); // ~20 cells away, within long scan

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
        // Cargo intentionally empty — extract_bale will fail on first tick.
        assert!(miner.cargo.is_empty());
    }

    tick_miners_n(&mut sim, &rules, 5);

    let miner = get_miner(&sim, miner_id);
    assert!(
        miner.cargo.is_empty(),
        "No ore was on the cell, so no bales should have been extracted"
    );
    assert_eq!(
        miner.target_ore_cell,
        Some((40, 20)),
        "Empty-cargo cell depletion should fall through SearchOre and find the \
         long-scan ore at (40, 20)"
    );
    assert!(
        matches!(miner.state, MinerState::MoveToOre),
        "Miner should be heading to the new ore cell; state was {:?}",
        miner.state,
    );
}

// ==========================================================================
// Test 10: Cargo pips always show 5 steps of 20%
// ==========================================================================
#[test]
fn cargo_pips_five_steps() {
    let config = MinerConfig::default();
    let mut miner = Miner::new(MinerKind::War, &config, 0);
    // War Miner capacity = 40 bales
    assert_eq!(miner.cargo_pips(), 0);

    // 20% = 8 bales → 1 pip
    for _ in 0..8 {
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
    }
    assert_eq!(miner.cargo_pips(), 1);

    // 40% = 16 bales → 2 pips
    for _ in 0..8 {
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
    }
    assert_eq!(miner.cargo_pips(), 2);

    // 60% = 24 bales → 3 pips
    for _ in 0..8 {
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
    }
    assert_eq!(miner.cargo_pips(), 3);

    // 80% = 32 bales → 4 pips
    for _ in 0..8 {
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
    }
    assert_eq!(miner.cargo_pips(), 4);

    // 100% = 40 bales → 5 pips
    for _ in 0..8 {
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
    }
    assert_eq!(miner.cargo_pips(), 5);
}

// ==========================================================================
// Test 11: After unload, home_refinery rebinds to the refinery used
// ==========================================================================
#[test]
fn home_refinery_rebinds_after_unload() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.home_refinery = None; // Start without a home
    }

    // Tick until unload completes: 1 bale × unload_interval=57 ticks.
    tick_miners_n(&mut sim, &rules, 70);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(
        miner.home_refinery,
        Some(2),
        "Home refinery should rebind to the refinery used for unloading"
    );
}

// ==========================================================================
// Test 12: Forced return (MinerReturn command) triggers Chrono teleport
// ==========================================================================
#[test]
fn forced_return_chrono_teleports() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Must be > ChronoHarvTooFarDistance (50 cells) from dock cell (14, 11)
    // so the chrono teleport triggers instead of driving.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 80, 80);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::ForcedReturn;
        miner.forced_return = true;
    }

    // Tick 1: finds refinery, issues teleport command. Relocate not yet run.
    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_some(),
        "Forced return should have issued a teleport"
    );

    // Tick 2: Relocate snaps position to queue cell.
    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (14, 11),
        "Position should be at queue cell after Relocate"
    );

    // Run enough ticks for the chrono delay to expire and dock sequence to complete.
    // Diagonal exit drive (pad → exit via Chebyshev unit-step path) is ~22 ticks.
    tick_miners_n(&mut sim, &rules, 600);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_none(),
        "Teleport should be complete"
    );
    // After teleport + dock sequence, miner exits at the refinery exit cell.
    // For the 4×3 refinery at (10, 10) the gamemd-formula exit cell is (11, 12).
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (11, 12),
        "Forced return should have teleported and docked — now at exit cell"
    );
}

// ==========================================================================
// Test: Chrono teleport emits ChronoInSound + ChronoOutSound at correct cells
// ==========================================================================
/// On a chrono miner return-warp, the sim must emit two `ChronoTeleport` sound
/// events:
///   - one at the source cell with the unit's `ChronoOutSound=`
///   - one at the destination cell with the unit's `ChronoInSound=`
#[test]
fn chrono_teleport_emits_in_and_out_sounds_at_correct_cells() {
    use crate::sim::world::SimSoundEvent;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Far miner so the warp branch fires (>ChronoHarvTooFarDistance from dock).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 80, 80);
    spawn_refinery(&mut sim, 2, 10, 10);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    // Drain pre-existing sound events so we only see this tick's emissions.
    sim.sound_events.clear();

    // Tick once — miner finds refinery, far_enough=true → spawn_warp_effects fires.
    tick_miners_n(&mut sim, &rules, 1);

    // Collect ChronoTeleport events; each carries a resolved InternedId sound name.
    let chrono_events: Vec<_> = sim
        .sound_events
        .iter()
        .filter_map(|e| match e {
            SimSoundEvent::ChronoTeleport { sound_id, rx, ry } => {
                Some((sim.interner.resolve(*sound_id).to_string(), *rx, *ry))
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        chrono_events.len(),
        2,
        "self-teleport must emit exactly two chrono sound events (out at source, in at dest)"
    );
    // Source cell = miner's start position (80, 80); dest cell = refinery dock (14, 11).
    assert!(
        chrono_events
            .iter()
            .any(|(s, rx, ry)| s == "ChronoMinerTeleport" && *rx == 80 && *ry == 80),
        "ChronoOutSound must fire at the source cell. got: {:?}",
        chrono_events
    );
    assert!(
        chrono_events
            .iter()
            .any(|(s, rx, ry)| s == "ChronoMinerTeleport" && *rx == 14 && *ry == 11),
        "ChronoInSound must fire at the dest cell. got: {:?}",
        chrono_events
    );
}

/// Variant of `miner_rules()` where CMIN omits the per-unit `ChronoInSound`
/// and `ChronoOutSound` keys, and `[General]` sets distinctive fallback
/// values. Used by the fallback-path test to prove the resolver reads from
/// Rules when the per-unit field is absent.
fn miner_rules_fallback_only() -> RuleSet {
    let ini = IniFile::from_str(
        "[General]\n\
         ChronoInSound=FALLBACKIN\n\
         ChronoOutSound=FALLBACKOUT\n\
         [InfantryTypes]\n\
         [VehicleTypes]\n\
         0=HARV\n\
         1=CMIN\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GAREFN\n\
         [HARV]\n\
         Name=War Miner\n\
         Cost=1400\n\
         Strength=600\n\
         Armor=heavy\n\
         Speed=4\n\
         ROT=5\n\
         Sight=5\n\
         TechLevel=1\n\
         Owner=Americans\n\
         Harvester=yes\n\
         Dock=GAREFN\n\
         [CMIN]\n\
         Name=Chrono Miner\n\
         Cost=1400\n\
         Strength=400\n\
         Armor=light\n\
         Speed=4\n\
         Sight=5\n\
         TechLevel=1\n\
         Owner=Americans\n\
         Harvester=yes\n\
         Teleporter=yes\n\
         Dock=GAREFN\n\
         [GAREFN]\n\
         Name=Ore Refinery\n\
         Cost=2000\n\
         Strength=900\n\
         Armor=wood\n\
         TechLevel=1\n\
         Owner=Americans\n\
         Foundation=4x3\n\
         Refinery=yes\n\
         FreeUnit=CMIN\n",
    );
    RuleSet::from_ini(&ini).expect("miner fallback rules")
}

/// When the per-unit `ChronoInSound=` / `ChronoOutSound=` are absent, the
/// resolver must fall back to the `[General]` values from Rules. Confirms
/// the two-level lookup matches the original engine's behavior.
#[test]
fn chrono_teleport_sound_falls_back_to_rules_general() {
    use crate::sim::world::SimSoundEvent;

    let mut sim = Simulation::new();
    let rules = miner_rules_fallback_only();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 80, 80);
    spawn_refinery(&mut sim, 2, 10, 10);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    sim.sound_events.clear();
    tick_miners_n(&mut sim, &rules, 1);

    let chrono_events: Vec<_> = sim
        .sound_events
        .iter()
        .filter_map(|e| match e {
            SimSoundEvent::ChronoTeleport { sound_id, rx, ry } => {
                Some((sim.interner.resolve(*sound_id).to_string(), *rx, *ry))
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        chrono_events.len(),
        2,
        "fallback path must still emit exactly two sound events"
    );
    assert!(
        chrono_events
            .iter()
            .any(|(s, rx, ry)| s == "FALLBACKOUT" && *rx == 80 && *ry == 80),
        "Rules [General] ChronoOutSound must fire at source. got: {:?}",
        chrono_events
    );
    assert!(
        chrono_events
            .iter()
            .any(|(s, rx, ry)| s == "FALLBACKIN" && *rx == 14 && *ry == 11),
        "Rules [General] ChronoInSound must fire at dest. got: {:?}",
        chrono_events
    );
}

// ==========================================================================
// Test: Chrono Miner drives to ore (does NOT warp — only warps on return)
// ==========================================================================
#[test]
fn chrono_miner_drives_to_ore() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 10, 10);
    place_ore(&mut sim, 12, 10, 1200);

    // Set up: miner knows about ore, state = MoveToOre.
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.target_ore_cell = Some((12, 10));
        miner.state = MinerState::MoveToOre;
    }

    // After one tick, chrono miner should NOT have a teleport — it drives.
    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_none(),
        "Chrono Miner should drive to ore, not warp"
    );
}

// ==========================================================================
// Test 13: SearchOre transitions to WaitNoOre when map has no resources
// ==========================================================================
#[test]
fn search_ore_becomes_wait_when_empty() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // No ore placed!

    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(
        miner.state,
        MinerState::WaitNoOre,
        "Miner should enter WaitNoOre when no resources exist"
    );
}

// ==========================================================================
// Test 14: WaitNoOre rescans after cooldown
// ==========================================================================
#[test]
fn wait_no_ore_rescans_after_cooldown() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::WaitNoOre;
        miner.rescan_cooldown = config.rescan_cooldown_ticks;
    }

    // rescan_cooldown_ticks = 105 (0x69 frames from original engine).
    // After half the cooldown, should still be waiting.
    let half_cooldown = (config.rescan_cooldown_ticks / 2) as usize;
    tick_miners_n(&mut sim, &rules, half_cooldown);
    assert_eq!(
        get_miner(&sim, miner_id).state,
        MinerState::WaitNoOre,
        "Should still be waiting mid-cooldown"
    );

    // Place ore so that when rescan fires it finds something.
    place_ore(&mut sim, 20, 20, 100);

    // Tick the remaining cooldown + 5 extra (transition tick + SearchOre tick).
    let remaining = (config.rescan_cooldown_ticks as usize) - half_cooldown + 5;
    tick_miners_n(&mut sim, &rules, remaining);
    let state = get_miner(&sim, miner_id).state;
    assert!(
        state != MinerState::WaitNoOre,
        "Should have rescanned and found ore, got {:?}",
        state,
    );
}

#[test]
fn harvester_uses_dock_list_for_refinery_selection() {
    let mut sim = Simulation::new();
    let rules = dock_rules();
    let miner_id = sim
        .spawn_object("MODHARV", "Americans", 30, 30, 64, &rules, &BTreeMap::new())
        .expect("spawn harvester");
    spawn_structure(&mut sim, 2, "OTHERPROC", 28, 28);
    spawn_structure(&mut sim, 3, "MODPROC", 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.reserved_refinery, Some(3));
    assert_eq!(miner.state, MinerState::ReturnToRefinery);
}

#[test]
fn harvester_waits_when_no_dock_compatible_refinery_exists() {
    let mut sim = Simulation::new();
    let rules = dock_rules();
    let miner_id = sim
        .spawn_object("MODHARV", "Americans", 30, 30, 64, &rules, &BTreeMap::new())
        .expect("spawn harvester");
    spawn_structure(&mut sim, 2, "OTHERPROC", 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.reserved_refinery, None);
    assert_eq!(miner.state, MinerState::WaitNoOre);
}

// ==========================================================================
// Test 15: Dock cell calculation for 3x3 foundation
// ==========================================================================
#[test]
fn dock_cell_for_4x3_refinery() {
    // refinery_dock_cell(rx, ry, width, height)
    // Dock is just outside the east edge, vertically centered: (rx + width, ry + height/2).
    // For 4x3 at (10, 10): (10 + 4, 10 + 1) = (14, 11).
    // None = no art.ini QueueingCell override, falls back to geometric computation.
    let dock = super::miner_system::refinery_dock_cell(10, 10, 4, 3, None);
    assert_eq!(dock, (14, 11));
}

// ==========================================================================
// Test 16: pick_best_resource_node prefers gems over ore
// ==========================================================================
#[test]
fn pick_best_resource_node_prefers_gems_over_ore() {
    use crate::sim::production::pick_best_resource_node;
    use std::collections::BTreeMap;

    let mut nodes: BTreeMap<(u16, u16), ResourceNode> = BTreeMap::new();
    // Ore node equidistant from miner (at 5,5).
    nodes.insert(
        (5, 3),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 500,
        },
    );
    // Gem node at same distance.
    nodes.insert(
        (5, 7),
        ResourceNode {
            resource_type: ResourceType::Gem,
            remaining: 500,
        },
    );

    let chosen = pick_best_resource_node(&nodes, (5, 5), None);
    assert_eq!(
        chosen,
        Some((5, 7)),
        "Miner should prefer gems over equidistant ore"
    );
}

// ==========================================================================
// Test 17: pick_best_resource_node prefers denser ore when same type
// ==========================================================================
#[test]
fn pick_best_resource_node_prefers_higher_density() {
    use crate::sim::production::pick_best_resource_node;
    use std::collections::BTreeMap;

    let mut nodes: BTreeMap<(u16, u16), ResourceNode> = BTreeMap::new();
    // Sparse ore node equidistant from miner (at 5,5).
    nodes.insert(
        (5, 3),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 100,
        },
    );
    // Dense ore node at same distance.
    nodes.insert(
        (5, 7),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 900,
        },
    );

    let chosen = pick_best_resource_node(&nodes, (5, 5), None);
    assert_eq!(
        chosen,
        Some((5, 7)),
        "Miner should prefer the denser (remaining=900) ore node"
    );
}

// ==========================================================================
// Dock sequence tests
// ==========================================================================

/// Verify the dock sequence progresses through all phases when given enough ticks.
#[test]
fn dock_sequence_progresses_through_phases() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Miner at queue cell (14, 11), refinery at (10, 10) with 4x3 foundation.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.reserved_refinery = Some(2);
    }

    // Tick 1: Approach → Linked (dock is free, reservation granted; miner
    // re-targets the pad cell, which is walkable courtesy of RemoveOccupy).
    tick_miners_n(&mut sim, &rules, 1);
    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::Linked);

    // Tick enough for movement onto pad + per-bale unload + exit drive.
    // 1 bale * 14.4 ticks/bale ≈ 15 ticks unload, plus pad enter/exit.
    tick_miners_n(&mut sim, &rules, 200);
    let m = get_miner(&sim, miner_id);
    // With only 1 bale (unload_tick_interval=14), unloading takes ~15 ticks.
    // After that, Departing → SearchOre.
    // After docking, miner transitions to SearchOre. Since there's no ore
    // on the map, it immediately goes to WaitNoOre. Both are valid endpoints.
    assert!(
        m.state == MinerState::SearchOre || m.state == MinerState::WaitNoOre,
        "Miner should complete dock sequence, got state={:?} phase={:?}",
        m.state,
        m.dock_phase,
    );
}

/// Verify the Approach phase grants the dock reservation when free and
/// transitions to Linked immediately.
#[test]
fn dock_wait_grants_reservation_when_free() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.reserved_refinery = Some(2);
    }

    tick_miners_n(&mut sim, &rules, 1);

    // Dock should be occupied by this miner; phase advances to Linked.
    assert!(sim.production.dock_reservations.is_occupied(2));
    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::Linked);
    assert!(!m.dock_queued);
}

/// Verify pad cell and exit cell computation for a 4x3 refinery.
#[test]
fn refinery_pad_and_exit_cells() {
    use super::miner_dock_sequence::{refinery_exit_cell, refinery_pad_cell, refinery_queue_cell};

    // 4x3 foundation at (10, 10), no art.ini overrides:
    // queue = (14, 11), pad = (13, 11)
    // exit = foundation_centroid_lepton + (-0x80, +0x80):
    //   x = (10*256 + 4*128 - 128) / 256 = 2944 / 256 = 11
    //   y = (10*256 + 3*128 + 128) / 256 = 3072 / 256 = 12
    assert_eq!(refinery_queue_cell(10, 10, 4, 3, None), (14, 11));
    assert_eq!(refinery_pad_cell(10, 10, 4, 3, None), (13, 11));
    assert_eq!(refinery_exit_cell(10, 10, 4, 3), (11, 12));

    // 3x3 foundation at (5, 5), no art.ini overrides:
    // queue = (8, 6), pad = (7, 6)
    // exit = (5*256 + 3*128 - 128)/256 = 1536/256 = 6,
    //        (5*256 + 3*128 + 128)/256 = 1792/256 = 7
    assert_eq!(refinery_queue_cell(5, 5, 3, 3, None), (8, 6));
    assert_eq!(refinery_pad_cell(5, 5, 3, 3, None), (7, 6));
    assert_eq!(refinery_exit_cell(5, 5, 3, 3), (6, 7));

    // With QueueingCell override from art.ini:
    assert_eq!(refinery_queue_cell(10, 10, 4, 3, Some((4, 1))), (14, 11)); // same result for standard
    assert_eq!(refinery_queue_cell(10, 10, 4, 3, Some((3, 2))), (13, 12)); // custom position
}

/// Verify the Unloading phase awards credits like the old handle_unload.
#[test]
fn dock_unloading_phase_awards_credits() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Place miner directly in Unloading phase at pad cell (13, 11).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..5 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
    }

    // Pre-reserve the dock so release works correctly.
    sim.production.dock_reservations.try_reserve(2, miner_id);

    let before = credits_for_owner(&sim, "Americans");
    // 5 bales × unload_interval=14 = ~70 ticks + margin.
    tick_miners_n(&mut sim, &rules, 100);
    let after = credits_for_owner(&sim, "Americans");

    assert_eq!(after - before, 125, "5 ore bales × 25 = 125 credits");
}

/// Verify that after unloading finishes, the miner exits and returns to SearchOre.
#[test]
fn dock_exit_returns_to_search_ore() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
    }

    sim.production.dock_reservations.try_reserve(2, miner_id);

    // Tick enough for unload (1 bale at 14 ticks) + exit movement + margin.
    tick_miners_n(&mut sim, &rules, 50);

    let m = get_miner(&sim, miner_id);
    // After unloading, miner goes to SearchOre → WaitNoOre (no ore on map).
    assert!(
        m.state == MinerState::SearchOre || m.state == MinerState::WaitNoOre,
        "Should finish dock sequence, got {:?}",
        m.state,
    );
    assert_eq!(m.home_refinery, Some(2), "Home refinery should be set");
    assert!(m.cargo.is_empty(), "Cargo should be empty");
}

/// After Departing arrival, both `target_ore_cell` and `last_harvest_cell` must
/// be cleared so SearchOre re-scans from the exit cell instead of biasing
/// toward the previous patch (which may sit on the back side of the refinery).
#[test]
fn exit_pad_clears_ore_targets_on_arrival() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    // 4×3 refinery at (10, 10). Exit cell = foundation_centroid + (-0x80, +0x80) leptons:
    //   x = (10*256 + 4*128 - 128) / 256 = 11
    //   y = (10*256 + 3*128 + 128) / 256 = 12
    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 12);

    // Set up the miner mid-Departing with stale archive populated.
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::Departing;
    miner.reserved_refinery = Some(100);
    miner.dock_queued = false;
    miner.target_ore_cell = Some((20, 20)); // pre-dock target
    miner.last_harvest_cell = Some((20, 20)); // pre-dock archive

    // Tick the miner system — should detect arrival and run the cleanup.
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(
        miner.state,
        MinerState::SearchOre,
        "must transition to SearchOre"
    );
    assert!(
        miner.target_ore_cell.is_none(),
        "target_ore_cell must be cleared"
    );
    assert!(
        miner.last_harvest_cell.is_none(),
        "last_harvest_cell must be cleared"
    );
    assert!(
        miner.reserved_refinery.is_none(),
        "reserved_refinery must be cleared"
    );
}

/// Departing must NOT transition to SearchOre while a teleport is in progress
/// (`entity.teleport_state.is_some()`). Without this gate a chrono miner
/// mid-warp could leave the dock sub-state machine prematurely.
#[test]
fn exit_pad_blocks_transition_during_teleport() {
    use crate::sim::movement::teleport_movement::{TeleportPhase, TeleportState};

    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 100, 10, 10);
    // Exit cell for the 4×3 refinery at (10, 10) is (11, 12) under the gamemd formula.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 12);

    // Set up miner at the exit cell, in Departing, with a teleport in progress.
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::Departing;
    miner.reserved_refinery = Some(100);
    miner.target_ore_cell = Some((20, 20));
    // Inject an active teleport state to trip the gate.
    entity.teleport_state = Some(TeleportState {
        phase: TeleportPhase::ChronoDelay,
        target_rx: 20,
        target_ry: 20,
        being_warped_ticks: 16,
    });

    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(
        miner.state,
        MinerState::Dock,
        "must stay in Dock state during teleport"
    );
    assert_eq!(
        miner.dock_phase,
        RefineryDockPhase::Departing,
        "must stay in Departing"
    );
    assert_eq!(
        miner.target_ore_cell,
        Some((20, 20)),
        "ore target must NOT be cleared while teleport is active"
    );
}

/// Smoke test for the post-undock flow with a stale archive.
///
/// Sets up a chrono miner mid-Departing with `last_harvest_cell` pointing to a
/// patch outside the local scan radius. After the fix, the archive is cleared
/// at exit and SearchOre runs with the miner's current position as search
/// center, picking the only ore patch in range. Verifies the field-clear
/// behavior end-to-end (state transitions Departing → SearchOre → MoveToOre,
/// archive is cleared, fresh target is picked from current position).
///
/// NOTE: this does NOT verify the headbutt symptom is fixed. The fix clears
/// the archive but the search algorithm still picks by geometric distance
/// (no pathfinding-aware reachability). If a back-side ore patch is the
/// closest in the user's scenario, the headbutt may recur. That hypothesis
/// must be tested in-game.
#[test]
fn chrono_miner_archive_cleared_after_undock_picks_new_target() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    // Refinery at (10, 10), 4x3 foundation. Gamemd-formula exit cell = (11, 12).
    spawn_refinery(&mut sim, 100, 10, 10);

    // Place ONE ore patch at (15, 13): within local_continuation_radius
    // (default 6) of exit cell (11, 12). This is what the fresh local scan
    // from current position should pick.
    sim.production.resource_nodes.insert(
        (15, 13),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 1200,
        },
    );

    // Spawn miner at exit cell (11, 12), mid-Departing. Stale archive points
    // far away (50, 50) — outside any scan radius from current position,
    // and no ore at that cell. If the archive were NOT cleared, the search
    // would start from (50, 50), the local scan would find nothing, the
    // archive check would also find nothing, and only the long scan would
    // eventually fall back to current position. With the fix the local scan
    // from current position immediately picks (15, 13).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 12);
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::Departing;
    miner.reserved_refinery = Some(100);
    miner.target_ore_cell = Some((50, 50));
    miner.last_harvest_cell = Some((50, 50));
    miner.cargo.clear();

    // Tick twice: (1) Departing → SearchOre with cleared archive,
    // (2) SearchOre → MoveToOre with target picked.
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");

    // The stale (50, 50) target must be replaced. Any other value (None or
    // Some((15, 13))) is acceptable — the precise target depends on which
    // tick SearchOre ran in. The key property: the stale archive does not
    // survive the dock cycle.
    assert_ne!(
        miner.target_ore_cell,
        Some((50, 50)),
        "stale archive must be replaced after Departing → SearchOre. \
         Got state={:?}, target={:?}",
        miner.state,
        miner.target_ore_cell,
    );

    // After the second tick, the only available ore should be the picked target.
    if let Some(target) = miner.target_ore_cell {
        assert_eq!(
            target,
            (15, 13),
            "the only ore at (15, 13) should be picked. Got {:?}",
            target
        );
    }
}

/// Ore in a disconnected zone (cut off by impassable terrain) must be
/// filtered out by the reachability check. With no reachable ore on the
/// map, the harvester transitions to WaitNoOre rather than picking the
/// unreachable cell.
#[test]
fn unreachable_ore_filtered_out() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Build a 16x16 path grid with an impassable wall column at x=8 that
    // splits the map into two zones (left and right halves).
    let mut grid = PathGrid::new(16, 16);
    for y in 0..16u16 {
        grid.set_blocked(8, y, true);
    }
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 16, 16);
    sim.zone_grid = Some(zone_grid);

    // Harvester on the LEFT side at (3, 8). Ore on the RIGHT side at (12, 8).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 3, 8);
    place_ore(&mut sim, 12, 8, 1200);

    // Drive the miner into SearchOre state.
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::SearchOre;
    }

    // Tick once — search runs, finds nothing reachable, transitions to WaitNoOre.
    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.state,
        MinerState::WaitNoOre,
        "must wait — only ore on the map is in a disconnected zone, so unreachable",
    );
    assert!(
        m.target_ore_cell.is_none(),
        "must not have targeted unreachable ore, got {:?}",
        m.target_ore_cell,
    );
}

/// When a closer ore cell is unreachable (different zone) but a farther
/// one is reachable, the harvester must pick the farther reachable cell
/// rather than fall through to WaitNoOre.
#[test]
fn reachable_ore_picked_over_closer_unreachable() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // 16x16 grid with an impassable wall column at x=8.
    let mut grid = PathGrid::new(16, 16);
    for y in 0..16u16 {
        grid.set_blocked(8, y, true);
    }
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 16, 16);
    sim.zone_grid = Some(zone_grid);

    // Harvester at (3, 8) on the LEFT side.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 3, 8);
    // Closer ore at (10, 8) is on the RIGHT side (unreachable).
    place_ore(&mut sim, 10, 8, 1200);
    // Farther ore at (1, 1) is on the LEFT side (reachable).
    place_ore(&mut sim, 1, 1, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::SearchOre;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.state, MinerState::MoveToOre);
    assert_eq!(
        m.target_ore_cell,
        Some((1, 1)),
        "reachable farther ore at (1,1) must be picked over unreachable closer ore at (10,8). \
         Got {:?}",
        m.target_ore_cell,
    );
}

/// When the harvester is standing on a cell marked impassable in the path
/// grid (mirrors mid-harvest on Tiberium), the effective-zone probe must
/// find a valid zone via a neighbor and the filter must still apply.
/// Specifically: nearby reachable ore is picked, distant unreachable ore
/// is filtered.
#[test]
fn harvester_on_tiberium_falls_back_to_neighbor_zone() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // 16x16 grid. Wall column at x=8 splits LEFT and RIGHT zones.
    // Harvester's cell at (3, 8) is also blocked (simulates standing on
    // Tiberium that the path grid marks impassable).
    let mut grid = PathGrid::new(16, 16);
    for y in 0..16u16 {
        grid.set_blocked(8, y, true);
    }
    grid.set_blocked(3, 8, true);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 16, 16);
    sim.zone_grid = Some(zone_grid);

    // Harvester at (3, 8) on the blocked cell.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 3, 8);
    // Reachable ore at (5, 8) on the LEFT side.
    place_ore(&mut sim, 5, 8, 1200);
    // Unreachable ore at (10, 8) on the RIGHT side.
    place_ore(&mut sim, 10, 8, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::SearchOre;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.state, MinerState::MoveToOre);
    assert_eq!(
        m.target_ore_cell,
        Some((5, 8)),
        "left-side reachable ore must be picked even with the harvester on a \
         blocked cell — the effective-zone probe finds a passable neighbor. \
         Got {:?}",
        m.target_ore_cell,
    );
}

/// End-to-end pin for the head-butt-after-unload fix. Exercises the full
/// chain: phase_exit_pad's bypass_grid drive → arrival → SearchOre → A*
/// from a blocked-start cell → MoveToOre. Uses a real PathGrid with the
/// refinery foundation blocked, so the test would FAIL without the
/// bypass_grid wiring AND the A* start-relaxation.
#[test]
fn harvester_undocks_through_foundation_to_outside_ore() {
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::rng::SimRng;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    // 4x3 GAREFN at (10, 10) — foundation occupies (10..=13, 10..=12).
    spawn_refinery(&mut sim, 100, 10, 10);

    // Ore patch at (11, 14) — south of the foundation, reachable once the
    // harvester clears the south edge.
    place_ore(&mut sim, 11, 14, 1200);

    // PathGrid with the foundation footprint blocked. This is the critical
    // setup that makes the test meaningful — without it, movement_step's
    // walkability check would succeed regardless of bypass_grid.
    let mut path_grid = PathGrid::new(32, 32);
    path_grid.block_building_footprint(10, 10, "4x3", &[], &[]);

    // Harvester at the dock pad (13, 11), cargo emptied, dock_phase=Departing.
    // Simulates "just finished unloading".
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("harvester entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.clear();
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(100);
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);

    // Tick the full pipeline: miner state machine + movement with the
    // blocked-footprint path_grid. Use enough ticks for: drive to exit
    // (~17 ticks for diagonal-ish (-2, +1) at HARV speed) + arrival +
    // SearchOre + A* + drive south toward ore.
    let alliances = HouseAllianceMap::new();
    let terrain_costs = BTreeMap::new();
    let mut occupancy = OccupancyGrid::new();
    let mut rng = SimRng::new(0);

    for _tick in 0..120 {
        crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));
        crate::sim::movement::tick_movement_with_grid(
            &mut sim.entities,
            Some(&path_grid),
            &terrain_costs,
            &alliances,
            &mut occupancy,
            &mut rng,
            67,
            sim.tick,
            &mut sim.interner,
        );
        sim.tick += 1;
    }

    let entity = sim.entities.get(miner_id).expect("harvester still alive");
    let miner = entity.miner.as_ref().expect("miner component");

    // (1) Harvester transitioned out of Dock state — phase_exit_pad reached
    //     the arrival branch and ran cleanup.
    assert_ne!(
        miner.state,
        MinerState::Dock,
        "harvester should have transitioned out of Dock; pos=({},{}) state={:?}",
        entity.position.rx,
        entity.position.ry,
        miner.state,
    );

    // (2) phase_exit_pad cleared the dock reservation on arrival.
    assert!(
        miner.reserved_refinery.is_none(),
        "phase_exit_pad should have cleared reserved_refinery; got {:?}",
        miner.reserved_refinery,
    );

    // (3) Harvester either escaped the foundation south edge OR is targeting
    //     the ore patch — both prove SearchOre + A* succeeded from the
    //     (formerly blocked) start cell.
    let escaped = entity.position.ry > 12 || entity.position.rx < 10 || entity.position.rx > 13;
    let targeting = miner.target_ore_cell == Some((11, 14));
    assert!(
        escaped || targeting,
        "harvester should have escaped foundation or be targeting ore; \
         pos=({},{}) target_ore={:?} state={:?}",
        entity.position.rx,
        entity.position.ry,
        miner.target_ore_cell,
        miner.state,
    );
}

/// End-to-end pin for the foundation-bump bug. Places a refinery at (10, 10)
/// with its foundation cells registered in OccupancyGrid (the real-game
/// configuration), then drives a harvester into the pad. Asserts the refinery's
/// position is unchanged and it never receives a movement_target — i.e. the
/// bypass_grid filter prevents the building from being treated as a scatter
/// candidate when the harvester crosses into a foundation cell.
#[test]
fn harvester_drives_into_refinery_foundation_without_bumping_it() {
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::occupancy::OccupancyGrid;
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::rng::SimRng;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    // 4x3 GAREFN at (10, 10) — foundation occupies (10..=13, 10..=12).
    // spawn_refinery returns (); EntityStore is keyed by stable_id, so we use
    // the sid we passed in (100) as the entity_id directly.
    spawn_refinery(&mut sim, 100, 10, 10);
    let refinery_id: u64 = 100;
    // Capture initial position fields. Position is Clone but not Copy, so we
    // can't `let p = entity.position` through a borrow — read individual
    // fields into primitives instead.
    let (rx_before, ry_before, sub_x_before, sub_y_before) = {
        let r = sim
            .entities
            .get(refinery_id)
            .expect("refinery just spawned");
        (
            r.position.rx,
            r.position.ry,
            r.position.sub_x,
            r.position.sub_y,
        )
    };

    // Register foundation cells in OccupancyGrid (the real-game configuration —
    // this is what the existing undock test omits, which is why it didn't catch
    // the bump bug).
    let mut occupancy = OccupancyGrid::new();
    for ry in 10u16..=12 {
        for rx in 10u16..=13 {
            occupancy.add(rx, ry, refinery_id, MovementLayer::Ground, None);
        }
    }

    let mut path_grid = PathGrid::new(32, 32);
    path_grid.block_building_footprint(10, 10, "4x3", &[], &[]);

    // Harvester at queue cell (14, 11), state=Dock, dock_phase=Approach.
    // Reservation already held; first tick re-targets the pad and goes Linked.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("harvester entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.reserved_refinery = Some(100);
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);

    let alliances = HouseAllianceMap::new();
    let terrain_costs = BTreeMap::new();
    let mut rng = SimRng::new(0);

    // Tick enough for: drive 1 cell west onto the pad. 60 ticks gives plenty of slack.
    for _ in 0..60 {
        crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));
        crate::sim::movement::tick_movement_with_grid(
            &mut sim.entities,
            Some(&path_grid),
            &terrain_costs,
            &alliances,
            &mut occupancy,
            &mut rng,
            67,
            sim.tick,
            &mut sim.interner,
        );
        sim.tick += 1;
    }

    let refinery = sim.entities.get(refinery_id).expect("refinery still alive");

    // (1) Refinery position is exactly unchanged.
    assert_eq!(
        refinery.position.rx, rx_before,
        "refinery rx must not change when harvester docks; got rx={}",
        refinery.position.rx,
    );
    assert_eq!(
        refinery.position.ry, ry_before,
        "refinery ry must not change when harvester docks; got ry={}",
        refinery.position.ry,
    );
    assert_eq!(
        refinery.position.sub_x, sub_x_before,
        "refinery sub_x must not change",
    );
    assert_eq!(
        refinery.position.sub_y, sub_y_before,
        "refinery sub_y must not change",
    );

    // (2) Refinery never received a movement_target.
    assert!(
        refinery.movement_target.is_none(),
        "refinery must not have a movement_target — buildings cannot scatter",
    );

    // (3) Harvester drove past the queue cell. After 60 ticks it should be
    // at the pad cell or further along the dock sequence — definitely not
    // still at queue (14, 11) which would indicate sub-cell oscillation
    // when crossing into a foundation cell.
    let harvester = sim.entities.get(miner_id).expect("harvester still alive");
    assert_ne!(
        (harvester.position.rx, harvester.position.ry),
        (14u16, 11u16),
        "harvester must have driven past the queue cell into the foundation; \
         oscillating in place at queue means a deferred-occupancy check is \
         bouncing it back. phase={:?}",
        harvester.miner.as_ref().map(|m| m.dock_phase),
    );
}

// ===========================================================================
// New tests for the collapsed FSM: Approach/Linked/Unloading/Departing.
// ===========================================================================

/// Approach phase polls the dock reservation each tick and transitions to
/// Linked the moment the reservation is granted, issuing a direct move to the
/// pad cell. RemoveOccupy keeps the pad cell walkable so no bypass_grid hack
/// is required.
#[test]
fn approach_to_linked_on_reservation_grant() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.reserved_refinery = Some(2);
    }

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::Linked);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.movement_target.is_some(),
        "issue_direct_move should have set movement_target on reservation grant"
    );
}

/// Unloading emits one BaleDepositEvent per bale popped.
#[test]
fn unloading_emits_bale_event_per_bale() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Place miner directly in Unloading at the pad cell.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..5 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.unload_timer = 0;
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    // Tick enough to cycle through 5 bales (~14 ticks each, plus the trailing
    // empty-cargo tick that transitions to Departing).
    tick_miners_n(&mut sim, &rules, 200);

    assert_eq!(
        sim.bale_events.len(),
        5,
        "expected one BaleDepositEvent per bale popped, got {} events",
        sim.bale_events.len(),
    );
    for event in &sim.bale_events {
        assert_eq!(event.building_id, 2);
    }
}

/// Build a minimal rules with HARV + GAREFN (Refinery) + GAPURI (OrePurifier).
fn purifier_rules(bonus_pct: i32) -> RuleSet {
    let ini = IniFile::from_str(&format!(
        "[General]\nPurifierBonus={}\n\
         [InfantryTypes]\n\
         [VehicleTypes]\n0=HARV\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n0=GAREFN\n1=GAPURI\n\
         [HARV]\n\
         Name=War Miner\nCost=1400\nStrength=600\nArmor=heavy\nSpeed=4\nROT=5\nSight=5\n\
         TechLevel=1\nOwner=Americans\nHarvester=yes\nDock=GAREFN\n\
         [GAREFN]\n\
         Name=Ore Refinery\nCost=2000\nStrength=900\nArmor=wood\nTechLevel=1\n\
         Owner=Americans\nFoundation=4x3\nRefinery=yes\n\
         [GAPURI]\n\
         Name=Ore Purifier\nCost=2500\nStrength=1000\nArmor=wood\nTechLevel=1\n\
         Owner=Americans\nFoundation=2x2\nOrePurifier=yes\n",
        // Rules expects PurifierBonus= as a fraction; we use the integer-pct path
        // by writing the fraction value (e.g., 0.25 → 25%). Use the float string.
        bonus_pct as f32 / 100.0,
    ));
    RuleSet::from_ini(&ini).expect("purifier rules")
}

/// Per-bale purifier bonus is applied inline as each bale is deposited
/// (matches gamemd's per-bale credit application). With one bale (value 100)
/// and a 25% PurifierBonus, total credits gain = 100 + 25 = 125.
#[test]
fn unloading_applies_per_bale_purifier_bonus() {
    let mut sim = Simulation::new();
    let rules = purifier_rules(25);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    // Spawn an OrePurifier-flagged building owned by the same player.
    spawn_structure(&mut sim, 3, "GAPURI", 20, 20);

    let credits_before = credits_for_owner(&sim, "Americans");

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 100,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.unload_timer = 0;
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    tick_miners_n(&mut sim, &rules, 200);

    let credits_after = credits_for_owner(&sim, "Americans");
    assert_eq!(
        credits_after - credits_before,
        125,
        "100 base + 25 (25% purifier) = 125, got delta {}",
        credits_after - credits_before,
    );
}

/// Exit cell formula matches gamemd: foundation_centroid_lepton + (-0x80, +0x80).
#[test]
fn departing_uses_gamemd_exit_cell_formula() {
    use super::miner_dock_sequence::refinery_exit_cell;
    // 4x3 refinery at (10, 20):
    //   x = (10*256 + 4*128 - 128) / 256 = 2944 / 256 = 11
    //   y = (20*256 + 3*128 + 128) / 256 = 5632 / 256 = 22
    assert_eq!(refinery_exit_cell(10, 20, 4, 3), (11, 22));
    // 3x3 refinery at (5, 5):
    //   x = (1280 + 384 - 128)/256 = 1536/256 = 6
    //   y = (1280 + 384 + 128)/256 = 1792/256 = 7
    assert_eq!(refinery_exit_cell(5, 5, 3, 3), (6, 7));
    // 1x1 refinery at origin: centroid = (128, 128); exit_lepton = (0, 256) → cell (0, 1).
    assert_eq!(refinery_exit_cell(0, 0, 1, 1), (0, 1));
}

/// Departing snaps facing to 0x47 (east-southeast) and returns to SearchOre
/// on arrival at the exit cell.
#[test]
fn departing_snaps_facing_to_0x47() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 100, 10, 10);
    // Place miner at the gamemd-formula exit cell (11, 12) for the 4×3 refinery at (10, 10).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 11, 12);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.facing = 0; // pre-set facing != 0x47
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(100);
    }

    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("entity");
    assert_eq!(
        entity.facing, 0x47,
        "Departing arrival must snap facing to 0x47, got {:#x}",
        entity.facing,
    );
    let m = entity.miner.as_ref().expect("miner component");
    assert_eq!(m.state, MinerState::SearchOre);
    assert!(m.reserved_refinery.is_none());
}

/// Linked sets the UnloadingClass display override and emits a DockDeploy
/// sound on pad arrival, then transitions to Unloading.
#[test]
fn linked_to_unloading_on_pad_arrival() {
    use crate::sim::world::SimSoundEvent;
    let mut sim = Simulation::new();
    // Custom rules with UnloadingClass=HORV on HARV so the override path runs.
    let rules = {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n[VehicleTypes]\n0=HARV\n[AircraftTypes]\n\
             [BuildingTypes]\n0=GAREFN\n\
             [HARV]\nName=War Miner\nCost=1400\nStrength=600\nArmor=heavy\nSpeed=4\n\
             ROT=5\nSight=5\nTechLevel=1\nOwner=Americans\nHarvester=yes\n\
             Dock=GAREFN\nUnloadingClass=HORV\n\
             [GAREFN]\nName=Ore Refinery\nCost=2000\nStrength=900\nArmor=wood\n\
             TechLevel=1\nOwner=Americans\nFoundation=4x3\nRefinery=yes\n",
        );
        RuleSet::from_ini(&ini).expect("custom rules")
    };
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 2, 10, 10);
    // Place miner at the pad cell with no movement_target → simulates arrival.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.movement_target = None;
        entity.display_type_override = None;
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Linked;
        miner.reserved_refinery = Some(2);
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::Unloading);
    assert_eq!(m.unload_timer, 0);

    let entity = sim.entities.get(miner_id).expect("entity");
    let override_id = entity
        .display_type_override
        .expect("UnloadingClass override should be set");
    assert_eq!(sim.interner.resolve(override_id), "HORV");

    let dock_deploy_count = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::DockDeploy { building_id: 2 }))
        .count();
    assert_eq!(
        dock_deploy_count, 1,
        "Linked → Unloading must emit one DockDeploy sound for refinery 2"
    );
}

/// End-to-end dock cycle: war miner forced-returns to a refinery, drives onto
/// the pad, deposits N bales, drives off the exit cell. Verifies bale event
/// count, total credits, final position, final facing, dock release.
#[test]
fn full_dock_cycle_war_miner() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);

    // Pre-load 10 bales (smaller than full capacity to keep test fast).
    let bale_count: i32 = 10;
    let bale_value: i32 = 25;
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..bale_count {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: bale_value as u16,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.reserved_refinery = Some(100);
    }

    let credits_before = credits_for_owner(&sim, "Americans");

    // Tick enough for: Approach → Linked (1 tick) + drive onto pad +
    // 10 bales × ~14 ticks unload + drive to exit cell + arrival snap.
    tick_miners_n(&mut sim, &rules, 400);

    // Bale events: one per bale.
    assert_eq!(
        sim.bale_events.len(),
        bale_count as usize,
        "expected {} bale events, got {}",
        bale_count,
        sim.bale_events.len(),
    );

    // Credits: bale_count * bale_value (no purifier in miner_rules).
    let credits_after = credits_for_owner(&sim, "Americans");
    assert_eq!(
        credits_after - credits_before,
        bale_count * bale_value,
        "expected +{} credits, got delta {}",
        bale_count * bale_value,
        credits_after - credits_before,
    );

    // Final position at the exit cell (4×3 refinery at (10, 10) → exit (11, 12)).
    let entity = sim.entities.get(miner_id).expect("entity");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (11, 12),
        "miner should land at gamemd-formula exit cell"
    );
    assert_eq!(entity.facing, 0x47, "facing must snap to 0x47 on arrival");

    let m = entity.miner.as_ref().expect("miner");
    // After Departing → SearchOre, with no ore on the map the miner falls
    // through to WaitNoOre. Either is a valid post-dock state.
    assert!(
        matches!(m.state, MinerState::SearchOre | MinerState::WaitNoOre),
        "post-dock state must be SearchOre or WaitNoOre, got {:?}",
        m.state,
    );
    assert!(m.cargo.is_empty(), "cargo must be drained");
    assert!(
        m.reserved_refinery.is_none(),
        "reservation must be released"
    );
    assert!(
        !sim.production.dock_reservations.is_occupied(100),
        "dock must be free for the next miner"
    );
}
