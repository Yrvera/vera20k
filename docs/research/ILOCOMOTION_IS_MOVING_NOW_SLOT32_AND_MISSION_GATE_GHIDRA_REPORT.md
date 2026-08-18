# ILocomotion `Is_Moving_Now` (slot 32) and the Mission readiness gate

**Date:** 2026-07-30
**Program:** `gamemd.exe`, Ghidra project `testProsjekt`, image base `0x400000`, 10036 functions.
**Method:** static decode only — `decompile_function`, `disassemble_bytes`, `read_memory`,
`get_xrefs_to`, `search_instructions`. No emulation, no live trace.
**Companion code:** landed as `9f153be3` (derive readiness at the gate instead of caching
per tick). Raw per-lane decodes with full per-call citations are preserved beside this file
as `ILOCOMOTION_IS_MOVING_NOW_SLOT32_LANE[1-4]_*.md`.

Per project rules this is a set of **VERIFIED reads of the binary**, not a VERIFIED parity
claim about the Rust: no gamemd-derived executable check compares the two.

---

## Why this document exists

The Rust readiness producers had per-family doc comments asserting a native predicate shape,
and it was unclear whether each had been read off **slot 4 (`Is_Moving`)** or **slot 32
(`Is_Moving_Now`)**. Those are different predicates in every live family except one, and the
Mission gate reads only slot 32. This settles the table, and settles where the gate actually
is.

Answer to the motivating worry: **all six Rust predicates were already slot-32 shapes.** The
defects found were elsewhere — see "Corrections" below.

---

## Frame convention (load-bearing, verified three independent ways)

The ILocomotion vtable pointer is the object's **second** vptr, at `object+0x04`. So an
ILocomotion method's `this` is `object+4`, and **every offset below is interface-relative**
(object offset = listed + 4). `owner = *(void**)(iface+0x8)`.

Proved by: constructor stores (`disassemble_function 0x00661ec0` writes `[ESI+4]`), the RTTI
complete-object-locator `offset = 4`, and slot 16 doing `LEA EDI,[ESI-0x4]` before calling
`Process`.

Slot index N sits at byte offset N*4. **Slot 7 is `0x0055abf0`
(`LocomotionClass::Can_Enter_Cell`) in every family**, which makes it a reliable check that a
vtable base is correctly aligned before trusting anything else read from it.

Vtable length is **50 slots** for Drive/Walk/base (38 for Fly) — an earlier "40 slots"
figure was wrong. Not load-bearing for this report; every interesting slot is below 38.

---

## The settled slot-32 table

| Family | ILocomotion base | slot 32 | Exact predicate (interface-relative) |
|---|---|---|---|
| Drive | `0x007e7eb0` | `0x004afc20` | `BodyFacing_IsRotating(owner+0x388) \|\| ( slot4() && !(i32 this[+0x3c/+0x40/+0x44] == NullCoord) && (i32)owner->vt[0x538]() > 0 )` — signed `JLE` |
| Ship | `0x007f2d8c` | `0x0069f330` | byte-identical shape to Drive (same facing term), same offsets, own NullCoord globals `0x00b077f8/fc/800` |
| Walk | `0x007f69f8` | `0x0075ab40` | `slot4() && ((double)owner[+0x578] > 0.0) && !(this[+0x24/+0x28/+0x2c] == NullCoord)` |
| Hover | `0x007eacfc` | `0x00514c80` | `slot4() && ((double)this[+0x44] != 0.0)` |
| Fly | `0x007e89f4` | `0x004ccac0` | `((double)this[+0x44] != 0.0)` — no slot-4 call, no owner deref |
| Jumpjet | `0x007ecd68` | `0x0054d0d0` | `s = (i32)this[+0x4c]; s != 0 && s != 2` — enum decoded, see RESOLVED below |
| Teleport | `0x007f5000` | `0x004b6610` | inherited base thunk → `slot4()` → `byte this[+0x30] == 1` |
| Rocket | `0x007f0b1c` | `0x00661f90` | `s = (i32)this[+0x3c]; 3 <= s && s <= 5` (signed `JL`/`JG`) |
| Mech (dormant TS) | `0x007edb6c` | `0x005b19e0` | Drive-shaped, different offsets (coord at `this+0x20`) |
| DropPod (dormant TS) | `0x007e8278` | `0x004b6610` | inherited base thunk; base slot 4 is `XOR AL,AL` ⇒ **always false** |
| LocomotionClass base | `0x007eadf4` | `0x004b6610` | `slot4()`; base slot 4 `0x0055acd0` = always false |
| Tunnel | *none* | — | Tunnel's only COL has `offset = 0`: **no ILocomotion subobject, no slot 32** |
| Parachute | *does not exist* | — | there is no `ParachuteLocomotionClass` in the binary (12 locomotor classes total) |

Details that invert behaviour if misread:

- **Hover and Fly test `!= 0.0`, not `> 0.0`** (`TEST AH,0x40`, C3 only). A *negative* speed
  counts as moving. NaN reads as not-moving on both, since an unordered compare also sets C3.
- **Teleport tests equality with exactly 1**, not `!= 0`. Its byte is one of a three-byte
  group the constructor zeroes, so other values are representable and read as not-moving.
- **Drive slot 4 compares X and Y only.** The Z word `owner+0xa4` is loaded to a stack slot
  and never `CMP`'d — a dead 12-byte CoordStruct copy. Verified in raw asm at
  `0x004afbdd`/`0x004afbe0`, not just the decompiler. (For Ship this detail is INFERRED from
  the decompile only.)
- **Which coord triple is which, for Drive:** slot 5 `Destination` (`0x004afc90`) copies out
  `this+0x30..0x38`; slot 6 `Head_To_Coord` (`0x004afcc0`) copies out `this+0x3c..0x44`,
  falling back to the owner's coord when null. Slot 32's null test uses the **head-to**
  triple. For Walk, slot 5 copies `this+0x18..0x20` and slot 6 copies `this+0x24..0x2c`, so
  Walk slot 32's coord term is likewise the **head-to**, not the destination.

**Inheritance, settled exhaustively.** `get_xrefs_to 0x004b6610` returns exactly three data
refs — `0x007e82f8`, `0x007eae74`, `0x007f5080` — each precisely its vtable base + `0x80`
(DropPod, base, Teleport). So among live YR families **Teleport is the only one that keeps
the base slot-32 answer**, and it does so through a forwarding thunk rather than a body in
the `0x0055Axxx` range. Drive, Ship, Walk, Hover, Fly, Jumpjet and Rocket all override.

---

## The gate

**It is not in `FootClass::AI`, and not in the mission dispatcher.** It is a virtual on the
object's **own** vtable at offset `0x200`, and `Commence` is offset `0x1ec`.

`MissionClass::Queue_Mission` (`0x005b35e0`) is the shape:

```c
if (commence_now != 0) {
    cVar2 = (**(code **)(*param_1 + 0x200))();   // readiness
    if (cVar2 != 0) {
        (**(code **)(*param_1 + 0x1ec))();       // Commence
    }
}
```

Base found from `InfantryClass__Constructor` storing `0x7eb058`; cross-checked because
`0x007eb058+0x204` lands on `MissionClass__Mission_Default`, which
`MissionClass__Mission_Dispatch` uses for its `case 0`/`default`.

Every implementation of slot `0x200`:

| impl | class | consults the locomotor? |
|---|---|---|
| `0x00521b60` | InfantryClass | **yes — ILocomotion slot 32**, live call at `0x00521ba7` |
| `0x00744270` | UnitClass | **yes — ILocomotion slot 32** |
| `0x0041b5e0` | AircraftClass | **no** — `mission != 6 && mission != 0x15 && (byte[+0x6d2]==0 \|\| mission==0x1e)` then `return byte[+0x6d4] != 0` |
| `0x00454250` | BuildingClass | no — `return byte[+0x6dd] != 0` |
| `0x004e0140` | abstract intermediates (4 vtables) | no — `return 1` |

The Infantry gate defers whenever slot 32 says moving, except for two `vtbl[0x184]`
categories that are exempt outright and a third exempt only when `+0x2b4` is non-zero; it
additionally defers while the current sequence's flag byte in the table at `0x007eaf7c` is
zero. The Unit gate has the same shape plus its own preconditions and a deploy/bridge-hut
proximity test.

### Ordering — the decisive finding

**Readiness is evaluated live at every gate call. No cached per-frame moving flag exists
anywhere on that path.** Each invocation is a fresh virtual call on the object's own vtable
(`disassemble_bytes 0x0051bbf0`: `MOV EAX,[ESI]; MOV ECX,ESI; CALL [EAX+0x200]`), and the
gate body itself then performs a fresh `CALL [ECX+0x80]` on the locomotor interface.

Two things make this more than stylistic:

1. **The same object's readiness is evaluated on both sides of its own locomotion within one
   tick.** `InfantryClass__AI`: gate `0x0051bc1c` → Commence `0x0051bc51` → `FootClass::AI`
   `0x0051bc9f` (whose locomotor `Process` call is at `0x004da877`) → gate `0x0051bed1` →
   Commence `0x0051bf03`. `UnitClass__AI`: `0x00736465` → `0x00736473` → `0x0073647b` →
   `0x007366ef` → `0x007366fd`. `AircraftClass__AI` calls `FootClass::AI` first, at
   `0x00414da3`, then gates at `0x0041504a`.
2. **The gate is consulted from far more than the AI loops.** An exhaustive
   `search_instructions` for `CALL [reg+0x200]` returns **28 hits, not truncated**, including
   `FootClass__Receive_Radio` ×2, `UnitClass__Receive_Radio`, `UnitClass__PerCellProcess`,
   `TechnoClass__Unlimbo`, `AircraftClass__Set_Destination`,
   `UnitClass__Mission_Deploy_Building` ×5, `TeleportLocomotionClass__Process`,
   `BuildingClass__Update` ×2. These fire mid-tick, in response to events that themselves
   change locomotor state.

A once-per-tick cache is therefore correct for at most one of those sites — the one
immediately preceding that object's own locomotion — and stale for the rest.

### Coverage for aircraft and missiles

Aircraft **do** reach the gate (`0x0041504a` in `AircraftClass__AI`); the AircraftClass
*override* is simply what never consults the locomotor. And Rocket-locomotor objects are
`AircraftClass` too, not vehicles: `V3ROCKET`, `DMISL` and `CMISL` are listed under
`[AircraftTypes]` in `ini/rulesmd.ini` (roster lines 1163/1165/1171) with
`Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}` (lines 11389, 11429, 11472).

So the accurate statement is: **for any Fly- or Rocket-locomotor object the readiness answer
is independent of the locomotor's moving predicate** — not that the slot is absent or dead.
Slot 32 for those families is still read every tick by `FootClass::AI` for the
sight/occupancy refresh and the move-sound state, and `AircraftClass__Is_Weapon_Ready` is
literally its negation.

---

## Corrections to earlier claims in this repo

1. **`ILocomotion__Is_Moving_Now_Thunk @ 0x004b6610` is correctly named.** An earlier pass
   this session read its body calling `[ECX+0x10]` (slot 4) and concluded the *label* was
   drift, renaming it. That was wrong — it is installed only at slot 32, in exactly three
   vtables, and forwarding to slot 4 is what the base implementation *does*. The rename was
   reverted and the function now carries a plate comment recording both the correct role and
   the trap. Reading a callee's slot tells you nothing about the caller's slot.
2. **Hover's and Walk's existing `Is_Moving_Now` labels were already correct**, and Teleport's
   `Is_Moving` label too — all confirmed against the vtable slot bytes.
3. **`UnitClass__ShouldIdle @ 0x00744270` is misnamed.** It occupies self-vtable slot `0x200`,
   the same slot as `AircraftClass__Is_Ready`, and it is what `Queue_Mission` consults before
   `Commence`. It is the UnitClass readiness override, not an idle-selection helper.
4. **`FUN_00521b60` is the InfantryClass readiness override** and is unnamed.
5. **`AircraftClass__Override_Mission @ 0x0041b870` sits in the `Commence` slot**, and calls
   the base `MissionClass__Commence` at `0x0041b880` — i.e. it is the Commence override. The
   `Override_Mission` name is drift.
6. Unlabeled despite being load-bearing here: Ship slot 32 (`0x0069f330`), Jumpjet slot 32
   (`0x0054d0d0`) and slot 4 (`0x0054ae50`), Rocket slot 32 (`0x00661f90`) and slot 4
   (`0x00661f50`), Fly slot 32 (`0x004ccac0`, no function even defined), Hover slot 4
   (`0x00514c30`), and the base bodies `0x0055acd0` / `0x0055ace0`.
7. **Anything in `0x004b4xxx`/`0x004b6xxx` named after a specific predicate should be
   re-checked against the `[ECX+offset]` it dispatches to.** That block is one-line
   forwarding thunks, and it is where the trap in item 1 lives.

---

## RESOLVED 2026-07-30 (were the top two unknowns)

### `owner+0x388` is the owner's body FacingClass

The countdown Drive and Ship slot 32 test as their first term. **The term means "the hull is
still rotating."** Landed as `45f6f2b1`.

The field is a facing interpolator, not a bare timer:

| offset | type | meaning |
|---|---|---|
| `+0x00` | dword | current/target facing (low word is the 16-bit facing) |
| `+0x04` | dword | previous facing — the value being interpolated *from* |
| `+0x08` | dword | timer start frame (`-1` = not started) |
| `+0x0c` | dword | timer field |
| `+0x10` | dword | timer duration, in frames |
| `+0x14` | int16 | turn **rate**; `<= 0` disables interpolation entirely |

`FacingClass::Set` (`0x004c9220`, was mislabeled `RateTimer__Set`) early-outs when the new
facing equals the current one; otherwise it moves `+0x00` into `+0x04` (partially advancing it
first if a turn is already in flight), stores the new facing, stamps `+0x08` with the current
frame and sets `+0x10 = |new - previous| / rate`. So **the timer's duration is the turn
length**. Verified via `disassemble_function 0x004c9220`.

`FacingClass::Is_Rotating` (`0x004c9480`, was mislabeled `CDTimerClass__Remaining` — it
returns a bool, not a count) gates on `rate > 0`, then answers whether the interpolation still
has frames left. Verified via `disassemble_function 0x004c9480`.

**What ties the field to turning rather than to some other countdown:** the locomotor's own
`Do_Turn` slot writes exactly it, through exactly that setter.
`disassemble_function 0x004b0ef0` is five instructions —
`MOV EDX,[ESP+4]; MOV EAX,[ESP+8]; LEA ECX,[ESP+8]; PUSH ECX; MOV ECX,[EDX+0x8];
ADD ECX,0x388; CALL 0x004c9220`. `[EDX+0x8]` is the owner, so the locomotor steers the hull
through this object.

Caveat on the *name*: `Is_Rotating`'s 15 callers include `UnitClass__Facing_Update`, both
ground locomotors' `Process` and `Is_Moving_Now`, Mech's slot 32, and three BuildingClass
sites (defence turret facing fits) — but also `ParticleSystemClass__AI_Fire`, which does not
obviously fit a *facing* reading. If the struct is a more general interpolator that
FacingClass merely instantiates, the class name is too specific. The decoded behaviour is
correct either way. UNCHECKED.

### Jumpjet's state enum at `iface+0x4c`

Decoded from the locomotor's per-frame `Process` switch (`decompile_function 0x0054aec0`),
which is the field's only writer and dispatches one handler per state. Note the `param_1`
`int*` indexing: `param_1[0x13]` is byte offset `0x4c`.

| value | meaning | handler | leaves to |
|---|---|---|---|
| 0 | on the ground, idle | `0x0054b980` | 1, when slot 4 reports a move |
| 1 | ascending / taking off | `0x0054ba30` | 2 at altitude, or 3 to translate |
| 2 | holding station at altitude | `0x0054bd30` | 3 to translate, 4 to come down |
| 3 | translating at altitude | `0x0054bff0` | 2 on arrival, or 4 |
| 4 | descending, target altitude 0 | `0x0054c550` | 0 once height reaches 0 |
| 5 | touchdown, resolving target cell | `0x0054ca90` | 6 when grounded, or 4 |
| 6 | post-landing finalise | *(no case)* | — |

Since the predicate is `s != 0 && s != 2`, **moving-now spans 1, 3, 4, 5, 6 — including a
descending jumpjet.** Not-moving is only "on the ground" and "hovering in place".

Two independent confirmations that 4 is the descent and 0 the settled ground state:

1. `0x0054c550` sets target altitude (`object+0x80`) to 0, and only once `Get_Height`
   (owner vtable `+0x1c8`) returns 0 does it call `SetSpeedFraction(0)` (owner `+0x544`), null
   out the head-to triple at `object+0x40..0x48`, and set state 0.
2. The shared altitude integrator `0x0054d0f0` tests `state != 4 && state != 0` before
   substituting a flight target height — so 4 and 0 aim at the ground — and applies the
   hover-wobble sine at `object+0x88` only when the state is 2 or 3.

All six handlers and both helpers are now named in Ghidra with these citations.

---

## UNCHECKED / UNKNOWN
- **`owner->vt[0x538]`** (slot 334) — body and slot index verified, returns a signed int. The
  name `Apparent_Speed` is a pre-existing Ghidra label, not evidence.
- **`owner+0x578`** (Walk's double) and **`owner+0x2e8`** (Fly's float, slot 4 only) —
  read exactly as fields; roles inferred from context only.
- **Teleport's `+0x30` writers** were not enumerated, so the mapping of native value 1 to a
  specific warp phase is unverified.
- **The `vtbl[0x184]` result** compared against 1/2/5/7/0xf in both gates. It is *not* the
  mission (that is read directly from `+0xac`). Identity UNKNOWN, so the exempt-category sets
  cannot be named — this is what a full port of the gate still needs.
- Gate precondition fields: `+0x2b4`, `+0x68d`, `+0x8d` (Infantry), `+0x6e1`, `+0x6e2`,
  `+0x6d1` (Unit), `+0x6dd` (Building).
- The per-sequence flag table at `0x007eaf7c`: stride 4; exact width, length and field
  meanings UNCHECKED.
- Ship slot 4's body was decompiled but its raw asm tail was not re-read, so the
  "Z load is dead" detail is VERIFIED for Drive and INFERRED for Ship.
- Parachute, Mech and Tunnel beyond what is tabulated above.
