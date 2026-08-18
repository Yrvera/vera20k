# FootClass::Receive_Radio — Full Switch Decompile

**Address:** `0x004D8FB0`
**Dispatch slot:** vtable+0x194 (verified via `read_memory 0x007E8E28` → `B0 8F 4D 00` = `0x004D8FB0`)
**Confidence:** HIGH — all cases verified by live `decompile_function 0x004D8FB0` + `disassemble_function 0x004D8FB0` + jump-table decode via `read_memory 0x004D9258` (jump targets) and `read_memory 0x004D9274` (index table)
**Active in YR:** Conditional — per-case below
**Report date:** 2026-05-20
**Scope:** Full case-by-case decode of every switch arm in `FootClass::Receive_Radio @ 0x004D8FB0`. TechnoClass::Receive_Radio (already covered in Phase 1 slot 2) is referenced for context but NOT re-decompiled here.

---

## 1. Overview

`FootClass::Receive_Radio` is the radio protocol handler for the FootClass layer — shared by vehicles (UnitClass), infantry (InfantryClass), and aircraft (AircraftClass). InfantryClass has **no Receive_Radio override** and inherits this function directly. UnitClass and AircraftClass override Receive_Radio but fall through to this function for cases they do not handle.

**Callers (direct):**
- `UnitClass::Receive_Radio @ 0x00737430` — case 0x16 calls FootClass first; case 0x03, 0x07, 0x17 delegate here
- `AircraftClass::Receive_Radio @ 0x004190B0` — cases 0x0E, 0x12, 0x13, 0x1F delegate here; others fall through
- Confirmed via `get_function_callers 0x004D8FB0` → `AircraftClass__Receive_Radio @ 004190b0` and `UnitClass__Receive_Radio @ 00737430`

**Signature (verified from decompile):**
```c
int __thiscall FootClass__Receive_Radio(
    int *param_1,        // this: FootClass* (offsets as direct byte; param_1 is int*)
    undefined4 param_2,  // sender (TechnoClass*)
    undefined4 param_3,  // message code
    int *param_4         // in/out payload
)
```

> **CRITICAL CORRECTION vs. the task scoping brief:**
> The brief listed expected cases as {0x05, 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x12, 0x14, 0x18, 0x20, 0xC8}.
> The live jump table shows NONE of those IDs have direct handlers in FootClass.
> The actual handled cases are: **0x11, 0x12, 0x13, 0x17, 0x1C, 0x23**.
> Case 0xC8 (200) does NOT exist — the switch range spans only 0x11..0x23. See §2.

---

## 2. Jump Table Decode (Actual Case ID Set)

### 2.1 Switch preamble disassembly (verified)

```asm
004d8fbd: LEA EAX, [EDI + -0x11]      ; normalize: param_3 - 0x11
004d8fc0: CMP EAX, 0x12               ; if > 0x12 (18), out of range → fallthrough
004d8fc3: JA  0x004d90cc              ; → TechnoClass::Receive_Radio
004d8fc9: XOR ECX, ECX
004d8fcb: MOV CL, byte ptr [EAX + 0x4d9274]  ; index table
004d8fd1: JMP dword ptr [ECX*0x4 + 0x4d9258] ; jump table
```

Switch range: `param_3 ∈ [0x11, 0x11+0x12] = [0x11, 0x23]` (17 to 35 decimal).
Any param_3 < 0x11 or > 0x23 → falls through directly to `TechnoClass::Receive_Radio @ 0x006F4AB0`.

### 2.2 Jump table (`0x4D9258`, 7 entries × 4 bytes)

Verified via `read_memory 0x004D9258, 28 bytes`:

| idx | Target address | Case(s) mapped to it |
|-----|----------------|---------------------|
| 0 | `0x004D9219` | 0x11 |
| 1 | `0x004D9139` | 0x12 |
| 2 | `0x004D90E8` | 0x13 |
| 3 | `0x004D902B` | 0x17 |
| 4 | `0x004D900E` | 0x1C |
| 5 | `0x004D8FD8` | 0x23 |
| 6 | `0x004D90CC` | all others → TechnoClass fallthrough |

### 2.3 Index table (`0x4D9274`, 19 bytes for param_3 0x11..0x23)

Verified via `read_memory 0x004D9274, 20 bytes`:

| param_3 | dec | idx → target | Case |
|---------|-----|--------------|------|
| 0x11 | 17 | 0 → `0x004D9219` | HANDLED |
| 0x12 | 18 | 1 → `0x004D9139` | HANDLED |
| 0x13 | 19 | 2 → `0x004D90E8` | HANDLED |
| 0x14 | 20 | 6 → TechnoClass | fallthrough |
| 0x15 | 21 | 6 → TechnoClass | fallthrough |
| 0x16 | 22 | 6 → TechnoClass | fallthrough |
| 0x17 | 23 | 3 → `0x004D902B` | HANDLED |
| 0x18 | 24 | 6 → TechnoClass | fallthrough |
| 0x19 | 25 | 6 → TechnoClass | fallthrough |
| 0x1A | 26 | 6 → TechnoClass | fallthrough |
| 0x1B | 27 | 6 → TechnoClass | fallthrough |
| 0x1C | 28 | 4 → `0x004D900E` | HANDLED |
| 0x1D | 29 | 6 → TechnoClass | fallthrough |
| 0x1E | 30 | 6 → TechnoClass | fallthrough |
| 0x1F | 31 | 6 → TechnoClass | fallthrough |
| 0x20 | 32 | 6 → TechnoClass | fallthrough |
| 0x21 | 33 | 6 → TechnoClass | fallthrough |
| 0x22 | 34 | 6 → TechnoClass | fallthrough |
| 0x23 | 35 | 5 → `0x004D8FD8` | HANDLED |

### 2.4 Cases NOT in FootClass (all fall through to TechnoClass)

Any message with param_3 < 0x11 OR param_3 in {0x14, 0x15, 0x16, 0x18, 0x19, 0x1A, 0x1B, 0x1D..0x22} OR param_3 > 0x23 falls through to `TechnoClass::Receive_Radio @ 0x006F4AB0`.

This includes: 0x02 (HELLO), 0x03 (BREAK), 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x0E, 0x0F, 0x10, 0x14, 0x15 (PREPARE), 0x16 (TIMING_SYNC), 0x18 (ENTER_DOCK), 0x19 (LEAVE_DOCK), 0x1A, 0x1B, 0x1C handled by TechnoClass, 0x1E, 0x1F, 0x20, 0x21, 0x22.

---

## 3. Vtable Binding Verification

- FootClass vtable base: `0x007E8C94` (from `FootClass::Constructor @ 0x004D345D`)
- Receive_Radio slot = vtable+0x194 → address `0x007E8C94 + 0x194 = 0x007E8E28`
- `read_memory 0x007E8E28`, 4 bytes → `B0 8F 4D 00` = little-endian `0x004D8FB0` ✓

**InfantryClass inherits FootClass::Receive_Radio:**
- InfantryClass vtable +0x194 at `0x007EB1EC` → `read_memory` → `B0 8F 4D 00` = `0x004D8FB0` ✓
  (verified in Phase 1 slot 5 report)

---

## 4. Case-by-Case Decode

> **Conventions:**
> - `this` = FootClass instance = `param_1` (int*, direct byte offsets = `param_1[i]` means byte offset `i*4`)
> - Byte offsets cited as `this+0xNN` (derived from decompile: `param_1[0xNN/4]` → `0xNN`)
> - `sender` = `param_2` (TechnoClass*)
> - `msg` = `param_3`
> - `payload` = `param_4` (int*)
> - All offsets verified from `decompile_function 0x004D8FB0` + `disassemble_function 0x004D8FB0`

---

### Case 0x11 — DRIVE_TO (Move-to-Cell dispatch / locomotor mission start)

**Entry point:** `0x004D9219`
**Expected sender:** Building (refinery/factory) — sends DRIVE_TO to a vehicle or aircraft
**Protocol name:** (unnamed in docs; closest is "DRIVE_TO" or "NAVIGATE")

**Logic (from decompile, entry at `0x004D9219`):**
```c
// case 0x11:
iVar5 = this->vtable[0x184]();   // GetCurrentMission()
if (iVar5 == 7) {                 // Mission 7 = Mission_Harvest or Mission_Move (needs verification)
    TechnoClass__Receive_Radio(sender, 0x11, payload);
    return 1;
}
// Also: check this+0xB4
if (param_1[0x2d] == 7) {        // this+0xB4 == 7
    TechnoClass__Receive_Radio(sender, 0x11, payload);
    return 1;
}
// else: fallthrough → TechnoClass::Receive_Radio (default path)
// The 'break' in the switch falls through to the common TechnoClass tail
```

**Disassembly confirms (at `0x004D9219`):**
```asm
004d9219: MOV EAX, [ESI]
004d921b: MOV ECX, ESI
004d921d: CALL [EAX+0x184]         ; GetCurrentMission()
004d9223: CMP EAX, 0x7             ; mission == 7?
004d9226: JZ  0x004d9235           ; yes → delegate to TechnoClass
004d9228: CMP dword ptr [ESI+0xb4], 0x7  ; this+0xB4 == 7?
004d922f: JNZ 0x004d90cc           ; no → TechnoClass default tail
004d9235: [push args for TechnoClass call]
004d9242: CALL 0x006f4ab0          ; TechnoClass::Receive_Radio
004d9247: [return 1]
```

**Side effects on `this`:** None in FootClass; TechnoClass base may have effects.

**Reply transmitted:** None from FootClass level; reply from TechnoClass base (returns 1 = ROGER if delegated, otherwise falls to TechnoClass default).

**Return value:**
- If (mission == 7) OR (`this+0xB4 == 7`): delegates to TechnoClass and returns 1 (ROGER)
- Otherwise: falls through to common TechnoClass tail at `0x004D90CC`

**Constants:**
- `0x7` = Mission 7 (likely `Mission_Harvest` for harvesters or `Mission_Move` — exact ID TBD; not verified in this session)
- `this+0xB4` = field checked for value 7 (role: team-related field or secondary mission state — not fully decoded; likely `TeamID` or `SubMission`)

**Active in YR:** YES — fires whenever any building sends msg 0x11 to a FootClass-derived entity (vehicle, infantry, aircraft).

---

### Case 0x12 — MOVE_TO_CELL (Accept assigned cell from building)

**Entry point:** `0x004D9139`
**Expected sender:** Building (refinery, factory) — sends the harvest/dock queue cell as payload
**Protocol name:** MOVE_TO_CELL (per RADIO_CLASS_PROTOCOL doc and BuildingClass case 0x0E)

**Logic (from decompile):**
```c
// case 0x12:
// Step 1: check if we're already at the target cell
if (*payload != 0) {
    // Get the target cell's coordinates (payload[0] = TechnoClass* pointing to target)
    piVar6 = (int *)(*payload->vtable[0x48])(local_c);  // GetCoords on payload object
    iVar5 = piVar6[0];   // target X (leptons)
    iVar1 = piVar6[1];   // target Y (leptons)
    // Get our own current cell (as packed shorts)
    psVar7 = (short *)(this->vtable[0x1b8])(&sender);   // Get_Cell_Packed → (cellX, cellY)
    // Convert target leptons → cell coords (sign-correct arithmetic shift)
    short cellX_target = (short)(iVar5 + (iVar5 >> 31 & 0xFF)) >> 8;
    short cellY_target = (short)(iVar1 + (iVar1 >> 31 & 0xFF)) >> 8;
    if (psVar7[0] == cellX_target && psVar7[1] == cellY_target) {
        return 0x14;   // ALREADY_THERE
    }
}

// Step 2: locomotor/mission state adjustments
iVar5 = this->vtable[0x184]();    // GetCurrentMission()
if (iVar5 == 5 && this+0xB4 == -1) {
    // Mission==GUARD (5) and no team: switch to Mission 2 (= Mission_Move or Mission_Attack?)
    this->vtable[0x1e8](2, 0);    // SetMission(2, 0)
}
// Fire action if current object-type == 7
if (this+0xB4 == 7) {
    if (this->vtable[0x200]() != 0) {   // CanFire()?
        this->vtable[0x1ec]();           // Fire() / DoAction()
    }
}

// Step 3: set destination and record timestamps
this->vtable[0x480](*payload, 1);    // SetDestination(*payload, forceOverride=1)
this+0xC8 (field_0xC8) = g_CurrentFrameCounter;   // [0x00a8ed84]
this+0xCC (field_0xCC) = iStack_10;  // stack slot (second part of 2-word timestamp)
this+0xD0 (field_0xD0) = 0;         // clear third field

return 1;
```

**Disassembly confirms at `0x004D91f1`:**
```asm
004d91f1: MOV EAX, [0x00a8ed84]     ; g_CurrentFrameCounter
004d91f6: MOV EDX, [ESP+0x20]       ; iStack_10 (stack frame value)
004d91fa: XOR ECX, ECX
004d91fc: ADD ESI, 0xc8              ; ESI = this+0xC8
004d9203: MOV [ESI], EAX             ; this+0xC8 = g_CurrentFrameCounter
004d9205: MOV EAX, 0x1
004d920a: MOV [ESI+0x4], EDX         ; this+0xCC = iStack_10
004d920d: MOV [ESI+0x8], ECX         ; this+0xD0 = 0
```

**Side effects on `this`:**
- `this+0xC8` written with `g_CurrentFrameCounter` (frame-timestamp of the MOVE_TO_CELL assignment)
- `this+0xCC` written with a stack-frame value (second word of frame timestamp structure — likely a tick count or sequence counter)
- `this+0xD0` cleared to 0
- `this->vtable[0x480](*payload, 1)` called — SetDestination to the payload cell/pointer
- If mission==5 and `this+0xB4==-1`: mission changed to 2

**Reply transmitted:** None (return value only)

**Return value:**
- `0x14` (ALREADY_THERE) if unit is already at the target cell
- `1` (ROGER) otherwise

**Constants:**
- `0x480` = `SetDestination` vtable slot (confirmed from UnitClass report)
- `0x1b8` = `Get_Cell_Packed` vtable slot (confirmed from VTABLE_BINDING report)
- `0x48` = `GetCoords` vtable slot
- `0x14` = ALREADY_THERE response code
- `5` = Mission_Guard
- `2` = Mission_Move (or Mission_Attack — exact ID not verified in this session)
- `0x1e8` = `SetMission` vtable slot
- `0x200` = `CanFire` vtable slot
- `0x1ec` = `Fire` / `DoAction` vtable slot
- `this+0xC8..0xD0` = 12-byte timestamp struct (frame counter + secondary counter + zero-pad)
- `this+0xB4` = team-ID or sub-mission state field (value -1 = no team; 7 = unknown state)

**Note on sign-correct arithmetic shift:** The coordinate conversion uses `(x + (x >> 31 & 0xFF)) >> 8` — this is the floor-correct lepton→cell conversion the game uses throughout (same as `GetCoords`/`Get_Cell_Packed` pattern).

**Active in YR:** YES — fires every time a building assigns a cell to a unit (refinery queue cell assignment, factory exit cell assignment, etc.). This is one of the highest-frequency FootClass radio cases.

---

### Case 0x13 — REQUEST_DOCK_CELL (Query: can you accept my dock request?)

**Entry point:** `0x004D90E8`
**Expected sender:** Building (refinery, factory) — queries whether the unit can dock
**Protocol name:** REQUEST_DOCK_CELL / NEED_TO_MOVE (per RADIO_CLASS_PROTOCOL doc, 0x13 = NEED_TO_MOVE)

**Logic (from decompile):**
```c
// case 0x13:
*payload = this+0x5A4;     // Write chrono-teleport destination field into payload
                           // (tells building what the unit's chrono target is)
if (this+0x5A4 != 0) {    // unit is chrono-teleporting (has a chrono destination)
    // Locomotor null check
    if (this+0x674 == 0) {     // ILocomotion ptr (= param_1[0x19d]*4 = param_1 at byte 0x674)
        GameDebugLog__Assert(0x80004003);
    }
    // Call ILocomotion::Is_Moving via vtable+0x10
    cVar3 = ILocomotion_vtable[0x10](this+0x674);  // Is_Moving()
    if (cVar3 != 0) {
        return 10;   // NEGATORY — chrono in-flight, still moving
    }
}
return 1;   // ROGER
```

**Disassembly confirms at `0x004D90E8`:**
```asm
004d90e8: MOV EDX, [ESP+0x34]         ; payload ptr
004d90ec: MOV ECX, [ESI+0x5a4]        ; this+0x5A4 (chrono destination)
004d90f2: MOV [EDX], ECX              ; *payload = this+0x5A4
004d90f4: MOV EAX, [ESI+0x5a4]
004d90fa: TEST EAX, EAX               ; if 0, skip locomotor check
004d90fc: JZ  0x004d9247              ; → return 1
004d9102: MOV EAX, [ESI+0x674]        ; ILocomotion* (at FootClass+0x674)
004d9108: TEST EAX, EAX
004d910a: JNZ 0x004d9116
004d910c: PUSH 0x80004003
004d9111: CALL 0x007dc720             ; GameDebugLog__Assert
004d9116: MOV ESI, [ESI+0x674]
004d911c: PUSH ESI
004d911d: MOV EAX, [ESI]              ; ILocomotion vtable
004d911f: CALL [EAX+0x10]             ; ILocomotion::Is_Moving()
004d9122: TEST AL, AL
004d9124: JZ  0x004d9247              ; not moving → return 1
; else: return 0xA (NEGATORY)
004d912a: POP EDI
004d912b: POP ESI
004d912c: POP EBP
004d912d: MOV EAX, 0xa
004d9132: POP EBX
004d9133: ADD ESP, 0x18
004d9136: RET 0xc
```

**Side effects on `this`:** None (read-only).
**Side effect on payload:** `*payload = this+0x5A4` (written before any guard).

**Reply transmitted:** None (return value only).

**Return value:**
- `1` (ROGER) if not chrono-teleporting OR chrono done (locomotor not moving)
- `10` (NEGATORY) if chrono-teleporting AND locomotor still moving

**Constants:**
- `this+0x5A4` = chrono-teleport destination pointer / chrono state (non-zero = actively chrono-teleporting). Also used in case 0x17 (same field).
- `this+0x674` = `ILocomotion*` COM pointer (FootClass locomotor interface)
- `ILocomotion::vtable+0x10` = `Is_Moving()` method
- `0x80004003` = E_POINTER HRESULT (asserted on null ILocomotion)

**Active in YR:** YES — fires for every harvester/aircraft dock admission poll (building side sends 0x13 to ask "can I send you toward the dock?"). The chrono guard is also active because chrono miners can receive this while warping inbound.

---

### Case 0x17 — DEPLOY_UNLOAD (Unit self-deploy / locomotor swap trigger)

**Entry point:** `0x004D902B`
**Expected sender:** Building (refinery) or any entity — tells unit to begin unload/deploy
**Protocol name:** DEPLOY_UNLOAD (matches UnitClass case 0x17 which delegates here for non-Weeder)

**Logic (from decompile):**
```c
// case 0x17:
// Step 1: if has valid path and destination == current chrono dest, clear destination
cVar3 = PathType__Has_Valid_Steps();   // 0x0065AE30
if (cVar3 != 0) {
    iVar5 = FootClass__GetDestination(0);   // 0x0065AD30
    if (this+0x5A4 == iVar5) {              // chrono dest matches current destination
        this->vtable[0x480](0, 1);          // SetDestination(NULL, forceOverride=1) — clear nav
    }
}

// Step 2: mission transitions
iVar5 = this->vtable[0x184]();    // GetCurrentMission()
if (iVar5 == 0) {                  // Mission == 0 (NONE/STOP)
    this->vtable[0x1e8](5, 0);    // SetMission(GUARD=5, 0)
    if (this->vtable[0x200]() != 0) {  // CanFire()?
        this->vtable[0x1ec]();          // Fire() / DoAction()
    }
}
iVar5 = this->vtable[0x184]();    // GetCurrentMission() again
if (iVar5 == 7) {
    this->vtable[0x1e8](5, 0);    // SetMission(GUARD=5, 0) if mission is 7
}

// Step 3: chrono + locomotor guard for the "Undock" transition
if (this+0x6AF != 0) goto fallthrough;  // chrono-teleporting: skip
if (this+0x5A4 != 0) goto fallthrough;  // also skip if chrono dest set
// Not chrono-teleporting: trigger locomotor change (likely exit-dock loco swap)
this->vtable[0x174](&DAT_008b3da8, 1, 1);  // ChangeLocomotorTo(nullCLSID, 1, 1)

// fall through to TechnoClass::Receive_Radio via break
```

**Disassembly confirms at `0x004D902B`:**
```asm
004d902b: MOV ECX, ESI
004d902d: CALL 0x0065ae30           ; PathType__Has_Valid_Steps
004d9032: TEST AL, AL
004d9034: JZ  0x004d9055            ; no valid path, skip
004d9036: PUSH 0x0
004d9038: MOV ECX, ESI
004d903a: CALL 0x0065ad30           ; FootClass__GetDestination(0)
004d903f: CMP [ESI+0x5a4], EAX     ; this+0x5A4 == GetDestination result?
004d9045: JNZ 0x004d9055
004d9047: MOV EDX, [ESI]
004d9049: PUSH 0x1
004d904b: PUSH 0x0
004d904d: MOV ECX, ESI
004d904f: CALL [EDX+0x480]          ; SetDestination(NULL, 1)
004d9055: [GetCurrentMission check for 0]
004d9059: CALL [EAX+0x184]
004d905f: TEST EAX, EAX
004d9061: JNZ 0x004d9088
004d9063: [SetMission(5,0)]
004d9070: [CanFire?]
004d907c: JZ  0x004d9088
004d907e: [Fire()]
004d9088: [GetCurrentMission check for 7]
004d9092: CMP EAX, 0x7
004d9095: JNZ 0x004d90a5
004d9097: [SetMission(5,0)]
004d90a5: MOV AL, [ESI+0x6af]      ; this+0x6AF chrono flag
004d90ab: TEST AL, AL
004d90ad: JNZ 0x004d90cc           ; chrono flag set → TechnoClass tail (no loco swap)
004d90af: MOV EAX, [ESI+0x5a4]    ; this+0x5A4 chrono dest
004d90b5: TEST EAX, EAX
004d90b7: JNZ 0x004d90cc           ; chrono dest set → skip
004d90b9: MOV EAX, [ESI]
004d90bb: PUSH 0x1
004d90bd: PUSH 0x1
004d90bf: PUSH 0x8b3da8            ; &DAT_008b3da8 (null CLSID, all zeros at static time)
004d90c4: MOV ECX, ESI
004d90c6: CALL [EAX+0x174]         ; vtable+0x174 (ChangeLocomotorTo or LocoSwap)
004d90cc: [TechnoClass::Receive_Radio tail]
```

**Side effects on `this`:**
- May clear destination via `SetDestination(NULL, 1)` if path valid and destination == chrono dest
- May change mission from 0 → 5 (GUARD) or from 7 → 5
- May call `vtable[0x1ec]` (Fire/DoAction)
- If not chrono-teleporting: calls `vtable[0x174](&DAT_008b3da8, 1, 1)` — locomotor swap with null CLSID

**Reply transmitted:** None directly; TechnoClass base called at end (may have side effects).

**Return value:** Falls through to TechnoClass::Receive_Radio return value (typically 1).

**Constants:**
- `this+0x5A4` = chrono destination pointer (non-null = actively chrono-teleporting; same field as case 0x13)
- `this+0x6AF` = chrono-teleporting flag (byte; non-zero = in-flight chrono)
- `0x174` = vtable slot for locomotor change (likely `ChangeLocomotorTo` or `LocoSwap`)
- `DAT_008b3da8` = 16-byte all-zeros CLSID (verified: `read_memory 0x008b3da8` → all zeros at static time; represents null/default locomotor class)
- `0x1e8` = `SetMission` vtable slot
- `0x480` = `SetDestination` vtable slot
- `0x200` = `CanFire` vtable slot
- `0x1ec` = `Fire`/`DoAction` vtable slot
- `5` = Mission_Guard
- `7` = Mission_Harvest (or Mission_Move — same as case 0x11)

**Active in YR:** YES — fires for any unit (vehicle, infantry, aircraft) receiving DEPLOY_UNLOAD. In practice this is the harvester deploying at the refinery, but also any unit being told to undeploy or switch locomotion.

---

### Case 0x1C — IS_REPAIRING (Health query: do you need repair?)

**Entry point:** `0x004D900E`
**Expected sender:** Repair building (UnitRepair/Hospital) or any entity querying repair eligibility
**Protocol name:** IS_REPAIRING (per RADIO_CLASS_PROTOCOL doc; also handled by ObjectClass::Receive_Radio @ 0x005F5320)

**Logic (from decompile):**
```c
// case 0x1C:
if (this+0x5A4 != 0) {    // chrono destination set (actively chrono-teleporting)
    return 10;             // NEGATORY — can't repair while chrono-teleporting
}
// else: break → fall through to TechnoClass::Receive_Radio
// TechnoClass case 0x1C does the actual health-ratio-based repair logic
```

**Disassembly confirms at `0x004D900E`:**
```asm
004d900e: MOV EAX, [ESI+0x5a4]    ; this+0x5A4 chrono dest
004d9014: TEST EAX, EAX
004d9016: JZ  0x004d90cc           ; no chrono → TechnoClass (real repair logic)
004d901c: [pop regs]
004d901f: MOV EAX, 0xa             ; return 10 (NEGATORY)
004d9024: [pop regs]
004d9028: RET 0xc
```

**Side effects on `this`:** None.
**Reply transmitted:** None.

**Return value:**
- `10` (NEGATORY) if chrono-teleporting (this+0x5A4 != 0)
- Otherwise: TechnoClass case 0x1C result (ROGER/NEGATORY based on health + spend-money logic)

**Constants:**
- `this+0x5A4` = chrono destination (same field across cases 0x13, 0x17, 0x1C)

**FootClass ADD vs TechnoClass:** FootClass ADDS a chrono gate before TechnoClass's health-ratio repair logic. Without this gate, a unit could be repair-polled while mid-chrono-warp. This is the only case where FootClass adds something substantive on top of TechnoClass for 0x1C.

**Active in YR:** YES — fires every time a repair building polls a unit asking "do you need repair?" Very common during UnitRepair dock sequences.

---

### Case 0x23 — IS_OCCUPIED (Carryall / transport capacity query)

**Entry point:** `0x004D8FD8`
**Expected sender:** Carryall, transport, or any entity querying occupancy
**Protocol name:** IS_OCCUPIED (per BuildingClass report case 0x0F and AircraftClass case 0x1D patterns)

**Logic (from decompile):**
```c
// case 0x23:
uVar4 = this->vtable[0x48](local_18);   // GetCoords() into stack buffer
CellClass__Get_Cell_At(uVar4);           // 0x00565730 — get CellClass at our position
iVar5 = Look_up_building_in_cell();      // 0x0047C520 — is there a building in this cell?
// Return: 1 + (building_found ? 9 : 0)
//   = 10 if in a building, 1 if not
return (-(uint)(iVar5 != unaff_retaddr) & 9) + 1;
```

**Disassembly confirms at `0x004D8FD8`:**
```asm
004d8fd8: MOV EDX, [ESI]
004d8fda: LEA EAX, [ESP+0x10]      ; local buffer for coords
004d8fde: PUSH EAX
004d8fdf: MOV ECX, ESI
004d8fe1: CALL [EDX+0x48]           ; GetCoords() into buffer
004d8fe4: PUSH EAX                  ; pass coord result
004d8fe5: MOV ECX, 0x87f7e8        ; MapClass singleton ptr?
004d8fea: CALL 0x00565730           ; CellClass__Get_Cell_At
004d8fef: MOV ECX, EAX
004d8ff1: CALL 0x0047c520           ; Look_up_building_in_cell()
004d8ff6: MOV EDI, [ESP+0x2c]      ; restore param_2
004d8ffa: SUB EAX, EDI             ; EAX - EDI (compare result to something)
004d8ffc: POP EDI
004d8ffd: NEG EAX
004d8fff: SBB EAX, EAX
004d9001: AND EAX, 0x9             ; 0 or 9
004d9002: POP ESI
004d9005: POP EBP
004d9006: INC EAX                  ; 1 or 10
004d9007: POP EBX
004d9008: ADD ESP, 0x18
004d900b: RET 0xc
```

**Note on return calculation:** The `NEG EAX; SBB EAX,EAX; AND EAX,0x9; INC EAX` idiom maps:
- `iVar5 != 0` (building found) → `EAX = 0x9 + 1 = 10` (NEGATORY — unit is inside a building)
- `iVar5 == 0` (no building) → `EAX = 0 + 1 = 1` (ROGER — unit is free/not inside building)

**Side effects on `this`:** None (read-only query).
**Reply transmitted:** None.

**Return value:** `1` (ROGER = unit is NOT inside a building) or `10` (NEGATORY = unit IS inside a building cell).

**Constants:**
- `vtable+0x48` = `GetCoords()` (leptons)
- `0x00565730` = `CellClass__Get_Cell_At`
- `0x0047C520` = `Look_up_building_in_cell`
- `0x87f7e8` = `g_MapClass` or map-related singleton pointer

**Active in YR:** YES — used during garrison entry, transport boarding, and carryall pickup sequences to check if the unit is currently inside a building. UnitClass case 0x24 (DOCK_QUERY) checks this before allowing carryall pickup.

---

### Case 0xC8 — TS-Legacy Verdict

**DOES NOT EXIST in FootClass::Receive_Radio.**

The switch covers param_3 range 0x11..0x23 (max jump = 0x12). Any param_3 ≥ 0x24 (including 0xC8 = 200) fails the `CMP EAX, 0x12` / `JA` guard at `0x004D8FC3` and falls through directly to `TechnoClass::Receive_Radio`. There is no case 0xC8 handler.

Verified: index table span is 19 bytes (`0x004D9274` to `0x004D9286`), covering only `param_3 ∈ [0x11, 0x23]`.

**Active in YR: N/A — case does not exist in this function.**

---

## 5. Q1/Q2 Transmit-Source Investigation

### Q1 — Who actually sends 0x19 LEAVE_DOCK?

**Answer: `TechnoClass::Receive_Radio` case 0x03 (BREAK) transmits 0x19 LEAVE_DOCK as a side effect.**

From `decompile_function 0x006F4AB0` (TechnoClass::Receive_Radio):
```c
case 3:  // BREAK
    if (this->DockedIn != 0 && sender+0x418 != 0) {
        // Both flags set: we are docked, and sender has a destination flag
        this->vtable[0x278](0x19, sender);   // Transmit(LEAVE_DOCK=0x19, sender)
        // then fall through to RadioClass::Receive_Radio (which nulls Contacts[] slot)
    }
    RadioClass__Receive_Radio(sender, 3, payload);
    return 1;
```

Where:
- `this->DockedIn` = bit at `param_1[6].UniqueID` = `this+0x18` (RadioClass base) + ObjectClass fields — the exact byte offset on TechnoClass is where DockedIn is stored (from Phase 1 slot 2: `param_1[6]` in `ObjectClass*` context = byte offset 0x18 from RadioClass base, which on TechnoClass resolves to the DockedIn byte)
- `sender+0x418` = `has-destination` flag on the sender (FootClass field at `this+0x418`)

**Chain for 0x19 in the harvester-refinery dock cycle:**
1. Refinery (or other building) sends BREAK (0x03) to harvester
2. FootClass case 0x03 is NOT handled → falls through to TechnoClass::Receive_Radio case 0x03
3. TechnoClass case 0x03 checks: if building has `DockedIn` set AND harvester has `+0x418` flag → transmit 0x19 LEAVE_DOCK back to the harvester
4. TechnoClass then calls `RadioClass::Receive_Radio` case 0x03 which nulls the contact slot

**Condition guards:** The 0x19 transmission only fires if BOTH:
- **Receiver's** (harvester's) `DockedIn` byte (`param_1[6].UniqueID`) is set — the entity *receiving* BREAK is the one whose DockedIn flag is checked, not the building's (corrected 2026-05-29: was "Building's DockedIn byte is set"; binary shows `TechnoClass__Receive_Radio` case 3 checks `(char)param_1[6].UniqueID` where `param_1` = entity receiving BREAK = harvester in the "refinery sends BREAK to harvester" path; confirmed via `decompile_function 0x006F4AB0` — INFERENCE_HARDENED)
- **Sender's** (refinery's) `+0x418` flag is set — `*(char *)(param_2 + 0x418)` where `param_2` = entity that sent BREAK = refinery; this is the *building's* `+0x418`, not the harvester's (corrected 2026-05-29: was "Sender (harvester) has this+0x418"; attribution was reversed — confirmed via `decompile_function 0x006F4AB0` — INFERENCE_HARDENED)

**Active in YR:** YES — fires every time a refinery breaks the dock link with a harvester that has completed its deposit.

---

### Q2 — Who actually sends 0x07 DOCKING_COMPLETE?

**Answer: One verified direct sender in this scope; it is not `Mission_Deploy_Building` and not `BuildingClass::MissionRepairAndProduce`.**

**Confirmed in prior research (Phase 1 slot 2, MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD report):**
- `Mission_Deploy_Building` does NOT send 0x07. Verified by exhaustive `PUSH 0x7` search in that function — no match found.

**Sender 1: `AircraftClass::Do_MISSION_MOVE_Carryall @ 0x00416D50`** — carryall pickup path.
- In sub-state 0 (VALIDATE_LZ): after HELLO (0x02) and WANT_RIDE (0x24) both return ROGER, sends `vtable[0x274](7)` (Transmit_Radio_ToFirst(0x07)) to the cargo unit.
- Verified in `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` §carryall pickup table.
- The msg 0x07 here signals "you are confirmed as my cargo — lock in", not "unloading complete".

**Refuted prior sender: `BuildingClass::MissionRepairAndProduce`** (UnitRepair path).
- `RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md` decompiled `BuildingClass::MissionRepairAndProduce @ 0x0044B780` and found that its `PUSH 0x7` sites call animation-slot helpers (`0x00451E40` / `0x00451890`), not radio transmit slots.
- The verified radio-like sends in that function use other message codes (`0x13`, `0x1C`, `0x1D`, `0x1F`, `0x03` depending on branch), not `0x07`.

**For standard refinery unload (HARV/CMIN):** No 0x07 is ever sent. The harvester's departure from the refinery is driven by `Mission_Deploy_Building` state transitions and `SetMission` calls on the harvester side — not a 0x07 radio signal from the refinery. UnitClass case 0x07 at `0x0073750a` exists and handles 0x07, but for the refinery path it is never reached in the standard ore loop.

**Active in YR:** CONDITIONAL. The carryall path is live when a carryall pickup occurs. Standard refinery unload does not send 0x07, and UnitRepair is no longer a confirmed 0x07 sender.

---

## 6. Cross-Cutting: Fields Touched

All offsets verified from `decompile_function 0x004D8FB0` + `disassemble_function 0x004D8FB0`.

| Field | Byte offset | Type | Cases that READ | Cases that WRITE | Notes |
|-------|-------------|------|----------------|-----------------|-------|
| chrono destination | `this+0x5A4` | ptr/int | 0x13, 0x17, 0x1C | — (read-only in FootClass) | Non-null = unit is chrono-teleporting |
| ILocomotion ptr | `this+0x674` | ILocomotion* | 0x13 | — | COM interface to locomotor; null-asserted before vtable call |
| chrono-teleporting flag | `this+0x6AF` | byte | 0x17 | — | Separate from +0x5A4 (both must be zero for loco swap) |
| team/sub-mission field | `this+0xB4` | int | 0x11, 0x12 | — | Value 7 = unknown state; -1 = no team |
| radio/contact flag | `this+0x418` | byte flag | — | — | Referenced in Q1 LEAVE_DOCK chain; set by TechnoClass radio 0x18 and cleared by radio 0x19, not by SetDestination |
| frame timestamp | `this+0xC8..0xD0` | 12-byte struct | — | 0x12 | Written: [0]=g_CurrentFrameCounter, [4]=stack value, [8]=0 |
| Current mission | via vtable+0x184 | int | 0x11, 0x12, 0x17 | — | Read to gate mission changes |

---

## 7. Diffs vs Phase 1 Slot 2 (TechnoClass Inline Decompile)

**TechnoClass::Receive_Radio @ 0x006F4AB0** handles a completely different set of cases: {3, 7, 8, 9, 0x16, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1E, 0x1F}.

**FootClass does NOT overlap with TechnoClass case IDs.** Every case FootClass handles (0x11, 0x12, 0x13, 0x17, 0x1C, 0x23) is outside the range TechnoClass handles (max TechnoClass case ID is 0x1F, but case 0x1C is shared — FootClass handles 0x1C first and only delegates to TechnoClass if not chrono-teleporting).

**FootClass adds vs TechnoClass:**

| Case | TechnoClass handling | FootClass addition |
|------|---------------------|--------------------|
| 0x11 | Not in TechnoClass | NEW: gate on mission==7 or this+0xB4==7 before delegating |
| 0x12 | Not in TechnoClass | NEW: full MOVE_TO_CELL acceptance + already-at-cell check + frame timestamp |
| 0x13 | Not in TechnoClass | NEW: chrono-gate + ILocomotion Is_Moving check |
| 0x17 | Not in TechnoClass | NEW: path clear + mission transitions + locomotor swap (if not chrono) |
| 0x1C | TechnoClass handles repair tick logic | FootClass ADDS chrono gate: if chrono-teleporting → NEGATORY before TechnoClass |
| 0x23 | Not in TechnoClass | NEW: building-in-cell occupancy check |

**TechnoClass cases that FootClass lets fall through (no interception):**
- 0x03 (BREAK), 0x07 (DOCKING_COMPLETE), 0x08 (REQUEST_DOCKING_CLEARANCE), 0x09, 0x16 (TIMING_SYNC), 0x18 (ENTER_DOCK), 0x19 (LEAVE_DOCK), 0x1A, 0x1B, 0x1E, 0x1F — all go directly to TechnoClass with no FootClass logic.

---

## 8. Tiny Details (Constants, Clamps, Write Orders, Edge Cases)

1. **`this+0xC8` frame-timestamp write order in case 0x12:** The three fields are written in order `[0]=framecount, [4]=iStack_10, [8]=0`. The `iStack_10` value comes from a local stack slot (ESP+0x20 in the disassembly) — its origin is not traced in this session. It may be the second word of a `FILETIME`-like 8-byte counter, or a tick-sequence counter. The zero-clear of `[8]` always follows.

2. **Case 0x12 lepton→cell conversion uses `(x + (x >> 31 & 0xFF)) >> 8`:** This is the floor-correct arithmetic-right-shift by 8 (matches `Get_Cell_Packed` used elsewhere). For negative coordinates, the correction `+ (x >> 31 & 0xFF)` ensures floor semantics rather than truncation.

3. **Case 0x17 double mission check:** GetCurrentMission() is called twice — first to check for mission==0, then again to check for mission==7. This is NOT a re-read for freshness; it's two separate checks in sequence, each potentially changing the mission before the next check. If mission starts at 0, it is changed to 5 (GUARD), and the second check at `iVar5==7` will NOT fire (since mission was just set to 5). The double-call is live but the second call can only fire if mission was already 7 on entry.

4. **Case 0x17 locomotor swap target is null CLSID (`DAT_008b3da8` = 16 bytes of zeros at static time):** This means `vtable[0x174](&nullCLSID, 1, 1)` is a "detach current locomotor / reset to default" call. The null CLSID arg likely signals "remove the piggyback locomotor" — consistent with harvester detaching the WalkLocomotor piggyback at dock exit.

5. **Case 0x1C has exactly one field check (this+0x5A4):** The gate is pure pass/fail on chrono state. No distance, mission, or type checking. This is intentionally minimal — the real repair logic is all in TechnoClass.

6. **Case 0x23 uses `Look_up_building_in_cell @ 0x0047C520` via `ECX=EAX` from `CellClass__Get_Cell_At @ 0x00565730`:** The cell is looked up by the unit's *current* (real-time) coords, not the sender's cell. So the query is always "is THIS unit inside a building right now?" — not "is the sender's cell occupied?"

7. **All 6 cases return at most 3 distinct non-delegate outcomes: 1 (ROGER), 10 (NEGATORY), 0x14 (ALREADY_THERE).** No other return codes are produced by FootClass itself before delegation.

8. **`this+0x6AF` and `this+0x5A4` are checked together in case 0x17** (disassembly: check 0x6AF first, then 0x5A4 separately). Both must be zero for the locomotor swap to fire. They represent two different aspects of chrono state: 0x6AF is the "currently teleporting" flag; 0x5A4 is the chrono destination pointer (set before 0x6AF clears). The double guard ensures the swap only fires after both are clear.

---

## 9. Open Questions — Final State

| # | Question | Status |
|---|----------|--------|
| OQ-1 | `this+0xB4` exact role (value 7 vs -1 in cases 0x11/0x12) — is it TeamID, SubMission, or another field? | OPEN — not traced in this session. Byte offset `0xB4` on FootClass is known from disassembly (`[ESI+0xb4]`). Not critical for Rust port — behavior is correct from decompile. |
| OQ-2 | `iStack_10` in case 0x12 write to `this+0xCC` — what is the source of this value? | OPEN — stack slot at ESP+0x20 in the disassembly. Origin not traced. Likely the high word of a 64-bit counter. |
| OQ-3 | `vtable+0x174` identity — is it `ChangeLocomotorTo` or another locomotor-swap variant? | OPEN — consistent with DriveLocomotion piggyback detach, but not decompiled in this session. MEMORY entry `project_force_track_bib_step.md` flags related locomotor wiring as deferred. |
| OQ-4 | Does `BuildingClass::MissionRepairAndProduce` actually send 0x07 for UnitRepair? | RESOLVED 2026-05-21: No. The apparent `PUSH 0x7` sites are animation-slot helper calls, not radio sends. See `RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md`. |
| OQ-5 | `g_MapClass ptr at 0x87f7e8` used in case 0x23 — is this the `g_TheMap` singleton? | OPEN — address used in `CellClass__Get_Cell_At` call. Likely `g_TheMap` or MapClass singleton. Not critical. |

---

## Sources

| Claim | Verification |
|-------|-------------|
| FootClass::Receive_Radio full decompile | `decompile_function 0x004D8FB0` |
| FootClass switch preamble + jump dispatch | `disassemble_function 0x004D8FB0` (`004d8fbd`–`004d8fd1`) |
| Jump table (7 entries) | `read_memory 0x004D9258, 28 bytes` → 7 targets confirmed |
| Index table (19 bytes, cases 0x11..0x23) | `read_memory 0x004D9274, 20 bytes` → case mapping confirmed |
| Vtable +0x194 → `0x004D8FB0` | `read_memory 0x007E8E28` → `B0 8F 4D 00` (also confirmed by Phase 1 slot 5) |
| `this+0x5A4` chrono dest (cases 0x13, 0x17, 0x1C) | `disassemble_function 0x004D8FB0`: `MOV ECX,[ESI+0x5a4]` at `004d90ec`, `004d900e`, `004d90af` |
| `this+0x674` ILocomotion ptr (case 0x13) | `disassemble_function 0x004D8FB0`: `MOV EAX,[ESI+0x674]` at `004d9102` |
| `this+0x6AF` chrono flag (case 0x17) | `disassemble_function 0x004D8FB0`: `MOV AL,[ESI+0x6af]` at `004d90a5` |
| `this+0xB4` team/state field (cases 0x11, 0x12) | `disassemble_function 0x004D8FB0`: `CMP [ESI+0xb4],7` at `004d9228`; `CMP [ESI+0xb4],-1` at `004d91a9` |
| `this+0xC8..0xD0` timestamp write (case 0x12) | `disassemble_function 0x004D8FB0`: `ADD ESI,0xc8; MOV [ESI],EAX; MOV [ESI+4],EDX; MOV [ESI+8],ECX` at `004d91fc`–`004d920d` |
| `g_CurrentFrameCounter @ 0x00a8ed84` (case 0x12) | `disassemble_function 0x004D8FB0`: `MOV EAX,[0x00a8ed84]` at `004d91f1`; `read_memory 0x00a8ed84` → 4 bytes confirmed |
| `DAT_008b3da8` = null CLSID (case 0x17) | `read_memory 0x008b3da8, 16 bytes` → all zeros |
| TechnoClass::Receive_Radio case set | `decompile_function 0x006F4AB0` (this session) |
| TechnoClass case 0x03 sends LEAVE_DOCK (0x19) | `decompile_function 0x006F4AB0`: case 3 transmits 0x19 when DockedIn && sender+0x418 |
| AircraftClass sends 0x07 in carryall VALIDATE_LZ | RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md §carryall table (from prior session) |
| Mission_Deploy_Building does NOT send 0x07 | MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md §4 (prior session) |
| BuildingClass::MissionRepairAndProduce does NOT send 0x07 | RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md |
| Case 0xC8 does not exist | Switch range = `[0x11, 0x23]`; `CMP EAX,0x12; JA` at `004d8fc3` eliminates any value > 0x23 |
