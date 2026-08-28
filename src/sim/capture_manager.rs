//! Narrow `CaptureManagerClass` authority shared by mind-control producers and
//! receiver-synchronous retaliation.
//!
//! Native constructs this manager only when the owner's primary weapon uses a
//! `MindControl=yes` warhead. The manager snapshots the weapon's signed
//! `Damage=` as its finite link limit and `InfiniteMindControl=` as its capacity
//! bypass. Victim links are controller-owned and retain insertion order; they
//! are not reconstructed by scanning the independent permanent-control byte.

use serde::{Deserialize, Serialize};

use crate::map::entities::EntityCategory;
use crate::rules::locomotor_type::MovementZone;
use crate::rules::object_type::ObjectCategory;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::anim_class::AnimWorldCoord;
use crate::sim::components::{AnimClassSpawnDescriptor, NavTargetRef};
use crate::sim::intern::InternedId;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::team_script_vm::{TeamMemberTypeIdentity, TeamScriptMember};
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::native_x87::{
    NativeF32Bits, X87Chop53, X87Ordering, distance_3d_leptons,
};

/// One persistent native MCNode. The victim pointer and the House owner saved
/// at capture time are independent: House-wide destruction resolves effective
/// ownership from this saved owner, not from the victim's current House.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaptureNodeState {
    pub victim_id: u64,
    pub original_owner: InternedId,
    /// Raw signed `g_CurrentFrameCounter` sampled after successful ChangeOwner.
    pub capture_frame: i32,
    /// Signed Rules `MindControlAttackLineFrames` copied per node.
    pub link_visible_frames: i32,
}

/// Persistent controller-side subset of native `CaptureManagerClass`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaptureManagerState {
    /// Signed `WeaponType.Damage`, captured when the manager is constructed.
    pub max_control: i32,
    /// `WeaponType.InfiniteMindControl`; skips the capacity gate when true.
    pub infinite_mind_control: bool,
    /// Native MCNode construction order, including each node's saved original
    /// House owner. Victim-side controller identity is stored reciprocally on
    /// `GameEntity` and validated on snapshot admission.
    pub controlled_nodes: Vec<CaptureNodeState>,
}

impl CaptureManagerState {
    /// `FUN_004722A0`, called by `TechnoClass::ShouldRetaliate`: infinite
    /// managers never block; finite managers block when `max <= count`.
    pub fn blocks_retaliation(&self) -> bool {
        if self.infinite_mind_control {
            return false;
        }
        let count = i32::try_from(self.controlled_nodes.len()).unwrap_or(i32::MAX);
        self.max_control <= count
    }

    /// Register one native MCNode-equivalent link. `CanCapture` rejects an
    /// already-controlled target, so a stable id can occur at most once.
    pub(crate) fn link_controlled_entity(
        &mut self,
        victim_id: u64,
        original_owner: InternedId,
        capture_frame: i32,
        link_visible_frames: i32,
    ) {
        if !self
            .controlled_nodes
            .iter()
            .any(|node| node.victim_id == victim_id)
        {
            self.controlled_nodes.push(CaptureNodeState {
                victim_id,
                original_owner,
                capture_frame,
                link_visible_frames,
            });
        }
    }

    /// Pointer-expiry listener: a victim's UnInit removes its MCNode before a
    /// later receiver/attacker can observe this manager's capacity.
    pub(crate) fn pointer_expired(&mut self, stable_id: u64) {
        self.controlled_nodes
            .retain(|node| node.victim_id != stable_id);
    }
}

/// Exact active-YR `CaptureManagerClass::CanCapture @ 0x00471C90` admission.
///
/// House identity is deliberately a literal inequality. Native does not ask
/// the alliance graph here, so a target owned by a distinct allied House is
/// capturable while a target owned by the controller's own House is not.
pub(crate) fn can_capture(
    sim: &Simulation,
    rules: &RuleSet,
    controller_id: u64,
    target_id: u64,
    current_frame: u32,
) -> bool {
    let Some(controller) = sim.substrate.entities.get(controller_id) else {
        return false;
    };
    let Some(manager) = controller.capture_manager.as_ref() else {
        return false;
    };
    let Some(target) = sim.substrate.entities.get(target_id) else {
        return false;
    };

    if !target.is_object_alive() || !target.is_alive() || target.owner == controller.owner {
        return false;
    }
    let Some(target_type) = rules.object(sim.interner.resolve(target.type_ref)) else {
        return false;
    };
    if target_type.immune_to_psionics {
        return false;
    }

    // Techno+0x2E4 is the class-wide dock/bunker reciprocal partner. The
    // native special rejection is category-exact: only Infantry with a live
    // partner is rejected. An installed Unit/tank remains capturable.
    if target.category == crate::map::entities::EntityCategory::Infantry
        && (target.bunker_link.installed_in().is_some() || target.bunker_occupant.is_some())
    {
        return false;
    }
    if target.is_mind_controlled()
        || target.temporary_owner_transfer_marker.is_some()
        || crate::sim::superweapon::invulnerability::is_invulnerable(
            target.invulnerability.as_ref(),
            current_frame,
        )
    {
        return false;
    }

    let count = i32::try_from(manager.controlled_nodes.len()).unwrap_or(i32::MAX);
    if !manager.infinite_mind_control && count >= manager.max_control && manager.max_control != 1 {
        return false;
    }

    !matches!(
        target.mission.current().known(),
        Some(MissionType::Construction | MissionType::Selling)
    )
}

/// `CaptureManagerClass::CaptureUnit @ 0x00471D40` state/ownership transaction.
/// The caller must run this synchronously at the MindControl warhead detonation
/// rung so later projectiles observe the new owner and reciprocal link.
pub(crate) fn capture_unit(
    sim: &mut Simulation,
    rules: &RuleSet,
    controller_id: u64,
    target_id: u64,
    current_frame: u32,
) -> bool {
    if !can_capture(sim, rules, controller_id, target_id, current_frame) {
        return false;
    }

    let override_victims = sim
        .substrate
        .entities
        .get(controller_id)
        .and_then(|controller| controller.capture_manager.as_ref())
        .filter(|manager| !manager.infinite_mind_control && manager.max_control == 1)
        .map(|manager| {
            manager
                .controlled_nodes
                .iter()
                .rev()
                .map(|node| node.victim_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for victim_id in override_victims {
        let _ = free_unit(sim, rules, controller_id, victim_id);
    }

    let Some((controller_owner, original_owner)) = sim
        .substrate
        .entities
        .get(controller_id)
        .zip(sim.substrate.entities.get(target_id))
        .map(|(controller, target)| (controller.owner, target.owner))
    else {
        return false;
    };
    sim.change_owner_for_mind_control(target_id, controller_owner, rules, controller_id);
    if sim
        .substrate
        .entities
        .get(target_id)
        .is_none_or(|target| target.owner != controller_owner)
    {
        return false;
    }

    {
        let Some(controller) = sim.substrate.entities.get_mut(controller_id) else {
            return false;
        };
        let Some(manager) = controller.capture_manager.as_mut() else {
            return false;
        };
        manager.link_controlled_entity(
            target_id,
            original_owner,
            current_frame as i32,
            rules.general.mind_control_attack_line_frames,
        );
    }
    let Some(target) = sim.substrate.entities.get_mut(target_id) else {
        if let Some(manager) = sim
            .substrate
            .entities
            .get_mut(controller_id)
            .and_then(|controller| controller.capture_manager.as_mut())
        {
            manager.pointer_expired(target_id);
        }
        return false;
    };
    target.mind_control_controller_id = Some(controller_id);

    if !capture_clear_to_guard_is_skipped(target, rules, sim.interner.resolve(target.type_ref)) {
        clear_to_guard(sim, target_id, current_frame);
    }
    decide_unit_fate(sim, rules, controller_id, target_id, current_frame);
    create_control_ring(sim, rules, target_id);
    true
}

fn capture_clear_to_guard_is_skipped(target: &crate::sim::game_entity::GameEntity, rules: &RuleSet, type_id: &str) -> bool {
    let mission = target.mission.current().raw();
    mission == 0x12
        || mission == 0x13
        || (target.category == EntityCategory::Unit
            && mission == 0x10
            && rules.object(type_id).is_some_and(|object| object.is_simple_deployer))
}

/// Shared relevant-vtable `+0x3D0 -> 0x0070F850` continuation used by all
/// four Techno families. It is not the movement Scatter helper.
fn clear_to_guard(sim: &mut Simulation, victim_id: u64, current_frame: u32) {
    let Some(victim) = sim.substrate.entities.get_mut(victim_id) else {
        return;
    };
    crate::sim::mission::concrete_effects::represented_assign_destination_mode_one(victim, None);
    crate::sim::mission::concrete_effects::represented_assign_target(victim, None);
    victim.rally_target = None;
    if let Some(miner) = victim.miner.as_mut() {
        miner.last_harvest_cell = None;
    }
    let _ = sim.mission_assign_exact(
        victim_id,
        MissionId::from_known(MissionType::Guard),
        current_frame,
    );
}

fn entity_type_identity(
    entity: &crate::sim::game_entity::GameEntity,
) -> TeamMemberTypeIdentity {
    TeamMemberTypeIdentity {
        category: match entity.category {
            EntityCategory::Unit => ObjectCategory::Vehicle,
            EntityCategory::Infantry => ObjectCategory::Infantry,
            EntityCategory::Aircraft => ObjectCategory::Aircraft,
            EntityCategory::Structure => ObjectCategory::Building,
        },
        id: entity.type_ref,
    }
}

fn entity_distance(sim: &Simulation, lhs: u64, rhs: u64) -> Option<i32> {
    let lhs = sim.anim_owner_coords(lhs)?;
    let rhs = sim.anim_owner_coords(rhs)?;
    Some(distance_3d_leptons(
        [lhs.x, lhs.y, lhs.z],
        [rhs.x, rhs.y, rhs.z],
    ))
}

#[derive(Clone, Copy)]
enum HouseCaptureFacilityVector {
    Grinder,
    Absorber,
}

fn nearest_owned_building(
    sim: &Simulation,
    rules: &RuleSet,
    victim_id: u64,
    vector: HouseCaptureFacilityVector,
    accepts: impl Fn(&Simulation, &ObjectType, u64, u64) -> bool,
) -> Option<u64> {
    let owner = sim.substrate.entities.get(victim_id)?.owner;
    let house = sim.houses.get(&owner)?;
    let facility_order = match vector {
        HouseCaptureFacilityVector::Grinder => &house.grinder_building_order,
        HouseCaptureFacilityVector::Absorber => &house.absorber_building_order,
    };
    let mut best = None;
    let mut best_distance = i32::MAX;
    for &candidate_id in facility_order.iter().rev() {
        let Some(candidate) = sim.substrate.entities.get(candidate_id) else {
            continue;
        };
        if candidate.category != EntityCategory::Structure || candidate.owner != owner {
            continue;
        }
        let Some(object) = rules.object(sim.interner.resolve(candidate.type_ref)) else {
            continue;
        };
        if !accepts(sim, object, victim_id, candidate_id) {
            continue;
        }
        let Some(distance) = entity_distance(sim, victim_id, candidate_id) else {
            continue;
        };
        if distance < best_distance {
            best_distance = distance;
            best = Some(candidate_id);
        }
    }
    best
}

fn building_can_enter_absorber(
    sim: &Simulation,
    rules: &RuleSet,
    object: &ObjectType,
    victim_id: u64,
    building_id: u64,
) -> bool {
    // Active-retail exclusion for Building+0x534 (CurrentAnimState/BState):
    // only YAPOWR has InfantryAbsorb=yes; UnitAbsorb has zero stock authors and
    // 187 installed maps author no overrides. The ctor value is -1, while the
    // only zero writers are Construction/Selling (already rejected below), and
    // construction completion/ordinary Guard restore 1. Therefore every
    // active-retail absorber reaching this helper is necessarily nonzero. Do
    // not alias this native field to `building_damage_state_active`; a custom
    // UnitAbsorb producer would require its own separately evidenced BState.
    let Some(victim) = sim.substrate.entities.get(victim_id) else {
        return false;
    };
    let Some(building) = sim.substrate.entities.get(building_id) else {
        return false;
    };
    if building.dying
        || building.health.current <= 0
        || !crate::map::houses::is_allied_with(
        &sim.house_alliances,
        sim.interner.resolve(building.owner),
        sim.interner.resolve(victim.owner),
    ) || matches!(building.mission.current().raw(), 0x12 | 0x13)
    {
        return false;
    }
    let Some(victim_object) = rules.object(sim.interner.resolve(victim.type_ref)) else {
        return false;
    };
    if (victim_object.movement_zone != MovementZone::Amphibious
        && object.naval != victim_object.naval)
        || victim_object.balloon_hover
        || !crate::sim::power_system::is_building_powered(
            &sim.power_states,
            rules,
            building,
            &sim.interner,
        )
    {
        return false;
    }
    let category_admitted = match victim.category {
        EntityCategory::Infantry => object.infantry_absorb,
        EntityCategory::Unit => object.unit_absorb,
        _ => false,
    };
    if !category_admitted
        || victim
            .capture_manager
            .as_ref()
            .is_some_and(CaptureManagerState::blocks_retaliation)
    {
        return false;
    }
    let victim_size = victim_object.size;
    building
        .passenger_role
        .cargo()
        .is_some_and(|cargo| cargo.can_accept(victim_size))
}

fn detach_outgoing_temporal(sim: &mut Simulation, owner_id: u64) {
    let Some(manager) = sim
        .substrate
        .entities
        .get(owner_id)
        .and_then(|owner| owner.temporal_manager)
    else {
        return;
    };
    let Some(target_id) = manager.target_id else {
        return;
    };
    if let Some(previous_id) = manager.previous_owner_id {
        if let Some(previous) = sim
            .substrate
            .entities
            .get_mut(previous_id)
            .and_then(|owner| owner.temporal_manager.as_mut())
        {
            previous.next_owner_id = manager.next_owner_id;
        }
    } else if let Some(target) = sim.substrate.entities.get_mut(target_id) {
        target.temporal_targeting_me_id = manager.next_owner_id;
        if manager.next_owner_id.is_none() {
            target.being_temporally_warped_out = false;
        }
    }
    if let Some(next_id) = manager.next_owner_id
        && let Some(next) = sim
            .substrate
            .entities
            .get_mut(next_id)
            .and_then(|owner| owner.temporal_manager.as_mut())
    {
        next.previous_owner_id = manager.previous_owner_id;
        if manager.previous_owner_id.is_none() {
            next.warp_points = next.warp_points.wrapping_add(manager.warp_points);
        }
    }
    if let Some(detached) = sim
        .substrate
        .entities
        .get_mut(owner_id)
        .and_then(|owner| owner.temporal_manager.as_mut())
    {
        detached.target_id = None;
        detached.previous_owner_id = None;
        detached.next_owner_id = None;
        detached.warp_points = 0;
    }
}

fn health_below_wounded_mark(
    health: crate::sim::components::Health,
    strength: i32,
    mark: NativeF32Bits,
) -> bool {
    if strength == 0 {
        return false;
    }
    let Ok(current_f32) = X87Chop53::store_f32(X87Chop53::load_i32(i32::from(health.current)))
    else {
        return false;
    };
    let Ok(ratio) = X87Chop53::div(
        X87Chop53::load_f32(current_f32).expect("a finite i32 stores as finite f32"),
        X87Chop53::load_i32(strength),
    ) else {
        return false;
    };
    let Ok(mark) = X87Chop53::load_f32(mark) else {
        return false;
    };
    X87Chop53::compare(ratio, mark) == X87Ordering::Less
}

fn x87_mul_f32(lhs: NativeF32Bits, rhs: NativeF32Bits) -> NativeF32Bits {
    let lhs = X87Chop53::load_f32(lhs).expect("rules f32 multiplier is finite");
    let rhs = X87Chop53::load_f32(rhs).expect("rules f32 multiplier is finite");
    X87Chop53::store_f32(X87Chop53::mul(lhs, rhs))
        .expect("active-retail cost multiplier product fits f32")
}

fn x87_ftol_i32_product(base: i32, factors: &[NativeF32Bits]) -> i32 {
    let mut value = X87Chop53::load_i32(base);
    for factor in factors {
        value = X87Chop53::mul(
            value,
            X87Chop53::load_f32(*factor).expect("rules f32 multiplier is finite"),
        );
    }
    X87Chop53::ftol_i64(value).expect("active-retail refund fits signed i32") as i32
}

/// Active-retail projection of the f32 slots rebuilt by
/// `HouseClass::CalculateCostMultipliers @ 0x0050BF60`.
///
/// Retail has exactly one FactoryPlant type (`NAINDP`, UnitsCostBonus=.75), so
/// the live owner/type filter is order-independent. Multiple custom
/// FactoryPlant types would need the native House-owned FactoryPlant vector
/// before their intermediate f32 stores can be claimed; tactical/BuildConst
/// order is deliberately not substituted here.
fn active_retail_factory_plant_bonus(
    sim: &Simulation,
    rules: &RuleSet,
    owner: InternedId,
    refunded: &ObjectType,
) -> NativeF32Bits {
    let mut accumulated = NativeF32Bits::ONE;
    for &stable_id in sim.tactical_registration_order() {
        let Some(entity) = sim.substrate.entities.get(stable_id) else {
            continue;
        };
        if entity.owner != owner
            || entity.category != EntityCategory::Structure
            || !entity.is_object_alive()
            || entity.lifecycle.in_limbo
        {
            continue;
        }
        let Some(factory_plant) = rules.object(sim.interner.resolve(entity.type_ref)) else {
            continue;
        };
        if factory_plant.factory_plant {
            accumulated = x87_mul_f32(
                accumulated,
                factory_plant.factory_cost_bonus_for(refunded),
            );
        }
    }
    accumulated
}

/// Exact active Grinder refund leaf (`TechnoTypeClass` wrapper `0x0070ADA0`,
/// leaf `0x00711F60`). The returned signed credits are applied to the
/// facility's *current* House by the per-cell transaction.
fn grinder_refund_value(
    sim: &Simulation,
    rules: &RuleSet,
    refunded: &ObjectType,
    refund_house: Option<InternedId>,
) -> i32 {
    let refund_percent = X87Chop53::store_f32(
        X87Chop53::load_f64(rules.general.refund_percent)
            .expect("Rules RefundPercent is finite binary64"),
    )
    .expect("Rules RefundPercent narrows to finite binary32");
    let Some(owner) = refund_house else {
        return x87_ftol_i32_product(refunded.cost, &[refund_percent]);
    };
    let Some(house) = sim.houses.get(&owner) else {
        return x87_ftol_i32_product(refunded.cost, &[refund_percent]);
    };
    let country_bonus = house
        .country
        .map(|country| rules.country_cost_bonus(sim.interner.resolve(country), refunded))
        .unwrap_or(NativeF32Bits::ONE);
    if refunded.soylent != 0 {
        return x87_ftol_i32_product(refunded.soylent, &[country_bonus]);
    }

    let accumulated = active_retail_factory_plant_bonus(sim, rules, owner, refunded);
    let value = x87_ftol_i32_product(refunded.cost, &[accumulated, country_bonus]);
    if house.is_controlled_by_human(sim.session.game_mode_nonzero) {
        x87_ftol_i32_product(value, &[refund_percent])
    } else {
        value
    }
}

fn fate_weights<'a>(
    sim: &Simulation,
    rules: &'a RuleSet,
    controller_id: u64,
    victim_id: u64,
) -> &'a [i32] {
    let Some(victim) = sim.substrate.entities.get(victim_id) else {
        return &rules.general.ai_capture_normal;
    };
    let Some(controller_owner) = sim
        .substrate
        .entities
        .get(controller_id)
        .map(|controller| controller.owner)
    else {
        return &rules.general.ai_capture_normal;
    };
    let victim_strength = rules
        .object(sim.interner.resolve(victim.type_ref))
        .map_or(0, |object| object.strength);
    if sim
        .houses
        .get(&controller_owner)
        .is_some_and(|house| house.credits < rules.general.ai_capture_low_money_mark)
    {
        &rules.general.ai_capture_low_money
    } else if sim
        .power_states
        .get(&controller_owner)
        .is_some_and(|power| power.total_drain != 0 && power.total_output < power.total_drain)
    {
        &rules.general.ai_capture_low_power
    } else if health_below_wounded_mark(
        victim.health,
        victim_strength,
        rules.general.ai_capture_wounded_mark,
    ) {
        &rules.general.ai_capture_wounded
    } else {
        &rules.general.ai_capture_normal
    }
}

fn select_fate_action(
    sim: &mut Simulation,
    rules: &RuleSet,
    controller_id: u64,
    victim_id: u64,
) -> Option<i32> {
    let weights = fate_weights(sim, rules, controller_id, victim_id).to_vec();
    let roll = sim.scenario_rng.next_range_i32_inclusive(1, 100);
    let mut cumulative = 0i32;
    let mut selected = None;
    for (index, weight) in weights.into_iter().take(6).enumerate() {
        cumulative = cumulative.wrapping_add(weight);
        if roll <= cumulative {
            selected = Some(index as i32 + 1);
            break;
        }
    }
    let override_action = sim
        .team_script_vm
        .mind_control_decision_for_member(controller_id);
    if override_action != 0 {
        Some(override_action)
    } else {
        selected
    }
}

fn assign_mission(sim: &mut Simulation, victim_id: u64, mission: MissionType, current_frame: u32) {
    let _ = sim.mission_assign_exact(victim_id, MissionId::from_known(mission), current_frame);
}

fn hunt(sim: &mut Simulation, victim_id: u64, current_frame: u32) {
    assign_mission(sim, victim_id, MissionType::Hunt, current_frame);
}

fn decide_unit_fate(
    sim: &mut Simulation,
    rules: &RuleSet,
    controller_id: u64,
    victim_id: u64,
    current_frame: u32,
) {
    let Some((identity, victim_owner, open_topped, passengers, victim_category)) = sim
        .substrate
        .entities
        .get(victim_id)
        .map(|victim| {
            let object = rules.object(sim.interner.resolve(victim.type_ref));
            (
                entity_type_identity(victim),
                victim.owner,
                object.is_some_and(|object| object.open_topped),
                victim
                    .passenger_role
                    .cargo()
                    .map(|cargo| cargo.passengers.clone())
                    .unwrap_or_default(),
                victim.category,
            )
        })
    else {
        return;
    };

    let _ = sim.team_script_vm.remove_member(victim_id, identity);
    detach_outgoing_temporal(sim, victim_id);
    if open_topped {
        for passenger_id in passengers {
            if let Some(passenger) = sim.substrate.entities.get_mut(passenger_id) {
                crate::sim::mission::concrete_effects::represented_assign_target(passenger, None);
            }
        }
    }
    if sim.houses.get(&victim_owner).is_some_and(|house| house.is_human) {
        return;
    }

    // A short authored vector can leave the 1..100 roll above the final
    // cumulative weight. Native falls through switch value 0, whose default
    // is the same Hunt continuation as actions 4/6; only action 5 is a no-op.
    let action = select_fate_action(sim, rules, controller_id, victim_id).unwrap_or(0);
    match action {
        1 => {
            let controller_owner = sim.substrate.entities.get(controller_id).map(|entity| entity.owner);
            let controller_is_foot = sim
                .substrate
                .entities
                .get(controller_id)
                .is_some_and(|controller| controller.category != EntityCategory::Structure);
            if controller_is_foot
                && controller_owner == Some(victim_owner)
                && sim.team_script_vm.add_member_to_controller_team(
                    controller_id,
                    TeamScriptMember {
                        entity_id: victim_id,
                        member_type: identity,
                    },
                )
            {
                return;
            }
            hunt(sim, victim_id, current_frame);
        }
        2 if victim_category != EntityCategory::Structure => {
            let grinder = nearest_owned_building(
                sim,
                rules,
                victim_id,
                HouseCaptureFacilityVector::Grinder,
                |_, object, _, _| object.grinding,
            );
            if let Some(grinder_id) = grinder {
                if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
                    crate::sim::mission::concrete_effects::represented_assign_destination_mode_one(
                        victim,
                        Some(NavTargetRef::building(grinder_id)),
                    );
                    victim.navigation.pending_arrival_clear = true;
                }
                assign_mission(sim, victim_id, MissionType::Eaten, current_frame);
            } else {
                hunt(sim, victim_id, current_frame);
            }
        }
        3 if victim_category != EntityCategory::Structure => {
            let absorber = nearest_owned_building(
                sim,
                rules,
                victim_id,
                HouseCaptureFacilityVector::Absorber,
                |sim, object, victim, building| {
                    building_can_enter_absorber(sim, rules, object, victim, building)
                },
            );
            if let Some(absorber_id) = absorber {
                let should_retarget = sim.substrate.entities.get(victim_id).is_some_and(|victim| {
                    victim.radio_contacts.slot(0) != Some(absorber_id)
                        && victim.navigation.nav_com != Some(NavTargetRef::building(absorber_id))
                });
                if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
                    victim.ai_absorb_enter_pending = true;
                }
                if should_retarget {
                    assign_mission(sim, victim_id, MissionType::Enter, current_frame);
                    if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
                        crate::sim::mission::concrete_effects::represented_assign_destination_mode_one(
                            victim,
                            Some(NavTargetRef::building(absorber_id)),
                        );
                    }
                    establish_capture_fate_contact(sim, victim_id, absorber_id);
                    if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
                        victim.navigation.pending_arrival_clear = true;
                    }
                }
            } else {
                if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
                    victim.ai_absorb_enter_pending = false;
                }
                hunt(sim, victim_id, current_frame);
            }
        }
        5 => {}
        _ => hunt(sim, victim_id, current_frame),
    }
}

/// The HELLO/contact side effect inside `FootClass::Set_Destination_Internal`
/// for Mission Enter. Action 3 assigns mission 7 first, then calls the setter;
/// action 2's mission 9 setter path deliberately never comes here.
fn establish_capture_fate_contact(sim: &mut Simulation, victim_id: u64, building_id: u64) {
    let _ = crate::sim::radio::transmit_pre_admitted_hello(sim, victim_id, building_id);
}

pub(crate) fn capture_fate_force_enter(sim: &Simulation, victim_id: u64) -> bool {
    // `FootClass::Mission_Enter @ 0x004D9290` tests the class-common byte at
    // Techno+0x418 after radio 0x0E. Rust's exact represented owner is the
    // dock-entered link, and the gate is not Unit-only.
    sim.substrate
        .entities
        .get(victim_id)
        .is_some_and(|victim| victim.dock_entered_with.is_some())
}

fn create_control_ring(sim: &mut Simulation, rules: &RuleSet, victim_id: u64) {
    let Some(type_name) = rules.general.controlled_animation_type.as_deref() else {
        return;
    };
    let Some((type_ref, category, position)) = sim
        .substrate
        .entities
        .get(victim_id)
        .map(|victim| (victim.type_ref, victim.category, victim.position.clone()))
    else {
        return;
    };
    let Some(object) = rules.object(sim.interner.resolve(type_ref)) else {
        return;
    };
    let Some(mut world) = sim.anim_owner_coords(victim_id) else {
        return;
    };
    world.z = if category == EntityCategory::Structure {
        let art_height = rules
            .art_registry
            .get(&object.image)
            .or_else(|| rules.art_registry.get(&object.id))
            .map_or(2, |art| art.height);
        world.z.wrapping_add(art_height.wrapping_mul(104))
    } else {
        world.z.wrapping_add(object.mind_control_ring_offset)
    };
    let type_id = sim.interner.intern(type_name);
    let mut descriptor = AnimClassSpawnDescriptor::new(
        type_id,
        position.rx,
        position.ry,
        position.sub_x,
        position.sub_y,
        position.z,
    );
    descriptor.draw_flags = 0x600;
    let Ok(anim_id) = sim.spawn_anim_at_world(rules, descriptor, world) else {
        return;
    };
    if !sim.set_anim_owner_object(anim_id, Some(victim_id)) {
        sim.destroy_anim(anim_id);
        return;
    }
    if category == EntityCategory::Structure {
        sim.set_anim_frame_and_z_adjust(anim_id, 0, -1024);
    }
    if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
        victim.mind_control_anim_id = Some(anim_id);
    }
}

fn victim_sound_coord(sim: &Simulation, victim_id: u64) -> Option<AnimWorldCoord> {
    // The sound call samples the target's Object/Techno GetCoords result.
    // `anim_owner_coords` is the shared projection, including a structure's
    // foundation-center adjustment and exact world Z when present.
    sim.anim_owner_coords(victim_id)
}

pub(crate) fn emit_capture_sound_after_success(
    sim: &mut Simulation,
    rules: &RuleSet,
    controller_id: u64,
    target_id: u64,
    target_was_human: bool,
) {
    let controller_is_human = sim
        .substrate
        .entities
        .get(controller_id)
        .is_some_and(|controller| is_human_player_exact(sim, controller.owner));
    if !target_was_human && !controller_is_human {
        return;
    }
    let Some(sound_id) = rules.general.yuri_mind_control_sound.clone() else {
        return;
    };
    let Some(world) = victim_sound_coord(sim, target_id) else {
        return;
    };
    sim.sound_events.push(SimSoundEvent::MindControlSound { sound_id, world });
}

/// Exact `HouseClass::IsHumanPlayer @ 0x0050B6F0` projection used by the
/// successful mind-control sound gate. Nonzero game modes compare the live
/// player pointer; mode zero accepts either human-seat byte.
pub(crate) fn is_human_player_exact(sim: &Simulation, owner: InternedId) -> bool {
    sim.houses.get(&owner).is_some_and(|house| {
        if sim.session.game_mode_nonzero {
            house.player_control
        } else {
            house.is_human || house.player_control
        }
    })
}

/// Release one reversible MCNode in native order: remove its ring, emit the
/// cleared sound, restore owner and run AI fate while the reciprocal node still
/// exists, then clear the victim backlink and compact the controller vector.
pub(crate) fn free_unit(
    sim: &mut Simulation,
    rules: &RuleSet,
    controller_id: u64,
    victim_id: u64,
) -> bool {
    if !sim.substrate.entities.contains(victim_id) {
        return false;
    }
    let matches = sim
        .substrate
        .entities
        .get(controller_id)
        .and_then(|controller| controller.capture_manager.as_ref())
        .map(|manager| {
            manager
                .controlled_nodes
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, node)| node.victim_id == victim_id)
                .map(|(index, node)| (index, node.original_owner))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for (index, original_owner) in matches {
        if let Some(anim_id) = sim
            .substrate
            .entities
            .get(victim_id)
            .and_then(|victim| victim.mind_control_anim_id)
        {
            sim.destroy_anim(anim_id);
        }
        let cleared_sound = sim
            .substrate
            .entities
            .get(victim_id)
            .and_then(|victim| {
                rules
                    .object(sim.interner.resolve(victim.type_ref))
                    .and_then(|object| object.mind_cleared_sound.clone())
            })
            .or_else(|| rules.general.mind_cleared_sound.clone());
        if let Some(sound_id) = cleared_sound
            && let Some(world) = victim_sound_coord(sim, victim_id)
        {
            sim.sound_events
                .push(SimSoundEvent::MindControlSound { sound_id, world });
        }
        sim.change_owner_with_rules(victim_id, original_owner, rules);
        let current_frame = sim.session.binary_frame;
        decide_unit_fate(
            sim,
            rules,
            controller_id,
            victim_id,
            current_frame,
        );
        if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
            if victim.mind_control_controller_id == Some(controller_id) {
                victim.mind_control_controller_id = None;
            }
        }
        if let Some(manager) = sim
            .substrate
            .entities
            .get_mut(controller_id)
            .and_then(|controller| controller.capture_manager.as_mut())
            && index < manager.controlled_nodes.len()
            && manager.controlled_nodes[index].victim_id == victim_id
        {
            manager.controlled_nodes.remove(index);
        }
    }
    true
}

/// Release every victim tail-to-head, matching native `FreeAll`'s reverse
/// compaction walk rather than snapshotting owner state from the victims.
pub(crate) fn free_all(sim: &mut Simulation, rules: &RuleSet, controller_id: u64) {
    let victims = sim
        .substrate
        .entities
        .get(controller_id)
        .and_then(|controller| controller.capture_manager.as_ref())
        .map(|manager| {
            manager
                .controlled_nodes
                .iter()
                .rev()
                .map(|node| node.victim_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for victim_id in victims {
        let _ = free_unit(sim, rules, controller_id, victim_id);
    }
}

/// Psychic Dominator's irreversible conversion: release any reversible node,
/// transfer ownership, then set the independent permanent byte. It creates no
/// MCNode and ordinary producers therefore never create both states.
pub(crate) fn make_permanent(
    sim: &mut Simulation,
    rules: &RuleSet,
    target_id: u64,
    new_owner: InternedId,
) -> bool {
    if let Some(controller_id) = sim
        .substrate
        .entities
        .get(target_id)
        .and_then(|target| target.mind_control_controller_id)
    {
        let _ = free_unit(sim, rules, controller_id, target_id);
    }
    if !sim.substrate.entities.contains(target_id) {
        return false;
    }
    sim.change_owner_with_rules(target_id, new_owner, rules);
    let Some(target) = sim.substrate.entities.get_mut(target_id) else {
        return false;
    };
    if target.owner != new_owner {
        return false;
    }
    target.permanently_mind_controlled = true;
    true
}

/// TechnoClass::Init_Managers construction predicate and immutable snapshots.
pub(crate) fn init_capture_manager(
    object_type: &ObjectType,
    rules: &RuleSet,
) -> Option<CaptureManagerState> {
    let weapon = rules.weapon(object_type.primary.as_deref()?)?;
    let warhead = rules.warhead(weapon.warhead.as_deref()?)?;
    warhead.mind_control.then(|| CaptureManagerState {
        max_control: weapon.damage,
        infinite_mind_control: weapon.infinite_mind_control,
        controlled_nodes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::combat::{AttackTarget, TargetKind};
    use crate::sim::game_entity::{BunkerLink, GameEntity};
    use crate::sim::house_state::HouseState;
    use crate::sim::mission::state::MissionTestFixture;
    use crate::sim::mission::{MissionDispatchTimer, MissionId};
    use crate::sim::power_system::PowerState;
    use crate::sim::team_script_vm::{
        TeamScriptAction, TeamScriptDefinition, TeamTaskForceDefinition, TeamTaskForceEntry,
        TeamTypeDefinition,
    };
    use crate::rules::team_ai_ini::TeamAiDefinitionSource;

    fn capture_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\nFixtureOnly=1\n\
             [AudioVisual]\nMindClearedSound=GlobalClear\n\
             [VehicleTypes]\n0=CTRL\n1=TARGET\n2=IMMUNE\n3=OTHER\n\
             [InfantryTypes]\n0=INF\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [CTRL]\nStrength=100\nPrimary=MIND\n\
             [TARGET]\nStrength=100\nIsSimpleDeployer=yes\nMindClearedSound=TypeClear\n\
             [IMMUNE]\nStrength=100\nImmuneToPsionics=yes\n\
             [OTHER]\nStrength=100\n\
             [INF]\nStrength=100\n\
             [MIND]\nDamage=1\nWarhead=CONTROLLER\n\
             [CONTROLLER]\nMindControl=yes\n",
        ))
        .expect("mind-control fixture rules")
    }

    #[test]
    fn grinder_refund_uses_native_soylent_country_factory_and_human_branches() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[Countries]\n0=TestCountry\n[TestCountry]\nCostUnitsMult=.8\n[General]\nRefundPercent=50%\n[VehicleTypes]\n0=ZERO\n1=SOYL\n[BuildingTypes]\n0=NAINDP\n[ZERO]\nCost=1000\nSoylent=0\n[SOYL]\nCost=1000\nSoylent=250\n[NAINDP]\nFactoryPlant=yes\nUnitsCostBonus=.75\nStrength=100\n",
        ))
        .expect("refund fixture rules");
        let mut sim = Simulation::new();
        sim.session.game_mode_nonzero = true;
        let owner = sim.interner.intern("Owner");
        let country = sim.interner.intern("TestCountry");
        sim.houses.insert(
            owner,
            HouseState::new(owner, 0, Some(country), true, 0, 10),
        );
        sim.session.house_order.push(owner);

        let plant_id = sim.allocate_stable_id();
        let mut plant = GameEntity::test_default(plant_id, "NAINDP", "Owner", 5, 5);
        plant.owner = owner;
        plant.type_ref = sim.interner.intern("NAINDP");
        plant.category = EntityCategory::Structure;
        sim.substrate.entities.insert(plant);
        let _ = sim.reveal(plant_id);
        assert!(sim.tactical_registration_order().contains(&plant_id));

        let zero = rules.object("ZERO").expect("zero-soylent type");
        let soylent = rules.object("SOYL").expect("nonzero-soylent type");
        assert_eq!(
            grinder_refund_value(&sim, &rules, zero, Some(owner)),
            300,
            "ftol(1000 * .75 * .8)=600, then human ftol(600 * .5)=300",
        );
        assert_eq!(
            grinder_refund_value(&sim, &rules, soylent, Some(owner)),
            200,
            "nonzero Soylent ignores FactoryPlant and RefundPercent: ftol(250 * .8)",
        );
        assert_eq!(
            grinder_refund_value(&sim, &rules, zero, None),
            500,
            "null House uses only base Cost and narrowed RefundPercent",
        );

        let second_plant_id = sim.allocate_stable_id();
        let mut second_plant =
            GameEntity::test_default(second_plant_id, "NAINDP", "Owner", 6, 5);
        second_plant.owner = owner;
        second_plant.type_ref = sim.interner.intern("NAINDP");
        second_plant.category = EntityCategory::Structure;
        sim.substrate.entities.insert(second_plant);
        let _ = sim.reveal(second_plant_id);
        assert!(sim.tactical_registration_order().contains(&second_plant_id));
        assert_eq!(
            grinder_refund_value(&sim, &rules, zero, Some(owner)),
            225,
            "two live NAINDP instances store .75*.75 in f32 before cost/country/refund",
        );
        sim.substrate
            .entities
            .get_mut(second_plant_id)
            .expect("second plant")
            .lifecycle
            .in_limbo = true;

        sim.houses.get_mut(&owner).expect("owner").is_human = false;
        assert_eq!(
            grinder_refund_value(&sim, &rules, zero, Some(owner)),
            600,
            "AI House does not apply RefundPercent",
        );
        sim.substrate
            .entities
            .get_mut(plant_id)
            .expect("plant")
            .lifecycle
            .in_limbo = true;
        assert_eq!(
            grinder_refund_value(&sim, &rules, zero, Some(owner)),
            800,
            "limbo FactoryPlant is excluded from the live accumulated slot",
        );
    }

    #[test]
    fn grinder_refund_two_stage_ftol_preserves_native_rounding_edge() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[Countries]\n0=EdgeCountry\n[EdgeCountry]\nCostUnitsMult=.77\n[General]\nRefundPercent=91%\n[VehicleTypes]\n0=EDGE\n[BuildingTypes]\n0=EDGEPLANT\n[EDGE]\nCost=2\n[EDGEPLANT]\nFactoryPlant=yes\nUnitsCostBonus=.73\nStrength=100\n",
        ))
        .expect("rounding fixture rules");
        let mut sim = Simulation::new();
        sim.session.game_mode_nonzero = true;
        let owner = sim.interner.intern("Owner");
        let country = sim.interner.intern("EdgeCountry");
        sim.houses.insert(
            owner,
            HouseState::new(owner, 0, Some(country), true, 0, 10),
        );
        let plant_id = sim.allocate_stable_id();
        let mut plant = GameEntity::test_default(plant_id, "EDGEPLANT", "Owner", 5, 5);
        plant.owner = owner;
        plant.type_ref = sim.interner.intern("EDGEPLANT");
        plant.category = EntityCategory::Structure;
        sim.substrate.entities.insert(plant);
        let _ = sim.reveal(plant_id);
        assert!(sim.tactical_registration_order().contains(&plant_id));

        assert_eq!(
            grinder_refund_value(
                &sim,
                &rules,
                rules.object("EDGE").expect("edge type"),
                Some(owner),
            ),
            0,
            "ftol(2*.73*.77)=1 then ftol(1*.91)=0; one combined ftol would be 1",
        );
    }

    fn capture_sim() -> (Simulation, RuleSet, [InternedId; 3], [u64; 3]) {
        let rules = capture_rules();
        let mut sim = Simulation::new();
        let controller_house = sim.interner.intern("Controller");
        let allied_house = sim.interner.intern("AlliedTarget");
        let third_house = sim.interner.intern("Third");
        for (side, owner) in [controller_house, allied_house, third_house]
            .into_iter()
            .enumerate()
        {
            sim.houses.insert(
                owner,
                HouseState::new(owner, side as u8, None, false, 0, 10),
            );
            sim.session.house_order.push(owner);
        }
        sim.house_alliances
            .entry("CONTROLLER".to_string())
            .or_default()
            .insert("ALLIEDTARGET".to_string());
        sim.house_alliances
            .entry("ALLIEDTARGET".to_string())
            .or_default()
            .insert("CONTROLLER".to_string());

        let controller_id = sim.allocate_stable_id();
        let first_id = sim.allocate_stable_id();
        let second_id = sim.allocate_stable_id();
        let mut controller =
            GameEntity::test_default(controller_id, "CTRL", "Controller", 5, 5);
        controller.owner = controller_house;
        controller.type_ref = sim.interner.intern("CTRL");
        controller.capture_manager = init_capture_manager(rules.object("CTRL").unwrap(), &rules);
        controller.attack_target = Some(AttackTarget {
            target: TargetKind::Entity(first_id),
            ..AttackTarget::new(first_id)
        });
        let mut first =
            GameEntity::test_default(first_id, "TARGET", "AlliedTarget", 7, 5);
        first.owner = allied_house;
        first.type_ref = sim.interner.intern("TARGET");
        let mut second =
            GameEntity::test_default(second_id, "OTHER", "AlliedTarget", 9, 5);
        second.owner = allied_house;
        second.type_ref = sim.interner.intern("OTHER");
        for entity in [controller, first, second] {
            sim.substrate.entities.insert(entity);
        }
        for id in [controller_id, first_id, second_id] {
            let _ = sim.reveal(id);
        }
        (
            sim,
            rules,
            [controller_house, allied_house, third_house],
            [controller_id, first_id, second_id],
        )
    }

    fn set_raw_mission(entity: &mut GameEntity, raw: i32) {
        entity.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_raw(raw),
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
    }

    fn install_controller_decision(
        sim: &mut Simulation,
        controller_id: u64,
        owner: InternedId,
        decision: i32,
    ) -> u64 {
        let script = sim.interner.intern(&format!("CaptureScript{decision}"));
        let task_force = sim.interner.intern(&format!("CaptureTaskForce{decision}"));
        let team_type = sim.interner.intern(&format!("CaptureTeam{decision}"));
        let identity = TeamMemberTypeIdentity {
            category: ObjectCategory::Vehicle,
            id: sim.substrate.entities.get(controller_id).unwrap().type_ref,
        };
        sim.team_script_vm.register_script(TeamScriptDefinition {
            id: script,
            actions: vec![TeamScriptAction {
                action_id: 2,
                argument: 0,
            }],
            source: TeamAiDefinitionSource::FixedAimd,
        });
        sim.team_script_vm
            .register_task_force(TeamTaskForceDefinition {
                id: task_force,
                group: -1,
                entries: vec![TeamTaskForceEntry {
                    member_type: identity,
                    count: 1,
                }],
                source: TeamAiDefinitionSource::FixedAimd,
            });
        sim.team_script_vm.register_team_type(TeamTypeDefinition {
            id: team_type,
            script_id: script,
            task_force_id: task_force,
            priority: 0,
            is_base_defense: false,
            mind_control_decision: decision,
            combined_movement_zone: MovementZone::Normal,
            base_zone_relation_enforced: false,
            transport_crossing_required: false,
        });
        sim.team_script_vm.create_team_from_type(
            owner,
            team_type,
            &[TeamScriptMember {
                entity_id: controller_id,
                member_type: identity,
            }],
            None,
            0,
        )
    }

    fn fate_facility_sim(decision: i32) -> (Simulation, RuleSet, u64, u64, u64) {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n0=CTRL\n\
             [InfantryTypes]\n0=TARGET\n\
             [BuildingTypes]\n0=GRINDER\n1=BIO\n\
             [AircraftTypes]\n\
             [CTRL]\nStrength=100\n\
             [TARGET]\nStrength=100\nSize=1\nMovementZone=Infantry\n\
             [GRINDER]\nStrength=500\nGrinding=yes\n\
             [BIO]\nStrength=500\nInfantryAbsorb=yes\nPassengers=5\nSizeLimit=1\n",
        ))
        .unwrap();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Owner");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, false, 10_000, 10));
        sim.session.house_order.push(owner);
        let controller_id = sim
            .spawn_object("CTRL", "Owner", 4, 4, 0, &rules, &Default::default())
            .unwrap();
        let victim_id = sim
            .spawn_object("TARGET", "Owner", 5, 4, 0, &rules, &Default::default())
            .unwrap();
        let facility = sim
            .spawn_object(
                if decision == 2 { "GRINDER" } else { "BIO" },
                "Owner",
                6,
                4,
                0,
                &rules,
                &Default::default(),
            )
            .unwrap();
        let _team = install_controller_decision(&mut sim, controller_id, owner, decision);
        (sim, rules, controller_id, victim_id, facility)
    }

    fn absorber_gate_sim(
        target_extra: &str,
        absorber_extra: &str,
    ) -> (Simulation, RuleSet, InternedId, u64, u64) {
        let rules = RuleSet::from_ini(&IniFile::from_str(&format!(
            "[VehicleTypes]\n\
             [InfantryTypes]\n0=TARGET\n\
             [BuildingTypes]\n0=BIO\n\
             [AircraftTypes]\n\
             [TARGET]\nStrength=100\n{target_extra}\
             [BIO]\nStrength=500\nInfantryAbsorb=yes\nPassengers=5\nSizeLimit=1\nGrinding=yes\n{absorber_extra}"
        )))
        .unwrap();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Owner");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, false, 10_000, 10));
        sim.session.house_order.push(owner);
        let victim_id = sim
            .spawn_object("TARGET", "Owner", 5, 5, 0, &rules, &Default::default())
            .unwrap();
        let building_id = sim
            .spawn_object("BIO", "Owner", 6, 5, 0, &rules, &Default::default())
            .unwrap();
        (sim, rules, owner, victim_id, building_id)
    }

    fn absorber_gate_result(
        target_extra: &str,
        absorber_extra: &str,
        mutate_sim: impl FnOnce(&mut Simulation, InternedId, u64, u64),
        mutate_type: impl FnOnce(&mut ObjectType),
    ) -> bool {
        let (mut sim, rules, owner, victim_id, building_id) =
            absorber_gate_sim(target_extra, absorber_extra);
        let mut object = rules.object("BIO").unwrap().clone();
        mutate_sim(&mut sim, owner, victim_id, building_id);
        mutate_type(&mut object);
        building_can_enter_absorber(&sim, &rules, &object, victim_id, building_id)
    }

    #[test]
    fn finite_and_infinite_capacity_match_native_helper() {
        let ini = IniFile::from_str(
            "[BuildingTypes]\n0=YAPSYT\n\
             [VehicleTypes]\n\
             [InfantryTypes]\n\
             [AircraftTypes]\n\
             [YAPSYT]\nPrimary=MultipleMindControlTower\n\
             [MultipleMindControlTower]\nDamage=3\nWarhead=Controller\n\
             [Controller]\nMindControl=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("capture-manager fixture");
        let mut manager = init_capture_manager(rules.object("YAPSYT").unwrap(), &rules).unwrap();
        assert_eq!(manager.max_control, 3);
        assert!(!manager.blocks_retaliation());
        let original_owner = InternedId::from_index(7);
        manager.link_controlled_entity(10, original_owner, 0, 20);
        manager.link_controlled_entity(11, original_owner, 0, 20);
        assert!(!manager.blocks_retaliation());
        manager.link_controlled_entity(12, original_owner, 0, 20);
        assert!(manager.blocks_retaliation());
        manager.pointer_expired(11);
        assert!(!manager.blocks_retaliation());

        manager.infinite_mind_control = true;
        manager.max_control = 0;
        assert!(!manager.blocks_retaliation());
    }

    #[test]
    fn mind_control_presentation_none_sentinels_remain_invalid() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\nControlledAnimationType=none\n\
             [AudioVisual]\nYuriMindControlSound=<none>\nMindClearedSound=none\n\
             [VehicleTypes]\n0=TARGET\n\
             [InfantryTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TARGET]\nStrength=100\nMindClearedSound=<none>\n",
        ))
        .unwrap();
        assert!(rules.general.controlled_animation_type.is_none());
        assert!(rules.general.yuri_mind_control_sound.is_none());
        assert!(rules.general.mind_cleared_sound.is_none());
        assert!(rules.object("TARGET").unwrap().mind_cleared_sound.is_none());
    }

    #[test]
    fn can_capture_uses_house_inequality_and_exact_partner_immunity_gates() {
        let (mut sim, rules, owners, [controller_id, target_id, _]) = capture_sim();
        assert!(
            can_capture(&sim, &rules, controller_id, target_id, 0),
            "distinct allied Houses remain capturable"
        );

        sim.substrate.entities.get_mut(target_id).unwrap().owner = owners[0];
        assert!(
            !can_capture(&sim, &rules, controller_id, target_id, 0),
            "same-House target is rejected"
        );
        sim.substrate.entities.get_mut(target_id).unwrap().owner = owners[1];

        sim.substrate.entities.get_mut(target_id).unwrap().bunker_link =
            BunkerLink::Installed(99);
        assert!(
            can_capture(&sim, &rules, controller_id, target_id, 0),
            "Techno+0x2E4 does not reject an installed Unit"
        );
        sim.substrate.entities.get_mut(target_id).unwrap().category = EntityCategory::Infantry;
        assert!(
            !can_capture(&sim, &rules, controller_id, target_id, 0),
            "only Infantry with a reciprocal partner is rejected"
        );

        let target = sim.substrate.entities.get_mut(target_id).unwrap();
        target.category = EntityCategory::Unit;
        target.bunker_link = BunkerLink::None;
        target.temporary_owner_transfer_marker = Some(owners[1]);
        target.temporary_owner_transfer_source = None;
        assert!(
            !can_capture(&sim, &rules, controller_id, target_id, 0),
            "marker presence rejects even when saved source is null"
        );
    }

    #[test]
    fn capture_clear_wrapper_and_native_skip_missions_precede_fate() {
        let (mut sim, rules, owners, [controller_id, target_id, _]) = capture_sim();
        sim.houses.get_mut(&owners[0]).unwrap().is_human = true;
        let target = sim.substrate.entities.get_mut(target_id).unwrap();
        set_raw_mission(target, MissionType::Attack as i32);
        target.navigation.nav_com = Some(NavTargetRef::cell(11, 12));
        target.attack_target = Some(AttackTarget::new(controller_id));
        target.rally_target = Some((21, 22));

        assert!(capture_unit(&mut sim, &rules, controller_id, target_id, 11));
        let target = sim.substrate.entities.get(target_id).unwrap();
        assert_eq!(target.navigation.nav_com, None);
        assert!(target.attack_target.is_none());
        assert_eq!(target.rally_target, None);
        assert_eq!(target.mission.current(), MissionId::from_known(MissionType::Guard));

        let (mut skipped, rules, owners, [controller_id, target_id, _]) = capture_sim();
        skipped.houses.get_mut(&owners[0]).unwrap().is_human = true;
        let target = skipped.substrate.entities.get_mut(target_id).unwrap();
        set_raw_mission(target, 0x10);
        target.navigation.nav_com = Some(NavTargetRef::cell(13, 14));
        target.attack_target = Some(AttackTarget::new(controller_id));
        assert!(capture_unit(
            &mut skipped,
            &rules,
            controller_id,
            target_id,
            12,
        ));
        let target = skipped.substrate.entities.get(target_id).unwrap();
        assert_eq!(target.navigation.nav_com, Some(NavTargetRef::cell(13, 14)));
        assert!(target.attack_target.is_some());
        assert_eq!(target.mission.current().raw(), 0x10);

        for (raw, expected) in [(0x11, false), (0x12, true), (0x13, true), (0x14, false)] {
            set_raw_mission(skipped.substrate.entities.get_mut(target_id).unwrap(), raw);
            let target = skipped.substrate.entities.get(target_id).unwrap();
            assert_eq!(
                capture_clear_to_guard_is_skipped(target, &rules, "TARGET"),
                expected,
                "raw mission {raw:#x}",
            );
        }
    }

    #[test]
    fn fate_category_reads_controller_house_and_f32_strength_contract() {
        let (mut sim, mut rules, owners, [controller_id, target_id, _]) = capture_sim();
        rules.general.ai_capture_normal = vec![1];
        rules.general.ai_capture_low_money = vec![2];
        rules.general.ai_capture_low_power = vec![3];
        rules.general.ai_capture_wounded = vec![4];
        assert_eq!(rules.general.ai_capture_wounded_mark.bits(), 0x3e80_0000);

        sim.houses.get_mut(&owners[0]).unwrap().credits = 1_999;
        sim.houses.get_mut(&owners[1]).unwrap().credits = 9_999;
        assert_eq!(
            fate_weights(&sim, &rules, controller_id, target_id),
            rules.general.ai_capture_low_money,
        );

        sim.houses.get_mut(&owners[0]).unwrap().credits = 9_999;
        sim.power_states.insert(
            owners[0],
            PowerState {
                total_output: -1,
                total_drain: 0,
                ..PowerState::default()
            },
        );
        assert_eq!(
            fate_weights(&sim, &rules, controller_id, target_id),
            rules.general.ai_capture_normal,
            "drain zero forces native GetPowerRatio to 1 even for negative output",
        );
        sim.power_states.get_mut(&owners[0]).unwrap().total_drain = 1;
        assert_eq!(
            fate_weights(&sim, &rules, controller_id, target_id),
            rules.general.ai_capture_low_power,
        );

        sim.power_states.remove(&owners[0]);
        let target = sim.substrate.entities.get_mut(target_id).unwrap();
        target.health.current = 30;
        target.health.max = 1_000;
        assert_eq!(
            fate_weights(&sim, &rules, controller_id, target_id),
            rules.general.ai_capture_normal,
            "native divides f32-rounded current HP by type Strength, not runtime max HP",
        );
        sim.substrate.entities.get_mut(target_id).unwrap().health.current = 24;
        assert_eq!(
            fate_weights(&sim, &rules, controller_id, target_id),
            rules.general.ai_capture_wounded,
        );
    }

    #[test]
    fn successful_sound_uses_mode_aware_human_player_identity() {
        let (mut sim, mut rules, owners, [controller_id, target_id, _]) = capture_sim();
        rules.general.yuri_mind_control_sound = Some("YuriCapture".to_string());
        let controller = sim.houses.get_mut(&owners[0]).unwrap();
        controller.is_human = true;
        controller.player_control = false;

        sim.session.game_mode_nonzero = true;
        assert!(!is_human_player_exact(&sim, owners[0]));
        emit_capture_sound_after_success(
            &mut sim,
            &rules,
            controller_id,
            target_id,
            false,
        );
        assert!(sim.sound_events.is_empty());

        sim.houses.get_mut(&owners[0]).unwrap().player_control = true;
        assert!(is_human_player_exact(&sim, owners[0]));
        emit_capture_sound_after_success(
            &mut sim,
            &rules,
            controller_id,
            target_id,
            false,
        );
        assert!(matches!(
            sim.sound_events.as_slice(),
            [SimSoundEvent::MindControlSound { sound_id, .. }] if sound_id == "YuriCapture"
        ));

        sim.sound_events.clear();
        sim.session.game_mode_nonzero = false;
        sim.houses.get_mut(&owners[0]).unwrap().player_control = false;
        assert!(is_human_player_exact(&sim, owners[0]));
    }

    #[test]
    fn control_ring_uses_type_offset_and_building_art_height_factor() {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\nControlledAnimationType=MINDANIM\n\
             [VehicleTypes]\n0=TARGET\n\
             [BuildingTypes]\n0=BTARGET\n\
             [InfantryTypes]\n\
             [AircraftTypes]\n\
             [TARGET]\nStrength=100\nMindControlRingOffset=37\n\
             [BTARGET]\nStrength=100\n",
        ))
        .expect("control-ring fixture rules");
        let mut art = crate::rules::art_data::ArtRegistry::from_ini(&IniFile::from_str(
            "[BTARGET]\nHeight=3\n\
             [MINDANIM]\nEnd=1\nLoopEnd=1\nRate=1\n",
        ));
        art.bind_anim_frame_count_for_test("MINDANIM", 1);
        rules.merge_art_data(&art);
        rules.art_registry = art;
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Owner");
        let unit_id = sim.allocate_stable_id();
        let mut unit = GameEntity::test_default(unit_id, "TARGET", "Owner", 2, 3);
        unit.owner = owner;
        unit.type_ref = sim.interner.intern("TARGET");
        sim.substrate.entities.insert(unit);
        sim.substrate
            .entities
            .get_mut(unit_id)
            .unwrap()
            .position
            .exact_z_leptons = Some(500);
        create_control_ring(&mut sim, &rules, unit_id);
        let unit_ring = sim.substrate.entities.get(unit_id).unwrap().mind_control_anim_id.unwrap();
        assert_eq!(sim.anim_absolute_coord(unit_ring).unwrap().z, 537);
        let unit_anim = sim.anim(unit_ring).unwrap();
        assert_eq!(unit_anim.owner_entity, Some(unit_id));
        assert_eq!(unit_anim.draw_flags, 0x600);
        assert_eq!(unit_anim.z_adjust, 0);

        let building_id = sim.allocate_stable_id();
        let mut building = GameEntity::test_default(building_id, "BTARGET", "Owner", 5, 6);
        building.owner = owner;
        building.type_ref = sim.interner.intern("BTARGET");
        building.category = EntityCategory::Structure;
        sim.substrate.entities.insert(building);
        create_control_ring(&mut sim, &rules, building_id);
        let building_ring = sim
            .substrate
            .entities
            .get(building_id)
            .unwrap()
            .mind_control_anim_id
            .unwrap();
        assert_eq!(sim.anim_absolute_coord(building_ring).unwrap().z, 3 * 104);
        assert_eq!(sim.anim(building_ring).unwrap().z_adjust, -1024);
    }

    #[test]
    fn team_override_still_draws_rng_then_routes_grinder_and_bioreactor() {
        for (decision, mission) in [(2, MissionType::Eaten), (3, MissionType::Enter)] {
            let (mut sim, rules, controller_id, victim_id, _) = fate_facility_sim(decision);
            let mut expected_rng = sim.scenario_rng.clone();
            let _ = expected_rng.next_range_i32_inclusive(1, 100);
            decide_unit_fate(
                &mut sim,
                &rules,
                controller_id,
                victim_id,
                7,
            );
            assert_eq!(sim.scenario_rng.logical_state(), expected_rng.logical_state());
            let victim = sim.substrate.entities.get(victim_id).unwrap();
            assert_eq!(victim.mission.current(), MissionId::from_known(mission));
            assert!(matches!(
                victim.navigation.nav_com,
                Some(NavTargetRef::Building { .. })
            ));
            assert_eq!(victim.ai_absorb_enter_pending, decision == 3);
            assert!(victim.navigation.pending_arrival_clear);
            if decision == 3 {
                let facility_id = match victim.navigation.nav_com {
                    Some(NavTargetRef::Building { id }) => id,
                    _ => unreachable!("asserted Building target"),
                };
                assert_eq!(victim.radio_contacts.slot(0), Some(facility_id));
                assert!(
                    sim.substrate
                        .entities
                        .get(facility_id)
                        .unwrap()
                        .radio_contacts
                        .contains(victim_id)
                );
            } else {
                assert!(victim.radio_contacts.is_empty());
            }
        }
    }

    #[test]
    fn capture_fate_hello_is_idempotent_and_receiver_saturation_refuses_without_sender_write() {
        let (mut sim, _rules, _controller_id, victim_id, facility_id) = fate_facility_sim(3);
        let blocker_id = sim.allocate_stable_id();
        sim.substrate.entities.insert(GameEntity::test_default(
            blocker_id, "TARGET", "Owner", 1, 1,
        ));
        sim.substrate
            .entities
            .get_mut(facility_id)
            .unwrap()
            .radio_contacts
            .insert(blocker_id);

        establish_capture_fate_contact(&mut sim, victim_id, facility_id);
        assert!(
            !sim.substrate
                .entities
                .get(victim_id)
                .unwrap()
                .radio_contacts
                .contains(facility_id)
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(facility_id)
                .unwrap()
                .radio_contacts
                .slot(0),
            Some(blocker_id)
        );

        sim.substrate
            .entities
            .get_mut(facility_id)
            .unwrap()
            .radio_contacts
            .remove(blocker_id);
        establish_capture_fate_contact(&mut sim, victim_id, facility_id);
        let first = sim.substrate.entities.get(victim_id).unwrap().radio_contacts.clone();
        establish_capture_fate_contact(&mut sim, victim_id, facility_id);
        assert_eq!(sim.substrate.entities.get(victim_id).unwrap().radio_contacts, first);
        assert_eq!(
            sim.substrate
                .entities
                .get(facility_id)
                .unwrap()
                .radio_contacts
                .len(),
            1
        );
    }

    #[test]
    fn capture_fate_grinder_approach_physically_crosses_to_foundation_center_over_multiple_ticks() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n0=CTRL\n1=TARGET\n\
             [InfantryTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n0=GRINDER\n\
             [CTRL]\nStrength=100\n\
             [TARGET]\nStrength=100\nSpeed=8\nROT=5\nMovementZone=Normal\nLocomotor={4A582741-9839-11D1-B709-00A024DDAFD1}\n\
             [GRINDER]\nStrength=500\nGrinding=yes\nFoundation=3x3\n\
             [Eaten]\nRate=.016\n",
        ))
        .expect("capture-fate movement rules");
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Owner");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, false, 10_000, 10));
        sim.session.house_order.push(owner);
        let controller_id = sim
            .spawn_object("CTRL", "Owner", 1, 3, 0, &rules, &Default::default())
            .unwrap();
        let victim_id = sim
            .spawn_object("TARGET", "Owner", 2, 3, 0, &rules, &Default::default())
            .unwrap();
        let facility_id = sim
            .spawn_object("GRINDER", "Owner", 9, 3, 0, &rules, &Default::default())
            .unwrap();
        let _team = install_controller_decision(&mut sim, controller_id, owner, 2);

        decide_unit_fate(&mut sim, &rules, controller_id, victim_id, 0);
        let initial = {
            let victim = sim.substrate.entities.get(victim_id).unwrap();
            assert_eq!(victim.mission.current(), MissionId::from_known(MissionType::Eaten));
            assert_eq!(
                victim.navigation.nav_com,
                Some(NavTargetRef::building(facility_id))
            );
            (victim.position.rx, victim.position.ry)
        };
        let footprint = {
            let facility = sim.substrate.entities.get(facility_id).unwrap();
            crate::sim::production::building_base_foundation_cells(
                facility.position.rx,
                facility.position.ry,
                &facility.foundation,
            )
        };
        assert_eq!(footprint.len(), 9, "fixture must exercise a blocked 3x3 foundation");
        let expected_coord = crate::sim::movement::resolve_entity_nav_target_drive_coord(
            NavTargetRef::building(facility_id),
            &sim.substrate.entities,
        )
        .expect("Building GetCoords projection");
        let expected_goal = (
            u16::try_from(expected_coord.x.div_euclid(256)).unwrap(),
            u16::try_from(expected_coord.y.div_euclid(256)).unwrap(),
        );
        assert!(footprint.contains(&expected_goal));
        let mut grid = crate::sim::pathfinding::PathGrid::new(20, 10);
        for &(rx, ry) in &footprint {
            grid.set_blocked(rx, ry, true);
        }

        let mut first_arrival_frame = None;
        let mut consumed_on_arrival = false;
        let mut distinct_positions = std::collections::BTreeSet::new();
        distinct_positions.insert(initial);
        for frame in 0..240u32 {
            sim.advance_tick(
                &[],
                Some(&rules),
                &Default::default(),
                Some(&grid),
                None,
                16,
            );
            let Some(victim) = sim.substrate.entities.get(victim_id) else {
                // Slice C's synchronous PerCell continuation is allowed to
                // consume on the first selected-Building foundation cell. The
                // approach contract is still proved by the multi-frame live
                // positions and center-aimed path observed before this frame.
                first_arrival_frame = Some(frame);
                consumed_on_arrival = true;
                break;
            };
            let position = (victim.position.rx, victim.position.ry);
            distinct_positions.insert(position);
            if frame == 0 {
                assert_ne!(
                    position, expected_goal,
                    "selector/first process frame must not teleport to the facility"
                );
                assert_eq!(
                    victim
                        .movement_target
                        .as_ref()
                        .and_then(|target| target.final_goal),
                    Some(expected_goal),
                    "the physical path must aim at Building GetCoords, not its north-west anchor"
                );
            }
            if footprint.contains(&position) {
                first_arrival_frame = Some(frame);
                break;
            }
            assert_eq!(
                victim.navigation.nav_com,
                Some(NavTargetRef::building(facility_id)),
                "pre-arrival frames must preserve the selected Building target"
            );
        }

        let arrival_frame = first_arrival_frame.expect("mover must physically enter the selected facility");
        assert!(arrival_frame > 0, "arrival must require more than one native frame");
        assert!(
            distinct_positions.len() > 2,
            "the approach must traverse intermediate cells rather than teleport"
        );
        if !consumed_on_arrival {
            let victim = sim.substrate.entities.get(victim_id).unwrap();
            assert!(
                footprint.contains(&(victim.position.rx, victim.position.ry)),
                "without the per-cell consumer, the mover stops on a selected-Building footprint cell"
            );
        }
    }

    #[test]
    fn team_override_actions_four_and_five_route_hunt_and_noop_after_rng() {
        for (decision, expected) in [
            (4, MissionId::from_known(MissionType::Hunt)),
            (5, MissionId::from_known(MissionType::Guard)),
        ] {
            let (mut sim, rules, owners, [controller_id, victim_id, _]) = capture_sim();
            set_raw_mission(
                sim.substrate.entities.get_mut(victim_id).unwrap(),
                MissionType::Guard as i32,
            );
            let _team = install_controller_decision(
                &mut sim,
                controller_id,
                owners[0],
                decision,
            );
            let mut expected_rng = sim.scenario_rng.clone();
            let _ = expected_rng.next_range_i32_inclusive(1, 100);
            decide_unit_fate(
                &mut sim,
                &rules,
                controller_id,
                victim_id,
                8,
            );
            assert_eq!(sim.scenario_rng.logical_state(), expected_rng.logical_state());
            assert_eq!(sim.substrate.entities.get(victim_id).unwrap().mission.current(), expected);
        }
    }

    #[test]
    fn team_override_action_one_head_links_victim_without_hunt_or_count_change() {
        let (mut sim, rules, owners, [controller_id, target_id, _]) = capture_sim();
        let team_id = install_controller_decision(&mut sim, controller_id, owners[0], 1);
        let counts_before = sim
            .team_script_vm
            .team(team_id)
            .unwrap()
            .member_type_counts()
            .to_vec();
        let mut expected_rng = sim.scenario_rng.clone();
        let _ = expected_rng.next_range_i32_inclusive(1, 100);

        assert!(capture_unit(&mut sim, &rules, controller_id, target_id, 41));

        let team = sim.team_script_vm.team(team_id).unwrap();
        assert_eq!(team.members(), &[target_id, controller_id]);
        assert_eq!(team.member_type_counts(), counts_before);
        assert_eq!(
            sim.substrate.entities.get(target_id).unwrap().mission.current(),
            MissionId::from_known(MissionType::Guard),
            "successful forced AddMember returns without the Hunt fallback"
        );
        assert_eq!(sim.scenario_rng.logical_state(), expected_rng.logical_state());
    }

    #[test]
    fn uncovered_authored_fate_vector_routes_default_hunt_after_one_draw() {
        let (mut sim, mut rules, _, [controller_id, target_id, _]) = capture_sim();
        rules.general.ai_capture_normal = vec![0];
        rules.general.ai_capture_wounded = vec![0];
        rules.general.ai_capture_low_power = vec![0];
        rules.general.ai_capture_low_money = vec![0];
        let mut expected_rng = sim.scenario_rng.clone();
        let _ = expected_rng.next_range_i32_inclusive(1, 100);

        decide_unit_fate(&mut sim, &rules, controller_id, target_id, 42);

        assert_eq!(
            sim.substrate.entities.get(target_id).unwrap().mission.current(),
            MissionId::from_known(MissionType::Hunt),
        );
        assert_eq!(sim.scenario_rng.logical_state(), expected_rng.logical_state());
    }

    #[test]
    fn absorber_radio_admission_preserves_every_active_retail_gate() {
        assert!(absorber_gate_result("", "", |_, _, _, _| {}, |_| {}));
        assert!(!absorber_gate_result(
            "",
            "",
            |sim, _, victim_id, _| {
                let outsider = sim.interner.intern("Outsider");
                sim.substrate.entities.get_mut(victim_id).unwrap().owner = outsider;
            },
            |_| {},
        ));
        for raw in [0x12, 0x13] {
            assert!(!absorber_gate_result(
                "",
                "",
                |sim, _, _, building_id| {
                    set_raw_mission(sim.substrate.entities.get_mut(building_id).unwrap(), raw);
                },
                |_| {},
            ));
        }
        assert!(!absorber_gate_result(
            "",
            "",
            |_, _, _, _| {},
            |object| object.naval = true,
        ));
        assert!(absorber_gate_result(
            "MovementZone=Amphibious\n",
            "Naval=yes\n",
            |_, _, _, _| {},
            |_| {},
        ));
        assert!(!absorber_gate_result(
            "BalloonHover=yes\n",
            "",
            |_, _, _, _| {},
            |_| {},
        ));
        assert!(!absorber_gate_result(
            "",
            "Powered=yes\nPower=-1\n",
            |sim, owner, _, _| {
                sim.power_states.insert(
                    owner,
                    PowerState {
                        is_low_power: true,
                        ..PowerState::default()
                    },
                );
            },
            |_| {},
        ));
        assert!(!absorber_gate_result(
            "",
            "",
            |_, _, _, _| {},
            |object| object.infantry_absorb = false,
        ));
        assert!(!absorber_gate_result(
            "",
            "",
            |sim, _, victim_id, _| {
                sim.substrate.entities.get_mut(victim_id).unwrap().capture_manager =
                    Some(CaptureManagerState {
                        max_control: 0,
                        infinite_mind_control: false,
                        controlled_nodes: Vec::new(),
                    });
            },
            |_| {},
        ));
        assert!(!absorber_gate_result(
            "",
            "",
            |sim, _, _, building_id| {
                sim.substrate
                    .entities
                    .get_mut(building_id)
                    .unwrap()
                    .passenger_role
                    .cargo_mut()
                    .unwrap()
                    .capacity = 0;
            },
            |_| {},
        ));
        assert!(!absorber_gate_result(
            "Size=2\n",
            "SizeLimit=1\n",
            |_, _, _, _| {},
            |_| {},
        ));
    }

    #[test]
    fn facility_selection_uses_house_vector_tail_and_native_3d_ties() {
        let (mut sim, rules, _, victim_id, older_id) = absorber_gate_sim("", "");
        let newer_id = sim
            .spawn_object("BIO", "Owner", 4, 5, 0, &rules, &Default::default())
            .unwrap();
        let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
        victim.position.sub_x = crate::util::fixed_math::SimFixed::from_num(128);
        victim.position.sub_y = crate::util::fixed_math::SimFixed::from_num(128);

        assert_eq!(
            sim.houses[&victim.owner].grinder_building_order,
            vec![older_id, newer_id]
        );
        assert_eq!(
            sim.houses[&victim.owner].absorber_building_order,
            vec![older_id, newer_id]
        );
        // Native does not consume the global Logic/Techno registration order
        // here. Deliberately make its reverse order choose `older_id`; the
        // House's separate facility-vector tail must still win the exact tie.
        sim.set_logic_order_for_test(vec![victim_id, newer_id, older_id]);

        assert_eq!(
            nearest_owned_building(
                &sim,
                &rules,
                victim_id,
                HouseCaptureFacilityVector::Grinder,
                |_, object, _, _| object.grinding,
            ),
            Some(newer_id),
            "strict-less ties preserve the first House-vector tail candidate"
        );
        assert_eq!(
            nearest_owned_building(
                &sim,
                &rules,
                victim_id,
                HouseCaptureFacilityVector::Absorber,
                |sim, object, victim, building| {
                    building_can_enter_absorber(sim, &rules, object, victim, building)
                },
            ),
            Some(newer_id),
        );

        sim.substrate.entities.get_mut(newer_id).unwrap().position.exact_z_leptons = Some(2_000);
        assert_eq!(
            nearest_owned_building(
                &sim,
                &rules,
                victim_id,
                HouseCaptureFacilityVector::Grinder,
                |_, object, _, _| object.grinding,
            ),
            Some(older_id),
            "native Distance3D includes Z before whole-lepton strict comparison"
        );
    }

    #[test]
    fn capture_facility_vectors_follow_limbo_reveal_and_change_owner() {
        let (mut sim, rules, owner, _victim_id, older_id) = absorber_gate_sim("", "");
        let newer_id = sim
            .spawn_object("BIO", "Owner", 4, 5, 0, &rules, &Default::default())
            .unwrap();
        let other_owner = sim.interner.intern("OtherOwner");
        sim.houses.insert(
            other_owner,
            crate::sim::house_state::HouseState::new(other_owner, 1, None, false, 0, 10),
        );

        assert_eq!(sim.houses[&owner].grinder_building_order, vec![older_id, newer_id]);
        assert_eq!(sim.techno_limbo(older_id), crate::sim::world::ConcealOutcome::Concealed);
        assert_eq!(sim.houses[&owner].grinder_building_order, vec![newer_id]);
        assert_eq!(sim.houses[&owner].absorber_building_order, vec![newer_id]);

        assert!(matches!(
            sim.reveal(older_id),
            crate::sim::world::RevealOutcome::Revealed { .. }
        ));
        assert_eq!(sim.houses[&owner].grinder_building_order, vec![newer_id, older_id]);
        assert_eq!(sim.houses[&owner].absorber_building_order, vec![newer_id, older_id]);

        sim.change_owner_with_rules(older_id, other_owner, &rules);
        assert_eq!(sim.houses[&owner].grinder_building_order, vec![newer_id]);
        assert_eq!(sim.houses[&owner].absorber_building_order, vec![newer_id]);
        assert_eq!(sim.houses[&other_owner].grinder_building_order, vec![older_id]);
        assert_eq!(sim.houses[&other_owner].absorber_building_order, vec![older_id]);

        sim.change_owner_with_rules(older_id, owner, &rules);
        assert!(sim.houses[&other_owner].grinder_building_order.is_empty());
        assert!(sim.houses[&other_owner].absorber_building_order.is_empty());
        assert_eq!(sim.houses[&owner].grinder_building_order, vec![newer_id, older_id]);
        assert_eq!(sim.houses[&owner].absorber_building_order, vec![newer_id, older_id]);
    }

    #[test]
    fn capture_override_release_and_arbitrary_owner_change_preserve_reciprocals() {
        let (mut sim, rules, owners, [controller_id, first_id, second_id]) = capture_sim();
        assert!(capture_unit(&mut sim, &rules, controller_id, first_id, 0));
        assert_eq!(
            sim.substrate.entities.get(first_id).unwrap().owner,
            owners[0]
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(first_id)
                .unwrap()
                .mind_control_controller_id,
            Some(controller_id)
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(controller_id)
                .unwrap()
                .capture_manager
                .as_ref()
                .unwrap()
                .controlled_nodes,
            vec![CaptureNodeState {
                victim_id: first_id,
                original_owner: owners[1],
                capture_frame: 0,
                link_visible_frames: 20,
            }]
        );
        assert!(
            matches!(
                sim.substrate
                    .entities
                    .get(controller_id)
                    .unwrap()
                    .attack_target
                    .as_ref()
                    .map(|target| target.target),
                Some(TargetKind::Entity(id)) if id == first_id
            ),
            "CaptureUnit's transient manager target suppresses detach cleanup"
        );

        sim.change_owner_with_rules(first_id, owners[2], &rules);
        assert_eq!(
            sim.substrate
                .entities
                .get(first_id)
                .unwrap()
                .mind_control_controller_id,
            Some(controller_id),
            "arbitrary ChangeOwner preserves reversible link"
        );
        assert!(free_unit(&mut sim, &rules, controller_id, first_id));
        assert_eq!(
            sim.substrate.entities.get(first_id).unwrap().owner,
            owners[1],
            "FreeUnit restores the MCNode owner sampled at capture"
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(first_id)
                .unwrap()
                .mind_control_controller_id,
            None
        );

        assert!(capture_unit(&mut sim, &rules, controller_id, first_id, 1));
        assert!(capture_unit(&mut sim, &rules, controller_id, second_id, 2));
        assert_eq!(sim.substrate.entities.get(first_id).unwrap().owner, owners[1]);
        assert_eq!(
            sim.substrate
                .entities
                .get(first_id)
                .unwrap()
                .mind_control_controller_id,
            None,
            "max_control=1 releases the old tail before linking the replacement"
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(second_id)
                .unwrap()
                .mind_control_controller_id,
            Some(controller_id)
        );
    }

    #[test]
    fn permanent_conversion_removes_reversible_node_before_setting_byte() {
        let (mut sim, rules, owners, [controller_id, target_id, _]) = capture_sim();
        assert!(capture_unit(&mut sim, &rules, controller_id, target_id, 0));
        assert!(make_permanent(
            &mut sim,
            &rules,
            target_id,
            owners[2]
        ));
        let target = sim.substrate.entities.get(target_id).unwrap();
        assert_eq!(target.owner, owners[2]);
        assert_eq!(target.mind_control_controller_id, None);
        assert!(target.permanently_mind_controlled);
        assert!(
            sim.substrate
                .entities
                .get(controller_id)
                .unwrap()
                .capture_manager
                .as_ref()
                .unwrap()
                .controlled_nodes
                .is_empty(),
            "Dominator conversion creates no MCNode"
        );
    }

    #[test]
    fn free_unit_returns_true_for_nonnull_untracked_victim_and_removes_all_duplicates() {
        let (mut sim, rules, owners, [controller_id, target_id, second_id]) = capture_sim();
        assert!(
            free_unit(&mut sim, &rules, controller_id, target_id),
            "native returns true for every nonnull victim even without a matching MCNode"
        );
        assert!(!free_unit(&mut sim, &rules, controller_id, u64::MAX));

        let controller = sim.substrate.entities.get_mut(controller_id).unwrap();
        let manager = controller.capture_manager.as_mut().unwrap();
        manager.controlled_nodes = vec![
            CaptureNodeState {
                victim_id: target_id,
                original_owner: owners[2],
                capture_frame: 7,
                link_visible_frames: 20,
            },
            CaptureNodeState {
                victim_id: second_id,
                original_owner: owners[1],
                capture_frame: 8,
                link_visible_frames: 20,
            },
            CaptureNodeState {
                victim_id: target_id,
                original_owner: owners[1],
                capture_frame: 9,
                link_visible_frames: 20,
            },
        ];
        sim.substrate
            .entities
            .get_mut(target_id)
            .unwrap()
            .mind_control_controller_id = Some(controller_id);
        assert!(free_unit(&mut sim, &rules, controller_id, target_id));
        assert_eq!(
            sim.substrate.entities.get(target_id).unwrap().owner,
            owners[2],
            "reverse scan processes the earlier duplicate last"
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(controller_id)
                .unwrap()
                .capture_manager
                .as_ref()
                .unwrap()
                .controlled_nodes,
            vec![CaptureNodeState {
                victim_id: second_id,
                original_owner: owners[1],
                capture_frame: 8,
                link_visible_frames: 20,
            }]
        );
    }

    #[test]
    fn free_unit_prefers_per_type_cleared_sound_then_global_fallback() {
        let (mut sim, rules, _, [controller_id, target_id, other_id]) = capture_sim();
        assert_eq!(
            rules.general.mind_cleared_sound.as_deref(),
            Some("GlobalClear"),
            "the fixture must bind the native AudioVisual fallback"
        );
        assert!(capture_unit(&mut sim, &rules, controller_id, target_id, 0));
        sim.sound_events.clear();
        assert!(free_unit(&mut sim, &rules, controller_id, target_id));
        assert!(matches!(
            sim.sound_events.as_slice(),
            [SimSoundEvent::MindControlSound { sound_id, .. }] if sound_id == "TypeClear"
        ), "unexpected per-type FreeUnit event vector: {:?}", sim.sound_events);

        assert!(capture_unit(&mut sim, &rules, controller_id, other_id, 1));
        sim.sound_events.clear();
        assert!(free_unit(&mut sim, &rules, controller_id, other_id));
        assert!(matches!(
            sim.sound_events.as_slice(),
            [SimSoundEvent::MindControlSound { sound_id, .. }] if sound_id == "GlobalClear"
        ), "unexpected fallback FreeUnit event vector: {:?}", sim.sound_events);
    }

    #[test]
    fn controller_direct_uninit_releases_victim_before_pointer_expiry() {
        let (mut sim, rules, owners, [controller_id, target_id, _]) = capture_sim();
        assert!(capture_unit(&mut sim, &rules, controller_id, target_id, 0));
        sim.uninit_with_rules(controller_id, &rules);
        let victim = sim.substrate.entities.get(target_id).unwrap();
        assert_eq!(victim.owner, owners[1]);
        assert_eq!(victim.mind_control_controller_id, None);
        assert!(
            sim.substrate.entities.get(controller_id).is_some(),
            "ObjectClass UnInit leaves the controller resolvable through deferred deletion"
        );
    }
}
