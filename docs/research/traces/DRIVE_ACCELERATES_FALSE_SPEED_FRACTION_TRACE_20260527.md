# Drive Accelerates=false Speed Fraction Authority Trace - 2026-05-27

## Scenario

Concrete trace only: a stock `[MTNK]` / Grizzly-style DriveLocomotion vehicle moves from flat `[Clear]` land into a flat slower terrain cell represented by `[Tiberium]` speed rules. The only mechanic traced here is `Accelerates=false` speed-fraction authority on the first tick affected by the slower target cell.

Adjacent Drive tick-order, queue arrival, chain lookahead, and true-ramp behavior are out of scope.

## Evidence Summary

- Stock `[MTNK]` uses `Speed=7`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`, and `Accelerates=false` in `ini/rulesmd.ini:6603`, `:6618`, `:6636`, `:6638`, `:6643`.
- `[Clear]` has `Track=100%`; `[Tiberium]` has `Track=70%` in `ini/rulesmd.ini:30191-30199` and `:30266-30275`.
- Read-only Ghidra confirms active standard-YR Drive code, not TS legacy:
  - `DriveLocomotionClass::Process @ 0x004B0500` calls the live Drive movement/track path.
  - `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` reads `TechnoType+0xDBD` and, when false, calls owner vtable `+0x544` with `DriveLocomotion+0x50`.
  - `TechnoClass::SetSpeedFraction @ 0x004D3710` clamps to `[0.0, 1.0]` and writes owner current speed fraction at `+0x578`.
  - `FootClass::GetCurrentSpeed @ 0x004DB1A0` consumes the owner current speed fraction before returning the integer movement budget.
- Existing reports agree: `DRIVE_ACCELERATES_TRUE_FALSE_SPEED_RAMP_GHIDRA_REPORT.md:48-58`, `:64-72`, `:123-138`; `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md:76-95`; `DRIVE_RULES_FIELDS_SPEED_INPUTS_GHIDRA_REPORT.md:47-75`.

## Pipeline

Move order/path target -> Drive target speed fraction from target terrain -> `Accelerates=false` current-fraction snap -> `GetCurrentSpeed` integer budget -> DriveTrack point budget consumption -> visible subcell movement cadence.

## Stage Verdicts

| Stage | gamemd value/order | Current Rust value/order | Verdict |
|---|---|---|---|
| 1. Stock data and slower target cell | `[MTNK] Speed=7`, DriveLocomotion CLSID, `Accelerates=false`; flat `[Tiberium] Track=70%` gives target fraction `0.7`. | Rules source contains the same values; Rust speed-cost path represents `Track=70` as `SimFixed(70/100)`. | PASS |
| 2. `Accelerates=false` current fraction authority | `Process_Drive_Track` reads `+0xDBD == 0`, calls `SetSpeedFraction(Drive+0x50)`, so target `0.7` becomes owner current fraction before budget. | `movement_tick.rs:1008-1020` calls `update_drive_speed_fraction`; `drive_locomotion.rs:54-57` sets `target_speed_fraction=0.7` and `current_speed_fraction=0.7` when `drive_accelerates=false`. | PASS |
| 3. Raw speed preservation | gamemd leaves raw `Speed=7` as the base speed; `Accelerates=false` only changes owner current fraction. Parsed type speed is `floor(7*256/100)=17` leptons per 15 Hz game frame. | `MovementTarget.speed` remains raw top speed; `movement_tests.rs:1633-1645` asserts raw speed is not mutated and current speed is raw times current fraction. | PASS |
| 4. Same affected tick authority | gamemd calls `SetSpeedFraction(0.7)` before `GetCurrentSpeed` in the same `Process_Drive_Track` call, so fresh budget uses the new current fraction. | Rust sets `target.current_speed = target.speed * drive.current_speed_fraction` before `advance_lepton_position`, and `movement_step.rs:412-418` consumes that value into Drive residual budget. | PASS |
| 5. First affected tick integer budget | gamemd fresh budget for this no-bonus, no-veteran, no-half-speed case is `trunc(17 * 0.7) = 11`, then plus residual. | Current app tick rate is `SIM_TICK_HZ=45` (`fixed_math.rs:51`) and `SIM_TICK_MS=1000/45=22`; Rust computes `floor((17*15 leptons/sec * 0.7) * 22/1000) = floor(3.927) = 3` fresh budget for one Rust tick. | FAIL |
| 6. DriveTrack point consumption on first affected tick | With budget `11`, gamemd's strict `budget > 7` loop consumes one DriveTrack point and stores residual `4`. | With budget `3`, Rust consumes zero points and stores residual `3`; later 45 Hz ticks may accumulate, but the first affected tick is not numerically equal. | FAIL |

## Findings

### FAIL 1 - Same current fraction, different first-tick movement budget

Player-visible difference: the vehicle's slowdown fraction is applied, but the first affected DriveTrack budget is split across Rust's 45 Hz movement ticks instead of gamemd's 15 Hz game-frame budget. In the concrete `Speed=7`, target `0.7` case, gamemd spends `11` fresh budget immediately, while Rust spends `3` on the first affected tick.

Rust evidence: `src/util/fixed_math.rs:51`, `src/app_types.rs:27`, `src/sim/movement/movement_tick.rs:1071-1075`, `src/sim/movement/movement_step.rs:412-418`.

gamemd evidence: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` calls `SetSpeedFraction` before `GetCurrentSpeed`, then uses `(retry ? 0 : speed) + residual`; `FootClass::GetCurrentSpeed @ 0x004DB1A0` consumes `+0x578`; `Speed=7` parser stores `floor(7*256/100)=17`.

### FAIL 2 - First affected tick DriveTrack point cadence differs

Player-visible difference: gamemd consumes one 7-budget track point immediately (`11 > 7`, residual `4`), while Rust consumes no point on the first 22 ms tick (`3 <= 7`, residual `3`). This can shift subcell motion and facing/track cadence even though the current speed fraction itself is now authoritative.

Rust evidence: `src/sim/movement/drive_track.rs:3741-3748`, `src/sim/movement/movement_step.rs:412-418`.

gamemd evidence: `Process_Drive_Track @ 0x004B0F20` strict point loop `budget > 7`, subtract `7`, store residual at Drive `+0x4C`; confirmed in `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md:87-95`.

## Adjacent Findings

- The previous no-active-track trace already found command-dispatch-vs-Drive-process ownership drift. That can compound this budget cadence difference, but it is not re-traced here.
- Full `FootClass::GetCurrentSpeed` modifiers such as house bonus, `+0x580`, veteran speed, and half-speed flag are not traced beyond confirming they are inactive/neutral for this concrete no-bonus stock case.
- `Process_Movement` target fraction producer is only traced for flat `Track=70%`; slope, health, zero-speed promotion, and crowd/occupancy modifiers are adjacent.

## Verdict Tally

PASS: 4 | FAIL: 2 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

## Sources

- Read-only Ghidra decompile: `DriveLocomotionClass::Process @ 0x004B0500`.
- Read-only Ghidra decompile: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`.
- Read-only Ghidra decompile: `TechnoClass::SetSpeedFraction @ 0x004D3710`.
- Read-only Ghidra decompile: `FootClass::GetCurrentSpeed @ 0x004DB1A0`.
- `docs/research/DRIVE_ACCELERATES_TRUE_FALSE_SPEED_RAMP_GHIDRA_REPORT.md`.
- `docs/research/DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`.
- `docs/research/DRIVE_RULES_FIELDS_SPEED_INPUTS_GHIDRA_REPORT.md`.
- `ini/rulesmd.ini`.
- `src/sim/movement/drive_locomotion.rs`.
- `src/sim/movement/movement_tick.rs`.
- `src/sim/movement/movement_step.rs`.
- `src/sim/movement/drive_track.rs`.
- `src/util/fixed_math.rs`.
