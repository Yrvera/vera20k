# Refinery Dock Exit Chain — Verification + Q1/Q2 Resolution

**Addresses:** 0x004D9290 (FootClass::Mission_Enter), 0x004595C0 (BuildingClass::ReleaseDockedHarvester), 0x004593A0 (BuildingClass::UndockUnit)  
**Date:** 2026-05-20  
**Confidence:** HIGH — all three functions re-decompiled and disassembled live in this session via `decompile_function` + `disassemble_function`. Transmit_Radio inventory from raw PUSH literals in disassembly.  
**Active in YR:** Yes — core ore harvesting loop (ReleaseDockedHarvester every delivery; UndockUnit on interrupt; FootClass::Mission_Enter every enter-mission dispatch)

---

## 1. Overview

This report covers:
- **Part A** — `FootClass::Mission_Enter` at 0x004D9290: new full decompile (no prior doc).
- **Part B** — `BuildingClass::ReleaseDockedHarvester` at 0x004595C0: verification of `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`.
- **Part C** — `BuildingClass::UndockUnit` at 0x004593A0: verification of `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`.
- **Part D** — Synthesis: full exit chain map, and **definitive answer** on where (if anywhere) radio codes 0x19 (LEAVE_DOCK) and 0x07 (DOCKING_COMPLETE) are transmitted in the refinery exit chain.

---

## 2. FootClass::Mission_Enter (0x004D9290) — New Decompile

### 2.1 Identity and vtable binding

**Label:** `FootClass__Mission_Enter` at 0x004D9290.  
**Body:** 0x004D9290 – 0x004D949B.  
**Vtable presence:** 0x004D9290 appears as a DATA xref at three vtable addresses:
- 0x007E8ED4 — read via `read_memory 0x007e8ed4`; confirmed bytes `90 92 4d 00`.
- 0x007EB298 — read via `read_memory 0x007eb298`; confirmed bytes `90 92 4d 00`.
- 0x007F5EB0 — read via `read_memory 0x007f5eb0`; confirmed bytes `90 92 4d 00`.

These are three different class vtables that inherit or override the same Mission_Enter slot. No direct CALL sites exist (function is called exclusively through vtable dispatch). Verified via `get_function_callers FootClass__Mission_Enter` → "No callers found" (expected for vtable-only dispatch).

**Active in YR:** Yes — dispatched whenever a unit's mission = 7 (Enter).

### 2.2 Relationship to UnitClass::Mission_Enter / PerCellProcess (0x00739EC0)

**These are NOT the same function and are NOT a dispatch hierarchy.**  
`FootClass::Mission_Enter` (0x004D9290) is the **base-class mission handler** for mission=7 in `FootClass`. `UnitClass::Mission_Enter` at 0x00739EC0 (now labeled `UnitClass__PerCellProcess` per the 2026-05-19 label-drift note in the prior doc) is the **per-cell hook** (`vtable+0x18C`) that drives the actual refinery dock choreography.

The two functions serve different purposes in the mission dispatch:

- `FootClass::Mission_Enter` (0x004D9290): Called each tick a unit is in Mission_Enter state. Handles the **approach and dock-queue** phase: sends `radio(0xE)` to the destination building, manages loco unpiggyback via `FUN_0045af20`, handles chrono-type branch (described below), and falls through to timer/jitter return. This is the outer tick loop.
- `UnitClass::PerCellProcess` (0x00739EC0): Called on **cell crossing** events while in mission=7. Drives the physical dock-cell arrival protocol (CLSID_WalkLocomotion check, `radio(0x15)`, loco Power_Off). This is the cell-triggered inner handler.

`FootClass::Mission_Enter` does NOT call `UnitClass::PerCellProcess` directly. They are invoked by different dispatch chains (mission tick vs. per-cell hook). `UnitClass` overrides the mission handler slot in its vtable, so `UnitClass::Mission_Enter` (if it exists as a separate function) replaces the FootClass version for UnitClass instances — the two functions overlap in mission=7 handling for different sub-roles.

**Verified:** via `decompile_function 0x004D9290` showing no CALL to 0x00739EC0; function body is self-contained.

### 2.3 Dispatch logic — what FootClass::Mission_Enter does

**Signature:** `int __fastcall FootClass__Mission_Enter(int *param_1)`  
`param_1` = `FootClass*` (unit instance). Return value = tick-delay integer (jitter return).

**High-level flow:**

1. **Destination check:** `FootClass__GetDestination(0)` at 0x0065AD30. If destination is null AND `Filter_AbstractType_InMap(field_0x218)` is also null → enter the **no-destination path** (abort/cleanup).

2. **No-destination path (destination == null && field_0x218 == null):**
   - Check `FUN_0070d8f0()` (loco movement status).
   - If NOT moving AND loco type is NOT 1 or 2 (not Drive/Walk): call `SetMission(0, 1)` via `vtable+0x484`. This resets the unit to idle/sleep mission.
   - Call `QueueMission()` via `vtable+0x1ec`.
   - Disassembly: `004d9454 CALL dword ptr [EAX + 0x484]` with PUSH 1 / PUSH 0 before it = `SetMission(mission=0, queued=1)`.

3. **Destination exists path** (main branch):
   - Send `radio(0xE, destination)` via `vtable+0x278` (Transmit_Radio_ToObject slot). Arg = 0xE = CAN_DOCK.
   - Check return value: if return == 1 OR `byte[param_1 + 0x418]` (IsHarvester flag) != 0:
     - **Sub-branch A — loco count > 0 AND no dock-link (param_1[0x169] == 0, param_1[0x166] > 0):**
       - Call `FUN_0045af20(loco_ptr)` — this is the **IPiggyback QueryInterface helper**. It calls `QueryInterface(IID_IPiggyback, &out)` on the locomotor. See §2.4.
       - If piggyback interface is available AND `Is_Ok_To_End()` (vtable+0x14) returns true:
         - Release the piggyback loco (call Release on inner loco, zero the pointer, call `IPiggyback::End_Piggyback` via vtable+0x10).
       - Call `Set_Destination(dock_queue[0], 0)` via `vtable+0x480` — navigates unit toward first dock-queue entry.
       - Decrement dock-queue count (`param_1[0x166] -= 1`), shift remaining entries.
     - **Sub-branch B — dock-link present OR no dock-queue (else branch):**
       - Call `vtable+0x84` to get TypeClass pointer.
       - Check `byte[TypeClass + 0xCD4]` (Teleporter flag).
       - If Teleporter != 0 (i.e., chrono miner): clear `param_1[0x169]` (dock-link) and `param_1[0x168]`, then call `Set_Destination(saved_dock_link, 1)` via `vtable+0x480`.
       - Disassembly: `004d93f2 MOV CL,byte ptr [EAX + 0xcd4]` / `004d9409 MOV dword ptr [ESI + 0x5a0],0x0` / `004d9413 MOV dword ptr [ESI + 0x5a4],0x0`.
   - If return of `radio(0xE)` != 1 AND IsHarvester == 0:
     - Send `radio(3)` via `vtable+0x274` (CLEAR_LINK=3) — breaks radio contact.
     - Call `SetMission(0, 1)` via `vtable+0x484` — reset to idle.
     - Disassembly: `004d92d0 PUSH 0x3` / `004d92d4 CALL dword ptr [EAX + 0x274]`.

4. **Timer epilogue (all paths):** Call `MissionClass__GetMissionTimerEntry()` → `Math__ftol()` → `Random__RandomRanged(0, 2)` → return jitter delay. This is the standard 1–3 tick jitter return for FootClass mission handlers.

### 2.4 FUN_0045af20 — IPiggyback QueryInterface helper

**Address:** 0x0045AF20. Not previously documented. Verified via `decompile_function 0x0045af20`.

This is **NOT** a "thunk to EnterBuildingOrDock" — it is a COM `QueryInterface` wrapper for `IID_IPiggyback`. The function:
- Takes a loco pointer and an output pointer for the piggyback interface.
- Calls `QueryInterface(IID_IPiggyback, &out)` on the locomotor.
- Returns the HRESULT. Returns -0x7FFFBFFE (= E_NOINTERFACE equivalent) if loco pointer is null.

**Active in YR:** Yes — called every tick a unit in Mission_Enter has a loco in the dock-queue path. For chrono miners with piggybacked DriveLoco, this retrieves the IPiggyback interface to check `Is_Ok_To_End()` for loco unpiggyback.

### 2.5 Transmit_Radio inventory — FootClass::Mission_Enter

Every PUSH immediate before a `CALL [reg+0x274]` or `CALL [reg+0x278]` in 0x004D9290:

| Address | PUSH value | vtable slot | Meaning |
|---------|-----------|-------------|---------|
| 0x004D92B5 | PUSH 0xE | +0x278 (Transmit_Radio_ToObject) | CAN_DOCK → building |
| 0x004D92D0 | PUSH 0x3 | +0x274 (RadioCommand) | CLEAR_LINK → only on failure path (radio(0xE) returned != 1 AND not harvester) |

**Neither 0x19 (LEAVE_DOCK) nor 0x07 (DOCKING_COMPLETE) is transmitted in FootClass::Mission_Enter.**  
Verified by full disassembly scan of 0x004D9290–0x004D949B.

---

## 3. ReleaseDockedHarvester (0x004595C0) — Verification

### 3.1 Re-decompile result

Re-decompiled via `decompile_function 0x004595C0`. Result matches the prior doc (`RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`) in every material claim. Caller confirmed via `get_xrefs_to 0x004595C0`: sole call site is `UnitClass__Mission_Deploy_Building @ 0x0073D66D` — UNCONDITIONAL_CALL. Prior doc claim verified.

### 3.2 Transmit_Radio inventory — ReleaseDockedHarvester

Scanned every PUSH before CALL [reg+0x274] or CALL [reg+0x278] in raw disassembly (0x004595C0 – 0x00459839):

| Address | PUSH value | vtable slot | Context |
|---------|-----------|-------------|---------|
| 0x00459828 | PUSH 0x3 | +0x274 on building | RadioCommand(CLEAR=3) — dock free signal, step 13 |

**That is the only transmit-radio call in the entire function.**

- **0x19 (LEAVE_DOCK):** NOT present. No `PUSH 0x19` anywhere in the body.
- **0x07 (DOCKING_COMPLETE):** NOT present. No `PUSH 0x7` anywhere in the body.

Verified by full disassembly scan. The only radio call is `PUSH 0x3` at 0x00459828, which is `RadioCommand(CLEAR=3)` — the dock-free signal to the production system.

Prior doc claim "Transmit_Radio_ToFirst(3)" is correct: it's `CALL dword ptr [EAX + 0x274]` (vtable+0x274 = RadioCommand on building) with arg 3 = CLEAR. No discrepancy.

### 3.3 Diffs vs prior doc

**No material diffs found.** Every step in the prior doc is confirmed by the live decompile:
- Anim slots 0xA+0xB cleared at entry: `004595cb CALL 0x00451e40` (slot 0xA), `004595d4 CALL 0x00451e40` (slot 0xB). ✅
- VOC at RulesClass+0x244: `004595de CMP dword ptr [EAX + 0x244], -0x1` / `CALL 0x007509e0`. ✅
- Anim slots 0xC and 0xD created by `CALL 0x00451890` with args 0xC and 0xD respectively. ✅
- Early-exit guard on `param_1->field_0x2e4 == null` → `SetMission(5)` + return. ✅
- Loco type guard: `CALL dword ptr [EAX + 0x2c]` / `CMP EAX, 0x1`. ✅
- `piVar1[0xb9] = 0` (unit dock-link clear at `004596e6`). ✅
- `Power_On` via loco vtable+0x58: `CALL dword ptr [ECX + 0x58]` at 0x00459709. ✅
- `Force_Track` with `PUSH 0x47` at 0x00459751, `SUB EBP, 0x80` at 0x00459726, `ADD EBX, 0x80` at 0x0045972c. ✅
- Speed 1.0: `PUSH 0x3ff00000` / `PUSH EBX(=0)` at 0x00459767. ✅
- `Get_Cell_Packed` via vtable+0x1b8 at 0x0045977e, NW-1/NW+1 arithmetic at 0x0045978b/0x0045978d. ✅
- `Find_Nearby_Passable_Cell` at 0x0056DC20 called at 0x004597e3. ✅
- `Set_Destination` via vtable+0x480 at 0x004597fa. ✅
- `SetMission(MOVE=2)` via vtable+0x1e8 at 0x00459807: `PUSH 0x2`. ✅
- Dock teardown: `field_0x2e4 = 0` at 0x00459814, `field_0x718 = 0` at 0x0045981a. ✅
- Building `SetMission(5)` at 0x00459820: `PUSH 0x5`. ✅
- `RadioCommand(CLEAR=3)` at 0x0045982c: `PUSH 0x3`. ✅

**Status: prior doc fully verified. Zero stale or wrong claims.**

---

## 4. UndockUnit (0x004593A0) — Verification

### 4.1 Re-decompile result

Re-decompiled via `decompile_function 0x004593A0`. Result matches the prior doc (`BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`) in every material claim.

### 4.2 Transmit_Radio inventory — UndockUnit

Scanned every PUSH before CALL [reg+0x274] or CALL [reg+0x278] in raw disassembly (0x004593A0 – 0x0045946F):

| Address | PUSH value | vtable slot | Context |
|---------|-----------|-------------|---------|
| 0x00459458 | PUSH 0x3 | +0x274 on building | RadioCommand(CLEAR=3) — dock free signal |

**That is the only transmit-radio call in the entire function.**

- **0x19 (LEAVE_DOCK):** NOT present. No `PUSH 0x19` anywhere in the body.
- **0x07 (DOCKING_COMPLETE):** NOT present. No `PUSH 0x7` anywhere in the body.

Verified by full disassembly scan of 0x004593A0–0x0045946F (29 instructions total, compact function). The only radio call is `PUSH 0x3` at 0x00459458 → `CALL dword ptr [EAX + 0x274]` at 0x00459462 on the building.

### 4.3 Callers verified

`get_xrefs_to 0x004593A0` returns:
- `BuildingClass__Sell @ 0x0044AAB0` — UNCONDITIONAL_CALL ✅
- `TemporalClass__Update @ 0x0071AA15` — UNCONDITIONAL_CALL ✅
- `BuildingClass__ReceiveDamage @ 0x004424EA` — UNCONDITIONAL_CALL ✅

Prior doc listed all three. Verified exact addresses. No additional callers. **Normal dock completion does NOT call UndockUnit.**

### 4.4 (-0x80, +0x80) and track 0x47 — verified in disassembly

- `SUB EBX, 0x80` confirmed at 0x00459401 (bytes: `81 EB 80 00 00 00`).
- `ADD EBP, 0x80` confirmed at 0x00459407 (bytes: `81 C5 80 00 00 00`).
- `PUSH 0x47` confirmed at 0x0045942C — the Force_Track argument.

Prior doc claim: "bytes `81 EB 80 00 00 00` / `81 C5 80 00 00 00` confirmed in raw memory read" — ✅ verified in live disassembly this session.

### 4.5 Diffs vs prior doc

**No material diffs found.** All five load-bearing facts in the prior doc verified:
1. UndockUnit is interrupt/destroy handler only (3 callers, no normal path). ✅
2. `(-0x80, +0x80)` hardcoded lepton deltas in instruction stream. ✅
3. Speed 1.0 = IEEE 754 double (`PUSH 0x3FF00000` / `PUSH EBX(0)` at 0x00459442/0x00459447). ✅
4. 0x47 is a hardcoded drive track index (`PUSH 0x47` at 0x0045942C). ✅
5. `[0xB9]` int-index = byte offset 0x2E4 — dock-link cleared on both sides (`004593a9 MOV ESI,[EDI+0x2e4]` and `00459450 MOV dword ptr [ESI+0x2e4],EBX` / `0045945c MOV dword ptr [EDI+0x2e4],EBX`). ✅

**Status: prior doc fully verified. Zero stale or wrong claims.**

---

## 5. Full Exit-Chain Synthesis

### 5.1 Superseded: conditional reciprocal-link exit chain

> **Correction 2026-05-21:** The `ReleaseDockedHarvester` chain below remains
> valid when the nonzero reciprocal-link branch reaches it. It is not the normal
> stock zero-link `CMIN/HARV -> GAREFN/NAREFN` ore-delivery exit. Stock
> DockUnload completion uses `UnitClass::Mission_Deploy_Building` state 4.

```
UnitClass::Mission_Deploy_Building (0x0073D630)
  → state 4 depart: SetMission(Harvest=10), radio(3) CLEAR_LINK, QueueMission
  → param_1[0xB9] != 0 branch: CALL 0x004595C0  ← ReleaseDockedHarvester
      → ClearAnimSlot(A, B), PlayVOC, CreateAnimSlot(C, D)
      → Power_On, Force_Track(0x47, center-0x80, center+0x80)
      → SetSpeedMult(1.0)
      → Find_Nearby_Passable_Cell(NW-1, NW+1)
      → Set_Destination(passable_cell), SetMission(MOVE=2)
      → Clear both dock-links, building SetMission(5)
      → RadioCommand(CLEAR=3)  ← ONLY radio call
```

**BREAK(3) / CLEAR_LINK emitter:** `BuildingClass::ReleaseDockedHarvester` at address 0x0045982C.  
(Also: `UnitClass::Mission_Deploy_Building` state-4 sends radio(3) at 0x0073E273 if radio contact active — this is a second CLEAR call on the same exit path, before ReleaseDockedHarvester is called.)

### 5.2 Interrupt exit chain (sell / destroy / chrono-wipe)

```
BuildingClass::Sell (0x0044AAB0)          ─┐
BuildingClass::ReceiveDamage (0x004424EA)  ├→ CALL 0x004593A0  ← UndockUnit
TemporalClass::Update (0x0071AA15)        ─┘
      → Power_On (loco vtable+0x58: note: prior doc says Stop, but decompile shows vtable+0x58 = ILocomotion::Stop)
      → Force_Track(0x47, center-0x80, center+0x80)
      → SetSpeedMult(1.0)
      → Clear both dock-links
      → RadioCommand(CLEAR=3)  ← ONLY radio call
```

**BREAK(3) emitter:** `BuildingClass::UndockUnit` at address 0x00459462.

---

### 5.3 Who emits 0x19 (LEAVE_DOCK) in the dock exit chain? — DEFINITIVE ANSWER

**Neither ReleaseDockedHarvester nor UndockUnit transmits 0x19.**

Full scan of all three functions in this session found zero instances of `PUSH 0x19` followed by a Transmit_Radio call:
- `FootClass::Mission_Enter` (0x004D9290): no 0x19.
- `BuildingClass::ReleaseDockedHarvester` (0x004595C0): no 0x19.
- `BuildingClass::UndockUnit` (0x004593A0): no 0x19.

Combined with Phase 1 finding that `UnitClass::Mission_Deploy_Building` (0x0073D630) does NOT transmit 0x19, and Phase 2 slot 2 covering FootClass functions — **0x19 LEAVE_DOCK is not transmitted anywhere in the normal or interrupt refinery exit chain.**

**What 0x19 (LEAVE_DOCK) actually does:** Per the prior doc `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` Radio Command table, 0x19 = LEAVE_DOCK, direction Building→Unit. Per TechnoClass Receive_Radio case 0x19 (documented in Phase 1 inline TechnoClass decompile): clears the `DockedIn` flag on the unit side. However, this radio code may be transmitted from a **different caller** — likely `BuildingClass::MissionRepairAndProduce` (the building unload state machine) when transitioning from unloading to release, or from `BuildingClass::Receive_Radio` case 0x15 (DOCK_NOW). These functions are outside the scope of this slot's investigation.

**The dock-link teardown in both exit functions works by directly zeroing the pointer fields** (building+0x2E4 and unit+0x2E4), not by radio protocol. The `DockedIn` flag (unit-side) is cleared by `piVar1[0xb9] = 0` directly in ReleaseDockedHarvester and by `piVar1[0xb9] = 0` in UndockUnit — without sending radio 0x19.

### 5.4 Who emits 0x07 (DOCKING_COMPLETE) in the dock exit chain? — DEFINITIVE ANSWER

**Neither ReleaseDockedHarvester nor UndockUnit transmits 0x07.**

Full scan found zero instances of `PUSH 0x7` followed by a Transmit_Radio call in either function. Also not present in `FootClass::Mission_Enter`.

**What 0x07 (DOCKING_COMPLETE) actually does:** Per the prior doc, 0x07 = DOCKING_COMPLETE, direction Building→Unit. This is described in `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §8.3 as sent by the building when the dock becomes free, causing the docked unit to "clear its destination and enter Guard mission." This is likely transmitted from `BuildingClass::MissionRepairAndProduce` (the building's unload loop state machine, addresses around 0x44xxxx), which is outside the scope of this three-function slot.

**Implication:** The Phase 1–2 investigation confirms that neither of the two exit functions (ReleaseDockedHarvester, UndockUnit) sends 0x07 or 0x19. If these codes exist in the binary, they originate from the building's own mission tick handler — not the exit-sequence functions.

---

## 6. Tiny Details

1. **FootClass::Mission_Enter field 0x5A0/0x5A4 chrono branch:** The Teleporter branch in FootClass::Mission_Enter (0x004D9409/0x004D9413) clears both `param_1[0x5A0]` and `param_1[0x5A4]` before calling `Set_Destination(saved_dock_link, 1)`. Field 0x5A0 is a secondary slot not mentioned in prior docs; likely the "approach target" companion to the dock-link at 0x5A4. Verified: `004d9409 MOV dword ptr [ESI + 0x5a0],0x0`.

2. **FootClass::Mission_Enter dock-queue fields 0x598 and 0x58C:** `param_1[0x166]` = `param_1+0x598` = dock-queue count; `param_1[0x163]` = `param_1+0x58C` = dock-queue array pointer. The function dequeues the first entry (`Set_Destination(queue[0], 0)`) and shifts remaining entries down. These are FootClass-level fields, not previously documented in the struct layout tables.

3. **FootClass::Mission_Enter loco type guard at 0x004D9430:** The no-destination abort path checks loco type (`vtable+0x2C` returning 1 or 2) before calling SetMission(0,1). If loco type IS 1 (Drive) or 2 (Walk), the abort is skipped — unit is presumed still moving to a valid destination under locomotor control.

4. **ReleaseDockedHarvester field 0x718:** `building->field_0x718` is cleared in two places — the early-exit guard (null-unit path) and the main teardown (step 13). Semantically: a dock-state/unloading-in-progress flag on BuildingClass. Not yet named in struct layout docs; byte offset 0x718 confirmed both locations in disassembly: `004596c2 MOV dword ptr [EDI + 0x718],EBX` and `0045981a MOV dword ptr [EDI + 0x718],EBX`.

5. **UndockUnit loco call is Stop not Power_On:** The prior doc header comment says "ILocomotion::Stop (vtable+0x58)" — confirmed. But the doc body (Step 5) also says "Stop" via vtable+0x58. ReleaseDockedHarvester uses vtable+0x58 for **Power_On** per the prior doc. Both functions call `CALL dword ptr [ECX + 0x58]` — same vtable slot. The semantic difference (Stop vs Power_On) is in naming, not address. Ghidra shows same slot for both. Functionally both prepare the locomotor before Force_Track.

---

## 7. Open Questions — Final State

| Question | Status |
|----------|--------|
| Does ReleaseDockedHarvester transmit 0x19 LEAVE_DOCK? | **NO** — definitively closed. |
| Does ReleaseDockedHarvester transmit 0x07 DOCKING_COMPLETE? | **NO** — definitively closed. |
| Does UndockUnit transmit 0x19 LEAVE_DOCK? | **NO** — definitively closed. |
| Does UndockUnit transmit 0x07 DOCKING_COMPLETE? | **NO** — definitively closed. |
| Does FootClass::Mission_Enter transmit 0x19 or 0x07? | **NO** — definitively closed. |
| Where IS 0x19 LEAVE_DOCK transmitted? | **Open** — outside this slot. Likely `BuildingClass::MissionRepairAndProduce` (building unload tick handler, ~0x44xxxx). Investigate separately. |
| Where IS 0x07 DOCKING_COMPLETE transmitted? | **Open** — outside this slot. Same candidate function. Investigate separately. |
| FootClass dock-queue fields 0x598/0x58C struct names | **Open** — not in canonical struct layout docs. Add to FootClass struct table. |
| FUN_0045af20 — should be labelled | **Open** — label candidate: `FootClass__QueryIPiggyback` or `ILocomotion__QueryIPiggyback`. Re-decompile confirmed COM QueryInterface wrapper for IID_IPiggyback. |

---

## Sources

- `decompile_function 0x004D9290` — FootClass::Mission_Enter, verified this session.
- `decompile_function 0x004595C0` — BuildingClass::ReleaseDockedHarvester, verified this session.
- `decompile_function 0x004593A0` — BuildingClass::UndockUnit, verified this session.
- `decompile_function 0x0045af20` — FUN_0045af20 (IPiggyback QI helper), verified this session.
- `disassemble_function 0x004D9290` — full instruction scan for radio transmit calls.
- `disassemble_function 0x004595C0` — full instruction scan for radio transmit calls.
- `disassemble_function 0x004593A0` — full instruction scan for radio transmit calls.
- `get_xrefs_to 0x004D9290` — vtable binding at 0x007E8ED4, 0x007EB298, 0x007F5EB0.
- `get_xrefs_to 0x004595C0` — sole caller at 0x0073D66D (UnitClass::Mission_Deploy_Building).
- `get_xrefs_to 0x004593A0` — three interrupt callers confirmed.
- `read_memory 0x007e8ed4`, `0x007eb298`, `0x007f5eb0` — vtable slot contents verified as 0x004D9290.
- Prior docs: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`, `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`, `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`, `MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md`, `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`.
