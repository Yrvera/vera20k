//! Tests for `Simulation::tick_attack_pursuit` — the pre-combat stage
//! that walks units toward out-of-range attack targets and halts them
//! when in range.

use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::combat::AttackTarget;
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

// ---------------------------------------------------------------------------
// The pursuit predicate is the fire gate's predicate, walk included.
//
// `FootClass::Mission_Attack @ 0x004D4DC0` dispatches to the approach search
// through vtable slot `+0x53C` (`CALL [EAX+0x53c]` at `0x004D4E6A`; the body is
// `FootClass::Greatest_Threat_Scan @ 0x004D5690`, and InfantryClass's override
// `0x00522340` chains into it at `0x0052236E`), and that body decides with
// `TechnoClass::InRange @ 0x006F7220`
// — called at `0x004D622C` and `0x004D6550` with a candidate coordinate as arg1
// and TarCom as arg2. `InRange` ends in the wall/cliff walk
// (`CALL 0x004CC310` at `0x006F7642`). So in gamemd the approach test and the
// fire test are literally the same call, and VERA's two stages must not use
// different predicates: if pursuit measured the plain radius while the fire
// gate ran the walk, a unit ordered to shoot across a wall would halt here and
// then be refused the shot, freezing under a live order.
// ---------------------------------------------------------------------------

/// Rules for the wall cases: a `SubjectToWalls=yes` projectile whose warhead
/// carries no `Wall=`, so `0x004CC342` cannot re-admit the blocked shot.
///
/// `[OverlayTypes]` is read by DECLARATION index (`RulesClass::Process`
/// `XOR EBX,EBX` at `0x00668CF3`, `PUSH EBX` at `0x00668D0A`), so `GAWALL` has
/// to be the third entry to land on the id `IsWallConnectableInDirection`
/// 0x00480510 accepts.
fn wall_pursuit_rules() -> RuleSet {
    let ini_str: &str = "\
[OverlayTypes]\n1=GASAND\n2=CYCL\n3=GAWALL\n\n\
[GASAND]\nWall=yes\n\n[CYCL]\n\n[GAWALL]\nWall=yes\n\n\
[VehicleTypes]\n0=MTNK\n1=HTNK\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[HTNK]\nStrength=400\nArmor=heavy\nSpeed=5\nPrimary=105mm\n\n\
[105mm]\nDamage=65\nROF=50\nRange=6\nProjectile=CANNON\nWarhead=AP\n\n\
[CANNON]\nSubjectToWalls=yes\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n";
    let ini: IniFile = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("wall_pursuit_rules should parse")
}

fn wall_pursuit_registry() -> crate::map::overlay_types::OverlayTypeRegistry {
    let ini_str: &str = "\
[OverlayTypes]\n1=GASAND\n2=CYCL\n3=GAWALL\n\n\
[GASAND]\nWall=yes\n\n[CYCL]\n\n[GAWALL]\nWall=yes\n";
    crate::map::overlay_types::OverlayTypeRegistry::from_ini(&IniFile::from_str(ini_str), None)
}

const WALL_TEST_GRID: u16 = 64;
/// `GAWALL`'s declaration index in the fixture's `[OverlayTypes]`.
const WALL_TEST_GAWALL_ID: u8 = 2;

fn wall_test_cell(rx: u16, ry: u16) -> crate::map::resolved_terrain::ResolvedTerrainCell {
    crate::map::resolved_terrain::ResolvedTerrainCell {
        rx,
        ry,
        source_tile_index: 0,
        source_sub_tile: 0,
        final_tile_index: 0,
        final_sub_tile: 0,
        is_wood_bridge_repair_tile: false,
        level: 0,
        filled_clear: true,
        tileset_index: Some(0),
        land_type: 0,
        yr_cell_land_type: 0,
        slope_type: 0,
        template_height: 0,
        render_offset_x: 0,
        render_offset_y: 0,
        terrain_class: Default::default(),
        speed_costs: Default::default(),
        is_water: false,
        is_cliff_like: false,
        is_rough: false,
        is_road: false,
        height_in_pixels: 0,
        variant: 0,
        has_ramp: false,
        canonical_ramp: None,
        ground_walk_blocked: false,
        terrain_object_blocks: false,
        terrain_object_occupation: None,
        overlay_blocks: false,
        overlay_zone_type: None,
        outside_playfield: false,
        zone_type: 0,
        base_ground_walk_blocked: false,
        base_build_blocked: false,
        base_land_type: 0,
        base_yr_cell_land_type: 0,
        base_terrain_class: Default::default(),
        base_speed_costs: Default::default(),
        build_blocked: false,
        has_bridge_deck: false,
        bridge_walkable: false,
        bridge_transition: false,
        bridge_deck_level: 0,
        bridge_layer: None,
        bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
        tube_index: None,
        radar_left: [0; 3],
        radar_right: [0; 3],
        accepts_smudge: true,
        allows_tiberium: false,
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    }
}

/// Flat terrain plus an overlay plane carrying the listed walls, installed on
/// the sim so `tick_attack_pursuit_with_overlay_registry` can run the 3-D gate.
fn install_wall_map(sim: &mut Simulation, walls: &[(u16, u16)]) {
    let cells: Vec<crate::map::resolved_terrain::ResolvedTerrainCell> = (0..WALL_TEST_GRID)
        .flat_map(|ry| (0..WALL_TEST_GRID).map(move |rx| wall_test_cell(rx, ry)))
        .collect();
    sim.resolved_terrain = Some(
        crate::map::resolved_terrain::ResolvedTerrainGrid::from_cells(
            WALL_TEST_GRID,
            WALL_TEST_GRID,
            cells,
        ),
    );
    let mut overlays = crate::sim::overlay_grid::OverlayGrid::new(WALL_TEST_GRID, WALL_TEST_GRID);
    for &(rx, ry) in walls {
        overlays.cell_mut(rx, ry).overlay_id = Some(WALL_TEST_GAWALL_ID);
    }
    sim.overlay_grid = Some(overlays);
}

/// **The deadlock tripwire.** Grizzly at (2,5), Rhino at (6,5): four cells, so
/// the plain radius says "in range" against `Range=6`. A `GAWALL` sits at (4,5),
/// and `[CANNON]` is `SubjectToWalls=yes` while `[AP]` has no `Wall=`, so the
/// fire gate refuses the shot at `0x006F7642`.
///
/// If pursuit judged range with the 2-D twin it would produce no pursuit cell,
/// the fire gate would refuse, and the tank would stand still under a live
/// attack order forever. Native never gets there: its approach routine measures
/// with the same `InRange` that runs the walk, so it keeps repositioning.
#[test]
fn a_wall_on_the_line_keeps_pursuit_closing_instead_of_freezing() {
    let mut grizzly = make_unit(1, "MTNK", "Americans", 2, 5, 300);
    grizzly.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 6, 5, 400);
    let (mut sim, grid) = make_sim(vec![grizzly, rhino]);
    install_wall_map(&mut sim, &[(4, 5)]);
    let rules = wall_pursuit_rules();
    let registry = wall_pursuit_registry();

    sim.tick_attack_pursuit_with_overlay_registry(
        &rules,
        Some(&grid),
        Some(&registry),
        &std::collections::BTreeSet::new(),
    );

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_some(),
        "the order survives — pursuit never drops a target it is still closing on"
    );
    assert!(
        entity.movement_target.is_some(),
        "a shot the fire gate refuses must keep pursuit moving, not freeze the unit"
    );
}

/// The control: the same fixture with the wall removed still halts, so the
/// tripwire above is pinning the walk and not merely "pursuit always moves".
#[test]
fn without_the_wall_the_same_shot_halts_pursuit() {
    let mut grizzly = make_unit(1, "MTNK", "Americans", 2, 5, 300);
    grizzly.attack_target = Some(AttackTarget::new(2));
    grizzly.movement_target = Some(crate::sim::components::MovementTarget::default());
    let rhino = make_unit(2, "HTNK", "Soviet", 6, 5, 400);
    let (mut sim, grid) = make_sim(vec![grizzly, rhino]);
    install_wall_map(&mut sim, &[]);
    let rules = wall_pursuit_rules();
    let registry = wall_pursuit_registry();

    sim.tick_attack_pursuit_with_overlay_registry(
        &rules,
        Some(&grid),
        Some(&registry),
        &std::collections::BTreeSet::new(),
    );

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.movement_target.is_none(),
        "with a clear line at four cells the shot is legal, so pursuit halts to fire"
    );
}

/// A `MinimumRange` refusal must NOT produce a pursuit cell.
///
/// Native's approach search 0x004D5690 scans candidate coordinates and takes
/// one where `InRange` holds, so a V3 that has been closed on backs off. VERA's
/// pursuit has exactly one candidate — the target's own cell — which for a
/// too-close refusal is the worst cell in the set. Feeding the fire gate's full
/// verdict into pursuit without this arm would send the V3 driving AT the tank
/// it cannot shell, which is a worse symptom than the hold it replaces.
///
/// `MinimumRange=` is ordinary stock data: `V3Launcher` 5, `DredLauncher` and
/// `CruiseLauncher` 8, `MagneticBeam` 3, `HowitzerGun` 2, the
/// `MissileLauncher`/`HoverMissile` family 1.
#[test]
fn inside_minimum_range_pursuit_holds_instead_of_closing() {
    let ini_str: &str = "\
[VehicleTypes]\n0=MTNK\n1=HTNK\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=LOBBER\n\n\
[HTNK]\nStrength=400\nArmor=heavy\nSpeed=5\nPrimary=LOBBER\n\n\
[LOBBER]\nDamage=200\nROF=150\nRange=20\nMinimumRange=5\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n";
    let rules = RuleSet::from_ini(&IniFile::from_str(ini_str)).expect("min-range rules parse");

    // Four cells apart: inside MinimumRange=5, well inside Range=20.
    let mut lobber = make_unit(1, "MTNK", "Americans", 2, 5, 300);
    lobber.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 6, 5, 400);
    let (mut sim, grid) = make_sim(vec![lobber, rhino]);
    install_wall_map(&mut sim, &[]);

    sim.tick_attack_pursuit_with_overlay_registry(
        &rules,
        Some(&grid),
        None,
        &std::collections::BTreeSet::new(),
    );

    let entity = sim.substrate.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_some(),
        "the order survives a too-close refusal"
    );
    assert!(
        entity.movement_target.is_none(),
        "inside MinimumRange pursuit must hold, never drive at the target"
    );
}
