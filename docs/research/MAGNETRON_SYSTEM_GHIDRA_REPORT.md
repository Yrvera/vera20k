# Magnetron System — Ghidra Research Report

Research date: 2026-04-19
**Addresses:** `0x004690B0` (Detonate), `0x00710000` (TechnoClass::PerformDeploy), `0x00489280` (Apply_area_damage), `0x0075D590` (WarheadTypeClass::ReadINI), `0x00772080` (WeaponTypeClass::ReadINI)
**Confidence:** HIGH (offsets + dispatch branching verified from disassembly); MEDIUM on end-of-lift teardown (piggyback-release timer not fully traced)
**Active in YR:** YES — Magnetron unit `[TELE]` is a YR-exclusive Yuri unit, not TS legacy.

---

## 1. Overview

The Magnetron is a Yuri-faction anti-vehicle unit that disables enemy ground vehicles by
replacing their locomotor with a Jumpjet locomotor. The replacement is implemented
through the generic `IsLocomotor=yes` warhead flag on the `LocomotorBeam` warhead; the
`Locomotor={...}` GUID on the warhead selects which locomotor type is piggyback-swapped
onto the target (Chronosphere uses the same mechanism with a Teleport CLSID; Magnetron
uses Jumpjet).

The Magnetron has two weapons:
- **Primary `[MagneticBeam]`** — fires `LocomotorBeam` warhead → locomotor hijack (the lift).
- **Secondary `[MagneShake]`** — fires `MagneShakeWH` warhead → regular damage against buildings
  (no locomotor effect; buildings are excluded from the swap via the `flag & 4` early-return).

---

## 2. Class Layout / Key Offsets

### 2.1 WarheadTypeClass — verified byte offsets

Extracted from disassembly of `WarheadTypeClass__ReadINI` at `0x0075D590–0x0075DEAC`. All offsets
below are **direct byte offsets** from `this` (param_1 is `int *`, but each access is an explicit
typed offset, not index × 4).

| Offset | Field | INI key | Type | Notes |
|--------|-------|---------|------|-------|
| 0x14B  | (bool) | *(string@0x847df0)* | bool | First bool read by ReadINI |
| 0x14C  | (bool) | *(string@0x836f04)* | bool | |
| 0x14E  | `Rocker` | `Rocker` | bool | String at 0x847de8 |
| 0x14F  | `DirectRocker` | `DirectRocker` | bool | String at 0x847dd8. **Triggers vtable+0x3D8 knockback in Detonate main path (see §3.3)** |
| 0x150  | (bool) | *(string@0x847dd0)* | bool | |
| 0x151  | (bool) | *(string@0x847dc0)* | bool | |
| 0x152  | (bool) | *(string@0x847db0)* | bool | |
| 0x153  | (bool) | *(string@0x847da0)* | bool | **NOT IsLocomotor** — Rust code comment is wrong |
| 0x154  | (bool) | *(string@0x847d60)* | bool | |
| 0x155  | `MindControl` | `MindControl` | bool | Dispatched first in Detonate |
| 0x157  | `IvanBomb` / IsBomb | | bool | |
| 0x158  | `ElectricAssault` | | bool | |
| 0x159  | `Parasite` | `Parasite` | bool | String at 0x81717c |
| 0x15A  | `Temporal` | `Temporal` | bool | String at 0x817168 |
| **0x15B** | **`IsLocomotor`** | **`IsLocomotor`** | **bool** | **String at 0x847d3c, single xref. Verified.** |
| **0x15C–0x16B** | **`Locomotor`** | **`Locomotor`** | **GUID (16 bytes)** | **Read via `CCINIClass::ReadCLSID` (0x00527920). Default is TeleportLocomotion `{4A582747-9839-11d1-B709-00A024DDAFD1}` if unset.** |
| 0x16C  | `Airstrike` | | bool | |
| 0x16E  | `BombDisarm` | | bool | |
| 0x175  | `MakesDisguise` | | bool | |
| 0x176  | `NukeMaker` | | bool | |

### 2.2 WeaponTypeClass — verified byte offsets

Extracted from disassembly of `WeaponTypeClass__ReadINI` at `0x00772080`. Same direct-byte-offset
rule as WarheadTypeClass (param_1 is `int *` but accesses are typed).

| Offset | Field | INI key | Type | Notes |
|--------|-------|---------|------|-------|
| **0x15C** | **`IsMagBeam`** | **`IsMagBeam`** | **bool** | **String at 0x84928c, single xref to 0x007728f0. Verified.** Marks a weapon as rendering the Magnetron magnetic beam visual (color = `[General] MagnaBeamColor`). |

**Note:** `IsMagBeam` (WeaponType+0x15C, bool) and `Locomotor_GUID_start` (WarheadType+0x15C, 16-byte GUID)
**share a numeric offset** (0x15C) but are on **different classes**. Do not confuse them.

### 2.3 BulletClass field usage observed in Detonate

Decompilation of `WarheadTypeClass__Detonate` (0x004690B0) is flagged as a thiscall with
`this = BulletClass*`. Relevant accesses during the IsLocomotor branch:

| Access in decomp | Byte offset | Meaning (inferred from context) |
|------------------|-------------|---------------------------------|
| `param_1[0x2b]` | 0xAC | BulletType pointer |
| `param_1[0x2c]` | 0xB0 | **Owner** — the firer TechnoClass*. Confirmed via `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`: "0xB0 — ptr — Owner — `-> TechnoClass (firer/source)`". The decompiler's cast-to-coord-ptr in the Apply_area_damage call is a representation artifact (__fastcall register scheduling); the offsets `+0x2AC`, `+0x2B0` used alongside it are standard TechnoClass fields. |
| `param_1[0x43]` | 0x10C | Target pointer |
| `param_1[0x4a]` | 0x128 | Warhead pointer (WarheadTypeClass*) |
| `param_1[0x54]` | 0x150 | Damage multiplier / scaling |

These match existing BulletClass reports. `param_1[0x2c]` (= Bullet+0xB0) is the
**firer's TechnoClass pointer** (resolved 2026-04-19 via BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md §Owner).
In the IsLocomotor branch (§3.4), the code reads `firer->0x2AC` and `firer->0x2B0`,
which are standard TechnoClass fields (resolved 2026-04-19 via
TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md):

- **TechnoClass+0x2AC = LocomotorTarget** (pointer to chronoshifted unit being warped-to)
- **TechnoClass+0x2B0 = LinkedBuilding** (back-pointer from the warped unit to the
  ChronoSphere that warped it; bidirectional pair with 0x2AC)

Both are ChronoSphere-building deploy-link fields. **For the Magnetron (a vehicle
unit, not a building), both fields are always 0** at runtime. The IsLocomotor
branch's pre-deploy gate (`firer->0x2AC != target && firer->0x2B0 == 0`) therefore
passes trivially when the firer is a Magnetron — the real Chrono-collision
bookkeeping only matters when the firer is a ChronoSphere building.

Also resolved 2026-04-19 for the final per-target gate:

- **TechnoTypeClass+0x380 = SizeWeight** (double, verified via
  MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md §refinery dock size check)
- **Bullet+0x6C = Health/Damage** (int, the bullet's effective damage, set from
  `weapon.Damage`)

The final gate `target.type.SizeWeight < (double)bullet.Damage` means: *a unit is
Magnetron-eligible if its `SizeWeight` (unit's structural-mass INI field) is less
than the weapon's damage value*. For standard `MagneticBeam` (damage=600), this
works out to virtually all ground vehicles (most units have SizeWeight < 600), but
could exclude very heavy units if any existed. Elite version `MagneticBeamE`
(damage=10000) has no practical weight limit.

---

## 3. Core Logic

### 3.1 Fire path (primary weapon)

Magnetron's `MagneticBeam` weapon has `IsMagBeam=yes`, which triggers the magnetic-beam
visual render (color from `[General] MagnaBeamColor=255,200,255`). The projectile is
`InvisibleHigh` (no visual trail), and the Warhead is `LocomotorBeam`. The beam visual
is cosmetic; the gameplay effect comes entirely from `LocomotorBeam.IsLocomotor=yes` +
`Locomotor={jumpjet GUID}`.

### 3.2 Warhead dispatch in `WarheadTypeClass::Detonate` (0x004690B0)

`Detonate` is a `__thiscall` called on the BulletClass at bullet impact. After an
optional radsite spawn, it checks the warhead's special-type flags in this **if/else-if
chain** (mutually exclusive):

```text
if (warhead->MindControl @ 0x155)        → mind-control branch
else if (warhead->IvanBomb @ 0x157)      → IvanBomb branch (BombClass ctor)
else if (warhead->ElectricAssault @ 0x158) → electric assault branch (FUN_00452820 for infantry)
else if (warhead->Parasite @ 0x159)      → parasite attach (FUN_0062a980)
else if (warhead->Temporal @ 0x15A)      → TemporalClass::InitiateWarp
else if (warhead->IsLocomotor @ 0x15B)   → *** LOCOMOTOR HIJACK (§3.4) ***
else if (warhead->Airstrike @ 0x16C)     → airstrike branch (FUN_0041d830)
else if (warhead->BombDisarm @ 0x16E)    → bomb-disarm branch (FUN_004389b0)
else if (warhead->MakesDisguise @ 0x175) → disguise vtable+0x46c
else if (warhead->NukeMaker @ 0x176)     → NukeMaker::SpawnDownwardNuke
else                                     → Apply_area_damage (regular damage)
```

Note that before the else-chain falls through to Apply_area_damage, there is an
additional test for `warhead->DirectRocker (0x14F)` that takes a completely different
code path (§3.3) before normal damage.

### 3.3 DirectRocker vs Rocker — these are NOT IsLocomotor

**Important correction to prior research** (WARHEAD_DETONATE_GHIDRA_REPORT.md
and MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md both conflated these with IsLocomotor):

- **DirectRocker (offset 0x14F)** — triggers a knockback-style logic block inside
  `WarheadTypeClass::Detonate` *before* the else-branch to Apply_area_damage. The
  block computes a per-target push direction (`target.coords - source.coords` normalized,
  scaled by `BridgeDiag_NonBridge` = 10.0) and calls `(**(piVar14 + 0x3d8))(&newCoords, speed)`
  — i.e. TechnoClass vtable offset 0x3D8 (`ApplyLocomotor` / `Movement_AI` per Techno vtable docs).
  Speed is computed as `(damage * strength >> 8) * RulesClass+0x18b4 / 0x0081aef8` and clamped
  to 4.0 when exceeding the `_DAT_007e3cc8` threshold.
- **Rocker (offset 0x14E)** — triggers the 7×7-cell rocker loop inside
  `Apply_area_damage` (0x00489280). The loop iterates `dx = -3..+3, dy = -3..+3`, grabs
  each cell's object list (above- or under-bridge depending on altitude flag), and calls
  `(piVar12 + 0x3d8)(coord_arg, speed)` on every qualifying object. Gate:
  `*(char*)(param_4 + 0x14e) != 0 && _DAT_007e5138 < speed`.
  **This is the cell-wide "Rocker" screen/unit shake — NOT Magnetron lift.**

Neither of these paths is what the Magnetron's `LocomotorBeam` uses. The Magnetron
dispatches into the IsLocomotor branch (§3.4) and never reaches DirectRocker/Rocker logic
(the else-chain returns/falls-through before Apply_area_damage is called for IsLocomotor
warheads — Apply_area_damage only runs in the final `else` normal-damage arm).

### 3.4 IsLocomotor branch (the actual Magnetron/Chronosphere mechanic)

When `warhead->IsLocomotor @ 0x15B != 0`, the branch runs with `uVar16 = param_1[0x2c]`
and `piVar14 = param_1[0x43]` (target pointer):

```text
if (uVar16 != 0 &&
    *(uVar16 + 0x2ac) != piVar14 &&       // firer's 0x2AC (chrono-warp target ptr) != target
    *(uVar16 + 0x2b0) == 0)               // firer's 0x2B0 (chrono-warp link state) == 0
{
    // Early-return for buildings that are already ChronoWarp-deploying
    if ((firer.flag & 4) != 0 /* WhatAmI==building-ish */ &&
        *(char*)(firer + 0x6AD) != 0)      // "is currently deploying"
        goto LAB_00469aa4 (skip to explosion anim);

    local_75 = 0;
    // Snapshot target status (chrono-warp collision detection)
    if (target != 0 && target->WhatAmI() == 1 /* UnitClass */) {
        ...
        // If target is already a deployed ChronoWarp receiver on same cell → local_75=1
        if (looked-up-building == target) local_75 = 1;
    }

    // If firer already has a warp-target queued: deploy it now (finish prior chrono-warp)
    if (*(firer + 0x2AC) != 0)
        BuildingClass::DeployUnit_ChronoWarp();

    // Main locomotor swap — the Magnetron/Chronosphere core
    if (firer != 0 && local_75 == 0) {
        if ((firer.flag & 4) != 0) {
            if (firer->GetMissionID() != 2 /* MISSION_SLEEP? */)
                goto LAB_00469aa4;
        }
        // Checks on TARGET (piVar14 = param_1[0x43]):
        //   - is ground unit (flag & 4)
        //   - not undeploying (vtable+0x160 == 0)
        //   - WhatAmI==1 (Unit) OR WhatAmI==2 (Infantry)
        //   - target->0x6AD (deployed flag) == 0
        //   - target->GetType()->ChronoTrigger (@ +0x380) < warhead.some_field (@ +0x1B)
        if (all-checks-pass)
            TechnoClass::PerformDeploy(firer);  // ← entry to §3.5
    }
}
```

**Critical reading**: the `TechnoClass::PerformDeploy` call receives **the firer**
(`param_1[0x2c]`), not the target. The PerformDeploy function (0x00710000) is what does
the actual locomotor CoCreateInstance + Begin_Piggyback swap — and it operates on a unit
implied through `in_stack_00000004` (the unit being operated on, passed as a stack
argument — see §3.5).

The `param_1[0x2c]` semantics are imperfectly resolved by Ghidra's decompiler; context
suggests it is the Bullet's firer (TechnoClass*) but the 0xB0 offset needs separate
verification (see Open Questions §7).

### 3.5 `TechnoClass::PerformDeploy` (0x00710000) — the locomotor swap

Signature: `void __thiscall TechnoClass::PerformDeploy(TechnoClass *this)` with an
additional stack argument `in_stack_00000004` (the unit being operated on).

Core sequence (distilled, Ghidra decomp is heavily register-split):

1. **Kill spawns:** if `target->0xB4 /* SpawnManager */ != 0` → `SpawnManagerClass::Kill_All_Spawns()`.
2. **Detach parasite:** if target has a `ParasiteClass` attached via `target->0x1A5`/`0x1A7`
   and that parasite has `TypeClass->0xCCE != 0` → `WarpAttachClass::Detach()`.
3. **Preflight piggyback-end check:** call `FUN_0045AF20` on the locomotor ptr at
   `target + 0x19D × 4 = 0x674` (FootClass locomotor slot). Assert on failure.
4. **Conditional swap:**
   - If `target_type->0x5EC == 0` OR a specific cond — take the **replace-locomotor
     path**:
     - Clear owner's destination: `(*locomotor->vtable[0x9c])(0)`
     - Call `vtable[0x70](target, -1, g_NullCoord_Chrono)` (unlink target from source coord)
     - Set `target->0x6B6 = 1` (deployed flag) and `vtable[0x3D0]()` (movement stop)
     - `COM::CoCreateInstance_Locomotor(&newLoco, warhead_CLSID, 7)` — create new locomotor
       COM object using the warhead's CLSID (offset 0x15C-0x16B)
     - `Begin_Piggyback(newLoco, oldLoco)` (conceptual — represented as vtable calls `+0xC`
       and ref-count churn in decomp)
     - **Chrono-warp bookkeeping** via two `BuildingClass::DeployUnit_ChronoWarp` calls
       (args 0 then 1), which link the firer to the target for later "drop off" animation
     - `newLoco->0xAB = target` and `target->0x2B0 = newLoco` — bidirectional link
     - Call `newLoco.vtable[0x480](1)` (activate)
     - Call `newLoco.vtable[0x150]()` (mission transition)
     - If `target->0x175` set: `FUN_006EA870` (AudioClass hook? per naming)
     - Set `target->0x6AD = 1` (piggybacked flag)
     - Footclass destination fix-up: if destination is type 6 (BuildingClass), call
       `target.vtable[0x280](3)` (MISSION reset)
     - `newLoco.vtable[0x1E8](5, 1)` and possibly `(0xF, 0)` — state flips
   - Otherwise — take the **End_Piggyback path** (already-swapped → release):
     - `vtable[0x14]` returns true → treat as "can end"; drop existing wrapper, clear
       `target->loco = 0`, call outer vtable[0x10]
     - `vtable[0x14]` returns false → proceed with vtable[0x48] (coord get), vtable[0x1F0](-1),
       vtable[0x1E8](0xD, 0) — abort the swap and go to mission 0xD
5. Release the temporary COM ref (vtable+0x8).

**Lift duration / end-of-effect behavior:** `End_Piggyback` is triggered when
`newLoco.Is_Ok_To_End()` returns true (per DRIVE_LOCOMOTION_CLASS.md §1546–1565). For
Jumpjet, "Is_Ok_To_End" is gated on the jumpjet state machine (hovering stable, not
destroying, not in transit). When the target's HP reaches 0 while lifted, the jumpjet
locomotor's crash/fall behavior produces the characteristic Magnetron "drop kill." **The
precise end-condition for a non-fatal lift was not traced in this investigation** — see §7.

### 3.6 Secondary weapon — MagneShake anti-building

`MagneShake` warhead (`MagneShakeWH`) has no `IsLocomotor=` flag. Its `Verses=` is
`0%,0%,0%,0%,100%,0%,100%,100%,100%,0%,0%` — only effective against heavy/wood/concrete/
special armors (i.e., buildings and specific heavies). It is normal damage dispatched via
the `else` arm of the Detonate if/else-chain → `Apply_area_damage`. No locomotor swap.

Note MagneShake **also** has `IsMagBeam=yes` on the weapon side (line 24361 of rulesmd.ini),
so the visual beam draws on secondary fire too. The `Report=MagnetronMagneShake` voice
line and separate `VoiceSecondaryWeaponAttack=MagnetronMagneShakeVoice` handle the audio.

---

## 4. INI Keys

All values verified from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` (the
base `rules.ini` has no Magnetron entries — YR-only unit).

### 4.1 Unit `[TELE]` — lines 8586–8637

Key fields relevant to Magnetron behavior (full table in research agent B's output):

| Key | Value | Note |
|-----|-------|------|
| `Primary` | `MagneticBeam` | |
| `Secondary` | `MagneShake` | Anti-building |
| `ElitePrimary` | `MagneticBeamE` | Elite upgrade (2× damage, same mechanics) |
| `Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | **DriveLocomotion** (the Magnetron itself moves as a normal tracked vehicle — unrelated to the lift effect it inflicts) |
| `Speed` | 5 | |
| `MovementZone` | `Destroyer` | Can move through water (!) — unusual for a Yuri vehicle |
| `Owner` | `YuriCountry` | |
| `Cost` | 1000 | |

### 4.2 Primary weapon `[MagneticBeam]` — lines 24333–24343

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | 5000 | Very high nominal damage (but Verses limits applicability) |
| `ROF` | 20 | |
| `Range` | 12 | |
| `MinimumRange` | 3 | **Cannot fire at adjacent cells** |
| `Speed` | 100 | |
| `Projectile` | `InvisibleHigh` | `Inviso=yes`, no visual trail |
| `Warhead` | `LocomotorBeam` | **Triggers the lift** |
| `Report` | `MagnetronAttack` | Audio |
| `IsMagBeam` | `yes` | **Triggers magnetic-beam visual render (uses `[General] MagnaBeamColor=255,200,255`)** |

`[MagneticBeamE]` (elite) is identical except `Damage=10000`.

### 4.3 Secondary weapon `[MagneShake]` — lines 24346–24361

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | 80 | Low per-hit |
| `ROF` | 110 | Slow |
| `Range` | 10 | |
| `Warhead` | `MagneShakeWH` | Building damage only |
| `Spread` | 2 | Slight inaccuracy |
| `IsMagBeam` | `yes` | **Shares the beam visual** |

### 4.4 Primary warhead `[LocomotorBeam]` — lines 27294–27302

| Key | Value | Effect |
|-----|-------|--------|
| `Verses` | `0%,0%,0%,100%,100%,100%,0%,0%,0%,100%,0%` | 100% against medium/heavy/structure/unit armors; **0% against infantry, wood, flak, concrete, special armors** |
| `IsLocomotor` | `yes` | **Offset 0x15B — triggers IsLocomotor dispatch in Detonate §3.4** |
| `Locomotor` | `{92612C46-F71F-11d1-AC9F-006008055BB5}` | **JumpjetLocomotion GUID** — determines what locomotor is piggybacked onto the target |

The `Verses` table means Magnetron only picks up vehicles: infantry take 0% (no lift, no
damage), buildings take 100% (would trigger lift, but buildings are excluded by the
`flag & 4` check in §3.4 IsLocomotor branch — they are simply damaged/skipped).

### 4.5 Secondary warhead `[MagneShakeWH]` — lines 27551–27553

| Key | Value | Effect |
|-----|-------|--------|
| `Verses` | `0%,0%,0%,0%,100%,0%,100%,100%,100%,0%,0%` | Structures + special armor |
| `Bullets` | `yes` | Counts as bullet damage (subject to bullet-resistant armor) |

### 4.6 Global

- `[General] MagnaBeamColor=255,200,255` (line 602) — RGB for the magnetic beam visual.

---

## 5. Integration Points

- **Callers of `WarheadTypeClass::Detonate` (0x004690B0):**
  - `BulletClass::BulletDetonation` (0x00468D80) — the standard bullet-impact callsite.
    Note: the Ghidra function name `WarheadTypeClass__Detonate` is misleading; its
    effective `this` is a `BulletClass*`, not a warhead, based on the 0x4A/0x43/0x128
    field access pattern.
  - `FUN_0041BC30`
  - `FUN_0070D690`

- **Callers of `TechnoClass::PerformDeploy` (0x00710000):** called from
  `WarheadTypeClass::Detonate`'s IsLocomotor branch. May also be called from MCV-deploy
  paths and ChronoSphere delivery — not fully traced this session.

- **CoCreateInstance_Locomotor (FUN_0041C250 per earlier docs)** is used by both
  Chronosphere (Teleport CLSID) and Magnetron (Jumpjet CLSID) via the same code path.

- **Tick-cycle placement:** Detonate runs when a bullet reaches its target (mid-tick).
  The locomotor swap is applied synchronously in the same tick. The Jumpjet locomotor
  then drives its own state-machine on subsequent ticks (ascent → hover → descent/fall).

---

## 6. Current Rust Implementation Status

Summary from `src/` grep (research agent C):

| Component | Rust site | Status |
|-----------|-----------|--------|
| `WarheadType.is_locomotor` parsing | `src/rules/warhead_type.rs:67-68, 159` | **Parsed but offset comment is WRONG** — says `+0x153`, binary confirms `+0x15B` |
| `WarheadType.locomotor` GUID | — | **Not parsed** — CLSID field is entirely missing from the Rust struct |
| `WeaponType.is_mag_beam` parsing | `src/rules/weapon_type.rs:174-175, 274` | Parsed. Offset comment `+0x15c` is correct. |
| Warhead effect dispatch | `src/sim/combat/mod.rs:1260-1277` | **`_wh_id` is explicitly ignored** — no dispatch exists. All warheads currently collapse to raw damage. |
| Locomotor override infrastructure | `src/sim/movement/locomotor.rs:175-178, 315-345, 381-400` | Exists for `Teleport` and `DropPod`. `OverrideKind` enum has no `Magnetron` / `Jumpjet-forced` variant. |
| Jumpjet air-movement | `src/sim/movement/air_movement.rs:87-98` | Complete state machine (`Landed → Ascending → Cruising/Hovering → Descending`) for native jumpjet units |
| MagnaBeam visual | — | **Not rendered** — `is_mag_beam` flag is parsed but not used by the render layer |
| MagnaBeamColor parsing | — | Not in `src/rules/` |

**What's missing to implement Magnetron faithfully:**

1. Fix the `is_locomotor` offset comment in `warhead_type.rs:67` (change `+0x153` → `+0x15B`).
2. Parse the `Locomotor=` CLSID into the warhead struct (16 bytes, offset 0x15C). For our
   use case the CLSID can be mapped to an enum (Teleport / Jumpjet / Drive / …) rather
   than stored as raw GUID bytes.
3. Add an `OverrideKind::Jumpjet` (or similar "force-lifted") variant with a policy for
   duration / end condition.
4. Dispatch in combat: when a damage event's warhead has `is_locomotor=true`, look at
   its resolved `locomotor` kind and invoke the corresponding `LocomotorState::begin_override`
   on the target entity — **but only if** the target passes the §3.4 gates (ground unit,
   not already deployed, type's "ChronoTrigger" threshold satisfied).
5. Render `MagneticBeam` visual in the bullet-impact or weapon-fire render path (driven by
   `is_mag_beam`, color = `MagnaBeamColor`).
6. Parse `[General] MagnaBeamColor=R,G,B` and plumb to render.

**Do not implement:**
- Do not reuse `DirectRocker`/`Rocker` paths — they are unrelated knockback/shake effects
  (§3.3). The existing docs that grouped them with IsLocomotor are wrong.

---

## 7. Open Questions

1. ~~**Bullet+0xB0 semantics**~~ — **RESOLVED 2026-04-19**: Bullet+0xB0 is the firer's
   TechnoClass pointer per BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md. The decompiler's
   cast to int* for the Apply_area_damage call is an artifact of __fastcall register
   scheduling. The `+0x2AC` / `+0x2B0` offsets off this pointer are standard
   TechnoClass fields (presumed ChronoSphere-building warp-target link state for
   buildings; see §7 item #3 for TechnoClass semantics).

2. **End-of-lift teardown timing** — **FULLY RESOLVED 2026-04-19**:

   The kill occurs in `JumpjetLocomotionClass::Process` state 5 (Abort/Emergency)
   at `FUN_0054CA90`, following the same pattern as TeleportLocomotion's
   `PostWarpValidation` at 0x007187A0.

   **Flow summary:**

   a) The `LocomotorBeam` warhead is a **zero-HP-damage** weapon. The IsLocomotor
      branch in `WarheadTypeClass::Detonate` never calls `Apply_area_damage`.
   b) `TechnoClass::PerformDeploy` (0x00710000) creates a Jumpjet piggyback on
      the target (via CoCreateInstance with the warhead's Locomotor GUID).
   c) `BuildingClass::DeployUnit_ChronoWarp` (0x0070FEE0) is called when the
      piggyback ends; its altitude > 0 branch sets `target.flag_0x425 = 1`,
      `target.flag_0x427 = 1`, stores parent in `target+0x428/0x42C`, and calls
      `target.vtable[0x3A0] StopFiring` — but does NOT kill.
   d) Subsequent ticks run JumpjetLocomotion Process → state 5 (Abort/Emergency,
      `FUN_0054CA90`).

   **State 5 kill logic (verified at FUN_0054CA90):**

   ```
   iVar9 = target.vtable[0x1AC](current_cell, -1, -1, 0, 1);  // CanEnterCell
   if (target is Infantry && !CellClass::CheckCellPassability(cell))
       iVar9 = 7 (MOVE_NO)

   if (((target.type+0xD94 == 0) || cell passable) && iVar9 == 0) {
       state = 4;                       // can land → normal descent
   } else {
       target.ShouldSelfDestruct /* +0x3CD */ = 1;
       target.vtable[0x3A0]();          // KillSelf (same as PostWarpValidation)
       if (target.LinkedBuilding /* +0x2D8 */) {
           FUN_006B0AE0(target.ChronoSourceBuilding /* +0x428 */, 0);
           target.LinkedBuilding.vtable[0x20](1);   // AnimClass::Remove(1)
           target.LinkedBuilding = 0;
       }
       // Scatter-target cleanup paths
   }
   ```

   **Kill condition**: the target's OWN locomotor's `CanEnterCell` check fails for
   the current cell. For a DRIVE-native ground unit (tank), the Jumpjet-lifted
   position typically becomes unreachable by ground pathfinding — the unit can't
   "land" back where it was because (a) the Magnetron itself often occupies the
   adjacent cell, (b) airborne-to-ground transition isn't a valid pathfinding
   move for a DRIVE locomotor, or (c) the original cell may have been taken by
   another unit during the lift. When CanEnterCell returns `MOVE_NO (7)`, the
   ShouldSelfDestruct + KillSelf path fires.

   **The `+0xD94` check** on target's type: likely `Harvester` or a special-class
   flag that SKIPS the kill for specific unit types (they can survive the drop).
   Combined with `CellClass::CheckCellPassability`, it exempts harvesters/specials
   from the death path.

   **Skip-death escape paths (LAB_0054cfb7 / jumps to LAB_0054d019)**:
   - Target is on a bridge cell (flag 0x100) with sufficient altitude
   - Target is on a beach/water cell with `LandType==6` AND `unaff_BL=0`
   - Target has already cleared its deployed flag (`0x6AD == 0`)

   **Tail handler (LAB_0054D019)**: regardless of kill/survive, the function plays
   crash voice `0x117C` (`(**locomotor.vtable[0x0])(0x117C, 0)`) when the unit
   enters state 6 (terminal) — this is the "Magnetron death scream" audible in-game.

   **Implication for Rust**: faithful implementation requires:
   - Piggyback lifecycle for Jumpjet locomotor swap
   - Per-tick state machine with state-5 abort-check
   - CanEnterCell validation using the target's NATIVE locomotor rules (not the
     piggyback's)
   - `ShouldSelfDestruct` flag + `KillSelf` hook (analogous to entity destruction)
   - LinkedBuilding cleanup hook (for ChronoSphere compatibility; Magnetron won't
     have a LinkedBuilding to clean up, so the branch is effectively no-op for
     Magnetron but must exist for faithful shared code with Chronosphere)

3. ~~**`firer + 0x2AC` and `firer + 0x2B0` field semantics**~~ → **RESOLVED 2026-04-19**:
   TechnoClass+0x2AC = LocomotorTarget, TechnoClass+0x2B0 = LinkedBuilding (see §2.3).
   For Magnetron units these are always 0; the pre-deploy gate passes trivially.
   Rust implementation does NOT need these fields on a Magnetron-type unit — they are
   only relevant for ChronoSphere buildings.

4. ~~**ChronoTrigger field at type_class+0x380**~~ → **RESOLVED 2026-04-19**: the field
   at TechnoTypeClass+0x380 is `SizeWeight` (double), not ChronoTrigger. The gate is
   `target.type.SizeWeight < bullet.Damage`, i.e., a weapon-damage-vs-unit-mass check
   that determines lift eligibility. See §2.3 for the full gate. (True `ChronoTrigger`
   is an INI bool parsed onto RulesClass+0xBF8, unrelated.)

5. ~~**Double-dispatch against buildings**~~ → **RESOLVED 2026-04-19 (partial)**:
   Multiple pre-fire gates in `TechnoClass::GetFireError` (0x006FC0B0) block
   Magnetron from firing at various invalid targets:
   - **IsLocomotor weapon + target already chronoshifted** (`FUN_00746DB0` returns
     nonzero when `target+0x6E1 != 0 || target+0x6E2 != 0`) → FIRE_ILLEGAL.
     Magnetron can't hit units already in a warp/deployed state.
   - **IsLocomotor weapon + target.type+0xD94 flag + target+0x674 locomotor-ptr
     checks** → FIRE_ILLEGAL for some type class of immune units.
   - (The per-target gate `target.type.SizeWeight < bullet.Damage` in
     WarheadTypeClass::Detonate's IsLocomotor branch is the final eligibility
     check; see §2.3.)

   Still not resolved in this session: what happens if the Magnetron *does* hit a
   building via AoE (CellSpread=0, but weapon could theoretically impact on a
   cell containing a building). From Detonate §3.4: the branch's `flag & 4`
   short-circuit handles the case — if target is flagged as a techno but not a
   valid unit/infantry type, the branch early-returns before PerformDeploy. No
   observed damage-to-building path exists in the IsLocomotor dispatch — the
   primary weapon is a "lift-only, zero actual damage" weapon against buildings.
   Requires in-game test to confirm.

---

## Sources

**Ghidra addresses decompiled/disassembled:**
- `0x0075D590` — WarheadTypeClass::ReadINI (full disasm; verified offsets 0x14B-0x17C)
- `0x004690B0` — WarheadTypeClass::Detonate (full decomp; verified IsLocomotor dispatch)
- `0x00489280` — Apply_area_damage (full decomp; verified Rocker=0x14E, not IsLocomotor)
- `0x00710000` — TechnoClass::PerformDeploy (full decomp; verified CoCreateInstance_Locomotor swap)
- `0x00468D80` — BulletClass::BulletDetonation (caller of Detonate)
- `0x00772080` — WeaponTypeClass::ReadINI (full disasm; verified IsMagBeam=0x15C)
- String xrefs: `IsLocomotor` (0x847d3c), `IsMagBeam` (0x84928c), `DirectRocker` (0x847dd8),
  `Rocker` (0x847de8)

**Existing reports referenced (and corrected where needed):**
- `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md` — IsLocomotor=0x15B confirmed; "changes locomotor" description accurate
- `WARHEAD_DETONATE_GHIDRA_REPORT.md` — **needs correction**: the "Knockback (Step 9)" section is
  actually Rocker (0x14E) in Apply_area_damage, not IsLocomotor (0x15B). The "else if
  warhead->IsLocomotor → locomotor hijack" line at §5 is correct but the cross-reference to
  the Apply_area_damage knockback logic is mislinked.
- `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` — its "Row 7 | Offset 0x16C | IsLocomotor" line is a
  transcription error (should be 0x15B, and 0x16C is Airstrike). Also **needs correction**.
- `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` — IsMagBeam=0x15C confirmed
- `DRIVE_LOCOMOTION_CLASS.md` — Begin_Piggyback / End_Piggyback sequence (confirmed)
- `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — referenced for Jumpjet state machine
- `PARASITE_CLASS_GHIDRA_REPORT.md` — uses `WarpAttachClass` base shared with PerformDeploy detach

**INI files checked:**
- `ini/rulesmd.ini` — `[TELE]`, `[MagneticBeam]`, `[MagneticBeamE]`, `[MagneShake]`,
  `[LocomotorBeam]`, `[MagneShakeWH]`, `[InvisibleHigh]`, `[General]`, `[Warheads]` (entry 87)
- `ini/artmd.ini` — no Magnetron-specific entries (confirmed: no custom anims)
- `ini/rules.ini`, `ini/art.ini` — no Magnetron entries (YR-only)
