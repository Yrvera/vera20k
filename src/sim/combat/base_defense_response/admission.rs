//! Destination, reachability and exact-5 fire-admission helpers.

use crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::{InternedId, StringInterner};
use crate::util::fixed_math::SimFixed;
use crate::util::lepton::ground_height_leptons;

use super::super::combat_weapon::{VersesGate, primary_for_tier, verses_gate};
use super::super::{TargetKind, armor_index, combat_target_category, object_world_z_leptons};
use super::{BaseDefenseResponseContext, ExistingTargetDisposition, ResponderPeekFireError};

pub(super) fn object_has_weapon(object: &ObjectType) -> bool {
    object.primary.is_some() || object.secondary.is_some() || !object.weapon_list.is_empty()
}

pub(super) fn entity_coord(entity: &GameEntity, terrain: Option<&ResolvedTerrainGrid>) -> [i32; 3] {
    [
        i32::from(entity.position.rx as i16)
            .wrapping_mul(256)
            .wrapping_add(entity.position.sub_x.to_num::<i32>()),
        i32::from(entity.position.ry as i16)
            .wrapping_mul(256)
            .wrapping_add(entity.position.sub_y.to_num::<i32>()),
        object_world_z_leptons(entity, terrain),
    ]
}

fn nav_target_coord(
    target: crate::sim::components::NavTargetRef,
    entities: &EntityStore,
    terrain: Option<&ResolvedTerrainGrid>,
) -> Option<[i32; 3]> {
    use crate::sim::components::NavTargetRef;
    match target {
        NavTargetRef::Cell { rx, ry } => Some([
            i32::from(rx as i16).wrapping_mul(256).wrapping_add(128),
            i32::from(ry as i16).wrapping_mul(256).wrapping_add(128),
            0,
        ]),
        NavTargetRef::Entity { id }
        | NavTargetRef::Object { id }
        | NavTargetRef::Building { id } => {
            entities.get(id).map(|entity| entity_coord(entity, terrain))
        }
    }
}

fn destination_coord(
    entity: &GameEntity,
    entities: &EntityStore,
    terrain: Option<&ResolvedTerrainGrid>,
) -> [i32; 3] {
    if let Some(nav_com) = entity.navigation.nav_com
        && let Some(coord) = nav_target_coord(nav_com, entities, terrain)
    {
        return coord;
    }
    if let (Some(tube_state), Some(terrain)) = (entity.low_bridge_tube_state, terrain)
        && let Some(tube) = terrain.tube(tube_state.tube_id)
    {
        return [
            i32::from(tube.exit.0 as i16)
                .wrapping_mul(256)
                .wrapping_add(128),
            i32::from(tube.exit.1 as i16)
                .wrapping_mul(256)
                .wrapping_add(128),
            0,
        ];
    }
    entity_coord(entity, terrain)
}

fn lepton_to_cell_component(value: i32) -> i32 {
    value.wrapping_add((value >> 31) & 255) >> 8
}

pub(super) fn destination_cell(
    entity: &GameEntity,
    entities: &EntityStore,
    terrain: Option<&ResolvedTerrainGrid>,
) -> (i32, i32) {
    let coord = destination_coord(entity, entities, terrain);
    (
        i32::from(lepton_to_cell_component(coord[0]) as i16),
        i32::from(lepton_to_cell_component(coord[1]) as i16),
    )
}

fn ground_height_at_coord(terrain: &ResolvedTerrainGrid, coord: [i32; 3]) -> Option<i32> {
    let cell = (
        lepton_to_cell_component(coord[0]),
        lepton_to_cell_component(coord[1]),
    );
    if cell.0 < 0 || cell.1 < 0 {
        return None;
    }
    let cell = terrain.cell(cell.0 as u16, cell.1 as u16)?;
    ground_height_leptons(cell.level, cell.slope_type, coord[0], coord[1]).ok()
}

/// Response-local implementation of `ObjectClass::ShouldBeOnBridge` using the
/// exact destination returned above. The general movement path still carries
/// its separately recorded wider signature residual.
///
/// gamemd-derived: `ObjectClass::ShouldBeOnBridge @ 0x005F6A70` and the Foot
/// override `0x004DDC40`; the height threshold is `3 * 104` leptons.
pub(super) fn should_be_on_bridge_for_response(
    entity: &GameEntity,
    entities: &EntityStore,
    terrain: &ResolvedTerrainGrid,
) -> Option<bool> {
    const HEIGHT_THRESHOLD: i32 = 3 * 104;
    let current = entity_coord(entity, Some(terrain));
    let destination = destination_coord(entity, entities, Some(terrain));
    let current_ground = ground_height_at_coord(terrain, current)?;
    let destination_ground = ground_height_at_coord(terrain, destination)?;
    let destination_cell = (
        lepton_to_cell_component(destination[0]),
        lepton_to_cell_component(destination[1]),
    );
    let destination_has_bridge = terrain
        .cellclass_bridge_flags_0x1180(destination_cell.0, destination_cell.1)
        & BRIDGE_FLAG_STRUCTURAL
        != 0;

    if !entity.on_bridge
        && current_ground.wrapping_sub(destination_ground) > HEIGHT_THRESHOLD
        && destination_has_bridge
    {
        return Some(true);
    }
    if entity.on_bridge && destination_ground.wrapping_sub(current_ground) > HEIGHT_THRESHOLD {
        return Some(false);
    }
    Some(entity.on_bridge)
}

pub(super) fn current_target_disposition(
    candidate: &GameEntity,
    attacker_id: u64,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
) -> ExistingTargetDisposition {
    match candidate.attack_target.as_ref().map(|target| target.target) {
        Some(TargetKind::Entity(id)) if id == attacker_id => {
            ExistingTargetDisposition::RequestedAttacker
        }
        Some(TargetKind::Entity(id))
            if entities
                .get(id)
                .and_then(|target| rules.object(interner.resolve(target.type_ref)))
                .is_some_and(object_has_weapon) =>
        {
            ExistingTargetDisposition::OtherArmedTarget
        }
        _ => ExistingTargetDisposition::NoneOrUnarmed,
    }
}

pub(super) fn primary_range_leptons(
    candidate: &GameEntity,
    object: &ObjectType,
    rules: &RuleSet,
) -> i32 {
    primary_for_tier(object, candidate.veterancy)
        .and_then(|weapon_id| rules.weapon(weapon_id))
        .map(|weapon| (weapon.range * SimFixed::from_num(256)).to_num::<i32>())
        .unwrap_or(0)
}

/// Represented exact-code classifier for the response's read-only weapon-zero
/// peek. Non-5 errors intentionally remain distinct and are admitted by the
/// caller.
///
/// gamemd-derived: `TechnoClass__GetFireError @ 0x006FC0B0`, called through
/// vtable `+0x3BC` with range checking disabled by
/// `TechnoClass__RespondToBaseAttack @ 0x00708276/0x007084AC`.
///
/// RESIDUAL: state that VERA does not yet represent (Ivan bomb attachment,
/// temporal/drain target latches, several Magnetron immunity bytes and the
/// subclass-only illegal arms) cannot yet be classified here. The GSI row stays
/// open until those active-YR producers and the Unit/Infantry overrides have
/// executable coverage.
pub(crate) fn responder_peek_fire_error(
    candidate: &GameEntity,
    target: &GameEntity,
    candidate_object: &ObjectType,
    target_object: &ObjectType,
    rules: &RuleSet,
    interner: &StringInterner,
) -> ResponderPeekFireError {
    if candidate.slave_harvester.is_some()
        || target.lifecycle.in_limbo
        || candidate
            .passenger_role
            .inside_transport_id()
            .is_some_and(|transport_id| transport_id == target.stable_id)
    {
        return ResponderPeekFireError::Illegal;
    }

    let Some(weapon_id) = primary_for_tier(candidate_object, candidate.veterancy) else {
        return ResponderPeekFireError::Cant;
    };
    let Some(weapon) = rules.weapon(weapon_id) else {
        return ResponderPeekFireError::Cant;
    };
    if candidate.passenger_role.inside_transport_id().is_some() && !weapon.fire_in_transport {
        return ResponderPeekFireError::Illegal;
    }
    let Some(warhead) = weapon.warhead.as_deref().and_then(|id| rules.warhead(id)) else {
        return ResponderPeekFireError::Cant;
    };
    let target_category = combat_target_category(target, rules, interner);
    let projectile = weapon
        .projectile
        .as_deref()
        .and_then(|id| rules.projectile(id));
    let projectile_legal = match target_category {
        EntityCategory::Aircraft => projectile.is_some_and(|projectile| projectile.aa),
        EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Structure => {
            projectile.is_none_or(|projectile| projectile.ag)
        }
    };
    if !projectile_legal
        || verses_gate(
            warhead
                .verses
                .get(armor_index(&target_object.armor))
                .copied()
                .unwrap_or(100),
        ) == VersesGate::Blocked
        || (warhead.psychedelic && target_object.immune_to_psionics)
        || (warhead.psychedelic && target.bunker_link.installed_in().is_some())
    {
        return ResponderPeekFireError::Illegal;
    }

    if candidate.railgun_system_id.is_some() && weapon.is_railgun {
        return ResponderPeekFireError::Busy;
    }
    if candidate
        .cloak
        .as_ref()
        .is_some_and(|cloak| cloak.state != 0)
        && weapon.decloak_to_fire
    {
        return ResponderPeekFireError::Cloaked;
    }
    ResponderPeekFireError::Clear
}

pub(super) fn candidate_admitted(
    candidate: &GameEntity,
    candidate_object: &ObjectType,
    target: &GameEntity,
    victim_owner: InternedId,
    attacker_id: u64,
    context: &BaseDefenseResponseContext<'_>,
) -> bool {
    if !candidate.is_object_alive()
        || candidate.owner != victim_owner
        || context
            .teams
            .team_for_member(candidate.stable_id)
            .is_some_and(|(_, is_base_defense)| !is_base_defense)
        || !candidate.base_defense_response.recruitable_a
        || !candidate.base_defense_response.recruitable_b
        || !object_has_weapon(candidate_object)
        || (!context.game_mode_nonzero
            && !candidate
                .mission
                .current()
                .known()
                .and_then(|mission| context.rules.mission_control.entry(mission))
                .is_some_and(|entry| entry.recruitable))
        || responder_peek_fire_error(
            candidate,
            context
                .entities
                .get(attacker_id)
                .expect("entry retained attacker"),
            candidate_object,
            context
                .rules
                .object(
                    context.interner.resolve(
                        context
                            .entities
                            .get(attacker_id)
                            .expect("entry retained attacker")
                            .type_ref,
                    ),
                )
                .expect("entry retained attacker type"),
            context.rules,
            context.interner,
        ) == ResponderPeekFireError::Illegal
    {
        return false;
    }
    if candidate.category == EntityCategory::Unit
        && (candidate_object.resource_gatherer
            || candidate.bunker_link.installed_in().is_some()
            || target.slave_harvester.is_some())
    {
        return false;
    }
    true
}
