# DriveLocomotionClass — Complete Binary Reference

Verified against `gamemd.exe` via Ghidra MCP. All addresses, offsets, and algorithms
confirmed from live decompilation. See `DRIVE_TRACK_SYSTEM.md` for the track data tables
and stepping algorithm — this document covers the full locomotion class.

## Class Identity

- **CLSID:** `{4A582741-9839-11d1-B709-00A024DDAFD1}`
- **COM interfaces:** IUnknown (+0x00), ILocomotion (+0x04), IPiggyback (+0x18)
- **Object size:** 0x6C bytes (108 bytes)
- **Constructor:** `0x004af540`
- **ILocomotion vtable:** `0x007e7eb0`
- **IUnknown vtable:** `0x007e7f7c`
- **IPiggyback vtable:** `0x007e7e8c`

ShipLocomotionClass (`{2BEA74E1-7CCA-11d3-BE14-00104B62A16C}`) is **~95% identical**
to DriveLocomotionClass but has 6 concrete differences. See
`SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` for the full delta. Key differences:
Ship has 67 TurnTrack entries (vs 72), 14 RawTrack entries (vs 16), spawns wake
animations every 8 frames (vs 10 for dust), reads deceleration directly from
TypeClass+0x678 instead of calling a virtual function, and has slightly different
check ordering in Process_Movement. Most findings here still apply to Ship.

## Object Field Layout

All offsets from the IUnknown `this` pointer (== object base).

| Offset | Size | Type | Init | Field | Verified From |
|--------|------|------|------|-------|---------------|
| +0x00 | 4 | ptr | vtable | IUnknown vtable | Constructor |
| +0x04 | 4 | ptr | vtable | ILocomotion vtable | Constructor |
| +0x08 | 4 | ptr | — | ref_count / sub_locomotor_ptr | Is_Moving |
| +0x0C | 4 | ptr | — | **linked_techno** (FootClass*) | All functions |
| +0x18 | 4 | ptr | vtable | IPiggyback vtable | Constructor |
| +0x1C | 4 | int | 0 | **cached_slope_index** (from CellClass+0x11C) | Process (slope detect) |
| +0x20 | 4 | int | 0 | **previous_slope_index** (prior cached_slope_index snapshot; NOT a frame-timer sentinel) | Process (corrected 2026-07-18: was `slope_timer_start_frame, Init -1`; Constructor at 0x4af540 sets this field to literal 0 unconditionally (`param_1[8] = 0`), and Process at 0x4b0500 writes `piVar2[7] = piVar2[6]` — the PRIOR cached_slope_index value — into it just before +0x1C is updated to the new value; Force_New_Slope at 0x4afb40 writes the same facing param into both +0x1C and +0x20. No code path anywhere compares this field against -1. Verified via `decompile_function 0x4af540`, `decompile_function 0x4b0500`, `decompile_function 0x4afb40` — INFERENCE_HARDENED) |
| +0x24 | 4 | int | CurrentFrame | frame_stamp | Constructor |
| +0x28 | 4 | int | — | slope_timer_remaining (3-frame blend) | Draw_Matrix |
| +0x2C | 4 | int | 0 | slope_timer_total (duration of slope transition) | Draw_Matrix |
| +0x30 | 4 | int | 0 | unknown_30 | Constructor |
| +0x34 | 12 | Coord3D | NullCoord | **destination** (X, Y, Z) | Set_Destination |
| +0x40 | 12 | Coord3D | NullCoord | **head_to** / next waypoint (X, Y, Z) | Process_Movement |
| +0x4C | 4 | int | 0 | **residual_ticks** (movement budget leftover per tick) | Process_Drive_Track |
| +0x50 | 8 | double | 0.0 | **current_speed** (Force_Track writes 1.0 here) | Process_Drive_Track, Force_Track |
| +0x58 | 4 | int | -1 | **track_index** (into TurnTrack[72], -1 = none) | Process_Movement, Force_Track |
| +0x5C | 4 | int | -1 | **point_index** (step within active track) | Process_Drive_Track |
| +0x60 | 1 | byte | 0 | **is_reversed** (use short/reverse track) | Apply_Track_Delta |
| +0x61 | 1 | byte | 0 | has_active_path | Process_Movement |
| +0x62 | 1 | byte | 0 | deploy_flag / was_waiting_flag (dual use) | Process, Process_Drive_Track |
| +0x63 | 1 | byte | 0 | **is_on_track** (1 = actively following) | Process_Drive_Track |
| +0x64 | 1 | byte | 0 | can_crush_flag | Process_Movement |
| +0x65 | 1 | byte | 1 | first_tick_flag / initialized | Constructor |
| +0x68 | 4 | ptr | NULL | piggybacked_locomotor (ILocomotion*) | Destructor |

### Null Coordinate Sentinels

| Locomotor | Address | Value |
|-----------|---------|-------|
| Drive | `0x008a0790` | {0, 0, 0} |
| Ship | `0x00b077f8` | {0, 0, 0} |

Coordinates compared against all three components to detect "no destination" state.

## Linked TechnoClass Fields

The locomotor reads/writes fields on the FootClass/TechnoClass at `*(this + 0x0C)`.

| Techno Offset | Type | Field | Used In |
|---------------|------|-------|---------|
| +0x44 | int | owner_house_index (-1 = none) | Can_Enter_Cell crush check |
| +0x74 | byte | cloak_state | Process_Drive_Track (saved/restored during Mark) |
| +0x81 | byte | is_falling | Process_Drive_Track (death check) |
| +0x8C | byte | on_bridge | Process_Movement (bridge transition) |
| +0x8D | byte | is_sinking | Process_Drive_Track (death check) |
| +0x90 | byte | is_alive | Process_Drive_Track (death check) |
| +0x9C | 12 | Coord3D | current position (X, Y, Z) | Everywhere |
| +0x15E | 8 | double | max_speed (from type) | Speed computation |
| +0x598 | int | tether_target (non-zero = tethered) — accessed in binary as `param_1[0x166]` where `param_1` is `int *`, so the array index 0x166 × 4 = byte offset **0x598**. Prior table row listed `+0x166` as the byte offset (the array-index ×4 pitfall). Verified via `decompile_function 0x4afe00` (Stop_Moving) and `decompile_function 0x4b0500` (Process). | Stop handling |
| +0x178 | int | movement_state (-1=none, 8=stopped) | State machine |
| +0x328 | float | body_roll (slope tilt) | Draw_Matrix |
| +0x32C | float | body_pitch (slope tilt) | Draw_Matrix |
| +0x334 | float | turret_rotation | Process_Drive_Track |
| +0x3CD | byte | is_decelerating_flag | Speed computation |
| +0x578 | int | formation_speed | Formation propagation |
| +0x5E0 | int[24] | **path_queue** (direction entries, -1 = end, 8 = stopped) | Process_Movement |
| +0x640 | int | movement_delay_start (frame, -1 = no delay) | Blocked handling |
| +0x644 | int | movement_delay_facing | Blocked handling |
| +0x648 | int | movement_delay_ticks | Blocked handling |
| +0x64C | int | path_stuck_counter (set to 10 on failure) | Process_Movement |
| +0x668 | int | blocked_delay_start | Can_Enter_Cell code 2 |
| +0x66C | int | blocked_delay_facing | Can_Enter_Cell code 2 |
| +0x670 | int | blocked_delay_ticks | Can_Enter_Cell code 2 |
| +0x674 | ptr | locomotor_ptr (ILocomotion*) | Stop_Moving convoy chain |
| +0x684 | byte | map_edge_waypoint_idx | Map edge retreat |
| +0x685 | byte | map_edge_waypoint_flag | Map edge retreat |
| +0x688 | byte | abandon_target_flag | Process_Movement |
| +0x68A | byte | has_pending_scatter | Process_Movement |
| +0x68B | byte | bridge_transition_flag | Process_Movement |
| +0x6AD | byte | deploy_state | Is_Moving |
| +0x6B5 | byte | is_braking | Speed computation |
| +0x6B6 | byte | at_destination_marker | Process_Drive_Track |
| +0x6B7 | byte | path_blocked_flag | Can_Enter_Cell code 2 |
| +0x6C8 | ptr | next_in_convoy (linked list) | Stop_Moving, formation |
| +0x6D0 | byte | convoy_state_flag | Stop_Moving |
| +0x2B4 | ptr | tow_target | Process_Movement (towing validity) |
| +0x2D0 | int | nav_queue_count | Process_Movement (tether guard) |
| +0x558 | int | cell_destination (packed cell) | Process_Movement (finalize) |
| +0x5D4 | int | convoy_nav_queue | Process_Movement (convoy chain) |
| +0x63C | int | something_63C (cleared to -1) | Process_Movement (finalize) |

### TechnoTypeClass Fields (via vtable +0x84)

| Type Offset | Type | INI Key | Used In |
|-------------|------|---------|---------|
| +0x11C | byte | `ROT` | Update_Facing_From_Type |
| +0x15E | 8 | double | `Speed` | Speed computation |
| +0x2F8 | int | — | decel_threshold_distance | Speed computation |
| +0x300 | 8 | double | — | decel_rate | Speed computation |
| +0x308 | 8 | double | — | accel_rate | Speed computation |
| +0x5B4 | int | — | locomotor_id (12 = drive) | Process_Movement |
| +0x678 | int | — | decel_steps | Speed computation |
| +0x67C | int | `SpeedType` | Speed table lookup |
| +0xC94 | byte | `IsTrain` | Multi-purpose: (a) convoy chain stop-propagation in `Stop_Moving` (walks `+0x6C8` next-in-convoy linked list, verified via `decompile_function 0x4afe00`); (b) "trains pass through any soft blocker" gate in `Process_Movement` — when set, downgrades all Can_Enter_Cell codes <7 to 0 (5 reads of +0xC94 in 0x4b2630, verified via `decompile_function 0x4b2630`). Both behaviors are consistent with the `IsTrain` flag confirmed via ReadINI (see §"Convoy / Formation Propagation"). **Cross-doc resolution:** UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md previously labelled this offset as `Crusher` — that was wrong; the binary at all 5 +0xC94 read sites reads IsTrain semantics, not Crusher. Real `Crusher` lives at `+0xD28` (this doc, separately confirmed) and has a narrower path (codes 4/5 only, with cell-overlay guard). UNIT_COLLISION has been patched. |
| +0xCA1 | byte | — | deploy_while_moving | Process_Drive_Track |
| +0xD28 | byte | `Crusher` | Can_Enter_Cell crush |
| +0xD2B | byte | — | can_destroy_walls | Wall overlay crush (Process_Drive_Track) |
| +0xDBD | byte | — | is_formation_leader | Speed computation |
| +0xE0C | byte | — | convoy_exempt_decel | Skips deceleration for convoy leaders |

### CellClass Fields Referenced

| Offset | Type | Field | Used In |
|--------|------|-------|---------|
| +0xE4 | ptr | objects_ground (linked list) | Process_Drive_Track (cell occupant iteration) |
| +0xE8 | ptr | objects_bridge (linked list) | Process_Drive_Track (bridge-level occupants) |
| +0xEC | int | land_type | Speed computation |
| +0x11B | byte | height_level | Bridge ramp detection |
| +0x140 | int | cell_flags (bit 8 = bridge) | Bridge detection, Can_Enter_Cell |

## Function Table

All 15 labeled functions at `0x004af-0x004b5`.

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| 0x4af540 | Constructor | 160 | Init fields, set vtables |
| 0x4af5e0 | Destructor | ~80 | Release piggybacked locomotor, call base dtor |
| 0x4af970 | **Is_Ok_To_End** | ~60 | Returns true iff: Is_Moving()==false AND speed!=0 AND is_on_track AND deploy_state==0. **NOT** Is_Moving — the actual `Is_Moving` lives at **0x4afb80** (vtable slot 4 — see §IPiggyback / ILocomotion vtable). Identity verified via `decompile_function 0x4af970`. (See line ~1469 vtable table which already correctly labels this Is_Ok_To_End, and line ~1727 Piggyback vtable.) |
| 0x4afc20 | Is_Moving_Now | ~80 | True if turn timer active OR has waypoint + speed > 0 |
| 0x4afd40 | Set_Destination | ~120 | Set dest coord with bridge Z adjustment; guarded by 4 state checks |
| 0x4afe00 | Stop_Moving | ~200 | Clamp speed to 0.3, clear dest, propagate stop to convoy chain |
| 0x4aff60 | Draw_Matrix | ~600 | Build 3x4 VXL matrix with turn interpolation + slope pitch/roll |
| 0x4b04d0 | Update_Facing_From_Type | ~30 | Read ROT from TechnoTypeClass, pass to SetFacing |
| 0x4b0ad0 | Apply_Track_Delta | ~280 | Apply track end-point offset to unit position |
| 0x4b0ef0 | vtable22 (Ramp_Update) | ~30 | Forward ramp/slope interpolation to TechnoClass+0x388 |
| **0x4b0f20** | **Process_Drive_Track** | **~5860** | **Per-tick track stepping — the inner state machine** |
| **0x4b2630** | **Process_Movement** | **~8500** | **Top-level movement AI — the outer state machine** |
| 0x4b4780 | Transform_Track_Coords | ~180 | Mirror/flip track deltas via 3-bit flag field |
| 0x4b4890 | Stop_And_Scatter | ~80 | Stop + scatter, or just scatter if no active task |
| 0x4b4d00 | ScalarDeletingDestructor | ~60 | Standard C++ RTTI destructor with optional dealloc |

## State Machine

The locomotor operates as a two-level state machine.

### Outer Level: Process_Movement (0x4b2630)

Called when `track_index == -1` (no active track). Decides what to do next.

**Full decompilation:** See `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` (1033 lines)
for the complete 11-phase state machine with all branch conditions, timer logic,
and recursive call patterns.

**Recursive calls (VERIFIED from assembly — 5 CALL sites, 9 code paths):**
- Code 1 (first cell, retry): recurse(0, 0) at 0x4b397f
- Code 1 (second cell, retry): recurse(0, 0) at 0x4b4480
- Code 2 (second cell ONLY — first cell re-pathfinds without recursion): recurse(param_2, 1) at 0x4b4219
- Codes 4/5/6/7 (first cell, retry): shared LAB_004b4541 → delay + recurse(0, 0) at 0x4b4552
- Codes 4/5 (second cell, cleanup): recurse(param_2, 1) at 0x4b41f7
- Codes 6/7 (second cell, retry): also via shared LAB_004b4541

**Track selection formula (VERIFIED):** `index = next_dir + current_dir * 8` (LEA at 0x4b4016)
**Speed table indexing (VERIFIED):** `table[speed_type + land_type * 9]` (LEA+ADD at 0x4b3c98)
**Slope direction (VERIFIED):** `target > current` = uphill, `target < current` = downhill

```
IDLE
  path_queue[0] == -1 AND Is_Moving() == false
  -> Clear waypoint
  -> If Guard mission: try ScanForTarget
  -> return 0

HAS_DESTINATION_NO_PATH
  path_queue[0] == -1 AND destination != NullCoord
  -> Check movement_delay timer (skip if still counting down)
  -> Call FootClass::Find_Path(dest_cell)
  -> On failure: check CloseEnough, try adjacent cell
  -> On success: transition to HAS_PATH

HAS_PATH
  path_queue[0] != -1 AND != 8
  -> Check mission queue for path truncation (missions 1, 15)
  -> Read direction from path_queue[0] (0-7)
  -> Compute target cell from direction offsets
  -> Resolve bridge flags, terrain height
  -> Set bridge_transition_flag if crossing bridge boundary
  -> Validate facing (turn toward target if needed, return 1 if still turning)
  -> Call Can_Enter_Cell on target cell
  -> Handle result (see dispatch table below)
  -> If OK: compute speed, select drive track, set head_to waypoint
  -> Shift path_queue (advance to next entry)
  -> Set is_on_track = 1
  -> Transition to TRACKING

MAP_EDGE_RETREAT
  movement_state == 8 AND track_index == -1
  -> Look up map edge waypoint from TechnoTypeClass+0x116
  -> Set head_to to waypoint coordinates
  -> Interpolate Z between current and waypoint heights
  -> Populate path queue from waypoint data

STOPPED
  path_queue[0] == 8
  -> return 0 (no processing)
```

### Inner Level: Process_Drive_Track (0x4b0f20)

Called when `track_index != -1` (active track). Steps through track points.

**Full decompilation:** See `PROCESS_DRIVE_TRACK_DECOMPILATION.md` (1087 lines) for the
complete 7-phase state machine with all branch conditions.

**Key details not in summary below:**
- **JumpJet obstacle scatter** deals 10000 damage to cell occupants and 20 self-damage
- **Wall/overlay crush** sets `turret_rotation = -0.05f` (visible turret twitch)
- **Track chaining at jump_index** includes a full Can_Enter_Cell dispatch with 4 cases
  (0/2=chain, 1=redraw, 3=crushable scatter, 6=obstacle scatter with bridge height check)
- **Residual interpolation** computes fractional visual position using `budget * (1/7)`,
  with safety check that the interpolated cell is valid (fallback to full-step coords)
- **Building entry** checks `locomotor_id == 0xC` (Drive) before setting is_braking

```
GUARD_FAIL
  is_on_track == 0 AND track_index == -1
  -> residual_ticks = 0
  -> return 0

DEPLOY_CHECK
  deploy_flag != 0 AND type.deploy_while_moving == 0
  -> residual_ticks = 0
  -> return 0

SPEED_COMPUTE
  -> Compute distance to destination (3D euclidean)
  -> If distance < decel_threshold: decelerate
  -> If is_decelerating_flag: apply alt deceleration
  -> If is_braking: clamp to braking target speed
  -> Else: accelerate toward max_speed
  -> Propagate speed to formation/convoy units

MAP_EDGE_RETREAT (movement_state == 8, track_index == -1)
  -> Look up waypoint, set head_to
  -> Populate path queue from waypoint data
  -> Interpolate Z

STEP_LOOP
  budget = speed + residual_ticks
  while budget > 7:
      read track step (x_delta, y_delta) from track data
      budget -= 7

      if x_delta == 0 AND y_delta == 0 AND step != 0:
          === TRACK END ===
          -> Compute manhattan distance to waypoint, add to budget
          -> Set at_destination_marker = 1
          -> If same cell: Mark in place (save/restore cloak_state)
          -> If different cell: full cell transition (ExitCell + EnterCell)
          -> Clear waypoint, track_index = -1, point_index = 0
          -> If reached final destination: clear destination, is_on_track = 0
          -> Set mission to Guard
          -> If still alive: override mission, scan for targets, check scatter
          -> return 1

      else:
          === MID-TRACK STEP ===
          -> Transform (x, y, z) deltas to world coords via Transform_Track_Coords
          -> Convert to cell coords
          -> If same cell: update position (save/restore cloak_state)
          -> If different cell:
              -> ExitCell, Mark at new position
              -> Bridge ramp detection (height diff == 4 levels + bridge flag)
              -> Set on_bridge flag accordingly
              -> JumpJet obstacle scatter check
              -> Can_Enter_Cell on new cell
              -> If blocked: stop track, scatter

  store remaining budget as residual_ticks
```

## Can_Enter_Cell Return Code Dispatch

Both Process_Movement and Process_Drive_Track dispatch on the return value of
`Can_Enter_Cell` (called via TechnoClass vtable +0x1AC).

**Full decompilation:** See `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` (799 lines) for the
complete 466-line function with all conditions for each return code.

| Code | Meaning | Response | Trigger Conditions (in UnitClass) |
|------|---------|----------|-----------------------------------|
| **0** | OK — cell is passable | Proceed with movement | Default; also tunnel entry, transport dest, IsTrain pass-through |
| **1** | Special blockage | Clear path entry, `Mark_Objects_Redraw`, recurse | Neutral/civilian object with MissionData+0x220==2 (RTTI-filtered) |
| **2** | Temporarily blocked | Set `path_blocked_flag`, BlockedDelay timer, re-pathfind | Moving allied unit; crushable enemy after cloak check; infantry on crushable cell |
| **3** | Scatter required | Scatter `FUN_00578ad0`, clear waypoint, stop | Allied garrisonable building (HasActiveAnim+0x16B7) that can't be garrisoned |
| **4** | Friendly blocking | Crusher+neutral: treat as 0. Else: delay+retry | Friendly/owned wall overlay (IsWall+0x2A8 + ownership check) |
| **5** | Enemy blocking | Same crush logic as 4. Also: Fire_At (vtable+0x1F4) | Enemy wall/unit/building; non-crushable enemy with weapons; infantry+AG projectile (WeaponType→Projectile+0xA5, NOT CellSpread — CORRECTED) |
| **6** | Obstacle | JumpJet: scatter. Else: close-enough check + height tolerance | Allied non-building stationary object; allied building not moving |
| **7** | Impassable | Clear dest, stop. If retry: delay+recurse | 15+ triggers: wrong tunnel, shroud+RequiresRevealedCells, speed_table==0.0, laser fence, deadlock prevention (same-direction within 0x200 leptons), Mission_Unload+cargo |

### Crush Logic Detail (codes 4 and 5)

```
if (code < 7 AND type.JumpJet != 0) OR
   ((code == 4 OR code == 5) AND type.Crusher != 0 AND target.owner_house == -1):
    treat as code 0 (proceed)
```

### Crusher operates at 4 points in Can_Enter_Cell

1. **Wall overlay bypass**: Crusher ignores friendly/enemy wall overlays
2. **Infantry cell handling**: Crusher can move through infantry-occupied cells (returns code 2 for moving, 0 for stationary)
3. **Enemy object crush-candidate marking**: Marks enemy units as crush targets
4. **Cloak interaction**: Crusher check interacts with cloaked unit detection

### Deadlock Prevention (Code 7)

Units moving in the same direction within the same facing octant and within 0x200 leptons
of each other return code 7 to prevent two units from blocking each other head-on.
This is the **same-direction deadlock** check. Uses per-unit MoveTimer octant at +0x388,
NOT a global tick. Blocker's timer offset by +0x7FFF (half-turn) detects opposing phases.

### Verification Corrections to UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md

**Error 1 — Code 5 infantry weapon check:** Report says `Warhead->CellSpread_0x2A5`. Actual
binary reads `WeaponType->Projectile_0xA0->AG_0x2A5` (BulletTypeClass anti-ground flag).
CellSpread is at WarheadType+0x124, not +0x2A5.

**Error 2 — hasFriendlyMoving logic INVERTED:** Report says conditions are ANDed
(`NavTarget==NULL && timer==0 && IsMoving`). Actual binary uses OR:
`NavTarget != NULL || CDTimerClass__Remaining != 0 || Locomotor->IsMoving()`.
Any single condition marks the blocker as "moving" → returns code 2 (temp blocked).

**All other claims verified** — 8 return codes, 21 code-7 triggers, 4-point Crusher,
0x200 lepton deadlock, all field offsets confirmed from assembly.

## Speed Computation Algorithm

Computed in Process_Movement before track assignment, stored at locomotor+0x50.

```
// 1. Base terrain speed
land_type = cell.land_type                          // cell offset +0xEC
speed_type = techno_type.SpeedType                  // type offset +0x67C
base_speed = SpeedType_LandType_Table[speed_type + land_type * 9]
if base_speed > 1.0:
    base_speed = 1.0                                // cap at 1.0

// 2. Slope modifier (only for convoy_type == 1)
src_height = GetGroundHeight(current_cell)
dst_height = GetGroundHeight(target_cell)
if dst_height > src_height:                         // going UPHILL
    if speed_type == 1 (Tracked):
        base_speed *= Rules.SlopeClimb_Tracked      // Rules + 0x768
    else:
        base_speed *= Rules.SlopeClimb_Other        // Rules + 0x778
elif src_height > dst_height:                       // going DOWNHILL
    if speed_type == 1 (Tracked):
        base_speed *= Rules.SlopeDescend_Tracked    // Rules + 0x770
    else:
        base_speed *= Rules.SlopeDescend_Other      // Rules + 0x780

// 3. Zero-speed safety
if base_speed == 0.0:
    base_speed = 0.5                                // prevent deadlock

// 4. Damaged-unit slowdown
health_ratio = GetHealthRatio(techno)
if health_ratio <= Rules.TrafficJamThreshold:       // Rules + 0x1700
    base_speed *= TrafficJamMultiplier              // DAT_007e7fc0

// 5. Store
if track_index < 0x40:
    locomotor.current_speed = base_speed            // direct store
else:
    techno.SetSpeed(base_speed)                     // vtable call
```

### Acceleration / Deceleration (in Process_Drive_Track)

```
distance_to_dest = sqrt(dx^2 + dy^2 + dz^2)        // 3D euclidean
target_speed = techno.max_speed                      // double at techno+0x15E

if distance < type.decel_threshold:                  // type+0x2F8
    target_speed -= type.decel_steps * type.decel_rate  // type+0x678 * type+0x300
    target_speed = max(target_speed, MIN_DECEL_SPEED)   // DAT_007f1308 (double)
    decelerating = true

elif techno.is_decelerating_flag:                    // techno+0x3CD
    target_speed -= type.decel_steps * ALT_DECEL_RATE   // DAT_007f1318
    target_speed = max(target_speed, MIN_DECEL_SPEED_ALT)  // DAT_007f1310
    decelerating = true

if techno.is_braking:                                // techno+0x6B5
    target_speed = BRAKING_TARGET                    // DAT_007e3548
    if current_speed < target_speed:
        target_speed = current_speed
    locomotor.current_speed = target_speed

elif decelerating:
    techno.SetSpeed(target_speed)

elif max_speed < current_speed:                      // over max — slow down
    target_speed = type.accel_rate + max_speed       // type+0x308
    if current_speed < target_speed:
        target_speed = current_speed
    techno.SetSpeed(target_speed)

elif current_speed < max_speed:                      // under max — speed up
    target_speed = max_speed - type.decel_steps * type.decel_rate
    if target_speed < current_speed:
        target_speed = current_speed
    techno.SetSpeed(target_speed)
```

### Movement Budget (VERIFIED from assembly at 0x4b1274)

```
tick_budget = (param_2 ? 0 : GetCurrentSpeed()) + residual_ticks
// When param_2==true (retry after track-to-track chain), speed is ZEROED
// Only residual carries over — prevents double-counting speed in same tick
// Each track step costs exactly 7 units (SUB EDI, 0x7 at 0x4b159d)
// Loop continues while budget > 7
// Remaining budget stored as residual_ticks at loco+0x4C
```

### Manhattan Distance Reclaim at Track End (VERIFIED)

When a track ends, the remaining distance to the waypoint is converted back to budget:
```
manhattan = abs(dest.x - pos.x) + abs(dest.y - pos.y)
reclaimed = ftol((1.0 - manhattan * (1/11)) * 7.0)
budget += reclaimed
```
Constants: `1/11` at 0x7E7FB8, `7.0` at 0x7E7FB0, `1.0` at 0x7E1718.

## Drive Track Selection Formula

In Process_Movement, after Can_Enter_Cell returns 0:

```
next_direction = path_queue[0] & 7                   // 0-7
current_facing = techno.current_facing               // extracted from facing

track_index = next_direction + current_facing * 8    // 0-63

// Check if this track entry has a valid curve
if TurnTrack[track_index].normal_track == 0:
    track_index = current_facing * 9                 // fallback: straight line

// Check if track crosses a cell boundary (flag bit 3)
if TurnTrack[track_index].flags & 8:
    // Two-cell track: validate the NEXT cell too
    next_next_cell = current_cell + direction_offset[next_direction]
    result = Can_Enter_Cell(next_next_cell, next_direction, ...)
    // Apply same crush/JumpJet override logic
    if result != 0:
        // Can't enter next cell — advance path queue, set bridge_transition
        // Fall through to straight movement
```

## Bridge Detection

Bridge transitions detected at multiple points:

```
// In Set_Destination:
cell = GetCellAt(destination)
if cell.flags & 0x100:                               // cell+0x140 bit 8
    destination.Z += BridgeZOffset                   // Drive: DAT_008a07d0-derived

// In Process_Movement (cell transition):
src_cell = GetCellAt(current_coords)
dst_cell = GetCellAt(target_coords)
if (dst_cell.flags >> 8 & 1) != techno.on_bridge:
    techno.bridge_transition_flag = 1

// In Process_Drive_Track (mid-track step):
src_height = src_cell.height_level                   // cell+0x11B
dst_height = dst_cell.height_level
if dst_height == src_height - 4:                     // bridge ramp = 4 levels
    if dst_cell.flags & 0x100:
        techno.on_bridge = 1
    elif src_cell.flags & 0x100:
        techno.on_bridge = 0

// Height difference check for "close enough":
abs(techno.coord_z / HeightStep - cell.height_level) < 3
```

## Draw_Matrix — VXL Rendering Transform

Produces a 3x4 (12-float) matrix for VXL body rendering.

```
// Slope transition interpolation
if slope_timer_total != 0:
    remaining = CDTimer_Remaining(slope_timer)
    t = (total - remaining) / total                  // 0.0 to 1.0
else:
    t = 1.0                                          // no slope transition = complete

// Two paths:
if t >= 1.0 AND abs(pitch) < threshold AND abs(roll) < threshold:
    // SIMPLE: pure facing rotation, no slope
    facing_matrix = VXL_GetFacingMatrix(body_facing)
    // Or interpolated: VXL_InterpolatedFacing(body_facing, t)

else:
    // COMPLEX: has slope — apply pitch/roll rotation
    sin_pitch = sin(techno.body_pitch)               // techno+0x32C
    cos_pitch = cos(techno.body_pitch)
    sin_roll  = sin(techno.body_roll)                // techno+0x328
    cos_roll  = cos(techno.body_roll)
    // Build rotation matrix from sin/cos
    // Apply as column shears + axis rotations
    // Set facing_index = -1 to disable VXL cache
    facing_matrix = VXL_InterpolatedFacing(body_facing, t)
    result = slope_matrix * facing_matrix
```

## Convoy / Formation Propagation

Stop_Moving and Process_Drive_Track propagate speed/stop to linked units.

```
// In Stop_Moving:
if head_to != NullCoord AND type.IsTrain (TechnoTypeClass+0xC94) AND !convoy_state_flag:
    next = techno.next_in_convoy                     // techno+0x6C8
    while next != NULL AND next != next.next_in_convoy:
        next.locomotor.Stop_Moving()                 // via vtable+0x48
        next = next.next_in_convoy

// In Process_Drive_Track (speed propagation):
if techno.GetConvoyType() == 1:
    next = techno.next_in_convoy                     // techno+0x6C8
    while next != NULL AND next != next.next:
        next.SetSpeed(techno.formation_speed)        // techno+0x578
        next = next.next_in_convoy
```

**Full convoy system:** See `CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md` (277 lines).

### Key Convoy Discoveries

- **Convoy chains are built at map load time** in `ScenarioClass::Read_Units_Section`
  (0x743270), NOT dynamically by player group-move commands.
- **IsTrain** (TechnoTypeClass+0xC94, confirmed via ReadINI): enables mutual pass-through
  in Can_Enter_Cell — both units must have IsTrain and be UnitClass.
- **Team convoy**: `TeamClass::Set_Convoy_Target` (0x6E9050) selects highest-ThreatPosed
  member as pathfinder; others follow via mission 0xF destination sync.
- **Speed propagation**: Leader with `Accelerates=true` (type+0xDBD) in Guard mission
  writes speed to techno+0x578, then propagates to all followers via `Set_Speed_Percent`.

## Set_Destination Guard Checks

Four vtable checks must ALL return false before a new destination is accepted:

| Vtable Offset | Check | Meaning |
|---------------|-------|---------|
| +0x37C | IsCrashing() | Don't path while crashing |
| +0x380 | IsSinking() | Don't path while sinking |
| +0x1D4 | IsUnloading() | Don't path while unloading |
| +0x1D8 | IsLanding() | Don't path while landing |

## Stop_Moving Behavior

1. Check if unit has trailer chain — propagate stop to all linked units
2. **Clamp speed to 0.3** (double at `DAT_007e6240`) — does NOT instantly stop
3. Clear destination to NullCoord
4. Does NOT clear head_to or track state — active track continues to completion

## Global Address Reference

### DriveLocomotionClass

| Address | Name | Purpose |
|---------|------|---------|
| 0x008a0790 | g_NullCoord_Drive | Null coordinate sentinel {0,0,0} |
| 0x008a07d0 | HeightStep_Drive | Z leptons per terrain height level |
| 0x007e7a28 | g_DriveTrackData_Array | 16-entry RawTrack metadata (16 bytes each) |
| 0x007e7b28 | g_DriveTrackIndex_Table | 72-entry TurnTrack table (12 bytes each) |
| 0x007e7b30 | g_DriveTrackFlags_Table | Transform flags (at +8 of each TurnTrack entry) |
| 0x007e7eb0 | ILocomotion vtable | 24-slot COM vtable |
| 0x007e7f7c | IUnknown vtable | 3-slot COM vtable |
| 0x007e7e8c | IPiggyback vtable | Piggybacking interface vtable |

### ShipLocomotionClass (parallel addresses)

| Address | Name |
|---------|------|
| 0x00b077f8 | g_NullCoord_Ship |
| 0x00b07838 | HeightStep_Ship |
| 0x007f2960 | g_ShipTrackData_Array (g_DriveTrackStepArrays) |
| 0x007f2a40 | g_ShipTrackIndex_Table (g_DriveTrackDescriptors) |

### Rules.ini Offsets (via `DAT_008871e0`)

| Offset | Type | INI Key | Purpose |
|--------|------|---------|---------|
| +0x768 | double | SlopeClimb (Tracked) | Uphill speed multiplier for SpeedType=1 |
| +0x770 | double | SlopeDescend (Tracked) | Downhill speed multiplier for SpeedType=1 |
| +0x778 | double | SlopeClimb (Other) | Uphill speed multiplier for other SpeedTypes |
| +0x780 | double | SlopeDescend (Other) | Downhill speed multiplier for other SpeedTypes |
| +0xFA8 | int | ScatterDistance | Max scatter distance in leptons |
| +0x1700 | double | TrafficJamThreshold | Health ratio threshold for speed penalty |
| +0x1718 | int | CloseEnough | Distance for "arrived at destination" (leptons) |
| +0x1760 | double | `PathDelay` | Movement delay between pathfinding attempts (VERIFIED) |
| +0x1768 | int | `BlockagePathDelay` | Blocked retry delay ticks (VERIFIED from 0x83d314) |

### Float / Double Constants

| Address | Value | Purpose |
|---------|-------|---------|
| 0x007e1718 | 1.0 (double) | Identity multiplier |
| 0x007e1718 | 1.0 (double, `0x3FF0000000000000`) | Identity multiplier, speed cap ceiling |
| 0x007e2800 | 0.0 (double, `0x0000000000000000`) | Zero-speed check |
| 0x007e3548 | 0.2 (double, `0x3FC999999999999A`) | Braking/traffic-jam target speed |
| 0x007e44e8 | 0.005 (double, `0x3F747AE147AE147B`) | Slope tilt threshold — below this pitch/roll is "flat" |
| 0x007e6240 | 0.3 (double, `0x3FD3333340000000`) | Stop_Moving speed clamp |
| 0x007e7fa8 | 1/7 (double, `0x3FC2492492492492`) | Residual tick interpolation factor (corrected 2026-07-18: was `0x3FC2492492244992`, two hex digit-pairs transposed; binary bytes read `92 24 49 92 24 49 C2 3F` LE = `0x3FC2492492492492` via `read_memory 0x007e7fa8` — OFFSET_RETYPED_WRONG) |
| 0x007e7fc0 | 0.75 (double, `0x3FE8000000000000`) | Damaged-unit speed multiplier |
| 0x007f1308 | 0.3 (double) | Minimum speed during deceleration (normal) |
| 0x007f1310 | 0.1 (double) | Minimum speed during deceleration (is_decelerating) |
| 0x007f1318 | 0.0015 (double) | Alternative deceleration rate per tick |
| 0x007e7fb0 | 7.0 (double) | Track end manhattan reclaim multiplier (VERIFIED) |
| 0x007e7fb8 | 1/11 (double, 0.0909...) | Track end manhattan scaling factor (VERIFIED) |

### Terrain Speed Table — Complete Structure

| Address | Name |
|---------|------|
| 0x0089ea40 | g_SpeedType_LandType_Table |
| 0x0081da58 | g_SpeedTypeNameTable (8 string pointers) |

**Layout:** `float[LandType * 9 + SpeedType]` — 12 land types × 9 entries (8 speed + 1 buildable flag).
Total size: 432 bytes (12 × 36). Populated by `SpeedType_TablePopulator` (0x674000) from rules.ini.

**SpeedType enum** (verified from name table at 0x81da58):

| Index | Name | INI Key | Notes |
|-------|------|---------|-------|
| 0 | Foot | `Foot=` | Infantry |
| 1 | Track | `Track=` | Tracked vehicles (tanks) |
| 2 | Wheel | `Wheel=` | Wheeled vehicles |
| 3 | Hover | `Hover=` | Hovercraft |
| 4 | Winged | — | **Hardcoded to 1.0** (aircraft always pass) |
| 5 | Float | `Float=` | Floating/naval |
| 6 | Amphibious | `Amphibious=` | Amphibious vehicles |
| 7 | FloatBeach | `FloatBeach=` | Beach landing craft |

**LandType enum** (from `CellClass::RecalcLandType` at 0x483c80):

| Index | Name | Set When |
|-------|------|----------|
| 0 | Clear | Default for passable terrain |
| 1 | Road | Overlay has road flag (+0x22D) |
| 2 | Rough | Overlay has rough flag (+0x2A8), or tile is rough type |
| 3 | Rock | Tile type == 6 (cliff/rock) |
| 4 | Water | Tile type == 2 |
| 5 | Tiberium | Terrain object is ore/tiberium |
| 6 | Beach/Impassable | Zero speed in table, or blocking building, or overlay +0x2B5 |
| 7 | OutOfBounds | Cell outside playfield |

**Indexing:** `speed = table[land_type * 9 + speed_type]`. Values are floats, default 1.0,
capped at 1.0. A value of 0.0 means impassable for that speed type on that land type.

**Buildable flag:** Byte at `table + land_type * 36 + 32` — read from `Buildable=` INI key
per land type section. Controls whether structures can be placed on this land type.

### Speed Table Values from rulesmd.ini (YR defaults)

All values are percentages → stored as float 0.0-1.0. `—` = 0%.

| LandType | Foot | Track | Wheel | Hover | Float | Amphib | FltBeach | Build |
|----------|------|-------|-------|-------|-------|--------|----------|-------|
| Clear | 100 | 100 | 100 | 50 | — | 80 | — | yes |
| Road | 100 | 100 | 100 | 75 | — | 100 | — | yes |
| Rough | 100 | 100 | 100 | 50 | — | 80 | — | yes |
| Rock | — | — | — | — | — | — | — | no |
| Water | — | — | — | 100 | 100 | 100 | 100 | no |
| Wall | — | — | — | — | — | — | — | no |
| Tiberium | 90 | 70 | 50 | 50 | — | 50 | — | no |
| Weeds | 50 | 70 | 50 | 100 | — | 50 | — | no |
| Beach | — | — | — | 75 | — | 60 | 100 | no |
| Ice | 50 | 80 | 50 | 100 | — | 50 | — | no |
| Railroad | 90 | 100 | 50 | 100 | — | 50 | — | no |
| Tunnel | 100 | 100 | 100 | 100 | — | 100 | — | no |

**Key observations for Drive locomotor (SpeedType=Track or Wheel):**
- Track: 100% on Clear/Road/Rough/Railroad/Tunnel, 70% on Tiberium/Weeds, 80% on Ice, 0% everywhere else
- Wheel: 100% on Clear/Road/Rough/Tunnel, 50% on Tiberium/Weeds/Ice/Railroad, 0% on Water/Rock/Wall/Beach
- Winged (index 4): always 1.0 (hardcoded, not in INI) — aircraft fly over everything

## Tick Execution Order

```
FootClass::AI (0x4da530)
  -> ILocomotion::Process (vtable+0x40)
      -> DriveLocomotionClass__Process (0x4b0500, ~1600 bytes)
          1. Update facing ROT from TechnoType if changed
          2. if track_index != -1 AND is_on_track:
              -> Process_Drive_Track(0x4b0f20, retry=0)
              -> if track finished mid-tick:
                  -> Process_Movement(0x4b2630) to select next track
                  -> Process_Drive_Track(retry=1) for seamless chaining
          3. else (no active track):
              -> Follow-mission destination update from convoy leader
              -> Arrival detection (position == destination)
              -> Turn timer handling
              -> Mission-specific checks (Guard, Harvest)
              -> Process_Movement(0x4b2630) to pathfind + select track
              -> Process_Drive_Track(0x4b0f20) if track was assigned
          4. Post-movement: Mark occupation, idle speed check
             -> Every 10 frames with cell.LandType==2 (Water): spawn `Wake=` animation (from RulesClass+0x94). Verified via `decompile_function 0x4b0500`. SpeedType==2 (Wheel) is unused in this check — see CORRECTION at §"Water wake animation".
```

## Ship vs Drive — Difference Table (CORRECTED)

**Previous claim "byte-for-byte clone" is INCORRECT.** ~95% identical but with 6 verified
differences. Full analysis: `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` (392 lines).

| Aspect | DriveLocomotionClass | ShipLocomotionClass |
|--------|---------------------|---------------------|
| CLSID | 4A582741-... | 2BEA74E1-... |
| Constructor | 0x004af540 | 0x0069ec50 |
| Process | 0x004b0500 | 0x0069fc10 |
| Process_Drive_Track | 0x004b0f20 | 0x006a05f0 |
| Process_Movement | 0x004b2630 | 0x006a1c80 |
| NullCoord | 0x008a0790 | 0x00b077f8 |
| HeightStep | 0x008a07d0 | 0x00b07838 |
| TrackData array | 0x007e7a28 | 0x007f2960 |
| **TurnTrack entries** | **72 entries** | **67 entries** (no dock/undock 67-71) |
| **RawTrack entries** | **16 entries** (tracks 0-15) | **14 entries** (tracks 0-13, no 14-15) |
| ILocomotion vtable | 0x007e7eb0 | 0x007f2d8c |

### 6 Verified Differences

1. **Track table size**: Ship has 67 TurnTrack + 14 RawTrack (vs 72 + 16). Missing
   tracks 14-15 and entries 67-71 = refinery dock/undock curves (ships don't dock).
2. **Wake anim frequency**: Ship spawns every **8 frames**, Drive every **10 frames**.
   Ship also skips a TypeClass+0xD69 flag check.
3. **Decel rate source**: Ship reads TypeClass+0x678 directly; Drive calls virtual +0x38C.
4. **Check ordering**: Ship checks tether count before deploy/unload; Drive reverses this.
5. **Convoy logic**: Drive has convoy leader mission-target sync; Ship omits it entirely.
6. **Shared track data**: All 14 common curves (tracks 1-13) are byte-identical point data.

The only behavioral difference is the data — Ship may have different track curves
optimized for water movement, and uses a separate set of null-coord/height globals.
The actual movement algorithm, state machine, speed computation, and Can_Enter_Cell
dispatch are byte-for-byte identical.

## ILocomotion Vtable — Complete 40-Slot Map (0x007e7eb0)

All addresses verified from live Ghidra memory read.

| Slot | Offset | Address | Method | Impl |
|------|--------|---------|--------|------|
| 0 | +0x00 | 0x4b4d90 | QueryInterface | Drive — compares GUID vs IPiggyback |
| 1 | +0x04 | 0x4b4da0 | AddRef | Drive — InterlockedIncrement(this+0x14) |
| 2 | +0x08 | 0x4b4db0 | Release | Drive — InterlockedDecrement, calls dtor if 0 |
| 3 | +0x0C | 0x55a710 | Link_To | Base — sets this+0x04 and this+0x08 to FootClass* |
| 4 | +0x10 | 0x4afb80 | Is_Moving | Drive — returns true iff: (1) dest != NullCoord; OR (2) head_to != NullCoord AND head_to.XY != current.XY. Units mid-track with no destination but an active head_to waypoint still report Is_Moving=true (4 conditions total — see body breakdown at line ~1803). Verified via `decompile_function 0x4afb80`. |
| 5 | +0x14 | 0x4afc90 | Destination | Drive — returns dest coord (this+0x30..0x3B) |
| 6 | +0x18 | 0x4afcc0 | Head_To_Coord | Drive — returns head_to, fallback to techno pos |
| 7 | +0x1C | 0x55abf0 | Can_Enter_Cell | Base — stub, always returns 0 |
| 8 | +0x20 | 0x55abe0 | Is_To_Have_Shadow | Base — always returns 1 |
| 9 | +0x24 | 0x4aff60 | Draw_Matrix | Drive — 3x4 VXL matrix, turn interp + slope |
| 10 | +0x28 | 0x4b0410 | Shadow_Matrix | Drive — shadow variant, forces facing=-1 on slope |
| 11 | +0x2C | 0x55abd0 | Shadow_Point | Base — returns {0, 0} |
| 12 | +0x30 | 0x55a8c0 | Draw_Point | Base — returns {0, z_adjust} |
| 13 | +0x34 | 0x55abc0 | Visual_Character | Base — returns 0 |
| 14 | +0x38 | 0x4b4870 | Z_Adjust | Drive — returns 0 |
| 15 | +0x3C | 0x4b4880 | Z_Gradient | Drive — returns 2 (Deg45) |
| **16** | **+0x40** | **0x4b0500** | **Process** | **Drive — MAIN TICK ENTRY (~1600 bytes)** |
| 17 | +0x44 | 0x4afd40 | Move_To / Set_Destination | Drive — with bridge Z adjust |
| 18 | +0x48 | 0x4afe00 | Stop_Moving | Drive — clamp speed 0.3, convoy propagation |
| 19 | +0x4C | 0x4b0ef0 | Do_Turn | Drive — forwards to RateTimer facing interp |
| 20 | +0x50 | 0x4b04d0 | Unlimbo | Drive — reads ROT, inits facing |
| 21 | +0x54 | 0x55ab90 | Tilt_Pitch_AI | Base — no-op |
| 22 | +0x58 | 0x55a8f0 | Power_On | Base — sets powered flag, self-calls refresh |
| 23 | +0x5C | 0x55a910 | Power_Off | Base — clears powered flag |
| 24 | +0x60 | 0x55a930 | Is_Powered | Base — reads powered flag at this+0x0C |
| 25 | +0x64 | 0x55a940 | Is_Ion_Sensitive | Base — returns false |
| 26 | +0x68 | 0x55ab70 | Push | Base — stub, returns false |
| 27 | +0x6C | 0x55ab80 | Shove | Base — stub, returns false |
| 28 | +0x70 | 0x4b0c40 | Force_Track | Drive — force onto specific track (350 bytes) |
| 29 | +0x74 | 0x4b4820 | In_Which_Layer | Drive — returns 2 (Layer::Ground) |
| 30 | +0x78 | 0x55ac00 | Force_Immediate_Destination | Base — no-op stub |
| 31 | +0x7C | 0x4afb40 | Force_New_Slope | Drive — sets slope, inits turn timer |
| 32 | +0x80 | 0x4afc20 | Is_Moving_Now | Drive — true if turning OR has speed+waypoint |
| 33 | +0x84 | 0x55ad10 | Apparent_Speed | Base — delegates to techno GetSpeed |
| 34 | +0x88 | 0x55acf0 | Drawing_Code | Base — returns 0 |
| 35 | +0x8C | 0x55ad00 | Can_Fire | Base — returns 0 (no locomotor fire restriction) |
| 36 | +0x90 | 0x4b4c60 | Get_Status | Drive — returns 0 |
| 37 | +0x94 | 0x4b4c70 | Acquire_Hunter_Seeker_Target | Drive — no-op |
| 38 | +0x98 | 0x4b4c80 | Is_Surfacing | Drive — returns false |
| 39 | +0x9C | 0x4b48d0 | Mark_All_Occupation_Bits | Drive — Apply_Track_Delta on head_to |

## Verified Global Constants

All values confirmed by reading raw bytes from gamemd.exe via Ghidra MCP.

### .rdata Constants (baked into PE)

| Address | Raw Hex (LE) | Value | Purpose |
|---------|-------------|-------|---------|
| 0x007e1718 | `00 00 00 00 00 00 F0 3F` | **1.0** | Speed cap ceiling, turn ratio max |
| 0x007e2800 | `00 00 00 00 00 00 00 00` | **0.0** | Zero-speed check → replaced with 0.5 |
| 0x007e3548 | `9A 99 99 99 99 99 C9 3F` | **0.2** | Braking target speed (traffic jam) |
| 0x007e44e8 | `7B 14 AE 47 E1 7A 74 3F` | **0.005** | Slope tilt threshold (below = "flat") |
| 0x007e6240 | `00 00 00 40 33 33 D3 3F` | **0.3** | Stop_Moving speed clamp |
| 0x007e7fa8 | `92 24 49 92 24 49 C2 3F` | **1/7 (0.14286)** | Residual tick → interpolation factor |
| 0x007e7fc0 | `00 00 00 00 00 00 E8 3F` | **0.75** | Damaged-unit speed multiplier |
| 0x007f1308 | `00 00 00 40 33 33 D3 3F` | **0.3** | Min decel speed (normal) |
| 0x007f1310 | `00 00 00 A0 99 99 B9 3F` | **0.1** | Min decel speed (alt/is_decelerating) |
| 0x007f1318 | `00 00 00 C0 74 93 58 3F` | **0.0015** | Alt deceleration rate per tick |

Note: 0x007f1308/10/18 are duplicates of 0x007e6240/48/50 — MSVC emitted separate constant pools.

### .data Globals (computed at runtime, zero in PE)

| Address | Type | Value | Source |
|---------|------|-------|--------|
| 0x008a0790 | Coord3D (12 bytes) | {0, 0, 0} | Drive NullCoord — set by init at 0x004af4e0 |
| 0x00b077f8 | Coord3D (12 bytes) | {0, 0, 0} | Ship NullCoord — set by init at 0x0069ebf2 |
| 0x008a07d0 | int | runtime | Drive HeightStep — `ftol(DAT_008a0758 - DAT_008a0780)` |
| 0x00b07838 | int | runtime | Ship HeightStep — mirror of Drive logic |
| 0x00b0782c | int | runtime | BridgeZOffset — set during map load |
| 0x0089ea40 | float[] | runtime | SpeedType_LandType_Table — from rules.ini |
| 0x0089f688 | short[8][2] | runtime | CellDirOffset — per-direction cell deltas |
| 0x0089f6d8 | int[8][2] | runtime | SubCellDirOffset — per-direction lepton deltas |

## Helper Functions Called by DriveLocomotionClass

All identified and renamed in Ghidra. These are external functions the locomotor
calls but are not class members.

| Address | Name | Class | Purpose |
|---------|------|-------|---------|
| 0x004c93d0 | RateTimer__Current | RateTimer | Read interpolated facing value from timer struct |
| 0x00578460 | MapClass__Is_Cell_In_Playfield | MapClass | Validate cell is within playable map bounds |
| 0x00578ad0 | MapClass__Check_Crushable_Obstacle | Global | Scatter friendly crushable infantry from cell |
| 0x005657a0 | MapClass__Get_CellClass | MapClass | Convert {X,Y} cell coord to CellClass* (Y*512+X index) |
| 0x00480a30 | CellClass__Get_Center_Coords | CellClass | Cell to lepton center: X=cellX*256+128, Y=cellY*256+128 |
| 0x0047c3d0 | CellClass__Find_Nearest_Object | CellClass | Find closest object in cell by 2D distance |
| 0x0047c5a0 | CellClass__Find_Blocking_Object | CellClass | Iterate cell objects for WhatAmI==2, fallback to Find_Nearest |
| 0x005f5f00 | CellClass__Get_Effective_Height | CellClass | cell.height_level + (on_bridge ? 4 : 0) |
| 0x00481a00 | CellClass__Can_Enter_Cell_General | CellClass | 787-line general cell passability check |
| 0x0056d100 | MapClass__Can_Reach_Zone | MapClass | Zone ID comparison for reachability |
| 0x006ec3a0 | TechnoClass__Clear_Convoy_Chain | TechnoClass | Clear linked convoy chain, set target=0 |
| 0x004f9a90 | HouseClass__Is_Ally | HouseClass | Alliance bitmask check between two houses |
| 0x0065ae30 | PathType__Has_Valid_Steps | Global | Check if pathfinding result has at least one step |
| 0x004d3920 | FootClass__Find_Path | FootClass | A* pathfinder entry, see PATHFINDING_ASTAR_GHIDRA_REPORT.md |
| 0x004cbba0 | FootClass__Run_AStar | FootClass | Wrapper: zone precheck → A* → 3 smoothing passes |
| 0x00429a90 | AStar_main_loop | Global | Cell-level A* with 8+1 directions, dual ground/bridge layers |
| 0x00429830 | AStar_compute_edge_cost | Global | LandTypeCost × cliff(4.0) × bridge × speed factors |
| 0x0042c290 | Zone_precheck | Global | Hierarchical zone graph mini-A* before cell search |
| 0x00481670 | CellClass__Scatter_Objects | CellClass | Scatter all eligible objects from cell |
| 0x00483480 | CellClass__Mark_Objects_Redraw | CellClass | Mark all cell objects for visual redraw |
| 0x006b7d80 | RadioClass__Tether_Count | RadioClass | Count active tether links on object |
| 0x0041c380 | CoordStruct__Distance3D | Global | sqrt(X*X + Y*Y + Z*Z), result in FPU stack |
| 0x004d3920 | FootClass__Find_Path | FootClass | Full A* pathfinder, populates 24-entry path queue |
| 0x004df0d0 | FootClass__Stop_Moving | FootClass | Zero out movement delta fields (+0x5A0, +0x5A4) |
| 0x004da2a0 | FootClass__Is_Mission_Harvest | FootClass | Returns true if current mission == 7 (Harvest) |
| 0x005f6cd0 | TechnoClass__CanCrushCheck | TechnoClass | Checks Crusher/Insignificant flags + alliance |
| 0x0070d0d0 | TechnoClass__HasWeaponAbility | TechnoClass | Checks weapon flags at type+0x29C+idx |
| 0x0075f540 | CoordStruct__ScaleByFactor | Global | ftol(x*s), ftol(y*s), ftol(z*s) — residual interp |
| 0x004c9300 | FacingClass__UpdateFacing | FacingClass | Update facing timer/angle for turn interpolation |
| 0x004db810 | FootClass__EnterCell | FootClass | vtable+0x1B4, marks occupation with cloak_state |
| 0x004db9b0 | FootClass__CheckNextPathOrScatter | FootClass | vtable+0x504, checks nav_com/scatter |
| 0x005f5fa0 | ObjectClass__SetHeight | ObjectClass | vtable+0x1CC, adjusts Z from ground height |

### Helper Detail: MapClass__Get_CellClass (0x005657a0)

Most-called function in the engine. Map is 512 cells wide.
```
index = Y * 0x200 + X
if index < 0 OR index > 0x3FFFF:
    return g_NullCellSingleton (0x00abdc50)
return CellArray[index]      // array base at MapClass+0x13C
```

### Helper Detail: CellClass__Get_Center_Coords (0x00480a30)

```
X_lepton = cell.X * 256 + 128
Y_lepton = cell.Y * 256 + 128
Z_lepton = GetGroundHeight(cell)    // via FUN_0047b3a0
```

### Helper Detail: CoordStruct__Distance3D (0x0041c380)

```
result = sqrt(X*X + Y*Y + Z*Z)     // left on x87 FPU stack
caller consumes via Math__ftol()    // converts to int
```

### Helper Detail: FootClass__Stop_Moving (0x004df0d0)

Extremely simple — just 14 bytes:
```
*(this + 0x5A0) = 0
*(this + 0x5A4) = 0
```

### Mission IDs Referenced by Locomotor

| ID | Name | Where Used |
|----|------|------------|
| 0 | Sleep | Process (idle check) |
| 2 | Guard | Process_Drive_Track (post-arrival), Process_Movement (idle) |
| 5 | Guard_Area | Process (check) |
| 7 | Harvest | Is_Mission_Harvest, Process_Movement (special tolerance) |
| 0xB | Hunt | Process (Follow locomotor check) |
| 0xF | Follow | Process (convoy destination update) |
| 0x10 | Special | Process (skip to mark_check) |

## Process Entry Point Detail (0x004b0500)

The main tick function is **not** a simple Process_Drive_Track/Process_Movement
dispatcher. It contains significant orchestration logic (~1600 bytes, cyclomatic
complexity 77, 38 call sites).

**Full decompilation:** See `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md` (638 lines).

ShipLocomotionClass::Process (0x69FC10) is **86.5% similar** — byte-for-byte clone
with different data table pointers.

1. **~~ROT change detection~~ SLOPE DETECTION** (CORRECTED): Reads the current cell's
   **SlopeIndex** from `CellClass+0x11C` (via `GetOccupiedCell`, vtable+0x1BC).
   Compares against cached value at `loco+0x1C`. If different, starts a 3-frame
   slope transition timer for smooth body tilt blending in Draw_Matrix.
   Fields +0x1C through +0x2C control slope interpolation, NOT turn timing.

2. **Two-phase track execution**: When Process_Drive_Track completes a track
   mid-tick (track_index resets to -1), Process immediately calls Process_Movement
   to select the next track, then calls Process_Drive_Track again with retry=1.
   This produces seamless track-to-track transitions within a single tick.

3. **Convoy/Follow destination sync**: Only for `RTTI_Unit` (1) following
   `RTTI_Aircraft` (0xF). Destination is updated every tick from the followed
   target's action coordinates.

4. **Zone reachability early-out**: Before calling Process_Movement on the idle path,
   `CanReachDestination` (vtable+0x2CC) is checked. Unreachable destinations cause
   immediate give-up and scatter.

5. **Arrival detection**: When position == destination AND no active track, the
   unit is considered arrived. **Tethered units** (techno+0x598 != 0) get
   `Stop_Moving + Scatter_Force`; untethered just get `Scatter`.

6. **Water wake animation** (CORRECTED): Every 10 frames, if the unit's cell has
   **LandType==2 (Water)** (NOT SpeedType==2 as previously documented), spawns a
   `Wake=` animation from `RulesClass+0x94`. This is a water wake effect for
   naval units using Drive locomotor, not a dust cloud.

7. **Idle speed forcing**: If all coordinates are null AND movement_state == -1
   AND max_speed > 0, calls techno.SetSpeed(0,0) to force idle state.

## Offset Convention Warning

In Ghidra decompilations, `param_1` / `this` for ILocomotion vtable functions
is the **ILocomotion interface pointer** at `object_base + 4`. Add 4 to all
decompiler offsets to get the object-base offset documented in this file.

| Decompiler | Object | Field |
|------------|--------|-------|
| param_1+0x08 | +0x0C | linked_techno |
| param_1+0x30 | +0x34 | dest_x |
| param_1+0x3C | +0x40 | head_to_x |
| param_1+0x4C | +0x50 | current_speed (low dword of double) |
| param_1+0x54 | +0x58 | track_index |
| param_1+0x58 | +0x5C | point_index |
| param_1+0x5F | +0x63 | is_on_track |

## TechnoClass Vtable — Verified Offset Map (via UnitClass vtable at 0x7F5C70)

Every offset used by the Drive locomotor, verified by reading UnitClass vtable
entries and decompiling each target function.

| Offset | Address | Name | Purpose |
|--------|---------|------|---------|
| +0x048 | 0x5F65A0 | ObjectClass__GetCoords | Copy pos (X,Y,Z) from +0x9C/A0/A4 |
| +0x084 | 0x6F3270 | GetTechnoType (trampoline→0x741490) | Returns TypeClass* from this+0x6C4 |
| +0x0F0 | 0x7441B0 | ObjectClass__Mark_Occupation | Sets bit 0x20 in cell occupancy flags |
| +0x0F4 | 0x744210 | ObjectClass__Clear_Occupation | Clears bit 0x20 in cell occupancy flags |
| +0x124 | 0x4D3780 | TechnoClass__Cloak | Re-evaluate cloaking state after move |
| +0x184 | 0x5B3040 | MissionClass__GetCurrentMission | Returns *(this+0xAC), fallback *(this+0xB4) |
| +0x1AC | 0x73F0A0 | UnitClass__Can_Enter_Cell | 465-line cell passability check, returns 0-7 |
| +0x1BC | 0x5F6960 | ObjectClass__GetOccupiedCell | Returns CellClass* for current cell |
| +0x1D0 | 0x5F5F30 | ObjectClass__GetHeight | Returns *(this+0xA4) = Z coordinate |
| +0x1D4 | 0x70C5B0 | Deploy state flag A | Returns *(this+0x270), blocks movement |
| +0x1D8 | 0x70C5C0 | Deploy state flag B | Returns *(this+0x271), blocks movement |
| +0x2CC | 0x4D3810 | FootClass__CanReachDestination | Zone map reachability check |
| +0x37C | 0x746C90 | UnitClass__IsCrashing | *(this+0x6D8)!=-1 OR *(this+0x504)>0 |
| +0x380 | 0x4DE770 | IsInRearmTimer | Weapon rearm timer check (NOT IsSinking) |
| +0x480 | 0x741970 | TechnoClass__Scatter | Full scatter logic, 921 lines |
| +0x484 | 0x738970 | Scatter_Force | Forced scatter variant, 128 lines |
| +0x534 | 0x7416A0 | PerCellProcess / Enter_Cell | Crush/scatter on cell entry |
| +0x538 | 0x4DB1A0 | FootClass__GetCurrentSpeed | Speed in leptons/frame from type |
| +0x544 | 0x4D3710 | TechnoClass__SetSpeedPercentage | Stores speed % at +0x578/0x57C |

**Correction:** vtable+0x380 is `IsInRearmTimer` (weapon busy), NOT `IsSinking`
as previously documented. It checks `CurrentFrame - *(this+0x6A0) < *(this+0x6A8)`.

## Complete TurnTrack[72] Table (0x007E7B28)

Verified by reading all 864 bytes from binary. Each entry: 12 bytes
`{ u8 normal_track, u8 short_track, u8[2] pad, i32 direction, i32 flags }`.

Index = `from_facing * 8 + to_facing`. Facings: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW.

### Standard Tracks (0-63): 8x8 direction matrix

```
From  To    normal short dir  flags  Description
──────────────────────────────────────────────────
N     N     1      0     0x00   0    Straight north
N     NE    3      7     0x20   8    45° turn, cell-crossing
N     E     4      9     0x40   8    90° turn, cell-crossing
N     SE    0      0     0x60   0    No curve (>90°)
N     S     0      0     0x80   0    No curve (180°)
N     SW    0      0     0xA0   0    No curve (>90°)
N     W     4      9     0xC0  10    90° turn, mirrored+cell
N     NW    3      7     0xE0  10    45° turn, mirrored+cell
──────────────────────────────────────────────────
NE    N     6      8     0x00  15    Wide turn, all mirrors+cell
NE    NE    2      0     0x20   0    Straight NE diagonal
NE    E     6      8     0x40   8    Wide turn, cell-crossing
NE    SE    5     10     0x60   8    45° turn, cell-crossing
NE    S     0      0     0x80   0    No curve (>90°)
NE    SW    0      0     0xA0   0    No curve (180°)
NE    W     0      0     0xC0   0    No curve (>90°)
NE    NW    5     10     0xE0  15    45° turn, all mirrors+cell
──────────────────────────────────────────────────
E     N     4      9     0x00  15    90° turn, all mirrors+cell
E     NE    3      7     0x20  15    45° turn, all mirrors+cell
E     E     1      0     0x40   3    Straight E (swap XY+negate X)
E     SE    3      7     0x60  11    45° turn, negate X+Y+cell
E     S     4      9     0x80  11    90° turn, negate X+Y+cell
E     SW    0      0     0xA0   0    No curve (>90°)
E     W     0      0     0xC0   0    No curve (180°)
E     NW    0      0     0xE0   0    No curve (>90°)
──────────────────────────────────────────────────
SE    N     0      0     0x00   0    No curve (>90°)
SE    NE    5     10     0x20  12    45° turn, negate Y+cell
SE    E     6      8     0x40  12    Wide turn, negate Y+cell
SE    SE    2      0     0x60   4    Straight SE (negate Y)
SE    S     6      8     0x80  11    Wide turn, negate X+Y+cell
SE    SW    5     10     0xA0  11    45° turn, negate X+Y+cell
SE    W     0      0     0xC0   0    No curve (>90°)
SE    NW    0      0     0xE0   0    No curve (>90°)
──────────────────────────────────────────────────
S     N     0      0     0x00   0    No curve (180°)
S     NE    0      0     0x20   0    No curve (>90°)
S     E     4      9     0x40  12    90° turn, negate Y+cell
S     SE    3      7     0x60  12    45° turn, negate Y+cell
S     S     1      0     0x80   4    Straight S (negate Y)
S     SW    3      7     0xA0  14    45° turn, negate X+Y+cell
S     W     4      9     0xC0  14    90° turn, negate X+Y+cell
S     NW    0      0     0xE0   0    No curve (>90°)
──────────────────────────────────────────────────
SW    N     0      0     0x00   0    No curve (>90°)
SW    NE    0      0     0x20   0    No curve (180°)
SW    E     0      0     0x40   0    No curve (>90°)
SW    SE    5     10     0x60   9    45° turn, swap XY+cell
SW    S     6      8     0x80   9    Wide turn, swap XY+cell
SW    SW    2      0     0xA0   1    Straight SW (swap XY)
SW    W     6      8     0xC0  14    Wide turn, negate X+Y+cell
SW    NW    5     10     0xE0  14    45° turn, negate X+Y+cell
──────────────────────────────────────────────────
W     N     4      9     0x00  13    90° turn, swap+negate+cell
W     NE    0      0     0x20   0    No curve (>90°)
W     E     0      0     0x40   0    No curve (180°)
W     SE    0      0     0x60   0    No curve (>90°)
W     S     4      9     0x80   9    90° turn, swap XY+cell
W     SW    3      7     0xA0   9    45° turn, swap XY+cell
W     W     1      0     0xC0   1    Straight W (swap XY)
W     NW    3      7     0xE0  13    45° turn, swap+negate+cell
──────────────────────────────────────────────────
NW    N     6      8     0x00  13    Wide turn, swap+negate+cell
NW    NE    5     10     0x20  13    45° turn, swap+negate+cell
NW    E     0      0     0x40   0    No curve (>90°)
NW    SE    0      0     0x60   0    No curve (180°)
NW    S     0      0     0x80   0    No curve (>90°)
NW    SW    5     10     0xA0  10    45° turn, mirrored+cell
NW    W     6      8     0xC0  10    Wide turn, mirrored+cell
NW    NW    2      0     0xE0   2    Straight NW (negate X)
```

### Special Tracks (64-71)

```
Idx  normal short dir  flags  Description
64   11     11    0xA0   0    Special A (SW)
65   12     12    0xA0   0    Special B (SW)
66   13     13    0xA0   0    Special C (SW, long 68-pt curve)
67   14     14    0x20   0    Special D (NE)
68   14     14    0x60   4    Special D (SE, negate Y)
69   14     14    0xA0   1    Special D (SW, swap XY)
70   14     14    0xE0   2    Special D (NW, negate X)
71   15     15    0xC0   0    Special E (W)
```

### Key Patterns

- **normal_track=0** → no smooth curve available, unit does stop-rotate-go
- **Turns >90° are impossible** — all entries with >90° difference have normal_track=0
- **6 base curves** (tracks 1-6) generate all 64 standard entries via transform flags
- **flags & 8** = track crosses a cell boundary (requires Can_Enter_Cell validation)
- **Short tracks** (7-10) used for high-speed vehicles; `short_track=0` means no short variant

## Force_Track Detail (0x004B0C40)

Forces the locomotor onto a specific track, bypassing Process_Movement.
Used by deploy/unload scripts and MCV placement.

```
Force_Track(track_number, target_x, target_y, target_z):
    track_index = track_number
    point_index = 0

    if target == NullCoord: return

    if head_to != NullCoord:
        head_to = NullCoord
        is_on_track = 0

    is_on_track = 1
    head_to = target

    cell = GetCellAt(target)
    can_enter = Can_Enter_Cell(cell)

    if can_enter AND NOT is_falling:
        Apply_Track_Delta(target, mode=1)    // mark occupation
        destination = target
        current_speed = 1.0                  // forced, not terrain-adjusted; writes BOTH
                                              // dwords of the +0x50 double (residual_ticks
                                              // at +0x4C is NOT touched — corrected 2026-07-18,
                                              // see Detailed Function Decompilations section)
    elif is_alive:                           // corrected 2026-07-18: was "elif is_dead" (inverted)
        head_to = NullCoord
        is_on_track = 0
    // else (dead AND blocked/falling): no action at all
```

## Is_Moving Three-Tier Check (0x004AFB80)

```
1. dest != NullCoord?           → true (has destination)
2. head_to == NullCoord?        → false (no waypoint)
3. head_to.XY == current.XY?    → false (arrived)
   else                         → true (still en route)
```

Note: only checks X and Y in step 3 — Z is ignored. A unit at the same XY
but different Z (bridge vs ground) reports "not moving."

## Mark/Unmark Occupation (vtable +0xF0 / +0xF4)

In Apply_Track_Delta, the `mode` parameter controls cell marking:
- **mode 0**: calls vtable+0xF4 (`Clear_Occupation`) — unmark old cell
- **mode 1 or 3**: calls vtable+0xF0 (`Mark_Occupation`) — mark new cell

### Mark_Occupation (0x7441B0) — Decompiled

```
cell = GetCellAt(techno.position)
ground_height = GetGroundHeight(techno.position)
if (techno.Z > ground_height + BridgeThreshold) AND (cell.flags & 0x100):
    cell+0x128 |= 0x20          // bridge-level occupation bit
else:
    cell+0x124 |= 0x20          // ground-level occupation bit
```

### Clear_Occupation (0x744210) — Decompiled

```
cell = GetCellAt(techno.position)
ground_height = GetGroundHeight(techno.position)
if (techno.Z > ground_height + BridgeThreshold):
    cell+0x128 &= ~0x20         // clear bridge bit
else:
    cell+0x124 &= ~0x20         // clear ground bit
```

### CellClass Occupation Flags Layout

The cell has **two separate flag words** for ground and bridge layers:

| Offset | Name | Bit 0x20 Meaning |
|--------|------|-----------------|
| +0x124 | ground_occupation_flags | Unit present at ground level |
| +0x128 | bridge_occupation_flags | Unit present on bridge |

**Bridge threshold**: `DAT_00b1d0ac` — if unit Z exceeds ground_height + this threshold
AND cell has bridge flag (bit 8 in cell+0x140), the bridge layer is used.

**Bit layout** (verified from `CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md`):
- Bits 2-4 (0x1C): Infantry sub-cell occupation (NE/SW/SE)
- Bit 5 (0x20): Vehicle occupation
- Bit 6 (0x40): Building presence

**CORRECTION:** TechnoClass+0x74 was previously labeled "facing_lock" — it is actually
**cloak_state**. The save/restore pattern in Process_Drive_Track prevents unnecessary
cloak/uncloak cycles during sub-cell movement within the same cell. Full cloak transitions
only happen when crossing cell boundaries.

**Bridge layer asymmetry:** `Mark_Occupation` checks BOTH height + bridge flag (0x100).
`Clear_Occupation` only checks height. This is deliberate — handles bridge destruction
while a unit is on it.

### Cell Transition Sequence (complete order)

```
1. DoCloak(0)           — remove from old cell's cloak tracking
2. Set_Coords           — update position (object+0x9C/0xA0/0xA4)
3. Bridge ramp detect   — check height diff == 4 levels + flag 0x100
4. PerCellProcess       — crush occupants, scatter infantry
5. DoCloak(1)           — add to new cell's cloak tracking
6. Set_Height           — adjust Z for bridge elevation
7. Cell triggers        — fire actions 7, 0x30, 0x1D
```

### Cell Object Lists

Two singly-linked lists per cell (linked via object+0x30 NextObject):
- **cell+0xE4**: Ground-level objects
- **cell+0xE8**: Bridge-level objects

Buildings are appended at the **tail**; all other objects prepended at the **head**.

**Full CellClass struct layout** (328 bytes): see `CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md`.

### PerCellProcess / Enter_Cell (0x7416A0) — Summary

Called when a vehicle enters a cell. Key behaviors:
1. **Bridge detection**: If cell has bridge (flags & 0x100) and height diff == 4, set bridge flag
2. **Crush processing**: If Crusher flag or HasWeaponAbility(0x11):
   - Iterates cell object list (bridge: cell+0xE8, ground: cell+0xE4)
   - For each crushable object: spawn crush animation, apply damage, remove
   - Special: if infantry following this unit (tow_target == self), picks up infantry
3. **Scatter on approach**: If entry mode is "approach" (param_3 != 0), scatter objects
   with infantry bits (0x1F) set in the appropriate layer
4. **Turret twitch**: Sets `turret_rotation = -0.05f` on successful crush

### CDTimerClass Layout (12 bytes)

Full details: `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`

```
+0x00  int  start_frame      (set to g_CurrentFrameCounter by Init)
+0x04  int  field_04          (unused? or pause state)
+0x08  int  duration          (total timer length in frames)
```

**Init** (0x46b640): `start_frame = CurrentFrame; duration = param`
**GetTimeRemaining** (0x426630): `remaining = duration - (now - start_frame)`; if start_frame==-1, returns raw duration (paused). Clamped to 0.
**Remaining** (0x4c9480): Returns 1 if time left, 0 if expired. Used for boolean checks.

Purely computed — no self-updating. Just stores the start frame and compares with g_CurrentFrameCounter.

### RateTimer / FacingClass (22 bytes)

```
+0x00  short  desired_facing    (target)
+0x02  short  desired_high      (high word, usually 0)
+0x04  short  saved_facing      (snapshot of facing when Set was called)
+0x06  short  saved_high
+0x08  CDTimer(12)              (embedded countdown timer)
+0x14  short  rate              (facing units per frame)
```

**RateTimer::Set** (0x4c9220): Snapshots current interpolated facing to saved_facing,
sets new desired_facing, computes timer duration = `abs(delta) / rate`.

**RateTimer::Current** (0x4c93d0): Returns interpolated facing:
`result = desired - (delta / step_size) * time_remaining`. Smoothly interpolates
**backward** from target facing. When timer expires, returns desired_facing directly.

**Do_Turn** (0x4b0ef0): Trivial wrapper that calls `RateTimer::Set`.

### Zone Map System (13 Categories)

Full details: `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`

**Two-level lookup:**
1. Cell → zone_cluster_id (via cell_data array at MapClass+0x68)
2. Zone_cluster_id → zone_id (via per-category array at MapClass+0x18)

**13 MovementZone values** (CORRECTED — +0x5B4 is the raw MovementZone enum, NOT a
computed combination of SpeedType+MovementZone as previously stated):
TechnoTypeClass+0x5B4 = `MovementZone=` INI key, parsed via string table at 0x81ba88.

**Passability matrix** at `0x82a594`: 13 rows × 8 columns of dwords (1=passable, 2=impassable, 3=special).

**Zone computation**: Flood-fills connected cell clusters per passability category
in `MapClass::UpdateBridgeZonesHelper` (0x56c510).

**Bridge zones**: Separate invalidate/validate functions update zone edges when bridges
are built/destroyed.

### Movement Delay System

Two separate delay timer systems, both CDTimerClass:

**Movement delay** (FootClass+0x640/0x644/0x648):
- Started when pathfinding is attempted and fails
- Controls pacing between retry attempts
- Duration based on path_stuck_counter

**Blocked delay** (FootClass+0x668/0x66C/0x670):
- Started when Can_Enter_Cell returns code 2 (temporarily blocked by friendly)
- Duration from `Rules+0x1768` (BlockedDelay)
- When expired: triggers aggressive re-pathfinding with scatter flag = 2
- Blocked flag at FootClass+0x6B7 tracks current blocked state

## RawTrack[16] Point Counts (verified from pointer gaps and binary data)

| Track | Points | Entry | Jump | Type | Facing Range | Description |
|-------|--------|-------|------|------|-------------|-------------|
| 0 | 0 | — | — | Null | — | Empty |
| 1 | 23+s | 0 | -1 | Straight cardinal | 0→0 (N) | Straight N, y decreases 245→-8 |
| 2 | 31+s | 0 | -1 | Straight diagonal | 32→32 (NE) | Straight NE, x/y decrease equally |
| 3 | 54+s | 12 | 22 | 45° cardinal→diagonal | 0→32 | N→NE turn with cell crossing |
| 4 | 38+s | 11 | 19 | 90° cardinal→cardinal | 0→64 | N→E turn with cell crossing |
| **5** | **61+s** | **15** | **31** | **90° diagonal→diagonal** | **32→96 (NE→SE)** | **CORRECTED: was labeled 45°** |
| **6** | **56+s** | **16** | **27** | **45° diagonal→cardinal** | **32→64 (NE→E)** | **CORRECTED: was labeled 90°** |
| 7 | 27+s | 0 | -1 | Short 45° cardinal | 0→32→0 | Symmetric curve, returns to N |
| 8 | 21+s | 0 | -1 | Short 90° diagonal | 32→64 | NE→E short curve |
| 9 | 30+s | 0 | -1 | Short 90° cardinal | 0→~21→0 | Gentle curve with return |
| 10 | 27+s | 0 | -1 | Short 45° diagonal | 32→64 | NE→E short curve |
| 11 | 13+s | 0 | -1 | Dock approach A (SW) | 160 (fixed) | Straight SW refinery approach |
| 12 | 12+s | 0 | -1 | Dock approach B (SW) | 160 (fixed) | Reverse of track 11 (exit) |
| 13 | 68+s | 0 | -1 | Dock approach C (E) | 64 (fixed) | Long E approach curve (refinery) |
| 14 | 15+s | 0 | -1 | Dock approach D (NE) | 32 (fixed) | Short NE approach (refinery) |
| 15 | 16+s | 0 | -1 | Dock undock (S→W) | 128→188 | S→W undock curve |

**Corrections applied:**
- Track 5 description changed from "45° turn (diagonal→cardinal)" to "90° turn (diagonal→diagonal)" — facing goes 32→96 (NE→SE)
- Track 6 description changed from "90° turn (diagonal→diagonal)" to "45° turn (diagonal→cardinal)" — facing goes 32→64 (NE→E)
- Tracks 11-15 now identified as refinery dock/undock curves (see Special Tracks 64-71 section)

### Track Point Data — Characteristic Samples

**Track 5 (90° diagonal, 62 pts):** Starts NE (-504,-8,f=32), straight diagonal lead-in for 15 pts,
curve begins at entry_index=15, facing accelerates 32→35→38→41...→96, exits as straight SE.
Cell crossing at jump_index=31.

**Track 6 (45° diagonal→cardinal, 57 pts):** Starts NE (-512,256,f=32), straight diagonal lead-in for
16 pts, curve begins at entry_index=16, facing transitions 32→35→37...→64, exits straight E.
Cell crossing at jump_index=27.

**Track 7 (short 45° cardinal, 28 pts):** Symmetric looping curve: starts at (-1,6,f=0), curves to
facing ~30 at midpoint, then curves back to facing 32 (NE). Used for high-speed turns.

**Track 11 (dock approach SW, 14 pts):** Straight SW approach at facing 160. x increases 0→96,
y decreases 256→85. Used by refinery docking (track indices 64-66).

**Track 13 (dock approach E, 69 pts):** Longest track. Extremely gradual E approach at facing 64.
x from -670 to 0, y from -68 to -1. Used by refinery docking (track index 66, "Special C").

**Track 15 (undock S→W curve, 17 pts):** Starts facing 128 (S), curves through 132→188 toward
W. x from 128→0, y from -128→-4. Used by refinery undocking (track index 71).
| 13 | 68+sentinel | 0 | -1 | -1 | Special C (longest) |
| 14 | 15+sentinel | 0 | -1 | -1 | Special D |
| 15 | 16+sentinel | 0 | -1 | -1 | Special E |

Binary TrackPoint is 12 bytes `{i32 x, i32 y, i32 facing}`. The Rust code
correctly packs these into `{i16 x, i16 y, u8 facing}` — values fit.

**Correction:** The "Jump" column for tracks 5 and 6 in this table differs from
`DRIVE_TRACK_SYSTEM.md`. The verified binary values from `g_DriveTrackData_Array` are:
- Track 5: total_count=45, entry_index=15, **jump_index=31**
- Track 6: total_count=44, entry_index=16, **jump_index=27**

---

## LocomotionClass Base Class Layout

All locomotor classes (Drive, Ship, Walk, Fly, etc.) inherit from `LocomotionClass`.
Constructor at `0x0055a6c0`, destructor at `0x0055a6f0`.

```
+0x00  IUnknown vtable          (class-specific)
+0x04  ILocomotion vtable       (class-specific)
+0x08  linked_techno_1          (FootClass*, set by Link_To_Object)
+0x0C  linked_techno_2          (FootClass*, same value as +0x08)
+0x10  is_powered               (byte, init=1)
+0x11  flag_11                  (byte, init=1)
+0x14  ref_count                (int, InterlockedIncrement/Decrement)
+0x18  IPiggyback vtable        (class-specific, for Drive at +0x18)
```

**Link_To_Object** (vtable slot 3, `0x0055a710`): Stores the FootClass pointer at both
`+0x08` and `+0x0C`. Both are used interchangeably in decompilations.

**AddRef** (`0x0055a950`): `InterlockedIncrement(this+0x14)`, also increments global
locomotor count at `0x00abcd3c`.

**Release** (`0x0055a970`): `InterlockedDecrement(this+0x14)`, if zero calls
`ScalarDeletingDestructor(1)` to deallocate. Also decrements global count.

### Base Class Default Method Implementations

These LocomotionClass methods provide defaults; subclasses override as needed.

| Address | Name | Return | Notes |
|---------|------|--------|-------|
| 0x0055a710 | Link_To_Object | 0 (S_OK) | Stores techno at +0x08 and +0x0C |
| 0x0055a8c0 | Draw_Point | {0, z_adj} | Calls techno vtable+0x1C8, then AdjustForZ |
| 0x0055a8f0 | Power_On | void | Sets byte at +0x10 to 1, calls Is_Powered on self |
| 0x0055a910 | Power_Off | void | Sets byte at +0x10 to 0, calls Is_Powered on self |
| 0x0055a930 | Is_Powered | byte | Returns `*(this+0x0C)` (the powered flag) |
| 0x0055a940 | Is_Ion_Sensitive | false | Always returns 0 |
| 0x0055a950 | AddRef | int | InterlockedIncrement(ref_count) |
| 0x0055a970 | Release | int | InterlockedDecrement, free on zero |
| 0x0055a9b0 | QueryInterface | HRESULT | Compares against IID_IUnknown, IID_ILocomotion, IID_IPiggyback |
| 0x0055ab70 | Push | false | Stub, always returns 0 |
| 0x0055ab80 | Shove | false | Stub, always returns 0 |
| 0x0055ab90 | Tilt_Pitch_AI | void | No-op |
| 0x0055abb0 | Z_Gradient | 2 | Returns 2 (Deg45 gradient — used for all ground units) |
| 0x0055abc0 | Visual_Character | 0 | Always returns 0 (Normal) |
| 0x0055abd0 | Shadow_Point | {0, 0} | Returns zero offset |
| 0x0055abe0 | Is_To_Have_Shadow | 1 | Always returns true |
| 0x0055abf0 | Can_Enter_Cell | 0 | Base stub, always returns 0 (OK) |
| 0x0055ac00 | Force_Immediate_Destination | void | No-op stub |
| 0x0055acf0 | Drawing_Code | 0 | Always returns 0 |
| 0x0055ad00 | Can_Fire | 0 | Always returns 0 (no locomotor fire restriction) |
| 0x0055ad10 | Apparent_Speed | int | Delegates to techno vtable+0x538 (GetCurrentSpeed) |

---

## IPiggyback Interface — Complete Vtable (0x007e7e8c)

The IPiggyback COM interface allows one locomotor to temporarily replace another.
Used when a Chrono Legionnaire warps a unit: the TeleportLocomotionClass piggybacks
on top of the DriveLocomotionClass, which is saved and restored after the warp.

### Vtable Layout

| Slot | Offset | Address | Name | Purpose |
|------|--------|---------|------|---------|
| 0 | +0x00 | 0x4b4dc0 | QueryInterface | Thunk → 0x4af720, adds IPiggyback GUID check |
| 1 | +0x04 | 0x4b4dd0 | AddRef | Thunk → 0x4b4cb0 → LocomotionClass__AddRef |
| 2 | +0x08 | 0x4b4de0 | Release | Thunk → 0x4b4cc0 → LocomotionClass__Release |
| 3 | +0x0C | 0x4af8e0 | **Begin_Piggyback** | Store locomotor at +0x68, AddRef it |
| 4 | +0x10 | 0x4af930 | **End_Piggyback** | Return stored locomotor, clear +0x68 |
| 5 | +0x14 | 0x4af970 | **Is_Ok_To_End** | True if: Is_Moving()==false AND has speed AND flag +0x4D AND deploy_state==0 |
| 6 | +0x18 | 0x4af610 | **Piggybacker_CLSID** | Get CLSID of piggybacked locomotor (via IPersist QI) |
| 7 | +0x1C | 0x4b4cd0 | **Is_Piggybacking** | Returns `this+0x68 != 0` (has saved locomotor) |

### Piggybacking Lifecycle

**Begin_Piggyback** (`0x004af8e0`):
```
if (param_loco == NULL) return E_POINTER (0x80004003)
if (this+0x68 != 0) return E_FAIL (0x80004005)     // already piggybacking
this+0x68 = param_loco
param_loco->AddRef()
return S_OK (0)
```

**End_Piggyback** (`0x004af930`):
```
if (out_param == NULL) return E_POINTER (0x80004003)
if (this+0x68 == 0) return S_FALSE (1)              // nothing piggybacked
*out_param = this+0x68
this+0x68 = NULL
return S_OK (0)                                      // caller must Release
```

**Is_Ok_To_End** (`0x004af970`):
```
if (Is_Moving() != false) return false
if (this+0x50 == 0) return false                     // no speed data
if (byte at this+0x4D == 0) return false             // flag not set
if (techno.deploy_state != 0) return false           // deploying
return true
```

**Destructor** (`0x004af5e0`): If `this+0x68` is non-null, calls `Release()` on the
piggybacked locomotor before destroying.

### Who Uses Piggybacking

The IPiggyback interface is queried via `IID_IPersist` (`{0000010C-0000-0000-C000-000000000046}`)
at `0x00818858`. Key callers that reference this IID:

| Address | Function | Purpose |
|---------|----------|---------|
| 0x0045aea0 | LocomotionClass__QueryInterface_IPiggyback | Helper: QI for IPiggyback on a locomotor |
| 0x00520f40 | FootClass__Locomotion_AI | Post-movement: checks if piggyback can end |
| 0x00742815 | TechnoClass__Set_Destination | Routes Set_Destination through piggyback |
| 0x0073a456 | UnitClass__Mission_Enter | Transport enter — piggyback state check |
| 0x0073e7b7 | UnitClass__Mission_Harvest | Chrono miner harvest piggyback check |
| 0x0041c250 | CoCreateInstance wrapper | Creates new locomotor by CLSID |

### Which Locomotor Classes Implement IPiggyback

| Class | Has IPiggyback | Piggybacked Ptr Offset |
|-------|---------------|----------------------|
| DriveLocomotionClass | **YES** | +0x68 |
| ShipLocomotionClass | **YES** | +0x68 |
| TeleportLocomotionClass | **YES** | +0x48 |
| JumpjetLocomotionClass | **YES** | (varies) |
| DropPodLocomotionClass | **YES** | +0x2C |
| WalkLocomotionClass | No | — |
| HoverLocomotionClass | No | — |
| MechLocomotionClass | No | — |
| RocketLocomotionClass | No | — |
| FlyLocomotionClass | No | — |

### Piggybacking Scenarios

**Scenario A — Chrono warp (Teleport replaces Drive):**
When a Chrono Legionnaire targets a unit, the engine creates a `TeleportLocomotionClass`,
calls `Begin_Piggyback` on it (storing the unit's current DriveLocomotionClass), and installs
the teleport locomotor as the active one. When the warp completes, `FootClass::Locomotion_AI`
detects `Is_Ok_To_End() == true` and triggers `End_Piggyback` to restore the original Drive.

**Scenario B — Magnetron lift (replaced locomotor varies):**
Magnetron lifting temporarily replaces the unit's locomotor with a special movement mode.
Same Begin/End_Piggyback mechanism.

**Piggyback start** happens in `TechnoClass::Set_Destination` (0x742815):
1. QI current locomotor for IPiggyback
2. Create new locomotor via `CoCreateInstance` (0x41c250)
3. `Link_To_Object` on new locomotor
4. `Begin_Piggyback` on new locomotor, passing old as parameter
5. Install new locomotor as active

**Piggyback end** happens in `FootClass::Locomotion_AI` (0x520f40), every tick:
1. Check `Is_Ok_To_End()` on current locomotor
2. If true, QI for IPiggyback → `End_Piggyback()` → restore old locomotor
3. Compare piggybacker CLSID against known types to assign correct post-restore mission

### TeleportLocomotionClass IPiggyback Differences

`End_Piggyback` (0x719ee0): Clears owner FootClass destination coords (+0x428, +0x42C)
before restoring — prevents unit from continuing to a chrono-destination after unwarping.

`Is_Ok_To_End` (0x719f30): Stricter — also checks timer flag (+0x1D), ChronoInTransit
(+0x27C), and locomotor state (+0x20) == 0.

---

## Static Initialization Functions

These functions run during `gamemd.exe` static init (referenced from init table at `0x00812d40`).
They compute runtime constants from compiled-in math constants.

### Constants Used in Initialization

| Address | Type | Value | Meaning |
|---------|------|-------|---------|
| 0x007e6238 | double | 0.01745329 (π/180) | Degrees to radians |
| 0x007e1730 | double | 90.0 | Angle constant |
| 0x007e1740 | double | 0.00390625 (1/256) | Lepton fraction |

### Init Functions

| Address | Name | Logic | Output |
|---------|------|-------|--------|
| 0x4af3e0 | StoreTileHeight | `DAT_008a0758 = (π/180) * 90.0` → π/2 | Runtime double at 0x008a0758 |
| 0x4af400 | InitHeightStep_A | `g_DriveHeightStep = ftol(sin(DAT_008a0758 - DAT_008a0780))` | HeightStep at 0x008a07d0 |
| 0x4af440 | ComputeFromHeightStep | `DAT_008a0788 = atan(HeightStep * 1/256)` | Angle for slope |
| 0x4af470 | ComputeBridgeRenderOffset | `DAT_008a07a0 = atan(HeightStep*2 / DAT_008a0778)` | Bridge render |
| 0x4af4a0 | ComputeBridgeZOffset | `g_BridgeZOffset = ftol(HeightStep * 4)` | Bridge = 4 height levels |
| 0x4af4d0 | InitNullCoords2 | `DAT_008a0770 = 0; DAT_008a0772 = 0` | Null cell coord |
| 0x4af4e0 | InitNullCoords | `g_NullCoord = {0, 0, 0}` | Null coord sentinel |
| 0x4af500 | InitHeightStep2 | `DAT_008a07b8 = 0x80; DAT_008a07bc = 0x80; DAT_008a07c0 = 0` | Cell center |
| 0x4af520 | InitSomething3 | Zeroes 4 ints at 0x008a0760-0x008a076c | Scratch state |

### Runtime Global Variables

| Address | Type | Name | Set By |
|---------|------|------|--------|
| 0x008a0758 | double | tile_height_angle | StoreTileHeight (= π/2) |
| 0x008a0760 | int[4] | scratch_state | InitSomething3 (zeroed) |
| 0x008a0770 | short[2] | null_cell_coord | InitNullCoords2 |
| 0x008a0780 | double | base_angle | (set elsewhere, used by InitHeightStep) |
| 0x008a0788 | double | slope_angle | ComputeFromHeightStep |
| 0x008a07a0 | double | bridge_render_offset | ComputeBridgeRenderOffset |
| 0x008a07b8 | int | cell_center_x (0x80=128) | InitHeightStep2 |
| 0x008a07bc | int | cell_center_y (0x80=128) | InitHeightStep2 |
| 0x008a07c0 | int | init_zero | InitHeightStep2 |
| 0x008a07d0 | int | g_DriveHeightStep | InitHeightStep_A |
| 0x00abcd3c | int | g_LocomotorGlobalRefCount | AddRef/Release |

### Verified Constants

**Sin/Cos lookup naming** (VERIFIED from binary — sin table[0]=0.0):
- `Sin_lookup` (0x4CAD00) adds +2048 phase → mathematically returns **cosine**
- `Cos_lookup` (0x4CACB0) no phase shift → mathematically returns **sine**
- Table at 0x84F084: 8192 float entries, one full circle. Names are counterintuitive but
  preserved from prior sessions. When using in new code, call `Sin_lookup` for cos and
  `Cos_lookup` for sin.

**1/7 interpolation factor** (0x7E7FA8): `3FC2492492492492` = 0.142857... = 1/7 CONFIRMED
(corrected 2026-07-18: previously transcribed as `3FC2492492244992`, two hex digit-pairs
transposed; re-verified via `read_memory 0x007e7fa8` — OFFSET_RETYPED_WRONG).

### Zone Passability Matrix (0x82A594, VERIFIED)

13 rows × 8 columns of i32. Values: 1=passable, 2=impassable, 3=bridge/special.

```
Cat  Col0 Col1 Col2 Col3 Col4 Col5 Col6 Col7
  0:  1    2    2    2    2    2    2    3
  1:  1    1    2    2    2    2    2    3
  2:  1    1    1    2    2    2    2    3
  3:  1    1    1    1    1    1    2    3
  4:  1    1    2    1    1    2    2    3
  5:  1    2    2    1    1    2    2    3
  6:  1    1    1    2    2    2    1    3
  7:  1    2    2    2    2    1    2    3
  8:  1    1    1    2    2    1    2    3
  9:  1    1    1    1    1    1    1    3
 10:  2    2    2    2    1    2    2    3
 11:  2    2    2    1    1    2    2    3
 12:  1    1    1    2    2    2    2    3
```

**Row index** = MovementZone enum (0-12, directly from TechnoTypeClass+0x5B4).
**Column index** = CellZoneType (0-7, from CellClass::RecalcZoneType at 0x483c80).

### MovementZone Enum (VERIFIED from string table at 0x81ba88)

| Index | Name | Passable Cols |
|-------|------|---------------|
| 0 | Normal | 0 only |
| 1 | Crusher | 0,1 |
| 2 | Destroyer | 0,1,2 |
| 3 | AmphibiousDestroyer | 0,1,2,3,4,5 |
| 4 | AmphibiousCrusher | 0,1,3,4 |
| 5 | **Amphibious** | 0,3,4 |
| 6 | Subterranean | 0,1,2 |
| 7 | Infantry | 0,6 |
| 8 | InfantryDestroyer | 0,4 |
| 9 | Fly | 0,1,2,3,4,5,6 (all) |
| 10 | Water | 4 only |
| 11 | WaterBeach | 3,4 |
| 12 | CrusherAll | 0,1,2 |

### Cell Zone Type (column index, from CellClass::RecalcZoneType)

| Col | ZoneType | Meaning |
|-----|----------|---------|
| 0 | Ground | Normal passable terrain |
| 1 | Road | Road overlay |
| 2 | Water (overlay) | Shallow water from overlay |
| 3 | Beach | Sandy beach terrain |
| 4 | Water (deep) | Deep water from tile |
| 5 | Building | Building present |
| 6 | Impassable | Blocked terrain |
| 7 | OutOfBounds | Map edge sentinel (always 3=special) |

### RUST IMPLEMENTATION BUG (from verification)

**MovementZone enum in `src/rules/locomotor_type.rs:218` is MISSING index 5 (Amphibious)**
and has incorrect variant ordering. The binary order is:
Normal(0), Crusher(1), Destroyer(2), AmphibiousDestroyer(3), AmphibiousCrusher(4),
**Amphibious(5)**, Subterranean(6), Infantry(7), InfantryDestroyer(8), Fly(9),
Water(10), WaterBeach(11), CrusherAll(12).

**SpeedType enum order also differs from Rust** — binary order:
Foot(0), Track(1), Wheel(2), **Hover(3)**, **Winged(4)**, **Float(5)**,
**Amphibious(6)**, **FloatBeach(7)**.
Check `src/rules/locomotor_type.rs` against this binary-verified order.

---

## Complete Function Catalog — All 50 DriveLocomotionClass Functions

Every function in the `0x004af3e0`–`0x004b4e00` range, organized by category.
All are labeled in Ghidra.

### Static Initializers (not class methods)

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| 0x4af3e0 | StoreTileHeight | ~30 | Compute tile angle = π/180 * 90 |
| 0x4af400 | InitHeightStep_A | ~40 | HeightStep = sin(angle diff) |
| 0x4af440 | ComputeFromHeightStep | ~40 | Slope angle from HeightStep |
| 0x4af470 | ComputeBridgeRenderOffset | ~30 | Bridge render offset angle |
| 0x4af4a0 | ComputeBridgeZOffset | ~20 | BridgeZ = HeightStep * 4 |
| 0x4af4d0 | InitNullCoords2 | ~16 | Zero null cell coord |
| 0x4af4e0 | InitNullCoords | ~20 | Zero null coord sentinel |
| 0x4af500 | InitHeightStep2 | ~20 | Cell center = 128, 128 |
| 0x4af520 | InitSomething3 | ~20 | Zero scratch state |

### Constructor / Destructor

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| 0x4af540 | Constructor | ~160 | Init all fields, set 3 vtable ptrs |
| 0x4af5e0 | Destructor | ~80 | Release piggybacked loco, call base dtor |
| 0x4b4d00 | ScalarDeletingDestructor | ~60 | RTTI destructor with optional dealloc |

### IPiggyback Methods

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| 0x4af610 | Piggybacker_CLSID | ~110 | QI piggybacked loco for IPersist → GetClassID |
| 0x4af720 | QueryInterface_With_IPiggyback | ~100 | Base QI + IPiggyback GUID check |
| 0x4af8e0 | Begin_Piggyback | ~80 | Store + AddRef piggybacked locomotor |
| 0x4af930 | End_Piggyback | ~60 | Return + clear piggybacked locomotor |
| 0x4af970 | Is_Ok_To_End | ~60 | Check if piggyback can safely end |

### State Query Methods

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| 0x4afb40 | Force_New_Slope | ~40 | Set slope facing, init turn timer |
| 0x4afb80 | Is_Moving | ~100 | 3-tier check: dest → head_to → XY match |
| 0x4afc20 | Is_Moving_Now | ~80 | CDTimer active OR has waypoint+speed |
| 0x4afc90 | Destination | ~30 | Returns dest coord from +0x34 |
| 0x4afcc0 | Head_To_Coord | ~50 | Returns head_to, fallback to techno pos |

### Movement Control Methods

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| 0x4afd40 | Set_Destination | ~120 | Set dest with bridge Z adj, 4 guard checks |
| 0x4afe00 | Stop_Moving | ~200 | Clamp speed 0.3, clear dest, convoy propagate |
| 0x4b04d0 | Update_Facing_From_Type | ~30 | Read ROT from TechnoType → SetFacing |
| 0x4b0ad0 | Apply_Track_Delta | ~280 | Apply track endpoint offset, mark/unmark cells |
| 0x4b0c40 | Force_Track | ~350 | Force onto specific track, bypass Process_Movement |
| 0x4b0ef0 | Do_Turn | ~30 | Forward to RateTimer facing interpolation |
| 0x4b4780 | Transform_Track_Coords | ~180 | Mirror/flip track deltas via 3-bit flags |
| 0x4b4890 | Stop_And_Scatter | ~80 | Stop + scatter, context-dependent |

### Core Tick Functions

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| **0x4b0500** | **Process** | **~1600** | **Main tick entry — orchestrates everything** |
| **0x4b0f20** | **Process_Drive_Track** | **~5860** | **Inner state machine — track stepping** |
| **0x4b2630** | **Process_Movement** | **~8500** | **Outer state machine — pathfinding + track select** |

### Rendering Methods

| Address | Name | Size | Purpose |
|---------|------|------|---------|
| 0x4aff60 | Draw_Matrix | ~600 | 3x4 VXL matrix: turn interp + slope pitch/roll |
| 0x4b0410 | Shadow_Matrix | ~120 | Shadow variant, forces facing=-1 on slope |

### Simple Stubs / Constant Returns

| Address | Name | Return | Purpose |
|---------|------|--------|---------|
| 0x4b4820 | In_Which_Layer | 2 | Layer::Ground always |
| 0x4b4870 | Z_Adjust | 0 | No Z adjustment |
| 0x4b4880 | Z_Gradient | 2 | Deg45 gradient (thunk → base 0x55abb0) |
| 0x4b48d0 | Mark_All_Occupation_Bits | void | Apply_Track_Delta on head_to if non-null |
| 0x4b4c60 | Get_Status | 0 | Always returns 0 |
| 0x4b4c70 | Acquire_Hunter_Seeker_Target | void | No-op |
| 0x4b4c80 | Is_Surfacing | false | Always returns 0 |
| 0x4b4cd0 | Is_Piggybacking | bool | Returns `this+0x68 != 0` |

### COM Interface Thunks

These are thin wrappers that adjust `this` pointer between IUnknown/ILocomotion/IPiggyback
interfaces on the same object.

| Address | Name | Thunks To |
|---------|------|-----------|
| 0x4b4cb0 | IUnknown_AddRef | → LocomotionClass__AddRef (0x55a950) |
| 0x4b4cc0 | IUnknown_Release | → LocomotionClass__Release (0x55a970) |
| 0x4b4d90 | ILocomotion_QueryInterface | → QueryInterface_With_IPiggyback (0x4af720) |
| 0x4b4da0 | ILocomotion_AddRef | → IUnknown_AddRef (0x4b4cb0) |
| 0x4b4db0 | ILocomotion_Release | → IUnknown_Release (0x4b4cc0) |
| 0x4b4dc0 | IPiggyback_QueryInterface | → QueryInterface_With_IPiggyback (0x4af720) |
| 0x4b4dd0 | IPiggyback_AddRef | → IUnknown_AddRef (0x4b4cb0) |
| 0x4b4de0 | IPiggyback_Release | → IUnknown_Release (0x4b4cc0) |

---

## Detailed Function Decompilations

### Is_Moving (0x004afb80) — Three-Tier Check

```
1. if dest != NullCoord → return TRUE (has destination)
2. if head_to == NullCoord → return FALSE (no waypoint)
3. if head_to.X == techno.pos.X AND head_to.Y == techno.pos.Y → return FALSE (arrived)
   else → return TRUE (en route)
```
Note: step 3 only checks X and Y — Z is ignored. A unit at the same XY but different Z
(bridge vs ground) reports "not moving."

### Is_Moving_Now (0x004afc20) — Immediate Movement Check

```
1. if CDTimerClass__Remaining(slope_timer) != 0 → return TRUE (still in slope transition)
2. if Is_Moving() AND head_to != NullCoord:
      if techno.GetCurrentSpeed() > 0 → return TRUE
3. return FALSE
```

### Head_To_Coord (0x004afcc0) — Next Waypoint

```
if head_to == NullCoord:
    return techno.position (X,Y,Z from techno+0x9C/0xA0/0xA4)
else:
    return head_to (loco+0x40,0x44,0x48)
```

### Force_New_Slope (0x004afb40) — Slope Facing Init

Absolute object-base offsets (converted from decompiler `param_1` = ILocomotion
interface pointer = object_base+4; corrected 2026-07-18 — the original version of
this block wrote unconverted decompiler-relative offsets as if they were absolute,
which collided with +0x18 = IPiggyback vtable ptr and mislabeled +0x24 as
"uninitialized slope timer remaining" when the CurrentFrameCounter write actually
lands on +0x24, not +0x20. Verified via `decompile_function 0x4afb40` — PARAM1_TYPE_MISREAD):

```
+0x1C (cached_slope_index)      = param_facing
+0x20 (previous_slope_index)    = param_facing        // both old and new set to same
+0x24 (frame_stamp / CDTimer.start_frame) = g_CurrentFrameCounter
+0x28 (slope_timer_remaining / CDTimer.field_04) = <uninitialized local var, written as-is>
+0x2C (slope_timer_total / CDTimer.duration) = 0
+0x30 (unknown_30)              = 0
```

### Stop_And_Scatter (0x004b4890) — Context-Dependent Stop

```
if techno.tether_target == 0:           // techno+0x598 (index 0x166)
    techno.Scatter(0, 1)                // vtable+0x480, no direction
else:
    FootClass__Stop_Moving()            // 0x4df0d0: zero movement delta
    techno.Scatter_Force(0, 1)          // vtable+0x484
```

### Apply_Track_Delta (0x004b0ad0) — Cell Marking at Track End

Two-stage operation:
1. **Track end-point delta**: If track is active, reads the track's `total_count` point,
   transforms it via `Transform_Track_Coords`, computes world position.
   - `mode=0`: calls techno vtable+0xF4 (`Clear_Occupation`) to unmark old cell
   - `mode=1 or 3`: calls techno vtable+0xF0 (`Mark_Occupation`) to mark new cell
2. **Target coord fallback**: If no track delta, marks/unmarks using the provided
   coordinate directly.

### Force_Track (0x004b0c40) — Bypass Normal Pathing

Used by deploy scripts, MCV placement, and forced movement. Sets track directly:
```
this+0x58 = track_number        // track_index
this+0x5C = 0                   // point_index reset

if target == NullCoord: return

// Clear existing head_to
if head_to != NullCoord:
    head_to = NullCoord
    is_on_track = 0

// Validate target cell
is_on_track = 1
head_to = target
cell = GetCellAt(target)
can_enter = Can_Enter_Cell(cell)        // via FUN_00481a00 (CellClass__Can_Enter_Cell_General;
                                         // confirmed by disassembly, NOT the decompiler's
                                         // "CrateClass__PickupDispatch" display label — stale
                                         // Ghidra label, verified via `disassemble_function 0x4b0c40`
                                         // showing CALL 0x00481a00 directly)

if can_enter AND NOT is_falling:
    Apply_Track_Delta(target, mode=1)   // mark new occupation
    destination = target
    current_speed = 1.0                 // forced full speed — writes BOTH dwords of the
                                         // +0x50 double (EBP+0x4c=0x00000000 low dword,
                                         // EBP+0x50=0x3FF00000 high dword, interface-relative,
                                         // = absolute +0x50/+0x54). residual_ticks at absolute
                                         // +0x4C is NEVER written by Force_Track — corrected
                                         // 2026-07-18 (was "residual_ticks = 0", fabricated).
elif is_alive:                          // corrected 2026-07-18: was "elif is_dead" (inverted —
                                         // disassembly at 0x4b0d6a-0x4b0d94 shows the clear only
                                         // fires when techno+0x90 (is_alive) is NONZERO; if dead,
                                         // the function falls through to RET with no action)
    head_to = NullCoord
    is_on_track = 0
```
Both corrections verified via `disassemble_function 0x4b0c40` (raw assembly, not decompiler
pseudocode) — root cause PARAM1_TYPE_MISREAD (residual_ticks) and OPERATOR_OR_ORDER_DRIFT
(is_dead/is_alive inversion).

### Set_Destination (0x004afd40) — Guarded Destination Set

Four vtable guard checks must ALL return false before accepting:
```
if techno.IsCrashing() → abort       // vtable+0x37C
if techno.IsInRearmTimer() → abort   // vtable+0x380 (NOT IsSinking — corrected)
if techno.IsWarpingOut() → abort     // vtable+0x1D4
if techno.IsBeingWarped() → abort    // vtable+0x1D8

dest = {param_x, param_y, param_z}
if dest != NullCoord:
    cell = GetCellAt(dest)
    if cell.flags & 0x100:           // bridge present
        dest.Z += g_BridgeZOffset_Drive
```

### Stop_Moving (0x004afe00) — Gradual Stop with Convoy

```
// Convoy propagation (only if has_trailer and not already stopped)
if head_to != NullCoord AND type.IsTrain (TechnoTypeClass+0xC94) AND !convoy_state_flag:
    next = techno.next_in_convoy      // techno+0x6C8
    while next != NULL AND next != next.next:
        next.locomotor.Stop_Moving()  // vtable+0x48
        next = next.next

// Speed clamp — NOT instant stop
speed = min(current_speed, 0.3)       // DAT_007e6240 = 0.3
current_speed = speed

// Clear destination only
dest = NullCoord                      // does NOT clear head_to or track state
```

### Draw_Matrix (0x004aff60) — VXL Body Transform

Two rendering paths based on slope magnitude:

**Path 1 — Flat terrain** (|pitch| < 0.005 AND |roll| < 0.005 AND turn complete):
```
if slope_timer_total == 0 OR slope progress >= 1.0:
    facing_matrix = VXL_GetFacingMatrix(body_facing)
else:
    facing_matrix = VXL_InterpolatedFacing(body_facing, slope_progress)
result = facing_matrix  // facing_index preserved for VXL cache
```

**Path 2 — Sloped terrain** (has pitch/roll tilt):
```
sin_pitch = sin(techno.body_pitch)      // techno+0x32C
cos_pitch = cos(techno.body_pitch)
sin_roll  = sin(techno.body_roll)       // techno+0x328
cos_roll  = cos(techno.body_roll)

// Build slope rotation matrix
Matrix_shear_col3_by_col2()
Matrix_shear_col3_by_col0()
Matrix_shear_col3_by_col1()
Matrix_rotate_x_axis()
Matrix_rotate_y_axis()

// Force facing_index = -1 to disable VXL cache
facing_matrix = VXL_InterpolatedFacing(body_facing, slope_progress)
result = slope_matrix * facing_matrix
```

**Slope transition interpolation** (computed in both paths — controls 3-frame blend):
```
if slope_timer_total != 0:
    if slope_timer_start_frame != -1:
        elapsed = g_CurrentFrameCounter - slope_timer_start_frame
        remaining = max(0, slope_timer_remaining - elapsed)
    progress = (total - remaining) / total     // 0.0 → 1.0
else:
    progress = 1.0                             // instant completion
```

### Shadow_Matrix (0x004b0410) — Shadow Rendering

Simplified variant of Draw_Matrix:
```
// Same turn interpolation as Draw_Matrix
if has_slope_tilt OR turn not complete:
    facing_index = -1           // disable cache for shadows on slopes
return FUN_0055a7d0(this, matrix, facing_ptr)   // base class shadow builder
```

### Transform_Track_Coords (0x004b4780) — Mirror/Flip Track Points

Applies the 3-bit transform flags from `g_DriveTrackFlags_Table` to produce all
directional variants from 6 base curves:

```
flags = g_DriveTrackFlags_Table[track_index * 12]    // byte at +0x08 of TurnTrack entry
x = track_point.x
y = track_point.y
facing = track_point.facing

if flags & 1:       // Swap X↔Y, adjust facing
    facing = (-facing - 0x40) & 0xFF
    swap(x, y)

if flags & 2:       // Negate X, negate facing
    x = -x
    facing = (-facing) & 0xFF

if flags & 4:       // Negate Y, subtract 0x80 from facing
    y = -y
    facing = (-facing - 0x80) & 0xFF

// Add to cell origin
result.x = head_to.x + x
result.y = head_to.y + y
```

---

## Decompiler Offset Quick Reference

In all Ghidra decompilations of ILocomotion vtable methods, `param_1` is the
**ILocomotion interface pointer** = `object_base + 4`. Mapping:

| Decompiler Access | Object Offset | Field |
|-------------------|---------------|-------|
| param_1 + 0x08 | +0x0C | linked_techno |
| param_1 + 0x18 | +0x1C | cached_slope_index (CellClass+0x11C) |
| param_1 + 0x1C | +0x20 | previous_slope_index (corrected 2026-07-18: was `slope_timer_start_frame` — see Object Field Layout table for evidence) |
| param_1 + 0x20 | +0x24 | slope_timer_remaining (3-frame blend) |
| param_1 + 0x24 | +0x28 | slope_timer_remaining_cache |
| param_1 + 0x28 | +0x2C | slope_timer_total |
| param_1 + 0x2C | +0x30 | unknown_30 |
| param_1 + 0x30 | +0x34 | dest.X |
| param_1 + 0x34 | +0x38 | dest.Y |
| param_1 + 0x38 | +0x3C | dest.Z |
| param_1 + 0x3C | +0x40 | head_to.X |
| param_1 + 0x40 | +0x44 | head_to.Y |
| param_1 + 0x44 | +0x48 | head_to.Z |
| param_1 + 0x48 | +0x4C | current_speed (double, low dword — 8 bytes spanning +0x4C to +0x53) |
| param_1 + 0x4C | +0x50 | current_speed (double, high dword) |
| param_1 + 0x50 | +0x54 | track_index |
| param_1 + 0x54 | +0x58 | point_index |
| param_1 + 0x58 | +0x5C | is_reversed |
| param_1 + 0x59 | +0x5D | has_active_path |
| param_1 + 0x5A | +0x5E | deploy_flag |
| param_1 + 0x5B | +0x5F | is_on_track |
| param_1 + 0x5C | +0x60 | can_crush_flag |
| param_1 + 0x5D | +0x61 | first_tick_flag |
| param_1 + 0x60 | +0x64 | piggybacked_locomotor |

---

## FootClass::AI — Locomotor Integration (0x004da530)

375 lines, ~1500 bytes. Called every tick for all mobile units. This is the function that
dispatches to `ILocomotion::Process`.

### Execution Order

**BEFORE ILocomotion::Process:**

1. `TechnoClass::AI()` — parent class tick (0x6f9e50)
2. Early-out if unit is dead (`+0x90 == 0`)
3. Clear per-frame flag at `+0x6B3`
4. Self-heal on tiberium (every `Rules+0x1808` frames) — via vtable+0x16C (ReceiveDamage with negative damage)
5. Veteran rank-up with visual animation
6. Movement counter / fog update (every 16 frames)

**THE LOCOMOTOR CALL** (at address `0x4da877`):
```asm
CALL dword ptr [ECX + 0x40]    ; ILocomotion::Process, vtable slot 16
```

Guard conditions checked first:
- `techno+0x674` != 0 (locomotor pointer exists)
- `techno+0x3CD` == 0 (not in special state)
- `techno+0x8D` == 0 (not sinking)
- `techno+0x81` == 0 (not falling)
- Owner house flag check

**AFTER ILocomotion::Process:**

7. Movement counter increment if `Is_Moving()` returns true
8. Falling animation management
9. Rank transition / deploy animations
10. **IPiggyback locomotor swap**: Queries locomotor for IPiggyback. If `Is_Ok_To_End()`
    returns true, calls `End_Piggyback` to retrieve the saved locomotor, releases the
    current one, and installs the saved one. **This is how temporary locomotors (Magnetron
    lift, Chrono warp) get removed when done.**
11. `TryEnterTransport` if not falling
12. Team AI dispatch

### FootClass Fields Accessed by Locomotor Integration

| Offset | Type | Field | Accessed When |
|--------|------|-------|---------------|
| +0x090 | byte | is_alive | Early-out check |
| +0x538 | int | movement_counter | Incremented after Process if Is_Moving |
| +0x674 | ptr | locomotor_ptr (ILocomotion*) | Guard check + all dispatch |
| +0x6B3 | byte | per_frame_flag | Cleared each tick |

---

## Special Tracks 64-71 — Refinery Dock/Undock System

**Force_Track** (vtable slot 28, `0x004b0c40`) is called **exclusively** by building
docking logic. Special tracks 64-71 are not used during normal movement — they are
dedicated refinery dock/undock approach curves.

### Callers of Force_Track

| Address | Function | Tracks Used | Context |
|---------|----------|-------------|---------|
| 0x458e50 | BuildingClass::DockUnit | 67, 68, 69, 70 | Refinery docking — facing-based track selection |
| 0x4593a0 | BuildingClass::UndockUnit | 71 | Refinery undocking |
| 0x4595c0 | BuildingClass::FinishUndock | 71 | Undock completion variant |

### Docking Track Selection (by unit facing)

```
facing == 0x40  (N)  → Track 67 (special D, dir=0x20/NE)
facing == 0xC0  (E)  → Track 68 (special D, dir=0x60/SE, negate Y)
facing == 0x1C0 (S)  → Track 70 (special D, dir=0xE0/NW, negate X)
else (W/default)     → Track 69 (special D, dir=0xA0/SW, swap XY)
```

All four use raw track 14 (Special D, 15 points) with different transform flags.

### Undocking

Track 71 (raw track 15, Special E, 16 points, dir=0xC0/W) is always used for undocking.
The target coordinate is building center offset by (-128, +128) leptons.
Speed is forced to 1.0 during undock.

---

## Serialization — Load/Save (IPersistStream)

### Load (0x004af780, ~127 lines)

Deserializes DriveLocomotionClass from an IStream:
1. Restores vtable pointers (IUnknown, ILocomotion, IPiggyback)
2. Reads piggyback flag from stream
3. If piggybacked: calls `OleLoadFromStream` to deserialize the saved locomotor
4. Restores linked_techno pointer from stream

### Save (0x004af800, ~223 lines)

Serializes DriveLocomotionClass to an IStream:
1. Writes class state fields
2. Writes piggyback flag
3. If piggybacking: calls `OleSaveToStream` to serialize the piggybacked locomotor
4. Writes linked_techno reference

### ClassFactory (0x006c4010)

COM factory that creates DriveLocomotionClass instances:
```
1. operator_new(0x70)                   // 112 bytes = 0x6C padded to 0x70
2. DriveLocomotionClass__Constructor(mem)
3. QueryInterface(mem, requested_iid, out_ptr)
```
Factory pointer stored at `0x007f3c84`. Looked up by CLSID when `Locomotor=` INI key
matches `{4A582741-9839-11d1-B709-00A024DDAFD1}`.

---

## Extended ILocomotion Vtable — Additional Slots (40-49)

Verified from the vtable at 0x007e7eb0. These slots extend beyond the standard 40-slot
ILocomotion interface and are Drive-specific.

| Slot | Offset | Address | Name | Purpose |
|------|--------|---------|------|---------|
| 40 | +0xA0 | 0x4b4920 | Is_To_Have_Shadow_Override | Checks if at track end; computes height delta from ground |
| 41 | +0xA4 | 0x4b4b00 | Can_Use_Track | Validates track availability from path_queue direction vs facing |
| 42 | +0xA8 | 0x4b4c50 | Stub (no-op) | — |
| 43 | +0xAC | 0x4b4c90 | Stub (return 0) | — |
| 44 | +0xB0 | 0x4b4ca0 | Stub (no-op) | — |

---

## Verification Status — Assembly-Level Cross-Check Complete

Every major claim in this document and the 11 satellite reports has been verified against
the gamemd.exe binary via Ghidra MCP decompilation and raw memory reads. 18 research agents
and 6 verification agents were used across the session.

### Corrections Applied During Verification

| # | Original Claim | Corrected To | Source |
|---|----------------|-------------|--------|
| 1 | +0x1C-0x2C = turn timer | **Slope timer** (3-frame SLERP blend) | Process analysis |
| 2 | +0x74 = facing_lock | **cloak_state** (prevents cloak spam in sub-cell) | Cell occupation |
| 3 | Wake anim: SpeedType==2 | **LandType==2** (Water) | Process analysis |
| 4 | Ship = byte-for-byte clone | **6 verified differences** | Ship comparison |
| 5 | Track 5 = 45° diagonal | **90° diagonal** (NE→SE, facing 32→96) | Binary read |
| 6 | Track 6 = 90° diagonal | **45° diagonal** (NE→E, facing 32→64) | Binary read |
| 7 | +0x5B4 = ZoneSpeedCategory | **MovementZone** enum directly (0-12) | Zone verify |
| 8 | Budget = speed + residual | **Zeroes speed when param_2==true** (retry) | Assembly trace |
| 9 | Manhattan reclaim = 1/7 | **ftol((1.0 - manhattan/11) * 7.0)** | Assembly trace |
| 10 | Code 2 always recurses | **First-cell re-pathfinds; second-cell recurses** | Assembly trace |
| 11 | hasFriendlyMoving: AND | **OR** (any condition triggers "moving") | Assembly trace |
| 12 | Infantry: CellSpread+0x2A5 | **WeaponType→Projectile+0xA0→AG+0x2A5** | Assembly trace |
| 13 | Sin_lookup = sin | **Sin_lookup = cos** (phase +2048/8192) | Memory + RotateZ |
| 14 | BlockedDelay INI key | **"BlockagePathDelay"** at Rules+0x1768 | Binary string |

### Rust Implementation Bugs Found

1. **`MovementZone::Amphibious` MISSING** from `src/rules/locomotor_type.rs:218` —
   binary index 5 with passability row [1,2,2,1,1,2,2,3] (water+beach, no rough).
   Units with `MovementZone=Amphibious` in rules.ini (at least 4 units) will fall through
   to `Normal`, giving wrong pathfinding (ground-only instead of amphibious).

   Also missing from `src/sim/zone_map.rs:52` `ZoneCategory::from_movement_zone` —
   needs to map to `ZoneCategory::Amphibious`.

2. **MovementZone variant order differs from binary** — AmphibiousCrusher/AmphibiousDestroyer
   swapped vs binary (3/4 in binary, 3/4 in Rust but reversed). Not a bug since Rust uses
   name-based matching, but noteworthy for documentation.

3. **SpeedType variant order differs from binary** — Hover=3/Winged=4/Float=5 in binary vs
   Float=3/Amphibious=4/Winged=5 in Rust. Also not a bug (name-based), but the speed table
   INI parsing uses name matching so it works correctly.

## Correction Log

**2026-04-06 — Fixed slope_timer field names and ILocomotion offset note:**
- **+0x4C/+0x50 layout was INCORRECTLY changed, then REVERTED.** Force_Track appears
  to write a double at +0x4C, but this is because Force_Track receives its `this`
  pointer as the **ILocomotion interface** (object_base + 4), NOT the object base.
  So Force_Track's `param_1 + 0x4C` = absolute +0x50 (speed low dword), and
  `param_1 + 0x50` = absolute +0x54 (speed high dword). The original layout is correct:
  +0x4C = int residual_ticks, +0x50 = double current_speed (8 bytes), +0x58 = track_index.
  **Verified from constructor at 0x4af540:** ESI = object_base, +0x58 init to -1 (track_index),
  +0x5C init to -1 (point_index), +0x4C/+0x50/+0x54 all init to 0.
- **CRITICAL NOTE:** Functions called through the ILocomotion vtable (+0x04) receive
  `this = object_base + 4`. All offsets in their decompilation are 4 bytes LESS than
  the absolute offset from object_base. E.g., `[param_1 + 0x54]` in Force_Track =
  `[object_base + 0x58]` = track_index. Always verify the calling convention before
  extracting field offsets.
- **+0x20 through +0x2C renamed from turn_timer_* to slope_timer_*.** These fields control
  a 3-frame slope transition blend (not turn/facing interpolation), as confirmed by the
  DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md Phase 1 analysis showing CellClass+0x11C (SlopeIndex)
  detection driving these timers.
