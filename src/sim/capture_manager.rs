//! Narrow `CaptureManagerClass` authority shared by mind-control producers and
//! receiver-synchronous retaliation.
//!
//! Native constructs this manager only when the owner's primary weapon uses a
//! `MindControl=yes` warhead. The manager snapshots the weapon's signed
//! `Damage=` as its finite link limit and `InfiniteMindControl=` as its capacity
//! bypass. Victim links are controller-owned and retain insertion order; they
//! are not reconstructed by scanning the independent permanent-control byte.

use serde::{Deserialize, Serialize};

use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::mission::MissionType;
use crate::sim::world::Simulation;

/// One persistent native MCNode. The victim pointer and the House owner saved
/// at capture time are independent: House-wide destruction resolves effective
/// ownership from this saved owner, not from the victim's current House.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaptureNodeState {
    pub victim_id: u64,
    pub original_owner: InternedId,
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
    ) {
        if !self
            .controlled_nodes
            .iter()
            .any(|node| node.victim_id == victim_id)
        {
            self.controlled_nodes.push(CaptureNodeState {
                victim_id,
                original_owner,
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
        manager.link_controlled_entity(target_id, original_owner);
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
    true
}

/// Release one reversible MCNode in native order: restore owner while the
/// reciprocal node still exists, then clear the victim backlink and compact
/// the controller's node vector. Presentation and AI-fate continuations are
/// owned by their dedicated producers and must bracket this state transaction.
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
        sim.change_owner_with_rules(victim_id, original_owner, rules);
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

    fn capture_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n0=CTRL\n1=TARGET\n2=IMMUNE\n3=OTHER\n\
             [InfantryTypes]\n0=INF\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [CTRL]\nStrength=100\nPrimary=MIND\n\
             [TARGET]\nStrength=100\n\
             [IMMUNE]\nStrength=100\nImmuneToPsionics=yes\n\
             [OTHER]\nStrength=100\n\
             [INF]\nStrength=100\n\
             [MIND]\nDamage=1\nWarhead=CONTROLLER\n\
             [CONTROLLER]\nMindControl=yes\n",
        ))
        .expect("mind-control fixture rules")
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
        manager.link_controlled_entity(10, original_owner);
        manager.link_controlled_entity(11, original_owner);
        assert!(!manager.blocks_retaliation());
        manager.link_controlled_entity(12, original_owner);
        assert!(manager.blocks_retaliation());
        manager.pointer_expired(11);
        assert!(!manager.blocks_retaliation());

        manager.infinite_mind_control = true;
        manager.max_control = 0;
        assert!(!manager.blocks_retaliation());
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
                original_owner: owners[1]
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
            },
            CaptureNodeState {
                victim_id: second_id,
                original_owner: owners[1],
            },
            CaptureNodeState {
                victim_id: target_id,
                original_owner: owners[1],
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
            }]
        );
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
