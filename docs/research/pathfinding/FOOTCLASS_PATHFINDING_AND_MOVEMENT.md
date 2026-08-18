# FootClass Pathfinding and Movement Systems — Ghidra Report

**Source:** gamemd.exe decompilation via Ghidra MCP
**Confidence:** High — all functions decompiled and cross-referenced
**Status:** Active in standard YR skirmish (not TS-gated)

---

## 1. Architecture Overview

Movement in YR is split across three layers:

1. **FootClass** — owns the path queue, pathfinding request/storage, stuck counters, and
   destination (NavCom). Lives in the `sim/` hierarchy.
2. **Locomotion classes** (DriveLocomotionClass, WalkLocomotionClass, etc.) — COM objects
   that implement `ILocomotion`. Called via vtable pointer stored at FootClass+0x674.
   They handle per-tick position updates, drive tracks, facing changes.
3. **AStar pathfinder** — a global singleton used by FootClass::Find_Path to compute cell
   paths. Returns a PathType struct with direction steps.

The flow: `Set_Destination -> ILocomotion::Head_To_Coord -> per-tick Process -> Process_Movement -> Find_Path -> AStar -> path queue consumption -> drive track stepping`

---

## 2. FootClass Struct Layout (Movement-Related Fields)

All offsets are **byte offsets** from FootClass base (param_1 type is `int *`, so multiply
index by 4 to get byte offset):

### Path Queue and Navigation
| Byte Offset | Field[index] | Init Value | Description |
|-------------|-------------|------------|-------------|
| 0x5E0 | [0x178] | 0xFFFFFFFF | **path_queue[0]** — current direction step (0-7 = direction, 8 = tunnel, -1 = empty) |
| 0x5E4 | [0x179] | — | path_queue[1] — next direction |
| ... | ... | — | path_queue[2..23] — 24 entries total, each a direction index |
| 0x63C | [0x18F] | — | path_queue[23] — last entry |

The path queue is a 24-entry array of `int` at +0x5E0. Each entry is a **direction index**
(0-7 for the 8 compass directions, 8 for tunnel entry, -1 for "no step"). When a step
is consumed, the queue shifts left: `memcpy(+0x5E0, +0x5E4, 23*4)`.

### Movement Timers and Stuck Counter
| Byte Offset | Field[index] | Init Value | Description |
|-------------|-------------|------------|-------------|
| 0x640 | [0x190] | frame | path_timer.start_frame — when last path was computed |
| 0x644 | [0x191] | — | path_timer.cookie |
| 0x648 | [0x192] | 0 | path_timer.delay — frames to wait before re-pathing |
| 0x64C | [0x193] | 10 | **path_blocked_counter** — decremented each time blocked, when 0 give up |
| 0x668 | — | — | repath_urgency_timer.start_frame |
| 0x66C | — | — | repath_urgency_timer.cookie |
| 0x670 | — | — | repath_urgency_timer.delay (set to Rules+0x1768 = [General]PathDelay) |

### Destination and Speed
| Byte Offset | Field[index] | Description |
|-------------|-------------|-------------|
| 0x5A0 | [0x168] | cleared to 0 on Set_Destination_Internal |
| 0x5A4 | [0x169] | **NavCom** — destination target (abstract target pointer) |
| 0x558 | [0x156] | last_path_cell — CellStruct of last cell entered from path |
| 0x578 | [0x15E..0x15F] | **current_speed** (double) — 0.0 to 1.0 fractional speed |
| 0x598 | [0x166] | tether_count — nonzero means tethered to something |
| 0x674 | [0x19D] | **ILocomotion* pointer** — the active locomotor COM object |

### Other Movement State
| Byte Offset | Field[index] | Description |
|-------------|-------------|-------------|
| 0x5D4 | [0x175] | is_towing flag (for tow trucks / slave miners) |
| 0x8C | — | on_bridge flag (0 or 1) |
| 0x68A | — | is_moving_sound_playing flag |
| 0x68B | — | bridge_transition flag |
| 0x6B5 | — | is_entering_building flag |
| 0x6B6 | — | **blocked_delay** (byte; verified from UNIT_COLLISION_AND_REPATH_TRIGGERS @ 0x4df0d0) |
| 0x6B7 | — | **repath_urgency_armed** flag (byte; set on first friendly-blocker encounter, cleared on arrival and on new destination; gates Find_Path urgency=1 vs urgency=2) |

---

## 3. Pathfinding Pipeline

### 3.1 FootClass::Find_Path (0x004D3920)

Entry point for pathfinding. Called when:
- A unit needs a new path (destination set, no valid path in queue)
- A unit gets blocked and needs to re-route

**Flow:**
1. Check if destination is reachable via `Can_Enter_Cell` (vtable+0x2CC)
2. If destination is blocked (result==6, friendly unit), call `Find_Nearby_Passable_Cell`
   to find an alternate nearby cell
3. If destination is a building (result==7), look up the building and find passable cell
4. Call `FootClass::Run_AStar` to compute the actual path
5. If path found: copy direction steps into the 24-entry path queue at +0x5E0
6. Update path_timer (frame counter at +0x640, delay at +0x648)
7. (Counter reset does NOT happen here — callers reset +0x64C on success: infantry in
   WalkLocomotionClass::ProcessMovement after Find_Path returns true; vehicles do NOT
   reset on Find_Path success, only on crush / off-playfield retry in Process_Movement.)
8. If path NOT found and unit is infantry (result==1) with a tow chain, re-path each
   towed unit as well
9. If path NOT found: compute distance to dest; if > 1 cell away and not on bridge,
   call scatter/clear destination for AI units

### 3.2 FootClass::Run_AStar (0x004CBBA0)

Thin wrapper that:
1. Gets unit's center coord via vtable+0x4C
2. Calls `Path_walk_directions_to_cell` to compute start cell from existing partial path
3. Calls `AStar_pathfind_search` with the unit, start/end cells, zone info

### 3.3 AStar_pathfind_search (0x0042C900)

The top-level A* dispatcher:
1. Calls `PathfinderClass::Reset` to clear the open/closed sets
2. Resolves start and end cells accounting for bridges (`MapClass::ResolvePathCoord_BridgeAware`)
3. Gets zone IDs for start and end cells
4. If zones match (same connected region) AND hierarchical pathing was already enabled (a flag
   computed earlier from `TypeClass+0xC94`/`param_4+0x3D5` and both cells being in the playfield):
   calls `Zone_precheck` to validate; if it **fails**, hierarchical is **disabled** for this search
   (not enabled — `Zone_precheck` can only turn hierarchical off here, never on) and a
   "Hierarchical findpath failure" debug message is logged
   (corrected 2026-07-12: was "enables hierarchical pre-check via Zone_precheck", which inverted the
   effect direction; binary shows the hierarchical-enabled flag is set before this block from
   unrelated conditions, and `Zone_precheck` failure only clears it — via
   `decompile_function 0x0042C900` - OPERATOR_OR_ORDER_DRIFT)
5. If zones differ and hierarchical check is on, **returns 0 immediately** (unreachable)
6. Enters retry loop (up to 5 attempts):
   - Calls `AStar_main_loop`
   - If fails and hierarchical enabled, calls `PathfinderClass::UpdateHierarchicalEdges` and retries
   - Decrements attempt counter; gives up after `iStack_14` iterations

**Retry count:** 5 if `param_6 == -1` (normal), 1 if explicit limit given.

### 3.4 AStar_main_loop (0x00429A90)

The actual A* search:
1. Resolves start/end cells to CellClass pointers
2. Computes start and end "height levels" from `cell+0x11B` (ground height index)
   - If on bridge (`cell+0x140 & 0x100`), adds 4 to height level
3. Stores speed type from TypeClass+0x67C into pathfinder state
4. Creates initial node for start cell
5. **Main loop** (max `param_6` iterations; when the caller passes -1, `param_6` defaults to
   `0xfff7` = 65527 inside this function — `FootClass::Run_AStar` always passes -1 through this
   chain, so 65527 is the effective default, not 10000. Separately, a found path is only accepted
   if the loop ran a number of iterations other than exactly 10000 and other than exactly
   `param_6`; hitting either value discards the result as if the search failed
   (corrected 2026-07-12: was "max 65527 iterations or param_6 limit, capped at 10000 default",
   which presented 10000 as the default iteration cap; binary shows the default cap is 0xfff7
   (65527), and 10000 is a separate exact-match rejection check unrelated to the cap — via
   `decompile_function 0x00429a90` - INFERENCE_HARDENED):
   - Pop best node from priority queue (min-heap sorted by f-cost at node+8)
   - If this is the destination cell at the right height, **path found**
   - For each of 8 neighbors + tunnel (direction 0-7, plus index 8 for tunnel):
     a. Get neighbor CellClass pointer
     b. Check bridge height compatibility
     c. Get zone index
     d. Check closed/open set membership via generation counters
     e. Call `Can_Enter_Cell` (vtable+0x1AC) to get move result (0-7)
     f. If result < 7: compute edge cost via `AStar_compute_edge_cost`, create node
     g. If result == 7 (impassable): skip unless it's the destination cell
   - Track hierarchical zone progress
6. If path found (and step count > 1):
   - Call `AStar_reconstruct_path` to build direction array
   - Call `Path_smooth_corners` and `Path_optimize_straight_segments` for path smoothing

### 3.5 AStar_compute_edge_cost (0x00429830)

Computes the cost of moving from one cell to an adjacent cell:

- **Base cost** from `g_AStar_EdgeCost_BaseTable` indexed by `Can_Enter_Cell` result
- **Result 2 (friendly blocker) special handling:**
  - Walks up to 10 cells along the blocker's predicted path (using blocker's path_queue[0]
    or timer-pseudo-random direction if the blocker has velocity)
  - If the blocker's path leads to an empty cell within 10 steps: exits loop early via
    `goto AStar_cost_predict_blocker_clears` → cost = base table value (1.0) unless urgency==2
  - If blocker is stationary with no path (`path_queue[0] == -1`): same early-exit path,
    cost = 1.0 (base table value), overridden to 1000.0 if urgency==2
  - If the 10-step prediction loop exhausts without finding an empty cell (traffic jam):
    falls through to `param_5 = 4.0` (verified via `decompile_function 0x00429830`)
  - If `urgency == 2`: cost = 1000.0 (force re-route around); overrides the 4.0 too
  <!-- MISLEADING correction 2026-05-28: original doc said "stationary blocker cost = 1.0" and "traffic jam cost = 4.0" without explaining the two distinct code paths. Clarified: stationary (no path) exits the loop early and keeps base-table cost; 4.0 is the loop-exhaustion (traffic jam) fallthrough before the urgency==2 override. -->
- **Ice cell penalty:** if cell+0x140 bit 0x40000 set, multiply cost by constant
- **Bridge corner avoidance:** when navigating on bridges, checks adjacent bridge cells
  and applies directional penalties to avoid sharp turns

### 3.6 Path Reconstruction and Smoothing

`AStar_reconstruct_path` (0x0042AA90):
- Walks backwards through parent pointers to build direction array
- Each step encoded as direction 0-7 (computed from cell coordinate deltas)
- Direction 8 used for tunnel traversal
- Stores result in global PathType struct at 0x0089A2D8

`Path_smooth_corners` (0x0042B210): Removes unnecessary zigzag steps.
`Path_optimize_straight_segments` (0x0042B7F0): Collapses straight-line sequences.

---

## 4. Movement State Machine

### 4.1 FootClass::AI (0x004DA530) — Per-Tick Update

Called every frame for each FootClass-derived object. Key movement operations:

1. **TechnoClass::AI** parent call
2. **Tiberium self-heal**: every `Rules+0x1808` frames, heal if on tiberium
3. **Locomotion::Process**: calls `ILocomotion::Process` (vtable+0x40) — this is where
   DriveLocomotionClass::Process or WalkLocomotionClass::Process runs
4. **Movement counter tracking**: increments `movement_counter` at +0x538 each frame
   the locomotor reports movement
5. **Sound effects**: manages driving/crushing/deploying sounds based on state transitions
6. **Idle scatter**: every 64 frames (`g_CurrentFrameCounter & 0x3F == 0x3F`), if:
   - Not navigating (NavCom == 0)
   - Not on bridge
   - Not currently occupied (IsOccupied check)
   - Current speed is 0
   Then calls `Scatter` with zero coord to do an idle shuffle
   <!-- AUDIT NOTE 2026-05-28: Audit claimed mask should be 0x8000000f (16-frame period). Re-verified via decompile_function 0x004DA530: the 0x8000000f mask is a separate earlier check (IPiggyback/movement); the scatter guard at the bottom of FootClass__AI uses (byte)g_CurrentFrameCounter & 0x3f == 0x3f (64-frame period). Doc is CORRECT. -->
7. **IPiggyback swap**: checks if locomotor wants to hand off to a piggybacked locomotor
8. **TryEnterTransport**: if unit is near a transport, attempt boarding

### 4.2 DriveLocomotionClass::Process (0x004B0500)

The main orchestrator for vehicle movement. Called each tick from FootClass::AI.

**State machine:**
1. **Height change detection**: checks current cell height vs stored height, starts
   transition timer if changed (3-frame slope transition)
2. **If drive track active** (track_index at loco+0x58 != -1 AND has HeadTo coord):
   - Call `Process_Drive_Track` to step along the track
   - If track completes and unit still needs to move, call `Process_Movement`
3. **If no drive track active**:
   - Check if NavCom target is at current position → stop moving
   - Check if mission is "move" and already at destination → stop
   - Wait for slope transition timer if active
   - Call `Process_Movement` to pick next cell and start a new track

### 4.3 DriveLocomotionClass::Process_Movement (0x004B2630)

~8500 bytes, the most complex movement function. Handles:

**Phase 1: Validate current state**
- If not moving and no path step: clear HeadTo, return
- If destination coord is null: return
- If deploying or undeploying: return
- If tethered and in dock: return 1 (wait)
- If being warped/chronoed: return 1

**Phase 2: Need new path?**
If path_queue[0] == -1 (no current step):
- Check **path timer** (+0x640/+0x648): if delay hasn't expired, return 0
- Call `FootClass::Find_Path` with destination cell
- If path found: continue (note: vehicle path does NOT reset path_blocked_counter here;
  the counter is only reset in Process_Movement's crush / off-playfield error handler)
- If path NOT found:
  - If close to destination and mission is Move or Follow: stop and clear dest
  - Otherwise: pick random adjacent direction, try scatter

**Phase 3: Try to enter next cell**
With a valid path_queue[0]:
- If NavCom target is nearby (< 0x18 cells) and within 1 cell of target's position:
  truncate path at that point
- Compute target coord from current position + direction delta
- Check `Can_Enter_Cell` result for target cell:

| Result | Name | Action |
|--------|------|--------|
| 0 | Clear | Proceed — compute speed, select drive track |
| 1 | Crushable | Mark redraw, if urgent retry without step, else stop |
| 2 | Friendly | Repath with urgency (see below) |
| 3 | Crush target | Call `Check_Crushable_Obstacle`, set blocked counter = 10 |
| 4-5 | Blocked/Occupied | Find blocker, attack if enemy; if friendly, try to path around |
| 6 | Occupied by ally | Scatter the occupant, set blocked counter = 10 |
| 7 | Impassable | Clear path, stop |

**Phase 4: Speed computation**
For a clear cell:
- Look up `g_SpeedType_LandType_Table[speed_type + land_type * 9]` for base speed
- Apply slope modifiers from `Rules+0x768..0x780`:
  - Going downhill: `WheelDownhillBrake` or `TrackDownhillBrake`
  - Going uphill: `WheelUphillBrake` or `TrackUphillBrake`
- If speed == 0.0, clamp to 0.5
- If health <= `Rules+0x1700` (ConditionYellow threshold), multiply by damage factor
- Set unit speed via vtable+0x544

**Phase 5: Drive track selection**
- Compute track index from `current_direction * 8 + next_direction`
- Look up in `g_DriveTrackIndex_Table` (72 entries, each 12 bytes)
- If the TurnTrack flags mask `0x08` is clear, shift the path queue left by one step immediately; if
  `0x08` is set, validate the next-next cell first and shift only on the successful chaining path
  (corrected 2026-06-01: was "advance path flag (bit 8) shifts path queue"; binary shows the
  `g_DriveTrackFlags_Table[track_index * 12] & 8` branch performs next-cell validation while the
  clear-mask branch shifts one entry via decompile_function 0x004B2630 - OPERATOR_OR_ORDER_DRIFT)

**Phase 6: Apply occupancy**
- Store next cell into `last_cell` at +0x558
- Clear movement sound flag
- Apply track delta via `DriveLocomotionClass::Apply_Track_Delta`

### 4.4 DriveLocomotionClass::Process_Drive_Track (0x004B0F20)

Steps along a pre-computed drive track:

1. **Speed management**: adjusts current speed toward target speed with acceleration/
   deceleration from TypeClass+0x300/0x308. Brakes near obstacles or when deploying.
2. **Movement budget**: accumulates `speed * GetCurrentSpeed()` per tick as movement points.
   Each track step costs 7 movement points.
3. **Per-step processing**:
   - Read dx/dy from `g_DriveTrackData_Array` (16-byte entries)
   - Transform via `DriveLocomotionClass__Transform_Track_Coords`
   - Update unit position
   - Handle bridge transitions (detect height level changes)
   - Handle crushable objects on the path
4. **Track completion**: when dx==0 && dy==0 and step > 0:
   - Finalize position, update cell occupancy
   - Clear drive track state
   - Check if NavCom target reached
   - Call `Notify_Arrival` (vtable+0x504)

### 4.5 WalkLocomotionClass::ProcessMovement (0x0075AEC0)

Infantry-specific movement processing, similar structure but simpler:

**If no HeadTo coord set (idle):**
- If no path in queue: check path timer, call Find_Path
- If path found: set blocked_counter = 10, try next step
- Path_queue[0] == 8: tunnel/tube handling (see OPEN QUESTION below)
- Otherwise: compute next cell from direction, check Can_Enter_Cell

> **OPEN QUESTION — Tube branch TS-legacy vs. live-YR (2026-05-28):**
> Binary fact (verified via `decompile_function 0x0075AEC0`): when `path_queue[0] == 8`,
> `WalkLocomotionClass::ProcessMovement` branches to code that reads `g_TubeArray` with
> **no YR-specific gate** (no SpecialFlags check, no INI flag, no conditional disable).
> The branch is structurally live.
>
> Unresolved contradiction: project rule `feedback_no_tunnel_subterranean.md` flags
> tunnel/subterranean as TS legacy not present in RA2/YR and says to skip it. However,
> a sibling UNITCLASS audit found `TubeMovement` called from `UnitClass::AI` with no gate,
> and bridge tubes (surface tube overlays on bridges) are a known YR feature that *does*
> use `g_TubeArray`. It is unclear whether `path_queue[0] == 8` refers exclusively to
> TS-era underground tunnels (skip) or also covers YR bridge tubes (implement).
>
> **Do not implement this branch until the user resolves the question.** Both sides of
> the evidence are documented here; the decision on scope is the user's to make.

**Can_Enter_Cell results (infantry):**
| Result | Action |
|--------|--------|
| 0 | Call `FindSubCellDest` for sub-cell positioning, start walking |
| 2 | Set repath_urgency_armed, start urgency timer, re-path |
| 3 | Crush obstacle, set blocked_counter = 10 |
| 4-5 | Find blocker, attack if enemy |
| 6 | Scatter occupants from target cell |
| 7 | Clear path, stop |

**If HeadTo coord set (walking):**
- Compute distance to target coord
- If < 17 leptons: **arrived at cell**
  - Shift path queue left
  - Update cell occupancy
  - Check if at final destination → stop
  - Otherwise: continue to next step
- If >= 17 leptons: **still walking**
  - Compute facing from current pos to HeadTo
  - Set facing via locomotor
  - Compute intermediate position along path
  - Handle bridge crossings

---

## 5. Obstacle Handling and Retry Logic

### 5.1 The path_blocked_counter (+0x64C)

- **Init value:** 10 (set in constructor)
- **Reset to 10** (infantry, WalkLocomotionClass::ProcessMovement): after successful
  Find_Path, after `Can_Enter_Cell` result 3 (crushable obstacle)
  <!-- AUDIT NOTE 2026-05-28: Audit claimed reset fires BEFORE Find_Path. Re-verified via decompile_function 0x0075AEC0: `*(+0x64c) = 10` appears immediately after `if (cVar4 == '\\0') { ...handle failure... }` — i.e., AFTER Find_Path returns true. Doc is CORRECT. -->
- **Reset to 10** (vehicles, DriveLocomotionClass::Process_Movement): after
  `Can_Enter_Cell` result 3 (crushable obstacle), after target cell fails playfield
  check. NOT reset on successful Find_Path.
- **Decremented by 1:** each time the unit is blocked and can't proceed
- **When reaches 0:** the unit gives up on current path:
  - For vehicles: stop moving, play honk sound if applicable
  - For infantry: clear path, stop, call ILocomotion::Stop

This prevents infinite retries — a unit tries 10 times before giving up.

### 5.2 Repath Urgency System

When a unit encounters a friendly blocker (Can_Enter_Cell == 2):

1. **First encounter:** set `repath_urgency_armed` flag (+0x6B7), start urgency timer
   with `Rules.PathDelay` (Rules+0x1768)
2. **Check path timer** (+0x640): if timer not expired, wait (return)
3. **If urgency timer expired:** call Find_Path with `urgency = 2` (aggressive reroute,
   AStar uses cost 1000.0 for friendly blockers)
4. **If urgency timer NOT expired:** call Find_Path with `urgency = 1` (normal, AStar
   uses cost 4.0 for friendly blockers)
5. After re-path: update path timer with new delay

### 5.3 Scatter on Block

When a unit encounters an allied unit blocking its path (result 6):
- Call `CellClass::Scatter_Objects` on the blocking cell with `force=1`
- The bridge-awareness flag is computed: if unit is on bridge and height difference
  to blocker is >= 3 levels, pass bridge=true so only bridge-level objects scatter

### 5.4 Convoy Repath

For infantry (`result == 1` from `What_Am_I`), after finding a path, if the unit has a
tow chain (field at +0x6C8), all towed units are re-pathed to follow the leader.

---

## 6. Scatter / Flee System

### 6.1 UnitClass::Scatter (0x00743A50)

Vehicle scatter logic:
1. Check if unit can scatter (vtable+0x28C)
2. Skip if locomotor is TeleportLocomotionClass
3. Skip if scatter cooldown timer active and not forced
4. Skip if in convoy, deploying, or undeploying
5. Check if locomotor is currently moving — if not, skip

**Direction selection:**
- If source coord is null (idle scatter): pick random direction from timer
- If source coord provided (threat scatter): compute angle from threat, add random +-1

**Cell search loop (8 directions):**
- For each candidate direction: check `Can_Enter_Cell`, skip if nonzero
- Find first passable cell that doesn't cross a bridge boundary
- If `Find_Nearby_Passable_Cell` can't find anything, try any passable cell
- Set mission to Move(2) and set destination to chosen cell

### 6.2 InfantryClass::Scatter (0x0051D0D0)

Infantry scatter, similar but with additional checks:
- C4/engineer/spy infantry types always scatter on forced scatter (types 0x1B-0x1E)
- Player-controlled units don't scatter on non-forced
- Check `Rules.ScatterEnabled` (Rules+0x17ED)
- Armed infantry (has weapon) scatter more readily
- Uses sub-cell positioning for final placement
- 8-direction search with `Can_Enter_Cell` validation

### 6.3 DriveLocomotionClass::Stop_And_Scatter (0x004B4890)

Simple helper:
- If tether_count == 0: call vtable+0x480 (clear destination, pass 0,1)
- If tether_count != 0: call `FootClass::Stop_Moving`, then vtable+0x484

### 6.4 Idle Scatter (in FootClass::AI)

Every 64 frames, if unit is not navigating and has zero speed, the engine calls
`Scatter` with a zero coord vector to do a random idle shuffle. This only happens
for non-bridge, non-occupied units with no NavCom target.

---

## 7. Locomotion Interface

### 7.1 ILocomotion COM Interface

Stored at FootClass+0x674 as a COM pointer. Key vtable methods used by FootClass:

| VTable Offset | Method | Description |
|---------------|--------|-------------|
| +0x10 | Is_Moving | Returns bool if currently executing movement |
| +0x40 | Process | Per-tick update (called from FootClass::AI) |
| +0x44 | Head_To_Coord | Set movement target coordinate |
| +0x48 | Stop_Moving | Halt current movement |
| +0x4C | Set_Facing / Do_Turn | Update requested facing/turn; WalkLocomotion vtable+0x4C is `WalkLocomotionClass__Set_Facing`, DriveLocomotion vtable+0x4C is `DriveLocomotionClass__Do_Turn` (corrected 2026-06-01: was Get_Facing; binary shows setters at vtable+0x4C via read_memory 0x007F69F8, read_memory 0x007E7EB0, decompile_function 0x0075AE00, and decompile_function 0x004B0EF0 - INFERENCE_HARDENED) |
| +0x60 | Is_Powered | Locomotor power gate used by scatter callers, not a dedicated scatter-permission method (corrected 2026-06-01: was Can_Scatter; binary shows DriveLocomotion vtable+0x60 -> `LocomotionClass__Is_Powered`, returning byte +0x0C, via read_memory 0x007E7EB0 and decompile_function 0x0055A930 - RTTI_LABEL_DRIFT) |
| +0x80 | Is_Moving_Now | Returns bool if actually in motion right now |

### 7.2 IPiggyback Interface

Some locomotors support piggybacking (e.g., TeleportLocomotionClass piggybacks onto
DriveLocomotionClass after teleporting). Queried via `QueryInterface` with `IID_IPiggyback`.

In FootClass::AI, each tick checks if the piggybacked locomotor wants to take over
(IPiggyback+0x14 returns true). If so, releases the current locomotor and promotes
the piggybacked one.

### 7.3 DriveLocomotionClass Layout

DriveLocomotionClass stores its own state relative to the locomotor base:

| Offset (from loco) | Description |
|---------------------|-------------|
| +0x04 | ILocomotion vtable pointer (secondary) |
| +0x0C | back-pointer to owning FootClass |
| +0x18..0x2B | slope transition timer (CDTimerClass, 12 bytes) |
| +0x30 | destination coord X |
| +0x34 | destination coord Y |
| +0x38 | destination coord Z |
| +0x3C | head-to coord X (next cell) |
| +0x40 | head-to coord Y |
| +0x44 | head-to coord Z |
| +0x4C | movement_budget (accumulated sub-step points) |
| +0x50 | target_speed (double) — desired speed for this terrain |
| +0x58 | drive_track_index (-1 = none) |
| +0x5C | drive_track_step (current step within track) |
| +0x5E | slope_transition_active (byte) |
| +0x5F | head_to_valid (byte) — 1 when head-to coord is loaded; gates Process_Movement vs Process_Drive_Track |
| +0x60 | track_reversed (byte) — selects byte[1] (reversed) vs byte[0] (forward) of g_DriveTrackIndex_Table entry |
| +0x61 | has_arrived (byte) |
| +0x62 | on_ramp (byte) |
| +0x63 | track_delta_applied (byte) |
| +0x64 | entering_building_track (byte) — set when target cell has a `TypeClass+0x5B4 == 0xC` (lab-type) building |

### 7.4 WalkLocomotionClass Layout

| Offset (from loco) | Description |
|---------------------|-------------|
| +0x0C | back-pointer to owning FootClass |
| +0x1C | head-to coord X |
| +0x20 | head-to coord Y |
| +0x24 | head-to coord Z |
| +0x28 | next-cell coord X |
| +0x2C | next-cell coord Y |
| +0x30 | next-cell coord Z |
| +0x36 | is_walking (byte) |

---

## 8. Drive Track System

Vehicles don't move in straight lines — they follow pre-computed **drive tracks** that
give smooth curved motion between cells.

### 8.1 Track Data Structures

- **g_DriveTrackIndex_Table** (0x007E7B28): 72 entries, each 12 bytes. Indexed by
  `next_direction + current_direction * 8` (corrected 2026-06-01: was 0x7E7A28-ish and
  ambiguously `entry_direction * 8 + exit_direction`; binary shows 0x007E7A28 is the 16-entry
  raw-track metadata table and 0x007E7B28 is the 72-entry TurnTrack table via read_memory
  0x007E7A28, read_memory 0x007E7B28, and decompile_function 0x004B2630 - OFFSET_RETYPED_WRONG).
  Contains:
  - byte[0]: track data index (into g_DriveTrackData_Array)
  - byte[1]: reversed track data index
  - byte[4]: chain direction byte
  - flags at byte offset +0x08 of the 12-byte entry; mask `0x08` is the cell-crossing /
    next-cell-validation flag (corrected 2026-06-01: was "flags at byte offset 0" and
    "bit 8 = advance path flag"; binary shows flags read through `g_DriveTrackFlags_Table`
    at entry +0x08 via read_memory 0x007E7B28 and decompile_function 0x004B2630 -
    OFFSET_RETYPED_WRONG)

- **g_DriveTrackData_Array**: Track step arrays. Each step is 12 bytes: `{dx, dy, facing}`.
  A track ends when `dx==0 && dy==0 && step > 0`. Max ~30 steps per track.

### 8.2 Track Selection

`track_index = next_direction + current_direction * 8` (corrected 2026-06-01: was
`entry_dir + exit_dir * 8`; binary computes `iVar5 = uVar19 + uVar18 * 8`, where
`uVar18` is the current `path_queue[0]` direction and `uVar19` is the next/chained
direction, via decompile_function 0x004B2630 - OPERATOR_OR_ORDER_DRIFT)

If the track table entry is null (no valid track for that turn), fall back to
`current_direction * 9` (straight ahead). If the track has the `0x08` cell-crossing flag,
the next-next cell is checked for passability first; the one-step queue shift is the
clear-flag path, while the checked chaining path can shift two entries after a clear
next-next cell (corrected 2026-06-01: was "advance and check" shifts the queue directly;
binary shows clear-mask one-step shift and set-mask validation/chaining branches via
decompile_function 0x004B2630 - OPERATOR_OR_ORDER_DRIFT).

### 8.3 Track Coordinate Transform

`DriveLocomotionClass__Transform_Track_Coords` converts the track-local dx/dy deltas
into world coordinates based on the unit's current facing and position.

---

## 9. FootClass::Set_Destination_Internal (0x004D94B0)

Called when a move order is issued:

1. Clear `+0x5A0` field
2. Guard clauses — skip if:
   - Unit is being chronoed (`+0x6AD != 0`) and dest != 0
   - Unit is being warped (`+0x82 != 0`) and dest != 0
   - Unit is garrisoned (`+0x2E4 != 0`) and dest != 0
3. If has pending chrono target and dest != 0: deploy chrono warp
4. Store NavCom: `[+0x5A4] = target`
5. If clearing dest (target==0):
   - If was being chronoed and has attacktarget: clear chrono chain
   - Call `ILocomotion::Stop_Moving`
6. If setting dest:
   - Clear C1 (whatever that is)
   - Query locomotor CLSID; if WalkLocomotionClass: set path_timer delay = 1
   - Get target's coordinates via target vtable+0x4C
   - Call `ILocomotion::Head_To_Coord` with those coordinates
7. Reset repath_urgency_armed to 0
8. Set path timer to `Rules.PathDelay`
9. Reset path_timer delay to 0

---

## 10. FootClass::CanReachDestination (0x004D3810)

Quick reachability check:
1. Get unit's speed type from TypeClass+0x5B4; if -1, return true (can reach anywhere)
2. If destination is the map null cell, return false
3. Get unit's current cell
4. Get unit's bridge status (cell+0x140 bit 8)
5. Compute zone IDs for current and destination cells
6. Call `MapClass::Can_Reach_Zone` to check if same zone

---

## 11. FootClass::Stop_Moving (0x004DF0D0)

Extremely simple — just clears two fields:
```
+0x5A0 = 0
+0x5A4 = 0   (NavCom = null)
```

This does NOT stop the locomotor — it only clears the high-level destination.
The locomotor's own Stop_Moving must be called separately.

---

## 12. Key Constants and Globals

| Address | Name | Value/Description |
|---------|------|-------------------|
| Rules+0x1718 | CloseEnoughDistance | Max lepton distance to consider "arrived" |
| Rules+0x1768 | PathDelay | Frames between re-path attempts |
| Rules+0x768 | WheelUphillBrake | Speed multiplier going uphill (wheel) |
| Rules+0x770 | WheelDownhillBrake | Speed multiplier going downhill (wheel) |
| Rules+0x778 | TrackUphillBrake | Speed multiplier going uphill (track) |
| Rules+0x780 | TrackDownhillBrake | Speed multiplier going downhill (track) |
| Rules+0x1700 | ConditionYellow | Health ratio threshold for damaged speed penalty |
| Rules+0x17ED | ScatterEnabled | Whether infantry scatter is enabled |
| 0x0089F68A | g_DirectionOffsetsY | 8-entry Y offset table for 8 directions |
| g_DirectionOffsets | g_DirectionOffsetsX | 8-entry X offset table for 8 directions |
| g_SpeedType_LandType_Table | — | 9x13 float table: speed_type * 9 + land_type |
| g_DriveTrackIndex_Table | — | 72-entry track selection table |
| g_DriveTrackData_Array | — | Track step data (dx, dy, facing per step) |

---

## 13. Summary: Tick-by-Tick Movement Flow

For a vehicle moving from A to B:

1. **Set_Destination_Internal**: stores NavCom, calls ILocomotion::Head_To_Coord
2. **FootClass::AI** each tick: calls ILocomotion::Process
3. **DriveLocomotionClass::Process**: dispatches to Process_Movement or Process_Drive_Track
4. **Process_Movement** (no active track):
   - If no path: check timer, call Find_Path
   - If path exists: read direction from queue[0], validate next cell
   - If clear: compute speed, select drive track, shift queue, apply occupancy
   - If blocked: handle based on result (scatter, repath, attack, give up)
5. **Process_Drive_Track** (active track):
   - Adjust speed toward target (accelerate/decelerate)
   - Accumulate movement budget from speed
   - For each 7-point step: read dx/dy, transform to world, update position
   - On track completion: finalize position, check for arrival

For infantry, steps 3-5 are replaced by WalkLocomotionClass::ProcessMovement which uses
direct lerp movement instead of drive tracks.
