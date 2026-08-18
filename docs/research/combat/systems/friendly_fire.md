# Friendly Fire — `AffectsAllies` + Friendly Damage Gates

This doc is the canonical reference for **friendly-fire damage gating** in gamemd.exe:

- The `AffectsAllies=` warhead flag (`wh+0x179`) — what it gates, where it gates
- The single concrete check site (`TechnoClass::ReceiveDamage`)
- Splash composition (per-target dispatch, no separate splash filter)
- ForceFire interaction
- Whether AI vs player ever differ
- The Psychedelic / Psionic separate gate that uses ally-status independently

Out-of-scope:
- The general damage transform → [`damage_formula.md`](damage_formula.md)
- The AoE dispatcher itself → [`splash_cellspread.md`](splash_cellspread.md)
- General target-eligibility gates → [`can_target_gates.md`](can_target_gates.md)
- Self-target gate (different from ally gate) → [`splash_cellspread.md`](splash_cellspread.md) §8

---

## 1. The flag

| Field | Value |
|---|---|
| Offset | `wh+0x179` |
| Type | `bool` |
| INI key | `AffectsAllies=` |
| Default | `false` (warhead does NOT affect allies) |
| Parser | `WarheadTypeClass::ReadINI` at `0x0075DD80`, key string `"AffectsAllies"` at `0x00847CC8` (xref into ReadINI at `0x0075d9df`, verified live 2026-05-17) |

### Default meaning

If `AffectsAllies` is unset in INI, the warhead defaults to `false` — meaning **damage
is blocked when attacker and target are allied**. This is the default for virtually
every shipping warhead (Tanya pistol, GI gun, tank cannons, etc.). Conversely,
warheads with `AffectsAllies=yes` damage allies normally — used by:

- `[NUKE]` (the nuke superweapon)
- `[CometWH]` (Lightning Storm)
- `[DominatorWH]` (Psychic Dominator)
- Demolitions / chain reactions
- A few specific weapons that need to nuke friendly units (e.g., Crazy Ivan bombs that may be placed on allied buildings)

### Confidence

- **Content: HIGH** — the offset `+0x179` matches the existing
  [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md) and
  the gate code in `TechnoClass::ReceiveDamage` reads `*(char*)(wh + 0x179)`.
- **Identity: HIGH** — single INI key string with a single xref into the ReadINI parser.
- **Binding: HIGH** — the flag is read by every per-target damage application, gating
  the friendly-fire reject.

---

## 2. The single gate site (verified live)

`TechnoClass::ReceiveDamage` at `0x00701900` contains exactly **one** gate that reads
`AffectsAllies`:

```c
// Inside TechnoClass::ReceiveDamage, after the immunity checks
// in_stack_0000000c = warhead pointer
// in_stack_00000010 = sourceHouse pointer (HouseClass*)
// this->Owner       = target's HouseClass*

if (warhead->byte+0x179 == 0                                  // AffectsAllies = false
   && sourceHouse != NULL                                      // there IS a firer
   && HouseClass__IsAlliedWith(
        sourceHouse->Owner   /* +0x87 = +0x21C / 4 */,
        this->Owner)) {                                        // attacker and target allied
    *pDamage = 0;
    return 0;     // damage nullified
}
```

The comparison is `HouseClass::IsAlliedWith(attacker.Owner, target.Owner)`. The
function returns true when both houses are in the same alliance. Self (same house) is
always considered allied.

### Confidence

- **Content: HIGH** — verified live 2026-05-17 in the ReceiveDamage decomp.
- **Identity: HIGH** — single check, single field read.
- **Binding: HIGH** — `TechnoClass::ReceiveDamage` is the per-target damage entry for every Techno (`vtable+0x16C` in the standard combat pipeline).

---

## 3. Where this gate fires in the pipeline

```
[Fire_At]
   → projectile launched
        │
        ▼
[Apply_area_damage] (splash)
   for each target in CellSpread radius:
     target->ReceiveDamage(damage, distance, wh, attacker, false, false, sourceHouse)
                  │
                  ▼
[TechnoClass::ReceiveDamage]                ◄── AffectsAllies gate HERE
   apply country armor mult
   apply veterancy armor div
   immunity gates (Radiation / Psionic / Poison)
   ★ AffectsAllies gate
   psychedelic special path
        │
        ▼
[ObjectClass::ReceiveDamage]
   apply Verses + CellSpread falloff via GetDamage
   apply MaxDamage clamp
   subtract Health, kill check
```

**Key fact:** the AffectsAllies gate is in the **receiver-side** of the damage
pipeline, not the firer-side. Consequences:

1. The weapon **fires normally** at a friendly target. Projectile leaves the muzzle, flies, detonates. The animation plays. Only the damage is zeroed.
2. **Splash damage**: every target in the CellSpread radius gets its own ReceiveDamage call with the firer's `sourceHouse`. Friendly units in the splash radius take 0 damage; enemy units take normal damage. There is **no separate splash filter** that pre-removes friendly targets — every target is iterated, every target's AffectsAllies is checked per ReceiveDamage call.
3. **Sound / visual effects** still play for friendly impacts. Hit-flash animation, impact sound, dust cloud — all visible. Only the HP doesn't change.

---

## 4. Splash composition (verified)

In `Apply_area_damage` (`0x00489280`, documented in [`splash_cellspread.md`](splash_cellspread.md)),
the target collection loop does **NOT** pre-filter friendlies. Every target in range
is appended to the damage_vector. The damage dispatch loop then calls each target's
`ReceiveDamage` with the firer's `sourceHouse`. AffectsAllies is checked on each call.

So a nuke (`AffectsAllies=yes` on `[NUKE]`) hits friendlies in radius. A tank shell
(`AffectsAllies=no` on `[AP]`) splashes friendlies for 0 damage. This is the
fundamental observable behavior.

### Building cell-effects (NOT subject to AffectsAllies)

The cell-side warhead effects in `Apply_area_damage` — tiberium reduction, overlay
destruction, bridge destruction, IC-barrel chain — are **NOT** ally-gated. A friendly
tank shell shrub-deletes overlays in the radius regardless of `AffectsAllies`. Per
[`splash_cellspread.md`](splash_cellspread.md) §6, those effects are gated only by
the warhead's `Wall=` / `Tiberium=` / `WallAbsoluteDestroyer=` flags.

---

## 5. ForceFire interaction

ForceFire (player Ctrl-click) **does NOT bypass** the AffectsAllies gate in
ReceiveDamage. The player can:

- Target a friendly unit (the GetFireError target-eligibility gates allow it under ForceFire — see [`can_target_gates.md`](can_target_gates.md) §8).
- The weapon fires the projectile.
- On impact, if `AffectsAllies=no`, ReceiveDamage zeros the damage.

So ForceFire-shooting your own ally with an AP cannon produces flash and sound but no
damage. This is the long-standing RA2 behavior — the player can "ForceFire" to
suppress fire (e.g., make a Prism Tower waste its cooldown) without harming the ally.

The exception is warheads with `AffectsAllies=yes` — those DO damage friendlies under
ForceFire (and even under auto-fire, but auto-target scans normally don't pick
allies). NUKE, Lightning Storm strikes, Psychic Dominator, Crazy Ivan placement —
all bypass friendly protection.

### Confidence

- **Content: HIGH** — ReceiveDamage decomp shows no ForceFire-bypass branch around the AffectsAllies check.
- **Identity / Binding: HIGH** — the check is unconditional on the `forceFire` arg (which doesn't reach this gate as a parameter).

---

## 6. AI vs player asymmetry — none at this gate

The AffectsAllies gate is symmetric. It runs the same regardless of whether the
firer is human-player-controlled or AI-controlled. The only place AI behavior differs
is **upstream**:

- AI auto-target scan refuses to pick allied targets (filter at scan time).
- Player auto-target on the move-or-attack command also refuses allied targets.
- Player ForceFire DOES allow allied targeting (but damage is zeroed at impact for `AffectsAllies=no` warheads).
- AI does NOT have a "ForceFire" path — the AI never voluntarily fires at an ally.

The asymmetry is in **target selection**, not in **damage application**. Once a weapon
fires at a target, both AI and player follow the same `ReceiveDamage` pipeline.

---

## 7. Psychedelic / Psionic — separate ally gate

There's a **second** ally gate in `TechnoClass::ReceiveDamage` that uses the
Psychedelic flag (`wh+0x16D`):

```c
if (warhead->Psychedelic != 0) {
    if (HouseClass::IsAlliedWith(this->Owner, sourceHouse_alt)) {
        return 0;          // Psychedelic warhead can't affect allied units
    }
    // ... continue Psychedelic processing
}
```

This is distinct from AffectsAllies — even if a Psychedelic warhead has
`AffectsAllies=yes`, allied units are STILL not affected by the psychedelic mode.
(They DO take normal damage from the warhead's base damage value, just not the
psychedelic warp effect.)

The two flags layer:
- `AffectsAllies=no`: ally damage = 0 entirely. Both base damage and psychedelic.
- `AffectsAllies=yes, Psychedelic=yes`: ally takes base damage but no psychedelic warp.
- `AffectsAllies=yes, Psychedelic=no`: ally takes base damage normally.

This Psychedelic-side gate is the gate at `wh+0x16D` referenced in
[`damage_formula.md`](damage_formula.md) §11 step 8. See that doc for the full
Psychedelic path.

---

## 8. Other ally-using gates (NOT same as AffectsAllies)

Several other gates use `HouseClass::IsAlliedWith` for **different** purposes than
AffectsAllies. Listed here so they're not confused:

| Site | Field tested | What it does |
|---|---|---|
| `GetFireError` gate #59 (`can_target_gates.md`) | `Verses[target.Armor] == 0` | Engine-side weapon-vs-armor block. Not ally-related. |
| `SelectWeaponAgainst` Phase H | `weapon.NavalGunboat & target.NavalTarget & !IsAllied` | Naval-gunboat swap to secondary only fires against ENEMY naval. Allied naval is ignored. |
| `SelectWeaponAgainst` Phase K | `target = allied tech building` | ElectricAssault primary swaps if target is allied tech building (intent: don't damage allied buildings with EA primary). |
| `Apply_area_damage` IsSelfHealing | `attacker.Type.IsSelfHealing` | Allows self-targeting; not ally-related. |
| Psychic Dominator MindControlArea | various | Mass MC, applies AffectsAllies internally. |
| Mind-control captures (CaptureManagerClass) | various | Captures from enemy only; allied units are not "captured." |
| AI threat-acquire scan | various | AI scanning logic filters allies pre-fire. |
| `TechnoClass::ShouldRetaliate` | various | After taking damage, decide whether to fire back; checks the attacker's house. |

These all use ally-status but for different decision points. **Only the one gate
in §2 is the friendly-fire damage filter.**

---

## 9. Edge cases

| Case | Behavior |
|---|---|
| Self (same unit) | `IsAlliedWith(self, self) == true`. So a unit can't hurt itself unless `AffectsAllies=yes`. The C4Warhead self-target path in `Apply_area_damage` (`Rules+0xFAC`) bypasses both the C4-self-target gate AND has `AffectsAllies=yes` — barrel explosions damage their owner. |
| Same house, different units | Allied → damage zero unless AffectsAllies=yes. |
| Same alliance, different houses (player + ally) | Allied → damage zero unless AffectsAllies=yes. |
| Unaligned attacker (sourceHouse == NULL) | The gate has `sourceHouse != NULL` check — if NULL, the gate is **skipped** and damage applies. This is the "ambient damage" / "death weapon" / "scripted trigger" path. Ambient sources damage everyone. |
| ForceFire on ally with non-AffectsAllies | Damage = 0. Animation plays. Sound plays. |
| Splash damage on mixed allied + enemy units | Allies take 0, enemies take falloff-adjusted damage. Both go through individual ReceiveDamage. |
| NUKE on ally | `[NUKE]` has `AffectsAllies=yes` → ally takes full nuke damage. |
| Crazy Ivan bomb placed on ally building, then detonated | `[IvanBomb]` warhead has `AffectsAllies=yes` (so the bomb goes off on the ally building it was placed on). Verify in rulesmd.ini if exact behavior matters. |
| Mind-Control on ally | Different gate (Psychedelic / MindControl). Allies can't be mind-controlled regardless of AffectsAllies. |

---

## 10. TS-legacy filter

- `AffectsAllies` is fully LIVE in YR. Most damage events route through this gate.
- The separate Psychedelic ally check (`+0x16D`) is also LIVE — Yuri's psy weapons rely on it.
- No TS-only branch involved.

---

## 11. Open follow-ups

1. **`HouseClass.Owner` offset.** The decomp references `sourceHouse[0x87]` (= offset `+0x21C`). Verify this is `HouseClass.Owner` (the HouseClass that owns this HouseClass — i.e., the player slot) vs other HouseClass fields. Most likely the parent-house pointer. Priority: LOW (the check works as long as `IsAlliedWith` is fed two valid house pointers).
2. **Ambient damage path with sourceHouse=NULL.** Confirmed in the gate that ambient (no-firer) damage bypasses the ally check. Trace which callers actually pass `sourceHouse=NULL` — death-weapon, tiberium-tick damage, fire-aura, AnimDamage. Priority: MEDIUM.
3. **`[IvanBomb]` warhead `AffectsAllies` value.** Verify from rulesmd.ini quote. The behavior of placing an Ivan bomb on an ally building is a common edge case. Priority: LOW.
4. **`IsAlliedWith` decomp.** Quickly verify the function returns true for same-house, and which alliance vector it checks. Priority: LOW.
5. **Why does the early-out also clear `*pDamage = 0` before return?** The function returns 0 on ally rejection AND zeros the damage pointer. This is the contract — callers (Apply_area_damage's loop) check the post-call damage value. Priority: LOW.

---

## 12. Sources

- Live decompilation of `TechnoClass::ReceiveDamage` at `0x00701900` (2026-05-17).
- Live xref of `"AffectsAllies"` string at `0x00847CC8` → single DATA xref into `WarheadTypeClass::ReadINI` at `0x0075d9df`.
- Existing canonical doc: [`../../DAMAGE_MATH_GHIDRA_REPORT.md`](../../DAMAGE_MATH_GHIDRA_REPORT.md) §4 step 7 (the AffectsAllies gate, plus the surrounding immunity gates).
- Existing canonical doc: [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md) (offset `+0x179`).
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`receive_damage_pipeline.md`](receive_damage_pipeline.md), [`splash_cellspread.md`](splash_cellspread.md), [`can_target_gates.md`](can_target_gates.md), [`anti_air_dispatch.md`](anti_air_dispatch.md).
