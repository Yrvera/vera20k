# Miner-Stuck-on-Ore — Harvester-vs-Tiberium Passability Bypass Trace

**Scenario:** Track-type Chrono Miner (SpeedType=Track, MovementZone=Crusher) stepping
onto a Tiberium-overlay cell. Per our `PASSABILITY_MATRIX` col 5 (Tiberium) all rows have
value 2 (blocked), so `grid.is_walkable(nx, ny)` = false → unit stuck at cell boundary.

**Date:** 2026-05-20
**Scope:** Narrow — harvester bypass of SpeedType×LandType gate only. No other systems.
**Iron Law:** PASS requires binary-verified equality. Anything inferred is UNCHECKED.

---

## Stage Table

| # | Stage | gamemd behavior | Our behavior | Verdict |
|---|-------|----------------|-------------|---------|
| T1 | SpeedType×LandType value — Track+Tiberium | `[Tiberium] Track = 0.7` (70%) from rulesmd.ini; `g_SpeedType_LandType_Table[Track + Tiberium*9] = 0.7` ≠ 0.0 | `PASSABILITY_MATRIX[Crusher][Tiberium] = 2` (blocked); `grid.is_walkable(nx,ny) = false`; `is_cell_passable_for_mover` returns `false` | FAIL |
| T2 | RecalcZoneType classification of Tiberium cells | `RecalcZoneType @ 0x483C80`: IsTiberium overlay → ZoneType 6 (Impassable), step 2d in decision tree. Matrix row 1 (Crusher), col 6 = 2 (blocked). Tiberium cells ARE zone-Impassable in gamemd too | Our `overlay_grid.rs:195` sets `overlay_blocks = true` for tiberium; `zone_type = IMPASSABLE`. Zone matrix same behavior. | PASS |
| T3 | A* goal-cell impassability handling | `AStar_main_loop @ 0x429A90` §3.4 item g: result==7 is skipped UNLESS neighbor == destination cell. Per doc: "If result==7 (impassable): skip unless it's the destination cell." Goal cell is expanded even if impassable zone. A* can produce a path terminating AT the ore cell directly. | `core.rs:516-526`: `if !goal_ground_ok && !goal_bridge_ok { return None; }` — exits early if goal is not walkable. Never attempts pathfinding to an impassable goal. | FAIL |
| T4 | Can_Enter_Cell speed-table check — Track+Tiberium | `UnitClass::Can_Enter_Cell @ 0x73F0A0` (post-occupant loop, line `0x0073fab5`): `if table[LandType*9 + SpeedType] == 0.0 { return 7; }`. For Track+Tiberium: `0.7 ≠ 0.0` → does NOT return 7. Returns 0 (Clear). Ore cell IS passable per Can_Enter_Cell. | Our `movement_step.rs:491`: checks `path_grid.is_walkable(nx, ny)`. PathGrid marks Tiberium cells `ground_walkable = false` (line `core.rs:1400`). Returns false → unit blocked. | FAIL |
| T5 | Is_Cell_Harvestable passability call | `FootClass::Is_Cell_Harvestable @ 0x4DCE80` step 5: calls `Can_Enter_Cell(cell, -1, -1, 0, 1)`. Track+Tiberium speed = 0.7 ≠ 0.0 → returns 0 (OK). Harvester CAN enter any ore cell it can see. | `search_local_ore` in `miner_system.rs` has no equivalent per-cell passability check; `build_reachable_filter` uses zone-based reachability which marks ore cells unreachable (they are zone IMPASSABLE). | FAIL |
| T6 | Process_Movement per-step Can_Enter_Cell | `DriveLocomotionClass::Process_Movement` calls `Can_Enter_Cell` for each next-step cell. Tiberium result = 0 (Clear) since speed = 0.7. Unit proceeds normally across ore cells. | `movement_step.rs:491`: grid check blocks without checking speed table. Our equivalent of "result 0 = proceed" is `grid.is_walkable` which is always false for Tiberium. | FAIL |
| T7 | adjacency direct-move (issue_direct_move) for ore | `miner_system.rs:396-400`: when `dx ≤ 1 && dy ≤ 1`, calls `issue_direct_move` with `ignore_terrain_cost: true`. This sets `terrain_ok = true` (line 493) but NOT `bypass_grid = true`. `grid_ok = path_grid.is_walkable(nx,ny)` = false. Still blocked. | Same (described above). The `bypass_grid` field IS present on `MovementTarget` but is not set by `issue_direct_move`. | FAIL |
| T8 | `bypass_grid` mechanism exists in our code | Not applicable — no equivalent in gamemd. gamemd uses `Can_Enter_Cell` result 0 for the step, which is computed from the speed table. `bypass_grid` is our engine's internal flag. | `MovementTarget.bypass_grid` exists and bypasses `grid.is_walkable` check at line 491. Setting this flag on the issue_direct_move movement target would fix the adjacent-step case. | NOT-IMPLEMENTED |
| T9 | Zone-based reachability scan filter | `Is_Cell_Harvestable` step 3: calls `Can_Reach_Zone(cell, SpeedType_zone)` — respects zone connectivity; zones for Tiberium cells ARE impassable (ZoneType 6), so `Can_Reach_Zone` returns false for cells only reachable via tiberium. For adjacent ore cells (miner is next to them), zone reachability fails even though the miner can physically step there. | `build_reachable_filter` in miner_system.rs also uses zone connectivity, which similarly excludes ore cells from the zone-map. Our filter is stricter: it filters ore out of the candidate list entirely. | UNCHECKED — both engines exclude far tiberium cells via zone; both can reach adjacent ore via direct step. The key question is whether our zone filter incorrectly filters adjacent-ore when gamemd's Can_Reach_Zone would also return false. |
| T10 | How gamemd reaches ore despite Can_Reach_Zone=false | gamemd's `Search_For_Tiberium_And_Move` calls `Set_Destination(ore_cell)` unconditionally after `Is_Cell_Harvestable` returns 1. A* is then asked to reach an impassable goal. Result: A* produces a path to the nearest passable cell adjacent to the ore goal, and then `Process_Movement` allows stepping onto the ore cell directly via `Can_Enter_Cell` (result 0, since speed=0.7). The zone-mismatch means A* routes to the cell next to the ore; the final step onto the ore cell itself passes Can_Enter_Cell. | Our A* early-exits at goal check (`core.rs:524`) if goal is not walkable. If it did reach the adjacent cell, `movement_step.rs:491` would block the final step anyway. Two separate failures stack. | FAIL (dual failure) |

---

## Root-Cause Summary

There are two stacked failures, not one:

### Root Cause A — A* Goal Passability Gate (T3)
**Location:** `src/sim/pathfinding/core.rs:516-526`

Our A* returns `None` if the goal cell's `is_cell_passable_for_mover` check fails. For
any Tiberium-overlay cell, `overlay_blocks = true` → `ground_walkable = false` → goal
check fails → `return None`.

gamemd's A* does NOT bail on an impassable goal. It instead allows the expansion to reach
the destination cell even if it's result==7 (impassable), handling it as a near-miss: the
path terminates at the cell adjacent to the impassable goal.

**Fix:** Do not early-exit when goal is impassable. Allow A* to produce a path to the
nearest passable cell, and then the direct-move final step handles the last hop. Alternatively,
mark the goal as passable for the purposes of A* when the miner is targeting an ore cell.

### Root Cause B — Per-Step Grid Check Not Bypassed for Harvesters (T4, T7)
**Location:** `src/sim/movement/movement_step.rs:491`

```rust
target.bypass_grid || path_grid.map_or(true, |grid| grid.is_walkable(nx, ny))
```

`grid.is_walkable(nx, ny)` = false for Tiberium cells. `issue_direct_move` sets
`ignore_terrain_cost: true` (controls `terrain_ok` at line 493) but NOT `bypass_grid: true`
(controls `grid_ok` at line 491). So even the direct-move adjacency step is blocked.

gamemd's equivalent: `Can_Enter_Cell` for Track+Tiberium returns 0 (Clear), not 7
(Impassable), because `g_SpeedType_LandType_Table[Track + Tiberium*9] = 0.7 ≠ 0.0`.
The per-step gate is purely the speed-table check, not a zone-matrix check.

**Fix:** Set `bypass_grid: true` when issuing the direct-move step to an ore cell. This
makes the harvester's final step onto the ore cell bypass the PathGrid's `is_walkable`
check, matching gamemd's Can_Enter_Cell returning 0 for Track+Tiberium.

The minimal fix for the stuck-miner bug is Root Cause B: set `bypass_grid = true` in
`issue_direct_move` calls from the miner (or set it only when the target is a known ore
cell). Root Cause A is a pathfinding correctness issue that also needs fixing for cases
where the miner is >1 cell away from ore and A* needs to route to it.

---

## Key Binary Evidence

| Claim | Binary Evidence | Confidence |
|-------|----------------|-----------|
| `[Tiberium] Track = 0.7` | `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` §4, row LT=5 col Track: 0.7. Verified from rulesmd.ini `[Tiberium] Track=70%` + loader trace at `0x89ea44` + 4 bytes per LT row. | HIGH |
| `Can_Enter_Cell` speed check: `table[LT*9+ST] == 0.0 → return 7` | `UnitClass::Can_Enter_Cell @ 0x73F0A0`, instruction `0x0073fab5`: `FLD float ptr [EDX*0x4 + 0x89ea40]` + `FCOMP float ptr [0x7e1748]`; 0x7e1748 = 0.0 (read_memory verified in SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md §3.2). | HIGH |
| A* allows impassable destination cell | `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md §3.4` item g: "If result == 7 (impassable): skip unless it's the destination cell." | HIGH (doc-sourced; not re-decompiled this pass) |
| Tiberium ZoneType = 6 (Impassable) | `RecalcZoneType` decision tree step 2d (MOVEMENT_CLASSIFIERS_REFERENCE.md §4): "IsTiberium → 6 (Impassable), RETURN." | HIGH |
| Crusher zone, col 6 = 2 (blocked) | `MOVEMENT_CLASSIFIERS_REFERENCE.md §6` matrix: row 1 (Crusher) col 6 = 2 (blocked). Binary verified `read_memory 0x82A594 len 416`. | HIGH |

---

## Top 5 Player-Visible Failures

1. **Harvester stuck at adjacent ore cell boundary** — miner approaches to within 1 cell of ore, stops, never harvests. Fires on every harvest cycle as the miner approaches the ore patch. `src/sim/movement/movement_step.rs:491` grid check blocks direct-move; `movement_commands.rs:137` sets `ignore_terrain_cost:true` but not `bypass_grid:true`. gamemd: Can_Enter_Cell returns 0 (Clear) for Track+Tiberium (speed=0.7≠0.0).

2. **A* fails to route to ore cells beyond adjacency range** — if miner is >1 cell away from ore and no passable cell exists adjacent, A* returns None (`core.rs:524-526`) because goal `ground_walkable=false`. gamemd allows the impassable goal in A* (special-case in AStar_main_loop). Fires whenever ore field has surrounding impassable cells, preventing scan from working correctly.

3. **Zone-based scan filter excludes all ore cells** — `build_reachable_filter` in `miner_system.rs` marks ore cells as zone-unreachable (correct per zone-map, since Tiberium=ZoneType 6=Impassable→zone=INVALID). Scan returns no candidates. gamemd's `Is_Cell_Harvestable` uses `Can_Reach_Zone` + `Can_Enter_Cell` — `Can_Reach_Zone` fails but `Can_Enter_Cell` passes, so ore is found. Our filter discards ore before the Can_Enter_Cell-equivalent step even runs. Frequency: every scan tick when all ore is on Tiberium cells (always).

4. **`issue_direct_move` used for dock sequence also affected** — `miner_dock_sequence.rs:422, 701` also call `issue_direct_move`. Any dock sequence step that crosses a Tiberium overlay (e.g., refinery pad on or adjacent to ore) would be blocked by the same `grid_ok` gate. Less common but same root cause.

5. **Incorrect passability matrix column assignment** — `passability.rs:115`: comment states "5 Tiberium → 6 Impassable (binary col 6)" and sets all Tiberium values to 2 (blocked). This is the correct zone-type classification for zone-connectivity purposes (Tiberium IS ZoneType 6 in gamemd). However, the PathGrid's per-cell `ground_walkable` field (built at `core.rs:1400`) uses `overlay_blocks` directly, bypassing the speed-table check entirely. gamemd's per-step gate in `Process_Movement` uses `Can_Enter_Cell` (speed table check), not the zone matrix. Our PathGrid conflates zone-connectivity (impassable = no paths through) with per-step passability (impassable = can't even step onto).

---

## Verdict Tally

PASS: 1 | FAIL: 6 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

---

## The Fix (minimal, for swarm consumers)

**File:** `src/sim/miner/miner_system.rs`
**Location:** `handle_move_to_ore`, lines 396-400

The adjacency direct-move at `issue_direct_move(…)` needs `bypass_grid = true` on the
resulting `MovementTarget` — just as the doc comment at `movement_commands.rs:95` says
"callers that also need to bypass `path_grid` walkability … should set `bypass_grid = true`
on the resulting `MovementTarget`."

After calling `issue_direct_move`, patch the movement target:
```rust
if let Some(entity) = sim.entities.get_mut(snap.entity_id)
    && let Some(ref mut mt) = entity.movement_target
{
    mt.bypass_grid = true;  // ore cells are Tiberium-blocked in PathGrid but
                             // Track+Tiberium speed=0.7 → Can_Enter_Cell returns 0 in gamemd
}
```

A* goal-check also needs fixing (`core.rs:524`) — allow impassable goals to attempt
pathfinding (returning the near-miss path) rather than bailing immediately.

---

## Status: COMPLETE

**Sources:**
- `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` — full table, Track+Tiberium=0.7 verified
- `MOVEMENT_CLASSIFIERS_REFERENCE.md` — passability matrix, RecalcZoneType decision tree
- `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` — AStar_main_loop item g (impassable goal handling)
- `MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md` — Is_Cell_Harvestable §5
- `UnitClass::Can_Enter_Cell @ 0x73F0A0` — live decompile this pass
- `FootClass::LocomotorPassabilityCheck @ 0x4D9C10` — live decompile this pass
- `src/sim/pathfinding/core.rs:516-526, 750-774` — goal passability gate
- `src/sim/movement/movement_step.rs:491-495` — grid_ok vs terrain_ok split
- `src/sim/movement/movement_commands.rs:88-150` — issue_direct_move sets ignore_terrain_cost but not bypass_grid
- `src/sim/pathfinding/passability.rs:100-146` — PASSABILITY_MATRIX col 5 = Tiberium = blocked
- `src/map/resolved_terrain.rs:389-398` — Tiberium overlay sets land_type=Tiberium + overlay_blocks
- `src/sim/pathfinding/core.rs:1400` — overlay_blocks → ground_walkable=false
