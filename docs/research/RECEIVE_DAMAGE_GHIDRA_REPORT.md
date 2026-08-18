# TechnoClass::ReceiveDamage — Ghidra Research Report

Address: `0x00701900` (gamemd.exe)
Size: ~5154 bytes, 682 lines decompiled, cyclomatic complexity 250
Confidence: HIGH (verified from binary decompilation)

---

## 1. Parameters

`ReceiveDamage` is a `__thiscall` virtual method. The `this` pointer is in ECX (TechnoClass*).
Stack parameters (from Ghidra's `in_stack_*` variables):

| Stack offset | Name | Type | Description |
|---|---|---|---|
| +0x04 | Damage | `int*` | Pointer to damage amount (read/write — may be modified) |
| +0x08 | DistanceFromEpicenter | `int` | Lepton distance from blast center |
| +0x0C | Warhead | `WarheadTypeClass*` | Warhead rules pointer |
| +0x10 | Attacker | `TechnoClass*` | Source of damage (nullable) |
| +0x14 | IgnoreDefenses | `bool` | If true, skips armor/shield/IronCurtain checks |
| +0x18 | (unused in this func) | | |
| +0x1C | AttackerHouse | `HouseClass*` | House that owns the attacker |

**Return value**: `DamageState` enum (uint):
- 0 = Unaffected
- 1 = ConditionYellow (first transition to half-health)
- 2 = ConditionYellow
- 3 = ConditionRed
- 4 = NowDead
- 5 = PostMortem (already dead / trigger killed)

---

## 2. Flow Overview (Step by Step)

### Phase 1: Damage Modification (~0x701900–0x701A40)

1. **Negative damage check**: `bVar13 = (*Damage < 0)` — stored for later (negative = healing).

2. **Type-based damage multiplier** (only if `!IgnoreDefenses && !bVar13`):
   - Calls `FUN_0050bd30(this->GetType())` — returns a float multiplier based on the
     victim's AbstractType (switch on `WhatAmI()`):
     - Type 3 (Unit): reads `Rules+0x108` (probably `UnitDamageMult`)
     - Type 0x10 (Aircraft): reads `Rules+0x100`
     - Type 0x28 (Building): reads `Rules+0x104`
     - Type 7 (Infantry): checks locomotor mode 5 → `Rules+0x110`, else `Rules+0x10c`
   - Multiplies `*Damage` by this float, converts via `ftol`.

3. **Veterancy damage bonus** (only if `!IgnoreDefenses && !bVar13`):
   - Calls `FUN_0074ff90()` (IsVeteran — checks experience float in range
     `[_DAT_007e2ac8, _DAT_007e37b4)`) or `FUN_00750010()` (IsElite — experience >=
     `_DAT_007e37b4`)
   - If veteran AND `TechnoTypeClass+0x29d != 0` (VeteranAbility: YOURFIRE_POW), OR
     if elite AND (`TechnoTypeClass+0x29d != 0` OR `TechnoTypeClass+0x2af != 0`
     (EliteAbility: YOURFIRE_POW)) — applies additional damage scaling via ftol.
   - **Note**: These are ATTACKER veterancy bonuses applied to incoming damage,
     not defensive bonuses. The attacker's vet status was already used before calling
     ReceiveDamage. These flags check if the VICTIM type has these vet abilities —
     likely for self-damage or special interactions.

4. **Minimum damage floor**: If `*Damage < 1` after modifiers, clamps to 1.

5. **TypeImmune check** (`TechnoTypeClass+0xc8c`): If the victim's type has
   `TypeImmune=yes` and the attacker's type matches, AND they share the same Owner,
   damage is zeroed and returns 0 (e.g., Tanya immune to own Tanya's attacks).

### Phase 2: Invulnerability Checks (~0x701A40–0x701AD0)

6. **IronCurtain check** (vtable+0x160, `TechnoClass::IsIronCurtained` at 0x0041bf40):
   - Checks `this+0x18c` (IC start frame) and `this+0x194` (IC duration).
   - If the IronCurtain timer is still active AND `!IgnoreDefenses && !bVar13`:
     - Plays spark animation (`FUN_0048a620`) — uses anim type 6 if `this+0x1c4 == 1`
       (ForceShield), else type 1 (normal IC spark).
     - Sets `*Damage = 0`, returns 0.

7. **Warping-out check** (vtable+0x1d4, `TechnoClass::IsWarpingOut` at 0x0070c5b0):
   - Reads `this+0x270`. If the unit is currently being chronoshifted/warped out AND
     `!IgnoreDefenses`, sets `*Damage = 0`, returns 0.

### Phase 3: Special Ammo/Shield Handling (~0x701AD0–0x701BC0)

8. **Ammo-based damage absorption** (`TechnoTypeClass+0x6b1`):
   - If type has ammo-absorption flag (likely `DamageSparks` or similar at +0x6b1):
     - Calculates absorbed damage as a fraction: `(float)(*Damage) / (float)Strength *
       TechnoTypeClass+0x6b4`
     - Deducts from `this->Ammo` (TechnoClass field)
     - Calls `FUN_006fb080()` to trigger ammo depletion animation

### Phase 4: ForceShield / Bunker Logic (~0x701BC0–0x701C00)

9. **ForceShield timer** (`this+0x2e4`):
   - If ForceShield is active (field_0x2e4 != 0) AND `!IgnoreDefenses`:
     - For buildings (`WhatAmI() == 6`): If warhead has `CausesDelayKill` (WH+0x130),
       skips to warhead checks. Otherwise tests if warhead has `PenetratesBunker`
       (WH+0x146) — if not, checks if the attacker is in the same building cell; if so,
       damage = 0.
     - For non-buildings: If warhead does NOT have `PenetratesBunker`, removes
       ForceShield and checks cell.

### Phase 5: Warhead Special Effect Checks (~0x701BF6–0x701D70)

10. **Warhead immunity checks** (only if Warhead != null):
    - `WH+0x177` = `Radiation`: If target has `TechnoTypeClass+0xd37`
      (`ImmuneToRadiation`), damage = 0, return 0.
    - `WH+0x178` = `PsychicDamage`: If target has `TechnoTypeClass+0xd36`
      (`ImmuneToPsionicWeapons`), damage = 0, return 0.
    - `WH+0x156` = `Poison`: If target has `TechnoTypeClass+0xd3b`
      (`ImmuneToPoison`), damage = 0, return 0.
    - `WH+0x179` = `AffectsAllies`: If false AND attacker is allied with victim,
      damage = 0, return 0.

11. **Psychedelic / Mind Control** (`WH+0x16d` = `Psychedelic`):
    - If attacker is allied with victim, returns 0 (no friendly-fire mind control).
    - If `TechnoTypeClass+0xd35` (`ImmuneToPsionics`), returns 0.
    - If victim is a building (`WhatAmI() == 6`), returns 0 (can't MC buildings).
    - Applies mind control damage: reads weapon armor interaction, stores damage
      at `this+0x29c`, sets `this+0x298` (MCed flag) to 1.
    - Ejects passengers if any (`FUN_006ea870`).
    - Calls vtable+0x3c8 (set mission idle) and vtable+0x1e8.
    - Returns 1.

### Phase 6: ObjectClass::ReceiveDamage (~0x701D70)

12. **Core damage application**: Calls `ObjectClass::ReceiveDamage` (0x005f8c90) which:
    - **Early exit**: If `Health < 1` or `*Damage == 0`, returns 0.
    - If `!IgnoreDefenses` and `TechnoTypeClass+0x233` (Insignificant flag) is set, returns 0.
    - **Armor/Verses calculation**: Calls `FUN_00489180` with:
      - param_1 = `*Damage` (current damage value)
      - param_2 = Warhead pointer
      - param_3 = Warhead (for armor lookup)
      - param_4 = armor index from `TechnoTypeClass+0x9c`
    - For negative damage (healing): If ArmorType > 7, returns 0 (special armors
      can't be healed). Otherwise heals up to MaxHealth.
    - For positive damage: Clamps to current HP if exceeds it.
    - **Health decrement**: `this->Health = Health - *Damage` (offset 0x6C).
    - **Building special**: If `WhatAmI() == 0xf` (Building), health reduced below 1,
      and building has a `ConditionRed` gate flag — plays ConditionRed anim, clamps HP
      to 1, forces mission to 6 (Idle), sets condition state = 3.
    - **Damage state transitions**:
      - State 1: damage reduced HP (no threshold crossed yet)
      - State 2 (ConditionYellow): HP crossed from >= Strength/2 to < Strength/2
      - State 3 (ConditionRed): HP crossed below `Strength * Rules+0x1708`
        (ConditionRed ratio, from `[General]` `ConditionRed=`)
    - **Trigger events**: Fires map trigger events 0x26-0x2C for damage states
      (first damaged, half-health, quarter-health, etc.)
    - **Death**: If `Health == 0`:
      - Calls vtable+0xE0 (RegisterDestruction with attacker)
      - OR vtable+0xE4 (RegisterDestruction with attacker's house)
      - Calls vtable+0xDC (MarkForDeath, param=1)
      - Returns state 4 (NowDead)

### Phase 7: Post-ObjectClass Processing (~0x701DA0–0x701FA6)

13. **Attacker score tracking**: If attacker exists, calls into the attacker's type
    scoring system (`TechnoTypeClass->vfunc at +0xac`) with the damage dealt.

14. **Death result (state == 4)**: Special handling:
    - For buildings with `CausesDelayKill` warhead (WH+0x130) — checks if the building
      type at `BuildingType+0x520` has `SelfHealing` flag (+0x1551). If yes, implements
      the delay-kill timer system:
      - Sets `this+0x6df` = 1 (delayKill active flag)
      - Records start frame at `this+0x528`
      - Records delay duration at `this+0x530`
      - Forces Health=1, IsAlive=true
      - Returns 5 (PostMortem — kept alive)

15. **Retaliation tracking** (state != 0 and != 4):
    - Sets `this+0x174` = currentFrame, `this+0x178` = distance, `this+0x17c` =
      `Rules+0x8c` (retaliation delay)
    - If `TechnoTypeClass+0xd2f` is set AND +0xd30 is NOT set (Trainable +
      DamageReducesReadiness check), AND unit is human-controlled (vtable+0xc4):
      - Calls vtable+0x470 (scatter/flee)
      - Records `this+0x1e0/0x1e4/0x1e8` for flee behavior timing

### Phase 8: Death Processing — Cleanup and Debris (~0x702040–0x702700)

16. **Death case (state == 4, Health == 0)**:
    - **Mind control cleanup**: Breaks MC links at `this+0x2d8`, `this+0x1cc`,
      `this+0x1d0`, `this+0x1d4` — disconnects from MC controller.
    - **CaptureManager cleanup**: Releases captured units.
    - **EVA announcements**: Plays `DeathVoice` if attacker's type has DeathVoice
      (TechnoTypeClass+0x4cc and +0x520).
    - **Fire mission release**: vtable+0x280(3), vtable+0x3a0.
    - **Temporal link cleanup** (`this+0x304`): Removes temporal warp.
    - **Debris spawning** (`TechnoTypeClass+0x5bc`): Uses `DebrisTypes`, `DebrisMaximums`,
      `DebrisAnims` to spawn VoxelAnimClass and AnimClass debris objects at the
      death location.
    - **Survivor ejection** (`TechnoTypeClass+0xd15 = Explodes`): If type has Explodes=yes
      AND (is veteran/elite with appropriate ability OR weapon has `Suicide=yes` at
      WeaponType+0x144), ejects passengers and survivors. Calls `FUN_006ea870` for
      passenger release.
    - **Crew spawning**: Calls `FUN_0070d690(0)` for infantry survivors.

### Phase 9: Condition Animations (~0x702717–0x702900)

17. **Damage state anims** (state 1 or 2/3):
    - Plays damage state notification anim (TechnoTypeClass+0x538, DamageParticleSystems).
    - If `TechnoTypeClass+0xc96` (Repairable?) or `this+0x3cf` is set, AND NOT an AI
      player, AND attacker exists: calls `FUN_00708080` — the **threat assessment/base
      defense response** function (300 lines, triggers AI defense reactions).

### Phase 10: Health Ratio Effects (~0x702900–0x702B00)

18. **Damage smoke particles** (when `GetHealthRatio() <= Rules+0x1700` = ConditionYellow
    threshold):
    - If state is 2 (ConditionYellow) or 3 (ConditionRed):
      - Iterates `TechnoTypeClass+0x788` (DamageParticleSystems count) backwards.
      - For each particle system type, checks `ParticleSystemType+0x2b4 == 0`.
      - Creates a `ParticleSystemClass` at the victim's location plus a turret offset
        (`FUN_007178c0`), stored at `this+0x310`.
    - If health is ABOVE yellow threshold and `this+0x310` exists, destroys the
      existing damage particle system.

### Phase 11: Retaliation (~0x702B00–0x702D20)

19. **Auto-retaliation** via `FUN_007087c0` (0x007087c0):
    - Called with (this, attacker, warhead).
    - Returns 1 if retaliation should occur, 0 if not.
    - **Conditions for retaliation** (ALL must be true):
      - Attacker is not null
      - `TechnoTypeClass+0xd9a` (CanRetaliate) is true
      - Victim is not currently garrisoned (field_0x2dc == 0)
      - Victim is not mind-controlled (field_0x2d8 == 0, field_0x1cc == 0)
      - No CaptureManager active, OR capture not in progress
      - Victim not currently in a special state (field_0x2d0 == 0)
      - If AI: target must be null (not already attacking something)
      - Attacker is an enemy (not allied)
      - Victim has weapon range > 0
      - vtable+0x2ac returns true (CanFire check?)
      - If human player: compares distance to current target vs attacker — retaliates
        only against the closer threat
      - Checks weapon's Verses against attacker's armor — if Verses <= 0.01
        (effectively immune), does NOT retaliate (won't waste ammo)
    - If retaliation approved: calls vtable+0x2E4 (SetTarget to attacker),
      vtable+0x1F4 (Assign mission), checks range and enters combat.

20. **Scatter-on-damage** (if retaliation function returns 0):
    - Checks if unit is player-controlled (`this->field_0x14 & 4`)
    - Checks if unit is not passive (`FUN_005b3a00` session check)
    - Checks if `this->field_0x418` is not set (not frozen/halted)
    - If unit has no target and no queued mission:
      - If `WhatAmI() != 2` (not infantry — infantry doesn't auto-scatter)
      - Checks `Rules+0x17ed` (MultiplayPassive flag)
      - Veteran/elite with appropriate scatter ability → proceeds
      - Calls vtable+0x174 — **Scatter** away from attacker
    - **Range check for retaliation**: If attacker exists, computes 3D distance.
      If distance exceeds `TechnoTypeClass+0x5e8` (GuardRange) scaled by constants,
      skips retaliation and just scatters.

---

## 3. Armor Calculation / Verses

**Function**: `FUN_00489180` at 0x00489180
Called from `ObjectClass::ReceiveDamage` with:
- param_1 = damage amount (uint)
- param_2 = warhead pointer
- param_3 = (unused here, passed through)
- param_4 = armor type index (from `TechnoTypeClass+0x9c`)

**Logic**:
```
if (damage == 0 || warhead == 0 || DAT_00a8b230 bit5 set):
    return 0

if (damage < 0):  // healing
    return (armorType > 7) ? 0 : damage   // special armors can't be healed

// Positive damage:
verses_multiplier = *(float*)(warhead + 300)  // WH+0x12C... but see below
result = ftol(damage * verses_multiplier)

// Clamp to maximum from Rules
if (result >= Rules+0x16c8):   // MaxDamage cap
    return Rules+0x16c8

return result
```

**Verses array**: Located at WarheadTypeClass+0xA0, parsed in `WarheadTypeClass__ReadINI`
from the `Verses=` key. This is an array of 11 doubles (0xA0 to 0xF8), one per armor
type. The function reads `*(float*)(warhead + 300)` which corresponds to an indexed
access into this array based on the victim's armor index.

**Armor type index**: Stored at `TechnoTypeClass+0x9c` (the `Armor=` INI key value,
parsed as an enum index 0-10: none, flak, plate, light, medium, heavy, wood, steel,
concrete, special_1, special_2).

---

## 4. Veterancy Defense Bonus

There is **no explicit veteran/elite DEFENSIVE damage reduction** in
`TechnoClass::ReceiveDamage` itself. The veterancy checks at the start of the function
(`FUN_0074ff90` / `FUN_00750010`) modify the incoming damage based on VeteranAbilities
and EliteAbilities flags at the TYPE level.

The relevant TechnoTypeClass veteran ability offsets:
- +0x29d = VeteranAbility: YOURFIRE_POW (damage modifier when veteran)
- +0x2af = EliteAbility: YOURFIRE_POW
- +0x29f = VeteranAbility: related to scatter/retaliation
- +0x2a6 = VeteranAbility: related to retaliation eligibility
- +0x2b1 = EliteAbility: scatter on damage
- +0x2b8 = EliteAbility: retaliation eligibility
- +0x2aa = VeteranAbility: used in retaliation range check

However, the `FUN_0050bd30` multiplier at the very start applies a **global type-based
damage multiplier** from Rules.ini (e.g., `[General]` `UnitDamageMultiplier=`,
`AircraftDamageMultiplier=`, etc.) which is NOT veterancy per se but a global balance
knob.

**Veterancy armor bonus** (YOURARM): Handled OUTSIDE ReceiveDamage — the Verses
multiplier or armor override is applied at the caller level or in the armor lookup.

---

## 5. Shield / IronCurtain / ForceShield

### IronCurtain (vtable+0x160, function at 0x0041bf40)
- **Instance offsets**: `this+0x18c` (start frame, -1 = inactive), `this+0x194` (duration in frames)
- **Check**: `currentFrame - startFrame < duration` → still active
- **Effect**: Damage = 0, plays spark anim via `FUN_0048a620`:
  - If `this+0x1c4 == 1` (ForceShield variant): plays anim type 6
  - Else: plays anim type 1 (normal IC spark)
- **Returns 0** (Unaffected)

### Warping Out (vtable+0x1d4, function at 0x0070c5b0)
- Reads `this+0x270`. If nonzero (unit is being chronoshifted out), damage = 0.

### ForceShield (`this+0x2e4`)
- Separate from IronCurtain. If active, applies bunker/shield penetration logic.
- For buildings: checks `WH+0x146` (PenetratesBunker) and `WH+0x130` (CausesDelayKill).

---

## 6. Health Tracking

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| ObjectClass+0x6C | Health | int | Current hit points |
| TechnoTypeClass+0xA0 | Strength | int | Maximum HP (from INI `Strength=`) |

**Health decrement** in `ObjectClass::ReceiveDamage`:
```c
this->Health = oldHealth - *Damage;
```

**Health ratio**: `(double)Health / (double)Strength` — computed by
`ObjectClass::GetHealthRatio` (0x0041b8d0).

**Healing** (negative damage): `Health = min(Health - Damage, Strength)` — clamps to max.

---

## 7. Death Check

Death occurs when `Health` reaches 0 after damage application:

```c
if (this->Health == 0) {
    // Determine kill credit
    if (attackerHouse == 0 || (attacker != 0 && attackerHouse == attacker->OwnerHouse)) {
        vtable+0xE0(attacker);     // RegisterDestruction(attacker)
    } else {
        vtable+0xE4(attackerHouse); // RegisterDestruction(house)
    }
    vtable+0xDC(1);                 // MarkForDeath
    return 4;                       // NowDead
}
```

**Building special case**: Buildings with `SelfHealing` at `BuildingType+0x520+0x1551`
AND hit by `CausesDelayKill` warhead get a delayed death — HP is forced to 1,
`IsAlive = true`, a timer is started, and return 5 (PostMortem).

---

## 8. Damage State Transitions

Computed in `ObjectClass::ReceiveDamage` (0x005f8c90):

| State | Threshold | Meaning |
|-------|-----------|---------|
| 0 | No damage dealt | Unaffected |
| 1 | HP reduced but no threshold crossed | Minor damage |
| 2 (ConditionYellow) | HP crossed from >= `Strength/2` to < `Strength/2` | Half health |
| 3 (ConditionRed) | HP crossed below `Strength * Rules+0x1708` | Critical health |
| 4 (NowDead) | HP reached 0 | Destroyed |
| 5 (PostMortem) | Already dead or killed by trigger | Special state |

**Rules offsets**:
- `Rules+0x1700` = ConditionYellow threshold (double, typically 0.5 or 1.0)
- `Rules+0x1708` = ConditionRed threshold (double, typically 0.25)

**Trigger events fired** (in ObjectClass::ReceiveDamage):
- Event 0x26 = First damaged (HP drops from full)
- Event 0x27 = Half health reached (with attacker)
- Event 0x28 = Quarter health reached (with attacker)
- Event 0x29 = First damaged (any source)
- Event 0x2A = Half health (any source)
- Event 0x2B = Quarter health (any source)
- Event 0x2C = Attacked by (with attacker) — if not dead

---

## 9. Special Warhead Effects Dispatch

All special effects are checked BEFORE the normal damage pipeline
(`ObjectClass::ReceiveDamage`), with the exception of Psychedelic which short-circuits.

| WH Offset | INI Key | Effect in ReceiveDamage |
|-----------|---------|------------------------|
| +0x130 | CausesDelayKill | Building delay-kill logic (HP forced to 1, timer set) |
| +0x146 | PenetratesBunker | Bypasses ForceShield/bunker protection |
| +0x156 | Poison | Checked against ImmuneToPoison → zero damage |
| +0x16d | Psychedelic | **Mind Control path** — short-circuits, applies MC, returns 1 |
| +0x177 | Radiation | Checked against ImmuneToRadiation → zero damage |
| +0x178 | PsychicDamage | Checked against ImmuneToPsionicWeapons → zero damage |
| +0x179 | AffectsAllies | If false + allied target → zero damage |

**Mind Control dispatch** (WH+0x16d = Psychedelic):
1. Alliance check — no friendly MC
2. ImmuneToPsionics check (TechnoTypeClass+0xd35)
3. Building check (WhatAmI == 6 → immune)
4. Calculates MC "damage" from weapon armor interaction
5. Stores result at `this+0x29c`, sets `this+0x298 = 1`
6. Ejects passengers
7. Sets mission to idle (vtable+0x3c8)
8. Returns 1

**Note**: Temporal, EMP, and Radiation warhead EFFECTS are not directly dispatched
inside ReceiveDamage. Those warheads do zero damage (via the immunity checks above),
and their actual effects (temporal warp, EMP disable, radiation field) are applied
in `WarheadTypeClass::Detonate` (0x004690b0) or in the BulletClass impact code,
not here. ReceiveDamage only handles the DAMAGE aspect and immunity gating.

---

## 10. Retaliation

**Primary retaliation function**: `FUN_007087c0` at 0x007087c0

Called near the end of ReceiveDamage after all damage processing. Returns bool
(1 = will retaliate, 0 = won't).

**Key conditions** (all must be true):
1. Attacker exists (not null)
2. `TechnoTypeClass+0xd9a` = CanRetaliate flag is set
3. Not garrisoned (`this+0x2dc == 0`)
4. Not mind-controlled (`this+0x2d8 == 0`, `this+0x1cc == 0`)
5. No CaptureManager active or capture not in progress
6. Not in a special movement state (`this+0x2d0 == 0`)
7. If AI: not already targeting something
8. Attacker is an enemy
9. Victim has weapon range > 0
10. `vtable+0x2ac` CanFire check passes
11. **Verses check**: Weapon's Verses against attacker's armor must be > 0.01
    (won't retaliate if weapon can't hurt attacker)
12. **Distance check** (human player): If already has a target, only retaliates
    against closer threats

**If retaliation approved**:
- `vtable+0x2E4` — SetTarget(attacker)
- `vtable+0x1F4` — Assign combat mission
- Enters engagement

**If retaliation denied** (scatter path):
- Player-controlled units with no current target:
  - Check `Rules+0x17ed` (MultiplayPassive)
  - Check veteran/elite scatter abilities (TechnoTypeClass+0x29f, +0x2b1)
  - Call `vtable+0x174` — Scatter away from attacker

---

## 11. Key Struct Offsets

### TechnoClass Instance Offsets
| Offset | Type | Field |
|--------|------|-------|
| +0x6C | int | Health (inherited from ObjectClass) |
| +0x118 | ptr | Passenger list head |
| +0x174 | int | Last damage frame |
| +0x178 | int | Last damage distance |
| +0x17c | int | Retaliation delay (from Rules+0x8c) |
| +0x18c | int | IronCurtain start frame (-1 = inactive) |
| +0x194 | int | IronCurtain duration (frames) |
| +0x1c4 | int | ForceShield flag (1 = ForceShield variant) |
| +0x1cc | ptr | MindControl controller (TechnoClass*) |
| +0x1d0 | ptr | MindControl victim link |
| +0x1d4 | ptr | Temporal warp link |
| +0x1e0 | int | Flee start frame |
| +0x1e4 | int | Flee distance |
| +0x1e8 | int | Flee damage*2 |
| +0x270 | byte | IsWarpingOut flag |
| +0x298 | byte | IsMindControlled flag |
| +0x29c | int | MindControl damage value |
| +0x2d0 | int | Special movement state |
| +0x2d8 | int | MindControl link (outgoing) |
| +0x2dc | int | Garrison state |
| +0x2e4 | int | ForceShield timer |
| +0x304 | ptr | Temporal link (incoming) |
| +0x310 | ptr | DamageParticleSystem |
| +0x3cf | byte | Repairable / AI defense flag |
| +0x3d1 | byte | WasAttacked flag (set when damaged by enemy) |
| +0x418 | byte | Halted / frozen flag |
| +Ammo | int | Ammo count (exact offset depends on class) |

### TechnoTypeClass Offsets (via GetType, vtable+0x84)
| Offset | Type | INI Key |
|--------|------|---------|
| +0x9c | int | Armor (enum index) |
| +0xa0 | int | Strength (max HP) |
| +0x233 | bool | Insignificant |
| +0x29d | bool | VeteranAbility: YOURFIRE_POW |
| +0x29f | bool | VeteranAbility: scatter-on-damage |
| +0x2a6 | bool | VeteranAbility: retaliation |
| +0x2aa | bool | VeteranAbility: retaliation range |
| +0x2af | bool | EliteAbility: YOURFIRE_POW |
| +0x2b1 | bool | EliteAbility: scatter-on-damage |
| +0x2b8 | bool | EliteAbility: retaliation |
| +0x4cc | int | DeathVoice (EVA/sound index) |
| +0x520 | int | DeathWeapon (or building type ptr) |
| +0x538 | int | DamageParticleSystems anim index |
| +0x5bc | int | DebrisTypes count |
| +0x5c0 | int | DebrisAnims start index |
| +0x5e8 | int | GuardRange (leptons) |
| +0x6b1 | bool | DamageSparks / ammo absorption flag |
| +0x6b4 | float | Ammo absorption rate |
| +0xc8c | bool | TypeImmune |
| +0xc96 | bool | Repairable / needs repair flag |
| +0xd15 | bool | Explodes |
| +0xd2f | bool | Trainable |
| +0xd30 | bool | DamageReducesReadiness |
| +0xd35 | bool | ImmuneToPsionics |
| +0xd36 | bool | ImmuneToPsionicWeapons |
| +0xd37 | bool | ImmuneToRadiation |
| +0xd3b | bool | ImmuneToPoison |
| +0xd9a | bool | CanRetaliate |

### WarheadTypeClass Offsets
| Offset | Type | INI Key |
|--------|------|---------|
| +0x98 | double | Deform |
| +0xA0..+0xF8 | double[11] | Verses (one per armor type) |
| +0xF8 | double | ProneDamage |
| +0x100 | int | DeformThreshold |
| +0x120 | int | InfDeath |
| +0x130 | bool | CausesDelayKill |
| +0x134 | int | DelayKillFrames |
| +0x138 | float | DelayKillAtMax |
| +0x13c | float | CombatLightSize |
| +0x144 | bool | Wall |
| +0x145 | bool | WallAbsoluteDestroyer |
| +0x146 | bool | PenetratesBunker |
| +0x14b | bool | Sonic |
| +0x14c | bool | Fire |
| +0x14d | bool | Conventional |
| +0x14e | bool | Rocker |
| +0x14f | bool | DirectRocker |
| +0x150 | bool | Bright |
| +0x151 | bool | CLDisableRed |
| +0x152 | bool | CLDisableGreen |
| +0x153 | bool | CLDisableBlue |
| +0x154 | bool | EMEffect |
| +0x155 | bool | MindControl |
| +0x156 | bool | Poison |
| +0x157 | bool | IvanBomb |
| +0x158 | bool | ElectricAssault |
| +0x159 | bool | Parasite |
| +0x15a | bool | Temporal |
| +0x15b | bool | IsLocomotor |
| +0x15c | CLSID | Locomotor (16 bytes) |
| +0x16c | bool | Airstrike |
| +0x16d | bool | Psychedelic |
| +0x16e | bool | BombDisarm |
| +0x170 | int | Paralyzes |
| +0x174 | bool | Culling |
| +0x175 | bool | MakesDisguise |
| +0x176 | bool | NukeMaker |
| +0x177 | bool | Radiation |
| +0x178 | bool | PsychicDamage |
| +0x179 | bool | AffectsAllies (default=true) |
| +0x17a | bool | Bullets |
| +0x17b | bool | Veinhole |

### Key Functions Called
| Address | Name/Purpose |
|---------|-------------|
| 0x005f8c90 | ObjectClass::ReceiveDamage — core HP deduction |
| 0x00489180 | Armor/Verses damage calculation |
| 0x0050bd30 | Type-based damage multiplier (Unit/Aircraft/Building/Infantry) |
| 0x0074ff90 | IsVeteran check (experience threshold) |
| 0x00750010 | IsElite check (experience threshold) |
| 0x0041bf40 | IsIronCurtained (timer check at +0x18c/+0x194) |
| 0x0070c5b0 | IsWarpingOut (reads +0x270) |
| 0x0048a620 | Play spark/flash animation (IC spark, ForceShield spark) |
| 0x006fb080 | Ammo depletion animation trigger |
| 0x007087c0 | Retaliation eligibility check |
| 0x00708080 | Threat assessment / AI base defense response |
| 0x007178c0 | Get turret offset for particle placement |
| 0x006ea870 | Eject passengers |
| 0x0070d690 | Crew/survivor spawning on death |
| 0x00707cb0 | Passenger kill/eject on host death |
| 0x004690b0 | WarheadTypeClass::Detonate (separate from ReceiveDamage) |

### Rules.ini Global Offsets (g_RulesClass_Instance +)
| Offset | Purpose |
|--------|---------|
| +0x8c | RetaliationDelay (frames) |
| +0x100 | AircraftDamageMultiplier |
| +0x104 | BuildingDamageMultiplier |
| +0x108 | UnitDamageMultiplier |
| +0x10c | InfantryDamageMultiplier |
| +0x110 | InfantryDamageMultiplier (prone/deployed) |
| +0x140 | ExplosionAnims array pointer |
| +0x14c | ExplosionAnims count |
| +0x16c8 | MaxDamage cap |
| +0x1700 | ConditionYellow threshold (double) |
| +0x1708 | ConditionRed threshold (double) |
| +0x17ec | MultiplayPassive flag |
| +0x17ed | MultiplayPassive2 flag |

---

## Summary Flowchart

```
ReceiveDamage(Damage*, dist, warhead, attacker, ignoreDefenses, attackerHouse)
  |
  +--> [1] Apply type-based damage multiplier (Unit/Air/Bldg/Inf)
  +--> [2] Apply veteran/elite damage modifier
  +--> [3] Clamp damage >= 1
  +--> [4] TypeImmune check → return 0
  |
  +--> [5] IronCurtain active? → spark anim, damage=0, return 0
  +--> [6] WarpingOut? → damage=0, return 0
  |
  +--> [7] Ammo absorption (if applicable)
  +--> [8] ForceShield / bunker penetration
  |
  +--> [9] Warhead immunity checks:
  |        Radiation + ImmuneToRadiation → 0
  |        PsychicDamage + ImmuneToPsionicWeapons → 0
  |        Poison + ImmuneToPoison → 0
  |        !AffectsAllies + allied → 0
  |
  +--> [10] Psychedelic (MindControl) → apply MC, return 1
  |
  +--> [11] ObjectClass::ReceiveDamage:
  |         - Armor/Verses calculation
  |         - Health -= Damage
  |         - Determine state (Yellow/Red/Dead)
  |         - Fire trigger events
  |         - If dead: RegisterDestruction, MarkForDeath
  |
  +--> [12] Post-damage: score tracking, retaliation timing
  +--> [13] Death cleanup: MC links, passengers, debris, survivors
  +--> [14] Damage particles (ConditionYellow/Red smoke)
  +--> [15] Retaliation or scatter
  |
  +--> return DamageState
```
