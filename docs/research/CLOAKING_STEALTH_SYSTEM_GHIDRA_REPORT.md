# Cloaking and Stealth Detection System -- Comprehensive Ghidra Report

**Source:** Live Ghidra decompilation of `gamemd.exe`, consolidating and extending
findings from `CLOAKING_VISUAL_PIPELINE.md`, `CLOAKING_INTERACTIONS_REPORT.md`, and
`SENSOR_CLOAK_DETECTION.md`.

**Confidence:** HIGH for all offset tables and state machine logic (verified from binary).
MEDIUM for some disguise rendering details (inferred from field offsets and vtable dispatch).

---

## Table of Contents

1. [Cloak State Machine](#1-cloak-state-machine)
2. [Cloak Triggers (What Causes Cloaking)](#2-cloak-triggers)
3. [Decloak Triggers (What Forces Uncloaking)](#3-decloak-triggers)
4. [Detection Mechanics](#4-detection-mechanics)
5. [Gap Generator System](#5-gap-generator-system)
6. [Disguise System](#6-disguise-system)
7. [Visual Rendering](#7-visual-rendering)
8. [Key Struct Field Tables](#8-key-struct-field-tables)
9. [INI Keys Reference](#9-ini-keys-reference)
10. [Implementation Notes](#10-implementation-notes)

---

## 1. Cloak State Machine

### CloakState Enum (TechnoClass+0x220, DWORD)

```
0 = Uncloaked     -- fully visible, no cloaking active
1 = Cloaking      -- fade-out animation in progress (CloakProgress counting UP)
2 = Cloaked       -- fully invisible to enemies
3 = Uncloaking    -- fade-in animation in progress (CloakProgress counting DOWN)
```

### State Transition Graph

```
                  StartCloaking()           fully done (visual>=5)
    [0 Uncloaked] -----------------> [1 Cloaking] -------------------> [2 Cloaked]
         ^                              |                                  |
         |                              | damaged + random 10%,            | ShouldUncloak()
         |                              | sensor detected,                 | returns true
         |                              | fire command                     |
         |                              v                                  v
         |                         StartUncloaking()                 StartUncloaking()
         |                              |                                  |
         |        fully done            v                                  v
         <---------------------- [3 Uncloaking] <--------------------------+
```

Cross-transitions allowed:
- State 1->3: `StartUncloaking()` while mid-cloak (e.g., forced to fire)
- State 2->3: `StartUncloaking()` when fully cloaked and detected/ordered to fire
- State 3->1: `StartCloaking()` can restart cloaking during uncloak (at visual state 1)
- State 0->2: Direct assignment when exiting transport (bypasses animation)

### CloakingTick -- 0x006FB740 (called from TechnoClass::AI every tick)

**State 0 (Uncloaked) path:**
1. Check `IsCloakable()` (vtable+0x288) -- if false, check veteran/elite CLOAK ability
2. Check `IsFiring()` (vtable+0x37C), `IsTeleporting()` (vtable+0x380), `IsWarpingIn()` (vtable+0x1D4), `IsWarpingOut()` (vtable+0x1D8) -- if any true, skip unless veteran/elite has CLOAK
3. Check destination is not a WeaponsFactory building (prevents cloak during dock)
4. Advance CloakStepTimer if applicable
5. Check `CanAutoCloak()` (vtable+0x2A0) -- full eligibility check
6. Health check: if below `ConditionRed` (RulesClass+0x1708), only 4% chance per tick to start cloaking
7. If all checks pass: call `StartCloaking(0)` (vtable+0x460)

**State 1 (Cloaking) path:**
1. Call `ObjectClass::Mark(2)` (vtable+0x124) -- hide/reveal in overlap list (see vtable correction below)
2. Advance CloakProgress timer
3. Get visual state via `GetVisualState(1,0)` (vtable+0x68)
4. If visual == 2 AND health < ConditionRed (RulesClass+0x1708): 10% chance per tick to abort -> `StartUncloaking(1)`
5. If visual == 3 or 5 (transition complete):
   - Set CloakState = 2, reset CloakProgress/CloakingSpeed to 0
   - **MCV deploy check:** if UnitClass with deploy target (+0x6CC != -1), force uncloak
   - **Mind control scatter:** iterate g_TechnoClass_Array, find all units with MindControllerPtr == this, scatter them (but do NOT release MC link)
   - Limbo self after scatter

**State 2 (Cloaked) path:**
1. Call `ShouldUncloak()` (vtable+0x2A4) each tick
2. If true: call `StartUncloaking(0)` (vtable+0x45C)

**State 3 (Uncloaking) path:**
1. Call `ObjectClass::Mark(2)` (vtable+0x124)
2. Advance CloakProgress timer (counting DOWN)
3. Get visual state
4. If visual == 0 (fully uncloaked):
   - Reset CloakState = 0, CloakProgress = 0, CloakingSpeed = 0
   - Start `ReCloakDelayTimer` = CloakDelay * 900 ticks (RulesClass+0x1410, default 0.02 min = 18 ticks at 15fps)
5. If visual == 1 (almost uncloaked) AND `CanAutoCloak()`: immediately restart cloaking

### Transition Timer Mechanics

- **CloakProgress** (+0x224): integer counter. Counts UP by +1 each step when cloaking, DOWN by -1 when uncloaking.
- **CloakStepTimer** (+0x22C): CDTimerClass (12 bytes). When expired, advances CloakProgress by CloakStepDelta.
- **CloakingSpeed** (+0x238): DWORD copied from TechnoTypeClass+0x310. Frames between each CloakProgress step. Lower = faster cloaking.
- **CloakStepDelta** (+0x23C): +1 during cloaking, -1 during uncloaking.
- **ReCloakDelayTimer** (+0x240/+0x248): CDTimerClass (start-frame at +0x240, duration at +0x248). After fully uncloaking (state 3->0), `CloakingTick` computes `CloakDelay * 900` (RulesClass+0x1410) and stores it here — verified via `disassemble_function 0x006FB740` (writes at 0x6fb9f8-0x6fba0c: `LEA EDX,[ESI+0x240]`, then `[EDX]`=current frame, `[EDX+8]`=computed duration). `CanAutoCloak` reads this same pair as `param_1[0x90]` (=+0x240) / `param_1[0x92]` (=+0x248) and blocks re-cloaking until it expires — verified via `disassemble_function 0x006FBDC0` (0x6fbf2b-0x6fbf56). (corrected 2026-07-12: this field was previously mislabeled "SecondaryGatingTimer, purpose TBD" and the +0x2EC field below was called "ReCloakDelayTimer" — swapped. The original pass never traced the setter; this session did — OFFSET_RETYPED_WRONG / INFERENCE_HARDENED.)
- **Unidentified timer** (+0x2EC/+0x2F4): A distinct CDTimerClass (start-frame at +0x2EC, duration at +0x2F4), gated FIRST in `CanAutoCloak` at `param_1[0xBB]`/`param_1[0xBD]` — checked immediately after the `CloakState != 2` test, before the ReCloakDelayTimer check above. Verified via `disassemble_function 0x006FBDC0` (0x6fbebe-0x6fbeea). No setter for this field was found this session in `CloakingTick`, `StartCloaking`, `StartUncloaking`, `DoCloak`, or `DoUncloak` — purpose remains genuinely TBD. (corrected 2026-07-12: this field was previously mislabeled "ReCloakDelayTimer"; it is NOT the CloakDelay-driven timer — that is +0x240/+0x248 above.)

### CloakProgress to Visual State Mapping (CloakingStages=9)

The visual state is computed in `GetVisualState` (0x00703860):
```
visual = (int)((double)CloakProgress / (double)CloakingStages * 256.0)
```

| CloakProgress | Computed Visual | Visual State | Meaning |
|---------------|-----------------|--------------|---------|
| 0 | 0 | 0 | Fully opaque |
| 1 | 28 | 1 | Light shimmer |
| 2 | 56 | 1 | Light shimmer |
| 3 | 85 | 2 | Heavier distortion |
| 4 | 113 | 2 | Heavier distortion |
| 5 | 142 | 2 | Heavier distortion |
| 6 | 170 | 2 | Heavier distortion |
| 7 | 199 | 3 | Semi-transparent |
| 8 | 227 | 4 | Near-invisible |
| 9+ | 256+ | 5 | Fully invisible -> triggers state 1->2 |

Visual state thresholds: <0x40=1, <0x80=2, <0xC0=3, <0xFF=4, >=0xFF=5.

### Key Functions

| Address | Name | Description |
|---------|------|-------------|
| 0x006FB740 | TechnoClass::CloakingTick | Per-tick state machine driver |
| 0x007036C0 | TechnoClass::StartUncloaking | Transition to state 3 (vtable+0x45C) |
| 0x00703770 | TechnoClass::StartCloaking | Transition to state 1 (vtable+0x460) |
| 0x00703860 | TechnoClass::GetVisualState | Compute visual state 0-5 (vtable+0x68) |
| 0x004D3780 | TechnoClass::DoCloak | Wrapper with trigger events |
| 0x006F4EB0 | TechnoClass::DoUncloak | Wrapper with discovery/MC handling |
| 0x006FBDC0 | TechnoClass::CanAutoCloak | Full eligibility check (vtable+0x2A0) |
| 0x006FBC90 | TechnoClass::ShouldUncloak | Decision for cloaked units (vtable+0x2A4) |
| 0x005F5850 | ObjectClass::Mark | Hide/reveal in overlap list (called with arg 2 during cloak transitions). Not TechnoClass::ProcessCloakMode — verified via `get_function_by_address 0x5F5850` |
| 0x006F4A70 | TechnoClass::ProcessCloakAndNotify | ProcessCloakMode + radar update |
| 0x004DBDA0 | FootClass::IsCloakable | Check HasStealthAbility + CloakStop (vtable+0x288) |

---

## 2. Cloak Triggers

### Sources of Cloaking Ability

**A. Innate type flag (`Cloakable=yes`)**
- INI key `Cloakable=` parsed to TechnoTypeClass+0xCD0 (bool)
- Copied to TechnoClass+0x3D2 (`HasStealthAbility`) during construction:
  - UnitClass constructor (0x7355B6): `this+0x3D2 = UnitTypeClass->Cloakable`
  - InfantryClass constructor (0x517D88): `this+0x3D2 = InfantryTypeClass->Cloakable`
  - AircraftClass/BuildingClass do NOT copy; they use vtable+0x288 override instead

**B. Veterancy promotion**
- `VeteranAbilities=CLOAK` -> TechnoTypeClass+0x2A2 (bool), index 6 in abilities array
- `EliteAbilities=CLOAK` -> TechnoTypeClass+0x2B4 (bool), index 6
- CloakingTick checks these at runtime: veteran/elite units with CLOAK ability can cloak even while firing/moving
- Infantry only checks VeteranAbilities[CLOAK]; vehicles check BOTH veteran AND elite

**C. Crate pickup**
- `CrateClass::PickupCloak` (0x0048294F): sets TechnoClass+0x3D2 = 1
- Runtime-only modification; does not change the TypeClass

**D. CloakStop gate (`CloakStop=yes`)**
- TechnoTypeClass+0xC93 (bool)
- `FootClass::IsCloakable()` at 0x4DBDA0: if CloakStop is set AND locomotion is busy (moving), returns false
- Effect: units like Mirage Tank can only cloak when stationary

**E. Near a cloak-generating structure**
- Buildings with `CloakGenerator=yes` (BuildingTypeClass+0x16C7) provide area cloaking
- NOT a per-unit cloaking grant; instead uses the Gap Generator / CloakShroud cell system
- See section 5 (Gap Generator)

### Auto-Cloak Eligibility -- CanAutoCloak (0x006FBDC0)

Returns true when ALL of these hold:
1. Unit has cloaking capability (IsCloakable OR veteran/elite CLOAK ability)
2. Cell is visible to owning house, or HasStealthAbility is true
3. CloakState != 2 (not already fully cloaked)
4. ReCloakDelayTimer has expired
5. No enemy actively targeting this unit
6. CloakProgress == 0 (for non-buildings)
7. Unit is not deployed+selected in a specific state
8. `vtable+0x1C8()` returns < 1 (mission/locomotion state check)

---

## 3. Decloak Triggers

### What Forces Decloaking

**A. Firing a weapon with `DecloakToFire=yes`**
- WeaponTypeClass+0x133 (bool), INI key `DecloakToFire=`
- Default is YES (most weapons decloak). `DecloakToFire=no` is explicit opt-out.
- Checked in `TechnoClass::GetFireError` (0x006FC0B0):
  - If weapon->DecloakToFire AND CloakState != 0: return FIRE_MUST_DECLOAK (9)
  - Exception: aircraft (WhatAmI==2) in transition states (1 or 3) CAN fire; only fully cloaked (state 2) aircraft must decloak. Non-aircraft (including UnitClass, WhatAmI==1) get NO transition exception — any CloakState!=0 forces FIRE_MUST_DECLOAK. (corrected 2026-07-12: was "vehicles" — WhatAmI==2 is AircraftClass, not UnitClass/vehicles. Verified `UnitClass::WhatAmI` returns 1 (`read_memory 0x00746e20` = `B8 01 00 00 00 C3`) and `AircraftClass::WhatAmI` returns 2 (`read_memory 0x0041c180` = `B8 02 00 00 00 C3`), resolved via `vtable__UnitClass`@0x7f5c70+0x2c and `vtable__AircraftClass`@0x7e22a4+0x2c — RTTI_LABEL_DRIFT. This means a cloaked Mirage Tank, a UnitClass, does NOT get to fire mid-transition.)
- On receiving FIRE_MUST_DECLOAK, caller triggers `StartUncloaking()` and retries next tick

**B. Sensor detection**
- When a unit with `SensorsSight > 0` covers a cell, it calls `DoUncloak()` on all units in that cell
- `DoUncloak()` (0x006F4EB0) forces `StartUncloaking()` if the unit is cloaked

**C. ShouldUncloak check (per-tick for state 2)**
- `ShouldUncloak()` (0x006FBC90) returns true if:
  - Unit has no cloaking ability (IsCloakable false AND no veteran/elite CLOAK)
  - Unit is actively firing/teleporting/warping (unless veteran/elite with CLOAK)
  - Cell is not visible to owner house (cloaked unit in fog -> uncloak)

**D. Taking damage (indirect)**
- Damage does not directly trigger decloaking
- BUT: damaged units below `ConditionRed` (RulesClass+0x1708) health have reduced auto-cloak chance:
  - State 0: only 4% chance per tick to START cloaking
  - State 1: 10% chance per tick to ABORT cloaking and start uncloaking
- Net effect: damaged units decloak probabilistically

**E. Entering a transport (Limbo)**
- CloakState is NOT reset by Limbo. The value persists in memory.
- However, since the unit is off-map, CloakingTick effectively does nothing.

**F. Exiting a transport (Unlimbo)**
- `UnitClass::Unlimbo` (0x737BA0) at 0x737BEB:
  ```c
  if (HasStealthAbility && !field_0x3D5) {
      CloakState = 2;  // Instantly fully cloaked, skip animation
  }
  ```
- Cloakable units exiting transports appear immediately invisible.

**G. Chronoshift / Teleport**
- While IsWarpingIn (+0x270) or IsWarpingOut (+0x271) is set, auto-cloaking is blocked
- CloakingTick explicitly checks these flags in the state 0 path
- Flags are cleared when warp animation completes

**H. Mind control scatter**
- When a unit transitions 1->2 (becomes fully cloaked) and has mind-controlled subjects:
  - Subjects are scattered to adjacent cells (positional only)
  - MC link is NOT broken; subjects remain mind-controlled
  - Cloaking unit Limbos itself after scatter

---

## 4. Detection Mechanics

### 4.1 Sensor Sight (Unit-Level Detection)

**INI key:** `SensorsSight=<int>` (TechnoTypeClass+0x5F0)

Units with SensorsSight > 0 project a circular sensor field that forces cloaked enemies to uncloak.

**Cell-level counters:** CellClass+0x7C = `short SensorCount[MaxHouses]` (per-house)

| Function | Address | Vtable | Description |
|----------|---------|--------|-------------|
| AddSensorsAt | 0x004DE7B0 | +0x4E8 | Increment sensor counts in radius, DoUncloak all units |
| RemoveSensorsAt | 0x004DE940 | +0x4EC | Decrement sensor counts, re-check visibility |

Called on: unit placement (Unlimbo), unit movement (remove old + add new), unit removal.

Range check: `dx*dx + dy*dy < SensorsSight*SensorsSight` (circular, no +1). Verified via `decompile_function 0x4DE7B0` and `decompile_function 0x4DE940`.

When sensor count for a house becomes > 0 on a cell, all cloaked enemies in that cell are forced to uncloak via `DoUncloak()`.

### 4.2 Sensor Array (Building-Level Detection)

**INI key:** `SensorArray=yes` (BuildingTypeClass+0x16C8)

Buildings with SensorArray use the SAME cell counter system (CellClass+0x7C) as unit sensors.

| Function | Address | Vtable | Description |
|----------|---------|--------|-------------|
| BuildingClass::AddSensorArrayAt | 0x00455820 | +0x4F4 | Uses TechnoTypeClass::SensorsSight as range |
| BuildingClass::RemoveSensorArrayAt | 0x004556D0 | +0x4F8 | Uses BuildingTypeClass::CloakRadiusInCells as range |

**Note:** Both add and remove functions use the SAME range source: `SensorsSight` at TypeClass+0x5F0. `RemoveSensorsAt` does NOT use `CloakRadiusInCells` — it reads TypeClass+0x5F0 identically. Verified via `decompile_function 0x4DE940`.

Includes a power check (vtable+0x350): sensor array only works when building is powered.

### 4.3 Psychic Detection

**INI key:** `PsychicDetectionRadius=<int>` (BuildingTypeClass+0x170C)

- Read from INI in `BuildingTypeClass_ReadINI_Water` at 0x00460C39
- Used in `BuildingClass::GetSensorRange` (0x004566B0):
  ```c
  if (PsychicDetectionRadius >= 1) return PsychicDetectionRadius;
  ```
- Takes priority over GapGenerator and SensorArray ranges in the sensor range calculation
- In practice, used by the Psychic Sensor building (PSYCHIC sensor, `PsychicDetectionRadius=15`)

### 4.4 Disguise Detection

**INI keys:**
- `DetectDisguise=yes` (TechnoTypeClass+0xD31, bool) -- unit can detect disguised enemies
- `DetectDisguiseRange=<int>` (TechnoTypeClass+0x5F4) -- range in cells

**Separate cell counter:** CellClass+0xAC = `short DisguiseDetectCount[MaxHouses]` (per-house)

| Function | Address | Vtable | Description |
|----------|---------|--------|-------------|
| AddDetectDisguiseAt | 0x00455A80 | +0x4FC | Increment disguise detect counts |
| RemoveDetectDisguiseAt | 0x00455980 | +0x500 | Decrement disguise detect counts |

**Key difference from sensor detection:** Disguise detection does NOT call DoUncloak. It only reveals the true identity of disguised units within range; it does not force cloaked units to uncloak.

### 4.5 Three Separate Detection Systems Summary

| System | INI Keys | Cell Array Offset | Effect on Target |
|--------|----------|-------------------|------------------|
| **Sensor Sight** | `SensorsSight=<int>` | CellClass+0x7C | Forces DoUncloak |
| **Sensor Array** | `SensorArray=yes` + `SensorsSight=` | CellClass+0x7C | Forces DoUncloak |
| **Disguise Detect** | `DetectDisguise=yes` + `DetectDisguiseRange=` | CellClass+0xAC | Reveals true identity only |

Sensor sight and sensor arrays share the same cell counter (+0x7C). Disguise detection has its own (+0xAC).

---

## 5. Gap Generator System

### Overview

Gap Generators re-shroud an area for enemy players. They use a cell-based counter system separate from the sensor counters.

**INI keys:**
- `GapGenerator=yes` (TechnoTypeClass+0xCD1, bool) -- note: NOT the same offset as Cloakable
- `GapRadiusInCells=<int>` (TechnoTypeClass+0xCD2, byte)
- `SuperGapRadiusInCells=<int>` (TechnoTypeClass+0xCD3, byte)

**Building-specific:**
- `CloakGenerator=yes` (BuildingTypeClass+0x16C7, bool) -- building generates cloak field
- `CloakRadiusInCells=<int>` (BuildingTypeClass+0x1707, byte)

### Cell-Level Gap Counters

| CellClass Offset | Type | Purpose |
|-------------------|------|---------|
| +0x130 | int | Gap shroud level (>0 = reshrouded for that player) |
| +0x134 | int | Gap generator overlay count |
| +0x13C | int | Allied gap exclusion count |
| +0x12C (300) | uint | Shroud flags (bit 0x08 = needs reveal, bit 0x10 = needs redraw) |

### UpdateCloakShroud -- 0x006FB170

Called when a gap generator is placed. Applies shroud to cells in radius:

```c
void TechnoClass::UpdateCloakShroud() {
    if (PlayerPtr == NULL || CloakShroudActive || !IsPowered()) return;

    radius = CloakShroudRadius;  // cached from TypeClass+0xCD2
    if (radius == 0) {
        radius = TypeClass->GapRadiusInCells;
        CloakShroudRadius = radius;
    }

    CloakShroudActive = 1;  // +0x269
    center = GetCoords() -> cell coords;

    for dy in (-radius-2 .. radius+2):
        for dx in (-radius-2 .. radius+2):
            if (dx*dx + dy*dy < (radius+1)^2):  // circular
                // For ENEMY players (not owner, not allied):
                cell->GapShroudLevel++;     // +0x130
                cell->GapOverlayCount++;    // +0x134
                if (GapShroudLevel > 0):
                    cell->ShroudFlags &= ~0x10;  // clear redraw flag
                    cell->ShroudFlags &= ~0x08;  // clear reveal flag

                // For ALLIED players (owner or allied):
                cell->AlliedGapExclusion++;  // +0x13C

    PlayerPtr->field_0x240 = 0;
    RefreshRadar();
}
```

### RemoveCloakShroud -- 0x006FB470

Called when a gap generator is destroyed or loses power. Reverses the shroud:

```c
void TechnoClass::RemoveCloakShroud() {
    if (PlayerPtr == NULL || !CloakShroudActive) return;

    CloakShroudActive = 0;
    // Same radius iteration as UpdateCloakShroud

    for each cell in radius:
        // For ENEMY players:
        cell->GapOverlayCount--;      // +0x134
        if (PlayerPtr->field_0x577A && GapOverlayCount <= 0):
            cell->GapShroudLevel--;   // +0x130
            if (GapShroudLevel <= 0):
                cell->ShroudFlags |= 0x08;   // mark for reveal
                cell->ShroudFlags |= 0x10;   // mark for redraw

        // For ALLIED players:
        cell->AlliedGapExclusion--;   // +0x13C

    RefreshRadar();
}
```

### Building Gap Generator Tick -- 0x00454DB0

Per-tick update for buildings with CloakGenerator=yes. Manages a building-specific cloaking animation:

**BuildingClass fields:**
- +0x220 (inherited) = BuildingCloakPhase: 0=uncloaked, 1=cloaking_in, 2=fully_cloaked, 3=uncloaking
- +0x6ED = BuildingCloakStage: byte counter 0-15+, used for visual stage
- +0x660 = CloakAnimDirection: 0=cloaking_in, 1=cloaking_out

**State machine:**
- **Phase 1 (cloaking_in):** BuildingCloakStage increments from 0 to 15, one step per tick
  - At stages 1, 6, 11: mark for redraw
  - At stage 15: if GetVisualState returns 5, set stage to 16
  - Update all 21 animation slots (+0x55C, 0x15 entries) with current stage
  - When stage reaches 15: transition to Phase 2 (fully_cloaked)
  - Destroy associated particle system on full cloak

- **Phase 3 (uncloaking):** BuildingCloakStage decrements from current to 0
  - At stages 0, 5, 10: mark for redraw
  - When stage reaches 0: transition to Phase 0 (uncloaked)
  - Create new particle system (BuildingTypeClass+0x764)

- **Phase 2 (fully_cloaked):** check ShouldUncloak (vtable+0x2A4); if true, StartUncloaking
- **Phase 0 (uncloaked):** check CanAutoCloak (vtable+0x2A0); if true, StartCloaking

### Gap Generator Visual State (BuildingClass override)

`BuildingClass::GetVisualState` (0x004544A0) uses BuildingCloakStage instead of CloakProgress:
```
stage 0:     normal GetVisualState path
stage 1-5:   return 1 (shimmer)
stage 6-10:  return 2 (semi-transparent)
stage >= 11: complex visibility checks -> return 3 (allied) or 5 (enemy invisible)
```

---

## 6. Disguise System

### Overview

Disguise and cloaking are **completely independent systems**. They use different fields, different vtable entries, and different state machines. A unit CAN be simultaneously cloaked AND disguised (e.g., Mirage Tank has both `Cloakable=yes` and `DisguiseWhenStill=yes`).

- **Disguise** determines WHAT the unit appears as to enemies
- **Cloaking** determines WHETHER the unit is visible at all

### TechnoTypeClass Disguise Fields

| Offset | Size | INI Key | Type | Description |
|--------|------|---------|------|-------------|
| +0xD2F | 1 | `CanDisguise=` | bool | Unit can disguise (Spy) |
| +0xD30 | 1 | `PermaDisguise=` | bool | Always disguised (Spy has this) |
| +0xD31 | 1 | `DetectDisguise=` | bool | Can detect disguised enemies |
| +0xD32 | 1 | `DisguiseWhenStill=` | bool | Disguise when stationary (Mirage Tank) |

**Correction:** earlier drafts of this doc claimed `DetectDisguise` and
`DisguiseWhenStill` share byte +0xD31. That was wrong.
`DISGUISE_SYSTEM_GHIDRA_REPORT.md` §1 verified via hex-read of
`TechnoTypeClass::ReadINI` at `0x00714400-0x00714470` that the four bytes
`CanDisguise` / `PermaDisguise` / `DetectDisguise` / `DisguiseWhenStill` are
separate, consecutive bool fields at +0xD2F / +0xD30 / +0xD31 / +0xD32.

### TechnoClass Disguise Instance Fields

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x518 | 4 | Disguise | TechnoTypeClass* -- what type this unit appears as |
| +0x51C | 4 | DisguisedAsHouse | HouseClass* -- whose unit it appears to be |

Both initialized to 0 (NULL) in TechnoClass constructor. Set at runtime when a Spy enters an enemy building or Mirage Tank stops moving.

### Spy Disguise Defaults (RulesClass)

| RulesClass Offset | INI Key | Default | Description |
|-------------------|---------|---------|-------------|
| +0xD58 | `AlliedDisguise=` | E1 | Default Allied infantry disguise |
| +0xD5C | `SovietDisguise=` | E2 | Default Soviet infantry disguise |
| +0xD60 | `ThirdDisguise=` | INIT | Default Yuri infantry disguise |
| +0xD6C | `AttackCursorOnDisguise=` | yes (YR) | Show attack cursor on disguised units |

### Mirage Tank Disguise (DefaultMirageDisguises)

| RulesClass Offset | INI Key | Default |
|-------------------|---------|---------|
| +0xFF8 | `DefaultMirageDisguises=` | TREE01,TREE02,TREE03,TREE04 |
| +0x1014 | `InfantryBlinkDisguiseTime=` | (int) frames for blink effect |

- Mirage Tank (`DisguiseWhenStill=yes`) picks a random tree from DefaultMirageDisguises when it stops moving
- The disguise is stored in TechnoClass+0x518 (Disguise pointer)
- Must be TerrainType objects (trees/boxes, NOT rocks)

### Disguise Detection

Detected via the cell-level `DisguiseDetectCount` system (see section 4.4). When a cell has DisguiseDetectCount > 0 for a house, that house can see the true identity of any disguised enemy on that cell.

**DisabledDisguiseDetectionPercent:** RulesClass+0xE10 (vector) -- detection probability when building is low power.

### Weapon Disguise Fields

| WeaponTypeClass Offset | INI Key | Type | Description |
|------------------------|---------|------|-------------|
| +0x13B | `DisguiseFireOnly=` | bool | Weapon only fires while disguised |
| +0x13C | `DisguiseFakeBlinkTime=` | int | Frames for fake blink animation |

### Warhead Disguise Field

| WarheadTypeClass Offset | INI Key | Type | Description |
|-------------------------|---------|------|-------------|
| +0x175 | `MakesDisguise=` | bool | Warhead applies disguise effect on hit |

### 6.x Dog & Detector Piercing — cross-reference

For the complete disguise state machine, INI schema, and — relevant to
cloak/stealth because dogs are the classic anti-Spy and anti-Mirage
unit — the Attack Dog disguise-piercing mechanism, see
`DISGUISE_SYSTEM_GHIDRA_REPORT.md` §7.

Key cross-cutting facts (verified in that doc's §7 follow-up pass,
2026-04-21):

- Dog disguise piercing is NOT a hardcoded Dog-vs-Spy branch. All four dog
  InfantryTypes (`[ADOG]`, `[DOG]`, `[YADOG]`, `[YDOG]`) set
  `DetectDisguise=yes` (TechnoTypeClass +0xD31), which is consumed in
  `TechnoClass::Evaluate_Candidate` at `0x006F84D4` as a targeting-legality
  gate.
- The gate reads: `target.IsDisguised (vtable+0xC8 → +0x1D8) && !attacker.
  TypeClass.DetectDisguise`. If true, non-piercer attackers fail to see the
  disguise (subject to a blink-timer + `DisabledDisguiseDetectionPercent`
  chance gate for AI attackers). Piercers skip the block entirely.
- **Unit-level `DetectDisguise=yes` does NOT stamp the cell `+0xAC`
  counter.** Only buildings do (via `BuildingClass::OnConstructionComplete`
  → vtable+0x4FC). A dog therefore pierces disguise *for itself only* — it
  does not reveal a Spy to the rest of its house's units.
- Dogs *can* target disguised Mirage Tanks (same gate), but their weapon
  `ParasiteDog` has `Verses=...,0%,0%,...` vs vehicle armor — they deal no
  damage. Proximity would auto-break the Mirage's disguise anyway via
  `UnitClass::TurretAI`'s 3x3 enemy scan.
- `vtable+0xC8` on TechnoClass (the stub at `0x0041C020` previously called
  `ReturnFalse_0C8`) is actually `IsDisguised_Getter`: it returns
  `*(byte*)(this + 0x1D8)`. Used by `Evaluate_Candidate` for the disguise
  gate. This should be re-labeled in Ghidra.

Pierce-capable units in retail YR (full list in DISGUISE doc §7.5):

- Infantry (targeting-gate only, no cell-counter contribution): `ADOG`,
  `DOG`, `YADOG`, `YDOG`, `PTROOP` (Psi-Corps Trooper), `YURI` (basic Yuri).
- Buildings (stamp +0xAC counter for owning house): `NAPSIS` (Psychic
  Sensor), `YAPSYT` (Psychic Tower), `NAPSYB` (Psychic Beacon).

The `Doggie=` InfantryType INI flag (+0xEC7) is dead — parsed but never
read and not set on any unit in rulesmd.ini. Do NOT implement.

---

## 7. Visual Rendering

### Visual State -> Draw Flags (SHP units, TechnoClass::DrawSHP at 0x705E00)

| Visual State | Progress Range | SHP Flags | VXL Flags | Effect |
|-------------|----------------|-----------|-----------|--------|
| 0 | N/A | 0x00 | 0x2000 | Fully opaque |
| 1 | 0-24% | 0x02 | 0x2002 | 75/25 alpha shimmer |
| 2 | 25-49% | 0x04 | 0x2004 | 50/50 alpha blend |
| 3 | 50-74% | 0x04 | 0x2004 | 50/50 alpha blend |
| 4 | 75-99% | 0x02 or 0x04 | 0x200A/0x200C | Near-invisible |
| 5 | 100% | skip draw | skip draw | Invisible |

### Flag Bit Meanings

| Bit | Value | Meaning |
|-----|-------|---------|
| 0x02 | Shimmer | 75/25 alpha blend (75% source, 25% dest) |
| 0x04 | Semi-transparent | 50/50 alpha blend |
| 0x06 | Combined | 25/75 blend (chrono warp + cloak combined) |
| 0x800 | Remap | Apply house color remapping |
| 0x2000 | Custom frame | VXL frame index |
| 0x4000 | Mirror/flip | Horizontal flip |

### Blitter Selection (0x00490B90)

The `flags & 6` value selects the blitter family:

| flags & 6 | With 0x800 | Blitter Offset | Visual |
|-----------|-----------|----------------|--------|
| 0x00 | 0x800 | +0x6C | Opaque with remap |
| 0x02 | 0x802 | +0x7C | Shimmer: `(src>>2 & mask)*3 + (dest>>2 & mask)` |
| 0x04 | 0x804 | +0x78 | 50% blend: `(src>>1 & mask) + (dest>>1 & mask)` |
| 0x06 | 0x806 | +0x74 | 25% blend: `(src>>2 & mask) + (dest>>2 & mask)*3` |

Mask = 0xF7DE (R5G6B5) or 0x7BDE (R5G5B5), preserving color channel boundaries.

### Chrono Warp Visual

In `TechnoClass::Draw` (0x706640):
```c
if (IsWarpingIn() || IsWarpingOut()) {
    if (RTTI != UnitClass || !unit->field_0x6D3) {
        drawFlags |= 0x2004;  // chrono shimmer
    }
}
```

### Allied Cloaked Unit Shimmer (ModifyCloakDrawFlags at 0x0070ED80, vtable+0x43C)

For cloaked units owned by the local player, a pulsing shimmer cycle is applied:

256-frame repeating cycle (~17 seconds at 15fps):
- Frames 0x00-0x3F: opaque
- Frames 0x40-0x43: shimmer (|= 0x02)
- Frames 0x44-0x4B: 50% blend (|= 0x04)
- Frames 0x4C-0x4F: shimmer (|= 0x02) (corrected 2026-07-18: was "opaque flash" — binary falls through the same nested-if chain to the shared `return param_2 | 2` at function end, identical to the 0x40-0x43 branch — verified via `decompile_function 0x0070ED80` — OPERATOR_OR_ORDER_DRIFT)
- Frames 0x50-0x6F: opaque
- Frames 0x70-0x73: shimmer (|= 0x02)
- Frames 0x74-0x7B: 50% blend (|= 0x04)
- Frames 0x7C-0x7F: shimmer (|= 0x02) (corrected 2026-07-18: was merged into "0x7C-0xFF: opaque" — binary falls through to the shared `return param_2 | 2`, same fallthrough as 0x4C-0x4F — verified via `decompile_function 0x0070ED80` — OPERATOR_OR_ORDER_DRIFT)
- Frames 0x80-0xFF: opaque

Four shimmer windows per period (0x40-0x43, 0x4C-0x4F, 0x70-0x73, 0x7C-0x7F), not two as previously stated — the two "opaque flash" gaps were actually shimmer. This lets the player track their own cloaked units.

### Gap Generator Visual Update (TechnoClass::UpdateGapVisual at 0x0070E920)

10-state animation sequence for gap generator visual effect on nearby units:

| State | Duration | Next State |
|-------|----------|------------|
| 0 | immediate | 1 (start timer=6) |
| 1 | 6 frames | 2 (timer=4) |
| 2 | 4 frames | 3 (timer=20 +/- random 5) |
| 3 | ~20 frames | 4 (timer=64) |
| 4 | 64 frames | 5 (timer=64) |
| 5 | 64 frames | 4 (loop) OR 6 (if CDTimer < 158) |
| 6 | wait | 7 (when CDTimer < 31, timer=6) |
| 7 | 6 frames | 8 (timer=4) |
| 8 | 4 frames | 9 (timer=20) |
| 9 | 20 frames | 10 (final) |

---

## 8. Key Struct Field Tables

### TechnoClass Instance Fields

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x220 | 4 | CloakState | 0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking |
| +0x224 | 4 | CloakProgress | Animation counter (0..CloakingStages) |
| +0x228 | 1 | CloakDirty | Set to 1 when CloakProgress changes |
| +0x22C | 12 | CloakStepTimer | CDTimerClass controlling tick rate |
| +0x238 | 4 | CloakingSpeed | Frames per step (from TypeClass+0x310) |
| +0x23C | 4 | CloakStepDelta | +1 (cloaking) or -1 (uncloaking) |
| +0x240 | 12 | ReCloakDelayTimer | Delay before re-cloaking after uncloak |
| +0x269 | 1 | CloakShroudActive | 1 when gap shroud cells applied |
| +0x26C | 4 | CloakShroudRadius | Cached from TypeClass+0xCD2 |
| +0x270 | 1 | IsWarpingIn | Set during chrono warp arrival |
| +0x271 | 1 | IsWarpingOut | Set during chrono warp departure |
| +0x2B4 | 4 | MindControllerPtr | TechnoClass* that mind-controls this unit |
| +0x3D2 | 1 | HasStealthAbility | Runtime cloakable flag |
| +0x3D5 | 1 | (unknown) | Override flag; if set, prevents instant recloak on Unlimbo |
| +0x41A | 1 | IsDiscoveredByCurrentPlayer | Per-player discovery flag |
| +0x518 | 4 | Disguise | TechnoTypeClass* -- disguised appearance |
| +0x51C | 4 | DisguisedAsHouse | HouseClass* -- disguise owner |

### TechnoTypeClass Fields

| Offset | Size | INI Key | Description |
|--------|------|---------|-------------|
| +0x29C | 18 | `VeteranAbilities=` | Boolean array (18 abilities) |
| +0x2A2 | 1 | (index 6) | VeteranAbilities[CLOAK] |
| +0x2AE | 18 | `EliteAbilities=` | Boolean array (18 abilities) |
| +0x2B4 | 1 | (index 6) | EliteAbilities[CLOAK] |
| +0x310 | 4 | `CloakingSpeed=` | Frames between cloak steps |
| +0x5F0 | 4 | `SensorsSight=` | Sensor detection range (cells) |
| +0x5F4 | 4 | `DetectDisguiseRange=` | Disguise detection range (cells) |
| +0xC93 | 1 | `CloakStop=` | Must stop moving to cloak |
| +0xC9A | 1 | `Invisible=` | Always hidden unless discovered |
| +0xC9D | 1 | `Sensors=` | Boolean sensor flag |
| +0xCD0 | 1 | `Cloakable=` | Main cloakable flag |
| +0xCD1 | 1 | `GapGenerator=` | Is a gap generator |
| +0xCD2 | 1 | `GapRadiusInCells=` | Gap radius |
| +0xCD3 | 1 | `SuperGapRadiusInCells=` | Super gap radius |
| +0xD2F | 1 | `CanDisguise=` | Can disguise (Spy) |
| +0xD30 | 1 | `PermaDisguise=` | Always disguised |
| +0xD31 | 1 | `DetectDisguise=` | Can detect disguised enemies |
| +0xD32 | 1 | `DisguiseWhenStill=` | Disguise when still (Mirage Tank) |

### BuildingTypeClass Fields

| Offset | Size | INI Key | Description |
|--------|------|---------|-------------|
| +0x16C7 | 1 | `CloakGenerator=` | Building generates cloak field |
| +0x16C8 | 1 | `SensorArray=` | Building is a sensor array |
| +0x1707 | 1 | `CloakRadiusInCells=` | Building-specific cloak/sensor radius |
| +0x170C | 4 | `PsychicDetectionRadius=` | Psychic detection range |

### BuildingClass Instance Fields

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x55C | 84 | AnimSlots[21] | 21 animation pointers for building cloak effects |
| +0x660 | 1 | CloakAnimDirection | 0=cloaking in, 1=cloaking out |
| +0x6ED | 1 | BuildingCloakStage | Byte counter (0-16) for building cloaking |

### WeaponTypeClass Fields

| Offset | Size | INI Key | Description |
|--------|------|---------|-------------|
| +0x133 | 1 | `DecloakToFire=` | Must decloak before firing |
| +0x13B | 1 | `DisguiseFireOnly=` | Weapon only fires while disguised |
| +0x13C | 4 | `DisguiseFakeBlinkTime=` | Frames for fake disguise blink |

### WarheadTypeClass Fields

| Offset | Size | INI Key | Description |
|--------|------|---------|-------------|
| +0x175 | 1 | `MakesDisguise=` | Warhead applies disguise effect |

### RulesClass Fields

| Offset | Size | INI Key | Default | Description |
|--------|------|---------|---------|-------------|
| +0x628 | 4 | `CloakingStages=` | 9 | Number of steps in cloak animation |
| +0x1410 | 8 | `CloakDelay=` | 0.02 (min) | Re-cloak delay in minutes |
| +0x1700 | 8 | `ConditionYellow=` | (float) | Health threshold for yellow damage state |
| +0x1708 | 8 | `ConditionRed=` | (float) | Health threshold for red/critical damage state; read by CloakingTick for damaged-unit cloak chance. Verified via `decompile_function 0x6691E0` (param_1[0x5C0]=+0x1700=ConditionYellow, param_1[0x5C2]=+0x1708=ConditionRed) |
| +0xD58 | 4 | `AlliedDisguise=` | E1 | Default Allied spy disguise |
| +0xD5C | 4 | `SovietDisguise=` | E2 | Default Soviet spy disguise |
| +0xD60 | 4 | `ThirdDisguise=` | INIT | Default Yuri spy disguise |
| +0xD6C | 1 | `AttackCursorOnDisguise=` | yes | Attack cursor on disguised units |
| +0xFF8 | vec | `DefaultMirageDisguises=` | TREE01-04 | Terrain types for Mirage Tank |
| +0x1014 | 4 | `InfantryBlinkDisguiseTime=` | (int) | Blink timer for infantry disguise |
| +0xE10 | vec | `DisabledDisguiseDetectionPercent=` | (vector) | Detection % when low power |

### CellClass Fields

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x78 | 4 | VisibilityBitmask | 32-bit bitmask, 1 bit per house |
| +0x7C | varies | SensorCount[] | short array, per-house sensor counts |
| +0xAC | varies | DisguiseDetectCount[] | short array, per-house disguise detect counts |
| +0x12C | 4 | ShroudFlags | Bit flags (0x08=reveal, 0x10=redraw) |
| +0x130 | 4 | GapShroudLevel | >0 = reshrouded for enemies |
| +0x134 | 4 | GapOverlayCount | Gap generator coverage count |
| +0x13C | 4 | AlliedGapExclusion | Allied gap exclusion count |

---

## 9. INI Keys Reference

### Unit/Type-Level Keys

| INI Key | Type | Location | Default | Description |
|---------|------|----------|---------|-------------|
| `Cloakable=` | bool | TechnoTypeClass | no | Unit can cloak |
| `CloakStop=` | bool | TechnoTypeClass | no | Must stop to cloak |
| `CloakingSpeed=` | int | TechnoTypeClass | (inherited) | Frames per cloak step (lower = faster) |
| `Invisible=` | bool | TechnoTypeClass | no | Always hidden unless sensor-detected |
| `Sensors=` | bool | TechnoTypeClass | no | Has sensor capability |
| `SensorsSight=` | int | TechnoTypeClass | 0 | Sensor detection range in cells |
| `DetectDisguise=` | bool | TechnoTypeClass | no | Can detect disguised enemies |
| `DetectDisguiseRange=` | int | TechnoTypeClass | 0 | Range for disguise detection |
| `CanDisguise=` | bool | TechnoTypeClass | no | Can disguise as enemy (Spy) |
| `PermaDisguise=` | bool | TechnoTypeClass | no | Always disguised |
| `DisguiseWhenStill=` | bool | TechnoTypeClass | no | Disguise when stationary (Mirage) |
| `GapGenerator=` | bool | TechnoTypeClass | no | Is a gap generator |
| `GapRadiusInCells=` | int | TechnoTypeClass | 0 | Gap shroud radius |
| `SuperGapRadiusInCells=` | int | TechnoTypeClass | 0 | Super gap radius |

### Building-Level Keys

| INI Key | Type | Location | Description |
|---------|------|----------|-------------|
| `CloakGenerator=` | bool | BuildingTypeClass | Building generates cloak field |
| `SensorArray=` | bool | BuildingTypeClass | Building is a sensor array |
| `CloakRadiusInCells=` | int | BuildingTypeClass | Building cloak/sensor radius |
| `PsychicDetectionRadius=` | int | BuildingTypeClass | Psychic detection range |

### Weapon-Level Keys

| INI Key | Type | Location | Default | Description |
|---------|------|----------|---------|-------------|
| `DecloakToFire=` | bool | WeaponTypeClass | yes | Must decloak before firing |
| `DisguiseFireOnly=` | bool | WeaponTypeClass | no | Only fires while disguised |
| `DisguiseFakeBlinkTime=` | int | WeaponTypeClass | 0 | Fake disguise blink frames |

### Warhead-Level Keys

| INI Key | Type | Location | Description |
|---------|------|----------|-------------|
| `MakesDisguise=` | bool | WarheadTypeClass | Warhead applies disguise on hit |

### Global [General] Keys

| INI Key | Type | Default | Description |
|---------|------|---------|-------------|
| `CloakingStages=` | int | 9 | Animation step count |
| `CloakDelay=` | double | 0.02 | Re-cloak delay (minutes) |
| `AlliedDisguise=` | string | E1 | Default Allied spy disguise type |
| `SovietDisguise=` | string | E2 | Default Soviet spy disguise type |
| `ThirdDisguise=` | string | INIT | Default Yuri spy disguise type |
| `AttackCursorOnDisguise=` | bool | yes (YR) | Attack cursor on disguised enemies |
| `DefaultMirageDisguises=` | string list | TREE01-04 | Terrain types for Mirage Tank disguise |
| `InfantryBlinkDisguiseTime=` | int | (varies) | Disguise blink timer for infantry |
| `DisabledDisguiseDetectionPercent=` | int list | (varies) | Detection % when building low power |

### YR Retail Examples

```ini
; Mirage Tank [MGTK]
Cloakable=yes
CloakingSpeed=5        ; Slowish, low is faster
DisguiseWhenStill=yes

; Spy [SPY]
CanDisguise=yes
PermaDisguise=yes

; Gap Generator [GAWALL]
GapGenerator=yes
GapRadiusInCells=10
SuperGapRadiusInCells=10

; Psychic Sensor [PSYCHIC] / Spy Satellite [NACLON]
SensorArray=yes
SensorsSight=15
DetectDisguise=yes
DetectDisguiseRange=15
PsychicDetectionRadius=15

; [General]
CloakingStages=9
CloakDelay=.02
AlliedDisguise=E1
SovietDisguise=E2
ThirdDisguise=INIT
DefaultMirageDisguises=TREE01,TREE02,TREE03,TREE04
```

---

## 10. Implementation Notes

### For the Rust Engine

1. **Cloak state as an enum:** Map CloakState to a 4-variant Rust enum (Uncloaked, Cloaking, Cloaked, Uncloaking). Store alongside CloakProgress (u32), CloakingSpeed (u32), CloakStepDelta (i32), and timer fields.

2. **CloakingTick in sim tick order:** Should run during the "turrets + combat" or "building anims + cleanup" phase. Must run after movement (locomotion may clear CloakStop gate) but before rendering.

3. **Gap Generator is local-player-relative:** UpdateCloakShroud and RemoveCloakShroud only affect the local player's view. For multiplayer, each client computes gap effects independently based on enemy gap generators visible to them. This is shroud/rendering layer, NOT sim state.

4. **Sensor counters are per-house per-cell:** Use `Vec<i16>` arrays on CellData indexed by house ID. Increment/decrement on unit placement/movement/removal. This IS sim state (affects targeting and detection).

5. **Disguise is a separate system:** Store Disguise and DisguisedAsHouse as separate fields on the entity, independent of CloakState. The rendering layer checks both to determine what to draw and whether to draw it.

6. **DecloakToFire defaults to YES:** When not explicitly set to `no` in INI, weapons require decloaking. The fire error check must return a "must decloak" error and the combat system must handle the retry-after-uncloak flow.

7. **Transport enter/exit:** Do NOT touch CloakState on Limbo. On Unlimbo, if HasStealthAbility AND NOT override flag, set CloakState directly to Cloaked (skip animation).

8. **Veterancy CLOAK ability:** Veteran/elite units with CLOAK can cloak even while firing, moving, or warping. This is a stronger form of cloaking than the base Cloakable=yes.

9. **Visual state for rendering:** Compute from CloakState + CloakProgress + CloakingStages. The 0-5 value selects the alpha blending mode. Allied cloaked units get the pulsing shimmer cycle. Enemies at visual state 5 are simply not drawn.

10. **TS legacy warning:** Fog of war (`SpecialFlags & 0x1000`) is NOT active in standard YR. Gap Generators re-shroud cells; they do NOT create fog. Only implement shroud (black for unexplored) and gap (re-shrouded for enemies).

---

## Vtable Dispatch Reference

All offsets from TechnoClass primary vtable:

| Offset | Function | Address | Description |
|--------|----------|---------|-------------|
| +0x2C | WhatAmI/GetRTTI | varies | Type ID (1=Unit, 2=Aircraft, 15=Infantry, 6=Building — 1/2 verified 2026-07-12 via `read_memory` on `vtable__UnitClass`+0x2c and `vtable__AircraftClass`+0x2c; 6 verified 2026-07-18 via `decompile_function 0x00459ec0` (`BuildingClass__WhatAmI` returns 6); Infantry corrected 2026-07-18: was "5" — `InfantryClass__What_Am_I` at 0x00523340 (= `vtable__InfantryClass`+0x2c, base read via `read_memory 0x007eb058`) returns 0xF=15, not 5 — cross-confirmed by `BuildingClass__UpdateGapGenerator_Tick` (0x00454DB0) grouping WhatAmI values 1/0xf/2 (Unit/Infantry/Aircraft) as the cloak-capable technos for its sensor-uncloak check — INFERENCE_HARDENED) |
| +0x48 | GetCoords | varies | Current position |
| +0x68 | GetVisualState | 0x703860 | Returns 0-5 visual state |
| +0x84 | GetTypeClass | varies | TechnoTypeClass pointer |
| +0xDC | Limbo | varies | Remove from map |
| +0xFC | StartUncloaking_Wrapper | varies | Calls vtable+0x45C(0) |
| +0x124 | ObjectClass::Mark | 0x5F5850 | Hide/reveal in overlap list; called with arg 2 during cloak transitions. Label `ProcessCloakMode` was wrong — verified via `get_function_by_address 0x5F5850` |
| +0x134 | MarkForRedraw | varies | Flag for visual update |
| +0x150 | UpdateVisibility | varies | Shroud/radar update |
| +0x174 | Scatter | varies | Move to random position |
| +0x1D4 | IsWarpingIn | varies | Returns +0x270 |
| +0x1D8 | IsWarpingOut | varies | Returns +0x271 |
| +0x274 | UpdateRadarTracking | varies | Radar update with param |
| +0x288 | IsCloakable | 0x4DBDA0 | Check HasStealthAbility + CloakStop |
| +0x2A0 | CanAutoCloak | 0x6FBDC0 | Full auto-cloak eligibility |
| +0x2A4 | ShouldUncloak | 0x6FBC90 | Should cloaked unit uncloak? |
| +0x350 | IsPowered | varies | Building power check |
| +0x37C | IsCrashing/IsFiring | varies | Activity check |
| +0x380 | IsTeleporting | varies | Chrono teleport check |
| +0x3C8 | ScatterFromMindControl | varies | Scatter MC subjects |
| +0x420 | DoUncloak | 0x6F4EB0 | Force uncloak with discovery |
| +0x43C | ModifyCloakDrawFlags | 0x70ED80 | Allied shimmer cycle |
| +0x45C | StartUncloaking | 0x7036C0 | CloakState -> 3 |
| +0x460 | StartCloaking | 0x703770 | CloakState -> 1 |
| +0x4E8 | AddSensorsAt | 0x4DE7B0 | Add sensor coverage |
| +0x4EC | RemoveSensorsAt | 0x4DE940 | Remove sensor coverage |
| +0x4F4 | AddSensorArrayAt | 0x455820 | Building sensor add |
| +0x4F8 | RemoveSensorArrayAt | 0x4556D0 | Building sensor remove |
| +0x4FC | AddDetectDisguiseAt | 0x455A80 | Building disguise detect add |
| +0x500 | RemoveDetectDisguiseAt | 0x455980 | Building disguise detect remove |
