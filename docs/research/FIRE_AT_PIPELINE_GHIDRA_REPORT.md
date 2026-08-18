---
title: Fire_At Pipeline — Ghidra Research Report
date: 2026-04-23
---

# Fire_At Pipeline — Ghidra Research Report

**Primary function:** `TechnoClass::Fire_At` at `0x006FDD50`
**Confidence:** High (primary + upstream callers decompiled end-to-end)
**Active in YR:** Yes — every weapon discharge in a YR skirmish runs this pipeline.

This report complements `C:/Users/enok/Documents/ra2-rust-game-docs/FIRE_AT_ANALYSIS.md`,
which covers the *internals* of `Fire_At` itself. The scope here is the **pipeline around**
`Fire_At` — the per-tick callers that select a weapon, validate the shot, and dispatch
the actual call, plus verification spot-checks on critical claims from the existing doc.

---

## 1. Overview

Firing in gamemd.exe is a **decoupled three-stage per-tick pipeline**:

1. **Tick AI** (`InfantryClass::AI`, `UnitClass::AI`, `BuildingClass::AI`/`Mission_Attack`)
   drives the firing cadence — runs every frame and calls the class-specific
   `Fire_At_Target` helper when the unit has a target.

2. **Fire_At_Target** (class-specific: `InfantryClass::Fire_At_Target`,
   `UnitClass::Fire_At_Target`, plus the charge-mode block inside
   `BuildingClass::Mission_Attack`) performs weapon-index selection, fire validation,
   and animation synchronization. It calls `Fire_At` only when the unit is in the
   correct firing frame and all gates pass.

3. **Fire_At** (`TechnoClass::Fire_At` with virtual overrides for
   `AircraftClass::Fire_At`, `InfantryClass::Fire_At_Override`, and the unlabeled
   `FUN_00741340` for UnitClass) actually creates the bullet, sets the ROF timer,
   spawns visual effects, and returns the bullet pointer (or NULL for spawner/drain/
   disk-laser/suicide weapons).

Mission state machines (`FootClass::Mission_Attack`, `BuildingClass::Mission_Attack`)
do **movement and target tracking**. They do *not* call `Fire_At` directly — firing
happens every frame in the tick AI. This is a critical architectural point:
"Mission = Attack" does not mean "firing right now"; it means "allowed to fire and
chasing a target if needed."

---

## 2. Pipeline call graph

```
[Per-tick AI]                        [Fire_At_Target]            [Fire_At override]             [TechnoClass::Fire_At]
──────────────                       ────────────────            ──────────────────             ────────────────────────
InfantryClass::AI       ──▶ InfantryClass::Fire_At_Target ──▶ vtbl+0x3CC                 ──▶ TechnoClass::Fire_At
  0x0051BAB0                    0x005206B0                       InfantryClass::Fire_At_Override    0x006FDD50
                                                                   0x0051DF70

UnitClass::AI           ──▶ UnitClass::Fire_At_Target     ──▶ vtbl+0x3CC                 ──▶ TechnoClass::Fire_At
  0x007360C0                    0x00736DF0                       FUN_00741340 (UnitClass::Fire)

AircraftClass::Mission_Attack (via AircraftClass::What_Weapon_Should_I_Use 0x0041A9E0)
                                                              ──▶ AircraftClass::Fire_At   ──▶ TechnoClass::Fire_At
                                                                    0x00415EE0

BuildingClass::Mission_Attack ──▶ vtbl+0x3CC (own wrapper)   ──▶ TechnoClass::Fire_At
  0x0044ACF0                      (fires BOTH weapon 0 & 1 on IsChargeMode)
```

**Key virtual offsets driving the pipeline** (all on TechnoClass vtable unless noted):

| Offset | Role | Wrapper notes |
|--------|------|---------------|
| `+0x084` | `GetTechnoType()` | Returns `TechnoTypeClass*` — used heavily for per-type flags. |
| `+0x0B0` | `GetFLH(weapon_idx)` | Muzzle coordinates (FireLocation + barrel offset from ArtMD). |
| `+0x2E4` | `Get_Fire_Weapon_Idx(target)` | Returns which weapon to fire (0=primary, 1=secondary, gattling stage). Overridden by Infantry/Unit — both wrap `TechnoClass::SelectWeaponAgainst` at `0x006F3330` with a `ForceWeapon` check. |
| `+0x300` | `GetTargetCoords(out, weapon_idx)` | Target-side coord helper used for aircraft and arc pitch. |
| `+0x308` | `GetTurretFacing()` | 16-bit facing 0..0xFFFF, 0x4000 units per quadrant. |
| `+0x318` | `GetROF()` | Rate of fire including per-unit modifiers (veterancy ROF multiplier applied inside). |
| `+0x3C0` | `GetFireError(target, weapon_idx, fire_flag)` | Returns 0 = OK to fire, or a non-zero error code (see §4). |
| `+0x3C8` | `StopFiring()` | Called for FireOnce, Spawner-no-target, and out-of-range cases. |
| `+0x3CC` | `Fire_At(target, weapon_idx)` | The class-specific Fire_At override that ultimately reaches `TechnoClass::Fire_At`. |
| `+0x3F8` | `GetWeapon(weapon_idx)` | Returns `WeaponTypeClass*` for the given slot; overridden for IFV / garrison / charge-mode. |
| `+0x3FC` | `HasBurstRate()` | Checks if the distributed-fire burst-rate feature applies (IFV-style). |
| `+0x400` | `IsNaval()` | Gates naval damage multiplier + OccupantAnim fallback. |
| `+0x408` | `GetBarrelCount()` | Building multi-barrel cycling divisor. |
| `+0x45C` | Decloak/reveal on cloaked-target error | Called when `GetFireError == 9`. |
| `+0x4E4` | "Is still orienting" check (UnitClass) | When true with `FireError == 0 || 2`, the unit enters "aim" state instead of firing. |

---

## 3. SelectWeaponAgainst — weapon index resolution

**Address:** `0x006F3330` — `TechnoClass::SelectWeaponAgainst(target) → int weapon_index`

Returns the weapon slot to fire (0 = primary, 1 = secondary, or 2N / 2N+1 for
gattling stages). Called indirectly via the vtbl+0x2E4 wrapper; the wrappers at
`0x005218E0` (InfantryClass) and `0x00746CD0` (UnitClass) short-circuit to return
`ForceWeapon` (typeClass+0x6A8) when `ForceWeapon_Active` (typeClass+0x6AC) is set —
this is the mechanism INI uses to pin a weapon index for specific unit-type modes.

### Decision ladder (verified from decompilation)

Evaluated top to bottom; first matching rule returns.

1. If type has **Gattling** (`typeClass+0xCD5`) and `CurrentWeaponNumber` already
   assigned: return that cached index. (Gattling stages are sticky between frames.)
2. If unit **is naval** (`vtbl+0x400 != 0`): skip to the fallthrough (return 0). Naval units only use primary.
3. Get secondary weapon (`GetWeapon(1)`). If NULL, return 0 (primary).
4. Get primary weapon (`GetWeapon(0)`). If NULL → fallthrough (return 0).
5. If secondary's `weapon+0x136` (`NeverUse`) is set, no fallback to secondary is possible → return 0.
6. If **Airstrike** (`this+0x82`) set AND type has a fixed airstrike weapon
   index (`typeClass+0xD50 != -1`): return `typeClass+0xD50`.
7. If type is **Gattling** (`typeClass+0xCD5`):
   - Base weapon = `CurrentGattlingStage * 2` (ground weapon).
   - If target's Projectile has `AA=yes` (`projectile+0x2A4`) and target is in-air
     (`piStack_4 bit 0`) and target `vtable+0x54` (IsInAir) returns true:
     return `stage*2 + 1` (AA weapon for that stage).
   - Else return `stage*2` (ground weapon).
8. If primary warhead `warhead+0x16C` (`BombDisarm`/`Airburst`-gate-per-class) is set, the
   following secondary-routing checks are SKIPPED and flow proceeds to verses-based
   selection. Otherwise:
   - If secondary warhead has `warhead+0x15B` (Wall/Building target flag) AND
     target is RTTI type 6 (building): return 1.
   - If primary is **DrainWeapon** (`weapon+0x142`) AND target type has
     `Drainable` (`typeClass+0x5EF`) AND `this->ForceDrain(this+0x1CC)==0` AND
     target is not allied: return 1.
   - If primary is **AreaFire** (`weapon+0x150`) AND `vtbl+0x184` (action) == 0x10
     (force-fire cell): return 1.
   - If `this` is a building (type 6) AND type has `NukeAmmo` (`typeClass+0xCD1`):
     return 1 (nuke silo uses secondary for launch).
   - If target is allied building with `Occupiable` (`target.type+0x1575`) AND
     primary warhead has `warhead+0x158` (cell-targeting flag): return 1.
   - If `this` is RTTI type 2 (building) AND `typeClass+0xCAA` (garrison-sec flag):
     return 1.
   - Terrain object branch (type 0xB) with specific garrison/sensor flags: return 1.
9. **Verses-based fallback** — final stage:
   - Look up `weapon->warhead+0xA0 + target.armor*8` (Verses double).
   - If primary Verses is zero vs this armor AND secondary Verses is nonzero: return 1.
   - If target is in-air AND (secondary's projectile is AA): return 1 (gattling-lite fallback).
   - Otherwise call `vtbl+0x2E8` (custom per-class selector) — if that returns not -1,
     use its result.
10. Default: return 0 (primary).

**Rust status:** `src/sim/combat/combat_weapon.rs:96` implements the verses-based
core (rule 9) and a simplified primary/secondary fallback. Rules 1 (sticky gattling
stage), 6 (airstrike), 7 (gattling AG/AA split), 8 (multiple secondary-route
conditions), and the force-weapon override are **not yet implemented**.

---

## 4. GetFireError — shot validation

**Address:** `0x006FC0B0` — `TechnoClass::GetFireError(target, weapon_idx, fire_flag) → FireError`

Returns 0 if the unit may fire *right now*. Non-zero codes describe *why* firing is
blocked. Upstream callers (`Fire_At_Target`, `BuildingClass::Mission_Attack`) branch
on the code to decide whether to stop firing, keep turning, play decloak voice, etc.

### FireError return codes (verified from decompile — values observed in upstream switches)

| Code | Meaning | Upstream handling |
|------|---------|-------------------|
| `0` | OK — fire now | Proceed to `vtbl+0x3CC` |
| `1` | Reloading — `this->Ammo == 0` | Aircraft returns to base; ground units wait |
| `2` | Not-yet-aligned / facing mismatch | UnitClass flips to aim state + sets facing timer |
| `3` | Busy (FireTimer not expired, or particle system still active, or spawner has no live spawns, or LocomotorTarget mismatch, or deploy-recharge active) | Skip this frame |
| `5` | Illegal (target invalid, target in limbo, disguise mismatch, iron curtain active, target undamageable, wall/cliff block, airstrike target constraint fails, `Verses==0`, many other structural gates) | Varies by caller — often drop target |
| `6` | Out of range or target shrouded beyond sight | UnitClass stops firing if range is negative |
| `8` | Cloak gating / special assertion | Clear firing flag |
| `9` | Target cloaked and weapon requires visible target | Call `vtbl+0x45C` (reveal/decloak check) |
| `11` / `0xB` | Reserved — handled alongside 8 | Clear firing flag |

**Critical gating logic (checked in order) — each returns the listed code:**

1. `target == NULL` → 5
2. `this+0x2DC != 0` (IronCurtain/ForceShield timer) → 5
3. `vtbl+0x1D8` (is disabled/sleeping) → 3
4. `this.LocomotorTarget == target` (already tracking) → 3
5. `vtbl+0x1D4` (is in transport?) → 5
6. `this+0x1C8` (some disable flag), `IsSinking`, `ArchiveTarget == self`, `Target == self` → 5
7. `target+0x81 != 0` (target in limbo) → 5
8. Airstrike (`this+0x298` actually `field_0x8 + 0x290` equivalent — see existing doc) AND target-type has `ImmuneToAirstrike` (`typeClass+0x690`) → 5
9. ShroudCheck: target visibility 5 (shrouded) — if `GetWeaponRange > 0`, return 6; else if not allied with target's owner, return 6.
10. `GetWeapon(target)` returns NULL → 6
11. Weapon `+0x14F` flag (GuardRange / TerrainFire) + cell-validity helper fails → 6
12. DrainWeapon (`weapon+0x142`) + target already draining (`target+0x74 != 0`) → 5
13. DrainWeapon + target type lacks `Drainable` (`type+0x5EF`) → 5
14. Warhead `+0x16D` (wall-destroyer?) + target is building + target has `UndeployInto` (`type+0xD35`) → 5
15. Warhead `+0x15B` (cell/bridge-only?) + target on bridge (target+0xB9) + target is infantry → 5
16. Warhead `+0x15B` + target is "infantry cell target" — nested conditions → 5
17. Airstrike (`this+0x82`) + weapon `+0x143` (airstrike-compatible?) mismatch → 5
18. Spawner weapon (`weapon+0x131`): on bridge → 6; `vtbl+0x380` returns nonzero → 6; `SpawnManagerClass::CountAliveSpawns == 0` → 3
19. Existing particle/sonic systems for this weapon still active (`this+0x304`, `+0x308`, `+0x314`, `Wave`) → 6 (on GetWeapon(0) re-read path)
20. Target invulnerable (`target->vtbl+0x54`) + weapon projectile is not IgnoreInvul (`+0x2A4`): return 3 if LocomotorTarget != target else 5
21. Target in air + not aircraft-type + projectile not AA (`+0x2A4`) → 5
22. No techno target (cell/terrain) + projectile not AG (`+0x2A5`) → 5
23. **Burst-delay check for infantry** — if not yet firing and type is infantry (vtable+0x2C == 1):
    - `delay_idx = CurrentBurstIndex % weapon.Burst`
    - If `delay_idx < 2`: read `type+0xE40+delay_idx*4` (= `BurstDelay0` or `BurstDelay1`).
    - If `delay != -1` AND `this+0x2A0 (burstDelayCounter)` is set AND it mismatches the expected delay → 3.
24. Otherwise FireTimer: `remaining = this+0x2F4 - (g_CurrentFrameCounter - this+0x2EC)`; if `remaining > 0` → 3.
25. Particle system still alive re-check → 3
26. `this->Ammo == 0` → 1
27. `weapon+0x133` (DecloakToFire) + `this->CloakState != 0` + (not type==2 or CloakState==2) → 9
28. `type+0xD27` (AI_Ignore or similar) → 8
29. Warhead `+0x159` (Cliff-gate) + firer not on ground + cell-unreachable helper fails → 5
30. Warhead `+0x159` + in-air target with reservation timer active (`target+0x1A6 > g_CurrentFrameCounter`) → 5
31. Target invulnerable + warhead `+0x159` → 5 (via vtbl+0x160)
32. Warhead `+0x155` (MindControl) + `CaptureManager::CanCapture == false` → 5
33. `Verses[target.armor] == 0` on the warhead damage table → 5
34. Warhead `+0x16E` (RequiresDamageForCapture?) + target.Health == 0 → 5
35. Warhead `+0x157` (HealsIfUndamaged?) + target.Health != 0 → 5
36. Target `+0x3CD` (temporal/fortified?) → 5
37. Bridge-level mismatch + elevation flags + warhead `+0x159` (Z-delta > `2 * DAT_00b0eb34`) → 5
38. Type `+0xD97` (deploy-and-fire gate) + `vtbl+0x380` true → 5
39. If `fire_flag != 0`: `vtbl+0x3A8` (per-class last-chance gate) returns false → 8
40. Return 0.

**Observation:** Many of these conditions appear to be dormant TS-legacy features
or extreme edge cases that rarely trigger in YR (item 8 airstrike immunity, items
14–16 wall/bridge warhead flags, item 29 cliff-clearance). The *common-case*
conditions that fire in every skirmish are: target validity (1, 7), range (9),
reload (24, 26), verses gate (33), cloak (27, 31–32), and ammo (26).

**Rust status:** The Rust pipeline at `src/sim/combat/combat_fire_gate.rs:31` and
`combat_targeting.rs:67` covers roughly items 1, 24, 26 and a simplified verses
gate (33). Items 2, 3, 20, 21, 22, 27, 33's-zero-verses-distinction, and
many others are not currently modeled.

---

## 5. Fire_At_Target — the animation-synchronized dispatch

### InfantryClass::Fire_At_Target (0x005206B0)

Called every tick from `InfantryClass::AI` when a target is set. This is where the
**animation frame drives the fire moment**:

```
if (target == NULL) { clear_firing_flag; return; }
weapon_idx = vtbl+0x2E4(target)
if (!already_in_fire_anim) {
    err = vtbl+0x3C0(target, weapon_idx)        // GetFireError
    switch (err) {
        case 0:   select infantry sequence (FIRE_UP / FIRE_PRONE / etc via vtbl+0x558)
                  set firing_flag = 1
        case 5:   if GetWeaponRange < 0 → scatter or StopFiring
        case 9:   vtbl+0x45C (decloak/reveal handling)
    }
}
fire_frame = type+0xE40 + variants for prone / deploy / turret-anim
if (current_anim_frame == fire_frame && firing_flag) {
    err = vtbl+0x3C0(target, weapon_idx)         // re-check — target may have moved
    if (err == 0) vtbl+0x3CC(target, weapon_idx) // ← Fire_At_Override → TechnoClass::Fire_At
    else          revert to appropriate wait state
}
if (target_still_valid && weapon.Speed < Rules+0x16C0) CellClass::Scatter_Objects  // melee nearby friendlies
```

The re-check at `fire_frame` is what prevents infantry from "firing at nothing"
when the target moves or dies during the windup animation. If `err != 0` at the
re-check, the unit aborts the shot and returns to an idle/aim state.

### UnitClass::Fire_At_Target (0x00736DF0)

Different pattern — vehicles fire immediately when aligned, not on anim frame:

```
if (target == NULL) { maybe gatling decay; return; }
if (GetWeapon(0) == NULL) { maybe gatling decay; return; }
weapon_idx = vtbl+0x2E4(target)
err = vtbl+0x3C0(target, weapon_idx, 1)         // GetFireError

if ((err == 0 || err == 2) && vtbl+0x4E4) {    // still orienting
    vtbl+0x1E8(0x10, 0)                         // set "aiming" state
    return
}

switch (err) {
    case 0:
        if (!type+0xE10 IsUnit-flag) firing_flag = 0     // immediate-fire types clear the latch
        if (type+0xE18 or type+0xE19 muzzle-flash)
            set muzzle flash timer (this+0xF8..+0x10C — frame + 5-frame duration)
        vtbl+0x3CC(target, weapon_idx)           // ← UnitClass::Fire_At → FUN_00741340 → TechnoClass::Fire_At
        break
    case 2: set facing timer (RateTimer) to continue turning
    case 5: if GetWeaponRange < 0 → StopFiring unless target is an infantry in good health
    case 6: SpawnManagerClass::ClearAllTargets (spawner can't reach)
    case 9: firing_flag=0; CanFireAt true → vtbl+0x45C (reveal cloaked)
    case 8/11: firing_flag = 0
}

if (type is gattling) {
    if (err ∈ {0, 2, 3, 4}) TechnoClass::IncreaseGattlingStage(1)    // charging
    else                    TechnoClass::UpdateGattlingStage(1)      // decaying
    if (GattlingValue > 0) this+0x148 += 1                           // advance gattling counter
}
```

### BuildingClass::Mission_Attack (0x0044ACF0) — dual-path

Two completely separate code paths gated by `type+0x16B8` (IsChargeMode, Tesla-coil):

**Path A — normal turret** (the charge-mode flag is off):
- `No target → StopFiring, clear garrison fire index (this+0x664)`.
- `weapon_idx = vtbl+0x2E4(target)`; `err = vtbl+0x3C0(target, weapon_idx, 1)`.
- **If err == 2** and type has `OmniFire` (`type+0x16C5`) and `HasBurstRate` true,
  checks facing timer — if already over the turn-in-place delay (`type+0x71C`),
  updates facing immediately and re-checks FireError. Else falls through.
- **If err < 0xB**, jumps via the 11-entry table at `PTR_LAB_0044b728` (one branch
  per error code). This is the building-specific fire-error jump table.
- The `err==0` branch calls `vtbl+0x3CC(target, weapon_idx)`, which for buildings
  is the garrison round-robin (round-robins through `this+0x664` garrison slots)
  and calls the occupant's `TechnoClass::Fire_At` with the occupant's weapon.

**Path B — charge mode** (Tesla-coil-like, `type+0x16B8 == 1`):
- Tracks a charge state at `this->field_0xBC` (0 = idle, 1 = charging).
- Idle + has target + has power (or PowerOff not required): set up charge timer
  using the standard vtbl+0x4E8 timer slot; charge for `>= 0x2001` rate-timer
  units (about 6 seconds), then advance to state 1.
- Charging (state 1): on each tick, re-check `vtbl+0x3C0`:
  - err ∈ {5, 6, 8}: StopFiring, reset charge state.
  - err == 0: call `vtbl+0x3CC(target, 0)` **then** `vtbl+0x3CC(target, 1)` —
    **the building fires both its primary AND secondary weapons on release**. This
    is how Tesla Coils discharge with two bolts.

**Rust status:** The building attack logic at `src/sim/combat/mod.rs:626` (garrison)
and building-specific paths implement round-robin fire-index (Phase 3c at
line 1247). Charge mode (Tesla pre-charge + dual-weapon discharge) is not modeled.

---

## 6. TechnoClass::Fire_At — verifications and clarifications to existing doc

The existing report `ra2-rust-game-docs/FIRE_AT_ANALYSIS.md` covers Fire_At's 13 phases
in detail. I spot-checked the following critical claims by re-reading the binary:

### 6.1 Gatling scatter table at `0x00B0EAA8` — VERIFIED

Re-read of `Fire_At 0x006FE000..006FE07D`: the 8-entry, 12-byte-per-entry table is
initialized on first entry (gated by `DAT_00b0eb30 & 1`). Exact values:

| Index | X | Y | Z | Byte value (X/Y) |
|-------|---|---|---|------------------|
| 0 | `0x100` (256)   | `0x0`           | `0` | `+256, 0` |
| 1 | `0xB4` (180)    | `0xB4` (180)    | `0` | `+180, +180` |
| 2 | `0x0`           | `0x100` (256)   | `0` | `0, +256` |
| 3 | `0xFFFFFF4C` (-180) | `0xB4` (180) | `0` | `-180, +180` |
| 4 | `0xFFFFFF00` (-256) | `0x0`        | `0` | `-256, 0` |
| 5 | `0xFFFFFF4C` (-180) | `0xFFFFFF4C` (-180) | `0` | `-180, -180` |
| 6 | `0x0`           | `0xFFFFFF00` (-256) | `0` | `0, -256` |
| 7 | `0xB4` (180)    | `0xFFFFFF4C` (-180) | `0` | `+180, -180` |

Z is always 0 — gatling scatter is horizontal-only. The pattern is a regular
octagonal ring at 256 leptons radius (cardinal points at 256, diagonals at
180 ≈ 256·cos(45°)).

**Scatter index advance:**

- First shot of a burst (`CurrentBurstIndex == 0`): `this+0x2A0 = Random_RandomRanged(0, 7)` — random start.
- Subsequent shots: `this+0x2A0 = (this+0x2A0 + 8/Burst) & 0x80000007` with sign-correction to keep positive.

So the step between scatter entries is `8/Burst` — Burst=1 steps by 8 (one entry
per visible cycle of random starts), Burst=2 steps by 4, Burst=4 by 2, Burst=8 by 1.
This produces a coherent rotating pattern within a burst.

### 6.2 Pre-damage subtraction (existing doc §7) — VERIFIED with correction

At `006FE4D0`-ish (phase 7 of existing doc):

```c
// piVar7 = bullet, piStack_38 = target (techno), iStack_90 = weapon
if ((bullet+0xB4 byte == 0)                              // NOT flagged as "bright/aircraft-fire-marked"
    && (BulletType+0x2A2 == 0)) {                        // NOT Inaccurate
    target = this->Target;                                // NOTE: uses this->Target, not the parameter
    if (target is techno)
        predmg = FUN_006fdb80(target, weapon);
        *(target + 0x70) -= predmg;                      // subtract from EstimatedHealth
}
```

The existing doc is correct. **Important clarification:** the pre-damage is applied
to `this->Target` (the firer's Target field), **not** the `in_stack_00000004` target
parameter passed to Fire_At. For typical unit firing these are the same, but for
force-fired shots and some secondary-weapon paths they can diverge — the bullet is
aimed at the parameter target, but the damage reservation is booked against the
firer's Target field. This could cause the "pre-subtract" to miss when a unit is
scripted to fire at a cell but retains a techno target in its Target slot.

**`target+0x70` is `EstimatedHealth`**, distinct from `target+0x6C` (`Health`).
EstimatedHealth is the "reserved" HP used by multi-unit threat scoring (see
`TARGET_ACQUISITION_GHIDRA_REPORT.md`) to prevent overkill when many units
shoot the same target in the same tick.

### 6.3 FireTimer fields — VERIFIED

After a successful Fire_At, these four fields are written:

```c
this+0x2F8 = ROF        // ROF value — also halved if this+0x298 byte is set
this+0x2EC = g_CurrentFrameCounter    // fire start frame
this+0x2F0 = uStack_A8                // NOT "duration" — this is the clamped distance/range value computed earlier
this+0x2F4 = ROF        // initial-ROF (the value GetFireError reads for the timer check)
```

**Correction to existing doc §6:** `this+0x2F0` is **not a "computed duration"** —
the value stored is `uStack_A8`, which by the time of this assignment holds a
bullet-range / distance-clamped value (min of `GetMaxBallisticRange(weapon)` — a
speed-derived range limit, **not** the INI `Range=` value — and `actual_distance/2`).
(corrected 2026-05-29: was "min of `weapon.Range` and `actual_distance/2`"; binary via
`decompile_function 0x006FDD50` shows `uStack_a8 = FUN_00773070()` = `GetMaxBallisticRange`
then clamped by `iVar9/2`; `FUN_00773070` is speed-based, not the INI Range= field —
ROOT_CAUSE: MISLEADING)
Its purpose is not the ROF gate (that uses `+0x2F4` and `+0x2EC`). It may feed
visual effects like the railgun beam length or the sonic wave travel distance;
further research needed.

GetFireError's timer check (verified at `0x006FC91E`):

```c
int remaining = this+0x2F4;
if (this+0x2EC != -1) {
    int elapsed = g_CurrentFrameCounter - this+0x2EC;
    if (remaining <= elapsed) return OK;   // gate cleared
    remaining -= elapsed;
}
if (remaining != 0) return BUSY;          // err 3
```

So `+0x2F4` is the authoritative ROF duration; `+0x2F8` is a mirror also written
but not read by the gate. The gate uses `+0x2EC` as the epoch.

### 6.4 Weapon index dispatch for Fire_At overrides — clarified

`TechnoClass::Fire_At` takes its `weapon_index` from the `in_stack_00000008` second
stack parameter. Both overrides (`AircraftClass::Fire_At`, `InfantryClass::Fire_At_Override`,
and the unlabeled `FUN_00741340` for UnitClass) pass it straight through after
their own pre/post-processing. The override entry points are what `vtbl+0x3CC`
resolves to for each class.

### 6.5 AircraftClass::Fire_At (0x00415EE0) — extra behavior

The aircraft override wraps `TechnoClass::Fire_At` with these additions:

1. If `this+0x118` (Paradrop payload) is set, calls `AircraftClass::Drop_Payload()`
   and returns without firing — parabombs / paratroopers / engineers come out
   instead of a bullet.
2. After a successful Fire_At, if the bullet is Arcing (`bulletType+0x2DC == 0`)
   with certain conditions, re-computes the velocity vector using aircraft speed
   and facing — the bullet's initial velocity is `(aircraft_speed · dir_unit_vec)`
   so it inherits forward motion from the plane. This makes strafing runs land
   ahead of where the aircraft is currently positioned.
3. If `bulletType+0x2DC == 1` (Dropping), uses aircraft altitude and atan2 to
   set a downward-angled velocity (bombs and paradrops).
4. If the firer is human-controlled and not shrouded, calls
   `MapClass::RevealAroundCell(location, Rules+0x18 /* SightRange */, owner, ..., 1)`
   — aircraft fire reveals shroud around the plane (not the target), twice
   (spread of 1 cell with both the "update" and "reveal" flags).

### 6.6 InfantryClass::Fire_At_Override (0x0051DF70) — extra behavior

Wraps `TechnoClass::Fire_At` with:

1. Clears `this+0x68D` (a "pending fire" flag) before calling.
2. If Fire_At succeeded AND firer is not in limbo AND type has `Crawls` or
   `IsLeech` (`typeClass+0xEBF`) AND `this+0xBF*4 == 0` (not prone / not digging):
   sets `this+0x1B5*4 = 300` (a cooldown or retaliation timer, ≈ 300 frames = 20 seconds).
   Then if type is infantry (1) or cell (0xF), calls `vtbl+0x1E8()` — likely a
   post-fire stance reset or facing update for infantry-specific anim states.

---

## 7. Special case: what drives the UnitClass::Fire_At wrapper (FUN_00741340)

This function is unnamed in Ghidra but is the UnitClass vtbl+0x3CC target. Its body:

```c
weapon = GetWeapon(weapon_idx);
if (weapon == NULL) return 0;
if (weapon_idx == 0) {
    // Infantry-burst-delay carry-over (for infantry-driving-vehicle types):
    delay_idx = CurrentBurstIndex % weapon.Burst;
    if (delay_idx < 2) delay_value = type+0xE40+delay_idx*4;  // BurstDelay0 or BurstDelay1
    else delay_value = -1;
    check_delay = (this[1].field_0x1a0 != -1) && (delay_value != -1);
}
fire_can_dispatch = vtbl+0x3C0(unaff_retaddr, weapon_idx, 1);  // GetFireError equivalent
if (fire_can_dispatch == 0) {
    bullet = TechnoClass::Fire_At(this);
    if (bullet != 0) {
        ammo_threshold = type+0x684;                        // AmmoThresholdForReload
        if (ammo_threshold > 0 && this.Ammo < ammo_threshold && !type+0xD24 (NoPreAmmo)) {
            FUN_006fb080();                                 // Pre-ammo-drop-warning
        }
        if (!check_delay && type+0xE5D > 0) {
            this[1].field_0x1a0 = type+0xE5D * 2 - 1;       // reset burst-delay counter
        }
        if (bullet->type+0x2EC) FUN_0046b280();             // BulletTypeFlag → ???
        if (type+0xD32) {                                    // one-shot-reload flag
            uVar2 = iStack_8+0x13C;                          // visual marker
            this+0x1EC = g_CurrentFrameCounter;
            this+0x1F0 = unaff_EDI;
            this+0x1F4 = uVar2;
        }
    }
}
return bullet;
```

The point: `FUN_00741340` is the **unit-side burst-delay scheduler**. It's where
`BurstDelay0` / `BurstDelay1` (from `rules.ini` `[InfantryType] BurstDelay*`) take
effect for units that are driven by infantry (GGI, tesla trooper). The burst delay
is tracked at `this[1].field_0x1a0` (offset ~`0x694` on InfantryClass base, need
struct-size verification).

---

## 8. Integration points — end-to-end flow (tick-by-tick)

Per game tick (15 Hz), for each techno with a target:

```
1. TickAI calls Fire_At_Target (class-specific)
2. Fire_At_Target:
   a. weapon_idx = SelectWeaponAgainst(target)
   b. err       = GetFireError(target, weapon_idx, fire_flag)
   c. If err == 0 AND (for infantry) anim_frame == fire_frame:
      → call vtbl+0x3CC (class Fire_At override)
3. Class override:
   → does class-specific pre-work (paradrop check / burst delay / etc)
   → calls TechnoClass::Fire_At(this)
4. TechnoClass::Fire_At:
   → re-validates (special weapon types short-circuit: Suicide, Spawner, Drain)
   → computes muzzle pos (GetFLH) and damage (with veterancy / naval / IC / airstrike modifiers)
   → creates BulletClass (or DiskLaser) unless weapon is non-projectile (sonic, laser, electric bolt)
   → pre-subtracts predicted damage from target+0x70 (EstimatedHealth) unless Inaccurate/Bright
   → computes velocity vector (arcing / inaccurate scatter / homing pitch)
   → launches bullet via vtbl+0x1F0
   → sets ROF timer (this+0x2EC, +0x2F4, +0x2F8)
   → modulos CurrentBurstIndex by weapon.Burst
   → creates muzzle flash anim + plays weapon Report sound
   → creates special visuals (laser / wave / electric bolt / rad beam)
   → vtbl+0x390 (MarkFired), vtbl+0x124(2) (FireVoice)
   → handles RevealOnFire, LimboLaunch special-case, FireOnce StopFiring
5. Class override post-work (aircraft: reveal shroud; infantry: stance reset)
6. Fire_At_Target updates gattling stage (if applicable)
7. Next tick: GetFireError returns 3 (BUSY) until the ROF timer expires
```

The **bullet** itself is a separate per-tick simulated entity. Warhead effects
(damage, AoE, InfDeath, Radiation, MindControl, etc.) are applied by
`BulletClass::Detonate` on impact — **not** inside `Fire_At` (except the
pre-damage subtraction and the Sonic/Laser instant-damage paths, which apply at
fire time).

---

## 9. Current Rust implementation status

| Pipeline stage | Rust location | Coverage |
|----------------|---------------|----------|
| Tick AI firing loop | `src/sim/combat/mod.rs:259..1337` (`tick_combat_with_fog`) | **Structurally correct** — 7-phase tick mirrors the decoupled per-tick model. |
| Target acquisition | `src/sim/combat/combat_targeting.rs:67..150` | Partial — nearest-hostile-in-range, no threat scoring. |
| SelectWeaponAgainst | `src/sim/combat/combat_weapon.rs:86..230` | Verses-based primary/secondary fallback only. **Missing:** gatling stages, force-weapon, airstrike weapon-index, drain/area-fire/suicide routing, wall-warhead routing. |
| GetFireError gates | `src/sim/combat/combat_fire_gate.rs:31..128` | ~10 of the ~40 gate conditions implemented (ammo, power, garrison, locomotor-busy). No verses-zero gate, no cloak gate, no IronCurtain gate, no invulnerability check, no burst-delay gate. |
| Fire_At_Target (anim-sync dispatch) | `src/sim/combat/mod.rs:1074..1200` | Turret alignment via lepton-facing check. **No** animation-frame gating — fires whenever aligned. Infantry anim sync is not modeled. |
| Fire_At (bullet creation) | — | **Not implemented.** `SimFireEvent` is emitted for rendering/sound but no projectile entity, no trajectory, no detonation. Damage is applied directly to target (line 1259). |
| Muzzle flash anim | `src/sim/combat/mod.rs:1162` | Fire event captures `weapon_slot`, `garrison_muzzle_index`, `occupant_anim` — deferred to render. |
| Fire sound (Report=) | `src/sim/combat/mod.rs:1159` | Pushed to sound sink as `SimSoundEvent::WeaponFired`. |
| RevealOnFire | `src/sim/combat/mod.rs:1173` | 3-cell radius shroud clear at fire location. |
| Pre-damage (EstimatedHealth) | — | **Not implemented.** No "damage reservation" concept. |
| Gatling scatter table | — | **Not implemented.** No gatling stage tracking. |
| Gatling stage charge/decay | — | **Not implemented.** |
| Burst cycling (CurrentBurstIndex % Burst) | `src/sim/combat/mod.rs:116..127` | Implemented (tracks `burst_remaining` on AttackTarget). |
| BurstDelay0/1/2/3 | — | **Not implemented.** Inter-shot delay currently uses a single value. |
| ROF / reload timer | `src/sim/combat/mod.rs:1400..1410` | Implemented (converts frames to ticks; garrison ROF/count division). Matches `+0x2EC/+0x2F4` gate model. |
| Veterancy firepower multiplier | — | **Not implemented.** Base damage only. |
| IronCurtain / Airstrike damage multiplier | — | **Not implemented.** |
| Naval damage multiplier (vtbl+0x400) | — | **Not implemented.** |
| Special weapons (laser/sonic/etc.) | Parsed from INI but not rendered | Sprite/visual effects not produced from fire events. |
| Aircraft paradrop path | — | **Not implemented.** |
| BuildingClass IsChargeMode (Tesla) | — | **Not implemented.** |

---

## 10. INI keys referenced by this pipeline

### Primary keys used every shot

| Key | Section | Default | Role in pipeline |
|-----|---------|---------|------------------|
| `Damage` | `[WeaponType]` | 0 | Base damage; scaled by veterancy / IC / naval / airstrike |
| `ROF` | `[WeaponType]` | — | Rate of fire — frames between shots (gate written to `+0x2F4`) |
| `Range` | `[WeaponType]` | — | Range check in GetFireError step 9 |
| `Burst` | `[WeaponType]` | 1 | `CurrentBurstIndex % Burst` modulo; scatter step = `8/Burst` |
| `Speed` | `[WeaponType]` | — | Velocity vector magnitude for non-arcing bullets |
| `Projectile` | `[WeaponType]` | — | `weapon+0xA0` → BulletTypeClass |
| `Warhead` | `[WeaponType]` | — | `weapon+0xAC` → WarheadTypeClass, used for Verses + effects |
| `Report` | `[WeaponType]` | — | Fire sound played via `VocClass::PlayAt` |
| `AmbientDamage` | `[WeaponType]` | 0 | Applied to nearby cells on fire (not a bullet mechanic) |

### Gate-affecting keys (GetFireError)

| Key | Role |
|-----|------|
| `MinimumRange` | Part of GetFireError step 9 range check (close-range block) |
| `NeverUse` (secondary) | `weapon+0x136` — forbids secondary-weapon fallback |
| `DecloakToFire` | `weapon+0x133` — triggers FireError 9 / decloak handling |
| `Suicide` | `weapon+0x144` — Fire_At short-circuits with vtbl+0x16C SetTarget(self) |
| `DrainWeapon` | `weapon+0x142` — SelectWeaponAgainst rule 8; GetFireError step 12 |
| `AreaFire` | `weapon+0x150` — SelectWeaponAgainst rule 8 |
| `Spawner` | `weapon+0x131` — GetFireError steps 18–19 |
| `LimboLaunch` | `weapon+0x132` — Fire_At post-phase cleanup |
| `FireOnce` | `weapon+0x135` — Fire_At post-phase StopFiring |
| `RevealOnFire` | `weapon+0x137` — Fire_At post-phase shroud reveal |

### BulletType (Projectile=) keys

| Key | Role |
|-----|------|
| `Arcing` | BulletType+0x29B — trajectory branch |
| `Dropping` | BulletType+0x29C — aircraft bomb path |
| `Inaccurate` | BulletType+0x2A2 — scatter applied; disables pre-damage subtraction |
| `FlakScatter` | BulletType+0x2A3 — proportional scatter mode |
| `Inviso` | BulletType+0x29E — invisible; sets bullet "InfDeath" flag under conditions |
| `Level` | BulletType+0x236 — flat trajectory (ignores Z delta in pitch) |
| `Floater` | BulletType+0x295 — custom gravity via `FUN_0048acf0` |
| `ROT` | BulletType+0x2DC — rate of turn; 0 = unguided ballistic |
| `AA` / `AG` | BulletType+0x2A4 / +0x2A5 — air/ground targeting gates |

### Rules keys

| Key | Section | Role |
|-----|---------|------|
| `Gravity` | `[General]` | RulesClass+0x16B8 — arc / floater gravity |
| `BallisticScatter` | `[CombatDamage]` | RulesClass+0x1734 — Inaccurate random scatter distance |
| `VeteranROF` / `VeteranSpeed` / `VeteranDamage` | `[General]` | Veterancy multipliers |
| `MinMoveSpeed` | `[General]` | RulesClass+0x16C0 — infantry scatter-on-melee threshold |
| `SightRange` (default) | `[General]` | RulesClass+0x18 — aircraft RevealOnFire radius |

### Infantry per-unit burst timing

| Key | Section | Struct offset | Role |
|-----|---------|---------------|------|
| `BurstDelay0` | `[InfantryType]` | type+0xE40 | Frames between shot 0 and shot 1 |
| `BurstDelay1` | `[InfantryType]` | type+0xE44 | Frames between shot 1 and shot 2 |
| `BurstDelay2` | `[InfantryType]` | type+0xE48 (prone) | Alternate delay for prone state |
| `BurstDelay3` | `[InfantryType]` | type+0xE4C (deploy) | Alternate delay for deployed state |

### Firepower-gating abilities

| Key | Struct offset | Role |
|-----|---------------|------|
| `VeteranAbilities=FIREPOWER,...` | type+0x29E | Enables veteran firepower multiplier |
| `EliteAbilities=FIREPOWER,...` | type+0x2B0 | Enables elite firepower multiplier |

---

## 11. Follow-up investigation (2026-04-23) — answered open questions

Six of the eight original open questions have been answered through further Ghidra
analysis. Summary below; details in the subsections that follow.

| # | Question | Status |
|---|----------|--------|
| 1 | `this+0x298` semantics | **RESOLVED** — Psychedelic/Frenzy state flag |
| 2 | `this+0x2F0` readers | **RESOLVED** — no readers; dead TS-legacy write |
| 3 | BuildingClass 11-entry FireError jump table | **RESOLVED** — full decode |
| 4 | `weapon+0x136` INI mapping | **RESOLVED** — `NeverUse` bool |
| 5 | BurstDelay2/3 vs DVC collision | **RESOLVED** — Infantry uses 0xE40–0xE4C, Unit uses 0xE48–0xE54, all four read but only idx 0 and 1 used by the fire pipeline |
| 6 | `typeClass+0xE5D` / `this[1].field_0x1a0` | Still open — needs another pass |
| 7 | vtable+0x2E8 per-class overrides | **RESOLVED** — all derived classes use the TechnoClass base (no overrides) |
| 8 | 3-arg GetFireError `fire_flag` param | Still open — step 39's `vtable+0x3A8` gate warrants its own trace |

### 11.1 `this+0x298` = **Psychedelic/Frenzy state flag**

Writer at `0x00701d8c` inside `TechnoClass::ReceiveDamage` (the block gated by
`warhead+0x16D != 0`). When a warhead with `Psychedelic=yes` (INI bool, parsed to
`warhead+0x16D` at `WarheadTypeClass::ReadINI 0x0075D8F2`) hits a non-allied,
non-immune, non-building target:

```c
this->field_0x29C = damage_computed;   // frenzy duration counter (from armor-table lookup)
if (this->field_0x298 == 0) {
    this->field_0x298 = 1;              // SET frenzy flag
    // eject passengers (FUN_006ea870), StopFiring (vtbl+0x3C8), reset anim (vtbl+0x1E8)
}
return 1;  // damage is NOT applied — frenzy substitutes for HP loss
```

While the flag is set:
- **`TechnoClass::Fire_At 0x006FFEC9`:** halves the new ROF value
  (`if (this->field_0x298 != 0) rof = rof / 2`), so the frenzied unit fires **twice
  as fast**.
- **`TechnoClass::Scan_Cell_For_Target 0x006f8aaa` and `0x006f8bd0`:** allows the
  scanner to target allied units (the normal "skip allies" branch is bypassed when
  `this->field_0x298 != 0`).
- **`TechnoClass::AI_Update 0x006F9F0D`:** each tick, decrements `this->field_0x29C`.
  When it reaches 0, clears the flag, resets the counter, calls StopFiring, and
  restores the normal idle anim state.

**Known INI uses in YR content:** rulesmd warheads with `Psychedelic=yes` include
those on Yuri-psychic effects. This is an **active YR mechanic** — the target of
a Psychedelic-flagged weapon enters a temporary frenzy state (fires at 2× ROF and
will attack allies until the timer expires). Not TS-legacy.

**Rust status:** Not implemented. The frenzy/psychedelic mechanic, ROF halving
from this flag, and the allied-target bypass are all missing from the current
combat pipeline.

### 11.2 `this+0x2F0` — no readers; write-only dead field

Searched all TechnoClass-plausible read patterns against the binary (`8B 86`,
`8B 8E`, `8B 81`, `8A 86`, `8B 96`, `DB 86`, `83 BE` prefixes with `F0 02 00 00`
displacement). Every match lands in an unrelated struct (ParticleSystemTypeClass,
HouseClass, BulletTypeClass, VoxelAnimTypeClass, SidebarClass, TriggerCondition) —
which happen to have their own field at offset 0x2F0. **No function reads
`TechnoClass::field_0x2F0`**.

Conclusion: the two writes in Fire_At (`*(uint *)&this->field_0x2f0 = uStack_a8`
in both the main and DiskLaser paths) are **dead assignments** — presumably TS
legacy where this field held a separate muzzle-flash or beam duration that was
consolidated into `+0x2F4` (ROF initial) in YR. Safe to treat as unused in any
YR-faithful re-implementation.

### 11.3 BuildingClass 11-entry FireError jump table (full decode)

Table at `0x0044B728`, dispatched at `0x0044B0D7` via
`JMP [EDI*4 + 0x0044B728]` after the range check `CMP EDI, 0xA / JA default`:

| FireError | Target | Behavior |
|-----------|--------|----------|
| 0 (OK) | `0x0044B2BC` | **Fire path** — if garrisoned (`type+0x702 != 0` and `this+0x5EC` has occupants), dispatch to garrison occupant's `vtbl+0x3CC`. Else if `this.type == Rules[0x008871E0]+0x498` (`[General] PrismType`), run the prism supporter/cascade sequence. Else fall through to standard non-garrison fire, including the generic `IsAnimDelayedFire` branch. |
| 1 (Reloading) | `0x0044B0DE` | **StopFiring + gattling decay** — vtbl+0x3C8, clear garrison idx, if `vtbl+0x430` true call `0x00705D60`, check `vtbl+0x184 == 0x1C` (state), if gattling call `UpdateGattlingStage(1)` to decay, set anim state 5, fall through to `StopFiring`. |
| 2 (NotAligned) | `0x0044B187` | **Track target** — call `vtbl+0x4E8(target, &out)` to get the target's current facing, call `FacingClass::SetTargetFacing (0x4C9220)` on `&this+0x388` (turret facing field) to begin rotating, then gattling decay if applicable. Returns 1 (mission tick continues). |
| 3 (Busy) | `0x0044B1DE` | **Charge + keep tracking** — same tracking intro as 2. If gattling, **`IncreaseGattlingStage`** to build charge (not decay). Else increment `this+0x148` (ChargeCounter). Returns **2** (keep mission, shorter re-dispatch). |
| 4 | `0x0044B14E` | **Return-with-facing-reset** — if target exists, set facing timer via `vtbl+0x4E8`, call `0x004C9220` (FacingClass reset), clear `this+0xC4`. Returns 1. No gattling update. |
| 5 (Illegal) | `0x0044B0DE` | Same as 1 — StopFiring path. |
| 6 (OutOfRange) | `0x0044B0DE` | Same as 1 — StopFiring path. |
| 7 | `0x0044B14E` | Same as 4 — return-with-facing-reset. |
| 8 | `0x0044B0DE` | Same as 1 — StopFiring path. |
| 9 (Cloaked) | `0x0044B284` | **Decloak** — call `vtbl+0x45C(0)` to handle cloaked target (reveal/decloak animation). If gattling: decay stage + reset GattlingValue, jump to shared gattling-post handler. Else jump to shared `set anim 5 / ret 1`. |
| 10 | `0x0044B24F` | **Gattling decay only** — if gattling, `UpdateGattlingStage(GattlingValue)` + reset `this+0xC4`. Returns 1. No facing reset, no StopFiring. |

Key observations:
- **Tesla Coil / charge-mode** buildings care most about err 3 (Busy) — it's where
  they build up charge without firing. The `IncreaseGattlingStage` call for gattling
  buildings on err 3 is how the Iraqi Tank Bunker-style charge-up works.
- **Prism Tower / AA Gun** (non-gattling, non-charge) follow the straightforward
  path: err 0 → fire, err 2 → track, err 5/6/8 → StopFiring.
- **PrismType** has a special path inside err 0 via the RulesClass `+0x498`
  type comparison; it dispatches the prism supporter/cascade sequence.

**Rust status:** The Rust BuildingClass path currently implements garrison round-robin
fire (Phase 3c) but not charge-mode, not prism cascade dispatch, and not turret tracking
via err 2/4. The jump table structure itself is unlikely to be worth mirroring; a
match on FireError is idiomatic in Rust.

### 11.4 `weapon+0x136` = **NeverUse** (bool INI key)

Verified at `WeaponTypeClass::ReadINI 0x0077216C`:
```asm
PUSH "NeverUse"          ; s_NeverUse_008494F0
CALL CCINIClass::ReadBool
MOV byte [ECX+0x136], AL
```

Confirms the guess in SelectWeaponAgainst rule 5. The full WeaponTypeClass bool
layout across `+0x129..+0x15C` is now known in detail (see the *Weapon flag map
sidebar* below).

**Weapon flag byte-map (offsets 0x129–0x15C) — extracted from WeaponTypeClass::ReadINI:**

| Offset | INI key |
|--------|---------|
| 0x129 | `UseFireParticles` |
| 0x12A | `UseSparkParticles` |
| 0x12B | `OmniFire` |
| 0x12C | `DistributedWeaponFire` |
| 0x12D | `IsRailgun` |
| 0x12E | `Lobber` |
| 0x12F | `Bright` |
| 0x130 | `IsSonic` |
| 0x131 | `Spawner` |
| 0x132 | `LimboLaunch` |
| 0x133 | `DecloakToFire` |
| 0x134 | `CellRangefinding` |
| 0x135 | `FireOnce` |
| 0x136 | **`NeverUse`** ← resolved |
| 0x137 | `RevealOnFire` |
| 0x138 | `TerrainFire` |
| 0x139 | `SabotageCursor` |
| 0x13A | `MigAttackCursor` |
| 0x13B | `DisguiseFireOnly` |
| 0x13C | `DisguiseFakeBlinkTime` (int32) |
| 0x140 | `InfiniteMindControl` |
| 0x141 | `FireWhileMoving` |
| 0x142 | `DrainWeapon` |
| 0x143 | `FireInTransport` |
| 0x144 | `Suicide` |
| 0x145 | `TurboBoost` |
| 0x146 | `Supress` (note single 'p') |
| 0x147 | `Camera` |
| 0x148 | `Charges` |
| 0x149 | `IsLaser` |
| 0x14A | `DiskLaser` |
| 0x14B | `IsLine` |
| 0x14C | `IsBigLaser` |
| 0x14D | `IsHouseColor` |
| 0x14E | `LaserDuration` (int32) |
| 0x14F | `IonSensitive` |
| 0x150 | `AreaFire` |
| 0x151 | `IsElectricBolt` |
| 0x152 | `DrawBoltAsLaser` |
| 0x153 | `IsAlternateColor` |
| 0x154 | `IsRadBeam` |
| 0x155 | `IsRadEruption` |
| 0x158 | `RadLevel` (int32) |
| 0x15C | `IsMagBeam` |

### 11.5 BurstDelay fields — **different layouts on Infantry vs Unit**

The "BurstDelay%d" format string is xref'd ONLY from `UnitTypeClass::ReadINI`.
Infantry uses 4 separate hardcoded reads at the tail of `InfantryTypeClass::ReadINI`.
Layout:

| Class | BurstDelay0 | BurstDelay1 | BurstDelay2 | BurstDelay3 | Reader |
|-------|-------------|-------------|-------------|-------------|--------|
| InfantryTypeClass | +0xE40 | +0xE44 | +0xE48 | +0xE4C | 4 separate ReadInt calls at `0x005242XX` tail |
| UnitTypeClass | +0xE48 | +0xE4C | +0xE50 | +0xE54 | format-string loop at `0x00747B04` |

Pipeline consumption (verified at `GetFireError 0x006FC91B` and
`UnitClass::Fire_At_Wrapper FUN_00741340 0x00741368`):

```c
// Both readers are gated on CurrentBurstIndex % weapon.Burst < 2:
iVar = *(int *)(typeClass + 0xE40 + (CurrentBurstIndex % Burst) * 4);
```

Critical observations:
- On **InfantryTypeClass**: the `+0xE40 + i*4` base hits BurstDelay0 (i=0) and
  BurstDelay1 (i=1) — matches the parsed layout.
- On **UnitTypeClass**: the same `+0xE40 + i*4` base hits fields 0xE40/0xE44, which
  are **NOT BurstDelay** (BurstDelay0 on UnitType starts at 0xE48). These offsets
  are earlier in the UnitTypeClass layout and appear to be unrelated fields.
- The `i < 2` gate means **only shots 0 and 1 of any burst use per-shot delays**
  — BurstDelay2 and BurstDelay3 are **parsed from INI but never consumed** in
  either pipeline path. They are dead-reads, TS-legacy artifacts.

So the "DVC collision" concern in the prior burst report is **not a practical
issue in YR** — BurstDelay2/3 are written into what may overlap DVC memory on
some derived types, but nothing reads those slots during firing.

**Rust status:** The current Rust `AttackTarget.burst_delay_ticks` uses a single
delay. To match YR exactly, infantry should consult `InfantryTypeClass.BurstDelay0`
and `BurstDelay1` for the first two shots of a burst (`shot_index == 0` and
`shot_index == 1`), and use weapon.ROF for subsequent shots. BurstDelay2/3 do
not need to be honored.

### 11.6 `vtable+0x2E8` — SelectWeaponAgainst fallback selector

All four TechnoClass-derived vtables (Aircraft, Infantry, Unit, Building) point
`+0x2E8` at **`FUN_006F3820` at `0x006F3820`** — the base TechnoClass implementation.
**No class overrides this slot.**

```c
// FUN_006F3820(this, target, out_ptr)
if (target == NULL || (target.flags & 1) == 0) return -1;  // reject non-techno
char cD69 = target.type + 0xD69;   // TS-era flag (likely "MovementZone=Fly" or similar)
char cD97 = target.type + 0xD97;   // (possibly "BallonHover")
int  i67C = target.type + 0x67C;   // target zone ID?
char c694 = target.type + 0x694;   // (possibly "Naval" or "ImmuneToVeins")
switch (this.type + 0x600) {       // per-firer-type selector mode
    case 0:  if (cD69 == 0) return 0; else if (target+0x220 != 0) return -1; else return 0;
    case 1:  return cD69 != 0 ? 1 : 0;
    case 2:  return cD69 != 0 ? 0 : -1;
    case 3:  if (cD97 || c694) return 1; break;
    case 4:  if (i67C != 3 && !cD97) return 1; break;
    case 6:  return -1;
}
return 0;
```

The switch key `this.type+0x600` is a per-type enum — likely the TechnoType's
weapon-selection mode (possibly "Size" or "MovementZone"; the exact INI key was not
traced here). The function provides a last-resort override of the verses-based
weapon pick in `SelectWeaponAgainst` rule 9. In practice it returns 0 (primary),
1 (secondary), or -1 (no opinion — use verses fallback).

**Rust status:** Not implemented. Given that the switch key is a per-type enum
that has not been mapped to an INI key yet, and the function's outcomes are
already captured by a simpler "choose secondary when Verses-against-target is
nonzero and primary Verses is zero" rule, this is likely not urgent to port.

### 11.7 `typeClass+0xE5D` = **FiringFrames** (UnitTypeClass only)

Writer at `UnitTypeClass::ReadINI 0x0074781A`:
```
PUSH byte [EDI+0xE5D]    ; default = current value
PUSH "FiringFrames"       ; s_FiringFrames_00845D84
PUSH pINI
MOV ECX, EBX              ; this (CCINIClass)
CALL CCINIClass::ReadInt  ; or ReadInt-to-byte; AL is the return
MOV [EDI+0xE5D], AL       ; store result
```

Single xref in the binary to "FiringFrames" → only on UnitTypeClass (vehicles,
aircraft). Not on InfantryTypeClass, BuildingTypeClass, or WeaponTypeClass. This
is the RA2 INI key that controls the firing-animation frame count for tanks and
vehicles with a firing-anim sequence (e.g., Grizzly Tank's barrel recoil, IFV
turret flash sequence).

**Consumption in `FUN_00741340` (UnitClass::Fire_At wrapper)** — immediately after
a successful `TechnoClass::Fire_At`:

```c
if ((!bVar3) && (cVar1 = type->FiringFrames, cVar1 > 0)) {
    this[1].field_0x1a0 = cVar1 * 2 - 1;   // reset burst-delay state counter
}
```

Where `bVar3` was set earlier based on the `CurrentBurstIndex % Burst < 2`
and BurstDelay-match branch — meaning this reset runs only when the current shot
was NOT gated by a BurstDelay. The formula is `FiringFrames * 2 - 1` — for
`FiringFrames=1` → 1, `=2` → 3, `=3` → 5. This stored value is then matched against
a per-frame decrement counter in `GetFireError`'s burst gate (step 23 of §4).

**Purpose:** synchronizes the fire moment with the animation's peak — a vehicle
with `FiringFrames=3` waits `3*2-1 = 5` anim-frame decrements before being
allowed to fire the next burst shot. The `*2` is because the counter decrements
twice per anim frame (at 30 internal updates per 15fps tick).

**Rust status:** Not modeled. Vehicle firing in Rust is currently triggered by
turret alignment only, not animation-frame-sync. Matching this exactly would
require tracking animation frame index per unit and gating fire on the peak
frame.

### 11.8 `GetFireError` `fire_flag` — commit vs peek mode (range-check gate)

**Step 39 of §4** calls `vtbl+0x3A8` when `fire_flag != 0`. Resolved:

- **`vtbl+0x3A8` = `TechnoClass::CanFireAt` at `0x006F77B0`** — performs the
  range/visibility/in-range check (calls `TechnoClass::InRange` internally).
  Confirmed: all four derived class vtables point this slot at `0x006F77B0`.
- When `fire_flag=1`: GetFireError returns 8 if CanFireAt returns false (i.e.,
  target is out of range or unreachable).
- When `fire_flag=0`: the CanFireAt check is skipped entirely — returns 0 as if
  the range is fine.

**Caller survey (verified by byte-pattern search for `FF 90 C0 03 00 00`):**

| Address | Function | fire_flag | Purpose |
|---------|----------|-----------|---------|
| `0x0044ADAE` | `BuildingClass::Mission_Attack` normal path | **1** | Actual fire dispatch — range must pass |
| `0x0044B00F` | `BuildingClass::Mission_Attack` charge mode | **1** | Release Tesla-coil charge — range must pass |
| `0x005206F3` | `InfantryClass::Fire_At_Target` | **1** | Commit the infantry fire frame — range must pass |
| `0x0073DD20` | UnitClass callsite | **1** | (likely `UnitClass::Fire_At_Target`) — commit |
| `0x006FC09E` | **Unnamed wrapper function at `0x006FC090`** | **0** | Peek — skip range check |

The unnamed wrapper at `0x006FC090` is the **peek adapter** — a small
method that forwards `(target, weapon_idx)` to `GetFireError` with `fire_flag=0`
and returns the error code. It occupies **vtbl+0x3BC** on all TechnoClass-derived
vtables (verified at `0x007E2660`, `0x007E4278`, `0x007E9050`, `0x007EB414`,
`0x007F4D1C`, `0x007F602C`).
NOTE: Ghidra does not have a function boundary defined at `0x006FC090` — the vtable
entry points there (confirmed `read_memory 0x007E2660` = `90 C0 6F 00`) but
`get_function_by_address 0x006FC090` returns no match, meaning Ghidra has not
auto-created the function. The address is valid; the function boundary needs manual
`create_function 0x006FC090` to become callable by name.
(annotated 2026-05-29: was described as a defined 4-instruction function; binary shows
address is correct but no Ghidra function boundary exists — ROOT_CAUSE: STALE)

**Callers of the peek wrapper (`vtbl+0x3BC`):**

| Address | Function | Purpose |
|---------|----------|---------|
| `0x00708282` | `FUN_00708080` (passive targeting helper) | Cheap "can I fire at this target?" test during scan |
| `0x007084B8` | `FUN_00708080` | Same — secondary check |
| `0x007088ED` | `TechnoClass::ShouldRetaliate` | Decide whether to return-fire at attacker — skip range because the retaliation target is usually nearby and range will be rechecked when actually firing |

The pattern in all three callers: `CMP EAX, 5 / JZ skip` — check if the result
is FireError 5 (ILLEGAL). If illegal, the unit won't retaliate / scan. Range
errors (6) don't disqualify — the unit might still retaliate once it gets
closer.

**Semantic summary:**

- `fire_flag = 0` (**peek mode**): "Would firing be legal in principle, ignoring
  range?" Used by retaliation logic and passive target scoring. Optimization:
  skipping the range check avoids the costly `InRange` calculation during
  pre-filter queries.
- `fire_flag = 1` (**commit mode**): "Am I allowed to fire RIGHT NOW, including
  being in range?" Used by the three actual fire-dispatch sites
  (InfantryClass/UnitClass Fire_At_Target, BuildingClass Mission_Attack).

My earlier guess that this was UI cursor feedback vs fire-commit was **wrong**.
It's strictly a **range-check inclusion toggle** — no UI paths call either
variant.

**Rust status:** The Rust `combat_fire_gate.rs` collects all fire-blocking
conditions in a single pass. If the pipeline ever needs distinct peek/commit
modes (e.g., to optimize large-scale passive scans that pre-filter targets),
the range check is the obvious split point — everything else is O(1) per unit
while `InRange` requires geometry.

## 12. Sources

### Functions decompiled in §11 (follow-up passes)
- `0x00701900` — `TechnoClass::ReceiveDamage` (Psychedelic write site)
- `0x006F9E50` — `TechnoClass::AI_Update` (Psychedelic counter decrement + clear)
- `0x006F8960` — `TechnoClass::Scan_Cell_For_Target` (Psychedelic ally-target bypass)
- `0x006F3820` — `TechnoClass::SelectWeaponFallback` (vtbl+0x2E8)
- `0x006F77B0` — `TechnoClass::CanFireAt` (vtbl+0x3A8 — the range gate)
- `0x006FC090` — peek-wrapper for `GetFireError` (vtbl+0x3BC — fire_flag=0)
- `0x007087C0` — `TechnoClass::ShouldRetaliate` (peek-mode caller)
- `0x00708080` — `FUN_00708080` passive target helper (peek-mode caller)
- `0x0077209A` — `WeaponTypeClass::ReadINI` (full bool field map)
- `0x0075D8C0` — `WarheadTypeClass::ReadINI` around the Psychedelic block
- `0x005240A0` — `InfantryTypeClass::ReadINI` (BurstDelay0–3 at +0xE40)
- `0x00747B04` — `UnitTypeClass::ReadINI` BurstDelay loop (at +0xE48)
- `0x00747801` — `UnitTypeClass::ReadINI` `FiringFrames` parse (writes +0xE5D)

### Memory regions inspected in §11
- Jump table at `0x0044B728` (11 × 4 bytes — one entry per FireError code 0–10)
- Jump-target branches at `0x0044B0DE`, `0x0044B14E`, `0x0044B187`, `0x0044B1DE`,
  `0x0044B24F`, `0x0044B284`, `0x0044B2BC`
- VTable slots `+0x2E8` on Aircraft (`0x007E258C`), Techno/ambiguous
  (`0x007E41A4`), Infantry (`0x007E8F7C`), Unit (`0x007F5F58`), Building
  (`0x007F4C48`) — all four resolve to `0x006F3820`
- VTable slots `+0x3A8` (CanFireAt) on multiple classes — all resolve to `0x006F77B0`
- VTable slots `+0x3BC` (peek-wrapper) on multiple classes — all resolve to `0x006FC090`

---

### Functions decompiled in the original report (§1–§10)
- `0x006F3330` — `TechnoClass::SelectWeaponAgainst`
- `0x006FC0B0` — `TechnoClass::GetFireError`
- `0x006FDD50` — `TechnoClass::Fire_At` (re-read for verification)
- `0x005218E0` — `InfantryClass::SelectWeaponAgainst_Wrapper`
- `0x00746CD0` — `UnitClass::SelectWeaponAgainst_Wrapper`
- `0x00736DF0` — `UnitClass::Fire_At_Target`
- `0x005206B0` — `InfantryClass::Fire_At_Target`
- `0x0051DF70` — `InfantryClass::Fire_At_Override`
- `0x00741340` — `UnitClass::Fire_At_Wrapper` (unnamed)
- `0x00415EE0` — `AircraftClass::Fire_At`
- `0x0041A9E0` — `AircraftClass::What_Weapon_Should_I_Use` (misnamed; is really fire-action decider)
- `0x0044ACF0` — `BuildingClass::Mission_Attack`
- `0x004D4DC0` — `FootClass::Mission_Attack`
- `0x006FDB80` — pre-damage calculator
- `0x00773070` — `WeaponType::GetMaxBallisticRange` (ballistic-aware speed)
- `0x0046B270` — small helper: reads `weapon+0x130`

### Existing docs referenced
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIRE_AT_ANALYSIS.md` (primary prior report — extended here, not duplicated)
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BURST_WEAPON_FIRING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TARGET_ACQUISITION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`

### INI files checked
- `ini/rulesmd.ini` (YR patch — authoritative)
- `ini/artmd.ini` (YR patch — authoritative)
- `ini/rules.ini` (RA2 base — fallback)

### Rust files inventoried
- `src/sim/combat/mod.rs`
- `src/sim/combat/combat_weapon.rs`
- `src/sim/combat/combat_targeting.rs`
- `src/sim/combat/combat_fire_gate.rs`
- `src/sim/combat/combat_aoe.rs`
- `src/rules/weapon_type.rs`
- `src/rules/projectile_type.rs`
- `src/rules/warhead_type.rs`
- `src/sim/aircraft/attack_mission.rs`
