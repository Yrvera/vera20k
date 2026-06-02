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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::game_entity::GameEntity;

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
}
