# WaterfallEast Edge Slope 0 vs Middle Trace

**Scenario:** On a non-lunar theater with active waterfall ranges, compare a Grizzly Tank ordered across `WaterfallEast + 0` with slope byte `0` versus `WaterfallEast + 1`.

**Scope:** One concrete passability classification boundary: East waterfall edge tile with passable slope byte versus adjacent middle tile.

**Status:** PARTIAL. The numeric gamemd/Rust theater classifier was computed and matches. Full end-to-end Grizzly path acceptance/rejection remains UNCHECKED because this run did not compute both active gamemd A* output and Rust A* output on an instantiated map path.

## Scenario Values

Using `ini/temperatmd.ini` as the non-lunar active-theater fixture:

- `WaterfallEast = 49`
- Cumulative tile-id start for tileset ordinal `49` = `539`
- `WaterfallEast + 0` = tile id `539`
- `WaterfallEast + 1` = tile id `540`
- Grizzly Tank rules object = `[MTNK]`, `Name=Grizzly Battle Tank`, `MovementZone=Normal`, no explicit `SpeedType`, so Rust defaults the unit speed type to `Track`.

## Pipeline

Player move order -> unit movement rules resolve Grizzly as normal tracked ground mover -> map cell metadata supplies tile id and slope byte -> waterfall numeric classifier marks cell cliff/impassable or not -> resolved terrain and path grid expose ground walkability -> A* accepts or rejects entering the candidate cell -> unit movement either enters the cell or routes/halts.

## Computed Stage Results

### Stage 1 - Theater Load Numeric Start

- gamemd: `Read_Theater_TileSets_INI @ 0x00545150` reads `WaterfallEast` as a tileset ordinal, then writes the current cumulative tile-id cursor to `DAT_00aa073c` when the loop reaches that ordinal.
- Rust: `resolve_cliff_ranges` resolves `WaterfallEast` through `resolve_tileset_start` and stores it in `TheaterCliffRanges.waterfall_east`.
- Computed value: `49 -> 539` for `temperatmd.ini`.
- Verdict: PASS.

Evidence:

- `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
- `src/map/theater.rs:914`
- `src/map/theater.rs:931`

### Stage 2 - WaterfallEast + 0, Slope Byte 0

- Input: tile id `539`, slope byte `0`.
- gamemd: `IsCliffOrImpassableTile @ 0x004863d0` checks `DAT_00aa073c <= tile < DAT_00aa073c + 4`; for East edge tiles `base` and `base + 3`, slope bytes `0` and `4` return not impassable.
- gamemd output: `0` / not cliff-or-impassable.
- Rust: `waterfall_blocks(Some(539), 539, 0, &[0, 4])` has offset `0`; `passable.contains(0)` is true; returns `false`.
- Rust output: `false` / not cliff-or-impassable.
- Verdict: PASS.

Evidence:

- Ghidra read-only decompile of `0x004863d0`
- `src/map/theater.rs:185`
- `src/map/theater.rs:193`
- `src/map/theater.rs:218`

### Stage 3 - WaterfallEast + 1 Middle Tile

- Input: tile id `540`, slope byte not material to the East middle-tile branch.
- gamemd: same helper checks tile in `[base, base + 4)`; because tile is neither `base` nor `base + 3`, it returns impassable.
- gamemd output: `1` / cliff-or-impassable.
- Rust: `waterfall_blocks(Some(539), 540, slope, &[0, 4])` has offset `1`; non-edge offset returns `true`.
- Rust output: `true` / cliff-or-impassable.
- Verdict: PASS.

Evidence:

- Ghidra read-only decompile of `0x004863d0`
- `src/map/theater.rs:193`
- `src/map/theater.rs:225`
- `src/map/theater.rs:228`

### Stage 4 - Rust Ground Cost Consequence

- Input from classifier: East+0 is not `is_cliff_like` by numeric waterfall classification; East+1 is `is_cliff_like`.
- Rust middle tile consequence: `apply_theater_cliff_ranges` sets `is_cliff_like`, `ground_blocked`, and `build_blocked` true for East+1; `TerrainCostGrid::from_resolved_terrain` hard-blocks a non-ramp cliff-like cell with `COST_BLOCKED = 0`; pathfinding rejects non-passable neighbors.
- Rust edge tile consequence: numeric waterfall classification does not set cliff blocking for East+0/slope 0. Final movement acceptance still depends on TMP land byte, water/name metadata, bridge state, overlay blockers, terrain-object blockers, height legality, and the cost grid.
- Verdict: UNCHECKED for full edge-tile Grizzly movement, PASS only for the middle tile's classifier-to-blocked propagation shape.

Evidence:

- `src/map/resolved_terrain.rs:1273`
- `src/map/resolved_terrain.rs:1278`
- `src/map/resolved_terrain.rs:1281`
- `src/sim/pathfinding/terrain_cost.rs:57`
- `src/sim/pathfinding/terrain_cost.rs:65`
- `src/sim/pathfinding/core.rs:1017`
- `src/sim/pathfinding/core.rs:1025`

### Stage 5 - Active YR Grizzly Movement Outcome

- gamemd classifier result: East+0/slope 0 is not impassable by `IsCliffOrImpassableTile`; East+1 is impassable.
- Full gamemd movement result: UNCHECKED. This run did not compute active YR pathfinder entry/route output for a concrete map path containing those two cells.
- Full Rust movement result: UNCHECKED. This run did not instantiate the same map/path in Rust and compute the returned path.
- Verdict: UNCHECKED.

## Findings

No player-visible FAIL or NOT-IMPLEMENTED finding is proven in this slot.

The classifier parity target for this scenario is met:

- `WaterfallEast + 0`, slope byte `0`: gamemd `0`, Rust `false`, both allow the tile to escape the broad cliff/impassable classifier.
- `WaterfallEast + 1`: gamemd `1`, Rust `true`, both classify the middle waterfall tile as cliff/impassable.

The player-visible movement outcome remains UNCHECKED until a follow-up trace computes both engines' actual path result on the same concrete cells. This matters because Rust still has additional post-classifier gates for water/name metadata, TMP land type, overlays, bridge state, height legality, terrain costs, and path-grid walkability.

## Adjacent Findings

- The supplied research doc states direct pathfinding callers for `IsCliffOrImpassableTile @ 0x004863d0` were not found in that investigation slice, and actual runtime movement primarily consumes downstream cell land/slope/height bytes. This trace therefore does not claim the helper alone proves gamemd movement behavior.
- `IsOnBridgeRamp @ 0x00578d80` uses the same East waterfall edge/middle exception but is bridge-application specific and not part of this concrete ground-move trace.

## Verdict Tally

PASS: 3 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Sources

- `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
- Ghidra read-only decompile: `Read_Theater_TileSets_INI @ 0x00545150`
- Ghidra read-only decompile: `IsCliffOrImpassableTile @ 0x004863d0`
- Ghidra read-only decompile: `IsOnBridgeRamp @ 0x00578d80`
- `ini/temperatmd.ini`
- `ini/rulesmd.ini`
- `src/map/theater.rs`
- `src/map/resolved_terrain.rs`
- `src/sim/pathfinding/terrain_cost.rs`
- `src/sim/pathfinding/core.rs`
