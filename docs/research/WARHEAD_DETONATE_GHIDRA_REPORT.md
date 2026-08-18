# Warhead Detonation System — Ghidra Research Report

**Functions analyzed:**
- `WarheadTypeClass::Detonate` at `0x004690B0` (~4692 bytes, 543 decompiled lines)
- `Apply_area_damage` at `0x00489280` (~4224 bytes, 529 decompiled lines)
- `Warhead__SelectExplosionAnim` at `0x0048A4F0` (corrected 2026-05-28: was `FUN_0048a4f0`; binary label confirmed via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT)
- `FUN_0048a620` (Spawn combat light) at `0x0048A620`
- `BulletClass__SpawnShrapnel` at `0x0046A310` (corrected 2026-05-28: was "Screen shake / anim spawning"; binary label confirmed via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT)
- `NukeMaker__SpawnDownwardNuke` at `0x0046B310` (corrected 2026-05-28: was "Nuke launch handler"; binary label confirmed via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT)

**Confidence:** HIGH — all offsets verified from ReadINI string xrefs and constructor defaults.

---

## 1. Parameters

`WarheadTypeClass::Detonate` is a **thiscall** on BulletClass (`this` = BulletClass*).
The second parameter is a CoordStruct* (impact coordinates).

### BulletClass struct offsets (param_1 is `int*`, multiply index by 4)

| Index | Byte Offset | Field | Type |
|-------|------------|-------|------|
| 0x1b  | 0x6C  | Strength (damage value) | int |
| 0x24  | 0x90  | IsAlive | bool (char) |
| 0x27  | 0x9C  | Location.X | int |
| 0x28  | 0xA0  | Location.Y | int |
| 0x29  | 0xA4  | Location.Z | int |
| 0x2b  | 0xAC  | BulletTypeClass* | ptr |
| 0x2c  | 0xB0  | Owner/Firer (TechnoClass*) | ptr |
| 0x38  | 0xE0  | Bright flag | bool (char) |
| 0x43  | 0x10C | Target (AbstractClass*) | ptr |
| 0x4a  | 0x128 | WarheadTypeClass* | ptr |
| 0x4c  | 0x130 | WeaponTypeClass* | ptr |
| 0x54  | 0x150 | ? (used in mind control speed calc) | int |

---

## 2. WarheadTypeClass Struct Offsets

Mapped from `WarheadTypeClass::ReadINI` at `0x0075D590` and the parent ReadINI preamble
(disassembly around `0x0075D3C0`). Constructor at `0x0075CEC0` confirms defaults.

### Core combat fields

| Byte Offset | INI Key | Type | Default | Notes |
|-------------|---------|------|---------|-------|
| 0x98  | Deform | double | 1.0 | Ground deformation amount |
| 0xA0  | Verses (11 doubles) | double[11] | 1.0 each | Armor type damage multipliers (100%=1.0). Parsed as comma-separated `%` or float values at offsets 0xA0..0xF0 |
| 0xF8  | ProneDamage | double | 0.0 | Damage multiplier for prone infantry |
| 0x100 | DeformThreshold | int | 0 | Damage threshold before terrain deforms |
| 0x104 | AnimList (DynamicVectorClass) | vector | empty | Animation types to spawn on impact |
| 0x114 | AnimList count sub-field | int | — | Number of entries for anim lookup |
| 0x120 | InfDeath | int | 0 | Infantry death anim index |
| 0x124 | CellSpread | float | 0.0 | Splash damage radius in cells |
| 0x128 | CellInset | float | 0.0 | ? |
| 0x12C | PercentAtMax | float | 0.0 | Damage percentage at max spread distance |
| 0x130 | CausesDelayKill | bool | false | |
| 0x134 | DelayKillFrames | int | 0 | |
| 0x138 | DelayKillAtMax | float | 0.0 | |
| 0x13C | CombatLightSize | float | 0.0 | |
| 0x140 | Particle system pointer | int | 0 | ParticleSystemType* for warhead |

### Boolean flags (1 byte each)

| Byte Offset | INI Key | Default |
|-------------|---------|---------|
| 0x144 | Conventional | false |
| 0x145 | WallAbsoluteDestroyer | false |
| 0x146 | PenetratesBunker | false |
| 0x147 | Wood | false |
| 0x148 | unknown | false |
| 0x149 | OrganicImmune (auto-set) | false — set to 1 if Verses[2]==0 && Verses[4]==0 |
| 0x14A | ? | false |
| 0x14B | Sonic | false |
| 0x14C | Fire | false |
| 0x14D | ? | false |
| 0x14E | Rocker | false |
| 0x14F | DirectRocker | false |
| 0x150 | Bright | false |
| 0x151 | CLDisableRed | false |
| 0x152 | CLDisableGreen | false |
| 0x153 | CLDisableBlue | false |
| 0x154 | EMEffect | false |
| 0x155 | MindControl | false |
| 0x156 | Poison | false |
| 0x157 | IvanBomb | false |
| 0x158 | ElectricAssault | false |
| 0x159 | Parasite | false |
| 0x15A | Temporal | false |
| 0x15B | IsLocomotor | false |

### Locomotor & special fields

| Byte Offset | INI Key | Type |
|-------------|---------|------|
| 0x15C | Locomotor (CLSID, 16 bytes) | GUID |
| 0x16C | Airstrike | bool |
| 0x16D | Psychedelic | bool |
| 0x16E | BombDisarm | bool |
| 0x170 | Paralyzes | int |
| 0x174 | Culling | bool |
| 0x175 | MakesDisguise | bool |
| 0x176 | NukeMaker | bool |
| 0x177 | Radiation | bool |
| 0x178 | PsychicDamage | bool |
| 0x179 | AffectsAllies | bool (default=true) |
| 0x17A | Bullets | bool |
| 0x17B | Veinhole | bool |

### Screen shake

| Byte Offset | INI Key | Type |
|-------------|---------|------|
| 0x17C | ShakeXlo | int |
| 0x180 | ShakeXhi | int |
| 0x184 | ShakeYlo | int |
| 0x188 | ShakeYhi | int |

### Debris system

| Byte Offset | INI Key | Type |
|-------------|---------|------|
| 0x18C | DebrisTypes (DynamicVectorClass) | vector of VoxelAnimType* |
| 0x190 | DebrisTypes.data | ptr |
| 0x19C | DebrisTypes.count | int | (corrected 2026-05-28: was 0x194; binary uses `*(int*)(warhead+0x19C)` for count comparison in decompile_function 0x004690B0 — ROOT_CAUSE: OFFSET_RETYPED_WRONG)
| 0x1A8 | DebrisMaximums (DynamicVectorClass) | vector of int |
| 0x1AC | DebrisMaximums.data | ptr |
| 0x1B0 | DebrisMaximums.count | int |
| 0x1C4 | MaxDebris | int |
| 0x1C8 | MinDebris | int (clamped >= 0; MaxDebris clamped >= MinDebris) |

---

## 3. Flow Overview — WarheadTypeClass::Detonate (0x004690B0)

The function executes in this order:

### Step 1: Screen Shake
```
if (warhead->ShakeXlo != 0 || warhead->ShakeXhi != 0)
    g_ShakeX = Random(ShakeXlo, ShakeXhi);
if (warhead->ShakeYlo != 0 || warhead->ShakeYhi != 0)
    g_ShakeY = Random(ShakeYlo, ShakeYhi);
```
Globals `DAT_0087f7ec` (ShakeX) and `DAT_0087f7f0` (ShakeY) are set from randomized
ranges.

### Step 2: Radiation Site Creation
```
if (weaponType != NULL && weaponType->RadLevel > 0) {
    cell = Map.GetCellAt(impactCoords);
    existing = FindRadSite(cell);
    if (existing == NULL) {
        site = new RadSiteClass();
        // Initialize and activate rad site
    } else {
        // Boost existing rad site
    }
}
```
Radiation is handled via `WeaponTypeClass`, not the warhead directly. A `RadSiteClass`
(size 0x74) is created at the impact cell. If one already exists, it is boosted
(`FUN_0065b530`).

### Step 3: Special Warhead Type Dispatch

The function checks warhead boolean flags in a cascading if/else chain. Only ONE
special type is processed per detonation — they are mutually exclusive:

```
if (warhead->MindControl)       → mind control logic
else if (warhead->IvanBomb)     → ivan bomb attach
else if (warhead->ElectricAssault) → electric assault
else if (warhead->Parasite)     → parasite attach
else if (warhead->Temporal)     → temporal warp erase
else if (warhead->IsLocomotor)  → locomotor hijack
else if (warhead->Airstrike)    → airstrike marker
else if (warhead->BombDisarm)   → ivan bomb defusal
else if (warhead->MakesDisguise) → disguise application
else if (warhead->NukeMaker)    → nuke launch
else {
    // Normal warhead — apply area damage
    if (bulletType->HasShakeAnim)
        BulletClass__SpawnShrapnel();  // 0x0046A310 — corrected 2026-05-28: was FUN_0046a310 "screen shake + anim"; binary label is BulletClass__SpawnShrapnel
    Apply_area_damage(...);
}
```

### Step 4: Post-Detonation Effects

After the special-type dispatch, regardless of path:

1. **Impact coordinates adjustment** — if `bulletType->Airburst` is set, coordinates
   are randomized via `FUN_0049f420`.

2. **Crater/impact animation selection** (`FUN_0048a4f0`) — selects an animation based on:
   - Damage amount (indexes into AnimList by damage/25)
   - Whether impact is on a bridge (uses bridge crater anims from Rules)
   - Special warhead (e.g., `g_RulesClass + 0x17B4` check = IonCannonWarhead)
   - EMEffect warheads get random anim from their AnimList

3. **Terrain deformation** (`FUN_0048a620`) — spawns a SmudgeClass (crater) if:
   - Enough frames have elapsed (`DAT_00a8eb78` check)
   - `warhead->Bright` is true OR damage > 0
   - Crater size = `(damage << 6) >> 8`, clamped to [0x15, 0x3F]
   - If `warhead->CombatLightSize > 0`, uses that for size instead
   - Flags for CLDisableRed/Green/Blue are OR'd into the smudge flags

4. **Impact animation construction** — `AnimClass` is constructed using the selected
   crater anim, with owner facing direction from `FUN_0048ace0`.

5. **Nuke flash check** — if `warhead == Rules->NukeWarhead` (offset 0xF8C in RulesClass),
   a special flash/whiteout effect triggers (`FUN_004251f0`).

### Step 5: Debris Spawning

```
debrisCount = Random(MinDebris, MaxDebris - 1);
if (DebrisTypes.count > 0) {
    // VoxelAnimClass debris (typed)
    typeIndex = 0;
    while (debrisCount > 0) {
        maxForType = DebrisMaximums[typeIndex];
        spawnCount = Random(0...) % (maxForType + 1);
        spawnCount = min(spawnCount, debrisCount);
        for each piece:
            new VoxelAnimClass(DebrisTypes[typeIndex], impactCoords);
        debrisCount -= spawnCount;
        typeIndex = (typeIndex + 1) % DebrisTypes.count;
    }
} else if (debrisCount > 0) {
    // Generic debris (from Rules->MetallicDebris list)
    for each piece:
        coords = GetRandomCoords();  // via vtable 0x48
        coords.Z += 20;  // slight vertical offset
        animIndex = Random(0, Rules->MetallicDebrisCount - 1);
        new AnimClass(Rules->MetallicDebris[animIndex], coords);
}
```

Key details:
- `DebrisTypes` are `VoxelAnimTypeClass*` entries — they spawn `VoxelAnimClass` (size 0x148)
- When no DebrisTypes are specified, generic `AnimClass` debris (size 0x1C8) is spawned
  from `Rules->MetallicDebris` (offset 0x140 in RulesClass, count at 0x14C)
- Each generic debris piece gets Z offset +0x14 (20 leptons)
- Debris pieces rotate across DebrisTypes in round-robin when there are multiple types

### Step 6: Particle System Spawning (BulletTypeClass driven)

If `bulletType->HasTrail` (offset 0x294):
```
trailAnim = bulletType->TrailAnimType (offset 0x2B0);
speed = trailAnim->speed;
facing = bullet->GetFacing();
for i in 0..8:
    // Create DirectX particle via CoCreateInstance
    // with random spherical velocity based on speed/10
    CoCreateInstance(CLSID_..., IID_..., &particle);
    FUN_004664c0(trailAnim, facing, ...);
    // Set velocity from trig: sin/cos with random angle
```
Uses `CoCreateInstance` with a specific CLSID (at `DAT_007e96e0`) to create 8 + 1
particle effects with randomized spherical velocity vectors.

---

## 4. Apply_area_damage (0x00489280) — Splash Damage Distribution

### Signature
```c
bool __fastcall Apply_area_damage(
    int* impactCoords,    // ECX - CoordStruct*
    int  damage,          // EDX - base damage
    int* sourceObj,       // stack - attacker TechnoClass*
    int  warheadType,     // stack - WarheadTypeClass*
    char destroyTiberium, // stack - whether to destroy tiberium
    int  ownerHouse       // stack - HouseClass* of attacker
);
```

Returns: `true` (1) normally, `2` if a building was captured/mind-controlled (bVar5 case).

### Step 1: Early Exit
If damage==0, game session flags prevent damage (`*DAT_00a8b230 & 0x20`), or warheadType==0,
returns true immediately.

### Step 2: Compute Spread Radius
```c
spreadRadius = ftol(warhead->CellSpread);  // offset 0x124, float → int cells
```
The `CellSpread` value (float at WarheadType+0x124) is converted to an integer cell count.

Check if this is the `C4Warhead` (special instant-kill):
```c
bVar21 = (warheadType == Rules->C4Warhead);  // Rules + 0xFAC
```

### Step 3: Convert Impact to Cell Coordinates
```c
cellX = (impactCoords->X + (sign >> 8)) >> 8;  // lepton-to-cell
cellY = (impactCoords->Y + (sign >> 8)) >> 8;
impactCell = Map.GetCellAt(cellX, cellY);
```

### Step 4: Check for Elevated Targets (aircraft/jumping)
```c
canHitAir = (warhead->CellSpread > 0.0);  // if spread > 0, set flag
// Center point for damage calc:
centerX = cellX * 256 + 128;  // cell center in leptons
centerY = cellY * 256 + 128;
centerZ = 0;

groundHeight = impactCell->GetGroundHeight();
if (groundHeight < impactCoords->Z) {
    // Impact is above ground — scan for airborne targets
    // Uses FUN_00412b40 to find objects in cell at altitude
    // Iterates linked list from FUN_004137a0
    for each airborne object:
        if (object->IsAlive && object->IsOnMap && object->Health > 0):
            dist = Distance3D(object->coords, impactCenter);
            if (dist <= spreadRadius):
                add to target list with distance
                // Also check for building capture (dist < 0x55 = 85 leptons)
}
```

### Step 5: Bridge Check
```c
if (impactCell->Flags & 0x100) {  // cell has bridge
    bridgeGroundHeight = impactCell->GetGroundHeight();
    if (bridgeGroundHeight + BridgeHeight/2 < impactCoords->Z) {
        // Impact is above bridge — search upper cell layer
        searchAboveBridge = true;
    }
}
```

### Step 6: Iterate Cells in Spread Radius
```c
spreadCells = ftol(warhead->CellSpread);
for cellIndex = 0; cellIndex < CellSpreadTable[spreadCells]; cellIndex++ {
    offsetX = CellSpreadOffsetX[cellIndex] + cellX;
    offsetY = CellSpreadOffsetY[cellIndex] + cellY;
    currentCell = Map.GetCellAt(offsetX, offsetY);
```
`CellSpreadTable` at `DAT_007ed3d0` contains the number of cells to check for each
integer spread radius. `CellSpreadOffsetX/Y` at `DAT_00abd490/00abd492` are pre-computed
offset tables.

### Step 6a: Overlay/Tiberium Destruction
Within each cell in range:
```c
overlayType = currentCell->OverlayTypeIndex;
if (overlayType != -1) {
    typeEntry = OverlayTypes[overlayType];
    if (typeEntry->IsTiberium) {
        if (!typeEntry->IsVein || warhead->Wood) {  // "Wood" flag = can destroy veins
            if (destroyTiberium)
                CellClass::Reduce_Tiberium();
        }
    }
    if (typeEntry->IsWall) {
        if (warhead->WallAbsoluteDestroyer || warhead->Conventional
            || (warhead->Wood && overlayType.material == 6)) {
            CellClass::DestroyOverlay();
        }
    }
}
```

### Step 6b: Collect Targets in Each Cell
Objects are found via the cell's occupant linked list:
```c
// If searching above bridge, use cell->AltObjectList (offset 0xE8)
// Otherwise use cell->ObjectList (offset 0xE4)
objectList = searchAboveBridge ? currentCell[0x3a] : currentCell[0x39];

for each object in linked list (next at object[0xC]):
    // Skip the source if it's not C4Warhead and source has SelfHealing
    if (object == sourceObj && !source->TypeClass->SelfHeal_C && !bVar21)
        continue;
    if (!object->IsAlive) continue;

    // Check building immune list
    whatAmI = object->WhatAmI();
    if (whatAmI == 1 /*Building*/ && (gameFlags & 0x800)) {
        typeIndex = object->GetTypeIndex();
        // Check against Rules->ImmuneToRadiation list
        for i in Rules->ImmuneList:
            if match: skip this object
    }

    // Calculate distance from impact
    if (whatAmI == 6 /*Building overlay?*/) {
        // Special distance calc for buildings not in center cell
        if (cellIndex != 0) {
            centerCoords = currentCell->GetCenterCoords();
            dist = Distance3D(centerCoords - impactCoords);
        } else {
            dist = 0;  // Building in impact cell takes full damage
        }
    } else {
        objectCenter = object->GetCenterCoords();
        dist = Distance3D(objectCenter - impactCoords);
    }

    // For buildings on bridges, check altitude difference
    if (whatAmI == 6 && cellIndex != 0) {
        if (BridgeHeight * 2 < impactZ - buildingZ) {
            // Recalculate distance excluding bridge
            dist = dist + BridgeHeight * -2;
        }
    }

    // Check capture proximity (< 0x55 = 85 leptons)
    if (canHitAir && cellIndex == 0 && object is TechnoClass) {
        if (object->CanBeCommanded() && object->ParasiteCount == 0 && dist < 0x55)
            bVar5 = true;  // potential capture
    }

    add {object, distance} to target list;
```

### Step 7: Apply Damage to Each Target
```c
for each {object, distance} in target list:
    if (!object->IsAlive) continue;
    whatAmI = object->WhatAmI();
    if (whatAmI == 6 && typeClass->Intangible) continue;

    if (bVar5) {
        // Capture mode: only damage TechnoClass objects that can be captured
        if (object is not TechnoClass || !object->CanBeCommanded())
            continue;  // skip non-capturable
    }

    // Half damage for aircraft (WhatAmI == 2) if InAir
    if (whatAmI == 2 && object->IsInAir())
        distance /= 2;

    // Apply damage if: alive, on map, not intangible, within radius
    if (object->Health > 0 && object->IsOnMap
        && !object->IsIntangible && distance <= spreadRadius) {
        object->ReceiveDamage(&damage, distance, warheadType, sourceObj, 0, 0, ownerHouse);
    }
```

### Step 8: Damage Falloff (inside ReceiveDamage, not here)

The `distance` parameter passed to `ReceiveDamage` is the raw lepton distance from impact
to target. **The falloff calculation happens inside ReceiveDamage itself**, not in
Apply_area_damage. Apply_area_damage's job is just to collect targets and measure distances.

The formula (from ReceiveDamage, vtable offset 0x16C) uses:
- `warhead->CellSpread` — max radius
- `warhead->PercentAtMax` — minimum damage percentage at edge of spread
- Linear interpolation between 100% at center and PercentAtMax at edge

### Step 9: Knockback / Rocker Override
```c
// (corrected 2026-05-28: gate is Rocker (0x14E), NOT IsLocomotor (0x15B); binary: `*(char*)(param_4+0x14e)` via decompile_function 0x00489280 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
if (warhead->Rocker && CellSpread > 0.0) {
    // Iterate 7x7 cell grid around impact (-3..+3 in X and Y)
    for dx = -3 to +3:
        for dy = -3 to +3:
            cell = Map.GetCellAt(cellX+dx, cellY+dy);
            objects = searchAboveBridge ? cell->AltObjectList : cell->ObjectList;
            for each object in cell:
                if (dx == cellX && dy == cellY && sourceObj != NULL):
                    // Object in center cell: push away from attacker
                    dir = normalize(object->coords - source->coords);
                    velocity = dir * KNOCKBACK_SPEED;
                    object->ApplyLocomotor(&newCoords, speed);
                else if (CellSpread > 0.0):
                    // Object in outer cells: push away from impact
                    object->ApplyLocomotor(impactCell, speed);
```
The knockback speed is `(float)uStack_cc * 0.01` (the cell damage count scaled),
clamped to max 4.0. `ApplyLocomotor` is vtable offset 0x3D8.

### Step 10: Terrain Effects

#### Bridge Damage
```c
// Check for pavement/road tiles
iVar19 = (tile - PavementTileBase) + 1;
// Check against BridgeTilesets (DAT_00abad30, DAT_00aa1028)
// If it's a bridge tile AND warhead->Conventional:
if (warhead->Conventional) {
    // Probabilistic destruction:
    if (warhead == Rules->C4Warhead || Random(1, Rules->BridgeStrength) < damageCount) {
        success = ApplyDamageToCell();  // tries up to 4 times
        if (success) TechnoClass::StopAllTargeting();
    }
}

// Low bridge overlay (overlayIndex 0x4A..0x63 = indices 74..99)
if (overlayIndex > 0x49 && overlayIndex < 100) {
    if (warhead == C4Warhead || Random(1, BridgeStrength) < damageCount)
        DestroyBridge_Low();

// High bridge overlay (overlayIndex 0xCD..0xE6 = indices 205..230)
if (overlayIndex > 0xCC && overlayIndex < 0xE7) {
    if (warhead == C4Warhead || Random(1, BridgeStrength) < damageCount)
        DestroyBridge_High();
```
`Rules->BridgeStrength` is at `g_RulesClass_Instance + 0x1740`.

#### Fence/Wall Overlay Destruction
```c
if (cell->OverlayTypeIndex != -1) {
    overlayType = OverlayTypes[cell->OverlayTypeIndex];
    if (overlayType->IsARock) {  // offset 0x2B0 in OverlayTypeClass
        FUN_00486e70();  // remove overlay
        cell->OverlayTypeIndex = -1;
        CellClass::RecalcAttributes();
        // Spawn explosion anim from Rules (offset 0x54)
        new AnimClass(Rules->BarrelExplode, cellCoords);
        // Recursive area damage! Uses Rules->BarrelDamage warhead
        Apply_area_damage(cellCoords, Rules->BarrelDamage, 1, ownerHouse);
        // Random voxel debris
        for each Rules->BarrelDebris:
            if (Random(0,99) < 15)
                new VoxelAnimClass(BarrelDebris[i], cellCoords);
                break;  // only one
        // Random particle system (25% chance)
        if (Random(0,99) < 25)
            new ParticleSystemClass(Rules->BarrelParticle, cellCoords);
    }
}
```
**Note:** Barrel/rock overlays trigger recursive `Apply_area_damage` — chain reactions
are possible.

#### Warhead Particle System
```c
if (warhead->ParticleSystem != 0) {  // offset 0x140
    new ParticleSystemClass(warhead->ParticleSystem, cellCoords, ownerHouse);
}
```

### Return Value
- Returns `true` (1) normally = "no special capture"
- Returns `2` if a building was in the capture zone (bVar5) — this is checked by
  the caller to trigger the "building captured" flash anim
- Returns `true` (1) normally (when no special capture occurred) — this is `!bVar5` in the binary (corrected 2026-05-28: was "returns false (0) if target list empty"; binary returns `!bVar5` = true on normal path, never 0 for empty list; via decompile_function 0x00489280 — ROOT_CAUSE: INFERENCE_HARDENED)

---

## 5. Special Warhead Types — Detailed

### Mind Control (offset 0x155)
```c
target = bullet->Target;  // offset 0x10C
if (target && target->WhatAmI() == 1) {  // target is a Foot/TechnoClass
    attacker = bullet->Owner;
    if (attacker) {
        attackerAsFoot = (attacker->AbstractFlags & 1) ? attacker : NULL;
        if (target && attackerAsFoot && !target->IsMindControlled()) {
            // Calculate push speed for mind control beam visual
            speed = (bullet->field_150 * bullet->Strength >> 8)
                    * Rules->MindControlSpeed / some_const;
            speed = min(speed, 4.0);

            // Direction vector from target to attacker
            dir = normalize(attacker->coords - target->coords);
            // Move target slightly toward attacker
            newCoords = target->coords + dir * KNOCKBACK_CONST;

            target->ApplyLocomotor(&newCoords, speed);  // vtable 0x3D8
            target->MindControlledBy = attacker;  // offset 0x2A8
            attacker->MindControlTarget = target; // offset 0x2A8 on attacker
        }
    }
}
```

### Temporal Warp (offset 0x15A)
```c
if (bullet->Target && bullet->Owner
    && (bullet->Owner->AbstractFlags & 2)  // has temporal capability
    && bullet->Owner->TemporalPtr != 0) {  // offset 0x38
    TemporalClass__InitiateWarp();  // 0x0071af20 — corrected 2026-05-28: was FUN_004389b0 (wrong address — that's BombClass__Defuse); binary: TemporalClass__InitiateWarp confirmed via get_function_by_address 0x0071af20
}
```

### Parasite (offset 0x159)
```c
target = bullet->Target;
attacker = bullet->Owner;
if (target && attacker && (attacker->AbstractFlags & 1)) {
    if (attacker->ParasiteCount != 0 && attacker->WhatAmI() == 1) {
        target->ReceiveParasite();  // vtable 0x3C8
    }
    TemporalClass__InitiateWarp();  // 0x0071af20 — NOTE: corrected 2026-05-28: was labelled "additional parasite logic"; binary shows TemporalClass__InitiateWarp via get_function_by_address — parasite dispatch may call warp-initiation differently; needs re-investigation
}
```

### Ivan Bomb (offset 0x157)
```c
if (target && attacker && (attacker->AbstractFlags & 1)) {
    attackerAsFoot = (attacker->AbstractFlags & 4) ? attacker : NULL;
    if (attackerAsFoot && attackerAsFoot->IsAlive && attackerAsFoot->HasBomb) {
        EVA_Notify(6, attacker, ...);   // EVA speech
        EVA_Notify(0x2C, attacker, ...); // more EVA
    }
    if (successful && Rules->IvanWarningAnim != -1
        && (IsPlayerControlled || bVar5)) {
        // Spawn warning animation at bomb coords
        FUN_007509e0();  // play warning sound/anim
    }
}
```
Actually attaches the bomb via `BombClass__Attach(attacker, target)` — the function at `0x00438170` does not exist as `BombClass::Constructor`; the Detonate decompile calls the already-labeled `BombClass__Attach` directly (corrected 2026-05-28: was `BombClass::Constructor at 0x00438170`; address is wrong and label is wrong; binary Detonate decompile shows `BombClass__Attach` call via decompile_function 0x004690B0 — ROOT_CAUSE: RTTI_LABEL_DRIFT).

### Electric Assault (offset 0x158)
```c
if (target && attacker && (attacker->AbstractFlags & 4)) {
    techno = (attacker->AbstractFlags & 4) ? attacker : NULL;
    if (techno->GetMission() != 2) goto done;  // must be in attack mission
}
// Then checks if target is a Foot/Infantry that can be controlled
if (target->IsDeployable() && !target->IsMindControlled()
    && (target->WhatAmI() == 1 || target->WhatAmI() == 2)
    && !target->IsDisguised
    && target->TypeClass->MaxSpeed < bullet->Strength) {
    TechnoClass::PerformDeploy(bullet->Owner);  // force deploy
}
```

### Bomb Disarm (offset 0x16E)
```c
if (bullet->Owner && (bullet->Owner->AbstractFlags & 2)
    && bullet->Owner->BombPtr != 0) {
    BombClass__Defuse();  // 0x004389b0 — corrected 2026-05-28: was FUN_004389b0; binary label confirmed via get_function_by_address
}
```

### NukeMaker (offset 0x176)
```c
FUN_0046b310();  // launches a superweapon nuke at the impact location
// Resolves NukeType from WeaponType, gets coordinates,
// creates superweapon launch via CoCreateInstance
```

### IsLocomotor (offset 0x15B)
When set, the warhead hijacks the target's locomotor rather than dealing damage.
See Step 9 (Knockback) above. The CLSID at offset 0x15C is applied via vtable 0x3D8.

### Airstrike (offset 0x16C)
```c
if (target && attacker->WhatAmI() == 6 /*Building*/
    && target->WhatAmI() == 0xF /*Aircraft?*/) {
    FUN_00452820();  // trigger airstrike at target
}
```

---

## 6. Special Effects

### Bright Flash
If `bullet->Bright` flag (offset 0xE0) is set, the function takes a different path
after Apply_area_damage. If the result was a "capture" (return 2), it spawns a special
capture animation from `Rules->WarheadCapture` (offset 0x350).

When Bright is set and NOT a capture:
```c
flags = 0;
if (warhead->CLDisableRed) flags |= 2;
if (warhead->CLDisableGreen) flags |= 4;
if (warhead->CLDisableBlue) flags |= 8;
FUN_0048a620(coords, 1, flags);  // spawn combat light with color mask
```

### Combat Light (FUN_0048a620 at 0x0048A620)
Creates a SmudgeClass (size 0x18) representing a temporary light flash:
- Size is calculated from damage: `(damage << 6) >> 8`, clamped to [0x15=21, 0x3F=63]
- If `warhead->CombatLightSize > 0`, uses `ftol(CombatLightSize)` instead
- Color channel disable flags (CLDisableRed/Green/Blue) are OR'd into the effect

### BulletClass__SpawnShrapnel (at 0x0046A310) — previously mislabelled "Screen Shake"
<!-- corrected 2026-05-28: binary label confirmed via get_function_by_address -->
Complex function that:
1. Reads `BulletTypeClass->ShakeIntensity` (offset 0x2B4) and anim settings (0x2B8)
2. If ShakeIntensity < 0, calculates intensity from distance between bullet and target
3. Iterates cells in a spread radius around impact
4. For each cell in range, spawns explosion animations
5. Uses DirectX `CoCreateInstance` to create particle effects at each cell

### Capture Flash Animation
When Apply_area_damage returns 2 (captured building):
```c
animType = Rules->WarheadCapture;  // offset 0x350
if (animType) {
    facing = FUN_0048ace0(x, y, z);
    new AnimClass(animType, &impactCoords, 0, 1, 0x2600, facing, 0);
}
```

---

## 7. Crater/Impact Animation Selection (FUN_0048a4f0)

```c
int SelectCraterAnim(int damage, int warheadType, int landType, CoordStruct* coords) {
    if (damage == 0 || warheadType == 0) return 0;

    // Bridge check
    if (landType == 2) {  // on bridge
        if (!warhead->IsBridge) return ...;  // no bridge crater
        cell = Map.GetCellAt(coords);
        if (!(cell->Flags & 0x100)) {  // not elevated
            groundHeight = cell->GetGroundHeight();
            if (coords->Z < groundHeight + BridgeHeight * 2) {
                // Under bridge — use bridge crater anims
                count = Rules->BridgeCraterCount;  // offset 0xBD0
                index = min(damage, count * 35 - 1);
                return Rules->BridgeCraterAnims[index / 35];  // offset 0xBC4
            }
        }
    }

    // Ion cannon special case
    if (warheadType == Rules->IonCannonWarhead)  // offset 0x17B4
        return Rules->IonCannonCrater;  // offset 0x2F4

    // Normal crater from AnimList
    animCount = warhead->AnimList.Count;  // offset 0x114
    if (animCount > 0) {
        if (warhead->EMEffect) {  // offset 0x154
            // Random anim from list
            index = Random(0, animCount - 1);
            return warhead->AnimList[index];
        } else {
            // Damage-based selection: each anim covers 25 damage
            index = min(damage, animCount * 25 - 1);
            return warhead->AnimList[index / 25];
        }
    }
    return 0;
}
```

**Key insight:** AnimList entries are selected by damage magnitude. Each entry covers
a 25-damage band. Higher damage = later entries in the list = bigger explosions.
EMEffect warheads use random selection instead.

---

## 8. Key Global Addresses

| Address | Description |
|---------|-------------|
| 0x0087f7ec | g_ShakeX — screen shake X offset |
| 0x0087f7f0 | g_ShakeY — screen shake Y offset |
| 0x00a8b230 | Game session flags (bit 0x20 = no damage, 0x800 = building immunity check, 0x8000 = bridge damage) |
| 0x0089e864 | CellHeight — base cell height in leptons |
| 0x0089e870 | BridgeHeight — bridge height in leptons |
| 0x00a83d84 | OverlayTypeClass array base pointer |
| 0x007ed3d0 | CellSpreadTable — cells-to-check count per spread radius |
| 0x00abd490 | CellSpreadOffsetX — X offsets for cell iteration |
| 0x00abd492 | CellSpreadOffsetY — Y offsets for cell iteration |
| 0x00abad30 | BridgeTilesetIndex1 |
| 0x00aa1028 | BridgeTilesetIndex2 |
| 0x00abad1c | PavementTileBase |

---

## 9. Key Functions Called

| Address | Name | Purpose |
|---------|------|---------|
| 0x00489280 | Apply_area_damage | Distributes damage to all objects in CellSpread radius |
| 0x0048a4f0 | SelectCraterAnim | Picks crater/explosion animation based on damage |
| 0x0048a620 | SpawnCombatLight | Creates terrain deformation smudge/light |
| 0x0046a310 | BulletClass__SpawnShrapnel | Spawn shrapnel/explosion anims in radius (corrected 2026-05-28: was "ShakeAndExplode"; binary label confirmed via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x0046b310 | NukeLaunch | Fires nuke superweapon at coordinates |
| 0x0048ace0 | GetImpactFacing | Returns facing direction for anim orientation |
| 0x0065b4c0-0x0065b580 | RadSite init functions | Initialize radiation site properties |
| 0x00487c70 | RegisterRadSite | Add rad site to global tracking |
| 0x00487c80 | FindRadSite | Find existing rad site at cell |
| 0x0065b530 | BoostRadSite | Increase existing rad site strength |
| 0x004389b0 | BombClass__Defuse | Defuse Ivan bomb (corrected 2026-05-28: was "InitiateTemporal"; binary shows BombClass__Defuse via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x00438170 | BombClass::Constructor | Attach Ivan bomb to target |
| 0x00452820 | TriggerAirstrike | Launch airstrike at target |
| 0x0062a980 | PoisonTarget | Apply poison damage to target |
| 0x0071af20 | TemporalClass__InitiateWarp | Initiate temporal warp effect (corrected 2026-05-28: was "ParasiteLogic / Additional parasite attachment"; binary shows TemporalClass__InitiateWarp via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x004664c0 | BulletClass__Init | Initialize bullet/particle effect (corrected 2026-05-28: was "LaunchParticle"; binary shows BulletClass__Init via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x0049f420 | RandomizeCoords | Randomize coordinates for airburst |
| 0x004251f0 | NukeFlashWhiteout | Trigger nuke screen flash |

---

## 10. Summary of Damage Flow

```
BulletClass::Detonate(impactCoords)
  │
  ├─ Screen shake (random ShakeX/Y range)
  ├─ Radiation site creation (if WeaponType->RadLevel > 0)
  │
  ├─ Special warhead check (mutually exclusive):
  │   ├─ MindControl → locomotor push + set MindControlledBy
  │   ├─ IvanBomb → BombClass::Constructor
  │   ├─ ElectricAssault → force deploy
  │   ├─ Parasite → ReceiveParasite
  │   ├─ Temporal → InitiateTemporal
  │   ├─ IsLocomotor → locomotor hijack
  │   ├─ Airstrike → TriggerAirstrike
  │   ├─ BombDisarm → defuse bomb
  │   ├─ MakesDisguise → apply disguise
  │   └─ NukeMaker → NukeLaunch
  │
  ├─ Normal warhead path:
  │   ├─ BulletClass__SpawnShrapnel (if BulletType->HasShakeAnim) — corrected 2026-05-28
  │   └─ Apply_area_damage(coords, damage, source, warhead)
  │       ├─ Collect targets in CellSpread radius
  │       │   ├─ Check airborne targets if impact above ground
  │       │   ├─ Handle bridge layer separation
  │       │   ├─ Iterate CellSpreadTable cells
  │       │   ├─ Destroy tiberium/overlays in range
  │       │   └─ Measure 3D distance per target
  │       ├─ Apply damage to each target via ReceiveDamage
  │       │   └─ Distance passed for falloff calc (linear interpolation)
  │       ├─ Locomotor knockback in 7x7 grid (if IsLocomotor)
  │       ├─ Bridge damage (probabilistic, based on BridgeStrength)
  │       ├─ Barrel chain reactions (recursive Apply_area_damage)
  │       └─ ParticleSystem spawn
  │
  ├─ Select and spawn crater animation
  ├─ Spawn combat light / terrain deformation
  ├─ Spawn debris (VoxelAnim or generic Anim)
  └─ Spawn trail particles (8 with random spherical velocity)
```
