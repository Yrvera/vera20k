# Radiation & EMP Systems — Ghidra Decompilation Report

Decompiled from `gamemd.exe`. Confidence: **high** — all offsets verified from binary.

---

## Part 1: Radiation System

### Overview

The radiation system uses `RadSiteClass` objects to represent areas of radioactive contamination.
When a weapon with `RadLevel > 0` detonates, it creates (or augments) a RadSite at the impact
cell. The RadSite then persists for a duration, applying damage to objects in irradiated cells
and emitting a colored light glow that fades over time.

### 1.1 RadSiteClass Struct Layout

**Size: 0x74 bytes** (confirmed from vtable entry returning `0x74`).

| Offset | Type     | Name               | Notes |
|--------|----------|--------------------|-------|
| 0x00   | ptr      | vtable             | Points to 0x007F0810 |
| 0x04   | ptr      | vtable_secondary_4 | INoticeSink vtable |
| 0x08   | ptr      | vtable_secondary_8 | |
| 0x0C   | ptr      | vtable_secondary_C | |
| 0x10–0x23 | ...   | AbstractClass base | Inherited fields |
| 0x24   | ptr      | LightSource        | LightSourceClass* for the glow |
| 0x28   | int      | RadLevelTimer_Start| Frame counter for rad level timer |
| 0x2C   | int      | RadLevelTimer_?    | (secondary timer field) |
| 0x30   | int      | RadLevelTimer_Delay| Timer delay (= RadLevelDelay from rules) |
| 0x34   | int      | RadLightTimer_Start| Frame counter for light update timer |
| 0x38   | int      | RadLightTimer_?    | (secondary timer field) |
| 0x3C   | int      | RadLightTimer_Delay| Timer delay (= RadLightDelay from rules) |
| 0x40   | short    | CellX              | Center cell X coordinate |
| 0x42   | short    | CellY              | Center cell Y coordinate |
| 0x44   | int      | Spread             | Radius in cells |
| 0x48   | int      | SpreadInLeptons    | = Spread * 256 + 128 |
| 0x4C   | int      | RadLevel           | Current radiation level (from weapon) |
| 0x50   | int      | RadLevelPerStep    | = TotalDuration / RadLevelDelay |
| 0x54   | int      | LightIntensity     | Computed from RadLevel * RadLightFactor |
| 0x58   | int      | LightTintRed       | Red component, from RadColor * RadTintFactor |
| 0x5C   | int      | LightTintGreen     | Green component |
| 0x60   | int      | LightTintBlue      | Blue component |
| 0x64   | int      | LightIntensityPerStep | = TotalDuration / RadLightDelay |
| 0x68   | int      | LightIntensityDecrement | = LightIntensity / LightIntensityPerStep |
| 0x6C   | int      | TotalDuration      | = RadLevel * RadDurationMultiple |
| 0x70   | int      | RemainingDuration  | Decremented by 1 each tick |

### 1.2 Rules.ini Keys (Section: `[Radiation]`)

Read by function at **0x0066CF70** into the global `RulesClass` instance.

| Key                  | Type   | RulesClass Offset | Description |
|----------------------|--------|-------------------|-------------|
| `RadDurationMultiple`| int    | 0x1804            | Multiplied by RadLevel to get total duration in frames |
| `RadApplicationDelay`| int    | 0x1808            | Frames between radiation damage applications |
| `RadLevelMax`        | int    | 0x180C            | Maximum radiation level (caps RadLevel) |
| `RadLevelDelay`      | int    | 0x1810            | Frames between rad-level decay steps |
| `RadLightDelay`      | int    | 0x1814            | Frames between light intensity updates |
| `RadLevelFactor`     | double | 0x1818            | Factor for computing per-cell damage from level |
| `RadLightFactor`     | double | 0x1820            | Factor for computing light intensity from level |
| `RadTintFactor`      | double | 0x1828            | Factor for computing color tint from RadColor |
| `RadColor`           | color  | 0x1830            | RGB color of the radiation glow (3 bytes) |
| `RadSiteWarhead`     | string | 0x1834            | Warhead type pointer used for radiation damage |

### 1.3 WeaponTypeClass Fields

| Key        | Type | Offset | Description |
|------------|------|--------|-------------|
| `RadLevel` | int  | 0x158  | Radiation level this weapon deposits. 0 = no radiation. |

Read at address **0x007728DA** in `WeaponTypeClass::ReadINI`.

### 1.4 TechnoTypeClass Immunity

| Key                 | Type | Offset  | Description |
|---------------------|------|---------|-------------|
| `ImmuneToRadiation` | bool | 0xD37   | If true, unit is immune to radiation damage |

Read at address **0x00714D53** in `TechnoTypeClass::ReadINI`.

### 1.5 RadSite Creation (from `WarheadTypeClass::Detonate`)

**Address: 0x004690B0** — the `WarheadTypeClass::Detonate` function.

Radiation creation code (starting around offset +0x110 in the function):

```
// param_1[0x4C] = BulletClass pointer
// param_1[0x4C] → offset 0x158 = weapon's RadLevel
if (param_1[0x4C] != 0 && *(int*)(param_1[0x4C] + 0x158) > 0) {
    cell = MapClass::Get_CellClass(impact_coords);
    existingRadSite = CellClass::GetRadSite(cell);   // offset 0xF8

    if (existingRadSite == 0) {
        // Create new RadSiteClass (size 0x74)
        radSite = new RadSiteClass();                 // 0x0065B1E0
        RadSiteClass::SetCell(radSite, cell);         // 0x0065B4C0 → stores at +0x40
        RadSiteClass::SetSpread(radSite, spread);     // 0x0065B4D0 → stores at +0x44, +0x48
        RadSiteClass::SetRadLevel(radSite, radLevel); // 0x0065B4F0 → stores at +0x4C, +0x6C, +0x70
        RadSiteClass::Activate(radSite);              // 0x0065B580 → creates light, sets cell levels
        CellClass::SetRadSite(cell, radSite);         // 0x00487C70 → stores at cell +0xF8
    } else {
        RadSiteClass::AddRadLevel(radSite, additionalLevel); // 0x0065B530
    }
}
```

**Key behavior:**
- If a cell already has a RadSite, the new RadLevel is *added* to the existing one
  (function at 0x0065B530 recalculates duration and reactivates)
- The spread is derived from the weapon/warhead (number of cells radius)
- `SpreadInLeptons = Spread * 256 + 128` (0x0065B4D0)
- `TotalDuration = RadLevel * RadDurationMultiple` (0x0065B4F0)

### 1.6 RadSite Activation (`0x0065B580`)

When activated, the RadSite:

1. Sets the RadLevelDelay and RadLightDelay timers from RulesClass
2. Computes light intensity: `LightIntensity = ftol(RadLevel * RadLightFactor)`
3. Computes tint RGB: `TintR/G/B = ftol(RadColor.R/G/B * RadTintFactor)`
4. Computes per-step decrements:
   - `RadLevelPerStep = TotalDuration / RadLevelDelay`
   - `LightIntensityPerStep = TotalDuration / RadLightDelay`
   - `LightDecrement = LightIntensity / LightIntensityPerStep`
5. Creates a `LightSourceClass` at the center cell's 3D coordinates (if first activation)
   or updates existing light source intensity/tint
6. Calls `FUN_0065B9C0` to set initial radiation levels on all cells within the spread radius

### 1.7 Cell Radiation Level Setup (`0x0065B9C0`)

Iterates all cells in a square region `(CellX - Spread)` to `(CellX + Spread)` on both axes.

For each cell:
1. Computes 3D distance from center cell to target cell
2. If distance <= SpreadInLeptons:
   - `cellRadLevel = ((SpreadInLeptons - distance) / SpreadInLeptons) * RadLevel`
   - Linear falloff from center (full RadLevel) to edge (zero)
3. Calls `CellClass::IncreaseRadLevel(cellRadLevel)` (0x00487CE0)
   - Adds to the double at CellClass offset 0xF0

### 1.8 RadSiteClass::AI / Per-Tick Update (`0x0065B800`)

**Vtable offset 0x5C** — called every game tick.

```
AI() {
    RemainingDuration--;                    // offset 0x70

    // Timer 1: RadLevelDelay — apply radiation damage
    if (RadLevelTimer has expired) {
        ApplyRadDamage();                   // 0x0065BD00
        // Reset timer with RadLevelDelay from rules
        RadLevelTimer_Start = currentFrame;
        RadLevelTimer_Delay = RulesClass->RadLevelDelay;
    }

    // Timer 2: RadLightDelay — update light source
    if (RadLightTimer has expired) {
        // Compute fading tint based on remaining duration
        factor = RemainingDuration / TotalDuration;
        newR = (TintRed * RemainingDuration) / TotalDuration;
        newG = (TintGreen * RemainingDuration) / TotalDuration;
        newB = (TintBlue * RemainingDuration) / TotalDuration;
        newIntensity = LightSource.intensity - LightDecrement;
        LightSource.Update(newIntensity, newR, newG, newB, 0);
        // Reset timer
        RadLightTimer_Start = currentFrame;
        RadLightTimer_Delay = RulesClass->RadLightDelay;
    }

    // Self-destruct when duration expires
    if (RemainingDuration <= 0) {
        this->~RadSiteClass();  // via vtable offset 0x20, flag=1
    }
}
```

### 1.9 Radiation Damage Application (`0x0065BD00`)

Called when the RadLevelDelay timer fires. Iterates all cells in the spread radius:

```
for each cell in (CellX - Spread) to (CellX + Spread):
    distance = 3D_distance(center_cell, target_cell);
    if (distance <= SpreadInLeptons):
        radAmount = ((SpreadInLeptons - distance) / SpreadInLeptons) * RadLevel;
    else:
        radAmount = 0.0;
    CellClass::DecreaseRadLevel(target_cell, radAmount / RadLevelPerStep);
```

This **decreases** the radiation level stored in each cell proportionally, causing the
radiation field to decay over time.

**Note:** The actual damage to units from radiation is applied through the cell's radiation
level interacting with objects during the game's per-object update loop. Objects in cells
with `CellClass.RadLevel > 0` (offset 0xF0) receive periodic damage using the
`RadSiteWarhead` warhead type. Units with `ImmuneToRadiation=yes` (TechnoTypeClass offset
0xD37) are skipped.

### 1.10 CellClass Radiation Fields

| Offset | Type   | Name     | Description |
|--------|--------|----------|-------------|
| 0xF0   | double | RadLevel | Current radiation intensity in this cell |
| 0xF8   | ptr    | RadSite  | Pointer to the RadSiteClass affecting this cell |

### 1.11 Visual Effects

The radiation glow is implemented via `LightSourceClass`:
- Created during RadSite activation at the center cell coordinates
- Color comes from `RadColor` INI key, scaled by `RadTintFactor`
- Intensity computed from `RadLevel * RadLightFactor`
- Both intensity and tint fade linearly over the RadSite's lifetime
- Light updates happen every `RadLightDelay` frames
- The animation played is `EMPulseSparkles` from `RulesClass + 0x17F4` (also used for
  EMP sparkle effects)

### 1.12 Global Data

| Address    | Description |
|------------|-------------|
| 0x00B04BD0 | RadSiteClass DynamicVectorClass vtable pointer |
| 0x00B04BD4 | RadSiteClass array data pointer |
| 0x00B04BD8 | RadSiteClass array capacity |
| 0x00B04BDD | RadSiteClass array growth flag |
| 0x00B04BE0 | RadSiteClass array count |
| 0x00B04BE4 | RadSiteClass array growth increment |

---

## Part 2: EMP System

### Overview

The EMP system has two separate mechanisms:
1. **EMPulseClass** — area-effect EMP from the EMPulse Cannon superweapon (case 3 in
   `SuperClass::Launch`)
2. **WarheadTypeClass::EMEffect** — per-warhead flag that applies EMP on detonation

Both ultimately set `TechnoClass::EMPLockRemaining` on affected units, which disables them
for a duration.

### 2.1 EMPulseClass Struct Layout

**Size: 0x34 bytes** (AbstractClass-derived, small struct).

| Offset | Type  | Name           | Notes |
|--------|-------|----------------|-------|
| 0x00   | ptr   | vtable         | Points to vtable__EMPulseClass |
| 0x04   | ptr   | vtable_secondary_4 | INoticeSink |
| 0x08   | ptr   | vtable_secondary_8 | |
| 0x0C   | ptr   | vtable_secondary_C | |
| 0x10–0x23 | ... | AbstractClass base | |
| 0x24   | short | CellX          | Target cell X |
| 0x26   | short | CellY          | Target cell Y |
| 0x28   | int   | Range          | Effect range in cells |
| 0x2C   | int   | StartFrame     | g_CurrentFrameCounter at creation |
| 0x30   | int   | Duration       | Duration in frames |

**Constructor:** 0x004C52B0
**Destructor:** 0x004C5370 (also handles removal from global vector)

### 2.2 WarheadTypeClass EMEffect Flag

| Key        | Type | Offset | Description |
|------------|------|--------|-------------|
| `EMEffect` | bool | 0x154  | If true, detonation triggers EMP on the target |

Read at **0x0075D7B8** in `WarheadTypeClass::ReadINI`.

### 2.3 EMPulse Application Logic (`0x004C54E0`)

Called from `EMPulseClass::Constructor` immediately upon creation. This is the function that
actually disables units and buildings.

**Two loops:**

#### Loop 1: Iterate all Technos (non-building mobile objects)
```
for each Techno in global Techno array:
    if (techno.IsAlive && !techno.InLimbo && !techno.IsCrashing && techno.Health > 0):
        distance = CoordStruct::Distance3D(techno.coords, emp_center_leptons);
        if (distance < Range * 256):
            techno->vtable[0x3DC](duration);  // FootClass::ReceiveEMP
```

#### Loop 2: Iterate cells in range (for buildings)
```
for y = (CellY - Range) to (CellY + Range):
    for x = (CellX - Range) to (CellX + Range):
        if (x*x + y*y <= Range*Range):   // circular check
            cell = MapClass::Get_CellClass(x, y);
            building = LookUpBuildingInCell(cell);

            if (building != null):
                if (building is at foundation origin cell):
                    if (!building.Type.ImmuneToRadiation):     // offset 0x1701 in BuildingTypeClass
                        BuildingClass::ApplyOfflineEffects();  // 0x00452480
                        building.EMPLockRemaining = duration;
                        if (building.Type has radar):          // offset 0x16A4
                            house.NeedsRadarRecalc = true;
            else:
                // Find nearest non-building Techno in cell
                techno = CellClass::FindNearestObject();
                if (techno is Foot/Vehicle && techno.HasLocomotor):
                    if (techno.Locomotor is NOT immune):
                        // Stop locomotor
                        techno.Locomotor->Stop();
                        if (techno.Locomotor.CanTurnWhileStopped):
                            techno.Locomotor->StopMoving();
                        techno.EMPLockRemaining = duration;
                        // Create sparkle animation
                        anim = new AnimClass(RulesClass->EMPulseSparkles, ...);
                        anim.SetOwner(techno);
```

### 2.4 What EMP Disables

#### For Buildings (`BuildingClass::ApplyOfflineEffects` at 0x00452480):
- `StuffEnabled` flag set to false (disables production, power contribution, etc.)
- If building has a LightSource (e.g., spotlight), turns it off
- If building has active building anims that are power-dependent, removes them
- If building has sensor capability (`Type + 0xCD1`), sensor is deactivated
- If building is a wall, wall connections are recalculated (broken appearance)
- Radar buildings trigger house radar power state update

#### For FootClass units (vtable 0x3DC → `FUN_004DEBB0` at 0x004DEBB0):
- Plays "EMP hit" voice/EVA event (event 0x26)
- Plays "unit disabled" voice (event 0x29)
- Calls `FootClass::StopLocomotor()` via vtable 0xE0 with the duration
- Sets unit mission to Guard/Stop (mission 3) via vtable 0x274
- Clears current orders via vtable 0x3A0
- **Recursively EMPs all passengers** via `FUN_00707CB0` at 0x00707CB0:
  - Iterates passenger list at offset 0x118
  - For each passenger: recursively apply EMP, stop locomotor, clear facing
- Sets random rocking angles (cosmetic wobble when disabled)

### 2.5 TechnoClass::EMPLockRemaining

| Offset | Type | Field             |
|--------|------|-------------------|
| 0x504  | int  | EMPLockRemaining  |

**`TechnoClass::IsUnderEMP`** at **0x0070EFD0**:
```c
bool TechnoClass::IsUnderEMP() {
    return this->EMPLockRemaining > 0;  // offset 0x504
}
```

This is checked by various systems to prevent actions while EMP'd.

### 2.6 EMP Recovery (`TechnoClass::AI_Update` at 0x006F9E50)

At the very end of `TechnoClass::AI_Update` (near line 600+):

```c
if (EMPLockRemaining > 0) {
    EMPLockRemaining--;
    if (EMPLockRemaining == 0) {
        // EMP has worn off — recover
        int whatAmI = this->WhatAmI();  // vtable 0x2C

        if (whatAmI == 6) {  // Building
            if (!this->Type->ImmuneToRadiation) {  // type offset 0x1701
                BuildingClass::RestoreOnlineEffects();  // 0x00452410
                if (this->Type has radar) {             // type offset 0x16A4
                    house.NeedsRadarRecalc = true;
                }
            }
        } else if (this is FootClass) {
            // Restart locomotor
            if (this->Locomotor != null) {
                this->Locomotor->Unlock();    // vtable 0x58
            }
            // Clear any EMP sparkle animations
            for each anim in global anim array:
                if (anim.Owner == this && anim.Type == RulesClass->EMPulseSparkles):
                    anim.Invisible = false;  // allow it to finish
        }
    }
}
```

#### Building Recovery (`FUN_00452410` at 0x00452410):
- Sets `StuffEnabled` back to true (offset 0x6EA)
- Restores production animation if building was producing
- Reconnects walls
- Reattaches power-dependent building animations

### 2.7 EMP Immunity

Units/buildings are immune to EMP if:

1. **`ImmuneToRadiation=yes`** on TechnoTypeClass (offset 0xD37) — despite the name,
   this flag also controls EMP immunity for buildings (checked at BuildingTypeClass + 0x1701)
2. **Unit is crashing** (InLimbo or crash state) — skipped during application
3. **Unit is already dead** (Health <= 0) — skipped
4. **Building origin cell check** — EMP only applies to buildings at their foundation
   origin cell to prevent double-application

### 2.8 EMPulse Superweapon (SuperClass::Launch case 3)

The EMPulse Cannon superweapon:
1. Stores target cell at `SuperClass + 0x62`
2. Plays targeting animation
3. Calls `SuperClass::Activate` which creates the cannon-firing anim
   (`RulesClass + 0x330` = animation type)
4. The actual EMPulseClass is created when the superweapon animation completes,
   passing the cell target, range (from `SuperWeaponTypeClass::Range` at offset 0xF8),
   and duration

### 2.9 EMPulse Cleanup (`0x004C54A0`)

Iterates the global EMPulseClass array in reverse:
```c
for (i = count - 1; i >= 0; i--) {
    emp = array[i];
    if (emp->StartFrame + emp->Duration <= currentFrame) {
        emp->~EMPulseClass(1);  // destroy via vtable 0x20
    }
}
```

### 2.10 Rules.ini Keys (Section: `[SpecialWeapons]`)

Read by function at **0x00668FB0**.

| Key                | Type   | RulesClass Offset | Description |
|--------------------|--------|-------------------|-------------|
| `EMPulseWarhead`   | string | 0x0FA0            | Warhead used for EMP projectile |
| `EMPulseProjectile`| string | 0x0FA4            | Projectile type for EMP |

### 2.11 Rules.ini Keys (Section: `[General]`)

| Key              | Type | RulesClass Offset | Description |
|------------------|------|-------------------|-------------|
| `EMPulseSparkles`| anim | 0x17F4            | Animation played on EMP'd units |

### 2.12 Global Data

| Address    | Description |
|------------|-------------|
| 0x008A3870 | EMPulseClass DynamicVectorClass vtable pointer |
| 0x008A3874 | EMPulseClass array data pointer |
| 0x008A3878 | EMPulseClass array capacity |
| 0x008A387D | EMPulseClass array growth flag |
| 0x008A3880 | EMPulseClass array count |
| 0x008A3884 | EMPulseClass array growth increment |

---

## Key Function Address Summary

### Radiation System
| Address    | Function |
|------------|----------|
| 0x0065B1E0 | RadSiteClass::Constructor |
| 0x0065B2F0 | RadSiteClass::Destructor |
| 0x0065B3A0 | RadSiteClass::GetSize (returns 0x74) |
| 0x0065B3D0 | RadSiteClass::Load (save/load) |
| 0x0065B450 | RadSiteClass::Save |
| 0x0065B470 | RadSiteClass::GetClassID (RTTI) |
| 0x0065B4C0 | RadSiteClass::SetCell |
| 0x0065B4D0 | RadSiteClass::SetSpread |
| 0x0065B4F0 | RadSiteClass::SetRadLevel |
| 0x0065B510 | RadSiteClass::GetCurrentRadLevel |
| 0x0065B530 | RadSiteClass::AddRadLevel (augment existing) |
| 0x0065B580 | RadSiteClass::Activate (create light, set cell levels) |
| 0x0065B800 | RadSiteClass::AI (per-tick update, vtable 0x5C) |
| 0x0065B9C0 | RadSiteClass::SetCellRadLevels (initial setup) |
| 0x0065BB50 | RadSiteClass::DecreaseCellRadLevels (used by AddRadLevel) |
| 0x0065BD00 | RadSiteClass::ApplyRadDamage (per-tick cell decay) |
| 0x0066CF70 | RulesClass::ReadRadiation |
| 0x00487C70 | CellClass::SetRadSite |
| 0x00487C80 | CellClass::GetRadSite |
| 0x00487CE0 | CellClass::IncreaseRadLevel |
| 0x00487D00 | CellClass::DecreaseRadLevel |

### EMP System
| Address    | Function |
|------------|----------|
| 0x004C52B0 | EMPulseClass::Constructor (with params) |
| 0x004C5370 | EMPulseClass::Constructor (default/load) |
| 0x004C5470 | EMPulseClass::DeleteAll |
| 0x004C54A0 | EMPulseClass::UpdateAll (cleanup expired) |
| 0x004C54E0 | EMPulseClass::Apply (apply EMP to all objects in range) |
| 0x004DEBB0 | FootClass::ReceiveEMP (vtable 0x3DC) |
| 0x00707CB0 | FootClass::EMPPassengers (recursive passenger EMP) |
| 0x0070EFD0 | TechnoClass::IsUnderEMP |
| 0x00452480 | BuildingClass::ApplyOfflineEffects |
| 0x00452410 | BuildingClass::RestoreOnlineEffects |
| 0x006F9E50 | TechnoClass::AI_Update (contains EMP recovery at end) |
| 0x006CC390 | SuperClass::Launch (case 3 = EMPulse superweapon) |
| 0x00668FB0 | RulesClass::ReadSpecialWeapons |
