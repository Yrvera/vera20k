# Ground Path CliffRamps Traversal Trace

**Date:** 2026-05-26  
**Trace slot:** `/trace-swarm` slot 2  
**Scenario:** On a non-lunar TEMPERATE fixture/map, order a Grizzly Tank across a `CliffRamps` tile that connects lower and upper terrain. Trace whether active YR `gamemd.exe` and Rust both allow the ramp cell as traversable instead of treating all cliff-like range tiles as blocked.  
**Scope:** One concrete player-visible movement/pathing scenario only. Adjacent bridge, waterfall, lunar, and ordinary cliff-face cases are out of scope.

## Verdict

Overall status: **COMPLETE** for the read-only trace. No Rust, INI, or published docs were edited. This report is the only file written.

Verdict tally: **PASS: 2 | FAIL: 0 | UNCHECKED: 5 | NOT-IMPLEMENTED: 0**

No FAIL or NOT-IMPLEMENTED finding was proven in this slot. The main residual risk is that Rust admits the canonical ramp cell, but the full active-YR A* path result for a concrete map cell sequence was not computed numerically in this run, so the end-to-end player-visible path decision remains UNCHECKED rather than PASS.

## Scenario Inputs

Concrete fixture values used for the trace:

- Theater: TEMPERATE, non-lunar, `ini/temperatmd.ini`.
- Unit: `[MTNK]` Grizzly Battle Tank from `ini/rulesmd.ini`; `MovementZone=Normal`, `Speed=7`, no explicit `SpeedType`, so Rust defaults the object to `SpeedType::Track` where no parsed value overrides it.
- Ramp tile class: TEMPERATE `[General] CliffRamps=25`.
- Resolved TEMPERATE tile-set starts from local `temperatmd.ini`: tileset 25 starts at tile id `384`, count `10`; tileset 26 starts at `394`, count `10`.
- Binary `CliffRamps` classifier range: `[CliffRamps, CliffRamps + 0x14)`, so the concrete range is `[384, 404)`.
- Concrete ramp cell chosen for computation: tile id `384`, with TMP terrain slope byte represented in Rust as `slope_type=2`, a canonical nonzero terrain ramp. Neighbor height delta for the intended traversal is one level.

## Pipeline

```text
Player move order
  -> unit movement data (MTNK MovementZone/SpeedType)
  -> theater numeric CliffRamps range load
  -> resolved terrain metadata for tile 384 and slope byte 2
  -> terrain cost grid for SpeedType::Track
  -> A* neighbor admission and height-diff ramp gate
  -> movement path/visible tank traversal
```

## Stage Results

### 1. Unit Movement Data

Rust evidence: `rulesmd.ini:6603` defines `[MTNK]`; lines `6618` and `6638` provide `Speed=7` and `MovementZone=Normal`. Rust command setup uses `SpeedType::Track` as the fallback for vehicles when no parsed speed type overrides it in `src/sim/world/world_commands.rs`.

gamemd evidence: `[MTNK]` is stock YR content. This slot did not decompile the full TechnoType parser and speed-type default chain for this exact unit.

Verdict: **UNCHECKED**. The player-visible unit choice is concrete, but literal Rust-vs-gamemd equality for the speed-type/default parse stage was not recomputed here.

### 2. TEMPERATE CliffRamps Numeric Range

gamemd evidence: `THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md` verifies `Read_Theater_TileSets_INI @ 0x00545150` reads `CliffRamps` as a tileset ordinal and stores the cumulative tile-id start. It also verifies `IsCliffOrImpassableTile @ 0x004863d0` uses half-open `CliffRamps + 0x14`. The report marks this active in loaded non-lunar theaters, including standard YR TEMPERATE.

Rust evidence: `src/map/theater.rs:914` resolves cliff ranges, `src/map/theater.rs:921` to `923` stores `cliff_ramps` from the tileset start, and `src/map/theater.rs:185` to `188` applies the `0x14` half-open range. Local `temperatmd.ini` parsing gives `CliffRamps=25 -> start 384`.

Computed output:

- gamemd: `CliffRamps start = 384`, range `[384,404)`.
- Rust: `cliff_ramps = Some(384)`, range `[384,404)`.

Verdict: **PASS**. The computed start and half-open range match numerically for this TEMPERATE case.

### 3. Broad Cliff/Impassable Classifier for Tile 384

gamemd evidence: `IsCliffOrImpassableTile @ 0x004863d0` returns true for `tile >= CliffRamps && tile < CliffRamps + 0x14`; active in standard non-lunar theater classification per the classification report.

Rust evidence: `src/map/theater.rs:185` to `188` includes `in_fixed_range(self.cliff_ramps, tile_id, 0x14)`.

Computed output:

- Input: `tile_id=384`, `slope_byte=2`.
- gamemd: `384 >= 384 && 384 < 404 -> true`.
- Rust: `in_fixed_range(Some(384), 384, 0x14) -> true`.

Verdict: **PASS**. Both classify this tile id as part of the broad cliff/impassable range.

### 4. Resolved Terrain Ramp Override

Rust evidence: `src/map/resolved_terrain.rs:1273` to `1291` marks a matching numeric cliff/ramp tile as `is_cliff_like`, `ground_blocked`, and `build_blocked`. `src/map/resolved_terrain.rs:419` computes `canonical_ramp`, and `src/map/resolved_terrain.rs:457` clears the base ground walk block when `canonical_ramp.is_some()`. The focused Rust test at `src/map/resolved_terrain.rs:2179` to `2271` checks that a canonical ramp remains ground-passable but non-buildable.

Computed Rust output for `tile_id=384`, `slope_type=2`:

- `is_cliff_like = true`.
- `canonical_ramp = Some(RampDirection::North)`.
- `ground_walk_blocked = false`.
- `build_blocked = true`.

gamemd evidence: `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md` verifies active terrain slope bytes and distinct cell bytes, and `THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md` verifies numeric `CliffRamps` classification. This run did not compute active `CellClass::RecalcAttributes` output for the exact TEMPERATE tile id/subtile/cell with the exact TMP slope byte.

Verdict: **UNCHECKED**. Rust’s intended ramp override was computed, but exact gamemd cell metadata equality for this concrete cell was not.

### 5. Track Terrain Cost for Ramp Cell

Rust evidence: `src/sim/pathfinding/terrain_cost.rs:57` to `68` makes `ramp_passable = cell.canonical_ramp.is_some()`, excludes canonical ramps from cliff hard-blocking, and returns `COST_NORMAL` for ramp cells. `src/sim/pathfinding/terrain_cost.rs:290` to `307` verifies a cliff-like rock-land canonical ramp yields `COST_NORMAL` for `SpeedType::Track`.

Computed Rust output:

- Input: `SpeedType::Track`, `is_cliff_like=true`, `canonical_ramp=Some(_)`, no bridge/overlay/terrain object blockers.
- `hard_blocked = (true && false) || false || false = false`.
- `cost_at(ramp) = 100`.

gamemd evidence: The active YR movement docs verify slope/ramp systems and `CliffRamps` range classification, but this slot did not compute the exact `Can_Enter_Cell`/terrain-speed result for the same Grizzly/ramp cell.

Verdict: **UNCHECKED**. Rust admits the cell in the cost grid; active gamemd numeric cost/admission for this exact movement step was not computed.

### 6. A* Neighbor Admission Across One-Level Ramp

Rust evidence: `src/sim/pathfinding/core.rs:932` to `985` gates non-bridge height changes: diff `0` is legal; diff `1` is legal only if the lower cell has nonzero `slope_type`; other diffs are blocked. `src/sim/pathfinding/core.rs:1008` to `1024` checks ground passability, then `src/sim/pathfinding/core.rs:1082` to `1094` rejects only terrain cost `0`. `src/sim/pathfinding/core.rs:1120` to `1123` applies a cliff-cost multiplier when effective path height changes.

Computed Rust output for lower-to-upper one-level ramp over slope byte `2`:

- `diff.abs() = 1`.
- `lower_slope = 2`.
- height gate legal: `true`.
- ground passability: allowed if the resolved ramp cell is the neighbor and no blockers are present.
- terrain cost: `100`, not rejected.
- step cost: `1000 * 4 + direction_tiebreak` because height changes.

gamemd evidence: `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md` verifies active slope bytes, slope cost docs, and cliff speed multipliers. The exact active YR A* neighbor admission and resulting path for this concrete start/ramp/goal sequence were not recomputed in this run.

Verdict: **UNCHECKED**. Rust’s one-step ramp admission is computed; gamemd’s matching step/path output is not.

### 7. Player-Visible Path Result

Rust result from code-level computation: a clear lower cell -> canonical `CliffRamps` tile id `384` with `slope_type=2` -> clear upper cell is not rejected by the numeric classifier, resolved terrain ground walkability, terrain cost grid, or the one-level height gate, assuming no dynamic entity/object/overlay blockers.

gamemd result: Not computed for a concrete active YR map cell path in this run. The existing research proves the relevant mechanisms are active, but does not supply a literal path list or final move-order result for this exact three-cell scenario.

Verdict: **UNCHECKED**. The trace did not produce the required computed equality for a PASS.

## Failures

None proven.

## Not Implemented

None proven for this scenario.

## Adjacent Findings

- `PathGrid::from_map_cells` still has a name-based legacy path in `src/sim/pathfinding/core.rs:1589` to `1609`. This trace did not prove it is used for the concrete current map-load pipeline, and it was not traced further.
- Build placement remains correctly blocked for canonical ramps in Rust (`build_blocked=true`), but building placement parity around ramps is outside this slot.
- Waterfalls, bridge-deck overrides, ordinary cliff faces, and lunar zeroing are covered by other swarm slots.

## Sources

- `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
- `docs/research/CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md`
- `ini/temperatmd.ini`
- `ini/rulesmd.ini`
- `src/map/theater.rs`
- `src/map/theater_tests.rs`
- `src/map/resolved_terrain.rs`
- `src/sim/pathfinding/terrain_cost.rs`
- `src/sim/pathfinding/core.rs`
