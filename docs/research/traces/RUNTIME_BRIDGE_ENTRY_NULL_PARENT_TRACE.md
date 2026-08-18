# Runtime Bridge Entry Null-Parent Trace

Date: 2026-05-27

Scenario: A standard ground vehicle using Drive locomotion attempts to enter a high-bridge bridgehead/bridge-transition cell from ground during runtime movement. This trace is limited to the runtime `Can_Enter_Cell` tuple and `CheckBridgeTraversal` null-parent fallback for that entry.

Concrete numeric probe:

- Current/predecessor cell: `(10,10)`, ground level `4`, no structural bridge bit, object `OnBridge = 0`.
- Target cell: `(11,10)`, ground level `0`, structural bridge bit `0x100 = 1`, bridgehead/transition bit `0x200 = 1`.
- Runtime direction: east `2`.
- Runtime height: current effective height `4`.
- `Can_Enter_Cell` parent/current-cell argument: `0`.
- `Can_Enter_Cell` arg5: `1`.

This is the high-bridge entry branch where the vehicle stays at effective height `4` while entering a bridgehead cell whose terrain level is `0` and deck height is `0 + 4 = 4`.

## Pipeline

Player move/path step -> `DriveLocomotionClass::Process_Drive_Track` runtime entry check -> virtual `UnitClass::Can_Enter_Cell(target, direction, height, parent, arg5)` -> `CheckBridgeTraversal(candidate, direction, &height, &list_byte, parent)` -> object-list / occupancy-layer selection -> movement either continues onto the bridgehead or blocks/repaths.

## Stage Table

| Stage | Our value | gamemd value | Verdict |
|---|---|---|---|
| 1. Live standard-YR entry point | Runtime boundary crossing in `src/sim/movement/movement_step.rs:520` calls `evaluate_runtime_can_enter_cell` before terrain/occupancy handling. For standard ground vehicles, this is the Drive runtime movement path. | `DriveLocomotionClass::Process_Drive_Track @ 0x004B1C3E` is active Drive locomotion code used by standard YR ground vehicles; the decompiled function contains runtime bridge, track, collision, and virtual `Can_Enter_Cell` logic. | PASS |
| 2. Runtime argument tuple | For `(10,10)->(11,10)`, `runtime_can_enter_direction` returns `2`; `runtime_current_effective_height` returns `4 + 0 = 4`; `RuntimeCanEnterCellArgs::runtime` sets `parent_current_cell=None` and `arg5=1`. Rust lines: `src/sim/movement/movement_occupancy.rs:37`, `47-68`, `78-123`; callsite `src/sim/movement/movement_step.rs:520-531`. | Drive runtime pushes `arg5=1`, parent/current-cell `0`, current effective height from `0x005F5F00`, direction, and target cell at `0x004B1C3E`; the callsite matrix records the same tuple shape for Drive runtime movement. | PASS |
| 3. Null-parent predecessor reconstruction | `resolve_parent_for_bridge_traversal` sees no explicit parent, rotates `(2 - 4) & 7 = 6`, applies west delta `(-1,0)`, and resolves predecessor `(10,10)` from target `(11,10)`. Rust: `src/sim/pathfinding/core.rs:483-504`. | `CheckBridgeTraversal @ 0x004D9C60` checks `param_5 == 0`, computes `param_2 - 4 & 7`, adds that direction offset to candidate cell coordinates, then calls `MapClass::Get_CellClass`. For direction `2`, this reconstructs `(11,10)+west=(10,10)`. | PASS |
| 4. Bridge-entry legality branch | Candidate level `0`; reconstructed parent is not structural, so selected parent height is path height `4`; `diff = 4 - 0 = 4`; branch `candidate_level == parent.level - 4` is true; candidate structural + bridgehead is true; Rust returns `allowed=true`, `force_bridge_list=true`, `path_height=4`. Rust: `src/sim/pathfinding/core.rs:535-590`. | `CheckBridgeTraversal` computes candidate signed level from `candidate+0x11B`; because parent structural bit is clear, it uses `*height=4`; `diff=4`; branch `candidate.level == parent.level - 4` requires candidate structural bit `0x100` and bridgehead bit `0x200`, writes `*list_byte = 1`, and returns `0`. | PASS |
| 5. Layer/list result consumed by runtime evaluator | `evaluate_runtime_can_enter_cell` applies `force_bridge_list`, then `can_enter_layer_context` with `path_height=4` and candidate `0+4`; object list is Bridge and occupancy bits layer is Bridge. Rust: `src/sim/movement/movement_occupancy.rs:150-184`; `src/sim/pathfinding/core.rs:594-612`. | Binary writes bridge list byte `1` on this branch. Existing verified bridge runtime docs state runtime bridge split remains a two-pass decision: list selection follows height/list byte, and deck-height structural cells use bridge occupancy bits. | PASS |
| 6. Full `UnitClass::Can_Enter_Cell` final return for the complete live map cell | Rust would continue into terrain/occupancy/building-blocker handling after the bridge traversal result. For the synthetic empty target assumed here, no blocker was executed in a live Rust harness during this trace. | gamemd `CheckBridgeTraversal` returns `0` for the bridge sub-check, but this trace did not run a live gamemd state sample through the entire `UnitClass::Can_Enter_Cell` object-list, gate, building, crush, and terrain body for the exact cell. | UNCHECKED |

## Findings

No FAIL or NOT-IMPLEMENTED finding was found for the scoped null-parent bridge-entry tuple.

The previous fix appears correct for this concrete bridge-entry shape:

- Runtime movement uses a binary-shaped tuple instead of substituting the mover's current cell as an explicit parent.
- Parent/current-cell remains `None`/`0` through the runtime evaluator.
- `CheckBridgeTraversal` reconstructs the predecessor from `target + opposite(direction)`.
- The high-bridge entry case computes the same literal values: direction `2`, height `4`, parent fallback `(10,10)`, candidate level `0`, diff `4`, bridge-list byte/result `1`, bridge traversal return `0`.

Residual risk: this trace did not prove the final full `UnitClass::Can_Enter_Cell` return code for every blocker/refinery/gate/bunker branch in the same target cell. That belongs to the other swarm slots and adjacent runtime evaluator traces, not this null-parent bridge-entry slot.

## Active Standard-YR Check

The traced runtime call is in `DriveLocomotionClass::Process_Drive_Track`, not a dormant TS-only path. Standard YR ground vehicles use Drive locomotion, and the audited callsite is inside the live movement/collision path. The separate Mech/older-locomotor site noted in the callsite matrix is explicitly excluded from this trace.

## Evidence

- Ghidra read-only decompile: `CheckBridgeTraversal @ 0x004D9C60`.
- Ghidra read-only decompile: `DriveLocomotionClass::Process_Drive_Track @ 0x004B1C3E`.
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md:207-345`.
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md:19-30`, `31-49`, `227-242`.
- Rust: `src/sim/movement/movement_occupancy.rs:37-184`.
- Rust: `src/sim/movement/movement_step.rs:520-531`.
- Rust: `src/sim/pathfinding/core.rs:483-612`.

## Adjacent Findings

- `movement_step.rs` passes `projected_on_bridge_state`, not the stored `snap.on_bridge`, into the height helper. For this ground-entry scenario both are `false`, so this does not affect the verdict here. Bridge-exit or lookahead scenarios should trace whether gamemd samples persistent `OnBridge` or a projected transition state at each callsite.
- This trace does not cover refinery pads, gates, bunkers, infantry Walk locomotion, or Drive lookahead; those are separate trace-swarm slots.

## Verdict Tally

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0
