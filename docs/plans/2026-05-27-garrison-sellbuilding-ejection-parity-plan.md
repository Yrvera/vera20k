# Garrison SellBuilding Ejection Parity - Implementation Plan

> Execute this plan task-by-task. This plan supersedes the garrison occupant
> ejection placement/scatter parts of the 2026-05-04 unified garrison eject
> plan. Do not use the older plan's parachute, Guard->Scatter, or per-occupant
> edge-cell wording for this target.

## Goal

Match active YR `BuildingClass::SellBuilding @ 0x00457DE0` behavior for
`CanBeOccupied` garrison occupant ejection in Rust sell/destruction paths.

Scope is ejection placement and immediate post-placement handoff only. Do not
change civilian ownership reconciliation, command availability, visuals, or
full InfantryClass Scatter internals in this pass.

## Evidence

- `docs/research/GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`
- `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`
- `docs/research/GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`
- `docs/research/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
- `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`

## Architecture Context

Current Rust owner:

- `src/sim/production/production_sell.rs`
  - `sell_building` calls `eject_garrison_occupants` for player sell.
  - `eject_destruction_garrison` handles `CombatTickResult.destroyed_garrison_buildings`.
  - `eject_garrison_passengers_at_edges` currently uses sorted survivor cells,
    unique cells per occupant, and immediate `% 8` direct moves.
- `src/sim/combat/mod.rs`
  - `DestroyedGarrisonBuilding` snapshots destruction-time building and cargo data.
- `src/sim/world/mod.rs`
  - Drains destroyed garrison events after combat.

Keep crew survivor behavior separate. `sell_survivor_positions` is not a
garrison ejection primitive and should not be changed for this parity fix.

## Tiny-Detail Ledger

Implementation must preserve:

- Reset `PassengerCargo.garrison_fire_index` to zero when the helper clears the
  building cargo.
- If there are no occupants, do no placement and consume no RNG.
- Candidate edge scan order is:
  1. east/right column from southeast corner up to northeast outside corner
  2. south/bottom row from southeast corner west to southwest outside corner
  3. north/top row from `(rx, ry - 1)` east to northeast outside corner
  4. west/left row from `(rx - 1, ry)` south to southwest outside corner
- NW outside corner is skipped; SE, NE, and SW can be tested twice.
- Native does not clamp negative/off-map candidate cells in `SellBuilding`;
  candidates are passed through `MapClass::Get_CellClass` and accepted/rejected
  by the first-occupant predicate. Rust's current `u16` cell surfaces cannot
  represent negative cells cleanly, so any map-edge skip/clamp must be explicitly
  named as a temporary approximation until exact signed-cell handling exists.
- Candidate predicate is based on occupant slot 0 only. Native calls
  `Can_Enter_Cell(cell, -1, -1, 0, 1)` and accepts return `0`.
- Rust does not yet have exact `InfantryClass::Can_Enter_Cell` semantics in this
  helper. Use a named local approximation and keep it isolated behind a function
  so it can be replaced by the exact predicate later.
- Select one exit cell once, before occupant iteration.
- Reuse the same selected coordinate for every occupant.
- Iterate occupants high-to-low.
- Native failed `Unlimbo(chosen_coord, 0)` removes/destroys only that occupant
  and continues the loop. Rust does not currently expose an exact Unlimbo
  success/failure predicate in this helper, so do not claim failed-placement
  parity unless the patch adds a real, named approximation with tests.
- Do not draw RNG in the ejection helper for scatter.
- Do not call `movement::issue_direct_move` from the ejection helper.
- No-exit behavior is mode-specific:
  - destruction/red-HP: null branch removes occupants with no scatter/mission/RNG
  - player sell: use inside-foundation fallback `(rx + width - 1, ry + height - 1)`
- Successful placement handoff is archive-target clear plus direct Scatter in
  native. Rust does not have that exact Scatter surface yet; this pass should
  remove the incorrect direct movement/RNG and leave a narrow TODO for exact
  Scatter rather than replacing one wrong movement with another.
- Later mission `0xF` block is first-argument gated and not active for the direct
  callers checked by the 2026-05-27 swarm. Do not unconditionally model it here.

## Chosen Approach

Add a garrison-specific ejection path in `production_sell.rs`:

- `enum GarrisonEjectMode { PlayerSell, DestructionNoExitRemove }`
- `fn garrison_sellbuilding_exit_cells(...) -> Vec<(u16, u16)>`
- `fn choose_garrison_exit_cell(...) -> Option<(u16, u16)>`
- Update `eject_garrison_passengers_at_edges` to accept a mode and use the new
  scan/placement contract.

This keeps `sell_survivor_positions` unchanged for crew survivor spawn.

## Patch Tasks

### Task 1: Add exact edge-order helper

File: `src/sim/production/production_sell.rs`

Add `garrison_sellbuilding_exit_cells(rx, ry, width, height)`.

Requirements:

- Use signed intermediate math so native negative candidates can be represented
  during scan construction without wrapping.
- If the first implementation must skip negative cells before converting back to
  `u16`, name that as a temporary Rust boundary approximation in the helper and
  tests.
- Preserve duplicate-capable SE/NE/SW cells.
- Do not include NW outside corner.
- Add unit tests for a 2x2 building at `(10,10)` and a map-edge case at `(0,0)`.

Proposed tests:

- `garrison_sellbuilding_scan_order_matches_gamemd_edges_2x2`
- `garrison_sellbuilding_scan_order_handles_map_edge_without_u16_wrap`

### Task 2: Add one-coordinate selection

File: `src/sim/production/production_sell.rs`

Add `choose_garrison_exit_cell`.

Initial Rust predicate:

- First occupant only.
- Reject occupied live map cells as the current best local approximation.
- Do not use `used_cells`.
- Do not examine every occupant.

The approximation must be named/commented as a temporary stand-in for
`Can_Enter_Cell(cell,-1,-1,0,1)`. Because the verified report explicitly says
native does not model the predicate as occupancy-only checks, this approximation
is a known DRIFT reduction step, not full accepted-cell parity.

Proposed tests:

- `garrison_exit_probe_uses_first_occupant_only`
- `garrison_sellbuilding_reuses_single_exit_coord_for_all_lifo_occupants`

### Task 3: Replace per-occupant ejection placement

File: `src/sim/production/production_sell.rs`

Change `eject_garrison_passengers_at_edges`:

- Accept `GarrisonEjectMode`.
- Choose one coordinate once.
- Iterate `passenger_ids.iter().rev()`.
- For each occupant:
  - set `PassengerRole::None`
  - apply `owner_override` if present
  - set position to chosen coordinate or fallback coordinate
  - update occupancy
  - do not call `issue_direct_move`
  - do not consume RNG
- Reuse the chosen coordinate for every occupant; do not advance to the next edge
  cell to avoid stacking.
- Do not add a fake failed-placement branch based on `OccupancyGrid::add`; that
  API does not reject placements. If a real temporary Unlimbo predicate is added
  later, failed placement should mark only that occupant dead/dying and continue,
  approximating native failed `Unlimbo -> +0xF8`.

Proposed tests:

- `garrison_sellbuilding_reuses_single_exit_coord_for_all_lifo_occupants`
- `garrison_ejection_does_not_consume_raw_scatter_rng`

### Task 4: Implement caller-specific no-exit modes

File: `src/sim/production/production_sell.rs`

- `eject_garrison_occupants` passes `GarrisonEjectMode::PlayerSell`.
- `eject_destruction_garrison` passes `GarrisonEjectMode::DestructionNoExitRemove`.

Behavior:

- Player sell: when no edge cell is accepted, place from inside-foundation
  fallback `(rx + width - 1, ry + height - 1)` and continue LIFO placement.
- Destruction/red-HP: when no edge cell is accepted, remove/destroy all occupants
  high-to-low, no occupancy placement, no scatter, no RNG.

Proposed tests:

- `garrison_player_sell_no_exit_uses_inside_foundation_fallback`
- `garrison_destruction_no_exit_removes_without_rng_or_scatter`

### Task 5: Update stale comments and destruction event docs

Files:

- `src/sim/production/production_sell.rs`
- `src/sim/combat/mod.rs`

Update comments that still say:

- random foundation placement
- sorted edge scan
- parachute fallback
- direct scatter movement

Do not change combat event fields unless implementation requires it.

### Task 6: Run focused verification

Commands:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
cargo test -q garrison --lib
cargo check -q
```

If another cargo/rustc process is active, wait rather than running in parallel.

## Acceptance Criteria

- Garrison edge scan order matches the verified report.
- Garrison ejection no longer consumes `% 8` RNG or direct-moves occupants.
- Player-sell and destruction no-exit branches differ as verified.
- Occupants eject high-to-low.
- One selected coordinate is reused for all occupants.
- Existing captured civilian sell behavior still removes/refunds building as
  covered by the current tests.
- `cargo test -q garrison --lib` and `cargo check -q` pass, unless blocked by
  unrelated existing work.

## Known Follow-Ups

- Exact `InfantryClass::Can_Enter_Cell(cell,-1,-1,0,1)` in this context.
- Exact signed/off-map `MapClass::Get_CellClass` candidate behavior at map
  boundaries.
- Exact `Unlimbo(chosen_coord, 0)` placement success/failure behavior for
  multiple occupants reusing one coordinate.
- Full InfantryClass Scatter mission/destination implementation.
- Red-HP garrison ejection caller if/when the Rust health reconciliation path is
  implemented.
