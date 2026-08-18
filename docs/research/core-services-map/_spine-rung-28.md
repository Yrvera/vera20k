# LogicClass::PerTickUpdate — Rung 28 (AB): Last-ref-object camera/audio follow + temp-vector teardown

**Status:** VERIFIED from binary this session.
**Parent:** `LogicClass::PerTickUpdate` @ `0x0055AFB0` (label `LogicClassPerTickUpdateLiveVector`).
**Authority:** binary -> Ghidra. Body site keyed to **disassembly** at
`disassemble_function 0x0055AFB0`; gate driver `disassemble_function 0x004AEB10`;
follow driver `decompile_function 0x006D6070`; temp-free `decompile_function 0x007C8B3D`.

---

## Order / position

- **Order:** 28 of 28 — the **last** rung of `LogicClass::PerTickUpdate`.
- Runs immediately **after** Rung AA (HouseClass tick, per-entry `vt+0x5c` over
  `g_HouseClass_Array`, body `0055b68d`–`0055b6b1`). Nothing follows it; the function
  epilogue (`POP/ADD ESP,0x28/RET`) is at `0055b719`–`0055b71c`.

## Body site (exact)

`disassemble_function 0x0055AFB0`, instructions `0055b6b3`–`0055b71c`:

```
0055b6b3  MOV  ECX,0x87f7e8          ; receiver = DisplayClass singleton (0x0087f7e8), __fastcall this
0055b6b8  CALL 0x004aeb10            ; GetLastRefObject  (GATE: returns last-ref object ptr or 0)
0055b6bd  POP  EDI                   ; (epilogue regs restored early; EAX still holds the result)
0055b6be  POP  ESI
0055b6bf  POP  EBP
0055b6c0  POP  EBX
0055b6c1  TEST EAX,EAX               ; GATE: is there a live last-ref object?
0055b6c3  JZ   0x0055b6f8            ;   no -> skip the follow entirely
0055b6c5  MOV  ECX,0x87f7e8          ;   yes -> re-fetch the object ptr (second GetLastRefObject call)
0055b6ca  CALL 0x004aeb10
0055b6cf  ADD  EAX,0x9c              ; point at object coord triple obj+0x9c/+0xa0/+0xa4
0055b6d4  MOV  ECX,[EAX]             ; obj+0x9c   (X / cell-lepton coord component)
0055b6d6  MOV  [ESP+0x4],ECX         ; -> local copy (uStack_24)
0055b6da  LEA  ECX,[ESP+0x4]         ; &copy = arg2 for follow driver
0055b6de  MOV  EDX,[EAX+0x4]         ; obj+0xa0   (local_20)
0055b6e1  PUSH ECX
0055b6e2  MOV  ECX,[0x00887324]      ; receiver = g_Tactical (*0x00887324), __fastcall this for follow
0055b6e8  MOV  [ESP+0xc],EDX         ; copy obj+0xa0
0055b6ec  MOV  EAX,[EAX+0x8]         ; obj+0xa4   (uStack_1c)
0055b6ef  MOV  [ESP+0x10],EAX        ; copy obj+0xa4
0055b6f3  CALL 0x006d6070            ; FUN_006d6070: scroll/recenter camera onto the object  (THIS RUNG)
0055b6f8  MOV  EAX,[ESP+0x14]        ; local_14 = Rung-L (TeamClass) temp vector buffer ptr
0055b6fc  MOV  [ESP+0x10],0x7e9f84   ; restore vtable slot of the local DynamicVector (PTR_FUN_007e9f84)
0055b704  TEST EAX,EAX               ; temp buffer non-null?
0055b706  JZ   0x0055b719
0055b708  MOV  CL,[ESP+0x1d]         ; local_b = "buffer is owned/heap-allocated" flag
0055b70c  TEST CL,CL
0055b70e  JZ   0x0055b719
0055b710  PUSH EAX
0055b711  CALL 0x007c8b3d           ; free the Rung-L temp vector heap buffer
0055b716  ADD  ESP,0x4
0055b719  ADD  ESP,0x28
0055b71c  RET
```

- **Two halves, both verified:**
  1. **Camera follow** (`0055b6b3`–`0055b6f3`): gated by the last-ref object existing.
  2. **Temp-vector teardown** (`0055b6f8`–`0055b716`): frees the `local_14` heap buffer
     that Rung L (`FUN_0055bb40` @ `0055b4fd`) allocated to hold the filtered TeamClass
     temp list. Gated by `local_14 != 0 && local_b != 0` (buffer present **and** owned).
     This is plain cleanup, runs every tick a temp buffer was allocated; no RNG, no
     observable game effect (heap hygiene only).

- **Receiver(s) confirmed:**
  - `GetLastRefObject` (`0x004AEB10`) called with `ECX = 0x0087f7e8` — the **DisplayClass
    singleton**. Same `0x87f7e8` receiver used by every prior g_Tactical/Display rung
    (Rung B/G `FUN_004f42f0`, Rung X `UpdateCrateRegenTimers`, Rung Y `g_Tactical->vt+0x5c`).
    (`0x0087f7e8` reads as zero at static-analysis time — singleton is runtime-constructed.)
  - `FUN_006d6070` (`0x006D6070`) called with `ECX = *0x00887324` — `g_Tactical`, the
    separately-stored DisplayClass pointer used as the scroll/tactical-view receiver.
    Matches the spine ladder's `g_Tactical (0x00887324)` for the adjacent Rung Y.

## Purpose (one line)

Each tick, if a "last-referenced object" is registered on the DisplayClass, **recenter the
tactical view (camera scroll target + minimap rect) onto that object's world coords**; then
**free the per-tick temp vector buffer** that Rung L allocated for its filtered TeamClass list.

## Gate (confirmed / corrected)

**Spine prompt said:** "gate last-ref object non-null (Display+0x119c set and +0x11a0 != 0);
temp vector owned (local_b)." **Confirmed exact.**

- `DisplayClass__GetLastRefObject` @ `0x004AEB10` (`disassemble_function 0x004AEB10`):
  ```
  004aeb10  MOV  AL,byte ptr [ECX + 0x119c]   ; has_last_ref bool
  004aeb16  TEST AL,AL
  004aeb18  JZ   0x004aeb24                   ; if !has_last_ref -> return 0
  004aeb1a  MOV  EAX,dword ptr [ECX + 0x11a0] ; the last-ref object POINTER
  004aeb20  TEST EAX,EAX
  004aeb22  JNZ  0x004aeb26                   ; if ptr != 0 -> return ptr
  004aeb24  XOR  EAX,EAX                       ; else return 0
  004aeb26  RET
  ```
  So the gate is exactly `Display+0x119c != 0  AND  Display+0x11a0 != 0`. The follow
  (`FUN_006d6070`) only runs when both hold.
- The temp-free is independently gated by `local_14 != 0` (`[ESP+0x14]`) and `local_b != 0`
  (`[ESP+0x1d]`), i.e. the Rung-L temp buffer was both allocated and is heap-owned.

### What "last-ref object" is (offset correction vs DISPLAYCLASS doc)

`+0x11a0` holds a **raw object pointer**, not an index, and `+0x119c` is the
"has last-ref" bool. Confirmed by the writer `DisplayClass__SetLastRefObject` @ `0x004AEB30`
(`decompile_function 0x004AEB30`):
```
*(int *)(this + 0x11a0) = obj_ptr;
*(bool *)(this + 0x119c) = (obj_ptr != 0);
```
`SetLastRefObject(0)` is called from `ObjectClass__Deselect` (`0x005F44A0`) and
`ObjectClass__Destroy` (`0x005F5280`) when the deselected/destroyed object IS the current
last-ref, clearing the dangling reference (`get_function_callers 0x004AEB10`). So the
last-ref object is the **most-recently selected/referenced live object**.

> **LABEL-DRIFT NOTE (correct the DISPLAYCLASS doc):**
> `DISPLAYCLASS_GHIDRA_REPORT.md` (lines ~84–85, ~547–551) states `GetLastRefObject` reads
> `+0x1198` (`last_ref_object_idx`, an int index). **WRONG.** `disassemble_function
> 0x004AEB10` proves it reads `+0x119c` (bool) and `+0x11a0` (a **pointer**, not an index).
> `+0x1198` is a separate field this function never touches. The same doc's caller table
> (line ~567) frames this rung as "Recenter tactical view **after save/load**" — also too
> narrow: it recenters **every tick** there is a last-ref object, not only post-load.

## Follow driver — `FUN_006d6070` @ `0x006D6070`

`decompile_function 0x006D6070` (`__thiscall`, `this = g_Tactical`, `param_2 = &coordCopy`):

- Reads the object coord copy `param_2[0]` (obj+0x9c) and `param_2[1]` (obj+0xa0).
- Computes an **isometric cell-to-pixel** scroll target:
  `local_8 = (X*0x3c)/2 + (Y*-0x3c)/2`, `iVar3 = (X*0x1e)/2 + (Y*0x1e)/2`
  (the standard 60x30 half-tile iso transform), with `>>8` lepton-to-pixel scaling.
- `Tactical__AdjustForZ` @ `0x006D20E0` adjusts the Y for bridge/cliff Z height
  (`decompile_function 0x006D20E0` — just a `Math__ftol`, no RNG).
- Clamps the target to the map viewport via `FUN_006d8640` @ `0x006D8640`
  (`decompile_function 0x006D8640` — viewport-edge clamp math, no RNG); skips the clamp
  result when `g_IsMapEditor != 0`.
- Writes the scroll target to `g_Tactical+0xd64/+0xd68` and the "follow" target
  `+0xd74/+0xd78`, recomputes the minimap/radar rect via `FUN_006d8b30` @ `0x006D8B30`
  (`Matrix3x4_TransformPoint` + `Math__ftol`, no RNG), and sets the "scroll dirty" flag
  `g_Tactical+0xd7d = 1`. (`+0xd64` matches MEMORY note: "+0xD64 is g_Tactical scroll".)

Net observable effect: the tactical camera scrolls to keep the last-referenced object in
view (the in-game "scroll-to / center-on selected object" behavior).

## Temp-free driver — `FUN_007C8B3D` @ `0x007C8B3D`

`decompile_function 0x007C8B3D` -> tail-calls `FUN_007C93E8` @ `0x007C93E8`
(`decompile_function 0x007C93E8`): an `operator delete` wrapper — enters a heap critical
section (`FUN_007cd9f5(9)` / `FUN_007cda56(9)`), and `HeapFree(DAT_00b78b9c, 0, buf)`.
Pure heap cleanup of the Rung-L `local_14` buffer. No RNG, no game state, no observable
effect.

## RNG

- **Draws RNG:** **NO.**
- **rng_stream:** `none`.
- Walked the entire rung-28 call tree for randomness:
  - `GetLastRefObject` (0x004AEB10) — two field reads + compares. No RNG.
  - `FUN_006d6070` (0x006D6070) and its callees `Tactical__AdjustForZ` (0x006D20E0),
    `FUN_006d8640` (0x006D8640), `FUN_006d8b30` (0x006D8B30) — deterministic
    iso/viewport/matrix camera math (`Math__ftol`, `Matrix3x4_TransformPoint`). No RNG.
  - `FUN_007c8b3d` (0x007C8B3D) -> `FUN_007c93e8` (0x007C93E8) — heap free
    (`HeapFree`). No RNG.
- **Lockstep:** this rung consumes **0 RNG draws**, so it does not advance any RNG stream
  (`Scen->Random`, `g_MainRng`, or `g_MapGenRng`). Its effects (camera scroll target,
  minimap rect, heap free) are **view/UI-local and non-deterministic-safe** — they read sim
  state but write only DisplayClass/g_Tactical view fields, never sim state. Order relative
  to neighbors is irrelevant to the RNG-draw contract (no draws), but it correctly runs
  last so the camera follows the post-tick object position.

## Active-in-YR / Tiberian Sun legacy

- **active_in_yr: yes (conditional on a last-ref object existing).** The camera-follow half
  fires in a normal YR skirmish whenever a last-ref object is set (i.e. an object has been
  selected/referenced and not since cleared) — directly player-visible as the tactical view
  recentering on it. The temp-free half runs whenever Rung L allocated a temp buffer.
- **ts_legacy: NO.** This is live RA2/YR display plumbing (DisplayClass/g_Tactical scroll +
  minimap), not gated behind any SpecialFlags/FogOfWar bit, not a TS-only path. Reachable
  and visible every match.

## walks (one phrase)

Single object: reads the last-ref object's coord triple at `obj+0x9c/+0xa0/+0xa4`
(`0055b6cf`–`0055b6ef`) and recenters the camera on it; then frees one heap buffer
(the Rung-L temp vector). Not a vector walk.

## Ghidra calls cited

- `disassemble_function 0x0055AFB0` — body site `0055b6b3`–`0055b71c` (order, receivers, gate).
- `disassemble_function 0x004AEB10` — gate driver field reads `+0x119c`/`+0x11a0`.
- `decompile_function 0x004AEB30` — `SetLastRefObject` (proves `+0x11a0` = pointer).
- `get_function_callers 0x004AEB10` — Deselect/Destroy clear last-ref (semantics).
- `decompile_function 0x005F44A0` — `ObjectClass__Deselect` (confirms last-ref = selection).
- `decompile_function 0x006D6070` + callees `0x006D20E0`, `0x006D8640`, `0x006D8B30` — follow math, no RNG.
- `decompile_function 0x007C8B3D` -> `0x007C93E8` — temp-vector heap free, no RNG.
