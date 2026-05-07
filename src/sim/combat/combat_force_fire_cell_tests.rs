//! Force-fire-on-cell unit tests for `issue_attack_cell_command`.
//!
//! Verifies the sim-side entry point for `Command::ForceAttackCell` —
//! Ctrl + left-click on empty terrain.

use super::{AttackTarget, TargetKind, issue_attack_cell_command};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::{Health, MovementTarget};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::test_interner;

/// Minimal RuleSet: armed MTNK (Primary=105mm) and unarmed ENGI (no Primary).
fn ff_rules() -> RuleSet {
    let ini_str: &str = "\
[VehicleTypes]\n0=MTNK\n\n\
[InfantryTypes]\n0=ENGI\n\n\
[BuildingTypes]\n\n\
[AircraftTypes]\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[ENGI]\nStrength=75\nArmor=none\nSpeed=4\n\n\
[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n";
    let ini: IniFile = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("ff_rules should parse")
}

fn make_unit(id: u64, type_ref: &str, rx: u16, ry: u16, hp: u16) -> GameEntity {
    let mut e = GameEntity::test_default(id, type_ref, "Americans", rx, ry);
    e.health = Health {
        current: hp,
        max: hp,
    };
    e
}

#[test]
fn issue_attack_cell_sets_cell_target_for_armed_unit() {
    let mut store = EntityStore::new();
    store.insert(make_unit(1, "MTNK", 5, 5, 300));
    let interner = test_interner();
    let rules = ff_rules();

    let ok = issue_attack_cell_command(&mut store, 1, 50, 50, Some(&rules), &interner);

    assert!(ok, "issue_attack_cell_command should succeed for armed unit");
    let attack = store.get(1).unwrap().attack_target.as_ref().unwrap();
    assert!(matches!(attack.target, TargetKind::Cell(50, 50)));
    assert_eq!(attack.cooldown_ticks, 0);
    assert_eq!(attack.burst_remaining, 0);
}

#[test]
fn issue_attack_cell_rejects_unarmed_attacker() {
    let mut store = EntityStore::new();
    store.insert(make_unit(1, "ENGI", 5, 5, 75));
    let interner = test_interner();
    let rules = ff_rules();

    let ok = issue_attack_cell_command(&mut store, 1, 50, 50, Some(&rules), &interner);

    assert!(!ok, "ForceAttackCell on unarmed unit must return false");
    assert!(store.get(1).unwrap().attack_target.is_none());
}

#[test]
fn issue_attack_cell_clears_movement_target() {
    let mut store = EntityStore::new();
    store.insert(make_unit(1, "MTNK", 5, 5, 300));
    store.get_mut(1).unwrap().movement_target = Some(MovementTarget::default());
    let interner = test_interner();
    let rules = ff_rules();

    let ok = issue_attack_cell_command(&mut store, 1, 50, 50, Some(&rules), &interner);

    assert!(ok);
    assert!(store.get(1).unwrap().movement_target.is_none());
}

#[test]
fn issue_attack_cell_returns_false_for_missing_attacker() {
    let mut store = EntityStore::new();
    let interner = test_interner();
    let rules = ff_rules();

    let ok = issue_attack_cell_command(&mut store, 999, 50, 50, Some(&rules), &interner);

    assert!(!ok, "Should return false when attacker entity does not exist");
}

#[test]
fn for_cell_constructor_creates_cell_variant() {
    let at = AttackTarget::for_cell(42, 17);
    assert!(matches!(at.target, TargetKind::Cell(42, 17)));
    assert_eq!(at.cooldown_ticks, 0);
    assert_eq!(at.burst_remaining, 0);
    assert_eq!(at.burst_delay_ticks, 0);
}

#[test]
fn new_constructor_creates_entity_variant() {
    let at = AttackTarget::new(123);
    assert!(matches!(at.target, TargetKind::Entity(123)));
}
