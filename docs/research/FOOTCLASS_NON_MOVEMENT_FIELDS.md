# FootClass Non-Movement Fields & Methods

## Source
Ghidra decompilation of gamemd.exe. All offsets are byte offsets from object base.
Confidence: HIGH for fields confirmed by multiple function cross-references.
MEDIUM for fields seen in constructor init only.

## Class Hierarchy Context

```
AbstractClass -> ObjectClass -> MissionClass -> RadioClass -> TechnoClass -> FootClass
```

- **TechnoClass** ends at approximately byte offset **0x520** (field 0x147 * 4 + 4).
- **FootClass** fields start at byte offset **0x520**.

---

## TechnoClass Fields Referenced (for context)

These are NOT FootClass fields but are accessed by FootClass methods:

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x0081 | byte | 0 | IsInLimbo | Checked extensively in AI, team code |
| 0x0082 | byte | 0 | Unknown_IsParalyzed | Blocks destination set in Set_Destination_Internal |
| 0x0083 | byte | 0 | Unknown_IsDeselected | Checked in ControlGroup__Recall |
| 0x008D | byte | 0 | IsFalling | Checked in AI for movement blocking |
| 0x008E | byte | 0 | PreviousFallingState | Compared to 0x8D in AI for sound |
| 0x008F | byte | 1 | AllowFallSound | Used with falling state for sound playback |
| 0x00B0 | int  | -  | SuspendedMission | Set to -1 when removed from team (offset 0x2C * 4) |
| 0x0118 | int  | 0 | Cargo_FirstPassenger (CargoClass) | EMPPassengers iterates from here |
| 0x0214 | int  | -1 | Group | Ctrl+1..9 group number (0-based, -1=none) |
| 0x021C | ptr  | param | Owner (HouseClass*) | Constructor param_2 |
| 0x0274 | ptr  | 0 | TemporalClass* | Chrono erase weapon manager (Init_Managers: param_1[0x9D]) |
| 0x0294 | ptr  | 0 | AirstrikeClass* | Airstrike manager (Init_Managers: param_1[0xA5]) |
| 0x02B8 | int  | 0 | Unknown_TeamRelated | Cleared on team removal |
| 0x02BC | ptr  | 0 | CaptureManagerClass* | Mind control manager (Init_Managers: param_1[0xAF]) |
| 0x02D0 | ptr  | 0 | SpawnManagerClass* | Spawn manager for carrier aircraft (Init_Managers: param_1[0xB4]) |
| 0x02D8 | ptr  | 0 | SlaveManagerClass* | Slave miner manager (Init_Managers: param_1[0xB6]) |
| 0x0500 | ptr  | 0 | TransportTarget | TryEnterTransport reads this |

---

## FootClass Field Map (byte offsets from object base)

### Core State (0x520 - 0x53F)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x520  | int  | -1 (0xFFFFFFFF) | Unknown_0x520 | Constructor param_1[0x148] |
| 0x524  | u16  | 0 | Unknown_0x524 | Constructor |
| 0x526  | u16  | 0 | Unknown_0x526 | Constructor |
| 0x528  | u16  | 0 | Unknown_0x528 | Constructor |
| 0x52A  | u16  | 0 | Unknown_0x52A | Constructor |
| 0x530  | double | (varies) | SlopeSpeedFactor | Get_Slope_Speed_Factor returns *(double*)(this+0x530). Team override returns 1.0 |
| 0x538  | int  | 0 | Unknown_0x538 | Constructor param_1[0x14E] = movement counter in AI |
| 0x53C  | bool | 0 | IsMovementSoundPlaying | Constructor param_1[0x14F], toggled in AI for move sounds |
| 0x540  | int  | 0 | MoveSoundCountdown | param_1[0x150], countdown for move sound shutoff |

### Path Start / Cell Tracking (0x554 - 0x56F)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x558  | u16  | 0 | PathStartCell_X | Find_Path sets param_1[0x156] = cell.X on success |
| 0x55A  | u16  | 0 | PathStartCell_Y | Along with X |
| 0x55C  | u16  | 0 | Unknown_Cell_0x55C | |
| 0x55E  | u16  | 0 | Unknown_Cell_0x55E | |
| 0x560  | u16  | 0 | Unknown_Cell_0x560 | |
| 0x562  | u16  | 0 | Unknown_Cell_0x562 | |
| 0x564  | CellStruct (4 bytes) | NullCell | LastOccupiedCell | Destructor checks if map cell's occupier matches this |
| 0x568  | int  | 0 | Unknown_0x568 | |
| 0x56C  | int  | 0 | Unknown_0x56C | |

### DynamicVectorClass #1 - Path Steps (0x588 - 0x59F)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x588  | ptr  | &PTR_FUN_007e91ec | PathSteps_VTable | DynamicVectorClass vtable |
| 0x58C  | ptr  | (alloc) | PathSteps_Buffer | Data pointer |
| 0x590  | int  | (varies) | PathSteps_Capacity | |
| 0x594  | byte | 1 | PathSteps_IsAllocated | Byte at offset 0xC from DynVec base; FUN_004e0e80 sets *(param_1+3) byte = 1 (corrected 2026-05-29: was listed as int Count at 0x594; binary shows IsAllocated byte here via decompile_function 0x4E0E80 — OFFSET_RETYPED_WRONG) |
| 0x595  | byte | 0 | PathSteps_IsAllocated2 | Second allocation flag at DynVec+0xD; FUN_004e0e80 sets byte at (int)param_1+0xD = 0 (corrected 2026-05-29: was listed as IsAllocated at 0x595; primary IsAllocated is at 0x594 — OFFSET_RETYPED_WRONG) |
| 0x598  | int  | 0 | PathSteps_Count | Constructor: param_1[0x166] = 0; position confirmed by DynVec layout from FUN_004e0e80 (corrected 2026-05-29: field was missing; doc had Count at 0x594 which is actually IsAllocated — OFFSET_RETYPED_WRONG) |
| 0x59C  | int  | 10 | PathSteps_GrowthStep | Constructor: param_1[0x167] = 10; confirmed via decompile_function 0x4D31E0 |

### NavCom / Destination (0x5A0 - 0x5AF)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x5A0  | int  | 0 | Unknown_0x5A0 | Stop_Moving clears both 0x5A0 and 0x5A4 |
| 0x5A4  | int (target) | 0 | NavCom (destination target) | Set_Destination_Internal writes param_1[0x169]. Plate comment confirms "FootClass+0x5A4" |
| 0x5A8  | int (target) | 0 | SuspendedNavCom | Set_NavCom_With_Suspend saves NavCom here before overwriting. Cleared on team removal |

### DynamicVectorClass #2 - NavQueue / Waypoint Queue (0x5AC - 0x5C3)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x5AC  | ptr  | &PTR_FUN_007e91ec | NavQueue_VTable | DynamicVectorClass vtable |
| 0x5B0  | ptr  | (alloc) | NavQueue_Buffer | Data pointer |
| 0x5B4  | int  | (varies) | NavQueue_Capacity | Enter_Destination: param_1[0x16D] used in capacity check |
| 0x5B8  | byte | 1 | NavQueue_IsAllocated | Byte at DynVec+0xC; same pattern as PathSteps (corrected 2026-05-29: was listed as int Count; Enter_Destination reads Count at param_1[0x16F]=0x5BC, not here — OFFSET_RETYPED_WRONG) |
| 0x5B9  | byte | 0 | NavQueue_IsAllocated2 | Enter_Destination checks `*(char*)((int)param_1 + 0x5b9)` for resize gate (corrected 2026-05-29: was listed as IsAllocated; primary IsAllocated is at 0x5B8 — OFFSET_RETYPED_WRONG) |
| 0x5BC  | int  | 0 | NavQueue_Count | Enter_Destination reads/writes param_1[0x16F]=0x5BC; increments as `param_1[0x16f] = iVar2 + 1` (corrected 2026-05-29: was listed as GrowthStep=10; GrowthStep is at 0x5C0 — OFFSET_RETYPED_WRONG via decompile_function 0x4DA0E0) |
| 0x5C0  | int  | 10 | NavQueue_GrowthStep | Enter_Destination: param_1[0x170]=0x5C0 used in resize call (corrected 2026-05-29: field was missing from table; was incorrectly placed at 0x5BC — OFFSET_RETYPED_WRONG via decompile_function 0x4DA0E0) |

This is the shift-click waypoint queue. `Enter_Destination` appends targets to this vector.
When NavCom becomes 0, the next entry from NavQueue is popped and set as destination.
`param_1[0x16D]` at 0x5B4 is the capacity check used in Enter_Destination.

### Command/Assignment Fields (0x5C0 - 0x5D7)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x5C0  | int  | 0 | Unknown_0x5C0 | |
| 0x5C4  | int  | -1 (0xFFFFFFFF) | AssignedTarget_Type | Clear_All_TarCom sets to -1. Assign_Target_Command writes 0x1D here (param_1[0x171]) |
| 0x5C8  | int  | 0 | AssignedTarget_Primary | Assign_Target_Command writes when param_4!=0 (param_1[0x172]) |
| 0x5CC  | int  | 0 | AssignedTarget_Secondary | Assign_Target_Command writes when bVar4 (param_1[0x173]) |
| 0x5D1  | byte | 0 | ClearAssignment_Flag | Clear_All_TarCom clears; Assign_Target_Command clears |
| 0x5D4  | ptr (TeamClass*) | 0 | Team | Confirmed by: Get_Slope_Speed_Factor, Evaluate_Target_Threat, team add/remove (FUN_006ea500/FUN_006ea870), Find_Path |
| 0x5D8  | ptr (FootClass*) | 0 | NextTeamMember | Linked list pointer for team iteration. TeamClass__Recruit_Or_Add iterates via [0x176] |

### Path / A* Data (0x5E0 - 0x63F)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x5E0  | int  | -1 | PathStepIndex | Find_Path sets to -1 on failure. Checked extensively |
| 0x5E4+ | byte[24] | (path data) | PathSteps_Inline | Find_Path copies up to 24 path step bytes starting at 0x5E0 + offset |

The path steps are stored inline starting at 0x5E0. The data comes from Run_AStar result.

### CDTimer Fields / Frame Counters (0x640 - 0x66F)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x640  | CDTimerClass | g_CurrentFrameCounter | PathDelay_Timer | Checked in Set_Destination_Internal for WalkLocomotion. param_1[400] = 0x640 |
| 0x644  | int  | (frame hi) | PathDelay_Timer_FrameHi | |
| 0x648  | int  | 0 | PathDelay_Timer_Duration | Set to 0/1 in Set_Destination_Internal. Used as "path rethink delay" |
| 0x64C  | int  | 0 | Unknown_Timer_0x64C | |
| 0x650  | CDTimerClass | g_CurrentFrameCounter | VisionUpdate_Timer | param_1[0x194], used in AI for shroud/vision updates |
| 0x658  | int  | 0 | VisionUpdate_Duration | |
| 0x65C  | CDTimerClass | g_CurrentFrameCounter | MovementSound_Timer | param_1[0x197] |
| 0x664  | int  | 0 | MovementSound_Duration | |
| 0x668  | CDTimerClass | g_CurrentFrameCounter | BlockagePath_Timer | param_1[0x19A], set from RulesClass+0x1768 (BlockagePathDelay) |
| 0x670  | int  | 0 | BlockagePath_Duration | |

### Locomotor / ILocomotion (0x674 - 0x67F)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x674  | ptr (ILocomotion*) | 0 | ActiveLocomotor | Head_To_Coord_Dispatch, GetVisualState, IsCloakable, Set_Destination_Internal all use this. Locomotion_AI calls Process() on it |

This is the COM interface pointer to the active locomotor (DriveLocomotionClass, WalkLocomotionClass, etc.).
IPiggyback swapping in FootClass::AI replaces this pointer.

### Flags and State (0x684 - 0x6BF)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x684  | byte | 0xFF | Unknown_0x684 | param_1[0x1A1] init to 0xFF |
| 0x685  | byte | 0 | Unknown_0x685 | |
| 0x686  | byte | 0 | Unknown_0x686 | |
| 0x687  | byte | 0 | Unknown_0x687 | Save_Convoy_State serializes this |
| 0x688  | byte | 0 | Unknown_0x688 | |
| 0x689  | byte | 0 | IsTeamLeader | Set by team add (FUN_006ea500): true if unit is first in team. Checked in team remove |
| 0x68A  | byte | 0 | Unknown_0x68A | |
| 0x68B  | byte | 0 | Unknown_0x68B | |
| 0x68C  | byte | 0 | Unknown_0x68C | |
| 0x68D  | byte | 0 | Unknown_0x68D | |
| 0x68E  | byte | 0 | Unknown_0x68E | |
| 0x68F  | byte | 0 | Unknown_0x68F | |
| 0x690  | byte | 0 | IsDocking | Find_Nearest_Dock sets param_1[0x1A4]=1 when dock found, 0 otherwise |
| 0x691  | byte | 0 | Unknown_0x691 | |
| 0x694  | int  | 0 | Unknown_Ptr_0x694 | param_1[0x1A5], checked in AI - calls into +0x69C object |
| 0x698  | int  | 0 | Unknown_0x698 | |
| 0x69C  | ptr (ParasiteClass*) | 0 | Parasite | Init_Managers creates ParasiteClass here (param_1[0x1A7]). Destructor calls Release(1). Only created for units with Parasite=yes on their weapon's warhead. ReceiveDamage reads victim's +0x69C for parasite damage relay |
| 0x6A0  | CDTimerClass | g_CurrentFrameCounter | Unknown_Timer_0x6A0 | |
| 0x6A8  | int  | 0 | Unknown_0x6A8 | |
| 0x6AC  | byte | 0 | IsUnderground | Checked extensively in Set_Destination_Internal (blocks new dest), ReceiveEMP (blocks health=0) |
| 0x6AD  | byte | 0 | IsTunneling | Set_Destination_Internal: if tunneling and dest cleared, clears tunnel link. Also checked in AI |
| 0x6AE  | byte | 0 | TunnelComplete | Set to 1 when tunneling completed (dest cleared while underground) |
| 0x6AF  | byte | 0 | Unknown_ScatterFlag | Receive_Radio case 0x17: checked before scatter on radio-released |
| 0x6B0  | byte | 0 | Unknown_0x6B0 | |
| 0x6B1  | byte | 0 | NavQueue_IsSelfTarget | Enter_Destination: set to 1 if entering self-destination with existing NavQueue |
| 0x6B2  | byte | 0 | Unknown_0x6B2 | |
| 0x6B3  | byte | 0 | WasMovingThisTick | AI clears at start of tick, other code sets it |
| 0x6B4  | byte | 0 | Unknown_0x6B4 | |
| 0x6B5  | byte | 0 | Unknown_0x6B5 | |
| 0x6B6  | byte | 1 | **blocked_delay** | Blocked-delay timer state (verified from UNIT_COLLISION_AND_REPATH_TRIGGERS). Constructor inits to 1 |
| 0x6B7  | byte | 0 | **path_blocked** | 0=clear, 1=blocked. Set on code-2 blocking. Infantry clear on arrival; vehicles never clear. Set_Destination_Internal clears at end |
| 0x6B8  | byte | 0 | IsLeavingTeam | FUN_006ea870 sets param_2[0x1AE]=1 at start of team removal |

### Convoy / Following / Formation (0x6C4 - 0x6D2)

| Offset | Type | Init | Field Name | Evidence |
|--------|------|------|------------|----------|
| 0x6C4  | ptr (FootClass*) | 0 | ConvoyLeader / FollowingUnit_Primary | Clear_Convoy_On_Delete checks, Save_Convoy_State reads. Find_Path iterates [0x1B2] chain. TeamClass script 0x2C/0x2D writes [0x1B1] as unit type ptr |
| 0x6C8  | ptr (FootClass*) | 0 | ConvoyTarget / FollowingUnit_Secondary | Clear_Convoy_On_Delete checks, Save_Convoy_State reads |
| 0x6CC  | int  | 0 | ConvoyState | Save_Convoy_State serializes. GetCurrentSpeed: if UnitClass and [0x1B3]!=-1, speed /= 2 |
| 0x6D0  | byte | 0 | ConvoyFlag_0x6D0 | Save_Convoy_State serializes (param_1+0x6D0) |
| 0x6D1  | byte | 0 | ConvoyFlag_0x6D1 | Save_Convoy_State serializes |
| 0x6D2  | byte | 0 | ConvoyFlag_0x6D2 | Save_Convoy_State serializes |

Note: Offset 0x6C4 (param_1[0x1B1]) is overloaded. In Find_Path, it's used as a "following unit" pointer
for formation movement. In team scripts 0x2C/0x2D (TRUCKB/TRUCKA disguise), it's written as a UnitTypeClass*.
The GetCurrentSpeed function treats [0x1B3] (0x6CC) as -1 meaning "no convoy penalty".

---

## Summary of Key Fields by Category

### Passenger / Transport
- **0x0118** (TechnoClass): CargoClass first passenger pointer (linked list head)
- **0x0500** (TechnoClass): Transport target pointer - what to enter
- **TryEnterTransport** (0x0070d7e0): Reads [0x140]=0x500 for transport target, calls radio 0x02 and 0x0F, then enters

### Team Membership
- **0x5D4**: TeamClass* pointer (set by team add, cleared on remove)
- **0x5D8**: Next team member linked list pointer
- **0x689**: IsTeamLeader byte flag
- **0x6B8**: IsLeavingTeam flag

### Control Group (Ctrl+1..9)
- **0x0214** (TechnoClass): Group number (int, 0-based, -1 = no group)
- Set by team system (FUN_006ea500) and by ControlGroup__Recall (0x007311c0)
- ControlGroup__Recall iterates all TechnoClass objects checking [0x85]=0x214

### NavQueue (Shift-click Waypoints)
- **0x5AC-0x5C3**: DynamicVectorClass of target pointers
- **0x5BC** (Count): Number of queued waypoints (corrected 2026-05-29: was 0x5B8; Enter_Destination uses param_1[0x16F]=0x5BC — OFFSET_RETYPED_WRONG)
- **0x5C0** (GrowthStep): Growth increment = 10 (corrected 2026-05-29: was missing; was incorrectly placed at 0x5BC — OFFSET_RETYPED_WRONG)
- **Enter_Destination** appends; when NavCom is cleared, next entry is popped

### Following / Formation / Convoy
- **0x6C4**: Primary following/convoy unit pointer
- **0x6C8**: Secondary convoy target
- **0x6CC**: Convoy state (affects speed when != -1 for UnitClass)
- **0x6D0-0x6D2**: Convoy flags (serialized)

### Locomotor
- **0x674**: ILocomotion* COM pointer (active locomotor)
- IPiggyback swap happens in FootClass::AI each tick

### Special Movement Modes
- **0x6AC**: IsUnderground (byte) - blocks destination setting, damage
- **0x6AD**: IsTunneling (byte) - tunnel movement in progress
- **0x6AE**: TunnelComplete (byte) - tunnel animation finished

### Docking
- **0x690**: IsDocking flag (set by Find_Nearest_Dock)

### Path Delay Timers
- **0x640**: PathDelay CDTimer (rethink delay after Set_Destination)
- **0x668**: BlockagePathDelay CDTimer (from Rules BlockagePathDelay= value at RulesClass+0x1768)

---

## SpawnManagerClass (separate object, not embedded in FootClass)

Pointed to from TechnoClass. Fields (param_1 is int*, multiply by 4):

| Index | Offset | Field |
|-------|--------|-------|
| 0x09  | 0x24   | Owner TechnoClass* |
| 0x0A  | 0x28   | SpawnType (AircraftTypeClass*) |
| 0x0B  | 0x2C   | MaxSpawns (int) |
| 0x0C  | 0x30   | RegenRate |
| 0x0D  | 0x34   | ReloadRate |
| 0x0E-0x13 | 0x38-0x4C | DynamicVectorClass of SpawnNode* |
| 0x14-0x16 | 0x50-0x58 | CDTimer (spawn regen) |
| 0x17-0x19 | 0x5C-0x64 | CDTimer (reload) |
| 0x1A  | 0x68   | Target |
| 0x1B  | 0x6C   | Unknown |
| 0x1C  | 0x70   | Unknown |

SpawnNode is a 0x18-byte struct:
- +0x00: FootClass* (spawned unit pointer)
- +0x04: int Status
- +0x08-0x0C: CDTimer
- +0x10: int Unknown
- +0x14: int IsSpiderMine (set for Desolator/Chaos Drone special spawns)

---

## ParasiteClass (separate object, at FootClass+0x69C)

Small standalone class inheriting from AbstractClass. Size = 0x58 bytes.
Created by TechnoClass__Init_Managers for units whose weapon warhead has Parasite=yes.
Stored on **FootClass** (not TechnoClass) at offset 0x69C.

| Index | Offset | Field |
|-------|--------|-------|
| 0x0B  | 0x2C   | CDTimer (damage tick) |
| 0x0D  | 0x34   | Unknown |
| 0x0E  | 0x38   | CDTimer |
| 0x10  | 0x40   | Unknown |

---

## Manager Pointer Summary (on TechnoClass)

All created by `TechnoClass__Init_Managers` (0x006f3f40):

| Offset | Type | Condition | Description |
|--------|------|-----------|-------------|
| 0x0274 | TemporalClass* | Weapon warhead has Temporal=yes | Chrono erase / temporal weapon |
| 0x0294 | AirstrikeClass* | TypeClass+0x61C > 0 | Airstrike (Boris-style laser designator) |
| 0x02BC | CaptureManagerClass* | Weapon has MindControl ability | Mind control manager |
| 0x02D0 | SpawnManagerClass* | TypeClass+0xD58 != 0 (Spawns=) | Carrier/Dreadnought spawn system |
| 0x02D8 | SlaveManagerClass* | TypeClass+0xD40 != 0 (Slave=) | Slave miner system |
| 0x069C | ParasiteClass* | Weapon warhead Parasite=yes (FootClass only!) | Terror Drone parasite attachment |

---

## ControlGroup System

`ControlGroup__Recall` at 0x007311c0:
- Iterates all objects in g_TechnoClass_Array
- Checks `obj[0x85]` (offset 0x214) == group_number - 1 (groups are 1-based in input, 0-based stored)
- Double-tap within 800ms centers camera on group
- Uses timeGetTime() for double-tap detection (global vars at 0x00845550, 0x00845554)
