# Bridge Jumpjet Height -1 Runtime Can_Enter_Cell - Ghidra Research Report

Report: BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md
Date: 2026-05-14
Scope: Focused audit of the two live-looking Jumpjet runtime `Can_Enter_Cell(target, -1, -1, 0, 1)` callsites from the bridge runtime callsite matrix.
Primary binary: `gamemd.exe` Yuri's Revenge 1.001.

This report extends:

- `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
- `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`
- `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`
- `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`

## Executive Summary

Both Jumpjet `height == -1` runtime callsites are binary-verified and live through the Jumpjet state machine:

1. `0x0054C66D`, inside `FUN_0054C550`, state 4 descend/land.
2. `0x0054CE34`, inside `FUN_0054CA90`, state 5 abort/emergency landing.

Both pass the same five `Can_Enter_Cell` arguments:

```text
(target_cell, direction = -1, height = -1, parent/current = 0, arg5 = 1)
```

Because `direction == -1`, `CheckBridgeTraversal` uses the candidate-only branch already verified in `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`:

```text
if height == -1 and target_cell.flags & 0x100:
    local height = target_cell.level + 4
return 0
```

That means these Jumpjet landing checks can select bridge-deck list/occupancy behavior from the landing candidate cell itself. They do not run the directed parent/predecessor bridgehead, slope, or height-diff checks.

Player-visible risk: a Jumpjet landing, emergency descent, or recovery around a bridge can choose deck-vs-ground collision semantics based on this candidate-only bridge height seed. A Rust implementation that treats air landing as "ignore terrain occupancy until altitude zero" or as a single preselected layer cannot reproduce this exact binary shape.

## Prior-State Row

Prior state: partial/high-confidence callsite matrix with explicit open trigger question.

The existing matrix had already verified the push sequence at `0x0054C66D` and `0x0054CE34`. This pass audits:

- how Jumpjet reaches state 4 and state 5,
- the branch conditions immediately around each callsite,
- whether the paths are active in standard YR,
- and what Rust must preserve.

This pass does not re-cover Jumpjet layer sorting, Hover, Drive, Ship, or Walk locomotor behavior except where needed for comparison.

## Binary-Verified Jumpjet Process Dispatch

`JumpjetLocomotionClass::Process @ 0x0054AEC0` receives `this = instance + 4` through the ILocomotion vtable. It dispatches state handlers by the true instance field `instance + 0x50`, read as `[ESI + 0x4C]` because `ESI = instance + 4`.

Assembly evidence:

```asm
0054aec4  MOV ESI,dword ptr [ESP + 0x28] ; this = instance + 4
...
0054b032  MOV EAX,dword ptr [ESI + 0x4c] ; instance+0x50 state
0054b036  CMP EAX,0x6
0054b03c  JMP dword ptr [EAX*0x4 + 0x54b19c]

0054b06b  LEA ECX,[ESI + -0x4]
0054b06e  CALL 0x0054c550                ; state 4 descend/land

0054b075  LEA ECX,[ESI + -0x4]
0054b078  CALL 0x0054ca90                ; state 5 abort/emergency
```

Process only calls the state machine when either vtable `+0x10` returns true or vtable `+0x80` returns true:

```asm
0054aed1  CALL dword ptr [ECX + 0x10]
0054aed8  TEST AL,AL
0054aeda  JNZ 0x0054aeed
0054aedf  CALL dword ptr [EDX + 0x80]
0054aee5  TEST AL,AL
0054aee7  JZ 0x0054b16c
0054aeed  LEA ECX,[ESI + -0x4]
0054aef0  CALL 0x0054d0f0
```

Identity and binding confidence: HIGH. The vtable and state machine were previously verified in `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, and the fresh assembly confirms state 4 and state 5 dispatch.

## Callsite Matrix

| Function | State | Callsite | Target arg | Direction | Height | Parent/current | Arg5 | Live YR confidence |
|---|---:|---:|---|---:|---:|---:|---:|---|
| `FUN_0054C550` | 4 | `0x0054C66D` | `EDI`, destination/candidate cell from `CellClass__Get_Cell_At(instance+0x40)` | `-1` | `-1` | `0` | `1` | High |
| `FUN_0054CA90` | 5 | `0x0054CE34` | `ESI`, current/recovery candidate cell from linked object's current coord | `-1` | `-1` | `0` | `1` | Medium-high |

## State 4 Landing Callsite, 0x0054C66D

Function: `FUN_0054C550 @ 0x0054C550`

Role: Jumpjet state 4 descend/land handler.

### Target Cell Source

At entry, state 4 builds the candidate from the Jumpjet destination coordinate stored at `instance + 0x40/+0x44/+0x48`.

```asm
0054c55e  LEA EBP,[ESI + 0x40]   ; instance destination coord
0054c561  PUSH EBP
0054c562  CALL 0x00565730        ; CellClass__Get_Cell_At
0054c567  MOV ECX,dword ptr [ESI + 0xc]
0054c56a  MOV EDI,EAX            ; EDI = candidate target cell
```

Therefore the `Can_Enter_Cell` target at `0x0054C66D` is the Jumpjet destination/candidate cell, not a path predecessor or current cell.

### Exact Five Arguments

Assembly evidence:

```asm
0054c65f  MOV ECX,dword ptr [ESI + 0xc] ; linked object
0054c662  PUSH 0x1                      ; arg5 traversal/context flag
0054c664  PUSH 0x0                      ; arg4 parent/current cell
0054c666  PUSH -0x1                     ; arg3 height
0054c668  MOV EAX,dword ptr [ECX]
0054c66a  PUSH -0x1                     ; arg2 direction
0054c66c  PUSH EDI                      ; arg1 target cell
0054c66d  CALL dword ptr [EAX + 0x1ac]  ; Unit/Infantry Can_Enter_Cell
0054c673  MOV dword ptr [ESP + 0x14],EAX ; save return
```

Binary-verified argument tuple:

```text
(target = EDI, direction = -1, height = -1, parent/current = 0, arg5 = 1)
```

### Branches Before The Call

The call is skipped only by earlier state-4 special cases.

State 4 first checks linked object byte `+0x6AD`. If it is nonzero, it jumps to the later landing/finalization block and skips the `Can_Enter_Cell` probe:

```asm
0054c56c  MOV AL,byte ptr [ECX + 0x6ad]
0054c572  TEST AL,AL
0054c574  JNZ 0x0054c737
```

If `+0x6AD == 0`, it checks the candidate cell land type:

```asm
0054c57a  MOV EAX,dword ptr [EDI + 0xec] ; candidate LandType
0054c580  CMP EAX,0x2
0054c583  JZ 0x0054c58e
0054c585  CMP EAX,0x6
0054c588  JNZ 0x0054c65f                ; non-water/beach -> Can_Enter_Cell
```

For LandType 2 or 6, state 4 may return before the call:

```asm
0054c58e  CMP dword ptr [ECX + 0xb4],0x7
0054c595  JZ 0x0054c65f
0054c59d  CALL dword ptr [EAX + 0x184]
0054c5a3  CMP EAX,0x7
0054c5a6  JZ 0x0054c65f
```

If the water/beach special path stays active, `FUN_004135A0` chooses between two early returns:

```asm
0054c5b5  CALL 0x004135a0
0054c5ba  TEST AL,AL
0054c5bc  JZ 0x0054c61e
```

If true, the handler picks a random direction, issues a path/retry, sets state 3, and returns:

```asm
0054c5c4  PUSH 0x7
0054c5c6  PUSH 0x0
0054c5ce  CALL 0x0065c7e0                ; Random__RandomRanged(0,7)
...
0054c5ea  CALL 0x00481810                ; path/retry helper
...
0054c610  MOV dword ptr [ESI + 0x50],0x3 ; state 3
0054c61a  ADD ESP,0x14
0054c61d  RET
```

If false, it clears/rebuilds occupancy around the cell, sets state 2, restores the target altitude from `instance + 0x2C`, and returns:

```asm
0054c62c  CALL 0x00487d70
...
0054c648  MOV EDX,dword ptr [ESI + 0x2c]
0054c64b  MOV dword ptr [ESI + 0x50],0x2 ; state 2
0054c652  MOV dword ptr [ESI + 0x80],EDX
0054c65b  ADD ESP,0x14
0054c65e  RET
```

Therefore, the `0x0054C66D` call is the normal state-4 landing candidate check after water/beach avoidance has either been bypassed or rejected.

### Branches After The Call

The return from `Can_Enter_Cell` is stored at `[ESP + 0x14]`. State 4 also checks subcell availability:

```asm
0054c673  MOV dword ptr [ESP + 0x14],EAX ; Can_Enter_Cell return
0054c679  CALL 0x004810a0                ; CellClass__GetSubCell(destination)
0054c684  MOV EBX,EAX                    ; subcell
0054c68c  PUSH ECX                       ; bridge-ish flag from candidate flags
0054c68d  PUSH EBX
0054c690  CALL 0x00481130                ; CellClass__IsSubCellFree
```

If the subcell is free, or if `instance + 0x90` is already set, state 4 sets a local obstruction/retry byte:

```asm
0054c695  TEST AL,AL
0054c697  JNZ 0x0054c6a8
0054c699  MOV AL,byte ptr [ESI + 0x90]
0054c69f  MOV byte ptr [ESP + 0x13],0x0
0054c6a4  TEST AL,AL
0054c6a6  JZ 0x0054c6ad
0054c6a8  MOV byte ptr [ESP + 0x13],0x1
```

One object-type/class check also forces that local byte when the linked object type code is `1` and the chosen subcell is `0`:

```asm
0054c6b2  CALL dword ptr [EDX + 0x2c]
0054c6b5  CMP EAX,0x1
0054c6b8  JNZ 0x0054c6c2
0054c6ba  TEST EBX,EBX
0054c6bc  JNZ 0x0054c6c2
0054c6be  MOV byte ptr [ESP + 0x13],AL
```

If that local byte is set, the handler re-fetches the destination cell and uses the `Can_Enter_Cell` return specially:

```asm
0054c6db  MOV AL,byte ptr [ESP + 0x13]
0054c6df  TEST AL,AL
0054c6e1  JZ 0x0054ca3c
0054c6ed  CALL 0x00565730                ; re-fetch dest cell
0054c6f2  MOV ECX,dword ptr [EAX + 0x140]
0054c6f8  TEST CH,0x1                    ; candidate bridge flag 0x100
0054c6fb  JNZ 0x0054c71a
0054c6fd  MOV EAX,dword ptr [ESP + 0x14] ; Can_Enter_Cell return
0054c701  CMP EAX,0x2
0054c704  JG 0x0054ca3c
0054c70a  JNZ 0x0054c71a
0054c70c  MOV AL,byte ptr [ESI + 0x90]
0054c714  JZ 0x0054ca3c
```

If the destination cell is a bridge cell, this branch accepts the obstruction/retry path without testing return code `<= 2`. That makes the candidate bridge flag a direct post-call branch input.

If `instance + 0x90` was not set yet, state 4 sets it and calls the linked object's vtable `+0xF0` with the destination coordinate:

```asm
0054c71a  MOV AL,byte ptr [ESI + 0x90]
0054c720  TEST AL,AL
0054c722  JNZ 0x0054c737
0054c724  MOV ECX,dword ptr [ESI + 0xc]
0054c727  MOV byte ptr [ESI + 0x90],0x1
0054c72e  PUSH EBP
0054c731  CALL dword ptr [EDX + 0xf0]
```

The exact semantic name of `instance + 0x90` is still inferred. It behaves as a landing-abort/retry flag in this function.

## State 4 Reachability

State 4 is entered from at least state 2 and state 3 during normal Jumpjet approach/landing logic.

State 2 (`FUN_0054BD30`) sets state 4 at `0x0054BED1` after arrival/landing-target handling:

```c
*(undefined4 *)(param_1 + 0x50) = 4;
```

State 3 (`FUN_0054BFF0`) sets state 4 when the horizontal distance to the destination is less than `0x14` leptons and the landing path is not redirected:

```c
if (iVar9 < 0x14) {
    ...
    *(undefined4 *)(param_1 + 0x80) = 0;
    *(undefined4 *)(param_1 + 0x50) = 4;
}
```

State 5 can also recover into state 4 at `0x0054CFA2` after its own emergency landing check succeeds.

Active in YR: Yes, conditional by unit behavior. Jumpjet locomotor is used in standard YR data by Rocketeer/Lunar Infantry, Floating Disk, Siege Chopper variants, Kirov-style airship data, SHAD/HIND transport data, and related jumpjet aircraft definitions. Units with `BalloonHover=yes` do not normally choose voluntary ground landing at idle, but state 4 remains part of the live Jumpjet state machine for landable jumpjet units, deployment/landing flows, and recovery paths. Confidence: HIGH for binary reachability; HIGH that Jumpjet locomotor is standard YR-active; MEDIUM-HIGH for per-unit frequency because it depends on rules flags such as `BalloonHover`, `Landable`, deploy behavior, and mission state.

## State 5 Abort/Emergency Callsite, 0x0054CE34

Function: `FUN_0054CA90 @ 0x0054CA90`

Role: Jumpjet state 5 abort/emergency landing or invalid-destination recovery handler.

### State 5 Entry From Process

`Process @ 0x0054AEC0` forces state 5 when all of these are true:

- linked object byte `+0x425` is nonzero,
- current Jumpjet state is not 5,
- current Jumpjet state is not 6,
- linked object altitude from vtable `+0x1C8` is greater than zero,
- and either linked byte `+0x6AD` is zero, or fetched coordinate comparison says the object has reached the relevant cell.

Assembly evidence:

```asm
0054af33  MOV AL,byte ptr [ECX + 0x425]
0054af39  TEST AL,AL
0054af3b  JZ 0x0054b032
0054af41  MOV EAX,dword ptr [ESI + 0x4c] ; state
0054af44  CMP EAX,0x5
0054af47  JZ 0x0054b032
0054af4d  CMP EAX,0x6
0054af50  JZ 0x0054b032
0054af58  CALL dword ptr [EDX + 0x1c8]   ; Get_Height
0054af5e  TEST EAX,EAX
0054af60  JLE 0x0054b032
```

When the abort condition wins, Process sets the working value at instance `+0x80` to `-5` and state `+0x50` to `5`:

```asm
0054b006  MOV dword ptr [ESI + 0x7c],0xfffffffb ; true instance+0x80 = -5
0054b00d  MOV dword ptr [ESI + 0x4c],0x5        ; true instance+0x50 = state 5
```

If the object class code is `0xF`, it also calls vtable `+0x558` with script/event `0x22`:

```asm
0054b019  CALL dword ptr [EAX + 0x2c]
0054b01c  CMP EAX,0xf
0054b021  MOV ECX,dword ptr [ESI + 0x8]
0054b024  PUSH 0x0
0054b026  PUSH 0x0
0054b028  PUSH 0x22
0054b02c  CALL dword ptr [EDX + 0x558]
```

Semantic names for `+0x425`, `+0x6AD`, and the class-code `0xF` branch remain partly inferred from prior reports. The state transition itself is binary-verified.

### Bridge-Deck Crossing Gate Before The Call

State 5 computes a local "bridge plane crossing" byte at `[ESP + 0x10]`.

First it computes the current cell's ground height plus `DAT_00ABC5DC`, then fetches the current cell:

```asm
0054cb3e  LEA ECX,[ESP + 0x1c]  ; current coord local
0054cb48  CALL 0x00578080       ; CellClass__GetGroundHeight
0054cb4d  MOV ESI,EAX
0054cb4f  MOV EAX,[0x00abc5dc]  ; Jumpjet bridge altitude threshold
0054cb5e  ADD ESI,EAX           ; threshold = ground_z + bridge threshold
0054cb60  CALL 0x00565730       ; CellClass__Get_Cell_At
0054cb65  MOV ECX,dword ptr [EAX + 0x140]
```

Then it sets `[ESP + 0x10] = 1` only if:

- current cell has bridge flag `0x100`,
- current Z (`EBP`, saved from the linked object's coord Z before update) is at or above `ground + DAT_00ABC5DC`,
- and computed next/local landing Z at `[ESP + 0x24]` is below that same threshold.

Assembly evidence:

```asm
0054cb6b  TEST CH,0x1
0054cb6e  JZ 0x0054cb81
0054cb70  CMP EBP,ESI
0054cb72  JL 0x0054cb81
0054cb74  MOV EAX,dword ptr [ESP + 0x24]
0054cb78  MOV byte ptr [ESP + 0x10],0x1
0054cb7d  CMP EAX,ESI
0054cb7f  JL 0x0054cb86
0054cb81  MOV byte ptr [ESP + 0x10],0x0
```

This is the key bridge-specific trigger detail for state 5: the abort handler is allowed to attempt a landing/recovery cell check not only at altitude zero, but also when the falling Jumpjet crosses the bridge-deck altitude plane over a bridge cell.

### Gate To The Candidate Check

After optional layer remove/resubmit work for in-bounds cells, state 5 checks:

```asm
0054cc20  CALL dword ptr [EDX + 0x1c8] ; Get_Height
0054cc26  TEST EAX,EAX
0054cc28  JLE 0x0054cc36
0054cc2a  MOV AL,byte ptr [ESP + 0x10] ; bridge crossing flag
0054cc30  JZ 0x0054d019                ; skip landing attempt
0054cc36  MOV ECX,dword ptr [EDI + 0xc]
0054cc39  MOV AL,byte ptr [ECX + 0x427]
0054cc3f  TEST AL,AL
0054cc41  JZ 0x0054d019                ; skip landing attempt
```

Therefore the `0x0054CE34` `Can_Enter_Cell` call only runs when:

- linked object byte `+0x427` is nonzero, and
- either the Jumpjet has reached altitude `<= 0`, or it is currently crossing the bridge deck height plane over a bridge cell.

This makes the path explicitly bridge-sensitive at runtime.

### Target Cell Source

Before the call, state 5 re-fetches the candidate from the linked object's current coordinate:

```asm
0054cdc3  MOV ECX,dword ptr [EDI + 0xc]
0054cdcb  ADD ECX,0x9c
0054cdd1  MOV EDX,dword ptr [ECX]
0054cdd3  MOV dword ptr [ESP + 0x28],EDX
0054cddf  MOV dword ptr [ESP + 0x30],EAX
0054cde6  MOV dword ptr [ESP + 0x34],ECX
0054cdea  MOV ECX,0x87f7e8
0054cdef  CALL 0x00565730        ; CellClass__Get_Cell_At
0054cdf4  MOV ESI,EAX            ; ESI = candidate target cell
```

The target is the current/recovery landing candidate cell, not the stored destination at `instance + 0x40`.

### Exact Five Arguments

Assembly evidence:

```asm
0054ce26  MOV ECX,dword ptr [EDI + 0xc] ; linked object
0054ce29  PUSH 0x1                      ; arg5 traversal/context flag
0054ce2b  PUSH 0x0                      ; arg4 parent/current cell
0054ce2d  PUSH -0x1                     ; arg3 height
0054ce2f  MOV EAX,dword ptr [ECX]
0054ce31  PUSH -0x1                     ; arg2 direction
0054ce33  PUSH ESI                      ; arg1 target cell
0054ce34  CALL dword ptr [EAX + 0x1ac]  ; Unit/Infantry Can_Enter_Cell
0054ce3d  MOV EBP,EAX                   ; save return
```

Binary-verified argument tuple:

```text
(target = ESI, direction = -1, height = -1, parent/current = 0, arg5 = 1)
```

### Branches After The Call

State 5 applies additional passability checks after `Can_Enter_Cell`.

If the linked object's class code is `2`, a helper at `0x004834A0` can override the result to `7`:

```asm
0054ce3f  TEST ECX,ECX
0054ce41  JZ 0x0054ce6a
0054ce45  CALL dword ptr [EDX + 0x2c]
0054ce48  CMP EAX,0x2
0054ce4b  JNZ 0x0054ce6a
...
0054ce5c  CALL 0x004834a0
0054ce61  TEST AL,AL
0054ce63  JNZ 0x0054ce6a
0054ce65  MOV EBP,0x7
```

If the linked object's type has byte `+0xD94` set, another `0x004834A0` check participates:

```asm
0054ce6f  CALL dword ptr [EAX + 0x84] ; type
0054ce75  MOV CL,byte ptr [EAX + 0xd94]
0054ce7b  TEST CL,CL
0054ce7d  JZ 0x0054ce98
...
0054ce8f  CALL 0x004834a0
0054ce94  TEST AL,AL
0054ce96  JZ 0x0054cea0
0054ce98  TEST EBP,EBP
0054ce9a  JZ 0x0054cf3c
```

On a successful recovery path, state 5 transitions back to state 4 and calls linked vtable `+0x480` with `(0,1)`:

```asm
0054cf77  MOV ESI,dword ptr [EDI + 0xc]
...
0054cfa2  MOV dword ptr [EDI + 0x50],0x4 ; state 4
0054cfa9  PUSH 0x1
0054cfab  PUSH 0x0
0054cfaf  CALL dword ptr [EAX + 0x480]
```

On the terminal failure path, state 5 sets state 6, clears vertical motion fields, plays voice/event `0x117C`, and clears target pointers:

```asm
0054d07d  MOV dword ptr [EDI + 0x78],EBX
0054d083  MOV dword ptr [EDI + 0x50],0x6 ; state 6
0054d08a  MOV dword ptr [EDI + 0x7c],EBX
0054d08d  PUSH EBX
0054d090  PUSH 0x117c
0054d095  CALL dword ptr [EDX]
0054d09e  MOV dword ptr [EAX + 0x428],EBX
0054d0a7  MOV dword ptr [EAX + 0x42c],EBX
```

## State 5 Reachability

State 5 is active in the standard Jumpjet state machine but conditional. It is not the normal "arrived cleanly" landing path; it is the abort/emergency path entered when linked object byte `+0x425` is set while airborne and the current state is neither 5 nor 6.

Prior reports connect this state to invalid destination, abort, and emergency landing behavior. Other reports also mention this handler in Magnetron/forced-lift contexts. This pass confirms the runtime code path and bridge deck crossing trigger, but does not fully name every writer of `+0x425` or `+0x427`.

Active in YR: Yes, conditional. Jumpjet locomotor is standard YR-active. State 5 is reachable through the Process dispatch in standard code, but only when abort/emergency flags are set. Confidence: MEDIUM-HIGH for gameplay reachability; HIGH for binary path and callsite details.

## Why Direction -1 Matters Here

Both Jumpjet callsites use:

```text
direction = -1
height = -1
parent/current = 0
```

Through Unit/Infantry `Can_Enter_Cell`, that reaches `CheckBridgeTraversal @ 0x004D9C60`. The parent fallback report verified that direction `-1` is candidate-only:

```asm
004d9cbc  CMP EBX,-0x1
004d9cbf  JZ  0x004d9e3e
...
004d9e42  CMP dword ptr [EAX],-0x1
004d9e45  JNZ 0x004d9e5e
004d9e47  MOV ECX,dword ptr [EDI + 0x140]
004d9e4d  TEST CH,0x1
004d9e50  JZ  0x004d9e5e
004d9e52  MOVSX ECX,byte ptr [EDI + 0x11b]
004d9e59  ADD ECX,0x4
004d9e5c  MOV dword ptr [EAX],ECX
004d9e61  XOR EAX,EAX
```

Consequences for Jumpjet:

1. Landing on a bridge candidate seeds the local path height from the candidate cell, not from a parent or current cell.
2. The helper does not require bridgehead flag `0x200`.
3. The helper does not run directed slope or height-diff legality.
4. The Unit/Infantry two-pass list/occupancy logic can therefore treat the landing candidate as bridge deck for collision/list purposes even though the caller supplied no explicit bridge layer.
5. State 5 has an additional bridge-plane crossing gate before the call, so emergency descent can test a bridge-deck landing/recovery before reaching ground altitude.

## INI / Standard YR Activity Notes

Read-only INI scan found `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}` in active `rulesmd.ini` sections including:

- `JUMPJET` Rocketeer, `JumpJet=yes`, `BalloonHover=yes`, `ConsideredAircraft=yes`
- `LUNR` Lunar Infantry, `JumpJet=yes`, `BalloonHover=yes`, `ConsideredAircraft=yes`
- `DISK` Floating Disk, `BalloonHover=yes`, `ConsideredAircraft=yes`
- `SHAD` BlackHawk Transport, `JumpJet=yes`, `Landable=yes`, `ConsideredAircraft=yes`
- `HIND` Hind Transport, `JumpJet=yes`, `Landable=yes`, `ConsideredAircraft=yes`
- Siege Chopper variants, `JumpJet=yes`, `ConsideredAircraft=yes`
- Kirov-style airship data, `BalloonHover=yes`, `ConsideredAircraft=yes`

`LocomotorBeam` also contains the GUID string but is not a normal unit type consumer; it is not used as standard Jumpjet locomotor evidence.

The presence of `BalloonHover=yes` on several units reduces voluntary landing frequency, but does not make the state handlers dead. Landable jumpjet aircraft and abort/emergency paths keep the audited callsites gameplay-relevant.

## Current Rust Read-Only Status

No Rust code was changed.

Read-only scan found:

- `src/sim/movement/air_movement.rs` models air movement as altitude phases plus straight/facing movement, with comments saying air units ignore terrain and ground occupancy.
- `src/sim/movement/jumpjet_movement.rs` models Jumpjet altitude, hover, landing, acceleration, and crash descent, but does not expose a binary-shaped runtime `Can_Enter_Cell(target, direction, height, parent, arg5)` check for landing.
- `src/sim/movement/movement_bridge.rs` handles ground cell-boundary `on_bridge` transitions and is not a replacement for Jumpjet's candidate-only landing `Can_Enter_Cell` shape.
- Existing pathfinding/cell-entry code remains layer-oriented and cannot directly represent `height == -1`, `direction == -1`, nullable parent/current cell, and the Unit/Infantry local two-pass bridge split at this runtime landing site.

Current expressiveness gap: Rust has no obvious way to model "airborne Jumpjet crossing a bridge deck plane, ask Unit/Infantry `Can_Enter_Cell(candidate, -1, -1, 0, 1)`, let the callee seed candidate bridge height locally, then branch on the result."

## Future Rust Invariant

A future implementation should preserve these output-determining invariants:

1. Jumpjet state 4 landing checks must be able to call a binary-shaped `Can_Enter_Cell` equivalent with `(target, -1, -1, 0, 1)`.
2. Jumpjet state 5 abort/emergency checks must do the same, and must be allowed to trigger when the falling unit crosses the bridge-deck altitude plane, not only when altitude reaches zero.
3. `direction == -1` must use candidate-only bridge height seeding. Do not substitute current cell, parent cell, or A* path layer.
4. Candidate bridge cells should be able to select bridge-deck list/occupancy semantics in the Unit/Infantry `Can_Enter_Cell` body even when the caller is an air locomotor.
5. State 5 recovery must distinguish "recover to state 4 landing" from "terminal state 6 crash/abort" after the candidate check and follow-up passability checks.
6. The sim layer must own this logic without dependencies on render, UI, sidebar, audio, or net.

## Binary-Verified Findings vs Inference

Binary-verified:

1. `Process @ 0x0054AEC0` dispatches state 4 to `0x0054C550` and state 5 to `0x0054CA90`.
2. State 4 callsite `0x0054C66D` passes `(EDI, -1, -1, 0, 1)`.
3. State 4 target `EDI` is produced from `CellClass__Get_Cell_At(instance+0x40)`.
4. State 4 has water/beach early-return paths before the call.
5. State 4 post-call logic reads both the `Can_Enter_Cell` return and candidate bridge flag `0x100`.
6. Process sets state 5 when linked byte `+0x425` is set, state is not 5/6, and altitude is greater than zero.
7. State 5 computes a bridge-deck crossing byte from current cell bridge flag, current Z, next/local Z, ground height, and `DAT_00ABC5DC`.
8. State 5 only proceeds toward the landing candidate check when `+0x427` is set and either altitude is `<= 0` or the bridge-deck crossing byte is set.
9. State 5 callsite `0x0054CE34` passes `(ESI, -1, -1, 0, 1)`.
10. State 5 target `ESI` is produced from the linked object's current coordinate.
11. State 5 can transition to state 4 after the check or to state 6 terminal crash/abort.

Inference:

1. Human-readable names "landing-abort/retry flag" for `instance + 0x90`, "abort/emergency request" for `linked + 0x425`, and "landing allowed/armed" for `linked + 0x427` are inferred from usage and prior reports.
2. Exact per-unit frequency in normal skirmish depends on rules flags and missions. State 4 is high-confidence common for landable jumpjet flows; state 5 is medium-high because it is flag/abort dependent.
3. `DAT_00ABC5DC` is the Jumpjet bridge altitude threshold, already established in prior reports. This pass verifies its use in the state 5 bridge-plane crossing gate.

## Open Questions

1. Fully enumerate writers of linked object bytes `+0x425`, `+0x427`, and `+0x6AD` to name every gameplay trigger of state 5.
2. Verify with runtime observation which standard YR units hit state 4 voluntary landing most often, given `BalloonHover` and `Landable` variation.
3. Verify whether Magnetron/forced-lift Jumpjet piggyback paths reuse the same state 5 bridge-deck crossing branch in standard YR.
4. Identify helper `0x004834A0` argument semantics in the state 5 post-call checks if a later implementation needs exact non-bridge landing failure behavior.

## Recommended Next Audit

`BRIDGE_JUMPJET_ABORT_FLAG_WRITERS_GHIDRA_REPORT.md`

Scope: xref/write audit for linked object bytes `+0x425`, `+0x427`, and `+0x6AD`, plus state 5 entry triggers from Magnetron, deploy, crash, and invalid-destination flows. This would convert state 5 gameplay trigger confidence from MEDIUM-HIGH to HIGH.

## Files / Implementation Impact

No Rust implementation files were changed.

This research document is intended to guide a future implementation only after the user explicitly asks for code changes.

