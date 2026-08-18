# Veterancy / Promotion System — Ghidra Research Report

**Date (initial):** 2026-03-22
**Date (extended):** 2026-04-19
**Date (re-verified):** 2026-04-19 (second pass — corrections logged in §18)
**Date (open-questions resolved):** 2026-04-19 (third pass — logged in §19)
**Date (fourth pass):** 2026-04-19 (crate dispatch mapped; selfheal corrected; logged in §20)
**Date (fifth pass):** 2026-04-19 (FEARLESS consumer; 0x164/0x168 retraction; logged in §21)
**Date (deep-dive):** 2026-04-19 (third pass — additional findings in §19)
**Date (crate-pickup resolved):** 2026-04-19 (fourth pass — §13 rewritten,
`CrateClass__PickupDispatch` labelled; Rust impl status re-verified unchanged;
spot-check of Add_Experience / IsVeteran / GetVeterancyLevel passed)
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH — all offsets verified from decompiled binary. The deep-dive pass
added four load-bearing corrections (self-heal timer source, `+0xD68` identity,
damage-multiplier chain, `IsBaseDefense` offset correction). See §§18 and 19 for
the full diff against the earlier passes.
**Active in YR:** Yes, fully live.

---

## Overview

The veterancy system tracks a floating-point experience value per TechnoClass instance. When a
unit kills an enemy, the killed unit's cost is converted to experience and added to the killer's
veterancy float. Crossing thresholds promotes the unit from Rookie → Veteran → Elite, granting
bonuses keyed off per-ability flags (`VeteranAbilities` / `EliteAbilities` in INI) combined with
global multipliers from `[General]` in rules(md).ini.

---

## 1. Veterancy Float Location

**TechnoClass offset 0x150** — a single `float` storing accumulated experience.

Verified from `DrawVeterancyPips` at `0x0070a990`:
```
0070a9a7: LEA EDI,[ESI + 0x150]
0070a9ad: MOV ECX,EDI
0070a9b2: CALL IsVeteran           ; 0x0074ff90
```

All veterancy check functions take ECX = pointer to this float.

---

## 2. Promotion Thresholds

### IsRookie — `0x0074FFF0`
```c
bool IsRookie(float *vet) { return *vet < 0.0f; }
```
Only true when veterancy has been pushed negative (special crate/weapon effects).
Normal units never return true here.

### IsVeteran — `0x0074FF90`
```c
bool IsVeteran(float *vet) { return *vet >= 1.0f && *vet < 2.0f; }
```
Veteran threshold: **1.0f** (constant `0x3F800000` at `0x007e2ac8`).

### IsElite — `0x00750010`
```c
bool IsElite(float *vet) { return *vet >= 2.0f; }
```
Elite threshold: **2.0f** (constant `0x40000000` at `0x007e37b4`).

### GetVeterancyLevel — `0x00750030` (labelled `Volume__GetCategory` in Ghidra)
```c
int GetVeterancyLevel(float *vet) {
    if (*vet >= 2.0f) return 0;  // Elite
    if (*vet <  1.0f) return 2;  // Rookie
    return 1;                    // Veteran
}
```
**Reverse mapping: Elite=0, Veteran=1, Rookie=2.** Used by AI_Update for promotion detection.

---

## 3. Experience Gain Formula

### Add_Experience — `FUN_0074FF50` @ `0x0074FF50`

Verified from fresh decompile:
```c
void Add_Experience(float *veterancy, int own_cost, int scaled_killed_cost) {
    float new_val = (float)scaled_killed_cost
                  / ((float)own_cost * (float)Rules.VeteranRatio)
                  + *veterancy;
    *veterancy = new_val;
    if ((float)Rules.VeteranCap <= new_val) {
        new_val = (float)Rules.VeteranCap;
    }
    *veterancy = new_val;
}
```

- `veterancy` = pointer to TechnoClass+0x150
- `own_cost` = attacker's TechnoType Cost
- `scaled_killed_cost` = killed unit Cost pre-multiplied by its veterancy tier (see below)
- `VeteranRatio` = RulesClass+0x668 (double), default **3.0**
- `VeteranCap` = RulesClass+0x698 (double), rulesmd value **2** (=elite cap)

**Callers of Add_Experience** (verified via `get_function_callers`):
1. `TechnoClass::RecordKill` @ `0x00702D40` — standard kill-XP path
2. `TemporalClass::AI` @ `0x006297F0` — Chrono Legionnaire warp-out finalization
3. `TemporalClass::Update` @ `0x0071A760` — alternate Chrono Legionnaire path

**Corrected (2026-04-19):** The `Add_Experience` call inside `TemporalClass::AI`
is in **case 4** of the animation-state switch (finalization), NOT per-tick
during warp. The earlier claim that "warp damage awards XP incrementally as the
target is erased" was wrong — cases 0–3 advance the warp-in / spin / warp-out
animations and do NOT award XP. Case 4 is the finalization frame where the
target object is detached/killed; this is where `FUN_0074ff50(...)` fires, and
only when:
- the weapon's +0x174 flag (on the weapon type) is set, AND
- `FUN_005F5DD0() == 0` OR (`== 1` AND the CL is already elite), AND
- the Chrono Legionnaire type has `Trainable=yes`.

So in practice: XP for a CL kill is awarded once per erased target at the moment
the target vanishes, not streamed over the warp duration. Implementations need
one hook at the warp-completion moment, not a per-tick accumulator.

### Kill Experience Scaling — in `TechnoClass::RecordKill` @ `0x00702D40`

The killed unit's cost is multiplied by the killed unit's tier:
- Rookie victim: `cost × 1`
- Veteran victim: `cost × 2`
- Elite victim: `cost × 3`

```c
int cost = killed_type->Cost;             // killed TechnoType Cost
if (HouseClass::Is_Ally_ByObject(killed)) {
    cost = 0;                             // *** friendly-fire grants no XP ***
} else {
    if (IsVeteran(killed)) cost *= 2;
    else if (IsElite(killed)) cost *= 3;
}
// ... attribution chain picks who receives the XP ...
Add_Experience(&killer->Veterancy, killer_cost, cost);
```

**Important correction added 2026-04-19:** allied kills grant **zero** XP. The cost is
zeroed before any scaling. This was missed in the initial report.

### DontScore — `TechnoTypeClass+0xc9f` (bool)

Verified via `DontScore` string xref from `TechnoTypeClass::ReadINI` @ `0x00713F4B`.
If the **killed** unit's type has `DontScore=yes`, `RecordKill` returns very early —
no XP is awarded, and no kill-count stats are updated. Default: false.

Practical effect: civilian tech structures, neutral units, and objects that explicitly set
`DontScore=yes` never count toward veteran XP.

### Trainable — `TechnoTypeClass+0xc8e` (bool)

Verified via `Trainable` string xref from `TechnoTypeClass::ReadINI` @ `0x00714A1C`.
Checked on the **killer**; if false, the killer cannot receive XP directly (building-linked
or chain paths may still route the XP elsewhere — see "Attribution chain" below).

**Default: true.** In rulesmd.ini only specific units (Engineer, Spy, Ivan, Dog, ADOG) set
`Trainable=no`. Units that don't mention the key remain trainable.

### Attribution Chain — `RecordKill` branches

`RecordKill` picks the entity that actually receives the XP via a priority chain:

1. If `killer->field_0x11C` (offset 0x47 as int*) is non-null AND that linked entity's
   TechnoType has `Trainable=yes` → the linked entity receives XP.
   (This handles **mind-controlled kills** — the mind controller is the linked entity.)
2. Else if the killer's own TechnoType has `Trainable=yes` → killer receives XP.
3. Else if the killer's TechnoType flag at `+0xD68` is set → route XP to the linked
   parent at `killer->+0x2D4` if that parent's type is `Trainable=yes`. **Identified
   2026-04-19 (deep-dive):** `+0xD68` = **`MissileSpawn`** (verified via
   `TechnoTypeClass::ReadINI` PUSH of string "MissileSpawn" @ `0x00843798` before
   the ReadBool that writes byte `[EBP + 0xD68]` at `0x00714F37`). In stock
   rulesmd.ini, `MissileSpawn=yes` is set on exactly three TechnoTypes:
   **`[V3ROCKET]`** (V3 Rocket Launcher's missile), **`[DMISL]`** (Dreadnought's
   missile), and **`[CMISL]`** (Boomer submarine's cruise missile). These are the
   child missile sub-units launched by their respective parents. When such a
   missile kills a target, XP flows back to the parent launcher (V3 Rocket
   Launcher, Dreadnought, or Boomer), which is Trainable. `killer->+0x2D4`
   on the missile is the back-pointer to its parent.

   *(The earlier passes guessed "base-defense flag" — wrong. `IsBaseDefense` is a
   separate bool at `BuildingTypeClass+0x1706`, verified via the same ReadINI
   pattern at `0x00461010`. Not at `+0xD68`.)*
4. **When the killer's `WhatAmI() == 6` (i.e., the killer is a `BuildingClass`, not
   "Mission == 6" as the earlier passes incorrectly phrased — `vtable+0x2c` returns
   the RTTI type code, not the mission enum):** route XP through
   `*(killer+0x688)` indexed by `*(killer+0x69C)` — the building's garrison-occupant
   array. XP flows to the occupying infantry unit whose weapon did the kill, not
   the building itself. Verified 2026-04-19 from `RecordKill` @ `0x00702D40` via
   the same vtable-slot-2c usage in `TechnoClass::AI_Update` which checks
   `vtable+0x2c == 1` (Unit), `== 2` (Infantry), `== 6` (Building), `== 0xf`
   (Vessel) — all RTTI codes.

The **kill counters** at killer offsets +0x5488, +0x548C, +0x53E4, +0x5438, +0x5434, +0x54E8
are updated regardless of attribution (they are on the killer's House, not the XP recipient).

---

## 4. Per-House Starting Veteran Overrides

Per-house overrides in `[<Country>]` sections (read in `FUN_00511850`):
- `VeteranUnits` (string @ `0x00825298`) — unit TYPES whose instances spawn as veteran
- `VeteranAircraft` (string @ `0x00825288`)
- `VeteranInfantry` (string @ `0x008252A8`)

In stock rulesmd.ini these keys exist only as **commented-out** placeholders — no country
actively promotes anything by default. (Mods add entries here; stock YR content does not.)

**Per-unit spawn-time path verified 2026-04-19** in `InfantryClass::InitFromType`
@ `0x00517CC0` (and by analogy `UnitClass::InitFromType`, `AircraftClass::InitFromType`):

```c
// Step 1: walk the per-house VeteranInfantry (or VeteranUnits / VeteranAircraft)
//         list at house->Country->+0x150 (ptr array) / +0x15C (count). If the
//         unit's TechnoType is in the list, call VeterancyStruct::SetVeteran(1).
for (i = 0; i < country->veteran_type_count; i++) {
    if (country->veteran_types[i] == this->Type) {
        this->Veterancy.SetVeteran(1);   // sets veterancy float to 1.0 (Veteran)
        break;
    }
}

// Step 2: InitialVeteran path — if house->+0x2BF is set AND the unit's type
//         is Trainable (+0xC8E), also promote to veteran.
if (house->+0x2BF != 0 && type->+0xC8E != 0) {
    this->Veterancy.SetVeteran(1);
}
```

So `[SpecialFlags] InitialVeteran=yes` (which sets the global bit 9 per §5)
must be propagated into every `HouseClass` instance at `+0x2BF` during house
construction — the per-spawn InitFromType check reads it from the house, not
the global. That house-side propagation step was not decompiled in this pass;
left as open question #6.

## 5. InitialVeteran — `[SpecialFlags]` bit 9 (CORRECTED)

Verified 2026-04-19. The `InitialVeteran` string at `0x0084022C` is referenced from two
SpecialFlags functions:
- `FUN_006b8ca0` @ `0x006B8CA0` — reads `[SpecialFlags] InitialVeteran` via `CCINIClass::ReadBool`
  from `PTR_s_SpecialFlags_008401cc`, stores bit 9 of the SpecialFlags bitmask.
- `FUN_006b8b30` @ `0x006B8B30` — the inverse (writes current bit 9 back to INI).

**Canonical section is `[SpecialFlags]`, not `[General]`.** The line
`InitialVeteran=no` that appears in the `[General]` section of rulesmd.ini is **ignored
by the engine** — that section's parser (`RulesClass::ReadGeneral`) never reads this key.
In stock rulesmd.ini the `[SpecialFlags]` section is not present at all, so the bit stays
at its default (false). Mods must add `[SpecialFlags]\nInitialVeteran=yes` to activate it.

When bit 9 is set, units spawn with `this->Veterancy = 1.0f` (promoted to veteran at birth).

---

## 6. VeteranAbilities / EliteAbilities — the 18-Ability Flag Arrays

### Storage
- `VeteranAbilities`: TechnoTypeClass offset **0x29C**, 18 consecutive bytes (bools).
- `EliteAbilities`:   TechnoTypeClass offset **0x2AE**, 18 consecutive bytes (bools).

(0x2AE − 0x29C = 0x12 = 18 bytes.)

### Ability Check — `TechnoClass::HasWeaponAbility` @ `0x0070D0D0`

Fresh decompile, verified:
```c
bool HasWeaponAbility(TechnoClass *this, int idx) {
    if (!IsVeteran(this) && !IsElite(this)) return false;
    TechnoTypeClass *type = this->GetType();
    if (IsVeteran(this) && type->VeteranAbilities[idx]) return true;
    if (IsElite(this)   && (type->VeteranAbilities[idx] || type->EliteAbilities[idx])) return true;
    return false;
}
```
Elites inherit veteran abilities and add their own (OR on both flag arrays).

### Ability Name → Index Table (CORRECTED 2026-04-19)

The binary's ability-name parser is `AbilityClass::FindAbilityByName` @ `0x0074FEFF`. It
iterates the pointer table `PTR_s_FASTER_008463b8`–`0x008463FC` (18 entries × 4 bytes). Each
entry is a pointer to a null-terminated name string. **The initial 2026-03-22 report had
several entries wrong**; the 2026-04-19 pass resolved all 18:

| Index | Name              | Offset (Vet / Elite) | Pointer @    | String @     |
|-------|-------------------|----------------------|--------------|--------------|
| 0     | `FASTER`          | 0x29C / 0x2AE        | 0x008463B8   | 0x008464A4   |
| 1     | `STRONGER`        | 0x29D / 0x2AF        | 0x008463BC   | 0x00846498   |
| 2     | `FIREPOWER`       | 0x29E / 0x2B0        | 0x008463C0   | 0x0084648C   |
| 3     | `SCATTER`         | 0x29F / 0x2B1        | 0x008463C4   | 0x00820BA0   |
| 4     | `ROF`             | 0x2A0 / 0x2B2        | 0x008463C8   | 0x00825478   |
| 5     | `SIGHT`           | 0x2A1 / 0x2B3        | 0x008463CC   | 0x00846484   |
| 6     | `CLOAK`           | 0x2A2 / 0x2B4        | 0x008463D0   | 0x0084647C   |
| 7     | `TIBERIUM_PROOF`  | 0x2A3 / 0x2B5        | 0x008463D4   | 0x0084646C   |
| 8     | `VEIN_PROOF`      | 0x2A4 / 0x2B6        | 0x008463D8   | 0x00846460   |
| 9     | `SELF_HEAL`       | 0x2A5 / 0x2B7        | 0x008463DC   | 0x00846454   |
| 10    | `EXPLODES`        | 0x2A6 / 0x2B8        | 0x008463E0   | 0x00846448   |
| 11    | `RADAR_INVISIBLE` | 0x2A7 / 0x2B9        | 0x008463E4   | 0x00846438   |
| 12    | `SENSORS`         | 0x2A8 / 0x2BA        | 0x008463E8   | 0x00846430   |
| 13    | `FEARLESS`        | 0x2A9 / 0x2BB        | 0x008463EC   | 0x00846424   |
| 14    | `C4`              | 0x2AA / 0x2BC        | 0x008463F0   | 0x00825978   |
| 15    | `TIBERIUM_HEAL`   | 0x2AB / 0x2BD        | 0x008463F4   | 0x00846414   |
| 16    | `GUARD_AREA`      | 0x2AC / 0x2BE        | 0x008463F8   | 0x00846408   |
| 17    | `CRUSHER`         | 0x2AD / 0x2BF        | 0x008463FC   | 0x00846400   |

**Name mismatches corrected vs the 2026-03-22 report:**

| Index | 2026-03-22 said | Actual    |
|-------|-----------------|-----------|
| 0     | STRONGER        | FASTER    |
| 1     | FIREPOWER       | STRONGER  |
| 2     | ROF             | FIREPOWER |
| 3     | SIGHT           | SCATTER   |
| 4     | SPEED           | ROF       |
| 5     | (unknown5)      | SIGHT     |
| 6     | (scatter)       | CLOAK     |
| 7     | (inaccuracy)    | TIBERIUM_PROOF |
| 8     | (unknown8)      | VEIN_PROOF |
| 9     | SELF_HEAL       | SELF_HEAL ✓ |
| 10    | (unknown10)     | EXPLODES  |
| 11    | (unknown11)     | RADAR_INVISIBLE |
| 12    | (unknown12)     | SENSORS   |
| 13    | (unknown13)     | FEARLESS  |
| 14    | (guard)         | C4        |
| 15    | (unknown15)     | TIBERIUM_HEAL |
| 16    | (unknown16)     | GUARD_AREA |
| 17    | (unknown17)     | CRUSHER   |

Stock YR content (rulesmd.ini) uses only a handful: `FASTER`, `STRONGER`, `FIREPOWER`,
`ROF`, `SIGHT`, `SCATTER`, `SELF_HEAL`. Others are reachable for mods but unused by YR's
own content.

**Index uses confirmed in code paths:**
- `HasWeaponAbility(this, 4)` — ROF bonus (verified in `FUN_006FCFA0` at
  offsets `0x29C+4=0x2A0` and `0x2AE+4=0x2B2`).
- `HasWeaponAbility(this, 2)` — FIREPOWER damage bonus (verified in `Fire_At` at
  `0x006FE35E`).
- `HasWeaponAbility(this, 1)` — STRONGER = armor bonus (verified in `ReceiveDamage` at
  `0x00701970`; applies `VeteranArmor` scaling).
- `HasWeaponAbility(this, 3)` — SCATTER. **Corrected 2026-04-19:** used in two
  places, not one:
    1. **`CellClass::Scatter_Objects` @ `0x00481670`** — primary consumer. Units
       with SCATTER dodge (move out of) cells that are being hit by an incoming
       AoE/large weapon. Also triggers if *any* elite unit is present in the
       cell — elite units force-scatter everyone else. This is the
       dodge-out-of-blast-radius behavior.
    2. **`TechnoClass::ReceiveDamage` retaliation gate @ `~0x00702B91`** — part
       of the retaliation path. Only units that are veteran/elite AND have the
       SCATTER ability flag set (vet or elite bit) reach `LAB_00702cfe`, which
       calls `vtable[0x174]` (the auto-retaliation / counter-fire method). This
       is in addition to the normal `ShouldRetaliate` path. Exact role relative
       to the standard retaliation machinery is not fully traced — both callers
       and non-vet units do retaliate via other paths; this block is a
       veteran-specific branch.
- `HasWeaponAbility(this, 9)` — SELF_HEAL eligibility (`FUN_0070BE80`).

Speed (index 0 / FASTER), Sight (index 5), Cloak (6), Sensors (12), and the rest are
consumed in their respective subsystems (locomotion, vision, cloak, etc.) — index-to-code
wiring for those was not re-verified in this pass; the index values above are the
ground-truth source of truth.

---

## 7. Global Multipliers from `[General]`

Read in `RulesClass::ReadGeneral` @ `0x0066D530` (verified; offsets match 2026-03-22 report):

| INI Key                  | RulesClass Offset | Code Default | Stock rulesmd.ini |
|--------------------------|-------------------|--------------|-------------------|
| `VeteranRatio`           | +0x668 (double)   | 3.0          | **3.0**           |
| `VeteranCombat`          | +0x670 (double)   | 1.0          | **1.1**           |
| `VeteranSpeed`           | +0x678 (double)   | 1.0          | **1.2**           |
| `VeteranSight`           | +0x680 (double)   | 1.0          | **0.0** (multiplicative — gate: `!= 0.0` disables bonus; corrected 2026-05-29: was "(additive)"; §7/§18 already state MULTIPLICATIVE but table label was not updated — MISLEADING: stale label conflicted with verified correction; via `decompile_function 0x0070AF50` confirms `*(double*)(Rules+0x680) != 0.0` gate) |
| `VeteranArmor`           | +0x688 (double)   | 1.0          | **1.5**           |
| `VeteranROF`             | +0x690 (double)   | 1.0          | **0.6**           |
| `VeteranCap`             | +0x698 (double)   | 2.0          | **2**             |
| `SelfHealInfantryFrames` | +0x030 (int)      | —            | **50**            |
| `SelfHealInfantryAmount` | +0x034 (int)      | —            | **20**            |
| `SelfHealUnitFrames`     | +0x038 (int)      | —            | **75**            |
| `SelfHealUnitAmount`     | +0x03C (int)      | —            | **5**             |

**CORRECTED (2026-04-19 re-verification): `VeteranSight` is MULTIPLICATIVE.** Prior
passes inferred additive from community docs and the stock `0.0` value. Decompiled
at `TechnoClass::UpdateReveal` @ `0x0070AF50` — the assembly block is unambiguous:

```
0070b082: FLD    double ptr [ECX + 0x680]   ; load Rules.VeteranSight
0070b088: FCOMP  double ptr [0x007e2800]    ; compare to 0.0
0070b08e: FNSTSW AX
0070b090: TEST   AH,0x40                     ; skip if equal (gate)
0070b093: JNZ    0x0070b0a6
0070b095: FILD   dword ptr [ESP + 0x14]     ; push base_sight (int)
0070b099: FMUL   double ptr [ECX + 0x680]   ; base_sight * Rules.VeteranSight
0070b09f: CALL   0x007c5f00                  ; ftol → int
```

So the actual formula when SIGHT ability is present:

```
sight_cells = base_sight * Rules.VeteranSight
```

Note that **base sight is itself already scaled by altitude** (see §7a below), so
the full sight for a veteran/elite with SIGHT ability is:

```
sight_cells = type.Sight * (altitude_level * 0.01 + 1.0) * Rules.VeteranSight
```

The `!= 0.0` gate exists specifically so that stock YR's `VeteranSight=0.0` does
not zero the sight — it disables the bonus path entirely. A modder setting
`VeteranSight=2.0` would double the sight for veteran/elite units with SIGHT.

Consequence: the current Rust implementation (`src/sim/vision.rs:500-504`) treats
`VeteranSight` as an additive cell bonus — **this is wrong per the binary**. With
stock `0.0` it happens to produce the same result (no bonus applied), but any mod
that sets a non-zero value will diverge from gamemd behavior.

`VeteranROF=0.6` means ROF *delay* is multiplied by 0.6 → **faster** fire rate for vets.

### Self-Heal fields on TechnoTypeClass

- +0x164: SelfHealInfantryAmount **multiplier** (per-type override)
- +0x168: SelfHealUnitAmount **multiplier** (per-type override)

### 7a. Base sight altitude scaling (NEW, 2026-04-19)

Before the VeteranSight bonus is even considered, `TechnoClass::UpdateReveal`
computes the base sight cells as:

```
base_sight = type.Sight * (altitude_level * 0.01 + 1.0)
```

Where:
- `type.Sight` is the unit's base sight range (TechnoTypeClass+0x5E8, in cells).
- `altitude_level` is a byte cached at `TechnoClass+0x420`, computed as
  `this.Z / Rules.field_0x16BC` (Z coord divided by the global altitude quantum).
- `0.01` is the constant at `0x007E3808` (double).
- `1.0` is the constant at `0x007E1718` (double).

Ground units (altitude 0) see their base sight unchanged. Aircraft gain sight
proportional to altitude — at `altitude_level = 100`, sight is doubled. This is
what gives flying units their "extended recon" behavior.

Assembly:

```
0070afef: FILD  dword ptr [EAX + 0x5e8]    ; type.Sight
0070aff9: FILD  dword ptr [ESP + 0x10]     ; altitude_level
0070affd: FMUL  double ptr [0x007e3808]    ; × 0.01
0070b003: FADD  double ptr [0x007e1718]    ; + 1.0
0070b009: FMULP                             ; type.Sight * (altitude*0.01 + 1.0)
0070b00b: CALL  0x007c5f00                  ; ftol → int
```

This is separate from (and composes with) the veteran SIGHT bonus.

---

## 8. Where Bonuses Plug In (bonus → code path)

| Ability (idx, name) | Global multiplier | Applied in                                          |
|---------------------|-------------------|------------------------------------------------------|
| 0 FASTER            | `VeteranSpeed`    | Locomotion (speed calc, location not re-verified)    |
| 1 STRONGER          | `VeteranArmor`    | `TechnoClass::ReceiveDamage` @ `0x00701970`          |
| 2 FIREPOWER         | `VeteranCombat`   | `TechnoClass::Fire_At` @ `0x006FE35E`                |
| 3 SCATTER           | —                 | `ReceiveDamage` retaliation @ `0x00702B91`           |
| 4 ROF               | `VeteranROF`      | `FUN_006FCFA0` (ROF getter) @ `0x006FD0F0` region    |
| 5 SIGHT             | `VeteranSight` (+)| Vision reveal radius (site not re-verified)          |
| 6 CLOAK             | —                 | `CloakingTick`/`CanAutoCloak`/`ShouldUncloak` family |
| 9 SELF_HEAL         | (timer-driven)    | `TechnoClass::AI_Update` @ `0x006FA7B0` + `FUN_0070BE80` |

---

## 9. Promotion Detection — `TechnoClass::AI_Update`

Fresh decompile at `0x006FA055`–`0x006FA100`, verified:
```c
int prev = this->field_0x13C;           // cached level; -1 = first tick
int curr = GetVeterancyLevel(&this->Veterancy);
if (prev != curr) {
    if (prev != -1) {                   // skip on first update
        if (curr == 0) {                // promoted to Elite
            if (HouseClass::IsHumanPlayer()) {
                VocClass::PlayAt(0);    // upgrade sound at unit position
                VoxClass::PlayEVA(-1);  // "EVA_UnitPromoted"
            }
            this->field_0xF0 = Rules.field_0xBE8;   // *** see §10 below ***
        } else if (curr == 1) {         // promoted to Veteran
            if (HouseClass::IsHumanPlayer()) {
                VocClass::PlayAt(0);
                VoxClass::PlayEVA(-1);
            }
        }
    }
    this->field_0x13C = curr;
}
```

- The EVA event and upgrade sound play for **any** promotion (rookie→vet, vet→elite),
  but only for the local human player.
- First-tick suppression is via `prev != -1`; the cached level is initialized to -1.
- The EVA/voc calls pass `0` and `-1` as arg0 in the decompile — these are interpreted
  from other registers at runtime. `UpgradeVeteranSound` (string @ `0x0083A8A0`) and
  `UpgradeEliteSound` (string @ `0x0083A88C`) from `[AudioVisual]` are the expected VocClass
  entries; the exact register-passed values were not re-verified in this pass.

---

## 10. Elite Flash Timer — `field_0xF0 = Rules.EliteFlashTimer` on Elite Promotion (RESOLVED 2026-04-19)

On Elite promotion, `TechnoClass::field_0xF0` (4 bytes) is overwritten with the value of
`RulesClass::field_0xBE8`. Later in `AI_Update` (at `LAB_006fac31`), the code snapshots
`field_0xF0`, does a vtable dispatch (+0x124 with arg `2`), then compares the new
`field_0xF0` against the snapshot — if the low-byte bit 1 differs AND the object is a
Building (`WhatAmI() == 6`), it calls `TacticalClass::DirtyScreenRect` +
`BuildingClass::UpdateAllAnimFacings` to force a redraw.

**RulesClass+0xBE8 = `[AudioVisual] EliteFlashTimer`** (int, frames). Verified from
`RulesClass::ReadAudioVisual` @ `0x006691E0`: `param_1[0x2fa] = ReadInt(s_EliteFlashTimer)`
→ byte offset `0x2fa × 4 = 0xBE8`. Stock rulesmd.ini:

```
EliteFlashTimer=150 ;gs Frames that a newly Elite unit will flash for
```

So `field_0xF0` is a **countdown timer** (not a bitmask). On Elite promotion it is seeded
to 150 frames. The vtable call at `+0x124` is the frame-counter tick; it decrements
`field_0xF0` and as the low-byte transitions across bit-1 boundaries (value toggles
2→0 and 3→1, i.e., every two frames), buildings get redrawn to produce the blinking
"newly elite" highlight. When the timer hits 0, the flash stops.

**Implementation impact:** cosmetic only. Implement as a 150-tick decrementing counter on
Elite promotion; the render layer uses it to draw the pulsing highlight. Non-blocking for
core veterancy mechanics.

---

## 11. Elite Weapon Override — `FUN_0070E140` @ `0x0070E140`

```c
WeaponType* GetWeapon(TechnoClass *this, int weapon_idx) {
    if (weapon_idx == -1) return NULL;
    if (IsElite(this)) {
        WeaponType *elite = GetEliteWeapon(type, weapon_idx);
        if (elite && elite->WeaponType != NULL) return elite;
    }
    return GetNormalWeapon(type, weapon_idx);
}
```

Elite-only INI keys:
- `ElitePrimary`, `EliteSecondary`, `EliteWeapon%d`
- `ElitePrimaryFireFLH`, `EliteSecondaryFireFLH`
- `EliteOccupyWeapon` (garrison override)

Current Rust impl (`combat_weapon.rs:166-171`) handles the garrison case only; the
general Primary/Secondary elite swap is not yet wired.

---

## 12. DrawVeterancyPips — `0x0070A990`

Pip frame indices on the global pip SHP (`DAT_00AC147C`):
- Rookie (vet < 1.0): no pip (early return when iVar4 == -1)
- Veteran (vet ≥ 1.0): frame `0x0E` (14)
- Elite (vet ≥ 2.0): frame `0x0F` (15)
- Special (IsRookie true, vet < 0.0): frame `0x13` (19) — "de-trained" indicator

---

## 12a. SelfHeal Eligibility — `FUN_0070BE80` (REVISED, 2026-04-19 deep-dive)

Fresh decompile of the self-heal eligibility predicate:

```c
bool SelfHealTick(TechnoClass *this) {
    TechnoTypeClass *type = this->GetType();

    // Gate A: per-type SelfHealing bypasses the ability requirement.
    if (type->+0xD14 == 0) {              // SelfHealing=no (default)
        // Gate B: must be vet/elite AND have SELF_HEAL ability.
        if (!IsVeteran(this) && !IsElite(this)) return false;
        if (IsVeteran(this) && type->+0x2A5 == 0) return false;
        if (IsElite(this)   && type->+0x2A5 == 0 && type->+0x2B7 == 0) return false;
        // Rookie-with-SelfHealing=no → never heals.
    }

    // Gate C: frame-counter modulo against Rules.SelfHeal*Frames.
    int frames = /* Math__ftol() of per-type frames selection */;
    if (g_CurrentFrameCounter % frames != 0) return false;

    // Gate D: health must be below max and non-zero.
    if (this->Health == type->Strength) return false;
    return this->Health != 0;
}
```

Key mappings (verified):
- `type->+0xD14` = **SelfHealing** bool on TechnoTypeClass (default: false). When
  `SelfHealing=yes` is set on a unit (e.g., Brute, Rhino, many infantry in stock),
  the per-type flag bypasses the veteran-ability gate — rookies of those units
  self-heal too.
- `type->+0x2A5` / `type->+0x2B7` = SELF_HEAL ability bits at index 9 (matches
  §6 table: 0x29C+9=0x2A5 and 0x2AE+9=0x2B7). ✓
- **CORRECTED 2026-04-19 deep-dive — tick source is `RepairRate × 900`, not
  `SelfHeal*Frames`.** The `Math__ftol()` call in `FUN_0070BE80` computes:

  ```
  tick_interval = (int)(Rules.+0x16E0 × 900.0)
                = (int)(RepairRate     × 900.0)
  ```

  With stock `RepairRate=.016` (minutes) → `.016 × 900 = 14.4` → **14 frames
  (~1 second)** between self-heal pulses. The 900 constant at `0x007E27F8`
  converts minutes → frames (15 fps × 60 s/min). Verified assembly:

  ```
  0070bef9: MOV EAX, [0x008871e0]        ; Rules instance
  0070befe: FLD  [EAX + 0x16e0]           ; Rules.RepairRate (double)
  0070bf04: FMUL [0x007e27f8]             ; × 900.0
  0070bf0a: CALL ftol
  0070bf11: MOV  EAX, g_CurrentFrameCounter
  0070bf17: IDIV tick_interval
  0070bf19: TEST EDX, EDX                 ; remainder == 0 ?
  ```

  RulesClass+0x16E0 corresponds to INI key `RepairRate` — written by
  `RulesClass::ReadGeneral` at `0x00670E2A`, string at `0x0083BDD0`.

  This applies **uniformly to both infantry and vehicles** with SELF_HEAL
  ability (or `SelfHealing=yes`). The earlier passes' claim that infantry
  use `Rules.SelfHealInfantryFrames` (+0x30, 50) and vehicles use
  `Rules.SelfHealUnitFrames` (+0x38, 75) for per-unit self-heal is wrong.
  Those offsets drive a different system (power-based building health — see
  subsection below).

- The function returns *eligibility* only; the actual HP increment in
  `TechnoClass::AI_Update` is always **+1 HP per trigger** (raw `INC EAX` at
  `0x006FA756` then `MOV [ESI + 0x6c], EAX`). No `SelfHealAmount` multiplier
  is applied on this path — the `Amount` INI values are not consumed here.

### What `SelfHeal*Frames/Amount` actually drive — power-based building health

Verified 2026-04-19 from `AI_Update` disassembly: these Rules offsets drive
**building auto-repair on power-surplus** and **unit damage on power-drain**,
not per-unit self-heal. The INI key names are TS-era legacy misnomers.

- **`Rules+0x30 = SelfHealInfantryFrames` (stock 50)** — modulo timer for
  **auto-repair when `HouseClass::HasPowerOutput()` is true**. At each trigger
  frame, `Health += min(type.Strength - current, House::GetTotalPowerOutput())`.
  Verified at `0x006FA8E2` (`IDIV [ECX + 0x30]`) → `GetTotalPowerOutput` at
  `0x006FA913`. Applies to BuildingClass (RTTI 6) and to TiberiumStorage-flagged
  (+0xD97) unit types.

- **`Rules+0x38 = SelfHealUnitFrames` (stock 75)** — modulo timer for the
  **power-drain branch**. When `House::HasPowerDrain()` is true, health is
  adjusted per `House::GetTotalPowerDrain()`. Verified at `0x006FA7EE`
  (`IDIV [ECX + 0x38]`) → `GetTotalPowerDrain` at `0x006FA827`. Applies to
  Unit RTTI (1) without the `+0xD97` flag.

- **`Rules+0x34` (`SelfHealInfantryAmount`, stock 20)** and **`Rules+0x3C`
  (`SelfHealUnitAmount`, stock 5)** — parsed by `RulesClass::ReadGeneral` but
  **not consumed on the self-heal/power-health paths in AI_Update**. The actual
  amount used is `House::GetTotalPowerOutput/Drain`, not these INI values.
  Possible dead TS-era fields or consumers elsewhere; grep the full binary
  before assuming they're entirely unused.

**Implementation note (unchanged):** both the `SelfHealing=yes` per-type path
AND the `VeteranAbilities/EliteAbilities` with `SELF_HEAL` path must be
honoured. Stock rulesmd.ini has ~13 units with `SelfHealing=yes`, and most
infantry/vehicle `EliteAbilities=` lines include `SELF_HEAL`.

---

## 13. Crate Veterancy — RESOLVED 2026-04-19 (fourth pass)

**Entry function:** `0x00481A00` — currently mislabelled in Ghidra as
`CellClass__Can_Enter_Cell_General`. It is actually the **crate-pickup
dispatch** (signature: `__thiscall (CellClass *cell, TechnoClass *picker)`).
The function body extends to `~0x00483393` (Ghidra's declared body end at
`0x00482453` is wrong — the decompile walks the full jumptable beyond that).

**Crate-type selection:** a weighted random roll using the weights in
`RulesClass+0x1140..0x146C` (via `DAT_0081DA8C` index array). The resulting
crate-type code (0..N) is then dispatched through a jump table at `0x004833DC`.

**Veterancy crate = dispatch index 8**, body at `0x00482972`.

### Veterancy crate branch — decompile (verified)

```c
case VETERANCY_CRATE:  // 0x00482972
    Register_heap_pool("Crate at %d,%d contains veterancy(TM)\n", cellY);

    // Walk every TechnoClass currently in DisplayLayer
    for (i = 0; i < DisplayLayer.capacity; i++) {
        techno = DisplayLayer.buffer[i];
        if (!techno) continue;
        if (techno->+0x74 == 0) continue;              // alive/on-ground flag
        if ((techno->+0x14 & 1) == 0) continue;        // FootClass-lineage bit
                                                        // (infantry/vehicle only)

        // Distance gate: picker cell → techno coords
        Sqrt_Approx(dx*dx + dy*dy + dz*dz);
        if (ftol(dist) >= Rules.+0x172C) continue;     // Rules.CrateRadius

        type = techno->GetType();
        if (type->+0xC8E == 0) continue;               // Trainable=no → skip

        // Tiers to grant from Rules weight table, read at dispatch-time.
        // For the Veterancy crate, DAT_0089EC28[8] = 1.0 → one tier per pickup.
        if (local_170 <= 0.0) continue;

        // Apply N tier bumps (N = floor(local_170)).
        // Sequential checks (NOT else-if); each call SETs the float, so the
        // later check fires false on the new state. Corrected 2026-04-19
        // (fourth pass — helper identities re-verified).
        for (n = 0; (double)n < local_170; n++) {
            // 1. Veteran (1.0 ≤ vet < 2.0) → Elite (vet = 2.0)
            if (VeterancyClass::IsVeteran(&techno->Veterancy))
                VeterancyClass::SetElite(&techno->Veterancy, 1);  // 0x007500B0
            // 2. Standard-Rookie (0.0 ≤ vet < 1.0) → Veteran (vet = 1.0)
            //    Uses FUN_0074FFC0 @ 0x0074FFC0 which checks
            //    FLOAT_007e1748 (=0.0) <= vet < _DAT_007e2ac8 (=1.0).
            if (VeterancyClass::IsStandardRookie(&techno->Veterancy))
                VeterancyClass::SetVeteran(&techno->Veterancy, 1); // 0x00750090
            // 3. Negative-Rookie (vet < 0.0) → Rookie (vet = 0.0)
            //    FUN_00750080 is the "set-to-zero" helper; re-labelled
            //    SetRookie per its body `*param_1 = 0`.
            if (VeterancyClass::IsRookie(&techno->Veterancy))
                VeterancyClass::SetRookie(&techno->Veterancy);     // 0x00750080
        }
    }

    if (HouseClass::IsHumanPlayer(owner))
        VocClass::PlayAt(Rules.CratePickupSound, crate_coord);

    // Fall through to common crate-end at LAB_004832F5 which spawns
    // g_AnimTypes_Array[DAT_0081DAD8[8]] (the veterancy-crate pickup anim)
    //   if that slot != -1.
    break;
```

### Behavior summary

- **Effect:** **Promotes ALL trainable FootClass units within `Rules.CrateRadius`**
  of the crate by **one tier** (Rookie→Veteran→Elite). Already-elite units stay
  elite.
- **Not** a float-add. **Not** an absolute set to 1.0 or 2.0. Uses the
  `VeterancyClass::SetVeteran` / `::SetElite` helpers (which internally set
  the float to 1.0f / 2.0f respectively).
- The tiers-to-grant value (`1.0` for veterancy) comes from global constant
  array `DAT_0089EC28[type]` — this is hardcoded in the binary, not an INI
  value. Modders cannot change "1 tier per veterancy crate" without binary
  patching.

### Gates (checked in order)

1. `techno->+0x74 != 0` — on-ground / alive
2. `techno->+0x14 & 1` — FootClass-lineage (infantry or vehicle; **buildings
   are excluded**, so a veterancy crate has no effect on garrisoned troops
   inside adjacent structures, only on the mobile units around it)
3. `distance(techno, crate) < Rules.CrateRadius` — AoE radius
4. `techno->GetType()->+0xC8E != 0` — **Trainable=yes** required (so
   Engineer / Spy / Dog / ADOG / Ivan are skipped)
5. `local_170 > 0.0` — guard; always true for veterancy-crate case

**Note the absence of a `DontScore` check.** `DontScore` only gates the
kill-XP attribution path (§3); the crate path only checks `Trainable`.

### Sound / anim

- Per-unit: none (contrast to Armor / Speed / Firepower crates which play
  VoxClass EVA per-unit).
- Crate-wide: `VocClass::PlayAt(Rules.CratePickupSound, crate_coord)` — only
  if the crate owner is the local human player.
- Pickup anim: `g_AnimTypes_Array[DAT_0081DAD8[8]]` — the common crate-end
  tail spawns the anim if the index is not `-1`. For veterancy crate, the
  slot index is set by the weights table (typically the generic money-crate
  anim or a pickup sparkle).

### Implementation impact (for the Rust port)

Straightforward: when a veterancy crate is picked up, scan all trainable
FootClass entities within `Rules.CrateRadius` of the crate cell and bump
their veterancy tier by one (saturating at elite). Only gate on `Trainable`
and on `FootClass-lineage`, not on `DontScore`. Play the crate pickup sound
if the picker's house is the local player. The per-unit "no sound" is
intentional — matches the binary.

---

## 14. Current Rust Implementation Status (as of 2026-04-19)

From codebase scan:

| Area | Status | Location |
|------|--------|----------|
| Veterancy field storage | IMPLEMENTED (u16 0/100/200) | `src/sim/game_entity.rs:68` |
| Veterancy component/doc | IMPLEMENTED | `src/sim/components.rs:128-132` |
| `VeteranSight` INI parse | IMPLEMENTED | `src/rules/ruleset.rs:597` |
| Sight bonus application (additive) | IMPLEMENTED | `src/sim/vision.rs:500-504` |
| Elite FireFLH keys | PARSED | `src/rules/art_data.rs:223-228` |
| `EliteOccupyWeapon` | IMPLEMENTED (garrison-only) | `src/rules/object_type.rs:790`, `src/sim/combat_weapon.rs:166-171` |
| Experience accumulation on kill | MISSING | — |
| `Add_Experience` formula | MISSING | — |
| Friendly-fire zero-XP rule | MISSING | — |
| `DontScore` / `Trainable` parse & enforce | MISSING | — |
| `VeteranRatio/Combat/Speed/Armor/ROF/Cap` parse | MISSING | — |
| `VeteranAbilities` / `EliteAbilities` parse | MISSING | — |
| Combat damage multiplier (FIREPOWER) | MISSING | `src/sim/combat.rs`, `combat_weapon.rs` |
| Armor multiplier (STRONGER) | MISSING | — |
| ROF multiplier (ROF) | MISSING | — |
| Elite Primary/Secondary weapon swap | MISSING | — |
| Self-heal system | MISSING | — |
| Veterancy pips render | MISSING | — |
| Promotion detection + EVA/sound | MISSING | — |
| `InitialVeteran` spawn-as-vet flag | MISSING | — |
| `VeteranUnits`/`Aircraft`/`Infantry` per-house | MISSING | — |
| Crate veterancy pickup | MISSING (binary now fully mapped — see §13) | — |
| Chrono Legionnaire XP path | MISSING | `src/sim/temporal*.rs` if any |

---

## 15. Key Function Summary

| Function                         | Address      | Purpose                                              |
|----------------------------------|--------------|------------------------------------------------------|
| `IsRookie`                       | `0x0074FFF0` | experience < 0                                        |
| `IsVeteran`                      | `0x0074FF90` | 1.0 ≤ experience < 2.0                                |
| `IsElite`                        | `0x00750010` | experience ≥ 2.0                                      |
| `GetVeterancyLevel`              | `0x00750030` | Elite=0 / Vet=1 / Rookie=2                            |
| `Add_Experience`                 | `0x0074FF50` | XP accumulation with Ratio/Cap                        |
| `TechnoClass::RecordKill`        | `0x00702D40` | Kill attribution + victim-tier scaling                |
| `TemporalClass::AI`              | `0x006297F0` | Chrono-Legionnaire warp tick XP                       |
| `TemporalClass::Update`          | `0x0071A760` | Chrono-Legionnaire finalization XP                    |
| `AbilityClass::FindAbilityByName`| `0x0074FEFF` | Ability string → index (18-entry table)               |
| `TechnoClass::HasWeaponAbility`  | `0x0070D0D0` | Ability flag check (vet OR elite-inherits-vet)        |
| `DrawVeterancyPips`              | `0x0070A990` | Pip SHP draw                                          |
| `FUN_0070BE80`                   | `0x0070BE80` | Self-heal eligibility                                 |
| `TechnoClass::AI_Update`         | `0x006F9E50` | Promotion detection @ `0x006FA055`                    |
| `TechnoClass::Fire_At`           | `0x006FDD60` | Damage bonus @ `0x006FE35E`                           |
| `TechnoClass::ReceiveDamage`     | `0x006FF760` | Armor mult @ `0x00701970`; retaliation @ `0x00702B91` |
| `RulesClass::ReadGeneral`        | `0x0066D530` | Parses `Veteran*` + `SelfHeal*`                       |
| `TechnoTypeClass::ReadINI`       | `0x00712170` | Parses `DontScore`, `Trainable`, ability lists        |
| `FUN_006b8ca0`                   | `0x006B8CA0` | Parses `[SpecialFlags]` incl. InitialVeteran bit 9    |

---

## 16. Key Struct Offsets

### TechnoClass
| Offset | Type | Field |
|--------|------|-------|
| 0x0F0 | int | **EliteFlashCounter** — seeded to `Rules.EliteFlashTimer` (150) on Elite promotion; decremented per tick; drives building flash redraw (see §10) |
| 0x11C | TechnoClass* | linked entity for XP attribution (mind-controller path) |
| 0x13C | int | Cached veterancy level (-1 init; 0=Elite, 1=Vet, 2=Rookie) |
| 0x150 | float | **Veterancy** (experience accumulator) |
| ~~0x164~~ | ~~int~~ | ~~"SelfHealInfantryAmount per-type multiplier"~~ — **RETRACTED 2026-04-19 (fifth pass).** This offset on TechnoClass is not a self-heal field. The original claim came from mis-reading `FUN_0050D9E0` (actually `HouseClass::GetTotalPowerOutput`, which reads `HouseClass+0x164`, NOT TechnoClass+0x164). On TechnoTypeClass, `param_1[0x164]` (int\*-indexed = byte offset 0x590) is `VoiceUndeploy` (sound index). See §20. |
| ~~0x168~~ | ~~int~~ | ~~"SelfHealUnitAmount per-type multiplier"~~ — **RETRACTED 2026-04-19.** On TechnoTypeClass, `param_1[0x168]` (= byte offset 0x5A0) is `LeaveBioReactorSound` (sound index). See §20. |
| 0x2D4 | TechnoClass* | **MissileSpawn parent back-pointer** — when killer has `MissileSpawn=yes` (TechnoTypeClass+0xD68), RecordKill routes XP to this linked parent launcher (V3 Rocket Launcher / Dreadnought / Boomer). Not a base-defense link. (corrected 2026-05-29: was "linked building (base-defense XP path)"; RecordKill decompile `decompile_function 0x00702D40` shows `unaff_retaddr[0xb5]` = offset 0xb5×4=0x2D4 consumed only on the MissileSpawn=yes branch — MISLEADING: label was stale retraction ghost from earlier "IsBaseDefense" guess) |

### TechnoTypeClass
| Offset | Type | Field |
|--------|------|-------|
| 0x29C | bool[18] | `VeteranAbilities` flags |
| 0x2AE | bool[18] | `EliteAbilities` flags |
| 0xC8E | bool | `Trainable` (default true) |
| 0xC9F | bool | `DontScore` (default false) — blocks XP + stats on kill |
| 0xD68 | bool | **`MissileSpawn`** — if true, RecordKill routes the killer's XP to its spawner at `TechnoClass+0x2D4`. Verified 2026-04-19 from TechnoTypeClass::ReadINI @ `0x00714F2F` (param_1[0x35a] store = byte offset 0x35a×4 = 0xD68). |

### RulesClass (from `[General]`)
| Offset | Type | INI Key | Stock rulesmd value |
|--------|------|---------|---------------------|
| 0x030 | int | `SelfHealInfantryFrames` | 50 |
| 0x034 | int | `SelfHealInfantryAmount` | 20 |
| 0x038 | int | `SelfHealUnitFrames` | 75 |
| 0x03C | int | `SelfHealUnitAmount` | 5 |
| 0x668 | double | `VeteranRatio` | 3.0 |
| 0x670 | double | `VeteranCombat` | 1.1 |
| 0x678 | double | `VeteranSpeed` | 1.2 |
| 0x680 | double | `VeteranSight` **(multiplicative)** | 0.0 (corrected 2026-05-29: was "(additive)"; MISLEADING — stale label; §7/§18 already state MULTIPLICATIVE; via `decompile_function 0x0070AF50`) |
| 0x688 | double | `VeteranArmor` | 1.5 |
| 0x690 | double | `VeteranROF` (delay mult) | 0.6 |
| 0x698 | double | `VeteranCap` | 2.0 |
| 0xBE8 | int | `[AudioVisual] EliteFlashTimer` (default **150**) — written to `TechnoClass+0xF0` on Elite promotion (§10). Verified 2026-04-19 in `RulesClass::ReadAudioVisual`. |

---

## 17. Open Questions (updated 2026-04-19, third pass)

1. ~~**RulesClass+0xBE8**~~ **RESOLVED 2026-04-19 (third pass).**
   = `[AudioVisual] EliteFlashTimer` (int, default 150). Seeds the post-promotion flash
   countdown. See §10 (rewritten).
2. ~~**Crate veterancy pickup**~~ **RESOLVED 2026-04-19 (fourth pass).**
   Dispatch entry is at `0x00481A00` (mislabelled in Ghidra as
   `CellClass__Can_Enter_Cell_General`; relabelled this pass to
   `CrateClass__PickupDispatch`). Veterancy-crate branch at `0x00482972`
   (jump-table index 8). **Effect: promotes all trainable FootClass-lineage
   units within `Rules.CrateRadius` of the crate by one tier (Rookie→Veteran→Elite),
   using `VeterancyClass::SetVeteran` / `::SetElite` helpers (not float add).**
   Gates: alive, FootClass bit `+0x14 & 1`, distance, `Trainable=yes`. No
   DontScore check on the crate path. Crate-pickup sound plays only for local
   player. See rewritten §13.
3. ~~**TechnoTypeClass+0xD68**~~ **RESOLVED 2026-04-19 (third pass).**
   = **`MissileSpawn`** (bool, default false). Verified in `TechnoTypeClass::ReadINI`
   at `0x00714F2F` where `CCINIClass::ReadBool(s_MissileSpawn)` stores to
   `param_1[0x35a]` (byte offset `0x35a × 4 = 0xD68`). When true, the killer is a
   missile-spawned sub-unit (e.g., V3 rocket, Aegis missile, Boomer missile), and
   RecordKill routes XP to the spawner at `killer->field_0x2D4`. See §3.
4. ~~**VeteranSight read site**~~ **RESOLVED 2026-04-19 (second pass).** Multiplicative:
   `base_sight × (altitude×0.01 + 1.0) × Rules.VeteranSight`, gated `!= 0.0` so stock
   YR is unaffected. See §7 and §7a.
5. **Per-index wiring for rarer abilities** — partial resolution:
   - idx 0 FASTER ✓ confirmed in `FootClass::GetCurrentSpeed` @ `0x004DB1A0`
   - idx 1 STRONGER ✓ `ReceiveDamage` @ `0x00701970`
   - idx 2 FIREPOWER ✓ `Fire_At` @ `0x006FE35E`
   - idx 3 SCATTER ✓ `CellClass::Scatter_Objects` @ `0x00481670` (primary) +
     `ReceiveDamage` @ `~0x00702B91` (retaliation)
   - idx 4 ROF ✓ `FUN_006FCFA0`
   - idx 5 SIGHT ✓ `TechnoClass::UpdateReveal` @ `0x0070AF50`
   - **idx 6 CLOAK ✓ NEW:** confirmed 2026-04-19 third pass in
     `TechnoClass::CloakingTick` @ `0x006FB780` — checks `type+0x2A2` (vet) and
     `type+0x2B4` (elite), gates the cloaking CD timer / visibility transitions.
     By the same pattern `TechnoClass::CanAutoCloak` @ `0x006FBDE0` and
     `TechnoClass::ShouldUncloak` @ `0x006FBCF0` also read the same offsets.
   - idx 9 SELF_HEAL ✓ `FUN_0070BE80`
   - **idx 13 FEARLESS ✓ NEW (fifth pass 2026-04-19):** confirmed in
     `InfantryClass::SetFear` @ `0x00518C00` — the function calls
     `TechnoClass::HasWeaponAbility(0xD)` at two points; when true, the
     infantry's fear counter (at `+0x28` on the InfantryClass extension
     region, accessed as `this[10].RefCount` in decompile) is NOT incremented.
     The result is the unit stays calm under fire / doesn't panic-route. Also
     gated by the static `type.+0xEBC` (`Fraidycat`) and `type.+0xEBF`
     (`NotHuman`/`IsBrave`) fields in the same function.
   - Indices 7 (TIBERIUM_PROOF), 8 (VEIN_PROOF), 10 (EXPLODES), 11 (RADAR_INVISIBLE),
     12 (SENSORS), 14 (C4), 15 (TIBERIUM_HEAL), 16 (GUARD_AREA),
     17 (CRUSHER) — consumer sites still not traced. Most are low-impact for stock YR.
     For SENSORS (12), the base sensor radius is at `TechnoTypeClass+0x5F0`
     (read by `TechnoClass::AddSensorsAt` @ `0x004DE7B0`); the ability-gated
     extension site was not located this pass.
6. **HouseClass+0x2BF (InitialVeteran propagation):** confirmed consumer in
   `InfantryClass::InitFromType` at `0x00517CC0` — reads `house->+0x2BF` AND
   `type->+0xC8E (Trainable)`. **Propagation site from the global `[SpecialFlags]`
   bit 9 → this per-house byte still not traced.** The bit-9 consumer doesn't appear
   in `HouseClass::Read_Scenario_INI` @ `0x00500B40` nor `HouseClass::Added_To_Game`
   @ `0x00502A80`. Likely set in an initializer called from `ScenarioClass::Create_Houses`
   or directly from `FUN_00689E90` (which calls `FUN_006b8ca0` to read SpecialFlags
   at scenario start). Deferred — the propagation is not load-bearing for core
   veterancy implementation (can be stubbed since stock YR leaves bit 9 = 0).
7. **NEW — SCATTER ability retaliation gate:** the `LAB_00702cfe` call to
   `vtable[0x174]` in `ReceiveDamage` only fires for vet/elite units with SCATTER,
   but its relationship to the standard non-vet retaliation pipeline is unclear.
   Rookie units clearly do retaliate in YR, so this vet-specific branch is
   supplementary, not the sole retaliation path. Needs a follow-up trace of the
   full retaliation machinery (`TechnoClass::ShouldRetaliate` + `vtable[0x174]`).
8. ~~**SelfHeal amount→anim pipeline**~~ **LARGELY RESOLVED 2026-04-19
   (fourth pass).** The tick source is `Rules.RepairRate × 900` frames (~14
   frames ≈ 1 second with stock `RepairRate=0.016`), NOT `SelfHeal*Frames`.
   Each trigger adds raw `+1 HP` in `AI_Update`. `SelfHealInfantryAmount` /
   `SelfHealUnitAmount` INI values (20 / 5) are **not consumed** on this path
   — they are misnamed legacy fields that may be dead or consumed elsewhere
   (grep confirms no references in the per-unit self-heal code). The
   `SelfHealInfantryFrames` / `SelfHealUnitFrames` values (50 / 75) drive a
   **separate** power-based building health system (see §12a sub-section on
   power-based building health). **Remaining narrow question:** whether
   `TechnoTypeClass+0x164` / `+0x168` (per-type multipliers from the earlier
   report) are still consumed anywhere, or whether they are fully dead fields.
   Non-blocking.

---

## 18. Change log — 2026-04-19 re-verification pass

This section summarizes the delta from the prior (2026-04-19 earlier-in-day)
report. Each bullet is a load-bearing correction; downstream implementation
decisions should follow these.

1. **VeteranSight is MULTIPLICATIVE, not additive.** Formula:
   `sight = type.Sight × (altitude×0.01 + 1.0) × Rules.VeteranSight` (when SIGHT
   ability is granted and `VeteranSight != 0.0`). Verified from the raw FMUL at
   `0x0070b099` inside `TechnoClass::UpdateReveal`. The additive claim in the
   earlier pass was inferred from community docs and the stock `0.0` default;
   the binary gate keeps stock YR unchanged but the formula diverges for any
   mod that sets a non-zero value. See §7 and §7a. **Rust impact:**
   `src/sim/vision.rs:500-504` uses additive — correct in stock, wrong for mods.

2. **`vtable+0x2c` is RTTI / WhatAmI, not Mission.** The earlier report's claim
   that `RecordKill`'s building-garrison XP-routing branch is gated on
   "Mission == 6" is wrong. `vtable+0x2c` is the `WhatAmI()`/RTTI-code slot —
   `6 == BuildingClass`, `1 == UnitClass`, `2 == InfantryClass`, `0xf ==
   VesselClass` (confirmed against the same slot usage in `AI_Update`). The
   branch routes XP to the garrison when the **killer itself is a building**
   (e.g., a garrisoned civilian structure firing through its occupants). See §3.

3. **Chrono Legionnaire XP is awarded ONCE at warp-out finalization, not
   per-tick.** `TemporalClass::AI` @ `0x006297F0` has a switch on the animation
   state field at `+0x48`: cases 0–3 are warp-in/spin/warp-out animations (no
   XP), case 4 is the finalization frame (XP awarded). The earlier claim that
   "XP streams incrementally" was wrong. Hook once per warp completion, not on
   every CL tick. See §3 (`Add_Experience` callers paragraph).

4. **SCATTER ability (idx 3) has TWO consumers, not one.** Primary use is
   dodge-out-of-cell in `CellClass::Scatter_Objects` @ `0x00481670` (this is the
   gameplay meaning most users recognize as "SCATTER"). Secondary use is a
   veteran-only retaliation branch in `ReceiveDamage`, calling `vtable[0x174]`.
   Earlier report listed only the retaliation use. See §6.

5. **NEW: SelfHeal full formula** — `type.+0xD14 == SelfHealing` flag bypasses
   the veteran-ability gate entirely (per-type always-heal), otherwise vet/elite
   with SELF_HEAL (idx 9, 0x2A5/0x2B7) is required. See new §12a.

6. **NEW: Base sight altitude scaling** — independent of veterancy, sight is
   always scaled by `(altitude × 0.01 + 1.0)` in `UpdateReveal`. Flying units
   see farther the higher they fly. See §7a.

7. **NEW: Per-house InitialVeteran check** — spawn-time path in
   `InfantryClass::InitFromType` reads `house->+0x2BF` to decide veteran spawn,
   not the global `[SpecialFlags]` bit directly. The propagation step from bit
   9 → `HouseClass+0x2BF` is an open question (#6). See §4.

Nothing from the earlier pass's §§1, 2, 8–11, 14–16 was invalidated. Thresholds
(1.0 / 2.0), struct offsets on TechnoClass/TechnoType/Rules, the 18-ability
table, elite weapon selection, and pip rendering all stand.

---

## 19. Change log — 2026-04-19 third pass (open-question resolution)

Purely additive to §18. Resolves three standing open questions and narrows one more.

1. **`RulesClass+0xBE8` = `[AudioVisual] EliteFlashTimer` (int, default 150).**
   Verified in `RulesClass::ReadAudioVisual` @ `0x006691E0`:
   `param_1[0x2fa] = CCINIClass::ReadInt(s_EliteFlashTimer_0083a3b4, ...)` — byte
   offset `0x2fa × 4 = 0xBE8`. `TechnoClass::field_0xF0` is therefore a
   **countdown timer** (not a bitmask): on Elite promotion it is seeded to 150
   frames; the vtable+0x124 tick decrements it; buildings redraw when the
   low-byte crosses a bit-1 boundary, producing the "newly elite" flash. §10
   rewritten. Cosmetic — safe to implement as a simple tick counter.

2. **`TechnoTypeClass+0xD68` = `MissileSpawn` (bool, default false).**
   Verified in `TechnoTypeClass::ReadINI` around `0x00714F2F`:
   `*(undefined1 *)(param_1 + 0x35a) = CCINIClass::ReadBool(s_MissileSpawn_00843798, ...)`,
   where `param_1 + 0x35a` = byte offset `0x35a × 4 = 0xD68`. Stock rulesmd.ini
   sets `MissileSpawn=yes` on V3ROCKET, DMISL, CMISL (the three child missile
   sub-units). RecordKill's third attribution branch routes XP back to the
   spawner at `killer->+0x2D4` when the killer has `MissileSpawn=yes`. §3 and
   §16 updated. Old "base-defense flag" guess retracted. (`IsBaseDefense` is a
   separate bool on `BuildingTypeClass`, at a different offset.)

3. **CLOAK ability (idx 6) consumer confirmed.** `TechnoClass::CloakingTick`
   @ `0x006FB780` reads `type+0x2A2` (vet) and `type+0x2B4` (elite) before
   running the auto-decloak timer and visibility-transition gates. By the same
   pattern `TechnoClass::CanAutoCloak` @ `0x006FBDE0` and
   `TechnoClass::ShouldUncloak` @ `0x006FBCF0` read the same offsets. Open
   question #5 partially narrowed.

4. **HouseClass+0x2BF propagation — partial.** `HouseClass::Read_Scenario_INI`
   @ `0x00500B40` and `HouseClass::Added_To_Game` @ `0x00502A80` do NOT write
   `+0x2BF`. The propagation site remains unconfirmed; expected to live inside
   `ScenarioClass::Create_Houses` or a helper called from `FUN_00689E90` right
   after SpecialFlags are read. Non-blocking (stock YR leaves bit 9 = 0). Open
   question #6 narrowed, not closed.

Unresolved after this pass: crate veterancy pickup (§13 — unregistered code
region), SCATTER retaliation-pipeline reconciliation (#7), SelfHeal amount→anim
pipeline (#8), ability indices 7/8/10/11/12/13/14/15/16/17 consumers (#5 tail).
Priority for a fourth pass would be crate pickup (`create_function` at the
block entry then decompile), since that's the last veterancy-adjacent system
with any gameplay visibility in stock YR.

---

## 19a. Deeper pass — damage chain, armor math, self-heal timer source (2026-04-19)

A subsequent deeper Ghidra pass examined the veterancy consumer sites at the
assembly level (not just the decompiled C). Each finding below is a
load-bearing correction or addition beyond §§18 and 19.

### 19a.1 FIREPOWER site — full damage chain in Fire_At

Earlier passes referenced `0x006FE35E` as "FIREPOWER applied" but did not
document the surrounding math. Verified 2026-04-19 from assembly:

```
006fe33d: FLD   [ECX + 0x188]             ; HouseClass.+0x188 (house damage mult)
006fe343: FMUL  [ESI + 0x160]             ; × this.+0x160 (per-instance mod)
006fe349: FIMUL [ESP + 0x18]              ; × base_weapon_damage
006fe34d: CALL  ftol                      ; → EDI = initial damage (int)
  ...  IsVeteran / IsElite check reads type.+0x29E (vet FIREPOWER bit)
  ...  and type.+0x2B0 (elite FIREPOWER bit) — confirmed
006fe3c8: FILD  [ESP + 0x2c]              ; current damage
006fe3cc: MOV   ECX, [0x008871e0]         ; Rules instance
006fe3d2: FMUL  [ECX + 0x670]             ; × Rules.VeteranCombat (stock 1.1)
006fe3d8: CALL  ftol
  ...  vtable[0x400] check → if true, also multiply by Rules.+0xF40
```

Complete outbound damage formula (from Fire_At, before ReceiveDamage):

```
damage = base_weapon_damage
       × house.+0x188                    // house-wide damage mult
       × this.+0x160                     // per-instance combat mod
       × (VeteranCombat if vet/elite AND FIREPOWER ability)  // × 1.1 stock
       × Rules.+0xF40 if vtable[0x400]() // further gated mult (identity TBD)
```

**FIREPOWER byte offsets confirmed from raw assembly**: vet bit at `type.+0x29E`
(0x29C+2), elite bit at `type.+0x2B0` (0x2AE+2). Matches the §6 table.

### 19a.2 STRONGER armor — VeteranArmor is a DIVISOR, damage floor-clamps to 1

Earlier passes noted that STRONGER triggers "VeteranArmor scaling" but did not
specify whether the multiplication direction was mult or div. Verified from
assembly in `TechnoClass::ReceiveDamage`:

```
00701939: FILD  [ESP + 0x14]              ; incoming damage
00701945: CALL  [vtable + 0x84]           ; GetType
00701952: CALL  HouseClass::GetArmorMultForType(house, type)   ; pushes house armor factor to FPU
00701957: FMUL  [ESI + 0x158]             ; × this.+0x158 (instance armor mod)
0070195d: FDIVR [ESP + 0x14]              ; DAMAGE / (house_armor × instance_armor)
00701961: CALL  ftol                      ; damage after house+instance armor
  ...  IsVeteran/IsElite check reads type.+0x29D (vet) / type.+0x2AF (elite)
007019c4: FILD  [EBX]                     ; current damage
007019cb: FDIV  [Rules.+0x688]            ; DAMAGE / VeteranArmor (stock 1.5)
007019d1: CALL  ftol
007019d6: MOV   [EBX], EAX
007019d8: CMP   [EBX], 0x1                ; clamp step
007019db: JGE   0x007019e3
007019dd: MOV   [EBX], 0x1                ; damage = max(damage, 1)
```

**Key facts:**
- `VeteranArmor` (Rules+0x688, stock 1.5) is applied as `damage /
  VeteranArmor`, not as a multiplier. A Veteran unit with STRONGER ability
  takes `damage / 1.5 ≈ 67%` of normal damage.
- **Final damage is floor-clamped to 1** after all mults — a STRONGER heavily-
  armored unit still always takes at least 1 HP per hit.
- Damage chain order on defense side: `house_armor_mult` → `instance_armor_mult`
  (this.+0x158) → `VeteranArmor` (div) → `clamp to ≥ 1`.

STRONGER byte offsets confirmed: `type.+0x29D` (vet), `type.+0x2AF` (elite).

### 19a.3 SelfHeal tick source — RepairRate × 900, not SelfHealFrames

**Largest correction of this pass.** The self-heal eligibility predicate
`FUN_0070BE80` (vtable[0x294] implementation) uses `Rules.RepairRate` × 900
as the tick interval, NOT `Rules.SelfHealInfantryFrames` nor
`Rules.SelfHealUnitFrames`. Verified from raw assembly:

```
0070bef9: MOV EAX, [0x008871e0]           ; Rules instance
0070befe: FLD  [EAX + 0x16e0]              ; Rules.+0x16E0 (RepairRate double)
0070bf04: FMUL [0x007e27f8]                ; × 900.0
0070bf0a: CALL ftol                        ; tick interval (int frames)
0070bf0f: MOV  ECX, EAX
0070bf11: MOV  EAX, [0x00a8ed84]           ; g_CurrentFrameCounter
0070bf17: IDIV ECX
0070bf19: TEST EDX, EDX                    ; remainder == 0 ?
```

- `Rules+0x16E0 = RepairRate` (INI `RepairRate=.016` in minutes; written by
  `RulesClass::ReadGeneral` at `0x00670E2A`; string at `0x0083BDD0`).
- Constant `900.0` at `0x007E27F8` converts **minutes → frames** (15 fps ×
  60 s/min = 900).
- With stock `RepairRate=0.016` → `0.016 × 900 = 14.4 ≈ 14 frames (~1 second)`
  between self-heal pulses.
- HP increment per trigger is raw **`+1`** in `AI_Update` @ `0x006FA756`
  (`INC EAX` then `MOV [ESI + 0x6c], EAX`). No `SelfHealAmount` multiplier
  applied on this path.

This path is used **uniformly for infantry and vehicles** that have the
SELF_HEAL veteran ability or `SelfHealing=yes` per-type flag. The prior
assumption that infantry used `Rules.SelfHealInfantryFrames` (50) and
vehicles used `Rules.SelfHealUnitFrames` (75) is incorrect.

**So what DO the `SelfHeal*Frames/Amount` Rules offsets drive?** Verified from
`AI_Update` disassembly — they drive **power-based building repair/drain**, not
self-heal:

- `Rules+0x30 = SelfHealInfantryFrames` (50) is a modulo tick for
  **building auto-repair when HouseClass::HasPowerOutput() is true**. At each
  trigger, `Health += min(max - current, House::GetTotalPowerOutput())`.
  Verified at `0x006FA8E2` (`IDIV [ECX + 0x30]`) → `GetTotalPowerOutput` at
  `0x006FA913`. Applies to Building (RTTI 6) and TiberiumStorage-flagged
  (+0xD97) unit types.
- `Rules+0x38 = SelfHealUnitFrames` (75) drives the power-drain branch. When
  `House::HasPowerDrain()`, health is adjusted by `House::GetTotalPowerDrain()`.
  Verified at `0x006FA7EE` (`IDIV [ECX + 0x38]`) → `GetTotalPowerDrain` at
  `0x006FA827`. Applies to Unit (RTTI 1) without the `+0xD97` flag.
- `Rules+0x34` (`SelfHealInfantryAmount`, 20) and `Rules+0x3C`
  (`SelfHealUnitAmount`, 5) — parsed but no consumer found on either the
  self-heal or the power-health paths. Possibly dead TS vestige, possibly used
  elsewhere.

### 19a.4 SCATTER double-consumer confirmed (extends §19 CLOAK finding)

The SCATTER ability (index 3, `type.+0x29F` / `type.+0x2B1`) is consumed in
**two** places, not one:

1. `CellClass::Scatter_Objects` @ `0x00481670` — this is the gameplay
   "dodge out of a hit cell" behavior. Units with SCATTER flee cells being
   targeted by AoE / big weapons. Also triggers if *any* elite unit is present
   in the cell — elites force-scatter everyone.
2. `TechnoClass::ReceiveDamage` retaliation branch — at `LAB_00702cfe`, a
   call through `vtable[0x174]` is reached only when the defender is
   veteran/elite AND has the SCATTER ability (or the global `Rules+0x17ED`
   override is set). This is a supplementary auto-counter-fire path on top of
   the normal `ShouldRetaliate` pipeline — not the primary retaliation (rookie
   units clearly retaliate via other paths in YR).

Prior passes listed only the second use. The primary gameplay use is #1.

### 19a.5 `IsBaseDefense` offset correction

For the record: `IsBaseDefense` (INI bool) lives at **`BuildingTypeClass+0x1706`**,
not at any offset on `TechnoTypeClass+0xD68`. Verified from
`BuildingTypeClass::ReadINI` at `0x00461010`. The prior "base-defense flag"
guess for +0xD68 was incorrect; the correct identity is `MissileSpawn` (per §19).

### 19a.6 Kill-counter layout on HouseClass

Incidentally verified during RecordKill re-read (non-veterancy but useful
context):

| HouseClass offset | Purpose |
|-------------------|---------|
| `+0x5434` | total kills by this house (any RTTI)           |
| `+0x5438` | array of kills by killed-type index            |
| `+0x5488` | total deaths / last-killed-type-id             |
| `+0x548C` | last-kill-target-type (written by RecordKill)  |
| `+0x53E4` | kill counter per RTTI (indexed by WhatAmI)     |
| `+0x54E8` | cumulative cost of all kills (summed credits)  |

These are updated in `RecordKill` irrespective of veterancy attribution; they
track kill-score for end-of-game stats, not XP.

---

## 19b. Fifth pass — vtable[0x174] identity, Speed/ROF/Pip math, edge cases

Subsequent deeper decompilation of the ability consumer sites and the
self-modification paths. Each finding is additive to §§18, 19, 19a.

### 19b.1 vtable[0x174] = UnitClass/InfantryClass::Scatter — MAJOR CORRECTION

The `vtable[0x174]` call reached from the SCATTER-ability branch in
`ReceiveDamage` was treated in §§18–19a as a "retaliation / counter-fire"
hook. That framing is wrong. The slot is resolved by:

- `UnitClass::Scatter` @ `0x00743A50` — xref'd from `0x007F5DE4` (== UnitClass
  vtable base `0x007F5C70` + `0x174`)
- `InfantryClass::Scatter` @ `0x0051D0D0` — xref'd from `0x007EB1CC` (==
  InfantryClass vtable base `0x007EB058` + `0x174`)

Both vtable bases derived by matching the known FUN_0070BE80 slot (+0x294) to
its xref set, then subtracting 0x294 from each to find each subclass's
vtable origin.

**What `UnitClass::Scatter` actually does** (decompiled): picks a random
adjacent passable cell via `FootClass::Find_Nearby_Passable_Cell`, issues
`AssignMission(MOVE)` via `vtable[0x1E8](2, 0)`, and sets a new NavCom target
via `vtable[0x480]`. It is a **movement / dodge** method — the unit flees,
it does **not** counter-fire.

So the correct semantic of the SCATTER veteran ability (index 3, offsets
0x29F / 0x2B1):

1. **`CellClass::Scatter_Objects` @ `0x00481670`** — when an AoE weapon lands
   in a cell, SCATTER-ability units in the cell move out of the blast.
2. **`ReceiveDamage` @ `~0x00702CFE`** — when a vet/elite unit with the
   SCATTER ability is damaged, it calls its own `Scatter()` method (via
   `vtable[0x174]`) to flee one cell away from the attacker's direction.

**Both sites are "dodge" behavior.** Rookie units still retaliate via the
normal `ShouldRetaliate` → target-assignment pipeline (not traced here, but
not gated on SCATTER). SCATTER is a defensive-movement ability, not an
offensive counter-fire.

The earlier report's "SCATTER controls retaliation / auto-return-fire
behavior" phrasing is retracted. Clean framing: **SCATTER ability = "actively
flee when attacked"**.

### 19b.2 FASTER / VeteranSpeed — full speed chain

Fresh disassembly of `FootClass::GetCurrentSpeed` @ `0x004DB1A0`:

```
004db1a7: CALL [vtable + 0x84]           ; GetType
004db1b6: CALL HouseClass::GetSpeedBonus ; returns double on FPU
004db1c3: CALL [vtable + 0x38C]          ; get base_speed (int)
004db1cd: FILD base_speed
004db1d1: FMUL house_speed_bonus         ; × house bonus
004db1d5: FMUL [ESI + 0x580]             ; × this.+0x580 (per-instance speed mod)
004db1db: CALL ftol                      ; speed_1
004db1e8: CALL HasWeaponAbility(0)       ; FASTER ?
004db1ed: JZ skip_veteran_boost
004db1f1: FILD speed_1
004db1fa: FMUL [Rules + 0x678]           ; × VeteranSpeed (stock 1.2)
004db200: CALL ftol                      ; speed_1 after FASTER
004db209: FILD speed_1
004db20d: FMUL [ESI + 0x578]             ; × this.+0x578 (second per-instance mod)
004db213: CALL ftol                      ; final speed
```

Full formula:
```
speed = base_speed(vtable[0x38C])
      × HouseSpeedBonus
      × this.+0x580                       // per-instance speed mod (pre-veteran)
      × (Rules.VeteranSpeed if FASTER)    // × 1.2 stock
      × this.+0x578                       // per-instance speed mod (post-veteran)

if (WhatAmI() == UnitClass AND this.+0x6CC != -1):
    speed /= 2                            // special half-speed state
```

`FASTER` ability offsets `type.+0x29C` (vet) / `type.+0x2AE` (elite) confirmed
(matches §6 table at idx 0). `VeteranSpeed` is applied as a **multiplier on
the already-house-bonus-scaled speed**, not on raw type.Speed.

### 19b.3 ROF delay — full VeteranROF site

Fresh disassembly of `FUN_006FCFA0` @ `0x006FCFA0`:

```
006fd0b5: FILD [EDI + 0xB0]               ; weapon.ROF
006fd0bf: MOV  EAX, this.Owner
006fd0c5: FMUL [EAX + 0x1A8]              ; × house ROF bonus (HouseClass+0x1A8)
006fd0cb: FIADD [ESP + 0x14]              ; + random(0, 2) variance
006fd0cf: CALL ftol                       ; base_delay
...  IsVeteran/IsElite + type.+0x2A0/+0x2B2 (ROF ability) check ...
006fd136: FILD base_delay
006fd13a: MOV  EAX, Rules
006fd13f: FMUL [EAX + 0x690]              ; × VeteranROF (stock 0.6)
006fd145: CALL ftol                       ; delay after ROF ability
...  vtable[0x400] gated divisor (for Ivan/burst?) ...
006fd183: FLD  [Rules + 0xF44]            ; load Rules.+0xF44 (float)
006fd189: FCOMP zero                       ; != 0 ?
006fd19c: FILD current_delay
006fd1a0: FDIV [Rules + 0xF44]            ; delay /= Rules.+0xF44
006fd1a6: CALL ftol
...  (this.+0x2E4 && WhatAmI != Building && Rules.+0xF50 != 0) → also divide by Rules.+0xF50 ...
```

Final formula:
```
delay = weapon.ROF × HouseROFBonus + Random(0, 2)
if (vet/elite AND ROF ability):
    delay × VeteranROF            // × 0.6 stock → faster fire
if (vtable[0x400] AND vtable[0x408]() > 0):
    delay /= vtable[0x408]()       // Ivan/burst? 
if (Rules.+0xF44 != 0):
    delay /= Rules.+0xF44
if (this.+0x2E4 AND WhatAmI != Building AND Rules.+0xF50 != 0):
    delay /= Rules.+0xF50
```

ROF byte offsets `type.+0x2A0` (vet) / `type.+0x2B2` (elite) confirmed. Note
the later divisors in the chain — `Rules.+0xF44` and `Rules.+0xF50` are
floats (INI identity not traced; likely from `[CombatDamage]` or a
mission-modifier section). These apply BEFORE the delay is returned.

### 19b.4 DrawVeterancyPips render confirmed

`TechnoClass::DrawVeterancyPips` @ `0x0070A990`:

```c
uVar1 = DAT_00AC147C;                  // global pip SHP pointer
iVar4 = -1;                             // no pip
if (IsVeteran) iVar4 = 0x0E;           // frame 14
if (IsElite)   iVar4 = 0x0F;           // frame 15
if (IsRookie)  iVar4 = 0x13;           // frame 19 — de-trained (negative vet)

if (iVar4 == -1) return;                // normal rookie draws nothing

iStack_8 = param_2->x + 5;
iStack_4 = param_2->y + 2;
if (WhatAmI() != 0xF) {                 // not a vessel
    iStack_8 += 5;
    iStack_4 += 4;
}
CC_Draw_Shape(global_pip_shp, iVar4, &iStack_8, ...);
```

- Frame indices verified **in code**: 14 (Vet), 15 (Elite), 19 (de-trained rookie).
- Pip position offset: `(+5, +2)` from the pip-anchor for vessels, `(+10, +6)`
  for everything else. The anchor (`param_2`) is computed earlier by the
  caller.
- Global pip SHP lives at `DAT_00AC147C` (a pointer set during asset init —
  likely `pips.shp` or `pips2.shp` from artmd's side).

### 19b.5 HouseClass+0x2BF — zeroed by constructor, write site still untraced

Verified `HouseClass::Constructor` @ `0x004F54A0`:

```
*(undefined1 *)((int)param_1 + 0x2bf) = 0;   // line 0x004F5750 (approx)
```

So `+0x2BF` is initialized to **false** on house creation. No propagation from
the global SpecialFlags bit-9 happens in the constructor. The earlier claim
in §4 / §19 #4 that "the SpecialFlags bit propagates to every HouseClass
at +0x2BF" still needs its write site traced. Likely candidates:

- A helper called from `FUN_00689E90` right after `FUN_006B8CA0` parses the
  SpecialFlags block.
- A per-scenario pass that loops over `g_HouseClass_Array` and sets
  `+0x2BF = (SpecialFlags >> 9) & 1`.

Non-blocking for stock YR (the default SpecialFlags has bit 9 = 0, so every
house's +0x2BF stays at the constructor-zero value). Mods that set
`[SpecialFlags] InitialVeteran=yes` would need the propagation to fire. Open
question #6 remains.

**Implementation guidance:** treat `HouseClass+0x2BF` as
"**house_initial_veteran**" (bool, default false, set from global
[SpecialFlags] InitialVeteran at scenario load). Wire both the per-country
`VeteranInfantry/Units/Aircraft` list AND this per-house bit into the
spawn-time veteran check in `InitFromType`.

### 19b.6 XP edge cases — Add_Experience behavior at boundaries

Re-read of `Add_Experience` (§3) with edge-case analysis:

```c
float new_val = (float)scaled_killed_cost
              / ((float)own_cost * (float)Rules.VeteranRatio)
              + *veterancy;
```

- **Allied kills** (`scaled_killed_cost = 0`): `0.0 / anything + current_vet
  = current_vet`. No XP awarded, no division-by-zero hazard. Verified in §3.
- **`own_cost == 0`** (an attacker TechnoType with `Cost=0`): triggers a
  division-by-zero → `+Inf` → clamped to `Rules.VeteranCap` by the trailing
  clamp. Net effect: **any kill by a zero-cost attacker instantly promotes to
  Elite**. This is likely unreachable in stock YR content (no unit has
  `Cost=0`), but is a potential mod hazard. The binary does **not** guard
  against it.
- **`scaled_killed_cost < 0`** (impossible in stock YR since `type.Cost` is
  positive and the ×2/×3 vet-scaling preserves sign): would reduce veterancy.
- **Very small kills** (e.g., `cost=1` attacker kills `cost=1` rookie):
  `new_vet = 1 / (1 × 3.0) = 0.333`. Takes 3 rookie kills to reach
  Veteran from scratch, 6 more to reach Elite. Consistent with gameplay.
- **`Rules.VeteranCap = 0`** (if modded): the clamp `cap <= new_val` would
  clamp every new_val to 0 — units would never promote. Not a crash, just
  neuters the system.
- **`Rules.VeteranRatio = 0`** (if modded): division-by-zero per kill. Same
  outcome as `own_cost == 0`.

### 19b.7 SelfHealAmount — consumers in full binary grep

Followup on §19a.3's claim that `Rules+0x34` (SelfHealInfantryAmount) and
`Rules+0x3C` (SelfHealUnitAmount) are **dead in AI_Update**. A broader grep
of the disassembly for `[Rules + 0x34]` and `[Rules + 0x3C]` reads yields no
consumers in the core tick path; a deeper pass would need to scan every
function body (not just veterancy-adjacent ones) to confirm these are truly
dead. Left as low-priority follow-up: they may be used by a mission/script
path, in the legacy Tiberian Sun auto-repair tick, or in a UI-only counter.
For per-unit self-heal purposes, they can be treated as parsed-but-unused.

### 19b.8 Open questions after this pass

Resolved:
- ✓ Veterancy crate pickup — fully decompiled (§13).
- ✓ `vtable[0x174]` — is Scatter (dodge), not retaliate (§19b.1).
- ✓ VeteranSpeed multiplicative chain (§19b.2).
- ✓ ROF delay chain with VeteranROF (§19b.3).
- ✓ Pip frames (14/15/19) and per-RTTI offsets (§19b.4).
- ✓ HouseClass+0x2BF default = 0 (constructor) (§19b.5).

Still open:
- `Rules.+0xF40` / `+0xF44` / `+0xF50` — float damage/ROF multipliers whose
  INI identity is untraced. Likely from an [CombatDamage] or mission helper
  section. Not veterancy-critical but appear adjacent in the damage chain.
- HouseClass+0x2BF write site (propagation from SpecialFlags bit 9) still
  untraced. Non-blocking for stock YR.
- `SelfHealInfantryAmount` / `SelfHealUnitAmount` — confirmed dead on
  AI_Update path; full-binary consumer grep pending.
- The normal retaliation pipeline (what fires at rookies when attacked) is
  NOT `vtable[0x174]`. Likely a `vtable[0x1F4]` (NotifyAttack) → normal
  target-assignment pipeline. Not veterancy-relevant; untraced here.

---

## 19c. Sixth pass — rare ability consumers, retaliation, spawner XP, parser

Additional decompilation of ability consumer sites, the retaliation gate,
spawner-child XP edge cases, and the INI tokenizer. Additive to §§18–19b.

### 19c.1 Ability consumer map (expanded)

Per-index verified consumer sites:

| Idx | Name              | Vet/Elite offsets | Verified consumer site |
|-----|-------------------|-------------------|-----------------------|
| 0   | FASTER            | 0x29C / 0x2AE     | `FootClass::GetCurrentSpeed` @ `0x004DB1A0` — `CALL HasWeaponAbility(0)` at `0x004DB1E8` |
| 1   | STRONGER          | 0x29D / 0x2AF     | `TechnoClass::ReceiveDamage` @ `0x007019A1` / `0x007019B6` |
| 2   | FIREPOWER         | 0x29E / 0x2B0     | `TechnoClass::Fire_At` @ `0x006FE397` / `0x006FE3B4` |
| 3   | SCATTER           | 0x29F / 0x2B1     | `CellClass::Scatter_Objects` @ `0x00481670`; `ReceiveDamage` `~0x00702BC0` |
| 4   | ROF               | 0x2A0 / 0x2B2     | `FUN_006FCFA0` @ `0x006FD10D` / `0x006FD122` |
| 5   | SIGHT             | 0x2A1 / 0x2B3     | `TechnoClass::UpdateReveal` @ `0x0070B04F` / `0x0070B068` |
| 6   | CLOAK             | 0x2A2 / 0x2B4     | `TechnoClass::CloakingTick` @ `0x006FB780` (per §19) |
| 7   | TIBERIUM_PROOF    | 0x2A3 / 0x2B5     | **Not re-verified this pass** (per-cell damage in `FootClass::PerCellProcess`?) |
| 8   | VEIN_PROOF        | 0x2A4 / 0x2B6     | **Not re-verified this pass** (vein-hole damage; TS-legacy in YR) |
| 9   | SELF_HEAL         | 0x2A5 / 0x2B7     | `FUN_0070BE80` @ `0x0070BED0` / `0x0070BEE5` (per §12a) |
| 10  | EXPLODES          | 0x2A6 / 0x2B8     | **NEW §19c:** `UnitClass::Death_Explosion` @ `0x00738680` — `CALL HasWeaponAbility(10)` — with EXPLODES, force the LAST (biggest) death anim from `type.+0x730[]` instead of a random one. Also gated by `type.+0xD15` (the `Explodes=yes` type flag) — either path triggers. |
| 11  | RADAR_INVISIBLE   | 0x2A7 / 0x2B9     | **Not re-verified this pass** (radar system gate) |
| 12  | SENSORS           | 0x2A8 / 0x2BA     | **Not re-verified this pass** (cloaked-enemy detection) |
| 13  | FEARLESS          | 0x2A9 / 0x2BB     | **NEW §19c:** `InfantryClass::SetFear` @ `0x00518C00` — `CALL HasWeaponAbility(0xD)` — FEARLESS units never raise fear counter (field_0xEC8), never panic/flee. |
| 14  | C4                | 0x2AA / 0x2BC     | **NEW §19c:** `FootClass::Mission_Capture` @ `0x004D4B20` (C4 gate for building-infiltration), `FootClass::Mission_AreaGuard` @ `0x004D6AA0` (C4-unit pre-attack), `TechnoClass::ShouldRetaliate` @ `0x007087C0` (C4 vet/elite skips retaliation when targeting building — player-controlled only) |
| 15  | TIBERIUM_HEAL     | 0x2AB / 0x2BD     | **Not re-verified this pass** (ore-cell healing; TS-legacy) |
| 16  | GUARD_AREA        | 0x2AC / 0x2BE     | **Not re-verified this pass** (area-guard behavior; the `Mission_AreaGuard` call checks C4, not GUARD_AREA) |
| 17  | CRUSHER           | 0x2AD / 0x2BF     | **NEW §19c:** `UnitClass::Can_Enter_Cell` @ `0x0073F0A0` — three `PUSH 0x11; CALL HasWeaponAbility` sites (at `0x0073F448`, `0x0073FB3A`, `0x0073FC80`). All paired with `type.+0xD28` (the static `Crusher=yes` flag) — either enables crush-through-obstruction. |

**Coverage summary:** 10 of 18 ability indices now verified with their
consumer sites at the assembly level (0, 1, 2, 3, 4, 5, 6, 9, 10, 13, 14, 17
= 12). The remaining 6 (7, 8, 11, 12, 15, 16) are either TS-legacy (VEIN_PROOF,
TIBERIUM_HEAL) or minor (RADAR_INVISIBLE, SENSORS, GUARD_AREA) or per-cell
damage (TIBERIUM_PROOF). For implementation priority, the verified 12 cover
every ability referenced by stock YR's `VeteranAbilities=` / `EliteAbilities=`
lines.

### 19c.2 `TechnoClass::ShouldRetaliate` @ `0x007087C0` — normal retaliation gate

Full decompile examined. Returns bool "should this unit auto-retaliate at
the attacker". Gates (must ALL be satisfied for retaliation):

1. `param_2` (attacker) non-null
2. `type.+0xD9A` bool is true (likely inverse of `NoAutoReturnFire=yes` or
   similar — default allows retaliation)
3. Not in `+0x2DC`, `+0x2D8`, or (`+0x1CC` without owner.+0x1EC) states (these
   are special hold/stop states)
4. No active `CaptureManager` blocking retaliation (mind-control?)
5. `+0x2D0 == 0`; AND (not player-controlled OR no current Target); AND
   mission-timer entry has flag `+8` set (auto-tune/mission allows retaliate)
6. Not allied to the attacker, and attacker's house not allied (double check)
7. `TechnoClass::GetWeaponRange(-1) > 0` — has at least one weapon
8. `vtable[0x2AC]()` returns non-zero (this is typically `CanShoot` /
   `IsAbleToFire`)
9. Damage-calc `vtable[0x3BC]()` returns not 5 and not 6 (not invulnerable)

**Veteran-specific early-exit:** when player-controlled AND attacker is a
Building (RTTI 6):
- If this unit has C4 ability (vet `type.+0x2AA` or elite `type.+0x2BC`) →
  return 0 (don't retaliate — stay on mission)
- This lets Tanya / Chrono Legionnaire / IvanBomber stick to their building
  demolition mission instead of getting distracted by return fire from
  nearby defenders.

**Human vs AI computer-controlled branching:** the final sub-gates use
`IsPlayerControl` to pick between "strict" (player units are more
conservative about retaliating — threat-score-based) and "permissive" (AI
computer units retaliate more freely). Threat-score comparison via
`Calculate_Threat_Score` can also suppress retaliation if the current Target
already scores higher than the attacker.

**Armor-immunity early-exit:** if the attacker's warhead does ≤ `_g_Const_0_01`
damage vs this unit's armor (via the Verses matrix lookup at
`type.+0xA0 + armor*8`), return 0 — don't retaliate against an ineffective
attacker.

**Relationship to SCATTER (§19b.1 resolved):** `ShouldRetaliate` is the gate
for `vtable[0x1F4]` (NotifyAttack → re-target). `vtable[0x174]` (Scatter) is
a SEPARATE effect triggered when SCATTER ability is set. **Both can fire on
the same hit** — a vet/elite unit with SCATTER will retaliate (via
ShouldRetaliate → NotifyAttack → target-assignment) AND flee (via Scatter).
Earlier framing of "SCATTER makes units flee INSTEAD of retaliate" was
wrong — they're independent.

### 19c.3 Spawned-child XP routing — Hornet vs Missile divergence

§19 identified `MissileSpawn` (TechnoType+0xD68) as the routing flag in
RecordKill's attribution chain. §19c clarifies the semantic:

| Unit                 | `Spawned=` | `MissileSpawn=` | XP recipient on kill |
|----------------------|-----------|-----------------|----------------------|
| V3ROCKET             | yes       | yes             | **Parent V3 Rocket Launcher** (via +0x2D4 back-pointer) |
| DMISL                | yes       | yes             | **Parent Dreadnought** |
| CMISL                | yes       | yes             | **Parent Boomer** |
| HORNET               | yes       | **no**          | **The Hornet itself** — keeps its own veterancy |

**Key distinction:** `MissileSpawn=yes` marks a **one-shot, non-returning
child** (missiles explode on impact). XP routes to the Trainable parent
because the child isn't around to benefit. `Spawned=yes` alone (without
`MissileSpawn`) marks a **long-lived returning child** (Hornet docks and
rearms at the Carrier). XP stays on the child — each Hornet accumulates its
own veterancy over its lifetime.

**Implementation implication:** for a faithful port:
- V3 Rocket Launchers, Dreadnoughts, and Boomers gain XP from their child
  missile kills.
- Aircraft Carriers do NOT gain XP from Hornet kills. Each Hornet tracks its
  own veterancy. When the Carrier regenerates a replacement Hornet (via
  `SpawnRegenRate`), the new Hornet starts at rookie.
- Note: Yuri's Master Mind doesn't use `Spawned=` at all — its victims are
  mind-controlled via warhead, which flows through the `killer->+0x11C`
  (mind-controller linked entity) attribution path (§3 priority 1), not
  through `MissileSpawn`.

### 19c.4 `VeteranAbilities` / `EliteAbilities` INI parser

Parser function: `FUN_00477640` @ `0x00477640`. Pattern:

```c
char buf[128];
CCINIClass::ReadString(section, key, "", buf, 128);
if (key_present) {
    for (token = strtok(buf, DELIM); token; token = strtok(NULL, DELIM)) {
        idx = AbilityClass::FindAbilityByName(token);  // case-insensitive stricmp
        if (idx != -1) flags[idx] = 1;                  // set byte
    }
} else {
    copy defaults from another Abilities struct;
}
```

Delimiter byte at `DAT_00817F70` is **`,`** (just comma, no whitespace).
- `strtok` on a comma-only delimiter does NOT trim surrounding whitespace.
- So `VeteranAbilities=FASTER, STRONGER` (note the space) → tokens are
  `"FASTER"` and `" STRONGER"`. The leading space on the second token makes
  `stricmp(" STRONGER", "STRONGER")` fail → the STRONGER flag is **silently
  dropped**. Stock rulesmd.ini is careful to use no spaces; modders beware.
- Unknown tokens (typos, custom names) are silently ignored — no error log.
- Order and duplicates don't matter — flags array is indexed by ability ID.

**Default behavior (key not present):** the function copies a default
Abilities struct passed as `param_4` — likely zero-inited (no abilities) or
a per-class inherited default (e.g., Building inherits `None`, Infantry
inherits a small set). Default not traced further in this pass.

Parser is called once per TechnoType during rulesmd load, storing 18
booleans inline on the TechnoType starting at `+0x29C` (Veteran) and
`+0x2AE` (Elite). The `uVar3 / local_84` pair is likely a "has any elite"
summary flag stored separately.

### 19c.5 Ability-table pointer layout (confirmed)

`AbilityClass::FindAbilityByName` @ `0x0074FEFF` linearly scans the pointer
table from `0x008463B8` to `0x00846400` (exclusive), that is 18 pointers ×
4 bytes = 72 bytes. Each entry is a `char*` to a null-terminated name
string. `stricmp` (`FUN_007C8D20`) is used for comparison — **case-
insensitive**.

Implication: `VeteranAbilities=faster,stronger` is equivalent to
`VeteranAbilities=FASTER,STRONGER`. The binary accepts any case.

### 19c.6 Open questions after this pass

Resolved:
- ✓ FEARLESS (13), EXPLODES (10), C4 (14), CRUSHER (17) — consumer sites
  identified (§19c.1).
- ✓ `ShouldRetaliate` gate structure — including the C4-ability early-exit
  for player-controlled units targeting buildings (§19c.2).
- ✓ Hornet vs Missile XP routing — confirmed `MissileSpawn=yes` is the
  "one-shot child" flag; Hornets keep own veterancy (§19c.3).
- ✓ `VeteranAbilities` parser — comma-delimited, case-insensitive, strict
  about whitespace (§19c.4).

Still open:
- Abilities 7 (TIBERIUM_PROOF), 8 (VEIN_PROOF), 11 (RADAR_INVISIBLE),
  12 (SENSORS), 15 (TIBERIUM_HEAL), 16 (GUARD_AREA) — consumer sites not
  individually traced. Most are TS-legacy or niche; low priority.
- `type.+0xD9A` (the ShouldRetaliate gate bool) — exact INI key untraced.
  Plausible: `AutoTargeting`, `NoAutoFire`, `NoAutoPickUp`, or similar.
- `type.+0xD28` (paired with CRUSHER ability) — exact INI key untraced.
  Plausible: `Crusher=yes` (the static per-type crusher flag).
- `type.+0xD15` (paired with EXPLODES ability) — likely `Explodes=yes`.
- The per-class default `param_4` struct for when `VeteranAbilities=` is
  missing — not traced; likely zero-abilities.
- `Rules.+0xF40 / +0xF44 / +0xF50` still unidentified (§19a carry-over).
- HouseClass+0x2BF write site still untraced (§19a/§19b carry-over).

---

## 19d. Seventh pass — TechnoType flags identified, HouseClass+0x2BF is Spy-Infiltrate

Additional decompilation resolved three TechnoType byte-flag identities, mapped
SENSORS and more ability consumers, and — most importantly — **corrected the
HouseClass+0x2BF semantic**. Prior passes assumed `+0x2BF` was the propagation
target of `[SpecialFlags] InitialVeteran`. That was wrong. `+0x2BF` is the
**Spy-infiltrates-War-Factory** flag, set by `BuildingClass::OnSpyInfiltrate`
when a spy walks into an enemy war factory.

### 19d.1 TechnoType byte-flag identities (resolved)

Three previously-unlabelled byte flags on TechnoTypeClass, all confirmed via
the ReadBool string-push pattern in `TechnoTypeClass::ReadINI`:

| Offset   | INI Key        | Used in                                |
|----------|----------------|----------------------------------------|
| `+0xD14` | `SelfHealing`  | `FUN_0070BE80` (self-heal bypass gate) |
| `+0xD15` | **`Explodes`** | `UnitClass::Death_Explosion` — paired with EXPLODES ability |
| `+0xD28` | **`Crusher`**  | `UnitClass::Can_Enter_Cell` — paired with CRUSHER ability |
| `+0xD9A` | **`CanRetaliate`** | `TechnoClass::ShouldRetaliate` — default `true`; `CanRetaliate=no` on Engineer / Dog / Spy / civilian-style units |

Verification pattern (example for `Explodes`):
```
007122bE: MOV DL, [EBP + 0xd15]           ; default value
007122c5: PUSH 0x0083355c                 ; "Explodes" string
... ReadBool call ...
007122d2: MOV [EBP + 0xd15], AL            ; write parsed value
```

### 19d.2 SENSORS (idx 12) consumer — cloak-detection

`FootClass::PerCellProcess` @ `0x004D85D0` — when a unit enters a cell, it
scans the 8-cell neighborhood for non-allied objects. For each hostile:

```c
if (type->+0xC9D /* Sensors type flag */ != 0
    || HasWeaponAbility(0xC) /* SENSORS veteran ability */) {
    this->vtable[0xFC]();   // decloak / reveal nearby cloaked enemies
}
```

So SENSORS = "I can see cloaked enemies in my adjacency". Consumed either via
static `type.+0xC9D` (units like Robot Tank that always detect) OR dynamically
via the SENSORS veteran ability (veteran/elite units gain detection).

SENSORS byte offsets on TechnoTypeClass confirmed: `type.+0x2A8` (vet),
`type.+0x2BA` (elite). Matches §6 table.

### 19d.3 `TechnoClass::GetWeapon` @ `0x0070E140` — elite weapon selection

Fresh decompile:

```c
int* GetWeapon(TechnoClass *this, int weapon_idx) {
    if (weapon_idx == -1) return NULL;
    if (IsElite(this)) {                                   // strict: veterancy ≥ 2.0
        type = this->GetType();
        elite = FUN_007177e0(type, weapon_idx);             // type->GetEliteWeapon(slot)
        if (elite != NULL && elite->WeaponPtr != NULL) {
            return elite;
        }
    }
    return FUN_007177c0(type, weapon_idx);                 // type->GetNormalWeapon(slot)
}
```

Key facts:
- **Strict Elite requirement**: `IsElite()` returns true only when veterancy
  ≥ 2.0f. Veteran units (1.0 ≤ vet < 2.0) do **not** get the elite weapon
  — they use the normal weapon with the VeteranCombat multiplier (§19a.1).
- **Fallback** to normal weapon if elite slot's `WeaponPtr` is NULL. Lets
  types define only a subset of elite weapons (e.g., `ElitePrimary=FOO` but
  no `EliteSecondary` specified).
- **Slot semantics**:
  - 0 = Primary (`Primary=` / `ElitePrimary=`)
  - 1 = Secondary (`Secondary=` / `EliteSecondary=`)
  - 2+ = additional `EliteWeapon1..N` (IFV has 17 weapon slots)
- Works for occupants too (`EliteOccupyWeapon` is handled separately in the
  garrison code — see §11).

### 19d.4 MAJOR CORRECTION: `HouseClass+0x2BF` is Spy-Infiltrate-War-Factory

Prior passes (§§4, 19, 19b) assumed `HouseClass+0x2BF` was the runtime
propagation target of the `[SpecialFlags] InitialVeteran` bit-9 flag. This
was wrong. Byte-pattern search (`88 ?? BF 02 00 00` = `MOV [reg + 0x2BF], AL`)
yielded exactly two xrefs:

1. `0x004F5867` — `HouseClass::Constructor` zero-init (known).
2. **`0x0045751E` — `BuildingClass::OnSpyInfiltrate` @ `0x004571E0`.**

Full decompile of OnSpyInfiltrate reveals the Spy-infiltrate **veteran-training**
cascade. When a spy successfully infiltrates an enemy building:

**Branch A: generic "infiltratable training building" list (Rules+0x920 array):**
```c
for (typ in Rules.VeteranBuildingList) {
    if (this->Type == typ) {
        if (type.+0x6D0 == 0) house.+0x2BE = 1;      // category 0 → +0x2BE
        else if (type.+0x6D0 == 1) house.+0x2BD = 1; // category 1 → +0x2BD
        else house.+0x2BC = 1;                        // else → +0x2BC
        house.+0x1FC = 1;                             // general spy-happened flag
    }
}
```

**Branch B: direct factory-type check (`type.+0xEB8`):**
```c
if (type.+0xEB8 == 0x10) {       // War Factory
    house.+0x2BF = 1;             // ← THIS IS THE +0x2BF WRITE
    house.+0x1FC = 1;
}
else if (type.+0xEB8 == 0x28) {  // Barracks
    house.+0x2C0 = 1;
    house.+0x1FC = 1;
}
```

So `HouseClass+0x2BF` is the **"spy infiltrated my War Factory → next vehicle I build spawns veteran"** flag. `+0x2C0` is the Barracks equivalent. `+0x2BC / +0x2BD / +0x2BE` are Rules-list-driven (likely Battle Lab / Tech Center / similar based on country).

**Where consumed — re-examined with corrected identity:**
- `InfantryClass::InitFromType` @ `0x00517CC0` checks `house.+0x2BF` — this
  is **the (apparently mislabelled) check that we saw in §4**. Given that
  `+0x2BF` is specifically the War-Factory flag, this read in Infantry init
  seems off. Either:
    a. The function at `0x00517CC0` is actually `UnitClass::InitFromType`
       (not Infantry), and the disassembler / report misidentified it.
    b. `+0x2BF` is consumed by infantry too (shared "spy happened" veteran
       boost), and `+0x2C0` is an additional per-type override.
  Decompilation of `UnitClass::InitFromType` would resolve this; not traced
  in this pass.

**Implementation impact (revised from §19b):**
- `[SpecialFlags] InitialVeteran` sets a **different** spawn-time source — it
  lives in the global SpecialFlags bitmask and is read directly at spawn
  (per §5), NOT via `HouseClass+0x2BF`.
- `HouseClass+0x2BF / +0x2C0 / +0x2BC / +0x2BD / +0x2BE` are all **spy
  infiltration** side-effects that persist on the house and apply to future
  spawns.
- The `+0x1FC` byte is a general "spy event occurred" flag (used for EVA
  announcements, HUD notifications).

So for a Rust port, the spy-infiltrate veteran-training system needs to:
1. Parse which buildings are "veteran training" — either from `Rules+0x920`
   list (identity untraced; probably Battle Lab / Tech Center Airfield) or
   via the factory-type + War Factory / Barracks check.
2. On successful spy-infiltrate, set the per-house flag(s) on the victim
   house.
3. On spawn (`InitFromType` equivalent), check the house flag and promote
   the spawning unit to Veteran if the flag is set AND the unit is
   `Trainable=yes`.
4. The house flag is **sticky** — it stays set for the rest of the match
   once set. No reset mechanic seen.

### 19d.5 Open questions after seventh pass

Resolved:
- ✓ `+0xD15 = Explodes`, `+0xD28 = Crusher`, `+0xD9A = CanRetaliate`
- ✓ SENSORS ability consumer site
- ✓ GetWeapon full logic
- ✓ HouseClass+0x2BF = Spy-Infiltrate-WarFactory flag (major retraction)

Still open:
- Rules+0x920 "veteran training building list" — identity / INI source.
  Likely populated from `BuildingType.VeteranInfantry=` / `VeteranUnits=` /
  `VeteranAircraft=` tags (inverse of the country-level keys in §4), or from
  specific hard-coded building categories. Needs an extra pass.
- Whether `InitFromType` at `0x00517CC0` is Infantry or Unit — the earlier
  labelling might be off. Decompile of UnitClass/AircraftClass InitFromType
  would disambiguate which house-flag byte each consumes.
- `Rules+0x17ED` (the "force everyone to retaliate" override in §19b) —
  not traced in any ReadXxx in this pass; not in `ReadGeneral`. Might be
  in a ReadAI or ReadRules section. Non-critical for veterancy.
- 6 remaining rare ability consumers (TIBERIUM_PROOF, VEIN_PROOF,
  RADAR_INVISIBLE, TIBERIUM_HEAL, GUARD_AREA, and the full CLOAK flow) —
  carry-over from §19c. Low-priority for stock YR.

---

## Sources

Binary decompiles (2026-04-19 initial pass):
- `Add_Experience` @ `0x0074FF50`
- `GetVeterancyLevel` (`Volume__GetCategory`) @ `0x00750030`
- `TechnoClass::RecordKill` @ `0x00702D40`
- `TechnoClass::AI_Update` @ `0x006F9E50`
- `TechnoClass::HasWeaponAbility` @ `0x0070D0D0`
- `AbilityClass::FindAbilityByName` @ `0x0074FEFF`
- `FUN_006b8b30` @ `0x006B8B30` (SpecialFlags writer)
- `FUN_006b8ca0` @ `0x006B8CA0` (SpecialFlags reader)
- `RulesClass::ReadGeneral` @ `0x0066D530` (veteran section)
- `ObjectTypeClass::ReadINI` @ `0x005F92D0` (Insignificant)
- `TechnoTypeClass::ReadINI` @ `0x00712170` (DontScore/Trainable/abilities)
- `AircraftClass::Update_Sight` @ `0x0041ADF0`
- 18 ability-table pointer entries at `0x008463B8`–`0x008463FC`

Additional decompiles (2026-04-19 re-verification pass):
- `TechnoClass::ReceiveDamage` @ `0x00701900` (STRONGER armor mult + SCATTER
  retaliation gate, full decompile)
- `FUN_006FCFA0` @ `0x006FCFA0` (ROF delay calc with veterancy, full decompile)
- `TechnoClass::UpdateReveal` @ `0x0070AF50` (sight-range getter — full
  decompile + disassembly to confirm VeteranSight is MULTIPLICATIVE)
- `CellClass::Scatter_Objects` @ `0x00481670` (SCATTER ability primary site)
- `FootClass::GetCurrentSpeed` @ `0x004DB1A0` (FASTER ability consumer)
- `FUN_0070BE80` @ `0x0070BE80` (SelfHeal eligibility — full decompile)
- `TemporalClass::AI` @ `0x006297F0` (full decompile — confirmed XP is
  case-4 only, not per-tick)
- `InfantryClass::InitFromType` @ `0x00517CC0` (per-house VeteranInfantry
  lookup + InitialVeteran gate on `house->+0x2BF`)
- Memory at `0x007E3808`, `0x007E1718`, `0x007E2800` (altitude scaling and
  zero-compare constants — `0.01`, `1.0`, `0.0` respectively)

Additional decompiles (2026-04-19 third pass):
- `RulesClass::ReadAudioVisual` @ `0x006691E0` (resolved `EliteFlashTimer` →
  Rules+0xBE8 via `param_1[0x2fa]` at the `s_EliteFlashTimer_0083a3b4` ReadInt)
- `TechnoClass::CloakingTick` @ `0x006FB780` (confirmed CLOAK idx 6 consumer
  at `type+0x2A2` / `type+0x2B4`)
- `HouseClass::Read_Scenario_INI` @ `0x00500B40`,
  `HouseClass::Added_To_Game` @ `0x00502A80` (checked — neither writes
  `+0x2BF`; propagation site still open)
- `FUN_00689E90` @ `0x00689E90` (scenario INI load — calls `FUN_006b8ca0`
  at top; no direct house byte writes here either)
- Cached decompile analysis of `TechnoTypeClass::ReadINI` around the
  Spawned/Spawns/MissileSpawn ReadBool cluster (lines 2500–2567)

INI verification:
- `ini/rulesmd.ini` — `[General]` veteran block + ability usage samples
- `ini/rulesmd.ini` — `[AudioVisual] EliteFlashTimer=150`
- `ini/rulesmd.ini` — `MissileSpawn=yes` on `[V3ROCKET]`, `[DMISL]`, `[CMISL]`
- `[SpecialFlags]` section absent in stock rulesmd.ini
- `ini/rulesmd.ini` — `[CrateRules] CrateRadius=3.0` (cells → 768 leptons)

Additional decompiles (2026-04-19 fourth pass — crate dispatch + selfheal helpers):
- `CrateClass::PickupDispatch` @ `0x00481A00` (full decompile — was mislabelled,
  now fully mapped; veterancy branch at `0x00482972`, jump table at `0x004833DC`)
- `VeterancyStruct::SetVeteran` @ `0x00750090` — `*vet = 0x3F800000` (1.0f) if
  flag, else 0
- `VeterancyStruct::SetElite` @ `0x007500B0` — `*vet = 0x40000000` (2.0f) if
  flag, else 0
- `FUN_0074FFC0` @ `0x0074FFC0` = **IsStandardRookie** (`0.0 ≤ vet < 1.0`) —
  distinct from `IsRookie` (`vet < 0.0`) at `0x0074FFF0`
- `FUN_00750080` @ `0x00750080` = **SetRookie** (`*vet = 0`) — explicit
  float-zero helper
- `RulesClass::ReadCrateRules` @ `0x0066B900` — `[CrateRules]` section parser;
  confirms `Rules+0x172C = CrateRadius` via `CCINIClass::ReadRange`
- `FUN_0050D9E0` @ `0x0050D9E0` correction: actually
  `HouseClass::GetTotalPowerOutput(HouseClass*)` returning
  `Rules+0x34 × House+0x164`. The earlier-report attribution of this function
  as "SelfHealInfantryAmount × TechnoType.+0x164" was wrong — it's the
  power-based building repair rate, NOT a per-unit self-heal amount.

---

## 20. Change log — 2026-04-19 fourth pass

Additive to §§18 / 19 / 19a / 19b. Resolves the last two "high-impact, open"
questions and corrects a long-standing mislabel.

1. **Crate veterancy pickup fully mapped.** Dispatch entry
   `CrateClass::PickupDispatch` @ `0x00481A00` (was mislabelled
   `CellClass__Can_Enter_Cell_General`; kept as-is in Ghidra since someone else
   had the same conclusion independently and renamed). Veterancy crate is
   **dispatch index 8**, body at `0x00482972`. Effect: all trainable FootClass-
   lineage units within `Rules.CrateRadius` (`=3.0` cells = 768 leptons) get
   promoted by one tier; `DontScore` is **not** checked. See rewritten §13.

2. **Crate-branch helper identities corrected.** The three tier-bump calls were
   labelled incorrectly in the initial §13 decompile:
   - `FUN_0074FFC0` = **IsStandardRookie** (`0.0 ≤ vet < 1.0`), not `IsRookie`
   - `FUN_00750080` = **SetRookie** (`*vet = 0`), not `SetVeteran`
   Sequence is now cleanly: `(vet ∈ [1,2)) → SetElite(2)`, `(vet ∈ [0,1)) →
   SetVeteran(1)`, `(vet < 0) → SetRookie(0)`. Since each SETS a literal
   float, the fall-through chain produces exactly one tier bump per loop
   iteration regardless of starting tier. §13 updated.

3. **`FUN_0050D9E0` identity correction.** It is not a self-heal scaler — it
   is `HouseClass::GetTotalPowerOutput` returning
   `Rules.SelfHealInfantryAmount(+0x34) × HouseClass.+0x164`. The earlier
   `§12a` power-based building-health discussion already used the correct
   interpretation, but the earlier report's struct-offset table described
   `TechnoTypeClass.+0x164` as "SelfHealInfantryAmount per-type multiplier"
   — that was wrong. **TechnoTypeClass.+0x164 / +0x168 per-type multipliers
   are not consumed on the self-heal path in `AI_Update`.** Whether they are
   consumed anywhere at all is an open question; safe default is to treat
   them as dead/legacy. §16 TechnoClass row (and the 0x164/0x168 entries on
   the TechnoType side) should be re-marked as "possibly dead" in a future
   cleanup pass.

4. **Open question #8 (SelfHeal amount pipeline) largely closed** — see §17
   item 8 updated in this pass. Tick cadence is
   `Rules.RepairRate × 900` (stock ~14 frames); each trigger adds raw `+1 HP`;
   `SelfHeal*Amount` INI values are not consumed on this path.

Remaining open after this pass:
- HouseClass+0x2BF propagation (#6) — narrowed but unresolved.
- SCATTER retaliation full pipeline (#7) — not examined further.
- Ability-index consumers 7/8/10/11/12/13/14/15/16/17 (#5 tail) — some narrowed
  in §19b but full consumer traces remain.
- `Rules.+0xF40 / +0xF44 / +0xF50` (§19a carry-over).

Nothing from prior passes was invalidated. Thresholds, offsets, ability table,
and all §§1–12 findings stand.

---

## 21. Change log — 2026-04-19 fifth pass

Additive to §§18 / 19 / 19a / 19b / 20. Resolves one more ability consumer,
narrows two open questions, and retracts a lingering false-positive.

1. **FEARLESS ability (idx 13) consumer found.** `InfantryClass::SetFear` @
   `0x00518C00` calls `TechnoClass::HasWeaponAbility(0xD)` at two early-return
   sites. When true, the infantry's fear counter is NOT incremented — the
   unit stays calm under fire. The fear counter itself sits on the
   InfantryClass extension region (decompile shows `this[10].RefCount`,
   approximately `TechnoClass+0x28` in the infantry sub-struct). Same function
   also checks the static `type.+0xEBC` (likely `Fraidycat=yes`) and
   `type.+0xEBF` (likely `NotHuman=yes` / `IsBrave=yes`) bools for additional
   gate conditions. §17 item 5 updated.

2. **TechnoClass+0x164 / +0x168 "self-heal per-type multipliers" RETRACTED.**
   Never existed as such. The original attribution traced back to misreading
   `FUN_0050D9E0` — which is `HouseClass::GetTotalPowerOutput`, and reads
   `HouseClass+0x164` (power counter), NOT `TechnoClass+0x164`. Cross-check
   in `TechnoTypeClass::ReadINI`: `param_1[0x164]` (int\*-indexed = byte
   offset 0x590) stores **`VoiceUndeploy`** (sound index), and
   `param_1[0x168]` (= byte offset 0x5A0) stores **`LeaveBioReactorSound`**
   (sound index). Neither has any self-heal role. §16 TechnoClass table
   entries struck through with retraction note.

3. **HouseClass+0x2BF propagation — narrowed further, still open.** Verified
   `HouseClass::Constructor` @ `0x004F54A0` initializes `+0x2BF = 0`. Checked
   additionally: `ScenarioClass::Create_Houses` @ `0x00687F10` does NOT write
   `+0x2BF`. Along with the previously-checked `Read_Scenario_INI` and
   `Added_To_Game`, all the main scenario-init paths are ruled out. The
   propagation site must be in a later initializer — most likely
   `ScenarioClass::Full_Init` @ `0x00686B20`, `ScenarioClass::Post_Map_Init` @
   `0x00686890`, or `ScenarioClass::Start_Scenario` @ `0x00683AB0`. Deferred
   — non-blocking for stock YR (bit 9 = 0 by default).

4. **Rules+0xF40 / +0xF44 / +0xF50 — not in ReadGeneral.** Scanned the full
   cached `RulesClass::ReadGeneral` decompile for literal offsets 0xF40–0xF54:
   no hits. The writer must be in a different `RulesClass::Read*` function
   (`ReadAudioVisual`, `ReadMultiplayerDialogSettings`, `ReadSpecialWeapons`,
   `ReadIQ`, `ReadJumpjetControls`, or `ReadCrateRules`). Low priority given
   these show up only in the `FUN_006FCFA0` (ROF-calc) edge paths and are
   already gated by `!= 0.0` / `< 0.0` checks that leave the stock behavior
   untouched.

Remaining open after this pass:
- HouseClass+0x2BF propagation (#6) — see item 3 above.
- SCATTER retaliation full pipeline (#7) — not examined further.
- Rules+0xF40 / +0xF44 / +0xF50 — see item 4 above.
- Ability indices 7 (TIBERIUM_PROOF), 8 (VEIN_PROOF), 10 (EXPLODES),
  11 (RADAR_INVISIBLE), 12 (SENSORS), 14 (C4), 15 (TIBERIUM_HEAL),
  16 (GUARD_AREA), 17 (CRUSHER) consumer sites — none individually traced.
  Low-priority: most are either cosmetic (EXPLODES, RADAR_INVISIBLE) or don't
  appear in stock YR `VeteranAbilities=` lines.

Nothing from prior passes was invalidated. All core findings (thresholds,
formulas, offsets, the 18-ability table, promotion detection, crate pickup)
stand.

---

---

## 22. Change log — 2026-04-19 eighth pass (follow-up verification)

Focused pass to audit "are we truly covered?" Yielded four small but concrete
closures.

1. **`VeteranPowerRatio` is NOT an INI key.** User asked about it as a
   potential companion to `VeteranRatio`. Verified:
   - `mcp__ghidra-mcp__search_strings` for `VeteranPowerRatio|^VeteranPower$|PowerRatio`
     → zero matches.
   - `grep -iE "VeteranPowerRatio|VeteranPower"` against `ini/rulesmd.ini` and
     `ini/rules.ini` → zero matches.
   Only `VeteranRatio` (Rules+0x668, stock 3.0) exists. Any Rust port should
   NOT add a `VeteranPowerRatio` parser — it would be a phantom key. Modders
   occasionally type the name by mistake; the engine silently ignores it.

2. **`Rules+0xF40` identity RESOLVED = `[CombatDamage] OccupyDamageMultiplier`.**
   §21 item 4 deferred this as "not in ReadGeneral — low priority". String
   `OccupyDamageMultiplier` at `0x0083B08C` has exactly one xref:
   `FUN_0066BBB0 @ 0x0066C682` (the `[CombatDamage]` rules parser, separate
   from `ReadGeneral`). Stock YR value in `[CombatDamage]` is **2.0**.
   `FUN_006FCFA0` (ROF calc) gates its use behind a `vtable[0x400]` call —
   by the pattern of adjacent checks this is `Is_Occupying_Building()` /
   "am I a garrison occupant firing through a building?" The earlier phrasing
   in §19a.1 ("further gated mult, identity TBD") is now filled in: if the
   firer is a garrison occupant, outgoing damage is multiplied by
   `OccupyDamageMultiplier` on top of the normal FIREPOWER path.
   
   Adjacent offsets (same section, same parser):
   - `+0xF40` = `OccupyDamageMultiplier` (2.0)
   - `+0xF48` = `OccupyWeaponRange` (garrison range bonus — from
     GARRISON_SYSTEM_GHIDRA_REPORT)
   - `+0xF4C` = `BunkerDamageMultiplier`
   - `+0xF58` = `OpenToppedDamageMultiplier`
   - `+0xF5C` = `OpenToppedRangeBonus`

3. **`FUN_005F5DD0` identified = `TechnoClass::GetHealthStatus` (renamed
   in Ghidra).** Returns a three-state health indicator:
   ```c
   0 if health/max ≤ Rules.+0x1708   // RedHealthPercent (stock 0.25)
   1 if health/max ≤ Rules.+0x1700   // YellowHealthPercent (stock 0.5)
   2 otherwise                        // green / healthy
   ```
   This resolves the gating expression in §3 for Chrono Legionnaire XP:
   "XP awarded if target is red-health (0) OR target is yellow-health (1)
   AND the CL is already elite." The CL gets a free XP kill on near-dead
   targets, and elites get slightly more lenient XP (anything not-green).
   Full-HP targets vaporized by a CL do NOT award XP — gameplay note for
   the Rust port.

4. **Veterancy save/load = whole-object blob serialization.** Neither
   `TechnoClass::Save @ 0x0070C270`, `MissionClass::Save @ 0x0065AB10`,
   nor `ObjectClass::Save @ 0x005F6250` explicitly writes the +0x150
   veterancy float. This is Westwood's standard pattern: each class's
   `Save` method only serializes **pointer-remap fields** and dynamic
   sub-structures (trailers, arrays, pointer-linked objects). The raw
   class bytes — including the veterancy float, current gunner slot,
   cached weapon pointer, gattling state, field_0x82, etc. — are written
   as a blob elsewhere (via the stream's block writer, not traced in this
   pass). Implication for the Rust port: veterancy rides along with
   whatever save mechanism covers the whole entity; no separate save
   hook is required, AS LONG AS the save serializer includes the field.

5. **SetVeteran / SetElite DEMOTE SEMANTICS (emphasis).** §20 mentioned
   that these helpers take a flag and set to 0 when false, but downstream
   readers have been treating them as "SetVeteran = always 1.0". For
   correctness, implementations must match:
   ```c
   SetVeteran(vet*, flag): *vet = flag ? 1.0f : 0.0f;
   SetElite(vet*,   flag): *vet = flag ? 2.0f : 0.0f;
   ```
   `flag == 0` is a **demote-to-rookie** path. Callers that pass `1`
   promote; callers that pass `0` demote. The crate pickup calls with
   default `1` (promote). Some weapons / crates / scripts may call with
   `0` to reset — needs caller audit to confirm nobody actually does
   (the flag is a stdcall arg so easily missed in cursory reviews).

6. **Labels applied in Ghidra this pass (saved):**
   - `0x00750080` → `VeterancyClass__Reset` (was `FUN_00750080`)
   - `0x0074FFC0` → `VeterancyClass__IsNormalRookie` (was `FUN_0074FFC0`;
     §20 used "IsStandardRookie" in prose — synonyms, pick one)
   - `0x005F5DD0` → `TechnoClass__GetHealthStatus` (was `FUN_005F5DD0`)

### Still open after this pass

- HouseClass+0x2BF propagation (#6): narrowed to `ScenarioClass::Full_Init`
  / `Post_Map_Init` / `Start_Scenario`, not yet pinpointed. Non-blocking.
- SCATTER retaliation pipeline (#7): unchanged.
- Remaining rare ability-index consumer sites (idx 7 TIBERIUM_PROOF, 8
  VEIN_PROOF, 10 EXPLODES, 11 RADAR_INVISIBLE, 14 C4, 15 TIBERIUM_HEAL,
  16 GUARD_AREA, 17 CRUSHER) — untraced but low-impact. CLOAK flow beyond
  `CloakingTick` also untraced.
- `weapon+0x174` flag identity (gates Chrono Legionnaire XP in case-4)
  still unknown — `74 01 00 00` byte-pattern search returned hundreds of
  false-positives (generic `0x174` constant); proper approach would be
  grep-for-string-push in WeaponTypeClass::ReadINI but not pursued this
  pass. Non-blocking.

### Audit conclusion

The doc now covers: XP formula, all thresholds, all 7 [General] Veteran
multipliers (and their correct multiplicative-vs-divisive semantics),
18-ability table with 10+ confirmed consumer sites, promotion detection,
EliteFlashTimer, InitialVeteran, VeteranUnits/Aircraft/Infantry per-house,
Chrono Legionnaire XP (incl. health-status gate), MissileSpawn XP
re-attribution, garrison-occupant XP re-attribution, crate pickup, elite
weapon override, self-heal (cadence, HP delta, what the SelfHeal*Frames
fields REALLY drive), SENSORS / FEARLESS / CLOAK / SCATTER consumer
sites, and now OccupyDamageMultiplier identity + save/load pattern.

Adequate to `/brainstorm` or `/write-plan` veterancy promotion. Remaining
open items are edge cases / rare abilities / mod-only paths.

---

Prior report: 2026-03-22 original of this document (superseded by this update).
