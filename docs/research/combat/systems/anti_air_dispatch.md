# Anti-Air Dispatch & Primary/Secondary Weapon Selection

This doc is the canonical reference for **`TechnoClass::SelectWeaponAgainst`** at
`0x006F3330` — the function that decides **which weapon** (Primary, Secondary, or a
Gattling stage) is fired at a given target. It implements:

- Anti-air weapon selection (`Projectile.AA` + target locomotor)
- Anti-ground default
- Verses-driven primary↔secondary swap ("primary can't damage this armor → use secondary")
- Naval-vs-air target gates
- Building-vs-vehicle special branches (Magnetron, ElectricAssault, Airstrike)
- Gattling stage-pair selection (ground stage × 2, AA stage × 2 + 1)
- Deploy-fire stickiness
- Sub-passenger / open-topped weapon overrides
- Dogfight aircraft swap
- Cell-target naval-targeting

Out-of-scope:
- The fire-error / can-target gate set → [`can_target_gates.md`](can_target_gates.md)
- The damage transform after a weapon is picked → [`damage_formula.md`](damage_formula.md)
- Veterancy-based weapon swap (`ElitePrimary=`/`EliteSecondary=`) → [`veterancy_weapon_swap.md`](veterancy_weapon_swap.md)
- Gattling stage progression (which stage is active) → [`gattling_spool.md`](gattling_spool.md)
- The Verses table layout → [`verses_armor_matrix.md`](verses_armor_matrix.md)

---

## 1. Function identity

| Field | Value |
|---|---|
| Address | `0x006F3330` |
| Ghidra label | `TechnoClass__SelectWeaponAgainst` (named) |
| Calling convention | `__thiscall(this: TechnoClass*, target: AbstractClass*) → int (weapon index)` |
| Returns | `0` = Primary, `1` = Secondary, `stage × 2` = Gattling ground stage, `stage × 2 + 1` = Gattling AA stage. A special `TechnoType.byte+0xD50` override can also be returned. |

### Callers (live)

```
FUN_005218e0 @ 005218e0   (likely InfantryClass weapon-pick helper)
FUN_00746cd0 @ 00746cd0   (likely UnitClass weapon-pick helper)
```

Plus indirect vtable callers from `Mission_Attack` / `Fire_At_Target` paths
(consistent with the `WhatAmI`-keyed per-tick attack dispatch documented in the
existing `FIRE_AT_ANALYSIS.md`).

### Confidence

- **Content: HIGH** — decompiled live 2026-05-17.
- **Identity: HIGH** — named in Ghidra annotation set; signature matches `int(TechnoClass*, AbstractClass*)` pattern.
- **Binding: MEDIUM** — only 2 direct callers in xref index; rest are vtable-indirect from per-tick attack handlers. Live binding to the combat pipeline is well-established via existing canonical docs.

---

## 2. Return code semantics

| Return | Meaning |
|---:|---|
| `0` | Use **Primary** (slot 0) |
| `1` | Use **Secondary** (slot 1) |
| `stage × 2` (e.g., 0, 2, 4, 6, 8, 10) | Gattling ground weapon for stage 0..5 |
| `stage × 2 + 1` (e.g., 1, 3, 5, 7, 9, 11) | Gattling AA weapon for stage 0..5 |
| `type.byte+0xD50` | Per-type weapon-index override (open-topped passenger weapon, etc.) |

For Gattling weapons, `WeaponSlot[N]` is laid out as pairs: `[stage0_ground,
stage0_AA, stage1_ground, stage1_AA, ...]`. See [`gattling_spool.md`](gattling_spool.md)
for the stage-progression machinery.

---

## 3. Decision tree (decompiled, in order)

```
TechnoClass::SelectWeaponAgainst(this, target):

# A. Deploy-fire stickiness (non-Gattling units in deploy state)
type = this.vtable+0x84()      # GetTechnoType
if FUN_00717880(type)           # type.byte+0x808 > 0 (DeployFire weapon defined)
   AND type.IsGattling (type.byte+0xCD5) == 0:
    return (CurrentWeaponNumber != -1) ? CurrentWeaponNumber : 0
    # i.e., stick with whatever weapon the deploy is currently using.

# B. Garrison/Occupied attackers return 0 (Primary only)
if this.vtable+0x400()           # IsOccupied / IsGarrisoned
    return 0
    # Garrison units always use weapon 0.

# C. Load both weapons
secondary = this.vtable+0x3F8(1)   # GetWeapon(1)
if secondary == NULL:
    return 0    # No secondary → only Primary available

primary = this.vtable+0x3F8(0)     # GetWeapon(0)
if primary == NULL:
    return 0    # No primary either → use 0 (vacuous)

if secondary.byte+0x136 != 0:       # LimboLaunch-style secondary
    return 0    # Secondary is a deploy-only weapon → don't pick it for normal fire

if target == NULL:
    return 0

# D. Open-topped passenger override (sub-passenger weapon table)
target_techno = (target.byte+5 & 1) ? target : NULL
if this.field_0x82 (IsOpenTopped) != 0
   AND type.field_0xD50 != -1:
    return type.field_0xD50    # OpenTransportWeapon — specific weapon index for passenger

# E. Gattling — stage × 2 ± 1 dispatch
if type.IsGattling (+0xCD5) != 0:
    stage = this.CurrentGattlingStage
    if secondary.Projectile.AA (+0x2A4) != 0
       AND target_techno != NULL
       AND target.IsAirborne (vtable+0x54):
        return stage × 2 + 1     # Gattling AA stage
    return stage × 2              # Gattling ground stage

# F. Airstrike branch (primary.warhead.Airstrike)
if primary.warhead.Airstrike (warhead+0x16C) != 0:
    if target.WhatAmI() == 6 (Building):
        building_type = target.Type
        if building_type.byte+0x5ED == 0:    # cannot-airstrike-this-building flag
            return 1
        if building_type.byte+0x5EC == 0:    # cannot-airstrike flag 2
            return 1
    fall through to G  (i.e., Airstrike-primary against non-building uses primary normally)

# G. Secondary-prefers-Building (Magnetron)
if secondary.warhead.IsLocomotor (warhead+0x15B) != 0
   AND target_techno != NULL
   AND target.WhatAmI() == 6 (Building):
    return 1
    # Magnetron's primary tries to lift; against buildings, use secondary direct damage.

# H. Naval-gunboat → Secondary against naval-flagged targets
if secondary.byte+0x142 (NavalGunboat / NavalTargeting flag) != 0
   AND target_techno != NULL
   AND target.Type.byte+0x5EF (NavalTarget) != 0
   AND this.field_0x1CC == 0    # not in team-leader mission
   AND !HouseClass::Is_Ally_ByObject(target_techno):
    return 1

# I. Animation-state-dependent swap
if secondary.byte+0x150 != 0
   AND this.vtable+0x184() (AnimState) == 0x10:
    return 1
    # When attacker is in a specific animation state, swap to secondary.

# J. Building-internal-garrison swap
if this.WhatAmI() == 6 (Building)
   AND this[1].field_0x549 != 0:    # IsInternalGarrison or similar
    return 1

# K. Electric-Assault on allied tech building
if target.WhatAmI() == 6 (Building)
   AND HouseClass::Is_Ally_ByObject_WithFlag(target)
   AND secondary.warhead.ElectricAssault (warhead+0x158) != 0
   AND target.Building.Type.byte+0x1575 (IsAlliedTechBuilding) != 0:
    return 1

# L. Dogfight aircraft swap
if this.WhatAmI() == 2 (Aircraft)
   AND this[1].field_0x1AA (IsDogfighting) != 0:
    return 1

# M. Cell-target dispatch
if target.WhatAmI() == 0xB (CellClass):
    if (target.byte+0xEC != 2                     # not in attached-to-bridge state
        AND target.vtable+0x50()                  # cell.IsBeach / IsWater
        OR (cell.byte+0x140 bit 0x100             # cell is bridge
            AND type.byte+0xCCE != 0))            # NavalGunboat=2 enables bridge targeting
        AND target.vtable+0x54() == 0             # not airborne
        AND type.field_0x604 == 2:                # NavalGunboat=2 flag
        return 1

# N. Verses-driven swap (the MOST important branch)
if target_techno != NULL:
    target_armor = target.Type.Armor (type+0x9C)
    if secondary.warhead.Verses[target_armor] != 0:    # secondary CAN damage target
        if primary.warhead.Verses[target_armor] == 0:  # primary CANNOT damage target
            return 1                                    # USE SECONDARY (primary is useless)

        # Both can damage — apply tie-breakers
        is_naval_target = (target.locomotor_mode == 2 || == 6)
        if target.IsAirborne (vtable+0x54):
            is_naval_target = false

        if !is_naval_target AND target.OnBridge (target.byte+0x23) == 0:
            # Both weapons hit; check naval-targeting override
            override = this.vtable+0x2E8(target)  # GetNavalWeaponSelect / similar
            if override != -1:
                return override
        else:
            if !target.IsAirborne AND type.field_0x604 == 2:    # NavalGunboat=2
                return 1
            if secondary.Projectile.AA != 0 AND target.IsAirborne:
                return 1     # use Secondary for AA

# Z. Default: Primary
return 0
```

---

## 4. Branch summary table

| Phase | Trigger | Result |
|---|---|---|
| A | Deploy-fire active (non-Gattling) | **stick** with CurrentWeaponNumber |
| B | Attacker is garrisoned | Primary (0) |
| C | No secondary OR no primary OR secondary is LimboLaunch | Primary (0) |
| D | Attacker is open-topped passenger w/ type override | type.byte+0xD50 |
| E | Gattling, AA-target match | stage × 2 + 1 |
| E | Gattling, default | stage × 2 |
| F | Primary is Airstrike, target is non-Airstrikable Building | Secondary (1) |
| G | Secondary is Magnetron, target is Building | Secondary (1) |
| H | Secondary is NavalGunboat, target is naval & enemy | Secondary (1) |
| I | Secondary `+0x150` flag, attacker in anim-state 0x10 | Secondary (1) |
| J | Attacker is Building w/ internal-garrison | Secondary (1) |
| K | Target is allied tech-building, secondary is ElectricAssault | Secondary (1) |
| L | Aircraft dogfighting | Secondary (1) |
| M | Target is Cell on water/bridge w/ NavalGunboat=2 | Secondary (1) |
| N | Verses: primary 0%, secondary > 0% | Secondary (1) |
| N | Verses tie-breaker (both > 0%): naval/air rules | varies |
| Z | Default | Primary (0) |

---

## 5. Key flag identities

### Weapon flags (`weapon+offset`)

| Offset | Flag | Branch |
|---|---|---|
| `+0x9C` | `Burst` | (not read by this function) |
| `+0xA0` | `Projectile` (pointer to BulletTypeClass) | E (AA check via Projectile.AA) |
| `+0xAC` | `Warhead` (pointer to WarheadTypeClass) | F/G/H/K/N (warhead flags) |
| `+0xB4` | `Range` | (not read here) |
| `+0x136` | LimboLaunch / spawner-only flag | C (skip secondary if set) |
| `+0x142` | `NavalGunboat` / `NavalTargeting` | H |
| `+0x150` | (unknown anim-state flag) | I |

### Warhead flags (`warhead+offset`)

| Offset | Flag | Branch |
|---|---|---|
| `+0xA0` | `Verses[11]` (double array) | N |
| `+0x158` | `ElectricAssault` | K |
| `+0x15B` | `IsLocomotor` (Magnetron) | G |
| `+0x16C` | `Airstrike` | F |

### Projectile flags (`projectile+offset`)

| Offset | Flag | Branch |
|---|---|---|
| `+0x2A4` | `AA` | E, N |
| `+0x2A5` | `AG` | (used by GetFireError, not here) |

### TechnoType flags (`type+offset`)

| Offset | Field | Branch |
|---|---|---|
| `+0x9C` | `Armor` (int 0..10) | N (target lookup) |
| `+0x5EC` / `+0x5ED` | Airstrike-not-applicable building flags | F |
| `+0x5EF` | `NavalTarget` | H |
| `+0x604` | `NavalGunboat` (int, 2 = naval-and-cell-target) | M, N |
| `+0x808` | DeployFire-related (`>0` = unit has a deploy weapon) | A |
| `+0xCCE` | `NavalGunboat=2`-on-bridge flag | M |
| `+0xCD5` | `IsGattling` | A, E |
| `+0xD50` | OpenTransportWeapon override index (or -1) | D |

### TechnoClass instance fields

| Offset | Field | Branch |
|---|---|---|
| `+0x82` | `IsOpenTopped` | D |
| `+0x140` | `CurrentGattlingStage` (int) | E |
| `+0x1AA` (offset relative to `this[1]`) | `IsDogfighting` | L |
| `+0x549` (offset relative to `this[1]`) | `IsInternalGarrison` | J |
| `+0x1CC` | `team_leader_target` | H |

These should be cross-verified against the relevant struct docs in the root
`ra2-rust-game-docs/` — many are TS-legacy candidates flagged below.

---

## 6. The Verses swap is the load-bearing branch

Phase N is the most important branch in this function — and the most-frequently-hit
one in normal gameplay. The contract is simple:

```
If secondary CAN damage target's armor (Verses[armor] != 0):
    If primary CANNOT damage target's armor (Verses[armor] == 0):
        Use secondary.
```

This is why an Allied Grizzly Tank (whose primary cannon has `Verses[infantry-armor]=0%`)
automatically uses its **machine-gun secondary** against infantry, even though the
player gave no explicit orders. The opposite (primary can hit, secondary can't) does
NOT swap — primary stays.

When **both** weapons can hit (Verses > 0 on both), the tie-breaker rules in Phase N's
second half pick the more specialized weapon. The mechanism uses target locomotor mode,
naval-targeting flags, and bridge-attachment state to pick the "right" weapon for the
context. Aircraft → secondary (likely AA-specialized), naval → naval-specialized weapon,
etc.

### When BOTH Verses are 0

If neither weapon can damage the target's armor, the function still picks one (defaults
to Primary via the Phase Z fall-through). The actual fire then either:
- Returns FireError code 6 ("CANT") from GetFireError gate #59, OR
- Fires anyway under ForceFire and deals 0 damage.

See [`can_target_gates.md`](can_target_gates.md) §7 for the GetFireError-side
behavior, and [`damage_formula.md`](damage_formula.md) §6 for the damage-side.

---

## 7. Airstrike branch (Phase F) detail

The Airstrike flag (`warhead+0x16C`) marks a weapon that **calls in an off-map
airstrike** rather than firing a normal projectile. Harriers, Black Eagles, and the
Airstrike support power use this.

The branch tries to determine whether the target is a **valid airstrike target**:
- For Buildings: target.Type has two flags `+0x5EC` and `+0x5ED` that gate "is this
  building airstrike-able." If EITHER is zero (the actual condition is paired and the
  semantics aren't fully traced), the function falls through to Secondary (return 1).
- For non-Buildings: the airstrike fires normally (Primary, falls through to default).

The `+0x5EC` / `+0x5ED` paired check is suspicious — looks like it might be
"AllowedToCallAirstrike" + "MustHaveAirfield" or similar. Open follow-up.

---

## 8. Open-topped passenger weapon (Phase D)

When an attacker is in an open-topped transport (`+0x82`), the **transport** is
firing on behalf of the passenger. The transport's TechnoType has a field at
`+0xD50` that maps to an integer "weapon index to use when the open-topped occupant
is firing." If the field is `!= -1`, that index is returned directly, bypassing all
of Phases E..Z.

Used to define IFV-style weapon-swap behavior at the type level (IFV's many INI
sub-weapons are implemented this way per existing `IFV_WEAPON_TABLE`-style docs —
flagged as a hardcoded-weapon doc TODO `weapons/IFVWeaponTable.md`).

---

## 9. Gattling dispatch (Phase E)

Gattling weapons differ from normal Primary/Secondary in that they have **N stage-pairs**
in their weapon list. `WeaponSlot[2k]` = ground weapon for stage k. `WeaponSlot[2k+1]`
= AA weapon for stage k. Phase E:

```
stage = this.CurrentGattlingStage
if target is airborne AND weapon[2k+1].Projectile.AA:
    return 2k + 1
else:
    return 2k
```

The "secondary" pointer for the AA check is actually the AA-slot for the current
stage — not weapon slot index 1. The decomp's `secondary` variable is overloaded as
"weapon slot index for the current stage's AA variant" when IsGattling is set.

The CurrentGattlingStage value comes from the gattling progression system documented
in [`gattling_spool.md`](gattling_spool.md). It cycles up while firing (or
near-target) and decays while idle.

---

## 10. NavalGunboat semantics (Phases H, M, N)

The function references THREE separate "naval-targeting" concepts that should not be
conflated:

1. **`weapon.byte+0x142`** — the secondary weapon's `NavalGunboat=`/`NavalTargeting=` flag (probable). Branch H. Triggers swap when target is naval-flagged + enemy.
2. **`target.Type.byte+0x5EF`** — the TARGET's `NavalTarget=` flag. Branch H reads this on the candidate target.
3. **`attacker.Type.field_0x604`** — the ATTACKER type's `NavalGunboat=` integer (treated as `== 2` test). Branch M, Branch N. Triggers swap for cell targets on water/bridge AND for naval targets on bridges.

The `NavalGunboat=` INI key is parsed as an int with values 0, 1, 2 (with 2 being
the "full naval" mode). Specific INI key mapping not re-verified in this pass — flag
for follow-up.

---

## 11. TS-legacy filter

Most branches are LIVE in YR. Suspect TS-legacy:

- **Phase J** (`this[1].field_0x549` IsInternalGarrison check on Building) — likely active for Battle Bunker / Garrison structures. Live.
- **Phase L** (aircraft dogfighting `this[1].field_0x1AA`) — dogfight was a TS feature; YR keeps it for Harrier/Black Eagle/MIG. Live, but rarely exercised.
- **Phase F** Airstrike branch — Airstrike is a YR feature (Black Eagle, Harrier carrier-call). Live.
- **`weapon.byte+0x150`** Phase I anim-state flag — not seen in shipping INIs. Possibly TS-legacy or modder-only. Open follow-up to verify.
- **`type.field_0x808`** Phase A — DeployFire is a YR feature (Siege Chopper, Deployer Truck). Live.

No branch confirmed dead in YR.

---

## 12. Edge cases

| Case | Behavior |
|---|---|
| Target is dead during selection | Filtered out in upstream Mission_Attack — SelectWeaponAgainst still runs and returns a number, but the actual fire is gated by GetFireError. |
| Both weapons have Verses=0% for target | Falls through Phase N (no swap) → returns 0 (Primary). Subsequent GetFireError will reject the fire. |
| Both weapons have Verses=100%+ for target | Phase N inner block runs tie-breaker; for ground non-bridge targets, no swap (return 0 default). |
| Target.Armor index out-of-range | Out-of-range reads past the Verses array into adjacent fields. In practice, `FUN_00772a50` (the armor-name → index lookup) clamps to 0..10, so this never happens. |
| Attacker has no secondary weapon | Phase C returns 0. |
| Gattling unit attacks an air target with no-AA-projectile at current stage | Phase E falls back to `stage × 2` (ground weapon) — even though target is in air. Weapon will then fail GetFireError's AA check and produce a misfire. The cure is to ensure AA-stage weapons have `Projectile.AA=yes`. |
| Aircraft attacker on Airstrike weapon | Phase F enters, target's airstrike-flags determine whether to swap. Combined with the dogfight check (Phase L), priority is L → F (L runs later, so L wins for dogfighting aircraft). |

---

## 13. Open follow-ups

1. **`+0x5EC` / `+0x5ED` building-airstrike flag identity** (Phase F). Suspected to be `CanBeOccupied` / `IsTechBuilding` or similar. Needs INI parser trace.
2. **`weapon.byte+0x150`** identity (Phase I). No retail INI sets it; could be TS-legacy. Priority: LOW.
3. **`this.vtable+0x2E8`** identity (Phase N's tie-breaker) — used as the "GetNavalWeaponSelect" override. Decompile and document the vtable slot semantics. Priority: MEDIUM (matters for naval engagements).
4. **`weapon.byte+0x142`** vs **`type.field_0x604`** vs **`type.field_0xCCE`** — three "naval" flags. Resolve INI key for each. Priority: MEDIUM.
5. **OpenTransportWeapon `type.field_0xD50`** — the integer is a weapon index, but the INI parsing key isn't verified. Trace `IFV_WEAPON_TABLE.md` or similar; flag the hardcoded-weapon doc to use it. Priority: HIGH (IFV depends on this).
6. **`type.byte+0x549` Building "InternalGarrison" flag** identity (Phase J). Priority: LOW.
7. **`type.field_0x808`** identity (Phase A) — DeployFire-related, probably `DeployFireWeapon=` or `DeploysInto=`. Trace ReadINI. Priority: MEDIUM.
8. **Verses tie-breaker (Phase N second half)** when both weapons damage target — the exact tie-breaker order needs more careful trace, including `IsHighFlying` interactions. Priority: HIGH (parity-critical for AA dispatch).

---

## 14. Sources

- Live decompilation of `TechnoClass::SelectWeaponAgainst` at `0x006F3330` (2026-05-17).
- Live decompilation of `FUN_00717880` at `0x00717880` (the deploy-fire predicate — returns `type+0x808 > 0`).
- Caller list via `get_function_callers 0x006F3330`: `FUN_005218e0`, `FUN_00746cd0` (plus indirect vtable dispatch from per-tick attack handlers).
- Existing canonical doc: [`../../DAMAGE_MATH_GHIDRA_REPORT.md`](../../DAMAGE_MATH_GHIDRA_REPORT.md) §9 — prior coverage of SelectWeaponAgainst's decision tree, partial. This doc supersedes for AA dispatch / weapon selection logic.
- Cross-reference: [`damage_formula.md`](damage_formula.md) §6 (Verses formula), [`verses_armor_matrix.md`](verses_armor_matrix.md) (Verses array + armor type layout), [`can_target_gates.md`](can_target_gates.md) §7 (engine-side Verses=0 target-block), [`gattling_spool.md`](gattling_spool.md) (CurrentGattlingStage progression).
