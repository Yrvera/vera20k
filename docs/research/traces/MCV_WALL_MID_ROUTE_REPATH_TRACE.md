# MCV Wall Mid-Route Repath Trace

**Mechanic:** Allied AMCV ordered (50,50) → (60,50) on flat grass. Wall (GAWALL) placed
dynamically at (57,50) when AMCV is at approximately (54,50), mid-route heading East.
Trace covers: wall lands, PathGrid update timing, blockage detection, repath trigger,
new path direction, stop ticks, facing update, arrival at (60,50).

**Scenario parameters:**
- AMCV INI: `MovementZone=Normal`, `Crusher=yes`, `Locomotor={4A582741}` (Drive),
  `Speed=4`, `ROT=5` (gradual turning). INI source: `rulesmd.ini:6998-7000`.
- GAWALL INI: `Wall=yes`, placed as overlay on (57,50). IsWall=yes.
- Route: 10 cells due East. Wall at cell (57,50) = 3 cells ahead of AMCV detection point.
- PathDelay=0.01min → `path_delay_ticks=9` ticks. BlockagePathDelay=60 frames.
  Source: `rulesmd.ini:3106-3107`, `src/sim/world/mod.rs:484-485`.

**Binary reference:** gamemd.exe YR 1.001 verified via Ghidra MCP (read-only). Primary docs:
- `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` — code-7 handler verified.
- `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` — Can_Enter_Cell return codes.
- `PATHFIND_REPATH_MIDROUTE_BLOCK_TRACE.md` — prior generic-tank trace (Grizzly).
  NOTE: that trace's Stage 5 / Stage 13 FAIL verdicts are **stale** — the Rust code
  was fixed after that doc was written. See Stage 5 and Stage 13 in this report.

**Date:** 2026-05-27
**Independent confirmation target:** overlapping parallel session "amcv_obstacle_detour"
traces a static pre-existing obstacle. This trace is the **dynamic insertion** case
(wall built while AMCV is moving) — distinct disparity-risk profile.

---

## Stage Table

| # | Stage | gamemd | Our Engine | Verdict |
|---|-------|--------|------------|---------|
| 1 | Initial A* at move command | AMCV issues move order → `FootClass::Find_Path` (0x4D3920). `MovementZone=Normal` (row 0). (57,50) not yet blocked → clear path (50,50)→(51,50)→…→(60,50), all code-0 cells | `issue_move_command_with_layered` → `zone_search::find_layered_path_zoned_marker`. PathGrid at command time has no wall at (57,50) → straight 10-cell east path. `MovementTarget.movement_delay=0`. | PASS |
| 2 | GAWALL wall placement occupancy type | GAWALL is a `Wall=yes` building type. `BuildingClass::Unlimbo` @ `0x00440580` checks `type+0x1571 (Wall=) != 0` → plants as **overlay** on the CellClass, not as a BuildingClass entity. `CellClass::CalculateArea` marks the cell with overlay data. No separate `BuildingClass` entity exists at (57,50). | Our engine: `PlaceReadyBuilding` for a `wall=true` object does NOT spawn a building entity. Instead, `inject_placed_wall_overlays` adds an `OverlayEntry` to `state.overlays`. The wall is tracked as an overlay, not an entity. Wall is NOT present in `sim.entities` — only in `state.overlays` (app layer). | PASS |
| 3 | PathGrid update timing after wall placement | gamemd: No static PathGrid concept. `Can_Enter_Cell` reads live `CellClass` state every tick. The moment `BuildingClass::Unlimbo` plants the wall overlay, the next `Can_Enter_Cell` call on (57,50) returns code ≥ 7 (Impassable). Zero lag. | Our engine: `PlaceReadyBuilding` command executes at start of `advance_tick` (line 1204-1226 in `world/mod.rs`). `spawned_entities=true` is set. Movement runs NEXT in the same `advance_tick` (line 1231). PathGrid is NOT rebuilt inside `advance_tick`. `rebuild_dynamic_path_grid` is called AFTER the entire frame's tick loop at app_sim_tick.rs:759. Wall appears in PathGrid only on the NEXT RENDER FRAME. | FAIL — see Stage 3 detail |
| 4 | When does movement first see the wall? | gamemd: On the tick the AMCV's `Process_Movement` tries to enter (57,50). `Can_Enter_Cell` reads live CellClass state → code 7 immediately. No lag from wall-placement tick to detection tick as long as movement runs in the same binary frame after Unlimbo. | Our engine: On the render frame the wall is placed, `advance_tick` runs movement with the OLD PathGrid. If the AMCV has not yet reached (57,50) on that tick, movement proceeds normally. `rebuild_dynamic_path_grid` runs AFTER the tick loop. On the NEXT render frame, PathGrid now marks (57,50) non-walkable. Detection happens the next tick the AMCV tries to enter (57,50). **Net lag: 0–1 render frames** (up to 1 full sim tick on a 1-tick-per-frame schedule; more on multi-tick frames). | DRIFT — 0–1 tick lag vs gamemd's same-tick detection |
| 5 | Detection mechanism: poll vs notification | gamemd: **LAZY** — `DriveLocomotionClass::Process_Movement` (0x4B2630) Phase 4 calls `Can_Enter_Cell` (vtable+0x1AC) on the next path cell each tick. No event bus, no eager notification. The wall being placed does NOT notify any locomotor. DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §Phase 4 confirmed. | Our engine: **Also lazy** — `process_cell_crossings` in `movement_step.rs:492` checks `path_grid.is_walkable(nx, ny)` at cell-crossing time. Equivalent to `Can_Enter_Cell` code-7 check. No event bus. | PASS |
| 6 | Can_Enter_Cell return code for GAWALL | gamemd: `UnitClass::Can_Enter_Cell` (0x73F0A0) for GAWALL overlay. AMCV `MovementZone=Normal` (row 0). Passability matrix row 0, column 1 (Wall/Road ZoneType) = 2 (blocked). `Crusher=yes` at TechnoType+0xD28 only downgrades codes 4/5 (friendly unit/wall blocking), NOT code 7. GAWALL (overlay) → code ≥ 7 (Impassable). UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md §3 table confirmed: code 7 = "Head-on deadlock / impassable." | Our engine: `path_grid.is_walkable(nx, ny)` returns false for wall overlay cells — `rebuild_dynamic_path_grid` calls `grid.block_building_movement_cells(entry.rx, entry.ry, "1x1", false)` for every wall overlay (app_sim_tick.rs:879). Equivalent to code-7 hard block. `mover_is_crusher` computed but `skip_grace_period=true` regardless (terrain block path). | PASS |
| 7 | Code-7 first-encounter handler | gamemd: DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §Code 7 (lines 660-675): `if is_retry==0` (first encounter): clear `locomotor.head_to` to NullCoord, call `vtable+0x480 (StopMission)(0,1)`, return 0. Then next tick enters `HAS_DESTINATION_NO_PATH` state (Phase 2) → calls `FootClass::Find_Path` immediately if `movement_delay` expired. | Our engine: `movement_step.rs:516-574` — `layer_walkable=false` → sub-position snapped to cell center → `drive_track_state=None` → `movement_delay=0` (forced) → calls `handle_blocked_tick(skip_grace_period=true)`. `handle_blocked_tick`: first encounter → `path_blocked=true`, `blocked_delay=0` (code-7 skips grace). Since `movement_delay==0`, proceeds to `try_repath_after_block` immediately (urgency=2). Repath runs **in the same tick** as detection. | PASS — same-tick repath (fixed from prior trace) |
| 8 | Stop ticks while repathing | gamemd: Zero stop ticks in the common case. Phase 2a checks `movement_delay` timer (techno+0x640): if elapsed ≥ ticks, `Find_Path` called immediately. On the tick the code-7 fires, `StopMission` → clears destination state → next Phase 2 call re-paths. `Process_Drive_Track(is_retry=1)` chains in same tick. For non-retry first encounter the delay timer is set to 0 (or not started), so next tick re-paths without pause. The unit may complete one sub-cell track step before the block fires, then stops. | Our engine: `handle_blocked_tick` → `try_repath_after_block` runs in the SAME detection tick (urgency=2, `movement_delay=0` forced). On success, `movement_path.rs:468-470` explicitly does NOT set `movement_delay` ("Do NOT set movement_delay on successful repath"). New path starts consuming next tick. **Stop duration: 1 tick** (the detection tick completes with no movement, new path begins next tick). gamemd also stalls 1 tick at the boundary (PhaseStop before Phase 2a re-path). Net delta: 0 extra ticks vs gamemd. | PASS |
| 9 | A* start cell on repath | gamemd: `FootClass::Run_AStar` (0x4CBBA0) uses `Path_walk_directions_to_cell` (0x429780) to find current position in path_queue, then calls `AStar_pathfind_search` with source = current cell (not original origin). PATHFIND_REPATH_MIDROUTE_BLOCK_TRACE.md Stage 6 confirmed. | Our engine: `try_repath_after_block` passes `current` (entity's current `rx,ry` at detection time — approximately (55,50) or (56,50) depending on AMCV progress) as the A* start cell. `movement_path.rs:369`. | PASS |
| 10 | Zone precheck on repath A* | gamemd: `Zone_precheck` (0x42C290) runs hierarchical A* before cell-level A*. On flat map with single 1-cell wall, zone adjacency unchanged (paths north or south around wall exist) → precheck passes. | Our engine: `zone_search::find_layered_path_zoned_marker` calls `ZoneGrid` reachability check. Single wall at (57,50) does not disconnect zones. Precheck passes. | PASS |
| 11 | New path direction: north or south around (57,50)? | gamemd: A* neighbor enumeration order: N(0), NE(1), E(2), SE(3), S(4), SW(5), W(6), NW(7). PATHFIND_ASTAR_TANK_AROUND_WALL_TRACE.md Stage 6 confirmed. Cost to go north (55,50)→(56,50)→(56,49)→(57,49)→(58,49)→(59,49)→(59,50)→(60,50) = 8 steps + dir_epsilon. Cost to go south symmetrically = 8 steps + dir_epsilon. North enumerated first with lower tiebreak → **north detour preferred**. Exact path: (current_cell)→…→(56,49)→(57,49)→(58,49)→(59,49)→(60,50) or similar. Specific tie-break depends on exact AMCV position at repath time. | Our engine: `NEIGHBORS[(dx,dy); 8]` order: (0,-1)=N first, (0,+1)=S fifth. DIR_TIEBREAK[N]=1 < DIR_TIEBREAK[S]=5. North costs the same cell-count but lower tiebreak. North wins. Same enumeration and tiebreak as gamemd. | PASS |
| 12 | Path smoothing on repath output | gamemd: `Path_smooth_corners` (0x42B210) + `Path_optimize_straight_segments` (0x42B7F0) post-process. Two-pass: zigzag smoothing then straight-segment shortcutting via Can_Enter_Cell validation. Truncated to 20 entries. | Our engine: `path_smooth::smooth_layered_path` + `path_smooth::optimize_layered_path` then `truncate_layered_path(MAX_PATH_SEGMENT_STEPS=24)`. Algorithm differs from gamemd's two-pass. Exact cell sequence around wall corner may diverge (PATHFIND_ASTAR_TANK_AROUND_WALL_TRACE.md Stage 10-11 UNCHECKED). | UNCHECKED — smoothing algorithm differs from gamemd; exact post-repath path cell sequence unverified |
| 13 | Diagonal corner-cutting during repath A* | gamemd: No corner-cutting check in A* — only target cell checked via `Can_Enter_Cell`. Diagonal from (55,50) to (56,49) allowed even if (55,49) is occupied or impassable. | Our engine: `core.rs:819-851` — diagonal blocked if either flanking cardinal is impassable. PATHFIND_ASTAR_TANK_AROUND_WALL_TRACE.md Stage 7 confirmed FAIL. With (57,50) blocked and AMCV rounding the NW corner: diagonal (55,50)→(56,49) requires both (55,49) and (56,50) to be walkable. On flat open grass both are clear → no divergence for the specific single-wall 10-cell route. For a denser obstacle configuration the diagonal FAIL would be triggered. | PASS for this specific scenario (no blocking flanking cells) |
| 14 | movement_delay on successful repath | gamemd: After `Find_Path` succeeds, `Process_Drive_Track(is_retry=1)` called in the SAME tick. Movement_delay set but immediately available next tick (timer starts at 0 from successful repath). Zero-tick gap between repath and resuming movement. DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §2b, §Code 2 success path confirmed. | Our engine: `movement_path.rs:468-470` — explicit comment and code: "Do NOT set movement_delay on successful repath." `movement_delay` stays 0 after success. New path begins next tick. **This was a FAIL in the prior generic trace but has since been fixed.** | PASS (fixed since PATHFIND_REPATH_MIDROUTE_BLOCK_TRACE.md Stage 5/13) |
| 15 | Facing update during repath | gamemd: Code-7 handler clears `head_to` → `StopMission`. Facing remains as it was (East, 0x40) during the stop tick. Next tick, `Process_Drive_Track(is_retry=1)` picks up the new path's first direction and begins a turn drive-track curve from East (0x40) toward North (0x00). Facing changes gradually via drive-track interpolation at AMCV's ROT=5. No instant facing snap. | Our engine: `try_repath_after_block` → `movement_path.rs:478` — `*facing = facing_from_delta(dx, dy)` sets facing **immediately** to the new first-step direction (North=0x00 for north detour) at repath time. Then `configure_motion_after_transition` selects a drive-track curve based on old facing vs new facing — since facing was already updated to 0x00, the curve search starts from 0x00 not 0x40. This suppresses the East→North turn curve that gamemd would initiate. The unit body will snap 90° visually at the repath tick instead of smoothly turning. | FAIL — immediate facing snap vs gamemd's gradual turn-track from prior heading |
| 16 | Sound / EVA cue on repath | gamemd: No audio cue for `Can_Enter_Cell` code-7 detection. No "cannot move there" EVA for path re-routing. `StopMission` fires but does not trigger blocked VO (path_blocked_flag +0x68A is for code-2 friendly-block sound, not code-7). Silent recompute. | Our engine: No sound event emitted in `handle_blocked_tick` for code-7 (`skip_grace_period=true`). `DebugEventKind::Blocked` logged only. No audio. | PASS |
| 17 | PathGrid rebuild lag on multi-tick frames | gamemd: `Can_Enter_Cell` reads live CellClass state every tick. No frame-rate lag. If `Unlimbo` runs on tick T and movement runs on tick T+1 within the same binary frame, wall is visible on tick T+1. | Our engine: `rebuild_dynamic_path_grid` runs once per render frame AFTER ALL sim ticks complete. If 2+ sim ticks run in one render frame (accumulator > 2×SIM_TICK_MS), the tick that places the wall and the tick that detects the block both use the PRE-WALL PathGrid. The block detection is delayed by one additional tick. **Frequency: every render frame that accumulates ≥2 sim ticks.** | DRIFT — multi-tick frame causes 1+ tick extra detection lag (extends Stage 4 DRIFT for multi-tick scenarios) |
| 18 | Arrival at (60,50) after detour | gamemd: AMCV follows repath, enters (60,50). `Distance3D(current-destination) < CloseEnough` (2.25 cells = 576 leptons) → mission complete. StopMission fires. No facing change at arrival — AMCV retains last drive-track facing (approximately North or East depending on final approach direction). | Our engine: `process_cell_crossings` → path exhausted at goal → `finished_entities.push(entity_id)`. Locomotor state → Idle. Facing at arrival = last drive-track update. Path consumed normally via `next_index`. | PASS |

---

## gamemd.exe Mechanism Summary (verified from binary docs)

**Wall placement type:** GAWALL is `Wall=yes` — planted as overlay by `BuildingClass::Unlimbo`
@ `0x00440580` (type+0x1571 gate). No BuildingClass entity for walls. Cell gets overlay data in
CellClass only. Verified: WALL_PLACEMENT_AND_PROTECTWITHWALL_GHIDRA_REPORT.md §5.

**Detection function:** `DriveLocomotionClass::Process_Movement` (0x4B2630) Phase 4.
Calls `Can_Enter_Cell` (vtable+0x1AC = 0x73F0A0 for AMCV/UnitClass) on next path cell each tick.
AMCV MovementZone=Normal → passability row 0, col 1 (Wall/Road) = 2 (blocked).
GAWALL overlay → code 7 (Impassable). Crusher flag at +0xD28 only affects codes 4/5, not 7.

**Code 7 first-encounter (no retry):**
- Clear `locomotor.head_to` to NullCoord (offsets 0x40, 0x44, 0x48).
- If tether_target != 0: `FootClass::Stop_Moving()` + vtable+0x484; else vtable+0x480 (StopMission)(0,1), return 0.
- No `blocked_delay` timer started (code-7 skips grace period).
- Next tick: Phase 2 (HAS_DESTINATION_NO_PATH) → `Find_Path` called immediately if movement_delay==0.

**Zero extra ticks (common case):** movement_delay at +0x640 is 0 or expired → Find_Path called on tick T+1. On success, `Process_Drive_Track(is_retry=1)` chains in same tick. Unit resumes without visible pause beyond the one stop tick at code-7 boundary.

**Facing during repath:** gamemd starts a turn drive-track curve from current facing (East=0x40) toward the new direction (North=0x00) using the standard drive-track table. No instant facing snap. Turn completes over ROT-rate ticks.

**North vs south:** A* runs north-first. With symmetric cost, north detour is always preferred in gamemd as in our engine.

---

## AMCV-Specific Notes vs Generic Tank

1. **Crusher=yes does not change repath behavior for GAWALL.** The Crusher flag at `TechnoType+0xD28` only downgrades codes 4/5 (friendly entity on cell). GAWALL is code-7 regardless. Source: UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md §3.

2. **ROT=5 makes the facing FAIL more visible.** A tank with ROT=2 would show a coarser snap. AMCV with ROT=5 has a finer turn — the 90° instant snap at repath is more obviously wrong vs a smooth drive-track turn.

3. **MovementZone=Normal.** Same as Grizzly Tank. GAWALL is code-7 impassable for both. This trace is AMCV-specific in scenario but the pathfinding mechanism is identical to the generic-tank case from PATHFIND_REPATH_MIDROUTE_BLOCK_TRACE.md, except for the facing FAIL.

---

## Verdict Tally

**PASS: 10 | FAIL: 2 | DRIFT: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0**

| Verdict | Stages |
|---------|--------|
| PASS | 1, 2, 5, 6, 7, 8, 9, 10, 11, 14, 16, 18, 13 (scenario-specific) |
| FAIL | 15 (facing snap on repath) |
| DRIFT | 4 (0–1 tick detection lag per render frame), 17 (multi-tick frame extends lag) |
| UNCHECKED | 12 (path smoothing exact sequence) |

---

## Ranked Player-Visible Failures

### 1. [Stage 15] Instant facing snap on repath — FAIL

**Player sees:** When AMCV detects the wall and recomputes route, its body rotates instantly 90°
(East→North) in a single tick rather than doing a smooth turn-track curve. For AMCV (ROT=5, large
slow vehicle), this is a visible snap that breaks the "heavy vehicle slowly turns" feel.

**Trigger frequency:** Every time a wall or building is placed in an active MCV's path, once
per repath event. In normal skirmish play this fires every time the player wall-rushes around
a moving MCV — a plausible tactical scenario.

**File:line:** `src/sim/movement/movement_path.rs:478` — `*facing = facing_from_delta(dx, dy)` sets
facing unconditionally at repath. Should set `facing_target` for vehicles with ROT>0 and let the
drive-track selection handle the turn from the previous facing.

**gamemd evidence:** DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md §Code 7 — facing unchanged by code-7
handler. New drive-track selected from old facing toward new direction by `Process_Drive_Track(is_retry=1)`.

---

### 2. [Stage 4 / Stage 17] PathGrid detection lag: 0–1 ticks per render frame — DRIFT

**Player sees:** When the wall is placed and the AMCV is 1–3 cells from it, the AMCV may
advance one cell toward the wall before the PathGrid is updated. On multi-tick-per-frame
renders (fast-forward, frame accumulation), the lag can be 2+ cells. The AMCV appears to
briefly walk toward the wall before pivoting.

**Trigger frequency:** Every wall placement while a unit is within PathGrid-detection range.
Worse on multi-tick frames (frame-rate drops, speed-up mode). In standard 15fps × 1tick/frame
play, lag = 0 ticks (PathGrid rebuilt before next frame's movement tick). Noticeable in
fast-forward modes.

**gamemd mechanism:** `Can_Enter_Cell` reads live CellClass data on every tick — zero lag.

**Files:**
- `src/app_sim_tick.rs:758-760` — `rebuild_dynamic_path_grid` after tick loop.
- `src/app_sim_tick.rs:266-675` — tick loop (placement runs inside, movement runs inside too).

**Note:** At standard 1-tick-per-frame, the lag is actually 0 because the PathGrid is rebuilt
at the END of the prior frame, so the movement tick in the NEXT frame uses the updated PathGrid.
The DRIFT is real only when the wall placement tick and the movement tick are in the SAME frame
(multi-tick frames), OR when the AMCV crosses the cell boundary on the same tick the wall appears.

---

### 3. [Stage 12] Path smoothing: exact cell sequence unverified — UNCHECKED

**Player sees:** Post-repath route around the wall may take a different number of steps or use
a different corner cell than gamemd. Smoothing algorithm differs (our two-pass vs gamemd's
direction-delta + straight-segment shortcutting). For a single 1-cell wall detour, likely
produces the same visible route, but unverified.

**File:** `src/sim/pathfinding/path_smooth.rs` — not verified numerically against gamemd output.

---

## Prior Trace Status Update

`PATHFIND_REPATH_MIDROUTE_BLOCK_TRACE.md` (generic Grizzly Tank, same wall scenario) has two
stale FAILs:
- **Stage 5 (movement_delay on repath):** Marked FAIL. Now PASS — `movement_path.rs:468-470`
  explicitly does NOT set movement_delay on successful repath. This was fixed after that trace
  was written.
- **Stage 13 (drive-track continuation):** Marked FAIL. Now PASS — same fix as Stage 5.

Those verdicts should be updated in the canonical trace doc.

---

## Report File

`docs/research/traces/MCV_WALL_MID_ROUTE_REPATH_TRACE.md`

**Status: COMPLETE**
