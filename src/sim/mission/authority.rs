//! Dormant exact Mission authority and category-aware wrapper transactions.
//!
//! Current gameplay remains on `compatibility`; these methods have no
//! production callers.  They preserve native base transitions, Aircraft leaf
//! policy, synchronous Queue/Ready/Commence order, and the verified
//! Target/NavCom wrapper order.  Concrete setters use a two-phase provider so
//! unavailable effects cannot leave a partially-written transaction.

use crate::map::entities::EntityCategory;
use crate::sim::combat::TargetKind;
use crate::sim::components::NavTargetRef;
use crate::sim::world::Simulation;

use super::concrete_effects::{
    AuthorityUnavailable, ConcreteMissionEffects, ConcreteSetterRequest,
    UnavailableConcreteMissionEffects,
};
use super::readiness::{
    AircraftReadyView, BuildingReadyView, InfantryReadyView, ReadyLeptonPoint, ReadyResult,
    ReadyUnavailable, UnitReadyBuilding, UnitReadyContact, UnitReadyView, UnitReadyWorld,
    aircraft_ready_to_commence, building_ready_to_commence, infantry_ready_to_commence,
    unit_ready_to_commence,
};
use super::verb::{self, QueueContinuation};
use super::{MissionCom, MissionId};

mod ready_private {
    pub trait Sealed {}
}

const AIRCRAFT_ACTION_EXCEPTION: MissionId = MissionId::from_raw(0x1e);
const MISSION_GUARD: MissionId = MissionId::from_raw(5);
const AIRCRAFT_PROTECTED: [MissionId; 5] = [
    MissionId::from_raw(4),
    MissionId::from_raw(0x1a),
    MissionId::from_raw(0x1b),
    MissionId::from_raw(0x1e),
    MissionId::from_raw(0x1f),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverridePacket {
    pub mission: MissionId,
    pub combat_target: Option<TargetKind>,
    pub destination: Option<NavTargetRef>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MissionAuthorityError {
    #[error("Mission receiver {0} does not exist")]
    MissingReceiver(u64),
    #[error(transparent)]
    Readiness(#[from] ReadyUnavailable),
    #[error(transparent)]
    AuthorityUnavailable(#[from] AuthorityUnavailable),
}

/// Supplies exact non-Mission inputs for Queue's synchronous readiness call.
///
/// Validation runs against a copied post-Queue Mission preview before the real
/// queue write.  The final readiness value is deliberately read again after
/// the real write and is never cached from validation. The trait is sealed:
/// successful validation guarantees that this fresh read is available, so an
/// integration error cannot be returned after Queue has mutated real state.
pub(crate) trait ReadyInputProvider: ready_private::Sealed {
    fn validate_ready_inputs(
        &self,
        sim: &Simulation,
        receiver: u64,
        preview: &MissionCom,
    ) -> Result<(), ReadyUnavailable>;

    fn ready_to_commence(&self, sim: &Simulation, receiver: u64, mission: &MissionCom) -> bool;
}

/// Production view over currently represented exact inputs.
///
/// Unit factory/contact lookup and signed-height ownership are not available
/// through this narrow provider yet, so branches that reach those reads return
/// an explicit error. Early native false branches still short-circuit normally.
#[derive(Debug, Default)]
pub(crate) struct EntityReadyInputProvider;

struct UnavailableUnitWorld;

impl ready_private::Sealed for EntityReadyInputProvider {}

impl UnitReadyWorld for UnavailableUnitWorld {
    fn contact_slot_zero(&self) -> Result<Option<UnitReadyContact>, ReadyUnavailable> {
        Err(ReadyUnavailable::WorldLookup)
    }

    fn building_under_in_stored_order(
        &self,
        _unit_position: ReadyLeptonPoint,
    ) -> Result<Option<UnitReadyBuilding>, ReadyUnavailable> {
        Err(ReadyUnavailable::WorldLookup)
    }
}

impl EntityReadyInputProvider {
    fn evaluate(&self, sim: &Simulation, receiver: u64, mission: &MissionCom) -> ReadyResult {
        let entity = sim
            .substrate
            .entities
            .get(receiver)
            .ok_or(ReadyUnavailable::WorldLookup)?;
        let locomotor = entity
            .locomotor
            .as_ref()
            .and_then(|locomotor| locomotor.mission_ready_state);
        let attack_target_present = entity.attack_target.is_some();

        match entity.category {
            EntityCategory::Unit => {
                let leaf = entity
                    .mission_leaf
                    .as_unit()
                    .ok_or(ReadyUnavailable::WorldLookup)?;
                let position = ReadyLeptonPoint::new(
                    i32::from(entity.position.rx)
                        .wrapping_mul(256)
                        .wrapping_add(entity.position.sub_x.to_num::<i32>()),
                    i32::from(entity.position.ry)
                        .wrapping_mul(256)
                        .wrapping_add(entity.position.sub_y.to_num::<i32>()),
                );
                unit_ready_to_commence(UnitReadyView {
                    mission,
                    leaf,
                    unload_active: entity
                        .miner
                        .as_ref()
                        .is_some_and(|miner| miner.unload_active),
                    locomotor,
                    // The current u8 terrain level is not the verified signed
                    // ObjectClass height dword.
                    signed_height: None,
                    attack_target_present,
                    position,
                    world: &UnavailableUnitWorld,
                })
            }
            EntityCategory::Infantry => {
                let leaf = entity
                    .mission_leaf
                    .as_infantry()
                    .ok_or(ReadyUnavailable::WorldLookup)?;
                infantry_ready_to_commence(InfantryReadyView {
                    mission,
                    leaf,
                    object_is_falling_down: entity.object_is_falling_down,
                    locomotor,
                    attack_target_present,
                })
            }
            EntityCategory::Aircraft => {
                let leaf = entity
                    .mission_leaf
                    .as_aircraft()
                    .ok_or(ReadyUnavailable::WorldLookup)?;
                aircraft_ready_to_commence(AircraftReadyView { mission, leaf })
            }
            EntityCategory::Structure => {
                let leaf = entity
                    .mission_leaf
                    .as_building()
                    .ok_or(ReadyUnavailable::WorldLookup)?;
                building_ready_to_commence(BuildingReadyView { leaf })
            }
        }
    }
}

impl ReadyInputProvider for EntityReadyInputProvider {
    fn validate_ready_inputs(
        &self,
        sim: &Simulation,
        receiver: u64,
        preview: &MissionCom,
    ) -> Result<(), ReadyUnavailable> {
        self.evaluate(sim, receiver, preview).map(|_| ())
    }

    fn ready_to_commence(&self, sim: &Simulation, receiver: u64, mission: &MissionCom) -> bool {
        self.evaluate(sim, receiver, mission)
            .expect("successful readiness preflight must make the fresh read available")
    }
}

fn aircraft_allows(entity: &crate::sim::game_entity::GameEntity, requested: MissionId) -> bool {
    let Some(leaf) = entity.mission_leaf.as_aircraft() else {
        return true;
    };
    leaf.airstrike_manager_present()
        || !AIRCRAFT_PROTECTED.contains(&entity.mission.current())
        || AIRCRAFT_PROTECTED.contains(&requested)
}

fn commence_leaf(entity: &mut crate::sim::game_entity::GameEntity, now: u32) -> bool {
    if entity.mission_leaf.as_aircraft().is_some() {
        let old_current = entity.mission.current();
        if old_current != AIRCRAFT_ACTION_EXCEPTION {
            entity.mission_leaf.clear_aircraft_action_for_commence();
        }
    }
    verb::commence_base(&mut entity.mission, now)
}

impl Simulation {
    pub(crate) fn mission_assign_exact(
        &mut self,
        receiver: u64,
        requested: MissionId,
        now: u32,
    ) -> Result<(), MissionAuthorityError> {
        let entity = self
            .substrate
            .entities
            .get_mut(receiver)
            .ok_or(MissionAuthorityError::MissingReceiver(receiver))?;
        if !aircraft_allows(entity, requested) {
            return Ok(());
        }
        verb::assign_base(&mut entity.mission, requested, now);
        Ok(())
    }

    pub(crate) fn mission_commence_exact(
        &mut self,
        receiver: u64,
        now: u32,
    ) -> Result<bool, MissionAuthorityError> {
        let entity = self
            .substrate
            .entities
            .get_mut(receiver)
            .ok_or(MissionAuthorityError::MissingReceiver(receiver))?;
        Ok(commence_leaf(entity, now))
    }

    pub(crate) fn mission_queue_exact(
        &mut self,
        receiver: u64,
        requested: MissionId,
        commence_now: i32,
        now: u32,
        readiness: &impl ReadyInputProvider,
    ) -> Result<(), MissionAuthorityError> {
        let entity = self
            .substrate
            .entities
            .get(receiver)
            .ok_or(MissionAuthorityError::MissingReceiver(receiver))?;
        if !aircraft_allows(entity, requested) {
            return Ok(());
        }
        // Derived wrappers forward the full dword, but MissionClass::Queue
        // reads only its low byte to decide whether promotion is immediate.
        let commence_immediately = commence_now as u8 != 0;

        let mut preview = entity.mission;
        if verb::queue_base(&mut preview, requested) == QueueContinuation::OuterGuardBlocked {
            return Ok(());
        }
        if commence_immediately {
            readiness.validate_ready_inputs(self, receiver, &preview)?;
        }

        let entity = self
            .substrate
            .entities
            .get_mut(receiver)
            .expect("receiver was resolved before Queue mutation");
        let continuation = verb::queue_base(&mut entity.mission, requested);
        debug_assert_eq!(continuation, QueueContinuation::Continue);
        if !commence_immediately {
            return Ok(());
        }

        let ready = {
            let entity = self
                .substrate
                .entities
                .get(receiver)
                .expect("receiver remains present during synchronous Queue");
            readiness.ready_to_commence(self, receiver, &entity.mission)
        };
        if ready {
            let entity = self
                .substrate
                .entities
                .get_mut(receiver)
                .expect("receiver remains present during synchronous Commence");
            commence_leaf(entity, now);
        }
        Ok(())
    }

    pub(crate) fn mission_override_exact(
        &mut self,
        receiver: u64,
        packet: OverridePacket,
    ) -> Result<(), MissionAuthorityError> {
        let mut effects = UnavailableConcreteMissionEffects;
        self.mission_override_exact_with_effects(receiver, packet, &mut effects)
    }

    pub(crate) fn mission_restore_exact(
        &mut self,
        receiver: u64,
    ) -> Result<bool, MissionAuthorityError> {
        let mut effects = UnavailableConcreteMissionEffects;
        self.mission_restore_exact_with_effects(receiver, &mut effects)
    }

    pub(crate) fn mission_refinery_completion_exact(
        &mut self,
        receiver: u64,
        now: u32,
    ) -> Result<(), MissionAuthorityError> {
        self.mission_queue_exact(receiver, MISSION_GUARD, 0, now, &EntityReadyInputProvider)?;
        let entity = self
            .substrate
            .entities
            .get_mut(receiver)
            .expect("Queue resolved the refinery-completion receiver");
        entity.mission.set_movement_bypass_after_verified_queue();
        Ok(())
    }

    pub(crate) fn mission_jumpjet_move_to_completion_exact(
        &mut self,
        receiver: u64,
        now: u32,
        readiness: &impl ReadyInputProvider,
    ) -> Result<(), MissionAuthorityError> {
        self.validate_jumpjet_second_gate_previews(receiver, now, readiness)?;
        self.mission_queue_exact(receiver, MISSION_GUARD, 1, now, readiness)?;
        {
            let entity = self
                .substrate
                .entities
                .get_mut(receiver)
                .expect("Queue resolved the jumpjet-completion receiver");
            entity.mission.set_movement_bypass_after_verified_queue();
        }
        let ready = {
            let entity = self
                .substrate
                .entities
                .get(receiver)
                .expect("receiver remains present for the second Jumpjet gate");
            readiness.ready_to_commence(self, receiver, &entity.mission)
        };
        if ready {
            let entity = self
                .substrate
                .entities
                .get_mut(receiver)
                .expect("receiver remains present for the second Jumpjet Commence");
            commence_leaf(entity, now);
        }
        Ok(())
    }

    fn validate_jumpjet_second_gate_previews(
        &self,
        receiver: u64,
        now: u32,
        readiness: &impl ReadyInputProvider,
    ) -> Result<(), MissionAuthorityError> {
        let entity = self
            .substrate
            .entities
            .get(receiver)
            .ok_or(MissionAuthorityError::MissingReceiver(receiver))?;

        let mut post_queue = entity.mission;
        let queue_continues = aircraft_allows(entity, MISSION_GUARD)
            && verb::queue_base(&mut post_queue, MISSION_GUARD) == QueueContinuation::Continue;

        if !queue_continues {
            post_queue.set_movement_bypass_after_verified_queue();
            readiness.validate_ready_inputs(self, receiver, &post_queue)?;
            return Ok(());
        }

        let mut no_commence = post_queue;
        no_commence.set_movement_bypass_after_verified_queue();
        readiness.validate_ready_inputs(self, receiver, &no_commence)?;

        let mut successful_commence = post_queue;
        verb::commence_base(&mut successful_commence, now);
        successful_commence.set_movement_bypass_after_verified_queue();
        readiness.validate_ready_inputs(self, receiver, &successful_commence)?;
        Ok(())
    }

    pub(crate) fn mission_try_consume_building_ready_exact(
        &mut self,
        receiver: u64,
        now: u32,
    ) -> Result<bool, MissionAuthorityError> {
        let ready = {
            let entity = self
                .substrate
                .entities
                .get(receiver)
                .ok_or(MissionAuthorityError::MissingReceiver(receiver))?;
            let leaf = entity
                .mission_leaf
                .as_building()
                .ok_or(ReadyUnavailable::WorldLookup)?;
            building_ready_to_commence(BuildingReadyView { leaf })?
        };
        if !ready {
            return Ok(false);
        }

        let entity = self
            .substrate
            .entities
            .get_mut(receiver)
            .expect("building-ready receiver remains present");
        if !commence_leaf(entity, now) {
            return Ok(false);
        }
        entity.mission_leaf.set_building_ready_latch(0);
        Ok(true)
    }

    fn mission_override_exact_with_effects<E: ConcreteMissionEffects>(
        &mut self,
        receiver: u64,
        packet: OverridePacket,
        effects: &mut E,
    ) -> Result<(), MissionAuthorityError> {
        let entity = self
            .substrate
            .entities
            .get(receiver)
            .ok_or(MissionAuthorityError::MissingReceiver(receiver))?;
        if !aircraft_allows(entity, packet.mission) {
            return Ok(());
        }
        let request = match entity.category {
            EntityCategory::Structure => ConcreteSetterRequest::Target {
                requested: packet.combat_target,
            },
            EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft => {
                ConcreteSetterRequest::TargetAndDestination {
                    requested_target: packet.combat_target,
                    requested_destination: packet.destination,
                }
            }
        };
        let prepared = effects.preflight(self, receiver, request)?;

        {
            let entity = self
                .substrate
                .entities
                .get_mut(receiver)
                .expect("preflight cannot remove receiver");
            if entity.category != EntityCategory::Structure {
                entity.navigation.suspended_nav_com = entity.navigation.nav_com;
            }
            entity.suspended_attack_target =
                entity.attack_target.as_ref().map(|target| target.target);
            verb::override_base(&mut entity.mission, packet.mission);
        }
        effects.apply_target(self, &prepared, packet.combat_target);
        if !matches!(
            self.substrate
                .entities
                .get(receiver)
                .map(|entity| entity.category),
            Some(EntityCategory::Structure)
        ) {
            effects.apply_destination_mode_one(self, &prepared, packet.destination);
        }
        Ok(())
    }

    fn mission_restore_exact_with_effects<E: ConcreteMissionEffects>(
        &mut self,
        receiver: u64,
        effects: &mut E,
    ) -> Result<bool, MissionAuthorityError> {
        let entity = self
            .substrate
            .entities
            .get(receiver)
            .ok_or(MissionAuthorityError::MissingReceiver(receiver))?;
        if entity.mission.suspended() == MissionId::NONE {
            return Ok(false);
        }
        let saved_target = entity.suspended_attack_target;
        let saved_destination = entity.navigation.suspended_nav_com;
        let category = entity.category;
        let request = match category {
            EntityCategory::Structure => ConcreteSetterRequest::Target {
                requested: saved_target,
            },
            EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft => {
                ConcreteSetterRequest::TargetAndDestination {
                    requested_target: saved_target,
                    requested_destination: saved_destination,
                }
            }
        };
        let prepared = effects.preflight(self, receiver, request)?;

        let restored = {
            let entity = self
                .substrate
                .entities
                .get_mut(receiver)
                .expect("preflight cannot remove receiver");
            verb::restore_base(&mut entity.mission)
        };
        debug_assert!(restored);
        effects.apply_target(self, &prepared, saved_target);
        if category != EntityCategory::Structure {
            effects.apply_destination_mode_one(self, &prepared, saved_destination);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    use super::super::MissionDispatchTimer;
    use super::super::concrete_effects::{ConcreteEffectEvent, RecordingConcreteMissionEffects};
    use super::super::leaf::MissionLeafState;
    use super::super::state::MissionTestFixture;
    use super::*;
    use crate::sim::animation::SequenceKind;
    use crate::sim::combat::{AttackTarget, PendingInfantryFire};
    use crate::sim::game_entity::GameEntity;

    const GUARD: MissionId = MissionId::from_raw(5);
    const MOVE: MissionId = MissionId::from_raw(2);
    const ATTACK: MissionId = MissionId::from_raw(1);

    struct TestReadyProvider {
        validation: Result<(), ReadyUnavailable>,
        ready: bool,
        validations: Cell<u32>,
        reads: Cell<u32>,
        missions: RefCell<Vec<MissionCom>>,
    }

    struct SequencedReadyProvider {
        values: RefCell<VecDeque<bool>>,
        validations: Cell<u32>,
        reads: Cell<u32>,
        missions: RefCell<Vec<MissionCom>>,
    }

    struct RejectSuccessfulJumpjetPreview {
        validations: Cell<u32>,
        reads: Cell<u32>,
        missions: RefCell<Vec<MissionCom>>,
    }

    impl SequencedReadyProvider {
        fn new(values: impl IntoIterator<Item = bool>) -> Self {
            Self {
                values: RefCell::new(values.into_iter().collect()),
                validations: Cell::new(0),
                reads: Cell::new(0),
                missions: RefCell::new(Vec::new()),
            }
        }
    }

    impl ready_private::Sealed for SequencedReadyProvider {}

    impl ReadyInputProvider for SequencedReadyProvider {
        fn validate_ready_inputs(
            &self,
            _sim: &Simulation,
            _receiver: u64,
            preview: &MissionCom,
        ) -> Result<(), ReadyUnavailable> {
            self.validations.set(self.validations.get() + 1);
            self.missions.borrow_mut().push(*preview);
            Ok(())
        }

        fn ready_to_commence(
            &self,
            _sim: &Simulation,
            _receiver: u64,
            _mission: &MissionCom,
        ) -> bool {
            self.reads.set(self.reads.get() + 1);
            self.values
                .borrow_mut()
                .pop_front()
                .expect("sequenced readiness value")
        }
    }

    impl ready_private::Sealed for RejectSuccessfulJumpjetPreview {}

    impl ReadyInputProvider for RejectSuccessfulJumpjetPreview {
        fn validate_ready_inputs(
            &self,
            _sim: &Simulation,
            _receiver: u64,
            preview: &MissionCom,
        ) -> Result<(), ReadyUnavailable> {
            self.validations.set(self.validations.get() + 1);
            self.missions.borrow_mut().push(*preview);
            if preview.movement_bypass_latch() != 0
                && preview.current() == GUARD
                && preview.queued() == MissionId::NONE
            {
                Err(ReadyUnavailable::WorldLookup)
            } else {
                Ok(())
            }
        }

        fn ready_to_commence(
            &self,
            _sim: &Simulation,
            _receiver: u64,
            _mission: &MissionCom,
        ) -> bool {
            self.reads.set(self.reads.get() + 1);
            false
        }
    }

    impl TestReadyProvider {
        fn ready(value: bool) -> Self {
            Self {
                validation: Ok(()),
                ready: value,
                validations: Cell::new(0),
                reads: Cell::new(0),
                missions: RefCell::new(Vec::new()),
            }
        }
    }

    impl ready_private::Sealed for TestReadyProvider {}

    impl ReadyInputProvider for TestReadyProvider {
        fn validate_ready_inputs(
            &self,
            _sim: &Simulation,
            _receiver: u64,
            preview: &MissionCom,
        ) -> Result<(), ReadyUnavailable> {
            self.validations.set(self.validations.get() + 1);
            self.missions.borrow_mut().push(*preview);
            self.validation
        }

        fn ready_to_commence(
            &self,
            _sim: &Simulation,
            _receiver: u64,
            mission: &MissionCom,
        ) -> bool {
            self.reads.set(self.reads.get() + 1);
            self.missions.borrow_mut().push(*mission);
            self.ready
        }
    }

    fn entity(category: EntityCategory, current: MissionId) -> GameEntity {
        let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        entity.category = category;
        entity.mission_leaf = MissionLeafState::for_entity_category(category);
        entity.mission.apply_test_fixture(MissionTestFixture {
            current,
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0xa5,
            handler_state: 0x1122_3344,
            mission_start_frame: 0x5566_7788,
            ai_counter: 0x99aa_bbcc,
            dispatch_timer: MissionDispatchTimer::from_raw(-17, -29),
        });
        entity
    }

    fn sim_with(entity: GameEntity) -> Simulation {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(entity);
        sim
    }

    #[test]
    fn mission_authority_assign_applies_aircraft_gate_before_writes() {
        let mut aircraft = entity(EntityCategory::Aircraft, MissionId::from_raw(4));
        aircraft.mission_leaf = MissionLeafState::aircraft_raw_for_test(9, 1, false);
        let before = aircraft.mission;
        let mut sim = sim_with(aircraft);

        sim.mission_assign_exact(1, MissionId::NONE, 10).unwrap();
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission, before);

        sim.mission_assign_exact(1, MissionId::from_raw(0x1a), 10)
            .unwrap();
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.current(),
            MissionId::from_raw(0x1a)
        );
    }

    #[test]
    fn mission_authority_assign_preserves_base_guard_and_raw_ids() {
        let mut sim = sim_with(entity(EntityCategory::Unit, MissionId::from_raw(28)));
        let before = sim.substrate.entities.get(1).unwrap().mission;
        sim.mission_assign_exact(1, GUARD, 10).unwrap();
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission, before);

        let unknown = MissionId::from_raw(0x1234_5678);
        sim.mission_assign_exact(1, unknown, 11).unwrap();
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.current(),
            unknown
        );
    }

    #[test]
    fn mission_authority_queue_unavailable_validation_is_atomic() {
        let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
        let before = sim.substrate.entities.get(1).unwrap().clone();
        let readiness = TestReadyProvider {
            validation: Err(ReadyUnavailable::SignedHeight),
            ready: true,
            validations: Cell::new(0),
            reads: Cell::new(0),
            missions: RefCell::new(Vec::new()),
        };

        assert!(matches!(
            sim.mission_queue_exact(1, MOVE, 1, 10, &readiness),
            Err(MissionAuthorityError::Readiness(
                ReadyUnavailable::SignedHeight
            ))
        ));
        let after = sim.substrate.entities.get(1).unwrap();
        assert_eq!(after.mission, before.mission);
        assert_eq!(readiness.validations.get(), 1);
        assert_eq!(readiness.reads.get(), 0);
    }

    #[test]
    fn mission_authority_queue_zero_never_reads_or_commences() {
        let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
        let readiness = TestReadyProvider::ready(true);

        sim.mission_queue_exact(1, MOVE, 0, 10, &readiness).unwrap();
        let entity = sim.substrate.entities.get(1).unwrap();
        assert_eq!(entity.mission.current(), GUARD);
        assert_eq!(entity.mission.queued(), MOVE);
        assert_eq!(readiness.validations.get(), 0);
        assert_eq!(readiness.reads.get(), 0);
    }

    #[test]
    fn mission_authority_queue_tests_only_low_byte_of_commence_now() {
        for commence_now in [0x100, 0x1_0000, i32::MIN] {
            let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
            let readiness = TestReadyProvider::ready(true);

            sim.mission_queue_exact(1, MOVE, commence_now, 10, &readiness)
                .unwrap();

            let after = sim.substrate.entities.get(1).unwrap();
            assert_eq!(after.mission.current(), GUARD);
            assert_eq!(after.mission.queued(), MOVE);
            assert_eq!(readiness.validations.get(), 0);
            assert_eq!(readiness.reads.get(), 0);
        }

        let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
        let readiness = TestReadyProvider::ready(true);
        sim.mission_queue_exact(1, MOVE, 0x101, 10, &readiness)
            .unwrap();

        let after = sim.substrate.entities.get(1).unwrap();
        assert_eq!(after.mission.current(), MOVE);
        assert_eq!(after.mission.queued(), MissionId::NONE);
        assert_eq!(readiness.validations.get(), 1);
        assert_eq!(readiness.reads.get(), 1);
    }

    #[test]
    fn mission_authority_queue_ready_false_keeps_queue_ready_true_commences_inline() {
        let mut false_sim = sim_with(entity(EntityCategory::Unit, GUARD));
        let false_ready = TestReadyProvider::ready(false);
        false_sim
            .mission_queue_exact(1, MOVE, 1, 10, &false_ready)
            .unwrap();
        let false_entity = false_sim.substrate.entities.get(1).unwrap();
        assert_eq!(false_entity.mission.current(), GUARD);
        assert_eq!(false_entity.mission.queued(), MOVE);
        assert_eq!(false_ready.validations.get(), 1);
        assert_eq!(false_ready.reads.get(), 1);

        let mut true_sim = sim_with(entity(EntityCategory::Unit, GUARD));
        let true_ready = TestReadyProvider::ready(true);
        true_sim
            .mission_queue_exact(1, MOVE, 1, 10, &true_ready)
            .unwrap();
        let entity = true_sim.substrate.entities.get(1).unwrap();
        assert_eq!(entity.mission.current(), MOVE);
        assert_eq!(entity.mission.queued(), MissionId::NONE);
        assert_eq!(true_ready.validations.get(), 1);
        assert_eq!(true_ready.reads.get(), 1);
    }

    #[test]
    fn aircraft_mission_authority_queue_gate_covers_manager_unknown_and_none() {
        for requested in [MissionId::NONE, MissionId::from_raw(0x1234_5678)] {
            let mut aircraft = entity(EntityCategory::Aircraft, MissionId::from_raw(4));
            aircraft.mission_leaf = MissionLeafState::aircraft_raw_for_test(9, 1, false);
            let before_mission = aircraft.mission;
            let before_leaf = aircraft.mission_leaf;
            let mut sim = sim_with(aircraft);
            let readiness = TestReadyProvider::ready(true);

            sim.mission_queue_exact(1, requested, 1, 10, &readiness)
                .unwrap();

            let after = sim.substrate.entities.get(1).unwrap();
            assert_eq!(after.mission, before_mission);
            assert_eq!(after.mission_leaf, before_leaf);
            assert_eq!(readiness.validations.get(), 0);
            assert_eq!(readiness.reads.get(), 0);
        }

        let mut protected_request = entity(EntityCategory::Aircraft, MissionId::from_raw(4));
        protected_request.mission_leaf = MissionLeafState::aircraft_raw_for_test(9, 1, false);
        let mut sim = sim_with(protected_request);
        sim.mission_queue_exact(
            1,
            MissionId::from_raw(0x1a),
            0,
            10,
            &TestReadyProvider::ready(false),
        )
        .unwrap();
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.queued(),
            MissionId::from_raw(0x1a)
        );

        let mut managed = entity(EntityCategory::Aircraft, MissionId::from_raw(4));
        managed.mission_leaf = MissionLeafState::aircraft_raw_for_test(9, 1, true);
        let unknown = MissionId::from_raw(0x1234_5678);
        let mut sim = sim_with(managed);
        sim.mission_queue_exact(1, unknown, 0, 10, &TestReadyProvider::ready(false))
            .unwrap();
        assert_eq!(
            sim.substrate.entities.get(1).unwrap().mission.queued(),
            unknown
        );
    }

    #[test]
    fn mission_authority_queue_outer_guards_do_not_read_readiness() {
        for (current, requested) in [
            (MissionId::from_raw(19), MOVE),
            (MissionId::from_raw(28), GUARD),
        ] {
            let mut sim = sim_with(entity(EntityCategory::Unit, current));
            let before = sim.substrate.entities.get(1).unwrap().mission;
            let readiness = TestReadyProvider::ready(true);

            sim.mission_queue_exact(1, requested, 1, 10, &readiness)
                .unwrap();

            assert_eq!(sim.substrate.entities.get(1).unwrap().mission, before);
            assert_eq!(readiness.validations.get(), 0);
            assert_eq!(readiness.reads.get(), 0);
        }
    }

    #[test]
    fn mission_authority_queue_none_and_redundant_requests_still_read_once() {
        for requested in [MissionId::NONE, GUARD] {
            let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
            let before = sim.substrate.entities.get(1).unwrap().mission;
            let readiness = TestReadyProvider::ready(false);

            sim.mission_queue_exact(1, requested, 1, 10, &readiness)
                .unwrap();

            assert_eq!(sim.substrate.entities.get(1).unwrap().mission, before);
            assert_eq!(readiness.validations.get(), 1);
            assert_eq!(readiness.reads.get(), 1);
        }
    }

    #[test]
    fn aircraft_queue_owned_commence_clears_action_except_for_old_1e() {
        for (old_current, requested, expected_action) in [
            (ATTACK, MOVE, 0),
            (AIRCRAFT_ACTION_EXCEPTION, MissionId::from_raw(0x1a), 9),
        ] {
            let mut aircraft = entity(EntityCategory::Aircraft, old_current);
            aircraft.mission_leaf = MissionLeafState::aircraft_raw_for_test(9, 1, false);
            let mut sim = sim_with(aircraft);
            let readiness = TestReadyProvider::ready(true);

            sim.mission_queue_exact(1, requested, 1, 10, &readiness)
                .unwrap();

            let after = sim.substrate.entities.get(1).unwrap();
            assert_eq!(after.mission.current(), requested);
            assert_eq!(after.mission.queued(), MissionId::NONE);
            assert_eq!(
                after.mission_leaf.as_aircraft().unwrap().action_latch(),
                expected_action
            );
            assert_eq!(readiness.validations.get(), 1);
            assert_eq!(readiness.reads.get(), 1);
        }
    }

    #[test]
    fn aircraft_mission_authority_commence_hook_runs_even_with_empty_queue() {
        let mut aircraft = entity(EntityCategory::Aircraft, ATTACK);
        aircraft.mission_leaf = MissionLeafState::aircraft_raw_for_test(9, 1, false);
        let mut sim = sim_with(aircraft);

        assert!(!sim.mission_commence_exact(1, 10).unwrap());
        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission_leaf
                .as_aircraft()
                .unwrap()
                .action_latch(),
            0
        );
    }

    #[test]
    fn override_target_unavailable_is_fieldwise_noop() {
        let mut building = entity(EntityCategory::Structure, GUARD);
        building.attack_target = Some(AttackTarget::new(7));
        let before = building.clone();
        let mut sim = sim_with(building);

        assert!(matches!(
            sim.mission_override_exact(
                1,
                OverridePacket {
                    mission: ATTACK,
                    combat_target: Some(TargetKind::Entity(8)),
                    destination: None,
                }
            ),
            Err(MissionAuthorityError::AuthorityUnavailable(
                AuthorityUnavailable::TargetSetter(1)
            ))
        ));
        let after = sim.substrate.entities.get(1).unwrap();
        assert_eq!(after.mission, before.mission);
        assert_eq!(
            after.attack_target.as_ref().map(|target| target.target),
            before.attack_target.as_ref().map(|target| target.target)
        );
        assert_eq!(
            after.suspended_attack_target,
            before.suspended_attack_target
        );
    }

    #[test]
    fn override_destination_unavailable_is_fieldwise_noop() {
        let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
        let before = sim.substrate.entities.get(1).unwrap().clone();
        let mut effects = RecordingConcreteMissionEffects::available();
        effects.allow_destination = false;

        assert!(matches!(
            sim.mission_override_exact_with_effects(
                1,
                OverridePacket {
                    mission: ATTACK,
                    combat_target: None,
                    destination: Some(NavTargetRef::cell(8, 9)),
                },
                &mut effects,
            ),
            Err(MissionAuthorityError::AuthorityUnavailable(
                AuthorityUnavailable::DestinationSetter(1)
            ))
        ));
        let after = sim.substrate.entities.get(1).unwrap();
        assert_eq!(after.mission, before.mission);
        assert_eq!(
            after.navigation.suspended_nav_com,
            before.navigation.suspended_nav_com
        );
    }

    #[test]
    fn foot_override_provider_order_includes_same_identity_target_dispatch() {
        let mut unit = entity(EntityCategory::Unit, GUARD);
        let mut active_target = AttackTarget::new(7);
        active_target.cooldown_ticks = 17;
        active_target.burst_remaining = 3;
        active_target.burst_delay_ticks = 2;
        active_target.pending_infantry_fire = Some(PendingInfantryFire {
            sequence: SequenceKind::Attack,
            fire_frame: 4,
        });
        unit.attack_target = Some(active_target);
        unit.navigation.nav_com = Some(NavTargetRef::cell(1, 2));
        unit.navigation.pending_arrival_clear = true;
        let mut sim = sim_with(unit);
        let mut effects = RecordingConcreteMissionEffects::available();

        sim.mission_override_exact_with_effects(
            1,
            OverridePacket {
                mission: ATTACK,
                combat_target: Some(TargetKind::Entity(7)),
                destination: Some(NavTargetRef::cell(8, 9)),
            },
            &mut effects,
        )
        .unwrap();

        assert_eq!(effects.events.len(), 3);
        assert!(matches!(
            effects.events[0],
            ConcreteEffectEvent::Preflight {
                request: ConcreteSetterRequest::TargetAndDestination { .. },
                ..
            }
        ));
        assert!(matches!(
            effects.events[1],
            ConcreteEffectEvent::Target {
                requested: Some(TargetKind::Entity(7)),
                mission_current: ATTACK,
                suspended_mission: GUARD,
                archived_target: Some(TargetKind::Entity(7)),
                archived_destination: Some(NavTargetRef::Cell { rx: 1, ry: 2 }),
                ..
            }
        ));
        assert!(matches!(
            effects.events[2],
            ConcreteEffectEvent::Destination {
                requested: Some(NavTargetRef::Cell { rx: 8, ry: 9 }),
                mission_current: ATTACK,
                installed_target: Some(TargetKind::Entity(7)),
                ..
            }
        ));
        let installed = sim
            .substrate
            .entities
            .get(1)
            .unwrap()
            .attack_target
            .as_ref()
            .unwrap();
        assert_eq!(installed.cooldown_ticks, 17);
        assert_eq!(installed.burst_remaining, 3);
        assert_eq!(installed.burst_delay_ticks, 2);
        assert_eq!(
            installed.pending_infantry_fire,
            Some(PendingInfantryFire {
                sequence: SequenceKind::Attack,
                fire_frame: 4,
            })
        );
        assert!(
            !sim.substrate
                .entities
                .get(1)
                .unwrap()
                .navigation
                .pending_arrival_clear
        );
    }

    #[test]
    fn override_transaction_traces_each_concrete_category_and_building_never_sets_nav() {
        for category in [
            EntityCategory::Structure,
            EntityCategory::Unit,
            EntityCategory::Infantry,
            EntityCategory::Aircraft,
        ] {
            let mut receiver = entity(category, GUARD);
            receiver.attack_target = Some(AttackTarget::new(7));
            receiver.navigation.nav_com = Some(NavTargetRef::cell(1, 2));
            let mut sim = sim_with(receiver);
            let mut effects = RecordingConcreteMissionEffects::available();

            sim.mission_override_exact_with_effects(
                1,
                OverridePacket {
                    mission: ATTACK,
                    combat_target: Some(TargetKind::Entity(8)),
                    destination: Some(NavTargetRef::cell(8, 9)),
                },
                &mut effects,
            )
            .unwrap();

            let after = sim.substrate.entities.get(1).unwrap();
            assert_eq!(after.mission.current(), ATTACK);
            assert_eq!(after.mission.suspended(), GUARD);
            assert_eq!(after.suspended_attack_target, Some(TargetKind::Entity(7)));
            assert!(matches!(
                effects.events[0],
                ConcreteEffectEvent::Preflight { .. }
            ));
            assert!(matches!(
                effects.events[1],
                ConcreteEffectEvent::Target {
                    mission_current: ATTACK,
                    suspended_mission: GUARD,
                    archived_target: Some(TargetKind::Entity(7)),
                    ..
                }
            ));

            if category == EntityCategory::Structure {
                assert_eq!(effects.events.len(), 2);
                assert_eq!(after.navigation.nav_com, Some(NavTargetRef::cell(1, 2)));
                assert_eq!(after.navigation.suspended_nav_com, None);
            } else {
                assert_eq!(effects.events.len(), 3);
                assert_eq!(
                    after.navigation.suspended_nav_com,
                    Some(NavTargetRef::cell(1, 2))
                );
                assert_eq!(after.navigation.nav_com, Some(NavTargetRef::cell(8, 9)));
                assert!(matches!(
                    effects.events[2],
                    ConcreteEffectEvent::Destination {
                        mission_current: ATTACK,
                        installed_target: Some(TargetKind::Entity(8)),
                        ..
                    }
                ));
            }
        }
    }

    #[test]
    fn guarded_override_base_still_archives_and_runs_concrete_setters() {
        let deliberate = MissionId::from_raw(28);
        let mut unit = entity(EntityCategory::Unit, deliberate);
        unit.attack_target = Some(AttackTarget::new(7));
        unit.navigation.nav_com = Some(NavTargetRef::cell(1, 2));
        let before_mission = unit.mission;
        let mut sim = sim_with(unit);
        let mut effects = RecordingConcreteMissionEffects::available();

        sim.mission_override_exact_with_effects(
            1,
            OverridePacket {
                mission: GUARD,
                combat_target: Some(TargetKind::Entity(8)),
                destination: Some(NavTargetRef::cell(8, 9)),
            },
            &mut effects,
        )
        .unwrap();

        let after = sim.substrate.entities.get(1).unwrap();
        assert_eq!(after.mission, before_mission);
        assert_eq!(after.suspended_attack_target, Some(TargetKind::Entity(7)));
        assert_eq!(
            after.navigation.suspended_nav_com,
            Some(NavTargetRef::cell(1, 2))
        );
        assert_eq!(
            after.attack_target.as_ref().map(|target| target.target),
            Some(TargetKind::Entity(8))
        );
        assert_eq!(after.navigation.nav_com, Some(NavTargetRef::cell(8, 9)));
        assert!(matches!(
            effects.events.as_slice(),
            [
                ConcreteEffectEvent::Preflight { .. },
                ConcreteEffectEvent::Target {
                    mission_current,
                    suspended_mission,
                    ..
                },
                ConcreteEffectEvent::Destination { .. }
            ] if *mission_current == deliberate && *suspended_mission == MissionId::NONE
        ));
    }

    #[test]
    fn blocked_aircraft_override_has_empty_trace_and_byte_identical_state() {
        let mut aircraft = entity(EntityCategory::Aircraft, MissionId::from_raw(4));
        aircraft.mission_leaf = MissionLeafState::aircraft_raw_for_test(9, 1, false);
        aircraft.attack_target = Some(AttackTarget::new(7));
        aircraft.navigation.nav_com = Some(NavTargetRef::cell(1, 2));
        let before = bincode::serialize(&aircraft).expect("serialize blocked Aircraft");
        let mut sim = sim_with(aircraft);
        let mut effects = RecordingConcreteMissionEffects::available();

        sim.mission_override_exact_with_effects(
            1,
            OverridePacket {
                mission: MissionId::NONE,
                combat_target: Some(TargetKind::Entity(8)),
                destination: Some(NavTargetRef::cell(8, 9)),
            },
            &mut effects,
        )
        .unwrap();

        let after =
            bincode::serialize(sim.substrate.entities.get(1).unwrap()).expect("serialize result");
        assert_eq!(after, before);
        assert!(effects.events.is_empty());
    }

    #[test]
    fn restore_empty_does_not_require_concrete_provider() {
        let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
        assert!(!sim.mission_restore_exact(1).unwrap());
    }

    #[test]
    fn restore_success_unavailable_does_not_pop_mission() {
        let mut unit = entity(EntityCategory::Unit, ATTACK);
        unit.mission.apply_test_fixture(MissionTestFixture {
            current: ATTACK,
            suspended: GUARD,
            queued: MOVE,
            movement_bypass_latch: 9,
            handler_state: 10,
            mission_start_frame: 11,
            ai_counter: 12,
            dispatch_timer: MissionDispatchTimer::from_raw(13, 14),
        });
        let before = unit.mission;
        let mut sim = sim_with(unit);

        assert!(matches!(
            sim.mission_restore_exact(1),
            Err(MissionAuthorityError::AuthorityUnavailable(_))
        ));
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission, before);
    }

    #[test]
    fn restore_provider_order_is_target_then_destination_and_retains_archives() {
        let mut unit = entity(EntityCategory::Unit, ATTACK);
        unit.mission.apply_test_fixture(MissionTestFixture {
            current: ATTACK,
            suspended: GUARD,
            queued: MOVE,
            movement_bypass_latch: 9,
            handler_state: 10,
            mission_start_frame: 11,
            ai_counter: 12,
            dispatch_timer: MissionDispatchTimer::from_raw(13, 14),
        });
        unit.suspended_attack_target = Some(TargetKind::Entity(7));
        unit.navigation.suspended_nav_com = Some(NavTargetRef::cell(8, 9));
        let mut sim = sim_with(unit);
        let mut effects = RecordingConcreteMissionEffects::available();

        assert!(
            sim.mission_restore_exact_with_effects(1, &mut effects)
                .unwrap()
        );
        let entity = sim.substrate.entities.get(1).unwrap();
        assert_eq!(entity.mission.current(), GUARD);
        assert_eq!(entity.mission.queued(), MOVE);
        assert_eq!(entity.suspended_attack_target, Some(TargetKind::Entity(7)));
        assert_eq!(
            entity.navigation.suspended_nav_com,
            Some(NavTargetRef::cell(8, 9))
        );
        assert!(matches!(
            effects.events.as_slice(),
            [
                ConcreteEffectEvent::Preflight { .. },
                ConcreteEffectEvent::Target { .. },
                ConcreteEffectEvent::Destination { .. }
            ]
        ));
    }

    #[test]
    fn missing_receiver_is_atomic_for_every_exact_authority_entry() {
        let mut sim = Simulation::new();
        let readiness = TestReadyProvider::ready(true);

        assert!(matches!(
            sim.mission_assign_exact(99, MOVE, 0),
            Err(MissionAuthorityError::MissingReceiver(99))
        ));
        assert!(matches!(
            sim.mission_queue_exact(99, MOVE, 1, 0, &readiness),
            Err(MissionAuthorityError::MissingReceiver(99))
        ));
        assert!(matches!(
            sim.mission_commence_exact(99, 0),
            Err(MissionAuthorityError::MissingReceiver(99))
        ));
        assert!(matches!(
            sim.mission_override_exact(
                99,
                OverridePacket {
                    mission: MOVE,
                    combat_target: None,
                    destination: None,
                }
            ),
            Err(MissionAuthorityError::MissingReceiver(99))
        ));
        assert!(matches!(
            sim.mission_restore_exact(99),
            Err(MissionAuthorityError::MissingReceiver(99))
        ));
    }

    #[test]
    fn mission_authority_operations_preserve_all_rng_streams() {
        let mut sim = sim_with(entity(EntityCategory::Unit, GUARD));
        let before = (
            sim.scenario_rng.state(),
            sim.main_rng.state(),
            sim.mapgen_rng.state(),
        );
        let readiness = TestReadyProvider::ready(false);

        sim.mission_assign_exact(1, ATTACK, 1).unwrap();
        sim.mission_queue_exact(1, MOVE, 1, 2, &readiness).unwrap();
        let after = (
            sim.scenario_rng.state(),
            sim.main_rng.state(),
            sim.mapgen_rng.state(),
        );
        assert_eq!(after, before);
    }

    #[test]
    fn mission_b8_owner_sequence_refinery_queues_then_sets_latch() {
        let mut sim = sim_with(entity(EntityCategory::Unit, ATTACK));

        sim.mission_refinery_completion_exact(1, 10).unwrap();

        let mission = &sim.substrate.entities.get(1).unwrap().mission;
        assert_eq!(mission.current(), ATTACK);
        assert_eq!(mission.queued(), GUARD);
        assert_eq!(mission.movement_bypass_latch(), 1);
    }

    #[test]
    fn mission_b8_owner_sequence_jumpjet_second_gate_can_commence_and_clear_latch() {
        let mut sim = sim_with(entity(EntityCategory::Unit, ATTACK));
        let readiness = SequencedReadyProvider::new([false, true]);

        sim.mission_jumpjet_move_to_completion_exact(1, 10, &readiness)
            .unwrap();

        let mission = &sim.substrate.entities.get(1).unwrap().mission;
        assert_eq!(readiness.validations.get(), 3);
        assert_eq!(readiness.reads.get(), 2);
        let previews = readiness.missions.borrow();
        assert_eq!(previews[0].current(), ATTACK);
        assert_eq!(previews[0].queued(), GUARD);
        assert_eq!(previews[0].movement_bypass_latch(), 1);
        assert_eq!(previews[1].current(), GUARD);
        assert_eq!(previews[1].queued(), MissionId::NONE);
        assert_eq!(previews[1].movement_bypass_latch(), 1);
        assert_eq!(previews[2].current(), ATTACK);
        assert_eq!(previews[2].queued(), GUARD);
        assert_eq!(previews[2].movement_bypass_latch(), 0);
        assert_eq!(mission.current(), GUARD);
        assert_eq!(mission.queued(), MissionId::NONE);
        assert_eq!(mission.movement_bypass_latch(), 0);
    }

    #[test]
    fn mission_b8_owner_sequence_jumpjet_later_false_leaves_latch_set() {
        let mut sim = sim_with(entity(EntityCategory::Unit, ATTACK));
        let readiness = SequencedReadyProvider::new([false, false]);

        sim.mission_jumpjet_move_to_completion_exact(1, 10, &readiness)
            .unwrap();

        let mission = &sim.substrate.entities.get(1).unwrap().mission;
        assert_eq!(readiness.validations.get(), 3);
        assert_eq!(readiness.reads.get(), 2);
        assert_eq!(mission.current(), ATTACK);
        assert_eq!(mission.queued(), GUARD);
        assert_eq!(mission.movement_bypass_latch(), 1);
    }

    #[test]
    fn mission_b8_owner_sequence_jumpjet_second_preflight_error_is_atomic() {
        let mut sim = sim_with(entity(EntityCategory::Unit, ATTACK));
        let before_mission = sim.substrate.entities.get(1).unwrap().mission;
        let before_leaf = sim.substrate.entities.get(1).unwrap().mission_leaf;
        let readiness = RejectSuccessfulJumpjetPreview {
            validations: Cell::new(0),
            reads: Cell::new(0),
            missions: RefCell::new(Vec::new()),
        };

        assert!(matches!(
            sim.mission_jumpjet_move_to_completion_exact(1, 10, &readiness),
            Err(MissionAuthorityError::Readiness(
                ReadyUnavailable::WorldLookup
            ))
        ));

        let after = sim.substrate.entities.get(1).unwrap();
        assert_eq!(after.mission, before_mission);
        assert_eq!(after.mission_leaf, before_leaf);
        assert_eq!(readiness.validations.get(), 2);
        assert_eq!(readiness.reads.get(), 0);
        let previews = readiness.missions.borrow();
        assert_eq!(previews[0].current(), ATTACK);
        assert_eq!(previews[0].queued(), GUARD);
        assert_eq!(previews[0].movement_bypass_latch(), 1);
        assert_eq!(previews[1].current(), GUARD);
        assert_eq!(previews[1].queued(), MissionId::NONE);
        assert_eq!(previews[1].movement_bypass_latch(), 1);
    }

    #[test]
    fn building_ready_consume_empty_queue_preserves_latch() {
        let mut building = entity(EntityCategory::Structure, GUARD);
        building.mission_leaf = MissionLeafState::building_raw_for_test(1);
        let mut sim = sim_with(building);

        assert!(!sim.mission_try_consume_building_ready_exact(1, 10).unwrap());
        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .mission_leaf
                .as_building()
                .unwrap()
                .ready_latch(),
            1
        );
    }

    #[test]
    fn building_ready_consume_success_clears_latch() {
        let mut building = entity(EntityCategory::Structure, GUARD);
        building.mission_leaf = MissionLeafState::building_raw_for_test(1);
        building.mission.apply_test_fixture(MissionTestFixture {
            current: GUARD,
            suspended: MissionId::NONE,
            queued: MOVE,
            movement_bypass_latch: 9,
            handler_state: 10,
            mission_start_frame: 11,
            ai_counter: 12,
            dispatch_timer: MissionDispatchTimer::from_raw(13, 14),
        });
        let mut sim = sim_with(building);

        assert!(sim.mission_try_consume_building_ready_exact(1, 10).unwrap());
        let entity = sim.substrate.entities.get(1).unwrap();
        assert_eq!(entity.mission.current(), MOVE);
        assert_eq!(entity.mission_leaf.as_building().unwrap().ready_latch(), 0);
    }
}
