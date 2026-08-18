# Coordinate/Height Trace: Screen Pick On High Bridge Move

Scenario: A selected ground unit is ordered to move by clicking visually on an intact high-bridge deck cell `(rx, ry)`.

Scope: screen -> world -> iso/cell conversion, bridge-height disambiguation, move-order creation, path/occupancy layer selection, and visible destination feedback. Adjacent bridge-edge and low-bridge behavior is not traced here.

## Verdict Summary

PASS: 3 | FAIL: 1 | UNCHECKED: 5 | NOT-IMPLEMENTED: 0

## Pipeline

Player screen click -> app screen/world conversion -> terrain screen-to-iso with bridge-height search -> context order target cell -> `Command::Move` -> sim movement command -> layered A* -> movement target/path layers -> bridge occupancy during movement -> visible target line/unit target.

## Concrete Coordinate Model

For a high bridge deck cell with ground level `g = 0` and bridge deck level `z = 4`, Rust renders the bridge tile via:

```text
tile_nw_x = (rx - ry) * 30 - 30
tile_nw_y = (rx + ry) * 15 + 15 - 4 * 15
deck_center_x = tile_nw_x + 30 = (rx - ry) * 30
deck_center_y = tile_nw_y + 15 = (rx + ry) * 15 - 30
```

At `(rx, ry) = (10, 10)`, Rust's visible deck center is `(0, 270)`.

Rust `screen_to_iso()` first treats that click as `z = 0`:

```text
col = 0 / 30 = 0
row = (270 - 30) / 15 = 16
initial = (8, 8)
```

Then `screen_to_iso_with_height_and_bridges()` searches bridge cells in a 7x7 neighborhood. Candidate `(10,10)` with `bridge_z = 4` corrects:

```text
corrected_y = 270 + 4 * 15 = 330
col = 0 / 30 = 0
row = (330 - 30) / 15 = 20
resolved = (10, 10)
dist = 0
```

So the clicked bridge deck cell resolves numerically to `(10,10)` in Rust for the center-click case.

## Stage Results

1. Screen/camera preprocessing: UNCHECKED. Rust uses `screen_x / zoom + camera_x`, `screen_y / zoom + camera_y` at `src/app_sim_tick.rs:1166`; gamemd inverse at `FUN_006d6590` subtracts radar viewport offsets and applies a camera matrix. I did not compute an identical live viewport/camera input pair.

2. Bridge deck center pick: PASS for the concrete center-click case. Rust resolves the bridge deck center back to `(rx,ry)` through `src/map/terrain.rs:275`; gamemd `FUN_006d6590` is active in YR, iterates cell height, checks `CellClass+0x140 & 0x100`, and applies bridge neighbor/deck correction. Both output the clicked bridge cell for a center click.

3. Bridge disambiguation edge pixels: UNCHECKED. Gamemd uses directional/cardinal bridge-neighbor tests with a `15` pixel threshold in `FUN_006d6590`; Rust uses a 7x7 closest-candidate search at `src/map/terrain.rs:304`. The center click is covered; boundary pixels are adjacent findings, not this trace.

4. High-bridge action/cursor classification: UNCHECKED for literal numeric equality. Gamemd `UnitClass__What_Action_OnCell` and `FootClass__What_Action_OnCell` are active in YR and return the normal Move action for intact high bridge cells; Rust emits a typed `Command::Move` at `src/app_context_order.rs:694`. The user-visible action category matches, but the representations are not directly numeric-equal.

5. Move target command cell: PASS. Rust queues `Command::Move { target_rx: rx, target_ry: ry }` at `src/app_context_order.rs:694`; the picked cell is not redirected because `is_any_layer_walkable()` accepts bridge-layer walkability at `src/app_sim_tick.rs:1158` and `src/sim/pathfinding/core.rs:1157`.

6. Input delay / execution tick: UNCHECKED. Rust schedules the command at `sim.tick + sim.input_delay_ticks` in `src/app_context_order.rs:61`. I did not compute gamemd's exact click-to-order tick timing for this scenario.

7. Layered path goal height: PASS. Rust sets `goal_height = goal_cell.bridge_deck_level` when `goal_bridge_ok` and the mover is not infantry at `src/sim/pathfinding/core.rs:536`. Gamemd `AStar_main_loop @ 0x00429A90` sets destination height to `CellClass+0x11B + 4` for non-aircraft moving to a `flags & 0x100` bridge cell. For `g=0`, both produce `4`.

8. Bridge occupancy during movement: UNCHECKED. Rust has bridge path layers and `BridgeOccupancy { deck_level }`, but I did not compute the exact tick where the moving unit's bridge occupancy flips versus gamemd's `CellClass+0xE4/+0xE8` list transfer.

9. Visible move target line endpoint: FAIL. Rust records the right cell, but `build_target_line_instances()` projects cell destinations using only `height_map`, not `bridge_height_map`, at `src/app_target_lines.rs:170`. For `(10,10)`, the line endpoint is `(0,330)` at ground center instead of `(0,270)` at deck center: a 60-pixel downward error. Gamemd's active projection uses Z in `TacticalClass__CoordsToClient2`, and the active bridge cell action path adds bridge height when `CellClass+0x140 & 0x100` is set, so bridge-cell visual feedback should be tied to deck height, not ground.

## Failures

### Stage 9 - Destination line endpoint uses ground height on bridge cells

Player-visible difference: after the click, the move feedback line points below the high bridge deck, visually targeting the ground/water underneath the bridge even though the actual move target/path is the bridge deck cell.

Rust evidence:

- `src/app_target_lines.rs:170` resolves move destinations as `LineDest::Cell { rx, ry }`.
- `src/app_target_lines.rs:171` reads `height_map.get(&(rx, ry))`.
- No bridge-height map is passed into `build_target_line_instances()`.

Concrete numeric difference for `(10,10)`, `ground=0`, `deck=4`:

```text
Rust target line endpoint = iso_to_screen(10,10,0) + (30,15) = (0,330)
Correct deck endpoint     = iso_to_screen(10,10,4) + (30,15) = (0,270)
Delta                     = +60 px screen Y
```

Gamemd evidence:

- `TacticalClass__CoordsToClient2` projects Z upward by subtracting the Z contribution from screen Y.
- `FootClass__What_Action_OnCell` is active in YR and adds bridge height when `CellClass+0x140 & 0x100` is set.
- `AStar_main_loop @ 0x00429A90` is active in YR and sets bridge destination height to `Level + 4`.

## Not Implemented

None for this concrete scenario.

## Unchecked Items

- Exact gamemd click-to-command tick timing.
- Exact screen/camera/radar viewport equivalence for a live viewport.
- Exact bridge-boundary pixel choices away from the deck center.
- Exact tick of Rust `BridgeOccupancy` flip versus gamemd object-list transfer.
- Exact gamemd target-line/destination-marker draw function; the failure is grounded in the active bridge Z projection/action/path evidence and the Rust endpoint math.

## Adjacent Findings

- Rust's bridge-pick strategy is square-radius/closest-candidate; gamemd's is cardinal/directional with a 15-pixel threshold. This is likely to matter at bridge/ramp boundaries, but it is outside this center-click trace.
- Rust target-line rendering should probably accept `bridge_height_map` or a per-cell effective render height for move destinations.

## Status

COMPLETE
