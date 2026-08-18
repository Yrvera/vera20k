# ObjectClass — Ghidra Research Report

**Primary vtable address:** `0x007EF060` (122 entries, 0x1E8 bytes)
**Constructor:** `0x005F3900` (main), `0x005F3B50` (vtables-only, used by Load path)
**Destructor:** `0x005F3B80` (full destructor body — Ghidra mislabels as Constructor; called from derived-class destructors such as `VoxelAnimClass::Destructor`, `OverlayClass::Destructor`, `BuildingLightClass::Destructor`)
**Class size:** `0xAC` bytes (172 decimal)
**Confidence:** HIGH (all offsets verified from constructor assembly + cross-referenced with 10+ existing reports)
**Active in YR:** Yes — this is the core base class for all game objects

## 1. Overview

ObjectClass is the base class for all visible game objects in gamemd.exe. It inherits from
AbstractClass (0x24 bytes) and adds fields for health, position, visibility state, falling
physics, linked-list cell occupancy, bomb attachments, parachutes, and rendering flags.

**Inheritance chain** (base interfaces verified from RTTI ClassHierarchyDescriptor at
`0x00807438` — ObjectClass reports 8 contained bases, AbstractClass reports 7):

```
IUnknown (.?AUIUnknown@@)
  +-- IPersist (.?AUIPersist@@)
        +-- IPersistStream (.?AUIPersistStream@@)
  +-- IRTTITypeInfo (.?AUIRTTITypeInfo@@)

AbstractClass (.?AVAbstractClass@@) inherits (offsets = where each sub-object lives):
  +-- IPersistStream      at offset  0  (primary vtable slots +0x00..+0x1C hold its methods)
  +-- IRTTITypeInfo       at offset +4  (secondary vtable at 0x7EF044)
  +-- INoticeSink         at offset +8  (secondary vtable at 0x7EF03C)
  +-- INoticeSource       at offset +12 (secondary vtable at 0x7EF034)

Derived-class chain (data-size layout):
AbstractClass (0x00-0x23, 36 bytes — includes 4 vtable ptrs at 0x00/0x04/0x08/0x0C)
  +-- ObjectClass (0x24-0xAB, adds 136 bytes, total 172 bytes)
        +-- MissionClass (+0xAC)
              +-- RadioClass
                    +-- TechnoClass (0xF0-0x51F, total 1312 bytes)
                          +-- FootClass, BuildingClass
        +-- AnimClass, OverlayClass, SmudgeClass, TerrainClass,
            ParticleClass, ParticleSystemClass, VoxelAnimClass, WaveClass
```

ObjectClass is the widest-used base — even non-Techno objects (anims, overlays, smudges,
terrain, particles, voxel anims, waves) inherit from it directly.

## 2. Class Layout / Key Offsets

### AbstractClass Base (0x00-0x23)

| Offset | Size | Type | Name | Init Value | Evidence |
|--------|------|------|------|------------|----------|
| 0x00 | 4 | ptr | vtable | ObjectClass primary vtable at `0x7EF060` (122 entries; also holds the IUnknown + IPersistStream slots at +0x00..+0x1C) | Constructor ASM `MOV [ESI], 0x7EF060` |
| 0x04 | 4 | ptr | vtable_IRTTITypeInfo | `0x7EF044` — 6-entry secondary vtable: QI/AddRef/Release thunks + Process/GetID/AssignUniqueID. Ghidra auto-labels the methods `AbstractClass__IRTTITypeInfo_*`. **NOT IPersistStream** — IPersistStream is in the primary vtable. | Constructor ASM |
| 0x08 | 4 | ptr | vtable_INoticeSink | `0x7EF03C` — 1-entry vtable (stub returning 0). Ghidra-inferred name `vtable__INoticeSink`. | Constructor ASM |
| 0x0C | 4 | ptr | vtable_INoticeSource | `0x7EF034` — 1-entry vtable (empty void stub). Ghidra-inferred name `vtable__INoticeSource`. | Constructor ASM |
| 0x10 | 4 | int | UniqueID | -1 (0xFFFFFFFF) | AbstractClass ctor |
| 0x14 | 1 | byte | AbstractFlags | bits 0-2 cleared, then bit 1 set | See below |
| 0x15-0x17 | 3 | — | padding | — | — |
| 0x18 | 4 | int | unknown_0x18 | 0 | AbstractClass ctor |
| 0x1C | 4 | int | RefCount | 0 | AbstractClass ctor |
| 0x20 | 1 | byte | Dirty | 0 (false) | AbstractClass ctor |
| 0x21-0x23 | 3 | — | padding | — | — |

**AbstractFlags bits (offset 0x14)** — all three verified from the derived-class constructors that set them:

- **Bit 0 (0x01): IsTechno** — set in `TechnoClass::Constructor` at `0x006F322F` (`OR AL, 0x1`). Checked in `Select`, `UnInit`, and any path that reads `TechnoTypeClass` fields through the object.
- **Bit 1 (0x02): IsObject** — set in `ObjectClass::Constructor` at `0x005F3B37` (`OR byte [ESI+0x14], 2`). Set immediately even though `InLimbo` is still true, so this is NOT "IsOnMap" — it simply means "this went through ObjectClass ctor".
- **Bit 2 (0x04): IsFoot** — set in `FootClass::Constructor` at `0x004D34DD` (`OR DL, 0x4`). True for FootClass-family (InfantryClass, UnitClass, AircraftClass), false for BuildingClass.

### ObjectClass Fields (0x24-0xAB)

| Offset | Size | Type | Name | Init Value | Evidence |
|--------|------|------|------|------------|----------|
| 0x24 | 4 | int | unknown_0x24 | 0 | Constructor ASM `MOV [ESI+0x24], 0` |
| 0x28 | 4 | int | unknown_0x28 | 0 | Constructor ASM |
| 0x2C | 4 | int | FallRate | 0 | Constructor ASM; used by AI() for gravity |
| 0x30 | 4 | ptr | NextObject | NULL | Constructor ASM; linked list in cell |
| 0x34 | 4 | ptr | AttachedTag | NULL | Constructor ASM; trigger tag pointer |
| 0x38 | 4 | ptr | AttachedBomb | NULL | Constructor ASM; IvanBomb/C4 |
| 0x3C | 20 | CDTimerClass | Timer1 | constructed | `CALL 0x00405BE0` at ESI+0x3C |
| 0x50 | 20 | CDTimerClass | Timer2 | constructed | `CALL 0x00405BE0` at ESI+0x50 |
| 0x64 | 4 | int | CustomSound | -1 (0xFFFFFFFF) | Constructor ASM; -1 = no override |
| 0x68 | 1 | bool | BombVisible | false | Constructor ASM |
| 0x69-0x6B | 3 | — | padding | — | — |
| **0x6C** | **4** | **int** | **Health** | **255 (0xFF)** | Constructor ASM; ReceiveDamage reads/writes |
| **0x70** | **4** | **int** | **EstimatedHealth** | **255 (0xFF)** | Constructor ASM; pre-impact estimate |
| **0x74** | **1** | **bool** | **IsMarked** | **false** | Constructor ASM; Mark(PUT) sets to 1, Mark(REMOVE) sets to 0 |
| 0x75-0x77 | 3 | — | padding | — | — |
| 0x78 | 4 | int | Layer | 1 | Constructor ASM; map layer index |
| 0x7C | 4 | int | unknown_0x7C | 0 | Constructor ASM |
| 0x80 | 1 | bool | NeedsRedraw | false | Constructor ASM; cleared by DrawIt |
| **0x81** | **1** | **bool** | **InLimbo** | **true (1)** | Constructor ASM; Reveal sets to 0, Conceal sets to 1 |
| 0x82 | 1 | bool | InOpenToppedTransport | false | Constructor ASM |
| **0x83** | **1** | **bool** | **IsSelected** | **false** | Constructor ASM; Select sets to 1 |
| 0x84 | 1 | bool | HasParachute | false | Constructor ASM |
| 0x85-0x87 | 3 | — | padding | — | — |
| 0x88 | 4 | ptr | Parachute | NULL | Constructor ASM; AnimClass* for parachute |
| **0x8C** | **1** | **bool** | **OnBridge** | **false** | Constructor ASM; set by Unlimbo, used by GetHeight |
| 0x8D | 1 | bool | IsFallingDown | false | Constructor ASM; set by DropIn, cleared on landing |
| 0x8E | 1 | bool | WasFallingDown | false | Constructor ASM |
| 0x8F | 1 | bool | IsABomb | false | Constructor ASM; causes landing damage |
| **0x90** | **1** | **bool** | **IsAlive** | **true (1)** | Constructor ASM; UnInit sets to 0 |
| 0x91-0x93 | 3 | — | padding | — | — |
| 0x94 | 4 | int | LastLayer | -1 (0xFFFFFFFF) | Constructor ASM |
| 0x98 | 1 | bool | IsInLogic | false | Constructor ASM |
| 0x99 | 1 | bool | IsVisible | true (1) | Constructor ASM |
| 0x9A-0x9B | 2 | — | padding | — | — |
| **0x9C** | **4** | **int** | **Location.X** | 0 (from global) | Leptons; GetCoords reads, Set_Raw_Coords writes |
| **0xA0** | **4** | **int** | **Location.Y** | 0 (from global) | Leptons |
| **0xA4** | **4** | **int** | **Location.Z** | 0 (from global) | Leptons; GetHeight returns this |
| 0xA8 | 4 | ptr | LineTrailer | NULL | Constructor ASM; LineTrailClass* |

**Total:** 0xAC bytes (172 decimal). Verified from constructor, serialization, and cross-references.

## 3. Core Logic

### 3.1 Constructor (0x005F3900)

Pseudocode from verified assembly (NOT decompiler — offsets read directly from instructions):
```
ObjectClass::Constructor(this):
    INoticeSink::Constructor(this)          // sets AbstractClass fields
    this[0x24..0x28] = 0                    // clear unknowns
    this[0x2C] = 0                          // FallRate
    this[0x6C] = 0xFF                       // Health = 255 (default)
    this[0x70] = 0xFF                       // EstimatedHealth = 255
    this[0x78] = 1                          // Layer = 1
    this[0x81] = 1                          // InLimbo = true (not on map)
    this[0x90] = 1                          // IsAlive = true
    this[0x99] = 1                          // IsVisible = true
    this[0x30..0x38] = NULL                 // NextObject, Tag, Bomb
    this[0x64] = -1                         // CustomSound = none
    this[0x68] = 0                          // BombVisible
    this[0x74] = 0                          // IsMarked = false
    this[0x7C] = 0                          // unknown (see §9)
    this[0x80] = 0                          // NeedsRedraw
    this[0x82..0x84] = 0                    // InOpenTopped, IsSelected, HasParachute
    this[0x88] = NULL                       // Parachute anim
    this[0x8C..0x8F] = 0                    // OnBridge, IsFallingDown, WasFallingDown, IsABomb
    this[0x94] = -1                         // LastLayer
    this[0x98] = 0                          // IsInLogic
    this[0x9C..0xA4] = {0, 0, 0}           // Location = origin (from global at 0x00AC1380)
    this[0xA8] = NULL                       // LineTrailer
    // Set vtable pointers
    this[0x00] = 0x007EF060                 // ObjectClass primary vtable (also contains IPersistStream methods at +0x00..+0x1C)
    this[0x04] = 0x007EF044                 // IRTTITypeInfo vtable (6 methods: QI/AddRef/Release + Process/GetID/AssignUniqueID)
    this[0x08] = 0x007EF03C                 // INoticeSink vtable (1 method)
    this[0x0C] = 0x007EF034                 // INoticeSource vtable (1 method)
    // Initialize embedded timers
    CDTimerClass::Constructor(this + 0x3C)
    CDTimerClass::Constructor(this + 0x50)
    // Register in 4 global DynamicVector arrays
    g_AbstractArray.Add(this)               // 0x00A8E360
    g_ObjectArray1.Add(this)                // 0x00B0F720
    g_ObjectArray2.Add(this)                // 0x00B0F670
    g_ObjectArray3.Add(this)                // 0x00B0F618
    // Set "exists" flag
    this->AbstractFlags |= 0x02
```

### 3.2 Health System

**GetHealthRatio** (0x005F5C60):
```
return (double)this->Health / (double)this->TypeClass->Strength
```
Where `TypeClass->Strength` is at TypeClass + 0xA0 (max hit points from rules.ini).

**IsRedHP** (0x005F5CD0):
```
ratio = Health / TypeClass->Strength
return (ratio <= RulesClass->ConditionRed) AND (Health > 0)
```

**IsYellowHP** (0x005F5D20):
```
ratio = Health / TypeClass->Strength
return (ratio > RulesClass->ConditionRed) AND (ratio <= RulesClass->ConditionYellow)
```

**RulesClass thresholds:**
- `ConditionYellow` at RulesClass + 0x1700 (double) — default 0.50 (50%)
- `ConditionRed` at RulesClass + 0x1708 (double) — default 0.25 (25%)

**DamageState return values from ReceiveDamage:**
- 0 = NoDamage (no change)
- 1 = Unaffected (damage applied but no threshold crossed)
- 2 = Yellow (crossed Yellow threshold — see note below)
- 3 = Red (crossed ConditionRed threshold)
- 4 = Dead (Health reached 0)
- 5 = PostMortem (object no longer alive — early exit)

> **Split threshold behavior (verified via `decompile_function 0x005F5390`):**
> `ReceiveDamage` uses a **hardcoded** `maxHealth >> 1` (integer right-shift, always 50% of
> Strength) for the Yellow threshold-crossing event — it does **not** read `ConditionYellow`
> (RulesClass+0x1700). By contrast, `IsYellowHP` (0x005F5D20, verified via
> `decompile_function 0x005F5D20`) reads the configurable `ConditionYellow` value from
> RulesClass+0x1700. Consequently, if `ConditionYellow` is set to any value other than 50%,
> the health-bar color change (driven by `IsYellowHP`) and the Yellow damage event fired by
> `ReceiveDamage` will trigger at different HP levels.

### 3.3 ReceiveDamage (0x005F5390)

This is the core damage pipeline. Parameters (from stack):
- `int *damage` — pointer to damage value (modified in-place by armor calc)
- `WarheadTypeClass *warhead`
- `int distance_from_epicenter`
- `TechnoClass *attacker`
- `bool ignoreDefenses`
- `bool preventPassengerEscape`
- `HouseClass *attackerHouse`

```
ReceiveDamage(this, &damage, warhead, ..., attacker, ignoreDefenses, ..., attackerHouse):
    // Early exit: already dead, zero damage, or Immune=yes
    if Health <= 0 OR damage == 0:
        return NoDamage
    if NOT ignoreDefenses AND TypeClass->Immune:
        return NoDamage

    maxHealth = TypeClass->Strength

    // Armor calculation (unless ignoreDefenses)
    if NOT ignoreDefenses:
        damage = ArmorCalc(damage, warhead->Armor, TypeClass->ArmorType)
        // ArmorCalc at 0x00489180 uses Verses[] at warhead+300

    // Building special: force minimum 1 damage
    if WhatAmI() == Building AND NOT GodMode:
        if damage < 1: damage = 1

    if damage == 0: return NoDamage

    // HEALING (negative damage)
    if damage < 0:
        this->Health -= damage    // subtracting negative = adding
        if Health > maxHealth: Health = maxHealth
        if health changed: call vtable+0x148 (NotifyHealthChanged) with arg 7
        return NoDamage

    // Track threshold crossings
    damageState = Unaffected

    // Check Yellow threshold crossing
    if damage < Health:
        if (Health >= maxHealth/2) AND (Health - damage < maxHealth/2):
            damageState = Yellow
    else:
        damage = Health           // cap at remaining HP

    // Check Red threshold crossing
    redThreshold = maxHealth * ConditionRed
    if (Health > redThreshold) AND (Health - damage < redThreshold):
        damageState = Red

    // Apply damage
    this->Health = Health - damage

    // Special: Building with Crewed=yes, first lethal hit creates crew spawn anim
    // (if WhatAmI()==Building AND TypeClass has crew AND not already crewed)
    // Sets Health to ceil(some value), minimum 1, to survive first lethal hit

    // Fire trigger events for threshold crossings
    if damageState == Yellow:
        ProcessTrigger(0x27, ...)  // "Attacked" trigger
        ProcessTrigger(0x2A, ...)  // "HalfHealth" trigger
    if damageState == Red:
        ProcessTrigger(0x28, ...)  // "QuarterHealth" trigger
        ProcessTrigger(0x2B, ...)

    // Death handling
    if Health == 0:
        if attackerHouse == 0 OR attackerHouse == attacker->House:
            vtable+0xE0(attacker)          // RegisterDestruction(killer)
        else:
            vtable+0xE4(attackerHouse)     // RegisterDestruction(killerHouse)
        damageState = Dead
        vtable+0xDC(true)                  // Destroy(animated=true)

    // Fire generic damage triggers
    ProcessTrigger(0x06, ..., attacker)     // "Damaged" event
    ProcessTrigger(0x2C, ..., attacker)     // "AnyDamage" event

    // Redraw if selected
    if damageState != NoDamage AND IsSelected:
        vtable+0x124(2)                     // UpdateDisplay

    return damageState
```

### 3.4 Select (0x005F4520)

```
Select(this):
    if NOT g_AllowAllSelect:
        if InLimbo OR IsSelected: return false
        if NOT vtable+0x138(): return false    // CanBeSelected check

    currentPlayer = GetCurrentPlayer()
    if NOT g_AllowAllSelect AND IsSelectable():
        if currentPlayer == NULL: goto addToSelection
        if currentPlayer->IsObserver: return false

    if currentPlayer != NULL AND currentPlayer->IsDefeated():
        return false

    if g_MultiplayerDialogActive: return false

    // Add to CurrentSelection array at 0x00A8ECBC
    // Special case: if TypeClass->JumpjetTurnRate != 0 (Insignificant flag at +0xC9C)
    //   insert at FRONT of selection (prioritized)
    // Otherwise: append to end

    this->IsSelected = true  // byte at +0x83
    return true
```

### 3.5 Visibility: Conceal / Reveal

**Conceal** (0x005F4D30) — hides object from display:
```
Conceal(this):
    if NOT g_GameActive OR InLimbo: return false
    Deselect()                          // vtable+0x150
    vtable+0xDC(true)                   // NeedsRedraw
    vtable+0x124(0)                     // UpdateDisplay(remove)
    RemoveFromSortedLayer()
    DetachAnim()
    // If TypeClass->HasAlphaImage: remove fogged image
    // Dirty screen rect around object dimensions
    vtable+0x11C()                      // ClearDrawnState
    this->InLimbo = true                // +0x81 = 1
    this->NeedsRedraw = false           // +0x80 = 0
    return true
```

**Reveal** (0x005F4EC0) — makes object visible on display:
```
Reveal(this, coords):
    if coords == g_OriginSentinel: return false   // globals at 0x00AC1380/84/88 (currently {0,0,0})
    if NOT g_GameActive: return false
    if InLimbo:
        if NOT MapEditorMode:
            // Check if cell allows this object (vtable+0x1AC)
            if NOT allowed: return false
        this->InLimbo = false            // +0x81 = 0
        this->NeedsRedraw = false        // +0x80 = 0
        // Get TypeClass for dimension info
        // Adjust coords through TypeClass transform
        Set_Raw_Coords(adjustedCoords)    // vtable+0x1B4
        if Mark(MARK_PUT) succeeds:
            if IsAlive:
                DisplayClass::Submit_Object(this)
                // If TypeClass has AlphaImage: create AlphaShapeClass
                // If TypeClass has LineTrail: create LineTrailClass
            return true
        else:
            this->InLimbo = true         // revert
            return false
    return false
```

### 3.6 AI / Per-Tick Update (0x005F3E70)

ObjectClass::AI handles gravity/falling, sound playback, and parachute physics:
```
AI(this):
    // Play destruction sound
    if NOT InLimbo:
        if TypeClass->DestroySound != -1:
            VocClass::PlayAt(TypeClass->DestroySound, Location)
        if CustomSound != -1:
            VocClass::PlayAt(CustomSound, Location)

    // Gravity / falling physics
    if NOT IsFallingDown: return

    prevLayer = GetMapLayer()              // vtable+0x78
    rawZ = GetHeight_Raw()                 // vtable+0x1D0 — returns absolute Location.Z, NOT height-above-ground

    // Apply gravity: Location.Z = rawZ + FallRate
    if NOT IsMarked (0x74 flag):
        Location.Z = rawZ + FallRate
    else:
        Mark(MARK_REMOVE)
        Location.Z = rawZ + FallRate
        Mark(MARK_PUT)

    // Check if landed
    if GetHeight() < 1:
        SetHeight(0)                        // vtable+0x1CC
        IsFallingDown = false               // +0x8D = 0
        vtable+0x18C(2)                     // land notification
        // Clear parachute anim reference
        if Parachute != NULL:
            Parachute->ParachuteActive = 0

    // Apply gravity acceleration
    if NOT HasParachute:
        FallRate = ftol(FallRate * gravity_factor)
        clamp to RulesClass->MaxFallRate    // +0x7BC
    else:
        FallRate -= 1
        clamp to RulesClass->ParachuteFallRate  // +0x7B8

    // Update display layer if changed
    if prevLayer != GetMapLayer():
        DisplayClass::Submit_Object(this)

    // Landing damage (if IsABomb and Health > 0)
    if NOT IsFallingDown AND IsABomb AND Health > 0:
        ReceiveDamage(&Health, 0, RulesClass->C4Warhead, 0, true, true, 0)

    // Bouncer check: if infantry and landed on water
    if WhatAmI() == Infantry AND TypeClass->IsBouncer:
        // set some state, create splash anim
```

### 3.7 Mark Function (vtable+0x124, 0x005F5850)

The Mark function is the central function for placing/removing objects on the map grid.
It controls the **IsMarked** field at offset 0x74. Called by Conceal, Reveal, SetHeight,
movement systems, and many others.

```
Mark(this, markType):
    // markType: 0=MARK_REMOVE, 1=MARK_PUT, 2=MARK_CHANGE (redraw), 3=MARK_PUT (alias of 1)
    if InLimbo: return false

    if markType == 2:    // MARK_CHANGE
        if NeedsRedraw == 0 AND IsMarked:
            vtable+0x134()   // MarkNeedsRedraw
            return true
    else:
        // Type-specific checks for buildings...
        if markType == 1 || markType == 3:   // MARK_PUT
            if IsMarked == 0:
                IsMarked = true  // +0x74 = 1
                vtable+0x134()
                return true
        if markType == 0:   // MARK_REMOVE
            if IsMarked != 0:
                IsMarked = false  // +0x74 = 0
                return true
    return false
```

**IsMarked (offset 0x74) state machine:**
- false → true: via Mark(MARK_PUT) — object is now registered in cell grid
- true → false: via Mark(MARK_REMOVE) — object is removed from cell grid
- InLimbo objects cannot be marked
- IsMarked is separate from InLimbo: an object can be "not in limbo" but "not yet marked"
  during the brief window between Reveal and Mark(PUT)

### 3.8 Occupation Marking

**Mark_Occupation** (0x007441B0) — sets bit 0x20 on cell flags:
```
Mark_Occupation(coords):
    cell = CellClass::Get_Cell_At(coords)
    groundHeight = CellClass::GetGroundHeight(coords)
    if (coords.Z >= groundHeight + BridgeHeight) AND (cell->Flags & 0x100):
        cell->BridgeOccupation |= 0x20     // +0x128
    else:
        cell->GroundOccupation |= 0x20     // +0x124
```

**Clear_Occupation** (0x00744210) — clears bit 0x20:
```
Clear_Occupation(coords):
    cell = CellClass::Get_Cell_At(coords)
    groundHeight = CellClass::GetGroundHeight(coords)
    if (coords.Z >= groundHeight + BridgeHeight):
        cell->BridgeOccupation &= ~0x20    // +0x128
    else:
        cell->GroundOccupation &= ~0x20    // +0x124
```

**Mark_Put** (vtable+0xF0, 0x005F60A0) — sets bit 0x40 (object placed):
```
Mark_Put(coords):
    // Same bridge-height check, sets bit 0x40 instead of 0x20
    cell->GroundFlags |= 0x40   OR   cell->BridgeFlags |= 0x40
```

**Mark_Remove** (vtable+0xF4, 0x005F6120) — clears bit 0x40:
```
Mark_Remove(coords):
    cell->GroundFlags &= ~0x40   OR   cell->BridgeFlags &= ~0x40
```

### 3.9 Coordinate Functions

**GetCoords** (0x005F65A0): `return {this+0x9C, this+0xA0, this+0xA4}`

**Set_Raw_Coords** (0x005F6940): `this+0x9C = X; this+0xA0 = Y; this+0xA4 = Z`

**GetHeight (simple)** (0x005F5F30): `return this->Location.Z` (raw Z value)

**GetHeight (above ground)** (0x005F5F40):
```
groundZ = CellClass::GetGroundHeight(this->Location)
height = this->Location.Z - groundZ
if this->OnBridge:
    height -= BridgeHeight     // global at 0x00AC13BC
return height
```

**SetHeight** (0x005F5FA0):
```
SetHeight(this, newHeight):
    if OnBridge: newHeight += BridgeHeight
    if IsMarked:                   // gated on +0x74, not a separate "OnMap" flag
        Mark(MARK_REMOVE)
        Location.Z = GetGroundHeight(Location) + newHeight
        Mark(MARK_PUT)
    else:
        Location.Z = GetGroundHeight(Location) + newHeight
```

### 3.10 Y-Sort / Render Order

**GetYSort** (0x005F6BD0):
```
GetYSort(this):
    coords1 = vtable+0xAC()     // GetRenderCoords
    coords2 = vtable+0xAC()     // GetRenderCoords (again)
    return coords1.Y + coords2.X  // Y position + X offset for sort key
```

**YSortComparator** (0x005F6220):
```
YSortComparator(a, b):
    return b->GetYSort() < a->GetYSort()   // descending Y order
```

### 3.11 UnInit (0x005F65F0)

```
UnInit(this):
    // Detach bomb if present
    if AttachedBomb != NULL:
        DetachBomb()
    // Remove EMP effects if TechnoClass
    if (AbstractFlags & 0x01):    // IsTechno
        FootClass::EMPPassengers(0)
    RemoveFromSortedLayerList()
    vtable+0xD4()                 // Conceal (remove from display)
    this->IsAlive = false         // +0x90 = 0
    // Add to pending-delete DynamicVector at 0x00B0F69C
    g_PendingDeleteArray.Add(this)
```

### 3.12 Receive_Radio (0x005F5320)

ObjectClass base handles two radio messages:
```
Receive_Radio(this, from, message):
    if message == 0x0D:          // RADIO_NEED_REDRAW
        vtable+0x124(2)          // UpdateDisplay
        return RADIO_ROGER (1)

    if message == 0x22:          // RADIO_CAN_REPAIR
        ratio = Health / TypeClass->Strength
        if ratio >= RulesClass->RepairThreshold:   // +0x16F8
            return RADIO_NEGATIVE (10)
        return RADIO_ROGER (1)

    return RADIO_STATIC (0)      // unhandled
```

### 3.13 Save / Serialization (0x005F6250)

Serializes these fields in order:
```
Save(this, stream):
    AbstractClass::ComputeCRC(stream)       // 0x00410410 — the first call is ComputeCRC, not a generic Save
    // NextObject and AttachedTag are persisted via a virtual call on their
    // secondary COM vtable (at NextObject+4), not a simple pointer write:
    //   if (this+0x30 != NULL): stream.WriteDword( vtable2[0x10]((this+0x30)+4) )
    //   if (this+0x34 != NULL): stream.WriteDword( vtable2[0x10]((this+0x34)+4) )
    SavePtr(this + 0x30)         // NextObject (see note above)
    SavePtr(this + 0x34)         // AttachedTag (see note above)
    Save_DWord(this + 0x6C)      // Health
    Save_Byte(this + 0x74)       // IsMarked
    // Conditional on game mode (NOT campaign, NOT scenario editor):
    Save_Byte(this + 0x80)       // NeedsRedraw
    Save_Byte(this + 0x83)       // IsSelected
    Save_Byte(this + 0x81)       // InLimbo
    Save_Byte(this + 0x84)       // HasParachute
    Save_Byte(this + 0x8C)       // OnBridge
    Save_Byte(this + 0x8D)       // IsFallingDown
    Save_Byte(this + 0x8F)       // IsABomb
    Save_Byte(this + 0x90)       // IsAlive
    Save_DWord(this + 0x9C)      // Location.X
    Save_DWord(this + 0xA0)      // Location.Y
    Save_DWord(this + 0xA4)      // Location.Z
```

### 3.14 Detach (0x005F5230)

Called when a referenced object is being removed:
```
Detach(this, target, all):
    if target == AttachedTag AND AttachedTag != NULL:
        AttachedTag->RefCount--
        AttachedTag = NULL
    if all AND target == NextObject AND NextObject != NULL:
        NextObject = NextObject->NextObject    // relink chain
    if target == Parachute:
        Parachute = NULL
```

## 4. VTable Map (0x007EF060)

Partial map of the 122-entry vtable with verified method identifications:

The first 8 slots (+0x00..+0x1C) are the **IUnknown + IPersistStream COM interface
methods** inherited via AbstractClass — not ObjectClass-specific C++ methods. The
C++ class's own virtual methods start at +0x20 (scalar deleting destructor).

| Offset | Address | Method | Confidence |
|--------|---------|--------|------------|
| +0x00 | 0x410260 | **IUnknown::QueryInterface** (compares IID against 4 known GUIDs, calls AddRef on match) | HIGH |
| +0x04 | 0x410300 | **IUnknown::AddRef** (stub — returns 1, no refcounting in this engine) | HIGH |
| +0x08 | 0x410310 | **IUnknown::Release** (stub — returns 1) | HIGH |
| +0x0C | 0x4C9150 | __purecall — **IPersistStream::GetClassID** (pure, derived overrides) | HIGH |
| +0x10 | 0x410450 | **IPersistStream::IsDirty** (returns `this->Dirty (+0x20) == 0` — inverted) | HIGH |
| +0x14 | 0x5F5E80 | **IPersistStream::Load** — ObjectClass::Load | HIGH |
| +0x18 | 0x4C9150 | __purecall — **IPersistStream::Save** (pure; derived classes override. ObjectClass::Save at 0x5F6250 is a separate C++ method at +0x34, not this COM slot) | HIGH |
| +0x1C | 0x4103E0 | **IPersistStream::GetSizeMax** (returns `vtable[0x30]() + 4`) | HIGH |
| +0x20 | 0x5F6DC0 | ObjectClass::scalar_deleting_dtor (first C++ vtable slot; calls destructor body at 0x5F3B80, frees if delete flag set) | HIGH |
| +0x24 | 0x410470 | AbstractClass stub (empty `RET`) | HIGH |
| +0x28 | 0x5F5230 | ObjectClass::Detach | HIGH |
| **+0x2C** | **0x4C9150** | **__purecall — AbstractClass::What_Am_I (returns AbstractType enum: Unit=1, Aircraft=4, Building=6, Infantry=0xF, Overlay=0x14, Terrain=0x24)** | **HIGH** |
| +0x30 | 0x4C9150 | __purecall — **"Get serialized size"** (called by GetSizeMax at +0x1C; derived class returns its byte count, GetSizeMax adds 4 and reports it as the COM size-out) | HIGH |
| +0x34 | 0x5F6250 | ObjectClass::Save | HIGH |
| +0x38 | 0x410490 | AbstractClass stub | MEDIUM |
| +0x3C | 0x4104A0 | AbstractClass stub | MEDIUM |
| +0x40 | 0x4104B0 | AbstractClass stub | MEDIUM |
| +0x44 | 0x5F6690 | **ObjectClass::IsDead** (returns !IsAlive) | **HIGH** |
| **+0x48** | **0x5F65A0** | **ObjectClass::GetCoords** | **HIGH** |
| +0x4C | 0x4104F0 | AbstractClass stub (GetFiringCoords) | MEDIUM |
| +0x50 | 0x5F6B60 | **ObjectClass::IsLowFlying** (IsMarked && heightAboveGround < 2 * FlightAltThreshold @ 0x00AC13C8) | **HIGH** |
| +0x54 | 0x5F6B90 | **ObjectClass::IsHighFlying** (IsMarked && heightAboveGround >= 2 * FlightAltThreshold @ 0x00AC13C8) | **HIGH** |
| +0x58 | 0x410540 | AbstractClass stub | MEDIUM |
| +0x5C | 0x5F3E70 | **ObjectClass::AI** (per-tick: gravity, sound, falling) | **HIGH** |
| +0x60 | 0x5F6DA0 | **ObjectClass::DetachParachute** | **HIGH** |
| +0x64 | 0x426390 | stub | LOW |
| +0x68 | 0x4263A0 | stub | LOW |
| +0x6C | 0x5F3E30 | ObjectClass::GetTypeClass_indirect | HIGH |
| +0x70 | 0x5F4250 | ObjectClass::Limbo (stub, returns 0, 3 params) | HIGH |
| +0x74 | 0x5F4240 | ObjectClass::Unlimbo_stub (returns 0, 2 params) | HIGH |
| +0x78 | 0x5F4260 | ObjectClass::GetMapLayer | HIGH |
| +0x7C | 0x5F6C10 | **ObjectClass::IsAboveGround** (height > -20) | **HIGH** |
| +0x80 | 0x4263B0 | stub | LOW |
| +0x84 | 0x5F6BC0 | ObjectClass::GetActionOnCell | MEDIUM |
| +0x88 | 0x4E0130 | ObjectClass::GetTypeClass | HIGH |
| +0x8C | 0x5F42A0 | **ObjectClass::GetThreatRating** (returns INT_MAX) | **HIGH** |
| +0x90 | 0x4263C0 | ObjectClass::GetName (returns default) | LOW |
| +0x94 | 0x5F42B0 | ObjectClass stub (returns 0) | MEDIUM |
| +0x98 | 0x5F42C0 | ObjectClass stub (returns 0) | MEDIUM |
| +0x9C | 0x5F42D0 | ObjectClass stub (returns 0) | MEDIUM |
| +0xA0 | 0x5F42E0 | ObjectClass stub (returns 0) | HIGH |
| +0xA4 | 0x41BDD0 | ObjectClass::GetActionCoords (calls GetCoords) | HIGH |
| +0xA8 | 0x5F6C80 | ObjectClass::GetTargetCoords (calls GetCoords) | HIGH |
| **+0xAC** | **0x41BE00** | **ObjectClass::GetRenderCoords** | **HIGH** |
| +0xB0 | 0x4263D0 | stub | LOW |
| +0xB4 | 0x41BE30 | ObjectClass::GetExitCoords (calls GetCoords) | HIGH |
| **+0xB8** | **0x5F6BD0** | **ObjectClass::GetYSort** | **HIGH** |
| +0xBC | 0x5F6A70 | **ObjectClass::ShouldBeOnBridge** | **HIGH** |
| +0xC0-0xC8 | 0x4264xx | stubs | LOW |
| +0xCC | 0x41BE60 | ObjectClass stub (returns 0) | MEDIUM |
| +0xD0 | 0x41BE70 | ObjectClass stub (returns 0) | MEDIUM |
| +0xD4 | 0x5F4D30 | **ObjectClass::Conceal** | **HIGH** |
| +0xD8 | 0x5F4EC0 | **ObjectClass::Reveal** | **HIGH** |
| +0xDC | 0x5F5280 | **ObjectClass::Destroy** | **HIGH** |
| +0xE0 | 0x5F42F0 | ObjectClass::RegisterDestruction (stub, `RET 4`) | HIGH |
| +0xE4 | 0x5F4300 | ObjectClass::RegisterDestruction2 (stub, `RET 4`) | HIGH |
| +0xE8 | 0x5F5940 | **ObjectClass::Unlimbo_Full** (place on map w/ parachute) | **HIGH** |
| +0xEC | 0x5F4160 | **ObjectClass::DropIn** (begin falling) | **HIGH** |
| **+0xF0** | **0x5F60A0** | **ObjectClass::Mark_Put** (set bit 0x40 on cell) | **HIGH** |
| **+0xF4** | **0x5F6120** | **ObjectClass::Mark_Remove** (clear bit 0x40 on cell) | **HIGH** |
| **+0xF8** | **0x5F65F0** | **ObjectClass::UnInit** | **HIGH** |
| +0xFC | 0x5F4310 | ObjectClass stub (empty) | HIGH |
| +0x100 | 0x5F4320 | ObjectClass stub (returns 0) | HIGH |
| +0x104 | 0x5F4B10 | **ObjectClass::DrawIt** | **HIGH** |
| +0x108 | 0x5F5B90 | ObjectClass::GetImage (delegates to TypeClass) | HIGH |
| +0x10C-0x110 | 0x42644x | stubs | LOW |
| +0x114 | 0x5B3A50 | ObjectClass::DrawSHP_base | MEDIUM |
| +0x118 | 0x5F65D0 | ObjectClass::DrawVeterancyPips (wraps +0x114) | HIGH |
| +0x11C | 0x5F4330 | ObjectClass stub (empty, ClearDrawnState) | HIGH |
| +0x120 | 0x5F4340 | ObjectClass stub (empty) | HIGH |
| +0x124 | 0x5F5850 | **ObjectClass::Mark** (0=remove, 1=put, 2=redraw) | **HIGH** |
| +0x128 | 0x5F4730 | **ObjectClass::GetDrawExtent** | **HIGH** |
| +0x12C | 0x5F4870 | **ObjectClass::GetDrawRect** | **HIGH** |
| +0x130 | 0x41BE80 | ObjectClass stub (empty) | HIGH |
| +0x134 | 0x5F4D10 | **ObjectClass::MarkNeedsRedraw** (sets +0x80=1) | **HIGH** |
| +0x138 | 0x5F6C30 | **ObjectClass::CanBeSelected** | **HIGH** |
| +0x13C | 0x5F6C70 | ObjectClass::CanBeSelected_wrapper | HIGH |
| +0x140 | 0x5F4360 | ObjectClass stub (returns 0) | HIGH |
| +0x144 | 0x5F4350 | ObjectClass stub (returns 0) | HIGH |
| +0x148 | 0x5F4370 | ObjectClass::NotifyHealthChanged (stub, empty) | HIGH |
| +0x14C | 0x5F4520 | **ObjectClass::Select** | **HIGH** |
| +0x150 | 0x5F44A0 | **ObjectClass::Deselect** | **HIGH** |
| +0x154-0x160 | 0x42646x-0x42649x | stubs | LOW |
| +0x164 | 0x5F4380 | ObjectClass stub (returns 0) | HIGH |
| +0x168 | 0x5F4390 | ObjectClass stub (returns 0) | HIGH |
| **+0x16C** | **0x5F5390** | **ObjectClass::ReceiveDamage** | **HIGH** |
| +0x170 | 0x4264A0 | stub | LOW |
| +0x174 | 0x5F43A0 | ObjectClass::Scatter (stub, empty) | HIGH |
| +0x178 | 0x5F43B0 | ObjectClass stub | HIGH |
| +0x17C | 0x5F43C0 | ObjectClass stub | HIGH |
| +0x180 | 0x5F43D0 | ObjectClass stub (returns 0) | HIGH |
| +0x184 | 0x5F43E0 | ObjectClass stub (returns -1) | HIGH |
| +0x188 | 0x41BE90 | ObjectClass stub (empty) | HIGH |
| +0x18C | 0x4264B0 | stub | LOW |
| +0x190 | 0x5F5C20 | ObjectClass::CreateRadialIndicator | HIGH |
| +0x194 | 0x5F5320 | **ObjectClass::Receive_Radio** | **HIGH** |
| +0x198 | 0x5F5930 | ObjectClass stub — `return param_1 != 0` (trivial non-null predicate) | HIGH |
| +0x19C | 0x5F43F0 | ObjectClass stub (empty) | HIGH |
| +0x1A0 | 0x5F4400 | ObjectClass stub (empty) | HIGH |
| +0x1A4 | 0x5F6B50 | ObjectClass stub (empty) | HIGH |
| +0x1A8 | 0x5F4410 | **ObjectClass::UpdatePosition** | **HIGH** |
| +0x1AC | 0x4264C0 | stub (cell passability check) | LOW |
| +0x1B0 | 0x4264D0 | stub | LOW |
| **+0x1B4** | **0x5F6940** | **ObjectClass::Set_Raw_Coords** | **HIGH** |
| +0x1B8 | 0x41BEA0 | **ObjectClass::GetCellCoords** (leptons / 256) | **HIGH** |
| +0x1BC | 0x5F6960 | **ObjectClass::GetOccupiedCell** | **HIGH** |
| +0x1C0 | 0x5F69C0 | ObjectClass::GetOccupiedCellClass | HIGH |
| +0x1C4 | 0x5F6A10 | ObjectClass::GetOccupiedCellClass2 | HIGH |
| **+0x1C8** | **0x5F5F40** | **ObjectClass::GetHeight (above ground)** | **HIGH** |
| **+0x1CC** | **0x5F5FA0** | **ObjectClass::SetHeight** | **HIGH** |
| **+0x1D0** | **0x5F5F30** | **ObjectClass::GetHeight (raw Z)** | **HIGH** |
| +0x1D4-0x1E4 | 0x4264E0-0x426520 | stubs (5 entries) | LOW |

## 5. Global Arrays

ObjectClass instances are registered in 4 global `DynamicVectorClass<ObjectClass*>` arrays
on construction, and added to a pending-delete array on UnInit:

| Global Base | Purpose | Evidence |
|-------------|---------|----------|
| 0x00A8E360 | AbstractClass master array | Constructor + xrefs |
| 0x00B0F720 | Object tracking array 1 | Constructor + ALPHA_SHAPE report |
| 0x00B0F670 | Object tracking array 2 | Constructor |
| 0x00B0F618 | Object tracking array 3 | Constructor |
| 0x00B0F698 | Pending-delete array | UnInit adds here |

## 6. INI Keys

ObjectClass-level properties parsed from rules.ini / rulesmd.ini:

| Key | Type | Default | Effect | RulesClass Offset |
|-----|------|---------|--------|-------------------|
| `Strength` | int | 0 | Max hit points (TypeClass+0xA0) | — |
| `Armor` | enum | none | Armor type for Verses[] lookup | — |
| `Immune` | bool | false | Ignores all damage (TypeClass+0x233) | — |
| `Selectable` | bool | true | Can be selected by player | — |
| `LegalTarget` | bool | true | Can be targeted by weapons | — |
| `Insignificant` | bool | false | Doesn't count for victory | — |
| `HasRadialIndicator` | bool | false | Shows range circle | — |
| `ConditionYellow` | float | 50% | Health bar turns yellow | +0x1700 (double) |
| `ConditionRed` | float | 25% | Health bar turns red | +0x1708 (double) |
| `MaxDamage` | int | — | Damage cap per hit | +0x16C8 |
| `RepairThreshold` | float | — | Ratio below which repair is needed | +0x16F8 (double) |

## 7. Integration Points

**Who creates ObjectClass instances:**
- Derived class constructors only (AnimClass, OverlayClass, MissionClass/TechnoClass chain,
  TerrainClass, SmudgeClass, ParticleClass, VoxelAnimClass, WaveClass). ObjectClass itself
  is never directly instantiated.

**Who calls ReceiveDamage:**
- Combat system, warhead splash damage, IvanBomb detonation, C4, radiation, EMP,
  crushing, mind control break, trigger actions — 35+ call sites.

**When AI() runs in tick cycle:**
- Called as part of the main game loop per-object update. Handles gravity/falling and
  sound playback. Derived classes override this heavily (TechnoClass::AI, FootClass::AI, etc.)
  but always call `ObjectClass::AI()` first via the base pointer.

**Where in the sim tick:**
- ObjectClass::AI runs during the object iteration phase. The `World::advance_tick` order
  in our engine should call object AI after commands but before heavy sim systems.

## 8. Current Rust Implementation Status

The Rust engine uses a unified `GameEntity` struct in `src/sim/game_entity.rs` rather than
a class hierarchy. Key mappings:

| ObjectClass Field | GameEntity Field | Status |
|-------------------|------------------|--------|
| Health (+0x6C) | `health: Health` (current + max) | Implemented |
| Location (+0x9C-0xA4) | `position: Position` | Implemented (different coord system) |
| IsSelected (+0x83) | `selected: bool` | Implemented |
| InLimbo (+0x81) | (implicit — entity not in store) | Different approach |
| OnBridge (+0x8C) | `on_bridge: bool` | Implemented |
| IsFallingDown (+0x8D) | Not implemented | Missing |
| IsABomb (+0x8F) | Not implemented | Missing |
| FallRate (+0x2C) | Not implemented | Missing (gravity system) |
| HasParachute (+0x84) | Not implemented | Missing |
| NextObject (+0x30) | (cell occupancy via different mechanism) | Different approach |
| AttachedTag (+0x34) | Not implemented | Missing (triggers) |
| AttachedBomb (+0x38) | Not implemented | Missing (IvanBomb) |
| LineTrailer (+0xA8) | Not implemented | Missing |
| Timers (+0x3C, +0x50) | Not implemented | Missing |
| Layer (+0x78) | `locomotor.layer` | Partial |
| IsAlive (+0x90) | `dying: bool` (inverse) | Implemented (different semantics) |
| NeedsRedraw (+0x80) | (frame-based rendering) | Different approach |
| Conceal/Reveal | (shroud system) | Partially implemented |
| ReceiveDamage | `src/sim/combat/mod.rs` | Implemented (verify accuracy) |

**Missing systems:**
1. Gravity / falling physics (FallRate, IsFallingDown, parachute)
2. IvanBomb attachment system
3. Trigger tag system (AttachedTag, ProcessTrigger)
4. Line trail rendering
5. Embedded CDTimerClass timers
6. Object-to-cell linked list (NextObject)

## 9. Open Questions

1. **Offsets 0x24 and 0x28**: Always initialized to 0, NOT serialized in Save/Load. Confirmed
   absent from the constructor bodies of every direct derived class checked:
   `AnimClass::Constructor (0x00421EA0)`, `BulletClass::Constructor (0x00466380)`,
   `MissionClass::Constructor (0x005B2DA0)`, `TechnoClass::Constructor (0x006F2B40)`,
   `FootClass::Constructor (0x004D31E0)` — all begin their derived-field writes at `0xAC` or
   later. Fields are likely either truly unused (TS-era leftover) or written only in specific
   runtime paths not part of the construction chain. **Confidence: MEDIUM that these fields
   are inert at construction; LOW on any runtime purpose claim.**

2. ~~**Offset 0x74**~~: **RESOLVED** — This is **IsMarked**. Set to 1 by Mark(MARK_PUT),
   cleared to 0 by Mark(MARK_REMOVE). Controls whether the object is currently registered
   in cell occupancy/display. The Mark function at vtable+0x124 (0x005F5850) reads/writes
   this field. Also used by IsLowFlying/IsHighFlying height checks (vtable+0x50/+0x54).
   **Confidence: HIGH**

3. **Offset 0x7C**: 4-byte field, initialized to 0. Not serialized (absent from `ObjectClass::Save`).
   Not written by any of the direct-derived constructors checked (Anim/Bullet/Mission/Techno/Foot
   — all start derived-field writes at `0xAC` or later). Like 0x24/0x28, it is either inert or
   touched only in runtime methods not explored here. **Confidence: MEDIUM that it is inert at
   construction.**

4. **Timer objects at 0x3C and 0x50**: CDTimerClass instances (20 bytes each) embedded in
   ObjectClass. Constructed by `CDTimerClass::Constructor (0x00405BE0)`. Neither timer is
   referenced by the ObjectClass methods decompiled in this investigation (`Mark`, `Save`,
   `Load`, `AI`, `Conceal`, `Reveal`, `UnInit`, etc.), and neither is serialized in
   `ObjectClass::Save`. Derived-class constructors likewise don't reference them by offset
   inside their own ctor bodies. The timers are therefore wired only from runtime methods in
   one of the deeper derived classes (TechnoClass/FootClass AI, combat, or animation paths).
   Best guess — ROF delay and cloak/animation — remains unverified. **Confidence: LOW on
   purpose. Existence, size (20 bytes), and ctor call confirmed.**

5. ~~**AbstractFlags bit 1 semantics**~~: **RESOLVED** — Bit 1 is `IsObject`, set by
   `ObjectClass::Constructor` at `0x005F3B37`. Together with bit 0 (IsTechno, `TechnoClass::Constructor`
   at `0x006F322F`) and bit 2 (IsFoot, `FootClass::Constructor` at `0x004D34DD`), the three low
   bits form a class-family type mask that lets runtime code cheaply discriminate
   Object/Techno/Foot without a virtual call. **Confidence: HIGH.**

6. **Offset 0x82 (InOpenToppedTransport)**: Initialized to false, purpose inferred from
   naming convention. Needs verification that this is the correct interpretation. Likely
   used by the garrison/transport drawing system.
   **Confidence: MEDIUM**

## Sources

### Ghidra Addresses Decompiled (50+ functions)
- 0x005F3900 — ObjectClass::Constructor (main, 158 ASM instructions verified)
- 0x005F3B50 — ObjectClass::Constructor (copy/load)
- 0x005F5390 — ObjectClass::ReceiveDamage
- 0x005F4520 — ObjectClass::Select
- 0x005F44A0 — ObjectClass::Deselect
- 0x005F5C60 — ObjectClass::GetHealthRatio
- 0x005F5CD0 — ObjectClass::IsRedHP
- 0x005F5D20 — ObjectClass::IsYellowHP
- 0x005F65A0 — ObjectClass::GetCoords
- 0x005F5F30 — ObjectClass::GetHeight (raw Z)
- 0x005F5F40 — ObjectClass::GetHeight (above ground)
- 0x005F5FA0 — ObjectClass::SetHeight
- 0x005F65F0 — ObjectClass::UnInit
- 0x005F6220 — ObjectClass::YSortComparator
- 0x005F6BD0 — ObjectClass::GetYSort
- 0x005F6940 — ObjectClass::Set_Raw_Coords
- 0x005F6960 — ObjectClass::GetOccupiedCell
- 0x007441B0 — ObjectClass::Mark_Occupation
- 0x00744210 — ObjectClass::Clear_Occupation
- 0x005F60A0 — ObjectClass::Mark_Put
- 0x005F6120 — ObjectClass::Mark_Remove
- 0x005F4D30 — ObjectClass::Conceal
- 0x005F4EC0 — ObjectClass::Reveal
- 0x005F5280 — ObjectClass::Destroy
- 0x005F3E70 — ObjectClass::AI
- 0x005F5320 — ObjectClass::Receive_Radio
- 0x005F6250 — ObjectClass::Save
- 0x005F5230 — ObjectClass::Detach
- 0x005F5940 — ObjectClass::Unlimbo_Full
- 0x005F4160 — ObjectClass::DropIn
- 0x005F4B10 — ObjectClass::DrawIt
- 0x005F4730 — ObjectClass::GetDrawExtent
- 0x005F4870 — ObjectClass::GetDrawRect
- 0x005F5E80 — ObjectClass::Load
- 0x005F5850 — ObjectClass::Mark (vtable+0x124)
- 0x005F4250 — ObjectClass::Limbo (stub)
- 0x005F4240 — ObjectClass::Unlimbo (stub)
- 0x005F6690 — ObjectClass::IsDead
- 0x005F6B60 — ObjectClass::IsLowFlying
- 0x005F6B90 — ObjectClass::IsHighFlying
- 0x005F6DA0 — ObjectClass::DetachParachute
- 0x005F6C10 — ObjectClass::IsAboveGround
- 0x005F42A0 — ObjectClass::GetThreatRating (returns INT_MAX)
- 0x005F6A70 — ObjectClass::ShouldBeOnBridge
- 0x005F4D10 — ObjectClass::MarkNeedsRedraw
- 0x005F6C30 — ObjectClass::CanBeSelected
- 0x005F5C20 — ObjectClass::CreateRadialIndicator
- 0x005F4410 — ObjectClass::UpdatePosition
- 0x005F69C0 — ObjectClass::GetOccupiedCellClass
- 0x005F6A10 — ObjectClass::GetOccupiedCellClass2
- 0x005F5B90 — ObjectClass::GetImage
- 0x0041BEA0 — ObjectClass::GetCellCoords (leptons / 256)
- 0x0041BDD0 — ObjectClass::GetActionCoords
- 0x0041BE30 — ObjectClass::GetExitCoords
- 0x00489180 — ArmorCalc (damage * verses)
- 0x00410170 — INoticeSink/AbstractClass::Constructor
- 0x0041BE00 — ObjectClass::GetRenderCoords
- 0x00405BE0 — CDTimerClass::Constructor
- 20+ additional stubs decompiled and confirmed as empty/default returns

### Assembly Verified
- Constructor at 0x005F3900: all 158 instructions examined for direct byte offsets
- Limbo stub at 0x005F4250: `XOR EAX,EAX; RET 0xC` (3 params, returns 0)
- Unlimbo stub at 0x005F4240: `XOR EAX,EAX; RET 0x8` (2 params, returns 0)
- Death handlers at 0x5F42F0, 0x5F4300: `RET 0x4` (stubs, 1 param)

### Existing Research Docs Referenced
- BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md (ObjectClass field map, cross-referenced)
- TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md (inheritance chain verified)
- GAMEMD_ARCHITECTURE.md (class hierarchy)
- OBJECT_FOG_VISIBILITY_GHIDRA_REPORT.md (Conceal/Reveal vtable offsets)
- DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md (Mark_Occupation verified)
- ALPHA_SHAPE_CLASS_LIFECYCLE.md (global tracking arrays)
- ADDRESS_MAP.md (known function addresses)

### INI Files Checked
- ini/rulesmd.ini: ConditionYellow=50%, ConditionRed=25%, Strength, Armor, Immune, etc.
- ini/rules.ini: base RA2 values (overridden by rulesmd.ini)

---

## Tier 5 application record (2026-08-17, Claude Code session)

Corridor: `docs/plans/2026-08-17-ghidra-typing-corridor-program.md` row 5, "ObjectClass core
slots + fields". Snapshot before mutations:
`C:/Users/enok/Documents/ghidra-backups/2026-08-17-pre-tier5` (17 files, 243,310,601 bytes,
byte-count verified against source with the program closed). Live Ghidra is the authority on
applied-ness; this section records evidence and holes only.

Prior-tier spot samples re-verified live before extending: tier 1
`MissionClass__Assign_Mission` `void __thiscall(MissionClass*, YR_Mission)`; tier 2
`CellClass__RecalcAttributes` `void __thiscall(CellClass*, int levelOverride)`; tier 3
`DriveLocomotionClass__ILocomotion_Is_Moving` `bool __stdcall(void*)`; tier 4 `/FootClass`
1728 B with its 41 fields. All hold.

**Structure size is unchanged at 172 bytes** — `/MissionClass` (212 B) embeds
`ObjectClass base_ObjectClass` at offset 0, so any size change would silently corrupt every
derived layout. Verified after every mutation.

### Corrections this tier makes to THIS document

The field table above is wrong in four places. Do not cite it for these rows:

1. **0x3C and 0x50 are NOT 20-byte `CDTimerClass` timers.** They are two 16-byte sound
   handles. `ObjectClass__Constructor` initialises both through `VocHandle__Init` 0x00405BE0,
   which writes only +0x0/+0x4/+0x8 (zero) and +0xC (the tag 0x0087E294); no accessor in the
   family touches +0x10 of an instance. 0x3C carries the type's `AmbientSound=`
   (ObjectTypeClass+0x1F4, bound by `ObjectTypeClass__ReadINI` at 0x005F9403 from the
   `"AmbientSound"` string at 0x00832BE4); 0x50 carries `CustomSound` — proven by
   `ObjectClass__Set_Custom_Sound` 0x005F6CB0, where writing the -1 sentinel to +0x64 stops
   exactly the +0x50 handle. The 0x14 spacing is padding, not the type's size: TechnoClass
   spaces the same type 0x1C apart.
2. **0x78 is NOT `Layer`.** The constructor writes 1 and nothing in the program ever reads it
   (two independent untruncated scans). The layer is computed on demand by
   `ObjectClass__InWhichLayer` 0x005F4260, which never touches 0x78, and the cached layer is
   **0x94** (`DisplayClass__Submit_Object` 0x004A9720 writes it, `RemoveFromLayer` 0x004A9770
   restores the -1 sentinel). 0x78's init value of 1 is itself inconsistent with the -1
   "no layer" sentinel.
3. **0x84 is not `HasParachute`.** Its only writers are `AnimClass__SetOwnerObject`
   (0x00424C30 set / 0x00424BB6 clear, the latter only after proving no other anim owns the
   object) and `AnimClass__Destructor` 0x0042295B. It means "an anim is attached". The
   parachute reading came from its one ObjectClass consumer — `ObjectClass__AI` 0x005F3FB2
   picks the parachute fall-rate floor when it is set — which is reached because
   `ObjectClass__Paradrop` stores the parachute anim at +0x88 then calls SetOwnerObject.
   Applied as `IsAnimAttached`.
4. **0x8F is not `IsABomb`.** Written only by `ObjectClass__DropIn` 0x005F4171; consumed by
   `ObjectClass__AI` 0x005F4021, which on landing calls ReceiveDamage with damage taken from
   the object's own current Health and warhead RulesClass+0xFA8 — guaranteed death on impact.
   The bomb pointer is 0x38. Applied as `DiesOnImpact`.

Also corrected: **0x99 is a per-frame drawn latch, not persistent visibility.**
`Tactical_ObjectRenderingLoop` clears it as each object is pulled from the layer array
(0x006D8F48) and sets it at each of five emit points immediately before the draw dispatch
(0x006D907E and four others); a later pass reads it to skip undrawn objects. Applied as
`IsDrawnThisFrame`. And **0x1C is not a `RefCount`** — AddRef/Release at 0x00410300/0x00410310
are both literally `MOV EAX,0x1; RET 0x4`, and no INC/DEC of the slot exists program-wide. Its
only proven property is that `AbstractClass__Load` rescues it across the raw stream overwrite.
Applied as `nReserved_0x1C`.

### Fields applied (30 rows)

Widths come from the access instructions; roles from readers/writers. `ObjectClass__Load`
0x005F5E80 swizzles 0x30/0x34/0x38/0x18/0x88 — pointer proof for all five.
`ObjectClass__ComputeCRC` 0x005F6250 gives independent width proof for twelve rows via its
`AddInt32`/`AddBool` split.

`UniqueID` 0x10 int · `bAbstractFlags` 0x14 **byte, not int** (bits 0=Techno / 1=Object /
2=Foot; every access program-wide is 8-bit) · `pUnknownSwizzled_0x18` void* · `nReserved_0x1C`
int · `Dirty` 0x20 bool · `FallRate` 0x2C int · `pNextObject` 0x30 **`ObjectClass *`** ·
`AttachedTag` 0x34 · `AttachedBomb` 0x38 · `AmbientSoundHandle` 0x3C `VocHandle` ·
`CustomSoundHandle` 0x50 `VocHandle` · `CustomSound` 0x64 · `BombVisible` 0x68 · `Health` 0x6C ·
`EstimatedHealth` 0x70 · `IsOnMap` 0x74 · `NeedsRedraw` 0x80 · `InLimbo` 0x81 ·
`InOpenToppedTransport` 0x82 · `IsSelected` 0x83 · `IsAnimAttached` 0x84 · `Parachute` 0x88 ·
`OnBridge` 0x8C · `IsFallingDown` 0x8D · `WasFallingDown` 0x8E · `DiesOnImpact` 0x8F ·
`IsAlive` 0x90 · `LastLayer` 0x94 · `IsInLogic` 0x98 · `IsDrawnThisFrame` 0x99 ·
`Location_X/Y/Z` 0x9C/0xA0/0xA4 · `LineTrailer` 0xA8.

New datatypes: `CoordStruct` (12 B, three ints — proven by `GetCoords` 0x005F65A4 and
`Set_Raw_Coords` 0x005F6944 copying exactly three contiguous dwords from this+0x9C) and
`VocHandle` (16 B: `void *pEvent`, `undefined4 dwEventStamp1/2` compared against the event's
+0x138 and +0x24, `undefined4 dwPoolTag` holding the constant 0x0087E294 — typed as a dword,
not a pointer, because it is a sentinel that is never dereferenced).

### Holes — recorded, not guessed

| Offset | What was tried | Why it stays a hole |
|---|---|---|
| 0x18 | Pointer-ness PROVEN (`ObjectClass__Load` swizzles it at 0x005F5EBE). Complete non-truncated program-wide `mov` sweep (8070 matches, 1155 non-stack) plus a full `cmp` sweep across every derived class range. | Zero readers or writers beyond the constructor's zeroing. `FactoryClass__Load` and `ScriptClass__Load` do not swizzle it, so only ObjectClass persists it. Applied as `void *` with an honest name; role UNKNOWN. Likely a TS-era attachment slot that YR still serializes and never populates. |
| 0x1C | AddRef/Release read (both `MOV EAX,1; RET 4`); program-wide INC/DEC sweeps; complete `mov` sweep (6807 matches, 909 non-stack). | Only proven property is survival across the raw overwrite in `AbstractClass__Load` (0x004103BE / 0x004103D0). Value is always 0 in stock play. |
| 0x24, 0x28 | Complete non-truncated `mov` sweeps (5508 / 5229 matches). The one promising `CMP word ptr [ESI+0x24]` at 0x00425E23 was chased and resolves to a CellClass receiver (ESI from the cell lookup at 0x00425DCB). | Constructor zeroing is the only access. No field applied. |
| 0x4C, 0x60 | 2326 and 1650 candidate sites enumerated and each resolved to another receiver. | Genuine unused slots — and NOT artifacts of a mis-sized VocHandle, since that type is 16 bytes by its own accessors. No field applied. |
| 0x78 | Two independent untruncated scans (`cmp` and `test`, 1,159,317 instructions) plus a 1098-site `mov` enumeration. | Write-once, never read. `Layer` refuted; see above. |
| 0x7C | Complete `mov` sweep (745 matches) and a full `cmp` sweep. | Write-once, never read. |
| 0x15–0x17, 0x21–0x23, 0x85–0x87, 0x91–0x93, 0x9A–0x9B | Per-offset binary-wide operand scans; every hit resolves to a stack local or another class (0x9B has **zero** matches anywhere in the binary). | True alignment padding. Left undefined rather than invented. |

Partial rows, honestly labelled: `IsInLogic` 0x98 — the register/unregister pair at
0x0055BAA0 / 0x0055BAE0 and the destructor's guarded removal are proven, but the container at
0x0087F778 is not independently verified as the Logic list. `LineTrailer` 0xA8 — lifecycle
proven (allocated in `Unlimbo`, freed and nulled in the destructor, zeroed rather than
restored on load, so runtime-only); the *identity* rests on the pre-existing
`LineTrail__DetachFromOwner` label, not on that function's own bytes.
`Location_X` vs `Location_Y` ordering follows from coord[0]/coord[1] becoming the first and
second cell words at 0x005F59C3 / 0x005F59DA; the axis sign convention was not re-derived.

### Functions typed (24) and receivers (20)

Every argument count comes from the callee's own RET immediate, never from a rendered
callsite. A fresh critic re-read every return site: no function has mixed RET immediates.

`AI` 0x005F3E70 (slot +0x5C) · `Mark` 0x005F5850 (+0x124) · `Limbo` 0x005F4D30 (+0xD4) ·
`Unlimbo` 0x005F4EC0 (+0xD8) · `UnInit` 0x005F65F0 (+0xF8) · `GetCoords` 0x005F65A0 (+0x48) ·
`Set_Raw_Coords` 0x005F6940 (+0x1B4) · `GetHeight` 0x005F5F40 (+0x1C8) · `GetCoordZ`
0x005F5F30 (+0x1D0) · `IsDead` 0x005F6690 (+0x44) · `MarkNeedsRedraw` 0x005F4D10 (+0x134) ·
`Deselect` 0x005F44A0 (+0x150) · `Select` 0x005F4520 (+0x14C) · `DetachParachute` 0x005F6DA0
(+0x60) · `PointerExpired` 0x005F5230 (+0x28) · `Destroy` 0x005F5280 (+0xDC) · `InWhichLayer`
0x005F4260 (+0x78) · `Set_Custom_Sound` 0x005F6CB0 (non-virtual) · `ComputeCRC` 0x005F6250
(+0x34) · `Paradrop` 0x005F5940 (+0xE8) — all 20 with `ObjectClass *` receivers, each proven
by incoming-ECX dataflow.

**Four prototypes applied with the receiver deliberately left UNTYPED**, because their bodies
never read incoming ECX and membership therefore cannot be proven from the body alone:
`Mark_Put` 0x005F60A0 (+0xF0), `Mark_Remove` 0x005F6120 (+0xF4), `What_Action_OnCell`
0x005F4250 (+0x70), `What_Action_OnObject` 0x005F4240 (+0x74).

`ObjectClass__Unlimbo` deserves a specific note: **it takes two stack args, not the one the
decompiler renders.** `RET 0x8` at both 0x005F5219 and 0x005F522C; the ObjectClass body reads
only arg1 and overrides use arg2. Sixteen sampled callsites all push two, with arg2 taking the
facing values 0x00 / 0x60 / 0x80.

### Label corrections applied (8, each logged with an evidence plate comment)

| Address | Old name | New name | Why the old name is refuted |
|---|---|---|---|
| 0x005F4250 | `ObjectClass__Limbo` | `ObjectClass__What_Action_OnCell` | `XOR EAX,EAX; RET 0xC` at vtable +0x70. Caller 0x00417DC8 converts leptons to a cell then compares the result against action codes; 0x004AE8BA feeds it straight into the paired Clicked_Action virtual at +0x140. A return-zero stub cannot be Limbo. |
| 0x005F4240 | `ObjectClass__Unlimbo` | `ObjectClass__What_Action_OnObject` | `XOR EAX,EAX; RET 0x8` at +0x74. `AircraftClass__What_Action` 0x00417CC0 has exactly one DATA xref — AircraftClass vtable +0x74 — and itself delegates to +0x70. Consumer 0x00417BE1 dispatches the result through a 16-way action jump table. |
| 0x005F6250 | `ObjectClass__Save` | `ObjectClass__ComputeCRC` | IPersistStream::Save is slot **+0x18**, proven from four untouched labels (BuildingClass 0x00454190, CellClass 0x00483C10, BombClass 0x00438BD0, AircraftClass 0x0041B5C0); ObjectClass's +0x18 is the shared return-zero stub 0x004C9150. This body is at +0x34 and feeds a table-driven CRC-32 (table 0x0081F7B4, head `00000000 77073096 EE0E612C 990951BA`, reflected poly 0xEDB88320). |
| 0x005F5940 | `ObjectClass__Unlimbo` (duplicate) | `ObjectClass__Paradrop` | Sits at +0xE8 and *calls* the real unlimbo through +0xD8 with facing 0x80, then builds the parachute anim from Rules+0xBB8/+0xBBC into +0x88. |
| 0x005F4D30 | `ObjectClass__Conceal` | `ObjectClass__Limbo` | Slot +0xD4 holds `TechnoClass__Limbo` (0x006F6AC0) and `BuildingClass__Limbo` (0x00445880) — pre-existing labels. The body's defining state write is InLimbo 0x81. |
| 0x005F4EC0 | `ObjectClass__Reveal` | `ObjectClass__Unlimbo` | Slot +0xD8 holds `TechnoClass__Unlimbo` (0x006F6CA0) and `BuildingClass__Unlimbo` (0x00440580). "Reveal" also collides with the unrelated live shroud family (`MapClass__RevealShroud`, `CellClass__RevealShroudFlags`, `TechnoClass__ReReveal`). |
| 0x00405D40 | `AnimClass__Detach` | `VocHandle__StopAndClear` | Touches no AnimClass field, vtable, or method; its entry guard is `CMP dword [EAX+0xC],0x87E294`, the tag `VocHandle__Init` writes. Called on ObjectClass+0x3C from `Limbo` at 0x005F4D81. |
| 0x005F6CB0 | `FUN_005f6cb0` | `ObjectClass__Set_Custom_Sound` | New label on an unnamed function; writes +0x64 and stops the +0x50 handle on the -1 sentinel. Callers 0x006DE883, 0x006E1A1A. |

`AbstractClass__ComputeCRC` 0x00410410 was NOT renamed — an investigator misreported its
current name as `AbstractClass__Save`; the live database already had it right. A rename was
applied on that false premise and immediately reverted, net zero change. Lesson folded into
the method: verify the current name from the live database before every rename.

A pre-existing plate comment on 0x005F4D30 asserted "detach +0x3C animation". That is wrong —
0x005F39C3 initialises +0x3C via `VocHandle__Init` — and it is the likely origin of the
refuted `AnimClass__Detach` label. Comment corrected.

### Critic pass

Three fresh read-only agents re-verified every applied row from raw bytes without the
applier's reasoning.

- **Prototypes/slots:** 24 of 24 arities confirmed at every return site; 21 of 21 vtable
  bindings confirmed; all 20 receiver typings confirmed and all four withheld ones confirmed
  as genuinely ECX-free. **One real defect:** `Destroy`'s second parameter had been applied as
  `skipDeselect` with inverted polarity — 0x005F52B0 `JNZ` on a nonzero argument jumps
  directly to the Deselect call, so nonzero *forces* it. Corrected to `forceDeselect`.
- **Labels:** 6 of 6 renames upheld on independently derived evidence. The critic also
  refuted the applier's decision to leave `Reveal`/`Conceal` alone; the applier re-verified
  the slot evidence and applied the two further renames, which are in the table above.
- **Struct fields:** all 30 rows upheld, including all three contested overrides and the
  0x1C demotion. The critic raised three further refutations — `Dirty` 0x20, `BombVisible`
  0x68, `EstimatedHealth` 0x70 — on the grounds that no reader exists. **All three are false
  negatives**, caused by its own mnemonic-filtered scan (which it flagged as a limitation).
  Readers were re-read directly: 0x20 by `IsDirty` at 0x00410456 (`MOV DL,byte [ECX+0x20]`,
  vtable +0x10, with +0x18=Save independently pinning the IPersistStream slot order); 0x68 at
  0x006F51B0, gated together with `AttachedBomb` 0x38 before the bomb is drawn; 0x70 at
  0x006F9F6E, where it is clamped down to Health, and at 0x006FE622 `SUB dword [EDI+0x70],EAX`
  on the *target* — the pre-deduction of in-flight damage that stops several attackers piling
  onto one doomed unit. Rows kept.

Refutation rate for the class: 1 genuine defect (`Destroy`'s parameter name) across 60 applied
rows = 1.7%, well under the 10% tripwire. One further correction the applier had wrongly
declined (`Limbo`/`Unlimbo`).

### Port-facing notes

- `FallRate` 0x2C clamps against RulesClass+0x7B8 = **ParachuteMaxFallRate** (key string
  0x0083C83C) and RulesClass+0x7BC = **NoParachuteMaxFallRate** (key 0x0083C824), selected by
  `IsAnimAttached` 0x84. This is the one row here that changes what a player sees on every
  fall.
- `ObjectClass__ComputeCRC` folds `NeedsRedraw` 0x80 and `IsSelected` 0x83 into the sync
  checksum **only** when `g_GameMode == 0 or 5`, and never folds `EstimatedHealth` 0x70. Any
  VERA mirror of this checksum has to reproduce that mode-dependent field set.
- `EstimatedHealth` 0x70 is a mechanism, not a cache: attackers pre-deduct their in-flight
  damage from the target's copy. Replacing it with a constant would visibly change focus-fire
  spread.
- 0x18, 0x1C, 0x24, 0x28, 0x4C, 0x60, 0x78 and 0x7C have no reader anywhere in the binary. A
  Rust `ObjectClass` equivalent can omit all eight.
