# Refinery FreeUnit Completion and Stock Primary Placement Design

## Goal

Make stock Allied and Soviet refineries create their configured free miner exactly
once on the existing Rust building-up completion transition, at the verified stock
primary cell, without adding a second scheduler or persistent one-shot state.

## Architecture Context

The scoped player path is ordinary Yuri's Revenge skirmish construction of
`GAREFN` or `NAREFN`. Retail rules configure these buildings with
`FreeUnit=CMIN` and `FreeUnit=HARV`, respectively, and retail art gives both a
`4x3` foundation. The active behavior is therefore data-driven; simulation code
must not select a miner from faction identity or hard-code any of those four type
names.

Current placement authority is
`src/sim/production/production_placement.rs::place_ready_building`. It validates
the footprint, creates the building, and attaches a `BuildingUp` component with
the current approximate 30-tick duration. It also calls
`maybe_spawn_refinery_harvester` immediately, which creates the miner before the
building is complete.

The deterministic completion owner already exists in
`src/sim/world/mod.rs::tick_building_up`. It visits `EntityStore` stable IDs in
sorted order, increments each active `BuildingUp`, gathers finished IDs, and
clears their components. Phase 9 of `run_late_region` owns this transition and
already receives the rules, path-grid, and height-map inputs needed by the
production helper. The same phase accumulates an existing `spawned_entities`
result for downstream atlas refresh.

`src/sim/production/production_refinery.rs` owns the current refinery lookup,
foundation lookup, primary/fallback cell choice, facing selection, and call into
the generic world spawn path. That remains the production-policy owner. The
world tick should identify completions and preserve order, but should not absorb
refinery-specific rules policy.

By the completion tick, the app can have rebuilt `PathGrid` from the placed
building. Current Rust correctly keeps the interior `NW+(2,2)` cell blocked by
the refinery's ordinary movement footprint; stock `Bib=yes` relaxes only the
east-edge `x=3` column. Therefore the FreeUnit primary attempt must not use the
source refinery's static `PathGrid` mark as a rejection. The already-validated
foundation makes the terrain admissible, while result-bearing rejection of an
independently injected dynamic blocker remains a separate lifecycle residual.

The verified active-YR native path calls the building construction-completion
hook after construction completes, not during placement. That hook reads the
resolved BuildingType `FreeUnit`, derives a building-center cell, moves one cell
south, tries primary placement with facing `0xC0`, and then has two fallback
attempts with `0xA0`. For a `4x3` refinery anchored at north-west `(rx, ry)`, the
primary is `(rx+2, ry+2)`. Sources:

- `docs/research/miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`, section 2,
  backed by the construction-completion caller at `0x00449A50`, completion hook
  at `0x00445F80`, and center calculation at `0x00447AC0`.
- `ini/rulesmd.ini:11722-11740` and `ini/rulesmd.ini:12515-12534`.
- `ini/artmd.ini:1706-1716` and `ini/artmd.ini:1763-1773`.
- `docs/contracts/2026-07-28-refinery-freeunit-completion-implementation-contract.md`.

There is no relevant TS-versus-YR ambiguity for this scoped activation:
`GAREFN` and `NAREFN` use the key in active stock YR data, and the verified
ordinary construction mission reaches the completion hook.

## Impact Analysis

### Directly touched code

- `src/sim/production/production_placement.rs`
  - Remove the immediate refinery FreeUnit call and its private-module import.
- `src/sim/world/mod.rs`
  - Return completed stable IDs from `tick_building_up`.
  - Invoke the production completion service in Phase 9.
  - OR successful creation into `spawned_entities`.
- `src/sim/production/mod.rs`
  - Expose the narrow completion service within `crate::sim`.
- `src/sim/production/production_refinery.rs`
  - Accept completed building IDs.
  - Snapshot and resolve building data.
  - Derive the checked stock primary cell.
  - Return whether any entity was created.
  - Correct stale timing and facing prose.
- `src/sim/production/production_placement_tests.rs`
  - Replace immediate-placement expectations with completion-driven tests.

### Downstream effects

- Free miners appear later, on the current Rust completion tick rather than the
  placement-command tick. This intentionally changes state hashes and replay
  timelines for refinery construction.
- `TickResult.spawned_entities` becomes true on the completion tick, ensuring the
  existing app/render refresh path sees the new unit without a simulation-to-
  render dependency.
- Owner counts, logic registration, mission state, and visibility continue to
  flow through the existing generic spawn/lifecycle path.
- No data format, snapshot version, command format, INI schema, or network
  protocol changes are required.

### Risk areas

- Clearing `BuildingUp` without attempting the FreeUnit would consume the
  one-shot owner. Phase 9 must process the returned IDs immediately after the
  transition.
- Entity creation mutates `EntityStore`; completion IDs must be collected before
  committing any spawned units and processed in their existing sorted order.
- Coordinate overflow or underflow must not saturate into an unrelated map cell.
- The rebuilt `PathGrid` normally marks `NW+(2,2)` blocked by the source
  refinery. Treating that mark as primary-cell rejection would force every
  ordinary completion into Rust's uncertified fallback.
- The production path requires a `RuleSet`. Existing `rules=None` calls are
  synthetic/headless paths without production-data authority; they do not gain
  a deferred side-effect queue.
- Existing dynamic placement admission is incomplete. The scoped primary-success
  tests must not be presented as fallback, overlap-rejection, or refund parity.

## Chosen Approach

Use the current building-up transition as the deterministic event owner and an
immediately adjacent production service as the effect owner:

1. `tick_building_up` advances all buildings and clears completed components.
2. It returns its already stable-ID-ordered finished list.
3. Phase 9 immediately passes that list to a production-owned completion
   service, together with the current rules, path grid, and height map.
4. The service resolves and attempts each eligible stock refinery FreeUnit.
5. It returns whether any unit was created; Phase 9 folds that into
   `spawned_entities`.

This introduces no persistent event, pending marker, polling rule, or second
scheduler. The one-shot guarantee is the serialized `BuildingUp -> None`
transition itself. The design follows the existing split in which world code
owns deterministic tick order while `production/` owns build and placement
policy.

## Player-Experience Detail Ledger

- **MILESTONE-BLOCKING — completion timing:** no miner exists on building
  placement or any pre-completion tick; exactly one attempt occurs on the same
  tick that `BuildingUp` becomes `None`. A placement-time miner is visible and
  economically usable too early in every ordinary refinery build.
  `[GHIDRA 0x00449A50 -> 0x00445F80; contract timing row]`
- **MILESTONE-BLOCKING — stock primary cell:** a stock `4x3` refinery at
  `(rx,ry)` uses `(rx+2,ry+2)`, not the current `(rx+2,ry+3)`. This is the native
  bottom-middle bay and is exercised by every ordinary stock refinery.
  `[GHIDRA 0x00447AC0 and 0x00445F80; artmd.ini Foundation=4x3]`
- **MILESTONE-BLOCKING — data and owner:** `GAREFN` resolves `CMIN`, `NAREFN`
  resolves `HARV`, and the unit belongs to the building owner. No ConYard or
  faction-derived compensation is added.
  `[ini: rulesmd.ini FreeUnit=; corrected docking report section 6]`
- **COMPOUNDING — deterministic ordering:** simultaneous completions commit in
  stable-ID order, including stable-ID allocation, mission initialization, and
  state-hash effects. Reordering would threaten replay and multiplayer
  determinism. `[current Rust: tick_building_up keys_sorted]`
- **COMPOUNDING — downstream spawn notification:** successful completion
  creation sets `spawned_entities` in the same tick so presentation caches
  refresh through the existing result boundary. `[current Rust:
  run_late_region/TickResult]`
- **COMPOUNDING FOLLOW-UP — dynamic placement admission and refund:** the scoped
  path must deliberately ignore the source refinery's own static footprint for
  its internal primary bay. Native primary placement can still reject an
  independent blocker, retry one constructed object, and refund total failure.
  Current generic Rust spawning supplies `PlacementEvidence::MarkSucceeded` and
  cannot provide that transaction. The ordinary placement path begins with an
  unoccupied foundation and then blocks normal entry, so this does not block the
  selected primary-success scenario; alternative load/editor/injected occupancy
  can still expose the drift.
  Expected ordinary frequency is low, but lifecycle and credit effects make it
  a required follow-up rather than verified behavior.
  `[GHIDRA 0x00445F80; contract result-bearing/refund rows]`
- **EXACTIFICATION-RESIDUAL — fallback order:** native uses two ordered
  `Find_Nearby_Passable_Cell` calls with one undecoded boolean difference.
  Rust's radius-1-through-5 row-major perimeter scan remains DRIFT and is not
  certified by this design. Trigger: unavailable primary cell. Player effect:
  miner may choose a different fallback. Downstream risk is bounded while the
  selected stock primary cell is available.
  `[corrected docking report section 2; implementation contract blocker 1]`
- **EXACTIFICATION-RESIDUAL — successful mission handoff:** preserve the current
  Harvest mission result, numeric ID 10. Native assign-plus-Commence equivalence
  over timers, queued mission, and first eligible AI tick remains unverified.
  `[GHIDRA 0x00445F80; contract mission row]`
- **EXACTIFICATION-RESIDUAL — player-control gate:** the verified native
  conditional involving `BuildingClass+0x300` has no implementation-safe
  semantic mapping or demonstrated frequent stock trigger. No speculative Rust
  field or gate is added. `[contract blocker 2]`
- **EXACTIFICATION-RESIDUAL — mod activation:** native activation is generic to
  a completed BuildingType with a resolved UnitType `FreeUnit`; current Rust
  restricts the lookup to `Refinery=yes`. Stock assigns the key only to the two
  refineries, so generic mod behavior remains outside this stock slice.
  `[GHIDRA 0x0045FE50 and 0x00445F80; stock FreeUnit census]`
- **TEST-ONLY — facing bytes and prose:** preserve `0xC0` for primary and `0xA0`
  for fallback. Under project coordinates these are west and southwest,
  respectively; stale Rust comments calling them south-facing are corrected.
  `[ENGINE.md coordinate conventions; GHIDRA 0x00445F80]`

## Design

### Components

#### Building-up transition

`tick_building_up` remains responsible only for advancing and clearing the
generic completion state. Its output becomes the ordered list of stable IDs
whose state transitioned during this call. It does not inspect refinery rules
or spawn units.

The existing local `finished: Vec<u64>` becomes the return value, so this design
does not add another per-tick collection beyond the current implementation.

#### Production completion service

A crate-internal production entry point receives:

- `&mut Simulation`;
- the ordered completed-building IDs;
- `&RuleSet`;
- optional `&PathGrid`;
- the height map.

For each ID, it re-reads the still-live entity and snapshots the minimum data
needed before mutating the store: building type, owner, north-west cell, and
foundation. Missing or destroyed IDs are skipped safely.

The service stays refinery-scoped for this stock slice by using the existing
data-driven `refinery_free_unit` lookup. It does not hard-code stock IDs and
does not silently claim native generic-mod parity.

#### Primary-cell derivation

The primary helper derives the building-center cell in the canonical north-west
cell frame, then adds one cell south:

```text
center_x = rx + width/2
center_y = ry + height/2
primary  = (center_x, center_y+1)
```

Arithmetic uses a signed or wider checked intermediate and converts back to
`u16` only after range validation. For the scoped `4x3` foundation, this is
exactly north-west `+(2,2)`.

#### Spawn and notification

The completion service preserves primary facing byte `0xC0` and the current
Harvest mission result. A successful `spawn_object` contributes `true` to the
service result. Phase 9 ORs that result into `spawned_entities`.

For a valid primary coordinate, the service attempts that coordinate without
consulting `PathGrid`: the grid's blocker is the completing source refinery
itself, and allowing that expected co-occupant is required for the native
internal bay. The existing fallback helper may remain temporarily reachable
only when the checked primary coordinate cannot be represented. Its comments
and tests must call it an uncertified fallback rather than native-equivalent
behavior.

### Interfaces / Contracts

- `tick_building_up` output order is ascending stable ID.
- Every returned ID has already had `building_up` cleared.
- Phase 9 consumes the list immediately and exactly once.
- The completion service does not retain IDs or create persistent pending work.
- One eligible completion creates at most one living FreeUnit.
- Missing/invalid `FreeUnit` data is a no-op, not a failed allocation and not a
  refund case.
- No call crosses from `sim/` into app, render, audio, UI, sidebar, or network.
- A missing `RuleSet` means the production completion service cannot run. The
  generic building-up transition still completes; synthetic no-rules callers
  remain outside the production behavior contract.

### Data Flow

```text
PlaceReadyBuilding command
  -> place_ready_building
  -> spawn refinery + attach BuildingUp
  -> no FreeUnit yet

Phase 9 on later ticks
  -> tick_building_up in stable-ID order
  -> clear each completed BuildingUp
  -> return completed IDs
  -> production completion service
  -> refinery_free_unit lookup
  -> checked center-plus-south primary cell
  -> spawn_object with owner, 0xC0, and existing Harvest handoff
  -> spawned_entities = true on success
```

No RNG, command injection, render callback, or next-tick queue participates.

### Error Handling

- A completed ID no longer present in `EntityStore` is skipped.
- A non-building, non-refinery, absent-key, or invalid-target entity is skipped
  without refund.
- Invalid checked primary coordinates are treated as unavailable; they must not
  clamp or wrap into another cell.
- A valid primary is not rejected because `PathGrid` contains the completing
  source refinery's own movement blocker.
- Existing fallback/no-cell logging may remain, but no message may claim exact
  native fallback parity.
- Generic spawn failure remains visible through a warning and no spawned result
  in this slice. Exact constructed-object cleanup and owner-aware refund are
  deliberately not approximated before the result-bearing placement contract is
  designed.

### Testing Strategy

Use scoped `--lib` production tests while implementing. Do not run the full
library suite until the eventual merge-to-`dev` owner performs the single
project-required merge validation.

Required acceptance tests:

1. `stock_refinery_free_unit_spawns_on_building_up_completion_once`
   - Place GAREFN.
   - Assert zero CMIN immediately and on every pre-completion tick.
   - Assert exactly one CMIN on the transition tick.
   - Advance later ticks and assert no duplicate.
2. `stock_4x3_refinery_free_unit_uses_native_primary_cell`
   - Place GAREFN at `(20,20)`.
   - Before completion, mark the complete `4x3` foundation blocked in the test
     `PathGrid`, matching the normal post-placement grid rebuild.
   - Assert owner-matched CMIN at `(22,22)`, facing `0xC0`, mission Harvest/10,
     and no unit at `(22,23)`.
3. `stock_soviet_refinery_completion_spawns_harv`
   - Repeat the primary fixture with NAREFN and assert HARV.
4. `gacnst_completion_has_no_free_unit`
   - Complete a stock Allied ConYard and assert no miner and no mechanism-driven
     credit change.
5. `simultaneous_refinery_completions_preserve_stable_id_order`
   - Complete at least two eligible buildings on one tick.
   - Assert spawned unit stable IDs and type/owner association follow the
     completing-building order.
6. Completion-tick result assertion
   - Assert `TickResult.spawned_entities` is false before completion and true on
     successful completion.

Tests should use merged retail rules/art where stock type mapping and foundation
are the assertion. Small local fixtures may cover pure timing/order mechanics,
but they do not replace the Allied and Soviet retail-data fixtures.

No fallback-coordinate, occupied-primary, or refund test may be labelled a
parity acceptance test until the corresponding blockers in the implementation
contract are resolved.

## Architectural Decisions

- **Followed pattern:** world tick code owns deterministic phase and stable-ID
  order; a subsystem module owns domain policy.
- **Followed pattern:** creation remains inside the existing lifecycle/spawn
  authority and is surfaced through `TickResult`.
- **Followed pattern:** retail INI/art data chooses the target and foundation.
- **Intentional deviation from current code:** the building-up transition now
  emits a completion result rather than silently clearing state. This is a
  narrow interface extension, not a generic event bus.
- **Rejected native-structure copying:** no BuildingClass inheritance, virtual
  callback model, global direction table, or pointer-owned pending object is
  reproduced. Rust preserves the verified timing, ordering, data lookup, cell,
  facing, and same-tick consequence.
- **No new technical debt for one-shot state:** completion ownership reuses
  serialized `BuildingUp`; no new flag or migration is introduced.
- **Recorded existing debt:** result-bearing dynamic placement, exact fallback
  order, refund cleanup, generic mod activation, and mission Commence semantics
  remain explicit follow-ups rather than hidden approximations.

## Alternatives Considered

### Execute refinery policy directly inside `tick_building_up`

This would pass rules and map inputs into the generic timer function and spawn
while clearing each entity. It is mechanically small but mixes animation-state
progression with production rules policy, weakens isolated testing, and makes
future completion consumers harder to add without growing world code.

### Add a pending-FreeUnit marker or completion queue

Placement would create persistent pending work that a later system consumes
after observing completion. This adds snapshot/hash state, creates a second
one-shot owner, and risks a tick-late effect. It solves no requirement the
existing serialized transition cannot already own.

### Keep placement-time spawning and adjust only the cell

This would fix the visible coordinate but preserve the more important timing
and ownership drift. Every refinery would still grant its miner early, so it
does not meet the ordinary-skirmish goal.
