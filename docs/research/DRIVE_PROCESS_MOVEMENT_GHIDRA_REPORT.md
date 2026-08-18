# DriveLocomotionClass::Process_Movement — Complete Decompilation Report

> **[CORRECTED 2026-05-19]** Every reference to `vtable+0x484 (ScanForTarget)` in this doc is mislabeled. The slot is **post-arrival mission dispatch** (`OnArrival` → convoy dequeue → `Queue_Mission`), not a target scanner. Base impl `0x00709A40` (confirmed: called only by `FootClass__OnArrival` via `get_function_callers`). **UnitClass override `0x00738970` is WRONG** — that address is `UnitClass__Scatter_Force`, not an arrival/mission-dispatch override. Correct UnitClass vtable+0x484 override address is unknown. (corrected 2026-05-28: ROOT_CAUSE: RTTI_LABEL_DRIFT) The Drive::Process call site guards on `FootClass+0x598 != 0` (waypoint queue non-empty); when the queue is empty, vtable+0x480 (StopMission stub) fires instead. See `TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md` for the corrected analysis.

Address: `0x4b2630` to `~0x4b4766` (~8500 bytes, 1149 decompiled lines)
Source: Ghidra MCP live decompilation of `gamemd.exe`
Confidence: HIGH — full function decompiled with pagination, all branches traced

## Function Signature

```c
undefined1 __thiscall DriveLocomotionClass__Process_Movement(
    int param_1,        // ILocomotion* (this, object_base + 4)
    undefined4 param_2, // is_retry flag (non-zero = called recursively after block)
    int *param_3        // force_repath flag (set to 1 on recursive calls for codes 4/5)
);
```

**Return values:**
- `0` = no movement this tick (idle, waiting, stopped)
- `1` = movement in progress (turning, waiting for delay, track assigned)

## Pointer Conventions

`param_1` is the ILocomotion interface pointer, which is `object_base + 4`.

- `param_1 + 0xNN` = locomotor field at object_base offset `0xNN + 4`
- `*(param_1 + 0x0C)` = `FootClass*` linked techno (the game entity)
- `*(*(param_1 + 0x0C))` = techno vtable pointer
- `*(param_1 + 0x04)` = ILocomotion vtable (self, for calling Is_Moving etc.)

## Complete State Machine

### Phase 0: Pre-checks (lines 56-100)

Before entering any state, a series of guards filter out units that should not move:

```
1. path_entry = techno->path_queue[0]     // *(techno + 0x5E0)
2. is_moving = ILocomotion::Is_Moving()    // vtable+0x10 on *(param_1+0x04)
3. If NOT moving AND path_entry == -1:     → IDLE state
4. If destination == NullCoord:            → return 0 (no destination set)
5. If techno.IsDeploying():               → return 0  // vtable+0x1D4
6. If techno.IsUnloading():               → return 0  // vtable+0x1D8
7. If techno.NavQueue != 0 (offset 0x2D0) AND Tether_Count() != 0:
                                          → return 1 (tethered, wait)
8. If techno.IsFalling():                 → return 1  // vtable+0x37C
9. If techno.IsSinking():                 → return 1  // vtable+0x380
```

**Vtable calls identified:**
| Vtable Offset | Purpose | Address in Dispatch |
|---------------|---------|---------------------|
| +0x10 | ILocomotion::Is_Moving | Via *(param_1+0x04) |
| +0x1D4 | TechnoClass::Is_Deploying | Via *(*(param_1+0xC)) |
| +0x1D8 | TechnoClass::Is_Unloading | Via *(*(param_1+0xC)) |
| +0x37C | ObjectClass::IsFalling | Via *(*(param_1+0xC)) |
| +0x380 | ObjectClass::IsSinking | Via *(*(param_1+0xC)) |

### Phase 1: IDLE (no path, not moving) — lines 56-77

**Condition:** `Is_Moving() == false AND path_queue[0] == 0xFFFFFFFF`

```
1. Clear locomotor.has_active_path (offset 0x61) = 0
2. Clear head_to coord to NullCoord (offsets 0x40, 0x44, 0x48)
3. Clear locomotor.is_on_track (offset 0x63) = 0
4. Check mission via vtable+0x184 (GetMission):
   - If mission == 2 (Guard):
     → Call vtable+0x484 (ScanForTarget)(0, 1)
5. Return 0
```

### Phase 2: HAS_DESTINATION_NO_PATH (path_queue[0] == -1, destination != NullCoord)

**Condition:** `path_queue[0] == 0xFFFFFFFF` (reached via goto LAB_004b281c or fallthrough)

#### 2a. Movement Delay Timer Check (lines 107-130)

```
techno_ptr = *(param_1 + 0xC)
delay_start  = techno + 0x640    // movement_delay_start frame
delay_ticks  = techno + 0x648    // movement_delay_ticks remaining

if delay_start != -1:
    elapsed = CurrentFrame - delay_start
    if elapsed < delay_ticks:
        remaining = delay_ticks - elapsed
        if remaining != 0: return 0     // still counting down
// else: delay expired, proceed
```

**Timer structure at techno offsets:**
| Offset | Field | Purpose |
|--------|-------|---------|
| +0x640 | movement_delay_start | Frame when delay was set (-1 = no delay) |
| +0x644 | movement_delay_facing | Saved facing during delay |
| +0x648 | movement_delay_ticks | Total ticks to wait |

#### 2b. Pathfinding Call (lines 130-140)

```
// Convert destination coord to cell coordinates
dest_cell = CONCAT22(
    (short)((dest.Y + (dest.Y >> 31 & 0xFF)) >> 8),
    (short)((dest.X + (dest.X >> 31 & 0xFF)) >> 8)
)

// Store delay timer info
techno.movement_delay_start = CurrentFrame
techno.movement_delay_facing = saved_facing
techno.movement_delay_ticks = Math__ftol(...)

success = FootClass__Find_Path(dest_cell, 0, 0)
```

**FootClass::Find_Path** at `0x4d3920` (corrected 2026-05-28: was `0x4d97d0` — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT):
- `param_1`: cell coordinate (packed short x, short y)
- `param_2`: allow_through_crushable (bool)
- `param_3`: urgency (0=normal, 1=once, 2=urgent)
- Returns: bool (path found)

#### 2c. Path Failure Handling (lines 140-220)

On `Find_Path` failure:

```
1. If techno == NULL: set return to 1, return 0

2. Check vtable+0x2CC (Can_Still_Move):
   - If false → call vtable+0x480 (StopMission)(0,1), return 0
   - If destination is now NullCoord → return 0

3. Check Is_Mission_Harvest (mission == 7):
   - If harvesting → skip close-enough check, go to random scatter

4. Close-enough check:
   distance = Distance3D(current_pos - destination)
   if distance < Rules.CloseEnough (Rules + 0x1718):
     AND (mission == 2 [Guard] OR mission == 0x0B [Move]):
       → Clear head_to to NullCoord
       → If techno.tether_target (offset 0x166*4 = 0x598) != 0:
           FootClass__Stop_Moving()
           Call vtable+0x484 (ScanForTarget)(0,1)
       → Else:
           Call vtable+0x480 (StopMission)(0,1)
       → If techno.is_alive (offset 0x90): goto random scatter check
       → Else: return 0
```

**Global: `g_RulesClass_Instance + 0x1718` = `CloseEnough` (int, lepton distance)**

#### 2d. Random Scatter on Failure (lines 190-235)

When path fails and not close enough:

```
// Pick random direction
random_seed = RateTimer__Current()
random_dir = ((random_seed >> 12) + 1) >> 1 & 7     // 0-7

// Compute cell in random direction
current_pos = techno.Get_Coords()  // vtable+0x48
scatter_cell = CONCAT22(
    g_DirectionOffsetsY[random_dir] + (short)(pos.Y >> 8),
    g_DirectionOffsetsX[random_dir] + (short)(pos.X >> 8)
)

if MapClass__Is_Cell_In_Playfield(scatter_cell, 1):
    // Try Can_Enter_Cell on the random cell
    result = Can_Enter_Cell(scatter_cell, random_dir, FUN_005f5f00(...))

    if result == 3:   // crushable obstacle
        MapClass__Check_Crushable_Obstacle(techno, scatter_cell)

    elif result == 6: // terrain/building obstacle
        → Complex ally-check and bridge-height logic (see below)
```

**Bridge-height ally scatter logic (code 6 during scatter):**
```
target_cell_class = MapClass__Get_CellClass(scatter_cell)
center_coord = cell_center(scatter_cell)
ground_height = CellClass__GetGroundHeight(center_coord)
on_bridge = (ground_height + DriveHeightStep*2 < techno.coord_Z)

nearest_object = CellClass__Find_Nearest_Object(flags, on_bridge, 0)
if nearest_object != NULL AND HouseClass__Is_Ally(nearest_object):
    owner_type = techno.GetTechnoType()  // vtable+0x84
    if type.JumpJet (offset 0xC94) == 0:
        // Check close-enough and path validity
        distance = Distance3D(current - destination)
        if distance < CloseEnough AND Has_Valid_Steps() == false:
            // Check height within tolerance
            height_diff = abs(techno.Z - coord_Z)
            if height_diff < DriveHeightStep * 2:
                cell_at = CellClass__Get_Cell_At(current_coords)
                if cell.land_type (offset 0xEC) != 10:
                    // Not on bridge ramp → stop here
                    Clear head_to, stop moving
                    return 0/1

        // Check bridge scatter flag
        if cell.flags (offset 0x140) & 0x100:
            // On bridge
            unit_height = techno.Z / DriveHeightStep
            cell_height = cell.height_level (offset 0x11B)
            if abs(unit_height - cell_height) >= 3:
                scatter_with_force = true
            else:
                scatter_with_force = false
        else:
            scatter_with_force = false

        CellClass__Scatter_Objects(NullCoord, 1, 1, scatter_with_force)
```

#### 2e. Path Retry Counter (lines 280-312)

```
techno.path_stuck_counter = techno + 0x64C   // *(techno + 0x193*4)

if path_stuck_counter < 1:
    // Counter exhausted → give up
    Clear head_to to NullCoord
    if techno.tether_target != 0:
        FootClass__Stop_Moving()
        Call ScanForTarget(0,1)
    else:
        Call StopMission(0,1)
    // Check has_pending_scatter (techno + 0x68A)
    if has_pending_scatter != 0:
        CreditUpDown_Sound(1.0f, 0)    // play blocked sound
    techno.has_pending_scatter = 0
else:
    // Decrement retry counter
    path_stuck_counter -= 1
```

#### 2f. Towing/Target Abandon Check (lines 315-340)

After path failure handling, if unit is not currently moving:

```
is_moving = ILocomotion::Is_Moving()
if NOT is_moving:
    tow_target = techno + 0xAD*4  (techno + 0x2B4)
    if tow_target != 0:
        can_tow = techno.vtable+0x3AC (CanTow)(tow_target)
        if NOT can_tow:
            techno.abandon_target_flag (offset 0x688) = 1
            if techno.NavQueue (offset 0x5D4) != 0:
                FUN_006ec3a0()  // clear convoy chain
            techno.vtable+0x3C8 (SetTarget)(0)
```

Then clears head_to, track_index = -1, has_active_path = 0. Returns 0.

### Phase 3: HAS_PATH with Mission Check (lines 345-460)

**Condition:** `path_queue[0] != 0xFFFFFFFF` (valid direction entry)

#### 3a. Mission Truncation (lines 345-380)

```
// Check if unit is following a mission 1 or 15 target
target_ptr = techno.NavQueue_target (offset 0x5A4)
if target_ptr != NULL:
    what_am_i = target.WhatAmI()   // vtable+0x2C
    if what_am_i == 1 (Infantry) OR what_am_i == 15 (Vehicle):
        // Target is mobile → check if close enough to truncate path
        distance = Distance3D(current_pos - destination)
        cell_distance = (distance + (distance >> 31 & 0xFF)) >> 8
        if cell_distance < 0x18 (24 cells):
            // Truncate path by nullifying the entry at that distance
            techno.path_queue[cell_distance] = -1
            // Re-read path_entry
```

#### 3b. Stopped State (lines 390-395)

```
if path_queue[0] == 8:    // movement_state == STOPPED
    return 0
```

#### 3c. Target Cell Computation (lines 395-430)

```
// Read direction from path_queue[0]
direction = path_queue[0] & 7   // mask to 0-7

// Current position
current_coords = techno.coords (offsets 0x9C, 0xA0, 0xA4)

// Compute target cell coordinates
target_coord.X = current_coords.X + g_DirectionDeltaX_Table[direction * 8]
target_coord.Y = current_coords.Y + g_DirectionDeltaY_Table[direction * 8]
target_coord.Z = current_coords.Z   // Z unchanged initially
```

**Direction delta tables:**
| Address | Label | Format |
|---------|-------|--------|
| 0x89f6d8 | g_DirectionDeltaX_Table | 8 ints, indexed by direction*8 |
| 0x89f6dc | g_DirectionDeltaY_Table | 8 ints, indexed by direction*8 |
| 0x89f688 | g_DirectionOffsets (cell) | 8 short pairs (X,Y), indexed by direction*2 |

#### 3d. Bridge Height Resolution (lines 430-470)

```
// Get height level of current cell
src_cell = CellClass__Get_Cell_At(current_coords)
src_height = (int)src_cell.height_level (offset 0x11B)  // signed char
if techno.on_bridge (offset 0x8C):
    src_height += 4     // bridge is 4 height levels above ground

// Get height of destination cell
dst_cell = CellClass__Get_Cell_At(target_coord)
dst_height = dst_cell.height_level

// Height difference for slope calculation
height_diff = abs(src_height - dst_height)
if height_diff >= 2:
    // Significant slope — use current height for speed calc
    effective_height = src_height
    effective_land_type = 1  // forced to "rough"
else:
    // Flat or gentle — use destination cell's data
    effective_height = dst_cell.height_level
    effective_land_type = dst_cell.land_type  // offset 0xEC
```

#### 3e. Bridge Transition Detection (lines 470-480)

```
// Check if target cell has bridge flag
if (dst_cell.flags (offset 0x140) >> 8 & 1) != (uint)techno.on_bridge:
    techno.bridge_transition_flag (offset 0x68B) = 1
```

#### 3f. Facing Validation (lines 480-500)

```
// Call vtable+0x29C (CanFace / ValidateFacing)
can_face = techno.vtable+0x29C()
if NOT can_face:
    return 1    // still turning toward target direction
```

#### 3g. Crushable Obstacle Pre-check (lines 500-510)

```
// Convert target coord to cell
target_cell = coord_to_cell(target_coord)
success = MapClass__Check_Crushable_Obstacle(techno, target_cell)
if NOT success:
    return 1    // blocked by uncrushed obstacle
```

#### 3h. Facing Delta Check (lines 510-525)

```
// Compare current facing to required direction
current_facing = RateTimer__Current()   // reads body facing timer
target_facing = (short)(direction << 13)    // direction * 0x2000
facing_delta = abs(current_facing - target_facing)

if facing_delta > 0:
    // Still need to turn — issue facing command
    ILocomotion.vtable+0x4C (SetFacing)(target_facing)
    return 1
```

### Phase 4: Can_Enter_Cell Dispatch (lines 525-820)

After facing is aligned, the function calls Can_Enter_Cell on the target cell:

```
target_cell_class = MapClass__Get_CellClass(target_cell_coord)

// Disable cloaking for movement check
techno.vtable+0x124 (SetCloak)(0)   // uncloak

can_enter = techno.vtable+0x1AC (Can_Enter_Cell)(
    target_cell_class,
    direction,        // path_queue entry
    effective_height, // from height resolution
    0, 1             // additional params
)

techno.vtable+0x124 (SetCloak)(1)   // re-enable cloak
```

#### Override Logic (JumpJet / Crusher)

Before dispatching on the result code, two overrides are checked:

```
// JumpJet override: any code < 7 → treat as 0 (passable)
if can_enter < 7:
    type = techno.GetTechnoType()  // vtable+0x84
    if type.JumpJet (offset 0xC94) != 0:
        can_enter = 0

// Crusher override: codes 4 or 5 → treat as 0 if Crusher + empty cell
if (can_enter == 4 OR can_enter == 5):
    type = techno.GetTechnoType()
    if type.Crusher (offset 0xD28) != 0:
        cell_owner = target_cell.owner_index (offset 0x44)
        if cell_owner == 0:  // no building owner
            can_enter = 0
```

#### Building WalkOver Check (lines 565-580)

```
cell_building_index = target_cell.building_index  // offset 0x44
if cell_building_index != -1:
    building_type = *(DAT_00a83d84 + building_index * 4)
    if building_type.IsWalkable (offset 0x22D) != 0:
        → treat as passable (fall through to code 0 handling)
    if building_type.IsCrushable (offset 0x2A8) != 0
       AND techno_type.locomotor_id (offset 0x5B4) == 12:
        → set bVar4 = true (will be used later for bridge flag)
```

**Global: `DAT_00a83d84` = BuildingTypeClass array pointer (runtime)**

#### Code 0: OK — Proceed (lines 820+)

When Can_Enter_Cell returns 0 (passable), the function proceeds to speed computation and track selection.

#### Code 1: Special Blockage (lines 780-810)

```
// Mark objects in cell for redraw
MapClass__Get_CellClass(target_cell)
CellClass__Mark_Objects_Redraw()

if is_retry (param_2) != 0:
    // Already retrying — give up on this path entry
    techno.path_queue[0] = -1
    // Recurse with param_2=0, param_3=0
    return DriveLocomotionClass__Process_Movement(retaddr, 0, 0)

// First encounter — clear head_to and stop
Clear head_to to NullCoord
if techno.tether_target != 0:
    FootClass__Stop_Moving()
    return vtable+0x484 (ScanForTarget)(0, 1)
else:
    vtable+0x480 (StopMission)(0, 1)
    return 0
```

#### Code 2: Temporarily Blocked (lines 590-620)

```
// Set path_blocked_flag if not already set
if techno.path_blocked_flag (offset 0x6B7) == 0:
    techno.path_blocked_flag = 1
    // Start blocked delay timer
    techno.blocked_delay_start (offset 0x668) = CurrentFrame
    techno.blocked_delay_facing (offset 0x66C) = saved_facing
    techno.blocked_delay_ticks (offset 0x670) = Rules.BlockedDelay (Rules + 0x1768)

// Check movement delay timer (same logic as Phase 2a)
// Check blocked delay timer
// Determine urgency: if blocked timer expired → urgency = 2, else urgency = 1

// Re-pathfind with urgency
dest_cell = coord_to_cell(destination)
success = FootClass__Find_Path(dest_cell, 0, urgency)

if success OR Can_Still_Move():
    // Reset movement delay
    techno.movement_delay_start = CurrentFrame
    techno.movement_delay_facing = saved_facing
    techno.movement_delay_ticks = Math__ftol(...)
    return 1

// Failed → go to StopMission
goto StopMission handler
```

#### Code 3: Crushable Obstacle (lines 580-585)

```
MapClass__Check_Crushable_Obstacle(techno, target_cell)
// Then falls through to clear head_to and stop
→ goto code-default stop handler
```

#### Code 4 / Code 5: Friendly/Enemy Unit Blocking (lines 610-660)

```
if is_retry (param_2) != 0:
    // Already retrying — give up
    techno.path_queue[0] = -1
    techno.movement_delay_start = CurrentFrame
    goto LAB_004b4541:
        // Store delay info, recurse with param_2=0
        DriveLocomotionClass__Process_Movement(retaddr, 0, 0)
        return result

// First encounter
// Attempt to find blocking object and redirect
FUN_0047c5a0(...)   // find nearest blocking object in cell
→ Try Can_Enter_Cell + crush override logic
→ If enemy (code 5): call vtable+0x1F4 (Fire_At)(1, blocking_object)
```

**FUN_0047c5a0** at `0x47c5a0`: Scans the target cell's object list for blockage. Iterates the cell's object chain (offset 0xE4), looking for objects with WhatAmI == 2 (vehicle), then falls back to CellClass__Find_Nearest_Object.

#### Code 6: Terrain/Building Obstacle (lines 665-775)

```
type = techno.GetTechnoType()
if type.JumpJet (offset 0xC94) != 0:
    → goto default stop handler (JumpJet units just stop)

if is_retry (param_2) != 0:
    // Set movement_state to -1 (invalidate)
    techno.movement_state (offset 0x178*4) = -1
    techno.movement_delay_start = CurrentFrame
    goto LAB_004b4541 (delay + recurse)

// Not retry — check close-enough
distance = Distance3D(current - destination)
if distance < Rules.CloseEnough:
    // Check height tolerance
    height_diff = abs(destination.Z - techno.Z)
    if height_diff < DriveHeightStep * 2:
        // Check if standing on bridge ramp
        cell = CellClass__Get_Cell_At(current_coords)
        if cell.land_type != 10:   // not bridge ramp
            Clear head_to, stop
            return 0/1

// Not close enough — scatter
cell = MapClass__Get_CellClass(target_cell)
if cell.flags & 0x100:   // bridge present
    // Bridge scatter logic (same as 2d)
    if abs(unit_height - cell_height) >= 3:
        force_scatter = true
    else:
        force_scatter = false
else:
    force_scatter = false

CellClass__Scatter_Objects(NullCoord, 1, 1, force_scatter)
→ goto default stop handler
```

#### Code 7: Impassable (lines 660-675)

```
if is_retry (param_2) != 0:
    // Already retried — give up completely
    techno.movement_state = -1
    techno.movement_delay_start = CurrentFrame
    goto LAB_004b4541 (delay + recurse)

// First encounter
Clear head_to to NullCoord
if techno.tether_target != 0:
    FootClass__Stop_Moving()
    return vtable+0x484 (ScanForTarget)(0, 1)
else:
    vtable+0x480 (StopMission)(0, 1)
    return 0
```

#### Default Stop Handler (LAB_004b3607)

Shared cleanup for codes 2-7:

```
Clear head_to to NullCoord (offsets 0x40, 0x44, 0x48)
Clear locomotor.is_on_track (offset 0x63) = 0
```

### Phase 5: Speed Computation (lines 820-870)

After Can_Enter_Cell returns 0 (or override makes it 0):

```
// 1. Height difference check for slope classification
src_cell = MapClass__Get_CellClass(current_cell)
src_height = src_cell.height_level                    // offset 0x11B

if abs(effective_height - src_height) < 2:
    // Flat or gentle — use destination cell data
    dst_cell = MapClass__Get_CellClass(target_cell)
    final_height = dst_cell.height_level
    land_type = dst_cell.land_type                    // offset 0xEC
else:
    // Steep — use source height, force land_type = 1
    final_height = effective_height
    land_type = 1

// 2. Base terrain speed from lookup table
type = techno.GetTechnoType()    // vtable+0x84
speed_type = type.SpeedType      // offset 0x67C
base_speed = g_SpeedType_LandType_Table[speed_type + land_type * 9]
                                 // table at 0x89ea40, float[9*N]
if base_speed > 1.0:
    base_speed = 1.0             // cap at 1.0

// 3. Slope modifier (only if convoy_type == 1)
src_ground = CellClass__GetGroundHeight(current_cell)
dst_ground = CellClass__GetGroundHeight(target_cell)

if dst_ground > src_ground:                           // UPHILL
    convoy = techno.vtable+0x2C (GetConvoyType)()
    if convoy == 1:
        if speed_type == 1 (Tracked):
            base_speed *= Rules.SlopeClimb_Tracked    // Rules + 0x768 (double)
        else:
            base_speed *= Rules.SlopeClimb_Other      // Rules + 0x778 (double)

elif src_ground > dst_ground:                         // DOWNHILL
    convoy = techno.vtable+0x2C (GetConvoyType)()
    if convoy == 1:
        if speed_type == 1 (Tracked):
            base_speed *= Rules.SlopeDescend_Tracked  // Rules + 0x770 (double)
        else:
            base_speed *= Rules.SlopeDescend_Other    // Rules + 0x780 (double)

// 4. Zero-speed safety
if base_speed == 0.0:
    base_speed = 0.5                                  // prevent deadlock

// 5. Damaged-unit slowdown
health_ratio = ObjectClass__GetHealthRatio(techno)
if health_ratio <= Rules.ConditionYellow              // Rules + 0x1700 (double)
    base_speed *= 0.75                                // DAT_007e7fc0 = 0.75

// 6. Store speed
if locomotor.track_index (offset 0x58) < 0x40:       // 64
    locomotor.current_speed (offset 0x50) = base_speed  // direct store
else:
    techno.vtable+0x544 (SetSpeed)(base_speed)        // vtable call
```

**Speed table:** `g_SpeedType_LandType_Table` at `0x89ea40`
- Float array, 9 entries per SpeedType
- Indexed as: `[speed_type_index + land_type * 9]`
- SpeedType values: 0=Foot, 1=Tracked, 2=Wheeled, 3=Hover, 4=Amphibious, ...

**RulesClass slope offsets:**
| Offset | Purpose | Type |
|--------|---------|------|
| +0x768 | SlopeClimb for Tracked | double |
| +0x770 | SlopeDescend for Tracked | double |
| +0x778 | SlopeClimb for Others | double |
| +0x780 | SlopeDescend for Others | double |
| +0x1700 | ConditionYellow threshold | double |
| +0x1718 | CloseEnough distance | int (leptons) |
| +0x1768 | BlockedDelay ticks | int |

### Phase 6: Next Path Entry & Distance Check (lines 870-920)

```
// Set destination cell for speed propagation
dest_cell = coord_to_cell(target_coord)
techno.vtable+0x534 (SetDestinationCell)(dest_cell, 1)

// Read next path entry (path_queue[1])
next_entry = techno.path_queue[1]   // techno + 0x5E4

if next_entry == -1:
    // Last entry in path — check distance to final destination
    distance = Distance3D(current_pos - destination)
    if distance > 0x200 (512 leptons = 2 cells):
        // Too far — re-pathfind
        type = techno.GetTechnoType()
        dest_cell = coord_to_cell(destination)
        Find_Path(dest_cell, type.JumpJet, 0)
        if failed AND techno != NULL:
            can_move = Can_Still_Move()
            if NOT can_move:
                StopMission(0, 1)
        next_entry = techno.path_queue[1]    // re-read
        goto next_entry_check

    else:
        // Close enough — use current direction as next
        next_entry = current_direction    // uVar18

elif next_entry == 8:
    // Next is STOPPED
    next_entry = current_direction

elif next_entry == -1 OR param_3 != 0:
    next_entry = current_direction
```

### Phase 7: Bridge/Building Approach Detection (lines 920-960)

```
// Check if the NEXT-NEXT cell has a building/bridge
if next_entry != -1:
    next_next_cell = MapClass__Get_CellClass(next_cell_coord)
    // Pathfinding_update_continued() computes the cell
    if next_next_cell != NULL AND cell.building_index != -1:
        building_type = BuildingTypeArray[cell.building_index]
        if building_type.IsWalkable (offset 0x22D) != 0:
            → set can_crush_flag (locomotor offset 0x64) = 1
            → force next_entry = current_direction
        elif building_type.IsCrushable (offset 0x2A8) != 0
             AND techno_type.locomotor_id == 12:
            → set can_crush_flag = 1
        elif techno_type.locomotor_id == 12
             AND CellClass__FindFirstBuilding() != 0:
            → set can_crush_flag = 1

    if bVar4 (from earlier building crush check):
        can_crush_flag = 1
        next_entry = current_direction

    else:
        can_crush_flag = 0
```

### Phase 8: Track Selection (lines 960-1000)

```
// Compute track table index
track_index = next_entry + current_direction * 8     // 0-63 (or 0-71)

locomotor.is_reversed (offset 0x60) = 0
locomotor.track_index (offset 0x58) = track_index

// Check if this track entry has a valid normal_track
if g_DriveTrackIndex_Table[track_index * 12] == 0:   // byte at +0x00
    // No curve for this direction combo → straight line
    locomotor.track_index = current_direction * 9     // fallback

// Check if track crosses a cell boundary (flag bit 3)
if g_DriveTrackFlags_Table[track_index * 12] & 8:
    // Cell-crossing track — validate the NEXT cell too
    ...see below
```

**Track table structure (12 bytes per entry):**
| Offset | Size | Field |
|--------|------|-------|
| +0x00 | 1 | normal_track (raw track index, 0 = no curve) |
| +0x01 | 1 | short_track (for high-speed) |
| +0x02 | 2 | padding |
| +0x04 | 4 | direction (target facing 0x00-0xE0) |
| +0x08 | 4 | flags (bit 3 = cell-crossing) |

### Phase 9: Cell-Crossing Track Validation (lines 1000-1080)

If the track has flag bit 3 (cell-crossing), the function must validate that the unit can also enter the cell BEYOND the next cell:

```
// Call FUN_00481a00 to check if destination cell is occupied/special
// [ADDRESS UNVERIFIED — 0x481a00 is CrateClass__PickupDispatch, not a cell-occupancy check; correct address unknown]
cell = MapClass__Get_CellClass(target_cell)
can_occupy = FUN_00481a00(cell)   // address WRONG (see FUN_* table correction)

if can_occupy AND techno.is_falling (offset 0x81) == 0:
    // Can occupy — check further
    if techno.is_alive (offset 0x90) == 0:     // byte at [0x24]*4
        return 0

    // Compute the cell beyond (two cells ahead)
    beyond_coord.X = target_coord.X + g_DirectionDeltaX_Table[next_entry * 8]
    beyond_coord.Y = target_coord.Y + g_DirectionDeltaY_Table[next_entry * 8]
    beyond_cell = coord_to_cell(beyond_coord)

    // Can_Enter_Cell on the beyond cell
    result = Can_Enter_Cell(beyond_cell, next_entry, effective_height, 0x100000000)

    // Apply same JumpJet/Crusher overrides
    if result < 7 AND type.JumpJet: result = 0
    if (result == 4 OR result == 5) AND type.Crusher AND cell.owner == 0: result = 0
```

If the beyond-cell check fails:

```
if result != 0:
    // Can't cross — use alternative approach
    if result == 3:   // crushable
        MapClass__Check_Crushable_Obstacle(...)
    elif result == 2: // temp blocked
        → recurse with param_3 = 1
    elif result == 6: // obstacle
        → same close-enough + scatter logic as Phase 4 code 6
    elif result == 1: // special
        → mark redraw, recurse or stop
    elif result == 7: // impassable
        → clear path, delay + recurse or stop
    elif result == 4 or 5: // unit blocking
        → delay + recurse
```

If result is 0 (both cells passable):

```
// Advance the path queue (shift left by 1)
for i in 0..23:
    techno.path_queue[i] = techno.path_queue[i+1]   // copy from +0x5E4 to +0x5E0
// Last entry becomes -1

// Also set bridge transition flag
techno.bridge_transition_flag (offset 0x68B) = 1
goto FINALIZE
```

### Phase 10: Single-Cell Track (No Cell-Crossing)

If the track does NOT have flag bit 3:

```
// Advance path queue (shift left by 1)
for i in 0..23:
    techno.path_queue[i] = techno.path_queue[i+1]
goto FINALIZE
```

### Phase 11: FINALIZE — Set Head-To and Commit (lines 1090-1149)

```
// Clear any previous head_to
locomotor.head_to = NullCoord
techno.something_63c = -1                    // techno + 0x63C

// Set new head_to as cell coordinate
techno.cell_destination (offset 0x558) = coord_to_cell(target_coord)

// Clear has_pending_scatter
techno.has_pending_scatter (offset 0x68A) = 0

// Reset point_index for new track
locomotor.point_index (offset 0x5C) = 0

// Set head_to if target is valid (not NullCoord)
if target_coord != NullCoord:
    locomotor.is_on_track (offset 0x63) = 1    // mark as "on track"
    locomotor.head_to.X (offset 0x40) = target_coord.X
    locomotor.head_to.Y (offset 0x44) = target_coord.Y
    locomotor.head_to.Z (offset 0x48) = target_coord.Z

    // Check FUN_00481a00 again (cell occupancy) [ADDRESS UNVERIFIED — see FUN_* table]
    CellClass__Get_Cell_At(target_coord)
    can_occupy = FUN_00481a00()   // address WRONG (see FUN_* table correction)
    if can_occupy AND techno.is_falling == 0:
        // Start track with initial delta
        DriveLocomotionClass__Apply_Track_Delta(target_coord, 1)
        return 0

    // Unit is on ground
    if techno.is_alive (offset 0x90):
        // Clear head_to (was set but can't use it)
        locomotor.head_to = NullCoord
        locomotor.is_on_track = 0

// Final cleanup
locomotor.track_index = -1
techno.path_queue[0] = -1

// Set speed to 0
techno.vtable+0x544 (SetSpeed)(0, 0)
return 0
```

## Map Edge Retreat (movement_state == 8)

A special path reachable when the path_queue contains only entries leading off-map:

```
// At LAB_004b3282:
techno.path_stuck_counter (offset 0x64C) = 10
// Re-enters the main path with the current direction
```

This sets a high retry counter and re-enters the main HAS_PATH processing. The actual retreat waypoint logic is handled in the caller (Process_Drive_Track or the AI system).

## All FUN_* Calls Referenced

| Address | Parameters | Purpose |
|---------|------------|---------|
| `0x5f5f00` | (techno) | Compute effective height: `cell.height_level + (on_bridge ? 4 : 0)` |
| `0x47c5a0` | (cell, find_flags, ignore_self) | Find blocking object in cell: iterates object list, checks WhatAmI==2 (vehicle), falls back to Find_Nearest_Object |
| `0x481a00` | (cell, techno) | **[WRONG ADDRESS]** (corrected 2026-05-28: was described as "complex cell occupancy check (~787 lines)"; actual function at `0x481a00` is `CrateClass__PickupDispatch` — handles crate pickup rewards, entirely unrelated to cell occupancy. The real cell-occupancy check function must be located elsewhere; address is UNKNOWN. Verified via `decompile_function 0x481a00` — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT or RTTI_LABEL_DRIFT.) |
| `0x6ec3a0` | (techno) | Clear convoy chain: calls `TeamClass__Set_Convoy_Target(0)`, then iterates all linked units (via next-link at `piVar1[0x176]` = techno+0x5D8) setting target to 0 via vtable+0x3c8 and setting abandon flag at `+0x1a2*4 = +0x688`. (corrected 2026-05-28: was "calls FUN_006e9050(0)"; decompile confirms `TeamClass__Set_Convoy_Target`, not `FUN_006e9050` — ROOT_CAUSE: INFERENCE_HARDENED) |
| `FootClass__Find_Path` at `0x4d3920` | (dest_cell, allow_crush, urgency) | A* pathfinding. Fills path_queue[0..23] with directions. Returns bool. (corrected 2026-05-28: was `0x4d97d0` — that address is `FootClass__Evaluate_Target_Threat`; correct address verified via search_functions) |
| `FootClass__Stop_Moving` | (techno) | Clears NavQueue offset 0x5A0 and 0x5A4 to 0 |
| `FootClass__Is_Mission_Harvest` | (techno) | Returns mission == 7 |
| `RadioClass__Tether_Count` | (techno) | Count of active tether links (radio connections) |
| `PathType__Has_Valid_Steps` | (techno) | Checks if path_queue has any non-zero entries |
| `ObjectClass__GetHealthRatio` | (techno) | Returns Health / Type.Strength as double |
| `MapClass__Is_Cell_In_Playfield` | (cell, validate) | Bounds-checks cell coordinates against map |
| `MapClass__Get_CellClass` | (cell_coord) | Returns CellClass* for a cell coordinate |
| `MapClass__Check_Crushable_Obstacle` | (techno, cell) | Check and mark crushable objects in cell |
| `CellClass__GetGroundHeight` | (coord) | Returns terrain height at world coordinate |
| `CellClass__Get_Cell_At` | (coord) | Returns CellClass* from world coordinates |
| `CellClass__Find_Nearest_Object` | (flags, on_bridge, ignore) | Find closest object in cell |
| `CellClass__Scatter_Objects` | (coord, force, flags, bridge_force) | Scatter all objects from cell |
| `CellClass__Mark_Objects_Redraw` | () | Mark all objects in cell for rendering |
| `CellClass__FindFirstBuilding` | () | Find first building in cell's object list |
| `CoordStruct__Set` | (x, y, z) | Construct a Coord3D |
| `CoordStruct__Distance3D` | () | 3D euclidean distance |
| `HouseClass__Is_Ally` | (object) | Check if object belongs to allied house |
| `RateTimer__Current` | () | Read current timer/counter value |
| `Math__ftol` | () | Float-to-long conversion |
| `Sqrt_Approx` | (double) | Fast approximate square root |
| `CreditUpDown_Sound` | (volume, type) | Play credit/blocked sound effect |
| `DriveLocomotionClass__Apply_Track_Delta` | (coord, mode) | Apply track endpoint offset to unit pos |
| `Pathfinding_update_continued` | (direction) | Compute cell at direction offset from current |

## All Global Data Addresses

| Address | Label | Type | Purpose |
|---------|-------|------|---------|
| 0x89f688 | g_DirectionOffsets | short[16] | 8 cell-offset (X,Y) pairs for directions 0-7 |
| 0x89f6d8 | g_DirectionDeltaX_Table | int[8] | Lepton X deltas per direction (stride 8 bytes) |
| 0x89f6dc | g_DirectionDeltaY_Table | int[8] | Lepton Y deltas per direction (stride 8 bytes) |
| 0x89ea40 | g_SpeedType_LandType_Table | float[N*9] | Speed multiplier per SpeedType x LandType |
| 0x7e7b28 | g_DriveTrackIndex_Table | struct[72] | 12-byte entries: normal_track, short_track, dir, flags |
| 0x7e7b30 | g_DriveTrackFlags_Table | (offset into above) | Flags byte at +8 of each 12-byte entry |
| 0x7e7a28 | g_DriveTrackData_Array | struct[16] | 16-byte RawTrack entries: points_ptr, count, entry, jump |
| 0x8a0790 | g_NullCoord_Drive_X | int | NullCoord sentinel X (0) |
| 0x8a0794 | g_NullCoord_Drive_Y | int | NullCoord sentinel Y (0) |
| 0x8a0798 | g_NullCoord_Drive_Z | int | NullCoord sentinel Z (0) |
| 0x8a07d0 | g_DriveHeightStep | int | Height step for bridge calculations |
| 0x8871e0 | g_RulesClass_Instance | ptr | Pointer to global RulesClass |
| 0xa8ed84 | g_CurrentFrameCounter | int | Current game frame number |
| 0xa83d84 | DAT_00a83d84 | ptr | BuildingTypeClass array base (runtime) |
| 0x7e7fc0 | DAT_007e7fc0 | double | 0.75 — damaged-speed multiplier |

## All TechnoClass Field Offsets Accessed

Listed as (techno + offset), where techno = *(param_1 + 0x0C).

| Techno Offset | Type | Field | Access |
|---------------|------|-------|--------|
| +0x44 | int | building_owner_index (-1 = none) | CellClass.offset+0x44, for crush check |
| +0x68 | ?? | (via vtable) | — |
| +0x81 | byte | is_falling | Guard check |
| +0x8C | byte | on_bridge | Bridge transition detection |
| +0x90 | byte | is_alive | Multiple death checks |
| +0x9C | int | coord_X | Current position |
| +0xA0 | int | coord_Y | Current position |
| +0xA4 | int | coord_Z | Current position |
| +0x15E | double(8) | max_speed | Speed comparison |
| +0x166 * 4 = +0x598 | int | tether_target | Stop handling (array index) |
| +0x178 * 4 = +0x5E0 | int | path_queue[0] | First path direction |
| +0x179 * 4 = +0x5E4 | int | path_queue[1] | Next path direction |
| +0x193 * 4 = +0x64C | int | path_stuck_counter | Retry counter |
| +0x2B4 | ptr | tow_target (offset 0xAD*4) | Towing check |
| +0x2D0 | int | nav_queue_count | Tether guard |
| +0x558 | int | cell_destination (packed cell) | Set during finalize |
| +0x5A0 | int | stop_flag_A | Cleared by Stop_Moving |
| +0x5A4 | ptr | stop_flag_B / NavQueue target | Mission truncation |
| +0x5D4 | int | convoy_nav_queue | Convoy chain check |
| +0x5E0 | int[24] | path_queue[0..23] | Direction entries |
| +0x63C | int | something_63c | Cleared to -1 during finalize |
| +0x640 | int | movement_delay_start | Frame-based delay timer |
| +0x644 | int | movement_delay_facing | Saved facing during delay |
| +0x648 | int | movement_delay_ticks | Total delay frames |
| +0x64C | int | path_stuck_counter | Decremented on retry |
| +0x668 | int | blocked_delay_start | Code 2 blocked timer |
| +0x66C | int | blocked_delay_facing | Code 2 facing |
| +0x670 | int | blocked_delay_ticks | Code 2 duration |
| +0x674 | ptr | locomotor_ptr | Convoy chain |
| +0x688 | byte | abandon_target_flag | Set when tow fails |
| +0x68A | byte | has_pending_scatter | Scatter sound flag |
| +0x68B | byte | bridge_transition_flag | Set on bridge boundary |
| +0x6B7 | byte | path_blocked_flag | Set on code 2 |

## All TechnoTypeClass Offsets (via vtable+0x84)

| Type Offset | Type | INI Key | Purpose |
|-------------|------|---------|---------|
| +0x5B4 | int | (locomotor_id) | == 12 for DriveLocomotionClass |
| +0x67C | int | SpeedType | Index into speed table |
| +0xC94 | byte | JumpJet | Override Can_Enter_Cell codes < 7 |
| +0xD28 | byte | Crusher | Override Can_Enter_Cell codes 4/5 |

## All Vtable Calls

| Vtable Offset | Method | Called On |
|---------------|--------|----------|
| +0x10 | Is_Moving | ILocomotion (self) |
| +0x2C | WhatAmI / GetConvoyType | TechnoClass |
| +0x48 | Get_Coords | TechnoClass |
| +0x4C | Set_Facing (ILocomotion) | ILocomotion (self) |
| +0x84 | GetTechnoType | TechnoClass |
| +0x88 | GetObjectType | ObjectClass (for health) |
| +0xF0 | EnterCell (Mark mode 1/3) | TechnoClass |
| +0xF4 | ExitCell (Mark mode 0) | TechnoClass |
| +0x124 | SetCloak | TechnoClass |
| +0x184 | GetMission | TechnoClass |
| +0x1AC | Can_Enter_Cell | TechnoClass |
| +0x1BC | Get_Current_Cell | TechnoClass |
| +0x1D0 | Get_Height | TechnoClass |
| +0x1D4 | Is_Deploying | TechnoClass |
| +0x1D8 | Is_Unloading | TechnoClass |
| +0x29C | CanFace / ValidateFacing | TechnoClass |
| +0x2CC | Can_Still_Move | FootClass |
| +0x37C | IsFalling | ObjectClass |
| +0x380 | IsSinking | ObjectClass |
| +0x3AC | CanTow | TechnoClass |
| +0x3C8 | SetTarget | TechnoClass |
| +0x480 | StopMission | TechnoClass |
| +0x484 | ScanForTarget | TechnoClass |
| +0x534 | SetDestinationCell | TechnoClass |
| +0x544 | SetSpeed | TechnoClass |

## CellClass Field Offsets

| Offset | Type | Field |
|--------|------|-------|
| +0x44 | int | building_owner_index (-1 = none) |
| +0xE4 | ptr | first_object (linked list head) |
| +0xEC | int | land_type (0-15, 10 = bridge ramp) |
| +0x11B | char | height_level (signed) |
| +0x140 | uint | flags (bit 8 = bridge present) |

## Recursive Calls

The function calls itself in three scenarios:

1. **Code 1 (special blockage):** Clears path_queue[0] to -1, recurses with `param_2=0, param_3=0`
2. **Code 2 (temp blocked):** Recurses with `param_3=1` (force_repath)
3. **Codes 4/5 (unit blocking):** Sets delay, recurses with `param_2=0, param_3=0`
4. **Codes 4/5 (second cell):** Delay + recurse after clearing path entry
5. **Code 7 (impassable, retry):** Delay + recurse
6. **Final fallback (codes 4/5 after full failure):** Recurse with `param_2=param_2, param_3=1`
