# Damage Math — Complete Ghidra Decompilation Report

Source: Direct decompilation of gamemd.exe via Ghidra MCP
Key functions: `0x00489180`, `0x006fdb80`, `0x005f5390`, `0x00701900`, `0x00489280`,
`0x0050bd30`, `0x006f3330`, `0x006fdd50`

> **2026-07-13 active-binary correction (supersedes conflicting Fire_At and
> Verses parser summaries below):** `disassemble_function(address="0x006fdd50",
> program="gamemd.exe")` proves ordinary `Damage <= 0` skips country/per-unit/
> veteran scaling (`0x006fe32f..0x006fe331`); positive damage groups
> `(house+0x188 * techno+0x160) * integer weapon Damage` before one `Math__ftol`
> (`0x006fe33d..0x006fe34d`). The Wave/special branch stores zero and rejoins at
> the containment stages rather than returning (`0x006fe328`,
> `0x006fe3df..0x006fe455`). The three later stages are civilian garrison, tank
> bunker, and open-topped, not the older deploy/gattling labels.
> `disassemble_function(address="0x0075d590", program="gamemd.exe")`,
> `read_memory(address="0x00847c40", length=128, program="gamemd.exe")`,
> `decompile_function(address="0x00528a10", program="gamemd.exe")`, and
> `disassemble_function(address="0x007caf30", program="gamemd.exe")` prove the
> Verses reader uses a bounded 0x80-byte ReadString result, parses the missing-key
> eleven-`100%%` fallback, skips parsing for present trimmed-empty input, loops
> exactly 11 times with native `strtok` empty-token collapse, and faults via
> `strchr(NULL, '%')` when a nonempty list exhausts tokens early.

---

## 1. Master Damage Formula: FUN_00489180 (WarheadTypeClass__GetDamage)

**Address:** `0x00489180`
**Signature:** `int __fastcall(int damage, WarheadTypeClass* wh, int armorType, int distance)`
- ECX = `damage` (raw weapon damage)
- EDX = `wh` (WarheadTypeClass pointer)
- Stack arg 1 = `armorType` (0-10, from target's TypeClass+0x9C)
- Stack arg 2 = `distance` (in leptons, distance from impact point to target)

### Early-out conditions

```
if (damage == 0) return 0;
if (ScenarioClass_flags & 0x20) return 0;   // bit 5 of a byte at offset 0 of *g_ScenarioClass_Instance
if (wh == NULL) return 0;
```

(corrected 2026-07-18: label was "GameOptions"; `disassemble_function(0x00489180)` shows
`MOV EAX,[0x00a8b230]; TEST byte ptr [EAX],0x20` — a dereference of the same global
(`0x00a8b230`) that `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md` independently identified as
`g_ScenarioClass_Instance` (tested there as `*g_ScenarioClass_Instance & 0x80` for
TiberiumSpreads). The exact INI-level meaning of bit 0x20 on this byte was not traced this
session — the early-out mechanism (address, dereference, bitmask, effect) is confirmed;
the specific "GameOptions" name is not — INFERENCE_HARDENED)

### Negative damage (healing) path

```
if (damage < 0):
    if (distance < 8):
        return damage;    // point-blank (< 8 leptons from impact): allow healing
    else:
        return 0;         // outside 8-lepton radius: block healing
```

Verified from assembly at `0x004891af–0x004891bd`: the check is on `param_4` (distance,
in leptons), not on armor type. `CMP EDI,0x8; SETGE AL; DEC EAX; AND EAX,ESI` produces
`0` when distance ≥ 8 and returns the (negative) damage otherwise. This gates AoE healing
to a narrow radius around the impact point; it does not discriminate by armor type.

### Positive damage: distance-based falloff

```
baseDamage = (float)damage;
percentAtMaxDamage = baseDamage * wh->PercentAtMax;         // wh+0x12C (float)
cellSpread_leptons = ftol(wh->CellSpread * 256.0);          // wh+0x124 (float), 256 leptons/cell

if (percentAtMaxDamage != baseDamage && cellSpread_leptons != 0):
    // Linear falloff from 100% at center to PercentAtMax at edge
    falloffDamage = percentAtMaxDamage + (baseDamage - percentAtMaxDamage) * (cellSpread_leptons - distance) / cellSpread_leptons;
    damage = ftol(falloffDamage);
```

The key formula in plain math:

```
effectiveDamage = percentAtMax * base + (1 - percentAtMax) * base * max(0, (cellSpread - dist)) / cellSpread
```

Or equivalently:

```
t = clamp(distance / cellSpread_leptons, 0, 1)
effectiveDamage = base * lerp(1.0, PercentAtMax, t)
```

### Verses (armor) multiplier

```
if (damage <= 0): damage = 0;                     // clamp negative to zero
versed_damage = ftol((float)damage * wh->Verses[armorType]);   // wh+0xA0 + armorType*8 (double[11])
```

### MaxDamage cap

```
if (versed_damage >= Rules->MaxDamage):            // Rules+0x16C8 (ctor fallback 1000; stock 10000)
    return Rules->MaxDamage;
return versed_damage;
```

### Complete formula (single expression)

```
raw = weapon.Damage
t = distance / (CellSpread * 256.0)

if t >= 1.0:
    spreadDamage = raw * PercentAtMax
else:
    spreadDamage = raw * PercentAtMax + raw * (1.0 - PercentAtMax) * (1.0 - t)
    // = raw * (PercentAtMax + (1 - PercentAtMax) * (1 - t))
    // = raw * lerp(1.0, PercentAtMax, t)

finalDamage = ftol(ftol(spreadDamage) * Verses[armorType])
finalDamage = min(finalDamage, MaxDamage)
```

**Note:** The function uses two `ftol()` conversions — first after the distance falloff to get an
integer spread-adjusted damage, then that integer is converted back to float, multiplied by the
Verses double, and truncated to int again. This double-truncation is a subtle behavioral detail.

### Worked example

100-damage weapon, Verses = 50% against Heavy armor (type 2), CellSpread = 1.0,
PercentAtMax = 0.25, distance = 128 leptons (half of 256 = half CellSpread):

```
cellSpread_leptons = 1.0 * 256.0 = 256
percentAtMaxDmg = 100 * 0.25 = 25.0
baseDamage = 100.0

t = 128 / 256 = 0.5
falloffDamage = 25.0 + (100.0 - 25.0) * (256 - 128) / 256
             = 25.0 + 75.0 * 0.5
             = 62.5
damage_after_spread = ftol(62.5) = 62

versed_damage = ftol(62.0 * 0.5) = ftol(31.0) = 31

Result: 31 damage
```

---

## 2. WarheadTypeClass Data Layout (verified offsets)

| Offset | Type | Field | INI Key |
|--------|------|-------|---------|
| 0x98 | double | Deform | `Deform` |
| 0xA0 | double[11] | Verses | `Verses` (11 armor types) |
| 0xF8 | double | ProneDamage | `ProneDamage` |
| 0x100 | int | DeformThreshold | `DeformThreshhold` |
| 0x114 | DynamicVector | AnimList | `AnimList` |
| 0x120 | int | InfDeath | `InfDeath` |
| 0x124 | float | CellSpread | `CellSpread` (in cells, default 0) |
| 0x128 | float | CellInset | `CellInset` |
| 0x12C | float | PercentAtMax | `PercentAtMax` (default 1.0) |
| 0x130 | bool | CausesDelayKill | `CausesDelayKill` |
| 0x134 | int | DelayKillFrames | `DelayKillFrames` |
| 0x138 | float | DelayKillAtMax | `DelayKillAtMax` |
| 0x13C | float | CombatLightSize | `CombatLightSize` |
| 0x140 | ParticleSystemTypeClass* | Particle | `Particle` |
| 0x144 | bool | Wall | `Wall` |
| 0x145 | bool | WallAbsoluteDestroyer | `WallAbsoluteDestroyer` |
| 0x146 | bool | PenetratesBunker | `PenetratesBunker` |
| 0x147 | bool | ? | (unknown bool) |
| 0x148 | bool | Tiberium | `Tiberium` |
| 0x149 | bool | IsNonDamaging | (computed: Verses[4]==0 && Verses[6]==0 — medium and wood, verified at 0x0075DE5A/0x0075DE6D) |
| 0x14A | bool | Sparky | `Sparky` |
| 0x14B | bool | Sonic | `Sonic` |
| 0x14C | bool | Fire | `Fire` |
| 0x14D | bool | Conventional | `Conventional` |
| 0x14E | bool | Rocker | `Rocker` |
| 0x14F | bool | DirectRocker | `DirectRocker` |
| 0x150 | bool | Bright | `Bright` |
| 0x154 | bool | EMEffect | `EMEffect` |
| 0x155 | bool | MindControl | `MindControl` |
| 0x156 | bool | Poison | `Poison` |
| 0x157 | bool | IvanBomb | `IvanBomb` |
| 0x158 | bool | ElectricAssault | `ElectricAssault` |
| 0x159 | bool | Parasite | `Parasite` |
| 0x15A | bool | Temporal | `Temporal` |
| 0x15B | bool | IsLocomotor | `IsLocomotor` |
| 0x15C | CLSID | Locomotor | `Locomotor` (16 bytes) |
| 0x16C | bool | Airstrike | `Airstrike` |
| 0x16D | bool | Psychedelic | `Psychedelic` |
| 0x16E | bool | BombDisarm | `BombDisarm` |
| 0x170 | int | Paralyzes | `Paralyzes` |
| 0x174 | bool | Culling | `Culling` |
| 0x175 | bool | MakesDisguise | `MakesDisguise` |
| 0x176 | bool | NukeMaker | `NukeMaker` |
| 0x177 | bool | Radiation | `Radiation` |
| 0x178 | bool | PsychicDamage | `PsychicDamage` |
| 0x179 | bool | AffectsAllies | `AffectsAllies` |
| 0x17A | bool | Bullets | `Bullets` |
| 0x17B | bool | Veinhole | `Veinhole` |
| 0x17C | int | ShakeXlo | `ShakeXlo` |
| 0x180 | int | ShakeXhi | `ShakeXhi` |
| 0x184 | int | ShakeYlo | `ShakeYlo` |
| 0x188 | int | ShakeYhi | `ShakeYhi` |
| 0x18C | DynamicVector | DebrisTypes | `DebrisTypes` |
| 0x1A8 | DynamicVector | DebrisMaximums | `DebrisMaximums` |
| 0x1C4 | int | MaxDebris | `MaxDebris` |
| 0x1C8 | int | MinDebris | `MinDebris` (clamped >= 0, MaxDebris >= MinDebris) |

### Verses parsing

The `Verses` INI value is a comma-separated list of 11 values. Each value can be:
- A percentage like `"100%"` -> `atoi("100") * 0.01 = 1.0`
- A decimal like `"0.5"` -> strtod-family full-f64 prefix parse = 0.5

Missing-key fallback: eleven `100%%` tokens at `0x00847c40`, parsed through the
same 11-store loop. Present trimmed-empty input skips the loop and retains the
constructor's eleven 1.0 values. The source is truncated/forced-NUL in a
0x80-byte buffer before trim and `strtok`; a nonempty list with fewer than 11
post-`strtok` tokens faults instead of filling a default tail.

The 11 armor types (indexed 0-10) are: none, flak, plate, light, medium, heavy, wood,
steel, concrete, special_1, special_2.

---

## 3. Country-Specific Armor Multiplier: FUN_0050bd30

**Address:** `0x0050bd30`
**Signature:** `float __thiscall(HouseClass* this, ObjectClass* target)`

This function provides a **defender's country bonus** that modifies damage based on the
*target's* WhatAmI() type and the *defender's* house type. It is called in
TechnoClass::ReceiveDamage with `this->Owner` (the damaged unit's owner).

### Switch on target->WhatAmI()

| WhatAmI | Type | HouseTypeClass Offset | INI Key | Default |
|---------|------|-----------------------|---------|---------|
| 3 | Infantry | HouseType+0x108 | `ArmorInfantryMult` | 1.0 |
| 7 | Unit (ground) | HouseType+0x10C | `ArmorUnitsMult` | 1.0 |
| 7 (locomotor==5) | Unit (flying) | HouseType+0x110 | `ArmorAircraftMult` | 1.0 |
| 16 (0x10) | Aircraft | HouseType+0x100 | `ArmorDefensesMult` | 1.0 |
| 40 (0x28) | ? | HouseType+0x104 | `ArmorBuildingsMult` | 1.0 |
| default | other | constant 1.0 | (none) | 1.0 |

The multiplication happens as:
```
damage = ftol((float)damage * countryArmorMult)
```

**Note:** These are NOT on WarheadTypeClass. They are on **HouseTypeClass** (the country definition),
accessed via `HouseClass+0x34` (HouseTypeClass pointer). The offsets 0x100-0x110 here refer to
HouseTypeClass, not WarheadTypeClass (which has different fields at those offsets).
They modify the damage RECEIVED by units of that country. A value > 1.0 means the country
takes MORE damage to that unit type; < 1.0 means it takes less.

---

## 4. TechnoClass::ReceiveDamage (0x00701900) — Full Damage Pipeline

This is the main entry point for applying damage to any Techno (unit, infantry, building, aircraft).
Called with 7 stack parameters via vtable offset 0x16C.

**Signature:**
```c
int __thiscall TechnoClass::ReceiveDamage(
    int* pDamage,           // [in/out] damage amount, modified in place
    int distance,           // distance from impact in leptons
    WarheadTypeClass* wh,   // warhead doing the damage
    TechnoClass* attacker,  // who fired (can be NULL)
    bool ignoreDefenses,    // skip armor/immunities (e.g. from triggers)
    bool unknown,           // unused parameter
    HouseClass* sourceHouse // house responsible for the kill
);
// Returns: damage result code (0=nodamage, 1=damaged, 2=yellow, 3=red, 4=killed, 5=already_dead)
```

### Step-by-step damage modification (for positive damage, ignoreDefenses=false)

#### Step 1: Country armor multiplier

```c
float countryMult = this->Owner->HouseType->GetArmorMultForType(this);
// FUN_0050bd30: reads ArmorInfantryMult, ArmorUnitsMult, etc.
*pDamage = ftol((float)*pDamage * countryMult);
```

#### Step 2: Veterancy firepower bonus (DEFENDER's armor, counterintuitively)

The code checks the DAMAGED unit's veterancy level and whether its type has the
ARMOR veteran/elite ability enabled. If so, it reduces the damage.

```c
float vetLevel = this->Veterancy;   // float at some offset

bool isVeteran = (vetLevel >= 1.0 && vetLevel < 2.0);   // FUN_0074ff90
bool isElite   = (vetLevel >= 2.0);                       // FUN_00750010

if (isVeteran || isElite):
    TechnoTypeClass* type = this->GetType();

    bool hasVetArmor = false;
    if (isVeteran && type->VeteranAbilities.Armor):       // TypeClass+0x29D
        hasVetArmor = true;
    if (isElite && (type->EliteAbilities.Armor || type->EliteAbilities.DVARMOR)):
        // TypeClass+0x29D (vet armor flag) or TypeClass+0x2AF (elite armor flag)
        hasVetArmor = true;

    if (hasVetArmor):
        *pDamage = ftol((float)*pDamage / VeteranArmor);
        // VeteranArmor = Rules+0x???? (default 1.5 from rulesmd.ini)
        // This DIVIDES damage, making the unit tougher
```

**IMPORTANT:** The VeteranArmor INI comment says "damage is divided by this", confirmed by
the binary. A VeteranArmor=1.5 means veteran units effectively have 50% more health.

#### Step 3: Minimum damage floor

```c
if (*pDamage < 1):
    *pDamage = 1;       // minimum 1 damage for positive hits
```

#### Step 4: NotHuman immunity check

```c
if (attacker != NULL && this->GetType()->NotHuman):   // TypeClass+0xC8C
    if (attacker->GetType() == this->GetType() && this->Owner == attacker->Owner):
        *pDamage = 0;
        return 0;  // same-type friendly units can't damage NotHuman types
```

#### Step 5: Iron Curtain / Force Shield check

```c
if (this->IsIronCurtained() && !ignoreDefenses && damage > 0):
    // play iron curtain hit animation
    *pDamage = 0;
    return 0;
```

#### Step 6: Shield (ForceShield) check

```c
if (this->IsForceShielded() && !ignoreDefenses):
    *pDamage = 0;
    return 0;
```

#### Step 7: ImmuneToPsionics / ImmuneToRadiation / ImmuneToPoison warhead checks

```c
if (wh != NULL):
    if (wh->Radiation && type->ImmuneToRadiation):     // wh+0x177, type+0xD37
        *pDamage = 0; return 0;
    if (wh->PsychicDamage && type->ImmuneToPsionics):  // wh+0x178, type+0xD36
        *pDamage = 0; return 0;
    if (wh->Poison && type->ImmuneToPoison):            // wh+0x156, type+0xD3B
        *pDamage = 0; return 0;
    if (!wh->AffectsAllies && attacker && IsAllied(attacker->Owner, this->Owner)):
        *pDamage = 0; return 0;                         // wh+0x179
```

#### Step 8: Psychedelic (mind-control-like warhead) handling

```c
if (wh->Psychedelic):   // wh+0x16D
    if (IsAllied(this->Owner, sourceHouse)):
        return 0;
    if (type->ImmuneToPsionics):   // type+0xD35
        return 0;
    if (this is a building):
        return 0;
    // Apply the Verses formula via FUN_00489180
    *pDamage = FUN_00489180(weapon->Damage, wh, armorType, 0);
    // Store as "warping damage" and begin warp state
    return 1;
```

#### Step 9: Pass to ObjectClass::ReceiveDamage

After all the TechnoClass-level modifications, control falls through to
`ObjectClass::ReceiveDamage` which performs the Verses calculation (if ignoreDefenses
was false) and the actual health subtraction.

---

## 5. ObjectClass::ReceiveDamage (0x005f5390) — Health Modification

### Step 1: Verses application (if not ignoreDefenses)

```c
if (!ignoreDefenses):
    int armorType = this->GetType()->Armor;       // TypeClass+0x9C
    *pDamage = FUN_00489180(*pDamage, wh, armorType, distance);
```

### Step 2: Building minimum damage

```c
if (this is a Building && !BuildingTypeClass->CanC4):  // TypeClass+0x1577
    if (*pDamage < 1):
        *pDamage = 1;    // buildings without CanC4=yes always take at least 1 damage
```

### Step 3: Zero damage early out

```c
if (*pDamage == 0):
    return 0;   // no damage dealt
```

### Step 4: Negative damage (healing)

```c
if (*pDamage < 0):
    this->Health -= *pDamage;   // subtracting negative = adding
    if (this->Health > this->GetType()->Strength):
        this->Health = this->GetType()->Strength;   // cap at max
    return 0;
```

### Step 5: Cap damage to remaining health

```c
if (*pDamage >= currentHealth):
    *pDamage = currentHealth;   // can't deal more than remaining HP
```

### Step 6: Health condition tracking

```c
int maxHealth = this->GetType()->Strength;     // TypeClass+0xA0

// Check transition to Yellow health
if (currentHealth >= maxHealth/2 && (currentHealth - *pDamage) < maxHealth/2):
    result = 2;   // transitioning to yellow

// Check transition to Red health
double redThreshold = (double)maxHealth * Rules->ConditionRed;   // Rules+0x1708
if ((double)currentHealth > redThreshold && (double)(currentHealth - *pDamage) < redThreshold):
    result = 3;   // transitioning to red
```

### Step 7: Subtract health

```c
this->Health = currentHealth - *pDamage;
```

### Step 8: NUKE/VeinholeMonster survival (special types only)

For WhatAmI == 0x0F (VeinholeMonster?) with special conditions:

```c
if (this->Health <= 0 && WhatAmI() == 0x0F && !ignoreDefenses):
    if (special_flag && !already_nuked):
        // Animate explosion
        // Set health to: ftol(maxHealth * 0.25)
        // Minimum health = 1
        // Enter nuked state
        result = 3;
```

### Step 9: Death handling

```c
if (this->Health == 0):
    if (sourceHouse == 0 || (attacker != 0 && sourceHouse == attacker->Owner)):
        this->Killed(attacker);        // vtable+0xE0
    else:
        this->KilledByHouse(house);    // vtable+0xE4
    this->Destroy(true);               // vtable+0xDC
    result = 4;
```

### Step 10: Trigger events

Various map trigger events fire based on damage state transitions:
- Event 0x26: First hit (was at full health)
- Event 0x27: Health goes below 50% (yellow)
- Event 0x28: Health goes below ConditionRed threshold (red)
- Event 0x29: Any damage dealt
- Event 0x2A: Goes yellow (with attacker)
- Event 0x2B: Goes red (with attacker)
- Event 0x2C: Attacked (with attacker info)

---

## 6. Apply_area_damage (0x00489280) — Area/Splash Damage Distribution

**Address:** `0x00489280`
**Signature:**
```c
bool __fastcall Apply_area_damage(
    CoordStruct* impactCoord,     // impact location in leptons
    int baseDamage,               // raw damage from weapon
    TechnoClass* attacker,        // who fired
    WarheadTypeClass* wh,         // warhead
    bool allowTiberiumChain,      // whether to reduce tiberium in affected cells
    HouseClass* sourceHouse       // responsible house
);
```

### Distance calculation (max radius in leptons)

```c
int maxRadius_leptons = ftol(wh->CellSpread * 256.0);    // wh+0x124
```

### Target collection phase

The function collects all potential targets within the CellSpread radius:

1. **Airborne units:** If impact Z > ground height, iterates objects in the impact cell's
   air layer. Calculates 3D distance from impact to each airborne object.

2. **Ground/cell iteration:** Iterates cells in a square pattern using pre-computed cell
   offset tables (`DAT_00abd490/DAT_00abd492`). The number of cells to check comes from
   `DAT_007ed3d0[spread_index]` where `spread_index = ftol(CellSpread)`.

3. For each object in each cell (via the cell's object linked list at cell+0xE4 or cell+0xE8):
   - Calculates distance from impact to object center
   - For buildings (WhatAmI == 6): uses special distance from cell center, and if the
     building is directly at the impact cell with height difference > 2*CellHeight, uses
     adjusted distance subtracted by 2*CellHeight
   - For other objects: uses 3D distance via sqrt(dx^2 + dy^2 + dz^2)
   - Stores `{object*, distance}` pairs in a dynamic array

### Target filtering

Certain targets are skipped:
- The attacker itself (unless the warhead is the `C4Warhead` from Rules+0xFAC, or the
  target has IsSelfHealing enabled at TypeClass+0xCA0)
- Units in the `ProtectedFromAOE` list (Rules+0xB40, count at Rules+0xB4C) — these are
  type classes immune to area damage
- Dead objects (Health <= 0, or IsAlive == false)
- Objects with `InLimbo` flag set (at object+0x81)

### Bridge under-fire detection

If CellSpread > 0.5 and the impact cell is a bridge, objects on the bridge at the
impact cell are checked for whether they're infantry (WhatAmI == 1). If so, and they're
at ground level on the bridge (within 0x55 leptons), a `bVar5` flag is set indicating
"bridge infantry should take reduced damage."

### Damage application phase

For each collected target (sorted by distance):
```c
for each {object, distance} in targetList:
    if (!object->IsAlive || (WhatAmI == 6 && building_type->CanC4 == false)):
        skip;

    if (bVar5):   // bridge infantry check
        // Only damage objects that are on the bridge (infantry on bridge flag)
        if (object is not bridge infantry): skip;

    // Half distance for aircraft in flight
    if (WhatAmI == 2 && object->IsInAir()):
        distance = distance / 2;

    // Final eligibility: object must be alive, on map, health > 0, within radius
    if (object->Health > 0 && !object->InLimbo && distance <= maxRadius_leptons):
        object->ReceiveDamage(&baseDamage, distance, wh, attacker, false, false, sourceHouse);
```

**Critical finding:** Apply_area_damage passes the SAME `baseDamage` to every target's
ReceiveDamage, along with each target's individual `distance`. The distance-based falloff
is NOT calculated here — it happens inside FUN_00489180 (called from ObjectClass::ReceiveDamage).
Each target gets the full weapon damage, and the Verses function internally computes the
falloff based on that target's distance from the impact point.

### Post-damage effects

After applying damage to units, Apply_area_damage handles:
- **Tiberium/overlay destruction:** If warhead->Tiberium or warhead->Wall, reduces tiberium
  and destroys overlays in affected cells
- **Bridge destruction:** Checks bridge tiles and overlay indices. If damage is sufficient
  (random check against CellSpread intensity), calls `ApplyDamageToCell` to destroy bridge
  sections. Both low bridges (overlay 0x4A-0x63) and high bridges (overlay 0xCD-0xE6) are
  checked.
- **Sparky effect:** If warhead->Sparky and CellSpread > 0, pushes nearby units with a force
  proportional to CellSpread
- **IC barrel explosion:** If the impact cell has an IC overlay (offset+0x2B0 flag),
  triggers a chain reaction with a C4 warhead (Rules+0xFA8)

---

## 7. Pre-Fire Damage Estimation: FUN_006fdb80

**Address:** `0x006fdb80`
**Signature:** `int __thiscall(TechnoClass* attacker, TechnoClass* target, WeaponTypeClass* weapon)`

Called from `TechnoClass::Fire_At` (0x006fdd50) before creating the bullet projectile.
Used for the **EstimatedHealth overkill prevention** system — the engine pre-subtracts
estimated damage from the target's EstimatedHealth (at target+0x70) so multiple units
don't all target the same nearly-dead enemy.

### Estimation logic

```c
if (target == NULL || weapon->Damage <= 0 || weapon->CausesDelayKill || weapon->SomeFlag):
    return 0;

int estimatedDamage = weapon->Damage;   // weapon+0xA4

// Apply attacker's veterancy firepower bonus
estimatedDamage = ftol((float)estimatedDamage * vetFirepowerMult);

bool isVet = (attacker->Veterancy >= 1.0 && < 2.0);
bool isElite = (attacker->Veterancy >= 2.0);

if (isVet || isElite):
    TechnoTypeClass* aType = attacker->GetType();
    if ((isVet && aType->VeteranAbilities.Firepower) ||     // type+0x29E
        (isElite && (aType->EliteAbilities.Firepower ||      // type+0x29E
                     aType->EliteAbilities.ExtraFirepower))): // type+0x2B0
        estimatedDamage = ftol((float)estimatedDamage * VeteranCombat);
        // VeteranCombat = Rules (default 1.1)

// Apply country firepower bonus
float countryFirepowerMult = attacker->Owner->HouseType->FirePower;  // HouseType+0x00
estimatedDamage = ftol((float)estimatedDamage * countryFirepowerMult);

// Apply target's veterancy armor
if (isVet || isElite):
    TechnoTypeClass* tType = target->GetType();
    if ((isVet && tType->VeteranAbilities.Armor) ||
        (isElite && (tType->EliteAbilities.Armor || tType->EliteAbilities.DVARMOR))):
        estimatedDamage = ftol((float)estimatedDamage / VeteranArmor);

// Apply Verses (armor multiplier) with distance=0 (point blank estimate)
int armorType = target->GetType()->Armor;   // target type+0x9C
estimatedDamage = FUN_00489180(estimatedDamage, weapon->Warhead, armorType, 0);

return estimatedDamage;
```

### Usage in Fire_At

```c
// At address 0x6FE5C0 in TechnoClass::Fire_At:
if (!bullet->IsInaccurate && !weapon->IsAreaFire):
    AbstractClass* targetAbstract = this->Target;
    TechnoClass* targetTechno = (targetAbstract->IsATechno) ? targetAbstract : NULL;

    BulletClass* bullet = CreateBullet(...);
    if (targetTechno != NULL && bullet != NULL):
        int estimatedDmg = FUN_006fdb80(targetTechno, bullet->WeaponType);
        targetTechno->EstimatedHealth -= estimatedDmg;   // target+0x70
```

The EstimatedHealth is periodically synced back to actual Health, preventing the estimated
damage from drifting too far from reality.

---

## 8. TechnoClass::Fire_At (0x006fdd50) — Damage Modifiers at Fire Time

In Fire_At, the weapon damage is modified before being stored on the BulletClass:

```c
int damage = (wave_or_special) ? 0 : weapon->Damage; // weapon+0xA4

if (!wave_or_special && damage > 0) {
    damage = ftol((houseFirepower * unitFirepower) * damage);
    if (has_veteran_or_elite_firepower_ability) {
        damage = ftol(damage * VeteranCombat);
    }
}

// Still reachable for ordinary non-positive and special stored-zero damage:
if (civilian_garrison) damage = ftol(damage * OccupyDamageMultiplier);
if (tank_bunker)       damage = ftol(damage * BunkerDamageMultiplier);
if (open_topped)       damage = ftol(damage * OpenToppedDamageMultiplier);
```

These modified damage values are stored on the bullet and later passed to
Apply_area_damage or directly to ReceiveDamage.

---

## 9. Weapon Selection: TechnoClass::SelectWeaponAgainst (0x006f3330)

**Address:** `0x006f3330`
**Signature:** `int __thiscall TechnoClass::SelectWeaponAgainst(AbstractClass* target)`
**Returns:** weapon index (0 = Primary, 1 = Secondary, or gattling stages 0..N*2+1)

### Decision tree (simplified)

```
1. If unit is currently deploying and NoDeployedWeapon flag is NOT set:
   return CurrentWeaponNumber (whatever weapon was being used)

2. If unit only has Primary weapon (no Secondary):
   return 0

3. If Secondary weapon is LimboLaunch (deploy weapon):
   return 0

4. If target is NULL:
   return 0

5. GATTLING UNITS (type->IsGattling):
   weapon_index = CurrentGattlingStage * 2;
   if (target warhead is AA-capable && target->IsInAir()):
       return weapon_index + 1;
   return weapon_index;

6. NON-GATTLING with SECONDARY WEAPON:

   a. If Primary warhead has Airstrike flag (wh+0x16C — NOT MindControl;
      true MindControl is at wh+0x155, verified in WarheadTypeClass::ReadINI):
      - If target is a Building with CanC4=yes (TypeClass+0x1577) AND
        TypeClass+0x5EC or TypeClass+0x5ED is zero: return 1
      (the airstrike-to-secondary fallback. Exact semantics of the +0x5EC/+0x5ED
      conditions were not fully traced — consult 0x006F3330 before implementing.)

   b. If Primary weapon has OccupantCapturer flag and target is an OccupiableBuilding:
      - If not already mind-controlling and target is enemy: return 1
      (use secondary for direct damage)

   c. If Primary is deploy-only (DeployFire) and locomotion == hover:
      return 1

   d. If attacker is a Building and IsInternalGarrison:
      return 1

   e. If target is a Building and it's on a bridge and type allows NavalTargeting
      and the building type is immune to primary:
      return 1

   f. If attacker is Aircraft and is dogfighting:
      return 1

   g. If target is a CELL (terrain target):
      Special cell targeting logic:
      - If cell is a bridge or water, and unit has NavalGunboat=2:
        return 1
      - Otherwise: return 0

   h. VERSES-BASED SELECTION (the key logic):
      int targetArmor = target->GetType()->Armor;

      // Check if Secondary warhead has non-zero Verses for this armor type
      if (SecondaryWarhead->Verses[targetArmor] != 0.0):
          // Check if Primary warhead has ZERO Verses for this armor type
          if (PrimaryWarhead->Verses[targetArmor] == 0.0):
              return 1;  // Primary can't damage this armor, use Secondary

          // If target is on water or amphibious terrain
          bool isNavalTarget = (target->GetCell()->LandType == Water ||
                                target->GetCell()->LandType == Beach);
          if (!target->IsInAir()):
              isNavalTarget = isNavalTarget;

          if (!target->IsAlive && isNavalTarget):
              // Dead naval target: check weapon select override
              int override = this->GetNavalWeaponSelect(target);
              if (override != -1): return override;
          else:
              if (!target->IsInAir() && type->NavalGunboat == 2):
                  return 1;   // use secondary for naval targets

              // Check AA priority
              if (PrimaryWarhead->IsAntiAir && target->IsInAir()):
                  return 1;
```

### Key behavioral notes

- The engine prefers Primary (weapon 0) by default and only switches to Secondary when
  specific conditions are met
- Verses == 0.0 for the target's armor type means the weapon literally cannot damage that
  target, triggering a switch to the other weapon
- For Gattling weapons, the stage index doubles (stage 0 = weapons 0/1, stage 1 = weapons 2/3)
- MindControl weapons always use Secondary against buildings and immune targets
- Aircraft in dogfight mode always use Secondary

---

## 10. Veterancy System — Key Constants

### RulesClass veterancy fields (from rulesmd.ini [General])

| Field | Default | Effect |
|-------|---------|--------|
| VeteranRatio | 3.0 | Kill value ratio to promote |
| VeteranCombat | 1.1 | Firepower multiplier (multiply damage by this) |
| VeteranSpeed | 1.2 | Movement speed multiplier |
| VeteranArmor | 1.5 | Armor multiplier (divide damage by this) |

### TechnoTypeClass ability flags

Abilities are stored as byte arrays at:
- VeteranAbilities: TechnoTypeClass + 0x29C (base)
- EliteAbilities: TechnoTypeClass + 0x2AE (base)

The abilities are parsed from comma-separated ability names in INI. Relevant offsets:
- `+0x29D` / `+0x2AF`: ARMOR ability (Vet/Elite) (corrected 2026-07-18: was "FIREPOWER"; this row
  was swapped with the one below. `decompile_function(0x00701900)` — TechnoClass::ReceiveDamage's
  damage-divide check reads `*(char*)(type+0x29d)` (vet) and `*(char*)(type+0x2af)` (elite) for the
  target's own type, matching this doc's own Section 4 text (TypeClass+0x29D/+0x2AF armor flags) —
  OFFSET_RETYPED_WRONG)
- `+0x29E` / `+0x2B0`: FIREPOWER ability (Vet/Elite) (corrected 2026-07-18: was "ARMOR"; swapped
  with the row above. `decompile_function(0x006fdb80)` — FUN_006fdb80's attacker-side firepower
  multiplier check reads `*(char*)(type+0x29e)` (vet) and `*(char*)(type+0x2b0)` (elite), matching
  this doc's own Section 7 text (type+0x29E/+0x2B0 firepower flags) — OFFSET_RETYPED_WRONG)
  (Note: these are byte offsets from TechnoTypeClass base, NOT from the ability array start)

### Veterancy level thresholds (FUN_0074ff90 / FUN_00750010)

```c
bool IsVeteran(float vetLevel):
    return vetLevel >= 1.0f && vetLevel < 2.0f;

bool IsElite(float vetLevel):
    return vetLevel >= 2.0f;
```

Constants verified from binary:
- `_DAT_007e2ac8` = 1.0f (veteran threshold)
- `_DAT_007e37b4` = 2.0f (elite threshold)

---

## 11. Complete Damage Flow Summary

For a standard attack (non-special warhead, positive damage):

```
1. ATTACKER SIDE (in Fire_At, before bullet creation):
   damage = 0 for Wave/special, else weapon.Damage
   if ordinary damage > 0:
       damage = ftol((countryFirepowerMult * unitFirepowerMult) * damage)
       if vet/elite with FIREPOWER ability:
           damage = ftol(damage * VeteranCombat)
   if civilian_garrison: damage = ftol(damage * OccupyDamageMultiplier)
   if tank_bunker:       damage = ftol(damage * BunkerDamageMultiplier)
   if open_topped:       damage = ftol(damage * OpenToppedDamageMultiplier)
   -> stored on BulletClass

2. IMPACT (in Apply_area_damage or direct ReceiveDamage):
   -> baseDamage and distance passed to each target's ReceiveDamage

3. TARGET SIDE (in TechnoClass::ReceiveDamage):
   damage *= defenderCountryArmorMult      (ArmorInfantryMult etc, default 1.0)
   if (vet/elite with ARMOR ability):
       damage /= VeteranArmor              (default 1.5)
   damage = max(damage, 1)                 (minimum 1 for positive)
   -> check immunities (IronCurtain, ForceShield, Radiation, Psionic, Poison)
   -> check AffectsAllies

4. ARMOR APPLICATION (in ObjectClass::ReceiveDamage -> FUN_00489180):
   distance falloff:
       t = distance / (CellSpread * 256)
       damage = damage * lerp(1.0, PercentAtMax, t)
   verses multiplier:
       damage *= Warhead.Verses[target.Armor]
   cap:
       damage = min(damage, MaxDamage)      (constructor fallback 1000; stock 10000)

5. HEALTH SUBTRACTION (in ObjectClass::ReceiveDamage):
   damage = min(damage, currentHealth)      (can't overkill)
   Health -= damage
   if (Health == 0): kill the unit
```

### Complete single-expression formula

```
finalDamage = min(
    ftol(
        ftol(
            ftol(
                ftol(
                    weapon.Damage
                    * countryFirepower
                    * vetCombatMult
                )
                * defenderCountryArmor
                / vetArmorDiv
            )
            * lerp(1.0, PercentAtMax, dist / (CellSpread * 256))
        )
        * Verses[armorType]
    ),
    MaxDamage
)
```

Where each `ftol()` represents an integer truncation boundary. The multiple truncation
steps mean the final result can differ from a single floating-point computation by
several points due to accumulated rounding.

---

## 12. Key Constants (verified from binary)

| Address | Value | Meaning |
|---------|-------|---------|
| 0x007e2224 | 256.0f | Leptons per cell (CellSpread * this = spread in leptons) |
| 0x007e2ac8 | 1.0f | Veteran threshold |
| 0x007e37b4 | 2.0f | Elite threshold |
| 0x007e3808 | 0.01 (double) | Percentage to fraction converter (for `100%` -> `1.0`) |
| 0x007e5168 | 0.5f | CellSpread threshold for bridge checks in Apply_area_damage |
| 0x007ef250 | 0.25 (double) | NUKE survival health fraction (VeinholeMonster) |
| Rules+0x16C8 | 10000 (int) | MaxDamage cap |
| Rules+0x1700 | double | ConditionYellow threshold ratio |
| Rules+0x1708 | double | ConditionRed threshold ratio |
| Rules+0xFA8 | WarheadTypeClass* | C4Warhead (used for IC barrel chain reactions) |
| Rules+0xFAC | WarheadTypeClass* | C4Warhead (duplicate reference for area damage self-check) |
| Rules+0xFF0 | WarheadTypeClass* | Another special warhead reference |
