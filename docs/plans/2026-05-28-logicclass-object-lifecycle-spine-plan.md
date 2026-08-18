# LogicClass Scheduler + ObjectClass Lifecycle Spine — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Commit after each.

**Goal:** Build gamemd's active-object spine in Rust — a `LogicVector` order owner
with a `+0x98`-equivalent membership flag, faithful tail-append / compacting-remove
ops, direct save/load of the order, and membership derived on load — fixing the
three current DRIFTs without touching the broad iteration-order migration.

**Architecture:** `Simulation` already holds a `live_object_order: Vec<u64>` with
register/unregister/snapshot methods whose add/remove *shapes* already match native.
This plan extracts the contract into a `LogicVector` type, adds the membership flag,
removes the sorted fallback (DRIFT 1), stops registering limbo objects (DRIFT 2),
rebuilds membership on load (DRIFT 3), and adds the order to the state hash. The
phased `advance_tick` is unchanged.

**Design Doc:** [docs/plans/2026-05-28-logicclass-object-lifecycle-spine-design.md](2026-05-28-logicclass-object-lifecycle-spine-design.md)

---

## Scope Boundary (read first)

This plan delivers the **spine foundation only**:

- `LogicVector` type + membership flag + lifecycle mechanism (register/unregister =
  reveal/conceal).
- The three DRIFT fixes: sorted-fallback removal, limbo-not-registered (+ paradrop
  reveal-on-drop), membership-derived-on-load.
- Save/load of the order verbatim + state-hash inclusion.

**Deferred to a follow-up plan (named, not silently cut):** the order-authority
migration of the ~55 `keys_sorted()` AI/parity phases to vector order. The design
frames this as incremental and per-phase regression-gated; bundling it here would be
a god-plan. The **only** current consumer of the order
(`passenger.rs` garrison reconciliation at :355) already reads
`live_object_order_snapshot`, so the foundation is independently valuable and a
strict parity improvement the moment it lands. The inter-phase interleaving DRIFT,
global subsystem reorder, and late frame-counter remain deferred per the design.

---

## Grounding Summary

- **Docs:** 5 cited reports — `LOGICCLASS_OBJECT_LIFECYCLE_SPINE_SYSTEM_MODEL_SYNTHESIS`,
  `SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION`, `LOGICCLASS_PERTICKUPDATE_SCHEDULER`,
  `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN`, `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0`.
- **Ghidra (verified live this session):** adder `0x0055BAA0` (+0x98 guard → tail
  insert → set flag); remover `0x0055BAE0` (order-preserving left-shift compaction,
  not swap-remove, clears flag); Reveal `0x005F4EC0` (clears +0x81, gated by
  `type+0x234`/`piVar5[0x8d]` + game-mode, calls adder); Save `0x00551B20` / Load
  `0x00551B90` (count + ordered pointers; restore in saved order then swizzle);
  `ObjectClass::Save 0x005F6250` does **not** serialize `+0x98`; the only `+0x98`
  register-form writer is `0x0055bac6` inside the adder.
- **Repo pattern this mirrors:** `EntityStore` (entity_store.rs) — newtype with a
  manual `Serialize`/`Deserialize` that serializes its inner collection and rebuilds
  derived caches (`rebuild_owner_index`) on deserialize. `LogicVector` follows the
  same shape; membership rebuild follows `rebuild_caches_after_load` (mod.rs:796).
- **INI keys:** none. The spine is structural; `ObjectTypeClass+0x234` is type data,
  not an INI-tunable, and Rust gates membership at the call site (active spawn =
  reveal), not via a parsed key.
- **Still unknown after grounding:** what re-sets `+0x98` in gamemd after load
  (deferred OQ-SL-007). Rust sidesteps it entirely by rebuilding membership from
  vector presence. The broad order-authority migration is a separate effort.

## Key Technical Decisions

- **`LogicVector` newtype owning `order: Vec<u64>`, serialized transparently as
  `Vec<u64>`.** Keeps the existing bincode wire shape (the old field already
  serialized a `Vec<u64>`). — **Confidence:** high — **Source:** repo pattern
  `src/sim/entity_store.rs:152-168`.
- **Membership flag `GameEntity.in_logic_vector: bool`, `#[serde(skip)]`, rebuilt on
  load.** Mirrors `+0x98` (not round-tripped) and the open-gap requirement. —
  **Confidence:** high — **Source:** Ghidra `0x005F6250` (no +0x98 save), verified.
- **Keep method names `register_live_object` / `unregister_live_object` as the
  reveal/conceal mechanism** rather than adding 1:1 `reveal`/`conceal`/`unlimbo`/
  `uninit` alias methods. The behavior contract (not native method names) is what
  parity requires; the `LogicVector` type + doc-comments carry the vocabulary. Minor
  deviation from the design's named-helpers, taken to avoid churn/over-abstraction. —
  **Confidence:** high — **Source:** CLAUDE.md "translate mechanisms, don't port
  literally" + "don't add abstractions beyond what the task requires".
- **Add `live_object_order` to the state hash + bump hash schema.** The order is
  authoritative (garrison reconciliation reads it → affects ownership outcomes) and
  is currently unhashed (pre-existing gap). Membership flag is *derived*, so it is
  NOT separately hashed. — **Confidence:** high — **Source:** Sim Checklist; world_hash.rs.
- **DRIFT-2 fix lands coupled with the sorted-fallback removal.** Today all spawn
  paths register, so the fallback is a no-op until limbo stops registering; the two
  must land together, and the paradrop drop site must reveal. — **Confidence:** high
  — **Source:** read world_spawn.rs:588, drop_payload.rs:203-223.

## Open Questions

### Resolved During Planning

- *Where does a dropped paratrooper become active?* → `drop_payload.rs:203-211`
  repositions the same limbo entity and sets `passenger_role = None`, then attaches
  parachute at :223. Register after a successful parachute attach.
- *Does adding a `GameEntity` field ripple?* → No; helpers funnel through
  `GameEntity::new` (game_entity.rs:394), a single full struct literal. Update that
  one initializer.
- *Is the save wire format broken by the field rename?* → No; bincode is positional
  and `LogicVector` serializes as the same `Vec<u64>`. We bump `SNAPSHOT_VERSION`
  anyway to mark the semantic change (dev saves are disposable).
- *Who consumes the order today?* → only `passenger.rs:355` (garrison reconciliation).

### Deferred to Implementation / Follow-up

- The broad `keys_sorted()` → vector-order migration of AI phases (separate plan).
- Concealing on transport-board / garrison-entry and revealing on unload for the
  *general* case (beyond paradrop). Today those entities stay registered; this is
  part of the deferred migration. Not a regression here because the only consumer is
  garrison reconciliation, which operates on currently-registered entities.
- Whether any limbo-create site other than paradrop exists. Grep shows
  `spawn_object_limbo_at_height` has exactly one caller (paradrop). If a future site
  appears, it must reveal on activation.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/world/logic_vector.rs` | `LogicVector` order owner + contract ops + unit tests |
| Modify | `src/sim/world/mod.rs` | declare module; hold `logic: LogicVector`; rewrite register/unregister/snapshot; despawn unregister-first; membership rebuild on load; hash inclusion; test helper |
| Modify | `src/sim/game_entity.rs` | add `in_logic_vector: bool` (`#[serde(skip)]`) + to `new()` |
| Modify | `src/sim/world/world_spawn.rs` | limbo spawn stops registering (DRIFT 2a) |
| Modify | `src/sim/aircraft/drop_payload.rs` | register paratrooper on successful drop (DRIFT 2b) |
| Modify | `src/sim/world/world_hash.rs` | hash the live object order |
| Modify | `src/sim/snapshot.rs` | bump `SNAPSHOT_VERSION` |
| Modify | `src/sim/passenger.rs` | update `:1351` test to the new order-set API |

## Interface Changes

- `Simulation.live_object_order: Vec<u64>` → `Simulation.logic: LogicVector`
  (`pub(crate)`). Public methods `register_live_object` / `unregister_live_object` /
  `live_object_order_snapshot` keep their signatures — external callers unaffected.
  Direct field access (`passenger.rs:1351` test) migrates to a new
  `set_logic_order_for_test` helper.
- `GameEntity` gains `in_logic_vector: bool`. `GameEntity::new` gains no parameter
  (defaults to `false`); membership is set via `register_live_object`.

## Sim Checklist

- [x] No new f32/f64 — `LogicVector` is `Vec<u64>` + a `bool`; no math.
- [x] New authoritative state (`live_object_order`) added to the state hash; derived
      membership flag intentionally not hashed.
- [x] No dependencies on render/ui/sidebar/audio/net — `logic_vector.rs` imports only
      `std` + serde.
- [x] Tick ordering: `advance_tick` phase sequence unchanged (foundation only).
- [x] BTreeMap iteration order: the whole point — order comes from the `LogicVector`,
      never `keys_sorted()`; the sorted fallback is removed.

## Risk Areas

- **Coupled removal of sorted fallback + limbo registration (highest risk).** If any
  active object is not registered, it silently drops from the order (and from garrison
  reconciliation). Mitigations: a `debug_assert!` invariant that every id in the order
  exists in the store and every member entity is in the order; the audit that all
  three active spawn paths register; and the integration tests in Task 11.
- **Hash schema bump** invalidates existing replays/saves keyed on the hash — expected
  and intended.
- **Paradrop drop reveal** must fire only on successful placement; the rollback path
  (drop_payload.rs:228, parachute attach fails → back to `Inside`) must not leave the
  entity registered. Register *after* `begin_parachute_descent` returns `true`.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Tail-append, no sort | Native append is at the tail (reveal timing), never sorted | Ghidra `0x0055BAA0`; unit test `register_appends_to_tail_no_sort` |
| 1 | Order-preserving compacting remove (not swap-remove) | Native left-shifts; swap-remove would reorder the tail and change AI/recon order | Ghidra `0x0055BAE0`; unit test `unregister_preserves_order_compacting` |
| 1 | Idempotent re-register via membership gate | Re-reveal must not double-append | Ghidra `0x0055BAA5`; unit test `reregister_is_idempotent` |
| 3 | Snapshot is order verbatim — no sorted fallback | Sorted fallback injects non-native order and masks missing registrations | Ghidra (no sort anywhere); test `snapshot_is_order_verbatim` |
| 5,6 | Limbo objects absent from order until reveal | Native registers at reveal, not construction | Ghidra `0x005F4EC0`; test `limbo_object_registers_only_on_reveal` |
| 7 | `+0x98` not serialized; membership derived on load | Native does not round-trip it; deriving avoids the §3.4 stale/double-add hazard | Ghidra `0x005F6250`; test `saveload_restored_member_removes_cleanly` |
| 9,10 | Save order verbatim, cleared before restore | Native serializes the vector directly and restores in saved order | Ghidra `0x00551B20/0x00551B90`; test `saveload_restores_live_object_order_verbatim` |
| 10 | Map-load seed order follows native section sequence | Terrain→Units→Aircraft→Infantry→Structures→Smudge then key order, not sort-by-ID | Ghidra `0x00686B20`; test `map_load_live_object_order_follows_native_section_sequence` (if it fails, parser ordering is a follow-up, flagged not hidden) |

---

## Tasks

### Task 1: Create the `LogicVector` order owner

**Why:** The contract (tail-append, idempotent, compacting-remove, no-sort,
transparent serialization) lives in one testable type before anything consumes it.

**Files:**
- Create: `src/sim/world/logic_vector.rs`

**Pattern:** Newtype with manual serde, mirroring `EntityStore`
(`src/sim/entity_store.rs:152-168`).

**Step 1: Write the type, ops, and serde**
```rust
//! The LogicClass active-object vector: the single authority on object order.
//!
//! Owns an insertion-ordered list of stable_ids. Tail-append on reveal,
//! order-preserving compacting remove on conceal, no sort. Membership itself is
//! tracked by a flag on each entity (see `GameEntity::in_logic_vector`); this type
//! owns only the order. Serializes transparently as its inner `Vec<u64>` so the
//! saved order is restored verbatim.
//!
//! Dependency rules: part of sim/ — depends only on std + serde.

/// Insertion-ordered, membership-gated active-object order.
#[derive(Debug, Default, Clone)]
pub struct LogicVector {
    order: Vec<u64>,
}

impl LogicVector {
    pub fn new() -> Self {
        Self { order: Vec::new() }
    }

    /// Tail-append. Caller guarantees `id` is not already present (the membership
    /// flag guard lives in `Simulation::register_live_object`).
    pub fn push(&mut self, id: u64) {
        self.order.push(id);
    }

    /// Order-preserving compacting remove. No-op if absent. Never swap-remove.
    pub fn remove(&mut self, id: u64) {
        self.order.retain(|&x| x != id);
    }

    /// The order verbatim — no sorted fallback, no filtering.
    pub fn snapshot(&self) -> Vec<u64> {
        self.order.clone()
    }

    /// Borrow the order for hashing / iteration.
    pub fn as_slice(&self) -> &[u64] {
        &self.order
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn clear(&mut self) {
        self.order.clear();
    }

    /// Test-only: force a specific order (e.g. opposite stable-id order).
    #[cfg(test)]
    pub fn set_order_for_test(&mut self, order: Vec<u64>) {
        self.order = order;
    }
}

impl serde::Serialize for LogicVector {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.order.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for LogicVector {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self {
            order: Vec::<u64>::deserialize(deserializer)?,
        })
    }
}
```

**Step 2: Add unit tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_appends_to_tail_no_sort() {
        let mut v = LogicVector::new();
        v.push(5);
        v.push(1);
        v.push(3);
        assert_eq!(v.snapshot(), vec![5, 1, 3]); // insertion order, not sorted
    }

    #[test]
    fn unregister_preserves_order_compacting() {
        let mut v = LogicVector::new();
        v.push(10);
        v.push(20);
        v.push(30);
        v.remove(20);
        assert_eq!(v.snapshot(), vec![10, 30]); // left-shift, tail preserved
    }

    #[test]
    fn unregister_absent_id_is_safe() {
        let mut v = LogicVector::new();
        v.push(1);
        v.remove(99);
        assert_eq!(v.snapshot(), vec![1]);
    }

    #[test]
    fn snapshot_is_order_verbatim() {
        let mut v = LogicVector::new();
        v.push(7);
        v.push(2);
        assert_eq!(v.snapshot(), v.as_slice().to_vec());
    }

    #[test]
    fn serde_roundtrip_preserves_order() {
        let mut v = LogicVector::new();
        v.push(9);
        v.push(4);
        v.push(6);
        let bytes = bincode::serialize(&v).expect("serialize");
        let back: LogicVector = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(back.snapshot(), vec![9, 4, 6]);
    }
}
```

**Step 3: Verify**
Run: `cargo test -p <crate> logic_vector -- --nocapture`
Expected: all 5 PASS.

**Step 4: Commit** — `sim/world: add LogicVector active-object order owner`

---

### Task 2: Add the `in_logic_vector` membership flag to `GameEntity`

**Why:** Mirror `+0x98` for an O(1) dedup guard and a clean invariant anchor. Must
exist before the register/unregister rewrite reads it.

**Files:**
- Modify: `src/sim/game_entity.rs` (struct at :130, `new` at :394)

**Pattern:** Existing `#[serde(skip)]`/`#[serde(default)]` optional markers on
`GameEntity`.

**Step 1: Add the field to the struct** (after `repairing: bool` at game_entity.rs:167)
```rust
    /// LogicClass active-vector membership — mirrors gamemd ObjectClass+0x98.
    /// True iff this entity is currently in `Simulation::logic`. Not serialized:
    /// rebuilt from the restored order on load (native does not round-trip it).
    #[serde(skip)]
    pub in_logic_vector: bool,
```

**Step 2: Initialize it in `GameEntity::new`** (inside the `Self { ... }` literal,
next to `repairing: false,` near game_entity.rs:441)
```rust
            in_logic_vector: false,
```

**Step 3: Verify**
Run: `cargo build`
Expected: compiles. (`#[serde(skip)]` requires `Default` for the field on deserialize;
`bool: Default` is satisfied. No other constructor needs changes — helpers funnel
through `new`/`test_default`.)

**Step 4: Commit** — `sim/game_entity: add in_logic_vector membership flag (+0x98)`

---

### Task 3: Replace the order field + rewrite the register/unregister/snapshot mechanism

**Why:** Make `LogicVector` the order owner, wire the membership-gated ops, and
**remove the sorted fallback (DRIFT 1)**.

**Files:**
- Modify: `src/sim/world/mod.rs` (module decl; field :289; init :454; methods :612-639)

**Pattern:** Disjoint-field borrows (read the entity flag in a scoped match, then
touch `self.logic`).

**Step 1: Declare the module** (with the other `mod` decls at the top of
`src/sim/world/mod.rs`)
```rust
mod logic_vector;
pub(crate) use logic_vector::LogicVector;
```

**Step 2: Replace the field** (mod.rs:286-289)
```rust
    /// LogicClass active-object vector — the single authority on object order.
    /// Tail-append on reveal, compacting-remove on conceal. Serialized verbatim.
    #[serde(default)]
    pub(crate) logic: LogicVector,
```

**Step 3: Update the constructor init** (mod.rs:454, replace `live_object_order: Vec::new(),`)
```rust
            logic: LogicVector::new(),
```

**Step 4: Rewrite the three methods** (replace mod.rs:612-639)
```rust
    /// Native Reveal's append: +0x98 guard → tail-append → set flag. Idempotent.
    pub(crate) fn register_live_object(&mut self, stable_id: u64) {
        match self.entities.get_mut(stable_id) {
            Some(e) if !e.in_logic_vector => e.in_logic_vector = true,
            _ => return, // absent, or already a member (idempotent)
        }
        self.logic.push(stable_id);
    }

    /// Native Conceal's remove: gate on flag → clear flag → compacting remove.
    pub(crate) fn unregister_live_object(&mut self, stable_id: u64) {
        if let Some(e) = self.entities.get_mut(stable_id) {
            if !e.in_logic_vector {
                return; // not a member — nothing to remove
            }
            e.in_logic_vector = false;
        }
        // Entity present-and-member, or already gone from store: scrub the order.
        self.logic.remove(stable_id);
    }

    /// The active order, verbatim. No sorted-ID fallback (was DRIFT).
    pub(crate) fn live_object_order_snapshot(&self) -> Vec<u64> {
        self.logic.snapshot()
    }
```

**Step 5: Add the test-only order setter** (near the methods above)
```rust
    /// Test-only: force the active order and sync membership flags to it.
    #[cfg(test)]
    pub(crate) fn set_logic_order_for_test(&mut self, order: Vec<u64>) {
        for &id in &order {
            if let Some(e) = self.entities.get_mut(id) {
                e.in_logic_vector = true;
            }
        }
        self.logic.set_order_for_test(order);
    }
```

**Step 6: Verify**
Run: `cargo build` — expect errors only at the two direct field-access sites fixed in
Tasks 4 and 8/12 (despawn :697 still uses `unregister_live_object` — unchanged; the
passenger test :1351 is fixed in this task's Step 7).

**Step 7: Fix the passenger test** (`src/sim/passenger.rs:1351`, replace
`sim.live_object_order = vec![pax, bldg];`)
```rust
        sim.set_logic_order_for_test(vec![pax, bldg]);
```

**Step 8: Verify**
Run: `cargo test -p <crate> production_garrison_owner_order_uses_live_object_order_not_stable_id`
Expected: PASS (order is now driven by `LogicVector`, no sorted fallback).

**Step 9: Commit** — `sim/world: LogicVector-backed register/unregister, drop sorted fallback (DRIFT 1)`

---

### Task 4: Route `despawn_entity` to unregister before store removal

**Why:** Mirror native conceal-then-free ordering and guarantee `unregister` sees the
entity (so the membership flag is read and cleared before the entity is gone).

**Files:**
- Modify: `src/sim/world/mod.rs` (despawn body around :695-697)

**Step 1: Reorder the removal** (replace the tail of `despawn_entity`, mod.rs:695-697)
```rust
        self.clear_radio_contacts_for(stable_id);
        self.unregister_live_object(stable_id); // conceal: leave the active order first
        self.entities.remove(stable_id);        // then free the slot
```

**Step 2: Verify**
Run: `cargo test -p <crate> --lib sim::world`
Expected: PASS (no behavior change beyond ordering; flag is now cleared on the live
entity).

**Step 3: Commit** — `sim/world: despawn unregisters before store removal (conceal-then-free)`

---

### Task 5: Stop registering limbo-created objects (DRIFT 2a)

**Why:** Native registers at reveal, not construction. Limbo objects must be in the
store but absent from the active order.

**Files:**
- Modify: `src/sim/world/world_spawn.rs` (`spawn_object_limbo_at_height`, the
  `register_live_object` call at :588)

**Step 1: Remove the registration** (delete line world_spawn.rs:588
`self.register_live_object(stable_id);`). Leave the `entities.insert` and
`increment_owned_count` calls. Add a one-line comment in its place:
```rust
        // Limbo objects are NOT registered in the active order — registration
        // happens at reveal/unlimbo (e.g. paradrop drop), mirroring ObjectClass+0x98.
```

**Step 2: Verify**
Run: `cargo build`
Expected: compiles. (Paradrop is the only caller; its reveal wiring is Task 6.)

**Step 3: Commit** — `sim/world: limbo spawn does not register active membership (DRIFT 2a)`

---

### Task 6: Register the paratrooper on successful drop (DRIFT 2b)

**Why:** With Task 5, the paradropped infantry leaves the plane in limbo and must be
revealed (registered) the moment it is placed on the playfield with a parachute.

**Files:**
- Modify: `src/sim/aircraft/drop_payload.rs` (after the successful
  `begin_parachute_descent` at :223)

**Step 1: Register after a successful parachute attach.** Locate the
`begin_parachute_descent(...)` call (drop_payload.rs:223) and its success path (the
rollback that returns the passenger to `Inside` is at :228). Immediately after the
success branch — where `passenger_role` has been set to `None` (:211) and the
parachute has attached — add:
```rust
    // Reveal/unlimbo: the dropped passenger is now an active object on the
    // playfield. Mirrors ObjectClass::Reveal → adder (+0x98).
    sim.register_live_object(passenger_id);
```
Place it so it runs only on the success path (after the `if !begin_parachute_descent(...)
{ ...rollback...; return ...; }` block, not inside the rollback).

**Step 2: Verify the rollback path leaves it unregistered.** Confirm the failure
branch (drop_payload.rs:228, sets `passenger_role = Inside`) is *before* the new
`register_live_object` call and returns early, so a failed drop never registers.

**Step 3: Verify**
Run: `cargo test -p <crate> paradrop`
Expected: PASS — existing paradrop tests still green; dropped passenger is now in the
active order.

**Step 4: Commit** — `sim/aircraft: register paratrooper on drop (reveal/unlimbo, DRIFT 2b)`

---

### Task 7: Rebuild membership from the restored order on load (DRIFT 3)

**Why:** `+0x98` is `#[serde(skip)]`, so after deserialize all flags are false while
the order is restored. Derive membership from vector presence (avoids the §3.4
stale/double-add hazard regardless of what gamemd does). Extract it as a **standalone
method** so the load-membership property is independently testable without the heavy
`rebuild_caches_after_load` argument set (see Task 10).

**Files:**
- Modify: `src/sim/world/mod.rs` (new method + call from `rebuild_caches_after_load`
  at :796)

**Step 1: Add the standalone rebuild method** (near `rebuild_caches_after_load`)
```rust
    /// Rebuild LogicClass membership flags from the restored active order.
    ///
    /// `+0x98` is not serialized (native does not round-trip it); vector presence
    /// is authoritative. Idempotent — safe to call after any load. Standalone (no
    /// heavy load-arg dependency) so save/load membership is unit-testable.
    pub(crate) fn rebuild_logic_membership(&mut self) {
        for &id in &self.logic.snapshot() {
            if let Some(entity) = self.entities.get_mut(id) {
                entity.in_logic_vector = true;
            }
        }
    }
```

**Step 2: Call it from `rebuild_caches_after_load`** (inside the function, after the
screen-coords loop at mod.rs:816-818, before the occupancy rebuild)
```rust
        // 2b. Rebuild LogicClass membership from the restored order.
        self.rebuild_logic_membership();
```

**Step 3: Verify**
Run: `cargo build`
Expected: compiles.

**Step 4: Commit** — `sim/world: derive logic membership from restored order on load (DRIFT 3)`

---

### Task 8: Add the active order to the state hash

**Why:** The order is authoritative (garrison reconciliation reads it → affects
outcomes) and is currently unhashed. Include it for desync detection. Membership flag
is derived → not separately hashed.

**Files:**
- Modify: `src/sim/world/world_hash.rs` (`state_hash` at :33)

**Step 1: Hash the order** (in `state_hash`, after
`self.next_stable_entity_id.hash(&mut hasher);` at world_hash.rs:40)
```rust
        // LogicClass active-object order — authoritative (drives reconciliation order).
        let order = self.logic.as_slice();
        order.len().hash(&mut hasher);
        for id in order {
            id.hash(&mut hasher);
        }
```

**Step 2: Verify**
Run: `cargo test -p <crate> --lib sim::world`
Expected: PASS (existing hash tests may assert specific values — if any pin a literal
hash, update the expected value, since adding state legitimately changes the hash).

**Step 3: Commit** — `sim/world: include active object order in state hash`

---

### Task 9: Bump the snapshot version

**Why:** Mark the semantic change to the spine explicitly; reject pre-spine dev saves.

**Files:**
- Modify: `src/sim/snapshot.rs` (`SNAPSHOT_VERSION` at :16)

**Step 1: Bump the constant**
```rust
const SNAPSHOT_VERSION: u32 = 11;
```

**Step 2: Verify**
Run: `cargo test -p <crate> snapshot`
Expected: PASS.

**Step 3: Commit** — `sim/snapshot: bump version for LogicVector spine`

---

### Task 10: Spine integration tests

**Why:** Prove the lifecycle + save/load contract end-to-end, covering the DRIFT
fixes and the ledger items.

**Files:**
- Modify: `src/sim/snapshot.rs` (add tests to the existing `#[cfg(test)] mod tests`)
  or `src/sim/world/world_tests.rs` — place where `Simulation` test helpers already
  exist.

**Step 1: Add the tests** (adapt helper names to the existing test fixtures in the
chosen file; the assertions are the contract)
```rust
    #[test]
    fn limbo_object_registers_only_on_reveal() {
        // Spawn in limbo: present in store, absent from order.
        // (Use the project's limbo-spawn test helper / spawn_object_limbo_at_height.)
        // assert!(sim.entities.contains(id));
        // assert!(!sim.live_object_order_snapshot().contains(&id));
        // Reveal it:
        // sim.register_live_object(id);
        // assert_eq!(sim.live_object_order_snapshot().last(), Some(&id)); // tail append
    }

    #[test]
    fn saveload_restores_live_object_order_verbatim() {
        // Force order [B, A, C] where creation IDs differ from order.
        // sim.set_logic_order_for_test(vec![b, a, c]);
        let bytes = GameSnapshot::save(&sim, 0, 0, "m", 0);
        let restored = GameSnapshot::load(&bytes).expect("load").sim;
        assert_eq!(restored.live_object_order_snapshot(), vec![b, a, c]);
    }

    #[test]
    fn saveload_restored_member_removes_cleanly() {
        // After load, membership is rebuilt from the order; unregister removes once.
        let mut restored = GameSnapshot::load(&bytes).expect("load").sim;
        restored.rebuild_logic_membership(); // the real load-path step (Task 7)
        // Sanity: flags are false straight after deserialize, true after rebuild.
        // assert!(restored.entities.get(a).unwrap().in_logic_vector);
        restored.unregister_live_object(a);
        assert!(!restored.live_object_order_snapshot().contains(&a)); // no stale entry
        restored.register_live_object(a);
        assert_eq!(
            restored.live_object_order_snapshot().iter().filter(|&&x| x == a).count(),
            1
        ); // no double-add
    }
```
Note: this test exercises the **real** load-membership path via the standalone
`rebuild_logic_membership()` (Task 7), not the test-only `set_logic_order_for_test`.
It proves the §3.4 hazard is avoided: a restored member unregisters exactly once
(no stale entry) and re-registers without duplicating (no double-add).

**Step 2: Verify**
Run: `cargo test -p <crate> saveload_ limbo_object_registers`
Expected: PASS.

**Step 3: Commit** — `sim: integration tests for LogicVector spine + save/load`

---

### Task 11: Full regression + determinism gate

**Why:** Confirm nothing in the phased tick regressed and the order is deterministic
across a save/load round-trip.

**Files:** none (verification only)

**Step 1: Run the full suite**
Run: `cargo test`
Expected: all PASS. Investigate any failure in files this plan touched; per CLAUDE.md,
ignore failures only in files another session is editing.

**Step 2: Determinism round-trip check**
Confirm (via an existing snapshot round-trip test, or a temporary assertion) that
`sim.state_hash()` is identical before save and after load+`rebuild_caches_after_load`
for a populated session. The order and membership must survive byte-identically.

**Step 3: Commit** — only if Step 1/2 required a fix; otherwise no commit.

---

### Task 12: Verify against gamemd.exe (no code)

**Why:** Confirm the spine reproduces the verified binary behavior. Documentation /
inspection only.

**Verify:**
- **Tail-append + idempotent + compacting-remove:** matches adder `0x0055BAA0` /
  remover `0x0055BAE0` (re-verified live this session). Unit tests in Task 1 encode it.
- **Membership not serialized, derived on load:** matches `ObjectClass::Save
  0x005F6250` (no +0x98) and the §3.4 hazard avoidance. Task 7 + Task 10 test it.
- **Save order verbatim:** matches `DynamicVectorClass::Save 0x00551B20` /
  `Load 0x00551B90`. Task 10 `saveload_restores_live_object_order_verbatim`.
- **Limbo not registered until reveal:** matches `ObjectClass::Reveal 0x005F4EC0`
  (registers at reveal, gated by `type+0x234`). Task 5/6 + the limbo test.
- **Map-load seed order** (ledger 12): if
  `map_load_live_object_order_follows_native_section_sequence` fails, the parser/spawn
  slice ordering is the cause — flag it as a follow-up (out of this plan's scope), do
  not silently sort.

**No commit** (verification task).

## Sources & References

- **Design doc:** [docs/plans/2026-05-28-logicclass-object-lifecycle-spine-design.md](2026-05-28-logicclass-object-lifecycle-spine-design.md)
- **Ghidra reports:** `LOGICCLASS_OBJECT_LIFECYCLE_SPINE_SYSTEM_MODEL_SYNTHESIS.md`,
  `SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION_GHIDRA_REPORT.md`,
  `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`,
  `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`,
  `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`.
- **gamemd.exe addresses (verified live this session):** adder `0x0055BAA0`
  (+0x98 set at `0x0055bac6`); remover `0x0055BAE0`; Reveal `0x005F4EC0`
  (`type+0x234`); Save `0x00551B20`; Load `0x00551B90`; `ObjectClass::Save 0x005F6250`
  (no +0x98); scheduler `0x0055B5FB..0x0055B619`; Full_Init `0x00686B20`.
- **Related code:** `src/sim/entity_store.rs` (newtype-serde pattern),
  `src/sim/world/mod.rs:796` (`rebuild_caches_after_load`),
  `src/sim/world/world_spawn.rs:588` (limbo spawn),
  `src/sim/aircraft/drop_payload.rs:203-223` (paradrop drop),
  `src/sim/passenger.rs:355,1351` (order consumer + test).
- **INI keys:** none.
