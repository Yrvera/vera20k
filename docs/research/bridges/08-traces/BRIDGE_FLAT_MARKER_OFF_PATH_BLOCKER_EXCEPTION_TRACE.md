# Bridge Flat Marker Off-Path Blocker Exception Trace

**Scenario:** Flat ground hierarchy marker A* where level-0 `Zone_precheck` has marked zones `[1, 3]`, the middle candidate cell is in unmarked zone `2`, and the `CellClass+0x122` equivalent blocker-neighbor count decides whether cell A* may enter that off-marker cell.

**Status:** COMPLETE for the scoped marker/count branch comparison. Downstream live movement/render result is UNCHECKED because this trace did not run a live gamemd route or Rust executable; the hard constraints allowed only this report file write.

**Verdict tally:** PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Pipeline

`Zone_precheck level-0 marker output -> AStar_main_loop neighbor zone lookup -> hierarchy marker miss -> blocker-neighbor exception check -> candidate accepted/skipped -> path availability`

## Stage Table

| Stage | Boundary checked | gamemd output | Rust output | Verdict |
|---|---|---:|---:|---|
| 1 | Level-0 marker input for the concrete scenario | marked zones `[1, 3]` | `BTreeSet::from([1, 3])` in the scoped tests | PASS |
| 2 | Middle candidate level-0 zone | zone `2`, not marked | `row_level0_graph(&[1, 2, 3])`, cell `(1,0)` -> zone `2`, not marked | PASS |
| 3 | Off-marker candidate with count zero and hierarchy enabled | skip neighbor | `HierarchyGate::allows(1,0)` -> false, A* `continue` | PASS |
| 4 | Off-marker candidate with count nonzero and hierarchy enabled | accept candidate into normal A* path | `set_count(1,0,1)` makes `HierarchyGate::allows(1,0)` -> true | PASS |
| 5 | Concrete 3-cell path branch consequence | zero count blocks the only middle step; nonzero count permits the middle step | zero-count fixture returns `None`; count-one fixture returns `[(0,0),(1,0),(2,0)]` | PASS |
| 6 | Player-visible unit movement after the path result | not runtime-measured in gamemd | not executed in Rust this run | UNCHECKED |

## Evidence

### gamemd

`AStar_main_loop @ 0x00429A90` is active in standard YR. Existing reports and this read-only Ghidra spot-check confirm the live chain `FootClass::Run_AStar -> AStar_pathfind_search -> Zone_precheck/AStar_main_loop`, with no TS-only gate for this slice.

For each neighbor, gamemd reads the candidate's level-0 zone and checks the chosen marker array:

- `0x00429E85`: calls `ZoneMap__CellToZoneIndex`.
- `0x00429E8A..0x00429E9A`: reads the first 16-bit zone id from the cell-zone tuple.
- `0x00429EA4..0x00429EA7`: compares the level-0 marker entry with the current epoch; marked candidates enter the normal path.

For unmarked normal/near-height candidates, the blocker-neighbor exception is immediate:

- `0x00429EB1`: reads `byte [neighbor_cell + 0x122]`.
- `0x00429EB7..0x00429EB9`: nonzero jumps to the accepted normal path at `0x00429EC7`.
- `0x00429EBB..0x00429EC1`: zero plus hierarchy flag set skips the neighbor at `0x0042A1A1`.

`CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md` identifies `+0x122` as a per-cell 8-neighbor blocker refcount. The reader is in `AStar_main_loop`, not `Can_Enter_Cell`. Writer paths cover blockers such as walls, buildings, units, terrain objects, and landing aircraft. Active in YR: Yes.

The concrete branch truth table for this trace is therefore:

| marked? | count | hierarchy flag | gamemd decision |
|---:|---:|---:|---|
| false | `0` | true | skip |
| false | nonzero | true | accept |
| true | any | true | accept |

### Rust

Relevant current Rust surfaces:

- `src/sim/pathfinding/core.rs:237`: `BlockerNeighborCounts::count_at` returns the per-cell `u8` count, or `0` out of bounds.
- `src/sim/pathfinding/core.rs:290`: `HierarchyGate::allows` returns `marked_level0.contains(&zone) || blocker_neighbor_counts.count_at(x, y) != 0`.
- `src/sim/pathfinding/core.rs:983`: `astar_search` applies the gate before continuing to later corridor/cost work.
- `src/sim/pathfinding/core_tests.rs:2312`: zero-count fixture uses zones `[1,2,3]`, markers `[1,3]`, no blocker count, and asserts no path.
- `src/sim/pathfinding/core_tests.rs:2340`: count-one fixture sets `(1,0)` to `1` and asserts path `[(0,0),(1,0),(2,0)]`.
- `src/sim/pathfinding/zone_search.rs:278`: production flat hierarchy path only enables the marker gate when blocker-neighbor counts are available.
- `src/sim/movement/movement_tick.rs:410`: movement tick builds blocker-neighbor counts from the current path grid/entity/terrain surface before pathfinding.
- `src/sim/movement/bump_crush.rs:229`: `build_blocker_neighbor_counts` builds the current Rust count grid.

The Rust branch truth table for the concrete fixture is:

| cell | zone | marked? | count | `HierarchyGate::allows` | A* result |
|---|---:|---:|---:|---:|---|
| `(1,0)` | `2` | false | `0` | false | neighbor skipped |
| `(1,0)` | `2` | false | `1` | true | neighbor eligible |

This matches the gamemd marker/`+0x122` polarity for the scoped scenario.

## Player-Visible Impact

No FAIL or NOT-IMPLEMENTED finding was found for the scoped branch. In this synthetic 3-cell flat route, the player-visible consequence would be whether a unit can route through the middle off-marker cell: zero blocker-neighbor count should prevent that hierarchy A* attempt from entering the middle cell, while nonzero count should permit it.

The actual on-screen unit movement, timing, path smoothing, and render result remain UNCHECKED because this trace did not run a live gamemd/Rust scenario. The branch decision itself is the scoped mechanic requested.

## Adjacent Findings

- Production `BlockerNeighborCounts` writer parity is broader than this trace. Rust currently derives counts from terrain blockers and entities, with tests for bridge-layer occupants and expanded structure foundations, but this trace did not re-audit every gamemd `+0x122` writer timing path.
- Tube direction `8`, high-bridge layered semantics, retry-edge production, path smoothing, and exact stock-map route output are adjacent and intentionally not traced here.
- Existing tests were not re-run in this slot because cargo test/build activity can write outside the single allowed report file.

## Sources

- Read-only Ghidra decompile/disassembly: `AStar_main_loop @ 0x00429A90`.
- Read-only Ghidra decompile: `AStar_pathfind_search @ 0x0042C900`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`.
- Rust read-only scan of `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/core_tests.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/movement/bump_crush.rs`, and `src/sim/movement/movement_tick.rs`.
