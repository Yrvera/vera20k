//! Tests for `Simulation::tick_attack_pursuit` — the pre-combat stage
//! that walks units toward out-of-range attack targets and halts them
//! when in range.

use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::combat::{AttackTarget, TargetKind};
use crate::sim::components::Health;
use crate::sim::docking::aircraft_dock::AircraftAmmo;
use crate::sim::game_entity::GameEntity;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

/// Minimal RuleSet for pursuit tests: armed Grizzly + Rhino, AP warhead with
/// non-zero Verses against heavy. Range=6 cells.
fn pursuit_rules() -> RuleSet {
    let ini_str: &str = "\
[VehicleTypes]\n0=MTNK\n1=HTNK\n\n\
[InfantryTypes]\n0=ENGI\n\n\
[BuildingTypes]\n0=GAPILL\n\n\
[AircraftTypes]\n0=ORCA\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[HTNK]\nStrength=400\nArmor=heavy\nSpeed=5\nPrimary=105mm\n\n\
[ENGI]\nStrength=75\nArmor=none\nSpeed=4\n\n\
[GAPILL]\nStrength=400\nArmor=heavy\nPrimary=105mm\n\n\
[ORCA]\nStrength=150\nArmor=light\nSpeed=14\nPrimary=105mm\n\n\
[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n";
    let ini: IniFile = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("pursuit_rules should parse")
}

/// Construct a Simulation with a flat 64x64 PathGrid and the given entities
/// pre-inserted. Returns the sim plus the path grid (kept alive separately
/// because tick_attack_pursuit borrows it).
///
/// Replaces the sim's interner with the thread-local test interner so the
/// type_ref / owner IDs that `GameEntity::test_default` baked in via
/// `test_intern()` resolve correctly.
fn make_sim(entities: Vec<GameEntity>) -> (Simulation, PathGrid) {
    let mut sim = Simulation::new();
    for e in entities {
        sim.substrate.entities.insert(e);
    }
    sim.interner = crate::sim::intern::test_interner();
    let grid = PathGrid::test_all_passable(64, 64);
    (sim, grid)
}

fn make_unit(id: u64, type_ref: &str, owner: &str, rx: u16, ry: u16, hp: u16) -> GameEntity {
    let mut e = GameEntity::test_default(id, type_ref, owner, rx, ry);
    e.health = Health {
        current: hp,
        max: hp,
    };
    e
}

#[test]
fn cell_target_out_of_range_issues_movement() {
    // Grizzly at (5,5), force-fire Cell(15,15). Range=6, distance=10 → out of range.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 5, 5, 300);
    grizzly.attack_target = Some(AttackTarget::for_cell(15, 15));
    let (mut sim, grid) = make_sim(vec![grizzly]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_some(),
        "attack_target preserved during pursuit"
    );
    assert!(
        entity.movement_target.is_some(),
        "out-of-range cell target should issue movement"
    );
}

#[test]
fn cell_target_in_range_clears_movement() {
    // Grizzly at (8,5), force-fire Cell(10,5). Distance=2 → in range.
    // Pre-set a movement_target as if pursuit had issued one earlier.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 8, 5, 300);
    grizzly.attack_target = Some(AttackTarget::for_cell(10, 5));
    grizzly.movement_target = Some(crate::sim::components::MovementTarget::default());
    let (mut sim, grid) = make_sim(vec![grizzly]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_some(),
        "attack_target preserved on range entry"
    );
    assert!(
        entity.movement_target.is_none(),
        "in-range pursuit should halt movement"
    );
}

#[test]
fn entity_target_out_of_range_pursues() {
    // Grizzly at (0,0) attacking Rhino at (10,0). Out of range.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    grizzly.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 10, 0, 400);
    let (mut sim, grid) = make_sim(vec![grizzly, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(entity.attack_target.is_some());
    assert!(
        entity.movement_target.is_some(),
        "out-of-range entity target should issue movement"
    );
}

#[test]
fn entity_target_dying_pursuit_skips() {
    // Target marked dying — resolve_target_coords still resolves, but combat
    // tick will clean up. Pursuit should not crash here.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    grizzly.attack_target = Some(AttackTarget::new(2));
    let mut rhino = make_unit(2, "HTNK", "Soviet", 10, 0, 0);
    rhino.dying = true;
    rhino.health.current = 0;
    let (mut sim, grid) = make_sim(vec![grizzly, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));
    assert!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .attack_target
            .is_some()
    );
}

#[test]
fn aircraft_attack_target_skipped_by_pursuit() {
    // Aircraft has its own attack-mission state machine; pursuit must not
    // touch its movement.
    let mut orca = make_unit(1, "ORCA", "Americans", 0, 0, 150);
    orca.attack_target = Some(AttackTarget::new(2));
    orca.aircraft_mission = Some(AircraftMission::Attack {
        sub_state: 3,
        has_fired: false,
        is_strafe: false,
    });
    orca.aircraft_ammo = Some(AircraftAmmo::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 30, 0, 400);
    let (mut sim, grid) = make_sim(vec![orca, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.movement_target.is_none(),
        "aircraft pursuit must not be touched by ground pursuit stage"
    );
}

#[test]
fn structure_attack_target_skipped_by_pursuit() {
    // Garrisoned building (or any structure) has attack_target but cannot move.
    let mut pillbox = make_unit(1, "GAPILL", "Americans", 5, 5, 400);
    pillbox.category = crate::map::entities::EntityCategory::Structure;
    pillbox.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 30, 5, 400);
    let (mut sim, grid) = make_sim(vec![pillbox, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.movement_target.is_none(),
        "structures must not pursue"
    );
}

#[test]
fn deployed_infantry_skipped_by_pursuit() {
    // Deploy-fire infantry (e.g., GI in deployed state) cannot move.
    let mut gi = make_unit(1, "ENGI", "Americans", 5, 5, 75);
    gi.category = crate::map::entities::EntityCategory::Infantry;
    gi.deploy_state = Some(crate::sim::deploy::DeployPhase::Deployed);
    gi.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 30, 5, 400);
    let (mut sim, grid) = make_sim(vec![gi, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.movement_target.is_none(),
        "deployed infantry must not pursue"
    );
}

#[test]
fn pursuit_uses_same_range_as_combat_no_oscillation() {
    // Place attacker exactly at the boundary. The combat tick range check
    // and pursuit range check use the same `is_within_range_leptons`, so
    // both must agree at the boundary. Verify: at exactly Range cells,
    // pursuit treats it as in-range (clears movement if any).
    //
    // 105mm Range=6. Place Grizzly at (0,0), target Cell(6,0). Distance = 6 cells exactly.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    grizzly.attack_target = Some(AttackTarget::for_cell(6, 0));
    grizzly.movement_target = Some(crate::sim::components::MovementTarget::default());
    let (mut sim, grid) = make_sim(vec![grizzly]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    // is_within_range_leptons is inclusive at the boundary. Pursuit should
    // halt (clear movement). If pursuit and combat used different math,
    // this would fail.
    assert!(
        entity.movement_target.is_none(),
        "at exactly weapon range, pursuit must halt (matches combat tick range check)"
    );
}

/// **Sticky never chases.** Guard(5) and Sticky(6) share one mission handler,
/// and the single place the engine tells them apart is here: when the object
/// cannot already fire at its target, a Sticky object drops both the target and
/// the destination and produces no pursuit cell — ahead of the fallthrough that
/// lets a Guard-family object pursue. That is the whole of `[Sticky]`'s "just
/// like guard mode, but cannot move". Stock skirmish maps park neutral civilian
/// traffic on this mission (46 authored placements across the stock MP bundle,
/// 17 on one map), so without it a shot-at civilian truck drives at the shooter.
#[test]
fn sticky_drops_the_target_instead_of_chasing_it() {
    let mut civilian = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    civilian.attack_target = Some(AttackTarget::new(2));
    civilian
        .mission
        .apply_test_fixture(crate::sim::mission::state::MissionTestFixture {
            current: crate::sim::mission::MissionId::from_known(
                crate::sim::mission::MissionType::Sticky,
            ),
            suspended: crate::sim::mission::MissionId::NONE,
            queued: crate::sim::mission::MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: crate::sim::mission::MissionDispatchTimer::at_frame(0),
        });
    // 105mm Range=6; the Rhino sits at 10 cells, so the can-fire-at query fails.
    let rhino = make_unit(2, "HTNK", "Soviet", 10, 0, 400);
    let (mut sim, grid) = make_sim(vec![civilian, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_none(),
        "Sticky drops the target it cannot shoot"
    );
    assert!(
        entity.movement_target.is_none(),
        "Sticky produces no pursuit cell"
    );
    assert!(entity.navigation.nav_com.is_none());
}

/// The same object on Guard — the mission Sticky shares its handler with —
/// still pursues. This is the tripwire proving the clause keys on the mission
/// id and not on something both missions share.
#[test]
fn guard_still_chases_where_sticky_would_not() {
    let mut guard = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    guard.attack_target = Some(AttackTarget::new(2));
    guard
        .mission
        .apply_test_fixture(crate::sim::mission::state::MissionTestFixture {
            current: crate::sim::mission::MissionId::from_known(
                crate::sim::mission::MissionType::Guard,
            ),
            suspended: crate::sim::mission::MissionId::NONE,
            queued: crate::sim::mission::MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: crate::sim::mission::MissionDispatchTimer::at_frame(0),
        });
    let rhino = make_unit(2, "HTNK", "Soviet", 10, 0, 400);
    let (mut sim, grid) = make_sim(vec![guard, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(entity.attack_target.is_some());
    assert!(
        entity.movement_target.is_some(),
        "Guard is not short-circuited"
    );
}
