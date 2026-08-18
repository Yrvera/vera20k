> **RAW LANE OUTPUT — NOT THE AUTHORITY.** This is the unedited lane 4 (Fly and Rocket fields) from the
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

# LANE 4 — Fly and Rocket moving-predicate fields

Session date 2026-07-29. Program: gamemd.exe (testProsjekt), image base 0x400000.
All addresses below carry an inline citation of the tool call that produced them.

---

## VERIFIED

### V0. THE FRAME SHIFT — every ILocomotion slot body is in `object_base + 4`

This is the single most load-bearing finding in this lane, and it changes how every
previously-recorded "+0xNN" locomotor field offset must be read.

`RocketLocomotionClass__Constructor` (verified via `disassemble_function 0x00661ec0`):

```
00661ec1: MOV ESI,ECX                       ; ESI = object base
00661ec3: CALL 0x0055a6c0                   ; LocomotionClass base ctor
00661ec8: MOV ECX,dword ptr [0x00b04e38]
00661ece: LEA EAX,[ESI + 0x18]
00661ed1: MOV dword ptr [ESI + 0x18],ECX    ; object+0x18 <- global 0x00b04e38
00661ed4: MOV EDX,dword ptr [0x00b04e3c]
00661eda: MOV dword ptr [EAX + 0x4],EDX     ; object+0x1c <- global 0x00b04e3c
00661edd: MOV ECX,dword ptr [0x00b04e40]
00661ee3: MOV dword ptr [EAX + 0x8],ECX     ; object+0x20 <- global 0x00b04e40
...
00661f19: MOV dword ptr [ESI],0x7f0be8      ; object+0x00 = IUnknown-side vtable
00661f1f: MOV dword ptr [ESI + 0x4],0x7f0b1c ; object+0x04 = ILocomotion vtable
```

Rocket ILocomotion **slot 4** body at 0x00661f50 (hand-decoded from
`read_memory 0x00661f50 length 112`) reads those very same three fields at
`[this+0x14] / [this+0x18] / [this+0x1c]` and compares them against the same three
globals 0x00b04e38 / 0x00b04e3c / 0x00b04e40.

object+0x18 == interface+0x14  =>  **interface `this` = object base + 4.**

That is the expected layout: the ILocomotion vtable pointer lives at object+0x04, so a
call made through an `ILocomotion*` passes object_base+4. Therefore **any offset read
inside a slot body is 4 lower than the same field's offset in the constructor's frame.**

Corollary that resolves the Lane-4 question directly: Fly slot 32 reads
`double [this+0x44]`, i.e. **object+0x48**, not object+0x44.

### V1. Rocket ILocomotion vtable = 0x007f0b1c

From `MOV dword ptr [ESI + 0x4],0x7f0b1c` at 0x00661f1f
(verified via `disassemble_function 0x00661ec0`).
IUnknown-side vtable = 0x007f0be8 (same listing).

Slots read via `read_memory 0x007f0b1c length 132` (33 dwords):

| slot | target | note |
|---|---|---|
| 4  | 0x00661f50 | Is_Moving (decoded below) |
| 5  | 0x00661fb0 | — |
| 7  | 0x0055abf0 | **matches the base-class anchor => vtable base alignment CONFIRMED** |
| 9  | 0x00663470 | (Move_To position by analogy with Fly; UNCHECKED) |
| 16 | 0x006622c0 | |
| 17 | 0x006632e0 | |
| 18 | 0x006633c0 | |
| 29 | 0x00663460 | |
| 32 | 0x00661f90 | **Is_Moving_Now** (decoded below) |

### V2. Rocket slot 4 (Is_Moving) @ 0x00661f50 — destination-vs-null-sentinel test

Hand-decoded from `read_memory 0x00661f50 length 112`:

```
00661f50: 8b 44 24 04        MOV EAX,[ESP+4]        ; this (= object+4)
00661f54: 8b 15 384eb000     MOV EDX,[0x00b04e38]
00661f5a: 8b 48 14           MOV ECX,[EAX+0x14]
00661f5d: 3b ca              CMP ECX,EDX
00661f5f: 75 1f              JNZ 0x00661f80
00661f61: 8b 50 18           MOV EDX,[EAX+0x18]
00661f64: 8b 0d 3c4eb000     MOV ECX,[0x00b04e3c]
00661f6a: 3b d1              CMP EDX,ECX
00661f6c: 75 12              JNZ 0x00661f80
00661f6e: 8b 40 1c           MOV EAX,[EAX+0x1c]
00661f71: 8b 0d 404eb000     MOV ECX,[0x00b04e40]
00661f77: 3b c1              CMP EAX,ECX
00661f79: 75 05              JNZ 0x00661f80
00661f7b: 33 c0              XOR EAX,EAX            ; all three equal -> 0
00661f7d: c2 04 00           RET 4
00661f80: b8 01000000        MOV EAX,1              ; any differ -> 1
00661f85: c2 04 00           RET 4
```

`Rocket::Is_Moving() == (stored destination coord != the null-coord sentinel triple)`.
The sentinel triple is exactly what the constructor stored, so a freshly constructed
Rocket locomotor reports NOT moving. Structural shape identical to a "has destination"
flag, expressed as a coordinate comparison rather than a bool.

### V3. Rocket slot 32 (Is_Moving_Now) @ 0x00661f90 — a PHASE-RANGE test, not a speed test

Hand-decoded from the same `read_memory 0x00661f50 length 112` window
(0x00661f90 = 0x00661f50 + 64; padding NOPs at 0x00661f88..0x00661f8f):

```
00661f90: 8b 44 24 04        MOV EAX,[ESP+4]        ; this (= object+4)
00661f94: 8b 40 3c           MOV EAX,[EAX+0x3c]     ; => object+0x40, a 32-bit int
00661f97: 83 f8 03           CMP EAX,3
00661f9a: 7c 0a              JL  0x00661fa6
00661f9c: 83 f8 05           CMP EAX,5
00661f9f: 7f 05              JG  0x00661fa6
00661fa1: b0 01              MOV AL,1
00661fa3: c2 04 00           RET 4
00661fa6: 32 c0              XOR AL,AL
00661fa8: c2 04 00           RET 4
```

So **`Rocket::Is_Moving_Now() == (3 <= (int)object[+0x40] <= 5)`** — an inclusive range
test on a small integer, i.e. a trajectory-phase enum. The constructor zeroes object+0x40
(`MOV dword ptr [ESI + 0x40],EAX` with EAX=0 at 0x00661f06, `disassemble_function
0x00661ec0`), so phase 0 = not moving now. Phases 1 and 2 are also "not moving now",
and anything >5 is "not moving now" as well.

Note how different this is from Fly's slot 32 (a float compare) and from Drive's.
**Slot 32 has no common shape across families.** Any Rust code that assumes one
predicate template for all six families is structurally wrong.

### V4. Fly ILocomotion vtable = 0x007e89f4, slot inventory

`read_memory 0x007e89f4 length 160` (40 slots):

slot0 0x004d0510, slot1 0x004d0520, slot2 0x004d0530, slot3 0x004cca20,
**slot4 0x004cca90**, slot5 0x004ccae0, slot6 0x0055aca0, **slot7 0x0055abf0 (anchor OK)**,
slot8 0x0055abe0, slot9 0x004cf610, slot10 0x004cfb00, slot11 0x004cf830,
slot12 0x004cf940, slot13 0x0055abc0, slot14 0x0055aba0, slot15 0x0055abb0,
slot16 0x004ccb40, slot17 0x004ccc80, slot18 0x004ccfd0, slot19 0x004cfc10,
slot20 0x0055ac20, slot21 0x0055ab90, slot22 0x0055a8f0, slot23 0x004cfd20,
slot24 0x004cfd90, slot25 0x004cfda0, slot26 0x0055ab70, slot27 0x0055ab80,
slot28 0x0055ac10, slot29 0x004cfcf0, slot30 0x0055ac00, slot31 0x0055ace0,
**slot32 0x004ccac0**, slot33 0x004cfe20, slot34 0x0055acf0, slot35 0x0055ad00,
slot36 0x004cfe50, slot37 0x004cfe80, slot38 0x004b4c80, slot39 0x004b6620.

Confirms the anchors' Fly slot4/slot7/slot32. Also confirms the labels
`Move_To 0x004cf610` = slot 9 and `Stop_Moving 0x004cf830` = slot 11.

**0x004cd600 (`FlyLocomotionClass__Process`) is NOT in the ILocomotion vtable** — it is an
internal helper. `get_function_callers 0x004cd600` returns exactly one caller:
`FlyLocomotionClass__Layer @ 0x004ccb40`, which IS slot 16. So Process runs on whatever
frame slot 16 hands it (see UNCHECKED section).

### V5. Fly object+0x40 and object+0x48 are two separate doubles

`FlyLocomotionClass__Constructor` (`decompile_function 0x004cc9a0`) zeroes, among others,
`param_1[0x10]`=+0x40, `param_1[0x11]`=+0x44, `param_1[0x12]`=+0x48, `param_1[0x13]`=+0x4c
— i.e. two 8-byte-aligned doubles at object+0x40 and object+0x48, both starting at 0.0.

`decompile_function 0x004cefb0` (`FlyLocomotionClass__Horizontal_Step`) uses both in the
*object* frame within one basic block, which is what proves they are distinct fields:

```
if (iVar4 < 0x80) {                                  // horiz distance < 0.5 cell
    if (cStack_4 != '\0') {
      *(undefined4 *)(iVar8 + 0x40) = 0;
      *(undefined4 *)(iVar8 + 0x44) = 0;             // object+0x40 <- 0.0
    }
    if ((*(char *)(iVar8 + 0x5c) == '\0') && (*(double *)(iVar8 + 0x48) < 0.05)) {
      FlyLocomotionClass__Begin_Landing();
    }
}
else if (iVar4 < 0x200) {                            // < 2 cells
    if (cStack_4 != '\0') {
      *(undefined4 *)(iVar8 + 0x40) = 0;
      *(undefined4 *)(iVar8 + 0x44) = 0x3fe00000;    // object+0x40 <- 0.5
    }
}
else if ((iVar4 < 0x300) && (cStack_4 != '\0')) {
      *(undefined4 *)(iVar8 + 0x40) = 0;
      *(undefined4 *)(iVar8 + 0x44) = 0x3fe80000;    // object+0x40 <- 0.75
}
```

Two independent reasons the writes are the *high half of the double at +0x40* and not a
field at +0x44:
1. The written constants are exactly the high dwords of IEEE-754 doubles with zero low
   half — 0x3FE00000 = 0.5, 0x3FE80000 = 0.75 — and each is paired with a `= 0` store at
   +0x40 (the low half).
2. The same frame reads `*(double *)(iVar8 + 0x48)`, so +0x48 is where the *other* double
   starts; a double at +0x44 would overlap it and would be only 4-byte aligned.

Same function, `FlyLocomotionClass__Process` writes (`search_instructions` mnemonic-free,
`operand_pattern "+0x44]"`, `function FlyLocomotionClass__Process`) — all four object-frame
stores are the same high-half idiom:

```
004ce1ec: MOV dword ptr [ESI + 0x44], 0x3ff00000   ; object+0x40 <- 1.0
004ce222: MOV dword ptr [ESI + 0x44], 0x3ff00000   ; object+0x40 <- 1.0
004ce231: MOV dword ptr [ESI + 0x44], EBP
004ce27f: MOV dword ptr [ESI + 0x44], 0x3fb99999   ; object+0x40 <- ~0.1 (high half)
004ce294: MOV dword ptr [ESI + 0x44], EBP
```

0x3FF00000 = 1.0, 0x3FB99999 = high half of 0.1.

**Reading of the two fields (object frame):**
- object+0x40 = a *commanded / target* speed fraction, set to discrete literals
  1.0 / 0.75 / 0.5 / 0.1 / 0.0 as a function of remaining horizontal distance
  (0x300 = 3 cells, 0x200 = 2 cells, 0x80 = 0.5 cell in leptons at 256/cell).
- object+0x48 = the *current* speed fraction, which is what Fly slot 32 tests, and which
  Horizontal_Step separately compares against 0.05 to decide to start landing.

**Therefore Fly slot 32 == "current speed fraction != 0.0", reading object+0x48, and the
field written by Horizontal_Step/Process at object+0x40 is the TARGET, a different
field.** A doc comment that says Fly's predicate reads the field the step functions write
is off by one field.

### V5b. Third and fourth independent confirmations of the +4 frame shift

1. `FlyLocomotionClass__Process` (`decompile_function 0x004cd600`) calls its own slot 32
   **through the interface pointer it constructs by hand**:
   ```
   uStack_78 = (int *)(param_1 + 4);
   cVar4 = (**(code **)(*(int *)(param_1 + 4) + 0x80))();     // at 0x004cd64a
   ```
   `*(param_1+4)` is the ILocomotion vtable and `+0x80` is slot 32. It also does
   `piVar6 = (int *)(param_1 + 4); ... (**(code **)(*piVar6 + 0x10))()` = slot 4.
   So **Process's `param_1` is the object base** and the interface is `param_1+4`.
2. `FlyLocomotionClass__Is_Moving` (slot 4, `decompile_function 0x004cca90`) reads the
   owner techno at `*(int *)(param_1 + 8)`, while `Horizontal_Step`, `Begin_Takeoff`,
   `Begin_Landing` and `Process` all read it at `param_1 + 0xc`. Same field, two frames,
   4 apart.

**Consequence for the whole locomotor audit: an offset harvested from a slot-32 or slot-4
body is interface-relative. To compare it against anything harvested from a constructor,
a `*_Step` function, or `Process`, add 4.** Any earlier note that recorded a Fly field as
"+0x44" without naming its frame is ambiguous and, read as an object offset, wrong.

### V6. Fly slot 32 @ 0x004ccac0 — exact body, hand-decoded

`read_memory 0x004ccac0 length 32`:

```
004ccac0: 8b 44 24 04        MOV EAX,[ESP+4]              ; this = object+4
004ccac4: dd 40 44           FLD  qword ptr [EAX+0x44]    ; => object+0x48
004ccac7: dc 1d 00287e00     FCOMP qword ptr [0x007e2800]
004ccacd: df e0              FNSTSW AX
004ccacf: f6 c4 40           TEST AH,0x40                 ; C3 = "equal"
004ccad2: 74 05              JZ  0x004ccad9               ; not equal -> moving
004ccad4: 32 c0              XOR AL,AL                    ; equal to 0.0 -> 0
004ccad6: c2 04 00           RET 4
004ccad9: b0 01              MOV AL,1
004ccadb: c2 04 00           RET 4
```

Matches the anchor. `Fly::Is_Moving_Now() == (double object[+0x48] != 0.0)`. Note it is an
*inequality against zero*, not `> 0` — a negative value would also report moving (not
reachable in practice, see V8 ramp clamps).

### V7. Fly slot 4 @ 0x004cca90 — a DIFFERENT predicate, on DIFFERENT fields

`decompile_function 0x004cca90`:

```c
undefined4 FlyLocomotionClass__Is_Moving(int param_1)   // param_1 = object+4
{
  if ((*(char *)(param_1 + 0x30) == '\0') &&
      (*(float *)(*(int *)(param_1 + 8) + 0x2e8) <= 0.0)) {
    return 0;
  }
  return 1;
}
```

Translated to the object frame:
`Fly::Is_Moving() == (byte object[+0x34] != 0) || ((float owner_techno[+0x2e8]) > 0.0)`
where `object[+0x0C]` is the owner techno pointer.

So slot 4 = "has a move destination flag set, OR the owner still has residual pitch/bank
to unwind"; slot 32 = "current speed fraction is non-zero". **They share no field.** Any
Rust doc comment describing Fly's readiness predicate in terms of a destination flag has
described slot 4, not the gate's slot 32.

`Process` writes that very techno float: `*(float *)(*(int *)(param_1 + 0xc) + 0x2e8) =
(float)fVar20;` inside the deceleration block (`decompile_function 0x004cd600`), derived
from `typeclass[+0x3b0]`. Slot 4 is also consumed *inside* Process as an early-out:
`cVar4 = (**(code **)(*piVar6 + 0x10))(); if (cVar4 == '\0') return;` — i.e. **slot 4 gates
whether Fly does any per-tick work at all, and slot 32 is the movement state produced by
that work.** That is the structural reason they must not be interchanged.

### V8. Every writer of Fly object+0x48 (the field slot 32 reads)

All of them are in `FlyLocomotionClass__Process`; verified via `decompile_function
0x004cd600` and cross-checked against `search_instructions operand_pattern "0x48]"
function FlyLocomotionClass__Process` (16 matches; the `[ESI + 0x48]` ones at 0x004ce288,
0x004ce297, 0x004ce29a, 0x004ce2a4, 0x004ce2bb, 0x004ce2d1, 0x004ce2fd, 0x004ce449,
0x004ce44f, 0x004ce479, 0x004ce495 are the field, the `[ESP + 0x48]` ones are locals).

Naming object+0x40 = `target` and object+0x48 = `current`, `dist` = the ftol'd horizontal
distance in leptons from the locomotor's stored destination to the owner's coord:

1. **Target selection** (guarded by: owner in-map, `!IsDescending(+0x51)`, and either
   `!IsAscending(+0x50)` or altitude >= FlightLevel/2, and destination != null-coord):
   ```c
   target = min(1.0, (double)dist / (double)typeclass[+0x2f8]);
   if (target < 0.1) {
       if (dist < 0x56) { target = 0.0;  current = current * 0.5; }   // <-- writer A
       else             { target = 0.1; }
   }
   if ((double)dist < current) { current = (double)dist; }             // <-- writer B
   if (target == 0.0 && current == 0.0 && dist > 0) {
       current = 0.05;                                                // <-- writer C
   }
   ```
   (0x9999999a/0x3fb99999 = 0.1; 0x9999999a/0x3fa99999 = 0.05.)
2. **The ramp**, at the very end of Process, unconditional except `owner in-map`:
   ```c
   if (target <= current) {
       if (current <= target) { /* equal: no write */ }
       else { current = max(current - 0.1, target); }                 // <-- writer D
   } else {
       current = min(current + 0.1, target);                          // <-- writer D
   }
   ```
3. **Constructor** zeroes it (`decompile_function 0x004cc9a0`, `param_1[0x12] = 0;
   param_1[0x13] = 0;`) — a fresh Fly locomotor reports **not moving now**.

**So what makes the predicate flip to "not moving": `current` reaching exactly 0.0.**
That happens by exactly two routes, and both are gated:

- **Writer D (ramp) driving `current` down to a `target` of 0.0**, which takes up to 10
  ticks from full speed. `target` is set to 0.0 by:
  - writer A's branch — the computed fraction < 0.1 **and** `dist < 0x56` leptons
    (~1/3 cell);
  - the `typeclass[+0xd27]` branch when `IsAscending == 0` and `owner[+0x2b4] == 0`,
    else that branch sets 1.0;
  - `Horizontal_Step` when `dist < 0x80` leptons (`decompile_function 0x004cefb0`).
- **Writer B**, the clamp to remaining distance, which can only zero `current` when
  `dist == 0` exactly (for `dist >= 1` the clamp is a no-op against a fraction < 1.0).

And **writer C actively prevents the flip**: whenever `target` and `current` have both
reached 0.0 but `dist > 0` still, `current` is forced back to 0.05. Native deliberately
keeps a Fly locomotor reporting "moving now" while any distance remains. A Rust mapping
that lets Fly report "not moving" while a destination is still outstanding is the wrong
direction relative to native, not merely conservative.

Neither `Begin_Takeoff` (0x004cf950, `decompile_function`) nor `Begin_Landing`
(0x004cfa70, `decompile_function`) touches object+0x40 or +0x48; they write only
IsAscending(+0x50), IsDescending(+0x51), +0x52 and FlightLevel(+0x38). **Takeoff and
landing do not zero the speed** — the ramp does, indirectly, via `target`.

**Answer to "speed / per-axis step / altitude rate / something else":** object+0x48 is a
**dimensionless speed *fraction*** (0.0 .. 1.0, quantised in practice to multiples of 0.1
plus the 0.05 floor), not leptons and not an altitude rate. Evidence: it is produced as
`dist / typeclass[+0x2f8]` clamped to 1.0; it is ramped by ±0.1/tick; and the draw-matrix
function compares it against a *typeclass double* to decide whether to apply a pitch
rotation. Altitude is a separate integer path in the same function (FlightLevel at
object+0x38, per-tick step 6/0x10/0x14..0x32 leptons).

### V9. Rocket's phase field (object+0x40) is written by slot 16 @ 0x006622c0

`decompile_function 0x006622c0` — frame is the interface (it calls
`(**(code **)(*param_1 + 0x80))(param_1)` = slot 32 and
`(**(code **)(*param_1 + 0x10))(param_1)` = slot 4 on `param_1` itself), so
`param_1[0xf]` = interface+0x3c = **object+0x40 = exactly the field slot 32 range-tests**.
The function is a `switch(param_1[0xf])` with cases 1..6:

| phase | role (from the case body) | Is_Moving_Now |
|---|---|---|
| 0 | constructor initial value | **false** |
| 1 | pre-launch hold on a timer; zeroes the speed double (`param_1[0x11]`, `param_1[0x12]`); exits to phase 2 or phase 6 | **false** |
| 2 | launch tilt ramp over a timer; on completion plays the launch anim + `AuxSound1` and sets phase 3 | **false** |
| 3 | boost: `speed += typeclass-ish[+0x18]` clamped to `typeclass[+0x678]`; when altitude >= `[+0x1c]` sets phase 4 and latches the ground range | **true** |
| 4 | cruise: keeps accelerating, tracks distance, sets phase 5 when close | **true** |
| 5 | terminal: turns the pitch toward the target at a per-tick rate | **true** |
| 6 | alternate (sub-launched) launch sequence; ends by setting phase 3 | **false** |

That is *semantically* exactly the `3 <= phase <= 5` test decoded in V3, arrived at
independently. `Is_Moving_Now` for a Rocket therefore means **"in powered flight"**, and it
is false during the launch wind-up and false forever after construction until launch.

Also from the same body: the slot-32 result is used locally to gate a 3-frame-cadence
trail animation, and slot 4's result is what slot 16 returns to its caller.

### V10. Reachability — Fly

Verified from `ini/rulesmd.ini` (in-repo, the authority for stock behaviour). Fly CLSID is
`{4A582746-9839-11d1-B709-00A024DDAFD1}`; `Grep` found 13 occurrences, of which **8 are
live and 5 are commented out**.

Live (`Locomotor=` uncommented):

| line | section | unit |
|---|---|---|
| 10631 | `[ORCA]` (line 10582) | Intruder / Harrier — `Name=Intruder`, `Image=FALC` |
| 10683 | `[ASW]` (line 10646) | Osprey — Destroyer's helicopter, `Spawned=yes` |
| 10730 | `[HORNET]` (line 10694) | Hornet — Aircraft Carrier fighter, `Spawned=yes` |
| 10789 | `[BEAG]` (line 10747) | Black Eagle — Korea |
| 11302 | `[BPLN]` (line 11276) | Soviet MIG, `Spawned=yes`, `Selectable=no` |
| 11348 | `[SPYP]` (line 11323) | Soviet Spy Plane, `Spawned=yes`, `Selectable=no` |
| 11560 | `[PDPLANE]` (line 11536) | Paratrooper Cargo Plane, `Spawned=yes`, `Selectable=no` |
| 11594 | `[CARGOPLANE]` (line 11572) | Transport Plane |

Commented out in stock (these therefore fall back to the *default* locomotor, i.e. they are
NOT Fly): lines 10552, 10851, 10912, 11180 (each written `;Locomotor={4A58...} ;flying`)
and 11528 (a fully commented-out unit block). The 10552/10851/10912/11180 sections are the
helicopter-style aircraft — worth a separate lane to confirm which CLSID they land on,
because "aircraft ⇒ Fly" is false in stock YR.

**Player-visible frequency: high.** Harrier and Black Eagle are ordinary tech-3 airfield
units; Osprey and Hornet ship automatically with the Destroyer and Aircraft Carrier;
Spy Plane and Paradrop planes are stock support powers. An ordinary skirmish sees Fly
locomotion within the first ten minutes in most matches.

**Do they run the mission readiness gate? YES.**
- `get_function_callers FootClass__AI` returns exactly
  `AircraftClass__AI @ 0x00414bb0`, `InfantryClass__AI @ 0x0051bab0`,
  `UnitClass__AI @ 0x007360c0`. **Aircraft go through `FootClass::AI`.**
- `FootClass__AI`'s slot-32 callsite at 0x004da692, hand-decoded from
  `read_memory 0x004da680 length 40`:
  ```
  004da689: 8b 86 74060000     MOV EAX,[ESI+0x674]      ; the ILocomotion interface ptr
  004da68f: 50                 PUSH EAX
  004da690: 8b 08              MOV ECX,[EAX]            ; its vtable
  004da692: ff 91 80000000     CALL dword ptr [ECX+0x80] ; slot 32 = Is_Moving_Now
  004da698: 84 c0              TEST AL,AL
  004da69a: 0f 84 10010000     JZ  +0x110
  ```
  So the readiness consumer reads slot 32 off `techno[+0x674]`, and the "not moving now"
  answer skips a 0x110-byte block.
- `AircraftClass` reads slot 32 directly as well:
  `AircraftClass__Is_Weapon_Ready` (`decompile_function 0x0041b9c5`) ends with
  `cVar1 = (**(code **)(**(int **)(param_1 + 0x674) + 0x80))(*(int **)(param_1 + 0x674));
  return (uint)(cVar1 == '\0');` — **weapon-ready is literally `!Is_Moving_Now`** for the
  two types matching `RulesClass[+0x4e0]` and `RulesClass[+0x514]`.
  `search_instructions mnemonic CALL operand "+ 0x80]"` also lands slot-32-shaped calls in
  `AircraftClass__Is_Firing_Possible` @ 0x0041b965 and `AircraftClass__Mission_Attack`
  @ 0x0041816b (receivers for those two UNCHECKED).

### V11. Reachability — Rocket

Rocket CLSID `{B7B49766-E576-11d3-9BD9-00104B972FE8}`, `Grep` on `ini/rulesmd.ini` — three
live users, no commented ones:

| line | section | unit |
|---|---|---|
| 11389 | `[V3ROCKET]` (line 11364) | V3 Rocket — `Spawned=yes`, `MissileSpawn=yes`, `Selectable=no`, `DontScore=yes` |
| 11429 | `[DMISL]` (line 11404) | Dreadnought Missile — same flags, plus `FlyBack=true` |
| 11472 | `[CMISL]` (line 11445) | Cruise Missile (Boomer sub) — `Owner=YuriCountry`, same flags |

**They are AircraftTypes, not projectiles.** Verified from the `[AircraftTypes]` roster in
the same file: `1163:4=V3ROCKET`, `1165:6=DMISL`, `1171:12=CMISL` (alongside `1161:2=ORCA`,
`1162:3=HORNET`, `1164:5=ASW`, `1166:7=PDPLANE`, `1167:8=BEAG`, `1169:10=BPLN`,
`1170:11=SPYP`). Each section also carries the comment
`Ammo=1 ;Aircraft are hard wired to require ammo`.

**So the lane's hypothesis — "projectile-like objects that never run a unit mission" — is
WRONG for stock YR, and the "not needed" answer is not available.** Being AircraftTypes
entries, these are `AircraftClass` objects, and `AircraftClass__AI @ 0x00414bb0` calls
`FootClass__AI` (verified above), whose slot-32 site reads `techno[+0x674]`. A V3 rocket in
flight therefore *does* have its `Is_Moving_Now` read by the same readiness code path as a
Harrier.

**Player-visible frequency: high.** V3 Launcher, Dreadnought and Boomer are stock buildable
units in ordinary skirmish; every shot they fire spawns one of these objects.

What is genuinely different about them, and what softens the priority: they are
`Selectable=no` / `Spawned=yes` / `DontScore=yes`, so no *player* order ever reaches them —
their mission is set by the launcher. So the consequence of a wrong Rocket mapping is not
"my unit ignores my order" but "a missile stalls mid-air or fires its trail anim wrongly".
Still player-visible, and still a stalled-entity risk.

---

## INFERRED (not verified this session — do not cite as fact)

- Fly `typeclass[+0x2f8]` is the lepton distance over which the approach speed fraction is
  interpolated (a "slow-down distance"). Its INI key is **UNCHECKED** — I did not verify
  the ReadINI field map.
- Fly `typeclass[+0x3a8]` and `[+0x3b0]` are doubles compared against / scaled into the
  current speed and the pitch angle respectively, in both the draw-matrix function
  (0x004cf610) and Process's deceleration block. `PitchSpeed=` and `PitchAngle=` are the
  obvious INI candidates (stock aircraft carry `PitchSpeed=1.1`/`0.9` and `PitchAngle=0`),
  and if `+0x3a8` really is `PitchSpeed=1.1` then the pitch branch can never fire, since
  the speed fraction is clamped to 1.0. **Both the field identification and that dead-branch
  consequence are INFERRED and worth their own check** — it is exactly the kind of thing
  that looks like a bug and is actually stock behaviour.
- Fly object+0x34 (the byte slot 4 reads) is the "has move destination" flag, per the
  constructor's pre-existing plate comment. I verified the constructor zeroes it and that
  slot 4 reads it; I did **not** find its writer, so the name is INFERRED.
- Rocket `typeclass[+0x678]` is the speed cap the phase-3/4 acceleration clamps to;
  `[+0x18]`/`[+0x1c]`/`[+0x10]`/`[+0x2c]` on the `RulesClass`-relative block chosen at the
  top of 0x006622c0 are the per-phase acceleration, hand-off altitude, turn rate and a
  flag. Not verified to INI keys.
- The two Rules type slots at `RulesClass[+0x4e0]` / `[+0x514]` / `[+0x548]` select which
  rocket behaviour block and which weapon-ready rule applies. Roles UNCHECKED.

## UNCHECKED / UNKNOWN

- **Whether the Fly slot-32 flip direction is what the Rust gate wants.** I established the
  native producer; I did not run any Rust test, and per the task I did not touch `src/`.
- Receivers of the slot-32-shaped calls in `AircraftClass__Is_Firing_Possible` @ 0x0041b965
  and `AircraftClass__Mission_Attack` @ 0x0041816b. The offset `+0x80` alone is not proof —
  `search_instructions` for `CALL [reg+0x80]` returns 80+ program-wide hits, most of them
  unrelated vtables (Bink/VQA movie code, GadgetClass, BuildingClass). Only the
  `Is_Weapon_Ready` and `FootClass__AI` receivers were verified.
- Which locomotor the four *commented-out* aircraft sections (rulesmd lines 10552, 10851,
  10912, 11180) actually get. They are NOT Fly in stock YR; the default is UNCHECKED here.
- Fly slots 5, 10, 12, 16..19, 23..25, 29, 33, 36..39 roles.
- Rocket slots other than 4, 7, 16 and 32.
- Whether `FootClass__AI`'s other three slot-32 sites (0x004da8bb, 0x004da96d, 0x004daa24)
  use the same `techno[+0x674]` receiver. Only 0x004da692 was decoded.
- Rocket object+0x48 (`param_1[0x11]` in the interface frame) is a double used as the
  flight speed by phases 3/4; I did not enumerate all of its writers.

## LABEL-DRIFT-FOUND

1. **`FlyLocomotionClass__Move_To @ 0x004cf610` is misnamed.** It is ILocomotion slot 9
   (`read_memory 0x007e89f4`), and its body (`decompile_function 0x004cf610`) calls
   `Matrix3x4_SetIdentity`, `Matrix3x4_RotateZ`, `Matrix_rotate_x_axis`,
   `Matrix_rotate_y_axis` and then copies 12 dwords into its out-parameter. That is a
   **draw/transform matrix provider**, not a movement command. It moves nothing.
2. **`FlyLocomotionClass__Stop_Moving @ 0x004cf830` is misnamed.** Slot 11. Its body
   (`decompile_function 0x004cf830`) computes `g_CurrentFrameCounter % 0x14`, feeds it
   through `Math__SinFromTable(x * 0.3141592653589793)`, and writes
   `param_2[0] = 0; param_2[1] = <sine>` — a 20-frame **sine bob offset** for drawing. It
   stops nothing and writes no locomotor state.
3. **`FlyLocomotionClass__Layer @ 0x004ccb40` is misnamed.** It is slot 16, it calls the
   helper labelled `FlyLocomotionClass__Process` (0x004cd600) plus `Horizontal_Step`, and it
   **returns slot 4 (`Is_Moving`)** via `(**(code **)(*param_1 + 0x10))(param_1)`. Shape of
   `ILocomotion::Process`, not a layer accessor. By the same token 0x004cd600, the function
   *labelled* `Process`, is an internal per-tick helper that is not in the vtable at all
   (`get_function_callers 0x004cd600` → one caller, 0x004ccb40).
4. **`FlyLocomotionClass__Begin_Takeoff @ 0x004cf950` is NOT slot 12.** Slot 12 is
   0x004cf940, a 16-byte stub (`read_memory 0x004cf940 length 16`):
   `MOV EAX,[ESP+8]; XOR ECX,ECX; XOR EDX,EDX; MOV [EAX],ECX; MOV [EAX+4],EDX; RET 8` —
   it just zeroes an 8-byte out-parameter. The `Begin_Takeoff` label itself looks correct
   for its own function (it sets IsAscending and FlightLevel), it simply is not a vtable
   entry. Same for `Begin_Landing` 0x004cfa70.
5. Confirming the anchors' own drift report from the other direction: the base-class-heavy
   slots (0x0055Axxx) recur identically in Fly and Rocket, and slot 7 = 0x0055abf0 in both,
   so the "slot 7 is the alignment check" anchor held on two more families.

No drift found in: Fly slot 4 / slot 32 addresses, Rocket vtable derivation, or the
`Move_To`=slot 9 / `Stop_Moving`=slot 11 *slot positions* (only their names are wrong).

---

## PROPOSAL (explicitly NOT a verified mapping)

Read against `src/sim/movement/ready_producer.rs` (read-only; nothing edited).

### The file's current claim about Fly and Rocket needs a correction

The `_ => None` arm at line 86 justifies itself with:

> `is_moving_now` has exactly two production consumers, the Unit and Infantry readiness
> branches in `sim::mission::readiness`. Aircraft readiness decides from its own two
> latches and never reads the locomotor at all, and Rocket-locomotor objects are not Unit-
> or Infantry-category to begin with.

As a statement about *VERA's current consumer set* that is consistent with what I saw
(Rocket objects are aircraft, so they are indeed not Unit/Infantry). As a statement about
native it is **DRIFT**:

- gamemd's aircraft **do** consult slot 32 — `AircraftClass__AI @ 0x00414bb0` calls
  `FootClass__AI`, whose gate reads `techno[+0x674]` slot 32 (both verified above), and
  `AircraftClass__Is_Weapon_Ready` computes `!Is_Moving_Now` directly.
- V3ROCKET / DMISL / CMISL are `[AircraftTypes]` (verified), so they travel that same path.

So the honest label for that comment is: *"no VERA consumer today; native has one, and the
Fly and Rocket producers are deferred residuals."* The trigger frequency for the residual
is high — every Harrier sortie and every V3 shot — but it only becomes player-visible once
VERA's aircraft path mirrors `FootClass::AI` or implements a weapon-ready rule from the
locomotor. Recording it as a residual with that trigger, rather than as "does not need
one", is the part I would change.

### If/when a Fly producer is warranted

Fly needs **one** input, a `speed_fraction != 0` test on the equivalent of object+0x48 —
*not* a destination-flag test (that is slot 4). Candidate VERA carrier, in preference
order:

1. **A new `SimFixed` field on the Fly/air locomotor state, e.g.
   `air_speed_fraction: SimFixed`,** ramped ±0.1/tick toward a target fraction, with the
   target derived from remaining horizontal distance exactly as in V8, **including
   writer C's 0.05 floor** — because that floor is what stops native from ever answering
   "not moving" while distance remains. Feeding the predicate from a fresh field rather
   than reusing `movement_target.current_speed` matters here: native's value is a
   dimensionless 0..1 fraction quantised to 0.1 steps, while `current_speed` in
   `drive_family` is a lepton-ish speed, and the two are not interchangeable.
2. The existing `F64_BITS_*` table trick generalises cleanly, because the reachable value
   set is small and closed: `{0.0, 0.05, 0.1, 0.2, ... 1.0}`. A `SimFixed`→bits lookup
   keeps float arithmetic out of `sim/`, consistent with the module's existing
   `hover_request_bits` approach.
3. `AirMovePhase` is **not** a faithful carrier on its own. Ascending/Descending are
   separate native booleans (object+0x50/+0x51) that Fly's slot 32 does not read at all, and
   a Fly aircraft is "moving now" during long stretches of every phase.

### If/when a Rocket producer is warranted

Rocket needs a **phase enum**, structurally like the existing `jumpjet` producer, not a
speed test: `is_moving_now = (3..=5).contains(&phase)` with 0/1/2/6 all false. The natural
VERA carrier is a dedicated `RocketPhase { Idle, PreLaunch, TiltRamp, Boost, Cruise,
Terminal, SubLaunch }` mapped to 0,1,2,3,4,5,6 so the predicate stays a literal range test
against the native integers. Reusing `AirMovePhase` would be wrong — its value set and
its native meanings are a different family's.

Both proposals are **UNCHECKED as mappings**: no gamemd-derived executable check compares
them, so they would land in the same "well-provenanced correspondence, not proof" tier the
module's header already uses for its six existing producers.

