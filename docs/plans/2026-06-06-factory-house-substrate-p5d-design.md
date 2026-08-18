---
title: Factory/House Production Substrate — P5d (retire queues_by_owner into Factory.queue) — design + plan
date: 2026-06-06
status: design VALIDATED by adversarial review (ready after the blocker + 2 high fixes below are folded in — they ARE folded in here). Produced by the p5d-queue-of-record-retirement dynamic workflow (6 understand lanes + synthesis + adversarial critic); full raw output in the wf_95e24299-06e task transcript.
scope: P5d ONLY — retire the BuildQueueItem mirror / queues_by_owner queue-of-record by moving FIFO membership + enqueue_order INTO the registry (Factory head + Factory.queue tail). Own SNAPSHOT_VERSION 18->19. Preserve bit-for-bit: lockstep determinism, the P5c acceptance gates, C6/C7/C8 + one-wallet semantics, and the player-visible sidebar build-queue display.
OUT: matching_factory_count_for_owner MultipleFactory rescan retirement (see DRIFT-1 — the registry key-count CANNOT supply it; its own later slice); active_producer_by_owner removal; Ship category (D2).
rule: Rust-native structure, gamemd-native semantics. sim/ never depends on render/ui/etc. All sim math fixed-point; deterministic BTreeMap/sorted iteration; the hash fold order == iter_insertion_ordered == step_all charge order is the contract.
---

# P5d — queues_by_owner → Factory.queue

## 0. Verdict + the shape

Move the queue-of-record into the registry. The §11 P5b prescription `VecDeque<(InternedId,u64)>`
is INSUFFICIENT (no `total_base_frames`, which is hashed + the live sidebar-ETA basis). The entry is a
3-field struct; per-item `state` is NOT stored — it is DERIVED at view time.

```rust
// factory.rs — one queued (not-yet-active) build behind the active object.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QueueEntry {
    pub type_id: InternedId,
    pub enqueue_order: u64,
    pub total_base_frames: u32,
}
```

`Factory` deltas: `queue: VecDeque<InternedId>` → `VecDeque<QueueEntry>`; NEW `active_total_base_frames: u32`
(the active build's ETA basis — today `BuildQueueItem.total_base_frames` of the front); `insertion_seq`
unchanged (== active build's enqueue_order, D1).

**Retired:** `BuildQueueItem`, `ProductionState.queues_by_owner`, `reconcile_from_queues`,
`pop_completed_front`, `refresh_queue_states`, `prune_empty_queues`, `queues_for_owner_mut`,
`queue_for_owner_category_mut`, and the `hash_production` per-item fold.
**Activated:** `Factory::start_next_queued` (dormant — no production caller today) becomes the C7
delivery/abandon advance.
**Kept:** `next_enqueue_order` (mint) on ProductionState, hashed; `ready_by_owner` /
`active_producer_by_owner` (NOT P5d — must survive the fold deletion intact); `matching_factory_count_for_owner`.

## 1. The 18->19 hash fold (world_hash.rs)

DELETE the `hash_production` per-`BuildQueueItem` loop. KEEP ready_by_owner / active_producer_by_owner /
next_enqueue_order / resources EXACTLY. Rewrite `hash_factory_registry` — field order preserved through
`manual`/`special`; insert `active_total_base_frames` after `original_balance`; the tail fold becomes per-QueueEntry:

```
for f in iter_insertion_ordered() {
    owner; (category as u8); insertion_seq; progress; step_rate_frames; step_timer;
    balance; original_balance; active_total_base_frames;        // NEW
    object presence-tag (Some: type_id + entity_id-tag / None: 0u8);   // unchanged
    on_hold; suspended; manual;
    special 3-state (NoneNeg1=0/NoneZero=1/Item(v)=2,v);        // unchanged, never collapse
    queue.len() as u64;
    for e in &queue { e.type_id; e.enqueue_order; e.total_base_frames }   // CHANGED
}
```
Tail folds in VecDeque (FIFO) order — deterministic, no sort. SNAPSHOT_VERSION 18->19; flip
`snapshot_version_is_18`→`_is_19`; history comment. `remaining_base_frames` is GONE (derived at view time,
not hashed) — add an inline comment so a future audit doesn't flag it as dropped-but-written (LOW-7).

## 2. Command-path rebinds

- **enqueue_by_type**: keep all validation incl. affordability gate + `total_base_frames`. Replace the
  push_back(BuildQueueItem) + refresh_queue_states with `registry.enqueue(key, type_id, seq, total_base_frames, cost)`:
  no factory → create + seed the active build inline (object=Some, progress=0, balance=original_balance=cost,
  active_total_base_frames=total, insertion_seq=seq, queue empty); factory exists → `queue.push_back(QueueEntry{..})`.
  Cost resolved by the caller (holds &rules). Defensive: object.is_none() factory → treat as create arm.
- **cancel_by_type_for_owner**: delete the mirror block. registry_cancel_active (wallet-shim) unchanged.
  On AbandonedActive{refund}: call the cost-seeded `start_next_queued` (C7 abandon advance) so a tail item
  becomes active; then prune if object.is_none() && queue.is_empty(). NoMatch → fall through to
  cancel_ready_by_type_for_owner (unchanged).
- **cancel_last_for_owner**: scan the owner's factories; candidate stamp = `f.queue.back().map(|e| e.enqueue_order).unwrap_or(f.insertion_seq)`;
  pick the global MAX via max_by_key (stamps unique → tie-break moot). If that factory's queue non-empty →
  pop_back (uncharged, no refund); else registry_cancel_active (C8 partial refund), NO advance, prune.
- **toggle_pause_for_owner_category**: replace front.state toggle + refresh with `f.manual = !f.manual`
  on the (owner,category) factory (meaningful when object held). step_all already skips manual; resume auto.
- **tick_production_with_overlay_registry (delivery, the closed loop)**: iterate the registry via
  `iter_insertion_ordered()` (NOT BTreeMap key order — same-frame multi-completion order + hash contract).
  Read pass collects completed keys (completion = `progress>=PRODUCTION_STEPS && object.is_some() && !manual`)
  + pre-resolves each next-entry cost; mutate pass spawns/places, then `f.object=None; f.start_next_queued(cost,total)`,
  then prune. DELETE the remaining_base_frames mirror write + the `front.state=Done` mark + pop_completed_front.
  Building→ready_by_owner + sound (C12) stays. Use the `std::mem::take(&mut factory_shadow)` discipline for the
  borrow window (cost-seed needs &sim while spawning).
- **advance_one_step**: UNCHANGED (suspends-with-object-held at 54).

## 3. start_next_queued (activate + fix — HIGH-C7 + HIGH-round-trip)

The dormant body pops the front but seeds NOTHING. Rewrite to take the resolved cost + total_base_frames and set:
`insertion_seq = popped.enqueue_order`, `balance = original_balance = cost`,
`active_total_base_frames = popped.total_base_frames`, `progress = 0`, `step_timer = 0` (so step_all recomputes
rate then charges next eligible cadence — NEVER the same tick it is popped), clear on_hold/suspended; leave manual
per policy. Both the delivery path AND the cancel-abandon path call this cost-seeded advance. Without the
insertion_seq set, D1 breaks → hash fold order + charge order corrupt; without the cost seed, a popped build is
free; without active_total_base_frames, the post-load sidebar ETA is 0.

## 4. Sidebar view rebind (queue_view_for_owner) — byte-identical (BLOCKER fix folded in)

Project from the registry. Order: per owner factory, emit head FIRST then queue tail; sort combined by
`(category, stamp)` where stamp = insertion_seq (head) / QueueEntry.enqueue_order (tail) — algebraically
identical to today's `(queue_category, enqueue_order)` sort.

**Derived `state` (the BLOCKER — Done is NOT a same-tick transient; a blocked-exit vehicle/aircraft persists
Done across ticks at production_queue.rs:564-565/651-652 via `if is_vehicle { continue }` skipping the pop):**
- head: `manual` → Paused; else `object held && progress >= PRODUCTION_STEPS` → Done; else Building.
- tail: Queued.
(Matches refresh_queue_states' set {Building,Paused,Done}; NoFunds never surfaced — on-hold stayed Building.)

**Derived `remaining` (truncation order EXACT):** head → `(active_total_base_frames * (PRODUCTION_STEPS - progress)) / PRODUCTION_STEPS`
(multiply-then-divide, integer); tail → `total_base_frames`. Then through effective_time_to_build_frames_for_type →
estimated_real_time_ms (unchanged). total_ms: head → active_total_base_frames.max(1); tail → total_base_frames.max(1).

Other readers: `count_owned_and_queued` (production_tech.rs, AtBuildLimit gate) → count Factory.object(matching) +
matching QueueEntrys; `owner_has_building_production_busy` (world_spawn.rs, MCV-undeploy) → (owner,Building) factory
has object or non-empty queue. No sidebar/ai/app code touches queues_by_owner directly (census clean).

## 5. world/mod.rs (co-edited — 2 minimal anchored edits)

- `refresh_production_shadow`: gut the reconcile call (anchor `registry.reconcile_from_queues(self, rules);`);
  keep `refresh_economy_shadow`. The Phase-7 step_all block is UNCHANGED.
- `debug_assert_factory_invariants` invariant A (MEDIUM): re-express as a REAL registry self-check (not a
  tautology): iter_insertion_ordered yields strictly-increasing insertion_seq (no ties = total order), and for
  every factory with object.is_some(), insertion_seq == active enqueue_order (D1). Anchor on the existing
  `for (&owner, queues) in &self.production.queues_by_owner` block. Flag to the concurrent world/mod.rs owner.

## 6. Ordered micro-steps (author in this order; cargo guard each)

M0 entry type + Factory fields + reconcile builds QueueEntry tails + active_total_base_frames + hash fold rewrite
(keep queues_by_owner fold live; relative replay determinism stays green). M1 registry mutators (enqueue,
start_next_queued cost-seed) + unit tests. M2 wire delivery to start_next_queued alongside pop. M3 pause+cancel
to registry. M4 enqueue to registry. M5 flip the VIEW + the sidebar_view_parity golden test. M6 flip
count_owned_and_queued / owner_has_building_production_busy / invariant A. M7 stop writing the mirror.
M8 atomic retirement (delete BuildQueueItem/queues_by_owner/reconcile/refresh_queue_states/prune/the per-item
fold) + SNAPSHOT_VERSION 19 + re-baseline replay/pin/round-trip snapshots ONCE + full cargo green.

(Pragmatic option: implement the END STATE directly and let cargo check/test be the guard — the replay gates are
RELATIVE so they catch nondeterminism regardless of absolute hash; the compiler catches the type breaks. M-staging
is the fallback if the direct pass gets unwieldy.)

## 7. Tests

INVERT (rewrite queues_by_owner/BuildQueueItem fixtures to build registry state via the M1 enqueue helper or real
commands): production_shadow_tests.rs (insert_queue helper, persist/seq/order/refund/determinism fixtures),
production_queue_tests.rs (view/sort/enqueue/cancel/pause, remaining_base_frames reads → derived view value),
production_placement_tests.rs (cancel_last cross-category latest-stamp pin), deploy_tests.rs (undeploy-busy),
production_tests.rs:586. DELETE/retarget legacy_progress_carry_removed_from_hash + progress_carry probes.

STAY GREEN (re-point only post-condition mirror reads): production_replay_tests.rs P5c gates (bodies drive real
commands → unaffected; re-baseline absolute hash at M8). snapshot_version_is_18→_is_19. round_trip_preserves_state_hash
(the no-post-load-fixup gate). snapshot_roundtrip_factory_registry — rewrite to assert Factory.queue round-trips
verbatim with reconcile DELETED.

NEW: (1) sidebar_view_parity — frozen golden LITERAL (independent of queues_by_owner) covering active Building,
Paused, Queued tail, partial-progress remaining truncation, blocked-vehicle progress==54 Done, underfunded-mid-progress
stays Building. (2) queue_of_record_round_trips_in_factory — each QueueEntry's 3 fields + insertion_seq +
active_total_base_frames survive serde + a post-load refresh_production_shadow (no fixup) leaves the hash unchanged.
(3) start_next_queued_advances_on_delivery — new active has insertion_seq==#2 stamp, balance seeded, NOT charged the
pop tick. (4) derived_view_state_never_surfaces_nofunds_on_stall. (5) cancel_last_picks_global_max_stamp_across_categories.

## 8. DRIFT / UNKNOWN

- **DRIFT-1 (refutes a stale premise):** the P5b design §4.3/§6 + P5c handoff claim P5d retires
  matching_factory_count_for_owner via the registry key-count. The LIVE code refutes this (factory.rs:662-666):
  1 key per (owner,category) + naval-collapses-to-Vehicle (D2) means the key count CANNOT supply MultipleFactory's
  per-building count. P5d does NOT retire the rescan; it stays as-is. "Erase U-FACTORYCOUNT" means the membership
  single-key limit (genuinely erased), NOT the rescan. Its retirement is a separate slice (needs a real
  building-count source).
- The state-derivation byte-identity is proven by tests (1)+(4); until green it is DRIFT.
- The delivery borrow puzzle (cost-seed needs &sim while spawning) — take-pattern; a wrong scope is a compile error, not a silent bug.

## 9. File ownership

NO P5d edit touches src/sim/miner/*, cell_rect.rs, rng.rs, particles/* (concurrent session). world/mod.rs is
co-edited — exactly the 2 anchored edits in §5. active_producer_by_owner must survive the fold deletion intact.
