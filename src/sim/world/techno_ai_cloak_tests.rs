use super::*;
use crate::map::playfield::PlayfieldBounds;
use crate::rules::ini_parser::IniFile;
use crate::sim::combat::combat_weapon::WeaponSlot;
use crate::sim::combat::{AttackTarget, TargetKind};
use crate::sim::game_entity::PendingBuildingFire;
use crate::sim::snapshot::GameSnapshot;

fn rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[General]\nCloakingStages=9\nCloakDelay=.02\n\
         [AudioVisual]\nCloakSound=NavalUnitEmerge\n\
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
    let entity = sim.substrate.entities.get(id).unwrap();
    assert!(matches!(
        sim.sound_events.as_slice(),
        [crate::sim::world::SimSoundEvent::CloakSound {
            sound_id,
            rx,
            ry,
            sub_x,
            sub_y,
            world_z_leptons: 0,
        }] if sound_id == "NavalUnitEmerge"
            && *rx == entity.position.rx
            && *ry == entity.position.ry
            && *sub_x == entity.position.sub_x
            && *sub_y == entity.position.sub_y
    ));
    assert_eq!(
        sim.interner.get("NavalUnitEmerge"),
        None,
        "the transient entering-cloak cue must not mutate serialized interner state"
    );
    for frame in 1..=5 {
        sim.session.binary_frame = frame;
        tick_stock_cloak_producer(&mut sim, id, &rules);
    }
    assert_eq!(sim.substrate.entities.get(id).unwrap().cloak.as_ref().unwrap().state, 2);
    assert_eq!(sim.sound_events.len(), 1, "states one/two do not replay CloakSound");
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
    assert!(matches!(
        sim.sound_events.as_slice(),
        [crate::sim::world::SimSoundEvent::CloakSound { sound_id, .. }]
            if sound_id == "NavalUnitEmerge"
    ));
    let hash_with_transient_cue = sim.state_hash();
    let emitted = std::mem::take(&mut sim.sound_events);
    assert_eq!(
        sim.state_hash(),
        hash_with_transient_cue,
        "the transient positional cue is outside deterministic world hashing"
    );
    sim.sound_events = emitted;

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
    assert!(visible.sound_events.is_empty(), "no transition means no positional cue");
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

#[test]
fn sensor_callback_reassigns_admitted_targeters_in_forward_techno_registration_order() {
    let (mut sim, rules, cloaker) = spawned_sub();
    let (cell, cloaker_owner) = sim
        .substrate
        .entities
        .get(cloaker)
        .map(|entity| ((entity.position.rx, entity.position.ry), entity.owner))
        .unwrap();
    sim.fog
        .mark_visible_for_owner(cloaker_owner, cell.0, cell.1);

    let sensor_admitted = sim
        .spawn_object_at_height("RANKED", "Americans", cell.0 + 1, cell.1, 0, 0, &rules)
        .unwrap();
    let excluded = sim
        .spawn_object_at_height("RANKED", "Neutral", cell.0 + 2, cell.1, 0, 0, &rules)
        .unwrap();
    let same_owner = sim
        .spawn_object_at_height("RANKED", "Soviet", cell.0 + 3, cell.1, 0, 0, &rules)
        .unwrap();
    let later_sensor_admitted = sim
        .spawn_object_at_height("RANKED", "Americans", cell.0 + 4, cell.1, 0, 0, &rules)
        .unwrap();
    for id in [sensor_admitted, excluded, same_owner, later_sensor_admitted] {
        let entity = sim.substrate.entities.get_mut(id).unwrap();
        entity.attack_target = Some(AttackTarget::new(cloaker));
        entity.passively_acquired_target = true;
    }
    let american_owner = sim.substrate.entities.get(sensor_admitted).unwrap().owner;
    sim.fog
        .increment_sensor_at(american_owner, cell.0, cell.1);

    // Logic registration is deliberately opposite to prove the callback uses
    // Techno class-array construction order, not the active-object vector.
    sim.substrate.logic.set_order_for_test(vec![
        later_sensor_admitted,
        same_owner,
        excluded,
        sensor_admitted,
        cloaker,
    ]);
    let outcome = sensor_reevaluate_stock_cloak(&mut sim, cloaker, &rules);

    assert!(outcome.cloak_transitioned);
    assert_eq!(
        outcome.reassigned_targeters,
        vec![sensor_admitted, same_owner, later_sensor_admitted],
        "native reverse collect plus reverse dispatch is forward Techno registration order"
    );
    assert!(
        !sim.substrate
            .entities
            .get(sensor_admitted)
            .unwrap()
            .passively_acquired_target
    );
    assert!(
        !sim.substrate
            .entities
            .get(same_owner)
            .unwrap()
            .passively_acquired_target,
        "same owner admits without targeter sensor coverage"
    );
    assert!(
        !sim.substrate
            .entities
            .get(later_sensor_admitted)
            .unwrap()
            .passively_acquired_target
    );
    assert!(
        sim.substrate
            .entities
            .get(excluded)
            .unwrap()
            .passively_acquired_target,
        "an unrelated owner without sensor coverage is not collected"
    );
    assert!(matches!(
        sim.sound_events.as_slice(),
        [crate::sim::world::SimSoundEvent::CloakSound { sound_id, .. }]
            if sound_id == "NavalUnitEmerge"
    ));
}

#[test]
fn sensor_callback_rejected_cloak_still_clears_only_same_target_passive_provenance() {
    let (mut sim, rules, cloaker) = spawned_sub();
    let (cell, cloaker_owner) = sim
        .substrate
        .entities
        .get(cloaker)
        .map(|entity| ((entity.position.rx, entity.position.ry), entity.owner))
        .unwrap();
    sim.fog
        .mark_visible_for_owner(cloaker_owner, cell.0, cell.1);
    sim.substrate
        .entities
        .get_mut(cloaker)
        .unwrap()
        .cloak
        .as_mut()
        .unwrap()
        .establish_unlimbo_fully_cloaked();

    let targeter = sim
        .spawn_object_at_height("NAYARD", "Soviet", cell.0 + 1, cell.1, 0, 0, &rules)
        .unwrap();
    {
        let entity = sim.substrate.entities.get_mut(targeter).unwrap();
        let mut attack = AttackTarget::new(cloaker);
        attack.cooldown_ticks = 17;
        attack.burst_remaining = 3;
        attack.burst_delay_ticks = 4;
        entity.pending_building_fire = Some(PendingBuildingFire {
            remaining_ticks: 7,
            weapon_slot: WeaponSlot::Secondary,
        });
        entity.attack_target = Some(attack);
        entity.passively_acquired_target = true;
    }
    sim.scenario_rng = crate::sim::rng::SimRng::new(0);
    let mission_before = sim.substrate.entities.get(targeter).unwrap().mission;
    let hash_before = sim.state_hash();

    let outcome = sensor_reevaluate_stock_cloak(&mut sim, cloaker, &rules);

    assert!(!outcome.cloak_transitioned, "state two rejects StartCloaking");
    assert_eq!(outcome.reassigned_targeters, vec![targeter]);
    assert!(sim.sound_events.is_empty(), "rejected StartCloaking is silent");
    assert_eq!(
        sim.substrate
            .entities
            .get(cloaker)
            .unwrap()
            .cloak
            .as_ref()
            .unwrap()
            .state,
        2
    );
    let entity = sim.substrate.entities.get(targeter).unwrap();
    assert!(!entity.passively_acquired_target);
    let attack = entity.attack_target.as_ref().unwrap();
    assert_eq!(attack.target, TargetKind::Entity(cloaker));
    assert_eq!(attack.cooldown_ticks, 17);
    assert_eq!(attack.burst_remaining, 3);
    assert_eq!(attack.burst_delay_ticks, 4);
    assert_eq!(
        entity.pending_building_fire,
        Some(PendingBuildingFire {
            remaining_ticks: 7,
            weapon_slot: WeaponSlot::Secondary,
        })
    );
    assert_eq!(entity.mission, mission_before);
    let hash_after = sim.state_hash();
    assert_ne!(hash_after, hash_before, "passive provenance is hashed authority");

    let bytes = GameSnapshot::save(&sim, 0, 0, "sensor-targeter", 0);
    let restored = GameSnapshot::load(&bytes).expect("v88 sensor targeter snapshot").sim;
    assert_eq!(restored.state_hash(), hash_after);
    assert!(
        !restored
            .substrate
            .entities
            .get(targeter)
            .unwrap()
            .passively_acquired_target
    );
}
