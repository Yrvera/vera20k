//! Focused GSI-04.18 persisted-shroud and aggregate SpySat contracts.

use super::*;
use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::{BuildingUp, Health};
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::mission::state::MissionTestFixture;
use crate::sim::mission::{MissionDispatchTimer, MissionId, MissionType};
use crate::sim::movement::teleport_movement::{TeleportPhase, TeleportState};

fn spy_sat_rules() -> RuleSet {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n\
         [BuildingTypes]\n0=GASPYSAT\n1=GAGAP\n\
         [GASPYSAT]\nName=Spy Satellite\nSpySat=yes\nPowered=yes\nPower=-100\n\
         Strength=100\nCost=1000\nFoundation=1x1\n\
         [GAGAP]\nName=Gap Generator\nGapGenerator=yes\nGapRadiusInCells=3\n\
         Powered=yes\nPower=-100\nStrength=100\nCost=1000\nFoundation=1x1\n",
    );
    RuleSet::from_ini(&ini).expect("GSI-04.18 rules")
}

fn fixture() -> (Simulation, RuleSet, InternedId) {
    let mut sim = Simulation::with_seed(0x418);
    sim.fog.width = 24;
    sim.fog.height = 24;
    let owner = sim.interner.intern("Americans");
    sim.houses
        .insert(owner, HouseState::new(owner, 0, None, true, 10_000, 10));
    sim.session.house_order.push(owner);
    (sim, spy_sat_rules(), owner)
}

fn insert_structure(
    sim: &mut Simulation,
    stable_id: u64,
    owner: InternedId,
    type_name: &str,
    rx: u16,
) {
    let type_ref = sim.interner.intern(type_name);
    let mut entity = GameEntity::new_at_frame_zero_for_test(
        stable_id,
        rx,
        12,
        0,
        0,
        owner,
        Health {
            current: 100,
            max: 100,
        },
        type_ref,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    entity.lifecycle.in_limbo = false;
    entity.lifecycle.cell_marked = true;
    sim.substrate.entities.insert(entity);
}

fn insert_sight_unit(sim: &mut Simulation, stable_id: u64, owner: InternedId, rx: u16, ry: u16) {
    let type_ref = sim.interner.intern("GSI418SIGHT");
    let mut entity = GameEntity::new_at_frame_zero_for_test(
        stable_id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 100,
            max: 100,
        },
        type_ref,
        EntityCategory::Unit,
        0,
        3,
        false,
    );
    entity.lifecycle.in_limbo = false;
    entity.lifecycle.cell_marked = true;
    sim.substrate.entities.insert(entity);
}

fn set_missions(entity: &mut GameEntity, current: MissionId, queued: MissionId) {
    entity.mission.apply_test_fixture(MissionTestFixture {
        current,
        suspended: MissionId::NONE,
        queued,
        movement_bypass_latch: 0,
        handler_state: 0,
        mission_start_frame: 0,
        ai_counter: 0,
        dispatch_timer: MissionDispatchTimer::at_frame(0),
    });
}

#[test]
fn gsi_04_18_two_uplinks_reshroud_only_after_the_last_provider_is_concealed() {
    let (mut sim, rules, owner) = fixture();
    insert_structure(&mut sim, 1, owner, "GASPYSAT", 6);
    insert_structure(&mut sim, 2, owner, "GASPYSAT", 8);
    let scenario_rng_before = sim.scenario_rng.state();
    let main_rng_before = sim.main_rng.state();

    sim.reconcile_active_vision_structures(&rules);
    assert!(sim.houses[&owner].spy_sat_active);
    assert!(sim.houses[&owner].map_is_clear);
    assert!(sim.fog.is_cell_revealed(owner, 23, 23));

    sim.uninit(1);
    sim.reconcile_active_vision_structures(&rules);
    assert!(sim.houses[&owner].spy_sat_active);
    assert!(sim.houses[&owner].map_is_clear);
    assert!(sim.fog.is_cell_revealed(owner, 23, 23));

    sim.uninit(2);
    sim.reconcile_active_vision_structures(&rules);
    assert!(!sim.houses[&owner].spy_sat_active);
    assert!(!sim.houses[&owner].map_is_clear);
    assert!(!sim.fog.is_cell_revealed(owner, 23, 23));
    assert_eq!(sim.scenario_rng.state(), scenario_rng_before);
    assert_eq!(sim.main_rng.state(), main_rng_before);
}

#[test]
fn gsi_04_18_last_uplink_loss_preserves_surviving_techno_sight() {
    let (mut sim, rules, owner) = fixture();
    insert_structure(&mut sim, 1, owner, "GASPYSAT", 6);
    insert_sight_unit(&mut sim, 2, owner, 4, 4);
    sim.reconcile_active_vision_structures(&rules);
    sim.refresh_fog(None, &vision::VisionConfig::default(), Some(&rules));
    assert!(sim.fog.is_cell_visible(owner, 4, 4));
    assert!(sim.fog.is_cell_revealed(owner, 23, 23));
    assert!(!sim.fog.is_cell_visible(owner, 23, 23));
    let scenario_rng_before = sim.scenario_rng.state();
    let main_rng_before = sim.main_rng.state();

    sim.uninit(1);
    sim.reconcile_active_vision_structures(&rules);

    assert!(!sim.houses[&owner].spy_sat_active);
    assert!(!sim.houses[&owner].map_is_clear);
    assert!(!sim.fog.is_cell_revealed(owner, 23, 23));
    assert!(sim.fog.is_cell_visible(owner, 4, 4));
    assert!(sim.fog.is_cell_revealed(owner, 4, 4));
    assert_eq!(sim.scenario_rng.state(), scenario_rng_before);
    assert_eq!(sim.main_rng.state(), main_rng_before);
}

#[test]
fn gsi_04_18_spy_sat_candidate_is_independent_of_low_or_offline_power() {
    let (mut sim, rules, owner) = fixture();
    insert_structure(&mut sim, 1, owner, "GASPYSAT", 6);
    let power = sim.power_states.entry(owner).or_default();
    power.is_low_power = true;
    power.power_blackout_remaining = 30;

    sim.reconcile_active_vision_structures(&rules);

    assert!(sim.houses[&owner].spy_sat_active);
    assert!(sim.houses[&owner].map_is_clear);
    assert!(sim.fog.is_cell_revealed(owner, 23, 23));
}

#[test]
fn gsi_04_18_sale_waits_for_the_next_house_rung_before_reshrouding() {
    let (mut sim, rules, owner) = fixture();
    insert_structure(&mut sim, 1, owner, "GASPYSAT", 6);
    sim.reconcile_active_vision_structures(&rules);
    assert!(sim.fog.is_cell_revealed(owner, 23, 23));

    assert!(crate::sim::production::sell_building(&mut sim, &rules, 1));
    assert!(sim.houses[&owner].spy_sat_active);
    assert!(sim.houses[&owner].map_is_clear);
    assert!(
        sim.fog.is_cell_revealed(owner, 23, 23),
        "EventClass sale must not bypass the earlier House-rung aggregate scan"
    );

    sim.reconcile_active_vision_structures(&rules);
    assert!(!sim.houses[&owner].spy_sat_active);
    assert!(!sim.houses[&owner].map_is_clear);
    assert!(!sim.fog.is_cell_revealed(owner, 23, 23));
}

#[test]
fn gsi_04_18_first_warping_candidate_blocks_later_uplink_but_selling_is_skipped() {
    let (mut sim, rules, owner) = fixture();
    insert_structure(&mut sim, 1, owner, "GASPYSAT", 6);
    insert_structure(&mut sim, 2, owner, "GASPYSAT", 8);
    sim.substrate.entities.get_mut(2).unwrap().building_up = Some(BuildingUp {
        elapsed_ticks: 0,
        total_ticks: 20,
    });

    sim.reconcile_active_vision_structures(&rules);
    assert!(sim.houses[&owner].spy_sat_active);
    assert!(sim.houses[&owner].map_is_clear);

    sim.substrate.entities.get_mut(1).unwrap().teleport_state = Some(TeleportState {
        phase: TeleportPhase::Relocate,
        target_rx: 10,
        target_ry: 10,
        being_warped_ticks: 0,
    });

    sim.reconcile_active_vision_structures(&rules);
    assert!(!sim.houses[&owner].spy_sat_active);
    assert!(!sim.houses[&owner].map_is_clear);

    let selling = MissionId::from_known(MissionType::Selling);
    set_missions(
        sim.substrate.entities.get_mut(1).unwrap(),
        MissionId::NONE,
        selling,
    );
    sim.reconcile_active_vision_structures(&rules);
    assert!(
        sim.houses[&owner].spy_sat_active,
        "queued Selling skips the first uplink, and BuildingUp does not exclude the second"
    );

    set_missions(
        sim.substrate.entities.get_mut(1).unwrap(),
        selling,
        MissionId::NONE,
    );
    sim.reconcile_active_vision_structures(&rules);
    assert!(
        sim.houses[&owner].spy_sat_active,
        "current Selling also skips to the later eligible uplink"
    );
}

#[test]
fn gsi_04_18_house_rung_applies_spy_sat_before_gap_and_recovers_after_gap_conceal() {
    let (mut sim, rules, owner) = fixture();
    let gapper = sim.interner.intern("Soviet");
    insert_structure(&mut sim, 1, owner, "GASPYSAT", 6);
    insert_structure(&mut sim, 2, gapper, "GAGAP", 12);

    sim.reconcile_active_vision_structures(&rules);
    assert!(sim.fog.is_cell_revealed(owner, 23, 23));
    assert!(!sim.fog.is_cell_revealed(owner, 12, 12));
    assert!(sim.fog.is_cell_gap_covered(owner, 12, 12));

    sim.uninit(2);
    sim.fog.mark_visible_for_owner(owner, 3, 3);
    sim.reconcile_active_vision_structures(&rules);
    assert!(sim.houses[&owner].spy_sat_active);
    assert!(sim.fog.is_cell_revealed(owner, 12, 12));
    assert!(!sim.fog.is_cell_gap_covered(owner, 12, 12));
    assert!(
        sim.fog.is_cell_visible(owner, 3, 3),
        "House-rung Gap replacement must preserve Phase-3 line of sight"
    );
}

#[test]
fn gsi_04_18_spy_sat_latch_and_persisted_fog_are_hash_authority() {
    let (mut sim, _rules, owner) = fixture();
    let baseline = sim.state_hash();
    sim.houses.get_mut(&owner).unwrap().spy_sat_active = true;
    let latch_hash = sim.state_hash();
    assert_ne!(latch_hash, baseline);
    sim.houses.get_mut(&owner).unwrap().spy_sat_active = false;
    assert_eq!(sim.state_hash(), baseline);

    sim.fog.reveal_all_for_owner(owner);
    let revealed_hash = sim.state_hash();
    let gapper = sim.interner.intern("Soviet");
    crate::sim::vision::apply_gap_generators(&mut sim.fog, &[(gapper, 12, 12, 3)], &sim.interner);
    assert_ne!(sim.state_hash(), revealed_hash);
}
