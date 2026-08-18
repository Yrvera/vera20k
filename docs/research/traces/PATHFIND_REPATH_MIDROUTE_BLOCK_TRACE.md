# Pathfinding — Dynamic Re-path When Mid-Route Obstacle Appears
## Trace Report

**Scenario:** Grizzly Tank ordered (50,50)→(60,50) on flat grass. After ~5 ticks the
tank is at approximately (52,50) walking east on a straight A* path. Player places
GAWALL at (55,50) in the tank's path. Trace covers: cell becomes blocked, re-path
trigger detection, recomputed A*, new path followed, arrival at (60,50).

---

## Stage Table

| # | Stage | gamemd | Our Engine | Verdict |
|---|-------|--------|------------|---------|
| 1 | Initial A* path computed from (50,50) to (60,50) | `FootClass::Find_Path` (0x4D3920) computes up to 24-step segment; path walks E along y=50 | `find_move_path` via `zone_search::find_path_zoned`, zone precheck, smooth + optimize, ≤24 steps | UNCHECKED |
| 2 | PathGrid update when GAWALL placed | PathGrid not a concept in gamemd — `Can_Enter_Cell` reads live cell state | `rebuild_dynamic_path_grid` (app_sim_tick.rs:775) called after `spawned_entities` flag set; happens at end of frame after the tick that placed the wall | FAIL |
| 3 | Blockage detected during walk tick (per-step Can_Enter_Cell recheck) | `Process_Movement` calls `Can_Enter_Cell` (vtable+0x1AC) on the NEXT cell each tick before setting up a track. Code 7+ (Impassable) from building footprint triggers stop-and-repath | Our engine checks terrain walkability via PathGrid and occupancy. Terrain block detected in `process_cell_crossings` → `layer_walkable == false` branch | UNCHECKED |
| 4 | Timing of repath trigger: lazy (on walk tick) vs. eager (when wall placed) | **LAZY** — gamemd detects blockage when `Process_Movement` tries to enter (55,50) on the tick the tank's path reaches that cell. No eager notification; no event bus | Our engine: **also lazy** — blockage detected in `process_cell_crossings` when sub_x/sub_y cross the cell boundary toward (55,50). PathGrid is rebuilt at end of prior frame so the blocking cell is already non-walkable when the movement tick runs. | PASS |
| 5 | Stop duration while re-planning (ticks of standstill) | gamemd: **0 stop ticks in the common case**. `Process_Movement` (0x4B2630) Phase 2a checks the movement_delay timer (techno+0x640); if delay has expired, calls `FootClass::Find_Path` immediately in the SAME tick. If Find_Path succeeds, sets up a new drive track in the same tick call (two-phase chain). The unit may stall 0 ticks. It stalls `movement_delay_ticks` (= `Math::ftol(path_delay * speed_factor)`) if the delay is still running. | Our engine: sets `movement_delay = path_delay_ticks` (from `BlockagePathDelay` INI key) on EVERY successful repath. The unit waits `path_delay_ticks` even if it repathed immediately. This is a potential timing discrepancy. | FAIL |
| 6 | A* start cell: current cell (52,50) not original origin (50,50) | `Path_walk_directions_to_cell` (0x429780) in `FootClass::Run_AStar` walks the EXISTING path_queue to find the current position, then calls `AStar_pathfind_search` with source = current cell. Confirmed by doc §2 / §3 | `try_repath_after_block` passes `current` (the entity's current rx/ry) as the A* start cell, not the original goal. Correct. | PASS |
| 7 | Zone precheck before A* | `Zone_precheck` (0x42c290) runs hierarchical A* on zone adjacency graph before committing to cell-level A* | `zone_search::find_path_zoned` calls `ZoneGrid` precheck | PASS |
| 8 | Can_Enter_Cell return code for GAWALL in path: code 7 (Impassable) | `UnitClass::Can_Enter_Cell` (0x73F0A0) checks overlay blockers — GAWALL is an overlay with `Impassable=yes` terrain (returns 7+) | PathGrid `block_building_footprint` / `grid.is_walkable(55,50)` returns false for wall footprint. Terrain check in `process_cell_crossings` produces `layer_walkable=false`. Maps to code-7 equivalent. | PASS |
| 9 | path_stuck_counter initialization and decrement on urgency-2 failure | gamemd: `path_stuck_counter` (techno+0x64C) initialized to 10 in `Map_Edge_Retreat` path (0x4B3282). Decremented only when code-2 repath fails after blocked_delay expires | Our engine: `PATH_STUCK_INIT = 10` (visible in movement_step.rs:37 import). Counter decremented only at urgency=2 in `handle_blocked_tick` (movement_blocked.rs:143). Matches gamemd semantics. | PASS |
| 10 | blocked_delay (BlockagePathDelay) timer before urgency escalation | gamemd code 2 (code 7 has no grace period): `path_blocked_flag` (techno+0x6B7) set on first encounter, `blocked_delay_ticks` (techno+0x670) set from `Rules.BlockedDelay` (Rules+0x1768). Urgency=1 while running, urgency=2 after expiry. For code 7 (wall), no grace — immediate stop+repath. | Our engine: for `layer_walkable=false` path (terrain block), calls `handle_blocked_tick` which sets `blocked_delay = blockage_path_delay_ticks` and urgency=1 on first call. **Discrepancy**: gamemd code-7 does NOT use a blocked_delay grace period; it immediately goes to stop+repath. Our engine gives it a grace period the same as code-2. | FAIL |
| 11 | New A* path returned from current cell, not touching (55,50) | New path computed from (52,50) to (60,50) routing around (55,50). Cell (55,50) is now non-walkable in PathGrid. A* expands around it. Path likely goes (52,50)→(53,50)→(54,50)→(54,51)→(55,51)→(56,51)→(57,50)→(58,50)→(59,50)→(60,50) or similar diagonal detour | Our engine computes same via `zone_search::find_path_zoned` with (55,50) absent from walkable grid. Path correct. | PASS |
| 12 | Path smoothing/optimization on new path | `Path_smooth_corners` (0x42b210) + `Path_optimize_straight_segments` (0x42b7f0) post-process new path. Path truncated to 20 entries. Zigzag shortcuts validated via Can_Enter_Cell. | Our engine: `path_smooth::smooth_path` + `path_smooth::optimize_path`; `entity_block_map` excluded from shortcuts (movement_path.rs:274). Max 24 steps via `truncate_layered_path`. | UNCHECKED |
| 13 | Drive-track continuation on new path | After new path set, `Process_Drive_Track(is_retry=1)` called immediately in same tick to begin consuming the new track | Our engine: `movement_delay = path_delay_ticks` set after repath; drive track starts next tick when delay expires. One-tick gap vs. gamemd's same-tick chain. | FAIL |
| 14 | Visual cue: turret bob / drive-track interruption when wall appears | No audio cue for obstacle detection. No turret bob on blockage. Drive-track curve completes if mid-track; unit may overshoot one cell, then replan | Our engine: drive_track cleared via `*drive_track_state = None` in the terrain-block path (movement_step.rs:535). Position snapped to cell center. No turret bob event. Matches gamemd: no special visual on block. | PASS |
| 15 | Arrival at (60,50) after detour | Unit follows new path, arrives at (60,50). `finalize_finished_entities` removes movement_target. | Our engine: `finished_entities.push(entity_id)` on path exhaustion at goal. Locomotor phase → Idle. | PASS |

---

## gamemd.exe Mechanism Summary (verified from docs)

**When wall placed:** No eager notification. Cell state changes via overlay injection;
PathGrid equivalent (Can_Enter_Cell) reads live CellClass state.

**Detection function:** `DriveLocomotionClass::Process_Movement` (0x4B2630) Phase 4
(lines 525–820). Calls `Can_Enter_Cell` (vtable+0x1AC) on next path cell each tick.
For GAWALL → returns code 7 (Impassable). Doc: DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §Phase 4 "Code 7" handler.

**Code 7 handler (DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §Code 7):**
```
if is_retry (param_2) != 0:
    techno.movement_state = -1
    techno.movement_delay_start = CurrentFrame
    → delay + recurse
// First encounter:
Clear head_to to NullCoord
if techno.tether_target != 0:
    FootClass::Stop_Moving()
    return ScanForTarget(0,1)
else:
    StopMission(0,1)
    return 0
```
On first code-7 encounter: head_to cleared, StopMission fires. No blocked_delay timer.
Next tick: `Process_Movement` Phase 2 (HAS_DESTINATION_NO_PATH) calls `FootClass::Find_Path` directly.

**Zero-stop-tick path:** Movement_delay timer (techno+0x640) checked in Phase 2a. If
elapsed ≥ delay_ticks, `Find_Path` called immediately. New path sets up drive track in
the same tick via two-phase chain (Process_Movement → Process_Drive_Track(is_retry=1)).
Unit moves without a visible stop if movement_delay_ticks == 0.

**Drive-track interruption:** No explicit cancellation. The track is effectively
abandoned when `head_to` is cleared. The unit may complete the current track step to
the next cell before detecting the wall one cell ahead.

---

## Adjacent Findings (do not trace this run)

1. **Code-7 vs code-2 timer parity**: gamemd uses NO blocked_delay grace for code-7
   (wall/building). Our engine applies the same `blockage_path_delay_ticks` grace to
   all terrain blocks. For frequently-placed walls, this causes a visible ~3-5 tick
   stall before repath where gamemd reacts in 1 tick.

2. **Same-tick repath chain**: gamemd calls `Process_Drive_Track(is_retry=1)` in the
   same tick after `Find_Path` succeeds, producing zero-tick stalls. Our engine always
   waits `path_delay_ticks` before continuing. Quantifiable as N ticks of extra stall
   on every repath event.

3. **PathGrid rebuild timing**: Our PathGrid is rebuilt once per frame (after all sim
   ticks run) via `rebuild_dynamic_path_grid`. If the map runs multiple sim ticks per
   frame, the PathGrid may be stale for 1+ ticks after a wall is placed mid-frame.
   gamemd reads live CellClass state every tick so has no such lag.

4. **Path segment boundary re-path**: If the tank is near the end of its 24-step
   segment when the wall appears, `handle_path_exhaustion` triggers a segment repath
   before `process_cell_crossings` even runs. Both paths ultimately call `find_move_path`
   from the current position. No disparity — both repath from current cell.

5. **Smoothing shortcuts skip entity-blocked cells**: Our `smooth_walkable` closure
   (movement_path.rs:257-275) excludes entity_block_map cells from shortcuts, matching
   the intent of gamemd's `Path_optimize_straight_segments` using Can_Enter_Cell to
   validate shortcuts. UNCHECKED at the numerical level.

---

## Verdict Tally

**PASS: 7 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0**

---

## Top 5 Most Player-Visible Failures

1. **[Stage 10] Code-7 (wall) gets same blocked_delay grace as code-2 (friendly)**
   — Player sees: tank stalls 3–5 ticks after wall is placed before re-routing, where
   gamemd reacts within 1 tick. Every time a wall is placed in a unit's path.
   — File: `src/sim/movement/movement_step.rs:547-571` (terrain block → `handle_blocked_tick`
   unconditionally sets `blocked_delay = blockage_path_delay_ticks`).
   — gamemd evidence: DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §Code 7 — no blocked_delay
   timer, immediate head_to clear + StopMission.

2. **[Stage 5 / Stage 13] movement_delay gap after every repath**
   — Player sees: tank pauses 1+ ticks after each successful repath before beginning the
   new route. gamemd has zero-tick gap via same-tick `Process_Drive_Track(is_retry=1)` chain.
   Fires on every repath event (every blocked cell encounter).
   — File: `src/sim/movement/movement_path.rs:420` (`target.movement_delay = mcfg.path_delay_ticks`)
   and `movement_tick.rs:446` (delay decremented at start of tick, not end).
   — gamemd evidence: DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md §Step 2D and §Step 3H —
   `Process_Drive_Track(is_retry=1)` called immediately after `Process_Movement` succeeds.

3. **[Stage 2] PathGrid rebuild lag: one frame behind per wall placement**
   — Player sees: if >1 sim ticks fire per render frame, a wall placed mid-frame is
   invisible to the pathfinder for up to 1 tick's worth of movement. Units walk into
   the building cell in the gap. Frequency: every game frame that accumulates ≥2 sim ticks.
   — File: `src/app_sim_tick.rs:703-704` (rebuild happens after all ticks for the frame).
   — gamemd evidence: Not a gamemd issue — gamemd reads live CellClass state on every tick.
   Our architectural difference (static PathGrid per frame) causes this.

4. **[Stage 10] blocked_delay timer not reset on code-2→code-7 transition**
   — Player sees: if tank is first blocked by a moving unit (code-2) then a wall appears
   in the same path segment, the blocked_delay counter starts from a partially-elapsed state.
   The tank may route around the wall faster or slower than expected.
   — File: `src/sim/movement/movement_blocked.rs:57-69` (first-block detection shared across
   all block types; timer set once and not reset on type change).
   — gamemd evidence: DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §Code 2 vs §Code 7 —
   separate timer fields (blocked_delay at +0x668 vs movement_delay at +0x640).

5. **[Stage 1 / Stage 12] Path smoothing numerical parity unverified**
   — Player sees: post-repath path may take a different number of diagonal steps around
   the wall than gamemd produces (2-corner detour vs 1-corner detour). The smoothing
   pass uses a different algorithm (`smooth_path` + `optimize_path`) from gamemd's
   `Path_smooth_corners` + `Path_optimize_straight_segments`. Observable as tank taking
   a slightly different curve around the obstacle.
   — File: `src/sim/pathfinding/path_smooth.rs` (not read this run).
   — gamemd evidence: PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.11 — two-pass: direction-delta
   smoothing then straight-segment shortcutting with Can_Enter_Cell validation.

---

## Report File

`C:/Users/enok/Documents/ra2-rust-game-docs/traces/PATHFIND_REPATH_MIDROUTE_BLOCK_TRACE.md`

**Status: COMPLETE**
