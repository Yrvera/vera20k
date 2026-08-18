# S2 — UnitClass dispatch→Process Authority Flip: Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Promote per-object `tick_counter++ → mission dispatch → locomotor Process` ordering to
authoritative for scoped UnitClass movers, by interleaving the dispatch step into the existing
Phase-1 mover loop (hash-affecting; `SNAPSHOT_VERSION` 19→20 + golden re-baseline).

**Architecture:** The dispatch *decision* moves into the scoped branch of the existing per-mover
loop in `sim/movement/movement_tick.rs`; the tail projection (`refresh_mission_shadow`) skips
dispatched ids via a per-tick set threaded through `advance_tick`. Movement bodies, whole-set
setup, and tick phase order are untouched. The post-load mission re-derive is deleted so load
trusts the serialized (hashed-authoritative) `MissionCom`.

**Design Doc:** `docs/plans/2026-06-06-s2-unit-dispatch-flip-design.md` (v2, all review
findings resolved 2026-06-10; reviewed twice).

---

## Grounding Summary

- **Design + review evidence:** the v2 design was re-verified against current post-merge code
  on 2026-06-10 (this session). All Rust-side claims confirmed by direct reads (file:line in
  this plan are from that pass). Binary claims (arrival tick keeps `Move` — 0x004D4200;
  dispatch precedes Process — 0x006F9E50; `+0xC4`-before-dispatch order) carried from the
  2026-06-06 design review, which verified them via live decompile. Not re-verified today;
  nothing in this plan touches their substance.
- **Repo patterns mirrored:** S1 read-only host + `scoped_move_unit` test helper
  (`techno_ai.rs:690-770`); L5 visibility-widening seam pattern (`miner_system.rs`
  `pub(super) process_miner`); golden-baseline discipline
  (`global_parity_harness_tests.rs:37-40`).
- **Key code facts (verified this session):**
  - Mover collection guards: `movement_tick.rs:963-976` (skips `forced_drive_processed`,
    target-less, `low_bridge_tube_state`, Air/Underground).
  - Per-mover scoped `get_mut` block: `movement_tick.rs:~1033` (`let Some(entity) =
    entities.get_mut(entity_id) else { continue };`) — insertion point.
  - Arrival clear: `finalize_finished_entities` (`movement_tick.rs:1724-1733`) sets
    `movement_target = None` post-loop. A `MovementTarget::default()` (empty path) unit
    arrives the same tick via `PathExhaustionResult::Finished` — the cheap arrival fixture.
  - Tail projection: `refresh_mission_shadow` (`mod.rs:916-923`), called at `mod.rs:2645`,
    writes current/substate + `tick_counter++` for ALL entities. Only non-test
    `mission.current` readers: `world_hash.rs:37/55` and debug-only S1 shadow
    (`techno_ai.rs:327`).
  - Load paths: `snapshot.rs:259` and `app_input.rs:755` → `rebuild_caches_after_load`
    (`mod.rs:1370`) → `rebuild_logic_membership` (`mod.rs:1382-1403`) which re-derives
    `mission.current`/`substate` — the P1 deletion target.
  - Scope predicate: `is_s1_scoped_move_unit` (`techno_ai.rs:285-293`), currently private.
  - `SNAPSHOT_VERSION: u32 = 19` (`snapshot.rs:37`).
  - `advance_tick` signature: `mod.rs:1940-1948`; movement call at `mod.rs:1989`.
  - Dense churn fixture exists: `dispatch_churn_measurement_dense_converging_battle`
    (`global_parity_harness_tests.rs:263`).
- **INI:** none. S2 changes ordering/authority only; no INI-driven constants. (Verified: the
  design names no INI keys; dispatch cadence gating is S4.)
- **Still unknown:** whether `forced_drive_processed`/`low_bridge_tube_state` units can
  simultaneously satisfy the scope predicate (bounded — uncollected scoped units fall to tail
  authority, the pre-S2 status quo; single-increment guaranteed by the set mechanism either way).

## Key Technical Decisions

- **Dispatch step interleaved in the scoped `get_mut` block, before `handle_path_exhaustion`** —
  arrival (`Finished` → `continue`) must still be preceded by dispatch, matching gamemd
  (dispatch runs before Process; arrival is Process-time). — **Confidence: high**
  - Source: design §Chosen Approach; `movement_tick.rs:1033-1075` read this session;
    0x006FA655 ordering (carried).
- **Per-tick `BTreeSet<u64>` local in `advance_tick`, passed `&mut` to movement and `&` to the
  tail** — deterministic membership, no new Simulation field, no serde/hash surface. Allocation
  is per-dispatched-mover; consistent with existing per-mover allocations in the loop
  (`debug_events: Vec::new()` per mover). — **Confidence: high** — Source: repo pattern
  (explicit-args threading at `mod.rs:1989`).
- **`refresh_mission_shadow_except(&BTreeSet<u64>)` as the primary; `refresh_mission_shadow()`
  stays as an empty-set wrapper** — keeps the 3+ test call sites compiling and the tail call
  explicit. — **Confidence: high** — Source: `mod.rs:916`, test callers `techno_ai.rs:711/727/752`.
- **`is_s1_scoped_move_unit` widened to `pub(crate)`** — movement_tick already references
  `crate::sim::world` types (e.g. `SimSoundEvent` param), so no layering violation.
  — **Confidence: high** — Source: `movement_tick.rs:838` signature read.
- **Delete the post-load mission re-derive entirely (not conditionally)** — value-identical for
  non-S2 units (save-point invariant: tail projection writes ALL entities before every
  `state_hash`), required for S2 units. — **Confidence: high** — Source: design §Save/load
  authority; `mod.rs:916-923`, `mission/mod.rs:186-202`, `world_hash.rs:36-56` verified.
- **Arrival-tick test fixture = `scoped_move_unit` with `MovementTarget::default()`** — empty
  path ⇒ `PathExhaustionResult::Finished` on tick 1 ⇒ `finalize_finished_entities` clears the
  target post-loop: a same-tick arrival with no path-grid scaffolding. — **Confidence: medium**
  (fixture behavior inferred from code read, not yet executed — first test run validates it;
  fallback: drive a 1-cell move via `Command::Move` as in `slice6_retask_tests.rs:101`).

## Open Questions

### Resolved During Planning

- **Dock↔move double-writer risk (design gate):** `retask.rs:97` (`assign_mission_keep_fields`)
  and `assign_mission_with_teardown` are command-time writers (run during command processing,
  before movement). A unit retasked into dock/attack that tick has `dock_state`/`attack_target`
  set ⇒ fails the scope predicate ⇒ no in-loop dispatch ⇒ tail authority (unchanged semantics).
  A post-movement writer (e.g. retaliation verb) legitimately overwrites a dispatched unit's
  `current` — same final value as today's tail re-derivation (machine state drives both). At
  most one *authority* writer per unit-tick: in-loop for dispatched ids, tail otherwise.
- **Borrow compatibility (design gate):** the dispatch step lives inside the existing scoped
  `&mut` entity block (`movement_tick.rs:1033`), before `ref mut target` is taken; it needs no
  cross-entity access. No conflict with the crush/bump immutable lookups (those run after the
  block ends).
- **Where does the dispatched set live?** Local in `advance_tick`; both consumers
  (`mod.rs:1989` movement call, `mod.rs:2645` tail call) are in `advance_tick`.

### Deferred to Implementation

- Whether the empty-path arrival fixture arrives on tick 1 exactly as code-read predicts
  (validated by the first test run; fallback fixture named above).
- Whether forced-drive/low-bridge states can co-occur with scope (does not affect correctness
  of the mechanism; documented UNVERIFIED in the design).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/techno_ai.rs:285` | widen scope predicate to `pub(crate)` |
| Modify | `src/sim/world/mod.rs:916-923` | `refresh_mission_shadow_except` + wrapper |
| Modify | `src/sim/world/mod.rs:1985-1995` | declare + thread dispatched set |
| Modify | `src/sim/world/mod.rs:2645` | tail call switches to `_except` |
| Modify | `src/sim/world/mod.rs:1382-1403` | delete post-load mission re-derive (P1) |
| Modify | `src/sim/movement/movement_tick.rs:820-840` | new `dispatched` param |
| Modify | `src/sim/movement/movement_tick.rs:~1033` | in-loop dispatch step |
| Modify | `src/sim/mission/mod.rs:177-185` | fix stale doc comment (hashed since Slice 8) |
| Modify | `src/sim/snapshot.rs:37` | `SNAPSHOT_VERSION` 19→20 |
| Modify | `src/sim/world/techno_ai.rs` (tests) | new S2 tests |
| Modify | `src/sim/world/global_parity_harness_tests.rs:40` | golden re-baseline |

## Interface Changes

- `tick_movement_with_grids` gains `dispatched: &mut BTreeSet<u64>` (one caller: `mod.rs:1989`;
  test callers of the movement fn: none — verified by grep, it is only called from `mod.rs`).
- `refresh_mission_shadow()` keeps its signature (wrapper); new `pub(crate)
  refresh_mission_shadow_except(&mut self, skip: &BTreeSet<u64>)`.
- `is_s1_scoped_move_unit` becomes `pub(crate)` (read-only predicate; no callers change).

## Sim Checklist

- [x] No float — only integer wrapping_add and enum writes.
- [x] No NEW hashed state — `mission` already hashed (Slice 8); S2 changes *when/where* the
  hashed values are written ⇒ `SNAPSHOT_VERSION` bump + golden re-baseline (Tasks 5/7).
- [x] No render/ui/sidebar/audio/net dependency added.
- [x] Tick ordering: advance_tick phase order unchanged; dispatch decision relocates into the
  existing Phase-1 movement pass for scoped units (the design's verified gamemd ordering).
- [x] Iteration order: mover loop stays live-order; tail stays BTreeMap ascending-id;
  `BTreeSet` used for membership only.

## Risk Areas

- **Golden baseline shift is EXPECTED (churn ticks only).** If `state_hash` changes on ticks
  with no arrivals/no scoped movers, that is a bug (double-count or clobber), not a re-baseline.
- **Tail skip must cover BOTH writes** (`current`/`substate` AND `tick_counter`) — skipping only
  one desyncs the hash from the dispatch values.
- **Save/load:** any future re-derive re-introduction breaks lockstep restores; the round-trip
  test is the regression guard.
- **Dense scenario position fingerprint** (Task 2) is the movement-neutrality tripwire: it is
  captured pre-flip and must stay green post-flip.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 3 | Per-object order: counter++ → dispatch → Process | gamemd's verified per-object frame order; the slice's whole point | carried binary citations (0x006fa64f/0x006FA655/0x004DA877) + `s2_tick_counter_increments_exactly_once` |
| 3 | Arrival tick hashes `mission.current = Move` (not Sleep) | gamemd keeps Move on arrival tick (0x004D4200); fires on every unit arrival, every match | `arrival_tick_mission_is_move_not_sleep` |
| 3 | Exactly one `tick_counter` increment per unit-tick | hashed counter; double/zero-count = permanent lockstep drift | `s2_tick_counter_increments_exactly_once` |
| 2→3 | Movement byte-neutrality | positions/occupancy must be untouched by the flip | position fingerprint captured pre-flip (Task 2), must hold post-flip (Task 3) |
| 4 | Load trusts serialized MissionCom | save/load on arrival tick must restore identical hash (MP/lockstep hard requirement) | `save_load_round_trip_on_arrival_tick` |
| 7 | Golden re-baseline only for churn reasons | hash changes must be exactly the documented arrival-tick fidelity fix | determinism assert + churn measurement still pass; one-line documented re-baseline |

---

## Tasks

### Task 1: Interface prep — predicate visibility + tail-skip API

**Why:** Contracts first; both later tasks consume them. Behavior-neutral.

**Files:**
- Modify: `src/sim/world/techno_ai.rs:285`
- Modify: `src/sim/world/mod.rs:916-923`

**Pattern:** L5 seam visibility widening (`miner_system.rs` `pub(super) process_miner`).

**Step 1: Widen the predicate** (`techno_ai.rs:285`) — replace
`fn is_s1_scoped_move_unit(e: &GameEntity) -> bool {` with:

```rust
/// `pub(crate)` so the S2 in-loop dispatch step (movement_tick.rs) can gate on
/// the same scope predicate the host/shadow uses; widening is behavior-neutral.
pub(crate) fn is_s1_scoped_move_unit(e: &GameEntity) -> bool {
```

**Step 2: Split the tail projection** (`mod.rs:916-923`) — replace the body of
`refresh_mission_shadow` with a wrapper + `_except` primary (keep the existing doc comment on
the wrapper, and extend it with the S2 sentence shown):

```rust
    /// Refresh the `mission` component's `current`/`substate` on every entity
    /// from the authoritative `Option<T>` machines, and advance its per-entity
    /// `tick_counter`. As of Slice 8 `mission` IS folded into `world_hash`, so
    /// this is the canonical projection writer: `current`/`substate` are a
    /// deterministic function of the authoritative machines (the verbs own
    /// `queued`/`suspended`/`timer`). Runs before `state_hash()` each tick tail,
    /// so the folded value reflects the current tick. BTreeMap `values_mut()`
    /// yields deterministic ascending-id order.
    pub(crate) fn refresh_mission_shadow(&mut self) {
        self.refresh_mission_shadow_except(&BTreeSet::new());
    }

    /// Tail projection with an S2 skip set: ids dispatched in-loop this tick
    /// already committed `current`/`substate` and incremented `tick_counter` at
    /// host time (authoritative); rewriting them here would clobber the
    /// dispatch-time value and double-count the counter.
    pub(crate) fn refresh_mission_shadow_except(&mut self, dispatched: &BTreeSet<u64>) {
        for entity in self.substrate.entities.values_mut() {
            if dispatched.contains(&entity.stable_id) {
                continue;
            }
            let (current, substate) = entity.derived_mission();
            entity.mission.current = current;
            entity.mission.substate = substate;
            entity.mission.tick_counter = entity.mission.tick_counter.wrapping_add(1);
        }
    }
```

(`EntityStore` exposes `values_mut()` but no keyed `iter_mut()` — verified
`entity_store.rs:96-151`; the membership check uses `entity.stable_id`. Add
`use std::collections::BTreeSet;` to `mod.rs` imports if not already present.)

**Step 3: Verify**
Run: `cargo check -p vera20k` then `cargo test -p vera20k refresh_mission -- --nocapture`
Expected: compiles; existing tests pass (wrapper preserves behavior).

**Step 4: Commit** — `sim: S2 T1 — scope-predicate visibility + tail-skip projection API (behavior-neutral)`

### Task 2: Thread the dispatched set + capture the pre-flip position fingerprint

**Why:** Plumbing lands hash-neutral (set stays empty), and the movement-neutrality tripwire is
baselined BEFORE the flip so Task 3 must keep it green.

**Files:**
- Modify: `src/sim/movement/movement_tick.rs:820-840` (signature), `src/sim/world/mod.rs:1985-1995` (call), `src/sim/world/mod.rs:2645` (tail call)
- Modify: `src/sim/world/global_parity_harness_tests.rs` (fingerprint test)

**Pattern:** explicit-args threading used by every movement param (`mod.rs:1989`).

**Step 1:** Add the param to `tick_movement_with_grids` (after `sound_events`):

```rust
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
    /// S2: ids whose dispatch step ran in-loop this tick (tail projection skips them).
    dispatched: &mut BTreeSet<u64>,
) -> MovementTickStats {
```

(The param is consumed in Task 3. To keep this task warning-free, add a one-line stub at the
top of the function body — `let _ = &*dispatched; // consumed by the S2 dispatch step (T3)` —
and delete it in Task 3. Do NOT rename the param with a leading underscore; T3 uses it.)

**Step 2:** At `mod.rs`, immediately before the movement call (`:1989`):

```rust
        // S2: ids dispatched in-loop this tick; consumed by the tail projection.
        let mut s2_dispatched: BTreeSet<u64> = BTreeSet::new();
        let movement_stats = movement::tick_movement_with_grids(
            ...existing args unchanged...,
            &mut s2_dispatched,
        );
```

**Step 3:** Switch the tail call (`mod.rs:2645`):

```rust
        self.refresh_mission_shadow_except(&s2_dispatched);
```

(Confirm `s2_dispatched` is still in scope at `:2645` — both sites are in `advance_tick`; if a
block boundary intervenes, declare it at the top of `advance_tick`.)

**Step 4: Position fingerprint test** — in `global_parity_harness_tests.rs`, alongside the
dense scenario (`:263`), add a test that runs the SAME dense converging-battle construction and
folds every entity's `(id, rx, ry, sub_x, sub_y)` per tick into a `DefaultHasher`, asserting a
committed constant:

```rust
/// S2 movement-neutrality tripwire: the dispatch flip must not move anyone.
/// Captured pre-flip (Task 2); Task 3 must keep it green UNCHANGED.
#[test]
fn s2_dense_scenario_position_fingerprint_stable() {
    let (mut sim, rules, heights, ticks) = dense_converging_setup(); // extract from :263 if needed
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for _ in 0..ticks {
        let _ = sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
        for (id, e) in sim.substrate.entities.iter_sorted() {
            use std::hash::Hash;
            (id, e.position.rx, e.position.ry, e.position.sub_x, e.position.sub_y).hash(&mut h);
        }
    }
    use std::hash::Hasher;
    assert_eq!(h.finish(), POSITION_FINGERPRINT, "see Task-2 capture note");
}
```

Run once with a dummy constant, paste the printed/actual value as `POSITION_FINGERPRINT`
(`const POSITION_FINGERPRINT: u64 = <captured>;`) with a comment naming the capture commit.
If `:263`'s construction is not trivially extractable into `dense_converging_setup()`, factor
the minimal shared builder — construction only, no assertions moved.

**Step 5: Verify**
Run: `cargo test -p vera20k global_ -- --nocapture` and `cargo test -p vera20k`
Expected: ALL green including the untouched golden baseline (the set is empty ⇒ bit-identical).

**Step 6: Commit** — `sim: S2 T2 — thread dispatched-set plumbing (hash-neutral) + pre-flip position fingerprint`

### Task 3: The flip — in-loop dispatch step (hash-changing)

**Why:** The slice's behavior change, on top of verified plumbing.

**Files:**
- Modify: `src/sim/movement/movement_tick.rs:~1033`

**Pattern:** design §Chosen Approach data flow; gamemd per-object order (carried citations).

**Step 1:** Remove the Task-2 `let _ = &*dispatched;` stub. In the per-mover scoped block,
immediately after `let Some(entity) = entities.get_mut(entity_id) else { continue; };` and
BEFORE `active_layer`/`handle_path_exhaustion`:

```rust
            // S2 per-object dispatch (authoritative for scoped move units): the
            // counter ticks and the dispatch-time mission commits BEFORE this
            // unit's locomotor Process — including the arrival tick, where the
            // committed mission is still Move (the target clears post-loop).
            // The tail projection skips these ids (no double-count, no clobber).
            if crate::sim::world::techno_ai::is_s1_scoped_move_unit(entity) {
                entity.mission.tick_counter = entity.mission.tick_counter.wrapping_add(1);
                let (current, substate) = entity.derived_mission();
                entity.mission.current = current;
                entity.mission.substate = substate;
                dispatched.insert(entity_id);
            }
```

**Step 2: Verify the expected failure surface**
Run: `cargo test -p vera20k`
Expected: `global_skirmish_replay_is_deterministic_and_baseline_stable` FAILS on the committed
final hash (churn ticks now hash dispatch-time values) — this exact failure is the Task-7
re-baseline input. `s2_dense_scenario_position_fingerprint_stable` MUST STILL PASS (if it
fails, STOP: the flip moved someone — that is a bug, not a re-baseline). The S1 shadow asserts
must still pass (host-record and in-loop values agree; nothing mutates scoped machines between
them).

**Step 3: Commit** — `sim: S2 T3 — in-loop dispatch authority for scoped move units (hash-changing; golden re-baselined in T7)`

### Task 4: Load trusts serialized MissionCom (P1 fix) + stale comments

**Why:** S2 makes `mission.current` non-re-derivable state; the post-load overwrite would desync
save/load on arrival ticks.

**Files:**
- Modify: `src/sim/world/mod.rs:1382-1403`
- Modify: `src/sim/mission/mod.rs:177-185`

**Step 1:** In `rebuild_logic_membership`, DELETE these lines (keeping the presence reconcile):

```rust
            // `mission` round-trips via serde now, but current/substate are
            // re-derived from the just-restored authoritative machines so a
            // save/load round-trip restores identical derived state.
            let (current, substate) = entity.derived_mission();
            entity.mission.current = current;
            entity.mission.substate = substate;
```

and extend the loop's remaining comment so the contract is stated:

```rust
        // Presence is #[serde(skip)] → all-default (Limbo) straight after
        // deserialize. Reconcile it from the just-restored authoritative gates so
        // a save/load round-trip restores identical presence (Slice 2 acceptance).
        // `mission` is NOT re-derived: it is hashed authoritative state (Slice 8)
        // that round-trips via serde, and as of S2 the dispatch-time value can
        // legitimately differ from a fresh derivation (arrival tick) — a re-derive
        // here would desync the restored hash.
        for entity in self.substrate.entities.values_mut() {
            entity.presence = entity.derived_presence();
        }
```

**Step 2:** Fix the stale `MissionCom` doc comment (`mission/mod.rs:181-185`): replace

```rust
/// The Slice-6 verb API writes this component in parallel with the legacy
/// `Option<T>` machines (which stay authoritative); `current`/`substate` are also
/// re-derived from those machines each tick. It round-trips via serde but is NOT
/// yet folded into `world_hash`, so it cannot perturb the lockstep hash. A later
/// slice hashes it and retires the redundant `Option<T>` selectors.
```

with:

```rust
/// Canonical hashed lockstep state (Slice 8): folded into `world_hash` and fully
/// serde round-tripped — load trusts it verbatim (no post-load re-derivation).
/// `current`/`substate` are written by the tail projection for most units and,
/// as of S2, at host/dispatch time for scoped move units (where the dispatch-time
/// value is authoritative — e.g. an arrival tick hashes `Move`). The verb API
/// owns `queued`/`suspended`/`timer`.
```

**Step 3: Verify**
Run: `cargo test -p vera20k saveload -- --nocapture` and `cargo test -p vera20k snapshot`
Expected: existing save/load + membership tests still green (serialized == derived for all
their fixtures, by the save-point invariant).

**Step 4: Commit** — `sim: S2 T4 — load trusts serialized MissionCom (delete post-load re-derive); stale doc comments fixed`

### Task 5: SNAPSHOT_VERSION bump

**Why:** Hash-affecting authority change; old snapshots must not load into the new contract.

**Files:** Modify: `src/sim/snapshot.rs:37`

**Step 1:** `const SNAPSHOT_VERSION: u32 = 20;` and extend the constant's comment with one
line: `// 19→20: S2 — mission.current authority at dispatch time for scoped movers (+ load trusts serde).`

**Step 2: Verify** — `cargo test -p vera20k snapshot`
Expected: version-mismatch tests pass (they compare against the constant).

**Step 3: Commit** — `sim: S2 T5 — SNAPSHOT_VERSION 19->20 (dispatch-time mission authority)`

### Task 6: S2 acceptance tests

**Why:** The design's named failure modes each get a direct guard.

**Files:** Modify: `src/sim/world/techno_ai.rs` (tests module — reuse `scoped_move_unit`)

**Pattern:** S1 tests (`techno_ai.rs:705-770`); saveload test (`snapshot.rs:455-475`).

**Design-test mapping note:** the design names two further tests this plan covers elsewhere —
`scoped_and_unscoped_unit_same_cell_contention` and `unit_move_start_slip_matches_dispatch_then_process`
are both subsumed by the Task-2 dense-converging position fingerprint (20 tanks contending for
converging cells, captured pre-flip, byte-identical post-flip ⇒ contention resolution and
start-slip movement outputs unchanged). `guard_skipped_scoped_unit_single_count` is realized as
the idle-unit half of `s2_tick_counter_increments_exactly_once` (uncollected unit → tail path,
single increment); a forced-drive variant needs war-factory scaffolding and is deferred with
the scope-overlap unknown (see Deferred).

**Step 1: Arrival-tick fidelity + exactly-once counter + idle (uncollected) single count:**

```rust
    /// S2: the arrival tick hashes the dispatch-time mission (`Move`) — the
    /// target clears post-loop, so a tail re-derivation would say `None` (the
    /// machine-less fall-through; the idle→Guard mapping is S3). The
    /// transition away happens on the NEXT tick (gamemd-faithful).
    #[test]
    fn arrival_tick_mission_is_move_not_sleep() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1)); // default target: arrives tick 1
        sim.set_logic_order_for_test(vec![1]);
        let heights = BTreeMap::new();

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        let e = sim.substrate.entities.get(1).unwrap();
        assert!(e.movement_target.is_none(), "fixture must arrive on tick 1");
        assert_eq!(e.mission.current, MissionType::Move, "arrival tick keeps Move");

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        let e = sim.substrate.entities.get(1).unwrap();
        // Machine-less derivation falls through to None (game_entity.rs:555);
        // S3 owns the idle→Guard mapping.
        assert_eq!(e.mission.current, MissionType::None, "post-arrival tick transitions");
    }

    /// S2: exactly one tick_counter increment per unit-tick — in-loop for a
    /// dispatched mover, tail for an idle (never-collected) unit. Double or
    /// zero count is permanent lockstep drift.
    #[test]
    fn s2_tick_counter_increments_exactly_once() {
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1)); // dispatched on tick 1
        let mut idle = scoped_move_unit(2);
        idle.movement_target = None; // never collected; never scoped
        sim.substrate.entities.insert(idle);
        sim.set_logic_order_for_test(vec![1, 2]);
        let heights = BTreeMap::new();

        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission.tick_counter, 1);
        assert_eq!(sim.substrate.entities.get(2).unwrap().mission.tick_counter, 1);
        let _ = sim.advance_tick(&[], None, &heights, None, None, 67);
        assert_eq!(sim.substrate.entities.get(1).unwrap().mission.tick_counter, 2);
        assert_eq!(sim.substrate.entities.get(2).unwrap().mission.tick_counter, 2);
    }

    /// S2 P1 guard: a save taken on the arrival tick (current=Move while a fresh
    /// derivation says None) must restore an IDENTICAL state hash. Guards the
    /// deleted post-load re-derive against reintroduction.
    #[test]
    fn save_load_round_trip_on_arrival_tick() {
        use crate::sim::snapshot::GameSnapshot;
        let mut sim = Simulation::new();
        sim.substrate.entities.insert(scoped_move_unit(1));
        sim.set_logic_order_for_test(vec![1]);
        let heights = BTreeMap::new();
        let _ = sim.advance_tick(&[], None, &heights, None, None, 67); // arrival tick

        let e = sim.substrate.entities.get(1).unwrap();
        assert_eq!(e.mission.current, MissionType::Move, "precondition: divergent window");
        assert!(e.movement_target.is_none());
        let hash_before = sim.state_hash();

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let mut restored = GameSnapshot::load(&bytes).expect("load").sim;
        restored.rebuild_logic_membership(); // the real post-deserialize step
        assert_eq!(restored.state_hash(), hash_before, "load must trust serialized MissionCom");
        assert_eq!(
            restored.substrate.entities.get(1).unwrap().mission.current,
            MissionType::Move,
        );
    }
```

Adjust `advance_tick`'s `rules` arg to `Some(&...)` with the nearest minimal rules helper if
`None` trips a movement-path unwrap — `world_tests.rs:547` shows the `Some(&rules)` form.
If `scoped_move_unit`'s default-target fixture does NOT arrive on tick 1 (see Deferred), switch
the fixture to a 1-cell `Command::Move` as in `slice6_retask_tests.rs:101` and advance until
`movement_target.is_none()`, asserting the SAME post-conditions on that tick.

**Step 2: Verify** — `cargo test -p vera20k s2_ -- --nocapture` and
`cargo test -p vera20k arrival_tick` and `cargo test -p vera20k save_load_round_trip`
Expected: all PASS. `save_load_round_trip_on_arrival_tick` MUST FAIL if Task 4's deletion is
reverted (sanity-check once by mentally tracing, not by reverting).

**Step 3: Commit** — `test(sim): S2 T6 — arrival-tick fidelity, exactly-once counter, save/load-on-arrival round-trip`

### Task 7: Golden re-baseline + full verification

**Why:** The committed final hash legitimately shifts (churn ticks now hash dispatch-time
values); the re-baseline is documented once, with the gamemd-cited reason.

**Files:** Modify: `src/sim/world/global_parity_harness_tests.rs:40`

**Step 1:** Run `cargo test -p vera20k global_skirmish_replay -- --nocapture`. Read the
printed actual final hash from the assertion message. Update:

```rust
/// Re-baselined for S2 (dispatch-time mission authority for scoped movers):
/// arrival ticks now hash `Move` (gamemd-faithful, 0x004D4200 — see the S2
/// design doc); churn ticks were the only hash drivers.
const GLOBAL_HARNESS_FINAL_HASH: u64 = <actual from the run>;
```

(Address citation lives in the doc-comment of a TEST baseline constant — this is a docs-grade
citation, but per the no-binary-refs-in-code rule, cite the DESIGN DOC instead if in doubt:
"see docs/plans/2026-06-06-s2-unit-dispatch-flip-design.md §Impact Analysis". Use the design-doc
form.)

**Step 2:** Confirm determinism still holds (the same test's replay==record assertion) and the
dense churn measurement (`dispatch_churn_measurement_dense_converging_battle`) passes.

**Step 3: Full suite** — `cargo test -p vera20k` then `cargo clippy -p vera20k`
Expected: `test result: ok` on every binary, 0 failed; no new clippy warnings in touched files.

**Step 4: Commit** — `sim: S2 T7 — golden re-baseline (arrival-tick Move churn; design-doc cited reason)`

### Task 8: End-to-end sanity vs gamemd expectation

**Why:** Result-oriented close-out: the observable contract, not just unit tests.

**Verify:**
- A scoped unit ordered to move: per-tick hashed `mission.current` is `Move` from the first
  dispatch tick through the arrival tick inclusive, `None` after (the machine-less derivation;
  idle→Guard is S3) — matches the design's gamemd-verified timeline (arrival keeps Move;
  transition next tick).
- Save/load mid-march AND on the arrival tick both restore identical hashes
  (`save_load_round_trip_on_arrival_tick` + existing saveload suite).
- The position fingerprint and golden determinism prove no movement/occupancy delta.
- Run the game (`cargo run`) for a 2-minute skirmish smoke: order tanks around, save/load once;
  no asserts, no desync panic, movement feel unchanged.

## Sources & References

- **Design doc:** `docs/plans/2026-06-06-s2-unit-dispatch-flip-design.md` (v2)
- **Ladder:** `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §9
- **Ghidra reports:** `docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md` §4/§9,
  `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §5,
  `docs/research/S2_MISSION_DISPATCH_VS_PASSIVE_ACQUIRE_ORDERING.md`
- **gamemd.exe addresses (provenance: 2026-06-06 design review, live-decompiled then):**
  0x004D4200 (Mission_Move arrival semantics), 0x006F9E50 (dispatch precedes Process),
  0x006fa64f/0x006FA655/0x004DA877 (+0xC4 → dispatch → Process order). Kept here, not in code.
- **INI keys:** none (ordering/authority slice).
- **Related code (verified this session):** `src/sim/world/techno_ai.rs:285-293, 705-770`,
  `src/sim/world/mod.rs:916-923, 1370-1403, 1940-1948, 1985-1995, 2645`,
  `src/sim/movement/movement_tick.rs:820-840, 963-976, ~1033, 1724-1733`,
  `src/sim/mission/mod.rs:177-202`, `src/sim/mission/timer.rs:13-17`,
  `src/sim/world/world_hash.rs:28-56`, `src/sim/snapshot.rs:37, 259, 455-525`,
  `src/app_input.rs:755`, `src/sim/world/global_parity_harness_tests.rs:37-40, 263`
- **Prior commits:** S1 host (`unit-mission-dispatch-host` branch, merged PR #102); L5 seam
  (`aa660f62`); Slice 8 mission authority (`9d060cad..b452c537`).
