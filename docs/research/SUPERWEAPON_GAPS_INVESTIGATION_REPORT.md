# Superweapon System — Gap Investigation Report

**Date:** 2026-04-02
**Scope:** Previously undocumented internals and verification of existing claims
**Confidence:** HIGH (all functions decompiled from binary)
**Active in YR:** Yes — all findings verified as active

---

## 1. FUN_0050bcd0 — HouseClass::SetStormWarning (0x0050BCD0)

**Previously:** Referenced but unidentified in Lightning Storm report.
**Finding:** This is NOT an EVA playback function. It sets a timer on a HouseClass.

```c
void __thiscall HouseClass__SetStormWarning(HouseClass* this, int duration) {
    this+0x5779 = 1;                    // Storm warning flag
    this+0x2B0 = g_CurrentFrameCounter; // Timer StartFrame
    this+0x2B4 = (padding);             // Timer Reserved
    this+0x2B8 = duration;              // Timer Duration (frames)
}
```

**HouseClass timer layout at +0x2B0:**

| Offset | Type | Purpose |
|--------|------|---------|
| 0x2B0 | int | Storm warning timer StartFrame |
| 0x2B4 | int | (CDTimerClass padding) |
| 0x2B8 | int | Storm warning timer Duration |

**Note:** This is adjacent to but separate from SpyPowerSabotage timer (+0x2A4/+0x2AC).
The flag at +0x5779 is initialized to **1** in the HouseClass constructor, suggesting
it defaults to "warning acknowledged" and is re-set when new storms arrive.

**Context from LightningStorm::Start:**
```
for each house NOT allied with LS owner AND NOT defeated:
    HouseClass__SetStormWarning(house, param_2)
```

Each non-allied house gets a storm warning timer. This likely gates some AI behavior
(e.g., scatter units, seek shelter) during the warning period.

**Confidence:** HIGH
**Active in YR:** Yes

---

## 2. PsychicDominator::Process (0x0053AF40) — Precise State Machine

**Previously:** Documented as "5-phase" but frame thresholds were approximate.
**Finding:** Exact phase transition conditions verified.

```
State 0: Inactive (no processing)

State 1 → 2: IMMEDIATE (single-frame transition)

State 2 → 3: When first anim (DAT_00a9fac4) current frame / total frames
             >= DominatorFireAtPercentage (Rules+0x304) / 100.0
    THEN: call PsychicDominator::Fire() — MC + area damage
    Default: fires at 20% through the animation

State 3 → 4: When first anim has < 11 frames remaining
    (totalFrames - currentFrame < 11)

State 4 → 5: When first anim has < 2 frames remaining
    (totalFrames - currentFrame < 2)
    THEN: reset PD target cell, clear anim pointer, begin lighting fade

State 5 → 0: When ambient lighting has fully returned to normal
    (ScenarioClass+0x3530 == ScenarioClass+0x352C)
```

**Frame access:** Anim current frame is at `AnimClass+0xAC`. Total frame count is
obtained via `AnimClass->AnimType (vtable chain) ->GetFrameCount()` returning
`*(short *)(typeResult + 6)`.

**Percentage constant:** `_DAT_007e3808` = 0.01 (confirmed by the multiplication
`DominatorFireAtPercentage * 0.01` producing a 0.0-1.0 fraction).

**Confidence:** HIGH
**Active in YR:** Yes

---

## 3. HouseClass SW Array Creation (in Constructor at 0x004F54A0)

**Previously:** Documented as "created in constructor" but mechanism not shown.
**Finding:** Full creation loop verified.

```c
// Near end of HouseClass::Constructor (lines ~500-520):

// For each SuperWeaponTypeClass in the global array:
for (i = 0; i < g_SuperWeaponTypeClass_Count; i++) {
    // Allocate SuperClass instance (0x80 = 128 bytes)
    SuperClass* sw = new(0x80) SuperClass(
        g_SuperWeaponTypeClass_Array[i],  // type
        this                              // owner house
    );

    // Add to house's DynamicVectorClass at +0x258
    house->SuperArray.Add(sw);  // +0x258=data, +0x264=count
}
```

**SuperClass confirmed size: 0x80 (128 bytes).**

**DynamicVectorClass layout at HouseClass+0x254:**

| Offset | Size | Type | Purpose |
|--------|------|------|---------|
| 0x254 | 4 | ptr | DynamicVectorClass vtable/allocator |
| 0x258 | 4 | ptr | Data pointer (SuperClass* array) |
| 0x25C | 4 | int | Capacity |
| 0x260 | 1 | bool | CanGrow flag |
| 0x264 | 4 | int | Count (current number of entries) |
| 0x268 | 4 | int | GrowAmount |

**Key detail:** ALL SuperClass instances are created at house construction time,
one per SuperWeaponTypeClass. They start as inactive (IsActive=false). Buildings
activate them later via `SuperClass::Activate` when `SuperWeapon=`/`SuperWeapon2=`
buildings complete construction.

**Global arrays:**
- `DAT_00a8e334` = SuperWeaponTypeClass array data pointer
- `DAT_00a8e340` = SuperWeaponTypeClass count

**Confidence:** HIGH
**Active in YR:** Yes

---

## 4. HouseClass::AI_ManageProduction — SW Tick Logic (0x0050AF10)

**Previously:** Referenced as "AI_ResumeProduction" but internal logic not documented.
**Finding:** This is the per-tick SW management function that handles building grants,
power suspension, and deactivation.

```
for each SW in house->SuperArray:
    if NOT IsActive: skip
    if AllowSuspension == false AND (not IsCharged OR not IsActive): skip
    if house defeated: skip

    hasBuilding = false      // any granting building exists
    hasPoweredBuilding = false  // granting building with power

    if NOT defeated:
        for each building in global BuildingClass array:
            if building alive AND owned by this house:
                // Check BuildingTypeClass+0x16F0 (SuperWeapon1)
                // Check BuildingTypeClass+0x16F4 (SuperWeapon2)
                if building grants this SW type:
                    hasBuilding = true
                    if building has power (Building+0x660): hasPoweredBuilding = true

    // DisableableFromShell check
    if type->DisableableFromShell AND disabled_in_shell:
        hasBuilding = false

    // Power ratio check
    if PowerOutput < PowerDrain AND PowerDrain != 0:
        if PowerOutput == 0 OR ratio < 1.0:
            hasPoweredBuilding = false

    // Decision:
    if hasBuilding AND NOT defeated:
        if NOT hasPoweredBuilding AND can_suspend:
            SuperClass::Suspend(1)      // low power → suspend
        elif hasPoweredBuilding:
            SuperClass::Suspend(0)      // power restored → unsuspend
    else:
        SuperClass::Deactivate()        // building lost → deactivate
        if is player: clear cursor, refresh sidebar
```

**Building SW index fields:**
- BuildingTypeClass+0x16F0 = SuperWeapon1 type index
- BuildingTypeClass+0x16F4 = SuperWeapon2 type index

**Confidence:** HIGH
**Active in YR:** Yes

---

## 5. EVA Voice Tables in AI_Charging / AI_Ready

**Previously:** Assumed type-specific EVA per SW type.
**Finding:** The switch tables are **empty** — all cases fall through to the same
`VoxClass::PlayEVA(-1)` call.

```c
// AI_Charging (0x006CC080):
switch(Type) {
    case 0: case 1: case 2: case 3:     // fall through
    case 5: case 6:                      // fall through
    case 7: case 8: case 9: case 10:     // fall through
    case 11:                             // fall through
        VoxClass::PlayEVA(-1);           // SAME call for all
    case 4:                              // ChronoWarp: SKIP (no EVA)
}
```

**Interpretation:** The parameter `-1` (0xFFFFFFFF) likely means "play the default
SW-ready EVA event" which is determined by the EVA system, not by the switch. The
switch's only purpose is to **exclude Case 4 (ChronoWarp)** — all other types trigger
the same generic EVA call.

The per-SW voice differentiation comes from `RechargeVoice=` in the INI, which is
played from a separate code path (not these switch tables).

**Confidence:** HIGH
**Active in YR:** Yes

---

## 6. SuperWeaponTypeClass Serialization (Load at 0x006CE800)

**Previously:** Not documented.
**Finding:** Load function restores base class, sets vtables, and registers two
pointer fields for swizzle fixup.

```c
int SuperWeaponTypeClass::Load(stream) {
    AbstractClass::Load(this, stream);   // restore base class
    AbstractTypeClass::Constructor();     // rebuild type base
    this+0xB8 = 0;                       // clear SidebarImageSHP (will reload)

    // Set vtable pointers
    this[0] = vtable__SuperWeaponTypeClass;
    this[1-3] = secondary vtables;

    // Register pointer fields for swizzle fixup:
    SwizzleRegister(&this+0x9C);   // WeaponType (WeaponTypeClass*)
    SwizzleRegister(&this+0xC8);   // AuxBuilding (BuildingTypeClass*)

    // Reload sidebar image SHP from MIX
    LoadFileFromMIX(this+0xCC);    // SidebarImageName string
    this+0xB8 = loaded SHP;

    return 0;
}
```

**SwizzleRegister (FUN_006cf240):** Records original pointer value + field address for
later resolution. Zeroes the field during registration. The swizzle system later
resolves all IDs back to live pointers in a batch pass.

**Confidence:** HIGH
**Active in YR:** Yes

---

## 7. SuperClass Destructor (0x006CB120)

**Previously:** Not documented in detail.
**Finding:** Cleans up anims and removes from 3 tracking arrays.

```
1. Restore vtable pointers
2. If anim at +0x68 exists:
    mark anim for deletion (anim+0x195 = 0)
    clear +0x68 = NULL
    remove from logic display via DAT_00b0f5b8
3. If field +0x6C flag set:
    remove from logic display
    clear +0x6C = 0
4. Remove from ShowTimer array (DAT_00a83d50)
5. Remove from global SuperClass array (DAT_00a83cb8)
6. Remove from third tracking array (DAT_00b0f670)
```

**Three global tracking arrays for SuperClass instances:**

| Array Base | Count | Purpose |
|-----------|-------|---------|
| DAT_00a83cb8 | DAT_00a83cc8 | Global SuperClass instances |
| DAT_00a83d50 | DAT_00a83d60 | ShowTimer display tracking |
| DAT_00b0f670 | DAT_00b0f680 | Third tracking array (purpose unclear) |

**Confidence:** HIGH
**Active in YR:** Yes

---

## 8. Nuke Silo Door — Building-Side Handling

**Previously:** "Door animation not fully traced."
**Finding from Launch Case 0 Path B:**

```
// Find the nuke silo building
for each BuildingTypeClass:
    if NukeSilo flag (+0x16BA) AND building's SW matches this SW type:
        building = HouseClass::Find_Building_Of_Type()
        if building exists:
            // Store launch target
            HouseClass+0x5784 = target cell (packed CellStruct)
            building+0x5F8 = SW type index

            // Trigger door animation
            building->vtable+0x1E8()   // Mission dispatch (begin open)
            building->vtable+0x1EC()   // Secondary action (finalize departure)

            // Play sounds
            VocClass::PlayAtCoord(building coords)
            VoxClass::PlayEVA()
```

**Key fields:**
- BuildingTypeClass+0x16BA = `NukeSilo` flag (bool)
- HouseClass+0x5784 = packed nuke target cell (CellStruct, set during path B)
- BuildingClass+0x5F8 = SW type index being fired (set on the silo building)

**Confidence:** HIGH for the dispatch mechanism.
MEDIUM for exact vtable+0x1E8/+0x1EC behavior (not resolved to concrete methods).

---

## 9. ChronoSphere Source Anim (FUN_006CB3A0)

**Previously:** Referenced as "setup anim at source" without details.
**Finding:** Creates `ChronoPlacement` animation at the ChronoSphere source cell.

```c
void SuperClass::SetupSourceAnim(coords, height) {
    // Clean up existing anims
    if this+0x68 (anim) exists:
        mark for deletion
        clear this+0x68
    if this+0x6C flag set:
        clear flag

    // Create ChronoPlacement anim
    anim = new AnimClass(
        Rules+0x330,     // ChronoPlacement = CHRONOAR
        coords,
        0, 1, 0x600, 0, 0  // flags: layer=0x600
    );
    this+0x68 = anim;   // store reference
    track in logic display array
}
```

**Rules+0x330 = ChronoPlacement = CHRONOAR** (confirmed from INI: line 544).

**Confidence:** HIGH
**Active in YR:** Yes

---

## 10. ParaDrop / SpyPlane Aircraft Spawn Mechanism

### ParaDrop (FUN_0065E660)

The paradrop spawner creates aircraft instances and loads infantry as cargo:

1. Get infantry type from the paradrop list (AllyParaDropInf, SovParaDropInf, etc.)
2. Check infantry type's `+0xDF8` field — this is the aircraft type index
   - If -1: skip (no associated aircraft)
3. Create infantry instance via TypeClass::CreateInstance
4. Calculate map edge spawn position via FUN_004aa440 based on house Side
5. Place aircraft at edge coordinates
6. If WhatAmI() == 2 (Aircraft): load additional cargo via CargoClass::AddPassenger
7. Aircraft flies to target cell, drops paratroopers

### SpyPlane (FUN_0065EAB0)

Similar to ParaDrop but:
- Creates SPYP aircraft type
- Sets mission to overfly target
- Does NOT load infantry cargo
- Aircraft fires SpyCameraWeapon every `SpyPlaneCameraFrames` (16) frames during flyover
- SpyCameraWeapon has Damage=6 → reveals 6-cell radius of shroud

### CargoClass::AddPassenger (0x004733A0)

Linked-list insertion for aircraft cargo:
```
function AddPassenger(cargoList, unit):
    traverse linked list via unit+0x30 (next passenger)
    append unit to end of list
    increment cargoList count at *cargoList
```

**Confidence:** MEDIUM (parameter mapping uncertain due to __fastcall)
**Active in YR:** Yes

---

## 11. BuildingClass SW Index Helpers

### GetSuperWeaponIndex1 (0x00457630)

```c
int BuildingClass::GetSuperWeaponIndex1() {
    swIndex = this->BuildingType->SuperWeapon1 (+0x16F0)
    if swIndex != -1:
        swType = g_SuperClass_Array[swIndex]->Type
        if swType->AuxBuilding (+0xC8) != NULL:
            aircraftType = swType->AuxBuilding+0xDF8  // aircraft requirement
            if HouseClass::CountOwnedInstances(aircraftType) == 0:
                return -1  // can't fire without required aircraft
    return swIndex
}
```

### GetSuperWeaponIndex2 (0x00457690)

Same as above but reads from BuildingTypeClass+0x16F4 (SuperWeapon2 slot).

**Purpose:** These check whether a building can actually provide its SW, including
verifying that any required auxiliary aircraft type is owned by the house.

**Confidence:** HIGH
**Active in YR:** Yes

---

## 12. HouseClass Flag Fields Near Storm/Power Timers

| Offset | Type | Init | Purpose |
|--------|------|------|---------|
| 0x2A4 | CDTimerClass | — | Power blackout timer (SpyPowerSabotage) |
| 0x2B0 | CDTimerClass | — | Storm warning timer (SetStormWarning) |
| 0x5778 | byte | 0 | Power sabotage active flag |
| 0x5779 | byte | 1 | Storm warning flag (default=1, re-set on new storm) |
| 0x577A | byte | 0 | Unknown flag |
| 0x577B | byte | 0 | Unknown flag |
| 0x577C | int | varies | House map edge side (0-3, used for paradrop spawn) |

---

## Sources

**Ghidra functions decompiled:**
- 0x0050BCD0 (HouseClass::SetStormWarning — 15 lines)
- 0x0053AF40 (PsychicDominator::Process — 50 lines)
- 0x004F54A0 (HouseClass::Constructor — 632 lines, SW init at ~500)
- 0x0050AF10 (HouseClass::AI_ManageProduction — 102 lines)
- 0x006CAEC0 (SuperClass::Constructor 0-param — 51 lines)
- 0x006CE800 (SuperWeaponTypeClass::Load — 33 lines)
- 0x006CF240 (SwizzleRegister — 33 lines)
- 0x006CB120 (SuperClass::Destructor — 78 lines)
- 0x006CB3A0 (SuperClass::SetupSourceAnim — 58 lines)
- 0x0065E660 (ParaDrop spawner — 84 lines)
- 0x0065EAB0 (SpyPlane spawner — 58 lines)
- 0x004733A0 (CargoClass::AddPassenger — 39 lines)
- 0x00457630 (BuildingClass::GetSuperWeaponIndex1 — 17 lines)
- 0x00457690 (BuildingClass::GetSuperWeaponIndex2 — 17 lines)
- 0x0050DA80 (HouseClass::GetSideIndex — 13 lines)
- 0x004AA440 (MapEdge spawn position — 284 lines, partial)

**Total: 16 functions, ~1,500+ lines of decompilation.**

## 13. Additional Verification Findings (1st pass)

### 13.1 Nuke Flash Shares PD State Machine (NEW)

`ScreenNukeFlash` at `0x0053AB70` sets `DAT_00a9fabc` (PD_State) to 1 and delay to **0x1E
(30 frames)**. The LightningStorm::Process function then handles the 1→2→0 transition:

```
Phase 1: Flash ON for 30 frames (PD_State=1)
Phase 2: Lighting transition for 15 frames (PD_State=2)
Phase 3: Return to normal (PD_State=0)
Total flash duration: 45 frames (3 seconds at 15fps)
```

The nuke flash, PD flash, and LS ambient lighting ALL share the same state variable
(DAT_00a9fabc) and the same transition code in LightningStorm::Process. This means:
- A nuke detonation during an active PD will reset the PD state to 1 (flash phase)
- Nuke flash and PD flash cannot coexist — the last one to fire wins

**Confidence:** HIGH (verified from ScreenNukeFlash + LightningStorm::Process)

### 13.2 ForceShield Uses 3D Euclidean Distance, Not Manhattan

The ForceShield radius check in Launch case 10 uses `CoordStruct::Distance3D()` —
true euclidean distance (sqrt(dx² + dy² + dz²)). The comparison is:
```
if Distance3D(building, target) < ForceShieldRadius × 256 leptons:
    apply invulnerability
```

This is **different** from Lightning Storm bolt separation which uses **manhattan distance**
(|dx| + |dy|). The distinction matters for buildings on hills or bridges.

**Confidence:** HIGH

### 13.3 Iron Curtain 3×3 Grid Verified

Address range 0xB0C038 to 0xB0C05B = 36 bytes = 9 CellStruct entries (2 shorts each).
Iterator increments by 2 shorts per step. Loop continues while `< 0xB0C05C` (exclusive bound).

Standard 3×3 offset pattern:
```
(-1,-1) (0,-1) (1,-1)
(-1, 0) (0, 0) (1, 0)
(-1, 1) (0, 1) (1, 1)
```

**Confidence:** HIGH (loop bounds confirmed from Launch case 1 code)

### 13.4 PD Permanent MC Offsets Confirmed

From PD::Fire (0x0053B080):
- `piVar4[0xB1]` = TechnoClass+0x2C4 (byte) = **IsPermanentlyMindControlled** (set to 1) ✓
- `piVar4[0xB2]` = TechnoClass+0x2C8 (ptr) = **MC anim pointer** (stores MINDANIMR anim) ✓

### 13.5 LS Bolt Separation Uses Manhattan Distance Confirmed

From LightningStorm::Process random bolt placement:
```c
delta = |randomCell.X - boltCell.X| + |randomCell.Y - boltCell.Y|
if delta < LightningSeparation (Rules+0x17AC, default 3):
    tooClose = true
```

This matches the INI comment: "SJM: city-block distance in cells between clouds/bolts"

**Confidence:** HIGH

### 13.6 ForceShield GetAction Special Cursor (NEW)

`SuperWeaponTypeClass::GetAction` at `0x006CEF80` has special logic for Type==10 (ForceShield):
- If target is an **allied building** (WhatAmI()==6 AND IsAlliedWith): use normal Action cursor
- Otherwise: return **0x46** (70 = NoForceShield cursor)

This means ForceShield shows a "can't target" cursor when hovering over enemies or non-buildings.

**Confidence:** HIGH

### 13.7 Apply_area_damage Damage Source — Open Question

Neither PD::Fire nor LS::GroundStrike pass an explicit damage value to `Apply_area_damage`.
Both calls pass: (coords_in_ECX, warhead_ptr, flags, owner_house). The function signature
has 6 params total (__fastcall), but the damage value's source is unclear:

- It may be in a register argument hidden by the decompiler
- It may be looked up from Rules inside Apply_area_damage based on the warhead
- It may come from the warhead's own damage field

The INI values `DominatorDamage=1000` and `LightningDamage=250` are read from Rules at
offsets 0x30C and an unidentified offset respectively, but their path to Apply_area_damage
is not directly traceable from the decompilation. Further investigation needed.

**RESOLVED:** `param_2` (EDX register) IS the damage value. Apply_area_damage saves it
as `local_c4 = param_2` at the start, then for each target in CellSpread range:
```c
iStack_bc = local_c4;  // restore damage value
TechnoClass::ReceiveDamage(&iStack_bc, distance, warhead, source, 0, 0, owner);
```

DominatorDamage (Rules+0x30C = 1000) and LightningDamage (Rules+0x1798 = 250) are loaded
into EDX before the call. Ghidra's decompiler hides register setup for __fastcall.

**Complete Apply_area_damage signature:**
```c
__fastcall Apply_area_damage(
    CoordStruct* center,   // ECX: ground zero
    int damage,            // EDX: damage value per target
    TechnoClass* source,   // stack: attacker (NULL for SW area effects)
    WarheadTypeClass* wh,  // stack: warhead for Verses/armor calc
    char flags,            // stack: behavior flags
    HouseClass* owner      // stack: house for attribution
)
```

**Confidence:** HIGH (verified from function body + ReadGeneral offsets)

### 13.8 MakeInfantry Spawn Chain Partially Verified

AnimClass::AI at 0x00423AC0 (lines 516-576) handles MakeInfantry spawn when anim completes:

1. Check `AnimTypeClass+0x34C` (MakeInfantry index) != -1
2. Validate against `Rules+0xCF4` (AnimToInfantry count)
3. Resolve owner house (use existing owner or find neutral house by Side)
4. Look up type from `AnimToInfantry[MakeInfantry]` (Rules+0xCE8 array)
5. Access `InfantryTypeClass+0xDF8` for aircraft/unit type index
6. Create instance via TypeClass::CreateInstance
7. Place at anim coordinates
8. If on bridge cell: adjust Z position

The spawn chain is confirmed but the exact type lookup chain through +0xDF8 into what
Ghidra labels as `g_AircraftTypeClass_Array` needs more investigation — the Ghidra label
may be incorrect for this context.

**Confidence:** MEDIUM

---

## 14. Extended Verification Pass (2nd session)

### 14.1 Complete Lightning Storm INI Key → Rules Offset Map

Verified from `RulesClass::ReadGeneral` (lines 1680-1710 of 2068):

| Rules Offset | INI Key | Type | Default |
|-------------|---------|------|---------|
| 0x1794 | `LightningDeferment` | int | 250 |
| 0x1798 | `LightningDamage` | int | 250 |
| 0x179C | `LightningStormDuration` | int | 180 |
| 0x17A0 | `LightningHitDelay` | int | 10 |
| 0x17A4 | `LightningScatterDelay` | int | 5 |
| 0x17A8 | `LightningCellSpread` | int | 10 |
| 0x17AC | `LightningSeparation` | int | 3 |
| 0x17B0 | `LightningPrintText` | bool | (false) |
| 0x17B4 | `LightningWarhead` | ptr | IonWH |

All 9 Lightning Storm keys now mapped. `LightningPrintText` was the last unknown.
Not in rulesmd.ini because standard YR doesn't set it — defaults to false.

**Confidence:** HIGH

### 14.2 Psychic Dominator Rules Offsets Verified

From ReadGeneral lines 351-380:

| Rules Offset | INI Key | Type | Default |
|-------------|---------|------|---------|
| 0x2F8 | `DominatorWarhead` | WarheadType* | DominatorWH |
| 0x2FC | `DominatorFirstAnim` | AnimType* | PDFXCLD |
| 0x300 | `DominatorSecondAnim` | AnimType* | PDFXLOC |
| 0x304 | `DominatorFireAtPercentage` | int | 20 |
| 0x308 | `DominatorCaptureRange` | int | 1 (clamped max 10) |
| 0x30C | `DominatorDamage` | int | 1000 |

**Confidence:** HIGH

### 14.3 PsychicDominator::Start Verified (0x0053AE50)

```
Guard: DominatorFirstAnim AND DominatorSecondAnim must be non-null.
If either is 0: PD silently fails (does nothing).

PD_TargetCell = target cell
PD_OwnerHouse = owner house (for damage, NOT for MC)
Create DominatorFirstAnim (PDFXCLD) at target
PD_CurrentAnim = created anim
PD_State = 1

ScenarioClass+0x1248 = CurrentFrame  (SAME timer slot as ScreenNukeFlash)
ScenarioClass+0x1250 = 1
UpdateLighting()
```

PD::Start and ScreenNukeFlash share ScenarioClass+0x1248 timer — nuke during active PD
resets the timer.

**Confidence:** HIGH

### 14.4 Teleporter Flag = Chrono Death (TypeClass+0xCD4)

In Chronosphere Launch case 4, units with `Teleporter=yes` (TypeClass+0xCD4) are
**KILLED**, not warped. The eligibility check:

```
if Chronoshiftable==false (0xD97) OR Teleporter==true (0xCD4) OR InLimbo:
    → KILL unit
else:
    → WARP unit to destination
```

Chrono Miners and Chrono Legionnaires are killed by Chronosphere.
This matches real YR behavior.

**Confidence:** HIGH

### 14.5 PsychicRevealRadius Capped at 10

`PsychicRevealRadius` read in `FUN_0066BBB0` at `0x0066C65D` → Rules+0xFEC = 15.
But `MapClass::RevealAroundCell` clamps radius to max 10 (from CellSpread table).
**Effective reveal radius is 10 cells, not 15.**

**Confidence:** HIGH

### 14.6 Chrono Animation Keys from ReadGeneral

| Rules Offset | INI Key | AnimType |
|-------------|---------|---------|
| 0x0328 | `ChronoBlast` | CHRONOFD (source departure) |
| 0x0330 | `ChronoPlacement` | CHRONOAR (source select) |
| 0x0334 | `ChronoBeam` | CHRONOBM |

**Confidence:** HIGH

---

## Extended Sources

**Additional functions examined:**
- 0x0053AE50 (PsychicDominator::Start — 34 lines)
- 0x0053AB70 (ScreenNukeFlash — 20 lines)
- Apply_area_damage lines 260-380 (damage application via ReceiveDamage)
- RulesClass::ReadGeneral lines 350-380, 1600-1760 (INI offset verification)

**String searches:**
- `LightningPrintText` → 0x0083BC74 → ReadGeneral 0x0067107F
- `LightningDamage` → 0x0083BD08 → ReadGeneral 0x00670FA2
- `DominatorDamage` → 0x0083CE0C → ReadGeneral 0x0066E05F
- `PsychicRevealRadius` → 0x0083B0A4 → FUN_0066BBB0 0x0066C65D
- `C4Warhead` → 0x0083B1D4 → ReadCombatDamage 0x0066C31F
- `RechargeVoice` → **NOT FOUND IN BINARY**

---

## 15. Third Verification Pass — Critical Findings

### 15.1 RechargeVoice INI Key — NOT READ BY THE ENGINE

**The string "RechargeVoice" does not exist in gamemd.exe.** Only "RechargeTime" exists
as a related string. The key appears in rulesmd.ini (`NukeSpecial: RechargeVoice=00-I154`,
etc.) but is **completely ignored** by the original engine.

This means:
- Per-SW EVA voice differentiation does NOT come from `RechargeVoice=`
- The EVA system for SW readiness uses VoxClass::PlayEVA with event name strings
- The AI_Charging/AI_Ready switch tables set event name strings via register (ECX),
  which Ghidra's decompiler hides, making them appear identical
- `RechargeVoice=` may only work with third-party mods (e.g., Ares)

**Confidence:** HIGH (binary string search confirmed absence)

### 15.2 DAT_00a9fab0 — Nuke Screen Flash State Machine (NOT "NukeActive")

The PD report labeled 0x00A9FAB0 as "NukeActive." This is **incorrect**. It's actually
the **Nuke Screen Flash State** — a 3-phase visual overlay state machine managed by
`Process_QueuedEvents`.

```
State 0: Inactive

State 1: White flash overlay
    - Blit white screen each tick via FUN_0053BBA0
    - Decrement counter DAT_00a9fa98
    - When counter == 0: advance to State 2, counter = 45 (0x2D)

State 2: Explosion render phase
    - Fill screen with white color (RGB encoded)
    - Decrement counter
    - At counter == 1: begin transition
    - At counter == 0:
        - Swap rendering surfaces
        - Render one actual game frame (RenderFrame_main)
        - Play nuke explosion sound
        - Advance to State 3

State 3: Post-flash fade
    - Continue blitting via FUN_0053BBA0
    - Decrement counter
    - At counter == 0:
        - Restore rendering surfaces
        - Resume EVA voices
        - Clean up sound objects
        - Return to State 0
```

**This is SEPARATE from the ambient lighting state (DAT_00a9fabc).** A nuke detonation
triggers BOTH:
1. DAT_00a9fabc = 1 (ambient lighting flash, 30 frames) — via ScreenNukeFlash
2. DAT_00a9fab0 = 1 (screen white overlay) — via Process_QueuedEvents_WithSuspend

PD only uses #1 (ambient), not #2 (screen overlay).

**Confidence:** HIGH (decompiled Process_QueuedEvents, 70 lines)

### 15.3 Chronosphere Kill Mechanism Verified

Non-chronoshiftable units are killed with:
```c
damage = TypeClass+0xA0 (Strength = max HP)
warhead = Rules+0xFA8 (C4Warhead, from [CombatDamage] section)
ReceiveDamage(&damage, 0, C4Warhead, 0, 0, 0)
```

C4Warhead is a 100% Verses warhead designed to ensure instant kill regardless of
armor type. Damage equals the unit's full HP.

**Rules+0xFA8 = C4Warhead** (verified from ReadCombatDamage at 0x0066C31F)
**Rules+0xFAC = CrushWarhead** (adjacent, also verified)

**Confidence:** HIGH

### 15.4 PD Damage House (DAT_00a9facc) — NOT Set by PD::Start

PD::Start (0x0053AE50) sets:
- DAT_00a9fac8 = owner house (for SetOwner/MC)
- DAT_00a9fa48 = target cell
- DAT_00a9fac4 = first anim
- DAT_00a9fac0 = state

But it does **NOT** set DAT_00a9facc. This address is:
- Written by `LightningStorm::Start` as LS_OwnerHouse
- Read by `PD::Fire` for Apply_area_damage damage attribution

This means DAT_00a9facc is a **shared** field. For PD damage attribution, it must
be set by the Launch dispatcher or by some other mechanism before PD::Start is called.
If it retains a stale LS owner value, PD damage could be attributed to the wrong house.

**Confidence:** HIGH (verified from PD::Start decompilation)
**Status:** Open question — where is DAT_00a9facc set for PD?

### 15.5 SuperWeaponEffects::ResetAll (0x00539760)

Global reset function that clears ALL superweapon effect state. Called at match
start/end. Resets:
- All target cells to default
- All state flags to 0
- All timers to -1
- All anim tracking arrays (frees memory)
- Lighting back to normal ambient
- Sound objects cleaned up
- DAT_00a9fab0 (NukeFlashState) = 0

**Confidence:** HIGH (49 lines, fully decompiled)

### 15.6 FUN_004F42F0 — Tactical Display Redraw Trigger

Called by ScreenNukeFlash and LightningStorm::Start:
```c
g_Tactical+0xD7D = 1;  // Dirty flag for tactical display
this+0xC = param_2;    // Set redraw mode (if not already 2)
FUN_00578AC0();         // Trigger actual redraw
```

**Confidence:** HIGH

---

## 16. Fourth Verification Pass — Deep Details

### 16.1 HouseClass+0x1FC is ProductionChanged, NOT "Nuke Launched Flag"

The nuke reports state that Launch case 0 sets `HouseClass+0x1FC = 1` as a "nuke in flight
flag." The HOUSECLASS_GHIDRA_REPORT documents this field as **ProductionChanged** — a generic
dirty flag triggering `AI_ManageProduction` and `AI_ResumeProduction`.

Setting it after nuke launch causes the SW management system to re-evaluate SW states
(update sidebar, trigger recharge). NOT nuke-specific.

**Confidence:** HIGH (cross-referenced with HOUSECLASS_GHIDRA_REPORT)

### 16.2 Iron Curtain Damage Rejection Verified from Binary

From `TechnoClass::ReceiveDamage` at `0x00701900` (682-line function, lines 93-105):

```
isInvulnerable = IsIronCurtainActive()      // vtable+0x160
if (isInvulnerable AND NOT force_damage AND damage >= 0):
    if (this+0x1C4 == 1):  flash_type = 6   // ForceShield spark
    else:                  flash_type = 1   // IronCurtain spark
    FUN_0048A620(X, Y, Z, 1, flash_type)    // Visual spark effect
    *damage = 0                              // Nullify
    return 0

// Second invulnerability gate (right after IC check):
isWarping = vtable+0x1D4()                  // "being warped" check
if (isWarping AND NOT force_damage):
    *damage = 0                              // Silent nullify (no spark)
    return 0
```

Key details:
- Negative damage (healing) bypasses IC check (`bVar13 = *damage < 0`)
- `force_damage` flag (param_5) bypasses IC check
- vtable+0x1D4 is a SECOND invulnerability gate — likely "currently being chronoshifted"
  — providing silent damage rejection without spark effect
- Flash type 6 (ForceShield) and 1 (IronCurtain) produce different visual sparks

**Confidence:** HIGH (verified from binary)

### 16.3 CDTimerClass::Pause/Resume Helpers

**FUN_006CE280 — Pause (save remaining time):**
Calculates `remaining = Duration - (CurrentFrame - StartFrame)`, stores in Duration,
sets StartFrame = -1 (stopped).

**FUN_006CE2C0 — Resume (restart from saved):**
If StartFrame == -1, sets StartFrame = CurrentFrame. Duration still has the remaining
time from the Pause call.

Used by SuperClass::Activate and SuperClass::Suspend for ManualControl handling.

**Confidence:** HIGH

### 16.4 SW Global State Serialization Order

From Save function at `0x00539890` (249 lines), the complete serialization order:

```
 1. LS_Active           (1 byte)   DAT_00a9fab4
 2. LS_StartFrame       (4 bytes)  DAT_00827fc0
 3. LS_Duration          (4 bytes)  DAT_00827fc4
 4. LS_QueueCountdown   (4 bytes)  DAT_00a9fab8
 5. LS_Ending           (1 byte)   DAT_00a9fad0
 6. StateStartFrame     (4 bytes)  DAT_00827fc8  (shared PD/LS/Nuke)
 7. StateDelay          (4 bytes)  DAT_00827fcc  (shared PD/LS/Nuke)
 8. PD_FlashState       (4 bytes)  DAT_00a9fabc  (shared nuke/PD/LS)
 9. PD_ProcessState     (4 bytes)  DAT_00a9fac0
10. PD_CurrentAnim      (4 bytes)  DAT_00a9fac4  (ptr, swizzled)
11. LS_TargetCell       (4 bytes)  DAT_00a9f9cc
12. PD_TargetCell       (4 bytes)  DAT_00a9fa48
13. Shared_DamageHouse  (4 bytes)  DAT_00a9facc  (ptr, swizzled)
14. PD_MCHouse          (4 bytes)  DAT_00a9fac8  (ptr, swizzled)
15. Strike anim count + array (variable)
16. Cloud bolt count + array (variable)
```

**Confidence:** HIGH

### 16.5 PD Damage House Bug — Full Xref Trace

All WRITE xrefs to DAT_00a9facc:

| Function | Address | Value |
|----------|---------|-------|
| LightningStorm::Start | 0x00539F4B | LS owner house |
| LightningStorm::Process | 0x0053A8E4 | 0 (cleanup) |
| SuperWeaponEffects::ResetAll | 0x005397B9 | 0 (reset) |

**PD::Start never writes to this address.** PD area damage is attributed to the last
LS owner house or NULL. This is likely an engine bug but rarely matters in practice
because PD's main effect is permanent MC (correctly attributed via DAT_00a9fac8).

**Confidence:** HIGH

### 16.6 Nuke Screen Flash Trigger — Process_QueuedEvents_WithSuspend

`Process_QueuedEvents_WithSuspend` at `0x0053B4F0` (46 lines):

1. Create offscreen surface, copy current screen
2. Set `DAT_00a9fab0 = 1` (start NukeFlashState machine)
3. Set countdown = `param_1` (flash duration)
4. Pause all audio (sounds, voice, music)
5. Play nuke detonation sound
6. Save weather state, disable weather overlay
7. Suspend EVA voice system

Called from the warhead detonation chain, not from SuperClass::Launch directly.

**Confidence:** HIGH

### 16.7 DynamicVectorClass::Remove (FUN_006CE2D0)

Standard array removal by shifting elements left. Used to remove SuperClass instances
from tracking arrays (ShowTimer, global instances).

**Confidence:** HIGH

**Date:** 2026-04-02 (4th pass)
