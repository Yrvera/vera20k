# Rate of Fire & Burst Timing

This doc is the canonical reference for **fire cadence** in gamemd.exe:

- `ROF=` on WeaponTypeClass — frames between reloads
- `Burst=` on WeaponTypeClass — shot count per fire trigger
- The runtime `CurrentBurstIndex` counter on TechnoClass that cycles them
- `BurstDelay0..3` on InfantryTypeClass — per-shot delay slots (rarely used in retail)
- `FiringSyncFrame0..1` on InfantryTypeClass — animation-frame gating
- Random-jitter behavior (end-of-burst and inter-shot)
- Veterancy, naval-barrel, crate-powerup ROF modifiers (where they live)
- Interactions: Sonic/Fire/Spark/Railgun bypass; IsGattling scatter; Airburst/Shrapnel/Inaccurate composition

Out-of-scope:
- The damage-per-shot itself → [`damage_formula.md`](damage_formula.md)
- Gattling stage progression (which weapon is selected) → [`gattling_spool.md`](gattling_spool.md)
- Airburst sub-weapon spawn → [`airburst.md`](airburst.md)
- Bullet flight after launch → [`bullet_lifecycle.md`](bullet_lifecycle.md)
- The full Fire_At pipeline (target re-resolve, FLH, bullet creation) → [`fire_at_pipeline.md`](fire_at_pipeline.md)

---

## 1. Cadence model in one paragraph

`Burst=N` does **not** loop inside one `Fire_At` call. It produces N independent
`Fire_At` invocations, one per tick (subject to the inter-shot delay timer). The shared
fire timer at `this+0x2F8` is set on each shot by `GetROF`'s return value. For mid-burst
shots, `GetROF` returns a short value (typically 3–5 frames); for the final shot of a
burst (when `CurrentBurstIndex` is about to wrap to 0), `GetROF` returns the full
`ROF=` value with veterancy/naval/crate modifiers applied. The counter at
`this+0x3B8 (CurrentBurstIndex)` is incremented at the end of every `Fire_At` and
wraps `% weapon.Burst`. **It is never explicitly reset** — a partial burst that gets
interrupted (target dies, target out of range) leaves the counter mid-cycle, and the
next engagement against any target picks up at the carry-over index.

---

## 2. Key offsets (verified)

### WeaponTypeClass

| Offset | Type | Field | INI key | Default |
|---|---|---|---|---|
| `+0x9C` | `int` | `Burst` | `Burst=` | 1 |
| `+0xB0` | `int` | `ROF` | `ROF=` | (no default in INI; weapon-specific) |
| `+0x129` | `bool` | `UseSparkParticles` | `UseSparkParticles=` | false |
| `+0x12A` | `bool` | `UseFireParticles` | `UseFireParticles=` | false |
| `+0x12D` | `bool` | `IsRailgun` | `IsRailgun=` | false |
| `+0x130` | `bool` | `IsSonic` | `IsSonic=` | false |

The four flags `IsSonic/UseSparkParticles/UseFireParticles/IsRailgun` form the "sticky
beam" group — when any of them is set AND the corresponding particle system is active
on the firer, GetROF returns the full ROF regardless of burst index (i.e., burst is
silently disabled for these visuals).

### TechnoClass (runtime per-unit)

| Byte offset | Field | Notes |
|---|---|---|
| `+0x2A0` | `GattlingScatterIndex` | random base for Gattling scatter ring |
| `+0x2EC` | `FireTimer.StartFrame` | `g_CurrentFrameCounter` at last shot |
| `+0x2F0` | `FireTimer.Range` | per-shot range scratch |
| `+0x2F4` | `FireTimer.InitialValue` | initial cooldown (for remaining-time math) |
| `+0x2F8` | `FireTimer.ROF` | active cooldown — the value GetROF returned |
| `+0x2FC` | `MultiBarrelFlag` | building multi-barrel shortcut (`>1` → GetROF returns 1) |
| `+0x3B8` | **`CurrentBurstIndex`** | 0..Burst-1, modular counter |
| `+0x43C` | `BarrelRotationIndex` | Gattling angular-offset (separate from burst) |

### InfantryTypeClass (infantry-specific firing)

| Byte offset | Field | INI key | Notes |
|---|---|---|---|
| `+0xE40` | `int` | `FiringSyncFrame0` | anim frame to trigger primary weapon fire |
| `+0xE44` | `int` | `FiringSyncFrame1` | anim frame to trigger secondary weapon fire |
| `+0xE48` | `int` | `BurstDelay0` | inter-shot delay for burst_idx=1 (2nd shot); default 0 |
| `+0xE4C` | `int` | `BurstDelay1` | inter-shot delay for burst_idx=2 (3rd shot); default 0 |
| `+0xE50` | `int` | `BurstDelay2` | **UNSAFE** — corrupts adjacent DynamicVectorClass |
| `+0xE54` | `int` | `BurstDelay3` | **UNSAFE** — same problem |

**Caveat verified:** `InfantryTypeClass::Constructor` at `0x005236A0` initializes a
DynamicVectorClass at index `[0x394]` (byte `0xE50`) with `&PTR_FUN_007eb6d4`. The
`BurstDelay2`/`BurstDelay3` INI writes at `0xE50`/`0xE54` will overwrite that DVC's
vtable pointer and internal size fields. **Only `BurstDelay0` and `BurstDelay1` are
safely usable.** Modders using `Burst≥4` on infantry should expect `BurstDelay2/3` to
crash or behave erratically.

**Survey:** No `BurstDelay%d=` keys appear in shipping `rulesmd.ini`. The parser exists,
but retail YR never uses it. All shipping burst weapons use the random-3-to-5 fallback.

### Confidence (offsets)

- **Content: HIGH.** All offsets re-verified live in `GetROF` decompilation at `0x006FCFA0` (read 2026-05-17): `param_1[0xee] = +0x3B8` (CurrentBurstIndex), `iVar2+0x9c = Burst`, `iVar2+0xb0 = ROF`, `iVar2+0x130/0x12a/0x129/0x12d` for the four sticky-beam flags, `(uVar4+0x6c4)+0xe44+iVar5*4` for InfantryType.BurstDelay[burst_idx-1].
- **Identity: HIGH.** WeaponTypeClass struct match against [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md); InfantryTypeClass `BurstDelay%d` loop verified via `s_BurstDelay_d_00845ca0` string xref to `InfantryTypeClass::ReadINI` (Ghidra-mislabeled `UnitTypeClass__ReadINI` at `0x00747620`).
- **Binding: HIGH** (writers): `WeaponTypeClass::ReadINI` writes `+0x9C`/`+0xB0` (string xref from `"Burst"` at `0x00849438` to ReadINI at `0x007722C1`). `InfantryTypeClass::ReadINI` writes `+0xE40`..`+0xE54` (the BurstDelay/FiringSyncFrame loops). `Fire_At` writes `+0x3B8` and `+0x2EC..+0x2F8`. **Binding: HIGH (consumers)**: `GetROF` is the sole reader of `+0x3B8` for cadence purposes; the Gattling branch in `Fire_At` also reads it for scatter offset.

---

## 3. `GetROF` — the cadence decision (verified, full branches)

Function: **`FUN_006fcfa0`** at `0x006FCFA0`. Called via `vtable+0x318` from `Fire_At`'s
end-of-shot block. Decompilation read 2026-05-17 confirms the algorithm below:

```
GetROF(this, weaponIndex):
    // ── Branch 1: building multi-barrel shortcut ──
    if this.WhatAmI() == 6 && this->byte_0x2FC > 1:
        return 1

    weapon = this.GetWeapon(weaponIndex)        // vtable+0x3F8
    if weapon == NULL || weapon.type == NULL:
        return 1

    // ── Branch 2: sticky beams skip burst shortening ──
    // Reads the four flags on weapon and checks the matching particle system on this
    if weapon.IsSonic                            // weapon+0x130
        OR (weapon.UseFireParticles  && this.fireParticleSys)    // 0x12A & +0x308
        OR (weapon.UseSparkParticles && this.sparkParticleSys)   // 0x129 & +0x304
        OR (weapon.IsRailgun         && this.railgunParticleSys):// 0x12D & +0x314
        return weapon.ROF                        // weapon+0xB0

    isInfantry = (this.WhatAmI() == 1)
    burstIdx = this.CurrentBurstIndex            // +0x3B8

    if burstIdx < weapon.Burst:
        // ── Branch 3: mid-burst — short inter-shot delay ──
        if 0 < burstIdx < 5 && isInfantry:
            // Read InfantryType.BurstDelay[burstIdx-1]
            // Address = (this->InfantryType)[0xE44 + burstIdx*4]
            //   burstIdx=1 → +0xE48 = BurstDelay0
            //   burstIdx=2 → +0xE4C = BurstDelay1
            //   burstIdx=3 → +0xE50 = BurstDelay2 (UNSAFE)
            //   burstIdx=4 → +0xE54 = BurstDelay3 (UNSAFE)
            delay = this.InfantryType[0xE44 + burstIdx*4]
            if delay != -1:                      // sentinel -1 = "use random"
                return delay

        return Random.RandomRanged(3, 5)         // non-infantry or sentinel fallthrough

    // ── Branch 4: end-of-burst — full ROF with modifiers ──
    Random.RandomRanged(0, 2)                    // jitter, result captured below
    rof = ftol(weapon.ROF * <jittered float>)    // small + jitter

    if IsVeteran(this) || IsElite(this):
        type = this.GetTechnoType()              // vtable+0x84
        if (IsVeteran && type.VeteranAbilities & 0x2A0_FIREPOWER_bit) ||
           (IsElite   && (type.VeteranAbilities & 0x2A0_FIREPOWER_bit
                           || type.EliteAbilities & 0x2B2_DVARMOR_bit)):
            rof = ftol(rof * <veteran combat mult>)   // Rules.VeteranCombat ~1.1

    // ── Branch 5: naval (multi-barrel) ROF divide ──
    if this.IsNaval():                           // vtable+0x400
        barrels = this.GetBarrelCount()          // vtable+0x408
        if barrels > 0: rof /= barrels
        if Rules.NavalFirepowerMult > 1.0:       // RulesClass+0xF44
            rof = ftol(rof * <naval mult>)

    // ── Branch 6: crate-powerup mult ──
    if this->field_0x2E4_cratePowerup != 0:      // param_1[0xb9]
        if this.WhatAmI() != 6:                  // not building
            if Rules.CrateFirepowerMult != 1.0:  // RulesClass+0xF50
                rof = ftol(rof * <crate mult>)

    return rof
```

### Important details

1. **The 3-5 frame default** (`Random::RandomRanged(3, 5)`) is the inter-shot delay for
   any non-infantry burst weapon. At YR's 15-frame-per-second sim rate, that's
   approximately 200–333ms — fast enough to look like a rapid double-shot.

2. **The end-of-burst jitter** (`Random::RandomRanged(0, 2)`) prevents perfectly
   synchronized cadences across multiple identical units. Two Tesla Coils with ROF=80
   will *not* fire at exactly the same tick offsets — they drift by 0–2 ticks per
   reload cycle.

3. **Sentinel `-1` on BurstDelay**: if the modder sets `BurstDelay0=-1`, the engine
   treats it as "no override, use random 3-5". `0` means "fire on the very next tick";
   any positive integer is the literal frame count.

4. **Veterancy ROF doesn't make units fire faster** in a normal sense — it multiplies
   the cooldown by `VeteranCombat` (~1.1 default in `[General]`). But wait — that
   *increases* the cooldown? Re-read: the existing canonical doc says this multiplies
   firepower (damage), not cadence. **Looking at the live decomp more carefully:**
   the veterancy block reaches `iVar2 = Math__ftol()` and updates `iVar2` (rof) — but
   we cannot tell from the decomp alone whether the multiplier is the `VeteranCombat`
   firepower bonus (which should NOT apply to ROF) or a separate `VeteranROF` mult.
   **Status: MEDIUM confidence on this specific branch.** Open follow-up below.

5. **Naval barrel divide**: a ship with `BarrelCount=4` reloads in `ROF/4` ticks per
   burst-end. Compose with `Burst=4` and you get a "4 shots in rapid succession, then
   short reload" feel — exactly the Aegis Cruiser pattern.

6. **Crate powerup**: the firepower crate that grants a damage bonus also grants an
   ROF reduction (Rules `CrateFirepowerMult` < 1.0 means faster fire) — unless the
   firer is a building.

### Caller list (live verification)

`get_function_callers FUN_006fcfa0` returned no results — Ghidra's caller index here
is partial because GetROF is called exclusively through `vtable+0x318` (an indirect
call). The vtable slot is set in `TechnoClass`'s vtable initializer. **The single
known caller pattern** is `Fire_At` (`0x006FDD50`):

```
rof = (*(int (*)(int*))(this->vtable + 0x318))(this);   // GetROF
this->field_0x2F8 = rof;                                // active cooldown
this->field_0x2EC = g_CurrentFrameCounter;
```

For caller-trace verification, dispatches via `vtable+0x318` are inferred rather than
xref-able directly. This is the standard Tibsun/RA2 vtable pattern; treat indirect
binding as HIGH given the unambiguous offset in the existing `FIRE_AT_ANALYSIS.md`
doc, but be prepared to add overrides if any subclass overrides GetROF (likely candidates: `BuildingClass`, infantry — none found in vtable surveys to date).

---

## 4. `Fire_At` — burst-index cycling and FireTimer write (verified)

Within `TechnoClass::Fire_At` at `0x006FDD50` (decompiled in [`../../FIRE_AT_ANALYSIS.md`](../../FIRE_AT_ANALYSIS.md)):

```
// End-of-shot block, after bullet launch:
this.CurrentBurstIndex += 1;                  // +0x3B8
rof = this.vtable.GetROF(weaponIndex);
if this->field_0x298 != 0: rof /= 2;          // half-ROF modifier flag (verified, purpose TBD)
this.FireTimer.ROF = rof;                     // +0x2F8 — the active cooldown
this.FireTimer.StartFrame = g_CurrentFrameCounter;
this.FireTimer.Range = uStack_a8_scratch;     // some range tracking
this.FireTimer.InitialValue = rof;
this.CurrentBurstIndex %= weapon.Burst;       // wrap
```

So if `Burst=2`:
- pre-shot-1: `CurrentBurstIndex == 0`
- post-shot-1 increment: `1`, mod 2 → `1`
- pre-shot-2: `CurrentBurstIndex == 1`
- post-shot-2 increment: `2`, mod 2 → `0`

`GetROF` is consulted **after the increment but before the mod**, so it sees:
- post-shot-1: idx=1, `1 < 2 (Burst)` → branch 3 (mid-burst) → 3–5 frames
- post-shot-2: idx=2 (then mod), `2 < 2` is false → branch 4 (end-of-burst) → full ROF

### The `field_0x298` half-ROF modifier

Unresolved. It is read at end of Fire_At and halves the ROF result. Likely the "deploy fire" or "secondary weapon" flag — but specific consumers are not traced in this pass. Open follow-up below.

---

## 5. `GetFireError` — the gate that stops fires while the timer counts

Not decompiled in detail in this pass, but the contract is:

- Each tick, `Mission_Attack` (UnitClass/InfantryClass/BuildingClass) calls
  `GetFireError(weaponIndex, target)`.
- If `FireTimer.ROF` (`+0x2F8`) shows time remaining (compared against
  `g_CurrentFrameCounter - FireTimer.StartFrame`), it returns `FIRE_BUSY` and Fire is
  not dispatched.
- When the timer elapses, returns `FIRE_OK` (subject to range/facing/ammo checks).

This is what causes the visible "burst → reload → burst" cadence: the short ROF
return during mid-burst means `FIRE_BUSY` only blocks for 3–5 ticks; the full ROF
return at end-of-burst means `FIRE_BUSY` blocks for the weapon's stated ROF.

Full `GetFireError` documentation belongs in [`fire_at_pipeline.md`](fire_at_pipeline.md).

---

## 6. Spatial spread within a burst

### Non-Gattling weapons: NO engine-side spread

Every burst shot re-runs the entire aim pipeline (target position re-fetched, FLH
recomputed, atan2 redone). Visible "fan" patterns come from:

- **`Inaccurate=yes`** on the BulletTypeClass (`+0x2A2`) — adds per-bullet angle/distance scatter. Independent of burst.
- **`FlakScatter=yes`** on the BulletTypeClass (`+0x2A3`) — distance-proportional scatter.
- **Target motion** between shots — if the target moves a cell or two in 3–5 frames, the second shot is aimed at the new position.

These are **per-shot, not per-burst**: they run independently on every shot regardless
of burst index. So `Burst=3 + Inaccurate=yes` produces three independently-scattered
impact points, not a triangle pattern.

### IsGattling weapons: 8-octant scatter ring

When `IsGattling=yes` on the TechnoType (`+0xCD5`), `Fire_At` runs an extra block
before bullet launch that offsets the muzzle position:

```
if this.CurrentBurstIndex == 0:
    this.GattlingScatterIndex = Random.RandomRanged(0, 7)
else:
    this.GattlingScatterIndex =
        (this.GattlingScatterIndex + (8 / weapon.Burst)) & 0x80000007
    // negative-modulo fixup so the index wraps 0..7

offsetTable = DAT_00B0EAA8  // 8 entries × 12 bytes (X, Y, Z as int)
muzzleOffset = offsetTable[this.GattlingScatterIndex]
bulletOrigin = this.GetLocation() + muzzleOffset
```

The 8-entry octagonal pattern (radius 256 leptons = 1 cell):

| Index | X | Y | Z |
|---:|---:|---:|---:|
| 0 | +256 | 0 | 0 |
| 1 | +180 | +180 | 0 |
| 2 | 0 | +256 | 0 |
| 3 | -180 | +180 | 0 |
| 4 | -256 | 0 | 0 |
| 5 | -180 | -180 | 0 |
| 6 | 0 | -256 | 0 |
| 7 | +180 | -180 | 0 |

So Gattling bursts:
- **Shot 1:** random starting octant.
- **Shots 2..N:** step `8 / Burst` octants per shot.
- `Burst=2` → step 4 → opposite octants (a "left-right" alternation).
- `Burst=4` → step 2 → every 90° around the ring.
- `Burst=8` → step 1 → full ring per burst.

**This is the only built-in spatial spread for bursts.** Flak Trooper (which has
`Burst=2` on its weapon and the apparent "alternating barrels" look) does **not** use
this — its alternation is purely an artifact of the infantry firing animation.

### Confidence (Gattling scatter)

- **Content: HIGH** (existing doc decompiles the Fire_At Gattling branch and the table layout).
- **Identity: HIGH** (the DAT address `0x00B0EAA8` is referenced only from this block in Fire_At).
- **Binding: HIGH** (single writer + single reader; runs every Fire_At when `IsGattling=yes`).

---

## 7. Retargeting mid-burst (verified by inspection)

`Fire_At` is invoked per-tick by mission-attack dispatch. **The target pointer is
re-read every tick** through `this->Target` (a recomputed `AbstractClass*`). For
`Burst=2`:

- **Tick T:** GetFireError(target=T) → FIRE_OK → Fire_At fires shot 1 at T → CurrentBurstIndex=1 → ROF returned ~3–5 → FireTimer set.
- **Tick T+1..T+4:** GetFireError → FIRE_BUSY (timer counting down). No fire.
- **Tick T+~5:** Timer elapsed. GetFireError(target=T-still-alive?) →
  - **If T still alive:** FIRE_OK → Fire_At fires shot 2 at T's *new position* → CurrentBurstIndex=0 → full ROF.
  - **If T died between T and T+5:** target re-resolves to whatever `SelectWeaponAgainst`/mission-attack picks. If null, Fire_At is not called; `CurrentBurstIndex` stays at 1. The next engagement against any target picks up from idx=1, so its FIRST shot fires with the short delay and the SECOND shot gets the full ROF. **This is the carry-over edge case** — partial bursts are normal and CurrentBurstIndex is never reset.

### Open follow-up

Confidence on the "carry-over" claim is MEDIUM. No code path that writes
`this->field_0x3B8 = 0` outside the `% Burst` wraparound was found in this pass, but a
broader search (constructor, Limbo, target-change events) has not been performed. If
any such reset exists, the carry-over edge case doesn't apply.

---

## 8. Composition with other mechanics

| Mechanic | Composition with Burst |
|---|---|
| `Airburst=yes` (warhead) | Each of the N Burst shots can airburst into M sub-projectiles. Total: N × (1 + sub-spawns). See [`airburst.md`](airburst.md). |
| `Shrapnel=yes` / `ShrapnelCount=` (bullet) | Each shot's impact produces shrapnel independently. Composes additively with Burst. |
| `Inaccurate=yes` (bullet) | Per-shot scatter; each Burst shot scatters independently. |
| `FlakScatter=yes` (bullet) | Same — per-shot scatter, distance-proportional. |
| `IsSonic`/`UseFireParticles`/`UseSparkParticles`/`IsRailgun` | **Burst neutralized** — GetROF returns full ROF every shot. In retail, these weapons all use Burst=1 anyway, so no observable interaction. |
| `IsGattling` (TechnoType) | Composes — Gattling stages can have their own Burst values, and the scatter table activates per-burst-shot. See [`gattling_spool.md`](gattling_spool.md). |
| `Suicide=yes` (weapon) | Composes — owner dies on shot 1 if `Suicide=yes`; subsequent burst shots never happen because the owner is dead. See [`suicide_weapons.md`](suicide_weapons.md). |
| `DiskLaser=yes` (weapon) | Composes — same Fire_At code path, just creates DiskLaserClass instead of BulletClass. Retail shipping uses Burst=1 for DiskLaser, so untested in vanilla. |

---

## 9. Retail rulesmd.ini survey

Grep `^Burst=` in `ini/rulesmd.ini` returns 80+ matches. Representative sample with
shipping values:

| Weapon | Burst | Used by |
|---|---:|---|
| `[JumpCannon]` | 2 | Rocketeer |
| `[VirusShot]` | 2 | Virus |
| `[DesolatorShot]` | 2 / 3 | Desolator |
| `[TeslaTroopGun]` | 2 | Tesla Trooper |
| `[IFVMissile]` etc. | 2 – 4 | IFV variants |
| `[AegisGun]` | 4 | Aegis Cruiser |
| `[V3Airburst]` | varies | V3 Rocket missile split |

**Notable absences:** `[FlakTrackGun]` and `[FlakGuyGun]` are **not** `Burst>1` in
shipping YR despite their visible double-fire animation; that effect is driven by the
unit's firing animation (`FiringFrames`/`Anim=GUNFIRE`), not by `Burst=`. The visible
"two flak shots" you see come from the firing animation triggering twice in the
frame-sync system, not from the Burst mechanism.

**No `BurstDelay%d=` keys appear in shipping rulesmd.ini.** The InfantryTypeClass
parser supports it but retail uses random-3-to-5 for every burst-using infantry.

---

## 10. Edge cases & footguns

| Case | Behavior |
|---|---|
| `Burst=0` | Modular wrap `% 0` is undefined behavior (likely div-by-zero crash). Don't do it. The parser does not bounds-check. |
| `Burst=1` (default) | `CurrentBurstIndex < 1` is false on first shot (idx starts at 0, but the read happens after increment to 1), so GetROF takes branch 4 every shot → full ROF every time. Burst=1 = "always full reload." |
| `ROF=0` | GetROF returns 0 → FireTimer.ROF=0 → GetFireError never returns FIRE_BUSY → fires every tick. Used by some test/debug weapons; avoid in shipping content. |
| Mid-burst target death | CurrentBurstIndex persists; next engagement starts with carry-over idx. See §7 open follow-up. |
| Mid-burst out-of-range | Same as death — FireError returns FIRE_RANGE, Fire is not called, CurrentBurstIndex stays. |
| `Burst=2` on a sonic weapon | Burst silently neutralized (branch 2 short-circuit) → full ROF on every shot → effectively Burst=1. |
| Vet/elite firepower bonus | The ROF block in branch 4 *does* multiply rof by a vet/elite factor — but **whether this is `VeteranCombat` (firepower) or a separate `VeteranROF` multiplier is unverified.** Conservative interpretation: the same `VeteranCombat` value is used for ROF (so vets fire faster, since rof is the cooldown — but actually if it MULTIPLIES, cooldown is LONGER, so vets fire SLOWER, which is wrong). **Open follow-up.** |
| `BurstDelay0=-1` | Sentinel = "use random 3-5". |
| `BurstDelay0=0` | Fire on the very next tick (T+1). Visible as "near-simultaneous" double shot. |
| Building multi-barrel (`+0x2FC > 1`) | GetROF returns 1 unconditionally — buildings rapid-fire across barrels with no per-shot cooldown. Used by Tesla Coil, Prism Tower (in combination with Prism cascade). |

---

## 11. Open follow-ups

1. **Vet/elite ROF multiplier identity (Branch 4):** the decomp shows a multiply but the exact `RulesClass` field address feeding the multiply was not extracted in this pass. Resolve before implementing veterancy-modified ROF — should be a small targeted Ghidra session re-reading the FUN_006fcfa0 vet branch with `read_memory` on the float constant.
2. **`field_0x298` half-ROF modifier in Fire_At:** the half-divide at the end of Fire_At is conditioned on this flag. Purpose (and writer) unidentified. Probably tied to "secondary weapon firing while primary still on cooldown" but needs trace.
3. **CurrentBurstIndex reset audit:** confirm there is no implicit reset of `+0x3B8` on target-clear, limbo, deploy/undeploy, or unit-destruction (other than the modular wrap). If a reset exists, the §7 carry-over claim is wrong.
4. **`IsRailgun` particle-system field offset:** the existing doc names `+0x314` for `railgunParticleSys`. Not independently verified in this pass; flag for `rail_gun.md` to resolve.
5. **Multi-barrel `+0x2FC` vs `+0x69C`:** `MultiBarrelFlag` (the shortcut) vs `MultiBarrelIndex` (the running counter for visual selection). Their interaction is described in the existing doc but the `+0x69C` writer-set has not been re-confirmed. Flag for the building-multi-barrel section of `fire_at_pipeline.md`.

---

## 12. TS-legacy filter

All branches of GetROF are reachable in vanilla YR play:
- Branch 1 (multi-barrel): triggered by Tesla Coil, Prism Tower, Aegis, any naval ship.
- Branch 2 (sonic/particle): triggered by Sonic Tank (`[Sonic]`), Desolator deploy-beam, Tesla weapons, Rail Gun (Allied special).
- Branch 3 (mid-burst): triggered by every Burst>1 weapon.
- Branch 4 (end-of-burst with vet/naval/crate mults): triggered every full-reload.

`BurstDelay0..1` keys are parseable but unused in retail — not TS-legacy (they're YR-era parser additions for mod compatibility).

No TS-only gates identified.

---

## 13. Sources

- Live decompilation of `FUN_006fcfa0` (`GetROF`) at `0x006FCFA0` (read 2026-05-17).
- Existing canonical doc: [`../../BURST_WEAPON_FIRING_GHIDRA_REPORT.md`](../../BURST_WEAPON_FIRING_GHIDRA_REPORT.md) — content migrated, 3-axis confidence applied, "Current Rust Implementation Status" section deliberately omitted (this doc is the spec, not the gap report; gap analysis belongs in the implementation plan).
- Existing canonical doc for the Fire_At call site: [`../../FIRE_AT_ANALYSIS.md`](../../FIRE_AT_ANALYSIS.md).
- WeaponTypeClass struct: [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md).
- TechnoClass struct: [`../../TECHNOCLASS_STRUCT_LAYOUT.md`](../../TECHNOCLASS_STRUCT_LAYOUT.md).
- Gattling stage system: [`../../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`](../../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md).
