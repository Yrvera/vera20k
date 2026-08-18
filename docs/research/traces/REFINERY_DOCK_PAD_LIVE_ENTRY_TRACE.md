# Refinery Dock Pad Live Entry Trace

**Scenario:** A harvester with live refinery radio contact approaches a stock `GAREFN`/`NAREFN`.

**Scope:** `UnitClass::Can_Enter_Cell` behavior for refinery-relative `(3,1)` dock pad, `(2,1)` interior, and `(4,1)` queue cell. Checks base foundation object list, `NumberImpassableRows` contact skip, `HasBib` east-edge skip, and Add/Remove hidden occupancy non-effect.

**Verdict:** PARTIAL DRIFT. Current Rust can let the contacted harvester enter the stock accepted `(3,1)` cell, but the mechanism is not one binary-shaped runtime object-list evaluator. Rust still splits the decision across static `PathGrid` building blockers, a live contact skip map, and separate occupancy/classification paths. A nearby helper still encodes stock pad as `(2,1)`.

## Evidence Summary

- Active YR binary: `UnitClass::Can_Enter_Cell @ 0x0073F0A0` scans the selected cell object list and applies live building skip branches while scanning.
- Active YR binary: `FUN_00458A00 @ 0x00458A00` returns true while `candidate_x < building_origin_x + NumberImpassableRows`; in the contact branch, the building is skipped only when this helper returns false.
- Active YR binary: `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` accepts stock `DockUnload` refineries and sends the cell `building_nw + (3,1)`.
- Active YR binary: `BuildingClass::GetDockCoord @ 0x00447B20` first branch `+0x16BC` returns `NW+(2,1)`, but `+0x16BC` is `Weeder=`, not stock `GAREFN/NAREFN`.
- Prior verified docs: `REFINERY_DOCK_PAD_CAN_ENTER_CELL_STACK_GHIDRA_REPORT.md`, `STOCK_REFINERY_ART_REMOVE_OCCUPY_PAD_CELL_GHIDRA_REPORT.md`, and `BUILDINGCLASS_GETDOCKCOORD_STOCK_REFINERY_BRANCH_GHIDRA_REPORT.md`.

## Concrete Cell Matrix

Assume stock refinery NW origin `(10,10)`.

| Cell | gamemd active YR result | Current Rust result | Verdict |
|---|---|---|---|
| `(13,11)` = `NW+(3,1)` | Building object is present in base foundation, but live contact `NumberImpassableRows=3` skips it; `HasBib` east-edge also skips it. Net building blocker: clear. | `PathGrid` statically unblocks east edge via `building_movement_blocking_cells`; occupancy still contains the structure, then `build_live_vehicle_building_entry_skip_map` ignores the contacted building for x >= 13. Net transition can proceed. | PASS for this stock contacted output; FAIL for mechanism shape/order. |
| `(12,11)` = `NW+(2,1)` | Building object is present; contact helper returns true because `12 < 10 + 3`; `HasBib` east neighbor `(13,11)` is the same building. Net building blocker remains. | `PathGrid` statically blocks this cell before a live object-list building branch can make the decision. Net movement blocked, but not by the binary object-list path. | FAIL for mechanism shape/order; output legality matches only at the blocked/not-blocked level. |
| `(14,11)` = `NW+(4,1)` | Outside `Foundation=4x3`; no refinery object-list blocker. This is `QueueingCell=4,1`, not the physical pad. | `refinery_queue_cell` uses art `QueueingCell`; no foundation occupancy from the refinery at `(14,11)`. | PASS for stock queue-vs-pad separation. |

## Stage Verdicts

| Stage | Finding | Verdict |
|---|---|---|
| Stock INI data | `GAREFN/NAREFN` use `Foundation=4x3`, `QueueingCell=4,1`, `Bib=yes`, `NumberImpassableRows=3`, `DockUnload=yes`, `Refinery=yes`, and no stock `Weeder=yes`. | PASS |
| Accepted dock cell | gamemd `Receive_Radio(0x0E)` sends `NW+(3,1)`; Rust `refinery_can_dock_queue_cell` returns `(rx+3, ry+1)`. | PASS |
| Base foundation object list | gamemd keeps the refinery object in all twelve `4x3` cells; Rust structure spawn inserts base foundation cells into occupancy. | PASS |
| Add/Remove passability non-effect | gamemd `AddOccupy/RemoveOccupy` affect hidden occupancy, not real `Cell+0xE4`; Rust `PathGrid::block_building_footprint` ignores add/remove for movement. | PASS |
| Contact skip boundary | gamemd first clear x is `origin_x + NumberImpassableRows = 13`; Rust `decide_live_vehicle_building_entry` also skips only `candidate_x >= 13`. | PASS |
| `HasBib` east-edge branch | gamemd applies this during live object-list scan by probing east neighbor; Rust pre-erases east-edge movement blockers in `PathGrid`. | FAIL |
| `(3,1)` contacted entry | Final stock contacted outcome is clear in both, but Rust reaches it through static grid plus skip map, not one binary-shaped scan. | PASS/FAIL split |
| `(2,1)` contacted entry | gamemd reaches building blocker through live object-list scan; Rust blocks earlier through static grid. | FAIL |
| `refinery_pad_cell` helper | gamemd stock passable/accepted pad is `NW+(3,1)`; Rust fallback returns `NW+(2,1)`. Current live mission-enter ignores `_pad`, but the helper and tests are stale. | FAIL |
| Hidden occupancy visual counters | Rust has hidden occupancy helper separation, but this trace did not compute visual `Cell+0x100` reader outputs against gamemd. | UNCHECKED |
| Exact return codes/timing for all ownership/mission variants | This slot traced same-owner live refinery contact only; other ownership/action branches were not replayed. | UNCHECKED |

## Rust Touchpoints

- `src/sim/movement/movement_occupancy.rs:262` builds `LiveBuildingEntrySkipMap` from all base foundation cells.
- `src/sim/movement/movement_occupancy.rs:302` passes `candidate_x`, `building_origin_x`, `number_impassable_rows`, and contact state to the live row helper.
- `src/sim/movement/movement_occupancy.rs:218` skips deferred occupancy entirely when `bypass_grid` is set; this would be too broad if used for refinery interior traversal.
- `src/sim/pathfinding/cell_entry.rs:321` implements the `NumberImpassableRows` skip boundary.
- `src/sim/pathfinding/core.rs:1936` applies `HasBib` as static movement-grid blocking removal.
- `src/sim/production/production_tech.rs:719` implements static east-edge movement blockers for bib buildings.
- `src/sim/miner/miner_dock_sequence.rs:187` returns accepted stock cell `NW+(3,1)`.
- `src/sim/miner/miner_dock_sequence.rs:199` returns fallback pad cell `NW+(2,1)`, which is not stock `GAREFN/NAREFN` pad evidence.
- `src/sim/miner/miner_dock_sequence.rs:848` uses `accepted_cell`, not `_pad`, for the current live mission-enter arrival gate.

## Failures

1. **Runtime evaluator still is not one binary-shaped object-list scan.** `HasBib` is represented as static `PathGrid` removal, while contact skip is represented as a per-mover occupancy ignore map. The stock contacted `(3,1)` result matches, but the mechanism/order does not.

2. **Interior `(2,1)` is blocked by the wrong stage.** gamemd scans the building object and keeps it as a blocker after contact and bib checks. Rust can reject the cell in the static grid before the live building branch runs.

3. **`refinery_pad_cell` still names the stock pad as `(2,1)`.** Existing binary evidence ties stock accepted/passable refinery entry to `(3,1)`; `(2,1)` is the `Weeder=` `GetDockCoord` branch, not stock `GAREFN/NAREFN`.

## Adjacent Findings

- `movement_tick.rs:887` drive-track lookahead still requires selected occupancy layers to be empty and does not apply the live building skip map. This can matter for chained turns into a structure-occupied but live-skippable refinery edge cell, but it belongs to the drive-track slot rather than this trace.
- `movement_occupancy.rs:218` `bypass_grid` is a broad structure-occupancy bypass. It is not currently used by `phase_mission_enter` after `issue_direct_move`, but if reused for refinery entry it could open `(2,1)` incorrectly.

## Verdict Tally

PASS: 6 | FAIL: 4 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Sources

- Ghidra read-only decompile: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `FUN_00458A00 @ 0x00458A00`, `BuildingClass::Receive_Radio @ 0x0043C2D0`, `BuildingClass::GetDockCoord @ 0x00447B20`.
- `docs/research/REFINERY_DOCK_PAD_CAN_ENTER_CELL_STACK_GHIDRA_REPORT.md`
- `docs/research/STOCK_REFINERY_ART_REMOVE_OCCUPY_PAD_CELL_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_GETDOCKCOORD_STOCK_REFINERY_BRANCH_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- Rust files listed in the touchpoints section.
