# `EnterOrderCounter` — Enter-Order Counter Ownership (Slice 5) — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the hand-threaded raw `&mut u64` occupancy enter-order counter with a typed
`EnterOrderCounter` newtype whose only mutator is `next()` — a hash-identical refactor, no
`SNAPSHOT_VERSION` bump.

**Architecture:** Slice 5 of the `ObjectSubstrate` consolidation (parent design §7 item 3, §8
Slice 5). The counter stays a serialized + hashed field on `ObjectSubstrate`; only its *type*
changes (`u64` → `EnterOrderCounter`) and the bare `&mut u64` threaded through movement becomes
`&mut EnterOrderCounter`. The increment formula (`saturating_add`) gets exactly one home.

**Design Doc:** [docs/plans/2026-06-01-slice5-enter-order-counter-design.md](2026-06-01-slice5-enter-order-counter-design.md)

---

## Grounding Summary

- **Parent design (§7.3, §8 Slice 5, §6, critic #12):** substrate's occupancy ops own the
  enter-order counter; replace `&mut u64` threading; counter + per-entity order ARE hashed.
  Hash-identical, no version bump, no gamemd artifact (pure determinism refactor — no new behavior).
- **No RE needed.** This slice introduces zero new gamemd-matching behavior, so no Ghidra
  verification or `docs/research/` report applies. The oracle is the self-replay state hash.
- **Current code (grounded this session, git-reverified — no commits landed on the movement files
  since the design):**
  - Counter field: `ObjectSubstrate.next_occupancy_enter_order: u64` (substrate.rs:31), init `1`
    (substrate.rs:52). Serialized (derive on the struct, not skipped) + hashed (world_hash.rs:49).
  - Per-entity field: `GameEntity.occupancy_enter_order: u64` (game_entity.rs:218), default
    `stable_id` (game_entity.rs:509), hashed (world_hash.rs:387). **Stays `u64` — unchanged.**
  - Three live assign-sites, each the identical read-increment-write triple:
    `add_entity_occupancy` (mod.rs:793-795), `movement_tick.rs:1316-1318`, `movement_step.rs:1198-1200`.
  - Threading: `advance_tick` (mod.rs:1663-1670, calls `tick_movement_with_grids`) → param
    `next_occupancy_enter_order: &mut u64` (movement_tick.rs:826) → passed to
    `process_cell_crossings` (movement_tick.rs:1463-1464) → param (movement_step.rs:909-910).
  - Consumer: `OccupancyGrid::rebuild` sorts on `(occupancy_enter_order, stable_id)`
    (occupancy.rs:121) — uses only the per-entity field, not the global counter.
  - Call site mod.rs:1670 passes `&mut self.substrate.next_occupancy_enter_order` — becomes
    `&mut EnterOrderCounter` automatically when the field type flips (**no source edit there**).
  - Legacy/test-only: `tick_movement_with_grid` (singular) + local `= 1` counters at
    movement/mod.rs:281, movement_tests.rs:1730/1806, prone_speed_tests.rs:84;
    movement_tests.rs:761/785 pass `&mut substrate.next_occupancy_enter_order` (auto-retypes).
- **INI keys:** none.
- **Still unknown after grounding:** nothing. Hash-identical is the acceptance; the full lib
  suite + `saveload_*` + occupancy-rebuild tests are the oracles.

## Key Technical Decisions

- **`EnterOrderCounter` newtype, `#[serde(transparent)]`, derived `Hash`; counter stays in
  `ObjectSubstrate`.** **Confidence:** high — **Source:** design §Chosen Approach + grounded
  current code (substrate.rs:31/52, world_hash.rs:49).
- **Hash-identical via two guarantees:** (1) `#[derive(Hash)]` on a single-field tuple struct
  hashes only the inner field with no prefix → bit-identical to `u64::hash`, so world_hash.rs:49
  stays literally unchanged; (2) `#[serde(transparent)]` over `u64` emits identical wire bytes →
  save/load compatible, no `SNAPSHOT_VERSION` bump. **Confidence:** high — **Source:** Rust std
  derive semantics + serde transparent docs; empirically confirmed by the replay-hash + `saveload_*`
  suites (Task 2/3).
- **Per-entity `occupancy_enter_order` stays `u64`; only the counter param flips type.**
  **Confidence:** high — **Source:** the field is the entity's stored value, not the counter; the
  movement `occupancy_enter_order: &mut u64` param is the entity field and is unchanged.
- **The field-type migration is one atomic commit (Task 2).** **Confidence:** high — **Source:**
  changing a struct field type breaks compilation at every consumer simultaneously; Rust requires
  all sites update together. Task 1 (define the type) is separable and compiles alone.

## Open Questions

### Resolved During Planning
- **Does moving to a newtype change the hash or save format?** No — derived `Hash` on a 1-field
  tuple struct == `u64::hash`; `#[serde(transparent)]` == identical bytes. Both verified by the
  determinism + saveload suites in Task 2/3.
- **Does the counter live in the grid?** No — `OccupancyGrid` is `#[serde(skip)]`; the counter
  must stay serialized in `ObjectSubstrate`.
- **Borrow conflict in `add_entity_occupancy`?** No — `self.substrate.next_occupancy_enter_order.next()`
  takes `&mut` of the same disjoint field the current `= …saturating_add(1)` assignment already
  mutates while `entity` (from `self.substrate.entities`) is live; identical borrow shape.

### Deferred to Implementation
- None. The full lib suite (hash oracle) + `saveload_*` are the gates; any cadence/value drift
  surfaces as a hash mismatch.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/substrate.rs` | Define `EnterOrderCounter`; field type `u64`→newtype; init; unit test |
| Modify | `src/sim/world/mod.rs` | Re-export `EnterOrderCounter`; route `add_entity_occupancy` triple through `.next()` |
| Modify | `src/sim/movement/movement_tick.rs` | Counter param type; assign-site `.next()` |
| Modify | `src/sim/movement/movement_step.rs` | `process_cell_crossings` counter param type; assign-site `.next()` |
| Modify | `src/sim/movement/mod.rs` | Test-wrapper local counter init → `EnterOrderCounter::new()` |
| Modify | `src/sim/movement/movement_tests.rs` | Test local counter inits → `EnterOrderCounter::new()` |
| Modify | `src/sim/movement/prone_speed_tests.rs` | Test local counter init → `EnterOrderCounter::new()` |

## Interface Changes

- **Added:** `EnterOrderCounter` (`pub(crate)`, defined in `substrate.rs`, re-exported from `world`
  as `crate::sim::world::EnterOrderCounter`) with `new() -> Self` (const) and `next(&mut self) -> u64`.
- **Changed (type only):** `ObjectSubstrate.next_occupancy_enter_order: u64` → `EnterOrderCounter`
  (same field name/position). The counter parameter on `tick_movement_with_grids`
  (movement_tick.rs:826) and `process_cell_crossings` (movement_step.rs:910): `&mut u64` →
  `&mut EnterOrderCounter`. **Depends on:** the tick call site (mod.rs:1670, auto-retypes), the
  movement-tick pass-through (movement_tick.rs:1464, auto-matches), the test wrappers (updated).
- **Unchanged:** `GameEntity.occupancy_enter_order: u64`; the movement `occupancy_enter_order:
  &mut u64` param; `OccupancyGrid::add`/`move_entity` signatures; `world_hash.rs:49` call site;
  serialized layout; `SNAPSHOT_VERSION`.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (monotonic `u64` counter bookkeeping; no sim-quantity arithmetic).
- [x] New state included in deterministic state hash — **no new hashed state.** Counter stays
      hashed at world_hash.rs:49 (now via derived `Hash` == `u64::hash`); per-entity order unchanged.
- [x] No dependencies on render/ui/sidebar/audio/net — all edits in `sim/`.
- [x] Tick ordering impact — **none.** The assign-site stays exactly where it is (inside the
      per-crossing loop); only the increment expression changes (`*c = c.saturating_add(1)` →
      `c.next()`), same value + cadence.
- [x] BTreeMap iteration order — unaffected; `OccupancyGrid::rebuild` sort key unchanged.

## Risk Areas

- **Hash drift if the newtype hashes ≠ bare `u64`.** Mitigation: derived `Hash` on a 1-field
  tuple struct is defined to hash only the inner field (no discriminant/length prefix); world_hash
  call site untouched; full lib suite + replay-hash tests in Task 2 confirm bit-identical.
- **Save/load incompatibility if serialized bytes differ.** Mitigation: `#[serde(transparent)]`;
  `saveload_*` suite (incl. `saveload_occupancy_list_order_matches_incremental`) green in Task 2/3.
- **Accidental cadence change** (moving the assign out of the per-crossing loop, or dropping the
  saturating semantics). Mitigation: each assign-site is a mechanical 3-line→2-line swap in place;
  `next()` reproduces `saturating_add(1)` exactly; the per-crossing loop body is otherwise untouched.
- **Mid-migration non-compilation.** Mitigation: Task 1 only *adds* the type (compiles alone);
  Task 2 flips the field + every consumer in **one atomic commit** so the tree always builds.

## Parity-Critical Items

Determinism-preserving refactor — the parity stake is **absence of change** to hashed state.

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `next()` = pre-increment value then `saturating_add(1)`; `new()` = 1 | Must reproduce the exact value sequence the three assign-sites produced | `enter_order_counter_*` unit tests |
| Task 2 | Counter increments once per cell-crossing (loop body) + once per occupancy-add; saturating | Counter is hashed; a different cadence/value moves the replay hash | full lib suite hash bit-identical; `saveload_*` green |
| Task 2 | Newtype hashes == `u64`; serializes == `u64` | Any divergence changes the replay hash or breaks load | replay-hash + `saveload_*` tests; `SNAPSHOT_VERSION` unchanged |

---

## Tasks

### Task 1: Define `EnterOrderCounter` + unit test (no field change yet)

**Why:** Establish the typed counter first; it compiles standalone (the field still `u64`), giving
a clean checkpoint that `next()`/`new()` produce the exact value sequence before the migration.

**Files:**
- Modify: `src/sim/world/substrate.rs` (add the type after the `use` block ~line 18; add a
  `#[cfg(test)] mod tests` at end of file ~line 65).

**Pattern:** Project newtype-wrapper convention for typed sim quantities; mirrors the Slice 1-4
"one owner for a cross-cutting invariant" funnel.

**Step 1: Add the newtype** — insert immediately after the existing imports (after `use
crate::sim::occupancy::OccupancyGrid;`, currently line 18), before the `ObjectSubstrate` doc/struct:
```rust
/// Monotonic source for rebuilt CellClass-style object-list (enter) order. Each
/// entity stores the last value assigned when it entered a cell list; this counter
/// hands out the next one. The sole mutator is `next()` — callers cannot mis-increment
/// or skip the saturating semantics. Serialized + hashed at its `ObjectSubstrate` field
/// (a `#[serde(transparent)]` + derived-`Hash` newtype is byte- and hash-identical to the
/// bare `u64` it replaces).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct EnterOrderCounter(u64);

impl EnterOrderCounter {
    /// Fresh counter. Starts at 1; 0 is the reserved sentinel.
    pub(crate) const fn new() -> Self {
        Self(1)
    }

    /// Return the current order value and advance. Saturating — never wraps,
    /// matching the pre-consolidation `saturating_add(1)` at every assign-site.
    pub(crate) fn next(&mut self) -> u64 {
        let order = self.0;
        self.0 = self.0.saturating_add(1);
        order
    }
}
```

**Step 2: Add the unit test** — append at end of `substrate.rs` (after the `impl Default` block,
line 65):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_order_counter_new_starts_at_one() {
        let mut c = EnterOrderCounter::new();
        // First handout is 1 (0 is the reserved sentinel).
        assert_eq!(c.next(), 1);
    }

    #[test]
    fn enter_order_counter_next_returns_pre_increment_then_advances() {
        let mut c = EnterOrderCounter::new();
        assert_eq!(c.next(), 1);
        assert_eq!(c.next(), 2);
        assert_eq!(c.next(), 3);
    }

    #[test]
    fn enter_order_counter_saturates_at_max() {
        let mut c = EnterOrderCounter(u64::MAX);
        // Returns MAX, then stays MAX (saturating, never wraps to 0).
        assert_eq!(c.next(), u64::MAX);
        assert_eq!(c.next(), u64::MAX);
    }
}
```
(The bare-tuple construction `EnterOrderCounter(u64::MAX)` is legal inside the defining module.)

**Step 3: Verify**
Run: `cargo test -p vera20k --lib -- enter_order_counter`
Expected: read the literal `test result:` line — 3 passed. (The type is exercised only by these
tests until Task 2 wires it in; a non-test `cargo check` would flag transient dead-code, which
clears in Task 2 — do not "fix" it here.)

**Step 4: Commit** (`refactor(sim): add EnterOrderCounter newtype (Slice 5)`)

---

### Task 2: Migrate the field + every consumer atomically; verify hash-identical

**Why:** Flip `next_occupancy_enter_order` to the newtype and route all three assign-sites + the
movement threading + test wrappers through it. Must be one commit — changing the field type breaks
compilation at every consumer until all are updated.

**Files (all modified together):**
- `src/sim/world/substrate.rs` (field type 31; init 52)
- `src/sim/world/mod.rs` (`add_entity_occupancy` 793-795)
- `src/sim/movement/movement_tick.rs` (param 826; assign 1316-1318)
- `src/sim/movement/movement_step.rs` (param 910; assign 1198-1200)
- `src/sim/movement/mod.rs` (test-wrapper local init 281)
- `src/sim/movement/movement_tests.rs` (local inits 1730, 1806)
- `src/sim/movement/prone_speed_tests.rs` (local init 84)

**Pattern:** Mechanical type migration — each assign-site is the same 3-line→2-line swap; each
local-counter init swaps the literal `1` for `EnterOrderCounter::new()`.

**Step 1: Change the field type + init** (`substrate.rs`):
- Line 31, replace `pub(crate) next_occupancy_enter_order: u64,` with:
```rust
    pub(crate) next_occupancy_enter_order: EnterOrderCounter,
```
- Line 52, replace `next_occupancy_enter_order: 1,` with:
```rust
            next_occupancy_enter_order: EnterOrderCounter::new(),
```
(Update the field's existing doc comment lines 28-30 to drop "stores the last order value" detail
now that it lives on `EnterOrderCounter`; not load-bearing — the rustdoc on the type covers it.)

**Step 1b: Re-export the type for crate-wide use** (`mod.rs`) — `mod substrate;` (mod.rs:16) is
private, so movement must reach the newtype through a `world` re-export, exactly as `ObjectSubstrate`
is. Add immediately after line 23 (`pub(crate) use substrate::ObjectSubstrate;`):
```rust
pub(crate) use substrate::EnterOrderCounter;
```
The crate-wide path is then `crate::sim::world::EnterOrderCounter`.

**Step 2: Route `add_entity_occupancy`** (`mod.rs`) — replace lines 793-795:
```rust
        let order = self.substrate.next_occupancy_enter_order;
        self.substrate.next_occupancy_enter_order = self.substrate.next_occupancy_enter_order.saturating_add(1);
        entity.occupancy_enter_order = order;
```
with:
```rust
        let order = self.substrate.next_occupancy_enter_order.next();
        entity.occupancy_enter_order = order;
```

**Step 3: Change the movement-tick param + assign** (`movement_tick.rs`):
- Line 826, in `tick_movement_with_grids`, replace `next_occupancy_enter_order: &mut u64,` with:
```rust
    next_occupancy_enter_order: &mut EnterOrderCounter,
```
- Lines 1316-1318, replace:
```rust
                        let order = *next_occupancy_enter_order;
                        *next_occupancy_enter_order = order.saturating_add(1);
                        entity.occupancy_enter_order = order;
```
with:
```rust
                        let order = next_occupancy_enter_order.next();
                        entity.occupancy_enter_order = order;
```
- Add the import at the top of `movement_tick.rs` (with the other `use crate::sim::…` imports):
```rust
use crate::sim::world::EnterOrderCounter;
```
(The pass-through at movement_tick.rs:1463-1464 needs no edit — `next_occupancy_enter_order` is now
the newtype and matches the updated `process_cell_crossings` param.)

**Step 4: Change the `process_cell_crossings` param + assign** (`movement_step.rs`):
- Line 910, replace `next_occupancy_enter_order: &mut u64,` with:
```rust
    next_occupancy_enter_order: &mut EnterOrderCounter,
```
(Leave line 909 `occupancy_enter_order: &mut u64,` UNCHANGED — that's the entity field.)
- Lines 1198-1200, replace:
```rust
        let order = *next_occupancy_enter_order;
        *next_occupancy_enter_order = order.saturating_add(1);
        *occupancy_enter_order = order;
```
with:
```rust
        let order = next_occupancy_enter_order.next();
        *occupancy_enter_order = order;
```
- Add the import at the top of `movement_step.rs` (with the other `use crate::sim::…` imports):
```rust
use crate::sim::world::EnterOrderCounter;
```

**Step 5: Update the test-wrapper local counters** (all use the crate-wide re-export path
`crate::sim::world::EnterOrderCounter`):
- `movement/mod.rs:281`, replace `let mut next_occupancy_enter_order = 1;` with:
```rust
    let mut next_occupancy_enter_order = crate::sim::world::EnterOrderCounter::new();
```
- `movement_tests.rs:1730` and `movement_tests.rs:1806`, replace each
  `let mut next_occupancy_enter_order = 1;` with:
```rust
    let mut next_occupancy_enter_order = crate::sim::world::EnterOrderCounter::new();
```
- `prone_speed_tests.rs:84`, replace `let mut next_occupancy_enter_order = 1;` with:
```rust
    let mut next_occupancy_enter_order = crate::sim::world::EnterOrderCounter::new();
```
(`movement_tests.rs:761/785` pass `&mut …substrate.next_occupancy_enter_order` — no edit; the field
is now the newtype and matches the updated param. The `&mut self.substrate.next_occupancy_enter_order`
call site at `mod.rs:1670` likewise needs no edit.)

**Step 6: Confirm no other consumer** — grep `next_occupancy_enter_order` and confirm every hit is
either updated above, the unchanged hash site (world_hash.rs:49), the field def/init (substrate.rs),
or one of the auto-retyping call/pass-through sites that need no edit because the value they pass is
now the newtype: `mod.rs:1670`, `movement_tick.rs:1464`, `movement_tests.rs:761/785` (pass the
substrate field), and `movement/mod.rs:289`, `prone_speed_tests.rs:94`, `movement_tests.rs:1743/1819`
(pass the Step-5-updated local by `&mut`).
Run: grep `next_occupancy_enter_order` across `src`.

**Step 7: Verify — compile + full determinism suite (the hash oracle; run BEFORE commit)**
Run: `cargo test -p vera20k --lib 2>&1 | tail -8`
Expected: read the literal `test result:` line — all pass, same total as Slice 4's green run plus
the 3 new `enter_order_counter_*` tests; replay-hash / `world_hash` / `saveload_*` / occupancy /
movement tests unchanged. (The import path is `crate::sim::world::EnterOrderCounter` via the Step 1b
re-export.)

**Step 8: Commit** (`refactor(sim): thread occupancy enter-order via EnterOrderCounter (Slice 5)`)

---

### Task 3: Acceptance verification against the design contract

**Why:** Confirm Slice 5's acceptance clauses. No new gamemd behavior, so no gamemd-side artifact
is required (parent design §8).

**Step 1: Save/load determinism** — Run: `cargo test -p vera20k --lib -- saveload`
Expected: read the literal `test result:` line — all `saveload_*` green, including
`saveload_occupancy_list_order_matches_incremental` (rebuild-after-load reproduces live order via
the per-entity field) and `saveload_rebuild_is_deterministic`.

**Step 2: Clippy on touched code** — Run: `cargo clippy -p vera20k 2>&1` and confirm no new warning
references `EnterOrderCounter`, `next_occupancy_enter_order`, `substrate`, `movement_tick`, or
`movement_step` (pre-existing unrelated lints may remain).

**Step 3: Confirm the contract clauses hold:**
- **Hash identical:** Task 2 full lib suite green, replay-hash/world_hash tests unchanged,
  `SNAPSHOT_VERSION` not bumped → satisfied. (world_hash.rs:49 was left untouched — derived `Hash`
  on the newtype == `u64::hash`.)
- **`OccupancyGrid::rebuild` reproduces live order:** the occupancy-rebuild test +
  `saveload_occupancy_list_order_matches_incremental` green (Step 1) → satisfied. The rebuild reads
  only the per-entity `occupancy_enter_order` (unchanged `u64`).
- **No bare `&mut u64` counter threading remains:** Step 6 grep + the param types in
  movement_tick.rs:826 / movement_step.rs:910 are now `&mut EnterOrderCounter`; the formula lives
  only in `EnterOrderCounter::next()` → satisfied.

**Expected result:** all clauses hold; the counter is a typed owner with one increment site, the
replay hash is bit-identical, and the save format is unchanged.

## Sources & References

- **Design doc:** [docs/plans/2026-06-01-slice5-enter-order-counter-design.md](2026-06-01-slice5-enter-order-counter-design.md)
- **Parent design:** docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md — §7 item 3 (line
  211), §8 Slice 5 (line 233), §6 occupancy/enter-order (line 200), critic #12 (enter-order IS
  hashed, line 262).
- **Current code:** `src/sim/world/substrate.rs` (counter field 31, init 52),
  `src/sim/world/mod.rs` (`add_entity_occupancy` 793-795, tick call 1663-1670),
  `src/sim/movement/movement_tick.rs` (`tick_movement_with_grids` param 826, assign 1316-1318,
  pass-through 1463-1464), `src/sim/movement/movement_step.rs` (`process_cell_crossings` params
  909-910, assign 1198-1200), `src/sim/movement/mod.rs` (test wrapper 281), `src/sim/occupancy.rs`
  (rebuild sort 121), `src/sim/game_entity.rs` (field 218, default 509), `src/sim/world/world_hash.rs`
  (counter 49, per-entity 387).
- **Prior slice commits (dev):** Slice 4 `4ab1bf6`/`0d37ada`/`288ab4b`/`8cc7022`/`74e1ca0`/`69f0b2b`;
  Slice 3 `df59c36`/`bfb6cfe`/`a58e8fd`/`fc9c461`.
- **INI keys:** none.
