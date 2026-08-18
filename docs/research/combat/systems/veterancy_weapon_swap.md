# Veterancy & Weapon Swap

This doc is the canonical reference for the **veterancy system** in gamemd.exe as it
relates to combat:

- The three veterancy tiers (Rookie / Veteran / Elite) and their float-threshold gate
- `Veterancy` storage on every TechnoClass instance (a single float)
- The **Elite weapon swap** mechanism via `TechnoClass::GetWeapon` (`0x0070E140`) and
  its two helper offset functions (`+0x898` regular, `+0xA94` elite)
- `ElitePrimary=` / `EliteSecondary=` INI keys and their parser location
- The `VeteranAbilities=` / `EliteAbilities=` flag arrays
- `VeteranCombat=`, `VeteranArmor=`, `VeteranSpeed=`, `VeteranSight=`, `VeteranCap=`,
  `VeteranRatio=` Rules-class constants
- How veterancy points are awarded (kill-cost ratio)

Out-of-scope:
- How `VeteranCombat` enters the damage-side multiplier chain → [`damage_formula.md`](damage_formula.md) §9, [`fire_at_pipeline.md`](fire_at_pipeline.md)
- How `VeteranArmor` enters the receive-damage chain → [`receive_damage_pipeline.md`](receive_damage_pipeline.md)
- How vet/elite ROF behaves in the GetROF branch 4 → [`rof_burst_timing.md`](rof_burst_timing.md) §3 branch 4
- Self-healing veteran ability (`SELF_HEAL`) animation/tick logic → separate doc TBD

---

## 1. Three tiers

`Veterancy` is a `float` stored on every TechnoClass instance (offset TBD — see open
follow-up #1). The tier predicates (verified live 2026-05-17):

| Predicate | Function | Condition | Verified address |
|---|---|---|---|
| `IsRookie` | `VeterancyClass::IsNormalRookie 0x0074FFC0` | `0.0 <= v < 1.0` | constants: `FLOAT_007e1748 = 0.0f`, `_DAT_007e2ac8 = 1.0f` |
| `IsVeteran` | `VeterancyClass::IsVeteran 0x0074FF90` | `1.0 <= v < 2.0` | constants: `_DAT_007e2ac8 = 1.0f`, `_g_BridgeDiag_BothSides_2_0 = 2.0f` (Ghidra-mislabeled constant) |
| `IsElite` | `VeterancyClass::IsElite 0x00750010` | `2.0 <= v` | constant: `_g_BridgeDiag_BothSides_2_0 = 2.0f` |
| `Reset` | `VeterancyClass::Reset 0x00750080` | sets `*v = 0.0f` | — |

### Confidence

- **Content: HIGH** — all four predicates decompiled live; single-comparison bodies.
- **Identity: HIGH** — named functions in Ghidra annotation set.
- **Binding: HIGH** — predicates are called from every veterancy-gated branch (GetWeapon, GetROF, ReceiveDamage, Fire_At, etc.).

### Float constant identification note

The `2.0f` threshold is referenced via the global symbol `_g_BridgeDiag_BothSides_2_0`
because the same `2.0f` constant is reused by bridge-diagonal math. Ghidra's symbol
heuristic picked one name for both call sites. This is a known Ghidra quirk — both
veteran-tier comparisons and bridge diagonal scaling read the same float.

---

## 2. ElitePrimary / EliteSecondary weapon storage layout

### Parser location

`TechnoTypeClass::ReadINI` reads:
- `ElitePrimary=` (string at `0x008442DC`) via xref at `0x00712a32`
- `EliteSecondary=` (string at `0x008442CC`) via xref at `0x00712a5f`

### Memory layout

`TechnoTypeClass` stores weapon definitions as **two parallel arrays** of weapon slot
structures, each slot being **`0x1C` (28) bytes**:

| Array | Base offset | Slot stride | Slot 0 | Slot 1 | ... |
|---|---|---|---|---|---|
| Regular weapons (`Primary=`, `Secondary=`, `Weapon3=`, ...) | `type+0x898` | `0x1C` | `+0x898` | `+0x8B4` | ... |
| Elite weapons (`ElitePrimary=`, `EliteSecondary=`, `EliteWeapon3=`, ...) | `type+0xA94` | `0x1C` | `+0xA94` | `+0xAB0` | ... |

The slot structure contains the weapon pointer plus auxiliary fields (FLH offset,
turret index, etc.) — full slot layout TBD; cross-reference
[`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md).

### Helper functions (verified)

```
FUN_007177C0(type, idx) → int   // GetRegularWeaponSlotAddr
    return type + 0x898 + idx * 0x1C

FUN_007177E0(type, idx) → int   // GetEliteWeaponSlotAddr
    return type + 0xA94 + idx * 0x1C
```

Both are trivial pointer-arithmetic helpers — they return the **address of the slot**,
not the weapon pointer itself. The first int of the slot is the WeaponTypeClass
pointer.

### Confidence (storage)

- **Content: HIGH** (both helpers decompiled — single-instruction adds).
- **Identity: HIGH** — only `GetWeapon` calls these helpers; they're a matched pair.
- **Binding: HIGH** — `TechnoTypeClass::ReadINI` writes to these slots via `Weapon%d=` and `EliteWeapon%d=` INI loops (cross-reference to existing `TECHNOTYPECLASS_GHIDRA_REPORT.md`).

---

## 3. `TechnoClass::GetWeapon` — the swap mechanism

**`TechnoClass::GetWeapon`** at `0x0070E140` is the single function every combat path
calls to **resolve a weapon slot index (0..N) to a WeaponTypeClass pointer**. This is
where Elite weapon swap actually happens:

```c
WeaponSlot* TechnoClass::GetWeapon(this, idx) {
    if (idx == -1) return NULL;        // sentinel "no weapon"

    if (VeterancyClass::IsElite(this->Veterancy)) {
        TechnoType* type = this->GetTechnoType();   // vtable+0x84
        WeaponSlot* eliteSlot = (WeaponSlot*)(type + 0xA94 + idx * 0x1C);
        if (eliteSlot != NULL && eliteSlot->weapon != NULL) {
            return eliteSlot;          // elite has a weapon at this index — use it
        }
        // else fall through to regular
    }

    TechnoType* type = this->GetTechnoType();
    return (WeaponSlot*)(type + 0x898 + idx * 0x1C);
}
```

**Behavioral contract:**

- Only **Elite** tier triggers the swap. Veteran rank does **NOT** swap weapons — only damage/armor/speed multipliers apply.
- If `EliteWeapon[idx]` is unset (NULL pointer in the slot), Elite units fall back to `Weapon[idx]`. Mixed setups work — e.g. `ElitePrimary=BigCannon` with no `EliteSecondary=` keeps the regular secondary.
- The swap is dispatched at **every weapon lookup**, not at promotion time. There is no precomputed "active weapon" — `GetWeapon(0)` and `GetWeapon(1)` are re-resolved each call.

### Confidence (GetWeapon)

- **Content: HIGH** (decompiled live 2026-05-17, body matches above pseudocode).
- **Identity: HIGH** (named in Ghidra; called from `SelectWeaponAgainst`, `Fire_At`, `GetROF`, `GetFireError`, `InRange`).
- **Binding: HIGH** — every weapon-pointer lookup goes through this function. There is no alternate path to a weapon slot.

---

## 4. Rules-class veterancy constants (verified strings)

All five fields parsed in `RulesClass::ReadGeneral`:

| INI key | String address | Xref into ReadGeneral | Rules offset | Type | Default | Effect |
|---|---|---|---|---|---|---|
| `VeteranCap=` | `0x0083C97C` | (live) | `+0x???` | int? | 2 (max tier index) | clamps `Veterancy` |
| `VeteranArmor=` | `0x0083C994` | `0x0066EF3B` | `+0x???` | float | `1.5` | DIVIDES incoming damage on vet/elite w/ ARMOR ability |
| `VeteranSight=` | `0x0083C9A4` | (live) | `+0x???` | float | `1.0`-ish | sight-radius multiplier |
| `VeteranSpeed=` | `0x0083C9B4` | (live) | `+0x???` | float | `1.2` | move-speed multiplier |
| `VeteranCombat=` | `0x0083C9C4` | `0x0066EEC9` | `+0x???` | float | `1.1` | MULTIPLIES outgoing damage on vet/elite w/ FIREPOWER ability |
| `VeteranRatio=` | `0x0083C9D4` | (live) | `+0x???` | float | `2.0` (or `3.0` — varies by report) | kill-cost ratio to earn 1.0 veterancy point |

The xrefs to `VeteranCombat` and `VeteranArmor` strings confirm both keys are parsed
into `RulesClass`. Exact Rules offsets for each field were not fully extracted in this
pass (the ReadGeneral function is too large to decompile in one read) — see open
follow-up #2.

Defaults are conventional RA2/YR; the actual default reads come from the constructor's
initialization, which would need a separate decomp pass to verify.

### Confidence (Rules constants)

- **Content: HIGH** for the INI key strings (live xref).
- **Identity: MEDIUM** for the exact Rules offsets (one-shot read of full ReadGeneral failed; only 2 of 6 string xrefs to specific addresses verified).
- **Binding: HIGH** for the *existence* of these constants and their parser location (`RulesClass::ReadGeneral`).

---

## 5. How veterancy is awarded (overview)

When a unit kills a victim, it accrues **veterancy points** proportional to the
victim's cost. The formula is approximately:

```
points_earned = victim.Type.Cost / VeteranRatio / attacker.Type.Cost
attacker.Veterancy += points_earned
```

So killing a unit of equal cost gives `1 / VeteranRatio` veterancy points; killing
multiple equal-cost units accumulates linearly. With `VeteranRatio=2.0`, **2 equal-cost
kills** = 1.0 veterancy = Veteran tier; **4 equal-cost kills** = 2.0 = Elite tier.

Promotion happens when the float crosses a tier threshold — there is no
post-increment normalization. A unit at 1.99 that earns 0.05 jumps directly to 2.04
(Elite), and its weapon-pointer lookup immediately returns the Elite slot on next
fire.

The detailed scoring/award path lives in `TechnoClass::Killed` and
`TechnoClass::Award_Score` — covered separately (existing
[`../../RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md`](../../RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md)
touches on score attribution; the veterancy-add specifically is open follow-up #3).

---

## 6. Veteran/Elite ability flags

Ability arrays on `TechnoTypeClass`:

| Field | Offset (existing canonical doc) | INI key |
|---|---|---|
| `VeteranAbilities` | `+0x29C` (base of byte-flag array) | `VeteranAbilities=` (comma-separated names) |
| `EliteAbilities` | `+0x2AE` (base of byte-flag array) | `EliteAbilities=` |

Specific flags referenced in this iteration's veterancy-aware functions:

| Offset | Flag | Used by |
|---|---|---|
| `+0x29D` | Vet `FIREPOWER` | `Fire_At` (damage boost), `GetROF` branch 4 |
| `+0x29E` | Vet `ARMOR` | `ReceiveDamage` (damage divisor) |
| `+0x2AF` | Elite `FIREPOWER` | same |
| `+0x2B0` | Elite `ARMOR` (a.k.a. `DVARMOR`) | same |
| `+0x2B2` | Elite extra-armor flag (`STRONG_ARMOR`-class) | `GetROF` branch 4 vet/elite ROF gate (per `rof_burst_timing.md` §3) |

These offsets and identities are inherited from existing canonical docs
([`../../DAMAGE_MATH_GHIDRA_REPORT.md`](../../DAMAGE_MATH_GHIDRA_REPORT.md) §10) and have
not been independently re-verified in this iteration. **Cross-reference required**
before reimplementing. See open follow-up #4.

Full ability-name → offset map (FIREPOWER, ARMOR, SIGHT, SPEED, ROF, SELF_HEAL,
STRONGER, TUFF, FASTER, CRUSHER, EXTRA_FIREPOWER, DVARMOR, ...) is in
[`../../TECHNOTYPECLASS_GHIDRA_REPORT.md`](../../TECHNOTYPECLASS_GHIDRA_REPORT.md) (probable
location — verify).

---

## 7. The damage-side composition

This doc covers WHERE veterancy interacts with combat, not the math itself. Brief
recap from sibling docs:

### Outgoing damage (in Fire_At or pre-fire estimate)

```
damage = weapon.Damage
if IsVeteran(this.Veterancy) and type.VeteranAbilities has FIREPOWER:
    damage = ftol(damage * Rules.VeteranCombat)   // default *1.1
elif IsElite(this.Veterancy) and (type.EliteAbilities has FIREPOWER or EXTRA_FIREPOWER):
    damage = ftol(damage * Rules.VeteranCombat)   // same constant, same multiplier
```

Source: [`damage_formula.md`](damage_formula.md) §9 + [`fire_at_pipeline.md`](fire_at_pipeline.md) (when written).

### Incoming damage (in TechnoClass::ReceiveDamage)

```
if IsVeteran(this.Veterancy) and type.VeteranAbilities has ARMOR:
    *pDamage = ftol((float)*pDamage / Rules.VeteranArmor)   // default /1.5
elif IsElite(this.Veterancy) and (type.EliteAbilities has ARMOR or DVARMOR):
    *pDamage = ftol((float)*pDamage / Rules.VeteranArmor)
```

Source: [`damage_formula.md`](damage_formula.md) §11 step 1, [`receive_damage_pipeline.md`](receive_damage_pipeline.md) (when written).

### ROF (in GetROF branch 4)

Veteran/elite with FIREPOWER (or DVARMOR for elite) applies a multiplier to the ROF
return value. The exact multiplier identity (whether it's `VeteranCombat`, a separate
`VeteranROF`, or something else) is unresolved — see [`rof_burst_timing.md`](rof_burst_timing.md) §3
open follow-up.

---

## 8. Veterancy-Cap behavior

`Rules.VeteranCap` clamps the maximum veterancy value. In retail YR, the cap is `2`
(meaning Elite is the highest tier and a unit cannot exceed 2.0). Some mods set it to
3+ to enable additional tiers, but the engine treats anything `>= 2.0` as Elite —
there are no "Heroic" or "Mega-Elite" tiers wired into the predicates above.

The cap is applied at promotion time (when the veterancy float is updated). Open
follow-up: confirm whether the cap check uses `min(v, 2.0)` or `min(v, Rules.VeteranCap)`.

---

## 9. TS-legacy filter

- **All four tier predicates are LIVE in YR** — `Rookie`, `Veteran`, `Elite` are core gameplay tiers.
- **`VeterancyClass::Reset`** is LIVE — used by Iron Curtain expiry (unit resets to base), retaliate-fail, etc.
- **The GetWeapon Elite-swap branch** is LIVE — Elite Tanya pistol, Elite GI machine-gun, Elite Apocalypse cannon, etc., all use ElitePrimary/EliteSecondary.
- **The `VeteranAbilities=` flag system** as a whole is LIVE; specific flag NAMES (`STRONGER`, `TUFF`, `FASTER` etc.) are RA2-era and remain in YR.
- No TS-only dead branches identified in the verified pieces.

---

## 10. Edge cases

| Case | Behavior |
|---|---|
| `Veterancy == 0.99999...` (just below 1.0) | `IsRookie` true; no abilities or weapon swap. Floating-point precision matters at the exact threshold. |
| `Veterancy == 1.0` exactly | `IsVeteran` true (`>=` comparator). |
| `Veterancy == 1.99999...` | `IsVeteran` true (`< 2.0`). |
| `Veterancy == 2.0` exactly | `IsElite` true; `IsVeteran` false. |
| Elite unit with no `EliteWeapon[i]=` defined | Falls through to regular `Weapon[i]=`. Damage / armor multipliers still apply (FIREPOWER / DVARMOR check). |
| Veteran unit | NEVER swaps weapon. Damage / armor / speed multipliers only. |
| Unit promoted mid-burst | Next `GetROF` and `GetWeapon` call returns elite values. If burst was already mid-cycle, the remaining shots fire with the elite weapon. CurrentBurstIndex is not reset. |
| Iron Curtain on a promoted unit | When IC ends, `VeterancyClass::Reset` is called → veterancy = 0.0. Unit reverts to Rookie tier instantly. Confirm: is this actually how IC ends, or does IC just expire normally? Open follow-up #5. |
| Killing crates/civilians | Their cost is 0 or trivial; veterancy gain is negligible. |

---

## 11. Open follow-ups

1. **Veterancy field offset on TechnoClass instance.** The predicates take `float*` — we need the byte offset within TechnoClass where Veterancy is stored. The existing canonical doc says it's at "some offset" but doesn't name it. Priority: HIGH — needed for any veterancy-aware reimplementation.
2. **Exact Rules offsets** for `VeteranCap` / `VeteranArmor` / `VeteranSight` / `VeteranSpeed` / `VeteranCombat` / `VeteranRatio`. The `RulesClass::ReadGeneral` decompilation exceeds the tool's read window; needs a targeted re-decomp focused on these six writes. Priority: MEDIUM (numerical defaults are conventional, but byte offsets affect impl).
3. **Veterancy-award code path.** Trace from `TechnoClass::Killed` → `Award_Score` (or similar) — where exactly is `attacker.Veterancy += victim.Cost / Ratio / attacker.Cost` computed, and is the formula exactly that or a variant? Priority: HIGH (parity-critical — exact promotion curve must match).
4. **VeteranAbilities / EliteAbilities ability-name → byte-offset table.** Verify `FIREPOWER`, `ARMOR`, `SPEED`, `SIGHT`, `ROF`, `SELF_HEAL`, `STRONGER`, `TUFF`, `FASTER`, `CRUSHER`, `EXTRA_FIREPOWER`, `DVARMOR` map to consecutive bytes at `+0x29D..+0x2AB` (vet) and `+0x2AF..+0x2BD` (elite). Priority: MEDIUM.
5. **Iron Curtain expiry: does it reset veterancy?** Trace `VeterancyClass::Reset` callers to see whether IC end is one. Priority: LOW (probably yes but easy to verify).
6. **`VeteranSpeed` consumer.** Where is the move-speed multiplier applied? Probably `LocomotionClass::Get_Speed_To_Cell` or similar. Priority: LOW (out of scope for combat docs).
7. **Self-healing veteran ability animation.** SELF_HEAL grants per-tick health regen; tick rate and amount are RulesClass constants. Trace the tick handler. Priority: LOW (separate doc).
8. **Building veteran promotion.** Buildings can also have `Veterancy` — but no `ElitePrimary=` retail use is known. Confirm whether the GetWeapon swap activates for buildings. Priority: LOW.
9. **Aircraft veteran behavior.** Same as buildings — confirm veteran applies. Priority: LOW.
10. **Sub-passenger of an open-topped transport.** If passenger is Elite but transport is Rookie, whose veterancy gates the GetWeapon swap? Probably the passenger's, since SelectWeaponAgainst's Phase D returns `type.field_0xD50` (a type-level override), not a per-instance lookup. Verify. Priority: LOW.

---

## 12. Sources

- Live decompilations (2026-05-17):
  - `TechnoClass::GetWeapon @ 0x0070E140`
  - `FUN_007177C0 @ 0x007177C0` (regular weapon slot helper)
  - `FUN_007177E0 @ 0x007177E0` (elite weapon slot helper)
  - `VeterancyClass::IsVeteran @ 0x0074FF90`
  - `VeterancyClass::IsElite @ 0x00750010`
  - `VeterancyClass::IsNormalRookie @ 0x0074FFC0`
- String xrefs (2026-05-17):
  - `"ElitePrimary"` at `0x008442DC` → `TechnoTypeClass::ReadINI` at `0x00712a32`
  - `"EliteSecondary"` at `0x008442CC` → `TechnoTypeClass::ReadINI` at `0x00712a5f`
  - `"VeteranArmor"` at `0x0083C994` → `RulesClass::ReadGeneral` at `0x0066EF3B`
  - `"VeteranCombat"` at `0x0083C9C4` → `RulesClass::ReadGeneral` at `0x0066EEC9`
  - Plus three other Veteran* strings (`VeteranCap`, `VeteranSight`, `VeteranSpeed`, `VeteranRatio`) — xrefs not pulled this pass.
- Existing canonical docs cross-referenced:
  - [`../../DAMAGE_MATH_GHIDRA_REPORT.md`](../../DAMAGE_MATH_GHIDRA_REPORT.md) §10 (veteran threshold constants, ability-flag offsets).
  - [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md) (weapon slot layout).
  - [`../../TECHNOTYPECLASS_GHIDRA_REPORT.md`](../../TECHNOTYPECLASS_GHIDRA_REPORT.md) (probable home of full ability-name table).
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`receive_damage_pipeline.md`](receive_damage_pipeline.md), [`rof_burst_timing.md`](rof_burst_timing.md), [`anti_air_dispatch.md`](anti_air_dispatch.md), [`fire_at_pipeline.md`](fire_at_pipeline.md).
