use std::collections::BTreeMap;

use crate::sim::snapshot::GameSnapshot;
use crate::sim::team_script_vm::{TeamScriptAction, TeamScriptDefinition};

use super::{MasterFrameTestRung, Simulation};

fn action(action_id: u8, argument: i32) -> TeamScriptAction {
    TeamScriptAction {
        action_id,
        argument,
    }
}

fn install_wait_then_advance_fixture(sim: &mut Simulation, members: Vec<u64>) -> u64 {
    let owner = sim.interner.intern("Americans");
    let opening = sim.interner.intern("TEAM_OPENING");
    sim.team_script_vm.register_script(TeamScriptDefinition {
        id: opening,
        actions: vec![action(24, 0)],
    });
    let team = sim
        .team_script_vm
        .create_team(owner, opening, members, Some(99));
    assert!(sim.team_script_vm.set_delay(team, 2));
    team
}

#[test]
fn team_script_vm_advances_in_the_master_frame_and_survives_save_load() {
    let mut original = Simulation::with_seed(0xA11CE);
    let team_id = install_wait_then_advance_fixture(&mut original, vec![19, 7, 11]);
    let heights = BTreeMap::new();

    original.advance_tick(&[], None, &heights, None, None, 67);
    assert!(
        original
            .take_master_frame_test_trace()
            .contains(&MasterFrameTestRung::TeamScript)
    );
    let team = original.team_script_vm.team(team_id).expect("team");
    assert_eq!(team.cursor(), 0);
    assert_eq!(team.wait_frames_remaining(), 1);
    assert_eq!(team.members(), &[19, 7, 11]);
    assert_eq!(team.target(), Some(99));

    // Native in-scenario load resets Scenario RNG; isolate team-script
    // persistence by comparing against that same post-load baseline.
    original.scenario_rng = crate::sim::rng::SimRng::new(0);
    let snapshot = GameSnapshot::save(&original, 0, 0, "team_vm_test", 0);
    let mut restored = GameSnapshot::load(&snapshot).expect("snapshot").sim;
    assert_eq!(original.state_hash(), restored.state_hash());

    for _ in 0..6 {
        let expected = original.advance_tick(&[], None, &heights, None, None, 67);
        let actual = restored.advance_tick(&[], None, &heights, None, None, 67);
        assert_eq!(expected.state_hash, actual.state_hash);
    }

    let team = restored.team_script_vm.team(team_id).expect("team");
    assert!(team.completed());
    assert_eq!(team.members(), &[19, 7, 11]);
    assert_eq!(team.target(), Some(99));
}

#[test]
fn team_member_order_is_hashed() {
    let mut forward = Simulation::with_seed(7);
    let mut reverse = Simulation::with_seed(7);
    install_wait_then_advance_fixture(&mut forward, vec![19, 7, 11]);
    install_wait_then_advance_fixture(&mut reverse, vec![11, 7, 19]);

    assert_ne!(forward.state_hash(), reverse.state_hash());
}
