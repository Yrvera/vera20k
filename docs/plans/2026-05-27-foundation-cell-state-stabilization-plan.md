# Foundation / Cell-State Stabilization - Implementation Plan

> Execute this plan task-by-task. This plan covers Phase 1 only: contract
> extraction, naming cleanup, and guard tests. Do not implement hidden-counter
> storage, `CanBeHidden`, `[General] Behind`, exact foundation table dumps, or
> broad file splits in this plan.

**Goal:** Move building cell derivation out of `production_tech.rs`, remove the
ambiguous `building_footprint_cells` public API, and force callers/tests to choose
base-foundation or movement-blocking semantics explicitly.

**Design Doc:** [docs/plans/2026-05-27-foundation-cell-state-stabilization-design.md](2026-05-27-foundation-cell-state-stabilization-design.md)

---

## Grounding Summary

- `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md` verifies that
  `Foundation=` selects a fixed foundation table/list and that `AddOccupy` /
  `RemoveOccupy` do not alter the normal foundation cell list.
- `BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`
  verifies ready placement and MCV/unit deploy validation walk base foundation
  cells and do not use add/remove hidden occupancy modifiers.
- `BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md` verifies
  movement/passability must keep base foundation object-list cells separate from
  `HasBib`, `NumberImpassableRows`, and hidden occupancy counters.
- `CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md` verifies
  `CellClass+0x100` is a hidden-object counter consumed by the behind-object
  path, not a movement/placement/C4/selection/radar blocker.
- `BUILDING_FOUNDATION_ANCHOR_SEMANTICS_GHIDRA_REPORT.md` verifies foundation
  origin semantics and warns that current Rust rectangle-derived foundation
  cells are mostly aligned for rectangular stock foundations, while exact binary
  foundation offset-list table contents remain a follow-up for special ids.

## Key Decisions

- **Create `src/sim/building_cells.rs` as the owner.** Building cell semantics
  are used by production, movement, pathfinding, gates, C4/building entry, and
  future hidden visibility. They do not belong in `production_tech.rs`.
- **Do not expose `building_footprint_cells`.** The name is too broad and the
  current function returns hidden adjusted occupancy, not real foundation cells.
- **Do not expose a Phase 1 hidden-counter API.** The current adjusted hidden set
  does not model `OccupyHeight`, enter/exit counter operations, or the
  `CellClass+0x100` writer sequence.
- **Keep current rectangle-derived base cells for Phase 1.** Exact gamemd
  foundation offset-list contents are deferred unless a later task scopes that
  verification.
- **Do not change static/live passability architecture in this plan.** Existing
  base/movement helpers are moved and renamed, not replaced by a new global
  `Can_Enter_Cell` implementation.

## Non-Goals

- No hidden-counter storage grid.
- No `CanBeHidden` or `[General] Behind` marker implementation.
- No exact foundation table memory dump.
- No large `drive_track.rs`, `pathfinding/core.rs`, `ruleset.rs`, or `app.rs`
  split.
- No generic cleanup or style-only refactors.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/sim/building_cells.rs` | Own base-foundation and movement-blocking helper contracts |
| Modify | `src/sim/mod.rs` | Publish `building_cells` module |
| Modify | `src/sim/production/production_tech.rs` | Remove moved helper implementations and keep production-specific tech logic |
| Modify | `src/sim/production/mod.rs` | Stop re-exporting `building_footprint_cells`; keep only scoped compatibility re-exports described below |
| Modify | `src/sim/pathfinding/core.rs` | Import building-cell helpers from new module and rename/remove compatibility wrapper surface |
| Modify | `src/sim/gate_runtime.rs` | Import base-foundation helper from new module |
| Modify | `src/sim/movement/bump_crush.rs` | Import base/movement helpers from new module |
| Modify | `src/sim/movement/movement_occupancy.rs` | Import base-foundation helper from new module |
| Modify | `src/sim/movement/movement_tests.rs` | Replace `building_footprint_cells` usage with explicit base helper |
| Modify | `src/sim/miner/miner_tests.rs` | Replace `block_building_footprint` usage with explicit movement blocker API |
| Modify | `src/sim/pathfinding/core_tests.rs` | Rename wrapper tests or assert new explicit API |

## Interface Changes

- New module: `crate::sim::building_cells`.
- Preferred names:
  - `foundation_dimensions`
  - `base_foundation_cells`
  - `movement_blocking_cells`
  - `movement_blocking_cells_for_state`
- Removed from public production exports:
  - `building_footprint_cells`
- Temporary production compatibility exports:
  - Keep `foundation_dimensions` re-exported from `crate::sim::production` for
    Phase 1 because it has a broad existing call surface and is not the ambiguous
    hidden-vs-foundation hazard this plan is fixing.
  - Allow old `building_base_foundation_cells`,
    `building_movement_blocking_cells`, and
    `building_movement_blocking_cells_for_state` aliases only as a Task 1 bridge.
    Remove them before final Phase 1 checks.
- Avoid adding a public hidden adjusted-set replacement in Phase 1. If tests still
  need the old adjusted helper, keep it private under an explicitly simplified
  name inside test scope.

## Sim Checklist

- [x] No floating-point sim math.
- [x] No new dependencies.
- [x] No `sim/` dependency on render/ui/sidebar/audio/net.
- [x] No new entity iteration order.
- [x] No state hash or snapshot change in Phase 1.
- [x] No change to `EntityStore` type or occupancy list storage.

## Risk Areas

| Risk | Mitigation |
|---|---|
| Moving helpers changes behavior accidentally | First task is a pure move plus compatibility aliases, with a green compile checkpoint before semantic renames |
| Tests keep using old alias with empty add/remove and hide the API hazard | Remove public alias and update tests to explicit base/movement helper names |
| Hidden occupancy helper is mistaken for real `CellClass+0x100` model | Do not expose a public hidden-counter API in Phase 1; document any remaining helper as compatibility/simplified |
| Static path-grid blockers become treated as full live `Can_Enter_Cell` parity | Keep wrapper names scoped to static movement blockers; defer live passability matrix work |
| Rectangle-derived base cells are overclaimed as exact gamemd table parity | Add comments/tests that Phase 1 preserves current rectangular implementation and defers exact offset-list dumps |
| `foundation_dimensions` migration balloons the task | Keep its production re-export temporarily; plan a separate cleanup only after the ambiguous building-cell API is gone |

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1-2 | `AddOccupy` / `RemoveOccupy` cannot be real foundation cells | Common refineries/factories use these modifiers; wrong callers cause false blockers and missing real cells | GAREFN/NAREFN tests and no public `building_footprint_cells` |
| 2 | GAREFN `(rx+3, ry+1)` remains base foundation despite `RemoveOccupy1=3,1` | Physical refinery pad is a real foundation cell; passability comes from bib/contact logic, not deletion | Existing/new C4 and movement blocker tests |
| 2 | GAREFN `(rx-1, ry)` and `(rx-1, ry-1)` are not base/path blocker cells | These are hidden occupancy only | New base-foundation and path-grid tests |
| 3 | `HasBib` applies to base foundation topology, not hidden adjusted shape | Applying bib after add/remove changes edge topology | Existing `garefn_bib_static_blockers_only_relax_east_edge` plus import move |
| 4 | `NumberImpassableRows` stays live-context scoped | Baking row count into static path grid can open/close wrong cells | Existing `movement_blocking_cells_for_state` tests preserved |

---

## Tasks

### Task 1: Create `sim::building_cells` With Moved Helpers

**Why:** Establish the correct ownership boundary before changing names. This
task should be behavior-preserving.

**Files:**
- Create: `src/sim/building_cells.rs`
- Modify: `src/sim/mod.rs`
- Modify: `src/sim/production/production_tech.rs`
- Modify: `src/sim/production/mod.rs`

**Steps:**

1. Create `src/sim/building_cells.rs` with a `//!` module doc explaining:
   - `sim/` owner, no render/ui/audio deps.
   - Base foundation, movement blocker, and hidden counter are separate gamemd
     concepts.
   - This module is Phase 1 and does not implement `CellClass+0x100`.
2. Move these functions from `production_tech.rs` without changing bodies:
   - `foundation_dimensions`
   - `building_base_foundation_cells`
   - `building_movement_blocking_cells`
   - `building_movement_blocking_cells_for_state`
3. Rename them in the new module:
   - `building_base_foundation_cells` -> `base_foundation_cells`
   - `building_movement_blocking_cells` -> `movement_blocking_cells`
   - `building_movement_blocking_cells_for_state` -> `movement_blocking_cells_for_state`
4. Add temporary compatibility `pub use` aliases in `production/mod.rs` for:
   - `foundation_dimensions`
   - `base_foundation_cells as building_base_foundation_cells`
   - `movement_blocking_cells as building_movement_blocking_cells`
   - `movement_blocking_cells_for_state as building_movement_blocking_cells_for_state`
   Do not include `building_footprint_cells`.
5. Add `pub mod building_cells;` to `src/sim/mod.rs`.
6. Update `production/mod.rs` re-exports to remove `building_footprint_cells`.
   Keep the scoped compatibility aliases above so this task ends green.

**Verification:**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
cargo check -q
```

Expected: PASS. Do not leave this task with unresolved imports. If compile fails,
fix compatibility aliases or moved-helper imports before continuing.

### Task 2: Migrate Runtime Callers To Explicit Helper Names

**Why:** Runtime code should describe which gamemd cell concept it consumes.

**Files:**
- `src/sim/gate_runtime.rs`
- `src/sim/movement/bump_crush.rs`
- `src/sim/movement/movement_occupancy.rs`
- `src/sim/pathfinding/core.rs`
- Any additional hits from:
  `rg "building_base_foundation_cells|building_movement_blocking_cells|building_movement_blocking_cells_for_state|block_building_footprint" src --glob '!src/graphify-out/**'`

Do not migrate broad `foundation_dimensions` callers in this task unless they are
already in the touched files. Leave the temporary production re-export for that
symbol in place through Phase 1.

**Steps:**

1. Replace `crate::sim::production::building_base_foundation_cells` with
   `crate::sim::building_cells::base_foundation_cells`.
2. Replace `crate::sim::production::building_movement_blocking_cells` with
   `crate::sim::building_cells::movement_blocking_cells`.
3. Replace `crate::sim::production::building_movement_blocking_cells_for_state`
   with `crate::sim::building_cells::movement_blocking_cells_for_state`.
4. In `PathGrid::block_building_movement_cells`, call the new module directly.
5. Rename `PathGrid::block_building_footprint` if feasible:
   - preferred: remove it and migrate tests to `block_building_movement_cells`.
   - acceptable if tests need it temporarily: mark it `#[cfg(test)]` and rename
     to `block_building_movement_cells_compat`.
6. Remove the temporary production compatibility aliases for:
   - `building_base_foundation_cells`
   - `building_movement_blocking_cells`
   - `building_movement_blocking_cells_for_state`
7. Run:

```powershell
rg "building_footprint_cells|block_building_footprint|building_base_foundation_cells|building_movement_blocking_cells|building_movement_blocking_cells_for_state" src --glob '!src/graphify-out/**'
```

Expected: no production/runtime usage remains. Test-only wrapper hits are allowed
only if they are renamed away in Task 3.

**Verification:**

```powershell
cargo check -q
```

Expected: PASS. Do not defer compile failures to Task 3.

### Task 3: Migrate Tests And Remove The Ambiguous Alias

**Why:** Tests must stop normalizing the ambiguous terminology.

**Files:**
- `src/sim/production/production_tech.rs`
- `src/sim/movement/movement_tests.rs`
- `src/sim/miner/miner_tests.rs`
- `src/sim/pathfinding/core_tests.rs`

**Steps:**

1. Delete `building_footprint_cells` from runtime code.
2. Replace movement test uses of `building_footprint_cells(..., &[], &[])` with
   `base_foundation_cells(...)`.
3. Replace path/miner test uses of `block_building_footprint` with
   `block_building_movement_cells`, unless a test-only compat wrapper remains.
4. If existing hidden adjusted-set unit tests are worth keeping, move them into
   `building_cells.rs` test module under a private helper named
   `simplified_hidden_adjusted_cells_for_legacy_tests` or similar, with a comment:
   "This is not the final `CellClass+0x100` writer model."
5. Add a compile-time/API guard by running:

```powershell
rg "building_footprint_cells" src --glob '!src/graphify-out/**'
```

Expected: no hits.

**Verification:**

```powershell
cargo test -q building_cells
cargo test -q garefn
cargo test -q c4_claims_from_remove_occupy_foundation_cell
```

Expected: PASS.

### Task 4: Add Focused GAREFN/NAREFN Contract Tests

**Why:** These are the high-frequency stock structures that expose the old
hidden-vs-foundation ambiguity.

**Files:**
- Prefer: `src/sim/building_cells.rs` test module.
- Existing tests may stay in `production_tech.rs` only if helpers remain there
  during migration, but the final owner should be `building_cells.rs`.

**Tests:**

1. `garefn_base_foundation_keeps_remove_occupy_pad`
   - origin `(10, 20)`, foundation `4x3`
   - assert `(13, 21)` is present
   - assert `(9, 20)` and `(9, 19)` are absent
2. `narefn_remove_occupy_does_not_change_base_foundation`
   - origin `(10, 20)`, foundation `4x3`
   - assert base count is `12`
   - assert representative remove-only offsets do not alter base set
3. `garefn_bib_movement_uses_base_topology`
   - derive base foundation, then movement blockers with `has_bib=true`
   - assert east edge `(13, 20)`, `(13, 21)`, `(13, 22)` is relaxed
   - assert `(12, 21)` remains blocked
   - assert west add-only cells are absent

**Verification:**

```powershell
cargo test -q building_cells
cargo test -q garefn_bib
```

Expected: PASS.

### Task 5: Final Checks And Documentation Cleanup

**Why:** Ensure the first slice did not drift into hidden-counter implementation
or leave ambiguous public APIs behind.

**Steps:**

1. Run API searches:

```powershell
rg "building_footprint_cells|hidden_occupancy_counter_cells" src --glob '!src/graphify-out/**'
rg "CellClass\\+0x100" src --glob '!src/graphify-out/**'
rg "crate::sim::production::building_" src --glob '!src/graphify-out/**'
rg "crate::sim::production::foundation_dimensions|production::foundation_dimensions|use crate::sim::production::foundation_dimensions" src --glob '!src/graphify-out/**'
```

Expected:
   - no `building_footprint_cells` in `src/`
   - no new `hidden_occupancy_counter_cells`
   - `CellClass+0x100` hits are allowed only in comments/docstrings that state
     Phase 1 does not implement the hidden-counter writer model
   - no production-owned building-cell imports from runtime callers
   - `foundation_dimensions` production hits are allowed as temporary Phase 1
     compatibility debt and should be listed in the final implementation notes

2. Run focused tests:

```powershell
cargo test -q building_cells
cargo test -q pathfinding::core_tests
cargo test -q world_orders_c4_tests
```

3. Run final check:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
cargo check -q
```

4. Review changed files with:

```powershell
git diff -- src/sim/building_cells.rs src/sim/production/production_tech.rs src/sim/production/mod.rs src/sim/pathfinding/core.rs
```

Expected: changes are helper ownership/name cleanup plus tests only. No hidden
counter storage, no render integration, no broad refactor.

## Follow-Up Plans

Create separate plans before implementing any of these:

- Phase 2: `CellClass+0x100` hidden-counter writer lifecycle, including
  `OccupyHeight` diagonal coverage and enter/exit counter operations.
- Phase 3: `CanBeHidden` / `[General] Behind` marker lifecycle.
- Exact foundation offset-list table verification for non-rectangular/special
  foundation ids.
- File splits for large movement/pathfinding/rules/app modules after contracts
  are stable.
