//! Stock Techno cloak producer at the Techno AI head.

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;

use super::Simulation;

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
    let Some((
        category,
        type_ref,
        veterancy,
        owner,
        health,
        position,
        moving,
        firing,
        chrono_active,
        mind_controlled,
        contact0,
    )) = sim.substrate.entities.get(id).map(|entity| {
        (
            entity.category,
            entity.type_ref,
            entity.veterancy,
            entity.owner,
            entity.health,
            (entity.position.rx, entity.position.ry, entity.position.z),
            crate::sim::movement::drive_locomotor_is_moving(entity)
                || entity.movement_target.is_some(),
            entity.attack_target.is_some(),
            entity.teleport_state.is_some(),
            entity.mind_controlled,
            entity.radio_contacts.slot(0),
        )
    }) else {
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

    // FootClass::IsCloakable @ 0x004DBDA0: the copied runtime ability is
    // suppressed only by CloakStop plus a busy locomotor.
    let is_cloakable = object.cloakable && (!object.cloak_stop || !moving);
    let activity = firing || chrono_active;
    let state_zero_head_allows = is_cloakable && !activity || rank_cloak;

    // CloakingTick's pre-CanAuto destination exclusion is Contact_With_Whom(0)
    // resolving to a WeaponsFactory building (naval-yard repair contact), not
    // an arbitrary movement destination.
    let destination_is_weapons_factory = contact0.is_some_and(|contact_id| {
        sim.substrate
            .entities
            .get(contact_id)
            .filter(|contact| contact.category == EntityCategory::Structure)
            .and_then(|contact| rules.object(sim.interner.resolve(contact.type_ref)))
            .is_some_and(|contact_type| contact_type.weapons_factory)
    });
    let current_frame = sim.session.binary_frame as i32;
    let delay_expired = sim
        .substrate
        .entities
        .get(id)
        .and_then(|entity| entity.cloak.as_ref())
        .is_some_and(|cloak| cloak.recloak_delay_expired(current_frame));
    let can_auto_cloak = !destination_is_weapons_factory
        && delay_expired
        && !firing
        && !mind_controlled
        && position.2 < 1
        && (is_cloakable
            || rank_cloak
            || sim.fog.is_cell_visible(owner, position.0, position.1));

    // ShouldUncloak @ 0x006FBC90 returns early while intrinsic cloakability is
    // idle, and rank CLOAK suppresses the later activity path. The surviving
    // branch is the current owner-cell visibility test.
    let should_uncloak = if (is_cloakable || object.cloakable) && !activity || rank_cloak {
        false
    } else {
        !sim.fog.is_cell_visible(owner, position.0, position.1)
    };
    let facts = crate::sim::cloak_disguise::CloakTickFacts {
        current_frame,
        state_zero_head_allows,
        can_auto_cloak,
        should_uncloak,
        health_above_red: health_strictly_above_condition_red(
            health,
            rules.general.condition_red_x1000,
        ),
        cloaking_speed: object.cloaking_speed,
        cloak_delay_frames: rules.general.cloak_delay_frames,
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
