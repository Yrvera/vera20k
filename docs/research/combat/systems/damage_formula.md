# Damage Formula — Master Math

This doc is the canonical reference for the master damage function in gamemd.exe:
**`WarheadTypeClass__GetDamage`** at `0x00489180` (a.k.a. `FUN_00489180`).

It covers ONLY the per-target damage transformation: how a raw integer damage value
becomes a per-target final damage value via CellSpread falloff, Verses (armor)
multiplier, healing gate, and MaxDamage clamp.

Out-of-scope (each has its own systems doc):
- Fire_At-side attacker modifiers (strictly-positive grouped country/per-unit
  firepower, vet/elite firepower, then civilian-garrison/tank-bunker/open-topped
  containment; special zero rejoins containment) →
  [`fire_at_pipeline.md`](fire_at_pipeline.md). Corrected from active
  `disassemble_function 0x006fdd50` on 2026-07-13.
- ReceiveDamage-side defender modifiers (country armor mult, vet/elite armor, immunities, Iron Curtain, ForceShield, friendly fire gate) → [`receive_damage_pipeline.md`](receive_damage_pipeline.md)
- The Verses array layout and parsing → [`verses_armor_matrix.md`](verses_armor_matrix.md)
- CellSpread target collection + per-target distance computation → [`splash_cellspread.md`](splash_cellspread.md)
- PercentAtMax linear-interpolation derivation in fuller depth → [`percentatmax_falloff.md`](percentatmax_falloff.md)
- MaxDamage clamp consumers across the codebase → [`maxdamage_clamp.md`](maxdamage_clamp.md)

---

## 1. Function identity

| Field | Value |
|---|---|
| Address | `0x00489180` |
| Body | `0x00489180 – 0x0048926c` (verified via `get_function_by_address` 2026-05-17) |
| Ghidra label | `FUN_00489180` (unlabeled in current annotation set) |
| Callers | three: `FUN_006fdb80` (pre-fire estimation), `ObjectClass__ReceiveDamage` at `0x005f5390`, `TechnoClass__ReceiveDamage` at `0x00701900` |
| Calling convention | `__fastcall(uint damage, int wh, undefined4 armorType, int distance)` (ECX=damage, EDX=wh, stack0=armorType, stack1=distance) |

### Confidence (3-axis)

- **Content: HIGH.** Decompilation read 2026-05-17, body matches the formula stated below. Two `Math__ftol` calls within the conditional confirm two truncation points (one after distance falloff, one after Verses multiply). Early-out structure and MaxDamage clamp confirmed at `Rules+0x16c8`.
- **Identity: HIGH.** Function is the only callee of all three damage-pipeline call sites that take a `(damage, warhead, armor, distance)` quadruple. The structure of the function — Verses lookup at `wh+0xA0+armorType*8`, CellSpread at `wh+0x124`, PercentAtMax at `wh+0x12C` — uniquely identifies it as the per-target damage transform.
- **Binding: HIGH.** Verified via `get_function_callers 0x00489180` on 2026-05-17. Three call sites: pre-fire estimation (`FUN_006fdb80`), the per-target armor application inside `ObjectClass::ReceiveDamage`, and the Psychedelic-warhead path inside `TechnoClass::ReceiveDamage`. There is no other entry point into the damage transform — every damage number a player sees passes through here.

### TS-legacy filter

The function itself is fully live in YR — every caller is on a path reachable in a vanilla
skirmish (fire-at-target, pre-fire estimation, psychedelic mind-control hit). No TS-only
gating.

---

## 2. Inputs

| Param | Source | Meaning |
|---|---|---|
| `damage` | integer stored after Fire_At's positive-only grouped country/per-unit stage, optional veterancy, then civilian-garrison/tank-bunker/open-topped containment; special zero rejoins containment | integer |
| `wh` | `WeaponTypeClass.Warhead` (`weapon+0xC4`) of the firing weapon | pointer to `WarheadTypeClass`, may be NULL |
| `armorType` | `TechnoTypeClass.Armor` of the target (`type+0x9C`), an integer 0–10 indexing the Verses array | int 0–10 |
| `distance` | leptons from the **impact point** (not the firing unit) to the **target center**. Computed in `Apply_area_damage` per-target. For point-blank / no-splash cases (e.g. pre-fire estimate) it is `0`. | int leptons |

### Lepton scale (verified constant)

| Address | Value | Meaning |
|---|---|---|
| `0x007e2224` | `256.0f` | leptons per cell. `CellSpread * 256.0` gives spread radius in leptons. |

---

## 3. Early-out conditions (verified)

```
if (damage == 0) return 0;
if (g_ScenarioClass_Instance->Flag & 0x20) return 0;  // some global blocks all damage
if (wh == NULL) return 0;
```

The `0x20` ScenarioClass flag is checked from a global instance; its setter has not been
re-traced in this iteration. Treat as "exists but rarely set in normal play" — flag for
follow-up if observed in a save-game.

---

## 4. Negative damage (healing) path — verified asm

When `damage < 0` (a heal warhead, e.g. `[ParaBomb]`-style with negative `Damage=`):

Decompilation (Ghidra 2026-05-17):
```
return (7 < param_4) - 1 & param_1;
```

This is a branchless gate:
- `distance ≤ 7` leptons → `(7 < dist) = 0`, `0 - 1 = -1 = 0xFFFFFFFF`, `0xFFFFFFFF & damage = damage` → returns the negative heal value unchanged.
- `distance ≥ 8` leptons → `(7 < dist) = 1`, `1 - 1 = 0`, `0 & damage = 0` → blocks heal.

Plain math:
```
if (damage < 0):
    if (distance < 8): return damage    // heal allowed within 8 leptons of impact
    else:              return 0          // heal blocked at distance
```

This gates AoE healing to a tiny radius around the impact point (1/32 of a cell). It does
**not** discriminate by armor type — Verses is not consulted for healing.

### Confidence

- **Content: HIGH** (single-instruction decode of branchless assembly, both numeric edges checked).
- **Identity: HIGH** (this is the `(damage < 0)` branch of the master `GetDamage`).
- **Binding: HIGH** (only consumer of this path is the same three callers as the function).

---

## 5. Positive damage: CellSpread distance falloff

For `damage > 0`:

```
percentAtMax_dmg = (float)damage * wh->PercentAtMax              // wh+0x12C (float, default 1.0)
cellSpread_leptons = ftol(wh->CellSpread * 256.0)                // wh+0x124 (float, default 0.0)

if (percentAtMax_dmg != (float)damage  &&  cellSpread_leptons != 0):
    // linear lerp from 100% at center to PercentAtMax at the spread edge
    falloff = percentAtMax_dmg
            + (damage - percentAtMax_dmg) * (cellSpread_leptons - distance) / cellSpread_leptons
    damage = ftol(falloff)
```

Equivalent form:
```
t = distance / cellSpread_leptons                          // unclamped
damage = ftol( damage * lerp(1.0, PercentAtMax, t) )
```

Notes:

- If `PercentAtMax == 1.0` (the default), the first condition fails (`percentAtMax_dmg == damage`) and the falloff block is skipped — damage stays at full strength regardless of distance.
- If `CellSpread == 0.0` (the default for almost every weapon), `cellSpread_leptons` is 0 and the condition is also skipped — no falloff applied, single-target damage.
- The lerp is **not clamped** in the source — `distance > cellSpread_leptons` produces negative `(cellSpread - distance)`, which makes the second term negative and pushes damage **below** `PercentAtMax * base`. In practice, every caller (`Apply_area_damage` and the Verses test) ensures `distance ≤ cellSpread_leptons` before invoking `GetDamage`, so this edge is not exercised in normal play. Still: do not assume implicit clamping.
- For `distance == 0` (the pre-fire estimate path and dead-center hits): `falloff = percentAtMax_dmg + (damage - percentAtMax_dmg) * 1 = damage` → full damage, as expected.
- For `distance == cellSpread_leptons` (the very edge): `falloff = percentAtMax_dmg + 0 = percentAtMax * damage` → PercentAtMax of base, as expected.

### Worked example

100-damage weapon, Verses=50% vs Heavy(2), `CellSpread=1.0`, `PercentAtMax=0.25`, `distance=128` leptons:

```
cellSpread_leptons = ftol(1.0 * 256.0) = 256
percentAtMax_dmg   = 100 * 0.25 = 25.0
falloff            = 25.0 + (100 - 25.0) * (256 - 128) / 256
                   = 25.0 + 75.0 * 0.5
                   = 62.5
damage_after_spread = ftol(62.5) = 62
```

Two-decimal walk for the Verses + clamp steps continues in §6 and §7.

### Confidence

- **Content: HIGH** for the algebraic form (matches Ghidra decompilation read 2026-05-17). The `(float)damage * fVar1 != (float)damage` test in the decomp is the `percentAtMax != 1.0` short-circuit.
- **Identity: HIGH** (field offsets `wh+0x124` and `wh+0x12C` match `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`).
- **Binding: HIGH** (single callsite-set with three callers, all verified).

### Cross-ref

- The CellSpread target-collection loop (which cells are scanned, what `distance` is set to for each candidate) lives in [`splash_cellspread.md`](splash_cellspread.md).
- A long-form derivation of the lerp form is in [`percentatmax_falloff.md`](percentatmax_falloff.md).

---

## 6. Verses (armor) multiplier

After the spread falloff:

```
if (damage <= 0):
    damage = 0                       // clamp negative-after-spread to zero
verses = wh->Verses[armorType]        // wh+0xA0 + armorType*8, double
damage = ftol((float)damage * verses)
```

Field layout (verified):

| Offset | Type | Field |
|---|---|---|
| `wh+0xA0` | `double[11]` | `Verses` |

The 11 armor types are 0..10 (none, flak, plate, light, medium, heavy, wood, steel, concrete, special_1, special_2). Each entry is a double parsed from the comma-separated `Verses=` INI key.

Worked example continued (Verses=0.5 vs Heavy):
```
versed_damage = ftol(62 * 0.5) = ftol(31.0) = 31
```

### Confidence

- **Content: HIGH** (decomp shows two adjacent `Math__ftol` calls — one for the spread result, one for the Verses-multiplied result).
- **Identity: HIGH** (offset `0xA0` and stride `8` (sizeof double) match the struct doc and INI parser).
- **Binding: HIGH** (same three-caller binding).

### Cross-ref

- Full Verses array semantics and INI parsing → [`verses_armor_matrix.md`](verses_armor_matrix.md).
- "What does Verses=0% actually do?" (it makes the weapon unable to target that armor — see SelectWeaponAgainst) → [`anti_air_dispatch.md`](anti_air_dispatch.md) and [`can_target_gates.md`](can_target_gates.md).

---

## 7. MaxDamage clamp

```
if (versed_damage >= Rules->MaxDamage):    // Rules+0x16C8, int
    return Rules->MaxDamage
return versed_damage
```

Stock merged value: `MaxDamage=10000` (from `[CombatDamage]` in rulesmd.ini);
the constructor/missing-key fallback is 1000. Verified at
`Rules+0x16c8` (int, not float). The clamp is **applied per target**, after Verses,
**not** to the raw weapon damage — so a 5000-damage weapon at 0% Verses can still
return `0`, and a 200% Verses on a 6000-damage attack still clamps to 10000.

### Confidence

- **Content: HIGH** (decomp shows direct integer compare `(int)*(uint *)(g_RulesClass_Instance + 0x16c8) <= (int)uVar3`).
- **Identity: HIGH** (offset `0x16C8` cross-referenced to `[CombatDamage] MaxDamage` parsing).
- **Binding: HIGH** (in this function — but consumers of the clamp elsewhere in the codebase are documented in [`maxdamage_clamp.md`](maxdamage_clamp.md)).

---

## 8. Complete single-expression form

```
GetDamage(damage, wh, armorType, distance) =
    if damage == 0 or wh == NULL or scenarioFlag & 0x20:
        return 0
    elif damage < 0:
        return damage if distance < 8 else 0
    else:
        // distance falloff
        t = distance / (wh.CellSpread * 256.0)              if wh.CellSpread > 0 and wh.PercentAtMax != 1.0 else 0
        spread = ftol(damage * lerp(1.0, wh.PercentAtMax, t))
        spread = max(spread, 0)
        // verses
        versed = ftol(spread * wh.Verses[armorType])
        // clamp
        return min(versed, Rules.MaxDamage)
```

### Two truncations matter

Both `ftol` calls happen at integer boundaries, so the final value can differ from a
single-float computation by a couple of points due to accumulated truncation. Any Rust
reimplementation must use **integer-truncating** semantics at both boundaries
(`spread → int`, then `int * Verses → int`) — not a single-pass float multiply.

The truncation is **toward zero** (standard `ftol` x87 behavior with the default control
word). Negative-after-spread values are clamped to 0 before the Verses multiply, so the
second truncation never sees a negative.

---

## 9. Where this fits in the full pipeline

```
[Fire_At]                                                  [src: fire_at_pipeline.md]
    Wave/special ? 0 : weapon.Damage
    if ordinary Damage > 0:
        ftol((countryFirepower * unitFirepower) * Damage)
        ftol(* VeteranCombat) when ability enabled
    ftol(* OccupyDamageMultiplier) when civilian-garrisoned
    ftol(* BunkerDamageMultiplier) when in tank bunker
    ftol(* OpenToppedDamageMultiplier) when open-topped
                            │
                            ▼
[BulletClass.Damage stored, projectile flies]              [src: bullet_lifecycle.md]
                            │
                            ▼
[Bullet impact → Apply_area_damage(impactCoord, dmg, ...)] [src: splash_cellspread.md]
    for each target in CellSpread radius:
        compute per-target distance (leptons from impact to target center)
        call target.ReceiveDamage(&dmg, distance, wh, ...)
                            │
                            ▼
[TechnoClass::ReceiveDamage]                               [src: receive_damage_pipeline.md]
    *dmg = ftol(*dmg * countryArmorMult)                   // House.Armor{Inf,Unit,...}Mult
    *dmg = ftol(*dmg / VeteranArmor) if vet/elite ARMOR    // Rules.VeteranArmor ~1.5
    *dmg = max(*dmg, 1)
    immunities, IC, ForceShield, AffectsAllies, ...
                            │
                            ▼
[ObjectClass::ReceiveDamage]                               [src: receive_damage_pipeline.md]
    if not ignoreDefenses:
        *dmg = FUN_00489180(*dmg, wh, type.Armor, distance)  ◄── THIS DOC
    Health -= min(*dmg, currentHealth)
    handle yellow/red transitions, kill, score
```

**Critical detail:** `Apply_area_damage` passes the **same raw `damage`** to every
target's `ReceiveDamage`, with each target's individual `distance`. The falloff is
**computed inside `GetDamage`**, not in the dispatcher. This means callers must not
pre-attenuate; the falloff is the warhead's contract.

---

## 10. Edge cases and surprises

| Case | Behavior | Source |
|---|---|---|
| `damage == 0` | early-out 0 | §3 |
| `wh == NULL` | early-out 0 | §3 |
| Scenario flag 0x20 set | early-out 0 | §3 |
| `damage < 0, distance ≤ 7` | return `damage` (heal) | §4 |
| `damage < 0, distance ≥ 8` | return 0 (heal blocked) — note Verses is **not** consulted for healing | §4 |
| `PercentAtMax = 1.0` (default) | falloff skipped, full damage to every CellSpread target | §5 |
| `CellSpread = 0` (default) | falloff skipped (the cellSpread_leptons==0 short-circuit) | §5 |
| `distance > CellSpread*256` | uncllamped lerp → damage below `PercentAtMax*base`. In practice, callers gate this, so unexercised — but DO NOT rely on implicit clamping | §5 |
| `Verses[armor] = 0` | returns 0 regardless of `damage` | §6. Note: this also drives weapon selection (Primary→Secondary swap when Primary can't damage target armor) — see [`anti_air_dispatch.md`](anti_air_dispatch.md). |
| `Verses[armor] = 2.0` (200%) | doubles damage, still subject to MaxDamage clamp | §6, §7 |
| Damage that would exceed `MaxDamage` | clamped to `MaxDamage` (constructor fallback 1000; stock merged 10000) | §7 |
| Spread-result rounded to negative (e.g. PercentAtMax negative — invalid but parseable) | clamped to 0 before Verses | §6 |

---

## 11. Pre-fire estimation note

`FUN_006fdb80` calls `GetDamage(damage, wh, armorType, /*distance=*/0)` after applying
attacker veterancy / country / target-veterancy modifiers, using `distance=0` (point-blank
assumption). This drives the **EstimatedHealth overkill prevention** on target+0x70.

This is **not the actual damage value** delivered at impact — it's a pre-commit estimate
so the engine doesn't over-allocate firepower at one target. Full coverage in
[`fire_at_pipeline.md`](fire_at_pipeline.md).

---

## 12. Open follow-ups

- Trace the writer of the ScenarioClass `0x20` flag in §3 — currently un-traced. Suspect it's a global "no damage" debug or trigger flag; flag for `combat_damage_globals.md` to resolve.
- Confirm the truncation rounding mode (`ftol`) reads control word 0x027F (default Win32) at the entry of damage-dealing code paths — not yet verified, but consistent with all other observed `ftol` sites in gamemd.exe.

---

## 13. Sources

- Live Ghidra decompilation of `FUN_00489180` at `gamemd.exe:0x00489180` (read 2026-05-17).
- Existing root-level canonical doc: [`../../DAMAGE_MATH_GHIDRA_REPORT.md`](../../DAMAGE_MATH_GHIDRA_REPORT.md) — superseded for the master formula by this file; still authoritative for the full multi-stage pipeline pending the per-system docs that take over each section.
- WarheadTypeClass struct layout: [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md).
- Function caller list verified via `get_function_callers 0x00489180` on 2026-05-17 — three callers (FUN_006fdb80, ObjectClass::ReceiveDamage at 0x005f5390, TechnoClass::ReceiveDamage at 0x00701900).
