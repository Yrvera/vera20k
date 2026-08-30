//! Shared native AI-list BuildingType identity predicate.
//!
//! Naval placement and BasePlan Recalc both call the same HouseClass helper;
//! this module keeps that predicate separate from generic production gates.

use crate::rules::object_type::{ObjectCategory, ObjectType};
use crate::rules::ruleset::{CountryIdx, RuleSet};

pub(crate) fn country_bit(index: CountryIdx) -> u32 {
    1u32.wrapping_shl(u32::from(index.0 & 31))
}

pub(crate) fn house_token_mask(tokens: &[String], rules: &RuleSet) -> u32 {
    tokens.iter().fold(0u32, |mask, token| {
        rules
            .trigger_house_type_index(token)
            .map_or(mask, |index| mask | country_bit(index))
    })
}

pub(crate) fn owner_allows(candidate: &ObjectType, country_bit: u32, rules: &RuleSet) -> bool {
    !candidate.owner.is_empty() && house_token_mask(&candidate.owner, rules) & country_bit != 0
}

/// Exact identity/shell tail of `HouseClass__FirstBuildableFromArray`.
///
/// gamemd-derived: `HouseClass__FirstBuildableFromArray @ 0x005051E0`.
/// `TechnoTypeClass` construction at `0x00711193` initializes Owner to zero;
/// reader block `0x007149E1..0x007149F5` preserves that missing-key default.
pub(crate) fn candidate_allowed(
    candidate: &ObjectType,
    country_bit: u32,
    side_index: u8,
    super_weapons: bool,
    rules: &RuleSet,
) -> bool {
    if !owner_allows(candidate, country_bit, rules) {
        return false;
    }
    if !candidate.required_houses.is_empty()
        && house_token_mask(&candidate.required_houses, rules) & country_bit == 0
    {
        return false;
    }
    if !candidate.forbidden_houses.is_empty()
        && house_token_mask(&candidate.forbidden_houses, rules) & country_bit != 0
    {
        return false;
    }
    if candidate.ai_base_planning_side != -1
        && candidate.ai_base_planning_side != i32::from(side_index)
    {
        return false;
    }
    if super_weapons {
        return true;
    }
    let Some(primary) = candidate.super_weapon.as_deref() else {
        return true;
    };
    if rules
        .build_tech_types
        .iter()
        .any(|type_id| type_id.eq_ignore_ascii_case(&candidate.id))
    {
        return true;
    }
    rules
        .super_weapon(primary)
        .is_some_and(|super_weapon| !super_weapon.disableable_from_shell)
}

/// Resolve the first passing BuildingType pointer from one authored AI list.
pub(crate) fn first_buildable_from_array<'a>(
    rules: &'a RuleSet,
    type_ids: &[String],
    country_name: &str,
    side_index: u8,
    super_weapons: bool,
) -> Option<&'a ObjectType> {
    let bit = country_bit(rules.trigger_house_type_index(country_name)?);
    type_ids.iter().find_map(|type_id| {
        let candidate = rules.object_in_category(ObjectCategory::Building, type_id)?;
        candidate_allowed(candidate, bit, side_index, super_weapons, rules).then_some(candidate)
    })
}
