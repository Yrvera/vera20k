# Lunar Zero-Count Cliff Ranges Walkable Trace

**Scenario:** On a LUNAR map/fixture, order a Grizzly Tank (`MTNK`) across ordinary lunar terrain at tile id `49`, the first ordinary lunar dirt tile vulnerable to false blocking if the 0-count `CliffSet` range is not zeroed.

**Verdict:** No FAIL found in the verified numeric-classification portion. Full Grizzly movement/path equality remains **UNCHECKED** because no active `gamemd.exe` path result was computed for the order.

## Concrete Setup

- Theater: `LUNAR`, `ini/lunarmd.ini`.
- Unit: `MTNK` / Grizzly Battle Tank, `MovementZone=Normal` in `ini/rulesmd.ini:6603-6638`.
- Vulnerable cell: tile id `49`, set ordinal `13`, `[TileSet0013] SetName=LAT Moon Dirt Dark`, `TilesInSet=1`.
- Why tile `49`: `CliffSet=10` maps to cumulative start `49` because `[TileSet0010]` is 0-count; without the lunar zeroing branch, the `CliffSet` fixed range `[49, 89)` would classify tile `49` as cliff/impassable.

## Pipeline

`LUNAR theater load -> cumulative tileset starts -> lunar zeroing -> IsCliffOrImpassableTile predicate -> resolved terrain flags -> terrain cost / PathGrid -> Grizzly order path`

## Stage Table

### Stage 1 - Scenario Data And Vulnerable Tile

- Rust/INI input: `lunarmd.ini [General] CliffSet=10`; tile set 10 has `TilesInSet=0`; cumulative start for set 10 is `49`.
- Concrete tile: tile id `49`, slope byte treated as `0` for the classifier check.
- Computed value: unzeroed `CliffSet` range would be `[49, 89)`, so tile `49` would be caught.
- gamemd evidence: `Read_Theater_TileSets_INI @ 0x00545150` reads `CliffSet` as a tileset ordinal and stores the current cumulative tile id into `DAT_00aa1020`.
- Rust evidence: `resolve_cliff_ranges` maps ordinals through `resolve_tileset_start` in `src/map/theater.rs:914-941`; the regression test proves ordinal-to-start mapping and the lunar 0-count hazard at `src/map/theater_tests.rs:250-279`.
- Verdict: **PASS** for the vulnerability setup: both systems use ordinal-to-cumulative-start semantics before lunar zeroing.

### Stage 2 - Active YR Lunar Zeroing

- gamemd output: when `local_95c == 5` (LUNAR theater), the active loader clears `DAT_00aa1020` (`CliffSet`), `DAT_00aa101c` (`WaterCliffs`), `DAT_00aa0e28` (`BridgeSet`), and `DAT_00abad1c` (`WoodBridgeSet`) to `-1` before return.
- Rust output: `apply_lunar_cliff_zeroing("LUNAR", ...)` clears `bridge_set`, `wood_bridge_set`, and all `TheaterCliffRanges` to default `None` in `src/map/theater.rs:944-956`.
- Concrete comparison for this scenario: gamemd `CliffSet=-1`; Rust `cliff_set=None`. Both make tile `49` fail the cliff range gate.
- Active YR check: the branch is inside the normal theater INI loader for theater id `5`; this is not a dormant TS-only helper.
- Verdict: **PASS** for the specific `CliffSet` zeroing needed by tile `49`.

### Stage 3 - Broad Cliff/Impassable Predicate

- gamemd formula: `IsCliffOrImpassableTile @ 0x004863d0` returns true for `CliffSet` only when `DAT_00aa1020 != -1 && tile >= CliffSet && tile < CliffSet + 0x28`.
- gamemd concrete output: after lunar zeroing, `DAT_00aa1020 == -1`, so tile `49` returns false through the `CliffSet` arm. The other lunar zeroed water/bridge arms relevant to this concrete tile also cannot make tile `49` true.
- Rust formula: `TheaterCliffRanges::is_cliff_or_impassable_tile` checks `Option<u16>` starts with half-open ranges in `src/map/theater.rs:183-216`.
- Rust concrete output: after lunar zeroing, `TheaterCliffRanges::default().is_cliff_or_impassable_tile(49, 0) == false`; the existing test checks the same defaulted predicate shape at `src/map/theater_tests.rs:274-279`.
- Verdict: **PASS** for numeric cliff/impassable classification of tile `49`.

### Stage 4 - Resolved Terrain Non-Blocking / Build Classification

- Rust path: `ResolvedTerrainGrid::build` loads metadata, then applies theater cliff ranges at `src/map/resolved_terrain.rs:1180-1222`; if the numeric predicate is false, `apply_theater_cliff_ranges` returns without setting `is_cliff_like`, `ground_blocked`, or `build_blocked` at `src/map/resolved_terrain.rs:1273-1290`.
- Rust concrete expectation for tile `49`: no numeric cliff forced block. With ordinary lunar dirt metadata and no overlay/object, `base_ground_walk_blocked=false`, `ground_walk_blocked=false`, and `build_blocked` follows normal land metadata in `src/map/resolved_terrain.rs:457-491` and `src/map/resolved_terrain.rs:520-562`.
- gamemd status: the trace verified the numeric classifier does not force cliff/impassable. It did not compute the full active `CellClass::RecalcAttributes` land/build result for this exact TMP sub-tile.
- Verdict: **UNCHECKED** for full build classification equality. The numeric range contribution is PASS, but the final CellClass land/build bytes were not computed in both engines.

### Stage 5 - Terrain Cost / PathGrid Entry

- Rust path: `TerrainCostGrid::from_resolved_terrain` hard-blocks `cell.is_cliff_like && !ramp_passable` at `src/sim/pathfinding/terrain_cost.rs:52-89`; `PathGrid::from_resolved_terrain_with_bridges` blocks cliff-like cells at `src/sim/pathfinding/core.rs:1638-1692`.
- Rust concrete expectation: because tile `49` is not cliff-like from the lunar numeric ranges, the numeric-range fix does not set cost `0` or `ground_walkable=false` for this cell.
- gamemd status: no active `Can_Enter_Cell` / A* computation was run for this exact Grizzly order.
- Verdict: **UNCHECKED** for path-grid equality. Rust is not falsely blocked by the numeric range; full gamemd path gate equality was not computed.

### Stage 6 - Player-Visible Grizzly Order

- Rust path entry points: `find_path` and `find_path_with_costs` route through A* in `src/sim/pathfinding/core.rs:1970-2005`.
- Player-visible expectation: a Grizzly ordered across a clear lunar strip containing tile `49` should be able to route through that cell instead of treating it as a cliff face.
- gamemd status: no live or replayed active YR path output was captured for the same start, goal, map cells, unit state, and blocker state.
- Verdict: **UNCHECKED**. A PASS would require literal equality of the computed path or movement decision in gamemd and Rust.

## Findings

No FAIL or NOT-IMPLEMENTED findings were found for this concrete trace.

The important verified parity result is narrow: active YR clears the lunar `CliffSet` global before the broad classifier can catch ordinary lunar tile id `49`, and Rust now clears the corresponding numeric ranges before resolved terrain/pathing consume them.

## Adjacent Findings

- Active gamemd visibly zeros `CliffSet`, `WaterCliffs`, `BridgeSet`, and `WoodBridgeSet` in the lunar branch; the decompiled slice did not show every cliff-adjacent global being zeroed. This trace did not expand into `CliffRamps`, waterfall, destroyable cliff, or water cave lunar edge cases.
- Full CellClass land-byte derivation and search-time `Can_Enter_Cell` remain outside this slot.

## Evidence

- Ghidra read-only decompile: `Read_Theater_TileSets_INI @ 0x00545150`.
- Ghidra read-only decompile: `IsCliffOrImpassableTile @ 0x004863d0`.
- Research: `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`.
- INI: `ini/lunarmd.ini`, `ini/rulesmd.ini`.
- Rust: `src/map/theater.rs`, `src/map/theater_tests.rs`, `src/map/resolved_terrain.rs`, `src/sim/pathfinding/terrain_cost.rs`, `src/sim/pathfinding/core.rs`.
- Local verification run: `cargo test -q lunar_theater_zeroing_clears_numeric_cliff_and_bridge_ranges` passed.

## Verdict Tally

PASS: 3 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0
