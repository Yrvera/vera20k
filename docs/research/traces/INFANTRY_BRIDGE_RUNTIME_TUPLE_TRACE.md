# Infantry Bridge Runtime Tuple Trace

**Scenario:** Infantry `WalkLocomotionClass::ProcessMovement` attempts one adjacent step onto/across a bridge-sensitive cell and calls `InfantryClass::Can_Enter_Cell`.

**Concrete values used for literal comparison:** current cell `(10,10)`, target cell `(11,10)`, direction east `2`, current cell level `0`, target cell level `0`. Two height cases are traced:

- Ground-to-bridge-sensitive target: `OnBridge=false`, so runtime height `0`.
- Bridge-deck crossing / split-layer case: `OnBridge=true`, so runtime height `4`.

**Scope limits:** This trace covers only the Walk runtime tuple and the early bridge/height effects that feed object-list and occupancy-bit layer selection. It does not trace gates, bunkers, refinery pads, drive locomotion, A* path expansion, rendering, or tube path execution beyond noting adjacent unresolved facts.

**Live Ghidra status:** Read-only `batch_decompile` for `0x0075AEC0`, `0x0051BF90`, and `0x004D9C60` returned `Function not found` in this MCP session. Binary facts below are therefore taken from already-verified research reports that cite live Ghidra assembly/decompilation. No mutating Ghidra operation was used.

## Evidence Inputs

- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`: verifies Walk runtime call stack at `0x0075B669..0x0075B690`: `(target_cell, direction, current_effective_height, 0, 1)`.
- Same report: verifies `0x005F5F00` effective height helper as `current_cell.level + (Object.OnBridge ? 4 : 0)`.
- Same report: verifies `CheckBridgeTraversal @ 0x004D9C60` derives parent from `target + Direction[(direction - 4) & 7]` when arg4 is zero.
- `docs/research/INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md`: verifies Infantry vtable `+0x1AC = 0x0051BF90`, `+0x1B0 = 0x004D9C60`, shared bridge prologue, layer-separated `+0x124/+0x128` occupancy bits, and the infantry-only `path_height - cell.Level > 4 -> return 0`.
- `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`: confirms Walk is active in standard YR and used by stock infantry; bridge code in Walk is live.

## Pipeline

1. Walk runtime computes next target cell from the path direction.
2. Walk calls `InfantryClass::Can_Enter_Cell` through vtable `+0x1AC`.
3. The runtime tuple is pushed as `target`, `direction`, `current_effective_height`, `0`, `1`.
4. `InfantryClass::Can_Enter_Cell` preselects an object-list layer from target bridge flag and height-vs-level.
5. Infantry has a class-local high-path shortcut: if `height - target.Level > 4`, return code `0` before bridge traversal.
6. Otherwise Infantry calls shared `CheckBridgeTraversal @ 0x004D9C60`.
7. If arg4 is null, `CheckBridgeTraversal` infers parent/current from target plus the opposite direction.
8. After bridge traversal, `Can_Enter_Cell` uses bridge occupancy bits only when final `height == target.Level + 4`; otherwise it uses ground occupancy bits.
9. Runtime movement then uses the split layer context for deferred object/subcell classification.

## Stage Results

| Stage | Concrete output | gamemd output | Rust output | Verdict |
|---|---:|---:|---:|---|
| Runtime target cell | `(11,10)` | `(11,10)` | `RuntimeCanEnterCellArgs.target_cell = (11,10)` | PASS |
| Runtime direction | `2` | `2` for east step | `runtime_can_enter_direction((10,10),(11,10)) = 2` | PASS |
| Runtime height, ground case | `0` | `0 + 0 = 0` | `runtime_current_effective_height(..., OnBridge=false) = 0` | PASS |
| Runtime height, bridge case | `4` | `0 + 4 = 4` | `runtime_current_effective_height(..., OnBridge=true) = 4` | PASS |
| Hidden arg4 parent/current | `0` | `0` | `parent_current_cell = None` | PASS |
| Hidden arg5 | `1` | `1` | `arg5 = 1` but currently not consumed by evaluator | UNCHECKED |
| Null-parent fallback | `(10,10)` | `(11,10) + dir 6 = (10,10)` | `resolve_parent_for_bridge_traversal` uses `((direction - 4) & 7)` | PASS |
| Split layers, bridge-height case | object list bridge, occupancy bits bridge | bridge/bridge when height `4 == level 0 + 4` | `can_enter_layer_context` returns bridge occupancy bits under same predicate | PASS |
| Split layers, non-deck bridge-sensitive case | object-list may differ from occupancy bits | verified live split: list precheck/CBT, occupancy only at deck height | Rust has `CanEnterLayerContext`, but full terminal infantry return-code equivalence is not proven | UNCHECKED |
| Infantry high-path shortcut | return code `0` when `height - target.Level > 4` | `0x51C055..0x51C062`: return `0` before CBT | no mover-category input and no infantry shortcut in `evaluate_runtime_can_enter_cell` | NOT-IMPLEMENTED |

## Rust Comparison

`src/sim/movement/movement_step.rs:521` now calls `evaluate_runtime_can_enter_cell` before transition checks. The arguments at `movement_step.rs:524..530` are shaped correctly for the ordinary Walk runtime call: current cell, target cell, projected current `OnBridge`, and current `z` fallback.

`src/sim/movement/movement_occupancy.rs:47..67` stores the five runtime-shaped arguments. `runtime_can_enter_direction` at `movement_occupancy.rs:78..91` yields `2` for the east step. `runtime_current_effective_height` at `movement_occupancy.rs:94..110` matches `0x005F5F00` for the traced values. `runtime_can_enter_cell_args` at `movement_occupancy.rs:112..123` sets parent to `None` and arg5 to `1`.

`src/sim/movement/movement_occupancy.rs:126..190` reproduces the shared bridge layer split and null-parent bridge traversal path, but it has no infantry/vehicle discriminator. That means it cannot implement `InfantryClass::Can_Enter_Cell`'s early `height - target.Level > 4 -> 0` branch.

`src/sim/pathfinding/core.rs:483..504` matches the null-parent fallback geometry: when no explicit parent is passed, parent is `candidate + Direction[(direction - 4) & 7]`. For the east-step target `(11,10)`, this resolves to `(10,10)`.

`src/sim/pathfinding/core.rs:594..613` implements the verified post-bridge occupancy-bit predicate: bridge occupancy bits are selected only when `path_height != -1`, the target is structural bridge, and `path_height == target.Level + 4`.

`src/sim/pathfinding/cell_entry.rs:382..435` and `src/sim/movement/movement_occupancy.rs:201..244` use the split object-list and occupancy-bit layers for infantry subcell checks. Runtime deferred detection is stricter than the generic terrain phase because it also treats infantry on the selected object-list layer as a reason to defer. The full terminal `0x0051BF90` subcell return ladder remains unverified in the research corpus, so this cannot be marked PASS.

## Failures And Gaps

### NOT-IMPLEMENTED: Infantry high-path shortcut

Gamemd `InfantryClass::Can_Enter_Cell` has an infantry-only early return: if `height - target.Level > 4`, return code `0` before `CheckBridgeTraversal`. Current Rust runtime evaluation has no mover category and always enters the shared bridge traversal path after preselecting the object-list layer. In bridge-collapse/high-over-low edge cases, infantry can be accepted by gamemd while Rust may block or continue into object/occupancy classification.

Player-visible effect: infantry may fail or hesitate at abnormal bridge height transitions where gamemd lets them continue.

Rust touchpoint: `src/sim/movement/movement_occupancy.rs:126..190`.

Gamemd evidence: `INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md` section 3.2 and `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md` section 5.2.

### UNCHECKED: arg5 behavioral use

The runtime tuple now stores `arg5 = 1`, matching the Walk callsite, but the current evaluator does not consume `arg5`. The verified reports identify the value, not its complete downstream semantics in every passability branch. For this bridge-layer trace, no concrete non-bridge output difference was computed, so this stays UNCHECKED rather than FAIL.

### UNCHECKED: complete Infantry terminal subcell return ladder

Rust has split-layer subcell availability and selected object-list blocking, but the exact terminal return-code ladder inside `0x0051BF90` for all full/partial infantry subcell patterns is still not preserved in the available docs. For the empty/free-subcell traced case the runtime layer values match, but a full literal PASS for all subcell occupancy outputs is not justified.

## Adjacent Findings

- Infantry-specific building/garrison/weapon-range return codes are still outside this trace. Existing research says Rust should not reuse UnitClass building exceptions for infantry, and the current generic blocker classifier is not a complete `0x0051BF90` building policy.
- Low bridge TubeClass direction-8 entry is live but conditional and not traced here. This report only covers the adjacent normal direction `2` step.
- `movement_step.rs` supplies the current `OnBridge` state via `projected_on_bridge_state`. For the traced first crossing this equals the current object state; multi-crossing-in-one-tick equivalence was not computed.

## Verdict Tally

PASS: 7 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

