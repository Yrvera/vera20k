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
    // The capture-acquisition producer is not wired yet, but this is the
    // controller-owned insertion seam it must use.
    #[allow(dead_code)]
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
    use crate::rules::ini_parser::IniFile;

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
}
