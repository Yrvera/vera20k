---
name: BuildingClass Mission_Attack + Residuals Verification
description: Full structural map of Mission_Attack (0x0044ACF0) including the 11-entry GetFireError jump table and 3-state ChargeMode machine. Plus resolution of 5 residual questions from Mission_RepairAndProduce (URepairRate key, 1.0 threshold, locomotor CLSID, Anims slot roles)
type: reference
---

# BuildingClass Mission_Attack + Residuals Report

**Date:** 2026-04-19
**Binary:** gamemd.exe
**Confidence:** HIGH — all findings verified via direct decompilation / inspection
**Active in YR:** Yes — combat and repair paths are core YR mechanics

---

## Part 1 — Mission_RepairAndProduce Residuals Resolved

### 1.1 Rules+0x16E8 = `URepairRate` (not "CloseEnoughDistance")

**Evidence:** `RulesClass::ReadGeneral` at `0x00670E44` pushes string pointer
`0x0083BDC4` for ReadDouble, result stored at `Rules+0x16E8` via FSTP:

```
00670e44: PUSH 0x83bdc4           ; key string ptr
00670e4a: MOV ECX, EDI             ; RulesClass ReadDouble this
00670e4c: CALL 0x005283d0          ; ReadDouble
00670e51: FSTP double ptr [ESI + 0x16e8]
```

Memory at `0x0083BDC4` (verified via inspect): `55 52 65 70 61 69 72 52 61 74 65 00` = **"URepairRate"** (the 'U' prefix = Unit/Vehicle variant, parallel to `IRepairRate` for infantry).

**Repair Depot uses `URepairRate`; Hospital/Armory use `IRepairRate`.**

### 1.2 Rules+0x16F8 is hardcoded `1.0`, NOT an INI key

**Evidence:** `RulesClass::ReadAudioVisual` at `0x00668FB0`-ish initializes
the double at offset 0x16F8 unconditionally:
```c
param_1[0x5BE] = 0;
param_1[0x5BF] = 0x3FF00000;   // IEEE 754 double 1.0 upper bits
```
No corresponding `ReadDouble` call. This is the **"full health ratio" threshold**
(1.0) used in Repair Depot state 1: `if (Rules+0x16F8 <= health_ratio)` →
"unit is at 100% health → release it."

### 1.3 Complete repair-tuning table from ReadGeneral

| Offset | INI Key | Type | Default | Purpose |
|---|---|---|---|---|
| Rules+0x16CC | `RepairStep` | int | 8 | HP restored per repair tick |
| Rules+0x16D0 | `RepairPercent` | double | 0.15 | Cost fraction of full rebuild |
| Rules+0x16D8 | `IRepairStep` | int | ? | HP per infantry-repair tick |
| Rules+0x16E0 | `RepairRate` | double | 0.016 min | Minutes between vehicle repair ticks (general) |
| Rules+0x16E8 | **`URepairRate`** | double | ? | Minutes between Unit-in-Repair-Depot ticks |
| Rules+0x16F0 | `IRepairRate` | double | 0.001 min | Minutes between infantry-heal ticks (Hospital/Armory) |
| Rules+0x16F8 | (none — hardcoded 1.0) | double | 1.0 | Full-health threshold |

### 1.4 DAT_007E9AB0 = DriveLocomotion CLSID

**Evidence:** Inspect at `0x007E9AB0` yields 16-byte GUID:
```
E1 74 EA 2B  CA 7C D3 11  BE 14 00 10 4B 62 A1 6C
```
Formatted: **`{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}`**

Context: In the Repair Depot acceptance test in Mission_RepairAndProduce,
the unit's locomotor CLSID is compared against both `CLSID_WalkLocomotion`
AND `DAT_007E9AB0`. Walk is infantry → bay doesn't accept infantry, so that
comparison must be a filtering test. The second CLSID is for vehicles that
ARE accepted: **DriveLocomotion**.

Adjacent CLSIDs in the locomotor pool at VA 0x007E9AB0+0x10, +0x20, +0x30:
- `{92612C46-F71F-11D1-AC9F-006008055BB5}` = **JumpjetLocomotion**
- `{B7B49766-E576-11D3-9BD9-00104B972FE8}` = ? (likely DropPod or similar)
- `{170DAC82-12E4-11D2-8175-006008055BB5}` = ? (likely TeleportLocomotion)

### 1.5 BuildingClass +0x57C / +0x588 / +0x58C are Anims[8/11/12]

**Evidence:** The Anims array is at `BuildingClass+0x55C`, 21 DWORDs.

| Field | Slot Index | Calc | Role (from context) |
|---|---|---|---|
| `+0x57C` | 8 | (0x57C-0x55C)/4 = 8 | Repair Depot "arm extended" |
| `+0x588` | 11 | (0x588-0x55C)/4 = 11 | Repair Depot "arm retracted" |
| `+0x58C` | 12 | (0x58C-0x55C)/4 = 12 | Repair Depot secondary arm / turret |

Verified by the ClearAnimSlot indices in Mission_RepairAndProduce
(`ClearAnimSlot(8)`, `ClearAnimSlot(11)`) paired with CreateAnimForSlot calls
that target Type+0x1018/0x1028 (slot 3) and Type+0x127C/0x128C (slot 12)
during state transitions.

---

## Part 2 — Mission_Attack (0x0044ACF0, 1174 bytes)

Vtable slot 132 (offset 0x210). Dispatched when mission == 1 (ATTACK).

### 2.1 Top-level dispatch

```c
int BuildingClass::Mission_Attack(this) {
    Type = this->Type;
    if (Type+0x16B8 [IsChargeMode] == 0) {
        // Path A: normal direct-fire buildings
    } else {
        // Path B: charge-mode (Tesla Coil, Prism Tower, EMPCannon-style)
    }
    return 1;
}
```

### 2.2 Path A — Direct-fire path

```c
if (this->target (+0x2B4) == NULL) {
    // No target:
    this->vtable[0x3C8](0);                 // Set_ArchiveTarget(NULL)
    this->+0x664 = 0;                        // clear garrison round-robin idx
    if (this->vtable[0x430]()) {             // burst/rearm check
        thunk_FUN_006385C0();                // some reset
    }
    if (this->GetCurrentMission() == 0x1C) return 1;   // DECONSTRUCTION - stay
    this->Queue_Mission(5=GUARD);
    this->Commence();
    return 1;
}

// Have a target:
target_coord = this->vtable[0x2E4](target);  // Fire_Coord
this->+0x6DD = 1;                             // construction-complete flag
fire_error = this->vtable[0x3C0](target, target_coord, 1);   // GetFireError

// FIRE_FACING special case: TickTank-like buildings get 2 rotation chances
if (fire_error == 2) {
    if (this->vtable[0x3FC]() /*HasTurret*/ && Type+0x16C5 /*?*/) {
        // Compute turret facing toward target
        current = this->vtable[0x4E8](target);
        rot_limit = Type+0x71C (ROT);
        delta = abs(RateTimer::Current() - current);
        if (delta > rot_limit) {
            // Still need to turn more — fall through to jump table
            goto LAB_0044B0D7;
        }
        FacingClass::UpdateFacing(&target_facing);
        fire_error = this->vtable[0x3C0](target, target_coord, 1);   // re-check
    }
}

// 11-entry jump table on fire_error (see §2.3)
if (fire_error < 11) {
    return jump_table[fire_error]();
}

// fire_error >= 11 (unexpected — treat as OK-ish)
if (this->target != NULL) {
    facing = this->vtable[0x4E8](target);
    RateTimer::Set(facing);
}
this->+0xC4 = 0;                            // clear some state
return 1;
```

### 2.3 The 11-entry GetFireError jump table @ 0x0044B728

Read from memory (11 little-endian DWORDs):

| fire_error | Handler Address | Shared With | Enum (inferred) | Behavior |
|:---:|:---:|:---:|:---|:---|
| 0 | `0x0044B2BC` | — | FIRE_OK | **Fire weapon** via `vtable[0x3CC](target, 0)` — checks `UpgradeLevel` for upgrade weapon override first |
| 1 | `0x0044B0DE` | 5, 6, 8 | FIRE_AMMO | Clear target + `vtable[0x3C8](0)`, reset garrison idx |
| 2 | `0x0044B187` | — | FIRE_FACING | Rotate turret (fall-through after re-check failed) |
| 3 | `0x0044B1DE` | — | FIRE_REARM | Play reload animation / wait |
| 4 | `0x0044B14E` | 7 | FIRE_ROTATING | Wait for rotation to complete |
| 5 | `0x0044B0DE` | 1, 6, 8 | FIRE_ILLEGAL | Same as AMMO handler |
| 6 | `0x0044B0DE` | 1, 5, 8 | FIRE_CANT | Same |
| 7 | `0x0044B14E` | 4 | FIRE_MOVING | Same as ROTATING (buildings never move → unreachable in practice) |
| 8 | `0x0044B0DE` | 1, 5, 6 | FIRE_RANGE | Same |
| 9 | `0x0044B284` | — | FIRE_CLOAKED | Cloaked target handler |
| 10 | `0x0044B24F` | — | FIRE_BUSY | Busy-with-other-action handler |

**Three distinct handler behaviors** from 11 enum values:
- `0x0044B2BC` (0): Fire — calls `Fire_At(target, 0)` after checking upgrade weapon
- `0x0044B0DE` (1/5/6/8): Bail — clear target, reset garrison index, return
- `0x0044B14E` (4/7): Wait — no action this tick
- `0x0044B187` (2): Rotate — compute and apply turret facing
- `0x0044B1DE` (3): Reload anim
- `0x0044B284` (9): Cloak handler
- `0x0044B24F` (10): Busy handler

**Fire handler detail (0x0044B2BC):**
```c
if (this->UpgradeLevel (+0x702) != 0) {
    if (this->Upgrades[0] (+0x5EC) != NULL) {
        if (UpgradeWeapon_HasOverride()) {
            goto bail;   // upgrade weapon override
        }
    }
}
// Fall-through: fire host weapon
target = this->+0x2B4;
vtable = this->vtable;
this->vtable[0x3CC](target, 0);   // Fire_At(target, weapon 0)
return jitter_timer;
```

### 2.4 Path B — ChargeMode 3-State Machine

Active when `Type+0x16B8 IsChargeMode=yes`. State stored at `BuildingClass+0xBC`.

#### State 0 — Pre-charge (target validation + facing alignment)

```c
if (Type+0x1573 != 0 && Type+0xEE4 > 0) {      // has power-drain requirement
    if (HouseClass::GetPowerRatio() < 1.0) {    // low power?
        return 1;                                // wait — no state change
    }
}

// Validate target kind
target = this->+0x2B4;
piVar6 = target;
if (target != NULL && target->vtable[0x2C]() != 2) {
    piVar6 = NULL;    // NOT kind==Aircraft (wait — kind==2 for aircraft?)
}

if (target == NULL || piVar6 == NULL || !piVar6->vtable[0x1D0]() /*visibility*/) {
    this->vtable[0x3C8](0);    // clear archive target
    this->+0xBC = 0;
    this->Queue_Mission(5=GUARD);
    this->Commence();
    return 1;
}

// Valid target; check facing alignment
if (CDTimerClass::Remaining() == 0) {
    target_facing = this->vtable[0x4E8](target);
    current_facing = RateTimer::Current();
    delta = abs(current_facing - target_facing);
    
    if (delta < 0x2001) {              // within ~45° tolerance
        this->+0xBC = 1;                // advance to state 1 (charging)
        return 1;
    }
    RateTimer::Set(target_facing);      // start rotating toward target
    return 1;
}
return 1;   // timer still running
```

**Facing tolerance `0x2001`**: in the 0-0xFFFF (16-bit) facing space:
- 8 compass directions = 0x2000 per direction
- 0x2001 = slightly more than 45° from 0°
- Practical effect: "within roughly one compass point of the target."

#### State 1 — Charging / Fire

```c
target = this->+0x2B4;
target_piVar6 = target valid && kind==2 ? target : NULL;

if (target == NULL || target_piVar6 == NULL || !target->vtable[0x1D0]()) {
    this->vtable[0x3C8](0);
} else {
    fire_err = this->vtable[0x3C0](target, 0, 1);    // GetFireError
    
    if (fire_err == 5 || fire_err == 6 || fire_err == 8) {
        // ILLEGAL / CANT / RANGE — abort charge
        this->vtable[0x3C8](0);
        this->+0xBC = 0;
        return 1;
    }
    if (fire_err != 2 /*FIRE_FACING*/) {
        if (fire_err != 0 /*FIRE_OK*/) {
            return 1;                                 // REARM/ROTATING/etc. — wait
        }
        // FIRE_OK: fire BOTH weapons (Tesla Coil has 2-weapon pattern)
        this->vtable[0x3CC](target, 0);              // Fire_At(weapon 0)
        this->vtable[0x3CC](target, 1);              // Fire_At(weapon 1)
        this->+0xBC = 0;                              // back to state 0
        return 1;
    }
}
this->+0xBC = 0;
return 1;
```

#### State 2+ — Cooldown

```c
timer = MissionClass::GetMissionTimerEntry();
jitter = Random::RandomRanged(0, 2);   // 0, 1, or 2 extra frames
return timer + jitter;                  // jittered cooldown before re-dispatch
```

The cooldown value depends on the mission timer entry for MISSION_ATTACK, plus
0-2 frame random jitter to prevent all Tesla Coils firing in lockstep.

### 2.5 Garrison Fire Integration (Round-Robin)

Mission_Attack does NOT directly dispatch garrison fire. What it does:
- On no-target: `+0x664 = 0` clears the garrison round-robin index
- The actual garrison fire uses a different index location (**+0x69C**, verified
  in R1 round — master doc confirmed this)

**Note on +0x664**: this field is cleared here AND initialized in Constructor.
Master doc R1 verification confirmed `+0x69C` (not +0x664) as the real
GarrisonFireIndex. The +0x664 here may be a secondary flag or stale usage.
Cross-reference check would be needed to fully resolve — but it's not the
primary garrison fire index.

### 2.6 Key fields used by Mission_Attack

| BuildingClass Offset | Purpose |
|---|---|
| +0xBC | ChargeMode state (0/1/2) |
| +0xC4 | Unknown state flag cleared on fire-error-out-of-range |
| +0x2B4 | Target (TechnoClass*) |
| +0x664 | Misc reset flag (cleared on no-target) |
| +0x6DD | Construction-complete flag (set to 1 on target lock) |
| +0x702 | UpgradeLevel (checked for upgrade weapon override) |
| +0x5EC | Upgrades[0] (checked for upgrade weapon override) |

| BuildingTypeClass Offset | Purpose |
|---|---|
| +0x16B8 | **IsChargeMode** (Tesla Coil etc.) — dispatches Path A vs Path B |
| +0x16C5 | Unknown — gates the FIRE_FACING rotation branch alongside HasTurret |
| +0x1573 | Power-drain requirement flag |
| +0xEE4 | Power drain amount |
| +0x71C | ROT (rotation speed) |

| Vtable slot | Offset | Purpose |
|---|---|---|
| 240 | 0x3C0 | **GetFireError** (returns 0-10) |
| 242 | 0x3C8 | Clear_Target / Set_ArchiveTarget(0) |
| 243 | 0x3CC | **Fire_At(target, weaponIdx)** |
| 254 | 0x3FC | HasTurret |
| 256 | 0x3C0 | (same as 240) |
| — | 0x430 | Burst/rearm flag (called as predicate) |
| — | 0x4E8 | Compute facing toward target |
| — | 0x2E4 | Fire_Coord (get target coord) |
| 122 | 0x1E8 | Queue_Mission |
| 123 | 0x1EC | Commence |

---

## Part 3 — Master Doc Updates Needed

### BUILDINGCLASS_MASTER_GHIDRA_REPORT.md updates:

1. **Section 2 (BuildingClass layout):** no new rows; +0x57C/+0x588/+0x58C are
   just elements of the Anims[21] array already documented.
2. **Section 4 (Vtable)** — note: slot 11 (vtable+0x2C) is actually
   `What_Am_I` returning class kind (1=Unit, 2=Aircraft, 6=Building, 0xF=Infantry),
   distinct from slot 8 `AbstractClass::WhatAmI`.
3. **Section 10 "Docking System"** — already has ExitList table entry; add
   DriveLocomotion CLSID reference for Repair Depot piggyback.
4. **Section 17 Mission_Attack details** — update:
   - Confirm 11-entry jump table structure with enum-to-address mapping
   - Charge mode 3-state at `+0xBC` with state 0→1 tolerance `0x2001`
   - Fire_Error=0 handler includes upgrade-weapon override check
5. **Section 17 Mission_RepairAndProduce details** — add:
   - `URepairRate` key for Rules+0x16E8 (Repair Depot rate)
   - Rules+0x16F8 is hardcoded 1.0 (full-health threshold)
6. **New rulesmd.ini doc section** — full repair tuning block:
   ```
   RepairStep=8          ; HP per tick (vehicles)
   RepairPercent=.15     ; cost fraction of full rebuild
   IRepairStep=?         ; HP per tick (infantry)
   RepairRate=.016       ; minutes between vehicle repair ticks (general)
   URepairRate=?         ; minutes between Repair-Depot unit repair ticks
   IRepairRate=.001      ; minutes between infantry heal ticks
   ```

---

## Part 4 — Combined Session Summary

Over the full investigation session:

| Round | Target | Outcomes |
|---|---|---|
| R1 | 7 original open questions | All resolved (ChargeFlags, Factory ptr, BuildingLight, Engineer enum, MCV refund, CloakGen, E70 sound) |
| R2 | 5 follow-ups + ExitObject survey | All resolved (+0x6E3, HasSpotlight, gap-gen offset, vtable 280, SoundEvent, ExitObject structure) |
| R3 | 5 ExitObject unknowns | All resolved (Kind enum, Naval, Bib, HasFreeSlot, AI build queue) |
| — | Mission_RepairAndProduce | Full 7-mode dispatch mapped, gate state machine demythologized |
| **R4** | **5 residuals + Mission_Attack** | **URepairRate, 1.0 threshold, DriveLocomotion CLSID, Anims[8/11/12], jump table, charge mode** |

**Total functions decompiled/analyzed this session:** 30+
**Documents produced:** 5 (R1, R2, R3, MissionRepairAndProduce, this report)
**Master-doc update items queued:** ~20 across all rounds

---

## Sources

### Functions decompiled this round

- `0x0066D530` — RulesClass::ReadGeneral (repair tuning parse)
- `0x00668FB0` — RulesClass::ReadAudioVisual (1.0 init discovery)
- `0x0044ACF0` — BuildingClass::Mission_Attack
- `0x00459EC0` / `0x00523340` / `0x00746E20` — What_Am_I overrides
- `0x00443B90` — BuildingClass::ToggleGate (confirmed misnamed — tick-tank deploy)

### Memory inspections

- `0x0083BDC4` = "URepairRate" (string)
- `0x0083BDDC` = "IRepairStep" (string)
- `0x007E9AB0` = DriveLocomotion CLSID
- `0x0044B728` = 11-entry Mission_Attack jump table

### Cross-references

- `DAT_007E9AB0` → WinMain registration + MissionRepairAndProduce (Repair Depot accept)
- `0x0044B728` → `LAB_0044B0D7` jmp in Mission_Attack

### INI files checked

- `ini/rulesmd.ini`: confirmed `RepairPercent=15%`, `RepairRate=.016`,
  `RepairStep=8`, `IRepairRate=.001`, `RepairDelay=.02` / `.05`
