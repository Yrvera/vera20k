# Drive-Track Lookahead Runtime Can_Enter_Cell Tuple Trace

Date: 2026-05-27

Scenario: A Drive locomotor vehicle is mid drive-track and reaches the chain/lookahead point before a next-next bridge-sensitive cell. Scope is only the runtime `Can_Enter_Cell` tuple for that lookahead probe and the immediately consumed Rust decision in `movement_tick.rs` / `movement_occupancy.rs`.

## Verdict

Rust now matches the gamemd tuple shape for the chain/lookahead probe:

```text
(target_cell, direction, current_effective_height, parent/current = 0, arg5 = 1)
```

However, the lookahead still does not consume the resulting `Can_Enter_Cell` return code with gamemd's switch behavior. It converts the runtime layer reconstruction into a walkable-plus-empty boolean, so occupied/crush/scatter return-code cases can diverge at the same chain point.

Verdict tally: PASS: 5 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Evidence

- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` is active for standard YR DriveLocomotion units. Existing research records `DriveLocomotionClass::Process @ 0x004B0500` calling it at `0x004B0576` and `0x004B0AAA`; the read-only Ghidra decompile for this trace also resolved the active function body.
- The audited Drive `Process_Drive_Track` runtime callsite is `0x004B1C3E`. Existing assembly reports show `PUSH 0x1`, `PUSH 0x0`, call `0x005F5F00` for current effective height, push direction, map target coord to `CellClass`, then call vtable `+0x1AC`.
- `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md` gives the final stack as `target_cell = MapClass::Get_CellClass(head_to + DirectionDelta[direction & 7])`, `direction = current movement/track direction`, `height = current cell level + (OnBridge ? 4 : 0)`, `parent/current cell = 0`, `arg5 = 1`.
- `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md` confirms the null parent is not equivalent to passing the object's current cell. With a valid direction, `CheckBridgeTraversal` reconstructs the predecessor from target plus `(direction - 4) & 7`.
- Current Rust chain point is `src/sim/movement/movement_tick.rs:815-918`; runtime tuple construction is `src/sim/movement/movement_tick.rs:848-862`; runtime args and layer evaluation are `src/sim/movement/movement_occupancy.rs:47-189`.

## Stage Table

| Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|
| Active standard-YR path | `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` runs from Drive `Process`; no TS-only gate found for this callsite. | Rust DriveTrack chain code is active when `advance_drive_track` returns `DriveTrackChainReady`. | PASS |
| Concrete target for sample path | For current path/head cell `(11,10)`, next direction east `2`, target is `(12,10)`. | `after = target.path[target.next_index + 1]`; with `after=(12,10)`, target is `(12,10)`. | PASS |
| Direction argument | Direction is a valid track/path direction, sample east `2`. | `runtime_can_enter_direction((11,10), (12,10)) = 2`. | PASS |
| Height argument | `current_effective_height = current_cell.level + (OnBridge ? 4 : 0)`. For current level `0`, `OnBridge=false`, height `0`; for `OnBridge=true`, height `4`. | `runtime_current_effective_height(path_grid, entity current cell, entity.on_bridge, fallback_z)` computes the same formula. | PASS |
| Parent/current and arg5 | Parent/current-cell arg is literal `0`; arg5 is literal `1`. | `RuntimeCanEnterCellArgs::runtime` stores `parent_current_cell=None` and `arg5=1`, where `None` is the Rust representation of the null parent pointer. | PASS |
| Return-code consumption at chain point | The decompiled active function switches on the `Can_Enter_Cell` result: codes `0` and `2` continue into chain setup; codes `1`, `3`, and `6` have redraw/crush/scatter side effects. | Rust computes `next_walkable && not_reserved`; any occupancy in object-list or occupancy-bits layer blocks chaining, and no code `1`/`3`/`6` side-effect path runs here. `src/sim/movement/movement_tick.rs:863-896`. | FAIL |
| Exact CEC result for a live bridge map fixture | Not measured against gamemd for a retail map/save at the exact chain tick. | Not measured in Rust for the same fixture. | UNCHECKED |
| Test execution | Not run; running Cargo would write outside the single allowed report file. | Not run for the same reason. | UNCHECKED |

## Player-Visible Failures

1. Stage 6: At a bridge-adjacent drive-track chain point, a friendly moving blocker that gamemd reports as code `2` can still allow the chain, while Rust's `not_reserved` gate rejects the occupied layer and fails to chain. The player can see a vehicle hesitate, keep the old track, or choose a later movement response instead of the smooth gamemd continuation.
2. Stage 6: Gamemd code `6` can scatter friendly stationary objects from this drive-track loop, with bridge-layer selection influenced by the height check. Rust's lookahead path has no scatter side effect at this point, so a vehicle can stall or route differently around bridge-sensitive cells.
3. Stage 6: Gamemd code `3` can invoke crushable-obstacle handling from the active drive-track loop. Rust's lookahead boolean does not invoke the crush/obstacle side effect here, so chain setup around crushable blockers can diverge.

## Adjacent Findings

- This trace does not cover Drive `Process_Movement` next-path and late lookahead callsites at `0x004B2FF9`, `0x004B34C0`, or `0x004B4120`. Existing reports say they use the same parent `0`, arg5 `1`, and current-effective-height shape.
- This trace does not cover low-bridge TubeClass direction `8` entry or Jumpjet/Hover `direction=-1,height=-1` runtime callsites.

## Sources

- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`
- `docs/research/RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`
- `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`
- Read-only Ghidra decompile: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_occupancy.rs`
- `src/sim/pathfinding/core.rs`
