//! Exact active-retail refund boundary used by Building -> Unit failure paths.
//!
//! gamemd provenance: `TechnoTypeClass::GetRefundValue @ 0x00711F60`, as
//! frozen in `PHASE8_BUILDING_SELLING_FSM_TEARDOWN_REFUND_GHIDRA_REPORT.md`
//! section 7.4. The four retail reverse sources are the three Construction
//! Yards and YAREFN. Retail country Building/Defense cost factors are 1.0, and
//! the only active FactoryPlant (NAINDP) also has Buildings/Defenses bonuses
//! 1.0, so those two native inputs are exact identities for this bounded path.

use crate::rules::ruleset::RuleSet;
use crate::sim::world::Simulation;
use crate::util::native_x87::X87Chop53;

/// Return the source Building's exact active-retail reverse-failure refund.
///
/// - YAREFN takes the nonzero `Soylent` branch and ignores RefundPercent.
/// - A controlled-human ConYard takes two native integerization stages; the
///   first is identity under retail cost factors, then RefundPercent is applied.
/// - A nonhuman ConYard returns the first intermediate unchanged.
pub(in crate::sim) fn active_retail_reverse_refund_for_building(
    sim: &Simulation,
    rules: &RuleSet,
    stable_id: u64,
) -> Option<i32> {
    let entity = sim.substrate.entities.get(stable_id)?;
    let object = rules.object_case_insensitive(sim.interner.resolve(entity.type_ref))?;

    if object.soylent != 0 {
        // Native multiplies by House.GetCostBonus(Type) before ftol. That f32
        // factor is exactly 1.0 for every active-retail reverse Building.
        return Some(object.soylent);
    }

    // The first native ftol is `Cost * accumulated * country`; both f32
    // multipliers are exact 1.0 for the three retail Construction Yards.
    let intermediate = object.cost;
    let controlled_by_human = sim.houses.get(&entity.owner).map_or(true, |house| {
        house.is_controlled_by_human(sim.session.game_mode_nonzero)
    });
    if !controlled_by_human {
        return Some(intermediate);
    }

    let percent = X87Chop53::load_f64(rules.general.refund_percent).ok()?;
    let result =
        X87Chop53::ftol_i64(X87Chop53::mul(X87Chop53::load_i32(intermediate), percent)).ok()?;
    i32::try_from(result).ok()
}

/// Credit a reverse-conversion failure through native House signed addition.
/// `HouseClass::Add_Credits @ 0x004F9950` is a plain 32-bit `+= amount`.
pub(in crate::sim) fn credit_reverse_failure_refund(
    sim: &mut Simulation,
    owner: &str,
    refund: i32,
) {
    let credits = super::credits_entry_for_owner(sim, owner);
    *credits = credits.wrapping_add(refund);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::house_state::HouseState;

    fn refund_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\nRefundPercent=33%\n\
             [BuildingTypes]\n0=GACNST\n1=YAREFN\n\
             [GACNST]\nCost=3001\nConstructionYard=yes\nUndeploysInto=AMCV\n\
             [YAREFN]\nCost=1750\nSoylent=1750\nUndeploysInto=SMIN\n",
        ))
        .expect("reverse refund rules")
    }

    fn refund_sim(is_human: bool, type_name: &str) -> Simulation {
        let mut sim = Simulation::new();
        sim.session.game_mode_nonzero = true;
        let owner = sim.interner.intern(if is_human { "Human" } else { "AI" });
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, is_human, 0, 10));
        let mut building = GameEntity::test_default(7, type_name, "Owner", 4, 5);
        building.owner = owner;
        building.type_ref = sim.interner.intern(type_name);
        building.category = EntityCategory::Structure;
        sim.substrate.entities.insert(building);
        sim
    }

    #[test]
    fn controlled_human_conyard_uses_second_x87_truncation() {
        let rules = refund_rules();
        let sim = refund_sim(true, "GACNST");

        assert_eq!(
            active_retail_reverse_refund_for_building(&sim, &rules, 7),
            Some(990),
            "ftol(3001 * parsed f64 0.33) truncates toward zero"
        );
    }

    #[test]
    fn nonhuman_conyard_skips_refund_percent() {
        let rules = refund_rules();
        let sim = refund_sim(false, "GACNST");

        assert_eq!(
            active_retail_reverse_refund_for_building(&sim, &rules, 7),
            Some(3001)
        );
    }

    #[test]
    fn yarefn_soylent_ignores_owner_control_and_refund_percent() {
        let rules = refund_rules();
        for is_human in [true, false] {
            let mut sim = refund_sim(is_human, "YAREFN");
            sim.substrate.entities.get_mut(7).unwrap().health.current = 1;
            assert_eq!(
                active_retail_reverse_refund_for_building(&sim, &rules, 7),
                Some(1750)
            );
        }
    }

    #[test]
    fn failure_credit_uses_native_signed_wrapping_addition() {
        let mut sim = Simulation::new();
        *super::super::credits_entry_for_owner(&mut sim, "Human") = i32::MAX - 1;

        credit_reverse_failure_refund(&mut sim, "Human", 3);

        assert_eq!(
            *super::super::credits_entry_for_owner(&mut sim, "Human"),
            i32::MIN + 1
        );
    }
}
