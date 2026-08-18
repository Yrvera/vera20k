# Incremental `by_owner` Index + Owner-Change Chokepoint (Slice 4) — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make `EntityStore.by_owner` maintained incrementally (insert / remove /
`change_owner`), delete the dead per-tick full rebuild, and route every live owner mutation
through one `change_owner` chokepoint — replay hash bit-identical, no `SNAPSHOT_VERSION` bump.

**Architecture:** Slice 4 of the `ObjectSubstrate` consolidation (design §7 item 4, §8 Slice
4). Mirrors Slice 3's pattern — a single funnel owns a cross-cutting invariant, replacing
hand-maintenance. Pure determinism refactor: `by_owner` is not hashed and has no live
consumer, so the change cannot move the state hash.

**Design Doc:** [docs/plans/2026-05-31-slice4-incremental-by-owner-design.md](2026-05-31-slice4-incremental-by-owner-design.md)

---

## Grounding Summary

- **Design doc (§7.4, §8 Slice 4):** substrate updates `by_owner` + owned-counts on
  unlimbo/uninit/change_owner; drop the per-tick rebuild (keep for deserialize); route
  capture through `change_owner`. Hash-identical, no version bump, no gamemd artifact.
- **Current code (grounded this session):**
  - `EntityStore.by_owner` (`entity_store.rs:40`) is rebuilt wholesale by
    `rebuild_owner_index()` (`entity_store.rs:145`); `insert`/`remove` do NOT touch it.
  - Per-tick rebuild: `mod.rs:1644` (`advance_tick` top) — O(N) every tick.
  - Deserialize finalizer: `EntityStore::deserialize` (`entity_store.rs:161-170`) calls
    `rebuild_owner_index` after bulk-loading the primary map.
  - **`ids_for_owner` has ZERO live consumers** — every call is in `entity_store.rs` tests.
    The per-tick rebuild builds an index nothing reads; `by_owner` ordering cannot affect the
    hash today.
  - Owned-counts (`HouseState.owned_building_count`/`owned_unit_count`, `house_state.rs:38,40`)
    are a separate per-house tally, **hashed** (`world_hash.rs:136-137`), maintained by
    `increment_owned_count`/`decrement_owned_count` (`mod.rs:904,918`). Independent of `by_owner`.
- **GROUNDING CORRECTION (design under-counted live owner-mutation sites):** the design named
  two; there are **three** distinct live `entity.owner =` mutation paths (four writes):
  1. Engineer capture — `world_orders.rs:233`, `b.owner = engineer_owner`, **then**
     `decrement_owned_count(old)` + `increment_owned_count(new)`.
  2. Garrison reconcile occupy — `passenger.rs:600`, `building.owner = new_owner`, no counts.
  3. Garrison reconcile revert-to-civilian — `passenger.rs:611`, `building.owner = civilian_owner`, no counts.
  4. **Garrison eject placement — `production_sell.rs:438`**, `pax.owner = owner` under
     `owner_override`, in `place_garrison_passenger_at_cell` (live; reached by
     `eject_red_hp_garrison` + `eject_destruction_garrison`), no counts. **Missed by the design.**
  All other `.owner =` writes in `src/sim` are `#[cfg(test)]` helpers (genetic_converter
  307/317, lightning_storm 407/427/435, passenger 1156/1181/1895/1995/2032/2053/2157,
  production_sell 879/985, drop_payload, paradrop_mission, snapshot, entity_store) or
  spawn-time field init — NOT post-spawn live mutations. Re-verified by grep of `\.owner =`
  across `src/sim` excluding test files + reading each ambiguous site.
- **Mind control** does NOT change `owner` — it sets `entity.mind_controlled` (game_entity.rs:310);
  not an owner-mutation site.
- **Repo pattern this mirrors:** Slice 3's `place_spawned` chokepoint (`world_spawn.rs`) and
  the existing `EntityStore` method surface.
- **INI keys:** none.
- **Still unknown after grounding:** whether gamemd adjusts house building-counts on
  civilian-garrison ownership transfer (sites 2-4 don't today). Deferred — it's a separate
  hash-changing parity question (background task spawned). This slice preserves current
  no-count behavior verbatim.

## Key Technical Decisions

- **Incremental index lives in `insert`/`remove`/`change_owner`; per-tick rebuild deleted;
  deserialize keeps `rebuild_owner_index`.** **Confidence:** high — **Source:** design §8
  Slice 4 + grounded current code (`entity_store.rs:145-170`, `mod.rs:1644`).
- **`change_owner` owns the INDEX ONLY, not owned-counts.** Counts stay inline at each site:
  engineer keeps its decrement+increment, garrison/eject keep zero. **Confidence:** high —
  **Source:** the live sites legitimately differ on counts (`world_orders.rs:240-242` vs
  `passenger.rs:599-613` / `production_sell.rs:437-438`); unifying counts would change the
  hashed counts. User-approved "Index-only" option this session.
- **All four live owner-write sites route through `change_owner`.** **Confidence:** high —
  **Source:** grep + per-site read this session (the correction above). Required for index
  correctness once it's incremental.
- **`index_add` uses sorted (binary-search) insert; `index_remove` drops emptied buckets.**
  **Confidence:** high — **Source:** must equal `rebuild_owner_index` output, which produces
  ascending-id Vecs and omits empty owners (`entity_store.rs:145-152`).

## Open Questions

### Resolved During Planning
- **How many live owner-mutation sites?** Three paths / four writes (see correction above) —
  enumerated and each routed in Tasks 3-5.
- **Does `change_owner` touch counts?** No — index only; counts stay inline (hash-identical).
- **Does deserialize still rebuild?** Yes — primary map is bulk-loaded, bypassing `insert`,
  so the explicit finalizer stays.

### Deferred to Implementation
- None. The incremental≡rebuild test (Task 1) and full replay-hash suite (Task 6) are the
  oracles; if a routing site was missed, the equality test or a determinism test surfaces it.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/entity_store.rs` | Incremental `insert`/`remove`; add `change_owner` + `index_add`/`index_remove`; update doc; rewrite stale tests; add incremental≡rebuild test |
| Modify | `src/sim/world/mod.rs` | Add `Simulation::change_owner` delegate; delete per-tick `rebuild_owner_index()` (Task 6) |
| Modify | `src/sim/world/world_orders.rs` | Route engineer-capture owner write through `change_owner` (keep counts) |
| Modify | `src/sim/passenger.rs` | Route both garrison-reconcile owner writes through `change_owner` (no counts) |
| Modify | `src/sim/production/production_sell.rs` | Route garrison-eject `owner_override` write through `change_owner` (no counts) |

## Interface Changes

- **Added:** `EntityStore::change_owner(&mut self, stable_id: u64, new_owner: InternedId)` —
  moves the `by_owner` entry + sets `entity.owner`; index only. `pub`.
- **Added:** `Simulation::change_owner(&mut self, stable_id: u64, new_owner: InternedId)` —
  thin delegate to the store. `pub(crate)`.
- **Behavior change (internal):** `EntityStore::insert`/`remove` now maintain `by_owner`
  incrementally (signatures unchanged). The module doc + two unit tests asserting the old
  "rebuilt, not auto-synced" contract are updated.
- **Removed:** the per-tick `rebuild_owner_index()` call (`mod.rs:1644`). The method itself is
  retained for the deserialize finalizer.
- No serialized-layout change (`by_owner` was never serialized; deserialize still finalizes
  via rebuild). No `SNAPSHOT_VERSION` bump.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (index bookkeeping; no arithmetic on sim quantities).
- [x] New state included in deterministic state hash — **no new hashed state.** `by_owner` is
      not hashed and has no live consumer; owned-counts maintenance is unchanged.
- [x] No dependencies on render/ui/sidebar/audio/net — all edits in `sim/`.
- [x] Tick ordering impact — the deleted per-tick rebuild ran at tick top before command
      application; nothing consumed its output, so removal changes no ordering.
- [x] BTreeMap iteration order — `by_owner` Vecs stay ascending-id (sorted insert), identical
      to rebuild output; primary `entities` map untouched.

## Risk Areas

- **Missed live owner-write site → silent index desync.** Mitigation: grounding enumerated all
  four live writes; Tasks 3-5 route every one; Task 1's incremental≡rebuild test + Task 6's
  determinism suite catch any divergence. **Highest-stakes guard in the slice.**
- **`index_add` push-to-end instead of sorted insert → order ≠ rebuild.** Mitigation: Task 1
  uses `partition_point` sorted insert; incremental≡rebuild test asserts equality.
- **Emptied bucket left as empty Vec → `by_owner` map keys ≠ rebuild keys.** Mitigation:
  `index_remove` drops the key when the Vec empties; equality test covers a wiped-out owner.
- **Borrow conflict** routing `get_mut().owner =` through `change_owner` (which needs `&mut
  self`). Mitigation: each routing task drops the entity borrow before the call (exact code
  given per task).
- **Deleting the rebuild before all sites are routed.** Mitigation: rebuild is deleted LAST
  (Task 6), after Tasks 3-5 route every site — so every intermediate commit is correct even
  though the rebuild (redundant safety net) still runs.

## Parity-Critical Items

Determinism-preserving refactor — the parity stake is **absence of change** to hashed state.

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `by_owner` order = ascending id, emptied buckets dropped | Must equal `rebuild_owner_index` so deserialize-rebuild ≡ incremental | `incremental_index_matches_rebuild` unit test |
| Tasks 3-5 | owned-counts unchanged at every routed site | Counts are hashed; any added/removed count call changes the replay hash | engineer/garrison/eject tests stay green; Task 6 replay hash bit-identical |
| Task 6 | Replay hash + snapshot unchanged after rebuild deletion | The slice's whole guarantee is determinism-preserving | full lib suite green; `SNAPSHOT_VERSION` unchanged |

---

## Tasks

### Task 1: `EntityStore` — incremental index + `change_owner` + tests

**Why:** Establish the index-owning funnel first; everything else routes into it. The
per-tick rebuild (still present) coexists harmlessly — it clears+rebuilds, so incremental
updates are merely redundant until removed in Task 6.

**Files:**
- Modify: `src/sim/entity_store.rs` (struct doc ~27-40; `insert` 52-57; `remove` 59-62;
  add helpers + `change_owner` near `ids_for_owner`/`rebuild_owner_index` 136-152; tests).

**Pattern:** Mirrors Slice 3's chokepoint approach — a single funnel owns a cross-cutting
invariant. Uses `partition_point`/`binary_search` on the existing sorted-Vec representation.

**Step 1: Update the struct + field doc comments** — replace lines 27-40
(the `/// Maintains a secondary per-owner index...` block through the `by_owner` field doc):
```rust
/// Maintains a secondary per-owner index (`by_owner`) so that queries like
/// "all buildings owned by house X" are O(that house's entities) instead of
/// O(total entities). The index is maintained **incrementally**: `insert`,
/// `remove`, and `change_owner` keep it in sync, so `ids_for_owner()` is always
/// current with no rebuild needed. `rebuild_owner_index()` exists only for the
/// deserialize finalizer (the primary map is bulk-loaded, bypassing `insert`).
#[derive(Debug, Clone)]
pub struct EntityStore {
    /// Primary storage: stable_id -> GameEntity.
    entities: BTreeMap<u64, GameEntity>,
    /// Per-owner index: owner InternedId -> ascending-stable_id Vec of ids.
    /// Maintained incrementally by `insert`/`remove`/`change_owner`. Emptied
    /// owners are dropped from the map so a wiped-out house's `ids_for_owner`
    /// returns `&[]`, identical to a fresh rebuild. Deterministic iteration via
    /// BTreeMap key order + sorted Vecs.
    by_owner: BTreeMap<crate::sim::intern::InternedId, Vec<u64>>,
}
```

**Step 2: Make `insert`/`remove` incremental** — replace the current `insert` (52-57) and
`remove` (59-62):
```rust
    /// Insert an entity. Returns its stable_id. Maintains the `by_owner` index.
    /// If an entity with the same id already existed (rare — stable_ids are
    /// monotonic), its old owner entry is removed first.
    pub fn insert(&mut self, entity: GameEntity) -> u64 {
        let id = entity.stable_id;
        let owner = entity.owner;
        if let Some(old) = self.entities.insert(id, entity) {
            self.index_remove(old.owner, id);
        }
        self.index_add(owner, id);
        id
    }

    /// Remove an entity by stable_id. Returns the removed entity if it existed.
    /// Maintains the `by_owner` index.
    pub fn remove(&mut self, stable_id: u64) -> Option<GameEntity> {
        let removed = self.entities.remove(&stable_id);
        if let Some(ref e) = removed {
            self.index_remove(e.owner, stable_id);
        }
        removed
    }
```

**Step 3: Add `change_owner` + index helpers** — insert immediately after `ids_for_owner`
(after line 141, before `rebuild_owner_index`):
```rust
    /// Move an entity to a new owner: updates `entity.owner` AND the `by_owner`
    /// index together. Index only — does NOT touch HouseState owned-counts
    /// (callers own that, because count semantics differ by transfer kind).
    /// No-op if the entity is absent or already owned by `new_owner`.
    pub fn change_owner(&mut self, stable_id: u64, new_owner: crate::sim::intern::InternedId) {
        let old_owner = match self.entities.get_mut(&stable_id) {
            Some(e) if e.owner != new_owner => {
                let old = e.owner;
                e.owner = new_owner;
                old
            }
            _ => return,
        };
        self.index_remove(old_owner, stable_id);
        self.index_add(new_owner, stable_id);
    }

    /// Insert `id` into its owner bucket at the sorted (ascending) position.
    fn index_add(&mut self, owner: crate::sim::intern::InternedId, id: u64) {
        let v = self.by_owner.entry(owner).or_default();
        let pos = v.partition_point(|&x| x < id);
        v.insert(pos, id);
    }

    /// Remove `id` from its owner bucket; drop the bucket if it empties (so the
    /// map matches a fresh rebuild, which never stores empty owners).
    fn index_remove(&mut self, owner: crate::sim::intern::InternedId, id: u64) {
        if let Some(v) = self.by_owner.get_mut(&owner) {
            if let Ok(pos) = v.binary_search(&id) {
                v.remove(pos);
            }
            if v.is_empty() {
                self.by_owner.remove(&owner);
            }
        }
    }
```

**Step 4: Replace the two stale unit tests** — delete `test_owner_transfer_captured_by_rebuild`
(450-476) and `insert_does_not_auto_sync_owner_index` (478-492); replace both with:
```rust
    #[test]
    fn insert_indexes_immediately() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");
        let mut store = EntityStore::new();
        let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e.owner = americans;
        store.insert(e);
        // No rebuild: the index is current right after insert.
        assert_eq!(store.ids_for_owner(americans), &[1]);
    }

    #[test]
    fn remove_deindexes_immediately() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");
        let mut store = EntityStore::new();
        let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e.owner = americans;
        store.insert(e);
        store.remove(1);
        // Bucket emptied → owner dropped, identical to a fresh rebuild.
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);
    }

    #[test]
    fn change_owner_moves_entry_immediately_and_is_idempotent() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");
        let soviets = interner.intern("Russians");
        let mut store = EntityStore::new();
        let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e.owner = americans;
        store.insert(e);

        store.change_owner(1, soviets);
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);
        assert_eq!(store.ids_for_owner(soviets), &[1]);
        assert_eq!(store.get(1).unwrap().owner, soviets);

        // Same-owner call is a no-op (no duplicate in the bucket).
        store.change_owner(1, soviets);
        assert_eq!(store.ids_for_owner(soviets), &[1]);

        // Missing id is a no-op.
        store.change_owner(999, americans);
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);
    }

    #[test]
    fn change_owner_preserves_sorted_order_in_both_buckets() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let a = interner.intern("Americans");
        let b = interner.intern("Russians");
        let mut store = EntityStore::new();
        for id in [10u64, 20, 30] {
            let mut e = GameEntity::test_default(id, "HTNK", "Americans", 5, 5);
            e.owner = a;
            store.insert(e);
        }
        let mut e = GameEntity::test_default(15, "RHNO", "Russians", 6, 6);
        e.owner = b;
        store.insert(e);
        // Move 20 from a→b; both buckets must stay ascending.
        store.change_owner(20, b);
        assert_eq!(store.ids_for_owner(a), &[10, 30]);
        assert_eq!(store.ids_for_owner(b), &[15, 20]);
    }

    /// Acceptance: a store built purely by incremental ops has a `by_owner`
    /// byte-identical to one produced by a full rebuild — proving
    /// deserialize-rebuild ≡ incremental.
    #[test]
    fn incremental_index_matches_rebuild() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let a = interner.intern("Americans");
        let b = interner.intern("Russians");
        let c = interner.intern("Yuri");
        let mut store = EntityStore::new();
        for (id, owner) in [(5u64, a), (1, b), (3, a), (2, c), (4, b)] {
            let mut e = GameEntity::test_default(id, "HTNK", "Americans", 5, 5);
            e.owner = owner;
            store.insert(e);
        }
        store.change_owner(3, b); // a→b
        store.change_owner(2, a); // c→a (empties c)
        store.remove(5); // drops from a
        let incremental = store.by_owner.clone();
        store.rebuild_owner_index();
        assert_eq!(incremental, store.by_owner);
    }
```
(Keep `test_per_owner_index` and `test_rebuild_owner_index` as-is — they still pass; the
former's explicit `rebuild_owner_index()` calls are now redundant but harmless.)

**Step 5: Verify**
Run: `cargo test -p vera20k --lib -- entity_store`
Expected: read the literal `test result:` line — all pass, including the four new tests.

**Step 6: Commit** (`refactor(sim): incremental by_owner index + change_owner on EntityStore (Slice 4)`)

---

### Task 2: `Simulation::change_owner` delegate

**Why:** Give above-store callers a chokepoint that doesn't reach into `substrate.entities`
directly, keeping all owner transfers greppable. Added before the routing tasks consume it.

**Files:**
- Modify: `src/sim/world/mod.rs` — add the delegate next to the other substrate-entity
  helpers (e.g. just after `increment_owned_count`/`decrement_owned_count`, ~mod.rs:931).

**Pattern:** Thin delegate, like the existing `entities()`/`entities_mut()` accessors.

**Step 1: Add the method** inside `impl Simulation` (after `decrement_owned_count`):
```rust
    /// Change an entity's owner through the substrate chokepoint: updates the
    /// `by_owner` index and the entity's owner field together. Index only — the
    /// caller owns any HouseState owned-count adjustment (count semantics differ
    /// by transfer kind: engineer capture adjusts counts; garrison transfers do not).
    pub(crate) fn change_owner(&mut self, stable_id: u64, new_owner: InternedId) {
        self.substrate.entities.change_owner(stable_id, new_owner);
    }
```
(`InternedId` is already imported in `mod.rs` — `use crate::sim::intern::InternedId;` at line 49.)

**Step 2: Verify**
Run: `cargo check -p vera20k`
Expected: compiles (method is unused until Task 3 — acceptable single-step window; if a
dead-code warning fires for `pub(crate)`, it will clear in Task 3 which adds the first caller).

**Step 3: Commit** (`refactor(sim): add Simulation::change_owner delegate (Slice 4)`)

---

### Task 3: Route engineer capture through `change_owner`

**Why:** First live caller of `change_owner`. Engineer capture keeps its owned-count calls
(it legitimately adjusts hashed counts); only the owner write moves to the chokepoint.

**Files:**
- Modify: `src/sim/world/world_orders.rs:229-242` (inside `tick_capture_orders`).

**Pattern:** Replace the `get_mut().owner =` block with `self.change_owner(...)`; keep the
surrounding `old_owner` capture and the count calls verbatim.

**Step 1: Replace the owner-write block** — change lines 230-242 from:
```rust
                // CAPTURE: transfer building ownership.
                let old_owner = self.substrate.entities.get(building_id).map(|b| b.owner);
                if let Some(b) = self.substrate.entities.get_mut(building_id) {
                    b.owner = engineer_owner;
                }
                // Update house owned counts for both old and new owner.
                // Resolve interned IDs to strings before &mut self calls.
                let engineer_owner_str = self.interner.resolve(engineer_owner).to_string();
                if let Some(old_owner_id) = old_owner {
                    let old_owner_str = self.interner.resolve(old_owner_id).to_string();
                    self.decrement_owned_count(&old_owner_str, EntityCategory::Structure);
                }
                self.increment_owned_count(&engineer_owner_str, EntityCategory::Structure);
```
to:
```rust
                // CAPTURE: transfer building ownership through the substrate
                // chokepoint (updates by_owner + owner field together).
                let old_owner = self.substrate.entities.get(building_id).map(|b| b.owner);
                self.change_owner(building_id, engineer_owner);
                // Update house owned counts for both old and new owner.
                // Resolve interned IDs to strings before &mut self calls.
                let engineer_owner_str = self.interner.resolve(engineer_owner).to_string();
                if let Some(old_owner_id) = old_owner {
                    let old_owner_str = self.interner.resolve(old_owner_id).to_string();
                    self.decrement_owned_count(&old_owner_str, EntityCategory::Structure);
                }
                self.increment_owned_count(&engineer_owner_str, EntityCategory::Structure);
```
(Only the 4-line `if let Some(b) = ... get_mut ... { b.owner = engineer_owner; }` becomes the
single `self.change_owner(building_id, engineer_owner);`. Count logic unchanged.)

**Step 2: Verify**
Run: `cargo test -p vera20k --lib -- capture`
Expected: existing engineer-capture tests pass (owner transferred, counts unchanged).

**Step 3: Commit** (`refactor(sim): route engineer capture through change_owner (Slice 4)`)

---

### Task 4: Route garrison reconciliation through `change_owner`

**Why:** Garrison reconcile mutates `building.owner` directly at two sites with no count
adjustment. Both must route through `change_owner` (no count calls added) or the incremental
index desyncs.

**Files:**
- Modify: `src/sim/passenger.rs:599-601` and `passenger.rs:610-612` (inside
  `reconcile_civilian_garrison_owner_for_building`).

**Pattern:** Replace each `get_mut().owner =` block with `sim.change_owner(...)`; add no count
calls (preserves current no-count behavior verbatim → hash-identical).

**Step 1: Route the occupy-transfer** — replace lines 599-601:
```rust
        if let Some(building) = sim.substrate.entities.get_mut(building_id) {
            building.owner = new_owner;
        }
        return true;
```
with:
```rust
        sim.change_owner(building_id, new_owner);
        return true;
```

**Step 2: Route the revert-to-civilian** — replace lines 610-612:
```rust
        if let Some(building) = sim.substrate.entities.get_mut(building_id) {
            building.owner = civilian_owner;
        }
        return current_owner != civilian_owner;
```
with:
```rust
        sim.change_owner(building_id, civilian_owner);
        return current_owner != civilian_owner;
```
(`sim` is `&mut Simulation` in this free function — the delegate is in scope.)

**Step 3: Verify**
Run: `cargo test -p vera20k --lib -- garrison`
Expected: garrison reconcile tests pass — ownership still transfers/reverts; counts unchanged
(none were ever adjusted here).

**Step 4: Commit** (`refactor(sim): route garrison reconcile through change_owner (Slice 4)`)

---

### Task 5: Route garrison-eject placement through `change_owner`

**Why:** The grounding correction — `place_garrison_passenger_at_cell` mutates `pax.owner`
under `owner_override` (live, reached by red-HP + destruction garrison eject). Must route or
the incremental index desyncs. No counts (none today).

**Files:**
- Modify: `src/sim/production/production_sell.rs:433-439` (inside
  `place_garrison_passenger_at_cell`).

**Pattern:** Apply the owner change via `sim.change_owner` BEFORE taking the `get_mut` borrow
for the remaining field writes (avoids a borrow conflict; ordering is observably identical —
`change_owner` touches only index + owner field, and `reveal` happens later at line 459).

**Step 1: Reorder the owner write ahead of the borrow** — replace lines 433-439:
```rust
    let Some(pax) = sim.substrate.entities.get_mut(passenger_id) else {
        return false;
    };
    pax.passenger_role = PassengerRole::None;
    if let Some(owner) = owner_override {
        pax.owner = owner;
    }
    pax.position.rx = rx;
```
with:
```rust
    // Owner transfer (if any) goes through the substrate chokepoint first, so
    // the by_owner index stays in sync; then take the mutable borrow for the
    // remaining field writes. change_owner is a no-op if the id is absent —
    // the get_mut below still guards absence.
    if let Some(owner) = owner_override {
        sim.change_owner(passenger_id, owner);
    }
    let Some(pax) = sim.substrate.entities.get_mut(passenger_id) else {
        return false;
    };
    pax.passenger_role = PassengerRole::None;
    pax.position.rx = rx;
```
(The remaining `pax.position.*` / `pax.sub_cell` / `refresh_screen_coords` / `reveal` lines
below are unchanged.)

**Step 2: Verify**
Run: `cargo test -p vera20k --lib -- garrison eject sell`
Expected: red-HP eject, destruction eject, and player-sell garrison tests pass.

**Step 3: Commit** (`refactor(sim): route garrison-eject owner override through change_owner (Slice 4)`)

---

### Task 6: Delete the per-tick rebuild; full-suite verification

**Why:** With every live owner-write routed (Tasks 3-5) and insert/remove incremental
(Task 1), the per-tick rebuild is dead cost. Delete it and confirm the whole slice is
hash-identical.

**Files:**
- Modify: `src/sim/world/mod.rs:1642-1644` (remove the per-tick rebuild call).

**Step 1: Delete the per-tick rebuild** — remove lines 1642-1644:
```rust
        // Rebuild per-owner entity index. Cheap linear scan; captures any
        // owner mutations from the previous tick (engineer capture, mind control).
        self.substrate.entities.rebuild_owner_index();
```
(Leave `rebuild_owner_index` defined in `entity_store.rs` — the deserialize finalizer
`EntityStore::deserialize` still calls it. Confirm via grep in Step 3.)

**Step 2: Full lib suite**
Run: `cargo test -p vera20k --lib 2>&1 | tail -5`
Expected: read the literal `test result:` line — all pass. Replay-hash / `world_hash` /
snapshot / lifecycle tests unchanged; per-tick membership + presence asserts (Slices 1-2)
do not fire.

**Step 3: Confirm rebuild still reachable for deserialize + no stray per-tick caller**
Run: `cargo test -p vera20k --lib -- saveload` and grep `rebuild_owner_index`.
Expected: `saveload_*` tests green; grep shows `rebuild_owner_index` defined in
`entity_store.rs`, called from its `Deserialize` impl, and NO remaining call in
`advance_tick` (`mod.rs`). Targeted determinism: `cargo test -p vera20k --lib -- saveload_occupancy_list_order_matches_incremental` green (rebuild-after-load path intact).

**Step 4: Clippy on touched code**
Run: `cargo clippy -p vera20k 2>&1` and confirm no new warnings reference `entity_store`,
`change_owner`, `index_add`, or `index_remove` (pre-existing unrelated lints may remain).

**Step 5: Commit** (`refactor(sim): drop per-tick by_owner rebuild; index now incremental (Slice 4)`)

---

### Task 7: Verification against the design contract (no gamemd binary work needed)

**Why:** Confirm Slice 4's acceptance clauses. No new gamemd-matching behavior is introduced,
so per §8 no gamemd-side evidence artifact is required.

**Verify:**
- **Hash identical:** Task 6 full suite + replay-hash/world_hash tests unchanged → satisfied.
- **Mid-tick capture → `ids_for_owner(new)` immediate, no rebuild:** Task 1's
  `change_owner_moves_entry_immediately_and_is_idempotent` proves the index updates
  synchronously; Tasks 3-5 route every live transfer through it → satisfied. (Intentional
  staleness fix; no live consumer depended on the old stale window — grounding confirmed
  `ids_for_owner` has zero live callers.)
- **deserialize-rebuild ≡ incremental:** Task 1's `incremental_index_matches_rebuild` asserts
  byte-equal `by_owner` → satisfied.
- **owned-counts unchanged:** Tasks 3-5 added/removed no count calls; counts hashed and tests
  green → satisfied.

**Expected result:** all clauses hold; `by_owner` is incrementally correct, the per-tick
rebuild is gone, and all four live owner transfers route through one chokepoint with zero
hash change.

## Sources & References

- **Design doc:** [docs/plans/2026-05-31-slice4-incremental-by-owner-design.md](2026-05-31-slice4-incremental-by-owner-design.md)
- **Design (parent):** docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md — §7 item 9/4
  (line 212/217), §8 Slice 4 (line 231), critic #12 (enter-order hashed; by_owner is not).
- **Current code:** `src/sim/entity_store.rs` (by_owner 40, insert 53, remove 60,
  ids_for_owner 139, rebuild_owner_index 145, deserialize 161-170), `src/sim/world/mod.rs`
  (per-tick rebuild 1644, change_owner delegate target ~931, InternedId import 49,
  increment/decrement_owned_count 904/918), `src/sim/world/world_orders.rs` (capture 229-242),
  `src/sim/passenger.rs` (reconcile owner writes 600/611), `src/sim/production/production_sell.rs`
  (place_garrison_passenger_at_cell owner_override 437-438, eject callers 600/624),
  `src/sim/house_state.rs` (owned counts 38/40), `src/sim/world/world_hash.rs` (counts hashed
  136-137).
- **Prior slice commits:** Slice 3 `df59c36`/`bfb6cfe`/`a58e8fd`/`fc9c461`; Slice 2 `012d792`;
  Slice 1b `8197728`; Slice 1a `d924b20`.
- **INI keys:** none.
- **Deferred follow-up (background task):** whether gamemd adjusts house building-counts on
  civilian-garrison ownership transfer (sites 2-4 don't today) — separate hash-changing slice.
