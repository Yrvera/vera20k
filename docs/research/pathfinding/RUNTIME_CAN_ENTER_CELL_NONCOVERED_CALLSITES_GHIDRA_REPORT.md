# Runtime Can_Enter_Cell Noncovered Callsites - Ghidra Research Report

**Address(es):** `0x004B2B17`, `0x004B2FF9`, `0x004B34C0`, `0x004B4120`, `0x006A1288`, `0x006A2167`, `0x006A2649`, `0x006A2B0F`, `0x006A374C`, `0x00515570`, `0x005169AE`, `0x00516E9B`, `0x0054C66D`, `0x0054CE34`
**Investigation Mode:** coverage-map
**Claimed Scope:** Runtime locomotion/collision `Can_Enter_Cell` callsites left open by `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`, with emphasis on Drive `Process_Movement`, Ship mirrors, Hover, Jumpjet, and low-bridge/tube relevance.
**Non-Scope:** A* neighbor expansion except for contrast; full `UnitClass::Can_Enter_Cell` return-code tree; complete Hover Push/Shove external caller provenance; complete Jumpjet abort-flag writer audit; full low-bridge tube producer matrix.
**Confidence:** High for the stack argument tuples cited from existing Ghidra assembly reports; Medium for exact gameplay frequency of Hover Push and Jumpjet state 5; Low for any richer semantic name of `arg5` beyond the verified constant value `1`.
**Active in YR:** Yes / Conditional. Drive, Ship, normal Hover, and Jumpjet locomotors are active in standard YR. Hover Push/Shove and Jumpjet abort/emergency branches are conditional live paths.

## 1. Overview

The previously open runtime callsite matrix is now mostly covered by existing Ghidra reports. All audited live runtime locomotion/collision `Can_Enter_Cell` callsites pass five arguments, and none of the audited runtime callsites supplies a nonzero explicit parent/current cell. Direction-valid Drive, Ship, and normal Hover sites pass current effective height with parent `0`; Hover Push and Jumpjet landing/abort sites pass `direction = -1`, `height = -1`, parent `0`.

The Rust-facing hazard is not just layer selection. Runtime parent `0` is a binary-visible mode: direction-valid calls make `CheckBridgeTraversal` reconstruct the predecessor from `target + DirectionOffset[(direction - 4) & 7]`, while direction `-1` uses candidate-only bridge height seeding and skips directed bridgehead/diff/slope checks.

## 2. Class Layout / Key Offsets

| Field / Slot | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Unit/Foot vtable `+0x1AC` | `Can_Enter_Cell`, returns 0-7 | `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`; `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` | Yes |
| Unit/Foot vtable `+0x1B0` | `CheckBridgeTraversal` dispatch from `Can_Enter_Cell` | `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`, `0x004D9C60` | Yes |
| `Can_Enter_Cell` arg4 | Optional parent/current cell forwarded into `CheckBridgeTraversal` arg5 | Unit `0x0073F2EB`; Infantry `0x0051C0E6` | Yes |
| `Can_Enter_Cell` arg5 | Verified runtime value `1` at audited callsites; exact internal meaning not decoded beyond context/traversal flag | Runtime matrix callsites | Yes |
| `CellClass+0x11B` | Signed cell level | effective-height helper `0x005F5F00`; bridge traversal docs | Yes |
| `ObjectClass+0x8C` | Persistent `OnBridge` byte used by current effective height | effective-height helper `0x005F5F00` | Yes |
| `CellClass+0x140 & 0x100` | Structural bridge cell flag | bridge traversal docs; Hover/Jumpjet reports | Yes |
| `CellClass+0x116` | Low bridge / tube index | `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` | Yes |

## 3. Core Logic

### Runtime callsite matrix

| Function | Callsite(s) | Target cell | Direction | Height | Parent/current | Arg5 | Active in YR |
|---|---:|---|---|---|---:|---:|---|
| Drive `Process_Drive_Track` | `0x004B1C3E` | Next track cell | valid track direction | current effective height | `0` | `1` | Yes |
| Drive `Process_Movement` scatter/probe | `0x004B2B17` | adjacent probe cell | timer/random octant | current effective height via `0x005F5F00` | `0` | `1` | Yes |
| Drive `Process_Movement` next path cell | `0x004B2FF9` | next path-queue cell | path direction | current effective height via `0x005F5F00` | `0` | `1` | Yes |
| Drive `Process_Movement` lookahead | `0x004B34C0` | next-next/lookahead cell | lookahead direction | saved current effective height | `0` | `1` | Yes |
| Drive `Process_Movement` late lookahead | `0x004B4120` | secondary next-next cell | late direction | saved current effective height | `0` | `1` | Yes |
| Ship `Process_Drive_Track` | `0x006A1288` | ship next track cell | valid ship direction | current effective height via `0x005F5F00` | `0` | `1` | Yes |
| Ship `Process_Movement` mirrors | `0x006A2167`, `0x006A2649`, `0x006A2B0F`, `0x006A374C` | ship mirrors of Drive sites | valid ship directions | current/saved effective height | `0` | `1` | Yes |
| Hover normal movement | `0x00515570`, `0x005169AE` | normal hover next/probe cell | valid hover direction | current effective height via `0x005F5F00` | `0` | `1` | Yes |
| Hover Push/Shove | `0x00516E9B` | current cell plus push-facing octant | `-1` | `-1` | `0` | `1` | Conditional |
| Jumpjet landing state 4 | `0x0054C66D` | destination/candidate cell | `-1` | `-1` | `0` | `1` | Conditional, standard Jumpjet state |
| Jumpjet abort/emergency state 5 | `0x0054CE34` | current/recovery candidate cell | `-1` | `-1` | `0` | `1` | Conditional, standard Jumpjet state |

### Current effective height

Runtime Drive/Ship/normal Hover calls do not pass target height and do not pass `-1`. The helper at `0x005F5F00` computes:

```text
current_effective_height = current_cell.level + (object.OnBridge ? 4 : 0)
```

Active in YR: Yes. Evidence: `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`, helper assembly at `0x005F5F00`.

### Parent `0` behavior

When `Can_Enter_Cell` forwards parent/current `0`, `CheckBridgeTraversal @ 0x004D9C60` does not substitute the mover's actual current cell. For valid directions it reconstructs:

```text
parent = target_cell + DirectionOffset[(direction - 4) & 7]
```

For `direction = -1`, it uses the candidate-only branch:

```text
if height == -1 and target_cell has bridge flag 0x100:
    height = target_cell.level + 4
return allowed
```

Active in YR: Yes. Evidence: `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`, `0x004D9C60`.

### Arg5

Every audited runtime locomotion/collision callsite pushes `1` as `Can_Enter_Cell` arg5. This report did not find a live runtime locomotion/collision site with another value, and did not decode a richer consumer meaning. Treat `arg5` as an explicit binary argument that must be preserved, not as parent/current-cell presence and not as a layer enum.

Active in YR: Yes for value `1`; semantic beyond that remains uncertain. Evidence: runtime matrix callsites above.

## 4. INI Keys

| Source | Key / section | Relevance | Active in YR |
|---|---|---|---|
| `ini/rulesmd.ini` | `Locomotor={4A582741-...}` | Drive locomotor is used by standard ground vehicles | Yes |
| `ini/rulesmd.ini:7114`, `7166`, `7236`, `7290`, `7987`, `8037`, `8095`, `8161` | `Locomotor={2BEA74E1-...}` | Ship locomotor is used by standard naval units | Yes |
| `ini/rulesmd.ini:7056`, `7453`, `7933`, `8918` | `Locomotor={4A582742-...}` | Hover locomotor used by hover units | Yes |
| `ini/rulesmd.ini:3948`, `4740`, `8725`, `10553`, `10852`, `10913`, `11181`, `11259`, `27300` | `Locomotor={92612C46-...}` | Jumpjet locomotor used by standard YR jumpjet/balloon-hover units | Yes |
| `rulesmd.ini` / `rules.ini` low bridge overlay families | Low bridge overlays are not enough by themselves; live predicate also needs tube index and `LandType == 10` | Yes |

## 5. Integration Points

- Unit and Infantry `Can_Enter_Cell` preserve caller arg4 and forward it to `CheckBridgeTraversal`. Active in YR: Yes. Evidence: Unit `0x0073F2EB`, Infantry `0x0051C0E6`.
- A* passes an explicit parent/current node cell and current node/path height. Active in YR: Yes. Evidence: A* `0x00429F54` in `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`.
- Runtime Drive/Ship/Hover valid-direction movement passes parent `0`, causing fallback parent reconstruction. Active in YR: Yes. Evidence: runtime matrix callsites.
- Hover Push/Shove and Jumpjet landing/abort pass `direction=-1`, `height=-1`, parent `0`, causing candidate-only seed. Active in YR: Conditional. Evidence: `BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`; `BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`.
- Low bridge/tube movement is related but not a runtime `Can_Enter_Cell` callsite in this slice. Active in YR: Yes. Evidence: `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`.

## 6. Current Rust Implementation Status

Read-only scan only; no Rust files were changed.

| Surface | Current observed shape | Rust-facing implication |
|---|---|---|
| `src/sim/movement/movement_occupancy.rs:38` | `resolve_runtime_can_enter_layers` accepts current and next cells plus `next_layer` and `path_height` | It now models split layers, but it supplies an explicit parent/current cell to bridge traversal. |
| `src/sim/movement/movement_occupancy.rs:63-73` | Calls `check_bridge_traversal` with `direction: 0` and `parent: Some((parent, current_cell))` | Mismatch with audited runtime callsites, which pass parent `0` and valid direction, or `-1`/`-1`. |
| `src/sim/movement/movement_step.rs:634-640` | Passes `position.z` into runtime layer resolution | Potentially compatible with current effective height if `position.z` is the persistent current level, but the OnBridge helper formula should be explicitly preserved. |
| `src/sim/pathfinding/core.rs:265-372` | `check_bridge_traversal` and `can_enter_layer_context` already support nullable parent, `direction == -1`, and height `-1` | The core helper has much of the right shape; the runtime caller is not yet using the audited runtime tuple. |
| `src/sim/pathfinding/cell_entry.rs:97-110`, `331+` | Supports `CanEnterLayerContext` split for terrain/list/occupancy bits | Good direction, but does not by itself preserve callsite tuple semantics or `arg5`. |
| `src/sim/movement/tube_movement.rs` | Low-bridge tube movement exists separately | This should not be collapsed into ordinary runtime `Can_Enter_Cell` layer checks. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Drive `Process_Movement` runtime callsites | verified | `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`, `0x004B2B17`, `0x004B2FF9`, `0x004B34C0`, `0x004B4120` | none for stack tuple |
| Ship `Process_Movement` and `Process_Drive_Track` mirrors | verified | same matrix, `0x006A1288`, `0x006A2167`, `0x006A2649`, `0x006A2B0F`, `0x006A374C` | none for stack tuple |
| Hover normal movement | verified | same matrix, `0x00515570`, `0x005169AE` | none for stack tuple |
| Hover Push/Shove `height=-1` site | verified | `BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`, `0x00516E9B` | external standard-YR caller provenance |
| Jumpjet state 4 landing | verified | `BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`, `0x0054C66D` | per-unit frequency |
| Jumpjet state 5 abort/emergency | touched-not-exhausted | same report, `0x0054CE34` | writers of object bytes `+0x425`, `+0x427`, `+0x6AD` |
| Arg5 value across audited runtime callsites | verified | all runtime matrix rows push `1` | richer semantic meaning remains unresolved |
| A* contrast | verified | `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`, `0x00429F54` | no A* redo needed |
| Low-bridge TubeClass runtime movement | touched-not-exhausted | `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` | exact producers of unit/infantry `+0x684` not part of this slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Does Drive Process_Movement outside Process_Drive_Track pass explicit parent/current? -> No; all four audited sites pass parent/current 0 and arg5 1.` (evidence: `0x004B2B17`, `0x004B2FF9`, `0x004B34C0`, `0x004B4120`)
- `[RESOLVED] OQ-2 - Does Ship mirror Drive call shapes? -> Yes for the audited bridge-sensitive sites.` (evidence: `0x006A1288`, `0x006A2167`, `0x006A2649`, `0x006A2B0F`, `0x006A374C`)
- `[RESOLVED] OQ-3 - Do normal Hover path-step calls use current effective height? -> Yes, with parent 0 and arg5 1.` (evidence: `0x00515570`, `0x005169AE`)
- `[RESOLVED] OQ-4 - Does Hover have a direction/height -1 runtime site? -> Yes, in Hover Push/Shove adjacent-cell validation.` (evidence: `0x00516E9B`)
- `[RESOLVED] OQ-5 - Do Jumpjet landing/abort calls use direction/height -1? -> Yes, both audited state 4 and state 5 sites do.` (evidence: `0x0054C66D`, `0x0054CE34`)
- `[RESOLVED] OQ-6 - Are these paths active in YR? -> Drive/Ship/normal Hover are active; Hover Push and Jumpjet abort are conditional; Jumpjet landing is part of the standard Jumpjet state family.` (evidence: `rulesmd.ini` locomotor declarations and cited Ghidra reports)
- `[RESOLVED] OQ-7 - Should runtime parent 0 be replaced by object current cell? -> No; `CheckBridgeTraversal` reconstructs from target plus opposite direction, or uses candidate-only mode for direction -1.` (evidence: `0x004D9C60`)
- `[RESOLVED] OQ-8 - Is A* parent behavior the same as runtime? -> No; A* passes an explicit parent/current node cell.` (evidence: `0x00429F54`)
- `[RESOLVED] OQ-9 - Does the current Rust core helper have nullable-parent and direction -1 concepts? -> Yes in `src/sim/pathfinding/core.rs:265-372`.` (evidence: source scan)
- `[RESOLVED] OQ-10 - Does the current Rust runtime caller use null-parent fallback? -> No; `movement_occupancy.rs` passes `parent: Some` and `direction: 0`.` (evidence: `src/sim/movement/movement_occupancy.rs:63-73`)
- `[DEFERRED] OQ-11 - What is the exact semantic name of `arg5`?` (category: `requires-different-system-context`; reason: all audited runtime locomotion sites use value `1`, so callsite contrast cannot decode it; next-step-if-pursued: audit Unit/Infantry `Can_Enter_Cell` reads of its fifth argument and find non-1 callers)
- `[DEFERRED] OQ-12 - Which external standard-YR caller invokes Hover Push/Shove most often?` (category: `requires-different-system-context`; reason: branch tuple is verified but external virtual-call provenance is not exhaustive; next-step-if-pursued: ILocomotion slot `+0x68/+0x6C` provenance scan or runtime breakpoint)
- `[DEFERRED] OQ-13 - Which writers arm Jumpjet state 5 abort/emergency landing?` (category: `requires-different-system-context`; reason: state 5 tuple verified, writer matrix separate; next-step-if-pursued: `+0x425/+0x427/+0x6AD` writer audit)
- `[DEFERRED] OQ-14 - Which producer writes low-bridge tube active state into unit/infantry `+0x684`?` (category: `out-of-scope`; reason: TubeClass movement is related to ground/bridge parity but not this runtime `Can_Enter_Cell` callsite matrix; next-step-if-pursued: low-bridge tube producer audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Runtime Drive/Ship/Hover valid-direction calls pass parent/current `0`, not explicit current cell; `CheckBridgeTraversal` reconstructs predecessor from target plus `(direction - 4) & 7`. | `0x004B2B17`, `0x004B2FF9`, `0x004B34C0`, `0x004B4120`, `0x006A2167`, `0x00515570`, `0x004D9C60` | mismatch: runtime Rust passes `parent: Some((parent,current_cell))`, `direction: 0` | `src/sim/movement/movement_occupancy.rs`, `src/sim/movement/movement_step.rs`, `src/sim/pathfinding/core.rs` | Runtime entry checks must carry actual movement/lookahead direction and `parent=None` so the core fallback reconstructs from target. | A two-cell lookahead over a bridge edge validates against the predecessor of the probed target, not always the mover's occupied cell. Proposed test: `test_runtime_can_enter_cell_null_parent_reconstructs_lookahead_parent`. | Do not collapse A* explicit-parent semantics and runtime null-parent semantics. |
| Runtime current height is current cell level plus `4` only when persistent `OnBridge` is true; it is not target layer height. | helper `0x005F5F00`; runtime matrix | partially unchecked: Rust passes `position.z`, but the exact helper formula should be explicit at runtime call construction | `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_occupancy.rs` | Build runtime `path_height` from current effective height, not from `next_layer`; keep OnBridge persistent-state addend. | A unit leaving a bridge uses current deck height for the entry check even when target path layer is ground. Proposed test: `test_runtime_can_enter_cell_uses_current_height_not_next_layer`. | Do not infer height from A* path layer during runtime collision. |
| Hover Push and Jumpjet landing/abort call `Can_Enter_Cell(target, -1, -1, 0, 1)`, which uses candidate-only bridge height seeding. | Hover `0x00516E9B`; Jumpjet `0x0054C66D`, `0x0054CE34`; `CheckBridgeTraversal 0x004D9C60` | missing/unchecked: Rust movement/air/jumpjet paths do not expose this tuple through runtime `Can_Enter_Cell` | `src/sim/movement/jumpjet_movement.rs`, `src/sim/movement/air_movement.rs`, future Hover Push/Shove surface, `src/sim/pathfinding/core.rs` | Support `direction=-1`, `height=-1`, `parent=None`, `arg5=1`; seed bridge height from candidate, not parent/current. | A jumpjet emergency descent crossing a bridge-deck plane asks candidate bridge cell entry before ground altitude. Proposed test: `test_jumpjet_landing_can_enter_cell_candidate_seed_height_minus_one`. | Do not pass the computed push octant as `Can_Enter_Cell` direction for Hover Push; binary passes `-1`. |

### Negative Facts / Do Not Do

- Do not treat runtime parent `0` as `Some(current_cell)`. Evidence: `CheckBridgeTraversal @ 0x004D9C60` reconstructs from candidate plus `(direction - 4) & 7`; Active in YR: Yes.
- Do not collapse A* and runtime parent semantics. Evidence: A* `0x00429F54` pushes explicit parent/current node; runtime Drive/Ship/Hover callsites push `0`; Active in YR: Yes.
- Do not pass target/next layer height for Drive/Ship/normal Hover runtime entry checks. Evidence: helper `0x005F5F00` computes current effective height; Active in YR: Yes.
- Do not pass Hover Push's computed octant as the `Can_Enter_Cell` direction. Evidence: `0x00516E9B` pushes direction `-1`; the octant only selects the adjacent target cell; Active in YR: Conditional.
- Do not implement low bridges as ordinary road terrain or use TubeClass movement as a substitute for runtime `Can_Enter_Cell`. Evidence: `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`, `CellClass+0x116` plus `LandType==10`; Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md` section 10 should replace:
  - "`DriveLocomotionClass::Process_Movement` also has `Can_Enter_Cell` sites outside `Process_Drive_Track`; those were not part of this target and should get a separate argument audit.`"
  - with: "`DriveLocomotionClass::Process_Movement` outside `Process_Drive_Track` has four audited runtime sites (`0x004B2B17`, `0x004B2FF9`, `0x004B34C0`, `0x004B4120`); each passes `(target, direction, current_effective_height, parent/current=0, arg5=1)`."
- Same section should replace:
  - "`Ship, hover, jumpjet, and tunnel locomotors likely have their own runtime call shapes. They are not covered here.`"
  - with: "`Ship mirrors Drive for the audited bridge-sensitive runtime sites; normal Hover uses `(target, direction, current_effective_height, 0, 1)`; Hover Push and Jumpjet landing/abort use `(target, -1, -1, 0, 1)`. Low-bridge TubeClass movement is separate from this runtime `Can_Enter_Cell` callsite matrix.`"
- Same section should replace:
  - "`The exact gameplay meaning of runtime `Can_Enter_Cell` arg5 `1` should be mapped across all call sites; this report only verifies the value.`"
  - with: "`All audited runtime locomotion/collision callsites still pass `arg5=1`; no richer semantic meaning is proven by callsite contrast. Preserve it as an explicit argument and do not conflate it with parent/current-cell presence or layer selection.`"

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_occupancy.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_step.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/core.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/cell_entry.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/tube_movement.rs`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
