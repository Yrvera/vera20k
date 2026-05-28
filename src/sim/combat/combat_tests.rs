//! Tests for the combat system — weapon firing, damage, and entity death.
//!
//! Extracted from combat.rs to keep it under the 400-line limit.

use std::collections::BTreeMap;

use super::*;
use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::animation::{Animation, SequenceKind};
use crate::sim::components::Health;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::{InternedId, test_intern, test_interner};
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::power_system::PowerState;
use crate::sim::vision::FogState;

/// Build a minimal RuleSet for combat testing.
fn test_rules() -> RuleSet {
    let ini_str: &str = "\
[InfantryTypes]\n0=E1\n\n\
[VehicleTypes]\n0=MTNK\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n0=GAPOWR\n\n\
[E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[GAPOWR]\nStrength=750\nArmor=wood\n\n\
[M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
[SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
[AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n";
    let ini: IniFile = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("test rules should parse")
}

fn building_damage_state_aoe_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "\
[InfantryTypes]\n\n\
[VehicleTypes]\n0=MTNK\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n0=GAPOWR\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[GAPOWR]\nStrength=100\nArmor=wood\n\n\
[105mm]\nDamage=20\nROF=50\nRange=6\nWarhead=AP\n\n\
[AP]\nCellSpread=1\nPercentAtMax=100\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n\n\
[AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n",
    );
    RuleSet::from_ini(&ini).expect("building damage state AoE rules should parse")
}

fn infantry_fire_frame_rules() -> RuleSet {
    let rules_ini: IniFile = IniFile::from_str(
        "\
[InfantryTypes]\n0=E1\n1=E2\n\n\
[VehicleTypes]\n0=MTNK\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[E1]\nStrength=125\nArmor=flak\nSpeed=4\nImage=GI\nPrimary=M60\nSecondary=Para\nDeployFire=yes\n\n\
[E2]\nStrength=125\nArmor=flak\nSpeed=4\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\n\n\
[M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\nReport=GIAttack\nOccupantAnim=UCFLASH\n\n\
[Para]\nDamage=40\nROF=15\nRange=5\nWarhead=AP\nReport=GIAttackDeployed\nOccupantAnim=UCFLASH\n\n\
[SA]\nVerses=100%,100%,100%,90%,70%,0%,100%,25%,25%,0%,0%\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
    );
    let mut rules = RuleSet::from_ini(&rules_ini).expect("infantry rules should parse");
    let art_ini = IniFile::from_str(
        "[GI]\nCrawls=yes\nFireUp=2\nFireProne=3\nSecondaryFire=4\nSecondaryProne=5\n",
    );
    let art = crate::rules::art_data::ArtRegistry::from_ini(&art_ini);
    rules.merge_art_data(&art);
    rules
}

fn guardian_gi_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "\
[InfantryTypes]\n0=GGI\n1=E2\n2=ROCK\n\n\
[VehicleTypes]\n0=HTNK\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[General]\nMissileROTVar=.25\n\n\
[GGI]\nStrength=100\nArmor=none\nSpeed=4\nPrimary=M60\nSecondary=MissileLauncher\nDeployFire=yes\n\n\
[E2]\nStrength=125\nArmor=none\nSpeed=4\n\n\
[ROCK]\nStrength=125\nArmor=none\nSpeed=8\nConsideredAircraft=yes\n\n\
[HTNK]\nStrength=400\nArmor=heavy\nSpeed=5\n\n\
[M60]\nDamage=15\nROF=20\nRange=4\nWarhead=SA\nReport=GGIAttack\n\n\
[MissileLauncher]\nDamage=40\nROF=40\nRange=8\nBurst=1\nProjectile=AAHeatSeeker2\nSpeed=30\nWarhead=GUARDWH\nReport=GuardianGIDeployedAttack\nMinimumRange=1\n\n\
[AAHeatSeeker2]\nArm=2\nShadow=no\nProximity=no\nRanged=yes\nAA=yes\nAG=yes\nImage=DRAGON\nROT=60\nSubjectToCliffs=no\nSubjectToElevation=no\nSubjectToWalls=no\n\n\
[SA]\nVerses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%\n\n\
[GUARDWH]\nVerses=20%,20%,20%,100%,50%,100%,10%,10%,10%,100%,100%\n",
    );
    RuleSet::from_ini(&ini).expect("guardian GI rules should parse")
}

fn make_entity(id: u64, type_ref: &str, rx: u16, ry: u16, hp: u16) -> GameEntity {
    let mut e = GameEntity::test_default(id, type_ref, "Test", rx, ry);
    e.health = Health {
        current: hp,
        max: hp,
    };
    e
}

fn make_entity_owned(
    id: u64,
    type_ref: &str,
    rx: u16,
    ry: u16,
    hp: u16,
    owner: &str,
) -> GameEntity {
    let mut e = GameEntity::test_default(id, type_ref, owner, rx, ry);
    e.health = Health {
        current: hp,
        max: hp,
    };
    e
}

fn make_infantry_entity(id: u64, type_ref: &str, rx: u16, ry: u16, hp: u16) -> GameEntity {
    let mut e = make_entity(id, type_ref, rx, ry, hp);
    e.category = EntityCategory::Infantry;
    e.is_voxel = false;
    e.animation = Some(Animation::new(SequenceKind::Stand));
    e.infantry = Some(crate::sim::game_entity::InfantryRuntime::new());
    e
}

fn make_structure_entity(
    id: u64,
    type_ref: &str,
    rx: u16,
    ry: u16,
    current: u16,
    max: u16,
) -> GameEntity {
    let mut entity = make_entity(id, type_ref, rx, ry, max);
    entity.category = EntityCategory::Structure;
    entity.is_voxel = false;
    entity.health = Health { current, max };
    entity
}

fn considered_aircraft_weapon_rules() -> RuleSet {
    let ini_str: &str = "\
[InfantryTypes]
0=ROCK
1=E1
[VehicleTypes]
0=IFV
[AircraftTypes]
[BuildingTypes]

[IFV]
Strength=200
Armor=light
Speed=8
Primary=GroundGun
Secondary=AirGun

[ROCK]
Strength=125
Armor=none
Speed=8
ConsideredAircraft=yes

[E1]
Strength=125
Armor=none
Speed=4

[GroundGun]
Damage=10
ROF=20
Range=7
Projectile=GroundProj
Warhead=TestWH

[AirGun]
Damage=10
ROF=20
Range=7
Projectile=AirProj
Warhead=TestWH

[GroundProj]
AG=yes
AA=no

[AirProj]
AG=no
AA=yes

[TestWH]
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%
";
    let ini: IniFile = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("considered-aircraft combat rules should parse")
}

fn set_anim_frame(store: &mut EntityStore, id: u64, frame: u16) {
    store
        .get_mut(id)
        .unwrap()
        .animation
        .as_mut()
        .unwrap()
        .frame_index = frame;
}

#[test]
fn test_armor_index_lookup() {
    assert_eq!(armor_index("none"), 0);
    assert_eq!(armor_index("flak"), 1);
    assert_eq!(armor_index("heavy"), 5);
    assert_eq!(armor_index("wood"), 6);
    assert_eq!(armor_index("concrete"), 8);
    assert_eq!(armor_index("unknown"), 0);
}

#[test]
fn cell_center_coords_remains_ground_z_for_cell_targets() {
    let (rx, ry, sub_x, sub_y) = cell_center_coords(7, 9);
    assert_eq!((rx, ry), (7, 9));
    assert_eq!(sub_x.to_num::<i32>(), 128);
    assert_eq!(sub_y.to_num::<i32>(), 128);

    let entities = EntityStore::new();
    assert_eq!(
        attack_impact_z(TargetKind::Cell(7, 9), &entities),
        0,
        "force-fire cell targets must not inherit bridge/elevation Z from generic center coords"
    );
}

#[test]
fn considered_aircraft_infantry_is_air_for_projectile_legality() {
    let rules = considered_aircraft_weapon_rules();
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let attacker = sim
        .spawn_object("IFV", "Americans", 5, 5, 0, &rules, &heights)
        .expect("IFV should spawn");
    let target = sim
        .spawn_object("ROCK", "Soviet", 8, 5, 0, &rules, &heights)
        .expect("Rocketeer should spawn");

    let target_entity = sim.entities.get(target).expect("target should exist");
    assert_eq!(target_entity.category, EntityCategory::Infantry);
    assert!(
        rules
            .object(sim.interner.resolve(target_entity.type_ref))
            .is_some_and(|obj| obj.considered_aircraft)
    );
    assert_eq!(
        combat_target_category(target_entity, &rules, &sim.interner),
        EntityCategory::Aircraft
    );

    issue_attack_command(&mut sim.entities, attacker, target, None, &sim.interner);
    let result = tick_combat(
        &mut sim.entities,
        &mut sim.occupancy,
        &rules,
        &mut sim.interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert_eq!(result.fire_events.len(), 1);
    assert_eq!(
        sim.interner.resolve(result.fire_events[0].weapon_id),
        "AirGun"
    );
    assert_eq!(result.fire_events[0].weapon_slot, WeaponSlot::Secondary);
}

#[test]
fn ordinary_infantry_remains_ground_for_projectile_legality() {
    let rules = considered_aircraft_weapon_rules();
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let attacker = sim
        .spawn_object("IFV", "Americans", 5, 5, 0, &rules, &heights)
        .expect("IFV should spawn");
    let target = sim
        .spawn_object("E1", "Soviet", 8, 5, 0, &rules, &heights)
        .expect("ordinary infantry should spawn");

    let target_entity = sim.entities.get(target).expect("target should exist");
    assert_eq!(target_entity.category, EntityCategory::Infantry);
    assert_eq!(
        combat_target_category(target_entity, &rules, &sim.interner),
        EntityCategory::Infantry
    );

    issue_attack_command(&mut sim.entities, attacker, target, None, &sim.interner);
    let result = tick_combat(
        &mut sim.entities,
        &mut sim.occupancy,
        &rules,
        &mut sim.interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert_eq!(result.fire_events.len(), 1);
    assert_eq!(
        sim.interner.resolve(result.fire_events[0].weapon_id),
        "GroundGun"
    );
    assert_eq!(result.fire_events[0].weapon_slot, WeaponSlot::Primary);
}

#[test]
fn test_issue_attack_command() {
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));

    let result: bool = issue_attack_command(&mut store, 1, 2, None, &test_interner());
    assert!(result, "Should succeed for valid entities");

    let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
    assert!(matches!(
        attack.target,
        crate::sim::combat::TargetKind::Entity(2)
    ));
    assert_eq!(attack.cooldown_ticks, 0, "Initial cooldown should be 0");
}

#[test]
fn test_attack_nonexistent_target() {
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));

    let result: bool = issue_attack_command(&mut store, 1, 99, None, &test_interner());
    assert!(!result, "Should fail for nonexistent target");
}

#[test]
fn test_tick_combat_applies_damage() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();

    // MTNK attacks another MTNK (heavy armor).
    // 105mm: damage=65, warhead=AP, AP verses[heavy(5)] = 75%.
    // Integer math: 65 * 75 / 100 = 48.
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    let target_health = store.get(2).expect("target alive").health.current;
    assert_eq!(
        target_health,
        300 - 48,
        "Should take 48 damage (65 * 75 / 100)"
    );
}

#[test]
fn combat_damage_crossing_condition_yellow_sets_building_damage_state() {
    let rules = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_structure_entity(2, "GAPOWR", 8, 5, 60, 100));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert!(
        store
            .get(2)
            .expect("building survives")
            .building_damage_state_active
    );
}

#[test]
fn combat_damage_above_condition_yellow_leaves_building_damage_state_false() {
    let rules = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_structure_entity(2, "GAPOWR", 8, 5, 100, 100));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert!(
        !store
            .get(2)
            .expect("building survives")
            .building_damage_state_active
    );
}

#[test]
fn aoe_damage_crossing_condition_yellow_sets_building_damage_state() {
    let rules = building_damage_state_aoe_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_structure_entity(2, "GAPOWR", 8, 5, 60, 100));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert!(
        store
            .get(2)
            .expect("building survives")
            .building_damage_state_active
    );
}

#[test]
fn combat_damage_landed_applies_infantry_fear() {
    let rules = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_infantry_entity(2, "E1", 8, 5, 125));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert_eq!(
        store.get(2).unwrap().infantry.as_ref().unwrap().fear_level,
        100
    );
}

#[test]
fn ic_target_takes_zero_damage() {
    use crate::sim::superweapon::invulnerability::{InvulnKind, InvulnerabilityState};
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_infantry_entity(2, "E1", 8, 5, 125));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);
    // Apply IronCurtain invulnerability to the target.
    if let Some(target) = store.get_mut(2) {
        target.invulnerability = Some(InvulnerabilityState {
            start_frame: 0,
            duration_frames: 1000,
            kind: InvulnKind::IronCurtain,
        });
    }
    let initial_hp = store.get(2).expect("target alive").health.current;
    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        10u64,
        100,
        0u32,
    );
    assert_eq!(
        store.get(2).expect("target alive").health.current,
        initial_hp,
        "IC-invulnerable target must take zero damage"
    );
    assert!(
        store
            .get(2)
            .unwrap()
            .infantry
            .as_ref()
            .is_some_and(|inf| inf.fear_level == 0),
        "invulnerable targets should not gain fear because no damage lands"
    );
}

#[test]
fn test_tick_combat_only_emits_bridge_damage_for_wall_warheads() {
    let mut store = EntityStore::new();
    let rules_without_wall = test_rules();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);
    let result = tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules_without_wall,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );
    assert!(
        result.bridge_damage_events.is_empty(),
        "non-wall warheads must not emit bridge damage"
    );
    assert!(
        result.wall_damage_events.is_empty(),
        "non-wall warheads must not emit wall damage"
    );

    let mut bridge_rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=MTNK\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
         [AP]\nWall=yes\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
    ))
    .expect("bridge combat rules should parse");
    // Combat reads IonCannonWarhead at the bridge-damage emit boundary; tests
    // that drive tick_combat must resolve before invoking it.
    bridge_rules.resolve_bridge_warheads(&mut interner);
    let mut wall_store = EntityStore::new();
    wall_store.insert(make_entity(3, "MTNK", 5, 5, 300));
    wall_store.insert(make_entity(4, "MTNK", 8, 5, 300));
    issue_attack_command(&mut wall_store, 3, 4, None, &interner);
    let wall_result = tick_combat_with_fog(
        &mut wall_store,
        &mut OccupancyGrid::new(),
        &bridge_rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );
    assert_eq!(
        wall_result.bridge_damage_events,
        vec![BridgeDamageEvent {
            rx: 8,
            ry: 5,
            damage: 65,
            warhead_ref: interner
                .get("AP")
                .expect("AP warhead interned by tick_combat"),
            is_ion_cannon: false,
            impact_z: 0,
        }]
    );
    // Without an overlay grid+registry, the discriminator can't identify a wall
    // cell — events fall through to bridge_damage_events. wall_damage_events
    // requires both a grid lookup and Wall=yes in the registry.
    assert!(wall_result.wall_damage_events.is_empty());
}

#[test]
fn test_tick_combat_respects_cooldown() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    // First shot fires immediately (cooldown=0).
    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );
    let h1: u16 = store.get(2).unwrap().health.current;

    // Next tick should not fire again immediately.
    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );
    let h2: u16 = store.get(2).unwrap().health.current;
    assert_eq!(h1, h2, "Should not fire during cooldown");

    // After enough ticks, should fire again.
    for _ in 0..40 {
        tick_combat(
            &mut store,
            &mut OccupancyGrid::new(),
            &rules,
            &mut interner,
            &mut BTreeMap::new(),
            0u64,
            100,
            0u32,
        );
    }
    let h3: u16 = store.get(2).unwrap().health.current;
    assert!(h3 < h2, "Should fire after cooldown expires");
}

#[test]
fn test_tick_combat_kills_target() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    let mut attacker = make_entity(1, "MTNK", 5, 5, 300);
    let mut target = make_entity(2, "MTNK", 8, 5, 10);
    attacker.mark_live_contact_with(2);
    target.mark_live_contact_with(1);
    store.insert(attacker);
    store.insert(target);
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    assert!(store.get(2).is_none(), "Dead entity should be removed");
    assert!(
        store.get(1).unwrap().attack_target.is_none(),
        "AttackTarget removed after target dies"
    );
    assert!(
        !store.get(1).unwrap().has_live_contact_with(2),
        "immediate combat removal should clear stale radio contact"
    );
}

#[test]
fn test_tick_combat_out_of_range() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    // 105mm range = 6 cells. Target at distance 10.
    store.insert(make_entity(1, "MTNK", 0, 0, 300));
    store.insert(make_entity(2, "MTNK", 10, 0, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    let target_health = store.get(2).unwrap().health.current;
    assert_eq!(
        target_health, 300,
        "Out-of-range target should not take damage"
    );
    // Range failure preserves attack_target; pursuit (run from advance_tick,
    // not from tick_combat in isolation) walks the unit into range.
    assert!(
        store.get(1).unwrap().attack_target.is_some(),
        "AttackTarget preserved when out of range — pursuit closes the gap"
    );
}

#[test]
fn undeployed_guardian_gi_vs_infantry_uses_m60() {
    let rules = guardian_gi_rules();
    let mut store = EntityStore::new();
    store.insert(make_infantry_entity(1, "GGI", 0, 0, 100));
    store.insert(make_infantry_entity(2, "E2", 3, 0, 125));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(interner.resolve(ev.weapon_id), "M60");
    assert_eq!(ev.weapon_slot, WeaponSlot::Primary);
    assert_eq!(store.get(2).unwrap().health.current, 110);
}

#[test]
fn deployed_guardian_gi_vs_rhino_at_six_cells_uses_missilelauncher() {
    let rules = guardian_gi_rules();
    let mut store = EntityStore::new();
    let mut ggi = make_infantry_entity(1, "GGI", 0, 0, 100);
    ggi.deploy_state = Some(crate::sim::deploy::DeployPhase::Deployed);
    ggi.animation = Some(Animation::new(SequenceKind::Deployed));
    store.insert(ggi);
    store.insert(make_entity(2, "HTNK", 6, 0, 400));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(interner.resolve(ev.weapon_id), "MissileLauncher");
    assert_eq!(ev.weapon_slot, WeaponSlot::Secondary);
    assert_eq!(store.get(2).unwrap().health.current, 360);
}

#[test]
fn deployed_guardian_gi_vs_rocketeer_uses_missilelauncher() {
    let rules = guardian_gi_rules();
    let mut store = EntityStore::new();
    let mut ggi = make_infantry_entity(1, "GGI", 0, 0, 100);
    ggi.deploy_state = Some(crate::sim::deploy::DeployPhase::Deployed);
    ggi.animation = Some(Animation::new(SequenceKind::Deployed));
    store.insert(ggi);
    store.insert(make_infantry_entity(2, "ROCK", 6, 0, 125));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(interner.resolve(ev.weapon_id), "MissileLauncher");
    assert_eq!(ev.weapon_slot, WeaponSlot::Secondary);
}

#[test]
fn test_infantry_vs_heavy_armor() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    // E1 (M60) attacks MTNK (heavy armor).
    // M60: damage=25, warhead=SA, SA verses[heavy(5)] = 25%.
    // Integer math: 25 * 25 / 100 = 6.
    store.insert(make_entity(1, "E1", 5, 5, 125));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    let h: u16 = store.get(2).unwrap().health.current;
    assert_eq!(
        h,
        300 - 6,
        "Infantry vs heavy armor should do 6 damage (25 * 25 / 100)"
    );
}

#[test]
fn infantry_standing_fire_waits_for_fire_frame() {
    let rules = infantry_fire_frame_rules();
    let mut store = EntityStore::new();
    store.insert(make_infantry_entity(1, "E1", 5, 5, 125));
    store.insert(make_infantry_entity(2, "E2", 8, 5, 125));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );

    assert_eq!(store.get(2).unwrap().health.current, 125);
    assert!(result.fire_events.is_empty());
    let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
    assert_eq!(
        attack.pending_infantry_fire.unwrap(),
        PendingInfantryFire {
            sequence: SequenceKind::Attack,
            fire_frame: 2
        }
    );
    assert_eq!(
        store.get(1).unwrap().animation.as_ref().unwrap().sequence,
        SequenceKind::Attack
    );

    set_anim_frame(&mut store, 1, 1);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        1,
        100,
        0,
    );
    assert_eq!(store.get(2).unwrap().health.current, 125);
    assert!(result.fire_events.is_empty());

    set_anim_frame(&mut store, 1, 2);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        2,
        100,
        0,
    );
    assert_eq!(store.get(2).unwrap().health.current, 100);
    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(interner.resolve(ev.weapon_id), "M60");
    assert_eq!(ev.weapon_slot, WeaponSlot::Primary);
    assert_eq!(
        ev.report_sound_id.map(|id| interner.resolve(id)),
        Some("GIAttack")
    );
    assert_eq!(ev.garrison_muzzle_index, None);
    assert_eq!(ev.occupant_anim, None);
    assert!(
        store
            .get(1)
            .unwrap()
            .attack_target
            .as_ref()
            .unwrap()
            .pending_infantry_fire
            .is_none()
    );
}

#[test]
fn prone_infantry_uses_prone_fire_sequence_and_frame() {
    let rules = infantry_fire_frame_rules();
    let mut store = EntityStore::new();
    let mut attacker = make_infantry_entity(1, "E1", 5, 5, 125);
    attacker.infantry.as_mut().unwrap().is_prone = true;
    attacker.animation = Some(Animation::new(SequenceKind::Prone));
    store.insert(attacker);
    store.insert(make_infantry_entity(2, "E2", 8, 5, 125));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );
    let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
    assert!(result.fire_events.is_empty());
    assert_eq!(
        attack.pending_infantry_fire.unwrap(),
        PendingInfantryFire {
            sequence: SequenceKind::FireProne,
            fire_frame: 3
        }
    );
    assert_eq!(store.get(2).unwrap().health.current, 125);

    set_anim_frame(&mut store, 1, 2);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        1,
        100,
        0,
    );
    assert_eq!(store.get(2).unwrap().health.current, 125);
    assert!(result.fire_events.is_empty());

    set_anim_frame(&mut store, 1, 3);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        2,
        100,
        0,
    );
    assert_eq!(store.get(2).unwrap().health.current, 100);
    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(interner.resolve(ev.weapon_id), "M60");
    assert_eq!(ev.weapon_slot, WeaponSlot::Primary);
    assert_eq!(
        ev.report_sound_id.map(|id| interner.resolve(id)),
        Some("GIAttack")
    );
}

#[test]
fn deployed_gi_uses_deployed_fire_visual_with_deploy_fire_weapon() {
    let rules = infantry_fire_frame_rules();
    let mut store = EntityStore::new();
    let mut attacker = make_infantry_entity(1, "E1", 5, 5, 125);
    attacker.deploy_state = Some(crate::sim::deploy::DeployPhase::Deployed);
    attacker.animation = Some(Animation::new(SequenceKind::Deployed));
    store.insert(attacker);
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );
    let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
    assert!(result.fire_events.is_empty());
    assert_eq!(
        attack.pending_infantry_fire.unwrap(),
        PendingInfantryFire {
            sequence: SequenceKind::DeployedFire,
            fire_frame: 5
        }
    );
    assert_eq!(store.get(2).unwrap().health.current, 300);

    set_anim_frame(&mut store, 1, 5);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        1,
        100,
        0,
    );
    assert_eq!(
        store.get(2).unwrap().health.current,
        260,
        "deployed-fire should use the DeployFireWeapon secondary slot"
    );
    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(interner.resolve(ev.weapon_id), "Para");
    assert_eq!(ev.weapon_slot, WeaponSlot::Secondary);
    assert_eq!(
        ev.report_sound_id.map(|id| interner.resolve(id)),
        Some("GIAttackDeployed")
    );
}

#[test]
fn garrison_fire_keeps_occupant_anim_and_sound_path() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "\
[InfantryTypes]\n0=E1\n1=E2\n\n\
[VehicleTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n0=CAGAS\n\n\
[CAGAS]\nStrength=800\nArmor=wood\nCanBeOccupied=yes\nCanOccupyFire=yes\nMaxNumberOccupants=5\n\n\
[E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\nOccupyWeapon=M60\n\n\
[E2]\nStrength=125\nArmor=flak\nSpeed=4\n\n\
[M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\nReport=GIAttack\nOccupantAnim=UCFLASH\n\n\
[SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n",
    ))
    .expect("garrison rules parse");
    let mut store = EntityStore::new();
    let mut building = make_entity(10, "CAGAS", 5, 5, 800);
    building.category = EntityCategory::Structure;
    let mut cargo = crate::sim::passenger::PassengerCargo::new(5, 1);
    assert!(cargo.board(1, 1));
    building.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
    store.insert(building);
    let mut occupant = make_infantry_entity(1, "E1", 5, 5, 125);
    occupant.passenger_role = crate::sim::passenger::PassengerRole::Inside { transport_id: 10 };
    store.insert(occupant);
    store.insert(make_infantry_entity(2, "E2", 8, 5, 125));

    let mut interner = test_interner();
    issue_attack_command(&mut store, 10, 2, None, &interner);
    let mut sounds = Vec::new();
    let result = tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        Some(&mut sounds),
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0,
        100,
        0,
    );

    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(ev.garrison_muzzle_index, Some(0));
    assert_eq!(
        ev.occupant_anim.map(|id| interner.resolve(id)),
        Some("UCFLASH")
    );
    assert_eq!(
        ev.report_sound_id.map(|id| interner.resolve(id)),
        Some("GIAttack")
    );
    assert!(sounds.is_empty());
}

#[test]
fn delayed_infantry_fire_cancels_when_target_dies_before_fire_frame() {
    let rules = infantry_fire_frame_rules();
    let mut store = EntityStore::new();
    store.insert(make_infantry_entity(1, "E1", 5, 5, 125));
    store.insert(make_infantry_entity(2, "E2", 8, 5, 125));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
    );
    store.get_mut(2).unwrap().health.current = 0;
    set_anim_frame(&mut store, 1, 2);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        1,
        100,
        0,
    );

    assert_eq!(store.get(2).unwrap().health.current, 0);
    assert!(result.fire_events.is_empty());
    assert!(
        store.get(1).unwrap().attack_target.is_none(),
        "dead target should cancel delayed shot instead of spawning stale damage"
    );
}

#[test]
fn test_prone_infantry_takes_scaled_direct_damage() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n1=E2\n\n\
         [VehicleTypes]\n0=MTNK\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [E2]\nStrength=125\nArmor=flak\nSpeed=4\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=100\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\nProneDamage=50%\n",
    ))
    .expect("prone combat rules should parse");

    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    let mut target = make_infantry_entity(2, "E2", 8, 5, 125);
    target.infantry.as_mut().unwrap().is_prone = true;
    store.insert(target);

    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    let target_health = store.get(2).expect("target alive").health.current;
    assert_eq!(
        target_health, 75,
        "100 damage with ProneDamage=50% should deal 50"
    );
}

#[test]
fn test_prone_infantry_takes_scaled_aoe_damage() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n1=E2\n\n\
         [VehicleTypes]\n0=MTNK\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [E2]\nStrength=125\nArmor=flak\nSpeed=4\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=100\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nCellSpread=1\nPercentAtMax=1\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\nProneDamage=50%\n",
    ))
    .expect("prone aoe combat rules should parse");

    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    let mut target = make_infantry_entity(2, "E2", 8, 5, 125);
    target.infantry.as_mut().unwrap().is_prone = true;
    store.insert(target);

    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    let target_health = store.get(2).expect("target alive").health.current;
    assert_eq!(
        target_health, 75,
        "AoE center hit should also respect ProneDamage=50%"
    );
}

#[test]
fn test_cell_distance() {
    assert!((cell_distance(0, 0, 3, 4) - 5.0).abs() < 0.01);
    assert!((cell_distance(5, 5, 5, 5) - 0.0).abs() < f32::EPSILON);
    assert!((cell_distance(0, 0, 1, 0) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_tick_combat_visibility_blocks_fire() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity_owned(1, "MTNK", 5, 5, 300, "Americans"));
    store.insert(make_entity_owned(2, "MTNK", 8, 5, 300, "Soviet"));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let fog = FogState::default();
    tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        Some(&fog),
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );

    let target_health = store.get(2).expect("target alive").health.current;
    assert_eq!(target_health, 300, "Hidden target should not be damaged");
    assert!(
        store.get(1).unwrap().attack_target.is_none(),
        "AttackTarget removed when target is not visible and no replacement exists"
    );
}

#[test]
fn test_tick_combat_retargets_by_distance_then_stable_id() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity_owned(10, "MTNK", 5, 5, 300, "Americans"));
    store.insert(make_entity_owned(99, "MTNK", 6, 5, 0, "Soviet")); // dead
    store.insert(make_entity_owned(20, "MTNK", 7, 5, 300, "Soviet"));
    store.insert(make_entity_owned(3, "MTNK", 7, 5, 300, "Soviet"));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 10, 99, None, &interner);

    let mut fog = FogState::default();
    fog.mark_visible_for_owner(test_intern("Americans"), 7, 5);
    tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        Some(&fog),
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );

    let attack = store
        .get(10)
        .unwrap()
        .attack_target
        .as_ref()
        .expect("attacker should retarget");
    assert!(
        matches!(attack.target, crate::sim::combat::TargetKind::Entity(3)),
        "Tie should resolve to lower stable entity id"
    );
    assert!(
        !matches!(attack.target, crate::sim::combat::TargetKind::Entity(20)),
        "Should not target enemy_a (sid=20)"
    );
}

#[test]
fn test_tick_combat_retargets_prefers_threat_class_when_distance_equal() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity_owned(10, "MTNK", 5, 5, 300, "Americans"));
    store.insert(make_entity_owned(99, "MTNK", 6, 5, 0, "Soviet")); // dead
    let mut building = make_entity_owned(1, "GAPOWR", 7, 5, 750, "Soviet");
    building.category = crate::map::entities::EntityCategory::Structure;
    store.insert(building);
    store.insert(make_entity_owned(200, "MTNK", 7, 5, 300, "Soviet"));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 10, 99, None, &interner);

    let mut fog = FogState::default();
    fog.mark_visible_for_owner(test_intern("Americans"), 7, 5);
    tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        Some(&fog),
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );

    let attack = store
        .get(10)
        .unwrap()
        .attack_target
        .as_ref()
        .expect("attacker should retarget");
    assert!(
        matches!(attack.target, crate::sim::combat::TargetKind::Entity(200)),
        "Combat unit should rank above building at equal distance"
    );
    assert!(
        !matches!(attack.target, crate::sim::combat::TargetKind::Entity(1)),
        "Should not target building (sid=1)"
    );
}

// --- Ore destruction integration tests ---

/// Build a RuleSet with a CellSpread=2 AoE weapon for ore destruction testing.
fn test_rules_with_spread() -> RuleSet {
    let ini_str: &str = "\
[InfantryTypes]\n\n\
[VehicleTypes]\n0=MTNK\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=120mm\n\n\
[120mm]\nDamage=120\nROF=50\nRange=6\nWarhead=HE\n\n\
[HE]\nCellSpread=2\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let ini = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("test rules should parse")
}

#[test]
fn test_weapon_fire_destroys_ore_in_spread() {
    let rules = test_rules_with_spread();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    // Place ore at the target cell and a neighbor within CellSpread=2.
    let mut resource_nodes = BTreeMap::new();
    // 6 density levels of ore at target (8,5): remaining = 6 * 120 = 720.
    resource_nodes.insert(
        (8, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 720,
        },
    );
    // 3 density levels at (9,5): remaining = 3 * 120 = 360.
    resource_nodes.insert(
        (9, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 360,
        },
    );

    tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut resource_nodes,
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );

    // Damage=120, ore_damage = 120/10 = 12 density levels.
    // Cell (8,5) had 6 levels — 12 >= 6, so fully removed.
    assert!(
        resource_nodes.get(&(8, 5)).is_none(),
        "target cell ore should be fully destroyed (12 >= 6)"
    );
    // Cell (9,5) had 3 levels — 12 >= 3, so fully removed.
    assert!(
        resource_nodes.get(&(9, 5)).is_none(),
        "neighbor cell ore should be fully destroyed (12 >= 3)"
    );
}

#[test]
fn test_direct_hit_weapon_destroys_center_ore() {
    let rules = test_rules(); // AP warhead has CellSpread=0.
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let mut resource_nodes = BTreeMap::new();
    resource_nodes.insert(
        (8, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 720,
        },
    );
    // Ore at adjacent cell (9,5) should NOT be affected (CellSpread=0 = center only).
    resource_nodes.insert(
        (9, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 720,
        },
    );

    tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut resource_nodes,
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );

    // 105mm damage=65, ore_damage = 65/10 = 6.
    // Cell (8,5) had 6 density levels — 6 >= 6 → fully removed.
    assert!(
        resource_nodes.get(&(8, 5)).is_none(),
        "center cell ore should be destroyed (6 >= 6)"
    );
    // Cell (9,5) should be untouched.
    assert_eq!(
        resource_nodes.get(&(9, 5)).unwrap().remaining,
        720,
        "adjacent cell should be untouched with CellSpread=0"
    );
}

#[test]
fn test_weak_weapon_partial_ore_reduction() {
    let rules = test_rules(); // M60 damage=25.
    let mut store = EntityStore::new();
    // E1 attacks MTNK — E1's primary is M60 (damage=25, SA warhead, CellSpread=0).
    store.insert(make_entity(1, "E1", 5, 5, 125));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let mut resource_nodes = BTreeMap::new();
    // 10 density levels of ore: remaining = 10 * 120 = 1200.
    resource_nodes.insert(
        (8, 5),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 1200,
        },
    );

    tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut resource_nodes,
        None,
        None,
        None,
        0u64,
        100,
        0u32,
    );

    // M60 damage=25, ore_damage = 25/10 = 2.
    // 10 density levels, remove 2 → 8 remaining → 8 * 120 = 960.
    assert_eq!(
        resource_nodes.get(&(8, 5)).unwrap().remaining,
        960,
        "should reduce by 2 density levels (25/10=2)"
    );
}

// ---- Wall damage integration tests ----------------------------------------

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::sim::overlay_grid::{OverlayGrid, WallDamageEvent};
use crate::sim::world::Simulation;

/// INI containing GAWALL as both a [BuildingTypes] entry (so it has an
/// ObjectType with Wall=yes) and an [OverlayTypes] entry (so the overlay
/// registry knows it as a wall overlay). Strength=400, DamageLevels=4 are
/// representative of the real GAWALL.
fn wall_test_ini() -> &'static str {
    "[InfantryTypes]\n\
     [VehicleTypes]\n\
     [AircraftTypes]\n\
     [BuildingTypes]\n0=GAWALL\n\
     [OverlayTypes]\n0=GASAND\n1=CYCL\n2=GAWALL\n\
     [GAWALL]\nStrength=400\nArmor=concrete\nWall=yes\n\
     [GASAND]\nWall=yes\nStrength=400\n\
     [CYCL]\nWall=yes\nStrength=400\n"
}

/// Build a Simulation with a 10x10 OverlayGrid and a GAWALL placed at (rx, ry)
/// with both an OverlayCell entry and a matching wall GameEntity. Returns the
/// sim, ruleset, and registry tied to the same INI.
fn build_minimal_sim_with_gawall(rx: u16, ry: u16) -> (Simulation, RuleSet, OverlayTypeRegistry) {
    let ini = IniFile::from_str(wall_test_ini());
    let rules = RuleSet::from_ini(&ini).expect("wall rules parse");
    let registry = OverlayTypeRegistry::from_ini(&ini, None);

    let mut sim = Simulation::new();
    let mut grid = OverlayGrid::new(10, 10);
    // Place GAWALL (overlay_id=2). Initial frame = 0 (isolated, stage 0).
    grid.place_overlay(rx, ry, 2, 0);
    sim.overlay_grid = Some(grid);

    // Spawn a wall GameEntity at the same cell. Intern the type_ref and owner
    // through sim.interner so later lookups via sim.interner.resolve() succeed.
    let owner_id = sim.interner.intern("Test");
    let type_id = sim.interner.intern("GAWALL");
    let mut entity = GameEntity::test_default(1, "GAWALL", "Test", rx, ry);
    entity.owner = owner_id;
    entity.type_ref = type_id;
    entity.health = Health {
        current: 400,
        max: 400,
    };
    sim.entities.insert(entity);
    sim.entities.rebuild_owner_index();

    (sim, rules, registry)
}

#[test]
fn wall_warhead_damages_and_destroys_wall_overlay() {
    let (mut sim, rules, registry) = build_minimal_sim_with_gawall(5, 5);

    let initial_wall_entities = sim
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            rules
                .object(sim.interner.resolve(e.type_ref))
                .is_some_and(|o| o.wall)
        })
        .count();
    assert_eq!(
        initial_wall_entities, 1,
        "fixture must place exactly one wall entity"
    );

    // Forced destruction (u16::MAX bypasses the probabilistic gate).
    let events = [WallDamageEvent {
        rx: 5,
        ry: 5,
        damage: u16::MAX,
    }];
    sim.apply_wall_damage_events(&events, &rules, &registry);

    // Overlay cleared.
    let grid = sim
        .overlay_grid
        .as_ref()
        .expect("grid should still be present");
    assert!(
        grid.cell(5, 5).overlay_id.is_none(),
        "overlay should be cleared"
    );

    // Wall entity removed.
    let remaining = sim
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            rules
                .object(sim.interner.resolve(e.type_ref))
                .is_some_and(|o| o.wall)
        })
        .count();
    assert_eq!(
        remaining, 0,
        "wall entity should be despawned after overlay destruction"
    );
}

/// Build a Simulation with a row of GAWALL at `(rx_range, ry)`. Each cell gets
/// both an OverlayCell entry and a matching wall GameEntity.
fn build_minimal_sim_with_gawall_row(
    ry: u16,
    rx_range: std::ops::Range<u16>,
) -> (Simulation, RuleSet, OverlayTypeRegistry) {
    let ini = IniFile::from_str(wall_test_ini());
    let rules = RuleSet::from_ini(&ini).expect("wall rules parse");
    let registry = OverlayTypeRegistry::from_ini(&ini, None);

    let mut sim = Simulation::new();
    let mut grid = OverlayGrid::new(10, 10);
    let owner_id = sim.interner.intern("Test");
    let type_id = sim.interner.intern("GAWALL");
    let mut next_id: u64 = 1;
    for rx in rx_range {
        grid.place_overlay(rx, ry, 2, 0);
        let mut entity = GameEntity::test_default(next_id, "GAWALL", "Test", rx, ry);
        entity.owner = owner_id;
        entity.type_ref = type_id;
        entity.health = Health {
            current: 400,
            max: 400,
        };
        sim.entities.insert(entity);
        next_id += 1;
    }
    sim.overlay_grid = Some(grid);
    sim.entities.rebuild_owner_index();

    (sim, rules, registry)
}

#[test]
fn concrete_wall_chain_reaction_runs_without_panic() {
    // Row of 4 GAWALL at (4..8, 5).
    let (mut sim, rules, registry) = build_minimal_sim_with_gawall_row(5, 4..8);
    // Pre-set (5,5) to stage 2 with E+W connectivity so a single damage event
    // pushes it through the penultimate-stage chain trigger (stage 3 of
    // DamageLevels=4). Connectivity nibble 0b1010 = E+W = 0xA, byte = 0x2A.
    sim.overlay_grid
        .as_mut()
        .unwrap()
        .set_overlay_data(5, 5, 0x2A);

    // damage = Strength (400) — gate `damage < strength` is false, so the
    // probabilistic check is skipped and the damage applies. Stage advances
    // to 3 → chain triggers 200-damage events on pristine same-type cardinal
    // neighbors. Outcome of those events depends on RNG roll vs strength=400.
    let events = [WallDamageEvent {
        rx: 5,
        ry: 5,
        damage: 400,
    }];
    sim.apply_wall_damage_events(&events, &rules, &registry);

    // The chain code path ran (no panic). Assert (5,5) is at stage ≥ 3 or
    // gone — either outcome is consistent with the binary's behavior at the
    // penultimate damage level.
    let grid = sim.overlay_grid.as_ref().unwrap();
    let cell = grid.cell(5, 5);
    if let Some(id) = cell.overlay_id {
        assert_eq!(id, 2, "if not destroyed, must still be GAWALL");
        assert!(
            cell.overlay_data >> 4 >= 3,
            "stage should have advanced to ≥3 after applied damage"
        );
    }
    // No assertion about pristine neighbors — their fate depends on RNG.
}

/// Seeded variant of `build_minimal_sim_with_gawall` — used for determinism
/// replay tests where two sims must produce byte-identical state given the
/// same input event sequence.
fn build_minimal_sim_with_gawall_seeded(
    rx: u16,
    ry: u16,
    seed: u64,
) -> (Simulation, RuleSet, OverlayTypeRegistry) {
    let (mut sim, rules, registry) = build_minimal_sim_with_gawall(rx, ry);
    sim.rng = crate::sim::rng::SimRng::new(seed);
    (sim, rules, registry)
}

#[test]
fn wall_damage_deterministic_across_replays() {
    let seed: u64 = 0x1234_5678;
    let events = [
        WallDamageEvent {
            rx: 5,
            ry: 5,
            damage: 100,
        },
        WallDamageEvent {
            rx: 5,
            ry: 5,
            damage: 100,
        },
        WallDamageEvent {
            rx: 5,
            ry: 5,
            damage: 100,
        },
        WallDamageEvent {
            rx: 5,
            ry: 5,
            damage: 100,
        },
        WallDamageEvent {
            rx: 5,
            ry: 5,
            damage: 100,
        },
    ];

    let snapshot_a: (Option<u8>, u8) = {
        let (mut sim, rules, registry) = build_minimal_sim_with_gawall_seeded(5, 5, seed);
        sim.apply_wall_damage_events(&events, &rules, &registry);
        let cell = sim.overlay_grid.as_ref().unwrap().cell(5, 5);
        (cell.overlay_id, cell.overlay_data)
    };
    let snapshot_b: (Option<u8>, u8) = {
        let (mut sim, rules, registry) = build_minimal_sim_with_gawall_seeded(5, 5, seed);
        sim.apply_wall_damage_events(&events, &rules, &registry);
        let cell = sim.overlay_grid.as_ref().unwrap().cell(5, 5);
        (cell.overlay_id, cell.overlay_data)
    };

    assert_eq!(
        snapshot_a, snapshot_b,
        "wall damage must be RNG-deterministic"
    );
}

#[test]
fn pursuit_weapon_range_for_entity_target() {
    use crate::sim::combat::{TargetKind, pursuit_weapon_range};
    let rules = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 0, 0, 300));
    store.insert(make_entity(2, "MTNK", 5, 0, 300));
    let interner = test_interner();

    let attacker = store.get(1).unwrap();
    let range = pursuit_weapon_range(attacker, &TargetKind::Entity(2), &store, &rules, &interner);
    // 105mm Range=6.
    assert_eq!(range, Some(crate::util::fixed_math::SimFixed::from_num(6)));
}

#[test]
fn pursuit_weapon_range_for_cell_target() {
    use crate::sim::combat::{TargetKind, pursuit_weapon_range};
    let rules = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 0, 0, 300));
    let interner = test_interner();

    let attacker = store.get(1).unwrap();
    let range = pursuit_weapon_range(
        attacker,
        &TargetKind::Cell(50, 50),
        &store,
        &rules,
        &interner,
    );
    // Cell target uses synthetic Structure category. MTNK 105mm Cannon is AG=true
    // (default), AP Verses[heavy] = 75% > 0. Range = 6.
    assert_eq!(range, Some(crate::util::fixed_math::SimFixed::from_num(6)));
}

#[test]
fn pursuit_weapon_range_none_for_unarmed_attacker() {
    use crate::sim::combat::{TargetKind, pursuit_weapon_range};
    let rules_str = "[InfantryTypes]\n0=ENGI\n\n\
                     [VehicleTypes]\n\n[BuildingTypes]\n\n[AircraftTypes]\n\n\
                     [ENGI]\nStrength=75\nArmor=none\nSpeed=4\n";
    let ini = IniFile::from_str(rules_str);
    let rules = RuleSet::from_ini(&ini).expect("parse");
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "ENGI", 0, 0, 75));
    let interner = test_interner();

    let attacker = store.get(1).unwrap();
    let range = pursuit_weapon_range(
        attacker,
        &TargetKind::Cell(50, 50),
        &store,
        &rules,
        &interner,
    );
    assert_eq!(range, None);
}

#[test]
fn v3_non_killing_aoe_emits_one_smudge_request() {
    // V3-style splash hits a heavy-armor target with HP > splash damage.
    // Target survives — currently produces zero smudges in dev HEAD; with
    // the per-shot helper wired, must emit exactly one Anim smudge request.
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\n\
         [VehicleTypes]\n0=MTNK\n1=V3\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=V3W\n\n\
         [V3]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=V3W\n\n\
         [V3W]\nDamage=100\nROF=20\nRange=10\nWarhead=V3WH\n\n\
         [V3WH]\nCellSpread=1\nPercentAtMax=1\nAnimList=V3EXP\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("v3 test rules should parse");

    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300)); // full HP — won't die
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    assert!(
        store.get(2).map(|e| e.health.current > 0).unwrap_or(false),
        "target must survive (test setup invariant)"
    );
    let anim_count = result
        .smudge_spawn_requests
        .iter()
        .filter(|r| matches!(r, SmudgeSpawnRequest::Anim { .. }))
        .count();
    assert_eq!(
        anim_count, 1,
        "one detonation must emit one Anim smudge request even on non-kill"
    );
    let v3exp = interner.intern("V3EXP");
    assert!(
        result.smudge_spawn_requests.iter().any(
            |r| matches!(r, SmudgeSpawnRequest::Anim { anim_name, .. } if *anim_name == v3exp)
        ),
        "Anim smudge must reference the V3 warhead's AnimList entry"
    );
}

#[test]
fn v3_killing_aoe_emits_exactly_one_smudge_request() {
    // V3 splash kills a low-HP target. Only ONE detonation occurred → ONE
    // Anim smudge request. After the kill-handler emission is removed
    // (Task 4), the per-shot helper is the sole emitter on kills.
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\n\
         [VehicleTypes]\n0=MTNK\n1=WEAK\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=V3W\n\n\
         [WEAK]\nStrength=10\nArmor=heavy\nSpeed=6\n\n\
         [V3W]\nDamage=200\nROF=20\nRange=10\nWarhead=V3WH\n\n\
         [V3WH]\nCellSpread=1\nPercentAtMax=1\nAnimList=V3EXP\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("v3 kill test rules should parse");

    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "WEAK", 8, 5, 10)); // dies in one hit
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    assert_eq!(
        result.despawned_ids.len(),
        1,
        "target must die (test setup invariant)"
    );
    let anim_count = result
        .smudge_spawn_requests
        .iter()
        .filter(|r| matches!(r, SmudgeSpawnRequest::Anim { .. }))
        .count();
    assert_eq!(
        anim_count, 1,
        "kill must emit exactly one Anim smudge — no double from kill-handler"
    );
}

#[test]
fn death_weapon_aoe_emits_separate_anim_from_killing_shot() {
    // A Demo-Truck-style entity (Explodes=yes, primary warhead with its own
    // AnimList) is killed by a tank with a different warhead and AnimList.
    // Per-shot emission (Task 2) → tank's TANKEXP anim. Death-AoE emission
    // (Task 3) → demo's UCEXPLOD anim. Two distinct anim names must appear.
    //
    // Note: while the kill-handler block is still in place (removed in
    // Task 4), the killing-shot anim is also emitted from the death handler,
    // so the *count* of anim entries is currently 3. We verify the two
    // distinct names without asserting total count; Task 4's test asserts
    // single-emission per detonation.
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\n\
         [VehicleTypes]\n0=TNK\n1=DEMO\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [TNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=TANKW\n\n\
         [DEMO]\nStrength=100\nArmor=light\nSpeed=6\nPrimary=DEMOW\nExplodes=yes\n\n\
         [TANKW]\nDamage=100\nROF=20\nRange=10\nWarhead=TANKHIT\n\n\
         [DEMOW]\nDamage=200\nROF=50\nRange=4\nWarhead=DEMOWH\n\n\
         [TANKHIT]\nCellSpread=0\nPercentAtMax=1\nAnimList=TANKEXP\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\n\
         [DEMOWH]\nCellSpread=2\nPercentAtMax=0.5\nAnimList=UCEXPLOD\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("demo-truck test rules should parse");

    let mut store = EntityStore::new();
    store.insert(make_entity(1, "TNK", 5, 5, 300));
    store.insert(make_entity(2, "DEMO", 8, 5, 100));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
    );

    let tankexp = interner.intern("TANKEXP");
    let ucexplod = interner.intern("UCEXPLOD");
    let unique_anim_names: std::collections::BTreeSet<_> = result
        .smudge_spawn_requests
        .iter()
        .filter_map(|r| match r {
            SmudgeSpawnRequest::Anim { anim_name, .. } => Some(*anim_name),
            _ => None,
        })
        .collect();

    assert!(
        unique_anim_names.contains(&tankexp),
        "killing-shot warhead AnimList anim must be emitted"
    );
    assert!(
        unique_anim_names.contains(&ucexplod),
        "death-explosion warhead AnimList anim must be emitted"
    );
    assert_eq!(
        unique_anim_names.len(),
        2,
        "exactly two distinct anim names — killing shot + death explosion"
    );
}

// --- emit_warhead_detonation_effects helper tests ---------------------------

fn emit_helper_test_warhead(animlist: &[&str]) -> crate::rules::warhead_type::WarheadType {
    let animlist_csv = animlist.join(",");
    let ini_text = format!("[WH]\nAnimList={}\n", animlist_csv);
    let ini = IniFile::from_str(&ini_text);
    let section = ini.section("WH").expect("section parses");
    crate::rules::warhead_type::WarheadType::from_ini_section("WH", section)
}

#[test]
fn emit_warhead_detonation_effects_empty_animlist_emits_nothing() {
    let mut interner = crate::sim::intern::StringInterner::new();
    let wh = emit_helper_test_warhead(&[]);
    let mut explosions: Vec<ExplosionEffect> = Vec::new();
    let mut smudges: Vec<SmudgeSpawnRequest> = Vec::new();
    emit_warhead_detonation_effects(
        &wh,
        100,
        5,
        5,
        crate::util::lepton::CELL_CENTER_LEPTON,
        crate::util::lepton::CELL_CENTER_LEPTON,
        0,
        &mut interner,
        &mut explosions,
        &mut smudges,
    );
    assert!(explosions.is_empty());
    assert!(smudges.is_empty());
}

#[test]
fn emit_warhead_detonation_effects_single_animlist_entry_emits_one_pair() {
    let mut interner = crate::sim::intern::StringInterner::new();
    let wh = emit_helper_test_warhead(&["EXPLOSION1"]);
    let mut explosions: Vec<ExplosionEffect> = Vec::new();
    let mut smudges: Vec<SmudgeSpawnRequest> = Vec::new();
    emit_warhead_detonation_effects(
        &wh,
        100,
        5,
        5,
        SimFixed::from_num(160),
        SimFixed::from_num(96),
        0,
        &mut interner,
        &mut explosions,
        &mut smudges,
    );
    assert_eq!(explosions.len(), 1);
    assert_eq!(smudges.len(), 1);
    let expected_id = interner.intern("EXPLOSION1");
    assert_eq!(explosions[0].shp_name, expected_id);
    assert_eq!(explosions[0].rx, 5);
    assert_eq!(explosions[0].ry, 5);
    assert_eq!(explosions[0].sub_x.to_num::<i32>(), 160);
    assert_eq!(explosions[0].sub_y.to_num::<i32>(), 96);
    assert_eq!(explosions[0].z, 0);
    match &smudges[0] {
        SmudgeSpawnRequest::Anim {
            anim_name,
            rx,
            ry,
            z,
        } => {
            assert_eq!(*anim_name, expected_id);
            assert_eq!(*rx, 5);
            assert_eq!(*ry, 5);
            assert_eq!(*z, 0);
        }
        other => panic!("expected Anim variant, got {:?}", other),
    }
}

#[test]
fn emit_warhead_detonation_effects_animlist_index_is_damage_div_25_clamped() {
    let mut interner = crate::sim::intern::StringInterner::new();
    let wh = emit_helper_test_warhead(&["EXP1", "EXP2", "EXP3"]);

    // damage=0 → idx=0 → EXP1.
    let mut explosions: Vec<ExplosionEffect> = Vec::new();
    let mut smudges: Vec<SmudgeSpawnRequest> = Vec::new();
    emit_warhead_detonation_effects(
        &wh,
        0,
        0,
        0,
        crate::util::lepton::CELL_CENTER_LEPTON,
        crate::util::lepton::CELL_CENTER_LEPTON,
        0,
        &mut interner,
        &mut explosions,
        &mut smudges,
    );
    assert_eq!(explosions[0].shp_name, interner.intern("EXP1"));

    // damage=50 → idx=2 (50/25) → EXP3.
    let mut explosions: Vec<ExplosionEffect> = Vec::new();
    let mut smudges: Vec<SmudgeSpawnRequest> = Vec::new();
    emit_warhead_detonation_effects(
        &wh,
        50,
        0,
        0,
        crate::util::lepton::CELL_CENTER_LEPTON,
        crate::util::lepton::CELL_CENTER_LEPTON,
        0,
        &mut interner,
        &mut explosions,
        &mut smudges,
    );
    assert_eq!(explosions[0].shp_name, interner.intern("EXP3"));

    // damage=10000 → idx clamped to len-1 (2) → EXP3.
    let mut explosions: Vec<ExplosionEffect> = Vec::new();
    let mut smudges: Vec<SmudgeSpawnRequest> = Vec::new();
    emit_warhead_detonation_effects(
        &wh,
        10000,
        0,
        0,
        crate::util::lepton::CELL_CENTER_LEPTON,
        crate::util::lepton::CELL_CENTER_LEPTON,
        0,
        &mut interner,
        &mut explosions,
        &mut smudges,
    );
    assert_eq!(explosions[0].shp_name, interner.intern("EXP3"));
}
