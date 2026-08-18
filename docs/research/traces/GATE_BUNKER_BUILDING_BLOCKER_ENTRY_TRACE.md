# Gate/Bunker Building Blocker Entry Trace

**Scenario:** A ground vehicle or infantry attempts to enter cells occupied by an opening/open gate building, an occupied bunker, and an empty bunker/unit-repair style building.

**Scope:** Runtime cell-entry ordering for `UnitClass::Can_Enter_Cell` / `InfantryClass::Can_Enter_Cell` object-list building blockers only. Deployment UI, bunker enter/eject lifecycle, and full final return-code matrix are out of scope.

**Status:** PARTIAL. The gate and UnitRepair/Bunker binary branches are verified, and Rust predicates were inspected. Full live PASS is blocked by missing gate-state passability in Rust and by unproven object-list/final-classifier equality.

## Evidence Used

- Ghidra read-only spot checks:
  - `UnitClass::Can_Enter_Cell @ 0x0073F0A0`: active YR vehicle runtime entry evaluator; scans selected cell object list head-to-tail through object `+0x30`; contains gate helper branch and UnitRepair/Bunker row-helper branch.
  - `FUN_00458A00 @ 0x00458A00`: active helper used by the UnitRepair/Bunker branch; returns false to skip the current building occupant, true to keep it in normal blocker logic.
  - `BuildingClass::CanGarrison @ 0x004525F0`: active helper name is misleading; for `BuildingType+0x16B7` gate/damaged-door style buildings it returns true only when building mission is `0x18` and the open-animation helper returns true.
  - `InfantryClass::Can_Enter_Cell @ 0x0051BF90`: active YR infantry entry evaluator; has the gate helper branch but not the vehicle UnitRepair/Bunker row-helper branch.
- Research docs:
  - `docs/research/GATE_MECHANIC_BUILDING_GATE_PASSABILITY_GHIDRA_REPORT.md`
  - `docs/research/UNITREPAIR_BUNKER_NUMBER_IMPASSABLE_ROWS_SECOND_CALLSITE_GHIDRA_REPORT.md`
  - `docs/research/NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH_GHIDRA_REPORT.md`
- Stock INI/art:
  - `ini/rulesmd.ini:17186..17211`: `[GAGATE_A]`, `Gate=yes`, `GateCloseDelay=.2`.
  - `ini/artmd.ini:4204..4212`: `[GAGATE_A]`, `Foundation=3x1`, `GateStages=9`.
  - `ini/rulesmd.ini:13722..13751`: `[NATBNK]`, `Bunker=yes`, `NumberOfDocks=1`, `NumberImpassableRows=0`.
  - `ini/artmd.ini:5019..5022`: `[NATBNK]`, `Foundation=2x2`.
  - `ini/rulesmd.ini:11895,11913`: `[GADEPT]`, `UnitRepair=yes`, `NumberImpassableRows=1`.
  - `ini/artmd.ini:3838..3842`: `[GADEPT]`, `Foundation=3x3`.

## Pipeline

1. Trigger: mover probes candidate cell during runtime movement/pathing.
2. Data: selected cell object-list layer, building type flags, candidate cell X, building origin X, and bunker contained-unit pointer.
3. Binary runtime logic:
   - Vehicle: `UnitClass::Can_Enter_Cell`.
   - Infantry: `InfantryClass::Can_Enter_Cell`.
4. Building object-list branch:
   - Gate/opening gate: call `BuildingClass::CanGarrison` only for the gate/damaged-door branch.
   - UnitRepair/Bunker vehicle branch: call helper `0x00458A00` only for vehicle `UnitClass::Can_Enter_Cell`.
5. Result: either skip current building occupant and continue scanning later cell occupants, or keep it for normal ownership/crush/weapon/building blocker logic.

## Stage Verdicts

| Stage | Concrete check | gamemd output | Rust output | Verdict |
|---|---|---:|---:|---|
| Gate data activation | Stock `GAGATE_A` has `Gate=yes`, art `GateStages=9`; active gate helper reads a type byte and mission/open animation state | Gate building can be skipped only when open mission/open animation predicate succeeds | No `GateStages`, `Gate`, or `DamagedDoor` parser/runtime cell-entry state found under `src/rules` or `src/sim`; no gate-specific passability branch in `movement_occupancy.rs` / `cell_entry.rs` | NOT-IMPLEMENTED |
| Open gate runtime entry | Open gate object in cell list | building helper true; occupant contributes no block and loop continues | Structure occupancy remains ordinary blocker unless some external code removes it; no matching open-gate evaluator found | NOT-IMPLEMENTED |
| Closed gate runtime entry | Closed gate object in cell list | helper false; allied yields building scatter/block style code, enemy can become code 5/7 depending downstream checks | Normal structure blocker classification exists, but not through the verified gate helper/order | UNCHECKED |
| Vehicle UnitRepair west column | `GADEPT` origin `ox=10`, `NumberImpassableRows=1`, candidate `x=10` | helper true because `10 < 10 + 1`; keep building | `decide_live_vehicle_building_entry`: `candidate_x >= origin_x + rows` is false; keep building | PASS |
| Vehicle UnitRepair east columns | `GADEPT` origin `ox=10`, candidates `x=11,12` | helper false because `11/12 < 11` is false; skip current building occupant | Rust helper skips because `candidate_x >= 11` | PASS |
| Empty vehicle bunker | `NATBNK` origin `ox=10`, `rows=0`, `bunker_occupied=false`, candidates `x=10,11` | helper false because `x < 10 + 0` is false; skip current building occupant | Rust helper skips for `x >= 10` when not occupied | PASS |
| Occupied vehicle bunker predicate | `NATBNK`, `bunker_occupied=true`, candidates `x=10,11` | helper true before row math through `BuildingClass+0x2E4`; keep building | Rust helper keeps when `bunker_occupied=true` | PASS |
| Occupied vehicle bunker live state | Standard bunker becomes occupied through binary writer to `BuildingClass+0x2E4` | contained-unit pointer nonzero after install state | `GameEntity::bunker_occupant` exists, but no setter found; `movement_occupancy.rs` also treats generic passenger cargo count as bunker-occupied, which is not verified as `+0x2E4` equality | NOT-IMPLEMENTED |
| Vehicle object-list skip integration | Helper false skips only current building occupant, then continues the live cell object list | one occupant skipped; later occupants still scanned | `build_live_vehicle_building_entry_skip_map` precomputes per-cell building ids and assumes `candidate_building_id=Some(building.stable_id)` for each foundation cell; final classifier does not consume the skip map | FAIL |
| Infantry UnitRepair/Bunker branch absence | Infantry attempts same empty bunker/depot cells | no vehicle UnitRepair/Bunker helper branch in `InfantryClass::Can_Enter_Cell`; building remains subject to normal blocker logic | `build_live_vehicle_building_entry_skip_map` returns empty unless mover category is `Unit`; infantry deferred check does not use the vehicle skip map | PASS |
| Infantry gate branch | Infantry attempts open/closed gate cell | same gate helper shape appears in InfantryClass, with open gate skipped and closed gate blocking | no gate-state branch found in Rust infantry occupancy path | NOT-IMPLEMENTED |
| Final building blocker return codes | After helper true for gate-closed/occupied-bunker/depot-west-column | binary continues through ownership, weapon/crush, BridgeRepairHut, scatter/block code ordering | `classify_occupied_cell_with_layers` uses crush-first then first primary blocker friendship approximation; exact numeric equality for codes 3/5/6/7 not proven | UNCHECKED |

## Findings

1. **NOT-IMPLEMENTED - open gate passability.** In gamemd, an open gate is passable because the gate building branch calls `BuildingClass::CanGarrison` and skips the building occupant when mission/open-animation state says open. Rust has no matching gate parser/runtime branch in `src/sim/movement/movement_occupancy.rs` or `src/sim/pathfinding/cell_entry.rs`.

2. **FAIL - vehicle skip integration is not fully binary-shaped.** Rust now has the correct UnitRepair/Bunker helper predicate in `src/sim/pathfinding/cell_entry.rs:321`, and `src/sim/movement/movement_occupancy.rs:262` precomputes skip ids. But gamemd decides this while scanning the live selected cell object list; Rust precomputes `candidate_building_id=Some(building.stable_id)` for every foundation cell and the final classifier at `src/sim/pathfinding/cell_entry.rs:485` does not receive the skip map.

3. **NOT-IMPLEMENTED - occupied bunker live writer.** Gamemd occupied-bunker behavior depends on `BuildingClass+0x2E4 != 0`. Rust has `GameEntity::bunker_occupant`, but no assignment to `Some` was found. The current cargo-count fallback in `src/sim/movement/movement_occupancy.rs:291` is not verified as literal equality with gamemd `+0x2E4`.

4. **UNCHECKED - closed gate and kept-building return codes.** Gamemd does not stop at "building blocks"; after helper false/true it continues through ownership, weapon, crush, BridgeRepairHut, and scatter logic. Rust's `classify_blocker` at `src/sim/pathfinding/cell_entry.rs:577` is still a broad friendship/stationary classifier. Numeric equality for final codes is not proven.

5. **PASS only at the isolated vehicle helper predicate.** For the concrete stock values:
   - `GADEPT rows=1`: west column keeps, east columns skip.
   - empty `NATBNK rows=0`: both 2x2 columns skip.
   - occupied `NATBNK`: both columns keep if Rust is given `bunker_occupied=true`.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 4

## Adjacent Findings

- `src/rules/object_type.rs:709` still describes `NumberImpassableRows` as top/Y-axis rows, but the verified helper compares X from game west.
- Static path-grid blocking still exists for building foundations; this trace did not audit whether every pathfinding callsite now reaches the runtime-shaped skip decision before rejecting a depot/bunker candidate.
