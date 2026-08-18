> **RAW LANE OUTPUT — NOT THE AUTHORITY.** This is the unedited lane 3 (Walk flag/head-to lifetime) from the
> 2026-07-30 slot-32 investigation, kept for its per-call citations. It was written before
> the adversarial verify pass, and **some claims in it were subsequently refuted** — the
> verifiers found, among other things, a wrong vtable-offset constant, several claims with
> no citation, at least one omitted term in a decoded predicate, and an under-count of the
> gate's call sites.
>
> **Read `ILOCOMOTION_IS_MOVING_NOW_SLOT32_AND_MISSION_GATE_GHIDRA_REPORT.md` for the
> settled findings.** Treat anything here that the consolidated report does not repeat as
> UNCHECKED, and re-verify from the binary before relying on it.

---

# LANE 3 — Walk locomotor: moving flag + head-to lifetime

Session date 2026-07-29. Program: testProsjekt / gamemd.exe, image base 0x400000.
READ-ONLY session; no Ghidra mutations performed.

## CRITICAL FRAME NOTE (read before any offset below)

WalkLocomotionClass is a multiple-inheritance COM object. Its **object base** and its
**ILocomotion interface pointer** are 4 bytes apart:

- `obj+0x00` = IUnknown vtable ptr
- `obj+0x04` = ILocomotion vtable ptr  →  **ILocomotion `this` = obj + 4**

Verified via `disassemble_function 0x0075aa90` (constructor, __fastcall ECX=obj base):
`MOV dword ptr [ESI],0x7f6ac4` / `MOV dword ptr [ESI+0x4],0x7f69f8` /
`MOV dword ptr [ESI+0x18],0x7f69d4`.

Every ILocomotion vtable method takes the **iface** pointer (`[ESP+4]`), so a method's
`+0x30` is the object's `+0x34`. Two prior-session offset sets can look contradictory
purely because of this 4-byte shift. All offsets below are labelled `iface+` or `obj+`
explicitly.

---

# VERIFIED

## V1. Walk ILocomotion vtable base = 0x007f69f8

From the constructor store `MOV dword ptr [ESI+0x4],0x7f69f8`
(`disassemble_function 0x0075aa90`).

Read 160 bytes = 40 slots via `read_memory 0x007f69f8 length=160`. Decoded:

| slot | target | note |
|---|---|---|
| 0 | 0x0075cc30 | IUnknown (Walk-specific) |
| 1 | 0x0075cc40 | IUnknown |
| 2 | 0x0075cc50 | IUnknown |
| 3 | 0x0055a710 | base LocomotionClass |
| **4** | **0x0075ab30** | **Is_Moving** — matches session anchor ✓ |
| 5 | 0x0075aba0 | Destination (verified below) |
| 6 | 0x0075ac00 | (Walk-specific) |
| **7** | **0x0055abf0** | base Can_Enter_Cell — **alignment check PASSES** ✓ |
| 8 | 0x0055abe0 | base |
| 9 | 0x0055a730 | base |
| 10 | 0x0055a7d0 | base |
| 11 | 0x0055abd0 | base |
| 12 | 0x0055a8c0 | base |
| 13 | 0x0055abc0 | base |
| 14 | 0x0055aba0 | base |
| 15 | 0x0055abb0 | base |
| 16 | 0x0075ac80 | Walk-specific |
| 17 | 0x0075acb0 | Walk-specific |
| 18 | 0x0075ada0 | Walk-specific |
| 19 | 0x0075ae00 | Walk-specific |
| 20 | 0x0055ac20 | base |
| 21 | 0x0055ab90 | base |
| 22 | 0x0055a8f0 | base |
| 23 | 0x0055a910 | base |
| 24 | 0x0055a930 | base |
| 25 | 0x0055a940 | base |
| 26 | 0x0055ab70 | base |
| 27 | 0x0055ab80 | base |
| 28 | 0x0055ac10 | base |
| 29 | 0x0075c7e0 | Walk-specific |
| 30 | 0x0075ae30 | Walk-specific |
| 31 | 0x0055ace0 | base |
| **32** | **0x0075ab40** | **Is_Moving_Now (Walk)** |
| 33 | 0x0055ad10 | base |
| 34 | 0x0055acf0 | base |
| 35 | 0x0055ad00 | base |
| 36 | 0x004b4c60 | shared/other |
| 37 | 0x004b4c70 | shared/other |
| 38 | 0x004b4c80 | shared/other |
| 39 | 0x0075ca30 | Walk-specific |

Slot 7 reading 0x0055abf0 confirms the base is correctly aligned, so slot 32 =
**0x0075ab40** is a trustworthy read (not an off-by-one).

## V2. Walk struct field map (partial, from constructors)

`disassemble_function 0x0075aa90` (WalkLocomotionClass ctor) and
`decompile_function 0x0055a6c0` (LocomotionClass base ctor).

Base ctor (`param_1` is `undefined4*`, so index*4):
- `obj+0x08` = 0
- `obj+0x0C` = 0   (plate comment claims linked TechnoClass owner — UNCHECKED by me)
- `obj+0x10` = byte 1
- `obj+0x11` = byte 1
- `obj+0x14` = 0

Walk ctor raw stores (ESI = obj base):
- `obj+0x00` = 0x007f6ac4  (IUnknown vtable)
- `obj+0x04` = 0x007f69f8  (ILocomotion vtable)
- `obj+0x18` = 0x007f69d4  (IPiggyback vtable)
- `obj+0x1C / +0x20 / +0x24` = NullCoord triple, from globals
  `[0x00b45be8] / [0x00b45bec] / [0x00b45bf0]`   → **COORD A**
- `obj+0x28 / +0x2C / +0x30` = same NullCoord triple                → **COORD B**
- `obj+0x34` = byte 0
- `obj+0x35` = byte 0
- `obj+0x36` = byte 0
- `obj+0x38` = dword 0

Translated to the **iface** frame used by every vtable method (`iface = obj+4`):

| iface off | obj off | meaning |
|---|---|---|
| +0x14 | +0x18 | IPiggyback vtable ptr |
| +0x18/+0x1C/+0x20 | +0x1C/+0x20/+0x24 | **COORD A** (Destination source, see V4) |
| +0x24/+0x28/+0x2C | +0x28/+0x2C/+0x30 | **COORD B** |
| **+0x30** | **+0x34** | **the "is moving" byte read by slot 4** |
| +0x31 | +0x35 | byte, ctor-zeroed |
| **+0x32** | **+0x36** | **the "is moving here" byte** |
| +0x34 | +0x38 | dword, ctor-zeroed |

The NullCoord globals are 0x00b45be8/bec/bf0 (`disassemble_function 0x0075aa90`).

## V3. Walk slot 4 (Is_Moving) = the +0x30 byte, nothing else

`disassemble_function 0x0075ab30`:
```
0075ab30: MOV EAX,dword ptr [ESP + 0x4]
0075ab34: MOV AL,byte ptr [EAX + 0x30]
0075ab37: RET 0x4
```
Confirms the session anchor. iface+0x30 == obj+0x34.

## V4. Walk slot 5 = Destination(iface, CoordStruct* out) — gated by slot 4

`disassemble_function 0x0075aba0`:
```
0075aba0: PUSH ESI
0075aba1: MOV ESI,dword ptr [ESP + 0x8]      ; = arg1 = iface (this)
0075aba5: PUSH EDI
0075aba6: PUSH ESI
0075aba7: MOV EAX,dword ptr [ESI]
0075aba9: CALL dword ptr [EAX + 0x10]        ; slot 4 = Is_Moving
0075abac: TEST AL,AL
0075abae: JZ 0x0075abcc
0075abb0: MOV EAX,dword ptr [ESP + 0x10]     ; = arg2 = out CoordStruct*
0075abb4: MOV ECX,dword ptr [ESI + 0x18]     ; COORD A .x
0075abb7: MOV EDX,dword ptr [ESI + 0x1c]     ; COORD A .y
0075abba: MOV ESI,dword ptr [ESI + 0x20]     ; COORD A .z
... stores to [out],[out+4],[out+8]; RET 0x8
0075abcc: (else) *out = NullCoord from [0x00b45be8/bec/bf0]; RET 0x8
```

**This is a load-bearing structural fact:** Walk's Destination coord is *not* independently
nulled. `Destination()` returns COORD A only while the +0x30 byte is non-zero, and returns
NullCoord otherwise. So any consumer that tests "is the destination null?" on a walker is
transitively testing the same +0x30 byte. Coord-nullness and moving-ness cannot disagree
for Walk through this accessor.

## V5. LABEL CHECK — Get/Clear_Is_Moving_Here read +0x32, NOT the +0x30 moving byte

`decompile_function 0x0075cb20` → `return *(undefined1 *)(param_1 + 0x32);`
`decompile_function 0x0075cbc0` → `*(undefined1 *)(param_1 + 0x32) = 0;`

(`param_1` is typed `int` in both, so these are literal byte offsets in the **iface**
frame = obj+0x36.)

So the lane's suspicion is correct in effect: **"Is_Moving_Here" is a different byte from
the slot-4 moving byte.** iface+0x32 vs iface+0x30. Whether +0x32 denotes cell occupancy
specifically is addressed below; what is certain is that neither Get_ nor Clear_
Is_Moving_Here touches the byte slot 4 reads.

## V6. Walk slot 32 (Is_Moving_Now) = 0x0075ab40 — THREE conjuncts, one of them a coord

`disassemble_function 0x0075ab40` (authoritative) + `decompile_function 0x0075ab40`:

```
0075ab40: PUSH ESI
0075ab41: MOV ESI,dword ptr [ESP + 0x8]     ; iface (this)
0075ab45: PUSH ESI
0075ab46: MOV EAX,dword ptr [ESI]
0075ab48: CALL dword ptr [EAX + 0x10]       ; slot 4 = Is_Moving  (the +0x30 byte)
0075ab4b: TEST AL,AL
0075ab4d: JZ  0x0075ab90                    ; -> return 0
0075ab4f: MOV ECX,dword ptr [ESI + 0x8]     ; owner TechnoClass ptr (iface+0x08 = obj+0x0C)
0075ab52: FLD  double ptr [ECX + 0x578]     ; owner's double at +0x578
0075ab58: FCOMP double ptr [0x007e2800]     ; compare against 0.0
0075ab5e: FNSTSW AX
0075ab60: TEST AH,0x41                      ; C0 (0x01) | C3 (0x40)
0075ab63: JNZ 0x0075ab90                    ; -> return 0  if (< 0.0) or (== 0.0) or unordered
0075ab65: MOV EDX,dword ptr [ESI + 0x24]    ; COORD B .x
0075ab68: MOV EAX,[0x00b45be8]              ; NullCoord .x
0075ab6d: CMP EDX,EAX
0075ab6f: JNZ 0x0075ab8a                    ; -> return 1
0075ab71: MOV EAX,dword ptr [ESI + 0x28]    ; COORD B .y
0075ab74: MOV ECX,dword ptr [0x00b45bec]
0075ab7a: CMP EAX,ECX
0075ab7c: JNZ 0x0075ab8a                    ; -> return 1
0075ab7e: MOV ECX,dword ptr [ESI + 0x2c]    ; COORD B .z
0075ab81: MOV EAX,[0x00b45bf0]
0075ab86: CMP ECX,EAX
0075ab88: JZ  0x0075ab90                    ; all three equal -> return 0
0075ab8a: MOV AL,0x1  / RET 0x4
0075ab90: XOR AL,AL   / RET 0x4
```

`0x007e2800` is the same 0.0 constant the session already read for Fly (8 zero bytes).
`TEST AH,0x41` after `FCOMP` masks C0|C3, and the fallthrough (JNZ not taken) requires
C0=0 and C3=0, i.e. strictly **greater than** 0.0. Matches the decompiler's
`0.0 < *(double *)(param_1[2] + 0x578)`.

**Walk slot 32 ==  Is_Moving(+0x30 byte)  AND  (double)owner[+0x578] > 0.0
                   AND  COORD B (iface+0x24/0x28/0x2C) != NullCoord**

So the answer to lane question 4 is **yes, the gate's predicate for a walker DOES read a
coord field** — COORD B, the head-to/sub-step coord — in addition to the +0x30 byte and a
speed term. It is NOT "+0x30 byte plus a speed term" only.

`owner+0x578` is a `double` on the TechnoClass/FootClass owner. Its identity is INFERRED
as a speed magnitude (see INFERRED section) — I did not verify what writes it.

## V7. COORD A vs COORD B roles (both verified from bodies)

- **COORD A = iface+0x18/0x1C/0x20 (obj+0x1C/0x20/0x24)** — the commanded destination.
  Written by `Head_To_Coord` slot 17, read by `Destination` slot 5.
- **COORD B = iface+0x24/0x28/0x2C (obj+0x28/0x2C/0x30)** — the current head-to /
  sub-step coord. Read by `Head_To` slot 6 and by `Is_Moving_Now` slot 32.

`decompile_function 0x0075ac00` (slot 6, label `WalkLocomotionClass__Head_To`):
```c
if (COORD_B == NullCoord) { out = owner[+0x9C], owner[+0xA0], owner[+0xA4]; }
else                      { out = COORD_B; }
```
i.e. Walk's Head_To getter **never returns null** — it falls back to the owner's own
current coord. Only `Destination` (slot 5) can return NullCoord, and only because the
+0x30 byte is clear (V4).

## V8. WRITERS of the +0x30 moving byte — enumerated (iface frame +0x30 = obj+0x34)

### Writer 1: constructor 0x0075aa90 — `obj+0x34 = 0`
`disassemble_function 0x0075aa90`: `MOV byte ptr [ESI + 0x34],AL` with AL=0.

### Writer 2: `Head_To_Coord` slot 17 @ 0x0075acb0 — sets 1, and conditionally clears
`decompile_function 0x0075acb0` (param_1 typed `int` = iface):
```c
if (!owner->vt[0x37c]() && !owner->vt[0x1d4]() && !owner->vt[0x1d8]()) {
    COORD_A = (param2, param3, param4);                  // iface+0x18/0x1C/0x20
    if (COORD_A != NullCoord) {
        cell = MapClass__Get_CellClass_At_Coord(&coord);
        if (cell[0x140] & 0x100) COORD_A.z += FUN_006d2120();   // bridge/height adjust
        *(byte*)(iface+0x30) = 1;                        // <<< SET MOVING
        return;
    }
    // commanded destination is NULL  ==  "stop"
    if (COORD_B == NullCoord) {
        old = *(byte*)(iface+0x30);
        *(byte*)(iface+0x30) = 0;                        // <<< CLEAR MOVING
        if (old != 0) owner->vt[0x54c]();                // stopped-moving notify
    }
}
```
Two behaviours that matter to the port:
1. A non-null Head_To_Coord sets the moving byte to 1 **in the same call as the order** —
   before any Process tick runs.
2. A null Head_To_Coord (stop) **only clears the byte when COORD B is already null**. If
   the walker is mid-sub-step (COORD B non-null) the byte stays 1.
3. All three owner virtual predicates at owner-vtable byte offsets 0x37C, 0x1D4, 0x1D8
   can make Head_To_Coord a complete no-op — neither coord nor byte is touched. Their
   identities are UNCHECKED.

### Writer 3: `Stop_Moving` slot 18 @ 0x0075ada0 — same conditional clear
`decompile_function 0x0075ada0`:
```c
COORD_A = NullCoord;                       // unconditional
if (COORD_B == NullCoord) {
    *(byte*)(iface+0x30) = 0;              // <<< CLEAR MOVING
    *(byte*)(iface+0x32) = 0;              // <<< also clears the "is moving here" byte
    owner->vt[0x54c]();
}
```
Note Stop_Moving nulls COORD A unconditionally but leaves the moving byte set while
COORD B is non-null. This is the mechanism by which a stop order does not teleport-freeze
a walker mid-step.

## V9. `Process` slot 16 @ 0x0075ac80 uses +0x31 as a re-entrancy/in-process flag
`decompile_function 0x0075ac80`:
```c
*(byte*)(iface+0x31) = 1;
WalkLocomotionClass__ProcessMovement(...);
*(byte*)(iface+0x31) = 0;
slot4(this);                 // Is_Moving — value used as Process's return (see disasm)
```
So iface+0x31 (obj+0x35) is set only for the duration of the ProcessMovement call.

## V10. Walk function inventory (`search_functions name_pattern="Walk"`)
Addresses as reported by Ghidra symbols (labels are hints; bodies verified where cited):
`0075aa90` Constructor, `0075ab00` Destructor, `0075ab30` Is_Moving,
`0075ab40` Is_Moving_Now, `0075aba0` Destination, `0075ac00` Head_To,
`0075ac80` Process, `0075acb0` Head_To_Coord, `0075ada0` Stop_Moving,
`0075ae00` Set_Facing, `0075ae30` Mark_All_Occupation_Bits,
`0075aec0` ProcessMovement, `0075c240` FindSubCellDest, `0075c7e0`
Get_Locomotion_Type, `0075ca30` Power_On_Occupancy, `0075ca80` Is_At_Coord,
`0075cb20` Get_Is_Moving_Here, `0075cbc0` Clear_Is_Moving_Here,
`0075cbe0` Constructor(2nd), `0075cc30/40/50` QueryInterface/AddRef/Release.

The vtable-derived slot 32 (0x0075ab40) and the Ghidra label
`WalkLocomotionClass__Is_Moving_Now @ 0075ab40` **agree** — no drift here, unlike the
Hover case.

## V11. `Process` slot 16 returns Is_Moving, and calls ProcessMovement on the OBJ base

`disassemble_function 0x0075ac80` — this resolves the frame question for the whole family:
```
0075ac80: PUSH ESI
0075ac81: MOV ESI,dword ptr [ESP + 0x8]   ; ESI = iface
0075ac85: PUSH 0x1                        ; stack arg = 1
0075ac87: LEA ECX,[ESI + -0x4]            ; ECX = iface-4 = OBJECT BASE
0075ac8a: MOV byte ptr [ESI + 0x31],0x1
0075ac8e: CALL 0x0075aec0                 ; ProcessMovement(objbase /*ECX*/, 1 /*stack*/)
0075ac93: MOV byte ptr [ESI + 0x31],0x0
0075ac97: MOV EAX,dword ptr [ESI]
0075ac99: PUSH ESI
0075ac9a: CALL dword ptr [EAX + 0x10]     ; slot 4 = Is_Moving
0075ac9d: POP ESI
0075ac9e: RET 0x4                         ; returns Is_Moving's AL
```
So **`Process` returns Is_Moving (the +0x30 byte), not Is_Moving_Now**, and
`ProcessMovement` / `FindSubCellDest` are `__fastcall` on the **object base** (their `+0x28`
is COORD B, `+0x34` is the moving byte, `+0x0C` is the owner).

## V12. Complete writer set for the moving byte (mechanical enumeration)

`search_instructions mnemonic=mov operand_pattern="0x34]" function="WalkLocomotionClass__ProcessMovement"`
→ **9 matches, ALL of them `dword ptr [ESP + 0x34]`** (stack locals at 0075b030, 0075b51f,
0075b636, 0075b751, 0075b794, 0075b7be, 0075b85b, 0075ba5c, 0075c0cf). EBP is the object
base in this function (see the +0x36 hits below), so **ProcessMovement never writes the
moving byte directly.**

`search_instructions mnemonic=mov operand_pattern="0x34]" function="WalkLocomotionClass__FindSubCellDest"`
→ **0 matches** (334 instructions scanned). FindSubCellDest never writes the moving byte
either.

`search_instructions mnemonic=mov operand_pattern="0x30]" function="WalkLocomotionClass__Head_To_Coord"`
→ exactly 3: `MOV byte [ESI+0x30],0x1` @0075ad5a, `MOV AL,byte [ESI+0x30]` @0075ad74,
`MOV byte [ESI+0x30],0x0` @0075ad77.

**The moving byte has exactly four writers:**
1. `0x0075aa90` constructor — `obj+0x34 = 0`
2. `0x0075acb0` Head_To_Coord (slot 17) — `=1` @0075ad5a, `=0` @0075ad77 (conditional)
3. `0x0075ada0` Stop_Moving (slot 18) — `=0` (conditional on COORD B null)
4. `0x0075ae30` slot 30 — `=0` (conditional on COORD B **and** COORD A both null)

Everything else that "stops" a walker does so by calling Stop_Moving through the iface
vtable (`CALL [reg+0x48]`), which is byte 0x48 = slot 18. ProcessMovement does exactly that
at several exits.

## V13. Slot 30 @ 0x0075ae30 is a fourth writer — label is misleading

`disassemble_function 0x0075ae30` (iface frame; RET 0x10 = this + a by-value CoordStruct):
```
0075ae41: MOV ESI,dword ptr [ESP + 0x18]   ; ESI = arg1 = iface
0075ae4d: MOV BL,byte ptr [ESI + 0x30]     ; save old moving byte
0075ae56: LEA ECX,[ESI + -0x4]             ; obj base
0075ae5d: CALL 0x0075c240                  ; FindSubCellDest(objbase)   <-- may SET COORD B
0075ae67..0075ae95:  COORD B == NullCoord ? && COORD A == NullCoord ?  (else skip to ret)
0075ae97: TEST BL,BL
0075ae99: MOV byte ptr [ESI + 0x30],0x0    ; <<< CLEAR MOVING (unconditional once here)
0075ae9d: JZ 0x0075aeaa
0075ae9f: MOV ECX,dword ptr [ESI + 0x8]    ; owner
0075aea4: CALL dword ptr [EDX + 0x54c]     ; stopped-moving notify, only if it *was* moving
```
This also independently **confirms FindSubCellDest is `__fastcall` on the object base**
(`LEA ECX,[ESI-0x4]` before the call).

The Ghidra label `WalkLocomotionClass__Mark_All_Occupation_Bits` does not describe this
body: it re-derives the sub-cell destination and conditionally clears the locomotor's
moving byte + fires the stopped-moving notify. It does not itself mark occupation bits
(though `FindSubCellDest` reaches `CellClass__PlaceInfantryInCell`). Slot-30 identity is
**UNCHECKED**; body is VERIFIED. See LABEL-DRIFT section.

## V14. COORD B's only writers, and the `owner+0x578` speed term identified

`decompile_function 0x0075c240` (`FindSubCellDest`, obj frame) writes COORD B
(`obj+0x28/0x2C/0x30`) at three points:
- early bail: if the coord produced by `owner->vt[0xF4]()` is NullCoord →
  `COORD B = NullCoord`, then falls to the tail which returns **0**.
- main path: `COORD B = CellClass__PlaceInfantryInCell(...)` result.
- crate/undeploy check: `if (!CrateClass__PickupDispatch(owner) && !owner[+0x81])`
  → `COORD B = NullCoord`; then `if (!owner[+0x90]) return 0;`
- tail `LAB_0075c5c5`: if COORD B is NullCoord → `owner->vt[0xF0](owner_coord)`, **return 0**;
  else → `owner->vt[0xF0](COORD B)`, **return 1**.

`decompile_function 0x0075aec0` (`ProcessMovement`, obj frame) writes COORD B **only to
NullCoord**, at these points: the tube-lookup failure path, the `iVar7 == 6` blocked path,
the "arrived at final destination" path, the generic blocked/other path, and the
`iVar7==5/4` blocked path. It never sets a non-null COORD B itself — it always delegates
that to `FindSubCellDest`. (One exception: the **tube** branch at `owner[+0x5E0] == 8`
writes COORD B from the tube entry cell: `*puVar1 = (short)tube[0x28] * 0x100 + 0x80` etc.,
i.e. a cell-center coord.)

`decompile_function 0x00520f40` (`FootClass__Locomotion_AI`) pins the speed field:
`if (*(double *)(param_1 + 0x15e) <= 0.8)` with `param_1` an `int*` → byte `owner+0x578`,
selecting between two sequences (0x17 vs 0x18) — a walk-vs-run speed fraction.
`ProcessMovement` reads the same field as `0.0 < *(double*)(owner + 0x15e)`.
So the slot-32 speed conjunct is **the owner FootClass's current speed fraction (double) at
owner+0x578, tested strictly `> 0.0`**.

`FootClass__Locomotion_AI` also pins the locomotor pointer on the owner:
`param_1[0x19d]` (= owner+0x674) is the ILocomotion iface pointer — it is the receiver of
`vt[0x10]` (= slot 4, Is_Moving) calls at 0x00520f6a-ish and again before `LAB_00521144`.

## V15. ANCHOR CORRECTION — ILocomotion vtables are 50 slots, not 40

`read_memory 0x007f69f8 length=240` and `read_memory 0x007f6ab8 length=24`:
- slots 40..49 = 0x0075ca80, 0x004b6640, **0x0075cb20**, **0x0075cbc0**, 0x0075cb30,
  0x004b6650, 0x004b6660, 0x004b6670, 0x004b6680, 0x004b6690
- the dword at `0x007f6ac0` (the slot-50 position) is **0x0080d2a8**, a data address — the
  MSVC RTTI complete-object-locator for the *next* vtable
- `0x007f6ac4` is the IUnknown vtable base, exactly matching the constructor's
  `MOV dword ptr [ESI],0x7f6ac4`

So the ILocomotion vtable occupies 0x007f69f8..0x007f6abc = **slots 0..49 (50 slots)**.
The session anchor's "40 slots" is **wrong**, but none of its load-bearing claims depend on
it — slots 4, 7 and 32 all sit below 40 and are confirmed. What the correction buys is
byte 0xA8 and 0xAC:

- **slot 42 (byte 0xA8) = Get_Is_Moving_Here = 0x0075cb20**
- **slot 43 (byte 0xAC) = Clear_Is_Moving_Here = 0x0075cbc0**
- slot 40 (byte 0xA0) = Is_At_Coord = 0x0075ca80

## V16. What the +0x32 byte actually drives: the infantry walk/run ANIMATION

`decompile_function 0x00520f40` (`FootClass__Locomotion_AI`) calls
`locomotor->vt[0xa8]()` = **slot 42 = Get_Is_Moving_Here** (iface+0x32), then:
```c
if (Get_Is_Moving_Here() == 0) {
    // standing/idle sequence, chosen by mission (owner+0x6C4)
    mission 3/0x17/0x18 -> owner->vt[0x558](0)
    mission 6           -> owner->vt[0x558](2)
    mission 0x11        -> owner->vt[0x558](0x10)
    else                   return
}
// moving-here: walk vs run
if (speed_fraction(owner+0x578) <= 0.8) owner->vt[0x558](0x17);  // walk
else                                    owner->vt[0x558](0x18);  // run
```
So **iface+0x32 is the animation predicate** ("am I currently stepping between sub-cells"),
distinct from both slot 4 and slot 32. It is written only inside ProcessMovement
(6 sites: `=1` at 0075bc2a and 0075bd25; `=0` at 0075b6a3, 0075bf60, 0075bf95; `=AL` at
0075afd5) and cleared by Stop_Moving and Clear_Is_Moving_Here.

## V17. The speed conjunct: sole runtime writer is TechnoClass__SetSpeedFraction, clamped

`search_instructions mnemonic=mov operand_pattern="+ 0x578],"` over the whole program
(1,152,218 instructions scanned, **not truncated**) gives 7 matches. Only three are runtime
writes to a FootClass instance, all inside `TechnoClass__SetSpeedFraction` at **0x004d3710**
(0x004d3721, 0x004d3749, 0x004d3768). The rest are `FootClass__Constructor` at 0x004d327f
(init) and unrelated objects (`RulesClass__Constructor`, `TechnoTypeClass__*`).
`search_instructions mnemonic=fst operand_pattern="0x578]"` gives **0 matches**;
`mnemonic=fstp` gives 1 match, in `RulesClass__ReadGeneral` (a *float* on a different
object).

`decompile_function 0x004d3710`:
```c
void __thiscall TechnoClass__SetSpeedFraction(this, double v) {
    if (1.0 <= v)  { this[0x578] = 1.0; return; }   // clamp high
    if (v  <= 0.0) { this[0x578] = 0.0; return; }   // clamp low
    this[0x578] = v;
}
```
Clamped to [0.0, 1.0]. **Walk reaches it through the owner virtual at byte 0x544.**
`search_instructions mnemonic=call operand_pattern="0x544]" function=...ProcessMovement`
gives **9 call sites**: 0075b2c5, 0075b80d, 0075bb6f, 0075bca6, 0075bcc9, 0075bd03,
0075be2f, 0075bf38, 0075bfb5.
`search_instructions mnemonic=push operand_pattern="0x3ff00000" function=...ProcessMovement`
gives **2 sites** (0075bc9d, 0075bfac), immediately preceding the calls at 0075bca6 and
0075bfb5. `0x3ff00000` is the high dword of the double 1.0.

So **for a walker the fraction is only ever pushed as 1.0 or 0.0**, and
**ProcessMovement writes it at 9 sites** — it is emphatically not "never written by the Walk
locomotor". Whether any *other* caller of SetSpeedFraction can give an infantry unit an
intermediate fraction is **UNCHECKED** (I did not enumerate its callers); the `<= 0.8` test
in Locomotion_AI suggests the field is designed for intermediate values, but I found no
Walk-side writer of one.

---

# ANSWERS TO THE LANE QUESTIONS

## Q1 — every writer of the +0x30 byte
Four, listed in V12: constructor (0x0075aa90), Head_To_Coord slot 17 (0x0075acb0, sets 1 /
conditionally clears), Stop_Moving slot 18 (0x0075ada0, conditional clear), and slot 30
(0x0075ae30, conditional clear). ProcessMovement and FindSubCellDest **never** write it
directly (mechanically verified, V12) — they clear it only by calling Stop_Moving through
`CALL [iface_vtbl+0x48]`.

`Get_Is_Moving_Here` / `Clear_Is_Moving_Here` operate on a **different byte, +0x32**, and
that byte is the infantry walk/run **animation** predicate (V16), not the locomotor's
moving flag and not the gate's predicate. The lane's hypothesis was right to be suspicious.
It is *not* a cell-occupancy flag either — it is a per-locomotor "mid sub-step" bool.

## Q2 — destination and head-to fields (offsets)
In the **object** frame / **iface** frame (iface = obj+4):

| field | obj | iface | set by | read by |
|---|---|---|---|---|
| COORD A = commanded destination | +0x1C/0x20/0x24 | +0x18/0x1C/0x20 | Head_To_Coord slot 17; nulled by Stop_Moving slot 18 | `Destination` slot 5 (gated on slot 4) |
| COORD B = head-to / sub-step coord | +0x28/0x2C/0x30 | +0x24/0x28/0x2C | `FindSubCellDest` 0x0075c240 (non-null); ProcessMovement (null only) + tube branch | `Head_To` slot 6, **`Is_Moving_Now` slot 32** |
| moving byte | +0x34 | +0x30 | see Q1 | `Is_Moving` slot 4 |
| in-Process flag | +0x35 | +0x31 | `Process` slot 16 only | — |
| moving-here / anim byte | +0x36 | +0x32 | ProcessMovement (6 sites), Stop_Moving, Clear_Is_Moving_Here | `Get_Is_Moving_Here` slot 42 |
| piggyback ptr | +0x38 | +0x34 | ctor=0; released in dtor | — |
| owner TechnoClass | +0x0C | +0x08 | base ctor | slot 32, ProcessMovement, everything |

NullCoord globals: `0x00b45be8 / 0x00b45bec / 0x00b45bf0`. The height constant used in the
arrival tests is `DAT_00b45c28`.

## Q3 — the schedule across one walk leg
`Process` = slot 16 at 0x0075ac80, which calls `ProcessMovement` at 0x0075aec0 (found by
following `CALL 0x0075aec0` in the Process body, not assumed). ProcessMovement branches on
**COORD B**:

**A. COORD B is null on entry** (leg start, or after a failed step)
- COORD A also null: SetSpeedFraction(0.0) and return; nothing else touched.
- COORD A non-null: path work.
  - the `Find_Path` **retry timer** (owner+0x640/0x644/0x648, and owner+0x178/0x191/0x192)
    can make it call `owner->vt[0x548]()` and **return with COORD B still null**;
  - `Find_Path` fails: either Stop_Moving (clears the moving byte) or `owner->vt[0x480]`;
  - `Find_Path` succeeds: reads path head `owner+0x5E0`; if the head is the **tube
    sentinel 8** it sets COORD B from the tube cell centre, else it move-checks the adjacent
    cell via `owner->vt[0x1ac]`:
    - check clear: **`FindSubCellDest` (called at 0x0075bc1a)** sets COORD B; on its
      **failure** (returns 0) it does SetSpeedFraction(0.0) and **returns with COORD B
      null**;
    - check blocked: `iface+0x32 = 0` and blockage handling; several of those paths null
      COORD B and call Stop_Moving, and the "temporarily blocked" path re-paths on a timer
      and **returns with COORD B null**.

**B. COORD B is non-null on entry** (mid sub-step)
- `iface+0x32 = 1`, then distance from owner coord to COORD B:
  - `>= 0x11`: keep walking — facing via `Set_Facing` (slot 19), position advanced,
    **COORD B untouched**. Gate stays TRUE.
  - `< 0x11`: arrived at the sub-step —
    1. shift the path array down one step (`owner+0x5E4` into `owner+0x5E0`, 23 dwords)
    2. `owner+0x558 = cell_of(COORD B)`
    3. **`FindSubCellDest` at 0x0075be18 — repopulates COORD B for the NEXT sub-step, and
       its return value is NOT checked** (`disassemble_bytes 0x0075be0a-0x0075be48`:
       `CALL 0x0075c240` is followed directly by `MOV ECX,[EBP+0xc]`, no `TEST AL,AL`)
    4. arrival-at-final test against COORD A; if not arrived, jump to `LAB_0075bf64` and
       return
    5. if arrived: **COORD B = NullCoord, THEN Stop_Moving, THEN `iface+0x32 = 0`**,
       byte-exact from `disassemble_bytes 0x0075bf00-0x0075bf70`:
       `MOV [EBX],EAX` / `[EBX+4],ECX` / `[EBX+8],EDX` (EBX = &COORD B), then
       `LEA EAX,[EBP+0x4]; PUSH EAX; MOV ECX,[EAX]; CALL [ECX+0x48]` (= slot 18
       Stop_Moving), then `MOV byte [EBP+0x36],0x0`.

**Does head-to blink null between sub-steps while still walking?**
**Not at a tick boundary on the happy path.** Steps 2 and 3 above happen inside one
ProcessMovement call, i.e. inside one `Process` call, i.e. within one tick — an external
observer polling slot 32 between ticks never sees the gap. **But it does go null across a
tick boundary in four real situations**, all with COORD A still set and the moving byte
still 1:
- **W1** order issued: Head_To_Coord sets the byte to 1 synchronously with the order, while
  COORD B is only populated later, inside `Process`.
- **W2** Find_Path retry-timer delay.
- **W3** next cell blocked (`owner->vt[0x1ac] != 0`), leading to a timed re-path.
- **W4** `FindSubCellDest` fails — e.g. no free infantry sub-cell in the next cell.

## Q4 — does the gate's predicate for a walker read a coord field?
**Yes.** Walk slot 32 (0x0075ab40) is a three-way conjunction and the third conjunct is
`COORD B != NullCoord` — the **head-to** coord, not the destination. It is *not*
"+0x30 byte plus a speed term". Full predicate (V6):
`Is_Moving(+0x30 byte) && (double)owner[+0x578] > 0.0 && COORD_B != NullCoord`.

## Q5 — over one continuous multi-cell walk order, is the gate constant-true? NO, IT OSCILLATES.

This is the single fact the port needs, and the answer is unambiguous even in the happy
path, because of *where* the gate is sampled.

`FootClass__AI` at 0x004da530 samples slot 32 at **four** sites
(`search_instructions mnemonic=call operand_pattern="0x80]" function="FootClass__AI"` gives
0x004da692, 0x004da8bb, 0x004da96d, 0x004daa24 — all `CALL dword ptr [ECX + 0x80]`, with
ECX loaded from `param_1[0x19d]` = owner+0x674 = the locomotor iface), and it calls
`Process` (slot 16, byte 0x40) at **0x004da87a**. So:
- **0x004da692 samples the gate BEFORE Process** — it sees last tick's end state.
- 0x004da8bb / 0x004da96d / 0x004daa24 sample it **AFTER Process**.

On the first tick of a walk order the pre-Process sample reads **false** (COORD B null) and
the post-Process samples read **true**. The gate therefore has two different values within
a single tick, and flips again at every W2/W3/W4 event above. Infantry squads walking
together hit W4 ("next cell's sub-cells are full") and W3 (blocked) constantly, so this is
ordinary-play behaviour, not an edge case.

**Corroborating design evidence that gamemd expects it to flicker:** every native consumer
of slot 32 is tolerant of a false negative, and one of them adds explicit hysteresis.
The four `FootClass__AI` consumers are:
- **0x004da692** — gates the per-tick shroud/sight re-reveal (`MapClass__UpdateFogBorder`),
  itself already timer-gated on owner+0x197/0x199 with a 15-frame reload.
- **0x004da8bb and 0x004da96d** — gate the increment of the movement counter
  `param_1[0x14e]` (owner+0x538).
- **0x004daa24** — combined with "did the counter change this tick", gates the looping
  movement **sound**, and when it fires it sets `param_1[0x150] = 3`, a **3-tick keepalive**
  that is decremented rather than cut immediately. That countdown exists precisely so a
  one- or two-tick dropout in "is it moving" does not chop the sound.

**Critically: none of the four is a mission gate.** Fog reveal, a counter, and a sound.
The only other verified locomotor-receiver consumers I could attribute are
`FUN_00521b60` at 0x00521b60 (`decompile_function`: reads `param_1[0x19d]->vt[0x80]`, and a
true result can only ever make it return 0, never 1 — a suppressor, checked against the
per-mission table at `&DAT_007eaf7c`), `FootClass__IsCloakable` at 0x004dbddd, and the
piggyback delegations inside `DriveLocomotionClass__Process` at 0x004b078f,
`FlyLocomotionClass__Process` at 0x004cd644, `ShipLocomotionClass__Process` at 0x0069fe3c,
and `HoverLocomotionClass__Move` at 0x00514a24.

**So the premise in the lane brief — "answering moving when native says not moving makes the
game defer that unit's mission and can stall it permanently" — does not hold for gamemd's
own use of slot 32.** gamemd never gates a mission on Is_Moving_Now. If the Rust port has
wired slot-32 semantics into a mission-readiness gate, that gate is VERA-invented, and the
oscillation documented above is exactly the kind of thing it will trip over.

### Direct consequence for `src/sim/movement/ready_producer.rs`

Read at src/sim/movement/ready_producer.rs
(lines 194-221). Two of its three assertions are **confirmed correct**, one is wrong:

1. `Native predicate: moving_byte != 0 && applied_speed > 0 && head_to_nonnull` —
   **CORRECT**, and it was read off **slot 32**, not slot 4. Verified V6.
2. "the third input reads the locomotor's **head-to** coord ... not its final destination" —
   **CORRECT**. COORD B is the sub-step head-to; COORD A is the destination. Verified V7.
   Mapping it to `path[next_index]` is the right structural analogue.
3. "Native's speed fraction for a walker is only ever 1.0 (move start) or 0.0 (arrival /
   construction); **the Walk locomotor never writes it**" — the value range is **CORRECT**
   (only 1.0 and 0.0 are ever pushed, V17), but "never writes it" is **WRONG**:
   ProcessMovement calls the owner's SetSpeedFraction at **9 sites**, 2 with 1.0 and 7 with
   0.0. This matters because the Rust code ties `applied_speed_bits` to `live_move`, whereas
   native sets it at specific ProcessMovement exits — so the two can disagree on *timing*
   even when they agree on range. Not necessarily a behavioural drift; recorded as one to
   check.

The doc comment also says the `Blocked`-phase exclusion is VERA-internal with the gamemd
equivalent UNCHECKED, added because "without it a permanently blocked walker would report
moving forever and defer its mission forever". **That exclusion is now CHECKED and it is
directionally right:** native also reports **not moving** for a blocked walker, because a
blocked step leaves COORD B null (W3/W4) and slot 32's third conjunct then fails. Native
reaches the same answer by a different mechanism (COORD B lifetime rather than a phase
enum). It can be relabelled from "VERA-internal, gamemd UNCHECKED" to "matches native
outcome via COORD B nullity — verified this session", though the Rust route is still a
structural approximation, not the native mechanism.

---

# INFERRED (not proven this session)

- `obj+0x0C` / `iface+0x08` is the owner FootClass pointer. Strongly supported: the base
  ctor zeroes it, slot 32 dereferences it at +0x578, ProcessMovement dereferences it for
  every owner virtual call, and `FootClass__Locomotion_AI` reads the mirror pointer at
  owner+0x674. Not proven by a write-site trace of the link call (slot 3).
- `obj+0x38` / `iface+0x34` is the piggybacked-locomotor pointer. Inferred from
  `WalkLocomotionClass__Destructor` (`decompile_function 0x0075ab00`) calling
  `(**(code**)(*piVar1 + 8))(piVar1)` — an IUnknown::Release — on `param_1[0xe]` = obj+0x38.
- `owner+0x5E0` is the head of the path-step array (23+ dwords, shifted down by one on each
  sub-step arrival), and value 8 at the head is the tube sentinel. Consistent with the
  existing project memory note "tube sentinel-8". Verified as *behaviour* in
  ProcessMovement; the array's full layout is UNCHECKED.
- `owner+0x578` is a walk-vs-run speed **fraction** rather than a speed magnitude. Supported
  by the [0.0,1.0] clamp in SetSpeedFraction and the `<= 0.8` test in Locomotion_AI.
- `owner->vt[0x544]` is SetSpeedFraction. Inferred from the pushed argument pair
  `(0, 0x3ff00000)` = the double 1.0 and `(0,0)` = 0.0 matching SetSpeedFraction's
  signature, plus +0x578 being written nowhere else. I did not read the FootClass vtable
  slot bytes to confirm 0x544 resolves to 0x004d3710.
- `owner->vt[0x54c]` is the "stopped moving" notification (called by Stop_Moving, slot 30
  and Head_To_Coord only on a 1-to-0 transition of the moving byte). Role inferred from
  call context only.

# UNCHECKED / UNKNOWN

- **Where Head_To_Coord (slot 17) is called from, and in which tick phase relative to
  `FootClass__AI`.** This is the one remaining gap in the W1 story. I established that the
  moving byte is set synchronously inside Head_To_Coord and that COORD B is only populated
  inside Process, so *some* sample sees byte=1 with COORD B null; I did NOT verify which
  phase the order-issuing caller runs in. The within-tick oscillation conclusion for Q5 does
  not depend on this (the pre-Process sample at 0x004da692 establishes it independently).
- The three owner predicates that can make Head_To_Coord a complete no-op: owner vtable
  bytes **0x37C, 0x1D4, 0x1D8**. Identities unknown. 0x37C also appears in ProcessMovement's
  keep-walking branch as an abort.
- Slot 30's real identity (the label says `Mark_All_Occupation_Bits`; the body does not
  match — see LABEL DRIFT). Its callers are unenumerated, so I cannot say how often the
  fourth moving-byte writer fires.
- `owner->vt[0x1ac]` returns a move-check enum; I observed the values ProcessMovement
  branches on (0 = clear, and 1,2,3,4,5,6,7 handled distinctly) but did not decode the enum.
- Whether any non-Walk caller of `TechnoClass__SetSpeedFraction` can set an intermediate
  fraction on an *infantry* unit. Callers unenumerated.
- The receiver identity of most of the 181 program-wide `CALL [reg+0x80]` sites. I confirmed
  the locomotor receiver only for `FootClass__AI` (x4) and `FUN_00521b60`. I positively
  **excluded** `InfantryClass__PerCellProcess` at 0x00519998: its receiver reads
  `byte [ECX+0x14] & 1` immediately before the call
  (`disassemble_bytes 0x00519980-0x005199b8`), and iface+0x14 on a Walk locomotor is the
  IPiggyback vtable pointer, so that ECX is not a locomotor and its `+0x80` is a different
  vtable's slot 32.
- `owner+0x538` (the movement counter, `param_1[0x14e]`) — I did not trace its other
  consumers, so I cannot rule out an indirect mission dependency through it.

# LABEL-DRIFT-FOUND

1. **`WalkLocomotionClass__Mark_All_Occupation_Bits` @ 0x0075ae30 (slot 30) — misleading.**
   Verified body (`disassemble_function 0x0075ae30`): saves the moving byte, calls
   `FindSubCellDest`, and if BOTH COORD A and COORD B are null, clears the moving byte and
   fires the stopped-moving notify. It marks no occupation bits itself. Real identity
   UNCHECKED. This one matters because the label hides a **writer of the moving byte** —
   anyone enumerating writers from labels alone would miss it.
2. **Anchor correction, not a Ghidra label:** the session anchor "ILocomotion vtables are 40
   slots" is wrong; they are **50** (V15). Consequence: bytes 0xA0/0xA8/0xAC are valid slots
   (40/42/43) and `FootClass__Locomotion_AI`'s `vt[0xa8]` call is Get_Is_Moving_Here, which
   would otherwise look like an out-of-range vtable read.
3. **Confirmed NOT drift:** the label `WalkLocomotionClass__Is_Moving_Now @ 0x0075ab40`
   agrees with the vtable-derived slot 32 (V1). Same for `Is_Moving @ 0x0075ab30` (slot 4),
   `Destination @ 0x0075aba0` (slot 5), `Head_To @ 0x0075ac00` (slot 6),
   `Process @ 0x0075ac80` (slot 16), `Head_To_Coord @ 0x0075acb0` (slot 17),
   `Stop_Moving @ 0x0075ada0` (slot 18), `Get_Is_Moving_Here @ 0x0075cb20` (slot 42),
   `Clear_Is_Moving_Here @ 0x0075cbc0` (slot 43). Unlike Hover, the Walk labels are sound.
4. The existing plate comment on `WalkLocomotionClass__Constructor` claims a
   "7-state machine: Idle/Landed/Accelerating/Cruising/PathFollow/CellEntry/Stopping".
   I found **no state enum** in the Walk struct — the constructor initialises two coord
   triples, three bytes and one dword, and ProcessMovement branches on COORD B nullity plus
   the path head, not on a state field. Treat that plate comment as **unverified prose**.
