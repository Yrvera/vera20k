use super::*;
use crate::map::playfield::PlayfieldBounds;
use crate::rules::ini_parser::IniFile;
use crate::sim::combat::AttackTarget;

fn rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[General]\nCloakingStages=9\nCloakDelay=.02\n\
         [VehicleTypes]\n0=SUB\n1=RANKED\n\
         [BuildingTypes]\n0=NAYARD\n\
         [SUB]\nStrength=600\nSpeed=4\nCloakable=yes\nCloakingSpeed=1\nSensorsSight=7\n\
         [RANKED]\nStrength=600\nSpeed=4\nVeteranAbilities=CLOAK\nEliteAbilities=CLOAK\n\
         [NAYARD]\nStrength=1000\nWeaponsFactory=yes\n",
    ))
    .expect("stock cloak rules")
}

fn spawned_sub() -> (Simulation, RuleSet, u64) {
    let rules = rules();
    let mut sim = Simulation::with_seed(0xC10A_C001);
    sim.fog.width = 64;
    sim.fog.height = 64;
    sim.playfield_bounds = Some(PlayfieldBounds::from_normalized_local_size(
        64, 2, 2, 56, 52,
    ));
    let bounds = sim.playfield_bounds.unwrap();
    let (rx, ry) = (8u16..56)
        .flat_map(|ry| (8u16..56).map(move |rx| (rx, ry)))
        .find(|&(rx, ry)| bounds.contains_height_aware_packed(rx.into(), ry.into(), 0, 0))
        .expect("interior mode-one cell");
    let id = sim
        .spawn_object_at_height("SUB", "Soviet", rx, ry, 0, 0, &rules)
        .unwrap();
    (sim, rules, id)
}

#[test]
fn stock_cloak_producer_healthy_trace_uses_type_speed_and_no_rng() {
    let (mut sim, rules, id) = spawned_sub();
    let before = sim.scenario_rng.logical_state();
    tick_stock_cloak_producer(&mut sim, id, &rules);
    assert_eq!(sim.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 1);
    assert_eq!(sim.scenario_rng.logical_state(), before);
    for frame in 1..=5 {
        sim.session.binary_frame = frame;
        tick_stock_cloak_producer(&mut sim, id, &rules);
    }
    assert_eq!(sim.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 2);
}

#[test]
fn stock_cloak_producer_current_fire_and_weapons_factory_contact_block_entry() {
    let (mut sim, rules, id) = spawned_sub();
    sim.substrate.entities.get_mut(id).unwrap().attack_target = Some(AttackTarget::new(999));
    tick_stock_cloak_producer(&mut sim, id, &rules);
    assert_eq!(sim.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 0);

    sim.substrate.entities.get_mut(id).unwrap().attack_target = None;
    let yard = sim
        .spawn_object_at_height("NAYARD", "Soviet", 24, 20, 0, 0, &rules)
        .unwrap();
    sim.substrate.entities.get_mut(id).unwrap().radio_contacts.insert(yard);
    tick_stock_cloak_producer(&mut sim, id, &rules);
    assert_eq!(sim.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 0);

    sim.substrate.entities.get_mut(id).unwrap().radio_contacts.remove(yard);
    tick_stock_cloak_producer(&mut sim, id, &rules);
    assert_eq!(sim.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 1);
}

#[test]
fn stock_cloak_producer_should_uncloak_uses_current_activity_and_owner_visibility() {
    let (mut sim, rules, id) = spawned_sub();
    sim.substrate.entities
        .get_mut(id)
        .unwrap()
        .cloak
        .as_mut()
        .unwrap()
        .establish_unlimbo_fully_cloaked();
    sim.substrate.entities.get_mut(id).unwrap().attack_target = Some(AttackTarget::new(999));
    tick_stock_cloak_producer(&mut sim, id, &rules);
    assert_eq!(sim.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 3);

    let (mut visible, rules, id) = spawned_sub();
    let owner = visible.substrate.entities.get(id).unwrap().owner;
    let (rx, ry) = visible
        .substrate
        .entities
        .get(id)
        .map(|entity| (entity.position.rx, entity.position.ry))
        .unwrap();
    visible.fog.mark_visible_for_owner(owner, rx, ry);
    visible.substrate.entities
        .get_mut(id)
        .unwrap()
        .cloak
        .as_mut()
        .unwrap()
        .establish_unlimbo_fully_cloaked();
    visible.substrate.entities.get_mut(id).unwrap().attack_target = Some(AttackTarget::new(999));
    tick_stock_cloak_producer(&mut visible, id, &rules);
    assert_eq!(visible.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 2);
}

#[test]
fn stock_cloak_producer_honors_rank_selected_cloak_ability() {
    let (mut sim, rules, sub) = spawned_sub();
    let (rx, ry) = sim
        .substrate
        .entities
        .get(sub)
        .map(|entity| (entity.position.rx, entity.position.ry))
        .unwrap();
    let ranked = sim
        .spawn_object_at_height("RANKED", "Soviet", rx, ry, 0, 0, &rules)
        .unwrap();
    assert!(sim.substrate.entities.get(ranked).unwrap().cloak.is_none());
    sim.substrate.entities.get_mut(ranked).unwrap().veterancy = 100;
    tick_stock_cloak_producer(&mut sim, ranked, &rules);
    assert_eq!(
        sim.substrate.entities.get(ranked).unwrap().cloak.as_ref().unwrap().state,
        1
    );
}
