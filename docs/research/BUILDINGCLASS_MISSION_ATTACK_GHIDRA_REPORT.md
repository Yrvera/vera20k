# BuildingClass::Mission_Attack — Ghidra Analysis Report

**Address:** `0x0044ACF0` (1174 bytes, ends at 0x0044B186)
**Confidence:** HIGH — decompiled and verified against disassembly, vtable entries confirmed.

---

## Overview

`BuildingClass::Mission_Attack` is the combat mission handler for buildings. It implements
two completely different attack pipelines based on whether `IsChargeMode=yes` (Tesla Coil
style) or not (normal turret/garrison attack). The function always returns 1 (re-evaluate
next tick).

**Key structural split:**
```
if (BuildingTypeClass+0x16B8 == 0)  // IsChargeMode=no → NORMAL PATH
    goto normal_attack;
else                                 // IsChargeMode=yes → CHARGE PATH
    goto charge_attack;
```

---

## Critical Field Map

### BuildingTypeClass Fields (accessed via `this+0x520 → Type`)

| Offset | Size | Field | INI Key | Notes |
|--------|------|-------|---------|-------|
| +0x71C | 4 | ROT | `ROT=` | Turret rotation rate (leptons per tick) |
| +0xCD5 | 1 | IsGattling | `IsGattling=` | Has gattling weapon ramp |
| +0xEE4 | 4 | PowerDrain | `Power=` (negative) | Power consumption |
| +0x1573 | 1 | Powered | `Powered=` | Requires power to operate |
| +0x157B | 1 | CanBeOccupied | `CanBeOccupied=` | Can be garrisoned |
| +0x157C | 1 | CanOccupyFire | `CanOccupyFire=` | Garrison occupants can shoot |
| +0x16B8 | 1 | IsChargeMode | `IsChargeMode=` | Tesla Coil charge attack |
| +0x16C3 | 1 | EMPulseCannon | — | TS legacy, blocks firing |
| +0x16C5 | 1 | HasTurret | `HasTurret=` | Building has a turret |
| +0x16EC | 4 | DelayedFireDelay | `DelayedFireDelay=` | Ticks to delay after animation |
| +0x1710 | 1 | (unknown) | — | Used in constructor for RateTimer init |

### BuildingClass Instance Fields

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0xBC | 4 | MissionState | State machine index for current mission |
| +0xC4 | 4 | (cleared) | Cleared to 0 at various exit paths |
| +0x21C | 4 | Owner (HouseClass*) | Owner house pointer |
| +0x2B4 | 4 | TarCom | Current target pointer |
| +0x388 | 24 | TurretFacing | FacingClass for turret direction |
| +0x520 | 4 | Type | BuildingTypeClass pointer |
| +0x664 | 4 | GarrisonFireIndex (reset field) | Zeroed on no-target path in Mission_Attack; actual round-robin index used in GetWeapon is at +0x69C (param_1[0x1a7], corrected 2026-05-29 — ROOT_CAUSE: OFFSET_RETYPED_WRONG; via `decompile_function 0x004526F0`) |
| +0x688 | 4 | GarrisonOccupantArrayPtr | Base pointer of garrison occupant array used by GetWeapon (param_1[0x1a2]) |
| +0x69C | 4 | GarrisonFireRoundRobinIdx | Round-robin fire index used in GetWeapon (param_1[0x1a7]); compared against OccupantCount at +0x694 (corrected 2026-05-29: doc originally showed only +0x664 — ROOT_CAUSE: OFFSET_RETYPED_WRONG; via `decompile_function 0x004526F0`) |
| +0x694 | 4 | OccupantCount | Number of infantry garrisoned |
| +0x6DD | 1 | AnimComplete | Set to 1 when ready to fire |
| +0x702 | 1 | OccupantSlotCount | Number of occupied slots (for GetWeapon) |

### TechnoClass Fields (inherited)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x1D0 | — | (vtable slot, not field) | GetFireError deploying gate uses `TechnoClass__IsDeploying()` virtual call (vtable+0x1D8), not a direct LocomotionTarget field; (corrected 2026-05-29: was "+0x1D0 LocomotionTarget" — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT; via `decompile_function 0x00447F10`) |
| +0x2EC | 4 | ROFTimerStart | Frame when ROF timer started |
| +0x2F0 | 4 | (ROF related) | ROF tracking |
| +0x2F4 | 4 | ROFTimerDuration | Duration of ROF timer |

---

## Normal Attack Path (IsChargeMode=no)

### Step 1: Validate Target

```
if (TarCom == NULL) {
    SetTarget(NULL);             // vtable+0x3C8
    GarrisonFireIndex = 0;       // reset round-robin
    if (IsFiring()) {            // vtable+0x430
        thunk_FUN_006385c0();    // stop firing animation
    }
    if (GetMission() == 0x1C) {  // mission 0x1C = Unload
        return 1;
    }
    QueueMission(Guard, 0);      // vtable+0x1E8, mission 5 = Guard
    NextMission();               // vtable+0x1EC
    return 1;
}
```

### Step 2: Determine Weapon Index

```
weaponIdx = SelectWeaponAgainst(TarCom);   // vtable+0x2E4 (corrected 2026-05-29: was GetWeaponIndex — ROOT_CAUSE: RTTI_LABEL_DRIFT)
this->AnimComplete = 1;                // signal ready to fire
```

### Step 3: Call GetFireError

```
fireError = GetFireError(TarCom, weaponIdx, 1);  // vtable+0x3C0
```

### Step 4: Handle FIRE_FACING (error=2) — Turret Rotation

If `GetFireError` returns FACING (2), the building attempts turret rotation:

```
if (fireError == FIRE_FACING) {
    if (!HasTurret() || !Type->HasTurret)
        goto jump_table;  // can't rotate, use default handler

    // Get direction to target
    targetDir = GetTargetCoords(TarCom);  // vtable+0x4E8, returns facing value

    // Check if ROT (rotation rate) allows turning fast enough
    if (Type->ROT != 0) {
        currentFacing = TurretFacing.Current();
        rotTolerance = abs((short)(Type->ROT << 8));
        facingDelta = abs((short)(currentFacing - targetDir));
        if (rotTolerance < facingDelta)
            goto jump_table;  // too far to turn, give up for this tick
    }

    // Update turret facing toward target
    FacingClass::UpdateFacing(&this->TurretFacing, targetDir);

    // Re-check fire error after rotation
    fireError = GetFireError(TarCom, weaponIdx, 1);
}
```

**Key insight:** The building gets TWO chances to fire per tick — once before rotation, once after.
The ROT value determines the maximum turn rate. If the target is beyond the rotation tolerance,
the building skips to the jump table handler instead (which just updates facing and waits).

### Step 5: Fire Error Switch Table

Jump table at `0x0044B728` indexed by `fireError` (0–10):

| Error | Value | Handler | Behavior |
|-------|-------|---------|----------|
| FIRE_OK | 0 | 0x0044B2BC | **Fire the weapon** (see below) |
| FIRE_AMMO | 1 | 0x0044B0DE | Clear target, go to Guard |
| FIRE_FACING | 2 | 0x0044B187 | Update turret, handle IsGattling decay |
| FIRE_REARM | 3 | 0x0044B1DE | Wait for reload |
| FIRE_ROTATING | 4 | 0x0044B14E | Update facing toward target |
| FIRE_ILLEGAL | 5 | 0x0044B0DE | Clear target, go to Guard |
| FIRE_CANT | 6 | 0x0044B0DE | Clear target, go to Guard |
| FIRE_MOVING | 7 | 0x0044B14E | Update facing toward target |
| FIRE_BUSY | 8 | 0x0044B0DE | Clear target, go to Guard |
| FIRE_RANGE | 9 | 0x0044B284 | See below |
| FIRE_CLOAKED | 10 | 0x0044B24F | See below |
| >10 | — | 0x0044B14E | Default: update facing |

### Handler Details

#### Cases 1/5/6/8 — Target Invalid (0x0044B0DE)

```
SetTarget(NULL);
GarrisonFireIndex = 0;
if (IsFiring()) {
    StopFiringAnimation();
}
if (GetMission() == 0x1C)    // Unload mission
    return 1;

// IsGattling decay: if building has gattling weapon, decrease stage
if (Type->IsGattling) {
    TechnoClass::UpdateGattlingStage(this->field_0xC4);
    this->field_0xC4 = 0;
}

QueueMission(Guard, 0);
NextMission();
// falls through to final cleanup
```

#### Case 2 — FIRE_FACING (0x0044B187)

Turret is not facing the target. Updates turret facing and handles IsGattling decay:

```
if (TarCom != NULL) {
    targetDir = GetTargetCoords(TarCom);  // vtable+0x4E8
    FacingClass::UpdateFacing(&TurretFacing, targetDir);
}

// IsGattling decay when not firing
if (Type->IsGattling) {
    TechnoClass::UpdateGattlingStage(this->field_0xC4);
    this->field_0xC4 = 0;
}

return 2;  // NOTE: returns 2, not 1! (faster re-check)
```

#### Case 3 — FIRE_REARM (0x0044B1DE)

Weapon is reloading. Just keeps turret facing updated:

```
if (TarCom != NULL) {
    targetDir = GetTargetCoords(TarCom);
    FacingClass::UpdateFacing(&TurretFacing, targetDir);
}
return 1;
```

#### Case 4/7 — FIRE_ROTATING/MOVING (0x0044B14E)

Falls through to the "update facing toward target" block + cleanup.

#### Case 9 — FIRE_RANGE (0x0044B284)

Target is out of range. For buildings this effectively means "give up":

```
if (TarCom != NULL) {
    // Still update turret facing
    targetDir = GetTargetCoords(TarCom);
    FacingClass::UpdateFacing(&TurretFacing, targetDir);
}
// Note: does NOT clear target — building keeps tracking
return 1;
```

**Correction:** Looking at the disassembly more carefully, case 9 at 0x0044B284 actually
performs the same as 0x0044B14E (falls through to update facing + cleanup), which includes
clearing field_0xC4. This means out-of-range buildings still track targets but reset their
gattling counters.

#### Case 10 — FIRE_CLOAKED (0x0044B24F)

Target is cloaked. Same behavior as FIRE_RANGE — update facing, keep tracking.

#### Case 0 — FIRE_OK: The Actual Fire Sequence (0x0044B2BC)

This is the garrison fire logic, which is the most complex case:

```
// --- GARRISON FIRE PATH ---
if (this->OccupantSlotCount != 0) {   // building+0x702
    // Validate occupant exists
    if (CurrentOccupant == NULL) {
        // No valid occupant, skip firing
        goto update_turret;
    }
    if (!CanFire(occupant)) {
        goto fire_failed;
    }
    // Fire from the current occupant
    Fire_At(TarCom, primary=0);     // vtable+0x3CC, weapon 0
    Fire_At(TarCom, secondary=1);   // vtable+0x3CC, weapon 1
    // falls through to update_turret
}

// --- NORMAL (NON-GARRISON) FIRE PATH ---
// Check if building type matches superweapon requirement
if (Type != g_RulesInstance->SomeTypePtr+0x498) {
    goto skip_superweapon;
}

// Get coordinates for firing
coords = GetLocation();  // vtable+0xAC
delay = INT_MAX;          // initial delay sentinel

// --- DELAYED FIRE SYSTEM ---
// Check DelayedFireDelay (Type+0x16EC)
delayedFireTicks = Type->DelayedFireDelay;  // BuildingTypeClass+0x16EC
if (delayedFireTicks > 0) {
    // Iterate through occupants to fire from each one with delay
    for (i = 0; i < delayedFireTicks; i++) {
        occupant = OccupantArray[i % OccupantCount];
        if (occupant == NULL) continue;
        if (!occupant->IsAlive()) continue;
        // Check if occupant's type matches expected superweapon type
        if (occupant->Type != ExpectedType) continue;
        // Check ROF timer
        if (occupant->ROFTimer not expired) continue;
        // Actually fire
        occupant->Fire_At(TarCom, weaponIdx);
    }
}
```

**IMPORTANT:** The actual FIRE_OK case (0x0044B2BC) is very long (~600+ bytes). It branches
into garrison fire vs. normal building fire:

1. **Garrison fire** (OccupantSlotCount > 0):
   - Cycles through occupants using `GarrisonFireIndex` (BuildingClass+0x664)
   - Validates each occupant is alive and can fire
   - Fires both primary (0) and secondary (1) weapons via `Fire_At`
   - Increments garrison fire index after firing

2. **Normal building fire** (no garrison):
   - Calls `Fire_At(TarCom, weaponIdx)` via vtable+0x3CC
   - `Fire_At` is `TechnoClass::Fire_At` at 0x006FDD50

### Final Cleanup (all paths converge here at 0x0044B14E+)

```
if (TarCom != NULL) {
    targetDir = GetTargetCoords(TarCom);    // vtable+0x4E8
    RateTimer::Set(&TurretFacing, targetDir);  // update facing target
}
this->field_0xC4 = 0;  // clear delayed fire counter
return 1;
```

---

## Charge Mode Path (IsChargeMode=yes, e.g., Tesla Coil)

When `BuildingTypeClass+0x16B8` (IsChargeMode) is true, the function uses a 3-state
machine stored in `BuildingClass+0xBC` (MissionState):

### State 0: Charge-Up Phase

```
MissionState == 0:

// Power check: if Powered=yes AND PowerDrain > 0, check power ratio
if (Type->Powered && Type->PowerDrain > 0) {
    powerRatio = HouseClass::GetPowerRatio(this->Owner);
    if (powerRatio < 1.0) {
        return 1;  // not enough power, wait
    }
}

// Validate target (same as normal path)
target = Resolve(TarCom);
if (target == NULL || !target->IsAlive()) {
    SetTarget(NULL);
    MissionState = 0;
    QueueMission(Guard, 0);
    NextMission();
    return 1;
}

// Check if charge timer (CDTimer at this+0x388) has expired
if (!CDTimerClass::Remaining()) {
    // Timer expired → check facing
    targetDir = GetTargetCoords(TarCom);
    currentFacing = TurretFacing.Current();
    facingDelta = abs(currentFacing - targetDir);

    if (facingDelta < 0x2001) {
        // Close enough to target facing → transition to FIRE state
        MissionState = 1;  // → go to State 1
        return 1;
    } else {
        // Need to rotate more, update facing target
        targetDir2 = GetTargetCoords(TarCom);
        RateTimer::Set(&TurretFacing, targetDir2);
        return 1;
    }
}
// Timer still running, wait
return 1;
```

**Key insight:** The charge-up phase waits for:
1. Sufficient power (power ratio >= 1.0)
2. A valid target
3. A charge timer to expire
4. The turret to face within 0x2000 units (~45 degrees) of the target

The facing tolerance of 0x2001 (in the 0–0xFFFF facing system) is approximately
45 degrees — much more generous than normal turret fire.

### State 1: Fire Phase

```
MissionState == 1:

// Validate target again
target = Resolve(TarCom);
if (target == NULL || !target->IsAlive()) {
    SetTarget(NULL);
    MissionState = 0;
    return 1;
}

// Check fire error
fireError = GetFireError(TarCom, 0, 1);

if (fireError == FIRE_ILLEGAL || fireError == FIRE_CANT || fireError == FIRE_BUSY) {
    // Target became invalid
    SetTarget(NULL);
    MissionState = 0;
    return 1;
}

if (fireError == FIRE_FACING) {
    // Still need to rotate — go back to State 0
    MissionState = 0;
    return 1;
}

if (fireError == FIRE_OK) {
    // FIRE! Call Fire_At for both weapons
    Fire_At(TarCom, 0);   // primary weapon
    Fire_At(TarCom, 1);   // secondary weapon
    MissionState = 0;     // reset to charge-up
    return 1;
}

// Any other error: return 1 and retry
return 1;
```

### State 2+: Random Delay (cooldown)

```
MissionState >= 2:

// Get mission timer rate and add random jitter
delay = MissionClass::GetMissionTimerEntry() * some_constant;
delay = Math::ftol(delay);
jitter = Random::RandomRanged(0, 2);
return delay + jitter;
```

This state is the "cooldown" after firing, returning a longer delay before the next
attack cycle. The delay is based on the mission's configured timer rate plus 0-2 frames
of random jitter.

---

## BuildingClass::GetFireError (0x00447F10)

**Signature:** `int GetFireError(target, weaponIdx, checkRange)`

### Building-Specific Checks (before TechnoClass)

```
// 1. Garrison fire check
if (Type->CanBeOccupied) {
    if (!Type->CanOccupyFire) {
        return FIRE_ILLEGAL (5);  // garrisonable but can't fire from garrison
    }
    occupantCount = GetOccupantCount();  // vtable+0x408
    if (occupantCount == 0) {
        return FIRE_ILLEGAL (5);  // no occupants
    }
}

// 2. Check if building is deploying/undeploying
// (corrected 2026-05-29: was "if (this->LocomotionTarget != 0)"; binary `decompile_function 0x00447F10`
// shows `TechnoClass__IsDeploying()` virtual call, not a direct LocomotionTarget field read — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
if (TechnoClass::IsDeploying()) {
    return FIRE_ILLEGAL (5);
}

// 3. EMPulseCannon check (TS legacy)
if (Type->EMPulseCannon) {  // +0x16C3
    return FIRE_CANT (6);
}

// 4. Construction/selling check
mission = GetMission();
if (mission == 0x13 || mission == 0x12) {  // Selling or Constructing
    // Skip remaining checks, allow firing (this is weird — possibly TS legacy)
    goto skip;
}

// 5. Power check
if (!CanSellOrUndeploy()) {    // vtable+0x350 (corrected 2026-05-29: was IsPowerOnline — ROOT_CAUSE: RTTI_LABEL_DRIFT)
    return FIRE_CANT (6);
}

// 6. Upgrade lock check
if (this->UpgradeLock != 0) {  // BuildingClass+0x714
    return FIRE_REARM (3);     // locked by upgrade, wait
}

// 7. Delegate to TechnoClass::GetFireError
result = TechnoClass::GetFireError(target, weaponIdx, checkRange);

// 8. Turret facing check (building-specific)
if (result == FIRE_OK && HasTurret()) {
    targetDir = GetTargetCoords(target);    // vtable+0x4E8
    currentFacing = TurretFacing.Current();

    // Calculate tolerance based on HasTurret flag
    // If Type->HasTurret (+0x16C5): tolerance = (0xF8 + 8) << 8 = 0x10000 (full circle = always OK? No...)
    // Actually: tolerance = ((-bool(HasTurret) & 0xF8) + 8) << 8
    //   HasTurret=true:  (0xF8 + 8) << 8 = 0x100 << 8 = 0x10000 → wraps to 0
    //   HasTurret=false: (0x00 + 8) << 8 = 0x0800
    // So buildings WITH turrets have tolerance=0 (must face exactly), without have ~11 degrees

    facingDelta = abs(currentFacing - targetDir);
    if (abs(tolerance) < facingDelta) {
        return FIRE_FACING (2);  // turret not facing target
    }
}

return result;
```

**IMPORTANT correction on turret tolerance math:**
The expression `(-(cVar2 != '\0') & 0xF8U) + 8` evaluates to:
- If HasTurret is true: `(0xFF & 0xF8) + 8 = 0xF8 + 8 = 0x100`, then `<< 8` = `0x10000`
  which as a signed short is 0 (wraps). This means turret buildings MUST face exactly.
- If HasTurret is false: `(0x00 & 0xF8) + 8 = 0x08`, then `<< 8` = `0x0800`
  This gives ~11 degree tolerance for non-turret buildings.

This is counterintuitive but correct: turret buildings must be precisely aimed, while
non-turret buildings (garrison fire ports) have a small tolerance window.

---

## TechnoClass::GetFireError (0x006FC0B0)

This is the base class fire validation, called by BuildingClass after building-specific checks.
Returns a `FireErrorType` enum value.

### Check Order (abbreviated — full function is ~700 bytes)

1. **Target NULL** → FIRE_ILLEGAL (5)
2. **BerzerkDuration active** (+0x2DC) → FIRE_ILLEGAL (5)
3. **Is deploying** (vtable+0x1D8) → FIRE_REARM (3)
4. **Is locomotor-targeting same object** → FIRE_REARM (3)
5. **Is warping** (vtable+0x1D4) → FIRE_ILLEGAL (5)
6. **Is entering transport** (+0x1C8) → FIRE_ILLEGAL (5)
7. **Is sinking** → FIRE_ILLEGAL (5)
8. **Target is transport we're entering** (+0x1CC) → FIRE_ILLEGAL (5)
9. **Target is our current transport** (+0x11C) → FIRE_ILLEGAL (5)
10. **Target is warp destination** → FIRE_REARM (3)
11. **Target in limbo** → FIRE_ILLEGAL (5)
12. **Target is cloaked ally** → FIRE_ILLEGAL (5)
13. **Friendly fire check** → FIRE_ILLEGAL (5)
14. **Target has Sensors, sensor reveals** → check FIRE_CANT (6)
15. **Target in temporal warhead state** → FIRE_ILLEGAL (5)
16. **Is parasited** (+0x8D) → FIRE_ILLEGAL (5)
17. **Is on bridge mismatch** → FIRE_CANT (6)
18. **Get weapon** → if no weapon, FIRE_CANT (6)
19. **Spawner weapon check** → various
20. **Assaulter weapon check** → various
21. **ROF timer check** → FIRE_REARM (3) if not expired
22. **Particle system active** → FIRE_REARM (3) (beam/laser/etc)
23. **Wave active** → FIRE_REARM (3)
24. **Ammo check** → FIRE_AMMO (1) if depleted
25. **Cloak fire check** → FIRE_CLOAKED (9)
26. **NoFireWhileMoving** → FIRE_BUSY (8)
27. **Range check** (via vtable+0x3A8) → FIRE_BUSY (8) if out of range

**Return values (FireErrorType enum):**
| Value | Name | Meaning |
|-------|------|---------|
| 0 | FIRE_OK | Can fire |
| 1 | FIRE_AMMO | Out of ammo |
| 2 | FIRE_FACING | Turret not facing target |
| 3 | FIRE_REARM | Reloading/waiting |
| 4 | FIRE_ROTATING | Body still rotating |
| 5 | FIRE_ILLEGAL | Invalid target |
| 6 | FIRE_CANT | Can't fire (power, etc) |
| 7 | FIRE_MOVING | Moving |
| 8 | FIRE_BUSY | Occupied/processing |
| 9 | FIRE_CLOAKED | Cloaked, can't fire |
| 10 | FIRE_CLOAKED_TARGET | Target is cloaked |

---

## BuildingClass::GetWeapon (0x004526F0)

**Signature:** `WeaponStruct* GetWeapon(int weaponIdx)`

`param_1` is `int*` so all array indices are multiplied by 4 for byte offsets.

### Logic

```
// 1. GARRISON WEAPON OVERRIDE
// Check OccupantSlotCount (BuildingClass+0x702)
if (this->OccupantSlotCount > 0) {
    // Iterate through occupant slots
    for (i = 0; i < OccupantSlotCount; i++) {
        occupant = this->Occupants[i];   // array at BuildingClass+0x5EC (param_1[0x17B])
        if (occupant != NULL) {
            // Get the occupant's InfantryType weapon
            weaponStruct = InfantryTypeClass::GetWeapon(occupant, weaponIdx);
            // FUN_007177c0: returns TypeClass + 0x898 + weaponIdx * 0x1C
            // This accesses the WeaponStruct array in the type class
            if (weaponStruct->WeaponType != NULL) {
                return weaponStruct;  // USE OCCUPANT'S WEAPON
            }
        }
    }
}

// 2. GATTLING WEAPON CHECK
isGarrisoned = IsGarrisoned();  // vtable+0x400
if (!isGarrisoned || this->OccupantArrayCount <= this->GarrisonFireIndex) {
    // Normal (non-garrison) path
    // FUN_0070e140: TechnoClass::GetWeapon - checks elite status
    weaponStruct = TechnoClass::GetWeapon(weaponIdx);
    return weaponStruct;
} else {
    // Garrison path: get weapon from current firing occupant
    occupant = OccupantArray[GarrisonFireIndex];
    isElite = VeterancyClass::IsElite(occupant);
    typeClass = occupant->Type;  // occupant[0x1B0] = InfantryTypeClass

    if (isElite) {
        // Elite occupant weapon at TypeClass+0xE20
        if (*(TypeClass + 0xE20) != NULL) {
            return (TypeClass + 0xE20);
        }
    } else {
        // Normal occupant weapon at TypeClass+0xE04
        if (*(TypeClass + 0xE04) != NULL) {
            return (TypeClass + 0xE04);
        }
    }
    // Fallback: use occupant's default weapon
    return occupant->GetWeapon(0);  // vtable+0x3F8
}
```

### Key Points

1. **Garrison weapons take priority.** If building has occupants, the FIRST occupant with
   a valid weapon determines the weapon used.

2. **Round-robin firing:** The `GarrisonFireIndex` (BuildingClass+0x664) cycles through
   occupants. Each tick, a different occupant fires.

3. **Elite vs normal:** Garrison occupants use different weapon offsets based on veterancy:
   - Normal: `InfantryTypeClass+0xE04` (OccupyWeapon)
   - Elite: `InfantryTypeClass+0xE20` (EliteOccupyWeapon)

4. **WeaponStruct layout:** Each weapon entry is 0x1C (28) bytes, accessed at
   `TypeClass + 0x898 + idx * 0x1C`.

---

## BuildingClass::HasTurret (0x004527D0)

```
bool HasTurret() {
    // Check building's own type
    if (Type->HasTurret_0xCA1)     // TechnoTypeClass+0xCA1
        return true;

    // Check occupant types (garrison can provide turret)
    for (i = 0; i < OccupantSlotCount; i++) {
        if (Occupants[i] != NULL && Occupants[i]->Type->HasTurret_0xCA1)
            return true;
    }
    return false;
}
```

---

## BuildingClass::IsOccupied (vtable+0x400, 0x00458DD0)

```
bool IsOccupied() {
    if (Type->CanBeOccupied && Type->CanOccupyFire) {
        return GetOccupantCount() > 0;
    }
    return false;
}
```

---

## Delayed Fire System

### BuildingTypeClass+0x16EC: DelayedFireDelay

Read from INI via `BuildingTypeClass::ReadINI` at 0x004611C7:
```
this->DelayedFireDelay = INI.ReadInt(section, "DelayedFireDelay", this->DelayedFireDelay);
```

### How It Works

The delayed fire system is used by buildings like the Prism Tower. The `DelayedFireDelay`
value specifies a tick count. When the building's attack animation plays, it waits this
many ticks before actually spawning the projectile.

In `Mission_Attack`, the delayed fire interacts through:
1. The `AnimComplete` flag (+0x6DD) is set to 1 before GetFireError is called
2. The field at `BuildingClass+0xC4` tracks delayed fire state
3. When clearing target (cases 1/5/6/8), `field_0xC4` is reset to 0

The actual delayed fire processing happens in `BuildingClass::ProcessDelayedFire` (0x004503F0),
called from `BuildingClass::UpdateAI` each tick, not from Mission_Attack directly.

### ProcessDelayedFire Fields (BuildingClass)

| Offset | Field | Notes |
|--------|-------|-------|
| +0x704 | DelayedFireType | 0=none, 1=weapon, 2=superweapon |
| +0x708 | DelayedFireTarget | Target pointer |
| +0x70C | DelayedFireParam2 | Additional parameter |

---

## Garrison Fire: Round-Robin Through Occupants

### Data Structures

- `BuildingClass+0x5EC` (array start, accessed as `param_1[0x17B]` in int* context):
  Array of occupant pointers (InfantryClass*)
- `BuildingClass+0x664`: GarrisonFireIndex — which occupant fires next
- `BuildingClass+0x694`: OccupantCount — total occupants
- `BuildingClass+0x702`: OccupantSlotCount — number of occupied slots

### Fire Sequence (in FIRE_OK handler)

The garrison fire sequence in the FIRE_OK case works as follows:

1. **GetWeapon** uses the current occupant's weapon (via GarrisonFireIndex)
2. **Fire_At** is called, which:
   - Gets the weapon from the current garrison occupant
   - Spawns a bullet from the occupant's fire port position
   - Uses the occupant's OccupyWeapon (or EliteOccupyWeapon if elite)
3. After firing, the **GarrisonFireIndex is incremented** (modulo OccupantCount)
4. The increment happens in `TechnoClass::Fire_At` at 0x006FDD50, specifically:
   ```
   if (IsGarrisoned() && this is BuildingClass) {
       this->GarrisonFireIndex++;
       this->GarrisonFireIndex %= GetOccupantCount();
   }
   ```

### Fire Port Coordinates

In `TechnoClass::Fire_At`, when the firing unit is a garrisoned building
(`TechnoTypeClass+0x691` flag set), the muzzle flash position cycles through 8
predefined offset positions:

```
// 8 fire port offsets (initialized once), each is {x, y, z}:
// Stored as static data at 0x00B0EAA8
offsets[0] = { 0x100,    0,    0xB4 }
offsets[1] = {    0, 0xB4,       0  }
offsets[2] = { 0x100,    0, -0xB4   }
offsets[3] = { -0xB4,    0,  0xB4   }
offsets[4] = { -0x100,   0,     0   }
offsets[5] = { -0xB4,    0, -0xB4   }
offsets[6] = {     0,    0, -0x100  }
offsets[7] = {  0xB4,    0, -0xB4   }

// For first burst: random starting offset
if (CurrentBurstIndex == 0) {
    randomPortIdx = Random(0, 7);
} else {
    // Subsequent bursts rotate through ports
    randomPortIdx = (randomPortIdx + 8/Burst) % 8;
}
muzzle = Location + offsets[randomPortIdx];
```

---

## IsBaseDefense Behavior

Buildings with `IsBaseDefense=yes` (TechnoTypeClass+0xCD5) — **WAIT**, this is actually
the `IsGattling` flag at that offset. Let me correct this.

Looking at the Mission_Attack code at 0x0044B113–0x0044B131:
```
if (Type->IsGattling) {                          // +0xCD5
    TechnoClass::UpdateGattlingStage(field_0xC4);  // decrease gattling value
    field_0xC4 = 0;
}
```

The `IsBaseDefense` flag is NOT directly referenced in Mission_Attack. Instead,
base defense buildings differ through:

1. **Mission assignment:** Base defenses are assigned Mission_Attack by the AI targeting
   system when enemies enter range, not through special Mission_Attack logic.

2. **GetFireError flow:** Base defenses use the same GetFireError path. The
   `TechnoTypeClass+0xCA1` (HasTurret) flag affects turret rotation requirements.

3. **Target acquisition:** Handled by `BuildingClass::UpdateAI` and threat scanning,
   not Mission_Attack itself. Base defenses auto-acquire targets through
   `TechnoClass::ScanForThreat`.

---

## TechnoClass::Fire_At (0x006FDD50) — Key Points for Buildings

This is the massive (~2400 byte) function that actually fires weapons. Key building-specific
behavior:

### Garrison Fire Port Selection

When `TechnoTypeClass+0x691` is set (IsGarrison flag on the building's type), Fire_At
uses the 8-position fire port system described above. The fire port offsets are relative
to the building's center position.

### Garrison Occupant Cycling

After firing, if `IsGarrisoned()` returns true and the firer is a BuildingClass:
```
GarrisonFireIndex++;
GarrisonFireIndex %= GetOccupantCount();
```

### ROF (Rate of Fire) Timer

After successful fire:
```
this->CurrentBurstIndex++;
ROFTimerDuration = GetROF();  // vtable+0x318
if (this->field_0x298 != 0) {  // rapid fire bonus?
    ROFTimerDuration /= 2;
}
this->ROFTimerStart = g_CurrentFrameCounter;
this->CurrentBurstIndex %= weapon->Burst;
```

### Weapon Effects Created by Fire_At

- **Spawner weapons** (IsSpawner): Redirect to SpawnManager
- **Assaulter weapons** (IsAssaulter): Enter transport
- **DiskLaser weapons**: Create DiskLaserClass
- **Normal weapons**: Create BulletClass → set trajectory
- **Wave weapons** (IsSonic/IsRailgun): Create WaveClass
- **Beam weapons** (IsRadBeam/IsElectricBolt): Create beam effects
- **Particle systems**: Spawn ParticleSystemClass if configured
- **Firing animation**: Create AnimClass at muzzle position

---

## Return Value

`Mission_Attack` always returns **1** (re-check next tick), except:
- **Case 2 (FIRE_FACING):** Returns **2** (faster re-check for turret rotation)
- **State 2+ (Charge Mode cooldown):** Returns **MissionTimerRate + Random(0,2)**

---

## Summary: Complete Attack Pipeline

```
BuildingClass::Mission_Attack
├── IsChargeMode=yes (Tesla Coil path)
│   ├── State 0: Check power → validate target → wait for charge timer → check facing
│   │            → if facing OK within 45°: MissionState=1
│   ├── State 1: Validate target → GetFireError → Fire_At (both weapons) → MissionState=0
│   └── State 2+: Return MissionTimerRate + Random(0,2) cooldown
│
└── IsChargeMode=no (Normal/Garrison path)
    ├── No target? → SetTarget(NULL), GarrisonFireIndex=0, QueueMission(Guard)
    ├── SelectWeaponAgainst(target) → set AnimComplete flag
    ├── GetFireError(target, weaponIdx, 1)
    ├── If FIRE_FACING: try turret rotation, re-check GetFireError
    ├── Switch on fire error:
    │   ├── FIRE_OK (0): Fire_At (garrison: round-robin occupants)
    │   ├── FIRE_AMMO/ILLEGAL/CANT/BUSY (1,5,6,8): Clear target → Guard
    │   ├── FIRE_FACING (2): Update turret facing, return 2
    │   ├── FIRE_REARM (3): Update turret facing, wait
    │   ├── FIRE_ROTATING/MOVING (4,7): Update facing
    │   ├── FIRE_RANGE (9): Update facing, keep tracking
    │   └── FIRE_CLOAKED (10): Update facing, keep tracking
    └── Cleanup: update turret facing toward target, clear field_0xC4
```

---

## BuildingClass Vtable Mapping (base at 0x007E3EBC)

| Offset | Function | Address |
|--------|----------|---------|
| +0x184 | MissionClass::GetMission | 0x005B3040 |
| +0x1E8 | MissionClass::QueueMission | 0x005B35E0 |
| +0x1EC | MissionClass::NextMission | 0x005B3570 |
| +0x2E4 | TechnoClass::SelectWeaponAgainst | 0x006F3330 | (corrected 2026-05-29: was "GetWeaponIndex"; binary `get_function_by_address 0x006F3330` shows `TechnoClass__SelectWeaponAgainst` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x350 | BuildingClass::CanSellOrUndeploy | 0x004555D0 | (corrected 2026-05-29: was "IsPowerOnline"; binary `get_function_by_address 0x004555D0` shows `BuildingClass__CanSellOrUndeploy` — it checks HasPower, EMPLock, health, power-ratio, upgrade locks; used as the "can-operate" gate in GetFireError — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| +0x3C0 | BuildingClass::GetFireError | 0x00447F10 |
| +0x3C8 | BuildingClass::SetTarget | 0x00443B90 |
| +0x3CC | TechnoClass::Fire_At | 0x006FDD50 |
| +0x3F8 | BuildingClass::GetWeapon | 0x004526F0 |
| +0x3FC | BuildingClass::HasTurret | 0x004527D0 |
| +0x400 | BuildingClass::IsOccupied | 0x00458DD0 |
| +0x408 | BuildingClass::GetOccupantCount | 0x004581F0 |
| +0x430 | IsFiring | 0x00705D50 |
| +0x4E8 | BuildingClass::GetTargetCoords | 0x0043ED40 |
