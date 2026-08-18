# Superweapon System — Consolidated Ghidra Research Report

**Primary Addresses:**
- `SuperWeaponTypeClass::Constructor` — `0x006CE5B0`
- `SuperWeaponTypeClass::ReadINI` — `0x006CEA20`
- `SuperClass::Constructor` — `0x006CAF90`
- `SuperClass::Activate` — `0x006CB560`
- `SuperClass::Suspend` — `0x006CB4D0`
- `SuperClass::Deactivate` — `0x006CB7B0`
- `SuperClass::AI_Charging` — `0x006CC080`
- `SuperClass::AI_Ready` — `0x006CBCA0`
- `SuperClass::AnimStage` — `0x006CBEE0`
- `SuperClass::NameReadiness` — `0x006CC2B0`
- `SuperClass::GetRechargeTime` — `0x006CC260`
- `SuperClass::Launch` — `0x006CC390`
- `LightningStorm::Start` — `0x00539EB0`
- `LightningStorm::Process` — `0x0053A6C0`
- `LightningStorm::CreateCloudBolt` — `0x0053A140`
- `LightningStorm::GroundStrike` — `0x0053A300`
- `MapClass::RevealAroundCell` — `0x005678E0`
- `SuperWeaponEffects::UpdateLighting` — `0x0053C280`
- `AI::SuperLaunchCheck_SingleSW` — `0x006EFC70`
- `AI::SuperLaunchCheck_DualSW` — `0x006EFE60`
- `HouseClass::SpyPowerSabotage` — `0x0050BC90`

**Confidence:** HIGH (all functions decompiled from binary)
**Active in YR:** Yes — core gameplay system, all 12 types active

---

## 1. Overview

The superweapon system consists of two classes:
- **SuperWeaponTypeClass** — static type definitions loaded from INI (one per `[SWType]` section)
- **SuperClass** — runtime instances (one per SuperWeaponType per HouseClass)

Each HouseClass creates a SuperClass array at construction time. Buildings with `SuperWeapon=` or
`SuperWeapon2=` keys grant/enable superweapons on their owning house when completed. Superweapons
charge over time, can be suspended by low power, and dispatch type-specific effects when launched.

**Inheritance:**
- SuperWeaponTypeClass: AbstractClass → AbstractTypeClass → SuperWeaponTypeClass
  (NOT ObjectTypeClass — inherits directly from AbstractTypeClass)
- SuperClass: AbstractClass → SuperClass

---

## 2. SuperWeaponTypeClass Layout

**Class size:** 0x100 (256 bytes), confirmed from RTTI at `0x006CE900`.
**Constructor:** `0x006CE5B0` — `param_1` is `undefined4 *` (pointer-indexed: multiply by 4).
**ReadINI:** `0x006CEA20` — `param_1` is `int` (direct byte offsets).

### Inherited from AbstractTypeClass (0x00–0x9B)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0x00 | 16 | ptrs | vtable pointers (×4) | Primary + 3 secondary |
| 0x10 | 4 | int | UniqueID | -1 = unassigned |
| 0x14 | 1 | byte | AbstractFlags | Masked with 0xF8 in ctor |
| 0x24 | 25 | char[25] | ID | INI section name |
| 0x3D | 32 | char[32] | UINameLabel | CSF string key |
| 0x60 | 4 | wchar_t* | UIName | Resolved localized string |
| 0x64 | 49 | char[49] | Name | Display name from INI |
| 0x98 | 4 | int | ArrayIndex | Index in global type array |

### SuperWeaponTypeClass-Specific (0x9C–0xFF)

All fields verified from ReadINI at `0x006CEA20`.

| Offset | Size | Type | Field | Default | INI Key | Notes |
|--------|------|------|-------|---------|---------|-------|
| 0x9C | 4 | ptr | WeaponType | NULL | `WeaponType=` | WeaponTypeClass* (e.g. NukeCarrier) |
| 0xA0 | 4 | int | Unknown_A0 | -1 | (none) | Vestigial, not read from INI |
| 0xA4 | 4 | int | Unknown_A4 | -1 | (none) | Vestigial, not read from INI |
| 0xA8 | 4 | int | Unknown_A8 | -1 | (none) | Vestigial, not read from INI |
| 0xAC | 4 | int | Unknown_AC | -1 | (none) | Vestigial, not read from INI |
| 0xB0 | 4 | int | RechargeTime | 4500 | `RechargeTime=` | Frames. INI value in minutes × 900.0f |
| 0xB4 | 4 | int | Type | -1 | `Type=` | Enum index 0–11, see §3 |
| 0xB8 | 4 | ptr | SidebarImageSHP | NULL | (loaded) | SHPStruct* loaded from MIX |
| 0xBC | 4 | int | Action | 0 | `Action=` | Cursor action enum (table at 0x7E4C50) | (corrected 2026-05-29: was -1; constructor sets param_1[0x2f]=0 via decompile_function 0x006CE5B0 — STRUCT_FAMILY_CASCADE) |
| 0xC0 | 4 | int | SpecialSound | -1 | `SpecialSound=` | VocClass index |
| 0xC4 | 4 | int | StartSound | -1 | `StartSound=` | VocClass index |
| 0xC8 | 4 | ptr | AuxBuilding | NULL | `AuxBuilding=` | BuildingTypeClass* |
| 0xCC | 25 | char[25] | SidebarImageName | "" | `SidebarImage=` | Filename for cameo SHP |
| 0xE5 | 1 | bool | UseChargeDrain | false | `UseChargeDrain=` | Enables charge-drain cycle |
| 0xE6 | 1 | bool | IsPowered | true | `IsPowered=` | Charging pauses on low power |
| 0xE7 | 1 | bool | DisableableFromShell | false | `DisableableFromShell=` | Can be disabled in lobby |
| 0xE8 | 4 | int | FlashSidebarTabFrames | -1 | `FlashSidebarTabFrames=` | Flash duration | (corrected 2026-05-29: was 0; constructor sets param_1[0x3a]=0xffffffff via decompile_function 0x006CE5B0 — STRUCT_FAMILY_CASCADE) |
| 0xEC | 1 | bool | AIDefendAgainst | false | `AIDefendAgainst=` | AI target priority |
| 0xED | 1 | bool | PreClick | false | `PreClick=` | Requires source click first (ChronoSphere) |
| 0xEE | 1 | bool | PostClick | false | `PostClick=` | Requires destination click (ChronoWarp) |
| 0xF0 | 4 | int | PreDependent | -1 | `PreDependent=` | Type enum of prerequisite SW |
| 0xF4 | 1 | bool | ShowTimer | false | `ShowTimer=` | Display charge timer on sidebar |
| 0xF5 | 1 | bool | ManualControl | false | `ManualControl=` | Don't auto-resume on power restore |
| 0xF8 | 4 | float | Range | 0.0 | `Range=` | Targeting range in cells |
| 0xFC | 4 | int | LineMultiplier | 0 | `LineMultiplier=` | Cursor line drawing multiplier |

**RechargeTime conversion:** ReadINI reads `RechargeTime=` as a double (minutes), multiplies
by 900.0f (60 sec × 15 fps), then `Math::ftol()` to int. The FPU multiply is hidden by Ghidra
decompilation but confirmed by default value: 4500 = 5 minutes × 900.

---

## 3. Type Enum

String table at `0x008425C0` (12 entries ending at `0x008425F0`):

| Index | String | Superweapon | Active in YR |
|-------|--------|-------------|--------------|
| 0 | `MultiMissile` | Nuclear Missile | Yes |
| 1 | `IronCurtain` | Iron Curtain | Yes |
| 2 | `LightningStorm` | Lightning Storm | Yes |
| 3 | `ChronoSphere` | Chronosphere (source select) | Yes |
| 4 | `ChronoWarp` | Chrono Warp (destination select) | Yes |
| 5 | `ParaDrop` | Paradrop (side-dependent) | Yes |
| 6 | `AmerParaDrop` | American Paradrop | Yes |
| 7 | `PsychicDominator` | Psychic Dominator | Yes (YR only) |
| 8 | `SpyPlane` | Spy Plane Flyby | Yes |
| 9 | `GeneticConverter` | Genetic Mutator | Yes (YR only) |
| 10 | `ForceShield` | Force Shield | Yes (YR only) |
| 11 | `PsychicReveal` | Psychic Reveal | Yes (YR only) |

---

## 4. SuperClass Layout

**Constructor:** `0x006CAF90` (2-param). `param_1` is `undefined4 *` (pointer-indexed).

| Byte Offset | Size | Type | Field | Init Value | Notes |
|-------------|------|------|-------|------------|-------|
| 0x00–0x23 | 36 | — | AbstractClass base | — | 4 vtable ptrs, UniqueID, flags |
| 0x24 | 4 | int | CustomRechargeTime | -1 | -1 = use type default |
| 0x28 | 4 | ptr | Type | param_2 | SuperWeaponTypeClass* |
| 0x2C | 4 | ptr | Owner | param_3 | HouseClass* |
| 0x30 | 4 | int | Timer.StartFrame | CurrentFrame | -1 = timer stopped |
| 0x34 | 4 | int | Timer.Reserved | (unset) | Padding, not actively used |
| 0x38 | 4 | int | Timer.Duration | 0 | Total duration in frames |
| 0x3C | 4 | ptr | UIName | (set in Activate) | Copied from Type+0x60 |
| 0x40 | 1 | byte | field_40 | 0 | Cleared on Activate; purpose unknown |
| 0x44–0x47 | 4 | — | (unset) | — | Not referenced in key functions |
| 0x48 | 4 | int | field_48 | 0 | Not referenced in key functions |
| 0x4C | 4 | int | field_4C | 0 | Not referenced in key functions |
| 0x50 | 4 | int | SoundCountdown | -1 | Decremented per tick; plays SpecialSound at 0 |
| 0x54–0x5C | 12 | CoordStruct | EffectCoords | (set on fire) | X, Y, Z for ForceShield effect |
| 0x60 | 1 | bool | AllowSuspension | true | Gates suspend/unsuspend; always true in YR |
| 0x62 | 2+2 | CellStruct | ChronoTarget | default cell | ChronoSphere source cell (X,Y packed) |
| 0x64–0x67 | — | — | (gap) | — | — |
| 0x68 | 4 | ptr | Anim | NULL | AnimClass* for visual effect |
| 0x6C | 1 | bool | field_6C | false | Secondary anim tracking flag |
| 0x6D | 1 | bool | IsActive | false | SW has been granted to house |
| 0x6E | 1 | bool | IsCharged | false | One-shot charged (UseChargeDrain mode) |
| 0x6F | 1 | bool | IsReady | false | Ready to fire |
| 0x70 | 1 | bool | IsSuspended | false | Currently suspended (low power) |
| 0x74 | 4 | int | LastAnimStage | -100 | Previous anim stage for change detection |
| 0x78 | 4 | int | ReadyFrame | -1 | Frame when became ready |
| 0x7C | 4 | int | ChargeDrainState | -1 | UseChargeDrain: 0=empty, 1=charged, 2=draining |

**Correction from existing docs:** Field 0x60 was labeled "IsSuspendedByPlayer" but analysis
shows it gates whether suspension changes can occur. It's always initialized to true and never
set to false in any decompiled function. Renamed to "AllowSuspension."

---

## 5. Lifecycle State Machine

### 5.1 Activation (SuperClass::Activate — 0x006CB560)

Called when a building with `SuperWeapon=` completes construction.

```
if already active: return 0 (no-op)

set IsActive = true
set IsCharged = param_2  (can activate as already charged)
set field_40 = 0
store UIName from Type+0x60 → +0x3C

if NOT charged AND was suspended AND AllowSuspension:
    if ManualControl == false:
        start timer if not running (set StartFrame = CurrentFrame)
    else:
        save remaining time, stop timer (StartFrame = -1)
    clear IsSuspended

if ShowTimer: add to sidebar ShowTimer tracking array

if ManualControl == false:
    clean up existing anims at +0x68 and +0x6C
    if active AND NOT ready AND (NOT suspended OR PreClick):
        start recharge timer via CDTimerClass
        if UseChargeDrain: set ChargeDrainState = 0 (empty)
```

### 5.2 Suspend (SuperClass::Suspend — 0x006CB4D0)

Called when building loses power (param_2=1) or regains power (param_2=0).

```
if NOT active OR IsCharged: return 0
if param_2 == current IsSuspended: return 0 (no change)
if AllowSuspension == false: return 0

if unsuspending (param_2=0) AND ManualControl == false:
    start timer (set StartFrame = CurrentFrame if -1)
else (suspending):
    save remaining time to Duration
    stop timer (set StartFrame = -1)

IsSuspended = param_2
return 1
```

### 5.3 Deactivate (SuperClass::Deactivate — 0x006CB7B0)

Called when the SW-granting building is destroyed/sold.

```
if NOT active: return 0
set IsReady = false
set IsActive = false
remove from ShowTimer tracking array
return 1
```

### 5.4 Charging Tick (SuperClass::AI_Charging — 0x006CC080)

Called every tick while charging.

```
if NOT active: return

IsReady = true  // UNCONDITIONALLY set — corrected 2026-05-29: binary sets *(param_1+0x6f)=1
                // before the elapsed check; the "if elapsed==rechargeTime" branch is a
                // redundant second set. AI_Charging always marks the SW ready when called.
                // (via decompile_function 0x006CC080 — OPERATOR_OR_ORDER_DRIFT)

rechargeTime = (CustomRechargeTime != -1) ? CustomRechargeTime : Type->RechargeTime
elapsed = Math::ftol(...)  // current charge progress

// Update timer
StartFrame = CurrentFrame
Duration = rechargeTime - elapsed

if param_2 (play EVA):
    switch(Type):
        cases 0-3, 5-11: play EVA voice
        case 4: skip (ChronoWarp has no EVA)

LastAnimStage = CurrentFrame  // track for sidebar updates

if UseChargeDrain:
    ChargeDrainState = 1  // charging
```

### 5.5 Ready Tick (SuperClass::AI_Ready — 0x006CBCA0)

Called every tick while ready.

```
if SoundCountdown > 0:
    SoundCountdown--
if SoundCountdown == 0:
    SoundCountdown = -1
    play SpecialSound at 0  // fade warning sound

if cursor state != ChronoSphere (4) AND has anim:
    mark anim for deletion

if NOT active: return 0
if IsReady AND NOT UseChargeDrain: return 0
if IsSuspended: return 0

if timer not started (StartFrame == -1):
    if ReadyFrame != -1:
        clear ReadyFrame
        return -0xFF (special signal)
    return 0

remaining = Duration - (CurrentFrame - StartFrame)
if remaining > 0:
    // Still charging, check anim stage changes
    stage = AnimStage()
    if stage != LastAnimStage:
        LastAnimStage = stage
        return 1 (changed)
    return 0

// Timer complete!
if UseChargeDrain:
    if ChargeDrainState == 2 (draining):
        ChargeDrainState = 0  // restart charge cycle
        restart timer with full RechargeTime
    else:
        ChargeDrainState = 1  // charged
        IsReady = true
    return 1
else:
    IsReady = true
    play EVA based on Type (same switch as AI_Charging)
    LastAnimStage = CurrentFrame
    return 1
```

### 5.6 AnimStage (SuperClass::AnimStage — 0x006CBEE0)

Returns sidebar cameo animation frame (0–54).

```
if NOT active: return 0
if NOT UseChargeDrain AND IsReady: return 54 (0x36) — fully charged
percentage = Math::ftol(charge_fraction * 53)
if percentage > 53: return 53 (0x35) — capped during charge
return percentage
```

Total range: 0 (empty) → 53 (almost ready) → 54 (ready to fire).

### 5.7 NameReadiness (SuperClass::NameReadiness — 0x006CC2B0)

Returns localized status text for UI.

```
if IsSuspended: return "Offline" (CSF 0x3B6)
if NOT UseChargeDrain:
    if IsReady: return "Ready" (CSF 0x3B0)
else:
    switch ChargeDrainState:
        0 (empty): return "Charging" (CSF 0x397)
        1 (charged): return "Ready" (CSF 0x39A)
        2 (draining): return "Active" (CSF 0x39D)
return NULL
```

### 5.8 GetRechargeTime (SuperClass::GetRechargeTime — 0x006CC260)

```
if CustomRechargeTime != -1: return CustomRechargeTime
return Type->RechargeTime
```

---

## 6. Launch Dispatch (SuperClass::Launch — 0x006CC390)

958-line function with a 12-case switch on `Type->Type` (+0xB4).
All cases check `IsReady` (+0x6F) before proceeding.
After firing, most cases set `DAT_008809a0 = -1` (clear cursor) and call
`VoxClass::RemoveFromQueues()` to stop any EVA voice.

### Case 0: Nuclear Missile (MultiMissile)

**Two paths based on IsCharged (+0x6E) AND IsActive (+0x6D):**

**Path A — Direct fire (both true):**
1. Convert target cell to lepton coords (cell × 256 + 128)
2. Get ground height + bridge offset if on bridge
3. Look up `NukeCarrier` weapon type
4. Create BulletClass for upward missile:
   - Source: target ground position
   - Target: target position + 20000 Z (detonation altitude)
   - Projectile from NukeCarrier weapon
5. Calculate velocity via Sin/Cos lookup (spherical angle ~π/2)
6. Fire bullet via `BulletClass::vtable+0x1F0`
7. Play EVA voice + launch sound

**Path B — Silo door path (IsCharged false):**
1. Find AuxBuilding (NukeSilo) that has matching SuperWeapon= type
2. Open silo doors: `Building::vtable+0x1E8` and `+0x1EC`
3. Store target cell at HouseClass+0x5784
4. Store SW type at Building+0x5F8
5. Play sound + EVA

Sets HouseClass+0x1FC = 1 (nuke in flight flag).

### Case 1: Iron Curtain

1. Get center cell coords + bridge offset
2. Create animation at Rules+0x348 (ChronoBeam anim? IC anim)
3. Play EVA + create radar event
4. Iterate 3×3 cell grid (9 cells at `0x00B0C038`):
   - For each cell, get occupier linked list (bridge-aware: +0xE4 normal, +0xE8 bridge)
   - For each occupier: skip if limbo bit set or unit flag at +0x27C
   - Apply invulnerability: `TechnoClass::IronCurtain(vtable+0x154)` with Rules+0xFE8 duration

### Case 2: Lightning Storm

1. Call `LightningStorm::Start` (0x00539EB0) — see §7
2. Play EVA voice
3. Clear cursor

**Simplest dispatch — all logic delegated to the LS state machine.**

### Case 3: ChronoSphere (Source Select)

1. Store target cell at SuperClass+0x62 (ChronoTarget)
2. Get cell center coords
3. Call FUN_006CB3A0 (setup anim at source)
4. Set cursor state to 4 (waiting for destination click)
5. If existing anim at +0x68: mark for deletion

### Case 4: ChronoWarp (Destination)

1. Create radar events at both source and destination
2. Create departure anim: Rules+0x32C (ChronoBlast) at source coords
3. Create arrival anim: Rules+0x328 (ChronoBlastDest) at dest coords
4. Iterate 3×3 cell grid around source cell:
   - For each occupier: validate for warp eligibility:
     - NOT in limbo
     - NOT IronCurtained (vtable+0x160)
     - NOT in tunnel (flag at +0x27C)
     - NOT already warping (vtable+0x1D4 and vtable+0x1D8)
     - NOT sitting on a Chronoshiftable building
   - If in a Warp locomotor class building: detach warp first
   - Detach from existing locomotor
   - Create TeleportLocomotionClass instance
   - Calculate destination cell (source offset → dest offset)
   - Set unit as chronoshifted (+0x6B6 = 1)
   - Assign destination via locomotor interface
   - Set chrono lock timer from owner house
5. Play EVA voice

### Case 5: ParaDrop (Side-Dependent)

1. Get cell, check for bridge → find passable cell if needed
2. Check house Side (HouseClass+0x1E8):
   - Side 0 (Allied): AllyParaDropInf (Rules+0xC40, count +0xC4C)
   - Side 1 (Soviet, default): SovParaDropInf (Rules+0xC78, count +0xC84)
   - Side 2 (Yuri): YuriParaDropInf (Rules+0xCB0, count +0xCBC)
3. For each infantry type: call FUN_0065E660 (spawn PDPLANE cargo aircraft)

### Case 6: American ParaDrop

Same as Case 5 but always uses AmerParaDropInf (Rules+0xC08, count +0xC14).

### Case 7: Psychic Dominator

1. Create radar event
2. Call `PsychicDominator::Start` (0x0053AE50) — sets global state, begins 5-phase machine
3. Play EVA voice + sound at target coords

### Case 8: Spy Plane

1. Uses AllyParaDropInf count for iteration
2. For each: call FUN_0065EAB0 (spawn SPYP aircraft for recon flyover)
3. Share exit path with Cases 5/6

### Case 9: Genetic Mutator

1. Get center cell coords + bridge offset
2. Create IonBlast anim (Rules+0x298) at target
3. Play EVA + sound + radar event
4. **Two mutation paths based on `MutateExplosion` (Rules+0x17C8):**
   - **false (3×3 grid):** iterate 9 cells, for each infantry (WhatAmI == 0xF),
     apply MutateWarhead (Rules+0xF98) — kills infantry, death anim spawns brute
   - **true (area damage, YR default):** call `Apply_area_damage()` with
     MutateExplosionWarhead (radius from warhead's CellSpread)

Mutation is indirect: warhead kills infantry → death anim InfDeath=9 plays GENDEATH →
GENDEATH has MakeInfantry=0 → AnimClass::AI spawns AnimToInfantry[0] = BRUTE at anim location.

### Case 10: ForceShield

1. Get center cell coords + bridge offset
2. Create ForceShield anim (Rules+0x34C) at target
3. Set SoundCountdown = ForceShieldDuration - ForceShieldPlayFadeSoundTime
   (SuperClass+0x50 = Rules+0x17BC - Rules+0x17C4)
4. Store effect coords at SuperClass+0x54/58/5C
5. Play StartSound
6. **Trigger power blackout:** `HouseClass::SpyPowerSabotage(Rules+0x17C0 duration)`
   - Sets HouseClass+0x2A4 (blackout start frame), +0x2AC (blackout duration)
   - While active: HouseClass::AI_AssessPower zeros PowerOutput
7. **Apply invulnerability to allied buildings in radius:**
   - Iterate ALL buildings
   - Check if allied with firing house
   - Calculate 3D distance to target
   - If distance < ForceShieldRadius (Rules+0x17B8) × 256 leptons:
     - Apply `TechnoClass::IronCurtain` with ForceShieldDuration (Rules+0x17BC)

**Key difference from Iron Curtain:** ForceShield uses radius-based targeting (not 3×3),
applies only to buildings, triggers power blackout, and uses separate color tint.

### Case 11: Psychic Reveal

1. Get target cell center coords
2. Call `MapClass::RevealAroundCell` (0x005678E0) **twice** with:
   - Radius = Rules+0xFEC (PsychicRevealRadius, default 15 cells)
   - Owner = SuperClass->Owner
   - All optional flags = 0 (shroud reveal mode)
3. Play sound at target coords

**Double call:** The function is called twice with identical parameters. This may be
to handle both normal cells and bridge cells, or a safety measure to ensure complete
reveal. Both calls use param_7=0 (shroud reveal, not fog update).

---

## 7. Lightning Storm System

**→ See [LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md](LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md)**

Full system documented: two-phase bolt lifecycle (cloud bolt → ground strike at half anim
duration), Start/Process state machine, global variables, all INI keys identified
(`LightningHitDelay`, `LightningScatterDelay`, `LightningCellSpread`, `LightningSeparation`,
`LightningWarhead`, `WeatherConClouds`, `WeatherConBolts`, `LightningSounds`), queuing
behavior, ambient lighting transitions.

---

## 8. Psychic Reveal System

**→ See [PSYCHIC_REVEAL_SUPERWEAPON_GHIDRA_REPORT.md](PSYCHIC_REVEAL_SUPERWEAPON_GHIDRA_REPORT.md)**

Delegates to general-purpose `MapClass::RevealAroundCell` (0x005678E0) with
`PsychicRevealRadius` (Rules+0xFEC, default 15 cells). Reveals shroud in circular pattern
with ownership checks. Called twice with identical params in Launch case 11.

---

## 9. Shared Lighting System (SuperWeaponEffects::UpdateLighting — 0x0053C280)

Manages ambient lighting for all three weather events. Priority order:

```
if PD_State == 1 (PD flash) OR NukeFlash active:
    ambient = Flash ambient (ScenarioClass+0x3560)
    intensity = FlashIntensity (ScenarioClass+0x356C) * 1000 / 100

else if LS_Active:
    ambient = LS ambient (ScenarioClass+0x3548)
    intensity = LS_Intensity (ScenarioClass+0x3554) * 1000 / 100

else if PD_State == 0 or 5 (PD inactive/done):
    ambient = Normal ambient (ScenarioClass+0x3528)
    call FUN_0053AD00(-1, 0)  // restore normal
    return

else (PD active, states 2-4):
    ambient = PD ambient (ScenarioClass+0x357C)
    intensity = PD_Intensity (ScenarioClass+0x3588) * 1000 / 100

ScenarioClass+0x3530 = current ambient
call FUN_0053AD00(intensity, 1)
```

Lighting transition offsets (from ScenarioClass at DAT_00a8b230):
- +0x3528: Normal ambient
- +0x3530: Current ambient (actively updated)
- +0x3548: Lightning Storm ambient
- +0x3554: Lightning Storm intensity
- +0x3560: Flash ambient (Nuke/PD flash)
- +0x356C: Flash intensity
- +0x357C: Psychic Dominator ambient
- +0x3588: Psychic Dominator intensity

---

## 10. AI Superweapon Launch Logic

### 10.1 SingleSW Check (AI::SuperLaunchCheck_SingleSW — 0x006EFC70)

Used by AI for **Iron Curtain** (Type == 1).

```
// Find highest-value target building from team
for each team member:
    value = TypeClass+0x5FC (target value)
    if valid target AND (has target flag +0x689 OR WhatAmI() == 2):
        track highest value

// Find house's Iron Curtain SW (Type == 1)
for each SW in house:
    if Type == 1: found

if SW IsReady AND PowerRatio >= 1.0:
    fire at best target coords
    return

// Not ready — check if close enough to ready
remaining = timer remaining
total = GetRechargeTime()
// corrected 2026-05-29: condition direction and operator were wrong in prior doc
// Binary (SingleSW 0x006EFC70): proceed=true when (1.0-Rules+0xD70) < remaining/total
// i.e. when a HIGH fraction of time remains (far from ready), proceed with other tasks.
// When a LOW fraction remains (almost charged), do NOT set proceed (wait for SW).
if (1.0 - Rules+0xD70) < remaining / total:
    // Proceed with other AI tasks (SW is far from ready)
else:
    // Wait — SW is close to charged, hold for fire
// (corrected via decompile_function 0x006EFC70, 0x006EFE60 — OPERATOR_OR_ORDER_DRIFT)
```

### 10.2 DualSW Check (AI::SuperLaunchCheck_DualSW — 0x006EFE60)

Used by AI for **Chronosphere** (Types 3 + 4).

```
// Same target selection as SingleSW

// Find BOTH ChronoSphere (Type 3) AND ChronoWarp (Type 4)
for each SW in house:
    if Type == 3: chronoSW = this
    if Type == 4: warpSW = this

if BOTH found AND chronoSW IsReady AND PowerRatio >= 1.0:
    find best enemy building via TeamClass::Find_Best_Target_Building
    fire ChronoSphere at team position (source)
    fire ChronoWarp at target building (destination)
    set team convoy target
    return

// Same charge-threshold check as SingleSW (uses <= operator in DualSW):
// remaining/total <= (1.0 - Rules+0xD70) → wait; else → proceed
// corrected 2026-05-29 via decompile_function 0x006EFE60 — OPERATOR_OR_ORDER_DRIFT
```

Rules+0xD70: AI superweapon launch threshold (float). Controls how close to full charge
the AI will wait before considering other tasks. When `remaining/total` exceeds
`(1.0 - threshold)`, the AI proceeds; when at or below it, the AI waits for the SW.

---

## 11. INI Configuration Reference

### SuperWeapon Definitions (from rulesmd.ini)

| Section | Type | RechargeTime | IsPowered | DisableableFromShell | Range | Building |
|---------|------|-------------|-----------|---------------------|-------|----------|
| NukeSpecial | MultiMissile | 10 min | yes | yes | 7 | NAMISL |
| IronCurtainSpecial | IronCurtain | 5 min | yes | yes | 1.4 | NAIRON |
| LightningStormSpecial | LightningStorm | 10 min | yes | yes | 7 | GAWEAT |
| ChronoSphereSpecial | ChronoSphere | 7 min | yes | yes | 1.4 | GACSPH |
| ChronoWarpSpecial | ChronoWarp | 1 min | no | no | 1.4 | GADUMY |
| ParaDropSpecial | ParaDrop | 4 min | no | no | — | CAAIRP |
| AmericanParaDropSpecial | AmerParaDrop | 4 min | no | no | — | AMRADR |
| PsychicDominatorSpecial | PsychicDominator | 10 min | yes | yes | 1.4 | YAPPET |
| SpyPlaneSpecial | SpyPlane | 4 min | no | no | — | NARADR |
| GeneticConverterSpecial | GeneticConverter | 5 min | yes | yes | 5 | YAGNTC |
| ForceShieldSpecial | ForceShield | 5 min | yes | yes | 3.4 | GATECH/NATECH/YATECH |
| PsychicRevealSpecial | PsychicReveal | 4 min | no | no | — | NAPSIS |

### Key [General] / [CombatDamage] / [AudioVisual] Rules Offsets

| Offset | INI Key | Type | Default | Section |
|--------|---------|------|---------|---------|
| 0x0298 | IonBlast | AnimType* | — | General |
| 0x02C0 | WeatherConClouds | AnimList | WCCLOUD1-3 | General |
| 0x02DC | WeatherConBolts | AnimList | WCLBOLT1-3 | General |
| 0x0744 | LightningSounds | VocList | WeatherStrike | General |
| 0x0BEC | ChronoDelay | int | 60 | General |
| 0x0BF0 | ChronoReinfDelay | int | 200 | General |
| 0x0BF4 | ChronoDistanceFactor | int | 32 | General |
| 0x0BF8 | ChronoTrigger | bool | true | General |
| 0x0BFC | ChronoMinimumDelay | int | 0 | General |
| 0x0C00 | ChronoRangeMinimum | int | 0 | General |
| 0x0D70 | (AI SW threshold) | float | — | General |
| 0x0F98 | MutateWarhead | WarheadType* | Mutate | General |
| 0x0F9C | MutateExplosionWarhead | WarheadType* | MutateExplosion | General |
| 0x0FE8 | IronCurtainDuration | int | 750 | CombatDamage |
| 0x0FEC | PsychicRevealRadius | int | 15 | General |
| 0x0348 | IronCurtainInvokeAnim | AnimType* | IRONBLST | General |
| 0x034C | ForceShieldInvokeAnim | AnimType* | FORCSHLD | General |
| 0x1794 | LightningDeferment | int | 250 | General |
| 0x1798 | LightningDamage | int | 250 | General |
| 0x179C | LightningStormDuration | int | 180 | General |
| 0x17A0 | LightningHitDelay | int | 10 | General |
| 0x17A4 | LightningScatterDelay | int | 5 | General |
| 0x17A8 | LightningCellSpread | int | 10 | General |
| 0x17AC | LightningSeparation | int | 3 | General |
| 0x17B0 | LightningPrintText | bool | false | General |
| 0x17B4 | LightningWarhead | WarheadType* | IonWH | General |
| 0x17B8 | ForceShieldRadius | int | 4 | General |
| 0x17BC | ForceShieldDuration | int | 500 | General |
| 0x17C0 | ForceShieldBlackoutDuration | int | 1000 | General |
| 0x17C4 | ForceShieldPlayFadeSoundTime | int | 75 | General |
| 0x17C8 | MutateExplosion | bool | true | General |
| 0x17E7 | (ShareReveal) | bool | — | General |
| 0x17EE | (RevealByHeight) | bool | — | General |

---

## 12. Global Arrays

| Address | Type | Description |
|---------|------|-------------|
| 0x00A8E328 | DynamicVectorClass | SuperWeaponTypeClass array |
| 0x00A83CB8 | DynamicVectorClass | SuperClass instances (all houses) |
| 0x00A83D50 | DynamicVectorClass | ShowTimer tracking array |
| 0x008425C0 | ptr[12] | SuperWeaponType enum string table |
| 0x007E4C50 | ptr[73] | Action enum string table |
| 0x00B0C038 | CellStruct[9] | 3×3 cell offset grid |
| 0x007ED3D0 | int[11] | CellSpread table (cumulative cell counts) |
| 0x00ABD490 | CellStruct[] | Cell offset pairs for spread iteration |

---

## 13. Corrections to Previous Reports

### 13.1 DAT_00a9fab4 Mislabeled

The `PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md` labels address `0x00A9FAB4` as
"NukeActive." This is **incorrect** — verified from `LightningStorm::Start` (0x00539EB0)
which sets this byte to 1 when a Lightning Storm begins, and from `LightningStorm::Process`
(0x0053A6C0) which clears it when the storm ends. Correct label: **LS_Active**.

### 13.2 SuperClass+0x60 Relabeled

Existing docs label this "IsSuspendedByPlayer." Analysis of Suspend (0x006CB4D0) and
Activate (0x006CB560) shows this field *gates* whether suspension changes can occur — it
does not track player-initiated suspension. Initialized to true in constructor, never set to
false in any decompiled function. Renamed to **AllowSuspension**.

### 13.3 SuperClass+0x7C Initial Value

Existing docs say ChargeDrainState is "0=empty, 1=charging, 2=draining." The constructor
initializes it to **-1** (not 0). Value -1 means "not in charge-drain mode." The 0/1/2
values are only relevant when UseChargeDrain is true.

---

## 14. Current Rust Implementation Status

**No `src/sim/superweapon/` module exists.** The only superweapon references in code:

| Component | File | Status |
|-----------|------|--------|
| Game option toggle | `src/sim/game_options.rs:20` | `super_weapons: bool` ✓ |
| Cursor definitions | `src/app_types.rs:100-114` | All 12 cursor types ✓ |
| Cursor atlas frames | `src/render/cursor_atlas.rs:250-342` | Full cursor set ✓ |
| Weapon type flags | `src/rules/weapon_type.rs:149` | `charges` flag ✓ |
| Droppod movement | `src/sim/movement/droppod_movement.rs` | Paradrop entry ✓ |
| Chrono miner teleport | `src/sim/miner/miner_system.rs` | Partial chrono ✓ |
| State hash | `src/sim/world/world_hash.rs:42` | Options hashed ✓ |

**Not implemented:** SuperWeaponTypeClass/SuperClass structs, charging/timer system,
launch dispatch, all 12 type handlers, sidebar cameo readiness, AI launch logic,
power integration, lightning storm state machine, psychic reveal, force shield,
psychic dominator state machine, nuke missile system, chronosphere warp, paradrops-as-SW.

---

## 15. Verification Pass — Corrections to Prior Reports

### 15.1 Chronosphere Delay Formula — Ceiling Claim INCORRECT

The `CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md` claims:
> Ceiling: `min(delay, ChronoReinfDelay)` (Rules+0xBF0, default 200)

**This is WRONG for the ChronoSphere superweapon path.** Verified from
`TeleportLocomotionClass::StateMachineTick` at `0x007192F0`: the actual formula is:

```
delay = distance / ChronoDistanceFactor       (Rules+0xBF4, default 32)
if delay <= ChronoMinimumDelay:                (Rules+0xBFC, default 0)
    delay = ChronoMinimumDelay                 // floor
if distance < ChronoRangeMinimum:              (Rules+0xC00, default 0)
    delay = ChronoMinimumDelay                 // force minimum for short distances
if WhatAmI()==1 AND TypeClass+0xE0E:           // Chronoshiftable=yes on ANY vehicle
    delay = 0                                  // instant teleport
```

**There is NO ceiling using ChronoReinfDelay.** That value (Rules+0xBF0) is only used for
scripted chrono reinforcements (a different code path), not the ChronoSphere superweapon.
The distance-based delay has no upper cap in the superweapon path.

Also: the instant teleport (delay=0) applies to **all Chronoshiftable vehicles**, not just
harvesters. The prior report said "Harvesters with Chronoshiftable=yes" — the implementation
is more general (any unit with WhatAmI()==1 and Chronoshiftable flag at TypeClass+0xE0E).

### 15.2 PD Mind Control Filter — 5 Conditions, Not 6

The `PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md` states "6 conditions" for the MC
filter but only lists 5. Verified from `PsychicDominator::Fire` at `0x0053B080`:

1. `WhatAmI() != 6` — Skip buildings
2. `TypeClass+0xD35 == 0` — Not ImmuneToPsionics
3. `vtable+0x160() == 0` — Not IronCurtained
4. `TypeClass+0xD6A == 0` — Not BalloonHover
5. `vtable+0x54() == 0` — Not in limbo

**There are exactly 5 conditions.** The "6" count in the prior report is an error.

### 15.3 PD Scatter Parameters — Now Known

The prior report noted `vtable+0x1E8` call for non-local-player units but didn't know the
params. Verified: **scatter(0xF, 0)** — direction=0xF (all directions), force=0.

### 15.4 HouseClass::SpyPowerSabotage — Missing Flag

The prior report documents HouseClass+0x2A4 (StartFrame) and +0x2AC (Duration) but misses
**HouseClass+0x5778**, which is set to 1 by SpyPowerSabotage. This flag likely controls
whether AI_AssessPower zeros power output during blackout.

### 15.5 NukeMaker Warhead Flag — CONFIRMED

WarheadTypeClass+0x176 (NukeMaker) verified directly in `WarheadTypeClass::Detonate`
at `0x004690B0`. The flag sits in a priority chain of warhead special effects — when true,
it calls `NukeMaker::SpawnDownwardNuke` instead of normal `Apply_area_damage`.

### 15.6 Iron Curtain TechnoClass Offsets — CONFIRMED

All offsets verified from `TechnoClass::IronCurtain` at `0x0070E2B0`:
- +0x18C = StartFrame (g_CurrentFrameCounter) ✓
- +0x190 = padding (timer reserved) ✓
- +0x194 = Duration ✓
- +0x1A4 = cleared to 0 ✓
- +0x1C4 = IsForceShield (1 or 0) ✓

`TechnoClass::IsIronCurtainActive` at `0x0041BF40` confirmed:
`return (StartFrame != -1) AND (CurrentFrame - StartFrame < Duration)`

### 15.7 BuildingClass::IronCurtain — Override Details

At `0x00457C90`: clears building flag at +0x6DF, resets timer at +0x528/+0x530,
clears +0x540, then calls parent `TechnoClass::IronCurtain`. The +0x6DF flag is likely
a "building under construction" or "sell pending" state that needs clearing before
applying invulnerability.

### 15.8 Anim Type Offsets Identified

| Offset | INI Key | Default Anim | Used By |
|--------|---------|-------------|---------|
| 0x0348 | IronCurtainInvokeAnim | IRONBLST | IC Launch (Case 1) |
| 0x034C | ForceShieldInvokeAnim | FORCSHLD | FS Launch (Case 10) |

### 15.9 Lightning Storm INI Keys Resolved

| Rules Offset | INI Key | Default | Comment from INI |
|-------------|---------|---------|------------------|
| 0x17A4 | `LightningScatterDelay` | 5 | "Frame delay between random bolts -- DO NOT DECREASE -- PERFORMANCE HIT" |
| 0x17A8 | `LightningCellSpread` | 10 | "how far away random bolts can go (n by n square)" |
| — | `LightningDeferment` | 250 | "frames between announcement and commencement" (passed as param, not a Rules offset) |

Rules+0x17B0 remains unidentified — it's a bool gating LS countdown notifications.
May be hardcoded or read from a section not yet traced.

---

## 16. Remaining Open Questions

1. **Rules+0x17B0 INI key name** — Lightning Storm notification enable flag. Not found in INI.

2. **SuperClass fields 0x44–0x4C** — Not referenced in any decompiled lifecycle function.

3. **Double RevealAroundCell call in Case 11** — Purpose of calling twice unclear.

4. **Nuke silo door animation timing** — Path B door open/close not fully traced.

5. **LS countdown EVA interval = 225 (0xE1)** — Hardcoded constant, no INI key.

---

## Sources

**Ghidra functions decompiled in this investigation:**
- 0x006CC390 (SuperClass::Launch — 958 lines, all 12 cases)
- 0x006CC080 (SuperClass::AI_Charging — 61 lines)
- 0x006CBCA0 (SuperClass::AI_Ready — 107 lines)
- 0x006CBEE0 (SuperClass::AnimStage — 22 lines)
- 0x006CC2B0 (SuperClass::NameReadiness — 35 lines)
- 0x006CC260 (SuperClass::GetRechargeTime — 13 lines)
- 0x006CAF90 (SuperClass::Constructor — 72 lines)
- 0x006CB560 (SuperClass::Activate — 123 lines)
- 0x006CB4D0 (SuperClass::Suspend — 47 lines)
- 0x006CB7B0 (SuperClass::Deactivate — 24 lines)
- 0x006CEA20 (SuperWeaponTypeClass::ReadINI — 163 lines)
- 0x00539EB0 (LightningStorm::Start — 74 lines)
- 0x0053A6C0 (LightningStorm::Process — 184 lines)
- 0x0053A140 (LightningStorm::CreateCloudBolt — 62 lines)
- 0x0053A300 (LightningStorm::GroundStrike — 120 lines)
- 0x0053C280 (SuperWeaponEffects::UpdateLighting — 27 lines)
- 0x005678E0 (MapClass::RevealAroundCell — 180 lines)
- 0x00653830 (RevealFogCell wrapper — 10 lines)
- 0x006EFC70 (AI::SuperLaunchCheck_SingleSW — 82 lines)
- 0x006EFE60 (AI::SuperLaunchCheck_DualSW — 109 lines)

**Verification pass (additional functions):**
- 0x0046B310 (NukeMaker::SpawnDownwardNuke — 40 lines)
- 0x0053B080 (PsychicDominator::Fire — 139 lines)
- 0x0070E2B0 (TechnoClass::IronCurtain — 19 lines)
- 0x0041BF40 (TechnoClass::IsIronCurtainActive — 19 lines)
- 0x00457C90 (BuildingClass::IronCurtain — 20 lines)
- 0x0050BC90 (HouseClass::SpyPowerSabotage — 15 lines)
- 0x0065EAB0 (SpyPlane spawner — 58 lines)
- 0x0065E660 (ParaDrop spawner — 84 lines)
- 0x004690B0 (WarheadTypeClass::Detonate — 140 of 543 lines, NukeMaker check)
- 0x007192F0 (TeleportLocomotionClass::StateMachineTick — 180 of 354 lines, delay formula)

**Total: 30 functions, ~2,790+ lines of decompilation.**

**Existing docs referenced:**
- SUPERWEAPON_TYPE_CLASS_GHIDRA_REPORT.md
- SUPERCLASS_SYSTEM_GHIDRA_REPORT.md
- SUPERWEAPON_LAUNCH_HANDLERS_REPORT.md
- NUKE_SUPERWEAPON_GHIDRA_REPORT.md
- IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md
- CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md
- PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md

**INI files checked:**
- ini/rulesmd.ini (primary)
- ini/rules.ini (base RA2)

**Date:** 2026-04-02
