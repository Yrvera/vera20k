# AMCV Turning Diagonal Drive Trace - 2026-05-27

## Scope

Concrete scenario only: AMCV moves from cell `(40,40)` to `(44,44)` on flat clear land.

Compared stages: `Set_Destination`, path/track direction, DriveLocomotion turn/facing cadence, movement step positions, and arrival stop.

Starting-facing assumption: the only concrete Rust AMCV spawn facing found for this scenario class is the stock skirmish starting MCV facing `64` (east) in `src/app_skirmish.rs:517`. The gamemd stock skirmish-start MCV facing was not independently traced in this run, so that stage is `UNCHECKED`. All turn-cadence findings below use initial facing `64 -> 96` because the scenario did not specify a different initial facing.

## Sources Used

- `ini/rulesmd.ini:6969-7009`: `[AMCV]` has `Speed=4`, `ROT=5`, Drive locomotor `{4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`.
- `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: active YR A* path pipeline and direction table, including `SE=3`.
- `docs/research/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`: active YR path smoothing/optimization after A*.
- `docs/research/pathfinding/fn-path_smooth_corners.md`: `Path_smooth_corners @ 0x42B210` is active in standard YR after successful A*.
- `docs/research/pathfinding/fn-path_optimize_straight_segments.md`: `Path_optimize_straight_segments @ 0x42B7F0` is active in standard YR after corner smoothing.
- `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`: direction index and facing-byte mapping; `SE=3`, facing byte `96`, 16-bit facing target `0x6000`.
- `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`: active DriveLocomotion track point budget, residual, and facing-update behavior.
- Read-only Ghidra decompilation in this run:
  - `TechnoClass__Set_Destination @ 0x00741970`
  - `DriveLocomotionClass__Process @ 0x004B0500`
  - `DriveLocomotionClass__Process_Drive_Track @ 0x004B0F20`
  - `DriveLocomotionClass__Process_Movement @ 0x004B2630`

All gamemd functions listed above are active standard YR paths for normal Drive-locomotor objects; no TS-fog or dormant TS-only gate was used for this trace.

## Computed Scenario Values

- Scenario delta is `(dx,dy)=(4,4)`.
- Direction per verified YR/Rust direction order is four `SE` steps: `[3,3,3,3]`.
- Facing byte for `SE` is `96`; 16-bit facing target is `96 << 8 = 0x6000`.
- Rust stock skirmish starting AMCV facing is `64`, so the initial turn is `64 -> 96`.
- Rust `SIM_TICK_HZ=45`, `SIM_TICK_MS=22`; AMCV `ROT=5`.
- Rust `rot_to_facing_delta(5,22)` computes `ceil(5*256*15*22/(360*1000)) = 1` facing byte per simulation tick.
- Rust therefore rotates in place for ticks 1 through 31 (`65..95`), then snaps to `96` and first advances movement on tick 32.
- Rust AMCV Speed=4 converts to `150` leptons/second through `ra2_speed_to_leptons_per_second`.
- Rust diagonal vector for one `SE` cell is `(256,256)`, length approximately `362.038` leptons.
- Rust first movement tick after rotation advances by approximately `3.299` leptons along the diagonal, changing subcell center from `(128,128)` to about `(130.33,130.33)`.

## Stage Verdicts

| Stage | Verdict | Evidence |
|---|---:|---|
| AMCV INI movement data | PASS | Both sides source AMCV Drive locomotor, `Speed=4`, `ROT=5` from `rulesmd.ini`; Rust uses these fields through movement command and turn-speed code. |
| Direction/facing mapping for target delta | PASS | gamemd active direction table maps `(1,1)` to `SE=3` and facing byte `96`; Rust `facing_from_delta(1,1)` also yields `96`. |
| Flat-open path direction array | PASS | For `(40,40)->(44,44)` on flat clear land, verified A* direction order and Rust path tests/math compute `[(40,40),(41,41),(42,42),(43,43),(44,44)]`, directions `[3,3,3,3]`; smoothing has no corner/drift change for an all-SE path. |
| Initial skirmish AMCV facing | UNCHECKED | Rust stock skirmish uses `64`; gamemd stock skirmish-start facing was not traced in binary/runtime in this run. |
| `Set_Destination` to locomotion handoff | FAIL | Rust `issue_move_command` stores `facing_target=Some(96)` and defers movement through generic vehicle rotation. gamemd active `TechnoClass__Set_Destination` reaches Foot/DriveLocomotion destination machinery, and `DriveLocomotionClass__Process_Movement @ 0x004B2630` drives the next path direction through RateTimer/DriveLocomotion state. No literal-equivalence proof exists; mechanisms diverge before the first movement step. |
| Initial turn/facing cadence | FAIL | Rust rotates one facing byte per 22 ms tick and delays first movement until tick 32. gamemd `Process_Movement @ 0x004B2630` targets `direction << 13` (`3 << 13 = 0x6000`) through the active facing RateTimer/DriveLocomotion turn path, with AMCV `ROT=5` represented as 16-bit facing-rate state. Rust's 8-bit, 45 Hz in-place cadence is not numerically equal to gamemd's active DriveLocomotion RateTimer cadence. |
| First movement-step positions | FAIL | Rust does not use a DriveTrack for the first diagonal leg after pre-rotation; it advances a straight-line fixed-point vector from `(128,128)` to about `(130.33,130.33)` on the first moving tick. gamemd `Process_Movement @ 0x004B2630` selects/advances DriveLocomotion path state and `Process_Drive_Track @ 0x004B0F20` consumes drive-track points with 7-unit point cost and residual interpolation. Exact gamemd first-point coordinates were not dumped, but the active mechanisms are not equivalent. |
| Drive-track facing update cadence | FAIL | Rust holds body facing at `96` during the initial straight diagonal advance because no initial DriveTrack is active. gamemd `Process_Drive_Track @ 0x004B0F20` updates facing from each consumed track point by shifting the point heading byte left 8 and calling `FacingClass__UpdateFacing`, after the track point's coord/cell update. |
| Exact arrival stop tick and residual | UNCHECKED | Rust arrival depends on the preceding 32-tick pre-rotation and straight-line subcell movement. gamemd arrival depends on RateTimer state, drive-track point budget, residual, and path queue depletion. Exact final tick/residual equality was not computed for both sides. |
| Exact retail drive-track point list for this AMCV leg | UNCHECKED | The active track budget and heading-update mechanism were verified, but the exact selected retail track point coordinates for this scenario were not dumped in this run. |

## Player-Visible Findings

1. `FAIL - Initial turn/facing cadence`: AMCV waits and turns in Rust for 32 45 Hz ticks before moving; gamemd uses active DriveLocomotion RateTimer target `0x6000` for SE with ROT-derived 16-bit cadence. The first visible movement starts at a different time.
2. `FAIL - First movement-step positions`: Rust's first moving frame is a straight-line lepton interpolation from the cell center; gamemd advances through DriveLocomotion track points and residual interpolation. The AMCV nose/body path can diverge immediately after the turn.
3. `FAIL - Drive-track facing update cadence`: Rust keeps facing fixed at `96` once pre-rotation completes; gamemd applies per-track-point facing updates during `Process_Drive_Track`. The body-facing animation cadence can differ even on an all-diagonal route.
4. `FAIL - Set_Destination handoff`: Rust converts the command into generic `MovementTarget` plus `facing_target`; gamemd uses the active Techno/Foot/DriveLocomotion handoff. This is upstream of all later movement timing.

## Adjacent Findings

- Rust has a DriveTrack implementation and track table, but this scenario's initial diagonal leg bypasses it because `issue_move_command` pre-rotates the vehicle to the first path-facing target before movement.
- The exact gamemd stock skirmish starting MCV facing should be traced separately; this report used Rust's concrete stock skirmish facing because the requested scenario did not specify initial facing.
- AMCV acceleration/ramp behavior was not expanded in this trace. Only the concrete turn/diagonal DriveLocomotion scenario was evaluated.

## Verdict Tally

PASS: 3 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Status

PARTIAL: exact gamemd runtime drive-track point coordinates and exact arrival tick/residual were not captured, but active path direction, command handoff, turn-cadence mechanism, and Rust mismatches were traced for the requested scenario.
