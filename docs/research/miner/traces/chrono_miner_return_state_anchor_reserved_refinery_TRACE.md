# Chrono Miner ReturnToRefinery Anchor With Reserved Refinery Trace

Date: 2026-05-20  
Trace slot: 2  
Scenario: `CMIN` is already in `ReturnToRefinery`; `miner.reserved_refinery = Some(GAREFN)`; refinery is owned/friendly, alive, powered, and valid for `CMIN`.

> **Repo-status supersession 2026-05-25:** Adjacent notes below about Rust's
> hardcoded 2-cell chrono inbound threshold are stale. Current Rust reads
> `ChronoHarvTooFarDistance` for the close/far split. This trace's anchor and
> reserved-refinery observations should be read separately from that old status.

## Scope

This trace only answers which coordinate anchors the reserved-refinery return-to-dock transition for a Chrono Miner. It does not trace ore scanning, unloading cadence, refinery exit, alternate refinery choice, or busy-refinery queue eviction.

Concrete coordinate frame used for literal equality:

- `GAREFN` top-left cell: `(10, 10)`.
- Retail `GAREFN` art: `Foundation=4x3`, `RemoveOccupy1=3,1`, `QueueingCell=4,1`, no active `DockingOffset0`.
- `CMIN` transition tick input: current cell `(13, 11)`, `ReturnToRefinery`, `reserved_refinery = GAREFN`, no active teleport state, no movement target.

## Sources Checked

- Rust:
  - `src/sim/miner/miner_system.rs:580`
  - `src/sim/miner/miner_system.rs:613`
  - `src/sim/miner/miner_system.rs:638`
  - `src/sim/miner/miner_system.rs:643`
  - `src/sim/miner/miner_system.rs:1046`
  - `src/sim/miner/miner_dock_sequence.rs:84`
  - `src/sim/miner/miner_dock_sequence.rs:95`
  - `src/sim/miner/miner_dock_sequence.rs:310`
  - `src/sim/miner/miner_dock_sequence.rs:441`
- INI:
  - `ini/rulesmd.ini`: `[CMIN] Harvester=yes`, `Dock=NAREFN,GAREFN`, `Storage=20`, `UnloadingClass=CMON`.
  - `ini/rulesmd.ini`: `[GAREFN] DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `FreeUnit=CMIN`.
  - `ini/artmd.ini`: `[GAREFN] Foundation=4x3`, `QueueingCell=4,1`, `RemoveOccupy1=3,1`; `DockingOffset0` is not present for this section.
- Ghidra read-only:
  - `UnitClass__Mission_Harvest` at `0x73E5E0`.
  - `BuildingClass__Receive_Radio` at `0x43C2D0`.
  - `BuildingClass__GetDockCoord` at `0x447B20`.
  - `TechnoClass__Set_Destination` at `0x741970`.

No Ghidra mutating tools were used.

## Active-YR Confirmation

The traced binary branches are active in standard Yuri's Revenge for this scenario:

- `CMIN` has `Harvester=yes` and `Teleporter=yes`; `Mission_Harvest` state 2 uses the harvester path and the chrono distance branch.
- `GAREFN` has `DockUnload=yes` and `Refinery=yes`; `BuildingClass__Receive_Radio` case `0x0E` takes the DockUnload/refinery handling path.
- `GAREFN` has one dock (`NumberOfDocks=1`) and its retail art removes cell `(origin+3, origin+1)` from the foundation blockers, making that cell the visible dock pad.

These are not dormant TS-only paths for this concrete `CMIN`/`GAREFN` setup.

## Pipeline

1. Trigger: `tick_miners` snapshots `CMIN` and dispatches `ReturnToRefinery` to `handle_return`.
2. Reserved target: `handle_return` reads `reserved_refinery` and resolves the refinery through `refinery_dock_for_sid`.
3. Anchor computation: Rust calls `refinery_dock_cell`, which delegates to `refinery_can_dock_queue_cell(rx, ry) = (rx+3, ry+1)`.
4. Transition gate: Rust enters `MinerState::Dock` when the miner is at/adjacent to that anchor or within `CloseEnough`.
5. Dock phase: `handle_dock_sequence` recomputes `queue=(rx+3, ry+1)` and `pad=(rx+3, ry+1)` for retail `GAREFN`, then moves/links on the pad.
6. Binary equivalent: `Mission_Harvest` state 2 reserves the refinery by radio; `BuildingClass__Receive_Radio` case `0x0E` sends the unit to `building_cell + (3,1)`; `Mission_Enter`/`GetDockCoord` handles the dock coordinate, not the art `QueueingCell`.

## Stage Results

| Stage | Output Checked | Rust Output | gamemd.exe Output | Verdict |
|---|---:|---:|---:|---|
| Retail data activation | CMIN/GAREFN branch eligibility | `CMIN` harvester/chrono, `GAREFN` refinery dock | Same INI-driven active branches in `Mission_Harvest` and `Receive_Radio` | PASS |
| Reserved-refinery lookup | Target refinery retained | `reserved_refinery = GAREFN` used directly | State 2 uses the selected/found refinery and radio-reserves it | PASS |
| Transition anchor cell | Anchor for Return-to-Dock transition | `(10+3, 10+1) = (13,11)` | `Receive_Radio 0x0E`: `building_cell + (3,1) = (13,11)` | PASS |
| Pad/dock cell for physical link | Pad cell after transition | no DockingOffset fallback: `(10+4-1, 10+3/2) = (13,11)` | `GetDockCoord` refinery path / dock protocol lands on the same GAREFN dock cell `(13,11)` | PASS |
| Art QueueingCell distinction | Whether `QueueingCell=4,1` is authoritative here | Not used for reserved transition | Not used for accepted reserved dock transition; it is a waiting/queue-adjacent concept | PASS |
| Chrono distance preemption outside this tick | Far-return destination equality | Rust has a hardcoded 2-cell chrono inbound warp threshold | gamemd uses `ChronoHarvTooFarDistance=50` for the state-2 radio-vs-destination decision | UNCHECKED |
| Movement destination after far fallback | Destination from passable-cell search | Rust teleports/moves to the pad cell in this path | gamemd computes a passable cell from loaded dock-offset fields before `Set_Destination` | UNCHECKED |

## Answer

For the concrete reserved-refinery transition tick, gamemd's authoritative anchor is the refinery dock/pad cell at `GAREFN origin + (3,1)`.

With `GAREFN` at `(10,10)`, the authoritative anchor is exactly `(13,11)`.

It is not the refinery center `(12,11)` in Rust's cell math, not the adjacent art `QueueingCell=(14,11)`, and not a later arbitrary movement destination. For retail `GAREFN`, the accepted-radio movement target and the physical dock pad collapse to the same cell `(13,11)`, because `RemoveOccupy1=3,1` opens that foundation cell for the harvester dock.

## Failures

No player-visible FAIL was proven for the requested anchor scenario.

## Not Implemented

No NOT-IMPLEMENTED stage was found for this exact anchor transition.

## Adjacent Findings

- Rust's hardcoded 2-cell chrono inbound threshold differs from standard YR `ChronoHarvTooFarDistance=50`. This is player-visible for non-anchor positions, but it is outside this trace's concrete transition tick and was not expanded here.
- The far-return movement-destination path in gamemd uses a passable-cell search from loaded building dock-offset fields before `Set_Destination`. This trace did not audit the loaded `GAREFN` dock-offset field values beyond the reserved transition anchor.

## Verdict Tally

PASS: 5  
FAIL: 0  
UNCHECKED: 2  
NOT-IMPLEMENTED: 0

## Status

COMPLETE
