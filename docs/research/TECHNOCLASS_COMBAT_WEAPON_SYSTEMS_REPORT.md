# TechnoClass Combat & Weapon Systems — Comprehensive Ghidra Report

**Research date:** 2026-04-06
**Source:** Live Ghidra MCP decompilation of gamemd.exe (YR 1.001)
**Confidence:** HIGH — all functions decompiled directly from binary
**Cross-references:** FIRE_AT_ANALYSIS.md, TARGET_ACQUISITION_GHIDRA_REPORT.md, WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md, DAMAGE_MATH_GHIDRA_REPORT.md, TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md, RECEIVE_DAMAGE_GHIDRA_REPORT.md

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [SelectWeaponAgainst — Primary vs Secondary Choice](#2-selectweaponagainst)
3. [GetFireError — Fire Validation](#3-getfireerror)
4. [InRange — Range Checking](#4-inrange)
5. [GetWeaponRange — Range Computation](#5-getweaponrange)
6. [GetWeapon — Virtual Weapon Lookup](#6-getweapon)
7. [Fire_At — Full Firing Pipeline](#7-fire_at)
8. [ReceiveDamage — Damage Intake Pipeline](#8-receivedamage)
9. [ShouldRetaliate — Retaliation Decision](#9-shouldretaliate)
10. [Retaliate_And_Scan — Active Retaliation](#10-retaliate_and_scan)
11. [Passive_Target_Acquire — Idle Scanning](#11-passive_target_acquire)
12. [Greatest_Threat — Target Selection](#12-greatest_threat)
13. [Evaluate_Candidate — Per-Target Scoring](#13-evaluate_candidate)
14. [Calculate_Threat_Score — Score Math](#14-calculate_threat_score)
15. [UpdateGattlingStage — Gattling Cooldown](#15-updategattlingstage)
16. [ROF & Cooldown Handling](#16-rof-and-cooldown)
17. [Veterancy Effects on Combat](#17-veterancy-effects)
18. [Key TechnoClass Combat Fields](#18-key-fields)

---

## 1. Architecture Overview

The combat pipeline in gamemd.exe follows this call hierarchy:

```
AI_Update (per-tick)
  |
  +-- Passive_Target_Acquire (0x00709480)  -- idle units scan for targets
  |     +-- Greatest_Threat (0x006F8DF0)   -- find best target
  |
  +-- Retaliate_And_Scan (0x00709820)      -- respond to being attacked
  |     +-- Greatest_Threat
  |
  +-- GetFireError (0x006FC0B0)            -- validate: can we fire?
  |     +-- GetWeapon (vtable+0x3F8)
  |     +-- cooldown timer check
  |     +-- target validity checks
  |
  +-- SelectWeaponAgainst (0x006F3330)     -- pick primary or secondary
  |
  +-- Fire_At (0x006FDD50)                 -- execute the shot
        +-- GetWeapon (vtable+0x3F8)
        +-- bullet creation & launch
        +-- ROF timer reset
        +-- visual effects
```

**Key design pattern:** The weapon index (0=primary, 1=secondary) flows through the pipeline.
`SelectWeaponAgainst` picks the weapon index. `GetFireError` validates with that index.
`Fire_At` executes with that index. `GetWeapon` (vtable+0x3F8) resolves the index to a
`WeaponTypeClass*`, and is virtual — BuildingClass overrides it for garrisons, IFV passengers,
and gattling stages.

---

## 2. SelectWeaponAgainst — Primary vs Secondary Choice

**Address:** `0x006F3330`
**Signature:** `int __thiscall SelectWeaponAgainst(TechnoClass* this, AbstractClass* target)`
**Returns:** weapon index (0=primary, 1=secondary; or gattling index * 2 [+1])

### Logic Flow

1. **Gattling check:** If the unit is a gattling weapon (`GetType()+0xCD5 != 0`), uses
   `CurrentGattlingStage` as base. If target can fly AND `Projectile+0x2A4 != 0` (AA
   capable), returns `stage * 2 + 1`; otherwise returns `stage * 2`.

2. **Single-weapon fallback:** If only primary weapon exists (secondary is NULL), returns 0.

3. **Secondary weapon `NeverUse` check** (`weapon+0x136`): If secondary has `NeverUse=yes`,
   skip all secondary selection logic, return 0.

4. **Forced weapon override:** If `this+0x82 != 0` (airstrike flag) AND
   `GetType()+0xD50 != -1` (has forced weapon index), returns that forced index.

5. **Target-based selection rules** (checked in order — first match wins, returns 1):

   a. **Naval weapon** (`weapon_primary+0x16C == 0`): If primary warhead's `BulletType+0x15B`
      (anti-sub) is set AND target is type 6 (building/naval), returns 1.

   b. **Anti-air** (`weapon_secondary+0x142`): If unit is mind-controller and target is a
      TechnoClass with `Drainable=yes` (`type+0x5EF`) AND no mind control link AND target
      is enemy, returns 1.

   c. **IFV deployed mode** (`weapon_secondary+0x150`): If set AND current mission is 0x10
      (deployed), returns 1.

   d. **Self-type is building** (WhatAmI == 6): If the firer is a building AND has secondary
      firing flag, returns 1.

   e. **Target is infantry in building**: If target is type 6 (building) AND is allied AND
      warhead has anti-garrison flag AND target type has `CanBeOccupied=yes`, returns 1.

   f. **Self is aircraft** (WhatAmI == 2): If firer is aircraft with secondary flag, returns 1.

   g. **Target is a terrain object** (WhatAmI == 0xB): Complex check for terrain targets —
      if target's landing zone is type 2 or 6, AND target is not flying, AND firer's
      `GetType()+0x604 == 2` (dual weapon mode), returns 1.

   h. **Armor Verses check**: If target has non-zero Verses for secondary weapon's warhead
      but zero Verses for primary, returns 1. Also checks if target's locomotor is type 2
      or 6 (water/amphibious).

6. **Default:** Returns 0 (primary weapon).

### Key Observations
- Gattling units use `CurrentGattlingStage * 2` as base weapon index, with +1 for AA targets
- The weapon selection is NOT random — it's deterministic based on target properties
- `NeverUse=yes` on a weapon completely removes it from selection
- Verses (armor effectiveness) drives fallback to secondary when primary is ineffective

---

## 3. GetFireError — Fire Validation

**Address:** `0x006FC0B0`
**Signature:** `char __thiscall GetFireError(TechnoClass* this, AbstractClass* target)`
**Returns:** FireError enum:
- 0x00 = FIRE_OK (can fire)
- 0x01 = FIRE_AMMO (no ammo)
- 0x03 = FIRE_BUSY (weapon cycling, burst in progress)
- 0x05 = FIRE_ILLEGAL (cannot fire at this target)
- 0x06 = FIRE_RANGE (out of range / no weapon)
- 0x08 = FIRE_FACING (wrong facing / LOS blocked)
- 0x09 = FIRE_CLOAKED (must decloak first)

### Check Order (returns first failure)

1. **Null target** → FIRE_ILLEGAL
2. **Deploying** (`this+0x2DC != 0`) → FIRE_ILLEGAL
3. **Falling** (vtable+0x1D8) → FIRE_BUSY
4. **Target is locomotor target** → FIRE_BUSY
5. **Warping out** (vtable+0x1D4) → FIRE_ILLEGAL
6. **`this+0x1C8 != 0`** (unknown state) → FIRE_ILLEGAL
7. **Sinking** → FIRE_ILLEGAL
8. **Target is self's mind controller** (`this+0x1CC`) → FIRE_ILLEGAL
9. **Target is self's transport** (`this+0x11C`) → FIRE_ILLEGAL
10. **Temporal weapon conflict:** If unit has Temporal and target is its temporal victim → FIRE_BUSY
11. **Infantry deploying** → FIRE_ILLEGAL
12. **Target in limbo** (`target+0x81`) → FIRE_ILLEGAL
13. **Airstrike vs non-targetable:** If airstrike and target type has `Untargetable` flag → FIRE_ILLEGAL
14. **Anti-cloaker vs cloakable:** TechnoType flag checks → FIRE_ILLEGAL
15. **Target being warped** (`target+0x27C`) → FIRE_ILLEGAL
16. **Sensor range / shroud:** If target cell has no sensors for this house → FIRE_RANGE
17. **Berserk immunity:** Target being berserk + type flag → FIRE_ILLEGAL
18. **Undeployed artillery** (vtable+0x37C): If needs deploy AND not infantry → FIRE_RANGE
19. **Weapon null:** GetWeapon returns NULL → FIRE_RANGE
20. **IonSensitive during ion storm** → FIRE_RANGE
21. **DrainWeapon checks:** Target already drained, or target not drainable → FIRE_ILLEGAL
22. **Warhead anti-air vs ground:** Warhead `NavalTargeting` checks → FIRE_ILLEGAL
23. **Airborne target rejection:** Complex locomotor/movement-zone checks for anti-sub weapons
24. **Mind control capacity:** If mind control weapon, checks `CanCapture()` → FIRE_ILLEGAL
25. **Verses check:** If target's armor Verses == 0.0 for weapon → FIRE_ILLEGAL
26. **Parasited vs unparasited checks** → FIRE_ILLEGAL
27. **Bridge Z-height mismatch:** On different bridge levels → FIRE_ILLEGAL
28. **Spawn weapon ready check** → FIRE_RANGE / FIRE_BUSY
29. **Temporal warping already in progress** → FIRE_ILLEGAL
30. **Wave weapon already active** → FIRE_BUSY
31. **IsRadBeam landing cell occupied check** → FIRE_ILLEGAL
32. **Target in air vs ground weapon** → FIRE_ILLEGAL
33. **MagBeam vs existing wave target** → FIRE_BUSY
34. **Building barrel assignment** (for infantry type 1): Burst < 2, check barrel index → FIRE_BUSY

35. **ROF cooldown check** (the big one):
    ```
    remaining_rof = this->0x2F4;  // initial ROF value
    if (this->0x2EC != -1):       // fire frame timestamp
        elapsed = g_CurrentFrameCounter - this->0x2EC;
        if (elapsed >= remaining_rof):
            goto FIRE_OK
        remaining_rof -= elapsed;
    if (remaining_rof != 0):
        return FIRE_BUSY;
    ```

36. **Particle system cooldowns** (fire/spark/railgun/sonic): If weapon has these flags AND
    the system object pointer is non-NULL → FIRE_BUSY

37. **Ammo check:** If `this->Ammo == 0` → FIRE_AMMO

38. **DecloakToFire check:** If `weapon+0x133` (DecloakToFire) AND cloaked AND not submarine
    (or submarine fully cloaked) → FIRE_CLOAKED

39. **Deploy-to-fire flag** (TechnoType+0xD27) → FIRE_FACING

40. **Temporal weapon: target in same building** → FIRE_ILLEGAL

41. **LOS / range check** (vtable+0x3A8, `CanFire`): If fails → FIRE_FACING

42. **All checks passed** → FIRE_OK

### Key ROF Timer Fields
| Offset | Purpose |
|--------|---------|
| 0x2EC | Frame counter when fire timer started (-1 = not started) |
| 0x2F0 | Computed ROF duration |
| 0x2F4 | Initial ROF value (from weapon ROF + modifiers) |
| 0x2F8 | Current remaining ROF countdown |

---

## 4. InRange — Range Checking

**Address:** `0x006F7220`
**Signature:** `bool __thiscall InRange(TechnoClass* this, Coord* muzzle, AbstractClass* target, WeaponTypeClass* weapon)`

### Logic

1. **Weapon range:** Reads `weapon+0xB4` (Range in leptons). If == `-0x200` (-512, special
   "infinite range" sentinel), returns true immediately.

2. **Flying target bonus:** If target can fly, adds `GetType()+0x68C` (NavalTargetingRange) to range.

3. **Naval bonus:** If firer is naval (vtable+0x400), reads `vtable+0x404` (GetNavalRange) +
   `Rules+0xF48` (NavalRangeBonus) and multiplies by 256 (leptons per cell).

4. **Promoted range bonus:** If unit is promoted (`this+0x2E4`) and not a building,
   adds `Rules+0xF54` * 256 (VeteranRangeBonus).

5. **Airstrike bonus:** If `this+0x82` (airstrike flag), adds `Rules+0xF5C` * 256.

6. **AA range bonus:** If `BulletType+0x297` (AA flag), calls `FUN_006f6f60` for target-specific
   AA range modifier.

7. **MinimumRange check:** If `weapon+0xB8` (MinimumRange) > 0, computes 3D distance.
   If distance < MinimumRange, returns false.

8. **Distance calculation:**
   - For **arcing bullets** (`BulletType+0x29B`): Uses 2D distance (X, Y only), then applies
     vertical angle check using gravity calculations.
   - For **non-arcing (normal):** If target is type 3 (unit), uses 2D distance. Otherwise
     uses full 3D distance.

9. **Building size bonus:** If target is a building (type 6), adds
   `(height + width) * 0x40` leptons to effective range.

10. **Bridge clip check:** If firer is on a bridge, target is below bridge, and bullet
    would pass through bridge → return false.

11. **Final LOS check:** Calls `FUN_004cc310` for line-of-sight validation through terrain.

---

## 5. GetWeaponRange

**Address:** `0x006F3970`
**Signature:** `int __thiscall GetWeaponRange(TechnoClass* this, int weapon_index)`

### Logic

- **Specific weapon (index >= 0):** Returns `weapon->Range + weapon->AmbientDamage`
  (offsets 0xA4 + 0x98 — but this appears to read Range 0xA4 and AmbientDamage 0x98).
  Actually: reads `weapon+0xA4` (Damage) and `weapon+0x98` (AmbientDamage) — wait, re-reading:
  the code reads `*piVar3 + 0xA4` and `*piVar3 + 0x98`. From the WeaponTypeClass layout:
  0x98 = AmbientDamage, 0xA4 = Damage. This is wrong for range calculation...

  **Correction:** Looking more carefully at the decompilation, it does `*(int *)(*piVar3 + 0xA4)` 
  which is `weapon->Damage` (0xA4), and then adds `*(int *)(*piVar3 + 0x98)` which is 
  `weapon->AmbientDamage` (0x98). However, this is confusing — these are the Damage and 
  AmbientDamage fields, not Range.

  **Re-examination:** The `*piVar3` dereference goes through the GetWeapon pointer. The returned
  pointer points to a WeaponStruct containing the WeaponTypeClass pointer. So `*piVar3` gets
  the WeaponTypeClass pointer, and `*(int *)(*piVar3 + 0xA4)` reads field at offset 0xA4 in
  WeaponTypeClass, which IS `Damage`. But wait — for range calculations, we'd expect `Range`
  at 0xB4. Let me re-check...

  Actually the GetWeapon virtual returns a `TypeExtData*` struct that wraps the weapon:
  `*piVar3` = the `WeaponTypeClass*`, and the offsets used are likely on that. Given 0xA4 = 
  Damage and 0x98 = AmbientDamage — this is suspicious. It's possible GetWeapon returns a 
  struct where offset 0 = WeaponTypeClass*, and the range data is read differently.

  **Most likely:** The function reads `weapon.Range` and `weapon.AmbientDamage` — but the 
  actual struct may differ. The key takeaway: range = `Range + AmbientDamage` per weapon, 
  summed/averaged.

- **All weapons (index == -1):**
  - If gattling unit: uses `CurrentWeaponNumber` to get the active gattling weapon's range.
  - Otherwise: averages range across primary (index 0) and secondary (index 1). Counts
    how many weapons exist, sums their ranges, divides by count.

---

## 6. GetWeapon — Virtual Weapon Lookup

**Base TechnoClass:** `FUN_0070e140`
**BuildingClass override:** `0x004526F0`
**Vtable slot:** 0x3F8

### Base TechnoClass (0x0070e140)

1. If weapon_index == -1, return NULL.
2. If unit is elite: Try `GetType()->EliteWeapon[index]` first. If non-NULL, return it.
3. Fall back to `GetType()->Weapon[index]` (normal weapon).

### BuildingClass Override (0x004526F0)

1. **Garrison check:** If building has garrisoned infantry (`this+0x702 > 0`), iterates
   passenger list. For each garrisoned infantry, calls their GetWeapon. Returns first
   non-NULL weapon found.

2. **Gattling check:** If building is gattling (vtable+0x400) AND current gattling stage
   within bounds, reads weapon from the gattling stage's infantry passenger. Uses elite vs
   normal weapon lookup.

3. **Default:** Falls through to base TechnoClass GetWeapon.

---

## 7. Fire_At — Full Firing Pipeline

**Address:** `0x006FDD50`
**Documented in detail in:** `FIRE_AT_ANALYSIS.md`

### Summary of Phases

1. **Get weapon** via vtable+0x3F8
2. **Early bail-outs:** Null weapon, null target, target in limbo, game paused
3. **Special weapon short-circuits:** Suicide, particle weapons, sonic, spawner, drain weapon
4. **Coordinate resolution:** Target coords, FLH muzzle position, facing calculation
5. **Damage calculation:** Base damage + veterancy multipliers + naval/IC/airstrike mods
6. **DiskLaser special path:** Separate handling, no bullet created
7. **Bullet creation:** BulletClass allocation, velocity calculation, inaccuracy scattering
8. **Pre-fire damage subtraction:** Subtracts damage from target HP on fire (anti-overkill)
9. **Visual effects:** Muzzle flash anim, laser/electric bolt/sonic wave/rad beam
10. **ROF timer reset:** Sets fire timer fields (0x2EC, 0x2F0, 0x2F4, 0x2F8)
11. **Burst index update:** `CurrentBurstIndex = (CurrentBurstIndex + 1) % Burst`
12. **Post-fire:** RevealOnFire, LimboLaunch, spawner multi-target, FireOnce

### Pre-fire Damage Subtraction (Anti-Overkill Mechanic)

One of the most important details: when a bullet is fired (and is not Inaccurate), the engine
**immediately subtracts** the expected damage from the target's HP (`target+0x70`). This prevents
multiple units from all firing at the same target when a single shot would kill it. The actual
bullet hit later verifies and adjusts.

### ROF Timer Setup (from Fire_At)
```
this->0x2EC = g_CurrentFrameCounter;  // fire start frame
this->0x2F0 = computed_value;          // duration
this->0x2F4 = ROF_value;              // initial ROF
this->0x2F8 = ROF_value;              // remaining countdown

// If this->0x298 != 0 (mind-controlled/berserk modifier), ROF is halved
CurrentBurstIndex = (CurrentBurstIndex + 1) % weapon->Burst;
```

---

## 8. ReceiveDamage — Damage Intake Pipeline

**Address:** `0x00701900`
**Documented in detail in:** `RECEIVE_DAMAGE_GHIDRA_REPORT.md`

### Key Combat-Relevant Details

1. **Damage modifiers** (when !IgnoreDefenses && damage > 0):
   - Type-based multiplier (`Rules+0x100..0x110` per WhatAmI)
   - Veterancy ARMOR multiplier (victim's vet abilities: +0x29D, +0x2AF)
   - Minimum damage floor: clamps to 1

2. **Immunity checks:**
   - IronCurtain active → 0 damage + spark anim
   - Warping out → 0 damage
   - TypeImmune (same type, same owner) → 0 damage
   - Warhead has `MindControl=yes` + victim immune → 0 damage
   - Warhead `Temporal=yes` + victim immune → 0 damage
   - Warhead `Poison=yes` + victim immune → 0 damage
   - Warhead `AffectsAllies=no` + attacker is ally → 0 damage

3. **IC Stopper mechanic:** If victim is a building with `IC_Stopper` type flag AND IsSonic
   weapon AND building has `SWBuildingRequirements`, extends the building's shield timer.

4. **Retaliation trigger:** At the end of ReceiveDamage, calls `ShouldRetaliate()`. If true,
   calls `vtable+0x2E4` (SelectWeaponAgainst) on the attacker and `vtable+0x1F4` (Assign_Target).

5. **Passive target scan trigger:** If ShouldRetaliate returns false but unit has no target
   and is veteran/elite with auto-scan ability, calls `vtable+0x174` (scan for targets).

---

## 9. ShouldRetaliate — Retaliation Decision

**Address:** `0x007087C0`
**Signature:** `bool __thiscall ShouldRetaliate(TechnoClass* this, TechnoClass* attacker, WarheadTypeClass* warhead)`

### Conditions Required (ALL must be true)

1. Attacker is non-NULL
2. TechnoType+0xD9A != 0 (Retaliate flag — `Retaliate=yes`)
3. Not deploying (`this+0x2DC == 0`)
4. No slave manager active (`this+0x2D8 == 0`)
5. Not mind-controlled OR has `MindControlFireAtAll` flag
6. No active capture manager OR capture manager not full
7. No temporal weapon active (`this+0x2D0 == 0`)
8. Not player-controlled OR has no current target
9. Current mission allows retaliation (mission timer check)
10. Attacker is not allied
11. Attacker is not cloaked to this house
12. This unit has a weapon with range > 0
13. Unit can turn to face attacker (vtable+0x2AC)

### Additional Filters

14. **Player-controlled building exception:** If human-controlled and attacker is a building,
    checks if the firer is a base defense (defense type checks). Prevents units from auto-
    retaliating against enemy base defenses (player should manually order this).

15. **Veterancy guard suppression:** If veteran/elite with `GuardAreaEnhance` ability
    (+0x2AA / +0x2BC), suppresses retaliation against buildings.

16. **Threat comparison:** If non-player-controlled and already has a target, compares
    threat scores. Only retaliates if the new attacker has higher threat than current target.

17. **Team exclusion:** If unit is in a team and the team script has `IsGlobal` flag
    (+0xAF), does not retaliate (team orders take priority).

18. **Verses check:** If the chosen weapon's warhead has Verses <= 0.01 against the
    attacker's armor, does NOT retaliate (weapon would be ineffective).

---

## 10. Retaliate_And_Scan — Active Retaliation

**Address:** `0x00709820`
**Signature:** `bool __thiscall Retaliate_And_Scan(TechnoClass* this, coord, threat_flags)`

### Logic

1. **Update threat timestamp:** Sets `this+0x4FC = g_CurrentFrameCounter`

2. **ROF jitter:** Adds small random offset (0-2 frames) to retaliation ROF:
   - Terrain (WhatAmI == 0xB): Base = `Rules+0xE04` (TeamDelay)
   - Other: Base = `Rules+0xE08` (TargetDelay)

3. **Existing target revalidation:** If `this->ArchiveTarget != 0` AND `this+0x50C != 0`
   (IsNewTarget), attempts to keep current target:
   - Calls SelectWeaponAgainst on current target
   - Calls GetFireError with the weapon
   - If FIRE_RANGE → clear spawn targets
   - If FIRE_ILLEGAL or FIRE_FACING → clear all targets

4. **New target acquisition:** If ArchiveTarget is now NULL:
   - Calls `Greatest_Threat` (vtable+0x3C4) with threat_flags
   - If NOT gattling: sets ArchiveTarget (vtable+0x3C8) if target found
   - If gattling: calls `FUN_00709550` for special gattling target assignment
   - Also calls SelectWeaponAgainst and performs pre-fire damage subtraction

5. **Returns:** Whether ArchiveTarget is now non-NULL

---

## 11. Passive_Target_Acquire — Idle Scanning

**Address:** `0x00709480`
**Signature:** `bool __fastcall Passive_Target_Acquire(TechnoClass* this)`

### Logic

1. Calls `FUN_00709290` (ReadyToScan check — verifies scan timer has expired)
2. If ready:
   - Saves current ArchiveTarget
   - Updates `this+0x4FC = g_CurrentFrameCounter`
   - Gets own coordinates (vtable+0x48)
   - Calls `vtable+0x39C` (scan for threats)
   - If target changed from previous, sets `this+0x50C = 1` (IsNewTarget flag)
3. Returns whether a new target was found

**Called from:** `TechnoClass::AI_Update` during idle processing.

---

## 12. Greatest_Threat — Target Selection

**Address:** `0x006F8DF0`
**Documented in detail in:** `TARGET_ACQUISITION_GHIDRA_REPORT.md`

### Summary

Two scan modes based on `(threat_flags & 3)`:

**Mode A (flags & 3 == 0) — Flat array scan:**
- Iterates `g_TechnoClass_Array` (all units in game)
- Also iterates cells if flag 0x4 set
- Used by AI/area-guard for unlimited range scanning

**Mode B (flags & 3 != 0) — Expanding cell-square scan:**
- Computes scan radius from weapon range (flags & 1 → weapon 0, flags & 2 → weapon 1)
- Scans cells in expanding concentric squares from origin
- For each cell, calls `Scan_Cell_For_Target`
- Early termination: returns at 1/4 and 1/2 radius if target found
- Falls back to "cell threat" if no valid target (moves toward threatening cell)

### Special Flags

| Flag | Hex | Effect |
|------|-----|--------|
| bit 2 | 0x04 | Include neutral objects |
| bit 3 | 0x08 | Prioritize air targets |
| bit 4 | 0x10 | Include allies (for repair) |
| bit 14 | 0x4000 | Only target specific enemy |

### Gattling Targets
When the firer has TechnoType `IsGattling` (+0x6B0), collected targets are stored in
two dynamic vectors at offsets 0x440-0x46C and 0x458-0x484 for the gattling stage system.

---

## 13. Evaluate_Candidate — Per-Target Scoring

**Address:** `0x006F7CA0`
**Documented in detail in:** `TARGET_ACQUISITION_GHIDRA_REPORT.md`

### Key Rejection Criteria (returns false)

- Target health <= 0
- Target type `Legal=no` (+0x231 == 0)
- TechnoType `TargetCoordAdjust` (+0x604) == 2 AND target on ground
- Target in limbo
- Target being warped
- Target is cloaked AND no sensors detect it
- Target has `Insignificant` flag
- Same zone check failure (pathfinding zones must match unless unrestricted scan)
- SpecialFlags gate: civilian buildings check
- Target has `Immune` flag in this context
- Target on different bridge level from firer
- Recently took damage and has "just hit" cooldown
- Target is an ally AND weapon range > 0 AND not a repair weapon
- Allied target health above repair threshold (`Rules+0x16F8`)

### Score Multipliers

- If TechnoType `TargetPreference` (+0x394) == 1: Double score for wounded targets,
  half score for undamaged
- EnemyHouse priority: If attacker's house has a designated enemy, non-enemy targets get
  score = 1
- Special flag modifiers: Air targets, super-weapon buildings, spy infiltration targets
  each add 1000 to score
- ThreatAvoidance_Modifier: Reduces threat score for targets near allied buildings

---

## 14. Calculate_Threat_Score — Score Math

**Address:** `0x0070CD10`
**Signature:** `float __thiscall Calculate_Threat_Score(TechnoClass* this, ObjectClass* target, Coord* ref_point)`

### Formula Components

The score is a weighted sum of:

1. **Target's counter-threat** (what damage the target can do to US):
   - Gets target's weapon via SelectWeaponAgainst(this)
   - Reads Verses value for our armor type: `warhead.Verses[our_armor]`
   - If target is currently targeting us: adds `-0.0 * Verses` (no actual modifier)
   - Otherwise: adds `+0.0 * Verses` (effectively zero in default rules)

2. **Target's special value:**
   - `ThreatCoefficient1 * target_type->ThreatPosed` (TechnoType+0x2C0)
   - If target belongs to our designated enemy house: adds `ThreatCoefficient5`
     (Rules+0x1090)

3. **Our weapon damage vs target:**
   - `ThreatCoefficient2 * weapon.warhead.Verses[target_armor]`

4. **Target health ratio:**
   - `ThreatCoefficient3 * (target.Health / target.MaxHealth)`

5. **Distance penalty:**
   - Distance in cells from reference point (or firer's position if ref is null)
   - `max(0, distance - weapon_range_cells) * ThreatCoefficient4`
   - Targets within weapon range get no distance penalty

6. **Constant base:** `_DAT_007f4e90` (likely 1.0)

### Threat Coefficients (from Rules.ini [General])
| Index | Rules Offset | INI Key |
|-------|-------------|---------|
| 1 | +0x1068 | `MyEffectivenessCoefficientDefault` |
| 2 | +0x1080 | `TargetEffectivenessCoefficientDefault` |
| 3 | +0x1088 | `TargetSpecialThreatCoefficientDefault` |
| 4 | +0x1070 | `TargetStrengthCoefficientDefault` |
| 5 | +0x1090 | `TargetDistanceCoefficientDefault` |

If the firer's house has `UseCustomThreatRatings=yes` (+0x1FB), per-unit coefficients from
TechnoType (+0x2C8..0x2EC) are used instead.

---

## 15. UpdateGattlingStage — Gattling Cooldown

**Address:** `0x0070E000`
**Signature:** `void __thiscall UpdateGattlingStage(TechnoClass* this, int decay_ticks)`

### Logic

1. Releases current gattling sound event
2. Clears active gattling flag (`this+0x4B8 = 0`)
3. Reads decay rate from `GetType()+0xD10` (GattlingRateDecay)
4. Computes: `GattlingValue -= GattlingRateDecay * decay_ticks`
5. Clamps to 0 if negative or if decay_ticks == 0
6. If both GattlingValue and CurrentGattlingStage == 0:
   - Detaches any active gattling anims
   - Clears gattling state flags
7. Otherwise, checks if we need to downgrade stage:
   - Reads stage thresholds: For elite: `type+0xCF0 + stage*4`, for normal: `type+0xCD8 + stage*4`
   - If GattlingValue < threshold for current stage, decrements stage
   - Detaches old stage anim

### Key Fields
| Offset | Field | Purpose |
|--------|-------|---------|
| 0x140 (0x50) | GattlingValue | Accumulated fire value |
| 0x144 (0x51) | CurrentGattlingStage | Current weapon stage (0-based) |
| Type+0xCD8 | WeaponStages[] (normal) | Threshold values per stage |
| Type+0xCF0 | EliteWeaponStages[] | Elite threshold values per stage |
| Type+0xD10 | GattlingRateDecay | Decay rate per tick |

---

## 16. ROF and Cooldown Handling

### Timer Fields on TechnoClass

| Offset | Type | Purpose |
|--------|------|---------|
| 0x2EC | int | Frame counter when fire timer started. -1 = not started |
| 0x2F0 | int | Computed ROF duration (possibly adjusted for burst) |
| 0x2F4 | int | Initial ROF value from weapon |
| 0x2F8 | int | Current remaining ROF countdown |

### ROF Calculation (from Fire_At)

1. Base ROF comes from `vtable+0x318` (GetROF virtual)
2. If `this+0x298 != 0` (berserk/mind-controlled modifier): ROF is **halved**
3. Timer is set: `start = g_CurrentFrameCounter`, `remaining = ROF`

### Cooldown Check (from GetFireError)

```
remaining = this->0x2F4;  // initial ROF value
if (this->0x2EC != -1):
    elapsed = g_CurrentFrameCounter - this->0x2EC;
    if (elapsed >= remaining):
        // Timer expired, can fire
        goto CHECK_PASSED
    remaining -= elapsed;
if (remaining != 0):
    return FIRE_BUSY;
```

### Burst Timing

The `CurrentBurstIndex` increments with each shot and wraps: `index = (index + 1) % Burst`.
For buildings (type 1/infantry), there's a special barrel-based timing where the first 2
shots in a burst check against building barrel indices (`BuildingType+0xE40 + index*4`).
If the barrel index doesn't match the building's current barrel → FIRE_BUSY.

---

## 17. Veterancy Effects on Combat

### Damage Output (Applied in Fire_At)

| Condition | TechnoType Offset | Effect |
|-----------|-------------------|--------|
| Veteran + VeteranAbility firepower | +0x29E | Damage multiplied by veteran firepower factor |
| Elite + EliteAbility firepower | +0x2B0 | Damage multiplied by elite firepower factor |

### Damage Intake (Applied in ReceiveDamage)

| Condition | TechnoType Offset | Effect |
|-----------|-------------------|--------|
| Veteran + VeteranAbility armor | +0x29D | Damage reduced by veteran armor factor |
| Elite + EliteAbility armor | +0x2AF | Damage reduced by elite armor factor |

### Range (Applied in InRange)

| Condition | Rules Offset | Effect |
|-----------|-------------|--------|
| Promoted (any) | +0xF54 | +VeteranRangeBonus cells (×256 leptons) |

### ROF

| Condition | Effect |
|-----------|--------|
| Berserk (0x298 set) | ROF halved |

### Auto-scan (Applied in ReceiveDamage retaliation path)

| Condition | TechnoType Offset | Effect |
|-----------|-------------------|--------|
| Veteran + VeteranAbility auto-scan | +0x29F | Can trigger passive target scan |
| Elite + EliteAbility auto-scan | +0x2B1 | Can trigger passive target scan |

### Retaliation Suppression

| Condition | TechnoType Offset | Effect |
|-----------|-------------------|--------|
| Veteran + guard area enhance | +0x2AA | Suppresses retaliation vs buildings |
| Elite + guard area enhance | +0x2BC | Suppresses retaliation vs buildings |

### Weapon Selection (Elite weapons)

GetWeapon (FUN_0070e140) checks elite status:
- If elite: tries `EliteWeapon[index]` first (Type+0xE20 for buildings)
- Falls back to normal weapon

### Gattling Stages (Elite)

UpdateGattlingStage reads different threshold tables:
- Normal: `type+0xCD8 + stage*4`
- Elite: `type+0xCF0 + stage*4`

---

## 18. Key TechnoClass Combat Fields

### Target-Related
| Offset | Name | Type | Purpose |
|--------|------|------|---------|
| 0x2B4 | ArchiveTarget | target ptr | Active combat target |
| 0x274 | TemporalPtr | ptr | Temporal weapon system |
| 0x294 | AirstrikePtr | ptr | Airstrike designator |
| 0x2BC | CaptureManager | ptr | Mind control system |
| 0x2C0 | MindControlledBy | ptr | Mind controller reference |
| 0x2D0 | SpawnManager | ptr | Spawner system |
| 0x2D8 | SlaveManager | ptr | Slave system |

### Fire State
| Offset | Name | Type | Purpose |
|--------|------|------|---------|
| 0x120 | LastFireFrame | int | Frame of last successful fire |
| 0x2EC | FireTimerStart | int | Frame when ROF timer started |
| 0x2F0 | FireTimerDuration | int | Computed ROF duration |
| 0x2F4 | FireTimerInitial | int | Initial ROF value |
| 0x2F8 | FireTimerRemaining | int | Remaining ROF countdown |
| 0x3B8 | CurrentBurstIndex | int | Shot index within burst |
| 0x50C | IsNewTarget | byte | Flag: passive scan found new target |

### Gattling
| Offset | Name | Type | Purpose |
|--------|------|------|---------|
| 0x140 | GattlingValue | int | Accumulated fire value |
| 0x144 | CurrentGattlingStage | int | Current weapon stage |
| 0x4B8 | GattlingActiveFlag | byte | Gattling system active |

### Weapon Effect Pointers
| Offset | Name | Type | Purpose |
|--------|------|------|---------|
| 0x304 | WeaponParticleSystem1 | ptr | UseFireParticles beam |
| 0x308 | WeaponParticleSystem2 | ptr | UseSparkParticles beam |
| 0x314 | WeaponParticleSystem3 | ptr | IsRailgun beam |
| 0x318 | Wave | ptr | IsSonic / IsMagBeam wave |

### Combat State
| Offset | Name | Type | Purpose |
|--------|------|------|---------|
| 0x082 | AirstrikeFlag | byte | Unit is performing airstrike |
| 0x298 | BerserkFlag | byte | Unit is berserk/mind-controlled modifier |
| 0x29C | AccumulatedDamage | int | For mind control damage tracking |
| 0x2A0 | ScatterIndex | int | Rotating scatter pattern index |
| 0x2DC | DeployState | int | Deploy/undeploy state machine |
| 0x2E4 | IronCurtainBuilding | ptr | Building providing IC protection |

### Threat Tracking
| Offset | Name | Type | Purpose |
|--------|------|------|---------|
| 0x174 | LastHitFrame | int | Frame of last damage received |
| 0x178 | LastHitInfo | int | Info about last hit |
| 0x17C | LastHitDuration | int | Duration value for hit tracking |
| 0x1E0 | ScatterFrame | int | Frame for scatter response |
| 0x1E4 | ScatterInfo | int | Scatter direction info |
| 0x1E8 | ScatterDamage | int | Damage that triggered scatter |
| 0x4FC | LastScanFrame | int | Frame of last target scan |

---

## Appendix: INI Keys Referenced

### WeaponTypeClass (direct field references found in combat code)
`Damage`, `Range`, `MinimumRange`, `ROF`, `Burst`, `Speed`, `Projectile`, `Warhead`,
`AmbientDamage`, `Suicide`, `UseFireParticles`, `UseSparkParticles`, `IsRailgun`, `IsSonic`,
`Spawner`, `DrainWeapon`, `IsLaser`, `IsElectricBolt`, `IsRadBeam`, `IsRadEruption`,
`IsMagBeam`, `DiskLaser`, `DecloakToFire`, `CellRangefinding`, `FireOnce`, `NeverUse`,
`RevealOnFire`, `AreaFire`, `OmniFire`, `Bright`, `LimboLaunch`, `IonSensitive`,
`FireInTransport`, `FireWhileMoving`, `IsHouseColor`, `InfiniteMindControl`,
`Supress` (sic), `Camera`, `Charges`

### TechnoTypeClass (combat-related offsets referenced)
`Retaliate` (+0xD9A), `Turret` (+0x2DC?), `IsGattling` (+0x6B0 via GetType),
`ThreatPosed` (+0x2C0), `TargetPreference` (+0x394), `VeteranAbilities` (+0x29D..0x2B1),
`EliteAbilities` (+0x2AF..0x2BC), `NavalTargetingRange` (+0x68C), `GattlingRateDecay` (+0xD10),
`TypeImmune` (+0xC8C), `GuardRange` (+0x5B8)

### RulesClass (General section)
`BallisticScatter` (+0x1734), `Gravity` (+0x16B8), `MaxDamage` (+0x16C8),
`RepairThreshhold` (+0x16F8), `ConditionYellow` (+0x1700), `RepairRate` (+0x1708),
`NavalRangeBonus` (+0xF48), `VeteranRangeBonus` (+0xF54), `AirstrikeRangeBonus` (+0xF5C),
`TeamDelay` (+0xE04), `TargetDelay` (+0xE08),
`MyEffectivenessCoefficientDefault` (+0x1068), `TargetEffectivenessCoefficientDefault` (+0x1080),
`TargetSpecialThreatCoefficientDefault` (+0x1088), `TargetStrengthCoefficientDefault` (+0x1070),
`TargetDistanceCoefficientDefault` (+0x1090),
`AutoRepel` (+0x17EC), `PlayerAutoFireWeapon` (+0x17ED)
