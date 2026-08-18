# Cloaking Visual Pipeline — Ghidra Research Report

## Summary

Complete reverse-engineering of the cloaking visual state machine in gamemd.exe,
covering CloakState transitions, per-tick timer advancement, visual state mapping,
blitter selection for each cloaking stage, ShouldUncloak decision logic,
CanAutoCloak eligibility, CloakShroud (gap generator fog), and the full vtable
dispatch chain. Confidence: HIGH (verified from binary, vtable offsets confirmed
against UnitClass/InfantryClass/BuildingClass/AircraftClass vtables).

## TechnoClass Cloaking Fields

All offsets relative to TechnoClass base (`this`):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x220 | DWORD | CloakState | 0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking |
| +0x224 | DWORD | CloakProgress | Animation progress counter. Counts UP (0→max) when cloaking, DOWN (max→0) when uncloaking |
| +0x228 | BYTE | CloakDirty | Set to 1 when CloakProgress changes, 0 when timer not ticking |
| +0x22C | CDTimer(12) | CloakStepTimer | Controls tick rate for CloakProgress advancement |
| +0x238 | DWORD | CloakingSpeed | Copied from TechnoTypeClass+0x310, timer duration per step |
| +0x23C | DWORD | CloakStepDelta | +1 when cloaking (state 1), -1 when uncloaking (state 3) |
| +0x240 | CDTimer(12) | SecondaryCloakGateTimer | Secondary cloak-eligibility gate checked by CanAutoCloak (start: param_1[0x90]=+0x240, duration: param_1[0x92]=+0x248); initialized to `g_CurrentFrameCounter` / 0 in constructor. Separate from the primary ReCloakDelayTimer at +0x2EC/+0x2F4. |
| +0x2EC | CDTimer(8)  | ReCloakDelayTimer | Primary re-cloak delay timer: after full uncloak, unit must wait for this to expire. start: param_1[0xBB]=+0x2EC, duration: param_1[0xBD]=+0x2F4; initialized to `g_CurrentFrameCounter` / 0 in constructor. Verified: decompile_function 0x006FBDC0, decompile_function 0x006F2B40. |
| +0x269 | BYTE | CloakShroudActive | 1 when cloak shroud cells are applied, 0 when removed |
| +0x26C | DWORD | CloakShroudRadius | Cached from TechnoTypeClass+0xCD2, radius for shroud effect |
| +0x3D2 | BYTE | HasStealthAbility | Runtime cloak flag — see initialization chain below |

TechnoTypeClass fields:
| Offset | Size | Field | INI Key | Notes |
|--------|------|-------|---------|-------|
| +0x29C | BYTE[18] | VeteranAbilities | `VeteranAbilities=` | Boolean array of veteran promotion abilities (see enum below) |
| +0x2A2 | BYTE | VeteranAbilities[CLOAK] | `VeteranAbilities=CLOAK` | = +0x29C + 6. Unit gains cloaking at veteran rank. Checked by ShouldUncloak/CanAutoCloak. |
| +0x2AE | BYTE[18] | EliteAbilities | `EliteAbilities=` | Boolean array of elite promotion abilities |
| +0x2B4 | BYTE | EliteAbilities[CLOAK] | `EliteAbilities=CLOAK` | = +0x2AE + 6. Unit gains cloaking at elite rank. |
| +0x310 | DWORD | CloakingSpeed | `CloakingSpeed=` | Per-type, frames between CloakProgress steps |
| +0x5F0 | DWORD | SensorsSight | `SensorsSight=` | Sensor detection range in cells. Used by AddSensorsAt/RemoveSensorsAt |
| +0x5F4 | DWORD | DetectDisguiseRange | `DetectDisguiseRange=` | Disguise detection range in cells |
| +0xC93 | BYTE | CloakStop | `CloakStop=` | Can't cloak while locomotion is busy (= moving). Checked by IsCloakable (vtable+0x288) |
| +0xC9A | BYTE | Invisible | `Invisible=` | ~~Previously documented as SensorsSight~~ **CORRECTED.** If set, unit is fully hidden (visual state 5) unless discovered |
| +0xC9D | BYTE | Sensors | `Sensors=` | Boolean sensor flag |
| +0xCD0 | BYTE | Cloakable | `Cloakable=` | Main INI cloakable flag |
| +0xCD2 | BYTE | GapRadiusInCells | `GapRadiusInCells=` | Gap generator fog radius |
| +0xCD3 | BYTE | SuperGapRadiusInCells | `SuperGapRadiusInCells=` | Super gap generator radius |

**VeteranAbilities / EliteAbilities Enum** (string table at 0x008463B8, parser at 0x00477640):

| Index | Name | Veteran Offset | Elite Offset |
|-------|------|----------------|--------------|
| 0 | FASTER | +0x29C | +0x2AE |
| 1 | STRONGER | +0x29D | +0x2AF |
| 2 | FIREPOWER | +0x29E | +0x2B0 |
| 3 | SCATTER | +0x29F | +0x2B1 |
| 4 | ROF | +0x2A0 | +0x2B2 |
| 5 | SIGHT | +0x2A1 | +0x2B3 |
| 6 | CLOAK | +0x2A2 | +0x2B4 |
| 7 | TIBERIUM_PROOF | +0x2A3 | +0x2B5 |
| 8 | VEIN_PROOF | +0x2A4 | +0x2B6 |
| 9 | SELF_HEAL | +0x2A5 | +0x2B7 |
| 10 | EXPLODES | +0x2A6 | +0x2B8 |
| 11 | RADAR_INVISIBLE | +0x2A7 | +0x2B9 |
| 12 | SENSORS | +0x2A8 | +0x2BA |
| 13 | FEARLESS | +0x2A9 | +0x2BB |
| 14 | C4 | +0x2AA | +0x2BC |
| 15 | TIBERIUM_HEAL | +0x2AB | +0x2BD |
| 16 | GUARD_AREA | +0x2AC | +0x2BE |
| 17 | CRUSHER | +0x2AD | +0x2BF |

WeaponTypeClass fields:
| Offset | Size | Field | INI Key | Notes |
|--------|------|-------|---------|-------|
| +0x133 | BYTE | DecloakToFire | `DecloakToFire=` | Unit must uncloak before firing this weapon |

BuildingTypeClass fields:
| Offset | Size | Field | INI Key | Notes |
|--------|------|-------|---------|-------|
| +0x16C7 | BYTE | CloakGenerator | `CloakGenerator=` | This building is a gap/cloak generator |
| +0x16C8 | BYTE | SensorArray | `SensorArray=` | This building is a sensor array |
| +0x1707 | BYTE | CloakRadiusInCells | `CloakRadiusInCells=` | Building-specific cloak/gap radius |

RulesClass globals:
| Offset | Size | Field | INI Key | Default |
|--------|------|-------|---------|---------|
| +0x628 | DWORD | CloakingStages | `[General] CloakingStages=` | 9 |
| +0x1410 | DOUBLE | CloakDelay | `[General] CloakDelay=` | 0.02 (minutes) |
| +0x1700 | DOUBLE | ConditionYellow | `[General] ConditionYellow=` | (corrected 2026-07-18: added — verified: decompile_function 0x006691E0, `RulesClass::ReadAudioVisual` writes ConditionYellow at param_1[0x5c0]=+0x1700 — OFFSET_RETYPED_WRONG) |
| +0x1708 | DOUBLE | ConditionRed | `[General] ConditionRed=` | (corrected 2026-07-18: was documented as ConditionYellow at this offset; binary shows +0x1708=ConditionRed and +0x1700=ConditionYellow — verified: decompile_function 0x006691E0. CloakingTick's health-threshold checks below read +0x1708, i.e. **ConditionRed**, not ConditionYellow — OFFSET_RETYPED_WRONG) |

BuildingClass additional fields:
| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x55C | DWORD[21] | AnimSlots | 21 animation pointers for building cloak-in/out effects |
| +0x660 | BYTE | CloakAnimDirection | 0 = cloaking in, 1 = cloaking out |
| +0x6ED | BYTE | BuildingCloakStage | Separate building cloak counter (0-10+), used by gap generators |

## Vtable Dispatch Map for Cloaking

All vtable offsets verified against labeled vtables:
- `vtable__UnitClass` @ 0x007f5c70
- `vtable__InfantryClass` @ 0x007eb058
- `vtable_BuildingClass` @ 0x007e3ebc
- `vtable__AircraftClass` @ 0x007e22a4

| Vtable Offset | Function | Address (shared) | Description |
|---------------|----------|-----------------|-------------|
| +0x68 | GetVisualState | varies | Returns visual state 0-5 |
| +0x84 | GetTypeClass | varies | Returns TechnoTypeClass pointer |
| +0x124 | MarkForRedraw | varies | Flag for redraw with priority param |
| +0x288 | IsCloakable | varies | Check if unit has cloaking capability |
| +0x2A0 | CanAutoCloak | 0x006fbdc0 | Full eligibility check for auto-cloaking |
| +0x2A4 | ShouldUncloak | 0x006fbc90 | Decision: should a cloaked unit uncloak? |
| +0x37C | IsFiring | varies | Returns true if currently firing |
| +0x380 | IsTeleporting | varies | Returns true if chrono-teleporting |
| +0x1D4 | IsWarping | varies | Returns true if chrono-warping (1) |
| +0x1D8 | IsWarping2 | varies | Returns true if chrono-warping (2) |
| +0x43C | ModifyCloakDrawFlags | 0x0070ed80 | Allied shimmer cycle for draw flags |
| +0x45C | StartUncloaking | 0x007036c0 | Initiate uncloak animation |
| +0x460 | StartCloaking | 0x00703770 | Initiate cloak animation |

**BuildingClass overrides** at the same vtable offsets:
| Vtable Offset | Override | Address |
|---------------|----------|---------|
| +0x2A0 | BuildingClass::CanCloak | 0x00457770 |
| +0x2A4 | BuildingClass::ShouldUncloak | 0x004578c0 |

## CloakState Enum

```
0 = Uncloaked    — fully visible, no cloaking active
1 = Cloaking     — fade-out animation in progress (CloakProgress counting UP)
2 = Cloaked      — fully invisible (to enemies)
3 = Uncloaking   — fade-in animation in progress (CloakProgress counting DOWN)
```

## State Machine — Full Transition Graph

```
                  StartCloaking()           fully done
    [0 Uncloaked] ───────────────→ [1 Cloaking] ──────────→ [2 Cloaked]
         ↑                              │                       │
         │                              │ enemy detected,       │ ShouldUncloak
         │                              │ low health,           │ check (vtable+0x2a4)
         │                              │ 10% chance            │
         │                              ↓                       ↓
         │                         StartUncloaking()      StartUncloaking()
         │                              │                       │
         │        fully done            ↓                       ↓
         ←──────────────────── [3 Uncloaking] ←─────────────────┘
```

Cross-transitions also exist:
- State 1→3: `StartUncloaking()` can be called while cloaking (e.g., firing)
- State 2→3: `StartUncloaking()` when fully cloaked and detected or ordered to fire
- State 0→1: `StartCloaking()` from uncloaked state
- State 3→1: `StartCloaking()` can restart cloaking during uncloak

## Key Functions

### TechnoClass::StartCloaking — 0x00703770 (vtable+0x460)

Transitions: CloakState 0→1 or 3→1

```
Precondition: CloakState == 0 OR CloakState == 3
Actions:
  - Call vtable+0xDC (MarkAllOccupationBits) with param 0
  - CloakState = 1
  - CloakProgress = 0
  - CloakingSpeed = TechnoTypeClass->CloakingSpeed  (offset +0x310)
  - CloakStepTimer = { start=now, duration=CloakingSpeed }
  - CloakStepDelta = +1  (counting UP)
  - If not silent: play CloakSound at location (FUN_007509e0)
  - If not owned by player AND IsDiscoveredByCurrentPlayer: call vtable+0x150
```

### TechnoClass::StartUncloaking — 0x007036C0 (vtable+0x45C)

Transitions: CloakState 2→3 or 1→3

```
Precondition: CloakState == 2 OR CloakState == 1
Actions:
  - CloakState = 3
  - CloakProgress = Rules->CloakingStages - 1  (start from max)
  - CloakingSpeed = TechnoTypeClass->CloakingSpeed
  - CloakStepTimer = { start=now, duration=CloakingSpeed }
  - CloakStepDelta = -1  (counting DOWN)
  - If not silent: play CloakSound at location
```

### TechnoClass::DoCloak — 0x004D3780

Wrapper that calls StartCloaking with additional game logic:

```c
int DoCloak(TechnoClass* this, int mode) {
    if (mode == 2): return 1;  // already handling

    if (!CanEnterCloak(mode)): return 0;  // permission check

    cell = GetMapCell();  // vtable+0x78
    if (cell == 2):  // valid cell
        GetCenterCell();  // vtable+0x1b8
        if (mode == 0):
            FireTriggerEvent_Cloak();    // 0x005687f0
        elif (mode == 1 || mode == 3):
            FireTriggerEvent_ReCloak();  // 0x005683c0
            return 1;
    return 1;
}
```

### TechnoClass::DoUncloak — 0x006F4EB0

Wrapper that handles consequences of uncloaking:

```c
void DoUncloak(TechnoClass* this) {
    // Get cell position
    // If CloakState == 2 AND not allied with current player:
    //     Discover unit (vtable+0x150) — makes it visible to player

    // If cell visible to owner AND CanAutoCloak returns true:
    //     Gather units whose current Target (+0x2B4) == this unit (NOT mind-controlled
    //     units — corrected 2026-07-18, see "Target Re-Notification on Cloak Complete" below;
    //     verified: decompile_function 0x006F4EB0 shows the identical `iVar1+0x2b4 == param_1`
    //     pattern as the cloak-complete path — INFERENCE_HARDENED)
    //     Call StartCloaking(0) — immediately try to re-cloak
    //     For each collected unit: call vtable+0x3C8 (Assign_Target, not "scatter") passing
    //     this unit — re-assigns their target reference via Set_ArchiveTarget
}
```

### ShouldUncloak — 0x006FBC90 (vtable+0x2A4)

Shared implementation for UnitClass, InfantryClass, AircraftClass. Determines
whether a fully-cloaked unit (CloakState==2) should uncloak on this tick.

```c
bool ShouldUncloak(TechnoClass* this) {
    // Phase 1: Activity check
    if (IsCloakable() || this->HasStealthAbility) {
        // If NOT firing, NOT teleporting, NOT warping → stay cloaked
        if (!IsFiring() && !IsTeleporting() && !IsWarping() && !IsWarping2())
            return false;  // no activity, stay cloaked
    }
    // If we reach here: either no cloak ability, or unit is actively doing something

    // Phase 2: Veteran/Elite ability check — gated on the UNIT'S OWN RANK, not its type
    // (corrected 2026-07-18: previously documented as an IsInfantryType/IsVehicleType branch;
    //  no such type check exists in the binary — verified: decompile_function 0x006FBC90 — INFERENCE_HARDENED)
    TypeClass* type = GetTypeClass();
    if (IsVeteran(this)) {
        if (type->VeteranAbilities[CLOAK])  // +0x2A2
            return false;  // veteran-ranked unit with veteran CLOAK ability → stay cloaked despite activity
    } else if (IsElite(this)) {
        if (type->VeteranAbilities[CLOAK] || type->EliteAbilities[CLOAK])  // +0x2A2 or +0x2B4
            return false;  // elite-ranked unit checks BOTH arrays → stay cloaked despite activity
    }

    // Phase 3: Cell visibility check
    cell = GetCoords();  // convert to cell
    owner_house = this->OwnerHouse;  // +0x21C
    if (!CellVisibleToHouse(cell, owner_house->ArrayIndex))
        return true;   // cell not in owner's vision → uncloak

    return false;  // stay cloaked
}
```

**Interpretation:** Units with veteran/elite CLOAK ability stay cloaked even when
active (firing, etc.) — the veteran promotion grants persistent cloaking.
Units WITHOUT these abilities that became cloaked (e.g., via gap generator or
innate Cloakable=yes) will uncloak when their cell is no longer shrouded.
Note (corrected 2026-07-18): the branch is on the unit's own promotion **rank**
(Veteran vs Elite), not its **type** (infantry vs vehicle) — a Veteran-ranked
unit checks only `VeteranAbilities[CLOAK]`; an Elite-ranked unit checks BOTH
`VeteranAbilities[CLOAK]` and `EliteAbilities[CLOAK]`. The previous wording
("infantry only checks Veteran, vehicles check both") did not match the binary
— verified: decompile_function 0x006FBC90 — INFERENCE_HARDENED.

### BuildingClass::ShouldUncloak — 0x004578C0 (override)

Extends the base ShouldUncloak with building-specific logic:

```c
bool BuildingClass::ShouldUncloak() {
    // First check base TechnoClass logic
    if (TechnoClass::ShouldUncloak())
        return true;

    // Additional check: iterate all cells in building foundation
    cell = GetCenterCell();
    (width, height) = GetFoundationBounds();  // from BuildingTypeClass+0x520
    for dy in (-1..=height):
        for dx in (-1..=width):
            occupant = GetCellOccupant(cell + (dx, dy));
            if (occupant != null && occupant != this):
                if (occupant->OwnerHouse != this->OwnerHouse):
                    type = occupant->GetTypeClass();
                    if (type->field_0xC9D):  // detection capability flag
                        return true;  // enemy with detection on our foundation

    return false;  // no detection threat
}
```

### CanAutoCloak — 0x006FBDC0 (vtable+0x2A0)

Full eligibility check for whether a unit can begin auto-cloaking. Called from
the CloakingTick when in state 0 (uncloaked) and from the uncloaking completion
path (state 3→0 transition, visual state 1).

```c
bool CanAutoCloak(TechnoClass* this) {
    // Check basic cloaking ability
    if (!IsCloakable()) {
        // Fall back to type-range checks (infantry/vehicle)
        TypeClass* type = GetTypeClass();
        if (IsInfantryType) {
            if (!type->CloakableFlag)  // +0x2A2
                goto cell_check;
        } elif (IsVehicleType) {
            if (!type->CloakableFlag && !type->VehicleCloakFlag)  // +0x2A2, +0x2B4
                goto cell_check;
        } else {
            goto cell_check;
        }
    }

    cell_check:
    // Check cell visibility to owner
    cell = GetCoords();
    if (!CellVisibleToOwner(cell) && !this->HasStealthAbility)
        return false;

    // Already fully cloaked? Can't cloak again
    if (CloakState == 2): return false;

    // Check ReCloakDelayTimer — must have expired
    if (ReCloakDelayTimer remaining > 0): return false;

    // Check if unit is being targeted/attacked
    if (CurrentTarget != 0 && IsTargetAttacking(CurrentTarget))
        return false;

    // Non-building check: CloakProgress must be 0
    if (WhatAmI() != Building && CloakProgress != 0): return false;

    // Check another delay timer (secondary re-cloak)
    if (timer2 remaining > 0): return false;

    // Check if unit is deployed and in specific state
    if (IsDeployed && IsSelected && field_0x6AD): return false;

    // Final state check
    return (vtable+0x1C8() < 1);  // some mission/state check
}
```

### HasStealthAbility (+0x3D2) Initialization Chain

The runtime cloaking flag at TechnoClass+0x3D2 is NOT parsed from INI directly.
It's **copied from the type class** during construction:

```
INI Parse:                TechnoTypeClass::ReadINI (0x00712170)
                          "Cloakable=" → TechnoTypeClass+0xCD0
                                          ↓
TechnoClass Constructor:  TechnoClass::Constructor (0x006F2B40)
  0x006F2F4B:             this->HasStealthAbility = 0;  // initialize to false
                                          ↓
Subclass Constructors:    COPY type flag → instance flag
  UnitClass (0x7355B6):   this->HasStealthAbility = UnitTypeClass->Cloakable;
                          *(this+0x3D2) = *(this->Type[+0x6C4] + 0xCD0)
  InfantryClass (0x517D88): this->HasStealthAbility = InfantryTypeClass->Cloakable;
                          *(this+0x3D2) = *(this->Type[+0x6C0] + 0xCD0)
  AircraftClass:          does NOT copy — uses vtable+0x288 override instead
  BuildingClass:          does NOT copy — uses vtable+0x288 override instead
                                          ↓
Runtime Modification:     CrateClass::PickupCloak (0x0048294F)
  0x0048294F:             *(this+0x3D2) = 1;  // cloaking device crate
```

**Key insight:** Each concrete subclass (Unit, Infantry) copies its type's
`Cloakable=` flag to the instance during construction. Aircraft and Building
classes do NOT use +0x3D2 — they handle cloaking entirely through their
virtual `IsCloakable` (vtable+0x288) override, which may read the type flag
directly.

The +0x3D2 field is mutable at runtime: crate pickup can grant cloaking to
any unit/infantry, but this modification only affects the instance, not the type.

**UnitClass::Unlimbo** (0x00737BD9) also reads +0x3D2: if a unit is placed
on the map with HasStealthAbility already set, it starts fully cloaked
(CloakState = 2) immediately.

### IsCloakable — 0x004DBDA0 (vtable+0x288, FootClass)

Checks whether the unit currently has cloaking capability and is allowed to cloak:

```c
bool FootClass::IsCloakable() {
    if (!this->HasStealthAbility)  // +0x3D2
        return false;  // no cloaking ability at all

    TypeClass* type = GetTypeClass();
    if (type->CloakStop) {  // +0xC93, INI "CloakStop="
        // Unit can't cloak while moving
        ILocomotion* loco = this->Locomotion;  // +0x674 (param_1[0x19D])
        if (loco->IsBusy())  // locomotion vtable+0x80
            return false;  // locomotion is active → can't cloak
    }
    return true;  // can cloak
}
```

**CloakStop** means "stop moving to cloak" — units with this flag (like Mirage Tank)
can only cloak when stationary. `HasStealthAbility` (+0x3D2) is a runtime flag set
when the unit gains cloaking capability (innate via type or granted by a device).

### Cell Visibility Functions

Two critical cell-level checks used throughout the cloaking system:

**CellClass::IsVisibleToHouse** (0x004870B0):
```c
bool IsVisibleToHouse(CellClass* this, byte house_index) {
    return (this->VisibilityBitmask & (1 << (house_index & 0x1F))) != 0;
    // CellClass+0x78: 32-bit bitmask, 1 bit per house (supports up to 32 houses)
}
```

**CellClass::SensorCountForHouse** (0x004870D0) — (corrected 2026-07-18: was labeled
"GapCountForHouse"; live Ghidra symbol is `CellClass__SensorCountForHouse`, and the field
it reads (CellClass+0x7C) is the same per-house **sensor** counter array documented in the
"Sensor Detection vs Cloaking" section below, not a gap-generator counter — verified:
decompile_function 0x004870d0 — INFERENCE_HARDENED):
```c
bool SensorCountForHouse(CellClass* this, int house_index) {
    return this->SensorCountArray[house_index] > 0;
    // CellClass+0x7C: array of shorts, 2 bytes per house — sensor coverage count, NOT gap count
}
```

### IsPlayerControlled — 0x0050B6F0

Determines if a unit is controlled by the local player (used for allied shimmer):

```c
bool IsPlayerControlled(HouseClass* house) {
    if (g_GameMode != 0)  // multiplayer
        return house == g_PlayerPtr;
    // single-player: check observer flags at +0x1EC, +0x1ED
    return (house->field_0x1EC != 0 || house->field_0x1ED != 0);
}
```

### DecloakToFire — TechnoClass__GetFireError (0x006FC0B0)

The weapon-level `DecloakToFire=yes` flag is checked in the fire error evaluation
function. This is a large (~412 lines) function that returns a FireError enum
for the given weapon/target combination:

```c
int GetFireError(TechnoClass* this, ...) {
    // ... many checks for range, ammo, busy, etc. ...

    // DecloakToFire check (WeaponTypeClass+0x133)
    if (weapon->DecloakToFire) {         // +0x133
        if (this->CloakState != 0) {     // unit is in some cloaking state
            int whatAmI = this->WhatAmI();
            if (whatAmI != 2 || this->CloakState == 2) {
                // Not a vehicle, OR fully cloaked → must decloak first
                return 9;  // FIRE_MUST_DECLOAK
            }
        }
    }

    // ... more checks ...
}
```

**FireError return values** (partial, cloaking-relevant):

| Value | Meaning |
|-------|---------|
| 0 | FIRE_OK — can fire |
| 1 | FIRE_NO_AMMO |
| 3 | FIRE_BUSY — weapon reloading, burst timer, etc. |
| 5 | FIRE_ILLEGAL — target invalid, warhead mismatch, etc. |
| 6 | FIRE_MOVING — some weapon types can't fire while moving |
| 8 | FIRE_ILLEGAL2 — TypeClass+0xD27 flag check |
| 9 | FIRE_MUST_DECLOAK — DecloakToFire set and unit is cloaked |

When the caller receives `FIRE_MUST_DECLOAK` (9), it triggers `StartUncloaking()`
before attempting to fire again on the next tick. The unit enters CloakState 3
(uncloaking), and once fully visible (CloakState 0), the fire check passes.

**Note:** Vehicles (WhatAmI==2) that are NOT fully cloaked (CloakState 1 or 3,
i.e., in transition) are allowed to fire even with DecloakToFire — only fully
cloaked vehicles (CloakState==2) must decloak.

### Cloaking Tick Update — 0x006FB740 (called from TechnoClass::AI)

This is the per-tick state machine that advances cloaking. Main structure:

```
if (CloakState == 0):
    // UNCLOAKED PATH — check if we should start cloaking
    Check: IsCloakable (vtable+0x288), IsFiring (vtable+0x37C),
           IsTeleporting (vtable+0x380), IsWarping (vtable+0x1D4/0x1D8)
    If none active AND has Cloakable/Stealth type flags:
        Check CanAutoCloak (vtable+0x2A0)
        If conditions met:
            if (health ratio > ConditionRed [RulesClass+0x1708]): StartCloaking(0) unconditionally
            else: 4% chance PER TICK (RandomRanged(0,99) < 4) to StartCloaking(0) anyway
            // (corrected 2026-07-18: was "if below ConditionYellow, random 4% chance to NOT
            //  cloak" — inverted (binary is a 4% chance TO cloak while damaged, not a chance to
            //  skip cloaking), and the threshold field is ConditionRed at +0x1708, not
            //  ConditionYellow — verified: decompile_function 0x006FB740 —
            //  OPERATOR_OR_ORDER_DRIFT + OFFSET_RETYPED_WRONG)
    return

// CLOAKING/UNCLOAKING PATH (CloakState != 0):
// Advance the timer
if (CloakStepTimer has NOT expired) AND (CloakingSpeed[+0x238] != 0):
    CloakProgress += CloakStepDelta  // +1 or -1
    CloakDirty = 1
    Restart CloakStepTimer = { start=now, duration=CloakingSpeed }
else:
    CloakDirty = 0

// Clamp CloakProgress >= 0
if (CloakProgress < 0): CloakProgress = 0

switch (CloakState):
    case 3 (Uncloaking):
        MarkForRedraw(2)
        visibility = GetVisualState(1, 0)
        if (visibility == 0):
            // Fully uncloaked
            CloakingSpeed = 0
            CloakStepTimer = { start=now, duration=0 }
            CloakProgress = 0
            CloakState = 0
            // Start re-cloak delay timer:
            ReCloakDelayTimer = { start=now, duration=(int)(CloakDelay * 900.0) }
            MarkForRedraw(2)
        elif (visibility == 1):
            // Almost uncloaked — if still cloakable, re-start cloaking
            if CanAutoCloak(): StartCloaking(1)

    case 2 (Fully Cloaked):
        if ShouldUncloak(): StartUncloaking(0)

    case 1 (Cloaking):
        MarkForRedraw(2)
        if CloakingSpeed == 0: set timer to {1, 1}  // ensure minimum tick rate
        visibility = GetVisualState(1, 0)
        if (visibility == 2):
            // Partially visible — check health
            if (health <= ConditionRed [RulesClass+0x1708]) AND (random(0,99) < 10):
                StartUncloaking(1)  // 10% chance per tick to abort cloaking
                // (corrected 2026-07-18: threshold field is ConditionRed, not ConditionYellow —
                //  verified: decompile_function 0x006691E0 — OFFSET_RETYPED_WRONG)
        elif (visibility == 3) OR (visibility == 5):
            // Cloaking complete
            CloakState = 2
            CloakingSpeed = 0
            CloakStepTimer = { start=now, duration=0 }
            CloakProgress = 0
            MarkForRedraw(2)
            // If infantry and has deploy animation: call vtable+0xFC
            // Otherwise: re-notify units currently targeting this unit (see "Target
            // Re-Notification on Cloak Complete" — corrected 2026-07-18, was "scatter
            // nearby mind-controlled units")
```

## Visual State Mapping — TechnoClass_GetVisualState (0x00703860)

The function computes a visual state 0-5 from CloakState and CloakProgress.

### Logic

```
// Check if Invisible flag makes unit fully hidden
if (TypeClass->Invisible != 0) AND (this->IsDiscoveredByCurrentPlayer == false):
    if NOT in developer/spectator mode: return 5 (invisible)

// If CloakState == 0: return 0 (opaque)
// If in developer mode: return 0 (always visible)
// If WhatAmI() == 6 (Building): return 0

if (CloakState == 2):  // Fully Cloaked
    if (param_2 != 0):  // perspective-aware check (rendering for a specific viewer house)
        if (param_3 == 0): return 5  // no viewer house → invisible
        // Check SENSOR coverage of this cell for the viewer house — NOT general cell
        // visibility/shroud (corrected 2026-07-18: was "CellVisibleToHouse"; binary calls
        // CellClass__SensorCountForHouse at 0x004870D0 — verified: disassemble_function
        // 0x00703860 shows CALL 0x004870d0 — INFERENCE_HARDENED)
        if (SensorCountForHouse(cell, viewer.ArrayIndex)): return 3  // sensor coverage: semi-transparent
        return 5  // no sensor coverage: invisible
    // param_2 == 0: local-player rendering check
    if (g_hWnd == 0): return 3  // no window (headless) → semi-transparent
    if (IsDiscoveredByCurrentPlayer): return 3  // already discovered → semi-transparent
    // Check SENSOR coverage of this cell for g_PlayerPtr (added 2026-07-18 — this check was
    // missing from the doc entirely; verified: decompile_function 0x00703860)
    if (SensorCountForHouse(cell, g_PlayerPtr.ArrayIndex)): return 3
    if (g_GameMode == 0): return 5  // singleplayer, no sensor coverage, not discovered
    if (Owner == null || g_PlayerPtr == null): return 5
    if (IsAlliedWithPlayer, mutual check): return 3
    return 5  // enemy, no sensor coverage: invisible

if (CloakState == 1 or 3):  // Animating
    if (CloakProgress > 0):
        visual = (int)((double)CloakProgress / (double)CloakingStages * 256.0)

        if (visual < 0x40):  return 1   // 0-24% progress
        if (visual < 0x80):  return 2   // 25-49% progress
        if (visual < 0xC0):  return 3   // 50-74% progress
        if (visual >= 0xFF): return 5   // 100% (fully done)
        // 75-99%: return 4, BUT if param_2==0 AND IsDiscoveredByCurrentPlayer:
        //   return 3 instead of 4 (clamp for allied view)
        return (visual >= 0xFF) ? 5 : 4;
```

### Visual State Return Values

| Value | Progress Range | Visual Meaning |
|-------|---------------|----------------|
| 0 | N/A | Fully opaque (uncloaked or forced visible) |
| 1 | 0-24% | Light shimmer / warp effect |
| 2 | 25-49% | Heavier distortion |
| 3 | 50-74% | Semi-transparent (50% alpha blend) |
| 4 | 75-99% | Near-invisible (same blend, or conditional) |
| 5 | 100% or invisible | Skip draw entirely |

### CloakProgress Walkthrough Example (CloakingStages=9)

When cloaking (state 1), CloakProgress counts from 0 upward by +1 each CloakingSpeed ticks:

| CloakProgress | visual = Progress/9*256 | Visual State |
|---------------|-------------------------|--------------|
| 0 | 0 | 0 (still opaque, progress must be > 0) |
| 1 | 28 | 1 (28 < 0x40) |
| 2 | 56 | 1 (56 < 64) |
| 3 | 85 | 2 (85 >= 0x40, < 0x80) |
| 4 | 113 | 2 (113 < 128) |
| 5 | 142 | 2 (142 < 192) |
| 6 | 170 | 2 (170 < 192) |
| 7 | 199 | 3 (199 >= 0xC0) |
| 8 | 227 | 4 (227 >= 0xC0, < 0xFF) |
| 9+ | 256+ | 5 (>= 0xFF) → triggers transition to CloakState=2 |

So with CloakingStages=9, the progression is:
0→opaque, 1-2→state1, 3-6→state2, 7→state3, 8→state4, 9→state5

When uncloaking (state 3), CloakProgress starts at CloakingStages-1=8 and counts DOWN:
8→state4, 7→state3, 6-3→state2, 2-1→state1, 0→state0 (triggers CloakState=0)

## VXL Cloaking Draw — TechnoClass__Draw (0x00706640)

Voxel units use the same visual state system but different flag encoding:

```c
uint flags = 0x2000;  // base VXL flag (custom frame index)
if (param_11 == 0) {
    switch (GetVisualState(0, 0)) {
        case 0: flags = 0x2000; break;  // opaque VXL
        case 1: flags = 0x2002; break;  // shimmer (z-read)
        case 2:
        case 3: flags = 0x2004; break;  // 50% blend (z-write)
        case 4:
            if (CloakProgress == 0)
                flags = 0x200A;          // z-read + brightness variant
            else
                flags = 0x200C;          // z-write + brightness variant
            break;
        case 5: return;                  // skip draw entirely
    }
}

// Same IsWarping/IsTeleporting adds 0x04/0x06 (including building gate check)
// Same IsPlayerControlled → ModifyCloakDrawFlags shimmer modulation
// Always OR with 0x800 (remap)
// Flags passed to VXL_CacheBlit or TechnoClass__Render
```

VXL state 4 uses 0x200A (z-read|brightness) or 0x200C (z-write|brightness)
instead of SHP's simple 0x02/0x04, providing smoother near-invisible transitions
for 3D units.

## Blitter Selection in TechnoClass_DrawSHP (0x00705E00)

### Visual State → Draw Flags

The DrawSHP function converts visual state to Z-buffer/blend flags:

```c
uint flags = 0;
if (param_15 == 0) {  // not a special draw mode
    switch (visual_state) {
        case 0: flags = 0x00; break;  // opaque
        case 1: flags = 0x02; break;  // z-read mode
        case 2:
        case 3: flags = 0x04; break;  // z-write mode (50% blend)
        case 4:
            if (CloakProgress != 0)
                flags = 0x04;          // same as 2/3 if still animating
            else
                flags = 0x02;          // z-read if progress exhausted
            break;
        case 5: return;               // skip draw entirely
    }
}

// IsWarping/IsTeleporting adds 0x04 flag (z-write for chrono effect)
// Special case: buildings with flag at BuildingTypeClass+0x16B1 → flags = 0x06

// Always OR in 0x800 (remap flag) and 0x600 (standard draw)
flags |= 0x800;

// Building underside check (IsInfantry && field_0x6D3): adds 0x20 flag, clears alpha bits

// Allied shimmer modulation (only when IsPlayerOwned via vtable+0xC4):
//   flags = ModifyCloakDrawFlags(flags);  // vtable+0x43C

// Custom frame index → OR 0x2000
// Mirror/flip → OR 0x4000
// Mask out param_13 bits: flags &= ~param_13
```

### Flag Bits Meaning

| Bit | Value | Meaning |
|-----|-------|---------|
| 0x01 | Shadow | Shadow/darken blitter |
| 0x02 | Shimmer | 75/25 alpha blend (75% source, 25% dest) — used for early/late cloaking |
| 0x04 | Semi-transparent | 50/50 alpha blend — used for mid-cloaking |
| 0x06 | Combined | Shimmer + semi-transparent (chrono warp + cloak) |
| 0x08 | Brightness | Brightness-adjusted blitter variant |
| 0x10 | Z-buffer active | Set when z_height != 0 |
| 0x20 | Building underside | Flat building rendering |
| 0x40 | Flag 0x40 | Additional blitter selection flag |
| 0x100 | Flag 0x100 | Additional blitter selection flag |
| 0x200 | Center offset | Apply SHP center offset |
| 0x400 | Standard draw | Normal draw mode |
| 0x800 | Remap | Apply house color remapping |
| 0x2000 | Custom frame index | |
| 0x3000 | Frame mask | Mask checked for blitter selection |
| 0x4000 | Mirror/flip | |
| 0x8000 | Flag 0x8000 | Additional blitter selection flag |
| 0x10000 | Alt blend 1 | Alternative alpha blitter |
| 0x20000 | Alt blend 2 | Alternative alpha blitter |

### Blitter Selection (Blitter_selector @ 0x00490B90)

The `flags & 6` value selects the blitter family. The full selector is a large
decision tree (~50 blitter variants) considering flags 0x10, 0x20, 0x4000, 0x3000,
0x800, 0x08, 0x8000, 0x100, 0x40, 0x10000, 0x20000. The primary cloaking-relevant
paths:

| flags & 6 | With 0x800 (remap) | Blitter Table Offset | Visual Effect |
|-----------|---------------------|---------------------|---------------|
| 0x00 | 0x800 | +0x6C | **Opaque with remap** — standard fully-visible unit draw |
| 0x02 | 0x802 | +0x7C | **Z-read + remap** — shimmer/warp distortion effect |
| 0x04 | 0x804 | +0x78 | **Z-write + remap** — 50% alpha blend (semi-transparent) |
| 0x06 | 0x806 | +0x74 | **Z-read+write + remap** — combined (chrono warp + cloak) |

Additional blitter variants for special flag combinations:

| flags & 6 | Extra flags | Offset | Effect |
|-----------|-------------|--------|--------|
| 0x02 | +0x3000 | +0xA8 | Z-read with frame mask |
| 0x02 | +0x3000+0x08 | +0xB4 | Z-read + frame mask + brightness |
| 0x04 | +0x3000 | +0xA4 | Z-write with frame mask |
| 0x04 | +0x3000+0x08 | +0xB0 | Z-write + frame mask + brightness |

### Blitter Per-Pixel Operations (verified from decompilation)

**Shimmer blitter** (+0x7C, flag 0x02, vtable 0x007e5780, function 0x00494330):
```c
// 75/25 blend: 3 parts source + 1 part destination
intensity = clamp((param * 261) >> 11, 0, 254);
alpha = intensity_table[intensity * 512 + a_buffer_pixel];
src = remap_palette[(alpha | pixel_value) * 2];
*dest = (src >> 2 & mask) * 3 + (*dest >> 2 & mask);
```

**50% blend blitter** (+0x78, flag 0x04, vtable `vtable_ZBuf_50pct_blend`, function 0x00497CF0):
```c
// 50/50 blend: equal parts source and destination
src = remap_palette[(a_buffer_lookup | pixel_value) * 2];
*dest = (src >> 1 & mask) + (*dest >> 1 & mask);
```

**25% blend blitter** (+0x74, flag 0x06, vtable `vtable_ZBuf_25pct_blend`, function 0x00494080):
```c
// 25/75 blend: 1 part source + 3 parts destination
src = remap_palette[(a_buffer_lookup | pixel_value) * 2];
*dest = (src >> 2 & mask) + (*dest >> 2 & mask) * 3;
```

The mask preserves 16-bit color channel boundaries (R5G6B5: 0xF7DE or R5G5B5: 0x7BDE).
All blitters apply house color remapping via the palette table before blending.

**The "shimmer" vs "semi-transparent" visual comes from blend ratio, NOT pixel displacement:**
- Early/late cloaking (state 1/4): 75/25 → unit mostly visible, slight transparency
- Mid cloaking (state 2/3): 50/50 → equal mix, classic semi-transparent look
- Allied shimmer cycle: alternates between 75/25, 50/50, and opaque per frame

### Player-Owned Cloaked Unit Shimmer (vtable+0x43C = 0x0070ED80)

For cloaked units owned by the local player (allied), an additional visual
effect is applied via `vtable+0x43C`. This function modulates the draw flags
based on a cycling timer to create a **pulsing shimmer**:

```c
uint ModifyCloakDrawFlags(TechnoClass* this, uint flags) {
    // Check timer at +0x1F4 (CDTimer: start=+0x1EC, duration=+0x1F4)
    int remaining = this->field_0x1F4;
    if (this->field_0x1EC != -1) {
        int elapsed = g_CurrentFrameCounter - this->field_0x1EC;
        if (remaining > elapsed)
            remaining = remaining - elapsed;
        else
            goto compute_phase;  // timer expired
    }
    if (remaining != 0 && !IsPlayerOwned())
        return flags;  // no shimmer for non-player units with active timer

compute_phase:
    // Compute phase from frame counter
    phase = (g_CurrentFrameCounter - this->field_0x1DC + 0x40) & 0xFF;

    // Phase ranges create a repeating cycle (verified: decompile_function 0x0070ED80):
    // 0x00-0x3F: opaque
    // 0x40-0x43: shimmer (z-read distortion, |2)  — falls through when no prior if matches
    // 0x44-0x4B: 50% blend (|4)
    // 0x4C-0x4F: shimmer (|2)  — no matching `if`, falls through to `return param_2 | 2`
    // 0x50-0x6F: opaque
    // 0x70-0x73: shimmer (|2)  — falls through to `return param_2 | 2`
    // 0x74-0x7B: 50% blend (|4)
    // 0x7C-0x7F: shimmer (|2)  — no matching `if`, falls through to `return param_2 | 2`
    // 0x80-0xFF: opaque

    if (phase < 0x40): return flags;              // opaque
    if (phase <= 0x43): return flags | 0x02;      // shimmer (explicit branch)
    if (phase < 0x4C): return flags | 0x04;       // 50% blend
    if (phase <= 0x4F): return flags | 0x02;      // shimmer (fall-through — NOT "opaque flash")
    if (phase < 0x70): return flags;              // opaque
    if (phase <= 0x73): return flags | 0x02;      // shimmer (fall-through)
    if (phase < 0x7C): return flags | 0x04;       // 50% blend
    if (phase <= 0x7F): return flags | 0x02;      // shimmer (fall-through — NOT "opaque")
    return flags;                                  // default: opaque
}
```

This creates a 256-frame (~17 second at 15fps) repeating cycle where the allied
cloaked unit alternates between invisible, shimmer, 50% transparent, and briefly
opaque — so the player can track their own cloaked units.

The cycle has FOUR shimmer sub-ranges per period (verified: decompile_function 0x0070ED80):
- Pulse 1: frames 0x40-0x43 (shimmer), 0x44-0x4B (blend)
- Pulse 1 tail: frames 0x4C-0x4F (shimmer — fall-through, previously mislabeled "opaque flash")
- Pulse 2: frames 0x70-0x73 (shimmer), 0x74-0x7B (blend)
- Pulse 2 tail: frames 0x7C-0x7F (shimmer — fall-through, previously mislabeled "opaque")
- Rest of cycle (0x00-0x3F, 0x50-0x6F, 0x80-0xFF): opaque

## BuildingClass Cloaking (Gap Generators)

BuildingClass_GetVisualState (0x004544A0) has a separate code path using
`BuildingClass+0x6ED` (BuildingCloakStage), a byte counter managed by
`BuildingClass__StartCloaking` (0x004521C0) and `BuildingClass__StopCloaking`
(0x00452210).

```c
int BuildingClass::GetVisualState(param_2, param_3) {
    byte stage = this->BuildingCloakStage;  // +0x6ED
    if (stage == 0):
        return TechnoClass::GetVisualState(param_2, param_3);  // normal path
    if (stage < 11):
        return (stage > 5) ? 2 : 1;  // 1-5→shimmer, 6-10→50% blend
    // stage >= 11: fully cloaked
    // Complex visibility checks:
    //   if param_2 && param_3: check cell visibility → return 3 (allied) or 5 (enemy)
    //   if IsDiscoveredByCurrentPlayer or IsAlliedView or spectator → return 3
    //   if (vtable+0x328 returns true) → return 3  (building-specific check)
    //   if allied in multiplayer → return 3
    //   otherwise → return 5 (invisible)
}
```

### BuildingClass__StartCloaking (0x004521C0)

Sets `+0x660` (CloakAnimDirection) = 0 and iterates 21 animation slots at +0x55C:
```c
for (int i = 0; i < 0x15; i++) {
    AnimClass* anim = this->AnimSlots[i];  // +0x55C + i*4
    if (anim != null) {
        anim->field_0x11A = 1;   // enable animation
        anim->field_0x11B = 0;   // direction forward
        anim->field_0x11C = anim->field_0xAC;  // set rate from type
        anim->field_0x119 = 1;   // activate
    }
}
```

### BuildingClass__StopCloaking (0x00452210)

Sets `+0x660` (CloakAnimDirection) = 1 and reverses all animation slots:
```c
for (int i = 0; i < 0x15; i++) {
    AnimClass* anim = this->AnimSlots[i];
    if (anim != null) {
        anim->field_0x11A = 0;   // disable forward
        anim->field_0x11B = 1;   // direction reverse
        anim->field_0x119 = 0;   // deactivate forward
    }
}
```

## Gap Generator Tick — 0x00454DB0 (UpdateGapGenerator_Tick)

The complete building cloaking state machine, called every tick for gap generators
(buildings with `CloakGenerator=yes`):

```c
void UpdateGapGenerator_Tick(BuildingClass* this) {
    if (this->BuildingType == null) return;

    if (CloakState == 1) {  // CLOAKING
        if (BuildingCloakStage < 15) {
            BuildingCloakStage++;

            // Visual state transitions trigger redraw
            if (stage == 1 || stage == 6 || stage == 11)
                MarkForRedraw();

            // If stage reaches 15 and visual is invisible, set to 16
            if (stage == 15 && GetVisualState(0,0) == 5)
                stage = 16;

            // Update all 21 animation slot cloak stages
            for (i = 0; i < 21; i++)
                if (AnimSlots[i] != null)
                    AnimSlots[i]->CloakStage = stage;  // AnimClass+0x178

            if (BuildingCloakStage == 15) {
                CloakState = 2;  // fully cloaked
                // Destroy gap shroud particle system at +0xC3 (0x30C)
            }
        }
    }
    else if (CloakState == 3) {  // UNCLOAKING
        if (BuildingCloakStage > 0)
            BuildingCloakStage--;

        // Same redraw triggers at 0, 5, 10
        // Same anim slot updates

        if (BuildingCloakStage == 0) {
            CloakState = 0;  // fully uncloaked
            // Spawn particle system for gap generator visual effect
            //   (from BuildingTypeClass+0x764, positioned at building coords)
        }
    }

    // Standard cloak state checks (same vtable calls as TechnoClass)
    if (CloakState == 2)  // fully cloaked
        if (ShouldUncloak())  // vtable+0x2A4
            StartUncloaking(0);  // vtable+0x45C

    if (CloakState == 0)  // uncloaked
        if (CanCloak())  // vtable+0x2A0
            StartCloaking(0);  // vtable+0x460

    // Gap radius management (when operational)
    if (this->GapActive && BuildingType->CloakGenerator) {
        // Apply/remove fog per cell in CloakRadiusInCells range
        // Visual buffer update for gap generator shroud effect
        // Cross-building: search all BuildingClass instances for
        //   nearby gap generators within (CloakRadiusInCells+2) radius
        //   and set their GapActive flag
    }
}
```

### BuildingCloakStage → Visual State Mapping

| Stage Range | Visual State | Effect |
|-------------|-------------|--------|
| 0 | 0 | Fully opaque (uses TechnoClass path) |
| 1-5 | 1 | Shimmer / warp |
| 6-10 | 2 | 50% alpha blend |
| 11-14 | 3 (allied) or 5 (enemy) | Semi-transparent or invisible |
| 15 | Transition → CloakState=2 | Fully cloaked |
| 16 | Special (stage 15 + invisible) | Invisible + stage override |

Building cloaking is SLOWER than unit cloaking — it takes 15 steps
(one per tick) vs units which depend on CloakingStages (default 9)
divided by CloakingSpeed.

## CloakShroud System (Gap Generator Fog)

Two functions manage the fog-of-war darkening that gap generators create around
cloaked units:

### TechnoClass__UpdateCloakShroud — 0x006FB170

Applied when a unit with CloakShroudRadius (TechnoTypeClass+0xCD2) activates.
Creates a circular region of fog around the unit:

```c
void UpdateCloakShroud(TechnoClass* this) {
    if (g_PlayerPtr == null || this->CloakShroudActive) return;
    if (!vtable_0x350())  return;  // check if shroud capable

    if (this->CloakShroudRadius == 0)
        this->CloakShroudRadius = (int)TypeClass->field_0xCD2;

    this->CloakShroudActive = 1;
    int radius = this->CloakShroudRadius;
    CellStruct center = GetCellCoords();

    // Iterate cells in square (-2-radius .. +2+radius)
    for (dy = -2-radius; dy < radius+2; dy++) {
        for (dx = -2-radius; dx < radius+2; dx++) {
            if (dx*dx + dy*dy < (radius+1)*(radius+1)) {  // circular check
                CellStruct target = center + (dx, dy);

                // For non-allied, non-spectator cells:
                CellClass* cell = Map.GetCell(target);
                if (cell->ShroudCounter != 1 && cell->ShroudCounter >= 0)
                    cell->ShroudCounter++;      // +0x130
                cell->GapCounter++;             // +0x134
                if (cell->ShroudCounter > 0) {
                    cell->Flags &= ~0x10;       // clear "revealed" bit
                    cell->Flags &= ~0x08;       // clear "explored" bit
                }

                // For allied/spectator cells:
                cell->AllyGapCounter++;         // +0x13C
            }
        }
    }
    g_PlayerPtr->field_0x240 = 0;  // reset player fog state
    RefreshFog();   // 0x00657CE0
    UpdateRadar(2); // 0x004F42F0
}
```

### TechnoClass__RemoveCloakShroud — 0x006FB470

Reverses the shroud when the gap generator is destroyed/disabled:

```c
void RemoveCloakShroud(TechnoClass* this) {
    if (g_PlayerPtr == null || !this->CloakShroudActive) return;

    // Same radius/cell iteration as UpdateCloakShroud, but:
    this->CloakShroudActive = 0;

    for each cell in radius:
        // For non-allied:
        cell->GapCounter--;           // +0x134
        if (g_PlayerPtr->HasGapGen && cell->GapCounter < 1) {
            cell->ShroudCounter--;    // +0x130
            if (cell->ShroudCounter < 1) {
                cell->Flags |= 0x08;  // restore "explored"
                cell->Flags |= 0x10;  // restore "revealed"
            }
        }
        // For allied/spectator:
        cell->AllyGapCounter--;       // +0x13C

    RefreshFog();
    UpdateRadar(2);
}
```

### CellClass Shroud Fields

| Offset | Field | Description |
|--------|-------|-------------|
| +0x12C | Flags | Bit 0x08 = explored, bit 0x10 = revealed |
| +0x130 | ShroudCounter | Number of gap generators covering this cell |
| +0x134 | GapCounter | Gap effect reference count |
| +0x13C | AllyGapCounter | Allied gap counter (for spectator/allied view) |

## CloakDelay Re-Cloak Timer

**Correction (verified: decompile_function 0x006FBDC0, decompile_function 0x006F2B40):**
`CanAutoCloak` checks TWO separate CDTimer gates, not one:

1. **Primary ReCloakDelayTimer** at TechnoClass+0x2EC (start) / +0x2F4 (duration):
   `param_1[0xBB]` / `param_1[0xBD]` in CanAutoCloak. This is the actual CloakDelay
   timer set after a unit fully uncloaks.
2. **Secondary gate timer** at TechnoClass+0x240 (start) / +0x248 (duration):
   `param_1[0x90]` / `param_1[0x92]` in CanAutoCloak. A separate CDTimer also checked
   as an eligibility gate. Both timers must be expired before re-cloaking is allowed.

After a unit fully uncloaks (CloakState 3→0), the primary timer at +0x2EC/+0x2F4 is set:

```
duration = (int)(Rules->CloakDelay * 900.0)
```

With the default `CloakDelay=0.02` (minutes), this is:
```
0.02 * 900 = 18 frames ≈ 1.2 seconds at 15fps
```

Both timers are initialized in the TechnoClass constructor with start=`g_CurrentFrameCounter`
and duration=0, meaning they expire immediately at game start (no initial delay). Implementers
must check both +0x2EC/+0x2F4 AND +0x240/+0x248 gates in `CanAutoCloak`.

## Complete Visual Progression Timeline

### Cloaking (Uncloaked → Cloaked), CloakingStages=9, CloakingSpeed=1

```
Tick 0:   StartCloaking() → CloakState=1, Progress=0, Delta=+1
Tick 1:   Progress=1 → visual=28  → State 1 → Z-read shimmer
Tick 2:   Progress=2 → visual=56  → State 1 → Z-read shimmer
Tick 3:   Progress=3 → visual=85  → State 2 → 50% alpha blend
Tick 4:   Progress=4 → visual=113 → State 2 → 50% alpha blend
Tick 5:   Progress=5 → visual=142 → State 2 → 50% alpha blend
Tick 6:   Progress=6 → visual=170 → State 2 → 50% alpha blend
Tick 7:   Progress=7 → visual=199 → State 3 → ★ Triggers CloakState=2
          CloakState=2, Progress=0. Unit is now fully cloaked.
```

Note: GetVisualState returns 3 at progress=7, which the state machine at
0x6FBA8D interprets as "cloaking complete" and immediately transitions to
CloakState=2. The full 9 stages are NOT all animated — the transition
happens as soon as visual state 3 is reached.

### Uncloaking (Cloaked → Uncloaked), CloakingStages=9, CloakingSpeed=1

```
Tick 0:   StartUncloaking() → CloakState=3, Progress=8 (=Stages-1), Delta=-1
Tick 1:   Progress=7 → visual=199 → State 3 → 50% blend
Tick 2:   Progress=6 → visual=170 → State 2 → 50% blend
Tick 3:   Progress=5 → visual=142 → State 2 → 50% blend
Tick 4:   Progress=4 → visual=113 → State 2 → 50% blend
Tick 5:   Progress=3 → visual=85  → State 2 → 50% blend
Tick 6:   Progress=2 → visual=56  → State 1 → Z-read shimmer
Tick 7:   Progress=1 → visual=28  → State 1 → Z-read shimmer
Tick 8:   Progress=0 → visual=0   → State 0 → ★ Triggers CloakState=0
          CloakState=0, fully uncloaked. ReCloakDelayTimer (+0x2EC/+0x2F4) starts.
```

### CloakingSpeed > 1 (e.g., CloakingSpeed=5 for Mirage Tank)

CloakingSpeed controls the CDTimer duration between CloakProgress increments.
With CloakingSpeed=5, each progress step takes 5 game frames instead of 1,
so the cloaking animation takes 5x as long.

## Implementation Notes for Rust Engine

1. **CloakState** is an enum {Uncloaked=0, Cloaking=1, Cloaked=2, Uncloaking=3}
2. **CloakProgress** is a plain integer counter, NOT fixed-point (sim math exception)
3. **Visual state calculation** uses float division: `(progress as f64 / stages as f64 * 256.0) as i32` — this is one of the rare places the original uses FPU math in sim-adjacent code, but it only affects rendering, not sim state
4. **Blitter selection** maps to our shader pipeline:
   - State 0 → Normal opaque render (no special shader)
   - State 1 → Z-read distortion shader (heat shimmer)
   - State 2,3 → 50% alpha blend shader
   - State 4 → Either 50% blend or shimmer depending on CloakProgress
   - State 5 → Skip draw entirely
5. **Allied cloaked unit shimmer** cycles through opaque→shimmer→blend→opaque on a 256-frame period based on `(frame_counter - start_frame + 0x40) & 0xFF`. Two shimmer pulses per cycle.
6. **Re-cloak delay** after uncloak: `(CloakDelay_minutes * 900.0) as i32` frames
7. **Building cloaking** (gap generators) uses a separate byte counter at +0x6ED with simpler 0-10+ stage mapping, not the TechnoClass state machine
8. **ShouldUncloak** logic: units with cloaking type flags stay cloaked even while active; units without flags (gap-generator-cloaked) uncloak when cell leaves shroud
9. **CanAutoCloak** is the gatekeeper: checks timers, deployed state, mission state, and type flags before allowing a unit to enter cloaking
10. **CloakShroud** (gap generators): increments per-cell fog counters in a circular radius, clears explored/revealed bits. Reversed when gap generator dies.
11. **Target re-notification** (corrected 2026-07-18, see "Target Re-Notification on Cloak Complete" — was "mind-controlled units are scattered," INFERENCE_HARDENED): when a unit finishes cloaking (CloakState 1→2), other units whose current Target (+0x2B4) is that unit have their target reference re-assigned via the Assign_Target vtable slot (vtable+0x3C8 → `Set_ArchiveTarget`). No mind-control field is read or written by this path; the real MindControlledBy pointer lives at +0x2C0.
12. **CloakStop** (TechnoTypeClass+0xC93, INI `CloakStop=`): unit can only cloak when stationary — IsCloakable checks locomotion busy state
13. **DecloakToFire** (WeaponTypeClass+0x133, INI `DecloakToFire=`): per-weapon flag requiring uncloak before firing
14. **CloakGenerator** (BuildingTypeClass+0x16C7, INI `CloakGenerator=`): marks building as gap/cloak generator
15. **CloakRadiusInCells** (BuildingTypeClass+0x1707, INI `CloakRadiusInCells=`): building-specific cloak/gap radius
16. **GapRadiusInCells** (TechnoTypeClass+0xCD2, INI `GapRadiusInCells=`): type-level gap fog radius
17. **Cell visibility** uses per-house bitmask at CellClass+0x78 (32 bits, 1 per house) and per-house gap counter array at CellClass+0x7C (short per house)
18. **VXL cloaking** uses same visual state system but different flag base (0x2000 instead of 0x0000), with brightness variants 0x200A/0x200C for near-invisible state 4
19. **Transport interaction:** Limbo does NOT reset CloakState. On Unlimbo, cloakable units set CloakState=2 directly (skip animation)
20. **Chronoshift blocks cloaking:** IsWarpingIn (+0x270) / IsWarpingOut (+0x271) flags prevent auto-cloaking during chrono sequence
21. **Disguise is independent:** A unit can be simultaneously cloaked AND disguised (Mirage Tank). No shared fields or state between the two systems

## Cloaking System Interactions

### Transport Enter/Exit

**Enter (Limbo):** CloakState (+0x220) is NOT modified by the Limbo function
(vtable+0xDC at 0x4D9720). The cloak state persists in memory while inside
the transport.

**Exit (Unlimbo):** `UnitClass::Unlimbo` at 0x737BA0, specifically at 0x737BEB:
```c
if (this->HasStealthAbility && !this->field_0x3D5) {
    this->CloakState = 2;  // fully cloaked immediately
}
```
Cloakable units exiting transports become **immediately fully cloaked** — the
cloaking animation is completely skipped. The unit reappears already invisible
to enemies.

### Chronoshift / Teleport Warping

TechnoClass warp state fields:
| Offset | Size | Field | vtable | Description |
|--------|------|-------|--------|-------------|
| +0x270 | BYTE | IsWarpingIn | +0x1D4 (0x70C5B0) | Set during chrono arrival |
| +0x271 | BYTE | IsWarpingOut | +0x1D8 (0x70C5C0) | Set during chrono departure |

**CloakingTick guard** (0x6FB783): When CloakState == 0, if either warp flag
is set, auto-cloaking is blocked. The unit stays uncloaked until warping
completes. `TeleportLocomotionClass` manages these flags:
- Departure: sets IsWarpingOut=1, calls ProcessCloakMode(2)
- Arrival: sets IsWarpingIn=1, calls ProcessCloakMode(2)

**Visual:** Draw functions add the 0x2004 flag (chrono shimmer) when either
warp flag is true. This creates the characteristic chrono visual using the
same blitter pipeline as cloaking but with the combined 0x06 (shimmer+blend) flag.

### Target Re-Notification on Cloak Complete

**(Corrected 2026-07-18 — this section was WRONG at the foundation and has been
rewritten from a fresh decompile; root cause INFERENCE_HARDENED.)**

**TechnoClass+0x2B4** in the INSTANCE is **Target** (the unit's current
attack-target / "ArchiveTarget" reference), NOT a mind-control pointer:
- Verified: `decompile_function 0x006fcdb0` (`TechnoClass::Set_ArchiveTarget`)
  writes its new-target argument to `param_1[0xad]` = +0x2B4 (0xad×4=0x2B4).
- Verified: `decompile_function 0x007105e0` (`TechnoClass::IsMindControlled`)
  reads the real mind-control fields at `param_1+0x2c0` / `+0x2c4` — i.e. the
  actual **MindControlledBy** pointer is at **+0x2C0**, not +0x2B4.
- `TechnoTypeClass+0x2B4` (EliteAbilities[CLOAK]) is unrelated, as previously noted.

`vtable+0x3C8` is **not** a "scatter" function — it is the virtual
**Assign_Target** dispatch slot:
- Verified: `read_memory` on `vtable__UnitClass` (0x007f5c70) at +0x3C8 resolves
  to 0x006fcdb0 = `TechnoClass::Set_ArchiveTarget` itself (UnitClass does not
  override the slot).
- Verified: `decompile_function 0x0051b1f0` (InfantryClass's override of the
  same slot) performs infantry-specific bookkeeping (deploy/anim/sound state,
  reading `param_1[0xad]`=+0x2B4=Target for its own early-out) and then calls
  `TechnoClass__Set_ArchiveTarget(param_2)` — i.e. it (re)assigns the caller's
  target, it does not reposition or free anything.

Re-derived sequence at cloak-complete (CloakState 1→2), verified via
`decompile_function 0x006FB740`:
1. Iterate `g_TechnoClass_Array`.
2. For each other unit where `unit->Target (+0x2B4) == this` (i.e. units
   currently targeting the unit that just finished cloaking) — subject to an
   additional sensor/owner/random-sample filter not yet fully decoded — collect it.
3. Call `Limbo(false)` on the newly-cloaked unit (vtable+0xDC).
4. For each collected unit: call `vtable+0x3C8` (Assign_Target) passing the
   newly-cloaked unit as the argument — i.e. each collected unit's target
   reference is (re-)assigned to the now-cloaked unit via the same
   `Set_ArchiveTarget` path a normal target assignment would use.

**Key (revised):** This mechanism has nothing to do with mind control. It
re-notifies/re-resolves the target reference of units that were already
targeting the entity that just finished cloaking. The previous "gather
mind-controlled units and scatter them" narrative, and the offset/function
identifications it depended on, are UNVERIFIABLE as written and have been
replaced above with what the binary actually shows. The exact gameplay-level
purpose of the filter in step 2 and the downstream effect of the
Assign_Target call are NOT fully traced this session — flagged for follow-up,
not invented.

### Disguise Independence

Disguise is a **completely separate system** from cloaking. They share no
fields, state, or logic:

| Field | Offset | INI Key |
|-------|--------|---------|
| CanDisguise | TechnoTypeClass+0xD2F | `Disguise=` |
| PermaDisguise | TechnoTypeClass+0xD30 | `PermaDisguise=` |
| DisguiseWhenStill | TechnoTypeClass+0xD31 | `DisguiseWhenStill=` |

- CloakingTick never reads or writes any disguise field
- A unit CAN be simultaneously cloaked AND disguised (e.g., Mirage Tank)
- Disguise controls WHAT the unit appears as; cloaking controls WHETHER it is visible
- The vtable+0xC4 check in DrawSHP gates `ModifyCloakDrawFlags` using a separate
  byte flag at TechnoClass+0x1D8 (unrelated to CloakState)

## Sensor Detection vs Cloaking

Three separate detection systems interact with cloaking. Full details in
[SENSOR_CLOAK_DETECTION.md](SENSOR_CLOAK_DETECTION.md).

### 1. Sensor Sight (any TechnoClass with SensorsSight > 0)

| Item | Details |
|------|---------|
| Range field | TechnoTypeClass+0x5F0 (int, `SensorsSight=`) |
| Cell counter | CellClass+0x7C + houseIndex*2 (short array, per-house) |
| Add function | `TechnoClass::AddSensorsAt` (0x004DE7B0, vtable+0x4E8) |
| Remove function | `TechnoClass::RemoveSensorsAt` (0x004DE940, vtable+0x4EC) |
| Effect | Increments per-house sensor count on cells; calls `DoUncloak` on cloaked units in range |

### 2. Sensor Array Buildings (BuildingTypeClass+0x16C8 = true)

| Item | Details |
|------|---------|
| Range | Uses TechnoTypeClass::SensorsSight (+0x5F0) for add, CloakRadiusInCells (+0x1707) for remove |
| Add function | `BuildingClass::AddSensorArrayAt` (0x00455820, vtable+0x4F4) — includes power check |
| Remove function | `BuildingClass::RemoveSensorArrayAt` (0x004556D0, vtable+0x4F8) |

### 3. Disguise Detection (TechnoTypeClass+0xD31 = true)

| Item | Details |
|------|---------|
| Bool field | TechnoTypeClass+0xD31 (`DetectDisguise=`) |
| Range field | TechnoTypeClass+0x5F4 (int, `DetectDisguiseRange=`) |
| Cell counter | CellClass+0xAC + houseIndex*2 (**SEPARATE** short array from sensors) |
| Add function | `BuildingClass::AddDetectDisguiseAt` (0x00455A80, vtable+0x4FC) |
| Remove function | `BuildingClass::RemoveDetectDisguiseAt` (0x00455980, vtable+0x500) |
| Effect | Only increments disguise detect counter — does NOT call DoUncloak |

### Key: No "DetectCloak" INI Key

There is no `DetectCloaked` or `DetectCloak` INI key in the binary. Cloaked
unit detection is handled entirely by the `SensorsSight` and `SensorArray`
systems. Any unit or building with `SensorsSight > 0` will reveal cloaked
units within range by calling `DoUncloak`.
