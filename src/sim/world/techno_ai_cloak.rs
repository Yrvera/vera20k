//! Stock Techno cloak producer at the Techno AI head.

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;

use super::Simulation;

fn stock_cloak_tick_facts(
    sim: &Simulation,
    id: u64,
    rules: &RuleSet,
) -> Option<crate::sim::cloak_disguise::CloakTickFacts> {
    let entity = sim.substrate.entities.get(id)?;
    if entity.category != EntityCategory::Unit || entity.cloak.is_none() {
        return None;
    }
    let object = rules.object(sim.interner.resolve(entity.type_ref))?;
    let rank_cloak = entity.veterancy >= 100 && object.veteran_cloak
        || entity.veterancy >= 200 && object.elite_cloak;
    if !object.cloakable && !rank_cloak {
        return None;
    }
    let moving = crate::sim::movement::drive_locomotor_is_moving(entity)
        || entity.movement_target.is_some();
    let firing = entity.attack_target.is_some();
    let chrono_active = entity.teleport_state.is_some();
    let is_cloakable = object.cloakable && (!object.cloak_stop || !moving);
    let activity = firing || chrono_active;
    let state_zero_head_allows = is_cloakable && !activity || rank_cloak;

    // CloakingTick's pre-CanAuto destination exclusion is Contact_With_Whom(0)
    // resolving to a WeaponsFactory building (naval-yard repair contact), not
    // an arbitrary movement destination.
    let destination_is_weapons_factory = entity.radio_contacts.slot(0).is_some_and(|contact_id| {
        sim.substrate
            .entities
            .get(contact_id)
            .filter(|contact| contact.category == EntityCategory::Structure)
            .and_then(|contact| rules.object(sim.interner.resolve(contact.type_ref)))
            .is_some_and(|contact_type| contact_type.weapons_factory)
    });
    let current_frame = sim.session.binary_frame as i32;
    let delay_expired = entity
        .cloak
        .as_ref()
        .is_some_and(|cloak| cloak.recloak_delay_expired(current_frame));
    let can_auto_cloak = !destination_is_weapons_factory
        && delay_expired
        && !firing
        && !entity.mind_controlled
        && entity.position.z < 1
        && (is_cloakable
            || rank_cloak
            || sim
                .fog
                .is_cell_visible(entity.owner, entity.position.rx, entity.position.ry));

    // ShouldUncloak @ 0x006FBC90 returns early while intrinsic cloakability is
    // idle, and rank CLOAK suppresses the later activity path. The surviving
    // branch is the current owner-cell visibility test.
    let should_uncloak = if (is_cloakable || object.cloakable) && !activity || rank_cloak {
        false
    } else {
        !sim
            .fog
            .is_cell_visible(entity.owner, entity.position.rx, entity.position.ry)
    };
    Some(crate::sim::cloak_disguise::CloakTickFacts {
        current_frame,
        state_zero_head_allows,
        can_auto_cloak,
        should_uncloak,
        health_above_red: health_strictly_above_condition_red(
            entity.health,
            rules.general.condition_red_x1000,
        ),
        cloaking_speed: object.cloaking_speed,
        cloak_delay_frames: rules.general.cloak_delay_frames,
    })
}

/// Active `TechnoClass+0x420 @ 0x006F4EB0` owner-visible/CanAutoCloak arm.
/// Sensor lifecycle calls this in exact CellClass FirstObject order after the
/// corresponding signed counter mutation. The player-local redraw arm remains
/// presentation state and is deliberately not serialized or world-hashed.
pub(crate) fn sensor_reevaluate_stock_cloak(
    sim: &mut Simulation,
    id: u64,
    rules: &RuleSet,
) -> bool {
    let Some(facts) = stock_cloak_tick_facts(sim, id, rules) else {
        return false;
    };
    let owner_cell_visible = sim.substrate.entities.get(id).is_some_and(|entity| {
        sim.fog
            .is_cell_visible(entity.owner, entity.position.rx, entity.position.ry)
    });
    if !owner_cell_visible || !facts.can_auto_cloak {
        return false;
    }
    sim.substrate
        .entities
        .get_mut(id)
        .and_then(|entity| entity.cloak.as_mut())
        .is_some_and(|cloak| {
            cloak.start_cloaking_from_sensor(facts.current_frame, facts.cloaking_speed)
        })
}

fn health_strictly_above_condition_red(
    health: crate::sim::components::Health,
    condition_red_x1000: i64,
) -> bool {
    i64::from(health.current) * 1000 > i64::from(health.max) * condition_red_x1000
}

/// Produce the world-dependent virtual results consumed by
/// `TechnoClass::CloakingTick @ 0x006FB740`. Stock cloakable objects are Units;
/// the caller keeps this at the Unit Techno bracket head to preserve Scenario
/// RNG ordering relative to the rest of that object's AI visit.
pub(super) fn tick_stock_cloak_producer(sim: &mut Simulation, id: u64, rules: &RuleSet) {
    let Some((category, type_ref, veterancy)) = sim
        .substrate
        .entities
        .get(id)
        .map(|entity| (entity.category, entity.type_ref, entity.veterancy))
    else {
        return;
    };
    if category != EntityCategory::Unit {
        return;
    }
    let Some(object) = rules.object(sim.interner.resolve(type_ref)) else {
        return;
    };
    let rank_cloak = veterancy >= 100 && object.veteran_cloak
        || veterancy >= 200 && object.elite_cloak;
    if !object.cloakable && !rank_cloak {
        return;
    }

    if sim
        .substrate
        .entities
        .get(id)
        .is_some_and(|entity| entity.cloak.is_none())
        && let Some(entity) = sim.substrate.entities.get_mut(id)
    {
        entity.cloak = Some(crate::sim::cloak_disguise::CloakRuntime::new(
            sim.session.binary_frame as i32,
            rules.general.cloaking_stages,
        ));
    }

    let Some(facts) = stock_cloak_tick_facts(sim, id, rules) else {
        return;
    };
    if let Some(cloak) = sim
        .substrate
        .entities
        .get_mut(id)
        .and_then(|entity| entity.cloak.as_mut())
    {
        cloak.tick(facts, &mut sim.scenario_rng);
    }
}

#[cfg(test)]
#[path = "techno_ai_cloak_tests.rs"]
mod tests;
