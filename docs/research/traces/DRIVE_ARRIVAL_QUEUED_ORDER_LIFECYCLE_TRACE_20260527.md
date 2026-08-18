# Drive Arrival Queued Order Lifecycle Trace - 2026-05-27

Scenario: stock DriveLocomotion vehicle `[MTNK]` starts at cell `(40,40)`, receives a move order to `(42,40)`, then while moving receives a queued move order to `(45,40)`.

Scope is only DriveLocomotion arrival plus queued-order lifecycle for this concrete scenario. Adjacent path smoothing, blockage, formation, combat targeting, refinery docking, and full planning-mode waypoint rendering are out of scope.

## Verdict Summary

PASS: 2 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

The player-visible conclusion is: current Rust implements a runtime `navigation.nav_queue` append for a queued move command, but the verified standard YR player/team/trigger runtime paths do not append to `FootClass` NavQueue. Therefore the concrete scenario does not reach gamemd's non-empty NavQueue arrival consumer through a normal player queued move. Rust can visibly continue from `(42,40)` to `(45,40)` via `nav_queue`; gamemd evidence says `Foot+0x598` stays `0` for standard player queued movement, so the first arrival uses the empty-queue `Set_Destination(NULL,1)` path unless another separately verified command path reissued/replaced the destination.

## Concrete Data

- Unit: `[MTNK]` Grizzly Battle Tank.
- INI facts from `ini/rulesmd.ini:6603..6644`: `Speed=7`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`, `Accelerates=false`.
- Cell centers in Drive lepton coordinates:
  - `(40,40)` -> `(10368,10368,0)`.
  - `(42,40)` -> `(10880,10368,0)`.
  - `(45,40)` -> `(11648,10368,0)`.
- Flat-ground Z is assumed `0`; no bridge/deck adjustment is part of this concrete scenario.

## Pipeline

1. Player move command to `(42,40)` -> Unit destination vtable `+0x480`.
2. `UnitClass::Set_Destination` preprocessing -> `FootClass::Set_Destination_Internal`.
3. Owner `NavCom` set to first cell target -> active Drive locomotor receives `Head_To_Coord`.
4. While moving, Rust queued command appends `(45,40)` to `navigation.nav_queue`.
5. In gamemd standard runtime, no player/team/trigger NavQueue append producer was found; `Foot+0x598` remains `0` for this command class.
6. Arrival at `(42,40)` therefore diverges: Rust has a queued target; gamemd's standard runtime evidence has empty NavQueue.

## Stage Findings

### Stage 1 - Control Unit Uses Active Standard DriveLocomotion

gamemd: `[MTNK]` uses Drive CLSID `{4A582741-9839-11d1-B709-00A024DDAFD1}`, and DriveLocomotion process/arrival functions are active standard YR paths. Evidence: `rulesmd.ini:6635..6643`, `DriveLocomotionClass::Process @ 0x004B0500`, `DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md`.

Rust: `LocomotorKind::Drive` is the matching normal ground vehicle path.

Verdict: PASS for unit/locomotor classification.

### Stage 2 - First Destination `(42,40)` Owner/Drive State

gamemd: successful normal empty-cell destination writes owner `NavCom`, resolves target coords, and calls active Drive vtable `+0x44`; Drive writes destination coord `(10880,10368,0)` on flat ground. Evidence: `UNITCLASS_SET_DESTINATION_NORMAL_DRIVE_CELL_GHIDRA_REPORT.md`; Ghidra `0x004D94B0`, `0x004AFD40`.

Rust: `set_destination_internal_cell` writes `navigation.nav_com = Cell(42,40)` and `drive.destination/head_to = DriveCoord::cell(42,40,0)`. Relevant code: `src/sim/movement/navcom.rs:41..55`, `src/sim/movement/navcom.rs:109..115`.

Verdict: PASS for the concrete cell-coordinate owner/Drive destination values. Exact `AbstractClass*` pointer identity is not modeled in Rust and remains an architecture delta outside this stage's numeric coordinate check.

### Stage 3 - Queued Move Command Producer

gamemd: `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` found no standard YR runtime producer that appends player, TeamClass/AI, or trigger movement into `Foot+0x58C/+0x598`. `EventClass::Execute @ 0x004C6CB0` routes player commands through destination vtable calls and has no `Foot+0x58C/+0x598` append.

Rust: when `queue=true`, entity is already moving, locomotor is Drive, and `nav_com` is non-null, Rust appends the new destination to `entity.navigation.nav_queue` and returns without replacing the active movement target. Relevant code: `src/sim/movement/movement_commands.rs:364..380`. For this scenario, Rust stores `nav_queue = [Cell(45,40)]`.

Verdict: FAIL. Concrete state differs: gamemd standard runtime `Foot+0x598 = 0`; Rust `navigation.nav_queue.len() = 1`.

### Stage 4 - Selected Movement Line After Queued Command

gamemd: selected movement action-line endpoint is `NavQueue.Last` only if `NavCom` is non-null and queue count is nonzero; otherwise it uses `NavCom`. The queue consumer is active, but the standard player command producer is not verified. Evidence: `NAVCOM_NAVQUEUE_ACTION_LINE_ENDPOINT_VISIBILITY_GHIDRA_REPORT.md`; `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md`.

Rust: action-line rendering requires `nav_com`, then prefers `navigation.nav_queue.last()` over `nav_com`. Relevant code: `src/app_target_lines.rs:195..201`. In this scenario Rust draws to `(45,40)` from the queued entry while still moving toward `(42,40)`.

Verdict: FAIL for mechanism and live state. The exact gamemd visual endpoint after the user's queued click is UNCHECKED because this trace did not fully decode whether the command is ignored, reissued, or planning-mode-owned; it is not a `Foot NavQueue` append.

### Stage 5 - Arrival At First Destination `(42,40)`

gamemd: with `Foot+0x598 == 0`, the no-active-track arrival branch in `DriveLocomotionClass::Process @ 0x004B0500` calls owner vtable `+0x480(0,1)`, i.e. `Set_Destination(NULL,1)`, and does not call `OnArrival` from that empty-queue branch. Evidence: `DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md` findings 2, 4, and 8.

Rust: after path completion, `finalize_finished_entities` defers Drive arrival clear, leaves `nav_com` visible, and clears `movement_target`/`drive_track`; next movement tick `process_pending_drive_arrivals` sees nonempty `nav_queue`, removes the first queued cell, calls `foot_stop_moving`, calls `set_destination_internal_cell`, and issues a new `MovementTarget` to `(45,40)`. Relevant code: `src/sim/movement/movement_tick.rs:1539..1547`, `src/sim/movement/movement_tick.rs:394..539`.

Verdict: FAIL. Concrete arrival branch differs because Rust's unsupported runtime queue entry changes empty-queue gamemd arrival into a queued reissue.

### Stage 6 - Synthetic Nonzero NavQueue Consumer Ordering

gamemd: if `Foot+0x598 > 0` already exists, Drive arrival calls `FootClass::Stop_Moving`, then owner vtable `+0x484(0,1)`, which begins with `FootClass::OnArrival`; `OnArrival` has a re-entry guard, base arrival work, optional hooks, then pops `NavQueue[0]` through owner vtable `+0x480(next,0)`. Evidence: `0x004B0500`, `0x004DF0D0`, `0x004D82B0`, `0x004D94B0`.

Rust: `process_pending_drive_arrivals` manually removes `navigation.nav_queue[0]`, calls `foot_stop_moving`, and then calls `set_destination_internal_cell` directly before pathfinding/reissuing `MovementTarget`. Relevant code: `src/sim/movement/movement_tick.rs:428..539`, `src/sim/movement/navcom.rs:69..73`.

Verdict: FAIL for exact process order and missing `OnArrival` semantics. This is conditional because the concrete standard player scenario should not have produced a nonzero Foot/NavQueue entry in the first place.

### Stage 7 - Exact Movement Tick Count To Reach `(42,40)`

gamemd: exact tick count depends on DriveTrack point cadence, current-speed fraction, terrain cost, residual budget, and tick order. This trace did not compute a gamemd-vs-Rust literal tick count for `(40,40)->(42,40)`.

Rust: not executed in this trace with an instrumented fixture.

Verdict: UNCHECKED.

### Stage 8 - Exact Standard YR User Queued-Click Semantics

gamemd: this trace verifies the queued click does not append to Foot/NavQueue in the audited player command path. It did not complete a full UI/event/planning-mode trace proving whether the second click replaces current `NavCom`, is ignored under a specific mode, or belongs to a separate `WaypointPathClass` planning surface.

Rust: the command path passes `queue` through `world_commands.rs` to `issue_move_command_with_layered`, where the Drive-specific append occurs. Relevant code: `src/sim/world/world_commands.rs:251..258`, `src/sim/movement/movement_commands.rs:364..380`.

Verdict: UNCHECKED for the complete native queued-click outcome; FAIL already recorded for the unsupported Foot/NavQueue append.

### Stage 9 - Final Screen Result After Both Destinations

gamemd: exact final player-visible route/stop after the second queued click was not computed because Stage 8 is unresolved.

Rust: if pathfinding succeeds, current code will move to `(42,40)`, then reissue a path to `(45,40)` from the queued entry.

Verdict: UNCHECKED.

### Stage 10 - Standard-YR-Compatible Queued Move Lifecycle

gamemd: standard runtime player/team/trigger movement does not create `Foot+0x58C/+0x598` entries; nonzero queue is save-load/legacy/unknown state per current evidence.

Rust: standard runtime queued movement does create `navigation.nav_queue` entries for normal Drive movement.

Verdict: NOT-IMPLEMENTED. The standard-YR-compatible queued command lifecycle should not use Foot/NavQueue append for this player-visible command path.

## Adjacent Findings

- `Foot/NavQueue` storage should not be deleted: save/load, `OnArrival`, `Mission_Enter`, `PointerExpired`, and action-line readers prove the field exists and can matter when nonzero.
- Planning-mode path lines are a separate surface from this Drive/NavCom arrival trace.
- The exact second-click native behavior needs a separate trace starting at UI/EventClass command construction, not at Drive arrival.

## Top Player-Visible Findings

1. Stage 3 FAIL - Rust appends `(45,40)` to `navigation.nav_queue`, but standard YR player commands do not append to `Foot+0x58C/+0x598`; Rust `src/sim/movement/movement_commands.rs:364`, gamemd evidence `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` / `EventClass::Execute @ 0x004C6CB0`.
2. Stage 5 FAIL - Rust first arrival pops/reissues `(45,40)`, while gamemd standard runtime reaches empty-queue `Set_Destination(NULL,1)` at `(42,40)`; Rust `src/sim/movement/movement_tick.rs:428`, gamemd evidence `DriveLocomotionClass::Process @ 0x004B0500`.
3. Stage 4 FAIL - Rust selected movement line can point at the unsupported queued endpoint; Rust `src/app_target_lines.rs:195`, gamemd evidence `TechnoClass::DrawActionLines @ 0x004DC060` plus no player NavQueue producer.
4. Stage 6 FAIL - Rust synthetic queued-arrival consumer bypasses `OnArrival` guard/hooks and calls destination setup directly; Rust `src/sim/movement/movement_tick.rs:432`, gamemd evidence `FootClass::OnArrival @ 0x004D82B0`.
5. Stage 10 NOT-IMPLEMENTED - standard-YR-compatible queued move command lifecycle is not implemented; Rust `src/sim/world/world_commands.rs:251`, gamemd evidence `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md`.

## Sources

- Ghidra read-only decompile used/spot-checked in this run: `DriveLocomotionClass::Process @ 0x004B0500`, `DriveLocomotionClass::Set_Destination @ 0x004AFD40`, `DriveLocomotionClass::Stop_Moving @ 0x004AFE00`, `FootClass::Set_Destination_Internal @ 0x004D94B0`, `UnitClass/TechnoClass::Set_Destination @ 0x00741970`.
- Research docs: `DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md`, `DRIVELOCOMOTION_HEAD_TO_COORD_CLEAR_NAVIGATION_STATE_GHIDRA_REPORT.md`, `UNITCLASS_SET_DESTINATION_NORMAL_DRIVE_CELL_GHIDRA_REPORT.md`, `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md`, `NAVCOM_NAVQUEUE_ACTION_LINE_ENDPOINT_VISIBILITY_GHIDRA_REPORT.md`.
- Rust files read: `src/sim/components.rs`, `src/sim/movement/navcom.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/world/world_commands.rs`, `src/app_target_lines.rs`.

## Status

COMPLETE for the scoped Drive arrival versus Foot/NavQueue lifecycle mismatch in this concrete scenario. PARTIAL only for the adjacent exact native behavior of the second queued click, which needs its own UI/EventClass planning-mode trace.
