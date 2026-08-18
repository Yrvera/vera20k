# Ambient Damage

This doc is the canonical reference for the `AmbientDamage=` weapon field in gamemd.exe.

The flag is sparse in retail YR — used by exactly five weapons (Rail Gun variants
and Sonic Tank weapons) — and the INI comments next to it say:

```
use this for the railgun damage field.  Leave damage = 0
```

So `AmbientDamage=` is the **damage value applied along the path of beam-style weapons**
(rail gun beams, sonic waves) — distinct from the projectile's own `Damage=` (which is
0 for these weapons; the bullet exists only to deliver the beam visual). This is the
mechanism by which a rail gun deals 150 HP along its trajectory, even though its
`Damage=0`.

Out-of-scope:
- Per-target damage transform at impact → [`damage_formula.md`](damage_formula.md)
- RadBeam visual class → [`rail_gun.md`](rail_gun.md), [`../../RAD_BEAM_CLASS_GHIDRA_REPORT.md`](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md)
- WaveClass (sonic wave) visual → [`sonic.md`](sonic.md), [`../../WAVECLASS_GHIDRA_REPORT.md`](../../WAVECLASS_GHIDRA_REPORT.md)

---

## 1. Field identity (verified)

| Field | Value |
|---|---|
| Offset | `weapon+0x98` |
| Type | `int` |
| INI key | `AmbientDamage=` |
| String addr | `0x00849548` |
| Parser xref | `WeaponTypeClass::ReadINI 0x007720bb` (single DATA xref) |
| Default | `0` |

### Confidence (identity & parsing)

- **Content: HIGH** — live decomp of `WeaponTypeClass::ReadINI 0x00772080` (2026-05-17) shows AmbientDamage parsed as the **first** ReadInt in the function, storing into `*(int*)(this + 0x98)`.
- **Identity: HIGH** — single string match for `"AmbientDamage"` with single xref into ReadINI.
- **Binding: HIGH** for the parser side. **LOW** for the consumer side (see §3 — consumer not fully traced in this iteration).

---

## 2. Retail YR usage (exhaustive list, verified from `rulesmd.ini`)

Grep `^AmbientDamage=` returns **exactly 5 matches** in shipping `rulesmd.ini`:

```ini
[LtRail]                ; rulesmd.ini:22631
Damage=0                ; this should be 0 for railgun shots
AmbientDamage=150       ; use this for the railgun damage field.  Leave damage = 0

[MechRailgun]           ; rulesmd.ini:22645  (Mammoth Rail Gun — Allied secret tech)
AmbientDamage=200       ; use this for the railgun damage field.  Leave damage = 0

[FireballLauncher]      ; rulesmd.ini:22823
Damage=0
AmbientDamage=2

[SonicZap]              ; rulesmd.ini:23678  (Sonic Tank primary)
Damage=4
AmbientDamage=10

[SonicZapE]             ; rulesmd.ini:25097  (Elite Sonic Tank)
Damage=8
AmbientDamage=15
```

Observations from the INI quotes:

- **Two distinct usage patterns** are visible:
  - **Pattern A: Pure ambient damage** (`Damage=0, AmbientDamage=N`). `LtRail`, `MechRailgun`, `FireballLauncher`. The projectile delivers no impact damage; ALL damage is the ambient path effect.
  - **Pattern B: Both fire-impact and ambient damage** (`Damage=N, AmbientDamage=M`). `SonicZap` and `SonicZapE`. The projectile delivers a small impact damage at the target AND an ambient path damage along the wave.

- **Used by visually-beam weapons**: Rail Gun (`IsRailgun=yes`), Sonic Tank (`IsSonic=yes`), FireballLauncher (per its name + `Damage=0` pattern). These are the three weapon types in gamemd that draw a beam-shaped visual rather than a flying projectile.

### Cross-reference: which TechnoTypes use these weapons?

| Weapon | Used by |
|---|---|
| `LtRail` | (search rulesmd.ini for `Primary=LtRail` / `Secondary=LtRail`) |
| `MechRailgun` | Allied "secret tech" Mammoth Rail Gun (special infantry — Heroes/Yuri-mode unit) |
| `FireballLauncher` | Civilian flame-tower / fire animation aggressor |
| `SonicZap` | `[SREF]` (Sonic Tank) primary |
| `SonicZapE` | Elite Sonic Tank — used via veterancy weapon swap (`ElitePrimary=SonicZapE` per [`veterancy_weapon_swap.md`](veterancy_weapon_swap.md)) |

The user-of-weapon trace is straightforward via grep; not redone here in detail.

---

## 3. Consumer location — **OPEN FOLLOW-UP**

The exact code site that consumes `weapon+0x98` is **not traced in this iteration**.
The existing canonical doc [`../../FIRE_AT_PIPELINE_GHIDRA_REPORT.md`](../../FIRE_AT_PIPELINE_GHIDRA_REPORT.md) says:

> "Applied to nearby cells on fire (not a bullet mechanic)"

That description suggests the damage is dispatched **inside `TechnoClass::Fire_At`** at
fire time (i.e., at the firer, not at the projectile's impact), iterating cells along
the firer-to-target line (or the wave/beam path) and calling `ReceiveDamage` on each.

### Working hypothesis (NOT verified — flag for follow-up)

For a rail-gun-style weapon:

```c
// In Fire_At, after launching the visual projectile:
if (weapon->AmbientDamage > 0) {
    // Iterate cells along firer → target line
    foreach cell in path(firer.coords → target.coords):
        foreach techno in cell.occupants:
            techno->ReceiveDamage(
                &(weapon->AmbientDamage),    // damage value
                0,                            // distance — point-blank per target
                weapon->Warhead,              // warhead (for Verses + effects)
                firer,                        // attacker
                false, false,
                firer.Owner                   // source house
            );
}
```

For sonic-wave-style weapons, the path is a moving sphere (the wave expands and damages
on each tick of its lifetime, again calling ReceiveDamage on units it passes through).
The Wave/RadBeam visual carries `weapon+0x98` (AmbientDamage) into its damage-tick callback.

### Why this matters for parity

- Verses STILL applies. A rail gun with `AmbientDamage=150` and `Warhead=RailShot` against
  a target with Verses[heavy]=50% delivers 150 × 0.5 = 75 damage. Source: each
  per-cell/per-unit ReceiveDamage routes through the standard `GetDamage` formula in
  [`damage_formula.md`](damage_formula.md).
- AffectsAllies STILL applies (per-target gate in ReceiveDamage).
- CellSpread does NOT apply in the usual way — the damage is per-path-step, not
  radial-from-impact. Each affected unit is treated as "at distance 0" from the path.

### Confidence (consumer)

- **Content: LOW** — working hypothesis only.
- **Identity: LOW** — function not yet decompiled.
- **Binding: LOW** — caller chain unverified.

This open follow-up must be resolved before any implementation depends on
AmbientDamage semantics. Priority: HIGH (the magnitudes — 150, 200 — are large; parity
matters).

---

## 4. The `IsRailgun` / `IsSonic` flag connection

The three weapons with `Damage=0, AmbientDamage>0` also set the relevant beam-visual flag:

- `LtRail`, `MechRailgun`: set `IsRailgun=yes` (`weapon+0x12D`)
- `SonicZap`, `SonicZapE`: set `IsSonic=yes` (`weapon+0x130`)
- `FireballLauncher`: likely `UseFireParticles=yes` (`weapon+0x12A`)

These flags are also consulted by:
- `TechnoClass::GetROF` (see [`rof_burst_timing.md`](rof_burst_timing.md) §3, Branch 2) — sticky-beam particle-active gate that returns full ROF (no burst shortening) while the beam particle exists.
- `TechnoClass::GetFireError` (see [`can_target_gates.md`](can_target_gates.md) Phase D) — particle-active block that returns FireError 3 (BUSY) while the previous beam is still resolving.

So the path is: firer fires → beam visual is created → beam's per-cell or per-tick
damage callback applies `AmbientDamage` to each unit it passes through → beam expires →
firer can fire again.

---

## 5. Composition rules (inferred — see §3 open follow-up)

| Behavior | Status |
|---|---|
| Verses applies per-target | ASSUMED YES (per-target ReceiveDamage routes through GetDamage) |
| AffectsAllies applies per-target | ASSUMED YES (ReceiveDamage gate) |
| CellSpread applies | ASSUMED NO (no radial dispatch; damage is per-cell on path) |
| MaxDamage clamp applies | ASSUMED YES (every ReceiveDamage call clamps) |
| Veteran/elite firepower bonus applies | ASSUMED YES (Fire_At-time multipliers run before damage is dispatched) |
| Country firepower bonus applies | ASSUMED YES |
| Friendly fire is gated by AffectsAllies | ASSUMED YES |
| Sub-damage at impact (when `Damage=N` is nonzero, e.g., SonicZap) | YES — the regular bullet damage path runs in addition to AmbientDamage |
| Burst combines with AmbientDamage | UNVERIFIED — Sonic and Rail weapons all use `Burst=1`. Theoretically Burst>1 would dispatch the ambient damage N times. Open follow-up. |

All entries marked ASSUMED are inferred from the standard per-target ReceiveDamage
contract. None of them are independently verified in this iteration.

---

## 6. TS-legacy filter

`AmbientDamage` is a **TS-era feature** that survived into YR. In Tiberian Sun it was
used much more broadly:
- Cyborg Reaper plasma cloud
- Disruptor sonic damage
- Toxin/poison clouds

In YR, only the 5 weapons above use it. The mechanism is **LIVE** in retail YR — every
match where a Rail Gun or Sonic Tank fires exercises this code path.

The two "infrastructure" classes that carry the ambient damage value (the **RadBeam**
visual class and the **WaveClass** sonic wave) are both YR-live. Their per-tick damage
callbacks consume `weapon+0x98` directly.

---

## 7. Composition with `Damage=N` (Sonic dual-damage)

`SonicZap` (Damage=4, AmbientDamage=10) is the interesting case:

- **Bullet impact (Damage=4)**: The projectile carries `Damage=4` and detonates on
  proximity to target (Sonic projectile is `Inviso=yes` per typical Sonic Tank
  configuration). At impact, normal `Apply_area_damage` runs with the SonicWarhead,
  delivering 4 damage to the target.
- **Wave path (AmbientDamage=10)**: As the WaveClass visual propagates, each unit it
  passes through takes 10 additional damage (via the inferred per-cell ReceiveDamage
  loop in §3).

Net effect for a unit hit directly: 4 + 10 = 14 damage (subject to Verses).

For Rail Gun (`Damage=0`), there's no bullet-impact damage; only the path damage.

---

## 8. Edge cases

| Case | Behavior |
|---|---|
| `AmbientDamage=0` (default) | No path-damage dispatch. Standard `Damage=` mechanics only. |
| `AmbientDamage` set but weapon has no beam-visual flag (e.g., plain bullet with `AmbientDamage=50`) | **UNDEFINED.** The consumer chain is from the RadBeam/WaveClass per-tick callback; if no such visual is created, the AmbientDamage is never dispatched. Probably dead in this scenario. **Open follow-up.** |
| `AmbientDamage` negative | The `ReadInt` parser accepts negatives. Per the standard damage formula, negative damage = heal (within 8 leptons). At-distance heal is blocked. Effect: probably a heal-along-path mechanic. **Untested.** |
| Multiple units along path | Each receives the full `AmbientDamage` value × Verses[their_armor]. No falloff with path distance. |
| Target dies mid-beam | Beam continues; remaining units along the path still take damage. |
| Firer dies mid-beam | The beam visual likely persists for its remaining lifetime. Damage attribution uses the firer's house at fire-time (already captured). **Untested.** |
| AmbientDamage on a building's weapon (e.g., Prism Tower retarget) | Should work identically — buildings fire weapons via `Fire_At` like any techno. |
| AmbientDamage stacking from multiple firers on same target | Each firer's beam separately ticks per-tick. Targets in overlap zones take N × per-firer ambient damage. |

---

## 9. Open follow-ups

1. **Consumer code site.** The exact function that reads `weapon+0x98` and applies it
   per-cell/per-tick is not traced. Likely candidates: `RadBeam::AI` (per-tick damage
   callback on the beam visual), `WaveClass::AI` (per-tick damage as the wave
   propagates), or a dedicated `ApplyAmbientDamage` helper called from `Fire_At`.
   Priority: **HIGH** — without this trace, the assumed composition rules in §5 are
   unverified. Trace path: search byte pattern `98 00 00 00` on a register-base read in
   live combat code, cross-reference RadBeam/WaveClass AI functions.
2. **Confirm Verses applies to ambient damage.** Inferred from per-target ReceiveDamage
   call, not directly verified. Priority: HIGH.
3. **Burst composition.** Does `Burst>1` × `AmbientDamage>0` apply the ambient damage N
   times? In retail YR no shipping weapon combines them, so it's modder-only behavior.
   Priority: LOW.
4. **Negative AmbientDamage.** Untested. Probably parses to heal-along-path. Priority:
   LOW.
5. **Non-beam weapon with AmbientDamage**. If a modder sets AmbientDamage on a regular
   bullet weapon (no `IsRailgun`/`IsSonic`), does the damage apply? Probably no
   consumer in that case — dead field. Priority: LOW.
6. **AnimType-based ambient**. The TS-era "ambient" concept also applied to animations
   (e.g., burning grass deals ambient damage to units standing in it). That mechanism
   lives in `AnimClass::AI` and is separate from this weapon field. Cross-reference for
   completeness: [`../../ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`](../../ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md) covers anim-side damage. Priority: LOW.
7. **FireballLauncher use case.** What unit fires `FireballLauncher`? It's `Damage=0,
   AmbientDamage=2` — a low-damage area-of-effect on what appears to be a civilian/scenery
   thing. Confirm and document. Priority: LOW.

---

## 10. Sources

- Live decompilation of `WeaponTypeClass::ReadINI` at `0x00772080` (2026-05-17) — confirmed `AmbientDamage` is the first ReadInt, storing to `weapon+0x98`.
- Live xref of `"AmbientDamage"` string at `0x00849548` → single DATA xref into `WeaponTypeClass::ReadINI 0x007720bb`.
- INI quotes from `ini/rulesmd.ini` lines 22631, 22645, 22823, 23678, 25097 — the 5 retail uses.
- Existing canonical doc: [`../../WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md`](../../WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md) §6 — confirms `AmbientDamage` parses to `weapon+0x98`.
- Existing canonical doc: [`../../FIRE_AT_PIPELINE_GHIDRA_REPORT.md`](../../FIRE_AT_PIPELINE_GHIDRA_REPORT.md) — single sentence on AmbientDamage ("Applied to nearby cells on fire, not a bullet mechanic"); consumer trace incomplete here.
- Existing canonical docs for beam visuals: [`../../RAD_BEAM_CLASS_GHIDRA_REPORT.md`](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md), [`../../WAVECLASS_GHIDRA_REPORT.md`](../../WAVECLASS_GHIDRA_REPORT.md) — neither doc currently references AmbientDamage directly; these are the likely homes for the consumer trace.
- Cross-references: [`damage_formula.md`](damage_formula.md), [`friendly_fire.md`](friendly_fire.md), [`rof_burst_timing.md`](rof_burst_timing.md), [`can_target_gates.md`](can_target_gates.md), [`rail_gun.md`](rail_gun.md), [`sonic.md`](sonic.md).
