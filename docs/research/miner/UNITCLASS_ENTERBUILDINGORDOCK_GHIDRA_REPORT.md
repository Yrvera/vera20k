# UnitClass::EnterBuildingOrDock — Harvester-Side Sender

**Address:** 0x0041AA80
**Vtable slot:** AircraftClass vtable (base 0x7E22A4) + 0x480 = DATA ref at 0x007E2724 (verified via `read_memory 0x007E2724` → `80 AA 41 00`)
**Confidence:** HIGH on identity mislabel finding; HIGH on full body decompile; HIGH on vtable binding; MEDIUM on field semantics (several offsets unresolved)
**Active in YR:** YES — fires whenever an aircraft's destination is set to any building or cell

---

## ⚠ CRITICAL IDENTITY FINDING — Label Mislabel

The Ghidra label `UnitClass__EnterBuildingOrDock` at `0x0041AA80` is **WRONG**.
This function is the **AircraftClass override of `Set_Destination` (vtable+0x480)**, not a UnitClass helper.

**Proof chain (all verified this session):**
- AircraftClass constructor (`0x00413D20`) sets `[ESI] = 0x7E22A4` (vtable base) — verified via `disassemble_function 0x00413D20`
- AircraftClass vtable slot +0x480 = `0x7E22A4 + 0x480 = 0x7E2724`
- `read_memory 0x007E2724, 4` → `80 AA 41 00` = `0x0041AA80` ✓
- UnitClass primary vtable base = `0x7F5C70` (from `disassemble_function 0x007353C0`); slot +0x480 = `0x7F60F0` = `0x00741970` = `TechnoClass::Set_Destination` — UnitClass does NOT override `Set_Destination`
- FootClass vtable base = `0x7E8C94` (from `disassemble_function 0x004D31E0`); slot +0x480 = `0x7E9114` = `0x004D94A0` (simple `this+0x5A0 = param` setter, not this function)
- UnitClass range is `[0x7F5C70, …]`; AircraftClass range is `[0x7E22A4, …]` — `0x007E2724` is unambiguously in the AircraftClass vtable

**Implication for this investigation:**
The task scope called this "harvester-side sender of 0x0E/0x16 radio traffic during refinery dock approach." That is NOT what this function does. This function is `AircraftClass::Set_Destination` — it handles aircraft arriving at helipads and repair pads. There is no harvester-refinery 0x0E/0x16 handshake in this function. The harvester-side dock approach is driven elsewhere (see §10).

---

## 1. Overview

`0x0041AA80` is the AircraftClass override of the `Set_Destination` virtual (vtable+0x480). It is called whenever an aircraft's movement destination is changed — not per-tick, but on each new destination assignment. Its job is:

1. **Null/invalid destination** → clear destination (call `FootClass::Set_Destination_Internal(NULL, flag)`)
2. **Destination is in-transit** (`vtable+0x54` returns true) → clear destination immediately
3. **Destination is type 6 (BuildingClass) AND aircraft is type 7 (AircraftClass) or currently in mission 7** → enter the dock-approach logic:
   - Check path validity (`PathType::Has_Valid_Steps`)
   - If path is valid → run `Filter_AbstractType_InMap` on destination, then attempt `CAN_DOCK(0x0E)` via radio
   - If path is NOT valid (no path yet) → attempt `CAN_DOCK(0x0E)` or `CAN_ENTER(0x0F)` depending on free-slot availability
4. **After building logic** → walk the current cell's object list for any UnitRepair/UnitReload building and handle locomotor suspension
5. **Always tail-calls** `FootClass::Set_Destination_Internal(dest, flag)` to finalize the destination

---

## 2. Signature & Callers (who invokes this, when?)

### Signature (verified from decompile `0x0041AA80`)

```c
void __thiscall AircraftClass__Set_Destination(
    AircraftClass *this,     // param_1 (ECX) — the aircraft
    AbstractClass *param_2,  // [ESP+4] — new destination (building, cell, or NULL)
    undefined4    param_3    // [ESP+8] — force-flag passed down to Set_Destination_Internal
);
```

`param_1` is `int*` — offsets used directly as byte offsets throughout.
Convention: `__thiscall`, caller cleans 8 bytes (`RET 0x8`).

### Callers

`get_function_callers 0x0041AA80` returned "no callers" — all callers dispatch through vtable+0x480, not via direct CALL. The vtable entry at `0x007E2724` is the only DATA xref (`get_xrefs_to 0x0041AA80`). There are no direct CALL xrefs.

Dispatch path: any code that calls `vtable+0x480` on an AircraftClass object will dispatch here. Key callers include:
- `TechnoClass::Set_Destination` (`0x00741970`) — the base implementation, calls `(*vtable+0x480)` on the techno after performing the base path
- `FootClass::Mission_Enter` (`0x004D9290`) — when aircraft enters Mission_Enter, the destination is set via vtable+0x480
- `UnitClass::PerCellProcess` (`0x00739EC0`) — per-cell dock choreography sets destinations via vtable+0x480

**When is it called:** Each time an aircraft's movement target changes. This is a one-shot call per destination change, not a per-tick function.

---

## 3. Decompile Walk-Through

### Stage 0 — Null guard (0x0041AA8B)

```c
if (param_2 == NULL) goto TAIL_CALL;   // just call Set_Destination_Internal(NULL, flag)
```

### Stage 1 — In-transit check (0x0041AA91–0x0041AAAE)

```c
cVar1 = (**(vtable+0x54))(param_2);    // vtable+0x54 = IsBeingWarpedOut / InTransit flag
if (cVar1 != 0) {
    FootClass__Set_Destination_Internal(0, param_3);   // clear destination
    return;
}
```

Active in YR: YES — `vtable+0x54` fires for chrono-teleporting units. Aircraft queued at chrono destination get their destination cleared.

### Stage 2 — RTTI check: destination must be BuildingClass (type 6) (0x0041AAB1–0x0041AABE)

```c
iVar2 = (*(vtable_of_param_2 + 0xC))(param_2+4);   // AbstractClass::GetAbstractRTTI
// 0xC = offset 12 into secondary vtable, called on param_2+4
if (iVar2 != 6) goto CELL_SCAN;   // not a building — skip dock logic
```

BuildingClass RTTI = 6. If destination is not a building, skip straight to the cell-list scan.

### Stage 3 — Aircraft RTTI check (0x0041AAC4–0x0041AADA)

```c
iVar2 = (**(this->vtable + 0x184))();   // What_Am_I() for this aircraft
if (iVar2 != 7) {
    // also check param_1[0x2D] == 7  (this+0xB4 = mission state field; 7 = Mission_Enter)
    if (param_1[0x2D] != 7) goto CELL_SCAN;
}
```

`0x184/4 = 97` — vtable slot 97 = `What_Am_I()`. AircraftClass returns 2 (not 7). Wait — `iVar2 == 7` check fails for aircraft (which return 2). But the second check is `param_1[0x2D] == 7` (byte at `this+0xB4` = mission state = 7 = Mission_Enter). So this path is entered when:
- aircraft is type 7 (some subtype?) OR
- aircraft's current mission == 7 (Mission_Enter)

For a standard aircraft (type 2), only the mission==7 path applies. This entire dock block runs only when the aircraft is actively in Mission_Enter.

### Stage 4 — PathType::Has_Valid_Steps check (0x0041AAE2–0x0041AAEF)

```c
cVar1 = PathType__Has_Valid_Steps(param_1);  // 0x0065AE30
if (cVar1 != 0) goto PATH_VALID_BRANCH;     // path already computed
// else: no path yet → NO_PATH_BRANCH
```

`PathType::Has_Valid_Steps` at `0x0065AE30` walks `this->Contacts[]` (radio) checking for any non-null entry — verified via `decompile_function 0x0065AE30`. Returns 1 if any contact slot is non-zero.

**NOTE:** The Ghidra label `PathType__Has_Valid_Steps` is likely a mislabel — the body actually checks the radio Contacts array (`this+0xE4`, `this+0xE8`). Functionally it answers "does this aircraft already have a radio contact?" This guards the dock-initiation logic.

### Stage 4a — NO_PATH_BRANCH: First approach (no radio link yet) (0x0041AAF5–0x0041AC4A)

```c
// Try to find a free contact slot on destination building
cVar1 = FUN_0065adf0(param_2);   // = FindFreeContactSlot(building) — walks building's Contacts[]
if (cVar1 == 0) {
    // No free slot → SetGhostCell on building, then check building type:
    TechnoClass__SetGhostCell(param_2);
    if (*(Type + 0x16CB) != 0) {  // Helipad=yes
        // HELIPAD BRANCH — see §4 Helipad sub-path
    }
    if (*(Type + 0x16A9) != 0) {  // UnitRepair=yes
        param_1[0x140] = param_2;  // this+0x500 = store repair pad pointer
        param_2 = NULL;            // clear pending dest
    }
} else {
    // Free slot found → send CAN_DOCK(0x0E) to building
    iVar2 = (**(this->vtable + 0x278))(0xE, param_2);   // Transmit_Radio(0x0E, building)
    if (iVar2 != 1) {  // not ROGER
        (**(this->vtable + 0x274))(3);    // Transmit_Radio_ToFirst(OVER_AND_OUT=3)
        if (UnitRepair=yes OR UnitReload=yes) {
            TechnoClass__SetGhostCell(param_2);
            param_2 = NULL;
        }
    }
    if (NOT (UnitRepair=yes OR UnitReload=yes)) {
        TechnoClass__SetGhostCell(param_2 = NULL);
    } else if (param_1[0x169] != 0) {  // this+0x5A4 = some stored building ptr
        TechnoClass__SetGhostCell(param_1[0x169]);
    }
}
```

### Stage 4b — PATH_VALID_BRANCH: Already has radio contact (0x0041AC4C–0x0041ACBE)

```c
// Run RTTI filter on destination
iVar2 = Filter_AbstractType_InMap(param_2);  // 0x0040DD70 — accepts type 1,2,6,0xF
if (iVar2 == 0) goto CELL_SCAN;             // not a valid in-map techno → skip

// Check if building has a free contact slot
cVar1 = FUN_0065adf0(iVar2);   // FindFreeContactSlot on filtered dest
if (cVar1 == 0) {
    TechnoClass__SetGhostCell(param_2);
    goto CELL_SCAN;
}

// Check if aircraft is already in building's contacts
cVar1 = DynamicVectorClass__Contains(iVar2, this);  // 0x0065AD50
if (cVar1 == 0) {   // NOT already in contacts
    // Try CAN_DOCK via ToFirst (assumes link already established)
    iVar2 = (**(this->vtable + 0x274))(0xE);    // Transmit_Radio_ToFirst(0x0E)
    if (iVar2 == 1) return;    // ROGER → dock accepted, done
    // Rejected:
    (**(this->vtable + 0x274))(3);    // OVER_AND_OUT (break link)
    if (UnitRepair=yes OR UnitReload=yes) {
        TechnoClass__SetGhostCell(param_2);
        param_2 = NULL;
    }
}
// If already in contacts: fall through to CELL_SCAN
```

### Stage 5 — CELL_SCAN: Walk current cell object list (0x0041ACC0–0x0041AD9C)

```c
iVar2 = (**(this->vtable + 0x1BC))();    // Get current cell
// check cell flags: (*(iVar2 + 0x140) & 0x100) — bridge/water flag
if ((*(iVar2+0x140) & 0x100) != 0) goto TAIL_CALL;
// walk linked list: *(iVar2+0xE4) = first object in cell
for (piVar5 = *(iVar2+0xE4); piVar5 != NULL; piVar5 = *(piVar5+0x30)) {
    if (piVar5 == this) { piVar5 = piVar5->next; continue; }
    rtti = (**(piVar5->vtable + 0x2C))();
    if (rtti == 6) break;   // found a building in this cell
}
// If cell contains a UnitRepair or UnitReload building:
if (piVar5 != NULL && (UnitRepair=yes OR UnitReload=yes in piVar5.Type)) {
    // Locomotor suspension logic:
    if (param_1[0x19D] == 0) Assert(0x80004003);   // locomotor must exist
    cVar1 = (**(loco_vtable + 0x60))(loco);        // locomotor is-suspended?
    if (!cVar1 && !FUN_0053a130()) {               // FUN_0053a130 always returns 0
        (**(loco_vtable + 0x58))(loco);             // suspend locomotor
    }
    // Compare destinations:
    piVar4 = FootClass__GetDestination(0);   // 0x0065AD30
    if (piVar4 == piVar5) {    // destination IS the repair building
        piVar5 = FootClass__GetDestination(0);
        if (param_2 != piVar5) {    // new destination differs from current
            (**(this->vtable + 0x274))(3);   // OVER_AND_OUT — break link
        }
    }
}
```

### Stage 6 — TAIL_CALL (0x0041ADAC)

```c
FootClass__Set_Destination_Internal(param_2, param_3);
return;
```

All paths eventually reach this tail. `FootClass::Set_Destination_Internal` at `0x004D94B0` commits the destination.

---

## 4. Transmit_Radio Call Inventory

| Address    | Slot       | Offset   | Msg Code | Msg Name    | Target    | Gate                          | Reply Handling |
|------------|------------|----------|----------|-------------|-----------|-------------------------------|----------------|
| 0x0041ABD6 | vtable+0x278 | 0x278  | 0x0E     | CAN_DOCK    | param_2 (building) | No-path branch + free slot found | iVar2==1 → return; else → OVER_AND_OUT |
| 0x0041ABE7 | vtable+0x274 | 0x274  | 0x03     | OVER_AND_OUT | Contacts[0] | After rejected 0x0E in no-path branch | ignored |
| 0x0041AC81 | vtable+0x274 | 0x274  | 0x0E     | CAN_DOCK    | Contacts[0] | Path-valid branch + not yet in Contacts | iVar2==1 → return; else → OVER_AND_OUT |
| 0x0041AC96 | vtable+0x274 | 0x274  | 0x03     | OVER_AND_OUT | Contacts[0] | After rejected 0x0E in path-valid branch | ignored |
| 0x0041ADA6 | vtable+0x274 | 0x274  | 0x03     | OVER_AND_OUT | Contacts[0] | Cell-scan: dest changed while at repair pad | ignored |

**Helipad sub-path (Stage 4a, no-path branch, when Helipad=yes):**

| Address    | Slot       | Offset | Msg Code | Msg Name   | Target  | Gate | Reply Handling |
|------------|------------|--------|----------|------------|---------|------|----------------|
| 0x0041AB62 | vtable+0x278 | 0x278 | 0x0F    | CAN_ENTER  | piVar3 (nearest friendly airfield or found pad) | Helipad=yes, after SetDestination clears + nearest airfield search | iVar2==1 → Transmit(0x02, piVar3) then SetMission(7 or 2) |
| 0x0041AB73 | vtable+0x278 | 0x278 | 0x02    | HELLO      | piVar3 | After CAN_ENTER accepted | ignored |

**Summary:**
- This function sends `CAN_DOCK(0x0E)` and `OVER_AND_OUT(0x03)` to buildings. It does NOT send `NEED_TO_MOVE(0x13)`, `MOVE_TO_CELL(0x12)`, `ENTER_DOCK(0x18)`, or `TIMING_SYNC(0x16)` — those are all sent by the BUILDING's `Receive_Radio` case 0x0E, which replies to this function's `CAN_DOCK` request.
- No `DOCKING_COMPLETE(0x07)` or `LEAVE_DOCK(0x19)` is sent from this function.

---

## 5. State Transitions on `this` (Field Writes)

| Assembly Offset | Field Offset | Value Written | Condition |
|-----------------|-------------|---------------|-----------|
| 0x0041ABC4      | `+0x500` (= `param_1[0x140]`)  | `param_2` (building ptr) | No-path + no-free-slot + UnitRepair=yes |
| 0x0041AB4C      | via vtable+0x480 call = `0x004D94A0` stub | 0 | Helipad branch: `Set_Destination(NULL, 1)` clears destination |
| 0x0041AB90      | via vtable+0x1E8 | mission code (2 or 7) | Helipad branch: SetMission(2=RETURN or 7=ENTER) |

**Notable absent writes:** No write to `+0x84` (DockLink), no write to `+0x418` (has-destination bool), no write to `+0x6AF` (chrono flag), no write to `+0x2E4`. These are not modified here; destination commitment is delegated entirely to `FootClass::Set_Destination_Internal`.

---

## 6. Destination/Cell Calculation

This function does NOT compute a dock queue cell or any foundation-relative offset. There is no `MapClass::Get_CellClass`, no anchor+offset math, and no `DockingOffset` lookup. The destination cell computation for refinery approach (the `NW_cell + (3,1)` formula from `BuildingClass::Receive_Radio` case 0x0E) is NOT in this function.

The only destination-related action is:
- `param_2 = NULL` to abort (in various rejection paths)
- `param_2 = piVar3` (nearest friendly airfield result) in the helipad branch

`FootClass::Set_Destination_Internal(param_2, param_3)` at the tail commits whatever `param_2` remains.

---

## 7. Q1/Q2 Cross-Check: Does This Function Emit 0x07 or 0x19?

**Q1 — Does this function transmit `DOCKING_COMPLETE(0x07)`?** NO.
No message code 0x07 appears anywhere in the body. Verified by full disassembly scan.

**Q2 — Does this function transmit `LEAVE_DOCK(0x19)`?** NO.
No message code 0x19 appears in the body. Verified by full disassembly scan.

Both of these messages are sent by `TechnoClass::Receive_Radio` case 0x08 as documented in `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`.

---

## 8. Helipad Sub-Path Detail (Stage 4a)

When the destination building has `Helipad=yes` (`Type+0x16CB != 0`):

```c
// Get a nearby landing pad: this+0x6C4 = TypeClass ptr; +0x3E8 = type-specific offset
piVar3 = (**(vtable+0x528))(this+0x6C4 + 0x3E8, 0, 0);  // some "find nearest" function
(**(vtable+0x480))(0, 1);   // Set_Destination(NULL, force=1) — clear current dest
if (piVar3 != NULL) {
    iVar2 = (**(vtable+0x278))(0xF, piVar3);   // Transmit_Radio(CAN_ENTER, piVar3)
    if (iVar2 == 1) {
        (**(vtable+0x278))(2, piVar3);          // Transmit_Radio(HELLO, piVar3)
        mission = 7;                            // Mission_Enter
    }
} else {
    piVar3 = AircraftClass__Find_Nearest_Friendly_Airfield(); // 0x0041A160
    mission = 2;                               // Mission_Return
}
(**(vtable+0x1E8))(mission, 0);    // SetMission(7 or 2)
cVar1 = (**(vtable+0x200))();      // IsMoving? or HasPath?
if (cVar1) (**(vtable+0x1EC))();   // StopMoving / ClearPath
```

**Active in YR:** YES — used by aircraft landing at helipads.
**Does NOT relate to harvester/refinery.** The helipad flag check (`0x16CB`) is `Helipad=yes`, not `DockUnload=yes` or `Refinery=yes`.

---

## 9. Diffs vs Phase 1 Slot 2 (BuildingClass::Receive_Radio switch)

The prior doc (`BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`) documents the BUILDING-side receipt of `0x0E CAN_DOCK`. This function sends `0x0E` to a building from the aircraft side. The pairing:

| This function (aircraft/sender) | Building Receive_Radio case 0x0E (receiver) |
|---------------------------------|---------------------------------------------|
| Calls `Transmit_Radio(0x0E, building)` | Receives 0x0E |
| Checks reply: ROGER(1) → return; else OVER_AND_OUT | Sends back: NEED_TO_MOVE(0x13) → MOVE_TO_CELL(0x12) → ENTER_DOCK(0x18) → TIMING_SYNC(0x16) |
| Only for AircraftClass approach to helipad/repair | Only for unit/vehicle approach to refinery/helipad/repair |

The building's reply chain sends MOVE_TO_CELL back to the **aircraft** via `Transmit_Radio_Impl(0x12, &cell, aircraft)`. The aircraft's `Receive_Radio` (at vtable+0x194, NOT this function) handles those inbound messages. This function is the **outbound initiator** only.

---

## 10. PRIMARY OPEN QUESTIONS — FINAL STATE

### Q: Is this function the harvester-side 0x0E sender?

**NO.** This is the AIRCRAFT-side `Set_Destination` override. The label `UnitClass__EnterBuildingOrDock` is wrong — this is `AircraftClass::Set_Destination`. The harvester-side dock initiation is driven differently.

### Q: When is this function called?

On each destination assignment to an AircraftClass object. Called from vtable+0x480 dispatch — `TechnoClass::Set_Destination` (0x741970), `FootClass::Mission_Enter` (0x4D9290), and per-cell choreography functions. Not per-tick; per destination-change event only.

### Q: Where is the actual harvester-side 0x0E sender?

The harvester (a UnitClass) sends `0x0E CAN_DOCK` via `(*vtable+0x278)(0x0E, building)` in `TechnoClass::Set_Destination` (0x741970) during the `Mission_Enter` approach. The relevant logic in `TechnoClass::Set_Destination` (verified in the decompile this session):

```c
// In TechnoClass::Set_Destination, mission==7 (Mission_Enter), no existing path, destination is building:
iVar6 = (**(this->vtable + 0x278))(0xE, piVar10);   // Transmit_Radio(0x0E, building)
if (iVar6 == 1) {
    // ROGER — dock accepted; building has already sent MOVE_TO_CELL etc.
    if (DockUnload=yes && contacts changed) {
        param_2 = new_dest_from_contacts;
    }
} else {
    (**(this->vtable + 0x274))(3);     // OVER_AND_OUT
    if (UnitRepair=yes) SetGhostCell();
}
```

For harvesters specifically, `UnitClass::PerCellProcess` (0x739EC0) manages the step-by-step choreography after the initial approach.

### Q: What about the "NEED_TO_MOVE → MOVE_TO_CELL → ENTER_DOCK → TIMING_SYNC" chain from the sender side?

This function does NOT initiate or handle those messages. Those are sent by `BuildingClass::Receive_Radio` case 0x0E as replies back to the approaching unit. The approaching unit (aircraft or harvester) receives them via vtable+0x194 (Receive_Radio), NOT via this function. The chain is fully documented in `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`.

### Q: Does this function handle queued (0x17) reply by entering wander/wait?

No. The only reply codes checked are ROGER(1) at `0x0041ABDC` and `0x0041AC87`. A reply of 0x17 (QUEUED) falls through the `iVar2 != 1` branch and results in OVER_AND_OUT. There is no wander/wait state entered from this function. The QUEUED handling (FUN_00500200 wander helper) would be in the per-tick mission handler, not here.

### Q: `FootClass::Mission_Enter` vs `EnterBuildingOrDock` relationship?

`FootClass::Mission_Enter` (`0x004D9290`) calls vtable+0x480 (`Set_Destination`) on the unit. For AircraftClass, vtable+0x480 dispatches to THIS function (`0x0041AA80`). For UnitClass/harvester, vtable+0x480 dispatches to `TechnoClass::Set_Destination` (0x741970) — UnitClass does not override this slot. So for harvesters: `FootClass::Mission_Enter` → `TechnoClass::Set_Destination` → 0x0E radio sent.

---

## 11. Tiny Details

- **`FUN_0053a130` at `0x0053A130`** = stub that always returns 0 (verified via `decompile_function 0x0053A130`). The `if (!loco_suspended && !FUN_0053a130())` branch in the cell-scan is therefore always entered when the locomotor is not suspended.
- **`DynamicVectorClass__Contains` at `0x0065AD50`** = walks `this+0xE4` array up to `this+0xE8` count, returns 1 if param_2 found (verified via `decompile_function 0x0065AD50`).
- **`FUN_0065adf0` at `0x0065ADF0`** = FindFreeContactSlot — walks `param_1->Contacts[]` (`+0xE4`, capacity `+0xE8`); returns 1 if a zero slot OR a slot matching `param_2` is found (verified via `decompile_function 0x0065ADF0`).
- **`Filter_AbstractType_InMap` at `0x0040DD70`** = accepts RTTI types 1 (Unit), 2 (Aircraft), 6 (Building), 0xF (Infantry); returns NULL for others (verified via `decompile_function 0x0040DD70`).
- **`PathType::Has_Valid_Steps` at `0x0065AE30`** — despite its name, walks `this->Contacts[]` (`+0xE4`, `+0xE8`), returning 1 if any contact is non-NULL. Functionally = HasRadioContact. This label appears to be a mislabel.
- **`0x0041ABCA: XOR EDI,EDI`** — sets `param_2 = NULL` after writing `param_2` to `this+0x500`. This is the unconditional dest-clear after storing the UnitRepair building pointer.
- **`this+0x500` = offset `0x140*4` = `param_1[0x140]`** — stores the UnitRepair building pointer when the aircraft arrives at a repair pad with no free slot.
- **`this+0x5A4` = `param_1[0x169]`** — an additional stored building pointer checked in the OVER_AND_OUT path; used as alternate target for SetGhostCell.
- **`this+0x674`** = locomotor COM pointer (from constructor `006f3367`). Vtable+0x60 = Is_Suspended, vtable+0x58 = Suspend.
- **`GameDebugLog::Assert(0x80004003)`** fires if locomotor ptr is NULL when needed — E_POINTER guard.
- **Cell flag `(*(cell+0x140) & 0x100)`** = bridge/water occupancy flag — skips the cell-scan step when set.
- **Write order in helipad branch:** (1) Set_Destination(NULL) via vtable+0x480, (2) Transmit CAN_ENTER(0x0F), (3) Transmit HELLO(0x02) if accepted, (4) SetMission(7 or 2), (5) clear path if IsMoving.

---

## 12. Open Questions — Final State

| # | Question | Status |
|---|----------|--------|
| 1 | What is `vtable+0x528`? (called on `this` in helipad branch to find a landing pad) | OPEN — unlabeled function |
| 2 | Exact semantics of `this+0x500` — is this a "pending repair pad" field? | PROBABLE YES — stores repair-building ptr when no free slot |
| 3 | What does `FootClass::GetDestination(0)` vs `FootClass::GetDestination(non-0)` return? | Partial — `decompile_function 0x0065AD30` shows it reads `*(this+0xE4 + param_2*4)`, i.e. Contacts[param_2] |
| 4 | Why does the no-path branch call `FUN_0065adf0` with `param_2` (building) as receiver vs path-valid branch which passes `this` (aircraft)? | OPEN — asymmetry suggests different free-slot check targets |
| 5 | Which function IS the actual harvester-side per-tick CAN_DOCK sender? | ANSWERED: `TechnoClass::Set_Destination` (0x741970) for initial dock initiation; `UnitClass::PerCellProcess` (0x739EC0) for choreography |

---

## Sources

All claims verified via Ghidra MCP in this session:

- `decompile_function 0x0041AA80` — full body decompile
- `disassemble_function 0x0041AA80` — full assembly
- `get_xrefs_to 0x0041AA80` → `From 007e2724 [DATA]`
- `read_memory 0x007E2724, 4` → `80 AA 41 00` = `0x0041AA80` (vtable slot)
- `read_memory 0x007E22A4, 32` — AircraftClass primary vtable start
- `disassemble_function 0x00413D20` (AircraftClass ctor) → `[ESI] = 0x7E22A4`
- `disassemble_function 0x007353C0` (UnitClass ctor) → `[ESI] = 0x7F5C70`
- `disassemble_function 0x004D31E0` (FootClass ctor) → `[ESI] = 0x7E8C94`
- `read_memory 0x007F60F0, 8` → `0x00741970` = TechnoClass::Set_Destination at UnitClass vtable+0x480
- `read_memory 0x007E9114, 16` → `0x004D94A0` at FootClass vtable+0x480
- `decompile_function 0x0065ADF0` — FUN_0065adf0 = FindFreeContactSlot
- `decompile_function 0x0065AE30` — PathType::Has_Valid_Steps (contacts check)
- `decompile_function 0x0040DD70` — Filter_AbstractType_InMap
- `decompile_function 0x0053A130` — stub returning 0
- `decompile_function 0x0065AD50` — DynamicVectorClass::Contains
- `decompile_function 0x0065AD30` — FootClass::GetDestination
- `decompile_function 0x004D94A0` — FUN_004D94A0 (simple setter)
- `decompile_function 0x004D9290` — FootClass::Mission_Enter
- `decompile_function 0x00741970` — TechnoClass::Set_Destination (confirms harvester 0x0E send path)
- `get_function_by_address 0x004176F0` → AircraftClass::Enter_Idle_Mode (adjacent vtable slot confirms AircraftClass vtable)

Prior art cross-referenced:
- `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`
- `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`
- `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`
