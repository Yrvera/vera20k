# DriveLocomotion Active Track Same-Tick Retry Trace - 2026-05-27

**Status:** COMPLETE.

**Scope:** One concrete mechanic only: a DriveLocomotion vehicle already has an active DriveTrack, that track finishes on this tick, a next turn/track is selected in the same Drive process, and the newly installed track is immediately retried with residual-only budget.

**Concrete numeric scenario:** active raw track 15 starts this tick at point index 14 of 15, stored residual is 0, fresh current-speed contribution is 20 track-budget units, and a valid next track is selected after completion. This is the same numeric shape encoded by the current Rust focused test.

**Active YR confirmation:** This is standard YR Drive locomotion, not dormant TS legacy. `ini/rulesmd.ini:6603..6643` has Grizzly `[MTNK]` using `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` and `Accelerates=false`. `DriveLocomotionClass::Process @ 0x004B0500` calls `Process_Drive_Track(0)` on the active-track branch, then can call `Process_Movement`, then calls `Process_Drive_Track(1)`. No TS-only gate guards this branch in the decompiled active path.

## Pipeline

| Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|
| 1. Active track process entry | `DriveLocomotionClass::Process @ 0x004B0500` active-track branch calls `Process_Drive_Track(0)` first. | `tick_movement_with_grids` calls `advance_lepton_position`, which advances an existing `entity.drive_track` before normal cell crossings (`src/sim/movement/movement_tick.rs:1083`, `src/sim/movement/movement_step.rs:409`). | PASS for this active-track entry order. |
| 2. First track budget and finish | `Process_Drive_Track(0)` computes `(GetCurrentSpeed & mask_for_retry_0) + residual = 20 + 0 = 20`; strict point loop subtracts 7 once to finish point 15, leaving residual 13. | `advance_drive_track_with_budget(state, 20, &mut residual)` uses `budget = 20 + 0`; with raw track 15 point 14 -> 15, leaves `state.residual = 13` and caller residual `13` (`src/sim/movement/drive_track.rs:3741..3781`; test `drive_track_finish_preserves_residual_for_same_tick_retry`, `src/sim/movement/drive_track_tests.rs:183..194`). | PASS. |
| 3. Residual is preserved across old-track finish | gamemd stores leftover budget in DriveLocomotion `+0x4C`; completion does not clear it before the later retry in the same top-level Process call. | Rust writes both `state.residual` and `drive.residual_budget`; the finished-track branch clears/replaces the track state but does not clear the runtime residual before retry (`src/sim/movement/drive_track.rs:3777..3781`, `src/sim/movement/movement_step.rs:450..507`). | PASS. |
| 4. Same-tick next-track selection | gamemd reaches `Process_Movement` after active-track completion and, if movement selection succeeds, the same top-level call continues to `Process_Drive_Track(1)`. | Rust selects/begins the next track directly inside `advance_lepton_position` after finished track state (`src/sim/movement/movement_step.rs:457..499`). It is same-tick, but this trace did not prove every `Process_Movement` side effect and state byte around selector equivalence. | UNCHECKED. |
| 5. Retry masks fresh speed to zero | gamemd retry call uses nonzero `param_2`; budget expression masks fresh `GetCurrentSpeed` to 0, so retry budget is `0 + residual = 13`. | `advance_drive_track_retry_after_selection` calls `advance_drive_track_with_budget(track_state, 0, &mut drive.residual_budget)` (`src/sim/movement/movement_step.rs:324..340`). | PASS. |
| 6. Retry consumes residual-only budget | gamemd with retry budget 13 and strict `> 7` loop consumes one new-track point and leaves residual `13 - 7 = 6`; it must not add a second fresh speed contribution. | Current focused test `drive_track_completion_retries_new_track_with_residual_only` computes final `drive.residual_budget == 6`, new track `state.residual == 6`, and synchronized point index after same-tick retry (`src/sim/movement/movement_step.rs:269..320`). | PASS. |

## Findings

No FAIL or NOT-IMPLEMENTED finding for the concrete residual-only retry mechanics. Current Rust does preserve residual on old-track finish and immediately retries a newly installed DriveTrack with `fresh_budget = 0`.

Remaining unchecked parity: Rust's next-track selection is embedded in `advance_lepton_position`, not a byte-for-byte `DriveLocomotionClass::Process_Movement` ownership boundary. For this trace's numeric residual result, equality is proven; for all surrounding selector side effects, exact state-byte equality is UNCHECKED.

## Adjacent Findings

- Full Drive owner-loop parity still needs the surrounding `Process_Movement` side effects audited: path/NavCom writes, collision return handling, tube path state, and action-line target fields.
- Slope-first Drive process ordering was out of scope.
- No-active-track `Process_Movement -> Process_Drive_Track(0)` startup path was out of scope.

## Verdict Tally

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

