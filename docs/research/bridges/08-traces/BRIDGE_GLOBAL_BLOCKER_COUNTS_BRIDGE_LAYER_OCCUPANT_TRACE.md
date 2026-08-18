# Bridge Global Blocker Counts Bridge Layer Occupant Trace

**Scenario:** A flat hierarchy search near a high bridge/deck occupant where the deck occupant contributes to the global `CellClass+0x122` count even though the search is on the ground/object-list path.

**Scope:** One bridge-layer foot/entity source at cell `(2,2)` on a `5x5` map, one off-marker ground-search candidate adjacent to it. This trace compares gamemd writer/reader behavior to current Rust `build_blocker_neighbor_counts`, `BlockerNeighborCounts`, `HierarchyGate`, and current movement pathfinding plumbing.

**Status:** COMPLETE.

## Pipeline

1. Bridge/deck occupant enters or occupies source cell `(2,2)`.
2. gamemd writes global `CellClass+0x122` for the eight neighbor cells of `(2,2)`.
3. Rust builds `BlockerNeighborCounts` from all live object-list occupants, including bridge-list occupants.
4. Flat hierarchical A* considers an off-marker candidate adjacent to `(2,2)`.
5. Candidate is allowed when the candidate count is nonzero; zero is rejected only while hierarchy is enabled.
6. Production call sites must provide the count grid for the matching gate to run.

## Entry Points

`FootClass::Unlimbo @ 0x004D7170`: active in standard YR. Successful unit unlimbo increments the eight neighbor `CellClass+0x122` bytes of the current source cell. This applies regardless of whether `CellClass::AddContent` later inserts the object into the ground list or bridge list.

`FootClass::PerCellProcess @ 0x004D85D0`: active in standard YR. On cell change, it decrements the old source cell's eight-neighbor contribution, stores the current cell, then increments the new source cell's eight-neighbor contribution.

`CellClass::AddContent @ 0x0047E8A0` / `RemoveContent @ 0x0047EA90`: active in standard YR. These select `FirstObject` versus `AltObject` from the `OnBridge` argument, but they do not select a separate `+0x122` counter. This confirms bridge object lists are separate while the blocker-neighbor byte is global.

Rust `movement_tick` path: `src/sim/movement/movement_tick.rs:410` builds `BlockerNeighborCounts` from entities and terrain when a path grid is available, then passes it through `PathfindingContext` at `src/sim/movement/movement_tick.rs:420`.

Rust fresh/queued movement command path: `src/sim/movement/movement_commands.rs:249` and `src/sim/movement/movement_commands.rs:296` construct `PathfindingContext` with `zone_grid: None` and `blocker_neighbor_counts: None`, so fresh command pathfinding cannot run the hierarchy marker/count gate on that entry point.

## Stage Results

| Stage | gamemd concrete output | Rust concrete output | Verdict |
|---|---|---|---|
| 1. Bridge-list occupant as count source | A live foot object at `(2,2)` on the bridge list still executes foot lifecycle writers against global `CellClass+0x122`. Active in YR: yes; no TS-only gate found. | `GameEntity::occupancy_list_layer` returns `Some(Bridge)` when `on_bridge=true`, not `None`, so the builder includes it. | PASS |
| 2. Neighbor count values | Source `(2,2)` increments its 8 neighbors only. In this fixture: `count(1,2)=1`, `count(3,3)=1`, `count(2,2)=0`. | `blocker_neighbor_counts_include_bridge_layer_occupants_globally` asserts exactly `1`, `1`, `0` for those cells after `build_blocker_neighbor_counts`. | PASS |
| 3. Writer shape | Single-cell foot sources use eight-neighbor increments/decrements; source cell itself is not incremented. | `BlockerNeighborCounts::add_single_cell_neighbor_source` loops `dx,dy=-1..=1`, skips `(0,0)`, and increments in-bounds neighbors. | PASS |
| 4. Counter scope | `CellClass+0x122` is one global byte per cell. Object-list layer selection uses `+0xE4/+0xE8`, not a second `+0x122`. | `build_blocker_neighbor_counts` does not filter to ground; it skips only passengers and entities with no occupancy list. Bridge occupants are included. | PASS |
| 5. A* off-marker read | In hierarchical A*, off-marker normal/near-height candidate with `+0x122 != 0` continues; `+0x122 == 0 && hierarchy_flag != 0` skips. Active in YR: yes; `AStar_main_loop @ 0x00429A90` was spot-checked read-only. | `HierarchyGate::allows` returns `marked_level0.contains(zone) || count_at(x,y) != 0`. For an off-marker adjacent candidate with count `1`, output is `true`; with count `0`, output is `false`. | PASS |
| 6. Tick/repath production plumbing | Standard YR pathfinding has the global byte available as maintained `CellClass` state when hierarchical A* runs. | `movement_tick.rs:410` builds counts and `movement_path.rs:293` passes them to `find_path_zoned_marker`; `zone_search.rs:278` enables marker hierarchy only when counts are `Some`. | PASS |
| 7. Fresh command production plumbing | A player-issued long-range move can enter gamemd hierarchical A* with live global `+0x122` counts. | Fresh and queued move command path construction passes `zone_grid: None` and `blocker_neighbor_counts: None` at `movement_commands.rs:249` and `movement_commands.rs:296`; this entry point cannot exercise the matching `HierarchyGate` behavior. | FAIL |

## Computed Fixture

Source object: vehicle-like foot/entity at `(2,2)`, `on_bridge=true`, not inside transport, object-list layer `Bridge`.

gamemd count result for source `(2,2)`: the eight neighboring cells get `+1`; the source cell remains unchanged by its own writer. Therefore `(1,2)=1`, `(3,3)=1`, `(2,2)=0`.

Rust count result from the local test fixture: `counts.count_at(1,2)==1`, `counts.count_at(3,3)==1`, `counts.count_at(2,2)==0`.

Hierarchy gate result: for an off-marker candidate at `(1,2)` or `(3,3)`, both engines allow expansion because the candidate count is nonzero. For the source cell `(2,2)` as an off-marker candidate, both engines reject on the count exception alone because the count is zero.

## Player-Visible Findings

### FAIL-1 - Fresh move commands do not supply the count surface

Player-visible problem: on the initial move-order path, Rust can skip the new binary-style marker/count gate entirely instead of running flat hierarchical A* with the bridge occupant's global blocker-neighbor count. In cases where hierarchy route selection is intended to happen immediately on command issue, this can change early detours or force fallback behavior.

Rust evidence: `src/sim/movement/movement_commands.rs:249` and `src/sim/movement/movement_commands.rs:296` set `blocker_neighbor_counts: None`; `src/sim/pathfinding/zone_search.rs:278` only activates hierarchy-marker search when counts are present.

gamemd evidence: `AStar_main_loop @ 0x00429A90` reads `CellClass+0x122` in the active standard YR hierarchical A* chain; `FootClass` writers maintain the global byte independent of bridge object-list layer.

## Adjacent Findings

Aircraft descent also writes the same counter conditionally, but aircraft landing/descent is outside this bridge-deck occupant ground-search scenario.

Building sources use an expanded foundation rectangle instead of eight-neighbor-per-cell writes; this is covered by a separate local test and is not part of this single bridge-layer occupant fixture.

## Verdict Tally

PASS: 6 | FAIL: 1 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

## Sources

- Read-only Ghidra spot checks: `AStar_main_loop @ 0x00429A90`, `FootClass::Unlimbo @ 0x004D7170`, `FootClass::PerCellProcess @ 0x004D85D0`, `CellClass::AddContent @ 0x0047E8A0`, `CellClass::RemoveContent @ 0x0047EA90`.
- `docs/research/CELL_0X122_DYNAMIC_BLOCKER_LIFECYCLE_RUST_MAPPING_GHIDRA_REPORT.md`
- `docs/research/CELL_0X122_WRITER_TIMING_FLAT_ASTAR_GHIDRA_REPORT.md`
- `docs/research/ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`
- `src/sim/movement/bump_crush.rs`
- `src/sim/pathfinding/core.rs`
- `src/sim/pathfinding/zone_search.rs`
- `src/sim/movement/movement_path.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_commands.rs`
