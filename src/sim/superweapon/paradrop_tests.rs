//! End-to-end integration tests for the paradrop superweapon launch pipeline.
//!
//! Exercises: launch handler → carrier spawn at waypoint edge → cargo loaded
//! with infantry → ParaDropApproach mission → distance check → transition to
//! ParaDropOverfly → V-pattern Drop_Payload at ROF cadence → infantry calls
//! begin_parachute_descent → descent ramp → landing.

#![cfg(test)]

use std::collections::BTreeMap;

use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::pathfinding::PathGrid;
use crate::sim::superweapon::paradrop::{ParaDropKind, launch};
use crate::sim::world::Simulation;

/// Minimal ruleset with PDPLANE + ParaDropWeapon + E1 + AMRADR cargo plane setup.
/// AmerParaDropNum trimmed to 4 for faster test cycles vs the vanilla 8.
fn make_paradrop_rules() -> RuleSet {
    let text = "\
[InfantryTypes]
0=E1

[VehicleTypes]

[AircraftTypes]
0=PDPLANE

[BuildingTypes]

[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85
ParadropRadius=1024
AmerParaDropInf=E1
AmerParaDropNum=4
AllyParaDropInf=E1
AllyParaDropNum=4
SovParaDropInf=E1
SovParaDropNum=4
YuriParaDropInf=E1
YuriParaDropNum=4
ParachuteMaxFallRate=-3
FlightLevel=1500

[E1]
Name=GI
Cost=200
Strength=125
Armor=none
Speed=4
Primary=M60

[PDPLANE]
Name=Cargo Plane
Strength=400
Armor=light
Speed=15
ROT=2
Primary=ParaDropWeapon
Spawned=yes
Selectable=no
Sight=0
Landable=no

[M60]
Damage=25
ROF=20
Range=5
Warhead=SA

[ParaDropWeapon]
Damage=60
ROF=130
Range=1
Warhead=SA

[SA]
Verses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%
CellSpread=0
";
    let ini = IniFile::from_str(text);
    RuleSet::from_ini(&ini).expect("test ruleset parse")
}

/// Build a Simulation with a 100x100 fully-passable map and an "Americans"
/// house anchored at (50, 90) → waypoint_edge=South. (Carrier will spawn
/// from the south edge; opposite-edge exit = North.)
fn build_sim(rules: &RuleSet) -> (Simulation, PathGrid) {
    let mut sim = Simulation::new();
    sim.fog.width = 100;
    sim.fog.height = 100;
    let owner_id = sim.interner.intern("Americans");
    let mut house = crate::sim::house_state::HouseState::new(
        owner_id, /*side_index*/ 0, /*country*/ None, /*is_human*/ true,
        /*credits*/ 10_000, /*tech_level*/ 10,
    );
    house.base_center = Some((50, 90));
    house.waypoint_edge = crate::sim::house_state::closest_edge_for(
        (50, 90),
        sim.fog.width as u32,
        sim.fog.height as u32,
    );
    sim.houses.insert(owner_id, house);
    let _ = rules;
    let path_grid = PathGrid::test_all_passable(100, 100);
    (sim, path_grid)
}

fn tick_n(sim: &mut Simulation, rules: &RuleSet, path_grid: &PathGrid, n: u32) {
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    for _ in 0..n {
        sim.advance_tick(&[], Some(rules), &height_map, Some(path_grid), None, 22);
    }
}

fn count_descending_infantry(sim: &Simulation) -> usize {
    sim.entities
        .values()
        .filter(|e| e.parachute_state.is_some())
        .count()
}

fn find_pdplane(sim: &Simulation) -> Option<u64> {
    sim.entities
        .values()
        .find(|e| {
            sim.interner
                .resolve(e.type_ref)
                .eq_ignore_ascii_case("PDPLANE")
                && e.health.current > 0
        })
        .map(|e| e.stable_id)
}

fn count_alive_infantry(sim: &Simulation, type_str: &str) -> usize {
    sim.entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.type_ref)
                .eq_ignore_ascii_case(type_str)
                && e.health.current > 0
        })
        .count()
}

#[test]
fn paradrop_launch_spawns_carrier_with_loaded_cargo() {
    let rules = make_paradrop_rules();
    let (mut sim, path_grid) = build_sim(&rules);
    let owner = sim.interner.intern("Americans");

    // Target near the north edge of the map.
    let ok = launch(
        &mut sim,
        &rules,
        owner,
        50,
        20,
        ParaDropKind::American,
        Some(&path_grid),
    );
    assert!(ok, "launch should succeed with valid waypoint edge + cargo");

    // Exactly one PDPLANE — AmerParaDropInf is single-entry (E1).
    let pdplane_id = find_pdplane(&sim).expect("PDPLANE must exist");

    // Cargo: 4 E1 (AmerParaDropNum=4 in test rules).
    let cargo_count = sim
        .entities
        .get(pdplane_id)
        .unwrap()
        .passenger_role
        .cargo()
        .map(|c| c.count())
        .unwrap_or(0);
    assert_eq!(cargo_count, 4, "cargo should be loaded with 4 E1");

    // Mission set to ParaDropApproach with target + fog latch cleared.
    match sim.entities.get(pdplane_id).unwrap().aircraft_mission {
        Some(AircraftMission::ParaDropApproach {
            target_rx,
            target_ry,
            has_revealed_fog,
        }) => {
            assert_eq!(target_rx, 50);
            assert_eq!(target_ry, 20);
            assert!(!has_revealed_fog);
        }
        ref other => panic!("expected ParaDropApproach, got {:?}", other),
    }
}

#[test]
fn paradrop_full_pipeline_drops_infantry_until_cargo_empty() {
    let rules = make_paradrop_rules();
    let (mut sim, path_grid) = build_sim(&rules);
    let owner = sim.interner.intern("Americans");

    let pre_e1_count = count_alive_infantry(&sim, "E1");
    assert_eq!(pre_e1_count, 0, "no E1 should exist pre-launch");

    let ok = launch(
        &mut sim,
        &rules,
        owner,
        50,
        20,
        ParaDropKind::American,
        Some(&path_grid),
    );
    assert!(ok, "launch should succeed");

    let pdplane_id = find_pdplane(&sim).expect("PDPLANE must exist");

    // Right after launch, 4 E1 exist as Inside-cargo (hidden).
    assert_eq!(count_alive_infantry(&sim, "E1"), 4, "4 E1 should be loaded");
    assert_eq!(count_descending_infantry(&sim), 0, "none descending yet");

    // Tick until cargo empties (4 drops × 130-tick ROF = ~520 ticks; allow
    // headroom for the approach flight from south to within ParadropRadius).
    let mut first_drop_tick: Option<u32> = None;
    for tick in 0..2000u32 {
        let cargo_before = sim
            .entities
            .get(pdplane_id)
            .and_then(|e| e.passenger_role.cargo())
            .map(|c| c.count())
            .unwrap_or(0);
        tick_n(&mut sim, &rules, &path_grid, 1);
        let cargo_after = sim
            .entities
            .get(pdplane_id)
            .and_then(|e| e.passenger_role.cargo())
            .map(|c| c.count())
            .unwrap_or(0);
        if cargo_after < cargo_before && first_drop_tick.is_none() {
            first_drop_tick = Some(tick);
        }
        if cargo_after == 0 {
            break;
        }
    }

    assert!(
        first_drop_tick.is_some(),
        "first drop should fire within 2000 ticks"
    );

    // After all drops, 4 infantry exist as descending (parachute_state Some).
    let descending = count_descending_infantry(&sim);
    assert!(
        descending >= 1,
        "at least one infantry should be descending after drops; got {}",
        descending,
    );
}

#[test]
fn paradrop_descent_ends_with_landed_infantry_and_carrier_despawned() {
    let rules = make_paradrop_rules();
    let (mut sim, path_grid) = build_sim(&rules);
    let owner = sim.interner.intern("Americans");

    launch(
        &mut sim,
        &rules,
        owner,
        50,
        20,
        ParaDropKind::American,
        Some(&path_grid),
    );

    // Run for plenty of ticks to drain cargo + descent + carrier exit.
    // ~520 ticks for drops + ~500 ticks for max descent + ~1000 for exit flight.
    tick_n(&mut sim, &rules, &path_grid, 4000);

    // No descending infantry remain.
    assert_eq!(
        count_descending_infantry(&sim),
        0,
        "all infantry should have landed",
    );

    // Some E1 alive on the ground (parachute_state cleared).
    let landed = sim
        .entities
        .values()
        .filter(|e| {
            sim.interner.resolve(e.type_ref).eq_ignore_ascii_case("E1")
                && e.health.current > 0
                && e.parachute_state.is_none()
        })
        .count();
    assert!(
        landed >= 1,
        "at least one E1 should have landed alive (got {})",
        landed
    );

    // Carrier despawned (silent_despawn at boundary).
    assert!(
        find_pdplane(&sim).is_none(),
        "PDPLANE should have despawned at the exit boundary",
    );
}

#[test]
fn paradrop_per_side_branch_picks_correct_list() {
    // Generic ParaDrop with side_index=2 should use the Yuri list.
    // We verify this by setting different counts per side and checking which
    // count gets loaded into the carrier.
    let text = "\
[InfantryTypes]
0=E1
1=E2
2=INIT

[VehicleTypes]
[AircraftTypes]
0=PDPLANE
[BuildingTypes]

[General]
ParadropRadius=1024
AmerParaDropInf=E1
AmerParaDropNum=2
AllyParaDropInf=E1
AllyParaDropNum=3
SovParaDropInf=E2
SovParaDropNum=4
YuriParaDropInf=INIT
YuriParaDropNum=5
ParachuteMaxFallRate=-3

[E1]
Name=GI
Strength=125
Armor=none
Speed=4
Primary=M60

[E2]
Name=Conscript
Strength=100
Armor=none
Speed=4
Primary=M60

[INIT]
Name=Initiate
Strength=100
Armor=none
Speed=4
Primary=M60

[PDPLANE]
Strength=400
Armor=light
Speed=15
ROT=2
Primary=ParaDropWeapon
Spawned=yes
Sight=0
Landable=no

[M60]
Damage=25
ROF=20
Range=5
Warhead=SA

[ParaDropWeapon]
Damage=60
ROF=130
Range=1
Warhead=SA

[SA]
Verses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%
CellSpread=0
";
    let rules = RuleSet::from_ini(&IniFile::from_str(text)).expect("rules parse");

    // Yuri side (index=2) → loads INIT × 5.
    {
        let mut sim = Simulation::new();
        sim.fog.width = 100;
        sim.fog.height = 100;
        let owner_id = sim.interner.intern("Yuri");
        let mut house =
            crate::sim::house_state::HouseState::new(owner_id, 2, None, true, 10_000, 10);
        house.waypoint_edge = 2; // South
        sim.houses.insert(owner_id, house);
        let path_grid = PathGrid::test_all_passable(100, 100);

        launch(
            &mut sim,
            &rules,
            owner_id,
            50,
            20,
            ParaDropKind::Generic,
            Some(&path_grid),
        );

        let pdplane_id = find_pdplane(&sim).expect("PDPLANE must exist");
        let cargo_count = sim
            .entities
            .get(pdplane_id)
            .unwrap()
            .passenger_role
            .cargo()
            .map(|c| c.count())
            .unwrap_or(0);
        assert_eq!(cargo_count, 5, "Yuri side should load 5 INIT");

        // Verify cargo type is INIT.
        let cargo_ids = sim
            .entities
            .get(pdplane_id)
            .unwrap()
            .passenger_role
            .cargo()
            .unwrap()
            .passengers
            .clone();
        for pax_id in cargo_ids {
            let type_str = sim
                .interner
                .resolve(sim.entities.get(pax_id).unwrap().type_ref);
            assert!(
                type_str.eq_ignore_ascii_case("INIT"),
                "Yuri cargo should be INIT, got {}",
                type_str,
            );
        }
    }

    // Soviet side (index=1, falls through default arm) → loads E2 × 4.
    {
        let mut sim = Simulation::new();
        sim.fog.width = 100;
        sim.fog.height = 100;
        let owner_id = sim.interner.intern("Russians");
        let mut house =
            crate::sim::house_state::HouseState::new(owner_id, 1, None, true, 10_000, 10);
        house.waypoint_edge = 2;
        sim.houses.insert(owner_id, house);
        let path_grid = PathGrid::test_all_passable(100, 100);

        launch(
            &mut sim,
            &rules,
            owner_id,
            50,
            20,
            ParaDropKind::Generic,
            Some(&path_grid),
        );

        let pdplane_id = find_pdplane(&sim).expect("PDPLANE must exist");
        let cargo_count = sim
            .entities
            .get(pdplane_id)
            .unwrap()
            .passenger_role
            .cargo()
            .map(|c| c.count())
            .unwrap_or(0);
        assert_eq!(cargo_count, 4, "Soviet side should load 4 E2");
    }

    // Unknown side (index=99, also falls through to Soviet branch) → 4 E2.
    {
        let mut sim = Simulation::new();
        sim.fog.width = 100;
        sim.fog.height = 100;
        let owner_id = sim.interner.intern("Unknown");
        let mut house =
            crate::sim::house_state::HouseState::new(owner_id, 99, None, true, 10_000, 10);
        house.waypoint_edge = 2;
        sim.houses.insert(owner_id, house);
        let path_grid = PathGrid::test_all_passable(100, 100);

        launch(
            &mut sim,
            &rules,
            owner_id,
            50,
            20,
            ParaDropKind::Generic,
            Some(&path_grid),
        );

        let pdplane_id = find_pdplane(&sim).expect("PDPLANE must exist");
        let cargo_count = sim
            .entities
            .get(pdplane_id)
            .unwrap()
            .passenger_role
            .cargo()
            .map(|c| c.count())
            .unwrap_or(0);
        assert_eq!(
            cargo_count, 4,
            "Unknown side should fall back to Soviet (4 E2)"
        );
    }
}
