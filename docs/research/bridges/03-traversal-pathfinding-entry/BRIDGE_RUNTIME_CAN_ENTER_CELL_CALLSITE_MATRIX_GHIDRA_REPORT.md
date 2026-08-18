# Bridge Runtime Can_Enter_Cell Callsite Matrix Ghidra Report

Date: 2026-05-14
Scope: Remaining live runtime Unit/Infantry/Foot `Can_Enter_Cell` callsites related to locomotion/collision after the previously covered Drive `Process_Drive_Track` and Walk `ProcessMovement` sites.
Primary binary: `gamemd.exe`.

This report extends:
- `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`
- `BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`
- `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
- `SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md`
- `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`
- `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`
- `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`
- `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`

## Executive Summary

Binary assembly confirms that all audited live Drive and Ship runtime movement/collision `Can_Enter_Cell` calls pass `parent/current cell = 0` and `arg5 = 1`.

Drive and Ship `Process_Movement` do not pass `height == -1`. Their bridge-sensitive runtime checks use the current effective height, either by direct call to `CellClass` effective-height helper `0x005F5F00` or by an inline/saved equivalent value. That value is the current cell level plus the object's persistent `ObjectClass+0x8C OnBridge` deck offset.

Ship mirrors Drive for the bridge-sensitive argument shape in both `Process_Drive_Track` and `Process_Movement`.

The remaining live runtime exceptions are Hover and Jumpjet placement/landing-style checks. Hover has two normal path-step checks using current effective height, plus one live hover placement/settle check that passes `direction = -1` and `height = -1`. Jumpjet landing and abort/emergency landing checks also pass `direction = -1` and `height = -1`.

No audited live runtime locomotion/collision callsite passes an explicit parent/current-cell argument. All audited runtime callsites pass parent/current-cell as `0`, so `CheckBridgeTraversal` must be allowed to use its binary fallback behavior.

## Verified Callsite Matrix

| Function | Callsite | Target arg | Direction arg | Height arg / source | Parent/current-cell arg | Arg5 | Live YR confidence | Notes |
|---|---:|---|---|---|---|---:|---|---|
| `DriveLocomotionClass::Process_Drive_Track` | `0x004B1C3E` | Next drive-track cell from current/head cell plus direction delta | Current track direction (`EBX`) | `0x005F5F00` current effective height | `0` | `1` | High | Already covered by prior runtime report; retained as baseline. |
| `ShipLocomotionClass::Process_Drive_Track` | `0x006A1288` | Next ship-track cell from current/head cell plus direction delta | Current track direction | `0x005F5F00` current effective height | `0` | `1` | High for ship locomotor path | Exact bridge-sensitive argument shape mirrors Drive. |
| `DriveLocomotionClass::Process_Movement` | `0x004B2B17` | Adjacent scatter/probe cell derived from current cell and timer/random octant | Timer/random octant (`ESI`) | `0x005F5F00` current effective height | `0` | `1` | High | Precedes scatter/repath branch documented near `0x004B2DC0`. |
| `ShipLocomotionClass::Process_Movement` | `0x006A2167` | Ship mirror of Drive `0x004B2B17` probe target | Same corresponding octant | `0x005F5F00` current effective height | `0` | `1` | High for ship locomotor path | Mirrors Drive argument shape. |
| `DriveLocomotionClass::Process_Movement` | `0x004B2FF9` | Next path-queue cell | Current path direction from `FootClass+0x5E0` | `0x005F5F00` current effective height | `0` | `1` | High | Precedes branch documented near `0x004B327D`. |
| `ShipLocomotionClass::Process_Movement` | `0x006A2649` | Ship next path-queue cell | Current path direction from same Foot field family | `0x005F5F00` current effective height | `0` | `1` | High for ship locomotor path | Mirrors Drive argument shape. |
| `DriveLocomotionClass::Process_Movement` | `0x004B34C0` | Lookahead/next-next cell | Lookahead direction (`EBX`) | Inline/saved current effective height (`EDI`) | `0` | `1` | High | Height is not recomputed by an immediate helper call at the callsite, but is the same current effective-height value. |
| `ShipLocomotionClass::Process_Movement` | `0x006A2B0F` | Ship lookahead/next-next cell | Lookahead direction | Inline/saved current effective height | `0` | `1` | High for ship locomotor path | Mirrors Drive argument shape. |
| `DriveLocomotionClass::Process_Movement` | `0x004B4120` | Late/secondary next-next cell | Late/secondary direction (`ESI`) | Saved current effective height (`ECX` local at push) | `0` | `1` | High | Precedes branch documented near `0x004B4437`. |
| `ShipLocomotionClass::Process_Movement` | `0x006A374C` | Ship late/secondary next-next cell | Late/secondary direction | Saved current effective height | `0` | `1` | High for ship locomotor path | Mirrors Drive argument shape. |
| `HoverLocomotionClass` active path-step routine | `0x00515570` | Normal hover next/path-step cell | Path-step direction | `0x005F5F00` current effective height | `0` | `1` | High | Hover locomotor is active in standard YR for hover units. |
| `HoverLocomotionClass` retry/scatter branch | `0x005169AE` | Hover retry/probe cell | Retry/path direction | `0x005F5F00` current effective height | `0` | `1` | Medium-high | Same normal bridge-sensitive shape as Drive/Ship/Walk. |
| `HoverLocomotionClass` placement/settle branch | `0x00516E9B` | Computed desired/settle target cell | `-1` | `-1` | `0` | `1` | Medium-high | Live hover code path; exact gameplay trigger is less fully characterized than normal path-step checks. |
| `JumpjetLocomotionClass` landing state | `0x0054C66D` | Landing candidate cell (`EDI`) | `-1` | `-1` | `0` | `1` | High | State 4 landing path in active Jumpjet locomotor. |
| `JumpjetLocomotionClass` abort/emergency landing state | `0x0054CE34` | Alternate/emergency landing cell (`ESI`) | `-1` | `-1` | `0` | `1` | Medium-high | Active Jumpjet state handler, conditional on abort/emergency landing flow. |

## Representative Assembly Evidence

### Drive Process_Drive_Track baseline, `0x004B1C3E`

```asm
004b1ba7  PUSH 0x1            ; arg5
004b1ba9  PUSH 0x0            ; arg4 parent/current cell
...
004b1c24  CALL 0x005f5f00     ; current effective height
004b1c29  PUSH EAX            ; arg3 height
004b1c2e  PUSH EBX            ; arg2 direction
004b1c2f  PUSH ECX            ; coordinate pointer for MapClass lookup, not Can_Enter_Cell arg
004b1c35  CALL 0x005657a0     ; MapClass::Get_CellClass
004b1c3d  PUSH EAX            ; arg1 target cell
004b1c3e  CALL dword ptr [EDI+0x1ac]
```

### Ship Process_Drive_Track mirror, `0x006A1288`

```asm
006a11e8  PUSH 0x1            ; arg5
006a11ea  PUSH 0x0            ; arg4 parent/current cell
...
006a126a  CALL 0x005f5f00     ; current effective height
006a1273  PUSH EAX            ; arg3 height
006a1278  PUSH ECX            ; arg2 direction
006a1279  PUSH EDX            ; coordinate pointer for MapClass lookup
006a127f  CALL 0x005657a0     ; MapClass::Get_CellClass
006a1287  PUSH EAX            ; arg1 target cell
006a1288  CALL dword ptr [EDI+0x1ac]
```

### Drive Process_Movement helper-height site, `0x004B2B17`

```asm
004b2af6  PUSH 0x1            ; arg5
004b2af8  PUSH 0x0            ; arg4 parent/current cell
004b2afd  CALL 0x005f5f00     ; current effective height
004b2b02  PUSH EAX            ; arg3 height
004b2b07  PUSH ESI            ; arg2 direction
004b2b08  PUSH EDX            ; coordinate pointer for MapClass lookup
004b2b0e  CALL 0x005657a0     ; MapClass::Get_CellClass
004b2b16  PUSH EAX            ; arg1 target cell
004b2b17  CALL dword ptr [EDI+0x1ac]
```

### Drive Process_Movement path-queue site, `0x004B2FF9`

```asm
004b2fd0  PUSH 0x1            ; arg5
004b2fd2  PUSH 0x0            ; arg4 parent/current cell
004b2fd6  CALL 0x005f5f00     ; current effective height
004b2fde  PUSH EAX            ; arg3 height
004b2fe3  MOV  EDX,dword ptr [ECX+0x5e0]
004b2fee  PUSH EDX            ; arg2 direction
004b2fef  PUSH EAX            ; coordinate pointer for MapClass lookup
004b2ff0  CALL 0x005657a0     ; MapClass::Get_CellClass
004b2ff8  PUSH EAX            ; arg1 target cell
004b2ff9  CALL dword ptr [ESI+0x1ac]
```

### Drive Process_Movement inline/saved-height site, `0x004B34C0`

```asm
004b34b7  PUSH 0x1            ; arg5
004b34b9  PUSH 0x0            ; arg4 parent/current cell
004b34bb  PUSH EDI            ; arg3 height, saved current effective height
004b34bc  MOV  EAX,dword ptr [ECX]
004b34be  PUSH EBX            ; arg2 direction
004b34bf  PUSH ESI            ; arg1 target cell
004b34c0  CALL dword ptr [EAX+0x1ac]
```

### Drive Process_Movement late/saved-height site, `0x004B4120`

```asm
004b40ac  PUSH 0x1            ; arg5
004b40ae  PUSH 0x0            ; arg4 parent/current cell
...
004b40f9  PUSH ECX            ; arg3 saved current effective height
...
004b410a  PUSH ESI            ; arg2 direction
004b4111  PUSH EDX            ; coordinate pointer for MapClass lookup
004b4117  CALL 0x005657a0     ; MapClass::Get_CellClass
004b411f  PUSH EAX            ; arg1 target cell
004b4120  CALL dword ptr [EDI+0x1ac]
```

### Hover normal path-step site, `0x00515570`

```asm
0051551c  PUSH 0x1            ; arg5
00515525  PUSH 0x0            ; arg4 parent/current cell
00515552  CALL 0x005f5f00     ; current effective height
0051555b  PUSH EAX            ; arg3 height
00515560  PUSH EDX            ; arg2 direction
00515561  PUSH EAX            ; coordinate pointer for MapClass lookup
00515567  CALL 0x005657a0     ; MapClass::Get_CellClass
0051556f  PUSH EAX            ; arg1 target cell
00515570  CALL dword ptr [EDI+0x1ac]
```

### Hover `height == -1` placement/settle site, `0x00516E9B`

```asm
00516e4d  PUSH 0x1            ; arg5
00516e53  PUSH 0x0            ; arg4 parent/current cell
00516e5a  PUSH -0x1           ; arg3 height
00516e5c  PUSH -0x1           ; arg2 direction
...
00516e7e  PUSH EAX            ; coordinate pointer for MapClass lookup
00516e93  CALL 0x005657a0     ; MapClass::Get_CellClass
00516e98  PUSH EAX            ; arg1 target cell
00516e9b  CALL dword ptr [EBX+0x1ac]
```

### Jumpjet landing state, `0x0054C66D`

```asm
0054c662  PUSH 0x1            ; arg5
0054c664  PUSH 0x0            ; arg4 parent/current cell
0054c666  PUSH -0x1           ; arg3 height
0054c66a  PUSH -0x1           ; arg2 direction
0054c66c  PUSH EDI            ; arg1 target cell
0054c66d  CALL dword ptr [EAX+0x1ac]
```

### Jumpjet abort/emergency landing state, `0x0054CE34`

```asm
0054ce29  PUSH 0x1            ; arg5
0054ce2b  PUSH 0x0            ; arg4 parent/current cell
0054ce2d  PUSH -0x1           ; arg3 height
0054ce31  PUSH -0x1           ; arg2 direction
0054ce33  PUSH ESI            ; arg1 target cell
0054ce34  CALL dword ptr [EAX+0x1ac]
```

## Binary-Verified Findings

1. `DriveLocomotionClass::Process_Movement` has four audited runtime `Can_Enter_Cell` callsites at `0x004B2B17`, `0x004B2FF9`, `0x004B34C0`, and `0x004B4120`.
2. All four Drive `Process_Movement` callsites pass parent/current-cell argument `0` and arg5 `1`.
3. None of the audited Drive `Process_Movement` callsites passes `height == -1`.
4. The first two Drive `Process_Movement` sites call `0x005F5F00` immediately before pushing height.
5. The later Drive `Process_Movement` lookahead sites push a saved/inline current effective-height value rather than calling the helper immediately at the callsite.
6. `ShipLocomotionClass::Process_Drive_Track` mirrors Drive `Process_Drive_Track` for bridge-sensitive arguments.
7. `ShipLocomotionClass::Process_Movement` mirrors Drive `Process_Movement` for bridge-sensitive arguments at the corresponding four sites.
8. Hover has normal runtime path checks using current effective height at `0x00515570` and `0x005169AE`.
9. Hover has a live code-family check at `0x00516E9B` that passes `direction == -1` and `height == -1`.
10. Jumpjet landing and abort/emergency landing checks at `0x0054C66D` and `0x0054CE34` pass `direction == -1`, `height == -1`, parent/current-cell `0`, and arg5 `1`.
11. No audited live runtime locomotion/collision callsite passes an explicit nonzero parent/current-cell argument.

## Inference And Confidence Notes

The exact stack argument order is binary-verified from assembly. Semantic names such as scatter/probe target, lookahead cell, late/secondary cell, and settle target are inferred from surrounding locomotor control flow and prior reports.

Hover locomotor usage is live in standard YR for hover units. The normal path-step sites are high-confidence gameplay paths. The `0x00516E9B` site is in live hover code, but its exact player-visible trigger should be treated as medium-high confidence until a separate hover-state trace names the branch more precisely.

Jumpjet locomotor usage is live in standard YR for Rocketeer/Siege Chopper/Hornet-style locomotion. The landing state is high-confidence live. The abort/emergency landing state is live but conditional, so it is medium-high confidence as normal gameplay coverage.

A `Can_Enter_Cell` virtual call at `0x005B0979` has the normal runtime shape `(target, direction, current_effective_height, 0, 1)`, but the function family is not the Jumpjet state machine and appears to belong to Mech/older locomotor code. Since Mech locomotion is not standard YR gameplay, it is excluded from the live standard-YR matrix.

A `Can_Enter_Cell` virtual call at `0x00584271` belongs to `ZoneMap__FloodFillReachableZones`, not runtime locomotion/collision. It passes target neighbor cell, flood direction, target cell level, parent `0`, arg5 `1`. It is excluded from this runtime locomotor matrix.

## Answers To Research Questions

1. At each Drive `Process_Movement` callsite, the five `Can_Enter_Cell` arguments are listed in the matrix. All pass target cell, direction, current effective height, parent/current-cell `0`, and arg5 `1`.
2. Drive/Ship runtime movement checks do not pass `height == -1`. Hover placement/settle and Jumpjet landing/abort checks do pass `height == -1`.
3. No audited live runtime locomotion/collision callsite passes an explicit parent/current-cell argument. All pass `0`.
4. Drive `Process_Movement` uses the same current effective-height concept as `Process_Drive_Track`. Some sites call helper `0x005F5F00` immediately; others reuse an inline/saved equivalent.
5. Ship `Process_Movement` mirrors Drive `Process_Movement` for bridge-sensitive arguments.
6. Ship `Process_Drive_Track` mirrors Drive `Process_Drive_Track` for bridge-sensitive arguments.
7. Hover and Jumpjet do call `Can_Enter_Cell` with bridge-relevant runtime arguments. Hover has normal current-height path checks and one `height == -1` placement/settle check. Jumpjet landing/abort checks use `height == -1` and `direction == -1`.
8. A future Rust implementation must preserve runtime movement/collision calls as full binary-shaped checks: target cell, direction including `-1`, height including current effective height or `-1`, nullable parent/current cell with binary fallback behavior, and arg5. It must also preserve the Unit/Infantry two-pass bridge/ground list-vs-occupancy split every runtime tick, not only during A*.
9. Current Rust cannot express the full binary argument shape in the audited movement/collision path. Read-only scan showed the current pathfinding and movement occupancy code mostly carries a single target `MovementLayer` and lacks explicit runtime direction, `height == -1`, nullable parent/current-cell fallback, arg5, and mutable/two-pass bridge split semantics.

## Future Rust Invariant

Runtime locomotion/collision must not collapse bridge checking to an A* layer decision. It needs a binary-shaped `Can_Enter_Cell` equivalent that receives:

```text
(target_cell, direction, height, parent_or_current_cell, arg5)
```

Where:

- Drive, Ship, Walk, and normal Hover movement usually pass current effective height, not target layer height.
- Current effective height is based on the current cell plus persistent `ObjectClass+0x8C OnBridge` bridge deck offset.
- Hover and Jumpjet placement/landing checks must support `direction == -1` and `height == -1`.
- Parent/current-cell `0` is meaningful and must invoke the binary fallback path in `CheckBridgeTraversal`, including direction-based parent derivation when direction is available.
- The Unit/Infantry bridge split must remain a runtime two-pass decision: list iteration uses the bridge-vs-ground layer selected by height, while occupancy bits can be tested against the opposite pass.
- Arg5 must remain an explicit traversal/context flag, not be conflated with parent/current-cell presence.

## Files/Implementation Impact

No Rust implementation was changed for this report.

Relevant current Rust limitation observed read-only:

- `src/sim/pathfinding/cell_entry.rs` uses target layer style checks and does not model the full runtime `(target, direction, height, parent, arg5)` shape.
- `src/sim/movement/movement_occupancy.rs` carries deferred `(cell, MovementLayer)` checks and cannot represent `height == -1`, parent fallback, or the two-pass list/occupancy split.
- `src/sim/movement/movement_bridge.rs` models bridge state transitions but is not a replacement for binary-shaped runtime `Can_Enter_Cell` calls.
