# TechnoClass::Set_Destination (0x00741970) — Harvester-Side Sender (CORRECTED)

**Address:** 0x00741970  
**Identity verification:** UnitClass vtable+0x480 = `read_memory(0x007F60F0)` → `70 19 74 00` = 0x00741970. Ghidra labels it `TechnoClass__Set_Destination` but the active slot is the UnitClass override; the TechnoClass-level slot is a pure stub at 0x00709A30.  
**Confidence:** HIGH on identity (vtable-read verified), HIGH on radio inventory (all CALLs confirmed by address scan), HIGH on gate conditions (full disassemble read), MEDIUM on TypeClass field semantics at +0xDFC (name not resolved in this session)  
**Active in YR:** YES — called whenever any UnitClass object's destination is set  
**Report date:** 2026-05-20  
**Supersedes:** "UnitClass::EnterBuildingOrDock" plan citation (that label was on 0x0041AA80 which is AircraftClass::Set_Destination)  
**Correction (2026-07-13):** This doc originally glossed the `0x00741afc: CMP EAX,0xf` compare in Stage 3 as "RTTI==0xF (CellClass)". That was WRONG — the authoritative byte-verified RTTI table (`RTTI_WHATAMI_TYPEID_RECONCILE_GHIDRA_REPORT.md`) establishes `0xF = InfantryClass` and `0xB = CellClass`. Corrected in place below (§1 item 4, §4 Stage 3 gate list, radio table, Open Question 6). Open Question 6 (RTTI==2 identity) is now RESOLVED: RTTI 2 = AircraftClass, confirmed via `read_memory 0x0041c180` = `MOV EAX,0x2; RET` this session.

**Correction (2026-07-19):** A *second* RTTI mislabel — the Stage-4 destination-filter blocks glossed `RTTI==1` as `(CellClass)` (§1 item 5, §4 pseudo + sub-block heading, radio-table rows 0x741E5C–0x741E80, §7 state row 0x741CB8). `1 = UnitClass`, not CellClass (`0xB = CellClass`). Byte-verified this session (independent of the doc): `read_memory 0x00746e20` = `B8 01 00 00 00 C3` (`MOV EAX,0x1; RET` = `UnitClass::What_Am_I`), owned via `UnitClass` vtable+0x2C (`read_memory 0x007F5C9C` = `20 6e 74 00` = 0x00746e20); `read_memory 0x00487e60` = `B8 0B 00 00 00 C3` (`CellClass::What_Am_I`), owned via `CellClass` vtable+0x2C (`read_memory 0x007E4F18` = `60 7e 48 00` = 0x00487e60). The `What_Am_I()==1` branch stores the destination as a pending **UnitClass** target (+0x51C) and ghosts — it is NOT a cell path. Corrected in place below. **Also resolved this pass (was flagged in `RTTI_WHATAMI_TYPEID_RECONCILE_GHIDRA_REPORT.md` §5):** §Stage-3 item 6 and the §4 outer gate previously called the `vtable+0x184` result `What_Am_I()` and glossed `==0` as UnitClass. `+0x184` is NOT the `+0x2C` What_Am_I slot — it is `MissionClass::GetCurrentMission` (`0x005B3040`, reads `+0xAC` then `+0xB4`), a mission getter returning 0/7/0x10/0x14. Byte-verified: `read_memory 0x007F5DF4` (UnitClass vtable+0x184) = `40 30 5b 00` = 0x005B3040. Every such `What_Am_I()==0/==7` gloss corrected to `Get_Current_Mission()`; the bogus "`RTTI 0 = UnitClass`" reading is retracted (0 is mission 0, not an RTTI type).

---

## 1. Overview

`0x00741970` is the UnitClass-level implementation of the `Set_Destination` virtual (vtable+0x480). It is called once per destination change — not per tick. Its responsibilities, in order:

1. **Chrono-in-flight self-redirect** — if `this` has a chrono timer active and `param_2 == NULL` and an existing radio contact exists, compute a timing offset and possibly call `SetMission(Harvest)` or `StopMoving`.
2. **Early-out guard** — if `param_2 == existing destination` AND the "has-destination" flag (`+0x1f8`) is clear, return immediately (no-op).
3. **Stop-moving guard** — if locomotor flags indicate no movement capability, call `FootClass::Stop_Moving` and return.
4. **LEAVE_DOCK(0x19) cancel-docking path** — if there is already a radio contact AND `Get_Current_Mission()==0` (current mission is 0, via vtable+0x184 — NOT What_Am_I) AND `+0x418 != 0` AND the contact is RTTI==2 (AircraftClass — corrected 2026-07-13: was "Aircraft?"; confirmed via `AircraftClass::What_Am_I` raw bytes at 0x0041c180 = `MOV EAX,0x2; RET`, verified via `read_memory 0x0041c180` — RTTI table reconciled, see RTTI_WHATAMI_TYPEID_RECONCILE_GHIDRA_REPORT.md) AND the contact's TypeClass flag at +0xDFC is set: call `Set_Destination(NULL, 1)` on the contact (via vtable+0x480), then send `LEAVE_DOCK(0x19)` followed by `OVER_AND_OUT(0x03)` to the radio link. This is the "abort dock while approaching" path.
5. **Mission=7 (Mission_Enter) gate** — if `Get_Current_Mission()==7` OR `this->mission==7` AND `PathType::Has_Valid_Steps()==false`:
   - If the filtered dest is RTTI==1 (UnitClass) → store as pending unit-target (+0x51C) and ghost. Else if the dest filters to a bare/occupied cell (not a building, not a unit) with `IsOccupied` set → `HELLO(0x02)` dock approach.
   - Else if new dest is RTTI==6 (Building) AND no existing valid path → **send `CAN_DOCK(0x0E)` via vtable+0x278 (Transmit)**
   - Handle ROGER vs OVER_AND_OUT paths accordingly
6. **Locomotor swap** — if `Teleporter=` (`TechnoType+0xCD4`) and conditions are met, swap locomotor to `DriveLocomotion` or `TeleportLocomotion` (chrono outbound travel logic). Do not confuse this with `TechnoType+0xD6A`, which is now verified as `BalloonHover=`.
7. **Hover locomotor special** — if piggyback is HoverLocomotion and IsInAir and destination is passable: swap locomotor to DriveLocomotion, rebuild path.
8. **UnitRepair / UnitReload target sub-block** — HELLO(0x02) + CAN_DOCK(0x0E) sequence for repair/reload buildings.
9. **Current-destination-is-a-transport loop** — walks TypeClass array looking for transport carrier; if found and approach not already in progress: HELLO(0x02) → if ROGER then CAN_DOCK(0x0E) via vtable+0x274 → SetMission(7) or SetGhostCell.
10. **Locomotor suspend scan** — final cell-scan for UnitRepair/UnitReload building in the current cell; if found, suspend locomotor.
11. **Tail call** → `FootClass::Set_Destination_Internal(0x004D94B0)`.

---

## 2. Identity Verification — UnitClass vs TechnoClass vs FootClass vtable+0x480

All four readings verified in this session:

| Class | vtable base | slot +0x480 address | function | Notes |
|-------|------------|---------------------|----------|-------|
| UnitClass | 0x007F5C70 | `0x007F60F0` | 0x00741970 | `read_memory(0x007F60F0, 4)` → `70 19 74 00` |
| TechnoClass | 0x007F4960 | `0x007F4DE0` | 0x00709A30 | `read_memory(0x007F4DE0, 4)` → `30 9A 70 00` — pure stub `{ return; }` via `decompile_function 0x00709A30` |
| FootClass | 0x007E8C94 | `0x007E9114` | 0x004D94B0 | `read_memory(0x007E9114, 4)` → `B0 94 4D 00` = 0x004D94B0 = `FootClass__Set_Destination_Internal` (corrected 2026-05-29: was 0x004D94A0; `B0 94 4D 00` LE = 0x004D94B0 not 0x004D94A0 — hex misread; `0x004D94A0` is an unrelated one-instruction setter FUN_004d94a0 that writes `param_1+0x5A0`; verified via `read_memory 0x007E9114` + `get_function_by_address 0x004D94B0` — PARAM1_TYPE_MISREAD) |
| AircraftClass | 0x007E22A4 | `0x007E2724` | 0x0041AA80 | Phase 2 slot 1 verified; `read_memory(0x007E2724)` → `80 AA 41 00` |

**Conclusion:** `0x00741970` is the UnitClass-level override of `Set_Destination`. TechnoClass has a stub at this slot (no-op); UnitClass provides the real implementation. FootClass has a simple setter. AircraftClass has a full aircraft-dock handler (Phase 2 slot 1 doc). Since UnitClass overrides the slot, UnitClass objects (including harvesters) use `0x00741970` when called through vtable+0x480.

The label `TechnoClass__Set_Destination` in Ghidra is a slight mislabel — the TechnoClass-level slot is the stub at `0x00709A30`. The function at `0x00741970` is properly the UnitClass override.

---

## 3. Callers (Who Invokes This, When?)

`get_function_callers 0x00741970` returned "no callers" — all callers dispatch through vtable+0x480, not via direct CALL. The vtable entry at `0x007F60F0` is the only DATA xref.

This is a **one-shot call** per destination change, not per tick. Key dispatch sites:
- `FootClass::Mission_Enter` (`0x004D9290`) calls vtable+0x480 each time it assigns a destination. For UnitClass objects this dispatches here.
- Any code assigning a destination to a UnitClass via the virtual: right-click commands, waypoint assignment, player orders from the sidebar.
- `UnitClass::PerCellProcess` (`0x00739EC0`) calls vtable+0x480 for per-cell dock choreography.
- Per `TechnoClass::Set_Destination` decompile: `(**(vtable+0x480))(0, 1)` is called on sub-objects (e.g., chrono contact's Set_Destination) from within this function itself, dispatching back here recursively for UnitClass targets.

---

## 4. Decompile Walk-Through

**Signature** (from `decompile_function 0x00741970`; `param_1` = `ObjectClass*`, used as `int*` with direct byte offsets):

```c
void __thiscall TechnoClass__Set_Destination(
    ObjectClass *param_1,   // ECX = this (UnitClass instance)
    int *param_2,           // [ESP+4] = new destination (building, cell, or NULL)
    char param_3            // [ESP+8] = force-flag (passed down to Set_Destination_Internal)
);
```

### Stage 0 — Chrono self-redirect guard (0x00741970–0x00741a7F)

```
[EBP + 0x6c4] = TypeClass ptr
TypeClass[0xD6A] = `BalloonHover=` flag (checked as first gate for a null-destination intercept)
```
- Gate: `TypeClass[0xD6A] != 0` (`BalloonHover=yes`) AND `param_2 == NULL` AND `this->Contacts[0] != NULL` AND `this->SomeField2B4 != NULL`
- If gate passes: compares contacts, calls `RateTimer__Current` and `FUN_005F3DB0` (timer difference calc), calls `FUN_004D03D0` — if that returns non-zero, checks `What_Am_I() == 1` (Infantry?), then calls `vtable+0x1f0` (Stop) or `vtable+0x1e8(7,0)` (SetMission Enter), then returns.
- Verified via `decompile_function 0x00741970` — lines 0x741970–0x741a7F.
- **Active in YR:** YES (conditional on BalloonHover UnitClass runtime state; stock `DISK` is a relevant UnitClass example)

### Stage 1 — Early-out if no change (0x00741a80–0x00741a9E)

```c
if (param_2 == this->field_0x5a4 && this->field_0x1f8 == 0) return;
this->field_0x1f8 = 0;  // clear flag unconditionally
```
- `+0x5a4` = current destination pointer (verified via read)
- `+0x1f8` = has-destination flag
- Verified via assembly lines 82–88: `CMP EBX, [EBP+0x5a4]`, `MOV byte[EBP+0x1f8], 0x0`

### Stage 2 — Stop-moving guard (0x00741a96–0x00741ad0)

```
+0x6E0 = moving flag 1, +0x6E1 = moving flag 2, +0x6E2 = moving flag 3
+0x2B0 = locomotor-exists ptr
```
- If `+0x6E0 == 0` AND (`+0x6E1 != 0` OR `+0x6E2 != 0`) → `FootClass::Stop_Moving()` → return.
- If `+0x6E0 != 0` AND `+0x2B0 == 0` → `FootClass::Stop_Moving()` → return.
- Verified via assembly lines 87–104.

### Stage 3 — LEAVE_DOCK(0x19) cancel-dock path (0x00741ad1–0x00741bA9)

Gate chain:
1. `PathType__Has_Valid_Steps()` → has radio contact (cVar4 != 0)
2. `[EBP + 0x6c4]->field_0x5e0 > 0` — some timer/count field
3. `FootClass__GetDestination(0)` RTTI == 0xF (InfantryClass — corrected 2026-07-13: was CellClass; 0xF=InfantryClass per InfantryClass__What_Am_I 0x00523340 = MOV EAX,0xf, verified via disassemble_function — RTTI table reconciled, see RTTI_WHATAMI_TYPEID_RECONCILE_GHIDRA_REPORT.md) — existing contact is an InfantryClass object
4. `param_2 != NULL` AND `(param_2+0x14) & 1 != 0` — new destination is occupied cell
5. `PathType__Has_Valid_Steps()` != 0 — already has valid steps
6. `Get_Current_Mission() == 0` — current mission is 0. This is `vtable+0x184` = `MissionClass::GetCurrentMission` (`0x005B3040`; reads `+0xAC`, falls back to `+0xB4`), a MISSION getter — NOT `What_Am_I`/RTTI. The earlier "RTTI 0 = UnitClass" reading here was wrong (byte-verified 2026-07-19: `read_memory 0x007F5DF4` = `40 30 5b 00` = 0x005B3040)
7. `+0x418 != 0` — has-pending-destination flag
8. `FootClass__GetDestination(0)` RTTI == 2 — contact is RTTI 2 (AircraftClass — corrected 2026-07-13: was "Aircraft? or some type code"; confirmed via `read_memory 0x0041c180` = `MOV EAX,0x2; RET` — Open Question 6 below is RESOLVED)
9. Contact's `TypeClass[0xDFC] != 0` — some INI flag on the contact's type

If ALL gates pass:
- Call `(contact.vtable+0x480)(0, 1)` = Set_Destination(NULL, force=1) on the contact
- `Transmit_Radio_ToFirst(0x19)` — send LEAVE_DOCK to own radio link
- `Transmit_Radio_ToFirst(0x03)` — OVER_AND_OUT
- Fall through

Assembly at 0x741b42–0x741bA4 confirmed via disassemble read.

### Stage 4 — RTTI checks and Mission_Enter dock gate (0x00741bAA–0x00741e8F)

This is the main building-approach block:

**Outer gate (0x741c4F–0x741c78):**
- `Get_Current_Mission() == 7` (vtable+0x184, MissionClass::GetCurrentMission) OR `this->field_0xB4 == 7` (this->mission == 7 = Mission_Enter)
- `PathType__Has_Valid_Steps() == false` (no existing path/radio contact)

If NOT in Mission_Enter: skip to Stage 6 (write `param_1[8].{NeedsRedraw, InLimbo, ...}` flags).

**Main dock block (0x741c7E–0x741e8E):**

```
// Filter destination
piVar11 = Filter_AbstractType_InMap(param_2);  // 0x0040DD70
if piVar11 == NULL || RTTI(piVar11) != 1:    // dest is NOT a UnitClass (bare cell → HELLO sub-block, or building → CAN_DOCK below)
    ... (piVar10==NULL → occupied-cell HELLO(0x02) sub-block)
else:                                        // RTTI(piVar11)==1 → UnitClass dest: store pending unit-target (+0x51C), SetGhostCell, param_2=NULL
// RTTI==6 Building case:
ESI = filtered dest (building)
EDI = this->field_0x5a4 (current dock target)
PUSH ESI; PUSH 0xe; Transmit(vtable+0x278)(0xE, ESI)  // CAN_DOCK
```

CAN_DOCK send is at **0x741DDA** (vtable+0x278, Transmit):
```asm
00741dcc: MOV EAX,dword ptr [EBP]
00741dcf: MOV EDI,dword ptr [EBP + 0x5a4]
00741dd5: PUSH ESI          ; ESI = target building
00741dd6: PUSH 0xe          ; CAN_DOCK
00741dd8: MOV ECX,EBP
00741dda: CALL dword ptr [EAX + 0x278]   ; Transmit_Radio(0xE, building)
00741de0: CMP EAX,0x1       ; ROGER?
00741de3: JZ 0x00741e10     ; yes → dock accepted
```

**On ROGER (0x741e10):**
```asm
00741e10: MOV ECX,[ESI + 0x520]    ; TypeClass of building
00741e16: MOV AL,[ECX + 0x16b3]   ; DockUnload=yes
00741e1c: TEST AL,AL
00741e1e: JZ 0x00741e8e            ; if not DockUnload, skip
00741e20: MOV EAX,[EBP + 0x5a4]   ; current dock target
00741e26: CMP EAX,EDI              ; did dock target change?
00741e28: JZ 0x00741e8e            ; no change — skip
00741e2a: MOV EBX,EAX
00741e2c: MOV [ESP+0x94],EBX       ; update param_2 to new contact
```

So when the building replies ROGER: if the building has `DockUnload=yes` (TypeClass+0x16B3) AND the radio contact changed (Contacts[0] is different from before), update `param_2` to the newly established contact's address. This lets the building's `Receive_Radio(0x0E)` reply set where we drive.

**On NOT-ROGER (0x741de5):**
```asm
00741de5: MOV EDX,[EBP]
00741de8: PUSH 0x3                 ; OVER_AND_OUT
00741dea: CALL [EDX + 0x274]       ; Transmit_ToFirst(0x03)
00741df2: MOV EAX,[ESI + 0x520]
00741df8: MOV CL,[EAX + 0x16a9]   ; UnitRepair=yes
00741dfe: TEST CL,CL
00741e00: JZ 0x00741e8e
00741e06: PUSH EBX
00741e07: CALL TechnoClass__SetGhostCell  ; ghost if UnitRepair
```

**Occupied-cell sub-block (LAB_00741e35, 0x741e35–0x741e88):** reached when the filtered dest is NOT a UnitClass and `piVar10==NULL` (no building); `param_2` is an occupied cell. HELLO+CAN_DOCK for entering occupied cells: vtable+0x278 for HELLO(0x02), then vtable+0x274 for CAN_DOCK(0x0E) via ToFirst. Not the refinery path — and NOT triggered by RTTI==1 (that is the UnitClass-dest store-and-ghost else branch).

### Stage 5 — Write 0x5e0 (0x00741e88)

```asm
00741e88: MOV [EBP + 0x5e0], EDI
```

This writes the *previous* dock target (saved in EDI before the CAN_DOCK attempt) back into `+0x5e0`. This field appears to track a "reserved dock" pointer or approach-tick counter.

### Stage 6 — Locomotor handling (0x741e95–0x742b.ff)

Complex locomotor-swap logic for teleporter/chrono units (TeleportLocomotion ↔ DriveLocomotion) and hover units (HoverLocomotion → DriveLocomotion). Uses `IID_IPiggyback` COM queries. This is the chrono outbound locomotor assignment — swaps to TeleportLocomotion if heading to a cell with no building obstructing, enters `Mission_Enter(7)` and sets `+0x6AC=1`, `+0x1f8=1`. Detailed chrono locomotor swap runs only when `TypeClass[0xCD4] != 0` (`Teleporter=`).

### Stage 7 — UnitRepair/UnitReload HELLO+CAN_DOCK (0x00742C18–0x00742D3E)

For UnitRepair=yes (`TypeClass[0x16A9]`) and UnitReload=yes (`TypeClass[0x16AB]`) buildings found in the sweep:

```asm
00742c59: PUSH EBX; PUSH 0x2; Transmit_Broadcast(0x278)(0x02, EBX)  ; HELLO
00742c67: CMP EAX,0x1  ; ROGER?
00742c6f: PUSH 0xe; Transmit_ToFirst(0x274)(0x0E)                   ; CAN_DOCK
00742c79: CMP EAX,0x1  ; ROGER?
; → if ROGER: call FootClass::Set_Destination_Internal(0x004D94B0) directly, set +0x5e0 = -1, return
```

Also a second UnitReload block at 0x742cdb–0x742d3e with same pattern.

### Stage 8 — Transport carrier loop (0x742D46–0x742E37)

Walks `TypeClass` array looking for carrier transports. If found and approach not already in progress:
- `Transmit_Broadcast(0x278)(0x02, target)` — HELLO
- If ROGER: `Transmit_ToFirst(0x274)(0x0E)` — CAN_DOCK
- If ROGER: `vtable+0x1e8(7, 0)` = `SetMission(7 = Mission_Enter)` at 0x742F37
- Write `SetMission(7)` also at 0x742F35–0x742F37 (duplicate call found at 0x742F33 = `PUSH 0; PUSH 7; CALL vtable+0x1E8`)
- `Transmit_ToFirst(0x274)(0x03)` = OVER_AND_OUT in rejection paths

### Stage 9 — Locomotor suspend scan + tail (0x742F48–0x74316A)

- Read `+0x674` = locomotor COM ptr; if NULL, Assert(0x80004003)
- Call `loco->vtable+0x60` = Is_Suspended; if suspended → skip
- Call `FUN_0053A130()` (stub, always returns 0); if returns 1 → skip
- Get current cell via vtable+0x1BC; check cell flag `+0x140 & 0x100` (bridge); if set → skip
- Walk cell object list: for each object that is RTTI==6 AND (`UnitRepair=yes` OR `UnitReload=yes`): call `loco->vtable+0x58` = Suspend locomotor; break.
- **Tail call:** `FootClass::Set_Destination_Internal(0x004D94B0)(param_2, param_3)` at 0x743161.
- Alternate tail: `FootClass::Stop_Moving(0x004DF0D0)` at 0x743175 if early-out condition reached.

---

## 5. Radio Traffic Inventory

All CALLs confirmed by disassemble_function result read. Offset 0x274 = Transmit_Radio_ToFirst; 0x278 = Transmit_Radio (broadcast); 0x27C/0x280 not found in this function.

| Address | Slot | Offset | Msg Code | Msg Name | Target | Gate | Reply Handling |
|---------|------|--------|----------|----------|--------|------|----------------|
| 0x00741B22 | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | Cancel-dock path: already-in-contacts check fails (this already in destination's contact list) | ignored |
| 0x00741B97 | vtable+0x274 | 0x274 | 0x19 | LEAVE_DOCK | Contacts[0] | Has-contact + What_Am_I==0 + +0x418!=0 + contact RTTI==2 (AircraftClass, corrected 2026-07-13) + TypeClass[0xDFC]!=0 | ignored |
| 0x00741BA4 | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | Immediately after 0x19 | ignored |
| 0x00741D60 | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | UnitReload=yes (TypeClass[0x16AB]) check in path-valid branch | ignored |
| 0x00741DB8 | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | Health > threshold in UnitRepair health-check branch | ignored |
| **0x00741DDA** | **vtable+0x278** | **0x278** | **0x0E** | **CAN_DOCK** | **ESI = filtered building (refinery/repair/etc)** | **Mission==7, no existing path, dest RTTI==6 building** | **ROGER(1)→check DockUnload+contact-changed, update param_2; else OVER_AND_OUT** |
| 0x00741DEC | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | After CAN_DOCK rejected (non-ROGER) | ignored |
| 0x00741E5C | vtable+0x278 | 0x278 | 0x02 | HELLO | EBX = occupied-cell target | dest is an occupied cell (not a building/unit); no existing path | ROGER(1)→ send CAN_DOCK(0x0E) via ToFirst |
| 0x00741E6E | vtable+0x274 | 0x274 | 0x0E | CAN_DOCK | Contacts[0] | After HELLO accepted in occupied-cell path | ROGER(1)→return; else OVER_AND_OUT |
| 0x00741E80 | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | CAN_DOCK rejected in occupied-cell path | ignored |
| 0x00742C61 | vtable+0x278 | 0x278 | 0x02 | HELLO | EBX = UnitRepair building | UnitRepair=yes AND no existing path OR own contact | ROGER(1)→ CAN_DOCK via ToFirst |
| 0x00742C73 | vtable+0x274 | 0x274 | 0x0E | CAN_DOCK | Contacts[0] | After HELLO accepted for UnitRepair | ROGER(1)→Set_Destination_Internal+return; else OVER_AND_OUT |
| 0x00742C97 | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | CAN_DOCK rejected for UnitRepair | ignored |
| 0x00742CE3 | vtable+0x278 | 0x278 | 0x02 | HELLO | EBX = UnitReload building | UnitReload=yes path | ROGER(1)→ CAN_DOCK via ToFirst |
| 0x00742CF5 | vtable+0x274 | 0x274 | 0x0E | CAN_DOCK | Contacts[0] | After HELLO accepted for UnitReload | ROGER(1)→Set_Destination_Internal+return; else OVER_AND_OUT |
| 0x00742D2E | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | CAN_DOCK rejected for UnitReload | ignored |
| 0x00742E41 | vtable+0x274 | 0x274 | 0x03 | OVER_AND_OUT | Contacts[0] | Transport carrier loop: existing dest is different building | ignored |
| 0x00742EF7 | vtable+0x278 | 0x278 | 0x02 | HELLO | EBX = carrier | Transport carrier loop: same TypeClass, RTTI==1 (UnitClass this), carrier not in mission Enter/Harvest | ROGER(1)→ CAN_DOCK via ToFirst |
| 0x00742F37 | vtable+0x1E8 | 0x1E8 | 0x07 arg | SetMission(7) | self | After HELLO+CAN_DOCK ROGER in carrier loop | — (not radio) |

**Refinery-specific summary:** The refinery dock send is the single `Transmit_Radio(0x0E, building)` at **0x00741DDA** via vtable+0x278. All other 0x0E sends use vtable+0x274 (ToFirst) after a preceding HELLO(0x02) accepted.

---

## 6. Q1–Q4 Answers

### Q1: Does it send 0x0E CAN_DOCK?

**YES.** Multiple sites (see table above). The primary refinery-dock send is at **0x00741DDA**: vtable+0x278 (Transmit, not ToFirst), message 0x0E, target is the filtered building (refinery), under gate: `Get_Current_Mission()==7 OR mission==7 (Mission_Enter)` AND `PathType::Has_Valid_Steps()==false` AND `RTTI(dest)==6 (Building)`.

This is the **initial dock initiation** — when the harvester has no existing radio path to the refinery and is being sent there. On ROGER, the building's Receive_Radio(0x0E) has already replied with MOVE_TO_CELL etc. and established the radio link; the harvester then checks if `DockUnload=yes` (TypeClass+0x16B3) and the contact pointer changed, updating `param_2` accordingly.

Cross-check with Phase 2 slot 1 (AircraftClass::Set_Destination at 0x0041AA80): that function also sends 0x0E in its aircraft-dock path — so the unit-side analog confirmed here is consistent with the pattern established for aircraft.

**Active in YR:** YES — fires every time a harvester (Mission_Enter) is sent to a refinery with no existing radio contact.

### Q2: Does it send 0x10 RESERVE_DOCK?

**NO.** No PUSH 0x10 or CMP 0x10 found anywhere in the 1894-line disassembly of 0x00741970. Confirmed by exhaustive search. 0x10 is not in this function.

The Phase 2 slot 5 finding that `BuildingClass::Receive_Radio case 0x10` handles RESERVE_DOCK and returns ROGER for Refinery=yes buildings is real, but the sender is not this function — it is likely in `UnitClass::PerCellProcess` (0x739EC0) or `Mission_Enter` per-tick logic.

**Active in YR:** N/A (not in this function)

### Q3: Does it handle QUEUED(0x17) reply?

**NO.** The only reply code checked after any Transmit call is `CMP EAX, 0x1` (ROGER) at every site. There is no `CMP EAX, 0x17` anywhere in this function. A QUEUED reply (0x17) from the building falls through the `iVar6 != 1` branch identically to any other non-ROGER reply, resulting in `OVER_AND_OUT(0x03)` being sent.

Therefore: when the refinery replies QUEUED(0x17) to this function's 0x0E send, the harvester sends OVER_AND_OUT and then commits whatever destination was computed. The QUEUED wander/wait behavior (FUN_00500200) is NOT triggered from this function. It is in the per-tick handler (`Mission_Enter @ 0x004D9290`), which runs once per tick and re-sends 0x0E repeatedly until ROGER or it times out.

**Active in YR:** Confirmed absent.

### Q4: Does it send 0x07 or 0x19?

**0x19 LEAVE_DOCK:** YES — sent at **0x00741B97** via vtable+0x274 (ToFirst), under the very specific cancel-dock gate described in Stage 3 above. This is NOT the normal dock chain — it is the "abort dock approach" path that fires when the unit is redirected to a new destination while already approaching a building with a radio link established AND specific RTTI/flag conditions on the contact.

**0x07 DOCKING_COMPLETE:** NO. No PUSH 0x07 found anywhere in the function. Confirmed by exhaustive search. 0x07 is sent by the building side (BuildingClass::Receive_Radio case 0x08), not by the harvester here.

**Active in YR:** 0x19 = YES (conditional); 0x07 = absent.

---

## 7. State Transitions on `this`

Field offsets read directly from disassembly (EBP = `this`):

| Asm Address | Field | Offset | Value Written | Condition |
|-------------|-------|--------|---------------|-----------|
| 0x741A9C | `+0x1F8` | has-destination flag | 0 (clear) | Unconditionally on entry (after early-out check) |
| 0x7425C6 | `+0x1F8` | has-destination flag | 1 (set) | Chrono locomotor swap completed; `SetMission(7)` called |
| 0x741CB8 | `+0x500` | pending unit target | EAX (filtered UnitClass dest) | RTTI==1 UnitClass dest; store unit-target then ghost |
| 0x741CF9 | `+0x500` | pending dock target? | ESI (building) | RTTI==6 Building has DockUnload=yes; SetGhostCell |
| 0x741D9F | `+0x500` | pending dock target? | ESI (building) | UnitRepair building, health below threshold → store for deferred dock |
| 0x741E88 | `+0x5E0` | reserved-dock tick / dock-ptr | EDI (prior +0x5A4 value) | After CAN_DOCK block: write previous dock target back |
| 0x742D11 | `+0x5E0` | reserved-dock tick / dock-ptr | 0xFFFFFFFF (-1) | UnitRepair/UnitReload ROGER path: mark dock completed |
| 0x7425BF | `+0x6AC` | one-shot skip-`Head_To_Coord` flag | 1 | Chrono/teleporter preprocessing sets the next Set_Destination_Internal call to write NavCom but skip locomotor `Head_To_Coord` once |
| 0x741B4A | `+0x418` | has-destination flag (read) | read-only here | Gate for LEAVE_DOCK(0x19) path |

**Fields NOT written in this function:**
- `+0x84` (DockLink) — not written
- `+0x254` — not written  
- `+0x6AF` (chrono teleport pending flag) — not written; `+0x6AC` is written, not `+0x6AF`
- `+0x2E4` (DockedIn) — not written
- mission state (`vtable+0x1E8`) — IS called at 0x7425B9 (SetMission 7) and 0x742F37 (SetMission 7), and at 0x741A6D (vtable+0x1F0 StopMoving), but these are via vtable not direct field writes

Destination fields: committed only via the tail call to `FootClass::Set_Destination_Internal(0x004D94B0)`. The local `param_2` variable may be rewritten multiple times during the function before being passed to the tail.

---

## 8. Integration Synthesis

### Where this function sits in the dock chain

```
Player right-clicks harvester → refinery
  ↓ (destination assignment)
TechnoClass::Set_Destination @ 0x00741970   ← ONE-SHOT on destination change
  → if Mission_Enter AND no existing radio path AND dest is Building:
      Transmit(0x0E, building)  @ 0x741DDA
        → BuildingClass::Receive_Radio(0x0E) @ 0x0043C2D0
            → replies ROGER, sends NEED_TO_MOVE(0x13) → MOVE_TO_CELL(0x12) → etc.
        → harvester's Receive_Radio handles 0x12/0x13 inbound
  → tail: FootClass::Set_Destination_Internal

(Per-tick approach, once Mission_Enter is active:)
FootClass::Mission_Enter @ 0x004D9290
  → calls vtable+0x480 each tick with updated destination
    → dispatches to TechnoClass::Set_Destination @ 0x00741970
      → same 0x0E send if still no radio contact

(Once harvester enters the dock cell:)
UnitClass::PerCellProcess @ 0x00739EC0
  → dock choreography, sends 0x10/0x16/etc.

(Harvester docked, deposits ore, refinery releases:)
ReleaseDockedHarvester @ 0x004595C0
  → sends exit-side radio traffic

```

**Is Set_Destination a one-shot or per-tick call?**  
ONE-SHOT per destination change. However, `Mission_Enter` at `0x004D9290` calls `vtable+0x480` each tick while in the approach phase, which means this function IS called per tick during the mission — but with the same destination each time. The function's inner logic has `PathType::Has_Valid_Steps()` guards that prevent redundant 0x0E sends once a radio contact is established.

**Relationship to Mission_Enter (0x004D9290):**  
Mission_Enter is the per-tick driver that calls Set_Destination on each tick with the same target. Set_Destination performs the initial radio contact on the first call (when no radio path exists), and does nothing radio-related on subsequent calls (the `Has_Valid_Steps` check returns true once the radio link is established).

**Relationship to ReleaseDockedHarvester (0x004595C0):**  
None directly. ReleaseDockedHarvester runs after deposit is complete. The `+0x5E0` write in this function (writing the previous dock target) may be read by the exit path, but this is unresolved.

---

## 9. Tiny Details

- **`FUN_004D03D0` at `0x004D03D0`** — called in Stage 0 chrono path. Identity unresolved in this session; returns bool.
- **`RateTimer__Current` at `0x004C93D0`** — timer read; used to compute chrono approach timing offset. The hardcoded `0x4000` at 0x741A0C is the timer-based comparison operand (initial timer value).
- **`FUN_005F3DB0` at `0x005F3DB0`** — subtraction helper for two RateTimer values; produces a signed difference used in the chrono timing check.
- **`Filter_AbstractType_InMap` at `0x0040DD70`** — accepts RTTI types 1, 2, 6, 0xF; returns NULL for others. Confirmed from Phase 2 slot 1.
- **`DynamicVectorClass__Contains` at `0x0065AD50`** — walks Contacts array; returns 1 if already in the array. Called at 0x741B12 to check if `this` is already in the destination's contacts.
- **`FootClass__GetDestination(0)` at `0x0065AD30`** — reads `Contacts[0]` (first radio contact).
- **`TechnoClass__SetGhostCell` at `0x0070C610`** — clears the pending destination to a ghost (null-like) cell. Called in multiple rejection paths.
- **`FootClass::Enter_Destination` at `0x004DA0E0`** — called in the existing-dock "same destination, enter it directly" path (line 1641 area).
- **`0x742F37` writes `SetMission(7, 0)` after HELLO+CAN_DOCK ROGER in carrier loop** — confirmed via `PUSH 0; PUSH 7; CALL vtable+0x1E8`.
- **Write order for +0x5E0 = -1 path:** (1) `FootClass::Set_Destination_Internal(param_2)` at 0x742D0B, (2) write `+0x5E0 = 0xFFFFFFFF` at 0x742D11, (3) return.
- **Write order for CAN_DOCK ROGER path (main refinery path):** No immediate write — just update `param_2 = new contact address`, then fall through to tail call `FootClass::Set_Destination_Internal(param_2)`.
- **Null-destination path** — when `param_2 == NULL`: skips all dock logic, goes to locomotor-suspend scan, then tail `FootClass::Set_Destination_Internal(NULL, param_3)`.
- **`+0x5A4`** stores the current pending dock target (read at multiple points, written indirectly via Set_Destination_Internal through radio contact updates).
- **Signed right-shift cell conversion** at 0x743049–0x743052: `SAR EAX, 8` with `AND EDX, 0xFF` prefix (sign-correct arithmetic shift for lepton→cell conversion). Present in the `FootClass::Find_Nearby_Passable_Cell` call block.
- **`FUN_007447B0`** called in bridge-crossing path (bridge flag `+0x140 & 0x100` set, with `param_3 != 0`) — likely a bridge destination resolver. Not the refinery path.
- **No `CMP EAX, 0x17` or `CMP EAX, 0xA`** — QUEUED(0x17) and NEGATORY(0xA) replies are never checked. All non-ROGER paths go to OVER_AND_OUT.
- **`0x0053A130`** — always returns 0 (verified Phase 2 slot 1 doc); used in locomotor-suspend scan as a dead gate.

---

## 10. Open Questions — Final State

| # | Question | Status |
|---|----------|--------|
| 1 | What does `TypeClass+0xDFC` represent? (gate for LEAVE_DOCK(0x19) path) | OPEN — offset not resolved to INI key in this session |
| 2 | What does `+0x418` represent semantically? (read at 0x741B4A as "has-pending-destination") | PROBABLE — looks like a "has-committed-destination" flag but unverified |
| 3 | What does `+0x5E0` represent? (written to EDI/prior-dock-target after CAN_DOCK block; written -1 on UnitRepair ROGER) | PROBABLE dock-timer or dock-reservation-count field; unverified semantic |
| 4 | Does `UnitClass::PerCellProcess` (0x739EC0) send 0x10 RESERVE_DOCK? | DEFERRED — the per-cell choreography was out of scope for this investigation |
| 5 | Full semantics of `FUN_004D03D0` in the chrono self-redirect guard | OPEN |
| 6 | The "RTTI==2 contact" check at 0x741B64 — is RTTI==2 Aircraft in this context? If so, why would a harvester have an AircraftClass radio contact? | RESOLVED 2026-07-13: RTTI==2 is confirmed AircraftClass — `AircraftClass::What_Am_I` at 0x0041c180 = `MOV EAX,0x2; RET`, verified via `read_memory 0x0041c180`; ownership bound to AircraftClass vtable+0x2C in RTTI_WHATAMI_TYPEID_RECONCILE_GHIDRA_REPORT.md. Why a harvester's radio contact would be an AircraftClass instance in this gate is still unresolved (not re-investigated), but the RTTI identity itself is settled. |
| 7 | Does `+0x6AC` (written 1 in chrono teleport swap) correspond to `+0x6AF`(chrono) from the task scoping? | RESOLVED by `FOOTCLASS_SET_DESTINATION_GUARD_RECONCILIATION_GHIDRA_REPORT.md`: `+0x6AC` is a one-shot skip-`Head_To_Coord` byte; it is not `+0x6AF`. |

---

## Sources

All claims verified via Ghidra MCP in this session:

- `read_memory 0x007F60F0, 8` → `70 19 74 00` = 0x00741970 at UnitClass vtable+0x480
- `read_memory 0x007F4DE0, 8` → `30 9A 70 00` = 0x00709A30 at TechnoClass vtable+0x480 (stub)
- `read_memory 0x007E9114, 4` → `B0 94 4D 00` = 0x004D94B0 at FootClass vtable+0x480 = `FootClass__Set_Destination_Internal` (corrected 2026-05-29: was listed as `= 0x004D94A0`; `B0 94 4D 00` LE = 0x004D94B0; hex misread — PARAM1_TYPE_MISREAD)
- `get_function_by_address 0x00741970` → `TechnoClass__Set_Destination at 00741970; Body: 00741970-00743186`
- `decompile_function 0x00741970` — full pseudocode body (~500 lines)
- `disassemble_function 0x00741970` — full assembly (1894 lines, saved to tool-results file)
- `decompile_function 0x00709A30` → pure stub `{ return; }`
- `get_function_by_address 0x004D94B0` → `FootClass__Set_Destination_Internal`
- `get_function_by_address 0x004DF0D0` → `FootClass__Stop_Moving`
- `get_function_by_address 0x0070C610` → `TechnoClass__SetGhostCell`
- `get_function_by_address 0x004DA0E0` → `FootClass__Enter_Destination`
- `get_function_by_address 0x00500200` → `FUN_00500200` (unlabeled; body 0x00500200-0x005002FA)
- `get_function_by_address 0x004C93D0` → `RateTimer__Current`
- Assembly radio-call extraction via Python from tool-results disassembly file — all CALL [reg+0x274/0x278] addresses enumerated and verified against decompile pseudocode

**2026-07-13 correction session (RTTI 0xF / 0xB / Open Q6 fix):**
- `disassemble_function 0x00523340` → `MOV EAX,0xf; RET` = `InfantryClass__What_Am_I` — confirms 0xF is InfantryClass, not CellClass
- `disassemble_function 0x00741970` (re-read this session) → confirmed sole `CMP EAX,0xf` at `0x00741afc` (Stage 3 gate item 3, contact's `What_Am_I()`) and sole `CMP EAX,0x2` at `0x00741b64` (Stage 3 gate item 8 / Open Q6)
- `read_memory 0x0041c180, 16` → `B8 02 00 00 00 C3 90...` = `MOV EAX,0x2; RET` = `AircraftClass::What_Am_I` (raw bytes, function not Ghidra-named) — confirms RTTI 2 = AircraftClass, resolving Open Question 6
- Cross-referenced against `RTTI_WHATAMI_TYPEID_RECONCILE_GHIDRA_REPORT.md` (same-session parent doc identifying this error; independently re-verified from the binary rather than trusted as-is)

Prior art cross-referenced (not re-decompiled):
- `UNITCLASS_ENTERBUILDINGORDOCK_GHIDRA_REPORT.md` — Phase 2 slot 1 (AircraftClass::Set_Destination; vtable proofs)
- `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` — Phase 2 slot 2
- `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md` — Phase 2 slot 3
- `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` — Phase 1 slot 2
- `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`
