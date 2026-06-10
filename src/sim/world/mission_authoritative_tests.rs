//! Slice 8 — `MissionCom` is folded into `state_hash`. These pin that each
//! component field is hash-relevant (the inverse of the Slice-2 shadow tests).
//! No `order_intent` selector tripwire: `order_intent` is load-bearing substate
//! and is retained (the V5 "pure selector" map row was corrected to KEEP).

use super::Simulation;
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::{MissionTimer, MissionType};

fn two_sims() -> (Simulation, Simulation) {
    let mut a = Simulation::new();
    let mut b = Simulation::new();
    a.substrate
        .entities
        .insert(GameEntity::test_default(1, "MTNK", "Americans", 10, 10));
    b.substrate
        .entities
        .insert(GameEntity::test_default(1, "MTNK", "Americans", 10, 10));
    assert_eq!(a.state_hash(), b.state_hash(), "baseline sims must hash equal");
    (a, b)
}

#[test]
fn mission_current_changes_state_hash() {
    let (a, mut b) = two_sims();
    b.substrate.entities.get_mut(1).unwrap().mission.current = MissionType::Attack;
    assert_ne!(
        a.state_hash(),
        b.state_hash(),
        "mission.current must contribute to the state hash"
    );
}

#[test]
fn mission_timer_and_substate_change_state_hash() {
    let (a, mut b) = two_sims();
    // substate
    b.substrate.entities.get_mut(1).unwrap().mission.substate = 7;
    assert_ne!(a.state_hash(), b.state_hash(), "mission.substate must affect hash");
    // reset substate -> back to equal -> then perturb the timer
    b.substrate.entities.get_mut(1).unwrap().mission.substate = 0;
    assert_eq!(a.state_hash(), b.state_hash(), "substate reset restores equality");
    b.substrate.entities.get_mut(1).unwrap().mission.timer = MissionTimer::armed(5, 30);
    assert_ne!(a.state_hash(), b.state_hash(), "mission.timer must affect hash");
}

#[test]
fn mission_queued_and_suspended_change_state_hash() {
    let (a, mut b) = two_sims();
    b.substrate.entities.get_mut(1).unwrap().mission.queued = Some(MissionType::Guard);
    assert_ne!(a.state_hash(), b.state_hash(), "mission.queued must affect hash");
    b.substrate.entities.get_mut(1).unwrap().mission.queued = None;
    assert_eq!(a.state_hash(), b.state_hash(), "queued reset restores equality");
    b.substrate.entities.get_mut(1).unwrap().mission.suspended = Some(MissionType::Move);
    assert_ne!(a.state_hash(), b.state_hash(), "mission.suspended must affect hash");
}
