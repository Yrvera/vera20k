//! Per-object AI dispatch scaffold (TechnoClass/FootClass spine, Slice S0).
//!
//! Walks the substrate's live-object order and dispatches each live object
//! through a per-`EntityCategory` shell. THIS SLICE the shell is a strict
//! no-op: it visits every live, present, non-dying object exactly once in live
//! order and changes nothing the lockstep hash observes. Later slices replace
//! the no-op arms with the absorbed per-leaf behavior (movement, turret,
//! combat, mission dispatch) one at a time.
//!
//! Depends on: `world::Simulation` (substrate live order + entity store).
//! Must NOT depend on render/ui/sidebar/audio/net (sim invariant #1).
//! Dispatch is `match category` only — no trait object / dyn / vtable
//! (invariant #2). No RNG, no hashed-state mutation, no phase reorder.

use super::Simulation;
use crate::map::entities::EntityCategory;

// Slice S1 (shadow) imports — used only by the `#[cfg(any(test, debug_assertions))]`
// dispatch-before-locomotor observation below; gated to avoid release dead-code.
#[cfg(any(test, debug_assertions))]
use crate::sim::game_entity::GameEntity;
#[cfg(any(test, debug_assertions))]
use crate::sim::mission::MissionType;
#[cfg(any(test, debug_assertions))]
use crate::sim::movement::{DriveProcessOutcome, process_drive_locomotion_shell};

impl Simulation {
    /// Object-AI stage (Slice S0: instrumented no-op).
    ///
    /// Walks the live LogicVector order via `for_each_live_object` — the same
    /// re-read contract the native scheduler uses — and dispatches each live,
    /// present, non-dying object through the no-op `techno_ai_shell`. The shell
    /// does nothing behavior-bearing this slice; the stage exists to pin the
    /// dispatch + ordering scaffold and prove hash-neutrality.
    ///
    /// `record` is true only in debug builds, where the recorded visit trace is
    /// asserted to equal the live (present, non-dying) order — the first
    /// tripwire for any future arm that mutates live membership mid-pass.
    /// Release builds pass `false`, so the trace `Vec` is never pushed to and
    /// never allocates (no per-tick hot-path cost).
    pub(crate) fn object_ai_stage(&mut self) {
        let visited = self.object_ai_walk(cfg!(debug_assertions));

        #[cfg(debug_assertions)]
        debug_assert_eq!(
            visited,
            self.object_ai_live_order_filtered(),
            "object_ai_stage visit order diverged from live LogicVector order",
        );

        #[cfg(not(debug_assertions))]
        let _ = visited;
    }

    /// The walk: dispatch every live, present, non-dying object once, in live
    /// order, through the no-op shell. When `record`, return the dispatched ids
    /// in order (debug/test observation); otherwise the returned `Vec` is empty
    /// and unallocated. Reads only — touches no hashed state, consumes no RNG.
    fn object_ai_walk(&mut self, record: bool) -> Vec<u64> {
        let mut visited: Vec<u64> = Vec::new();
        self.for_each_live_object(|sim, id| {
            // Tolerate an absent id (the loop's documented contract). In S0 the
            // stage runs AFTER the end-of-tick flush_pending_delete drain, so the
            // order should not reference a freed slot — but inherit the guard.
            let Some(entity) = sim.substrate.entities.get(id) else {
                return;
            };
            // A dying object is mid death-teardown and is not dispatched (the
            // closest live `IsActive` analogue today).
            if entity.dying {
                return;
            }
            let category = entity.category;
            if record {
                visited.push(id);
            }
            techno_ai_shell(sim, id, category);
        });
        visited
    }

    /// The ids the walk dispatches, derived independently from the post-pass
    /// live order: present in the store and not dying, in live order. For the
    /// S0 no-op shell this always equals the recorded visit trace; a future arm
    /// that removes/reorders a live object mid-pass would break the equality.
    #[cfg(any(test, debug_assertions))]
    fn object_ai_live_order_filtered(&self) -> Vec<u64> {
        self.live_object_order_snapshot()
            .into_iter()
            .filter(|&id| self.substrate.entities.get(id).is_some_and(|e| !e.dying))
            .collect()
    }
}

/// Per-category dispatch shell. Slice S0: every arm is a strict no-op.
///
/// `match category` — NO trait / dyn / vtable (invariant #2). `sim`/`id` are
/// threaded so later slices can fill an arm with the absorbed behavior without
/// changing this signature. The match is exhaustive over the four real
/// variants (no `_` arm), so a future `EntityCategory` addition is a compile
/// error, intentionally.
#[allow(unused_variables)]
fn techno_ai_shell(sim: &mut Simulation, id: u64, category: EntityCategory) {
    match category {
        EntityCategory::Unit => {}      // S1+: absorb movement/turret/combat/mission dispatch
        EntityCategory::Infantry => {}  // S6: absorb fear / sequence / self-removal
        EntityCategory::Structure => {} // S8: absorb the BuildingClass::Update bracket
        EntityCategory::Aircraft => {}  // S7: absorb per-object aircraft dispatch
    }
}

// ===== Slice S1 — first behavior-bearing ordering (shadow) =====
//
// For one bounded scenario — a moving drive `UnitClass` on a pure Move mission —
// observe the mission decision THEN the locomotor `Process` marker within a
// single object pass, proving `dispatch_seq < process_seq` (the verified gamemd
// ordering: FootClass::AI runs the locomotor AFTER mission dispatch). Read-only,
// debug-only, never hashed — the authority flip is a later slice.

/// The bounded S1 scenario: a moving, drive-locomotor `UnitClass` on a pure
/// Move mission, with no combat, miner, dock, or aircraft concern.
/// `derived_mission()` yields exactly `(MissionType::Move, 0)` for this set.
/// Requiring a drive locomotor narrows the scope to the units the dispatch→
/// process ordering proof targets and makes the `is_drive` marker exact —
/// avoiding a false agreement-assert on a non-drive mover (ship / hover).
#[cfg(any(test, debug_assertions))]
fn is_s1_scoped_move_unit(e: &GameEntity) -> bool {
    e.category == EntityCategory::Unit
        && e.movement_target.is_some()
        && e.drive_locomotion.is_some()
        && e.miner.is_none()
        && e.dock_state.is_none()
        && e.attack_target.is_none()
        && e.aircraft_mission.is_none()
}

/// Read-only observation of one in-scope object's pass: where the mission
/// decision was observed relative to the locomotor `Process` marker. Never
/// committed to live state or the hash.
#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShellTrace {
    /// Ordinal at which the mission decision was observed this pass.
    dispatch_seq: u32,
    /// Ordinal at which the locomotor `Process` marker was observed.
    process_seq: u32,
    /// The observed mission (must be `Move` for an in-scope unit).
    mission: MissionType,
    /// Whether the locomotor `Process` marker reported a drive unit.
    is_drive: bool,
}

/// S1 shadow step: for one in-scope moving Unit, observe the mission decision
/// and THEN the locomotor `Process` marker within a single object pass, and
/// return the trace. READ-ONLY — `&Simulation`, mutates nothing (no entity, no
/// occupancy, no hash). `seq` is a shared monotonic counter across the pass;
/// `dispatch_seq < process_seq` by construction proves the decision is observed
/// before Process. Returns `None` for any object outside the bounded scope (the
/// over-claim guard) — and never rewrites the observed mission (a divergence is
/// surfaced, never silently equalized).
#[cfg(any(test, debug_assertions))]
fn unit_ai_shadow_step(sim: &Simulation, id: u64, seq: &mut u32) -> Option<ShellTrace> {
    let entity = sim.substrate.entities.get(id)?;
    if !is_s1_scoped_move_unit(entity) {
        return None;
    }
    // Mission dispatch (decision) FIRST. `mission.current` was refreshed by
    // refresh_mission_shadow this tick; reading it is the "decision ran" marker.
    let mission = entity.mission.current;
    let dispatch_seq = *seq;
    *seq += 1;
    // Locomotor Process SECOND — the read-only drive presence marker.
    let outcome = process_drive_locomotion_shell(entity);
    let process_seq = *seq;
    *seq += 1;
    Some(ShellTrace {
        dispatch_seq,
        process_seq,
        mission,
        is_drive: matches!(outcome, DriveProcessOutcome::Processed),
    })
}

impl Simulation {
    /// Debug-only S1 shadow pass: walk the live order and, for each in-scope
    /// moving Unit, assert the mission decision is observed before the locomotor
    /// Process within one object pass (the verified gamemd ordering) and that
    /// the observed mission is the in-scope `Move`. Read-only; never hashed,
    /// never serialized; a divergence is asserted with tick + id, never silently
    /// equalized. (The Slice-2 mission shadow-agreement assert it once mirrored
    /// was retired in Slice 8 when `mission` became hashed-authoritative.)
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_assert_s1_shadow(&self) {
        let mut seq = 0u32;
        for id in self.live_object_order_snapshot() {
            let Some(trace) = unit_ai_shadow_step(self, id, &mut seq) else {
                continue;
            };
            debug_assert!(
                trace.dispatch_seq < trace.process_seq,
                "S1: tick {} unit {}: dispatch_seq {} must precede process_seq {}",
                self.tick,
                id,
                trace.dispatch_seq,
                trace.process_seq,
            );
            debug_assert_eq!(
                trace.mission,
                MissionType::Move,
                "S1: tick {} unit {}: in-scope unit must derive Move, observed {:?}",
                self.tick,
                id,
                trace.mission,
            );
            debug_assert!(
                trace.is_drive,
                "S1: tick {} unit {}: in-scope unit must be a drive mover",
                self.tick,
                id,
            );
        }
    }
}

// ===== P2 (factory substrate) — Structure-arm read-only shadow trace (FIT a) =====
//
// FIT option (a): the per-(house, category) factory step is driven from the
// Structure arm of object_ai_stage() in LogicVector order; the FactoryRegistry is
// a LOOKUP, not a tick-loop owner. In P1+P2 there is no authoritative step, so the
// `EntityCategory::Structure` arm stays a no-op and this debug-only trace records
// each live Structure in LogicVector order — the same "proof lives beside, not
// inside, the no-op arm" shape as the S1 shadow. The order-follows-LogicVector
// property is proven by a test that injects a known non-sorted order
// (`factory_shadow_trace_order_matches_logic_vector`); the runtime debug_assert
// only checks the cheap intrinsic invariants (strictly-increasing visit ordinal;
// each traced id resolves to a live, non-dying Structure). Read-only, never hashed.

/// One Structure visited by the P2 factory shell trace, in LogicVector order.
#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FactoryShellTrace {
    structure_id: u64,
    visit_seq: u32,
}

impl Simulation {
    /// Build the P2 factory shell trace: each live, non-dying Structure in
    /// LogicVector order. Read-only; never hashed, never serialized. The order IS
    /// LogicVector order by construction (it walks `live_object_order_snapshot`) —
    /// the FIT-(a) ordering, exercised by the injected-order test.
    #[cfg(any(test, debug_assertions))]
    fn factory_shell_trace(&self) -> Vec<FactoryShellTrace> {
        let mut seq = 0u32;
        let mut traces: Vec<FactoryShellTrace> = Vec::new();
        for id in self.live_object_order_snapshot() {
            let is_live_structure = self
                .substrate
                .entities
                .get(id)
                .is_some_and(|e| !e.dying && e.category == EntityCategory::Structure);
            if !is_live_structure {
                continue;
            }
            traces.push(FactoryShellTrace {
                structure_id: id,
                visit_seq: seq,
            });
            seq += 1;
        }
        traces
    }

    /// Test-only accessor: the structure ids the P2 trace visits, in order. The
    /// test injects a non-sorted live order and asserts this equals it (so it
    /// would fail if the trace used BTreeMap/entity-id order instead).
    #[cfg(test)]
    pub(crate) fn factory_shell_trace_order(&self) -> Vec<u64> {
        self.factory_shell_trace()
            .iter()
            .map(|t| t.structure_id)
            .collect()
    }

    /// Debug-only P2 assert: the factory shell trace visits live, non-dying
    /// Structures with a strictly-increasing visit ordinal. INTRINSIC invariants
    /// only — not a self-comparison; the LogicVector-order property is proven by a
    /// dedicated injected-order test, never re-derived here.
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_assert_factory_shell_trace(&self) {
        let traces = self.factory_shell_trace();
        for w in traces.windows(2) {
            debug_assert!(
                w[0].visit_seq < w[1].visit_seq,
                "P2: tick {}: factory shell trace visit_seq must strictly increase",
                self.tick,
            );
        }
        for t in &traces {
            debug_assert!(
                self.substrate
                    .entities
                    .get(t.structure_id)
                    .is_some_and(|e| !e.dying && e.category == EntityCategory::Structure),
                "P2: tick {}: factory shell trace id {} must resolve to a live Structure",
                self.tick,
                t.structure_id,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::aircraft::AircraftMission;
    use crate::sim::combat::{AttackTarget, TargetKind};
    use crate::sim::components::{DriveLocomotionRuntime, MovementTarget};
    use crate::sim::docking::building_dock::{DockPhase, DockState};
    use crate::sim::game_entity::GameEntity;
    use crate::sim::miner::{Miner, MinerConfig, MinerKind};
    use crate::sim::mission::MissionType;

    /// Build a test entity of a specific category (`test_default` makes a Unit).
    fn entity_of(id: u64, category: EntityCategory) -> GameEntity {
        let mut e = GameEntity::test_default(id, "TEST", "Americans", 5, 5);
        e.category = category;
        e
    }

    #[test]
    fn techno_ai_shell_is_passthrough_no_hash_change() {
        // Mirrors `mission_shadow_does_not_change_state_hash`: the no-op stage,
        // walking live order and dispatching all four category arms, must leave
        // the lockstep hash bit-identical.
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        sim.substrate
            .entities
            .insert(entity_of(2, EntityCategory::Infantry));
        sim.substrate
            .entities
            .insert(entity_of(3, EntityCategory::Structure));
        sim.substrate
            .entities
            .insert(entity_of(4, EntityCategory::Aircraft));
        sim.set_logic_order_for_test(vec![1, 2, 3, 4]);

        let before = sim.state_hash();
        sim.object_ai_stage();
        let after = sim.state_hash();
        assert_eq!(
            before, after,
            "object_ai_stage (S0 no-op) must not perturb the state hash"
        );
    }

    #[test]
    fn techno_ai_shell_membership_matches_phase_snapshot() {
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        sim.substrate
            .entities
            .insert(entity_of(2, EntityCategory::Structure));
        sim.substrate
            .entities
            .insert(entity_of(3, EntityCategory::Aircraft));
        // Deliberately NON-sorted order to prove the walk preserves live order
        // verbatim (no sort).
        sim.set_logic_order_for_test(vec![3, 1, 2]);

        let visited = sim.object_ai_walk(true);
        assert_eq!(
            visited,
            sim.live_object_order_snapshot(),
            "every live object visited exactly once, in live order"
        );
        assert_eq!(visited, vec![3, 1, 2], "live order preserved verbatim (no sort)");
    }

    #[test]
    fn techno_ai_shell_preserves_advance_tick_phase_order() {
        // The stage is wired into advance_tick (called every tick, before
        // refresh_mission_shadow). Identical fixtures must produce identical
        // per-tick state_hash sequences — the stage introduces no nondeterminism
        // and no panic. Together with the hash-neutrality proof
        // (techno_ai_shell_is_passthrough_no_hash_change, which exercises the
        // entity walk directly) this shows the new stage perturbs no phase and
        // no surrounding ordering. The fixture is intentionally entity-free:
        // raw test_default entities carry interned ids that advance_tick's
        // entity systems would resolve against an empty interner (a fixture
        // concern unrelated to the stage); the stage still runs each tick over
        // the empty live order.
        fn run() -> Vec<u64> {
            let mut sim = Simulation::new();
            let heights = std::collections::BTreeMap::new();
            (0..5)
                .map(|_| {
                    sim.advance_tick(&[], None, &heights, None, None, 67);
                    sim.state_hash()
                })
                .collect()
        }
        assert_eq!(
            run(),
            run(),
            "advance_tick with the object-AI stage stays deterministic"
        );
    }

    #[test]
    fn object_ai_stage_skips_dying_object() {
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(entity_of(1, EntityCategory::Unit));
        sim.substrate
            .entities
            .insert(entity_of(2, EntityCategory::Unit));
        sim.set_logic_order_for_test(vec![1, 2]);
        // Mark id 2 dying AFTER set_logic_order_for_test — that helper resets
        // presence / in_logic_vector but does NOT touch `dying`, and id 2 stays
        // in the live order.
        sim.substrate.entities.get_mut(2).unwrap().dying = true;

        let visited = sim.object_ai_walk(true);
        assert_eq!(
            visited,
            vec![1],
            "dying object skipped; the live object is still visited"
        );
        // The internal order-proof assert filters dying members, so the stage
        // must not panic even with a dying member in the live order.
        sim.object_ai_stage();
    }

    #[test]
    fn object_ai_stage_tolerates_absent_id_in_order() {
        let mut sim = Simulation::new();
        let live_id = 1u64;
        let absent_id = 999u64;
        sim.substrate
            .entities
            .insert(entity_of(live_id, EntityCategory::Unit));
        // Force the live order to include an id with no entity in the store
        // (set_logic_order_for_test only flips flags on existing ids, so set the
        // order directly to keep the absent id a non-member with no entity).
        sim.substrate
            .logic
            .set_order_for_test(vec![absent_id, live_id]);

        let visited = sim.object_ai_walk(true);
        assert_eq!(
            visited,
            vec![live_id],
            "absent id skipped without panic; live id still visited"
        );
        // Stage must not panic on the absent member either.
        sim.object_ai_stage();
    }

    // ===== Slice S1 — dispatch-before-locomotor shadow =====

    /// A bounded-S1-scoped unit: a moving drive `UnitClass` with no combat,
    /// miner, dock, or aircraft concern. `derived_mission()` yields `(Move, 0)`.
    fn scoped_move_unit(id: u64) -> GameEntity {
        let mut e = GameEntity::test_default(id, "TEST", "Americans", 5, 5); // category Unit
        e.movement_target = Some(MovementTarget::default());
        e.drive_locomotion = Some(DriveLocomotionRuntime::default());
        e
    }

    #[test]
    fn unit_ai_mission_dispatch_precedes_locomotor_process() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.set_logic_order_for_test(vec![1]);
        sim.refresh_mission_shadow(); // mission.current = derived_mission() = Move

        let mut seq = 0u32;
        let trace =
            unit_ai_shadow_step(&sim, 1, &mut seq).expect("a scoped move unit yields a trace");
        assert!(
            trace.dispatch_seq < trace.process_seq,
            "mission dispatch must be observed before the locomotor Process"
        );
        assert_eq!(trace.mission, MissionType::Move);
        assert!(trace.is_drive);
    }

    #[test]
    fn unit_move_dispatch_then_process_shadow_agrees() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.substrate.entities.insert(scoped_move_unit(2));
        sim.set_logic_order_for_test(vec![1, 2]);
        sim.refresh_mission_shadow();

        // Agreement: every in-scope unit derives Move / is_drive / dispatch <
        // process, so the shadow pass asserts cleanly (no divergence, no panic).
        sim.debug_assert_s1_shadow();

        // Divergence is SURFACED, not equalized: force an in-scope unit's mission
        // to a non-Move value after the refresh; the step returns the OBSERVED
        // mission (Guard), it does not rewrite it to Move.
        sim.substrate.entities.get_mut(1).unwrap().mission.current = MissionType::Guard;
        let mut seq = 0u32;
        let trace = unit_ai_shadow_step(&sim, 1, &mut seq).expect("still in scope");
        assert_eq!(
            trace.mission,
            MissionType::Guard,
            "shadow surfaces the observed mission, never silently equalizes to Move"
        );
    }

    #[test]
    fn s1_no_hash_change_shadow() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.set_logic_order_for_test(vec![1]);
        sim.refresh_mission_shadow();

        let before = sim.state_hash();
        sim.debug_assert_s1_shadow(); // read-only shadow pass
        let after = sim.state_hash();
        assert_eq!(before, after, "the S1 shadow pass must not perturb the state hash");
    }

    #[test]
    fn s1_shadow_skips_non_scoped_units() {
        let mut sim = Simulation::new();

        // Miner (highest derived-mission priority) — disqualified.
        let mut miner = scoped_move_unit(1);
        miner.miner = Some(Miner::new(MinerKind::War, &MinerConfig::default(), 0));
        sim.substrate.entities.insert(miner);

        // Docking unit — disqualified.
        let mut docking = scoped_move_unit(2);
        docking.dock_state = Some(DockState {
            dock_building_id: 99,
            phase: DockPhase::Approach,
            service_timer: 0,
            no_funds_ticks: 0,
        });
        sim.substrate.entities.insert(docking);

        // Attacking unit — disqualified.
        let mut attacking = scoped_move_unit(3);
        attacking.attack_target = Some(AttackTarget {
            target: TargetKind::Entity(99),
            cooldown_ticks: 0,
            burst_remaining: 1,
            burst_delay_ticks: 0,
            pending_infantry_fire: None,
        });
        sim.substrate.entities.insert(attacking);

        // Aircraft — disqualified by the category gate.
        let mut aircraft = scoped_move_unit(4);
        aircraft.category = EntityCategory::Aircraft;
        aircraft.aircraft_mission = Some(AircraftMission::Guard);
        sim.substrate.entities.insert(aircraft);

        let mut seq = 0u32;
        for id in [1u64, 2, 3, 4] {
            assert!(
                unit_ai_shadow_step(&sim, id, &mut seq).is_none(),
                "non-scoped object {id} must be skipped by the S1 shadow"
            );
        }
        assert_eq!(seq, 0, "skipped objects never advance the ordinal counter");
    }

    #[test]
    fn s1_shadow_preserves_advance_tick_phase_order() {
        // The shadow runs (debug builds) inside advance_tick, between
        // refresh_mission_shadow and state_hash. Identical fixtures must produce
        // identical per-tick hash sequences — the read-only shadow perturbs no
        // phase. Entity-free for the same interner reason as
        // techno_ai_shell_preserves_advance_tick_phase_order.
        fn run() -> Vec<u64> {
            let mut sim = Simulation::new();
            let heights = std::collections::BTreeMap::new();
            (0..5)
                .map(|_| {
                    sim.advance_tick(&[], None, &heights, None, None, 67);
                    sim.state_hash()
                })
                .collect()
        }
        assert_eq!(run(), run());
    }
}
