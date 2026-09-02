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

use super::super::combat_weapon::{
    VersesGate, attacker_facts, primary_for_tier, target_is_high_flying, verses_gate,
    weapon_for_index,
};
use super::super::{TargetKind, armor_index, object_world_z_leptons};
use super::{BaseDefenseResponseContext, ExistingTargetDisposition, ResponderPeekFireError};

/// Native "is this object armed" predicate.
///
/// gamemd-derived: `TechnoClass::Is_Armed @ 0x00701120` (vtable `+0x2AC`;
/// Foot/Infantry/Unit/Aircraft all inherit it) is
/// `w = GetCurrentWeapon(); w != NULL && w->WeaponType != NULL`.
/// `TechnoClass::GetCurrentWeapon @ 0x0070E1A0` (vtable `+0x3F4`) asks
/// `GetWeapon(CurrentWeaponNumber (+0x138))` when
/// `TechnoTypeClass::HasTurrets @ 0x00717880` (`TurretCount (+0x808) > 0`) and
/// `GetWeapon(0)` otherwise. `BuildingClass::Is_Armed @ 0x00458DB0` overrides
/// the slot: an occupied building (`vt+0x400`) is armed unconditionally, and
/// otherwise falls through to the Techno test.
///
/// So only ONE slot is consulted — `Secondary=` never makes an object armed,
/// and a `TurretCount>0` type is armed only through its current gunner slot.
/// The elite tier is applied by `GetWeapon`, which is why this reads
/// `weapon_for_index` rather than the raw `Primary=` field.
pub(super) fn is_armed(entity: &GameEntity, object: &ObjectType) -> bool {
    let facts = attacker_facts(entity, object);
    // `BuildingClass::Is_Armed 0x00458DB0`: `IsOccupied() → 1`.
    if facts.is_occupied_building {
        return true;
    }
    let index = if object.turret_count > 0 {
        facts.current_weapon_number
    } else {
        0
    };
    weapon_for_index(object, facts.veterancy, index).is_some()
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

/// gamemd-derived: `FootClass::Evaluate_Target_Threat @ 0x004D97A0` returns
/// `-Cost` when the current `TarCom (+0x2B4)` is the requester, and 0 when it
/// is some other Techno (`+0x14 & 1`) that `Is_Armed (vt+0x2AC)`.
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
            if entities.get(id).is_some_and(|target| {
                rules
                    .object(interner.resolve(target.type_ref))
                    .is_some_and(|object| is_armed(target, object))
            }) =>
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
/// vtable `+0x3BC` (`0x006FC090`) with range checking disabled by
/// `TechnoClass__RespondToBaseAttack @ 0x00708276/0x007084AC`.
///
/// The air/ground arm is the same altitude-driven one the selection path uses
/// (`combat_weapon::targeting_fire_error_blocks`): `0x006FC705..0x006FC739`
/// blocks a shot at an `IsHighFlying` target without an AA projectile, and the
/// AG test at `0x006FC7EB` is reached only when `TEST EBP,EBP` @ `0x006FC762`
/// finds a non-`ObjectClass` target — a force-fired cell, never the attacking
/// Techno this peek is aimed at. Neither arm reads the target's category.
///
/// RESIDUAL: state that VERA does not yet represent (Ivan bomb attachment,
/// temporal/drain target latches, several Magnetron immunity bytes and the
/// subclass-only illegal arms) cannot yet be classified here. The GSI row stays
/// open until those active-YR producers and the Unit/Infantry overrides have
/// executable coverage.
///
/// RESIDUAL (UNCHECKED) — three GetFireError arms this peek still omits:
/// - `0x006FC76A..0x006FC7CA` (naval `-1`) and `0x006FC7D0..0x006FC868`
///   (`LandTargeting==1` off water) both need the target's occupied-cell
///   `LandType`, which the response has no terrain handle for here. Trigger: a
///   Dolphin/Squid/Sub recruited against a land attacker. Player effect: it is
///   admitted to the response list and walks to a shot it cannot take.
///   Frequency: only in naval-base defence. Downstream: none — the real fire
///   path re-runs the full verdict.
/// - `0x006FC727` splits the high-flying verdict into 3 (the target is this
///   object's `DeployedFrom`, `TechnoClass+0x2AC`) and 5, and the caller admits
///   3. VERA reports `Illegal` for both, so such a candidate is rejected where
///   gamemd keeps it. Trigger: shooting at the structure one deployed out of.
///   Frequency: rare. Downstream: none.
pub(crate) fn responder_peek_fire_error(
    candidate: &GameEntity,
    target: &GameEntity,
    candidate_object: &ObjectType,
    target_object: &ObjectType,
    rules: &RuleSet,
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
    let projectile = weapon
        .projectile
        .as_deref()
        .and_then(|id| rules.projectile(id));
    // `0x006FC705..0x006FC739`: altitude, not category — a landed Rocketeer is
    // an ordinary ground target and needs no AA projectile.
    let projectile_legal =
        !target_is_high_flying(target) || projectile.is_some_and(|projectile| projectile.aa);
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
        || !is_armed(candidate, candidate_object)
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
