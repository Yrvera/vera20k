# Ground Path CliffSet Face Detour Trace

**Scenario:** On a non-lunar TEMPERATE fixture, order a Grizzly Tank (`MTNK`) from clear ground on one side of an ordinary `CliffSet` face tile to clear ground on the other side, with a clear detour around the cliff.

**Concrete fixture used for Rust computation:** 5x3 synthetic resolved-grid shape, start `(0,1)`, goal `(4,1)`, ordinary `CliffSet` face at `(2,1)`, all other cells clear ground, no overlays, no terrain objects, no bridge deck.

**Scope:** One movement/pathfinding mechanic only: ground pathing rejects entry into the `CliffSet` face cell and routes around it.

**Write constraints honored:** No Rust, INI, or published research docs were edited. Ghidra was not mutated. This report is the only file written for this slot.

## Verdict

**PARTIAL.** Rust-side classifier, terrain-cost, and A* blocking values were computed for the concrete fixture. Active YR `gamemd.exe` evidence proves the numeric `CliffSet` classifier and live A* spine, but this run did not execute/compute the full gamemd route queue for the same synthetic fixture. Therefore the final "chooses the same detour path" stage is **UNCHECKED**, not PASS.

Verdict tally: **PASS: 3 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0**

## Pipeline

`Player move order -> MTNK movement data -> TEMPERATE theater numeric tile classification -> resolved terrain cell flags -> PathGrid/TerrainCostGrid -> A* neighbor expansion -> visible unit route around cliff`

## Stage Results

### Stage 1 - Move Trigger And Unit Data

- Input: player orders `MTNK` from `(0,1)` to `(4,1)`.
- Rules data: `MTNK` in `ini/rules.ini` is Grizzly Battle Tank, `Speed=7`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`.
- Rust route entry: `find_path_with_costs` starts ground-layer A* in `src/sim/pathfinding/core.rs:1991`.
- gamemd evidence: active YR pathfinding spine reaches `FootClass::Run_AStar @ 0x004CBBA0 -> AStar_pathfind_search @ 0x0042C900 -> AStar_main_loop @ 0x00429A90` per `docs/research/ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md` and `docs/research/ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`.
- Verdict: **UNCHECKED**. The active spine is verified, but this run did not compute gamemd's full order-to-route output for the exact fixture.

### Stage 2 - TEMPERATE CliffSet Numeric Classification

- Input: TEMPERATE `CliffSet=10` from `ini/temperatmd.ini`; `TileSet0010` has cumulative tile-id start `49` and `TilesInSet=40`.
- Concrete cliff face cell: tile id `49`, slope byte `0`.
- Rust formula: `TheaterCliffRanges::is_cliff_or_impassable_tile(49, 0)` checks `tile_id >= 49 && tile_id < 49 + 0x28`; output `true` in `src/map/theater.rs:185`.
- gamemd formula: `IsCliffOrImpassableTile @ 0x004863d0` checks `DAT_00aa1020 != -1 && tile >= CliffSet && tile < CliffSet + 0x28`; output `true` for tile `49` with TEMPERATE `CliffSet=49`.
- Active YR: `THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md` marks the non-lunar theater numeric classifier live in loaded YR theaters.
- Verdict: **PASS**. Computed Rust `true` equals computed gamemd `true`.

### Stage 3 - Resolved Terrain Application

- Input: cell `(2,1)`, tile id `49`, slope byte `0`, no overlay, no terrain object, no canonical ramp.
- Rust formula: `apply_theater_cliff_ranges` sees classifier `true` and writes `is_cliff_like=true`, `ground_blocked=true`, `build_blocked=true`; if not water, it sets `TerrainClass::Cliff` in `src/map/resolved_terrain.rs:1273`.
- Downstream resolved value: `base_ground_walk_blocked = canonical_ramp.is_none() && metadata.ground_blocked = true` at `src/map/resolved_terrain.rs:457`; `ground_walk_blocked=true` at `src/map/resolved_terrain.rs:488`.
- gamemd evidence: classifier output `AL=1` means "classified as cliff/impassable" in the verified helper; full `CellClass::RecalcAttributes` land-byte derivation is explicitly non-scope in the theater report.
- Verdict: **UNCHECKED**. Rust writes the expected blocked flags, but exact gamemd `CellClass` byte writes for this concrete fixture were not computed in this run.

### Stage 4 - Terrain Cost And Entry Rejection

- Input: resolved cliff face cell `(2,1)` with `is_cliff_like=true`, `canonical_ramp=None`, no bridge deck, no overlay block, no terrain object block.
- Rust PathGrid: `PathGrid::from_resolved_terrain_with_bridges` sets `ground_walkable=false` when `cell.is_cliff_like` is true at `src/sim/pathfinding/core.rs:1688`.
- Rust cost grid: for `SpeedType::Track`, `hard_blocked = (true && !false) || false || false = true`; `cost_at(2,1)=0` in `src/sim/pathfinding/terrain_cost.rs:57`.
- Rust A*: neighbor expansion rejects the cell because `is_cell_passable_for_mover`/layer walkability fails, and also rejects any ground neighbor with `terrain_cost == 0` at `src/sim/pathfinding/core.rs:1008` and `src/sim/pathfinding/core.rs:1092`.
- gamemd evidence: `IsCliffOrImpassableTile` returns impassable for `CliffSet` face; active A* calls `Can_Enter_Cell` during neighbor expansion before edge cost in `AStar_main_loop @ 0x00429F37..0x00429FEA`.
- Verdict: **PASS**. For the cliff-face entry decision, computed Rust reject equals computed gamemd impassable classifier result.

### Stage 5 - Detour Route Shape

- Rust synthetic route calculation using the current A* constants and neighbor order:
  - `STEP_COST=1000`, direction order `N, NE, E, SE, S, SW, W, NW`, direction tie-break `[1,5,2,6,3,7,4,8]`.
  - With `(2,1)` blocked, one computed Rust detour is `(0,1) -> (1,1) -> (2,0) -> (3,1) -> (4,1)`, total `g=4015`.
  - The path does not include `(2,1)`.
- gamemd evidence: A* has the same live standard spine, uses 8 compass directions, calls `Can_Enter_Cell` on the neighbor, uses uniform base edge cost with direction epsilon, and does not validate diagonal flanking cells for normal ground edges per existing pathfinding research and Rust comments linked to those reports.
- Missing computation: the actual gamemd route queue for this exact fixture was not executed or numerically reconstructed from binary heap/tie behavior.
- Verdict: **PASS for "does not include the cliff face"; UNCHECKED for "chooses the exact same detour route".**

## Failures

None found in this trace.

## Not Implemented

None found for this narrow scenario.

## Adjacent Findings

- The exact `CellClass::RecalcAttributes` byte writes behind the broad impassable classifier remain outside the supplied theater report's scope.
- Full route queue equality needs a gamemd-side computed fixture/oracle, not just classifier and A* spine evidence.
- The pathfinder's closed-list tolerance/reopen nuance is known from `ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`; this scenario did not exercise the blocked-goal fallback case.

## Evidence

- `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
- `docs/research/ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`
- `docs/research/ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`
- `ini/temperatmd.ini`
- `ini/rules.ini`
- `src/map/theater.rs`
- `src/map/resolved_terrain.rs`
- `src/sim/pathfinding/terrain_cost.rs`
- `src/sim/pathfinding/core.rs`
