# Can_Enter_Cell Water / Pier Legality Re-Swarm Slot 1

**Date:** 2026-05-27  
**Slot:** 1  
**Scope:** active-YR `gamemd.exe` cell-entry legality for non-water ground movers entering water, pier, and bridge-adjacent cells.  
**Requested focus:** `UnitClass::Can_Enter_Cell`, Foot/Infantry variants where relevant, `CheckPassability`, `CheckBridgeTraversal`, `MovementZone` / `ZoneType` matrix readers.  
**Ghidra access note:** no callable Ghidra MCP endpoint was exposed in this slot. I did not run fresh `decompile`, `batch_decompile`, `analyze_dataflow`, debugger read, or trace calls. Findings marked `VERIFIED` below are verified-from-prior Ghidra-backed reports cited in this document, not fresh live decompilation by this slot. No Ghidra state, Rust code, INI files, or non-report docs were modified.

## Executive Summary

For active YR, ordinary non-water ground units must not be allowed to enter bare water cells. The verified binary model is not "PathGrid says walkable"; it is a layered cell-entry pipeline:

1. A* / zone systems use the 13x8 `ZonePassabilityMatrix` at `0x0082A594`, indexed by `TechnoTypeClass+0x5B4 MovementZone` and reduced `CellClass+0x4C ZoneType`.
2. A* neighbor expansion calls the mover's virtual `Can_Enter_Cell` at vtable `+0x1AC`; for vehicles this is `UnitClass::Can_Enter_Cell @ 0x0073F0A0`.
3. `UnitClass::Can_Enter_Cell` calls `CheckBridgeTraversal @ 0x004D9C60` through vtable `+0x1B0`, then calls `FootClass::LocomotorPassabilityCheck @ 0x004D9C10`, and later performs a `SpeedType x LandType` zero-speed rejection on the ground path.
4. High bridge deck traversal is a bridge-layer exception, not water traversal. Low bridge / pier-like traversal is tube-backed in the verified low-bridge docs: valid `CellClass+0x116` tube index plus final `LandType == 10`, not "water made walkable".

Current Rust still has broad surfaces where non-water movers can use `PathGrid::is_walkable()` / `is_any_layer_walkable()` instead of a binary-shaped `Can_Enter_Cell` equivalent. Because `PathGrid::from_resolved_terrain_with_bridges` marks water `ground_walkable`, that is a real DRIFT risk for the visible "unit drives on water next to pier" symptom.

## Binary-Verified Findings

### Vtable and core entry points

| Finding | Status | Evidence | Active in YR |
|---|---|---|---|
| Vehicle A* cell-entry dispatch is vtable `+0x1AC`, `UnitClass::Can_Enter_Cell @ 0x0073F0A0`. | VERIFIED from prior Ghidra | `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`; `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`; A* call report | Yes |
| `UnitClass` / `InfantryClass` vtable `+0x1B0` is `CheckBridgeTraversal @ 0x004D9C60`, not the A* entry itself. | VERIFIED from prior Ghidra | vtable reads in `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` | Yes |
| `InfantryClass` has its own `Can_Enter_Cell @ 0x0051BF90`, but shares `CheckBridgeTraversal @ 0x004D9C60`. | VERIFIED from prior Ghidra | same hierarchy report | Yes |
| A* main loop calls `Can_Enter_Cell` per neighbor and rejects codes `>= 7`; codes `0..6` enter cost computation. | VERIFIED from prior Ghidra | `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`, `AStar_main_loop @ 0x00429A90` | Yes |

### Water legality for non-water ground movement

| Finding | Status | Evidence | Active in YR |
|---|---|---|---|
| Reduced `CellClass+0x4C` water column is value `4`, written when final `CellClass+0x48 LandType == 2`. | VERIFIED from prior Ghidra | `CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md`, branch `0x00483D2A..0x00483D36` | Yes |
| `ZonePassabilityMatrix @ 0x0082A594` is `int[13][8]`; rows are `MovementZone`, columns are reduced `ZoneType`, only value `1` passes. | VERIFIED from prior Ghidra | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` | Yes |
| Non-water ground rows block water column 4: Normal row 0 has `2` at col 4, Crusher row 1 has `2`, Infantry row 7 has `2`, etc. | VERIFIED from prior Ghidra | matrix dump in `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` | Yes |
| Amphibious-family rows are the intended water-capable ground exception; ordinary Normal / Crusher / Infantry are not. | VERIFIED from prior Ghidra | `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`; matrix rows 3/4/5 | Yes |
| `UnitClass::Can_Enter_Cell` performs terrain/locomotor legality before occupant soft-code handling. It calls `FootClass::LocomotorPassabilityCheck @ 0x004D9C10`; later, if not bridge-layer, it rejects `g_SpeedType_LandType_Table[cell.LandType * 9 + TechnoType.SpeedType] == 0.0`. | VERIFIED from prior Ghidra | `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` phases 8 and 11; `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` | Yes |

Interpretation: for an ordinary non-amphibious land vehicle, bare water has both zone-connectivity rejection through `MovementZone x ZoneType` and cell-entry rejection through `Can_Enter_Cell` / speed-land terrain legality. A local boolean pathgrid must not override either.

### Bridge-adjacent and bridge-layer legality

| Finding | Status | Evidence | Active in YR |
|---|---|---|---|
| `CheckBridgeTraversal @ 0x004D9C60` receives candidate cell, direction, `*path_height`, `*bridge_entered`, and optional parent/current cell. | VERIFIED from prior Ghidra | `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` | Yes |
| Legal height deltas are exactly diff `0`, `1` with slope gate, and `4` with bridge/bridgehead gates. Other deltas return `7`. | VERIFIED from prior Ghidra | same report | Yes |
| Bridge entry from low to high requires candidate bridge and bridgehead flags; the `bridge_entered` output is set only in the ascending diff-4 case. | VERIFIED from prior Ghidra | same report | Yes |
| Runtime drive/walk/ship calls pass `parent/current-cell = 0` and current effective height, causing `CheckBridgeTraversal` to infer parent from target + `(direction - 4) & 7`. A* passes an explicit current-node cell. | VERIFIED from prior Ghidra | `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`; `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md` | Yes |
| The bridge `Can_Enter_Cell` two-pass split is real: object-list layer can be selected before `CheckBridgeTraversal`, while occupancy bits can be reselected afterward from `cell+0x128` when final height equals `cell.Level + 4`. | VERIFIED from prior Ghidra | `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`; `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` | Yes |

Interpretation: a high bridge over water is passable to ground units only through the verified bridge layer / bridgehead transition logic. It is not equivalent to water being ground-walkable.

### Low bridge / pier-like caveat

| Finding | Status | Evidence | Active in YR |
|---|---|---|---|
| Low bridge pathing is tube-backed: `CellClass::IsLowBridgeCell @ 0x00484AB0` requires a valid `CellClass+0x116` tube index and final `LandType == 10`. | VERIFIED from prior Ghidra | `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` | Yes |
| Low bridge overlay `Land=Road` is not sufficient movement truth. | VERIFIED from prior Ghidra | same report; INI data cited there | Yes |
| Direction `8` is the tube jump sentinel in coordinate/path walking. | VERIFIED from prior Ghidra | `MapCoord_Step_By_Direction @ 0x0042D490`; `Path_walk_directions_to_cell @ 0x00429780` in low-bridge report | Yes |
| Exact "pier" visual-tile classification is not fully proven by this slot. If the visible bug involves map tiles the player calls "pier", slot 4 should classify those tiles as ordinary water, high bridge/waterbridge, low bridge/tube, beach, or some separate theater tile family. | UNCHECKED in this slot | no fresh Ghidra and no dedicated pier classification report found | Conditional |

## Current Rust Touchpoints

Read-only source scan found these relevant surfaces:

| Rust surface | Current behavior / risk |
|---|---|
| `src/sim/pathfinding/core.rs` `PathGrid::from_resolved_terrain_with_bridges` | Sets `ground_walkable` to true for `cell.is_water` when not blocked by overlay/terrain object. The comment says `TerrainCostGrid` should block ground units. This is not a safe replacement for binary `Can_Enter_Cell` at every call site. |
| `src/sim/pathfinding/core.rs` `is_cell_passable_for_mover` | Water movers use resolved terrain / passability matrix; non-water movers fall back to `grid.is_walkable(x, y)`. Because the grid marks water walkable, non-water callers that do not also apply cost/matrix can accept water. |
| `src/sim/movement/movement_path.rs` `is_move_goal_walkable` | For non-water movers, uses `grid.is_any_layer_walkable(goal)` rather than mover-specific `Can_Enter_Cell` / matrix legality. |
| `src/sim/movement/movement_path.rs` `nearest_move_goal` | For non-water movers, redirects to `nearest_walkable_any_layer`, again using pathgrid layer booleans rather than mover legality. |
| `src/sim/movement/movement_path.rs` smoothing closures | Layered smoothing checks `grid.is_walkable_on_layer`; flat smoothing checks `grid.is_walkable` for non-water movers. This can revalidate shortcuts through cells A* avoided if the shortcut predicate is weaker than binary `Can_Enter_Cell`. |
| `src/sim/pathfinding/cell_entry.rs` | Has a `CanEnterLayerContext` and code/result vocabulary, but its header still describes bridge legality as driven by path layers and notes pending terrain edge cases. It is not yet a full binary-shaped `(target, direction, height, parent_or_current, arg5)` evaluator. |
| `src/sim/pathfinding/zone_build.rs` | Contains the binary-shaped MovementZone x reduced-ZoneType check and is closer to verified matrix semantics, but other movement goal/pathgrid surfaces bypass this. |

## DRIFT / UNCHECKED Findings

| ID | Verdict | Finding | Player-visible risk |
|---|---|---|---|
| S1-D1 | DRIFT | `PathGrid` marks water cells `ground_walkable` while binary non-water ground entry must reject water through matrix and `Can_Enter_Cell` terrain checks. | Ordinary land units can be redirected or smoothed onto water/pier-adjacent cells. |
| S1-D2 | DRIFT | Non-water goal validation and nearby-goal redirection use `is_any_layer_walkable` / `nearest_walkable_any_layer`, not mover-specific cell-entry legality. | A click or helper move near pier/water can choose a visually invalid cell. |
| S1-D3 | DRIFT | Non-water `is_cell_passable_for_mover` falls through to `grid.is_walkable`, so any caller relying on it without a cost grid / reduced-zone matrix can accept water. | Shared, not Chrono Miner-specific; affects any ground mover call path using the helper. |
| S1-D4 | DRIFT | Path smoothing uses boolean layer walkability and does not reconstruct binary `Can_Enter_Cell(target, direction, height, parent/current, arg5)`. | A valid A* path can be postprocessed into a shortcut over water or the wrong bridge layer. |
| S1-D5 | DRIFT | Current Rust has partial bridge layer context, but runtime `Can_Enter_Cell` requires current effective height, nullable parent fallback, and the two-pass list/occupancy split. | Bridgehead / waterbridge / pier-adjacent movement can diverge even when ordinary terrain looks correct. |
| S1-U1 | UNCHECKED | Exact player-reported "pier" tile family is not classified in this slot. | If the bad maps use special waterbridge/pier theater tiles rather than ordinary water, the final fix may belong partly in resolved terrain classification. |
| S1-U2 | UNCHECKED | Full `InfantryClass::Can_Enter_Cell @ 0x0051BF90` water-specific terrain branch was not freshly decompiled here. Vtable identity and shared `CheckBridgeTraversal` are verified; infantry-specific blockers are adjacent. | Infantry may need a parallel exact evaluator, but the same water/matrix principle is already strongly supported. |

## Implementation Handoff

Do not fix this by adding one more special case to miner logic. The verified problem surface is shared cell-entry legality.

Required future Rust effects:

1. Introduce or complete a binary-shaped cell-entry evaluator for ground movers with this argument shape:

```text
Can_Enter_Cell(target_cell, direction, height, parent_or_current_cell, arg5)
```

2. Use that evaluator, or a proven equivalent adapter, for:
   - A* neighbor legality and cost code production;
   - move-goal validation and nearest-goal redirection;
   - path smoothing / straight-line shortcut validation;
   - runtime movement step collision/entry checks.

3. Keep MovementZone and SpeedType roles separate:
   - `MovementZone` + reduced `ZoneType` drives zone connectivity / matrix row behavior.
   - `SpeedType` + raw/final `LandType` drives locomotor terrain speed and zero-speed rejection.

4. Treat high bridge deck movement as bridge-layer traversal through `CheckBridgeTraversal` semantics, not as water/ground walkability.

5. Treat low bridge traversal as tube-backed where applicable: valid tube index plus `LandType == 10`, with direction-8 tube stepping. Do not model it as low bridge overlay `Land=Road` alone.

Acceptance scenarios to add after parent reconciliation:

| Scenario | Expected active-YR behavior |
|---|---|
| Normal tank ordered to bare water cell beside a pier | target rejected or redirected only to a legal land/bridge/tube cell; never drives on bare water |
| Chrono Miner returning near refinery/pier with adjacent water | staging/goal helper cannot select water for a non-amphibious miner |
| Amphibious hover unit on beach/water transition | allowed only according to its MovementZone row and SpeedType table; ordinary Normal units still blocked |
| Ground unit entering intact high bridge over water | allowed only through bridgehead/deck semantics, with correct height/layer handling |
| Unit near low bridge/waterbridge/pier-like low tile | legality follows tube/bridge classification, not `PathGrid::is_walkable` |

## Open Questions

1. Fresh live Ghidra spot-check of `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `FootClass::LocomotorPassabilityCheck @ 0x004D9C10`, and `CheckBridgeTraversal @ 0x004D9C60` should be rerun when Ghidra MCP is available, because this slot relied on prior reports.
2. The exact visual "pier" map tiles need classification. Determine whether they are ordinary water, WaterBridge LAT, high bridge, low bridge/tube, shore/beach, or a special theater tile range.
3. Full `InfantryClass::Can_Enter_Cell @ 0x0051BF90` should be audited if the final implementation shares one evaluator across vehicles and infantry.
4. Slot 2 / A* should confirm that all current Rust A* call paths supply enough direction, height, parent cell, bridge flag, and code-cost context to replace boolean walkability.
5. Slot 3 / smoothing should confirm exact gamemd shortcut validator call shape before the Rust smoothing predicate is changed.

## Shared Claims For Parent Reconciliation

- `slot-1`: `UnitClass::Can_Enter_Cell @ 0x0073F0A0` is the active vehicle A* entry at vtable `+0x1AC`; `CheckBridgeTraversal @ 0x004D9C60` is the bridge sub-check at vtable `+0x1B0`.
- `slot-1`: For non-water ground movers, bare water is blocked by both the MovementZone/ZoneType matrix path and the `Can_Enter_Cell` terrain/speed legality path. `PathGrid::ground_walkable == true` for water is not a valid cell-entry verdict.
- `slot-1`: High bridge over water is a bridge-layer legality problem; low bridge/pier-like low traversal is tube-backed (`Cell+0x116` valid and `LandType == 10`) where the low-bridge docs apply.
- `slot-1`: Current Rust drift surfaces are shared movement/pathfinding helpers, especially `PathGrid::is_walkable`, `is_any_layer_walkable`, nearest-goal redirection, and smoothing. This is not miner-only.
- `slot-1`: Exact pier tile classification remains `UNCHECKED` and should be owned by the classification slot before choosing the final patch boundary.

## Sources Read

- `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
- `docs/research/ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
- `docs/research/ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md`
- `docs/research/CELLRECT_CHECKPASSABILITY_0056E7C0_FULL_ARG_DECODE_GHIDRA_REPORT.md`
- `docs/research/NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`
- `docs/research/WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- Rust touchpoints scanned read-only: `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_path.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/pathfinding/zone_build.rs`, `src/map/resolved_terrain.rs`.
