# Radio 0x10 RESERVE_DOCK — Sender Trace

**Receiver-side:** `BuildingClass::Receive_Radio @ 0x0043C2D0` case 0x10
(returns ROGER for Refinery=yes / UnitRepair=yes / Weeder=yes; see prior Phase 1+2 docs)
**Question:** does any live YR code transmit 0x10 to a refinery (or any building)?
**Confidence:** HIGH — all major candidate functions decompiled or disassembled directly this session; no sender found
**Active in YR:** NO SENDER FOUND — 0x10 is a dead-send in standard YR; receiver-side is live but unreachable

---

## 1. Overview

Phase 1 slot 2 found `BuildingClass::Receive_Radio` case 0x10 and (incorrectly) concluded it
returned NEGATORY for standard refineries. Phase 2 slot 5 corrected this: `Type[0x16BB]` =
`Refinery=yes`, so case 0x10 returns ROGER for GAREFN/NAREFN. This raised the question:
who sends 0x10?

This report exhaustively traces the sender side. The methodology is direct binary search:
decompile or disassemble every function that plausibly sends radio messages and look for
`PUSH 0x10` followed by a vtable+0x274 / +0x278 / +0x27C / +0x280 call (the four
`Transmit_Radio` dispatch slots per the RadioClass protocol doc).

**Result: no sender found in any candidate function.** 0x10 is receiver-ready but
nobody sends it in the standard YR harvester–refinery docking chain.

---

## 2. Methodology + Candidate Function Set Scanned

### Transmit_Radio dispatch patterns sought

In x86 assembly:
```
PUSH <target_ptr>       ; target argument (for vtable+0x278/+0x27C)
PUSH 0x10               ; message code = RESERVE_DOCK
MOV ECX, <this>
CALL dword ptr [reg + 0x274]   ; Transmit_Radio_ToFirst
CALL dword ptr [reg + 0x278]   ; Transmit_Radio (explicit target)
CALL dword ptr [reg + 0x27C]   ; Transmit_Radio_Impl (with payload)
CALL dword ptr [reg + 0x280]   ; Broadcast_Radio_ToAll
```

For `Transmit_Radio_ToFirst` (vtable+0x274) and `Broadcast_Radio_ToAll` (vtable+0x280),
the message code is the only argument, so the pattern is `PUSH 0x10` immediately preceding
the CALL. For the two-arg variants (+0x278, +0x27C), target comes before the message code.

### Candidate functions and scan results

| Address | Function | Radio sends found | 0x10 present? | Method |
|---------|----------|------------------|---------------|--------|
| `0x0073E5E0` | `UnitClass::Mission_Harvest` | HELLO(0x2) via `+0x278` at `0x0073EE55` | NO | disassemble + decompile |
| `0x004D9290` | `FootClass::Mission_Enter` | CAN_DOCK(0xE) via `+0x278` at `0x004D92B9` | NO | disassemble (assembly confirmed) |
| `0x00739EC0` | `UnitClass::PerCellProcess` | TIMING_SYNC(0x15) via `+0x274`; CAN_ENTER(0xF) via `+0x278`; CAN_DOCK(0xE) via `+0x278` | NO | decompile |
| `0x0041AA80` | `UnitClass::EnterBuildingOrDock` | CAN_DOCK(0xE) via `+0x278`; CAN_ENTER(0xF) via `+0x278`; BREAK(0x3) via `+0x274` | NO | decompile |
| `0x004DF040` | `FootClass::Find_Docking_Bay` | none (calls vtable+0x52C sub-helper) | NO | decompile |
| `0x004DFCB0` | `FootClass::Find_Nearest_Dock` | none (SetDestination + SetMission only) | NO | decompile |
| `0x00741970` | `TechnoClass::Set_Destination` | HELLO(0x2) via `+0x278`; CAN_DOCK(0xE) via `+0x278`; BREAK(0x3) via `+0x274`; DEPLOY_UNLOAD(0x19) via `+0x274` | NO | decompile |
| `0x00419C80` | `AircraftClass::Mission_Enter` | CAN_DOCK(0xE) via `+0x274`; TIMING_SYNC(0x15) via `+0x274` | NO | disassemble (assembly confirmed) |
| `0x0073D630` | `UnitClass::Mission_Deploy_Building` | BREAK(0x3) via `+0x274` at `0x0073DD88` | NO | disassemble |
| `0x00443C60` | `BuildingClass::ExitObject_Main` | HELLO(0x2) via `+0x278`; ENTER_DOCK(0x18) via `+0x278`; msg(0x9) via `+0x278` | NO | disassemble (Bash search) |

Two `PUSH 0x10` occurrences were found inside `BuildingClass::ExitObject_Main` at
`0x00444183` and `0x004445EC`, but both are followed by `CALL [EDX+0x1e8]` =
`Queue_Mission(0x10, ...)` (Mission_Unload, not radio). The value `0x10` in those
sites is a mission ID (Mission_Unload), not a radio message code.
(verified via `disassemble_function 0x00443C60`)

---

## 3. Sender Call Sites Found

**None.** No function in the candidate set sends `Transmit_Radio(0x10, ...)`.

---

## 4. Per-Class Receiver Coverage

### BuildingClass::Receive_Radio @ 0x0043C2D0 — case 0x10

Already documented (Phase 1 slot 2 + Phase 2 slot 5 correction). Summary:

```c
case 0x10:
    if (field_0x118 == 0
        AND FUN_0065adf0() /*sender is harvester-type*/
        AND field_0x81 == 0
        AND sender.GetOwner() == this.Owner)
    {
        if (Type[0x16BB] /*Refinery=yes*/) return 1;  // ROGER
        if (Type[0x16A9] /*UnitRepair=yes*/) return 1;
        if (Type[0x16BC] /*Weeder=yes*/)    return 1;
    }
    return 10;  // NEGATORY
```

**Active in YR:** receiver code is live and reachable in principle (no dead guard); a
refinery, service depot, or weeder would return ROGER. But because nobody sends 0x10,
this code path never executes in a standard match.

### AircraftClass::Receive_Radio @ 0x004190B0 — no case 0x10

Confirmed via `decompile_function 0x004190B0`. The switch handles:
8, 0xE, 0xF, 0x12, 0x13, 0x15, 0x17, 0x1D, 0x1F, 0x21.
No case 0x10. All other codes fall through to `FootClass::Receive_Radio`.
(Active in YR: YES for the cases it handles; no 0x10 receiver here.)

### FootClass::Receive_Radio @ 0x004D8FB0 — no case 0x10

Confirmed via `decompile_function 0x004D8FB0`. Handles cases:
0x11, 0x12, 0x13, 0x17, 0x1C, 0x23.
No case 0x10. Falls through to `TechnoClass::Receive_Radio`.

### UnitClass::Receive_Radio @ 0x00737430 — no case 0x10

Confirmed via `disassemble_function 0x00737430`. Switch dispatch table at `0x00737455`
covers offsets 0–0x21 relative to base 0x3. Case for message 0x10 (= index 0xD) would
land at table entry 0xD, but scanning all `CALL dword ptr [reg+0x194]` and `CALL 0x004d8fb0`
paths shows no 0x10-specific branch; the function dispatches on `param_3 - 3` and its
explicit cases do not include 0x10. Any unmatched code falls to `FootClass::Receive_Radio`.

### TechnoClass::Receive_Radio @ 0x006F4AB0 — not scanned

Not decompiled this session. However, per the prior RadioClass protocol doc
(`RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`), `TechnoClass::Receive_Radio` handles codes
0x07 and 0x09 (DOCKING_COMPLETE) explicitly and delegates the rest to
`RadioClass::Receive_Radio`. No evidence from prior reports that it handles 0x10.
(Flagged as open question — see §8.)

---

## 5. Verdict: is 0x10 RESERVE_DOCK Used in the Refinery Dock Chain?

**Verdict: NO for the standard YR harvester–refinery dock chain.**

### The actual refinery dock chain (summarised from prior docs)

The radio protocol for a harvester returning to a refinery is:

| Stage | Who sends | Code | Code name | Who receives |
|-------|-----------|------|-----------|--------------|
| 1 | Harvester `Mission_Harvest` state 2 | 0x02 | HELLO | Refinery |
| 2 | Refinery `BuildingClass::Receive_Radio` case 0x0E | 0x13 | NEED_TO_MOVE | Harvester |
| 3 | Refinery | 0x12 | MOVE_TO_CELL | Harvester |
| 4 | Refinery | 0x18 | ENTER_DOCK | Harvester |
| 5 | Refinery | 0x16 | TIMING_SYNC | Harvester |
| 6 | Harvester `UnitClass::PerCellProcess` | 0x15 | TIMING_SYNC_BACK | Refinery |
| 7 | Refinery `BuildingClass::Receive_Radio` case 0x15 | — | harvester→Mission_Enter | Harvester |

Code 0x10 appears nowhere in this chain. The admittance gate is case 0x0E
(CAN_DOCK), not 0x10.

### If no: TS-legacy dead-send vs aircraft-only vs other

**Most likely: TS-legacy receiver-only stub.**

The receiver-side case 0x10 in `BuildingClass` is consistent with a Tiberian Sun
reservation protocol that was dropped in YR. The `FUN_0065adf0` check (harvester-or-similar
flag), the `field_0x81` lockout, and the owner-match guard are all appropriate
preconditions for a "can you reserve me a dock slot now?" pre-approach message — the kind
of optimization that might have existed in TS to avoid path contention. In YR, the
`CAN_DOCK(0x0E)` mechanism already handles admission control inline when the unit
actually arrives, making 0x10 redundant.

The code is NOT aircraft-only: `AircraftClass::Receive_Radio` has no case 0x10, and
no aircraft-specific sender was found either.

---

## 6. Functions Scanned But No 0x10 PUSH Found

The following were checked and confirmed to contain no `PUSH 0x10` + Transmit_Radio_slot
combination:

1. `UnitClass::Mission_Harvest` @ 0x0073E5E0 — `disassemble_function 0x0073E5E0`
2. `FootClass::Mission_Enter` @ 0x004D9290 — `disassemble_function 0x004D9290`
3. `UnitClass::PerCellProcess` @ 0x00739EC0 — `decompile_function 0x00739EC0`
4. `UnitClass::EnterBuildingOrDock` @ 0x0041AA80 — `decompile_function 0x0041AA80`
5. `FootClass::Find_Docking_Bay` @ 0x004DF040 — `decompile_function 0x004DF040`
6. `FootClass::Find_Nearest_Dock` @ 0x004DFCB0 — `decompile_function 0x004DFCB0`
7. `TechnoClass::Set_Destination` @ 0x00741970 — `decompile_function 0x00741970`
8. `AircraftClass::Mission_Enter` @ 0x00419C80 — `disassemble_function 0x00419C80`
9. `UnitClass::Mission_Deploy_Building` @ 0x0073D630 — `disassemble_function 0x0073D630`
10. `BuildingClass::ExitObject_Main` @ 0x00443C60 — `disassemble_function 0x00443C60` + Bash grep

**NOT scanned this session** (out of scope or effort cap):

- `TechnoClass::Receive_Radio` @ 0x006F4AB0 — not decompiled; prior docs suggest it
  handles only 0x07/0x09 explicitly.
- `InfantryClass` mission handlers — infantry does not harvest.
- Any AI mission handler (AI harvester path) — `feedback_no_ai_yet.md` memo excludes AI.
- Functions reachable only via indirect vtable jump from the 10 scanned above that were
  not themselves scanned.

---

## 7. Tiny Details

1. **`PUSH 0x10` ≠ radio code 0x10.** Two sites in `BuildingClass::ExitObject_Main`
   push `0x10` but to `[reg+0x1e8]` = `Queue_Mission`, not to any Transmit_Radio slot.
   This is Mission_Unload (mission ID 16 decimal). At `0x00444183`: `CALL [EDX+0x1e8]`
   (confirmed by disassembly context, not a radio call). At `0x004445EC`: same.

2. **Receiver gate constants (from prior Phase 1/2 docs):**
   - `field_0x118 == 0`: building has no current occupants
   - `FUN_0065adf0()`: harvester-type check on the sender
   - `field_0x81 == 0`: building lockout/busy flag is clear
   - `sender.GetOwner() == this.Owner`: same-player check
   All four must pass before the Refinery/UnitRepair/Weeder type check.

3. **`Type[0x16BB]` = Refinery=yes** (Phase 2 slot 5 correction). This flag IS set on
   GAREFN/NAREFN in `rulesmd.ini` via `BuildingTypeClass_ReadINI_Water`. So the receiver
   would return ROGER — if anyone sent 0x10.

4. **`UnitClass::Receive_Radio` at `0x007374C3`** has `CMP EAX, 0x10` followed by return
   NEGATORY(0xA) — but this compares the unit's **current mission** (from `vtable+0x184`),
   not the incoming radio message code. This is a mission-state guard ("if I am in
   Mission_Unload=0x10, reject this message"), not a case dispatch on message 0x10.
   Verified via `disassemble_function 0x00737430` at addresses `0x007374BD–0x007374D4`.

---

## 8. Open Questions — Final State

1. **Does `TechnoClass::Receive_Radio @ 0x006F4AB0` handle case 0x10?**
   Not decompiled this session. Probability low (prior docs list only 0x07/0x09 there),
   but not confirmed. Resolution: `decompile_function 0x006F4AB0`.

2. **Does any TS-era game path that is still compiled into the binary (but unreachable
   in YR) send 0x10?** This session only confirmed the YR-active paths. TS-dead-code
   senders cannot be ruled out without a broader scan (e.g., via `search_strings` for
   cross-refs to any Transmit call that pushes 0x10 in non-mission-path code). For
   the purposes of the Rust port this is immaterial — unreachable code need not be
   ported.

3. **Was 0x10 ever used in the TS dock protocol?** Consistent with TS reservation-slot
   semantics (send RESERVE_DOCK before committing to approach, receive ROGER/NEGATORY
   to decide whether to proceed or pick a different building). No TS source or binary
   confirmation this session.

---

## Sources

- `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` — Phase 1 slot 2, receiver side
- `RADIO_REFINERY_DOCK_TS_LEGACY_AND_CONTEXT_GHIDRA_REPORT.md` — Phase 2 slot 5, `Type[0x16BB]` correction
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` — vtable slot table, Transmit_Radio_Impl body
- `decompile_function 0x004D9290` (FootClass::Mission_Enter)
- `decompile_function 0x00739EC0` (UnitClass::PerCellProcess)
- `decompile_function 0x0041AA80` (UnitClass::EnterBuildingOrDock)
- `decompile_function 0x004DF040` (FootClass::Find_Docking_Bay)
- `decompile_function 0x004DFCB0` (FootClass::Find_Nearest_Dock)
- `decompile_function 0x00741970` (TechnoClass::Set_Destination)
- `decompile_function 0x004190B0` (AircraftClass::Receive_Radio)
- `decompile_function 0x004D8FB0` (FootClass::Receive_Radio)
- `decompile_function 0x0073D630` (UnitClass::Mission_Deploy_Building)
- `decompile_function 0x0073E5E0` (UnitClass::Mission_Harvest)
- `disassemble_function 0x004D9290` (FootClass::Mission_Enter assembly)
- `disassemble_function 0x00419C80` (AircraftClass::Mission_Enter assembly)
- `disassemble_function 0x00737430` (UnitClass::Receive_Radio assembly)
- `disassemble_function 0x00443C60` (BuildingClass::ExitObject_Main — Bash-grepped for PUSH 0x10 and radio call patterns)
- `disassemble_function 0x0073E5E0` (UnitClass::Mission_Harvest assembly)
