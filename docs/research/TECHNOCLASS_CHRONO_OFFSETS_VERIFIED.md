# TechnoClass Chrono-Related Field Offsets -- Definitive Verification

## Purpose

This document resolves conflicting information between v1 and v2 of the chrono reports
about TechnoClass field offsets. Every offset below has been verified by examining the
actual binary instructions in gamemd.exe using Ghidra MCP live decompilation.

All offsets are relative to the TechnoClass `this` pointer (byte offsets).

---

## Conflict #1: TechnoClass+0x244 (WarpFactor?)

**v1 claimed:** "float WarpFactor, visual progress 0.0 to 1.0"
**v2 claimed:** "not a float ramp, incorrect"

### Verdict: NEITHER report is correct about +0x244.

**+0x218 is the WarpState field, NOT +0x244.**

Evidence:
- `TechnoClass__SetWarpVisualState` (0x0070c610) writes `param` to `this+0x218`
- The TeleportLocomotionClass state machine reads `[techno+0x218]` (at 0x007196E3)
- In the constructor, `param_1[0x86]` (= byte offset 0x218) is initialized to 0
- State 5 and 7 of the state machine call `SetWarpVisualState(0)` to clear it
- +0x244 is NOT initialized in the TechnoClass constructor (falls in a timer sub-object gap)

**+0x244 is likely part of a CDTimerClass at +0x23C..+0x248** (param_1[0x8F]=1, [0x90]=CurrentFrame,
[0x91]=uninitialized/padding, [0x92]=0). It is NOT a chrono-related float.

**Confidence: 98%**

---

## Conflict #2: WarpingOut / BeingWarped byte offsets

**v1 claimed:** +0x268 = BeingWarpedOut (bool), +0x269 = WarpingOut (bool)
**v2 claimed:** +0x270 = WarpingOut, +0x271 = BeingWarped

### Verdict: v2 is CORRECT. +0x270 and +0x271.

Evidence from the binary:

1. `TechnoClass__IsBeingWarped` (0x0070C5C0):
   ```c
   return *(byte *)(this + 0x271);
   ```

2. `TechnoClass__IsNotWarping` (0x0070C5F0):
   ```c
   return (*(byte *)(this + 0x270) == 0) && (*(byte *)(this + 0x271) == 0);
   ```

3. `TechnoClass__IsWarpingOut` (0x0070C5B0, was incorrectly named BuildingClass__HasPower):
   ```c
   return *(byte *)(this + 0x270);
   ```

4. State machine at 0x7198DA: `MOV byte ptr [EAX + 0x271], 0x1` (sets BeingWarped)
5. State machine at 0x7198E4: `MOV byte ptr [ECX + 0x27C], 0x0` (clears ChronoInTransit)
6. State machine at 0x7198EE: `MOV byte ptr [EDX + 0x270], 0x0` (clears WarpingOut)
7. TimerCheck at 0x719C15: `MOV byte ptr [EAX + 0x271], 0x0` (clears BeingWarped)
8. Constructor at 0x6F2DB4: `MOV byte ptr [ESI + 0x270], BL` (init 0)
9. Constructor at 0x6F2DBA: `MOV byte ptr [ESI + 0x271], BL` (init 0)

**+0x268 and +0x269 are separate fields** (also initialized to 0 in the constructor)
but they are NOT the warp flags. Their purpose is currently unknown.

**Confidence: 100%**

---

## Conflict #3: TechnoClass+0x27C (ChronoInTransit)

**v1 claimed:** "int ChronoLockRemaining, frames left in transit"
**v2 claimed:** "byte ChronoInTransit, set externally by ChronoSphere"

### Verdict: v2 is CORRECT. It is a BYTE flag, not an int countdown.

Evidence:
1. Constructor: `*(byte *)(param_1 + 0x9F) = 0` -- initialized as single byte (via
   `undefined1` cast, NOT as dword `param_1[0x9F]`)
2. ChronoWarp handler in SuperClass__Launch case 4:
   `*(byte *)(piVar2 + 0x9F) = 1` -- piVar2 is int*, so byte offset = 0x27C. Written as byte value 1.
3. State machine at 0x719351: `MOV DL, byte ptr [ECX + 0x27C]` -- read as byte
4. State machine at 0x7198E4: `MOV byte ptr [ECX + 0x27C], 0x0` -- cleared as byte
5. Is_Ok_To_End at 0x719F54: `MOV CL, byte ptr [EAX + 0x27C]` -- checked as byte

The field is set to 1 by the ChronoSphere superweapon handler when a unit is being
chrono-warped. The state machine phase 0 checks this flag to decide whether to enter
the "externally initiated warp" path (phases 0->1->2->3->4->5->6->7).

**Confidence: 100%**

---

## Conflict #4: TechnoClass+0x280 (PendingWarpPhase)

**v1 claimed:** "CoordStruct ChronoDestCoords (12 bytes)"
**v2 claimed:** "int PendingWarpPhase, set to 3 by ChronoSphere"

### Verdict: v2 is CORRECT. It is an int PendingWarpPhase.

Evidence:
1. Constructor: `param_1[0xA0] = 0` (dword, byte offset 0x280)
2. ChronoSphere__WarpUnitsAtCell (0x0065EC30): `piVar6[0xA0] = 3` -- sets to 3
3. Also at 0x0065F29F: `MOV dword ptr [ESI + 0x280], 0x3` -- confirmed dword write of 3
4. State machine pre-check: `if (piVar1[0xA0] != 0) { param_1[0xD] = piVar1[0xA0]; }`
   -- The locomotor's WarpPhase is initialized FROM this field
5. State machine phase 7: `*(dword *)(uVar3 + 0x280) = 0` -- cleared at end
6. TeleportLocomotionClass__InitiateWarp: `*(dword *)(uVar1 + 0x280) = 0` -- cleared

The ChronoDestCoords are at +0x288/+0x28C/+0x290, NOT starting at +0x280.

**Confidence: 100%**

---

## Conflict #5: TechnoClass+0x284 (ChronoLockDuration)

**v2 claimed:** "int ChronoLockDuration = Rules->ChronoDelay"

### Verdict: PARTIALLY correct. Initially set to ChronoReinfDelay, later overwritten with ChronoDelay.

Evidence:
1. Constructor: `param_1[0xA1] = 0` (byte offset 0x284, initialized to 0)
2. ChronoSphere__WarpUnitsAtCell (0x0065EC30):
   `piVar6[0xA1] = *(int *)(g_RulesClass_Instance + 0xBF0)`
   -- Set to `Rules->ChronoReinfDelay` (NOT ChronoDelay)
3. State machine phase 3 (0x719983):
   `*(dword *)(param_1[2] + 0x284) = *(dword *)(g_RulesClass_Instance + 0xBEC)`
   -- Overwritten with `Rules->ChronoDelay` during phase 3
4. State machine phase 5 (0x719B23):
   `iVar6 = *(int *)(techno + 0x284)` -- used as the post-warp lockdown timer duration

RulesClass chrono offsets (verified from ReadGeneral assembly):
- `Rules+0xBEC` = ChronoDelay (int, read from "ChronoDelay" INI key)
- `Rules+0xBF0` = ChronoReinfDelay (int, read from "ChronoReinfDelay" INI key)
- `Rules+0xBF4` = ChronoDistanceFactor (int, read from "ChronoDistanceFactor")
- `Rules+0xBF8` = ChronoTrigger (bool, read from "ChronoTrigger")
- `Rules+0xBFC` = ChronoMinimumDelay (int, read from "ChronoMinimumDelay")
- `Rules+0xC00` = ChronoRangeMinimum (int, read from "ChronoRangeMinimum")

**Confidence: 98%**

---

## Conflict #6: TechnoClass+0x288/+0x28C/+0x290 (ChronoDestCoords)

**Both reports agreed** these are ChronoDestCoords (X, Y, Z).

### Verdict: CONFIRMED. These ARE ChronoDestCoords.

Evidence:
1. Constructor:
   - `param_1[0xA2] = g_NullCoord_Chrono_X` (byte offset 0x288)
   - `param_1[0xA3] = g_NullCoord_Chrono_Y` (byte offset 0x28C)
   - `param_1[0xA4] = g_NullCoord_Chrono_Z` (byte offset 0x290)
2. ChronoSphere__WarpUnitsAtCell (0x0065EC30):
   ```c
   piVar6[0xA2] = dest_X;  // +0x288
   piVar6[0xA3] = dest_Y;  // +0x28C
   piVar6[0xA4] = dest_Z;  // +0x290
   ```
3. SuperClass__Launch case 4 (ChronoWarp handler):
   ```c
   piVar2[0xA2] = X;  // +0x288
   piVar2[0xA3] = Y;  // +0x28C
   piVar2[0xA4] = Z;  // +0x290
   ```
4. State machine phase 2: reads `*(dword *)(techno + 0x288)` and passes to Update_Position
5. State machine phase 3: reads `techno[0xA2]` (= +0x288) and passes to Update_Position

There is a SEPARATE coord set at +0x254/+0x258/+0x25C also initialized to NullCoord,
which appears to be the ChronoSourceCoords (where the unit was BEFORE warping).

**Confidence: 100%**

---

## Additional Verified Offsets

### TechnoClass+0x428 / +0x42C (ChronoSourceBuilding / ChronoSourceHouse)

Both initialized to 0 in the constructor.

- **+0x428** = ptr to the source building (e.g., the ChronoSphere building that initiated
  the warp, or the War Factory deploying a unit). Set in `BuildingClass__DeployUnit_ChronoWarp`
  (0x0070FEE0) as `*(ptr *)(techno + 0x428) = building_this`.
- **+0x42C** = ptr to the owner HouseClass*. Set as `*(ptr *)(techno + 0x42C) = building->Owner`.
  In SuperClass::Launch case 4: `piVar2[0x10B] = super_param[0xB]` (= house pointer from the
  superweapon invocation context).
- Both are cleared to 0 in state machine phase 5 (after post-warp validation).
- Used in `TeleportLocomotionClass__PostWarpValidation` for kill credit when a chrono-warped
  unit dies on impassable terrain: `FUN_006b0ae0(techno+0x428, techno+0x42C)`.

**Confidence: 95%**

### TechnoClass+0x694 (Anim Pointer -- class-dependent)

- In FootClass: initialized to 0 in `FootClass__Constructor` (0x4D33E4)
- In BuildingClass: initialized differently in `BuildingClass__Constructor` (0x43B719/43B901)
- Used in the teleport state machine (0x7195BF): `if (*(int *)(techno + 0x694) != 0) { FUN_0062A4A0(); }`
  -- appears to be a flash/warhead animation pointer, removed before warp
- In TechnoTypeClass: stored as a byte at +0x694 (read/written in ReadINI) -- different class!

**Confidence: 85%** (dual meaning between instance and type class)

### TechnoClass+0x6AD (IsDeploying flag)

- Byte flag on FootClass, initialized to 0 in `FootClass__Constructor` (0x4D3414)
- Set to 1 in `TechnoClass__PerformDeploy` (0x710352)
- Checked in `TeleportLocomotionClass__Is_Ok_To_End` (0x719F65): teleport cannot end while deploying
- Heavily used in locomotor functions (54 references total)

**Confidence: 95%**

### TechnoClass+0x218 (WarpState / Visual Warp Factor)

- Int field initialized to 0 in the constructor
- Written by `TechnoClass__SetWarpVisualState` (0x0070C610)
- Read by the teleport state machine for rendering
- Cleared to 0 in phases 5 and 7 of the state machine
- Controls the visual warp-in/out rendering effect intensity

**Confidence: 98%**

---

## Complete Chrono Field Map (TechnoClass)

| Offset | Size | Name | Init | Purpose |
|--------|------|------|------|---------|
| +0x218 | 4 | WarpState | 0 | Visual warp effect state (set by SetWarpVisualState) |
| +0x254 | 12 | ChronoSourceCoords | NullCoord | Position before warp (X/Y/Z) |
| +0x268 | 1 | Unknown_268 | 0 | Unknown byte flag |
| +0x269 | 1 | Unknown_269 | 0 | Unknown byte flag |
| +0x270 | 1 | WarpingOut | 0 | True when unit is warping OUT (disappearing) |
| +0x271 | 1 | BeingWarped | 0 | True when unit is being chrono-warped |
| +0x272 | 1 | Unknown_272 | 0 | Unknown byte flag |
| +0x27C | 1 | ChronoInTransit | 0 | Byte flag: set to 1 by ChronoSphere handler |
| +0x280 | 4 | PendingWarpPhase | 0 | Set externally (3 by ChronoSphere); locomotor picks up as starting state |
| +0x284 | 4 | ChronoLockDuration | 0 | Initially ChronoReinfDelay, overwritten with ChronoDelay in phase 3 |
| +0x288 | 4 | ChronoDestCoord_X | NullCoord | Warp destination X |
| +0x28C | 4 | ChronoDestCoord_Y | NullCoord | Warp destination Y |
| +0x290 | 4 | ChronoDestCoord_Z | NullCoord | Warp destination Z |
| +0x428 | 4 | ChronoSourceBuilding | 0 | Ptr to building that initiated the warp |
| +0x42C | 4 | ChronoSourceHouse | 0 | Ptr to HouseClass of the warp initiator |

---

## Ghidra Functions Labeled in This Session

| Address | Name | Purpose |
|---------|------|---------|
| 0x0070C5B0 | TechnoClass__IsWarpingOut | Returns byte at +0x270 (was misnamed BuildingClass__HasPower) |
| 0x0070C5C0 | TechnoClass__IsBeingWarped | Returns byte at +0x271 |
| 0x0070C5F0 | TechnoClass__IsNotWarping | Returns true if both +0x270 and +0x271 are 0 |
| 0x0070C610 | TechnoClass__SetWarpVisualState | Writes param to +0x218 |
| 0x0065EC30 | ChronoSphere__WarpUnitsAtCell | ChronoSphere warp handler, sets +0x288-290, +0x271, +0x284, +0x280 |
| 0x00719400 | TeleportLocomotionClass__InitiateWarp | Shared block for warp initiation in the state machine |
| 0x007197D0 | TeleportLocomotionClass__StartWarpOut | Sets +0x270 = 1 |
| 0x00719790 | TeleportLocomotionClass__ClearPendingWarpPhase | Clears +0x280 = 0 |
| 0x0070FEE0 | BuildingClass__DeployUnit_ChronoWarp | Sets +0x428 (building) and +0x42C (house) on deployed unit |

## RulesClass Chrono Settings (verified from ReadGeneral)

| Offset | INI Key | Type |
|--------|---------|------|
| +0xBEC | ChronoDelay | int |
| +0xBF0 | ChronoReinfDelay | int |
| +0xBF4 | ChronoDistanceFactor | int |
| +0xBF8 | ChronoTrigger | bool |
| +0xBFC | ChronoMinimumDelay | int |
| +0xC00 | ChronoRangeMinimum | int |
