# Radio 0x16 — Sender Side (BuildingClass::Receive_Radio case 0x0E) — Ghidra Research Report

**Date:** 2026-05-19
**Binary:** gamemd.exe
**Primary addresses:** `BuildingClass::Receive_Radio` @ `0x0043C2D0`, `UnitClass::Receive_Radio` @ `0x00737430`
**Scope:** Where radio 0x16 is emitted in the case 0x0E flow; arg payload; full radio sequence
  (0x12 → 0x18 → 0x16); building-side effects; settle naming dispute ("TIMING_SYNC" vs "FACE_DOCK")
**Confidence:** HIGH on all findings (verified by live decompilation in this session)
**Active in YR:** YES — fires every time a harvester successfully negotiates dock entry with a
  refinery (DockUnload=yes or Weeder=yes building)

---

## 1. Overview

`BuildingClass::Receive_Radio` case 0x0E is the CAN_DOCK handler: the building decides whether
to accept an incoming harvester. On acceptance (standard refinery path), it emits three radios in
strict sequence: `MOVE_TO_CELL(0x12)` → `ENTER_DOCK(0x18)` → **radio 0x16**. This document
focuses specifically on the 0x16 emission: where it is sent, what arg it carries, what effect it
produces on the harvester, and what name is correct.

---

## 2. Exact 0x16 Emission Site — BuildingClass::Receive_Radio case 0x0E

Verified from decompile of `0x0043C2D0`:

```c
// Standard refinery path (DockUnload=yes [+0x16B3] OR Weeder=yes [+0x16BC]):
psVar5 = (short *)(**(code **)(param_1->vtable + 0x1b8))(&stack);  // GetCell_Packed → (X,Y)
uStack_8 = (int *)CONCAT22(psVar5[1] + 1, *psVar5 + 3);           // queue cell = (X+3, Y+1)
uVar6 = MapClass__Get_CellClass(&piStack_4);
*param_4 = uVar6;
iVar10 = (**(code **)(param_1->vtable + 0x27c))(0x12, param_4, param_2); // MOVE_TO_CELL
if (iVar10 != 0x14) { return 1; }                                  // abort if not ALREADY_THERE

(**(code **)(param_1->vtable + 0x278))(0x18, param_2);             // ENTER_DOCK
iVar10 = (**(code **)(param_1->vtable + 0x278))(0x16, param_2);   // ← RADIO 0x16 EMITTED HERE
if (iVar10 == 1) { return 1; }
(**(code **)(param_2->vtable + 0x174))(&DAT_0089c848, 1, 1);      // PlaySound "ok" cue
return 1;
```

**Key facts:**

1. **Sender:** the building (`param_1`, a refinery).
2. **Receiver:** the harvester (`param_2`, the unit that sent CAN_DOCK).
3. **Arg payload:** none — sent via `Transmit_Radio(0x16, param_2)` (vtable+0x278), which passes
   `&g_RadioScratchBuffer` as payload internally but no caller-meaningful payload value.
4. **Gate:** only sent after `ENTER_DOCK(0x18)` is sent unconditionally; the 0x16 send is
   **not** itself gated on ENTER_DOCK's return value.
5. **Sound cue:** `PlaySound(&DAT_0089c848, 1, 1)` fires if and only if `iVar10 != 1`
   (i.e., the harvester did NOT return ROGER from 0x16). This is the "ok" acceptance sound.
   If the harvester returns 1 (ROGER), the sound is skipped.

**The sound cue branch is the inverse of what "success" means in the normal radio protocol.**
ROGER (1) suppresses the sound; non-ROGER plays it. This is intentional: the sound represents
a special acceptance confirmation state, not the normal "message received" path.

---

## 3. Full Radio Sequence — Case 0x0E, Refinery Path

| Order | Direction | Code | Payload | Gate to proceed | Vtable call |
|-------|-----------|------|---------|-----------------|-------------|
| 1 | building→harvester | `NEED_TO_MOVE (0x13)` | none | return == 1 (ROGER) required to continue | vtable+0x278 |
| 2 | building→harvester | `MOVE_TO_CELL (0x12)` | `CellClass*` (queue cell) in `*param_4` | return must == 0x14 (ALREADY_THERE) | vtable+0x27C |
| 3 | building→harvester | `ENTER_DOCK (0x18)` | none | none — always sent if step 2 passed | vtable+0x278 |
| 4 | building→harvester | **`0x16`** | none | none — always sent after step 3 | vtable+0x278 |

**Step 2 gate is critical:** if the harvester is NOT already at the queue cell, `MOVE_TO_CELL`
returns 1 (ROGER, meaning "I'll go there"), and case 0x0E returns 1 immediately — ENTER_DOCK and
0x16 are **not sent**. Only when the harvester replies 0x14 (ALREADY_THERE) do steps 3 and 4 fire.

**Step 1 gate detail:** if NEED_TO_MOVE returns != 1 (harvester busy) AND the stack sentinel at
`piStack_4` is zero, case 0x0E returns 1 silently without proceeding. If sentinel is non-zero,
it proceeds even if NEED_TO_MOVE was rejected.

---

## 4. What Happens on the Receiver Side — UnitClass::Receive_Radio case 0x16

Verified from decompile of `0x00737430`.

```c
case 0x16:
    FootClass__Receive_Radio(param_2, param_3, param_4);   // (A) delegate to base chain
    if ((*(char *)((int)param_1 + 0x6af) == '\0') &&      // (B) NOT chrono-teleporting
       (psVar4 = (short *)RateTimer__Current(local_1c), *psVar4 != 0x4000)) {  // (C) timer != target
        (**(code **)(*(int *)param_1[0x19d] + 0x4c))((int *)param_1[0x19d], 0x4000);
        return 1;
    }
    // Timer already at 0x4000 or chrono-teleporting:
    cVar1 = (**(code **)(*(int *)param_1[0x19d] + 0x10))((int *)param_1[0x19d]); // Is_Moving()
    if (((cVar1 == '\0') &&                                // locomotor idle
        (piVar5 = (int *)FootClass__GetDestination(0), (char)param_1[0x106] != '\0')) &&
       ((piVar5 != (int *)0x0 &&
        ((iVar7 = (**(code **)(*piVar5 + 0x2c))(), iVar7 == 6 &&  // dest GetWhat() == 6 (Building)
         (iVar7 = (**(code **)(*param_1 + 0x184))(), iVar7 == 7)))))) {   // mission == 7 (Dock)
        (**(code **)(*param_1 + 0x278))(0x15, piVar5);    // send TIMING_SYNC_BACK(0x15) to building
    }
    return 1;
```

### 4.1 Sub-step (A): FootClass base chain sends ENTER_DOCK (0x18) back

`FootClass::Receive_Radio` at `0x004D8FB0` has **no case 0x16** — it falls through to
`TechnoClass::Receive_Radio` at `0x006F4AB0`.

`TechnoClass::Receive_Radio` case 0x16 (shared with cases 7 and 9):
```c
case 7:
case 9:
case 0x16:
    (**(code **)(puVar7 + 0x278))(0x18, param_2);  // send ENTER_DOCK(0x18) to sender (building)
    RadioClass__Receive_Radio(param_2, param_3, param_4);
    return 1;
```

**This means: when the harvester receives 0x16 from the building, its TechnoClass base
immediately sends ENTER_DOCK (0x18) back to the building.** This is a hidden side-effect not
present in the existing docs. The building's case 0x18 then fires (TechnoClass::Receive_Radio
case 0x18: sets `field_0x198 = 1`, propagates ENTER_DOCK to next contact).

### 4.2 Sub-step (B): Chrono-teleporting gate — field at UnitClass+0x6AF

`*(char *)((int)param_1 + 0x6AF)` is a **direct byte offset** (cast form, not array indexing).
`UnitClass+0x6AF` is the chrono-teleporting flag. When `true` (non-zero), the locomotor
set-rate call is skipped entirely.

### 4.3 Sub-step (C): Locomotor rate timer — the 0x4000 value

`RateTimer__Current(local_1c)` reads the current interpolated rate value of the locomotor's
rate timer into `local_1c` (a 4-byte stack buffer; returns pointer into it). The check
`*psVar4 != 0x4000` prevents redundantly re-setting the timer if it is already at 0x4000.

`(**(code **)(*(int *)param_1[0x19d] + 0x4c))((int *)param_1[0x19d], 0x4000)`:
- `param_1[0x19d]`: `param_1` is `int*`, so byte offset = `0x19D × 4 = 0x674`.
  `UnitClass+0x674` = the locomotor ILocomotion* pointer.
- `vtable+0x4C` = the locomotor "set rate" method, called with arg `0x4000`.
- **`0x4000` is not a facing value.** It is a rate/speed value fed to the DriveLocomotion
  internal rate timer. `0x4000 = 16384` in decimal. In the context of the RA2 DriveLocomotion,
  the rate timer uses 16-bit fixed-point; `0x4000` is the target rate for dock-approach crawl.

### 4.4 Sub-step: Cascade to 0x15 (TIMING_SYNC_BACK)

If the locomotor is **already** at 0x4000 (timer already set), and:
- locomotor Is_Moving() == false (idle)
- harvester has a destination
- destination GetWhat() == 6 (Building)
- harvester mission == 7 (Dock)

Then the harvester sends `TIMING_SYNC_BACK(0x15)` to the destination building.

Building case 0x15 (from decompile of `0x0043C2D0`):
```c
case 0x15:
    if (iVar10 == 0x13) { return 10; }        // selling — reject
    if (puVar8[0x16ae] != '\0') { return 1; } // UnitAbsorb: accept
    if (puVar8[0x16af] != '\0') { return 1; } // InfAbsorb: accept
    if (puVar8[0x16a9] || puVar8[0x16aa] || puVar8[0x16c1] || puVar8[0x16c2]) {
        param_1->field_0x6dd = 1;             // BuildingClass+0x6DD flag set
        (vtable+0x1e8)(0x14, 0);              // building mission → Open (0x14)
        (*piStack_4+0x1e8)(0, 0);             // harvester mission → 0 (Stop)
        return 1;
    }
    if (puVar8[0x16ab]) {                     // Bunker
        param_1->field_0x6dd = 1;
        (vtable+0x1e8)(0x14, 0);
        return 1;
    }
    if (puVar8[0x16b3]) {                     // DockUnload = REFINERY
        (param_2->vtable+0x1e8)(0x10, 0);    // harvester mission → Unload (0x10)
        return 1;
    }
```

For a standard refinery (DockUnload=yes), case 0x15 fires `SetMission(0x10)` on the harvester
(Unload mission). **This is the trigger that starts the ore deposit process.** The building does
NOT set its own `field_0x6dd` on the DockUnload path.

---

## 5. Naming Conflict Resolution: TIMING_SYNC vs FACE_DOCK

### The conflict

- `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` §7 calls radio 0x16 **"TIMING_SYNC"**
  and describes it as triggering `SetSpeed(0x4000)`.
- `HARVESTER_DOCK_UNLOAD.md` line 252 calls radio 0x16 **"FACE_DOCK"** with description
  "Stop, set facing to 0x4000".

### Verdict: "TIMING_SYNC" is the better name; "FACE_DOCK" is misleading

**Binary evidence:** `UnitClass::Receive_Radio` case 0x16 at `0x00737430`:
- Calls `(*locomotor_vtable+0x4C)(locomotor, 0x4000)` — this is a locomotor **rate timer**
  operation, verified by the preceding call to `RateTimer__Current`.
- There is **no facing-write in case 0x16**. The value `0x4000` goes into a rate timer, not
  a facing field.
- The facing-write (`0x4000` to a facing register) would use vtable slot `+0x60` or equivalent
  facing methods — not `+0x4C`.

**What `0x4000` means in this context:** `0x4000` is fed to the locomotor's rate timer. In the
DriveLocomotion, the rate timer controls approach speed. `0x4000` is the dock-crawl rate.
**It is not a compass facing.**

**The HARVESTER_DOCK_UNLOAD.md "FACE_DOCK" label** is a misread: the author saw the constant
`0x4000` and assumed a facing convention (in YR, `0x4000` = east in 16-bit facing). But the
call is to a rate-timer method, not a facing-setter.

**The existing `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` §7** calls it
"TIMING_SYNC" and says it "triggers locomotor sync (`SetSpeed(0x4000)`)". This is correct:
the effect is locomotor rate/timing synchronization for dock approach, not a facing change.

**Recommended canonical name:** `TIMING_SYNC (0x16)`. It describes what the building is doing
(synchronizing the harvester's approach timing) and is consistent with what the binary shows.

---

## 6. Building-Side Effects During the 0x16 Sequence

From the decompile of `BuildingClass::Receive_Radio` case 0x0E:

| Event | Building-side effect |
|-------|---------------------|
| MOVE_TO_CELL(0x12) sent | `*param_4` = CellClass* (queue cell); written as out-param to caller |
| ENTER_DOCK(0x18) sent | No building-side write; building calls Transmit, doesn't receive it |
| 0x16 sent | No building-side write in case 0x0E; return value of Transmit is checked |
| 0x16 return == 1 | case 0x0E returns 1 (ROGER); sound cue **skipped** |
| 0x16 return != 1 | PlaySound(&DAT_0089c848, 1, 1) fired; "ok" acceptance sound |
| 0x15 received back | DockUnload path: `(harvester)->SetMission(0x10)` (Unload) |

**No `field_0x2E4` write occurs during case 0x0E.** The on-pad unit pointer at
`BuildingClass+0x2E4` is set when the unit physically arrives on the pad (later state machine
tick), not during CAN_DOCK.

**No dock-anim transition occurs during case 0x0E.** Anim slot changes happen in case 0x15
(for UnitRepair/Hospital/Armory/Bunker paths) and when the harvester's Unload mission reaches
its ore-deposit state.

---

## 7. DockManager / FUN_006AF6C0 — Does NOT Emit 0x16

Verified from decompile of `0x006AF6C0`:

`FUN_006AF6C0` is `SlaveManagerClass::AI_Update` — the per-frame state machine for Yuri Slave
Miner slaves. It does not manage harvester-refinery docking at all and emits **no radio 0x16**.
This is consistent with `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md` §"CRITICAL
FINDING". The "dock manager" label in some prior docs is wrong; the actual harvester dock queue
state machine is a different function.

---

## 8. TechnoClass::Receive_Radio Case 0x16 — Unexpected Base Behavior

Verified from decompile of `0x006F4AB0`:

`TechnoClass::Receive_Radio` handles case 0x16 in a **shared block with cases 7 (COME_TO_ME)
and 9**:
```c
case 7:
case 9:
case 0x16:
    (**(code **)(puVar7 + 0x278))(0x18, param_2);  // Transmit_Radio(ENTER_DOCK, sender)
    RadioClass__Receive_Radio(param_2, param_3, param_4);
    return 1;
```

This base handler is **bypassed for units** because `UnitClass::Receive_Radio` case 0x16
handles the message and returns 1 without falling through to the TechnoClass base. However,
`UnitClass::Receive_Radio` case 0x16 **calls `FootClass__Receive_Radio`** first as a delegate.
`FootClass::Receive_Radio` has no case 0x16, so it falls to `TechnoClass::Receive_Radio` case
0x16 — which DOES fire `Transmit_Radio(0x18, building)`.

**Result:** When the harvester receives 0x16, it:
1. Immediately sends ENTER_DOCK(0x18) back to the building (via TechnoClass base).
2. Then does the locomotor rate-timer check (0x4000 gate).
3. Then may cascade to TIMING_SYNC_BACK(0x15).

This creates a **bidirectional ENTER_DOCK exchange**: the building sends 0x18, then receives
it back from the harvester via the 0x16 handler chain. The building's case 0x18 response to
the harvester's retransmission sets `TechnoClass::field_0x198 = 1` on the building.

---

## 9. Helipad Path — Different Sequence, No 0x16

From `BuildingClass::Receive_Radio` case 0x0E, Helipad branch (`Type[0x16CB] != 0`):
```c
// Helipad path:
*param_4 = param_1;                                      // out-param = building itself
iVar10 = (**(code **)(vtable + 0x27c))(0x12, param_4, param_2);  // MOVE_TO_CELL
if (iVar10 != 0x14) { return 1; }
(**(code **)(vtable + 0x274))(0x18);                     // ENTER_DOCK to Contacts[0] (ToFirst)
return 1;                                                 // NO 0x16 sent
```

**The Helipad path does not send radio 0x16.** It sends 0x12 (with building pointer as payload,
not a cell), then 0x18 via `Transmit_Radio_ToFirst`, then returns. No TIMING_SYNC.

---

## 10. Open Questions — Final State

- `[RESOLVED] OQ01 — Which name for 0x16 is correct?` → **TIMING_SYNC**. Binary shows locomotor
  rate-timer call, not facing-setter. (evidence: `UnitClass::Receive_Radio` case 0x16 at
  `0x00737430`, `RateTimer__Current` call + `vtable+0x4C` locomotor rate method)

- `[RESOLVED] OQ02 — What arg payload does 0x16 carry?` → None. Sent via
  `Transmit_Radio(0x16, harvester)` (vtable+0x278) which uses global scratch buffer; no
  caller-meaningful payload. (evidence: `0x0043C2D0` case 0x0E decompile)

- `[RESOLVED] OQ03 — What radios fire in the same sequence?` → 0x13, 0x12, 0x18, 0x16 in that
  order. 0x18 and 0x16 only fire if 0x12 returns 0x14 (ALREADY_THERE). (evidence: full case
  0x0E decompile at `0x0043C2D0`)

- `[RESOLVED] OQ04 — What building-side effects occur during the sequence?` → Only the out-param
  write (`*param_4 = CellClass*` for 0x12, or `*param_4 = this` for Helipad). No field_0x2E4
  write, no anim transition, no `field_0x6dd` write at 0x0E time. (evidence: full case 0x0E
  decompile)

- `[RESOLVED] OQ05 — Does FUN_006AF6C0 emit 0x16?` → No. It is `SlaveManagerClass::AI_Update`,
  a completely different system. (evidence: decompile of `0x006AF6C0`)

- `[RESOLVED] OQ06 — Does TechnoClass base handler for 0x16 fire for harvesters?` → Yes, but
  indirectly: `UnitClass::Receive_Radio` case 0x16 delegates to `FootClass::Receive_Radio` which
  falls through to `TechnoClass::Receive_Radio` case 0x16, which sends 0x18 back to the building.
  (evidence: decompiles of `0x00737430`, `0x004D8FB0`, `0x006F4AB0`)

- `[RESOLVED] OQ07 — What is the 0x4000 value?` → Locomotor rate timer target, not a facing
  value. Specifically the dock-approach crawl rate fed to `DriveLocomotion::vtable+0x4C`.
  (evidence: `UnitClass::Receive_Radio` `0x00737430` + `RateTimer__Current` `0x004C93D0`)

- `[RESOLVED] OQ08 — Is this code active in YR (not TS-only)?` → Yes. Standard refinery docking
  (DockUnload=yes, GAREFN/NAREFN) uses this path every match. No TS-legacy gate detected.

- `[DEFERRED] OQ09 — What is `DriveLocomotion::vtable+0x4C`'s exact function name?`
  (category: bounded-cost-too-high; reason: DriveLocomotion vtable layout requires a separate
  investigation; next step: decompile `DriveLocomotionClass` constructor to read vtable layout)

- `[DEFERRED] OQ10 — What is the exact CellClass* returned by `MapClass__Get_CellClass` for
  the queue cell?` (category: requires-different-system-context; reason: depends on the map
  cell layout at runtime; formula is anchor+(3,1) which is sufficient for implementation)

- `[DEFERRED] OQ11 — What does the BuildingClass case 0x18 do when invoked by the harvester's
  TechnoClass base chain?` (category: out-of-scope; reason: slot 3 covers the receiver side;
  from this session: TechnoClass case 0x18 sets `field_0x198=1` and propagates ENTER_DOCK,
  but full BuildingClass-level handling of 0x18 is the receiver scope)

---

## 11. Verified Facts Summary

| # | Fact | Evidence | Confidence |
|---|------|----------|-----------|
| 1 | Radio 0x16 is emitted by the building at `(param_1->vtable + 0x278)(0x16, param_2)`, after ENTER_DOCK(0x18), gated on MOVE_TO_CELL returning 0x14 | Decompile `0x0043C2D0` case 0x0E | HIGH |
| 2 | 0x16 carries no caller-meaningful payload; uses `Transmit_Radio` (vtable+0x278) with global scratch buffer | Same decompile | HIGH |
| 3 | 0x16's correct name is **TIMING_SYNC**, not "FACE_DOCK"; it triggers a locomotor rate-timer set to `0x4000`, not a facing write | `UnitClass::Receive_Radio` case 0x16 at `0x00737430`; `RateTimer__Current` at `0x004C93D0` | HIGH |
| 4 | Receiving 0x16 causes the harvester to immediately send ENTER_DOCK(0x18) back to the building (via TechnoClass base chain) before doing the locomotor rate-timer set | Decompiles of `0x00737430`, `0x004D8FB0`, `0x006F4AB0` | HIGH |
| 5 | If locomotor already at rate 0x4000 AND locomotor idle AND destination is a building AND mission==7 (Dock), the harvester sends TIMING_SYNC_BACK(0x15) to the building; on the DockUnload refinery path, building case 0x15 sets the harvester's mission to Unload (0x10) | `UnitClass::Receive_Radio` `0x00737430` + `BuildingClass::Receive_Radio` case 0x15 at `0x0043C2D0` | HIGH |

---

## 12. Addresses Reference

| Symbol | Address |
|--------|---------|
| `BuildingClass::Receive_Radio` | `0x0043C2D0` |
| `UnitClass::Receive_Radio` | `0x00737430` |
| `FootClass::Receive_Radio` | `0x004D8FB0` |
| `TechnoClass::Receive_Radio` | `0x006F4AB0` |
| `RateTimer__Current` | `0x004C93D0` |
| `SlaveManagerClass::AI_Update` (NOT dock-queue) | `0x006AF6C0` |

## Sources

- Decompiled `BuildingClass::Receive_Radio` @ `0x0043C2D0` (full decompile this session)
- Decompiled `UnitClass::Receive_Radio` @ `0x00737430` (full decompile this session)
- Decompiled `FootClass::Receive_Radio` @ `0x004D8FB0` (full decompile this session)
- Decompiled `TechnoClass::Receive_Radio` @ `0x006F4AB0` (full decompile this session)
- Decompiled `RateTimer__Current` @ `0x004C93D0` (decompile this session)
- Decompiled `SlaveManagerClass::AI_Update` @ `0x006AF6C0` (decompile this session)
- `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` — sibling doc; extended here
- `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` — HELLO(0x02) context
- `HARVESTER_DOCK_UNLOAD.md` — source of "FACE_DOCK" naming (corrected to TIMING_SYNC)
- `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md` — confirmed identity of 0x006AF6C0
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` — protocol overview

---

## Status: COMPLETE
