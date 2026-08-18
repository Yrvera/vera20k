# DRIVE_VTABLE_0x80_HAS_CARGO_PREDICATE — Ghidra Report

**Target:** DriveLocomotionClass vtable slot +0x80 called from `DriveLocomotionClass::Process`
(`0x004B0500`) at the Tiberium spill animation branch.

**Investigation date:** 2026-05-19
**Confidence:** HIGH (verified from binary disassembly)

---

## 1. Executive Summary

The function at vtable+0x80 is **`DriveLocomotionClass::Is_Moving_Now`** at address
**`0x004afc20`**. It is a DriveLocomotionClass method (ILocomotion interface slot),
called on the locomotor object itself, returning `bool`. It does NOT check cargo state.
It returns `true` if the unit is currently executing active movement (slope transition
or destination set with positive speed). The "has cargo" label in the task brief was a
misidentification; the predicate is strictly a movement-state gate.

---

## 2. Object Layout and Vtable Identification

`DriveLocomotionClass` constructor (`0x004af540`) assigns three vtable pointers:

| Offset in object | Vtable address | Interface      |
|-----------------|----------------|----------------|
| +0x00           | `0x007e7f7c`   | IUnknown       |
| +0x04           | `0x007e7eb0`   | ILocomotion    |
| +0x18           | `0x007e7e8c`   | IPiggyback     |

**Critical:** `DriveLocomotionClass::Process` (`0x004b0500`) is dispatched via the
ILocomotion vtable (slot `0x40` from `0x7e7eb0 + 0x40 = 0x7e7ef0`). When Process is
called, the caller passes `ILocomotion_this = object + 4` as the argument. Inside
Process, `ESI = object + 4`, so `[ESI] = ILocomotion vtable = 0x7e7eb0`.

Verified: constructor ASM at `004af5c8` sets `[ESI+4] = 0x7e7eb0`, and Locomotion_AI
at `0x00520f40` calls `(*(int *)param_1[0x19d] + 0x10)((int *)param_1[0x19d])`,
passing the locomotor object pointer directly.

---

## 3. Vtable Slot Resolution

In Process (`0x004b0500`), the spill branch begins at `LAB_004b078c`:

```asm
004b078c: MOV EAX, [ESI]           ; EAX = [ESI] = ILocomotion vtable = 0x7e7eb0
004b078e: PUSH ESI                  ; push locomotor+4 as stack argument
004b078f: CALL dword ptr [EAX+0x80] ; call vtable slot 0x80
004b0795: TEST AL, AL               ; test return value (bool)
```

`[ILocomotion_vtable + 0x80]` = `[0x7e7eb0 + 0x80]` = `[0x7e7f30]`:

```
read_memory(0x7e7f30, 4) = [20, fc, 4a, 00] = 0x004afc20
```

**Function pointer confirmed at `0x7e7f30` → `0x004afc20` = `DriveLocomotionClass::Is_Moving_Now`.**

This is an ILocomotion vtable slot (slot 32, offset 0x80 from the ILocomotion vtable
base). The call is `(*(locomotor_ILocomotion_vtable + 0x80))(this = locomotor+4)`.
It is NOT a TechnoClass virtual; it is a DriveLocomotionClass ILocomotion virtual.

---

## 4. Function Analysis: `DriveLocomotionClass::Is_Moving_Now` (`0x004afc20`)

### Signature (verified from disassembly)
```
bool __stdcall DriveLocomotionClass__Is_Moving_Now(int *locomotor_iface)
; arg1 [ESP+4] = locomotor+4 (ILocomotion interface this-pointer)
; returns: AL = 1 (moving), 0 (not moving)
```

Note: Ghidra labels it `__thiscall` but the raw asm loads `[ESP+8]` (the push argument)
into ESI, ignoring ECX. It is effectively `__stdcall` with respect to the `this` pointer.

### Logic (verified from disassembly at `0x004afc20`–`0x004afc84`)

```
004afc21: MOV ESI, [ESP+8]         ; ESI = locomotor+4 (the pushed arg)
004afc25: MOV ECX, [ESI+8]         ; ECX = locomotor+4+8 = locomotor+12 = owner_unit
004afc28: ADD ECX, 0x388           ; ECX = owner+0x388 = owner's slope CDTimer
004afc2e: CALL CDTimerClass::Remaining  ; is slope transition in progress?
004afc33: TEST AL, AL
004afc35: JZ .check_destination    ; no → check destination
004afc37: MOV AL, 1
004afc39: POP ESI; RET 4           ; → RETURN TRUE (slope transition active)

.check_destination:
004afc3d: MOV EAX, [ESI]           ; EAX = ILocomotion vtable
004afc3f: PUSH ESI
004afc40: CALL [EAX+0x10]          ; Is_Moving (locomotor flag at [ESI+0x11])
004afc43: TEST AL, AL
004afc45: JZ .return_false         ; locomotor flag not set → not moving
004afc47: MOV ECX, [ESI+0x3c]     ; head_to destination X (locomotor+0x40)
004afc4a-004afc6a: compare X,Y,Z against g_NullCoord
          JZ .return_false         ; if head_to == null → not moving
004afc6c: MOV ECX, [ESI+8]        ; ECX = owner_unit
004afc6f: MOV EDX, [ECX]
004afc71: CALL [EDX+0x538]         ; owner vtable+0x538 = GetCurrentSpeed (returns int)
004afc77: TEST EAX, EAX
004afc79: JLE .return_false        ; speed <= 0 → not moving
004afc7b: MOV AL, 1
004afc7d: POP ESI; RET 4           ; → RETURN TRUE (driving toward destination)

.return_false:
004afc81: XOR AL, AL; RET 4        ; → RETURN FALSE
```

**Return value:** `bool` — 1 (moving), 0 (stopped).
- Returns `true` in two cases:
  1. Owner unit's slope CDTimer (`owner+0x388`) has time remaining (slope/ramp transition).
  2. Locomotor's `Is_Moving` flag is set AND head_to destination is non-null AND owner speed > 0.

---

## 5. Spill Branch Full Condition (verified)

```c
// LAB_004b078c in DriveLocomotionClass::Process
cVar3 = Is_Moving_Now(locomotor+4);         // vtable+0x80: "is unit driving?"
if (cVar3 != 0
    && (g_CurrentFrameCounter % 10 == 0)    // every 10 frames
    && (owner[0x23] == 0))                   // not cloaked (owner+0x8C)
{
    cell = owner->vtable->Get_Current_Cell();   // owner vtable+0x1BC
    if (cell[0xec] == 2                         // CellClass+0xEC = overlay type = 2 (Blue Tiberium)
        && Rules+0x94 != 0)                     // Rules.TiberiumSpill anim defined
    {
        spawn AnimClass(Rules+0x94, owner_coord_XY, layer=0x600);
    }
}
```

The predicate at vtable+0x80 is purely a movement gate, not a cargo gate.
The spill fires: while actively driving (not parked), every 10 frames, when over
Blue Tiberium (`overlay_type == 2`), and the unit is not cloaked.

---

## 6. Caller Verification

Only callee via vtable (no static callers found via `get_function_callers`).
Cross-checked by reading adjacent ILocomotion vtable slots:
- `0x7e7f2c` (slot 0x7C) = `LocomotionClass__ForEach_SetSlopeIndex` (`0x004e1570`) — confirmed LocomotionClass method immediately before.
- `0x7e7f34` (slot 0x84) = `0x0055ad10` — adjacent slot also in ILocomotion range.

`Is_Moving_Now` callers confirmed within `DriveLocomotionClass::Process` at two call
sites: `0x004b059e` (vtable+0x10 = `ILocomotion::Is_Moving`) and `0x004b078f` (vtable+0x80).

---

## 7. Active in YR: YES

**Active in YR: Yes, unconditionally.**
- No flag gate, no SpecialFlags check.
- The only gate is `Rules+0x94 != 0` (TiberiumSpill animation defined) — which defaults
  to a valid anim in stock YR (`[General] TiberiumSpill=TIBSPL` or similar).
- Fires every match whenever any DriveLocomotionClass unit drives over a Blue Tiberium
  overlay cell, every 10 frames. Trigger frequency: moderate (whenever such units
  transit Blue Tiberium tiles; less common than standard Tiberium but not negligible).

---

## 8. Rust Port Implication

The spill branch is unimplemented in our Rust port. Implementation requires:
1. At the end of each tick, after movement step, if `locomotor.is_moving_now()`:
   - Frame % 10 == 0
   - Unit not cloaked
   - Current cell overlay type == 2 (Blue Tiberium)
   - `rules.tiberium_spill_anim` is `Some`
   → Spawn the TiberiumSpill anim at `(owner.pos.x, owner.pos.y)` in layer `0x600`.
2. `is_moving_now()` in Rust = slope CDTimer remaining OR (is_moving flag AND head_to != null AND speed > 0).
3. `rules.tiberium_spill_anim` = `RulesClass+0x94` parsed from `[General] TiberiumSpill=`.

---

## 9. Key Facts (load-bearing, verified)

1. **`[0x7e7f30]` = `0x004afc20`** — confirmed by `read_memory(0x7e7f30, 4)` = `[0x20,0xfc,0x4a,0x00]`.
2. **`DriveLocomotionClass::Is_Moving_Now` at `0x004afc20`** — confirmed by `get_function_by_address`.
3. **ESI = locomotor+4 (ILocomotion interface) in Process** — confirmed by `DriveLocomotionClass::Process` (`0x004b0500`) call setup: `PUSH ESI; CALL [EAX+0x80]` where ESI is the ILocomotion `this`.
4. **ILocomotion vtable = `0x7e7eb0`, slot 0x80 offset = `0x7e7f30`** — confirmed by constructor at `0x004af5c8` `MOV [ESI+4], 0x7e7eb0` and `0x7e7eb0 + 0x80 = 0x7e7f30`.
5. **`Rules+0x94` = TiberiumSpill anim** — confirmed by `CMP [EAX+0x94], EBX` at `004b07cf` in Process, with `EAX = g_RulesClass_Instance` at `0x008871e0`.

---

*Report generated 2026-05-19 from live Ghidra decompilation of gamemd.exe.*
