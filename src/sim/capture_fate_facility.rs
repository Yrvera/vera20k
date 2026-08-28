//! CaptureManager fate actions 2/3 terminal per-cell transactions.
//!
//! Selection and mission cadence live in `capture_manager` and `techno_ai`.
//! This module is the narrow Unit/Infantry PerCellProcess owner: it runs only
//! after normal movement committed a new cell containing the selected Building,
//! and it re-fetches stable identities across every synchronous callback.

use crate::map::entities::EntityCategory;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::NavTargetRef;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::passenger::PassengerRole;
use crate::sim::radio::{RadioMessage, RadioResponse};
use crate::sim::world::{SimSoundEvent, Simulation, TriggerInputs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureFatePerCellResult {
    None,
    GrinderConsumed,
    AbsorberBoarded,
    UnitAbsorberRejected,
}

#[derive(Debug, Clone, Copy)]
struct Arrival {
    victim_id: u64,
    building_id: u64,
    category: EntityCategory,
    mission: MissionType,
}

/// Run the class-specific terminal leaf after a real per-cell movement commit.
///
/// gamemd provenance: UnitClass::PerCellProcess @ 0x00739EC0 and
/// InfantryClass::PerCellProcess @ 0x00519630, including the Grinder and
/// Bio-Reactor arrival transactions documented in
/// CAPTURE_MANAGER_FATE_GRINDER_ABSORBER_CONTINUATIONS_GHIDRA_REPORT.md.
pub(crate) fn process_per_cell(
    sim: &mut Simulation,
    victim_id: u64,
    old_cell: Option<(u16, u16)>,
    new_cell: Option<(u16, u16)>,
    rules: &RuleSet,
    trigger_inputs: Option<TriggerInputs<'_>>,
) -> CaptureFatePerCellResult {
    if old_cell == new_cell {
        return CaptureFatePerCellResult::None;
    }
    let Some(arrival) = resolve_arrival(sim, victim_id, new_cell, rules) else {
        return CaptureFatePerCellResult::None;
    };
    match arrival.mission {
        MissionType::Eaten => {
            grinder(sim, rules, trigger_inputs, arrival);
            CaptureFatePerCellResult::GrinderConsumed
        }
        MissionType::Enter => absorber(sim, rules, trigger_inputs, arrival),
        _ => CaptureFatePerCellResult::None,
    }
}

fn resolve_arrival(
    sim: &Simulation,
    victim_id: u64,
    new_cell: Option<(u16, u16)>,
    rules: &RuleSet,
) -> Option<Arrival> {
    let cell = new_cell?;
    let victim = sim.substrate.entities.get(victim_id)?;
    if victim.dying
        || victim.lifecycle.in_limbo
        || !matches!(victim.category, EntityCategory::Unit | EntityCategory::Infantry)
    {
        return None;
    }
    let mission = victim.mission.current().known()?;
    if !matches!(mission, MissionType::Eaten | MissionType::Enter) {
        return None;
    }
    let NavTargetRef::Building { id: building_id } = victim.navigation.nav_com? else {
        return None;
    };
    // Look_up_building_in_cell walks the current ground CellClass list. A
    // selected foundation may consume on its first entered base cell; arrival
    // is not restricted to Building GetCoords' centre destination.
    if sim
        .substrate
        .occupancy
        .first_building_on_layer(cell.0, cell.1, MovementLayer::Ground)
        != Some(building_id)
    {
        return None;
    }
    let building = sim.substrate.entities.get(building_id)?;
    if building.dying || building.being_temporally_warped_out {
        return None;
    }
    let object = rules.object(sim.interner.resolve(building.type_ref))?;
    let admitted = match (victim.category, mission) {
        (_, MissionType::Eaten) => object.grinding,
        (EntityCategory::Unit, MissionType::Enter) => object.unit_absorb,
        (EntityCategory::Infantry, MissionType::Enter) => object.infantry_absorb,
        _ => false,
    };
    admitted.then_some(Arrival {
        victim_id,
        building_id,
        category: victim.category,
        mission,
    })
}

fn dispatch_tag(
    sim: &mut Simulation,
    inputs: Option<TriggerInputs<'_>>,
    tag_owner_id: u64,
    object_id: u64,
    event_id: i32,
) {
    if let Some(inputs) = inputs {
        let _ = sim.dispatch_attached_tag_event(inputs, tag_owner_id, object_id, event_id);
    }
}

fn emit_positional_sound(sim: &mut Simulation, victim_id: u64, sound_id: String) {
    let Some(world) = sim.anim_owner_coords(victim_id) else {
        return;
    };
    sim.sound_events
        .push(SimSoundEvent::CaptureFateSound { sound_id, world });
}

fn emit_random_type_sound(sim: &mut Simulation, victim_id: u64, choices: &[String]) {
    if choices.is_empty() {
        return;
    }
    let index = (sim.main_rng.next_u32() % choices.len() as u32) as usize;
    emit_positional_sound(sim, victim_id, choices[index].clone());
}

fn victim_object(sim: &Simulation, rules: &RuleSet, victim_id: u64) -> Option<ObjectType> {
    let type_ref = sim.substrate.entities.get(victim_id)?.type_ref;
    rules.object(sim.interner.resolve(type_ref)).cloned()
}

fn credit_refund(
    sim: &mut Simulation,
    rules: &RuleSet,
    refunded_id: u64,
    refund_owner: crate::sim::intern::InternedId,
) {
    let Some(object) = victim_object(sim, rules, refunded_id) else {
        return;
    };
    let value = crate::sim::capture_manager::grinder_refund_value(
        sim,
        rules,
        &object,
        Some(refund_owner),
    );
    if let Some(house) = sim.houses.get_mut(&refund_owner) {
        house.credits = house.credits.wrapping_add(value);
    }
}

fn detach_first_passenger(sim: &mut Simulation, carrier_id: u64) -> Option<u64> {
    let passenger_id = sim
        .substrate
        .entities
        .get_mut(carrier_id)?
        .passenger_role
        .cargo_mut()?
        .unload_first()?
        .0;
    if let Some(passenger) = sim.substrate.entities.get_mut(passenger_id) {
        let _ = passenger.passenger_role.leave_transport_if(carrier_id);
    }
    Some(passenger_id)
}

fn refund_and_uninit(
    sim: &mut Simulation,
    rules: &RuleSet,
    stable_id: u64,
    refund_owner: crate::sim::intern::InternedId,
) {
    credit_refund(sim, rules, stable_id, refund_owner);
    sim.uninit_with_rules(stable_id, rules);
}

fn drain_unit_grinder_cargo(
    sim: &mut Simulation,
    rules: &RuleSet,
    victim_id: u64,
    refund_owner: crate::sim::intern::InternedId,
) {
    while let Some(outer_id) = detach_first_passenger(sim, victim_id) {
        // UnitClass's grinder leaf explicitly descends one additional Cargo
        // link. Any third-level cargo is destroyed by the inner object's
        // ordinary UnInit, but it is not separately refunded by this walk.
        while let Some(inner_id) = detach_first_passenger(sim, outer_id) {
            refund_and_uninit(sim, rules, inner_id, refund_owner);
        }
        refund_and_uninit(sim, rules, outer_id, refund_owner);
    }
}

fn detach_grinder_parasite(
    sim: &mut Simulation,
    rules: &RuleSet,
    victim_id: u64,
    refund_owner: crate::sim::intern::InternedId,
) {
    let Some(attacker_id) = sim
        .substrate
        .entities
        .get(victim_id)
        .and_then(|victim| victim.parasite_attacker_id)
    else {
        return;
    };
    credit_refund(sim, rules, attacker_id, refund_owner);
    let detach_started_frame = sim.session.binary_frame as i32;
    if let Some(manager) = sim
        .substrate
        .entities
        .get_mut(attacker_id)
        .and_then(|attacker| attacker.parasite_manager.as_mut())
    {
        manager.victim_id = None;
        manager.detach_started_frame = detach_started_frame;
        manager.detach_duration_frames = 0x32;
    }
    if let Some(victim) = sim.substrate.entities.get_mut(victim_id) {
        if victim.parasite_attacker_id == Some(attacker_id) {
            victim.parasite_attacker_id = None;
        }
    }
}

fn grinder(
    sim: &mut Simulation,
    rules: &RuleSet,
    trigger_inputs: Option<TriggerInputs<'_>>,
    arrival: Arrival,
) {
    dispatch_tag(
        sim,
        trigger_inputs,
        arrival.building_id,
        arrival.victim_id,
        1,
    );
    let Some(object) = victim_object(sim, rules, arrival.victim_id) else {
        return;
    };
    let victim_owner = sim
        .substrate
        .entities
        .get(arrival.victim_id)
        .map(|victim| victim.owner);
    if victim_owner.is_some_and(|owner| crate::sim::capture_manager::is_human_player_exact(sim, owner))
    {
        emit_random_type_sound(sim, arrival.victim_id, &object.voice_die);
    }
    emit_random_type_sound(sim, arrival.victim_id, &object.die_sounds);

    // Event 1 may have changed the facility owner; native credits the owner
    // observed by the per-cell continuation after the callback.
    let Some(refund_owner) = sim
        .substrate
        .entities
        .get(arrival.building_id)
        .map(|building| building.owner)
    else {
        return;
    };
    credit_refund(sim, rules, arrival.victim_id, refund_owner);
    if arrival.category == EntityCategory::Unit {
        drain_unit_grinder_cargo(sim, rules, arrival.victim_id, refund_owner);
        detach_grinder_parasite(sim, rules, arrival.victim_id, refund_owner);
    }

    dispatch_tag(
        sim,
        trigger_inputs,
        arrival.victim_id,
        arrival.victim_id,
        0x30,
    );

    let still_grinding = sim
        .substrate
        .entities
        .get(arrival.building_id)
        .and_then(|building| rules.object(sim.interner.resolve(building.type_ref)))
        .is_some_and(|object| object.grinding);
    if still_grinding {
        if let Some(sound) = rules.general.enter_grinder_sound.clone() {
            emit_positional_sound(sim, arrival.victim_id, sound);
        }
        crate::sim::world::building_anim::trigger_grinder_special_anim(
            sim,
            rules,
            arrival.building_id,
        );
    }
    sim.uninit_with_rules(arrival.victim_id, rules);
}

fn absorber(
    sim: &mut Simulation,
    rules: &RuleSet,
    trigger_inputs: Option<TriggerInputs<'_>>,
    arrival: Arrival,
) -> CaptureFatePerCellResult {
    if arrival.category == EntityCategory::Infantry {
        dispatch_tag(
            sim,
            trigger_inputs,
            arrival.building_id,
            arrival.victim_id,
            1,
        );
        // The callback may replace either identity or its first contact.
        let relation_live = sim.substrate.entities.get(arrival.victim_id).is_some_and(|victim| {
            victim.navigation.nav_com == Some(NavTargetRef::building(arrival.building_id))
                && victim.radio_contacts.slot(0) == Some(arrival.building_id)
                && sim.substrate.occupancy.first_building_on_layer(
                    victim.position.rx,
                    victim.position.ry,
                    MovementLayer::Ground,
                ) == Some(arrival.building_id)
        });
        if !relation_live {
            return CaptureFatePerCellResult::None;
        }
    }

    let message = if arrival.category == EntityCategory::Unit {
        RadioMessage::CanEnter
    } else {
        RadioMessage::DockNow
    };
    let response = crate::sim::radio::transmit_capture_fate_absorber(
        sim,
        rules,
        arrival.victim_id,
        arrival.building_id,
        message,
    );
    if response != RadioResponse::Roger {
        if arrival.category == EntityCategory::Unit {
            if let Some(victim) = sim.substrate.entities.get_mut(arrival.victim_id) {
                crate::sim::movement::capture_fate_stop_moving(victim);
            }
            let _ = sim.mission_assign_exact(
                arrival.victim_id,
                MissionId::from_known(MissionType::Guard),
                sim.session.binary_frame,
            );
            let path_grid = sim.path_grid_snapshot();
            let _ = crate::sim::movement::bump_crush::scatter_blocker(
                &mut sim.substrate.entities,
                arrival.victim_id,
                path_grid.as_deref(),
                &sim.substrate.occupancy,
                MovementLayer::Ground,
                &mut sim.scenario_rng,
                Some(&rules.mission_control),
                false,
            );
            return CaptureFatePerCellResult::UnitAbsorberRejected;
        }
        return CaptureFatePerCellResult::None;
    }

    if let Some(victim) = sim.substrate.entities.get_mut(arrival.victim_id) {
        crate::sim::movement::capture_fate_stop_moving(victim);
    }
    if let Some(controller_id) = sim
        .substrate
        .entities
        .get(arrival.victim_id)
        .and_then(|victim| victim.mind_control_controller_id)
    {
        let _ = crate::sim::capture_manager::free_unit(
            sim,
            rules,
            controller_id,
            arrival.victim_id,
        );
    }

    if arrival.category == EntityCategory::Infantry
        && let Some(sound) = rules.general.enter_bio_reactor_sound.clone()
    {
        emit_positional_sound(sim, arrival.victim_id, sound);
    }

    let passenger_size = victim_object(sim, rules, arrival.victim_id).map_or(1, |object| object.size);
    if arrival.category == EntityCategory::Infantry
        && let Some(victim) = sim.substrate.entities.get_mut(arrival.victim_id)
    {
        // `InfantryClass::PerCell @ 0x00519630` sets +0x439 before Limbo.
        // House Add_Tracking observes this byte and cannot re-add the infantry
        // during the synchronous conceal/exit transaction.
        victim.infantry_absorber_occupant = true;
    }
    let conceal = sim.techno_limbo_with_rules(arrival.victim_id, rules);
    if !matches!(
        conceal,
        crate::sim::world::ConcealOutcome::Concealed
            | crate::sim::world::ConcealOutcome::AlreadyConcealed
    ) {
        return CaptureFatePerCellResult::None;
    }
    if arrival.category == EntityCategory::Infantry {
        // The native conditional House+0x2F4 decrement and +0x438 clear occur
        // only after Limbo returns. The absorber still owns the infantry as a
        // Techno, so the aggregate owned-unit count is deliberately untouched.
        sim.remove_infantry_tracking_once(arrival.victim_id);
    }
    if let Some(victim) = sim.substrate.entities.get_mut(arrival.victim_id) {
        victim
            .passenger_role
            .enter_transport_preserving_cargo(arrival.building_id);
    }
    let boarded = sim
        .substrate
        .entities
        .get_mut(arrival.building_id)
        .and_then(|building| building.passenger_role.cargo_mut())
        .map(|cargo| cargo.board_native_head(arrival.victim_id, passenger_size))
        .is_some();
    if !boarded {
        return CaptureFatePerCellResult::None;
    }

    if arrival.category == EntityCategory::Unit {
        let _ = sim.mission_assign_exact(
            arrival.victim_id,
            MissionId::NONE,
            sim.session.binary_frame,
        );
    } else if let Some(victim) = sim.substrate.entities.get_mut(arrival.victim_id) {
        // InfantryClass runs StopMoving after CargoClass::AddPassenger. This
        // clears any destination installed by FreeUnit's DecideUnitFate while
        // preserving its mission and persistent +0x68F absorb intent.
        crate::sim::movement::capture_fate_stop_moving(victim);
    }
    CaptureFatePerCellResult::AbsorberBoarded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::house_state::HouseState;
    use crate::sim::occupancy::OccupancyGrid;

    fn rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\nRefundPercent=50%\nAICaptureNormal=0,0,100,0\nAICaptureLowMoney=0,0,0,100\n\
             [AudioVisual]\nEnterGrinderSound=GrindEnter\nEnterBioReactorSound=BioEnter\n\
             [VehicleTypes]\n0=UNIT\n\
             [InfantryTypes]\n0=INF\n\
             [BuildingTypes]\n0=GRINDER\n1=BIO\n2=UBIO\n\
             [UNIT]\nStrength=100\nCost=100\nSoylent=75\nSize=2\nSpeed=5\nROT=5\nPassengers=5\nMovementZone=Normal\nLocomotor={4A582741-9839-11D1-B709-00A024DDAFD1}\nVoiceDie=VoiceA,VoiceB\nDieSound=DieA,DieB\n\
             [INF]\nStrength=100\nCost=80\nSize=1\nSpeed=4\nMovementZone=Infantry\nLocomotor={4A582744-9839-11D1-B709-00A024DDAFD1}\n\
             [GRINDER]\nStrength=500\nGrinding=yes\nPassengers=5\n\
             [BIO]\nStrength=500\nInfantryAbsorb=yes\nPassengers=5\nSizeLimit=15\nPower=150\nExtraPower=100\n\
             [UBIO]\nStrength=500\nUnitAbsorb=yes\nPassengers=5\nSizeLimit=15\nPower=150\nExtraPower=100\n\
             [Eaten]\nRate=.016\n\
             [Enter]\nRate=.016\n",
        ))
        .expect("capture-fate terminal rules")
    }

    fn fixture(victim_type: &str, facility_type: &str) -> (Simulation, RuleSet, u64, u64) {
        let rules = rules();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Owner");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, false, 0, 10));
        sim.session.house_order.push(owner);
        let victim_id = sim
            .spawn_object(victim_type, "Owner", 4, 4, 0, &rules, &Default::default())
            .expect("victim");
        let facility_id = sim
            .spawn_object(facility_type, "Owner", 7, 4, 0, &rules, &Default::default())
            .expect("facility");
        (sim, rules, victim_id, facility_id)
    }

    fn commit_arrival(
        sim: &mut Simulation,
        victim_id: u64,
        facility_id: u64,
        mission: MissionType,
    ) -> ((u16, u16), (u16, u16)) {
        let old = sim
            .substrate
            .entities
            .get(victim_id)
            .map(|victim| (victim.position.rx, victim.position.ry))
            .unwrap();
        let target = sim
            .substrate
            .entities
            .get(facility_id)
            .map(|building| (building.position.rx, building.position.ry))
            .unwrap();
        let _ = sim.mission_assign_exact(
            victim_id,
            MissionId::from_known(mission),
            sim.session.binary_frame,
        );
        let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
        victim.navigation.nav_com = Some(NavTargetRef::building(facility_id));
        victim.position.rx = target.0;
        victim.position.ry = target.1;
        sim.substrate.occupancy = OccupancyGrid::rebuild(&sim.substrate.entities);
        (old, target)
    }

    #[test]
    fn unit_grinder_terminal_credits_current_owner_draws_two_type_sounds_and_uninits() {
        let (mut sim, rules, victim_id, grinder_id) = fixture("UNIT", "GRINDER");
        sim.session.game_mode_nonzero = true;
        let victim_owner = sim.substrate.entities.get(victim_id).unwrap().owner;
        sim.houses.get_mut(&victim_owner).unwrap().player_control = true;
        let facility_owner = sim.interner.intern("FacilityOwner");
        sim.houses.insert(
            facility_owner,
            HouseState::new(facility_owner, 1, None, false, 0, 10),
        );
        sim.session.house_order.push(facility_owner);
        sim.change_owner_with_rules(grinder_id, facility_owner, &rules);
        sim.reseed_scenario_and_main(0xC0FFEE);
        let mut expected_main = sim.main_rng.clone();
        let _ = expected_main.next_u32();
        let _ = expected_main.next_u32();
        let (old, new) = commit_arrival(&mut sim, victim_id, grinder_id, MissionType::Eaten);
        let object = sim.substrate.entities.get_mut(victim_id).unwrap();
        object.lifecycle.object_alive = true;
        object.lifecycle.cell_marked = true;
        assert_eq!(
            process_per_cell(&mut sim, victim_id, Some(old), Some(new), &rules, None),
            CaptureFatePerCellResult::GrinderConsumed,
        );
        assert_eq!(sim.houses.get(&victim_owner).unwrap().credits, 0);
        assert_eq!(sim.houses.get(&facility_owner).unwrap().credits, 75);
        assert_eq!(sim.rng_views().main, expected_main.logical_view());
        assert!(!sim.substrate.entities.get(victim_id).unwrap().lifecycle.object_alive);
        assert!(sim.substrate.pending_delete.contains(&victim_id));
        assert_eq!(
            sim.sound_events.len(),
            3,
            "VoiceDie, DieSound, then EnterGrinderSound",
        );
        let ids = sim
            .sound_events
            .iter()
            .filter_map(|event| match event {
                SimSoundEvent::CaptureFateSound { sound_id, .. } => Some(sound_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(ids[0].starts_with("Voice"));
        assert!(ids[1].starts_with("Die"));
        assert_eq!(ids[2], "GrindEnter");
    }

    fn nest_cargo(
        sim: &mut Simulation,
        rules: &RuleSet,
        carrier_id: u64,
        passenger_id: u64,
        passenger_size: u32,
    ) {
        assert!(matches!(
            sim.techno_limbo_with_rules(passenger_id, rules),
            crate::sim::world::ConcealOutcome::Concealed
        ));
        sim.substrate
            .entities
            .get_mut(carrier_id)
            .unwrap()
            .passenger_role
            .cargo_mut()
            .expect("carrier CargoClass")
            .board_forced(passenger_id, passenger_size);
        sim.substrate
            .entities
            .get_mut(passenger_id)
            .unwrap()
            .passenger_role
            .enter_transport_preserving_cargo(carrier_id);
    }

    #[test]
    fn unit_grinder_drains_exactly_two_refund_levels_and_detaches_parasite_timer() {
        let (mut sim, rules, victim_id, grinder_id) = fixture("UNIT", "GRINDER");
        let owner = sim.substrate.entities.get(grinder_id).unwrap().owner;
        let outer_id = sim
            .spawn_object("UNIT", "Owner", 3, 3, 0, &rules, &Default::default())
            .unwrap();
        let inner_id = sim
            .spawn_object("UNIT", "Owner", 3, 4, 0, &rules, &Default::default())
            .unwrap();
        let third_id = sim
            .spawn_object("INF", "Owner", 3, 5, 0, &rules, &Default::default())
            .unwrap();
        let parasite_id = sim
            .spawn_object("UNIT", "Owner", 3, 6, 0, &rules, &Default::default())
            .unwrap();
        nest_cargo(&mut sim, &rules, victim_id, outer_id, 2);
        nest_cargo(&mut sim, &rules, outer_id, inner_id, 2);
        nest_cargo(&mut sim, &rules, inner_id, third_id, 1);
        assert!(matches!(
            sim.techno_limbo_with_rules(parasite_id, &rules),
            crate::sim::world::ConcealOutcome::Concealed
        ));
        sim.substrate.entities.get_mut(parasite_id).unwrap().parasite_manager = Some(
            crate::sim::parasite_attachment::ParasiteManagerState {
                victim_id: Some(victim_id),
                ..Default::default()
            },
        );
        sim.substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .parasite_attacker_id = Some(parasite_id);
        sim.session.binary_frame = 77;
        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
        let (old, new) = commit_arrival(&mut sim, victim_id, grinder_id, MissionType::Eaten);

        assert_eq!(
            process_per_cell(&mut sim, victim_id, Some(old), Some(new), &rules, None),
            CaptureFatePerCellResult::GrinderConsumed,
        );
        assert_eq!(
            sim.houses.get(&owner).unwrap().credits,
            300,
            "victim + outer + inner + parasite refund; third level only UnInit",
        );
        for stable_id in [victim_id, outer_id, inner_id, third_id] {
            let entity = sim.substrate.entities.get(stable_id).unwrap();
            assert!(!entity.lifecycle.object_alive, "{stable_id} uninitialized");
            assert!(sim.substrate.pending_delete.contains(&stable_id));
        }
        let parasite = sim.substrate.entities.get(parasite_id).unwrap();
        assert_eq!(parasite.parasite_manager.as_ref().unwrap().victim_id, None);
        assert_eq!(
            parasite
                .parasite_manager
                .as_ref()
                .unwrap()
                .detach_started_frame,
            77,
        );
        assert_eq!(
            parasite
                .parasite_manager
                .as_ref()
                .unwrap()
                .detach_duration_frames,
            0x32,
        );
        assert_eq!(
            sim.substrate.entities.get(victim_id).unwrap().parasite_attacker_id,
            None,
        );

        let expected_hash = sim.state_hash();
        let bytes = crate::sim::snapshot::GameSnapshot::save(
            &sim,
            0,
            0,
            "capture-fate-parasite-detach",
            0,
        );
        let mut restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("v117 capture-fate snapshot")
            .sim;
        restored
            .restore_after_snapshot_load()
            .expect("parasite detach references resolve");
        let manager = restored
            .substrate
            .entities
            .get(parasite_id)
            .unwrap()
            .parasite_manager
            .as_ref()
            .unwrap();
        assert_eq!((manager.detach_started_frame, manager.detach_duration_frames), (77, 0x32));
        assert_eq!(restored.state_hash(), expected_hash);
    }

    #[test]
    fn infantry_grinder_empty_type_sound_lists_consume_no_selection_rng() {
        let (mut sim, rules, victim_id, grinder_id) = fixture("INF", "GRINDER");
        let before_main = sim.main_rng.logical_state();
        let (old, new) = commit_arrival(&mut sim, victim_id, grinder_id, MissionType::Eaten);
        assert_eq!(
            process_per_cell(&mut sim, victim_id, Some(old), Some(new), &rules, None),
            CaptureFatePerCellResult::GrinderConsumed,
        );
        assert_eq!(sim.main_rng.logical_state(), before_main);
        assert!(matches!(
            sim.sound_events.as_slice(),
            [SimSoundEvent::CaptureFateSound { sound_id, .. }] if sound_id == "GrindEnter"
        ));
    }

    #[test]
    fn infantry_absorber_uses_weak_0x15_head_boards_preserves_mission_and_adds_power() {
        let (mut sim, rules, victim_id, bio_id) = fixture("INF", "BIO");
        let victim_owner = sim.substrate.entities.get(victim_id).unwrap().owner;
        assert_eq!(sim.houses[&victim_owner].tracked_infantry_count, 1);
        assert!(
            sim.substrate
                .entities
                .get(victim_id)
                .unwrap()
                .infantry_house_tracked
        );
        assert_eq!(
            crate::sim::radio::transmit_pre_admitted_hello(&mut sim, victim_id, bio_id),
            RadioResponse::Roger,
        );
        let new_owner = sim.interner.intern("NewOwner");
        sim.houses.insert(
            new_owner,
            HouseState::new(new_owner, 1, None, false, 0, 10),
        );
        sim.session.house_order.push(new_owner);
        sim.change_owner_with_rules(bio_id, new_owner, &rules);
        {
            let cargo = sim
                .substrate
                .entities
                .get_mut(bio_id)
                .unwrap()
                .passenger_role
                .cargo_mut()
                .unwrap();
            for id in 100..105 {
                cargo.board_forced(id, 1);
            }
        }
        let (old, new) = commit_arrival(&mut sim, victim_id, bio_id, MissionType::Enter);
        sim.substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .ai_absorb_enter_pending = true;
        assert_eq!(
            process_per_cell(&mut sim, victim_id, Some(old), Some(new), &rules, None),
            CaptureFatePerCellResult::AbsorberBoarded,
        );
        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert!(victim.lifecycle.in_limbo);
        assert!(victim.ai_absorb_enter_pending);
        assert!(victim.infantry_absorber_occupant);
        assert!(!victim.infantry_house_tracked);
        assert_eq!(
            sim.houses[&victim_owner].tracked_infantry_count,
            0,
            "PerCell removes House+0x2F4 only after successful Limbo",
        );
        assert_eq!(victim.mission.current().known(), Some(MissionType::Enter));
        assert!(matches!(
            victim.passenger_role,
            PassengerRole::Inside { transport_id } if transport_id == bio_id
        ));
        assert_eq!(
            sim.substrate
                .entities
                .get(bio_id)
                .unwrap()
                .passenger_role
                .cargo()
                .unwrap()
                .passengers,
            [victim_id, 100, 101, 102, 103, 104],
        );
        let _ = crate::sim::power_system::tick_power_states(
            &mut sim.power_states,
            &mut sim.substrate.entities,
            &rules,
            &sim.interner,
        );
        assert_eq!(sim.power_states.get(&new_owner).unwrap().total_output, 750);
        assert!(sim.houses.contains_key(&victim_owner));
        assert!(sim.sound_events.iter().any(|event| matches!(
            event,
            SimSoundEvent::CaptureFateSound { sound_id, .. } if sound_id == "BioEnter"
        )));
    }

    #[test]
    fn infantry_absorber_tracking_decrement_is_native_u32_wrapping() {
        let (mut sim, rules, victim_id, bio_id) = fixture("INF", "BIO");
        let owner = sim.substrate.entities.get(victim_id).unwrap().owner;
        sim.houses.get_mut(&owner).unwrap().tracked_infantry_count = 0;
        assert_eq!(
            crate::sim::radio::transmit_pre_admitted_hello(&mut sim, victim_id, bio_id),
            RadioResponse::Roger,
        );
        let (old, new) = commit_arrival(&mut sim, victim_id, bio_id, MissionType::Enter);
        assert_eq!(
            process_per_cell(&mut sim, victim_id, Some(old), Some(new), &rules, None),
            CaptureFatePerCellResult::AbsorberBoarded,
        );
        assert_eq!(sim.houses[&owner].tracked_infantry_count, u32::MAX);
        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert!(!victim.infantry_house_tracked);
        assert!(victim.infantry_absorber_occupant);
    }

    #[test]
    fn unit_absorber_frees_control_preserves_inner_cargo_and_clears_mission() {
        let (mut sim, rules, victim_id, bio_id) = fixture("UNIT", "UBIO");
        let controller_owner = sim.substrate.entities.get(bio_id).unwrap().owner;
        let original_owner = sim.interner.intern("OriginalOwner");
        sim.houses.insert(
            original_owner,
            HouseState::new(original_owner, 1, None, false, 0, 10),
        );
        sim.session.house_order.push(original_owner);
        sim.change_owner_with_rules(victim_id, original_owner, &rules);

        let controller_id = sim
            .spawn_object("UNIT", "Owner", 2, 2, 0, &rules, &Default::default())
            .expect("controller");
        sim.substrate.entities.get_mut(controller_id).unwrap().capture_manager = Some(
            crate::sim::capture_manager::CaptureManagerState {
                max_control: 1,
                infinite_mind_control: false,
                controlled_nodes: Vec::new(),
            },
        );
        assert!(crate::sim::capture_manager::capture_unit(
            &mut sim,
            &rules,
            controller_id,
            victim_id,
            0,
        ));
        assert_eq!(
            sim.substrate.entities.get(victim_id).unwrap().owner,
            controller_owner,
        );

        let inner_id = sim
            .spawn_object("INF", "Owner", 3, 3, 0, &rules, &Default::default())
            .expect("inner cargo");
        assert!(matches!(
            sim.techno_limbo_with_rules(inner_id, &rules),
            crate::sim::world::ConcealOutcome::Concealed
        ));
        sim.substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .passenger_role
            .cargo_mut()
            .unwrap()
            .board_forced(inner_id, 1);
        sim.substrate.entities.get_mut(inner_id).unwrap().passenger_role =
            PassengerRole::Inside {
                transport_id: victim_id,
            };

        assert_eq!(
            crate::sim::radio::transmit_pre_admitted_hello(&mut sim, victim_id, bio_id),
            RadioResponse::Roger,
        );
        let (old, new) = commit_arrival(&mut sim, victim_id, bio_id, MissionType::Enter);
        assert_eq!(
            process_per_cell(&mut sim, victim_id, Some(old), Some(new), &rules, None),
            CaptureFatePerCellResult::AbsorberBoarded,
        );

        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert_eq!(victim.owner, original_owner, "FreeUnit restores first");
        assert_eq!(victim.mind_control_controller_id, None);
        assert!(!victim.infantry_house_tracked);
        assert!(!victim.infantry_absorber_occupant);
        assert_eq!(victim.mission.current(), MissionId::NONE);
        assert_eq!(victim.passenger_role.inside_transport_id(), Some(bio_id));
        assert_eq!(
            victim.passenger_role.cargo().unwrap().passengers,
            [inner_id],
            "outer absorber boarding preserves the Unit's own CargoClass",
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
            [],
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(bio_id)
                .unwrap()
                .passenger_role
                .cargo()
                .unwrap()
                .passengers[0],
            victim_id,
        );

        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
        let expected_hash = sim.state_hash();
        let bytes = crate::sim::snapshot::GameSnapshot::save(
            &sim,
            0,
            0,
            "capture-fate-nested-cargo",
            0,
        );
        let mut restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("v117 nested cargo snapshot")
            .sim;
        restored
            .restore_after_snapshot_load()
            .expect("nested cargo references resolve");
        let restored_victim = restored.substrate.entities.get(victim_id).unwrap();
        assert_eq!(restored_victim.passenger_role.inside_transport_id(), Some(bio_id));
        assert_eq!(
            restored_victim.passenger_role.cargo().unwrap().passengers,
            [inner_id],
        );
        assert_eq!(restored.state_hash(), expected_hash);

        let mut corrupt = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("v117 nested cargo corruption fixture")
            .sim;
        if let PassengerRole::TransportInside { transport_id, .. } = &mut corrupt
            .substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .passenger_role
        {
            *transport_id = 999_999;
        } else {
            panic!("fixture must retain the nested CargoClass relation");
        }
        assert!(matches!(
            corrupt.restore_after_snapshot_load(),
            Err(crate::sim::snapshot::SnapshotRestoreError::UnresolvedObjectReference {
                source_registry: "EntityStore",
                source_id,
                field: "passenger_role.transport_inside",
                target_registry: "EntityStore",
                target_id: 999_999,
            }) if source_id == victim_id
        ));
    }

    #[test]
    fn captured_infantry_physically_approaches_then_weak_boards_stock_absorber() {
        let rules = rules();
        let mut sim = Simulation::new();
        let controller_owner = sim.interner.intern("ControllerOwner");
        let original_owner = sim.interner.intern("OriginalOwner");
        sim.houses.insert(
            controller_owner,
            HouseState::new(controller_owner, 0, None, false, 10_000, 10),
        );
        sim.houses.insert(
            original_owner,
            HouseState::new(original_owner, 1, None, false, 0, 10),
        );
        sim.session.house_order.extend([controller_owner, original_owner]);
        let controller_id = sim
            .spawn_object(
                "UNIT",
                "ControllerOwner",
                1,
                4,
                0,
                &rules,
                &Default::default(),
            )
            .unwrap();
        let victim_id = sim
            .spawn_object(
                "INF",
                "OriginalOwner",
                3,
                4,
                0,
                &rules,
                &Default::default(),
            )
            .unwrap();
        let bio_id = sim
            .spawn_object(
                "BIO",
                "ControllerOwner",
                10,
                4,
                0,
                &rules,
                &Default::default(),
            )
            .unwrap();
        sim.substrate.entities.get_mut(controller_id).unwrap().capture_manager = Some(
            crate::sim::capture_manager::CaptureManagerState {
                max_control: 1,
                infinite_mind_control: false,
                controlled_nodes: Vec::new(),
            },
        );
        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
        assert!(crate::sim::capture_manager::capture_unit(
            &mut sim,
            &rules,
            controller_id,
            victim_id,
            0,
        ));
        // Capture used the Normal action-3 vector above. Before absorber
        // arrival, move the controller into LowMoney so FreeUnit's mandatory
        // release DecideUnitFate selects action 4 (Hunt) and therefore does
        // not redispatch/fail action 3 and clear Foot +0x68F. This isolates
        // the native persistent-intent lifetime from a second selector call.
        sim.houses.get_mut(&controller_owner).unwrap().credits = 0;
        {
            let victim = sim.substrate.entities.get(victim_id).unwrap();
            assert_eq!(victim.mission.current().known(), Some(MissionType::Enter));
            assert_eq!(
                victim.navigation.nav_com,
                Some(NavTargetRef::building(bio_id)),
            );
            assert_eq!(victim.radio_contacts.slot(0), Some(bio_id));
        }

        let mut grid = crate::sim::pathfinding::PathGrid::new(20, 10);
        grid.set_blocked(10, 4, true);
        let mut positions = std::collections::BTreeSet::new();
        positions.insert((3, 4));
        let mut boarded_frame = None;
        for frame in 0..240u32 {
            sim.advance_tick(
                &[],
                Some(&rules),
                &Default::default(),
                Some(&grid),
                None,
                16,
            );
            let victim = sim.substrate.entities.get(victim_id).unwrap();
            positions.insert((victim.position.rx, victim.position.ry));
            if frame == 0 {
                assert_ne!(
                    (victim.position.rx, victim.position.ry),
                    (10, 4),
                    "capture fate never teleports to the absorber",
                );
            }
            if victim.lifecycle.in_limbo {
                boarded_frame = Some(frame);
                break;
            }
        }
        let boarded_frame = boarded_frame.expect("Infantry reaches and enters stock absorber");
        assert!(boarded_frame > 0);
        assert!(positions.len() > 2, "approach crosses intermediate cells");
        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert_eq!(victim.owner, original_owner, "FreeUnit precedes weak 0x15");
        assert_eq!(victim.mind_control_controller_id, None);
        assert!(victim.ai_absorb_enter_pending);
        assert!(victim.infantry_absorber_occupant);
        assert!(!victim.infantry_house_tracked);
        assert_eq!(sim.houses[&original_owner].tracked_infantry_count, 0);
        assert_eq!(victim.passenger_role.inside_transport_id(), Some(bio_id));
        assert_eq!(
            sim.substrate
                .entities
                .get(bio_id)
                .unwrap()
                .passenger_role
                .cargo()
                .unwrap()
                .passengers[0],
            victim_id,
        );
        let _ = crate::sim::power_system::tick_power_states(
            &mut sim.power_states,
            &mut sim.substrate.entities,
            &rules,
            &sim.interner,
        );
        assert_eq!(sim.power_states[&controller_owner].total_output, 250);

        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
        let baseline_hash = sim.state_hash();
        let bytes = crate::sim::snapshot::GameSnapshot::save(
            &sim,
            0,
            0,
            "absorbed-infantry-tracking",
            0,
        );
        let mut restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("v119 absorbed Infantry snapshot")
            .sim;
        restored
            .restore_after_snapshot_load()
            .expect("absorber cargo/tracking reciprocals restore");
        assert_eq!(restored.state_hash(), baseline_hash);
        let restored_victim = restored.substrate.entities.get(victim_id).unwrap();
        assert!(restored_victim.infantry_absorber_occupant);
        assert!(!restored_victim.infantry_house_tracked);
        assert_eq!(restored.houses[&original_owner].tracked_infantry_count, 0);

        let mut changed: Simulation = bincode::deserialize(
            &bincode::serialize(&sim).expect("serialize tracking fixture"),
        )
        .expect("clone tracking fixture");
        changed.houses.get_mut(&original_owner).unwrap().tracked_infantry_count = 1;
        assert_ne!(changed.state_hash(), baseline_hash, "House +0x2F4 hashes");
        let mut changed: Simulation = bincode::deserialize(
            &bincode::serialize(&sim).expect("serialize tracking fixture"),
        )
        .expect("clone tracking fixture");
        changed
            .substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .infantry_absorber_occupant = false;
        assert_ne!(changed.state_hash(), baseline_hash, "Infantry +0x439 hashes");
        let mut corrupt: Simulation = bincode::deserialize(
            &bincode::serialize(&sim).expect("serialize tracking fixture"),
        )
        .expect("clone tracking fixture");
        corrupt
            .substrate
            .entities
            .get_mut(victim_id)
            .unwrap()
            .infantry_house_tracked = true;
        assert_ne!(
            corrupt.state_hash(),
            baseline_hash,
            "Infantry +0x438 hashes independently",
        );
        assert!(matches!(
            corrupt.restore_after_snapshot_load(),
            Err(crate::sim::snapshot::SnapshotRestoreError::InvalidInfantryTrackingState {
                entity_id,
                reason: "absorber occupant still contributes to House tracking",
            }) if entity_id == victim_id
        ));

        let mut corrupt: Simulation = bincode::deserialize(
            &bincode::serialize(&sim).expect("serialize tracking fixture"),
        )
        .expect("clone tracking fixture");
        corrupt
            .substrate
            .entities
            .get_mut(bio_id)
            .unwrap()
            .passenger_role
            .cargo_mut()
            .unwrap()
            .clear_contents();
        let restore_error = corrupt
            .restore_after_snapshot_load()
            .expect_err("missing reciprocal absorber cargo membership must be rejected");
        assert!(
            matches!(
                restore_error,
                crate::sim::snapshot::SnapshotRestoreError::InvalidInfantryTrackingState {
                    entity_id,
                    reason: "absorber occupant lacks reciprocal absorber cargo membership",
                } if entity_id == victim_id
            ),
            "unexpected first reciprocal-cargo rejection: {restore_error:?}",
        );
    }

    #[test]
    fn unit_absorber_repeats_full_0x0f_and_rejection_takes_guard_tail() {
        let (mut sim, rules, victim_id, bio_id) = fixture("UNIT", "UBIO");
        let (old, new) = commit_arrival(&mut sim, victim_id, bio_id, MissionType::Enter);
        // Saturate after selection/approach. Unit repeats capacity through
        // 0x0F; Infantry's weaker 0x15 test above deliberately does not.
        {
            let cargo = sim
                .substrate
                .entities
                .get_mut(bio_id)
                .unwrap()
                .passenger_role
                .cargo_mut()
                .unwrap();
            for id in 100..105 {
                cargo.board_forced(id, 1);
            }
        }
        let mut expected_rng = sim.scenario_rng.clone();
        let _ = expected_rng.next_range_u32(8);
        assert_eq!(
            process_per_cell(&mut sim, victim_id, Some(old), Some(new), &rules, None),
            CaptureFatePerCellResult::UnitAbsorberRejected,
        );
        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert_eq!(victim.mission.current().known(), Some(MissionType::Guard));
        assert!(!victim.lifecycle.in_limbo);
        assert!(!victim.passenger_role.is_inside_transport());
        assert!(
            victim.movement_target.is_some(),
            "the rejected Unit receives the ordinary adjacent move-away"
        );
        assert_eq!(sim.rng_views().scenario, expected_rng.logical_view());
    }

    #[test]
    fn expired_facility_clears_reference_without_hunt_or_absorb_intent_reset() {
        let (mut sim, rules, victim_id, bio_id) = fixture("INF", "BIO");
        assert_eq!(
            crate::sim::radio::transmit_pre_admitted_hello(&mut sim, victim_id, bio_id),
            RadioResponse::Roger,
        );
        let _ = sim.mission_assign_exact(
            victim_id,
            MissionId::from_known(MissionType::Enter),
            sim.session.binary_frame,
        );
        {
            let victim = sim.substrate.entities.get_mut(victim_id).unwrap();
            victim.navigation.nav_com = Some(NavTargetRef::building(bio_id));
            victim.ai_absorb_enter_pending = true;
        }
        sim.uninit_with_rules(bio_id, &rules);
        let victim = sim.substrate.entities.get(victim_id).unwrap();
        assert_eq!(victim.navigation.nav_com, None);
        assert!(!victim.radio_contacts.contains(bio_id));
        assert_eq!(victim.mission.current().known(), Some(MissionType::Enter));
        assert!(victim.ai_absorb_enter_pending);
    }
}
