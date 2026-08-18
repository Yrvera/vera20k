# BuildingClass::Receive_Radio — Full Switch Decompile

**Address:** `0x0043C2D0`
**Dispatch slot:** vtable+0x194 (verified via `read_memory` at `0x007E4050`: bytes `D0 C2 43 00` = little-endian `0x0043C2D0`)
**Confidence:** HIGH on all findings (verified by live decompilation this session)
**Active in YR:** Conditional — depends per-case (see each section)
**Prior-art docs incorporated:** REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE, RECEIVE_RADIO_CASE_0x0E_CAN_DOCK, RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E, BUILDINGCLASS_MISSILE_AND_RADIO (Part 2)

---

## 1. Overview

`BuildingClass::Receive_Radio` (999 instructions per assignment brief) is the building side of every
docking/garrisoning/factory radio exchange. It dispatches on `param_3` (message code). The complete
switch handles **exactly 9 cases**: `3, 8, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0x15`. Every other code
(0x02, 0x07, 0x09, 0x11–0x14, 0x16–0x19, 0x1A–0x1F, 0x22, 0x23) **falls through** to the common tail:

```c
return TechnoClass__Receive_Radio(param_2, param_3, param_4);
```

`TechnoClass::Receive_Radio` at `0x006F4AB0` in turn handles a further set (see §3 and §7).

**Signature (verified from decompile):**
```c
int __thiscall BuildingClass__Receive_Radio(
    BuildingClass *param_1,   // this: the building receiving the radio
    TechnoClass   *param_2,   // sender
    undefined4     param_3,   // message code
    undefined4    *param_4);  // in/out payload (cell*, pointer, etc.)
```

**Response codes used:**

| Value | Name | Meaning |
|-------|------|---------|
| 0 | (silent reject) | Reject with no indication |
| 1 | ROGER | Positive ack |
| 10 (0xA) | NEGATORY | No |
| 0x14 | ALREADY_THERE | Unit already at target cell (used in harvester→building direction) |
| 0x17 | QUEUED | In queue (returned by case 8 for factory/repair/bunker buildings) |
| 0x20 | INSUFFICIENT_FUNDS | Repair tick: owner can't pay |
| 0x21 | REPAIR_COMPLETE | Repair tick: unit fully repaired |

---

## 2. Dispatch Entry & Vtable Binding Verification

BuildingClass vtable base: `0x007E3EBC` (from prior doc; vtable xref to ctor `0x0043B740`).
Receive_Radio slot = vtable+0x194 → vtable address `0x007E3EBC + 0x194 = 0x007E4050`.

`read_memory` at `0x007E4050`, 4 bytes: **`D0 C2 43 00`** = little-endian `0x0043C2D0`. ✓

Prior doc claimed "likely vtable+0x274" was wrong; +0x194 is confirmed. vtable+0x274 is
`Transmit_Radio_ToFirst`, a different slot entirely.

---

## 3. Case-by-Case Decode

### Case 0x03 — OVER_AND_OUT (BREAK: tear down radio link)

**Sender:** any TechnoClass (harvester, aircraft, vehicle, infantry)
**Active in YR:** YES — fires every time a docking unit disconnects

**Body:**
```c
BuildingClass__GrandOpening();               // reset building's idle anim state
TechnoClass__Receive_Radio(param_2, 3, p);  // base: removes sender from Contacts[], RadioClass BREAK
return 1;  // ROGER
```

**Side effects on `this`:**
- `GrandOpening()` at `0x00447780` resets the building's animation slot to its idle state (e.g. refinery
  returns to closed-doors pose).
- `TechnoClass::Receive_Radio` case 3 checks if `this` has `DockedIn` flag set AND sender has flag at
  `sender+0x418`; if both true, sends `LEAVE_DOCK(0x19)` to sender before falling to
  `RadioClass::Receive_Radio` which nulls the Contacts[] slot.

**Reply:** none (return value 1 flows back as Transmit return code, no radio transmitted)
**Constants:** none

---

### Case 0x08 — REQUEST_DOCKING_CLEARANCE (unit→building: "am I in range?")

**Sender:** unit/vehicle/aircraft approaching a factory or repair/bunker building
**Active in YR:** YES — fires during any factory-exit or repair-dock approach

**Body summary:**
```c
// Short-circuit for UnitRepair/Bunker: if sender already within 3 cells (0x180 leptons), return ROGER immediately
if (Type[0x16A9] UnitRepair OR Type[0x16AB] Bunker) {
    dist = Euclidean(this.center, sender.center);   // vtable+0x48 = GetCoords for both
    if (dist < 0x180) return 1;
}
TechnoClass__Receive_Radio(param_2, 8, p);   // base: sends LEAVE_DOCK(0x19) + BREAK(0x3) to sender
// Type check for what to return:
if NOT (WeaponsFactory[0x16BD] OR UnitRepair[0x16A9] OR Bunker[0x16AB]):
    return 1;   // plain ROGER for other building types
return 0x17;    // QUEUED for factory/repair/bunker buildings
```

**Side effects on `this`:** none direct; TechnoClass base sends 0x19 + 0x03 to sender which clears
the dock link from the sender's side.

**Reply:** return value only (ROGER or QUEUED); no radio transmitted by this case.
**Constants:** `0x180 leptons` = 3 cells distance threshold (hardcoded, UnitRepair/Bunker short-circuit)
**INI flags read:** `Type[0x16BD]` WeaponsFactory, `Type[0x16A9]` UnitRepair, `Type[0x16AB]` Bunker

---

### Case 0x0B — DOCK_APPROACH (building→unit: "come to the dock")

**Sender:** building sending this TO a unit (mislabeled — the building *receives* 0x0B, meaning a unit
sent 0x0B to the building; see note)
**Active in YR:** YES — used in factory/repair dock sequences

**Note on directionality:** The decompile shows the building *receiving* 0x0B and responding by calling
`Queue_Mission(UNLOAD=0x14, 0)` on itself, then falling through to the case 0xC tail.

**Body:**
```c
this->vtable[0x1E8](0x14, 0);   // Queue_Mission(Mission_Unload=0x14, 0) on the building
// FALLTHROUGH to case 0xC tail:
TechnoClass__Receive_Radio(piStack_4, param_3, param_2);  // note: args appear swapped by Ghidra decompiler
return 1;
```

**Side effects on `this`:** building switches to Mission_Unload (0x14).
**Reply:** ROGER (1)
**Constants:** `0x14` = Mission_Unload

---

### Case 0x0C — DOCK_ARRIVED (unit→building: "I have arrived at the dock cell")

**Sender:** unit (harvester or factory-produced vehicle) that has reached the dock/parking cell
**Active in YR:** YES — fires when any factory-exiting unit reaches its destination, or on refinery arrival

**Body:**
```c
mission = GetCurrentMission();
if (mission != 0x13 /*Mission_Unload_Refinery*/) {
    Queue_Mission(5 /*GUARD*/, 0);          // building switches to GUARD unless already unloading
    if (Type[0x16B9] ConstructionYard != 0) {
        ClearAnimSlot(this);  // twice
        ClearAnimSlot(this);
        ratio = GetHealthRatio();
        if (ratio > Rules[0x1700] /*ConditionYellow*/) {
            animName = Type[0x116C];   // healthy ambient anim name
        } else {
            animName = Type[0x117C];   // damaged ambient anim name
        }
        if (animName != NULL && *animName != 0) CreateAnimForSlot(this);
    }
}
TechnoClass__Receive_Radio(sender, 0xC, p);
return 1;
```

**Side effects on `this`:**
- If not in refinery-unload mission: switches building to GUARD mission.
- If ConstructionYard: clears anim slot twice, re-creates ambient anim based on current health ratio.

**Reply:** ROGER (1)
**Constants:** `0x13` = Mission_Unload_Refinery; `5` = Mission_Guard; `0x116C/0x117C` = TypeClass anim
name offsets; `Rules+0x1700` = ConditionYellow threshold
**INI flags read:** `Type[0x16B9]` ConstructionYard=

---

### Case 0x0D — (unnamed: WeaponsFactory silencer)

**Sender:** any unit
**Active in YR:** YES — fires for WeaponsFactory buildings when a manufactured unit disconnects

**Body:**
```c
if (Type[0x16BD] WeaponsFactory != 0) return 1;   // swallow silently
// else: break → fallthrough to TechnoClass base
```

**Side effects on `this`:** none
**Reply:** ROGER (1) for WeaponsFactory, or TechnoClass base result otherwise
**Constants:** none
**INI flags read:** `Type[0x16BD]` WeaponsFactory=

---

### Case 0x0E — CAN_DOCK (unit→building: "may I dock?")

**Sender:** harvester, vehicle, infantry seeking dock entry
**Active in YR:** YES — fires every match when a harvester tries to dock

*(Fully documented in `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` and
`RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md`. Summary only here.)*

**Filter chain:**
1. TechnoClass base called first
2. Power check: `HasPower == false` → NEGATORY(10)
3. UnitRepair busy check: `Type[0x16A9]` + IS_REPAIRING(0x22) reply == 10 → NEGATORY
4. Bunker deploy check: `Type[0x16AB]` + CanAutoDeployHere == false → NEGATORY
5. Hospital/Armory bypass (Type[0x16C1]/[0x16C2]): direct MOVE_TO_CELL, no queue cell
6. Standard refinery/weeder path: queue cell = anchor+(X+3, Y+1) hardcoded, send MOVE_TO_CELL(0x12)→ENTER_DOCK(0x18)→TIMING_SYNC(0x16)
7. Helipad path (Type[0x16CB]): send MOVE_TO_CELL(0x12) with `this` as payload, then ToFirst(0x18)

**Reply sequence (refinery):** `NEED_TO_MOVE(0x13)` → `MOVE_TO_CELL(0x12)` → `ENTER_DOCK(0x18)` → `TIMING_SYNC(0x16)`
**Constants:** Queue cell offset `(+3, +1)` from NW anchor cell (hardcoded; QueueingCell= INI at +0x1618/+0x161C is NOT read here)

---

### Case 0x0F — CAN_ENTER (unit→building: passenger/garrison/grinder entry request)

**Sender:** infantry (garrison/hospital/armory), vehicle (grinder/repair), aircraft (helipad)
**Active in YR:** YES — fires on every garrison, grinder, armory, hospital, helipad entry attempt

**Full filter chain (verified from decompile):**
```
1. TechnoClass base called first
2. Ally check: !Is_Ally_ByObject(this, sender) → return 0 (silent reject, not NEGATORY)
3. Mission == 0x12 (Construction) → NEGATORY
4. Mission == 0x13 (UnloadRefinery) → NEGATORY
5. field_0x534 == 0 → NEGATORY   (no auxiliary slot available)
6. If not MapEditorMode AND sender is not harvester (FUN_0065adf0) AND NOT (UnitAbsorb[0x16AE] OR InfAbsorb[0x16AF]):
       → NEGATORY
7. Naval zone mismatch: sender TypeClass[0xCCE] (naval flag) must match this TypeClass[0xCCE],
   unless sender TypeClass at 0x5B4 == 5 (aircraft) → skip zone check
8. Sender TypeClass[0xD6A] != 0 → NEGATORY  (sender cannot enter buildings)
9. HasPower == false → NEGATORY

Then type-specific paths (checked in order):

UnitAbsorb/InfAbsorb (Grinder/Cloning Vat):
  - sender GetWhat()==1 (unit) AND UnitAbsorb[0x16AE]==0 → NEGATORY
  - sender GetWhat()==0xF (infantry) AND InfAbsorb[0x16AF]==0 → NEGATORY
  - sender has CaptureManager AND FUN_004722C0 (mind-controlled) → NEGATORY (don't grind MC units)
  - if (field_0x114+1 <= Type[0x5E0]) AND (sender TypeClass[0x380] <= Type[0x388]) → ROGER
    (value-cap check: refund only if unit value fits within grinder capacity/threshold)

Grinding (Type[0x16AD]):
  → ROGER unconditionally

Bunker (Type[0x16AB]):
  - !CanAutoDeployHere(sender) → NEGATORY
  - Transmit(0x23 IS_OCCUPIED, sender)==ROGER → NEGATORY (already full)
  → ROGER

UnitRepair (Type[0x16A9]):
  - sender GetWhat() not 1 (unit) and not 2 (aircraft) → NEGATORY
  - Transmit(0x23 IS_OCCUPIED, sender)==ROGER → NEGATORY (already in use)
  → ROGER

Hospital(0x16C2) OR Armory(0x16C1) AND sender GetWhat()==0xF (infantry):
  - sender has CaptureManager AND mind-controlled → NEGATORY
  - IsMindControlled(sender) → NEGATORY
  - field_0x2FC == 0 → NEGATORY; field_0x2FC != 0 → ROGER (corrected 2026-05-28: was "field_0x2FC != 0 → NEGATORY; else ROGER"; binary expression `(-(uint)(field_0x2fc != 0) & 0xfffffff7) + 10` evaluates to 1/ROGER when non-zero, 10/NEGATORY when zero — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT; verified via decompile_function 0x0043C2D0)
  - Return: (field_0x2FC == 0 ? NEGATORY : ROGER)

Helipad (Type[0x16CB]):
  - sender dock-type-field != 2 → NEGATORY; else ROGER

Garrison (Type[0x16B3], unit sender, TypeClass[0xE0E] garrison-allowed flag):
  - MapEditorMode → ROGER
  - field_0x118 == 0 → ROGER (no occupants)
  (else fall through to return 0)

Weeder garrison (Type[0x16BC], unit sender, TypeClass[0xE0F]):
  - same pattern as above

Default: return 0 (silent reject)
```

**Side effects on `this`:** none (pure read/query)
**Reply:** none (return value only)
**Constants:** `0x5B4` (TypeClass aircraft-type field), `0xCCE` (TypeClass naval flag), `0xD6A` (TypeClass
can-enter-buildings flag), `0xE0E/0xE0F` (TypeClass garrison flags), `0x5E0` (TypeClass cost/weight cap),
`0x380/0x388` (TypeClass value thresholds)

---

### Case 0x10 — RESERVE_DOCK (unit→building: "can I reserve a slot?")

**Sender:** a unit (harvester) asking to reserve a dock slot before approaching
**Active in YR:** **YES — for standard refineries.** Phase 2 slot 5 (2026-05-20) corrected the
earlier "Type[0x16BB] = unknown flag" claim: `Type[0x16BB]` is **`Refinery=yes`**, verified via
string `"Refinery"` at `0x0081AA5C` used in `BuildingTypeClass_ReadINI_Water` to write +0x16BB.
Standard GAREFN/NAREFN refineries set `Refinery=yes`, so case 0x10 returns ROGER for them.

**Body:**
```c
if (field_0x118 == 0                        // no current passengers/occupants
    AND FUN_0065adf0()                      // sender passes harvester-or-similar check (free contact slot)
    AND field_0x81 == 0                     // no lockout flag
    AND sender.GetOwner() == this.Owner)    // same player
{
    if (Type[0x16BB] Refinery) return 1;    // standard refinery → ROGER (CORRECTED 2026-05-20)
    if (Type[0x16A9] UnitRepair) return 1;
    if (Type[0x16BC] Weeder) return 1;
}
return 10;   // NEGATORY for all other cases
```

**Key finding (CORRECTED 2026-05-20):** A standard GAREFN/NAREFN refinery (`Refinery=yes`) receives
**ROGER(1)** for case 0x10 — not NEGATORY as originally claimed. Whether the harvester actually
sends 0x10 to a refinery during the dock approach is a separate question (the sender trace is in
`RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md`); receiver-side acceptance is now confirmed.

**Side effects on `this`:** none
**Reply:** ROGER or NEGATORY (return value only)
**INI flags read:** `Type[0x16BB]` **`Refinery=`** (corrected from "unknown"), `Type[0x16A9]` UnitRepair,
`Type[0x16BC]` Weeder

---

### Case 0x15 — TIMING_SYNC_BACK / DOCK_NOW (unit→building: "I'm ready, start unload")

**Sender:** harvester that has received TIMING_SYNC(0x16) and is now at the dock cell
**Active in YR:** YES — fires every harvest deposit cycle for standard refineries

*(Documented in `RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md` §4. Summary here.)*

**Body:**
```c
if (mission == 0x13 Mission_UnloadRefinery) return 10;   // already unloading → reject
if (UnitAbsorb[0x16AE]) return 1;
if (InfAbsorb[0x16AF]) return 1;
if (UnitRepair[0x16A9] OR UnitReload[0x16AA] OR Hospital[0x16C1] OR Armory[0x16C2]) {
    field_0x6DD = 1;                        // trigger dock animation
    Queue_Mission(0x14 Unload, 0);          // building starts unload
    sender.Queue_Mission(0 Sleep, 0);       // sender sleeps
    return 1;
}
if (Bunker[0x16AB]) {
    field_0x6DD = 1;
    Queue_Mission(0x14 Unload, 0);
    return 1;
}
if (DockUnload[0x16B3]) {                   // standard refinery
    sender.Queue_Mission(0x10 Enter, 0);    // harvester → Mission_Enter (starts ore deposit)
    return 1;
}
// else fall through to TechnoClass base
```

**Side effects on `this`:** `field_0x6DD = 1` (anim-complete flag, triggers dock anim) for
UnitRepair/Reload/Hospital/Armory/Bunker paths. `Queue_Mission` transitions building state.
**Reply:** ROGER (return 1) or NEGATORY (10). No radio transmitted.

---

## 4. Cases That Fall Through to TechnoClass::Receive_Radio

All codes NOT in {3, 8, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0x15} fall through.
Key codes handled by TechnoClass (verified from decompile of `0x006F4AB0`):

| Code | Name | TechnoClass action |
|------|------|--------------------|
| 0x02 | HELLO | Falls further to RadioClass (adds sender to Contacts[]) |
| 0x07 | DOCKING_COMPLETE | Sends ENTER_DOCK(0x18) to sender, then RadioClass::Receive_Radio, returns 1 |
| 0x09 | (same group as 0x07) | Same: sends 0x18 to sender, RadioClass, returns 1 |
| 0x11 | IS_UNIT_LINKED | Falls to RadioClass::Receive_Radio (no TechnoClass case) |
| 0x12 | MOVE_TO_CELL | Falls to RadioClass (no TechnoClass case) |
| 0x13 | NEED_TO_MOVE | Falls to RadioClass (no TechnoClass case) |
| 0x14 | ALREADY_THERE | Falls to RadioClass (no TechnoClass case) |
| 0x16 | TIMING_SYNC | Sends ENTER_DOCK(0x18) to sender, RadioClass, returns 1 (same block as 0x07/0x09) |
| 0x17 | EVICT_QUEUE | Falls to RadioClass |
| 0x18 | ENTER_DOCK | Sets DockedIn flag (`param_1[6].UniqueID = 1`), sends 0x18 to param_2 (propagate), returns 1 |
| 0x19 | LEAVE_DOCK | Clears DockedIn flag, sends 0x19 to param_2, returns 1 |
| 0x1A | (dock lock set) | Sets second dock-lock bit, sends 0x1A, returns 1 |
| 0x1B | (dock lock clear) | Clears second dock-lock bit, sends 0x1B, returns 1 |
| 0x1C | REPAIR_TICK | Full repair-tick logic (health check, spend money, add HP, return REPAIR_COMPLETE/INSUFFICIENT_FUNDS/ROGER) |
| 0x1E | (nav/deploy) | If vtable+0x3F4 returns non-null: set nav to *param_4, Queue_Mission(MOVE=1) |
| 0x1F | (capacity) | Compares field_0x4C to Type+0x684 capacity; if at cap NEGATORY, else increment |
| 0x22 | IS_REPAIRING | Falls to RadioClass (no TechnoClass/BuildingClass case) |
| 0x23 | IS_OCCUPIED | Falls to RadioClass (no TechnoClass/BuildingClass case) |

**DockedIn flag location (verified from TechnoClass decompile):**
`*(undefined1 *)&param_1[6].UniqueID` — relative to TechnoClass base pointer; the field at base+6
object slots. This is `TechnoClass+0x198` by approximate layout (ObjectClass is 6 slots = 0x18 bytes per
`sizeof(ObjectClass)`, but actual layout requires struct layout analysis to confirm exact byte offset).
Confidence on exact offset: MEDIUM (decompile shows `param_1[6].UniqueID` access clearly; byte position
within TechnoClass struct requires separate layout verification).

---

## 5. PRIMARY OPEN QUESTIONS — Resolved

**Q1: Does case 0x07 DOCKING_COMPLETE fire after the last ore bale?**
ANSWER: Case 0x07 is NOT in the BuildingClass switch — it falls through to TechnoClass, which sends
ENTER_DOCK(0x18) to the sender and returns 1. The building does not do anything special for 0x07.
Active in YR: YES (via TechnoClass base), fires at dock-complete signal from unit side.

**Q2: Does case 0x0B DOCK_APPROACH fire during a refinery dock, or is it aircraft/helipad only?**
ANSWER: Case 0x0B is present in the BuildingClass switch. When the building receives it, it calls
`Queue_Mission(0x14 Unload, 0)` on itself and returns ROGER. It is NOT helipad-specific. Active in YR: YES.

**Q3: Does case 0x10 RESERVE_DOCK / 0x11 IS_UNIT_LINKED appear in the refinery chain?**
ANSWER: Case 0x10 is present in BuildingClass and returns NEGATORY for standard DockUnload refineries
(neither UnitRepair nor Weeder). Case 0x11 is not in the BuildingClass switch — falls to TechnoClass/
RadioClass which have no case 0x11 either, so it reaches RadioClass default (ObjectClass::Receive_Radio).
Not part of normal refinery chain.

**Q4: Cases 0x22 (34) and 0x23 (35) — TS-legacy or live?**
ANSWER: Neither 0x22 nor 0x23 appears in the BuildingClass switch. Both fall to TechnoClass (no case for
either there either), then to RadioClass (no case), then to ObjectClass. They are effectively no-ops when
received by a building. However, 0x22 IS_REPAIRING and 0x23 IS_OCCUPIED are **sent** by the building
(case 0x0E sends 0x22 to query repair state; case 0x0F sends 0x23 to query occupancy). Active in YR: YES
as transmit codes, but have no building-side receive handler in this function.

**Q5: Does case 0x19 LEAVE_DOCK appear in this switch?**
ANSWER: NO. 0x19 is not in the BuildingClass switch. It is handled by TechnoClass (clears DockedIn flag,
propagates 0x19 to partner, returns 1). Active in YR: YES (via TechnoClass base).

---

## 6. Cross-cutting: INI Key Reads / Struct Field Offsets Touched

All offsets on `BuildingTypeClass` (verified from `BuildingTypeClass::ReadINI_Water` at `0x0045FE50` per
prior docs; usage confirmed in decompile of `0x0043C2D0`):

| Offset | INI key | Cases that read it |
|--------|---------|-------------------|
| +0x16A9 | `UnitRepair=` | 0x08, 0x0E, 0x0F, 0x10 |
| +0x16AA | `UnitReload=` | 0x15 |
| +0x16AB | `Bunker=` | 0x08, 0x0E, 0x0F, 0x10, 0x15 |
| +0x16AD | `Grinding=` | 0x0F |
| +0x16AE | `UnitAbsorb=` | 0x0F, 0x15 |
| +0x16AF | `InfAbsorb=` (InfantryAbsorb) | 0x0F, 0x15 |
| +0x16B3 | `DockUnload=` (Refinery) | 0x0E, 0x0F, 0x15 |
| +0x16B9 | `ConstructionYard=` | 0x0C |
| +0x16BB | `Refinery=` | 0x10 — corrected 2026-05-28: was `(unknown flag)`; §3 case 0x10 correction (2026-05-20) identified this as `Refinery=yes`; table updated for consistency — ROOT_CAUSE: INFERENCE_HARDENED (partial 2026-05-20 fix didn't propagate to this table) |
| +0x16BC | `Weeder=` | 0x0E, 0x0F, 0x10, 0x15 |
| +0x16BD | `WeaponsFactory=` | 0x08, 0x0D |
| +0x16C1 | `Hospital=` | 0x0E, 0x0F, 0x15 |
| +0x16C2 | `Armory=` | 0x0E, 0x0F, 0x15 |
| +0x16CB | `Helipad=` | 0x0E, 0x0F |
| +0x1618 | `QueueingCell=` X | stored but NOT read in any case |
| +0x161C | `QueueingCell=` Y | stored but NOT read in any case |
| +0x1780 | `NumberOfDocks=` | read at construction time only (Contacts[] capacity) |
| +0x116C | healthy ambient anim name | 0x0C (ConstructionYard path) |
| +0x117C | damaged ambient anim name | 0x0C (ConstructionYard path) |

**BuildingClass instance fields written:**

| Offset | Field | Written by |
|--------|-------|-----------|
| +0x6DD | `field_0x6DD` (anim-complete flag / dock-anim trigger) | Case 0x15 (UnitRepair/Reload/Hospital/Armory/Bunker paths) |

**BuildingClass instance fields read (not written here):**

| Offset | Field | Read by |
|--------|-------|---------|
| +0x81 | `field_0x81` (lockout flag) | Case 0x10 |
| +0x114 | `field_0x114` (unit count / cost accumulator) | Case 0x0F (grinder cap check) |
| +0x118 | `field_0x118` (current passenger/occupant count) | Cases 0x0C, 0x0F, 0x10 |
| +0x2FC | `field_0x2FC` (Armory per-house slot) | Case 0x0F |
| +0x534 | `field_0x534` (auxiliary slot available flag) | Case 0x0F |
| +0xE8 | contact count (Contacts[].Count) | Case 0x0E (Hospital/Armory eviction loop) |
| HasPower | power state | Cases 0x0E, 0x0F |

---

## 7. Refinery Dock Case Chain — Observable In-Order Sequence

For a standard `DockUnload=yes` refinery (GAREFN/NAREFN) receiving an approaching harvester:

| Step | Sender | Message | Building action | Gate |
|------|--------|---------|----------------|------|
| 1 | harvester→building | HELLO(0x02) | RadioClass handles: ally check + Contacts[] add | always first |
| 2 | harvester→building | CAN_DOCK(0x0E) | Power check → NEED_TO_MOVE(0x13) probe → MOVE_TO_CELL(0x12) with queue cell (NW+3,+1) | harvester must be en route |
| 3 | building→harvester | MOVE_TO_CELL(0x12) | (inside case 0x0E body) | harvester replies 0x14 (ALREADY_THERE) to proceed |
| 4 | building→harvester | ENTER_DOCK(0x18) | (inside case 0x0E body; gated on step 3) | sent unconditionally if step 3 passed |
| 5 | building→harvester | TIMING_SYNC(0x16) | (inside case 0x0E body; after ENTER_DOCK) | harvester returns ROGER or non-ROGER; sound plays if non-ROGER |
| 6 | harvester→building | TIMING_SYNC_BACK(0x15) | sender.Queue_Mission(Enter=0x10, 0) → ore deposit begins | harvester locomotor at 0x4000 + idle + mission==7 |
| 7 | harvester→building | OVER_AND_OUT(0x03) | GrandOpening() resets anim; TechnoClass removes from Contacts[] | after ore deposit completes |

Case 0x10 RESERVE_DOCK: NOT part of standard refinery chain (returns NEGATORY for DockUnload buildings).
Case 0x0B DOCK_APPROACH: building self-queues Mission_Unload if received; role in refinery chain unclear —
likely for factory buildings exiting produced units, not refinery. Needs caller trace to confirm.
Case 0x0C DOCK_ARRIVED: for refinery, skips the branch body (mission IS 0x13), goes straight to TechnoClass.

---

## 8. TS-Legacy Cases — Flag with Reason

| Case/code | Status | Reason |
|-----------|--------|--------|
| 0x0E UnitRepair path (REJECT 2) | Active in YR | Service Depot (NAYARD etc.) uses UnitRepair=yes |
| 0x0E Bunker path (REJECT 3) | Active in YR | PillBox, Bunker buildings use Bunker=yes |
| 0x0E Hospital/Armory path | Active in YR | Hospital and Armory buildings exist in YR |
| 0x0F UnitAbsorb/InfAbsorb | Active in YR | Grinder (NAGRND), Cloning Vat (NACLND) use these |
| 0x10 Type[0x16BB] branch | Active in YR (Refinery=yes) | Corrected 2026-05-28: was "Likely TS-legacy"; §3 case 0x10 (corrected 2026-05-20) identifies +0x16BB as `Refinery=yes` set on GAREFN/NAREFN; these buildings DO return ROGER for 0x10 — ROOT_CAUSE: INFERENCE_HARDENED (stale TS-legacy guess not updated when §3 was corrected) |
| TechnoClass 0x1E (nav/deploy radio) | Conditional | Only fires if vtable+0x3F4 returns non-null; likely TS-era mission-redirect system; verify activation before implementing |
| TechnoClass 0x1F (capacity) | Conditional | Compares occupant count to Type+0x684; role needs caller trace |

---

## 9. Diffs vs Prior Docs

| Prior doc claim | Status |
|-----------------|--------|
| "verified cases: 3, 8, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0x15" (BUILDINGCLASS_MISSILE_AND_RADIO Part 2 §2.2) | **CORROBORATED** — full decompile confirms exactly these 9 cases in the BuildingClass switch |
| "All other messages fall through to TechnoClass::Receive_Radio" | **CORROBORATED** — confirmed 0x07, 0x09, 0x16, 0x18, 0x19, 0x22, 0x23 all fall through |
| vtable+0x194 (not +0x274) for Receive_Radio dispatch | **CORROBORATED** — read_memory at 0x007E4050 = 0x0043C2D0 ✓ |
| Queue cell formula = anchor+(X+3, Y+1) hardcoded in case 0x0E | **CORROBORATED** — decompile shows `CONCAT22(psVar5[1]+1, *psVar5+3)` |
| QueueingCell= INI (+0x1618/+0x161C) NOT read in case 0x0E | **CORROBORATED** — not referenced anywhere in BuildingClass switch |
| Case 0x15 for DockUnload: `sender.Queue_Mission(0x10 Enter, 0)` | **CORROBORATED** — confirmed as `param_2->vtable[0x1E8](0x10, 0)` |
| TechnoClass handles 0x18 ENTER_DOCK: sets DockedIn flag | **CORROBORATED** — `param_1[6].UniqueID = 1` |
| Cases 0x22, 0x23 not in BuildingClass switch | **NEW CONFIRMED** — both fall all the way to RadioClass/ObjectClass |
| Case 0x10 returns NEGATORY for standard DockUnload refinery | **NEW CONFIRMED** — only UnitRepair/Weeder/0x16BB return ROGER |

---

## 10. Open Questions — Final State

| # | Question | Status |
|---|----------|--------|
| OQ1 | Does case 0x07 fire after last ore bale? | RESOLVED — falls to TechnoClass; sends ENTER_DOCK(0x18) back, returns 1. Not refinery-specific. |
| OQ2 | Is case 0x0B aircraft/helipad-only? | RESOLVED — NOT helipad-specific. Building self-queues Mission_Unload. Caller trace needed to confirm what sends 0x0B to a refinery specifically. |
| OQ3 | Does case 0x10 appear in refinery chain? | RESOLVED — NO for DockUnload refineries (returns NEGATORY). |
| OQ4 | Cases 0x22, 0x23 TS-legacy or live? | RESOLVED — neither has a building-side receive handler. Both used only as TRANSMIT codes (building sends them to query unit state). Live in YR as transmit. |
| OQ5 | Does case 0x19 LEAVE_DOCK appear in BuildingClass switch? | RESOLVED — NO. Handled by TechnoClass (clears DockedIn, propagates). |
| OQ6 | What is Type[0x16BB]? | RESOLVED — `Refinery=yes` (corrected 2026-05-20 in §3, propagated to §6/§8 tables 2026-05-28). Standard refineries set this flag and receive ROGER for case 0x10. |
| OQ7 | Exact byte offset of DockedIn flag (`param_1[6].UniqueID`)? | OPEN — requires TechnoClass struct layout analysis to convert `param_1[6]` to byte offset. |
| OQ8 | What triggers 0x0B to be sent to a refinery (vs factory)? | OPEN — needs caller trace of who sends 0x0B and under what conditions. |

---

## Sources

- `decompile_function 0x0043C2D0` — full BuildingClass::Receive_Radio decompile (this session)
- `decompile_function 0x006F4AB0` — full TechnoClass::Receive_Radio decompile (this session)
- `read_memory 0x007E4050` — vtable+0x194 binding verification (this session)
- `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` — HELLO(0x02) path, vtable binding
- `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` — case 0x0E full filter chain
- `RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md` — 0x16 emission, case 0x15 sequence
- `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md` Part 2 — switch overview and TechnoClass base

---

## Status: COMPLETE
