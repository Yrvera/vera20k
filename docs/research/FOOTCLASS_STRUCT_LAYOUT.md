# FootClass Full Struct Layout — Ghidra Research Report

**Constructor (full):** `0x4D31E0`
**Constructor (load):** `0x4D3540`
**Primary vtable:** `0x7E8C94`
**Confidence:** HIGH (verified from constructor + 20 decompiled methods)
**Active in YR:** Yes — base class for all mobile units
**Date:** 2026-04-06

## 1. Class Boundaries (Verified)

| Class | Start | End | Size |
|-------|-------|-----|------|
| TechnoClass (parent) | 0x000 | 0x51F | 0x520 (1312 bytes) |
| FootClass-specific | 0x520 | 0x6BF | 0x1A0 (416 bytes) |
| Subclass start | 0x6C0 | — | — |

**Total through FootClass end: 0x6C0 = 1728 bytes.**

Verified by all three subclass constructors:
- InfantryClass (0x517A50): first field at `param_1[0x1B0]` = byte 0x6C0
- UnitClass (0x7353C0): first field at `param_1[0x1B0]` = byte 0x6C0
- AircraftClass (0x413D20): first field at `param_1[0x1B0]` = byte 0x6C0

## 2. Constructor Analysis

The full constructor at `0x4D31E0` uses `undefined4 *param_1` (int pointer), so ALL
indexed accesses `param_1[N]` are byte offset `N * 4`. Direct casts like
`*(type *)((int)param_1 + 0xNNN)` are literal byte offsets.

The load constructor at `0x4D3540` is minimal — it only sets the DynamicVectorClass
vtables (0x588, 0x5AC) and the ILocomotion pointer (0x674), plus the 4 vtable slots.

## 3. Complete Field Map (0x520–0x6BF)

### 3.1 First Field Block (0x520–0x53F)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x520 | 4 | int | 0xFFFFFFFF | **UnknownID_520** | Ctor [0x148] = -1 | MED |
| 0x524 | 2 | short | 0 | **Unknown_524** | Ctor (word at [0x149]) | LOW |
| 0x526 | 2 | short | 0 | **Unknown_526** | Ctor (byte at (int)+0x526) | LOW |
| 0x528 | 2 | short | 0 | **Unknown_528** | Ctor (word at [0x14A]) | LOW |
| 0x52A | 2 | short | 0 | **Unknown_52A** | Ctor (byte at (int)+0x52A) | LOW |
| 0x52C | 4 | — | — | **(gap/padding)** | Not initialized in ctor | LOW |

### 3.2 Slope Speed Factor (0x530–0x53F)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x530 | 8 | **double** | 0 | **SlopeSpeedFactor** | Ctor [0x14C,0x14D] = 0,0; Get_Slope_Speed_Factor reads `*(double*)(this+0x530)` and returns it. If in a team with TeamTypeClass+0xF2 flag, returns 1.0 instead. Used by pathfinding (Zone_precheck, Path_smooth, Path_Reroute) | **HIGH** |
| 0x538 | 4 | int | 0 | **MovementTickCounter** | Ctor [0x14E] = 0; AI increments when ILocomotion::Process is active; checked against previous value to detect actual movement | HIGH |
| 0x53C | 1 | bool | 0 | **RankUpAnimActive** | Ctor byte at [0x14F]; AI: set to 1 when rank-up anim/sound plays | HIGH |
| 0x53D–0x53F | 3 | — | — | **(padding)** | | |
| 0x540 | 4 | int | 0 | **RankUpAnimCountdown** | Ctor [0x150] = 0; countdown timer (ticks remaining) for rank-up visual effect | HIGH |

### 3.3 Looping Sound Handle (0x544–0x553) + Padding (0x554–0x557)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x544 | 4 | ptr | 0 | **LoopingSoundHandle.Ptr** | SafePointerHandle[0]; ctor via FUN_00405be0 at 0x4d3482; used in AI for vet anim, deploy/crash/falling sounds, looping sound update | HIGH |
| 0x548 | 4 | int | 0 | **LoopingSoundHandle.Stamp1** | SafePointerHandle[1]; validated against object+0x138 | HIGH |
| 0x54C | 4 | int | 0 | **LoopingSoundHandle.Stamp2** | SafePointerHandle[2]; validated against object+0x24 | HIGH |
| 0x550 | 4 | ptr | &0x87e294 | **LoopingSoundHandle.TypeTag** | SafePointerHandle[3]; global type sentinel | HIGH |
| 0x554 | 4 | — | — | **(padding)** | Never accessed as FootClass field; exhaustive byte-pattern search confirmed | HIGH |

### 3.4 HeadTo Coordinate Cache (0x558–0x567)

Four pairs of 16-bit coordinate components, possibly used by the locomotor for sub-cell
movement interpolation.

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x558 | 2 | short | 0 | **HeadToCoord_0_X** | Ctor word at [0x156] | MED |
| 0x55A | 2 | short | 0 | **HeadToCoord_0_Y** | Ctor byte at (int)+0x55A | MED |
| 0x55C | 2 | short | 0 | **HeadToCoord_1_X** | Ctor word at [0x157] | MED |
| 0x55E | 2 | short | 0 | **HeadToCoord_1_Y** | Ctor byte at (int)+0x55E | MED |
| 0x560 | 2 | short | 0 | **HeadToCoord_2_X** | Ctor word at [0x158] | MED |
| 0x562 | 2 | short | 0 | **HeadToCoord_2_Y** | Ctor byte at (int)+0x562 | MED |
| 0x564 | 2 | short | 0 | **HeadToCoord_3_X** | Ctor word at [0x159] | MED |
| 0x566 | 2 | short | 0 | **HeadToCoord_3_Y** | Ctor byte at (int)+0x566 | MED |

### 3.5 Navigation Auxiliary Fields (0x568–0x577)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x568 | 4 | int | 0 | **Unknown_568** | Ctor [0x15A] | LOW |
| 0x56C | 4 | int | 0 | **Unknown_56C** | Ctor [0x15B] | LOW |
| 0x570 | 4 | int | 0 | **Unknown_570** | Ctor [0x15C] | LOW |
| 0x574 | 4 | — | — | **(gap/padding)** | Not initialized in ctor (between [0x15C] and [0x15E]) | LOW |

### 3.6 Formation Speed & Speed Multiplier (0x578–0x587)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x578 | 8 | **double** | 0.0 | **FormationSpeed** | Ctor [0x15E,0x15F] = 0,0; Locomotion_AI reads `*(double*)(param_1+0x15E)` to select walk (0x17) vs run (0x18) infantry anim based on threshold comparison; Convoy report confirms formation speed propagation | **HIGH** |
| 0x580 | 8 | **double** | 1.0 | **SpeedMultiplier** | Ctor [0x160] = 0, [0x161] = 0x3FF00000 (IEEE754 double 1.0); separate from FormationSpeed; used as a scale factor for movement speed | **HIGH** |

### 3.7 Waypoint Queue — DynamicVectorClass<AbstractClass*> (0x588–0x59F)

Shift-click waypoint queue. Stores queued movement destinations.

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x588 | 4 | ptr | &DVec_vtable | **WaypointQueue.vtable** | Ctor [0x162] | HIGH |
| 0x58C | 4 | ptr | — | **WaypointQueue.Data** | [0x163] — heap array pointer | HIGH |
| 0x590 | 4 | int | — | **WaypointQueue.MaxSize** | [0x164] | MED |
| 0x594 | 4 | int | — | **WaypointQueue.field_0C** | [0x165] | LOW |
| 0x598 | 4 | int | 0 | **WaypointQueue.Count** | Ctor [0x166] = 0; Mission_Hunt checks `> 0` for last waypoint | HIGH |
| 0x59C | 4 | int | 10 | **WaypointQueue.Capacity** | Ctor [0x167] = 10 | HIGH |

### 3.8 NavCom System (0x5A0–0x5AB)

The NavCom (Navigation Computer) is the primary destination system.

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x5A0 | 4 | ptr | 0 | **NavCom_Aux** | Ctor [0x168]; Set_Dest_Internal clears to 0; Stop_Moving clears | HIGH |
| 0x5A4 | 4 | ptr | 0 | **NavCom** | Ctor [0x169]; THE primary navigation target. When non-zero, unit is heading somewhere. Cleared by Stop_Moving. Read by Receive_Radio(0x13). Set by Set_Destination_Internal. | **HIGH** |
| 0x5A8 | 4 | ptr | 0 | **SuspendedNavCom** | Ctor [0x16A]; Set_NavCom_With_Suspend copies NavCom here. Remove_Member clears. | HIGH |

**NavCom lifecycle:**
1. `Set_Destination_Internal` (0x4D94B0): Sets NavCom (0x5A4), calls ILocomotion::Head_To_Coord
2. `Set_NavCom_With_Suspend` (0x4D8F40): NavCom → SuspendedNavCom, then sets new NavCom
3. `Stop_Moving` (0x4DF0D0): Zeroes NavCom_Aux (0x5A0) and NavCom (0x5A4)
4. Guards in Set_Dest: IsDeploying (0x6AD), InLimbo (0x82), WarheadBusy (0x2E4) all block

### 3.9 Enter Queue — DynamicVectorClass<AbstractClass*> (0x5AC–0x5C3)

Queue of objects this unit wants to enter (buildings, transports).

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x5AC | 4 | ptr | &DVec_vtable | **EnterQueue.vtable** | Ctor [0x16B] | HIGH |
| 0x5B0 | 4 | ptr | — | **EnterQueue.Data** | [0x16C] — heap array | HIGH |
| 0x5B4 | 4 | int | — | **EnterQueue.MaxSize** | [0x16D] — capacity check in Enter_Destination | MED |
| 0x5B8 | 4 | — | — | **EnterQueue.field_0C** | byte at 0x5B9 = IsAllocated flag | LOW |
| 0x5BC | 4 | int | 0 | **EnterQueue.Count** | Ctor [0x16F] = 0; Enter_Destination reads/writes | HIGH |
| 0x5C0 | 4 | int | 10 | **EnterQueue.Capacity** | Ctor [0x170] = 10 | HIGH |

### 3.10 TarCom (Target Computer) System (0x5C4–0x5D3)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x5C4 | 4 | int | 0xFFFFFFFF | **TarCom_CommandType** | Ctor [0x171] = -1; Assign_Target_Command sets to 0x1D (attack); Clear_All_TarCom sets to -1 | **HIGH** |
| 0x5C8 | 4 | ptr | 0 | **TarCom_PrimaryTarget** | Ctor [0x172]; set on forced-fire; gate in Arrival_Target_Handler — when non-zero, takes "has forced target" path | **HIGH** |
| 0x5CC | 4 | ptr | 0 | **TarCom_FollowTarget** | Ctor [0x173]; set on normal-attack; Arrival_Target_Handler checks against CanFire (vtable+0x3B4) | **HIGH** |
| 0x5D0 | 1 | — | — | **(padding)** | | |
| 0x5D1 | 1 | bool | 0 | **TarCom_IsFollowing** | Ctor direct; set to 1 on successful engagement in Arrival_Target_Handler; cleared by Assign_Target_Command and Clear_All_TarCom | **HIGH** |
| 0x5D2–0x5D3 | 2 | — | — | **(padding)** | | |

### 3.11 Team & Following (0x5D4–0x5DF)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x5D4 | 4 | ptr | 0 | **Team** (TeamClass*) | Ctor [0x175]; TeamClass::Add_Member (0x6EA500) writes `[0x175] = team_ptr`; Remove_Member (0x6EA870) clears to 0. In AI: gates tiberium self-heal and fog update. In Find_Path: gates path length override (team path limit vs RulesClass MaxPathSteps). In Evaluate_Target_Threat: reads TeamTypeClass+0xF6 for independent targeting flag. In Get_Slope_Speed_Factor: reads TeamTypeClass+0xF2 for slope-ignore flag. | **HIGH** |
| 0x5D8 | 4 | ptr | 0 | **TeamNextMember** | Ctor [0x176]; intrusive linked list pointer. Add_Member prepends: `[0x176] = old_first`. Remove_Member clears to 0. | **HIGH** |
| 0x5DC | 4 | int | 0 | **GhostCell** | Ctor [0x177]; temp storage for cell being navigated to; set during Hunt when finding nearby passable cell | MED |

### 3.12 Path Queue (0x5E0–0x63F)

Stores up to 24 direction steps from A* pathfinding.

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x5E0 | 4 | int | 0xFFFFFFFF | **PathHeadIndex** | Ctor [0x178] = -1; Run_AStar writes; Find_Path reads/writes. -1 = no valid path. | **HIGH** |
| 0x5E4 | 92 | int[23] | — | **PathSteps[23]** | Find_Path copies up to 24 entries from A* result. Each entry = 4 bytes (FacingType direction enum). Total path buffer = 96 bytes (0x5E0-0x63F). | **HIGH** |

**Find_Path logic:**
```
max_to_copy = min(24 - current_offset, astar_result.count)
copy to this + 0x178 + offset (dword-indexed)
```

### 3.13 Timer Block — 4 CDTimerClass + 1 non-standard (0x640–0x6AB)

Timers 1, 3, 4, 5 are standard CDTimerClass (12 bytes each): `StartFrame` (int, -1=inactive),
`MidValue` (int), `Duration` (int).
Remaining time = max(0, Duration - (g_CurrentFrameCounter - StartFrame)).

**Timer 2 (0x64C–0x65B) is NOT a CDTimerClass** — it is 16 bytes with a leading `Max` dword
before the Start/Value/Duration pattern. The ctor sets [0x193]=10 (Max), [0x194]=CurrentFrame
(Start), skips [0x195] (Value, uninitialised), and [0x196]=0 (Duration). The exact type is
unknown but it resembles a RateTimer or CountDownTimer variant with a capacity/max field prepended.
(corrected 2026-05-28: was labelled CDTimerClass; binary ctor shows 4 dwords including a Max=10
leading field not present in CDTimerClass — ROOT_CAUSE: INFERENCE_HARDENED; verified via
`decompile_function 0x4D31E0`)

| Byte Offset | Size | Field Name | Init | Evidence | Confidence |
|-------------|------|------------|------|----------|------------|
| **Timer 1: Path Delay (Walk/Infantry)** | | | | | |
| 0x640 | 4 | **PathDelayTimer.Start** | CurrentFrame | Ctor [0x190]; Set_Dest; Find_Path | HIGH |
| 0x644 | 4 | **PathDelayTimer.Value** | — | Set_Dest [0x191] | MED |
| 0x648 | 4 | **PathDelayTimer.Duration** | 0 | Ctor [0x192]; Set_Dest sets to 1 for WalkLocomotion; Find_Path resets to 0 | HIGH |
| **Timer 2: General Movement (non-standard, 16 bytes)** | | | | | |
| 0x64C | 4 | **MovementTimer.Max** | 10 | Ctor [0x193]; leading capacity/max field — not a CDTimerClass.Start | MED |
| 0x650 | 4 | **MovementTimer.Start** | CurrentFrame | Ctor [0x194] | MED |
| 0x654 | 4 | **MovementTimer.Value** | — | Not initialized in ctor (ctor skips [0x195]) | LOW |
| 0x658 | 4 | **MovementTimer.Duration** | 0 | Ctor [0x196] | MED |
| **Timer 3: Fog/Shroud Update** | | | | | |
| 0x65C | 4 | **FogUpdateTimer.Start** | CurrentFrame | Ctor [0x197]; AI: used to gate fog border updates | HIGH |
| 0x660 | 4 | **FogUpdateTimer.Value** | — | Not initialized in ctor | LOW |
| 0x664 | 4 | **FogUpdateTimer.Duration** | 0 | Ctor [0x199]; AI sets to 0xF (15 frames) after fog update | HIGH |
| **Timer 4: NavCom/Path Retry** | | | | | |
| 0x668 | 4 | **PathRetryTimer.Start** | CurrentFrame | Ctor [0x19A]; Set_Dest_Internal resets | HIGH |
| 0x66C | 4 | **PathRetryTimer.Value** | — | Set_Dest [0x19B] | MED |
| 0x670 | 4 | **PathRetryTimer.Duration** | 0 | Ctor [0x19C]; Set_Dest sets to RulesClass+0x1768 (PathDelay=) | HIGH |

### 3.14 ILocomotion COM Interface Pointer (0x674)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| **0x674** | **4** | **ILocomotion*** | **0** | **Locomotor** | Ctor [0x19D]; THE locomotor COM pointer. Created from TechnoTypeClass+0x34C CLSID in subclass ctors. AI calls ILocomotion::Process (vtable+0x40), queries IPiggyback for chrono swap, calls Is_Moving_Now (vtable+0x80) for cloak check. Set_Dest calls Head_To_Coord (vtable+0x44). Assert fires if null during movement. | **HIGH** |

### 3.15 Last Known Good Coordinates (0x678–0x683)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x678 | 4 | int | DAT_008b3da8 | **LastGoodCoord_X** | Ctor [0x19E]; likely {0,0,0} sentinel | MED |
| 0x67C | 4 | int | DAT_008b3dac | **LastGoodCoord_Y** | Ctor [0x19F] | MED |
| 0x680 | 4 | int | DAT_008b3db0 | **LastGoodCoord_Z** | Ctor [0x1A0] | MED |

### 3.16 Flags & Byte Fields (0x684–0x6BF)

| Byte Offset | Size | Type | Init Value | Field Name | Evidence | Confidence |
|-------------|------|------|------------|------------|----------|------------|
| 0x684 | 1 | byte | 0xFF | **DriveTrackIndex** | Ctor byte at [0x1A1]; 0xFF = no active drive track. Index into drive track table for curved movement. | HIGH |
| 0x685 | 1 | byte | 0 | **Unknown_685** | Ctor | LOW |
| 0x686 | 1 | byte | 0 | **Unknown_686** | Ctor | LOW |
| 0x687 | 1 | byte | 0 | **ConvoyFlag_Serialized** | Ctor; serialized with convoy data in Save_Convoy_State | MED |
| 0x688 | 1 | bool | 0 | **ConvoyDisbanded** | Ctor; Clear_Convoy sets to 1 for each member | MED |
| 0x689 | 1 | bool | 0 | **ConvoyArrived** | Ctor; Add_Member: first member auto-arrives. Team movement tracking. | HIGH |
| 0x68A | 1 | byte | 0 | **Unknown_68A** | Ctor | LOW |
| 0x68B | 1 | byte | 0 | **Unknown_68B** | Ctor | LOW |
| 0x68C | 1 | byte | 0 | **Unknown_68C** | Ctor [0x1A3] | LOW |
| 0x68D | 1 | bool | 0 | **HasReachedDock** | Mission_Harvest: checked for dock proximity; Locomotion_AI: blocks walk→run transition when set | HIGH |
| 0x68E | 1 | bool | 0 | **HasFoundAutoTarget** | Mission_Guard/Mission_Harvest: set to 1 when guard mission finds auto-target building to enter (repair depot) | HIGH |
| 0x68F | 1 | bool | 0 | **IsReceivingRepair** | Mission_Guard: if set, calls RepairAI (vtable+0x340) | HIGH |
| 0x690 | 1 | bool | 0 | **IsDockingToBuilding** | Ctor [0x1A4] byte; Find_Nearest_Dock sets to 1; Mission_Guard checks for docking behavior | HIGH |
| 0x691 | 1 | bool | 0 | **IsWeedingHarvester** | Mission_Guard: if set, calls WeedHarvestAI (vtable+0x34C); Mission_Harvest checks | HIGH |
| 0x692 | 2 | — | 0 | **(padding/unknown)** | | LOW |
| 0x694 | 4 | ptr | 0 | **LargeObjectPtr** | Ctor [0x1A5]; AI calls `*(*(P+0x69C))->AI()`. **NOT TeamClass*** (TeamClass is only 0xA0 bytes, but this accesses +0x69C within the pointed object). Not cleared by Remove_Member. Points to a large (>0x69C byte) object with an AbstractClass-derived sub-object at +0x69C. | **MED** |
| 0x698 | 4 | int | 0 | **Unknown_698** | Ctor [0x1A6] | LOW |
| 0x69C | 4 | int | 0 | **Unknown_69C** | Ctor [0x1A7] | LOW |
| **Timer 5: Team/Idle Timer** | | | | | |
| 0x6A0 | 4 | int | CurrentFrame | **IdleTimer.Start** | Ctor [0x1A8] | MED |
| 0x6A4 | 4 | int | — | **IdleTimer.Value** | Not initialized in ctor | LOW |
| 0x6A8 | 4 | int | 0 | **IdleTimer.Duration** | Ctor [0x1AA] | MED |
| 0x6AC | 1 | bool | 0 | **SkipHeadToCoord** | Ctor [0x1AB] byte; Set_Dest_Internal: if set, skips ILocomotion::Head_To_Coord and clears self (one-shot skip) | HIGH |
| 0x6AD | 1 | bool | 0 | **IsDeploying** | Ctor; Set_Dest: blocks destination when deploying; AI: blocks IPiggyback swap; ReceiveEMP: blocks EMP reset of some field; critical deployment gate | **HIGH** |
| 0x6AE | 1 | bool | 0 | **IsUndeploying** | Set_Dest_Internal: set to 1 when undeploying (NavCom cleared while deploying and linked building cleared) | HIGH |
| 0x6AF | 1 | byte | 0 | **TurretRateSync** | Written by `UnitClass::Facing_Update` (0x736990) only — clear at 0x736ad5, set at 0x736b16 from `CDTimerClass::Remaining()` when `Turret=yes && TurretSpins=no && timer>0`. Always 0 for non-turret units (CMIN, regular harvester, etc.). The `Receive_Radio(0x17)` scatter-suppression and `Receive_Radio(0x16) TIMING_SYNC` locomotor-sync reads gate on this field; "ShouldNotScatter" was the consumer's effect, not the cause. Corrected 2026-05-19 via TECHNOCLASS_0x6AF_CHRONO_STATE_FIELD report. | HIGH |
| 0x6B0 | 1 | byte | 0 | **Unknown_6B0** | Ctor [0x1AC] | LOW |
| 0x6B1 | 1 | bool | 0 | **SelfEnterQueued** | Enter_Destination: set when Enter_Destination called with self as target (deploy action). Mission_Harvest checks. | HIGH |
| 0x6B2 | 1 | byte | 0 | **Unknown_6B2** | Ctor | LOW |
| 0x6B3 | 1 | bool | 0 | **TickProcessedFlag** | Ctor; AI: cleared to 0 at start of every AI tick | HIGH |
| 0x6B4 | 1 | bool | 0 | **IPiggybackChecked** | Ctor [0x1AD]; AI: cleared after IPiggyback check completes | MED |
| 0x6B5 | 1 | byte | 0 | **Unknown_6B5** | Ctor | LOW |
| 0x6B6 | 1 | bool | **1** | **IsNewlyCreated** | Ctor init to **1** (unique non-zero/non-sentinel init). Likely "first tick" flag. | HIGH |
| 0x6B7 | 1 | bool | 0 | **DestinationJustSet** | Set_Dest_Internal: cleared to 0 at end of function | MED |
| 0x6B8 | 1 | byte | 0 | **JustRemovedFromTeam** | Ctor [0x1AE]; Remove_Member sets to 1 at start of function | HIGH |
| 0x6B9–0x6BF | 7 | — | — | **(padding to 0x6C0)** | Alignment padding to next subclass boundary | |

## 4. Field Usage Cross-Reference

### 4.1 Fields used in FootClass::AI (0x4DA530)

AI `param_1` is `int*`, so `param_1[N]` = byte `N*4`:

| AI Index | Byte Offset | Field | Usage |
|----------|-------------|-------|-------|
| [0x87] | 0x21C | Owner (HouseClass*) | Allied check for fog update |
| [0x14E] | 0x538 | MovementTickCounter | Incremented when locomotor is processing |
| [0x14F] | 0x53C | RankUpAnimActive | Veteran promotion anim state |
| [0x150] | 0x540 | RankUpAnimCountdown | Countdown for rank-up effect |
| [0x169] | 0x5A4 | NavCom | Movement destination |
| [0x175] | 0x5D4 | Team* | Gates tiberium self-heal, fog update |
| [0x197-0x199] | 0x65C-0x664 | FogUpdateTimer | Fog border update timing |
| [0x19D] | 0x674 | ILocomotion* | Locomotor process + IPiggyback query |
| [0x1A5] | 0x694 | LargeObjectPtr | Calls sub-object AI at +0x69C |
| [0x1AD] | 0x6B4 | IPiggybackChecked | Cleared after check |
| byte 0x6AD | 0x6AD | IsDeploying | Blocks piggyback swap |
| byte 0x6B3 | 0x6B3 | TickProcessedFlag | Cleared at tick start |

### 4.2 Fields used in Set_Destination_Internal (0x4D94B0)

| AI Index | Byte Offset | Field | Usage |
|----------|-------------|-------|-------|
| [0x168] | 0x5A0 | NavCom_Aux | Cleared to 0 at entry |
| [0x169] | 0x5A4 | NavCom | Set to destination target |
| [0x16A] | 0x5A8 | SuspendedNavCom | (not directly, but related) |
| [0x190-0x192] | 0x640-0x648 | PathDelayTimer | Reset; WalkLoco gets duration=1 |
| [0x19A-0x19C] | 0x668-0x670 | PathRetryTimer | Reset with PathDelay from rules |
| [0x19D] | 0x674 | ILocomotion* | Head_To_Coord called |
| [0x1AB] | 0x6AC | SkipHeadToCoord | One-shot gate |
| byte 0x6AD | 0x6AD | IsDeploying | Blocks destination changes |
| byte 0x6AE | 0x6AE | IsUndeploying | Set when deploy→undeploy |
| byte 0x6B7 | 0x6B7 | DestinationJustSet | Cleared at end |

### 4.3 Fields used in Find_Path (0x4D3920)

| Index | Byte Offset | Field | Usage |
|-------|-------------|-------|-------|
| [0x156] | 0x558 | HeadToCoord_0 | Written with cell coords on successful path |
| [0x175] | 0x5D4 | Team* | Gates path length (team limit vs MaxPathSteps) |
| [0x178] | 0x5E0 | PathHeadIndex | Set to -1 on failure, updated on success |
| [0x178+] | 0x5E4+ | PathSteps[] | Path data copied from A* result |
| [0x190-0x192] | 0x640-0x648 | PathDelayTimer | Reset after pathfinding |

### 4.4 Fields used in Locomotion_AI (0x520F40) — Infantry Animation

| Index | Byte Offset | Field | Usage |
|-------|-------------|-------|-------|
| [0x15E] | 0x578 | FormationSpeed (double) | Compared against threshold for walk vs run anim |
| [0x169] | 0x5A4 | NavCom | Checked for movement state |
| [0x19D] | 0x674 | ILocomotion* | Is_Moving check |
| byte 0x68D | 0x68D | HasReachedDock | Blocks walk→run transition |

## 5. INI Keys Affecting FootClass Behavior

FootClass does not have its own ReadINI — it inherits from TechnoClass. These keys
directly affect FootClass fields/logic:

| INI Key | Section | Affects | Evidence |
|---------|---------|---------|----------|
| `Speed=` | [UnitType] | GetCurrentSpeed → locomotor init | Base movement speed |
| `SpeedType=` | [UnitType] | Pathfinding zone checks | Terrain traversal classification |
| `MovementZone=` | [UnitType] | CanReachDestination, zone precheck | What terrain unit can cross |
| `Locomotor=` | [UnitType] | ILocomotion* at 0x674 | COM CLSID for locomotor |
| `ROT=` | [UnitType] | Movement tick | Body rotation speed |
| `CloakStop=` | [UnitType] | IsCloakable (TypeClass+0xC93) | Only cloak when stationary |
| `Passengers=` | [UnitType] | Transport capacity | Max passenger count |
| `DefaultToGuardArea=` | [UnitType] | Mission_Guard (TypeClass+0x390) | Auto-engage nearby enemies |

**[General] section:**

| INI Key | RulesClass Offset | Affects | Evidence |
|---------|-------------------|---------|----------|
| `PathDelay=` | +0x1768 | PathRetryTimer (0x670) | Set on destination change |
| `MaxPathSteps=` | +0x1718 | Find_Path step limit | When not in team |

## 6. Summary Statistics

- **Total FootClass-specific fields:** ~65 fields (0x520-0x6BF)
- **Fields with HIGH confidence:** ~35
- **Fields with MED confidence:** ~15
- **Fields with LOW/unknown:** ~10 (mostly in gaps 0x52C, 0x574, plus byte flags; 0x544-0x557 now resolved as SafePointerHandle + padding)
- **DynamicVectorClass instances:** 2 (WaypointQueue, EnterQueue)
- **CDTimerClass instances:** 5 (PathDelay, Movement, FogUpdate, PathRetry, Idle)
- **Critical pointers:** NavCom (0x5A4), ILocomotion* (0x674), Team* (0x5D4)
- **Double-precision fields:** 3 (SlopeSpeedFactor at 0x530, FormationSpeed at 0x578, SpeedMultiplier at 0x580) (corrected 2026-05-28: was "2" — ROOT_CAUSE: INFERENCE_HARDENED; all three confirmed via ctor decompile_function 0x4D31E0)

## Tier 4 application record (2026-08-17, Claude Code session)

Created /FootClass in the live DTM: 1728 bytes (0x6C0, the three-subclass-constructor
boundary proof above), 41 fields. Snapshot before mutations:
<local>/Documents/ghidra-backups/2026-08-17-pre-tier4 (19 files, verified).

Applied: this doc's HIGH rows only (MED/LOW stay holes) — NavCom trio 0x5A0-0x5A8,
TarCom block 0x5C4-0x5D1, Team pair 0x5D4/0x5D8, path queue 0x5E0/0x5E4, both DVec
queues' HIGH dwords, timer Start/Duration dwords, Locomotor 0x674, SlopeSpeedFactor,
MovementTickCounter, and the 0x684-0x6AD flag bytes. Plus the critic-proven mission
trio 0xAC/0xB0/0xB4 as YR_Mission in the base region. TechnoClass region 0x000-0x51F
deliberately empty pending its own tier.

Live re-verification this session: Clear_All_TarCom 004df1a0 (0x5C4/C8/CC/D1),
Stop_Moving 004df0d0 (0x5A0/0x5A4), TeamClass__Add_Member 006ea500 ([0x175]/[0x176]
indexing + 0x689 first-member-auto-arrive + 0x5E0 corroboration), plus the same-day
critic pass rows (0x5A8 save in Override_Mission; 0x674 dispatch at five AI sites).

Receivers typed FootClass* __thiscall: Stop_Moving void(), Clear_All_TarCom void(),
Mission_Move int() (new); AI, Override_Mission(YR_Mission, void*, void*),
Restore_Mission bool() (upgraded from MissionClass* — bodies access FootClass region).

Residuals:
- Zero-stack-arg claims for the three NEW prototypes rest on decompiler stack
  analysis, not manually read RET immediates — verify in the next critic pass.
- DTM field names carry tool-normalizer Hungarian prefixes (pNavCom etc.); the
  create_struct success message showed unprefixed names but storage prefixed them.
- ~70 other labeled FootClass functions: type receivers on contact.
