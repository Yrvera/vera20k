# Bridge Hover height == -1 Runtime Push Branch - Ghidra Research Report

Report: BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md
Date: 2026-05-14
Scope: Targeted audit of Hover runtime Can_Enter_Cell callsite 0x00516E9B from BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md.

## Summary

The `Can_Enter_Cell(target, -1, -1, 0, 1)` site at `0x00516E9B` is not ordinary Hover path-step movement and is not a generic placement/settle check. It is inside Hover's ILocomotion `Push` override at `0x00516E10`. Hover's ILocomotion `Shove` override at `0x00516FC0` calls `Push` first and then adds a random wobble/facing disturbance on success.

Binary-verified argument shape at `0x00516E9B`:

| Function | Callsite | Target arg | Direction arg | Height arg | Parent/current-cell arg | Arg5 | Active in standard YR |
|---|---:|---|---:|---:|---:|---:|---|
| `HoverLocomotionClass::Push` | `0x00516E9B` | Adjacent cell computed from current cell plus push-facing octant | `-1` | `-1` | `0` | `1` | Conditional: Hover locomotor is used by retail YR hover units, but this branch runs only if an ILocomotion `Push`/`Shove` request reaches a hover locomotor. No ordinary Hover path-step or `CellClass::Scatter_Objects` trigger was confirmed. |

The previous matrix row should be renamed from "placement/settle branch" to "Push/Shove adjacent-cell validation branch".

## Prior-State Check

Parent report: `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`.

Prior state row applied: explicit open question / targeted extension. The matrix had already verified the five `Can_Enter_Cell` arguments at `0x00516E9B` but left the exact gameplay trigger at medium-high confidence. This report only audits that trigger and immediate Hover control flow.

## YR Liveness

Verified active data:

- Hover locomotor CLSID: `{4A582742-9839-11d1-B709-00A024DDAFD1}`.
- `rulesmd.ini` retail hover users include `[LCRF]`, `[ROBO]`, `[SAPC]`, and `[YHVR]` with that Locomotor and `SpeedType=Hover`.
- Hover rules defaults are active in `rulesmd.ini`: `HoverHeight=120`, `HoverDampen=40%`, `HoverBob=.04`, `HoverBoost=150%`, `HoverAcceleration=.02`, `HoverBrake=.03`.

Conditional liveness conclusion:

- The Hover implementation is live in standard YR.
- The `0x00516E9B` branch is live if the engine invokes ILocomotion `Push` or `Shove` on a hover unit.
- This pass did not confirm a standard-YR runtime caller that invokes `Push`/`Shove` on hover during ordinary movement, bridge crossing, settling, or `CellClass::Scatter_Objects`.
- Treat player-visible impact as conditional until a runtime breakpoint or exhaustive virtual-call provenance pass names the external caller. If invoked, the bridge-sensitive behavior is real because the call goes through Unit/Foot `Can_Enter_Cell` with `height == -1`.

## Vtable Identity

Hover ILocomotion vtable base: `0x007EACFC`.

Relevant slots verified by raw vtable scan:

| Slot | Offset | Target | Meaning |
|---:|---:|---:|---|
| 24 | `+0x60` | `0x00516C70` | Hover gate used by `Push`; true if powered or airborne. |
| 26 | `+0x68` | `0x00516E10` | Hover `Push`. Contains `Can_Enter_Cell` site `0x00516E9B`. |
| 27 | `+0x6C` | `0x00516FC0` | Hover `Shove`. Calls slot 26 `Push` first. |

Cross-references to concrete Hover implementations:

- `0x00516E10` has only the vtable data xref from `0x007EAD64`.
- `0x00516FC0` has only the vtable data xref from `0x007EAD68`.

Base-class ILocomotion defaults from prior docs remain relevant: `LocomotionClass__Push @ 0x0055AB70` and `LocomotionClass__Shove @ 0x0055AB80` are stubs that return false. Hover overrides both.

Offset note: assembly offsets in this report are relative to the ILocomotion subobject pointer used by the virtual call. Prior Hover docs describe object-base offsets. For this COM-style pointer, object-base offset is generally `loco_offset + 4`.

## Hover Push Entry Conditions

Function: `HoverLocomotionClass::Push @ 0x00516E10`.

Entry convention is COM-style stdcall, not C++ thiscall:

- `[esp+4]` on entry: ILocomotion pointer.
- `[esp+8]` on entry: raw push facing/direction argument.
- Function returns with `RET 0x8` through the normal exit.

Binary-verified entry gates before the `Can_Enter_Cell` call:

```asm
00516e10  SUB  ESP,0x10
00516e13  PUSH EBX
00516e14  PUSH ESI
00516e15  MOV  ESI,dword ptr [ESP + 0x1c]    ; ILocomotion* argument
00516e19  PUSH EDI
00516e1a  PUSH ESI
00516e1b  MOV  EAX,dword ptr [ESI]
00516e1d  CALL dword ptr [EAX + 0x60]        ; Hover slot 24 gate, 0x00516C70
00516e20  TEST AL,AL
00516e22  JZ   0x00516fb3                    ; fail
00516e28  MOV  AL,byte ptr [ESI + 0x6c]
00516e2b  TEST AL,AL
00516e2d  JNZ  0x00516fb3                    ; fail if already active/pending
```

Tiny findings that matter:

1. `Push` never reaches `Can_Enter_Cell` unless slot `+0x60` returns true.
2. `Push` also requires byte `[loco+0x6C] == 0` before attempting movement.
3. On success later, `Push` writes `[loco+0x6C] = 1`, so repeat pushes are blocked until another Hover path clears that byte.
4. This byte corresponds to object-base `+0x70` under the ILocomotion subobject pointer convention. Prior field naming around object-base `+0x68/+0x6C/+0x70` should be treated cautiously in Push/Shove context.

## Slot +0x60 Gate

Function: Hover slot 24 at `0x00516C70`.

```asm
00516c70  PUSH ESI
00516c71  MOV  ESI,dword ptr [ESP + 0x8]
00516c75  PUSH ESI
00516c76  CALL 0x0055a930                 ; base powered/available check
00516c7b  TEST AL,AL
00516c7d  JNZ  0x00516c94                 ; true if base check true
00516c7f  MOV  ECX,dword ptr [ESI + 0x8]  ; linked object
00516c82  MOV  EAX,dword ptr [ECX]
00516c84  CALL dword ptr [EAX + 0x1c8]    ; linked object altitude/height-like getter
00516c8a  TEST EAX,EAX
00516c8c  JG   0x00516c94                 ; true if > 0
00516c8e  XOR  AL,AL
00516c90  POP  ESI
00516c91  RET  0x4
00516c94  MOV  AL,0x1
00516c96  POP  ESI
00516c97  RET  0x4
```

Findings:

1. Hover `Push` is allowed if the base locomotor powered/available check returns true.
2. If that base check is false, Hover still allows `Push` when the linked object vtable `+0x1C8` result is greater than zero.
3. If unpowered and the linked object `+0x1C8` result is `<= 0`, `Push` returns false before any bridge-sensitive check.
4. The `> 0` comparison is signed (`JG`), not unsigned.

## Target Cell Construction

The target cell is computed before the virtual `Can_Enter_Cell` dispatch:

```asm
00516e33  MOV  ECX,dword ptr [ESI + 0x8]       ; linked Foot/Object
00516e36  LEA  EAX,[ESP + 0xc]
00516e3a  PUSH EAX
00516e3b  MOV  EDX,dword ptr [ECX]
00516e3d  CALL dword ptr [EDX + 0x1b8]         ; current cell coords helper
00516e43  MOV  DX,word ptr [EAX]               ; current cell X
00516e46  MOV  EDI,dword ptr [ESI + 0x8]       ; linked Foot/Object
00516e49  MOV  ECX,dword ptr [ESP + 0x24]      ; raw push facing arg
00516e4f  SHR  ECX,0xc
00516e52  INC  ECX
00516e55  SHR  ECX,0x1
00516e57  AND  ECX,0x7                         ; octant
00516e5e  ADD  DX,word ptr [ECX*0x4 + 0x89f688]
00516e66  LEA  ECX,[ECX*0x4 + 0x89f688]
00516e6d  MOV  CX,word ptr [ECX + 0x2]
00516e76  ADD  CX,word ptr [EAX + 0x2]         ; target Y
```

Formula:

```text
octant = (((raw_push_arg >> 12) + 1) >> 1) & 7
target = current_cell + DirectionDelta[octant]
DirectionDelta table base = 0x0089F688, stride = 4, dx short at +0, dy short at +2
```

Findings:

1. The raw Push/Shove argument is used only to choose the adjacent target cell.
2. That computed octant is not passed to `Can_Enter_Cell` as the direction argument.
3. `Can_Enter_Cell` receives direction `-1` unconditionally.
4. The current cell comes from linked object vtable `+0x1B8`; the branch does not use a cached Hover destination cell as the source.
5. The target is exactly one adjacent cell, not an arbitrary destination or current path waypoint.

## Exact Can_Enter_Cell Arguments

Callsite: `0x00516E9B`.

```asm
00516e4d  PUSH 0x1                          ; Can_Enter_Cell arg5
00516e53  PUSH 0x0                          ; parent/current-cell arg
00516e5a  PUSH -0x1                         ; height arg
00516e5c  PUSH -0x1                         ; direction arg
...
00516e7a  LEA  EAX,[ESP + 0x30]             ; target coord for MapClass::Get_CellClass
00516e7e  PUSH EAX
00516e8c  MOV  EBX,dword ptr [EDI]          ; linked Foot/Object vtable
00516e8e  MOV  ECX,0x87f7e8                 ; MapClass global
00516e93  CALL 0x005657a0                   ; MapClass::Get_CellClass(target coords)
00516e98  PUSH EAX                          ; Can_Enter_Cell target arg
00516e99  MOV  ECX,EDI                      ; this = linked Foot/Object
00516e9b  CALL dword ptr [EBX + 0x1ac]      ; Unit/Foot Can_Enter_Cell
00516ea1  TEST EAX,EAX
00516ea3  JNZ  0x00516fb3                   ; fail unless result == 0
00516ea9  MOV  byte ptr [ESI + 0x6c],0x1    ; success side effect
```

Stack argument order at the call:

| Parameter | Value | Evidence |
|---|---|---|
| target cell | `MapClass::Get_CellClass(current_cell + DirectionDelta[octant])` | `LEA [esp+0x30]`, call `0x005657A0`, then `PUSH EAX` at `0x00516E98` |
| direction | `-1` | `PUSH -0x1` at `0x00516E5C` |
| height | `-1` | `PUSH -0x1` at `0x00516E5A` |
| parent/current-cell | `0` | `PUSH 0x0` at `0x00516E53` |
| arg5 | `1` | `PUSH 0x1` at `0x00516E4D` |

Return-code finding:

- `Push` proceeds only when `Can_Enter_Cell` returns exactly `0`.
- Any nonzero return code cancels the push and returns false.
- This is stricter than treating the result as boolean pass/fail in the positive direction.

## Success Path Side Effects

After `Can_Enter_Cell == 0`, `Push` mutates Hover/Foot state.

Verified immediate side effect:

```asm
00516ea9  MOV byte ptr [ESI + 0x6c],0x1
```

Then `Push` branches on Hover slot `+0x10` / moving-state query:

```asm
00516eae  MOV EAX,dword ptr [ESI]
00516eb0  CALL dword ptr [EAX + 0x10]
00516eb3  TEST AL,AL
00516eb5  JZ 0x00516f78
```

Observed high-level behavior from assembly:

- If already moving:
  - Linked Foot field `+0x5E0` is set to `-1`, clearing a next-path-direction-like value.
  - If the old Hover next-cell waypoint is non-null, linked object vtable `+0xF4` is called to clear/unmark it.
  - A new destination/waypoint is built from the target cell center: `x = cell_x * 0x100 + 0x80`, `y = cell_y * 0x100 + 0x80`.
  - Ground-height helper `0x00578080` participates in computing destination Z.
  - If the current owner Z is at least ground height plus three bridge/hover threshold units, destination Z is raised by `DAT_00A8F1B4`.
  - Linked object vtable `+0xF0` is called with the new waypoint/destination.
- If not moving:
  - A target-center CoordStruct is constructed.
  - Hover vtable `+0x44` (`Move_To`) is called with that target coordinate.

Bridge-relevant detail:

- The `height == -1` `Can_Enter_Cell` gate happens before these movement state mutations.
- The later Z adjustment can raise the destination after successful entry validation, but it does not change the `Can_Enter_Cell` height argument. The validation still uses unknown height `-1`.

## Hover Shove Wrapper

Function: `HoverLocomotionClass::Shove @ 0x00516FC0`.

```asm
00516fc0  MOV  ECX,dword ptr [ESP + 0x8]   ; raw shove/push arg
00516fc4  PUSH ESI
00516fc5  MOV  ESI,dword ptr [ESP + 0x8]   ; ILocomotion* arg
00516fc9  PUSH ECX
00516fca  PUSH ESI
00516fcb  MOV  EAX,dword ptr [ESI]
00516fcd  CALL dword ptr [EAX + 0x68]      ; call Push(this, raw_arg)
00516fd0  TEST AL,AL
00516fd2  JZ   0x00517017                  ; fail if Push failed
00516fd4  MOV  byte ptr [ESI + 0x64],0x1
00516fd8  MOV  EDX,dword ptr [0x00a8b230]
00516fde  PUSH 0x1e
00516fe0  PUSH 0x14
00516fe2  LEA  ECX,[EDX + 0x218]
00516fe8  CALL 0x0065c7e0                  ; Random 20..30
00516fed  MOV  dword ptr [ESI + 0x68],EAX
00516ff0  MOV  EAX,[0x00a8b230]
00516ff5  PUSH 0x63
00516ff7  PUSH 0x0
00516ff9  LEA  ECX,[EAX + 0x218]
00516fff  CALL 0x0065c7e0                  ; Random 0..99
00517004  CMP  EAX,0x32
00517007  JGE  0x00517011
00517009  MOV  ECX,dword ptr [ESI + 0x68]
0051700c  NEG  ECX
0051700e  MOV  dword ptr [ESI + 0x68],ECX
00517011  MOV  AL,0x1
00517014  POP  ESI
00517015  RET  0x8
```

Findings:

1. `Shove` reaches the `height == -1` `Can_Enter_Cell` site only by calling `Push` first.
2. If `Push` fails, `Shove` returns false and does not write its random disturbance fields.
3. If `Push` succeeds, `Shove` writes byte `[loco+0x64] = 1`.
4. It then writes `[loco+0x68]` to a random integer from 20 to 30 inclusive.
5. A second random roll from 0 to 99 negates that value when the roll is less than 50. Thus the sign is 50/50 using `< 0x32`, not `<=`.
6. These offsets are relative to the ILocomotion subobject pointer, so they correspond to object-base `+0x68` and `+0x6C` under the prior object-base layout.

## What Does Not Trigger This Site

Binary/document verification in this pass rules out several likely but wrong interpretations.

### Ordinary Hover movement

Prior Hover movement reports identify ordinary Hover process/move routines around `0x00514310`, `0x00514D90`, `0x00515ED0`, and related methods. The `0x00516E9B` site is not in those normal per-tick movement routines; it is in vtable slot 26 `Push`.

### `CellClass::Scatter_Objects`

`CellClass::Scatter_Objects @ 0x00481670` does not call ILocomotion `Push` or `Shove`. It selects the ground vs bridge occupant list from its bridge/list parameter and invokes object vtable `+0x174` (`Scatter`) on selected occupants.

### Unit/Infantry Scatter

Prior scatter research shows Unit/Infantry `Scatter` chooses nearby cells and routes through destination/move behavior. This audit did not find it directly invoking Hover ILocomotion `Push`/`Shove`.

### False-positive virtual-call hits

A raw binary scan for short indirect calls through `+0x68` and `+0x6C` found many hits. Most are not ILocomotion calls because many class vtables have unrelated methods at the same byte offsets.

Important false positives checked:

| Address | Shape | Classification |
|---:|---|---|
| `0x0070F1E0` | `CALL [EAX+0x68]` with `push 0; push 0` on a Techno/Object pointer | Render/visual-state call, not ILocomotion `Push`. |
| `0x0073C5FF` | `CALL [EAX+0x6C]` near Unit draw code | Unit/Object vtable call, not ILocomotion `Shove`; nearby `Foot+0x674` loads are incidental to rendering context. |
| `0x00519298` | `CALL [EDX+0x6C]` with no Push/Shove stack shape | Object/class virtual, not ILocomotion `Shove`; no two-argument COM-style push pair. |
| `0x00692B1D` | Two-argument `CALL [EAX+0x6C]` through `DAT_00A8E334[DAT_008809A0]` | SuperWeaponType/UIMode targeting dispatch per existing docs, not locomotor. |

The only binary-confirmed call into Hover `Push` found in this audit is the internal `Shove -> Push` call at `0x00516FCD`. External virtual callers remain an open item.

## Bridge Semantics

The bridge-sensitive part is the exact argument tuple:

```text
Can_Enter_Cell(target_adjacent_cell, direction = -1, height = -1, parent = 0, arg5 = 1)
```

Implications from prior `CheckBridgeTraversal` / two-pass reports:

1. `height == -1` enables the unknown-height path. For bridge candidates, `CheckBridgeTraversal` can seed a bridge-deck height from the candidate bridge cell instead of using the mover's current effective height.
2. `direction == -1` means the helper does not use direction-based parent fallback.
3. `parent/current-cell == 0` means this call supplies no explicit parent cell. Any parent/current fallback must be the binary's null-parent behavior, not a caller-provided current cell.
4. `arg5 == 1` keeps this in the same traversal/context family as the other live runtime movement checks, but with the unique unknown-height/no-direction shape.
5. The raw push direction is still important because it selects the target adjacent cell; it is just not passed as the `Can_Enter_Cell` direction argument.

Player impact if this branch is reached:

- A hover unit pushed/shoved adjacent to a bridge can choose bridge-deck vs ground occupancy through the `height == -1` path rather than the normal current-effective-height runtime movement pattern.
- A Rust implementation that only supports `(target, direction, current_effective_height, 0, 1)` cannot represent this branch.
- A Rust implementation that passes the computed octant as the `Can_Enter_Cell` direction would not match this branch. The binary passes `-1`.

## Rust Parity Invariant For Future Work

No Rust implementation changes were made in this investigation.

Future implementation should preserve these invariants:

1. Runtime `Can_Enter_Cell` must be able to represent all five binary arguments independently: target cell, direction, height, parent/current-cell pointer, and arg5.
2. Hover `Push`/`Shove` must support the unknown-height tuple `(target_adjacent, -1, -1, 0, 1)`.
3. The target adjacent cell is selected from the raw push/shove facing via `(((raw >> 12) + 1) >> 1) & 7`; that octant must not be reused as the `Can_Enter_Cell` direction argument.
4. Parent/current-cell is explicitly null in this branch. Future bridge traversal must preserve null-parent fallback behavior instead of substituting the hover unit's current cell unless binary evidence says a callee does so internally.
5. The `Can_Enter_Cell` result must be interpreted like the binary: only result `0` allows the push. Nonzero cancels it.
6. The two-pass bridge/ground split must still run for this runtime call. This is not only an A* concern.
7. If Hover `Shove` is implemented, it must call the same `Push` validation first and only apply its random signed disturbance after `Push` succeeds.
8. The sim layer must own any future gameplay behavior without depending on render/ui/sidebar/audio/net.

## Current Expressiveness Gap

Based on the parent matrix and this audit, current Rust should be considered unable to fully express this binary shape until proven otherwise. The missing shape is specifically a runtime movement/collision entry check with:

```text
direction = -1
height = -1
parent/current-cell = null
arg5 = 1
target = adjacent cell from push-facing conversion
```

The gap is not merely Hover locomotion. It is the ability to pass and preserve the nullable parent and unknown-height arguments through runtime bridge occupancy checks.

## Confidence

High confidence:

- `0x00516E10` is Hover ILocomotion slot `+0x68` (`Push`).
- `0x00516FC0` is Hover ILocomotion slot `+0x6C` (`Shove`).
- `0x00516E9B` calls linked Unit/Foot vtable `+0x1AC` with `(target, -1, -1, 0, 1)`.
- The target cell is current cell plus `DirectionDelta[(((raw >> 12) + 1) >> 1) & 7]`.
- `Can_Enter_Cell` must return `0` for `Push` to continue.
- `Shove` calls `Push` and only writes random wobble/disturbance fields after `Push` succeeds.

Medium confidence:

- Field names around Hover object-base `+0x68/+0x6C/+0x70`; the writes are verified, but previous semantic names need a dedicated Hover state-field audit.
- The later success-path Z adjustment likely relates to bridge/altitude handling, but it occurs after the `Can_Enter_Cell` validation and was not the main scope.

Low / open:

- Exact external standard-YR caller(s) of ILocomotion `Push`/`Shove` that reach Hover during normal gameplay.
- Whether bridge crossing/collision in a real match commonly invokes this branch for LCRF/ROBO/SAPC/YHVR. Static evidence confirms the branch; runtime frequency needs a breakpoint or a complete virtual-call provenance pass.

## Open Questions / Next Audit

1. Run a dedicated ILocomotion virtual-call provenance scan for slots `+0x68` and `+0x6C`, not just raw vtable-offset matches. The scan needs to prove the receiver is `FootClass+0x674` or another ILocomotion pointer before classifying a call as Push/Shove.
2. Runtime-breakpoint `0x00516E10` and `0x00516FC0` in standard YR with hover units near bridges, collision, rocker/direct-rocker impacts, crate/chrono displacement, and blocked-cell interactions.
3. Audit Hover object-base fields `+0x68`, `+0x6C`, and `+0x70` across all Hover methods. This report verifies Push/Shove writes but does not fully name those fields.
4. Check whether non-Hover locomotors override `Push`/`Shove` in retail YR and whether any of them call `Can_Enter_Cell` with unknown height.

## Sources

- Ghidra assembly, `gamemd.exe`:
  - `HoverLocomotionClass::Push @ 0x00516E10`
  - `Can_Enter_Cell` callsite `0x00516E9B`
  - `HoverLocomotionClass::Shove @ 0x00516FC0`
  - Hover slot `+0x60` gate `0x00516C70`
  - Hover ILocomotion vtable `0x007EACFC`
- Existing research docs:
  - `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
  - `BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`
  - `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
  - `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`
  - `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
  - `ILOCOMOTION_COM_PROTOCOL_SPEC.md`
  - `SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md`
  - `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`
  - `DISPLAYCLASS_GHIDRA_REPORT.md` for the `DAT_00A8E334[DAT_008809A0]` false-positive classification
- INI data:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
