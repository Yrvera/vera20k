# Sonic — Sonic Tank / Wave Effect

This doc is the canonical reference for the **sonic weapon system** in gamemd.exe:
the wave-visual + damage delivery path used by the Sonic Tank (`[SREF]`) and its
Elite variant.

There are TWO interleaved mechanics:
1. **`IsSonic=yes`** weapon flag (`weapon+0x130`) — triggers the `WaveClass` visual
   effect (sprite-quad sonic wave drawn from firer to target).
2. **`AmbientDamage=N`** combined with normal `Damage=N` (per [`ambient_damage.md`](ambient_damage.md))
   — the Sonic Tank delivers BOTH per-shot bullet damage AND ambient path damage.

Crucially: **WaveClass does NOT apply damage** — it is purely visual. Damage flows
through the standard BulletClass path with the `Sonic` projectile and `SonicWarhead`.

Out-of-scope:
- WaveClass internal struct + AI lifecycle (rendering details) → [`../../WAVECLASS_GHIDRA_REPORT.md`](../../WAVECLASS_GHIDRA_REPORT.md) + [`../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md`](../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md)
- Magnetron `IsMagBeam=yes` (different WaveClass type-3 variant) → [`locomotor_warhead.md`](locomotor_warhead.md)
- AmbientDamage consumer trace → [`ambient_damage.md`](ambient_damage.md) (still HIGH-priority open follow-up)
- Damage transform → [`damage_formula.md`](damage_formula.md)

---

## 1. Correction to existing canonical doc

The [`../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md`](../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md) §3 claims:

> ⚠ CRITICAL: IsSonic is TS-LEGACY DEAD CODE IN YR
> Grep over `ini/rulesmd.ini` + `ini/rules.ini` for `IsSonic\s*=\s*(yes|true)`: zero matches across both files.

**This is incorrect.** The addendum's grep used lowercase `yes`. Retail rulesmd.ini
has `IsSonic=Yes` (capitalized `Y`) on two weapons:

```ini
; rulesmd.ini line 23677-23691
; Sonic Zap
[SonicZap]              ; Sonic Tank primary
Damage=4
AmbientDamage=10
ROF=120
Range=6
Projectile=Sonic
Speed=100
Warhead=SonicWarhead
Report=DolphinAttack
IsSonic=Yes              ← LIVE
DecloakToFire=no

; rulesmd.ini line 25097-25110
[SonicZapE]             ; Sonic Tank Elite
Damage=8
AmbientDamage=15
ROF=80
Range=6
Projectile=Sonic
Speed=100
Warhead=SonicWarhead
Report=DolphinAttack
IsSonic=Yes              ← LIVE
Burst=2
DecloakToFire=no
```

The INI parser is case-insensitive (`CCINIClass::ReadBool` accepts `yes`/`Yes`/`YES`/`true`/`True`/`1`), so both these `IsSonic=Yes` declarations are parsed. The corresponding `weapon+0x130` flag IS set, and the WaveClass type-0 path IS invoked when these weapons fire.

**Conclusion:** WaveClass type 0 IS LIVE in YR. The addendum's "TS-legacy dead code" claim is wrong. Open follow-up #1 to fix the canonical doc.

---

## 2. Flag layout (verified)

### WeaponTypeClass

| Offset | INI key | String addr | Effect |
|---|---|---|---|
| `weapon+0x130` | `IsSonic=` | (per `WeaponTypeClass::ReadINI 0x00772080` parse loop) | Triggers WaveClass type-0 construction in Fire_At; sticky-beam particle gate (GetROF Branch 2 + GetFireError Phase D — though no particle field for IsSonic specifically) |
| `weapon+0x15C` | `IsMagBeam=` | (same parser) | Triggers WaveClass type-3 — Magnetron beam. Out of scope here. |

### BulletTypeClass (for the projectile)

The `[Sonic]` projectile is referenced by `Projectile=Sonic` on both SonicZap weapons.
Its exact field layout is documented separately (see projectile docs to be written —
`projectiles/Sonic.md`).

### Confidence

- **Content: HIGH** — `weapon+0x130 = IsSonic` per existing canonical addendum (`WeaponTypeClass::ReadINI` full decompile).
- **Identity: HIGH** — single INI key, single parser.
- **Binding: HIGH** — Fire_At callsite at `0x006FF460` (the AddOn report's "Type 0 callsite") gates on this field.

---

## 3. The two parallel mechanisms

When a Sonic Tank fires its `[SonicZap]` weapon:

```
[Fire_At]
  ├─ create BulletClass(weapon=SonicZap, projectile=Sonic, target=...)
  │     → bullet flies, eventually detonates
  │     → SonicWarhead applied at impact
  │     → standard damage (Damage=4) delivered to target via Apply_area_damage
  │
  ├─ AmbientDamage=10 dispatched along the firer-target line
  │     → consumer site UNTRACED (open follow-up from ambient_damage.md)
  │
  └─ IsSonic=yes triggers WaveClass type 0 construction
        → visual-only sprite-quad wave drawn from firer to target
        → WaveClass::AI ticks per-frame for fade-in / track / fade-out
        → no damage applied by WaveClass
```

So the player sees a visual wave AND takes damage. The damage comes from the bullet
path + AmbientDamage path; the visual comes from WaveClass.

---

## 4. WaveClass type 0 — the visual

Allocated via `operator_new(0x240)` and constructed by `WaveClass__Constructor`
at `0x0075E950`. Pure visual class — no damage application.

### Construction (verified from existing canonical doc)

```c
// In TechnoClass::Fire_At at 0x006FF460 (Sonic gate):
if (weapon->IsSonic (+0x130) != 0):
    newWave = WaveClass::Constructor(
        operator_new(0x240),
        &fireSrcCoord,
        &targetCoord,
        firingTechno,
        0,                    // WaveType = 0 (Sonic)
        target
    )
    firingTechno->CurrentWave (+0x324) = newWave
```

The wave self-registers in the global wave list at `DAT_00A8EC3C` and is iterated
per-tick by the world update.

### Per-tick AI (verified from addendum)

`WaveClass::AI 0x00762AF0`:
1. Recompute owner/target coords (so the beam tracks if either moves).
2. Phase 1: deactivation tests (target gone, owner aim changed, owner moved too far).
3. Phase 2: geometry recompute via `FUN_00761640` (type 0 geometry helper).
4. Phase 3: fade-in / fade-out animation; remove via `vtable[0xF8]` when fade complete.

**No damage application** — the wave is purely visual.

### `firer+0x324 CurrentWave` slot

Per existing canonical addendum §5: the firing TechnoClass stores its active wave at
`+0x324`. This is a single-wave-per-firer slot — Sonic Tank can have only one wave
visual at a time. Subsequent fires before the wave completes would overwrite this
slot. (For Sonic Tank with ROF=120, the wave fades long before the next shot, so this
isn't normally observable.)

### Confidence

- **Content: HIGH** — existing addendum decompiled both Fire_At callsite and `WaveClass::AI`.
- **Identity: HIGH** — single Fire_At callsite, single AI function.
- **Binding: HIGH** — triggered every Sonic Tank shot.

---

## 5. The damage path

For Sonic Tank, damage is delivered through TWO parallel paths:

### Path 1: Standard projectile + warhead

The `[Sonic]` projectile carries `Damage=4` (or `Damage=8` for Elite) and detonates
via `SonicWarhead`. Standard Apply_area_damage flow at impact.

```ini
[SonicWarhead]            ; (referenced but content not extracted in this pass)
; Verses, CellSpread, AnimList all per warhead INI
```

### Path 2: AmbientDamage along path

`AmbientDamage=10` (or `15` Elite) is applied along the firer-target line. Per
[`ambient_damage.md`](ambient_damage.md), this is the per-cell / per-unit damage
dispatch that runs in Fire_At alongside the bullet creation. **Consumer site
STILL UNTRACED** as of this iteration.

Working hypothesis from ambient_damage.md §3:
```c
if (weapon->AmbientDamage > 0):
    for cell in path(firer.coords → target.coords):
        for techno in cell.occupants:
            techno->ReceiveDamage(weapon.AmbientDamage, 0, weapon.Warhead, ...)
```

### Total damage to target hit head-on

Per shot: bullet damage (4 or 8) + ambient damage (10 or 15) = 14 (regular) or 23 (elite).
Plus, for the Elite variant, `Burst=2` doubles the per-fire-trigger damage.

### Confidence (damage path)

- **Content: HIGH for the standard bullet path** (Apply_area_damage flow).
- **Content: LOW for the AmbientDamage path** — consumer hypothesis unverified.

---

## 6. Visual recap

| Component | Class | Live in YR | Owner |
|---|---|---|---|
| Wave visual (sprite-quad) | `WaveClass` type 0 | **LIVE** for Sonic Tank | Sonic Tank `[SREF]` |
| Wave visual (sprite-quad) | `WaveClass` type 3 | **LIVE** for Magnetron | Yuri Magnetron `[GMAGN]` |
| Wave visual (sprite-quad) | `WaveClass` type 1 / 2 | **DORMANT** — no live callsite | (none) |
| Bullet visual | `BulletClass` with `Projectile=Sonic` | **LIVE** — standard projectile path | Sonic Tank |
| Damage carrier | `BulletClass + SonicWarhead` | **LIVE** | Sonic Tank, Sonic Tank Elite |
| Damage carrier | AmbientDamage along path | **LIVE in INI** but consumer UNTRACED | Sonic Tank, Sonic Tank Elite |

---

## 7. Key offsets summary

| Symbol | Offset / Address |
|---|---|
| `weapon.IsSonic` | `weapon+0x130` |
| `weapon.IsMagBeam` | `weapon+0x15C` |
| `weapon.AmbientDamage` | `weapon+0x98` |
| `firer.CurrentWave` | `TechnoClass+0x324` |
| WaveClass size | `0x240` bytes (576) |
| `WaveClass::Constructor` (full) | `0x0075E950` |
| `WaveClass::Constructor` (default / load) | `0x0075EBE0` |
| `WaveClass::AI` | `0x00762AF0` |
| `FUN_00761640` | type 0/1/2 geometry helper |
| `FUN_00762070` | type 3 geometry helper |
| `WeaponTypeClass::ReadINI` IsSonic parse | within `0x00772080` body |
| Fire_At type-0 callsite | `0x006FF460` |
| Fire_At type-3 callsite | `0x006FF5F5` |
| Global wave list | `DAT_00A8EC3C` (data ptr), `DAT_00A8EC48` (count) |

---

## 8. TS-legacy filter

| Component | Status in YR |
|---|---|
| `IsSonic=yes` weapon flag (`+0x130`) | **LIVE** — `[SonicZap]` and `[SonicZapE]` (Sonic Tank weapons) set it as `IsSonic=Yes` |
| `IsMagBeam=yes` weapon flag (`+0x15C`) | **LIVE** — Magnetron `[GMAGN]` weapon |
| `WaveClass` C++ class | **LIVE** — instantiated by both Sonic Tank fires (type 0) and Magnetron fires (type 3) |
| `WaveClass::AI` per-tick | **LIVE** — tick handler decompiled in existing addendum |
| WaveType 1 (per addendum) | **DORMANT** — no Fire_At callsite |
| WaveType 2 (per addendum) | **DORMANT** — no Fire_At callsite, but LUT slots exist |
| Sonic Tank `[SREF]` | **LIVE** (Allied unit in YR, descended from Mirage Tank tech) |

The addendum's claim that IsSonic is TS-legacy dead is **incorrect** — see §1.

---

## 9. Composition

| Scenario | Behavior |
|---|---|
| Sonic Tank fires at vehicle target | Bullet (4 dmg) + AmbientDamage (10) along path + Wave visual. Total ~14 damage per shot, modulated by Verses. |
| Sonic Tank fires at infantry | Same. Bullet impact applies SonicWarhead AnimList (likely an EXPLOSML-style anim). |
| Two Sonic Tanks fire at same target | Each fires its own wave, its own bullet, its own AmbientDamage. Both waves visible simultaneously (different firer.CurrentWave slots). Total damage = 2 × (bullet + ambient). |
| Sonic Tank fires at a Cell (force-fire on ground) | Wave visual still spawns from Sonic Tank to cell. Bullet detonates at cell. Ambient damage may apply along path. Cell-only fire path may have edge cases (verify). |
| Sonic Tank under attack while firing | If killed mid-fire, the wave continues its fade animation but `OwnerLink` becomes invalid — AI detects this and triggers fade-out (per addendum §4 Phase 1). |
| Elite Sonic Tank's Burst=2 | Two fire triggers (3-5 ticks apart per Burst mechanics). Each spawns a wave (with the previous wave's `CurrentWave` slot probably overwritten or the new one rejected — verify). |
| Sonic Tank Elite | `Damage=8, AmbientDamage=15` — roughly double regular. With Burst=2, dramatically more per ROF cycle. |

---

## 10. Edge cases

| Case | Behavior |
|---|---|
| Mod sets `IsSonic=yes` on a non-Sonic weapon | Wave visual spawns; if `Damage=0` and `AmbientDamage=0`, only the visual fires (no damage). Useful for cosmetic "Sonic-style" effects. |
| Mod sets `IsSonic=yes` with `Damage=100` | Standard bullet damage (100) + visible wave. AmbientDamage path doesn't fire (no AmbientDamage value). |
| Mod removes `IsSonic=Yes` from `[SonicZap]` | Sonic Tank shots produce no wave visual. Damage still flows via bullet + ambient. (Note: visual-only — gameplay unaffected.) |
| `IsSonic=` value parsed as `Yes`/`yes`/`YES`/`true`/`1` | All accepted by case-insensitive `CCINIClass::ReadBool`. The addendum's case-sensitive grep led to a false TS-legacy claim. |
| `firer.CurrentWave` already set when re-firing | Per addendum: type-3 (Magnetron) path explicitly checks this and skips. Type-0 (Sonic) path may overwrite — verify behavior with Sonic Tank Elite's Burst=2. |

---

## 11. Open follow-ups

1. **Update existing canonical addendum.** The "IsSonic is TS-legacy dead" claim in [`../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md`](../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md) §3 is incorrect (case-sensitive grep error). Priority: MEDIUM — fix the canonical doc.
2. **AmbientDamage consumer for Sonic Tank.** Same open follow-up as ambient_damage.md #1 — where exactly is the per-cell/path AmbientDamage applied? Specifically for Sonic Tank: is it inside Fire_At, inside Apply_area_damage, or in a SonicWarhead-special path? Priority: HIGH.
3. **CurrentWave slot overwrite for Sonic Tank Elite Burst=2.** Does the second burst-shot create a new wave (overwriting `firer+0x324`) or get rejected? Priority: LOW (visually-only).
4. **`[SonicWarhead]` content survey.** Document the SonicWarhead's INI keys, Verses, CellSpread, AnimList. Priority: MEDIUM — needed for full Sonic Tank parity.
5. **`[Sonic]` projectile content survey.** Document the Sonic BulletType's INI keys, especially whether `Inviso=yes` or special flight behavior. Per existing ambient_damage.md, Sonic Tank's `Damage=4` is an impact damage on the target — meaning the projectile DOES fly and detonate at impact (not Inviso). Priority: MEDIUM.
6. **Sonic Tank vs Dolphin damage comparison.** Dolphin uses `[DolphinPulse]` (no IsSonic flag, no AmbientDamage in my survey). Dolphin's mechanic is regular projectile-style. Sonic Tank's combined bullet + ambient produces higher effective DPS. Priority: LOW.
7. **WaveType 1 and 2 dormancy verification.** Confirmed by addendum no Fire_At callsite; could a mod or special caller exist? Priority: LOW.
8. **`+0x130` IsSonic = WaveClass-type-0 gate verification.** Cross-check by tracing the byte at `weapon+0x130` to its ReadBool parser call — the addendum says this is IsSonic. Priority: LOW (high cross-confidence already).

---

## 12. Sources

- Live INI quote from `ini/rulesmd.ini` lines 23677-23691 (`[SonicZap]` with `IsSonic=Yes`), lines 25097-25110 (`[SonicZapE]` with `IsSonic=Yes` + `Burst=2`).
- Existing canonical doc: [`../../WAVECLASS_GHIDRA_REPORT.md`](../../WAVECLASS_GHIDRA_REPORT.md) (298 lines) — WaveClass struct + constructor + geometry helpers.
- Existing canonical doc: [`../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md`](../../WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md) (384 lines) — `WaveClass::AI` decomp + WeaponTypeClass flag map + `firer.CurrentWave (+0x324)` slot identification. **NOTE: §3 "IsSonic is dead" claim is incorrect; see §1 of this doc.**
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`ambient_damage.md`](ambient_damage.md) (the related still-open AmbientDamage consumer trace), [`rail_gun.md`](rail_gun.md) (RadBeam visual is sibling system), [`locomotor_warhead.md`](locomotor_warhead.md) (Magnetron's WaveClass type 3 — when written).
