# Bridge Runtime Can_Enter_Cell Arguments - Ghidra Research Report

**Addresses:** `0x004B0F20` DriveLocomotionClass::Process_Drive_Track, `0x0075AEC0` WalkLocomotionClass::ProcessMovement, `0x005F5F00` Object/Cell effective-height helper, `0x0073F0A0` UnitClass::Can_Enter_Cell, `0x0051BF90` InfantryClass::Can_Enter_Cell, `0x004D9C60` CheckBridgeTraversal

**Confidence:** High for the exact runtime stack arguments at the two investigated call sites.

**Active in YR:** Yes. These functions are normal drive and walk locomotor runtime paths.

## 1. Overview

Runtime drive and walk locomotion call `Can_Enter_Cell` with five arguments, even though older decompiler notes often showed only three. The two hidden arguments are pushed early and survive across the height and target-cell computations.

For both investigated runtime call sites, the actual call shape is:

```text
Can_Enter_Cell(target_cell, direction, current_effective_height, 0, 1)
```

The important result is that runtime locomotion does **not** pass an explicit parent/current cell into `Can_Enter_Cell`. It passes `0`. `CheckBridgeTraversal` then computes the opposite neighbor itself using `(direction - 4) & 7`.

## 2. Relevant Calling Convention

`UnitClass::Can_Enter_Cell` and `InfantryClass::Can_Enter_Cell` return with `RET 0x14`, so callers provide five stack arguments:

```text
arg1 = candidate/target CellClass*
arg2 = direction
arg3 = height
arg4 = parent/current CellClass* for CheckBridgeTraversal, may be 0
arg5 = flags/context byte/dword used by later passability checks
```

Inside UnitClass, `arg4` is forwarded to `CheckBridgeTraversal`:

```text
0073F2C9  MOV ESI, [ESP+0xA0]      ; Unit arg4
0073F2D2  LEA ECX, [ESP+0x13]      ; list-byte pointer
0073F2D6  PUSH ESI                 ; CBT arg5 = parent/current cell pointer
0073F2D7  LEA EDX, [ESP+0xA0]      ; height pointer
0073F2DE  PUSH ECX                 ; CBT arg4 = list-byte pointer
0073F2E6  PUSH EDX                 ; CBT arg3 = &height
0073F2E7  PUSH EDI                 ; CBT arg2 = direction
0073F2E8  PUSH ECX                 ; CBT arg1 = candidate cell
0073F2EB  CALL [EAX+0x1B0]
```

Inside InfantryClass, the same forwarding pattern appears:

```text
0051C0D0  MOV ECX, [ESP+0x44]      ; Infantry arg4
0051C0D7  PUSH ECX                 ; CBT arg5
0051C0D8  LEA EDX, [ESP+0x15]      ; list-byte pointer
0051C0DC  LEA ECX, [ESP+0x44]      ; height pointer
0051C0E0  PUSH EDX
0051C0E1  PUSH ECX
0051C0E2  PUSH EDI                 ; direction
0051C0E3  PUSH ESI                 ; candidate cell
0051C0E6  CALL [EAX+0x1B0]
```

Therefore the runtime caller's fourth argument is the value that decides whether `CheckBridgeTraversal` receives an explicit parent cell or must derive one.

## 3. Effective Height Helper

The runtime call sites do not pass `-1` and do not pass the target cell's height. They call the helper at `0x005F5F00`.

Assembly:

```text
005F5F00  PUSH ESI
005F5F01  MOV ESI, ECX
005F5F03  MOV EAX, [ESI]
005F5F05  CALL [EAX+0x1BC]         ; get current cell for this object
005F5F0B  MOV CL, [ESI+0x8C]       ; ObjectClass::OnBridge
005F5F12  MOVSX EDX, byte [EAX+0x11B]
005F5F19  NEG CL
005F5F1B  SBB ECX, ECX
005F5F1D  AND ECX, 0x4
005F5F20  ADD ECX, EDX
005F5F22  MOV EAX, ECX
005F5F24  RET
```

Formula:

```text
current_effective_height = current_cell.level + (object.OnBridge ? 4 : 0)
```

Tiny details:

- `Cell+0x11B` is sign-extended (`MOVSX`).
- `Object+0x8C` is read directly as the bridge state.
- The bridge addend is exactly `4`.
- The helper reads the object's current cell via vtable `+0x1BC`; it does not inspect the candidate cell.

## 4. DriveLocomotionClass::Process_Drive_Track

### 4.1 Exact stack argument construction

The investigated runtime drive call is in track chaining / mid-track continuation.

Assembly around `0x004B1BA1-0x004B1C3E`:

```text
004B1BA1  MOV ECX, [EBP+0x0C]      ; moving Techno/Object
004B1BA4  LEA ESI, [EBP+0x40]      ; locomotor head-to / target coord base
004B1BA7  PUSH 0x1                 ; hidden Can_Enter_Cell arg5
004B1BA9  PUSH 0x0                 ; hidden Can_Enter_Cell arg4 = parent cell null
...
004B1BC4  MOV EAX, [ESI]
004B1BC6  MOV EDI, [direction_delta_x + (EBX&7)]
004B1BD4  ADD EAX, EDI
004B1BD6  MOV EDI, [direction_delta_y + (EBX&7)]
004B1BD9  MOV EDX, [ESI+0x4]
004B1BDC  ADD EDI, EDX
...
004B1C24  CALL 0x005F5F00          ; EAX = current_effective_height
004B1C29  PUSH EAX                 ; arg3 = current effective height
004B1C2A  LEA ECX, [ESP+0x40]      ; candidate cell coord
004B1C2E  PUSH EBX                 ; arg2 = direction
004B1C2F  PUSH ECX                 ; MapClass::Get_CellClass arg only
004B1C30  MOV ECX, 0x87F7E8
004B1C35  CALL 0x005657A0          ; EAX = target CellClass*
004B1C3A  MOV ECX, [EBP+0x0C]      ; this = moving Techno/Object
004B1C3D  PUSH EAX                 ; arg1 = target cell
004B1C3E  CALL [EDI+0x1AC]         ; Can_Enter_Cell
```

Because `MapClass::Get_CellClass` consumes only its own pushed coordinate pointer, the two early pushes remain below the later `height`, `direction`, and `target_cell` pushes.

Final stack at the virtual call:

```text
arg1 target_cell               = MapClass::Get_CellClass(head_to + DirectionDelta[direction & 7])
arg2 direction                 = EBX, the current movement/track direction
arg3 height                    = current_cell.level + (OnBridge ? 4 : 0)
arg4 parent/current cell       = 0
arg5 flags/context             = 1
```

### 4.2 Consequence for bridge traversal

Drive runtime collision does not provide a parent cell to `CheckBridgeTraversal`. The helper computes it from the target cell:

```text
if (parent_arg == 0) {
    fallback_coord = target_cell.coord + DirectionOffsets[(direction - 4) & 7]
    parent = MapClass::Get_CellClass(fallback_coord)
}
```

So the parent used by the bridge validator is not a caller-supplied object-current cell. It is an inferred opposite neighbor of the target.

For ordinary one-cell movement this should usually resolve to the cell being left. If track geometry, direction state, or the candidate coord is stale/misaligned, the binary still follows the inferred-neighbor behavior.

## 5. WalkLocomotionClass::ProcessMovement

### 5.1 Exact stack argument construction

The investigated walk call is the infantry runtime movement check before subcell selection / blocked handling.

Assembly around `0x0075B669-0x0075B690`:

```text
0075B669  MOV ECX, [EBP+0x0C]      ; moving Infantry/Object
0075B66C  PUSH 0x1                 ; hidden Can_Enter_Cell arg5
0075B66E  PUSH 0x0                 ; hidden Can_Enter_Cell arg4 = parent cell null
0075B670  MOV ESI, [ECX]           ; vtable
0075B672  CALL 0x005F5F00          ; EAX = current_effective_height
0075B677  MOV ECX, [ESP+0x1C]      ; direction local
0075B67B  PUSH EAX                 ; arg3 = current effective height
0075B67C  LEA EDX, [ESP+0x1C]      ; candidate cell coord local
0075B680  PUSH ECX                 ; arg2 = direction
0075B681  PUSH EDX                 ; MapClass::Get_CellClass arg only
0075B682  MOV ECX, 0x87F7E8
0075B687  CALL 0x005657A0          ; EAX = target CellClass*
0075B68C  MOV ECX, [EBP+0x0C]      ; this = moving Infantry/Object
0075B68F  PUSH EAX                 ; arg1 = target cell
0075B690  CALL [ESI+0x1AC]         ; Can_Enter_Cell
```

Final stack at the virtual call:

```text
arg1 target_cell               = MapClass::Get_CellClass(next infantry cell)
arg2 direction                 = current walk/path direction local
arg3 height                    = current_cell.level + (OnBridge ? 4 : 0)
arg4 parent/current cell       = 0
arg5 flags/context             = 1
```

### 5.2 Consequence for bridge traversal

Walk runtime has the same parent-cell behavior as drive runtime: explicit parent is null, and `CheckBridgeTraversal` derives the opposite neighbor from candidate cell plus `(direction - 4) & 7`.

Infantry also retains its class-specific early return:

```text
if (height - target_cell.level > 4) return 0;
```

That early return runs before `CheckBridgeTraversal`, so in high-over-low edge cases infantry can skip both the bridge validator and the two-pass occupancy/list logic.

## 6. CheckBridgeTraversal Fallback Confirmed

At `0x004D9C60`, `CheckBridgeTraversal` reads its fifth argument into `ESI`.

Assembly:

```text
004D9C67  MOV ESI, [ESP+0x20]      ; arg5 parent/current cell
004D9C6C  MOV EDI, [ESP+0x14]      ; arg1 candidate/target cell
004D9C70  TEST ESI, ESI
004D9C72  JNZ 0x004D9CBC
004D9C74  LEA EAX, [EBX-0x4]
004D9C77  AND EAX, 0x7
004D9C7A  MOV CX, [EAX*4+0x89F688]
004D9C82  MOV DX, [EAX*4+0x89F68A]
004D9C8A  ADD CX, [EDI+0x24]
004D9C8E  ADD DX, [EDI+0x26]
004D9CB5  CALL 0x005657A0          ; inferred parent/current cell
004D9CBA  MOV ESI, EAX
```

So runtime drive/walk calls with arg4=0 intentionally enter this fallback path.

## 7. Answer To The Research Question

### 7.1 Exact runtime arguments

For both `DriveLocomotionClass::Process_Drive_Track` and `WalkLocomotionClass::ProcessMovement`:

```text
target_cell = computed next cell
direction = current movement/path direction
height = current_cell.level + (object.OnBridge ? 4 : 0)
parent/current cell = 0
flags/context = 1
```

### 7.2 Does this make the two-pass split live during runtime movement?

Yes. Runtime movement/collision calls enter Unit/Infantry `Can_Enter_Cell`, which still:

1. Selects the object list from the pre-CBT bridge byte.
2. Calls `CheckBridgeTraversal`.
3. Selects bridge occupancy bits only if final `height == target_cell.level + 4`.

Runtime does not pass `height == -1`, so `CheckBridgeTraversal` normally does not seed the height. The height is the mover's current effective height.

Therefore a live runtime split can occur when:

```text
target_cell is a bridge cell
abs(current_effective_height - target_cell.level) > 1   ; list selects bridge
current_effective_height != target_cell.level + 4       ; occupancy bits remain ground
```

`CheckBridgeTraversal` can also force the list byte to bridge in its diff-4 bridgehead branch, while the post occupancy predicate still depends on the unchanged concrete height.

### 7.3 Parent-cell distinction from A*

A* passes an explicit parent/current cell to `Can_Enter_Cell`.

Runtime drive/walk does not. It passes zero, causing `CheckBridgeTraversal` to derive the parent from target cell and direction.

This means the two-pass mechanism is not A*-only, but the runtime geometry source differs:

```text
A*:      parent cell is caller-supplied from the current A* node
runtime: parent cell is inferred inside CheckBridgeTraversal from target + opposite(direction)
```

## 8. Current Rust Status

No Rust code was changed in this investigation.

Relevant Rust locations:

- `src/sim/movement/movement_step.rs:468` chooses `next_layer` from the path layer.
- `src/sim/movement/movement_step.rs:632-638` passes `next_layer` into deferred occupancy detection.
- `src/sim/movement/movement_occupancy.rs:38-73` checks blockers/subcells only on `next_layer`.
- `src/sim/movement/movement_occupancy.rs:175-177` passes the same `next_layer` into `classify_occupied_cell`.
- `src/sim/pathfinding/cell_entry.rs:91-131` uses a single `target_layer` for terrain and occupancy.
- `src/sim/pathfinding/cell_entry.rs:152-228` uses the same `target_layer` for crush, blocker lookup, and selected occupancy iteration.

Rust therefore currently cannot model runtime `Can_Enter_Cell` as:

```text
list layer = current_effective_height-vs-target precheck, possibly forced by CBT
occupancy bits layer = current_effective_height == target.level + 4
parent cell = inferred from target + opposite(direction)
```

## 9. Future Fidelity Invariants

This section is research guidance only.

1. Runtime drive/walk `Can_Enter_Cell` checks should use the mover's current effective height, not the destination layer from A*.
2. Current effective height means current cell level plus `4` only when persistent `OnBridge` is true.
3. Runtime drive/walk should pass no explicit parent cell to `CheckBridgeTraversal`; it should reproduce the fallback parent inference from target cell plus `(direction - 4) & 7`.
4. Runtime and A* should not be collapsed into a single bridge-parent model: A* passes parent explicitly, runtime infers it.
5. The two-pass list/occupancy split is live in runtime movement, not just path search.
6. Implementations should keep `arg5 = 1` semantics distinct from parent-cell semantics; the `0` parent and `1` context flag are two different stack arguments.

## 10. Open Questions

1. `DriveLocomotionClass::Process_Movement` also has `Can_Enter_Cell` sites outside `Process_Drive_Track`; those were not part of this target and should get a separate argument audit.
2. Ship, hover, jumpjet, and tunnel locomotors likely have their own runtime call shapes. They are not covered here.
3. The exact gameplay meaning of runtime `Can_Enter_Cell` arg5 `1` should be mapped across all call sites; this report only verifies the value.
4. A fidelity probe should test a deliberately misaligned runtime case where inferred parent differs from the object's actual current cell, to determine whether this fallback ever becomes player-visible outside edge/stuck states.

## Sources

- Ghidra live assembly/decompilation:
  - `0x004B0F20` DriveLocomotionClass::Process_Drive_Track
  - `0x0075AEC0` WalkLocomotionClass::ProcessMovement
  - `0x005F5F00` effective-height helper
  - `0x0073F0A0` UnitClass::Can_Enter_Cell
  - `0x0051BF90` InfantryClass::Can_Enter_Cell
  - `0x004D9C60` CheckBridgeTraversal
- Existing bridge reports:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`
- Rust status scan:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_step.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_occupancy.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/cell_entry.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/core.rs`

