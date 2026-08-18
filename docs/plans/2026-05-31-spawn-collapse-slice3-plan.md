# Spawn 4-Step Collapse (Slice 3) — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Collapse the three duplicated spawn sequences (two active 4-steps + one limbo fork) into a
single `unlimbo(ge)` / `create_limbo(ge)` pair differing by one flag, preserving the exact current
step order so the replay hash stays bit-identical.

**Architecture:** Slice 3 of the `ObjectSubstrate` consolidation (design §7 item 9, §8 Slice 3).
Classified up front (critic #11) as a **pure no-op refactor** (user decision): the collapse keeps the
current `insert → reveal → increment → occupancy` order verbatim. gamemd's Mark-before-register
reorder is **out of scope** — it lands in Slice 7 with the reveal gate-chain + rollback. No hash
change, no `SNAPSHOT_VERSION` bump, no gamemd-side evidence artifact required.

**Design Doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md) — Slice 3 (line 229), §7 item 9, §4 (Unlimbo/spawn rows).

---

## Grounding Summary

- **What the design says (§7 item 9, Slice 3 @ line 229):** the per-spawn manual 4-step
  (`world_spawn.rs`) + the near-duplicate limbo-spawn fork collapse into "one `unlimbo()`/`create_limbo()`
  pair, difference = one flag." Critic #11: classify up front — pure no-op (hash-identical) **or**
  adopt gamemd Mark-before-register (hash-changing). **Decided: pure no-op.**
- **Current code (grounded this session):** three spawn sequences, all in `src/sim/world/world_spawn.rs`:
  - map-spawn loop (world_spawn.rs:241-244): `insert → reveal → increment_owned_count → add_entity_occupancy`
  - `spawn_object_at_height` (world_spawn.rs:397-400): identical 4-step
  - `spawn_object_limbo_at_height` (world_spawn.rs:530-533): `insert → increment_owned_count` (no reveal/occupancy)
  The active 4-step lands occupancy AFTER the active-vector append (design §4 Unlimbo row). The no-op
  keeps that order; nothing runs between the steps, so no caller observes the intermediate state.
- **Name-clash constraint (grounded this session):** a thin `unlimbo(id)` alias exists
  (`mod.rs:779`, body = `self.reveal(stable_id)`). Repurposing `unlimbo` to take a `GameEntity`
  collides with it (Rust has no overloading). Its only non-test caller is paradrop
  (drop_payload.rs:240), which adds occupancy **manually first** (drop_payload.rs:215) then calls
  `unlimbo` for the reveal — so the alias is reveal-only on purpose; extending it to add occupancy
  would double-add. Resolution (user-approved): replace the 2 thin `unlimbo(id)` callers with direct
  `reveal(id)` and remove the alias, freeing the name for `unlimbo(ge)`.
- **Repo pattern this mirrors:** the existing lifecycle primitives `reveal`/`conceal`/`uninit`
  (mod.rs:767/773/896) and `add_entity_occupancy`/`increment_owned_count` (mod.rs:783/863). The
  collapse just relocates the call sequence into a shared helper.
- **INI keys:** none. No INI-driven constants.
- **Still unknown after grounding:** nothing. The classification (the only design-flagged decision)
  is resolved; the reorder is explicitly Slice 7's.

## Key Technical Decisions

- **Pure no-op classification** — collapse preserves exact step order; hash bit-identical; no
  `SNAPSHOT_VERSION` bump. **Confidence:** high — **Source:** design Slice 3 / critic #11 + user
  decision (this session). Mark-before-register reorder deferred to Slice 7.
- **`place_spawned(ge, active)` private helper preserves verbatim order** — `insert; if active { reveal;
  increment; occupancy } else { increment }`. **Confidence:** high — **Source:** the three call sites
  are byte-for-byte this sequence (world_spawn.rs:241-244 / 397-400 / 530-533); relocating identical
  ops in identical order is a trivial no-op.
- **`unlimbo(ge)` / `create_limbo(ge)` pair, difference = one flag** — matches design §7 item 9.
  **Confidence:** high — **Source:** design §7 item 9.
- **Remove the `unlimbo(id)` alias; repoint callers to `reveal(id)`** — the alias body IS
  `reveal(id)`, so repointing is semantically identical; frees the name. **Confidence:** high —
  **Source:** mod.rs:779-781 (body), exhaustive grep of `unlimbo` callers this session (drop_payload.rs:240,
  snapshot.rs:643 test).

## Open Questions

### Resolved During Planning

- **Classification (critic #11):** pure no-op (preserve order). Reorder is Slice 7.
- **`unlimbo` name clash:** remove the `unlimbo(id)` alias, repoint its 2 callers to `reveal(id)`,
  free the name for `unlimbo(ge)`. Paradrop keeps its manual occupancy-then-reveal sequence
  unchanged (out of scope).
- **Does `create_limbo`+`reveal`-style decomposition reorder `increment` vs `reveal`?** Avoided by
  using a single `place_spawned(ge, active)` helper that preserves the verbatim
  `reveal → increment → occupancy` order, rather than composing `create_limbo` then reveal.

### Deferred to Implementation

- None. The replay-hash test is the oracle; if any reorder slipped in, it fails and the task stops.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/mod.rs` | Remove the `unlimbo(id)` alias; tidy its doc references |
| Modify | `src/sim/aircraft/drop_payload.rs` | Repoint `unlimbo(passenger_id)` → `reveal(passenger_id)` |
| Modify | `src/sim/world/world_spawn.rs` | Add `place_spawned` helper + `unlimbo(ge)` / `create_limbo(ge)`; migrate the 3 spawn sites |
| Modify | `src/sim/snapshot.rs` | Remove the obsolete `unlimbo_equals_reveal_appends_member` test; add Slice 3 regression tests |

## Interface Changes

- **Removed:** `Simulation::unlimbo(&mut self, stable_id: u64)` (mod.rs:779). Callers (paradrop +
  one test) repoint to `reveal(stable_id)` — semantically identical (the alias body was `reveal`).
- **Added:** `Simulation::unlimbo(&mut self, ge: GameEntity) -> u64` (spawn-and-place, active),
  `Simulation::create_limbo(&mut self, ge: GameEntity) -> u64` (spawn-into-limbo), and a private
  `Simulation::place_spawned(&mut self, ge: GameEntity, active: bool) -> u64` (world_spawn.rs). Both
  public methods are `pub(crate)`. No external (non-sim) dependents.
- **Unchanged signatures:** `spawn_from_map*`, `spawn_object`, `spawn_object_at_height`,
  `spawn_object_limbo_at_height` keep their signatures and return types — only their bodies' tail
  collapses. All their callers are unaffected.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (no arithmetic; just relocating calls).
- [x] New state included in deterministic state hash — **no new state.** No fields added; the hash
      is unchanged (this is the acceptance oracle).
- [x] No dependencies on render/ui/sidebar/audio/net — all edits are in `sim/`.
- [x] Tick ordering impact — none. `spawn_object` is called from production within the tick, but the
      collapse does not change *when* it runs, only how its body is structured.
- [x] BTreeMap iteration order — unchanged. Entities are inserted in the same order with the same
      stable_ids; `occupancy_enter_order` is assigned in the same sequence.

## Risk Areas

- **Accidental reorder** (e.g. `increment` before `reveal`, or `occupancy` before `reveal`) → would
  risk a hash change. Mitigation: `place_spawned` keeps the verbatim order; Task 5 asserts replay
  hash bit-identical. **Highest-stakes guard in the slice.**
- **Paradrop double-occupancy** if `unlimbo` were extended to add occupancy. Mitigation: paradrop is
  repointed to `reveal(id)` (Task 1), never to the new `unlimbo(ge)`; its manual `occupancy.add`
  (drop_payload.rs:215) stays the sole occupancy step. Task 4 includes a paradrop regression run.
- **Dead-code window** between adding a method and wiring its caller. Mitigation: each method is
  added together with its call-site migration in the same task (Task 2 wires `create_limbo`; Task 3
  wires `unlimbo`), so no commit carries an unused method.
- **Obsolete test** `unlimbo_equals_reveal_appends_member` (snapshot.rs:638) asserts the removed
  `unlimbo(id)==reveal` equivalence. Mitigation: removed in Task 1; reveal-by-id append is already
  covered by `reveal_then_conceal_roundtrips_membership` (snapshot.rs).

## Parity-Critical Items

Pure no-op refactor — the parity stake is **absence of change**. The spawn sequence produces
player-visible results (units appear, occupy cells, count toward the house); the collapse must
reproduce them bit-for-bit.

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 3 | Verbatim `reveal → increment → occupancy` order in `place_spawned` | Reordering occupancy vs reveal is gamemd's Mark-before-register change — Slice 7's job, hash-affecting; must NOT slip into Slice 3 | `place_spawned` body matches the 3 sites verbatim; Task 5 replay hash bit-identical |
| Task 5 | Replay hash + map-spawn count + snapshot unchanged | The whole slice's guarantee is determinism-preserving; spawn drives every match | Existing replay-hash / world_hash / snapshot tests green; explicit hash-equality assertion |
| Task 1 | Paradrop still reveals without double-occupancy | Paradropped infantry must land in exactly one cell-list entry, every paradrop | drop_payload test `failed placement must not unlimbo … into occupancy` (drop_payload.rs:504) + paradrop descent test stays green |

---

## Tasks

### Task 1: Free the `unlimbo` name — remove the `unlimbo(id)` alias, repoint callers

**Why:** `unlimbo(ge)` (Task 3) cannot coexist with the `unlimbo(id)` alias (no overloading). The
alias body is just `reveal(id)`, so repointing its callers is semantically identical. Doing this
first keeps every later task compiling.

**Files:**
- Modify: `src/sim/world/mod.rs` (remove alias at mod.rs:777-781)
- Modify: `src/sim/aircraft/drop_payload.rs` (drop_payload.rs:240)
- Modify: `src/sim/snapshot.rs` (remove obsolete test, ~snapshot.rs:636-648)

**Pattern:** N/A (removal + 1:1 caller repoint).

**Step 1: Repoint the paradrop caller** (drop_payload.rs:240)
```rust
    // Reveal: the dropped passenger leaves the transport's limbo and becomes an
    // active object. Occupancy was already added above (drop_payload.rs:215);
    // this only appends it to the active-object order.
    sim.reveal(passenger_id);
```
(Replaces `sim.unlimbo(passenger_id);` — same effect, since the alias was `reveal`.)

**Step 2: Remove the `unlimbo(id)` alias** (mod.rs:777-781) — delete the whole method:
```rust
    /// Native `TechnoClass::Unlimbo` -> Reveal: a limbo-created object joins the
    /// live set at unlimbo/landing time, not at construction.
    pub(crate) fn unlimbo(&mut self, stable_id: u64) {
        self.reveal(stable_id);
    }
```

**Step 3: Remove the obsolete test** (snapshot.rs ~636-648) — delete the whole test, including its
doc comment:
```rust
    /// `unlimbo` is `reveal`: a stored limbo object joins the active order.
    #[test]
    fn unlimbo_equals_reveal_appends_member() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        sim.substrate.entities
            .insert(GameEntity::test_default(7, "E1", "Americans", 3, 3));
        sim.unlimbo(7);
        assert!(sim.substrate.entities.get(7).unwrap().in_logic_vector);
        assert_eq!(sim.live_object_order_snapshot(), vec![7]);
    }
```
(Reveal-by-id append is already covered by `reveal_then_conceal_roundtrips_membership`.) Adjust the
adjacent section comment at snapshot.rs:617 only if it now reads wrong (e.g. drop the `/unlimbo`
token); leave it otherwise.

**Step 4: Verify**
Run: `cargo check -p vera20k`
Expected: compiles; no remaining `sim.unlimbo(` references resolve to the old id-alias (grep
`unlimbo` shows only doc-comment mentions and the new pair once Tasks 2-3 land).

**Step 5: Commit** (`refactor(sim): remove unlimbo(id) alias, repoint to reveal (Slice 3)`)

---

### Task 2: Add `place_spawned` helper + `create_limbo(ge)`; migrate the limbo spawn site

**Why:** Introduce the shared, order-preserving placement helper and the limbo wrapper, and wire the
one limbo call site so the method is used immediately (no dead-code window).

**Files:**
- Modify: `src/sim/world/world_spawn.rs` — add methods inside `impl Simulation { ... }` (e.g. just
  after `spawn_object_limbo_at_height`); migrate the limbo site at world_spawn.rs:528-534.

**Pattern:** Relocates the existing call sequence; uses the same `interner.resolve(...).to_string()`
+ `increment_owned_count` + `add_entity_occupancy` + `reveal` calls the sites use today.

**Step 1: Add the shared helper + `create_limbo` wrapper**
```rust
    /// Shared spawn placement. Inserts the entity, then either reveals + occupies
    /// it (active spawn) or leaves it in limbo. Preserves the exact pre-collapse
    /// step order (`insert → reveal → increment → occupancy`) so the replay hash
    /// is bit-identical (Slice 3 no-op classification). Returns the stable id.
    ///
    /// `active=true` reproduces the old 4-step; `active=false` reproduces the
    /// limbo fork. The gamemd Mark-before-register reorder is deferred to a later
    /// slice — do NOT swap occupancy ahead of reveal here.
    fn place_spawned(&mut self, ge: GameEntity, active: bool) -> u64 {
        let stable_id = ge.stable_id;
        let owner = self.interner.resolve(ge.owner).to_string();
        let category = ge.category;
        self.substrate.entities.insert(ge);
        if active {
            self.reveal(stable_id);
            self.increment_owned_count(&owner, category);
            self.add_entity_occupancy(stable_id);
        } else {
            self.increment_owned_count(&owner, category);
        }
        stable_id
    }

    /// Spawn an object directly into limbo: stored in EntityStore and owner counts
    /// but NOT registered in the active order or map occupancy. Registration
    /// happens later at reveal/landing (e.g. paradrop drop). Returns the stable id.
    pub(crate) fn create_limbo(&mut self, ge: GameEntity) -> u64 {
        self.place_spawned(ge, false)
    }
```

**Step 2: Migrate the limbo spawn site** (world_spawn.rs:528-534) — replace:
```rust
        let spawn_owner_str = self.interner.resolve(ge.owner).to_string();
        let spawn_category = ge.category;
        self.substrate.entities.insert(ge);
        // Limbo objects are NOT registered in the active order — registration
        // happens at reveal/unlimbo (e.g. paradrop drop), mirroring ObjectClass+0x98.
        self.increment_owned_count(&spawn_owner_str, spawn_category);
        Some(stable_id)
```
with:
```rust
        Some(self.create_limbo(ge))
```
(The `stable_id` local from `allocate_stable_id` is still used when building `ge` above; `create_limbo`
returns the same id.)

**Step 3: Verify**
Run: `cargo test -p vera20k --lib -- spawn 2>&1` (and any paradrop/limbo-spawn tests)
Expected: compiles; existing limbo-spawn / paradrop-cargo tests pass.

**Step 4: Commit** (`refactor(sim): add place_spawned + create_limbo, migrate limbo spawn (Slice 3)`)

---

### Task 3: Add `unlimbo(ge)`; migrate the two active spawn sites

**Why:** Add the active-spawn wrapper and wire both active 4-step sites, completing the collapse.

**Files:**
- Modify: `src/sim/world/world_spawn.rs` — add `unlimbo(ge)` next to `create_limbo`; migrate the map
  loop (world_spawn.rs:235-245) and `spawn_object_at_height` (world_spawn.rs:394-401).

**Pattern:** Same helper as Task 2 (`place_spawned(ge, true)`); migrates the verbatim 4-step.

**Step 1: Add the `unlimbo` wrapper** (next to `create_limbo`)
```rust
    /// Spawn an object and place it on the playfield in one step: insert, reveal
    /// (active-object order), increment owner counts, and register map occupancy —
    /// in that exact order. Returns the stable id. This is the active counterpart
    /// to [`Self::create_limbo`]; the two differ only by whether reveal+occupancy
    /// run.
    pub(crate) fn unlimbo(&mut self, ge: GameEntity) -> u64 {
        self.place_spawned(ge, true)
    }
```

**Step 2: Migrate the map-spawn loop** (world_spawn.rs:235-245) — replace:
```rust
            let owner_str = self.interner.resolve(ge.owner).to_string();
            let category = ge.category;
            let spawn_sid = ge.stable_id;
            if let Some(obj) = rules.and_then(|r| r.object(&map_ent.type_id)) {
                ge.foundation = obj.foundation.clone();
            }
            self.substrate.entities.insert(ge);
            self.reveal(spawn_sid);
            self.increment_owned_count(&owner_str, category);
            self.add_entity_occupancy(spawn_sid);
            count += 1;
```
with:
```rust
            if let Some(obj) = rules.and_then(|r| r.object(&map_ent.type_id)) {
                ge.foundation = obj.foundation.clone();
            }
            self.unlimbo(ge);
            count += 1;
```
(The `owner_str`/`category`/`spawn_sid` locals are extracted internally by `place_spawned` now; they
are used nowhere else in the loop body, so they are removed.)

**Step 3: Migrate `spawn_object_at_height`** (world_spawn.rs:394-401) — replace:
```rust
        let spawn_owner_str = self.interner.resolve(ge.owner).to_string();
        let spawn_category = ge.category;
        ge.foundation = obj.foundation.clone();
        self.substrate.entities.insert(ge);
        self.reveal(stable_id);
        self.increment_owned_count(&spawn_owner_str, spawn_category);
        self.add_entity_occupancy(stable_id);
        Some(stable_id)
```
with:
```rust
        ge.foundation = obj.foundation.clone();
        Some(self.unlimbo(ge))
```
(The `stable_id` local is still used when building `ge` above; `unlimbo` returns the same id. The
`spawn_owner_str`/`spawn_category` locals are removed — `place_spawned` extracts them.)

**Step 4: Verify**
Run: `cargo test -p vera20k --lib 2>&1 | tail -5`
Expected: full lib suite green (this exercises map spawn, production spawn, and the per-tick
membership + presence asserts across every replay test).

**Step 5: Commit** (`refactor(sim): collapse active spawn 4-step into unlimbo(ge) (Slice 3)`)

---

### Task 4: Slice 3 regression tests

**Why:** Encode the design's no-op acceptance: the collapse keeps both views consistent atomically
(no logic-before-occupancy window observable to a caller), and limbo spawn stays out of both views.

**Files:**
- Modify: `src/sim/snapshot.rs` — add tests in the existing `#[cfg(test)] mod tests` (near the other
  reveal/conceal lifecycle tests).

**Pattern:** Mirrors `reveal_then_conceal_roundtrips_membership` (snapshot.rs) — direct substrate
calls + assertions on `live_object_order_snapshot()` and occupancy.

**Step 1: Add the tests**
```rust
    /// Slice 3: `unlimbo(ge)` places the entity into BOTH the active order and
    /// occupancy in one atomic call — a caller can never observe it in `logic`
    /// without occupancy, because the method returns only after both. Owner count
    /// is incremented. (No-op collapse: same end state as the old 4-step.)
    #[test]
    fn unlimbo_ge_places_into_logic_and_occupancy_atomically() {
        use crate::sim::game_entity::{GameEntity, Presence};
        let mut sim = Simulation::new();
        let ge = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        let id = sim.unlimbo(ge);

        let e = sim.substrate.entities.get(id).expect("entity in store");
        assert!(e.in_logic_vector, "must be in the active order");
        assert_eq!(e.presence, Presence::InCell);
        assert_eq!(sim.live_object_order_snapshot(), vec![id]);
        assert!(
            sim.substrate.occupancy.contains_entity(5, 5, id),
            "must be registered in its foundation cell",
        );
        #[cfg(debug_assertions)]
        sim.debug_assert_presence_consistent();
    }

    /// Slice 3: `create_limbo(ge)` stores the entity and increments owner counts
    /// but leaves it OUT of the active order and OUT of occupancy (born InLimbo).
    #[test]
    fn create_limbo_leaves_entity_out_of_logic_and_occupancy() {
        use crate::sim::game_entity::{GameEntity, Presence};
        let mut sim = Simulation::new();
        let ge = GameEntity::test_default(2, "E1", "Americans", 6, 6);
        let id = sim.create_limbo(ge);

        let e = sim.substrate.entities.get(id).expect("entity in store");
        assert!(!e.in_logic_vector, "limbo object is not an active member");
        assert_eq!(e.presence, Presence::Limbo);
        assert!(sim.live_object_order_snapshot().is_empty());
        assert!(
            !sim.substrate.occupancy.contains_entity(6, 6, id),
            "limbo object must not occupy a cell",
        );
    }
```
(`OccupancyGrid::contains_entity(rx, ry, entity_id) -> bool` is verified at occupancy.rs:278.)

**Step 2: Verify**
Run: `cargo test -p vera20k --lib -- unlimbo_ge_places create_limbo_leaves --nocapture`
Expected: both pass.

**Step 3: Commit** (`test(sim): Slice 3 unlimbo/create_limbo placement regressions`)

---

### Task 5: Full-suite verification — hash bit-identical, count + snapshot unchanged

**Why:** The slice's headline guarantee (no-op): replay hash bit-identical, map-spawn count
unchanged, snapshot unchanged. Confirm across the full suite with the per-tick asserts active.

**Files:** none (verification only).

**Step 1: Full lib suite**
Run: `cargo test -p vera20k --lib 2>&1 | tail -5`
Expected: read the literal `test result:` line; all pass. The replay-hash, `world_hash`, `snapshot`,
and `world_tests` lifecycle tests must be unchanged. The per-tick `debug_assert_logic_membership_consistent`
+ `debug_assert_presence_consistent` (from Slices 1-2) must not fire.

**Step 2: Targeted determinism + paradrop checks**
Run: `cargo test -p vera20k --lib -- saveload_restores_live_object_order_verbatim saveload_occupancy_list_order_matches_incremental failed_placement paradrop --nocapture`
Expected: pass — confirms map-spawn order, occupancy enter-order, and paradrop (no double-occupancy)
are unchanged.

**Step 3: Confirm no snapshot/hash surface change** (read-only)
Confirm `SNAPSHOT_VERSION` is unchanged at snapshot.rs:22 (no bump — no field reorder). Confirm no
new field was added to any hashed struct (this slice adds no fields).

**Step 4: Clippy**
Run: `cargo clippy -p vera20k 2>&1 | grep -E "world_spawn|place_spawned|create_limbo|unlimbo"`
Expected: no warnings/errors referencing the new code (pre-existing unrelated lints in `render/vxl_*`
may remain — not introduced by this slice).

**Step 5: Commit** (only if Steps 1-4 surfaced incidental fixups; otherwise nothing to commit.)

---

### Task 6: Verification against the design contract (no gamemd binary work needed)

**Why:** Confirm Slice 3's no-op acceptance clauses. No new gamemd-matching behavior is introduced,
so per §8 no gamemd-side evidence artifact is required (unlike Slices 6/7).

**Verify:**
- **Hash identical:** Task 5 full suite + targeted determinism tests green; replay-hash tests
  unchanged → satisfied.
- **Re-entrant observer cannot see logic-before-occupancy:** `unlimbo(ge)` is atomic — it returns
  only after both reveal and occupancy; Task 4's `unlimbo_ge_places_into_logic_and_occupancy_atomically`
  asserts both views are consistent immediately after the call, and no public API exposes a mid-call
  state → satisfied.
- **Map-load count + snapshot unchanged:** the map-spawn loop still does `count += 1` per entity;
  `SNAPSHOT_VERSION` unchanged; Task 5 confirms map-spawn order/count tests green → satisfied.

**Expected result:** all clauses hold; the three spawn sequences now route through one
`place_spawned` helper with zero behavior or hash change.

## Sources & References

- **Design doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md)
  — Slice 3 (line 229), §7 item 9, §4 (Unlimbo/spawn rows: "spawn order insert→reveal→count→occupancy
  lands occupancy AFTER active-vector"), §8 (slice acceptance + SNAPSHOT_VERSION rule), critic #11 (line 261).
- **gamemd references (background, kept here, not in code):** `TechnoClass::Unlimbo 0x005F5940`,
  `ObjectClass::Reveal 0x005F4EC0`; active-vector flag `+0x98`. (Mark-before-register order is Slice 7.)
- **Related code:** `src/sim/world/world_spawn.rs` (map loop:241-244, `spawn_object`:253,
  `spawn_object_at_height`:267 / tail 394-401, `spawn_object_limbo_at_height`:407 / tail 528-534),
  `src/sim/world/mod.rs` (reveal:767, conceal:773, `unlimbo(id)` alias to remove:779,
  `add_entity_occupancy`:783, `increment_owned_count`:863), `src/sim/aircraft/drop_payload.rs`
  (manual occupancy:215, `unlimbo` caller:240, paradrop test:504), `src/sim/snapshot.rs`
  (obsolete test:638, `SNAPSHOT_VERSION`:22), `src/sim/occupancy.rs` (occupancy read accessor).
- **Prior slice commits:** `012d792` (Slice 2 Presence), `8197728` (Slice 1b), `d924b20` (Slice 1a),
  `c2b5153` (ObjectSubstrate intro).
- **User decision (this session):** pure no-op classification; remove `unlimbo(id)` alias and repoint
  callers to `reveal(id)`.
