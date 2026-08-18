# Object Substrate — Slice 1 (`ObjectSubstrate` wrapper) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> This is a **pure refactor**: no behavior change, replay-hash bit-identical.

**Goal:** Introduce an `ObjectSubstrate` struct in `sim/world/` that owns the active-object
vector (`logic`) plus the two monotonic bookkeeping counters (stable-id allocator,
occupancy enter-order) — establishing the single home that later slices (Presence FSM,
deferred-delete, reveal gate-chain) will extend — without changing any observable behavior.

**Architecture:** `ObjectSubstrate` is a field bundle owned by `Simulation` (in `sim/world/`).
The lifecycle methods (`reveal`/`conceal`/`unlimbo`/`uninit`/`register_live_object`/…) **stay
on `Simulation`** — they need `entities` + `occupancy` + `logic` together and
`for_each_live_object` hands `&mut Simulation` to its closure, so they cannot move onto a
sub-struct in Slice 1. Only **field paths change** (`self.logic` → `self.substrate.logic`,
etc.). The deterministic state hash reads field *values*, not struct bytes, so the hash is
preserved by construction; the bincode snapshot layout changes, so `SNAPSHOT_VERSION` bumps.

**Design Doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](../research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md) (Slice 1, §6, §8)

---

## Grounding Summary

- **Design doc** classifies Slice 1 as *"pure refactor, hash-identical, zero research
  dependency"* and the right starting point. It reproduces **no new gamemd behavior**, so no
  Ghidra verification is required for the slice itself. (Underlying binary evidence already
  exists — see Sources.)
- **`Simulation` struct** lives at [src/sim/world/mod.rs:266](../../src/sim/world/mod.rs) and
  is `#[derive(serde::Serialize, serde::Deserialize)]`. The five fields the *full* design
  names are: `entities: EntityStore` (270, `pub`), `next_stable_entity_id: u64` (315,
  `pub(crate)`), `next_occupancy_enter_order: u64` (319, `pub(crate)`), `logic: LogicVector`
  (322–323, `pub(crate)`, `#[serde(default)]`), `occupancy: OccupancyGrid` (370,
  `#[serde(skip)]`).
- **Blast radius (measured this session via grep over `src/`):**
  | Field | Refs | Files | Layers touched |
  |---|---|---|---|
  | `.entities` | **1340** | 75 | sim + **app + render + ui** |
  | `.occupancy` | **71** | 25 | sim + **app + render** |
  | `.logic` | **9** | 2 | sim/world only (`mod.rs`, `world_hash.rs`) |
  | `next_stable_entity_id` + `next_occupancy_enter_order` | **~55** | 13 | **sim only** (mostly tests) |
- **Hash authority** = [src/sim/world/world_hash.rs:33](../../src/sim/world/world_hash.rs)
  `state_hash()`. It hashes **individual field values** in a fixed order:
  `next_stable_entity_id` (48), `next_occupancy_enter_order` (49), `logic.as_slice()` (52–56),
  and per-entity `occupancy_enter_order` (387). It does **not** bincode the whole struct → moving
  fields into `substrate` only changes the *paths* the hasher reads, not the values or order →
  **hash bit-identical**. `occupancy` is never hashed directly (rebuilt skip-cache).
- **Snapshot** = [src/sim/snapshot.rs:18](../../src/sim/snapshot.rs): `GameSnapshot { … sim:
  Simulation }` bincode-serialized **positionally**. `SNAPSHOT_VERSION = 14`. Nesting any
  *serialized* field (`logic`, `next_stable_entity_id`, `next_occupancy_enter_order`) under
  `substrate` changes the byte layout → **bump to 15**. `GameSnapshotHeader` is a manual prefix
  ending before `sim`, so it is unaffected.
- **`LogicVector`** ([src/sim/world/logic_vector.rs:13](../../src/sim/world/logic_vector.rs)) and
  **`EntityStore`** ([src/sim/entity_store.rs:33](../../src/sim/entity_store.rs)) are already
  clean encapsulated types with **custom transparent serde** (serialize as inner `Vec<u64>` /
  `BTreeMap`). `OccupancyGrid` ([src/sim/occupancy.rs:98](../../src/sim/occupancy.rs)) holds
  `cells: BTreeMap<…>`, is skip-serialized, rebuilt via `OccupancyGrid::rebuild(&entities)`.
- **Lifecycle methods** (all in `sim/world/mod.rs`): `allocate_stable_id` (700),
  `register_live_object` (707), `unregister_live_object` (716), `reveal` (730), `conceal` (736),
  `unlimbo` (742), `add_entity_occupancy` (746, counter at 762–764), `remove_entity_occupancy`
  (772), `debug_assert_logic_membership_consistent` (786), `live_object_order_snapshot` (807),
  `for_each_live_object` (825, passes `&mut Simulation`), `set_logic_order_for_test` (836),
  `uninit` (879), `despawn_entity` (901), `rebuild_logic_membership` (1038), and the
  cache-rebuild caller `rebuild_caches_after_load` (1001).
- **Unknown after grounding:** none blocking. The only open item is the **scope decision**
  (recommended narrow set vs. literal design box) — see Key Technical Decisions.

## Key Technical Decisions

- **SCOPE — move the 3 sim-internal bookkeeping/ordering fields, defer `entities`+`occupancy`.**
  **Confidence:** high — **Source:** blast-radius grep (above) + design §6 borrow note.
  - **Recommended (this plan): Slice 1a.** `ObjectSubstrate` owns `logic`,
    `next_stable_entity_id`, `next_occupancy_enter_order`. ~64 edit sites, **100% inside `sim/`**,
    no app/render/ui churn. These three are co-located, all `pub(crate)`, all hashed together,
    and are exactly the state later slices extend (active vector, id allocator, enter-order
    counter; Slice 2 adds `presence`, Slice 6 adds `pending_delete`).
  - **Alternative: Slice 1b (full design box).** Also nest `entities` (1340 refs) and
    `occupancy` (71 refs) → ~1475 edits across 75 files **including the app/render layer**. This
    matches the §6 ASCII box literally but is a giant, merge-conflict-prone diff for *zero*
    additional contract value in Slice 1 (both are already well-encapsulated). The design's own
    borrow-discipline note ("storage stays independently borrowable"; "houses stays on
    Simulation and the substrate takes `&mut Houses`") supports leaving storage on `Simulation`
    and having the substrate borrow it — so deferring `entities`/`occupancy` is *consistent with*
    the design, not a violation.
  - **This plan implements Slice 1a.** ⚠ **User to confirm at review.** If you want the full
    box now, the task structure is identical — only the site list and layer surface grow (and a
    second `SNAPSHOT`-neutral note for `occupancy`, which isn't serialized/hashed).
- **Lifecycle methods stay on `Simulation`.** **Confidence:** high — **Source:** method bodies
  ([mod.rs:707–897](../../src/sim/world/mod.rs)). `register_live_object` touches `entities`+`logic`;
  `add_entity_occupancy` touches `entities`+counter+`occupancy`; `for_each_live_object` passes
  `&mut Simulation`. Moving bodies onto the sub-struct would force `&mut EntityStore`/`&mut
  OccupancyGrid` params — that is **Slice ≥2 work**, not the no-op wrapper. Slice 1 only relocates
  *fields* and rewrites paths.
- **`ObjectSubstrate` derives `Serialize`/`Deserialize`; `logic` keeps `#[serde(default)]`.**
  **Confidence:** high — **Source:** [mod.rs:322](../../src/sim/world/mod.rs),
  [logic_vector.rs:62](../../src/sim/world/logic_vector.rs). The two counters serialize plainly;
  `logic`'s `LogicVector` custom serde is preserved inside the sub-struct.
- **Bump `SNAPSHOT_VERSION` 14 → 15.** **Confidence:** high — **Source:**
  [snapshot.rs:18](../../src/sim/snapshot.rs) + design §8 ("bump on any field reorder — bincode is
  positional"). Layout changes; hash does not.
- **Borrow ergonomics preserved.** **Confidence:** high — **Source:** NLL split-borrow analysis.
  `self.substrate.X`, `self.entities`, `self.occupancy`, `self.houses` are disjoint fields of
  `self`, so existing split borrows (e.g. `add_entity_occupancy` holding `entity` from
  `self.entities` while reading the counter and calling `self.occupancy.add`) still compile.

## Open Questions

### Resolved During Planning
- *Does nesting change the replay hash?* **No** — `state_hash` reads field values, not struct
  bytes (world_hash.rs:48–56, 387). Resolved by reading the hash function.
- *Does it break the save header?* **No** — header prefix ends before `sim` (snapshot.rs:43–51).
- *Can the lifecycle methods move onto the substrate now?* **No** — borrow shape + the
  `&mut Simulation` closure in `for_each_live_object`. Deferred to a later slice.
- *Where is `logic` accessed outside sim/world?* **Nowhere** — grep: only `mod.rs` + `world_hash.rs`.

### Deferred to Implementation / User
- **Scope confirmation (Slice 1a vs 1b).** Recommended 1a; awaiting user sign-off at review.
- **Exact per-file counter site list.** ~55 sites across 13 sim files; the executor enumerates
  them with the grep in Task 3 rather than hand-listing each (pure mechanical rename).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/world/substrate.rs` | `ObjectSubstrate` — owns active-vector order + id/enter-order counters; thin accessors |
| Modify | `src/sim/world/mod.rs` | declare `mod substrate`; replace 3 fields with `substrate`; rewrite lifecycle-method field paths |
| Modify | `src/sim/world/world_hash.rs` | read `self.substrate.*` (hash values unchanged); fix test field paths |
| Modify | `src/sim/snapshot.rs` | bump `SNAPSHOT_VERSION` 14 → 15 + comment |
| Modify | `src/sim/movement/{movement_step,movement_tick,mod}.rs` | counter path rename (if referenced) |
| Modify | `src/sim/ai.rs` | counter path rename |
| Modify | sim test files (`world_tests`, `deploy_tests`, `miner_tests`, `production_tests`, `movement_tests`, `prone_speed_tests`, `world_orders_c4_tests`, `world_orders_bridge_repair_tests`) | counter/`logic` path rename in test setup |

## Interface Changes

- **New:** `pub(crate) struct ObjectSubstrate` in `sim/world/substrate.rs` with `pub(crate)`
  fields `logic`, `next_stable_entity_id`, `next_occupancy_enter_order`. Re-exported as
  `pub(crate) use substrate::ObjectSubstrate;` from `world/mod.rs`. **Internal to `sim/`** — no
  cross-crate or app-facing API changes.
- **Changed:** `Simulation` loses 3 fields, gains `pub(crate) substrate: ObjectSubstrate`.
  Dependents = only the in-`sim/` sites enumerated above (no app/render/ui). `Simulation`'s
  public method surface (`allocate_stable_id`, `reveal`, …) is **unchanged**.
- **Changed:** `SNAPSHOT_VERSION` 14 → 15 (rejects pre-existing on-disk saves, which are
  engine-private; in-build round-trips unaffected).

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (no math added; counters are `u64`).
- [x] New state included in deterministic state hash — **same values, same order**; verified the
      3 fields remain hashed via their new paths (world_hash.rs:48,49,52). No state added/removed.
- [x] No dependencies on render/ui/sidebar/audio/net — `substrate.rs` depends only on
      `sim/world/logic_vector` + std/serde.
- [x] Tick ordering impact — **none**; `advance_tick` logic untouched, only field paths.
- [x] BTreeMap iteration order — **unchanged**; `EntityStore`/`occupancy` not moved (Slice 1a).

## Risk Areas

- **Highest blast radius is intentionally excluded** (`entities`/`occupancy`); Slice 1a touches
  only ~64 sim-internal sites → low risk.
- **Single non-compiling window per field move** — mitigated by staging one field per
  task/commit (Tasks 1–3), each ending green.
- **Hash regression** — the one way this slice could change behavior. Guard: world_hash test
  modules (`rally_hash_tests`, `particle_hash_tests`, `rocking_hash_tests`, `c4_hash_tests`, …)
  + snapshot round-trip tests must stay green, **and** Task 4 adds an explicit
  spawn-then-hash determinism check.
- **Snapshot round-trip** — `saveload_restores_live_object_order_verbatim`,
  `saveload_occupancy_list_order_matches_incremental`, `saveload_rebuild_is_deterministic`
  ([snapshot.rs:411,459,505](../../src/sim/snapshot.rs)) must stay green after the version bump
  (they save→load in-build, so version matches).
- **Parallel sessions** — many files are already dirty in `git status`. Keep the diff inside
  `sim/` and commit promptly to minimize conflict surface.

## Parity-Critical Items

Slice 1 ships **no new gamemd behavior**, but the refactor must preserve two parity-load-bearing
properties exactly:

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1–3 | Replay state-hash bit-identity | Any drift = silent desync / non-deterministic replay; the lockstep contract | world_hash + snapshot test modules green; Task 4 spawn-then-hash determinism test |
| 1 | Active-vector order semantics (tail-append, order-preserving compacting remove, same-pass `for_each_live_object` re-read) | Drives object AI/update order and mid-tick-spawn visibility (design C8/C9) | `LogicVector` unit tests + `for_each_live_object` behavior unchanged (method body byte-identical except none) |
| 3 | `occupancy_enter_order` value sequence | Feeds occupancy list order (FirstObject/AltObject) and **is hashed** (world_hash.rs:387) | `OccupancyGrid::rebuild` determinism test (occupancy.rs) + snapshot occupancy-order test green |

---

## Tasks

### Task 1: Create `ObjectSubstrate` and migrate `logic`

**Why:** Establish the new owner type and move the narrowest field first (9 sites, 2 files) so
the wrapper exists and is exercised before the counters follow.

**Files:**
- Create: `src/sim/world/substrate.rs`
- Modify: `src/sim/world/mod.rs` (declare module; field swap; method paths)
- Modify: `src/sim/world/world_hash.rs:52`
- Modify: `src/sim/snapshot.rs:18` (version bump)

**Pattern:** Mirrors existing `sim/world/` submodules (e.g. `logic_vector.rs`) — `//!` header,
plain struct, `#[derive]` + custom-attribute serde, `#[cfg(test)]` unit tests.

**Step 1 — Create the substrate module (only `logic` for now).**
```rust
// src/sim/world/substrate.rs
//! The object substrate: the single owner of the active-object vector and the
//! monotonic identity/enter-order counters that the lifecycle contract mutates.
//!
//! Slice 1 wrapper: holds the bookkeeping/ordering state only. The lifecycle
//! methods (reveal/conceal/unlimbo/uninit) stay on `Simulation` because they
//! also touch `EntityStore`/`OccupancyGrid`; they read this struct by path.
//!
//! Dependency rules: part of sim/ — depends only on std + serde + sim/world::LogicVector.

use serde::{Deserialize, Serialize};

use super::logic_vector::LogicVector;

/// Owns the active-object order and the substrate's monotonic counters.
/// Field paths are `Simulation.substrate.*`. See the module header for why the
/// lifecycle methods are not (yet) defined here.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct ObjectSubstrate {
    /// LogicClass active-object vector — the single authority on object order.
    /// Tail-append on reveal, compacting-remove on conceal. Serialized verbatim.
    #[serde(default)]
    pub(crate) logic: LogicVector,
    // next_stable_entity_id / next_occupancy_enter_order added in Tasks 2–3.
}
```

**Step 2 — Wire the module + re-export in `world/mod.rs`.**
Find the existing `mod logic_vector;` (or the module declaration block near the top of
`src/sim/world/mod.rs`) and add alongside it:
```rust
mod substrate;
pub(crate) use substrate::ObjectSubstrate;
```
(If `logic_vector` is declared `pub(crate) mod`, match that visibility for `substrate`.)

**Step 3 — Swap the `logic` field on `Simulation`.**
In `src/sim/world/mod.rs`, delete the `logic` field (lines ~320–323):
```rust
    /// LogicClass active-object vector — the single authority on object order.
    /// Tail-append on reveal, compacting-remove on conceal. Serialized verbatim.
    #[serde(default)]
    pub(crate) logic: LogicVector,
```
and in its place (keep it adjacent to `next_stable_entity_id`/`next_occupancy_enter_order` so the
serialized order stays grouped) add:
```rust
    /// Object substrate — active-object order + monotonic id/enter-order counters.
    /// The single owner the lifecycle contract (reveal/conceal/unlimbo/uninit) mutates.
    #[serde(default)]
    pub(crate) substrate: ObjectSubstrate,
```
Update `Simulation::new()` (the constructor that initializes these fields) — replace the
`logic: LogicVector::new()` (or `LogicVector::default()`) initializer with
`substrate: ObjectSubstrate::default()`.

**Step 4 — Rewrite the 8 `logic` paths in `world/mod.rs`.**
Replace `self.logic` with `self.substrate.logic` in every method body:
`register_live_object` (712), `unregister_live_object` (724),
`debug_assert_logic_membership_consistent` (787), `live_object_order_snapshot` (808),
`for_each_live_object` (827, 828), `set_logic_order_for_test` (842),
`rebuild_logic_membership` (1042). Exact transforms:
```rust
// 712:  self.logic.push(stable_id);          -> self.substrate.logic.push(stable_id);
// 724:  self.logic.remove(stable_id);         -> self.substrate.logic.remove(stable_id);
// 787:  let order = self.logic.as_slice();    -> let order = self.substrate.logic.as_slice();
// 808:  self.logic.snapshot()                 -> self.substrate.logic.snapshot()
// 827:  while i < self.logic.len() {          -> while i < self.substrate.logic.len() {
// 828:  let id = self.logic.as_slice()[i];    -> let id = self.substrate.logic.as_slice()[i];
// 842:  self.logic.set_order_for_test(order); -> self.substrate.logic.set_order_for_test(order);
// 1042: for &id in &self.logic.snapshot() {   -> for &id in &self.substrate.logic.snapshot() {
```

**Step 5 — Rewrite the 1 `logic` path in `world_hash.rs`.**
```rust
// src/sim/world/world_hash.rs:52
// let order = self.logic.as_slice();
let order = self.substrate.logic.as_slice();
```

**Step 6 — Bump the snapshot version.**
```rust
// src/sim/snapshot.rs:15-18
/// Bump this when the snapshot binary format changes in a breaking way.
// Bumped 14 -> 15: active-vector + substrate counters relocated under
// `Simulation.substrate` (ObjectSubstrate); bincode layout changed (hash unchanged).
const SNAPSHOT_VERSION: u32 = 15;
```

**Step 7 — Verify.**
Run: `cargo check -p vera20k`
Expected: compiles clean (no remaining `self.logic` / `sim.logic`). Then
`cargo test -p vera20k logic_vector` and `cargo test -p vera20k world_hash` — Expected: PASS.

**Step 8 — Commit.** `refactor(sim): introduce ObjectSubstrate, move active-vector under it`

---

### Task 2: Migrate `next_stable_entity_id` into `ObjectSubstrate`

**Why:** Second field move; small and well-contained (id allocator + a handful of test setups).

**Files:**
- Modify: `src/sim/world/substrate.rs` (add field)
- Modify: `src/sim/world/mod.rs` (`allocate_stable_id`, field removal)
- Modify: `src/sim/world/world_hash.rs:48` and its test modules
- Modify: test files that set `sim.next_stable_entity_id` directly

**Step 1 — Add the field to `ObjectSubstrate`.**
```rust
// src/sim/world/substrate.rs — inside ObjectSubstrate, after `logic`
    /// Monotonic per-instance id source (never reused). gamemd's per-object
    /// unique id; ours is allocate-only, stale refs degrade to None.
    pub(crate) next_stable_entity_id: u64,
```

**Step 2 — Remove the field from `Simulation`** ([mod.rs:315](../../src/sim/world/mod.rs)):
delete `pub(crate) next_stable_entity_id: u64,` and its constructor initializer in
`Simulation::new()` (the substrate's `Default` now provides `0`; confirm `new()` previously set
it to `0` — if it seeded a non-zero start, replicate that in the substrate initializer instead of
`default()`).

**Step 3 — Update `allocate_stable_id`** ([mod.rs:700](../../src/sim/world/mod.rs)):
```rust
    pub(crate) fn allocate_stable_id(&mut self) -> u64 {
        let id = self.substrate.next_stable_entity_id;
        self.substrate.next_stable_entity_id =
            self.substrate.next_stable_entity_id.saturating_add(1);
        id
    }
```

**Step 4 — Update the hash path** (`world_hash.rs:48`):
```rust
// self.next_stable_entity_id.hash(&mut hasher);
self.substrate.next_stable_entity_id.hash(&mut hasher);
```

**Step 5 — Update direct test setters.** Enumerate with:
`rg -n "next_stable_entity_id" src/sim` — then replace each `sim.next_stable_entity_id` (and
`self.next_stable_entity_id` outside the methods already fixed) with
`sim.substrate.next_stable_entity_id`. Known sites include the `let id =
sim.next_stable_entity_id; sim.next_stable_entity_id += 1;` pairs in `world_hash.rs`
(`rocking_hash_tests` ~1219–1220, `c4_hash_tests` ~1298–1299) → both lines gain `.substrate`.

**Step 6 — Verify.** `cargo check -p vera20k` clean; `cargo test -p vera20k world_hash` PASS.

**Step 7 — Commit.** `refactor(sim): move stable-id allocator into ObjectSubstrate`

---

### Task 3: Migrate `next_occupancy_enter_order` into `ObjectSubstrate`

**Why:** Final field move; the enter-order counter (threaded through movement/occupancy code).

**Files:**
- Modify: `src/sim/world/substrate.rs` (add field)
- Modify: `src/sim/world/mod.rs` (`add_entity_occupancy`, field removal)
- Modify: `src/sim/world/world_hash.rs:49`
- Modify: `src/sim/movement/{movement_step,movement_tick,mod}.rs`, `src/sim/ai.rs`, and test files

**Step 1 — Add the field to `ObjectSubstrate`.**
```rust
// src/sim/world/substrate.rs — inside ObjectSubstrate, after next_stable_entity_id
    /// Monotonic source for rebuilt CellClass-style object-list order. Each entity
    /// stores the value assigned when it entered a cell list; OccupancyGrid is a
    /// skipped cache rebuilt from these on load.
    pub(crate) next_occupancy_enter_order: u64,
```

**Step 2 — Remove the field from `Simulation`** ([mod.rs:319](../../src/sim/world/mod.rs)) and its
`Simulation::new()` initializer (same `0`-default check as Task 2 Step 2).

**Step 3 — Update `add_entity_occupancy`** ([mod.rs:762–763](../../src/sim/world/mod.rs)). The
`entity` borrow from `self.entities` is held across this read/write; `self.substrate` is a
disjoint field so the split borrow still compiles:
```rust
        let order = self.substrate.next_occupancy_enter_order;
        self.substrate.next_occupancy_enter_order =
            self.substrate.next_occupancy_enter_order.saturating_add(1);
        entity.occupancy_enter_order = order;
```

**Step 4 — Update the hash path** (`world_hash.rs:49`):
```rust
// self.next_occupancy_enter_order.hash(&mut hasher);
self.substrate.next_occupancy_enter_order.hash(&mut hasher);
```

**Step 5 — Update remaining consumers.** Enumerate with
`rg -n "next_occupancy_enter_order" src/sim` and replace each `sim.`/`self.` access with the
`.substrate.` path. Expected files: `movement/movement_step.rs`, `movement/movement_tick.rs`,
`movement/mod.rs`, `ai.rs`, and test setups (`world_tests`, `deploy_tests`, `miner_tests`,
`production_tests`, `movement_tests`, `prone_speed_tests`, `world_orders_c4_tests`,
`world_orders_bridge_repair_tests`). For any movement function that takes the counter **by `&mut`
parameter** (the "threaded by hand" pattern), only the call-site argument changes
(`&mut sim.next_occupancy_enter_order` → `&mut sim.substrate.next_occupancy_enter_order`); the
function signature is unchanged.

**Step 6 — Verify.** `cargo check -p vera20k` clean (zero remaining bare
`next_occupancy_enter_order` field accesses outside the struct). `cargo test -p vera20k occupancy`
and `cargo test -p vera20k snapshot` PASS.

**Step 7 — Commit.** `refactor(sim): move occupancy enter-order counter into ObjectSubstrate`

---

### Task 4: Slice acceptance — determinism & full-suite verification

**Why:** Prove the slice's contract: hash bit-identical, all lifecycle/occupancy/snapshot
invariants hold. This is the gate per design §8 (Slice 1 accept: "5000-tick replay hash
bit-identical; lifecycle tests unchanged; membership invariant holds").

**Files:**
- Modify: `src/sim/world/world_hash.rs` (add one focused test in a `#[cfg(test)]` module)

**Step 1 — Add a spawn-then-hash determinism test.** Confirms two independently built sims that
exercise the substrate (id allocation + reveal/conceal ordering) produce identical hashes — the
property the relocation must preserve.
```rust
#[cfg(test)]
mod substrate_determinism_tests {
    use super::Simulation;

    fn build() -> Simulation {
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let ty = sim.interner.intern("HTNK");
        // Allocate ids through the substrate and register/unregister to drive
        // the active-vector order.
        let mut ids = Vec::new();
        for i in 0..8u16 {
            let id = sim.allocate_stable_id();
            let e = crate::sim::game_entity::GameEntity::new(
                id, 10 + i, 10, 0, 0, owner,
                crate::sim::components::Health { current: 400, max: 400 },
                ty, crate::map::entities::EntityCategory::Unit, 0, 5, true,
            );
            sim.entities.insert(e);
            sim.reveal(id);
            ids.push(id);
        }
        // Conceal two in the middle to exercise compacting removal order.
        sim.conceal(ids[2]);
        sim.conceal(ids[5]);
        sim
    }

    #[test]
    fn substrate_state_hash_is_reproducible() {
        let a = build();
        let b = build();
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "substrate-driven state must hash identically across independent builds",
        );
    }

    #[test]
    fn substrate_membership_invariant_holds() {
        let sim = build();
        #[cfg(debug_assertions)]
        sim.debug_assert_logic_membership_consistent();
        // 8 revealed, 2 concealed -> 6 live members.
        assert_eq!(sim.live_object_order_snapshot().len(), 6);
    }
}
```
*(Adjust the `GameEntity::new` argument list if the signature differs — mirror an existing
`world_hash.rs` test such as `tube_movement_hash_tests` which already calls `GameEntity::new`.)*

**Step 2 — Run the targeted suites.**
- `cargo test -p vera20k world_hash`
- `cargo test -p vera20k snapshot`
- `cargo test -p vera20k occupancy`
- `cargo test -p vera20k -- logic` (LogicVector + lifecycle)
Expected: all PASS. Read the literal `test result:` lines (per project rule — do not infer).

**Step 3 — Run the full suite + lint.**
- `cargo test -p vera20k`
- `cargo clippy -p vera20k`
Expected: green; no new clippy findings on `substrate.rs`.

**Step 4 — Confirm no behavioral drift.** Grep that nothing still references the old field
paths: `rg -n "self\.(logic|next_stable_entity_id|next_occupancy_enter_order)\b" src` and
`rg -n "\.\b(next_stable_entity_id|next_occupancy_enter_order)\b" src` should return **only**
`self.substrate.…` matches (or matches inside `substrate.rs` itself).

**Step 5 — Commit.** `test(sim): add ObjectSubstrate determinism + membership invariant tests`

---

### Task 5 (verification gate, not code): gamemd parity confirmation

**Why:** Slice 1 introduces **no new gamemd behavior**, so the parity bar is "observably
identical to the pre-refactor engine," not "matches gamemd anew." This task records that the
acceptance evidence is the determinism/replay-hash identity (Task 4), **not** a binary trace.

**Verify:**
- The state hash is preserved **by construction** (Grounding Summary: world_hash reads the same
  values in the same order via new paths). Task 4's tests are the regression guard.
- No `SNAPSHOT_VERSION`-gated old-save compatibility is claimed (saves are engine-private;
  version bumped intentionally).
- **Expected result:** identical in-engine behavior before/after; the design's Slice 1 acceptance
  ("hash bit-identical; lifecycle tests unchanged; membership invariant holds") is met without a
  new golden or gamemd artifact (those are required only for Slices 6/7).

## Sources & References

- **Design doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](../research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md) — Slice 1 (§8), boundary design (§6), contract C8/C9/C15/C16.
- **Underlying binary evidence (background; no new RE needed for Slice 1):**
  `docs/research/CELLCLASS_SUBSTRATE_LIVE_OBJECT_LIST_WRITERS_GHIDRA_REPORT.md`;
  `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`;
  `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`;
  `docs/research/ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md`.
- **Current Rust touchpoints:**
  [src/sim/world/mod.rs:266](../../src/sim/world/mod.rs) (Simulation struct + lifecycle methods 700–903, 1001–1047);
  [src/sim/world/logic_vector.rs:13](../../src/sim/world/logic_vector.rs);
  [src/sim/entity_store.rs:33](../../src/sim/entity_store.rs);
  [src/sim/occupancy.rs:98](../../src/sim/occupancy.rs);
  [src/sim/world/world_hash.rs:33](../../src/sim/world/world_hash.rs) (hash 48,49,52,387);
  [src/sim/snapshot.rs:18](../../src/sim/snapshot.rs) (SNAPSHOT_VERSION);
  [src/sim/game_entity.rs:138](../../src/sim/game_entity.rs) (`stable_id`/`in_logic_vector`/`occupancy_enter_order`).
- **INI keys:** none (pure refactor).
- **No new crates.** `substrate.rs` uses only `serde` + std + `sim/world::LogicVector`.
