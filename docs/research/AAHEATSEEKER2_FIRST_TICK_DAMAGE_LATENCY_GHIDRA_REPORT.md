# AAHeatSeeker2 First-Tick Damage Latency - Ghidra Research Report

**Date:** 2026-05-20
**Address(es):** `TechnoClass::Fire_At @ 0x006FDD50`, `BulletClass::Fire @ 0x00468670`, `BulletClass::AI @ 0x004666E0`, `LogicClass::PerTickUpdate @ 0x0055AFB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** first-tick scheduling and minimum detonation/damage latency for a newly fired `AAHeatSeeker2` `BulletClass` in the standard YR active-object tick loop.
**Non-Scope:** full `HomingTrack` turn math, complete target acquisition, exact all-aircraft altitude inventory, damage formula after `WarheadTypeClass::Detonate`.
**Confidence:** High for scheduler ordering and same-tick eligibility; High for binary detonation gates; Medium-High for standard-data minimum-frame examples because they combine binary thresholds with stock INI/art values.
**Active in YR:** Yes. The path is standard `TechnoClass::Fire_At -> BulletClass::Fire -> LogicClass::PerTickUpdate -> BulletClass::AI -> BulletClass::BulletDetonation` for stock `[GGI] Secondary=MissileLauncher`.

**2026-05-21 reconciliation note:** keep the scheduler and impact-time damage findings, but treat the numeric minimum-frame examples and the `current_speed * 90.0` close-hit threshold claim as provisional until rechecked against `AAHEATSEEKER2_HOMINGTRACK_EXACT_MATH_GHIDRA_REPORT.md`. The later exact-math report resolves stock `MissileROTVar=.25`, launch speed magnitude `1.0`, acceleration `3`, and a HomingTrack returned-distance comparison path that may supersede this report's scalar wording.

## 1. Overview

A bullet created by `TechnoClass::Fire_At` can run `BulletClass::AI` on the same game tick when it is revealed/inserted while `LogicClass::PerTickUpdate` is still walking the main logic vector. This is not a separate next-frame queue.

Same-frame real damage is possible: `BulletClass::AI` can reach `BulletClass::BulletDetonation @ 0x00468D80` in that first AI call, and detonation calls `WarheadTypeClass::Detonate`. `Arm=2` gates only `ProximityDetector::Check`; it does not gate the earlier ROT>0 close-hit branch.

## 2. Scheduling Evidence

| Finding | Evidence | Active in YR |
|---|---|---|
| `BulletTypeClass` marks bullets as logic-updated objects by default. | `BulletTypeClass::Constructor @ 0x0046BBC0` sets byte `+0x234 = 1`; prior `BULLETTYPECLASS_GHIDRA_REPORT.md` matches this. | Yes |
| `BulletClass::Fire` reveals the bullet, then submits it if active. | `BulletClass::Fire @ 0x00468670` calls `ObjectClass::Reveal`, then `DisplayClass::Submit_Object` when byte `Object+0x90` is nonzero. | Yes |
| `ObjectClass::Reveal` inserts logic-enabled object types into the main logic vector. | `ObjectClass::Reveal @ 0x005F5038..0x005F5040` calls `FUN_0055BAA0` with `ECX=0x87F778` when object type byte `+0x234` is set. | Yes |
| `FUN_0055BAA0` appends to the vector and sets object byte `+0x98`. | `FUN_0055BAA0 @ 0x0055BAA0` calls `DynamicVector__Insert` when `Object+0x98 == 0`; `DynamicVector__Insert @ 0x005519B0` writes the object pointer at `vector[count]` and increments count. | Yes |
| The main object AI loop does not snapshot count at loop entry. | `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619` calls `vtable+0x5C`, increments index, then reloads `*(param_1+0x10)` for the loop comparison. | Yes |

Consequence: if a firing techno's `AI` runs from this forward loop and appends a bullet before the loop reaches the new tail, the new bullet can receive `BulletClass::AI` on the same game frame. If `Fire_At` were called from a path after the main logic-vector pass, the first bullet AI would be next frame; that is a scheduling context difference, not a `BulletClass` delay.

## 3. Fire-To-Detonation Ordering

| Step | Ordering | Evidence | Active in YR |
|---|---|---|---|
| 1 | `TechnoClass::Fire_At` allocates and initializes the bullet. | `BulletClass::Allocate @ 0x0046B050` calls `BulletClass::Init @ 0x004664C0`; `TechnoClass::Fire_At @ 0x006FE55D` allocation site from parent report. | Yes |
| 2 | `TechnoClass::Fire_At` calls `BulletClass::Fire`, which reveals/inserts the bullet. | `TechnoClass::Fire_At @ 0x006FDD50`; `BulletClass::Fire @ 0x00468670`. | Yes |
| 3 | `BulletClass::Fire` sets detector arm delay: `0` for target `WhatAmI()==2`, otherwise projectile `Arm`. | `BulletClass::Fire @ 0x00468A3F..0x00468A63`; `[AAHeatSeeker2] Arm=2`. | Yes, conditional on target RTTI |
| 4 | First `BulletClass::AI` starts by calling `ObjectClass::AI`, then enters ROT>0 homing for `ROT=60`. | `BulletClass::AI @ 0x004666E0`; `[AAHeatSeeker2] ROT=60`. | Yes |
| 5 | AI ramps speed before close-hit testing: initial magnitude `1` becomes `4` on first AI for default `Acceleration=3`. | `BulletClass::Fire @ 0x00468670` normalizes ROT>0 velocity to `1.0`; AI adds `BulletType+0x2D0`, default `3`. | Yes |
| 6 | AI can set detonation before proximity check if the ROT>0 homing close-hit branch or height condition trips; exact scalar wording in this report is provisional. | `BulletClass::AI @ 0x004666E0` ROT>0 branch after `HomingTrack`; reconcile scalar math with `AAHEATSEEKER2_HOMINGTRACK_EXACT_MATH_GHIDRA_REPORT.md` before implementation. | Yes |
| 7 | Proximity detector runs later and is skipped only for `ROT < 1 && Ranged == false`; `AAHeatSeeker2` runs it. | `BulletClass::AI @ 0x004666E0` decompile branch before `ProximityDetector::Check`; `AAHeatSeeker2 ROT=60`, `Ranged=yes`. | Yes |
| 8 | Real damage is at `BulletClass::BulletDetonation`, not at `Fire_At`. | `BulletClass::AI @ 0x004666E0` calls `BulletClass::BulletDetonation @ 0x00468D80`, which calls `WarheadTypeClass::Detonate`. | Yes |

## 4. Minimum Standard-Data Latencies

Frame counts below are from the standard same-tick scheduling case: the firing techno is processed by the main logic loop, appends the bullet, and the bullet is reached later in the same loop. If first bullet AI is deferred by caller context, add one game frame.

Stock data used:

| Data | Value | Evidence | Active in YR |
|---|---:|---|---|
| `[GGI] Secondary` | `MissileLauncher` | `rulesmd.ini:3863..3868` | Yes |
| `[GGI] SecondaryFireFLH` | `80,0,90` | `artmd.ini:291..299` | Yes |
| `[MissileLauncher] Speed` | `30` | `rulesmd.ini:22569..22575` | Yes |
| `[MissileLauncher] MinimumRange` | `1` | `rulesmd.ini:22569..22578` | Yes |
| `[AAHeatSeeker2] Arm` | `2` | `rulesmd.ini:25678..25679` | Yes |
| `[AAHeatSeeker2] ROT` | `60` | `rulesmd.ini:25687` | Yes |
| `[AAHeatSeeker2] Acceleration` | absent -> default `3` | `BulletTypeClass::Constructor @ 0x0046BBC0` | Yes |
| Rocketeer / jumpjet cruise height | `JumpjetHeight=500` | `rulesmd.ini:3960`; jumpjet audit verifies `JumpjetHeight` parse/use | Yes |
| Aircraft flight level | `FlightLevel=1500` | `rulesmd.ini:67`; prior aircraft docs place `RulesClass+0x7B4` as `FlightLevel` | Yes |

| Target case | First relevant threshold | Minimum detonation frame in standard same-tick scheduling | Evidence and reasoning | Active in YR |
|---|---:|---|---|---|
| Normal ground target at legal minimum range | first AI: speed `4`, close threshold `360` leptons | same game frame as fire (`0` frame delay) | GGI secondary muzzle is 80 leptons forward and 90 up; a 1-cell target is about 176 horizontal leptons from muzzle, with total 3D distance below `360`. The ROT>0 close-hit branch precedes the `Arm=2` proximity gate. | Yes |
| Rocketeer / InfantryClass high-flying target | second AI: speed `7`, close threshold `630` leptons | next game frame after fire (`1` frame delay) | Rocketeer keeps `Arm=2` because `InfantryClass::WhatAmI()==0xF`; high target height `500` and muzzle Z `90` make first-AI 3D gap too large for `4*90`, but below `7*90` at the next AI. | Yes, conditional on airborne/high-flying state |
| True `AircraftClass` target with arm override | fifth AI: speed `16`, close threshold `1440` leptons for standard `FlightLevel=1500` | fourth game frame after fire (`4` frame delay) for a normal flight-level target | `BulletClass::Fire` passes arm `0` when `target->WhatAmI()==2`, but `ProximityDetector::Check` still needs proximity distances; the standard flight-level gap is far outside first-AI detector thresholds. The ROT close-hit threshold catches at speed `16` under the stock minimum-range/muzzle/flight-level inputs. | Yes, conditional on true AircraftClass target at normal flight level |

Important boundary: the true aircraft arm override makes same-frame detector detonation possible in principle for a true `AircraftClass` target already within detector thresholds, but normal stock flight-level aircraft are not close enough for that on the first AI. The normal stock-data minimum above is therefore four frames after fire when same-tick first AI occurs.

## 5. Open Questions - Final State

`[RESOLVED] OQ-AAH-LAT-001` - Can a bullet created by `TechnoClass::Fire_At` run `BulletClass::AI` on the same tick? Yes, when created during the main forward logic-vector loop; count is re-read after each iteration. Evidence: `0x0055B608..0x0055B619`, `0x0055BAA0`, `0x005519B0`.

`[RESOLVED] OQ-AAH-LAT-002` - Can it detonate and apply real damage on that same tick? Yes. `BulletClass::AI` can call `BulletClass::BulletDetonation`, and `BulletDetonation` calls `WarheadTypeClass::Detonate`. Evidence: `0x004666E0`, `0x00468D80`.

`[RESOLVED] OQ-AAH-LAT-003` - Does `Arm=2` forbid same-frame damage against ground targets? No. It gates `ProximityDetector::Check`, but the ROT close-hit branch runs before that detector result is decisive. Evidence: `BulletClass::AI @ 0x004666E0`; `ProximityDetector::Check @ 0x004E11F0`.

`[RESOLVED] OQ-AAH-LAT-004` - Does Rocketeer get the aircraft arm override? No. Rocketeer is `InfantryClass` (`WhatAmI()==0xF`), so it keeps `Arm=2`; its high-flying legality is a separate predicate. Evidence: target-type report; `BulletClass::Fire @ 0x00468A3F..0x00468A63`.

`[RESOLVED] OQ-AAH-LAT-005` - Does true AircraftClass arm `0` imply same-frame damage for normal aircraft? No for normal stock flight-level targets; it removes the detector arm latency, but distance/altitude still controls impact. Evidence: `rulesmd.ini:67`, `BulletClass::AI @ 0x004666E0`, `ProximityDetector::Check @ 0x004E11F0`.

`[DEFERRED] OQ-AAH-LAT-006` - Exact minimum for every aircraft type and non-cruise state. Category: out-of-scope. This report uses stock `FlightLevel=1500` as the normal true-Aircraft case; landing/takeoff/crashing states need a separate aircraft-state slice.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Main logic-vector iteration count behavior | verified | `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619` | none |
| Bullet logic insertion through `ObjectClass::Reveal` | verified | `0x005F5038..0x005F5040`, `FUN_0055BAA0 @ 0x0055BAA0`, `DynamicVector__Insert @ 0x005519B0` | none |
| Bullet type logic-enabled default | verified | `BulletTypeClass::Constructor @ 0x0046BBC0` sets byte `+0x234=1` | none |
| `TechnoClass::Fire_At -> BulletClass::Fire` | verified | `0x006FDD50`, `0x00468670`; parent report call site `0x006FE55D` | none |
| First AI speed ramp | verified | `BulletClass::Fire @ 0x00468670`, `BulletClass::AI @ 0x004666E0`, `BulletTypeClass::Constructor @ 0x0046BBC0` | none |
| ROT close-hit branch before detector | verified | `BulletClass::AI @ 0x004666E0`; prior speed report | exact `HomingTrack` math is separate slot |
| Proximity arming gate | verified | `ProximityDetector::Check @ 0x004E11F0`; arm report | none |
| Ground minimum latency | verified from binary + INI/art data | `artmd.ini:291..299`, `rulesmd.ini:22569..22578`, `0x004666E0` | none for standard data |
| Rocketeer minimum latency | verified from binary + INI/art data | `rulesmd.ini:3960`, target-type report, `0x004666E0` | full jumpjet state edge cases deferred |
| True aircraft normal flight-level minimum latency | touched-not-exhausted | `rulesmd.ini:67`, aircraft docs, `0x004666E0`, `0x004E11F0` | per-aircraft altitude/state inventory |

## Sources

- Ghidra decompile / disassembly, read-only:
  - `LogicClass::PerTickUpdate @ 0x0055AFB0`
  - `FUN_0055BAA0 @ 0x0055BAA0`
  - `DynamicVector__Insert @ 0x005519B0`
  - `ObjectClass::Reveal @ 0x005F4EC0`
  - `BulletTypeClass::Constructor @ 0x0046BBC0`
  - `TechnoClass::Fire_At @ 0x006FDD50`
  - `BulletClass::Allocate @ 0x0046B050`
  - `BulletClass::Fire @ 0x00468670`
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::BulletDetonation @ 0x00468D80`
  - `ProximityDetector::Check @ 0x004E11F0`
- Prior reports:
  - `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
  - `AAHEATSEEKER2_ARMING_PROXIMITY_DETECTOR_GHIDRA_REPORT.md`
  - `AAHEATSEEKER2_SPEED_LAUNCH_VECTOR_ACCELERATION_GHIDRA_REPORT.md`
  - `AAHEATSEEKER2_TARGET_TYPE_HOMING_GROUND_ROCKETEER_AIRCRAFT_GHIDRA_REPORT.md`
  - `BULLETTYPECLASS_GHIDRA_REPORT.md`
- INI/art:
  - `ini/rulesmd.ini`
  - `ini/artmd.ini`
