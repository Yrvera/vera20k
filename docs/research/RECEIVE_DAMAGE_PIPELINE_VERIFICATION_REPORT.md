# ReceiveDamage Pipeline — Verification Report

**Date:** 2026-04-04
**Method:** Live Ghidra MCP decompilation of gamemd.exe, cross-referenced against
existing docs (DAMAGE_MATH, RECEIVE_DAMAGE, WARHEAD_DETONATE reports).
**Confidence:** HIGH — all key addresses and offsets verified directly from binary.

> **2026-07-13 Fire_At correction:**
> `disassemble_function(address="0x006fdd50", program="gamemd.exe")` proves the
> attacker summary below had stale labels/order. Ordinary `Damage <= 0` skips
> country/per-unit/veterancy (`0x006fe32f..0x006fe331`); positive damage groups
> `(house+0x188 * techno+0x160) * integer Damage` before one conversion
> (`0x006fe33d..0x006fe34d`). Wave/special stores zero and rejoins, and the later
> stages are civilian garrison → tank bunker → open-topped at
> `0x006fe3e3..0x006fe455`.

---

## 1. ReceiveDamage Entry Point — CONFIRMED

**TechnoClass::ReceiveDamage** at `0x00701900` (body 0x00701900–0x00702d21, 682 decompiled lines).
Already labeled in Ghidra.

### Parameters — CONFIRMED

```c
int __thiscall TechnoClass::ReceiveDamage(
    int*             pDamage,        // +0x04: pointer to damage (read/write)
    int              distance,       // +0x08: lepton distance from blast center
    WarheadTypeClass* warhead,       // +0x0C: warhead rules pointer
    TechnoClass*     attacker,       // +0x10: source of damage (nullable)
    bool             ignoreDefenses, // +0x14: skip armor/immunities
    bool             unused,         // +0x18: unused
    HouseClass*      sourceHouse     // +0x1C: house responsible for kill
);
```

From decompilation:
- `in_stack_00000004` = pDamage (int*)
- `in_stack_0000000c` = warhead (WarheadTypeClass*)
- `in_stack_00000010` = attacker (TechnoClass*)
- `in_stack_00000014` = ignoreDefenses (char/bool)
- `in_stack_0000001c` = sourceHouse (HouseClass*)

**Return value:** DamageState enum (0=Unaffected, 1=Damaged, 2=ConditionYellow,
3=ConditionRed, 4=NowDead, 5=PostMortem). **CONFIRMED** from all return paths in binary.

---

## 2. Damage Modification Chain — VERIFIED ORDER

The complete chain, verified step by step from binary decompilation:

### Step 1: Country Armor Multiplier (CONFIRMED)

**Function:** `HouseClass__GetArmorMultForType` at `0x0050bd30`
(previously mislabeled `HouseClass__GetBuildSpeedBonus` — **renamed in this session**).

Called via `this->Owner` (the damaged unit's house). Reads from `HouseTypeClass`
(accessed via `HouseClass+0x34`).

| WhatAmI | Type | HouseTypeClass Offset | INI Key |
|---------|------|-----------------------|---------|
| 0x03 | Infantry | +0x108 | `ArmorInfantryMult` |
| 0x07 | Unit | +0x10C | `ArmorUnitsMult` |
| 0x07 (loco==5) | Unit (flying) | +0x110 | `ArmorAircraftMult` |
| 0x10 | Aircraft | +0x100 | (likely `ArmorAircraftMult` or `ArmorDefensesMult`) |
| 0x28 | Building | +0x104 | `ArmorBuildingsMult` |
| default | — | returns 1.0 | — |

**CORRECTION to existing docs:** The RECEIVE_DAMAGE report's table had WhatAmI values
swapped for Infantry vs Unit. Binary confirms: case 3 = Infantry (+0x108), case 7 = Unit
(+0x10C or +0x110 for flying), case 0x10 = Aircraft (+0x100), case 0x28 = Building (+0x104).
The default returns `_DAT_007e2ac8` which is `1.0f`.

```c
*pDamage = ftol((float)*pDamage * countryArmorMult);
```

**Only applied when:** `!ignoreDefenses && damage >= 0` (positive damage, not healing).

### Step 2: Veterancy Armor Bonus on Defender (CONFIRMED)

Checks the DEFENDER's (damaged unit's) veterancy level:
- `VeterancyClass__IsVeteran` (0x0074ff90): `1.0 <= vet < 2.0` — **renamed in this session**
- `VeterancyClass__IsElite` (0x00750010): `vet >= 2.0` — **renamed in this session**

Binary shows:
```c
if (isVeteran && TypeClass+0x29d != 0)  // VeteranAbility ARMOR
    OR
if (isElite && (TypeClass+0x29d != 0 || TypeClass+0x2af != 0))  // Elite ARMOR
{
    *pDamage = ftol((float)*pDamage / VeteranArmor);  // default 1.5, DIVIDES
}
```

**CONFIRMS** existing docs: damage is DIVIDED by VeteranArmor (default 1.5), making
the unit tougher. The ability flags are at TechnoTypeClass+0x29D (vet) and +0x2AF (elite).

**Only applied when:** `!ignoreDefenses && damage >= 0`.

### Step 3: Minimum Damage Floor (CONFIRMED)

```c
if (*pDamage < 1) *pDamage = 1;
```

After country armor and veterancy modifiers, damage is clamped to minimum 1.
**Only for positive damage path** (not healing).

### Step 4: TypeImmune Check (CONFIRMED)

```c
if (attacker != NULL && this->GetType()->TypeImmune)  // TypeClass+0xC8C
    if (attacker->GetType() == this->GetType() && this->Owner == attacker->Owner)
        return 0;  // same type, same owner = immune
```

**CONFIRMED** at TypeClass+0xC8C. E.g., Tanya immune to friendly Tanya.

### Step 5: IronCurtain Check (CONFIRMED)

Function `TechnoClass__IsIronCurtainActive` at `0x0041bf40`:
```c
remaining = (this+0x194) - (currentFrame - (this+0x18c));
return remaining > 0;
```

Fields:
- `this+0x18C` = IC start frame (-1 = inactive)
- `this+0x194` = IC duration in frames

If active AND `!ignoreDefenses && damage > 0`:
- Plays spark animation via `FUN_0048a620`:
  - If `this+0x1C4 == 1` (ForceShield variant): anim type 6
  - Else: anim type 1 (normal IC spark)
- Sets `*pDamage = 0`, returns 0.

**CONFIRMED** from binary — exactly matches existing docs.

### Step 6: Warping Out Check (CONFIRMED)

Vtable+0x1D4 (`TechnoClass::IsWarpingOut` at `0x0070c5b0`):
- Reads `this+0x270`. If unit is being chronoshifted AND `!ignoreDefenses`:
  `*pDamage = 0`, return 0.

### Step 7: Ammo Absorption (CONFIRMED)

If `TechnoTypeClass+0x6B1 != 0` (ammo absorption flag):
```c
float absorb = (float)*pDamage / (float)Strength * TypeClass+0x6B4;
this->Ammo = ftol(this->Ammo - absorb);
if (this->Ammo < 0) this->Ammo = 0;
FUN_006fb080();  // trigger ammo depletion anim
```

### Step 8: ForceShield / Bunker Logic (CONFIRMED)

If `this+0x2E4 != 0` (ForceShield active) AND `!ignoreDefenses`:
- **Buildings** (WhatAmI == 6): If warhead has PenetratesBunker (WH+0x146), skips
  protection. Otherwise `*pDamage = 0`, return 0.
- **Non-buildings**: If warhead does NOT have PenetratesBunker, checks if attacker
  is in same cell as building; if so, damage = 0.

### Step 9: Warhead Immunity Checks (CONFIRMED)

All offsets verified from binary:
```c
if (wh+0x177 && TypeClass+0xD37)  → Radiation + ImmuneToRadiation: damage=0
if (wh+0x178 && TypeClass+0xD36)  → PsychicDamage + ImmuneToPsionicWeapons: damage=0
if (wh+0x156 && TypeClass+0xD3B)  → Poison + ImmuneToPoison: damage=0
if (!wh+0x179 && allied)          → AffectsAllies=no + allied target: damage=0
```

### Step 10: Psychedelic / Mind Control (CONFIRMED)

If `wh+0x16D` (Psychedelic):
1. Allied check → return 0
2. ImmuneToPsionics (TypeClass+0xD35) → return 0
3. Building (WhatAmI == 6) → return 0
4. Calculates MC damage via `FUN_00489180`
5. Stores at `this+0x29C`, sets `this+0x298 = 1`
6. Ejects passengers (`FUN_006ea870`)
7. Returns 1

### Step 11: ObjectClass::ReceiveDamage (CONFIRMED)

Falls through to `ObjectClass__ReceiveDamage` at `0x005f5390` (body 0x5f5390–0x5f584c).

**CORRECTION:** The RECEIVE_DAMAGE report listed ObjectClass::ReceiveDamage at `0x005f8c90`.
This is WRONG — that address is inside `CDFileClass::Constructor`. The correct address
is `0x005f5390`, confirmed from Ghidra function listing.

---

## 3. ObjectClass::ReceiveDamage — Health Modification (CONFIRMED)

### Early exits

```c
if (Health < 1 || *pDamage == 0) return 0;
if (!ignoreDefenses && TypeClass+0x233 != 0)  // Insignificant flag
    return 0;
```

### Armor/Verses Application (CONFIRMED)

```c
if (!ignoreDefenses) {
    int armorType = this->GetType()->Armor;  // TypeClass+0x9C
    *pDamage = FUN_00489180(*pDamage, warhead, armorType, distance);
}
```

Called with the victim's armor index from TypeClass+0x9C.

### Building Minimum Damage (CONFIRMED)

```c
if (WhatAmI() == 6) {  // Building
    BuildingTypeClass* btype = ...; // via +0x520
    if (btype+0x1577 == 0) {  // CanC4 == false
        if (*pDamage < 1) *pDamage = 1;
    }
}
```

Buildings WITHOUT `CanC4=yes` always take minimum 1 damage. This is AFTER Verses.
**NOTE:** This inverts the usual understanding — buildings that CAN be C4'd do NOT get
the minimum damage floor. Buildings that CANNOT be C4'd always take at least 1.

### Negative Damage / Healing (CONFIRMED)

```c
if (*pDamage < 0) {
    this->Health -= *pDamage;  // subtracting negative = adding
    if (this->Health > Strength) this->Health = Strength;
    if (oldHealth != this->Health) {
        vtable+0x148(7);  // notify health changed
    }
    return 0;
}
```

### Damage Cap (CONFIRMED)

```c
if (*pDamage >= currentHealth) {
    *pDamage = currentHealth;  // can't overkill
}
```

### Condition Threshold Tracking (CONFIRMED)

```c
result = 1;  // default: minor damage
if (currentHealth >= Strength/2 && (currentHealth - *pDamage) < Strength/2)
    result = 2;  // ConditionYellow

double redThreshold = (double)Strength * *(double*)(Rules+0x1708);
if ((double)currentHealth > redThreshold && (double)(currentHealth - *pDamage) < redThreshold)
    result = 3;  // ConditionRed
```

### Health Subtraction (CONFIRMED)

```c
this->Health = currentHealth - *pDamage;  // at ObjectClass+0x6C
```

### Special Building Survival (WhatAmI == 0x0F)

If Health drops below 1 AND WhatAmI == 0x0F AND !ignoreDefenses:
- Spawns explosion animation from `Rules+0x9C`
- Sets Health to `ftol(Strength * 0.25)`, minimum 1
- Sets a flag, enters ConditionRed state
- Result = 3

**NOTE:** The existing docs say this is for VeinholeMonster (WhatAmI 0x0F). This needs
further verification — 0x0F in the WhatAmI enum may be Aircraft or another type.

### Death Handling (CONFIRMED)

```c
if (this->Health == 0) {
    if (sourceHouse == 0 || (attacker != 0 && sourceHouse == attacker->Owner+0x21C))
        vtable+0xE0(attacker);     // RegisterDestruction(attacker)
    else
        vtable+0xE4(sourceHouse);  // RegisterDestruction(house)
    result = 4;
    vtable+0xDC(1);                // MarkForDeath
}
```

### Trigger Events (CONFIRMED)

| Event | Condition | Attacker required? |
|-------|-----------|-------------------|
| 0x27 | ConditionYellow transition | Yes |
| 0x2A | ConditionYellow transition | No |
| 0x28 | ConditionRed transition | Yes |
| 0x2B | ConditionRed transition | No |
| 0x26 | First damaged (was at full health) | Yes |
| 0x29 | First damaged (was at full health) | No |
| 0x29 | First damaged (any, with attacker param) | Yes |
| 0x06 | Attacked by (not dead) | Yes |
| 0x2C | Attacked by (not dead) | Yes |

**Note:** Event 0x29 fires TWICE when first damaged — once without attacker, once with.
This matches the binary code where there are two separate `TechnoClass__ProcessCellAction(0x29,...)`
calls, one with `0` and one with `in_stack_00000010` (attacker).

---

## 4. Master Damage Formula: FUN_00489180 (CONFIRMED)

**Address:** `0x00489180`
**Decompiled and verified.**

### Parameter list

- param_1 (ECX) = damage amount (uint)
- param_2 (EDX) = WarheadTypeClass* pointer
- param_3 (stack) = armor type index (int, 0-10)
- param_4 (stack) = distance in leptons (int)

### Early exits (CONFIRMED)

```c
if (damage == 0 || (DAT_00a8b230 & 0x20) != 0 || warhead == NULL) return 0;
```

### Negative damage / healing (CONFIRMED)

```c
if (damage < 0) {
    return (armorType > 7) ? 0 : damage;
    // Special armors (8-10: concrete, special_1, special_2) block healing
}
```

The binary uses a clever bit trick: `(7 < param_4) - 1 & param_1` which equals
`param_1` when armorType <= 7 (mask = 0xFFFFFFFF) and `0` when armorType > 7 (mask = 0).

### Distance Falloff (CONFIRMED)

```c
float percentAtMax_dmg = (float)damage * wh->PercentAtMax;  // wh+0x12C
int cellSpread_leptons = ftol(wh->CellSpread * 256.0);      // wh+0x124

if (percentAtMax_dmg != (float)damage && cellSpread_leptons != 0) {
    falloff = percentAtMax_dmg + ((float)damage - percentAtMax_dmg)
              * (cellSpread_leptons - distance) / cellSpread_leptons;
    damage = ftol(falloff);
}
```

Formula: `damage = damage * lerp(1.0, PercentAtMax, distance / (CellSpread * 256))`

### Verses Multiplier (CONFIRMED)

```c
if (damage <= 0) damage = 0;  // clamp post-falloff
versed = ftol((float)damage * wh->Verses[armorType]);  // wh+0xA0 + armorType*8 (double[11])
```

**Note:** The Verses array is `double[11]` at WH+0xA0, NOT float. Each entry is 8 bytes.

### MaxDamage Cap (CONFIRMED)

```c
if (versed >= Rules+0x16C8)  // MaxDamage, default 10000
    return Rules+0x16C8;
return versed;
```

### Double Truncation (CONFIRMED)

Two `ftol()` calls: first after distance falloff (float→int), then that int is cast
back to float, multiplied by Verses double, and truncated again. This matches existing docs.

---

## 5. Verses System — VERIFIED

### Parsing (CONFIRMED from WarheadTypeClass__ReadINI at 0x0075d3a0)

The `Verses=` key is read as a comma-separated string. The parsing loop:
```c
pdVar10 = (double*)(this + 0xA0);  // start of Verses array
count = 11;
do {
    if (strchr(token, '%') != NULL) {
        value = (double)atoi(token) * 0.01;  // "100%" → 1.0
    } else {
        value = atof(token);                  // "0.5" → 0.5
    }
    *pdVar10 = value;
    token = strtok(NULL, ",");
    pdVar10++;
    count--;
} while (count != 0);
```

**CONFIRMED:** 11 doubles parsed, percentage detection via `%` character.

### OrganicImmune Auto-set (CORRECTED)

After Verses parsing:
```c
if (*(double*)(this + 0xC0) == 0.0 && *(double*)(this + 0xD0) == 0.0)
    *(this + 0x149) = 1;  // OrganicImmune
else
    *(this + 0x149) = 0;
```

Offsets 0xC0 and 0xD0 relative to struct base, with Verses starting at 0xA0:
- 0xC0 - 0xA0 = 0x20 = index 4 × 8 bytes = **Armor index 4 = Medium**
- 0xD0 - 0xA0 = 0x30 = index 6 × 8 bytes = **Armor index 6 = Wood**

**CORRECTION:** Both existing docs had this wrong.
- DAMAGE_MATH report claimed "Verses[3]==0 && Verses[5]==0" — WRONG
- WARHEAD_DETONATE report claimed "Verses[2]==0 && Verses[4]==0" — WRONG
- **Correct:** OrganicImmune = `Verses[Medium(4)]==0.0 && Verses[Wood(6)]==0.0`

### Hidden Flags in Verses (ForceFire, Retaliate, PassiveAcquire)

**These DO NOT EXIST in vanilla YR gamemd.exe.**

Searched for strings "ForceFire" and "PassiveAcquire" — not found in binary.
"Retaliate" exists only as a veteran ability name, not as a Verses flag.

The Verses system in vanilla YR is purely a damage multiplier: 11 doubles, one per
armor type. There are NO hidden boolean flags embedded in the Verses values.
ForceFire/Retaliate/PassiveAcquire per armor type are **Ares/Phobos extensions only**.

### Special Values (CONFIRMED)

- `0.0` (0%) = immune to this warhead (no damage dealt)
- `1.0` (100%) = full damage
- `> 1.0` (e.g., 200%) = bonus damage (amplified)
- Values < 0 are technically possible but would produce negative damage, clamped to 0
  by the `if (damage <= 0) damage = 0` check before Verses application

### Retaliation Verses Check (CONFIRMED)

In `TechnoClass__ShouldRetaliate` (0x007087c0, **renamed in this session**), the final
check before approving retaliation:

```c
double verses = warhead->Verses[target->Armor];  // wh+0xA0 + armorIndex*8
if (verses <= 0.01)  // _g_Const_0_01_ProdSpeedFloor
    return 0;  // won't retaliate if weapon can't hurt attacker
```

Units will NOT retaliate if their weapon's Verses against the attacker's armor is
<= 0.01 (effectively immune). This prevents wasted shots.

---

## 6. Armor Types — CONFIRMED

11 armor types, enum index 0-10. String table found at `0x007e5238` (pointer array
pointing to strings in `.rdata`):

| Index | Name | Typical Usage |
|-------|------|---------------|
| 0 | none | Default/unarmored |
| 1 | flak | Anti-air defense |
| 2 | plate | Light vehicles |
| 3 | light | Light armor |
| 4 | medium | Medium armor |
| 5 | heavy | Heavy armor |
| 6 | wood | Wooden structures |
| 7 | steel | Steel structures |
| 8 | concrete | Concrete (special armor, blocks healing) |
| 9 | special_1 | Special (blocks healing) |
| 10 | special_2 | Special (blocks healing) |

Armor index stored at `TechnoTypeClass+0x9C`, parsed from `Armor=` INI key.
Max health at `TechnoTypeClass+0xA0` (`Strength=`).

**Special armor behavior:** Armor types 8-10 (concrete, special_1, special_2)
block healing via negative damage in FUN_00489180.

---

## 7. WarheadTypeClass Boolean Flags — CORRECTED MAP

Verified every flag from the ReadINI decompilation at `0x0075d3a0`:

| Offset | INI Key | Notes |
|--------|---------|-------|
| 0x144 | **Wall** | **CORRECTED** — WARHEAD_DETONATE report had this as "Conventional" |
| 0x145 | WallAbsoluteDestroyer | |
| 0x146 | PenetratesBunker | |
| 0x147 | **Wood** | **CORRECTED** — was missing or misplaced in some docs |
| 0x148 | Tiberium | |
| 0x149 | *OrganicImmune* | Auto-set, not from INI. = Verses[Medium]==0 && Verses[Wood]==0 |
| 0x14A | Sparky | |
| 0x14B | Sonic | |
| 0x14C | **Fire** | **CORRECTED** — was listed as "Conventional" in WARHEAD_DETONATE |
| 0x14D | **Conventional** | **CORRECTED** — actual offset confirmed from INI string xref |
| 0x14E | Rocker | |
| 0x14F | DirectRocker | |
| 0x150 | Bright | |
| 0x151 | CLDisableRed | |
| 0x152 | CLDisableGreen | |
| 0x153 | CLDisableBlue | |
| 0x154 | EMEffect | |
| 0x155 | MindControl | |
| 0x156 | Poison | |
| 0x157 | IvanBomb | |
| 0x158 | ElectricAssault | |
| 0x159 | Parasite | |
| 0x15A | Temporal | |
| 0x15B | IsLocomotor | |
| 0x15C | Locomotor (CLSID, 16 bytes) | |
| 0x16C | Airstrike | |
| 0x16D | Psychedelic | |
| 0x16E | BombDisarm | |
| 0x170 | Paralyzes (int) | |
| 0x174 | Culling | |
| 0x175 | MakesDisguise | |
| 0x176 | NukeMaker | |
| 0x177 | Radiation | |
| 0x178 | PsychicDamage | |
| 0x179 | AffectsAllies (default=true) | |
| 0x17A | Bullets | |
| 0x17B | Veinhole | |

**Key corrections:** The DAMAGE_MATH report and WARHEAD_DETONATE report had
inconsistent/wrong assignments for offsets 0x144, 0x147, 0x14C, 0x14D.
The DAMAGE_MATH report had 0x144=Wall (correct), 0x14D=Conventional (correct) but
was missing 0x147=Wood and 0x14C=Fire.
The WARHEAD_DETONATE report had 0x144=Conventional (WRONG), 0x147=Wood (correct).

---

## 8. BuildingClass::ReceiveDamage — VERIFIED

**Address:** `0x00442230` (body 0x442230–0x442c03, 371 decompiled lines).

### Pre-checks before calling TechnoClass::ReceiveDamage

1. **Self-damage immunity:** If attacker == this building AND `TypeClass+0xCA0 == 0`
   (not SelfHealC4), return 0. Prevents self-inflicted splash damage.

2. **Insignificant + UnsellableTransport check:** If `TypeClass+0x16BF != 0` AND
   `!ignoreDefenses`, return 0.

3. **Insignificant + Insignificant check:** If `TypeClass+0x16B6 != 0` AND
   `TypeClass+0x233 != 0`, return 0.

4. **Already dead:** If `Health == 0`, jumps to death cleanup.

### After TechnoClass::ReceiveDamage returns

5. **ConditionYellow (case 2):** If building has a `LightSource` at `this+0x30C`,
   multiplies `LightSource+0xE8` by `_DAT_007e4460` — **this dims the building light
   when it goes to yellow health**. This is a finding not documented elsewhere.

6. **ConditionRed / ConditionYellow (cases 2, 3):** Plays damage sound from
   TypeClass+0x538, then iterates foundation cells and spawns damage fire/spark
   animations from `Rules+0xB78` based on `Sparky` warhead flag (WH+0x14A).
   Random selection from foundation width + height + 5 possible anim slots.

7. **Death (case 4):**
   - Removes ForceShield occupant if any (`this+0x2E4`)
   - Frees CaptureManager (`this+0x2BC`)
   - Deploys chronowarp unit if any (`this+0x2AC`)
   - **Garrison kill/eject**: Iterates occupants, calculates distance from occupant
     to building center. If distance < 0x100 (256 leptons) OR `TypeClass+0x16CB != 0`,
     kills occupant with C4Warhead (`Rules+0xFA8`) at 10× Strength.
     Otherwise ejects occupant via mission 0x17 and clears passenger link.
   - Checks `TypeClass+0x157B` — if set, sells building instead of destroying.
   - Clears LightSource if present.
   - Calls vtable+0x4EC for death effects.
   - Checks delay-kill timer (`this+0x528`, `this+0x530`).

8. **Attacker tracking:** If attacker exists AND not allied:
   - Sets `Owner+0x54D8` = current frame (last attacked time)
   - Sets `Owner+0x54DC` = attacker cell (for base defense AI)
   - Calls `FUN_00708080` (threat assessment / AI base defense response)
   - Records attacker cell at `this+0x53C`

9. **Building auto-retaliation:** Checks if the building can fight back:
   - Not already in certain missions (0x13)
   - Not allied with attacker
   - Has a weapon (via vtable+0x3F8)
   - Weapon warhead is not non-damaging (WH+0x2A4 == 0)
   - Not already attacking the same target OR capture not in progress
   - For human players: checks `Rules+0x17EC` (MultiplayPassive)
   - AI buildings: calls vtable+0x3C8 to set target

10. **Damage state animation update:** Iterates building anim slots (0x594 bytes,
    0x44 per slot = 21 slots) and updates them based on health ratio vs ConditionYellow
    threshold (`Rules+0x1700`). Reads anim names from `TypeClass+0xF4C` (healthy) and
    `TypeClass+0xF5C` (damaged).

---

## 9. Death Handling — VERIFIED

### Death trigger in ObjectClass::ReceiveDamage

Death occurs when `Health` reaches exactly 0 (damage is capped to remaining HP first).

```c
if (this->Health == 0) {
    // Credit kill to attacker
    if (sourceHouse == 0 || (attacker != 0 && sourceHouse == attacker->Owner))
        vtable+0xE0(attacker);     // Killed(attacker)
    else
        vtable+0xE4(sourceHouse);  // KilledByHouse(house)

    vtable+0xDC(1);                // Destroy(true)
    result = 4;                    // NowDead
}
```

### Death types

InfDeath (WH+0x120) determines infantry death animation:
- Parsed from `InfDeath=` in warhead INI section
- Integer index selecting which death anim sequence to play

### TechnoClass post-death processing (in ReceiveDamage case 4/default)

1. **Mind control cleanup:** Breaks MC links at `this+0x2D8`, `this+0x1CC`, `this+0x1D0`,
   `this+0x1D4`.
2. **CaptureManager release:** Frees all captured units.
3. **DeathVoice:** If TypeClass+0x4CC > 0, plays death sound for human players.
4. **DeathWeapon sound:** If TypeClass+0x520 > 0, plays additional sound.
5. **Mission release:** vtable+0x280(3), vtable+0x3A0.
6. **Temporal cleanup:** If `this+0x304` exists, calls its +0xF8 method, clears link.
7. **Land type check:** If land type at death location == 2 (water?), skips debris.
8. **Debris spawning:** From TypeClass+0x5BC (count) and TypeClass+0x324 (types):
   - VoxelAnimClass (size 0x148) for typed debris
   - AnimClass (size 0x1C8) for generic debris from Rules+0x140/0x14C
   - Generic debris gets +0x14 (20) Z offset
9. **Survivor ejection:** Checks `TypeClass+0xD15` (Explodes). If set, OR if veteran/elite
   with appropriate ability (TypeClass+0x2A6/0x2B8), OR weapon has `Suicide` (+0x144):
   iterates passengers, either creates debris anims for each or ejects them.
   Finally calls `FUN_0070d690(0)` for crew/survivor spawning.
10. **Temporal link final cleanup:** If `this+0x38` exists, calls `FUN_00438720`.

### CausesDelayKill Building Survival

For buildings killed by CausesDelayKill warheads:
```c
if (WhatAmI == 6 && wh+0x130 != 0) {  // Building + CausesDelayKill
    BuildingTypeClass* btype = *(this+0x520);
    if (btype+0x1551 != 0) {  // SelfHealing flag
        *(this+0x6DF) = 1;           // delayKill active
        *(this+0x528) = currentFrame; // start frame
        *(this+0x530) = delayFrames;  // duration
        this->IsAlive = true;
        this->Health = 1;
        return 5;  // PostMortem (kept alive)
    }
}
```

---

## 10. Damage Flash / Sparks (CONFIRMED)

When a unit takes damage and is NOT killed (states 1-3):

### Damage timing fields on TechnoClass

```c
this+0x174 = g_CurrentFrameCounter;  // last damage frame
this+0x178 = distance;               // last damage distance
this+0x17C = Rules+0x8C;             // retaliation delay
```

### Flee-on-damage

If TypeClass+0xD2F (Trainable) is set AND TypeClass+0xD30 (DamageReducesReadiness)
is NOT set, AND unit is human-controlled (vtable+0xC4):
```c
vtable+0x470();  // scatter/flee
this+0x1E0 = g_CurrentFrameCounter;  // flee start frame
this+0x1E4 = distance;               // flee distance
this+0x1E8 = *pDamage << 1;          // flee damage × 2
```

### Damage particle systems

When health ratio <= ConditionYellow threshold (`Rules+0x1700`):
- Iterates TypeClass+0x788 (DamageParticleSystems count) backwards
- Filters by ParticleSystemType+0x2B4 == 0
- Creates ParticleSystemClass (size 0x100) at unit location + turret offset
- Stored at `this+0x310`

When health goes ABOVE yellow threshold, destroys existing particle system at `this+0x310`.

### WasAttacked flag

```c
if (attacker != NULL && !IsAllied(attacker))
    this+0x3D1 = 1;  // WasAttacked flag
```

---

## 11. Key Struct Offsets — CONSOLIDATED & VERIFIED

### ObjectClass

| Offset | Type | Field |
|--------|------|-------|
| +0x6C | int | Health |
| +0x80 | byte | IsDirty (needs redraw) |
| +0x90 | byte | IsAlive |

### TechnoClass (inherits ObjectClass)

| Offset | Type | Field |
|--------|------|-------|
| +0x118 | ptr | Passenger list head |
| +0x174 | int | LastDamageFrame |
| +0x178 | int | LastDamageDistance |
| +0x17C | int | RetaliationDelay |
| +0x18C | int | IronCurtainStartFrame (-1=inactive) |
| +0x194 | int | IronCurtainDuration (frames) |
| +0x1C4 | int | ForceShieldFlag (1=ForceShield variant) |
| +0x1CC | ptr | MindControlController (TechnoClass*) |
| +0x1D0 | ptr | MindControlVictimLink |
| +0x1D4 | ptr | TemporalWarpLink |
| +0x1E0 | int | FleeStartFrame |
| +0x1E4 | int | FleeDistance |
| +0x1E8 | int | FleeDamage×2 |
| +0x270 | byte | IsWarpingOut |
| +0x298 | byte | IsMindControlled |
| +0x29C | int | MindControlDamageValue |
| +0x2D0 | int | SpecialMovementState |
| +0x2D8 | int | MindControlOutgoing |
| +0x2DC | int | GarrisonState |
| +0x2E4 | int | ForceShieldTimer |
| +0x304 | ptr | TemporalIncoming |
| +0x310 | ptr | DamageParticleSystem |
| +0x3CF | byte | Repairable/AIDefenseFlag |
| +0x3D1 | byte | WasAttacked |
| +0x418 | byte | Halted/Frozen |

### TechnoTypeClass

| Offset | Type | INI Key |
|--------|------|---------|
| +0x9C | int | Armor (enum 0-10) |
| +0xA0 | int | Strength (max HP) |
| +0x233 | bool | Insignificant |
| +0x29D | bool | VeteranAbility: ARMOR |
| +0x29F | bool | VeteranAbility: SCATTER |
| +0x2A6 | bool | VeteranAbility: YOURFIRE_ROF (retaliation ability) |
| +0x2AA | bool | VeteranAbility: YOURFIRE_POW (retaliation range) |
| +0x2AF | bool | EliteAbility: ARMOR |
| +0x2B1 | bool | EliteAbility: SCATTER |
| +0x2B8 | bool | EliteAbility: YOURFIRE_ROF (retaliation ability) |
| +0x4CC | int | DeathVoice |
| +0x520 | int | DeathWeaponSound / BuildingTypeClass ptr |
| +0x538 | int | DamageParticleSystems anim index |
| +0x5BC | int | DebrisTypes count |
| +0x5E8 | int | GuardRange (leptons) |
| +0x6B1 | bool | AmmoAbsorption flag |
| +0x6B4 | float | AmmoAbsorptionRate |
| +0x788 | int | DamageParticleSystems count |
| +0xC8C | bool | TypeImmune |
| +0xC96 | bool | Repairable |
| +0xD15 | bool | Explodes |
| +0xD2F | bool | Trainable |
| +0xD30 | bool | DamageReducesReadiness |
| +0xD35 | bool | ImmuneToPsionics |
| +0xD36 | bool | ImmuneToPsionicWeapons |
| +0xD37 | bool | ImmuneToRadiation |
| +0xD3B | bool | ImmuneToPoison |
| +0xD9A | bool | CanRetaliate |

### WarheadTypeClass

| Offset | Type | INI Key |
|--------|------|---------|
| +0x98 | double | Deform |
| +0xA0 | double[11] | Verses (88 bytes, one per armor type) |
| +0xF8 | double | ProneDamage |
| +0x100 | int | DeformThreshold |
| +0x120 | int | InfDeath |
| +0x124 | float | CellSpread |
| +0x128 | float | CellInset |
| +0x12C | float | PercentAtMax |
| +0x130 | bool | CausesDelayKill |
| +0x134 | int | DelayKillFrames |
| +0x138 | float | DelayKillAtMax |
| +0x13C | float | CombatLightSize |
| +0x140 | ptr | Particle (ParticleSystemTypeClass*) |
| +0x144 | bool | Wall |
| +0x145 | bool | WallAbsoluteDestroyer |
| +0x146 | bool | PenetratesBunker |
| +0x147 | bool | Wood |
| +0x148 | bool | Tiberium |
| +0x149 | bool | OrganicImmune (auto-set: Verses[Medium]==0 && Verses[Wood]==0) |
| +0x14A | bool | Sparky |
| +0x14B | bool | Sonic |
| +0x14C | bool | Fire |
| +0x14D | bool | Conventional |
| +0x14E | bool | Rocker |
| +0x14F | bool | DirectRocker |
| +0x150 | bool | Bright |

### RulesClass Global Offsets

| Offset | Type | Purpose |
|--------|------|---------|
| +0x8C | int | RetaliationDelay (frames) |
| +0x9C | ptr | ConditionRed anim (for special building survival) |
| +0x100 | float | AircraftArmorMult (HouseType) |
| +0x104 | float | BuildingArmorMult (HouseType) |
| +0x108 | float | InfantryArmorMult (HouseType) |
| +0x10C | float | UnitArmorMult (HouseType) |
| +0x110 | float | FlyingUnitArmorMult (HouseType) |
| +0x140 | ptr | MetallicDebris array |
| +0x14C | int | MetallicDebris count |
| +0xB78 | ptr | BuildingDamageAnims (Sparky anims) |
| +0xFA8 | ptr | C4Warhead |
| +0x16C8 | int | MaxDamage cap (default 10000) |
| +0x1700 | double | ConditionYellow threshold |
| +0x1708 | double | ConditionRed threshold |
| +0x17EC | bool | MultiplayPassive |
| +0x17ED | bool | MultiplayPassive2 |

---

## 12. Complete Damage Flow Summary

```
═══════════════════════════════════════════════════
ATTACKER SIDE — in TechnoClass::Fire_At (0x6fdd50)
═══════════════════════════════════════════════════
  damage = 0 for Wave/special, else weapon.Damage (WeaponType+0xA4)
  if ordinary damage > 0:
      damage = ftol((countryFirepowerMult * unitFirepowerMult) * damage)
      if vet/elite with FIREPOWER ability:
          damage = ftol(damage * VeteranCombat)
  if civilian_garrison: damage = ftol(damage * OccupyDamageMultiplier)
  if tank_bunker:       damage = ftol(damage * BunkerDamageMultiplier)
  if open_topped:       damage = ftol(damage * OpenToppedDamageMultiplier)
  → stored on BulletClass+0x6C

═══════════════════════════════════════════════════
IMPACT — Apply_area_damage (0x489280) or direct
═══════════════════════════════════════════════════
  Collects targets within CellSpread radius
  Measures distance from impact to each target
  Calls ReceiveDamage(baseDamage, distance, ...) per target

═══════════════════════════════════════════════════
TARGET SIDE — TechnoClass::ReceiveDamage (0x701900)
═══════════════════════════════════════════════════
  [1] damage *= defenderCountryArmorMult          (ArmorInfantryMult etc)
  [2] if (vet/elite with ARMOR ability):
      damage /= VeteranArmor                      (default 1.5)
  [3] damage = max(damage, 1)                     (minimum 1)
  [4] TypeImmune check                            → return 0
  [5] IronCurtain check                           → spark anim, return 0
  [6] WarpingOut check                            → return 0
  [7] Ammo absorption                             (if applicable)
  [8] ForceShield/bunker penetration              → return 0
  [9] Warhead immunity checks:
        Radiation + ImmuneToRadiation              → return 0
        PsychicDamage + ImmuneToPsionicWeapons     → return 0
        Poison + ImmuneToPoison                    → return 0
        !AffectsAllies + allied                    → return 0
  [10] Psychedelic (MindControl)                   → apply MC, return 1

═══════════════════════════════════════════════════
ARMOR — ObjectClass::ReceiveDamage (0x5f5390)
═══════════════════════════════════════════════════
  [11] Insignificant check                        → return 0
  [12] FUN_00489180 (Verses + distance falloff):
       - Distance falloff: lerp(1.0, PercentAtMax, dist/(CellSpread*256))
       - Verses multiplier: damage *= Verses[armorType]
       - MaxDamage cap (default 10000)
  [13] Building !CanC4: min damage = 1             (after Verses)
  [14] Zero check                                  → return 0
  [15] Negative damage (healing): add to health, cap at Strength

═══════════════════════════════════════════════════
HEALTH — still in ObjectClass::ReceiveDamage
═══════════════════════════════════════════════════
  [16] Cap damage to currentHealth
  [17] Track ConditionYellow/Red transitions
  [18] Health -= damage                            (at ObjectClass+0x6C)
  [19] Special building survival (WhatAmI 0x0F)
  [20] Death: if Health==0 → RegisterDestruction + MarkForDeath
  [21] Fire trigger events (0x26-0x2C)

═══════════════════════════════════════════════════
POST-DAMAGE — back in TechnoClass::ReceiveDamage
═══════════════════════════════════════════════════
  [22] Score tracking (attacker type scoring)
  [23] CausesDelayKill building survival
  [24] Retaliation timing (this+0x174/0x178/0x17C)
  [25] Flee-on-damage (if Trainable && !DamageReducesReadiness)
  [26] Death cleanup: MC, capture, temporal, passengers, debris, crew
  [27] Damage state anims and sounds
  [28] Damage particle systems (smoke at yellow/red health)
  [29] WasAttacked flag (this+0x3D1)
  [30] Retaliation check (TechnoClass__ShouldRetaliate)
  [31] Scatter-on-damage (if not retaliating)
```

---

## 13. Corrections to Existing Docs

| Issue | Old Value | Corrected Value | Source |
|-------|-----------|-----------------|--------|
| ObjectClass::ReceiveDamage address | 0x005f8c90 | **0x005f5390** | RECEIVE_DAMAGE report |
| WH+0x144 | "Conventional" (WARHEAD_DETONATE) | **Wall** | Binary ReadINI xref |
| WH+0x147 | missing | **Wood** | Binary ReadINI xref |
| WH+0x14C | missing or wrong | **Fire** | Binary ReadINI xref |
| WH+0x14D | missing or wrong | **Conventional** | Binary ReadINI xref |
| OrganicImmune formula | "Verses[3]&&Verses[5]" or "Verses[2]&&Verses[4]" | **Verses[4](Medium)==0 && Verses[6](Wood)==0** | Binary code at end of ReadINI |
| ForceFire/Retaliate/PassiveAcquire in Verses | mentioned as possible | **DO NOT EXIST** in vanilla YR | String search negative |
| HouseClass__GetBuildSpeedBonus name | GetBuildSpeedBonus | **GetArmorMultForType** | Decompiled, reads ArmorXxxMult |
| Volume__IsNormal / FUN_00750010 names | Volume__IsNormal / FUN_00750010 | **VeterancyClass__IsVeteran / VeterancyClass__IsElite** | Decompiled |
| FUN_007087c0 name | unnamed | **TechnoClass__ShouldRetaliate** | Decompiled |
| Country armor mult WhatAmI mapping | Swapped Infantry/Unit in one doc | Infantry=3(+0x108), Unit=7(+0x10C), Aircraft=0x10(+0x100), Building=0x28(+0x104) | Decompiled |

---

## 14. Functions Renamed in This Session

| Address | Old Name | New Name |
|---------|----------|----------|
| 0x0050bd30 | HouseClass__GetBuildSpeedBonus | HouseClass__GetArmorMultForType |
| 0x0074ff90 | Volume__IsNormal | VeterancyClass__IsVeteran |
| 0x00750010 | FUN_00750010 | VeterancyClass__IsElite |
| 0x007087c0 | FUN_007087c0 | TechnoClass__ShouldRetaliate |

Program saved after renaming.
