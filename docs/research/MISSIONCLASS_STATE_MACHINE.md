# MissionClass State Machine — Ghidra Report

**Date:** 2026-04-06
**Confidence:** HIGH (all offsets verified from binary disassembly and decompilation)

## Inheritance Chain

```
AbstractClass        (0x00 - 0x23)   vtable at various
  -> ObjectClass     (0x24 - 0xAB)   vtable at 0x7F2B18 (example)
    -> MissionClass  (0xAC - 0xD3)   vtable at 0x7EDCC0
      -> RadioClass  (0xD4 - 0xF3)   vtable at 0x7F0508
        -> TechnoClass ...
```

Constructor chain:
- `AbstractClass::Constructor` @ `0x00410170`
- `ObjectClass::Constructor` @ `0x005F3900` (called by MissionClass ctor)
- `MissionClass::Constructor` @ `0x005B2DA0` (called by RadioClass ctor)
- `RadioClass::Constructor` @ `0x0065A750` (called by TechnoClass ctor)
- `TechnoClass::Constructor` @ `0x006F2B40`

---

## MissionClass Struct Layout

All offsets are **byte offsets** from the start of the object (`this` pointer).
MissionClass adds fields at **0xAC - 0xD0** (36 bytes of new state on top of ObjectClass).

| Byte Offset | Size | Field Name          | Init Value | Description |
|-------------|------|---------------------|------------|-------------|
| 0xAC        | 4    | CurrentMission      | -1 (NONE)  | Active mission enum value |
| 0xB0        | 4    | SuspendedMission    | -1 (NONE)  | Mission saved by Override_Mission, restored by Restore_Mission |
| 0xB4        | 4    | QueuedMission       | -1 (NONE)  | Next mission to commence (set by Queue_Mission) |
| 0xB8        | 1    | IsCommenced         | 0          | Reset to 0 on every mission change; purpose unclear but checked in Queue_Mission |
| 0xBC        | 4    | MissionState        | 0          | Sub-state within a mission handler (0=init, 1+, handler-specific) |
| 0xC0        | 4    | MissionTimer_Start  | 0          | Frame counter snapshot for MissionTimer (CDTimer pattern) |
| 0xC4        | 4    | MissionTimer_??     | 0          | Saved with MissionTimer |
| 0xC8        | 4    | DispatchTimer_Start | g_CurrentFrameCounter | Frame counter snapshot for dispatch rate timer |
| 0xCC        | 4    | DispatchTimer_??    | (uninitialized) | Saved with DispatchTimer |
| 0xD0        | 4    | DispatchTimer_Rate  | 0          | Frames until next mission handler call; set by handler return value |

**Notes on param_1 type:** The constructor and most MissionClass methods receive `param_1`
as `int *` (pointer to int). Thus array indexing like `param_1[0x2b]` means byte offset
`0x2b * 4 = 0xAC`. The dispatch function also uses `int *`. Always multiply by 4.

---

## Mission Enum

32 values (indices 0-31) plus -1 for NONE. Stored in `g_MissionNameTable` at `0x00816CAC`
(array of 32 `char *` pointers). Table ends at `0x00816D2C`.

Helper functions:
- `Mission_From_Name(char *)` @ `0x005B3910` — linear search, returns index or -1
- `Mission_Name(int)` @ `0x005B3950` — returns string pointer, or "None" for -1

| Value | Name               | VTable Offset | Handler (base class) |
|-------|--------------------|---------------|----------------------|
| -1    | None               | —             | — |
| 0     | Sleep              | 0x204         | Mission_Sleep |
| 1     | Attack             | 0x210         | Mission_Attack |
| 2     | Move               | 0x22C         | Mission_Move |
| 3     | QMove              | 0x204 (same as Sleep) | Mission_Sleep (default fallback) |
| 4     | Retreat            | 0x230         | Mission_Retreat |
| 5     | Guard              | 0x21C         | Mission_Guard |
| 6     | Sticky             | 0x21C         | Mission_Guard (same handler) |
| 7     | Enter              | 0x240         | Mission_Enter |
| 8     | Capture            | 0x214         | Mission_Capture |
| 9     | Eaten              | 0x218         | Mission_Eaten |
| 10    | Harvest            | 0x224         | Mission_Harvest |
| 11    | Area Guard         | 0x220         | Mission_Area_Guard |
| 12    | Return             | 0x234         | Mission_Return |
| 13    | Stop               | 0x238         | Mission_Stop |
| 14    | Ambush             | 0x20C         | Mission_Ambush |
| 15    | Hunt               | 0x228         | Mission_Hunt |
| 16    | Unload             | 0x23C         | Mission_Unload |
| 17    | Sabotage           | 0x214         | Mission_Capture (same as Capture!) |
| 18    | Construction       | 0x244         | Mission_Construction |
| 19    | Selling            | 0x248         | Mission_Selling |
| 20    | Repair             | 0x24C         | Mission_Repair |
| 21    | Rescue             | 0x258 (600)   | Mission_Rescue |
| 22    | Missile            | 0x250         | Mission_Missile |
| 23    | Harmless           | 0x208         | Mission_Harmless |
| 24    | Open               | 0x254         | Mission_Open |
| 25    | Patrol             | 0x25C         | Mission_Patrol |
| 26    | Paradrop Approach   | 0x260         | Mission_ParadropApproach |
| 27    | Paradrop Overfly    | 0x264         | Mission_ParadropOverfly |
| 28    | Wait               | 0x268         | Mission_Wait |
| 29    | Attack Move        | — (no case)   | NOT DISPATCHED (case 0x1D missing from switch) |
| 30    | Spyplane Approach   | 0x26C         | Mission_SpyplaneApproach |
| 31    | Spyplane Overfly    | 0x270         | Mission_SpyplaneOverfly |

**Important:** Mission 3 (QMove) and the default case both dispatch to vtable+0x204
(Mission_Sleep). Mission 29 (Attack Move, 0x1D) has NO case in the switch statement
and falls through to the end without dispatching.

**In base MissionClass**, ALL mission handler vtable slots (0x204-0x270) point to the same
stub function at `0x005B2E10` which simply returns `0x1C2` (450 frames = 30 seconds at
15 fps). Derived classes override individual slots with real implementations.

---

## Dispatch Mechanism (Mission_AI)

**Function:** `MissionClass::Mission_Dispatch` @ `0x005B3060`
**VTable offset:** 0x05C

### Per-Tick Flow

```
Mission_Dispatch(this):
    1. Call ObjectClass::AI()  (base class tick)

    2. If !this->IsActive (byte at ObjectClass+0x90 == 0):
         return   // dead objects don't process missions

    3. Timer check:
       - Read DispatchTimer_Rate (0xD0) and DispatchTimer_Start (0xC8)
       - If DispatchTimer_Start != -1:
           elapsed = g_CurrentFrameCounter - DispatchTimer_Start
           if elapsed >= DispatchTimer_Rate:
               goto dispatch  // timer expired
           remaining = DispatchTimer_Rate - elapsed
       - If remaining != 0:
           return   // still waiting

    4. If this->Health (ObjectClass+0x6C) <= 0:
         return   // don't dispatch for dead objects

    5. Switch on CurrentMission (0xAC):
         case 0..28, 30, 31 (excluding 3, 29):
             call vtable[mission_vtable_offset]()
         case 3 (QMove):
             call vtable[0x204]()  // same as Sleep
         default:
             call vtable[0x204]()  // fallback to Sleep

    6. After handler returns:
         DispatchTimer_Start = g_CurrentFrameCounter
         DispatchTimer_Rate = handler_return_value
         // The return value is the number of frames before the handler
         // will be called again. This is the "mission rate".
```

### Return Value Semantics

Each mission handler returns an `int` representing the **number of game frames** before
the handler should be called again. Common values:

- `0` — call again immediately next tick
- `1` — call again next frame
- `0x0F` (15) — call again in 1 second (at 15 fps)
- `0x1C2` (450) — default stub value, 30 seconds
- Other values — handler-specific timing

---

## Key Virtual Methods

All vtable offsets below are byte offsets from the primary vtable pointer.

### Queue_Mission — vtable+0x1E8

**Base implementation:** `MissionClass::Queue_Mission` @ `0x005B35E0`
**Signature:** `void Queue_Mission(int mission, bool commence_now)`

```
Queue_Mission(this, mission, commence_now):
    // Guard: don't interrupt Repair/Produce (0x1C) with Guard (5)
    if CurrentMission == 0x1C && mission == 5:
        return
    // Guard: don't interrupt Selling (0x13)
    if CurrentMission == 0x13:
        return

    if mission != -1:
        // Only queue if different from current OR if queued != mission
        if CurrentMission != mission || (QueuedMission != mission && QueuedMission != -1):
            QueuedMission = mission
            IsCommenced = 0

    if commence_now:
        if this->ReadyToCommence():    // vtable+0x200
            this->Commence()           // vtable+0x1EC
```

### Commence — vtable+0x1EC

**Base implementation:** `MissionClass::Commence` @ `0x005B3570`

```
Commence(this):
    if QueuedMission == -1:
        return false

    CurrentMission = QueuedMission
    QueuedMission = -1
    MissionState = 0                    // reset sub-state
    MissionTimer_Start = g_CurrentFrameCounter
    MissionTimer = 0
    DispatchTimer_Start = g_CurrentFrameCounter
    DispatchTimer_Rate = 0              // dispatch immediately
    IsCommenced = 0

    return true
```

The key insight: **Commence moves QueuedMission into CurrentMission**, resets all timers
and state, and causes the new mission handler to be dispatched on the very next tick.

### Assign_Mission — vtable+0x1F0

**Base implementation:** `MissionClass::Assign_Mission` @ `0x005B2FD0`
**Signature:** `void Assign_Mission(int mission)`

```
Assign_Mission(this, mission):
    // Guard: don't overwrite Repair/Produce with Guard
    if CurrentMission == 0x1C && mission == 5:
        return

    CurrentMission = mission
    QueuedMission = -1          // clear queue
    IsCommenced = 0
    MissionState = 0            // reset sub-state
    MissionTimer_Start = g_CurrentFrameCounter
    MissionTimer = 0
    DispatchTimer_Start = g_CurrentFrameCounter
    DispatchTimer_Rate = 0      // dispatch immediately
```

**Key difference from Queue_Mission:** Assign_Mission sets CurrentMission **directly**
and immediately, bypassing the queue/commence cycle. It also fully resets all timers.

### Override_Mission — vtable+0x1F4

**Base implementation:** `MissionClass::Override_Mission` @ `0x005B3650`
**Signature:** `void Override_Mission(int mission)`

```
Override_Mission(this, mission):
    // Same guard as Assign: don't break Repair/Produce or Selling
    if CurrentMission == 0x1C && mission == 5:
        return
    if CurrentMission == 0x13:
        return

    // Save the current or queued mission before overriding
    if QueuedMission != -1:
        CurrentMission = mission
        SuspendedMission = QueuedMission
    else:
        SuspendedMission = CurrentMission
        CurrentMission = mission

    IsCommenced = 0
```

**Purpose:** Temporarily interrupts the current mission. The original mission is saved
in SuspendedMission and can be restored later via Restore_Mission.

### Restore_Mission — vtable+0x1F8

**Base implementation:** `MissionClass::Restore_Mission` @ `0x005B36B0`

```
Restore_Mission(this):
    if SuspendedMission == -1:
        return false

    CurrentMission = SuspendedMission
    SuspendedMission = -1
    IsCommenced = 0
    return true
```

### GetCurrentMission — vtable+0x184

**Base implementation:** `MissionClass::GetCurrentMission` @ `0x005B3040`

```
GetCurrentMission(this):
    if CurrentMission != -1:
        return CurrentMission
    return QueuedMission
```

<!-- corrected 2026-05-28: was "if QueuedMission != -1: return QueuedMission; else return CurrentMission"
     and "Returns the queued mission if one is pending, otherwise the current mission".
     Binary (decompile_function 0x005B3040) shows: reads CurrentMission first, returns it if != -1,
     else falls back to QueuedMission. Priority is inverted from what the doc claimed.
     ROOT_CAUSE: INFERENCE_HARDENED — the original description assumed QueuedMission was the
     "intended" mission and would take priority; the actual binary prefers CurrentMission. -->
Returns the current mission if one is active (not -1), otherwise falls back to QueuedMission.
This means code calling GetCurrentMission sees the *actively executing* mission first; it only
reports QueuedMission when no current mission is set.

### Is_Mission_Suspended — vtable+0x1FC

**Base implementation:** `MissionClass::Is_Mission_Suspended` @ `0x005B3A10`

```
Is_Mission_Suspended(this):
    return SuspendedMission != -1
```

### ReadyToCommence — vtable+0x200

Called by Queue_Mission to check if the unit is ready to transition. Subclasses
override this. In the base MissionClass vtable, this slot points to `0x004E0140`.

---

## Mission Timer (MissionState CDTimer)

MissionClass uses two frame-based countdown timers following the CDTimer pattern
(snapshot of `g_CurrentFrameCounter` at start, plus a duration).

### Timer 1: MissionTimer (offsets 0xC0-0xC4)
- `0xC0` (MissionTimer_Start): `g_CurrentFrameCounter` snapshot
- `0xC4`: Additional timer data
- Used by individual mission handlers for sub-state timing
- Reset to 0 on Commence/Assign_Mission

### Timer 2: DispatchTimer (offsets 0xC8-0xD0)
- `0xC8` (DispatchTimer_Start): `g_CurrentFrameCounter` snapshot
- `0xCC`: Additional data (saved/loaded)
- `0xD0` (DispatchTimer_Rate): Frames until next dispatch
- Set by the return value of mission handlers
- Controls how often Mission_Dispatch actually calls the handler

### MissionState (offset 0xBC)
- Sub-state counter within a mission handler
- Reset to 0 by Commence and Assign_Mission
- Individual mission handlers use this for multi-step state machines
  (e.g., 0=init, 1=approaching, 2=active)

### GetMissionTimerEntry (offset into global table)

**Function:** `MissionClass::GetMissionTimerEntry` @ `0x005B3A00`

```
GetMissionTimerEntry(this):
    return &g_MissionTimerTable[CurrentMission]
    // g_MissionTimerTable at 0x00A8E3A8, 8 bytes per entry
```

This is a global table of 32 entries (one per mission type), each 8 bytes.
Used for INI-configurable per-mission timing (Rate/AARate values read by
MissionClass::Read_INI).

---

## MissionClass::Read_INI — INI-Configured Mission Properties

**Function:** `MissionClass::Read_INI` @ `0x005B3760`

Each mission type has an INI section (named after the mission, e.g., `[Guard]`, `[Attack]`)
with the following keys:

| INI Key      | Type   | Description |
|-------------|--------|-------------|
| NoThreat    | Bool   | Unit ignores threats during this mission |
| Zombie      | Bool   | Unit cannot be given orders |
| Recruitable | Bool   | Unit can be recruited by AI |
| Paralyzed   | Bool   | Unit is paralyzed (no movement) |
| Retaliate   | Bool   | Unit retaliates when attacked |
| Scatter     | Bool   | Unit scatters from threats |
| Rate        | Double | Base execution rate (seconds) |
| AARate      | Double | Anti-air rate (defaults to Rate if 0) |

These are stored in 8-byte entries in the global `g_MissionTimerTable` at `0x00A8E3A8`.

---

## State Machine Transitions

```
                    Queue_Mission(M, true)
                    +---------+
                    |         v
    [NONE] --Assign_Mission(M)--> [CURRENT = M, STATE = 0]
         \                              |
          \                     handler returns rate
           \                            |
            \                   [wait 'rate' frames]
             \                          |
              \                 [dispatch handler again]
               \
                +--Queue_Mission(M, false)--> [QUEUED = M]
                                                  |
                                          ReadyToCommence()?
                                              yes |
                                          Commence()
                                                  |
                                          [CURRENT = M, QUEUED = -1, STATE = 0]

    Override_Mission(M):
        [CURRENT = old] --> [SUSPENDED = old, CURRENT = M]

    Restore_Mission():
        [SUSPENDED = old] --> [CURRENT = old, SUSPENDED = -1]
```

### Mission Change Guards

Two missions are specially protected from interruption:

1. **Mission 0x1C (Wait/Repair+Produce)** — Cannot be overridden by Guard (mission 5).
   Both Assign_Mission and Queue_Mission check for this.

2. **Mission 0x13 (Selling)** — Cannot be interrupted at all by Queue_Mission or
   Override_Mission.

---

## Notable Derived Class Overrides

### AircraftClass

- **Override_Mission** @ `0x0041B870`: Clears `field_0x6D2` unless current mission is
  SpyplaneApproach (0x1E), then calls base `MissionClass::Commence`.

- **Assign_Mission** @ `0x0041B9F0`: Blocks assignment when aircraft is in flight missions
  (Retreat=4, ParadropApproach=0x1A, ParadropOverfly=0x1B, SpyplaneApproach=0x1E,
  SpyplaneOverfly=0x1F) AND has no passenger (`field_0x294 == 0`), unless the new mission
  is also one of those flight missions.

- **Queue_Mission_Override** @ `0x0041BA90`: Calls base Queue_Mission then additional logic.

### Known Mission Handler Overrides (non-exhaustive)

| Class         | Mission       | Address    |
|---------------|---------------|------------|
| AircraftClass | Mission_Attack | 0x00417FE0 |
| AircraftClass | Mission_Guard  | 0x0041A5C0 |
| AircraftClass | Mission_Hunt   | 0x004151E0 |
| AircraftClass | Mission_Move   | 0x004166C0 |
| AircraftClass | Mission_Open   | 0x004158E0 |
| AircraftClass | Mission_QMove  | 0x00415A50 |
| AircraftClass | Mission_Rescue | 0x00415960 |
| AircraftClass | Mission_Sticky | 0x00419C80 |
| AircraftClass | Mission_SpyPlane | 0x00417300 |
| AircraftClass | Mission_ParaDropApproach | 0x004155F0 |
| AircraftClass | Mission_ParaDropOverfly  | 0x004157C0 |
| AircraftClass | Mission_Move_Carryall    | 0x00416D50 |
| BuildingClass | Mission_Attack | 0x0044ACF0 |
| BuildingClass | Mission_Missile | 0x0044C980 |
| FootClass     | Mission_Guard  | 0x004D5070 |
| FootClass     | Mission_Harvest | 0x004D6AA0 |
| FootClass     | Mission_Hunt   | 0x004D4280 |
| InfantryClass | Mission_Capture | 0x005202F0 |
| InfantryClass | Mission_Enter  | 0x005196A0 |
| UnitClass     | Mission_Deploy | 0x006AFD60 |
| UnitClass     | Mission_Deploy_Building | 0x0073D630 |
| UnitClass     | Mission_Enter  | 0x00739EC0 |
| UnitClass     | Mission_Harvest | 0x00737C90 / 0x0073E5E0 |
| UnitClass     | Mission_Guard_Harvester | 0x00740810 |
| UnitClass     | Mission_Unload | 0x004DDF90 |

---

## MissionClass Vtable Layout (at 0x7EDCC0)

Key method slots relevant to MissionClass (inherits ObjectClass slots before these):

| VTable Offset | Method | Base Address |
|---------------|--------|--------------|
| 0x020 | Constructor (Load) | 0x005B3A60 |
| 0x034 | Save | 0x005B3970 |
| 0x05C | Mission_Dispatch (AI) | 0x005B3060 |
| 0x114 | (unknown, in MissionClass range) | 0x005B3A50 |
| 0x184 | GetCurrentMission | 0x005B3040 |
| 0x188 | (unknown) | 0x0041BE90 |
| 0x1E8 | Queue_Mission | 0x005B35E0 |
| 0x1EC | Commence | 0x005B3570 |
| 0x1F0 | Assign_Mission | 0x005B2FD0 |
| 0x1F4 | Override_Mission | 0x005B3650 |
| 0x1F8 | Restore_Mission | 0x005B36B0 |
| 0x1FC | Is_Mission_Suspended | 0x005B3A10 |
| 0x200 | ReadyToCommence | 0x004E0140 |
| 0x204 | Mission_Sleep | 0x005B2E10 (stub: returns 450) |
| 0x208 | Mission_Harmless | 0x005B2E20 (stub) |
| 0x20C | Mission_Ambush | 0x005B2E30 (stub) |
| 0x210 | Mission_Attack | 0x005B2E40 (stub) |
| 0x214 | Mission_Capture | 0x005B2E50 (stub) |
| 0x218 | Mission_Eaten | 0x005B2E60 (stub) |
| 0x21C | Mission_Guard | 0x005B2E70 (stub) |
| 0x220 | Mission_Area_Guard | 0x005B2E80 (stub) |
| 0x224 | Mission_Harvest | 0x005B2E90 (stub) |
| 0x228 | Mission_Hunt | 0x005B2EA0 (stub) |
| 0x22C | Mission_Move | 0x005B2EB0 (stub) |
| 0x230 | Mission_Retreat | 0x005B2EC0 (stub) |
| 0x234 | Mission_Return | 0x005B2ED0 (stub) |
| 0x238 | Mission_Stop | 0x005B2EE0 (stub) |
| 0x23C | Mission_Unload | 0x005B2EF0 (stub) |
| 0x240 | Mission_Enter | 0x005B2F00 (stub) |
| 0x244 | Mission_Construction | 0x005B2F10 (stub) |
| 0x248 | Mission_Selling | 0x005B2F20 (stub) |
| 0x24C | Mission_Repair | 0x005B2F30 (stub) |
| 0x250 | Mission_Missile | 0x005B2F40 (stub) |
| 0x254 | Mission_Open | 0x005B2F50 (stub) |
| 0x258 | Mission_Rescue | 0x005B2F60 (stub) |
| 0x25C | Mission_Patrol | 0x005B2F70 (stub) |
| 0x260 | Mission_ParadropApproach | 0x005B2F80 (stub) |
| 0x264 | Mission_ParadropOverfly | 0x005B2F90 (stub) |
| 0x268 | Mission_Wait | 0x005B2FA0 (stub) |
| 0x26C | Mission_SpyplaneApproach | 0x005B2FB0 (stub) |
| 0x270 | Mission_SpyplaneOverfly | 0x005B2FC0 (stub) |

All 28 stub handlers in base MissionClass are at `0x005B2E10`-`0x005B2FC0`, spaced
16 bytes apart, each containing `MOV EAX, 0x1C2; RET` (return 450 frames).

---

## Summary of Functions Labeled in Ghidra

| Address    | Name |
|------------|------|
| 0x005B2DA0 | MissionClass__Constructor |
| 0x005B2E10 | MissionClass__Mission_Default |
| 0x005B2FD0 | MissionClass__Assign_Mission |
| 0x005B3040 | MissionClass__GetCurrentMission |
| 0x005B3060 | MissionClass__Mission_Dispatch |
| 0x005B3570 | MissionClass__Commence |
| 0x005B35E0 | MissionClass__Queue_Mission |
| 0x005B3650 | MissionClass__Override_Mission |
| 0x005B36B0 | MissionClass__Restore_Mission |
| 0x005B3760 | MissionClass__Read_INI |
| 0x005B3910 | Mission_From_Name |
| 0x005B3950 | Mission_Name |
| 0x005B3970 | MissionClass__Save |
| 0x005B3A00 | MissionClass__GetMissionTimerEntry |
| 0x005B3A10 | MissionClass__Is_Mission_Suspended |
| 0x005B3A50 | MissionClass__Mission_Load_Notify |
| 0x005B3A60 | MissionClass__Constructor (Load variant) |

---

## Global Data

| Address    | Name | Description |
|------------|------|-------------|
| 0x00816CAC | g_MissionNameTable | Array of 32 `char *` pointers to mission name strings |
| 0x00A8E3A8 | g_MissionTimerTable | Array of 32 entries (8 bytes each), per-mission INI properties |
| 0x00A8ED84 | g_CurrentFrameCounter | Global frame counter used for all timer calculations |
