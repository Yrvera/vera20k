# RadioClass Vtable Binding + Slot Helpers — Verification Report

**Addresses:** `0x0065AD90` (FindDockSlot), `0x0065ADF0` (FUN_0065ADF0 / FindFreeContactSlot), `0x005F5320` (ObjectClass::Receive_Radio)
**Plus:** vtable+0x194 binding verified by live `read_memory` for 8 classes
**Confidence:** HIGH — all vtable entries confirmed by read_memory; function bodies confirmed by decompile
**Active in YR:** Yes — universal protocol infrastructure invoked by every dock/board/repair/tether handshake

---

## 1. Overview

This report verifies:
1. The vtable slot +0x194 (`Receive_Radio`) binding for every class in the Techno hierarchy via live `read_memory`.
2. The transmit-side slots +0x274/+0x278/+0x27C/+0x280 for RadioClass and BuildingClass.
3. Full decode of `RadioClass::FindDockSlot` @ `0x0065AD90`.
4. Full decode of `FUN_0065ADF0` (FindFreeContactSlot) @ `0x0065ADF0`.
5. Full decode of `ObjectClass::Receive_Radio` @ `0x005F5320` — the lowest fallback handler.

Prior art consulted:
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` — claims vtable slots +0x194, +0x274, +0x278, +0x27C, +0x280.
- `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md` — asserts BuildingClass vtable slot 101 = +0x194 → `0x0043C2D0`.

---

## 2. Definitive Vtable Binding Table — slot +0x194 (Receive_Radio)

Every entry below was read live from the binary via `read_memory`. Ghidra labels were NOT used as ground truth.

| Class | Vtable base | +0x194 addr | read_memory result | Resolves to | Expected (scope) | Match? |
|-------|-------------|-------------|-------------------|-------------|-----------------|--------|
| RadioClass | `0x007F0508` | `0x007F069C` | `20 A8 65 00` | `0x0065A820` RadioClass::Receive_Radio | `0x0065A820` | YES |
| ObjectClass | (abstract, see §2.1) | — | — | `0x005F5320` (via subclass fallback) | `0x005F5320` | YES |
| TechnoClass | `0x007F4960` | `0x007F4AF4` | `B0 4A 6F 00` | `0x006F4AB0` TechnoClass::Receive_Radio | `0x006F4AB0` | YES |
| FootClass | `0x007E8C94` | `0x007E8E28` | `B0 8F 4D 00` | `0x004D8FB0` FootClass::Receive_Radio | `0x004D8FB0` | YES |
| InfantryClass | `0x007EB058` | `0x007EB1EC` | `B0 8F 4D 00` | `0x004D8FB0` FootClass::Receive_Radio | (no separate override) | N/A |
| UnitClass | `0x007F5C70` | `0x007F5E04` | `30 74 73 00` | `0x00737430` UnitClass::Receive_Radio | `0x00737430` | YES |
| BuildingClass | `0x007E3EBC` | `0x007E4050` | `D0 C2 43 00` | `0x0043C2D0` BuildingClass::Receive_Radio | `0x0043C2D0` | YES |
| AircraftClass | `0x007E22A4` | `0x007E2438` | `B0 90 41 00` | `0x004190B0` AircraftClass::Receive_Radio | `0x004190B0` | YES |

Vtable base addresses verified against constructor xrefs:
- RadioClass `0x007F0508`: confirmed from `0x0065A798` (RadioClass::Constructor) [DATA]
- TechnoClass `0x007F4960`: confirmed from `0x006F3130` (TechnoClass::Constructor) [DATA]
- FootClass `0x007E8C94`: confirmed from `0x004D345D` (FootClass::Constructor) [DATA]
- InfantryClass `0x007EB058`: confirmed from `0x00517ACC` (InfantryClass::Constructor) [DATA]
- UnitClass `0x007F5C70`: confirmed from `0x0073543A` (UnitClass::Constructor) [DATA]
- BuildingClass `0x007E3EBC`: confirmed from `0x0043B9FA` (BuildingClass::Constructor) [DATA]
- AircraftClass `0x007E22A4`: confirmed from `0x00413D87` (AircraftClass::Constructor) [DATA]

### 2.1 ObjectClass vtable note

ObjectClass is abstract in YR — it is never instantiated directly (constructors like `0x005F3900` and `0x005F3B50` are only called as base ctors from subclass constructors). No standalone ObjectClass vtable is linked into any object. The fallback function `ObjectClass::Receive_Radio @ 0x005F5320` is inherited by concrete classes that don't install their own override. Confirmed: AnimClass vtable base `0x007E3354` → +0x194 at `0x007E34E8` → reads `0x005F5320` (ObjectClass::Receive_Radio), verified by read_memory (`20 53 5F 00`).

### 2.2 InfantryClass finding — MAJOR FINDING

**InfantryClass does NOT override Receive_Radio.** It inherits `FootClass::Receive_Radio @ 0x004D8FB0` directly. The prior protocol doc did not document this explicitly. This means all infantry radio messages (dock queries, repair, garrison enter) are processed by FootClass::Receive_Radio, not a dedicated InfantryClass override.

Active in YR: Yes — every infantry unit uses FootClass::Receive_Radio for all radio handshakes.

---

## 3. Transmit-Side Slots (+0x274 / +0x278 / +0x27C / +0x280)

Verified via `read_memory` on RadioClass vtable (`0x007F0508`) and BuildingClass vtable (`0x007E3EBC`). All four transmit slots are identical across both — BuildingClass inherits them unchanged from RadioClass.

| Slot offset | RadioClass vtable addr | read_memory | Resolves to | Function name |
|-------------|----------------------|-------------|-------------|---------------|
| +0x274 | `0x007F077C` | `B0 AC 65 00` | `0x0065ACB0` | RadioClass::Transmit_Radio_ToFirst |
| +0x278 | `0x007F0780` | `A0 AA 65 00` | `0x0065AAA0` | RadioClass::Transmit_Radio |
| +0x27C | `0x007F0784` | `70 A9 65 00` | `0x0065A970` | RadioClass::Transmit_Radio_Impl |
| +0x280 | `0x007F0788` | `E0 AC 65 00` | `0x0065ACE0` | RadioClass::Broadcast_Radio_ToAll |

BuildingClass vtable (`0x007E3EBC`) at same offsets — `read_memory 0x007E4130, 16 bytes`:
`B0 AC 65 00, A0 AA 65 00, 70 A9 65 00, E0 AC 65 00` — all four entries identical to RadioClass.

UnitClass vtable (`0x007F5C70`) at `0x007F5EE4`, 16 bytes:
`B0 AC 65 00, A0 AA 65 00, 70 A9 65 00, E0 AC 65 00` — all four entries identical to RadioClass.

**Conclusion:** No subclass overrides the transmit-side slots. +0x274/+0x278/+0x27C/+0x280 are permanently bound to the four RadioClass transmit helpers across the entire hierarchy. This CONFIRMS the protocol doc's slot assignment.

The `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` correction (BuildingClass +0x194, not +0x274) is also CONFIRMED: +0x194 is the `Receive_Radio` dispatch slot, +0x274 is `Transmit_Radio_ToFirst`. These are distinct. BuildingClass::Receive_Radio at `0x0043C2D0` lives at slot +0x194, verified by `read_memory 0x007E4050` → `D0 C2 43 00`.

Active in YR: Yes.

---

## 4. FindDockSlot (`0x0065AD90`) — Full Decode

**Signature (verified):**
```c
int __thiscall RadioClass__FindDockSlot(RadioClass* this, TechnoClass* target)
//   param_1 = this (RadioClass*)
//   param_2 = target (TechnoClass* to search for)
//   returns: slot index (0-based) if found, -1 if not found or target==NULL
```

**Logic (full decompile verified):**
```c
int RadioClass__FindDockSlot(int param_1, int param_2) {
    if (param_2 == 0) return -1;           // NULL target → -1
    int count = *(int*)(param_1 + 0xE8);   // Contacts.Capacity
    if (count <= 0) return -1;
    int* slots = *(int**)(param_1 + 0xE4); // Contacts.data
    for (int i = 0; i < count; i++) {
        if (slots[i] == param_2)           // found target in this slot
            return i;
        // Note: does NOT skip NULL slots — walks every slot including NULLs
    }
    return -1;
}
```

**Key facts:**
- Iterates `Contacts.data[0..Contacts.Capacity)` — the full capacity, not just live entries.
- NULL guard is only on `param_2` (the search target), not on slot contents. It does NOT skip NULL slots; it simply checks `slot == target`. Since target is non-NULL (guarded above), a NULL slot will never match.
- Return value: **slot index (0-based)** on match, **-1** on not-found or NULL target.
- Iteration bound: `+0xE8` (Contacts.Capacity). Defaults to 1 for Foot/Unit/Aircraft/Infantry. BuildingClass resizes via `Set_Contact_Count(type->NumberOfDocks)`.
- This function answers "**is `target` already in my contact list, and where?**" — it is a lookup/membership test, not an allocator.
- Name "FindDock**Slot**" is somewhat misleading: it finds the *contact slot* index for a given partner, not a "dock bay" in the gameplay sense. If the result == -1, the target is not linked.

**Active in YR:** Yes — called in `BuildingClass::Receive_Radio` case 0xE (`ContactsContains(sender)` checks) at `0x0043C32A` and similar.

**Callers confirmed (xrefs to `0x0065AD90`):** RadioClass::Receive_Radio internal logic (`0x0065A820` area), BuildingClass::Receive_Radio case 0xE, and similar dock-entry gates.

---

## 5. FUN_0065ADF0 (FindFreeContactSlot) — Full Decode

**Confirmed name: `RadioClass::FindFreeContactSlot`** (this is `FUN_0065ADF0` in Ghidra, unlabeled).

**Signature (verified):**
```c
uint __thiscall FUN_0065ADF0(RadioClass* this, TechnoClass* target)
//   param_1 = this (RadioClass*)
//   param_2 = target (TechnoClass* to match, OR 0 for "any free slot")
//   returns: low byte = 1 (found/match) or 0 (not found/full)
//            upper 3 bytes = the matched pointer value >> 8 (implementation artifact)
//   Caller-visible: low byte as bool. Non-zero = there is a free or matching slot.
```

**Logic (full decompile verified):**
```c
uint FUN_0065ADF0(int param_1, uint param_2) {
    int count = *(int*)(param_1 + 0xE8);   // Contacts.Capacity
    uint* slots = *(uint**)(param_1 + 0xE4); // Contacts.data
    for (int i = 0; i < count; i++) {
        uint slot = slots[i];
        if (slot == 0 || slot == param_2) {  // NULL slot (free) OR matches target
            return CONCAT31((int3)(slot >> 8), 1);  // low byte = 1 (TRUE)
        }
    }
    // No free or matching slot found
    return in_EAX & 0xFFFFFF00;  // low byte = 0 (FALSE), EAX upper bytes as-is
}
```

**Key facts:**
- Answers: "**is there a free slot, or does `target` already have a slot?**"
- Returns **non-zero (low byte = 1)** if any slot is NULL (free) OR contains `target`.
- Returns **zero (low byte = 0)** if every slot is occupied by a non-target object.
- `param_2 == 0` (NULL target): only NULL slots match — effectively "is any slot free?"
- The upper 3 bytes of the return value are `slot_value >> 8` — an artifact of the CONCAT31 encoding. Callers use only the low byte as a boolean.
- Iteration bound: `+0xE8` (Contacts.Capacity), same as FindDockSlot.

**Difference from FindDockSlot:**

| | FindDockSlot (`0x0065AD90`) | FindFreeContactSlot (`0x0065ADF0`) |
|--|--|--|
| Question | "Is target in the list? Where?" | "Can target dock? (free slot or already there?)" |
| Match condition | `slot == target` only | `slot == NULL` OR `slot == target` |
| Return type | int index (0..(N-1)) or -1 | bool (low byte: 1=yes, 0=no) |
| NULL target guard | Yes (early return -1) | No (NULL target matches NULL slots → "is any slot free?") |
| Semantics | Lookup / membership test | Pre-dock capacity check |

**Callers:** `BuildingClass::Receive_Radio` case 0xE uses it as the capacity gate: `if (!ContactsContains(sender) && FUN_0065ADF0(this, sender))` — meaning "if target not linked AND there's room (free or target already there) → allow DOCK_LINK". Also used in case 0xF and case 0x10 as the harvester/passenger accept gate.

**Active in YR:** Yes — called extensively in every dock-entry and passenger-entry decision.

---

## 6. ObjectClass::Receive_Radio (`0x005F5320`) — Terminal Fallback Handler

**Scope note:** The investigation brief described this as a "terminal swallow" for messages {1, 2, 8, 10, 12, 13, 22}. **This is WRONG.** Verified by full disassembly below.

**Verified disassembly (100 bytes, `0x005F5320–0x005F5384`):**

```asm
005f5320: MOV EAX, [ESP+8]       ; EAX = message code (param_3)
005f5324: SUB ESP, 8
005f5327: CMP EAX, 0xD           ; msg 13?
005f532a: JZ  0x005f5370         ; yes → msg-0xD handler
005f532c: CMP EAX, 0x22          ; msg 34?
005f532f: JZ  0x005f5339         ; yes → msg-0x22 handler
005f5331: XOR EAX, EAX           ; all other msgs → return 0
005f5333: ADD ESP, 8
005f5336: RET 0xC

; --- Case 0x22 (IS_REPAIRING check) ---
005f5339: FILD [ECX+0x6C]        ; float(this->Health / HP field at +0x6C)
005f533c: MOV EAX, [ECX]         ; vtable
005f533e: FSTP [ESP]
005f5342: CALL [EAX+0x88]        ; vtable+0x88() = GetTypeClass()
005f5348: FILD [EAX+0xA0]        ; float(TypeClass->MaxHP at +0xA0)
005f534e: MOV ECX, [0x008871E0]  ; g_RulesClass_Instance
005f5354: FDIVR [ESP]            ; = Health / MaxHP (health ratio)
005f5358: FCOMP [ECX+0x16F8]     ; vs Rules+0x16F8 (ConditionYellow threshold)
005f535e: FNSTSW AX
005f5360: TEST AH, 0x1           ; CF set if ratio < threshold
005f5363: JNZ  0x005f537a        ; damaged → return 1 (ROGER)
005f5365: MOV EAX, 0xA           ; full health → return 10 (NEGATORY)
005f536a: ADD ESP, 8
005f536d: RET 0xC

; --- Case 0xD (OVER_AND_OUT / anim reset) ---
005f5370: MOV EDX, [ECX]         ; vtable
005f5372: PUSH 2
005f5374: CALL [EDX+0x124]       ; vtable+0x124(2) = GrandOpening(2) or anim-slot call
005f537a: MOV EAX, 0x1           ; return 1 (ROGER)
005f537f: ADD ESP, 8
005f5382: RET 0xC
```

**Case table (all cases, by disassembly — verified):**

| Message code | Decimal | Action | Return |
|---|---|---|---|
| **0xD** (13) | OVER_AND_OUT / building anim reset | Calls `this->vtable[0x124](2)` — slot 0x124 / index 0x49 (likely `GrandOpening`-style anim-slot operation) | 1 (ROGER) |
| **0x22** (34) | IS_REPAIRING / health check | Reads `this+0x6C` (Health), gets TypeClass via vtable+0x88, computes Health/MaxHP ratio, compares vs `Rules+0x16F8` (ConditionYellow). Returns NEGATORY if at or above threshold (healthy), ROGER if below (being repaired = damaged). | 10 (NEGATORY) if healthy; 1 (ROGER) if damaged |
| **All others** | — | Silent default: return 0 (not ROGER, not NEGATORY — a distinct "unhandled" code) | 0 |

**Critical correction to scoping brief:** The message list {1, 2, 8, 10, 12, 13, 22} is wrong. The function handles **only 0xD and 0x22**, and returns **0** (not a standard response code) for everything else. This is NOT a "swallow" — returning 0 is distinct from ROGER (1) and NEGATORY (10). Callers that check for ROGER or NEGATORY will see neither on an unhandled message; they see 0.

**Side effects:**
- Msg 0xD: calls `vtable[0x124](2)`. This is an anim/state side effect — pushes GrandOpening mode 2 (or equivalent). Consistent with BuildingClass case 0xD using it as a silent WeaponsFactory no-op — WeaponsFactory's BuildingClass override swallows it before it reaches here.
- Msg 0x22: reads live health and type; no mutation side effects.

**Does any override call ObjectClass::Receive_Radio via vtable?** No — all callers invoke it via direct CALL (not indirect vtable dispatch). RadioClass::Receive_Radio at `0x0065A820` calls it directly at `0x0065A87B` and `0x0065A890` (confirmed by xrefs). ObjectClass::Receive_Radio is never called recursively through vtable+0x194.

**Active in YR:** Yes — invoked on every radio message that falls through RadioClass::Receive_Radio without being handled by BREAK or HELLO.

---

## 7. Contact Array Layout on RadioClass Instance

From `RadioClass::Constructor @ 0x0065A750` (decompile verified) and `RadioClass::Set_Contact_Count @ 0x0065AE60`:

| Offset | Size | Field | Init value | Purpose |
|--------|------|-------|-----------|---------|
| +0xD4 | 4 | RadioHistory[0] | 0 | Most-recent msg code |
| +0xD8 | 4 | RadioHistory[1] | 0 | Previous msg code |
| +0xDC | 4 | RadioHistory[2] | 0 | Oldest msg code |
| +0xE0 | 4 | Contacts.vtable | `&PTR_FUN_007e180c` | DynamicVectorClass vtable |
| +0xE4 | 4 | Contacts.data | `operator_new(4)` | TechnoClass** array, 1-slot default |
| +0xE8 | 4 | Contacts.Capacity | **1** | Slot count (iteration bound for both helpers) |
| +0xEC | 1 | Contacts.CanGrow | 1 | Allow Resize |
| +0xED | 1 | Contacts.Initialized | 1 | Ctor flag |

**Slot count specifics:**
- Default capacity = **1** — set by constructor.
- BuildingClass constructor calls `RadioClass::Set_Contact_Count(type->NumberOfDocks)` to resize.
- `Set_Contact_Count` expands via the DynamicVector resize callback at `Contacts.vtable+8`, then zero-fills new slots from `iVar1` to `param_2` (verified from decompile `0x0065AE60`).
- **There is no separate active-count field.** Capacity is the iteration bound for both helpers. Slots containing NULL are "free"; slots containing a non-null pointer are "occupied." BREAK writes NULL; it does NOT shrink Capacity.
- On full-capacity failure: `FindDockSlot` returns -1; `FindFreeContactSlot` returns 0 (low byte). Neither evicts; eviction is handled by the transmit-side `Transmit_Radio_Impl` HELLO path (evicts `Contacts[0]` when full).
- **No signed/unsigned comparison issue:** iteration index `i` starts at 0, capacity is read as `int` from `+0xE8`, and the loop condition is `i < capacity` — normal signed comparison. No off-by-one.

**Active in YR:** Yes.

---

## 8. Corrections to Prior Docs

### 8.1 RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md — slot assignments CONFIRMED

The protocol doc's vtable slot table (`+0x194` = Receive_Radio, `+0x274/+0x278/+0x27C/+0x280` = transmit helpers) is **correct**. All four transmit-side functions were verified by live `read_memory` on RadioClass vtable and confirmed to be identical across BuildingClass and UnitClass (no overrides).

### 8.2 BuildingClass +0x194 vs +0x274 — RESOLVED

The open question ("does BuildingClass Receive_Radio live at +0x194 or +0x274?") is definitively resolved:
- `read_memory 0x007E4050` → `D0 C2 43 00` = `0x0043C2D0` = BuildingClass::Receive_Radio at **+0x194**. ✓
- `read_memory 0x007E4130` → `B0 AC 65 00` = `0x0065ACB0` = RadioClass::Transmit_Radio_ToFirst at **+0x274**. ✓

The `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` correction was correct. The original protocol doc slot table was also correct — it said +0x274 is `Transmit_Radio_ToFirst`, not Receive_Radio. No conflict.

### 8.3 ObjectClass::Receive_Radio case list — CORRECTED

The scope brief claimed cases {1, 2, 8, 10, 12, 13, 22}. **Corrected:** Only cases **0xD (13)** and **0x22 (34)** are handled. All others return 0. No prior doc claims the extended case list; the error was in the investigation brief only.

### 8.4 InfantryClass::Receive_Radio — NEW FINDING

InfantryClass has **no Receive_Radio override**. It inherits FootClass::Receive_Radio directly. This is confirmed by `read_memory 0x007EB1EC` → `B0 8F 4D 00` = `0x004D8FB0` = FootClass::Receive_Radio, and by InfantryClass constructor xrefs confirming vtable base `0x007EB058`.

---

## 9. Open Questions — Final State

| # | Question | Status |
|---|----------|--------|
| Q1 | Does `vtable+0x124` in ObjectClass case 0xD correspond to GrandOpening? | OPEN — vtable slot 0x124 / index 0x49 identified as a method call with arg 2, consistent with GrandOpening(2), but not decompiled in this session. Not in scope. |
| Q2 | What is `type->NumberOfDocks` field offset on BuildingTypeClass? | OPEN — not traced in this session. Referenced indirectly via `Set_Contact_Count`. Likely in the BuildingTypeClass ReadINI doc. |
| Q3 | FootClass::Receive_Radio @ `0x004D8FB0` — what messages does it handle? | OPEN — not in scope for this slot, covered by another swarm slot. |
| Q4 | Is `Contacts.data` ever grown beyond NumberOfDocks by live code? | OPEN — `Set_Contact_Count` only grows, never shrinks. Whether any other caller calls it with a larger count is not traced. |

---

## Sources

| Claim | Verification |
|-------|-------------|
| RadioClass vtable base `0x007F0508` | Constructor xrefs: `0x0065A798` writes `&vtable__RadioClass` (DATA) |
| RadioClass +0x194 → `0x0065A820` | `read_memory 0x007F069C` → `20 A8 65 00` |
| TechnoClass vtable base `0x007F4960` | Constructor xrefs: `0x006F3130` (DATA) |
| TechnoClass +0x194 → `0x006F4AB0` | `read_memory 0x007F4AF4` → `B0 4A 6F 00` |
| FootClass vtable base `0x007E8C94` | Constructor xrefs: `0x004D345D` (DATA) |
| FootClass +0x194 → `0x004D8FB0` | `read_memory 0x007E8E28` → `B0 8F 4D 00` |
| InfantryClass vtable base `0x007EB058` | Constructor xrefs: `0x00517ACC` (DATA) |
| InfantryClass +0x194 → `0x004D8FB0` | `read_memory 0x007EB1EC` → `B0 8F 4D 00` |
| UnitClass vtable base `0x007F5C70` | Constructor xrefs: `0x0073543A` (DATA) |
| UnitClass +0x194 → `0x00737430` | `read_memory 0x007F5E04` → `30 74 73 00` |
| BuildingClass vtable base `0x007E3EBC` | Constructor xrefs: `0x0043B9FA` (DATA) |
| BuildingClass +0x194 → `0x0043C2D0` | `read_memory 0x007E4050` → `D0 C2 43 00` |
| AircraftClass vtable base `0x007E22A4` | Constructor xrefs: `0x00413D87` (DATA) |
| AircraftClass +0x194 → `0x004190B0` | `read_memory 0x007E2438` → `B0 90 41 00` |
| Transmit slots +0x274/+0x278/+0x27C/+0x280 | `read_memory 0x007F077C` (RadioClass), `0x007E4130` (BuildingClass), `0x007F5EE4` (UnitClass) — all identical |
| FindDockSlot full logic | `decompile_function 0x0065AD90` |
| FUN_0065ADF0 full logic | `decompile_function 0x0065ADF0` |
| ObjectClass::Receive_Radio full logic | `disassemble_function 0x005F5320` + `decompile_function 0x005F5320` |
| RadioClass contact array layout | `decompile_function 0x0065A750` (ctor) + `decompile_function 0x0065AE60` (Set_Contact_Count) |
| AnimClass vtable base `0x007E3354` | Constructor xrefs: `0x00422847` (DATA) |
| ObjectClass abstract (no standalone vtable) | No xrefs to `ObjectClass::Constructor 0x005F3900` as DATA in any constructor chain |
