# DriveLocomotionClass::Process (0x4B0500) -- Full Decompilation Analysis

> **[CORRECTED 2026-05-19]** vtable+0x484 is described here as "Scatter_Force / Forced scatter variant." The Ghidra label `Scatter_Force` is the function name on the UnitClass override (`0x00738970`), but the *semantic* is **post-arrival mission dispatch**: it calls `FootClass::OnArrival` (tether-queue dequeue) → convoy-dequeue helper → `Queue_Mission` (Guard / Hunt / Idle depending on unit state). It is not a scatter operation. The call fires only when `FootClass+0x598 != 0` (waypoint queue non-empty); otherwise vtable+0x480 (StopMission stub) is called. See `TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md` for the corrected analysis.

**Address:** `0x004B0500`
**Size:** 485 instructions, 95 basic blocks, cyclomatic complexity 77
**Vtable slot:** ILocomotion slot 16 (voff +0x40)
**Called by:** `FootClass::AI` (0x4DA530) every tick
**Similar function:** `ShipLocomotionClass::Process` (0x69FC10) -- 86.5% similarity
**Confidence:** 95% -- all branches traced through both decompilation and disassembly

## Pointer Convention

Throughout this analysis:
- **ESI** = ILocomotion interface pointer (`object_base + 4`), passed as `param_1`
- **EDI** = `ESI - 4` = IUnknown / object base pointer
- **techno** = `*(ESI + 0x8)` = `*(object_base + 0xC)` = linked FootClass/TechnoClass

Field offsets prefixed `loco+` are from the IUnknown object base (EDI).
Field offsets prefixed `techno+` are from the linked TechnoClass pointer.

## Object Field Offsets Used (from object base)

| Object Offset | ESI-relative | Type | Field |
|---------------|-------------|------|-------|
| +0x18 (EDI+0x18) | ESI+0x14 | int | slope_timer (CDTimerClass base) |
| +0x1C (EDI+0x1C) | ESI+0x18 | int | **cached_slope_index** (CellClass+0x11C) |
| +0x20 (EDI+0x20) | ESI+0x1C | int | **previous_slope_index** |
| +0x24 (EDI+0x24) | ESI+0x20 | int | slope_timer_start_frame |
| +0x28 (EDI+0x28) | ESI+0x24 | int | slope_timer_remaining |
| +0x2C (EDI+0x2C) | ESI+0x28 | int | slope_timer_total |
| +0x30 (EDI+0x30) | ESI+0x2C | int | slope_interpolation_steps (always 3) |
| +0x34 (EDI+0x34) | ESI+0x30 | Coord3D | destination (X,Y,Z) |
| +0x40 (EDI+0x40) | ESI+0x3C | Coord3D | head_to / next waypoint (X,Y,Z) |
| +0x4C (EDI+0x4C) | ESI+0x48 | int | residual_ticks (movement budget leftover) |
| +0x50 (EDI+0x50) | ESI+0x4C | double | current_speed (8 bytes, spans +0x50 to +0x57) |
| +0x58 (EDI+0x58) | ESI+0x54 | int | track_index (-1 = none) |
| +0x5C (EDI+0x5C) | ESI+0x58 | int | point_index (step within track) |
| +0x62 (EDI+0x62) | ESI+0x5E | byte | was_waiting_flag |
| +0x63 (EDI+0x63) | ESI+0x5F | byte | is_on_track |

## TechnoClass Fields Accessed

| Techno Offset | Type | Field | Context |
|---------------|------|-------|---------|
| +0x00 | ptr | vtable | All vtable calls |
| +0x81 | byte | is_falling | Death/abort check after timer |
| +0x8C | byte | on_bridge | Scatter anim guard |
| +0x8D | byte | is_sinking | Death/abort check after timer |
| +0x90 | byte | is_alive | Continuation checks |
| +0x9C | Coord3D | position (X,Y,Z) | Arrival detection |
| +0xAC | int | current_mission | Mission==5 (Move) check |
| +0x3CD | byte | is_decelerating | Idle path -- clear waypoints |
| +0x3D5 | byte | is_on_map | Reachability check gate |
| +0x598 | int | tether_target | Stop-or-scatter decision |
| +0x5A0 | ptr | NavCom | Cleared by FootClass::Stop_Moving |
| +0x5A4 | ptr | NavTarget | Arrival check, convoy dest sync |
| +0x5E0 | int | path_queue[0] | No-path detection |
| +0x6D1 | byte | deploy_flag (UnitClass) | Convoy scatter guard |

## Vtable Calls on TechnoClass (techno->vtable+offset)

| Vtable Offset | Function | Address (UnitClass) | Purpose |
|---------------|----------|---------------------|---------|
| +0x2C | GetRTTI / What_Am_I | varies | Returns RTTI type enum |
| +0x184 | GetCurrentMission | 0x5B3040 | Returns *(this+0xAC), fallback *(this+0xB4) |
| +0x18C | SetMission(id) | 0x739EC0 | Sets current mission |
| +0x1B8 | GetCell | 0x41BEA0 | Returns packed {cellX, cellY} as shorts |
| +0x1BC | GetOccupiedCell | 0x5F6960 | Returns CellClass* for current position |
| +0x2CC | CanReachDestination | 0x4D3810 | Zone map reachability check |
| +0x480 | Scatter(0,1) | 0x741970 | Full scatter logic |
| +0x484 | Scatter_Force(0,1) | 0x738970 | Forced scatter variant |
| +0x544 | SetSpeedPercentage(0,0) | 0x4D3710 | Force idle speed to zero |

## Vtable Calls on ILocomotion (loco->vtable+offset)

| Vtable Offset | Slot | Function | Address (Drive) |
|---------------|------|----------|-----------------|
| +0x10 | 4 | Is_Moving | 0x4AFB80 |
| +0x44 | 17 | Set_Destination(coord) | 0x4AFD40 |
| +0x80 | 32 | Is_Moving_Now | 0x4AFC20 |

## Vtable Calls on NavTarget (AbstractClass*)

| Vtable Offset | Function | Purpose |
|---------------|----------|---------|
| +0x2C | GetRTTI | Type check (0xB = CellClass, 0xF = AircraftClass) |
| +0x4C | GetActionCoords(&out, techno) | Get target coordinates |

## Direct Function Calls (non-vtable)

| Address | Name | Purpose |
|---------|------|---------|
| 0x46B640 | CDTimerClass::Init(duration) | Initialize timer: sets start_frame=CurrentFrame, duration=param |
| 0x4B0F20 | Process_Drive_Track(is_retry) | Inner state machine -- step through active track |
| 0x4B2630 | Process_Movement(&result,1,0) | Outer state machine -- pathfinding and track selection |
| 0x4C9480 | CDTimerClass::Remaining() | Check if drive delay timer still active |
| 0x4DF0D0 | FootClass::Stop_Moving() | Zero out NavCom (+0x5A0) and NavTarget (+0x5A4) |
| 0x421EA0 | AnimClass::Constructor(...) | Spawn scatter/wake anim |
| 0x7C8E17 | operator_new(0x1C8) | Allocate AnimClass (0x1C8 bytes) |

## Global Constants

| Address | Type | Value | Name |
|---------|------|-------|------|
| 0x8A0790 | Coord3D | {0, 0, 0} | g_NullCoord_Drive (12 bytes) |
| 0x7E2800 | double | 0.0 | g_Const_0_0 (idle speed threshold) |
| 0xA8ED84 | int | runtime | g_CurrentFrameCounter |
| 0x8871E0 | ptr | runtime | g_RulesClass_Instance |
| g_Rules+0x94 | ptr | runtime | Rules.WakeAnimType (AnimTypeClass* for water wake) |

---

## Complete Control Flow

The function has two major branches based on `track_index`:
- **track_index != -1 AND is_on_track != 0**: Active track path (Process_Drive_Track first)
- **track_index == -1 OR is_on_track == 0**: No active track (idle/pathfinding path)

### PHASE 1: Slope Transition Detection (0x4B0500 -- 0x4B0559)

```asm
004b050b: MOV ECX,[ESI + 0x8]     ; ECX = techno
004b050e: MOV EAX,[ECX]           ; EAX = techno_vtable
004b0510: CALL [EAX + 0x1bc]      ; techno->GetOccupiedCell() -> CellClass*
004b051b: MOV CL,[EAX + 0x11c]   ; CellClass+0x11C = SlopeIndex (byte)
004b0523: MOV ECX,[EDI + 0x1c]   ; loco+0x1C = cached_slope_index
004b0526: CMP EAX,ECX            ; compare new slope vs cached
004b0528: JZ skip
```

**CRITICAL CORRECTION vs existing docs**: The existing DRIVE_LOCOMOTION_CLASS.md labels
`loco+0x1C` as "unknown" and the separate function `Update_Facing_From_Type` (0x4B04D0)
reads TechnoTypeClass+0x11C as ROT. However, Phase 1 of Process does NOT read ROT from
the type. It calls `techno->GetOccupiedCell()` (vtable+0x1BC = 0x5F6960) which returns a
**CellClass** pointer, then reads **CellClass+0x11C = SlopeIndex** (confirmed in
BRIDGE_SYSTEM.md: "Cell+0x11C is SlopeIndex set by TMP_ReadSlopeType").

```
cell = techno->GetOccupiedCell()                // vtable+0x1BC -> CellClass*
new_slope = *(byte*)(cell + 0x11C)              // CellClass.SlopeIndex

if (new_slope != loco.cached_slope_index):      // loco+0x1C
    loco.previous_slope_index = loco.cached_slope_index  // loco+0x20
    loco.cached_slope_index = new_slope                  // loco+0x1C

    // Initialize slope transition timer (3 frames)
    CDTimerClass::Init(3)                       // 0x46B640
    loco.slope_timer_start = timer.start        // loco+0x24
    loco.slope_timer_remaining = timer.remaining // loco+0x28
    loco.slope_timer_total = timer.total        // loco+0x2C
    loco.slope_interpolation_steps = 3          // loco+0x30
```

**Purpose**: Detects when the unit moves to a cell with a different terrain slope. When
detected, starts a 3-frame interpolation timer used by `Draw_Matrix` to smoothly blend
between old and new body pitch/roll angles, preventing visual snapping when units drive
onto or off of ramps.

**Corrected field names for loco object layout:**

| Offset | Old Name (DRIVE_LOCOMOTION_CLASS.md) | Corrected Name |
|--------|--------------------------------------|----------------|
| +0x1C | unknown | **cached_slope_index** |
| +0x20 | turn_timer_start_frame | **previous_slope_index** |
| +0x24 | frame_stamp | **slope_timer_start_frame** |
| +0x28 | turn_timer_remaining | **slope_timer_remaining** |
| +0x2C | turn_timer_total | **slope_timer_total** |
| +0x30 | unknown | **slope_interpolation_steps** (always 3) |

### PHASE 2: Main Branch -- Track Active vs Idle (0x4B055A)

```
if (loco.track_index != -1 AND loco.is_on_track != 0):
    goto ACTIVE_TRACK_PATH              // 0x4B0573
else:
    goto IDLE_PATH                      // 0x4B066C
```

---

## ACTIVE TRACK PATH (track_index != -1 AND is_on_track)

### Step 2A: Execute Drive Track (0x4B0573)

```
result = Process_Drive_Track(0)                 // 0x4B0F20, param=0 (not retry)

if (result != 0):                               // Track completed or still active
    goto RETURN_FALSE                           // 0x4B0AC2

if (techno.is_alive == 0):                      // techno+0x90
    goto RETURN_FALSE
```

If Process_Drive_Track returns non-zero (track step completed successfully) OR the
unit died, return false immediately.

### Step 2B: Post-Track Continuation Check (0x4B0592)

```
if (loco.track_index != -1):                    // Still have an active track
    goto IDLE_TAIL                              // 0x4B078C

// Track finished, check if there's more movement pending
is_moving = loco->Is_Moving()                   // ILocomotion vtable+0x10
if (is_moving == 0 AND techno.path_queue[0] == -1):  // techno+0x5E0
    goto IDLE_TAIL                              // 0x4B078C
```

### Step 2C: Convoy/Follow Destination Sync (0x4B05B4)

Only runs when the unit finished a track, is still moving, and has a valid path.

```
rtti = techno->GetRTTI()                        // vtable+0x2C
if (rtti == 1 AND techno.deploy_flag != 0):     // RTTI_Unit AND techno+0x6D1
    goto IDLE_TAIL                              // Don't path while deploying

param_1 = 0                                     // Clear output flag

if (rtti == 1):                                 // Is a UnitClass
    NavTarget = techno.NavTarget                // techno+0x5A4
    if (NavTarget != NULL):
        nav_rtti = NavTarget->GetRTTI()         // vtable+0x2C
        if (nav_rtti == 0xF):                   // NavTarget is AircraftClass
            target_coords = NavTarget->GetActionCoords(&temp, techno)  // vtable+0x4C
            if (target_coords != loco.destination):  // loco+0x34
                loco->Set_Destination(target_coords) // ILocomotion vtable+0x44
```

**Purpose**: When a Unit is following an Aircraft (RTTI 0xF), continuously update the
locomotor destination to track the moving target. This is the "convoy sync" that keeps
the follow unit chasing.

### Step 2D: Process_Movement + Chain to Process_Drive_Track (0x4B063D)

```
Process_Movement(&param_1, 1, 0)                // 0x4B2630

if (param_1 != 0):                              // Movement produced result
    goto RETURN_FALSE
if (techno.is_alive == 0):                      // Died during movement
    goto RETURN_FALSE

// TWO-PHASE CHAIN: immediately start the new track
uVar11 = 1
Process_Drive_Track(1)                          // 0x4B0F20, is_retry=1
goto POST_TRACK_CHECK                          // 0x4B0AAF
```

**Two-phase execution**: After the old track finishes, Process_Movement sets up the
NEXT track, then Process_Drive_Track(1) is called immediately with `is_retry=1` to
start consuming that new track in the same tick. This eliminates one-frame stalls
between track segments.

---

## IDLE PATH (track_index == -1 OR is_on_track == 0)

### Step 3A: NavTarget Arrival Check (0x4B066C -- 0x4B06D4)

```
NavTarget = techno.NavTarget                    // techno+0x5A4
if (NavTarget != NULL):
    nav_rtti = NavTarget->GetRTTI()             // vtable+0x2C
    if (nav_rtti == 0xB):                       // NavTarget is CellClass
        current_cell = techno->GetCell(&temp)   // vtable+0x1B8
        target_cell = *(CellStruct*)(NavTarget + 0x24)  // CellClass MapCoord

        if (current_cell.X == target_cell.X AND current_cell.Y == target_cell.Y):
            // ARRIVED at NavTarget cell
            if (techno.tether_target == 0):     // techno+0x598
                techno->Scatter(0, 1)           // vtable+0x480
                return false
            else:
                goto STOP_AND_SCATTER           // 0x4B0756
```

**Purpose**: If the NavTarget is a cell and the unit is in that cell, arrival is
detected. If untethered, scatter. If tethered (entering transport/building),
Stop_Moving + Scatter_Force.

### Step 3B: Mission::Move Position Arrival (0x4B06D5 -- 0x4B0774)

```
if (techno.current_mission == 5):               // Mission::Move (techno+0xAC)
    if (loco.is_on_track != 0):                 // loco+0x63
        goto DRIVE_TIMER_CHECK

    if (loco.destination == NullCoord):          // loco+0x34/38/3C
        goto DRIVE_TIMER_CHECK

    if (techno.position == loco.destination):    // techno+0x9C vs loco+0x34
        if (techno.tether_target == 0):         // techno+0x598
            techno->Scatter(0, 1)               // vtable+0x480
            return false
        else:
            goto STOP_AND_SCATTER
```

**Purpose**: For units on Move mission, check if position exactly matches destination.
This catches inter-track arrival (destination == current position).

### STOP_AND_SCATTER (0x4B0756)

```
FootClass::Stop_Moving()                        // 0x4DF0D0
techno->Scatter_Force(0, 1)                     // vtable+0x484
return false
```

### Step 3C: Drive Delay Timer Check (0x4B0775 -- 0x4B078B)

```
remaining = CDTimerClass::Remaining()           // 0x4C9480, timer at techno+0x388

if (remaining != 0):
    loco.was_waiting_flag = 1                   // loco+0x62
    goto IDLE_TAIL                              // 0x4B078C
```

If the drive delay timer is active (set by Process_Movement when blocked), set
`was_waiting_flag` and skip to idle tail. Unit is waiting for timer to expire.

### Step 3D: Post-Wait Resume (0x4B0896 -- 0x4B08CE)

```
if (loco.was_waiting_flag != 0):
    loco.was_waiting_flag = 0                   // Clear flag
    techno->SetMission(0)                       // vtable+0x18C, Mission::None

    // Death check
    if (techno.is_alive != 0                    // techno+0x90
        AND techno.is_falling == 0              // techno+0x81
        AND techno.is_sinking == 0):            // techno+0x8D
        goto MISSION_GATE                       // 0x4B08D1
    return false                                // Dead/dying
```

**Purpose**: When the drive timer expires, clear the mission (reset to None) to allow
re-evaluation. Abort if unit died while waiting.

### Step 3E: Mission Gate (0x4B08D1 -- 0x4B0900)

```
mission = techno->GetCurrentMission()           // vtable+0x184

if (mission == 5):                              // Mission::Move
    is_moving = loco->Is_Moving()               // ILocomotion vtable+0x10
    if (is_moving == 0):
        goto IDLE_TAIL                          // Nowhere to go

mission = techno->GetCurrentMission()           // vtable+0x184
if (mission == 0x10):                           // Mission::Unload (16)
    goto IDLE_TAIL                              // Don't move while unloading
```

### Step 3F: Movement Decision Fork (0x4B0903 -- 0x4B09AA)

```
is_moving = loco->Is_Moving()                   // ILocomotion vtable+0x10

if (is_moving != 0):
    goto CONVOY_REACHABILITY_CHECK              // 0x4B09AB

if (techno.path_queue[0] != -1):                // techno+0x5E0
    goto CONVOY_REACHABILITY_CHECK              // 0x4B09AB

// Truly idle -- try to acquire a destination
if (techno.is_decelerating != 0):               // techno+0x3CD
    // Clear head_to if set
    if (loco.head_to != NullCoord):             // loco+0x40
        loco.head_to = NullCoord
        loco.is_on_track = 0                    // loco+0x63
    loco.residual_ticks = 0                      // loco+0x4C (int, 4 bytes)
    goto IDLE_TAIL

else:
    // Ask NavTarget for coordinates
    NavTarget = techno.NavTarget                // techno+0x5A4
    if (NavTarget != NULL):
        target_coords = NavTarget->GetActionCoords(&temp, techno)
        loco->Set_Destination(target_coords)    // ILocomotion vtable+0x44
    goto IDLE_TAIL
```

**Purpose**: When idle with no path, either (a) clear waypoints if decelerating, or
(b) acquire a new destination from the NavTarget.

### Step 3G: Convoy Reachability Check (0x4B09AB -- 0x4B0A6A)

```
if (techno.is_on_map == 0):                     // techno+0x3D5
    goto CALL_PROCESS_MOVEMENT

mission = techno->GetCurrentMission()           // vtable+0x184
if (mission == 7):                              // Mission::Hunt
    goto CALL_PROCESS_MOVEMENT                  // Skip reachability for Hunt

is_moving = loco->Is_Moving()                   // ILocomotion vtable+0x10
if (is_moving == 0):
    goto CALL_PROCESS_MOVEMENT

// Unit is on map, not hunting, and is moving
can_reach = techno->CanReachDestination(&loco.destination)  // vtable+0x2CC
if (can_reach != 0):
    goto CALL_PROCESS_MOVEMENT                  // Reachable -- proceed

// CANNOT reach destination -- give up
if (loco.head_to != NullCoord):
    loco.head_to = NullCoord
    loco.is_on_track = 0

if (techno.tether_target == 0):                 // techno+0x598
    techno->Scatter(0, 1)                       // vtable+0x480
    goto IDLE_TAIL
else:
    FootClass::Stop_Moving()
    result = techno->Scatter_Force(0, 1)        // vtable+0x484
    if (result != 0):
        return false
    goto IDLE_TAIL
```

**Purpose**: Zone-map reachability check. If the unit's destination is in an unreachable
zone, give up immediately rather than wasting pathfinding cycles.

### Step 3H: Call Process_Movement (0x4B0A6B -- 0x4B0AA6)

```
param_1 = 0
Process_Movement(&param_1, 1, 0)                // 0x4B2630

if (param_1 != 0):
    return false                                // Movement found something
if (techno.is_alive == 0):
    return false                                // Died during movement

// Fall through to chain drive track
```

### Step 3I: Chain to Drive Track (0x4B0AA7 -- 0x4B0AC1)

```
Process_Drive_Track(0)                          // 0x4B0F20, is_retry=0

if (techno != NULL AND techno.is_alive != 0):
    goto IDLE_TAIL
else:
    return false
```

**Two-phase chaining (idle path)**: After Process_Movement sets up a track,
Process_Drive_Track(0) is called in the same tick to begin consuming it.

---

## IDLE TAIL (0x4B078C -- 0x4B0893)

Three responsibilities: wake animation, idle speed forcing, and final return.

### Wake Animation (0x4B078C -- 0x4B0827)

```
is_moving_now = loco->Is_Moving_Now()           // ILocomotion vtable+0x80

if (is_moving_now != 0):
    if (g_CurrentFrameCounter % 10 == 0):       // Every 10th frame
        if (techno.on_bridge == 0):             // techno+0x8C
            cell = techno->GetOccupiedCell()    // vtable+0x1BC -> CellClass*
            if (cell.land_type == 2):           // CellClass+0xEC == 2 (LandType::Water)
                if (Rules.WakeAnimType != 0):   // g_RulesClass+0x94
                    anim_mem = operator_new(0x1C8)
                    if (anim_mem != NULL):
                        coords = techno.position    // techno+0x9C
                        AnimClass::Constructor(
                            Rules.WakeAnimType, // g_RulesClass+0x94
                            &coords,
                            0,                  // delay
                            1,                  // loop_count
                            0x600,              // draw_flags (center sprite)
                            0,                  // z_adjust
                            0                   // reverse
                        )
```

**CORRECTION**: The user task description and some references said "SpeedType==2".
The actual binary check is `CellClass.land_type == 2` (LandType::Water), NOT SpeedType.
This spawns a water **wake** animation for amphibious vehicles/ships moving on water.
The anim type comes from `RulesClass+0x94` = the `Wake=` entry in rules.ini.

### Idle Speed Forcing (0x4B0828 -- 0x4B0885)

```
if (loco.destination == NullCoord               // loco+0x34
    AND loco.head_to == NullCoord               // loco+0x40
    AND techno.path_queue[0] == -1              // techno+0x5E0
    AND techno.max_speed > 0.0):                // techno+0x578 > 0.0 (at 0x7E2800)

    techno->SetSpeedPercentage(0, 0)            // vtable+0x544
```

**Purpose**: If the unit has no destination, no waypoint, no queued path, and its
current speed is nonzero, force speed to zero. Prevents "ghost movement" from
residual speed after all movement targets are cleared.

### Final Return (0x4B0886 -- 0x4B0893)

```
return loco->Is_Moving()                        // ILocomotion vtable+0x10
```

Returns whether the locomotor still has pending work.

### RETURN_FALSE (0x4B0AC2)

```
return false                                     // AL = 0
```

---

## Summary: Complete Branch Map

```
Process (0x4B0500)
  |
  +-- Phase 1: Slope transition detection (always runs)
  |     CellClass+0x11C changed? start 3-frame interpolation timer
  |
  +-- if (track_index != -1 AND is_on_track):
  |     |
  |     +-- [2A] Process_Drive_Track(0)
  |     |     +-- returns nonzero OR dead? -> RETURN_FALSE
  |     |
  |     +-- [2B] track still active? -> IDLE_TAIL
  |     +-- not moving AND no path? -> IDLE_TAIL
  |     |
  |     +-- [2C] Convoy sync (RTTI_Unit following RTTI_Aircraft)
  |     |     +-- deploying? -> IDLE_TAIL
  |     |     +-- NavTarget RTTI==0xF? update destination from it
  |     |
  |     +-- [2D] Process_Movement(&result, 1, 0)
  |     |     +-- result or dead? -> RETURN_FALSE
  |     |
  |     +-- Process_Drive_Track(1) [is_retry=1]
  |     +-- alive? -> IDLE_TAIL
  |     +-- dead? -> RETURN_FALSE
  |
  +-- else (no active track):
        |
        +-- [3A] NavTarget arrival (RTTI 0xB = Cell)?
        |     +-- at target cell? -> Scatter or STOP_AND_SCATTER
        |
        +-- [3B] Mission::Move + at destination?
        |     +-- position matches dest? -> Scatter or STOP_AND_SCATTER
        |
        +-- [3C] Drive delay timer active?
        |     +-- yes: set was_waiting -> IDLE_TAIL
        |
        +-- [3D] Timer just expired + was_waiting?
        |     +-- clear flag, SetMission(None)
        |     +-- dead/falling/sinking? -> RETURN_FALSE
        |
        +-- [3E] Mission::Move but !Is_Moving? -> IDLE_TAIL
        +-- Mission::Unload? -> IDLE_TAIL
        |
        +-- [3F] Not moving at all?
        |     +-- decelerating? clear waypoints -> IDLE_TAIL
        |     +-- has NavTarget? set dest from it -> IDLE_TAIL
        |
        +-- [3G] On map + !Hunt + moving?
        |     +-- zone unreachable? clear + scatter -> IDLE_TAIL
        |
        +-- [3H] Process_Movement(&result, 1, 0)
        |     +-- result or dead? -> RETURN_FALSE
        |
        +-- [3I] Process_Drive_Track(0)
        +-- alive? -> IDLE_TAIL
        +-- dead? -> RETURN_FALSE

IDLE_TAIL:
  +-- Wake anim (every 10 frames, on Water, not on bridge)
  +-- Idle speed forcing (no dest + no waypoint + no path -> speed=0)
  +-- return Is_Moving()

STOP_AND_SCATTER:
  +-- FootClass::Stop_Moving()
  +-- techno->Scatter_Force(0, 1)
  +-- return false
```

## RTTI Values Referenced

| Value | Hex | Type | Context in Process |
|-------|-----|------|--------------------|
| 1 | 0x01 | UnitClass | Convoy/follow check (is this techno a vehicle?) |
| 11 | 0x0B | CellClass | NavTarget arrival (navigating to a ground cell) |
| 15 | 0x0F | AircraftClass | NavTarget follow sync (chasing an aircraft) |

## Mission Values Referenced

| Value | Hex | Name | Context |
|-------|-----|------|---------|
| 0 | 0x00 | None | SetMission(0) after timer expires |
| 5 | 0x05 | Move | Arrival detection, "still moving?" gate |
| 7 | 0x07 | Hunt | Reachability check bypass (always path for Hunt) |
| 16 | 0x10 | Unload | Movement suppression (don't move while unloading) |

## Corrections to Existing Documentation (DRIVE_LOCOMOTION_CLASS.md)

### 1. Phase 1 is Slope Detection, NOT ROT Detection

The existing doc's field table has `loco+0x1C` as "unknown" and the Update_Facing_From_Type
function at 0x4B04D0 reads TechnoTypeClass+0x11C as ROT. Process() Phase 1 is a DIFFERENT
mechanism: it calls GetOccupiedCell (vtable+0x1BC = 0x5F6960) and reads CellClass+0x11C
(SlopeIndex), not TechnoTypeClass+0x11C (ROT).

Fields +0x1C through +0x30 on the loco object should be renamed from "turn_timer_*" to
"slope_timer_*" as they control slope transition interpolation, not turn/facing interpolation.

### 2. Scatter Animation: Water Wake, Not Dust

The anim condition is `cell.land_type == 2` (Water), not `SpeedType == 2`. The animation
comes from `RulesClass+0x94` which is the `Wake=` AnimType in rules.ini. This is a water
wake effect for amphibious/naval units, not a terrain dust cloud.

### 3. Drive Delay Timer Location

The CDTimerClass::Remaining call at 0x4B077B operates on a timer at `techno+0x388` (passed
via `ADD ECX, 0x388` before the call). This is NOT on the locomotor object -- it's on the
TechnoClass. The `was_waiting_flag` at `loco+0x62` tracks whether the locomotor was delayed
by this timer, while the timer itself lives on the techno.

### 4. Field +0x62 Purpose Correction

The existing doc labels `loco+0x62` as "deploy_flag". In Process(), it functions as
`was_waiting_flag` -- set to 1 when the drive delay timer is active, cleared when it
expires. The deploy functionality may be at a different offset or shared.

## Correction Log

**2026-04-06 — Fixed slope_timer naming; reverted incorrect current_speed offset change:**
- slope_timer_* naming corrections retained (previously turn_timer_*).
- **REVERTED** incorrect change that placed current_speed at +0x4C. The original layout
  was correct: +0x4C=int residual_ticks, +0x50=double current_speed, +0x58=track_index.
  Force_Track appeared to write a double at +0x4C because it receives `this` via the
  **ILocomotion interface** (object_base + 4), so its `[param_1 + 0x4C]` = absolute +0x50.
  Verified from constructor at 0x4af540: +0x58 init to -1 (track_index), +0x5C init to -1
  (point_index), +0x4C/+0x50/+0x54 all init to 0.
