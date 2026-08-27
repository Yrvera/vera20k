//! Focused House-tail integration tests for native AI activation latches.

use std::collections::BTreeMap;

use super::{HouseAiActivationOrderTestEvent, Simulation, TickLane};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::house_state::{HouseDifficulty, HouseState};

fn activation_rules(iq_production: i32) -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(&format!(
        "[General]\nFixtureOnly=1\n[IQ]\nProduction={iq_production}\n"
    )))
    .expect("House activation rules fixture parses")
}

fn insert_house(
    sim: &mut Simulation,
    name: &str,
    is_human: bool,
    current_iq: i32,
) -> crate::sim::intern::InternedId {
    let owner = sim.interner.intern(name);
    let mut house = HouseState::new(owner, 0, None, is_human, 0, 10);
    house.current_iq = current_iq;
    sim.houses.insert(owner, house);
    owner
}

fn advance(sim: &mut Simulation, rules: Option<&RuleSet>, lane: TickLane) {
    let result = sim.advance_master_frame(
        &[],
        rules,
        &BTreeMap::new(),
        None,
        None,
        67,
        lane,
        None,
    );
    assert!(result.frame_committed);
}

#[test]
fn house_ai_activation_forward_house_order_reaches_computer_and_passive_houses() {
    let rules = activation_rules(5);
    let mut sim = Simulation::new();
    sim.session.game_mode_nonzero = true;

    let human = insert_house(&mut sim, "CurrentPlayer", true, 9);
    let missing = sim.interner.intern("MissingHouse");
    let computer = insert_house(&mut sim, "Computer1", false, 5);
    let neutral = insert_house(&mut sim, "Neutral", false, 5);
    {
        let house = sim.houses.get_mut(&neutral).unwrap();
        house.multiplay_passive = true;
        house.is_defeated = true;
        house.difficulty = HouseDifficulty::Easy;
    }
    sim.session.house_order = vec![human, missing, computer, neutral];

    advance(&mut sim, Some(&rules), TickLane::Ordinary);

    assert_eq!(
        sim.houses[&human].ai_activation,
        Default::default(),
        "CurrentPlayer is the only neighboring state that gates this fixture"
    );
    for owner in [computer, neutral] {
        let latches = sim.houses[&owner].ai_activation;
        assert!(latches.production);
        assert!(latches.autocreate_allowed);
        assert!(!latches.ai_triggers_active);
        assert!(latches.auto_base_building);
    }
}

#[test]
fn house_ai_activation_full_frame_order_is_production_then_activation_defeat_and_ai() {
    let rules = activation_rules(5);
    let mut sim = Simulation::new();
    sim.session.game_mode_nonzero = true;
    sim.session.tick = 1;
    let owner = insert_house(&mut sim, "Computer1", false, 5);
    sim.session.house_order.push(owner);
    sim.ai_players
        .push(crate::sim::ai::AiPlayerState::new(owner));

    advance(&mut sim, Some(&rules), TickLane::Ordinary);

    assert_eq!(
        sim.take_house_ai_activation_order_test_trace(),
        vec![
            HouseAiActivationOrderTestEvent::ProductionCompleted,
            HouseAiActivationOrderTestEvent::HouseActivation,
            HouseAiActivationOrderTestEvent::DefeatProcessed,
            HouseAiActivationOrderTestEvent::AiGenerated,
        ],
        "moving activation across production, defeat, or actual tick_ai dispatch must fail"
    );
}

#[test]
fn house_ai_activation_empty_network_modal_runs_full_tail_but_rulesless_skips() {
    let rules = activation_rules(5);
    let mut modal = Simulation::new();
    modal.session.game_mode_nonzero = true;
    let modal_owner = insert_house(&mut modal, "Computer1", false, 5);
    modal.session.house_order.push(modal_owner);

    advance(&mut modal, Some(&rules), TickLane::NetworkModal);

    assert!(modal.houses[&modal_owner].ai_activation.production);
    assert!(modal.houses[&modal_owner].ai_activation.autocreate_allowed);
    assert!(modal.houses[&modal_owner].ai_activation.auto_base_building);

    let mut rulesless = Simulation::new();
    rulesless.session.game_mode_nonzero = true;
    let rulesless_owner = insert_house(&mut rulesless, "Computer1", false, i32::MAX);
    rulesless
        .houses
        .get_mut(&rulesless_owner)
        .unwrap()
        .ai_activation
        .auto_base_building = true;
    rulesless.session.house_order.push(rulesless_owner);

    advance(&mut rulesless, None, TickLane::Ordinary);

    let latches = rulesless.houses[&rulesless_owner].ai_activation;
    assert!(!latches.production);
    assert!(!latches.autocreate_allowed);
    assert!(!latches.ai_triggers_active);
    assert!(latches.auto_base_building);
}
