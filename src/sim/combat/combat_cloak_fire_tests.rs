//! Production DecloakToFire gate acceptance for stock Boomer-shaped units.

use std::collections::BTreeMap;

use super::*;
use crate::rules::ini_parser::IniFile;
use crate::sim::cloak_disguise::CloakRuntime;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::test_interner;

fn boomer_rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[General]\nCloakingStages=9\n\
         [VehicleTypes]\n0=BSUB\n1=TGTWOOD\n2=TGTLIGHT\n\
         [BSUB]\nStrength=2000\nArmor=heavy\nSpeed=4\nCloakable=yes\nCloakingSpeed=1\n\
         Primary=BoomerTorpedo\nSecondary=CruiseLauncher\n\
         [TGTWOOD]\nStrength=500\nArmor=wood\n\
         [TGTLIGHT]\nStrength=500\nArmor=light\n\
         [BoomerTorpedo]\nDamage=40\nROF=20\nRange=8\nWarhead=BoomWH\nDecloakToFire=no\n\
         [CruiseLauncher]\nDamage=25\nROF=20\nRange=8\nWarhead=CruiseWH\n\
         [BoomWH]\nVerses=100%,100%,100%,100%,100%,100%,0%,0%,0%,0%,0%\n\
         [CruiseWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
    ))
    .expect("Boomer cloak-fire fixture")
}

fn entities(target_type: &str) -> EntityStore {
    let mut store = EntityStore::new();
    let mut bsub = GameEntity::test_default(1, "BSUB", "Soviet", 10, 10);
    bsub.attack_target = Some(AttackTarget::new(2));
    let mut cloak = CloakRuntime::new(0, 9);
    cloak.establish_unlimbo_fully_cloaked();
    bsub.cloak = Some(cloak);
    store.insert(bsub);
    store.insert(GameEntity::test_default(2, target_type, "Americans", 11, 10));
    store
}

fn resolve_once(
    entities: &mut EntityStore,
    rules: &RuleSet,
    interner: &mut StringInterner,
    out: &mut CombatEmit,
) {
    let attacker = entities.get(1).unwrap();
    let attack = attacker.attack_target.as_ref().unwrap();
    let snap = build_attacker_snapshot(
        attacker,
        attack.target,
        attack.cooldown_ticks,
        attack.burst_remaining,
        attack.burst_delay_ticks,
        attack.pending_infantry_fire,
        None,
        None,
    );
    let mut resources = BTreeMap::new();
    let mut rng = SimRng::new(0xC10A_F1AE);
    let mut hooks: Option<&mut dyn CombatInlineHooks> = None;
    resolve_attacker_fire(
        &snap,
        entities,
        rules,
        interner,
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
        77,
        67,
        &mut rng,
        &mut hooks,
        out,
    );
}

#[test]
fn bsub_cruise_launcher_uncloaks_without_same_tick_fire_then_retry_fires() {
    let rules = boomer_rules();
    let mut entities = entities("TGTWOOD");
    let mut interner = test_interner();
    let mut blocked = CombatEmit::default();

    resolve_once(&mut entities, &rules, &mut interner, &mut blocked);

    assert_eq!(
        entities.get(1).unwrap().cloak.as_ref().unwrap().state,
        3,
        "0x736DF0 case 9 calls StartUncloaking"
    );
    assert!(entities.get(1).unwrap().attack_target.is_some(), "target is retained for retry");
    assert!(blocked.fire_events.is_empty());
    assert!(blocked.damage_events.is_empty());
    assert!(blocked.projectile_spawns.is_empty());
    assert!(blocked.burst_updates.is_empty(), "no ROF/burst write on the surfacing visit");

    // Represent completion of StartUncloaking's ordinary state-3 progression.
    let cloak = entities.get_mut(1).unwrap().cloak.as_mut().unwrap();
    cloak.state = 0;
    cloak.visual_phase = None;
    let mut retry = CombatEmit::default();
    resolve_once(&mut entities, &rules, &mut interner, &mut retry);
    assert_eq!(retry.fire_events.len(), 1);
    assert_eq!(interner.resolve(retry.fire_events[0].weapon_id), "CruiseLauncher");
    assert_eq!(retry.fire_events[0].weapon_slot, WeaponSlot::Secondary);
    assert!(!retry.burst_updates.is_empty(), "normal retry owns rearm state");
}

#[test]
fn bsub_boomer_torpedo_explicit_no_fires_while_fully_cloaked() {
    let rules = boomer_rules();
    let mut entities = entities("TGTLIGHT");
    let mut interner = test_interner();
    let mut out = CombatEmit::default();

    resolve_once(&mut entities, &rules, &mut interner, &mut out);

    assert_eq!(entities.get(1).unwrap().cloak.as_ref().unwrap().state, 2);
    assert_eq!(out.fire_events.len(), 1);
    assert_eq!(interner.resolve(out.fire_events[0].weapon_id), "BoomerTorpedo");
    assert_eq!(out.fire_events[0].weapon_slot, WeaponSlot::Primary);
}
