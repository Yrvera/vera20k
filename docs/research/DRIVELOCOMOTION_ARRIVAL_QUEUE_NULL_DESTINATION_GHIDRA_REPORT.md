# DriveLocomotion Arrival Queue / Null Destination - Ghidra Report

**Date:** 2026-05-27
**Investigation mode:** exhaustive-slice for normal Drive move-to-cell arrival branch shape; coverage-map for frame-render visibility and all non-cell destinations.
**Primary addresses:** `DriveLocomotionClass::Process @ 0x004B0500`, `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`, `FootClass::OnArrival @ 0x004D82B0`, `FootClass::Stop_Moving @ 0x004DF0D0`, `FootClass::Set_Destination_Internal @ 0x004D94B0`.
**Active in YR:** Yes. These are live standard YR Drive locomotor and FootClass paths for normal ground vehicles.

## Target Question

Verify DriveLocomotion arrival behavior for normal Drive move-to-cell destinations:

- empty `NavQueue` versus non-empty `NavQueue` split;
- when owner `Set_Destination(NULL, 1)` is called;
- when `FootClass::Stop_Moving` plus owner `OnArrival(0, 1)` is called;
- whether owner `NavCom` is cleared immediately or remains visible;
- exact ordering relative to Drive track/head-to clearing and owner `PerCellProcess`.

## Non-goals

- Do not redo refinery-specific accepted-cell docking except where prior accepted-cell visibility helps distinguish normal movement from dock admission.
- Do not prove every `PerCellProcess` building/dock branch.
- Do not prove action-line render frame ordering; only local state visibility inside locomotor and owner calls is verified here.
- Do not decode every `Process_Movement` block/collision branch beyond arrival/null-destination effects.

## Evidence Needed To Mark COMPLETE

- `DriveLocomotionClass::Process @ 0x004B0500` no-active-track arrival predicates and queue split.
- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` track-end clear order and owner hook calls.
- `FootClass::OnArrival @ 0x004D82B0` queue pop semantics.
- `FootClass::Stop_Moving @ 0x004DF0D0` exact owner field writes.
- `FootClass::Set_Destination_Internal @ 0x004D94B0` null-target owner and locomotor clear behavior.
- Current Rust surfaces for handoff, not implementation.

## Stop Conditions

Stop once the normal Drive cell-destination lifecycle is proven for:

- no-active-track arrival;
- track-end arrival;
- empty queue;
- non-empty queue;
- local `NavCom` visibility into `PerCellProcess`;
- Rust handoff and stale-doc wording.

Defer frame-render visibility, queue producers, building/dock destinations, chrono piggyback, aircraft, and exact mission-dispatch frame number.

## Verified Findings

### 1. `NavQueue.Count2` is owner `+0x598`, decompiler index `owner[0x166]`

`DriveLocomotionClass::Process` tests `((int *)owner)[0x166] == 0` in both no-active-track arrival branches. `FootClass::OnArrival` tests `param_1[0x166] > 0`, reads the first queued pointer from `param_1[0x163]` (`+0x58C`), calls owner vtable `+0x480` with that pointer and `0`, then decrements `+0x598` and shifts remaining entries left.

**Evidence:** `0x004B0500`, `0x004D82B0`.
**Active in YR:** Yes.

### 2. No-active-track cell arrival uses the empty/non-empty queue split in `DriveLocomotionClass::Process`

When Drive is not mid-track (`Drive+0x58 == -1` or head-to flag `+0x5F == 0`), `Process` checks owner `NavCom` at `+0x5A4`. If `NavCom->WhatAmI()` returns `0xB` (`CellClass`) and owner current cell from vtable `+0x1B8` equals the `CellClass` coord at `+0x24/+0x26`, arrival is accepted.

The split is:

- `owner+0x598 == 0`: call owner vtable `+0x480(0, 1)` and return. This is the empty-queue null-destination path.
- `owner+0x598 != 0`: jump to the shared arrival label, call `FootClass::Stop_Moving()`, then owner vtable `+0x484(0, 1)`, and return. This is the queued-arrival path.

**Evidence:** `DriveLocomotionClass::Process @ 0x004B0500`, first branch before the mission-5 coordinate branch.
**Active in YR:** Yes for normal Drive movement to a cell target.

### 3. Mission-5 coordinate arrival uses the same split

Still in the no-active-track part of `Process`, if owner current mission at `+0xAC` is `5`, Drive head-to flag `+0x5F` is clear, and Drive destination coords `+0x30/+0x34/+0x38` are non-null and equal owner world coords `+0x9C/+0xA0/+0xA4`, the same split is used:

- empty queue: owner `+0x480(0, 1)`;
- non-empty queue: `Stop_Moving()` then owner `+0x484(0, 1)`.

**Evidence:** `0x004B0500`, mission-5 block after the `CellClass` NavCom block.
**Active in YR:** Yes, conditional on mission/value state.

### 4. Empty queue calls owner `Set_Destination(NULL, 1)` and does not call `OnArrival` in that same branch

The empty-queue arrival branch in `Process` calls only owner vtable `+0x480(0, 1)` and returns. It does not call owner `+0x484` from that branch.

For Foot-derived ground units, `+0x480` resolves to `FootClass::Set_Destination_Internal @ 0x004D94B0` in the Foot vtable path. With `param_2 == 0`, it writes `NavCom = 0`, then usually calls active locomotor vtable `+0x48` (`Clear_Navigation`) unless the UnitClass attack/TarCom carve-out applies. Normal move-to-cell is not that attack exception.

**Evidence:** `0x004B0500`, `0x004D94B0`.
**Active in YR:** Yes.

### 5. Non-empty queue clears `NavCom` transiently, then `OnArrival` pops the next queued destination

The non-empty branch calls `FootClass::Stop_Moving @ 0x004DF0D0` first. That function only writes:

- owner `+0x5A0 = 0` (`NavCom_Aux`);
- owner `+0x5A4 = 0` (`NavCom`).

Then owner vtable `+0x484(0, 1)` is called. For Unit/Infantry arrival dispatch this begins with `FootClass::OnArrival`, and `OnArrival` immediately handles `NavQueue.Count2 > 0` by calling owner vtable `+0x480(NavQueue.Buffer[0], 0)`, then decrementing Count2 and shifting the queue left.

Therefore, in the queued-arrival path, `NavCom` is cleared before `OnArrival`, but if the queued target is accepted by `Set_Destination_Internal`, a new `NavCom` is installed before `OnArrival` returns.

**Evidence:** `0x004B0500`, `0x004DF0D0`, `0x004D82B0`, `0x004D94B0`.
**Active in YR:** Yes. Later producer coverage found no standard runtime player/team/trigger NavQueue append path; this consumer behavior still applies when save-load or legacy/unknown state leaves entries present.

### 6. Track-end arrival does not immediately perform the normal empty-queue `Set_Destination(NULL,1)` split

When Drive is mid-track, `Process` enters the active-track branch and calls `Process_Drive_Track(0)`. If `Process_Drive_Track` returns nonzero, `Process` returns without running the no-active-track cell-arrival check in the same call.

Inside `Process_Drive_Track`, the track-end branch is the zero track-delta marker: current track point `dx == 0`, `dy == 0`, and `step_index != 0`. In that branch Drive:

1. updates owner position/occupation through owner virtuals around `+0x1B4` and `+0x1CC`;
2. clears Drive head-to/intermediate coord `+0x40/+0x44/+0x48` to the Drive null coord and clears byte `+0x63`;
3. sets active track `+0x58 = -1`;
4. sets track step `+0x5C = 0`;
5. if owner `NavCom` exists and owner current cell equals `NavCom->GetDockCoord(owner)` cell, with Z delta `< g_DriveHeightStep * 2`, clears Drive destination coord `+0x34/+0x38/+0x3C` and clears head-to again;
6. calls owner vtable `+0x18C(2)`;
7. calls owner vtable `+0x504` when owner is active and not in the checked inactive/limbo bytes.

There is no generic empty-queue `owner+0x480(0,1)` call in this normal track-end sequence. Owner `NavCom` remains non-null for local consumers such as owner `+0x504` unless a separate conditional branch calls `Stop_Moving`.

**Evidence:** `Process_Drive_Track @ 0x004B0F20`, zero-delta track-end branch.
**Active in YR:** Yes for Drive track completion.

### 7. `PerCellProcess` observes Drive-cleared state before normal owner `NavCom` clear

The owner vtable `+0x504` call in `Process_Drive_Track` occurs after Drive has cleared its track/head-to fields and after owner vtable `+0x18C(2)`. In the normal track-end path, owner `NavCom` has not been cleared by `Set_Destination(NULL,1)` yet.

This means owner per-cell processing can see:

- current object coords already advanced to the arrived cell;
- Drive active track `+0x58 == -1`;
- Drive head-to/intermediate coord cleared;
- Drive destination coord cleared if the current cell matches the NavCom destination coord with the height tolerance;
- owner `NavCom` still pointing at the destination.

For UnitClass, prior and fresh decompile evidence resolves this owner hook to the Unit per-cell processing family, including `UnitClass::PerCellProcess @ 0x00739EC0` for vehicle-specific arrival/dock handling.

**Evidence:** `0x004B0F20`, `0x00739EC0`; prior accepted-cell visibility report.
**Active in YR:** Yes.

### 8. The next no-active-track `Process` pass is the normal place the empty-queue owner `NavCom` clear happens after a track-end arrival

After `Process_Drive_Track` has ended a track and returned without the no-active-track cell-arrival split, the next `DriveLocomotionClass::Process` invocation begins with no active track. If owner `NavCom` is a `CellClass` and current cell equals it, the branch in finding 2 fires:

- empty queue: owner `+0x480(0,1)` clears `NavCom`;
- non-empty queue: `Stop_Moving()` clears `NavCom`, then owner `+0x484(0,1)` pops/reissues the next queued destination.

**Evidence:** ordering in `0x004B0500` plus track-end return behavior in `0x004B0F20`.
**Active in YR:** Yes.

### 9. `OnArrival` has a re-entry guard and is not just a queue popper

`FootClass::OnArrival @ 0x004D82B0` first checks byte `+0x6B3`. If already set, it returns `0`. Otherwise it sets `+0x6B3 = 1`, calls the Techno base arrival helper directly, handles deferred hook byte `+0x687`, checks locomotor piggyback validity, then pops `NavQueue` if Count2 is positive. `NAVCOM_ONARRIVAL_TAIL_HOOKS_GHIDRA_REPORT.md` resolves `+0x687`: stock Unit/Infantry concrete vtable `+0x174` targets are Scatter functions, not EVA/audio.

If the queue is empty, `OnArrival` continues into target/attack/infantry shuffle logic and finally calls owner vtable `+0x544(0,0)` before returning `0`.

**Evidence:** `0x004D82B0`.
**Active in YR:** Yes.

### 10. `PathType::Has_Valid_Steps` is separate from NavQueue and DriveTrack state

`PathType::Has_Valid_Steps @ 0x0065AE30` checks path count at `+0xE8`, scans the path pointer at `+0xE4`, and returns true if any path entry is nonzero. It does not inspect:

- owner `NavCom` at `+0x5A4`;
- owner `NavQueue.Count2` at `+0x598`;
- Drive track `+0x58/+0x5C`;
- Drive head-to/destination fields.

**Evidence:** `0x0065AE30`.
**Active in YR:** Yes.

## Inference

- For a normal player move-to-cell with no queued waypoint, the player-visible target/destination state should not be modeled as "path finished so destination vanished" at the exact track-end point. The Drive track can end, owner per-cell hooks can run, and owner `NavCom` can still be visible until the subsequent no-active-track arrival check clears it through `Set_Destination(NULL,1)`.
- For a queued waypoint, the old `NavCom` is intentionally cleared before `OnArrival`, and the next destination is installed by `OnArrival` through `Set_Destination(next,0)`. This makes `NavCom == NULL` a transient state within the queued-arrival call chain.
- Exact action-line render visibility across frame boundaries is not proven here. Local code visibility is proven: `Process_Drive_Track` owner `+0x504` can see non-null `NavCom` after Drive track/head-to clear.

## Current Rust Handoff

Relevant Rust surfaces:

- `src/sim/components.rs:196` `MovementTarget`: movement path, current speed, and destination are one component; no separate owner `NavCom`, `NavQueue`, or Drive destination/head-to fields.
- `src/sim/movement/movement_commands.rs:255` `issue_move_command_with_layered`: current move command writes `MovementTarget` directly and appends queued paths to the same path vector.
- `src/sim/movement/movement_tick.rs:1156` `finalize_finished_entities`: current completion immediately clears `movement_target` and `drive_track`, snaps position, and sets locomotor phase idle.

Required implementation effects for Phase 1:

- Split "owner destination/NavCom" from "active movement path/DriveTrack" for normal Drive movement.
- Do not let Drive track completion alone delete the owner destination.
- Model the empty-queue arrival clear as owner `Set_Destination(NULL,1)` after the no-active-track arrival check, not as unconditional Rust path teardown.
- Model queued arrival as `Stop_Moving` plus `OnArrival(0,1)`: clear current NavCom, pop next queued target, then issue `Set_Destination(next,0)`.
- Preserve a local state where Drive track/head-to is cleared while owner `NavCom` remains visible to per-cell processing.

Acceptance scenarios:

- Normal Drive unit ordered to a cell with empty queue: track end clears Drive track/head-to; owner destination remains visible to per-cell processing; a later no-active-track arrival check clears owner NavCom through the null-destination path.
- Normal Drive unit with one queued destination: first arrival runs `Stop_Moving + OnArrival`, clears old NavCom transiently, installs queued destination, decrements queue count, and shifts queue.
- Rust tests should distinguish `drive_track == None` from `navcom == None`; those are not equivalent in gamemd.

## Do Not Do

- Do not clear Rust's owner-level destination merely because the last DriveTrack point was consumed.
- Do not equate `movement_target == None` with both Drive stopped and owner `NavCom == NULL` unless the null-destination path has actually run.
- Do not call `OnArrival` in the empty-queue `Process` arrival branch; gamemd calls `Set_Destination(NULL,1)` there and lets later mission/arrival handling run through the normal lifecycle.
- Do not treat `NavQueue` as the same thing as the path array checked by `PathType::Has_Valid_Steps`.
- Do not infer action-line frame visibility from this report alone; that belongs to the action-line/renderer timing slot.

## Uncertainty

- The exact producer set for `NavQueue` was resolved by the later `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` for standard YR runtime paths: no player command, TeamClass/AI movement, or trigger waypoint producer was found; only save-load reconstruction positively populates entries.
- The decompiler's local `cVar5` predicate inside the `Process_Drive_Track` track-end branch is not semantically named here. Its verified effect is conditional `Stop_Moving`, path-current-direction clear, and possible owner `+0x484(0,1)` before owner `+0x504`. It is not the generic empty-queue null-destination split.
- Exact frame at which `FootClass::Mission_Move` later calls `OnArrival` after empty-queue `Set_Destination(NULL,1)` depends on mission dispatch timing and is outside this slot.

## Stale-doc Wording

Replace wording that says:

> Drive arrival clears movement target and destination at the same time.

with:

> Drive track completion clears DriveLocomotion track/head-to state first. Owner `NavCom` can remain non-null and visible to owner per-cell processing until the no-active-track arrival check routes through `Set_Destination(NULL,1)` or the queued `Stop_Moving + OnArrival` path.

Replace wording that says:

> `OnArrival` is the empty-queue arrival path from DriveLocomotion.

with:

> In `DriveLocomotionClass::Process`, empty-queue arrival calls owner `Set_Destination(NULL,1)` and returns. The direct `Stop_Moving + OnArrival(0,1)` path is used for non-empty `NavQueue` and certain conditional interruption/arrival branches.

## Sources

- Ghidra read-only decompile: `DriveLocomotionClass::Process @ 0x004B0500`.
- Ghidra read-only decompile: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`.
- Ghidra read-only decompile: `FootClass::OnArrival @ 0x004D82B0`.
- Ghidra read-only decompile: `FootClass::Stop_Moving @ 0x004DF0D0`.
- Ghidra read-only decompile: `FootClass::Set_Destination_Internal @ 0x004D94B0`.
- Ghidra read-only decompile: `PathType::Has_Valid_Steps @ 0x0065AE30`.
- Ghidra read-only decompile: `UnitClass::PerCellProcess @ 0x00739EC0`.
- Existing context: `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`.
- Existing context: `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`.
- Existing context: `docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/components.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`.

## Status

COMPLETE for normal Drive move-to-cell arrival queue/null-destination ordering and local `NavCom` visibility. PARTIAL for render-frame action-line visibility and non-cell destination variants. NavQueue producer coverage for standard runtime player/team/trigger paths was resolved separately by `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md`.
