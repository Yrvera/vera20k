//! Infantry fear, prone stance, and Crawls speed helpers.
//!
//! This module owns sim-authoritative infantry stance state. Animation reflects
//! this state; combat and movement do not infer prone status from animation.

use crate::rules::object_type::ObjectType;
use crate::sim::animation::SequenceKind;
use crate::sim::deploy::DeployPhase;
use crate::sim::game_entity::GameEntity;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

const MAX_FEAR: u16 = 300;
const FIRST_HIT_FEAR: u16 = 100;
const REPEATED_RED_ADD: u16 = 50;
const REPEATED_YELLOW_ADD: u16 = 25;
const REPEATED_GREEN_ADD: u16 = 12;
const PRONE_THRESHOLD: u16 = 50;
const VETERAN_LEVEL: u16 = 100;
const ELITE_LEVEL: u16 = 200;

pub fn has_veteran_fearless_ability(obj: &ObjectType, entity: &GameEntity) -> bool {
    if entity.veterancy >= ELITE_LEVEL {
        obj.veteran_fearless || obj.elite_fearless
    } else if entity.veterancy >= VETERAN_LEVEL {
        obj.veteran_fearless
    } else {
        false
    }
}

pub fn is_fear_application_blocked(obj: &ObjectType, entity: &GameEntity) -> bool {
    obj.fearless || has_veteran_fearless_ability(obj, entity)
}

pub fn can_decay_fear(obj: &ObjectType) -> bool {
    !obj.fearless
}

pub fn apply_panic_force(obj: &ObjectType, entity: &mut GameEntity) {
    if is_fear_application_blocked(obj, entity) {
        return;
    }
    if let Some(infantry) = entity.infantry.as_mut() {
        infantry.fear_level = MAX_FEAR;
    }
}

pub fn apply_fear_from_damage(
    obj: &ObjectType,
    entity: &mut GameEntity,
    damage_landed: u16,
    damager_present: bool,
    condition_red_x1000: i64,
    condition_yellow_x1000: i64,
) {
    if damage_landed == 0 || entity.health.current == 0 || is_fear_application_blocked(obj, entity)
    {
        return;
    }
    let Some(infantry) = entity.infantry.as_mut() else {
        return;
    };
    if infantry.fear_level == 0 && damager_present {
        infantry.fear_level = if obj.fraidycat {
            MAX_FEAR
        } else {
            FIRST_HIT_FEAR
        };
        return;
    }
    if infantry.fear_level <= 99 && damager_present {
        return;
    }

    let add = repeated_fear_add(
        entity.health.current,
        entity.health.max,
        condition_red_x1000,
        condition_yellow_x1000,
    );
    infantry.fear_level = infantry.fear_level.saturating_add(add).min(MAX_FEAR);
}

fn repeated_fear_add(
    current_health: u16,
    max_health: u16,
    condition_red_x1000: i64,
    condition_yellow_x1000: i64,
) -> u16 {
    let max = max_health.max(1) as i64;
    let current = current_health as i64 * 1000;
    if current <= max * condition_red_x1000 {
        REPEATED_RED_ADD
    } else if current <= max * condition_yellow_x1000 {
        REPEATED_YELLOW_ADD
    } else {
        REPEATED_GREEN_ADD
    }
}

pub fn tick_fear_decay_and_prone(
    obj: &ObjectType,
    entity: &mut GameEntity,
) -> Option<SequenceKind> {
    if !can_decay_fear(obj) {
        return None;
    }
    let Some(infantry) = entity.infantry.as_mut() else {
        return None;
    };
    if infantry.fear_level > 0 {
        infantry.fear_level -= 1;
    }
    if entity.dying || entity.deploy_state.is_some() {
        return None;
    }

    if !infantry.is_prone && infantry.fear_level >= PRONE_THRESHOLD {
        infantry.is_prone = true;
        Some(SequenceKind::Down)
    } else if infantry.is_prone && infantry.fear_level < PRONE_THRESHOLD {
        infantry.is_prone = false;
        Some(SequenceKind::Up)
    } else {
        None
    }
}

pub fn tick_fear_for_entities(
    entities: &mut crate::sim::entity_store::EntityStore,
    rules: &crate::rules::ruleset::RuleSet,
    interner: &crate::sim::intern::StringInterner,
) {
    let keys = entities.keys_sorted();
    for id in keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        let Some(obj) = rules.object(interner.resolve(entity.type_ref)) else {
            continue;
        };
        if let Some(sequence) = tick_fear_decay_and_prone(obj, entity) {
            if let Some(anim) = entity.animation.as_mut() {
                anim.switch_to(sequence);
            }
        }
    }
}

pub fn is_prone_for_damage(entity: &GameEntity) -> bool {
    entity.infantry.is_some_and(|infantry| infantry.is_prone)
}

pub fn apply_prone_speed(speed: SimFixed, crawls: bool) -> SimFixed {
    if speed <= SIM_ZERO {
        return speed;
    }
    let whole_speed = speed.to_num::<i32>().max(0);
    let adjusted = if crawls {
        (whole_speed.saturating_mul(2) + 2) / 3
    } else {
        whole_speed + whole_speed / 2
    };
    SimFixed::from_num(adjusted)
}

pub fn prone_adjusted_speed(entity: &GameEntity, obj: &ObjectType, speed: SimFixed) -> SimFixed {
    if is_prone_for_damage(entity) {
        apply_prone_speed(speed, obj.crawls)
    } else {
        speed
    }
}

pub fn is_deploy_locked(entity: &GameEntity) -> bool {
    matches!(
        entity.deploy_state,
        Some(DeployPhase::Deploying { .. })
            | Some(DeployPhase::Deployed)
            | Some(DeployPhase::Undeploying { .. })
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::object_type::ObjectCategory;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::components::Health;
    use crate::sim::game_entity::{GameEntity, InfantryRuntime};
    use crate::sim::intern::test_intern;

    fn rules_for(section: &str) -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(&format!(
            "[InfantryTypes]\n0=E1\n\n[VehicleTypes]\n\n[AircraftTypes]\n\n[BuildingTypes]\n\n[E1]\nStrength=100\nArmor=flak\nSpeed=4\n{section}\n"
        )))
        .expect("rules should parse")
    }

    fn infantry(hp: u16) -> GameEntity {
        let mut e = GameEntity::new(
            1,
            0,
            0,
            0,
            0,
            test_intern("Test"),
            Health {
                current: hp,
                max: 100,
            },
            test_intern("E1"),
            EntityCategory::Infantry,
            0,
            5,
            false,
        );
        e.infantry = Some(InfantryRuntime::new());
        e
    }

    #[test]
    fn first_hit_and_fraidycat_set_fear() {
        let rules = rules_for("");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(90);
        apply_fear_from_damage(obj, &mut e, 10, true, 250, 500);
        assert_eq!(e.infantry.unwrap().fear_level, FIRST_HIT_FEAR);

        let rules = rules_for("Fraidycat=yes\n");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(90);
        apply_fear_from_damage(obj, &mut e, 10, true, 250, 500);
        assert_eq!(e.infantry.unwrap().fear_level, MAX_FEAR);
    }

    #[test]
    fn repeated_hit_adds_by_health_and_clamps() {
        let rules = rules_for("");
        let obj = rules.object("E1").unwrap();
        for (hp, expected) in [(80, 112), (50, 125), (25, 150)] {
            let mut e = infantry(hp);
            e.infantry.as_mut().unwrap().fear_level = 100;
            apply_fear_from_damage(obj, &mut e, 1, true, 250, 500);
            assert_eq!(e.infantry.unwrap().fear_level, expected);
        }
        let mut e = infantry(25);
        e.infantry.as_mut().unwrap().fear_level = 290;
        apply_fear_from_damage(obj, &mut e, 1, true, 250, 500);
        assert_eq!(e.infantry.unwrap().fear_level, MAX_FEAR);
    }

    #[test]
    fn fearless_type_and_abilities_block_application() {
        let rules = rules_for("Fearless=yes\n");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(90);
        apply_fear_from_damage(obj, &mut e, 1, true, 250, 500);
        apply_panic_force(obj, &mut e);
        assert_eq!(e.infantry.unwrap().fear_level, 0);

        let rules = rules_for("VeteranAbilities=FEARLESS\n");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(90);
        e.veterancy = 100;
        apply_fear_from_damage(obj, &mut e, 1, true, 250, 500);
        assert_eq!(e.infantry.unwrap().fear_level, 0);

        let rules = rules_for("EliteAbilities=FEARLESS\n");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(90);
        e.veterancy = 200;
        apply_panic_force(obj, &mut e);
        assert_eq!(e.infantry.unwrap().fear_level, 0);
    }

    #[test]
    fn decay_thresholds_and_fearless_decay_gate() {
        let rules = rules_for("");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(100);
        e.infantry.as_mut().unwrap().fear_level = 50;
        assert_eq!(tick_fear_decay_and_prone(obj, &mut e), None);
        assert!(!e.infantry.unwrap().is_prone);

        let mut e = infantry(100);
        e.infantry.as_mut().unwrap().fear_level = 51;
        assert_eq!(
            tick_fear_decay_and_prone(obj, &mut e),
            Some(SequenceKind::Down)
        );
        assert!(e.infantry.unwrap().is_prone);

        let mut e = infantry(100);
        e.infantry.as_mut().unwrap().fear_level = 50;
        e.infantry.as_mut().unwrap().is_prone = true;
        assert_eq!(
            tick_fear_decay_and_prone(obj, &mut e),
            Some(SequenceKind::Up)
        );
        assert!(!e.infantry.unwrap().is_prone);

        let rules = rules_for("Fearless=yes\n");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(100);
        e.infantry.as_mut().unwrap().fear_level = 100;
        assert_eq!(tick_fear_decay_and_prone(obj, &mut e), None);
        assert_eq!(e.infantry.unwrap().fear_level, 100);

        let rules = rules_for("VeteranAbilities=FEARLESS\n");
        let obj = rules.object("E1").unwrap();
        let mut e = infantry(100);
        e.veterancy = 100;
        e.infantry.as_mut().unwrap().fear_level = 100;
        assert_eq!(
            tick_fear_decay_and_prone(obj, &mut e),
            Some(SequenceKind::Down)
        );
        assert_eq!(e.infantry.unwrap().fear_level, 99);
    }

    #[test]
    fn prone_speed_rounding_is_exact() {
        assert_eq!(
            apply_prone_speed(SimFixed::from_num(10), true),
            SimFixed::from_num(7)
        );
        assert_eq!(
            apply_prone_speed(SimFixed::from_num(11), true),
            SimFixed::from_num(8)
        );
        assert_eq!(
            apply_prone_speed(SimFixed::from_num(10), false),
            SimFixed::from_num(15)
        );
        assert_eq!(
            apply_prone_speed(SimFixed::from_num(11), false),
            SimFixed::from_num(16)
        );
    }

    #[test]
    fn object_category_import_keeps_rules_fixture_infantry() {
        assert_eq!(
            rules_for("").object("E1").unwrap().category,
            ObjectCategory::Infantry
        );
    }
}
