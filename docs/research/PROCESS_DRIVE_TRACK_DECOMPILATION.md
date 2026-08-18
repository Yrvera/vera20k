# Process_Drive_Track — Full Decompilation Report

Address: `0x004b0f20` to `~0x004b2630` (~5860 bytes, 793 decompiled lines)
Function: `DriveLocomotionClass::Process_Drive_Track(int param_1, char param_2)`
Verified from: Ghidra MCP live decompilation of `gamemd.exe`, paginated in 150-line chunks.

`param_1` = ILocomotion interface pointer (object_base + 4). All offsets in decompiler
output are byte offsets from the ILocomotion `this`.
`param_2` = retry flag (0 = normal tick, nonzero = chained second call within same tick).

Called from `DriveLocomotionClass::Process` (0x4b0500) when `track_index != -1`.
Returns: 0 (finished/idle) or 1 (still moving/need more processing).

---

## High-Level Control Flow

```
1. GUARD_FAIL check          (lines 85-92)
2. DEPLOY_CHECK               (lines 93-96)
3. SPEED_COMPUTE               (lines 97-196, only when track_step < 0x40)
4. BUDGET accumulation         (lines 197-205)
5. MAP_EDGE_RETREAT handling   (lines 206-260)
6. STEP_LOOP (do..while)       (lines 261-695)
   6a. TRACK_END               (lines 304-425)
   6b. MID-TRACK STEP          (lines 426-695)
       - cell-same path        (lines 448-458)
       - cell-different path   (lines 459-695)
         * bridge ramp detect  (lines 466-481)
         * JumpJet scatter     (lines 487-524)
         * Can_Enter_Cell      (lines 536-695)
7. RESIDUAL interpolation      (lines 696-793)
```

---

## Phase 1: GUARD_FAIL (lines 85-96)

```c
if ( ((loco->is_on_track == 0 || loco->track_index == -1)
      && techno->path_queue[0] != 8)
  || (loco->deploy_flag != 0
      && technoType->deploy_while_moving == 0) )
{
    loco->residual_ticks = 0;
    return 0;
}
```

### Conditions for early-out:
- **No active track**: `is_on_track == 0` (loco+0x63) OR `track_index == -1` (loco+0x58),
  AND movement_state is not STOPPED (path_queue[0] != 8).
- **Deploy while moving blocked**: `deploy_flag` (loco+0x62) is set, but
  `technoType->deploy_while_moving` (+0xCA1) is false.

### Field references:
| Decompiler | Object | Field |
|-----------|--------|-------|
| `param_1 + 99` (0x63) | loco | is_on_track |
| `param_1 + 0x58` | loco | track_index |
| `*(param_1+0xc) + 0x5e0` | techno | path_queue[0] (movement_state) |
| `param_1 + 0x62` | loco | deploy_flag |
| via vtable+0x84 | technoType | +0xCA1 = deploy_while_moving |
| `param_1 + 0x4c` | loco | residual_ticks |

---

## Phase 2: SPEED_COMPUTE (lines 97-196)

Only executed when `loco->track_step < 0x40` (track index in the first 64 entries,
i.e. standard directional tracks, not special tracks 64-71).

### 2a. Formation leader speed logic (technoType->is_formation_leader at +0xDBD)

```c
technoType = vtable_call(techno, 0x84);  // GetTechnoType
if (technoType->is_formation_leader != 0)  // +0xDBD
{
    convoy_type = vtable_call(techno, 0x2c);  // What_Am_I
    if (convoy_type == 1 /* UNIT */
        && techno->convoy_leader_type->??? (+0x6C4)->+0xE0C != 0)
    {
        skip_deceleration = true;
    }
}
```

When formation leader, the following speed computation runs:

### 2b. Distance-to-destination and deceleration

```c
// Get current position from destination coords at loco+0x34..0x3C
dest = {loco->dest_x, loco->dest_y, loco->dest_z};  // loco+0x34/0x38/0x3C

// Get cell at destination, check bridge
cell = CellClass__Get_Cell_At(&dest);
bridge_z = (cell->flags_140 & 0x100) ? g_BridgeZOffset_Drive : 0;
ground_z = CellClass__GetGroundHeight(&dest);
dest.z = ground_z + bridge_z;

// 3D distance to techno position
dx = techno->pos_x - dest.x;    // techno+0x9C
dy = techno->pos_y - dest.y;    // techno+0xA0
dz = techno->pos_z - dest.z;    // techno+0xA4
distance = Sqrt_Approx(dx*dx + dy*dy + dz*dz);
distance_int = ftol(distance);

target_speed = techno->max_speed;            // double at techno+0x578 (via +0x15E as word-index)
decel_steps  = vtable_call(techno, 0x38c);   // get decel_steps count
```

### 2c. Deceleration logic

```c
decelerating = false;

// CASE 1: Within deceleration threshold
if (distance_int < technoType->decel_threshold)    // technoType+0x2F8
{
    decelerating = true;
    target_speed -= decel_steps * technoType->decel_rate;  // technoType+0x300
    if (target_speed < 0.3)                                // DAT_007e6240/44 = 0.3
        target_speed = 0.3;
}
// CASE 2: is_decelerating_flag set on techno
else if (techno->is_decelerating_flag != 0)        // techno+0x3CD
{
    target_speed -= decel_steps * ALT_DECEL_RATE;  // DAT_007e6250 = 0.0015
    if (target_speed < 0.1)                         // DAT_007e6248/4C = 0.1
        target_speed = 0.1;
    decelerating = true;
}
```

### 2d. Speed clamping and propagation

```c
// If is_braking (techno+0x6B5):
if (techno->is_braking != 0)
{
    target_speed = 0.2;                   // DAT_007e3548 = 0.2
    if (loco->current_speed < 0.2)
        target_speed = loco->current_speed;
    loco->current_speed = target_speed;
    techno->SetSpeed(target_speed);       // vtable+0x544
}
else if (decelerating)
{
    techno->SetSpeed(target_speed);       // vtable+0x544
}
else if (techno->max_speed < loco->current_speed)
{
    // Over max speed — apply acceleration toward max
    target_speed = technoType->accel_rate + techno->max_speed;  // technoType+0x308
    if (loco->current_speed < target_speed)
        target_speed = loco->current_speed;
    techno->SetSpeed(target_speed);       // vtable+0x544
}
else if (loco->current_speed < techno->max_speed)
{
    // Under max speed — decelerate toward max (braking from overshoot)
    target_speed = techno->max_speed - decel_steps * technoType->decel_rate;
    if (target_speed < loco->current_speed)
        target_speed = loco->current_speed;
    techno->SetSpeed(target_speed);       // vtable+0x544
}
```

### 2e. Convoy speed propagation

```c
// If unit is convoy leader (What_Am_I == 1):
convoy_type = vtable_call(techno, 0x2c);
if (convoy_type == 1)
{
    next = techno->next_in_convoy;         // techno+0x6C8
    while (next != NULL)
    {
        next->SetSpeed(techno->formation_speed);  // techno+0x578 via vtable+0x544
        next = next->next_in_convoy;               // next+0x6C8
        if (next == NULL || next == next->next_in_convoy) break;
    }
}
```

---

## Phase 3: BUDGET Accumulation (lines 197-205)

```c
speed = vtable_call(techno, 0x538);   // FootClass::GetCurrentSpeed -> 0x4DB1A0
budget = (~-(param_2 != 0) & speed) + loco->residual_ticks;
```

Translation: if `param_2` (retry) is nonzero, speed contribution is 0 (use only residual).
If param_2 is 0 (normal tick), add the full speed value to the residual budget.

- `vtable+0x538` = `FootClass::GetCurrentSpeed` at `0x004DB1A0`
- `loco->residual_ticks` at loco+0x4C

---

## Phase 4: MAP_EDGE_RETREAT (lines 206-260)

Activated when `movement_state == 8` AND `track_index == -1`:

```c
if (techno->path_queue[0] == 8 && loco->track_index == -1)
{
    vtable_call(techno, 0x124);  // DoCloak(0) — 0x4D3780

    // If head_to != NullCoord, clear it
    if (loco->head_to != NullCoord)
    {
        loco->head_to = NullCoord;
        loco->is_on_track = 0;
    }

    // Look up map-edge waypoint
    waypoint_idx = techno->waypoint_array[+0x116];  // short
    if (waypoint_idx >= 0 && waypoint_idx < DAT_008b4148)
    {
        waypoint = DAT_008b413c[waypoint_idx];

        // Set head_to from waypoint coordinates
        cell_xy = waypoint->+0x28;  // packed cell {X:16, Y:16}
        loco->head_to_x = (cell_xy & 0xFFFF) * 256 + 128;
        loco->head_to_y = (cell_xy >> 16) * 256 + 128;
        loco->head_to_z = 0;

        // Copy path data from waypoint into techno's path queue
        memcpy(techno+0x5E0, techno+0x5E4, 0x5C);  // shift path queue (23 * 4 bytes)
        techno->path_end_marker = -1;    // techno+0x63C
        techno->edge_waypoint_idx = waypoint_idx;  // techno+0x684
        techno->edge_waypoint_flag = 0;             // techno+0x685

        // Compute exit direction from waypoint flags
        direction = waypoint->+0x30 & 7;
        exit_cell = waypoint->+0x24 + direction_offsets[direction];

        // Get center coords of exit cell
        exit_center = CellClass__Get_Center_Coords(MapClass__Get_CellClass(exit_cell));
        techno->target_pos = exit_center;  // techno+0x568..0x570

        // Interpolate Z between current height and waypoint height
        src_height = CellClass__GetGroundHeight(techno->pos);
        dst_height = CellClass__GetGroundHeight(waypoint->+0x28);
        track_length = waypoint->+0x1C0;
        techno->target_pos_z = src_height + (dst_height - src_height) / track_length;

        loco->is_on_track = 1;
        loco->track_index = -1;
        return 0;
    }

    // Fallback: invalid waypoint
    techno->path_queue[0] = -1;
    loco->track_index = -1;
    loco->head_to = NullCoord;
    loco->is_on_track = 0;
    return 0;
}
```

### Vtable calls in this phase:
| Offset | Address | Method |
|--------|---------|--------|
| +0x124 | 0x4D3780 | TechnoClass::DoCloak |

---

## Phase 5: STEP_LOOP (lines 261-695)

The main stepping loop. Only entered when `budget > 7`.

### 5a. Track table lookup

```c
track_entry_offset = loco->track_index * 12;     // loco+0x58
track_table_entry = g_DriveTrackIndex_Table + track_entry_offset;  // 0x7e7b28

if (loco->is_reversed == 0)                       // loco+0x60
    step_array_idx = track_table_entry[0];         // normal track byte
else
    step_array_idx = track_table_entry[+1];        // reversed/short track byte

step_data_offset = step_array_idx * 16;            // into g_DriveTrackData_Array
step_data_ptr = g_DriveTrackData_Array[step_data_offset];  // pointer to TrackPoint array
```

### 5b. Direction validation

```c
if (movement_state != 8 && movement_state != -1)
{
    track_direction = g_DriveTrackDirection_Table[track_entry_offset];  // 0x7e7b2c
    expected_dir = ((track_direction >> 4) + 1) >> 1 & 7;
    if (expected_dir != movement_state)
        set_direction_mismatch_flag = true;
}
```

### 5c. Main step loop body

```c
do {
    budget -= 7;
    step_x = step_data[point_index * 12];        // TrackPoint.x
    step_y = step_data[point_index * 12 + 4];    // TrackPoint.y

    if (step_x == 0 && step_y == 0 && point_index != 0)
    {
        // === TRACK END ===   (section 6a)
    }
    else
    {
        // === MID-TRACK STEP === (section 6b)
    }

    point_index++;
} while (budget > 7);
```

---

## Phase 6a: TRACK_END Handling (lines 304-425)

When the track step has `(x=0, y=0)` and `point_index != 0`, the track has ended.

### Manhattan distance budget reclaim

```c
dx_to_waypoint = loco->head_to_x - techno->pos_x;   // loco+0x40 vs techno+0x9C
dy_to_waypoint = loco->head_to_y - techno->pos_y;    // loco+0x44 vs techno+0xA0
manhattan = abs(dx_to_waypoint) + abs(dy_to_waypoint);
extra_budget = ftol(manhattan);   // convert to int
budget += extra_budget;
```

### Cell transition: ExitCell / EnterCell

```c
techno->at_destination_marker = 1;  // techno+0x6B6
techno->path_blocked_flag = 0;      // techno+0x6B7

// Compare current cell to head_to cell
curr_cell = {techno->pos >> 8};
head_cell = {loco->head_to >> 8};

if (curr_cell == head_cell)
{
    // SAME CELL — mark in place
    old_facing_lock = techno->facing_lock;   // techno+0x74
    techno->facing_lock = 0;
    vtable_call(techno, 0x1B4);              // Set_Coords_With_Cloak (0x4DB810; corrected 2026-05-28: was EnterCell)
    techno->facing_lock = old_facing_lock;
}
else
{
    // DIFFERENT CELL — full cell transition
    vtable_call(techno, 0x124, 0);           // DoCloak(0) — unmark old cell
    vtable_call(techno, 0x1B4);              // Set_Coords_With_Cloak at new coords (corrected 2026-05-28: was EnterCell)
    vtable_call(techno, 0x1CC, 0);           // Set_Height_On_Bridge (0x5F5FA0; corrected 2026-05-28: was SetCoords) — update Z
    vtable_call(techno, 0x124, 1);           // DoCloak(1) — mark new cell
}
```

### Clear waypoint and track state

```c
// Clear head_to if set
if (loco->head_to != NullCoord)
{
    loco->head_to = NullCoord;
    loco->is_on_track = 0;
}
loco->track_index = -1;
loco->point_index = 0;
```

### Destination arrival check

```c
// Check if NavCom (techno+0x5A4) has a target
nav_target = techno->nav_com;  // techno+0x5A4
if (nav_target != NULL)
{
    // Get NavCom target's cell
    target_coords = vtable_call(nav_target, 0x4C, techno);  // GetActionCoords
    target_cell = vtable_call(techno, 0x1B8);                // GetCell (0x41BEA0)

    // If target cell matches current cell AND Z within tolerance:
    if (target_cell == nav_cell
        && abs(target_z - loco->dest_z) < g_DriveHeightStep * 2)
    {
        // Clear destination — we've arrived
        loco->dest = NullCoord;
        loco->head_to = NullCoord;
        loco->is_on_track = 0;
    }
}
```

### Post-track-end mission handling

```c
vtable_call(techno, 0x18C, 2);  // UnitClass::PerCellProcess (corrected 2026-05-28: was "SetMission(Guard=2)"; vtable+0x18C dispatches UnitClass__PerCellProcess at 0x739EC0)

// If alive and not falling/sinking:
if (techno->is_alive && !techno->is_falling && !techno->is_sinking)
{
    if (same_cell_arrived)
    {
        FootClass__Stop_Moving();
        techno->path_queue[0] = -1;
        mission = vtable_call(techno, 0x184);  // GetCurrentMission — 0x5B3040
        if (mission == 2 /* Guard */)
        {
            // Try Scatter_Force
            result = vtable_call(techno, 0x484, 0, 1);  // Scatter_Force — 0x738970
            if (result) return 1;
        }
    }

    // Check if path continues
    result = vtable_call(techno, 0x504);  // next-path check — 0x4DB9B0
    if (result == 0)
    {
        if (!techno->is_alive) return 0;
        break;  // fall through to residual handling
    }
}
return 1;
```

### Vtable calls in TRACK_END:
| Offset | Address | Method |
|--------|---------|--------|
| +0x0C0 | 0x41C070 | FUN_0041c070 (unnamed; reads bridge flag) |
| +0x0F4 | 0x744210 | ObjectClass::Clear_Occupation |
| +0x124 | 0x4D3780 | TechnoClass::DoCloak |
| +0x184 | 0x5B3040 | MissionClass::GetCurrentMission |
| +0x18C | 0x739EC0 | UnitClass::PerCellProcess (corrected 2026-05-28: was "UnitClass::Mission_Enter (SetMission)"; binary shows UnitClass__PerCellProcess via get_function_by_address 0x739EC0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x1B4 | 0x4DB810 | TechnoClass::Set_Coords_With_Cloak (corrected 2026-05-28: was "EnterCell"; binary shows TechnoClass__Set_Coords_With_Cloak via get_function_by_address 0x4DB810 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x1B8 | 0x41BEA0 | ObjectClass::Get_Cell_Packed (corrected 2026-05-28: was "BuildingClass::GetCell"; binary shows ObjectClass__Get_Cell_Packed via get_function_by_address 0x41BEA0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x1CC | 0x5F5FA0 | FootClass::Set_Height_On_Bridge (corrected 2026-05-28: was "SetHeight/SetCoords"; binary shows FootClass__Set_Height_On_Bridge via get_function_by_address 0x5F5FA0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x484 | 0x738970 | UnitClass::Scatter_Force |
| +0x504 | 0x4DB9B0 | FootClass::Check_Destination_Is_UnitRepair_Dock (corrected 2026-05-28: was "next-path-or-scatter check"; binary shows FootClass__Check_Destination_Is_UnitRepair_Dock via get_function_by_address 0x4DB9B0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |

### External calls in TRACK_END:
| Address | Function |
|---------|----------|
| 0x4DF0D0 | FootClass::Stop_Moving |

---

## Phase 6b: MID-TRACK STEP (lines 426-695)

When the step has nonzero dx/dy (still within a track).

### IsOnBridge pre-mark check

```c
is_on_bridge = vtable_call(techno, 0xC0);  // reads techno+0x6B6
if (is_on_bridge)
{
    // Save current position for re-mark
    saved_coords = techno->pos;
    vtable_call(techno, 0xF4);  // Clear_Occupation
    techno->at_destination_marker = 0;  // techno+0x6B6
    techno->path_blocked_flag = 0;      // techno+0x6B7
}
```

### Previous-step coordinate reconstruction

```c
if (point_index == 0)
{
    // First step: get cell from techno
    prev_cell = vtable_call(techno, 0x1B8);  // GetCell
}
else
{
    // Reconstruct from previous track point
    prev_step = step_data[(point_index - 1) * 12];
    Transform_Track_Coords(&result, &prev_step, &prev_heading);
    prev_cell = {result >> 8};
}
```

### Current step coordinate computation

```c
heading = step_data[point_index * 12 + 8];  // TrackPoint.facing
step_xy = {step_x, step_y};

Transform_Track_Coords(&world_coords, &step_xy, &heading);

// Convert to absolute coords by adding to techno position
new_x = techno->pos_x + (world_coords.x - techno->pos_x);  // adjusted
new_y = techno->pos_y + (world_coords.y - techno->pos_y);
new_cell = {new_x >> 8, new_y >> 8};
```

### Cell-same case

```c
if (new_cell == curr_cell)
{
    // Same cell — just update position
    old_facing_lock = techno->facing_lock;
    techno->facing_lock = 0;
    vtable_call(techno, 0x1B4);    // EnterCell
    techno->facing_lock = old_facing_lock;
}
```

### Cell-different case

When the step crosses into a new cell:

```c
// Exit old cell, enter new cell
vtable_call(techno, 0x124, 0);     // DoCloak(0)
vtable_call(techno, 0x1B4);        // EnterCell at new position
```

#### Bridge ramp detection

```c
src_cell_obj = MapClass__Get_CellClass(prev_cell);
dst_cell_obj = MapClass__Get_CellClass(new_cell);

dst_height = dst_cell_obj->height_level;  // cell+0x11B (signed char)
src_height = src_cell_obj->height_level;

if (dst_height == src_height - 4)
{
    // Ramp DOWN by 4 levels
    if (dst_cell_obj->flags_140 & 0x100)      // dst has bridge
        techno->on_bridge = 1;                  // techno+0x8C
    else if (src_cell_obj->flags_140 & 0x100)  // src has bridge
        techno->on_bridge = 0;
}
else
{
    // Not a 4-level ramp — check bridge flags anyway
    if (dst_cell_obj->flags_140 & 0x100)
        ; // on bridge (handled by goto)
    else if (src_cell_obj->flags_140 & 0x100)
        techno->on_bridge = 0;
}
```

#### JumpJet obstacle scatter

```c
technoType = vtable_call(techno, 0x84);  // GetTechnoType

if (technoType->jumpjet_flag != 0   // technoType+0xC94
    && techno->unk_6D0 == 0)         // techno+0x6D0 (as byte, via [0x1B4] indexing)
{
    if (techno->is_on_bridge == 0)   // techno->+0x8C
    {
        // Get ground-level occupants
        cell_z = techno->pos_z;
        ground_z = CellClass__GetGroundHeight(techno->pos) + g_BridgeZOffset_Drive;
        if (cell_z <= ground_z)
            occupant_list = dst_cell_obj->objects_ground;  // cell+0xE4
        else
            occupant_list = dst_cell_obj->objects_bridge;  // cell+0xE8
    }
    else
    {
        occupant_list = dst_cell_obj->objects_bridge;      // cell+0xE8
    }

    // Walk occupant linked list
    while (occupant = occupant_list; occupant != NULL)
    {
        occupant_list = occupant->next;     // occupant+0x30 (linked list)
        is_crushable = FUN_005f6cd0(techno);  // check if techno can crush occupant

        if (!is_crushable)
        {
            // Damage the occupant
            damage_amount = 10000;
            occupant->vtable_call(0x16C, &damage_amount, 0,
                Rules->ScatterDistance, 0, 1, 1, 0);  // ReceiveDamage

            // Also damage self lightly
            self_damage = 20;
            techno->vtable_call(0x16C, &self_damage, 0,
                Rules->ScatterDistance, 0, 1, 0, 0);
        }
    }
}
```

#### Can_Enter_Cell mid-track check

Only runs if `techno->flag_90` is set (the unit is still entering a cell mid-track):

```c
vtable_call(techno, 0x124, 1);   // DoCloak(1) — mark new cell

// Get overlay/terrain at target cell
cell_overlay = dst_cell_obj->overlay_id;  // cell+0x44 (-1 = none)
if (cell_overlay != -1)
{
    overlay_type = DAT_00a83d84[cell_overlay];

    if (loco->can_crush_flag)   // loco+0x64
    {
        technoType = vtable_call(techno, 0x84);
        // Check Crusher flag and specific overlay properties
        if ((technoType->crusher || FUN_0070d0d0(0x11))
            && overlay_type->is_wall          // overlay+0x2A8
            && technoType->can_destroy_walls)  // technoType+0xD2B
        {
            techno->turret_rotation = -0.05f;  // 0xBD4CCCCD as float
        }
    }
}

// Check for FindFirstBuilding at cell (building entry)
if (loco->can_crush_flag
    && CellClass__FindFirstBuilding(dst_cell_obj) != NULL)
{
    technoType = vtable_call(techno, 0x84);
    if (technoType->locomotor_id == 0xC)  // 12 = Drive
    {
        techno->is_braking = 1;  // techno+0x6B5
        if (technoType->can_destroy_walls)
            techno->turret_rotation = -0.05f;
    }
}
```

### Track chaining (mid-step, continuation to next track)

After the mid-track step, at the **mid-track index** (the `entry_index` from
g_DriveTrackData_Array+0x08), the engine checks if it can chain to a follow-on track:

```c
// At step == mid_step AND step == entry_index of current track:
if (point_index != 0
    && g_DriveTrackData_Array[step_data_offset + 0x0C] == point_index  // jump_index match
    && movement_state != 8 && movement_state != -1
    && direction_mismatch_flag)
{
    // Compute next track from follow-on direction
    next_direction = track_table_entry[+4];  // direction byte
    next_track_index = movement_state + ((next_direction >> 4 + 1) >> 1 & 7) * 8;
    next_entry = g_DriveTrackIndex_Table[next_track_index * 12];

    if (next_entry != 0 && g_DriveTrackData_Array[next_entry * 16 + 4] != 0)
    {
        // Compute Can_Enter_Cell for the follow-on cell
        next_cell_x = loco->head_to_x + g_DirectionDeltaX[movement_state & 7];
        next_cell_y = loco->head_to_y + g_DirectionDeltaY[movement_state & 7];

        current_height = FUN_005f5f00();  // get height+bridge offset
        target_cell = MapClass__Get_CellClass(next_cell);

        result = vtable_call(techno, 0x1AC,   // Can_Enter_Cell — 0x73F0A0
                             target_cell, movement_state, current_height);

        switch (result) {
```

#### Can_Enter_Cell dispatch (mid-track):

```
case 0 (OK):
case 2 (temporarily blocked → still OK for chaining):
    // Chain to next track
    if (What_Am_I != 1 || convoy_leader allows)
    {
        loco->is_reversed = 0;
        loco->track_index = next_track_index;
        step_data_offset = next_entry * 16;
        loco->point_index = g_DriveTrackData_Array[step_data_offset + 4] - 1;
        step_data_ptr = g_DriveTrackData_Array[step_data_offset];

        loco->head_to = NullCoord;
        loco->is_on_track = 1;

        // Per-cell processing update
        vtable_call(techno, 0x18C);  // UnitClass::PerCellProcess (corrected 2026-05-28: was "SetMission")
        loco->is_on_track = 0;

        // Check alive/falling/sinking
        if (!techno->is_alive || techno->is_falling || techno->is_sinking)
            return 0;

        // Apply track delta to new cell
        loco->is_on_track = 1;
        loco->head_to = {next_cell_x, next_cell_y, next_cell_z};

        cell = CellClass__Get_Cell_At(&next_cell);
        ok = CrateClass__PickupDispatch(techno, &next_cell);  // pick up crate at entered cell (corrected 2026-05-28: was "FUN_00481a00 validate/enter cell"; binary shows CrateClass__PickupDispatch — ROOT_CAUSE: INFERENCE_HARDENED)

        if (!ok || techno->is_falling)
        {
            // Failed — clear head_to
            loco->head_to = NullCoord;
            loco->is_on_track = 0;
        }
        else
        {
            // Success — apply full track delta
            DriveLocomotionClass__Apply_Track_Delta(&next_cell, 1);
            vtable_call(techno, 0x544);  // PerFrameVisualUpdate

            // Copy path queue forward
            memcpy(techno+0x5E0, techno+0x5E4, 0x5C);
            techno->path_end_marker = -1;
        }
    }
    break;

case 1 (special blockage):
    CellClass__Get_Cell_At();
    CellClass__Mark_Objects_Redraw();
    break;

case 3 (scatter required):
    MapClass__Check_Crushable_Obstacle(techno, &next_cell);
    break;

case 6 (obstacle):
    cell = CellClass__Get_Cell_At(loco->head_to);
    if (cell->flags_140 & 0x100)  // bridge check
    {
        // Height check: is unit above bridge level?
        techno_z = techno->pos_z;
        cell_height = cell->height_level;  // cell+0x11B
        bridge_level = techno_z / g_DriveHeightStep - cell_height;

        if (abs(bridge_level) >= 3)
            should_scatter = true;
        else
            should_scatter = false;
    }
    else
    {
        should_scatter = false;
    }

    // Scatter objects from cell
    CellClass__Get_Cell_At(loco->head_to);
    CellClass__Scatter_Objects(NullCoord, 1, should_scatter);
    break;
```

---

## Phase 7: RESIDUAL Interpolation (lines 696-793)

After the step loop exits (budget <= 7), the remaining budget is stored and a visual
interpolation position is computed.

### Store residual

```c
loco->residual_ticks = budget;  // loco+0x4C

if (budget < 1) return 0;
if (loco->track_index < 0) return 0;
```

### Compute interpolated position

```c
// Look up current track point
track_entry_offset = loco->track_index * 12;
if (loco->is_reversed == 0)
    step_idx = g_DriveTrackIndex_Table[track_entry_offset];
else
    step_idx = g_DriveTrackIndex_Table[track_entry_offset + 1];

step_data_ptr = g_DriveTrackData_Array[step_idx * 16];
point_index = loco->point_index;
step = step_data_ptr[point_index * 12];

// Check for track-end sentinel
if (step.x == 0 && step.y == 0 && point_index != 0)
    return 0;

// Get current techno position
saved_pos = techno->pos;    // techno+0x9C..0xA4
heading = step_data_ptr[point_index * 12 + 8];

// Transform the current step's delta
Transform_Track_Coords(&full_delta, &step, &heading);

// Compute full-step target position
full_x = techno->pos_x + full_delta.x;
full_y = techno->pos_y + full_delta.y;
full_z = 0;
full_cell = CellClass__Get_Cell_At({full_x, full_y, full_z});

// Interpolate by budget fraction: budget * (1/7) = fractional position
interpolated = FUN_0075f540(&full_delta, (float)budget * (1.0/7.0));
//                                                         ^^ DAT_007e7fa8

interp_x = saved_pos.x + interpolated.x;
interp_y = saved_pos.y + interpolated.y;
interp_z = saved_pos.z + interpolated.z;
interp_cell = CellClass__Get_Cell_At({interp_x, interp_y, interp_z});

// Safety check: interpolated cell must match either saved or full-step cell
current_cell = CellClass__Get_Cell_At(techno->pos);

if (interp_cell == current_cell || interp_cell == full_cell
    || budget > 3)   // if budget > 3, use full step coords instead
{
    visual_pos = {interp_x, interp_y, interp_z};
}
else
{
    // Fallback: use full step coords
    visual_pos = {full_x, full_y, full_z};
}
```

### Apply visual position to techno

```c
visual_cell_src = CellClass__Get_Cell_At(visual_pos);
visual_cell_dst = CellClass__Get_Cell_At(techno->pos);  // for bridge check

if (visual_cell == techno_cell)
{
    // Same cell — mark in place
    old_facing_lock = techno->facing_lock;
    techno->facing_lock = 0;
    vtable_call(techno, 0x1B4);  // EnterCell
    techno->facing_lock = old_facing_lock;
}
else
{
    // Different cell — full transition
    vtable_call(techno, 0x124, 0);   // DoCloak(0)
    vtable_call(techno, 0x1B4);      // EnterCell

    // Bridge ramp detection (same logic as mid-track step)
    src_height = src_cell->height_level;  // cell+0x11B
    dst_height = dst_cell->height_level;
    if (dst_height == src_height - 4)
    {
        if (dst_cell->flags_140 & 0x100)
            techno->on_bridge = 1;
    }
    else if (dst_cell->flags_140 & 0x100)
        ; // keep bridge state

    if (src_cell->flags_140 & 0x100)
        techno->on_bridge = 0;

    vtable_call(techno, 0x124, 1);   // DoCloak(1)
}

return 0;
```

---

## Complete Vtable Call Reference

All virtual calls made through `**(techno_vtable + offset)`:

| Vtable Offset | Address (UnitClass) | Method Name | Parameters | Purpose |
|---------------|---------------------|-------------|------------|---------|
| +0x02C | 0x746E20 | UnitClass::What_Am_I | () | Returns 1 for UNIT |
| +0x04C | 0x4DBDF0 | FootClass::GetDestinationCoords | (out Coord3D*) | Get destination cell center |
| +0x084 | 0x6F3270 | TechnoClass::GetTechnoType_Trampoline | () | Returns TechnoTypeClass* |
| +0x088 | 0x741490 | TechnoClass::GetTechnoType_Impl_Unit | () | Alternate type accessor (UnitClass) |
| +0x0C0 | 0x41C070 | FUN_0041c070 | () | Unnamed; reads bridge flag at techno+0x6B6 |
| +0x0F0 | 0x7441B0 | ObjectClass::Mark_Occupation | () | Mark cell occupation bits |
| +0x0F4 | 0x744210 | ObjectClass::Clear_Occupation | () | Clear cell occupation bits |
| +0x124 | 0x4D3780 | TechnoClass::DoCloak | (int state) | Cloak/uncloak at position |
| +0x160 | 0x41BF40 | TechnoClass::IsIronCurtainActive (corrected 2026-05-28: was "CanFidget"; binary shows TechnoClass__IsIronCurtainActive via get_function_by_address 0x41BF40 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | () | Check Iron Curtain active state |
| +0x16C | 0x737C90 | UnitClass::ReceiveDamage | (damage, ...) | Apply damage to object |
| +0x184 | 0x5B3040 | MissionClass::GetCurrentMission | () | Returns MissionType enum |
| +0x18C | 0x739EC0 | UnitClass::PerCellProcess (corrected 2026-05-28: was "SetMission"; binary shows UnitClass__PerCellProcess via get_function_by_address 0x739EC0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | (int mission) | Per-cell processing called at step transitions |
| +0x1AC | 0x73F0A0 | UnitClass::Can_Enter_Cell | (cell, dir, height) | Cell passability check |
| +0x1B4 | 0x4DB810 | TechnoClass::Set_Coords_With_Cloak (corrected 2026-05-28: was "EnterCell"; binary shows TechnoClass__Set_Coords_With_Cloak via get_function_by_address 0x4DB810 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | (Coord3D* pos) | Set coords and update cloak state |
| +0x1B8 | 0x41BEA0 | ObjectClass::Get_Cell_Packed (corrected 2026-05-28: was "GetCell"; binary shows ObjectClass__Get_Cell_Packed via get_function_by_address 0x41BEA0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | () | Get NW cell index (packed short) |
| +0x1BC | 0x5F6960 | ObjectClass::GetOccupiedCell | () | Get occupied cell pointer |
| +0x1CC | 0x5F5FA0 | FootClass::Set_Height_On_Bridge (corrected 2026-05-28: was "SetHeight"; binary shows FootClass__Set_Height_On_Bridge via get_function_by_address 0x5F5FA0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | (int z_offset) | Set height accounting for bridge level |
| +0x1D0 | 0x5F5F30 | ObjectClass::GetHeight | () | Read height value |
| +0x38C | — | GetDecelSteps | () | Deceleration step count |
| +0x484 | 0x738970 | UnitClass::Scatter_Force | (bool, bool) | Force scatter from pos |
| +0x504 | 0x4DB9B0 | FootClass::Check_Destination_Is_UnitRepair_Dock (corrected 2026-05-28: was "CheckNextPath"; binary shows FootClass__Check_Destination_Is_UnitRepair_Dock via get_function_by_address 0x4DB9B0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | () | Check if destination is a unit-repair dock |
| +0x538 | 0x4DB1A0 | FootClass::GetCurrentSpeed | () | Current speed as int |
| +0x544 | 0x4D3710 | TechnoClass::SetSpeedFraction (corrected 2026-05-28: was "PerFrameVisualUpdate"; binary shows TechnoClass__SetSpeedFraction via get_function_by_address 0x4D3710 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | (speed) | Set speed fraction for visual movement |

---

## Complete External Function Calls

| Address | Name | Called From | Purpose |
|---------|------|-------------|---------|
| 0x4B4780 | Transform_Track_Coords | Mid-step, residual | Mirror/flip track deltas |
| 0x4B0AD0 | Apply_Track_Delta | Track chain success | Apply end-of-track offset |
| 0x4C9300 | FUN_004c9300 (FacingUpdate) | After mid-step | Update facing timer/angle |
| 0x4DF0D0 | FootClass::Stop_Moving | Track end, same-cell | Stop movement, clear paths |
| 0x481A00 | CrateClass__PickupDispatch (corrected 2026-05-28: was "FUN_00481a00 (ValidateEnter)"; binary shows CrateClass__PickupDispatch via get_function_by_address 0x481A00 — ROOT_CAUSE: INFERENCE_HARDENED; called in track-chain success path to pick up crates at the entered cell) | Track chain | Crate pickup at entered cell |
| 0x5657A0 | MapClass::Get_CellClass | Bridge detect | Convert coords to CellClass* |
| 0x480A30 | CellClass::Get_Center_Coords | MAP_EDGE | Get cell center in leptons |
| 0x47C3D0 | CellClass::Find_Nearest_Object (corrected 2026-05-28: was "CellClass::GetGroundHeight"; binary shows CellClass__Find_Nearest_Object via get_function_by_address 0x47C3D0; actual GetGroundHeight wrapper is at 0x578080 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | Speed compute, bridge | NOTE: This address is wrong for GetGroundHeight; see 0x578080 |
| 0x578080 | CellClass::GetGroundHeight (corrected 2026-05-28: added; wrapper address per ADDRESS_MAP.md; inner interp at 0x47B3A0) | Speed compute, bridge | Ground Z at cell |
| 0x483480 | CellClass::Mark_Objects_Redraw | Can_Enter code 1 | Redraw cell objects |
| 0x481670 | CellClass::Scatter_Objects | Can_Enter code 6 | Scatter from cell |
| 0x578AD0 | MapClass::Check_Crushable_Obstacle | Can_Enter code 3 | Scatter crushables |
| 0x5F6CD0 | TechnoClass::CanCrushCheck | JumpJet scatter | Check if techno can crush target |
| 0x5F5F00 | CellClass::Get_Effective_Height (corrected 2026-05-28: was "FUN_005f5f00 (GetHeightLevel)"; binary shows CellClass__Get_Effective_Height via get_function_by_address 0x5F5F00 — ROOT_CAUSE: RTTI_LABEL_DRIFT) | Track chain | Height + bridge offset |
| 0x70D0D0 | FUN_0070d0d0 (HasWeaponAbility) | Overlay crush | Check weapon ability flag |
| 0x75F540 | FUN_0075f540 (ScaleCoord) | Residual interp | Scale {x,y,z} by float factor |

---

## Magic Constants

| Address | Value | Type | Purpose |
|---------|-------|------|---------|
| 0x7E6240+44 | 0.3 (double) | `CONCAT44(DAT_007e6244,DAT_007e6240)` | Min decel speed (normal) |
| 0x7E6248+4C | 0.1 (double) | `CONCAT44(DAT_007e624c,DAT_007e6248)` | Min decel speed (alt) |
| 0x7E6250 | 0.0015 (double) | `_DAT_007e6250` | Alt deceleration rate per tick |
| 0x7E3548 | 0.2 (double) | `_DAT_007e3548` | Braking target speed |
| 0x7E7FA8 | 1/7 (double) | `_DAT_007e7fa8` | Residual interpolation factor |
| 0xBD4CCCCD | -0.05 (float) | literal | Turret rotation on wall crush |
| 7 | — | literal | Step cost (budget -= 7 per step) |
| 0x40 | 64 | literal | Track index threshold for speed compute |
| 4 | — | literal | Bridge ramp height level difference |
| 3 | — | literal | Bridge height tolerance for close-enough |
| 0x100 | — | literal | Bridge flag bitmask in cell->flags_140 |

---

## Locomotor Field Offsets (from ILocomotion this)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x0C | 4 | linked_techno (FootClass*) | The game entity being driven |
| +0x34 | 4 | dest_x | Destination X (leptons) |
| +0x38 | 4 | dest_y | Destination Y (leptons) |
| +0x3C | 4 | dest_z | Destination Z (leptons) |
| +0x40 | 4 | head_to_x | Next waypoint X |
| +0x44 | 4 | head_to_y | Next waypoint Y |
| +0x48 | 4 | head_to_z | Next waypoint Z |
| +0x4C | 4 | residual_ticks | Budget leftover from last tick |
| +0x50 | 8 | current_speed (double) | Current movement speed |
| +0x58 | 4 | track_index | Active track (-1 = none) |
| +0x5C | 4 | point_index | Current step within track |
| +0x60 | 1 | is_reversed | Use short/reversed track variant |
| +0x62 | 1 | deploy_flag | Deploy-while-moving state |
| +0x63 | 1 | is_on_track | 1 = actively following a track |
| +0x64 | 1 | can_crush_flag | Checked during overlay/building entry |

## Linked Techno Field Offsets

| Offset | Size | Field | Access |
|--------|------|-------|--------|
| +0x74 | 1 | facing_lock | Saved/restored during EnterCell |
| +0x81 | 1 | is_falling | Death check, post-chain check |
| +0x8C | 1 | on_bridge | Set during bridge ramp detection |
| +0x8D | 1 | is_sinking | Death check |
| +0x90 | 1 | is_alive | Loop continuation check |
| +0x9C | 4 | pos_x | Current position X |
| +0xA0 | 4 | pos_y | Current position Y |
| +0xA4 | 4 | pos_z | Current position Z |
| +0x15E | 8 | max_speed (double, word-indexed) | Target speed for acceleration |
| +0x334 | 4 | turret_rotation (float) | Set to -0.05 on wall crush |
| +0x3CD | 1 | is_decelerating_flag | Triggers alt decel path |
| +0x568 | 12 | target_pos (Coord3D) | Target coords during retreat |
| +0x578 | 4 | formation_speed | Propagated to convoy members |
| +0x5A4 | 4 | nav_com (ptr) | Navigation target |
| +0x5E0 | 96 | path_queue[24] | Direction entries (-1=end, 8=stop) |
| +0x63C | 4 | path_end_marker | Set to -1 on queue shift |
| +0x684 | 1 | edge_waypoint_idx | Map edge waypoint |
| +0x685 | 1 | edge_waypoint_flag | Map edge state |
| +0x6B5 | 1 | is_braking | Triggers braking speed clamp |
| +0x6B6 | 1 | at_destination_marker | Set on track end |
| +0x6B7 | 1 | path_blocked_flag | Cleared on track end |
| +0x6C4 | 4 | convoy_leader_type_ptr | Leader's type for convoy check |
| +0x6C8 | 4 | next_in_convoy (ptr) | Linked list for formation |

## TechnoTypeClass Field Offsets (via vtable+0x84)

| Offset | Size | Field |
|--------|------|-------|
| +0x2F8 | 4 | decel_threshold_distance (int) |
| +0x300 | 8 | decel_rate (double) |
| +0x308 | 8 | accel_rate (double) |
| +0x5B4 | 4 | locomotor_id (12 = Drive) |
| +0xC94 | 1 | jumpjet_flag |
| +0xCA1 | 1 | deploy_while_moving |
| +0xD28 | 1 | crusher_flag |
| +0xD2B | 1 | can_destroy_walls |
| +0xDBD | 1 | is_formation_leader |
| +0xE0C | 1 | convoy_exempt_decel (on leader type) |

---

## CellClass Field Offsets

| Offset | Size | Field |
|--------|------|-------|
| +0x11B | 1 | height_level (signed char) |
| +0x140 | 4 | flags (bit 8 = 0x100 = has bridge) |
| +0x44 | 4 | overlay_id (-1 = none) |
| +0xE4 | 4 | objects_ground (linked list ptr) |
| +0xE8 | 4 | objects_bridge (linked list ptr) |

---

## Summary of State Machine Flow

```
Process_Drive_Track(loco, is_retry):
    |
    +-- GUARD_FAIL? (no track, not stopped) -> return 0
    +-- DEPLOY_CHECK? (deploying, type forbids) -> return 0
    |
    +-- if track_index < 64:
    |     SPEED_COMPUTE:
    |       distance_to_dest = 3D euclidean
    |       if near dest: decelerate (rate from type)
    |       elif decel_flag: alt deceleration (0.0015/tick, min 0.1)
    |       if braking: clamp to 0.2
    |       elif over max: accel toward max
    |       elif under max: decel toward max
    |       propagate speed to convoy chain
    |
    +-- budget = (is_retry ? 0 : GetSpeed()) + residual
    |
    +-- if movement_state==8 && track==-1:
    |     MAP_EDGE_RETREAT: populate path from waypoint
    |     return 0
    |
    +-- if budget > 7:
    |     lookup track table entry
    |     validate direction
    |
    |     do {
    |       budget -= 7
    |       read step (dx, dy)
    |
    |       if dx==0 && dy==0 && step!=0:
    |         TRACK_END:
    |           reclaim manhattan distance into budget
    |           ExitCell / EnterCell (same-cell or cross-cell)
    |           clear head_to, track_index=-1
    |           check destination arrival (nav_com match)
    |           SetMission(Guard)
    |           if alive: check next path or scatter
    |           return 1
    |       else:
    |         MID-TRACK STEP:
    |           transform coords
    |           if same cell: mark in place
    |           if different cell:
    |             exit old, enter new
    |             bridge ramp detection (4-level diff + flag 0x100)
    |             JumpJet obstacle scatter
    |             overlay/building crush check
    |           update facing via FacingUpdate
    |           if at jump_index: attempt track chaining
    |             Can_Enter_Cell dispatch (0=chain, 1=redraw, 3=scatter, 6=obstacle)
    |
    |       point_index++
    |     } while (budget > 7)
    |
    +-- residual_ticks = budget
    +-- if budget >= 1 && track valid:
    |     RESIDUAL INTERPOLATION:
    |       compute full-step target position
    |       interpolate by budget * (1/7)
    |       safety: verify interpolated cell is valid
    |       apply visual position to techno
    |       bridge ramp detection on visual pos
    +-- return 0
```
