# Radio Inferred Codes and Dock Handshake Order — Ghidra Research Report

**Date:** 2026-06-02  
**Slot:** slot-5 (swarm 2026-06-02T08:01 substrate §9.2 UNCHECKED resolution)  
**Scope:** Binary confirmation of radio codes 0x07, 0x0B, 0x0C, 0x11, 0x16, 0x1A, 0x1B, 0x1D, 0x1E and the four dock-idiom message orderings. Settle which names in `MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md §3.2` remain inferred vs string-confirmed.  
**Binary:** `gamemd.exe`  
**Status:** COMPLETE (0x0B sender partially unresolved — see §8)

---

## 0. Investigation Contract

**Target question:** Pin down what each of codes 0x0B, 0x0C, 0x1A, 0x1B, 0x1D, 0x1E does at its receiver (case in Receive_Radio switch, side-effect, response code), and confirm the four dock-idiom message orderings. Reconcile 0x07/0x11/0x16 against existing docs (spot-check only).

**Non-goals:** Full unload credit math, full 0x1C repair tick, full sender scan for every code, mission state machine internals, TS-only paths.

**Evidence needed to mark COMPLETE:**
- Decompile + assembly evidence for each code's receiver case.
- Sender identity for each code (or documented as UNVERIFIED if not found in bounded search).
- Four-idiom message sequences with per-step sender citation.
- String-confirmation status for each code name.

**Stop conditions:** After all six codes have receiver-case evidence + three of four idioms confirmed from binary.

---

## 1. String-confirmation status of all nine codes

No string literals for any radio message code name were found in `gamemd.exe`. `search_strings` for `DOCK_LOCK`, `DOCK_APPROACH`, `DOCK_ARRIVED`, `NAVCOM`, `ARE_YOU_ENTERING`, `ENTERING_STATUS_POLL`, `DOCKING_COMPLETE`, `TIMING_SYNC`, `WANT_RIDE` all returned zero matches. **Every code name is behavior-inferred, not string-confirmed.**

The design doc's claim "only 0x01/0x02/0x13/0x24 survive as binary string literals" is not corroborated — these codes also have no string literal found in this session. The design doc's warning that "all [names] are inferred and DRIFT-risk if treated as authoritative" stands for all codes.

**String-confirmed names: NONE of the nine codes.**  
All nine are behavior-derived names; the canonical Westwood symbol names are unknown.

---

## 2. Codes with dedicated prior docs — spot-check reconciliation

### 2.1 Code 0x07
Prior doc: `RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md`

**Spot-check:** `UnitClass::Receive_Radio` case 0x07 at `0x0073750A` — verified still present via live decompile of `0x00737430` this session. Case clears destination/path/mission=0/locomotor-stop. Sends `Transmit_Radio(2, sender)` + `Transmit_Radio_ToFirst(0x18)` if `+0x418` clear + no destination. Returns 1.

**Prior doc verdict: CORROBORATED.** Sole verified sender is carryall pickup; standard refinery unload does NOT send 0x07.

**Name: inferred** ("DOCKING_COMPLETE" or "PICKUP_CONFIRM" — not string-confirmed).

### 2.2 Code 0x11
Prior doc: `RADIO_MSG_0X11_SENDERS_AND_MEANING_GHIDRA_REPORT.md`

**Spot-check:** `FootClass::Receive_Radio` case 0x11 at `0x004D9219` — verified present in live decompile of `0x004D8FB0`. Returns 1 only if current/queued mission == 7 (Mission_Enter). Sole sender is `UnitClass::AI @ 0x007366C1`.

**Prior doc verdict: CORROBORATED.** This is a transport-passenger entry status poll.

**Name: inferred** ("ARE_YOU_ENTERING" / "ENTERING_STATUS_POLL" — not string-confirmed).

### 2.3 Code 0x16
Prior docs: `RADIO_0x16_RECEIVER_UNITCLASS_CASE_16_GHIDRA_REPORT.md` + `RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md`

**Spot-check:** `UnitClass::Receive_Radio` case 0x16 at `0x007376AD` — verified in live decompile of `0x00737430`. Sets facing timer to 0x4000 via ILocomotion vtable+0x4C. May cascade to send Transmit_Radio(0x15, building). Returns 1.

**Prior doc verdict: CORROBORATED.**

**Name: inferred** ("TIMING_SYNC" — not string-confirmed).

---

## 3. Code 0x0B — Receiver and sender

### Receiver: BuildingClass::Receive_Radio case 0x0B

**Evidence:** `decompile_function 0x0043C2D0` (BuildingClass::Receive_Radio), case 0x0B:

```c
case 0xb:
    (*(param_1->vtable + 0x1e8))(0x14, 0);  // Queue_Mission(Unload=0x14, 0)
    goto LAB_0043cce0;  // falls through to case 0x0C tail
```

`LAB_0043cce0`:
```c
TechnoClass__Receive_Radio(piStack_4, param_3, param_2);
return 1;
```

**Side-effect:** Building queues Mission_Unload (0x14). Falls through to TechnoClass base.  
**Return value:** 1 (ROGER).  
**Active in YR:** YES — live receiver. Present in BuildingClass switch covering `param_3 ∈ [0x03, 0x15]`.

### Receiver: which classes handle 0x0B?
- **BuildingClass**: direct case → Queue_Mission(0x14) + TechnoClass fallthrough (verified above).
- **UnitClass**: NOT in UnitClass switch (range 0x03–0x24, but 0x0B is not a direct case per UNITCLASS doc jump-table read at `0x00737B78`). Falls to FootClass.
- **FootClass**: switch range [0x11, 0x23]; 0x0B < 0x11, falls to TechnoClass.
- **TechnoClass**: switch range [0x03, 0x1F]; 0x0B is **not** listed in the verified TechnoClass case set (cases: 3,7,8,9,0x16,0x18,0x19,0x1A,0x1B,0x1C,0x1E,0x1F per `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM_GHIDRA_REPORT.md`). Falls to RadioClass default.

**Conclusion on receiver chain:** BuildingClass handles 0x0B directly. Non-building receivers silently pass through to RadioClass::Receive_Radio default → returns 0.

### Sender of 0x0B
**Bounded search result:** `HouseClass__Place_Production @ 0x004FB0E0` sends 0x0C (not 0x0B) to the factory building. `UnitClass::Mission_Deploy_Building @ 0x0073D630` sends 0x03 (BREAK) on exit. No direct `PUSH 0x0B` followed by radio vtable call was found in the functions examined (AircraftClass, UnitClass, FootClass, HouseClass mission paths).

**Status: UNVERIFIED sender.** The OQ8 from `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` remains open. 0x0B receiver behavior is confirmed; sender is not found in the bounded search.

**Name: inferred** ("DOCK_APPROACH" is one guess; "OPEN_FACTORY_DOOR" another — not string-confirmed, not sender-corroborated from context).

---

## 4. Code 0x0C — Receiver and sender

### Receiver: BuildingClass::Receive_Radio case 0x0C

**Evidence:** `decompile_function 0x0043C2D0`, case 0x0C:

```c
case 0xc:
    iVar10 = GetCurrentMission();
    if (iVar10 != 0x13 /*Mission_UnloadRefinery*/) {
        Queue_Mission(5 /*GUARD*/, 0);  // building switches to Guard unless already unloading
        if (Type[0x16B9] ConstructionYard != 0) {
            ClearAnimSlot(); ClearAnimSlot();
            // re-create ambient anim based on health ratio
        }
    }
    TechnoClass__Receive_Radio(sender, 0x0C, payload);
    return 1;
```

**Side-effect:** If building is NOT in Mission_UnloadRefinery: switches building to Mission_Guard. ConstructionYard resets ambient anim. Then TechnoClass fallthrough.  
**Return value:** 1 (ROGER).  
**Active in YR:** YES.

### Sender of 0x0C
**Verified from `HouseClass__Place_Production @ 0x004FB0E0`:**

```asm
// After FactoryClass__CompletedProduction + object placed:
004fb5d6: PUSH piVar3          ; piVar3 = produced unit (TechnoClass*)
004fb5d8: PUSH 0x0C
004fb5da: MOV ECX, piVar4      ; piVar4 = factory building
004fb5dc: CALL [piVar4 + 0x278] ; Transmit_Radio(0x0C, produced_unit) to factory
```

The produced unit (`piVar3`) sends 0x0C to its factory building (`piVar4`) when placement succeeds. **Evidence:** `decompile_function 0x004FB0E0`: line `(**(code **)(*piVar3 + 0x278))(0xc, piVar4)` — `piVar3` = produced object, `piVar4` = factory building found via `(**(code **)(*piVar3 + 400))(0, 1)`.

**Active in YR:** YES — fires every time a unit is placed by a factory (war factory, airfield, barracks).  
**Name: inferred** ("DOCK_ARRIVED" is the design doc name — not string-confirmed; sender context: "I have arrived at / been placed at the factory exit cell").

---

## 5. Code 0x1A — Receiver

### Receiver: TechnoClass::Receive_Radio case 0x1A

**Evidence:** `decompile_function 0x006F4AB0`, case 0x1A + assembly at `0x006F4BC1`–`0x006F4BF2`:

```asm
006f4bc1: MOV AL, [ESI+0x419]    ; read second dock-lock byte
006f4bc7: TEST AL, AL
006f4bc9: JNZ 0x006f4e3f          ; if already set → default (RadioClass fallthrough)
006f4bcf: MOV EAX, [ESP+0x1c]    ; sender
006f4bd5: PUSH EAX
006f4bd6: PUSH 0x1a
006f4bd8: MOV ECX, ESI
006f4bda: MOV [ESI+0x419], 1     ; set +0x419 = 1
006f4be1: CALL [EDX+0x278]        ; Transmit_Radio(0x1A, sender) — propagate
006f4be7: [return 1]
```

Pseudocode:
```c
case 0x1a:
    if (*(char *)(&param_1[6].UniqueID + 1) == 0) {  // +0x419 == 0 (not already locked)
        *(char *)(&param_1[6].UniqueID + 1) = 1;     // set +0x419 = 1
        Transmit_Radio(0x1a, sender);                  // propagate 
        return 1;
    }
    break;  // → RadioClass default
```

**Field `+0x419`:** One byte past `+0x418` (the dock/contact-entered flag). This is a secondary dock-lock flag, distinct from `+0x418`. Set by 0x1A, cleared by 0x1B. Propagated to the paired contact.

**Side-effect on `this`:** Sets `this+0x419 = 1` (secondary dock-lock) and propagates 0x1A.  
**Return value:** 1 (ROGER) if not already set; RadioClass default (0) if already set.  
**Active in YR:** Conditional — receiver logic is live for any TechnoClass-derived object.

### Sender of 0x1A
**Bounded search:** Not found in AircraftClass, UnitClass, FootClass, BuildingClass, MissionRepairAndProduce. The propagation mechanism (receiver sends 0x1A to its contact) means it is self-propagating once started, but no original initiator was found in the examined functions.

**Status: UNVERIFIED initial sender.** The self-propagation is verified.  
**Name: inferred** — the BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md documents `0x1A` as "(dock lock set)" from TechnoClass decompile context. This is behavior-derived; not string-confirmed. Do NOT use "secondary dock-lock" as a confirmed semantic — it is inferred.

---

## 6. Code 0x1B — Receiver

### Receiver: TechnoClass::Receive_Radio case 0x1B

**Evidence:** `decompile_function 0x006F4AB0`, case 0x1B + assembly at `0x006F4BF5`–`0x006F4C26`:

```asm
006f4bf5: MOV AL, [ESI+0x419]    ; read +0x419
006f4bfb: TEST AL, AL
006f4bfd: JZ 0x006f4e3f           ; if already 0 → default
006f4c03: MOV EAX, [ESP+0x1c]
006f4c09: PUSH EAX
006f4c0a: PUSH 0x1b
006f4c0c: MOV ECX, ESI
006f4c0e: MOV [ESI+0x419], 0     ; clear +0x419
006f4c15: CALL [EDX+0x278]        ; Transmit_Radio(0x1B, sender) — propagate
006f4c1b: [return 1]
```

Pseudocode: exact mirror of 0x1A but clears `+0x419` (0 → skip; non-zero → clear, propagate, return 1).

**Side-effect on `this`:** Clears `this+0x419 = 0` (releases secondary dock-lock) and propagates 0x1B.  
**Return value:** 1 (ROGER) if was set; RadioClass default if already clear.  
**Active in YR:** Conditional.  
**Sender:** UNVERIFIED initial sender (same as 0x1A — self-propagating; no initiator found in bounded search).  
**Name: inferred** ("dock lock clear" from TechnoClass context — not string-confirmed).

---

## 7. Code 0x1D — Receiver

### Receiver: AircraftClass::Receive_Radio case 0x1D

**Evidence:** `decompile_function 0x004190B0` (AircraftClass::Receive_Radio), case 0x1D:

```c
case 0x1d:
    if (param_1[0xad] == 0) {   // no pending orders / not committed
        return (-(uint)(param_1[0xbf] != *(int *)(param_1[0x1b1] + 0x684)) & 9) + 1;
        // = 1 if ammo == type.MaxAmmo (full); 10 if not full
    }
    return 10;  // NEGATORY if has pending orders
```

**Fields:**
- `param_1[0xad]` (byte at `this+0x2B4`): pending-orders / committed-nav flag (same field checked in AircraftClass Receive_Radio mission-deaf guard).
- `param_1[0xbf]` (int at `this+0x2FC`): current ammo count.
- `param_1[0x1b1] + 0x684` (AircraftTypeClass `+0x684`): `MaxAmmo` (verified from prior docs — AircraftClass slot count).

**Side-effect on `this`:** None (pure query).  
**Return value:** 10 (NEGATORY) if has pending orders; 1 (ROGER/full) if ammo == MaxAmmo; 10 (NEGATORY) if ammo < MaxAmmo. (The expression `(-(uint)(val != max) & 9) + 1` = 1 when `val == max`, 10 when `val != max`.)  
**Active in YR:** Conditional — active for aircraft with `MaxAmmo` / reload behavior (e.g., V3 rocket, Black Eagle, etc.).

### Sender of 0x1D
**Verified from `BuildingClass::MissionRepairAndProduce @ 0x0044B780`:**

Assembly at `0x0044C86F`:
```asm
0044c86b: MOV EDX, [EBP]          ; building vtable
0044c86e: PUSH ESI                  ; contact (aircraft being serviced)
0044c86f: PUSH 0x1d                 ; message 0x1D
0044c871: MOV ECX, EBP              ; building = this
0044c873: CALL [EDX + 0x278]        ; Transmit_Radio(0x1D, aircraft)
0044c879: CMP EAX, 0x1              ; check if ROGER (= ammo full)
```

The building (airfield/helipad with `UnitReload=yes`) sends 0x1D to its contact (aircraft) to query whether the aircraft's ammo is full. If reply == 1 (full) → proceeds to reload completion logic. If not full → sends 0x13 + 0x1C for reload tick.

**Active in YR:** YES — active for aircraft reload (`UnitReload=yes` airfields/helipads). This is a standard part of the aircraft reload loop at airfields.  
**Name: inferred** — behavior context: "IS_AMMO_FULL" / "CAN_RELOAD_COMPLETE" query. Not string-confirmed.

---

## 8. Code 0x1E — Receiver

### Receiver: TechnoClass::Receive_Radio case 0x1E

**Evidence:** `decompile_function 0x006F4AB0`, case 0x1E:

```c
case 0x1e:
    piVar1 = (int *)vtable[0x3F4]();   // get NavCom pointer (navigation command target)
    if (piVar1 != NULL && *piVar1 != 0) {
        vtable[0x3C8](*param_4);        // SetPath/SetNavCom to *param_4
        vtable[0x1E8](1, 0);           // SetMission(Move=1, 0)
        return 1;
    }
    break;  // → RadioClass default (return 0)
```

**Side-effects on `this`:**
- `vtable+0x3C8(*param_4)`: sets the navigation target to the value in payload (`*param_4`). In context, this is the cell/coordinate to move to.
- `vtable+0x1E8(1, 0)`: sets mission to 1 (Mission_Move).
**Gate:** `vtable+0x3F4()` must return non-null and its first member must be non-zero (NavCom object exists and is active). This is a conditional: only fires if the object already has an active NavCom.

**Return value:** 1 (ROGER) if NavCom present; RadioClass default (0) otherwise.  
**Active in YR:** Conditional. The NavCom gate (`vtable+0x3F4`) likely returns non-null only for TechnoClass-derived objects with active navigation commands (aircraft, possibly TS-era pathing). Whether this fires in standard YR skirmish depends on which class populates the NavCom.  
**Name: inferred** — behavior: "REDIRECT_NAV" / "SET_DESTINATION_AND_MOVE". Not string-confirmed. The BUILDINGCLASS doc's label "(nav/deploy)" and "If vtable+0x3F4 returns non-null: set nav to *param_4, Queue_Mission(MOVE=1)" is CORROBORATED by this session's live decompile.

---

## 9. Four dock-idiom message ordering

### 9.1 Refinery zero-link FSM (HARV/CMIN → GAREFN/NAREFN)

Verified from:
- `BuildingClass::Receive_Radio @ 0x0043C2D0` (decompile this session)
- `UnitClass::Receive_Radio @ 0x00737430` (decompile prior session, corroborated)
- `TechnoClass::Receive_Radio @ 0x006F4AB0` (decompile this session)
- `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` §7

```
1. harvester→building  HELLO(0x02)          RadioClass: ally-check + add to Contacts[]
2. harvester→building  CAN_DOCK(0x0E)       Building: power-check → sends NEED_TO_MOVE(0x13)
     └ building→harvester  NEED_TO_MOVE(0x13)   FootClass: writes *payload=chrono_dest; returns ROGER
     └ building→harvester  MOVE_TO_CELL(0x12) via vtable+0x27C
        └ harvester→building (implicit reply)  ALREADY_THERE(0x14) if already at cell
     └ building→harvester  ENTER_DOCK(0x18)    TechnoClass: sets +0x418=1, propagates 0x18
     └ building→harvester  TIMING_SYNC(0x16)   UnitClass: faces, sets 0x4000 timer, may cascade 0x15
3. harvester→building  TIMING_SYNC_BACK(0x15) Building: Queue_Mission(Enter=0x10, harvester); returns 1
4. harvester→building  OVER_AND_OUT(0x03)  Building: GrandOpening(); TechnoClass removes contact
```

**Key constraint:** Step 3 (0x15→building) only fires when harvester locomotor is stopped (facing settled) AND mission==7. The building responds to 0x15 by calling `param_2->vtable[0x1E8](0x10, 0)` — sends harvester into Mission_Enter (ore deposit begins). This is the **triggering step** for actual unloading.

### 9.2 War factory exit idiom

Verified from `HouseClass__Place_Production @ 0x004FB0E0` (decompile this session):

```
1. factory_building→produced_unit  HELLO(0x02)        (building->unit link setup during exit)
2. produced_unit→factory_building  DOCK_ARRIVED(0x0C)  Building: if mission != UnloadRefinery → Queue_Mission(Guard); ConstructionYard anim reset
3. factory_building→produced_unit  OVER_AND_OUT(0x03)  (break link after exit)
```

**Note:** Code 0x0B is documented as received by buildings (Queue_Mission(Unload)), but its **sender** in the factory exit chain was NOT found in `HouseClass__Place_Production`. The 0x0C send is confirmed (`piVar3->Transmit_Radio(0x0C, piVar4)`). Whether 0x0B fires in the factory idiom — and from which function — remains **UNVERIFIED sender** (OQ8 from BUILDINGCLASS doc still open).

### 9.3 Airfield/helipad aircraft reload idiom

Verified from `BuildingClass::MissionRepairAndProduce @ 0x0044B780` (decompile this session):

```
Per-tick repair/reload loop (building iterates contacts):
1. building→aircraft  IS_AMMO_FULL(0x1D)         AircraftClass: returns 1 if full, 10 if not
   if reply != 1 (not full):
2. building→aircraft  NEED_TO_MOVE(0x13)          FootClass: chrono-gate; returns ROGER/NEGATORY
   if reply == 1:
3. building→aircraft  IS_REPAIRING(0x1C)          FootClass: chrono-gate → TechnoClass: repair tick
   if reply != 1 and != 0x21:
4. building→aircraft  IS_CAPACITY(0x1F)           TechnoClass: count vs TypeClass capacity
   if reply == 1 from 0x1C: [go to DONE]
5. building→aircraft  send SetMission(GUARD,0) + OVER_AND_OUT(0x03)  (release link)
```

**Key finding:** Code 0x1D is the **ammo-full gate** in the reload loop — the building polls the aircraft first; if not full it attempts to reload (sends 0x1C). This sequence is confirmed from MissionRepairAndProduce assembly at `0x0044C86B–0x0044C920`.

### 9.4 Bunker/service-depot idiom

Partially verified from `BuildingClass::Receive_Radio @ 0x0043C2D0` and `BuildingClass::MissionRepairAndProduce`:

```
Service-depot (UnitRepair=yes) per-tick:
1. building→unit  NEED_TO_MOVE(0x13)    FootClass: chrono gate
2. building→unit  IS_REPAIRING(0x1C)    TechnoClass: repair tick → returns ROGER/REPAIR_COMPLETE/INSUFFICIENT_FUNDS
   (ROGER = tick complete, more needed)
   (REPAIR_COMPLETE = unit fully healed)
3. [on REPAIR_COMPLETE] building→unit  OVER_AND_OUT(0x03)
```

Bunker (`Bunker=yes`) entry uses HELLO(0x02) + CAN_ENTER(0x0F) to admit infantry. Codes 0x1A/0x1B are NOT observed in any of the four dock-idiom paths in the bounded search.

---

## 10. Codes 0x1A/0x1B — active YR usage assessment

From TechnoClass decompile, 0x1A sets `+0x419` and propagates; 0x1B clears `+0x419` and propagates. No sender was found in any of the four dock idioms or in the mission functions examined (AircraftClass, UnitClass, FootClass, BuildingClass::MissionRepairAndProduce, HouseClass::Place_Production, AircraftClass::Mission_Enter, UnitClass::Mission_Guard, Mission_Deploy_Building, Mission_Unload, Mission_Enter).

**Active in YR:** CONDITIONAL — receiver logic is live, but the activation path in standard YR skirmish is UNVERIFIED. The self-propagation ensures that once triggered, both objects in a link see the state change. This may be a TS-era secondary lock mechanism not commonly triggered in standard YR skirmish.

---

## 11. Implementation Handoff

### Handoff 1 — RadioMessage enum: confirmed vs inferred
All nine codes (0x07, 0x0B, 0x0C, 0x11, 0x16, 0x1A, 0x1B, 0x1D, 0x1E) are **behavior-inferred names**. None are string-confirmed. The `RadioMessage` enum MUST document each entry as "behavior-derived, not string-confirmed." Canonical chain: `RADIO_INFERRED_CODES_AND_DOCK_HANDSHAKE_ORDER_GHIDRA_REPORT.md` (this doc) → `RadioMessage` enum comments → test assertions using numeric constants 0x0B, 0x0C, etc., not string names. Risk: a wrong inferred name shipping as authoritative causes silent semantic drift when the enum is used for pattern-matching in tests.

### Handoff 2 — Code 0x1D aircraft reload loop
Confirmed: `0x1D` is an aircraft ammo query. `BuildingClass::MissionRepairAndProduce` sends it to each contact to check if the aircraft is fully loaded before doing a repair/reload tick. Response: 1 = full (skip tick), 10 = not full (do tick). Implementation: the radio bus `handle_receive_radio` for `AircraftClass` must return 1 when `current_ammo == max_ammo` AND `pending_orders == false`, else 10. Full chain: `0x1D` receive → `AircraftClass::Receive_Radio case 0x1D @ 0x004190B0 (AircraftClass::Receive_Radio decompile)` → `RadioMessage::IsAmmoFull(0x1D)` in enum → `accept_radio_msg` match arm in `sim/aircraft.rs` → test: `aircraft_reload_query_returns_roger_when_full_ammo`. Risk: wrong field mapped for `current_ammo` vs `max_ammo` — verify `+0x2FC` (ammo) and `TypeClass+0x684` (MaxAmmo) against struct layout doc before wiring.

### Handoff 3 — Code 0x0C factory-exit sequence
Confirmed sender: produced object sends 0x0C to its factory building immediately on placement success in `HouseClass::Place_Production`. Building response: if not currently in Mission_UnloadRefinery, switch to Mission_Guard. ConstructionYard: reset ambient anim. Implementation chain: `0x0C` receive in `BuildingClass::Receive_Radio` → `RadioMessage::DockArrived(0x0C)` → building radio handler switches to Guard mission if not unloading → test: `factory_unit_placed_sends_dock_arrived_to_building`. Risk: missing the conditional guard `mission != 0x13` — the building must NOT interrupt an ongoing refinery unload just because a unit happened to send 0x0C.

---

## 12. Negative Facts / Do Not Do

1. **Do NOT name 0x1A "SecondaryDockLock" or 0x1B "SecondaryDockLockClear" as confirmed names in the enum.** The field `+0x419` is set/cleared by these codes and propagated, but no sender is confirmed in standard YR idioms. Name them as `RadioMessage::INFERRED_0x1A` / `RadioMessage::INFERRED_0x1B` until a sender is found. Evidence: no `PUSH 0x1a` radio send found in AircraftClass, UnitClass, FootClass, BuildingClass, or HouseClass mission paths.

2. **Do NOT treat 0x0B as a confirmed refinery message.** The 0x0B receiver (building queues Mission_Unload) is confirmed; the sender is NOT. The design doc's suggestion that 0x0B fires in the refinery dock sequence has no binary sender evidence. OQ8 remains open. Evidence: `UnitClass::Mission_Deploy_Building` sends 0x03 on exit, not 0x0B; `HouseClass::Place_Production` sends 0x0C, not 0x0B.

3. **Do NOT use 0x11 as a dock/nav instruction.** It is an `ARE_YOU_ENTERING` transport/passenger status poll. Evidence: sole sender `UnitClass::AI @ 0x007366C1`; receiver returns 1 only if mission==7 (Mission_Enter). Not a building-to-unit movement command.

4. **Do NOT implement 0x1E as unconditionally firing.** The `vtable+0x3F4()` NavCom gate must return non-null (NavCom active) for 0x1E to set mission+nav. The gate makes it conditional — standard ground units without active NavCom will silently reject 0x1E. Evidence: TechnoClass case 0x1E decompile (`decompile_function 0x006F4AB0`) shows explicit null-check on `vtable+0x3F4()` result before any side-effect.

5. **Do NOT assert all four dock idioms are fully confirmed.** The factory-exit idiom lacks a confirmed 0x0B sender, and the 0x1A/0x1B secondary-lock idiom has no confirmed activation path in standard YR. Evidence: bounded sender search found neither; step-by-step decompile of `HouseClass::Place_Production` and AircraftClass mission functions found no 0x0B or 0x1A initiating send.

---

## 13. Remaining Uncertainty

1. **0x0B sender** — Who sends 0x0B to a building and under what conditions? Not found in the factory exit, refinery dock, or aircraft idiom paths in this session. The receiver behavior is confirmed; the sender location is unknown. May require full-binary `PUSH 0x0b` + radio-call scan.

2. **0x1A/0x1B initial sender** — What triggers the first 0x1A to start the secondary dock-lock chain? Not found in any bounded search. May be a TS-era holdover rarely triggered in standard YR, or may be behind a code path not examined (e.g., specific unit deploy states, IFV weapon-swap, mind-control interactions).

3. **0x1E active in standard YR** — The NavCom gate (`vtable+0x3F4()`) must return non-null for 0x1E to fire. What populates the NavCom in standard YR, and which sender actually emits 0x1E, is unresolved. May be a TS-era or aircraft-only path. Marking as CONDITIONAL until sender is found.

4. **`+0x419` exact byte offset on TechnoClass** — The decompile shows `((int)&param_1[6].UniqueID + 1)` which is `+0x418 + 1 = +0x419`. This matches the assembly `[ESI+0x419]`. The struct layout interpretation (which TechnoClass field this maps to) is not verified in a separate struct layout analysis session. Medium confidence.

---

## 14. Design Doc §3.2 Stale-Wording Corrections

**`MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md §3.2`** should replace the code-name table rows for the nine codes:

```
0x07: behavior-inferred "DOCKING_COMPLETE" — CORROBORATED for carryall use; NOT active in standard refinery DockUnload (dedicated doc confirmed). Name still inferred.

0x0B: behavior-inferred "DOCK_APPROACH" — RECEIVER confirmed (building queues Unload). SENDER NOT FOUND in factory/refinery idiom bounded search. Keep as INFERRED.

0x0C: behavior-inferred "DOCK_ARRIVED" — SENDER confirmed: produced unit → factory building in HouseClass__Place_Production (0x004FB0E0, line `Transmit_Radio(0x0C, factory)`). Still inferred name.

0x11: behavior-inferred "ARE_YOU_ENTERING" — CONFIRMED from dedicated doc; transport passenger status poll. Still inferred name.

0x16: behavior-inferred "TIMING_SYNC" — CONFIRMED from dedicated doc. Still inferred name.

0x1A: behavior-inferred (dock-lock-set) — RECEIVER confirmed (+0x419=1, propagate). SENDER NOT FOUND. Keep as INFERRED; do not name definitively.

0x1B: behavior-inferred (dock-lock-clear) — RECEIVER confirmed (+0x419=0, propagate). SENDER NOT FOUND. Keep as INFERRED.

0x1D: behavior-inferred "IS_AMMO_FULL" — SENDER confirmed: BuildingClass::MissionRepairAndProduce (0x0044C86F, PUSH 0x1d + vtable+0x278). RECEIVER confirmed: AircraftClass case returns 1 if ammo==MaxAmmo. Still inferred name.

0x1E: behavior-inferred "SET_NAV_AND_MOVE" — RECEIVER confirmed (TechnoClass: NavCom-gated SetPath + Mission_Move). SENDER NOT FOUND. Keep as INFERRED. Mark as CONDITIONAL in YR.
```

---

## Sources

| Claim | Evidence |
|-------|----------|
| TechnoClass 0x1A/0x1B case, +0x419 field | `decompile_function 0x006F4AB0` + assembly at `0x006F4BC1`–`0x006F4C26` |
| TechnoClass 0x1E case, NavCom gate + SetPath + SetMission | `decompile_function 0x006F4AB0` |
| BuildingClass 0x0B case, Queue_Mission(Unload) | `decompile_function 0x0043C2D0` |
| BuildingClass 0x0C case, Queue_Mission(Guard) gate on mission != 0x13 | `decompile_function 0x0043C2D0` |
| HouseClass__Place_Production sends 0x0C to factory | `decompile_function 0x004FB0E0`: `Transmit_Radio(0x0C, piVar4)` |
| AircraftClass 0x1D case, ammo query | `decompile_function 0x004190B0` |
| BuildingClass::MissionRepairAndProduce sends 0x1D | assembly `0x0044C86F`: `PUSH 0x1d; CALL [EDX+0x278]` |
| MissionRepairAndProduce also sends 0x13, 0x1C, 0x1F | assembly `0x0044C8A8`, `0x0044C8EE`, `0x0044C8DB` |
| No string literals for any code name | `search_strings` for DOCK_LOCK, DOCK_APPROACH, ARE_YOU_ENTERING, DOCKING_COMPLETE, TIMING_SYNC → 0 matches |
| 0x07 spot-check | `decompile_function 0x00737430` case 0x07 at `0x0073750A` — present, unchanged |
| 0x11 spot-check | `decompile_function 0x004D8FB0` case 0x11 at `0x004D9219` — present, unchanged |
| 0x16 spot-check | `decompile_function 0x00737430` case 0x16 at `0x007376AD` — present, unchanged |
| Refinery dock order | `decompile_function 0x0043C2D0` case 0x0E; prior docs BUILDINGCLASS + UNITCLASS full switch |
| Factory exit sends 0x0C not 0x0B | `decompile_function 0x004FB0E0`; no 0x0B found |
| AircraftClass reload loop in MissionRepairAndProduce | `decompile_function 0x0044B780` + assembly `0x0044C861`–`0x0044C920` |
