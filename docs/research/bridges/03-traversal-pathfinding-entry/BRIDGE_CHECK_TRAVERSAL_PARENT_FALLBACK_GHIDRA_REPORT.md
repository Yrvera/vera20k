# CheckBridgeTraversal Parent/Current-Cell Fallback - Ghidra Research Report

Report: BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md
Date: 2026-05-14
Scope: Focused audit of `CheckBridgeTraversal @ 0x004D9C60` caller argument 5: explicit parent/current cell versus `0`, and what Rust must preserve for bridge/pathfinding/locomotion parity.

## Summary

`CheckBridgeTraversal` does not treat parent/current-cell `0` as a simple synonym for "use the mover's current cell." It has two distinct fallback modes:

1. **Direction-valid + parent `0`:** reconstruct a predecessor cell from the candidate target cell and direction:

   ```text
   reconstructed_parent = candidate + DirectionOffset[(direction - 4) & 7]
   ```

   This is used by live runtime locomotion calls that pass parent/current-cell `0` with a real direction. For one-cell probes it usually reconstructs the mover's current cell. For lookahead/probe calls it reconstructs the immediate predecessor of the target edge, not necessarily the object's current occupied cell.

2. **Direction `-1`:** after the optional reconstruction work, the function ignores the parent/predecessor and uses candidate-only height seeding:

   ```text
   if (*height == -1 && candidate.flags & 0x100) {
       *height = candidate.level + 4;
   }
   return 0;
   ```

   This path does not run the directed bridgehead, diff, or slope checks. It is the path used by Hover `Push` and Jumpjet landing-style `Can_Enter_Cell(target, -1, -1, 0, 1)` calls.

Therefore Rust must model null-parent fallback separately from explicit-parent traversal. Collapsing parent `0` to "current entity cell" is wrong for lookahead edges and wrong for `direction == -1` calls.

## Prior-State Check

Relevant parent reports:

- `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- `BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`
- `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
- `BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`

Prior state row applied: targeted extension. Existing docs identified the bridge traversal helper and most height cases, but this audit resolves the parent/current-cell fallback wording and makes the runtime/A* distinction explicit.

Important correction: `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` section 1.1 labels `param_1` as source and `param_5` as destination. The assembly shows the opposite for the `Can_Enter_Cell` call chain: `param_1` is the candidate/target cell and `param_5` is the optional parent/current predecessor cell. `BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md` uses the corrected candidate/parent framing.

## Verified Signature

`CheckBridgeTraversal @ 0x004D9C60`, virtual slot `+0x1B0`, `RET 0x14`.

Binary-verified argument mapping:

```c
int CheckBridgeTraversal(
    CellClass* candidate,      // arg1, loaded into EDI
    int direction,             // arg2, loaded into EBX
    int* height_in_out,         // arg3, later loaded into EBP
    uint8_t* list_byte_out,     // arg4, used only for writing 1
    CellClass* parent_or_null   // arg5, loaded into ESI; may be 0
);
```

Entry evidence:

```asm
004d9c60  PUSH EBX
004d9c61  MOV  EBX,dword ptr [ESP + 0xc]   ; arg2 direction
004d9c65  PUSH EBP
004d9c66  PUSH ESI
004d9c67  MOV  ESI,dword ptr [ESP + 0x20]  ; arg5 parent/current cell
004d9c6b  PUSH EDI
004d9c6c  MOV  EDI,dword ptr [ESP + 0x14]  ; arg1 candidate cell
```

The helper is stdcall-style through the vtable slot, not a normal C++ thiscall method. The vtable dispatch receiver becomes stack arg1.

## Parent `0` Fallback, Direction-Valid

If arg5 is `0`, the helper computes a predecessor cell before checking whether direction is `-1`:

```asm
004d9c70  TEST ESI,ESI
004d9c72  JNZ  0x004d9cbc                  ; explicit parent supplied
004d9c74  LEA  EAX,[EBX + -0x4]
004d9c77  AND  EAX,0x7                     ; (direction - 4) & 7
004d9c7a  MOV  CX,word ptr [EAX*0x4 + 0x89f688]
004d9c82  MOV  DX,word ptr [EAX*0x4 + 0x89f68a]
004d9c8a  ADD  CX,word ptr [EDI + 0x24]    ; candidate.x + delta.x
004d9c8e  ADD  DX,word ptr [EDI + 0x26]    ; candidate.y + delta.y
...
004d9cb5  CALL 0x005657a0                  ; MapClass::Get_CellClass
004d9cba  MOV  ESI,EAX                     ; reconstructed parent/predecessor
```

Findings:

1. Parent `0` with valid direction is not "no parent." It is "derive the predecessor from candidate and direction."
2. The direction is rotated by 180 degrees with `(direction - 4) & 7`.
3. The reconstructed cell is based on the candidate's map coordinates at `candidate+0x24/+0x26`.
4. This can reconstruct the mover's current cell only when the candidate is the next adjacent cell and the direction is the actual movement direction.
5. For lookahead checks, the reconstructed parent is the immediate previous edge cell, not necessarily the unit's currently occupied cell.
6. If direction is invalid but not `-1` (for example `8`), parent `0` would still compute `(8 - 4) & 7 == 4`. Explicit parent avoids that reconstruction. This matters for any tube/tunnel-style path that uses non-0..7 direction values.

## Direction `-1` Branch

After the optional parent reconstruction, direction `-1` jumps to a candidate-only branch:

```asm
004d9cbc  CMP EBX,-0x1
004d9cbf  JZ  0x004d9e3e
...
004d9e3e  MOV EAX,dword ptr [ESP + 0x1c]   ; height pointer
004d9e42  CMP dword ptr [EAX],-0x1
004d9e45  JNZ 0x004d9e5e
004d9e47  MOV ECX,dword ptr [EDI + 0x140]  ; candidate flags
004d9e4d  TEST CH,0x1                      ; flags & 0x100
004d9e50  JZ  0x004d9e5e
004d9e52  MOVSX ECX,byte ptr [EDI + 0x11b] ; candidate level
004d9e59  ADD ECX,0x4
004d9e5c  MOV dword ptr [EAX],ECX          ; height = candidate.level + 4
004d9e5e  ...
004d9e61  XOR EAX,EAX                      ; return 0
```

Findings:

1. Direction `-1` ignores the explicit or reconstructed parent for traversal legality.
2. If `height != -1`, the branch returns OK without changing height.
3. If `height == -1` and the candidate has bridge flag `0x100`, it seeds `height = candidate.level + 4`.
4. It does not require candidate bridgehead flag `0x200`.
5. It does not run diff-0, diff-1 slope, or diff-4 bridge transition legality checks.
6. If parent was `0`, the function still performs a `MapClass::Get_CellClass` lookup using pseudo-direction `(-1 - 4) & 7 == 3` before this branch, but the result is not used by the direction `-1` branch. This appears to be wasted work, not traversal input.

This candidate-only mode is the key difference between `parent=0, direction=-1` and `parent=0, direction=valid`.

## Explicit Parent / Directed Traversal

When direction is not `-1` and a parent is available either explicitly or by reconstruction, the helper requires both `candidate` and `parent` to be non-null:

```asm
004d9cc5  TEST ESI,ESI      ; parent/predecessor
004d9cc7  JZ   ok_return
004d9ccd  TEST EDI,EDI      ; candidate
004d9ccf  JZ   ok_return
```

Then it performs directed height seeding and legality checks.

### Height `-1` Seed From Parent Bridge

```asm
004d9cd5  MOV  EBP,dword ptr [ESP + 0x1c]  ; height pointer
004d9cd9  CMP  dword ptr [EBP],-0x1
004d9cdd  JNZ  0x004d9d0e
004d9cdf  MOV  EAX,dword ptr [ESI + 0x140] ; parent flags
004d9ce5  TEST AH,0x1                      ; parent.flags & 0x100
004d9ce8  JZ   0x004d9d0e
004d9cea  MOVSX EDX,byte ptr [ESI + 0x11b] ; parent level
004d9cf1  ADD  EDX,0x4
004d9cf4  MOV  dword ptr [EBP],EDX         ; height = parent.level + 4
004d9cf7  MOV  EAX,dword ptr [EDI + 0x140] ; candidate flags
004d9cfd  TEST AH,0x2                      ; candidate.flags & 0x200
004d9d00  JNZ  0x004d9d0e
004d9d05  MOV  EAX,0x7                     ; blocked
```

Findings:

1. Directed `height == -1` seeding uses the parent/predecessor bridge cell, not the candidate bridge cell.
2. After seeding from a parent bridge, the candidate must be a bridgehead (`candidate.flags & 0x200`), or the helper returns `7`.
3. This bridgehead gate is absent from the direction `-1` candidate-only branch.
4. Parent `0` with valid direction enters this same logic after reconstruction.
5. A future Rust implementation cannot seed unknown height from the target in all cases; the source of the deck height depends on whether direction is `-1`.

### Directed Diff Calculation

```asm
004d9d0e  MOV    EAX,dword ptr [ESI + 0x140]
004d9d14  MOVSX  EBX,byte ptr [EDI + 0x11b] ; candidate.level
004d9d1b  AND    EAX,0x100                  ; parent bridge?
004d9d20  MOV    dword ptr [ESP + 0x1c],EAX
004d9d24  JZ     0x004d9d2f
004d9d26  MOVSX  ECX,byte ptr [ESI + 0x11b] ; if parent bridge: parent.level
004d9d2d  JMP    0x004d9d32
004d9d2f  MOV    ECX,dword ptr [EBP]        ; else: caller height
004d9d32  SUB    ECX,EBX                    ; diff = selected_parent_or_height - candidate.level
```

Findings:

1. If the parent/predecessor is a bridge cell, the diff compares `parent.level - candidate.level`.
2. If the parent/predecessor is not bridge, the diff compares `*height - candidate.level`.
3. The candidate level is signed (`MOVSX byte ptr [candidate+0x11B]`).
4. The directed diff is independent of whether parent was explicit or reconstructed; only the chosen predecessor cell matters.

### Directed Diff Cases

The helper permits only abs diff `0`, `1`, or `4`; other diffs return `7`.

Parent-sensitive details:

- `abs(diff) == 1`: if `diff > 0`, it tests candidate `+0x11C`; otherwise it tests parent `+0x11C`.
- `abs(diff) == 4`, first orientation: if `parent.level == candidate.level - 4`, then `*height` must equal `candidate.level` and the parent must be a bridge cell.
- `abs(diff) == 4`, opposite orientation: if `candidate.level == parent.level - 4`, then candidate must be bridge and bridgehead; writes `*list_byte_out = 1` and returns OK.
- `abs(diff) == 0`: may still return `7` when the path height contradicts the candidate/parent bridge state.

The parent/predecessor cell is therefore not optional decoration; it changes slope checks, bridgehead gates, and height comparisons.

## Caller Matrix: Parent `0` vs Explicit Parent

| Caller family | Representative callsite | `Can_Enter_Cell` parent arg | Direction | Height | `CheckBridgeTraversal` mode | Notes |
|---|---:|---:|---|---|---|---|
| A* neighbor expansion | `0x00429F54` | Explicit current node cell | Neighbor direction | `Pathfinder+0x30` current node/path height | Explicit-parent directed traversal | Avoids reconstruction; exact predecessor is passed. |
| Drive runtime movement | e.g. `0x004B1C3E`, `0x004B2B17`, `0x004B2FF9`, `0x004B34C0`, `0x004B4120` | `0` | Valid movement/lookahead direction | Current effective height | Null-parent directed reconstruction | Reconstructs the predecessor from target and direction. For lookahead, this is not necessarily object current cell. |
| Ship runtime movement | e.g. `0x006A1288`, `0x006A2167`, `0x006A2649`, `0x006A2B0F`, `0x006A374C` | `0` | Valid ship direction | Current effective height | Null-parent directed reconstruction | Mirrors Drive. |
| Hover normal movement | `0x00515570`, `0x005169AE` | `0` | Valid hover path/retry direction | Current effective height | Null-parent directed reconstruction | Active for standard YR hover units. |
| Hover Push branch | `0x00516E9B` | `0` | `-1` | `-1` | Direction `-1` candidate-only seed | Trigger conditional; if hit, skips directed parent checks. |
| Jumpjet landing/abort | `0x0054C66D`, `0x0054CE34` | `0` | `-1` | `-1` | Direction `-1` candidate-only seed | Active Jumpjet state family; more likely gameplay-relevant than Hover Push. |
| ZoneMap flood-fill helper | `0x00584271` | `0` | Flood direction | Candidate level | Null-parent directed reconstruction | Pathfinding support code, not runtime locomotion; still uses same parent fallback via Unit/Foot `Can_Enter_Cell`. |

## Unit/Infantry Can_Enter_Cell Binding

Unit and Infantry pass their own fourth `Can_Enter_Cell` argument through to `CheckBridgeTraversal` as arg5.

### Unit evidence, `0x0073F2EB`

```asm
0073f2c9  MOV  ESI,dword ptr [ESP + 0xa0]  ; Can_Enter_Cell arg4 parent/current
0073f2d0  MOV  EAX,dword ptr [EBX]
0073f2d2  LEA  ECX,[ESP + 0x13]            ; list byte pointer
0073f2d6  PUSH ESI                         ; CheckBridgeTraversal arg5 parent/current
0073f2d7  LEA  EDX,[ESP + 0xa0]            ; height pointer
0073f2de  PUSH ECX                         ; arg4 list byte pointer
0073f2df  MOV  ECX,dword ptr [ESP + 0x9c]  ; candidate target
0073f2e6  PUSH EDX                         ; arg3 height pointer
0073f2e7  PUSH EDI                         ; arg2 direction
0073f2e8  PUSH ECX                         ; arg1 candidate
0073f2eb  CALL dword ptr [EAX + 0x1b0]
```

### Infantry evidence, `0x0051C0E6`

```asm
0051c0d0  MOV  ECX,dword ptr [ESP + 0x44]  ; Can_Enter_Cell arg4 parent/current
0051c0d4  MOV  EAX,dword ptr [EBP]
0051c0d7  PUSH ECX                         ; CheckBridgeTraversal arg5 parent/current
0051c0d8  LEA  EDX,[ESP + 0x15]            ; list byte pointer
0051c0dc  LEA  ECX,[ESP + 0x44]            ; height pointer
0051c0e0  PUSH EDX                         ; arg4 list byte pointer
0051c0e1  PUSH ECX                         ; arg3 height pointer
0051c0e2  PUSH EDI                         ; arg2 direction
0051c0e3  PUSH ESI                         ; arg1 candidate
0051c0e6  CALL dword ptr [EAX + 0x1b0]
```

Findings:

1. `Can_Enter_Cell` does not replace parent `0` before calling `CheckBridgeTraversal`.
2. The null-parent fallback lives inside `CheckBridgeTraversal`.
3. Unit and Infantry both preserve the caller's parent/current-cell argument shape.
4. Parent/current-cell is arg4 of `Can_Enter_Cell`, but arg5 of `CheckBridgeTraversal` because `CheckBridgeTraversal` receives pointers for height and list byte.

## A* Explicit Parent Evidence

A* neighbor expansion calls Unit/Foot `Can_Enter_Cell` at `0x00429F54`.

```asm
00429f3a  MOV  ECX,dword ptr [ESP + 0x20]
00429f3e  MOV  EDI,dword ptr [ESP + 0x68]  ; moving Foot/Unit
00429f42  PUSH EAX                         ; Can_Enter_Cell arg5
00429f43  MOV  EAX,dword ptr [ECX]
00429f45  MOV  ECX,dword ptr [ESI + 0x30]  ; current node/path height
00429f48  MOV  EDX,dword ptr [EDI]
00429f4a  PUSH EAX                         ; arg4 explicit parent/current node cell
00429f4b  MOV  EAX,dword ptr [ESP + 0x20]  ; direction
00429f4f  PUSH ECX                         ; arg3 height
00429f50  PUSH EAX                         ; arg2 direction
00429f51  PUSH EBX                         ; arg1 candidate cell
00429f52  MOV  ECX,EDI
00429f54  CALL dword ptr [EDX + 0x1ac]
```

Findings:

1. A* passes an explicit parent/current node cell to `Can_Enter_Cell`.
2. A* also passes the current node/path height, not `-1` in normal neighbor expansion.
3. `CheckBridgeTraversal` therefore uses explicit-parent directed traversal for A*, not null-parent reconstruction.
4. This is the strongest binary evidence that A* and runtime movement should not be forced through one shared simplified "current cell" model.

## Runtime Parent `0` Evidence

Representative runtime locomotion call, Drive `Process_Drive_Track @ 0x004B1C3E`:

```asm
004b1ba7  PUSH 0x1            ; Can_Enter_Cell arg5
004b1ba9  PUSH 0x0            ; arg4 parent/current cell = null
...
004b1c24  CALL 0x005f5f00     ; current effective height
004b1c29  PUSH EAX            ; arg3 height
004b1c2e  PUSH EBX            ; arg2 direction
...
004b1c3d  PUSH EAX            ; arg1 target cell
004b1c3e  CALL dword ptr [EDI + 0x1ac]
```

Representative direction `-1` runtime call, Jumpjet landing `0x0054C66D`:

```asm
0054c662  PUSH 0x1            ; Can_Enter_Cell arg5
0054c664  PUSH 0x0            ; arg4 parent/current cell = null
0054c666  PUSH -0x1           ; arg3 height
0054c66a  PUSH -0x1           ; arg2 direction
0054c66c  PUSH EDI            ; arg1 target cell
0054c66d  CALL dword ptr [EAX + 0x1ac]
```

Findings:

1. Runtime movement/collision callsites commonly pass parent/current-cell `0`.
2. Direction-valid runtime calls rely on reconstruction from target plus direction.
3. Direction `-1` runtime calls rely on candidate-only height seeding and skip directed legality checks.
4. These are different behaviors despite both passing parent `0`.

## Rust Implications

Rust must preserve a binary-shaped bridge traversal input, not just a `(from_cell, to_cell, layer)` helper.

Required invariant:

```text
candidate_cell: required
direction: i32, including -1
height: i32 in/out, including -1
list_byte / bridge-list selector: mutable, can be forced to 1
parent_or_current_cell: Option<Cell>, where None is not the same as Some(current_cell)
arg5 traversal/context flag: preserved at Can_Enter_Cell layer
```

Specific rules:

1. `parent_or_current_cell = None` and `direction != -1` must reconstruct parent from the candidate and direction using `(direction - 4) & 7`.
2. `parent_or_current_cell = None` and `direction == -1` must not substitute the unit's current cell. It must use candidate-only height seeding.
3. `parent_or_current_cell = Some(parent)` and `direction != -1` must use the supplied parent, not recompute it.
4. `height == -1` with direction-valid traversal seeds from the parent bridge deck and requires candidate bridgehead.
5. `height == -1` with direction `-1` seeds from the candidate bridge deck and does not require bridgehead.
6. Runtime lookahead calls with parent `0` may reconstruct an intermediate predecessor cell. They must not be modeled as if the mover's current occupied cell was always the parent.
7. A* should keep explicit parent/current node cell behavior; this is not just an optimization over null-parent reconstruction.

Current Rust status from a read-only scan:

- `src/sim/pathfinding/core.rs` carries explicit parent and neighbor cells in A*, which is structurally compatible with explicit-parent traversal, but it does not expose the binary `Can_Enter_Cell(candidate, direction, height, parent, arg5)` shape.
- `src/sim/pathfinding/cell_entry.rs` takes a single `target_layer`, so it cannot express nullable parent fallback or `direction == -1` candidate-only height seeding.
- `src/sim/movement/movement_bridge.rs` models cell-boundary `on_bridge` state transitions, but it is not a replacement for `CheckBridgeTraversal` parent fallback semantics.

No Rust changes were made.

## Confidence

High confidence:

- `param_1` is candidate cell and `param_5` is optional parent/current predecessor cell.
- Parent `0` triggers reconstruction from candidate plus `(direction - 4) & 7` before direction `-1` handling.
- Direction `-1` uses candidate-only height seeding and returns OK without directed bridgehead/diff/slope checks.
- A* passes an explicit parent/current node cell at `0x00429F54`.
- Audited runtime locomotion callsites pass parent/current-cell `0`.

Medium confidence:

- Runtime lookahead semantic labels such as "intermediate predecessor" depend on surrounding locomotor control-flow interpretation, but the fallback formula itself is binary-verified.

Open:

- Whether any less-common non-locomotion caller passes `parent=0` with direction values outside `0..7` and reaches reconstruction. The ZoneMap flood-fill helper uses parent `0` with normal flood directions; no invalid-direction caller was confirmed in this pass.

## Recommended Next Audit

The clean next target is `Jumpjet height == -1 runtime landing audit`, because those calls use the candidate-only `direction == -1` branch and are likely more gameplay-relevant than Hover Push/Shove.

Questions to answer there:

1. What state exactly reaches `0x0054C66D` and `0x0054CE34`?
2. Is landing on bridge body cells allowed because `CheckBridgeTraversal` skips the bridgehead gate in direction `-1` mode?
3. Does Unit/Foot `Can_Enter_Cell` object-list/occupancy split add any later bridgehead-like restriction, or is candidate deck seeding sufficient?

## Sources

- Ghidra assembly, `gamemd.exe`:
  - `CheckBridgeTraversal @ 0x004D9C60`
  - Unit `Can_Enter_Cell` bridge traversal callsite `0x0073F2EB`
  - Infantry `Can_Enter_Cell` bridge traversal callsite `0x0051C0E6`
  - A* `Can_Enter_Cell` callsite `0x00429F54`
  - Drive runtime representative `0x004B1C3E`
  - Jumpjet landing representative `0x0054C66D`
  - Jumpjet abort/emergency landing representative `0x0054CE34`
  - ZoneMap flood-fill representative `0x00584271`
- Existing research docs:
  - `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
  - `BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`
  - `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
  - `BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`
  - `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`
