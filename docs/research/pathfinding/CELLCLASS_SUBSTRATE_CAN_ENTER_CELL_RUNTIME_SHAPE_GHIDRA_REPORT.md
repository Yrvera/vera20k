# CellClass Substrate Can_Enter_Cell Runtime Shape - Ghidra Research Report

**Address(es):** `0x0073F0A0` (`UnitClass::Can_Enter_Cell`), `0x004D9C60` (`CheckBridgeTraversal`), caller evidence at `0x00429F54`, `0x004B1C3E`, `0x004B2FF9`, `0x00515570`, `0x0054C66D`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** runtime argument shape and bridge/list-layer inputs needed for a Rust-native CellClass substrate migration, focused on `UnitClass::Can_Enter_Cell` and `CheckBridgeTraversal`; caller tracing only where it proves argument values.
**Non-Scope:** complete `Can_Enter_Cell` blocker tree, all locomotion classes, all InfantryClass policy, CellRect contracts, object-list writer lifecycle, and Rust patch design.
**Confidence:** High for argument count/order, parent/current-cell behavior, bridge/list/occupancy layer inputs, and runtime/A* caller contrast; Medium for richer semantic naming of the fifth argument beyond the verified locomotor-passability gate.
**Active in YR:** Yes for UnitClass vehicle movement/pathfinding and bridge traversal; Conditional for Jumpjet landing/abort and Hover push-style `(-1,-1)` caller shapes.

## Working Notes Gate

Target question: What exact runtime-shaped argument/context must a Rust-native CellClass substrate expose for `UnitClass::Can_Enter_Cell`, especially direction, height, parent/current cell, arg5, and bridge/list-layer inputs?

Non-goals: Do not re-investigate complete Unit/Infantry blocker policy, CellRect validators, object-list writers, or implement Rust; do not mutate Ghidra; write only this report and the shared claims row.

Evidence needed to mark COMPLETE: fresh Ghidra decompile plus assembly/callsite evidence for `0x0073F0A0`, `0x004D9C60`, A* and runtime caller argument values; Rust surface scan; implementation handoff with tests and negative facts.

Stop conditions: Stop after the substrate API inputs are verified; defer richer arg5 naming, full Infantry contrast, and non-vehicle locomotion policy to later slots.

## 1. Overview

`UnitClass::Can_Enter_Cell` is a five-stack-argument `thiscall` cell-entry predicate, not a terrain-only pathgrid query. It consumes `(target cell, direction, path/current height, parent/current cell pointer, locomotor-passability gate)` and internally derives two separate CellClass layer decisions: object-list layer (`Cell+0xE4` vs `Cell+0xE8`) and occupancy-bit layer (`Cell+0x124` vs `Cell+0x128`).

`CheckBridgeTraversal @ 0x004D9C60` is the shared bridge-height sub-check at vtable `+0x1B0`. It can mutate the caller's height and force bridge object-list selection; it also treats a null parent/current argument as an explicit runtime mode by reconstructing the predecessor from the target cell and direction.

## 2. Class Layout / Key Offsets

| Offset / slot | Verified role | Active in YR | Evidence |
|---|---|---|---|
| Unit vtable `+0x1AC` | `UnitClass::Can_Enter_Cell`, five stack args, returns code `0..7` | Yes | A* call `CALL [EDX+0x1ac]` at `0x00429F54`; Unit return cleanup `RET 0x14` near `0x0073F300` |
| Unit/Foot vtable `+0x1B0` | `CheckBridgeTraversal` sub-check, returns `0` or `7` | Yes | Unit call `CALL [EAX+0x1b0]` at `0x0073F2EB`; decompile `0x004D9C60` |
| Stack arg 1 | target `CellClass*` | Yes | A* pushes candidate before `0x00429F54`; runtime target from `MapClass::Get_CellClass` before `0x004B1C3E` |
| Stack arg 2 | direction: `0..7`, `8` for tube entry, `-1` for candidate-only bridge seed callers | Yes / Conditional | Unit tube branch `param_3 == 8`; CheckBridge `param_2 == -1`; Jumpjet call `0x0054C662..0x0054C66D` |
| Stack arg 3 | path/current effective height, mutable via `&height` in `CheckBridgeTraversal` | Yes | Unit forwards `&height` before `0x0073F2EB`; CheckBridge writes `*param_3 = level + 4` |
| Stack arg 4 | optional parent/current `CellClass*`; null is meaningful | Yes | Unit forwards it as CheckBridge arg5; CheckBridge fallback branch when `param_5 == 0` |
| Stack arg 5 | locomotor passability gate, passed to `FootClass::LocomotorPassabilityCheck` | Yes | Unit calls `0x004D9C10`; helper reads stack byte at `in_stack_00000014` and skips locomotor COM when zero |
| `Cell+0xE4` / `Cell+0xE8` | ground vs bridge object-list heads | Yes | Unit list selection at `0x0073F4F9..0x0073F51A` |
| `Cell+0x124` / `Cell+0x128` | ground vs bridge occupancy bits | Yes | initial read at `0x0073F0ED..0x0073F0FA`; bridge re-read at `0x0073F32C..0x0073F348` |
| `Cell+0x140 & 0x100` | structural bridge/deck bit | Yes | Unit early list choice; CheckBridge diff logic |
| `Cell+0x140 & 0x200` | bridgehead/transition bit required for bridge entry | Yes | CheckBridge low-to-high `diff==4` branch |
| `Cell+0x11B` | signed terrain level | Yes | `MOVSX` in Unit `0x0073F0CE`; CheckBridge decompile |
| `Cell+0x11C` | slope byte for `abs(diff)==1` branch | Yes | CheckBridge decompile |

## 3. Core Logic

### 3.1 UnitClass Five-Argument Shape

Active in YR: Yes.

The binary call contract is:

```text
UnitClass::Can_Enter_Cell(this, target_cell, direction, height, parent_or_current_cell, arg5)
```

Load-bearing details:

- The function uses `RET 0x14`, proving five explicit 32-bit stack arguments in addition to `this`.
- `AStar_main_loop` supplies all five arguments before `CALL [EDX+0x1AC]`: candidate cell, neighbor direction, current node/path height, explicit current node `CellClass*`, and a low byte from `Pathfinder+0x08`.
- Runtime Drive/Ship/Hover valid-direction callers supply target cell, valid direction, current effective height, parent/current `0`, and arg5 `1`.
- Jumpjet landing/abort and Hover Push/Shove-style callers supply target cell, direction `-1`, height `-1`, parent/current `0`, and arg5 `1`.

Evidence: `0x00429F54` assembly context; `0x004B1C3E`, `0x004B2FF9`, `0x00515570`, `0x0054C66D` assembly contexts; Unit decompile `0x0073F0A0`; helper `0x005F5F00`.

### 3.2 Fifth Argument Is Not The Parent Or A Layer Enum

Active in YR: Yes.

The fifth stack argument is preserved into the `FootClass::LocomotorPassabilityCheck @ 0x004D9C10` call. That helper reads its fifth stack byte and only calls the locomotor COM passability method when both `self+0x674` is non-null and that byte is nonzero; otherwise it returns `0`.

This proves the substrate must carry a locomotor-passability gate separately from:

- the optional parent/current cell pointer;
- the object-list layer byte;
- the terrain/path layer;
- the bridge occupancy-bit layer.

Runtime audited callsites use `arg5=1`. A* passes `Pathfinder+0x08` low byte at `0x00429F54`, so future migration should preserve this as an explicit field instead of assuming it is always true.

### 3.3 Early Object-List Layer Selection

Active in YR: Yes.

At function entry, UnitClass computes a local object-list byte:

```text
if !(target.flags & 0x100): ground list
else if height != -1 and abs(height - target.level) < 2: ground list
else bridge list
```

Tiny details:

- The level load is signed (`MOVSX` from `Cell+0x11B`).
- The threshold is strict: differences `0` and `1` use ground; differences `2+` use bridge.
- Height `-1` on a structural bridge target selects bridge in this early step.
- This byte is later consumed directly: zero selects `Cell+0xE4`, nonzero selects `Cell+0xE8`.

Evidence: `0x0073F0B7..0x0073F0E8` and list selection `0x0073F4F9..0x0073F51A`.

### 3.4 CheckBridgeTraversal Contract

Active in YR: Yes.

`CheckBridgeTraversal(candidate, direction, &height, &bridge_list_byte, parent_or_current)` returns `0` or `7` and may mutate `height` or `bridge_list_byte`.

Verified branches:

- If `parent_or_current == 0`, it reconstructs the predecessor as `candidate + DirectionOffset[(direction - 4) & 7]`.
- If `direction == -1`, it skips directed diff/slope validation; if `height == -1` and candidate has structural bridge `0x100`, it writes `height = candidate.level + 4`, then returns `0`.
- If explicit/fallback parent has structural bridge `0x100` and `height == -1`, it writes `height = parent.level + 4`; the candidate must have bridgehead bit `0x200` or the call returns `7`.
- `abs(diff)==0` is allowed only for unset/matching height or the all-bridge bridgehead case.
- `abs(diff)==1` requires a nonzero slope byte on the side selected by movement direction.
- `abs(diff)==4` high-to-low requires `height == candidate.level` and parent structural bridge.
- `abs(diff)==4` low-to-high requires candidate structural bridge and bridgehead bits, writes `*bridge_list_byte = 1`, and returns `0`.
- `abs(diff)==2`, `3`, and `5+` return `7`.

Evidence: decompile `0x004D9C60`; Unit callsite assembly `0x0073F2EB`; runtime null-parent trace confirmed values for direction `2`.

### 3.5 Post-Bridge Occupancy-Bit Layer Re-Read

Active in YR: Yes.

After `CheckBridgeTraversal`, UnitClass re-reads bridge occupancy bits only when:

```text
height != -1
and target.flags & 0x100 != 0
and height == target.level + 4
```

When true, it reads `Cell+0x58` and `Cell+0x128`; otherwise it keeps the earlier `Cell+0x54` and `Cell+0x124` snapshot.

This occupancy-bit layer is independent from the object-list byte except where `CheckBridgeTraversal` explicitly forces the list byte on the ascending `diff==4` bridgehead branch. A substrate API therefore needs both object-list layer and occupancy-bit layer, not one generic "movement layer".

Evidence: assembly `0x0073F303..0x0073F348`; object-list selection `0x0073F4F9..0x0073F51A`.

## 4. INI Keys

This slice did not require new INI decoding. The relevant YR activation evidence is caller/content based:

| Source | Relevance | Active in YR |
|---|---|---|
| Standard UnitClass ground vehicles with Drive locomotor | Reach runtime `Can_Enter_Cell` callsites | Yes |
| Ship/Hover/Jumpjet locomotor callsites from prior runtime matrix | Additional caller shapes with the same five-arg contract | Yes / Conditional |
| Bridge CellClass flags and tube/height fields | Runtime map state, not INI constants | Yes |

## 5. Integration Points

| Integration point | Runtime shape | Active in YR | Evidence |
|---|---|---|---|
| A* neighbor expansion | `(candidate, direction 0..8, current_path_height, explicit current-node cell, Pathfinder+0x08 low byte)` | Yes | `0x00429F54` decompile + assembly |
| Drive `Process_Drive_Track` | `(target, valid direction, current effective height, 0, 1)` | Yes | `0x004B1C3E`; `0x005F5F00` |
| Drive `Process_Movement` next/probe sites | `(target, valid direction, current effective height, 0, 1)` | Yes | `0x004B2FF9`; prior matrix for sibling sites |
| Hover normal movement | `(target, valid direction, current effective height, 0, 1)` | Yes | `0x00515570` |
| Jumpjet landing/abort | `(target, -1, -1, 0, 1)` | Conditional | `0x0054C66D`; prior matrix for `0x0054CE34` |
| Infantry contrast | shares early bridge/list and `CheckBridgeTraversal`, but policy after the shared layer substrate diverges | Yes | Infantry decompile `0x0051BF90` |

`CellClass__Get_Effective_Height @ 0x005F5F00` computes runtime height as:

```text
current_cell.level + (object.OnBridge ? 4 : 0)
```

Evidence: decompile `0x005F5F00`, reading `Object+0x8C` byte (`param_1[0x23]`) and `Cell+0x11B`.

## 6. Current Rust Implementation Status

Read-only scan only; no Rust was modified.

| Surface | Current observed status | Rust-facing implication |
|---|---|---|
| `src/sim/movement/movement_occupancy.rs:38-68` | Defines `RuntimeCanEnterCellArgs` with target, direction, height, nullable parent/current, and `arg5=1` for runtime | Good match for audited runtime tuple; future substrate should generalize arg5 for A* and non-runtime callers |
| `src/sim/movement/movement_occupancy.rs:127-190` | Computes early object-list layer, calls `check_bridge_traversal`, then computes `CanEnterLayerContext` with split layers | Correct architectural direction for CellClass substrate migration |
| `src/sim/pathfinding/core.rs:467-612` | `BridgeTraversalInput` and `check_bridge_traversal` model nullable parent, direction `-1`, height mutation, force-bridge-list, and occupancy-bit layer split | Reuse as substrate kernel, but verify all A* callers carry the fifth arg/gate |
| `src/sim/pathfinding/cell_entry.rs:119-180` | Terrain entry context is still terrain/layer-only | It is not yet a full native-shaped `Can_Enter_Cell` substrate because object-list writers, dynamic blocker policy, and arg5 are outside this context |
| `src/sim/movement/movement_step.rs:948-959` | Runtime transition constructs args from current/target cells and projected bridge state | Needs focused follow-up on persistent `OnBridge` vs projected bridge state for multi-crossing edge cases |
| `src/sim/pathfinding/core.rs:1274-1294` | A* soft-block costs use precomputed entity block maps rather than live vtable `Can_Enter_Cell` per neighbor | Broader A* parity gap; substrate migration should make the full entry context callable from A* |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-notes gate | verified | report section | none |
| UnitClass five-stack-arg shape | verified | `RET 0x14`; A*/runtime call contexts | none for argument count/order |
| Unit forwards parent/current into CheckBridgeTraversal | verified | `0x0073F2EB` call setup; `0x004D9C60` param5 fallback | none |
| Fifth arg locomotor-passability gate | verified | `0x004D9C10` reads stack byte; Unit passes original arg to helper | richer semantic name deferred |
| Early object-list byte | verified | `0x0073F0B7..0x0073F0E8`; `0x0073F4F9..0x0073F51A` | none |
| Post-bridge occupancy re-read | verified | `0x0073F303..0x0073F348` | none |
| CheckBridgeTraversal branch set | verified | decompile `0x004D9C60` | exact disassembly listing not needed beyond call/context evidence |
| Runtime Drive/Ship/Hover parent-null tuple | verified for sampled callsites, supported by prior matrix | `0x004B1C3E`, `0x004B2FF9`, `0x00515570` | full Ship sibling rows rely on prior matrix |
| Jumpjet `(-1,-1,0,1)` contrast | verified for landing site | `0x0054C66D` | Hover push and Jumpjet abort rely on prior matrix |
| Infantry contrast | touched-not-exhausted | decompile `0x0051BF90` | full Infantry policy out of scope |
| Rust status | touched-not-exhausted | source scan | slot 4 owns full caller inventory |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Does UnitClass::Can_Enter_Cell take four or five stack args? -> Five stack args; return cleanup is `RET 0x14`.` (evidence: `0x0073F300`; A* context `0x00429F54`)
- `[RESOLVED] OQ-2 - Which argument is parent/current cell? -> The fourth stack argument after height; Unit forwards it as CheckBridgeTraversal's parent/current argument.` (evidence: `0x0073F2EB`; `0x004D9C60`)
- `[RESOLVED] OQ-3 - What does null parent/current mean? -> It triggers predecessor reconstruction from target plus `(direction - 4) & 7`; it is not equivalent to passing the mover's current cell pointer.` (evidence: `0x004D9C60`)
- `[RESOLVED] OQ-4 - What is runtime height? -> Current cell level plus `4` when object `OnBridge` is true.` (evidence: `0x005F5F00`)
- `[RESOLVED] OQ-5 - What does arg5 do? -> It gates `FootClass::LocomotorPassabilityCheck`; zero skips the locomotor COM terrain check.` (evidence: `0x004D9C10`)
- `[RESOLVED] OQ-6 - Do audited runtime callers pass arg5=1? -> Yes for sampled Drive, Hover, and Jumpjet sites, consistent with prior runtime matrix.` (evidence: `0x004B1C3E`, `0x004B2FF9`, `0x00515570`, `0x0054C66D`)
- `[RESOLVED] OQ-7 - Does A* pass the same parent mode as runtime? -> No; A* passes explicit current-node cell, not null parent.` (evidence: `0x00429F54`)
- `[RESOLVED] OQ-8 - Is object-list layer the same as occupancy-bit layer? -> No; Unit computes an early list byte and independently re-reads bridge occupancy bits after bridge traversal.` (evidence: `0x0073F0B7..0x0073F0E8`; `0x0073F303..0x0073F348`)
- `[RESOLVED] OQ-9 - Can CheckBridgeTraversal force bridge object-list selection? -> Yes, only on the low-to-high `abs(diff)==4` bridgehead branch.` (evidence: `0x004D9C60`)
- `[RESOLVED] OQ-10 - Is direction -1 a live mode? -> Yes conditionally for Jumpjet landing/abort and Hover Push-style callers; CheckBridge seeds candidate bridge height and skips directed diff checks.` (evidence: `0x0054C66D`; `0x004D9C60`)
- `[RESOLVED] OQ-11 - Does Infantry share the substrate shape? -> It shares early bridge/list selection and `CheckBridgeTraversal`, but diverges in class-specific blocker policy.` (evidence: `0x0051BF90`)
- `[RESOLVED] OQ-12 - Does current Rust expose runtime tuple fields? -> Yes for runtime in `RuntimeCanEnterCellArgs`; full substrate/A* integration remains incomplete.` (evidence: `src/sim/movement/movement_occupancy.rs:38-68`)
- `[DEFERRED] OQ-13 - What is the exact semantic name/source of `Pathfinder+0x08` fifth-arg byte?` (category: `requires-different-system-context`; reason: this slot proved consumption and caller value, not Pathfinder construction; next-step-if-pursued: audit Pathfinder constructors/writers of `+0x08`)
- `[DEFERRED] OQ-14 - Does `movement_step.rs` use projected vs persistent OnBridge exactly for every multi-crossing runtime call?` (category: `requires-different-system-context`; reason: needs runtime movement tick trace, not substrate shape; next-step-if-pursued: trace bridge-exit multi-crossing call order)
- `[DEFERRED] OQ-15 - Full Ship/Hover/Jumpjet sibling caller matrix?` (category: `out-of-scope`; reason: prior matrix covers it and this slot sampled enough to prove shape; next-step-if-pursued: verify-doc pass on runtime matrix)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Can_Enter_Cell` has a five-stack-arg context: target, direction, mutable height, optional parent/current cell, and locomotor-passability gate. | `0x0073F300` `RET 0x14`; `0x00429F54`; `0x004D9C10`; Active in YR: Yes | partial: runtime has these fields, A*/terrain contexts do not all expose arg5/parent mode as first-class | `src/sim/movement/movement_occupancy.rs`, `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/cell_entry.rs` | Introduce/standardize a native-shaped `CanEnterCellContext` for substrate calls, with separate parent/current and arg5/gate fields. | A* can call the same substrate with explicit parent and `Pathfinder+0x08` gate while runtime calls with parent `None` and arg5 `1`; proposed test: `test_can_enter_context_preserves_parent_and_arg5_modes`. | Do not collapse arg5 into a layer enum or assume runtime `1` applies to all callers. |
| Runtime null parent is an active bridge mode; `CheckBridgeTraversal` reconstructs predecessor from target plus `(direction-4)&7`, while A* passes explicit parent. | `0x004D9C60`; `0x004B1C3E`; `0x00429F54`; Active in YR: Yes | mostly matched for runtime helper, but substrate migration must keep A*/runtime distinction | `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_occupancy.rs`, A* caller surfaces | Keep nullable parent semantics through all substrate APIs; use explicit parent only for A* and caller shapes that actually pass it. | Bridge entry from ground with target `(11,10)`, direction east, parent `None`, height `4` reconstructs `(10,10)` and forces bridge list; proposed test: `test_runtime_bridge_entry_null_parent_reconstructs_predecessor`. | Do not substitute mover current cell for null parent. |
| Object-list layer and occupancy-bit layer are independent outputs: early list byte selects `E4/E8`; post-bridge height may re-read `124/128`; ascending bridgehead can force list byte. | Unit `0x0073F0B7..0x0073F0E8`, `0x0073F303..0x0073F348`, `0x0073F4F9..0x0073F51A`; Active in YR: Yes | partially matched by `CanEnterLayerContext`, but broader callers still use terrain/layer-only entry contexts | `src/sim/pathfinding/cell_entry.rs`, `src/sim/occupancy.rs`, `src/sim/pathfinding/core.rs` | Substrate queries must return/use `{terrain_layer, object_list_layer, occupancy_bits_layer}` and selected CellClass list heads, not a single movement layer. | A bridgehead edge where height becomes `Level+4` reads bridge occupancy bits even if an earlier branch chose/kept a separate object-list layer; proposed test: `test_can_enter_layers_split_object_list_from_occupancy_bits`. | Do not drive live blocker checks from one `MovementLayer` value. |

### Negative Facts / Do Not Do

- Do not implement `Can_Enter_Cell` as a pathgrid-only terrain filter. Evidence: Unit reads live `Cell+0xE4/+0xE8` object lists and `Cell+0x124/+0x128` occupancy bits; Active in YR: Yes.
- Do not treat vtable `+0x1B0` as a parent `Can_Enter_Cell` call. Evidence: it is `CheckBridgeTraversal @ 0x004D9C60`, returning only `0/7`; Active in YR: Yes.
- Do not merge parent/current-cell and arg5. Evidence: parent/current is forwarded to `CheckBridgeTraversal`, while arg5 gates `FootClass::LocomotorPassabilityCheck @ 0x004D9C10`; Active in YR: Yes.
- Do not infer runtime height from target layer or target cell. Evidence: `0x005F5F00` uses current cell level plus object `OnBridge ? 4 : 0`; Active in YR: Yes.
- Do not use one "bridge layer" to choose both object list and occupancy bits. Evidence: Unit has separate early list byte and later occupancy re-read; Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `docs/research/RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md` section 6 should replace the current-Rust row saying `movement_occupancy.rs` passes `parent: Some((parent,current_cell))`, `direction: 0` with:
  - "`src/sim/movement/movement_occupancy.rs:38-68` now defines a runtime tuple with actual direction, current effective height, `parent_current_cell=None`, and `arg5=1`; this matches the audited runtime null-parent shape. Remaining risk is generalizing the same context to A* and preserving persistent-vs-projected OnBridge timing in every movement caller."

## 10. Remaining Uncertainty

- Exact semantic name and producer lifecycle for the A* fifth-argument byte at `Pathfinder+0x08`; this report proves it is consumed as the locomotor-passability gate but does not audit Pathfinder construction/writers.
- Exact persistent-vs-projected `OnBridge` timing for every Rust multi-crossing runtime caller; this is a movement-tick trace question, not a substrate argument-shape question.
- Full sibling runtime matrix for Ship/Hover/Jumpjet is accepted from prior reports except for the sampled sites cited here; a verify-doc pass can re-audit the older matrix if a future implementation depends on one specific sibling branch.

## Status

COMPLETE for the scoped CellClass substrate runtime argument shape and bridge/list-layer inputs.

## Sources

- Ghidra decompile: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`.
- Ghidra assembly context: Unit `CheckBridgeTraversal` call `0x0073F2EB`, bridge occupancy re-read `0x0073F303..0x0073F348`, object list selection `0x0073F4F9..0x0073F51A`.
- Ghidra decompile: `CheckBridgeTraversal @ 0x004D9C60`.
- Ghidra decompile: `FootClass::LocomotorPassabilityCheck @ 0x004D9C10`.
- Ghidra decompile: `CellClass__Get_Effective_Height @ 0x005F5F00`.
- Ghidra assembly context: `AStar_main_loop @ 0x00429F54`, Drive runtime `0x004B1C3E`, Drive movement `0x004B2FF9`, Hover runtime `0x00515570`, Jumpjet landing `0x0054C66D`.
- Ghidra decompile contrast: `InfantryClass::Can_Enter_Cell @ 0x0051BF90`.
- Existing docs consulted: `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`, `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`, `docs/research/RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`, `docs/research/traces/RUNTIME_BRIDGE_ENTRY_NULL_PARENT_TRACE.md`.
- Rust scan: `src/sim/movement/movement_occupancy.rs`, `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_step.rs`.
