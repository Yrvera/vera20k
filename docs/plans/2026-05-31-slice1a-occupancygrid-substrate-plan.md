# Slice 1a — Migrate `OccupancyGrid` into `ObjectSubstrate` Implementation Plan

> **For Claude:** Execute task-by-task. This is a pure mechanical refactor — NO behavior
> change, hash MUST stay bit-identical. Tasks 1–4 land together (a field move does not
> compile half-done); compile only after Task 4. Do NOT touch `EntityStore` (that is the
> separate phase 1b).

**Goal:** Move the `occupancy: OccupancyGrid` field off `Simulation` into `ObjectSubstrate`
as `substrate.occupancy`, so the substrate owns occupancy alongside the active-vector and
counters it already holds — the small half of Slice 1's deferred storage migration.

**Architecture:** Entirely inside `sim/`. `ObjectSubstrate` (`src/sim/world/substrate.rs`)
gains the `occupancy` field; `Simulation` (`src/sim/world/mod.rs`) drops it and exposes
`occupancy()`/`occupancy_mut()` accessors. Internal sim/ code uses the field path
`self.substrate.occupancy` (preserves disjoint-field borrows); the 3 external call sites
route through the accessors. No `render`/`ui`/`audio`/`net` dependency added.

**Design Doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](../research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md)
— §6 architecture (lines 176–182), Slice 1 (line 225), borrow discipline (line 193).

---

## Grounding Summary

- **Design (lines 176–182, 225):** Slice 1's target has `ObjectSubstrate` own
  `store`/`logic`/`occupancy`/counters. `logic` + both counters already landed (commit
  `c2b5153`); `occupancy` and `store` were deferred ([substrate.rs:8](../../src/sim/world/substrate.rs)).
  This plan does `occupancy` only. **No research/Ghidra/INI dependency — pure refactor.**
- **Borrow discipline (line 193):** "storage stays independently borrowable." Internal sim/
  co-mutates occupancy with entities/logic (verified hotspot: `tick_movement_with_grids`
  at [mod.rs:1586](../../src/sim/world/mod.rs) takes `&mut self.entities`, `&mut self.occupancy`,
  `&mut self.substrate.next_occupancy_enter_order` together). After the move these become
  `&mut self.entities` + `&mut self.substrate.occupancy` + `&mut self.substrate.next_occupancy_enter_order`
  — all **disjoint places** (`entities` is a Simulation field; `occupancy`/counter are
  disjoint fields of `substrate`), so Rust's disjoint-field borrow checker accepts it.
  Therefore internal sites use the **field path**, never an accessor method (a method would
  whole-borrow `substrate` and break co-mutation).
- **Hash-identity:** `occupancy` is already `#[serde(skip)]` ([mod.rs:367](../../src/sim/world/mod.rs))
  and rebuilt on load via `OccupancyGrid::rebuild(&self.entities)` ([mod.rs:1026](../../src/sim/world/mod.rs)).
  Moving it into the (also-serialized) `ObjectSubstrate` with `#[serde(skip)]` keeps it out of
  the serialized bytes exactly as before → snapshot layout unchanged. `OccupancyGrid: Default`
  exists ([occupancy.rs:108](../../src/sim/occupancy.rs)) so the skip reconstructs cleanly.
- **Completeness is compiler-guaranteed:** removing the field from `Simulation` turns every
  missed `self.occupancy` into a hard compile error ("no field `occupancy` on `Simulation`"),
  so the internal sweep cannot silently miss a site.
- **Call-site inventory (measured):** `.occupancy` = 67 refs in `sim/`, 4 outside. Of the 4
  external, only **3 touch `Simulation`** (2 read, 1 mutate); the 4th
  ([pixel_fx_sparkles.rs:266](../../src/render/pixel_fx_sparkles.rs)) reads a render-input
  struct's own `occupancy` field, not the sim — unaffected. Many of the 67 sim/ refs are
  local `&mut OccupancyGrid` parameters in `movement`/`occupancy` (e.g. inside
  `tick_movement_with_grids`) — those do NOT change; only accesses through a `Simulation`
  receiver (`self.occupancy`) change.
- **Unknown after grounding:** none. Premise stable (mod.rs/substrate.rs last touched by the
  same Slice-1 commit `c2b5153`; no parallel restructuring).

## Key Technical Decisions

- **Internal access via field path `self.substrate.occupancy`, external via accessor methods.**
  **Confidence:** high — **Source:** design doc line 193 + verified disjoint-borrow at
  [mod.rs:1586](../../src/sim/world/mod.rs). Field paths preserve the co-mutation borrows the
  movement/lifecycle code relies on; methods would not.
- **`occupancy` field in `ObjectSubstrate` keeps `#[serde(skip)]`.** **Confidence:** high —
  **Source:** current [mod.rs:367](../../src/sim/world/mod.rs) already skips it + rebuild on
  load + `OccupancyGrid: Default` ([occupancy.rs:108](../../src/sim/occupancy.rs)). Keeps the
  serialized snapshot byte-identical → hash unchanged.
- **`ObjectSubstrate::new()` initializes occupancy via `OccupancyGrid::new()`** (matching the
  removed [mod.rs:498](../../src/sim/world/mod.rs) initializer verbatim). **Confidence:** high
  — **Source:** repo pattern, preserves exact construction behavior.

## Open Questions

### Resolved During Planning
- *Does the move break the movement co-borrow?* → No. `&mut self.substrate.occupancy` and
  `&mut self.substrate.next_occupancy_enter_order` are disjoint fields → compiles (verified
  at [mod.rs:1586](../../src/sim/world/mod.rs)).
- *Does it change the snapshot/hash?* → No. occupancy was and stays `#[serde(skip)]`; the
  serialized fields (counters, logic) are untouched.
- *Which external sites change?* → 3 (`build_instances.rs:355`, `app_skirmish.rs:757` read;
  `app_sim_tick.rs:304` mutate). `pixel_fx_sparkles.rs:266` is downstream and unaffected.

### Deferred to Implementation
- Exact count of `self.occupancy` rewrite sites — the compiler enumerates them after Task 2
  removes the field (Task 3 fixes each). No guesswork needed.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/substrate.rs` | Add `occupancy: OccupancyGrid` field (`#[serde(skip)]`) + init in `new()` |
| Modify | `src/sim/world/mod.rs` | Remove `occupancy` field + its initializer; rebuild→`self.substrate.occupancy`; add `occupancy()`/`occupancy_mut()` accessors; rewrite `self.occupancy`→`self.substrate.occupancy` |
| Modify | `src/app_render/build_instances.rs:355` | `&sim.occupancy` → `sim.occupancy()` |
| Modify | `src/app_skirmish.rs:757` | `sim.occupancy.get(...)` → `sim.occupancy().get(...)` |
| Modify | `src/app_sim_tick.rs:304` | `sim.occupancy.remove(...)` → `sim.occupancy_mut().remove(...)` |

## Interface Changes

- **New public methods on `Simulation`:** `occupancy(&self) -> &OccupancyGrid` and
  `occupancy_mut(&mut self) -> &mut OccupancyGrid`. Nothing depends on them yet; they
  replace direct `pub occupancy` field access for the 3 external sites.
- **Removed:** `pub occupancy: OccupancyGrid` field on `Simulation`. All external access
  (3 sites) migrates to the accessors; internal access (sim/) uses `self.substrate.occupancy`.

## Sim Checklist

- [x] No new f32/f64 — pure field move, no math.
- [x] State-hash impact: **none expected.** occupancy stays `#[serde(skip)]`; serialized
      fields unchanged; world_hash reads the same data via the new path. Verified by the
      5000-tick bit-identical replay (Task 5).
- [x] No deps on render/ui/sidebar/audio/net — change is in `sim/` + the accessor calls in
      app/render are render→sim reads (allowed direction).
- [x] Tick ordering: unaffected.
- [x] BTreeMap iteration order: unaffected (occupancy is a separate grid; not reordered).

## Risk Areas

- **Borrow conflict (low):** the only way the move fails to compile is a site that
  whole-borrows `self.substrate` (e.g. `self.substrate.some_method(&mut …)`) while also
  borrowing `self.substrate.occupancy`. `ObjectSubstrate` currently has no `&mut self`
  methods (fields are accessed directly), so this should not occur. If it does, the fix is
  to use field paths, not to revert. The compiler surfaces it immediately.
- **Missed site (eliminated):** removing the field makes every missed `self.occupancy` a
  compile error — the sweep cannot be incomplete.
- **Stale comment:** [mod.rs:1591-1592](../../src/sim/world/mod.rs) comment says
  "co-borrows &mut self.entities/occupancy" — update to `self.substrate.occupancy` for
  accuracy while editing that call.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 5 | Deterministic state hash unchanged | A field-owner move must not alter lockstep state; any hash drift = a serialization or read-path mistake, not an intended change | 5000-tick replay hash bit-identical vs pre-change baseline + full `cargo test -p vera20k` green |

This is a refactor with **no** gamemd-mechanism change — the only parity stake is that the
hash and all snapshot/occupancy/lifecycle tests are byte-for-byte unchanged.

---

## Tasks

### Task 1: Add the `occupancy` field to `ObjectSubstrate`

**Why:** Give the substrate ownership of the occupancy grid before removing it from
`Simulation`. Adding it first compiles cleanly (the two fields coexist briefly).

**Files:**
- Modify: `src/sim/world/substrate.rs`

**Pattern:** Mirrors the existing `logic: LogicVector` field on the same struct (serde-skipped
cache rebuilt on load).

**Step 1 — Add the import** near the top of `substrate.rs` (it currently has
`use super::LogicVector;`):
```rust
use crate::sim::occupancy::OccupancyGrid;
```

**Step 2 — Add the field** to `pub(crate) struct ObjectSubstrate`, after `logic`:
```rust
    /// CellClass-style occupancy grid (per-cell object lists). A rebuilt cache:
    /// `#[serde(skip)]`, reconstructed from the entity store on load, so it never
    /// appears in the serialized snapshot and does not enter the state hash directly.
    #[serde(skip)]
    pub(crate) occupancy: OccupancyGrid,
```

**Step 3 — Initialize it in `ObjectSubstrate::new()`**, matching the previous
`Simulation::new` initializer (`OccupancyGrid::new()`):
```rust
            logic: LogicVector::new(),
            occupancy: OccupancyGrid::new(),
```

**Step 4 — Verify compile.** Run: `cargo check -p vera20k`. Expected: clean (occupancy now
exists on both `Simulation` and `ObjectSubstrate`; harmless until Task 2).

### Task 2: Remove the field from `Simulation` + add accessors + fix construction/rebuild

**Why:** Make the substrate the sole owner. This breaks every `self.occupancy` site — that is
intended; Task 3 sweeps them. Adding the accessors here gives the external sites their target.

**Files:**
- Modify: `src/sim/world/mod.rs`

**Step 1 — Delete the field declaration** at [mod.rs:366-368](../../src/sim/world/mod.rs):
```rust
    /// Rebuilt from entities on deserialization.
    #[serde(skip)]
    pub occupancy: OccupancyGrid,
```
(Remove all three lines.)

**Step 2 — Delete its initializer** in `Simulation::new` at [mod.rs:498](../../src/sim/world/mod.rs):
```rust
            occupancy: OccupancyGrid::new(),
```
(The `substrate: ObjectSubstrate::new()` initializer already builds the occupancy grid via
Task 1.)

**Step 3 — Add the accessors** in an `impl Simulation` block (place next to other small
field accessors; if unsure, immediately after the struct's `new` method):
```rust
    /// The occupancy grid (per-cell object lists). Read access for systems above sim/.
    pub fn occupancy(&self) -> &OccupancyGrid {
        &self.substrate.occupancy
    }

    /// Mutable occupancy access for the few above-sim callers that unmark cells.
    pub fn occupancy_mut(&mut self) -> &mut OccupancyGrid {
        &mut self.substrate.occupancy
    }
```

**Step 4 — Fix the load-rebuild site** at [mod.rs:1026](../../src/sim/world/mod.rs):
```rust
        self.substrate.occupancy = OccupancyGrid::rebuild(&self.entities);
```
(was `self.occupancy = OccupancyGrid::rebuild(&self.entities);`)

**Step 5 — Do NOT compile yet** — Step 1 broke the internal sites; Task 3 fixes them. (The
`OccupancyGrid` import at [mod.rs:58](../../src/sim/world/mod.rs) stays — still used by the
accessor return types and `OccupancyGrid::rebuild`.)

### Task 3: Sweep internal `self.occupancy` → `self.substrate.occupancy`

**Why:** Re-point every in-`Simulation` occupancy access at the new owner. The compiler lists
exactly which sites need it.

**Files:**
- Modify: `src/sim/world/mod.rs` (and any other `sim/` file the compiler flags that accesses a
  `Simulation`'s `.occupancy` through a receiver)

**Step 1 — Get the list.** Run: `cargo check -p vera20k 2>&1 | rg "no field .occupancy"`.
Every reported location is a `self.occupancy` (or `<sim_binding>.occupancy`) that must become
`self.substrate.occupancy` (or `<sim_binding>.substrate.occupancy`).

**Step 2 — Rewrite each flagged site** from `self.occupancy` to `self.substrate.occupancy`.
This includes the debug-rebuild compare site around [mod.rs:1538](../../src/sim/world/mod.rs)
(`let expected = OccupancyGrid::rebuild(&self.entities);` stays, but any `self.occupancy`
comparison target becomes `self.substrate.occupancy`) and the movement co-borrow call at
[mod.rs:1586-1591](../../src/sim/world/mod.rs) (`&mut self.occupancy` → `&mut self.substrate.occupancy`).

> **Do NOT** touch local `&mut OccupancyGrid` parameters (e.g. the `occupancy` parameter
> inside `tick_movement_with_grids` and other `movement`/`occupancy` functions) — those are
> not `Simulation` field accesses and the compiler will not flag them.

**Step 3 — Update the stale comment** at [mod.rs:1591-1592](../../src/sim/world/mod.rs):
change "co-borrows &mut self.entities/occupancy" to "co-borrows &mut self.entities and
&mut self.substrate.occupancy".

**Step 4 — Compile.** Run: `cargo check -p vera20k`. Expected: clean once all flagged
`self.occupancy` sites and the 3 external sites (Task 4) are fixed. Re-run Step 1's grep until
zero `no field` errors remain in `sim/`.

### Task 4: Route the 3 external sites through the accessors

**Why:** Above-sim code can no longer touch the field; use the new accessors.

**Files:**
- Modify: `src/app_render/build_instances.rs:355`
- Modify: `src/app_skirmish.rs:757`
- Modify: `src/app_sim_tick.rs:304`

**Step 1 — `build_instances.rs:355`** (read, passed by reference into a struct field):
```rust
        occupancy: sim.occupancy(),
```
(was `occupancy: &sim.occupancy,` — `occupancy()` already returns `&OccupancyGrid`, so drop
the `&`.)

**Step 2 — `app_skirmish.rs:757`** (read):
```rust
    if sim.occupancy().get(rx, ry).is_some() {
```

**Step 3 — `app_sim_tick.rs:304`** (mutation):
```rust
                    sim.occupancy_mut().remove(rx, ry, *dead_id);
```

**Step 4 — Confirm `pixel_fx_sparkles.rs:266` is untouched.** It reads `input.occupancy` (a
render-input struct field fed by `build_instances.rs:355`), not a `Simulation` — leave it.

**Step 5 — Compile.** Run: `cargo check -p vera20k`. Expected: clean.

### Task 5: Verify hash-identity + full suite + commit

**Why:** Prove the refactor is behavior-preserving — same hash, same tests — before committing.

**Step 1 — Find the replay/hash test.** Run:
`rg -n "5000|replay|state_hash|hash_identical|bit-identical" src/sim/world/world_hash.rs tests`
and identify the deterministic replay/hash test(s). Note their names.

**Step 2 — Targeted tests.** Run:
`cargo test -p vera20k -- world_hash occupancy snapshot lifecycle`
Expected: PASS (read the literal `test result:` line).

**Step 3 — Full suite.** Run: `cargo test -p vera20k`. Expected: **same pass count as before
this change, 0 failed** (baseline this session: lib 3389 passed / 0 failed). A hash-sensitive
test that moves means the serialized form or a read path changed — **stop and investigate**
(likely a missed `#[serde(skip)]` or a wrong rebuild target). Do not re-baseline a golden.

**Step 4 — Confirm no occupancy field access leaked.** Run:
`rg -n "\.occupancy\b" src --glob '!src/sim/occupancy.rs'`
Every remaining hit must be `self.substrate.occupancy`, a local `&mut OccupancyGrid` param, a
`.occupancy()`/`.occupancy_mut()` call, or the unaffected `input.occupancy` in
`pixel_fx_sparkles.rs`. No bare `sim.occupancy`/`self.occupancy` field access should remain.

**Step 5 — Commit.** `refactor(sim): move OccupancyGrid into ObjectSubstrate (Slice 1a)`

## Sources & References

- **Design doc:** docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md — §6 (lines
  176–182), Slice 1 (225), borrow discipline (193), retire-list #3 (211).
- **Current Rust:** `src/sim/world/substrate.rs` (target owner); `src/sim/world/mod.rs` —
  field 366-368, init 498, accessors target, rebuild 1026, debug-rebuild ~1538, movement
  co-borrow 1586-1591; `src/sim/occupancy.rs:108` (`OccupancyGrid: Default`), `:167`
  (`new()`). External: `src/app_render/build_instances.rs:355`, `src/app_skirmish.rs:757`,
  `src/app_sim_tick.rs:304`; unaffected `src/render/pixel_fx_sparkles.rs:266`.
- **Prior commit:** `c2b5153` (Slice 1 partial — substrate owns active-vector + counters).
- **INI keys:** none (pure refactor).
- **Ghidra:** none (no binary dependency).
