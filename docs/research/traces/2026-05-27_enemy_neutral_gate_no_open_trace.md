# Enemy / Neutral Gate No-Open Trace

**Date:** 2026-05-27  
**Trace slot:** 3  
**Scenario:** A ground mover attempts to enter a non-friendly `Gate=yes` building cell. The gate owner is enemy or neutral/non-allied relative to the mover.  
**Scope:** Mission assignment/open-request behavior and blocked cell-entry outcome for this one scenario only.  
**Status:** COMPLETE

## Verdict Summary

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

No player-visible FAIL / NOT-IMPLEMENTED findings were found in this slot.

## Pipeline

1. Data: `[GAGATE_A]` is a live `Gate=yes` building fixture.
2. Trigger: mover checks a cell occupied by the gate building.
3. Native opener gate: gamemd only calls the gate opener for allied gate buildings.
4. Rust opener gate: Rust only calls `request_open` when `are_houses_friendly(...)` is true.
5. Cell result: closed non-friendly gate remains occupied and blocks entry.

## Stage Table

### Stage 1 - Gate Data

**Rust surface:** `src/rules/object_type.rs:968` parses `Gate=` and `src/rules/object_type.rs:969..974` parses `DeployTime=` / `GateCloseDelay=`.  
**Rust value:** `[GAGATE_A] Gate=yes` -> `obj.gate == true`. `DeployTime=.044` -> `trunc(.044 * 900) == 39`; `GateCloseDelay=.2` -> `180`.  
**gamemd evidence:** `INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md:104` cites `[GAGATE_A] Gate=yes`; `GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:159` cites stock `[GAGATE_A] Gate=yes` and active mission writer path.  
**Active in standard YR:** Conditional on a map-placed `GAGATE_A`; the code and stock data are live, not TS legacy.  
**Verdict:** PASS for the in-scope gate flag and timing values used by current Rust.

### Stage 2 - Native Non-Friendly Mission Assignment

**gamemd path:** `MapClass__Check_Crushable_Obstacle @ 0x00578AD0`.  
**Read-only spot-check:** decompile shows object-list scan at cell `+0xE4`; when occupant `WhatAmI()==6` and `BuildingType+0x16B7 != 0`, it calls `HouseClass__Is_Ally_ByObject(param_1)`. Only if that returns nonzero does it call `FUN_00452540`. Otherwise it calls `BuildingClass__CanGarrison()` and, if false, continues scanning.  
**gamemd value for enemy/neutral closed gate:** `HouseClass__Is_Ally_ByObject == 0`; `FUN_00452540` calls = `0`; `Assign_Mission(0x18,0)` calls through this branch = `0`.  
**Active in standard YR:** Yes/conditional; the function is a live path/obstacle check and the branch requires a `Gate=yes` building.  
**Verdict:** PASS.

### Stage 3 - Native Opener Helper

**gamemd path:** `FUN_00452540 @ 0x00452540`.  
**Read-only spot-check:** decompile shows the helper clears/retargets, calls vtable `+0x1E8` with `(0x18,0)`, then vtable `+0x1EC`, and returns `0` on the assignment path.  
**In-scope effect:** This helper is not reached for enemy/neutral gates from Stage 2, so enemy/neutral contact cannot assign mission `0x18` through this live opener callsite.  
**Active in standard YR:** Yes/conditional; reached by the allied branch only.  
**Verdict:** PASS.

### Stage 4 - Rust Non-Friendly Mission Assignment

**Rust surface:** `src/sim/gate_runtime.rs:133..174`.  
**Rust value:** `request_gate_open_for_cell` scans the same selected occupancy layer, filters structures, resolves object rules, then requires both `obj.gate == true` and `are_houses_friendly(alliances, mover_owner, gate_owner) == true` before calling `request_open`.  
**Enemy case:** different house with no alliance -> `are_houses_friendly == false`; `request_open` calls = `0`; gate runtime remains unchanged.  
**Neutral case:** neutral/non-allied owner -> same false branch unless explicitly allied in the map alliance graph; `request_open` calls = `0`; gate runtime remains unchanged.  
**gamemd comparison:** matches Stage 2 for the mission-assignment/no-open question: non-friendly contact does not call the opener.  
**Verdict:** PASS.

### Stage 5 - Blocked Cell-Entry Outcome

**Rust surface:** `src/sim/movement/movement_occupancy.rs:493..505` requests opening before classification, then `src/sim/pathfinding/cell_entry.rs:561..584` classifies the remaining blocker. `src/sim/pathfinding/cell_entry.rs:625..629` maps non-friendly blockers to `CellEntryResult::OccupiedEnemy`, whose YR code is `5` at `src/sim/pathfinding/cell_entry.rs:73..76`.  
**Rust value for closed enemy/neutral gate:** no skip-map entry, blocker remains in object list, non-friendly structure -> `OccupiedEnemy`, YR code `5`, blocked.  
**gamemd evidence:** `BuildingClass__CanGarrison @ 0x004525F0` returns false for `Gate=yes` unless mission `0x18` and helper `0x004A51B0` stable-open are true. `GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md:81..82` says enemy gate path calls `CanGarrison` and is passable only if already open.  
**Exact numeric caveat:** For this slot's generic "unit" scenario, I confirmed the blocked/passable boolean and no-open mission behavior. I did not recompute every downstream native `Can_Enter_Cell` result-code variant for all mover classes from live decompile in this run.  
**Verdict:** UNCHECKED for exact native result-code equality; no-open and blocked outcome match.

## Adjacent Findings

- `INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md` verifies a narrower infantry-specific result mapping: failed allied gate -> code `3`; failed enemy gate with action capability -> code `5`; failed enemy gate without action capability -> code `7`. This trace did not expand infantry action-capability semantics because the slot was limited to enemy/neutral no-open mission assignment.
- An already-open enemy gate is a different scenario. Native `CanGarrison` can return true when mission `0x18` and stable-open helper bytes match, so this trace does not claim that an enemy is blocked by an already-open gate.

## Sources

- Read-only Ghidra spot-checks: `MapClass__Check_Crushable_Obstacle @ 0x00578AD0`, `FUN_00452540 @ 0x00452540`, `BuildingClass__CanGarrison @ 0x004525F0`.
- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md`
- `docs/research/INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/GATE_MECHANIC_BUILDING_GATE_PASSABILITY_GHIDRA_REPORT.md` used only as contextual prior; newer mission `0x18` docs supersede its older mission-label wording.
- Rust surfaces: `src/sim/gate_runtime.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/rules/object_type.rs`, `src/map/houses.rs`.
