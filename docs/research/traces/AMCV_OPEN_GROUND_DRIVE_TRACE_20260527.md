# AMCV Open-Ground Drive Locomotion Trace - 2026-05-27

Scenario: `[AMCV]` moves from cell `(40,40)` to `(45,40)` on flat clear land. Scope is destination setup, DriveLocomotion speed budget, facing, track stepping, arrival stop, and final cell/state only.

## Verdict Tally

PASS: 3 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Pipeline

`Command::Move -> resolve_move_info -> issue_move_command_with_layered -> MovementTarget/path/facing setup -> tick_movement_with_grids -> rotation/speed/step/crossing -> finalize_finished_entities -> rendered unit at final cell`

gamemd reference path: `FootClass::Set_Destination_Internal -> DriveLocomotionClass::Head_To_Coord -> DriveLocomotionClass::Process -> Process_Movement/Process_Drive_Track -> arrival Set_Destination(NULL)`.

## Scenario Data

- AMCV stock YR data: `ini/rulesmd.ini:6969` `[AMCV]`, `DeploysInto=GACNST` at `6977`, `Speed=4` at `6980`, `ROT=5` at `6986`, `Crusher=yes` at `6988`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` at `6998`, `MovementZone=Normal` at `7000`.
- Flat cell centers in lepton coordinates use `cell * 256 + 128`; start center `(40,40)` is `(10368,10368)`, goal center `(45,40)` is `(11648,10368)`.
- Direction delta is `(dx=+1, dy=0)` for every cell step, so target facing byte is East `64`.

## Stage Trace

| Stage | gamemd evidence | Current Rust output | Verdict |
|---|---|---|---|
| 1. Rules and active locomotor | Stock AMCV uses `Speed=4`, `ROT=5`, DriveLocomotion CLSID, `MovementZone=Normal`; this is standard YR AMCV data, not TS legacy. | Rules source has the same rows in `ini/rulesmd.ini:6969-7000`; `LocomotorState::from_object_type` maps Drive to ground layer and speed multiplier `1`. | PASS |
| 2. Requested destination | Active main lifecycle is `Set_Destination -> NavCom = target -> Locomotor.Head_To_Coord(cell center)`; `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md:14`, `145-211`, `249-260`. For `(45,40)`, Head_To_Coord receives `(11648,10368,0)` on flat ground. | Rust creates a `MovementTarget` with `final_goal=Some((45,40))`, path cells, and no NavCom/CellClass object or separate DriveLocomotion destination fields; see `src/sim/movement/movement_commands.rs:479-489`. | FAIL |
| 3. Straight path cells | Existing pathfinding/facing reports verify active adjacent direction order `N,NE,E,SE,S,SW,W,NW` and East facing `64`; straight paths are not smoothed or optimized away. | On flat clear land, A*/smoothing yields `[(40,40),(41,40),(42,40),(43,40),(44,40),(45,40)]`; `move_dir=(256,0)`, `move_dir_len=256`, `next_index=1`. | PASS |
| 4. Speed budget | gamemd AMCV `Speed=4` gives `leptons_per_tick = floor(4*256/100)=10`, i.e. `150 leptons/sec` at 15 Hz. DriveLocomotion then adjusts speed fraction in `Process_Drive_Track`; see `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md:248-358`. | `resolve_move_info` has `// DEBUG: 3x speed boost for MCVs`; because `DeploysInto` is set, AMCV uses `Speed=12`: `floor(12*256/100)=30`, `450 leptons/sec`; see `src/sim/world/world_commands.rs:73-75`. | FAIL |
| 5. Speed ramping start | gamemd DriveLocomotion has local `current_speed` fraction initialized separately and updated continuously by `Process_Drive_Track`; budget is `GetCurrentSpeed + residual` except retry ticks. | Rust sets `MovementTarget.current_speed = speed` immediately and only uses ramping if `accel_factor/decel_factor` were stamped; `AMCV` has no explicit keys, so it stays full speed. See `src/sim/movement/movement_commands.rs:479-484` and `src/sim/movement/movement_tick.rs:624-673`. | FAIL |
| 6. Target facing byte | Active gamemd direction table maps East step `(1,0)` to facing byte `64`; see `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md:41-64`, active in YR. | Rust `facing_from_delta(1,0)` returns `64`; movement command stores `facing_target=Some(64)` for non-infantry with ROT > 0; `src/sim/movement/movement_commands.rs:457-504`. | PASS |
| 7. Per-tick body-facing timeline | gamemd uses 16-bit FacingClass/ROT helpers and DriveTrack point facings; active `Process_Drive_Track` converts track point facing bytes with `byte << 8`; see `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md:81-88`, `158-166`. | Rust uses 8-bit body facing and `rot_to_facing_delta(ROT=5, tick_ms)` with integer `div_ceil`; current app tick is 22 ms (`SIM_TICK_HZ=45`), so delta is `2` facing bytes/tick. Exact gamemd frame-by-frame body facing was not runtime-traced for the initial facing. | UNCHECKED |
| 8. Drive-track stepping | gamemd active DriveLocomotion consumes path queue directions through `Process_Movement` and `Process_Drive_Track @ 0x004B0F20`; active tick branch documented in `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md:362-390`. | Rust straight east movement uses lepton vector advancement and boundary crossings; DriveTrack only runs when a `drive_track_state` exists. For a straight path with no direction change after setup, no DriveTrack curve is selected; see `src/sim/movement/movement_step.rs:267-357` and `src/sim/movement/movement_step.rs:411-535`. | FAIL |
| 9. Arrival stop | gamemd DriveLocomotion detects current cell equals NavCom cell and calls `Set_Destination(NULL,1)` for an empty queue, which clears NavCom through the null-target path; `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md:273-333`, active in YR. | Rust pushes the entity into `finished_entities`, then `finalize_finished_entities` clears `movement_target`, clears `drive_track`, snaps subcell, and sets phase Idle; see `src/sim/movement/movement_tick.rs:985-996` and `1110-1135`. | FAIL |
| 10. Final cell/state | gamemd final cell should be `(45,40)` with NavCom cleared after arrival; exact remaining DriveLocomotion residual/current-speed bytes for this scenario were not measured. | Rust final cell should be `(45,40)` if uninterrupted, with `movement_target=None`, `drive_track=None`, and subcell center `(128,128)`. Exact tick of arrival differs because speed budget is 3x. | UNCHECKED |

## Failures

1. AMCV moves with a development-only 3x speed multiplier.
   - Player-visible difference: the MCV crosses open ground about three times too fast in wall-clock time.
   - Rust: `src/sim/world/world_commands.rs:73-75`.
   - gamemd: stock `[AMCV] Speed=4` at `ini/rulesmd.ini:6980`; DriveLocomotion speed budget report `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md:347-358`.

2. Destination state is not a NavCom + Head_To_Coord lifecycle.
   - Player-visible difference: arrival signalling, action-line endpoint lifetime, queued order teardown, and stop semantics can diverge even if the cell goal matches.
   - Rust: `src/sim/movement/movement_commands.rs:479-489`.
   - gamemd: `FootClass::Set_Destination_Internal @ 0x004D94B0` writes NavCom and calls locomotor Head_To_Coord; active YR per `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md:14`, `145-211`.

3. Rust starts AMCV movement at full speed instead of DriveLocomotion's current-speed/residual budget model.
   - Player-visible difference: acceleration/braking cadence and arrival timing can differ even after the 3x multiplier is removed.
   - Rust: `src/sim/movement/movement_commands.rs:479-484`, `src/sim/movement/movement_tick.rs:624-673`.
   - gamemd: `Process_Drive_Track @ 0x004B0F20` speed fraction and residual budget; `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md:248-358`.

4. Straight open-ground stepping bypasses gamemd DriveTrack consumption.
   - Player-visible difference: subcell positions, facing cadence, and per-frame movement composition are not proven pixel-identical and can visibly drift during motion.
   - Rust: `src/sim/movement/movement_step.rs:267-357`, `src/sim/movement/movement_step.rs:411-535`.
   - gamemd: active `DriveLocomotionClass::Process -> Process_Movement -> Process_Drive_Track`; `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md:362-390`.

5. Arrival stop is not DriveLocomotion's self-issued `Set_Destination(NULL,1)` path.
   - Player-visible difference: stop timing, queue handoff, and "arrived" side effects can diverge at the destination.
   - Rust: `src/sim/movement/movement_tick.rs:985-996`, `src/sim/movement/movement_tick.rs:1110-1135`.
   - gamemd: active arrival flow in `DriveLocomotionClass::Process @ 0x004B0500`; `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md:273-333`.

## Not Implemented

None marked as NOT-IMPLEMENTED for this scoped scenario. The relevant Rust systems exist, but several are mechanism-drift implementations rather than absent features.

## Timing

- Command application and movement occur in the same Rust `advance_tick` after command dispatch; `world/mod.rs:1170-1244`.
- Current app tick is `1000 / 45 = 22 ms` (`src/util/fixed_math.rs:51`, `src/app_types.rs:24-27`).
- With the current speed multiplier, Rust AMCV budget is `450 * 0.022 = 9.9 leptons/Rust tick`, while gamemd `Speed=4` full-speed budget is `10 leptons/game frame` at 15 Hz. The per-step-looking number is similar, but Rust ticks about 3 times as often, so wall-clock travel is about 3x faster.
- Exact body-facing sequence and final residual bytes remain UNCHECKED because no runtime gamemd frame trace was captured for the unspecified starting facing.

## Adjacent Findings

- AMCV deploy behavior and ConYard placement are adjacent and intentionally not traced here.
- Obstacle, bridge/ramp, and crush-on-path AMCV movement are adjacent trace-swarm slots, not part of this flat clear land report.
- The stale comment in `src/util/fixed_math.rs` that labels `(1,0)` as NE is adjacent only; current code and tests produce East `64`.

## Sources

- `ini/rulesmd.ini:6969-7000`
- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`
- `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- `docs/research/ADDRESS_MAP.md`
- `src/sim/world/world_commands.rs`
- `src/sim/movement/movement_commands.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_step.rs`
- `src/util/fixed_math.rs`
- `src/util/lepton.rs`
