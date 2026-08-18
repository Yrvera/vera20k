# AAHeatSeeker2 Arming / Proximity Detector - Ghidra Research Report

**Date:** 2026-05-20  
**Binary:** `gamemd.exe` (Yuri's Revenge)  
**Address(es):** `BulletClass::Fire @ 0x00468670`, `BulletClass::AI @ 0x004666E0`, `ProximityDetector::Init @ 0x004E1100`, `ProximityDetector::Set/Arm @ 0x004E1130`, `ProximityDetector::Check @ 0x004E11F0`, `BulletTypeClass::ReadINI @ 0x0046BEE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** AAHeatSeeker2 `Arm=2`, `Proximity=no`, `Ranged=yes`, `ROT=60` interaction with the live `BulletClass` arming/proximity detector path.  
**Non-Scope:** full homing turn math, target invalidation notification writers, damage math, DRAGON rendering, and non-AAHeatSeeker2 projectile families.  
**Confidence:** High for the scoped detector init/arm/check thresholds and the `Proximity=no` verdict for this path.  
**Active in YR:** Yes. The path is reached from normal stock-YR `BulletClass::Fire` and per-tick `BulletClass::AI` for bullets whose projectile type has `ROT > 0` or `Ranged=yes`; AAHeatSeeker2 has both in `rulesmd.ini`.

## 1. Overview

AAHeatSeeker2's detector is not disabled by `Proximity=no`. The key is parsed into `BulletTypeClass+0x29F`, but the live `BulletClass::AI` call gate for `ProximityDetector::Check` uses `ROT` and `Ranged`, not `Proximity`.

For AAHeatSeeker2, `ROT=60` is already enough to keep the detector active. `Ranged=yes` is also a live gate for non-homing bullets, but it is redundant for this projectile because `ROT > 0`.

## 2. Key Offsets

| Owner | Offset | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `BulletClass` | `+0x9C/+0xA0/+0xA4` | current bullet coordinate passed to detector arm/check | `BulletClass::Fire @ 0x00468A63`; `BulletClass::AI @ 0x00467B7A+` | Yes |
| `BulletClass` | `+0xAC` | `BulletTypeClass*` | `BulletClass::Init @ 0x004664C0` | Yes |
| `BulletClass` | `+0xB8..+0xDF` | embedded `ProximityDetector` used by this path | `BulletClass::Fire @ 0x00468A8D` loads `ECX = this+0xB8` before calling `0x004E1130` | Yes |
| `BulletClass` | `+0x10C` | target object pointer, re-read by homing path | `BulletClass::AI @ 0x004666E0` | Yes |
| `BulletTypeClass` | `+0x29F` | parsed `Proximity=` bool | `BulletTypeClass::ReadINI @ 0x0046C0B7..0x0046C0C8` | Parsed yes; not consumed by this path |
| `BulletTypeClass` | `+0x2A0` | parsed `Ranged=` bool | `BulletTypeClass::ReadINI @ 0x0046C0D5..0x0046C0E8`; AI gate | Yes |
| `BulletTypeClass` | `+0x2DC` | parsed `ROT=` int | `BulletTypeClass::ReadINI @ 0x0046BEE0`; Fire/AI gates | Yes |
| `BulletTypeClass` | `+0x2F0` | parsed `Arm=` int | `BulletTypeClass::ReadINI @ 0x0046BEE0`; Fire call setup | Yes, conditional on target kind |

## 3. Detector Layout and Semantics

| Detector offset | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|
| `+0x00` | timer start for a broader/max timer set by `Set/Arm`; not used by `Check` in this slice | `0x004E1100`, `0x004E1130`, `0x004E11F0` | Conditional; initialized live, no scoped consumer |
| `+0x08` | max/outer duration; `Fire` passes `0x7FFFFFFF`, so `Set/Arm` stores that instead of `Arm` | `0x00468A69`, `0x004E113F..0x004E1158` | Yes, initialized; not the arming gate used by `Check` |
| `+0x0C` | arming timer start frame | `0x004E1163` writes current frame; `0x004E11F6` reads it | Yes |
| `+0x14` | arming delay used by `Check` | `0x004E1171`; `0x004E11F9..0x004E1211` | Yes |
| `+0x18/+0x1C/+0x20` | fixed reference coordinate copied at launch | `0x004E1176..0x004E1186`; `0x004E1217..0x004E1237` | Yes |
| `+0x24` | closest-distance sample | `0x004E11E1`, `0x004E1290`, `0x004E12A1` | Yes |

`ProximityDetector::Init @ 0x004E1100` sets both timer starts to `g_CurrentFrameCounter` (`0x00A8ED84` in the assembly context), clears the arming delay, clears the reference coordinate, and clears the closest-distance sample. Active in YR: Yes, as normal object construction/initialization state, but this report verified the live `Fire` re-arm that matters for AAHeatSeeker2 launch.

## 4. Fire-Time Arming

`BulletClass::Fire @ 0x00468670` calls `ProximityDetector::Set/Arm @ 0x004E1130` with:

| Argument | Value for AAHeatSeeker2 path | Evidence | Active in YR |
|---|---|---|---|
| detector `this` | `BulletClass+0xB8` | `0x00468A8D` | Yes |
| current coord | copy of `BulletClass+0x9C/+0xA0/+0xA4` | `0x00468A63..0x00468A8C` | Yes |
| reference coord | target/reference coordinate argument already on the Fire stack | `0x00468A80..0x00468A8C`; `Set/Arm @ 0x004E1176..0x004E1186` | Yes |
| arm delay | `0` for target object `WhatAmI()==2`, otherwise `BulletTypeClass+0x2F0` | `0x00468A3F..0x00468A5D` | Conditional |
| max/outer duration | `0x7FFFFFFF` | `0x00468A69` | Yes |

For stock `[AAHeatSeeker2]`, `Arm=2` is therefore live for non-`WhatAmI()==2` targets. If the target object exists and its vtable `+0x2C` type check returns `2`, `BulletClass::Fire` passes arm delay `0` instead. Active in YR: Conditional; this is on the standard YR fire path and depends on the target object's runtime class.

`Set/Arm` stores `max(param4, param5)` at detector `+0x08`. Because `BulletClass::Fire` passes `param5 = 0x7FFFFFFF`, detector `+0x08` becomes `0x7FFFFFFF`; the real `Check` arming delay is detector `+0x14`, which receives the `Arm` value or the aircraft override `0`. Active in YR: Yes.

Tiny detail: `Set/Arm` initializes detector `+0x24` with the full integer 3D distance from current coord to reference coord. Later `Check` compares and stores half-distance samples. This means the first armed sample almost never reports overshoot merely because it is farther than the launch sample; it usually seeds the half-distance baseline first. Active in YR: Yes.

## 5. Per-Tick Check Gate

At the end of `BulletClass::AI @ 0x004666E0`, the detector is skipped only when both conditions are true:

```text
BulletType.ROT < 1 AND BulletType.Ranged == false
```

Otherwise `ProximityDetector::Check` is called. Active in YR: Yes. Evidence is the decompiled branch immediately before the `ProximityDetector::Check` call in `BulletClass::AI @ 0x004666E0`.

For AAHeatSeeker2:

| Key | Stock value | Effect in this gate | Active in YR |
|---|---:|---|---|
| `ROT` | `60` | makes detector eligible even if `Ranged` were false | Yes |
| `Ranged` | `yes` | also makes detector eligible; redundant for AAHeatSeeker2 because `ROT > 0` | Yes |
| `Proximity` | `no` | no effect on this gate | Parsed yes; live for this path no |

`Proximity=no` is therefore parsed-only for this AAHeatSeeker2 arming/proximity detector path. This report does not claim the field has no readers anywhere else in the binary.

## 6. Check Timing and Return Values

`ProximityDetector::Check @ 0x004E11F0` uses the global frame counter and detector `+0x0C/+0x14`:

1. If `start_frame != -1` and `current_frame - start_frame < arm_delay`, it returns `0`.
2. If the remaining delay is nonzero, it returns `0`.
3. Once armed, it computes integer 3D distance from the current bullet coordinate to detector `+0x18/+0x1C/+0x20`, then divides by 2 using integer arithmetic.
4. If half-distance `< 0x20`, it returns `1`.
5. Else if half-distance `< 0x100` and previous closest sample `< half-distance`, it returns `2`.
6. Otherwise it stores half-distance to detector `+0x24` and returns `0`.

| Return | Meaning in this slice | Threshold | Evidence | Active in YR |
|---:|---|---|---|---|
| `0` | not armed yet, or no close/overshoot condition this tick | arming not elapsed, or update closest sample | `0x004E1201..0x004E1211`, `0x004E12A1..0x004E12A4` | Yes |
| `1` | close enough to reference coordinate | half-distance `< 0x20` | `0x004E1276..0x004E1286` | Yes |
| `2` | moved away after being within the arming/proximity window | half-distance `< 0x100` and previous sample `< current sample` | `0x004E1289..0x004E129E` | Yes |

`BulletClass::AI` treats detector return `2` specially for targets whose type has byte `+0xD94` set: it rewrites return `2` to `1`. Active in YR: Conditional; this is verified in `BulletClass::AI @ 0x004666E0`, but the target type flag inventory is outside this slot.

## 7. Reference Coordinate Does Not Track the Target

The detector reference coordinate is copied once in `ProximityDetector::Set/Arm` at launch. `ProximityDetector::Check` reads detector `+0x18/+0x1C/+0x20`; it does not fetch the current target object coordinate.

The homing path separately re-reads the target object's coordinate in `BulletClass::AI` before `BulletClass::HomingTrack @ 0x005B20F0`, but that value is not written back into the detector reference fields in the scoped path. Active in YR: Yes. Evidence: `BulletClass::Fire @ 0x00468A8D..0x00468A93` is the scoped `Set/Arm` call; `BulletClass::AI @ 0x004666E0` calls `ProximityDetector::Check` but not `ProximityDetector::Set/Arm`.

Parity consequence: AAHeatSeeker2 can home toward a moving target while its proximity/overshoot detector is still checking against the launch-time reference coordinate. That is a verified binary finding, not an inference from INI names.

## 8. INI Keys

| File | Section | Key | Value | Consumer | Active in YR |
|---|---|---|---|---|---|
| `ini/rulesmd.ini:25678` | `[AAHeatSeeker2]` | `Arm` | `2` | `BulletTypeClass+0x2F0`, passed to detector unless target `WhatAmI()==2` | Conditional |
| `ini/rulesmd.ini:25682` | `[AAHeatSeeker2]` | `Proximity` | `no` | parsed to `BulletTypeClass+0x29F`; not checked by scoped AI gate | Parsed yes; live here no |
| `ini/rulesmd.ini:25683` | `[AAHeatSeeker2]` | `Ranged` | `yes` | `BulletTypeClass+0x2A0`; keeps detector enabled when `ROT < 1` | Yes, but redundant here |
| `ini/rulesmd.ini:25687` | `[AAHeatSeeker2]` | `ROT` | `60` | `BulletTypeClass+0x2DC`; selects homing branch and detector eligibility | Yes |

`ini/rules.ini` contains the same `[AAHeatSeeker2]` values at lines `18505..18514`; YR `rulesmd.ini` is the priority source and matches for this slice.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BulletTypeClass::ReadINI` parse of `Arm`, `Proximity`, `Ranged`, `ROT` | verified | `0x0046BEE0`; parse stores include `+0x29F`, `+0x2A0`, `+0x2DC`, `+0x2F0`; `rulesmd.ini:25678..25687` | none for this slice |
| `ProximityDetector::Init` | verified | `0x004E1100` | none for this slice |
| `ProximityDetector::Set/Arm` | verified | `0x004E1130`; Fire call setup `0x00468A63..0x00468A93` | none for this slice |
| `ProximityDetector::Check` arming and thresholds | verified | `0x004E11F0`; assembly context `0x004E1276..0x004E129E` | none |
| `BulletClass::Fire` AAHeatSeeker2 detector setup | verified | `0x00468670`, especially `0x00468A3F..0x00468A93` | none for this slice |
| `BulletClass::AI` detector call gate | verified | `0x004666E0` decompile branch before `ProximityDetector::Check` | exact call-site assembly address not separately listed by tool output; decompile is unambiguous |
| Detector reference coordinate update after launch | verified | `Set/Arm @ 0x004E1176..0x004E1186`; `Check @ 0x004E1217..0x004E1237`; AI uses `Check` only in scoped path | none for this slice |
| Target type flag `+0xD94` that rewrites detector return `2` to `1` | touched-not-exhausted | `BulletClass::AI @ 0x004666E0` | target type flag inventory is out of scope |

## 10. Open Questions - Final State

`[RESOLVED] OQ-AAH-PROX-001` - Does `Proximity=no` disable the AAHeatSeeker2 detector? No for this path; the AI gate uses `ROT`/`Ranged`, not `Proximity`. Evidence: `BulletTypeClass::ReadINI @ 0x0046C0B7..0x0046C0C8`; `BulletClass::AI @ 0x004666E0`.

`[RESOLVED] OQ-AAH-PROX-002` - What are the detector thresholds and returns? Return `1` when half-distance `< 0x20`; return `2` when half-distance `< 0x100` and increasing from the previous sample; otherwise return `0`. Evidence: `ProximityDetector::Check @ 0x004E1276..0x004E12A4`.

`[RESOLVED] OQ-AAH-PROX-003` - Does `Arm=2` mean frame-count gating? Yes; `Check` uses `g_CurrentFrameCounter - start_frame >= arm_delay`. Evidence: `0x004E11F6..0x004E1211`.

`[RESOLVED] OQ-AAH-PROX-004` - Does the detector reference coordinate update as homing target coordinates update? No in this scoped path. Evidence: launch-time `Set/Arm @ 0x004E1130`; per-tick `Check @ 0x004E11F0`; no scoped AI `Set/Arm` call.

`[RESOLVED] OQ-AAH-PROX-005` - Is this TS legacy or standard YR? Standard YR-live. Evidence: normal `BulletClass::Fire` and `BulletClass::AI` paths, stock `rulesmd.ini` AAHeatSeeker2 values.

`[DEFERRED] OQ-AAH-PROX-006` - Which target type sets the `+0xD94` flag that rewrites return `2` to `1`? Category: out-of-scope. Next step: target type flag inventory if this edge case becomes implementation-blocking.

## Sources

- Ghidra decompile/read-only:
  - `BulletClass::Init @ 0x004664C0`
  - `BulletClass::Fire @ 0x00468670`
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::HomingTrack @ 0x005B20F0`
  - `BulletTypeClass::ReadINI @ 0x0046BEE0`
  - `ProximityDetector::Init @ 0x004E1100`
  - `ProximityDetector::Set/Arm @ 0x004E1130`
  - `ProximityDetector::Check @ 0x004E11F0`
- Assembly context/read-only:
  - `0x00468A3F..0x00468A93`
  - `0x004E1100..0x004E1127`
  - `0x004E113F..0x004E1186`
  - `0x004E11F6..0x004E12A4`
  - `0x0046C0B7..0x0046C0E8`
- Prior report:
  - `docs/research/GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
- INI:
  - `ini/rulesmd.ini:25678..25687`
  - `ini/rules.ini:18505..18514`
