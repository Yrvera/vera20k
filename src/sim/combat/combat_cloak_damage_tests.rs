//! The `ReceiveDamage` decloak and the damage outcomes that never reach it.
//!
//! `TechnoClass::ReceiveDamage @ 0x00701900` calls virtual `+0xFC`
//! (`StartUncloaking(0) @ 0x00703850`) at `0x0070281D`. The `CMP EDI,4 / JZ`
//! at `0x007027EE` is the only guard immediately in front of that call, but it
//! is not the only way native misses it: every defensive gate returns ABOVE the
//! `uVar7 = ObjectClass__ReceiveDamage(this)` join — the `TypeImmune`
//! (`type+0xC8C`) same-type/same-owner arm, `vt+0x160` (IronCurtain /
//! ForceShield), `vt+0x1D4` (warping in), the `AffectsAllies=no`
//! (`warhead+0x179`) allied arm, and the accepted Psychedelic arm.
//!
//! These pin the negative half. `[DLPH] TypeImmune=yes` in `rulesmd.ini` and
//! DLPH is one of the four stock `Cloakable=yes` types, so a player's own
//! Dolphins splashing each other is the ordinary trigger.

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::rules::ini_parser::IniFile;
use crate::sim::cloak_disguise::CloakRuntime;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::test_interner;
use crate::sim::superweapon::invulnerability::{InvulnKind, InvulnerabilityState};

const TICK: u64 = 100;
const CLOAKING_STAGES: i32 = 9;

fn dolphin_rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[General]\nCloakingStages=9\n\
         [AudioVisual]\nCloakSound=NavalUnitEmerge\n\
         [VehicleTypes]\n0=DLPH\n1=DEST\n\
         [DLPH]\nStrength=600\nArmor=none\nSpeed=4\nCloakable=yes\nCloakingSpeed=1\n\
         TypeImmune=yes\nPrimary=SonicZap\n\
         [DEST]\nStrength=600\nArmor=none\nSpeed=6\nPrimary=SonicZap\n\
         Secondary=AllyBlindZap\n\
         [SonicZap]\nDamage=40\nROF=20\nRange=8\nWarhead=SonicWH\n\
         [AllyBlindZap]\nDamage=40\nROF=20\nRange=8\nWarhead=AllyBlindWH\n\
         [SonicWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
         [AllyBlindWH]\nAffectsAllies=no\n\
         Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    ))
    .expect("dolphin damage fixture")
}

fn live(mut entity: GameEntity) -> GameEntity {
    entity.lifecycle.in_limbo = false;
    entity.in_playfield = true;
    entity
}

/// Entity 1 is the attacker (`attacker_type`), entity 2 a fully cloaked DLPH
/// one cell east. Both belong to `attacker_owner` / `Soviet` respectively.
fn store(attacker_type: &str, attacker_owner: &str) -> EntityStore {
    let mut store = EntityStore::new();
    store.insert(live(GameEntity::test_default(
        1,
        attacker_type,
        attacker_owner,
        10,
        10,
    )));
    let mut victim = live(GameEntity::test_default(2, "DLPH", "Soviet", 11, 10));
    let mut cloak = CloakRuntime::new(0, CLOAKING_STAGES);
    cloak.establish_unlimbo_fully_cloaked();
    victim.cloak = Some(cloak);
    store.insert(victim);
    store
}

struct Landed {
    cloak_state: i32,
    sounds: Vec<SimSoundEvent>,
    victim_hp: u16,
}

/// One `commit_damage_events` transaction: entity 1 hits entity 2 for 40 with
/// `warhead`, at the ordinary area distance zero.
fn hit(
    entities: &mut EntityStore,
    rules: &RuleSet,
    warhead: &str,
    alliances: &HouseAllianceMap,
) -> Landed {
    let mut interner = test_interner();
    let soviet = interner.intern("Soviet");
    let allies = interner.intern("Americans");
    let warhead_ref = interner.intern(warhead);
    let attacker_owner = entities.get(1).map(|attacker| attacker.owner);

    let mut houses = BTreeMap::from([
        (soviet, HouseState::new(soviet, 0, None, false, 0, 10)),
        (allies, HouseState::new(allies, 1, None, false, 0, 10)),
    ]);
    let house_order = [soviet, allies];
    let mut occupancy = OccupancyGrid::new();
    let mut main_rng = SimRng::new(11);
    let mut scenario_rng = SimRng::new(13);
    let mut handled_deaths = Vec::new();
    let mut resources = BTreeMap::new();
    let mut hooks = None;
    let mut collected: Vec<SimSoundEvent> = Vec::new();
    let mut sound_sink: Option<&mut Vec<SimSoundEvent>> = Some(&mut collected);

    let _ = commit_damage_events(
        &[EntityDamageEvent::area(
            2,
            40,
            0,
            1,
            attacker_owner,
            warhead_ref,
        )],
        entities,
        &mut occupancy,
        rules,
        &mut interner,
        &mut houses,
        &house_order,
        alliances,
        &mut main_rng,
        &mut scenario_rng,
        &mut handled_deaths,
        &mut resources,
        None,
        None,
        None,
        TICK,
        &mut hooks,
        &mut sound_sink,
    );

    let victim = entities.get(2).expect("victim survives the fixture");
    Landed {
        cloak_state: victim.cloak.as_ref().map_or(-1, |cloak| cloak.state),
        sounds: collected,
        victim_hp: victim.health.current,
    }
}

/// Positive control. Without it the three negative tests below would also pass
/// against a hook that never fires.
#[test]
fn an_ordinary_hostile_hit_surfaces_a_fully_cloaked_dolphin() {
    let rules = dolphin_rules();
    let mut entities = store("DEST", "Americans");
    let landed = hit(&mut entities, &rules, "SonicWH", &HouseAllianceMap::new());

    assert_eq!(landed.victim_hp, 60, "the hit lands for its 40");
    assert_eq!(
        landed.cloak_state, 3,
        "0x0070281D reaches vt+0xFC for a surviving receiver"
    );
    assert!(
        matches!(
            landed.sounds.as_slice(),
            [SimSoundEvent::CloakSound { sound_id, rx: 11, ry: 10, .. }]
                if sound_id == "NavalUnitEmerge"
        ),
        "StartUncloaking(0) owns one positional CloakSound, got {:?}",
        landed.sounds
    );
}

/// `TypeImmune` returns `0` from `TechnoClass::ReceiveDamage` well above the
/// `ObjectClass__ReceiveDamage` join, so `+0xFC` is never reached.
/// `[DLPH] TypeImmune=yes` makes this the routine case for a Dolphin pod.
#[test]
fn a_type_immune_sibling_splash_leaves_a_cloaked_dolphin_submerged() {
    let rules = dolphin_rules();
    let mut entities = store("DLPH", "Soviet");
    let landed = hit(&mut entities, &rules, "SonicWH", &HouseAllianceMap::new());

    assert_eq!(landed.victim_hp, 100, "TypeImmune nullifies the damage");
    assert_eq!(
        landed.cloak_state, 2,
        "native returns at type+0xC8C, above the vt+0xFC call"
    );
    assert!(
        landed.sounds.is_empty(),
        "no CloakSound: {:?}",
        landed.sounds
    );
}

/// `vt+0x160` (IronCurtain / ForceShield) writes `*damage = 0` and returns, so
/// the protected object never delegates to `ObjectClass::ReceiveDamage`.
#[test]
fn an_iron_curtained_cloaked_dolphin_stays_submerged() {
    let rules = dolphin_rules();
    let mut entities = store("DEST", "Americans");
    entities.get_mut(2).unwrap().invulnerability = Some(InvulnerabilityState {
        start_frame: TICK as u32 - 10,
        duration_frames: 100,
        kind: InvulnKind::IronCurtain,
    });
    let landed = hit(&mut entities, &rules, "SonicWH", &HouseAllianceMap::new());

    assert_eq!(landed.victim_hp, 100, "IronCurtain nullifies the damage");
    assert_eq!(landed.cloak_state, 2, "native returns at vt+0x160");
    assert!(
        landed.sounds.is_empty(),
        "no CloakSound: {:?}",
        landed.sounds
    );
}

/// `AffectsAllies=no` (`warhead+0x179`) with a present source and an allied
/// owner returns above the join too. A house is always allied with itself, and
/// an explicit two-house alliance behaves the same way.
#[test]
fn an_affects_allies_no_warhead_leaves_an_allied_cloaked_dolphin_submerged() {
    let rules = dolphin_rules();

    let mut own = store("DEST", "Soviet");
    let landed = hit(&mut own, &rules, "AllyBlindWH", &HouseAllianceMap::new());
    assert_eq!(landed.victim_hp, 100);
    assert_eq!(landed.cloak_state, 2, "same house is allied with itself");
    assert!(
        landed.sounds.is_empty(),
        "no CloakSound: {:?}",
        landed.sounds
    );

    let alliances: HouseAllianceMap = BTreeMap::from([
        (
            "AMERICANS".to_string(),
            BTreeSet::from(["SOVIET".to_string()]),
        ),
        (
            "SOVIET".to_string(),
            BTreeSet::from(["AMERICANS".to_string()]),
        ),
    ]);
    let mut allied = store("DEST", "Americans");
    let landed = hit(&mut allied, &rules, "AllyBlindWH", &alliances);
    assert_eq!(landed.victim_hp, 100);
    assert_eq!(landed.cloak_state, 2, "native returns at warhead+0x179");
    assert!(
        landed.sounds.is_empty(),
        "no CloakSound: {:?}",
        landed.sounds
    );

    // Control: the SAME warhead against a non-allied owner must still land.
    // Without this the two assertions above would also hold if `AllyBlindWH`
    // failed to resolve and the record were dropped before the receiver.
    let mut hostile = store("DEST", "Americans");
    let landed = hit(
        &mut hostile,
        &rules,
        "AllyBlindWH",
        &HouseAllianceMap::new(),
    );
    assert_eq!(landed.victim_hp, 60, "the warhead resolves and does damage");
    assert_eq!(landed.cloak_state, 3);
}
