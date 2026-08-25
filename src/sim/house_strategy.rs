//! HouseClass strategy-state substrate.
//!
//! This module intentionally does not schedule Strategy or execute its object
//! callbacks yet. Native order is AI-hate -> synchronous AI superweapon -> this
//! emergency block -> no-factory priority -> build-need/Manage -> reschedule.
//! Activating only the middle of that chain would perturb object and Scenario
//! RNG order. The pure state machine and candidate-bias decision can land
//! independently without inventing such a scheduler.

use crate::sim::house_state::{HouseState, HouseStrategyEmergencyState};
use crate::sim::intern::InternedId;

const LOW_WALLET_THRESHOLD: i32 = 25;
const ATTACK_SUPPRESSION_FRAMES: i32 = 900;

/// Ordered callbacks requested by the direct state-four block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmergencyAction {
    FireSale,
    AllToHunt,
}

/// Advance only `House+0x250`'s native state block.
///
/// `available_wallet` is deliberately a callback: state zero can transition to
/// one and immediately query a second time in the same invocation. Collapsing
/// this to a single sampled value would erase observable call count/order.
pub(crate) fn advance_emergency_state(
    state: &mut HouseStrategyEmergencyState,
    current_frame: i32,
    mut available_wallet: impl FnMut() -> i32,
) -> Vec<EmergencyAction> {
    if state.mode == 4 {
        return vec![EmergencyAction::FireSale, EmergencyAction::AllToHunt];
    }

    if state.mode == 0 && available_wallet() < LOW_WALLET_THRESHOLD {
        state.mode = 1;
    }
    if state.mode == 1 && available_wallet() >= LOW_WALLET_THRESHOLD {
        state.mode = 0;
    }

    let deadline = state
        .last_building_attack_frame
        .wrapping_add(ATTACK_SUPPRESSION_FRAMES);
    if state.mode == 3 {
        if deadline < current_frame {
            state.mode = 0;
        }
    } else if current_frame < deadline {
        state.mode = 3;
    }

    Vec::new()
}

/// Exact `House+0x249` decision consumed by native
/// `TechnoClass__Evaluate_Candidate @ 0x006F8765`.
///
/// VERA's current acquisition ranking is explicitly not that native evaluator,
/// so this helper remains disconnected until the expanding-ring score path is
/// implemented. Translating native score `1` into the current nearest-first
/// tuple would be an approximation, not parity.
pub(crate) fn all_to_hunt_score_override(
    attacker_house: &HouseState,
    candidate_owner: InternedId,
) -> Option<i32> {
    if attacker_house.strategy_emergency.all_to_hunt_bias
        && attacker_house
            .enemy_house
            .is_some_and(|enemy| candidate_owner != enemy)
    {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::house_state::HouseState;

    fn state(mode: i32, last_attack: i32) -> HouseStrategyEmergencyState {
        HouseStrategyEmergencyState {
            mode,
            all_to_hunt_bias: false,
            last_building_attack_frame: last_attack,
        }
    }

    #[test]
    fn state_zero_below_25_requeries_immediately() {
        let mut emergency = state(0, -900);
        let mut calls = 0;
        let actions = advance_emergency_state(&mut emergency, 1, || {
            calls += 1;
            if calls == 1 { 24 } else { 25 }
        });
        assert_eq!(calls, 2);
        assert_eq!(emergency.mode(), 0);
        assert!(actions.is_empty());
    }

    #[test]
    fn wallet_threshold_hysteresis_is_signed_24_25() {
        let mut low = state(1, -900);
        advance_emergency_state(&mut low, 1, || 24);
        assert_eq!(low.mode(), 1);

        let mut recovered = state(1, -900);
        advance_emergency_state(&mut recovered, 1, || 25);
        assert_eq!(recovered.mode(), 0);
    }

    #[test]
    fn state_three_deadline_equality_is_asymmetric() {
        let mut before = state(0, 100);
        advance_emergency_state(&mut before, 999, || 25);
        assert_eq!(before.mode(), 3);

        let mut armed_at_equal = state(3, 100);
        advance_emergency_state(&mut armed_at_equal, 1000, || 25);
        assert_eq!(armed_at_equal.mode(), 3);

        let mut unarmed_at_equal = state(0, 100);
        advance_emergency_state(&mut unarmed_at_equal, 1000, || 25);
        assert_eq!(unarmed_at_equal.mode(), 0);

        let mut after = state(3, 100);
        advance_emergency_state(&mut after, 1001, || 25);
        assert_eq!(after.mode(), 0);
    }

    #[test]
    fn attack_deadline_uses_wrapping_signed_addition() {
        let last_attack = i32::MAX - 100;
        let wrapped_deadline = last_attack.wrapping_add(ATTACK_SUPPRESSION_FRAMES);
        assert!(wrapped_deadline < 0);

        let mut emergency = state(3, last_attack);
        advance_emergency_state(&mut emergency, wrapped_deadline, || 25);
        assert_eq!(emergency.mode(), 3, "equality retains an existing state 3");
        advance_emergency_state(&mut emergency, wrapped_deadline.wrapping_add(1), || 25);
        assert_eq!(emergency.mode(), 0);
    }

    #[test]
    fn state_four_emits_ordered_actions_without_clearing() {
        let mut emergency = state(4, 0);
        let mut wallet_called = false;
        let actions = advance_emergency_state(&mut emergency, 10_000, || {
            wallet_called = true;
            0
        });
        assert_eq!(
            actions,
            vec![EmergencyAction::FireSale, EmergencyAction::AllToHunt]
        );
        assert_eq!(emergency.mode(), 4);
        assert!(!wallet_called);
    }

    #[test]
    fn all_to_hunt_override_is_persistent_and_follows_designated_enemy() {
        let owner = InternedId::from_index(1);
        let first_enemy = InternedId::from_index(2);
        let second_enemy = InternedId::from_index(3);
        let bystander = InternedId::from_index(4);
        let mut house = HouseState::new(owner, 0, None, false, 0, 10);

        house.enemy_house = Some(first_enemy);
        assert_eq!(all_to_hunt_score_override(&house, bystander), None);

        house.strategy_emergency.set_all_to_hunt_bias();
        assert_eq!(all_to_hunt_score_override(&house, first_enemy), None);
        assert_eq!(all_to_hunt_score_override(&house, bystander), Some(1));

        house.enemy_house = Some(second_enemy);
        assert_eq!(all_to_hunt_score_override(&house, first_enemy), Some(1));
        assert_eq!(all_to_hunt_score_override(&house, second_enemy), None);

        house.enemy_house = None;
        assert_eq!(all_to_hunt_score_override(&house, bystander), None);
        assert!(house.strategy_emergency.all_to_hunt_bias());
    }

    #[test]
    fn native_entry_writers_change_only_their_owned_fields() {
        let mut emergency = state(-7, 123);
        emergency.set_state_four();
        assert_eq!(emergency.mode(), 4);
        assert_eq!(emergency.last_building_attack_frame(), 123);
        assert!(!emergency.all_to_hunt_bias());

        emergency.note_building_attack(-55);
        assert_eq!(emergency.last_building_attack_frame(), -55);
        assert_eq!(emergency.mode(), 4);
    }

    #[test]
    fn non_bincode_missing_house_field_uses_native_constructor_defaults() {
        let owner = InternedId::from_index(1);
        let house = HouseState::new(owner, 0, None, false, 0, 10);
        let mut value = serde_json::to_value(house).expect("HouseState serializes to JSON");
        value
            .as_object_mut()
            .expect("HouseState JSON is an object")
            .remove("strategy_emergency");

        let restored: HouseState =
            serde_json::from_value(value).expect("serde default fills the absent field");
        assert_eq!(
            restored.strategy_emergency,
            HouseStrategyEmergencyState::default()
        );
        assert_eq!(restored.strategy_emergency.mode(), 0);
        assert!(!restored.strategy_emergency.all_to_hunt_bias());
        assert_eq!(restored.strategy_emergency.last_building_attack_frame(), 0);
    }
}
