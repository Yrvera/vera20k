# Rail Gun (`IsRailgun` + `IsRadBeam` Visual)

This doc is the canonical reference for **rail-gun-style weapons** in gamemd.exe — the
beam-visual weapons (Desolator, Chrono Legionnaire, and the secret-tech Mammoth Rail Gun /
Light Rail).

Two distinct flag systems compose to produce a rail-gun weapon:

1. **`IsRadBeam=yes`** on the WeaponTypeClass — triggers the `RadBeam` visual class to
   draw a colored beam from firer to target.
2. **`IsRailgun=yes`** on the WeaponTypeClass — gates the "sticky-beam" behavior in
   GetROF (no burst shortening while a beam is active) and GetFireError (FIRE_BUSY while
   the railgunParticleSystem is alive).

For weapons that need to deliver damage ALONG the beam path (LtRail / MechRailgun with
`Damage=0`), the actual damage delivery uses the `AmbientDamage=` weapon field —
documented in [`ambient_damage.md`](ambient_damage.md). The consumer site for that field
is **still NOT traced** in this iteration; see open follow-up #1.

Out-of-scope:
- The RadBeam visual class internals → [`../../RAD_BEAM_CLASS_GHIDRA_REPORT.md`](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md) (398 lines, exhaustive)
- Particle-system spawn for railgun sparks → [`../../PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`](../../PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md)
- Desolator deploy beam (RadEruption) → [`weapons/DesolatorDeploy.md`](../weapons/DesolatorDeploy.md) when written
- Damage transform → [`damage_formula.md`](damage_formula.md)
- AmbientDamage per-cell loop → [`ambient_damage.md`](ambient_damage.md)

---

## 1. Flag layout (verified)

### WeaponTypeClass flags

| Offset | INI key | String addr | Effect |
|---|---|---|---|
| `weapon+0x12D` | `IsRailgun=` | `0x00849368` (verified live 2026-05-17) | Sticky-beam particle gate (GetROF / GetFireError); enables `Owner+0x314 railgunParticleSys` tracking |
| `weapon+0x154` | `IsRadBeam=` | (per existing canonical doc, parsed at `WeaponTypeClass::ReadINI 0x007728B3`) | Triggers RadBeam visual via `TechnoClass__SpawnRadBeam 0x006FD620` |
| `weapon+0x155` | `IsRadEruption=` | per existing canonical doc | Triggers 8-beam 3×3 ring via `TechnoClass__SpawnRadEruption 0x006FD800` — Desolator deploy power; flagged as suspect-TS in existing doc but actually live for Desolator deploy |
| `weapon+0x98` | `AmbientDamage=` | per [`ambient_damage.md`](ambient_damage.md) | The per-path damage value for `Damage=0` rail/sonic weapons |

### WarheadTypeClass flags

| Offset | INI key | Effect |
|---|---|---|
| `wh+0x15A` | `Temporal=` | Combined with `IsRadBeam=yes`, swaps beam color from green (Desolator) to blue (Chrono Legionnaire). See [`temporal.md`](temporal.md). |

### Confidence (flags)

- **Content: HIGH** — `IsRailgun` string xref verified live 2026-05-17.
- **Identity: HIGH** — single string, single parse site.
- **Binding: HIGH** for `IsRailgun` ROF/FireError gates (verified in iterations 4 + 7); HIGH for `IsRadBeam` visual via existing canonical doc.

---

## 2. Retail YR usage

`grep ^IsRailgun=yes` in `ini/rulesmd.ini`:

```ini
[LtRail]                 ; rulesmd.ini:22631 — Allied secret tech "Light Rail" (rare)
Damage=0
AmbientDamage=150
IsRailgun=yes
IsRadBeam=yes
Warhead=RailShot

[MechRailgun]            ; rulesmd.ini:22645 — Mammoth Rail Gun (Allied secret tech)
Damage=0  (presumed; see ambient_damage.md)
AmbientDamage=200
IsRailgun=yes
IsRadBeam=yes
Warhead=RailShot2
```

`grep ^IsRadBeam=yes` adds the Desolator/Chrono Legionnaire weapons:

```ini
[RadBeamWeapon]          ; Desolator primary
Damage=...               (uses normal Damage, not AmbientDamage)
IsRadBeam=yes
Warhead=RadBeamWarhead

[CRRadBeamWeapon]        ; IFV-mounted Desolator variant
IsRadBeam=yes

[RadBeamWeaponE]         ; Elite Desolator
IsRadBeam=yes

[NeutronRifle]           ; Chrono Legionnaire — Damage=8 (this is the WarpHP-decrement rate)
Damage=8
ROF=120
IsRadBeam=yes
Warhead=ChronoBeam       ; Temporal=yes → blue beam
```

`grep ^IsRadEruption=yes`:
```ini
[DeplDesoWeapon]         ; Desolator's deployed-mode area weapon
IsRadEruption=yes
```

So the four user-facing rail/beam weapons in retail YR are:
- Desolator (3 variants: Primary, IFV-mounted, Elite) — green beam, normal damage path
- Desolator deployed — RadEruption 3×3 (separate mechanism)
- Chrono Legionnaire — blue beam via Temporal warhead, **Temporal damage path not normal damage** (see [`temporal.md`](temporal.md))
- Light Rail / Mammoth Rail Gun — green beam, `Damage=0 + AmbientDamage=N` path

### Notable usage patterns

| Pattern | Weapons | Damage path |
|---|---|---|
| `IsRadBeam=yes, Damage=N>0, normal warhead` | Desolator (`Damage=20`, warhead `RadBeamWarhead`) | **Standard damage** — projectile delivers Damage to target at impact, warhead radiation creates RadSite |
| `IsRadBeam=yes, Damage=N>0, Temporal warhead` | Chrono Legionnaire (`Damage=8`, warhead `ChronoBeam` Temporal=yes) | **Temporal erase** — Damage = WarpHP-decrement per tick, no standard damage |
| `IsRadBeam=yes, Damage=0, AmbientDamage=N>0` | LtRail (150), MechRailgun (200) | **AmbientDamage path** — applies along beam path; consumer site UNRESOLVED |
| `IsRadEruption=yes` | Desolator deploy (`DeplDesoWeapon`) | **3×3 cluster of beams** + standard area damage from warhead |

---

## 3. The two beam types (visual)

`TechnoClass__SpawnRadBeam` at `0x006FD620` selects beam color based on a 0/1/2 parameter:

| Parameter | Color source | Step size | Used for |
|---|---|---|---|
| 0 | `Rules+0x1830` (RadColor from `[Radiation] RadColor=0,255,0`) | 20 leptons | Desolator (green beam) |
| 1 | `Rules+0x1866` (hardcoded TS-era constant — no INI parser) | 10 leptons | Chrono Legionnaire (blue beam, via Temporal warhead) |
| 2 | `Rules+0x1869` (hardcoded TS-era red constant — no INI parser) | (n/a) | **TS-legacy** — no live callsite sets parameter=2 in YR |

The decision in `Fire_At` (per existing canonical doc):
```c
if (weapon->IsRadBeam) {
    int beam_type;
    if (warhead == NULL || warhead->Temporal == 0):
        beam_type = 0;       // green
    else:
        beam_type = 1;       // blue (Chrono Legion)
    TechnoClass::SpawnRadBeam(target, beam_type);
}
```

### Hardcoded beam parameters

Both beam types use:
- `RadBeam__SetDuration(0x0F)` — **15 frames** (~1.5 sec at 10fps sim)
- `RadBeam__SetAmplitudeAndPeriod(0, 0x40440000)` — amplitude = 3.0f, period = 0
- BeamType = 1 (straight line, after construction)

The beam is purely visual — it draws on top of the world via direct `g_PrimarySurface->DrawLine` (bypasses the LayerClass system). It does NOT carry damage in itself. See [`../../RAD_BEAM_CLASS_GHIDRA_REPORT.md`](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md) §8 for rendering details.

### Confidence

- **Content: HIGH** — fully decompiled in existing canonical doc; live re-decomp 2026-05-17 of `SpawnRadBeam 0x006FD620` confirms color-source switch.
- **Identity: HIGH** — named function with single caller pattern in Fire_At.
- **Binding: HIGH** — every IsRadBeam weapon goes through this.

---

## 4. Sticky-beam gates (verified in earlier iterations)

`IsRailgun=yes` weapons trigger two gates that change per-tick combat behavior:

### GetROF Branch 2 (sticky-beam — full ROF return)

Per [`rof_burst_timing.md`](rof_burst_timing.md) §3 Branch 2:

```c
if weapon.IsRailgun (+0x12D) != 0 && this.railgunParticleSys (+0x314) != 0:
    return weapon.ROF                  // no burst shortening
```

So while the rail-gun particle system is active on the firer, the ROF return is the
full reload value — preventing rapid re-fire mid-beam.

### GetFireError Phase D (sticky-beam — FIRE_BUSY block)

Per [`can_target_gates.md`](can_target_gates.md) Phase D gate #23:

```c
if weapon.IsRailgun (+0x12D) != 0 && this.railgunParticleSys (+0x314) != 0:
    return 3 (BUSY)                    // can't fire while previous beam particle alive
```

This prevents a new fire trigger from running while the previous beam is still
finishing. The two gates work together: GetFireError blocks until the particle expires,
GetROF ensures the cooldown timer is set to the full ROF value at the start.

### Where is `railgunParticleSys` written?

Per the existing canonical `WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS` doc:
> `AttachedParticleSystem` spawn-on-fire wired to `UseFireParticles`/`UseSparkParticles`/`IsRailgun` (triple-gate, and each has a per-weapon single-instance guard `this->field_0x304/0x308/0x314`).

So a weapon with `IsRailgun=yes` AND `AttachedParticleSystem=` spawns a ParticleSystem
on fire, stores the pointer at `this+0x314 (railgunParticleSys)`, and the gates above
fire while that pointer is non-NULL. The particle expires after its own lifetime; on
expiry, `this+0x314` is cleared and the gates open up.

### Confidence

- **Content: HIGH** — both gates verified in iterations 4 + 7.
- **Identity: HIGH** — single weapon flag, single instance pointer.
- **Binding: HIGH** — verified consumer call sites.

---

## 5. The damage path — three regimes

### Regime A: Standard damage (Desolator)

Desolator's `RadBeamWeapon` has `Damage=20, Warhead=RadBeamWarhead`. Standard
combat flow:
1. Fire_At creates a BulletClass with the Damage value, sets target.
2. SpawnRadBeam draws the visual beam (parameter 0 = green).
3. The projectile is invisible (or has minimal visual) and hits the target.
4. On impact, Apply_area_damage runs with the warhead, applying:
   - The 20 damage via Verses to the target.
   - The radiation site creation via `Radiation=yes` warhead flag (see [`radiation.md`](radiation.md)).

Total damage delivered: standard projectile damage + radiation tick damage over time.

### Regime B: Temporal erase (Chrono Legionnaire)

Chrono Legionnaire's `NeutronRifle` has `Damage=8, Warhead=ChronoBeam` (Temporal=yes).
Standard combat flow:
1. Fire_At creates a BulletClass.
2. SpawnRadBeam draws the visual beam (parameter 1 = blue).
3. Projectile hits target.
4. WarheadTypeClass::Detonate enters the Temporal branch (priority 4 in the warhead
   cascade) — see [`temporal.md`](temporal.md).
5. The `Damage=8` field is consumed as the **WarpHP decrement rate per tick**, NOT as
   a damage value applied via Verses.

Total damage: erasure-over-time at 8 WarpHP/tick.

### Regime C: AmbientDamage path (Light Rail / Mammoth Rail Gun)

`LtRail` and `MechRailgun` have `Damage=0, AmbientDamage=N`. Standard combat flow:
1. Fire_At creates a BulletClass with Damage=0.
2. SpawnRadBeam draws the visual beam.
3. **Somewhere in Fire_At**, AmbientDamage is applied along the beam path. Each cell
   between firer and target (or each unit within some radius of the line) receives
   the AmbientDamage value via `target->ReceiveDamage`, going through normal
   Verses/AffectsAllies/MaxDamage clamping.
4. The projectile's Damage=0 means the standard impact has no effect.

**The exact consumer site for AmbientDamage is NOT traced** in this iteration. Per
[`ambient_damage.md`](ambient_damage.md) §3 working hypothesis, the dispatch loop is
inside Fire_At, iterating cells along the firer→target line. **Resolution is open
follow-up #1.**

### Confidence by regime

- Regime A (standard): **HIGH** — verified via existing canonical doc flow.
- Regime B (Temporal): **HIGH** — verified in [`temporal.md`](temporal.md).
- Regime C (AmbientDamage): **LOW for the consumer mechanism**. HIGH for the parser (offset / default / which weapons use it).

---

## 6. Key addresses summary

| Address | Function |
|---|---|
| `0x006FD620` | `TechnoClass::SpawnRadBeam` (color + beam visual setup) |
| `0x006FD800` | `TechnoClass::SpawnRadEruption` (Desolator deploy 8-beam ring) |
| `0x00659110` | `RadBeam::Allocate` (200-byte allocation + array registration) |
| `0x006591B0` | `RadBeam::DrawAndTickAll` (per-frame draw + tick) |
| `0x00659650` | `RadBeam::DrawStraightBeam` (BeamType 1 draw) |
| `0x00659CA0` | `RadBeam::DrawSineBeam` (BeamType 2 draw — used by RadEruption) |
| `0x006FCFA0` | `TechnoClass::GetROF` (sticky-beam ROF gate — Branch 2) |
| `0x006FC0B0` | `TechnoClass::GetFireError` (sticky-beam FIRE_BUSY gate — Phase D) |
| `0x00772080` | `WeaponTypeClass::ReadINI` (IsRailgun / IsRadBeam / IsRadEruption parser) |

---

## 7. RadEruption — Desolator deploy

`IsRadEruption=yes` (separate flag at `weapon+0x155`) on the Desolator's `DeplDesoWeapon`
triggers `TechnoClass::SpawnRadEruption 0x006FD800`, which spawns **8 RadBeam instances**
in a 3×3 neighbor pattern around the firer's cell. Each beam is BeamType 2 (sine-wave
visual) with random amplitude (5..20) and random duration (100..500).

This is the visible "rad cloud" effect when a Desolator deploys. The damage is delivered
via the normal warhead `RadEruptionWarhead` (`Radiation=yes`) applied at the impact, plus
the RadSite created at the impact cell ticks damage to units standing in the area for
the duration of the radiation site (see [`radiation.md`](radiation.md)).

**Note:** Existing canonical RadBeam doc §11 flagged RadEruption as potentially TS-dormant
("does not appear in `rulesmd.ini` outside example/commented sections") — but the modern
retail rulesmd.ini has `[DeplDesoWeapon] IsRadEruption=yes` per my grep above. **This is
LIVE in YR.** The canonical-doc flag should be updated. Open follow-up #3 below.

---

## 8. TS-legacy filter

| Component | Status |
|---|---|
| `IsRadBeam=yes` weapon flag | **LIVE** — Desolator + Chrono Legionnaire |
| `IsRailgun=yes` weapon flag | **LIVE** — used by GetROF + GetFireError gates; LtRail/MechRailgun set it |
| `IsRadEruption=yes` weapon flag | **LIVE** — Desolator deploy power |
| `[Radiation] RadColor=` Rules color | **LIVE** — Desolator beam color |
| Beam type 2 (red — `Rules+0x1869`) | **TS-legacy DORMANT** — no live callsite |
| `Rules+0x1866` (Chrono Legion blue hardcoded) | **LIVE** — Chrono Legion beam color |
| RadBeam C++ class | **LIVE** (≠ IonBlastClass which is fully dormant; see [`ion_cannon.md`](ion_cannon.md)) |

---

## 9. Edge cases

| Case | Behavior |
|---|---|
| Weapon has `IsRadBeam=yes, IsRailgun=no, Damage=0, AmbientDamage=0` | Beam draws visually but does NO damage. Useful for "marker" weapons in mods. |
| Weapon has `IsRailgun=yes, IsRadBeam=no` | Sticky-beam gates apply but no visual beam is spawned. Particle-only feedback. |
| Two rail-gun units fire at same target in same tick | Each spawns its own RadBeam instance; both visuals overlap. Each ROF is independently locked while particle exists. |
| Rail-gun unit's target dies mid-beam | Beam visual continues for its 15-tick duration. AmbientDamage path (if any) was dispatched at fire time, so already applied. |
| Rail-gun unit takes damage mid-beam | Beam visual is unaffected (it's a separate RadBeam object). Owner's sticky-beam particle still gates re-fire. |
| Chrono Legion fires at Temporal-immune (Warpable=no) target | SpawnRadBeam draws blue beam (visual happens), but Temporal warhead's CanWarpTarget rejects the warp. Target takes no damage. Visible "Chrono Legion is firing but nothing's happening" bug-look. |
| Desolator fires at radiation-immune target (ImmuneToRadiation) | Standard beam visual draws, projectile delivers Damage=20 normally (Verses applies), but the warhead's RadSite creation is blocked. |
| Modder sets `Damage=50, AmbientDamage=100, IsRailgun=yes, IsRadBeam=yes` | Standard projectile damage (50) PLUS AmbientDamage path (100 along the beam path, per regime C). Compound damage. Untested combination. |

---

## 10. Open follow-ups

1. **AmbientDamage consumer code site** — Same open follow-up as [`ambient_damage.md`](ambient_damage.md) #1. The exact function that applies AmbientDamage along the beam path is NOT traced. Working hypothesis: a loop in `Fire_At` iterating cells along firer→target. **Priority: HIGH** — needed for LtRail / MechRailgun parity. Recommended approach: byte-pattern search for `mov eax, [reg + 98h]` in live combat code near SpawnRadBeam.
2. **`IsRadBeam` parse offset** — Existing canonical doc says `weapon+0x154`. Cross-reference against the `weapon+0x12D` / `weapon+0x130` cluster documented elsewhere — verify `+0x154` is correct. Priority: MEDIUM.
3. **Update canonical `RAD_BEAM_CLASS_GHIDRA_REPORT.md` RadEruption section** — That doc flags RadEruption as potentially dormant, but retail rulesmd.ini has `[DeplDesoWeapon] IsRadEruption=yes`. Update to mark LIVE. Priority: LOW.
4. **Particle-system lifetime that gates re-fire** — When does `this+0x314 railgunParticleSys` get cleared? Probably on particle expiry. Trace the clear site. Priority: LOW (the gate behavior is well-defined; the clear mechanism just affects timing).
5. **Step-size constants** — Beam visual uses step=10 (blue/temporal) or step=20 (green/normal). Are these hardcoded in `RadBeam__Allocate` or do they come from INI? Per existing canonical doc, hardcoded. Priority: LOW.
6. **`Rules+0x1866` Chrono Legion blue and `Rules+0x1869` red constants** — Are these INI-parsed under any key, or pure hardcoded? Existing canonical doc says hardcoded (no INI parser for either). Worth re-verifying. Priority: LOW.

---

## 11. Sources

- Existing canonical doc: [`../../RAD_BEAM_CLASS_GHIDRA_REPORT.md`](../../RAD_BEAM_CLASS_GHIDRA_REPORT.md) (398 lines) — exhaustive coverage of RadBeam class, struct layout, draw paths, color sources. Primary source for visual side.
- Existing canonical doc: [`../../PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`](../../PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md) — particle-spawn timing.
- Live verification (2026-05-17):
  - `"IsRailgun"` at `0x00849368` (single string match)
  - `TechnoClass__SpawnRadBeam 0x006FD620` decompiled — confirmed color-selector switch and visual setup, NO damage dispatch
- INI quotes from `ini/rulesmd.ini`:
  - line 22631: `[LtRail]` rail variant
  - line 22645: `[MechRailgun]` Mammoth Rail Gun
  - Desolator weapon sections (`RadBeamWeapon`, `CRRadBeamWeapon`, `RadBeamWeaponE`)
  - Chrono Legionnaire `NeutronRifle`
  - Desolator deploy `DeplDesoWeapon` with `IsRadEruption=yes`
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`temporal.md`](temporal.md), [`radiation.md`](radiation.md), [`rof_burst_timing.md`](rof_burst_timing.md) (sticky-beam Branch 2), [`can_target_gates.md`](can_target_gates.md) (Phase D gate #23), [`ambient_damage.md`](ambient_damage.md) (the still-open consumer trace).
