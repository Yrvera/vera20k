# Frame Basis: Movement Leptons/Frame & Turret ROT — Ghidra Research Report

**Date:** 2026-05-28
**Scope:** Confirm that movement (leptons/frame) and turret ROT are both clocked against
`g_CurrentFrameCounter @ 0x00A8ED84` increments ("1 increment = 1 logic step") and state
the authoritative function chain for each. Also confirms the pre-settled facts for ROF,
AnimClass Rate, and CDTimerClass.
**Active in YR:** Yes for all five timing consumers addressed here.
**Confidence:** HIGH for turret FacingClass (existing UNITCLASS_TURRET_TRACKING doc, confirmed
here). HIGH for movement budget chain (MCV trace + DRIVE_PROCESS_DRIVE_TRACK doc, confirmed here).
HIGH for ROF, AnimClass Rate, CDTimer (prior GLOBAL_TIMING_MODEL and COMPLETION docs, re-confirmed).

---

## Target Question

Are weapon ROF, movement leptons/frame, turret ROT, AnimClass Rate, and CDTimerClass durations
all counted in `g_CurrentFrameCounter` increments (1 increment = 1 binary logic step)?

## Non-Goals

Throttle/pacing math (slot 1), speed byte selection (slot 2), render coupling (slot 4),
guard flag internals (slot 5). Full `FootClass::GetCurrentSpeed` internals (out of scope;
the budget *chain* is confirmed; the exact house/ability multiplier path is deferred).

## Evidence Needed for COMPLETE

1. Movement: `DriveLocomotionClass::Process_Drive_Track` budget is per-binary-frame (one call per
   `g_CurrentFrameCounter` increment). Confirmed — existing doc.
2. Turret: `FacingClass::Set` uses `start_frame = g_CurrentFrameCounter`; duration is `abs(delta)/ROT`
   in binary frames. Confirmed — existing doc.
3. ROF: `TechnoClass::Fire_At` snapshots `g_CurrentFrameCounter` to `FireTimer.StartFrame +0x2EC`.
   Confirmed — existing doc.
4. AnimClass Rate: `AnimClass::AI` advances frame when CDTimer expires; timer uses
   `g_CurrentFrameCounter`. Confirmed — existing doc.
5. CDTimerClass: `Start` writes `start_frame = g_CurrentFrameCounter`; `GetTimeRemaining` derives
   elapsed from same counter. Confirmed — existing doc.

## Stop Conditions

All five chains confirmed. Rust surface mismatches documented.

---

## 1. Pre-Settled Facts (Confirmed, Not Re-Investigated)

### 1.1 CDTimerClass
- `CDTimerClass::Start @ 0x0046B640` writes `start_frame = g_CurrentFrameCounter`; `duration` in frames.
- `CDTimerClass::GetTimeRemaining @ 0x00426630` computes `elapsed = g_CurrentFrameCounter - start_frame`; derived-on-read.
- `start_frame == -1` returns raw duration (paused). Boundary: expired at `elapsed >= duration`.
- **Counter: `g_CurrentFrameCounter` directly.**
- Source: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md §3.4`; `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md §Timer Primitives`.

### 1.2 AnimClass Rate
- `AnimType::Rate` stored at `type+0x2B0` = `900 / INI Rate` integer frame delay
  (`900 / INI Rate` because 900 = 60×15, the art animation convention).
- `AnimClass::AI @ 0x00423AC0` advances frame when CDTimer expires; timer reloads from `FrameDelayReload`.
- `AnimClass::LastFrameTime @ this+0x0B4` written from `g_CurrentFrameCounter` on start/reload.
- **Counter: `g_CurrentFrameCounter` via CDTimer.**
- Source: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md §3.6`; `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md §AnimClass lifecycle`.

### 1.3 Weapon ROF
- `TechnoClass::Fire_At @ 0x006FDD50` fires one projectile, calls `GetROF @ 0x006FCFA0`, stores
  returned cooldown into `TechnoClass+0x2F8/+0x2F4 (FireTimer.ROF)`, and snapshots
  `g_CurrentFrameCounter` to `TechnoClass+0x2EC (FireTimer.StartFrame)`.
- The fire-timer is a standard CDTimer-layout field; `CanFire` checks whether elapsed frames >= cooldown.
- **Counter: `g_CurrentFrameCounter` via FireTimer snapshot.**
- Source: `GRIZZLY_ELITE_WEAPON_SWAP_BURST_CADENCE_GHIDRA_REPORT.md §4.2`.
  Verified inline: `g_CurrentFrameCounter → +0x2EC` at `Fire_At @ 0x006FDD50`.

---

## 2. Novel Focus: Movement Leptons/Frame

### 2.1 Call chain

```
Main_Tick → LogicClass::AI → FootClass::Locomotion_AI @ 0x00520F40
         → DriveLocomotionClass::Process @ 0x004B0500 (per active Drive unit)
         → DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20
         → FootClass::GetCurrentSpeed @ 0x004DB1A0 (vtable +0x538)
```

`DriveLocomotionClass::Process @ 0x004B0500` is called **once per active `Main_Tick`**, i.e.
once per `g_CurrentFrameCounter` increment. It calls `Process_Drive_Track` once per tick.
Active in YR: Yes. Source: `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §5`.

### 2.2 Budget computation inside `Process_Drive_Track @ 0x004B0F20`

```
budget = FootClass::GetCurrentSpeed() + DriveLocomotion+0x4C (residual)
```

`GetCurrentSpeed @ 0x004DB1A0` returns an **integer lepton budget for one binary frame**,
derived from `TechnoTypeClass+0x678` (Speed= INI value, stored after `floor(Speed_raw × 256 / 100)`
scaling), current speed fraction at `TechnoClass+0x578`, and house/veteran modifiers.

For `Speed=4`: `floor(4 × 256 / 100)` = 10 leptons-per-binary-frame at full speed fraction.
Active in YR: Yes. Source: `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.3`;
`MCV_DRIVE_10_CELLS_STRAIGHT_FLAT_GRASS_TRACE.md §Speed Detail`
(verified `FootClass::GetCurrentSpeed @ 0x004DB1A0`).

### 2.3 Budget consumption

Each drive-track point costs exactly `7` budget units; remainder stored at `DriveLocomotion+0x4C`
and re-added next tick (carry-forward residual). Interpolated sub-step if residual in `[1..7]`.
**Movement does not read wall-clock ms; budget is an integer scaled once per binary frame.**
Active in YR: Yes. Source: `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.4`.

### 2.4 Summary for movement

**1 `g_CurrentFrameCounter` increment = 1 call to `Process_Drive_Track` = 1 application of
`GetCurrentSpeed` lepton budget.** The per-frame lepton count is a fixed integer at full speed
(e.g. 10 lep/frame for Speed=4). The residual carry means fractional frames accumulate and
deliver an extra point every few frames at fractional budgets. No wall-clock ms, no separate counter.

---

## 3. Novel Focus: Turret ROT

### 3.1 FacingClass layout (24 bytes, confirmed)

| Byte offset | Field | Type |
|---|---|---|
| `+0x00` | Current (destination) | `short` |
| `+0x04` | Prev (rotation start value) | `short` |
| `+0x08` | CDTimer.StartFrame = `g_CurrentFrameCounter` | `int` |
| `+0x10` | CDTimer.Duration = `abs(Current - Prev) / ROT` | `int` |
| `+0x14` | ROT = `rot_byte << 8` (capped at 0x7F → 0x7F00) | `short` |

> **Correction (2026-07-18):** the old generic `TurretFacing` label for `+0x388`
> is stale for movement/pathfinding use. Live disassembly proves that
> `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`, Hover locomotion, and
> `TechnoClass::Unlimbo @ 0x006F6CA0` write object `+0x388`, while
> `UnitClass::Can_Enter_Cell` compares its animated value. Treat `+0x388` as the
> conservative locomotor/body-facing timer and keep it distinct from the
> `+0x3A0` aiming/barrel timer. See
> `FOOTCLASS_0X388_LOCOMOTOR_FACING_GHIDRA_REPORT.md`. This correction does not
> reclassify `+0x370`.

`TechnoClass` holds three distinct FacingClass instances at `+0x370`, `+0x388`,
and `+0x3A0`. `UnitClass::Constructor` sets the `+0x388` locomotor-facing timer
and `+0x3A0` aiming/barrel timer ROT to `Type+0x71C` (INI `ROT=`) via
`FacingClass::SetROT @ 0x004C9680`.
Source: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §1–§2.7`.

### 3.2 Rotation is timer-based, NOT per-tick delta

**There is no per-tick "advance facing by ROT" call.** The animated facing at any binary frame F is:

```
animated = Prev + sign(Current - Prev) * ROT * (F - StartFrame)
```

read via `FacingClass::Current @ 0x004C93D0`. Rotation completes after
`Duration = abs(Current - Prev) / ROT` binary frames.

`FacingClass::Set @ 0x004C9220` sets `StartFrame = g_CurrentFrameCounter` and computes Duration.
`UnitClass::Facing_Update @ 0x00736990` calls `FacingClass::Set` with the new target each tick.
Active in YR: Yes. Source: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §2.1–§2.4`.

### 3.3 ROT = per-frame angular delta in 16-bit facing units

ROT is stored after `<<8`: `ROT=5` → `rot_per_frame = 0x0500 = 1280` in 16-bit space.
Effective angular rate = `ROT * (65536 / 360°)` facing-units-per-degree × 360° / 65536 = ROT
facing-units per binary frame.

At ROT=5 (typical), a full 180° turn = 32768 units → duration = 32768 / 1280 ≈ 25.6 → 25 frames.
(Integer division, remainder snaps at final frame.)
Source: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §2.1, §2.7`.

### 3.4 Summary for turret ROT

**1 `g_CurrentFrameCounter` increment = 1 frame of FacingClass interpolation at ROT units/frame.**
The timer is set at `g_CurrentFrameCounter` and expires after `abs(delta)/ROT` increments.
No wall-clock ms, no separate angular-velocity integration — the facing is a pure linear function
of `(g_CurrentFrameCounter - StartFrame)`.

---

## 4. Clock Taxonomy Summary (All Five Consumers)

| Consumer | Field / function | Counts in | Unit per increment | Evidence |
|---|---|---|---|---|
| CDTimerClass | `start_frame @ +0x00`, `GetTimeRemaining @ 0x426630` | `g_CurrentFrameCounter` | 1 frame | `0x0046B640`, `0x00426630` decompile |
| AnimClass Rate | `LastFrameTime @ +0x0B4`, CDTimer inside `AnimClass::AI @ 0x423AC0` | `g_CurrentFrameCounter` | 1 frame = 1 cadence tick | `0x00423AC0`, `0x00427D00` decompile |
| Weapon ROF | `FireTimer.StartFrame @ TechnoClass+0x2EC`, `Fire_At @ 0x6FDD50` | `g_CurrentFrameCounter` | 1 frame | `0x006FDD50` decompile |
| Movement lep/frame | `Process_Drive_Track @ 0x4B0F20` via `GetCurrentSpeed @ 0x4DB1A0` | once per `Main_Tick` (= 1 frame) | `floor(Speed×256/100)` leptons/frame | `0x004B0F20`, `0x004DB1A0` decompile |
| Turret ROT | `FacingClass::Set @ 0x4C9220`, `StartFrame @ FC+0x08` | `g_CurrentFrameCounter` | ROT 16-bit-facing-units/frame | `0x004C9220`, `0x004C93D0` decompile |

---

## 5. Current Rust Surface vs. gamemd

### 5.1 FacingClass (turret) — MATCHES for barrel

`src/sim/movement/facing_class.rs` implements `FacingClass` struct with `start_frame: Option<u32>`,
`duration_frames`, `rot_per_frame`, using `binary_frame` argument to `set()` and `current()`.
`tick_turret_rotation` in `src/sim/movement/turret.rs` calls `barrel.set(target_facing, binary_frame)`.
**The barrel FacingClass is frame-counter-based — MATCHES gamemd.**

### 5.2 Body rotation (in-place turn) — DRIFT

`src/sim/movement/movement_step.rs:rot_to_facing_delta(rot, tick_ms)` converts ROT + milliseconds
to an 8-bit per-45Hz-tick body delta. This is ms-based, not binary-frame-based. In gamemd, body
facing also uses FacingClass at `TechnoClass+0x370` (ROT=3 default) with `g_CurrentFrameCounter`.
Rust body rotation uses a per-ms integration rather than the CDTimer/linear-interpolation model.
**DRIFT** — body turn speed is 45Hz-tick-rate-dependent, not binary-frame-count-dependent.
The MCV trace (`MCV_DRIVE_10_CELLS_STRAIGHT_FLAT_GRASS_TRACE.md §Turn Detail`) confirms this
fires every match when a player moves an MCV or any vehicle from a non-aligned heading.

### 5.3 Movement budget — PARTIALLY MATCHES

`src/sim/movement/movement_step.rs:drive_track_native_frame_count` correctly gates drive-track
advancement to ~15Hz by counting sub-ticks (`DRIVE_TRACK_NATIVE_FRAME_HZ = 15`). The budget
formula `current_speed_per_second / 15 * native_frames` is equivalent to leptons-per-binary-frame.
**MATCHES** the per-binary-frame budget semantics.

However, the upstream speed input uses `ra2_speed_to_leptons_per_second` (leptons/sec), which is
an intermediate form — functionally equivalent but not the exact `floor(Speed×256/100)` integer
returned by `GetCurrentSpeed`. Full ramp/`Accelerates` ownership mismatch is covered in
`DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §6`.

### 5.4 ROF — MATCHES

ROF uses CDTimer-equivalent fields in Rust (`FireTimer` with `start_frame` + `cooldown_ticks`).
`rof_to_cooldown_ticks` in `src/sim/combat/mod.rs:~2318` maps INI ROF to frame counts.
**MATCHES** binary-frame semantics.

---

## 6. Implementation Handoff

### H-1 Body rotation → binary-frame FacingClass
- **Behavior:** Body facing in gamemd uses `FacingClass @ TechnoClass+0x370` (ROT=3 default),
  `Start=g_CurrentFrameCounter`, `Duration=abs(delta)/3`. Rust uses ms-based `rot_to_facing_delta`.
- **Rust delta:** Replace `rot_to_facing_delta(rot, tick_ms)` body-rotation in `movement_step.rs`
  with a `FacingClass` set call receiving `binary_frame`, mirroring the barrel path.
- **Surface:** `src/sim/movement/movement_step.rs` (in-place rotation block),
  `src/sim/movement/facing_class.rs`.
- **Acceptance:** MCV turn duration matches gamemd (currently ~2× too fast per MCV trace).
- **Test name:** `test_body_rotation_matches_native_frame_duration`
- **Risk:** `rot_to_facing_delta` is also used for body facing during drive-track traversal
  (not just in-place pre-move rotation). Both uses need migration.

### H-2 Confirm `GetCurrentSpeed` integer vs. `ra2_speed_to_leptons_per_second`
- **Behavior:** gamemd `GetCurrentSpeed` returns `floor(Speed×256/100)` × speed_fraction as integer.
  Rust `ra2_speed_to_leptons_per_second` gives leptons/sec, divided back by 15 in the drive budget.
  Net result is equivalent for full-speed straight-line movement, but may diverge under fractional
  speed fractions or house bonuses.
- **Rust delta:** Audit `drive_track_fresh_budget_from_current_speed` for exact integer semantics:
  verify it produces the same integer budget as `floor(GetCurrentSpeed_int × speed_fraction)` when
  ramp is off.
- **Surface:** `src/sim/movement/movement_step.rs:drive_track_fresh_budget_from_current_speed`.
- **Acceptance:** Speed=4 flat-grass unit advances exactly 10 lep/frame (not 9 or 11) at full speed.
- **Test name:** `test_movement_advances_per_native_frame`
- **Risk:** Rounding differs between `to_num::<i32>()` (truncate) and `Math__ftol` (floor-toward-zero
  for positive, ceiling-toward-zero for negative); negative speed fractions are impossible, so
  difference only at sub-1-lepton speeds.

### H-3 Turret barrel FacingClass verified correct — no code change needed
- **Behavior:** `barrel.set(target, binary_frame)` with `FacingClass::current(frame)` interpolation.
  Matches gamemd FacingClass semantics for barrel/turret.
- **Rust delta:** None required for barrel.
- **Surface:** `src/sim/movement/turret.rs`, `src/sim/movement/facing_class.rs`.
- **Acceptance:** Grizzly turret at ROT=5 takes ~25 binary frames for 180° turn, matching gamemd.
- **Test name:** `test_facing_class_rof5_180deg_duration`
- **Risk:** None for barrel itself; separate risk is that body FacingClass (H-1) is not yet ported.

---

## 7. Negative Facts / Do Not Do

1. **Do NOT compute body rotation in milliseconds.** Body rotation in gamemd is a binary-frame
   FacingClass, not ms-integrated. `rot_to_facing_delta(rot, tick_ms)` is incorrect for this
   path. Source: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §1.2, §2.1`.

2. **Do NOT apply ROT as a per-tick angular delta.** gamemd turret/body rotation is a linear
   interpolation from `Prev` to `Current` evaluated at read time; the delta is not accumulated
   per tick. Source: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §2` ("no per-tick
   advance the facing by ROT call").

3. **Do NOT treat `Process_Drive_Track` as being called more than once per binary frame.** The
   caller `DriveLocomotionClass::Process @ 0x004B0500` fires once per active `Main_Tick`. The retry
   call (param_2=1) zeros fresh speed but does not double-count a frame. Source:
   `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.3`.

4. **Do NOT conflate `g_CurrentFrameCounter` with wall-clock ms.** All five timer consumers operate
   on frame counts, not milliseconds. The game frame counter increments once per active `Main_Tick`
   regardless of how many ms that tick consumed. Source:
   `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md §3.1`.

5. **Do NOT snap body-facing to destination instantly.** BodyFacing at `TechnoClass+0x370` has ROT=3
   (FacingClass-constructor default); even tanks with high locomotor turn rates have a rendered body
   that smooths over `abs(delta)/3` frames. Source:
   `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §1.2`.

---

## 8. Remaining Uncertainty

1. **Full `FootClass::GetCurrentSpeed @ 0x004DB1A0` internals:** house bonus, veteran/elite
   multipliers, and the `halve for infantry state` branch are not fully exhausted. The budget
   chain is confirmed; exact per-unit-type modifiers are medium-confidence. Covered as deferred
   in `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §Coverage Ledger`.

2. **BodyFacing ROT default inheritance:** The ROT=3 default for `TechnoClass+0x370` is well
   evidenced for UnitClass, but not re-verified for InfantryClass and AircraftClass in this session.
   Those classes are stated not to override it, but InfantryClass ROT behavior may be handled
   differently via locomotor facing rather than FacingClass interpolation.

---

## 9. Stale / Extend Notices

**`UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §10` (What's missing or wrong):**
States "Our turret rotation is a per-tick step (`max_delta` clamp), which approximates the binary's
behavior but is NOT timer-based." This is **partially stale** as of current code: the barrel
FacingClass in `src/sim/movement/facing_class.rs` IS now timer-based and uses `binary_frame`.
The "no FacingClass-equivalent timer-interpolation struct" bullet should be updated to reflect
that barrel is correct; body rotation remains the open item.

---

## Sources Used

| Document | Sections used |
|---|---|
| `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` | §3.1 tick order, §3.4 CDTimer, §3.5 RateTimer, §3.6 AnimClass |
| `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md` | Clock Taxonomy, Timer Primitives, Mobile objects, Animation Timing |
| `TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md` | Implementation Parity Model |
| `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` | §1–§2.7 (FacingClass layout, rotation algorithm), §5 (Facing_Update), §10 |
| `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md` | §4 (AI ordering), §6 (Rust status) |
| `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md` | §1–§5 (budget/residual chain) |
| `GRIZZLY_ELITE_WEAPON_SWAP_BURST_CADENCE_GHIDRA_REPORT.md` | §4.2 (ROF FireTimer snapshot) |
| `MCV_DRIVE_10_CELLS_STRAIGHT_FLAT_GRASS_TRACE.md` | §Speed Detail, §Turn Detail |
| `src/sim/movement/facing_class.rs` | Current Rust FacingClass implementation |
| `src/sim/movement/turret.rs` | `rot_to_facing_delta`, `tick_turret_rotation` |
| `src/sim/movement/movement_step.rs` | `drive_track_native_frame_count`, body rotation |
