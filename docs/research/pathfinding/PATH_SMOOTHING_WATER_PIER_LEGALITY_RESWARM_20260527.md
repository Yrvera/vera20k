# Path Smoothing Water/Pier Legality Reswarm - 2026-05-27

Swarm slot: 3
Target: active-YR path smoothing / shortcut / path postprocess legality around water, pier, bridge, and tube cells.

## Working Notes

- Target question: Does gamemd path smoothing validate shortcut/reroute cells through live unit cell-entry legality, or does it only trust the already-computed path/grid abstraction?
- Non-goals: Full decode of `UnitClass__Can_Enter_Cell`, full bridge/pier tile classification, A* neighbor expansion, zone precheck, and Rust implementation.
- Evidence needed to mark COMPLETE: active call chain from A* into smoothing, direct smoothing-time candidate-cell validation evidence, whether tube/bridge direction 8 is smoothed, and Rust callsites that currently use `PathGrid`-style closures.
- Stop conditions: Ghidra read-only access unavailable, function boundary missing, or evidence only from existing docs without fresh binary spot-check.

## Summary

Active YR smoothing does **not** use a precomputed path grid abstraction as its final shortcut legality check. The smoothing/postprocess pipeline revalidates candidate shortcut cells by looking up live `CellClass` objects and calling the mover's virtual `Can_Enter_Cell`-style function at vtable offset `+0x1AC`.

For standard ground units, `vtable__UnitClass + 0x1AC` resolves to `UnitClass__Can_Enter_Cell @ 0x0073F0A0` via the UnitClass vtable slot at `0x007F5E1C`. Therefore smoothing shortcuts are gated by the same dynamic unit/cell legality surface used by A* neighbor expansion, including terrain, height/bridge, tube, overlay, speed-table, locomotor, and occupant effects inside `Can_Enter_Cell`.

Current Rust smoothing is structurally different at the legality boundary. `movement_path.rs` passes closures that call `PathGrid::is_walkable`, `PathGrid::is_walkable_on_layer`, and only special-case water movers through `is_cell_passable_for_mover`. `path_smooth.rs` then asks only those closures whether the candidate cells are walkable. This is a DRIFT for non-water ground units near water/pier cells because Rust can allow a shortcut through any cell the broad `PathGrid` says is ground-walkable, while gamemd rechecks the concrete mover's `Can_Enter_Cell`.

## VERIFIED Binary Findings

### Active YR call chain

`AStar_main_loop @ 0x00429A90` calls:

- `AStar_reconstruct_path`
- `Path_smooth_corners @ 0x0042B210`
- `Path_optimize_straight_segments @ 0x0042B7F0`

Evidence:

- Ghidra xrefs: `Path_smooth_corners` is called from `0x0042A415` in `AStar_main_loop`.
- Ghidra xrefs: `Path_optimize_straight_segments` is called from `0x0042A41E` in `AStar_main_loop`.
- Fresh decompile of `AStar_main_loop` shows the two calls immediately after path reconstruction and before returning the path.

Active in YR: Yes. No TS-only or option gate was found in this call chain.

### Pass 1: corner smoothing revalidates shortcut cells

`Path_smooth_corners @ 0x0042B210` detects eligible zigzag runs and delegates each smoothing attempt to `Path_smooth_single_segment @ 0x0042B420`.

`Path_smooth_single_segment` validates the candidate shortcut cells by:

- stepping through candidate cells with direction offsets,
- calling `MapClass__Get_CellClass @ 0x005657A0`,
- calling the mover virtual at `(*this->vtable + 0x1AC)` with `(CellClass*, direction, height, 0, 1)`,
- rejecting if the virtual returns nonzero,
- rejecting if `CellClass+0x140 & 0x40000` is set,
- rejecting if the slope cost times the unit slope factor crosses the threshold.

Evidence:

- Decompile `0x0042B420`: virtual call expression `(**(code **)(*param_1 + 0x1ac))(iVar4, local_2c, iVar8, 0, 1)`.
- Assembly `0x0042B59D..0x0042B5B4`: pushes `1`, `0`, height, direction, `CellClass*`, then `CALL dword ptr [EAX + 0x1AC]`.
- Assembly `0x0042B5B4..0x0042B5F5`: nonzero return, `CellClass+0x140 & 0x40000`, or steep slope failure sets the abort flag.

Active in YR: Yes, via `AStar_main_loop`.

### Pass 2: straight-segment reroute revalidates shortcut cells

`Path_optimize_straight_segments @ 0x0042B7F0` calls `Path_Reroute_Straight_Line @ 0x0042BE20` for candidate reroutes.

`Path_Reroute_Straight_Line` validates every candidate cell in the replacement sequence by:

- trying one direction ordering, then retrying once with diagonal/cardinal order swapped on failure,
- calling `MapClass__Get_CellClass`,
- optionally counting steep cells using `MapClass__Get_Slope_Cost_At_Cell`,
- calling `(*this->vtable + 0x1AC)` with `(CellClass*, direction, height, 0, 1)`,
- rejecting if the virtual returns nonzero,
- rejecting if `CellClass+0x140 & 0x40000` is set,
- rejecting if steep-cell count exceeds the strict/lenient threshold.

Evidence:

- Xrefs: `Path_Reroute_Straight_Line` is called from `0x0042BA96` and `0x0042BC2E` in `Path_optimize_straight_segments`.
- Decompile `0x0042BE20`: virtual calls `(**(code **)(*piVar5 + 0x1ac))(iVar8, local_28, iVar1, 0, 1)` and later the same pattern for `local_24`.
- Assembly `0x0042BF62..0x0042BFB6`: candidate lookup and virtual call for the first segment direction.
- Assembly `0x0042BFB6..0x0042BFE5`: nonzero return, `CellClass+0x140 & 0x40000`, or slope threshold sets failure.
- Assembly `0x0042C061..0x0042C0BA` and `0x0042C0BA..0x0042C0E9`: same validation for the second segment direction.
- Assembly `0x0042C121..0x0042C149`: swaps direction/count ordering and retries once.

Active in YR: Yes, via `AStar_main_loop`.

### Direction 8 tube/bridge steps are not smoothed through ordinary shortcuts

`Path_smooth_corners` explicitly excludes direction `8` from zigzag detection. `Path_smooth_single_segment` also bypasses smoothing if either endpoint direction is `8`, walking the original segment instead.

Evidence:

- Decompile `0x0042B210`: zigzag branch requires both `uVar7 != 8` and `uVar2 != 8`.
- Assembly `0x0042B45A..0x0042B466`: `Path_smooth_single_segment` jumps to the walk-only path if either endpoint direction equals `8`.
- Decompile `0x0042B420`: direction `8` path calls `MapCoord_Step_By_Direction` and returns without replacing directions.

Active in YR: Yes.

### UnitClass vtable binding for standard units

The smoothing functions call the virtual slot, not a hardcoded function. For standard `UnitClass`, the slot resolves to `UnitClass__Can_Enter_Cell @ 0x0073F0A0`.

Evidence:

- Ghidra symbol: `vtable__UnitClass @ 0x007F5C70`.
- `0x007F5C70 + 0x1AC = 0x007F5E1C`.
- Ghidra xref to `UnitClass__Can_Enter_Cell @ 0x0073F0A0` from data address `0x007F5E1C`.
- Fresh decompile of `UnitClass__Can_Enter_Cell @ 0x0073F0A0` shows active terrain/height/tube/overlay/speed-table/occupant return-code logic, including final return codes up to `7`.

Active in YR: Yes for standard units using `UnitClass` movement.

## Inference From Verified Evidence

- Because smoothing calls the mover virtual at `+0x1AC`, a non-water ground unit should not be smoothed through a water/pier cell merely because that cell was present in a coarse path grid. The candidate cell must pass that unit's live `Can_Enter_Cell` logic.
- The smoothing functions themselves do not directly index the 13x8 MovementZone matrix. Zone/land legality is reached through the virtual passability function and its callees/fields. Existing sibling reports establish the broader MovementZone/ZoneType and speed-table contracts; this slot only verifies that smoothing reaches the virtual legality surface.
- The `CellClass+0x140 & 0x40000` rejection is bridge-pathfinding-specific protection that Rust smoothing currently lacks as a first-class cell flag.

## Current Rust Touchpoints

- `src/sim/movement/movement_path.rs:242`: layered smoothing closure checks `grid.is_walkable_on_layer(x, y, layer)`.
- `src/sim/movement/movement_path.rs:249`: layered smoothing rejects `entity_block_map` soft blockers.
- `src/sim/movement/movement_path.rs:256`: layered smoothing rejects layer-specific hard blocks.
- `src/sim/movement/movement_path.rs:261`: calls `path_smooth::smooth_layered_path`.
- `src/sim/movement/movement_path.rs:264`: calls `path_smooth::optimize_layered_path`.
- `src/sim/movement/movement_path.rs:304`: flat smoothing closure begins.
- `src/sim/movement/movement_path.rs:305`: water movers use `is_cell_passable_for_mover`.
- `src/sim/movement/movement_path.rs:314`: non-water movers use `grid.is_walkable(x, y)`.
- `src/sim/movement/movement_path.rs:324`: calls `path_smooth::smooth_path`.
- `src/sim/movement/movement_path.rs:325`: calls `path_smooth::optimize_path`.
- `src/sim/pathfinding/path_smooth.rs:121`: pass 1 validates only through the supplied `walkable` closure.
- `src/sim/pathfinding/path_smooth.rs:127`: Rust adds a diagonal corner-cutting check against adjacent cardinal cells.
- `src/sim/pathfinding/path_smooth.rs:206`: layered pass 1 validates only through the supplied layered closure.
- `src/sim/pathfinding/path_smooth.rs:211`: layered Rust adds the same adjacent-cardinal check.
- `src/sim/pathfinding/path_smooth.rs:472`: pass 2 reroute validates only through the supplied `walkable` closure.
- `src/sim/pathfinding/path_smooth.rs:476`: pass 2 also adds adjacent-cardinal corner-cutting checks.
- `src/sim/pathfinding/core.rs:1811`: `PathGrid` construction treats water as ground-walkable by using `!cell.ground_walk_blocked || cell.is_water`.

## DRIFT / UNCHECKED Findings

### DRIFT 1 - Rust smoothing does not call a Can_Enter_Cell-equivalent predicate

Verified gamemd behavior:

- Both smoothing passes call the mover virtual at `+0x1AC` per candidate shortcut cell.

Current Rust behavior:

- Smoothing receives a closure that reduces legality to `PathGrid::is_walkable`, `PathGrid::is_walkable_on_layer`, entity block maps, marker overlays, and a water-mover matrix helper.

Why this matters:

- For non-water units, Rust can approve a smoothing shortcut through a cell that `PathGrid` marks ground-walkable but gamemd `Can_Enter_Cell` would reject. Water/pier edge cells are a high-risk trigger because current `PathGrid` can mark water as ground-walkable.

Affected surfaces:

- `src/sim/movement/movement_path.rs`
- `src/sim/pathfinding/path_smooth.rs`
- future unified `Can_Enter_Cell`/cell-entry evaluator

### DRIFT 2 - Rust smoothing adds adjacent-cardinal corner-cutting checks not found in the verified smoothing functions

Verified gamemd behavior:

- `Path_smooth_single_segment` and `Path_Reroute_Straight_Line` validate the actual candidate cells they step into through `Can_Enter_Cell`; this slot found no extra requirement that the two cardinal flank cells adjacent to a diagonal shortcut must also be passable.

Current Rust behavior:

- `path_smooth.rs:127..143`, `211..225`, and `476..480` require flank/cardinal cells to be walkable for diagonal shortcuts/reroutes.

Why this matters:

- This can reject a gamemd-legal diagonal shortcut around an obstacle, changing path shape and timing.

### DRIFT 3 - Rust pass 2 ordering differs from gamemd

Verified gamemd behavior:

- `Path_Reroute_Straight_Line` writes all diagonal steps then all cardinal steps for the first attempt, and retries once with the order swapped if validation fails.

Current Rust behavior:

- `reroute_segment` interleaves diagonal and cardinal steps with a Bresenham-style ratio and has no exact two-ordering retry equivalent.

Why this matters:

- Around piers/bridges/water edges, the ordering decides which cells are validated and entered. A different ordering can either introduce a water shortcut or miss a gamemd shortcut.

### DRIFT 4 - Rust smoothing lacks explicit `CellClass+0x140 & 0x40000` bridge-marker rejection

Verified gamemd behavior:

- Both smoothing candidate validators reject cells with `CellClass+0x140 & 0x40000`.

Current Rust behavior:

- No direct equivalent was found in the smoothing closures or `path_smooth.rs` scan.

Why this matters:

- Bridge approach/pathfinder marker cells can be smoothed through in Rust when gamemd rejects them.

### UNCHECKED 1 - Exact pier/water branch inside `UnitClass__Can_Enter_Cell`

Verified in this slot:

- Smoothing reaches `UnitClass__Can_Enter_Cell` for UnitClass movers.

Not fully decoded in this slot:

- The exact internal branch that classifies a specific pier/waterbridge/shore tile as reject/accept for a given MovementZone and height/layer.

Why this remains open:

- This slot's scope was smoothing legality, not full cell/pier classification. Slot 4 should own bridge/pier/waterbridge/shore classification.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Smoothing validates shortcut cells through mover virtual `+0x1AC` / `UnitClass__Can_Enter_Cell`, not a boolean path grid. | Replace smoothing `walkable` closure contract with a Can_Enter_Cell-equivalent evaluator carrying mover MovementZone, SpeedType, height/layer, bridge flags, dynamic blockers, marker flags, and slope inputs. | `movement_path.rs`, `path_smooth.rs`, `cell_entry.rs`, `core.rs` | Ground unit path along a pier attempts a smoothed shortcut through an adjacent water cell; gamemd rejects because `Can_Enter_Cell` returns impassable; Rust must keep original land path. | `path_smoothing_ground_unit_rejects_water_shortcut_via_can_enter` | High player-visible risk: units drive onto water or take visibly different paths. |
| `Path_Reroute_Straight_Line` tries diagonal-then-cardinal, then cardinal-then-diagonal; every cell is rechecked. | Rework Rust pass 2 reroute to match gamemd ordering and retry instead of Bresenham interleave. | `path_smooth.rs::reroute_segment`, `path_smooth_tests.rs` | A pier-edge route where diagonal-first hits water but cardinal-first stays on pier should match gamemd's retry result. | `path_optimize_retries_swapped_order_before_rejecting_pier_edge` | Medium/high: path shape and shortcut legality diverge at edges. |
| Gamemd smoothing rejects `CellClass+0x140 & 0x40000` bridge-marker cells. | Add an equivalent bridge/path marker input to the smoothing legality evaluator; do not hide it in static `PathGrid` unless the exact writer/clearer semantics are preserved. | `resolved_terrain`, `movement_path.rs`, pathfinder bridge marker state | A bridge approach cell marked with the pathfinder flag is rejected as a smoothing shortcut even when otherwise walkable. | `path_smoothing_rejects_bridge_marker_40000_shortcut` | Medium: bridge/pier routing can cut across invalid approach cells. |

## Negative Facts / Do Not Do

- Do not treat `PathGrid::is_walkable()` as gamemd smoothing legality. Evidence: smoothing calls `vtable+0x1AC` at `0x0042B5AE`, `0x0042BFB6`, and `0x0042C0BA`.
- Do not smooth direction `8` tube/bridge transitions as ordinary adjacent cells. Evidence: `Path_smooth_corners @ 0x0042B210` excludes direction `8`; `Path_smooth_single_segment @ 0x0042B45A..0x0042B466` bypasses smoothing when an endpoint is `8`.
- Do not require adjacent cardinal flank cells for diagonal smoothing unless later binary evidence shows that check in another active function. Evidence: the verified smoothing validators check the candidate entered cells through `Can_Enter_Cell`, not extra flank cells.
- Do not interleave diagonal/cardinal reroute steps as a generic nice-looking path. Evidence: `Path_Reroute_Straight_Line @ 0x0042C15B..0x0042C1A8` writes one run then the second run, and `0x0042C121..0x0042C149` implements one swapped-order retry.
- Do not assume smoothing's legality is already guaranteed by A*. Evidence: both postprocess passes perform their own candidate-cell validation after path reconstruction.

## Shared Claims

- slot-3: Active YR path postprocess is `AStar_reconstruct_path -> Path_smooth_corners -> Path_optimize_straight_segments`.
- slot-3: Shortcut/reroute legality is live per-candidate `Can_Enter_Cell` virtual validation, not `PathGrid`/zone-grid reuse.
- slot-3: Direction `8` transitions are not smoothed as ordinary grid shortcuts.
- slot-3: Rust smoothing currently uses `PathGrid`-based closures, so the pier/water symptom can be introduced or amplified by smoothing even if A* initially avoided the cell.
- slot-3: Exact pier/tile classification remains for the bridge/pier classification slot; this slot proves the smoothing legality boundary.

## Remaining Uncertainty

- Exact internal pier/waterbridge/shore tile branch inside `UnitClass__Can_Enter_Cell @ 0x0073F0A0` was not fully decoded here.
- Exact mapping of `CellClass+0x140 & 0x40000` writer/clearer lifetime during pathfinder bridge updates was not decoded here; this slot only verifies smoothing rejects the bit.
- Aircraft and infantry class-specific `+0x1AC` overrides were not audited. The standard unit evidence is sufficient for vehicles/Chrono Miner style symptoms but not for every class-specific mover.

## Sources

- Ghidra read-only fresh decompile: `AStar_main_loop @ 0x00429A90`, `Path_smooth_corners @ 0x0042B210`, `Path_smooth_single_segment @ 0x0042B420`, `Path_optimize_straight_segments @ 0x0042B7F0`, `Path_Reroute_Straight_Line @ 0x0042BE20`, `UnitClass__Can_Enter_Cell @ 0x0073F0A0`.
- Ghidra read-only xrefs: `Path_smooth_corners` from `0x0042A415`; `Path_optimize_straight_segments` from `0x0042A41E`; `Path_smooth_single_segment` from `0x0042B382` and `0x0042B406`; `Path_Reroute_Straight_Line` from `0x0042BA96` and `0x0042BC2E`; `UnitClass__Can_Enter_Cell` data xref from `0x007F5E1C`.
- Ghidra read-only assembly: `Path_smooth_single_segment @ 0x0042B420`; `Path_Reroute_Straight_Line @ 0x0042BE20`.
- Existing docs read: `docs/research/pathfinding/fn-path_smooth_corners.md`, `fn-path_smooth_single_segment.md`, `fn-path_optimize_straight_segments.md`, `fn-path_reroute_straight_line.md`, `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`, `docs/research/NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/movement/movement_path.rs`, `src/sim/pathfinding/path_smooth.rs`, `src/sim/pathfinding/core.rs`.

## Status

COMPLETE for the target question: active-YR smoothing rechecks live mover cell-entry legality through `Can_Enter_Cell`-style virtual calls. PARTIAL only for exact pier/waterbridge tile classification inside `UnitClass__Can_Enter_Cell`, which belongs to the bridge/pier classification slot.
