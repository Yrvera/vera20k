# MCV Cliff Ramp Low-to-High Traversal Trace

**Date:** 2026-05-27
**Trace slot:** /trace-swarm slot 3
**Scenario:** Allied AMCV at cell (50, 50), facing East (0x40), moves east across a
cliff-ramp-up cell at (51, 50) onto high ground at (52..55, 50), stopping at (55, 50).
Concrete Theater: TEMPERATE. One cell height difference. Uphill direction.
**Scope guard:** Terrain cliff ramp only — NOT bridge ramp. Deploy, turret, build,
and waterfall cases are out of scope.

## Verdict Tally

PASS: 5 | FAIL: 3 | UNCHECKED: 5 | NOT-IMPLEMENTED: 0

## Pipeline

```
Right-click (55,50) -> A* pathfind (ramp cost gate, height-diff gate)
  -> Movement tick (speed modifier on ramp cell)
  -> Cell transition (position.z update)
  -> Rocking/slope tracker (prev/curr slope, 3-tick transition)
  -> VXL render (slope SLERP matrix)
  -> Final occupancy at (55,50) high ground
```

## Stage Results

### 1. AMCV Unit Data (SpeedType / MovementZone / Locomotor)

Rust evidence: `ini/rulesmd.ini:6969` `[AMCV]`, line 6998: `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` (Drive), line 6999: `MovementZone=Normal`, line 6977: `Speed=4`. No explicit `SpeedType=` key — defaults to `SpeedType::Track` for ground vehicles with Drive locomotor.

gamemd evidence: not recomputed from binary this pass. SpeedType default fallback chain not decompiled for AMCV specifically.

Verdict: **UNCHECKED**. Unit data is concrete from INI; SpeedType-default parser parity was not traced in binary.

---

### 2. CliffRamps Tile Classification for Ramp Cell (51, 50)

gamemd evidence: `THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md` verifies `IsCliffOrImpassableTile @ 0x004863D0` uses half-open range `[CliffRamps, CliffRamps + 0x14)`. `TEMPERATE CliffRamps=25` → tile-id start `384`, range `[384, 404)`.

Rust evidence: `src/map/theater.rs:185-188` applies the same `0x14` half-open range via `in_fixed_range`. `GROUND_PATH_CLIFFRAMPS_TRAVERSAL_TRACE.md` §2 confirmed both compute start `384`, range `[384, 404)` — PASS there.

Concrete output: tile at (51, 50) in range → `is_cliff_like = true`; `canonical_ramp = Some(_)` for nonzero slope byte; `ground_walk_blocked = false`; `build_blocked = true`.

gamemd concrete cell metadata: not recomputed from binary for this exact cell in this session.

Verdict: **PASS** (tile-range classification numerically equal per GROUND_PATH_CLIFFRAMPS_TRAVERSAL_TRACE §2-3; ramp override logic passes prior trace §4 at code level). Strict gamemd cell metadata equality for (51,50) specifically: UNCHECKED.

---

### 3. A* Height-Diff Gate: (50,50) flat → (51,50) ramp-up (diff = +1)

Rust evidence: `src/sim/pathfinding/core.rs:1090-1105` — height diff check:
```
diff = neighbor_height - current.height
lower_slope = if diff < 0 { neighbor_cell.slope_type } else { cur_cell.slope_type }
legal = match diff.abs() { 0 => true, 1 => lower_slope != 0, _ => false }
```
For uphill step (diff = +1): `lower_slope = cur_cell.slope_type`. The current cell (50,50) is flat (`slope_type = 0`). This makes `lower_slope = 0` → `legal = false`.

**This is a bug.** For an uphill step the lower cell is (50,50) (flat). Flat lower cell → `lower_slope = 0` → path rejected. The correct check should be: for an uphill step (diff > 0), the UPPER cell (51,50) has a nonzero slope byte, not the lower one.

gamemd evidence: `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md §7.1-7.3` confirms ramp cells are passable and the slope byte lives at `CellClass+0x11C` on the ramp cell itself. The path must be legal when the RAMP cell (the upper/destination cell) has a nonzero slope byte.

Verdict: **FAIL**. Rust gates height-diff=1 on `lower_slope != 0` where lower slope is the CURRENT (flat) cell. For a low→high traversal the relevant nonzero slope byte is on the DESTINATION ramp cell, not the origin. Pathfinder will reject this step.

Player-visible effect: AMCV cannot pathfind onto the ramp from flat ground — path will not be found or will route around. Fires every time a unit approaches a ramp cell from flat low ground.

---

### 4. A* Cost on Ramp Cell

Rust evidence: `src/sim/pathfinding/terrain_cost.rs:57-67` — for a canonical ramp cell (`canonical_ramp.is_some()`): `cost = COST_NORMAL = 100`. No extra penalty for ramp cells in the cost grid.

gamemd evidence: `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md §5-6` — slope cost is a zone-pathfinder hierarchical cost (`Zone_Estimate_Slope_Cost`, coarse 4×4 grid). `Get_Slope_Cost_At_Cell @ 0x56BCD0` reads from zone map. The per-terrain-type speed table (`g_SpeedType_LandType_Table`) governs actual cell admission. No separate "ramp cost penalty" distinct from the cliff-speed multiplier was found in the research docs.

Concrete gamemd pathfinder cost: not extracted from binary for this exact cell/speed combination this session.

Verdict: **UNCHECKED**. Rust assigns `COST_NORMAL = 100` to ramp cells. Whether gamemd applies any additional zone-cost penalty to cliff-ramp cells specifically was not numerically verified in this pass.

---

### 5. Per-Tick Speed on Ramp Cell — Cliff Speed Multiplier

gamemd evidence: `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md §7.3` + `RULES_CLIFF_SPEED_MULTIPLIERS_GHIDRA_REPORT.md` — verified binary formula:
```
when Mission_Move AND new_cell.GroundHeight > old_cell.GroundHeight:
  if SpeedType == 1 (Track): speed *= RulesClass+0x768 = TrackedUphill = 1.0
  else:                       speed *= RulesClass+0x778 = WheeledUphill = 1.0
```
Stock `rulesmd.ini:401` `TrackedUphill=1.0`. AMCV has Drive locomotor / Track SpeedType (assumed). Uphill multiplier: **1.0** — no penalty. This is applied when `GroundHeight` differs, keyed on `GroundHeight` (not `Level`).

Rust evidence: `src/sim/pathfinding/terrain_speed.rs:118-145` — Rust applies `config.slope_climb` (default `0.6`, read from INI key `SlopeClimb=`). `ini/rulesmd.ini` does NOT contain `SlopeClimb=` — confirmed by grep: zero matches. Rust falls back to compiled default `0.6`.

Computed output:
- gamemd: uphill speed multiplier = **1.0** (no penalty for tracked vehicles)
- Rust: uphill speed multiplier = **0.6** (40% penalty from `SlopeClimb` default)

Verdict: **FAIL**. Rust applies a 40% speed penalty to uphill ramp traversal; gamemd applies none for tracked vehicles (`TrackedUphill = 1.0`). The INI key Rust reads (`SlopeClimb`) does not exist in stock rulesmd.ini and is not a gamemd key. The correct mechanism uses `TrackedUphill`/`WheeledUphill` from `[General]`. Fires every uphill step for any tracked ground vehicle.

Additional note: Rust `slope_descend` default is `1.2`, which happens to match `TrackedDownhill = 1.2` numerically, but the mechanism and key-name are wrong for the downhill case too. The correct keys to read are `TrackedUphill`/`TrackedDownhill`/`WheeledUphill`/`WheeledDownhill` branched on SpeedType.

---

### 6. Z-Coordinate Transition Across Ramp Cell

gamemd evidence: `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md` (cited in VXL_RHINO trace §5): `ObjectClass.Coords.Z = cell_level * LevelHeight (104)` for ground units. `CellClass.Level` (signed byte at `+0x11B`) changes by 1 at the ramp. Z changes when the unit crosses from (50,50) Level=0 to (51,50) Level=1 (ramp cell) to (52,50) Level=1 (high ground). The Z transition is a step at cell-boundary crossing — not interpolated sub-cell.

Rust evidence: `src/sim/movement/movement_bridge.rs:143-144` — `BridgeTransition::NoChange` arm:
```rust
position.z = dst_cell.effective_cell_z_for_layer(next_layer);
```
This writes `position.z` at every cell boundary crossing (not just bridge transitions). For a terrain ramp step, `NoChange` fires, and `effective_cell_z_for_layer(Ground)` returns `ground_level` (= `CellClass.Level`). So Rust DOES update `position.z` on ground cell transitions via the `NoChange` arm.

Concrete values (AMCV uphill east): at crossing (50→51), z: 0→1. At crossing (51→52), z: 1→1 (already high). At crossing (52→53), (53→54), (54→55): z stays 1.

gamemd Z transition curve: step at cell-boundary, not sub-cell interpolated. Rust matches this: `position.z` is updated once per cell boundary in `resolve_cell_transition_bridge_state`.

This is a PASS at the structural level. Exact `LevelHeight` pixel mapping to screen Y was not numerically computed for the exact scenario.

Verdict: **PASS** (step-at-boundary pattern matches; `position.z` updated to destination ground level via `NoChange` arm for each terrain ramp step).

---

### 7. VXL Slope Tilt During Ramp Climb

gamemd evidence: `VXL_DRAW_MATRIX_GHIDRA_REPORT.md`, `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md` — `DriveLocomotionClass::Process` reads `CellClass+0x11C` (SlopeIndex) of the occupied cell at the top of each process call. On entry to the ramp cell (slope_type = 1..4 for a cliff ramp), a 3-tick transition starts: `prev_slope = old`, `curr_slope = new_ramp_slope`, timer = 3. SLERP blend over 3 ticks.

Rust evidence: `src/sim/rocking/rocking_system.rs:165-172` (`update_slope_transition`) + `SLOPE_TRANSITION_TICKS = 3` (`rocking_system.rs:45`). Rocking tick samples `terrain.cell(rx,ry).slope_type` after movement in tick order (`src/sim/world/mod.rs:1192`).

Timing discrepancy: `VXL_RHINO_CONTROL_DOWNHILL_EDGE_SLOPE4_TO_FLAT_TRACE.md §4` (FAIL there) — gamemd samples slope before movement in Process, Rust samples after movement in the same tick. This means Rust starts the ramp-tilt blend one tick early relative to gamemd.

AMCV is a voxel unit: `ini/artmd.ini` — AMCV uses `Image=MCV`; `[MCV]` in artmd.ini has `Voxel=yes`. VXL slope tilt path applies.

Slope type for a cliff ramp cell: `SlopeIndex` 1..4 (cardinal edge ramps). The specific value depends on the ramp orientation. For an east-climbing ramp, the ramp cell's SlopeIndex would be 1 or 2 (ramp rising NE or NW per the SlopeIndex enumeration). The AMCV will tilt to match the ramp's compass+tilt.

Verdict: **FAIL** (one-tick-early slope transition start, already confirmed in VXL_RHINO_CONTROL trace §4 as a general ground-movement bug; applies identically to AMCV on cliff ramp). Exact VXL matrix floats and pixel output: UNCHECKED.

---

### 8. Body Facing During Climb

gamemd evidence: `DriveLocomotionClass` maintains body facing as a byte (0x00=North..0xC0=West, clockwise). AMCV starts facing East (0x40). Moving east on a cardinal path, no facing change is needed. No body-facing correction for slope is applied — the VXL slope tilt matrix handles the visual incline, not a facing-byte change.

Rust evidence: `src/sim/movement/movement_step.rs:70-110` — `configure_motion_after_transition` uses `facing_from_delta(ndx, ndy)` to set the new facing. For east movement (dx=+1, dy=0), `facing_from_delta(1,0)` = 0x40 (East). Facing byte is unchanged throughout the east traversal.

Verdict: **PASS**. Body facing byte stays 0x40 (East) for all steps (50→51→...→55); no facing byte change on a straight cardinal path.

---

### 9. Final Cell Occupancy at (55, 50) High Ground

Rust evidence: `apply_cell_transition_remainder` sets `position.rx = nx, position.ry = ny` at each crossing; `occupancy.move_entity` transfers the occupancy entry. After the last crossing (54→55): `position.rx=55, position.ry=50`; occupancy records AMCV at (55,50) ground layer.

gamemd evidence: standard ground-cell occupancy; no special ramp handling for the destination cell.

`position.z` after (54→55): `effective_cell_z_for_layer(Ground)` = ground_level of (55,50) = 1 (high ground). Correct.

Verdict: **PASS**. Cell (55,50) at high-ground level is the final occupancy; z is set to high-ground level.

---

### 10. Sound — Ramp-Climb Cue

gamemd evidence: No dedicated ramp-climb sound event found in `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md` or companion docs. Terrain transition sounds (`MoveSound=MCVMoveStart`) are unit-level and do not change per terrain type for a cliff ramp. No specific "ramp" audio branch was identified in the binary.

Verdict: **UNCHECKED**. No evidence of a distinct ramp-climb sound. Absence in research docs is not confirmation of absence in binary. Dedicated sound decompilation was not performed this pass.

---

## Failures (Top 3 Visible)

1. **A* height-diff gate rejects low→high ramp** (Stage 3 — FAIL). Rust checks `lower_slope != 0` using the current (flat, slope=0) cell for an uphill step. The nonzero slope byte is on the DESTINATION ramp cell. AMCV cannot pathfind onto a cliff ramp from flat ground. Fires every unit-vs-terrain-ramp uphill approach.

2. **Uphill speed multiplier wrong mechanism** (Stage 5 — FAIL). Rust applies `SlopeClimb = 0.6` (40% penalty, INI key absent from rulesmd.ini). gamemd applies `TrackedUphill = 1.0` from `[General]` (no penalty). Speed is 40% too slow on any uphill ramp for tracked vehicles.

3. **VXL slope tilt starts one tick early** (Stage 7 — FAIL). Rust samples slope after movement; gamemd samples before. Body begins tilting to the ramp angle one render frame too soon. Fires at every cell-boundary slope change for voxel units.

## Adjacent Findings

- `SlopeDescend = 1.2` in Rust happens to equal `TrackedDownhill = 1.2` numerically for tracked vehicles, but the mechanism is wrong (wrong INI key, no SpeedType branch). Wheeled vehicles would get 1.2 for downhill in both — accidentally correct for stock values but will diverge for modded values or wheeled SpeedType.
- The `NoChange` arm in `resolve_cell_transition_bridge_state` updates `position.z` for all ground transitions — this is correct and avoids the "unit renders too high after elevation change" bug flagged in `VXL_RHINO_CONTROL` §5. That Rhino trace flagged Z as FAIL (position.z not written on non-bridge transitions), but the code does write it; the discrepancy may be timing/path_grid availability — this scenario is PASS.

## Sources

- `docs/research/CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md`
- `docs/research/RULES_CLIFF_SPEED_MULTIPLIERS_GHIDRA_REPORT.md`
- `docs/research/SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md`
- `docs/research/THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
- `docs/research/traces/GROUND_PATH_CLIFFRAMPS_TRAVERSAL_TRACE.md`
- `docs/research/traces/VXL_RHINO_CONTROL_DOWNHILL_EDGE_SLOPE4_TO_FLAT_TRACE.md`
- `docs/research/traces/VXL_CHRONO_MINER_UPHILL_FLAT_TO_EDGE_SLOPE4_TRACE.md`
- `src/sim/pathfinding/core.rs:1090-1105`
- `src/sim/pathfinding/terrain_cost.rs:57-67`
- `src/sim/pathfinding/terrain_speed.rs:118-145`
- `src/sim/movement/movement_bridge.rs:119-148`
- `src/sim/movement/movement_step.rs`
- `src/sim/rocking/rocking_system.rs:45,165-172`
- `ini/rulesmd.ini:6969-7010` (AMCV), lines 401-404 (cliff speed keys)
