use std::collections::BTreeMap;

use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::rules::team_ai_ini::{TeamAiDefinitionSource, TeamAiIniRegistry};
use crate::sim::snapshot::GameSnapshot;
use crate::sim::team_script_vm::{
    TeamAiInstallDiagnostic, TeamScriptAction, TeamScriptDefinition,
};

use super::{MasterFrameTestRung, Simulation};

fn action(action_id: i32, argument: i32) -> TeamScriptAction {
    TeamScriptAction {
        action_id,
        argument,
    }
}

fn zero_ai_trigger_comparison() -> String {
    "00".repeat(32)
}

fn install_wait_then_advance_fixture(sim: &mut Simulation, members: Vec<u64>) -> u64 {
    let owner = sim.interner.intern("Americans");
    let opening = sim.interner.intern("TEAM_OPENING");
    sim.team_script_vm.register_script(TeamScriptDefinition {
        id: opening,
        source: TeamAiDefinitionSource::FixedAimd,
        actions: vec![action(24, 0)],
    });
    let team = sim.team_script_vm.create_team(
        owner,
        opening,
        members,
        Some(99),
        sim.session.binary_frame as i32,
    );
    assert!(sim.team_script_vm.set_delay(team, 2));
    team
}

#[test]
fn team_script_vm_advances_in_the_master_frame_and_survives_save_load() {
    let mut original = Simulation::with_seed(0xA11CE);
    let team_id = install_wait_then_advance_fixture(&mut original, vec![19, 7, 11]);
    let heights = BTreeMap::new();

    original.advance_tick(&[], None, &heights, None, None, 67);
    let trace = original.take_master_frame_test_trace();
    assert_eq!(
        &trace[..4],
        &[
            MasterFrameTestRung::SessionCommands,
            MasterFrameTestRung::Triggers,
            MasterFrameTestRung::TeamScript,
            MasterFrameTestRung::LogicVector,
        ],
        "native TeamClass AI must finish before any live LogicClass object visit"
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

#[test]
fn production_install_boundary_resolves_aimd_without_creating_a_team() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n[E1]\nStrength=100\n",
    ))
    .expect("minimal rules");
    let comparison = zero_ai_trigger_comparison();
    let aimd = IniFile::from_str(&format!(
        "[TeamTypes]\n0=TT\n[TT]\nScript=S\nTaskForce=F\n\
         [ScriptTypes]\n0=S\n[S]\n0=2,0\n\
         [TaskForces]\n0=F\n[F]\n0=1,E1\n\
         [AITriggerTypes]\nA=Trigger,TT,<all>,2,4,<none>,{comparison},40,10,40,1,0,1,0,<none>,1,1,1\n"
    ));
    let registry = TeamAiIniRegistry::from_sources(&aimd, &IniFile::from_str(""), true);
    let mut sim = Simulation::new();
    sim.intern_rule_type_ids(&rules);
    sim.resolve_type_handles(&rules);

    let diagnostics = sim.install_team_ai_registry(&registry, &rules);

    assert!(diagnostics.is_empty());
    assert_eq!(sim.team_script_vm.registry_counts(), (1, 1, 1, 1));
    assert!(
        sim.team_script_vm.team(1).is_none(),
        "definition installation must not allocate a live TeamClass"
    );
}

#[test]
fn production_install_refuses_fixed_resolution_loss_but_keeps_scenario_omissions() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n[E1]\nStrength=100\n",
    ))
    .expect("minimal rules");
    let comparison = zero_ai_trigger_comparison();
    let fixed_with_unknown = IniFile::from_str(&format!(
        "[TeamTypes]\n0=TT\n[TT]\nScript=S\nTaskForce=F\n\
         [ScriptTypes]\n0=S\n[S]\n0=2,0\n\
         [TaskForces]\n0=F\n[F]\n0=1,GHOST\n\
         [AITriggerTypes]\nA=Trigger,TT,<all>,2,4,<none>,{comparison},40,10,40,1,0,1,0,<none>,1,1,1\n"
    ));
    let fixed_registry =
        TeamAiIniRegistry::from_sources(&fixed_with_unknown, &IniFile::from_str(""), true);
    assert!(fixed_registry.fixed_source_is_complete());

    let mut fixed_sim = Simulation::new();
    fixed_sim.intern_rule_type_ids(&rules);
    fixed_sim.resolve_type_handles(&rules);
    let fixed_diagnostics = fixed_sim.install_team_ai_registry(&fixed_registry, &rules);

    assert_eq!(
        fixed_diagnostics,
        vec![TeamAiInstallDiagnostic::UnknownTaskForceMember {
            task_force_id: "F".to_string(),
            member_type: "GHOST".to_string(),
            source: TeamAiDefinitionSource::FixedAimd,
        }]
    );
    assert!(fixed_diagnostics[0].is_fixed_source_refusal());
    assert_eq!(
        fixed_sim.team_script_vm.registry_counts(),
        (0, 0, 0, 0),
        "a fixed-origin resolution refusal must not install a partial registry"
    );

    let clean_fixed = IniFile::from_str(&format!(
        "[TeamTypes]\n0=TT\n[TT]\nScript=S\nTaskForce=F\n\
         [ScriptTypes]\n0=S\n[S]\n0=2,0\n\
         [TaskForces]\n0=F\n[F]\n0=1,E1\n\
         [AITriggerTypes]\nA=Trigger,TT,<all>,2,4,<none>,{comparison},40,10,40,1,0,1,0,<none>,1,1,1\n"
    ));
    let scenario = IniFile::from_str(
        "[TaskForces]\n0=MAP_F\n[MAP_F]\n0=1,GHOST\n",
    );
    let scenario_registry = TeamAiIniRegistry::from_sources(&clean_fixed, &scenario, true);
    assert!(scenario_registry.fixed_source_is_complete());

    let mut scenario_sim = Simulation::new();
    scenario_sim.intern_rule_type_ids(&rules);
    scenario_sim.resolve_type_handles(&rules);
    let scenario_diagnostics =
        scenario_sim.install_team_ai_registry(&scenario_registry, &rules);

    assert_eq!(
        scenario_diagnostics,
        vec![TeamAiInstallDiagnostic::UnknownTaskForceMember {
            task_force_id: "MAP_F".to_string(),
            member_type: "GHOST".to_string(),
            source: TeamAiDefinitionSource::Scenario,
        }]
    );
    assert!(!scenario_diagnostics[0].is_fixed_source_refusal());
    assert_eq!(
        scenario_sim.team_script_vm.registry_counts(),
        (2, 1, 1, 1),
        "scenario-origin omissions remain diagnosed, nonfatal overlays"
    );
}

#[test]
fn production_install_refuses_unknown_fixed_ai_trigger_object() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n[E1]\nStrength=100\n",
    ))
    .expect("minimal rules");
    let comparison = zero_ai_trigger_comparison();
    let fixed = IniFile::from_str(&format!(
        "[TeamTypes]\n0=TT\n[TT]\nScript=S\nTaskForce=F\n\
         [ScriptTypes]\n0=S\n[S]\n0=2,0\n\
         [TaskForces]\n0=F\n[F]\n0=1,E1\n\
         [AITriggerTypes]\nA=Trigger,TT,<all>,2,4,GHOST,{comparison},40,10,40,1,0,1,0,<none>,1,1,1\n"
    ));
    let registry = TeamAiIniRegistry::from_sources(&fixed, &IniFile::from_str(""), true);
    assert!(registry.fixed_source_is_complete());
    let mut sim = Simulation::new();
    sim.intern_rule_type_ids(&rules);
    sim.resolve_type_handles(&rules);

    let diagnostics = sim.install_team_ai_registry(&registry, &rules);

    assert_eq!(
        diagnostics,
        vec![TeamAiInstallDiagnostic::UnknownAiTriggerObject {
            trigger_id: "A".to_string(),
            object_type: "GHOST".to_string(),
            source: TeamAiDefinitionSource::FixedAimd,
        }]
    );
    assert!(diagnostics[0].is_fixed_source_refusal());
    assert_eq!(
        sim.team_script_vm.registry_counts(),
        (0, 0, 0, 0),
        "unknown fixed AITrigger token-6 references must refuse the whole registry install"
    );
}

#[test]
fn production_install_refuses_fixed_resolution_loss_masked_by_same_identity_map_overlays() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n0=E1\n[E1]\nStrength=100\n",
    ))
    .expect("minimal rules");
    let comparison = zero_ai_trigger_comparison();
    let fixed = IniFile::from_str(&format!(
        "[TeamTypes]\n0=TT\n[TT]\nScript=MISSING_SCRIPT\nTaskForce=F\nPriority=5\n\
         [ScriptTypes]\n0=S\n[S]\n0=2,0\n\
         [TaskForces]\n0=F\n[F]\n0=1,GHOST\n\
         [AITriggerTypes]\nA=Fixed bad,TT,<all>,2,4,GHOST,{comparison},40,10,40,1,0,1,0,<none>,1,1,1\n"
    ));
    let scenario = IniFile::from_str(&format!(
        "[TeamTypes]\n0=TT\n[TT]\nPriority=20\n\
         [TaskForces]\n0=F\n[F]\n0=1,E1\n\
         [AITriggerTypes]\nA=Map repair,TT,<all>,2,4,E1,{comparison},40,10,40,1,0,1,0,<none>,1,1,1\n"
    ));
    let registry = TeamAiIniRegistry::from_sources(&fixed, &scenario, true);
    assert!(registry.fixed_source_is_complete());
    let mut sim = Simulation::new();
    sim.intern_rule_type_ids(&rules);
    sim.resolve_type_handles(&rules);

    let diagnostics = sim.install_team_ai_registry(&registry, &rules);

    assert_eq!(
        diagnostics,
        vec![
            TeamAiInstallDiagnostic::UnknownTaskForceMember {
                task_force_id: "F".to_string(),
                member_type: "GHOST".to_string(),
                source: TeamAiDefinitionSource::FixedAimd,
            },
            TeamAiInstallDiagnostic::UnknownAiTriggerObject {
                trigger_id: "A".to_string(),
                object_type: "GHOST".to_string(),
                source: TeamAiDefinitionSource::FixedAimd,
            },
        ]
    );
    assert!(
        diagnostics
            .iter()
            .all(TeamAiInstallDiagnostic::is_fixed_source_refusal)
    );
    assert_eq!(
        sim.team_script_vm.registry_counts(),
        (0, 0, 0, 0),
        "map repair/relabeling cannot erase fixed-AIMD resolution obligations"
    );
}
