# Presence FSM Field (Slice 2) — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Add a single authoritative-shadow `Presence` field to `GameEntity`, set it on every
lifecycle transition with assert-only source-state validation, and prove (via a per-tick debug
assert + save/load test) that it always equals the value derivable from the existing gates —
without changing the replay hash or any observable behavior.

**Architecture:** Slice 2 of the `ObjectSubstrate` consolidation (design §6, §8). `presence` is a
`#[serde(skip)]` shadow: the old gates (`in_logic_vector`, store membership) stay authoritative;
`presence` rides alongside them and is reconciled by an assert. This is the safety net that proves
transition coverage is complete *before* later slices (6/7) make `presence` authoritative. No
behavior change, no hash change, no `SNAPSHOT_VERSION` bump.

**Design Doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md) §6 + Slice 2 (line 227).

---

## Grounding Summary

- **What the design says (§6, Slice 2 @ line 227):** one authoritative `presence` field, serde-skip,
  rebuilt on load; set on transitions; `debug_assert!` legal source state; keep old gates (presence
  *shadows*). Accept: `presence == derived-from-old-gates` every tick for a full replay; hash
  identical (presence NOT hashed); save/load (limbo cargo + transport-loaded infantry) restores
  identical presence.
- **gamemd behavior (Part A §1.6, §2(f), C3):** a single `InLimbo` bit (+0x81) — born=1; `Reveal`
  clears it, `Conceal` sets it, `UnInit` tears down then enqueues. gamemd has **one** limbo state, not
  two; a born-limbo cargo and a concealed/boarded unit are the *same* InLimbo state.
- **Decision (confirmed with user):** the Slice 2 enum is the **3-state core** `{ Limbo, InCell, Dying }`.
  `Concealed` from the §6 4-state sketch is folded into `Limbo` (matches the single InLimbo bit) and
  added later only when a transition needs it (Slice 6 Dying-window / Slice 7 gate-chain).
- **Why `dying` is NOT in the derivation:** a dying-but-animating unit stays `in_logic_vector=true`
  (still ticks, still marked in its cell, still drawn) — gamemd keeps it ACTIVE until `UnInit`. So it
  derives to `InCell`. `Presence::Dying` is therefore a *transient* marker set inside `uninit` right
  before the slot is freed; in current synchronous-removal code it never survives to a tick boundary
  (it becomes a persistent, derivable state only in Slice 6's deferred-delete window).
- **Repo pattern this mirrors:** the existing `in_logic_vector` shadow (game_entity.rs:180-184,
  `#[serde(skip)]`, rebuilt by `rebuild_logic_membership` mod.rs:1055) and the existing per-tick
  invariant `debug_assert_logic_membership_consistent` (mod.rs:802, called at mod.rs:2201). Slice 2
  adds an exact sibling for `presence`.
- **Gates available per entity:** `in_logic_vector: bool` (game_entity.rs:184), store membership.
  `dying` is intentionally not consulted (see above).
- **INI keys:** none. This slice introduces no INI-driven constants.
- **Still unknown after grounding:** nothing blocking. The 4th state (`Concealed`) and the deferred
  `Dying` window are explicitly out of scope (Slices 6/7).

## Key Technical Decisions

- **3-state enum `Presence { Limbo, InCell, Dying }`** — matches gamemd's single InLimbo bit; the only
  partition the current gates can derive. **Confidence:** high — **Source:** design §1.6/§2(f)/C3 +
  user decision (this session).
- **Derivation `derived_presence = if in_logic_vector { InCell } else { Limbo }`** (ignores `dying`).
  **Confidence:** high — **Source:** game_entity.rs:328 (`dying` = "playing death animation, not yet
  despawned" → still active) + gamemd C3 (ACTIVE until UnInit).
- **`presence` is `#[serde(skip)]`, rebuilt on load, NOT hashed.** **Confidence:** high — **Source:**
  Slice 2 acceptance ("assert presence NOT hashed"); mirrors `in_logic_vector` (game_entity.rs:183).
  No `SNAPSHOT_VERSION` bump (currently 16, snapshot.rs:22) because skipped fields are not serialized.
- **`Dying` set transiently in `uninit` before `entities.remove`.** **Confidence:** high — **Source:**
  C7 / Slice 6 (forward-compat: Slice 6 will defer the remove and `Dying` becomes persistent).
- **All four `in_logic_vector` writers must also set `presence`** — `register_live_object`,
  `unregister_live_object`, `set_logic_order_for_test` (mod.rs:851), `rebuild_logic_membership`
  (mod.rs:1055). **Confidence:** high — **Source:** exhaustive grep this session (only these 4 sites
  mutate `in_logic_vector`).

## Open Questions

### Resolved During Planning

- **Limbo vs Concealed:** resolved to 3-state (fold `Concealed` into `Limbo`) — user decision +
  gamemd single-InLimbo-bit evidence.
- **Does `dying` enter the derivation?** No — dying-animating units are still `in_logic_vector`
  (ACTIVE/`InCell`). `Dying` is a transient uninit marker only (game_entity.rs:328; C3).
- **`SNAPSHOT_VERSION` bump?** No — `presence` is serde-skip (not in the byte stream).

### Deferred to Implementation

- None. All behavior in this slice is mechanical and the acceptance is assert-driven; there are no
  execution-time unknowns (no timing/framerate observation needed).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/game_entity.rs` | Define `Presence` enum; add `presence` field (`#[serde(skip)]`); add `derived_presence()` method; init in `new()` |
| Modify | `src/sim/world/mod.rs` | Set `presence` + source-state asserts in `register_live_object` / `unregister_live_object` / `uninit` / `set_logic_order_for_test`; extend `rebuild_logic_membership`; add `debug_assert_presence_consistent` and call it at end of `advance_tick` |
| Modify | `src/sim/snapshot.rs` | Add save/load test: limbo cargo + transport-loaded infantry + active unit restore identical `presence` |

## Interface Changes

- **New public enum `Presence`** (`src/sim/game_entity.rs`). New consumers: `GameEntity` field +
  substrate asserts. No external dependents yet (introduced this slice).
- **New public field `GameEntity::presence: Presence`.** `GameEntity::new(...)` initializes it;
  `#[serde(skip)]` so the snapshot format is unchanged. `GameEntity::test_default` builds via `new()`,
  so it inherits the default automatically — no test churn.
- **New method `GameEntity::derived_presence(&self) -> Presence`** — pure, read-only.
- **New substrate method `Simulation::debug_assert_presence_consistent(&self)`** — `#[cfg(debug_assertions)]`,
  read-only.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (enum, no arithmetic).
- [x] New state included in deterministic state hash — **NO, by design.** `presence` MUST NOT be added
      to `hash_entities` (world_hash.rs:384). It is serde-skip and derived; hashing it is forbidden by
      the Slice 2 acceptance criterion.
- [x] No dependencies on render/ui/sidebar/audio/net — `Presence` is a plain enum in `sim/`; methods
      are pure sim.
- [x] Tick ordering impact — adds one `#[cfg(debug_assertions)]` assert immediately before
      `state_hash()` (mod.rs:2200-2202); compiled out of release; no ordering change.
- [x] BTreeMap iteration order — the per-tick assert is per-entity and order-independent.

## Risk Areas

- **Missed `in_logic_vector` writer** → global assert fires in debug. Mitigation: Task 3 enumerates all
  four writers; the assert *is* the regression net. Low blast radius (debug-only).
- **Accidental hash change** → would break every lockstep/replay test. Mitigation: Task 6 explicitly
  verifies the full suite stays green and that `presence` is absent from `hash_entities`. This is the
  single highest-stakes guard in the slice.
- **`set_logic_order_for_test` left stale** (mod.rs:851 flips `in_logic_vector` directly) → the
  membership-mix test (snapshot.rs:629) that calls `debug_assert_*_consistent` would fail. Mitigation:
  Task 3 updates it in the same task as the other writers.

## Parity-Critical Items

This slice is pure internal scaffolding — `presence` shadows existing gates, is not hashed, and is not
read by any gameplay/render path. The parity stake is therefore **the absence of change**: the
determinism oracle (replay hash) must stay bit-identical and the snapshot format must be untouched.

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 5 | `presence` excluded from `state_hash` / `hash_entities` | If hashed, every desync-detector golden + replay test diverges; presence is non-authoritative this slice so it has no business in the hash (Slice 2 acceptance) | grep `hash_entities` (world_hash.rs:384) shows no `presence`; full replay suite hash bit-identical |
| Task 5 | `presence` is `#[serde(skip)]` → no `SNAPSHOT_VERSION` bump | Bumping/serializing would break existing save compatibility for a derived field | snapshot.rs:22 unchanged at 16; save/load test (Task 5) passes without version change |
| Task 6 | No observable behavior change | The whole slice must be invisible to the player and to lockstep | `cargo test -p vera20k` green across lifecycle/snapshot/world_hash; existing replay-hash tests unchanged |

---

## Tasks

### Task 1: Define the `Presence` enum

**Why:** The type must exist before any field or method references it. Interface-first.

**Files:**
- Modify: `src/sim/game_entity.rs` (add near the top-level entity enums, e.g. just above
  `pub struct GameEntity` at game_entity.rs:134)

**Pattern:** Mirrors the small entity-field enums already in this file (e.g. `PassengerRole` at
passenger.rs:131, and the `#[default]` enums at game_entity.rs:70/79). `game_entity.rs` has **no
`use serde`** — every derive is path-qualified `serde::Serialize, serde::Deserialize`. The `presence`
field is `#[serde(skip)]`, so the enum does not strictly need serde at all; deriving it path-qualified
matches convention and keeps it forward-compatible. `Default` (Limbo) **is** required (serde-skip
fills the field via `Default::default()` on load).

**Step 1: Add the enum**
```rust
/// Authoritative-shadow lifecycle state of an object (the substrate `Presence`
/// FSM, Slice 2). Mirrors gamemd's single `InLimbo` bit: an object is either in
/// the active set (`InCell`) or out of it (`Limbo`). `Dying` is a transient
/// marker set during teardown right before the store slot is freed; it becomes a
/// persistent, observable state only once deferred-delete lands (later slice).
///
/// In this slice `presence` *shadows* the old gates (`in_logic_vector` + store
/// membership) — those stay authoritative — and a debug assert proves the two
/// never disagree. Not serialized (`#[serde(skip)]` on the field); rebuilt on
/// load from the restored active order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Presence {
    /// Out of the active set: born-in-limbo, concealed, or loaded as cargo. The
    /// default for a freshly constructed entity (born InLimbo).
    #[default]
    Limbo,
    /// In the active-object set and placed on the playfield (`in_logic_vector`).
    InCell,
    /// Teardown in progress — set after conceal, before the slot is freed.
    Dying,
}
```

**Step 2: Verify**
Run: `cargo check -p vera20k`
Expected: compiles (enum is unused so far; `#[derive(Default)]` + `#[default]` requires no extra import).

**Step 3: Commit** (`feat(sim): add Presence lifecycle enum (Slice 2)`)

---

### Task 2: Add the `presence` field and `derived_presence()` to `GameEntity`

**Why:** The field is the shadow being validated; `derived_presence()` is the ground-truth function
the assert compares against. Both must exist before transitions can set/validate them.

**Files:**
- Modify: `src/sim/game_entity.rs` — add field next to `in_logic_vector` (game_entity.rs:180-184);
  add initializer in `new()` next to `in_logic_vector: false` (game_entity.rs:465); add method in
  `impl GameEntity` (game_entity.rs:414).

**Pattern:** Field placement and `#[serde(skip)]` exactly mirror `in_logic_vector` (game_entity.rs:183-184).

**Step 1: Add the field** (immediately after the `in_logic_vector` field at game_entity.rs:184)
```rust
    /// Substrate lifecycle shadow (Slice 2). Tracks `Limbo | InCell | Dying`.
    /// Authoritative gates remain `in_logic_vector` + store membership; this
    /// field rides alongside them and a per-tick debug assert proves they agree.
    /// Not serialized (rebuilt from the restored active order on load), and NOT
    /// hashed (non-authoritative this slice).
    #[serde(skip)]
    pub presence: Presence,
```

**Step 2: Initialize in `new()`** (immediately after `in_logic_vector: false,` at game_entity.rs:465)
```rust
            presence: Presence::Limbo,
```

**Step 3: Add the derivation method** (inside `impl GameEntity { ... }`, game_entity.rs:414)
```rust
    /// Ground-truth presence derived from the authoritative gates. A unit in the
    /// active set is `InCell` (this includes a dying-but-animating unit, which
    /// keeps ticking and stays in its cell until teardown); otherwise `Limbo`.
    /// `Dying` is never *derived* in this slice — it is only ever set imperatively
    /// during `uninit`, after which the slot is freed in the same call.
    pub fn derived_presence(&self) -> Presence {
        if self.in_logic_vector {
            Presence::InCell
        } else {
            Presence::Limbo
        }
    }
```

**Step 4: Add a unit test for the derivation** (in `game_entity.rs`'s existing `#[cfg(test)] mod tests`;
if none exists in this file, add the module at the end of the file)
```rust
#[cfg(test)]
mod presence_tests {
    use super::*;

    #[test]
    fn derived_presence_tracks_active_membership() {
        let mut e = GameEntity::test_default(1, "E1", "Americans", 3, 3);
        // Born in limbo: not yet in the active set.
        assert!(!e.in_logic_vector);
        assert_eq!(e.derived_presence(), Presence::Limbo);

        // Joins the active set.
        e.in_logic_vector = true;
        assert_eq!(e.derived_presence(), Presence::InCell);

        // A dying-but-animating unit stays active → still InCell (dying ignored).
        e.dying = true;
        assert_eq!(e.derived_presence(), Presence::InCell);

        // Leaves the active set.
        e.in_logic_vector = false;
        assert_eq!(e.derived_presence(), Presence::Limbo);
    }
}
```

**Step 5: Verify**
Run: `cargo test -p vera20k derived_presence_tracks_active_membership -- --nocapture`
Expected: `test result: ok. 1 passed`

**Step 6: Commit** (`feat(sim): add GameEntity.presence shadow + derivation (Slice 2)`)

---

### Task 3: Set `presence` + assert legal source state in all `in_logic_vector` writers

**Why:** This is the core of the slice — every place that flips the authoritative gate must also flip
the shadow, with a `debug_assert!` on the legal source state. There are exactly four such writers
(confirmed by exhaustive grep): `register_live_object`, `unregister_live_object`,
`set_logic_order_for_test`, and `rebuild_logic_membership` (the load-path rebuild, handled in Task 4).
This task covers the first three.

**Files:**
- Modify: `src/sim/world/mod.rs` — `register_live_object` (mod.rs:724), `unregister_live_object`
  (mod.rs:733), `set_logic_order_for_test` (mod.rs:851).

**Pattern:** Source-state asserts mirror the existing `debug_assert_*` discipline already in the
substrate (mod.rs:802-817).

**Step 1: Import `Presence`** — `world/mod.rs` does **not** currently import anything from
`crate::sim::game_entity` (it never names `GameEntity` as a type; method return types are inferred).
So add a fresh import near the other `use crate::sim::...` lines at the top of the file:
```rust
use crate::sim::game_entity::Presence;
```
(Verify first that no `use crate::sim::game_entity::...` line already exists; if one is added by a
parallel session, fold `Presence` into it instead of duplicating.)

**Step 2: Update `register_live_object`** (replace the body at mod.rs:724-730)
```rust
    /// Native Reveal's append: +0x98 guard → tail-append → set flag. Idempotent.
    pub(crate) fn register_live_object(&mut self, stable_id: u64) {
        match self.substrate.entities.get_mut(stable_id) {
            Some(e) if !e.in_logic_vector => {
                // Legal source: the only non-active presence in this slice is
                // Limbo, so an object joining the active set must be in Limbo.
                debug_assert_eq!(
                    e.presence,
                    Presence::Limbo,
                    "register_live_object: entity {stable_id} joined active set from {:?}, expected Limbo",
                    e.presence,
                );
                e.in_logic_vector = true;
                e.presence = Presence::InCell;
            }
            _ => return, // absent, or already a member (idempotent)
        }
        self.substrate.logic.push(stable_id);
    }
```

**Step 3: Update `unregister_live_object`** (replace the body at mod.rs:733-742)
```rust
    /// Native Conceal's remove: gate on flag → clear flag → compacting remove.
    pub(crate) fn unregister_live_object(&mut self, stable_id: u64) {
        if let Some(e) = self.substrate.entities.get_mut(stable_id) {
            if !e.in_logic_vector {
                return; // not a member — nothing to remove
            }
            // Legal source: an object leaving the active set was InCell.
            debug_assert_eq!(
                e.presence,
                Presence::InCell,
                "unregister_live_object: entity {stable_id} left active set from {:?}, expected InCell",
                e.presence,
            );
            e.in_logic_vector = false;
            e.presence = Presence::Limbo;
        }
        // Entity present-and-member, or already gone from store: scrub the order.
        self.substrate.logic.remove(stable_id);
    }
```

**Step 4: Update `set_logic_order_for_test`** (replace the body at mod.rs:851-860). It flips
`in_logic_vector` directly, so it must keep `presence` in sync or the per-tick assert (Task 5) fires.
```rust
    /// Test-only: force the active order and sync membership flags to it.
    #[cfg(test)]
    pub(crate) fn set_logic_order_for_test(&mut self, order: Vec<u64>) {
        for &id in &order {
            if let Some(e) = self.substrate.entities.get_mut(id) {
                e.in_logic_vector = true;
                e.presence = Presence::InCell;
            }
        }
        self.substrate.logic.set_order_for_test(order);
    }
```

**Step 5: Verify**
Run: `cargo test -p vera20k -- reveal_then_conceal_roundtrips_membership unlimbo_equals_reveal saveload_restored_member_removes_cleanly --nocapture`
Expected: all pass (these exercise register/unregister/order paths; no assert should fire).

**Step 6: Commit** (`feat(sim): maintain presence shadow in active-vector writers (Slice 2)`)

---

### Task 4: Set transient `Dying` in `uninit`; extend `rebuild_logic_membership` to restore `presence`

**Why:** `uninit` is the teardown transition (`Dying`); and the load path must reconstruct `presence`
from the restored gates so a round-trip restores identical presence (Slice 2 acceptance). These are
the remaining two presence write sites.

**Files:**
- Modify: `src/sim/world/mod.rs` — `uninit` (mod.rs:896-914), `rebuild_logic_membership` (mod.rs:1055-1064).

**Pattern:** `rebuild_logic_membership` already iterates every entity to reset and re-set
`in_logic_vector` from the restored order — append a presence reconciliation in the same pass.

**Step 1: Set `Dying` in `uninit`** (insert between `self.conceal(stable_id);` at mod.rs:912 and
`self.substrate.entities.remove(stable_id);` at mod.rs:913)
```rust
        self.conceal(stable_id); // leave the active order before freeing the slot
        // Transient teardown marker. Conceal moved presence to Limbo (or it was
        // already Limbo for a never-revealed limbo object). In this slice the slot
        // is freed immediately below, so Dying never survives to a tick boundary;
        // deferred-delete (later slice) makes it a one-tick observable state.
        if let Some(e) = self.substrate.entities.get_mut(stable_id) {
            debug_assert_ne!(
                e.presence,
                Presence::Dying,
                "uninit: entity {stable_id} already Dying (double teardown?)",
            );
            e.presence = Presence::Dying;
        }
        self.substrate.entities.remove(stable_id); // then free the slot
```

**Step 2: Reconcile `presence` in `rebuild_logic_membership`** (replace the body at mod.rs:1055-1064)
```rust
    pub(crate) fn rebuild_logic_membership(&mut self) {
        for entity in self.substrate.entities.values_mut() {
            entity.in_logic_vector = false;
        }
        for &id in &self.substrate.logic.snapshot() {
            if let Some(entity) = self.substrate.entities.get_mut(id) {
                entity.in_logic_vector = true;
            }
        }
        // Presence is #[serde(skip)] → all-default (Limbo) straight after
        // deserialize. Reconcile it from the just-restored authoritative gates so
        // a save/load round-trip restores identical presence (Slice 2 acceptance).
        for entity in self.substrate.entities.values_mut() {
            entity.presence = entity.derived_presence();
        }
    }
```

**Step 3: Verify**
Run: `cargo test -p vera20k -- uninit_conceals_then_frees_store_slot despawn_entity_delegates_to_uninit saveload_restores_live_object_order_verbatim --nocapture`
Expected: all pass.

**Step 4: Commit** (`feat(sim): set transient Dying in uninit + rebuild presence on load (Slice 2)`)

---

### Task 5: Add the per-tick `debug_assert_presence_consistent` invariant

**Why:** This is the acceptance oracle — `presence == derived-from-old-gates` every tick. It fires in
debug builds if any current or future code flips a gate without updating the shadow.

**Files:**
- Modify: `src/sim/world/mod.rs` — add the method next to `debug_assert_logic_membership_consistent`
  (mod.rs:802-817); call it next to the existing call (mod.rs:2200-2201).

**Pattern:** Exact sibling of `debug_assert_logic_membership_consistent` (mod.rs:802) — same
`#[cfg(debug_assertions)]` gate, same call site.

**Step 1: Add the assert method** (immediately after `debug_assert_logic_membership_consistent`, mod.rs:817)
```rust
    /// Debug-only invariant: the `presence` shadow must equal the value derivable
    /// from the authoritative gates for every in-store entity. Proves transition
    /// coverage is complete (every gate flip set the shadow). O(n); compiled out of
    /// release builds. `Dying` is transient inside `uninit` (slot freed same call),
    /// so no in-store entity is ever `Dying` at a tick boundary in this slice.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_presence_consistent(&self) {
        for e in self.substrate.entities.values() {
            debug_assert_eq!(
                e.presence,
                e.derived_presence(),
                "entity {} presence {:?} != derived {:?} (in_logic_vector={})",
                e.stable_id,
                e.presence,
                e.derived_presence(),
                e.in_logic_vector,
            );
        }
    }
```
(Use the entity's actual stable-id accessor — `e.stable_id` per game_entity.rs; adjust if the field
name differs.)

**Step 2: Call it at end of `advance_tick`** (immediately after the existing membership assert at mod.rs:2200-2201)
```rust
        #[cfg(debug_assertions)]
        self.debug_assert_logic_membership_consistent();
        #[cfg(debug_assertions)]
        self.debug_assert_presence_consistent();
        let state_hash = self.state_hash();
```

**Step 3: Confirm `presence` is NOT hashed** (read-only check, no edit). Grep `hash_entities`
(world_hash.rs:384) and confirm there is no `presence` line. There must be none — adding one is
forbidden this slice.
Run: `cargo test -p vera20k --lib` (whole sim suite) — every debug build now runs both asserts each
tick across all replay/lifecycle tests.
Expected: green; no presence assert fires.

**Step 4: Commit** (`feat(sim): per-tick presence-consistency debug invariant (Slice 2)`)

---

### Task 6: Save/load presence round-trip test (limbo cargo + transport-loaded infantry + active)

**Why:** Directly encodes the Slice 2 acceptance: "save/load (limbo cargo + transport-loaded infantry)
restores identical presence," plus hash-identical.

**Files:**
- Modify: `src/sim/snapshot.rs` — add a test in the existing `#[cfg(test)] mod tests` (alongside
  `saveload_restored_member_removes_cleanly`, snapshot.rs:434).

**Pattern:** Mirrors `saveload_restored_member_removes_cleanly` (snapshot.rs:434) — `GameSnapshot::save`
→ `GameSnapshot::load` → `rebuild_logic_membership` → assert. `GameSnapshot::load(&bytes).sim` returns
the deserialized sim WITHOUT rebuild (gates default), so the test calls `rebuild_logic_membership`
explicitly, exactly as the real load path does via `rebuild_caches_after_load` (mod.rs:1043).

**Step 1: Add the test**
```rust
    /// Slice 2 acceptance: save/load restores identical `presence` for an active
    /// unit (InCell), a never-revealed limbo object (Limbo), and a boarded/cargo
    /// unit (Limbo) — and the state hash is unchanged by `presence` (it is
    /// serde-skip and not hashed).
    #[test]
    fn saveload_restores_presence_for_active_limbo_and_cargo() {
        use crate::sim::game_entity::{GameEntity, Presence};
        use crate::sim::passenger::PassengerRole;

        let mut sim = Simulation::new();
        // (1) Active unit on the playfield → InCell.
        sim.substrate
            .entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        sim.reveal(1);
        // (2) Never-revealed limbo object → Limbo (default, never joined active set).
        sim.substrate
            .entities
            .insert(GameEntity::test_default(2, "E1", "Americans", 0, 0));
        // (3) Transport-loaded infantry: revealed, then concealed while boarding → Limbo.
        let mut pax = GameEntity::test_default(3, "E1", "Americans", 6, 6);
        pax.passenger_role = PassengerRole::Inside { transport_id: 1 };
        sim.substrate.entities.insert(pax);
        sim.reveal(3);
        sim.conceal(3); // boards: leaves the active order → Limbo

        // Pre-save expectations.
        assert_eq!(sim.substrate.entities.get(1).unwrap().presence, Presence::InCell);
        assert_eq!(sim.substrate.entities.get(2).unwrap().presence, Presence::Limbo);
        assert_eq!(sim.substrate.entities.get(3).unwrap().presence, Presence::Limbo);
        let hash_before = sim.state_hash();

        // Round-trip + the real load-path membership rebuild.
        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let mut restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        restored.rebuild_logic_membership();

        // Presence restored identically.
        assert_eq!(restored.substrate.entities.get(1).unwrap().presence, Presence::InCell);
        assert_eq!(restored.substrate.entities.get(2).unwrap().presence, Presence::Limbo);
        assert_eq!(restored.substrate.entities.get(3).unwrap().presence, Presence::Limbo);

        // Hash is unaffected by presence (serde-skip + not hashed).
        assert_eq!(restored.state_hash(), hash_before);

        // The reconciled shadow agrees with the derivation everywhere.
        #[cfg(debug_assertions)]
        restored.debug_assert_presence_consistent();
    }
```
(Confirm the `PassengerRole::Inside { transport_id }` variant path matches the actual definition
referenced at passenger.rs:480; adjust the import/variant fields to the real signature.)

**Step 2: Verify**
Run: `cargo test -p vera20k saveload_restores_presence_for_active_limbo_and_cargo -- --nocapture`
Expected: `test result: ok. 1 passed`.

**Step 3: Commit** (`test(sim): save/load presence round-trip for active/limbo/cargo (Slice 2)`)

---

### Task 7: Full-suite verification — hash bit-identical, no behavior change

**Why:** The slice's headline guarantee is "hash identical, no observable change." Confirm the whole
sim suite (including every replay-hash and lifecycle test) stays green with both debug asserts active.

**Files:** none (verification only).

**Step 1: Run the full package test suite**
Run: `cargo test -p vera20k`
Expected: read the literal `test result:` line(s); all pass. In particular the existing replay-hash,
`world_hash`, `snapshot`, and lifecycle (`world_tests`, `production_sell` membership-assert at
production_sell.rs:1091) tests must be unchanged — no presence assert may fire.

**Step 2: Clippy**
Run: `cargo clippy -p vera20k`
Expected: no new warnings introduced by the `Presence` enum / field / methods.

**Step 3: Confirm snapshot format untouched**
Read `src/sim/snapshot.rs:22` and confirm `SNAPSHOT_VERSION` is still `16` (no bump — `presence` is
serde-skip).

**Step 4: Commit** (only if Steps 1-3 produced any incidental fixups; otherwise nothing to commit).

---

### Task 8: Verification against the design contract (no gamemd binary work needed)

**Why:** Confirm the implementation satisfies Slice 2's three acceptance clauses. This slice introduces
**no new gamemd-matching behavior** (it is a pure determinism-preserving refactor), so per §8 it needs
**no gamemd-side evidence artifact** — the self-replay hash + derivation assert are the correct oracle
here (unlike Slices 6/7).

**Verify:**
- **`presence == derived-from-old-gates` every tick:** `debug_assert_presence_consistent` runs at the
  end of every `advance_tick` (Task 5) across the full debug test suite (Task 7) → satisfied.
- **Hash identical / presence not hashed:** Task 6 asserts `state_hash` unchanged across the round-trip;
  Task 5 Step 3 confirmed `presence` absent from `hash_entities`; Task 7 Step 3 confirmed no
  `SNAPSHOT_VERSION` bump → satisfied.
- **Save/load restores identical presence (limbo cargo + transport-loaded infantry):** Task 6 test →
  satisfied.

**Expected result:** all three clauses demonstrably hold; no behavior or hash change vs. pre-slice.

## Sources & References

- **Design doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md)
  — §6 (Rust-native replacement boundary / Presence FSM), Slice 2 (line 227), C3/C7 (lifecycle FSM),
  §1.6 + §2(f) (gamemd InLimbo lifecycle).
- **gamemd references (kept here, not in code):** InLimbo bit `ObjectClass+0x81`; `Reveal 0x005F4EC0`,
  `Conceal 0x005F4D30`, `Unlimbo 0x005F5940`, `UnInit 0x005F65F0`; active-vector flag `+0x98`.
- **Related code:** `src/sim/game_entity.rs` (GameEntity:134, in_logic_vector:184, dying:330, new():416,
  test_default), `src/sim/world/mod.rs` (register_live_object:724, unregister_live_object:733,
  reveal:747, conceal:753, unlimbo:759, uninit:896, set_logic_order_for_test:851,
  debug_assert_logic_membership_consistent:802, rebuild_logic_membership:1055,
  advance_tick assert site:2200), `src/sim/world/substrate.rs` (ObjectSubstrate), `src/sim/world/world_hash.rs`
  (state_hash:33, hash_entities:384), `src/sim/snapshot.rs` (SNAPSHOT_VERSION:22, save/load tests:414-460),
  `src/sim/passenger.rs` (boarding conceal:487, unloading reveal:881, PassengerRole::Inside:480).
- **Prior slice commits:** `8197728` (Slice 1b), `d924b20` (Slice 1a), `c2b5153` (ObjectSubstrate intro).
- **User decision (this session):** 3-state enum `{ Limbo, InCell, Dying }`; fold `Concealed` into `Limbo`.
