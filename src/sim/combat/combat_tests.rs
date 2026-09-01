//! Tests for the combat system — weapon firing, damage, and entity death.
//!
//! Extracted from combat.rs to keep it under the 400-line limit.

use std::collections::BTreeMap;

use super::*;
use crate::map::entities::EntityCategory;
use crate::map::houses::HouseAllianceMap;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::animation::{Animation, SequenceKind};
use crate::sim::components::Health;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::intern::{InternedId, test_intern, test_interner};
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::mission::state::MissionTestFixture;
use crate::sim::mission::{MissionDispatchTimer, MissionId, MissionType};
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::power_system::PowerState;
use crate::sim::projectile::ProjectileDetonationReason;
use crate::sim::rng::SimRng;
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
[SA]\nTiberium=yes\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
[AP]\nTiberium=yes\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n";
    let ini: IniFile = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("test rules should parse")
}

#[test]
fn sonic_active_wave_gate_precedes_target_resolution_and_all_shot_work() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=DLPH\n1=TARGET\n\n\
         [DLPH]\nStrength=200\nArmor=light\nSpeed=8\nPrimary=SonicZap\n\n\
         [TARGET]\nStrength=100\nArmor=wood\n\n\
         [SonicZap]\nDamage=4\nAmbientDamage=10\nROF=20\nRange=6\nWarhead=SonicWH\nIsSonic=yes\n\n\
         [SonicWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
    ))
    .expect("Sonic whole-shot gate fixture");
    let mut entities = EntityStore::new();
    let mut firer = GameEntity::test_default(1, "DLPH", "Americans", 4, 5);
    firer.attack_target = Some(AttackTarget::new(999));
    firer.current_weapon_index = 1;
    entities.insert(firer);
    let mut interner = test_interner();
    let snap = build_attacker_snapshot(
        entities.get(1).expect("Dolphin"),
        TargetKind::Entity(999),
        0,
        0,
        0,
        None,
        None,
        None,
    );
    let mut resources = BTreeMap::new();
    let mut rng = SimRng::new(0x50_4e_49_43);
    let rng_before = rng.logical_state();
    let mut hooks: Option<&mut dyn CombatInlineHooks> = None;
    let mut emit = CombatEmit::default();

    resolve_attacker_fire(
        &snap,
        &mut entities,
        &rules,
        &mut interner,
        None,
        &mut resources,
        None,
        &OccupancyGrid::new(),
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        17,
        67,
        true,
        &mut rng,
        None,
        &mut hooks,
        &mut emit,
    );

    assert!(emit.fire_events.is_empty());
    assert!(emit.damage_events.is_empty());
    assert!(emit.projectile_spawns.is_empty());
    assert!(emit.current_weapon_updates.is_empty());
    assert!(emit.burst_updates.is_empty());
    assert!(emit.retarget_events.is_empty());
    assert!(emit.remove_attack.is_empty());
    assert_eq!(rng.logical_state(), rng_before);
    assert_eq!(entities.get(1).unwrap().current_weapon_index, 1);
}

#[test]
fn gsi_08_05_rof_is_a_native_frame_count_plus_a_zero_to_two_jitter() {
    // `TechnoClass::GetROF @ 0x006FCFA0` returns
    // `ftol(ROF * difficulty + RandomRanged(0, 2))`. The jitter is ADDED, so a
    // 20-frame weapon reloads in 20, 21 or 22 frames — never 40, and never a
    // flat 20.
    let mut rng = crate::sim::rng::SimRng::new(0xC0FFEE_1234);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..256 {
        seen.insert(rof_to_cooldown_frames(20, &mut rng));
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        vec![20, 21, 22],
        "the jitter spans exactly RandomRanged(0, 2)"
    );
    // The floor and the saturating ceiling still hold with the jitter applied.
    assert!((1..=3).contains(&rof_to_cooldown_frames(-1, &mut rng)));
    assert!((1..=2).contains(&rof_to_cooldown_frames(0, &mut rng)));
    assert_eq!(
        rof_to_cooldown_frames(i32::from(u16::MAX) + 1, &mut rng),
        u16::MAX
    );
}

#[test]
fn gsi_08_05_rof_jitter_is_deterministic_for_a_seed() {
    let mut a = crate::sim::rng::SimRng::new(7);
    let mut b = crate::sim::rng::SimRng::new(7);
    let left: Vec<u16> = (0..32)
        .map(|_| rof_to_cooldown_frames(26, &mut a))
        .collect();
    let right: Vec<u16> = (0..32)
        .map(|_| rof_to_cooldown_frames(26, &mut b))
        .collect();
    assert_eq!(left, right);
}

#[test]
fn gsi_04_11_tiberium_prelude_gates_and_signed_large_quotient() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
         [Warheads]\n0=TIBWH\n1=PLAINWH\n\
         [OverlayTypes]\n0=DEFAULTORE\n1=CHAINORE\n2=CHAINPLAIN\n\
         [TIBWH]\nTiberium=yes\n\
         [PLAINWH]\nTiberium=no\n\
         [DEFAULTORE]\nTiberium=yes\n\
         [CHAINORE]\nTiberium=yes\nChainReaction=yes\n\
         [CHAINPLAIN]\nTiberium=no\nChainReaction=yes\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("tiberium prelude gate rules");
    let registry = crate::map::overlay_types::OverlayTypeRegistry::from_ini(&ini, None);
    let mut overlay = OverlayGrid::new(3, 1);
    overlay.place_overlay(0, 0, registry.id_for_name("DEFAULTORE").unwrap(), 0);
    overlay.place_overlay(1, 0, registry.id_for_name("CHAINORE").unwrap(), 0);
    overlay.place_overlay(2, 0, registry.id_for_name("CHAINPLAIN").unwrap(), 0);

    assert!(!combat_aoe::tiberium_reduction_cell_admitted(
        Some(&overlay),
        Some(&registry),
        0,
        0
    ));
    assert!(combat_aoe::tiberium_reduction_cell_admitted(
        Some(&overlay),
        Some(&registry),
        1,
        0
    ));
    assert!(!combat_aoe::tiberium_reduction_cell_admitted(
        Some(&overlay),
        Some(&registry),
        2,
        0
    ));

    let tib = rules.warhead("TIBWH").unwrap();
    let plain = rules.warhead("PLAINWH").unwrap();
    assert_eq!(
        tiberium_reduction_amount(655_360, true, tib),
        Some(65_536),
        "native signed quotient must not wrap through u16"
    );
    assert_eq!(tiberium_reduction_amount(100, true, plain), None);
    assert_eq!(tiberium_reduction_amount(100, false, tib), None);
    assert_eq!(tiberium_reduction_amount(-100, true, tib), None);
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
[AP]\nCellSpread=1\nPercentAtMax=1\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n\n\
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
    e.lifecycle.in_limbo = false;
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
    e.lifecycle.in_limbo = false;
    e
}

fn gsi_04_05_attack_frame_rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=SOURCE\n1=PROTECTED\n\
         [BuildingTypes]\n0=NORMAL\n1=MOD1X1\n2=STOCK2X2\n3=SELFNO\n4=SELFYES\n5=IMMUNE\n6=PROTECTEDBLDG\n\
         [Warheads]\n0=HITWH\n\
         [SOURCE]\nStrength=100\nArmor=heavy\n\
         [PROTECTED]\nStrength=100\nArmor=heavy\nToProtect=yes\n\
         [NORMAL]\nStrength=100\nArmor=wood\nFoundation=2x2\n\
         [MOD1X1]\nStrength=100\nArmor=wood\nUndeploysInto=MODUNIT\nFoundation=1x1\n\
         [STOCK2X2]\nStrength=100\nArmor=wood\nUndeploysInto=MODUNIT\nFoundation=2x2\n\
         [SELFNO]\nStrength=100\nArmor=wood\nFoundation=2x2\nDamageSelf=no\n\
         [SELFYES]\nStrength=100\nArmor=wood\nFoundation=2x2\nDamageSelf=yes\n\
         [IMMUNE]\nStrength=100\nArmor=wood\nFoundation=2x2\nImmune=yes\n\
         [PROTECTEDBLDG]\nStrength=100\nArmor=wood\nFoundation=2x2\nToProtect=yes\n\
         [HITWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("House attack-frame rules parse")
}

#[derive(Debug, PartialEq, Eq)]
struct BaseDefenseResponseTraceEntry {
    site: BaseDefenseResponseCallSite,
    victim_id: u64,
    health: u16,
    last_attacker_house_index: i32,
}

#[derive(Default)]
struct BaseDefenseResponseTraceHook {
    entries: Vec<BaseDefenseResponseTraceEntry>,
}

impl CombatInlineHooks for BaseDefenseResponseTraceHook {
    fn respond_to_base_attack(
        &mut self,
        site: BaseDefenseResponseCallSite,
        victim_id: u64,
        _attacker_id: u64,
        entities: &mut EntityStore,
        _rules: &RuleSet,
        _interner: &StringInterner,
        houses: &mut BTreeMap<InternedId, HouseState>,
        _scenario_rng: &mut SimRng,
        _terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    ) {
        let victim = entities
            .get(victim_id)
            .expect("response victim remains live");
        let last_attacker_house_index = houses[&victim.owner]
            .strategy_emergency
            .last_attacker_house_index();
        self.entries.push(BaseDefenseResponseTraceEntry {
            site,
            victim_id,
            health: victim.health.current,
            last_attacker_house_index,
        });
    }

    fn fatal_lifecycle(
        &mut self,
        _rules: &RuleSet,
        _stage: FatalLifecycleStage,
        _stable_id: u64,
        _category: EntityCategory,
        _entities: &mut EntityStore,
        _occupancy: &mut OccupancyGrid,
        _interner: &mut StringInterner,
        _scenario_rng: &mut SimRng,
        _terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
        _terrain_area_state: Option<&mut TerrainAreaState>,
        _sound_sink: Option<&mut Vec<SimSoundEvent>>,
    ) {
    }

    fn commit_tiberium_reduction(
        &mut self,
        _rules: &RuleSet,
        _request: TiberiumReductionRequest,
        _scenario_rng: &mut SimRng,
        _resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        _overlay_grid: Option<&mut OverlayGrid>,
        _overlay_registry: Option<&OverlayTypeRegistry>,
        _terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
        _terrain_area_state: Option<&TerrainAreaState>,
    ) {
    }

    fn commit_smudge(
        &mut self,
        _rules: &RuleSet,
        _request: SmudgeSpawnRequest,
        _occupancy: &OccupancyGrid,
        _interner: &StringInterner,
        _scenario_rng: &mut SimRng,
        _resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        _overlay_grid: Option<&mut OverlayGrid>,
        _overlay_registry: Option<&OverlayTypeRegistry>,
        _terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
        _terrain_area_state: Option<&TerrainAreaState>,
    ) {
    }
}

#[test]
fn gsi_04_05_building_attack_frame_prelude_obeys_object_and_type_gates() {
    let rules = gsi_04_05_attack_frame_rules();
    let mut entities = EntityStore::new();
    entities.insert(make_entity_owned(1, "SOURCE", 4, 4, 100, "Enemy"));
    for (id, type_name) in [
        (2, "NORMAL"),
        (3, "MOD1X1"),
        (4, "STOCK2X2"),
        (5, "SELFNO"),
        (6, "SELFYES"),
    ] {
        let mut target = make_entity_owned(id, type_name, id as u16 + 3, 4, 100, "Victim");
        target.category = EntityCategory::Structure;
        entities.insert(target);
    }
    let mut unit_target = make_entity_owned(7, "SOURCE", 10, 4, 100, "Victim");
    unit_target.category = EntityCategory::Unit;
    entities.insert(unit_target);
    entities.insert(make_entity_owned(8, "SOURCE", 11, 4, 100, "Victim"));
    let mut dead_target = make_entity_owned(9, "NORMAL", 12, 4, 0, "Victim");
    dead_target.category = EntityCategory::Structure;
    entities.insert(dead_target);

    let mut interner = test_interner();
    let victim_owner = interner.intern("Victim");
    let source_owner = interner.intern("Enemy");
    let wh = interner.intern("HITWH");
    let mut houses = BTreeMap::from([(
        victim_owner,
        HouseState::new(victim_owner, 0, None, false, 0, 10),
    )]);
    let frame = |houses: &BTreeMap<InternedId, HouseState>| {
        houses[&victim_owner]
            .strategy_emergency
            .last_building_attack_frame()
    };

    let event = EntityDamageEvent::area(2, -7, 0, 1, Some(source_owner), wh);
    assert_eq!(
        apply_building_receive_prelude(&event, &entities, &rules, &interner, &mut houses, 41),
        BuildingReceivePrelude::Respond
    );
    assert_eq!(
        frame(&houses),
        41,
        "negative damage still records the frame"
    );

    let event = EntityDamageEvent::area(2, 0, 0, 1, Some(victim_owner), wh);
    assert_eq!(
        apply_building_receive_prelude(&event, &entities, &rules, &interner, &mut houses, 42),
        BuildingReceivePrelude::Respond
    );
    assert_eq!(frame(&houses), 42, "alliance and zero damage are not gates");

    let null_attacker = EntityDamageEvent::area(2, 10, 0, RAD_NO_ATTACKER, Some(source_owner), wh);
    apply_building_receive_prelude(
        &null_attacker,
        &entities,
        &rules,
        &interner,
        &mut houses,
        43,
    );
    assert_eq!(
        frame(&houses),
        42,
        "source House cannot replace a null object"
    );

    let removed_attacker = EntityDamageEvent::area(2, 10, 0, 999, None, wh);
    apply_building_receive_prelude(
        &removed_attacker,
        &entities,
        &rules,
        &interner,
        &mut houses,
        43,
    );
    assert_eq!(
        frame(&houses),
        43,
        "the retained non-null object argument does not require a live lookup"
    );

    let no_source_house = EntityDamageEvent::area(2, 10, 0, 1, None, wh);
    apply_building_receive_prelude(
        &no_source_house,
        &entities,
        &rules,
        &interner,
        &mut houses,
        44,
    );
    assert_eq!(
        frame(&houses),
        44,
        "source House is not required for the write"
    );

    let allied = EntityDamageEvent::area(2, 10, 0, 8, Some(victim_owner), wh);
    apply_building_receive_prelude(&allied, &entities, &rules, &interner, &mut houses, 45);
    assert_eq!(
        frame(&houses),
        45,
        "a distinct attacker owned by the victim House still writes"
    );

    let modded_skip = EntityDamageEvent::area(3, 10, 0, 1, Some(source_owner), wh);
    apply_building_receive_prelude(&modded_skip, &entities, &rules, &interner, &mut houses, 46);
    assert_eq!(
        frame(&houses),
        45,
        "a 1x1 undeployer takes vtable +0x80 skip"
    );

    let stock_shape = EntityDamageEvent::area(4, 10, 0, 1, Some(source_owner), wh);
    apply_building_receive_prelude(&stock_shape, &entities, &rules, &interner, &mut houses, 47);
    assert_eq!(
        frame(&houses),
        47,
        "larger retail-shaped undeployer records"
    );

    let already_dead = EntityDamageEvent::area(9, 10, 0, 1, Some(source_owner), wh);
    apply_building_receive_prelude(&already_dead, &entities, &rules, &interner, &mut houses, 48);
    assert_eq!(frame(&houses), 48, "Health zero is later than this prelude");

    let self_no = EntityDamageEvent::area(5, 10, 0, 5, Some(victim_owner), wh);
    assert_eq!(
        apply_building_receive_prelude(&self_no, &entities, &rules, &interner, &mut houses, 49,),
        BuildingReceivePrelude::ReturnZero
    );
    assert_eq!(frame(&houses), 48);

    let self_yes = EntityDamageEvent::area(6, 10, 0, 6, Some(victim_owner), wh);
    assert_eq!(
        apply_building_receive_prelude(
            &self_yes,
            &entities,
            &rules,
            &interner,
            &mut houses,
            0x1_8000_0001,
        ),
        BuildingReceivePrelude::Respond
    );
    assert_eq!(
        frame(&houses),
        i32::MIN + 1,
        "native stores the raw low dword"
    );

    let unit = EntityDamageEvent::area(7, 10, 0, 1, Some(source_owner), wh);
    apply_building_receive_prelude(&unit, &entities, &rules, &interner, &mut houses, 47);
    assert_eq!(frame(&houses), i32::MIN + 1, "the wrapper is Building-only");
}

#[test]
fn gsi_04_05_building_attack_frame_precedes_immune_receiver_exit() {
    let rules = gsi_04_05_attack_frame_rules();
    let mut entities = EntityStore::new();
    entities.insert(make_entity_owned(1, "SOURCE", 4, 4, 100, "Enemy"));
    let mut target = make_entity_owned(2, "IMMUNE", 5, 4, 100, "Victim");
    target.category = EntityCategory::Structure;
    entities.insert(target);
    let mut interner = test_interner();
    let victim_owner = interner.intern("Victim");
    let source_owner = interner.intern("Enemy");
    let wh = interner.intern("HITWH");
    let mut houses = BTreeMap::from([
        (
            victim_owner,
            HouseState::new(victim_owner, 0, None, false, 0, 10),
        ),
        (
            source_owner,
            HouseState::new(source_owner, 1, None, false, 0, 10),
        ),
    ]);
    let mut occupancy = OccupancyGrid::new();
    let mut main_rng = SimRng::new(5);
    let mut scenario_rng = SimRng::new(7);
    let mut handled_deaths = Vec::new();
    let mut resources = BTreeMap::new();
    let mut trace_hook = BaseDefenseResponseTraceHook::default();
    let mut fatal_lifecycle: Option<&mut dyn CombatInlineHooks> = Some(&mut trace_hook);
    let mut sound_sink = None;

    let _ = commit_damage_events(
        &[EntityDamageEvent::area(2, 10, 0, 1, Some(source_owner), wh)],
        &mut entities,
        &mut occupancy,
        &rules,
        &mut interner,
        &mut houses,
        &[victim_owner, source_owner],
        &HouseAllianceMap::new(),
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        77,
        &mut fatal_lifecycle,
        &mut sound_sink,
    );

    assert_eq!(entities.get(2).unwrap().health.current, 100);
    assert_eq!(
        trace_hook.entries,
        vec![BaseDefenseResponseTraceEntry {
            site: BaseDefenseResponseCallSite::BuildingPrelude,
            victim_id: 2,
            health: 100,
            last_attacker_house_index: 1,
        }],
        "Building response runs after the attacker-House index write and before immunity"
    );
    assert_eq!(
        houses[&victim_owner]
            .strategy_emergency
            .last_building_attack_frame(),
        77,
        "the House write happens before ObjectClass immunity rejects damage"
    );
}

#[test]
fn gsi_04_05_protected_techno_response_runs_after_object_health_commit() {
    let rules = gsi_04_05_attack_frame_rules();
    let mut entities = EntityStore::new();
    entities.insert(make_entity_owned(1, "SOURCE", 4, 4, 100, "Enemy"));
    entities.insert(make_entity_owned(2, "PROTECTED", 5, 4, 100, "Victim"));
    let mut protected_building = make_entity_owned(3, "PROTECTEDBLDG", 6, 4, 100, "Victim");
    protected_building.category = EntityCategory::Structure;
    entities.insert(protected_building);
    entities.insert(make_entity_owned(4, "SOURCE", 7, 4, 100, "Victim"));

    let mut interner = test_interner();
    let victim_owner = interner.intern("Victim");
    let source_owner = interner.intern("Enemy");
    let wh = interner.intern("HITWH");
    let mut houses = BTreeMap::from([
        (
            victim_owner,
            HouseState::new(victim_owner, 0, None, false, 0, 10),
        ),
        (
            source_owner,
            HouseState::new(source_owner, 1, None, false, 0, 10),
        ),
    ]);
    let mut occupancy = OccupancyGrid::new();
    let mut main_rng = SimRng::new(5);
    let mut scenario_rng = SimRng::new(7);
    let mut handled_deaths = Vec::new();
    let mut resources = BTreeMap::new();
    let mut trace_hook = BaseDefenseResponseTraceHook::default();
    let mut inline_hooks: Option<&mut dyn CombatInlineHooks> = Some(&mut trace_hook);
    let mut sound_sink = None;

    let _ = commit_damage_events(
        &[
            EntityDamageEvent::area(2, 10, 0, 1, Some(source_owner), wh),
            EntityDamageEvent::area(3, 10, 0, 1, Some(source_owner), wh),
            EntityDamageEvent::area(4, 10, 0, 1, Some(source_owner), wh),
        ],
        &mut entities,
        &mut occupancy,
        &rules,
        &mut interner,
        &mut houses,
        &[victim_owner, source_owner],
        &HouseAllianceMap::new(),
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        91,
        &mut inline_hooks,
        &mut sound_sink,
    );

    assert_eq!(
        trace_hook.entries,
        vec![
            BaseDefenseResponseTraceEntry {
                site: BaseDefenseResponseCallSite::ProtectedTechno,
                victim_id: 2,
                health: 90,
                last_attacker_house_index: -1,
            },
            BaseDefenseResponseTraceEntry {
                site: BaseDefenseResponseCallSite::BuildingPrelude,
                victim_id: 3,
                health: 100,
                last_attacker_house_index: 1,
            },
            BaseDefenseResponseTraceEntry {
                site: BaseDefenseResponseCallSite::ProtectedTechno,
                victim_id: 3,
                health: 90,
                last_attacker_house_index: 1,
            },
        ],
        "ToProtect calls once after Object health; Buildings first run their wrapper response"
    );
    assert_eq!(entities.get(2).unwrap().health.current, 90);
    assert_eq!(entities.get(3).unwrap().health.current, 90);
    assert_eq!(entities.get(4).unwrap().health.current, 90);
}

#[test]
fn gsi_04_05_building_self_damage_return_zero_stops_receiver_commit() {
    let rules = gsi_04_05_attack_frame_rules();
    let mut entities = EntityStore::new();
    let mut target = make_entity_owned(5, "SELFNO", 5, 4, 100, "Victim");
    target.category = EntityCategory::Structure;
    entities.insert(target);
    let mut interner = test_interner();
    let victim_owner = interner.intern("Victim");
    let wh = interner.intern("HITWH");
    let mut houses = BTreeMap::from([(
        victim_owner,
        HouseState::new(victim_owner, 0, None, false, 0, 10),
    )]);
    let mut occupancy = OccupancyGrid::new();
    let mut main_rng = SimRng::new(5);
    let mut scenario_rng = SimRng::new(7);
    let mut handled_deaths = Vec::new();
    let mut resources = BTreeMap::new();
    let mut fatal_lifecycle = None;
    let mut sound_sink = None;

    let _ = commit_damage_events(
        &[EntityDamageEvent::direct_receiver(
            5,
            10,
            0,
            5,
            Some(victim_owner),
            wh,
            ReceiverCallFlags {
                ignore_defenses: false,
                arg6: false,
            },
        )],
        &mut entities,
        &mut occupancy,
        &rules,
        &mut interner,
        &mut houses,
        &[victim_owner],
        &HouseAllianceMap::new(),
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        88,
        &mut fatal_lifecycle,
        &mut sound_sink,
    );

    assert_eq!(entities.get(5).unwrap().health.current, 100);
    assert_eq!(
        houses[&victim_owner]
            .strategy_emergency
            .last_building_attack_frame(),
        0
    );
}

#[test]
fn gsi_04_05_building_attack_frame_survives_world_receiver_merge() {
    let rules = gsi_04_05_attack_frame_rules();
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let source_id = sim
        .spawn_object("SOURCE", "Enemy", 4, 4, 0, &rules, &heights)
        .expect("source spawns");
    let target_id = sim
        .spawn_object("NORMAL", "Victim", 6, 4, 0, &rules, &heights)
        .expect("Building target spawns");
    let source_owner = sim.substrate.entities.get(source_id).unwrap().owner;
    let victim_owner = sim.substrate.entities.get(target_id).unwrap().owner;
    sim.houses.insert(
        source_owner,
        HouseState::new(source_owner, 0, None, false, 0, 10),
    );
    sim.houses.insert(
        victim_owner,
        HouseState::new(victim_owner, 1, None, false, 0, 10),
    );
    sim.session.house_order = vec![source_owner, victim_owner];
    sim.session.binary_frame = 77;
    let warhead = sim.interner.intern("HITWH");

    let event = EntityDamageEvent::area(target_id, 10, 0, source_id, Some(source_owner), warhead);
    sim.commit_noncombat_aoe_hits(&rules, None, &[event, event]);

    assert_eq!(
        sim.houses[&victim_owner]
            .strategy_emergency
            .last_building_attack_frame(),
        77,
        "the world owner must retain the timestamp written into its staged House map"
    );
    assert_eq!(
        sim.houses[&victim_owner]
            .strategy_emergency
            .last_attacker_house_index(),
        0,
        "the staged attacker-House index must merge back with the frame stamp"
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(target_id)
            .unwrap()
            .health
            .current,
        80,
        "repeated qualifying receiver records in one frame remain admitted"
    );
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

fn run_combat_death_handoff(
    entities: &mut EntityStore,
    rules: &RuleSet,
    interner: &mut crate::sim::intern::StringInterner,
    dead_entities: &[u64],
) -> DeathEffects {
    let mut occupancy = OccupancyGrid::new();
    let mut resource_nodes = BTreeMap::new();
    let mut houses = BTreeMap::new();
    let mut main_rng = SimRng::new(0);
    let mut scenario_rng = SimRng::new(0);
    let mut handled_deaths = Vec::new();
    let handles = crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, interner);
    handle_entity_deaths(
        entities,
        &mut occupancy,
        rules,
        interner,
        Some(handles),
        &mut houses,
        &[],
        &HouseAllianceMap::new(),
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        dead_entities,
        &[],
        &mut resource_nodes,
        None,
        None,
        None,
        &mut None,
        false,
        0,
        &mut None,
        &mut None,
    )
}

#[test]
fn gsi_04_10_projectile_inert_suppresses_bridge_ore_and_collector_rng() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [Warheads]\n0=WH\n\
         [WH]\nWall=yes\nCellSpread=1\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
    ))
    .expect("inert projectile rules");
    let mut interner = test_interner();
    let warhead = interner.intern("WH");
    let weapon = interner.intern("MissingWeapon");
    let owner = interner.intern("Owner");
    let detonation = ProjectileDetonation {
        projectile_id: 1,
        source_id: 99,
        target: ProjectileTarget::Cell { rx: 8, ry: 5 },
        impact: ProjectileCoord::new(8 * 256 + 128, 5 * 256 + 128, 0),
        payload: ProjectilePayload {
            base_damage: 100,
            warhead,
            weapon,
            owner,
        },
        reason: ProjectileDetonationReason::ReachedTarget,
    };
    let mut entities = EntityStore::new();
    let occupancy = OccupancyGrid::new();
    let mut scenario_rng = SimRng::new(77);
    let before_rng = scenario_rng.state();
    let mut emit = CombatEmit::default();
    let mut resource_nodes = BTreeMap::new();
    let mut inline_hooks = None;

    let handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
    emit_projectile_detonations(
        &[detonation],
        &mut entities,
        &occupancy,
        &rules,
        &mut interner,
        Some(handles),
        &mut resource_nodes,
        None,
        None,
        None,
        None,
        None,
        None,
        true,
        &HouseAllianceMap::new(),
        &mut scenario_rng,
        &mut inline_hooks,
        &mut emit,
    );

    assert!(emit.damage_events.is_empty());
    assert!(emit.wall_mutations.is_empty());
    assert!(emit.cell_target_detaches.is_empty());
    assert!(emit.bridge_damage_events.is_empty());
    assert!(emit.tiberium_reduction_requests.is_empty());
    assert_eq!(scenario_rng.state(), before_rng);
}

#[test]
fn lifecycle_authority_immediate_combat_death_reaches_uninit_without_precleanup() {
    let rules = test_rules();
    let mut store = EntityStore::new();

    let mut dead = make_entity(1, "MTNK", 5, 5, 0);
    dead.selected = true;
    dead.attack_target = Some(AttackTarget::new(2));
    dead.movement_target = Some(crate::sim::components::MovementTarget::default());
    dead.in_logic_vector = true;
    dead.lifecycle.in_limbo = false;
    dead.lifecycle.cell_marked = true;
    dead.radio_contacts.insert(2);
    store.insert(dead);

    let mut observer = make_entity(2, "MTNK", 6, 5, 300);
    observer.attack_target = Some(AttackTarget::new(1));
    store.insert(observer);

    let mut interner = test_interner();
    let result = run_combat_death_handoff(&mut store, &rules, &mut interner, &[1]);

    assert_eq!(result.immediate_uninit_ids, vec![1]);
    assert_eq!(result.despawned_ids, vec![1]);
    let dead = store
        .get(1)
        .expect("world must still be able to UnInit the victim");
    assert_eq!(dead.health.current, 0);
    assert!(
        !dead.dying,
        "immediate UnInit, not combat, owns the death gate"
    );
    assert!(dead.selected, "UnInit owns deselection");
    assert!(dead.attack_target.is_some(), "UnInit owns attack cleanup");
    assert!(
        dead.movement_target.is_some(),
        "UnInit owns movement cleanup"
    );
    assert!(dead.in_logic_vector, "UnInit owns LogicVector removal");
    assert!(dead.lifecycle.object_alive);
    assert!(!dead.lifecycle.in_limbo);
    assert!(dead.lifecycle.cell_marked);
    assert!(dead.radio_contacts.contains(2), "Techno Limbo owns BREAK");
    assert!(
        !dead.owned_count_released,
        "world lifecycle owns count release"
    );
    assert!(
        store.get(2).unwrap().attack_target.is_some(),
        "combat must not bulk-clear other objects' targets"
    );
}

#[test]
fn lifecycle_authority_combat_leaves_transport_cargo_for_carrier_uninit() {
    let rules = test_rules();
    let mut store = EntityStore::new();

    let mut cargo = crate::sim::passenger::PassengerCargo::new(2, 1);
    assert!(cargo.board(2, 1));
    let mut carrier = make_entity(1, "MTNK", 5, 5, 0);
    carrier.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
    store.insert(carrier);

    let mut passenger = make_infantry_entity(2, "E1", 5, 5, 125);
    passenger.passenger_role = crate::sim::passenger::PassengerRole::Inside { transport_id: 1 };
    passenger.selected = true;
    passenger.attack_target = Some(AttackTarget::new(3));
    passenger.movement_target = Some(crate::sim::components::MovementTarget::default());
    store.insert(passenger);
    store.insert(make_entity(3, "MTNK", 6, 5, 300));

    let mut interner = test_interner();
    let result = run_combat_death_handoff(&mut store, &rules, &mut interner, &[1]);

    assert_eq!(result.immediate_uninit_ids, vec![1]);
    assert!(result.destroyed_garrison_buildings.is_empty());
    let carrier = store.get(1).unwrap();
    assert_eq!(carrier.passenger_role.cargo().unwrap().passengers, vec![2]);
    let passenger = store.get(2).unwrap();
    assert_eq!(passenger.health.current, 125);
    assert!(!passenger.dying);
    assert!(matches!(
        passenger.passenger_role,
        crate::sim::passenger::PassengerRole::Inside { transport_id: 1 }
    ));
    assert!(passenger.selected);
    assert!(passenger.attack_target.is_some());
    assert!(passenger.movement_target.is_some());
}

#[test]
fn lifecycle_authority_animated_combat_handoff_changes_only_dying_and_sequence() {
    let rules = test_rules();
    let mut store = EntityStore::new();

    let mut dead = make_infantry_entity(1, "E1", 5, 5, 0);
    dead.selected = true;
    dead.attack_target = Some(AttackTarget::new(2));
    dead.movement_target = Some(crate::sim::components::MovementTarget::default());
    dead.in_logic_vector = true;
    dead.lifecycle.in_limbo = false;
    dead.lifecycle.cell_marked = true;
    dead.radio_contacts.insert(2);
    store.insert(dead);

    let mut observer = make_entity(2, "MTNK", 6, 5, 300);
    observer.attack_target = Some(AttackTarget::new(1));
    store.insert(observer);

    let mut interner = test_interner();
    let result = run_combat_death_handoff(&mut store, &rules, &mut interner, &[1]);

    assert!(result.immediate_uninit_ids.is_empty());
    assert_eq!(result.despawned_ids, vec![1]);
    let dead = store.get(1).unwrap();
    assert_eq!(dead.health.current, 0);
    assert!(dead.dying);
    assert_eq!(
        dead.animation.as_ref().unwrap().sequence,
        SequenceKind::Die1
    );
    assert!(dead.selected);
    assert!(dead.attack_target.is_some());
    assert!(dead.movement_target.is_some());
    assert!(dead.in_logic_vector);
    assert!(dead.lifecycle.object_alive);
    assert!(!dead.lifecycle.in_limbo);
    assert!(dead.lifecycle.cell_marked);
    assert!(dead.radio_contacts.contains(2));
    assert!(!dead.owned_count_released);
    assert!(
        store.get(2).unwrap().attack_target.is_some(),
        "selective removal listeners, not combat, own target invalidation"
    );
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
        attack_impact_z(TargetKind::Cell(7, 9), &entities, None),
        0,
        "with no loaded terrain there is no cell floor to read; the cell-centre \
         helper never invents one. The terrain-backed cases live in \
         `impact_height_tests`."
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

    let target_entity = sim
        .substrate
        .entities
        .get(target)
        .expect("target should exist");
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

    issue_attack_command(
        &mut sim.substrate.entities,
        attacker,
        target,
        None,
        &sim.interner,
    );
    let mut main_rng = SimRng::new(1);
    let result = tick_combat(
        &mut sim.substrate.entities,
        &mut sim.substrate.occupancy,
        &rules,
        &mut sim.interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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

    let target_entity = sim
        .substrate
        .entities
        .get(target)
        .expect("target should exist");
    assert_eq!(target_entity.category, EntityCategory::Infantry);
    assert_eq!(
        combat_target_category(target_entity, &rules, &sim.interner),
        EntityCategory::Infantry
    );

    issue_attack_command(
        &mut sim.substrate.entities,
        attacker,
        target,
        None,
        &sim.interner,
    );
    let mut main_rng = SimRng::new(1);
    let result = tick_combat(
        &mut sim.substrate.entities,
        &mut sim.substrate.occupancy,
        &rules,
        &mut sim.interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);
    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        10u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);
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
        &[],
        None,
        &mut main_rng,
    );
    assert!(
        result.bridge_damage_events.is_empty(),
        "non-wall warheads must not emit bridge damage"
    );
    assert!(
        result.wall_mutations.is_empty(),
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
    let _handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&bridge_rules, &mut interner);
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
        &[],
        None,
        &mut main_rng,
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
    // cell — events fall through to bridge_damage_events. Immediate wall
    // mutation requires both a grid lookup and Wall=yes in the registry.
    assert!(wall_result.wall_mutations.is_empty());
}

#[test]
fn gsi_04_07_damage_wad_precedes_wall_and_wood_armor_routing() {
    fn fire(extra_warhead_flags: &str, overlay_armor: &str) -> (CombatTickResult, OverlayGrid) {
        let ini = IniFile::from_str(&format!(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=MTNK\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Warheads]\n0=KillWH\n1=WallWH\n2=NoWallWH\n\
             [OverlayTypes]\n0=TESTWALL\n\
             [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=Gun\n\
             [Gun]\nDamage=65\nROF=50\nRange=6\nWarhead=WH\n\
             [WH]\n{extra_warhead_flags}\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [TESTWALL]\nWall=yes\nArmor={overlay_armor}\nStrength=1\n"
        ));
        let mut rules = RuleSet::from_ini(&ini).expect("wall route rules");
        let registry = OverlayTypeRegistry::from_ini(&ini, None);
        let mut entities = EntityStore::new();
        entities.insert(make_entity(1, "MTNK", 5, 5, 300));
        entities.insert(make_entity(2, "MTNK", 8, 5, 300));
        let mut interner = test_interner();
        let _handles =
            crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
        issue_attack_command(&mut entities, 1, 2, None, &interner);
        let mut overlays = OverlayGrid::new(12, 12);
        overlays.place_overlay(8, 5, 0, 0);
        let mut scenario_rng = SimRng::new(1);
        let result = tick_combat_with_fog(
            &mut entities,
            &mut OccupancyGrid::new(),
            &rules,
            &mut interner,
            None,
            &BTreeMap::new(),
            None,
            &mut BTreeMap::new(),
            Some(&mut overlays),
            Some(&registry),
            None,
            0,
            100,
            0,
            &[],
            None,
            &mut scenario_rng,
        );
        (result, overlays)
    }

    let (absolute, absolute_grid) = fire("WallAbsoluteDestroyer=yes\nWall=yes", "concrete");
    assert_eq!(
        absolute.wall_mutations,
        vec![crate::sim::overlay_grid::WallMutation {
            rx: 8,
            ry: 5,
            kind: crate::sim::overlay_grid::WallMutationKind::DirectRemoved,
        }],
        "WallAbsoluteDestroyer wins and commits forced removal inline"
    );
    assert_eq!(absolute_grid.cell(8, 5).overlay_id, None);
    assert!(absolute.bridge_damage_events.is_empty());

    let (wood, wood_grid) = fire("Wood=yes", "wood");
    assert!(!wood.wall_mutations.is_empty());
    assert_eq!(wood_grid.cell(8, 5).overlay_id, None);
    let (concrete, concrete_grid) = fire("Wood=yes", "concrete");
    assert!(concrete.wall_mutations.is_empty());
    assert_eq!(concrete_grid.cell(8, 5).overlay_id, Some(0));
}

#[test]
fn gsi_04_07_damage_live_order_second_attacker_reads_restored_target() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=MTNK\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [OverlayTypes]\n0=TESTWALL\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=Gun\n\
         [Gun]\nDamage=1\nROF=50\nRange=8\nWarhead=WH\n\
         [WH]\nWallAbsoluteDestroyer=yes\nWall=yes\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [TESTWALL]\nWall=yes\nArmor=concrete\nStrength=400\n",
    );
    let mut rules = RuleSet::from_ini(&ini).expect("live-order wall rules");
    let registry = OverlayTypeRegistry::from_ini(&ini, None);
    let mut entities = EntityStore::new();
    entities.insert(make_entity(10, "MTNK", 5, 5, 300));
    let mut second = make_entity(20, "MTNK", 6, 5, 300);
    second.mission.apply_test_fixture(MissionTestFixture {
        current: MissionId::from_known(MissionType::Attack),
        suspended: MissionId::from_known(MissionType::Guard),
        queued: MissionId::NONE,
        movement_bypass_latch: 0,
        handler_state: 0,
        mission_start_frame: 0,
        ai_counter: 0,
        dispatch_timer: MissionDispatchTimer::at_frame(0),
    });
    second.suspended_attack_target = Some(TargetKind::Entity(10));
    entities.insert(second);
    let mut interner = test_interner();
    let _handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
    assert!(issue_attack_cell_command(
        &mut entities,
        10,
        8,
        5,
        Some(&rules),
        &interner,
    ));
    assert!(issue_attack_cell_command(
        &mut entities,
        20,
        8,
        5,
        Some(&rules),
        &interner,
    ));

    let mut overlays = OverlayGrid::new(16, 16);
    overlays.place_overlay(8, 5, 0, 0);
    let mut scenario_rng = SimRng::new(31);
    // Two attackers fire this tick, so the scenario stream advances by exactly
    // two `GetROF` reload jitters (`RandomRanged(0, 2)` @ `0x006FD0B0`) and
    // nothing else — in particular the WAD warhead still draws no Strength.
    let mut expected_rng = scenario_rng.clone();
    expected_rng.next_range_u32_inclusive(0, 2);
    expected_rng.next_range_u32_inclusive(0, 2);
    let before = expected_rng.state();
    let result = tick_combat_with_fog(
        &mut entities,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::new(),
        None,
        &mut BTreeMap::new(),
        Some(&mut overlays),
        Some(&registry),
        None,
        0,
        100,
        0,
        &[10, 20],
        None,
        &mut scenario_rng,
    );

    assert_eq!(overlays.cell(8, 5).overlay_id, None);
    assert_eq!(
        scenario_rng.state(),
        before,
        "WAD consumes no Strength draw beyond the two reload jitters"
    );
    assert_eq!(
        result
            .fire_events
            .iter()
            .map(|event| (event.attacker_id, event.target))
            .collect::<Vec<_>>(),
        vec![(10, TargetKind::Cell(8, 5)), (20, TargetKind::Entity(10))],
        "second live-order attacker must not fire its stale cell snapshot"
    );
    assert_eq!(
        result
            .cell_target_detaches
            .iter()
            .map(|event| (event.listener_id, event.restored, event.cleared))
            .collect::<Vec<_>>(),
        vec![(10, false, true), (20, true, true)]
    );
}

#[test]
fn gsi_04_07_damage_prior_projectile_fatal_death_weapon_is_inline() {
    fn run(victim_hp: u16, explodes: bool) -> (CombatTickResult, OverlayGrid, EntityStore, u64) {
        let ini_text = format!(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=BOOMER\n1=SHOOTER\n2=TARGET\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [OverlayTypes]\n0=TESTWALL\n\
             [BOOMER]\nStrength=11\nArmor=heavy\nExplodes={}\nDeathWeapon=DeathBoom\n\
             [SHOOTER]\nStrength=100\nArmor=heavy\nPrimary=Gun\n\
             [TARGET]\nStrength=100\nArmor=heavy\n\
             [DeathBoom]\nDamage=214\nWarhead=WallWH\n\
             [Gun]\nDamage=0\nROF=50\nRange=8\nWarhead=NoWallWH\n\
             [WallWH]\nCellSpread=0\nWall=yes\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [NoWallWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [TESTWALL]\nWall=yes\nArmor=concrete\nStrength=400\n",
            if explodes { "yes" } else { "no" },
        );
        let ini = IniFile::from_str(&ini_text);
        let art = IniFile::from_str("[TESTWALL]\nDamageLevels=2\n");
        let mut rules = RuleSet::from_ini(&ini).expect("inline death-weapon rules");
        let registry = OverlayTypeRegistry::from_ini(&ini, Some(&art));
        assert_eq!(registry.flags(0).unwrap().strength, 400);
        assert_eq!(registry.flags(0).unwrap().damage_levels, 2);
        assert_eq!(
            rules.object("BOOMER").unwrap().death_weapon.as_deref(),
            Some("DeathBoom")
        );
        assert!(!rules.warhead("WallWH").unwrap().wall_absolute_destroyer);
        assert_eq!(rules.weapon("DeathBoom").unwrap().damage, 214);

        let mut entities = EntityStore::new();
        entities.insert(make_entity(10, "BOOMER", 8, 5, victim_hp));
        let mut later_attacker = make_entity(20, "SHOOTER", 6, 5, 100);
        later_attacker
            .mission
            .apply_test_fixture(MissionTestFixture {
                current: MissionId::from_known(MissionType::Attack),
                suspended: MissionId::from_known(MissionType::Guard),
                queued: MissionId::NONE,
                movement_bypass_latch: 0,
                handler_state: 0,
                mission_start_frame: 0,
                ai_counter: 0,
                dispatch_timer: MissionDispatchTimer::at_frame(0),
            });
        later_attacker.suspended_attack_target = Some(TargetKind::Entity(30));
        entities.insert(later_attacker);
        entities.insert(make_entity(30, "TARGET", 5, 5, 100));
        let mut interner = test_interner();
        let _handles =
            crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
        assert!(issue_attack_cell_command(
            &mut entities,
            20,
            8,
            5,
            Some(&rules),
            &interner,
        ));

        let mut overlays = OverlayGrid::new(16, 16);
        overlays.place_overlay(8, 5, 0, 0);
        let detonation = ProjectileDetonation {
            projectile_id: 1,
            source_id: 99,
            target: ProjectileTarget::Entity(10),
            impact: ProjectileCoord::new(8 * 256 + 128, 5 * 256 + 128, 0),
            payload: ProjectilePayload {
                base_damage: 10,
                warhead: interner.intern("NoWallWH"),
                weapon: interner.intern("Gun"),
                owner: interner.intern("Test"),
            },
            reason: ProjectileDetonationReason::ReachedTarget,
        };
        let mut scenario_rng = SimRng::new(1);
        let mut main_rng = SimRng::new(41);
        let mut houses = BTreeMap::new();
        let handles =
            crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
        let result = tick_combat_with_fog_and_main_rng(
            &mut entities,
            &mut OccupancyGrid::new(),
            &rules,
            &mut interner,
            Some(handles),
            None,
            &BTreeMap::new(),
            &mut houses,
            &[],
            &HouseAllianceMap::new(),
            None,
            &mut BTreeMap::new(),
            Some(&mut overlays),
            Some(&registry),
            None,
            0,
            100,
            0,
            &[10, 20, 30],
            &[detonation],
            &[],
            None,
            &[],
            &mut scenario_rng,
            &mut main_rng,
            None,
        );
        (result, overlays, entities, scenario_rng.state())
    }

    let (fatal, fatal_grid, fatal_entities, fatal_rng) = run(10, true);
    assert_eq!(fatal_entities.get(10).unwrap().health.current, 0);
    assert_eq!(fatal.immediate_uninit_ids, vec![10]);
    assert_eq!(fatal_grid.cell(8, 5).overlay_id, None);
    assert_eq!(
        fatal
            .fire_events
            .iter()
            .map(|event| (event.attacker_id, event.target))
            .collect::<Vec<_>>(),
        vec![(20, TargetKind::Entity(30))],
        "later live-order attacker must observe nested target restoration"
    );
    assert!(
        fatal
            .cell_target_detaches
            .iter()
            .any(|detach| { detach.listener_id == 20 && detach.restored && detach.cleared })
    );
    assert_eq!(
        fatal_entities
            .get(20)
            .unwrap()
            .attack_target
            .as_ref()
            .unwrap()
            .target,
        TargetKind::Entity(30)
    );
    // Detonation processing runs before the fire phase, so the inline wall
    // draw lands first and the surviving attacker's `GetROF` reload jitter
    // (`RandomRanged(0, 2)` @ `0x006FD0B0`) follows it.
    let mut one_draw = SimRng::new(1);
    assert_eq!(one_draw.next_range_u32_inclusive(0, 400), 213);
    one_draw.next_range_u32_inclusive(0, 2);
    assert_eq!(fatal_rng, one_draw.state(), "nested wall draw is inline");

    let (survives, surviving_grid, surviving_entities, surviving_rng) = run(11, true);
    // No wall draw when the wall survives — only the attacker's reload jitter.
    assert_eq!(surviving_rng, {
        let mut expected = SimRng::new(1);
        expected.next_range_u32_inclusive(0, 2);
        expected.state()
    });
    assert_eq!(surviving_grid.cell(8, 5).overlay_id, Some(0));
    assert!(survives.wall_mutations.is_empty());
    assert!(survives.cell_target_detaches.is_empty());
    assert!(survives.immediate_uninit_ids.is_empty());
    assert_eq!(surviving_entities.get(10).unwrap().health.current, 1);
    assert_eq!(
        survives
            .fire_events
            .iter()
            .map(|event| (event.attacker_id, event.target))
            .collect::<Vec<_>>(),
        vec![(20, TargetKind::Cell(8, 5))],
        "HP=damage+1 must not expose death-wall or restored-target state"
    );

    let (ungated, ungated_grid, ungated_entities, ungated_rng) = run(10, false);
    assert_eq!(ungated_entities.get(10).unwrap().health.current, 0);
    assert_eq!(ungated_grid.cell(8, 5).overlay_id, Some(0));
    assert_eq!(ungated_rng, {
        let mut expected = SimRng::new(1);
        expected.next_range_u32_inclusive(0, 2);
        expected.state()
    });
    assert!(ungated.wall_mutations.is_empty());
    assert!(ungated.cell_target_detaches.is_empty());
}

#[test]
fn gsi_04_07_damage_retaliation_is_receiver_synchronous_and_uses_mission_override() {
    #[derive(Debug)]
    struct Outcome {
        health: u16,
        current: MissionId,
        suspended: MissionId,
        target: Option<TargetKind>,
        nav_com: Option<crate::sim::components::NavTargetRef>,
        suspended_nav_com: Option<crate::sim::components::NavTargetRef>,
        has_movement: bool,
        last_attacker: Option<u64>,
        fired: Vec<(u64, TargetKind)>,
        immediate_uninit: Vec<u64>,
    }

    fn run(
        mission: MissionType,
        source_present: bool,
        allied: bool,
        victim_health: u16,
    ) -> Outcome {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=SOURCE\n1=VICTIM\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Warheads]\n0=IncomingWH\n1=ReturnWH\n\
             [SOURCE]\nStrength=200\nArmor=heavy\n\
             [VICTIM]\nStrength=100\nArmor=heavy\nSpeed=6\nPrimary=ReturnGun\nCanRetaliate=yes\n\
             [IncomingGun]\nDamage=10\nRange=8\nWarhead=IncomingWH\n\
             [ReturnGun]\nDamage=1\nROF=50\nRange=8\nWarhead=ReturnWH\n\
             [IncomingWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [ReturnWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [Harvest]\nRetaliate=no\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("retaliation receiver fixture");
        let mut interner = test_interner();
        let source_owner = interner.intern("SourceHouse");
        let victim_owner = interner.intern("VictimHouse");
        let source_type = interner.intern("SOURCE");
        let victim_type = interner.intern("VICTIM");
        let incoming_wh = interner.intern("IncomingWH");
        let incoming_weapon = interner.intern("IncomingGun");

        let mut entities = EntityStore::new();
        let mut source = make_entity(1, "SOURCE", 6, 5, 200);
        source.owner = source_owner;
        source.type_ref = source_type;
        source.lifecycle.in_limbo = false;
        source.lifecycle.cell_marked = true;
        entities.insert(source);

        let mut victim = make_entity(2, "VICTIM", 8, 5, 100);
        victim.owner = victim_owner;
        victim.type_ref = victim_type;
        victim.health.current = victim_health;
        victim.lifecycle.in_limbo = false;
        victim.lifecycle.cell_marked = true;
        victim.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(mission),
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
        victim.navigation.nav_com = Some(crate::sim::components::NavTargetRef::cell(9, 5));
        victim.movement_target = Some(crate::sim::components::MovementTarget::default());
        entities.insert(victim);

        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            6,
            5,
            1,
            crate::sim::movement::locomotor::MovementLayer::Ground,
            None,
            crate::sim::occupancy::CellListInsertion::PrependNonBuilding,
        );
        occupancy.add(
            8,
            5,
            2,
            crate::sim::movement::locomotor::MovementLayer::Ground,
            None,
            crate::sim::occupancy::CellListInsertion::PrependNonBuilding,
        );
        let detonation = ProjectileDetonation {
            projectile_id: 77,
            source_id: if source_present { 1 } else { RAD_NO_ATTACKER },
            target: ProjectileTarget::Entity(2),
            impact: ProjectileCoord::new(8 * 256 + 128, 5 * 256 + 128, 0),
            payload: ProjectilePayload {
                base_damage: 10,
                warhead: incoming_wh,
                weapon: incoming_weapon,
                owner: source_owner,
            },
            reason: ProjectileDetonationReason::ReachedTarget,
        };
        let mut alliances = HouseAllianceMap::new();
        if allied {
            alliances
                .entry("VICTIMHOUSE".to_string())
                .or_default()
                .insert("SOURCEHOUSE".to_string());
            alliances
                .entry("SOURCEHOUSE".to_string())
                .or_default()
                .insert("VICTIMHOUSE".to_string());
        }
        let mut scenario_rng = SimRng::new(11);
        let mut main_rng = SimRng::new(13);
        let mut houses = BTreeMap::new();
        let handles =
            crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
        let result = tick_combat_with_fog_and_main_rng(
            &mut entities,
            &mut occupancy,
            &rules,
            &mut interner,
            Some(handles),
            None,
            &BTreeMap::new(),
            &mut houses,
            &[],
            &alliances,
            None,
            &mut BTreeMap::new(),
            None,
            None,
            None,
            0,
            100,
            0,
            &[2],
            &[detonation],
            &[],
            None,
            &[],
            &mut scenario_rng,
            &mut main_rng,
            None,
        );
        let victim = entities.get(2).expect("deferred storage keeps victim");
        Outcome {
            health: victim.health.current,
            current: victim.mission.current(),
            suspended: victim.mission.suspended(),
            target: victim.attack_target.as_ref().map(|target| target.target),
            nav_com: victim.navigation.nav_com,
            suspended_nav_com: victim.navigation.suspended_nav_com,
            has_movement: victim.movement_target.is_some(),
            last_attacker: victim.last_attacker_id,
            fired: result
                .fire_events
                .iter()
                .map(|event| (event.attacker_id, event.target))
                .collect(),
            immediate_uninit: result.immediate_uninit_ids,
        }
    }

    let live = run(MissionType::Guard, true, false, 100);
    assert_eq!(live.health, 90);
    assert_eq!(live.current, MissionId::from_known(MissionType::Attack));
    assert_eq!(live.suspended, MissionId::from_known(MissionType::Guard));
    assert_eq!(live.target, Some(TargetKind::Entity(1)));
    assert_eq!(live.nav_com, None);
    assert_eq!(
        live.suspended_nav_com,
        Some(crate::sim::components::NavTargetRef::cell(9, 5))
    );
    assert!(
        !live.has_movement,
        "NULL destination stops the represented path"
    );
    assert_eq!(live.last_attacker, None, "receiver hit bypasses Phase 6");
    assert_eq!(
        live.fired,
        vec![(2, TargetKind::Entity(1))],
        "the later live slot reads the inline Override and fires this pass"
    );

    for (blocked, expected_mission) in [
        (
            run(MissionType::Harvest, true, false, 100),
            MissionType::Harvest,
        ),
        (
            run(MissionType::Guard, false, false, 100),
            MissionType::Guard,
        ),
        (run(MissionType::Guard, true, true, 100), MissionType::Guard),
    ] {
        assert_eq!(blocked.current, MissionId::from_known(expected_mission));
        assert_eq!(blocked.health, 90);
        assert!(blocked.target.is_none());
        assert!(blocked.fired.is_empty());
        assert!(blocked.nav_com.is_some());
        assert!(blocked.has_movement);
        assert!(blocked.last_attacker.is_none());
    }
    let fatal = run(MissionType::Guard, true, false, 10);
    assert_eq!(fatal.health, 0);
    assert_eq!(fatal.target, None);
    assert!(fatal.fired.is_empty());
    assert_eq!(fatal.immediate_uninit, vec![2]);
}

#[test]
fn gsi_04_07_damage_retaliation_peek_rejects_limbo_attacker() {
    fn run(attacker_in_limbo: bool) -> (u16, bool, MissionId, MissionId, Option<TargetKind>) {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=SOURCE\n1=VICTIM\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Warheads]\n0=HitWH\n1=ReturnWH\n\
             [SOURCE]\nStrength=200\nArmor=heavy\n\
             [VICTIM]\nStrength=100\nArmor=heavy\nSpeed=6\nPrimary=ReturnGun\nCanRetaliate=yes\n\
             [ReturnGun]\nDamage=1\nRange=8\nWarhead=ReturnWH\n\
             [HitWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [ReturnWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("retaliation peek fixture");
        let mut interner = test_interner();
        let source_owner = interner.intern("SourceHouse");
        let victim_owner = interner.intern("VictimHouse");
        let source_type = interner.intern("SOURCE");
        let victim_type = interner.intern("VICTIM");
        let hit_wh = interner.intern("HitWH");

        let mut entities = EntityStore::new();
        let mut source = make_entity(1, "SOURCE", 6, 5, 200);
        source.owner = source_owner;
        source.type_ref = source_type;
        source.lifecycle.in_limbo = attacker_in_limbo;
        source.lifecycle.cell_marked = !attacker_in_limbo;
        entities.insert(source);

        let mut victim = make_entity(2, "VICTIM", 8, 5, 100);
        victim.owner = victim_owner;
        victim.type_ref = victim_type;
        victim.lifecycle.in_limbo = false;
        victim.lifecycle.cell_marked = true;
        victim.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Guard),
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
        entities.insert(victim);

        let event = EntityDamageEvent::area(2, 10, 0, 1, Some(source_owner), hit_wh);
        let mut occupancy = OccupancyGrid::new();
        let mut main_rng = SimRng::new(5);
        let mut scenario_rng = SimRng::new(7);
        let mut handled_deaths = Vec::new();
        let mut resources = BTreeMap::new();
        let mut houses = BTreeMap::new();
        let mut fatal_lifecycle = None;
        let mut sound_sink = None;
        let _ = commit_damage_events(
            &[event],
            &mut entities,
            &mut occupancy,
            &rules,
            &mut interner,
            &mut houses,
            &[],
            &HouseAllianceMap::new(),
            &mut main_rng,
            &mut scenario_rng,
            &mut handled_deaths,
            &mut resources,
            None,
            None,
            None,
            0,
            &mut fatal_lifecycle,
            &mut sound_sink,
        );
        let victim = entities.get(2).expect("victim survives");
        (
            victim.health.current,
            victim.was_attacked_by_enemy,
            victim.mission.current(),
            victim.mission.suspended(),
            victim.attack_target.as_ref().map(|target| target.target),
        )
    }

    let illegal = run(true);
    assert_eq!(illegal.0, 90, "ReceiveDamage still commits before the peek");
    assert!(illegal.1, "the surviving hostile-hit postlude still runs");
    assert_eq!(illegal.2, MissionId::from_known(MissionType::Guard));
    assert_eq!(illegal.3, MissionId::NONE);
    assert_eq!(illegal.4, None, "FIRE_ILLEGAL suppresses Override");

    let legal = run(false);
    assert_eq!(legal.0, 90);
    assert!(legal.1);
    assert_eq!(legal.2, MissionId::from_known(MissionType::Attack));
    assert_eq!(legal.3, MissionId::from_known(MissionType::Guard));
    assert_eq!(legal.4, Some(TargetKind::Entity(1)));
}

#[test]
fn gsi_04_07_damage_invulnerability_impact_precedes_warping_and_postlude() {
    use crate::sim::movement::teleport_movement::{TeleportPhase, TeleportState};
    use crate::sim::superweapon::invulnerability::{InvulnKind, InvulnerabilityState};

    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=SOURCE\n1=VICTIM\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [Warheads]\n0=HitWH\n\
         [SOURCE]\nStrength=100\nArmor=heavy\nSpeed=6\n\
         [VICTIM]\nStrength=100\nArmor=heavy\nSpeed=6\n\
         [HitWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("invulnerability impact fixture");
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let source_id = sim
        .spawn_object("SOURCE", "SourceHouse", 6, 5, 0, &rules, &heights)
        .expect("source spawns");
    let source_owner = sim.substrate.entities.get(source_id).unwrap().owner;
    let protected = [
        (8, InvulnKind::IronCurtain, false),
        (9, InvulnKind::ForceShield, false),
        (10, InvulnKind::IronCurtain, true),
    ]
    .map(|(rx, kind, warping)| {
        let id = sim
            .spawn_object("VICTIM", "VictimHouse", rx, 5, 0, &rules, &heights)
            .expect("protected victim spawns");
        let victim = sim.substrate.entities.get_mut(id).unwrap();
        victim.invulnerability = Some(InvulnerabilityState {
            start_frame: 0,
            duration_frames: 100,
            kind,
        });
        if warping {
            victim.teleport_state = Some(TeleportState {
                phase: TeleportPhase::Relocate,
                target_rx: 20,
                target_ry: 20,
                being_warped_ticks: 1,
            });
        }
        id
    });
    let healing_id = sim
        .spawn_object("VICTIM", "VictimHouse", 11, 5, 0, &rules, &heights)
        .expect("healing control spawns");
    let ignored_id = sim
        .spawn_object("VICTIM", "VictimHouse", 12, 5, 0, &rules, &heights)
        .expect("ignore-defenses control spawns");
    for id in [healing_id, ignored_id] {
        sim.substrate.entities.get_mut(id).unwrap().invulnerability = Some(InvulnerabilityState {
            start_frame: 0,
            duration_frames: 100,
            kind: InvulnKind::IronCurtain,
        });
    }
    sim.substrate
        .entities
        .get_mut(healing_id)
        .unwrap()
        .health
        .current = 90;
    let hit_wh = sim.interner.intern("HitWH");
    let hits = vec![
        EntityDamageEvent::area(protected[0], 10, 0, source_id, Some(source_owner), hit_wh),
        EntityDamageEvent::area(protected[1], 20, 0, source_id, Some(source_owner), hit_wh),
        EntityDamageEvent::area(protected[2], 30, 0, source_id, Some(source_owner), hit_wh),
        EntityDamageEvent::area(healing_id, -10, 0, source_id, Some(source_owner), hit_wh),
        EntityDamageEvent::direct_receiver(
            ignored_id,
            10,
            0,
            source_id,
            Some(source_owner),
            hit_wh,
            ReceiverCallFlags {
                ignore_defenses: true,
                arg6: false,
            },
        ),
    ];

    sim.commit_noncombat_aoe_hits(&rules, None, &hits);

    for id in protected {
        let victim = sim.substrate.entities.get(id).unwrap();
        assert_eq!(victim.health.current, 100);
        assert!(!victim.was_attacked_by_enemy);
        assert_eq!(victim.damage_smoke_system_id, None);
    }
    assert_eq!(
        sim.substrate
            .entities
            .get(healing_id)
            .unwrap()
            .health
            .current,
        100,
        "negative healing bypasses IC without an impact"
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(ignored_id)
            .unwrap()
            .health
            .current,
        90,
        "ignoreDefenses bypasses IC without an impact"
    );

    let effects = &sim.invulnerability_impact_effects;
    assert_eq!(effects.len(), 3);
    assert_eq!(
        effects
            .iter()
            .map(|effect| effect.target_id)
            .collect::<Vec<_>>(),
        protected,
        "the dedicated combat-light handoff preserves receiver order"
    );
    assert_eq!(
        effects
            .iter()
            .map(|effect| effect.doubled_damage)
            .collect::<Vec<_>>(),
        vec![20, 40, 60]
    );
    assert_eq!(
        effects
            .iter()
            .map(|effect| effect.flags)
            .collect::<Vec<_>>(),
        vec![1, 6, 1],
        "native selector flags distinguish IC from ForceShield"
    );
    for (index, effect) in effects.iter().enumerate() {
        assert_eq!(effect.warhead_ref, hit_wh);
        assert!(effect.force_create);
        assert_eq!(
            effect.coord,
            ProjectileCoord::new((8 + index as i32) * 256 + 128, 5 * 256 + 128, 0),
            "the helper receives the protected target coordinate, not source/impact"
        );
    }
}

#[test]
fn gsi_04_07_damage_receiver_smoke_creation_precedes_retaliation() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=SOURCE\n1=MTNK\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [Warheads]\n0=HitWH\n1=ReturnWH\n\
         [ParticleSystems]\n0=SparkSys\n1=SmallGreySSys\n\
         [SOURCE]\nStrength=200\nArmor=heavy\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=ReturnGun\nCanRetaliate=yes\nDamageParticleSystems=SparkSys,SmallGreySSys\n\
         [ReturnGun]\nDamage=1\nROF=50\nRange=8\nWarhead=ReturnWH\n\
         [HitWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [ReturnWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [SparkSys]\nBehavesLike=Spark\nLifetime=5\n\
         [SmallGreySSys]\nBehavesLike=Smoke\nLifetime=-1\nSpawns=no\n\
         [AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n",
    ))
    .expect("damage-Smoke receiver fixture");
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let source_id = sim
        .spawn_object("SOURCE", "SourceHouse", 6, 5, 0, &rules, &heights)
        .expect("source spawns");
    let victim_id = sim
        .spawn_object("MTNK", "VictimHouse", 8, 5, 0, &rules, &heights)
        .expect("Grizzly spawns");
    let source_owner = sim.substrate.entities.get(source_id).unwrap().owner;
    let victim_owner = sim.substrate.entities.get(victim_id).unwrap().owner;
    sim.houses.insert(
        source_owner,
        HouseState::new(source_owner, 0, None, false, 0, 10),
    );
    sim.houses.insert(
        victim_owner,
        HouseState::new(victim_owner, 1, None, false, 0, 10),
    );
    {
        let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
        victim.health = Health {
            current: 180,
            max: 300,
        };
        victim.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Guard),
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
    }
    let hit_wh = sim.interner.intern("HitWH");
    let before_order = sim.live_object_order_snapshot();
    let before_rng = sim.scenario_rng.logical_state();

    sim.commit_noncombat_aoe_hits(
        &rules,
        None,
        &[EntityDamageEvent::area(
            victim_id,
            60,
            0,
            source_id,
            Some(source_owner),
            hit_wh,
        )],
    );

    let victim = sim.substrate.entities.get(victim_id).unwrap();
    assert_eq!(victim.health.current, 120);
    assert_eq!(
        victim.mission.current(),
        MissionId::from_known(MissionType::Attack)
    );
    assert_eq!(
        victim.attack_target.as_ref().map(|target| target.target),
        Some(TargetKind::Entity(source_id)),
        "retaliation sees the receiver after synchronous smoke creation"
    );
    let system_id = victim
        .damage_smoke_system_id
        .expect("yellow crossing attaches a Smoke system");
    let system = sim.particle_systems().get(system_id).unwrap();
    assert_eq!(
        rules.particle_system_type(system.type_id).name,
        "SmallGreySSys"
    );
    assert_eq!(system.owner_entity, Some(victim_id));
    assert_eq!(system.attached_entity, None);
    assert_eq!(system.owner_house, None);
    assert_eq!(
        system.coords,
        glam::IVec3::new(8 * 256 + 128, 5 * 256 + 128, 0)
    );
    let mut expected_order = before_order;
    expected_order.push(system_id);
    assert_eq!(sim.live_object_order_snapshot(), expected_order);
    assert_eq!(
        sim.scenario_rng.logical_state(),
        before_rng,
        "reverse filtering leaves one Smoke choice, so RandomRanged(0,0) draws nothing"
    );

    sim.commit_noncombat_aoe_hits(
        &rules,
        None,
        &[EntityDamageEvent::area(
            victim_id,
            1,
            0,
            source_id,
            Some(source_owner),
            hit_wh,
        )],
    );
    assert_eq!(
        sim.particle_systems().len(),
        1,
        "live +0x310 suppresses duplicates"
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(victim_id)
            .unwrap()
            .damage_smoke_system_id,
        Some(system_id)
    );

    sim.commit_noncombat_aoe_hits(
        &rules,
        None,
        &[EntityDamageEvent::area(
            victim_id,
            -61,
            0,
            source_id,
            Some(source_owner),
            hit_wh,
        )],
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(victim_id)
            .unwrap()
            .health
            .current,
        180
    );
    assert!(
        sim.particle_systems().get(system_id).unwrap().done_spawning,
        "recovery above ConditionYellow invokes mark-only ParticleSystem Destroy"
    );
    assert_eq!(
        sim.substrate
            .entities
            .get(victim_id)
            .unwrap()
            .damage_smoke_system_id,
        Some(system_id),
        "the owner pointer remains until physical pointer expiry"
    );
    sim.retire_particle_system(system_id);
    sim.process_pending_delete();
    assert!(sim.particle_systems().get(system_id).is_none());
    assert_eq!(
        sim.substrate
            .entities
            .get(victim_id)
            .unwrap()
            .damage_smoke_system_id,
        None,
        "physical finalization clears the +0x310 pointer"
    );
}

#[test]
fn gsi_04_07_damage_ai_retaliation_keeps_higher_scored_current_target() {
    #[derive(Debug)]
    struct Outcome {
        current_score: i64,
        attacker_score: i64,
        mission: MissionId,
        suspended: MissionId,
        target: TargetKind,
        nav_com: Option<crate::sim::components::NavTargetRef>,
        suspended_nav_com: Option<crate::sim::components::NavTargetRef>,
    }

    fn run(current_id: u64, attacker_id: u64) -> Outcome {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n0=TANY\n1=E1\n\
             [VehicleTypes]\n0=HTNK\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Warheads]\n0=HitWH\n1=AP\n2=HollowPoint2\n3=SA\n\
             [General]\nDumbMyEffectivenessCoefficient=200\nDumbTargetEffectivenessCoefficient=200\nDumbTargetSpecialThreatCoefficient=200\nDumbTargetStrengthCoefficient=200\nDumbTargetDistanceCoefficient=-1\n\
             [HTNK]\nStrength=400\nArmor=heavy\nSpeed=6\nPrimary=120mm\nCanRetaliate=yes\n\
             [TANY]\nStrength=200\nArmor=flak\nPrimary=DoublePistols\nSpecialThreatValue=1\n\
             [E1]\nStrength=125\nArmor=none\nPrimary=M60\n\
             [120mm]\nDamage=90\nRange=5.75\nWarhead=AP\n\
             [DoublePistols]\nDamage=125\nRange=6\nWarhead=HollowPoint2\n\
             [M60]\nDamage=15\nRange=4\nWarhead=SA\n\
             [HitWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [AP]\nVerses=25%,25%,15%,75%,100%,100%,65%,45%,60%,60%,100%\n\
             [HollowPoint2]\nVerses=100%,100%,100%,0%,0%,0%,1%,1%,1%,1%,100%\n\
             [SA]\nVerses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("stock threat-score fixture");
        let mut interner = test_interner();
        let ai_owner = interner.intern("AI");
        let tanya_owner = interner.intern("TanyaHouse");
        let gi_owner = interner.intern("GiHouse");
        let htnk_type = interner.intern("HTNK");
        let tanya_type = interner.intern("TANY");
        let gi_type = interner.intern("E1");
        let hit_wh = interner.intern("HitWH");

        let mut entities = EntityStore::new();
        let mut victim = make_entity(10, "HTNK", 8, 5, 400);
        victim.owner = ai_owner;
        victim.type_ref = htnk_type;
        victim.lifecycle.in_limbo = false;
        victim.lifecycle.cell_marked = true;
        victim.attack_target = Some(AttackTarget::new(current_id));
        victim.navigation.nav_com = Some(crate::sim::components::NavTargetRef::cell(9, 5));
        victim.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Guard),
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
        entities.insert(victim);

        let mut tanya = make_entity(20, "TANY", 6, 5, 200);
        tanya.owner = tanya_owner;
        tanya.type_ref = tanya_type;
        tanya.category = EntityCategory::Infantry;
        tanya.lifecycle.in_limbo = false;
        tanya.lifecycle.cell_marked = true;
        entities.insert(tanya);

        let mut gi = make_entity(30, "E1", 10, 5, 125);
        gi.owner = gi_owner;
        gi.type_ref = gi_type;
        gi.category = EntityCategory::Infantry;
        gi.lifecycle.in_limbo = false;
        gi.lifecycle.cell_marked = true;
        entities.insert(gi);

        let current_score = crate::sim::combat::combat_targeting::calculate_ai_threat_score(
            &entities, 10, current_id, &rules, &interner, None,
        )
        .and_then(|score| crate::util::native_x87::X87Chop53::ftol_i64(score).ok())
        .expect("current target score");
        let attacker_score = crate::sim::combat::combat_targeting::calculate_ai_threat_score(
            &entities,
            10,
            attacker_id,
            &rules,
            &interner,
            None,
        )
        .and_then(|score| crate::util::native_x87::X87Chop53::ftol_i64(score).ok())
        .expect("attacker score");

        let mut houses = BTreeMap::new();
        houses.insert(ai_owner, HouseState::new(ai_owner, 0, None, false, 0, 10));
        let source_house = entities.get(attacker_id).expect("attacker").owner;
        let event = EntityDamageEvent::area(10, 1, 0, attacker_id, Some(source_house), hit_wh);
        let mut occupancy = OccupancyGrid::new();
        let mut main_rng = SimRng::new(5);
        let mut scenario_rng = SimRng::new(7);
        let mut handled_deaths = Vec::new();
        let mut resources = BTreeMap::new();
        let mut fatal_lifecycle = None;
        let mut sound_sink = None;
        let _ = commit_damage_events(
            &[event],
            &mut entities,
            &mut occupancy,
            &rules,
            &mut interner,
            &mut houses,
            &[],
            &HouseAllianceMap::new(),
            &mut main_rng,
            &mut scenario_rng,
            &mut handled_deaths,
            &mut resources,
            None,
            None,
            None,
            0,
            &mut fatal_lifecycle,
            &mut sound_sink,
        );
        let victim = entities.get(10).expect("victim retained");
        Outcome {
            current_score,
            attacker_score,
            mission: victim.mission.current(),
            suspended: victim.mission.suspended(),
            target: victim
                .attack_target
                .as_ref()
                .expect("target retained")
                .target,
            nav_com: victim.navigation.nav_com,
            suspended_nav_com: victim.navigation.suspended_nav_com,
        }
    }

    let keep_tanya = run(20, 30);
    assert_eq!(
        (keep_tanya.current_score, keep_tanya.attacker_score),
        (100_450, 100_300)
    );
    assert_eq!(
        keep_tanya.mission,
        MissionId::from_known(MissionType::Guard)
    );
    assert_eq!(keep_tanya.suspended, MissionId::NONE);
    assert_eq!(keep_tanya.target, TargetKind::Entity(20));
    assert_eq!(
        keep_tanya.nav_com,
        Some(crate::sim::components::NavTargetRef::cell(9, 5))
    );
    assert_eq!(keep_tanya.suspended_nav_com, None);

    let switch_to_tanya = run(30, 20);
    assert_eq!(
        (
            switch_to_tanya.current_score,
            switch_to_tanya.attacker_score
        ),
        (100_300, 100_450)
    );
    assert_eq!(
        switch_to_tanya.mission,
        MissionId::from_known(MissionType::Attack)
    );
    assert_eq!(
        switch_to_tanya.suspended,
        MissionId::from_known(MissionType::Guard)
    );
    assert_eq!(switch_to_tanya.target, TargetKind::Entity(20));
    assert_eq!(switch_to_tanya.nav_com, None);
    assert_eq!(
        switch_to_tanya.suspended_nav_com,
        Some(crate::sim::components::NavTargetRef::cell(9, 5))
    );
}

#[test]
fn gsi_04_07_damage_spawn_and_slave_managers_block_retaliation() {
    fn run(slave_manager_shaped: bool) -> (MissionId, MissionId, Option<TargetKind>, bool) {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n0=SLAV\n\
             [VehicleTypes]\n0=SOURCE\n1=CARRIER\n2=SLAVEMASTER\n\
             [AircraftTypes]\n0=HORNET\n\
             [BuildingTypes]\n\
             [Warheads]\n0=HitWH\n1=ReturnWH\n\
             [SOURCE]\nStrength=100\nArmor=heavy\n\
             [CARRIER]\nStrength=100\nArmor=heavy\nPrimary=ReturnGun\nSpawns=HORNET\nSpawnsNumber=1\n\
             [SLAVEMASTER]\nStrength=100\nArmor=heavy\nPrimary=ReturnGun\nEnslaves=SLAV\nSlavesNumber=1\n\
             [HORNET]\nStrength=75\nArmor=light\n\
             [SLAV]\nStrength=100\nArmor=none\n\
             [ReturnGun]\nDamage=1\nRange=8\nWarhead=ReturnWH\n\
             [HitWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [ReturnWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("manager identity fixture");
        let mut interner = test_interner();
        let source_owner = interner.intern("SourceHouse");
        let victim_owner = interner.intern("VictimHouse");
        let source_type = interner.intern("SOURCE");
        let victim_type_name = if slave_manager_shaped {
            "SLAVEMASTER"
        } else {
            "CARRIER"
        };
        let victim_type = interner.intern(victim_type_name);
        let hit_wh = interner.intern("HitWH");

        let mut entities = EntityStore::new();
        let mut source = make_entity(1, "SOURCE", 6, 5, 100);
        source.owner = source_owner;
        source.type_ref = source_type;
        source.lifecycle.in_limbo = false;
        source.lifecycle.cell_marked = true;
        entities.insert(source);

        let mut victim = make_entity(2, victim_type_name, 8, 5, 100);
        victim.owner = victim_owner;
        victim.type_ref = victim_type;
        victim.lifecycle.in_limbo = false;
        victim.lifecycle.cell_marked = true;
        victim.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Guard),
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
        if !slave_manager_shaped {
            victim.spawn_manager = crate::sim::spawn_manager::init_spawn_manager(
                rules.object("CARRIER").expect("carrier type"),
                &rules,
                &mut interner,
                0,
            );
            assert!(victim.spawn_manager.is_some(), "live SpawnManager fixture");
        } else {
            assert!(
                rules
                    .object("SLAVEMASTER")
                    .and_then(|object| object.enslaves.as_deref())
                    .is_some_and(|slave| rules.object_case_insensitive(slave).is_some()),
                "resolved Enslaves profile creates native SlaveManager"
            );
        }
        entities.insert(victim);

        let event = EntityDamageEvent::area(2, 1, 0, 1, Some(source_owner), hit_wh);
        let mut occupancy = OccupancyGrid::new();
        let mut main_rng = SimRng::new(5);
        let mut scenario_rng = SimRng::new(7);
        let mut handled_deaths = Vec::new();
        let mut resources = BTreeMap::new();
        let mut houses = BTreeMap::new();
        let mut fatal_lifecycle = None;
        let mut sound_sink = None;
        let _ = commit_damage_events(
            &[event],
            &mut entities,
            &mut occupancy,
            &rules,
            &mut interner,
            &mut houses,
            &[],
            &HouseAllianceMap::new(),
            &mut main_rng,
            &mut scenario_rng,
            &mut handled_deaths,
            &mut resources,
            None,
            None,
            None,
            0,
            &mut fatal_lifecycle,
            &mut sound_sink,
        );
        let victim = entities.get(2).expect("victim survives");
        assert_eq!(victim.health.current, 99);
        (
            victim.mission.current(),
            victim.mission.suspended(),
            victim.attack_target.as_ref().map(|target| target.target),
            victim.spawn_manager.is_some(),
        )
    }

    let carrier = run(false);
    assert_eq!(carrier.0, MissionId::from_known(MissionType::Guard));
    assert_eq!(carrier.1, MissionId::NONE, "no override archives Guard");
    assert_eq!(carrier.2, None);
    assert!(carrier.3, "rejection preserves the live SpawnManager");

    let slave_master = run(true);
    assert_eq!(slave_master.0, MissionId::from_known(MissionType::Guard));
    assert_eq!(
        slave_master.1,
        MissionId::NONE,
        "no override archives Guard"
    );
    assert_eq!(slave_master.2, None);
    assert!(!slave_master.3);
}

#[test]
fn gsi_04_07_damage_full_capture_manager_blocks_retaliation() {
    struct Outcome {
        health: u16,
        mission: MissionId,
        suspended: MissionId,
        target: Option<TargetKind>,
        links: Vec<u64>,
    }

    fn run(link_count: usize) -> Outcome {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=SOURCE\n1=LINK\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n0=YAPSYT\n\
             [Warheads]\n0=HitWH\n1=Controller\n\
             [SOURCE]\nStrength=100\nArmor=heavy\n\
             [LINK]\nStrength=100\nArmor=heavy\n\
             [YAPSYT]\nStrength=100\nArmor=heavy\nPrimary=MultipleMindControlTower\n\
             [MultipleMindControlTower]\nDamage=3\nRange=7\nWarhead=Controller\n\
             [Controller]\nMindControl=yes\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
             [HitWH]\nCellSpread=0\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Psychic Tower manager fixture");
        let mut interner = test_interner();
        let source_owner = interner.intern("SourceHouse");
        let victim_owner = interner.intern("VictimHouse");
        let source_type = interner.intern("SOURCE");
        let victim_type = interner.intern("YAPSYT");
        let link_type = interner.intern("LINK");
        let hit_wh = interner.intern("HitWH");

        let mut entities = EntityStore::new();
        let mut source = make_entity(1, "SOURCE", 6, 5, 100);
        source.owner = source_owner;
        source.type_ref = source_type;
        source.lifecycle.in_limbo = false;
        source.lifecycle.cell_marked = true;
        entities.insert(source);

        let mut victim = make_structure_entity(2, "YAPSYT", 8, 5, 100, 100);
        victim.owner = victim_owner;
        victim.type_ref = victim_type;
        victim.lifecycle.in_limbo = false;
        victim.lifecycle.cell_marked = true;
        victim.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Guard),
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
        let mut manager = crate::sim::capture_manager::init_capture_manager(
            rules.object("YAPSYT").expect("YAPSYT type"),
            &rules,
        )
        .expect("Primary MindControl weapon constructs CaptureManager");
        assert_eq!(manager.max_control, 3, "stock tower link limit");

        for offset in 0..link_count {
            let id = 10 + offset as u64;
            let mut controlled = make_entity(id, "LINK", 10 + offset as u16, 5, 100);
            controlled.owner = victim_owner;
            controlled.type_ref = link_type;
            controlled.mind_controlled = true;
            controlled.lifecycle.in_limbo = false;
            controlled.lifecycle.cell_marked = true;
            entities.insert(controlled);
            manager.link_controlled_entity(id);
        }
        victim.capture_manager = Some(manager);
        entities.insert(victim);

        let event = EntityDamageEvent::area(2, 1, 0, 1, Some(source_owner), hit_wh);
        let mut occupancy = OccupancyGrid::new();
        let mut main_rng = SimRng::new(5);
        let mut scenario_rng = SimRng::new(7);
        let mut handled_deaths = Vec::new();
        let mut resources = BTreeMap::new();
        let mut houses = BTreeMap::new();
        let mut fatal_lifecycle = None;
        let mut sound_sink = None;
        let _ = commit_damage_events(
            &[event],
            &mut entities,
            &mut occupancy,
            &rules,
            &mut interner,
            &mut houses,
            &[],
            &HouseAllianceMap::new(),
            &mut main_rng,
            &mut scenario_rng,
            &mut handled_deaths,
            &mut resources,
            None,
            None,
            None,
            0,
            &mut fatal_lifecycle,
            &mut sound_sink,
        );

        let victim = entities.get(2).expect("tower survives");
        Outcome {
            health: victim.health.current,
            mission: victim.mission.current(),
            suspended: victim.mission.suspended(),
            target: victim.attack_target.as_ref().map(|target| target.target),
            links: victim
                .capture_manager
                .as_ref()
                .expect("manager retained")
                .controlled_entity_ids
                .clone(),
        }
    }

    let full = run(3);
    assert_eq!(full.health, 99, "receiver still commits the hostile hit");
    assert_eq!(full.mission, MissionId::from_known(MissionType::Guard));
    assert_eq!(full.suspended, MissionId::NONE);
    assert_eq!(full.target, None, "full manager rejects Override");
    assert_eq!(full.links, vec![10, 11, 12]);

    let below_capacity = run(2);
    assert_eq!(below_capacity.health, 99);
    assert_eq!(
        below_capacity.mission,
        MissionId::from_known(MissionType::Attack)
    );
    assert_eq!(
        below_capacity.suspended,
        MissionId::from_known(MissionType::Guard)
    );
    assert_eq!(below_capacity.target, Some(TargetKind::Entity(1)));
    assert_eq!(below_capacity.links, vec![10, 11]);
}

#[test]
fn gsi_04_07_damage_repair_bullet_cellspread_zero_keeps_signed_area_record() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=REPAIRER\n1=TARGET\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [REPAIRER]\nStrength=100\nArmor=light\nPrimary=RepairBullet\n\
         [TARGET]\nStrength=180\nArmor=heavy\n\
         [RepairBullet]\nDamage=-50\nROF=80\nRange=1.8\nProjectile=Invisible\nSpeed=100\nWarhead=Mechanical\n\
         [Mechanical]\nVerses=0%,0%,0%,100%,100%,100%,0%,0%,0%,100%,100%\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("RepairBullet/Mechanical rules");
    let weapon = rules.weapon("RepairBullet").expect("RepairBullet");
    let warhead = rules.warhead("Mechanical").expect("Mechanical");
    assert_eq!(weapon.damage, -50);
    assert_eq!(warhead.cell_spread_f64, 0.0);

    let mut entities = EntityStore::new();
    let mut target = make_entity(10, "TARGET", 8, 5, 180);
    target.health.current = 100;
    entities.insert(target);
    let mut occupancy = OccupancyGrid::new();
    let mut interner = test_interner();
    let warhead_ref = interner.intern("Mechanical");
    let weapon_ref = interner.intern("RepairBullet");
    let detonation = ProjectileDetonation {
        projectile_id: 1,
        source_id: 77,
        target: ProjectileTarget::Entity(10),
        impact: ProjectileCoord::new(8 * 256 + 128, 5 * 256 + 128, 0),
        payload: ProjectilePayload {
            base_damage: weapon.damage,
            warhead: warhead_ref,
            weapon: weapon_ref,
            owner: interner.intern("Test"),
        },
        reason: ProjectileDetonationReason::ReachedTarget,
    };
    let mut scenario_rng = SimRng::new(9);
    let mut emitted = CombatEmit::default();
    let mut resources = BTreeMap::new();
    let mut inline_hooks = None;
    let handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
    emit_projectile_detonations(
        &[detonation],
        &mut entities,
        &occupancy,
        &rules,
        &mut interner,
        Some(handles),
        &mut resources,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        &HouseAllianceMap::new(),
        &mut scenario_rng,
        &mut inline_hooks,
        &mut emitted,
    );
    let mut expected =
        EntityDamageEvent::area(10, -50, 0, 77, Some(interner.intern("Test")), warhead_ref);
    expected.near_center_ic_isolation_eligible = true;
    assert_eq!(
        emitted.damage_events,
        vec![combat_aoe::AreaDamageReceiver::Entity(expected)],
        "CellSpread=0 still enters the center receiver scan with raw signed damage"
    );

    let mut main_rng = SimRng::new(3);
    let mut handled_deaths = Vec::new();
    let mut houses = BTreeMap::new();
    let mut fatal_lifecycle = None;
    let mut sound_sink = None;
    let (death, pings) = commit_area_damage_receivers(
        &emitted.damage_events,
        &mut entities,
        &mut occupancy,
        &rules,
        &mut interner,
        &mut houses,
        &[],
        &HouseAllianceMap::new(),
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        None,
        0,
        &mut fatal_lifecycle,
        &mut sound_sink,
    );
    assert_eq!(entities.get(10).unwrap().health.current, 150);
    assert!(death.despawned_ids.is_empty());
    assert!(pings.is_empty());
}

#[test]
fn gsi_04_07_damage_receiver_updates_grudge_before_retaliation() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[General]\nConditionRed=25%\n\
         [VehicleTypes]\n0=TARGET\n1=ZERO\n2=SOURCE\n\
         [Warheads]\n0=HitWH\n1=ZeroWH\n2=ReturnWH\n\
         [TARGET]\nStrength=1000\nCost=700\nArmor=heavy\nPrimary=ReturnGun\nCanRetaliate=yes\n\
         [ZERO]\nStrength=1000\nCost=700\nArmor=heavy\nPrimary=ReturnGun\nCanRetaliate=yes\n\
         [SOURCE]\nStrength=1000\nCost=300\nArmor=heavy\n\
         [HitWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [ZeroWH]\nVerses=100%,100%,100%,100%,100%,0%,100%,100%,100%,100%,100%\n\
         [ReturnGun]\nDamage=1\nROF=1\nRange=8\nWarhead=ReturnWH\n\
         [ReturnWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [Guard]\nRetaliate=yes\n",
    ))
    .expect("anger feedback rules parse");
    let mut entities = EntityStore::new();
    let mut source = make_entity_owned(1, "SOURCE", 7, 5, 1000, "B");
    source.lifecycle.in_limbo = false;
    entities.insert(source);
    let mut damaged = make_entity_owned(2, "TARGET", 5, 5, 1000, "A");
    damaged.lifecycle.in_limbo = false;
    entities.insert(damaged);
    let mut zero = make_entity_owned(3, "ZERO", 6, 5, 1000, "A");
    zero.lifecycle.in_limbo = false;
    entities.insert(zero);

    // `GameEntity::test_default` interns through the thread-local test
    // registry. Clone it only after constructing the fixture so every entity
    // type/owner ID resolves through the receiver's local registry too.
    let mut interner = test_interner();
    let victim_house = interner.intern("A");
    let source_house = interner.intern("B");
    let hit_wh = interner.intern("HitWH");
    let zero_wh = interner.intern("ZeroWH");

    let mut houses = BTreeMap::from([
        (
            victim_house,
            HouseState::new(victim_house, 0, None, false, 0, 10),
        ),
        (
            source_house,
            HouseState::new(source_house, 1, None, false, 0, 10),
        ),
    ]);
    let house_order = [victim_house, source_house];
    let threat_persistence = |houses: &BTreeMap<InternedId, HouseState>| {
        let victim = &houses[&victim_house];
        let snapshot = bincode::serialize(&(victim.grudge_scores.clone(), victim.enemy_house))
            .expect("threat state serializes");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&victim.grudge_scores.len(), &mut hasher);
        for (other, score) in &victim.grudge_scores {
            std::hash::Hash::hash(other, &mut hasher);
            std::hash::Hash::hash(score, &mut hasher);
        }
        std::hash::Hash::hash(&victim.enemy_house, &mut hasher);
        (snapshot, std::hash::Hasher::finish(&hasher))
    };
    let mut occupancy = OccupancyGrid::new();
    let mut main_rng = SimRng::new(5);
    let mut scenario_rng = SimRng::new(7);
    let mut handled_deaths = Vec::new();
    let mut resources = BTreeMap::new();
    let mut fatal_lifecycle = None;
    let mut sound_sink = None;
    let threat_before_zero = threat_persistence(&houses);
    let (zero_death, _) = commit_damage_events(
        &[EntityDamageEvent::area(
            3,
            500,
            0,
            1,
            Some(source_house),
            zero_wh,
        )],
        &mut entities,
        &mut occupancy,
        &rules,
        &mut interner,
        &mut houses,
        &house_order,
        &HouseAllianceMap::new(),
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        0,
        &mut fatal_lifecycle,
        &mut sound_sink,
    );
    assert_eq!(entities.get(3).unwrap().health.current, 1000);
    assert_eq!(threat_persistence(&houses), threat_before_zero);
    let victim = &houses[&victim_house];
    assert!(!victim.grudge_scores.contains_key(&source_house));
    assert_eq!(victim.enemy_house, None);
    assert_eq!(
        zero_death.receiver_stage_trace,
        [
            ReceiverStageTrace::HouseThreat {
                target_id: 3,
                delta: 0,
            },
            ReceiverStageTrace::ShouldRetaliate { target_id: 3 },
        ],
        "fresh zero feedback rescans before retaliation without materializing a sparse node"
    );

    let (death, _) = commit_damage_events(
        &[EntityDamageEvent::area(
            2,
            500,
            0,
            1,
            Some(source_house),
            hit_wh,
        )],
        &mut entities,
        &mut occupancy,
        &rules,
        &mut interner,
        &mut houses,
        &house_order,
        &HouseAllianceMap::new(),
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        0,
        &mut fatal_lifecycle,
        &mut sound_sink,
    );
    assert_eq!(entities.get(2).unwrap().health.current, 500);
    let victim = &houses[&victim_house];
    assert_eq!(victim.grudge_scores.get(&source_house), Some(&350));
    assert_eq!(victim.enemy_house, Some(source_house));
    assert_eq!(
        death.receiver_stage_trace,
        [
            ReceiverStageTrace::HouseThreat {
                target_id: 2,
                delta: 350,
            },
            ReceiverStageTrace::ShouldRetaliate { target_id: 2 },
        ],
        "the first nonzero feedback materializes its sparse node before retaliation"
    );
}

#[test]
fn gsi_04_07_damage_postmortem_stock_barrel_delay_and_nested_order() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n0=CAMISC02\n\
         [Warheads]\n0=OilExplosionWH\n1=Super\n2=BarrelWallWH\n\
         [OverlayTypes]\n0=TESTWALL\n\
         [CombatDamage]\nC4Warhead=Super\n\
         [CAMISC02]\nStrength=5\nArmor=concrete\nCanC4=no\nExplodes=yes\n\
         EligibleForDelayKill=yes\nDeathWeapon=BarrelExplosion\n\
         [OilExplosionWH]\nCellSpread=4\nPercentAtMax=.5\nCausesDelayKill=yes\n\
         DelayKillFrames=5\nDelayKillAtMax=7.0\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [Super]\nCellSpread=0\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [BarrelExplosion]\nDamage=200\nWarhead=BarrelWallWH\n\
         [BarrelWallWH]\nCellSpread=0\nWall=yes\nWallAbsoluteDestroyer=yes\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [TESTWALL]\nWall=yes\nArmor=concrete\nStrength=400\n",
    );
    let mut rules = RuleSet::from_ini(&ini).expect("stock-shaped PostMortem rules");
    let registry = OverlayTypeRegistry::from_ini(&ini, None);
    let mut sim = crate::sim::world::Simulation::new();
    sim.resolve_type_handles(&rules);
    let heights = BTreeMap::new();
    let center = sim
        .spawn_object("CAMISC02", "Neutral", 8, 5, 0, &rules, &heights)
        .expect("center barrel");
    let middle = sim
        .spawn_object("CAMISC02", "Neutral", 10, 5, 0, &rules, &heights)
        .expect("middle barrel");
    let edge = sim
        .spawn_object("CAMISC02", "Neutral", 12, 5, 0, &rules, &heights)
        .expect("edge barrel");
    sim.substrate
        .entities
        .get_mut(edge)
        .unwrap()
        .pending_c4_detonation = Some(crate::sim::components::PendingC4Detonation {
        start_frame: 0,
        duration_frames: 40,
        source_entity_id: Some(center),
    });
    sim.overlay_grid = Some(OverlayGrid::new(20, 12));
    sim.overlay_grid.as_mut().unwrap().place_overlay(8, 5, 0, 0);
    let oil_wh = sim.interner.intern("OilExplosionWH");

    sim.commit_noncombat_aoe_hits(
        &rules,
        Some(&registry),
        &[
            EntityDamageEvent::area(center, 10, 0, RAD_NO_ATTACKER, None, oil_wh),
            EntityDamageEvent::area(middle, 10, 512, RAD_NO_ATTACKER, None, oil_wh),
            EntityDamageEvent::area(edge, 10, 1024, RAD_NO_ATTACKER, None, oil_wh),
        ],
    );

    let pending = |sim: &crate::sim::world::Simulation, id| {
        sim.substrate
            .entities
            .get(id)
            .and_then(|entity| entity.pending_c4_detonation)
            .expect("qualifying fatal barrel becomes PostMortem")
    };
    assert_eq!(pending(&sim, center).duration_frames, 5);
    assert_eq!(pending(&sim, middle).duration_frames, 20);
    assert_eq!(pending(&sim, edge).duration_frames, 35);
    assert_eq!(
        pending(&sim, edge).source_entity_id,
        None,
        "center's exact-zero Destroy notification expires the retained source pointer"
    );
    for id in [center, middle, edge] {
        let barrel = sim.substrate.entities.get(id).unwrap();
        assert_eq!(barrel.health.current, 1);
        assert!(barrel.lifecycle.object_alive && !barrel.dying);
    }
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(8, 5).overlay_id,
        Some(0)
    );

    // IC/FS uses the Building wrapper and cancels the shared timer outright.
    crate::sim::superweapon::invulnerability::apply_invulnerability(
        sim.substrate.entities.get_mut(middle).unwrap(),
        0,
        30,
        crate::sim::superweapon::invulnerability::InvulnKind::IronCurtain,
    );
    assert!(
        sim.substrate
            .entities
            .get(middle)
            .unwrap()
            .pending_c4_detonation
            .is_none()
    );

    sim.session.binary_frame = 4;
    sim.tick_pending_building_detonation(center, &rules, Some(&registry));
    assert!(sim.substrate.entities.get(center).is_some());
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(8, 5).overlay_id,
        Some(0)
    );

    sim.session.binary_frame = 5;
    sim.tick_pending_building_detonation(center, &rules, Some(&registry));
    let expired = sim
        .substrate
        .entities
        .get(center)
        .expect("UnInit keeps physical storage until the pending-delete drain");
    assert_eq!(expired.health.current, 0);
    assert!(expired.dying && !expired.lifecycle.object_alive);
    assert!(!expired.in_logic_vector);
    assert!(!sim.substrate.occupancy.contains_entity(8, 5, center));
    assert!(sim.substrate.pending_delete.contains(&center));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(8, 5).overlay_id,
        None,
        "the barrel DeathWeapon removes the wall in the same expiry transaction"
    );
    sim.flush_pending_delete();
    assert!(sim.substrate.entities.get(center).is_none());
}

#[test]
fn gsi_04_07_damage_postmortem_exact_zero_callbacks_precede_restore() {
    use crate::sim::world::LifecycleTestEvent;

    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=SOURCE\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n0=BARREL\n\
         [Warheads]\n0=DelayWH\n\
         [SOURCE]\nStrength=100\nArmor=heavy\nCost=100\n\
         [BARREL]\nStrength=5\nArmor=concrete\nCost=700\nCanC4=yes\n\
         EligibleForDelayKill=yes\n\
         [DelayWH]\nCellSpread=1\nPercentAtMax=1\nCausesDelayKill=yes\n\
         DelayKillFrames=5\nDelayKillAtMax=1\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("PostMortem callback rules");
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let source_id = sim
        .spawn_object("SOURCE", "SourceHouse", 6, 5, 0, &rules, &heights)
        .expect("source spawns");
    let target_id = sim
        .spawn_object("BARREL", "VictimHouse", 8, 5, 0, &rules, &heights)
        .expect("eligible target spawns");
    let source_owner = sim.substrate.entities.get(source_id).unwrap().owner;
    let victim_owner = sim.substrate.entities.get(target_id).unwrap().owner;
    sim.houses.insert(
        source_owner,
        HouseState::new(source_owner, 0, None, false, 0, 10),
    );
    sim.houses.insert(
        victim_owner,
        HouseState::new(victim_owner, 1, None, false, 0, 10),
    );
    sim.session.house_order.extend([source_owner, victim_owner]);
    let arm_source_target = |sim: &mut crate::sim::world::Simulation| {
        let source = sim.substrate.entities.get_mut(source_id).unwrap();
        source.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Attack),
            suspended: MissionId::from_known(MissionType::Guard),
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
        source.attack_target = Some(AttackTarget::new(target_id));
    };
    arm_source_target(&mut sim);
    sim.substrate.entities.get_mut(target_id).unwrap().selected = true;
    let delay_wh = sim.interner.intern("DelayWH");

    let hit = |sim: &mut crate::sim::world::Simulation| {
        sim.commit_noncombat_aoe_hits(
            &rules,
            None,
            &[EntityDamageEvent::area(
                target_id,
                10,
                0,
                source_id,
                Some(source_owner),
                delay_wh,
            )],
        );
    };
    hit(&mut sim);

    let pending = sim
        .substrate
        .entities
        .get(target_id)
        .unwrap()
        .pending_c4_detonation
        .expect("PostMortem arms the shared timer after callbacks");
    assert_eq!(pending.start_frame, 0);
    assert_eq!(pending.duration_frames, 5);
    assert_eq!(pending.source_entity_id, None);
    let target = sim.substrate.entities.get(target_id).unwrap();
    assert_eq!(target.health.current, 1);
    assert!(target.lifecycle.object_alive && !target.lifecycle.in_limbo);
    assert!(target.in_logic_vector && target.lifecycle.cell_marked);
    assert!(
        !target.selected,
        "ObjectClass::Destroy(1) deselects before detach"
    );
    assert!(sim.substrate.occupancy.contains_entity(8, 5, target_id));
    assert!(!sim.substrate.pending_delete.contains(&target_id));
    assert_eq!(
        target.killed_by, None,
        "the synchronous callback consumes attribution before HP1 restore"
    );
    let source = sim.substrate.entities.get(source_id).unwrap();
    assert_eq!(source.mission.current().known(), Some(MissionType::Guard));
    assert!(source.attack_target.is_none());
    assert_eq!(sim.houses[&victim_owner].stats.buildings_lost, 1);
    assert_eq!(sim.houses[&source_owner].stats.buildings_killed, 1);
    assert_eq!(sim.houses[&source_owner].stats.score_points, 700);

    let events = sim.lifecycle_test_events_for_test();
    let kill_index = events
        .iter()
        .position(|event| {
            *event
                == (LifecycleTestEvent::PostMortemKillBookkeeping {
                    stable_id: target_id,
                })
        })
        .expect("kill callback was traced");
    let destroy_index = events
        .iter()
        .position(|event| {
            *event
                == LifecycleTestEvent::PostMortemDestroyNotifyBoundary {
                    stable_id: target_id,
                }
        })
        .expect("Destroy notify boundary was traced");
    let radio_index = events
        .iter()
        .position(|event| {
            *event
                == (LifecycleTestEvent::PostMortemRadioBreakCompleted {
                    stable_id: target_id,
                })
        })
        .expect("Building Destroy broadcasts BREAK");
    let deselect_index = events
        .iter()
        .position(|event| {
            *event
                == (LifecycleTestEvent::PostMortemDeselected {
                    stable_id: target_id,
                })
        })
        .expect("Object Destroy deselects");
    let source_detach_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                LifecycleTestEvent::UninitRemovalListenerVisited {
                    expired_id,
                    listener_id,
                    target_alive: true,
                    target_in_limbo: false,
                } if *expired_id == target_id && *listener_id == source_id
            )
        })
        .expect("represented source receives pointer expiry");
    assert!(
        kill_index < radio_index
            && radio_index < deselect_index
            && deselect_index < destroy_index
            && destroy_index < source_detach_index
    );
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            LifecycleTestEvent::UninitClassPre { stable_id }
                | LifecycleTestEvent::UninitAliveCleared { stable_id }
                | LifecycleTestEvent::PendingDeleteQueued { stable_id }
                if *stable_id == target_id
        )
    }));

    // A later equal candidate is longer than the four frames remaining. Native
    // keeps the original timer bytes, but reruns Object's exact-zero callbacks.
    let original_pending = pending;
    sim.session.binary_frame = 1;
    arm_source_target(&mut sim);
    sim.clear_lifecycle_test_events_for_test();
    hit(&mut sim);
    let target = sim.substrate.entities.get(target_id).unwrap();
    assert_eq!(target.health.current, 1);
    assert_eq!(target.pending_c4_detonation, Some(original_pending));
    assert!(target.lifecycle.object_alive && target.in_logic_vector);
    assert!(sim.substrate.occupancy.contains_entity(8, 5, target_id));
    assert!(!sim.substrate.pending_delete.contains(&target_id));
    assert_eq!(sim.houses[&victim_owner].stats.buildings_lost, 2);
    assert_eq!(sim.houses[&source_owner].stats.buildings_killed, 2);
    assert_eq!(sim.houses[&source_owner].stats.score_points, 1_400);
    assert!(sim.lifecycle_test_events_for_test().iter().any(|event| {
        *event
            == (LifecycleTestEvent::PostMortemKillBookkeeping {
                stable_id: target_id,
            })
    }));
    assert!(
        sim.substrate
            .entities
            .get(source_id)
            .unwrap()
            .attack_target
            .is_none()
    );
}

#[test]
fn gsi_04_07_damage_postmortem_fresh_null_expiry_does_not_recredit_initial_killer() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=SOURCEA\n1=SOURCEB\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n0=BARREL\n\
         [Warheads]\n0=DelayWH\n1=OrdinaryWH\n2=Super\n\
         [CombatDamage]\nC4Warhead=Super\n\
         [SOURCEA]\nStrength=100\nArmor=heavy\n\
         [SOURCEB]\nStrength=100\nArmor=heavy\n\
         [BARREL]\nStrength=5\nArmor=concrete\nCost=700\nCanC4=yes\n\
         EligibleForDelayKill=yes\n\
         [DelayWH]\nCellSpread=1\nPercentAtMax=1\nCausesDelayKill=yes\n\
         DelayKillFrames=5\nDelayKillAtMax=1\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [OrdinaryWH]\nCellSpread=0\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [Super]\nCellSpread=0\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let mut rules = RuleSet::from_ini(&ini).expect("fresh PostMortem attribution rules");
    let mut sim = crate::sim::world::Simulation::new();
    sim.resolve_type_handles(&rules);
    let heights = BTreeMap::new();
    let source_a = sim
        .spawn_object("SOURCEA", "HouseA", 4, 5, 0, &rules, &heights)
        .expect("initial source spawns");
    let source_b = sim
        .spawn_object("SOURCEB", "HouseB", 5, 5, 0, &rules, &heights)
        .expect("later source spawns");
    let expiry_target = sim
        .spawn_object("BARREL", "VictimHouse", 8, 5, 0, &rules, &heights)
        .expect("expiry target spawns");
    let later_target = sim
        .spawn_object("BARREL", "VictimHouse", 10, 5, 0, &rules, &heights)
        .expect("later ordinary target spawns");
    let owner_a = sim.substrate.entities.get(source_a).unwrap().owner;
    let owner_b = sim.substrate.entities.get(source_b).unwrap().owner;
    let victim_owner = sim.substrate.entities.get(expiry_target).unwrap().owner;
    for (index, owner) in [owner_a, owner_b, victim_owner].into_iter().enumerate() {
        sim.houses.insert(
            owner,
            HouseState::new(owner, index as u8, None, false, 0, 10),
        );
        sim.session.house_order.push(owner);
    }
    let delay_wh = sim.interner.intern("DelayWH");
    let ordinary_wh = sim.interner.intern("OrdinaryWH");

    sim.commit_noncombat_aoe_hits(
        &rules,
        None,
        &[
            EntityDamageEvent::area(expiry_target, 10, 0, source_a, Some(owner_a), delay_wh),
            EntityDamageEvent::area(later_target, 10, 0, source_a, Some(owner_a), delay_wh),
        ],
    );
    assert_eq!(sim.houses[&owner_a].stats.buildings_killed, 2);
    assert_eq!(sim.houses[&owner_a].stats.score_points, 1_400);
    for target_id in [expiry_target, later_target] {
        let target = sim.substrate.entities.get(target_id).unwrap();
        assert_eq!(target.health.current, 1);
        assert_eq!(target.killed_by, None);
        assert_eq!(target.kill_award_points, 0);
    }

    // A different ordinary fatal transaction after restoration must not be
    // blocked by the consumed initial callback attribution.
    sim.session.binary_frame = 1;
    sim.commit_noncombat_aoe_hits(
        &rules,
        None,
        &[EntityDamageEvent::area(
            later_target,
            10,
            0,
            source_b,
            Some(owner_b),
            ordinary_wh,
        )],
    );
    let later = sim.substrate.entities.get(later_target).unwrap();
    assert_eq!(later.health.current, 0);
    assert_eq!(later.killed_by, Some(owner_b));
    assert_eq!(sim.houses[&owner_b].stats.buildings_killed, 1);
    assert_eq!(sim.houses[&owner_b].stats.score_points, 700);

    let initial_killer_before_expiry = (
        sim.houses[&owner_a].stats.buildings_killed,
        sim.houses[&owner_a].stats.score_points,
    );
    sim.session.binary_frame = 5;
    sim.tick_pending_building_detonation(expiry_target, &rules, None);
    let expired = sim.substrate.entities.get(expiry_target).unwrap();
    assert_eq!(expired.health.current, 0);
    assert_eq!(expired.killed_by, None);
    assert!(sim.substrate.pending_delete.contains(&expiry_target));
    assert_eq!(
        (
            sim.houses[&owner_a].stats.buildings_killed,
            sim.houses[&owner_a].stats.score_points,
        ),
        initial_killer_before_expiry,
        "fresh PostMortem expiry packet is sourceless and cannot recredit HouseA"
    );
}

#[test]
fn gsi_04_07_damage_death_weapon_gate_selection_and_native_damage() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=FV\n1=NANRCT\n2=SLOTGATE\n3=DEFAULTED\n4=CURRENT\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [FV]\nStrength=200\nArmor=light\nPrimary=HoverMissile\nDeathWeapon=CRNuke\n\
         [NANRCT]\nStrength=1000\nArmor=concrete\nExplodes=yes\nDeathWeapon=NukePayload\nDeathWeaponDamageModifier=.5\n\
         [SLOTGATE]\nStrength=100\nArmor=light\nPrimary=Ordinary\nSecondary=SuicideGun\nDeathWeapon=SlotBoom\n\
         [DEFAULTED]\nStrength=601\nArmor=light\nExplodes=yes\n\
         [CURRENT]\nStrength=100\nArmor=light\nExplodes=yes\nPrimary=Ordinary\nDeathWeaponDamageModifier=.5\n\
         [HoverMissile]\nDamage=25\nWarhead=OrdinaryWH\n\
         [CRNuke]\nDamage=999\nWarhead=NukeWH\n\
         [NukePayload]\nDamage=600\nWarhead=NukeWH\n\
         [Ordinary]\nDamage=40\nWarhead=OrdinaryWH\n\
         [SuicideGun]\nDamage=40\nWarhead=OrdinaryWH\nSuicide=yes\n\
         [SlotBoom]\nDamage=225\nWarhead=SlotWH\n\
         [DefaultDeath]\nDamage=999\nWarhead=DefaultWH\n\
         [CombatDamage]\nDeathWeapon=DefaultDeath\n\
         [OrdinaryWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [NukeWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [SlotWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [DefaultWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("death-weapon producer rules");
    let mut interner = test_interner();

    assert_eq!(
        death_weapon_aoe(
            &rules,
            rules.object("FV").unwrap(),
            0,
            0,
            None,
            &mut interner,
        ),
        None,
        "DeathWeapon without Explodes/current Suicide is inert (stock FV shape)"
    );

    let (nanrct_damage, nanrct_wh, nanrct_weapon) = death_weapon_aoe(
        &rules,
        rules.object("NANRCT").unwrap(),
        0,
        0,
        None,
        &mut interner,
    )
    .unwrap();
    assert_eq!(nanrct_damage, 300, "ftol(600 * 0.5f) must be 300");
    assert_eq!(interner.resolve(nanrct_wh), "NukeWH");
    assert_eq!(interner.resolve(nanrct_weapon), "NukePayload");

    assert_eq!(
        death_weapon_aoe(
            &rules,
            rules.object("SLOTGATE").unwrap(),
            0,
            0,
            None,
            &mut interner,
        ),
        None,
        "ordinary current Primary does not admit the helper"
    );
    let selected_suicide = interner.intern("SuicideGun");
    let (suicide_damage, suicide_wh, suicide_weapon) = death_weapon_aoe(
        &rules,
        rules.object("SLOTGATE").unwrap(),
        0,
        0,
        Some(selected_suicide),
        &mut interner,
    )
    .unwrap();
    assert_eq!(suicide_damage, 225);
    assert_eq!(interner.resolve(suicide_wh), "SlotWH");
    assert_eq!(interner.resolve(suicide_weapon), "SlotBoom");

    let (current_damage, current_wh, current_weapon) = death_weapon_aoe(
        &rules,
        rules.object("CURRENT").unwrap(),
        0,
        0,
        None,
        &mut interner,
    )
    .unwrap();
    assert_eq!(current_damage, 20);
    assert_eq!(interner.resolve(current_wh), "OrdinaryWH");
    assert_eq!(interner.resolve(current_weapon), "Ordinary");

    let (default_damage, default_wh, default_weapon) = death_weapon_aoe(
        &rules,
        rules.object("DEFAULTED").unwrap(),
        0,
        0,
        None,
        &mut interner,
    )
    .unwrap();
    assert_eq!(
        default_damage, 300,
        "Rules fallback is ftol(Strength * 0.5)"
    );
    assert_eq!(interner.resolve(default_wh), "DefaultWH");
    assert_eq!(interner.resolve(default_weapon), "DefaultDeath");
}

#[test]
fn gsi_08_05_tick_combat_respects_the_jittered_cooldown() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 5, 5, 300));
    store.insert(make_entity(2, "MTNK", 8, 5, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);
    let mut main_rng = SimRng::new(1);

    let mut fire_once =
        |store: &mut EntityStore, interner: &mut StringInterner, rng: &mut SimRng| {
            tick_combat(
                store,
                &mut OccupancyGrid::new(),
                &rules,
                interner,
                &mut BTreeMap::new(),
                0u64,
                100,
                0u32,
                rng,
            );
        };

    // First shot fires immediately (cooldown=0).
    fire_once(&mut store, &mut interner, &mut main_rng);
    let h1: u16 = store.get(2).unwrap().health.current;

    // `TechnoClass::GetROF @ 0x006FCFA0` returns `ROF + RandomRanged(0, 2)`,
    // so a `ROF=50` weapon reloads in 50, 51 or 52 frames — the exact value is
    // drawn, which is why this test counts the countdown instead of hardcoding
    // a frame number.
    let cooldown = store
        .get(1)
        .unwrap()
        .attack_target
        .as_ref()
        .unwrap()
        .cooldown_ticks;
    assert!(
        (50..=52).contains(&cooldown),
        "ROF=50 must reload in 50..=52 frames, got {cooldown}"
    );

    // Every frame up to the last one leaves the target untouched.
    for _ in 0..cooldown - 1 {
        fire_once(&mut store, &mut interner, &mut main_rng);
    }
    assert_eq!(
        store.get(2).unwrap().health.current,
        h1,
        "no second shot before the countdown reaches zero"
    );
    assert_eq!(
        store
            .get(1)
            .unwrap()
            .attack_target
            .as_ref()
            .unwrap()
            .cooldown_ticks,
        1
    );

    // The next update decrements 1 -> 0 before the fire decision.
    fire_once(&mut store, &mut interner, &mut main_rng);
    assert!(
        store.get(2).unwrap().health.current < h1,
        "the shot lands on the frame the countdown clears"
    );
}

fn selected_death_sounds_for(
    rules: &RuleSet,
    owner_is_human: bool,
    main_rng: &mut SimRng,
) -> Vec<String> {
    let mut entities = EntityStore::new();
    entities.insert(make_entity_owned(2, "E1", 8, 5, 1, "Americans"));
    let mut interner = test_interner();
    let owner = test_intern("Americans");
    let mut houses = BTreeMap::from([(
        owner,
        HouseState::new(owner, 0, Some(owner), owner_is_human, 5_000, 10),
    )]);
    let mut scenario_rng = SimRng::new(0);
    let mut handled_deaths = Vec::new();
    let handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
    let effects = handle_entity_deaths(
        &mut entities,
        &mut OccupancyGrid::new(),
        rules,
        &mut interner,
        Some(handles),
        &mut houses,
        &[owner],
        &HouseAllianceMap::new(),
        main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &[2],
        &[],
        &mut BTreeMap::new(),
        None,
        None,
        None,
        &mut None,
        false,
        0,
        &mut None,
        &mut None,
    );

    effects
        .death_sounds
        .into_iter()
        .map(|(sound, _, _)| interner.resolve(sound).to_string())
        .collect()
}

#[test]
fn fatal_sound_selection_uses_human_voice_then_die_sound_main_draws() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "\
[InfantryTypes]\n0=E1\n\n\
[VehicleTypes]\n0=MTNK\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[E1]\nStrength=1\nArmor=none\nVoiceDie=VoiceA,VoiceB,VoiceC\nDieSound=DieA,DieB\n\n\
[MTNK]\nStrength=100\nArmor=heavy\nPrimary=Gun\n\n\
[Gun]\nDamage=10\nROF=1\nRange=5\nWarhead=WH\n\n\
[WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("death-sound rules parse");

    let mut entities = EntityStore::new();
    entities.insert(make_entity_owned(1, "MTNK", 5, 5, 100, "Soviet"));
    entities.insert(make_entity_owned(2, "E1", 8, 5, 1, "Americans"));
    let mut interner = test_interner();
    issue_attack_command(&mut entities, 1, 2, None, &interner);
    let owner = test_intern("Americans");
    let mut houses = BTreeMap::from([(
        owner,
        HouseState::new(owner, 0, Some(owner), true, 5_000, 10),
    )]);
    let mut sounds = Vec::new();
    let mut scenario_rng = SimRng::new(73);
    // The one attacker fires this tick, so the scenario stream advances by its
    // `GetROF` reload jitter (`RandomRanged(0, 2)` @ `0x006FD0B0`) and nothing
    // else — the death-sound choices themselves still draw only on the human
    // and main streams.
    let scenario_before = {
        let mut expected = scenario_rng.clone();
        expected.next_range_u32_inclusive(0, 2);
        expected.state()
    };
    let mut human_rng = SimRng::new(1);
    let handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
    tick_combat_with_fog_and_main_rng(
        &mut entities,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        Some(handles),
        None,
        &BTreeMap::new(),
        &mut houses,
        &[owner],
        &HouseAllianceMap::new(),
        Some(&mut sounds),
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0,
        100,
        0,
        &[1, 2],
        &[],
        &[],
        None,
        &[],
        &mut scenario_rng,
        &mut human_rng,
        None,
    );
    assert_eq!(
        sounds
            .iter()
            .filter_map(|event| match event {
                SimSoundEvent::EntityDied { die_sound_id, .. } => {
                    Some(interner.resolve(*die_sound_id))
                }
                _ => None,
            })
            .collect::<Vec<_>>(),
        ["VoiceC", "DieA"]
    );
    assert_eq!(
        scenario_rng.state(),
        scenario_before,
        "death-sound choices must not consume Scenario RNG beyond the reload jitter"
    );
    let mut two_draw_reference = SimRng::new(1);
    two_draw_reference.next_u32();
    two_draw_reference.next_u32();
    assert_eq!(human_rng.state(), two_draw_reference.state());

    let mut ai_rng = SimRng::new(1);
    assert_eq!(
        selected_death_sounds_for(&rules, false, &mut ai_rng),
        ["DieB"]
    );
    let mut one_draw_reference = SimRng::new(1);
    one_draw_reference.next_u32();
    assert_eq!(ai_rng.state(), one_draw_reference.state());
}

#[test]
fn fatal_sound_empty_lists_skip_draws_but_single_choices_still_draw() {
    let empty_rules = RuleSet::from_ini(&IniFile::from_str(
        "\
[InfantryTypes]\n0=E1\n\n\
[VehicleTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[E1]\nStrength=1\nArmor=none\nVoiceDie= , \nDieSound=\n",
    ))
    .expect("empty death-sound rules parse");
    let mut empty_rng = SimRng::new(1);
    let empty_before = empty_rng.state();
    assert!(selected_death_sounds_for(&empty_rules, true, &mut empty_rng).is_empty());
    assert_eq!(empty_rng.state(), empty_before);

    let single_rules = RuleSet::from_ini(&IniFile::from_str(
        "\
[InfantryTypes]\n0=E1\n\n\
[VehicleTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[E1]\nStrength=1\nArmor=none\nVoiceDie=OnlyVoice\nDieSound=OnlyDie\n",
    ))
    .expect("single death-sound rules parse");
    let mut single_rng = SimRng::new(1);
    assert_eq!(
        selected_death_sounds_for(&single_rules, true, &mut single_rng),
        ["OnlyVoice", "OnlyDie"]
    );
    let mut two_draw_reference = SimRng::new(1);
    two_draw_reference.next_u32();
    two_draw_reference.next_u32();
    assert_eq!(single_rng.state(), two_draw_reference.state());
}

#[test]
#[ignore = "WIP: combat-death entity removal not yet landed"]
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
    );

    assert_eq!(result.fire_events.len(), 1);
    let ev = &result.fire_events[0];
    assert_eq!(interner.resolve(ev.weapon_id), "MissileLauncher");
    assert_eq!(ev.weapon_slot, WeaponSlot::Secondary);
    assert_eq!(store.get(2).unwrap().health.current, 400);
    assert_eq!(result.projectile_spawns.len(), 1);
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
        &mut main_rng,
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
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
        &mut main_rng,
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
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);
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
        &[],
        None,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut main_rng,
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
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);
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
        &[],
        None,
        &mut main_rng,
    );

    let target_health = store.get(2).expect("target alive").health.current;
    assert_eq!(target_health, 300, "Hidden target should not be damaged");
    assert!(
        store.get(1).unwrap().attack_target.is_none(),
        "AttackTarget removed when target is not visible and no replacement exists"
    );
}

#[derive(Clone, Copy)]
enum PlayfieldRetargetBranch {
    DeadOrMissing,
    NewlyFriendly,
    Invisible,
}

fn run_playfield_retarget_branch(
    branch: PlayfieldRetargetBranch,
    require_playfield_membership: bool,
) -> u64 {
    let rules = test_rules();
    let mut store = EntityStore::new();
    store.insert(make_entity_owned(10, "MTNK", 5, 5, 300, "Americans"));
    let (current_hp, current_owner) = match branch {
        PlayfieldRetargetBranch::DeadOrMissing => (0, "Soviet"),
        PlayfieldRetargetBranch::NewlyFriendly => (300, "Allies"),
        PlayfieldRetargetBranch::Invisible => (300, "Soviet"),
    };
    store.insert(make_entity_owned(
        99,
        "MTNK",
        8,
        5,
        current_hp,
        current_owner,
    ));
    let mut false_candidate = make_entity_owned(20, "MTNK", 6, 5, 300, "Soviet");
    false_candidate.in_playfield = false;
    store.insert(false_candidate);
    let mut true_candidate = make_entity_owned(30, "MTNK", 7, 5, 300, "Soviet");
    true_candidate.in_playfield = true;
    store.insert(true_candidate);

    let mut interner = test_interner();
    issue_attack_command(&mut store, 10, 99, None, &interner);
    let mut fog = FogState::default();
    let american = test_intern("Americans");
    fog.mark_visible_for_owner(american, 6, 5);
    fog.mark_visible_for_owner(american, 7, 5);
    if !matches!(branch, PlayfieldRetargetBranch::Invisible) {
        fog.mark_visible_for_owner(american, 8, 5);
    }
    if matches!(branch, PlayfieldRetargetBranch::NewlyFriendly) {
        fog.alliances
            .entry("AMERICANS".to_string())
            .or_default()
            .insert("ALLIES".to_string());
        fog.alliances
            .entry("ALLIES".to_string())
            .or_default()
            .insert("AMERICANS".to_string());
    }

    let mut occupancy = OccupancyGrid::new();
    let mut resources = BTreeMap::new();
    let mut scenario_rng = SimRng::new(1);
    if require_playfield_membership {
        let handles = Some(crate::sim::type_handle_table::ResolvedRuleHandles::resolve(
            &rules,
            &mut interner,
        ));
        let mut main_rng = SimRng::new(0);
        tick_combat_with_fog_and_main_rng_with_terrain_area(
            &mut store,
            &mut occupancy,
            &rules,
            &mut interner,
            handles,
            Some(&fog),
            &BTreeMap::new(),
            &mut BTreeMap::new(),
            &[],
            &HouseAllianceMap::new(),
            None,
            &mut resources,
            None,
            None,
            None,
            None,
            false,
            true,
            0,
            100,
            0,
            &[],
            &BTreeSet::new(),
            &BTreeSet::new(),
            &[],
            &[],
            None,
            &[],
            &mut scenario_rng,
            &mut main_rng,
            None,
            None,
        );
    } else {
        // Public/headless adapter deliberately has no live MapClass authority.
        tick_combat_with_fog(
            &mut store,
            &mut occupancy,
            &rules,
            &mut interner,
            Some(&fog),
            &BTreeMap::new(),
            None,
            &mut resources,
            None,
            None,
            None,
            0,
            100,
            0,
            &[],
            None,
            &mut scenario_rng,
        );
    }

    match store
        .get(10)
        .and_then(|entity| entity.attack_target.as_ref())
        .map(|attack| attack.target)
        .expect("attacker retargets")
    {
        TargetKind::Entity(stable_id) => stable_id,
        TargetKind::Cell(_, _) => panic!("retarget must stay object-backed"),
    }
}

#[test]
fn techno_playfield_dead_target_reacquisition_rejects_false_candidate() {
    assert_eq!(
        run_playfield_retarget_branch(PlayfieldRetargetBranch::DeadOrMissing, false),
        20,
        "headless adapter preserves candidate admission without MapClass authority"
    );
    assert_eq!(
        run_playfield_retarget_branch(PlayfieldRetargetBranch::DeadOrMissing, true),
        30,
        "Evaluate_Candidate 0x006F7DB0 rejects stored +0x3D5=false"
    );
}

#[test]
fn techno_playfield_newly_friendly_reacquisition_rejects_false_candidate() {
    assert_eq!(
        run_playfield_retarget_branch(PlayfieldRetargetBranch::NewlyFriendly, false),
        20
    );
    assert_eq!(
        run_playfield_retarget_branch(PlayfieldRetargetBranch::NewlyFriendly, true),
        30
    );
}

#[test]
fn techno_playfield_invisible_target_reacquisition_rejects_false_candidate() {
    assert_eq!(
        run_playfield_retarget_branch(PlayfieldRetargetBranch::Invisible, false),
        20
    );
    assert_eq!(
        run_playfield_retarget_branch(PlayfieldRetargetBranch::Invisible, true),
        30
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
    let mut main_rng = SimRng::new(1);
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
        &[],
        None,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);
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
        &[],
        None,
        &mut main_rng,
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
[HE]\nCellSpread=2\nTiberium=yes\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
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

    let ore_ini =
        IniFile::from_str("[OverlayTypes]\n0=ORE\n[ORE]\nTiberium=yes\nChainReaction=yes\n");
    let ore_registry = OverlayTypeRegistry::from_ini(&ore_ini, None);
    let mut overlays = OverlayGrid::new(16, 16);
    overlays.place_overlay(8, 5, 0, 5);
    overlays.place_overlay(9, 5, 0, 2);

    let mut main_rng = SimRng::new(1);
    let result = tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut resource_nodes,
        Some(&mut overlays),
        Some(&ore_registry),
        None,
        0u64,
        100,
        0u32,
        &[],
        None,
        &mut main_rng,
    );

    // Combat emits TiberiumReductionRequests (applied later by World via the
    // shared cell reducer); it no longer mutates resource_nodes directly.
    // Damage=120 → ore_damage = 120/10 = 12 density levels at each cell within
    // CellSpread=2. Both ore cells (8,5) and (9,5) get a reduction request.
    let req_amount = |rx: u16, ry: u16| {
        result
            .tiberium_reduction_requests
            .iter()
            .find(|r| r.rx == rx && r.ry == ry)
            .map(|r| r.amount)
    };
    assert_eq!(
        req_amount(8, 5),
        Some(12),
        "target cell should get a 12-level reduction request"
    );
    assert_eq!(
        req_amount(9, 5),
        Some(12),
        "neighbor cell within CellSpread=2 should get a 12-level reduction request"
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

    let ore_ini =
        IniFile::from_str("[OverlayTypes]\n0=ORE\n[ORE]\nTiberium=yes\nChainReaction=yes\n");
    let ore_registry = OverlayTypeRegistry::from_ini(&ore_ini, None);
    let mut overlays = OverlayGrid::new(16, 16);
    overlays.place_overlay(8, 5, 0, 5);
    overlays.place_overlay(9, 5, 0, 5);

    let mut main_rng = SimRng::new(1);
    let result = tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut resource_nodes,
        Some(&mut overlays),
        Some(&ore_registry),
        None,
        0u64,
        100,
        0u32,
        &[],
        None,
        &mut main_rng,
    );

    // Combat emits a TiberiumReductionRequest (applied later by World). 105mm
    // damage=65 → ore_damage = 65/10 = 6 density levels. CellSpread=0 → only the
    // impact cell (8,5) gets a request; the adjacent cell (9,5) gets none.
    let center = result
        .tiberium_reduction_requests
        .iter()
        .find(|r| r.rx == 8 && r.ry == 5);
    assert_eq!(
        center.map(|r| r.amount),
        Some(6),
        "center cell should get a 6-level reduction request"
    );
    assert!(
        !result
            .tiberium_reduction_requests
            .iter()
            .any(|r| r.rx == 9 && r.ry == 5),
        "adjacent cell should get no request with CellSpread=0"
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

    let ore_ini =
        IniFile::from_str("[OverlayTypes]\n0=ORE\n[ORE]\nTiberium=yes\nChainReaction=yes\n");
    let ore_registry = OverlayTypeRegistry::from_ini(&ore_ini, None);
    let mut overlays = OverlayGrid::new(16, 16);
    overlays.place_overlay(8, 5, 0, 9);

    let mut main_rng = SimRng::new(1);
    let result = tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut resource_nodes,
        Some(&mut overlays),
        Some(&ore_registry),
        None,
        0u64,
        100,
        0u32,
        &[],
        None,
        &mut main_rng,
    );

    // Combat emits a TiberiumReductionRequest (applied later by World). M60
    // damage=25 → ore_damage = 25/10 = 2 density levels at the impact cell.
    let req = result
        .tiberium_reduction_requests
        .iter()
        .find(|r| r.rx == 8 && r.ry == 5);
    assert_eq!(
        req.map(|r| r.amount),
        Some(2),
        "should emit a 2-density-level reduction request (25/10=2)"
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

/// Build a Simulation with an ephemeral GAWALL overlay at `(rx, ry)`.
fn build_minimal_sim_with_gawall(rx: u16, ry: u16) -> (Simulation, RuleSet, OverlayTypeRegistry) {
    let ini = IniFile::from_str(wall_test_ini());
    let rules = RuleSet::from_ini(&ini).expect("wall rules parse");
    let registry = OverlayTypeRegistry::from_ini(&ini, None);

    let mut sim = Simulation::new();
    let mut grid = OverlayGrid::new(10, 10);
    // Place GAWALL (overlay_id=2). Initial frame = 0 (isolated, stage 0).
    grid.place_overlay(rx, ry, 2, 0);
    sim.overlay_grid = Some(grid);

    (sim, rules, registry)
}

#[test]
fn wall_warhead_damages_and_destroys_wall_overlay() {
    let (mut sim, rules, registry) = build_minimal_sim_with_gawall(5, 5);

    let initial_wall_entities = sim
        .substrate
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            rules
                .object(sim.interner.resolve(e.type_ref))
                .is_some_and(|o| o.wall)
        })
        .count();
    assert_eq!(initial_wall_entities, 0, "wall state is cell-owned");

    // Forced destruction (literal -1 bypasses the probabilistic gate).
    let events = [WallDamageEvent {
        rx: 5,
        ry: 5,
        damage: -1,
    }];
    sim.apply_wall_damage_events(&events, &registry);
    // Overlay cleared.
    let grid = sim
        .overlay_grid
        .as_ref()
        .expect("grid should still be present");
    assert!(
        grid.cell(5, 5).overlay_id.is_none(),
        "overlay should be cleared"
    );
    assert_eq!(
        sim.radar_terrain_dirty_cells,
        vec![
            (5, 5),
            (5, 3),
            (6, 4),
            (4, 4),
            (5, 4),
            (4, 6),
            (3, 5),
            (4, 5),
            (6, 6),
            (5, 7),
            (5, 6),
            (7, 5),
            (6, 5),
        ],
        "direct wall damage uses the terminal DestroyOverlay visit stencil",
    );

    // No persistent wall entity is created or removed.
    let remaining = sim
        .substrate
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            rules
                .object(sim.interner.resolve(e.type_ref))
                .is_some_and(|o| o.wall)
        })
        .count();
    assert_eq!(remaining, 0);
}

#[test]
fn crusher_driveover_destroys_wall_but_noncrusher_does_not() {
    // A `Crusher=yes` drive vehicle standing on a wall cell after ground movement
    // flattens the wall (gamemd movement-side PerCellProcess crush), taking no
    // damage itself; a non-crusher on the same cell leaves the wall intact.
    // GAWALL is both a BuildingType (Wall=yes ObjectType) and an OverlayType;
    // BFRT is a Crusher drive vehicle, MTNK a plain drive vehicle.
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n0=BFRT\n1=MTNK\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n0=GAWALL\n\
         [OverlayTypes]\n0=GASAND\n1=CYCL\n2=GAWALL\n\
         [GAWALL]\nStrength=400\nArmor=concrete\nWall=yes\nDamageLevels=4\n\
         [BFRT]\nCrusher=yes\nLocomotor={4A582741-9839-11D1-B709-00A024DDAFD1}\n\
         [MTNK]\nLocomotor={4A582741-9839-11D1-B709-00A024DDAFD1}\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("rules parse");
    let registry = OverlayTypeRegistry::from_ini(&ini, None);

    // Build a sim with a GAWALL overlay at (5,5) and one
    // vehicle of `veh_type` placed on that same cell, its crusher flag +
    // locomotor derived from the real ObjectType.
    let build = |veh_type: &str| -> Simulation {
        let mut sim = Simulation::new();
        let mut grid = OverlayGrid::new(10, 10);
        grid.place_overlay(5, 5, 2, 0); // GAWALL overlay_id=2
        sim.overlay_grid = Some(grid);

        let owner_id = sim.interner.intern("Test");
        let obj = rules.object(veh_type).expect("veh object");
        let veh_type_id = sim.interner.intern(veh_type);
        let mut veh = GameEntity::test_default(2, veh_type, "Test", 5, 5);
        veh.owner = owner_id;
        veh.type_ref = veh_type_id;
        veh.regular_crusher = obj.crusher;
        veh.omni_crusher = obj.omni_crusher;
        veh.locomotor =
            Some(crate::sim::movement::locomotor::LocomotorState::from_object_type(obj, 0, 0));
        veh.health = Health {
            current: 300,
            max: 300,
        };
        sim.substrate.entities.insert(veh);
        sim.substrate.entities.rebuild_owner_index();
        sim
    };

    let wall_present = |sim: &Simulation| -> bool {
        sim.overlay_grid
            .as_ref()
            .unwrap()
            .cell(5, 5)
            .overlay_id
            .is_some()
    };

    // Crusher (BFRT): wall destroyed and crusher unharmed.
    let mut sim = build("BFRT");
    sim.apply_wall_crush_on_driveover(Some(&rules), Some(&registry));
    sim.flush_pending_delete();
    assert!(
        !wall_present(&sim),
        "crusher drive-over must remove the wall overlay"
    );
    assert_eq!(
        sim.radar_terrain_dirty_cells.len(),
        13,
        "crusher uses the terminal DestroyOverlay radar-dirty stencil",
    );
    assert_eq!(sim.radar_terrain_dirty_cells[0], (5, 5));
    assert!(
        sim.substrate
            .entities
            .get(2)
            .is_some_and(|e| e.health.current == 300),
        "crusher takes no damage from crushing the wall"
    );
    let walls_left = sim
        .substrate
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            rules
                .object(sim.interner.resolve(e.type_ref))
                .is_some_and(|o| o.wall)
        })
        .count();
    assert_eq!(walls_left, 0, "walls never create persistent entities");

    // Non-crusher (MTNK): wall stays intact.
    let mut sim = build("MTNK");
    sim.apply_wall_crush_on_driveover(Some(&rules), Some(&registry));
    sim.flush_pending_delete();
    assert!(
        wall_present(&sim),
        "a non-crusher drive vehicle must not remove the wall"
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
        sim.substrate.entities.insert(entity);
        next_id += 1;
    }
    sim.overlay_grid = Some(grid);
    sim.substrate.entities.rebuild_owner_index();

    (sim, rules, registry)
}

#[test]
fn concrete_wall_chain_reaction_runs_without_panic() {
    // Row of 4 GAWALL at (4..8, 5).
    let (mut sim, _rules, registry) = build_minimal_sim_with_gawall_row(5, 4..8);
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
    sim.apply_wall_damage_events(&events, &registry);

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
    sim.reseed_scenario_and_main(seed);
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
        let (mut sim, _rules, registry) = build_minimal_sim_with_gawall_seeded(5, 5, seed);
        sim.apply_wall_damage_events(&events, &registry);
        let cell = sim.overlay_grid.as_ref().unwrap().cell(5, 5);
        (cell.overlay_id, cell.overlay_data)
    };
    let snapshot_b: (Option<u8>, u8) = {
        let (mut sim, _rules, registry) = build_minimal_sim_with_gawall_seeded(5, 5, seed);
        sim.apply_wall_damage_events(&events, &registry);
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
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
fn gsi_04_11_death_weapon_anim_precedes_outer_detonation_anim() {
    // A Demo-Truck-style entity (Explodes=yes, primary warhead with its own
    // AnimList) is killed by a tank with a different warhead and AnimList.
    // ReceiveDamage synchronously completes the demo's UCEXPLOD death weapon;
    // only then does the outer Bullet detonation start TANKEXP.
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
    let mut main_rng = SimRng::new(1);

    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
        0u32,
        &mut main_rng,
    );

    let tankexp = interner.intern("TANKEXP");
    let ucexplod = interner.intern("UCEXPLOD");
    let ordered_anim_names: Vec<_> = result
        .smudge_spawn_requests
        .iter()
        .filter_map(|request| match request {
            SmudgeSpawnRequest::Anim { anim_name, .. } => Some(*anim_name),
            _ => None,
        })
        .collect();
    assert_eq!(ordered_anim_names, vec![ucexplod, tankexp]);
    assert_eq!(
        result
            .explosion_effects
            .iter()
            .map(|effect| effect.shp_name)
            .collect::<Vec<_>>(),
        vec![ucexplod, tankexp]
    );
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

fn inviso_weapon_rules(inviso: bool, with_anim: bool) -> RuleSet {
    let anim_list = if with_anim { "AnimList=PIFF\n" } else { "" };
    let ini = IniFile::from_str(&format!(
        "\
[InfantryTypes]\n\n\
[VehicleTypes]\n0=SHOOTER\n1=TARGET\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[SHOOTER]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=GUN\n\n\
[TARGET]\nStrength=500\nArmor=heavy\nSpeed=6\n\n\
[GUN]\nDamage=10\nROF=20\nRange=10\nProjectile=TESTPROJ\nWarhead=TESTWH\n\n\
[TESTPROJ]\nInviso={}\n\n\
[TESTWH]\n{}\
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        if inviso { "yes" } else { "no" },
        anim_list,
    ));
    RuleSet::from_ini(&ini).expect("Inviso test rules should parse")
}

fn persistent_projectile_rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\n[VehicleTypes]\n0=SHOOTER\n1=TARGET\n\n[AircraftTypes]\n\n[BuildingTypes]\n\n[SHOOTER]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=GUN\n\n[TARGET]\nStrength=500\nArmor=heavy\nSpeed=6\n\n[GUN]\nDamage=10\nROF=20\nRange=10\nSpeed=128\nProjectile=TESTPROJ\nWarhead=TESTWH\n\n[TESTPROJ]\nInviso=no\nImage=TESTBULLET\n\n[TESTWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("persistent projectile rules should parse")
}

#[test]
fn gsi_04_11_persistent_projectile_keeps_exact_lepton_z() {
    let rules = persistent_projectile_rules();
    let mut entities = EntityStore::new();
    let mut shooter = make_entity(1, "SHOOTER", 5, 5, 300);
    shooter.position.z = 7;
    shooter.position.exact_z_leptons = Some(733);
    entities.insert(shooter);
    let mut target = make_entity(2, "TARGET", 8, 5, 500);
    target.position.z = 11;
    target.position.exact_z_leptons = Some(1_177);
    entities.insert(target);
    let mut interner = test_interner();
    issue_attack_command(&mut entities, 1, 2, None, &interner);

    let result = tick_combat(
        &mut entities,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut SimRng::new(1),
    );

    assert_eq!(result.projectile_spawns.len(), 1);
    assert_eq!(result.projectile_spawns[0].origin.z, 733);
    assert_eq!(result.projectile_spawns[0].initial_target_position.z, 1_177);
}

#[test]
fn persistent_projectile_delays_damage_across_save_load_continuation() {
    let rules = persistent_projectile_rules();
    assert!(matches!(
        classify_projectile_delivery(rules.weapon("GUN").unwrap(), &rules),
        ProjectileDelivery::Persistent { .. }
    ));
    let mut entities = EntityStore::new();
    entities.insert(make_entity(1, "SHOOTER", 5, 5, 300));
    entities.insert(make_entity(2, "TARGET", 8, 5, 500));
    let mut interner = test_interner();
    issue_attack_command(&mut entities, 1, 2, None, &interner);

    let mut scenario_rng = SimRng::new(1);
    let fire = tick_combat(
        &mut entities,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut scenario_rng,
    );
    assert_eq!(entities.get(2).unwrap().health.current, 500);
    assert_eq!(fire.projectile_spawns.len(), 1);

    let mut sim = crate::sim::world::Simulation::new();
    let projectile_id = sim.allocate_stable_id();
    sim.admit_projectile(projectile_id, fire.projectile_spawns[0]);
    let target_positions =
        BTreeMap::from([(2, ProjectileCoord::new(8 * 256 + 128, 5 * 256 + 128, 0))]);
    let shared_cell_dummy = sim.effective_shared_cell_dummy();
    assert!(
        sim.projectiles
            .advance(&target_positions, None, &shared_cell_dummy, |_, _| None,)
            .detonations
            .is_empty()
    );

    let snapshot = crate::sim::snapshot::GameSnapshot::save(&sim, 0, 0, "projectile-flight", 0);
    let mut restored = crate::sim::snapshot::GameSnapshot::load(&snapshot)
        .expect("pending projectile snapshot should load")
        .sim;
    let restored_shared_cell_dummy = restored.effective_shared_cell_dummy();
    let mut detonations = Vec::new();
    for _ in 0..8 {
        detonations = restored
            .projectiles
            .advance(
                &target_positions,
                None,
                &restored_shared_cell_dummy,
                |_, _| None,
            )
            .detonations;
        if !detonations.is_empty() {
            break;
        }
    }
    assert_eq!(
        detonations.len(),
        1,
        "resumed projectile should reach target"
    );

    entities.remove(1);
    let mut main_rng = SimRng::new(1);
    let mut houses = BTreeMap::new();
    let handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
    tick_combat_with_fog_and_main_rng(
        &mut entities,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        Some(handles),
        None,
        &BTreeMap::new(),
        &mut houses,
        &[],
        &HouseAllianceMap::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        1,
        100,
        1,
        &[2],
        &detonations,
        &[],
        None,
        &[],
        &mut scenario_rng,
        &mut main_rng,
        None,
    );
    assert_eq!(entities.get(2).unwrap().health.current, 490);
}

fn explosion_coord(effect: &ExplosionEffect) -> (u16, u16, SimFixed, SimFixed) {
    (effect.rx, effect.ry, effect.sub_x, effect.sub_y)
}

#[test]
fn inviso_scatter_uses_scenario_rng_only_for_effect_and_paired_smudge() {
    let rules = inviso_weapon_rules(true, true);
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "SHOOTER", 5, 5, 300));
    store.insert(make_entity(2, "TARGET", 8, 5, 500));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);
    let target_coord = (
        store.get(2).unwrap().position.rx,
        store.get(2).unwrap().position.ry,
        store.get(2).unwrap().position.sub_x,
        store.get(2).unwrap().position.sub_y,
    );

    let mut scenario_rng = SimRng::new(1);
    let mut expected_rng = scenario_rng.clone();
    let expected_effect = inviso_scatter::scatter_inviso_effect_coord(
        &mut expected_rng,
        target_coord.0,
        target_coord.1,
        target_coord.2,
        target_coord.3,
    );
    // `TechnoClass::GetROF @ 0x006FCFA0` draws its `RandomRanged(0, 2)` reload
    // jitter after the shot, on this same instance (`[0x00A8B230] + 0x218` —
    // the one `FootClass::Mission_Attack @ 0x004D4DC0` also uses).
    expected_rng.next_range_u32_inclusive(0, 2);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut scenario_rng,
    );

    assert_eq!(scenario_rng.logical_state(), expected_rng.logical_state());
    assert_eq!(store.get(2).unwrap().health.current, 490);
    assert_eq!(result.explosion_effects.len(), 1);
    assert_eq!(
        explosion_coord(&result.explosion_effects[0]),
        expected_effect
    );
    assert_ne!(expected_effect, target_coord);
    assert!(
        result.tiberium_reduction_requests.is_empty(),
        "a non-Tiberium warhead without authoritative overlay context must not reduce ore"
    );
    match &result.smudge_spawn_requests[0] {
        SmudgeSpawnRequest::Anim {
            rx,
            ry,
            sub_x,
            sub_y,
            ..
        } => assert_eq!((*rx, *ry, *sub_x, *sub_y), expected_effect),
        other => panic!("expected paired animation smudge, got {other:?}"),
    }
}

#[test]
fn inviso_empty_animlist_still_consumes_one_draw() {
    let rules = inviso_weapon_rules(true, false);
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "SHOOTER", 5, 5, 300));
    store.insert(make_entity(2, "TARGET", 8, 5, 500));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    let mut scenario_rng = SimRng::new(1);
    let mut expected_rng = scenario_rng.clone();
    expected_rng.next_u32();
    // Plus the end-of-burst reload jitter, `GetROF @ 0x006FCFA0`.
    expected_rng.next_range_u32_inclusive(0, 2);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut scenario_rng,
    );

    assert_eq!(scenario_rng.logical_state(), expected_rng.logical_state());
    assert!(result.explosion_effects.is_empty());
    assert!(result.smudge_spawn_requests.is_empty());
}

#[test]
fn gsi_08_05_non_inviso_projectile_advances_scenario_rng_by_the_reload_jitter() {
    let rules = inviso_weapon_rules(false, true);
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "SHOOTER", 5, 5, 300));
    store.insert(make_entity(2, "TARGET", 8, 5, 500));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);
    let target_coord = (
        store.get(2).unwrap().position.rx,
        store.get(2).unwrap().position.ry,
        store.get(2).unwrap().position.sub_x,
        store.get(2).unwrap().position.sub_y,
    );

    let mut scenario_rng = SimRng::new(1);
    // A non-inviso shot takes no scatter draw, but it still reloads, and
    // `TechnoClass::GetROF @ 0x006FCFA0` draws `RandomRanged(0, 2)` for the
    // reload unconditionally. Exactly one draw, on the scenario instance.
    let mut expected_rng = scenario_rng.clone();
    expected_rng.next_range_u32_inclusive(0, 2);
    let result = tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0,
        100,
        0,
        &mut scenario_rng,
    );

    assert_eq!(scenario_rng.logical_state(), expected_rng.logical_state());
    assert_eq!(explosion_coord(&result.explosion_effects[0]), target_coord);
}

#[test]
fn two_inviso_attackers_consume_consecutive_draws_in_live_order() {
    let rules = inviso_weapon_rules(true, true);
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "SHOOTER", 5, 5, 300));
    store.insert(make_entity(2, "SHOOTER", 6, 5, 300));
    store.insert(make_entity(3, "TARGET", 8, 5, 500));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 3, None, &interner);
    issue_attack_command(&mut store, 2, 3, None, &interner);
    let target = store.get(3).unwrap();
    let target_coord = (
        target.position.rx,
        target.position.ry,
        target.position.sub_x,
        target.position.sub_y,
    );

    let mut scenario_rng = SimRng::new(1);
    let mut expected_rng = scenario_rng.clone();
    // Per attacker, in live order: the inviso scatter draws, then that
    // attacker's own `GetROF` reload jitter.
    let mut expected_shot = |rng: &mut SimRng| {
        let coord = inviso_scatter::scatter_inviso_effect_coord(
            rng,
            target_coord.0,
            target_coord.1,
            target_coord.2,
            target_coord.3,
        );
        rng.next_range_u32_inclusive(0, 2);
        coord
    };
    let expected = [
        expected_shot(&mut expected_rng),
        expected_shot(&mut expected_rng),
    ];
    let result = tick_combat_with_fog(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        None,
        &BTreeMap::<InternedId, PowerState>::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0,
        100,
        0,
        &[2, 1],
        None,
        &mut scenario_rng,
    );

    assert_eq!(
        result
            .fire_events
            .iter()
            .map(|event| event.attacker_id)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert_eq!(scenario_rng.logical_state(), expected_rng.logical_state());
    assert_eq!(result.explosion_effects.len(), 2);
    assert_eq!(explosion_coord(&result.explosion_effects[0]), expected[0]);
    assert_eq!(explosion_coord(&result.explosion_effects[1]), expected[1]);
}

// --- emit_warhead_detonation_effects helper tests ---------------------------

fn emit_helper_test_warhead(animlist: &[&str]) -> crate::rules::warhead_type::WarheadType {
    let animlist_csv = animlist.join(",");
    let ini_text = format!("[WH]\nFixtureOnly=1\nAnimList={}\n", animlist_csv);
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
        731,
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
            sub_x,
            sub_y,
            world_z_leptons,
        } => {
            assert_eq!(*anim_name, expected_id);
            assert_eq!(*rx, 5);
            assert_eq!(*ry, 5);
            assert_eq!(sub_x.to_num::<i32>(), 160);
            assert_eq!(sub_y.to_num::<i32>(), 96);
            assert_eq!(*world_z_leptons, 731);
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
        0,
        &mut interner,
        &mut explosions,
        &mut smudges,
    );
    assert_eq!(explosions[0].shp_name, interner.intern("EXP3"));
}

#[test]
fn combat_resolves_in_live_object_order_not_stable_id() {
    // Two attackers A (stable_id 1) and B (stable_id 2), same owner, both able to
    // lethally hit a shared enemy target T (stable_id 3) this tick. T has 50 HP, so
    // the single 58-damage 105mm shot (65 * 90% AP-vs-heavy) from the FIRST-resolved
    // attacker drops it to 0 and records the despawn.
    //
    // Phase 4 of tick_combat_with_fog applies damage_events in resolution order;
    // damage_events is built in Phase 2 by walking the snapshots in their sorted
    // order. fire_events is pushed in that same order, so fire_events[0].attacker_id
    // is exactly the first-resolved attacker. The new sort keys on live_order
    // position (stable_id tiebreak), so passing live_order = [2, 1] must make B
    // resolve first, and live_order = &[] must fall back to stable-id order (A first).
    fn build() -> (EntityStore, RuleSet, StringInterner) {
        let rules = test_rules();
        let mut store = EntityStore::new();
        // A and B are co-located is irrelevant; both are in range (<= 6) of T.
        store.insert(make_entity(1, "MTNK", 5, 5, 300)); // attacker A
        store.insert(make_entity(2, "MTNK", 6, 5, 300)); // attacker B
        store.insert(make_entity(3, "MTNK", 5, 6, 50)); // shared target T
        let interner = test_interner();
        issue_attack_command(&mut store, 1, 3, None, &interner);
        issue_attack_command(&mut store, 2, 3, None, &interner);
        (store, rules, interner)
    }
    let mut main_rng = SimRng::new(1);

    // Run 1: live order [B(2), A(1)] (reversed vs stable-id). B resolves first:
    // it fires first and lands the lethal shot.
    {
        let (mut store, rules, mut interner) = build();
        let result = tick_combat_with_fog(
            &mut store,
            &mut OccupancyGrid::new(),
            &rules,
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
            &[2, 1],
            None,
            &mut main_rng,
        );
        assert_eq!(
            result.fire_events[0].attacker_id, 2,
            "live order [2,1]: B (live-order-first) must fire first"
        );
        assert!(
            result.despawned_ids.contains(&3),
            "target must die this tick"
        );
    }

    // Run 2: empty live order falls back to stable-id order [A(1), B(2)]. A now
    // resolves first and fires first - the OPPOSITE of run 1 - proving live_order
    // controls the resolution sequence and that &[] reproduces the prior order.
    {
        let (mut store, rules, mut interner) = build();
        let result = tick_combat_with_fog(
            &mut store,
            &mut OccupancyGrid::new(),
            &rules,
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
            &[],
            None,
            &mut main_rng,
        );
        assert_eq!(
            result.fire_events[0].attacker_id, 1,
            "empty live order: stable-id fallback fires A first"
        );
        assert!(
            result.despawned_ids.contains(&3),
            "target must die this tick"
        );
    }
}

/// Slice 6 parity: a resolvable-but-`Dying` `last_attacker_id` (attacker present
/// with health 0) yields the same retaliation outcome as a freed/absent attacker —
/// both leave the victim with no `attack_target`. The deferred-delete window makes a
/// dead attacker resolvable by id where it used to be `None`; the retaliation gate
/// (`health > 0`) must treat the two identically. A live-attacker control branch
/// proves the test is non-vacuous (retaliation DOES fire when the attacker lives).
#[test]
fn dying_attacker_retaliation_matches_absent_attacker() {
    fn retal_rules() -> RuleSet {
        let ini_str: &str = "\
[VehicleTypes]\n0=MTNK\n1=TARGV\n\n\
[InfantryTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[TARGV]\nStrength=200\nArmor=none\nSpeed=5\n\n\
[105mm]\nDamage=65\nROF=20\nRange=6\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
        let ini = IniFile::from_str(ini_str);
        RuleSet::from_ini(&ini).expect("retal rules parse")
    }

    // Victim: armed MTNK last hit by attacker id 2; idle (no attack_target / order).
    fn victim() -> GameEntity {
        let mut v = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        v.health = Health {
            current: 300,
            max: 300,
        };
        v.last_attacker_id = Some(2);
        v
    }

    let rules = retal_rules();
    // Force the type/owner strings into the thread-local test interner BEFORE
    // snapshotting it, so resolve() of the test_default ids never indexes OOB.
    let _ = (
        test_intern("MTNK"),
        test_intern("Americans"),
        test_intern("TARGV"),
        test_intern("Russia"),
    );
    let interner = test_interner();
    let live_order = [1u64];

    // Branch A: attacker absent (already freed) — get(2) == None.
    let mut store_absent = EntityStore::new();
    store_absent.insert(victim());
    tick_retaliation(&mut store_absent, &rules, &interner, &live_order);

    // Branch B: attacker present but Dying (health 0) — the deferred-delete window.
    let mut store_dying = EntityStore::new();
    store_dying.insert(victim());
    let mut dead = GameEntity::test_default(2, "TARGV", "Russia", 6, 5);
    dead.health = Health {
        current: 0,
        max: 200,
    };
    dead.dying = true;
    store_dying.insert(dead);
    tick_retaliation(&mut store_dying, &rules, &interner, &live_order);

    // Parity: identical victim outcome — no retaliation issued in either case.
    let va = store_absent.get(1).unwrap();
    let vb = store_dying.get(1).unwrap();
    assert!(
        va.attack_target.is_none(),
        "absent-attacker: no retaliation"
    );
    assert!(vb.attack_target.is_none(), "dying-attacker: no retaliation");
    assert_eq!(
        va.last_attacker_id, vb.last_attacker_id,
        "dying attacker must leave the same last_attacker_id as an absent one",
    );

    // Control: a LIVE attacker DOES draw retaliation — proves the test isn't vacuous.
    let mut store_live = EntityStore::new();
    store_live.insert(victim());
    let mut live = GameEntity::test_default(2, "TARGV", "Russia", 6, 5);
    live.health = Health {
        current: 200,
        max: 200,
    };
    store_live.insert(live);
    tick_retaliation(&mut store_live, &rules, &interner, &live_order);
    let vc = store_live.get(1).unwrap();
    assert!(
        matches!(
            vc.attack_target.as_ref().map(|t| t.target),
            Some(TargetKind::Entity(2))
        ),
        "live attacker must draw retaliation (non-vacuous control)",
    );
    assert_eq!(
        vc.last_attacker_id, None,
        "retaliation against a live attacker clears last_attacker_id",
    );
}

// ---------------------------------------------------------------------------
// Radiation field (substrate Slice 7): periodic foot-unit damage through the
// [Radiation] RadSiteWarhead, building exemption, and the deployed
// self-irradiator (Desolator) re-fire loop.
// ---------------------------------------------------------------------------

/// Desolator-shaped rules: a deployable radiation infantry, a soft infantry
/// victim, a heavy-armor vehicle victim, and a building.
fn radiation_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "\
[General]\nVeteranArmor=1.5\n\n\
[InfantryTypes]\n0=DESO\n1=E2\n\n\
[VehicleTypes]\n0=MTNK\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n0=GAPOWR\n\n\
[DESO]\nStrength=200\nArmor=plate\nSpeed=4\nPrimary=RadBeamWeapon\nSecondary=RadEruptionWeapon\nDeployer=yes\nDeployFire=yes\nImmuneToRadiation=yes\n\n\
[E2]\nStrength=300\nArmor=none\nSpeed=4\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nVeteranAbilities=STRONGER\n\n\
[GAPOWR]\nStrength=750\nArmor=wood\n\n\
[RadBeamWeapon]\nDamage=25\nROF=70\nRange=6\nWarhead=RadSite\n\n\
[RadEruptionWeapon]\nDamage=1\nROF=60\nRange=4\nAreaFire=yes\nWarhead=RadEruptionWarhead\nRadLevel=500\nReport=DesolatorDeploy\n\n\
[RadEruptionWarhead]\nVerses=100%,100%,100%,20%,10%,10%,0%,0%,0%,100%,100%\nInfDeath=7\nRadiation=yes\nCellSpread=10\n\n\
[RadSite]\nVerses=100%,100%,100%,50%,10%,10%,0%,0%,0%,100%,100%\nInfDeath=7\nRadiation=yes\n\n\
[Radiation]\nRadDurationMultiple=1\nRadApplicationDelay=16\nRadLevelMax=500\nRadLevelDelay=90\nRadLevelFactor=.2\nRadSiteWarhead=RadSite\n",
    );
    RuleSet::from_ini(&ini).expect("radiation rules should parse")
}

/// One combat tick with the radiation field threaded through.
fn rad_combat_tick(
    sim: &mut crate::sim::world::Simulation,
    rules: &RuleSet,
    binary_frame: u32,
) -> CombatTickResult {
    let mut radiation = std::mem::take(&mut sim.radiation);
    let result = tick_combat_with_fog(
        &mut sim.substrate.entities,
        &mut sim.substrate.occupancy,
        rules,
        &mut sim.interner,
        None,
        &BTreeMap::new(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        0,
        100,
        binary_frame,
        &[],
        Some(&mut radiation),
        &mut sim.scenario_rng,
    );
    sim.radiation = radiation;
    result
}

/// Radiation damage applies only on `frame % RadApplicationDelay == 0`
/// boundaries, scaled by the RadSiteWarhead Verses per armor class:
/// trunc(min(500, 500) × 0.2) = 100 base → 100 vs none, 10 vs heavy.
#[test]
fn rad_damage_fires_on_application_delay_boundary_only() {
    let rules = radiation_rules();
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let inf = sim
        .spawn_object("E2", "Americans", 5, 5, 0, &rules, &heights)
        .expect("infantry spawns");
    let tank = sim
        .spawn_object("MTNK", "Americans", 6, 5, 0, &rules, &heights)
        .expect("tank spawns");
    sim.radiation.apply_detonation(
        crate::sim::radiation::RadDetonation {
            rx: 5,
            ry: 5,
            rad_level: 500,
            spread: 2,
        },
        0,
        &rules.radiation,
        None,
    );

    // Frame 15: not an application boundary — nobody takes damage.
    rad_combat_tick(&mut sim, &rules, 15);
    assert_eq!(sim.substrate.entities.get(inf).unwrap().health.current, 300);
    assert_eq!(
        sim.substrate.entities.get(tank).unwrap().health.current,
        300
    );

    // Frame 16: boundary. E2 stands on the center cell (level 500, clamped
    // 500): trunc(500 × 0.2) = 100, Verses none = 100% → 100 damage. The
    // tank is one cell out (falloff (640−256)/640 × 500 = 300): trunc(300 ×
    // 0.2) = 60, Verses heavy = 10% → 6 damage.
    rad_combat_tick(&mut sim, &rules, 16);
    let inf_hp = sim.substrate.entities.get(inf).unwrap().health.current;
    let tank_hp = sim.substrate.entities.get(tank).unwrap().health.current;
    assert_eq!(inf_hp, 200, "100 rad damage vs armor none");
    assert_eq!(tank_hp, 294, "6 rad damage vs heavy armor (10% Verses)");
    // Sourceless damage must not arm retaliation.
    assert_eq!(
        sim.substrate.entities.get(inf).unwrap().last_attacker_id,
        None
    );

    // Frame 17: off-boundary again.
    rad_combat_tick(&mut sim, &rules, 17);
    assert_eq!(sim.substrate.entities.get(inf).unwrap().health.current, 200);
}

#[test]
fn gsi_04_07_damage_periodic_radiation_enters_direct_receiver_once() {
    let rules = radiation_rules();
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let tank = sim
        .spawn_object("MTNK", "Americans", 5, 5, 0, &rules, &heights)
        .expect("veteran heavy target spawns");
    sim.substrate.entities.get_mut(tank).unwrap().veterancy = 100;
    sim.radiation.apply_detonation(
        crate::sim::radiation::RadDetonation {
            rx: 5,
            ry: 5,
            rad_level: 500,
            spread: 2,
        },
        0,
        &rules.radiation,
        None,
    );

    let result = rad_combat_tick(&mut sim, &rules, 16);
    let target = sim.substrate.entities.get(tank).unwrap();
    assert_eq!(
        target.health.current, 294,
        "raw 100 / VeteranArmor 1.5 = 66; ftol(66 x heavy 10%) = 6 once"
    );
    assert_eq!(target.last_attacker_id, None, "null attacker is retained");
    assert!(
        target.attack_target.is_none(),
        "periodic radiation cannot arm retaliation"
    );
    assert!(
        result.under_attack_events.is_empty(),
        "null source house cannot emit an enemy under-attack event"
    );
}

#[test]
fn gsi_04_07_damage_hostile_building_hit_latches_was_attacked_for_ai_repair() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[General]\n\
         FixtureOnly=1\n\
         [AI]\nCreditReserve=100\n\
         [IQ]\nMaxIQLevels=5\nRepairSell=2\nSellBack=2\n\
         [AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n\
         [InfantryTypes]\n\
         [VehicleTypes]\n0=MTNK\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n0=GAPOWR\n\
         [Warheads]\n0=HITWH\n\
         [MTNK]\nStrength=300\nArmor=heavy\n\
         [GAPOWR]\nStrength=1000\nArmor=wood\nCost=800\nCrewed=no\n\
         [HITWH]\nCellSpread=0\nPercentAtMax=1\nAffectsAllies=yes\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("hostile-hit rules");
    let mut sim = crate::sim::world::Simulation::new();
    let ai_owner = sim.interner.intern("AI");
    let enemy_owner = sim.interner.intern("ENEMY");
    let ally_owner = sim.interner.intern("ALLY");
    let scenario_ini = IniFile::from_str("[Houses]\n0=AI\n[AI]\nIQ=1\n");
    let scenario_houses =
        crate::map::houses::parse_house_roster(
            &scenario_ini,
            &rules.color_schemes,
            Some(&rules),
        );
    let mut ai_house = HouseState::new(ai_owner, 0, None, false, 0, 51);
    ai_house.current_iq =
        scenario_houses.houses[0].scenario_current_iq(rules.general.max_iq_levels);
    sim.houses.insert(ai_owner, ai_house);
    let heights = BTreeMap::new();
    let hostile_target = sim
        .spawn_object("GAPOWR", "AI", 5, 5, 0, &rules, &heights)
        .expect("hostile target");
    let allied_target = sim
        .spawn_object("GAPOWR", "AI", 7, 5, 0, &rules, &heights)
        .expect("allied target");
    let null_target = sim
        .spawn_object("GAPOWR", "AI", 9, 5, 0, &rules, &heights)
        .expect("null-source target");
    let hostile_source = sim
        .spawn_object("MTNK", "ENEMY", 5, 6, 0, &rules, &heights)
        .expect("hostile source");
    let allied_source = sim
        .spawn_object("MTNK", "ALLY", 7, 6, 0, &rules, &heights)
        .expect("allied source");
    for target_id in [hostile_target, allied_target, null_target] {
        sim.substrate
            .entities
            .get_mut(target_id)
            .unwrap()
            .health
            .current = 200;
    }
    let warhead_ref = sim.interner.intern("HITWH");
    let events = [
        EntityDamageEvent::area(
            hostile_target,
            10,
            0,
            hostile_source,
            Some(enemy_owner),
            warhead_ref,
        ),
        EntityDamageEvent::area(
            allied_target,
            10,
            0,
            allied_source,
            Some(ally_owner),
            warhead_ref,
        ),
        EntityDamageEvent::area(null_target, 10, 0, RAD_NO_ATTACKER, None, warhead_ref),
    ];
    let mut alliances = HouseAllianceMap::new();
    alliances
        .entry("AI".to_string())
        .or_default()
        .insert("ALLY".to_string());
    let mut main_rng = SimRng::new(3);
    let mut handled_deaths = Vec::new();
    let mut resources = BTreeMap::new();
    let mut fatal_lifecycle = None;
    let mut sound_sink = None;
    let _ = commit_damage_events(
        &events,
        &mut sim.substrate.entities,
        &mut sim.substrate.occupancy,
        &rules,
        &mut sim.interner,
        &mut sim.houses,
        &sim.session.house_order,
        &alliances,
        &mut main_rng,
        &mut sim.scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        0,
        &mut fatal_lifecycle,
        &mut sound_sink,
    );
    assert!(
        sim.substrate
            .entities
            .get(hostile_target)
            .unwrap()
            .was_attacked_by_enemy,
        "surviving hostile source sets the persistent Techno tail byte"
    );
    assert!(
        !sim.substrate
            .entities
            .get(allied_target)
            .unwrap()
            .was_attacked_by_enemy,
        "target-owner alliance suppresses the hostile latch"
    );
    assert!(
        !sim.substrate
            .entities
            .get(null_target)
            .unwrap()
            .was_attacked_by_enemy,
        "null radiation/environment source cannot set it"
    );
    let latched_hash = sim.state_hash();
    sim.substrate
        .entities
        .get_mut(hostile_target)
        .unwrap()
        .was_attacked_by_enemy = false;
    assert_ne!(
        sim.state_hash(),
        latched_hash,
        "the persistent byte is hashed"
    );
    sim.substrate
        .entities
        .get_mut(hostile_target)
        .unwrap()
        .was_attacked_by_enemy = true;

    let low_iq_rng = sim.scenario_rng.logical_state();
    crate::sim::production::tick_repairs(&mut sim, &rules);
    assert!(
        sim.substrate
            .entities
            .get(hostile_target)
            .unwrap()
            .lifecycle
            .object_alive,
        "scenario CurrentIQ 1 stays below RepairSell/SellBack 2"
    );
    assert_eq!(
        sim.scenario_rng.logical_state(),
        low_iq_rng,
        "an IQ-gated-out building draws no low-credit sale RNG"
    );

    sim.houses.get_mut(&ai_owner).unwrap().current_iq = 2;
    let mut expected_rng = sim.scenario_rng.clone();
    assert!(
        expected_rng.next_range_u32_inclusive(0, 0x32) < 51,
        "TechLevel 51 makes every inclusive native roll win"
    );
    crate::sim::production::tick_repairs(&mut sim, &rules);
    let sold = sim.substrate.entities.get(hostile_target).unwrap();
    assert!(!sold.lifecycle.object_alive && sold.lifecycle.in_limbo);
    assert!(
        sim.substrate
            .entities
            .get(allied_target)
            .unwrap()
            .lifecycle
            .object_alive
    );
    assert!(
        sim.substrate
            .entities
            .get(null_target)
            .unwrap()
            .lifecycle
            .object_alive
    );
    assert_eq!(
        sim.scenario_rng.logical_state(),
        expected_rng.logical_state(),
        "the single qualifying building consumes exactly one inclusive roll"
    );
}

/// Buildings never take radiation damage; an ImmuneToRadiation unit on the
/// same cell is also exempt.
#[test]
fn buildings_take_no_rad_damage() {
    let rules = radiation_rules();
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let building = sim
        .spawn_object("GAPOWR", "Americans", 5, 5, 0, &rules, &heights)
        .expect("building spawns");
    let deso = sim
        .spawn_object("DESO", "Americans", 6, 5, 0, &rules, &heights)
        .expect("desolator spawns");
    sim.radiation.apply_detonation(
        crate::sim::radiation::RadDetonation {
            rx: 5,
            ry: 5,
            rad_level: 500,
            spread: 2,
        },
        0,
        &rules.radiation,
        None,
    );

    rad_combat_tick(&mut sim, &rules, 16);
    assert_eq!(
        sim.substrate.entities.get(building).unwrap().health.current,
        750,
        "buildings are exempt from radiation damage"
    );
    assert_eq!(
        sim.substrate.entities.get(deso).unwrap().health.current,
        200,
        "ImmuneToRadiation units are exempt"
    );
}

/// A deployed Desolator force-fires its deploy weapon at its own cell when no
/// site exists there, arming a full-level site; while the site's effective
/// level stays at or above a third of the weapon's RadLevel the gate is
/// closed and it does not fire again.
#[test]
fn deployed_desolator_self_irradiates_and_refires_below_third() {
    let rules = radiation_rules();
    let mut sim = crate::sim::world::Simulation::new();
    let heights = BTreeMap::new();
    let deso = sim
        .spawn_object("DESO", "Americans", 10, 10, 0, &rules, &heights)
        .expect("desolator spawns");
    sim.substrate.entities.get_mut(deso).unwrap().deploy_state =
        Some(crate::sim::deploy::DeployPhase::Deployed);

    // Tick 1: gate open (no site) → self-targeted deploy-weapon shot.
    let result = rad_combat_tick(&mut sim, &rules, 1);
    assert_eq!(result.fire_events.len(), 1, "deployed self-irradiate fires");
    assert_eq!(
        sim.interner.resolve(result.fire_events[0].weapon_id),
        "RadEruptionWeapon"
    );
    assert_eq!(result.fire_events[0].target, TargetKind::Cell(10, 10));
    let site = sim
        .radiation
        .site_at((10, 10))
        .expect("detonation armed a site at the desolator's cell");
    assert_eq!(site.level, 500);
    assert_eq!(sim.radiation.cell_level((10, 10)), 500.0);

    // Tick 2: gate closed (effective 500 ≥ 500/3) → no fire, self-target
    // cleared.
    let result = rad_combat_tick(&mut sim, &rules, 2);
    assert_eq!(result.fire_events.len(), 0, "gate closed after re-arm");
    assert!(
        sim.substrate
            .entities
            .get(deso)
            .unwrap()
            .attack_target
            .is_none(),
        "synthesized self-target is cleared once the gate closes"
    );

    // Decay the site below RadLevel/3 (= 166): effective = remaining×500/500
    // drops below 166 once remaining < 167.
    for frame in 3..=340 {
        sim.radiation.tick_decay(frame, &rules.radiation, None);
    }
    let site = sim.radiation.site_at((10, 10)).expect("site still alive");
    assert!(crate::sim::radiation::RadiationState::current_site_level(site) < 500 / 3);

    // Gate reopens → fires again and merges the site back up.
    let result = rad_combat_tick(&mut sim, &rules, 341);
    assert_eq!(result.fire_events.len(), 1, "gate reopens below one third");
    let site = sim.radiation.site_at((10, 10)).expect("merged site");
    assert!(site.level > 500, "re-detonation merged effective + added");
}

#[test]
fn under_attack_events_fire_for_enemy_hit_structures_and_miners_only() {
    // The Phase-4 damage-apply producer: an enemy-damaged Structure emits a
    // base ping, an enemy-damaged harvester a miner ping; same-owner damage
    // and plain-unit victims emit nothing.
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=HARV\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=CAGAS\n\n\
         [CAGAS]\nStrength=800\nArmor=wood\n\n\
         [HARV]\nStrength=1000\nArmor=heavy\nSpeed=4\nHarvester=yes\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n",
    ))
    .expect("rules parse");

    let mut main_rng = SimRng::new(1);
    let mut run_attack = |victim: GameEntity| -> CombatTickResult {
        let mut store = EntityStore::new();
        store.insert(victim);
        let mut attacker = make_infantry_entity(1, "E1", 5, 5, 125);
        attacker.owner = test_intern("Attacker");
        store.insert(attacker);
        let mut interner = test_interner();
        issue_attack_command(&mut store, 1, 10, None, &interner);
        tick_combat(
            &mut store,
            &mut OccupancyGrid::new(),
            &rules,
            &mut interner,
            &mut BTreeMap::new(),
            0,
            100,
            0,
            &mut main_rng,
        )
    };

    // Enemy-owned Structure → one base ping for the VICTIM's owner.
    let mut building = make_entity_owned(10, "CAGAS", 8, 5, 800, "Defender");
    building.category = EntityCategory::Structure;
    let result = run_attack(building);
    assert_eq!(result.under_attack_events.len(), 1, "structure hit pings");
    let ev = &result.under_attack_events[0];
    assert!(!ev.miner);
    assert_eq!(ev.owner, test_intern("Defender"));
    assert_eq!((ev.rx, ev.ry), (8, 5));

    // Enemy-owned harvester (Miner component) → miner ping.
    let mut harv = make_entity_owned(10, "HARV", 8, 5, 1000, "Defender");
    harv.miner = Some(crate::sim::miner::Miner::new(
        crate::sim::miner::MinerKind::War,
        &crate::sim::miner::MinerConfig::default(),
        7,
    ));
    let result = run_attack(harv);
    assert_eq!(result.under_attack_events.len(), 1, "harvester hit pings");
    assert!(result.under_attack_events[0].miner);

    // SAME-owner structure damage → no ping (owner-differs hostility gate).
    let mut friendly = make_entity_owned(10, "CAGAS", 8, 5, 800, "Attacker");
    friendly.category = EntityCategory::Structure;
    let result = run_attack(friendly);
    assert!(
        result.under_attack_events.is_empty(),
        "same-owner damage never pings"
    );

    // Enemy plain unit (no miner, not a structure) → no ping.
    let plain = make_entity_owned(10, "HARV", 8, 5, 1000, "Defender");
    let result = run_attack(plain);
    assert!(
        result.under_attack_events.is_empty(),
        "plain unit hits do not ping"
    );
}

/// Build one ObjectType straight from an INI body, so the `Cost=` parse feeding
/// the score award is exercised rather than a hand-set field.
fn object_with_body(body: &str) -> ObjectType {
    let ini = IniFile::from_str(&format!(
        "[TEST]
{body}"
    ));
    ObjectType::from_ini_section(
        "TEST",
        ini.section("TEST").expect("test section"),
        crate::rules::object_type::ObjectCategory::Vehicle,
    )
}

#[test]
fn score_award_is_the_victim_cost_scaled_by_veterancy() {
    // gamemd's kill-record step values the victim at its `Cost=`, doubled at
    // veteran and tripled at elite. Anchored on stock Rhino (HTNK) Cost=900.
    let obj = object_with_body(
        "Cost=900
",
    );
    assert_eq!(obj.cost, 900, "Cost= parsed off the section");
    assert_eq!(score_award_for_victim(Some(&obj), 0), 900);
    assert_eq!(score_award_for_victim(Some(&obj), 99), 900);
    assert_eq!(score_award_for_victim(Some(&obj), 100), 1_800);
    assert_eq!(score_award_for_victim(Some(&obj), 199), 1_800);
    assert_eq!(score_award_for_victim(Some(&obj), 200), 2_700);
}

#[test]
fn score_award_ignores_the_dormant_points_key() {
    // `Points=` parses into a type field the binary never reads back — dormant
    // TS legacy in YR — so this engine does not parse it and a section carrying
    // only `Points=` is worth nothing. Stock GI (E1) is Cost=200 / Points=10; the
    // award must follow the cost, not the points.
    let points_only = object_with_body(
        "Points=10
",
    );
    assert_eq!(score_award_for_victim(Some(&points_only), 0), 0);

    let gi = object_with_body(
        "Cost=200
Points=10
",
    );
    assert_eq!(score_award_for_victim(Some(&gi), 0), 200);
}

#[test]
fn score_award_is_zero_without_a_cost_or_a_resolvable_type() {
    let obj = object_with_body(
        "Strength=100
",
    );
    assert_eq!(score_award_for_victim(Some(&obj), 200), 0);
    assert_eq!(score_award_for_victim(None, 200), 0);
}

#[test]
fn projectile_shrapnel_targets_hostile_head_before_random_cell_child() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n\n[MTNK]\nStrength=100\nArmor=heavy\nPrimary=PARENT\n\n[PARENT]\nDamage=20\nROF=10\nRange=6\nSpeed=30\nProjectile=PARENTPROJ\nWarhead=WH\n\n[PARENTPROJ]\nAirburst=yes\nShrapnelWeapon=CHILD\nShrapnelCount=2\n\n[CHILD]\nDamage=5\nROF=10\nRange=3\nSpeed=40\nProjectile=CHILDPROJ\nWarhead=WH\n\n[CHILDPROJ]\nSubjectToWalls=yes\n\n[WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("shrapnel rules");
    let mut entities = EntityStore::new();
    let mut source = make_entity_owned(1, "MTNK", 5, 5, 100, "Soviet");
    source.lifecycle.cell_marked = true;
    entities.insert(source);
    // First ring-table entry is (+1,-1).
    let mut target = make_entity_owned(2, "MTNK", 6, 4, 100, "Americans");
    target.lifecycle.cell_marked = true;
    entities.insert(target);
    let mut occupancy = OccupancyGrid::rebuild(&entities);
    let mut interner = test_interner();
    let detonation = crate::sim::projectile::ProjectileDetonation {
        projectile_id: 7,
        source_id: 1,
        target: crate::sim::projectile::ProjectileTarget::Cell { rx: 5, ry: 5 },
        impact: crate::sim::projectile::ProjectileCoord::new(5 * 256 + 128, 5 * 256 + 128, 0),
        payload: crate::sim::projectile::ProjectilePayload {
            base_damage: 20,
            warhead: interner.intern("WH"),
            weapon: interner.intern("PARENT"),
            owner: interner.intern("SOVIET"),
        },
        reason: crate::sim::projectile::ProjectileDetonationReason::ReachedTarget,
    };
    let mut scenario_rng = SimRng::new(0x46_a310);
    let mut expected_rng = scenario_rng.clone();
    let _ = expected_rng.next_range_u32_inclusive(0, 4);
    let _ = expected_rng.next_range_u32_inclusive(0, 4);
    let mut main_rng = SimRng::new(1);
    let mut houses = BTreeMap::new();

    let handles =
        crate::sim::type_handle_table::ResolvedRuleHandles::resolve(&rules, &mut interner);
    let result = tick_combat_with_fog_and_main_rng(
        &mut entities,
        &mut occupancy,
        &rules,
        &mut interner,
        Some(handles),
        None,
        &BTreeMap::new(),
        &mut houses,
        &[],
        &crate::map::houses::HouseAllianceMap::default(),
        None,
        &mut BTreeMap::new(),
        None,
        None,
        None,
        1,
        100,
        1,
        &[1, 2],
        &[detonation],
        &[],
        None,
        &[],
        &mut scenario_rng,
        &mut main_rng,
        None,
    );

    assert_eq!(result.projectile_spawns.len(), 2);
    assert_eq!(
        result.projectile_spawns[0].target,
        crate::sim::projectile::ProjectileTarget::Entity(2)
    );
    assert!(matches!(
        result.projectile_spawns[1].target,
        crate::sim::projectile::ProjectileTarget::Cell { .. }
    ));
    assert_eq!(scenario_rng.logical_state(), expected_rng.logical_state());
}

#[test]
fn gsi_04_01_projectile_shrapnel_captures_each_shared_dummy_lookup() {
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeStampSlot};
    use crate::sim::cell_rect::{CellRef, get_cellclass_fallback};
    use crate::sim::projectile::{
        ProjectileCoord, ProjectileTarget, dummy_cell_target_coord, projectile_random_shrapnel_cell,
    };
    use crate::util::lepton::{BRIDGE_HEIGHT_DELTA_LEPTONS, ground_height_leptons};

    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n\n[MTNK]\nStrength=100\nArmor=heavy\nPrimary=PARENT\n\n[PARENT]\nDamage=20\nROF=10\nRange=6\nSpeed=30\nProjectile=PARENTPROJ\nWarhead=WH\n\n[PARENTPROJ]\nAirburst=yes\nShrapnelWeapon=CHILD\nShrapnelCount=3\n\n[CHILD]\nDamage=5\nROF=10\nRange=3\nSpeed=40\nProjectile=CHILDPROJ\nWarhead=WH\n\n[CHILDPROJ]\nSubjectToWalls=yes\n\n[WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("shrapnel rules");
    let mut entities = EntityStore::new();
    let mut source = make_entity_owned(1, "MTNK", 5, 5, 100, "Soviet");
    source.lifecycle.cell_marked = true;
    entities.insert(source);
    let occupancy = OccupancyGrid::rebuild(&entities);
    let mut interner = test_interner();
    let detonation = crate::sim::projectile::ProjectileDetonation {
        projectile_id: 7,
        source_id: 1,
        target: ProjectileTarget::Cell { rx: 5, ry: 5 },
        impact: ProjectileCoord::new(5 * 256 + 128, 5 * 256 + 128, 0),
        payload: crate::sim::projectile::ProjectilePayload {
            base_damage: 20,
            warhead: interner.intern("WH"),
            weapon: interner.intern("PARENT"),
            owner: interner.intern("SOVIET"),
        },
        reason: crate::sim::projectile::ProjectileDetonationReason::ReachedTarget,
    };

    let mut scenario_rng = SimRng::new(0x46_a310);
    let mut expected_rng = scenario_rng.clone();
    let expected_cells = [
        projectile_random_shrapnel_cell(5, 5, &mut expected_rng),
        projectile_random_shrapnel_cell(5, 5, &mut expected_rng),
        projectile_random_shrapnel_cell(5, 5, &mut expected_rng),
    ];
    assert_ne!(expected_cells[0], expected_cells[1]);
    assert_ne!(expected_cells[1], expected_cells[2]);

    // The declared map has storage for every request, but only the first
    // random coordinate has a native CellClass pointer. The next two lookups
    // both return and restamp one shared dummy identity.
    let mut terrain = super::impact_height_tests::terrain_at_level(2);
    terrain.test_set_native_allocated_cells(&[(
        expected_cells[0].0 as u16,
        expected_cells[0].1 as u16,
    )]);
    terrain.test_set_dummy_cell_level_slope(2, 0);
    let dummy = terrain.shared_cell_dummy();
    dummy.apply_bridge_flag_slot(BridgeStampSlot::Anchor, true);

    let expected_target = |(rx, ry): (i32, i32), structural: bool| {
        let x = rx * 256 + 128;
        let y = ry * 256 + 128;
        let z = ground_height_leptons(2, 0, x, y).expect("flat CellClass surface is supported")
            + if structural {
                BRIDGE_HEIGHT_DELTA_LEPTONS as i32
            } else {
                0
            };
        ProjectileCoord::new(x, y, z)
    };
    let expected_positions = [
        expected_target(expected_cells[0], false),
        expected_target(expected_cells[1], true),
        expected_target(expected_cells[2], true),
    ];
    let mut out = CombatEmit::default();
    emit_projectile_shrapnel(
        &detonation,
        &entities,
        &occupancy,
        &rules,
        &mut interner,
        Some(&terrain),
        &HouseAllianceMap::default(),
        &mut scenario_rng,
        &mut out,
    );

    assert_eq!(scenario_rng.logical_state(), expected_rng.logical_state());
    assert_eq!(out.projectile_spawns.len(), 3);
    assert_eq!(
        out.projectile_spawns[0].target,
        ProjectileTarget::Cell {
            rx: expected_cells[0].0 as u16,
            ry: expected_cells[0].1 as u16,
        }
    );
    assert_eq!(out.projectile_spawns[1].target, ProjectileTarget::DummyCell);
    assert_eq!(out.projectile_spawns[2].target, ProjectileTarget::DummyCell);
    for (index, spawn) in out.projectile_spawns.iter().enumerate() {
        assert_eq!(spawn.initial_target_position, expected_positions[index]);
        assert_eq!(
            spawn.velocity,
            shrapnel_launch_velocity(detonation.impact, expected_positions[index], 40)
        );
    }
    assert_ne!(
        out.projectile_spawns[1].initial_target_position,
        out.projectile_spawns[2].initial_target_position
    );

    let final_snapshot = dummy.snapshot();
    assert_eq!(final_snapshot.coord, expected_cells[2]);
    assert_ne!(
        final_snapshot.bridge_flags_0x1180 & BRIDGE_FLAG_STRUCTURAL,
        0
    );
    assert!(dummy.same_identity(&terrain.shared_cell_dummy()));

    // A later miss restamps the retained pointer for BulletClass::AI without
    // rewriting either child's constructor-time launch coordinate.
    let later = get_cellclass_fallback(Some(&terrain), 9, 10);
    let CellRef::Dummy { cell: later_dummy } = later else {
        panic!("native-unallocated later lookup must retain the shared dummy");
    };
    assert!(dummy.same_identity(&later_dummy));
    assert_eq!(dummy.snapshot().coord, (9, 10));
    let later_target = dummy_cell_target_coord(&later_dummy);
    assert_eq!(later_target.x, 9 * 256 + 128);
    assert_eq!(later_target.y, 10 * 256 + 128);
    assert_eq!(later_target.z, 2 * 104 + BRIDGE_HEIGHT_DELTA_LEPTONS as i32);
    assert_ne!(
        later_target,
        out.projectile_spawns[2].initial_target_position
    );
    assert_eq!(
        out.projectile_spawns
            .iter()
            .map(|spawn| spawn.initial_target_position)
            .collect::<Vec<_>>(),
        expected_positions
    );
}

#[test]
fn gsi_04_10_near_center_iron_curtain_isolates_earlier_terrain_receiver() {
    use crate::sim::combat::combat_aoe::AreaDamageReceiver;
    use crate::sim::superweapon::invulnerability::{InvulnKind, InvulnerabilityState};
    use crate::sim::terrain_object::{TerrainObjectLifecycle, TerrainObjectState};

    fn run(kind: InvulnKind, techno_distance: i32) -> i32 {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\nTreeStrength=100\n\
             [InfantryTypes]\n\
             [VehicleTypes]\n0=VICTIM\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TerrainTypes]\n0=TREE01\n\
             [Warheads]\n0=WOODWH\n\
             [VICTIM]\nStrength=100\nArmor=wood\n\
             [TREE01]\nStrength=100\nArmor=wood\nImmune=no\n\
             [WOODWH]\nWood=yes\nCellSpread=.5\nPercentAtMax=1\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
        ))
        .expect("Terrain isolation rules");
        let mut sim = crate::sim::world::Simulation::new();
        sim.resolve_type_handles(&rules);
        let victim_id = sim
            .spawn_object("VICTIM", "VictimHouse", 5, 5, 0, &rules, &BTreeMap::new())
            .expect("protected Techno spawns");
        sim.substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .invulnerability = Some(InvulnerabilityState {
            start_frame: 0,
            duration_frames: 100,
            kind,
        });

        let terrain_id = 700;
        let terrain_ref = sim.interner.intern("TREE01");
        sim.production.terrain_objects.insert(
            terrain_id,
            TerrainObjectState {
                stable_id: terrain_id,
                in_logic_vector: false,
                type_ref: terrain_ref,
                rx: 5,
                ry: 5,
                health: 100,
                max_health: 100,
                occupation_bits: 4,
                lifecycle: TerrainObjectLifecycle::Live,
            },
        );
        sim.production
            .terrain_object_cells
            .insert((5, 5), terrain_id);

        let warhead_ref = sim.interner.intern("WOODWH");
        let mut entity_event = EntityDamageEvent::area(
            victim_id,
            10,
            techno_distance,
            RAD_NO_ATTACKER,
            None,
            warhead_ref,
        );
        entity_event.near_center_ic_isolation_eligible = true;
        let receivers = [
            AreaDamageReceiver::Terrain(TerrainDamageEvent {
                stable_id: terrain_id,
                rx: 5,
                ry: 5,
                damage: 10,
                distance_leptons: 0,
                warhead_ref,
                near_center_ic_isolation_eligible: true,
            }),
            AreaDamageReceiver::Entity(entity_event),
        ];
        sim.commit_noncombat_aoe_receivers(&rules, None, &receivers);
        sim.production.terrain_objects[&terrain_id].health
    }

    assert_eq!(
        run(InvulnKind::IronCurtain, 84),
        100,
        "the later active IC record arms isolation for an earlier Terrain record"
    );
    assert_eq!(
        run(InvulnKind::IronCurtain, 85),
        90,
        "the native distance boundary is strict less-than 85"
    );
    assert_eq!(
        run(InvulnKind::ForceShield, 84),
        90,
        "Force Shield receives through isolation but never arms it"
    );
}

#[test]
fn gsi_04_10_entity_fatal_hook_and_later_terrain_share_raw_occupation() {
    use crate::sim::combat::combat_aoe::AreaDamageReceiver;
    use crate::sim::terrain_object::{
        TerrainObjectLifecycle, TerrainObjectState, mark_terrain_raw_occupation,
    };

    let mut rules = RuleSet::from_ini(&IniFile::from_str(
        "[General]\nTreeStrength=10\n\
         [InfantryTypes]\n\
         [VehicleTypes]\n0=VICTIM\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [TerrainTypes]\n0=TREE01\n\
         [Warheads]\n0=WOODWH\n\
         [VICTIM]\nStrength=10\nArmor=wood\nSpeed=6\n\
         [TREE01]\nStrength=10\nArmor=wood\nImmune=no\n\
         [WOODWH]\nWood=yes\nCellSpread=1\nPercentAtMax=1\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
    ))
    .expect("shared raw-occupation rules");
    let mut sim = crate::sim::world::Simulation::new();
    sim.resolve_type_handles(&rules);
    let entity_id = sim
        .spawn_object("VICTIM", "VictimHouse", 4, 5, 0, &rules, &BTreeMap::new())
        .expect("fatal vehicle spawns");
    let terrain_id = 701;
    let terrain_ref = sim.interner.intern("TREE01");
    sim.production.terrain_objects.insert(
        terrain_id,
        TerrainObjectState {
            stable_id: terrain_id,
            in_logic_vector: false,
            type_ref: terrain_ref,
            rx: 5,
            ry: 5,
            health: 10,
            max_health: 10,
            occupation_bits: 4,
            lifecycle: TerrainObjectLifecycle::Live,
        },
    );
    sim.production
        .terrain_object_cells
        .insert((5, 5), terrain_id);
    mark_terrain_raw_occupation(&mut sim.substrate.raw_cell_occupation, (5, 5), 4);
    assert_ne!(sim.substrate.raw_cell_occupation.ground_bits(4, 5), 0);
    assert_ne!(sim.substrate.raw_cell_occupation.ground_bits(5, 5), 0);

    let warhead_ref = sim.interner.intern("WOODWH");
    let receivers = [
        AreaDamageReceiver::Entity(EntityDamageEvent::area(
            entity_id,
            10,
            0,
            RAD_NO_ATTACKER,
            None,
            warhead_ref,
        )),
        AreaDamageReceiver::Terrain(TerrainDamageEvent {
            stable_id: terrain_id,
            rx: 5,
            ry: 5,
            damage: 10,
            distance_leptons: 0,
            warhead_ref,
            near_center_ic_isolation_eligible: false,
        }),
    ];
    sim.commit_noncombat_aoe_receivers(&rules, None, &receivers);

    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(4, 5),
        0,
        "World UnInit clears the Techno bit through the lent authoritative raw grid"
    );
    assert_eq!(
        sim.substrate.raw_cell_occupation.ground_bits(5, 5),
        0,
        "the later Terrain finalize observes and mutates that same grid"
    );
    assert_eq!(
        sim.production.terrain_objects[&terrain_id].lifecycle,
        TerrainObjectLifecycle::Destroyed
    );
}

/// A Grizzly (`Cost=700`) killing rookie Rhinos (`Cost=900`) earns 900/(700*3)
/// per kill through the real damage path, so it wears a chevron on kill 3 and
/// goes elite on kill 5 — `TechnoClass::Record_The_Kill @ 0x00702D40` feeding
/// `VeterancyClass::Add @ 0x0074FF50`.
#[test]
fn gsi_08_12_a_grizzly_promotes_through_the_damage_path() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n[VehicleTypes]\n0=MTNK\n1=HTNK\n[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nCost=700\nPrimary=105mm\n[HTNK]\nStrength=1\nArmor=heavy\nSpeed=4\nCost=900\nPrimary=105mm\n[E1]\nStrength=125\nArmor=flak\nSpeed=4\nCost=200\nPrimary=M60\n[M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n[General]\nVeteranRatio=3.0\nVeteranCap=2\n",
    ))
    .expect("veterancy fixture parses");

    // Guard the fixture itself: the award divides by the killer's cost and
    // multiplies by the victim's, so a mistyped section silently yields zero.
    assert_eq!(rules.object("HTNK").map(|o| o.cost), Some(900));
    assert_eq!(rules.object("MTNK").map(|o| o.cost), Some(700));
    let mut ranks = Vec::new();
    let mut killer = make_entity_owned(1, "MTNK", 5, 5, 300, "Soviet");
    killer.owner = test_intern("Soviet");
    let mut store = EntityStore::new();
    store.insert(killer);
    // Intern the victim type before snapshotting the thread-local test
    // interner, or the award cannot resolve its `Cost=`.
    let _ = test_intern("HTNK");
    let mut interner = test_interner();
    let mut scenario_rng = SimRng::new(9);

    for victim_id in 2..=6u64 {
        store.insert(make_entity_owned(victim_id, "HTNK", 8, 5, 1, "Americans"));
        issue_attack_command(&mut store, 1, victim_id, None, &interner);
        if let Some(attacker) = store.get_mut(1) {
            if let Some(target) = attacker.attack_target.as_mut() {
                target.cooldown_ticks = 0;
            }
        }
        tick_combat(
            &mut store,
            &mut OccupancyGrid::new(),
            &rules,
            &mut interner,
            &mut BTreeMap::new(),
            0,
            100,
            0,
            &mut scenario_rng,
        );
        store.remove(victim_id);
        ranks.push(store.get(1).expect("killer").veterancy);
    }

    assert_eq!(ranks, vec![0, 0, 100, 100, 200], "ranks after kills 1..5");
}

/// `UnitClass::Death_Explosion @ 0x00738680` plays one anim from the dying
/// type's own `Explosion=` list and then one from `DestroyAnim=`, at its own
/// coordinate, one `Random__Next()` draw each. Before this the type's list had
/// no reader at all and every vehicle died with the warhead's puff.
#[test]
fn gsi_08_11_unit_death_plays_type_explosion_then_destroy_anim() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n1=HTNK\n[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nCost=700\nPrimary=105mm\n[HTNK]\nStrength=1\nArmor=heavy\nSpeed=4\nCost=900\nPrimary=105mm\nExplosion=TWLT070,TWLT120\nDestroyAnim=SMOKEY\n[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("death-explosion fixture parses");

    let mut store = EntityStore::new();
    store.insert(make_entity_owned(1, "MTNK", 5, 5, 300, "Soviet"));
    let _ = test_intern("HTNK");
    store.insert(make_entity_owned(2, "HTNK", 8, 5, 1, "Americans"));
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
        &mut SimRng::new(4),
    );

    let names: Vec<&str> = result
        .explosion_effects
        .iter()
        .map(|effect| interner.resolve(effect.shp_name))
        .collect();
    let explosion_index = names
        .iter()
        .position(|name| *name == "TWLT070" || *name == "TWLT120")
        .expect("the type's own Explosion= anim");
    let destroy_index = names
        .iter()
        .position(|name| *name == "SMOKEY")
        .expect("the type's DestroyAnim=");
    assert!(
        explosion_index < destroy_index,
        "Explosion= precedes DestroyAnim=: {names:?}"
    );
}

/// `Record_The_Kill @ 0x00702D40` reads the VICTIM type's `DontScore=` byte
/// (`+0xC9F`) at 0x00702E4E and returns before the multiplier, before the
/// accumulator and before the score add. Stock marks the V3, Dreadnought and
/// Boomer missiles that way, and they are shot down in most matches — without
/// the gate every interception promotes the interceptor.
#[test]
fn gsi_08_12_a_dont_score_victim_pays_no_experience() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n1=HTNK\n[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nCost=700\nPrimary=105mm\n[HTNK]\nStrength=1\nArmor=heavy\nSpeed=4\nCost=900\nPrimary=105mm\nDontScore=yes\n[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n[General]\nVeteranRatio=3.0\nVeteranCap=2\n",
    ))
    .expect("dont-score fixture parses");

    let mut store = EntityStore::new();
    store.insert(make_entity_owned(1, "MTNK", 5, 5, 300, "Soviet"));
    let _ = test_intern("HTNK");
    let mut interner = test_interner();
    let mut scenario_rng = SimRng::new(11);

    for victim_id in 2..=6u64 {
        let mut victim = make_entity_owned(victim_id, "HTNK", 8, 5, 1, "Americans");
        victim.dont_score = true;
        store.insert(victim);
        issue_attack_command(&mut store, 1, victim_id, None, &interner);
        if let Some(attacker) = store.get_mut(1)
            && let Some(target) = attacker.attack_target.as_mut()
        {
            target.cooldown_ticks = 0;
        }
        tick_combat(
            &mut store,
            &mut OccupancyGrid::new(),
            &rules,
            &mut interner,
            &mut BTreeMap::new(),
            0,
            100,
            0,
            &mut scenario_rng,
        );
        store.remove(victim_id);
    }

    let killer = store.get(1).expect("killer");
    assert_eq!(killer.veterancy, 0, "five DontScore kills earn nothing");
    assert_eq!(killer.veterancy_raw.bits(), 0);
}

/// The shot leaves the BARREL, not the hull centre.
///
/// `TechnoClass::Fire_At` launches from `GetFLH @ 0x006F3AD0`, and the stock
/// MTNK fixture (`PrimaryFireFLH=190,25,120`, body north, turret east) puts that
/// muzzle at `+189, -25, +120` leptons from the object coordinate — 189, not
/// 190, because retail composes two table rotations whose residual truncates the
/// X term down a lepton.
#[test]
fn gsi_08_04_projectile_spawns_at_the_muzzle_not_the_hull_centre() {
    let mut rules = RuleSet::from_ini(&IniFile::from_str(
        "[VehicleTypes]\n0=MTNK\n1=HTNK\n[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nCost=700\nPrimary=105mm\nTurret=yes\n[HTNK]\nStrength=2000\nArmor=heavy\nSpeed=4\nCost=900\nPrimary=105mm\n[105mm]\nDamage=65\nROF=50\nRange=6\nSpeed=40\nProjectile=Cannon\nWarhead=AP\n[Cannon]\nArcing=true\n[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("fire-origin fixture parses");
    rules.merge_art_data(&crate::rules::art_data::ArtRegistry::from_ini(
        &IniFile::from_str("[MTNK]\nPrimaryFireFLH=190,25,120\n"),
    ));

    let mut store = EntityStore::new();
    let mut shooter = make_entity_owned(1, "MTNK", 5, 5, 300, "Soviet");
    // Body facing north; the turret is aimed east at the target.
    shooter.facing = 0;
    shooter.barrel_facing = Some(crate::sim::movement::facing_class::FacingClass::new(
        0x4000, 0,
    ));
    store.insert(shooter);
    let _ = test_intern("HTNK");
    store.insert(make_entity_owned(2, "HTNK", 8, 5, 2000, "Americans"));
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
        &mut SimRng::new(3),
    );

    let spawn = result
        .projectile_spawns
        .first()
        .expect("the shot creates a tracked projectile");
    let hull_x = i32::from(store.get(1).unwrap().position.rx) * 256
        + store.get(1).unwrap().position.sub_x.to_num::<i32>();
    let hull_y = i32::from(store.get(1).unwrap().position.ry) * 256
        + store.get(1).unwrap().position.sub_y.to_num::<i32>();
    assert_eq!(
        (spawn.origin.x - hull_x, spawn.origin.y - hull_y),
        (189, -25),
        "muzzle offset in leptons"
    );
}
