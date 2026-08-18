# RadioClass Core Primitives — Verification Report

**Addresses:** `0x0065A820` (Receive_Radio), `0x0065A970` (Transmit_Radio_Impl), `0x0065AAA0` (Transmit_Radio), `0x0065ACB0` (Transmit_Radio_ToFirst)
**Confidence:** High — all four functions decompiled and assembly spot-checked 2026-05-20
**Active in YR:** Yes — fundamental inter-unit handshake, active in every skirmish
**Supersedes / corroborates:** RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md (mostly corroborated; three minor corrections noted in §10)

---

## 1. Overview

All four base RadioClass primitives implement the synchronous RPC radio protocol.
No message queuing exists. `Transmit_Radio_Impl` calls the target's `Receive_Radio`
directly via vtable slot +0x194 and returns the response code on the stack.

Key fact: every subclass override either handles the message itself or tail-calls
back to RadioClass::Receive_Radio (or via TechnoClass/ObjectClass in between). The
HELLO/BREAK contact-list bookkeeping is always centralised here.

---

## 2. RadioClass Instance Layout — Contacts[] Offset and Slot Count

Verified from `RadioClass__Receive_Radio` decompile (`decompile_function 0x0065A820`)
and `RadioClass__Transmit_Radio_Impl` decompile (`decompile_function 0x0065A970`).

| Byte offset | Field                         | Init | Notes |
|-------------|-------------------------------|------|-------|
| +0xD4       | `RadioHistory[0]` (most recent) | 0  | Dedup log — most-recently-received message code |
| +0xD8       | `RadioHistory[1]`             | 0    | Second-most-recent |
| +0xDC       | `RadioHistory[2]`             | 0    | Third-most-recent |
| +0xE4       | `Contacts.data` (int* array)  | new  | Array of TechnoClass* radio partners |
| +0xE8       | `Contacts.Capacity`           | 1    | Iteration bound (not a live count); default 1 for all non-building Technos |
| +0xEC       | `Contacts.CanGrow`            | 1    | DynamicVector resize flag |
| +0xED       | `Contacts.Initialized`        | 1    | Ctor-completed flag |

Offsets derived from `int*` view in `Transmit_Radio_Impl` (`param_1[0x39]` =
`+0xE4`, `param_1[0x3a]` = `+0xE8`). Cross-checked against `Receive_Radio`
direct byte offsets (`param_1+0xE4`, `param_1+0xE8`) — consistent.

**Contact slot count:** Defaults to **1** for all non-building Technos.
`BuildingClass::Constructor` calls `RadioClass::Set_Contact_Count(NumberOfDocks)`
to grow the array — sole caller (verified `get_function_callers(0x0065AE60)`).

**No active-count field.** Walking `[0 .. Capacity)` and counting non-null entries
is the only way to get a live contact count.

**RadioHistory is a 3-slot linear push-down dedup log, not a ring.**
Shift order confirmed from decompile: old-D8→DC, old-D4→D8, new-msg→D4.
Duplicate messages (msg == History[0]) do NOT shift; they fall through to the
handler unchanged. The history does not prevent the handler from executing — it
only records the last-three distinct codes for future dedup by subclasses.

---

## 3. Receive_Radio (0x0065A820) — Base Switch Case-by-Case

Decompiled via `decompile_function 0x0065A820`; assembly spot-checked at
`get_assembly_context 0x0065a915` and `0x0065a950`.

**Active in YR:** Yes.

### Step A — RadioHistory shift (always runs first)

```c
if (msg != *(int*)(this + 0xD4)) {
    old_d8 = *(int*)(this + 0xD8);
    *(int*)(this + 0xD8) = *(int*)(this + 0xD4);
    *(int*)(this + 0xDC) = old_d8;
    *(int*)(this + 0xD4) = msg;
}
// Duplicates of msg == History[0]: skip shift, fall through to handler
```

**Tiny detail:** The shift writes D8→DC before overwriting D8. This is a safe
swap — no temporary needed by the compiler. The shift only fires on a new code;
repeated sends of the same code do not advance the history but still reach the
handler below.

### Step B — BREAK (msg == 0x03)

```c
for (i = 0; i < Contacts.Capacity; i++) {
    if (Contacts[i] == sender) {
        ObjectClass__Receive_Radio(sender, 3, payload);  // side-effects on sender-side
        Contacts[i] = NULL;  // null the slot AFTER the call
        return 1;  // ROGER
    }
}
// sender not found — fall through to ObjectClass::Receive_Radio (returns 0)
```

**Order matters:** `ObjectClass::Receive_Radio` is called on `sender` *before*
nulling the slot. ObjectClass handles only msgs 0x0D and 0x22 — for msg 0x03
it returns 0 with no side effects (verified `decompile_function 0x005F5320`).
The call is structurally there for extensibility but is a no-op on BREAK.

**Early-out:** Only the first matching slot is nulled. Loop stops immediately
on match and returns 1. If sender is not found in Contacts, falls to the
default tail (ObjectClass::Receive_Radio returns 0 → RadioClass returns 0).

**No compaction.** Slot stays null until next HELLO fills it.

### Step C — HELLO (msg == 0x02), with alive guard

```c
if (msg == 0x02 && *(int*)(this + 0x6C) != 0) {
    // First ally check — receiver's POV
    if (!HouseClass__Is_Ally_ByObject(this, sender)) return 10;  // NEGATORY
    // Second ally check — gated on AbstractFlags bit 0
    if (this != NULL && (*(byte*)(this + 0x14) & 1) && !HouseClass__Is_Ally_ByObject(sender, ?))
        return 10;
    // Idempotent check — sender already linked?
    if (sender != NULL) {
        for (i = 0; i < Capacity; i++)
            if (Contacts[i] == sender) return 1;  // ROGER, already linked
    }
    // Free-slot scan
    for (i = 0; i < Capacity; i++) {
        if (Contacts[i] == NULL) { Contacts[i] = sender; return 1; }
    }
    return 10;  // NEGATORY — all slots full
}
```

**Alive guard:** `*(int*)(this+0x6C) != 0` — if this field is 0 the entire HELLO
branch is skipped and falls to ObjectClass. Dead/uninitialized objects reject HELLO.

**Double ally check:** Two calls to `HouseClass__Is_Ally_ByObject` at `0x004F9A90`.
First checks from receiver's owner perspective; second is gated on `AbstractFlags & 1`
(bit 0 at `this+0x14`) — only fires when receiver has the flag set.

**Idempotent HELLO returns ROGER(1) immediately** when sender is already in
any slot (assembly verified at `0x0065a952: MOV EAX, 1; RET 0xc`).

**Free-slot scan is a second independent pass** — not combined with the
idempotent check. Two loops.

**No capacity grow.** If all slots are full and sender is not already linked,
returns NEGATORY(10). Growing contacts is the caller's (Transmit_Radio_Impl)
responsibility via eviction.

### Step D — Default tail

All other messages:
```c
return ObjectClass__Receive_Radio(sender, msg, payload);
```
ObjectClass handles only 0x0D (anim-stop) and 0x22 (IS_REPAIRING). Everything
else returns 0 from ObjectClass.

---

## 4. Transmit_Radio_Impl (0x0065A970) — Sender + Vtable+0x194 Dispatch

Decompiled via `decompile_function 0x0065A970`; assembly verified at
`get_assembly_context 0x0065a9db`.

**Active in YR:** Yes.

### Null-target default (runs first)

```c
if (target == NULL) {
    target = Contacts.data[0];       // param_1[0x39] → *(param_1 + 0xE4)
    if (target == NULL) return 0;    // no partner — silent no-op, return 0
}
```

**Default is Contacts[0] only**, not the first non-null slot. If Contacts[0]
is null but a later slot is occupied, the call silently returns 0.

### BREAK path (msg == 0x03)

```c
// Null all matching slots (does NOT stop at first match)
for (i = 0; i < Capacity; i++)
    if (Contacts[i] == target) Contacts[i] = NULL;
// Then dispatch Receive_Radio on target
goto dispatch;
```

**BREAK nulls ALL slots matching target** (not just the first). This matters
when the same object somehow occupies multiple contact slots (shouldn't happen
under normal logic, but the loop handles it defensively).

### HELLO path (msg == 0x02)

```c
freeSlot = -1;
for (i = 0; i < Capacity; i++) {
    if (freeSlot == -1 && Contacts[i] == NULL) freeSlot = i;
    if (Contacts[i] == target) return 1;  // already linked — ROGER, NO retransmit
}
if (freeSlot == -1) {
    // All slots full — evict slot 0 via Transmit_Radio(BREAK, Contacts[0])
    this->vtable[0x278](this, 3, Contacts[0]);  // Transmit_Radio, NOT Impl
    freeSlot = 0;
}
// Dispatch HELLO to target
result = target->vtable[0x194](target, Filter_AbstractType_InMap(this), 2, payload);
if (result == 1) { Contacts[freeSlot] = target; return 1; }
return 10;  // NEGATORY
```

**Critical:** Eviction calls `vtable[0x278]` (Transmit_Radio) not `vtable[0x27C]`
(Transmit_Radio_Impl) — this uses the subclass override if any exists.

**Already-linked detection in HELLO path:** Returns ROGER=1 immediately and
**does not re-send HELLO to target**. The Receive_Radio HELLO path also has an
idempotent check, but at Transmit_Radio_Impl level the check fires first and
short-circuits before the vtable dispatch.

### General dispatch (all other messages)

```c
dispatch:
    vtable_ptr = *target;
    filtered_sender = Filter_AbstractType_InMap(this);  // __thiscall on this
    return (**(code**)(vtable_ptr + 0x194))(filtered_sender, ...);
```

Assembly at `0x0065A9D9`: ECX=target (thiscall receiver), EAX=filtered_sender pushed
as stack arg, msg and payload also pushed. Verified via `get_assembly_context 0x0065a9db`.

### Filter_AbstractType_InMap (0x0040DD70)

`decompile_function 0x0040DD70` confirms: returns `this` if `What_Am_I()` ∈
{1=Unit, 2=Aircraft, 6=Building, 0xF=Infantry}, else returns NULL. This is
the sole RTTI filter on the sender — any non-Techno sender is silenced.

---

## 5. Transmit_Radio (0x0065AAA0)

Decompiled via `decompile_function 0x0065AAA0`.

**Active in YR:** Yes — all radio calls from mission code go through this or ToFirst.

```c
void RadioClass__Transmit_Radio(this, msg, target) {
    this->vtable[0x27C](this, msg, &g_RadioScratchBuffer, target);
    // return value from Transmit_Radio_Impl is NOT explicitly discarded by
    // the assembly — EAX passes through to caller. Ghidra types as void
    // due to no direct call-site return-value reads, but vtable callers
    // may read EAX.
}
```

**g_RadioScratchBuffer address:** `0x00A8EC30` (verified from assembly at
`0x0065aaab: PUSH 0xa8ec30`).

**Key difference from Transmit_Radio_Impl:** This is the public-facing wrapper
that supplies the shared global scratch buffer. Callers that need to pass custom
payload must call Transmit_Radio_Impl directly (vtable+0x27C).

**Return type ambiguity:** Ghidra annotates as `void`. The assembly does not
clear EAX before `RET 0x8`, so EAX carries whatever Transmit_Radio_Impl returned.
Callers via vtable+0x278 that examine EAX will get the Impl's return code.

---

## 6. Transmit_Radio_ToFirst (0x0065ACB0)

Decompiled via `decompile_function 0x0065ACB0`.

**Active in YR:** Yes.

```c
undefined4 RadioClass__Transmit_Radio_ToFirst(this, msg) {
    if (*(int*)Contacts.data[0] != 0) {
        return this->vtable[0x27C](this, msg, &g_RadioScratchBuffer, Contacts.data[0]);
    }
    return 0;
}
```

**Targets Contacts[0] only — no slot scan.** If Contacts[0] is NULL, returns 0
immediately. Does NOT search for the first non-null contact. For a building with
multi-dock Contacts[], if Contacts[0] is null and Contacts[1..N] are occupied,
this call silently returns 0 and nobody receives the message.

**Return value:** Propagates the inner Transmit_Radio_Impl result on success,
returns literal 0 on null-Contacts[0].

**Uses same g_RadioScratchBuffer** (`0x00A8EC30`) as Transmit_Radio.

---

## 7. Vtable+0x194 Binding Verification (CRITICAL)

Verified via `get_xrefs_to` on both Receive_Radio overrides to locate their vtable
slots, then `read_memory` on those slots, then vtable-base arithmetic.

### BuildingClass

- `get_xrefs_to 0x0043C2D0` → single DATA xref from `0x007E4050`
- `read_memory 0x007E4050 length=4` → bytes `D0 C2 43 00` = `0x0043C2D0` ✓
- Vtable base: `BuildingClass::Constructor` sets `*param_1 = &vtable_BuildingClass`.
  Slot 0 of that vtable (at `0x007E3EBC`) = `0x00410260` = `AbstractClass__QueryInterface` (verified `get_function_by_address 0x00410260`).
- Offset: `0x007E4050 - 0x007E3EBC = 0x194` ✓

**vtable_BuildingClass_base = 0x007E3EBC; slot +0x194 → 0x0043C2D0 (BuildingClass__Receive_Radio)**

### UnitClass

- `get_xrefs_to 0x00737430` → single DATA xref from `0x007F5E04`
- `read_memory 0x007F5E04 length=4` → bytes `30 74 73 00` = `0x00737430` ✓
- Vtable base: slot 0 at `0x007F5C70` = `0x00410260` = `AbstractClass__QueryInterface` (same as Building — shared AbstractClass vtable head).
- Offset: `0x007F5E04 - 0x007F5C70 = 0x194` ✓

**vtable_UnitClass_base = 0x007F5C70; slot +0x194 → 0x00737430 (UnitClass__Receive_Radio)**

**CONCLUSION: vtable slot +0x194 is definitively Receive_Radio for both BuildingClass and UnitClass.**

---

## 8. Tiny Details — Constants, Clamps, Off-by-Ones, Edge Cases

1. **BREAK in Receive_Radio early-exits on first match; BREAK in Transmit_Radio_Impl scans all slots.** These are two different behaviors for the same message at different call points. The receiver's BREAK (RadioClass::Receive_Radio) only clears one slot and returns early. The sender's BREAK (Transmit_Radio_Impl pre-dispatch) clears all matching slots before forwarding.

2. **Transmit_Radio_Impl null-target default uses Contacts[0], not first-non-null.** Subtle: if `target == NULL` is passed and `Contacts[0]` is null, return 0 even if `Contacts[1]` is valid.

3. **HELLO eviction path:** calls `vtable[0x278]` (Transmit_Radio, offset 0x278), not `vtable[0x27C]` (Transmit_Radio_Impl, offset 0x27C). This goes through the vtable, so a subclass that overrides Transmit_Radio gets called.

4. **g_RadioScratchBuffer** at `0x00A8EC30` — shared static global. Any Receive_Radio callee writing `*payload = x` clobbers this global. Safe in single-threaded gamemd; unsafe in any concurrent reimplementation.

5. **RadioHistory shift is a save-then-shift, not ring:** `tmp = D8; D8 = D4; DC = tmp; D4 = msg`. DC gets the value D8 had before D8 was overwritten. The history is a linear push-down of last-3-distinct codes, oldest at DC.

6. **Alive guard on HELLO (Receive_Radio):** `*(int*)(this + 0x6C) != 0` gates the entire HELLO branch. Field at +0x6C is in ObjectClass range. If this is zero (dead/uninitialized object), HELLO falls through to ObjectClass tail (returns 0).

7. **Capacity-underflow guard on HELLO (Receive_Radio):** `if (Contacts.Capacity < 1) return 10;` runs before the free-slot scan. A zero-capacity object always rejects HELLO.

8. **Return value of Transmit_Radio:** Ghidra types as `void`; assembly preserves EAX from Impl. The doc's `int` annotation in the vtable table is more accurate for callers that examine EAX after a vtable call.

9. **ObjectClass::Receive_Radio on BREAK:** Called with `sender` as the object (not `this`). ObjectClass handles only 0x0D and 0x22 — for 0x03 it returns 0 with no side effects. The call is present but benign.

---

## 9. Active-in-YR Analysis per Case

| Function | Active in YR | Evidence |
|----------|-------------|---------|
| Receive_Radio base | Yes | Every HELLO/BREAK from any Techno pair hits this layer |
| Transmit_Radio_Impl | Yes | All mission code dispatches radio through this |
| Transmit_Radio | Yes | Wrapper used in all mission-level sends (vtable+0x278) |
| Transmit_Radio_ToFirst | Yes | Used by Foot/Unit missions for single-partner sends |
| HELLO (0x02) case | Yes | Fires every dock cycle (harvester↔refinery, etc.) |
| BREAK (0x03) case | Yes | Fires on dock completion, unit destruction, order cancellation |
| Default tail (ObjectClass) | Yes | Fires for every non-HELLO/BREAK code at base layer |
| Ally double-check in HELLO | Yes | Fires every HELLO in MP; both players must be allied |
| Alive guard (+0x6C) | Yes | Guards every HELLO; fires constantly during normal play |
| Filter_AbstractType_InMap | Yes | Called on every Transmit_Radio_Impl dispatch |

---

## 10. Diffs vs RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md

### Corroborated (no change needed)

- RadioHistory offset layout (+0xD4/+0xD8/+0xDC), shift direction, dedup semantics ✓
- Contacts.data at +0xE4, Contacts.Capacity at +0xE8 ✓
- Default Capacity = 1 for non-buildings ✓
- BREAK sparse-null behavior (no compaction) ✓
- HELLO already-linked idempotent → ROGER without retransmit ✓
- HELLO full-slots → evict Contacts[0] via Transmit_Radio ✓
- vtable slot assignment (+0x194 Receive_Radio, +0x278 Transmit_Radio, +0x27C Transmit_Radio_Impl, +0x274 ToFirst) ✓
- Filter_AbstractType_InMap RTTI filter cases (1/2/6/0xF) ✓
- g_RadioScratchBuffer used by Transmit_Radio and ToFirst ✓
- Synchronous protocol (no queue) ✓

### Corrections / Precision additions

1. **Transmit_Radio return type:** Doc vtable table lists `int (this, int msg, TechnoClass* target)`. Ghidra decompiles as `void`. Assembly shows EAX is not cleared before RET — return value passes through from Transmit_Radio_Impl. True return type is effectively `int` propagated through EAX. Low behavioral impact since most callers use `ToFirst` or `Impl` directly for the return value.

2. **BREAK in Transmit_Radio_Impl clears ALL matching slots:** Doc's pseudocode shows a `Contacts[i] = NULL` write inside a loop that continues (not breaks) after the match — this is actually correct in the doc's pseudocode. Confirmed: the loop does NOT break early; it nulls every matching slot before dispatching. The doc's prose ("Null every slot that matches target") is accurate.

3. **Transmit_Radio_ToFirst null check is on Contacts[0] only — not first-non-null:** Doc says "sends to Contacts[0] implicitly" which is correct, but doesn't explicitly note that it returns 0 if Contacts[0] is null even when later slots are occupied. Added as a tiny detail in §8.

4. **ObjectClass::Receive_Radio call in BREAK (Receive_Radio side):** Doc pseudocode labels this as "sender-side side effects" but ObjectClass handles only 0x0D/0x22 — for BREAK it returns 0 with no side effects. Not wrong in the doc, but the side-effect implication is misleading. Clarified in §3.

5. **g_RadioScratchBuffer address verified:** `0x00A8EC30` (from assembly `PUSH 0xa8ec30`). Doc did not cite the address.

---

## 11. Open Questions — Final State

| # | Question | Status |
|---|----------|--------|
| a | Is each base case live in YR or TS-vestigial? | [RESOLVED] All four functions and all cases within them are live in standard YR skirmish. See §9. |
| b | What is Filter_AbstractType_InMap actually filtering? | [RESOLVED] RTTI filter: passes through Unit/Aircraft/Building/Infantry (What_Am_I ∈ {1,2,6,0xF}), returns NULL for all others. Verified `decompile_function 0x0040DD70`. |
| c | Is the RadioHistory dedup buffer still consulted in YR? | [RESOLVED] RadioHistory is written on every Receive_Radio call (non-duplicate), but the base RadioClass::Receive_Radio does NOT read it back — it only writes. Subclass overrides may read it. The shift is always active and always costs 2 writes when the message changes. No "skip handler on duplicate" logic at base layer — duplicates fall through normally. |
| d | What is the exact contact slot count? | [RESOLVED] Default = 1 for all non-building Technos. Buildings get `NumberOfDocks` slots (min 1) via `Set_Contact_Count` at `0x0065AE60`, sole caller `BuildingClass::Constructor`. |
| e | Contradictions with existing protocol doc? | [RESOLVED] No behavioral contradictions found. Three precision additions (return type propagation, Contacts[0]-only null check in ToFirst, ObjectClass BREAK is a no-op). See §10. |
| f | What is the `+0x6C` alive-guard field? | [DEFERRED — ObjectClass/MissionClass scope] Field at +0x6C is in the ObjectClass/MissionClass range. Likely an "in-map" or "active" flag. Not investigated here — different slot. |

---

## Sources

All findings from live Ghidra MCP decompilation 2026-05-20:

- `decompile_function 0x0065A820` — RadioClass::Receive_Radio
- `decompile_function 0x0065A970` — RadioClass::Transmit_Radio_Impl
- `decompile_function 0x0065AAA0` — RadioClass::Transmit_Radio
- `decompile_function 0x0065ACB0` — RadioClass::Transmit_Radio_ToFirst
- `decompile_function 0x0040DD70` — Filter_AbstractType_InMap
- `decompile_function 0x005F5320` — ObjectClass::Receive_Radio
- `get_assembly_context 0x0065a9db` — vtable+0x194 dispatch call site (BREAK path)
- `get_assembly_context 0x0065a8c4` — HELLO ally check call site
- `get_assembly_context 0x0065a915` — HELLO idempotent check + slot scan assembly
- `get_assembly_context 0x0065a950` — HELLO ROGER return assembly
- `get_assembly_context 0x0065AAA0` — Transmit_Radio entry point
- `get_xrefs_to 0x0043C2D0` → `0x007E4050` [DATA] — BuildingClass vtable slot
- `get_xrefs_to 0x00737430` → `0x007F5E04` [DATA] — UnitClass vtable slot
- `read_memory 0x007E4050 4` → `0x0043C2D0` — BuildingClass vtable+0x194 ✓
- `read_memory 0x007F5E04 4` → `0x00737430` — UnitClass vtable+0x194 ✓
- `read_memory 0x007E3EBC 8` → slot 0 = `0x00410260` = AbstractClass__QueryInterface — BuildingClass vtable base confirmed
- `read_memory 0x007F5C70 8` → slot 0 = `0x00410260` = AbstractClass__QueryInterface — UnitClass vtable base confirmed
- `get_function_by_address 0x00410260` → AbstractClass__QueryInterface (vtable base anchor)
- `get_function_by_address 0x0043C2D0` → BuildingClass__Receive_Radio (address confirmed)
- `get_function_by_address 0x00737430` → UnitClass__Receive_Radio (address confirmed)
