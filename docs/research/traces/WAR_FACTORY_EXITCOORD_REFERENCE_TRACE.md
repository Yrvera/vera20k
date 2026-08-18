# War Factory ExitCoord / Building Reference Point Trace

Scenario: stock `GAWEAP` at building origin cell `(20,20)` produces a vehicle. Compare gamemd stock land war-factory spawn reference (`ExitCoord=512,256,0` as lepton offset from `BuildingClass` world coords, initial facing byte `0x40`) against current Rust production spawn / ExitCoord helpers.

Date: 2026-05-22

## Verdict

PASS: 5 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

No player-visible FAIL or NOT-IMPLEMENTED findings were found for this concrete stock centered-ExitCoord scenario.

## Pipeline

Production completion -> producer selection -> stock land war-factory ExitCoord path -> world lepton spawn coordinate -> entity spawn at selected cell center -> initial facing -> optional contact marker for building-entry row skip.

## Evidence Summary

- Existing gamemd research `DOCKING_QUEUE_EXIT_REFERENCE_POINTS_GHIDRA_REPORT.md` verifies stock land WFs bypass `GetDockCellForObject`, call `GetExitCoord`, and unlimbo produced vehicles at facing byte `0x40`.
- Existing gamemd research `BUILDING_FOUNDATION_ANCHOR_SEMANTICS_GHIDRA_REPORT.md` verifies building stored position is the foundation-origin cell center; the separate center virtual is not the stored anchor.
- Stock YR INI liveness is confirmed by `ini/rulesmd.ini`: `[GAWEAP]` has `WeaponsFactory=yes`, `Factory=UnitType`, `ExitCoord=512,256,0`, `NumberImpassableRows=1`, and no `Naval=yes`; `ini/artmd.ini` provides `Foundation=5x3`.
- Current Rust inspected surfaces: `src/rules/object_type.rs`, `src/sim/production/production_spawn.rs`, `src/sim/production/production_queue.rs`, `src/sim/world/world_spawn.rs`, `src/sim/game_entity.rs`, `src/sim/components.rs`, plus existing production tests.

No Ghidra mutating tools were used. No Rust, INI, in-repo docs, or claims files were modified.

## Concrete Values

Assumptions for this trace:

- Cell lepton size is 256.
- A cell center is represented as cell coordinate plus sub-cell `(128,128)`.
- Building origin cell `(20,20)` therefore has stored world coord `(20*256+128, 20*256+128, 0) = (5248,5248,0)`.

## Stage Trace

### Stage 1 - Active Stock Data

Input: standard YR `GAWEAP`.

gamemd expected: land war factory, not naval, `ExitCoord=(512,256,0)`.

Rust inspected: parser stores `exit_coord: Option<(i32,i32,i32)>` from `ExitCoord` in `src/rules/object_type.rs:482`, `src/rules/object_type.rs:1009`, `src/rules/object_type.rs:1215`; stock/test fixture uses `ExitCoord=512,256,0` in `src/sim/production/production_tests.rs:454`.

Output comparison: `ExitCoord=(512,256,0)` on both sides for this scenario.

Verdict: PASS.

### Stage 2 - Building Reference Point

Input: `GAWEAP` origin cell `(20,20)`.

gamemd expected: stored `BuildingClass` world coords are foundation-origin cell center, so `(5248,5248,0)`.

Rust inspected: entities store `(rx,ry)` plus sub-cell lepton offsets in `src/sim/components.rs:22`; `GameEntity::new` centers non-infantry/non-special spawns with `CELL_CENTER_LEPTON` in `src/sim/game_entity.rs:316`.

Rust computed: `(20,20)` plus `(128,128)` gives `(5248,5248,0)`.

Verdict: PASS.

### Stage 3 - ExitCoord Lepton Addition

Input: building world coord `(5248,5248,0)` and `ExitCoord=(512,256,0)`.

gamemd expected: `GetExitCoord` adds the lepton vector directly, producing `(5760,5504,0)`.

Rust inspected: `find_exact_exitcoord_spawn_cell` reads `exit_coord`, converts lepton offsets to cell offsets, and adds them to the producer origin in `src/sim/production/production_spawn.rs:261`.

Rust computed for this centered stock value: `lepton_to_cell(512)=2`, `lepton_to_cell(256)=1`, candidate cell `(22,21)`; `spawn_object` then creates a centered vehicle at `(22*256+128,21*256+128,0)=(5760,5504,0)` via `src/sim/world/world_spawn.rs:278` and `src/sim/game_entity.rs:316`.

Verdict: PASS.

### Stage 4 - Spawn Cell

Input: `GAWEAP` origin `(20,20)`.

gamemd expected: world coord `(5760,5504,0)` maps to cell `(22,21)`.

Rust inspected: exact land vehicle factories take the special exact path in `src/sim/production/production_spawn.rs:101`; candidate is produced at `src/sim/production/production_spawn.rs:272`.

Rust computed: `(20+2,20+1)=(22,21)`. Existing Rust tests assert this for the exact scenario in `src/sim/production/production_tests.rs:688` and for a clear path in `src/sim/production/production_placement_tests.rs:1125`.

Verdict: PASS.

### Stage 5 - Initial Facing

Input: completed produced vehicle.

gamemd expected: unlimbo facing byte `0x40` decimal `64`.

Rust inspected: production completion calls `sim.spawn_object(..., 64, ...)` in `src/sim/production/production_queue.rs:535`; `spawn_object` forwards the byte in `src/sim/world/world_spawn.rs:278`; `GameEntity::new` stores it in `src/sim/game_entity.rs:337`.

Rust computed: initial facing `64` decimal, equal to `0x40`.

Verdict: PASS.

### Stage 6 - Tick Timing / Same-Frame Ordering

gamemd expected: stock `ExitObject_Main` unlimbo occurs in the production-exit flow after completion.

Rust inspected: `tick_production` advances completion, selects a spawn, and calls `spawn_object` in the same queue tick in `src/sim/production/production_queue.rs:411`.

Computed equality: not computed against a concrete gamemd tick transcript for this exact scenario.

Verdict: UNCHECKED.

### Stage 7 - Contact Marker Shape

gamemd expected: research indicates stock land WF successful unlimbo creates the live contact relationship used by building-entry row-skip behavior.

Rust inspected: after spawn, `mark_war_factory_spawn_contact` marks the produced vehicle with the producer id in `src/sim/production/production_queue.rs:537` and `src/sim/production/production_spawn.rs:156`; the entity contact list is idempotent in `src/sim/game_entity.rs:425`.

Computed equality: current Rust has the produced-unit-to-factory contact needed by the Rust row-skip consumer, but I did not compute the full gamemd reciprocal `RadioClass` state numerically for this scenario.

Verdict: UNCHECKED.

## Adjacent Findings

- Rust correctly prevents the stock land WF exact path from probing neighboring cells when the primary ExitCoord cell is blocked: `src/sim/production/production_placement_tests.rs:1100`.
- Rust also prevents a blocked active war factory from silently routing the completed unit through another factory: `src/sim/production/production_placement_tests.rs:1068`.
- `preferred_exit_offsets` still has neighbor fallback candidates for generic/non-exact paths in `src/sim/production/production_spawn.rs:445`; this trace did not expand into those paths.
- The current exact helper collapses `ExitCoord` through `lepton_to_cell`; this is numerically safe for the stock `512,256,0` centered scenario, but non-cell-centered modded `ExitCoord` values would need a separate trace before claiming sub-cell parity.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None for this concrete stock `GAWEAP` at `(20,20)` producing a vehicle.

## Status

COMPLETE
