# Player-Sell Can_Enter_Cell Residual Postfix Trace

**Scenario:** Player sells an occupied `CanBeOccupied` garrison building where candidate edge cells include terrain, overlay, building, and occupancy blockers.

**Scope:** Only the shared Rust `garrison_infantry_can_enter_cell` gate used for player-sell exit placement and post-placement scatter destination filtering. This trace does not re-open scatter mission ordering, RNG timing, abandoned audio, render depth, or destruction no-exit behavior.

**Status:** COMPLETE. The residual is still real: Rust narrowed the garrison sell/scatter path to a shared gate, but that gate is still a phase-1 terrain/subcell approximation, not active YR `InfantryClass::Can_Enter_Cell`.

## Evidence Used

- Read-only Ghidra spot-check in this run:
  - `BuildingClass::SellBuilding @ 0x00457DE0`
  - `InfantryClass::Can_Enter_Cell @ 0x0051BF90`
- Verified research:
  - `docs/research/INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md`
  - `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`
  - `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`
  - `docs/research/traces/GARRISON_PLAYER_SELL_CANENTER_SCATTER_POSTFIX_TRACE.md`
- Rust source:
  - `src/sim/production/production_sell.rs`
  - `src/sim/pathfinding/cell_entry.rs`

## Active YR Confirmation

This is active in standard YR. `BuildingClass::SellBuilding @ 0x00457DE0` is reached from occupied `CanBeOccupied` garrison sell/destruction paths; the player-sell path scans candidate edge cells by calling the first occupant's vtable `+0x1AC` with `CellClass*,-1,-1,0,1` and accepts only return code `0`. `InfantryClass` binds vtable `+0x1AC` to `0x0051BF90`. No TS-only gate was found on this path.

## Pipeline

`player sell command` -> `SellBuilding-style garrison ejection` -> `edge candidate scan` -> `first occupant InfantryClass::Can_Enter_Cell == 0` -> `single exit coordinate or player-sell fallback` -> `occupants placed` -> `direct Scatter destination scan` -> `same Rust garrison_infantry_can_enter_cell approximation filters destinations`

## Stage Results

| Stage | gamemd output for blocker scenario | Current Rust output | Verdict |
|---|---|---|---|
| 1. Exit placement CEC callsite | For each candidate edge cell, `SellBuilding` calls first occupant vtable `+0x1AC` with `CellClass*,-1,-1,0,1`; return `0` accepts, nonzero continues. | `choose_garrison_exit_cell` probes only `passenger_ids.first()` and accepts `garrison_first_occupant_can_enter_cell == true`. | PASS |
| 2. Shared Rust gate inputs | Native receives the actual `CellClass*`, direction `-1`, path height `-1`, and final flag `1`; it can read terrain, overlay, object-list, bridge/list bits, owner, target, and building state. | `garrison_infantry_can_enter_cell` calls `check_terrain((rx,ry), Ground, infantry.category, None, None, &sim.occupancy)` and compares only to `TerrainCheckResult::Clear` (`production_sell.rs:300..327`). | FAIL |
| 3. Terrain-only blocker edge cell | Native `0x0051BF90` reaches land/speed and bridge/tube checks; impassable land or invalid tube/height cases return `7`, so the candidate is rejected. | With `path_grid=None` and `cost_grid=None`, `check_terrain` treats ground terrain as walkable by default (`cell_entry.rs:299..304`), so an otherwise empty terrain-only blocked candidate can return `Clear`. | FAIL |
| 4. Overlay-only blocker edge cell | Native reads `CellClass+0x44` overlay, overlay type flags, house/player gates, and can return `7` or set nonzero blocker code before final return. | Rust passes no map/overlay data into the gate; an overlay-only blocker that has not inserted occupancy is invisible to `garrison_infantry_can_enter_cell`. | FAIL |
| 5. Building blocker edge cell | Native has infantry-specific building logic: garrison/gate flags, `BuildingClass::CanGarrison`, owner/alliance, weapon range, and returns `0/3/5/7` depending on state. | Rust reduces building presence to `has_blockers_on`/subcell availability through occupancy only, with no building type, owner, CanGarrison, target, or weapon-range classifier. | FAIL |
| 6. Occupancy blocker edge cell | Native scans the selected ground/bridge object list and returns semantic codes including moving/friendly/enemy/building cases; exact terminal subcell-full ladder remains unresolved in prior research. | Rust checks whether an infantry subcell is available and whether selected-list blockers exist, then collapses everything except clear into `false`; it never computes native return code `1/2/3/5/6/7`. | UNCHECKED |
| 7. Bridge/layer edge cell | Native uses shared `CheckBridgeTraversal @ 0x004D9C60`, selected object-list layer, selected occupancy-bit layer, and the infantry `path_height - Level > 4 -> 0` shortcut. | Rust hardcodes `MovementLayer::Ground` and supplies no `CanEnterLayerContext`, bridge traversal input, path height, tube index, or layer split. | FAIL |
| 8. Scatter destination filtering | Native direct ejection Scatter searches candidate destination cells through active infantry scatter/passability behavior after its own pre-RNG gates. Blocked destination cells are rejected by native cell state. | `sellbuilding_direct_scatter_handoff` reuses `garrison_infantry_can_enter_cell(..., require_inside_transport=false)` for the eight adjacent candidates (`production_sell.rs:394..412`), so all stage 2-7 residuals also affect scatter destination choice. | FAIL |

## Verdict Tally

PASS: 1 | FAIL: 6 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Top Findings

1. **Stage 3 - terrain-only blocker FAIL.** Player-visible difference: ejected infantry can appear on or scatter into terrain cells native rejects. Rust: `src/sim/production/production_sell.rs:318`; `src/sim/pathfinding/cell_entry.rs:299`. gamemd: `InfantryClass::Can_Enter_Cell @ 0x0051BF90` land/tube/height branches return `7`.
2. **Stage 4 - overlay-only blocker FAIL.** Player-visible difference: overlay-only blockers can be ignored if they are not reflected in occupancy, changing exit/fallback and scatter destinations. Rust: `src/sim/production/production_sell.rs:318`. gamemd: `0x0051BF90` reads `CellClass+0x44` overlay and overlay type flags before returning nonzero.
3. **Stage 5 - building blocker FAIL.** Player-visible difference: Rust cannot match native garrison/gate/building owner cases, so a candidate building cell can be rejected or accepted differently. Rust: `src/sim/pathfinding/cell_entry.rs:317`. gamemd: `0x0051C4EB..0x0051C549` building/CanGarrison branch.
4. **Stage 7 - bridge/layer blocker FAIL.** Player-visible difference: edge cells on bridge/tube/layer boundaries can be classified using the wrong object list and occupancy bits. Rust: `src/sim/production/production_sell.rs:320`. gamemd: `0x0051BF90` calls vtable `+0x1B0` / `CheckBridgeTraversal @ 0x004D9C60`.
5. **Stage 8 - scatter filtering FAIL.** Player-visible difference: even after a correct exit cell, the scatter destination can move to a different adjacent cell or fail to move because the destination filter is the same approximate gate. Rust: `src/sim/production/production_sell.rs:405`. gamemd: `InfantryClass::Scatter @ 0x0051D0D0` destination scan plus native passability.

## Adjacent Findings

- Scatter mission queue side effects are adjacent; this run only traces whether destination filtering uses native `Can_Enter_Cell` semantics.
- Exact terminal infantry subcell-full return-code ladder remains unresolved by prior research, so occupancy-only blocker parity is `UNCHECKED` rather than `PASS`.
- Exact per-stock-map coordinates were not captured in a live replay; the failures above are branch/mechanism mismatches proven from active function code and Rust inputs.

## Status

COMPLETE
