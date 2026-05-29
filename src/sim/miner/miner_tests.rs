//! Acceptance tests for the miner (harvester) state machine system.
//!
//! Tests exercise the miner_system::tick_miners() pipeline with a minimal
//! EntityStore: miner entity + refinery structure + resource nodes. Verifies
//! payout math, dock queuing, Chrono teleport rules, incremental unloading,
//! local continuation, pip display, and refinery rebinding.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::locomotor_type::LocomotorKind;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;
use crate::sim::miner::{
    CargoBale, Miner, MinerConfig, MinerKind, MinerState, RefineryDockPhase, ResourceNode,
    ResourceType,
};
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
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
    if kind == MinerKind::Chrono {
        ge.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Teleport));
    }
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
    occupy_structure_cells(sim, sid, rx, ry, 4, 3);
    if sim.next_stable_entity_id <= sid {
        sim.next_stable_entity_id = sid + 1;
    }
}

fn spawn_structure(sim: &mut Simulation, sid: u64, type_id: &str, rx: u16, ry: u16) {
    spawn_structure_owned(sim, sid, type_id, "Americans", rx, ry);
}

fn spawn_structure_owned(
    sim: &mut Simulation,
    sid: u64,
    type_id: &str,
    owner: &str,
    rx: u16,
    ry: u16,
) {
    let owner_id = sim.interner.intern(owner);
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
    occupy_structure_cells(sim, sid, rx, ry, 1, 1);
    if sim.next_stable_entity_id <= sid {
        sim.next_stable_entity_id = sid + 1;
    }
}

fn occupy_structure_cells(
    sim: &mut Simulation,
    sid: u64,
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
) {
    for y in ry..ry.saturating_add(height) {
        for x in rx..rx.saturating_add(width) {
            sim.occupancy.add(
                x,
                y,
                sid,
                MovementLayer::Ground,
                None,
                CellListInsertion::AppendBuilding,
            );
        }
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
        sim.total_sim_ms = sim.total_sim_ms.saturating_add(67);
        sim.binary_frame = ((sim.total_sim_ms * 15) / 1000) as u32;
        crate::sim::movement::teleport_movement::tick_teleport_movement(
            &mut sim.entities,
            &mut OccupancyGrid::new(),
            67,
            sim.tick,
            None,
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
// Test 5: Chrono Miner teleports on far return (position snaps to QueueingCell)
// ==========================================================================
#[test]
fn chrono_miner_teleports_to_refinery_on_return() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

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

    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_some(),
        "Chrono Miner should have an active teleport after first return tick"
    );
    let teleport = entity.teleport_state.as_ref().expect("teleport state");
    assert_eq!(
        (teleport.target_rx, teleport.target_ry),
        (14, 11),
        "Far return should stage at QueueingCell, not the refinery pad"
    );
    assert_eq!(
        entity.miner.as_ref().and_then(|m| m.reserved_refinery),
        Some(2),
        "Return target should be selected before docking contact"
    );
    let loco = entity.locomotor.as_ref().expect("locomotor");
    assert_eq!(loco.active_kind(), LocomotorKind::Teleport);
    assert_eq!(loco.primary_kind(), LocomotorKind::Teleport);
    assert!(loco.piggyback.is_none());
    assert!(!loco.is_overridden());

    crate::sim::movement::teleport_movement::tick_teleport_movement(
        &mut sim.entities,
        &mut OccupancyGrid::new(),
        67,
        sim.tick,
        None,
    );

    let entity = sim.entities.get(miner_id).expect("entity");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (14, 11),
        "Position should snap to the QueueingCell staging cell after Relocate"
    );
    assert!(
        entity.teleport_state.is_none(),
        "Harvester teleport cleanup should clear TeleportState in the relocate tick"
    );
    let loco = entity.locomotor.as_ref().expect("locomotor");
    assert_eq!(loco.active_kind(), LocomotorKind::Teleport);
    assert_eq!(loco.primary_kind(), LocomotorKind::Teleport);
    assert!(loco.piggyback.is_none());
    assert!(!loco.is_overridden());
}

#[test]
fn chrono_far_return_uses_passable_search_from_queueing_cell() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let mut grid = PathGrid::new(64, 64);
    grid.set_blocked(14, 11, true);

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

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("entity");
    let teleport = entity.teleport_state.as_ref().expect("teleport state");
    assert_eq!(
        (teleport.target_rx, teleport.target_ry),
        (13, 10),
        "blocked QueueingCell should use passable search, not the refinery pad"
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
fn return_close_enough_to_refinery_enters_dock() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    // GAREFN at (85,180) has the radio dock target at (88,181). A miner
    // approaching from the south can be stopped by movement CloseEnough at
    // (88,183), two cells away, after the footprint blocks the next step.
    let miner_id = spawn_miner(&mut sim, 100, MinerKind::War, 88, 183);
    spawn_refinery(&mut sim, 99, 85, 180);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..20 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::ReturnToRefinery;
        miner.reserved_refinery = Some(99);
    }

    let grid = PathGrid::new(276, 276);
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(miner.state, MinerState::Dock);
    assert_eq!(miner.dock_phase, RefineryDockPhase::Approach);
}

#[test]
fn chrono_return_close_enough_enters_radio_dock_without_can_dock_move() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    let miner_id = spawn_miner(&mut sim, 100, MinerKind::Chrono, 88, 183);
    spawn_refinery(&mut sim, 99, 85, 180);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..20 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::ReturnToRefinery;
        miner.reserved_refinery = Some(99);
    }

    let grid = PathGrid::new(276, 276);
    sim.sound_events.clear();
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(
        miner.state,
        MinerState::Dock,
        "close chrono return should enter the radio dock sequence immediately"
    );
    assert_eq!(
        miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "accepted close-return HELLO queues Mission_Enter for the next tick"
    );
    assert!(entity.teleport_state.is_none());
    assert!(
        entity.movement_target.is_none(),
        "HELLO acceptance must not issue the accepted-cell move in the same tick"
    );
    assert!(
        sim.production.dock_reservations.has_contact(99, miner_id),
        "close-return HELLO should populate the refinery contact list"
    );
    assert!(
        sim.sound_events.iter().all(|event| !matches!(
            event,
            crate::sim::world::SimSoundEvent::ChronoTeleport { .. }
        )),
        "near return must not emit chrono teleport sounds"
    );
}

#[test]
fn chrono_return_exact_dock_cell_enters_dock() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    let miner_id = spawn_miner(&mut sim, 100, MinerKind::Chrono, 88, 181);
    spawn_refinery(&mut sim, 99, 85, 180);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
        miner.reserved_refinery = Some(99);
    }

    let grid = PathGrid::new(276, 276);
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.state, MinerState::Dock);
    assert_eq!(miner.dock_phase, RefineryDockPhase::MissionEnter);
}

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

    // Miner with lower stable_id (1) processes first, wins HELLO contact,
    // and queues Mission_Enter. m2 is denied HELLO/contact but keeps the
    // receiver-style CAN_DOCK retry path; the refinery contact list is not
    // evicted/replaced.
    assert_eq!(
        m1_miner.state,
        MinerState::Dock,
        "First miner should still be docking"
    );
    assert_eq!(
        m1_miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "First miner should queue Mission_Enter after HELLO/ROGER"
    );
    assert_eq!(
        m2_miner.state,
        MinerState::Dock,
        "Second miner should still be docking"
    );
    assert_eq!(
        m2_miner.dock_phase,
        RefineryDockPhase::Approach,
        "Second miner should remain in HELLO retry/staging until the refinery contact frees"
    );
    assert!(
        sim.production.dock_reservations.has_contact(2, m1),
        "busy refinery must keep the current HELLO contact"
    );
    assert!(
        !sim.production.dock_reservations.has_contact(2, m2),
        "incoming full HELLO must not evict or replace Contacts[0]"
    );
    assert!(
        sim.production.dock_reservations.is_waiting(2, m2),
        "busy stock refinery reply should leave the second miner in retry order"
    );
}

// ==========================================================================
// Test 8: Credits arrive per slot drain (whole-slot dump per timer tick)
// ==========================================================================
/// gamemd dumps an entire StorageClass slot (all bales of one resource type)
/// per HarvesterDumpRate threshold crossing. Pure-ore cargo drains in one
/// dump tick (~15 frames after dock-link); mixed ore+gems drains in two.
/// Test pure-ore (1 slot) and mixed (2 slots) and assert each slot fully
/// arrives on a single tick.
#[test]
fn credits_arrive_per_slot_during_unload() {
    // --- Pure ore (1 slot) ---
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

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
        miner.unload_timer = 0;
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    let before = credits_for_owner(&sim, "Americans");

    // Single tick at timer=0 drains the entire ore slot → 10 × 25 = 250
    // credits in one shot.
    tick_miners_n(&mut sim, &rules, 1);
    let after = credits_for_owner(&sim, "Americans");
    assert_eq!(
        after - before,
        250,
        "pure-ore cargo must drain in one slot dump (250 cr in one tick)",
    );

    // --- Mixed ore + gems (2 slots) ---
    let mut sim = Simulation::new();
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..10 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        for _ in 0..5 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Gem,
                value: 50,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.unload_timer = 0;
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    let before = credits_for_owner(&sim, "Americans");
    tick_miners_n(&mut sim, &rules, 1);
    let after_first_drain = credits_for_owner(&sim, "Americans");
    assert_eq!(
        after_first_drain - before,
        250,
        "first drain must be ORE slot (slot 0) = 10 × 25 = 250 cr",
    );

    // Second drain fires one full unload_tick_interval later. With the
    // decrement-then-check structure (timer -= 10 happens BEFORE the drain
    // check), timer crosses ≤ 0 on the 16th tick after the first drain
    // (144 → 134 → ... → 4 → -6 → drain).
    tick_miners_n(&mut sim, &rules, 16);
    let after_second_drain = credits_for_owner(&sim, "Americans");
    assert_eq!(
        after_second_drain - before,
        250 + 250,
        "second drain must be GEM slot = 5 × 50 = 250 cr (total 500)",
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
    // 2 density levels at (20, 20) and a richer patch nearby (within local
    // continuation radius of 6 cells).
    place_ore(&mut sim, 20, 20, 2 * 120);
    place_ore(&mut sim, 22, 20, 100 * 120);

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
    // Cell at miner's position: 2 density levels (2 × ore-base 120 = 240).
    place_ore(&mut sim, 20, 20, 2 * 120);
    // Nearby ore well within TiberiumShortScan (radius 6 cells).
    place_ore(&mut sim, 23, 20, 100 * 120);

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
    // Only the miner's cell has ore (2 density levels = 240 base units).
    // Nothing within the short-scan radius (default 6 cells). The further
    // ore patch is well outside.
    place_ore(&mut sim, 20, 20, 2 * 120);
    place_ore(&mut sim, 50, 50, 100 * 120); // far outside local_continuation_radius

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
// Test 9c: EMPTY-cargo cell depletion + short-scan miss → return to refinery
//          (gamemd case-1 parity: cargo is irrelevant to the miss → state 2
//           transition. Empty miners detour home before re-scanning, matching
//           gamemd's observable travel path.)
// ==========================================================================
#[test]
fn empty_cargo_cell_depletion_returns_to_refinery() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // No ore on the miner's cell. Nothing within short-scan radius (6 cells).
    // The far ore patch is outside short-scan; gamemd does NOT run a long
    // scan from case 1 — it transitions to state 2 (return) on miss.
    place_ore(&mut sim, 40, 20, 100);

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
    assert!(
        matches!(miner.state, MinerState::ReturnToRefinery | MinerState::Dock),
        "Empty-cargo miner on a depleted cell with no short-scan hit should \
         head to the refinery (gamemd state 2); state was {:?}",
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

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 80, 80);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::ForcedReturn;
        miner.forced_return = true;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_some(),
        "Forced return should issue an inbound chrono teleport"
    );
    let teleport = entity.teleport_state.as_ref().expect("teleport state");
    assert_eq!((teleport.target_rx, teleport.target_ry), (14, 11));
    assert_eq!(
        entity.miner.as_ref().and_then(|m| m.reserved_refinery),
        Some(2),
        "Forced return should select a refinery target before docking contact"
    );
}

#[test]
fn chrono_return_within_too_far_threshold_uses_close_radio_path() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::from_general_rules(&rules.general);
    let grid = PathGrid::new(64, 64);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 40, 40);
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

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_none(),
        "Chrono Miner inside ChronoHarvTooFarDistance should not take the far QueueingCell fallback"
    );
    assert_eq!(
        entity.miner.as_ref().and_then(|m| m.reserved_refinery),
        Some(2)
    );
    let movement = entity
        .movement_target
        .as_ref()
        .expect("close return should path toward the accepted dock cell");
    assert_eq!(
        movement
            .final_goal
            .or_else(|| movement.path.last().copied()),
        Some((13, 11)),
        "close return should use the refinery CAN_DOCK accepted cell, not QueueingCell"
    );
}

// ==========================================================================
// Chrono close/far return radio threshold pins.
// ==========================================================================
#[test]
fn chrono_return_at_exact_too_far_threshold_uses_close_radio_path() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::from_general_rules(&rules.general);
    let grid = PathGrid::new(96, 96);

    spawn_refinery(&mut sim, 2, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 60, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert!(
        entity.teleport_state.is_none(),
        "strict > threshold means exactly 50 cells is still the close radio path"
    );
    assert_eq!(miner.state, MinerState::Dock);
    assert_eq!(miner.dock_phase, RefineryDockPhase::MissionEnter);
    assert!(sim.production.dock_reservations.has_contact(2, miner_id));
}

#[test]
fn chrono_return_over_too_far_threshold_uses_queueingcell_teleport() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::from_general_rules(&rules.general);
    let grid = PathGrid::new(96, 96);

    spawn_refinery(&mut sim, 2, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 61, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("entity");
    let teleport = entity
        .teleport_state
        .as_ref()
        .expect("over-threshold chrono return should teleport");
    assert_eq!(
        (teleport.target_rx, teleport.target_ry),
        (14, 11),
        "far return should land at QueueingCell staging"
    );
    assert!(!sim.production.dock_reservations.has_contact(2, miner_id));
}

#[test]
fn chrono_close_hello_refused_stages_at_queueingcell_without_receiver_eviction() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::from_general_rules(&rules.general);
    let grid = PathGrid::new(64, 64);

    let occupant = spawn_miner(&mut sim, 1, MinerKind::Chrono, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::Chrono, 20, 10);
    assert!(sim.production.dock_reservations.try_reserve(2, occupant));

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::ReturnToRefinery;
    }

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let waiter_entity = sim.entities.get(waiter).expect("waiter entity");
    let waiter_miner = waiter_entity.miner.as_ref().expect("waiter miner");
    assert!(sim.production.dock_reservations.has_contact(2, occupant));
    assert!(
        !sim.production.dock_reservations.has_contact(2, waiter),
        "refused HELLO must not evict or replace the receiver-side contact"
    );
    assert!(sim.production.dock_reservations.is_waiting(2, waiter));
    assert_eq!(waiter_miner.state, MinerState::Dock);
    assert_eq!(waiter_miner.dock_phase, RefineryDockPhase::Approach);
    assert!(waiter_miner.dock_queued);
    let movement = waiter_entity
        .movement_target
        .as_ref()
        .expect("refused close-return miner should stage at QueueingCell");
    assert_eq!(
        movement
            .final_goal
            .or_else(|| movement.path.last().copied()),
        Some((14, 11)),
        "QueueingCell staging must stay distinct from accepted cell (13,11)"
    );
}

#[test]
fn cmin_close_hello_success_defers_can_dock_to_mission_enter() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::from_general_rules(&rules.general);
    let grid = PathGrid::new(64, 64);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 40, 40);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..miner.capacity_bales {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::ReturnToRefinery;
    }

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(miner.state, MinerState::Dock);
    assert_eq!(miner.dock_phase, RefineryDockPhase::MissionEnter);
    assert!(sim.production.dock_reservations.has_contact(2, miner_id));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id),
        "HELLO success must not set the entered flag"
    );
    assert!(
        entity.movement_target.is_none(),
        "HELLO success must not issue CAN_DOCK movement in the same tick"
    );

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let movement = entity
        .movement_target
        .as_ref()
        .expect("MissionEnter should now issue CAN_DOCK movement");
    assert_eq!(
        movement
            .final_goal
            .or_else(|| movement.path.last().copied()),
        Some((13, 11)),
        "CAN_DOCK must use accepted cell, not QueueingCell"
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id)
    );
}

#[test]
fn cmin_refused_close_return_stages_at_queueingcell_then_can_dock_uses_accepted_cell() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::from_general_rules(&rules.general);
    let grid = PathGrid::new(64, 64);

    let occupant = spawn_miner(&mut sim, 1, MinerKind::Chrono, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::Chrono, 20, 10);
    assert!(sim.production.dock_reservations.try_reserve(2, occupant));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, occupant);
    sim.production.dock_reservations.link_on_pad(2, occupant);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        for _ in 0..miner.capacity_bales {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::ReturnToRefinery;
    }

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let waiter_entity = sim.entities.get(waiter).expect("waiter entity");
    let waiter_miner = waiter_entity.miner.as_ref().expect("waiter miner");
    assert_eq!(waiter_miner.state, MinerState::Dock);
    assert_eq!(waiter_miner.dock_phase, RefineryDockPhase::Approach);
    assert!(waiter_miner.dock_queued);
    assert!(sim.production.dock_reservations.is_waiting(2, waiter));
    let movement = waiter_entity
        .movement_target
        .as_ref()
        .expect("refused close-return miner should stage at QueueingCell");
    assert_eq!(
        movement
            .final_goal
            .or_else(|| movement.path.last().copied()),
        Some((14, 11)),
        "refused close return stages at QueueingCell"
    );

    sim.production.dock_reservations.release_on_pad(2, occupant);
    sim.production
        .dock_reservations
        .release_contact(2, occupant);
    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        entity.position.rx = 14;
        entity.position.ry = 11;
        entity.position.refresh_screen_coords();
        entity.movement_target = None;
    }

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));
    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "Approach after release performs HELLO only"
    );
    assert!(sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));
    let waiter_entity = sim.entities.get(waiter).expect("waiter entity");
    let movement = waiter_entity
        .movement_target
        .as_ref()
        .expect("MissionEnter should move from QueueingCell to accepted cell");
    assert_eq!(
        movement
            .final_goal
            .or_else(|| movement.path.last().copied()),
        Some((13, 11)),
        "accepted CAN_DOCK uses NW+(3,1), not QueueingCell"
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
}

// ==========================================================================
// Test: Chrono miner does NOT warp outbound -- only inbound to refinery
// ==========================================================================
/// Regression: chrono miners warp ONLY on the inbound (ore -> refinery)
/// trip. Outbound (refinery -> ore) is a normal drive, matching the
/// original engine's Mission_Harvest state-0 behaviour (which forces a
/// DriveLocomotion piggyback before Set_Destination so the warp branch
/// is skipped). Reintroducing an outbound warp would be observable as
/// a chrono miner vanishing the instant it leaves the pad.
#[test]
fn chrono_miner_does_not_warp_outbound() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let grid = PathGrid::new(64, 64);

    // Chrono miner at the refinery exit cell, empty cargo, entering SearchOre.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    place_ore(&mut sim, 50, 50, 100);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::SearchOre;
        miner.cargo.clear();
    }

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let entity = sim.entities.get(miner_id).expect("entity");
    assert!(
        entity.teleport_state.is_none(),
        "chrono miner must NOT issue a teleport on outbound SearchOre — \
         only the inbound (ore → refinery) leg warps"
    );
    let miner = entity.miner.as_ref().expect("miner");
    assert_eq!(miner.target_ore_cell, Some((50, 50)));
    assert_eq!(miner.state, MinerState::MoveToOre);
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
        .filter(|e| matches!(e, SimSoundEvent::ChronoTeleport { .. }))
        .collect();

    assert_eq!(
        chrono_events.len(),
        2,
        "chrono return should emit one ChronoOut and one ChronoIn sound"
    );
    assert!(
        chrono_events
            .iter()
            .any(|event| matches!(event, SimSoundEvent::ChronoTeleport { rx: 14, ry: 11, .. })),
        "ChronoIn sound should be anchored at the QueueingCell staging cell"
    );
}

/// Stock zero-link refinery completion does not emit the conditional
/// `ReleaseDockedHarvester` departure sound.
#[test]
fn stock_dock_exit_does_not_emit_refinery_exit_sfx() {
    use crate::sim::world::SimSoundEvent;

    let mut sim = Simulation::new();
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=HARV\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GAREFN\n\
         [General]\n\
         [AudioVisual]\n\
         BunkerWallsDownSound=TankBunkerDown\n\
         [HARV]\n\
         Name=War Miner\n\
         Speed=4\n\
         Owner=Americans\n\
         Harvester=yes\n\
         Dock=GAREFN\n\
         [GAREFN]\n\
         Name=Ore Refinery\n\
         Foundation=4x3\n\
         Owner=Americans\n\
         Refinery=yes\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("rules with BunkerWallsDownSound");
    assert_eq!(
        rules.general.bunker_walls_down_sound.as_deref(),
        Some("TankBunkerDown"),
        "parser must read BunkerWallsDownSound from [AudioVisual]"
    );

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(2);
        assert!(
            miner.exit_cell.is_none(),
            "precondition: stock handoff starts without a cached exit cell"
        );
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);
    sim.sound_events.clear();

    // Single tick: stock state-4 handoff. No ReleaseDockedHarvester SFX.
    let config = MinerConfig::default();
    let grid = PathGrid::new(64, 64);
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let refinery_exit_events: Vec<_> = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::RefineryExitSfx { .. }))
        .collect();
    assert!(
        refinery_exit_events.is_empty(),
        "stock zero-link dock completion must not emit RefineryExitSfx"
    );
    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.state, MinerState::SearchOre);
    assert!(miner.exit_cell.is_none());
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
        .filter(|e| matches!(e, SimSoundEvent::ChronoTeleport { .. }))
        .collect();

    assert_eq!(
        chrono_events.len(),
        2,
        "fallback path should emit one ChronoOut and one ChronoIn sound"
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
    assert_eq!(dock, (13, 11));
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

    // Tick 1: Approach -> MissionEnter. HELLO has populated Contacts[], but
    // CAN_DOCK has not yet issued the accepted-cell move.
    tick_miners_n(&mut sim, &rules, 1);
    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::MissionEnter);

    // Tick enough for movement onto pad + per-bale unload + state-4 handoff.
    // 1 bale * 14.4 ticks/bale is about 15 ticks unload, plus pad entry.
    tick_miners_n(&mut sim, &rules, 200);
    let m = get_miner(&sim, miner_id);
    // With only 1 bale (unload_tick_interval=14), unloading takes ~15 ticks.
    // After that, Departing -> SearchOre.
    // After docking, miner transitions to SearchOre. Since there's no ore
    // on the map, it immediately goes to WaitNoOre. Both are valid endpoints.
    assert!(
        m.state == MinerState::SearchOre || m.state == MinerState::WaitNoOre,
        "Miner should complete dock sequence, got state={:?} phase={:?}",
        m.state,
        m.dock_phase,
    );
}

/// Verify the Approach phase grants HELLO contact when free and queues
/// Mission_Enter instead of immediately linking/unloading.
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

    // Contacts[] should contain this miner; pad/contact-entered state has not
    // started yet because CAN_DOCK runs in Mission_Enter.
    assert!(sim.production.dock_reservations.is_occupied(2));
    assert!(sim.production.dock_reservations.has_contact(2, miner_id));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id)
    );
    assert!(!sim.production.dock_reservations.is_on_pad(2, miner_id));
    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::MissionEnter);
    assert!(!m.dock_queued);
}

/// Verify stock pad cell and conditional reciprocal-link release cell helpers.
#[test]
fn refinery_pad_and_conditional_release_cells() {
    use super::miner_dock_sequence::{
        refinery_can_dock_queue_cell, refinery_exit_cell, refinery_pad_cell, refinery_queue_cell,
    };

    let grid = PathGrid::test_all_passable(64, 64);

    // 4×3 foundation at (10, 10), no art.ini overrides:
    //   queue = (14, 11), pad = (13, 11), conditional release = queue.
    // Stock zero-link unload completion does not call this release helper.
    assert_eq!(refinery_queue_cell(10, 10, 4, 3, None), (14, 11));
    assert_eq!(refinery_pad_cell(10, 10, 4, 3, None), (13, 11));
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&grid), None, 0),
        (14, 11),
    );

    // 3×3 foundation at (5, 5), no art.ini overrides:
    //   queue = (8, 6), pad = (8, 6), conditional release = queue.
    assert_eq!(refinery_queue_cell(5, 5, 3, 3, None), (8, 6));
    assert_eq!(refinery_pad_cell(5, 5, 3, 3, None), (8, 6));
    assert_eq!(
        refinery_exit_cell(5, 5, 3, 3, None, Some(&grid), None, 0),
        (8, 6),
    );

    // 2×2 foundation at (20, 20): queue/release = (22, 21).
    assert_eq!(
        refinery_exit_cell(20, 20, 2, 2, None, Some(&grid), None, 0),
        (22, 21)
    );

    // QueueingCell override unchanged:
    assert_eq!(refinery_queue_cell(10, 10, 4, 3, Some((4, 1))), (14, 11));
    assert_eq!(refinery_queue_cell(10, 10, 4, 3, Some((3, 2))), (13, 12));
    assert_eq!(
        refinery_can_dock_queue_cell(10, 10),
        (13, 11),
        "CAN_DOCK receiver target is hardcoded NW+(3,1), not art QueueingCell=4,1",
    );

    // Fallback: no path grid → return QueueingCell.
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, None, None, 0),
        (14, 11)
    );
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, Some((3, 2)), None, None, 0),
        (13, 12)
    );
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

/// gamemd parity: credits from a harvester deposit go to the REFINERY OWNER,
/// not to the harvester's current controller. Simulates a mind-control
/// scenario by spawning a refinery owned by "Americans" and overriding the
/// harvester's owner to "Russians" (as Yuri's mind-control would do). The
/// ore drop must credit "Americans" (the refinery owner), and "Russians"
/// (the harvester's current owner) must see zero delta.
///
/// Verified against `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`
/// §3d: `vtable+0x3C` (GetOwner on the building, address `EBX` in the
/// disassembly) is used as the credits recipient, then
/// `HouseClass__Add_Tiberium_Credits` at `0x004F9610` adds to that house.
#[test]
fn unloading_credits_refinery_owner_under_mind_control() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Refinery owned by Americans.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    // Mind-control: rewrite the harvester's owner to a different house.
    let mc_owner = sim.interner.intern("Russians");
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.owner = mc_owner;
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
    sim.production.dock_reservations.try_reserve(2, miner_id);

    let americans_before = credits_for_owner(&sim, "Americans");
    let russians_before = credits_for_owner(&sim, "Russians");
    tick_miners_n(&mut sim, &rules, 100);
    let americans_after = credits_for_owner(&sim, "Americans");
    let russians_after = credits_for_owner(&sim, "Russians");

    assert_eq!(
        americans_after - americans_before,
        125,
        "refinery owner (Americans) must receive 5 × 25 = 125 credits",
    );
    assert_eq!(
        russians_after - russians_before,
        0,
        "mind-control controller (Russians) must receive zero credits",
    );
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

    // Tick enough for unload (1 bale at 14 ticks), state-4 handoff, and
    // SearchOre/WaitNoOre with margin.
    tick_miners_n(&mut sim, &rules, 150);

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

/// After Departing arrival: `target_ore_cell` is cleared (the pending
/// pre-dock target has been consumed), but `last_harvest_cell` is
/// PRESERVED — gamemd's `+0x218` ghost-cell archive survives the
/// entire dock cycle (Mission_Deploy_Building and UndockUnit leave
/// it untouched), so the next SearchOre can return directly to the
/// nearby productive patch saved when this miner became full.
#[test]
fn exit_pad_preserves_archive_on_arrival() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    // 4×3 refinery at (10, 10). Place miner at queue cell (14, 11) and
    // pre-cache that as the exit cell so the test exercises the arrival
    // contract without depending on the spiral-search result for this
    // specific test grid.
    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 14, 11);

    // Set up the miner mid-Departing with an archive populated (as if a
    // prior State 1 full-path saved a nearby productive patch).
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::Departing;
    miner.reserved_refinery = Some(100);
    miner.dock_queued = false;
    miner.target_ore_cell = Some((20, 20)); // pre-dock target
    miner.last_harvest_cell = Some((20, 20)); // archive from State 1 full-path
    miner.exit_cell = Some((14, 11)); // pre-cache exit to (14, 11) where miner is placed

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
        "target_ore_cell must be cleared (pre-dock target consumed)"
    );
    assert_eq!(
        miner.last_harvest_cell,
        Some((20, 20)),
        "last_harvest_cell (archive) must SURVIVE the dock cycle — \
         gamemd's +0x218 is untouched by the unload state machine",
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
    // Exit cell for the 4×3 refinery at (10, 10) is the queue cell (14, 11)
    // — the cell directly outside the pad, on the same axis the miner
    // entered through.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 14, 11);

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
/// Sets up a chrono miner mid-Departing with `last_harvest_cell` pointing
/// to a depleted cell. The archive survives the dock cycle (gamemd parity)
/// but the next SearchOre's archive-consumption check sees no ore at the
/// archived location, clears the archive, and falls through to the long
/// scan which finds the only patch on the map.
#[test]
fn chrono_miner_archive_cleared_after_undock_picks_new_target() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    // Refinery at (10, 10), 4x3 foundation. Exit cell = queue cell (14, 11).
    spawn_refinery(&mut sim, 100, 10, 10);

    // Place ONE ore patch at (13, 13): within local_continuation_radius
    // (default 6) of exit cell (14, 11). This is what the fresh local scan
    // from current position should pick.
    sim.production.resource_nodes.insert(
        (13, 13),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 1200,
        },
    );

    // Spawn miner at exit cell (14, 11), mid-Departing. Stale archive points
    // far away (50, 50) — outside any scan radius from current position,
    // and no ore at that cell. If the archive were NOT cleared, the search
    // would start from (50, 50), the local scan would find nothing, the
    // archive check would also find nothing, and only the long scan would
    // eventually fall back to current position. With the fix the local scan
    // from current position immediately picks (13, 13).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 14, 11);
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::Departing;
    miner.reserved_refinery = Some(100);
    miner.target_ore_cell = Some((50, 50));
    miner.last_harvest_cell = Some((50, 50));
    miner.cargo.clear();
    miner.exit_cell = Some((14, 11)); // pre-cache exit to match miner spawn pos

    // Tick twice: (1) Departing → SearchOre with cleared archive,
    // (2) SearchOre → MoveToOre with target picked.
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");

    // The stale (50, 50) target must be replaced. Any other value (None or
    // Some((13, 13))) is acceptable — the precise target depends on which
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
            (13, 13),
            "the only ore at (13, 13) should be picked. Got {:?}",
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

/// End-to-end pin for the head-butt-after-unload fix. Exercises stock
/// state-4 handoff -> SearchOre -> A* from a blocked-start cell -> MoveToOre.
/// Uses a real PathGrid with the refinery foundation blocked, so the test
/// would fail without the A* start-relaxation.
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
    // setup that makes the test meaningful: SearchOre must be able to path
    // from the blocked pad start after state-4 handoff.
    let mut path_grid = PathGrid::new(32, 32);
    path_grid.block_building_footprint(10, 10, "4x3", &[], &[], false);

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
    // blocked-footprint path_grid. Use enough ticks for state-4 handoff,
    // SearchOre, A*, and drive south toward ore.
    let alliances = HouseAllianceMap::new();
    let terrain_costs = BTreeMap::new();
    let mut occupancy = OccupancyGrid::new();
    let mut rng = SimRng::new(0);

    // Phase A: tick until the miner exits the Dock state. This is when the
    // stock state-4 handoff clears reserved_refinery. Asserting at that
    // exact tick avoids racing the
    // subsequent harvest cycle (which legitimately re-reserves the
    // refinery once the cell is drained).
    //
    // The handoff should happen immediately; 200 ticks is a comfortable
    // upper bound that also covers subsequent search-ore movement if timing
    // changes.
    let mut departed_at: Option<usize> = None;
    let mut reservation_observed_clear = false;
    for tick in 0..200 {
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

        let miner = sim
            .entities
            .get(miner_id)
            .and_then(|e| e.miner.as_ref())
            .expect("miner alive");
        if miner.state != MinerState::Dock {
            if departed_at.is_none() {
                departed_at = Some(tick);
                // phase_departing's arrival branch clears reserved_refinery
                // before transitioning state; observe it exactly here.
                reservation_observed_clear = miner.reserved_refinery.is_none();
            }
            break;
        }
    }
    assert!(
        departed_at.is_some(),
        "harvester should have transitioned out of Dock within 60 ticks",
    );
    assert!(
        reservation_observed_clear,
        "phase_departing should have cleared reserved_refinery when state left Dock",
    );

    // Phase B: continue ticking. The miner now runs SearchOre → MoveToOre
    // toward the ore patch, proving the foundation-blocked path_grid did
    // not strand it on the pad. After enough ticks it either reaches the
    // ore cell or is in transit toward it.
    for _ in 0..120 {
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

    // Harvester either escaped the foundation south edge, is targeting the
    // ore patch, or has already harvested it and started returning — any
    // of these proves SearchOre + A* succeeded from the (formerly blocked)
    // pad cell.
    let escaped = entity.position.ry > 12 || entity.position.rx < 10 || entity.position.rx > 13;
    let targeting = miner.target_ore_cell == Some((11, 14));
    let returning = matches!(miner.state, MinerState::ReturnToRefinery | MinerState::Dock);
    assert!(
        escaped || targeting || returning,
        "harvester should have escaped foundation, be targeting ore, or be \
         returning after harvest; pos=({},{}) target_ore={:?} state={:?}",
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
            occupancy.add(
                rx,
                ry,
                refinery_id,
                MovementLayer::Ground,
                None,
                CellListInsertion::AppendBuilding,
            );
        }
    }

    let mut path_grid = PathGrid::new(32, 32);
    path_grid.block_building_footprint(10, 10, "4x3", &[], &[], false);

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
// Focused pins for the stock refinery inbound radio FSM.
// ===========================================================================

/// Approach sends HELLO first. Mission_Enter/CAN_DOCK runs on a later tick
/// and only then issues movement to the accepted cell.
#[test]
fn hello_before_mission_enter_then_can_dock_move() {
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
    assert_eq!(m.dock_phase, RefineryDockPhase::MissionEnter);
    assert!(
        sim.production.dock_reservations.has_contact(2, miner_id),
        "HELLO/ROGER should populate Contacts[]"
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id),
        "0x18/+0x418-style contact-entered flag must not be set by HELLO"
    );
    assert!(
        sim.entities
            .get(miner_id)
            .expect("entity")
            .movement_target
            .is_none(),
        "HELLO acceptance must not issue the CAN_DOCK move in the same tick"
    );

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::AwaitingAcceptedCell);
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id),
        "not at accepted cell yet: no 0x18/0x16 admission"
    );

    let entity = sim.entities.get(miner_id).expect("entity");
    let accepted_cell_move_issued = entity
        .movement_target
        .as_ref()
        .and_then(|target| target.path.last().copied())
        == Some((13, 11));
    assert!(
        accepted_cell_move_issued || (entity.position.rx, entity.position.ry) == (13, 11),
        "CAN_DOCK should move toward accepted cell (13,11)"
    );
}

/// Reaching the accepted cell only satisfies the move requested by 0x12. The
/// pivot/link handshake starts on the next Mission_Enter pass, when 0x12
/// returns already-there.
#[test]
fn accepted_cell_arrival_rechecks_can_dock_before_entered_flag() {
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
        miner.dock_phase = RefineryDockPhase::AwaitingAcceptedCell;
        miner.reserved_refinery = Some(2);
    }
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.dock_phase,
        RefineryDockPhase::MissionEnter,
        "arrival at accepted cell must re-enter CAN_DOCK before pivot/link"
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id),
        "accepted-cell movement alone must not set the entered flag"
    );
    assert!(!sim.production.dock_reservations.is_on_pad(2, miner_id));

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::FaceSync);
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id),
        "the next already-there 0x12 pass starts the entered handshake"
    );
}

#[test]
fn waiter_moves_from_queueingcell_to_accepted_cell_before_entered() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let waiter = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(2);
    }

    tick_miners_n(&mut sim, &rules, 1);

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::AwaitingAcceptedCell
    );
    assert!(sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, waiter),
        "QueueingCell position must not count as entered"
    );
    let entity = sim.entities.get(waiter).expect("waiter entity");
    let accepted_cell_move_issued = entity
        .movement_target
        .as_ref()
        .and_then(|target| target.path.last().copied())
        == Some((13, 11));
    assert!(
        accepted_cell_move_issued || (entity.position.rx, entity.position.ry) == (13, 11),
        "CAN_DOCK should move from QueueingCell (14,11) to accepted cell (13,11)"
    );

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        entity.position.rx = 13;
        entity.position.ry = 11;
        entity.position.refresh_screen_coords();
        entity.movement_target = None;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "accepted-cell move completion must re-enter CAN_DOCK before linking"
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );

    tick_miners_n(&mut sim, &rules, 16);

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(waiter_miner.dock_phase, RefineryDockPhase::FaceSync);
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
}

#[test]
fn occupied_can_dock_defers_without_clearing_waiting_miner_target() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let occupant = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(occupant).expect("occupant entity");
        let miner = entity.miner.as_mut().expect("occupant miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.unload_timer = 1000;
    }
    assert!(sim.production.dock_reservations.try_reserve(2, occupant));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, occupant);
    sim.production.dock_reservations.link_on_pad(2, occupant);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(2);
    }

    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, waiter);
    assert_eq!(miner.state, MinerState::Dock);
    assert_eq!(
        miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "busy CAN_DOCK should defer in MissionEnter, not clear the target or enter"
    );
    assert_eq!(miner.reserved_refinery, Some(2));
    assert!(miner.dock_queued);
    assert!(!sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
}

#[test]
fn queued_miner_enters_after_contact_and_pad_are_released() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let occupant = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    assert!(sim.production.dock_reservations.try_reserve(2, occupant));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, occupant);
    sim.production.dock_reservations.link_on_pad(2, occupant);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(2);
    }

    tick_miners_n(&mut sim, &rules, 1);
    assert_eq!(
        get_miner(&sim, waiter).dock_phase,
        RefineryDockPhase::MissionEnter,
        "precondition: occupied pad defers the queued miner"
    );

    sim.production.dock_reservations.release_on_pad(2, occupant);
    sim.production
        .dock_reservations
        .release_contact(2, occupant);

    tick_miners_n(&mut sim, &rules, 16);

    let miner = get_miner(&sim, waiter);
    assert_eq!(miner.dock_phase, RefineryDockPhase::FaceSync);
    assert!(!miner.dock_queued);
    assert!(sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
}

#[test]
fn two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let occupant = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(occupant).expect("occupant entity");
        let miner = entity.miner.as_mut().expect("occupant miner");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(2);
    }
    assert!(sim.production.dock_reservations.try_reserve(2, occupant));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, occupant);
    sim.production.dock_reservations.link_on_pad(2, occupant);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(2);
        miner.dock_queued = true;
    }
    assert_eq!(
        sim.production.dock_reservations.hello_or_wait(2, waiter, 1),
        crate::sim::miner::miner_dock::ContactAdmission::Waiting
    );

    tick_miners_n(&mut sim, &rules, 1);

    let occupant_miner = get_miner(&sim, occupant);
    assert_eq!(occupant_miner.state, MinerState::SearchOre);
    assert!(!sim.production.dock_reservations.has_contact(2, occupant));
    assert!(!sim.production.dock_reservations.is_on_pad(2, occupant));

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::FaceSync,
        "mission-dispatch-eligible waiter should claim only during its own MissionEnter pass"
    );
    assert!(!waiter_miner.dock_queued);
    assert!(sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
    assert!(
        !sim.production.dock_reservations.is_on_pad(2, waiter),
        "already-there CAN_DOCK sets entered/contact state before pad-arrival handoff"
    );

    let occupant_entity = sim.entities.get(occupant).expect("occupant entity");
    assert!(occupant_entity.forced_drive_track.is_none());
    assert!(occupant_entity.movement_target.is_none());
}

#[test]
fn two_miners_waiter_after_releaser_approach_hello_only() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let occupant = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::War, 14, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(occupant).expect("occupant entity");
        let miner = entity.miner.as_mut().expect("occupant miner");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(2);
    }
    assert!(sim.production.dock_reservations.try_reserve(2, occupant));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, occupant);
    sim.production.dock_reservations.link_on_pad(2, occupant);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.reserved_refinery = Some(2);
        miner.dock_queued = true;
    }
    assert_eq!(
        sim.production.dock_reservations.hello_or_wait(2, waiter, 1),
        crate::sim::miner::miner_dock::ContactAdmission::Waiting
    );

    tick_miners_n(&mut sim, &rules, 16);

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "Approach must perform only HELLO, even after an earlier same-tick release"
    );
    assert!(!waiter_miner.dock_queued);
    assert!(sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
    assert!(!sim.production.dock_reservations.is_on_pad(2, waiter));
    assert!(
        sim.entities
            .get(waiter)
            .expect("waiter entity")
            .movement_target
            .is_none(),
        "HELLO acceptance must not collapse into CAN_DOCK movement"
    );

    tick_miners_n(&mut sim, &rules, 1);

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(waiter_miner.dock_phase, RefineryDockPhase::FaceSync);
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
}

#[test]
fn two_miners_waiter_before_releaser_not_retroactively_promoted() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let waiter = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    let occupant = spawn_miner(&mut sim, 3, MinerKind::War, 13, 11);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(2);
        miner.dock_queued = true;
    }

    {
        let entity = sim.entities.get_mut(occupant).expect("occupant entity");
        let miner = entity.miner.as_mut().expect("occupant miner");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(2);
    }
    assert!(sim.production.dock_reservations.try_reserve(2, occupant));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, occupant);
    sim.production.dock_reservations.link_on_pad(2, occupant);
    assert_eq!(
        sim.production.dock_reservations.hello_or_wait(2, waiter, 1),
        crate::sim::miner::miner_dock::ContactAdmission::Waiting
    );

    tick_miners_n(&mut sim, &rules, 1);

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "waiter already processed before release; no retroactive promotion"
    );
    assert!(waiter_miner.dock_queued);
    assert!(sim.production.dock_reservations.is_waiting(2, waiter));
    assert!(!sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
    assert!(!sim.production.dock_reservations.is_on_pad(2, waiter));
    assert_eq!(get_miner(&sim, occupant).state, MinerState::SearchOre);

    tick_miners_n(&mut sim, &rules, 16);

    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::FaceSync,
        "waiter enters only on its next own MissionEnter pass"
    );
    assert!(!waiter_miner.dock_queued);
    assert!(sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
}

/// Once CAN_DOCK's accepted-cell move is already satisfied, the stock path
/// sets the 0x18/+0x418-style entered flag and runs ordinary 0x16 facing
/// sync. It does not turn that first handshake into radio 0x15 or unload
/// startup side effects.
#[test]
fn accepted_cell_arrival_sets_contact_entered_then_0x15_starts_unload_fsm() {
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
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(2);
    }
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::FaceSync);
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, miner_id),
        "already-there 0x12 reply should set the +0x418-like entered flag"
    );
    assert!(
        !sim.production.dock_reservations.is_on_pad(2, miner_id),
        "stock path must not depend on an early +0x2E4-style on-pad link"
    );

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.dock_phase,
        RefineryDockPhase::FaceSync,
        "the first ordinary 0x16 only syncs facing; it must not queue deploy"
    );
    assert!(
        !sim.production.dock_reservations.is_on_pad(2, miner_id),
        "radio 0x15 has not run, so unload-active pad bookkeeping must remain clear"
    );
}

/// Unloading emits one BaleDepositEvent per StorageClass slot drained
/// (matches gamemd: SpecialAnim fires per slot, not per bale).
#[test]
fn unloading_emits_one_event_per_slot_drain() {
    // --- 5 ore bales = 1 slot → 1 event ---
    let mut sim = Simulation::new();
    let rules = miner_rules();

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

    tick_miners_n(&mut sim, &rules, 200);

    assert_eq!(
        sim.bale_events.len(),
        1,
        "pure-ore cargo must drain in one slot dump = one BaleDepositEvent",
    );
    assert_eq!(sim.bale_events[0].building_id, 2);

    // --- 5 ore + 3 gems = 2 slots → 2 events ---
    let mut sim = Simulation::new();
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
        for _ in 0..3 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Gem,
                value: 50,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.unload_timer = 0;
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    tick_miners_n(&mut sim, &rules, 200);

    assert_eq!(
        sim.bale_events.len(),
        2,
        "ore + gem cargo must produce two BaleDepositEvents (one per slot)",
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

/// Purifier bonus is applied per slot drain on the full slot value
/// (matches gamemd's `bonus = slot_value × purifier_count × PurifierBonus`).
/// With one ore slot of value 100 and a 25% PurifierBonus, total credits
/// gain = 100 + (100 × 1 × 25 / 100) = 125.
#[test]
fn unloading_applies_per_slot_purifier_bonus() {
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

/// Conditional reciprocal-link release geometry is anchored at the queue cell.
/// Stock zero-link unload completion does not use it. For a 4x3 GAREFN at
/// (10, 10), the queue cell (14, 11) sits east of the foundation and is the
/// release target on a normal map where only the foundation itself is blocked.
/// The spiral only expands to ring 1+ when the queue cell is also blocked.
/// `tick % count` picks one of the ring's candidates each cycle.
#[test]
fn conditional_release_anchors_at_queue_cell() {
    use super::miner_dock_sequence::refinery_exit_cell;

    // Build a grid with the 4×3 foundation at (10, 10) blocked, simulating
    // real building occupancy. Queue cell (14, 11) is outside the
    // foundation and remains passable.
    let mut grid_garefn = PathGrid::test_all_passable(64, 64);
    for fx in 10..14 {
        for fy in 10..13 {
            grid_garefn.set_blocked(fx, fy, true);
        }
    }

    // Only foundation blocked: queue (14, 11) is itself walkable, so
    // ring 0 returns it deterministically for every tick.
    for tick in 0..6 {
        assert_eq!(
            refinery_exit_cell(10, 10, 4, 3, None, Some(&grid_garefn), None, tick),
            (14, 11),
            "exit must land at queue cell when it is passable (tick {tick})"
        );
    }

    // Now also block the queue cell, simulating another miner queued
    // there. Ring 1 around (14, 11) yields candidates in iteration order
    // (top + bottom rows per delta = -1..=1, then left + right columns):
    //   (13,10) FND, (13,12) FND, (14,10), (14,12), (15,10), (15,12),
    //   (13,11) FND, (15,11)
    // FND = blocked by foundation. Passable candidates, in order:
    //   (14, 10), (14, 12), (15, 10), (15, 12), (15, 11)
    let mut grid_blocked_queue = grid_garefn.clone();
    grid_blocked_queue.set_blocked(14, 11, true);

    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&grid_blocked_queue), None, 0),
        (14, 10)
    );
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&grid_blocked_queue), None, 1),
        (14, 12)
    );
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&grid_blocked_queue), None, 2),
        (15, 10)
    );
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&grid_blocked_queue), None, 3),
        (15, 12)
    );
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&grid_blocked_queue), None, 4),
        (15, 11)
    );
    // tick=5 → wraps (5 % 5 = 0) → (14, 10).
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&grid_blocked_queue), None, 5),
        (14, 10)
    );

    // Clean grid (no foundation blocking) — anchor (queue cell) is
    // itself walkable, so ring 0 returns it directly.
    let clean_grid = PathGrid::test_all_passable(64, 64);
    // 4×3 at (10, 10): queue (14, 11).
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, None, Some(&clean_grid), None, 0),
        (14, 11)
    );
    // 3×3 at (5, 5): queue (8, 6).
    assert_eq!(
        refinery_exit_cell(5, 5, 3, 3, None, Some(&clean_grid), None, 0),
        (8, 6)
    );
    // 2×2 at (12, 8): queue (14, 9).
    assert_eq!(
        refinery_exit_cell(12, 8, 2, 2, None, Some(&clean_grid), None, 0),
        (14, 9)
    );

    // Anchor + every cell within the max radius blocked → fallback to
    // QueueingCell. Block a 33×33 region covering radius 16.
    let mut fully_blocked = PathGrid::test_all_passable(64, 64);
    for x in 0..32 {
        for y in 0..32 {
            fully_blocked.set_blocked(x, y, true);
        }
    }
    assert_eq!(
        refinery_exit_cell(10, 10, 4, 3, Some((3, 2)), Some(&fully_blocked), None, 0),
        (13, 12),
        "exhausted spiral must fall back to art.ini QueueingCell"
    );
}

/// Stock zero-link Departing is a state-4 cleanup/handoff, not a cached
/// queue-cell exit drive.
#[test]
fn stock_departing_hands_directly_to_search_without_exit_move() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(100);
        assert!(
            miner.exit_cell.is_none(),
            "stock path must start without a cached release destination"
        );
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);
    sim.production.dock_reservations.link_on_pad(100, miner_id);

    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("entity");
    let m = entity.miner.as_ref().expect("miner");
    assert_eq!(m.state, MinerState::SearchOre);
    assert_eq!((entity.position.rx, entity.position.ry), (13, 11));
    assert!(entity.movement_target.is_none());
    assert!(entity.forced_drive_track.is_none());
    assert!(m.exit_cell.is_none());
    assert!(m.reserved_refinery.is_none());
    assert!(!sim.production.dock_reservations.is_occupied(100));
}

#[test]
fn stock_departing_does_not_start_force_track_0x47() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(100);
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);

    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(miner.state, MinerState::SearchOre);
    assert!(miner.exit_cell.is_none());
    assert!(entity.movement_target.is_none());
    assert!(
        entity.forced_drive_track.is_none(),
        "stock Departing must not seed Force_Track(0x47)"
    );
    assert_ne!(entity.facing, 0x47);
    assert_ne!(entity.facing_target, Some(0x47));
}

#[test]
fn stock_departing_does_not_start_explicit_exit_move() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(100);
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);

    tick_miners_n(&mut sim, &rules, 1);
    let after_first = sim.entities.get(miner_id).expect("miner entity");
    assert_eq!((after_first.position.rx, after_first.position.ry), (13, 11));
    assert!(after_first.forced_drive_track.is_none());
    assert!(after_first.movement_target.is_none());
    assert!(matches!(
        after_first.miner.as_ref().expect("miner").state,
        MinerState::SearchOre
    ));
    assert!(
        after_first
            .miner
            .as_ref()
            .expect("miner")
            .exit_cell
            .is_none()
    );
}

#[test]
fn sell_refinery_interrupts_docked_miner_with_force_track_0x47() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.display_type_override = Some(sim.interner.intern("CMON"));
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..miner.capacity_bales {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(100);
        miner.dock_queued = true;
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);
    sim.production.dock_reservations.link_on_pad(100, miner_id);

    assert!(crate::sim::production::sell_building(&mut sim, &rules, 100));

    assert!(sim.entities.get(100).is_none(), "refinery sold");
    assert!(
        !sim.production.dock_reservations.is_occupied(100),
        "sell interrupt must clear dock links"
    );
    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(miner.state, MinerState::ReturnToRefinery);
    assert_eq!(miner.dock_phase, RefineryDockPhase::Approach);
    assert_eq!(miner.reserved_refinery, None);
    assert_eq!(miner.exit_cell, None);
    assert_eq!(entity.display_type_override, None);
    assert!(entity.movement_target.is_none());
    let forced = entity
        .forced_drive_track
        .as_ref()
        .expect("sell interrupt must seed forced undock track");
    assert_eq!(forced.turn_track_index, 0x47);
    assert_eq!(forced.track.raw_track_index, 15);
}

#[test]
fn sell_refinery_cancels_contact_miner_without_force_track_0x47() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 14, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..miner.capacity_bales {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(100);
        miner.dock_queued = true;
    }
    assert!(sim.production.dock_reservations.try_reserve(100, miner_id));
    assert!(sim.production.dock_reservations.has_contact(100, miner_id));
    assert!(!sim.production.dock_reservations.is_on_pad(100, miner_id));

    assert!(crate::sim::production::sell_building(&mut sim, &rules, 100));

    assert!(sim.entities.get(100).is_none(), "refinery sold");
    assert!(
        !sim.production.dock_reservations.has_contact(100, miner_id),
        "sell interrupt must clear plain refinery contacts"
    );
    assert!(!sim.production.dock_reservations.is_on_pad(100, miner_id));
    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(miner.state, MinerState::ReturnToRefinery);
    assert_eq!(miner.dock_phase, RefineryDockPhase::Approach);
    assert_eq!(miner.reserved_refinery, None);
    assert_eq!(miner.exit_cell, None);
    assert!(entity.movement_target.is_none());
    assert!(
        entity.forced_drive_track.is_none(),
        "plain HELLO/contact miners are not physically docked and must not receive Force_Track(0x47)"
    );
}

/// Stock state-4 handoff must not depend on driving through the queue cell.
/// A waiting miner parked there cannot block cleanup because no explicit
/// exit movement is issued.
#[test]
fn departing_handoff_ignores_blocked_queue_cell() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    spawn_refinery(&mut sim, 100, 10, 10);
    // Miner A on the pad cell, ready to depart.
    let miner_a = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_a).expect("miner A");
        let miner = entity.miner.as_mut().expect("miner A component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(100);
    }
    sim.production.dock_reservations.try_reserve(100, miner_a);

    // Miner B parked at the QueueingCell (14, 11) — blocks miner A's only
    // adjacent walkable exit from the pad.
    let miner_b = spawn_miner(&mut sim, 2, MinerKind::War, 14, 11);
    {
        let entity = sim.entities.get_mut(miner_b).expect("miner B");
        let miner = entity.miner.as_mut().expect("miner B component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Approach;
        miner.dock_queued = true;
    }
    // Register B's occupancy at the queue cell so the deferred check sees it.
    sim.occupancy.add(
        14,
        11,
        miner_b,
        crate::sim::movement::locomotor::MovementLayer::Ground,
        None,
        crate::sim::occupancy::CellListInsertion::PrependNonBuilding,
    );

    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_a).expect("miner A entity");
    let m = entity.miner.as_ref().expect("miner A");
    assert_eq!(m.state, MinerState::SearchOre);
    assert!(
        m.reserved_refinery.is_none(),
        "miner A's dock reservation must be released during state-4 handoff",
    );
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (13, 11),
        "state-4 handoff must not issue a stock queue-cell exit move",
    );
}

/// Departing releases the dock reservation, clears any stale exit-cell cache,
/// and transitions back to SearchOre without pinning facing to 0x47.
#[test]
fn departing_handoff_releases_dock_and_returns_to_search() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(100);
        miner.exit_cell = Some((14, 11));
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);
    sim.production.dock_reservations.link_on_pad(100, miner_id);

    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.state, MinerState::SearchOre);
    assert!(m.reserved_refinery.is_none(), "dock reservation released");
    assert!(m.exit_cell.is_none(), "stale exit-cell cache cleared");
    assert!(
        !sim.production.dock_reservations.is_occupied(100),
        "dock slot freed for next miner",
    );
}

/// Linked sets the UnloadingClass display override, emits a DockDeploy
/// sound on pad arrival, kicks off the pivot to facing East (0x40), and
/// transitions to Pivoting. The pivot runs in phase_pivoting; once facing
/// converges the FSM advances to Unloading and seeds `unload_timer`.
/// Mirrors gamemd's radio 0x16 (FACE_AND_SYNC) RateTimer pivot which fires
/// before the dump cascade (radio 0x15 → SetMission(Mission_Unload)).
#[test]
fn linked_to_pivoting_then_unloading_on_pad_arrival() {
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
    // Place miner at the pad cell already facing 0x40 (East). This isolates
    // the Linked → Pivoting → Unloading transition from the per-tick
    // rotation step, so the test pins exactly the two-phase handshake
    // without depending on the precise rot_to_facing_delta value.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.movement_target = None;
        entity.display_type_override = None;
        entity.facing = 0x40;
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionQueued;
        miner.reserved_refinery = Some(2);
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    // Tick 1: radio 0x15 has only queued mission 0x10, so this advances to
    // the deploy mission without unload presentation side effects.
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    {
        let m = get_miner(&sim, miner_id);
        assert_eq!(m.dock_phase, RefineryDockPhase::Pivoting);
        assert_eq!(
            m.unload_timer, 0,
            "unload_timer must not be seeded until the pivot completes",
        );

        let entity = sim.entities.get(miner_id).expect("entity");
        assert_eq!(entity.facing_target, None);
        assert_eq!(entity.display_type_override, None);
        assert!(
            sim.sound_events
                .iter()
                .all(|e| !matches!(e, SimSoundEvent::DockDeploy { building_id: 2 })),
            "0x15 must not emit DockDeploy before mission 0x10 starts unload"
        );
    }

    // Tick 2: phase_pivoting sees facing already at the target — the
    // "close enough" branch fires immediately, snaps facing, seeds
    // unload_timer, and transitions to Unloading.
    tick_miners_n(&mut sim, &rules, 1);

    {
        let m = get_miner(&sim, miner_id);
        assert_eq!(
            m.dock_phase,
            RefineryDockPhase::Unloading,
            "Pivoting must transition to Unloading once facing reaches 0x40",
        );
        assert_eq!(m.unload_timer, 0, "Plan C does not preload unload_timer");
        assert!(m.unload_active, "unload-active latch should be set");
        assert_eq!(m.unload_accumulator, 0);
        assert_eq!(m.unload_cluster_start_frame, Some(sim.binary_frame));
        assert_eq!(m.unload_cluster_duration, 1);
        assert_eq!(m.unload_cluster_repeat, 1);
        assert_eq!(m.unload_accumulator_step, 1);
        assert!(
            (14..=16).contains(&m.mission_deploy_duration),
            "accepted unload-start should schedule stock 14..16 frames, got {}",
            m.mission_deploy_duration
        );

        let entity = sim.entities.get(miner_id).expect("entity");
        assert_eq!(
            entity.facing, 0x40,
            "pre-aligned facing should remain unchanged; unload-start must not snap it",
        );
        assert!(
            entity.facing_target.is_none(),
            "facing_target must be cleared once the pivot completes",
        );
        let override_id = entity
            .display_type_override
            .expect("UnloadingClass override should be set when unload starts");
        assert_eq!(sim.interner.resolve(override_id), "HORV");
        let dock_deploy_count = sim
            .sound_events
            .iter()
            .filter(|e| matches!(e, SimSoundEvent::DockDeploy { building_id: 2 }))
            .count();
        assert_eq!(
            dock_deploy_count, 0,
            "stock unload-start emits no DockDeploy"
        );
        assert!(
            !sim.production.dock_reservations.is_on_pad(2, miner_id),
            "stock zero-link unload must not set physical on_pad"
        );
    }
}

/// Pivoting phase advances facing toward 0x40 (East) one rotation step at
/// a time and only transitions to Unloading once facing reaches the target.
/// Verifies the smooth-rotation path (not the pre-aligned shortcut tested
/// in `linked_to_pivoting_then_unloading_on_pad_arrival`).
#[test]
fn pivoting_phase_smoothly_rotates_to_east() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 2, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.movement_target = None;
        entity.facing = 0; // North — must rotate 64 facing units clockwise to reach 0x40.
        entity.facing_target = Some(0x40);
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Pivoting;
        miner.reserved_refinery = Some(2);
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    let initial_facing = sim.entities.get(miner_id).expect("entity").facing;
    let rng_before = sim.scenario_rng.state();
    assert_eq!(initial_facing, 0);

    // The first direct tick initializes the FacingClass timer and samples its
    // current 16-bit facing; this is timer-derived motion, not manual 8-bit
    // facing stepping.
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));
    {
        let entity = sim.entities.get(miner_id).expect("entity");
        let m = entity.miner.as_ref().expect("miner");
        assert_eq!(
            entity.facing, initial_facing,
            "dock facing timer must not write visible body facing"
        );
        assert_eq!(m.dock_phase, RefineryDockPhase::Pivoting);
        assert_eq!(m.unload_timer, 0, "timer must not seed mid-pivot");
        assert_eq!(entity.facing_target, Some(0x40));
        assert!(m.dock_pivot_facing.is_some());
        assert_eq!(m.mission_deploy_duration, 5);
        assert_eq!(m.mission_deploy_start_frame, Some(sim.binary_frame));
        assert_eq!(
            sim.scenario_rng.state(),
            rng_before,
            "facing wait consumes no RNG"
        );
    }

    tick_miners_n(&mut sim, &rules, 1);
    {
        let entity = sim.entities.get(miner_id).expect("entity");
        let m = entity.miner.as_ref().expect("miner");
        assert_eq!(
            entity.facing, initial_facing,
            "passive mission delay must not advance visible facing"
        );
        assert_eq!(m.dock_phase, RefineryDockPhase::Pivoting);
        assert_eq!(m.unload_timer, 0, "timer must not seed mid-pivot");
        assert_eq!(entity.facing_target, Some(0x40));
    }

    // Tick until the pivot resolves. Cap is generous; stock harvester ROT=
    // remains the parsed INI value, so low-ROT cases can take up to 64 ticks.
    let mut ticks_until_done = 0;
    for _ in 0..128 {
        tick_miners_n(&mut sim, &rules, 1);
        ticks_until_done += 1;
        if get_miner(&sim, miner_id).dock_phase == RefineryDockPhase::Unloading {
            break;
        }
    }

    let entity = sim.entities.get(miner_id).expect("entity");
    let m = entity.miner.as_ref().expect("miner");
    assert_eq!(
        m.dock_phase,
        RefineryDockPhase::Unloading,
        "pivot must reach Unloading within 128 ticks (took {})",
        ticks_until_done,
    );
    assert_eq!(
        entity.facing, initial_facing,
        "dock mission must not force the visible body facing to East"
    );
    assert!(entity.facing_target.is_none());
    assert_eq!(m.unload_timer, 0);
    assert!(m.unload_active);
}

/// End-to-end dock cycle: war miner forced-returns to a refinery, drives onto
/// the pad, deposits N bales, and completes stock state-4 handoff. Verifies
/// bale event count, total credits, final position, and dock release.
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

    // Tick enough for: HELLO, MissionEnter/CAN_DOCK, accepted-cell/pad handoff,
    // 10 bales x ~14 ticks unload, then stock state-4 handoff.
    tick_miners_n(&mut sim, &rules, 400);

    // Bale events: one per slot drain. Pre-loaded with pure ore → 1 slot → 1 event.
    assert_eq!(
        sim.bale_events.len(),
        1,
        "expected 1 bale event (one slot drain), got {}",
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

    let entity = sim.entities.get(miner_id).expect("entity");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (13, 11),
        "stock state-4 handoff should not force a queue-cell exit move"
    );
    assert!(entity.forced_drive_track.is_none());

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

// ==========================================================================
// extract_bales_max — bulk-drain primitive (gamemd Harvest_Ore_Tick parity)
// ==========================================================================

#[test]
fn extract_max_empty_cell() {
    let mut sim = Simulation::new();
    let config = MinerConfig::default();
    let bales = super::miner_system::extract_bales_max(&mut sim, (5, 5), &config, 40);
    assert!(bales.is_empty(), "no node at cell → no bales");
}

#[test]
fn extract_max_full_drain_ore() {
    let mut sim = Simulation::new();
    let config = MinerConfig::default();
    // 11 density levels of ore at base 120: remaining = 11 * 120 = 1320.
    sim.production.resource_nodes.insert(
        (5, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 11 * 120,
        },
    );
    let bales = super::miner_system::extract_bales_max(&mut sim, (5, 5), &config, 40);
    assert_eq!(bales.len(), 11, "full drain extracts 11 bales");
    assert!(
        bales
            .iter()
            .all(|b| b.resource_type == ResourceType::Ore && b.value == config.ore_bale_value),
        "all bales are ore-type with configured value"
    );
    assert!(
        sim.production.resource_nodes.get(&(5, 5)).is_none(),
        "node removed after full drain"
    );
}

#[test]
fn extract_max_partial_capacity() {
    let mut sim = Simulation::new();
    let config = MinerConfig::default();
    sim.production.resource_nodes.insert(
        (5, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 11 * 120,
        },
    );
    let bales = super::miner_system::extract_bales_max(&mut sim, (5, 5), &config, 3);
    assert_eq!(bales.len(), 3, "capacity-limited to 3 bales");
    let after = sim
        .production
        .resource_nodes
        .get(&(5, 5))
        .expect("still present");
    assert_eq!(
        after.remaining,
        (11 - 3) * 120,
        "remaining decremented by 3 density levels"
    );
}

#[test]
fn extract_max_partial_density_exact_match() {
    let mut sim = Simulation::new();
    let config = MinerConfig::default();
    // 5 density levels of ore: remaining = 600. Empty capacity higher than
    // available density → drain exactly 5, node removed.
    sim.production.resource_nodes.insert(
        (5, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 5 * 120,
        },
    );
    let bales = super::miner_system::extract_bales_max(&mut sim, (5, 5), &config, 40);
    assert_eq!(bales.len(), 5, "extracts all 5 available density levels");
    assert!(
        sim.production.resource_nodes.get(&(5, 5)).is_none(),
        "exact match drains the cell"
    );
}

#[test]
fn extract_max_gem_cell() {
    let mut sim = Simulation::new();
    let config = MinerConfig::default();
    // 4 density levels of gems at base 180.
    sim.production.resource_nodes.insert(
        (5, 5),
        ResourceNode {
            resource_type: ResourceType::Gem,
            remaining: 4 * 180,
        },
    );
    let bales = super::miner_system::extract_bales_max(&mut sim, (5, 5), &config, 40);
    assert_eq!(bales.len(), 4, "gem cell yields 4 bales");
    assert!(
        bales
            .iter()
            .all(|b| b.resource_type == ResourceType::Gem && b.value == config.gem_bale_value),
        "all bales are gem-type with configured value"
    );
}

#[test]
fn extract_max_zero_capacity() {
    let mut sim = Simulation::new();
    let config = MinerConfig::default();
    sim.production.resource_nodes.insert(
        (5, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 11 * 120,
        },
    );
    let bales = super::miner_system::extract_bales_max(&mut sim, (5, 5), &config, 0);
    assert!(bales.is_empty(), "zero capacity → no bales");
    let after = sim
        .production
        .resource_nodes
        .get(&(5, 5))
        .expect("untouched");
    assert_eq!(after.remaining, 11 * 120, "node remaining untouched");
}

#[test]
fn extract_max_node_remaining_zero() {
    let mut sim = Simulation::new();
    let config = MinerConfig::default();
    // Edge case: node present but remaining == 0 (matches gamemd's
    // Reduce_Tiberium returning 0 for an empty cell).
    sim.production.resource_nodes.insert(
        (5, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 0,
        },
    );
    let bales = super::miner_system::extract_bales_max(&mut sim, (5, 5), &config, 40);
    assert!(bales.is_empty(), "remaining==0 → no bales");
}

// ==========================================================================
// Multi-bale extraction integration tests (parity contract for handle_harvest)
// ==========================================================================

/// Drives the full handle_harvest path: a War Miner sitting on an 11-density
/// ore cell drains the entire cell in a single extraction call, matching
/// gamemd's Harvest_Ore_Tick.
#[test]
fn harvester_drains_full_cell_in_one_extraction_tick() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    place_ore(&mut sim, 20, 20, 11 * 120);

    // War Miner (capacity 40) on the ore cell, already in Harvest state and
    // ready to fire (harvest_timer == 0).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    // Single tick: timer == 0 means extract_bales_max fires immediately and
    // drains the cell in one call.
    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(
        miner.cargo.len(),
        11,
        "full cell drained in one extraction call"
    );
    assert!(
        sim.production.resource_nodes.get(&(20, 20)).is_none(),
        "cell removed after full drain"
    );
}

/// One extraction call must not exceed remaining cargo capacity even when
/// the cell has more density than the miner can hold.
#[test]
fn harvester_caps_extraction_at_remaining_capacity() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    place_ore(&mut sim, 20, 20, 11 * 120);

    // War Miner with 38 of 40 bales already loaded — only 2 free slots.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..38 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: config.ore_bale_value,
            });
        }
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.cargo.len(), 40, "capped at capacity");

    // 2 bales extracted from an 11-density cell → 9 levels remain.
    let after = sim
        .production
        .resource_nodes
        .get(&(20, 20))
        .expect("cell still has ore");
    assert_eq!(after.remaining, 9 * 120, "cell drops to density 9");
}

/// After a partial-density cell is fully drained but the miner still has
/// capacity, the next harvest cycle's empty-cell branch should kick a
/// TiberiumShortScan continuation that picks up the neighbouring patch.
#[test]
fn harvester_continues_to_short_scan_when_partial_then_empty() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    // Density-5 cell at (20, 20). Another density-5 cell at (21, 20),
    // safely within the local continuation radius (6 cells).
    place_ore(&mut sim, 20, 20, 5 * 120);
    place_ore(&mut sim, 21, 20, 5 * 120);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    // First tick: drain (20, 20) in one extraction → 5 bales. The post-
    // success branch resets harvest_timer to harvest_tick_interval and the
    // miner stays in Harvest waiting for the next cycle.
    tick_miners_n(&mut sim, &rules, 1);
    {
        let miner = get_miner(&sim, miner_id);
        assert_eq!(miner.cargo.len(), 5, "5 bales from density-5 cell");
        assert_eq!(
            miner.state,
            MinerState::Harvest,
            "stays in Harvest, timer reset"
        );
        assert!(
            sim.production.resource_nodes.get(&(20, 20)).is_none(),
            "cell drained"
        );
    }

    // Tick out the harvest_tick_interval wait; the next extraction attempt
    // hits an empty cell and the short-scan picks up (21, 20).
    tick_miners_n(&mut sim, &rules, config.harvest_tick_interval as usize + 1);
    {
        let miner = get_miner(&sim, miner_id);
        assert_eq!(
            miner.state,
            MinerState::MoveToOre,
            "transitions to MoveToOre after empty-cell short scan"
        );
        assert_eq!(miner.target_ore_cell, Some((21, 20)));
    }
}

/// gamemd parity: the first dock bale must wait
/// `ceil(HarvesterDumpRate × 900) = 15` frames after the Linked →
/// Unloading transition, not fire immediately. The dump counter starts
/// at 0 on dock-link and a bale deposits only once the counter reaches
/// 14.4. With our tenths-of-a-tick precision (timer decrements by 10
/// per tick before the drain check) the first slot drain fires 15
/// unloading ticks after Linked, dumping ALL bales of the first
/// non-empty resource type at once (matches gamemd's per-slot dump).
#[test]
fn dock_first_slot_drain_waits_one_unload_interval() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    spawn_refinery(&mut sim, 2, 10, 10);
    // Place miner at the pad cell facing 0x40 (East) so the dock pivot
    // (Linked → Pivoting → Unloading) completes in two ticks and the
    // 14.4-frame dump gate timing this test pins lines up cleanly.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.movement_target = None;
        entity.facing = 0x40;
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..5 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: config.ore_bale_value,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionQueued;
        miner.reserved_refinery = Some(2);
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    // Tick 1: phase_linked transitions to Pivoting. No drain yet.
    // Tick 2: phase_pivoting sees facing already at 0x40, transitions to
    // Unloading and seeds the unload_timer. No drain yet.
    tick_miners_n(&mut sim, &rules, 2);

    let initial_cargo = get_miner(&sim, miner_id).cargo.len();
    assert_eq!(initial_cargo, 5, "no drain should fire before Unloading");
    assert_eq!(
        get_miner(&sim, miner_id).dock_phase,
        RefineryDockPhase::Unloading,
        "pivot should complete in one tick when facing is pre-aligned",
    );

    // Ticks 3..16 (14 unloading ticks): timer decrements past zero, no drain
    // yet (decrement-then-check returns before drain on the tick the
    // timer crosses ≤ 0).
    let mut drain_tick = None;
    for elapsed in 1..=20 {
        tick_miners_n(&mut sim, &rules, 1);
        if get_miner(&sim, miner_id).cargo.is_empty() {
            drain_tick = Some(elapsed);
            break;
        }
        assert_eq!(
            get_miner(&sim, miner_id).cargo.len(),
            initial_cargo,
            "no partial drain should fire before the slot dump gate"
        );
    }

    let drain_tick = drain_tick.expect("slot should drain within Plan C timing window");
    assert!(
        (15..=16).contains(&drain_tick),
        "Plan C first slot drain should be gated by accepted mission delay plus accumulator threshold, got tick {}",
        drain_tick
    );
    assert_eq!(get_miner(&sim, miner_id).cargo.len(), 0);
}

/// Verify the empty-slot gate + stock state-4 dock release:
/// 1. Cargo is already empty when the dump gate fires.
/// 2. The same tick advances to Departing, with the dock still occupied.
/// 3. The next tick runs the stock state-4 handoff and releases the dock.
///
/// Sets the miner up in Unloading with empty cargo and `unload_timer = 0`
/// so the cargo-empty branch fires on the very first tick.
#[test]
fn empty_unload_gate_releases_dock_on_next_stock_state4_handoff() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    let unloading_type = sim.interner.intern("HORV");

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.display_type_override = Some(unloading_type);
        let miner = entity.miner.as_mut().expect("miner component");
        // Empty cargo + zero timer → first tick hits the cargo-empty branch.
        miner.cargo.clear();
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.unload_timer = 0;
    }
    // Mark the dock occupied so we can assert release timing directly.
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));
    assert!(sim.production.dock_reservations.is_occupied(2));

    // First tick: phase_unloading sees empty cargo and advances to the
    // state-4 handoff without seeding another dump-gate cooldown.
    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.dock_phase,
        RefineryDockPhase::Departing,
        "empty-slot gate should transition directly to Departing",
    );
    assert_eq!(
        m.deposit_cooldown_ticks, 0,
        "empty-slot gate must not seed another unload interval",
    );
    assert!(
        sim.production.dock_reservations.is_occupied(2),
        "dock is still occupied until the Departing handler runs",
    );

    // Next tick runs the stock state-4 handoff.
    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert!(
        m.state == MinerState::SearchOre,
        "miner should have returned to search at state-4 handoff, got {:?}",
        m.state,
    );
    assert!(
        !sim.production.dock_reservations.is_occupied(2),
        "dock must be released by the stock state-4 handoff",
    );
    assert!(
        !m.unload_active,
        "state-4 handoff must clear the Unit+0x6D1 unload-active latch",
    );
    let entity = sim.entities.get(miner_id).expect("miner entity");
    assert_eq!(
        entity.display_type_override, None,
        "state-4 handoff must clear the unloading display override",
    );
}

#[test]
fn unload_state3_uses_west_cell_building_not_reserved_refinery() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 30, 30);
    spawn_structure_owned(&mut sim, 3, "GAREFN", "Germans", 12, 11);

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
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));

    let americans_before = credits_for_owner(&sim, "Americans");
    let germans_before = credits_for_owner(&sim, "Germans");
    tick_miners_n(&mut sim, &rules, 1);

    assert_eq!(credits_for_owner(&sim, "Americans"), americans_before);
    assert_eq!(credits_for_owner(&sim, "Germans") - germans_before, 100);
    assert_eq!(sim.bale_events.len(), 1);
    assert_eq!(sim.bale_events[0].building_id, 3);
}

#[test]
fn missing_west_cell_building_does_not_credit_or_emit_deposit_event() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 30, 30);

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
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));

    let credits_before = credits_for_owner(&sim, "Americans");
    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.cargo.len(), 1);
    assert_eq!(credits_for_owner(&sim, "Americans"), credits_before);
    assert!(sim.bale_events.is_empty());
}

#[test]
fn state3_null_lookup_preserves_full_cargo_and_returns_to_refinery_selection() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 30, 30);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..miner.capacity_bales {
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
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));

    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.cargo.len(), miner.capacity_bales as usize);
    assert_eq!(miner.state, MinerState::ReturnToRefinery);
    assert_eq!(miner.dock_phase, RefineryDockPhase::Approach);
    assert_eq!(miner.reserved_refinery, None);
}

#[test]
fn state3_null_lookup_does_not_clear_unload_display_latch() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 30, 30);
    let unloading_type = sim.interner.intern("HORV");

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.display_type_override = Some(unloading_type);
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
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));

    tick_miners_n(&mut sim, &rules, 1);

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert!(
        miner.unload_active,
        "state-3 null lookup must preserve the Unit+0x6D1 unload-active latch",
    );
    assert_eq!(entity.display_type_override, Some(unloading_type));
}

#[test]
fn reserved_refinery_released_but_not_used_for_unload_credit_identity() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 30, 30);
    spawn_structure_owned(&mut sim, 3, "GAREFN", "Germans", 12, 11);

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
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, miner_id);
    sim.production.dock_reservations.link_on_pad(2, miner_id);

    let germans_before = credits_for_owner(&sim, "Germans");
    tick_miners_n(&mut sim, &rules, 18);

    assert_eq!(credits_for_owner(&sim, "Germans") - germans_before, 100);
    assert_eq!(sim.bale_events[0].building_id, 3);
    assert!(!sim.production.dock_reservations.has_contact(2, miner_id));
    assert!(!sim.production.dock_reservations.is_on_pad(2, miner_id));
    assert_eq!(get_miner(&sim, miner_id).reserved_refinery, None);
}

#[test]
fn state4_refinery_yes_guard_is_caller_owned() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 30, 30);
    spawn_structure(&mut sim, 3, "GAPOWR", 12, 11);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.clear();
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Departing;
        miner.reserved_refinery = Some(2);
    }
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, miner_id);
    sim.production.dock_reservations.link_on_pad(2, miner_id);

    tick_miners_n(&mut sim, &rules, 1);

    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.state, MinerState::SearchOre);
    assert_eq!(miner.reserved_refinery, None);
    assert!(!sim.production.dock_reservations.has_contact(2, miner_id));
    assert!(!sim.production.dock_reservations.is_on_pad(2, miner_id));
}

#[test]
fn queued_miner_takes_over_immediately_after_empty_gate_handoff() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let occupant = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(occupant).expect("occupant entity");
        let miner = entity.miner.as_mut().expect("occupant miner");
        miner.cargo.clear();
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.unload_timer = 0;
    }
    assert!(sim.production.dock_reservations.try_reserve(2, occupant));
    sim.production
        .dock_reservations
        .mark_contact_entered(2, occupant);
    sim.production.dock_reservations.link_on_pad(2, occupant);

    {
        let entity = sim.entities.get_mut(waiter).expect("waiter entity");
        let miner = entity.miner.as_mut().expect("waiter miner");
        miner.cargo.push(CargoBale {
            resource_type: ResourceType::Ore,
            value: 25,
        });
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::MissionEnter;
        miner.reserved_refinery = Some(2);
        miner.dock_queued = true;
    }
    assert!(!sim.production.dock_reservations.try_reserve(2, waiter));

    tick_miners_n(&mut sim, &rules, 1);
    assert_eq!(
        get_miner(&sim, occupant).dock_phase,
        RefineryDockPhase::Departing,
        "empty gate should reach state-4 handoff before release",
    );
    assert!(
        sim.production.dock_reservations.is_waiting(2, waiter),
        "waiter must remain queued until the state-4 release tick",
    );

    tick_miners_n(&mut sim, &rules, 1);

    let occupant_miner = get_miner(&sim, occupant);
    assert_eq!(occupant_miner.state, MinerState::SearchOre);
    assert!(!sim.production.dock_reservations.has_contact(2, occupant));
    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::MissionEnter,
        "queued miner waits for the stock Enter retry after the freed contact tick",
    );
    tick_miners_n(&mut sim, &rules, 16);
    let waiter_miner = get_miner(&sim, waiter);
    assert_eq!(
        waiter_miner.dock_phase,
        RefineryDockPhase::FaceSync,
        "queued miner takes the freed contact on its next due MissionEnter pass",
    );
    assert!(!waiter_miner.dock_queued);
    assert!(sim.production.dock_reservations.has_contact(2, waiter));
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(2, waiter)
    );
}

/// Verify the purifier bonus scales linearly with the number of purifiers
/// owned. Two purifiers must produce 2× the bonus of one (regression for
/// the old boolean-based formula that capped the bonus at +25% regardless
/// of count).
#[test]
fn two_purifiers_stack_the_bonus_linearly() {
    let mut sim = Simulation::new();
    let rules = purifier_rules(25);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    // Two OrePurifier buildings owned by the same player.
    spawn_structure(&mut sim, 3, "GAPURI", 20, 20);
    spawn_structure(&mut sim, 4, "GAPURI", 24, 20);

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

    // 100 base + (100 × 2 purifiers × 25 / 100) = 100 + 50 = 150.
    let delta = credits_for_owner(&sim, "Americans") - credits_before;
    assert_eq!(
        delta, 150,
        "2 purifiers @ 25% each should stack to +50% (got {} cr)",
        delta,
    );
}

/// AI player with `is_human=false` should receive the virtual-purifier
/// bonus from `rules.general.ai_virtual_purifiers[difficulty]`. With the
/// default `[4, 2, 0]` and difficulty=0 (Brutal), no real purifiers, and
/// a 100-credit bale, total credits = 100 + (100 × 4 × 25 / 100) = 200.
#[test]
fn ai_brutal_gets_virtual_purifier_bonus() {
    use crate::sim::house_state::HouseState;

    let mut sim = Simulation::new();
    let rules = purifier_rules(25);

    // Mark the Americans house as AI, Brutal difficulty.
    let owner_id = sim.interner.intern("Americans");
    sim.houses.insert(
        owner_id,
        HouseState::new(owner_id, 0, None, false, 0, 10), // is_human=false
    );
    sim.game_options.ai_difficulty = 0; // Brutal (top of AIVirtualPurifiers)

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);
    // No real purifiers — bonus should come entirely from the AI table.

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

    let delta = credits_for_owner(&sim, "Americans") - credits_before;
    assert_eq!(
        delta, 200,
        "Brutal AI with 0 real purifiers should get +4 virtual × 25% = +100% (got {} cr)",
        delta,
    );
}

/// Human player with `is_human=true` must NOT get the AI virtual bonus
/// even though the table is configured. Guards against accidentally
/// rewarding the human in skirmish.
#[test]
fn human_player_does_not_get_ai_virtual_bonus() {
    use crate::sim::house_state::HouseState;

    let mut sim = Simulation::new();
    let rules = purifier_rules(25);

    let owner_id = sim.interner.intern("Americans");
    sim.houses.insert(
        owner_id,
        HouseState::new(owner_id, 0, None, true, 0, 10), // is_human=true
    );
    sim.game_options.ai_difficulty = 0;

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

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

    let delta = credits_for_owner(&sim, "Americans") - credits_before;
    assert_eq!(
        delta, 100,
        "human player with no real purifiers gets base credits only (got {} cr)",
        delta,
    );
}

/// Legacy DepositCooldown save states still count down and pass through to
/// Departing, even though stock unload no longer enters this phase.
#[test]
fn legacy_deposit_cooldown_passes_through_to_departing() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.clear();
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::DepositCooldown;
        miner.reserved_refinery = Some(2);
        miner.deposit_cooldown_ticks = 2;
    }
    sim.production.dock_reservations.try_reserve(2, miner_id);

    tick_miners_n(&mut sim, &rules, 1);
    let m = get_miner(&sim, miner_id);
    assert_eq!(m.dock_phase, RefineryDockPhase::DepositCooldown);
    assert_eq!(m.deposit_cooldown_ticks, 1);

    tick_miners_n(&mut sim, &rules, 2);
    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.dock_phase,
        RefineryDockPhase::Departing,
        "legacy cooldown completion should pass through to Departing",
    );
}

/// A miner flagged `dying = true` (death animation still playing) must NOT
/// hold its refinery dock reservation. Queued miners need to be promoted
/// on the next tick — without waiting for the death animation to finish
/// and `despawn_entity` to remove the corpse from the entity store.
#[test]
fn dying_occupant_releases_dock_to_queued_miner() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let occupant = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    let waiter = spawn_miner(&mut sim, 3, MinerKind::War, 14, 12);
    spawn_refinery(&mut sim, 2, 10, 10);

    sim.production.dock_reservations.try_reserve(2, occupant);
    assert!(!sim.production.dock_reservations.try_reserve(2, waiter));
    assert_eq!(
        sim.production.dock_reservations.has_contact(2, occupant),
        true,
        "precondition: occupant has refinery contact",
    );

    sim.entities
        .get_mut(occupant)
        .expect("occupant entity")
        .dying = true;

    tick_miners_n(&mut sim, &rules, 1);

    assert_eq!(
        sim.production.dock_reservations.is_waiting(2, waiter),
        true,
        "queued miner must remain next in retry order once the occupant enters its death phase",
    );
}

/// A full miner whose reserved refinery enters its death animation must not
/// fall back into ore search. Mission_Harvest checks full storage before
/// scanning for ore, so the miner keeps looking for a refinery target.
#[test]
fn full_miner_losing_dying_refinery_keeps_returning() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 5, 10);
    spawn_refinery(&mut sim, 2, 10, 10);
    spawn_refinery(&mut sim, 3, 24, 10);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..miner.capacity_bales {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::ReturnToRefinery;
        miner.reserved_refinery = Some(2);
        miner.target_ore_cell = Some((6, 10));
        miner.dock_queued = true;
    }
    sim.entities.get_mut(2).expect("refinery").dying = true;

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.state,
        MinerState::ReturnToRefinery,
        "full miner must keep returning after a reserved refinery becomes invalid",
    );
    assert_eq!(
        m.reserved_refinery, None,
        "invalid dying refinery reservation must be cleared before re-selection",
    );
    assert_eq!(
        m.cargo.len(),
        m.capacity_bales as usize,
        "cargo must be preserved when the refinery disappears",
    );
    assert_eq!(
        m.target_ore_cell, None,
        "full return fallback must not keep a stale ore target",
    );
    assert!(!m.dock_queued, "stale dock queue state must be cleared");

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.reserved_refinery,
        Some(3),
        "next return tick must choose the remaining live refinery, not the dying one",
    );
}

/// If the refinery is sold/destroyed while a miner is visually unloading,
/// Rust must abort the dock sequence instead of crediting more bales to a
/// dying building or leaving the miner rendered as its unloading class.
#[test]
fn dying_refinery_aborts_unload_without_credit_or_stuck_visual() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    spawn_refinery(&mut sim, 2, 10, 10);

    let credits_before = credits_for_owner(&sim, "Americans");
    let unloading_type = sim.interner.intern("HORV");

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.display_type_override = Some(unloading_type);
        entity.facing_target = Some(0x40);
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..miner.capacity_bales {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::Unloading;
        miner.reserved_refinery = Some(2);
        miner.dock_queued = true;
        miner.exit_cell = Some((14, 11));
        miner.deposit_cooldown_ticks = 7;
        miner.unload_timer = 0;
    }
    assert!(sim.production.dock_reservations.try_reserve(2, miner_id));
    sim.entities.get_mut(2).expect("refinery").dying = true;

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        credits_for_owner(&sim, "Americans"),
        credits_before,
        "dying refinery must not receive unload credits",
    );
    assert_eq!(
        m.cargo.len(),
        m.capacity_bales as usize,
        "abort must preserve the miner cargo instead of draining bales",
    );
    assert_eq!(
        m.state,
        MinerState::ReturnToRefinery,
        "full miner must return to refinery selection after an unload abort",
    );
    assert_eq!(m.dock_phase, RefineryDockPhase::Approach);
    assert_eq!(m.reserved_refinery, None);
    assert!(!m.dock_queued, "queued flag must be cleared on abort");
    assert_eq!(m.exit_cell, None, "exit cache must be cleared on abort");
    assert_eq!(
        m.deposit_cooldown_ticks, 0,
        "deposit cooldown must not survive an abort",
    );
    assert_eq!(m.unload_timer, 0, "unload timer must be reset on abort");
    assert!(
        !sim.production.dock_reservations.is_occupied(2),
        "dock reservation must be removed for a dying refinery",
    );

    let entity = sim.entities.get(miner_id).expect("miner entity");
    assert_eq!(
        entity.display_type_override,
        Some(unloading_type),
        "state-3 missing-building cleanup preserves the unload display latch until state-4/abort cleanup owns it",
    );
    assert_eq!(
        entity.facing_target, None,
        "dock pivot target must be cleared on abort",
    );
}

// ---------------------------------------------------------------------------
// Scan filter: cell-occupancy + path-grid exclusion (gamemd parity)
//
// Mirrors gamemd's `Scan_For_Tiberium` → `Is_Cell_Harvestable` →
// `UnitClass::Can_Enter_Cell` (vtable+0x1AC at 0x0073F0A0): rings 1+
// reject cells with vehicle occupants, terrain objects, or building
// footprints. Ring 0 (the harvester's own cell) is always allowed.
// ---------------------------------------------------------------------------

/// Path-grid-blocked ore cell (e.g. tree on tiberium) is rejected by the
/// scan; harvester targets the next-best clear cell instead.
#[test]
fn scan_skips_tree_blocked_ore_cell() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // 32×32 all-passable grid except for one tree on the would-be best ore
    // cell at (10, 10). The other ore at (12, 10) is also reachable but
    // farther, so without the path-grid filter the scan would pick (10, 10).
    let mut grid = PathGrid::new(32, 32);
    grid.set_blocked(10, 10, true);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 32, 32);
    sim.zone_grid = Some(zone_grid);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 5, 10);
    place_ore(&mut sim, 10, 10, 1200);
    place_ore(&mut sim, 12, 10, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        entity.miner.as_mut().expect("miner").state = MinerState::SearchOre;
    }

    // Use a path_grid that matches the blocked cell so build_scan_filter
    // sees the tree. tick_miners_n's default 64×64 all-passable grid would
    // miss it, so call tick_miners directly with the right grid.
    let config = MinerConfig::default();
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let m = get_miner(&sim, miner_id);
    assert_ne!(
        m.target_ore_cell,
        Some((10, 10)),
        "must not target tree-blocked ore cell (10,10)",
    );
    assert_eq!(
        m.target_ore_cell,
        Some((12, 10)),
        "must fall through to the next-best clear ore cell",
    );
}

/// An ore cell occupied by another vehicle (e.g. a war miner sitting on
/// it harvesting) is rejected by ring 1+ scan; harvester targets a
/// different cell.
#[test]
fn scan_skips_cell_occupied_by_other_miner() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    let grid = PathGrid::new(32, 32);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 32, 32);
    sim.zone_grid = Some(zone_grid);

    // Miner A sits on ore at (10, 10). Miner B at (5, 10) is the scanner.
    let _miner_a = spawn_miner(&mut sim, 1, MinerKind::War, 10, 10);
    sim.occupancy.add(
        10,
        10,
        1,
        MovementLayer::Ground,
        None,
        CellListInsertion::PrependNonBuilding,
    );
    let miner_b = spawn_miner(&mut sim, 2, MinerKind::War, 5, 10);
    sim.occupancy.add(
        5,
        10,
        2,
        MovementLayer::Ground,
        None,
        CellListInsertion::PrependNonBuilding,
    );

    place_ore(&mut sim, 10, 10, 1200);
    place_ore(&mut sim, 12, 10, 1200);

    {
        let entity = sim.entities.get_mut(miner_b).expect("miner B");
        entity.miner.as_mut().expect("miner").state = MinerState::SearchOre;
    }

    let config = MinerConfig::default();
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let m = get_miner(&sim, miner_b);
    assert_ne!(
        m.target_ore_cell,
        Some((10, 10)),
        "must not target cell occupied by another miner",
    );
    assert_eq!(
        m.target_ore_cell,
        Some((12, 10)),
        "must fall through to the next clear ore cell",
    );
}

/// Ring 0 (harvester's own cell) is unfiltered — a harvester standing on
/// its own ore cell continues to harvest it even though it appears as a
/// blocker in OccupancyGrid.
#[test]
fn scan_ring_0_allows_harvesters_own_cell() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    let grid = PathGrid::new(32, 32);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 32, 32);
    sim.zone_grid = Some(zone_grid);

    // Miner on ore at (10, 10). Register itself as occupant — ring 0
    // must still return (10, 10).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 10, 10);
    sim.occupancy.add(
        10,
        10,
        1,
        MovementLayer::Ground,
        None,
        CellListInsertion::PrependNonBuilding,
    );
    place_ore(&mut sim, 10, 10, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        entity.miner.as_mut().expect("miner").state = MinerState::SearchOre;
    }

    let config = MinerConfig::default();
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.target_ore_cell,
        Some((10, 10)),
        "ring-0 fast path must return the harvester's own ore cell",
    );
}

// ---------------------------------------------------------------------------
// MoveToOre per-tick rescan (gamemd parity for Mission_Harvest state 0)
// ---------------------------------------------------------------------------

/// If a tree blocks the initially-chosen ore cell, the miner must NOT
/// target it on first scan — the scan filter rejects it, and a different
/// ore cell is picked from the start.
#[test]
fn move_to_ore_avoids_tree_blocked_cell_from_start() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    let mut grid = PathGrid::new(32, 32);
    grid.set_blocked(12, 12, true);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 32, 32);
    sim.zone_grid = Some(zone_grid);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 8, 12);
    place_ore(&mut sim, 12, 12, 1200);
    place_ore(&mut sim, 13, 13, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        entity.miner.as_mut().expect("miner").state = MinerState::SearchOre;
    }

    let config = MinerConfig::default();
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let m = get_miner(&sim, miner_id);
    assert_ne!(
        m.target_ore_cell,
        Some((12, 12)),
        "tree-blocked cell rejected"
    );
    assert!(
        matches!(m.state, MinerState::MoveToOre | MinerState::Harvest),
        "must transition out of SearchOre — got {:?}",
        m.state,
    );
}

/// When the current target ore cell becomes blocked mid-move (a tree
/// appears, another miner parks on it), the per-tick rescan retargets
/// to the next-best available cell.
#[test]
fn move_to_ore_retargets_when_blocker_appears() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    let mut grid = PathGrid::new(32, 32);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 32, 32);
    sim.zone_grid = Some(zone_grid);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 8, 12);
    place_ore(&mut sim, 12, 12, 1200);
    place_ore(&mut sim, 11, 12, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        entity.miner.as_mut().expect("miner").state = MinerState::SearchOre;
    }

    let config = MinerConfig::default();
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let initial_target = get_miner(&sim, miner_id).target_ore_cell;
    assert!(initial_target.is_some(), "must pick an initial target");

    // Block whichever cell was picked. Build a fresh grid with that cell
    // blocked so the next tick's scan filter rejects it.
    let blocked_cell = initial_target.unwrap();
    grid.set_blocked(blocked_cell.0, blocked_cell.1, true);
    let zone_grid_2 = ZoneGrid::build(&grid, &BTreeMap::new(), 32, 32);
    sim.zone_grid = Some(zone_grid_2);

    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));

    let new_target = get_miner(&sim, miner_id).target_ore_cell;
    assert_ne!(
        new_target, initial_target,
        "must retarget when initial cell becomes blocked",
    );
    assert!(new_target.is_some(), "must pick an alternative cell");
}

/// Per-tick rescan must NOT thrash — with a stable world the chosen
/// target stays the same tick after tick.
#[test]
fn move_to_ore_target_stable_when_world_unchanged() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    let grid = PathGrid::new(32, 32);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 32, 32);
    sim.zone_grid = Some(zone_grid);

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 8, 12);
    place_ore(&mut sim, 14, 12, 1200);
    place_ore(&mut sim, 15, 12, 1200);
    place_ore(&mut sim, 16, 12, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        entity.miner.as_mut().expect("miner").state = MinerState::SearchOre;
    }

    let config = MinerConfig::default();
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));
    let t1 = get_miner(&sim, miner_id).target_ore_cell;

    // Several ticks later, with the world unchanged, the target must not
    // have shifted to a different ore cell. (It will only change once the
    // miner physically moves close enough that ring distances shift, but
    // a single tick without movement should be stable.)
    super::miner_system::tick_miners(&mut sim, &rules, &config, Some(&grid));
    let t2 = get_miner(&sim, miner_id).target_ore_cell;

    assert_eq!(t1, t2, "stable world → stable target across ticks");
}
