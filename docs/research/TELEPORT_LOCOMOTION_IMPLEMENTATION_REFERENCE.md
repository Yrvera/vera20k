# TeleportLocomotionClass -- Definitive Implementation Reference

## Purpose

This document consolidates ALL verified findings from 5 existing Ghidra reports into a
single authoritative implementation reference for the chrono miner teleport system. Every
offset, function address, and behavioral claim has been cross-checked against live Ghidra
decompilation of gamemd.exe (YR 1.001). Where prior reports conflict, this document notes
the correct answer and the evidence.

**Source reports consolidated:**
- CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md (v3)
- TELEPORT_LOCOMOTION_DEEP_DIVE.md
- CHRONO_WARP_VISUAL_RENDERING.md
- TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md
- CHRONO_MINER_SYSTEM_OVERVIEW.md

**Active in YR:** Yes. Used by Chrono Miner, Chrono Legionnaire, and any unit with
`Teleporter=yes` and `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}`.

---

## 1. TeleportLocomotionClass Struct Layout

**Constructor: 0x00718000** (param_1 type: `undefined4 *`, offsets are direct dword indices)

### Complete Field Map (verified from constructor + all function cross-refs)

| Offset | Size | Field | Init | Verified From |
|--------|------|-------|------|---------------|
| +0x00 | 4 | IUnknown vtable | 0x7F50CC | Constructor |
| +0x04 | 4 | ILocomotion vtable | 0x7F5000 | Constructor |
| +0x08 | 4 | Refcount | 0 | LocomotionClass base |
| +0x0C | 4 | LinkedTo (FootClass*) | 0 | LocomotionClass base |
| +0x10 | 1 | Powered | 1 | LocomotionClass base |
| +0x11 | 1 | field_11 | 1 | LocomotionClass base |
| +0x14 | 4 | field_14 | 0 | LocomotionClass base |
| +0x18 | 4 | IPiggyback vtable | 0x7F4FDC | Constructor |
| +0x1C | 4 | HeadToCoord.X | NullCoord | Constructor, HeadToCoord |
| +0x20 | 4 | HeadToCoord.Y | NullCoord | |
| +0x24 | 4 | HeadToCoord.Z | NullCoord | |
| +0x28 | 4 | DestCoord.X | NullCoord | Constructor, Process |
| +0x2C | 4 | DestCoord.Y | NullCoord | |
| +0x30 | 4 | DestCoord.Z | NullCoord | |
| +0x34 | 1 | IsMoving | 0 | Is_Moving, HeadToCoord |
| +0x35 | 1 | field_35 | 0 | Is_Ok_To_End |
| +0x36 | 1 | field_36 | 0 | Stop_Moving |
| +0x38 | 4 | WarpPhase | 0 | StateMachineTick |
| +0x3C | 4 | Timer.StartFrame | CurrentFrame | CDTimerClass |
| +0x40 | 4 | Timer.field_4 | (uninit) | CDTimerClass unused middle field |
| +0x44 | 4 | Timer.Duration | 0 | CDTimerClass |
| +0x48 | 4 | PiggybackedLocomotor | 0 | Begin_Piggyback, End_Piggyback |

**Total struct size: 0x4C (76 bytes). Confidence: 98%.**

### Calling Convention Note

When functions are called through the **ILocomotion vtable** (base+0x04), `this` points
to base+0x04. So `[this+0x08]` = base+0x0C = LinkedTo, `[this+0x30]` = base+0x34 = IsMoving.

When functions are called through the **IPiggyback vtable** (base+0x18), `this` points
to base+0x18. So `[this+0x30]` = base+0x48 = PiggybackedLocomotor.

### CDTimerClass Pattern

The timer at +0x3C/+0x40/+0x44 follows the standard CDTimerClass:
```
struct CDTimerClass {
    int StartFrame;   // +0x00: Frame when timer was set (-1 = invalid)
    int field_4;      // +0x04: UNUSED in countdown logic (vestigial)
    int Duration;     // +0x08: Duration in frames
};

// Remaining time check:
remaining = Duration;
if (StartFrame != -1) {
    elapsed = CurrentFrame - StartFrame;
    if (elapsed >= Duration) remaining = 0;
    else remaining = Duration - elapsed;
}
// Timer expired when remaining == 0
```

---

## 2. ILocomotion Vtable (0x7F5000) -- Key Methods

| Slot | Offset | Address | Method |
|------|--------|---------|--------|
| 0 | 0x00 | 0x71A160 | QueryInterface |
| 1 | 0x04 | 0x71A170 | AddRef |
| 2 | 0x08 | 0x71A180 | Release |
| 3 | 0x0C | 0x55A710 | Link_To_Object (base) |
| 4 | 0x10 | 0x718080 | **Is_Moving** |
| 5 | 0x14 | 0x7180A0 | **Destination** |
| 6 | 0x18 | 0x55ACA0 | Stop_Moving (base) |
| 13 | 0x34 | 0x55ABC0 | **Visual_Character** (base: always returns 0) |
| 16 | 0x40 | 0x7192F0 | **Process / StateMachineTick** |
| 17 | 0x44 | 0x718100 | **Head_To_Coord** |
| 18 | 0x48 | 0x718230 | **Stop_Moving / Clear_Coords** |

### Is_Moving (0x718080)

```asm
MOV EAX, [ESP+4]        ; EAX = ILocomotion this (base+4)
CMP byte [EAX+0x30], 1  ; check base+0x34 = IsMoving
SETZ AL
RET 4
```
Returns `true` if IsMoving (base+0x34) == 1. **Confidence: 99%.**

### Stop_Moving / Clear_Coords (0x718230)

```asm
MOV EAX, [ESP+4]
MOV [EAX+0x18], g_NullCoord_X   ; HeadToCoord = NullCoord (base+0x1C)
MOV [EAX+0x1C], g_NullCoord_Y
MOV [EAX+0x20], g_NullCoord_Z
MOV byte [EAX+0x30], 0          ; IsMoving = 0 (base+0x34)
MOV byte [EAX+0x32], 0          ; field_36 = 0 (base+0x36)
RET 4
```
**Confidence: 99%.**

---

## 3. HeadToCoord / Move_To (0x718100) -- ILocomotion Slot 17

Called by `FootClass::Set_Destination_Internal` (0x4D94B0) when TeleportLocomotionClass is
the active locomotor. This is how the teleport gets "armed".

**param_1 type:** ILocomotion interface pointer (base+0x04). Field offsets shift by -4.

### Logic

```
1. Guard: IsWarpingOut (vtable+0x37C) -> abort
2. Guard: IsWarpingIn (vtable+0x380)  -> abort
3. Guard: IsDeploying (vtable+0x1D4)  -> abort
4. Guard: IsUndeploying (vtable+0x1D8) -> abort
5. If techno has deployed flag AND is infantry: scatter at dest cell
6. Call Process(dest) to validate destination
7. If DestCoord == NullCoord: call SetOccupation(0,1), return (invalid)
8. Set IsMoving = 1 (base+0x34)
9. Copy DestCoord -> HeadToCoord
```

If any guard fires, the unit's NavCom target (+0x5A4) is cleared and function returns.

**IsMoving=1 at address 0x7181DB is the SOLE write** of IsMoving=1 in the entire binary
(verified by byte-pattern search). This is the ONLY way to trigger a warp.

**Confidence: 95%.**

---

## 4. Process (0x718B70) -- Destination Validation

Called from HeadToCoord to resolve and validate the warp destination. Returns 1 if valid.

**param_1 type:** TeleportLocomotionClass base pointer (NOT ILocomotion offset).

### Key Behaviors

**Non-infantry units (WhatAmI != 0xF), NOT ChronoInTransit:**
1. Basic passability check via `CanEnterCell(cell, -1, -1, 0, 1)`
2. Zone connectivity via `MapClass::GetZoneID` + `Pathfinding_validate_alternate`
3. Snap destination to cell center: `X = cellX * 256 + 128`, `Y = cellY * 256 + 128`
4. Set Z = ground height at destination

**Infantry (WhatAmI == 0xF):**
1. Check mission (7=Enter, 8=Capture, 9=Eaten, 25=Patrol) for dock check
2. Sub-cell placement via `CellClass::PlaceInfantryInCell` (0x480FA0)
3. If resulting cell blocked, set DestCoord = NullCoord (abort)

**ChronoInTransit path:** Use destination as-is (no pathfinding validation).

**Bridge detection:** Cell flags (+0x140) bit 0x100 = HasBridge. If set, Z comparison
determines if unit is above bridge level.

**Confidence: 85%** (complex function with heavy stack manipulation).

---

## 5. The State Machine -- StateMachineTick (0x7192F0)

**ILocomotion vtable slot 16 (offset 0x40).** Called every game tick.

`param_1` = ILocomotion interface pointer (base+0x04).
`techno` = LinkedTo FootClass* at param_1+0x08 (= base+0x0C).

### Pre-Phase Checks (before the switch)

```
Check 1: if (techno->BeingWarped && WarpPhase==0 && techno->PendingWarpPhase==0):
           → Call TimerCheck(), return. Unit is in post-warp cooldown.

Check 2: if (WarpPhase==0 && techno->PendingWarpPhase != 0):
           → WarpPhase = techno->PendingWarpPhase, return.
           (ChronoSphere sets PendingWarpPhase=3, locomotor picks it up here)

Check 3: if (techno->ChronoInTransit != 0 && WarpPhase==0):
           → Enter ChronoSphere path (phases 0-7 below)
```

### TWO DISTINCT PATHS

The state machine handles two completely different flows:

**Path A: Self-Teleport (Chrono Miner / Chrono Legionnaire)**
- Triggered when `ChronoInTransit == 0`, `WarpPhase == 0`, `Is_Moving() == true`
- Entire warp happens in a SINGLE TICK during Phase 0
- No multi-phase progression for the teleport itself
- Post-warp cooldown uses pre-phase check 1 (BeingWarped + TimerCheck)

**Path B: ChronoSphere Warp (externally initiated)**
- Triggered when `ChronoInTransit == 1` OR `PendingWarpPhase == 3`
- Uses full 8-phase state machine (phases 0-7)
- Multi-tick sequence with visual delays

---

## 6. Path A: Self-Teleport Warp Sequence (Phase 0)

This is the chrono miner's primary teleport. **Everything happens in one tick.**

### Exact Order of Operations (verified from 0x7192F0 decompilation)

```
 1. Check: Is_Moving() == true AND currentPos != destCoord AND destCoord != NullCoord
    If any false → call SetOccupation(1,0), TimerCheck, return (no warp)

 2. StopAllTargeting(techno)                    [0x70D4A0]
    Clear all units targeting/pursuing this one

 3. Detach all anim effects linked to this unit
    Iterate g_AnimArray, detach any anim whose OwnerTechno == techno

 4. Spawn WarpOut anim at DEPARTURE position
    AnimClass::Constructor(Rules+0x33C, &techno->Location, 0, 1, 0x600, 0, 0)

 5. Calculate 3D Euclidean distance (in leptons)
    dx = techno.X - dest.X
    dy = techno.Y - dest.Y
    dz = techno.Z - dest.Z
    distance = (int)sqrt((double)(dx*dx + dy*dy + dz*dz))

 6. Set timer:
    Timer.StartFrame = CurrentFrame
    Timer.Duration = 0  (default: no delay)

 7. If Rules->ChronoTrigger (Rules+0xBF8, bool):
    Timer.Duration = distance / Rules->ChronoDistanceFactor (Rules+0xBF4, int)

 8. Clamp to minimum:
    remaining = compute_remaining(Timer)  // effectively = Timer.Duration since just set
    if (remaining <= Rules->ChronoMinimumDelay (Rules+0xBFC)):
        Timer = { CurrentFrame, ?, ChronoMinimumDelay }

 9. Force minimum for short distances:
    if (distance < Rules->ChronoRangeMinimum (Rules+0xC00)):
        Timer = { CurrentFrame, ?, ChronoMinimumDelay }

10. Set BeingWarped flag:
    techno+0x271 = 1

11. SPECIAL: Harvester instant-warp check
    if (WhatAmI() == 1 AND UnitTypeClass(+0x6C4)->Harvester(+0xE0E) != 0):
        Timer.Duration = 0
        techno->BeingWarped = 0
    WhatAmI()==1 is CONFIRMED as UnitClass (verified: 0x746E20 returns 1).
    UnitTypeClass+0xE0E is CONFIRMED as the Harvester=yes flag (verified from
    CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT, HARVESTER_DOCK_UNLOAD, and
    HARVESTER_MISSION_HARVEST_GHIDRA_REPORT -- all independently confirm this).
    
    Effect: For chrono miners (Harvester=yes), the post-warp timer is zeroed
    and BeingWarped is cleared immediately. The unit appears at the destination
    with NO translucency and NO cooldown. The warp is truly instant.
    
    For Chrono Legionnaires (InfantryClass, WhatAmI=0xF, Harvester=no), this
    check does NOT fire. They get the full chrono delay timer with 50%
    translucency during the cooldown period.
    
    CORRECTION: The v3 report labeled +0xE0E as "ChronoKillInfantry on
    HouseTypeClass" -- this was WRONG. It is Harvester=yes on UnitTypeClass.
    Different struct at the same offset.

12. Detach flash anim if present:
    if (techno+0x694 != 0): WarpAttachClass::Detach()

13. Unmark from map:
    techno->vtable->Unmark(0)                      [vtable+0x124]

14. Play ChronoOutSound at departure:
    if (TypeClass->ChronoOutSound (+0x578) != -1 OR Rules->ChronoOutSound (+0x21C) != -1):
        VocClass::PlayAt(sound, 0, &techno->Location)

15. Set destination on techno:
    techno->vtable->SetDestination(destCoord)       [vtable+0x1B4]

16. Update bridge flag at destination:
    CellClass* destCell = MapClass::GetCellAt(destCoord)
    if (destCell->Flags & 0x100): techno->IsOnBridge (+0x8C) = 1
    else: techno->IsOnBridge = 0

17. Clear destination and mark at new position:
    techno->vtable->ClearDestination(0)             [vtable+0x1CC]
    techno->vtable->Mark(1)                         [vtable+0x124, param=1]

18. Play ChronoInSound at arrival:
    if (TypeClass->ChronoInSound (+0x574) != -1 OR Rules->ChronoInSound (+0x218) != -1):
        VocClass::PlayAt(sound, 0, &destCoord)

19. Set mission to GUARD_AREA (2):
    techno->vtable->SetMission(2)                   [vtable+0x18C]

20. Update locomotor layer:
    ILocomotion::UpdateLayer(this)                   [vtable+0x48]

21. Handle crate pickup at destination:
    FUN_00481A00(destCell, techno)

22. Update occupation:
    techno->vtable->SetOccupation(0, 1)             [vtable+0x480]

23. Spawn WarpOut anim at ARRIVAL position (SAME AnimType as departure!):
    AnimClass::Constructor(Rules+0x33C, &techno->Location, 0, 1, 0x600, 0, 0)

24. Clear PendingWarpPhase:
    techno+0x280 = 0
```

### Post-Warp Cooldown (subsequent ticks)

After phase 0 completes, `WarpPhase` remains 0 but `IsMoving` is 1 and `BeingWarped` is 1.
On subsequent ticks, the pre-phase check triggers:

```
if (BeingWarped && WarpPhase==0 && PendingWarpPhase==0):
    → TimerCheck()
```

TimerCheck (0x719BF0) counts down the timer. When it expires:
1. Clears BeingWarped (+0x271) = 0
2. If techno+0x2B4 == 0 (no pending action): calls idle behavior functions
3. Does NOT advance WarpPhase (stays at 0, but timer is done)

**The unit is now fully idle.** FootClass::AI detects Is_Ok_To_End==true and restores
the piggybacked locomotor if one exists.

**Confidence: 95%.**

---

## 7. Path B: ChronoSphere Warp (Phases 0-7)

Used when the ChronoSphere superweapon (0x65EC30) externally sets:
- ChronoInTransit (+0x27C) = 1
- PendingWarpPhase (+0x280) = 3
- ChronoDestCoords (+0x288/28C/290) = target position
- ChronoLockDuration (+0x284) = Rules->ChronoReinfDelay (+0xBF0)

### Phase Progression

| Phase | Name | Action |
|-------|------|--------|
| 0 | WARP_OUT_START | Set WarpingOut(+0x270)=1, Timer=60 frames, advance to 1 |
| 1 | WARP_OUT_WAIT | TimerCheck waits for 60-frame timer. When expired: advance to 2 |
| 2 | IN_TRANSIT_START | Spawn WarpOut anim, Unmark, play ChronoOutSound. Set BeingWarped=1, clear ChronoInTransit+WarpingOut+IsOnBridge. Read ChronoDestCoord(+0x288-290). Call Update_Position(dest, 0). Advance to 3 (or 4 if arrived) |
| 3 | IN_TRANSIT_CONTINUE | Call Update_Position(dest, 0) again. If arrived: advance to 4. Set ChronoLockDuration = Rules->ChronoDelay (+0xBEC) |
| 4 | WARP_IN_RELOCATE | Call Update_Position(dest, 1) with occupancy flag. SetDestination, ClearDestination, Mark(1). Advance to 5 |
| 5 | WARP_IN_COMPLETE | SetDestination, ClearDestination, Mark(1). Play ChronoInSound. Check playfield. Run PostWarpValidation (if PendingWarpPhase==0). If unit alive: SetMission(GUARD_AREA), clear +0x428/+0x42C, SetGhostCell(0), SetOccupation, set timer from ChronoLockDuration, spawn WarpOut anim. Advance to 6 |
| 6 | CHRONO_LOCK_WAIT | TimerCheck waits for lock timer. When expired: advance to 7 |
| 7 | WARP_DONE | Clear BeingWarped(+0x271), SetGhostCell(0), SetOccupation, clear IsMoving=0, clear PendingWarpPhase=0, reset WarpPhase=0 |

**Confidence: 95%.**

---

## 8. Timer System

### Chrono Delay Formula (Self-Teleport, Phase 0)

```
distance = (int)sqrt(dx^2 + dy^2 + dz^2)   // 3D Euclidean, in leptons

if ChronoTrigger:
    delay = distance / ChronoDistanceFactor
else:
    delay = 0

if delay <= ChronoMinimumDelay:
    delay = ChronoMinimumDelay

if distance < ChronoRangeMinimum:
    delay = ChronoMinimumDelay
```

### RulesClass Chrono Constants (verified from ReadGeneral assembly)

| Offset | INI Key | Type | Default | Purpose |
|--------|---------|------|---------|---------|
| +0xBEC | ChronoDelay | int | 30 | Post-warp lock for ChronoSphere (frames) |
| +0xBF0 | ChronoReinfDelay | int | 12 | Delay for ChronoSphere reinforcement |
| +0xBF4 | ChronoDistanceFactor | int | 48 | Distance divisor: delay = dist / factor |
| +0xBF8 | ChronoTrigger | bool | true | Enable distance-based delay calculation |
| +0xBFC | ChronoMinimumDelay | int | 16 | Minimum warp timer floor (frames) |
| +0xC00 | ChronoRangeMinimum | int | 25 | Below this distance, force minimum delay |

### Harvest-Specific Constants

| Offset | INI Key | Type | Default | Purpose |
|--------|---------|------|---------|---------|
| +0xD78 | HarvesterTooFarDistance | int | 5 | Regular harvester max drive range (cells) |
| +0xD7C | ChronoHarvTooFarDistance | int | 50 | Chrono miner max drive range (cells) |

Both are ONLY read in `UnitClass::Mission_Harvest` state 2 (0x73E5E0). The threshold
comparison is `distance_leptons <= RulesValue * 256`.

**Confidence: 98%.**

---

## 9. IPiggyback COM Interface

### Vtable: 0x7F4FDC (at TeleportLocomotionClass+0x18)

| Slot | Address | Method |
|------|---------|--------|
| 3 | 0x719E90 | Begin_Piggyback |
| 4 | 0x719EE0 | End_Piggyback |
| 5 | 0x719F30 | Is_Ok_To_End |
| 6 | 0x719F80 | Piggybacker_CLSID |
| 7 | 0x71A100 | Is_Piggybacking |

### Begin_Piggyback (0x719E90)

```c
HRESULT Begin_Piggyback(IPiggyback* this, ILocomotion* newLoco) {
    if (newLoco == NULL) return E_POINTER;     // 0x80004003
    if (this->PiggybackLoco != NULL)           // IPiggyback+0x30 = base+0x48
        return E_FAIL;                          // 0x80004005
    this->PiggybackLoco = newLoco;
    newLoco->AddRef();
    return S_OK;                                // 0
}
```

### End_Piggyback (0x719EE0)

```c
HRESULT End_Piggyback(IPiggyback* this, ILocomotion** ppOut) {
    if (ppOut == NULL) return E_POINTER;

    // Clear chrono source references on the TechnoClass
    FootClass* linked = this - 0x0C;  // IPiggyback-0x18+0x0C = base+0x0C
    if (linked != NULL) {
        linked->ChronoSourceBuilding (+0x428) = 0;
        linked->ChronoSourceHouse (+0x42C) = 0;
    }

    if (this->PiggybackLoco != NULL) {
        *ppOut = this->PiggybackLoco;
        this->PiggybackLoco = NULL;
        return S_OK;
    }
    return S_FALSE;  // 1
}
```

### Is_Ok_To_End (0x719F30) -- The 6 Conditions

Returns true only when ALL conditions are met:

```c
bool Is_Ok_To_End(IPiggyback* this) {
    // 1. Not currently moving (teleporting)
    if (Is_Moving()) return false;

    // 2. Must have a piggybacked locomotor
    if (PiggybackLoco == NULL) return false;

    // 3. field_35 (base+0x35) must be 0
    if (field_35 != 0) return false;

    // 4. TechnoClass ChronoInTransit (+0x27C) must be 0
    if (techno->ChronoInTransit != 0) return false;

    // 5. WarpPhase (base+0x38) must be 0
    if (WarpPhase != 0) return false;

    // 6. TechnoClass IsDeploying (+0x6AD) must be 0
    if (techno->field_6AD != 0) return false;

    return true;
}
```

### Is_Piggybacking (0x71A100)

```c
bool Is_Piggybacking(IPiggyback* this) {
    return (this->PiggybackLoco != 0);  // IPiggyback+0x30 = base+0x48
}
```

**Confidence: 98%** for all IPiggyback methods.

---

## 10. Teleport-vs-Drive Decision: TechnoClass::Set_Destination (0x741970)

### Decision Block at 0x7423CD

When `Set_Destination` is called on a unit with `Teleporter=yes` (TypeClass+0xCD4):

```
Preconditions (all must be true):
  - TypeClass+0xCD4 != 0  (Teleporter=yes)
  - techno+0x27C == 0     (not ChronoInTransit)
  - techno+0x2B0 == 0     (no suspended destination)
  - techno+0x6AD == 0     (not deploying)
```

Then:
```
Get current locomotor's CLSID via IPersistStream::GetClassID

IF NavTarget is a Building:
  Call CellClass::FindFirstBuilding(dest_cell, 0) at 0x47EBA0
    → Iterates cell's object list (FirstObject at +0xE4)
    → Returns first object where WhatAmI() == 1 (BuildingClass)

  IF FindFirstBuilding returns non-NULL (building on dest cell):
    → Piggyback DriveLocomotionClass over TeleportLoco
    → Unit DRIVES to destination

  IF FindFirstBuilding returns NULL (empty cell) AND CLSID == TeleportLoco:
    → SKIP piggyback
    → TeleportLocomotionClass remains active
    → Unit TELEPORTS to destination
```

### Assembly Verification (0x7424BD-0x7424FA)

```asm
007424D7: CALL CellClass__FindFirstBuilding
007424DC: TEST EAX, EAX
007424DE: JNZ  Drive_Piggyback_Path      ; building found -> drive
007424E4: MOV  ECX, 4                     ; no building:
007424E9: MOV  EDI, 0x7E9A90             ;   compare CLSID with TeleportLoco
007424F4: CMPSD.REPE                      ;   4-DWORD GUID comparison
007424FA: JZ   Skip_Piggyback_Path        ;   if match -> teleport!
```

### CellClass::FindFirstBuilding (0x47EBA0)

```c
int* FindFirstBuilding(CellClass* cell, char bridge_flag) {
    if (!g_GameActive) return NULL;
    ObjectClass* obj = bridge_flag ? cell->BridgeObjects (+0xE8) : cell->FirstObject (+0xE4);
    for (; obj != NULL; obj = obj->NextObject (+0x30)) {
        if (obj->WhatAmI() == 1) return obj;  // BuildingClass
    }
    return NULL;
}
```

### Why Building Presence Matters

The chrono miner must DRIVE the final approach to the refinery because docking requires
precise pathfinding to the dock pad. Teleporting directly onto a building cell would bypass
the docking protocol. When the destination cell contains a building (the refinery),
DriveLocomotionClass is piggybacked so the miner drives normally.

When the destination is an empty cell (e.g., the dock-adjacent cell computed by
Mission_Harvest, or a player-ordered move to open ground), teleporting is used.

**Confidence: 95%.**

---

## 11. Mission_Harvest State 2: The Distance Check (0x73E5E0)

### When the Chrono Miner Returns from Ore

```
1. Find nearest refinery: dock = Find_Docking_Bay(TypeClass->DockList, 0, 0)

2. Calculate 3D Euclidean distance (leptons):
   dist = sqrt(dx^2 + dy^2 + dz^2)

3. IF Teleporter AND dist <= ChronoHarvTooFarDistance * 256 (default: 50 cells):
   → Radio refinery for dock reservation (RADIO_DOCKING)
   → State = 3 (DOCK)
   → Unit will DRIVE to refinery (Mission_Enter + Set_Destination to building cell)

4. IF distance > threshold OR no dock found:
   → Compute dock-adjacent cell from BuildingType->DockOffset (+0x1618/+0x161C)
   → Call Pathfinding_validate_alternate to find valid cell
   → Call Set_Destination(validated_cell)
   → This cell is EMPTY → FindFirstBuilding returns NULL → unit TELEPORTS
```

### Key Insight

The miner teleports to a cell ADJACENT to the refinery, not onto the refinery itself.
After warping, the Is_Ok_To_End check eventually restores DriveLocomotionClass (if
piggybacked), and the miner drives the short remaining distance to dock.

---

## 12. Update_Position (0x718260) -- The Instant Teleport

Called in ChronoSphere path phases 2, 3, 4.

### Two Modes

**Mode 0 (applyOccupancy=false):** "Chrono damage + teleport"
- Iterates objects at destination cell
- Infantry vs infantry at same subcell: deal chrono damage to OTHER infantry
- Any other techno: deal chrono damage to them (full Strength * ChronoWarpDamagePercent)
- Flying unit at dest: deal chrono damage to the TELEPORTER (telefragged)
- Bridge integrity check

**Mode 1 (applyOccupancy=true):** "Read ChronoDestCoord + relocate"
- Read destination from TechnoClass+0x288/28C/290
- Set Z to ground height
- Handle bridge Z offset
- Place unit at destination via SetCoord

**The teleport is INSTANT.** No interpolation. The unit jumps from current position to
destination in a single call.

**Confidence: 85%.**

---

## 13. PostWarpValidation (0x7187A0) -- Death on Invalid Terrain

Called in Phase 5 when PendingWarpPhase==0 (self-teleport only, NOT ChronoSphere).

### Death Conditions

**Water death:**
```
if (destCell->LandType == 2 (WATER)
    AND NOT naval-capable
    AND NOT Chronoshiftable (TypeClass+0xCCE)
    AND NOT infantry
    AND destCell has NO bridge (flags bit 0x100)
    AND LandType != 1 (ROAD)):
  → techno->ShouldSelfDestruct (+0x3CD) = 1
  → techno->KillSelf()
  → Release passengers via FUN_006B0AE0 with ChronoSourceBuilding/House
```

**Blocked cell:**
```
if (CanEnterCell returns MOVE_NO (7)):
  if (has bridge AND NOT infantry):
    → Kill self (same as water death)
  else:
    → Deal chrono damage to self (full Strength * ChronoWarpDamagePercent)
    → Unit survives but takes heavy damage
```

### LandType Enum (CellClass+0xEC)

| Value | Name |
|-------|------|
| 0 | Clear |
| 1 | Road |
| 2 | **Water** (triggers death) |
| 3 | Rock |
| 4 | Wall |
| 5 | Tiberium |
| 6 | Beach |
| 7 | Rough |
| 8 | Ice |

**Confidence: 85%.**

---

## 14. Visual Rendering During Warp

### BeingWarped DOES Set 50% Translucency Draw Flags -- CONFIRMED

**In `TechnoClass::Draw` (0x706640), lines 57-65:**
```c
cVar5 = (**(code **)(*param_1 + 0x1D4))();   // vtable+0x1D4
if (cVar5 != '\0' || (cVar5 = (**(code **)(*param_1 + 0x1D8))(), cVar5 != '\0')) {
    uVar12 = uVar12 | 0x2004;   // OR in 50% translucency
}
```

**The key insight:** vtable+0x1D4 and +0x1D8 resolve to the SAME functions for ALL
TechnoClass-derived classes:

| vtable+0x1D4 | 0x0070C5B0 | IsWarpingOut | Returns byte at this+0x270 |
| vtable+0x1D8 | 0x0070C5C0 | IsBeingWarped | Returns byte at this+0x271 |

**Verified from binary:** Checked UnitClass vtable (0x7F5C70+0x1D4), BuildingClass
vtable (0x7E3EBC+0x1D4), and InfantryClass vtable (0x7EB058+0x1D4). All three point
to the same pair of functions: 0x70C5B0 and 0x70C5C0.

**Result:** For UnitClass (chrono miner), when EITHER WarpingOut (+0x270) OR
BeingWarped (+0x271) is true, `TechnoClass::Draw` ORs `0x2004` into the draw flags,
producing **50% translucency**. This is a binary on/off effect -- the unit appears
semi-transparent at the destination for the chrono delay duration, then snaps to full
opacity when the timer expires and BeingWarped is cleared.

### Draw Flag Bits

```
0x2000 = base opaque draw
0x2004 = 0x2000 | 0x0004 = 50% translucent
         Bits 1-2 (mask 0x6): 0x4 = 50% translucency level
0x0800 = Z-buffer read (always added)
```

### Visual_Character Note

TeleportLocomotionClass uses the base `LocomotionClass::Visual_Character` (0x55ABC0)
which always returns 0 (opaque). This sets the initial draw flags to `0x2000`. The
translucency comes ONLY from the vtable+0x1D4/+0x1D8 check, not from the locomotor's
Visual_Character override.

### What Happens Visually During Self-Teleport

1. **Position change is INSTANT** (single tick in Phase 0)
2. **WarpOut animation** (Rules+0x33C, AnimType "WARPOUT") spawned at both departure
   and arrival positions -- this is the blue flash effect
3. **ChronoOutSound** plays at departure, **ChronoInSound** at arrival
4. **The unit is drawn at 50% translucency** at the destination while BeingWarped is true
5. When the chrono delay timer expires, BeingWarped is cleared and unit goes fully opaque

### Chronosphere Path: WarpingOut Also Triggers Translucency

For ChronoSphere-warped units, phases 0-1 set WarpingOut (+0x270) = 1 for 60 frames.
Since vtable+0x1D4 checks +0x270, the unit is also drawn translucent during the
60-frame warp-out countdown at the departure position.

### The Temporal Weapon Is Different

The smooth fade people associate with "chrono" comes from the **Chrono Legionnaire's
erasing weapon** on its TARGET. This uses:
- `TemporalClass::InitiateWarp` (0x71AF20): sets WarpingOut (+0x270) = 1
- `UpdateTemporalVisual` (0x70E5A0): 10-phase state machine
- ZReadWarp blitters for the shimmer effect
- Completely unrelated to locomotor teleport

### Additional Visual Systems (NOT related to teleport)

- `ScaleByWarpInVisualPhase` (0x70E4B0): reads +0x1B4/+0x1BC/+0x1C0 -- this is the
  **gap generator visual phase** (UpdateGapVisual at 0x70E920), NOT chrono teleport
- `ScaleByTemporalVisualPhase` (0x70E5A0): reads +0x198/+0x1A0/+0x1A4 -- this is the
  **temporal weapon erasing** visual, NOT chrono teleport

### Warp Animations

| Rules Offset | INI Key | AnimType | Spawned When |
|-------------|---------|----------|-------------|
| +0x33C | WarpOut | WARPOUT | Departure AND arrival (same for both!) |

The `WarpIn` and `WarpAway` AnimTypes at Rules+0x338/+0x340 are parsed from INI but
are NOT spawned by these verified TeleportLocomotion rows. Only `WarpOut`
(Rules+0x33C) is used for the departure/arrival constructor rows.

Constructor flags: `AnimClass::Constructor(type, coords, delay=0, loop=1, flags=0x600, zAdj=0, reverse=0)`
Flag `0x600` = `0x200` (center sprite) | `0x400`.

### Per-Type Sound Overrides

| TechnoTypeClass Offset | INI Key | Fallback (RulesClass) |
|------------------------|---------|----------------------|
| +0x574 | ChronoInSound | Rules+0x218 |
| +0x578 | ChronoOutSound | Rules+0x21C |

Logic: if TypeClass->sound != -1 OR Rules->sound != -1, play the sound.

**Confidence: 95%.** The vtable dispatch to IsWarpingOut/IsBeingWarped is confirmed
from UnitClass vtable layout. The 0x2004 translucency flag is verified from
TechnoClass::Draw decompilation.

---

## 15. TechnoClass Chrono Field Map (Complete, Verified)

All offsets are BYTE offsets on TechnoClass.

| Offset | Size | Name | Init | Purpose |
|--------|------|------|------|---------|
| +0x08C | 1 | IsOnBridge | 0 | Updated during warp for bridge detection |
| +0x090 | 1 | IsAlive | varies | Checked after PostWarpValidation |
| +0x09C | 12 | Location (X,Y,Z) | varies | Unit's current world position |
| +0x218 | 4 | GhostCell (CellClass*) | 0 | Deploy preview cell, NOT warp-related |
| +0x21C | 4 | OwnerHouse (HouseClass*) | set | Unit's owner |
| +0x220 | 4 | CloakState | 0 | 0=uncloaked, 1=cloaking, 2=cloaked, 3=uncloaking |
| +0x254 | 12 | ChronoSourceCoords (X,Y,Z) | NullCoord | Position before warp |
| +0x270 | 1 | WarpingOut | 0 | Set by temporal weapon AND chrono-in-transit phase 0 |
| +0x271 | 1 | BeingWarped | 0 | Gameplay flag: unit in post-warp cooldown |
| +0x27C | 1 | ChronoInTransit | 0 | Set externally by ChronoSphere handler |
| +0x280 | 4 | PendingWarpPhase | 0 | Set to 3 by ChronoSphere; locomotor picks up |
| +0x284 | 4 | ChronoLockDuration | 0 | Initially ChronoReinfDelay, overwritten with ChronoDelay in phase 3 |
| +0x288 | 4 | ChronoDestCoord.X | NullCoord | Warp destination (set by ChronoSphere) |
| +0x28C | 4 | ChronoDestCoord.Y | NullCoord | |
| +0x290 | 4 | ChronoDestCoord.Z | NullCoord | |
| +0x2B4 | 4 | PendingAction (ptr?) | 0 | If non-zero, skip idle behavior after timer |
| +0x2D8 | 4 | LinkedBuilding (ptr) | 0 | Used in water death cleanup |
| +0x3CD | 1 | ShouldSelfDestruct | 0 | Set when warped onto invalid terrain |
| +0x3D5 | 1 | Discovered | varies | Cleared if dest not in playfield |
| +0x428 | 4 | ChronoSourceBuilding (ptr) | 0 | ChronoSphere that initiated warp (for kill credit) |
| +0x42C | 4 | ChronoSourceHouse (ptr) | 0 | House that owns the ChronoSphere |
| +0x5A4 | 4 | NavTarget / DockBuilding | 0 | Current movement/dock target |
| +0x694 | 4 | FlashAnim (ptr) | 0 | Detached during warp |
| +0x6AD | 1 | IsDeploying | 0 | Checked by Is_Ok_To_End |

**Confidence: 95%** for core chrono fields (+0x270-0x290, +0x428-0x42C).
**Confidence: 85%** for supporting fields (+0x2B4, +0x2D8, +0x694).

---

## 16. TimerCheck (0x719BF0) -- Verified Decompilation

```c
void TimerCheck(TeleportLocomotionClass* this) {
    // this = IUnknown pointer (base+0x00)
    int remaining = this->Timer.Duration;           // +0x44
    if (this->Timer.StartFrame != -1) {             // +0x3C
        int elapsed = CurrentFrame - this->Timer.StartFrame;
        if (elapsed >= remaining) goto expired;
        remaining -= elapsed;
    }
    if (remaining != 0) return;  // still counting

expired:
    // 1. Clear BeingWarped
    techno->BeingWarped (+0x271) = 0;

    // 2. If no pending action: idle behavior
    if (techno->PendingAction (+0x2B4) == 0) {
        FUN_0070F770(techno);                       // Random idle timer (4-8 frames)
        bool acquired = TechnoClass__Passive_Target_Acquire();
        if (!acquired) {
            techno->SetOccupation(0, 1);            // vtable+0x484
        }
    }

    // 3. Advance phase (ChronoSphere path only)
    if (this->WarpPhase > 0) {
        this->WarpPhase++;
    }
}
```

**Confidence: 98%.**

---

## 17. Edge Cases

### Destination Cell Occupied When Miner Arrives

**Self-teleport (Phase 0):** Process() validates the destination via CanEnterCell and
Pathfinding_validate_alternate BEFORE the warp occurs. If the destination is blocked,
DestCoord is set to NullCoord and HeadToCoord aborts. The warp never starts.

**ChronoSphere (Update_Position):** Objects at the destination take chrono damage
(full Strength * ChronoWarpDamagePercent warhead). Infantry at the same subcell are
killed. The teleporting unit still arrives.

### Refinery Destroyed During Teleport

The warp itself completes in a single tick (self-teleport), so the refinery state during
the warp is moot. After arrival, the miner is at the dock-adjacent cell with BeingWarped
set. When the timer expires, the miner enters idle behavior. If the refinery is gone,
Mission_Harvest state 2 will fail to find a docking bay and the miner will search for
another refinery or wander.

### Can a Chrono Miner Be Killed During Teleport?

**Self-teleport:** The unit cannot be killed "during" teleport because it's instant
(single tick). While BeingWarped is true (post-warp cooldown), the unit CAN be attacked
and damaged normally. BeingWarped does not provide invulnerability.

**ChronoSphere:** During phases 0-1 (60-frame warp-out), WarpingOut is set. During
phases 2-6, BeingWarped is set. The unit's position changes in phase 2. Units CAN be
attacked during these phases -- BeingWarped/WarpingOut are rendering/targeting flags
but do not prevent damage.

### PostWarpValidation Death

If the unit warps onto water (LandType==2) with no bridge and no naval capability,
it is killed. ShouldSelfDestruct (+0x3CD) is set and KillSelf() is called. Kill credit
goes to ChronoSourceHouse (+0x42C).

### The WhatAmI==1 Instant-Warp Check (Harvester=yes Units)

In Phase 0, step 11 checks `WhatAmI()==1 AND UnitTypeClass->Harvester(+0xE0E)`.

- **WhatAmI()==1 is UnitClass** (verified: 0x746E20 returns 1)
- **UnitTypeClass+0xE0E is Harvester=yes** (verified from 3 independent reports)
- **InfantryClass returns WhatAmI()==0xF** (verified: 0x523340 returns 0xF)

When the check passes (chrono miner = UnitClass + Harvester=yes), the post-warp
timer is zeroed and BeingWarped is cleared immediately. The miner appears at the
destination fully opaque with no cooldown.

Chrono Legionnaires (InfantryClass, WhatAmI=0xF) do NOT match this check. They get
the full chrono delay with 50% translucency at the destination.

**Confidence: 98%.**

---

## 18. Locomotor CLSID Reference

| CLSID | Address | Name |
|-------|---------|------|
| {4A582741-9839-11d1-B709-00A024DDAFD1} | 0x7E9A30 | DriveLocomotionClass |
| {4A582742-...} | 0x7E9A40 | WalkLocomotionClass |
| {4A582743-...} | 0x7E9A50 | HoverLocomotionClass |
| {4A582747-...} | 0x7E9A90 | TeleportLocomotionClass |

---

## 19. Key Globals

| Address | Name | Notes |
|---------|------|-------|
| 0xB0EBF8 | g_NullCoord_X | Sentinel for "no coordinate" (12 bytes) |
| 0xB0EC38 | g_BridgeZOffset | Height offset for bridge Z |
| 0xA8ED84 | g_CurrentFrameCounter | Global frame counter |
| 0x8871E0 | g_RulesClass_Instance | RulesClass singleton pointer |
| 0xA8ED44 | g_AnimArray | Array of all AnimClass instances |
| 0xA8ED50 | g_AnimCount | Count of anim instances |

---

## 20. Complete Call Graph

```
Player Move Order / Mission_Harvest / ChronoSphere
  │
  ├─► TechnoClass::Set_Destination (0x741970)
  │     ├─ Teleporter block (0x7423CD): FindFirstBuilding check
  │     │   ├─ Building on dest cell → Piggyback Drive, unit DRIVES
  │     │   └─ Empty cell + TeleportLoco → Skip piggyback, unit WARPS
  │     │
  │     └─► FootClass::Set_Destination_Internal (0x4D94B0)
  │           └─► ILocomotion::Head_To_Coord (vtable+0x44)
  │                 └─► TeleportLocomotionClass::HeadToCoord (0x718100)
  │                       ├─ Guards: warp/deploy state
  │                       ├─ Process (0x718B70): validate dest
  │                       └─ Set IsMoving=1, store HeadToCoord
  │
  └─► [every tick] FootClass::AI (0x4DA530)
        ├─► ILocomotion::Process (vtable+0x40)
        │     └─► StateMachineTick (0x7192F0)
        │           ├─ Pre-checks: BeingWarped idle, PendingWarpPhase pickup
        │           ├─ Path A (self-teleport): instant warp + timer cooldown
        │           └─ Path B (ChronoSphere): 8-phase state machine
        │
        └─► IPiggyback::Is_Ok_To_End check
              └─ If true: swap piggybacked locomotor back to active
```

---

## 21. Ghidra Functions Referenced

| Address | Name | Purpose |
|---------|------|---------|
| 0x718000 | TeleportLocomotionClass__Constructor | Field initialization |
| 0x718080 | TeleportLocomotionClass__Is_Moving | Returns base+0x34 == 1 |
| 0x7180A0 | TeleportLocomotionClass__Destination | Returns HeadToCoord or Location |
| 0x718100 | TeleportLocomotionClass__HeadToCoord | Arms the warp (IsMoving=1) |
| 0x718230 | TeleportLocomotionClass__ClearCoords | Clears dest, IsMoving, flags |
| 0x718260 | TeleportLocomotionClass__Update_Position | Instant teleport + chrono damage |
| 0x7187A0 | TeleportLocomotionClass__PostWarpValidation | Water/blocked death check |
| 0x718B70 | TeleportLocomotionClass__Process | Destination validation |
| 0x7192F0 | TeleportLocomotionClass__StateMachineTick | Main tick function |
| 0x719BF0 | TeleportLocomotionClass__TimerCheck | Timer expiry handler |
| 0x719E90 | TeleportLocomotionClass__Begin_Piggyback | Store locomotor |
| 0x719EE0 | TeleportLocomotionClass__End_Piggyback | Return stored locomotor |
| 0x719F30 | TeleportLocomotionClass__Is_Ok_To_End | 6 conditions for swap |
| 0x71A100 | TeleportLocomotionClass__Is_Piggybacking | PiggybackLoco != 0 |
| 0x741970 | TechnoClass__Set_Destination | Teleport-vs-drive decision |
| 0x47EBA0 | CellClass__FindFirstBuilding | Building presence check |
| 0x70C5B0 | TechnoClass__IsWarpingOut | Returns +0x270 |
| 0x70C5C0 | TechnoClass__IsBeingWarped | Returns +0x271 |
| 0x70C5F0 | TechnoClass__IsNotWarping | +0x270==0 AND +0x271==0 |
| 0x55ABC0 | LocomotionClass__Visual_Character | Always returns 0 (opaque) |
| 0x73E5E0 | UnitClass__Mission_Harvest | Harvest state machine |
| 0x65EC30 | ChronoSphere__WarpUnitsAtCell | ChronoSphere handler |
| 0x706640 | TechnoClass__Draw | Main draw function |
| 0x70E4B0 | TechnoClass__ScaleByWarpInVisualPhase | Gap generator visual scale |
| 0x70E5A0 | TechnoClass__UpdateTemporalVisual | Temporal weapon 10-phase visual |
| 0x70E920 | TechnoClass__UpdateGapVisual | Gap generator visual state machine |
