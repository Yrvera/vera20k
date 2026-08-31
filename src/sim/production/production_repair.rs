//! Exact BuildingClass click-repair pulse and same-building AI sell admission.
//!
//! Retail provenance: `BuildingClass::UpdateRepairAndPower @ 0x00450630`.
//! The click-repair tail is global-frame phased, runs in the Building's live
//! LogicVector visit, flips `Building+0x6DE` before affordability, and charges
//! one complete `RepairStep` even when the final heal clamps.

use crate::map::entities::EntityCategory;
use crate::rules::object_type::FactoryType;
use crate::rules::ruleset::RuleSet;
use crate::sim::mission::authority::EntityReadyInputProvider;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::world::Simulation;
use crate::util::native_x87::{NativeF64Bits, X87Chop53};

const REPAIR_RATE_SCALE: NativeF64Bits = NativeF64Bits::from_bits(900.0_f64.to_bits());

/// Native `Math__ftol(Rules.RepairRate * 900.0)` under the retail chop
/// control word. Zero is deliberately an error: gamemd reaches signed `IDIV 0`
/// for malformed data instead of silently clamping the interval to one.
pub(crate) fn building_repair_interval(rate: NativeF64Bits) -> Result<i32, &'static str> {
    let rate = X87Chop53::load_f64(rate).map_err(|_| "non-finite RepairRate")?;
    let scale = X87Chop53::load_f64(REPAIR_RATE_SCALE).expect("900.0 is finite");
    let interval = X87Chop53::ftol_i64(X87Chop53::mul(rate, scale))
        .map_err(|_| "RepairRate interval does not fit i64")?;
    if interval == 0 {
        return Err("RepairRate * 900 chops to zero");
    }
    i32::try_from(interval).map_err(|_| "RepairRate interval does not fit signed dword")
}

/// Exact ordinary-Building `BuildingTypeClass` repair-cost virtual:
/// two signed integer divisions, then one f64 percent multiply/chop, then
/// minimum one. Bridge and installed-addon specializations are stock-excluded.
pub(crate) fn building_repair_cost(
    effective_cost: i32,
    strength: i32,
    repair_step: i32,
    repair_percent: NativeF64Bits,
) -> Result<i32, &'static str> {
    let denominator = strength
        .checked_div(repair_step)
        .ok_or("RepairStep is zero or signed division overflowed")?;
    let integer_base = effective_cost
        .checked_div(denominator)
        .ok_or("Strength / RepairStep is zero or signed division overflowed")?;
    let percent = X87Chop53::load_f64(repair_percent).map_err(|_| "non-finite RepairPercent")?;
    let scaled = X87Chop53::mul(X87Chop53::load_i32(integer_base), percent);
    let chopped = X87Chop53::ftol_i64(scaled).map_err(|_| "repair cost does not fit i64")?;
    Ok(i32::try_from(chopped)
        .map_err(|_| "repair cost does not fit i32")?
        .max(1))
}

/// Evaluate the exact low-credit decision in this Building's own live slot.
///
/// Retail provenance: the admission prefix in
/// `BuildingClass::UpdateRepairAndPower @ 0x00450630`, followed by
/// `BuildingClass::StartSelling @ 0x00447110`. Gate order is observable because
/// only the suffix consumes Scenario RNG. A successful call queues Selling with
/// `commence_now=0` and then invokes Commence directly, leaving teardown,
/// refund, and eventual removal to the Selling mission owner.
pub(crate) fn tick_ai_low_credit_sell_start(
    sim: &mut Simulation,
    rules: &RuleSet,
    stable_id: u64,
) -> bool {
    let Some((owner, type_ref, initial_mission)) =
        sim.substrate.entities.get(stable_id).and_then(|entity| {
            (entity.category == EntityCategory::Structure
                && entity.is_active()
                && !entity.lifecycle.in_limbo
                && !entity.dying)
                .then_some((
                    entity.owner,
                    entity.type_ref,
                    entity.mission.effective().known(),
                ))
        })
    else {
        return false;
    };

    // 1. CurrentIQ is a signed House field and is distinct from scenario IQ.
    let Some(house) = sim.houses.get(&owner) else {
        return false;
    };
    if house.current_iq < rules.general.iq_repair_sell {
        return false;
    }
    // 2-3. These mission gates precede every type/credit/RNG read.
    if matches!(
        initial_mission,
        Some(MissionType::Construction | MissionType::Selling)
    ) {
        return false;
    }

    // 4. Building vtable +0x94: ClickRepairable, ordinary repairable body,
    // non-1x1-undeployer, nonzero health, and health not equal to Strength.
    let type_name = sim.interner.resolve(type_ref);
    let Some(object) = rules.object(type_name) else {
        return false;
    };
    let virtual_admits = sim.substrate.entities.get(stable_id).is_some_and(|entity| {
        entity.health.current != 0
            && object.click_repairable
            && object.repairable
            && !object.is_1x1_with_undeploy()
            && i32::from(entity.health.current) != object.strength
    });
    if !virtual_admits {
        return false;
    }

    // 5. Signed wallet comparison.
    if house.credits >= rules.general.credit_reserve {
        return false;
    }
    // 6-7. Nonzero game modes bypass Building+0x6DC; WasAttacked remains an
    // independent persistent Techno latch.
    let Some(entity) = sim.substrate.entities.get(stable_id) else {
        return false;
    };
    if !sim.session.game_mode_nonzero && !entity.building_ai_sell_enabled {
        return false;
    }
    if !entity.was_attacked_by_enemy {
        return false;
    }
    // 8. Native compares both dwords as unsigned here.
    if (house.scenario_iq as u32) < (rules.general.iq_sell_back as u32) {
        return false;
    }

    // 9-10. RandomRanged(0, 0x32) is inclusive, and the TechLevel compare is
    // unsigned. Every later rejection has already consumed this one draw.
    let roll = sim.scenario_rng.next_range_u32_inclusive(0, 0x32);
    if roll >= house.tech_level as u32 {
        return false;
    }

    // 11-13. Map tag, raw Factory enum 7 (BuildingType), then strict red HP.
    let Some(entity) = sim.substrate.entities.get(stable_id) else {
        return false;
    };
    if entity.attached_trigger_tag.is_some() {
        return false;
    }
    if object.factory == Some(FactoryType::BuildingType) {
        return false;
    }
    if i64::from(entity.health.current) * 1000
        >= i64::from(object.strength) * rules.general.condition_red_x1000
    {
        return false;
    }

    // 14. StartSelling's own late gates: a live build-up SHP manager, mission
    // still not Selling, and no shared C4/PostMortem (+0x6DF) latch.
    if !entity.building_make_shape_initialized
        || entity.mission.effective().known() == Some(MissionType::Selling)
        || entity.pending_c4_detonation.is_some()
    {
        return false;
    }

    let now = sim.session.binary_frame;
    let selling = MissionId::from_known(MissionType::Selling);
    if sim
        .mission_queue_exact(stable_id, selling, 0, now, &EntityReadyInputProvider)
        .is_err()
    {
        return false;
    }
    sim.mission_commence_exact(stable_id, now).unwrap_or(false)
}

/// Run one Building's click-repair tail in its live object visit.
///
/// Caller-owned gates are limited to a present, active, nonzero-HP Structure.
/// Inside the native tail there is deliberately no full-health, power,
/// operational, or mission gate.
pub(crate) fn tick_building_repair_tail(sim: &mut Simulation, rules: &RuleSet, stable_id: u64) {
    let repairing = sim
        .substrate
        .entities
        .get(stable_id)
        .is_some_and(|entity| entity.repairing);
    if !repairing {
        return;
    }

    let interval = building_repair_interval(rules.general.building_repair_rate)
        .expect("active RepairRate must produce a nonzero signed divisor");
    // Main::Frame is a signed 32-bit counter at this callsite. Preserve its
    // signed wrap domain before native IDIV-style remainder testing.
    let frame = sim.session.binary_frame as i32;
    if frame % interval != 0 {
        return;
    }

    // +0x6DE flips before type lookup, affordability, or any other pulse work.
    let Some((owner, type_ref)) = sim.substrate.entities.get_mut(stable_id).map(|entity| {
        entity.repair_pulse_latch = !entity.repair_pulse_latch;
        (entity.owner, entity.type_ref)
    }) else {
        return;
    };

    let type_name = sim.interner.resolve(type_ref);
    let object = rules
        .object(type_name)
        .expect("live Building repair requires its BuildingType rules");
    let repair_step = rules.general.building_repair_step;
    let strength = object.strength;
    let pulse_cost = building_repair_cost(
        object.cost,
        strength,
        repair_step,
        rules.general.building_repair_percent,
    )
    .expect("active Building repair divisors and cost must be valid");

    let affordable = sim
        .houses
        .get(&owner)
        .is_some_and(|house| house.credits >= pulse_cost);
    if !affordable {
        if let Some(entity) = sim.substrate.entities.get_mut(stable_id) {
            entity.repairing = false;
        }
        return;
    }

    sim.houses
        .get_mut(&owner)
        .expect("affordable Building repair owner remained present")
        .credits -= pulse_cost;

    let damage_state_changed = {
        let entity = sim
            .substrate
            .entities
            .get_mut(stable_id)
            .expect("repairing Building remained present through debit");
        let repaired = i32::from(entity.health.current).saturating_add(repair_step);
        let clamped = repaired.min(strength).clamp(0, i32::from(u16::MAX)) as u16;
        entity.health.current = clamped;
        if repaired >= strength {
            entity.health.current = strength.clamp(0, i32::from(u16::MAX)) as u16;
            entity.repairing = false;
        }
        let changed =
            entity.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
        changed
    };

    if damage_state_changed {
        crate::sim::world::building_anim::recreate_existing_slots_for_damage_state(
            sim, rules, stable_id,
        );
    }
    sim.stop_damage_smoke_if_above_yellow(stable_id, rules);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::art_data::ArtRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::command::{Command, CommandEnvelope};
    use crate::sim::components::{
        AnimOverlayState, BuildingAnimOverlays, Health, PendingC4Detonation,
    };
    use crate::sim::game_entity::GameEntity;
    use crate::sim::house_state::HouseState;
    use crate::sim::mission::{MissionId, MissionType};
    use crate::sim::particles::ParticleSystem;
    use crate::sim::projectile::{
        ProjectileCollisionPolicy, ProjectileCoord, ProjectilePayload, ProjectileSpawn,
        ProjectileTarget, ProjectileTrajectory, ProjectileVelocity, ProjectileVisualState,
        TargetExpiryPolicy,
    };
    use crate::sim::rng::SimRng;
    use crate::util::fixed_math::SimFixed;

    fn repair_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\n\
             RepairRate=.016\nRepairStep=8\nRepairPercent=15%\n\
             [AI]\nCreditReserve=100\n\
             [IQ]\nRepairSell=2\nSellBack=2\n\
             [AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n\
             [BuildingTypes]\n0=GAPOWR\n1=GACNST\n2=GAWEAP\n3=YAPOWR\n4=NACNST\n\
             [GAPOWR]\nStrength=100\nCost=800\nArmor=wood\nPower=-10\nPowered=yes\nClickRepairable=yes\nRepairable=yes\n\
             [GACNST]\nStrength=100\nCost=3000\nArmor=concrete\nClickRepairable=yes\nRepairable=yes\nFactory=BuildingType\n\
             [GAWEAP]\nStrength=100\nCost=2000\nArmor=wood\nClickRepairable=no\nRepairable=yes\n\
             [YAPOWR]\nStrength=100\nCost=600\nArmor=wood\nClickRepairable=yes\nRepairable=no\n\
             [NACNST]\nStrength=100\nCost=3000\nArmor=concrete\nClickRepairable=yes\nRepairable=yes\nUndeploysInto=AMCV\n\
             [Warheads]\n0=KILLWH\n\
             [KILLWH]\nCellSpread=0\nPercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("building repair rules")
    }

    fn insert_repairing_building(
        sim: &mut Simulation,
        stable_id: u64,
        owner: crate::sim::intern::InternedId,
        hp: u16,
    ) {
        let type_ref = sim.interner.intern("GAPOWR");
        let mut entity = GameEntity::new_at_frame_zero_for_test(
            stable_id,
            stable_id as u16 + 2,
            4,
            0,
            0,
            owner,
            Health {
                current: hp,
                max: 100,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            5,
            false,
        );
        entity.lifecycle.in_limbo = false;
        entity.repairing = true;
        entity.building_damage_state_active = hp <= 50;
        sim.substrate.entities.insert(entity);
    }

    fn repair_fixture(
        hp: u16,
        credits: i32,
    ) -> (Simulation, RuleSet, u64, crate::sim::intern::InternedId) {
        let rules = repair_rules();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, true, credits, 10));
        insert_repairing_building(&mut sim, 1, owner, hp);
        (sim, rules, 1, owner)
    }

    fn insert_ai_sell_building(
        sim: &mut Simulation,
        stable_id: u64,
        owner: crate::sim::intern::InternedId,
        type_id: &str,
        hp: u16,
    ) {
        let type_ref = sim.interner.intern(type_id);
        let mut entity = GameEntity::new_at_frame_zero_for_test(
            stable_id,
            stable_id as u16 + 2,
            4,
            0,
            0,
            owner,
            Health {
                current: hp,
                max: 100,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            5,
            false,
        );
        entity.lifecycle.in_limbo = false;
        entity.was_attacked_by_enemy = true;
        entity.building_ai_sell_enabled = true;
        entity.building_make_shape_initialized = true;
        // Keep unrelated damaged-building effect admission from consuming RNG
        // when a fixture runs through the complete live object visit.
        entity.damage_fire_state_active = true;
        sim.substrate.entities.insert(entity);
    }

    fn ai_sell_fixture(
        type_id: &str,
        hp: u16,
    ) -> (Simulation, RuleSet, u64, crate::sim::intern::InternedId) {
        let rules = repair_rules();
        let mut sim = Simulation::new();
        sim.scenario_rng = SimRng::new(0x39_08);
        let owner = sim.interner.intern("AI");
        let mut house = HouseState::new(owner, 0, None, false, 99, 51);
        house.current_iq = 2;
        house.scenario_iq = 2;
        sim.houses.insert(owner, house);
        insert_ai_sell_building(&mut sim, 1, owner, type_id, hp);
        (sim, rules, 1, owner)
    }

    #[test]
    fn ai_sell_early_gate_matrix_consumes_no_rng() {
        #[derive(Clone, Copy, Debug)]
        enum Case {
            CurrentIq,
            Construction,
            Selling,
            QueuedConstruction,
            QueuedSelling,
            ClickRepairable,
            Repairable,
            OneByOneUndeployer,
            ZeroHealth,
            FullStrength,
            CreditReserve,
            ScenarioModeByte,
            WasAttacked,
            ScenarioIqUnsigned,
        }

        for case in [
            Case::CurrentIq,
            Case::Construction,
            Case::Selling,
            Case::QueuedConstruction,
            Case::QueuedSelling,
            Case::ClickRepairable,
            Case::Repairable,
            Case::OneByOneUndeployer,
            Case::ZeroHealth,
            Case::FullStrength,
            Case::CreditReserve,
            Case::ScenarioModeByte,
            Case::WasAttacked,
            Case::ScenarioIqUnsigned,
        ] {
            let (mut sim, rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
            match case {
                Case::CurrentIq => sim.houses.get_mut(&owner).unwrap().current_iq = 1,
                Case::Construction => {
                    sim.mission_assign_exact(
                        id,
                        MissionId::from_known(MissionType::Construction),
                        0,
                    )
                    .unwrap();
                }
                Case::Selling => {
                    sim.mission_assign_exact(id, MissionId::from_known(MissionType::Selling), 0)
                        .unwrap();
                }
                Case::QueuedConstruction => {
                    sim.mission_queue_exact(
                        id,
                        MissionId::from_known(MissionType::Construction),
                        0,
                        0,
                        &EntityReadyInputProvider,
                    )
                    .unwrap();
                }
                Case::QueuedSelling => {
                    sim.mission_queue_exact(
                        id,
                        MissionId::from_known(MissionType::Selling),
                        0,
                        0,
                        &EntityReadyInputProvider,
                    )
                    .unwrap();
                }
                Case::ClickRepairable => {
                    sim.substrate.entities.get_mut(id).unwrap().type_ref =
                        sim.interner.intern("GAWEAP");
                }
                Case::Repairable => {
                    sim.substrate.entities.get_mut(id).unwrap().type_ref =
                        sim.interner.intern("YAPOWR");
                }
                Case::OneByOneUndeployer => {
                    sim.substrate.entities.get_mut(id).unwrap().type_ref =
                        sim.interner.intern("NACNST");
                }
                Case::ZeroHealth => sim.substrate.entities.get_mut(id).unwrap().health.current = 0,
                Case::FullStrength => {
                    sim.substrate.entities.get_mut(id).unwrap().health.current = 100;
                }
                Case::CreditReserve => sim.houses.get_mut(&owner).unwrap().credits = 100,
                Case::ScenarioModeByte => {
                    sim.substrate
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .building_ai_sell_enabled = false;
                }
                Case::WasAttacked => {
                    sim.substrate
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .was_attacked_by_enemy = false;
                }
                Case::ScenarioIqUnsigned => sim.houses.get_mut(&owner).unwrap().scenario_iq = 1,
            }

            let before_rng = sim.scenario_rng.logical_state();
            let before_entity = {
                let entity = sim.substrate.entities.get(id).unwrap();
                (
                    entity.mission,
                    entity.health.current,
                    entity.lifecycle.object_alive,
                    entity.lifecycle.in_limbo,
                )
            };
            let before_credits = sim.houses[&owner].credits;

            assert!(
                !tick_ai_low_credit_sell_start(&mut sim, &rules, id),
                "{case:?}"
            );
            assert_eq!(sim.scenario_rng.logical_state(), before_rng, "{case:?}");
            let entity = sim.substrate.entities.get(id).unwrap();
            assert_eq!(
                (
                    entity.mission,
                    entity.health.current,
                    entity.lifecycle.object_alive,
                    entity.lifecycle.in_limbo,
                ),
                before_entity,
                "{case:?}"
            );
            assert_eq!(sim.houses[&owner].credits, before_credits, "{case:?}");
        }
    }

    #[test]
    fn ai_sell_late_gate_matrix_consumes_exactly_one_ranged_draw() {
        #[derive(Clone, Copy, Debug)]
        enum Case {
            TechLevel,
            AttachedTag,
            BuildingFactory,
            RedEquality,
            MissingMakeShape,
            PendingC4,
        }

        for case in [
            Case::TechLevel,
            Case::AttachedTag,
            Case::BuildingFactory,
            Case::RedEquality,
            Case::MissingMakeShape,
            Case::PendingC4,
        ] {
            let (mut sim, rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
            match case {
                Case::TechLevel => sim.houses.get_mut(&owner).unwrap().tech_level = 0,
                Case::AttachedTag => {
                    let tag = sim.interner.intern("TAG_BLOCK");
                    sim.substrate
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .attached_trigger_tag = Some(tag);
                }
                Case::BuildingFactory => {
                    sim.substrate.entities.get_mut(id).unwrap().type_ref =
                        sim.interner.intern("GACNST");
                }
                Case::RedEquality => {
                    sim.substrate.entities.get_mut(id).unwrap().health.current = 25;
                }
                Case::MissingMakeShape => {
                    sim.substrate
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .building_make_shape_initialized = false;
                }
                Case::PendingC4 => {
                    sim.substrate
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .pending_c4_detonation = Some(PendingC4Detonation {
                        start_frame: 0,
                        duration_frames: 10,
                        source_entity_id: None,
                    });
                }
            }

            let before_mission = sim.substrate.entities.get(id).unwrap().mission;
            let mut expected_rng = sim.scenario_rng.clone();
            expected_rng.next_range_u32_inclusive(0, 0x32);
            assert!(
                !tick_ai_low_credit_sell_start(&mut sim, &rules, id),
                "{case:?}"
            );
            assert_eq!(
                sim.scenario_rng.logical_state(),
                expected_rng.logical_state(),
                "{case:?}"
            );
            assert_eq!(
                sim.substrate.entities.get(id).unwrap().mission,
                before_mission,
                "{case:?}"
            );
        }

        let (mut sim, rules, id, _) = ai_sell_fixture("GAPOWR", 24);
        sim.substrate
            .entities
            .get_mut(id)
            .unwrap()
            .building_ai_sell_enabled = false;
        let before_rng = sim.scenario_rng.logical_state();
        assert!(!tick_ai_low_credit_sell_start(&mut sim, &rules, id));
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);

        sim.session.game_mode_nonzero = true;
        let mut expected_rng = sim.scenario_rng.clone();
        expected_rng.next_range_u32_inclusive(0, 0x32);
        assert!(tick_ai_low_credit_sell_start(&mut sim, &rules, id));
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    #[test]
    fn ai_sell_signed_and_unsigned_boundaries_match_native() {
        let (mut sim, mut rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
        rules.general.iq_repair_sell = 0;
        sim.houses.get_mut(&owner).unwrap().current_iq = -1;
        let before_rng = sim.scenario_rng.logical_state();
        assert!(!tick_ai_low_credit_sell_start(&mut sim, &rules, id));
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);

        let (mut sim, rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
        {
            let house = sim.houses.get_mut(&owner).unwrap();
            house.scenario_iq = -1;
            house.tech_level = -1;
        }
        assert!(tick_ai_low_credit_sell_start(&mut sim, &rules, id));

        let (mut sim, mut rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
        rules.general.iq_sell_back = -1;
        sim.houses.get_mut(&owner).unwrap().scenario_iq = 1;
        let before_rng = sim.scenario_rng.logical_state();
        assert!(!tick_ai_low_credit_sell_start(&mut sim, &rules, id));
        assert_eq!(sim.scenario_rng.logical_state(), before_rng);

        let (mut probe, rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
        let mut expected_rng = probe.scenario_rng.clone();
        let roll = expected_rng.next_range_u32_inclusive(0, 0x32);
        probe.houses.get_mut(&owner).unwrap().tech_level = roll as i32;
        assert!(!tick_ai_low_credit_sell_start(&mut probe, &rules, id));
        assert_eq!(
            probe.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );

        let (mut pass, rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
        pass.houses.get_mut(&owner).unwrap().tech_level = roll as i32 + 1;
        assert!(tick_ai_low_credit_sell_start(&mut pass, &rules, id));
    }

    #[test]
    fn ai_sell_queues_zero_then_directly_commences_selling() {
        let (mut sim, rules, id, owner) = ai_sell_fixture("GAPOWR", 24);
        sim.session.binary_frame = 37;
        assert_eq!(
            sim.substrate
                .entities
                .get(id)
                .unwrap()
                .mission_leaf
                .as_building()
                .unwrap()
                .ready_latch(),
            0
        );
        let hp = sim.substrate.entities.get(id).unwrap().health.current;
        let credits = sim.houses[&owner].credits;
        let mut expected_rng = sim.scenario_rng.clone();
        expected_rng.next_range_u32_inclusive(0, 0x32);

        assert!(tick_ai_low_credit_sell_start(&mut sim, &rules, id));
        let entity = sim.substrate.entities.get(id).unwrap();
        assert_eq!(
            entity.mission.current(),
            MissionId::from_known(MissionType::Selling)
        );
        assert_eq!(entity.mission.queued(), MissionId::NONE);
        assert_eq!(entity.mission.mission_start_frame(), 37);
        assert_eq!(entity.mission.handler_state(), 0);
        assert_eq!(entity.mission.ai_counter(), 0);
        assert!(entity.is_active() && !entity.lifecycle.in_limbo);
        assert_eq!(entity.health.current, hp);
        assert_eq!(sim.houses[&owner].credits, credits);
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );

        let after_first = sim.scenario_rng.logical_state();
        assert!(!tick_ai_low_credit_sell_start(&mut sim, &rules, id));
        assert_eq!(sim.scenario_rng.logical_state(), after_first);
    }

    #[test]
    fn ai_sell_start_precedes_repair_tail_in_same_live_visit() {
        let (mut sim, rules, id, owner) = ai_sell_fixture("GAPOWR", 20);
        sim.houses.get_mut(&owner).unwrap().credits = 9;
        {
            let entity = sim.substrate.entities.get_mut(id).unwrap();
            entity.repairing = true;
            entity.building_damage_state_active = true;
        }
        sim.set_logic_order_for_test(vec![id]);
        sim.object_ai_stage(Some(&rules));

        let entity = sim.substrate.entities.get(id).unwrap();
        assert_eq!(
            entity.mission.current(),
            MissionId::from_known(MissionType::Selling)
        );
        assert_eq!(entity.mission.queued(), MissionId::NONE);
        assert_eq!(entity.health.current, 28);
        assert_eq!(sim.houses[&owner].credits, 0);
        assert!(entity.repair_pulse_latch);
        assert!(entity.repairing);
        assert!(entity.is_active() && !entity.lifecycle.in_limbo);
    }

    #[test]
    fn two_ai_buildings_start_selling_without_same_pass_sale_or_refund() {
        let (mut sim, rules, first, owner) = ai_sell_fixture("GAPOWR", 24);
        insert_ai_sell_building(&mut sim, 2, owner, "GAPOWR", 24);
        sim.set_logic_order_for_test(vec![first, 2]);
        let before_credits = sim.houses[&owner].credits;
        let mut expected_rng = sim.scenario_rng.clone();
        expected_rng.next_range_u32_inclusive(0, 0x32);
        expected_rng.next_range_u32_inclusive(0, 0x32);

        sim.object_ai_stage(Some(&rules));

        for id in [first, 2] {
            let entity = sim.substrate.entities.get(id).unwrap();
            assert_eq!(
                entity.mission.current(),
                MissionId::from_known(MissionType::Selling)
            );
            assert_eq!(entity.mission.queued(), MissionId::NONE);
            assert_eq!(entity.mission.handler_state(), 0);
            assert!(entity.is_active() && !entity.lifecycle.in_limbo);
        }
        assert_eq!(sim.houses[&owner].credits, before_credits);
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    #[test]
    fn capture_after_ai_admission_preserves_selling() {
        let (mut sim, rules, id, _) = ai_sell_fixture("GAPOWR", 24);
        sim.session.binary_frame = 37;
        assert!(tick_ai_low_credit_sell_start(&mut sim, &rules, id));
        {
            let entity = sim.substrate.entities.get_mut(id).unwrap();
            entity.repairing = true;
            entity.repair_pulse_latch = true;
        }
        let admitted = sim.substrate.entities.get(id).unwrap().mission;
        let rng_after_admission = sim.scenario_rng.logical_state();
        let new_owner = sim.interner.intern("Russia");
        sim.houses.insert(
            new_owner,
            HouseState::new(new_owner, 1, None, false, 50, 10),
        );

        sim.change_owner(id, new_owner);

        let captured = sim.substrate.entities.get(id).unwrap();
        assert_eq!(captured.owner, new_owner);
        assert_eq!(captured.mission, admitted);
        assert_eq!(
            captured.mission.current(),
            MissionId::from_known(MissionType::Selling)
        );
        assert_eq!(captured.mission.queued(), MissionId::NONE);
        assert!(!captured.repairing);
        assert!(captured.repair_pulse_latch);
        assert!(captured.is_active() && !captured.lifecycle.in_limbo);
        assert_eq!(sim.scenario_rng.logical_state(), rng_after_admission);
    }

    #[test]
    fn repair_interval_uses_native_chop_and_rejects_zero_divisor() {
        assert_eq!(
            building_repair_interval(NativeF64Bits::from_bits(0.016_f64.to_bits())),
            Ok(14)
        );
        assert_eq!(
            building_repair_interval(NativeF64Bits::from_bits(0.0166_f64.to_bits())),
            Ok(14),
            "14.94 chops instead of rounding to 15"
        );
        assert!(building_repair_interval(NativeF64Bits::from_bits(0.0001_f64.to_bits())).is_err());
    }

    #[test]
    fn repair_cost_runs_two_integer_divisions_then_percent_chop_and_minimum() {
        let percent = NativeF64Bits::from_bits(0.15_f64.to_bits());
        assert_eq!(building_repair_cost(800, 750, 8, percent), Ok(1));
        assert_eq!(building_repair_cost(3000, 1000, 8, percent), Ok(3));
        assert_eq!(building_repair_cost(2000, 1000, 8, percent), Ok(2));
        assert_eq!(building_repair_cost(600, 700, 8, percent), Ok(1));
        assert_eq!(
            building_repair_cost(800, 100, 8, percent),
            Ok(9),
            "all03umd GAPOWR type override"
        );
        assert!(building_repair_cost(800, 100, 0, percent).is_err());
        assert!(building_repair_cost(800, 1, 8, percent).is_err());
    }

    #[test]
    fn insufficient_and_exact_funds_follow_latch_and_full_step_rules() {
        let (mut poor, rules, id, owner) = repair_fixture(49, 8);
        tick_building_repair_tail(&mut poor, &rules, id);
        let building = poor.substrate.entities.get(id).unwrap();
        assert_eq!(building.health.current, 49);
        assert!(!building.repairing);
        assert!(building.repair_pulse_latch);
        assert_eq!(poor.houses[&owner].credits, 8);

        let (mut exact, rules, id, owner) = repair_fixture(49, 9);
        tick_building_repair_tail(&mut exact, &rules, id);
        let building = exact.substrate.entities.get(id).unwrap();
        assert_eq!(building.health.current, 57);
        assert!(building.repairing);
        assert!(building.repair_pulse_latch);
        assert_eq!(exact.houses[&owner].credits, 0);
    }

    #[test]
    fn final_partial_and_forged_full_pulses_charge_the_complete_cost() {
        for start_hp in [98, 100] {
            let (mut sim, rules, id, owner) = repair_fixture(start_hp, 9);
            tick_building_repair_tail(&mut sim, &rules, id);
            let building = sim.substrate.entities.get(id).unwrap();
            assert_eq!(building.health.current, 100);
            assert!(!building.repairing);
            assert_eq!(sim.houses[&owner].credits, 0);
        }
    }

    #[test]
    fn global_phase_and_live_logic_order_control_shared_wallet() {
        let rules = repair_rules();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        sim.houses
            .insert(owner, HouseState::new(owner, 0, None, true, 9, 10));
        insert_repairing_building(&mut sim, 1, owner, 49);
        insert_repairing_building(&mut sim, 2, owner, 49);
        sim.set_logic_order_for_test(vec![2, 1]);

        sim.session.binary_frame = 13;
        sim.object_ai_stage(Some(&rules));
        assert_eq!(sim.substrate.entities.get(1).unwrap().health.current, 49);
        assert_eq!(sim.substrate.entities.get(2).unwrap().health.current, 49);

        sim.session.binary_frame = 14;
        sim.object_ai_stage(Some(&rules));
        assert_eq!(sim.substrate.entities.get(2).unwrap().health.current, 57);
        assert_eq!(sim.substrate.entities.get(1).unwrap().health.current, 49);
        assert!(!sim.substrate.entities.get(1).unwrap().repairing);
        assert_eq!(sim.houses[&owner].credits, 0);
    }

    #[test]
    fn toggle_and_change_owner_preserve_the_private_latch_contract() {
        let (mut sim, rules, id, _owner) = repair_fixture(49, 18);
        {
            let building = sim.substrate.entities.get_mut(id).unwrap();
            building.repairing = false;
            building.repair_pulse_latch = false;
        }

        assert!(crate::sim::production::toggle_repair(&mut sim, &rules, id));
        assert!(sim.substrate.entities.get(id).unwrap().repair_pulse_latch);
        assert!(crate::sim::production::toggle_repair(&mut sim, &rules, id));
        assert!(!sim.substrate.entities.get(id).unwrap().repairing);
        assert!(sim.substrate.entities.get(id).unwrap().repair_pulse_latch);

        {
            let building = sim.substrate.entities.get_mut(id).unwrap();
            building.health.current = 100;
            building.repair_pulse_latch = false;
        }
        assert!(crate::sim::production::toggle_repair(&mut sim, &rules, id));
        assert!(!sim.substrate.entities.get(id).unwrap().repair_pulse_latch);
        assert!(crate::sim::production::toggle_repair(&mut sim, &rules, id));

        {
            let building = sim.substrate.entities.get_mut(id).unwrap();
            building.health.current = 101;
            building.repair_pulse_latch = false;
        }
        assert!(crate::sim::production::toggle_repair(&mut sim, &rules, id));
        assert!(
            sim.substrate.entities.get(id).unwrap().repair_pulse_latch,
            "native arms the latch for any health not exactly equal to Strength"
        );

        let new_owner = sim.interner.intern("Russia");
        sim.houses.insert(
            new_owner,
            HouseState::new(new_owner, 1, None, false, 50, 10),
        );
        sim.change_owner(id, new_owner);
        let captured = sim.substrate.entities.get(id).unwrap();
        assert_eq!(captured.owner, new_owner);
        assert!(!captured.repairing);
        assert!(captured.repair_pulse_latch);
    }

    #[test]
    fn already_armed_repair_ignores_low_power_and_selling_or_construction_mission() {
        for mission in [MissionType::Selling, MissionType::Construction] {
            let (mut sim, rules, id, owner) = repair_fixture(49, 9);
            sim.power_states.insert(
                owner,
                crate::sim::power_system::PowerState {
                    total_output: 0,
                    total_drain: 10,
                    is_low_power: true,
                    ..Default::default()
                },
            );
            assert!(
                !crate::sim::power_system::is_building_powered(
                    &sim.power_states,
                    &rules,
                    sim.substrate.entities.get(id).unwrap(),
                    &sim.interner,
                ),
                "fixture must actually represent an inoperational low-power consumer"
            );
            sim.mission_assign_exact(id, MissionId::from_known(mission), 0)
                .unwrap();

            tick_building_repair_tail(&mut sim, &rules, id);

            assert_eq!(sim.substrate.entities.get(id).unwrap().health.current, 57);
            assert_eq!(sim.houses[&owner].credits, 0);
        }
    }

    fn attach_damage_smoke(sim: &mut Simulation, building_id: u64, system_id: u64) {
        sim.particle_systems_mut().insert(ParticleSystem {
            stable_id: system_id,
            in_logic_vector: false,
            type_id: crate::rules::particle_system_type::ParticleSystemTypeId(0),
            coords: glam::IVec3::ZERO,
            offset: glam::IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(1),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1d,
            directionless: true,
            attached_entity: None,
            owner_entity: Some(building_id),
            target_coords: glam::IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        });
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .damage_smoke_system_id = Some(system_id);
    }

    #[test]
    fn yellow_equality_stays_damaged_and_only_above_yellow_marks_smoke_done() {
        let (mut equality, rules, id, _) = repair_fixture(42, 9);
        attach_damage_smoke(&mut equality, id, 99);
        tick_building_repair_tail(&mut equality, &rules, id);
        let building = equality.substrate.entities.get(id).unwrap();
        assert_eq!(building.health.current, 50);
        assert!(building.building_damage_state_active);
        assert_eq!(building.building_anim_reset_revision, 0);
        assert!(!equality.particle_systems().get(99).unwrap().done_spawning);

        let (mut above, rules, id, _) = repair_fixture(50, 9);
        attach_damage_smoke(&mut above, id, 99);
        tick_building_repair_tail(&mut above, &rules, id);
        let building = above.substrate.entities.get(id).unwrap();
        assert_eq!(building.health.current, 58);
        assert!(!building.building_damage_state_active);
        assert_eq!(building.building_anim_reset_revision, 1);
        assert_eq!(building.damage_smoke_system_id, Some(99));
        assert!(
            above.particle_systems().get(99).unwrap().done_spawning,
            "ParticleSystemClass +0xF8 is mark-only; the owner pointer survives until finalization"
        );
    }

    #[test]
    fn crossing_recreates_the_existing_slot_and_carries_relative_frame_once() {
        let mut rules = repair_rules();
        rules.merge_art_data(&ArtRegistry::from_ini(&IniFile::from_str(
            "[GAPOWR]\n\
             ActiveAnim=GAPOWR_A\nActiveAnimDamaged=GAPOWR_AD\n\
             [GAPOWR_A]\nStart=2\nLoopStart=1\nLoopEnd=8\nRate=300\n\
             [GAPOWR_AD]\nStart=12\nLoopStart=11\nLoopEnd=18\nRate=150\n",
        )));
        let (mut sim, _, id, _) = repair_fixture(49, 18);
        let damaged = sim.interner.intern("GAPOWR_AD");
        sim.substrate
            .entities
            .get_mut(id)
            .unwrap()
            .building_anim_overlays = Some(BuildingAnimOverlays {
            anims: vec![AnimOverlayState {
                anim_type: damaged,
                frame: 15,
                loop_start: 11,
                loop_end: 18,
                rate_logic_frames: 9,
                elapsed_logic_frames: 7,
                finished: true,
            }],
        });

        tick_building_repair_tail(&mut sim, &rules, id);
        let building = sim.substrate.entities.get(id).unwrap();
        let active = &building.building_anim_overlays.as_ref().unwrap().anims[0];
        assert_eq!(sim.interner.resolve(active.anim_type), "GAPOWR_A");
        // Old absolute 15 at damaged Start=12 carries CurrentFrame=3;
        // healthy Start=2 therefore publishes absolute frame 5.
        assert_eq!(active.frame, 5);
        assert_eq!(active.elapsed_logic_frames, 0);
        assert!(!active.finished);
        assert_eq!(building.building_anim_reset_revision, 1);

        {
            let active = &mut sim
                .substrate
                .entities
                .get_mut(id)
                .unwrap()
                .building_anim_overlays
                .as_mut()
                .unwrap()
                .anims[0];
            active.frame = 4;
            active.elapsed_logic_frames = 7;
        }
        tick_building_repair_tail(&mut sim, &rules, id);
        let building = sim.substrate.entities.get(id).unwrap();
        let active = &building.building_anim_overlays.as_ref().unwrap().anims[0];
        assert_eq!(active.frame, 4);
        assert_eq!(active.elapsed_logic_frames, 7);
        assert_eq!(building.building_anim_reset_revision, 1);
    }

    #[test]
    fn repair_toggle_at_the_event_tail_cannot_take_the_current_frame_pulse() {
        fn run_frame(sim: &mut Simulation, rules: &RuleSet, commands: &[CommandEnvelope]) {
            sim.advance_tick(
                commands,
                Some(rules),
                &std::collections::BTreeMap::new(),
                None,
                None,
                100,
            );
        }

        let (mut on_thirteen, rules, id, owner) = repair_fixture(49, 9);
        on_thirteen
            .substrate
            .entities
            .get_mut(id)
            .unwrap()
            .repairing = false;
        on_thirteen.set_logic_order_for_test(vec![id]);
        on_thirteen.session.binary_frame = 13;
        let toggle = CommandEnvelope::new(owner, 1, Command::ToggleRepair { entity_id: id });
        run_frame(&mut on_thirteen, &rules, &[toggle]);
        assert_eq!(
            on_thirteen
                .substrate
                .entities
                .get(id)
                .unwrap()
                .health
                .current,
            49
        );
        assert_eq!(on_thirteen.session.binary_frame, 14);
        run_frame(&mut on_thirteen, &rules, &[]);
        assert_eq!(
            on_thirteen
                .substrate
                .entities
                .get(id)
                .unwrap()
                .health
                .current,
            57
        );

        let (mut on_fourteen, rules, id, owner) = repair_fixture(49, 9);
        on_fourteen
            .substrate
            .entities
            .get_mut(id)
            .unwrap()
            .repairing = false;
        on_fourteen.set_logic_order_for_test(vec![id]);
        on_fourteen.session.binary_frame = 14;
        let toggle = CommandEnvelope::new(owner, 1, Command::ToggleRepair { entity_id: id });
        run_frame(&mut on_fourteen, &rules, &[toggle]);
        while on_fourteen.session.binary_frame < 28 {
            run_frame(&mut on_fourteen, &rules, &[]);
        }
        assert_eq!(
            on_fourteen
                .substrate
                .entities
                .get(id)
                .unwrap()
                .health
                .current,
            49
        );
        run_frame(&mut on_fourteen, &rules, &[]);
        assert_eq!(
            on_fourteen
                .substrate
                .entities
                .get(id)
                .unwrap()
                .health
                .current,
            57
        );
    }

    #[test]
    fn bullet_and_building_slots_commit_damage_and_repair_in_live_order() {
        fn run(projectile_first: bool) -> (u16, bool, i32) {
            let rules = repair_rules();
            let mut sim = Simulation::new();
            let owner = sim.interner.intern("Americans");
            sim.houses
                .insert(owner, HouseState::new(owner, 0, None, true, 9, 10));
            let building_id = 10;
            insert_repairing_building(&mut sim, building_id, owner, 5);
            let building = sim.substrate.entities.get(building_id).unwrap();
            let impact = ProjectileCoord::new(
                i32::from(building.position.rx) * 256 + 128,
                i32::from(building.position.ry) * 256 + 128,
                0,
            );
            let projectile_id = 20;
            let warhead = sim.interner.intern("KILLWH");
            let weapon = sim.interner.intern("MISSINGWEAPON");
            let projectile_owner = sim.interner.intern("Russia");
            sim.admit_projectile(
                projectile_id,
                ProjectileSpawn {
                    source_id: crate::sim::combat::RAD_NO_ATTACKER,
                    origin: impact,
                    target: ProjectileTarget::Entity(building_id),
                    initial_target_position: impact,
                    payload: ProjectilePayload {
                        base_damage: 10,
                        warhead,
                        weapon,
                        owner: projectile_owner,
                    },
                    speed_leptons_per_frame: 64,
                    velocity: ProjectileVelocity::new(64, 0, 0),
                    trajectory: ProjectileTrajectory::Straight,
                    guidance: None,
                    visual: ProjectileVisualState::new(0, 0, 0),
                    arm_frames: 0,
                    fuse_frames: Some(0),
                    ranged_fuse: false,
                    tracks_target: false,
                    target_expiry: TargetExpiryPolicy::Expire,
                    collision: ProjectileCollisionPolicy::NONE,
                },
            );
            sim.set_logic_order_for_test(if projectile_first {
                vec![projectile_id, building_id]
            } else {
                vec![building_id, projectile_id]
            });

            sim.object_ai_stage(Some(&rules));

            let building = sim.substrate.entities.get(building_id).unwrap();
            (
                building.health.current,
                building.dying,
                sim.houses[&owner].credits,
            )
        }

        assert_eq!(run(true), (0, true, 9));
        assert_eq!(
            run(false),
            (3, false, 0),
            "the Building slot heals and debits before the later Bullet lands"
        );
    }
}
