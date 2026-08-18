# AAHeatSeeker2 Speed, Launch Vector, and Acceleration - Ghidra Report

**Date:** 2026-05-20  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `MissileLauncher` / `MissileLauncherE` `Speed=` propagation into `BulletClass`, ROT>0 launch velocity setup for `AAHeatSeeker2`, close-range speed caps, target-coordinate corrections that can affect the launch vector, and ROT>0 `BulletClass::AI` speed ramp/course-lock behavior.  
**Non-Scope:** arming/proximity detector internals, target invalidation, target-type homing differences, DRAGON rendering, GUARDWH detonation presentation, full generic non-homing projectile behavior.  
**Primary Addresses:** `TechnoClass::Fire_At @ 0x006FDD50`, `FUN_00773070 @ 0x00773070`, `BulletClass::Init @ 0x004664C0`, `BulletClass::Fire @ 0x00468670`, `BulletClass::AI @ 0x004666E0`, `BulletClass::HomingTrack @ 0x005B20F0`, `TechnoClass::Resolve_ArchiveTarget_Coords @ 0x0070BCB0`.  
**Confidence:** HIGH for the scoped launch speed/vector/ramp facts.  
**Active in YR:** Yes. Evidence: standard `TechnoClass::Fire_At -> BulletClass::Init/Fire -> BulletClass::AI` path is used by `[GGI] Secondary=MissileLauncher` / `EliteSecondary=MissileLauncherE` in `rulesmd.ini`.

## 1. Overview

`MissileLauncher Speed=30` and `MissileLauncherE Speed=40` become `BulletClass+0x110` target speed for the `AAHeatSeeker2` bullet. They do not become the bullet's stored launch velocity magnitude. For ROT>0 bullets, `TechnoClass::Fire_At` builds an initial direction vector, then `BulletClass::Fire` normalizes that velocity to magnitude `1.0`; `BulletClass::AI` ramps the magnitude toward target speed using `BulletTypeClass+0x2D0 Acceleration`.

The normal deployed Guardian GI path is a YR-live path, not TS legacy. No TS-only flag is required for any scoped finding.

## 2. INI Inputs

| INI key | Value | Consumer | Active in YR |
|---|---:|---|---|
| `rulesmd.ini [MissileLauncher] Speed` | `30` | `WeaponType+0xA8`; copied to `BulletClass+0x110` for ROT>0 | Yes: GGI secondary at `rulesmd.ini:3868`, weapon at `rulesmd.ini:22569-22578` |
| `rulesmd.ini [MissileLauncherE] Speed` | `40` | Same | Yes: GGI elite secondary at `rulesmd.ini:3910`, weapon at `rulesmd.ini:25123-25132` |
| `rulesmd.ini [AAHeatSeeker2] ROT` | `60` | Selects ROT>0 homing path and launch normalization | Yes: projectile at `rulesmd.ini:25678-25690` |
| `rulesmd.ini [AAHeatSeeker2] Acceleration` | absent -> default `3` | `BulletTypeClass+0x2D0`; speed ramp per AI tick | Yes: constructor default at `0x0046BBC0`; ReadINI override at `0x0046BEE0` |
| `rulesmd.ini [AAHeatSeeker2] CourseLockDuration` | absent -> default `0` | course-lock release logic in `BulletClass::AI` | Yes structurally; for Speed 30/40 it clears on the first AI tick |
| `rulesmd.ini [General] MissileROTVar` | `.25` | ROT wobble/sidewinder turn scaling before `HomingTrack` | Yes, but only turn-rate modulation; not target speed |

## 3. Key Offsets

| Class | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `WeaponTypeClass` | `+0xA8` | `Speed=` internal value | `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`; read by `FUN_00773070 @ 0x00773070` | Yes |
| `BulletTypeClass` | `+0x2D0` | `Acceleration`, default `3` | Constructor `0x0046BBC0`, ReadINI `0x0046BEE0` | Yes |
| `BulletTypeClass` | `+0x2DC` | `ROT` | Read by `FUN_00773070`, `BulletClass::Fire`, `BulletClass::AI` | Yes for `AAHeatSeeker2 ROT=60` |
| `BulletTypeClass` | `+0x2E0` | `CourseLockDuration`, default `0` | Constructor `0x0046BBC0`, ReadINI `0x0046BEE0`, AI `0x004666E0` | Yes |
| `BulletClass` | `+0xE8/+0xF0/+0xF8` | velocity vector as three doubles | `BulletClass::Fire @ 0x00468670`, AI `0x004666E0` | Yes |
| `BulletClass` | `+0x105` | course-lock flag | `BulletClass::AI @ 0x004666E0` | Yes, but immediately cleared for Speed 30/40 |
| `BulletClass` | `+0x108` | course-lock counter | `BulletClass::AI @ 0x004666E0` | Conditional: only meaningful when `CourseLockDuration > 0` |
| `BulletClass` | `+0x110` | target speed | `BulletClass::Init @ 0x004664C0`, Fire_At write to `bullet[0x44]` | Yes |
| `TechnoTypeClass` | `+0x6A4` | `RadialFireSegments` | GGI report/audit index; Fire_At checks it before launch-speed override | Conditional: GGI does not set it |

## 4. Core Findings

### 4.1 Speed=30/40 target-speed chain

**Finding:** For `AAHeatSeeker2`, `FUN_00773070 @ 0x00773070` returns raw `WeaponType+0xA8 Speed` because the projectile pointer is non-null and `BulletType+0x2DC ROT` is nonzero. For `MissileLauncher`, that value is `30`; for `MissileLauncherE`, `40`.

**Evidence:** `FUN_00773070` branches: if `weapon->Projectile` exists and `Projectile.ROT == 0`, it computes a non-homing value; otherwise it returns `weapon+0xA8`. `AAHeatSeeker2 ROT=60` is in `rulesmd.ini:25687`. `WeaponType+0xA8` is `Speed=` per `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`.

**Active in YR:** Yes. This is called from `TechnoClass::Fire_At @ 0x006FE53A`, the normal deployed GGI bullet path.

### 4.2 Bullet target speed is explicitly the weapon speed

**Finding:** `BulletClass::Init @ 0x004664C0` writes its speed parameter to `BulletClass+0x110`. In the ROT>0/vertical branch of `TechnoClass::Fire_At`, the code also writes `bullet[0x44] = weapon+0xA8`, which is byte offset `+0x110`. This makes target speed the raw weapon `Speed`, not the launch vector magnitude.

**Evidence:** `BulletClass::Init @ 0x004664C0` writes `param_7 -> +0x110`; `TechnoClass::Fire_At @ 0x006FDD50` writes `piStack_94[0x44] = *(weapon+0xA8)` when `Projectile.ROT > 0` or `Vertical=yes`.

**Active in YR:** Yes. `[AAHeatSeeker2] ROT=60` takes this branch.

### 4.3 Launch velocity is not Speed=30

**Finding:** `TechnoClass::Fire_At` constructs a velocity vector before calling `BulletClass::Fire`, but for ROT>0 bullets `BulletClass::Fire @ 0x00468670` normalizes the copied velocity vector to magnitude `1.0`. Therefore the stored initial velocity for `AAHeatSeeker2` starts at speed `1`, while target speed is `30` or `40`.

**Evidence:** `BulletClass::Fire @ 0x00468670` copies six 32-bit words from the caller vector into `+0xE8..+0xFF`; if `BulletType+0x2DC > 0`, it applies a zero-vector guard (`VelX=100.0`) and scales all three velocity components by `1.0 / sqrt(vx^2+vy^2+vz^2)`.

**Active in YR:** Yes. `AAHeatSeeker2 ROT=60` always takes the ROT>0 normalization path.

### 4.4 GGI launch heading uses current facing, then homing turns it

**Finding:** In `TechnoClass::Fire_At`, non-ROT/non-dropping bullets derive heading from the vector to target. ROT>0 bullets instead use the firer's current facing source (`vtable+0x308`) for initial horizontal heading, unless special radial-fire logic applies. `BulletClass::AI` then calls `BulletClass::HomingTrack @ 0x005B20F0` to turn the vector toward the current target coordinate.

**Evidence:** Fire_At branch around the `Projectile.ROT == 0 && Dropping == false` test: direct `atan2(target - source)` is used only for non-ROT; the else branch reads the firing object's facing. `BulletClass::AI @ 0x004666E0` later calls `HomingTrack @ 0x005B20F0` for ROT>0.

**Active in YR:** Yes for GGI. `RadialFireSegments` is absent on `[GGI]`; only Aegis-style units set it in stock `rulesmd.ini`.

### 4.5 Close-range cap exists but does not change AAHeatSeeker2 stored launch speed

**Finding:** Fire_At caps the pre-Fire launch speed variable to `distance_to_target / 2` when the target is very close. For ROT>0 `AAHeatSeeker2`, this does not change the final stored launch speed because `BulletClass::Fire` normalizes the velocity to `1.0` after copying it. It also does not change `BulletClass+0x110` target speed, which remains `Speed=30/40`.

**Evidence:** `TechnoClass::Fire_At @ 0x006FDD50` compares computed 3D source-target distance against the launch speed variable and assigns `distance/2` if smaller. Later `BulletClass::Fire @ 0x00468670` normalizes ROT>0 velocity to unit length, and Fire_At's ROT branch writes `weapon+0xA8` to `bullet+0x110`.

**Active in YR:** Conditional. The cap code is live in YR, but for standard GGI `AAHeatSeeker2` it is neutralized for stored initial speed by ROT>0 normalization. `MinimumRange=1` also prevents normal deployed GGI shots at true zero distance.

### 4.6 Coordinate correction can alter direction, not target speed

**Finding:** `TechnoClass::Resolve_ArchiveTarget_Coords @ 0x0070BCB0` normally returns the target object's center coordinate. It has special coordinate correction for a target in a chrono/locomotor transition path; that correction can alter the vector aimed at launch, but it does not alter `BulletClass+0x110` target speed.

**Evidence:** `Resolve_ArchiveTarget_Coords @ 0x0070BCB0` writes only the returned coordinate triple; the target-speed write remains in `BulletClass::Init @ 0x004664C0` / Fire_At's ROT branch.

**Active in YR:** Conditional. Normal vehicle/Rocketeer targets use center coords; the correction is only active for the specific target object/type and locomotor state checked inside `0x0070BCB0`.

### 4.7 Acceleration toward target speed

**Finding:** In `BulletClass::AI @ 0x004666E0`, ROT>0 bullets compute current speed from the velocity magnitude and adjust it toward `BulletClass+0x110`. If below target, they add `BulletType.Acceleration` and cap at target. For `AAHeatSeeker2`, the ramp is `1 -> 4 -> 7 -> ... -> 30` for normal and `1 -> 4 -> ... -> 40` for elite.

**Evidence:** ROT>0 AI path reads `TargetSpeed` from `param_1[0x44]`, reads `Acceleration` from `BulletType+0x2D0`, adds it when target speed is greater than current magnitude, caps to target if the add overshoots, and normalizes the vector to the new magnitude. `BulletTypeClass::Constructor @ 0x0046BBC0` sets `+0x2D0 = 3`; `AAHeatSeeker2` does not override `Acceleration=`.

**Active in YR:** Yes. This is the standard `AAHeatSeeker2` per-tick speed path.

### 4.8 Deceleration above target speed

**Finding:** If current velocity magnitude exceeds target speed, `BulletClass::AI` subtracts integer `Acceleration / 2` from current magnitude and clamps only at zero, not at target speed. For default `Acceleration=3`, deceleration step is `1`.

**Evidence:** ROT>0 AI path computes `local_14c = Acceleration`; in the `target < current` branch, it subtracts `(int)local_14c / 2` and clamps `<= 0` to zero before normalizing.

**Active in YR:** Yes structurally. For GGI `AAHeatSeeker2` normal launch starts below target speed, so this branch is mainly relevant after other behavior produces an above-target velocity.

### 4.9 Course-lock effect for Speed 30/40

**Finding:** `AAHeatSeeker2` has `CourseLockDuration=0`. In `BulletClass::AI`, default course lock clears immediately for `Speed=40` because target speed is greater than `0x27` (`39`). For `Speed=30`, it also clears on the first AI tick because the initial speed is `1.0` and the branch clears when `target_speed <= current_speed + 90.0`. Thus course lock does not materially delay Speed 30/40 homing turns.

**Evidence:** `BulletClass::AI @ 0x004666E0` course-lock branch checks `BulletType+0x2E0`; for zero duration, it clears `BulletClass+0x105` if target speed is `> 0x27` or if target speed is within the `current_speed + 90.0` threshold. `BulletClass::Fire @ 0x00468670` normalizes ROT>0 launch velocity to magnitude `1.0`.

**Active in YR:** Yes. The branch is live; for `MissileLauncher` and `MissileLauncherE` the observed effect is immediate unlock.

## 5. Integration Points

| Function | Status | Role | Active in YR |
|---|---|---|---|
| `TechnoClass::Fire_At @ 0x006FDD50` | verified | Allocates bullet, resolves target coord, builds launch vector, calls `BulletClass::Fire` | Yes |
| `FUN_00773070 @ 0x00773070` | verified | Selects raw weapon speed for ROT>0 projectiles | Yes |
| `TechnoClass::Resolve_ArchiveTarget_Coords @ 0x0070BCB0` | verified for speed scope | Supplies launch target coordinate; special correction affects coordinate only | Conditional |
| `BulletClass::Init @ 0x004664C0` | verified | Stores target pointer, target speed, warhead, damage, type, owner | Yes |
| `BulletClass::Fire @ 0x00468670` | verified | Copies launch velocity and normalizes ROT>0 velocity to magnitude 1 | Yes |
| `BulletClass::AI @ 0x004666E0` | verified | Ramps speed, course-lock logic, calls `HomingTrack` | Yes |
| `BulletClass::HomingTrack @ 0x005B20F0` | touched for speed scope | Turns/pitches already-normalized velocity; does not rewrite target speed | Yes |

## 6. Current Rust Implementation Status

The current Rust implementation has a homing movement module, but it does not match this slice precisely.

| File | Status vs binary slice |
|---|---|
| `src/rules/projectile_type.rs` | Parses `Acceleration` default `3`, `CourseLockDuration` default `0`, `Arm`, and `ROT`; this matches the scoped INI/default facts. |
| `src/sim/movement/homing_movement.rs` | Stores `weapon_speed` directly as current `HomingState.speed` at attach time (`speed: weapon_speed.max(SIM_ONE)`), whereas binary `AAHeatSeeker2` stores launch velocity magnitude `1.0` and ramps toward target speed. |

No Rust files were modified by this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MissileLauncher Speed=30` / `MissileLauncherE Speed=40` INI | verified | `rulesmd.ini:22569-22578`, `rulesmd.ini:25123-25132` | none |
| `AAHeatSeeker2 ROT=60`, default Acceleration/CourseLock | verified | `rulesmd.ini:25678-25690`, `0x0046BBC0`, `0x0046BEE0` | none |
| `FUN_00773070` speed selector | verified | `0x00773070`, xref from `0x006FE53A` | non-ROT formula out of scope |
| `BulletClass::Init` target-speed write | verified | `0x004664C0`, `+0x110` write | none |
| Fire_At ROT target-speed overwrite | verified | `0x006FDD50`, `bullet[0x44] = weapon+0xA8` | none |
| Fire_At close-range speed cap | verified | `0x006FDD50` distance/2 cap | exact pathological zero-distance vector behavior deferred |
| Fire_At initial heading source | verified | `0x006FDD50` ROT branch uses firer facing | radial-fire units out of scope |
| Resolve target coordinate correction | touched-not-exhausted | `0x0070BCB0` | exact chrono target visual case out of scope |
| `BulletClass::Fire` ROT normalization | verified | `0x00468670` | none |
| `BulletClass::AI` acceleration/deceleration | verified | `0x004666E0` | none |
| `BulletClass::AI` course-lock Speed 30/40 behavior | verified | `0x004666E0`, `0x0046BBC0` default CourseLockDuration | none |
| `BulletClass::HomingTrack` | touched-not-exhausted | `0x005B20F0` | full pitch/terrain behavior belongs to homing/target-type slot |

## 8. Open Questions - Final State

| ID | Final state |
|---|---|
| OQ-AHS2-SPEED-001 | RESOLVED: `Speed=30/40` comes from `WeaponType+0xA8` and is stored as `BulletClass+0x110` target speed for ROT>0 (`0x00773070`, `0x004664C0`, `0x006FDD50`). |
| OQ-AHS2-SPEED-002 | RESOLVED: Stored launch velocity for `AAHeatSeeker2` is magnitude `1.0`, not `30/40`, because `BulletClass::Fire @ 0x00468670` normalizes ROT>0 velocity. |
| OQ-AHS2-SPEED-003 | RESOLVED: Close-range cap exists in Fire_At but does not alter final stored ROT>0 launch speed or target speed for GGI (`0x006FDD50`, `0x00468670`). |
| OQ-AHS2-SPEED-004 | RESOLVED: Target-coordinate correction can alter launch direction only; target speed remains the weapon speed (`0x0070BCB0`, `0x004664C0`). |
| OQ-AHS2-SPEED-005 | RESOLVED: AI acceleration is +3/tick capped to target; deceleration is `Acceleration/2` with zero clamp (`0x004666E0`, `0x0046BBC0`). |
| OQ-AHS2-SPEED-006 | RESOLVED: Course lock clears immediately/first tick for Speed 30/40 with default `CourseLockDuration=0`; it does not materially hold the homing course for these missiles (`0x004666E0`). |
| OQ-AHS2-SPEED-007 | DEFERRED: exact zero-distance pathological launch vector if a mod bypasses `MinimumRange=1`; category: out-of-scope, not standard GGI/YR path. |

## Sources

- Live Ghidra decompilation of `gamemd.exe`:
  - `TechnoClass::Fire_At @ 0x006FDD50`
  - `FUN_00773070 @ 0x00773070`
  - `TechnoClass::Resolve_ArchiveTarget_Coords @ 0x0070BCB0`
  - `BulletClass::Init @ 0x004664C0`
  - `BulletClass::Fire @ 0x00468670`
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::HomingTrack @ 0x005B20F0`
  - `BulletTypeClass::Constructor @ 0x0046BBC0`
  - `BulletTypeClass::ReadINI @ 0x0046BEE0`
- INI data:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
- Prior reports checked:
  - `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
  - `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`
  - `BULLET_CLASS_AI_GHIDRA_REPORT.md`
  - `BULLETCLASS_TRAJECTORY_AND_HOMING.md`
  - `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`
  - `BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`
