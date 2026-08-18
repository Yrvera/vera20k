> **RAW LANE OUTPUT — NOT THE AUTHORITY.** This is the unedited lane 1 (per-family slot-32 table) from the
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

# LANE 1 — ILocomotion slot 32 (Is_Moving_Now) across live YR locomotor families

Program: testProsjekt / gamemd.exe, image base 0x400000. All addresses below are cited to
the tool call that produced them. READ-ONLY session; no Ghidra mutations performed.

Convention established and used throughout: the ILocomotion interface pointer is the
SECOND vtable pointer in the locomotor object, stored at object offset +0x04 (constructors
write `MOV dword ptr [ESI+0x4], <ILocomotion vtable>`). Therefore an ILocomotion method's
`this` (the `ESP+4` / `param_1` it receives, __stdcall-ish `RET 4`) points at object+0x04,
and **every this-relative offset quoted below is relative to object+0x04**. Object-relative
offset = iface offset + 4. This is confirmed by Walk: constructor zeroes `[ESI+0x34]`
(disassemble_function 0x0075aa90) and Walk slot 4 returns `*(byte*)(this+0x30)`
(anchor + decompile below) — 0x34 = 0x30 + 4.

---

## VERIFIED — vtable bases (step 1 + 2)

Every base below was validated by `read_memory <base> 160` and confirming slot 7
(byte offset 0x1C) == 0x0055abf0 (`LocomotionClass::Can_Enter_Cell`). All eight passed.

| Family   | Constructor | ILocomotion vtable base | slot 7 check | tool call |
|---|---|---|---|---|
| Drive    | 0x004af540 | **0x007e7eb0** | 0x0055abf0 OK | read_memory 0x007e7eb0 |
| Ship     | 0x0069ec50 | **0x007f2d8c** | 0x0055abf0 OK | disassemble_function 0x0069ec50 (`MOV [ESI+0x4],0x7f2d8c` @0x0069ecd8) + read_memory 0x007f2d8c |
| Walk     | 0x0075aa90 | **0x007f69f8** | 0x0055abf0 OK | disassemble_function 0x0075aa90 (`MOV [ESI+0x4],0x7f69f8` @0x0075aae5) + read_memory 0x007f69f8 |
| Hover    | 0x00513c20 | **0x007eacfc** | 0x0055abf0 OK | disassemble_function 0x00513c20 (`MOV [ESI+0x4],0x7eacfc` @0x00513c97) + read_memory 0x007eacfc |
| Fly      | 0x004cc9a0 | **0x007e89f4** | 0x0055abf0 OK | disassemble_function 0x004cc9a0 (`MOV [ESI+0x4],0x7e89f4` @0x004cca12) + read_memory 0x007e89f4 |
| Jumpjet  | 0x0054ac40 | **0x007ecd68** | 0x0055abf0 OK | disassemble_function 0x0054ac40 (`MOV [ESI+0x4],0x7ecd68` @0x0054acc9) + read_memory 0x007ecd68 |
| Teleport | 0x00718000 | **0x007f5000** | 0x0055abf0 OK | disassemble_function 0x00718000 (`MOV [ESI+0x4],0x7f5000` @0x00718064) + read_memory 0x007f5000 |
| Rocket   | 0x00661ec0 | **0x007f0b1c** | 0x0055abf0 OK | disassemble_function 0x00661ec0 (`MOV [ESI+0x4],0x7f0b1c` @0x00661f1f) + read_memory 0x007f0b1c |

Fly base 0x007e89f4 (previously "derived" in the session anchors) is now DIRECTLY
CONFIRMED from the constructor immediate, not just from an xref.

## VERIFIED — slot 4 and slot 32 pointers (steps 3 + 5)

Read out of the same 160-byte dumps; slot N is at base + 4N (slot 4 = +0x10,
slot 32 = +0x80).

| Family   | slot 4 (Is_Moving) | slot 32 (Is_Moving_Now) | slot 32 is |
|---|---|---|---|
| Drive    | 0x004afb80 | 0x004afc20 | family override |
| Ship     | 0x0069f290 | 0x0069f330 | family override |
| Walk     | 0x0075ab30 | 0x0075ab40 | family override |
| Hover    | 0x00514c30 | 0x00514c80 | family override |
| Fly      | 0x004cca90 | 0x004ccac0 | family override |
| Jumpjet  | 0x0054ae50 | 0x0054d0d0 | family override |
| Teleport | 0x00718080 | 0x004b6610 | **forwarding thunk** (NOT a 0x0055Axxx base body) |
| Rocket   | 0x00661f50 | 0x00661f90 | family override |

**No family inherits slot 32 from the LocomotionClass base range (0x0055Axxx).** Zero of
eight slot-32 entries point into 0x0055Axxx. See the "prior claim" section at the end.

---

## VERIFIED — shared primitives the slot-32 bodies use

These were decoded so the predicates below can be written exactly.

**`owner` = `*(void**)(this + 0x8)`** (i.e. object+0xC). Proved from
`LocomotionClass::Link_To_Object`, ILocomotion slot 3 = 0x0055a710
(disassemble_function 0x0055a710): it is
`MOV ECX,[ESP+4]; MOV EAX,[ESP+8]; MOV [ECX+8],EAX; MOV [ECX+4],EAX; XOR EAX,EAX; RET 8`
— the linked object pointer is stored at BOTH iface+0x4 and iface+0x8.

**`TimerRemaining(p)` = the helper at 0x004c9480** (disassemble_bytes 0x004c9480, 64 bytes),
called as `__thiscall` with `ECX = owner + 0x388`:
```
if ((int16)p[+0x14] <= 0)          return 0;   // 2-byte "active/started" field
start = (int32)p[+0x08];  dur = (int32)p[+0x10];
if (start != -1) {
    elapsed = *(int32*)0x00a8ed84 /* global frame counter */ - start;
    if (elapsed >= dur)            return 0;
    dur -= elapsed;
}
return dur != 0;
```
So term 1 of Drive/Ship slot 32 is "the CDTimer-like object at **owner+0x388** still has
ticks remaining". (0x00a8ed84 is the same global the Ship/Teleport/Rocket constructors read
to seed timers — disassemble_function 0x0069ec50 / 0x00718000 / 0x00661ec0.)

**`owner->vf_0x538()`** — the virtual at owner vtable byte offset 0x538 (slot index 334).
Verified role: `LocomotionClass::Apparent_Speed`, which every family carries at ILocomotion
slot 33 = 0x0055ad10, is *nothing but* a forward to it
(disassemble_function 0x0055ad10 = `MOV EAX,[ESP+4]; MOV ECX,[EAX+8]; MOV EDX,[ECX];
CALL [EDX+0x538]; RET 4`). Its only 13 callsites in the program
(search_instructions CALL "+ 0x538]") are all locomotor movement code plus
`TechnoClass__Resolve_ArchiveTarget_Coords`. Returns a signed int.
The *identity* of the owning class's method name is UNCHECKED — I did not resolve slot 334
in a concrete TechnoClass/FootClass vtable this session.

**Null-coord sentinels are all (0,0,0).** Each family compares its stored coords against a
per-family 3-dword static:
- Drive `0x008a0790/94/98` — read_memory 0x008a0790 = 12 zero bytes; and
  `DriveLocomotionClass__InitNullCoords` @0x004af4e0 is exactly
  `XOR EAX,EAX; MOV [0x008a0790],EAX; MOV [0x008a0794],EAX; MOV [0x008a0798],EAX; RET`
  (disassemble_function 0x004af4e0).
- Ship `0x00b077f8/fc/0x00b07800` — read_memory 0x00b077f8 = 12 zero bytes.
- Walk `0x00b45be8/ec/f0` — read_memory 0x00b45be8 = 12 zero bytes.
- Hover `0x00a8f180/84/88` — referenced by both the constructor and slot 4
  (disassemble_function 0x00513c20, disassemble_bytes 0x00514c30). Value UNCHECKED
  (not read this session) but it is the same static the constructor seeds the coords from,
  so the test is structurally "coords still hold their initial value".
- Teleport `0x00b0ebf8/fc/0x00b0ec00`, Walk-shaped (disassemble_function 0x00718000).
- Rocket `0x00b04e38/3c/40` (disassemble_function 0x00661ec0).

**Float/double zero constants:** read_memory 0x007e2800 = 8 zero bytes = `0.0` (double);
read_memory 0x007e1748 = 4 zero bytes = `0.0f` (float).

**FPU compare decoding used below** (this is where a paraphrase would silently invert the
predicate, so it is spelled out): after `FLD x; FCOMP y; FNSTSW AX`, `AH` bit 0x40 is C3
(x == y) and bit 0x01 is C0 (x < y).
- `TEST AH,0x40` then branch-on-nonzero-to-false  ⇒ predicate is `x != y`
- `TEST AH,0x41` then branch-on-nonzero-to-false  ⇒ predicate is `x > y`

---

## VERIFIED — per-family predicates

`this` = ILocomotion interface pointer = object+0x04. Object offset = listed offset + 4.

### DRIVE — slot 32 = 0x004afc20, slot 4 = 0x004afb80

slot 32 (disassemble_bytes 0x004afc20 96 bytes + 0x004afc7f 10 bytes;
decompile_function 0x004afc20):
```
Is_Moving_Now():
    if ( TimerRemaining(owner + 0x388) )                    return true;   // early OUT
    if ( !this->vt[4](this) /* Drive Is_Moving, slot 4 */ )  return false;
    if (   (int32)this[+0x3c] == 0
        && (int32)this[+0x40] == 0
        && (int32)this[+0x44] == 0 )                        return false;  // dest coord is (0,0,0)
    if ( (int32)owner->vf_0x538() <= 0 )                    return false;  // signed JLE
    return true;
```
Exact conjunction, short-circuit order preserved:
`TimerRemaining(owner+0x388) || ( slot4() && !(this[+0x3c]==0 && this[+0x40]==0 && this[+0x44]==0) && owner->vf_0x538() > 0 )`

slot 4 (decompile_function 0x004afb80, tail confirmed by disassemble_bytes 0x004afbe0):
```
Is_Moving():
    if ( this[+0x30] != 0 || this[+0x34] != 0 || this[+0x38] != 0 ) return true;   // "immediate/head-to" coord non-null
    if ( this[+0x3c] == 0 && this[+0x40] == 0 && this[+0x44] == 0 ) return false;  // dest coord null
    if ( this[+0x3c] == (int32)owner[+0x9c] && this[+0x40] == (int32)owner[+0xa0] ) return false;  // dest == owner's current X,Y
    return true;
```
All fields int32. NOTE: the third test compares **X and Y only** — the Z word owner[+0xa4]
is loaded into a stack slot at 0x004afbdd/0x004afbe0 and never compared (dead 12-byte
CoordStruct copy). Verified in raw asm, not just the decompiler.

**Difference:** slot 32 adds (a) a timer short-circuit that reports moving even when slot 4
is false, and (b) a speed>0 requirement. slot 4 adds the "destination already equals my
current cell coord" test, which slot 32 does not have; slot 32's coord test is the plain
`dest == (0,0,0)` test including Z.

### SHIP — slot 32 = 0x0069f330, slot 4 = 0x0069f290

slot 32 (disassemble_bytes 0x0069f330 112 bytes): **byte-for-byte the same shape as Drive
slot 32**, same offsets (owner+0x388 timer, this+0x3c/0x40/0x44, owner->vf_0x538, signed
JLE), only the null-coord globals differ (0x00b077f8/fc/0x00b07800). The Ghidra function is
UNLABELED (`FUN_0069f330`).
```
TimerRemaining(owner+0x388)
|| ( slot4() && !(this[+0x3c]==0 && this[+0x40]==0 && this[+0x44]==0) && owner->vf_0x538() > 0 )
```
slot 4 body at 0x0069f290: **UNCHECKED this session** (address confirmed from the vtable
dump; body not decoded).

### WALK — slot 32 = 0x0075ab40, slot 4 = 0x0075ab30

slot 4 (disassemble_bytes 0x0075ab30): `MOV EAX,[ESP+4]; MOV AL,byte ptr [EAX+0x30]; RET 4`
```
Is_Moving():  return *(uint8*)(this + 0x30) != 0;     // object+0x34, the byte the ctor zeroes
```
slot 32 (disassemble_bytes 0x0075ab40 32 bytes + 0x0075ab4f 80 bytes):
```
Is_Moving_Now():
    if ( !this->vt[4](this) )                       return false;     // virtual call, not an inlined field read
    if ( !((double)owner[+0x578] > 0.0) )           return false;     // FLD double [owner+0x578]; FCOMP 0.0; TEST AH,0x41
    if (   (int32)this[+0x24] == 0
        && (int32)this[+0x28] == 0
        && (int32)this[+0x2c] == 0 )                return false;     // dest coord is (0,0,0)
    return true;
```
Exact form: `slot4() && ((double)owner[+0x578] > 0.0) && !(this[+0x24]==0 && this[+0x28]==0 && this[+0x2c]==0)`

**Difference:** slot 4 is a bare bool field; slot 32 = that bool AND a strictly-positive
double speed field on the owner AND a non-null destination coord. Walk reads a **double at
owner+0x578 directly**, where Drive/Ship call the **virtual owner->vf_0x538()** — different
mechanism, and whether the two agree in all states is UNCHECKED.

### HOVER — slot 32 = 0x00514c80, slot 4 = 0x00514c30

slot 4 (disassemble_bytes 0x00514c30 80 bytes):
```
Is_Moving():
    if (   this[+0x14]==NullX && this[+0x18]==NullY && this[+0x1c]==NullZ
        && this[+0x20]==NullX && this[+0x24]==NullY && this[+0x28]==NullZ ) return 0;
    return 1;
```
(NullX/Y/Z = the 0x00a8f180/84/88 static. Both triples are the ones the constructor seeds
from that same static — disassemble_function 0x00513c20 writes object+0x18.. and
object+0x24.., i.e. iface+0x14.. and iface+0x20.. All six fields int32.)

slot 32 (disassemble_bytes 0x00514c80 48 bytes):
```
Is_Moving_Now():
    if ( !this->vt[4](this) )              return 0;
    if ( !((double)this[+0x44] != 0.0) )   return 0;   // FLD double [this+0x44]; FCOMP 0.0; TEST AH,0x40
    return 1;
```
Exact form: `slot4() && ((double)this[+0x44] != 0.0)`
`this[+0x44]` is object+0x48, the 8-byte double the constructor zeroes
(`MOV [ESI+0x48],EBX` / `MOV [ESI+0x4c],EBX` @0x00513c6c/0x00513c78).

**Difference:** slot 32 = slot 4 AND a nonzero own-object double (a speed/velocity field on
the *locomotor*, not on the owner). Note `!= 0.0`, **not** `> 0.0` — a negative value
counts as moving-now. This is the single most easily-inverted detail in this lane; the
`TEST AH,0x40` (C3 only) is what proves it.

**The pre-existing Ghidra label `HoverLocomotionClass__Is_Moving_Now @ 0x00514c80` is
CORRECT** — it really is Hover's slot 32. (The session anchor listed it as UNCHECKED.)

### FLY — slot 32 = 0x004ccac0, slot 4 = 0x004cca90

slot 32 (disassemble_bytes 0x004ccac0 40 bytes) — reconfirmed independently:
```
Is_Moving_Now():  return ((double)this[+0x44] != 0.0);      // TEST AH,0x40 → C3 only → "!="
```
No slot-4 call, no owner dereference. `this[+0x44]` = object+0x48, zeroed by the
constructor at 0x004cc9e8/0x004cc9f4 (both halves of the double).

slot 4 (disassemble_bytes 0x004cca90 48 bytes):
```
Is_Moving():
    if ( *(uint8*)(this + 0x30) != 0 )               return 1;      // object+0x34, ctor-zeroed byte
    if ( (float)owner[+0x2e8] > 0.0f )               return 1;      // FLD float [owner+0x2e8]; FCOMP 0.0f; TEST AH,0x41; JZ→1
    return 0;
```
Exact form: `(this[+0x30] != 0) || ((float)owner[+0x2e8] > 0.0f)`  — a **disjunction**,
where every other family's slot 32 is a conjunction.

**Difference:** Fly's two predicates share nothing. slot 4 is "flag OR owner float > 0";
slot 32 is "own double != 0". Reading Fly's shape off slot 4 and using it as the gate would
be wrong in both directions (a grounded aircraft with a nonzero owner+0x2e8 reports
Is_Moving true / Is_Moving_Now false; a locomotor mid-glide with the flag clear and
owner+0x2e8 == 0 reports the reverse).

### JUMPJET â€” slot 32 = 0x0054d0d0, slot 4 = 0x0054ae50

slot 4 (disassemble_bytes 0x0054ae50 24 bytes): `MOV EAX,[ESP+4]; MOV AL,byte ptr [EAX+0x48]; RET 4`
```
Is_Moving():  return *(uint8*)(this + 0x48) != 0;   // object+0x4c, ctor-zeroed (MOV byte [ESI+0x4c],BL @0x0054ac6a)
```
slot 32 (disassemble_bytes 0x0054d0d0 64 bytes):
```
Is_Moving_Now():
    s = *(int32*)(this + 0x4c);        // object+0x50, ctor-zeroed dword @0x0054ac6d
    if (s == 0) return false;
    if (s == 2) return false;
    return true;
```
Exact form: `s = (int32)this[+0x4c];  return (s != 0 && s != 2);`

**Difference:** the two slots read **completely different fields** â€” slot 4 a bool byte at
object+0x4c, slot 32 an int32 state enum at object+0x50. No overlap at all, so a
slot-4-derived shape for Jumpjet would be pure invention. (Corroborating that +0x50 is a
flight-state enum: the adjacent `__thiscall` function at 0x0054d0f0, visible in the same
disassemble_bytes window, tests the same object+0x50 against 2 and 3.)

### TELEPORT â€” slot 32 = 0x004b6610 (forwarding thunk), slot 4 = 0x00718080

slot 32 (disassemble_bytes 0x004b6610 32 bytes):
```
0x004b6610:  MOV EAX,[ESP+4]; PUSH EAX; MOV ECX,[EAX]; CALL dword ptr [ECX+0x10]; RET 4
```
`[ECX+0x10]` is slot 4. So Teleport's Is_Moving_Now is *defined as* a virtual re-dispatch of
its own Is_Moving â€” no extra state, no speed term, no timer.

slot 4 (disassemble_bytes 0x00718080 32 bytes):
`MOV EAX,[ESP+4]; CMP byte ptr [EAX+0x30],0x1; SETZ AL; RET 4`
```
Is_Moving():      return (*(uint8*)(this + 0x30) == 1);   // object+0x34, ctor-zeroed
Is_Moving_Now():  the same value, reached through the vtable.
```
Note the comparison is **equality with exactly 1**, not `!= 0`. object+0x34 is one of a
three-byte group (0x34/0x35/0x36) the constructor zeroes (disassemble_function 0x00718000),
so values other than 0/1 are representable and would read as *not* moving.

**Difference:** none â€” slot 32 == slot 4 by construction for Teleport. This is the one
family where reading the shape off slot 4 gives the right answer.

### ROCKET â€” slot 32 = 0x00661f90, slot 4 = 0x00661f50

Both decoded from disassemble_bytes 0x00661f50 (80 bytes) + disassemble_bytes 0x00661fa1 (16 bytes).
```
Is_Moving():       // slot 4
    if ( this[+0x14]==0 && this[+0x18]==0 && this[+0x1c]==0 ) return 0;   // int32 triple vs 0x00b04e38/3c/40
    return 1;

Is_Moving_Now():   // slot 32
    s = *(int32*)(this + 0x3c);      // object+0x40, ctor-zeroed dword @0x00661f06
    if (s < 3) return false;         // signed JL
    if (s > 5) return false;         // signed JG
    return true;
```
Exact form: `s = (int32)this[+0x3c];  return (3 <= s && s <= 5);`

**Difference:** again two disjoint fields â€” slot 4 a coord-nonnull test on the triple at
object+0x18, slot 32 a signed range test on the state enum at object+0x40.

---

## VERIFIED â€” the base class, and who inherits slot 32

**The LocomotionClass base ILocomotion vtable is 0x007eadf4.** Proved by
get_xrefs_to 0x007eadf4 -> `From 0055a6db in LocomotionClass__Constructor [DATA]` and
`From 0055a6f6 in LocomotionClass__Destructor [DATA]`; derived constructors overwrite
`[ESI+4]` after calling it. Validated as an ILocomotion vtable: read_memory 0x007eadf4,
slot 7 == 0x0055abf0.

- base slot 4  = **0x0055acd0**, body `XOR AL,AL; RET 4` -> **always false**
  (disassemble_bytes 0x0055acd0).
- base slot 32 = **0x004b6610**, the forwarding thunk -> **`return Is_Moving()` through the
  vtable**. So the base default for Is_Moving_Now is "whatever my Is_Moving says", and for an
  unspecialised LocomotionClass that is `false`.

**Exactly three vtables install 0x004b6610, and all three install it at slot 32**
(get_xrefs_to 0x004b6610 -> 0x007e82f8, 0x007eae74, 0x007f5080; each is exactly base+0x80
for 0x007e8278, 0x007eadf4, 0x007f5000 respectively):

| vtable base | family | live in YR? |
|---|---|---|
| 0x007f5000 | Teleport | **live** (Chrono Legionnaire, chrono-warping units) |
| 0x007e8278 | DropPod  | dormant TS (slot 4 = 0x004b5b30; base confirmed by disassemble_function 0x004b5b00 writing `[ESI+4],0x7e8278`) |
| 0x007eadf4 | LocomotionClass base itself | n/a |

**Answer to the lane question:** among live YR families, **Teleport is the only one that
keeps the base slot-32 behaviour â€” CONFIRMED.** The prior session's claim is right in
substance. Its *mechanism* needs one correction: the base slot-32 entry is not a body in the
0x0055Axxx range, it is the forwarding thunk at 0x004b6610; nothing in 0x0055Axxx ever
appears at slot 32 in any vtable read this session. Drive, Ship, Walk, Hover, Fly, Jumpjet
and Rocket all override slot 32 with their own bodies.

---

## VERIFIED â€” which coord triple is which (settles the Drive/Walk field naming)

Checked because the Rust producers name their coord inputs, and swapping the two triples
would invert the gate.

**Drive:** slot 5 = 0x004afc90 (`Destination`) copies out **this+0x30/0x34/0x38**
(disassemble_bytes 0x004afc90 40 bytes). slot 6 = 0x004afcc0 (`Head_To_Coord`) copies out
**this+0x3c/0x40/0x44**, falling back to the owner's coord at owner+0x9c/0xa0/0xa4 when that
triple is null (decompile_function 0x004afcc0). Therefore:
- `this+0x30..0x38` = **Destination**
- `this+0x3c..0x44` = **Head-To** â€” and this is the triple that slot 32's null test uses, and
  the one slot 4 compares against the owner's current X,Y.

**Walk:** slot 5 = 0x0075aba0 (`Destination`) copies out **this+0x18/0x1c/0x20** (returning
the null coord when slot 4 is false) â€” disassemble_bytes 0x0075aba0 88 bytes. slot 6 =
0x0075ac00 (`Head_To`) copies out **this+0x24/0x28/0x2c** â€” disassemble_bytes 0x0075ac00
72 bytes. Therefore Walk slot 32's coord term reads the **Head-To**, not the Destination.

---

## Cross-check against the Rust producers (read-only, no files modified)

`src/sim/movement/locomotor_ready.rs` lines 69-99 and `src/sim/movement/ready_producer.rs`.
**Verdict: all six Rust predicates are slot-32 shapes, not slot-4 shapes.** The worry that
motivated this lane does not materialise.

| Rust predicate (locomotor_ready.rs:82-97) | verified slot 32 | match |
|---|---|---|
| Drive/Ship `turning_active \|\| (slot_moving && head_to_nonnull && owner_speed > 0)` | `TimerRemaining(owner+0x388) \|\| (slot4() && head_to_nonnull && vf_0x538() > 0)` | structure, short-circuit order and signedness all match |
| Hover `slot_moving && native_double_ordered_not_zero(speed_bits)` | `slot4() && (double)this[+0x44] != 0.0` | matches, including that it is `!= 0.0` and not `> 0.0`, and NaN -> false (an unordered compare sets C3, so the native `TEST AH,0x40` branch also returns false) |
| Walk `moving_byte != 0 && native_double_ordered_gt_zero(bits) && destination_nonnull` | `this[+0x30]!=0 && (double)owner[+0x578] > 0.0 && head_to_nonnull` | matches; NaN and -0.0 both -> false on each side |
| Teleport `state == 1` | `*(uint8*)(this+0x30) == 1` | exact, including `== 1` rather than `!= 0` |
| Jumpjet `state != 0 && state != 2` | `(int32)this[+0x4c] != 0 && != 2` | exact, signed i32 on both sides |

Two doc-level items for the Rust side (neither changes today's behaviour):

1. `ready_producer.rs:109` calls Drive/Ship's first term `turning_active`. What is verified
   is that the term is `TimerRemaining(owner + 0x388)`. **That owner+0x388 is the
   turn/rotation timer is UNCHECKED** â€” the field was not identified this session. This is
   the only Drive/Ship term whose misreading yields a false "moving", i.e. the stall
   direction, so it is worth pinning down.
2. `ready_producer.rs:83-85` says the non-produced families "do own a readiness-slot override
   rather than inheriting the base one". Verified TRUE for **Fly** (0x004ccac0) and **Rocket**
   (0x00661f90). **FALSE for DropPod**, which uses the base forwarding thunk 0x004b6610.
   Parachute, Mech and Tunnel slot 32 are UNCHECKED.
   Separately: Walk's third input is correctly documented as the head-to (verified above)
   even though the enum field is named `destination_nonnull`.

---

## LABEL DRIFT FOUND / CORRECTED

1. **The session anchor's claim that `ILocomotion__Is_Moving_Now_Thunk @ 0x004b6610` is
   MISNAMED is itself wrong â€” the label is correct.** The function is installed **only** at
   slot 32 (Is_Moving_Now), in exactly three vtables, and nowhere else in the program
   (get_xrefs_to 0x004b6610 returns 0x007e82f8 / 0x007eae74 / 0x007f5080, each = its vtable
   base + 0x80). Its body calling `[ECX+0x10]` (slot 4) is not evidence of misnaming: it *is*
   the Is_Moving_Now implementation, and it forwards to Is_Moving. The earlier inference read
   the callee slot and drew a conclusion about the caller's slot.
2. **`HoverLocomotionClass__Is_Moving_Now @ 0x00514c80` is CORRECT** â€” read_memory 0x007eacfc
   puts it at slot 32. (The anchor listed its slot as UNCHECKED.)
3. **`WalkLocomotionClass__Is_Moving_Now @ 0x0075ab40` is CORRECT** â€” read_memory 0x007f69f8
   slot 32.
4. **`TeleportLocomotionClass__Is_Moving @ 0x00718080` is CORRECT** â€” read_memory 0x007f5000
   slot 4.
5. **Ship's slot 32 is UNLABELED** (`FUN_0069f330`) despite being the same shape as
   `DriveLocomotionClass__Is_Moving_Now`. Also unlabeled: **Jumpjet slot 32 (0x0054d0d0)**,
   **Jumpjet slot 4 (0x0054ae50)**, **Rocket slot 32 (0x00661f90)**, **Rocket slot 4
   (0x00661f50)**, **Fly slot 32 (0x004ccac0)**, **Hover slot 4 (0x00514c30)**, and the base
   bodies **0x0055acd0** / **0x0055ace0**.
   NOT written back â€” this lane is read-only; no Ghidra mutation tool was called and
   `save_program` was not called.
6. Fly slot 39 and Rocket slot 39 are both **0x004b6620**, whose body is a bare `RET 8`
   (visible in the disassemble_bytes 0x004b6610 window) â€” a no-op two-argument stub.
   Unlabeled.

---

## UNCHECKED / UNKNOWN

- **`owner->vf_0x538()`'s owning class and method name** â€” verified only as "the virtual at
  owner-vtable byte offset 0x538 that `LocomotionClass::Apparent_Speed` forwards to,
  returning a signed int". Slot 334 was not resolved in a concrete TechnoClass/UnitClass
  vtable.
- **The meaning of `owner+0x388`** (the CDTimer-like object Drive/Ship slot 32 tests) â€” the
  helper at 0x004c9480 is decoded exactly, but which timer lives at owner+0x388 is UNKNOWN.
- **The meaning of `owner+0x578` (double, Walk slot 32) and `owner+0x2e8` (float, Fly slot
  4)** â€” read exactly as fields; roles inferred from context only.
- **Ship slot 4 body (0x0069f290)** â€” decompile_function 0x0069f290 shows the same shape as
  Drive slot 4, but its raw asm tail was not re-read, so the "X and Y only, Z load is dead"
  detail is INFERRED for Ship and VERIFIED only for Drive.
- **Hover's null-coord static value at 0x00a8f180/84/88** â€” not read this session.
- **Whether Walk's `owner+0x578` double and Drive/Ship's `vf_0x538()` int agree** in all
  states â€” UNCHECKED; they are different mechanisms.
- **Parachute, Mech and Tunnel slot 32** â€” not read. DropPod was the only dormant/TS family
  resolved, and only because it surfaced as an xref to the base thunk.
- No emulation and no live trace: every statement here is static decode. Per project rules
  that makes these VERIFIED *reads of the binary*, not a VERIFIED *parity claim* about the
  Rust â€” no gamemd-derived executable check compares the two.

