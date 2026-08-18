> **RAW LANE OUTPUT — NOT THE AUTHORITY.** This is the unedited lane 2 (the mission readiness gate) from the
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

# LANE 2 — the mission-readiness gate and what it actually calls

Session date 2026-07-29. Program: testProsjekt / gamemd.exe, image base 0x400000.
Every address below carries the tool call that produced it. Nothing here is carried over
from prior sessions except where explicitly marked as an inherited anchor being tested.

---

## VERIFIED

### V0. Correction to an inherited anchor: the ILocomotion vtable is 50 slots, not 40

`read_memory 0x007e7eb0 length 208` (DRIVE ILocomotion vtable) decodes to:

| slot | byte off | target |
|---|---|---|
| 0 | 0x00 | 0x004b4d90 |
| 1 | 0x04 | 0x004b4da0 |
| 2 | 0x08 | 0x004b4db0 |
| 3 | 0x0c | 0x0055a710 |
| 4 | 0x10 | 0x004afb80 |
| 5 | 0x14 | 0x004afc90 |
| 6 | 0x18 | 0x004afcc0 |
| 7 | 0x1c | 0x0055abf0 |
| 8 | 0x20 | 0x0055abe0 |
| 9 | 0x24 | 0x004aff60 |
| 10 | 0x28 | 0x004b0410 |
| 11 | 0x2c | 0x0055abd0 |
| 12 | 0x30 | 0x0055a8c0 |
| 13 | 0x34 | 0x0055abc0 |
| 14 | 0x38 | 0x004b4870 |
| 15 | 0x3c | 0x004b4880 |
| 16 | 0x40 | 0x004b0500 |
| 17 | 0x44 | 0x004afd40 |
| 18 | 0x48 | 0x004afe00 |
| 19 | 0x4c | 0x004b0ef0 |
| 20 | 0x50 | 0x004b04d0 |
| 21 | 0x54 | 0x0055ab90 |
| 22 | 0x58 | 0x0055a8f0 |
| 23 | 0x5c | 0x0055a910 |
| 24 | 0x60 | 0x0055a930 |
| 25 | 0x64 | 0x0055a940 |
| 26 | 0x68 | 0x0055ab70 |
| 27 | 0x6c | 0x0055ab80 |
| 28 | 0x70 | 0x004b0c40 |
| 29 | 0x74 | 0x004b4820 |
| 30 | 0x78 | 0x0055ac00 |
| 31 | 0x7c | 0x004afb40 |
| **32** | **0x80** | **0x004afc20** |
| 33 | 0x84 | 0x0055ad10 |
| 34 | 0x88 | 0x0055acf0 |
| 35 | 0x8c | 0x0055ad00 |
| 36 | 0x90 | 0x004b4c60 |
| 37 | 0x94 | 0x004b4c70 |
| 38 | 0x98 | 0x004b4c80 |
| 39 | 0x9c | 0x004b48d0 |
| 40 | 0xa0 | 0x004b4920 |
| 41 | 0xa4 | 0x004b4b00 |
| **42** | **0xa8** | **0x004b4c50** |
| 43 | 0xac | 0x004b4c90 |
| 44 | 0xb0 | 0x004b4ca0 |
| 45 | 0xb4 | 0x004b4be0 |
| 46 | 0xb8 | 0x004b4bf0 |
| 47 | 0xbc | 0x004b4c00 |
| 48 | 0xc0 | 0x004b4c10 |
| 49 | 0xc4 | 0x004b4c20 |
| — | 0xc8 | 0x007ffeb0 = **data**, so the vtable ends at slot 49 |

Slots 4 and 7 match the inherited anchors exactly (0x004afb80, 0x0055abf0), so the base is
aligned correctly. **The "40 slots" anchor is wrong: there are 50 (0..49), and slots
40..49 are real ILocomotion methods.** This matters because slot 42 turns out to be
load-bearing (V2 below).

Slots 0/1/2 are COM QueryInterface/AddRef/Release — proven by use, not by label:
`decompile_function 0x00520f40` shows `(**(code**)*iface)(iface, &DAT_00818858, &out)` (a
riid + ppv pair) and `(**(code**)(*iface + 8))(iface)` used as a Release after the QI.

### V1. The locomotor interface pointer lives at FootClass field +0x674

`disassemble_function 0x004da530` (FootClass__AI) — every locomotor call in that function
is preceded by `CMP dword ptr [ESI + 0x674],EBX` / `PUSH 0x80004003` (E_POINTER) /
`CALL 0x007dc720` (the assert), then `MOV EAX,dword ptr [ESI + 0x674]`. ESI is `this`
(set at 0x004da537 `MOV ESI,ECX`). `decompile_function 0x00520f40` shows the same field as
`param_1[0x19d]` (0x19d*4 = 0x674). Same field, two functions. So `+0x674` is the
ILocomotion pointer.

### V2. Slot 4 and slot 32 in Drive, and slot 42 is a forwarder to slot 32

`batch_decompile 0x004afc20,0x004afb40,0x004afb80`:

- **slot 4 = 0x004afb80** (`DriveLocomotionClass__ILocomotion_Is_Moving`) — pure state read:
  returns 1 if the destination triple at `+0x30/+0x34/+0x38` differs from the null-coord
  globals; else if `+0x3c/+0x40/+0x44` is null returns 0; else returns 0 when that coord
  equals the linked object's coord at `[[this+8]+0x9c]/[+0xa0]`; else 1. **No timer, no
  speed.**
- **slot 32 = 0x004afc20** — `CDTimerClass__Remaining() != 0 → true`; otherwise
  `slot4(this) && dest(+0x3c..0x44) != null && linkedObject->vtbl[0x538]() > 0 → true`;
  else false. So slot 32 is *strictly narrower* than slot 4 plus a timer escape hatch, and
  it consults the **linked techno's current speed** via that object's own vtable +0x538.
- slot 31 = 0x004afb40 is `Force_New_Slope`-style writer, unrelated.

`disassemble_bytes 0x004b4c50 length 24` — slot 42's whole body:

```
004b4c50  MOV  EAX, dword ptr [ESP + 0x4]
004b4c54  PUSH EAX
004b4c55  MOV  ECX, dword ptr [EAX]
004b4c57  CALL dword ptr [ECX + 0x80]      ; <-- slot 32, same this, no this-adjust
004b4c5d  RET  0x4
```

**Slot 42 is a virtual re-dispatch to slot 32 on the same object.** It is not an
independent predicate. (Slot 36..38 and 43..49 at 0x004b4c60/70/80 and 0x004b4c90/a0 are
in the same little thunk block; 0x004b4c60 is `XOR EAX,EAX; RET 4` — a constant-false
stub.)

This also means the label-drift pattern flagged in the anchors is a family: the
0x004b4xxx / 0x004b6xxx range holds one-line forwarding thunks, and the ones forwarding to
`[ECX+0x10]` (slot 4) versus `[ECX+0x80]` (slot 32) are easy to mislabel.

### V3. All four `CALL [reg+0x80]` sites in FootClass__AI are genuine ILocomotion slot-32 calls

`disassemble_function 0x004da530`. Receiver proven for each — in all four the register
holding the vtable is loaded from `[ESI+0x674]`, i.e. the locomotor interface, **not**
`this`:

**Site 1 — 0x004da692**
```
004da677  CMP  dword ptr [ESI + 0x674],EBX
004da67f  PUSH 0x80004003 / CALL 0x007dc720     ; null-iface assert
004da689  MOV  EAX,dword ptr [ESI + 0x674]      ; EAX = ILocomotion*
004da68f  PUSH EAX
004da690  MOV  ECX,dword ptr [EAX]              ; ECX = locomotor vtable
004da692  CALL dword ptr [ECX + 0x80]           ; ILocomotion slot 32
004da698  TEST AL,AL ; JZ 0x004da7b0
```
Consequence when TRUE: `[self+0x54]()`, then `0x004f9a50(house, global 0x00a83d4c)`, then a
frame-stamp/duration pair at `[ESI+0x65c]/[ESI+0x664]` gates it, then `[self+0x48c]` (4
args) and `[self+0x488]` (5 args), `[self+0x1c8]` stored to `[ESI+0x264]`, and a call to
`0x00567da0` with `ECX = 0x87f7e8`. This is the **occupancy re-mark + sight/reveal refresh
while moving**, not a mission decision.

**Site 2 — 0x004da8bb** (receiver: `MOV EAX,[ESI+0x674]` @0x004da8b2, `MOV ECX,[EAX]`
@0x004da8b9). Immediately preceded, in the same basic block, by
`CALL dword ptr [ECX + 0x40]` @0x004da877 on the *same* interface pointer = **slot 16 =
0x004b0500** — the locomotor's per-frame Process step. So site 2 reads slot 32 *after*
Process has already moved the unit this tick.

**Site 3 — 0x004da96d** (receiver: `MOV EAX,[ESI+0x674]` @0x004da964).
**Site 4 — 0x004daa24** (receiver: `MOV EAX,[ESI+0x674]` @0x004daa1b).

Sites 2/3/4 feed a counter at `[ESI+0x538]` (incremented at 0x004da9fb) and, from
0x004daa32 onward, the **movement-sound start/stop**: `[ESI+0x53c]` bool, `[ESI+0x540]`
countdown set to 3 at 0x004daaee, an audio object at `[ESI+0x544]`
(ctor 0x00405d40 / dtor 0x00406060), a random pick from the type-object list at
`type+0x4f4` with count at `type+0x504` via 0x0065c780, played by 0x007509e0. So the
engine-loop/footstep sound is driven by slot 32 as well.

**None of the four is a mission-readiness gate.** They are: reveal/occupancy refresh, and
move-sound state.

### V4. FootClass__Locomotion_AI (0x00520f40) reads slot 4 twice and slot 42 (→ slot 32) once

`decompile_function 0x00520f40`:

- `(**(code **)(*(int *)param_1[0x19d] + 0x10))(iface)` → **slot 4** at two places:
  once under `if (self->vtbl[0x184]() == 2)` (bail-out / re-target path that calls
  `self->vtbl[0x484](0)` or `self->vtbl[0x480](dest,1)` + `self->vtbl[0x544](0,0x3ff00000)`),
  and once in the `Can_Reach_Zone` guard that calls `self->vtbl[0x480](0,1)` when the
  destination is unreachable.
- `cVar2 = (**(code **)(*(int *)param_1[0x19d] + 0xa8))(iface)` → **slot 42**, which V2
  proved forwards to **slot 32**.
  - `cVar2 == 0` (not moving now): selects an idle/stopped sequence via
    `self->vtbl[0x558](k,0,0)` with k = 0 / 2 / 0x10 chosen from `param_1[0x1b1]`
    (offset 0x6c4).
  - `cVar2 != 0` (moving now): selects the *locomotion* sequence — `0x17`/`0x18` when a
    QI'd sub-interface reports a specific 16-byte state equal to `DAT_007e9ac0` and
    `*(double*)(param_1+0x15e) <= 0.8` decides between them, otherwise `3` (or `6` when
    `*(char*)((int)param_1 + 0x6db) != 0`).

So the **animation/sequence selection** is the other live consumer of the moving-now
predicate, reached through slot 42 rather than slot 32 directly.

### V5. MissionClass::Mission_Dispatch does NOT consult the locomotor at all

`decompile_function 0x005b3060` (`MissionClass__Mission_Dispatch`). The whole body:

1. `ObjectClass__AI()`
2. `if ((char)param_1[0x24] == 0) return;`  (byte at +0x90 — the same "still alive/active"
   byte FootClass__AI tests at `[ESI+0x90]`)
3. mission timer: `iVar2 = param_1[0x34]` (+0xd0); if `param_1[0x32]` (+0xc8) != -1 then
   `elapsed = g_CurrentFrameCounter - param_1[0x32]`, and if `iVar2 <= elapsed` fall
   through, else `iVar2 -= elapsed`; **`if (iVar2 != 0) return;`**
4. `if (0 < param_1[0x1b])` — a positive count at +0x6c
5. `switch (param_1[0x2b])` (+0xac = current mission) → one virtual per mission, and the
   returned delay is stored back into +0xd0 with +0xc8 = current frame.

There is **no `CALL [reg+0x80]`, no `[+0x674]` load, and no locomotor call of any kind** in
0x005b3060. Confirmed structurally by the exhaustive enumeration in V6: no slot-0x80
callsite falls inside 0x005b3060–0x005b3570.

**The only thing that defers a mission in the dispatcher is the frame-counted mission timer
at +0xc8/+0xd0.** Not "am I moving".

### V6. Exhaustive enumeration of every `CALL dword ptr [reg + 0x80]` in the binary

`search_instructions mnemonic=CALL operand_pattern="0x80]" limit=300` →
**match_count 181, truncated=false, instructions_scanned 1152218**. That is the complete
set. After discarding the DirectDraw/Surface/GadgetClass/OwnerDraw/Bink/VQA/Pipe/Straw
GUI-vtable hits, the game-object candidates are exactly:

```
0041816b AircraftClass__Mission_Attack        [EDX+0x80]
0041b965 AircraftClass__Is_Firing_Possible    [EAX+0x80]
0041b9c5 AircraftClass__Is_Weapon_Ready       [EAX+0x80]
004a357b FUN_004a33a0                         [ECX+0x80]
004b078f DriveLocomotionClass__Process        [EAX+0x80]
004b4c57 (slot-42 thunk, V2)                  [ECX+0x80]
004bafc3 FUN_004baf40                         [ECX+0x80]
004cd644 FlyLocomotionClass__Process          [EDX+0x80]
004da692 FootClass__AI                        [ECX+0x80]
004da8bb FootClass__AI                        [ECX+0x80]
004da96d FootClass__AI                        [ECX+0x80]
004daa24 FootClass__AI                        [ECX+0x80]
004dbddd FootClass__IsCloakable               [ECX+0x80]
00514a24 HoverLocomotionClass__Move           [EDX+0x80]
00514e66 (no function defined)                [ECX+0x80]
00519998 InfantryClass__PerCellProcess        [EDX+0x80]
0051cab5 (no function defined)                [ECX+0x80]
00521bb0 FUN_00521b60                         [ECX+0x80]
0069fe3c ShipLocomotionClass__Process         [ECX+0x80]
006b7349 SpawnManagerClass__AI                [EDX+0x80]
006f7fd5 TechnoClass__Evaluate_Candidate      [EAX+0x80]
006f8242 TechnoClass__Evaluate_Candidate      [EAX+0x80]
006f85be TechnoClass__Evaluate_Candidate      [EDX+0x80]
006fc4dd TechnoClass__GetFireError            [EDX+0x80]
0071b08b TemporalClass__InitiateWarp          [EDX+0x80]
0073960a UnitClass__Deploy                    [ECX+0x80]
0073d114 UnitClass__DrawExtras                [ECX+0x80]
00741117 (no function defined)                [EDX+0x80]
007411c0 (no function defined)                [ECX+0x80]
007442df UnitClass__ShouldIdle                [ECX+0x80]
00692865 DisplayClass__DetermineAction        [EAX+0x80]
00692a10 DisplayClass__DetermineAction        [EAX+0x80]
00692bba FUN_00692b60                         [EAX+0x80]
```

Receiver classification for each of these is below.

---

## VERIFIED (continued) — THE GATE

### V7. The mission-readiness gate is a virtual on the object's OWN vtable at offset 0x200

`decompile_function 0x005b35e0` (`MissionClass__Queue_Mission(this, mission, char commence_now)`):

```c
if (commence_now != 0) {
    cVar2 = (**(code **)(*param_1 + 0x200))();   // self vtable +0x200  = "is ready to commence"
    if (cVar2 != 0) {
        (**(code **)(*param_1 + 0x1ec))();       // self vtable +0x1ec  = Commence()
    }
}
```

Offset 0x1ec is proven to be `MissionClass::Commence` and 0x200 the readiness predicate by
vtable arithmetic against a base found from a constructor store:

- `search_instructions mnemonic=MOV operand_pattern="0x7eb"` →
  `00517acc InfantryClass__Constructor: MOV dword ptr [ESI], 0x7eb058`, so the **InfantryClass
  primary vtable base is 0x007eb058**.
- `read_memory 0x007eb238 length 40` gives, at that base: `+0x1e8` = 0x005b35e0
  (`Queue_Mission`), `+0x1ec` = 0x005b3570 (`Commence`), `+0x1f0` = 0x005b2fd0
  (`Assign_Mission`), `+0x1fc` = 0x005b3a10 (`Is_Mission_Suspended`), **`+0x200` = 0x00521b60**,
  `+0x204` = 0x005b2e10 (`Mission_Default`).
- Independent cross-check on the base: `decompile_function 0x005b3060`
  (`MissionClass__Mission_Dispatch`) uses `self->vtbl[0x204]` for `case 0`/`default` of the
  mission switch, and 0x007eb058+0x204 = 0x007eb25c = `MissionClass__Mission_Default`. The
  base is right.

**Every implementation of the 0x200 slot (verified by reading base+0x200 in each vtable
that also holds a `Commence` xref from `get_xrefs_to 0x005b3570`):**

| impl | vtable slot read | what it consults |
|---|---|---|
| 0x004e0140 (shared by 4 vtables: base+0x200 at 0x007e8e94, 0x007edec0, 0x007f0708, 0x007f4b60) | `read_memory` each | `decompile_function 0x004e0140` → **`return 1;`** unconditionally. The abstract-base default. |
| **0x00521b60** — InfantryClass (0x007eb258) | `read_memory 0x007eb238` | **ILocomotion slot 32** |
| **0x00744270** — UnitClass (0x007f5e70) | `read_memory 0x007f5e70` | **ILocomotion slot 32** |
| 0x0041b5e0 — AircraftClass (0x007e24a4) | `read_memory 0x007e2488`, `get_xrefs_to 0x0041b5e0` | **no locomotor call at all** |
| 0x00454250 — BuildingClass (0x007e40bc) | `read_memory 0x007e40bc` | `decompile_function 0x00454250` → `return this->byte[+0x6dd] != 0;` — no locomotor |

Slot alignment between the Aircraft and Infantry vtables was cross-checked on three shared
entries: 0x00705d70 at 0x007e2488/0x007eb23c, 0x004d8f80 at 0x007e249c/0x007eb250,
0x005b3a10 at 0x007e24a0/0x007eb254 — all a constant 0xCDB4 apart, so 0x007e24a4 and
0x007eb258 are the same slot.

### V8. The Infantry implementation (0x00521b60) — exact deferral condition

`decompile_function 0x00521b60`. `param_1[0x2b]` = +0xac = current mission,
`param_1[0x19d]` = +0x674 = locomotor, `param_1[0x1b1]` = +0x6c4 = current sequence/DoType,
`param_1[0xad]` = +0x2b4.

```
ready = false
if  mission != 6  and  mission != 0x15
and byte[+0x68d] == 0  and  byte[+0x8d] == 0:
        assert locomotor != null                      // PUSH 0x80004003
        moving_now = locomotor->vtbl[0x80]()          // <<< ILocomotion SLOT 32
        if moving_now:
              k = self->vtbl[0x184]()
              if k != 5 and k != 0xf:
                    if k != 1:            return false     // DEFER
                    if self[+0x2b4] != 0: return false     // DEFER
        seq = self[+0x6c4]
        if seq == -1 or ((byte*)0x007eaf7c)[seq*4] != 0:
              ready = true
return ready
```

So for infantry the gate **defers the mission whenever ILocomotion slot 32 says "moving
now"**, except for two `vtbl[0x184]` categories (5 and 0xf) that are exempt outright and a
third (1) that is exempt only if `+0x2b4` is non-zero. It *additionally* defers while the
current animation sequence's flag byte in the table at 0x007eaf7c is zero (a
"sequence not interruptible" table; `read_memory 0x007eafe0 length 64` shows 4-byte records
whose first byte is 0/1/3/4/6).

### V9. The Unit implementation (0x00744270, labeled `UnitClass__ShouldIdle`) — same slot-32 read

`decompile_function 0x00744270`:

```
if  mission != 6 and mission != 0x15
and byte[+0x6e1] == 0 and byte[+0x6e2] == 0 and byte[+0x6d1] == 0:
      if self[+0xb4] != 7:                            // queued mission != 7
            assert locomotor != null
            moving_now = locomotor->vtbl[0x80]()      // <<< ILocomotion SLOT 32
            if moving_now
               and self->vtbl[0x1c8]() >= 0
               and self->vtbl[0x184]() != 5
               and (self->vtbl[0x184]() != 1 or self[+0x2b4] != 0)
               and self[+0xb8] == 0:
                     return false                      // DEFER
      ... a further deploy/bridge-repair-hut proximity check ...
      return 1 / 0
return false
```

Same shape as infantry: **moving-now (slot 32) is the primary deferral input.** The label
`UnitClass__ShouldIdle` is a poor/drifted name — the function *is* the UnitClass override of
the readiness virtual (it occupies self-vtable slot 0x200, `read_memory 0x007f5e70`).

### V10. Ordering — the predicate is evaluated LIVE at the gate, and the gate runs TWICE per tick

`get_xrefs_to 0x004da530` (FootClass__AI) → non-virtual base calls from exactly three
places: `0051bc9f in InfantryClass__AI`, `0073647b in UnitClass__AI`,
`00414da3 in AircraftClass__AI`.

`search_instructions CALL "0x200]"` (28 hits, not truncated) and
`search_instructions CALL "0x1ec]"` (60 hits, not truncated) pin the checkpoints:

**InfantryClass__AI**
```
0x0051bc1c  gate  (0x200)      ─┐  BEFORE FootClass::AI
0x0051bc51  Commence (0x1ec)   ─┘
0x0051bc9f  CALL 0x004da530  = FootClass::AI  (locomotor Process runs inside, at 0x004da877)
0x0051bed1  gate  (0x200)     ─┐  AFTER FootClass::AI
0x0051bf03  Commence (0x1ec)  ─┘
```
`disassemble_bytes 0x0051bbf0 length 190` gives the exact first checkpoint:
```
0051bc18  MOV  EAX,dword ptr [ESI]          ; ESI = this; EAX = this's OWN vtable
0051bc1a  MOV  ECX,ESI                      ; thiscall this = self
0051bc1c  CALL dword ptr [EAX + 0x200]      ; readiness virtual
0051bc25  TEST AL,AL
0051bc27  JZ   0x0051bc57                   ; not ready -> skip Commence entirely
0051bc2d  CALL dword ptr [EDX + 0x184]
0051bc37  CMP  dword ptr [ESI + 0xb4],EBP   ; queued mission == -1 ?
0051bc47  CALL dword ptr [EAX + 0x484]
0051bc51  CALL dword ptr [EDX + 0x1ec]      ; Commence()
```

**UnitClass__AI**: gate 0x00736465 → Commence 0x00736473 → `CALL 0x004da530` at 0x0073647b;
second gate 0x007366ef → Commence 0x007366fd.

**AircraftClass__AI**: `CALL 0x004da530` at 0x00414da3 comes *first*, gate at 0x0041504a →
Commence 0x00415058.

**Answer to the ordering question:** the moving predicate is evaluated **on demand, inside
the same per-object dispatch, at the instant the gate runs** — the gate is a virtual call
that itself performs the `CALL [locomotor_vtbl+0x80]` with a fresh null-check assert. There
is no cached per-frame "is moving" byte anywhere in this path. And because the gate runs
once *before* and once *after* `FootClass::AI` (which is where the locomotor's `Process`
executes, slot 16 = 0x004b0500 for Drive, called at 0x004da877), the predicate can and does
return different values within a single tick for the same unit. **A Rust port must derive
readiness on demand at each gate call, not precompute it once per tick.**

Second Infantry checkpoint verified the same way, `disassemble_bytes 0x0051bec8 length 20`:
```
0051becd  MOV  EAX,dword ptr [ESI]
0051becf  MOV  ECX,ESI
0051bed1  CALL dword ptr [EAX + 0x200]
0051bed7  TEST AL,AL ; JZ 0x0051bf09
```

### V11. Object layout correction â€” ILocomotion is the object's SECOND vptr, at +0x4

This matters for reading every locomotor-internal call site, so it is proved from a
constructor rather than assumed.

`disassemble_function 0x00661ec0` (`RocketLocomotionClass__Constructor`) ends with:
```
00661f19  MOV dword ptr [ESI],       0x7f0be8  ; vptr[0] -> small IUnknown/IPersist-style vtable
00661f1f  MOV dword ptr [ESI + 0x4], 0x7f0b1c  ; vptr[1] -> ILocomotion vtable
```
`disassemble_bytes 0x004cca00` (`FlyLocomotionClass__Constructor`) does the same:
```
004cca0c  MOV dword ptr [ESI],       0x7e8ac0
004cca12  MOV dword ptr [ESI + 0x4], 0x7e89f4  ; = the Fly ILocomotion base from the anchors
```
`read_memory 0x007e8ac0 length 144` shows the +0x0 vtable is only **10 slots** (0..9, then a
`double 0.05` at 0x007e8ae8) â€” an IPersistStream-shaped interface. It has no slot 32.

Consequences:
- An ILocomotion `this` equals `objectBase + 4`. The linked techno sits at
  `objectBase + 0xC` = `ILocThis + 0x8`, which reconciles Drive slot 4's `*(this+8)` read
  with the Fly code's `[ESI+0xc]` reads.
- **Mechanical test for classifying a `CALL [reg+0x80]`:** ILocomotion is COM-style â€”
  callers `PUSH` the interface pointer and the callees `RET 0x4`. A site that instead does
  `MOV ECX,<obj>` (register `__thiscall`) and calls `[vtbl+0x80]` is a native C++ virtual on
  the object's own vtable and has nothing to do with locomotion.

### V12. Fly and Rocket ILocomotion vtables â€” full slot check

`read_memory 0x007e89f4 length 216` (FLY): slot 4 = 0x004cca90, slot 7 = 0x0055abf0 (base
alignment OK), slot 16 = 0x004ccb40, **slot 32 = 0x004ccac0**, slot 42 = 0x004b4c50, table
ends at slot 49 (0x007e8abc holds 0x008006b8 = data). Confirms the inherited Fly anchors and
confirms 50 slots is not Drive-specific.

`read_memory 0x007f0b1c length 176` (ROCKET, base from the constructor above): slot 4 =
0x00661f50, slot 7 = 0x0055abf0 (alignment OK), slot 16 = 0x006622c0,
**slot 32 = 0x00661f90**, slot 42 = 0x004b4c50.

**Slot 42 is the identical shared thunk 0x004b4c50 in Drive, Fly and Rocket** â€” so the
"slot 42 to slot 32" indirection used by `FootClass__Locomotion_AI` resolves to the family's
own slot-32 override for every family checked.

`disassemble_bytes 0x00661f50 length 96` gives both Rocket predicates outright:
- **Rocket slot 4** = `(this[+0x14],[+0x18],[+0x1c]) != (globals 0x00b04e38/3c/40)` â€” the
  usual null-coord destination test.
- **Rocket slot 32** = `phase = this->int[+0x3c]; return 3 <= phase && phase <= 5;` â€” a real
  per-family override reading the ballistic flight phase, not a stub and not inherited.

### V13. Receiver classification for every game-relevant `CALL [reg+0x80]` site

**GENUINE ILocomotion slot 32** (receiver is the locomotor interface â€” either
`[techno+0x674]` or, inside a locomotor body, the ILocomotion `this` from the stack):

| site | how the receiver was proved |
|---|---|
| 0x004da692 / 0x004da8bb / 0x004da96d / 0x004daa24 `FootClass__AI` | `[ESI+0x674]`, `disassemble_function 0x004da530` |
| 0x004b4c57 (slot-42 thunk) | `[[ESP+4]]`, `disassemble_bytes 0x004b4c50` |
| 0x004dbddd `FootClass__IsCloakable` | `param_1[0x19d]`, `decompile_function 0x004dbddd` |
| 0x00521bb0 (in 0x00521b60, the Infantry readiness impl) | `param_1[0x19d]`, `decompile_function 0x00521b60` |
| 0x0041b965 `AircraftClass__Is_Firing_Possible` | `*(int**)(param_1+0x674)`, `decompile_function 0x0041b965` |
| 0x0041b9c5 `AircraftClass__Is_Weapon_Ready` | same, `decompile_function 0x0041b9c5` |
| 0x0041816b `AircraftClass__Mission_Attack` | `[ESI+0x674]` + the 0x80004003 assert, `disassemble_bytes 0x00418150` |
| 0x0051cab5 (undefined function, InfantryClass region) | `[EBX+0x674]` + assert, `disassemble_bytes 0x0051ca90` |
| 0x007411c0 (undefined function, UnitClass region) | `[ESI+0x674]` + assert, `disassemble_bytes 0x007411a8` |
| 0x007442df (inside `UnitClass__ShouldIdle` @0x00744270, the Unit readiness impl) | `param_1[0x19d]`, `decompile_function 0x00744270` |
| 0x004b078f `DriveLocomotionClass__Process` | `MOV EAX,[ESI]; PUSH ESI` where ESI is the ILoc `this` (writes `[ESI+0x5e]`), `disassemble_bytes 0x004b0770` â€” a **self**-dispatch, so a derived override wins |
| 0x004cd644 (Fly region, in the function starting 0x004cd600) | `MOV EDX,[ESI+4]; LEA EAX,[ESI+4]; PUSH EAX` = the adjusted-this for the ILocomotion subobject, `disassemble_bytes 0x004cd5b0` + V11 |
| 0x00514a24 `HoverLocomotionClass__Move` | `MOV EDX,[ESI]; PUSH ESI`, ESI is the ILoc `this` (`[ESI+8]` = linked techno), `disassemble_bytes 0x00514a00` |
| 0x00514e66 (undefined, Hover region) | same pattern, `disassemble_bytes 0x00514dc0` |

**NOT ILocomotion â€” a slot on some other class's own vtable:**

| site | why |
|---|---|
| 0x00519998 `InfantryClass__PerCellProcess` | `decompile_function 0x00519998`: the receiver is `piVar10 = param_1[0x169]` (the techno's link/destination object at +0x5a4), and the same object is then asked `[+0x2c]` (the RTTI/What-Am-I virtual) and compared to 6 (building). It is a game object, not a locomotor. (This function *does* read the locomotor, but at **slot 4** â€” `param_1[0x19d]+0x10` â€” for its drown/impassable check.) |
| 0x00741117 (UnitClass region) | `MOV EDX,[EDI]; MOV ECX,EDI; CALL [EDX+0x80]` â€” register `__thiscall`, and EDI comes from `0x0040dd20(EBP)`. Native C++ virtual, not the COM interface. `disassemble_bytes 0x007410f8` |
| all DirectDraw / `Surface` / `GadgetClass` / `OwnerDraw_*` / Bink / VQA / `Pipe` / `Straw` hits in the V6 list | unrelated GUI/surface vtables |

**UNCHECKED receivers** (in the V6 list, not examined this session): 0x004a357b, 0x004bafc3,
0x0069fe3c `ShipLocomotionClass__Process`, 0x006b7349 `SpawnManagerClass__AI`,
0x006f7fd5 / 0x006f8242 / 0x006f85be `TechnoClass__Evaluate_Candidate`,
0x006fc4dd `TechnoClass__GetFireError`, 0x0071b08b `TemporalClass__InitiateWarp`,
0x0073960a `UnitClass__Deploy`, 0x0073d114 `UnitClass__DrawExtras`,
0x00692865 / 0x00692a10 `DisplayClass__DetermineAction`, 0x00692bba.

### V14. Coverage â€” do AIRCRAFT and ROCKET objects reach the gate?

**They reach it; the aircraft override just never asks the locomotor.**

- `search_instructions CALL "0x200]"` puts a gate call at **0x0041504a inside
  `AircraftClass__AI`**, with `Commence` at 0x00415058. So the gate *is* invoked for every
  aircraft every tick.
- The implementation it lands on is `AircraftClass__Is_Ready` @ 0x0041b5e0
  (`read_memory 0x007e2488` puts it at slot 0x200 of the Aircraft vtable).
  `decompile_function 0x0041b5e0`:
  ```c
  m = this->int[+0xac];                       // current mission
  if (m != 6 && m != 0x15 && (this->byte[+0x6d2] == 0 || m == 0x1e))
        return this->byte[+0x6d4] != 0;
  return 0;
  ```
  **No `+0x674` load, no locomotor call of any kind.**
- Rocket-locomotor objects are **AircraftTypes**, not vehicles: `ini/rulesmd.ini` lines
  1163 / 1165 / 1171 list `V3ROCKET`, `DMISL`, `CMISL` under `[AircraftTypes]` (section header
  at line 1159), and each carries
  `Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}` (lines 11389, 11429). So they are
  `AircraftClass` objects and take the *same* aircraft readiness path.

**Verdict on the prior Rust comment "Fly, Rocket have no readiness slot that this gate
consults":**

- Claim (a) *"no slot-32 override exists"* â€” **REFUTED.** Fly slot 32 = 0x004ccac0 and
  Rocket slot 32 = 0x00661f90 are both real per-family overrides with real bodies (see V12
  and the inherited Fly-body anchor).
- Claim (b) *"the gate never asks them"* â€” **REFUTED as literally worded, correct in
  effect.** The gate is invoked on aircraft at 0x0041504a; it is the AircraftClass *override*
  that never consults the locomotor. The accurate statement is: for any Fly- or
  Rocket-locomotor object the readiness answer is independent of the locomotor's moving
  predicate, because AircraftClass overrides the readiness virtual with a
  mission-plus-two-flags test.
- And slot 32 is **not dead** for those families. It is read every tick by
  `FootClass__AI` (which `AircraftClass__AI` calls at 0x00414da3) for the sight/occupancy
  refresh and the move-sound state, by `AircraftClass__Mission_Attack` @0x0041816b, by
  `AircraftClass__Is_Firing_Possible` @0x0041b965 and `AircraftClass__Is_Weapon_Ready`
  @0x0041b9c5 (these two only when `this->[+0x6c4]` matches one of two `RulesClass` fields â€”
  semantics of that gate UNCHECKED), and by the Rocket locomotor's own `Process`
  self-dispatch at 0x00662d77.

---

## INFERRED (reasoning beyond what a tool call directly showed)

- The four vtables whose slot 0x200 is the `return 1` stub 0x004e0140 are most likely the
  vftables of the abstract intermediates (`MissionClass`, `RadioClass`, `TechnoClass`,
  `FootClass`); one of them, base 0x007e8c94, has `FootClass__AI` at slot 0x5c
  (`get_xrefs_to 0x004da530` returns DATA 0x007e8cf0). Those objects are never instantiated
  in a skirmish, so the effective set of readiness implementations in play is
  {Aircraft, Building, Infantry, Unit}. **INFERRED â€” I did not enumerate the constructors
  that store those four bases.**
- Self-vtable offset 0x5c is probably the per-object `AI()` / `Update()` virtual, from the
  same xref. INFERRED.
- Slot 16 (0x40) of ILocomotion is the per-frame `Process` step: `FootClass__AI` calls it at
  0x004da877 on `[ESI+0x674]`, Drive's slot 16 is 0x004b0500 and the known
  `DriveLocomotionClass__Process` call site 0x004b078f lies inside that body, and Fly's slot
  16 is 0x004ccb40. Consistent but I did not disassemble each entry.
- The `[ESI+0x538]` counter and `[ESI+0x53c]` / `[+0x540]` / `[+0x544]` block in
  `FootClass__AI` is the movement-sound loop; identification rests on the type-object list at
  `type+0x4f4` with a count at `type+0x504`, a random index, and a play-at-coord call.
  INFERRED from shape.

## UNCHECKED / UNKNOWN

- Meaning of the `self->vtbl[0x184]()` result compared against 1 / 2 / 5 / 7 / 0xf in
  `FootClass__Locomotion_AI`, 0x00521b60 and 0x00744270. It is **not** the mission (that is
  read directly from `+0xac`). Its identity is UNKNOWN, and the exemption set {5, 0xf} in the
  Infantry gate therefore cannot be named yet. **This is the one remaining gap that a Rust
  port of the gate needs closed** â€” without it you cannot know which unit categories are
  exempt from the moving-now deferral.
- Field `+0x2b4` (the secondary exemption in both Infantry and Unit gates). UNKNOWN.
- Fields `+0x68d`, `+0x8d` (Infantry gate preconditions) and `+0x6e1`, `+0x6e2`, `+0x6d1`
  (Unit gate preconditions), and `+0x6dd` (Building readiness). UNKNOWN.
- The per-sequence flag table at 0x007eaf7c: record stride 4, first byte observed in
  {0,1,3,4,6} (`read_memory 0x007eafe0 length 64`). Its exact width, length and field
  meanings are UNCHECKED.
- `RulesClass` fields +0x4e0 and +0x514, and the techno field `+0x6c4` they are compared
  against in the two aircraft weapon predicates. `+0x6c4` reads as the current
  sequence/DoType elsewhere (`InfantryClass__AI` compares it to 0xb..0xf, 0x14, 0x15,
  0x22..0x24), which does not obviously fit a comparison against Rules pointers â€” flagging
  the tension rather than resolving it. UNCHECKED.
- Whether `Walk`, `Hover`, `Ship`, `Jumpjet`, `Teleport`, `Tunnel`, `Mech`, `DropPod` slot-32
  entries are overrides or inherited base bodies â€” Lane 1 territory, not read here. The one
  thing this lane establishes is that whatever those slot-32 bodies are, they are what the
  Infantry / Unit gate reads.
- The remaining `CALL [reg+0x80]` receivers listed as UNCHECKED in V13.
- Whether a mission handler ever *sets* the mission timer (`+0xc8` / `+0xd0`) as a substitute
  for the readiness deferral. Not examined.

## LABEL DRIFT FOUND

1. **`UnitClass__ShouldIdle` @ 0x00744270 is misnamed.** It occupies self-vtable slot 0x200
   (`read_memory 0x007f5e70`), the same slot as `AircraftClass__Is_Ready`, and it is the
   function `MissionClass__Queue_Mission` consults before calling `Commence`. It is the
   UnitClass *readiness-to-commence* override, not an idle-selection helper.
2. **`FUN_00521b60` is unnamed but is the InfantryClass readiness override** (slot 0x200 at
   0x007eb258). Worth naming.
3. **`AircraftClass__Override_Mission` @ 0x0041b870 sits in the `Commence` slot.** Slot
   alignment (0x007e2490 corresponds to 0x007eb244 = `MissionClass__Commence`, with three
   shared entries confirming a constant 0xCDB4 offset between the two vtables) shows
   0x0041b870 is the AircraftClass override of **Commence**, and `get_function_callers
   0x005b3570` shows it calls the base `MissionClass__Commence` at 0x0041b880 â€” exactly what
   an override does. Likewise 0x0041ba90 sits in the `Queue_Mission` slot and 0x0041b9f0 in
   the `Assign_Mission` slot. The `Override_Mission` name is drift.
4. **The inherited anchor "ILocomotion vtables are 40 slots" is wrong** â€” they are 50
   (0..49), verified independently for Drive (`read_memory 0x007e7eb0`), Fly
   (`read_memory 0x007e89f4`) and Rocket (`read_memory 0x007f0b1c`). Slots 40..49 are real
   methods, and slot 42 is load-bearing.
5. **`FlyLocomotionClass__Process`'s Ghidra function body appears to over-reach.** A clean
   thiscall prologue exists at 0x004cd600 (`PUSH EBP; MOV EBP,ESP; AND ESP,-8; SUB ESP,0x58;
   ... MOV ESI,ECX`), yet `search_instructions` attributes 0x004cd644 to
   `FlyLocomotionClass__Process`. Treat the boundary as unreliable; the *receiver* at
   0x004cd644 is still proven to be the ILocomotion subobject.
6. Minor: 0x004b4c60 (Drive/Fly/Rocket slot 36) is `XOR EAX,EAX; RET 4` â€” a constant-false
   stub, and 0x004b4c50/70/80/90/a0 are a block of one-line ILocomotion forwarding thunks.
   Anything in 0x004b4xxx / 0x004b6xxx named after a specific predicate should be re-checked
   against the `[ECX+offset]` it actually dispatches to â€” the same trap that produced the
   already-known `ILocomotion__Is_Moving_Now_Thunk @ 0x004b6610` misnaming.

---

## Bottom line for `src/sim/movement/ready_producer.rs`

1. The gate is **not** in `FootClass__AI` and **not** in `MissionClass::Mission_Dispatch`.
   It is the object's own virtual at self-vtable offset **0x200**, consulted by
   `MissionClass::Queue_Mission` (0x005b35e0) and again at two points per tick inside
   `InfantryClass__AI` / `UnitClass__AI` / `AircraftClass__AI`, and it calls **Commence
   (offset 0x1ec)** only when that virtual returns true.
2. The gate reads **ILocomotion slot 32**, never slot 4 â€” for Infantry (0x00521b60) and Unit
   (0x00744270). Slot 4 is read elsewhere (`FootClass__Locomotion_AI`,
   `InfantryClass__PerCellProcess`) for different decisions, and slot 42 is a thunk that
   redirects to slot 32.
3. **Per-family shape belongs on slot 32, not slot 4.** Any doc comment in
   `ready_producer.rs` that describes a family's readiness predicate using the slot-4 body is
   wrong for this gate. Drive is the clearest case: slot 4 is a pure destination-coord test,
   while slot 32 is `timer_remaining || (slot4 && has_dest && linked_speed > 0)` â€” those
   disagree exactly when a unit has a destination but zero current speed, which is the
   blocked/queued case that would strand a mission.
4. Derive readiness **on demand**; do not cache it per tick.
5. For Fly and Rocket families, readiness must not depend on the locomotor at all â€” model
   the aircraft override (mission + two flags). But keep their slot-32 predicates: sight,
   occupancy, move-sound and the aircraft attack/weapon predicates all read them.

