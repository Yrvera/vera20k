# Foundation / Cell-State Stabilization Design

## Goal

Stabilize building cell semantics so Rust callers cannot accidentally use hidden
occupancy cells as real building foundation, movement, placement, C4, selection,
or radar cells.

## Architecture Context

Current Rust has the right high-level layering goal: `rules/` parses object and
art data, `sim/` owns deterministic gameplay state, `pathfinding/` and
`movement/` consume sim-owned cell facts, and app/render code rebuilds view-facing
data from sim and map state. The foundation helpers currently live in
`src/sim/production/production_tech.rs` and are re-exported through
`src/sim/production/mod.rs`.

The current helper set already contains several gamemd-shaped names:

- `building_base_foundation_cells`: base rectangle/list used by normal building
  occupancy.
- `building_hidden_occupancy_cells`: current adjusted hidden-occupancy helper.
  This is not a complete gamemd hidden-counter writer because it does not model
  `OccupyHeight` diagonal coverage, enter/exit direction, or counter operations.
- `building_movement_blocking_cells` and
  `building_movement_blocking_cells_for_state`: movement blocker derivation.

The remaining hazard is the compatibility alias `building_footprint_cells`, which
returns hidden occupancy, not real foundation cells. That name is too broad for a
repo where gamemd has multiple distinct "building cell" concepts.

Current direct caller groups from `rg`:

- Static path grid blockers:
  `src/app_init.rs:904`, `src/app_sim_tick.rs:880`, `src/app_sim_tick.rs:891`,
  `src/sim/pathfinding/core.rs:1944`.
- Dynamic movement block sets:
  `src/sim/movement/bump_crush.rs:143`, `src/sim/movement/bump_crush.rs:154`.
- Structure occupancy/spawn:
  `src/sim/world/world_spawn.rs:250`, `src/sim/world/world_spawn.rs:443`.
- Gate and live occupancy helpers:
  `src/sim/gate_runtime.rs:108`,
  `src/sim/movement/movement_occupancy.rs:337`.
- Compatibility wrappers/tests:
  `src/sim/pathfinding/core.rs:1962`,
  `src/sim/miner/miner_tests.rs:2566`,
  `src/sim/miner/miner_tests.rs:2738`,
  `src/sim/movement/movement_tests.rs:1729`,
  `src/sim/movement/movement_tests.rs:1782`,
  `src/sim/movement/movement_tests.rs:1854`.

## Impact Analysis

Primary touched modules for the first implementation slice:

- `src/sim/production/production_tech.rs`: move or narrow building-cell helpers.
- `src/sim/production/mod.rs`: update public re-exports.
- New `src/sim/building_cells.rs` or `src/sim/cell_state/building_cells.rs`:
  preferred long-term owner for cross-system building cell contracts.
- `src/sim/pathfinding/core.rs`: rename compatibility blocker APIs and remove
  add/remove parameters from static blocker APIs.
- Tests in production, pathfinding, movement, and C4/building-entry surfaces.

Secondary or later-phase modules:

- `src/sim/occupancy.rs`: may gain hidden-counter adjacency or a sibling
  per-cell state grid. Do not fold hidden counters into occupant lists.
- `src/sim/world/world_spawn.rs`: eventual enter/exit hidden-counter writes for
  placed/spawned structures.
- `src/app_sim_tick.rs` or render/animation integration: eventual
  `CanBeHidden` / `[General] Behind` marker lifecycle.

Risk areas:

- Pathing can regress if static blockers replace live object-list checks too
  broadly. The design must keep static `PathGrid` and live
  `UnitClass::Can_Enter_Cell`-style checks distinct.
- Existing tests that import `building_footprint_cells` with empty add/remove
  args may keep passing while preserving the bad API. The alias must be removed
  or made private/test-local.
- Hidden occupancy is not a path blocker. Implementing it before its verified
  consumer would risk new false blockers.

## Chosen Approach

Use a staged contract extraction, not a repo-wide rewrite.

Phase 1 creates a single sim-owned building-cell contract module, removes the
ambiguous alias from production exports, and migrates direct callers to explicit
APIs. This phase should preserve current behavior where the current behavior is
already verified, and should only change behavior where the research says the old
alias was semantically wrong.

Phase 2 adds explicit hidden-counter state and writer semantics, but still does
not use it for pathing, placement, selection, targeting, C4, or radar.

Phase 3 implements the verified downstream hidden-counter reader and behind-marker
integration for non-building technos with `CanBeHidden`.

Phase 4 can split large files after the contracts are stable.

## Tiny-Detail Ledger

- `Foundation=` resolves to a fixed table/list, not a free-form shape. Unknown
  values fall back through the foundation table behavior. Source:
  `docs/research/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`
  lines 12, 20-22, 48; `docs/research/BUILDING_FOUNDATION_ANCHOR_SEMANTICS_GHIDRA_REPORT.md`
  lines 53-55.
- Rust currently derives base foundation cells from foundation dimensions. That
  is aligned for rectangular stock foundations, but the exact binary foundation
  offset-list table contents were not re-dumped in the anchor-semantics report.
  Source: `docs/research/BUILDING_FOUNDATION_ANCHOR_SEMANTICS_GHIDRA_REPORT.md`
  lines 121-122, 140, 201.
- Structure `position.rx/ry` is the foundation origin, not the projected building
  center. Base foundation offsets, add/remove offsets, refinery pad examples,
  and row-helper X comparisons are relative to that origin. Source:
  `docs/research/BUILDING_FOUNDATION_ANCHOR_SEMANTICS_GHIDRA_REPORT.md`
  lines 24, 174-176, 188.
- `Foundation=` is read from the art/image section first, then object/rules can
  override with a non-default resolver result. Source:
  `docs/research/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`
  lines 33-36.
- `AddOccupy1..8` and `RemoveOccupy1..8` are parsed as exactly eight numbered
  slots with sentinels for absent/malformed entries. Source:
  `docs/research/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`
  lines 40-43.
- `BuildingClass::Place_OccupyMap` walks the base foundation list only and does
  not read add/remove offsets. Source:
  `docs/research/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`
  lines 54-55.
- Placement validators and MCV/unit deploy validation walk base foundation cells;
  add/remove modifiers are not placement inputs. Source:
  `docs/research/BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`
  lines 12-14, 34, 58, 109-110.
- Placement/deploy does not add an all-cells-same-height foundation rejection.
  Source:
  `docs/research/BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`
  lines 12, 112.
- `TechnoClass__EnterCell_AddToMultiCells` first adds the object to every base
  foundation cell, then conditionally updates hidden counters if the object is a
  building with `CanHideThings`. Source:
  `docs/research/BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`
  lines 24-25.
- `RemoveOccupy` decrements/cancels the hidden counter only; it does not remove
  the building object from base content lists. Source:
  `docs/research/BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`
  lines 26, 61-67.
- `UnitClass::Can_Enter_Cell` scans selected cell object lists and does not use
  `CellClass+0x100` as a direct pathing input in the checked path. Source:
  `docs/research/BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`
  lines 27, 87-89; `docs/research/CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`
  lines 72, 106-107.
- `HasBib` is a live unit-entry relaxation branch for matching buildings and
  should be modeled as building/passability behavior, not as hidden occupancy.
  Source:
  `docs/research/BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`
  lines 28, 71-75, 89.
- `NumberImpassableRows` has verified behavior in helper `0x00458A00` and must
  remain tied to live building-entry/passability contexts, not generic
  foundation shape. Source:
  `docs/research/BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`
  lines 29, 77-81; `docs/research/BUILDING_FOUNDATION_ANCHOR_SEMANTICS_GHIDRA_REPORT.md`
  lines 162-165, 181, 191.
- `CellClass+0x100` is a hidden-object counter. The only verified semantic
  battle reader is the behind-building hiding helper, not movement, placement,
  targeting, selection, radar, or ordinary building render. Source:
  `docs/research/CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`
  lines 12, 72-79, 106-107, 116.
- The hidden reader has an edge case: if a building object is present in the
  cell object list and the hidden counter is exactly `1`, it reports not hidden;
  if the counter is greater than `1`, it reports hidden. Source:
  `docs/research/CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`
  lines 30, 44-52, 117.
- Hidden-object visual effect applies to non-building technos with
  `CanBeHidden=true`; buildings are excluded as hidden subjects. The marker is
  resolved through `[General] Behind`, not a hardcoded retail asset. Source:
  `docs/research/CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`
  lines 22-24, 58-66, 108-110, 118.
- Radar/minimap building registration iterates base foundation bucket offsets,
  not adjusted hidden occupancy offsets. Source:
  `docs/research/BUILDING_FOOTPRINT_CONSUMER_DISCREPANCY_AUDIT_GHIDRA_REPORT.md`
  lines 28, 69.
- Single-click building hit geometry for non-rectangular foundations is not
  proven by the anchor-semantics report. Do not cite this design as proof that
  click selection should use hidden occupancy or a rectangle for every future
  case. Source:
  `docs/research/BUILDING_FOUNDATION_ANCHOR_SEMANTICS_GHIDRA_REPORT.md`
  lines 170, 185, 199.

## Design

### Components

`sim::building_cells`

Owns cross-system building cell derivation. It should start by moving the helper
logic out of `production_tech.rs` without changing consumers unnecessarily.

Initial API:

```rust
pub fn foundation_dimensions(foundation: &str) -> (u16, u16);
pub fn base_foundation_cells(origin_rx: u16, origin_ry: u16, foundation: &str) -> Vec<(u16, u16)>;
pub fn movement_blocking_cells(base_foundation: &[(u16, u16)], has_bib: bool) -> Vec<(u16, u16)>;
pub fn movement_blocking_cells_for_state(...) -> Vec<(u16, u16)>;
```

Naming rule: never expose a generic `footprint` helper from this module. If a
caller needs a purpose-specific alias, the alias must include the purpose:
`selection_cells`, `radar_registration_cells`, `placement_cells`, etc.

Phase 1 should not expose a final hidden-counter API. The current
`building_hidden_occupancy_cells` adjusted-set helper may be moved only as a
private compatibility helper for existing tests, or renamed so it cannot be
confused with the verified `CellClass+0x100` writer model. The real hidden
counter writer belongs to Phase 2.

`sim::cell_state` or extension to `Simulation`

Later phase owner for hidden counters. Do not put hidden counters into
`OccupancyGrid::occupants`; gamemd separates object lists from `CellClass+0x100`.

Likely shape:

```rust
pub struct HiddenOccupancyGrid {
    counters: BTreeMap<(u16, u16), u32>,
}

pub enum HiddenOccupancyOp {
    Increment { cell: (u16, u16) },
    DecrementIfNonZero { cell: (u16, u16) },
}
```

It should provide increment/decrement-with-underflow-guard primitives, writer
logic that receives `CanHideThings`, `OccupyHeight`, `AddOccupy`, `RemoveOccupy`,
and enter/exit direction, plus a reader equivalent to the verified `FUN_00487E00`
semantics. The writer must model diagonal `OccupyHeight` coverage from base
foundation cells and not collapse the operation sequence into one final set.

`pathfinding::PathGrid`

Keep terrain/static blocking separate from live object-list semantics. Rename
`block_building_footprint` to a compatibility-only test helper or delete it after
callers migrate. Runtime code should call `block_building_movement_cells` or a
new name that makes static-grid scope explicit.

### Interfaces / Contracts

Base foundation contract:

- Used by placed-building normal occupancy, placement/deploy validation, C4 and
  building-entry target cell membership, gate hold footprint, selection geometry
  where research confirms foundation-class behavior, and radar/minimap building
  registration.
- Must not read `AddOccupy` or `RemoveOccupy`.
- Phase 1 keeps the current rectangle-derived Rust implementation for base
  cells. Exact foundation-list table contents remain a follow-up unless a scoped
  implementation task first verifies the special ids it needs.

Movement blocker contract:

- Derived from base foundation and live building passability flags.
- May use `HasBib` and, only in verified active contexts,
  `NumberImpassableRows`.
- Must not use hidden occupancy add/remove as object-list cells.

Hidden occupancy contract:

- Driven by `CanHideThings`, `OccupyHeight`, `AddOccupy`, and `RemoveOccupy`.
- Maintained as counters with nonzero decrement guards.
- Consumed only by the behind-object hiding path unless future research proves
  another active reader.
- Not represented by a single "hidden footprint" set in the final model. Enter
  and exit produce ordered counter operations.

Visual/render contract:

- Not owned by this design except for the eventual behind-marker integration.
  Do not derive sprite extents or bracket/render bounds from hidden occupancy
  without a composition/render report.

### Data Flow

Phase 1:

1. `rules/` continues to parse foundation, add/remove offsets, `CanHideThings`,
   `OccupyHeight`, `Bib`, and `NumberImpassableRows`.
2. `sim::building_cells` derives named cell sets from parsed rules data.
3. Existing production re-exports either point to the new module or are replaced
   by explicit imports.
4. Runtime callers migrate off `building_footprint_cells`.
5. Hidden-counter behavior remains deferred; no new public "counter cells" API is
   introduced.

Phase 2:

1. Structure spawn/place/despawn paths call hidden-counter writer functions only
   after base occupancy is registered.
2. Hidden-counter writer reads `CanHideThings`, `OccupyHeight`, `AddOccupy`, and
   `RemoveOccupy`.
3. Hidden-counter writer emits enter/exit counter operations, including diagonal
   `OccupyHeight` coverage and nonzero decrement guards.
4. Movement/pathing, placement, C4, selection, and radar ignore the hidden
   counter.

Phase 3:

1. Per-techno update checks `CanBeHidden`.
2. Current cell is resolved.
3. Hidden-counter reader applies the exact building-present/counter-equals-one
   carve-out.
4. Non-building technos create/keep/destroy the `[General] Behind` marker.

### Error Handling

No new fallible API is required in Phase 1. Foundation names should continue to
flow through the existing fixed foundation table fallback.

Hidden-counter Phase 2 should use saturating/nonzero-guard decrement semantics to
match the verified writer guard. Invalid/off-map computed cells should be dropped
the same way current helpers drop coordinates outside `u16` range, unless a later
binary check proves a different map-bound behavior.

### Testing Strategy

Phase 1 tests:

- GAREFN base foundation includes `(rx+3, ry+1)` despite `RemoveOccupy1=3,1`.
- GAREFN base foundation excludes `AddOccupy` cells `(rx-1, ry)` and
  `(rx-1, ry-1)`.
- Runtime path-grid static blockers do not block GAREFN add-only cells.
- C4/building entry uses base foundation membership, not hidden occupancy.
- `building_footprint_cells` is no longer exported to production/runtime callers.
- Any moved current hidden adjusted-set helper is private or explicitly named as
  compatibility/simplified, not as a final hidden-counter model.

Phase 2 tests:

- Hidden counter increments for add cells and diagonal/base-derived hidden cells
  based on `OccupyHeight` when
  `CanHideThings=true`.
- Hidden counter skips all hidden writes when `CanHideThings=false`.
- Remove cells decrement only when nonzero and do not affect object-list/base
  foundation membership.
- Exit reverses diagonal and add increments without "re-adding" remove cells.

Phase 3 tests:

- A non-building techno in a cell with hidden counter `1` and a building object
  in the ground list does not get a behind marker.
- The same setup with counter `2` does get the marker.
- Buildings never become hidden subjects.
- `[General] Behind` can be data-resolved rather than hardcoded.

Verification commands for implementation should start focused:

```powershell
cargo test -q building_cells
cargo test -q garefn
cargo test -q c4_claims_from_remove_occupy_foundation_cell
cargo test -q test_block_building_footprint
```

Then run a final `cargo check -q` after checking no other cargo/rustc process is
active.

## Architectural Decisions

- Keep this in `sim/`, not `production/`. Building cell semantics are consumed by
  production, movement, pathfinding, gates, C4, selection-facing queries, and
  future hidden visibility. Production is not the right owner.
- Do not introduce ECS or a broad cell-class rewrite. A small deterministic grid
  for hidden counters is enough for the verified behavior.
- Do not model hidden occupancy as path blocking. Research currently verifies
  the downstream semantic reader as behind-object visibility, and explicitly
  warns against treating `CellClass+0x100 != 0` as a global blocker.
- Do not expose the current adjusted hidden-occupancy set as the future
  `CellClass+0x100` model. The verified writer is operation/counter based and
  includes `OccupyHeight`.
- Keep static path-grid blockers as a pragmatic optimization only where they
  mirror verified base-foundation/object-list consequences. Live passability
  exceptions remain a separate movement/pathfinding concern.
- Split files only after the contract is stable. File splits without the contract
  would reduce line counts while preserving ambiguity.

## Alternatives Considered

### Big repo-wide refactor

Rejected for now. The repo has large files and some boundary debt, but a broad
cleanup would touch too many systems before the gamemd contracts are nailed down.
It would make parity regressions harder to isolate.

### Keep helpers in `production_tech.rs` and only rename the alias

Rejected as incomplete. The call sites are not production-specific. Keeping the
contract in production encourages unrelated systems to import from production and
keeps the ownership misleading.

### Implement hidden-counter rendering first

Rejected as the first slice. The hidden-counter reader and behind marker are real,
but the immediate root hazard is ambiguous foundation API usage. Build the
contract first, then add the counter and visual consumer.

### Static grid encodes all movement exceptions

Rejected. `HasBib` and `NumberImpassableRows` are live entry/passability logic
around building object lists. Some static blockers are useful, but the design
must not flatten all live UnitClass behavior into a precomputed grid.

## Implementation Handoff

Recommended first implementation slice:

1. Add `src/sim/building_cells.rs` with moved helper implementations and module
   docs tying each helper to the gamemd concept.
2. Re-export explicit names from `src/sim/mod.rs` or import via
   `crate::sim::building_cells`.
3. Remove `building_footprint_cells` from public production exports.
4. Replace test/runtime callers with explicit base or movement helper names. If
   hidden adjusted-set tests remain, move them to a private compatibility helper
   with wording that says it is not the final hidden-counter writer.
5. Add focused GAREFN/NAREFN tests for add/remove separation.
6. Run focused tests and `cargo check -q`.

Explicitly deferred:

- `CellClass+0x100` hidden-counter storage and writer lifecycle.
- `CanBeHidden` / `[General] Behind` marker lifecycle.
- Exact gamemd foundation offset-list table contents for special foundation ids
  beyond the currently rectangle-derived Rust implementation.
- Large file splits in movement, pathfinding, rules, and app orchestration.
- Generic cleanup or style-only refactors.
