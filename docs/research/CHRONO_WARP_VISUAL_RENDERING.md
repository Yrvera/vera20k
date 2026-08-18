# Chrono Warp Visual Rendering System -- Ghidra Research Report

## Overview

This report covers the visual rendering aspects of the chrono warp system in gamemd.exe,
including how units are drawn during teleport, the chrono sparkle effect, draw flag
translucency, and the blitter pipeline. All offsets are byte offsets unless noted.

---

## 1. TechnoClass+0x218 -- SETTLED: NOT a WarpFactor Float

**Previous claims:**
- v1 report: "WarpFactor" float ramping 0.0->1.0 during teleport
- v2 report: Contradicted v1, said no float ramp exists

**Verdict: v2 is correct. +0x218 is a CellClass pointer, not a float.**

### Evidence

1. **Constructor (0x006f2b40):** `param_1[0x86] = 0` -- initialized to 0 (null pointer),
   where param_1 is `undefined4 *`, so index 0x86 = byte offset 0x218.

2. **SetGhostCell (0x0070C610, formerly "SetWarpVisualState"):**
   ```c
   void TechnoClass__SetGhostCell(int param_1, undefined4 param_2) {
       *(undefined4 *)(param_1 + 0x218) = param_2;
   }
   ```
   This trivial setter stores whatever is passed. Looking at ALL callers:

3. **BuildingClass__ExitObject_Main (0x00443C60)** calls it with (corrected 2026-07-12: was
   "TacticalClass__DrawObjects (0x00443C40)" — no function of that name exists anywhere in the
   binary, verified via `search_functions_enhanced name_pattern="DrawObjects"` (0 results).
   0x00443C40 is inside the unrelated `BuildingClass__ToggleGate` (body 0x443b90-0x443c5e,
   verified via `get_function_by_address 0x443c40`). The actual caller with exactly 6 distinct
   SetGhostCell call sites — matching the "6 distinct calling contexts" in the confidence line
   below — is `BuildingClass__ExitObject_Main` (body 0x443c60-0x4456a4), verified via
   `get_xrefs_to 0x0070C610` (call sites 0x44449b, 0x44496c, 0x444997, 0x444ce2, 0x444d95,
   0x444db7, all inside that function) and `get_function_by_address 0x443c60`. ROOT_CAUSE:
   caller misattributed to a nonexistent function name, not a display-label drift on a real
   function.):
   - `MapClass__Get_CellClass(...)` -- a CellClass pointer
   - `0` -- clearing it
   - `param_1[0x86]` -- this-object member (the "TacticalClass member" attribution is wrong
     since `this` is BuildingClass here, not TacticalClass; the correct field identity was not
     re-verified this session)
   - `param_2[0x169]` -- an object pointer (offset 0x5A4 on a TechnoClass; confirmed this
     session via `get_assembly_context 0x00444ce2`: `MOV EAX,dword ptr [EDI + 0x5a4]`
     immediately precedes the SetGhostCell call)

4. **REFUTED by negative proof (2026-07-19):** The previously-UNVERIFIED "DrawObjects
   rendering" narrative below is now disproven, not merely unconfirmed. Per
   `CHRONO_WARP_TECHNOCLASS_0X218_READER_GHIDRA_REPORT.md` (this session's re-swarm), the ONLY
   confirmed reader of +0x218 is `BuildingClass__ExitObject_Main` (0x00443C60) itself: three
   sites read `param_1[0x86]` (=+0x218) and forward it to a **vtable+0x480 dispatch**
   (building-exit occupation/bookkeeping — confirmed via `decompile_function 0x00443C60`,
   e.g. `iVar5 = param_1[0x86]; ... (**(code **)(*piVar7 + 0x480))();`), never to a draw/blit
   call. The entire render pipeline was swept and none of it reads +0x218: `TechnoClass__Draw`
   0x00706640, `Render` 0x00706ED0, `DrawSHP` 0x00705E00, `DrawExtras` 0x006F5190,
   `ModifyCloakDrawFlags` 0x0070ED80, `ScaleByWarpInVisualPhase` 0x0070E4B0 (re-verified this
   session via `decompile_function 0x0070E4B0` — reads only param_1+0x1B4/+0x1BC/+0x1C0, never
   +0x218), `UpdateGapVisual` 0x0070E920. The formerly-cited "TacticalClass__DrawObjects" was
   already known not to exist in the binary (point 3 correction); the retired bullets below
   (GetCoords/direction-angle/ghost-draw/SetOccupation) described a function that was never
   real and a mechanism the binary does not implement. Superseded — retained struck for
   history:
   - ~~Calls `GetCoords()` on the stored CellClass pointer~~
   - ~~Calculates direction angle from building to that cell~~
   - ~~Draws a "ghost" building at the cell's position with translucent flags~~
   - ~~Calls `SetOccupation(param_1[0x86])` passing the CellClass pointer~~

**Conclusion:** TechnoClass+0x218 is a **CellClass pointer** (constructor-init-to-0 and
trivial-setter facts remain solid, HIGH confidence). Its "deploy-preview ghost rendering" use
is **REFUTED** as of 2026-07-19 by a negative proof over the full render pipeline — see point 4
above and `CHRONO_WARP_TECHNOCLASS_0X218_READER_GHIDRA_REPORT.md`. The confirmed role is
building-exit occupation/bookkeeping (`BuildingClass__ExitObject_Main` → vtable+0x480), not
rendering. Do not model +0x218 in Rust rendering. It is still confirmed NOT a warp-fade-ramp
float.

**Function renamed in Ghidra:** `TechnoClass__SetGhostCell` (was `TechnoClass__SetWarpVisualState`)

**Confidence: 90%** (upgraded 2026-07-19 from 60%: the "not a float / is a CellClass
pointer" facts stay HIGH confidence — constructor + trivial setter. The "deploy-preview ghost
rendering" caller narrative, previously UNVERIFIED, is now REFUTED by a negative proof sweeping
the confirmed reader (`BuildingClass__ExitObject_Main`, forwards to vtable+0x480, not draw) and
the entire render pipeline (none of it reads +0x218) — see
`CHRONO_WARP_TECHNOCLASS_0X218_READER_GHIDRA_REPORT.md`. Not 100% because the vtable+0x480
target itself (building-exit occupation bookkeeping) was not independently decompiled this
session — its role is inferred from the call context, not read directly.)

---

## 2. TechnoClass+0x244 -- SETTLED: CDTimerClass Middle Field

**Previous claims:** "WarpFactor" float ramping 0.0->1.0.

**Verdict: WRONG. +0x244 is the unused middle field of a CDTimerClass triple.**

### Evidence

From the TechnoClass constructor (param_1 is `undefined4 *`):
```
param_1[0x90] = g_CurrentFrameCounter;  // offset 0x240 = Timer.StartFrame
// param_1[0x91] is NOT explicitly initialized   // offset 0x244 = Timer.field_4
param_1[0x92] = 0;                      // offset 0x248 = Timer.Duration
```

This follows the exact CDTimerClass pattern documented in the v2 report:
```
struct CDTimerClass {
    int StartFrame;  // +0x00
    int field_4;     // +0x04 (never read for countdown logic)
    int Duration;    // +0x08
};
```

The middle field (+0x04 of the timer, which is TechnoClass+0x244) is written when
the timer is set but NEVER read during countdown checks. It is vestigial padding.

**Confidence: 95%** -- Verified from constructor initialization pattern.

---

## 3. TechnoClass+0x270 and +0x271 -- SETTLED

**v1 report:** BeingWarpedOut at +0x268, WarpingOut at +0x269.
**v2 report:** WarpingOut at +0x270, BeingWarped at +0x271.

**Verdict: v2 is correct.**

### TechnoClass+0x270 = WarpingOut (byte)

**Set to 1 by:**
- `TeleportLocomotion__Phase0_SetWarpingOut` (0x007197D0): `*(param_1 + 0x270) = 1`
  Called during chrono-in-transit phase 0, when ChronoInTransit flag is set.
- `TemporalClass__InitiateWarp` (0x0071AF20): `*(target + 0x270) = 1`
  Called when a temporal weapon (Chrono Legionnaire's erasing weapon) starts warping a unit.

**Set to 0 by:**
- TeleportLocomotionClass phase 2 (in-transit start): `techno->WarpingOut = 0`

**Read by:**
- `TechnoClass__IsNotWarping` (0x0070C5F0)

### TechnoClass+0x271 = BeingWarped (byte)

**Set to 1 by:**
- TeleportLocomotionClass phase 0 (warp start): `techno->BeingWarped = 1`
- TeleportLocomotionClass phase 2 (in-transit): `techno->BeingWarped = 1`

**Set to 0 by:**
- TeleportLocomotionClass phase 7 (complete): `techno->BeingWarped = 0`
- Timer expiry function (0x719BF0): clears when warp timer expires

**Read by:**
- `TechnoClass__IsBeingWarped` (0x0070C5C0): returns the byte directly
- Pre-phase check in TeleportLocomotionClass: gates whether unit is idle-warped

### TechnoClass__IsNotWarping (0x0070C5F0)
```c
bool TechnoClass__IsNotWarping(int techno) {
    if (*(char *)(techno + 0x270) == 0 && *(char *)(techno + 0x271) == 0)
        return true;
    return false;
}
```

### Byte Pattern Verification
All writes verified via byte pattern search:
- `C6 81 70 02 00 00 01` at 0x7197D0 (phase 0) and 0x71B0EA (temporal)
- `C6 81 71 02 00 00 01` at 0x719579 (phase 0) -- but wait, this writes to ECX+0x271
- `C6 80 71 02 00 00 01` at 0x7198DA (phase 2, inside StateMachineTick)
- `C6 80 71 02 00 00 00` at 0x719C15 (corrected 2026-07-12: was grouped above as "...01" i.e.
  a duplicate phase-2 SET; `read_memory 0x719c15` shows the immediate byte is `00`, a CLEAR,
  not a SET. This address is inside `TeleportLocomotionClass__TimerCheck` (body
  0x00719bf0-0x00719c57, verified via `get_function_by_address 0x719c15`), matching the
  "Timer expiry function (0x719BF0): clears when warp timer expires" bullet above, not phase 2.
  ROOT_CAUSE: transcription slip in the original byte-pattern pass — ancillary evidence list
  only, the field-description text above it was already correct.)

**Confidence: 99%** -- Verified from binary byte patterns, decompilation, and accessor functions.

---

## 4. FUN_0070C610 (SetGhostCell) -- Fully Decompiled

Already covered in section 1. It's a trivial one-line setter:
```c
void TechnoClass__SetGhostCell(int this, int cellPtr) {
    *(int *)(this + 0x218) = cellPtr;
}
```

**What it stores:** A CellClass pointer (or NULL). (corrected 2026-07-19: the "Used by
TacticalClass__DrawObjects..." claim is **REFUTED**, not merely unverified — see Section 1
point 4. No function of that name exists in the binary, and the confirmed reader,
`BuildingClass__ExitObject_Main` (0x00443C60), forwards +0x218 to a vtable+0x480
building-exit-bookkeeping dispatch, never a draw/blit call; a full render-pipeline sweep found
no reader of +0x218 anywhere. Verified via `decompile_function 0x00443C60` and
`decompile_function 0x0070E4B0`; see
`CHRONO_WARP_TECHNOCLASS_0X218_READER_GHIDRA_REPORT.md`. The setter is still called from 60+
sites across 25 functions, including `BuildingClass__ExitObject_Main` and
`TeleportLocomotionClass__StateMachineTick` itself, not narrowly from a deploy-preview
renderer.)

**Not a warp-fade-ramp** in the sense of Section 6's translucency finding (that part still
holds: chrono teleport draws the unit fully opaque). +0x218 does NOT feed chrono-warp/temporal-
erasure rendering (negative proof, 2026-07-19 — see Section 1 point 4); its confirmed role
beyond "clear on phase 7 complete" (Section 10) is building-exit occupation bookkeeping via
`BuildingClass__ExitObject_Main`'s vtable+0x480 dispatch. The "deploy/move preview render"
framing is REFUTED, not just unconfirmed.

---

## 5. CHRONOSK.SHP -- The Ivan Bomb Clock (NOT Chrono Sparkle)

### String and File Reference
- String `"CHRONOSK.SHP"` at 0x0083B0E0
- Loaded in RulesClass__ReadCombatDamage at 0x0066C5FB via `LoadFileFromMIX()`
- Stored at **RulesClass+0xFE0** (the SHP data pointer)

### What It Actually Is
Despite the "CHRONO" prefix, **CHRONOSK.SHP is the Ivan Bomb countdown clock graphic**.
It contains 13 frames (0-11 = clock positions, 12 = detonation). This was verified by
decompiling the frame calculation function `IvanBomb__GetClockFrame` (0x00438A00):
```c
int IvanBomb__GetClockFrame(int bomb) {
    if (bomb->State == 1) return 12;  // detonated
    int frame = ((g_CurrentFrame - bomb->StartFrame) / (Rules->IvanTimedDelay / 6)) * 2;
    if (g_CurrentFrame % (Rules->IvanIconFlickerRate * 2) >= Rules->IvanIconFlickerRate)
        frame++;  // flicker effect
    return min(frame, 11);
}
```

### How It's Drawn
In `TechnoClass__DrawExtras` (0x006F5190):
```c
if (((char)techno[0x1a] != 0) && (techno[0xe] != 0)) {
    // techno+0x68 = has bomb flag, techno+0x38 = bomb pointer
    int frame = IvanBomb__GetClockFrame(techno_bomb_ptr);
    CC_Draw_Shape(Rules->CHRONOSK_SHP, frame, &screen_coords, &viewport,
                  0xE00, 0, 0, 0, 1000, 0);
}
```

### ChronoSparkle1 Field
`ChronoSparkle1` is a standard AnimType field:
- String `"ChronoSparkle1"` at 0x0083CD64
- Read in RulesClass__ReadGeneral at 0x0066E300
- Stored at **RulesClass+0x344** (AnimType pointer)
- Parsed, but not referenced by the verified `TeleportLocomotionClass::InitiateWarp`
  constructor rows

---

## 6. Warp Visual Rendering Pipeline

### How Units Become Translucent During Warp

The chrono teleport does NOT use a special "warp shader" or "warp blitter". Instead:

1. **Self-teleport spawns only the WarpOut animation** (Rules+0x33C, AnimType from rules.ini),
   twice — once at the source cell and once at the destination cell. Verified by reading
   `TeleportLocomotionClass::InitiateWarp` (0x00719400): both `AnimClass::Constructor`
   calls reference `g_RulesClass + 0x33C`. The requested TeleportLocomotion rows use
   WarpOut, not WarpIn, WarpAway, or ChronoSparkle1.

2. **The unit itself is NOT rendered with translucency during chrono teleport.** The warp
   effect is purely the WarpOut animation overlay (the blue flash/shimmer of WARPOUT.shp).

3. **The BeingWarped flag (+0x271)** prevents the unit from being targeted, moving, etc.
   but does NOT directly affect rendering translucency.

### RulesClass Animation Offsets (CORRECTED, verified from raw byte analysis)
| Offset | Field | INI Key |
|--------|-------|---------|
| +0x328 | ChronoBlast AnimType | `ChronoBlast` |
| +0x32C | ChronoBlastDest AnimType | `ChronoBlastDest` |
| +0x334 | ChronoBeam AnimType | `ChronoBeam` |
| +0x338 | WarpIn AnimType | `WarpIn` |
| +0x33C | WarpOut AnimType | `WarpOut` |
| +0x340 | WarpAway AnimType | `WarpAway` |
| +0x344 | ChronoSparkle1 AnimType | `ChronoSparkle1` |

**Where spawned in TeleportLocomotionClass::InitiateWarp (0x00719400):**
- `AnimClass::Constructor(Rules+0x33C, &srcCoords)` — WarpOut at departure point
- `VocClass__PlayAt(...)` with TypeClass+0x578 (ChronoOutSound) or Rules+0x21C global fallback — per-unit warp-out **sound** at origin
- `VocClass__PlayAt(...)` with TypeClass+0x574 (ChronoInSound) or Rules+0x218 global fallback — per-unit warp-in **sound** at destination
- `AnimClass::Constructor(Rules+0x33C, &destCoords, 0, 1, 0x600, 0, 0)` — WarpOut at arrival point

So self-teleport spawns **only WarpOut** (Rules+0x33C, twice). WarpIn (+0x338),
WarpAway (+0x340), and ChronoSparkle1 (+0x344) are parsed, but they are not
referenced by these verified InitiateWarp constructor rows.

### Draw Flags and Translucency

The blitter selection in `Blitter_selector` (0x00490B90) uses draw flag bits:
```
Bits 1-2 (mask 0x6): Translucency level
  0x0 = opaque
  0x2 = 25% translucent
  0x4 = 50% translucent
  0x6 = 75% translucent

Bit 3 (0x8): Sub-variant selector within the ZReadWarp path (corrected 2026-05-28: was
  described as the primary selector for ZReadWarp blitters; binary shows this is WRONG.
  The primary ZReadWarp selector is mask 0x3000 — see below. Bit 0x8 is a secondary
  sub-selector only checked *after* the 0x3000 gate, choosing between two ZReadWarp
  sub-variants for the 25% and 75% translucency cases. ROOT_CAUSE: INFERENCE_HARDENED.
  Verified via decompile_function 0x00490B90 — all paths check _g_BlitterFlagMask_0x3000
  as primary gate, then check param_2 & 8 as sub-selector.)
Bit 4 (0x10): Z-buffer write
Bit 11 (0x800): Z-buffer read
Bit 12-13 (0x3000): XLat / ZReadWarp selector — this is the PRIMARY selector that
  routes to ZReadWarp blitters (named _g_BlitterFlagMask_0x3000 in binary). Within
  this path, bit 0x8 picks a sub-variant.
Bit 14 (0x4000): Alpha blend
```

The **ZReadWarp blitters** (e.g., `BlitTransLucent25ZReadWarp<unsigned_short>`) exist as
RTTI classes but are for the **temporal weapon erasing effect** (Chrono Legionnaire's
warping-away beam), NOT for chrono teleport. The temporal weapon gradually fades the
unit using the cloaking system while the ZReadWarp blitters handle the visual shimmer.

### The Cloaking System Drives Translucency

`TechnoClass_GetVisualState` (0x00703860) returns a cloak state (0-5):
- 0: Not cloaked/warping
- 1: 25% translucent (mapped to draw flag 0x2)
- 2: 50% translucent (mapped to draw flag 0x4)
- 3: 75% translucent (mapped to draw flag 0x6)
- 4: Nearly invisible
- 5: Fully invisible

This is driven by `CloakState` at TechnoClass+0x220 and `CloakProgress` at +0x224.

The **locomotor** can override this via `ILocomotion::Visual_Character` (vtable+0x34),
but TeleportLocomotionClass uses the base implementation which always returns 0 (not cloaked).
So **chrono teleporting units are drawn fully opaque** -- the verified
TeleportLocomotion constructor rows create WarpOut overlays at departure and arrival.

---

## 7. TechnoClass Field Map (Warp-Related, Verified)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x218 | 4 | GhostCell (CellClass*) | Building-exit occupation bookkeeping (vtable+0x480 dispatch via `BuildingClass__ExitObject_Main`); NOT a render field — negative proof, 2026-07-19, see Section 1 point 4 |
| +0x21C | 4 | OwnerHouse (HouseClass*) | Set in constructor |
| +0x220 | 4 | CloakState | 0=uncloaked, 1=cloaking, 2=cloaked, 3=uncloaking |
| +0x224 | 4 | CloakProgress | Counter for cloak animation |
| +0x240 | 4 | Timer_240.StartFrame | CDTimerClass start frame |
| +0x244 | 4 | Timer_240.field_4 | CDTimerClass unused middle field |
| +0x248 | 4 | Timer_240.Duration | CDTimerClass duration |
| +0x254 | 4 | ChronoSourceCoord.X | Init: NullCoord |
| +0x258 | 4 | ChronoSourceCoord.Y | Init: NullCoord |
| +0x25C | 4 | ChronoSourceCoord.Z | Init: NullCoord |
| +0x270 | 1 | WarpingOut | Set by temporal weapon and chrono transit |
| +0x271 | 1 | BeingWarped | Set during teleport sequence |
| +0x27C | 1 | ChronoInTransit | Set by ChronoSphere handler |
| +0x280 | 4 | PendingWarpPhase | Set to 3 by ChronoSphere, read by locomotor |
| +0x284 | 4 | ChronoLockDuration | ChronoDelay value during warp |
| +0x288 | 4 | ChronoDestCoord.X | Set by ChronoWarp handler |
| +0x28C | 4 | ChronoDestCoord.Y | Set by ChronoWarp handler |
| +0x290 | 4 | ChronoDestCoord.Z | Set by ChronoWarp handler |
| +0x428 | 4 | ChronoSourceBuilding | Building that initiated warp |
| +0x42C | 4 | ChronoSourceHouse | House that owns warp source |

---

## 8. Functions Labeled in Ghidra

| Address | Name | Description |
|---------|------|-------------|
| 0x0070C610 | TechnoClass__SetGhostCell | Sets +0x218 deploy preview cell |
| 0x0070C5C0 | TechnoClass__IsBeingWarped | Returns +0x271 |
| 0x0070C5F0 | TechnoClass__IsNotWarping | Checks +0x270 and +0x271 both zero |
| 0x006F5190 | TechnoClass__DrawExtras | Draws CHRONOSK.SHP, brackets, wrench |
| 0x004DA4E0 | FootClass__GetVisualState | Delegates to locomotor then TechnoClass |
| 0x00703860 | TechnoClass_GetVisualState | Returns cloak state 0-5 |
| 0x0070ED80 | TechnoClass__ModifyCloakDrawFlags | Adds translucency bits for cloaking |
| 0x007197D0 | TeleportLocomotion__Phase0_SetWarpingOut | Sets +0x270=1, starts timer |
| 0x0071AF20 | TemporalClass__InitiateWarp | Temporal weapon warp initiation |
| 0x0055ABC0 | LocomotionClass__Visual_Character | Base impl, always returns 0 (corrected 2026-05-28: was listed as ILocomotion__Visual_Character_Default; Ghidra label is LocomotionClass__Visual_Character, confirmed via get_function_by_address 0x0055abc0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x00438A00 | IvanBomb__GetClockFrame | Returns CHRONOSK.SHP frame (0-12) |
| 0x0070E5A0 | TechnoClass__UpdateTemporalVisual | 10-phase temporal erasure visual state machine |
| 0x0070E4B0 | TechnoClass__ScaleByWarpInVisualPhase | Gap-generator visual scale, not TeleportLocomotion warp |
| 0x00490B90 | Blitter_selector | Flag bits → blitter class (ZReadWarp for temporal) |
| 0x00490E50 | Blitter_selector_extended | Alpha/RLE variant selection |

### String Labels Created
| Address | Label |
|---------|-------|
| 0x0083B0E0 | str_CHRONOSK_SHP |
| 0x0083CD64 | str_ChronoSparkle1 |
| 0x0083CDB8 | str_WarpAway |
| 0x0083CDC4 | str_WarpOut |
| 0x0083CDCC | str_WarpIn |
| 0x0083CDF0 | str_ChronoBeam |

---

## 9. Summary of Corrections

| Claim | Status | Correction |
|-------|--------|------------|
| +0x244 is a WarpFactor float | **WRONG** | CDTimerClass unused middle field |
| +0x218 is WarpFactor or anim state | **WRONG** | CellClass pointer; "deploy ghost render" narrative **REFUTED** 2026-07-19 by negative proof — confirmed role is building-exit bookkeeping (vtable+0x480), not rendering — see Section 1 |
| +0x268 is BeingWarpedOut (v1) | **WRONG** | v2 offsets are correct |
| +0x270 is WarpingOut (v2) | **CORRECT** | Verified from binary |
| +0x271 is BeingWarped (v2) | **CORRECT** | Verified from accessor |
| Units become translucent during warp | **PARTIALLY WRONG** | Chrono teleport uses animation overlays, not unit translucency |

---

## 10. TeleportLocomotionClass State Machine (Full Detail)

The `StateMachineTick` at `0x007192F0` manages all chrono warp phases:

### Pre-checks
- If BeingWarped(+0x271) set AND state==0 AND PendingWarpPhase(+0x280)==0: call TimerCheck, return
- If state==0 AND PendingWarpPhase!=0: set state = PendingWarpPhase (ChronoSphere sets to 3)

### ChronoInTransit Path (ChronoSphere-initiated warp)

| State | Action |
|-------|--------|
| 0 | Set WarpingOut(+0x270)=1, timer=60 frames, advance to 1 |
| 1 | Wait for timer via TimerCheck |
| 2 | Spawn WarpOut anim (Rules+0x33C), unmark occupation, set BeingWarped=1, clear ChronoInTransit+WarpingOut, teleport to ChronoDestCoord (+0x288/28C/290). Advance to 3 (or 4 if Update_Position returns true) |
| 3 | Move to dest, set ChronoLockDuration(+0x284) = Rules->ChronoDelay (+0xBEC), advance to 4 |
| 4 | Move to dest, mark occupation at new pos, advance to 5 |
| 5 | Post-warp validation, clear +0x428/+0x42C, set timer from ChronoLockDuration, spawn WarpOut anim (Rules+0x33C), advance to 6 |
| 6 | Wait for timer via TimerCheck |
| 7 | Clear BeingWarped(+0x271), clear GhostCell, clear PendingWarpPhase, reset state to 0 |

### Normal Teleport Path (Chrono Miner)

State 0 only. `Process` finds valid dest → `InitiateWarp`:
1. Spawn WarpOut anim at source coords
2. Calculate distance, set timer = distance / ChronoDistanceFactor
3. Enforce minimum delay (Rules+0xBFC) and range check (Rules+0xC00)
4. Set BeingWarped(+0x271) = 1
5. **SPECIAL: If UnitClass AND Harvester=yes (type+0xE0E):** timer=0, BeingWarped=0 (instant!)
6. Unmark source, mark dest, set facing, spawn WarpOut anim
7. Clear PendingWarpPhase, done in one tick

### RulesClass Chrono Timing Offsets
| Offset | INI Key | Type | Purpose |
|--------|---------|------|---------|
| +0xBEC | ChronoDelay | int | Base delay for chronoshift (frames) |
| +0xBF0 | ChronoReinfDelay | int | Chrono reinforcement delay |
| +0xBF4 | ChronoDistanceFactor | int | Distance divisor for warp duration |
| +0xBF8 | ChronoTrigger | bool | Enable distance-based delay |
| +0xBFC | ChronoMinimumDelay | int | Minimum warp duration floor |
| +0xC00 | ChronoRangeMinimum | int | Below this distance, use minimum delay |

---

## 11. Temporal Erasure Visual System (Chrono Legionnaire)

Fundamentally different from chrono teleport. Uses ZReadWarp blitters, NOT animations.

### TemporalClass::InitiateWarp (0x0071AF20)
- No AnimClass objects spawned — visual is entirely blitter-driven
- Warp points = target TypeClass->Strength × 10
- If target already has temporal attached: linked list via +0x40/+0x44
- Sets WarpingOut(+0x270)=1 on target

### TechnoClass::UpdateTemporalVisual (0x0070E5A0)
10-phase state machine at TechnoClass offsets +0x198(StartFrame)/+0x19C(unused local)/
+0x1A0(Duration)/+0x1A4(Phase) — offsets confirmed via `decompile_function 0x0070E5A0`
(`param_1[0x66]`=+0x198, `param_1[0x67]`=+0x19C, `param_1[0x68]`=+0x1A0, `param_1[0x69]`=+0x1A4).

**Corrected 2026-07-12:** `decompile_function 0x0070E5A0` shows this function contains ONLY
phase-index/duration-timer transitions — it computes NO visual scale/alpha value in any case
branch. The "Visual Effect" formulas previously listed below do **not** appear in this
function. They loosely resemble (but don't match) formulas in the unrelated
`TechnoClass::ScaleByWarpInVisualPhase` (0x0070E4B0, `decompile_function 0x0070E4B0`) —
e.g. that function's case 3/9 is `(remaining+20)*256/20`, not the `(remaining*0x1CD+0x3FC)/20`
claimed here for phase 3; its case 4 is `(128-remaining)*256/64`, not `remaining*-0x4D+0x400`.
ROOT_CAUSE: INFERENCE_HARDENED — formulas were invented/misattributed, not read from this
function. The Duration column below IS confirmed accurate against the decompile. Also
previously undocumented: if phase 5's 16-frame duration elapses and
`CDTimerClass__Remaining() >= 0x36`, the state machine does **not** advance to phase 6 — it
loops back to phase 4 (duration 8) and retries phases 4→5 until the condition is met.

| Phase | Duration | Visual Effect |
|-------|----------|---------------|
| 0 | instant | Transition to phase 1 |
| 1 | 6 frames | UNVERIFIED (formula removed; not present in this function) |
| 2 | 4 frames | UNVERIFIED (formula removed; not present in this function) |
| 3 | 20±5 frames (`RandomRanged(-5,5)+20`) | UNVERIFIED (formula removed; not present in this function) |
| 4 | 8 frames | UNVERIFIED (formula removed; not present in this function) |
| 5 | 16 frames | Exit gated on `CDTimerClass__Remaining() < 0x36`; if not met, loops back to phase 4 instead of advancing (not previously documented) |
| 6 | waits | Waits for `CDTimerClass__Remaining() < 0x1F` |
| 7 | 6 frames | UNVERIFIED (formula removed; not present in this function) |
| 8 | 4 frames | UNVERIFIED (formula removed; not present in this function) |
| 9 | 20 frames | UNVERIFIED (formula removed; not present in this function) |
| 10 | terminal | Unit erased |

### TechnoClass::ScaleByWarpInVisualPhase (0x0070E4B0)
Simpler 6-phase system at +0x1B4/+0x1BC/+0x1C0. Fresh verification treats this
as the gap-generator visual phase, not the TeleportLocomotion warp visual path:

| Phase | Visual Effect |
|-------|---------------|
| 1, 7 | Fade in: `(12 - remaining) * 256 / 6` |
| 2, 8 | Full shimmer: 0x200 |
| 3, 9 | Fade out: `(remaining + 20) * 256 / 20` |
| 4 | `(128 - remaining) * 256 / 64` |
| 5 | `(remaining + 64) * 256 / 64` |
| 6 | 0x100 (normal) |

### ZReadWarp Blitters (12 variants)
Selected via `Blitter_selector` (`0x00490B90`) when draw flag mask 0x3000 (`_g_BlitterFlagMask_0x3000`) is set; within that path, bit 0x8 sub-selects a variant for 25% and 75% cases (confirmed via decompile_function 0x00490B90):

| Translucency | Plain | Alpha | RLE | RLE+Alpha |
|-------------|-------|-------|-----|-----------|
| 25% | BlitTransLucent25ZReadWarp | + Alpha | + RLE | + RLE+Alpha |
| 50% | BlitTransLucent50ZReadWarp | + Alpha | + RLE | + RLE+Alpha |
| 75% | BlitTransLucent75ZReadWarp | + Alpha | + RLE | + RLE+Alpha |

### Key Difference Summary
| Aspect | Chrono Teleport | Temporal Erasure |
|--------|----------------|------------------|
| Visual source | TeleportLocomotion AnimClass rows use WarpOut; temporal/wake rows are separate | ZReadWarp blitters during draw |
| Phase system | TeleportLocomotion state rows spawn WarpOut; ScaleByWarpInVisualPhase is not this path | UpdateTemporalVisual (10 phases) |
| TechnoClass fields | none confirmed (corrected 2026-07-12: this row previously listed +0x1B4/+0x1BC/+0x1C0 under "Chrono Teleport," contradicting this doc's own text two paragraphs above stating those are `ScaleByWarpInVisualPhase`'s gap-generator fields, not TeleportLocomotion's. Re-verified via `decompile_function 0x0070E4B0`: that function reads `param_1+0x1b4/0x1bc/0x1c0` and is called only from `TechnoClass_DrawSHP`/`TechnoClass__Draw`, never from TeleportLocomotionClass. Per Section 6, chrono teleport has no per-unit visual-phase field — the unit is drawn opaque, warp is animation-overlay only.) | +0x198/+0x19C/+0x1A0/+0x1A4 |
| Duration | Distance/ChronoDistanceFactor | TypeClass->Strength × 10 warp points |
| End result | Unit moves to new location | Unit erased |
