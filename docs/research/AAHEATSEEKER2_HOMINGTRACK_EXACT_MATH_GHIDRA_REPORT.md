# AAHeatSeeker2 HomingTrack Exact Math - Ghidra Research Report

**Date:** 2026-05-20  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `BulletClass::HomingTrack @ 0x005B20F0` and the immediate ROT>0 caller block in `BulletClass::AI @ 0x004666E0`, only for authoritative `AAHeatSeeker2` homing parity.  
**Non-Scope:** target invalidation writers, proximity detector internals, DRAGON draw-slot frame mapping, generic non-homing projectile behavior, all non-AAHeatSeeker2 projectile variants.  
**Confidence:** HIGH for caller-side dynamic ROT, yaw turn clamp, target-type aircraft flag effect, terrain pitch branch predicates, return value use, and velocity write-back. MEDIUM for naming of two tiny pitch helper functions because Ghidra has no semantic labels, though their arithmetic is verified.  
**Active in YR:** Yes. `[GGI] Secondary=MissileLauncher` uses `[AAHeatSeeker2] ROT=60`; `BulletClass::AI @ 0x004666E0` is the live per-tick bullet path and has the sole xref to `HomingTrack @ 0x005B20F0` at `0x00466D31`.

## 1. Overview

`AAHeatSeeker2` homing is not a simple "rotate by 60" step. `BulletClass::AI` first computes a per-tick dynamic turn allowance from `Rules.MissileROTVar`, a 15-frame cosine phase, `BulletType.ROT`, close-range distance, and course-lock state. `HomingTrack` then turns velocity yaw by at most that signed 16-bit allowance, modifies pitch through either ground-clearance avoidance or simple pitch tracking, integrates a rounded velocity step into the candidate position, and returns an adjusted target distance used by the caller's detonation check.

This report corrects a stale summary in the parent GGI report: stock YR `MissileROTVar` is `.25`, not `1.0`; close range multiplies the dynamic ROT integer by `1.5`, after which only the low byte is kept and shifted into 16-bit facing units.

## 2. Inputs and Key Offsets

| Input / field | Value for AAHeatSeeker2 | Evidence | Active in YR |
|---|---:|---|---|
| `[AAHeatSeeker2] ROT` | `60` | `ini/rulesmd.ini:25678-25690`; `BulletType+0x2DC` read in AI | Yes: selects ROT>0 branch |
| `[General] MissileROTVar` | `.25` | `ini/rulesmd.ini:74`; AI reads `RulesClass+0x598` via `g_Rules + 0x598` | Yes |
| `BulletType+0x294 Airburst` | default false | `BULLETTYPECLASS_GHIDRA_REPORT.md`; pushed to `HomingTrack` at `0x00466D13..0x00466D1A` | Conditional; false for AAHeatSeeker2 |
| `BulletType+0x299 VeryHigh` | default false | `BULLETTYPECLASS_GHIDRA_REPORT.md`; `0x00466D0C` | Conditional; false for AAHeatSeeker2 |
| `BulletType+0x29D Level` | default false | `BULLETTYPECLASS_GHIDRA_REPORT.md`; first pushed stack flag at `0x00466D06..0x00466D12` | Conditional; false for AAHeatSeeker2 |
| Target aircraft flag | strict `target->WhatAmI()==2` | `0x00466CC4..0x00466CE7`, prior target-type report | Conditional: true only for real `AircraftClass`, not Rocketeer infantry |
| Velocity vector | `BulletClass+0xE8/+0xF0/+0xF8` copied to local, then back | `0x00466B0C..0x00466B31`, `0x00466D49..0x00466DAF` | Yes |

## 3. AI Caller: Dynamic ROT and Sidewinder Input

Verified formula from `BulletClass::AI @ 0x00466BC2..0x00466D31`:

```text
phase = (bullet_id_like_value + g_CurrentFrameCounter) % 15
scalar = cos(phase * (1.0 / 15.0) * (2*pi)) * Rules.MissileROTVar
       + Rules.MissileROTVar
       + 1.0
turn_i = ftol(scalar * BulletType.ROT)
if distance_to_target_from_current_bullet_coord < 0x100:
    turn_i = ftol(turn_i * 1.5)
if BulletClass+0x105 course-lock flag is nonzero:
    turn_i = 0
rot_word = ((turn_i & 0xff) << 8) as signed 16-bit facing allowance
```

Constants verified from static Ghidra memory:

| Address | Value | Use | Active in YR |
|---|---:|---|---|
| `0x007E48F8` | `0.06666666666666667` | `1/15` phase scale | Yes |
| `0x007E3CC0` | `6.283185307179586` | `2*pi` | Yes |
| `0x007E1718` | `1.0` | scalar base add | Yes |
| `0x007E48F0` | `1.5` | close-range multiplier when target distance `< 256` | Yes |

For stock `AAHeatSeeker2`, far-range `turn_i` is approximately `ftol(60..90)` because `.25` makes the scalar range `[1.0, 1.5]`. Inside 256 leptons, the caller computes approximately `ftol(90..135)`. That is not a distance-proportional reduction; however, because the final value is stored as a signed 16-bit word after `low_byte << 8`, values above byte `127` wrap into a negative signed short and the facing helpers take `abs(short(rot_word))`. Therefore byte values `128..135` behave as effective absolute allowances `128..121`. This is the only close-range "reduction" verified in this path.

Active in YR: Yes. Evidence: caller block `0x00466BC2..0x00466D31`, `rulesmd.ini:74`, `rulesmd.ini:25687`. Course-lock suppression is structurally active, but prior speed report shows Speed 30/40 clears course lock before the HomingTrack call on the first AI tick.

## 4. HomingTrack Yaw Math

`HomingTrack @ 0x005B20F0` receives:

```text
ECX = CoordStruct* candidate_position in/out
EDX = Velocity3D* velocity in/out
stack +0x08 = target CoordStruct*
stack +0x0C = pointer to rot_word
stack +0x10 = aircraft flag
stack +0x14 = Airburst
stack +0x18 = composite word whose low byte is VeryHigh
stack +0x1C = Level
```

Valid target path:

1. Round current velocity components with `Math::ftol` and add them to the current bullet coordinate, writing the candidate position before steering math (`0x005B2350..0x005B23B8`).
2. Build `delta = target - candidate_position` and compute target distance (`0x005B23C7..0x005B2431`).
3. Compute current yaw from current velocity and desired yaw from `delta` using the binary facing conversion `atan2(...) - pi/2`, multiplied by `-32768/pi` (`0x007E2820`, `0x007E2818`).
4. If `abs((short)(desired - current)) <= abs((short)rot_word)`, snap yaw to desired (`Facing::IsWithinROT @ 0x005B2990`).
5. Otherwise call `Facing::GetTurnDelta @ 0x005B2950`; if the signed delta is negative subtract `rot_word`, otherwise add `rot_word`.
6. Rebuild horizontal velocity from the selected yaw and the existing horizontal magnitude: `VelX = sin(angle) * horiz_mag`, `VelY = -cos(angle) * horiz_mag` (`0x005B2555..0x005B25BD`).

Invalid/sentinel target path:

If target coordinate equals the sentinel triple at `0x00ABEF10/14/18` (all zero in static data), HomingTrack does not compute `target - position`. It turns the current velocity yaw toward fixed facing `0x2000` by the same ROT clamp, integrates the rounded velocity, and returns the velocity magnitude after the update (`0x005B2106..0x005B2347`).

Active in YR: Yes. The valid path is normal AAHeatSeeker2 tracking; the sentinel path is conditional on lost/null target coordinate and is reachable from `BulletClass::AI`'s null target handling.

## 5. Pitch and Aircraft Flag Effect

After yaw is rebuilt, HomingTrack computes current pitch from velocity Z versus horizontal magnitude, then chooses one of two pitch families.

Ground/Rocketeer-style path (`aircraft flag == false`):

- Terrain/ground-clearance pitch branch is enabled only if all of these are true: not `AircraftClass`, not `Level`, dynamic ROT group `((rot_word >> 7) + 1) >> 1 > 1`, and either `Airburst` is true or distance exceeds `0x300` leptons when `VeryHigh=false` (`0x600` when `VeryHigh=true`). For stock AAHeatSeeker2 (`Airburst=false`, `VeryHigh=false`, `Level=false`), this means non-aircraft targets use terrain avoidance only beyond 768 leptons.
- The branch samples a point six rounded velocity steps ahead, gets ground height, and adds bridge height `DAT_00ABEF44` if the cell flag `+0x140 & 0x100` is set (`0x005B263E..0x005B2703`).
- Desired clearance is `min(distance / 256, 5) * DAT_00ABEF50` for stock AAHeatSeeker2; the max becomes `10` only when `Airburst` or `VeryHigh` is true (`0x005B2703..0x005B2732`).
- If altitude error is below `-20`, candidate Z is raised by `18`; if above `+20`, candidate Z is lowered by `18` (`0x005B273D..0x005B2762`).
- If the error is below `-cell_height/2`, pitch is clamped toward `0x2000`; if above `+cell_height/2`, pitch is clamped toward `0x4800`; otherwise pitch is clamped toward `0x4000`. The clamp allowance is signed `rot_word / 2` (`0x005B2762..0x005B289A`).

Aircraft path (`aircraft flag == true`):

- A true `AircraftClass` target skips the terrain/ground-clearance block because the top-level pitch branch requires `aircraft flag == false`.
- It proceeds through the simpler pitch clamp path unless `Level` is true. This path uses tiny helpers at `0x005B2930` (16-bit add) and `0x005B2970` (signed 16-bit divide) before `Facing::ClampToROT @ 0x005B29C0`, then applies pitch with `Velocity::ApplyPitch @ 0x005B2A30`.

Rocketeer note:

An airborne Rocketeer can be a legal AA target, but it remains `InfantryClass::WhatAmI()==0xF`, so it passes `aircraft flag=false` to HomingTrack and follows the ground/Rocketeer pitch family above. Active in YR: Yes, conditional on target class; evidence `0x00466CC4..0x00466CE7` and prior `AAHEATSEEKER2_TARGET_TYPE_HOMING_GROUND_ROCKETEER_AIRCRAFT_GHIDRA_REPORT.md`.

## 6. Return Value and Caller Use

For valid targets, HomingTrack returns a `CoordStruct::Distance3D`-style integer distance using the post-step delta, but with final Z delta adjusted immediately before return: if `Airburst` is false, the Z delta is divided by 4 using signed arithmetic; if `Airburst` is true, the Z delta is set to zero (`0x005B28E5..0x005B291C`). For stock AAHeatSeeker2, `Airburst=false`, so the returned impact distance discounts vertical separation by 4.

`BulletClass::AI` stores the return in a local at `0x00466D40`, then checks:

```text
if returned_distance <= current_speed * 0.5 OR bullet height < 1:
    mark detonation/impact
```

Evidence: return stored after call at `0x00466D31..0x00466D40`; velocity magnitude and `* 0.5` comparison at `0x00466DB1..0x00466DF4`; `0x007E1738 = 0.5`.

Active in YR: Yes. This is the live ROT>0 close-impact condition for AAHeatSeeker2. It is separate from the proximity detector that runs later because `ROT>0`.

## 7. Output Velocity, Position, and Facing State

HomingTrack mutates the local velocity vector passed in EDX. `BulletClass::AI` copies the six 32-bit words of that local double-vector back into `BulletClass+0xE8/+0xF0/+0xF8` immediately after the call (`0x00466D49..0x00466DAF`). This is the authoritative per-tick output velocity for AAHeatSeeker2.

The candidate position is advanced inside HomingTrack, then the normal AI movement/occupancy path applies it through object virtual calls later in `BulletClass::AI` (`vtable+0x124`, `vtable+0x1B4`). At function end, AI writes packed last-cell XY to `BulletClass+0x14C` (`param_1[0x53]`) from the final position (`0x00467FBA`).

No separate `BulletClass` visual-facing field write was found in this HomingTrack/AI slice. The yaw and pitch are materialized as the rewritten velocity vector. Exact DRAGON `Rotates=yes` facing-to-SHP-frame arithmetic remains the neighboring render slot's responsibility; this slot only verifies that the motion-facing input is the post-HomingTrack velocity, not a fire-time facing.

Active in YR: Yes for velocity and position writes. Conditional/deferred for exact render frame selection; see `DRAGON_RENDER_AND_GUARDWH_IMPACT_PRESENTATION_GHIDRA_REPORT.md`.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| AI dynamic sidewinder/ROT input | verified | `0x00466BC2..0x00466D31`, `rulesmd.ini:74`, `0x007E48F8`, `0x007E3CC0`, `0x007E48F0` | none for AAHeatSeeker2 |
| Course-lock suppression of ROT word | verified | `0x00466CE9..0x00466D01`; prior speed report for immediate unlock | none |
| Close-range ROT handling | verified | `0x00466C8F..0x00466CB2`, multiplier `1.5`, low-byte shift | none |
| `Facing::IsWithinROT` | verified | `0x005B2990` | none |
| `Facing::GetTurnDelta` | verified | `0x005B2950` | none |
| `Facing::ClampToROT` | verified | `0x005B29C0` | none |
| `Velocity::ApplyPitch` | verified | `0x005B2A30` | none |
| Valid target yaw path | verified | `0x005B2350..0x005B25BD` | none |
| Sentinel target yaw path | verified | `0x005B2106..0x005B2347` | target-clear writer out of scope |
| Ground/Rocketeer pitch branch | verified | `0x005B25F1..0x005B28E5`, target-type prior report | none for AAHeatSeeker2 |
| True AircraftClass pitch branch | verified | `0x00466CC4..0x00466CE7`, `0x005B25F1..0x005B28E5` | none for homing math |
| HomingTrack return value use | verified | `0x005B28E5..0x005B291C`, `0x00466D31..0x00466DF4` | none |
| Velocity write-back | verified | `0x00466D49..0x00466DAF` | none |
| DRAGON frame mapping from velocity/facing | deferred | sibling render slot/report | out-of-scope |

## 9. Open Questions - Final State

[RESOLVED] OQ-AH2-HT-001 - What exact ROT value is passed into HomingTrack? It is `((ftol((cos(((id+frame)%15)/15*2pi)*MissileROTVar + MissileROTVar + 1) * ROT)`, optionally `*1.5` under 256 leptons, low byte only) `<< 8`, zeroed if course-locked. Evidence: `0x00466BC2..0x00466D31`.

[RESOLVED] OQ-AH2-HT-002 - Does close range reduce ROT? The explicit close-range operation is `turn_i = ftol(turn_i * 1.5)` when distance `< 0x100`; effective reduction can occur only after signed 16-bit wrapping for byte values above 127. Evidence: `0x00466C8F..0x00466CB2`.

[RESOLVED] OQ-AH2-HT-003 - How are yaw turns clamped? `IsWithinROT` uses `abs((short)(desired-current)) <= abs((short)rot)`, otherwise signed delta chooses `current +/- rot`. Evidence: `0x005B2990`, `0x005B2950`, `0x005B20F0`.

[RESOLVED] OQ-AH2-HT-004 - What does the aircraft flag do? Strict `WhatAmI()==2` skips non-aircraft terrain/ground-clearance pitch logic; Rocketeer remains non-aircraft for this branch. Evidence: `0x00466CC4..0x00466CE7`, `0x005B25F1..0x005B289C`.

[RESOLVED] OQ-AH2-HT-005 - What does HomingTrack return? For AAHeatSeeker2 it returns post-step target distance with Z delta divided by 4; AI compares it to `current_speed * 0.5` for close impact. Evidence: `0x005B28E5..0x005B291C`, `0x00466DB1..0x00466DF4`.

[RESOLVED] OQ-AH2-HT-006 - Where is output velocity written? AI copies HomingTrack's mutated local velocity back to `BulletClass+0xE8/+0xF0/+0xF8`. Evidence: `0x00466D49..0x00466DAF`.

[DEFERRED] OQ-AH2-HT-007 - Exact DRAGON SHP frame selection from velocity/facing. Category: out-of-scope; this slot verifies motion output only. Next step: slot 3 render draw-slot frame mapping.

## Sources

- Live/static Ghidra decompilation and disassembly:
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::HomingTrack @ 0x005B20F0`
  - `Facing::IsWithinROT @ 0x005B2990`
  - `Facing::GetTurnDelta @ 0x005B2950`
  - `Facing::ClampToROT @ 0x005B29C0`
  - `Velocity::ApplyPitch @ 0x005B2A30`
  - pitch helpers `0x005B2930`, `0x005B2970`
- Static data reads:
  - `0x007E48F8 = 1/15`
  - `0x007E3CC0 = 2*pi`
  - `0x007E1718 = 1.0`
  - `0x007E48F0 = 1.5`
  - `0x007E1738 = 0.5`
  - `0x007E2810 = -2*pi/65536`
  - `0x007E2818 = -32768/pi`
  - `0x007E2820 = pi/2`
- INI:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
- Prior reports:
  - `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
  - `AAHEATSEEKER2_SPEED_LAUNCH_VECTOR_ACCELERATION_GHIDRA_REPORT.md`
  - `AAHEATSEEKER2_TARGET_TYPE_HOMING_GROUND_ROCKETEER_AIRCRAFT_GHIDRA_REPORT.md`
  - `BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `BULLETCLASS_TRAJECTORY_AND_HOMING.md`
  - `DRAGON_RENDER_AND_GUARDWH_IMPACT_PRESENTATION_GHIDRA_REPORT.md`
