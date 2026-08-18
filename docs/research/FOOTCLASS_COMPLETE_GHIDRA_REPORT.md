# FootClass — Complete Ghidra Research Report

**Primary vtable:** `0x7E8C94`
**Constructor:** `0x4D31E0` (full), `0x4D3540` (minimal/load), `0x4D3590` (variant)
**FootClass::AI:** `0x4DA530`
**Confidence:** HIGH (90%+ overall, verified from binary)
**Active in YR:** Yes — FootClass is the base for all mobile units in YR
**Date:** 2026-04-01

## 1. Overview

FootClass is the base class for all **mobile** game objects in gamemd.exe. It inherits from
TechnoClass (which inherits AbstractClass → ObjectClass → MissionClass → RadioClass → TechnoClass)
and is the parent of InfantryClass, UnitClass, and AircraftClass.

FootClass owns: **navigation** (NavCom destination system), **pathfinding** (A* search + path
queue), **locomotion** (ILocomotion COM interface), **transport boarding** logic, **team
membership**, **movement timers**, **speed calculation**, and **mission implementations**
for Guard, Hunt, and Harvest.

**Class boundaries:**
- TechnoClass fields end at byte **0x520**
- FootClass-specific fields: bytes **0x520–0x6BF** (416 bytes)
- Subclass fields (InfantryClass/UnitClass/AircraftClass) start at byte **0x6C0**
- Total object size through FootClass: **0x6C0 = 1728 bytes**

This was verified by checking all three subclass constructors:
- InfantryClass constructor (0x517A50): first field at `param_1[0x1B0]` = byte 0x6C0
- UnitClass constructor (0x7353C0): first field at `param_1[0x1B0]` = byte 0x6C0
- AircraftClass constructor (0x413D20): first field at `param_1[0x1B0]` = byte 0x6C0

## 2. Complete FootClass Field Map (0x520–0x6BF)

All offsets are **byte offsets** from the start of the object (this pointer).
Fields above 0x520 are inherited from TechnoClass/ObjectClass/etc and are not listed.

### 2.1 Core Identity & Type (0x520–0x53F)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x520 | 4 | 0xFFFFFFFF | TypeOrAbstractID | Constructor [0x148] | Initialized to -1. Possibly the AbstractClass type token or a "not-yet-initialized" sentinel. |
| 0x524 | 2 | 0 | unknown_524 | Constructor | Word field |
| 0x526 | 2 | 0 | unknown_526 | Constructor | Word field |
| 0x528 | 2 | 0 | unknown_528 | Constructor | Word field |
| 0x52A | 2 | 0 | unknown_52A | Constructor | Word field |
| 0x52C | 4 | — | (gap) | | Between init groups |
| 0x530 | 4 | 0 | unknown_530 | Constructor [0x14C] | |
| 0x534 | 4 | 0 | unknown_534 | Constructor [0x14D] | |
| 0x538 | 4 | 0 | MovementCounter | Constructor [0x14E], AI report | Incremented each tick when locomotor is processing |
| 0x53C | 1 | 0 | VetAnimActive | Constructor [0x14F] byte, AI report | Whether veteran promotion anim is playing |
| 0x540 | 4 | 0 | VetAnimTimer | Constructor [0x150], AI report | Countdown timer for veteran anim duration |

### 2.2 Audio/Visual Handle + Padding (0x544–0x557)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x544 | 4 | 0 | LoopingSoundHandle_Ptr | SafePointerHandle field [0] | AnimClass/SoundEvent object pointer |
| 0x548 | 4 | 0 | LoopingSoundHandle_Stamp1 | SafePointerHandle field [1] | Validation stamp (vs object+0x138) |
| 0x54C | 4 | 0 | LoopingSoundHandle_Stamp2 | SafePointerHandle field [2] | Validation stamp (vs object+0x24) |
| 0x550 | 4 | &0x87e294 | LoopingSoundHandle_TypeTag | SafePointerHandle field [3] | Type sentinel pointer (global marker) |
| 0x554 | 4 | — | (padding) | Never accessed as FootClass field | Alignment gap before HeadToCoord block |

**Analysis (verified 2026-04-06):**
The 16-byte SafePointerHandle at 0x544–0x553 is initialized in the constructor
at 0x4d3408 (`LEA ECX,[ESI+0x544]`) → 0x4d3482 (`CALL FUN_00405be0`), and
again in FootClass::Load at 0x4db60e → 0x4db613.

In FootClass::AI, this handle is used for:
1. **Veteran rank-up anim** — AnimClass::Detach (0x405d40) at 0x4daa89 clears it,
   VocClass::PlayAt (0x7509e0) at 0x4daae2 plays the rank-up sound through it.
2. **Deploy/crash/falling sounds** — SoundEvent::Release (0x406060) clears it at
   0x4dab25, 0x4dacc6, 0x4dacf9, 0x4dadb1; VocClass::PlayAt writes through it
   at 0x4dac74 (crash sound), 0x4dacb7 (fallback crash), 0x4dada2 (falling sound).
3. **Looping sound update** — AnimClass::UpdateLoopingSound (0x750d40) at 0x4dadf0
   maintains the sound each tick.
4. **Limbo cleanup** — SoundEvent::Release at 0x4db353 releases it when entering limbo.

The handle is **not checksummed** (ComputeChecksum skips it) and is not save/load
serialized beyond the re-init in Load — consistent with it being a transient
audio/visual reference, not simulation state.

Bytes 0x554–0x557 are never accessed as a FootClass field anywhere in the binary.
Confirmed by exhaustive byte-pattern search for all register+0x554 displacement
encodings — the only hit was in BuildingClass context (different class layout).

### 2.3 Target Coordinate Block (0x558–0x577)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x558 | 2 | 0 | HeadToCoord_X1 | Constructor [0x156] word | Possibly last-known heading or target position |
| 0x55A | 2 | 0 | HeadToCoord_Y1 | Constructor | |
| 0x55C | 2 | 0 | HeadToCoord_X2 | Constructor [0x157] word | |
| 0x55E | 2 | 0 | HeadToCoord_Y2 | Constructor | |
| 0x560 | 2 | 0 | HeadToCoord_X3 | Constructor [0x158] word | |
| 0x562 | 2 | 0 | HeadToCoord_Y3 | Constructor | |
| 0x564 | 2 | 0 | HeadToCoord_X4 | Constructor [0x159] word | |
| 0x566 | 2 | 0 | HeadToCoord_Y4 | Constructor | |
| 0x568 | 4 | 0 | unknown_568 | Constructor [0x15A] | |
| 0x56C | 4 | 0 | unknown_56C | Constructor [0x15B] | |
| 0x570 | 4 | 0 | unknown_570 | Constructor [0x15C] | |
| 0x574 | 4 | — | (gap 574) | | Between [0x15C] and [0x15E] |
| 0x578 | 4 | 0 | FormationSpeed | Constructor [0x15E], Convoy report | Speed propagated from convoy leader to followers |
| 0x57C | 4 | 0 | unknown_57C | Constructor [0x15F] | |

### 2.4 Speed Multiplier (0x580–0x587)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x580 | 8 | 1.0 (double) | SpeedMultiplier | Constructor [0x160]=0, [0x161]=0x3FF00000 | IEEE754 double 1.0. Formation/convoy speed scale factor. |

### 2.5 Waypoint Queue — DynamicVectorClass (0x588–0x5A7)

This is a `DynamicVectorClass<AbstractClass*>` — the waypoint queue for queued move commands.

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x588 | 4 | &DVec_vtable | WaypointQueue_vtable | Constructor [0x162] | DynamicVectorClass vtable |
| 0x58C | 4 | — | WaypointQueue_Data | Mission_Harvest [0x163] | Pointer to heap-allocated element array |
| 0x590 | 4 | — | WaypointQueue_field2 | | |
| 0x594 | 4 | — | WaypointQueue_field3 | | |
| 0x598 | 4 | 0 | WaypointQueue_Count | Constructor [0x166], Mission_Hunt [0x166] | Current number of queued waypoints |
| 0x59C | 4 | 10 | WaypointQueue_Capacity | Constructor [0x167] | Initial capacity = 10 |

**Usage in Mission_Hunt (0x4D4280):**
```
if (WaypointQueue_Count > 0) {
    target = WaypointQueue_Data[Count - 1];  // last waypoint = final destination
} else {
    target = NavCom;
}
```

**2026-05-27 correction:** `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` found no standard YR runtime player, TeamClass/AI, or trigger waypoint producer that appends to this queue. The field is serialized/deserialized and consumed/cleaned when nonzero; do not infer normal shift-click waypoint chaining from the consumer code.

### 2.6 NavCom System (0x5A0–0x5AB)

The NavCom is the **navigation computer** — the current movement destination.

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x5A0 | 4 | 0 | NavCom_Aux | Constructor [0x168], Set_Dest, Stop_Moving | Cleared by Set_Destination_Internal and Stop_Moving. Auxiliary target used during destination changes. |
| 0x5A4 | 4 | 0 | **NavCom** | Constructor [0x169], AI, Set_Dest, Stop_Moving, Receive_Radio | **THE destination target pointer.** Primary navigation target. When non-zero, the unit is heading somewhere. Cleared by Stop_Moving. |
| 0x5A8 | 4 | 0 | SuspendedNavCom | Constructor [0x16A], Set_NavCom_With_Suspend | Backup of NavCom before suspension. Set_NavCom_With_Suspend copies NavCom here before overwriting NavCom with the new target. |

**NavCom lifecycle:**
1. `Set_Destination_Internal` (vtable+0x480): Sets NavCom, calls ILocomotion::Head_To_Coord
2. `Set_NavCom_With_Suspend`: Copies NavCom→SuspendedNavCom, then calls SetDestination
3. `Stop_Moving`: Clears both NavCom_Aux (0x5A0) and NavCom (0x5A4) to 0
4. `Receive_Radio(0x13)`: Returns NavCom + is_moving status
5. GuardArea check: If NavCom == 0 and mission is Guard, scatter

**Guards in Set_Destination_Internal:**
- If byte 0x6AD (deploy/locomotor-piggyback active guard) is set AND target is non-null → reject (return early)
- If byte 0x82 (InLimbo) is set AND target is non-null → reject
- If dword 0x2E4 (WarheadBusy?) is non-zero AND target is non-null → reject

### 2.7 Enter Queue — DynamicVectorClass (0x5AC–0x5C3)

A `DynamicVectorClass<AbstractClass*>` — queue of objects this unit wants to enter (buildings, transports).

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x5AC | 4 | &DVec_vtable | EnterQueue_vtable | Constructor [0x16B] | |
| 0x5B0 | 4 | — | EnterQueue_Data | Enter_Destination [0x16C] | Heap-allocated element array |
| 0x5B4 | 4 | — | EnterQueue_field2 | | |
| 0x5B8 | 4 | — | EnterQueue_IsAllocated | Enter_Destination [0x5B9] | Byte flag |
| 0x5BC | 4 | 0 | EnterQueue_Count | Constructor [0x16F], Enter_Destination [0x16F] | |
| 0x5C0 | 4 | 10 | EnterQueue_Capacity | Constructor [0x170] | Initial capacity = 10 |

**Usage in Enter_Destination (0x4DA0E0):**
- If target == self and count > 0: sets byte 0x6B1 to 1 (self-enter marker, e.g., deploy)
- Otherwise: pushes target onto the vector
- If NavCom == 0, mission is Guard(5), and destination isn't a docking building: calls vtable+0x484

### 2.8 Arrival Target Block (0x5C4–0x5D7)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x5C4 | 4 | 0xFFFFFFFF | ArrivalTarget_ID | Constructor [0x171] | Initialized to -1 (none) |
| 0x5C8 | 4 | 0 | ArrivalCheckTarget | Constructor [0x172], Arrival_Target_Handler | When non-zero, Arrival_Target_Handler takes the "has target" path |
| 0x5CC | 4 | 0 | ArrivalFollowTarget | Constructor [0x173], Arrival_Target_Handler | Target being followed; checked against CanFire |
| 0x5D0 | 1 | — | (padding) | | |
| 0x5D1 | 1 | 0 | IsFollowingTarget | Constructor direct, Arrival_Target_Handler | Set to 1 on arrival at attack position. Checked to decide whether to check AnotherTarget (0x2B4). |
| 0x5D2–0x5D3 | 2 | — | (padding) | | |
| 0x5D4 | 4 | 0 | TeamLeaderOrPtr | Constructor [0x175], AI, Evaluate_Target_Threat, Find_Path | Pointer to some team/group context. In AI: gates tiberium self-heal and fog update. In Evaluate_Target_Threat: checked for valid team. In Find_Path: gates path length override. |

### 2.9 Team & Following (0x5D8–0x5DF)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x5D8 | 4 | 0 | TeamNextMember | Constructor [0x176], Convoy report | Linked list pointer to next member in team |
| 0x5DC | 4 | 0 | GhostCell | Constructor [0x177], Mission_Hunt | Temp storage for cell being navigated to; set during Hunt state 0 when finding nearby passable cell |

### 2.10 Path Queue (0x5E0–0x63F)

The path queue stores up to **24 movement direction steps** from A* pathfinding.

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x5E0 | 4 | 0xFFFFFFFF | PathHeadIndex | Constructor [0x178], Find_Path, Run_AStar | Current step index in path. -1 = no path. |
| 0x5E4 | 92 | — | PathSteps[23] | Find_Path (copies up to 0x18 = 24 entries) | Direction steps copied from A* result. Each entry is 4 bytes (FacingType direction enum). Total = 24 × 4 = 96 bytes including PathHeadIndex. |

**Find_Path writes path steps:**
```
max_steps_to_copy = min(24 - current_offset, astar_result.count)
copy path data to this + 0x178 + offset (dword-indexed)
```

**Run_AStar (0x4CBBA0):**
- Calls `Path_walk_directions_to_cell` with `this + 0x178` (byte 0x5E0) as the path buffer
- Then calls `AStar_pathfind_search` with result written back

### 2.11 Timer Block (0x640–0x673)

Four CDTimerClass instances (each ~12 bytes: start_frame + value + duration).

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| **Timer 1: Walk/Infantry Path Delay** | | | | | |
| 0x640 | 4 | CurrentFrame | PathDelayTimer_Start | Constructor [0x190], Set_Dest, Find_Path | Base frame for path delay timer |
| 0x644 | 4 | — | PathDelayTimer_Value | Set_Dest [0x191] | Associated value (set during destination changes) |
| 0x648 | 4 | 0 | PathDelayTimer_Duration | Constructor [0x192], Set_Dest | Walk path delay. Set_Destination resets to 0 for fresh path, set to 1 when WalkLocomotion active. |
| **Timer 2: General Movement Timer** | | | | | |
| 0x64C | 4 | 10 | MovementTimer_Max | Constructor [0x193] | Initialized to 10, appears to be a max delay or capacity |
| 0x650 | 4 | CurrentFrame | MovementTimer_Start | Constructor [0x194] | |
| 0x654 | 4 | — | MovementTimer_Value | | Not initialized in constructor |
| 0x658 | 4 | 0 | MovementTimer_Duration | Constructor [0x196] | |
| **Timer 3: Fog/Shroud Update** | | | | | |
| 0x65C | 4 | CurrentFrame | FogUpdateTimer_Start | Constructor [0x197], AI report | Last frame fog border was updated |
| 0x660 | 4 | — | FogUpdateTimer_Value | | Not initialized in constructor |
| 0x664 | 4 | 0 | FogUpdateDelay | Constructor [0x199], AI report | Set to 0xF (15) in AI when fog update occurs |
| **Timer 4: NavCom/Path Retry** | | | | | |
| 0x668 | 4 | CurrentFrame | PathRetryTimer_Start | Constructor [0x19A], Set_Dest_Internal | Reset when destination changes |
| 0x66C | 4 | — | PathRetryTimer_Value | Set_Dest [0x19B] | |
| 0x670 | 4 | 0 | PathRetryDelay | Constructor [0x19C], Set_Dest | Set to `RulesClass+0x1768` (PathDelay from rules.ini [General]) on destination set |

### 2.12 ILocomotion Pointer (0x674)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| **0x674** | **4** | **0** | **ILocomotion*** | **Constructor [0x19D], AI, Set_Dest, Head_To_Coord, IsCloakable** | **THE locomotor COM interface pointer. Most critical field in FootClass.** Created via CoCreateInstance from TechnoTypeClass+0x34C CLSID in subclass constructors. Queried for IPiggyback every tick in AI. |

### 2.13 Last Known Good Coordinates (0x678–0x683)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x678 | 4 | DAT_008b3da8 | LastGoodCoord_X | Constructor [0x19E] | Initialized from global (possibly {0,0,0} or special sentinel). Used for scatter/fallback positioning. |
| 0x67C | 4 | DAT_008b3dac | LastGoodCoord_Y | Constructor [0x19F] | |
| 0x680 | 4 | DAT_008b3db0 | LastGoodCoord_Z | Constructor [0x1A0] | |

### 2.14 Flags Block (0x684–0x6BF)

| Byte Offset | Size | Init Value | Field Name | Evidence | Notes |
|-------------|------|------------|------------|----------|-------|
| 0x684 | 1 | 0xFF | DriveTrackIndex | Constructor [0x1A1] byte, AI report | 0xFF = no active drive track. Index into drive track table when vehicle is following a curved path. |
| 0x685 | 1 | 0 | unknown_685 | Constructor | |
| 0x686 | 1 | 0 | unknown_686 | Constructor | |
| 0x687 | 1 | 0 | DeferredArrivalHookFlag | Constructor, Save_Convoy_State, OnArrival | When set, OnArrival clears it and calls vtable `+0x174(&DAT_008B3DA8,1,0)`. Stock Unit/Infantry resolve this hook to Scatter; producers outside the OnArrival slice remain deferred. |
| 0x688 | 1 | 0 | ConvoyDisbanded | Constructor, Convoy report | Set to 1 in Clear_Convoy_Chain for each member |
| 0x689 | 1 | 0 | ConvoyArrived | Constructor, Convoy report | Used by team movement to track arrived members |
| 0x68A | 1 | 0 | unknown_68A | Constructor | |
| 0x68B | 1 | 0 | unknown_68B | Constructor | |
| 0x68C | 1 | 0 | unknown_68C | Constructor [0x1A3] | |
| 0x68D | 1 | 0 | HasReachedDock | Mission_Harvest | Checked in harvest mission for dock proximity |
| 0x68E | 1 | 0 | HasFoundAutoTarget | Mission_Guard | Set to 1 when guard mission finds an auto-target building to enter |
| 0x68F | 1 | 0 | IsReceivingRepair | Mission_Guard, Mission_Harvest | When set, Mission_Guard calls RepairAI (vtable+0x340) |
| 0x690 | 1 | 0 | IsDockingToBuilding | Constructor [0x1A4], Find_Nearest_Dock, Mission_Guard | Set to 1 by Find_Nearest_Dock when dock found. Checked in Mission_Guard for docking behavior. |
| 0x691 | 1 | 0 | IsWeedingHarvester | Mission_Guard, Mission_Harvest | When set, Mission_Guard calls WeedHarvestAI (vtable+0x34C) |
| 0x692–0x693 | 2 | 0 | (padding/unknown) | | |
| 0x694 | 4 | 0 | **LargeObjectPtr** | Constructor [0x1A5], AI report | **NOT TeamClass*** (corrected 2026-05-28: was "Team*"; binary shows this is a large-object pointer >0x69C bytes, not TeamClass which is only ~0xA0 bytes — ROOT CAUSE: RTTI_LABEL_DRIFT; TeamClass::Add_Member verified via `decompile_function 0x6EA500` writes Team* to param_1[0x175]=0x5D4, not 0x694). In AI, accessed as `*(ptr+0x69C)->vtable[0x5C]()` — sub-object whose AI() is dispatched each tick. See Section 9.2. Team* is at **0x5D4**, not here. |
| 0x698 | 4 | 0 | unknown_698 | Constructor [0x1A6] | |
| 0x69C | 4 | 0 | unknown_69C | Constructor [0x1A7] | |
| **Timer 5: Team/Idle Timer** | | | | | |
| 0x6A0 | 4 | CurrentFrame | IdleTimer_Start | Constructor [0x1A8] | |
| 0x6A4 | 4 | — | IdleTimer_Value | | Not initialized in constructor |
| 0x6A8 | 4 | 0 | IdleTimer_Duration | Constructor [0x1AA] | |
| 0x6AC | 1 | 0 | skip_head_to_coord_once | Constructor [0x1AB] byte, Set_Dest_Internal | One-shot flag: accepted destination still writes NavCom, then clears this byte and skips ILocomotion::Head_To_Coord once. |
| 0x6AD | 1 | 0 | **deploy_or_locomotor_piggyback_active** | Constructor, Set_Dest, PerformDeploy | Runtime Foot guard. Blocks non-null destination writes while set; null destination uses it with owner `+0x2B0` to drive linked-object cleanup. Do not reduce to only `IsDeploying` or `IsDeployed`. |
| 0x6AE | 1 | 0 | post_deploy_link_cleanup_marker | Set_Dest_Internal | Set to 1 after null destination clears the owner `+0x2B0` / linked object `+0x2AC` relationship while `+0x6AD` is set. |
| 0x6AF | 1 | 0 | ShouldNotScatter | Receive_Radio(0x17) | When set, prevents scatter on radio unload message |
| 0x6B0 | 1 | 0 | unknown_6B0 | Constructor [0x1AC] | |
| 0x6B1 | 1 | 0 | SelfEnterQueued | Enter_Destination, Mission_Harvest | Set to 1 when Enter_Destination is called with self as target (deploy). Checked in Mission_Harvest. |
| 0x6B2 | 1 | 0 | unknown_6B2 | Constructor | |
| 0x6B3 | 1 | 0 | TickProcessedFlag | Constructor, AI report | Cleared to 0 at start of every AI tick |
| 0x6B4 | 1 | 0 | IPiggybackChecked | Constructor [0x1AD], AI report | Cleared to 0 after IPiggyback check completes each tick |
| 0x6B5 | 1 | 0 | unknown_6B5 | Constructor | |
| 0x6B6 | 1 | **1** | IsNewlyCreated | Constructor (**initialized to 1!**) | The only field initialized to a non-zero/non-sentinel value. Likely "first tick" flag. |
| 0x6B7 | 1 | 0 | DestinationJustSet | Set_Dest_Internal, Constructor | Cleared to 0 at end of Set_Destination_Internal |
| 0x6B8 | 1 | 0 | unknown_6B8 | Constructor [0x1AE] byte | Last explicitly initialized byte in constructor |
| 0x6B9–0x6BF | 7 | — | (padding to 0x6C0) | | Alignment to next subclass boundary |

## 3. Core Logic — Key Methods

### 3.1 FootClass::AI (0x4DA530) — Per-Tick Update

Called every tick from subclass AI methods. 10 subsystems in order:

1. **TechnoClass::AI_Update** (parent) — at 0x6F9E50
2. **Tiberium self-heal** — if on tib cell and TeamLeaderOrPtr (0x5D4) is set
3. **Veteran promotion check** — VetAnimActive/VetAnimTimer at 0x53C/0x540
4. **ILocomotion::Process** — calls vtable+0x40 on ILocomotion* at 0x674
5. **Movement counter** — increments 0x538 when loco is processing
6. **Rank-up / falling anim** — spawns visual effects
7. **IPiggyback locomotor swap** — chrono miner restore sequence (see Section 3.7)
8. **TryEnterTransport** — attempts to board transport at 0x500
9. **Team::AI dispatch** — if Team* (0x694) is non-null
10. **Idle scatter** — every 64 frames, scatter idle units

### 3.2 FootClass::Set_Destination_Internal (0x4D94B0) — NavCom Write

The central destination-setting method. Called via vtable+0x480.

**Guards (reject destination if):**
- `deploy_or_locomotor_piggyback_active` (0x6AD) is set AND target is non-null
- `InLimbo` (0x82) is set AND target is non-null
- Warhead busy field (0x2E4) is non-zero AND target is non-null
- If owner `+0x2AC` active AND target is non-null: triggers the chrono/deploy helper before NavCom write

**When target is set (non-null):**
1. Clear combat link at 0x304 if present
2. Check if locomotor supports IPiggyback (for chrono units)
3. **Special WalkLocomotion path delay:** If locomotor CLSID == WalkLocomotion GUID, applies
   the infantry path delay timer (0x640-0x648). This prevents infantry from repathing too
   frequently.
4. If byte 0x6AC is clear: gets target coordinates and calls `ILocomotion::Head_To_Coord`
   (vtable+0x44 on ILocomotion*). If byte 0x6AC is set, clears it (one-shot skip).
5. Resets PathRetryTimer (0x668-0x670) with PathDelay from `RulesClass+0x1768`
6. Resets PathDelayTimer (0x640-0x648)
7. Clears DestinationJustSet (0x6B7) to 0

**When target is null (stop):**
- If `target == NULL && +0x6AD != 0 && owner +0x2B0 != 0`: clears linked object's `+0x2AC`, clears owner `+0x2B0`, then sets `+0x6AE`
- If locomotor exists: calls `ILocomotion::Stop` (vtable+0x48)

### 3.3 FootClass::Set_NavCom_With_Suspend (0x4D8F40)

Simple 3-step sequence:
1. Copy NavCom (0x5A4) → SuspendedNavCom (0x5A8)
2. Call intermediate function (target translation)
3. Call SetDestination (vtable+0x480) with new target

Used when a unit needs to temporarily divert (e.g., enter a repair depot) but should
remember where it was going.

### 3.4 FootClass::Stop_Moving (0x4DF0D0)

Minimal function — just zeroes two fields:
```
this+0x5A0 = 0  (NavCom_Aux)
this+0x5A4 = 0  (NavCom)
```
Does NOT call ILocomotion::Stop. The locomotor stop happens in Set_Destination_Internal
when called with target=0.

### 3.5 FootClass::GetCurrentSpeed (0x4DB1A0)

Speed calculation chain:
1. Get TypeClass (vtable+0x84)
2. Apply `HouseClass::GetSpeedBonus` (country multiplier)
3. Call vtable+0x38C (mission-specific speed modifier)
4. `Math::ftol` to integer
5. If `TechnoClass::HasWeaponAbility(FASTER)` (veteran): apply speed bonus
6. **Aircraft special:** If `What_Am_I() == 1` (aircraft) AND field 0x6CC != -1: return speed/2
   (NOTE: 0x6CC is in AircraftClass territory — this is the airport docking slot. Aircraft
   approaching their helipad fly at half speed.)

### 3.6 FootClass::Find_Path (0x4D3920) — Pathfinding Entry

Large function (238 lines). High-level flow:

1. **Early exits:** If destination unreachable (vtable+0x2CC returns false), set PathHeadIndex=-1
2. **Distance check:** Compute distance to destination
3. **Path length limit:** If TeamLeaderOrPtr (0x5D4) is set, use team's path limit; otherwise use RulesClass+0x1718 (MaxPathSteps)
4. **Impassable cell handling:** If destination cell is impassable (type 6 or 7), find nearby passable cell via `FootClass::Find_Nearby_Passable_Cell`
5. **A* search:** Call `FootClass::Run_AStar` → `AStar_pathfind_search`
6. **Copy path:** Copy up to 24 steps from A* result into path queue at 0x5E0
7. **Aircraft convoy propagation:** If aircraft (type 1) and PathHeadIndex != -1 and has
   convoy chain, propagate paths to convoy followers
8. **Failure handling:** If no path found, clear destination and attempt re-target for AI units

### 3.7 IPiggyback Locomotor Swap (in AI, 0x4DAE5F–0x4DAEC6)

Every tick, FootClass::AI checks if the active locomotor supports IPiggyback:

```pseudocode
loco = this.ILocomotion  // 0x674
if loco == null: skip

piggyback = loco.QueryInterface(IID_IPiggyback)
if piggyback == null: skip  // E_NOINTERFACE is expected for non-piggybacking locos

if piggyback.Is_Ok_To_End():
    loco.Release()          // release the piggyback (temporary) locomotor
    this.ILocomotion = null
    piggyback.End_Piggyback(&this.ILocomotion)  // moves stored loco back into 0x674
    // NO AddRef needed — ownership transferred
    // NO Link_To_Object needed — persists through piggyback

piggyback.Release()
```

**Key invariant:** runtime Foot `+0x6AD` (`deploy_or_locomotor_piggyback_active`) blocks this swap entirely.

### 3.8 FootClass::TryEnterTransport (0x70D7E0) — Transport Boarding

Called every tick from AI when TransportTarget (0x500) is set:

1. Check transport target at `this[0x140]` = byte 0x500
2. If transport is dead (`health == 0`): clear target, return
3. If transport is valid and not in limbo:
   a. Send `RADIO_CAN_LOAD` (2) to transport
   b. If accepted: send `RADIO_LOADING` (0xF)
      - If loading accepted: queue `MISSION_ENTER` (7), set destination to transport, clear target
      - If loading rejected: cancel mission, clear destination, send `RADIO_OVER_AND_OUT` (3)
   c. **Building garrison special:** If transport is BuildingClass (type 6) and this is
      InfantryClass (type 2) and building has `CanBeOccupied` (+0x16AA) but NOT
      `MaxNumberOccupants` check (+0x16A9): send radio 0xE (garrison request)

### 3.9 FootClass::Receive_Radio (0x4D8FB0) — Radio Message Handler

Handles 6 radio message types beyond TechnoClass:

| Message | ID | Handler |
|---------|-----|---------|
| RADIO_NEED_TO_MOVE | 0x11 | If mission is ENTER(7) or queued mission is ENTER: accept |
| RADIO_SET_RALLY | 0x12 | Compare sender coords to own cell; if different, set destination. If mission is Guard(5), queue Move(2). |
| RADIO_QUERY_DEST | 0x13 | Return NavCom (0x5A4). If NavCom is set and ILocomotion::Is_Moving returns true, return NEGATIVE(10). |
| RADIO_UNLOAD | 0x17 | If path has valid steps and dest matches NavCom: clear destination. If mission None(0): queue Guard(5). If no NavCom and not ShouldNotScatter(0x6AF): Scatter. |
| RADIO_BUSY | 0x1C | If NavCom is non-zero: return NEGATIVE(10). |
| RADIO_CHECK_BUILDING | 0x23 | Get cell, look up building in cell, return found/not-found. |

### 3.10 FootClass::Mission_Guard (0x4D5070) — Guard Mission

Complex guard behavior with priority checks:

1. **If IsReceivingRepair (0x68F):** Call RepairAI (vtable+0x340)
2. **If IsDockingToBuilding (0x690):** Call DockAI (vtable+0x348)
3. **If IsWeedingHarvester (0x691):** Call WeedHarvestAI (vtable+0x34C)
4. **If no attack target:** Scan 8 adjacent cells for friendly buildings with `IsRepairable`
   flag (+0x1575). If found and same owner: queue Attack(1) mission on that building.
   (This is the auto-repair-at-depot behavior.)
5. **If attack target exists and TypeClass has `DefaultToGuardArea` (+0x390):** Guard area behavior
6. **Aircraft auto-return:** If type 0xF and AI-controlled and has `SelfHeal` ability:
   queue mission 0x11 (Return to base)
7. **Timer-based return:** Uses timer at 0x2F4/0x2FC for periodic guard refresh

### 3.11 FootClass::Arrival_Target_Handler (0x4DF3A0)

Called when a unit reaches its movement destination. Decides whether to attack:

1. If `ArrivalCheckTarget` (0x5C8) is zero:
   - Check ArrivalFollowTarget (0x5CC) against CanFire (vtable+0x3B4)
   - Check IsFollowingTarget (0x5D1) against AnotherTarget (0x2B4) + CanFire
   - Clear AnotherTarget (0x2B4) and attempt engagement at current coords
   - On success: queue Attack(1) mission, set IsFollowingTarget = 1
2. If `ArrivalCheckTarget` (0x5C8) is non-zero:
   - Check IsFollowingTarget against AnotherTarget + CanFire
   - Attempt engagement at current coords
   - On success: queue Attack(1) mission, set IsFollowingTarget = 1

### 3.12 FootClass::Find_Nearest_Dock (0x4DFCB0)

Iterates owner house's building list (HouseClass+0xE4 array, count at +0xF0):
1. For each building: compute 3D Euclidean distance
2. Check `BuildingClass::CanDock` for compatibility
3. Pick nearest valid dock
4. Set IsDockingToBuilding (0x690) = 1
5. Set NavCom to dock building
6. Queue mission 8 (Enter/Dock)

### 3.13 FootClass::Evaluate_Target_Threat (0x4D97A0)

Target priority scoring for combat AI:

1. If target is current attack target (0x2AC): return -GetWeaponDamage (highest priority)
2. If attack target is valid and cloaked: return 0 (can't engage cloaked)
3. If in team (0x5D4) and team doesn't allow independent targeting: return 0
4. If mission is Sleep(10): return 0
5. Base score = GetWeaponDamage << 10
6. **Distance penalty:** score / (distance_to_target / max(speed, 1))
7. Minimum score: 1 (never returns 0 for valid engageable targets)

### 3.14 FootClass::IsCloakable (0x4DBDA0)

1. Check `TechnoClass::HasStealthAbility` (Cloakable= or elite CLOAK ability)
2. If TypeClass+0xC93 is set (**CloakStop=yes** — "only cloak when stationary"):
   - Check `ILocomotion::Is_Moving_Now` (vtable+0x80)
   - If moving: return NOT cloakable
3. Otherwise: return cloakable

### 3.15 FootClass::Head_To_Coord_Dispatch (0x4D55F0)

Simple forwarding function:
1. Assert ILocomotion* (0x674) is non-null
2. Call `ILocomotion::Head_To_Coord` (vtable+0x44) with the coordinate
3. Call `ILocomotion::Is_Moving` (vtable+0x10) and return result

## 4. VTable Layout (at 0x7E8C94)

Selected entries (offset from vtable start → function address):

| VTable Offset | Address | Method | Notes |
|---------------|---------|--------|-------|
| +0x00 | 0x410260 | AbstractClass::Save | |
| +0x2C | varies | What_Am_I | RTTI type ID |
| +0x38 | varies | GetSize | Instance size |
| +0x48 | 0x5F65A0 | GetCoords | Returns location as CoordStruct |
| +0x4C | varies | GetTargetCoords | |
| +0x54 | 0x5F6B90 | IsActive/IsHealthy | Thunk |
| +0x5C | **0x4DA530** | **FootClass::AI** | Per-tick update |
| +0x84 | 0x6F3270 | GetTechnoType | Returns TypeClass* |
| +0xBC | varies | GetZoneType | |
| +0x124 | varies | UpdateOccupancy | |
| +0x168 | varies | GetWeaponRange | |
| +0x16C | varies | ReceiveDamage | |
| +0x174 | 0x5F43A0 | Scatter | **No-op in FootClass** — overridden by Infantry/Unit |
| +0x184 | varies | GetCurrentMission | |
| +0x1AC | varies | GetMoveType | |
| +0x1B8 | varies | GetCell | |
| +0x1C8 | varies | GetHeight | |
| +0x1E8 | varies | QueueMission | |
| +0x1EC | varies | ResumeMission | |
| +0x200 | varies | CanResumeMission | |
| +0x278 | varies | TransmitRadio | |
| +0x2B0 | varies | IsNotAtDestination | |
| +0x2C0 | varies | GetWeaponDamage | |
| +0x2CC | varies | CanReachDestination | |
| +0x2E4 | varies | GetBestWeapon | |
| +0x31C | varies | GetGuardScanRange | |
| +0x330 | varies | ShouldRepairTarget | |
| +0x340 | varies | RepairAI | |
| +0x348 | varies | DockAI | |
| +0x34C | varies | WeedHarvestAI | |
| +0x388 | varies | GetThreatRange | |
| +0x38C | varies | GetMissionSpeedMult | |
| +0x39C | varies | CanEngageAtCoord | |
| +0x3A8 | varies | CanFireAtTarget | |
| +0x3B4 | varies | IsTargetValidForFire | |
| +0x3C4 | varies | Greatest_Threat | |
| +0x3C8 | varies | SetAttackTarget | |
| +0x3F8 | varies | GetWeaponStruct | |
| +0x478 | varies | UpdateGuardArea | |
| **+0x480** | **0x4D94B0** | **SetDestination** | FootClass::Set_Destination_Internal |
| +0x484 | 0x4D82B0 | SetDestination_Alt | Alternative destination setter |
| +0x488 | varies | (FootClass-specific) | |
| +0x500 | varies | OnPathFailure | |
| +0x52C | varies | CanDockWith | |
| +0x53C | varies | ClearAttackTarget | |
| +0x540 | varies | AssignPath | |

## 5. INI Keys Read by FootClass

FootClass itself doesn't have a separate ReadINI — it inherits from TechnoClass::ReadINI.
However, these keys directly affect FootClass behavior:

| INI Key | Section | Default | Used At | Effect |
|---------|---------|---------|---------|--------|
| `Speed=` | [UnitType] | 0 | GetCurrentSpeed, locomotor init | Base movement speed in leptons/frame |
| `SpeedType=` | [UnitType] | — | Pathfinding zone checks | Terrain traversal classification |
| `MovementZone=` | [UnitType] | — | CanReachDestination, zone precheck | What terrain types the unit can cross |
| `Locomotor=` | [UnitType] | — | Constructor (CLSID at TypeClass+0x34C) | COM CLSID for ILocomotion implementation |
| `ROT=` | [UnitType] | — | Movement tick | Body rotation speed |
| `CloakStop=` | [UnitType] | No | IsCloakable (TypeClass+0xC93) | Only cloak when not moving |
| `Passengers=` | [UnitType] | 0 | Transport capacity | Max passenger count |
| `SizeLimit=` | [UnitType] | — | Transport boarding | Max passenger size |
| `DefaultToGuardArea=` | [UnitType] | No | Mission_Guard (TypeClass+0x390) | Auto-engage nearby enemies when idle |

**From [General] section:**
| INI Key | Offset in RulesClass | Default | Effect |
|---------|---------------------|---------|--------|
| `PathDelay=` | +0x1768 | ? | Ticks between path retries (stored at 0x670 on destination set) |
| `MaxPathSteps=` | +0x1718 | ? | Maximum A* search depth per pathfinding call |
| `RepairRate=` | +0x1708 | ? | Health ratio threshold for auto-repair in Mission_Guard |

## 6. Integration Points

### Who calls FootClass::AI?
- `InfantryClass::AI` (0x51BAB0) → calls FootClass::AI at 0x51BC9F
- `UnitClass::AI` (0x7360C0) → calls FootClass::AI at 0x73647B
- `AircraftClass::AI` → calls FootClass::AI via vtable inheritance

### What does FootClass::AI call?
- `TechnoClass::AI_Update` (0x6F9E50) — parent tick
- `ILocomotion::Process` (vtable+0x40 on ILocomotion*) — locomotor state machine
- Unknown sub-object AI — if pointer at 0x694 is set, calls `(ptr+0x69C)->AI()`
- `FootClass::TryEnterTransport` (0x70D7E0) — if TransportTarget (0x500) is set
- IPiggyback swap sequence — every tick

### Tick position in World::advance_tick:
FootClass::AI runs as part of the per-object AI loop, which is **before** explicit combat
processing. The order within FootClass::AI itself is: locomotor first, then transport,
then team, then scatter — ensuring movement completes before boarding decisions.

## 7. Current Rust Implementation Status

### What's implemented:
- `GameEntity` struct (game_entity.rs) — flat equivalent of the entire class hierarchy
- `LocomotorState` (locomotor.rs) — replaces ILocomotion COM pointer, 7-state ground + 5-state air
- `NavigationState` (components.rs) — current NavCom-shaped owner state with `nav_com_aux`, `nav_com`, `suspended_nav_com`, and `nav_queue`
- `MovementTarget` (components.rs) — active path/movement execution, still not a byte-perfect replacement for native NavCom/locomotor split
- `PassengerRole` (passenger.rs) — replaces transport boarding logic
- Movement tick (movement_tick.rs) — full ground movement with drive tracks, cell transitions
- Pathfinding (pathfinding/core.rs) — A* with zone connectivity and terrain costs
- Scatter system (scatter.rs) — idle + deferred scatter
- Group destination spreading (group_destination.rs)
- Aircraft mission state machine (aircraft.rs)

### What's missing or differs:
- **SuspendedNavCom exists but lacks native cleanup/restore parity** — `NavigationState::suspended_nav_com` exists, but PointerExpired/mission restoration semantics are not complete
- **No EnterQueue (0x5AC DynamicVectorClass)** — we don't queue enter-destinations
- **WaypointQueue/NavQueue exists but normal runtime producers are wrong/too broad** — newer binary coverage found no standard YR player/team/trigger producer; save-load/consumer compatibility remains relevant
- **No exact `+0x6AD` guard** — local deploy state does not yet model the runtime Foot deploy/locomotor-piggyback active guard and its "clear aux, preserve NavCom" silent-drop order
- **No TeamClass membership** — no persistent team/group coordination (Team* at 0x5D4)
- **No Arrival_Target_Handler** — we don't have the "arrived, now attack?" decision
- **No IPiggyback swap** — teleport locomotor override exists but doesn't use COM-style piggybacking
- **No Walk path delay** — infantry don't have the special repathing delay timer
- **No PathRetryDelay** — from RulesClass PathDelay= setting
- **No FormationSpeed** — convoy/formation speed synchronization
- **No CloakStop behavior** — IsCloakable doesn't check movement state
- **No radio message system** — transport boarding is direct state machine, not radio-based

### What works differently:
- Our `GameEntity` is a single flat struct vs the deep inheritance hierarchy
- Our pathfinding returns full paths, not 24-step segments with repathing
- Our locomotor is an enum + state, not a COM object
- Owner destination state now exists separately in `NavigationState`, but movement execution still needs to preserve the native split between owner NavCom, Drive destination/head-to, and active path/track state.

## 8. Corrections to Previous Reports

**CORRECTION: Convoy fields are NOT in FootClass.**

The CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT attributed fields at 0x6C4-0x6D2 to FootClass.
This is **WRONG**. All three subclass constructors (InfantryClass, UnitClass, AircraftClass)
begin their fields at byte 0x6C0:
- UnitClass: 0x6C0 = -1 sentinel, 0x6C4 = UnitTypeClass*, 0x6C8+ = unit-specific
- InfantryClass: 0x6C0 = InfantryTypeClass*, 0x6C4 = -1 sentinel
- AircraftClass: 0x6C0 = secondary vtable, 0x6C4 = AircraftTypeClass*

The convoy fields (next_in_convoy at 0x6C8, convoy_data at 0x6CC, is_convoy_follower at
0x6D0, etc.) are in **UnitClass**, not FootClass. FootClass__Save_Convoy_State (0x744640)
operates on UnitClass instances.

FootClass convoy-related fields that ARE in FootClass:
- 0x578: FormationSpeed
- 0x5D8: TeamNextMember (linked list)
- 0x687: deferred arrival vtable+0x174 hook flag in the OnArrival slice; stock Unit/Infantry resolve that hook to Scatter
- 0x688: ConvoyDisbanded
- 0x689: ConvoyArrived

## 9. Addendum — Second Investigation Pass

### 9.1 Field 0x5D4 RESOLVED: TeamClass* (verified)

**Evidence:** `FUN_006EA500` (TeamClass::Add_Member) writes `piVar2[0x175] = param_1`
where param_1 is the TeamClass* and piVar2 is the FootClass* being added.

`FUN_006EA870` (TeamClass::Remove_Member) clears `param_2[0x175] = 0` when removing.

**Add_Member (0x6EA500) full sequence:**
1. Validate candidate via `FUN_006EA610`
2. If unit already in a team (`[0x175] != 0`): remove from old team first via `FUN_006EA870`
3. Set ConvoyArrived flag: `byte 0x689 = (first_member == 0)` — first member auto-arrives
4. Link into member list: `[0x176] = old_first`; `team+0x54 = this` (prepend to linked list)
5. **Set team pointer: `[0x175] = team_ptr`** — byte 0x5D4 = TeamClass*
6. Set movement speed from team: `[0x85] = FUN_006f1870()`
7. Increment team member count: `team+0x48 += 1`
8. Add weapon power to team: `team+0x4C += GetWeaponDamage()`
9. Copy veterancy flag: `byte 0x422 = team_type+0xF5`

**Remove_Member (0x6EA870) clears on the FootClass being removed:**
- `[0x176] = 0` (TeamNextMember)
- `[0x175] = 0` (Team*)
- `[0x2C] = -1` (queued mission → none)
- `[0x16A] = 0` (SuspendedNavCom)
- `[0xAE] = 0` (some TechnoClass field)
- `byte 0x6B8 = 1` (JustRemovedFromTeam flag — set at START of function)
- If alive, not in limbo, and not falling: calls `vtable+0x484(0,1)` to clear destination

**Evaluate_Target_Threat check on this field:**
`*(char*)(*(int*)(team+0x24) + 0xF6)` reads TeamTypeClass+0xF6 — a flag controlling
whether team members can independently acquire targets. If the flag is 0, the unit
returns threat=0 (cannot target independently while in this team).

### 9.2 Field 0x694 PARTIALLY RESOLVED: Unknown large-object pointer

**NOT a TeamClass\*.** The AI function accesses it as:
```asm
MOV ESI, [ESI+0x694]      ; P = this->field_694
MOV ECX, [ESI+0x69c]      ; Q = *(P + 0x69C)
MOV EDX, [ECX]             ; vtable = Q->vtable
CALL [EDX+0x5C]            ; Q->AI()  (vtable+0x5C = AbstractClass::AI)
```

Since TeamClass is only 0xA0 bytes, `P + 0x69C` would be past its end.
Field 0x694 points to a **large object** (> 0x69C bytes). At offset 0x69C within
that object, there's a pointer to an AbstractClass-derived object whose AI() method
is called every tick. **Not cleared by remove-from-team** (0x6EA870 doesn't touch it).

Needs further binary tracing to determine what writes to byte offset 0x694.

### 9.3 TarCom (Target Computer) System — 0x5C4-0x5D1 RESOLVED

**Evidence:** FootClass::Assign_Target_Command (0x4DF0E0) and
FootClass::Clear_All_TarCom (0x4DF1A0).

| Offset | Size | Field | Evidence |
|--------|------|-------|----------|
| 0x5C4 | 4 | TarCom_CommandType | Clear_All sets to -1. Assign_Target sets to 0x1D (attack). |
| 0x5C8 | 4 | TarCom_PrimaryTarget | Set on forced-fire. Gate in Arrival_Target_Handler. |
| 0x5CC | 4 | TarCom_FollowTarget | Set on normal-attack. Checked against CanFire on arrival. |
| 0x5D0 | 1 | (padding) | |
| 0x5D1 | 1 | TarCom_IsFollowing | Cleared by Assign and Clear. Set to 1 on engagement. |

### 9.4 Locomotion_AI (0x520F40) — Infantry Animation Dispatch

Despite the `FootClass__Locomotion_AI` label, this is in the InfantryClass address range
and handles **infantry animation state selection** based on locomotor state and formation speed.

Key finding: The double at bytes **0x578-0x57F is FormationSpeed**, compared against a
global threshold to select between infantry walk (anim 0x17) and run (anim 0x18) animations.
The double at **0x580-0x587 is SpeedMultiplier** (initialized to 1.0), separate from FormationSpeed.

### 9.5 FootClass::ReceiveDamage (0x4D7330)

Extends TechnoClass::ReceiveDamage:
1. **Temporal warhead detach:** If warhead has temporal flag and unit is under temporal warp: detach
2. **Damage redirect:** If under temporal warp and damage exceeds TypeClass+0xD6C: redirect excess×2
3. **Team damage notify:** If in a team, call team's damage handler (FUN_006EB380)
4. **Retaliation:** If attacker exists and mission timer in right state: call vtable+0x484(0,1)

### 9.6 FootClass::ReceiveEMP (0x4DEBB0)

1. Only affects units with GetHeight > 0
2. Sets IsFalling (0x425) = 1
3. Disconnects radio (vtable+0x274 arg 3)
4. Propagates EMP to passengers
5. Randomizes EMP spin: random angle at [0xCC]/[0xCD] (bytes 0x330/0x334)

### 9.7 Harvester Pathfinding

**Scan_For_Tiberium (0x4DD0A0):** Expanding diamond search, picks highest-value ore cell.
Greedy — returns first ring that has any ore. Checks 4 rotational symmetries per ring offset.

**Is_Cell_Harvestable (0x4DCE80):** Zone-reachable + overlay type 5 (ore) + passable.

**Is_Cell_Weedable (0x4DD9F0):** Zone-reachable + overlay type 0xB (weed) + value > 0x2F (47).

### 9.8 Corrections to Section 2

- **Field 0x5D4:** Renamed from "TeamLeaderOrPtr" → **"Team\* (TeamClass pointer)"**. VERIFIED.
- **Field 0x6B8:** Renamed from "unknown_6B8" → **"JustRemovedFromTeam"**. Set by Remove_Member.
- **0x578-0x57F:** Confirmed as **FormationSpeed double** (not dword). Controls infantry walk/run anim.
- **0x580-0x587:** Confirmed as **SpeedMultiplier double** (initialized 1.0). Separate from above.

## 10. Remaining Open Questions

1. **Field 0x694:** Points to large object (>0x69C bytes) with sub-AI at +0x69C. Identity unknown.
2. ~~**Fields 0x544-0x557:**~~ **RESOLVED** — 16-byte SafePointerHandle (looping sound) at 0x544-0x553 + 4 bytes padding at 0x554-0x557.
3. **Fields 0x524-0x52A (4 words):** 16-bit values initialized to 0. Purpose unknown.
4. **Timer semantics:** CDTimerClass countdown vs elapsed direction unverified.
5. **PathHeadIndex format (0x5E0):** Direction encoding via Path_walk_directions_to_cell.
6. **[RESOLVED 2026-05-27] Byte 0x6AC:** one-shot `skip_head_to_coord_once`; `TechnoClass::Set_Destination` chrono/teleporter preprocessing writes it, and `Set_Destination_Internal` clears it after writing NavCom while skipping locomotor `Head_To_Coord` once.
7. **TeamTypeClass+0xF6:** What INI key maps to this independent-targeting flag?

## Sources

### Ghidra addresses decompiled:
**Pass 1:**
- 0x4D31E0 — FootClass::Constructor (full)
- 0x4D3540 — FootClass::Constructor (minimal/load)
- 0x4D3810 — FootClass::CanReachDestination
- 0x4D3920 — FootClass::Find_Path (238 lines)
- 0x4D4280 — FootClass::Mission_Hunt (307 lines, first 200)
- 0x4D5070 — FootClass::Mission_Guard (115 lines)
- 0x4D55F0 — FootClass::Head_To_Coord_Dispatch
- 0x4D5690 — FootClass::Greatest_Threat_Scan (first 200 of 736)
- 0x4D6AA0 — FootClass::Mission_Harvest (first 150 of 230)
- 0x4D8F40 — FootClass::Set_NavCom_With_Suspend
- 0x4D8FB0 — FootClass::Receive_Radio
- 0x4D94B0 — FootClass::Set_Destination_Internal
- 0x4D97A0 — FootClass::Evaluate_Target_Threat
- 0x4DA0E0 — FootClass::Enter_Destination
- 0x4DA530 — FootClass::AI (374 lines, full)
- 0x4DB1A0 — FootClass::GetCurrentSpeed
- 0x4DBDA0 — FootClass::IsCloakable
- 0x4CBBA0 — FootClass::Run_AStar
- 0x4DF040 — FootClass::Find_Docking_Bay
- 0x4DF0D0 — FootClass::Stop_Moving
- 0x4DF3A0 — FootClass::Arrival_Target_Handler
- 0x4DFCB0 — FootClass::Find_Nearest_Dock
- 0x517A50 — InfantryClass::Constructor (boundary check)
- 0x7353C0 — UnitClass::Constructor (boundary check)
- 0x413D20 — AircraftClass::Constructor (boundary check)
- 0x70D7E0 — FootClass::TryEnterTransport
- 0x744640 — FootClass::Save_Convoy_State
- 0x65AD30 — FootClass::GetDestination

**Pass 2:**
- 0x4D7330 — FootClass::ReceiveDamage
- 0x4DEBB0 — FootClass::ReceiveEMP
- 0x4DA4E0 — FootClass::GetVisualState
- 0x4DF0E0 — FootClass::Assign_Target_Command
- 0x4DF1A0 — FootClass::Clear_All_TarCom
- 0x520F40 — FootClass::Locomotion_AI (infantry animation dispatch)
- 0x4DCF80 — FootClass::Search_For_Tiberium_And_Move
- 0x4DD0A0 — FootClass::Scan_For_Tiberium
- 0x4DCE80 — FootClass::Is_Cell_Harvestable
- 0x4DD9F0 — FootClass::Is_Cell_Weedable
- 0x4DDB90 — FootClass::Search_For_Tiberium_Short_And_Move
- 0x5F5FA0 — FootClass::Set_Height_On_Bridge
- 0x6E8A90 — TeamClass::Constructor
- 0x6E9050 — TeamClass::Set_Convoy_Target
- 0x6E9380 — TeamClass::Script_Dispatch (538 lines, partial)
- 0x6EA500 — TeamClass::Add_Member (KEY: writes 0x5D4)
- 0x6EA870 — TeamClass::Remove_Member (KEY: clears 0x5D4)
- 0x6EB490 — TeamClass::Convoy_Move_With_Target

### Prior documents referenced:
- FOOTCLASS_AI_GHIDRA_REPORT.md — AI subsystem analysis
- CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md — convoy fields (CORRECTION: some fields are UnitClass, not FootClass)
- DRIVE_LOCOMOTION_CLASS.md — ILocomotion interface
- PATHFINDERCLASS_GHIDRA_REPORT.md — A* pathfinding
- SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md — scatter mechanics
- TECHNOCLASS_STRUCT_LAYOUT.md — parent class layout

### INI files checked:
- ini/rulesmd.ini — Speed, Locomotor, MovementZone, ROT, CloakStop, etc.
- ini/rules.ini — base RA2 values
