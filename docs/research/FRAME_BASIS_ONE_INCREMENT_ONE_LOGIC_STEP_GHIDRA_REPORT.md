# Frame Basis: One g_CurrentFrameCounter Increment = One Logic Step — Ghidra Research Report

**Date:** 2026-05-28
**Scope:** Confirm that all five major gameplay rate primitives (ROF, movement leptons/frame,
turret ROT, AnimClass Rate, CDTimerClass durations) advance in exact units of one
`g_CurrentFrameCounter @ 0x00A8ED84` increment per `Main_Tick`, with no sub-frame loop or
wall-clock-ms substitution. Prioritises movement as the least-confirmed leg.
**Active in YR:** Yes for all five consumers.
**Confidence:** HIGH for all five (all settled by prior verified reports, reconciled here).

---

## Target Question

Is 1 `g_CurrentFrameCounter` increment = 1 logic step for: ROF, movement leptons/frame,
turret ROT, AnimClass Rate, CDTimerClass?

## Non-Goals

Full `FootClass::GetCurrentSpeed` house/ability modifier internals, throttle/pacing math,
render coupling, full AnimClass loop-count or RandomRate paths.

## Evidence Needed for COMPLETE

Per-family function chain citing the `g_CurrentFrameCounter` consumer, plus the Rust delta.
All five must be confirmed with address-level evidence from existing verified reports.

## Stop Conditions

All five consumer families confirmed and Rust surface delta documented. Report written.

---

## 1. Per-Family Confirmation Table

| Consumer | Timer field / key function | Counts in | Unit per increment | Evidence | Active in YR |
|---|---|---|---|---|---|
| **CDTimerClass** | `start_frame @+0x00`; `CDTimerClass::Start @ 0x0046B640`; `GetTimeRemaining @ 0x00426630` | `g_CurrentFrameCounter` | 1 binary frame | `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md §Timer Primitives` | Yes |
| **AnimClass Rate** | `type+0x2B0 = 900/INI_Rate`; CDTimer inside `AnimClass::AI @ 0x00423AC0`; `AnimClass::Constructor @ 0x00421EA0` stores `g_CurrentFrameCounter` | `g_CurrentFrameCounter` via CDTimer | 1 frame per cadence tick | `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md §Animation Timing`; `AnimTypeClass::ReadINI @ 0x00427D00` | Yes |
| **Weapon ROF** | `FireTimer.StartFrame @TechnoClass+0x2EC` ← `g_CurrentFrameCounter`; `TechnoClass::Fire_At @ 0x006FDD50`; cooldown stored at `+0x2F8` | `g_CurrentFrameCounter` via CDTimer-layout field | 1 frame | `GRIZZLY_ELITE_WEAPON_SWAP_BURST_CADENCE_GHIDRA_REPORT.md §4.2`; `BURST_WEAPON_FIRING_GHIDRA_REPORT.md §3.1` | Yes |
| **Movement lep/frame** | `DriveLocomotionClass::Process @ 0x004B0500` → `Process_Drive_Track @ 0x004B0F20` → `FootClass::GetCurrentSpeed @ 0x004DB1A0` (vtable+0x538); budget integer per call; called once per `Main_Tick` | once per `g_CurrentFrameCounter` increment (= once per Main_Tick) | `floor(Speed×256/100)` leptons at full fraction | `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.3–§5`; `FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md §2` | Yes |
| **Turret ROT** | `FacingClass::Set @ 0x004C9220` writes `StartFrame=g_CurrentFrameCounter`; duration=`abs(delta)/ROT` binary frames; `Current @ 0x004C93D0` interpolates | `g_CurrentFrameCounter` via RateTimer/FacingClass | ROT = rot_byte<<8 facing-units per frame | `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §2.1–§2.7`; `FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md §3` | Yes |

---

## 2. Movement: Highest-Priority Leg (Expanded)

The call chain is settled by `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`:

```
Main_Tick → LogicClass::AI → FootClass::Locomotion_AI @ 0x00520F40
          → DriveLocomotionClass::Process @ 0x004B0500  (once per active Main_Tick)
          → DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20
          → FootClass::GetCurrentSpeed @ 0x004DB1A0 (vtable+0x538)
```

- `Process @ 0x004B0500` calls `Process_Drive_Track` once per live `Main_Tick`. No inner
  frame loop. Retry param-2 path zeros fresh speed contribution, does not add a second frame.
  Source: `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.3` (asm
  `0x004B127A..0x004B128D`), `§5`.

- `GetCurrentSpeed @ 0x004DB1A0` returns an **integer lepton budget** — units are
  leptons-per-binary-frame. For Speed=4: `floor(4×256/100)=10` leptons/frame at full fraction.
  Source: same doc `§3.3`; `MCV_DRIVE_10_CELLS_STRAIGHT_FLAT_GRASS_TRACE.md §Speed Detail`.
  INI `Speed=` is stored at `TechnoTypeClass+0x678` after `floor(raw×256/100)` clamped to 255.
  Source: `DRIVE_RULES_FIELDS_SPEED_INPUTS_GHIDRA_REPORT.md §Stale Docs`.

- Budget consumption: each drive-track point costs **exactly 7** units; leftover stored at
  `DriveLocomotion+0x4C` and re-added next call. Asm: `SUB...,0x7` at `0x004B159D`;
  `CMP...,0x7; JG` loop exit at `0x004B1F50..0x004B1F56`.
  Source: `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.4`.

**Conclusion for movement:** `Speed=` is consumed as leptons-per-binary-frame. There is no
sub-frame integration loop and no wall-clock-ms path in the Drive locomotor position step.
The walk-SHP gate (`g_CurrentFrameCounter % WalkRate`) in `FootClass::AI @ 0x004DA530` is
a **body-animation** cadence gate, not a position-step gate — position steps every frame;
the walk SHP frame advances at WalkRate modulo.

---

## 3. Rust Surface vs. gamemd — What Is Matched and What Drifts

| Rust surface | gamemd behavior | Status |
|---|---|---|
| `src/sim/movement/facing_class.rs` + `turret.rs` barrel path | `FacingClass::Set @ 0x4C9220`, binary-frame timer | **MATCHES** |
| `src/sim/movement/movement_step.rs:drive_track_native_frame_count` | budget once per binary frame | **MATCHES** (15 Hz gate matches 15-fps native) |
| `src/sim/movement/movement_step.rs:rot_to_facing_delta(rot, tick_ms)` | BodyFacing `TechnoClass+0x370` uses FacingClass, `g_CurrentFrameCounter` | **DRIFT** — ms-based; fires every match when any vehicle turns |
| `src/sim/combat/mod.rs` ROF CDTimer fields | `TechnoClass+0x2EC/+0x2F8`, `g_CurrentFrameCounter` | **MATCHES** |
| `src/util/fixed_math.rs:ra2_speed_to_leptons_per_second` (×15 / 1000) | `GetCurrentSpeed` integer `floor(Speed×256/100)` | **FUNCTIONALLY EQUIVALENT** at full speed; diverges only under fractional speed fractions or integer-rounding edge; not a clean 1:1 match |

---

## 4. Implementation Handoff

### H-1 Body rotation DRIFT → FacingClass migration
- **Verified behavior → Rust delta:** BodyFacing at `TechnoClass+0x370` uses `FacingClass::Set @
  0x4C9220` keyed to `g_CurrentFrameCounter`. Rust `rot_to_facing_delta(rot, tick_ms)` is
  ms-integrated → replace with `FacingClass::set(binary_frame)` in `movement_step.rs` body path.
- **Affected surface:** `src/sim/movement/movement_step.rs` (in-place and drive-track body rotation),
  `src/sim/movement/facing_class.rs`.
- **Acceptance scenario:** MCV 90° body turn takes same frame count as gamemd (currently ~2× too
  fast per `MCV_DRIVE_10_CELLS_STRAIGHT_FLAT_GRASS_TRACE.md §Turn Detail`).
- **Proposed test:** `test_body_rotation_matches_native_frame_duration`
- **Risk:** `rot_to_facing_delta` is used in both in-place pre-move and mid-drive-track body turn
  paths — both need migration.

### H-2 Speed budget integer precision audit
- **Verified behavior → Rust delta:** `GetCurrentSpeed` returns `floor(Speed×256/100)×fraction`
  as integer; Rust converts Speed via `ra2_speed_to_leptons_per_second` (leptons/sec / 15).
  Functionally equivalent at full fraction, but integer rounding path differs (Rust `to_num::<i32>`
  truncates vs. gamemd `Math__ftol` floor-toward-zero). Negative fractions are impossible so only
  boundary matters at sub-1-lepton speeds.
- **Affected surface:** `src/sim/movement/movement_step.rs:drive_track_fresh_budget_from_current_speed`.
- **Acceptance scenario:** Speed=4 unit advances exactly 10 lep/frame (not 9 or 11) at full speed fraction.
- **Proposed test:** `test_movement_advances_per_native_frame`
- **Risk:** LOW — functionally equivalent for Speed≥1 at full fraction; only ramp mismatch is the
  bigger risk (covered in `DRIVE_ACCELERATES_TRUE_FALSE_SPEED_RAMP_GHIDRA_REPORT.md`).

### H-3 ROF and AnimClass Rate — no action needed
- Both already use frame-count CDTimer equivalents in Rust. No code change required.
  Source: `src/sim/combat/mod.rs:rof_to_cooldown_ticks`; `src/rules/art_data.rs` Rate field.

---

## 5. Negative Facts / Do Not Do

1. **Do NOT apply body rotation in ms.** BodyFacing uses FacingClass keyed to `g_CurrentFrameCounter`,
   not wall-clock. Source: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §1.2`.

2. **Do NOT call `Process_Drive_Track` more than once per binary frame** for normal units. The retry
   call (param_2=1) zeros fresh speed — it does not represent an additional frame.
   Source: `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §3.3` (`0x004B127A..0x004B128D`).

3. **Do NOT conflate the walk-SHP cadence gate** (`FootClass::AI @ 0x004DA530`: `g_CurrentFrameCounter
   % WalkRate`) with the position-step gate. Position advances every binary frame; SHP animation frame
   advances every WalkRate frames.

4. **Do NOT treat the drive-track point loop as a sub-frame multi-step.** The loop exits at `budget>7`;
   at Speed=4 (10 lep/frame) typically 1 point per frame. The "sub-loop" executes multiple track-point
   steps only when budget accumulates (e.g., very fast unit or ramp spike). It is still 1 call per frame.
   Source: asm `CMP EAX,0x7; JG` at `0x004B1F50..0x004B1F56`.

5. **Do NOT invent a sub-frame position integrator for Drive.** gamemd's Drive locomotor has no
   wall-clock delta integration path; it is a discrete per-frame integer-budget machine.
   Source: `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §1 Overview`.

---

## 6. Remaining Uncertainty

1. **`FootClass::GetCurrentSpeed @ 0x004DB1A0` house/veteran multiplier internals:** The budget
   chain is confirmed; exact modifier composition is deferred (`DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md §Coverage Ledger OQ-13`).

2. **BodyFacing ROT default for InfantryClass/AircraftClass:** UnitClass verified (ROT byte from
   `TechnoTypeClass+0x71C`). Infantry and aircraft body-facing mechanism not re-verified here; may
   differ (infantry locomotor handles turn in drive-facing path, not FacingClass directly).

---

## 7. Stale Doc Notices

`FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md` (written same day by slot-3) covers the same five
consumers. This doc is the canonical consolidated version for the "one increment = one step"
question. Both docs are consistent; prefer citing this one for the full per-family table.

`UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §10` note "Our turret rotation is a
per-tick step" is **partially stale**: barrel FacingClass in `src/sim/movement/facing_class.rs`
IS frame-counter-based. Body rotation remains the open DRIFT item.

---

## Sources

| Document | Sections used |
|---|---|
| `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md` | Clock Taxonomy; Timer Primitives; Mobile objects; Animation Timing |
| `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md` | §1 Overview; §3.3–§3.4 budget/residual; §5 Integration; §6 Rust status |
| `FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md` | §2 Movement; §3 Turret; §4 Taxonomy; §5–§7 Rust/Handoff |
| `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` | §3.1 Fire_At; FireTimer layout |
| `GRIZZLY_ELITE_WEAPON_SWAP_BURST_CADENCE_GHIDRA_REPORT.md` | §4.2 FireTimer.StartFrame snapshot |
| `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` | §2.1–§2.7 FacingClass algorithm |
| `DRIVE_RULES_FIELDS_SPEED_INPUTS_GHIDRA_REPORT.md` | §Stale Docs (TechnoTypeClass+0x678 Speed= storage) |
| `src/util/fixed_math.rs` | `ra2_speed_to_leptons_per_second`; `SIM_TICK_HZ=45` |
| `src/sim/movement/movement_step.rs` | `drive_track_native_frame_count`; `rot_to_facing_delta` |
