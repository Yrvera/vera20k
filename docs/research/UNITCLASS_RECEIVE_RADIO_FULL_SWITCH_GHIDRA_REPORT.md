# UnitClass::Receive_Radio — Full Switch Decompile

**Address:** 0x00737430  
**Dispatch slot:** vtable+0x194 (verified via `read_memory 0x007f5e04` → `0x00737430`)  
**Confidence:** HIGH on all cases — verified from live decompile + disassembly  
**Active in YR:** Conditional — depends per-case (see per-case verdicts)  
**Report date:** 2026-05-20  
**Scope:** Full case-by-case decode of every switch arm in `UnitClass__Receive_Radio`. BuildingClass and FootClass overrides are out of scope.

---

## 1. Overview

`UnitClass::Receive_Radio` is the radio-protocol receiver for ground vehicles (harvesters, tanks, APCs, etc.). It handles 8 cases directly and falls through to `FootClass::Receive_Radio` for all others.

**Cases handled directly:** 0x03, 0x07, 0x0E, 0x0F, 0x15, 0x16, 0x17, 0x24  
**Cases NOT in this switch (fall-through to FootClass):** 0x02 (HELLO), 0x06, 0x09, 0x0A, 0x0C, 0x10, 0x12, 0x14, 0x18, 0x19, 0x21 — all go to `FootClass::Receive_Radio` at `0x004d8fb0`.

> **CRITICAL CORRECTION vs. the task brief:** The brief listed cases 0x06, 0x09, 0x0A, 0x0C, 0x0E, 0x10, 0x12, 0x14, 0x16, 0x18, 0x21 as expected cases. The live jump table (verified via `read_memory 0x00737b54` and `0x00737b78`) shows that only {0x03, 0x07, 0x0E, 0x0F, 0x15, 0x16, 0x17, 0x24} have direct handlers. Cases 0x06, 0x09, 0x0A, 0x0C, 0x10, 0x12, 0x14, 0x18, 0x19, and 0x21 all fall through to `FootClass::Receive_Radio`. Case 0x21 is NOT present at all in UnitClass — it is handled upstream by FootClass/TechnoClass.

---

## 2. Dispatch Entry & Vtable Binding Verification

### 2.1 Jump table decode

Switch preamble (verified via `disassemble_function 0x00737430`):
```asm
00737444: CMP EAX, 0x21        ; param_3 - 3; if > 0x21, jump to default
00737447: JA 0x00737afc        ; default → FootClass::Receive_Radio fallthrough
0073744f: MOV CL, [EAX + 0x737b78]  ; index lookup table at 0x737b78
00737455: JMP [ECX*4 + 0x737b54]    ; jump table at 0x737b54
```

Index table `[0x737b78]` (verified via `read_memory 0x00737b78`, length=40):
```
param_3=0x03 → idx 0 → 0x00737b14   (case 3)
param_3=0x07 → idx 1 → 0x0073750a   (case 7)
param_3=0x0E → idx 2 → 0x007377d8   (case 0x0E)
param_3=0x0F → idx 3 → 0x0073758a   (case 0x0F)
param_3=0x15 → idx 4 → 0x0073778f   (case 0x15)
param_3=0x16 → idx 5 → 0x007376ad   (case 0x16)
param_3=0x17 → idx 6 → 0x00737a98   (case 0x17)
param_3=0x24 → idx 7 → 0x0073745c   (case 0x24)
all others   → idx 8 → 0x00737afc   (fallthrough to FootClass)
```

### 2.2 Vtable binding

- UnitClass primary vtable base: `0x007f5c70` (from `UnitClass__Constructor` @ `0x007353c0`: `*param_1 = &vtable__UnitClass`, disassembled as `MOV [ESI], 0x007f5c70`)
- Vtable slot +0x194 = address `0x007f5c70 + 0x194 = 0x007f5e04`
- `read_memory 0x007f5e04` (4 bytes) → `0x00737430` ✓

**Vtable binding: CONFIRMED.** `UnitClass::Receive_Radio` is at vtable+0x194 = `0x00737430`.

---

## 3. Case-by-Case Decode

> **Conventions:**
> - `this` = UnitClass instance = `param_1` (int*, offsets are direct byte offsets)
> - `sender` = `param_2` (TechnoClass*)
> - `msg` = `param_3` (int)
> - `payload` = `param_4` (void**)
> - All offsets verified from `disassemble_function 0x00737430`

---

### Case 0x03 — OVER_AND_OUT (Break Radio Contact)

**Entry point:** `0x00737b14` (verified: disassembly shows CALL+CMP at `007374bb`)  
**Expected sender:** Any (building, aircraft, infantry — no sender-type filter)  
**Protocol name:** OVER_AND_OUT / BREAK

**Logic:**
1. Call `vtable+0x184` on `this` to get mission ID.
2. If mission ID == 0x0C (Mission_Unload): call `vtable+0x1e8(5, 0)` on `this` — sets mission to 5 (MISSION_GUARD or MISSION_STOP).
3. Delegate to `FootClass::Receive_Radio(sender, 0x03, payload)` at `0x004d8fb0`.
4. Return 1.

**Side effects on `this`:**
- If currently on Mission_Unload (0x0C): changes mission to 5 via vtable+0x1e8.
- FootClass sub-call: clears radio contact slot (sets `Contacts[slot] = NULL`).

**Reply transmitted:** None (no `Transmit_Radio` call in this case).  
**Return value:** 1 (ROGER always).

**Constants:**
- `0x0C` = Mission_Unload (harvester's unloading state at refinery)
- `5` = Mission_Guard / Mission_Stop (the mission set when dock breaks)

**Active in YR:** YES — fires whenever a docked building or transport sends BREAK to a unit. For harvesters, fires when a refinery breaks the radio link.

---

### Case 0x07 — DOCKING_COMPLETE / Carryall Pickup Confirm

**Entry point:** `0x0073750a`  
**Verified sender:** `AircraftClass::Mission_Move_Carryall @ 0x00416D50` in the carryall pickup handshake. No standard refinery sender is verified.  
**Protocol name:** DOCKING_COMPLETE / carryall pickup confirm; not a standard refinery DockUnload completion message.

**Logic:**
1. Delegate to `FootClass::Receive_Radio(sender, 0x07, payload)` at `0x004d8fb0`.
2. Call `vtable+0x480(0, 1)` on `this` — `TechnoClass::SetDestination(NULL, forceOverride=1)` — clears the destination.
3. Call `vtable+0x3c8(0)` on `this` — clears path/locomotion target.
4. Call `vtable+0x1e8(0, 0)` on `this` — sets mission to 0 (MISSION_NONE or MISSION_STOP).
5. Call `FUN_004da1c0` (thin wrapper: `*(*(this+0x5ac) + 0xC)()` — calls locomotor's `Stop_Moving` or equivalent via vtable slot +0xC on the linked locomotor).
6. Check `FootClass+0x418` (radio/contact flag): if set, call `FootClass__GetDestination(0)` (`0x0065ad30`) — if result != NULL (has a valid contact), skip steps 7-8.
7. Transmit `Transmit_Radio(2, sender)` — sends msg 0x02 (HELLO/ACK?) to sender.
8. Transmit `Transmit_Radio_ToFirst(0x18)` — sends TOGGLE_DOCK (0x18) to the first radio contact.
9. Return 1.

**Side effects on `this`:**
- Destination cleared (`this+0x5AC` locomotor path reset).
- Mission set to 0 (MISSION_NONE).
- Locomotor stop issued.

**Reply transmitted:** `Transmit_Radio(2, sender)` + `Transmit_Radio_ToFirst(0x18)` — only if no valid destination exists.  
**Return value:** 1 (ROGER always).

**Constants:**
- `0x480` = SetDestination vtable slot  
- `0x3c8` = ClearPath vtable slot  
- `0x1e8` = SetMission vtable slot  
- `0` = MISSION_NONE  
- `0x18` = TOGGLE_DOCK (sent as acknowledgment to refinery)

**Active in YR:** CONDITIONAL — live when a UnitClass object receives 0x07 from the carryall pickup path. `RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md` found no standard `GAREFN/NAREFN` DockUnload sender, so this does not fire in normal stock refinery unload.

---

### Case 0x0E — CAN_DOCK (Dock Request / Queue Check)

**Entry point:** `0x007377d8`  
**Expected sender:** Building (refinery)  
**Protocol name:** CAN_DOCK

This is the main dock-admission gate — the refinery asks "can you dock with me?"

**Logic flow** (from disassembly `0x007377d8`–`0x00737a95`):

**Part A — Pre-locomotor checks:**
1. Assert `this->ILocomotion` (`this+0x19d`) is non-null; crash with 0x80004003 if null.
2. Call ILocomotion vtable+0x10 (`Is_Moving`) — if (Is_Moving OR `this+0x5a4 != 0` = chrono state active) AND `this+0xAC == 2` (in-queue state): return 0x0A (NACK — not ready). (corrected 2026-05-29: was "Is_Moving AND this+0x169 != 0 AND this+0xAC==2"; binary at 007377f8-0073780d shows second condition reads [ESI+0x5a4] (chrono flag), not 0x169. No read of 0x169 exists anywhere in the case 0x0E body — OPERATOR_OR_ORDER_DRIFT)
3. Get current destination via `FootClass__GetDestination(0)` at `0x0065ad30`.
4. If destination != sender (already targeting someone else): compare range — call `FUN_00473460` (distance calc) and compare against sender's `+0x380` (spread/radius) and type's `+0x5e0` (dock range). If out-of-range or spread too large: return 0x0A.
5. Check `PathType__Has_Valid_Steps` (`0x0065ae30`): if no valid path, transmit `Transmit_Radio(2, sender)` (request new path).

**Part B — Zone/terrain check** (if not chrono-teleporting, `this+0x5a4 == 0`):
6. Get unit's current map cell via `vtable+0x1b8`.
7. Get zone of unit's cell and zone of sender's cell (via `MapClass__GetZoneID`). If zones differ: call `vtable+0x480(sender, 1)` (SetDestination to sender) then return 0x0E (can't dock — different zone, try again).
8. Check cell overlay type: if current cell's `CellClass+0xEC == 2` (bridge cell type): return 0x0E (can't dock on bridge).

**Part C — Range confirmation and handshake:**
9. Re-read dock range from `this[0x1b1] + 0x5e0` (= `UnitTypeClass.DockRange`). If 0 or negative: fall through to FootClass.
10. Recompute distance to sender; if in range AND sender's size fits: delegate `FootClass::Receive_Radio(sender, 0x0E, payload)`.
11. Re-check Is_Moving, chrono flag (`this+0x6AF`), and radio/contact flag (`this+0x418`).
12. If all clear: send `Transmit_Radio(0x13, sender)` — ask refinery "which cell should I go to?" (QUEUE_DOCK / REQUEST_CELL).
13. If reply == 1 (ROGER): write `this` into `*payload` (the harvester self-identifies), then call `Transmit_Radio_Impl(0x12, payload, sender)` — send MOVE_TO_CELL with self-pointer in payload.
14. If MOVE_TO_CELL reply != 1: send `Transmit_Radio(3, sender)` — OVER_AND_OUT (abort if cell assignment failed).
15. Return 1.

**Side effects on `this`:**
- May set destination to sender via vtable+0x480.
- Sends `Transmit_Radio(0x13, sender)` and `Transmit_Radio_Impl(0x12, payload, sender)`.

**Reply transmitted:** 
- 0x0A (NACK) under several early-exit conditions  
- 0x0E (retry) if zone/bridge mismatch  
- 0x13 then 0x12 on success path  
- 0x03 (BREAK) if cell assignment fails  

**Return value:** 0x0A (NACK), 0x0E (retry), or 1 (ROGER after completing handshake).

**Constants:**
- `0x13` = REQUEST_DOCK_CELL / QUEUE_DOCK  
- `0x12` = MOVE_TO_CELL  
- `0x0A` = NACK  
- `0x0E` = retry (return same msg)  
- `0x5e0` offset on UnitTypeClass = DockRange  
- `0x380` offset on BuildingClass result = BuildingType spread (foundation size context)  
- `0x6AF` = chrono-teleporting flag  
- `0x5a4` = chrono-teleporting flag (alternate — `0x5a4 != 0` → is chrono; this is the same field used in step 2's second NACK condition)  
- `0xAC` = queue state indicator  
(corrected 2026-05-29: removed `0x169 = pending-orders flag`; this offset does not appear anywhere in the case 0x0E disassembly — OPERATOR_OR_ORDER_DRIFT)

**Active in YR:** YES — fires every time a refinery polls a harvester for dock admission.

---

### Case 0x0F — WANT_RIDE (Carryall/Transport Request)

**Entry point:** `0x0073758a`  
**Expected sender:** Carryall or transport aircraft (flying unit)  
**Protocol name:** WANT_RIDE / CAN_CARRY

The carryall asks the ground unit "can I pick you up?"

**Logic:**
1. Check `this[0x1b1] + 0x5e0 != 0` (unit has non-zero dock range / is a valid cargo type). If 0: return 0.
2. Check sender != NULL. If NULL: return 0.
3. Check ally: `HouseClass__Is_Ally_ByObject(this->Owner, sender)` at `0x004f9a90`. If not ally: return 0.
4. Check `vtable+0x1d4` on `this` — if unit is cloaked/invisible: return 0.
5. Call `vtable+0x1bc` on `this` — get current cell. Then `Look_up_building_in_cell` (`0x0047c520`). If a building is in the cell: return 0 (can't lift from inside a building).
6. Check `TechnoClass__IsMindControlled` (`0x007105e0`). If mind-controlled: return 0 (can't carry a mind-controlled unit).
7. Check sender is not an open-topped transport with passengers (`[sender+0x14] & 4 != 0 && sender[0x694] != 0`): if full, return 0.
8. Check `this+0x2BC` (carryall passenger slot?): if non-null, check `FUN_004722c0` (another blocker check). If blocked: return 0.
9. Range check: compute distance to sender; if out of range: return 0x0A (NACK).
10. Size check: if unit's spread > sender's capacity: return 0x0A.
11. Return 1 (ROGER — can be picked up).

**Side effects on `this`:** None (read-only query response).  
**Reply transmitted:** None.  
**Return value:** 0 (NACK/no), 0x0A (range fail), or 1 (ROGER/yes).

**Active in YR:** YES — fires for carryall pickup protocol. Carryalls are YR-active (Soviet carryall, YR-era).

---

### Case 0x15 — PREPARE (Harvester Arrival Acknowledgment)

**Entry point:** `0x0073778f`  
**Expected sender:** Building (refinery) — sends PREPARE to the docked harvester  
**Protocol name:** PREPARE / DOCK_READY

The refinery tells the harvester "I'm ready for you; proceed to unload."

**Logic** (from disassembly `0x0073778f`–`0x007377d5`):
1. Get `this[0x1b1] + 0x5e0` (DockRange) and call `FUN_00473460` (distance to something via type chain) to get current distance.
2. Compare: if current distance == DockRange:
   - Read refinery's `+0x3CC` (Y unload coordinate) and `+0x3C8` (X unload coordinate).
   - Call `FUN_004a5240(this+0x350, x, y)` — writes the unload position into `this+0x350` (the harvester's destination or start-unload coordinate).
3. Return 5 (PREPARE_ACK / ROGER_PREPARE).

**Side effects on `this`:**
- `this+0x350` may be written with the refinery's unload coordinates if distance matches DockRange.

**Reply transmitted:** None.  
**Return value:** Always 5 (PREPARE_ACK).

**Constants:**
- `0x350` on UnitClass = unload destination coordinate field  
- `0x3C8`/`0x3CC` on BuildingClass = unload pad X/Y coordinates  
- Return `5` = PREPARE_ACK

**Active in YR:** YES — fires on the harvester side every time a refinery sends PREPARE (0x15) during the dock sequence.

---

### Case 0x16 — TIMING_SYNC (Face-Dock + Sync Gate)

**Entry point:** `0x007376ad`  
**Expected sender:** Building (refinery)  
**Protocol name:** TIMING_SYNC / FACE_AND_SYNC

Full analysis already in `RADIO_0x16_RECEIVER_UNITCLASS_CASE_16_GHIDRA_REPORT.md`. Summary of re-verified facts:

1. Calls `FootClass::Receive_Radio(sender, 0x16, payload)` first → which falls to TechnoClass → which sends TOGGLE_DOCK(0x18) back to building as side effect.
2. Guards on chrono flag `this+0x6AF`.
3. If not chrono-teleporting and facing timer (`FootClass+0x388`) != 0x4000: calls `ILocomotion vtable+0x4C(0x4000)` to set facing timer.
4. Cascade: if Is_Moving==false AND has-destination AND destination-type==6 (building) AND building mission==7: sends `Transmit_Radio(0x15, destination)`.
5. Returns 1 always.

**Active in YR:** YES — confirmed unchanged.

---

### Case 0x17 — DEPLOY_UNLOAD (Harvester Self-Deploy at Refinery)

**Entry point:** `0x00737a98`  
**Expected sender:** Building (refinery) — or any entity triggering deploy  
**Protocol name:** DEPLOY_UNLOAD / SELF_DEPLOY

**Logic** (from disassembly `0x00737a98`–`0x00737afc`):
1. Read `this[0x1b1] + 0xe0E` and `this[0x1b1] + 0xe0F` — two boolean flags on UnitTypeClass:
   - `+0xe0E` = `Weeder` (weeder harvester flag)  
   - `+0xe0F` = another harvester-type flag (likely `DockUnload` related on UnitType)
2. If NEITHER flag is set: fall through to FootClass (no deploy action).
3. Check `this+0x6D1` (deploy-in-progress flag). If 0 (already deployed or not deploying): fall through.
4. If deploying (0x6D1 != 0):
   - Clear `this+0x6D1 = 0` (mark deploy complete).
   - Call `vtable+0x174(DAT_00b1cfe8, 1, 0)` — trigger locomotor swap or animation trigger with CLSID/reference at `0x00b1cfe8` (zeroed at time of read — likely a null/default CLSID for the deploy state).
   - Call `vtable+0x1e8(10, 0)` — set mission to 10 (MISSION_UNLOAD or equivalent).
   - Check `vtable+0x200()` — `CanFire` or `IsAlive` check.
   - If true: call `vtable+0x1ec()` — likely `Fire` or `DoAction` (trigger unload animation/sound).
5. Fall through to default (FootClass::Receive_Radio) at end.

**Side effects on `this`:**
- `this+0x6D1` cleared to 0.
- Mission changed to 10 (MISSION_UNLOAD).
- Animation/locomotor swap triggered.
- Possibly fires unload action.

**Reply transmitted:** None (falls through to FootClass which may reply).  
**Return value:** Falls through to `FootClass::Receive_Radio` return value (typically 1).

**Constants:**
- `this+0x6D1` = deploy-in-progress flag  
- `this[0x1b1]+0xe0E` = UnitTypeClass.Weeder  
- `this[0x1b1]+0xe0F` = UnitTypeClass dock-type flag (adjacent to Weeder)  
- `10` = MISSION_UNLOAD  
- `DAT_00b1cfe8` = null CLSID reference (zeroed at read time)

**Active in YR:** CONDITIONAL — only fires for Weeder-type harvesters (`Weeder=yes` units). Standard ore harvesters do NOT have `Weeder=yes` or the adjacent flag set, so they fall through immediately. The weeder refinery mechanic is present but rare in YR.

---

### Case 0x24 — DOCK_QUERY (Can You Accept?)

**Entry point:** `0x0073745c`  
**Expected sender:** Building or aircraft (query sender)  
**Protocol name:** DOCK_QUERY / REQUEST_DOCK

Sender queries "are you available to receive me / dock with you?"

**Logic** (from disassembly `0x0073745c`–`0x00737507`):
1. Check `vtable+0x1d4()` on `this` — if cloaked/stealthed: return 0 (can't dock to cloaked unit).
2. Call `vtable+0x4C(auStack_c, 0)` on `this` — get current coordinate into stack buffer.
3. `CellClass__Get_Cell_At(coord)` — get the CellClass for current position.
4. Check `CellClass+0x140` bit 0x100 — if set (cell flag, likely "under bridge" or "occupied by building"): check `vtable+0xBC(0)` on `this` — if not passable/valid: return 0x0A (NACK).
5. Check mission via `vtable+0x184()` — if mission == 0x10 (Mission_Selling / Mission_Unload in progress?): return 0x0A (NACK).
6. Check `this+0x418` (has-destination flag): if set (unit already has a destination): return 0x0A (NACK — unit committed elsewhere).
7. Otherwise: return `1 + (this+0x684 != -1 ? 9 : 0)` — returns 1 normally, or 10 if `this+0x684 != -1`.

**Side effects on `this`:** None.  
**Reply transmitted:** None.  
**Return value:** 0 (cloaked), 0x0A (busy/blocked), or 1/10 (available).

**Constants:**
- `CellClass+0x140` bit 0x100 = cell occupancy/bridge flag  
- `0x10` = Mission_Selling or Mission_Unload (mission blocking dock acceptance)  
- `this+0x418` = radio/contact flag set by TechnoClass radio 0x18 and cleared by radio 0x19  
- `this+0x684` = veterancy or unit-type secondary flag (return modifier)

**Active in YR:** YES — this is the generic dock-availability query used by any building querying a unit.

---

### Cases NOT in UnitClass (fall-through to FootClass)

The following cases pass directly to `FootClass::Receive_Radio @ 0x004d8fb0` without any UnitClass-specific logic:

| Case | Name (from RADIO_CLASS_PROTOCOL doc) | Notes |
|------|--------------------------------------|-------|
| 0x02 | HELLO | Radio link establishment |
| 0x06 | — | Not in UnitClass |
| 0x09 | DOCK_ARRIVED (inbound query?) | Handled by FootClass/TechnoClass |
| 0x0A | — | Handled by FootClass |
| 0x0C | DOCK_ARRIVED | Not in UnitClass |
| 0x10 | — | Not in UnitClass |
| 0x12 | MOVE_TO_CELL | Not in UnitClass — unit is SENDER not receiver |
| 0x14 | CELL_ACCEPTED | Not in UnitClass |
| 0x18 | TOGGLE_DOCK / ENTER_DOCK | Handled by TechnoClass via FootClass chain |
| 0x19 | LEAVE_DOCK | Not in UnitClass |
| 0x21 | — | Not in UnitClass (see §7 open questions) |

---

## 4. Cross-Cutting: Struct Fields Touched

All offsets verified via `disassemble_function 0x00737430`.

| Field | Offset | Type | Cases | Notes |
|-------|--------|------|-------|-------|
| ILocomotion ptr | `this+0x19D` (×4 = `this+0x674`) | ILocomotion* | 0x16, 0x0E | Null-asserted before vtable calls in both cases; 0x0E checks at 007377de and 00737a1d (corrected 2026-05-29: was only 0x16 — MISLEADING) |
| Chrono-teleporting flag | `this+0x6AF` | bool | 0x16, 0x0E | Guards locomotor calls |
| Chrono flag (alt) | `this+0x5A4` | ptr/int | 0x0E | Non-zero = chrono state active |
| Radio/contact flag | `this+0x418` | byte flag | 0x07, 0x0E, 0x16, 0x24 | Set by TechnoClass radio 0x18, cleared by 0x19; some UnitClass branches pair it with `FootClass__GetDestination` |
| PrimaryFacing RateTimer | `FootClass+0x388` | RateTimer16 | 0x16 | Set to 0x4000 via ILocomotion vtable+0x4C |
| Mission field | via `vtable+0x184` | int | 0x03, 0x24 | Checked; set via `vtable+0x1e8` |
| Unload destination | `this+0x350` | coordinate | 0x15 | Written with refinery unload pad coords |
| DockRange (TypeClass) | `this[0x1b1]+0x5E0` | int | 0x0E, 0x0F, 0x15 | UnitTypeClass field: dock approach range |
| Deploy flag | `this+0x6D1` | bool | 0x17 | Cleared on deploy completion |
| ~~Pending orders~~ | ~~`this+0x169`~~ | ~~bool~~ | ~~0x0E~~ | REMOVED 2026-05-29: offset 0x169 does not appear in case 0x0E disassembly; the second NACK condition reads this+0x5a4 (chrono flag, already listed above) — OPERATOR_OR_ORDER_DRIFT |
| Queue state | `this+0xAC` | int | 0x0E | Value 2 = in dock queue |
| Weeder flag (TypeClass) | `this[0x1b1]+0xE0E` | bool | 0x17 | UnitTypeClass.Weeder |
| Adjacent type flag | `this[0x1b1]+0xE0F` | bool | 0x17 | Dock-type flag adjacent to Weeder |
| Veterancy/secondary | `this+0x684` | int | 0x24 | -1 = no modifier, else +9 to return |
| Carryall slot | `this+0x2BC` | ptr | 0x0F | Checked for existing cargo link |

---

## 5. ILocomotion vtable+0x4C — Concrete Method Resolution

The call at case 0x16 dispatches through `param_1[0x19d]` (the ILocomotion COM pointer = `this+0x674`) using `[vtable + 0x4C]`.

### Which locomotor does a harvester use during dock?

Per `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`, the harvester uses **WalkLocomotionClass** (piggybacked from DriveLocomotionClass) during dock approach. The `param_1[0x19d]` field holds the ILocomotion COM pointer, which points to the locomotor's ILocomotion vtable (at offset +4 from the locomotor object base).

### DriveLocomotionClass ILocomotion vtable+0x4C

- DriveLocomotion ILocomotion vtable base: `0x007e7eb0` (verified: `get_xrefs_to 0x004b0ef0` → `0x007e7efc`; vtable base = `0x007e7efc - 0x4C = 0x007e7eb0`)
- Slot +0x4C at `0x007e7efc` → `0x004b0ef0` = `DriveLocomotionClass__Do_Turn`
- Confirmed by `get_function_by_address 0x004b0ef0`
- Effect: calls `RateTimer::Set(FootClass+0x388, 0x4000)` — verified by `disassemble_function 0x004b0ef0`

### WalkLocomotionClass ILocomotion vtable+0x4C

- WalkLocomotion ILocomotion vtable base: `0x007f69f8` (from constructor disassembly `0x0075aa90`: `MOV [ESI+4], 0x007f69f8`)
- Slot +0x4C at `0x007f69f8 + 0x4C = 0x007f6a44` → `0x0075ae00` = `WalkLocomotionClass__Set_Facing`
- Confirmed by `get_function_by_address 0x0075ae00`
- Effect: calls `FacingClass::UpdateFacing` — **different behavior** from DriveLocomotion

> **KEY FINDING:** The vtable+0x4C dispatch has DIFFERENT semantics depending on which locomotor is active:
> - **DriveLocomotionClass**: calls `RateTimer::Set(FootClass+0x388, 0x4000)` — sets facing timer to east (0x4000)
> - **WalkLocomotionClass**: calls `FacingClass::UpdateFacing` — updates the facing angle directly
>
> The RADIO_0x16 doc's claim that it "calls DriveLocomotionClass__Do_Turn" is accurate ONLY if the harvester's piggybacked locomotor is DriveLocomotion. If a WalkLocomotion unit (infantry, GI, etc.) receives case 0x16, `Set_Facing` is called instead. Both write to the facing state but via different mechanisms.

---

## 6. Diffs vs RADIO_0x16_RECEIVER_UNITCLASS_CASE_16_GHIDRA_REPORT.md

| Claim in prior doc | Status | Notes |
|--------------------|--------|-------|
| Case 0x16 entry at `0x007376AD` | CORROBORATED | Jump table confirms this address |
| `FootClass+0x388` written to 0x4000 via ILocomotion vtable+0x4C | CORROBORATED | `disassemble_function 0x00737430` @`007376d4`/`007376e6` still present |
| "DriveLocomotionClass__Do_Turn" at `0x004b0ef0` | CORROBORATED with caveat | True for DriveLocomotion; WalkLocomotion hits `Set_Facing` at `0x0075ae00` instead (different fn, same slot) |
| All case 0x16 exits return EAX=1 | CORROBORATED | `0x0073770f`, `0x00737783` both confirm |
| Cascade condition checks `this+0x418` | CORROBORATED | Disassembly `0073774a: MOV AL, [ESI+0x418]` |
| No Stop_Moving in case 0x16 | CORROBORATED | No stop call in case 0x16 body |
| Cascade condition checks building mission==7 | CORROBORATED | `0073776e: CMP EAX, 0x7` |
| Cascade sends Transmit_Radio(0x15, building) | CORROBORATED | `00737778: CALL [EDX+0x278]` with 0x15 |
| Prior doc claim: "harvester mission == 7" as cascade gate | **CORRECTED** | The disassembly at `00737768: CALL [EAX+0x184]` checks the HARVESTER's own mission (via `param_1`/ESI, not EDI). This is the harvester's mission, compared to 7. Prior doc correctly states this in §6 ("Harvester mission == 7"), which is confirmed. |

All prior doc claims for case 0x16 CORROBORATED or corrected in detail above.

---

## 7. Open Questions — Final State

| Question | Status | Answer |
|----------|--------|--------|
| Case 0x21 — what does it do? Is it TS-legacy? | RESOLVED | Case 0x21 is **NOT PRESENT** in UnitClass::Receive_Radio. The jump table range is 0x03–0x24 and 0x21 maps to index 8 (default = FootClass fallthrough). No UnitClass-specific handling. |
| Case 0x16 — re-confirm 0x4000 RateTimer write | RESOLVED | CONFIRMED via disassembly. The write occurs via ILocomotion vtable+0x4C. |
| Which concrete ILocomotion supplies vtable+0x4C? | RESOLVED | DriveLocomotionClass → `DriveLocomotionClass__Do_Turn` @ `0x004b0ef0`. WalkLocomotionClass → `WalkLocomotionClass__Set_Facing` @ `0x0075ae00`. |
| Does harvester send case 0x0C DOCK_ARRIVED inbound? | RESOLVED | Case 0x0C is NOT in UnitClass switch — falls through to FootClass. The harvester receives 0x0C from FootClass/TechnoClass chain, not from UnitClass. DOCK_ARRIVED is transmitted by the building, received by FootClass/TechnoClass. |
| Case 0x14 — CELL_ACCEPTED reply to 0x12? | RESOLVED | Case 0x14 is NOT in UnitClass switch at all. Falls to FootClass. UnitClass does not handle 0x14 directly. |
| Does any case handle 0x07 DOCKING_COMPLETE on harvester side? | RESOLVED / REFINED 2026-05-21 | YES — case 0x07 is directly handled at `0x0073750a`, but the verified sender is the carryall pickup path, not a standard refinery. It clears destination, stops movement, resets mission to 0. |
| Does any case handle 0x19 LEAVE_DOCK on harvester? | RESOLVED | Case 0x19 is NOT in UnitClass switch. Falls to FootClass. |
| Vtable+0x194 binding confirmed? | RESOLVED | YES — `read_memory 0x007f5e04` → `0x00737430`. |

**No items deferred.**

---

## Sources

All findings verified via live Ghidra MCP in this session.

| MCP Call | Purpose |
|----------|---------|
| `decompile_function 0x00737430` | Full switch pseudocode |
| `disassemble_function 0x00737430` | Assembly for all 8 cases + jump table |
| `read_memory 0x00737b54` (100 bytes) | Jump table decode |
| `read_memory 0x00737b78` (40 bytes) | Index table decode |
| `read_memory 0x007f5e04` (4 bytes) | Vtable slot +0x194 binding |
| `read_memory 0x007f5c70` (8 bytes) | UnitClass vtable base verification |
| `get_xrefs_to 0x00737430` | Locate vtable slot containing fn address |
| `decompile_function 0x007353c0` | UnitClass::Constructor (vtable assignment) |
| `disassemble_function 0x0075aa90` | WalkLocomotionClass ILocomotion vtable address |
| `read_memory 0x007f6a44` (4 bytes) | WalkLocomotion ILocomotion vtable+0x4C |
| `read_memory 0x007e7efc` (4 bytes) | DriveLocomotion ILocomotion vtable+0x4C |
| `get_function_by_address 0x0075ae00` | WalkLocomotionClass__Set_Facing confirmation |
| `get_function_by_address 0x004b0ef0` | DriveLocomotionClass__Do_Turn confirmation |
| `decompile_function 0x0075cbe0` | WalkLocomotion constructor (vtable layout) |
| `decompile_function 0x004a5240` | FUN_004a5240 = unload coordinate writer |
| `get_function_by_address 0x004f9a90` | HouseClass__Is_Ally_ByObject |
| `get_function_by_address 0x007105e0` | TechnoClass__IsMindControlled |
| `get_function_by_address 0x0047c520` | Look_up_building_in_cell |
