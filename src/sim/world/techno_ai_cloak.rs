//! Stock Techno cloak producer at the Techno AI head.

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::TargetKind;
use crate::sim::mission::concrete_effects::represented_assign_target;

use super::Simulation;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SensorCloakReevaluation {
    pub(crate) cloak_transitioned: bool,
    pub(crate) reassigned_targeters: Vec<u64>,
}

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

fn sensor_targeters_in_native_dispatch_order(sim: &Simulation, cloaker_id: u64) -> Vec<u64> {
    let Some(cloaker) = sim.substrate.entities.get(cloaker_id) else {
        return Vec::new();
    };
    let cloaker_owner = cloaker.owner;
    let cloaker_cell = (cloaker.position.rx, cloaker.position.ry);

    // TechnoClass+0x420 @ 0x006F4EB0 reverse-scans g_TechnoClass_Array,
    // appends admitted targeters, then reverse-dispatches the saved vector.
    // The two reversals produce forward Techno construction order. VERA's
    // stable object IDs are monotonic construction IDs, so the EntityStore's
    // ordered Techno walk is the same order without copying the native arrays.
    sim.substrate
        .entities
        .iter_sorted()
        .filter_map(|(targeter_id, targeter)| {
            let targets_cloaker = targeter.attack_target.as_ref().is_some_and(|target| {
                target.target == TargetKind::Entity(cloaker_id)
            });
            let admitted = targeter.owner == cloaker_owner
                || sim.fog.has_sensor_for_house(
                    targeter.owner,
                    cloaker_cell.0,
                    cloaker_cell.1,
                );
            (targets_cloaker && admitted).then_some(targeter_id)
        })
        .collect()
}

/// Active `TechnoClass+0x420 @ 0x006F4EB0` owner-visible/CanAutoCloak arm.
/// Sensor lifecycle calls this in exact CellClass FirstObject order after the
/// corresponding signed counter mutation. Under the outer gate, native first
/// snapshots admitted targeters, then starts cloak/sound, then reassigns the
/// saved same target through `Assign_Target @ 0x006FCDB0`; that setter clears
/// passive provenance before its same-target early return. The player-local
/// redraw arm remains presentation state and is not serialized or world-hashed.
pub(crate) fn sensor_reevaluate_stock_cloak(
    sim: &mut Simulation,
    id: u64,
    rules: &RuleSet,
) -> SensorCloakReevaluation {
    let Some(facts) = stock_cloak_tick_facts(sim, id, rules) else {
        return SensorCloakReevaluation::default();
    };
    let owner_cell_visible = sim.substrate.entities.get(id).is_some_and(|entity| {
        sim.fog
            .is_cell_visible(entity.owner, entity.position.rx, entity.position.ry)
    });
    if !owner_cell_visible || !facts.can_auto_cloak {
        return SensorCloakReevaluation::default();
    }

    // Snapshot before StartCloaking, its positional sound, or any targeter
    // mutation. This is the DynamicVector transaction in 0x006F4EB0.
    let reassigned_targeters = sensor_targeters_in_native_dispatch_order(sim, id);
    let start = sim.substrate
        .entities
        .get_mut(id)
        .and_then(|entity| entity.cloak.as_mut())
        .map(|cloak| cloak.start_cloaking_from_sensor(facts.current_frame, facts.cloaking_speed));
    if start.is_some_and(|start| start.play_sound) {
        emit_configured_cloak_sound(sim, id, rules);
    }
    for &targeter_id in &reassigned_targeters {
        let targeter = sim
            .substrate
            .entities
            .get_mut(targeter_id)
            .expect("saved Techno targeter remains registered during sensor callback");
        represented_assign_target(targeter, Some(TargetKind::Entity(id)));
    }
    SensorCloakReevaluation {
        cloak_transitioned: start.is_some_and(|start| start.transitioned),
        reassigned_targeters,
    }
}

fn emit_configured_cloak_sound(sim: &mut Simulation, id: u64, rules: &RuleSet) {
    let Some(sound_name) = rules.general.cloak_sound.as_deref() else {
        return;
    };
    let Some(position) = sim
        .substrate
        .entities
        .get(id)
        .map(|entity| entity.position.clone())
    else {
        return;
    };
    sim.sound_events
        .push(crate::sim::world::SimSoundEvent::cloak_sound(sound_name.to_owned(), &position));
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
    let result = sim
        .substrate
        .entities
        .get_mut(id)
        .and_then(|entity| entity.cloak.as_mut())
        .map(|cloak| cloak.tick(facts, &mut sim.scenario_rng));
    if result.is_some_and(|result| result.play_cloak_sound) {
        emit_configured_cloak_sound(sim, id, rules);
    }
}

#[cfg(test)]
#[path = "techno_ai_cloak_tests.rs"]
mod tests;
