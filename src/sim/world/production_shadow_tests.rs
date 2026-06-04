//! P1+P2 production+economy shadow tests (study §8 P1/P2 + the design proving set).
//!
//! These exercise the shadow build from the `world` level, where `Simulation::new`,
//! `advance_tick`, `state_hash`, `set_logic_order_for_test`, and the snapshot API
//! are reachable. The contract these pin: the shadow is DERIVED from the legacy
//! state, NEVER creates a house, and leaves `state_hash()` bit-identical (the new
//! fields are `#[serde(skip)]` with no serde derive, so `SNAPSHOT_VERSION` stays 17).

use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::economy::Economy;
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::intern::InternedId;
use crate::sim::production::{
    BuildQueueItem, BuildQueueState, CancelOutcome, ProductionCategory, StepOutcome,
    PRODUCTION_STEPS,
};
use std::collections::{BTreeMap, VecDeque};

fn empty_rules() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str("")).expect("empty rules parse")
}

/// Rules with exactly one OrePurifier building type (`GAPROC`). Used by the
/// purifier-count guard — proves the base is the building COUNT (the v2 finding),
/// not silo storage capacity.
fn rules_with_ore_purifier() -> RuleSet {
    RuleSet::from_ini(&IniFile::from_str(
        "[BuildingTypes]\n1=GAPROC\n[GAPROC]\nOrePurifier=yes\n",
    ))
    .expect("ore-purifier rules parse")
}

fn queued_item(
    owner: InternedId,
    ty: InternedId,
    cat: ProductionCategory,
    state: BuildQueueState,
    total: u32,
    remaining: u32,
    order: u64,
) -> BuildQueueItem {
    BuildQueueItem {
        owner,
        type_id: ty,
        queue_category: cat,
        state,
        total_base_frames: total,
        remaining_base_frames: remaining,
        progress_carry: 0,
        enqueue_order: order,
    }
}

fn insert_queue(sim: &mut Simulation, owner: InternedId, cat: ProductionCategory, item: BuildQueueItem) {
    let mut dq = VecDeque::new();
    dq.push_back(item);
    let mut cats = BTreeMap::new();
    cats.insert(cat, dq);
    sim.production.queues_by_owner.insert(owner, cats);
}

// ===== P1 — Economy shadow =====

#[test]
fn economy_shadow_tracks_legacy_credits() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let a = sim.interner.intern("Americans");
    let b = sim.interner.intern("Russians");
    sim.houses.insert(a, HouseState::new(a, 0, None, true, 5000, 10));
    sim.houses.insert(b, HouseState::new(b, 1, None, true, 1234, 10));
    sim.refresh_economy_shadow(Some(&rules));
    assert_eq!(sim.houses[&a].economy.credits, 5000);
    assert_eq!(sim.houses[&b].economy.credits, 1234);
    assert_eq!(sim.houses[&a].economy.purifier_count, 0);
}

#[test]
fn economy_shadow_does_not_create_houses() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let before_len = sim.houses.len();
    let before = sim.state_hash();
    sim.refresh_economy_shadow(Some(&rules));
    assert_eq!(sim.houses.len(), before_len, "shadow must not create houses");
    assert_eq!(before, sim.state_hash(), "shadow must not perturb the hash");
}

#[test]
fn economy_shadow_does_not_change_state_hash() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let a = sim.interner.intern("Americans");
    sim.houses.insert(a, HouseState::new(a, 0, None, true, 5000, 10));
    let before = sim.state_hash();
    sim.refresh_economy_shadow(Some(&rules));
    let after = sim.state_hash();
    assert_eq!(before, after, "economy shadow must not perturb the state hash");
}

/// CONCERN-1 regression guard for the v2 correction: the purifier-bonus base is the
/// OrePurifier *building count* (NOT silo storage capacity). Would fail if the impl
/// ever modeled storage capacity (it counts owned OrePurifier structures).
#[test]
fn economy_purifier_count_is_building_count() {
    let mut sim = Simulation::new();
    let rules = rules_with_ore_purifier();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 5000, 10));
    let proc_ty = sim.interner.intern("GAPROC");
    let powr_ty = sim.interner.intern("GAPOWR"); // not a purifier (absent from rules)

    let mut add_structure = |sim: &mut Simulation, id: u64, ty: InternedId, rx: u16| {
        let mut e = GameEntity::test_default(id, "GAPROC", "Americans", rx, 5);
        e.category = EntityCategory::Structure;
        e.owner = owner;
        e.type_ref = ty;
        sim.substrate.entities.insert(e);
    };
    // One purifier -> count 1.
    add_structure(&mut sim, 1, proc_ty, 5);
    sim.refresh_economy_shadow(Some(&rules));
    assert_eq!(sim.houses[&owner].economy.purifier_count, 1);

    // Second purifier + a non-purifier structure -> count 2 (the power plant is
    // NOT counted, proving it tracks OrePurifier buildings specifically).
    add_structure(&mut sim, 2, proc_ty, 6);
    add_structure(&mut sim, 3, powr_ty, 7);
    sim.refresh_economy_shadow(Some(&rules));
    assert_eq!(
        sim.houses[&owner].economy.purifier_count, 2,
        "purifier_count is the OrePurifier building count, not storage capacity"
    );
}

// ===== P2 — Factory / FactoryRegistry shadow =====

#[test]
fn factory_shadow_progress_tracks_legacy_remaining() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    // Half-built: 54 total frames, 27 remaining -> progress 27.
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 27, 1),
    );
    sim.refresh_production_shadow(Some(&rules));
    let view = sim
        .production
        .factory_shadow
        .view(owner, ProductionCategory::Vehicle)
        .expect("factory exists");
    assert_eq!(view.progress, 27, "half-remaining -> half progress");
    assert!(view.object.is_some(), "Building front => active object");

    // Drive remaining to 0 -> progress 54 (completion coincidence).
    sim.production
        .queues_by_owner
        .get_mut(&owner)
        .unwrap()
        .get_mut(&ProductionCategory::Vehicle)
        .unwrap()
        .front_mut()
        .unwrap()
        .remaining_base_frames = 0;
    sim.refresh_production_shadow(Some(&rules));
    let view = sim
        .production
        .factory_shadow
        .view(owner, ProductionCategory::Vehicle)
        .unwrap();
    assert_eq!(view.progress, PRODUCTION_STEPS, "remaining 0 -> progress reaches 54");
}

#[test]
fn factory_registry_iteration_is_insertion_ordered() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    for (i, name) in ["A", "B", "C"].iter().enumerate() {
        let owner = sim.interner.intern(name);
        let ty = sim.interner.intern(&format!("U{i}"));
        let mut cats = BTreeMap::new();
        for cat in [ProductionCategory::Vehicle, ProductionCategory::Infantry] {
            let mut dq = VecDeque::new();
            dq.push_back(queued_item(owner, ty, cat, BuildQueueState::Building, 54, 10, 1));
            cats.insert(cat, dq);
        }
        sim.production.queues_by_owner.insert(owner, cats);
    }
    sim.refresh_production_shadow(Some(&rules));
    let seqs: Vec<u64> = sim
        .production
        .factory_shadow
        .iter_insertion_ordered()
        .iter()
        .map(|f| f.insertion_seq)
        .collect();
    let mut sorted = seqs.clone();
    sorted.sort();
    assert_eq!(seqs, sorted, "iteration is monotonic in insertion_seq");
    assert_eq!(seqs.len(), 6, "3 owners x 2 categories = 6 factories");
}

#[test]
fn insertion_seq_stable_across_rebuild() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    sim.refresh_production_shadow(Some(&rules));
    let seq_a = sim.production.factory_shadow.iter_insertion_ordered()[0].insertion_seq;
    // Advance the build, rebuild — the same (owner, category) survives, same seq.
    sim.production
        .queues_by_owner
        .get_mut(&owner)
        .unwrap()
        .get_mut(&ProductionCategory::Vehicle)
        .unwrap()
        .front_mut()
        .unwrap()
        .remaining_base_frames = 10;
    sim.refresh_production_shadow(Some(&rules));
    let seq_b = sim.production.factory_shadow.iter_insertion_ordered()[0].insertion_seq;
    assert_eq!(seq_a, seq_b, "surviving factory keeps a stable insertion_seq");
}

#[test]
fn factory_registry_shadow_no_hash_change() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    let before = sim.state_hash();
    sim.refresh_production_shadow(Some(&rules));
    let after = sim.state_hash();
    assert_eq!(before, after, "factory shadow rebuild must not perturb the state hash");
}

#[test]
fn production_shadow_does_not_create_houses() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Ghost"); // no HouseState inserted
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    let before_houses = sim.houses.len();
    let before_queues = sim.production.queues_by_owner.len();
    let before = sim.state_hash();
    sim.refresh_production_shadow(Some(&rules));
    assert_eq!(sim.houses.len(), before_houses, "shadow must not create houses");
    assert_eq!(sim.production.queues_by_owner.len(), before_queues, "queues unchanged");
    assert_eq!(before, sim.state_hash(), "hash unchanged");
}

/// FIT (a): the factory shell trace visits live Structures in LogicVector order.
/// The injected order [3, 1, 2] is NOT entity-id-sorted ([1, 2, 3]) — so an equal
/// assertion proves the trace follows LogicVector order, not BTreeMap/id order.
#[test]
fn factory_shadow_trace_order_matches_logic_vector() {
    let mut sim = Simulation::new();
    for id in [3u64, 1, 2] {
        let mut e = GameEntity::test_default(id, "GAPOWR", "Americans", 5, 5);
        e.category = EntityCategory::Structure;
        sim.substrate.entities.insert(e);
    }
    sim.set_logic_order_for_test(vec![3, 1, 2]);
    assert_eq!(
        sim.factory_shell_trace_order(),
        vec![3, 1, 2],
        "trace follows LogicVector order, not entity-id/map order"
    );
    sim.debug_assert_factory_shell_trace(); // intrinsic invariants must hold
}

#[test]
fn snapshot_roundtrip_ignores_shadow() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 5000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    sim.refresh_production_shadow(Some(&rules));
    let hash_before = sim.state_hash();

    let bytes = crate::sim::snapshot::GameSnapshot::save(&sim, 0, 0, "test_map", 0);
    let restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
        .expect("load")
        .sim;

    assert_eq!(
        restored.houses[&owner].economy,
        Economy::default(),
        "skipped economy comes back Default"
    );
    assert!(
        restored.production.factory_shadow.is_empty(),
        "skipped factory_shadow comes back Default (empty)"
    );
    assert_eq!(
        restored.state_hash(),
        hash_before,
        "hash unchanged across the round-trip (shadow not load-bearing)"
    );
}

/// Identical fixtures over N ticks produce identical per-tick state_hash sequences
/// (the production shadow keeps advance_tick deterministic).
#[test]
fn production_shadow_preserves_advance_tick_phase_order() {
    fn run() -> Vec<u64> {
        let mut sim = Simulation::new();
        let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        (0..5)
            .map(|_| {
                sim.advance_tick(&[], None, &heights, None, None, 67);
                sim.state_hash()
            })
            .collect()
    }
    assert_eq!(run(), run(), "advance_tick with the production shadow stays deterministic");
}

// ===== P3 — per-step charge oracle (hash-neutral) =====

/// P3 no-hash guarantee: stepping a CLONE of a shadow factory against a CLONE of the
/// wallet 54 times leaves `state_hash()` bit-identical (the oracle never touches the
/// hashed wallet; `Factory`/`Economy` carry no serde derive). The acceptance test.
#[test]
fn factory_advance_step_does_not_change_state_hash() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    sim.refresh_production_shadow(Some(&rules)); // cost-based shadow built
    let before = sim.state_hash();

    // Step a CLONE of the shadow factory against a CLONE of the wallet, 54 times.
    let mut f = sim.production.factory_shadow.iter_insertion_ordered()[0].clone();
    // empty_rules -> cost 0; seed a real cost so the step machine actually charges.
    f.progress = 0;
    f.balance = 700;
    f.original_balance = 700;
    let mut oracle = sim.houses[&owner].economy.clone();
    for _ in 0..PRODUCTION_STEPS {
        let _ = f.advance_one_step(&mut oracle);
    }

    assert_eq!(
        before,
        sim.state_hash(),
        "P3 oracle stepping must not perturb the state hash (serde-skip + clone)"
    );
    assert_eq!(
        sim.houses[&owner].credits, 1_000_000,
        "the legacy wallet is untouched by oracle stepping"
    );
}

/// P3 determinism: identical fixtures over N ticks (with the cost-based rebuild +
/// the conservation assert active in debug) produce identical per-tick state_hash
/// sequences. `rules` is the 2nd positional arg to `advance_tick`.
#[test]
fn production_shadow_with_oracle_is_deterministic() {
    fn run() -> Vec<u64> {
        let mut sim = Simulation::new();
        let rules = empty_rules();
        let owner = sim.interner.intern("Americans");
        sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
        let ty = sim.interner.intern("GRIZZLY");
        insert_queue(
            &mut sim,
            owner,
            ProductionCategory::Vehicle,
            queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
        );
        let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        (0..5)
            .map(|_| {
                sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
                sim.state_hash()
            })
            .collect()
    }
    assert_eq!(run(), run(), "advance_tick with the P3 oracle path stays deterministic");
}

/// The FIT-(a) probe: build a cost-based shadow, place a live Structure for the
/// owner, and confirm the oracle probe steps a clone per (live structure, owner
/// factory-with-object) — read-only, deterministic, hash-neutral.
#[test]
fn factory_oracle_step_trace_walks_live_structures() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    // A live Structure for the owner (the war factory the probe walks).
    let mut e = GameEntity::test_default(1, "GAWEAP", "Americans", 5, 5);
    e.category = EntityCategory::Structure;
    e.owner = owner;
    sim.substrate.entities.insert(e);
    sim.set_logic_order_for_test(vec![1]);
    sim.refresh_production_shadow(Some(&rules));

    let before = sim.state_hash();
    let trace = sim.factory_oracle_step_trace();
    assert_eq!(trace.len(), 1, "one live structure x one owner factory-with-object");
    assert_eq!(trace[0].0, 1, "the outcome is attributed to the live structure id");
    assert_eq!(before, sim.state_hash(), "the probe must not perturb the hash");
    assert_eq!(
        trace,
        sim.factory_oracle_step_trace(),
        "the probe is deterministic across calls"
    );
}

// ===== P4 — FIFO queue + cancel + partial refund (hash-neutral oracle) =====

/// P4 no-hash guarantee (mirrors `factory_advance_step_does_not_change_state_hash`):
/// cancelling a mid-build active object on a CLONE of the registry against a CLONE of
/// the wallet leaves `state_hash()` bit-identical. With `empty_rules` the cost is 0
/// (refund 0); the contract here is the HASH + legacy-wallet invariants, which hold
/// regardless of the refund value (the nonzero refund is proven in the pure
/// `cancel_one_active_when_no_queued_copy` test).
#[test]
fn factory_cancel_one_does_not_change_state_hash() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    sim.refresh_production_shadow(Some(&rules)); // cost-based shadow built
    let before = sim.state_hash();
    let legacy_credits = sim.houses[&owner].credits;

    // Cancel (active abandon, mid-build) against a CLONE of the registry + a CLONE of
    // the wallet; the active GRIZZLY has no queued copy, so the active-abandon branch
    // fires (AbandonedActive).
    let mut reg = sim.production.factory_shadow.clone();
    let mut oracle = sim.houses[&owner].economy.clone();
    let outcome = reg.cancel_one(owner, ProductionCategory::Vehicle, ty, &mut oracle);
    assert!(
        matches!(outcome, CancelOutcome::AbandonedActive { .. }),
        "the active build (no queued copy) is abandoned on the clone"
    );

    assert_eq!(
        before,
        sim.state_hash(),
        "P4 cancel on a clone must not perturb the state hash (serde-skip + clone)"
    );
    assert_eq!(
        sim.houses[&owner].credits, legacy_credits,
        "the legacy wallet is untouched by the oracle cancel"
    );
}

/// P4 C7/C12: completion suspends with the object attached; `start_next_queued` does
/// NOT advance while the object is held; only after the object is CLEARED (simulating
/// the delivery commit, a later slice) does the queue front advance. Proves the
/// negative invariant end-to-end WITHOUT wiring delivery. Driven on a CLONE.
#[test]
fn queue_advances_only_after_delivery() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let active = sim.interner.intern("GRIZZLY");
    let next = sim.interner.intern("FV"); // the queued tail item
    // Front Building (active object) with a tail item behind it.
    let mut dq = VecDeque::new();
    dq.push_back(queued_item(owner, active, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
    dq.push_back(queued_item(owner, next, ProductionCategory::Vehicle, BuildQueueState::Queued, 54, 54, 2));
    let mut cats = BTreeMap::new();
    cats.insert(ProductionCategory::Vehicle, dq);
    sim.production.queues_by_owner.insert(owner, cats);
    sim.refresh_production_shadow(Some(&rules));

    let before = sim.state_hash();
    let mut f = sim.production.factory_shadow.iter_insertion_ordered()[0].clone();
    assert_eq!(f.object.as_ref().map(|o| o.type_id), Some(active), "active = GRIZZLY");
    assert_eq!(f.queue.iter().copied().collect::<Vec<_>>(), vec![next], "tail = [FV]");
    // empty_rules -> cost 0; seed a real cost so completion takes the full ladder.
    f.progress = 0;
    f.balance = 700;
    f.original_balance = 700;
    let mut oracle = sim.houses[&owner].economy.clone();
    loop {
        if matches!(f.advance_one_step(&mut oracle), StepOutcome::Completed) {
            break;
        }
    }
    assert!(f.suspended && f.object.is_some(), "C12: completion holds the object, suspended");
    // The queue does NOT advance on completion alone.
    assert_eq!(f.start_next_queued(), None, "C7: held object blocks the advance");
    assert_eq!(
        f.queue.iter().copied().collect::<Vec<_>>(),
        vec![next],
        "queue front unchanged while the object is held"
    );
    // Simulate the delivery commit: clear the object, THEN the queue advances.
    f.object = None;
    f.suspended = false;
    assert_eq!(f.start_next_queued(), Some(next), "after delivery the front pops");
    assert_eq!(f.object.as_ref().map(|o| o.type_id), Some(next), "active = FV");
    assert!(f.queue.is_empty(), "tail consumed");

    assert_eq!(before, sim.state_hash(), "the clone drive must not perturb the hash");
}

/// P4 determinism: identical fixtures over N ticks with a per-tick cancel probe on
/// CLONES produce identical per-tick state_hash sequences (mirrors
/// `production_shadow_with_oracle_is_deterministic`).
#[test]
fn production_shadow_with_cancel_is_deterministic() {
    fn run() -> Vec<u64> {
        let mut sim = Simulation::new();
        let rules = empty_rules();
        let owner = sim.interner.intern("Americans");
        sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
        let ty = sim.interner.intern("GRIZZLY");
        insert_queue(
            &mut sim,
            owner,
            ProductionCategory::Vehicle,
            queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
        );
        let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        (0..5)
            .map(|_| {
                sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
                // Per-tick clone cancel probe (NEVER written back).
                let mut reg = sim.production.factory_shadow.clone();
                let mut oracle = sim
                    .houses
                    .get(&owner)
                    .map(|h| h.economy.clone())
                    .unwrap_or_default();
                let _ = reg.cancel_one(owner, ProductionCategory::Vehicle, ty, &mut oracle);
                sim.state_hash()
            })
            .collect()
    }
    assert_eq!(run(), run(), "advance_tick with the P4 clone cancel probe stays deterministic");
}
