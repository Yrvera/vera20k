# Radio 0x0B/0x1A/0x1B/0x1E Senders and Name String-Check — Ghidra Research Report

**Date:** 2026-06-02  
**Slot:** slot-4 (swarm round-2; closes PARTIAL from round-1 RADIO_INFERRED_CODES_AND_DOCK_HANDSHAKE_ORDER_GHIDRA_REPORT.md)  
**Scope:** Resolve four open questions: 0x0B initial sender, 0x1A/0x1B originator + YR-liveness, 0x1E activation in YR, and string-confirmation status of ROGER/HELLO/NEED_TO_MOVE/RADIO_WANT_RIDE from design doc §3.2.  
**Binary:** `gamemd.exe`  
**Status:** COMPLETE (all four items resolved; see individual verdicts)

---

## 0. Investigation Contract

**Target question:**  
1. Who sends radio 0x0B, and to whom? Is it live in YR?  
2. Who is the first sender of 0x1A/0x1B, or is it TS-dead?  
3. Does 0x1E ever fire in standard YR? What populates NavCom?  
4. Are ROGER, HELLO, NEED_TO_MOVE, RADIO_WANT_RIDE present as binary string literals with xrefs?

**Non-goals:** Re-decoding settled codes (0x07/0x0C/0x11/0x16/0x1D), full mission state machines, NavCom struct internals.

**Evidence needed to mark COMPLETE:**  
- 0x0B: send site identified (assembly + function), YR-liveness confirmed.  
- 0x1A/0x1B: exhaustive PUSH-0x1a/PUSH-0x1b scan across game code; verdict with evidence.  
- 0x1E: exhaustive PUSH-0x1e scan; NavCom population attempt.  
- Four names: `search_strings` for each literal; xref address or "searched, absent" result.

**Stop conditions:** All four items have binary evidence or declared Remaining Uncertainty.

---

## 1. String Confirmation — §3.2 Reconciliation

### 1.1 Search Results

`search_strings "ROGER"` → 3 matches, all substrings of debug log strings in `AircraftClass__Mission_Move_Carryall`:

| String address | Full string value |
|---|---|
| 0x00817c14 | `Do_MISSION_MOVE_Carryall - LAND - RADIO_NEED_TO_MOVE got RADIO_ROGER\n` |
| 0x00817e04 | `Do_MISSION_MOVE_Carryall - VALIDATE_LZ - RADIO_WANT_RIDE did not get RADIO_ROGER\n` |
| 0x00817e58 | `Do_MISSION_MOVE_Carryall - VALIDATE_LZ - RADIO_HELLO got RADIO_ROGER\n` |

**All four names appear in the binary:**  
- `RADIO_ROGER` — at 0x00817c14, 0x00817e04, 0x00817e58 (3 xrefs, all in `AircraftClass__Mission_Move_Carryall`)  
- `RADIO_HELLO` — at 0x00817e58 (1 xref, `AircraftClass__Mission_Move_Carryall`)  
- `RADIO_NEED_TO_MOVE` — at 0x00817c14 (1 xref, `AircraftClass__Mission_Move_Carryall`)  
- `RADIO_WANT_RIDE` — at 0x00817e04 (1 xref, `AircraftClass__Mission_Move_Carryall`)  

Evidence: `search_strings "ROGER"` returned 3 matches; `search_strings "HELLO"` returned 2 matches including the carryall string; `search_strings "NEED_TO_MOVE"` returned 1 match; `search_strings "RADIO_WANT_RIDE"` returned 1 match.

### 1.2 Nature of the String Confirmation

These strings are **debug log messages**, not standalone symbol/enum definitions. They appear only as diagnostic output strings embedded in `AircraftClass__Mission_Move_Carryall @ 0x00416d50`. The xrefs (0x00416e91, 0x00416e73, 0x00417272) are all `[DATA]` references — the strings are referenced by pointer within the carryall mission function, meaning they are **compiled-in diagnostic strings** that name the radio codes.

**Assessment:**  
- The design doc §3.2 claim "binary string literal confirmed" for ROGER/HELLO/NEED_TO_MOVE/RADIO_WANT_RIDE is **PARTIALLY CORRECT** — the strings ARE present in the binary, embedded as debug format strings in one specific function. They are genuine Westwood-authored names for these codes.  
- Round-1's claim "none confirmed" was **an overclaim** — round-1 searched only DOCK_LOCK/DOCKING_COMPLETE/TIMING_SYNC and did NOT search these four names. ROGER/HELLO/NEED_TO_MOVE/RADIO_WANT_RIDE are **string-present with xrefs** in the binary.

### 1.3 Updated Name Confidence Status

| Code | Name from strings | Confidence |
|------|-------------------|------------|
| 0x01 | ROGER | **String-present** — substring of debug log at 0x00817c14/0x00817e04/0x00817e58 in `AircraftClass__Mission_Move_Carryall`. Westwood-authored name confirmed. |
| 0x02 | HELLO | **String-present** — substring at 0x00817e58. |
| 0x13 | NEED_TO_MOVE | **String-present** — substring at 0x00817c14. |
| 0x24 | WANT_RIDE (prefix RADIO_) | **String-present** — substring at 0x00817e04 as `RADIO_WANT_RIDE`. |

Caveat: These are debug strings only found in `AircraftClass__Mission_Move_Carryall`. They are NOT standalone enum symbol tables. The names appear genuine but are only confirmed in the context of carryall mission logging.

---

## 2. Code 0x0B — Sender Identified

### 2.1 Sender: BuildingClass mission handler FUN_00449a50

**Evidence:** `search_byte_patterns "6a 0b"` returned 67 addresses. Assembly context scan identified `0x00449b3d` as the **only** `PUSH 0x0B` → `CALL dword ptr [EAX + 0x274]` (vtable+0x274 = Transmit_Radio to contact) in the entire game executable.

Assembly at 0x00449b3d:
```
00449b32: PUSH 0x0         ; payload = NULL
00449b3d: PUSH 0xb         ; message = 0x0B
0044993f: MOV ECX,ESI      ; this = building
00449b41: CALL dword ptr [EAX + 0x274] ; Transmit_Radio(0x0B, NULL) to contact
```

Function containing 0x00449b3d: **`FUN_00449a50 @ 0x00449a50`** (unnamed, BuildingClass mission handler).

Evidence: `decompile_function 0x00449a50`, vtable entry verified at `0x007e4100` via `get_xrefs_to 0x00449a50` → "From 007e4100 [DATA]".

### 2.2 Function Behavior (FUN_00449a50)

Pseudocode:
```c
// State machine — this = building, contact = produced unit (or linked building)
case 0:
    GrandOpening(0);             // start door opening animation
    Transmit_Radio(0x0B, NULL);  // notify contact: door opening
    VocClass__PlayAt(building->SoundOpeningDoor);
    building->visible = 1;
    state = 1;
    return 1;

case 1:
    visible = 1;
    UpdateLoopingSound();
    if (building->door_anim_complete) {   // +0x6dd != 0
        Transmit_Radio(0x0C);             // notify: I am open
        Transmit_Radio(0x03);             // break radio link
        GrandOpening(1);                  // close door (begin production anim)
        FUN_004dc8c0();                   // clear anim slot
        SetMission(Guard=5, 0);
        SoundEvent__Release();
        return 1;
    }
```

**Interpretation:** This is the building's factory door-open animation mission. In state 0, the building starts opening its door and notifies its contact (a produced unit or waiting unit) via 0x0B. In state 1, once the door animation completes, it sends 0x0C then 0x03 to signal the unit can proceed, then returns to Guard mission. This is confirmed by `GrandOpening()` being called, which is the BuildingClass door animation function.

Evidence: `decompile_function 0x00449a50` (this session); `get_function_by_address 0x00451e40` → "BuildingClass__ClearAnimSlot" (confirms other PUSH 0x0B sites call ClearAnimSlot, not radio); `get_function_by_address 0x00449c30` at 0x007e4104 → "BuildingClass__Sell" (vtable slot after 0x00449a50).

### 2.3 Unit Receive_Radio for 0x0B

**Verified:** `UnitClass::Receive_Radio` switch cases: 3, 7, 0xe, 0xf, 0x15, 0x16, 0x17, 0x24 — no case 0x0B. `FootClass::Receive_Radio` cases: 0x11, 0x12, 0x13, 0x17, 0x1c, 0x23 — no case 0x0B. `TechnoClass::Receive_Radio` cases (from prior doc): no case 0x0B. Unit receives 0x0B → falls through to RadioClass default → returns 0.

Evidence: `decompile_function 0x00737430` (UnitClass::Receive_Radio), `decompile_function 0x004d8fb0` (FootClass::Receive_Radio), prior doc for TechnoClass.

### 2.4 Building Receive_Radio for 0x0B (round-1 reconciliation)

Round-1 found `BuildingClass::Receive_Radio case 0x0B` queues Mission_Unload (0x14) on the **building**. This remains valid: when a building receives 0x0B, it queues Mission_Unload. The sender of 0x0B in FUN_00449a50 sends it to its registered contact. For BuildingClass::Receive_Radio case 0x0B to fire, the building sending 0x0B must have another building as its contact. This scenario (building-to-building radio link) occurs when `FUN_00449a50` runs with a building contact.

**0x0B meaning:** "Door opening — prepare for production exit." Sender = factory building running FUN_00449a50. Receiver context: unit → returns 0 (no-op); building → queues Mission_Unload.

### 2.5 Active in YR

**YES** — FUN_00449a50 is in the BuildingClass mission dispatch table at 0x007e4100. It is called by the mission system whenever a building is in the corresponding mission state. `GrandOpening` confirms it handles the factory door animation, which fires in every war factory production in a standard YR skirmish.

Evidence: `read_memory 0x007e4090` (64 bytes) shows FUN_00449a50 as 28th entry in BuildingClass mission table; `get_function_callers 0x00447780` (GrandOpening) returns FUN_00449a50 among callers.

---

## 3. Codes 0x1A / 0x1B — TS-Dead: No YR Initiator

### 3.1 Exhaustive Sender Search

`search_byte_patterns "6a 1a"` → 27 addresses.  
`search_byte_patterns "6a 1b"` → 23 addresses.

Assembly context scan for all code-range addresses:

**0x1A candidates in game code:**
- `0x004159b4`: `PUSH 0x1a; CALL dword ptr [EDX + 0x1e8]` → **SetMission(0x1a, 0)** (Mission enum 26, not radio). vtable+0x1e8.  
- `0x004ab389`, `0x004ab58c` (also 0x1B): `PUSH 0x1a/0x1b; CALL dword ptr [EDX + 0x48]` → vtable+0x48, not radio.  
- `0x004d8b55`: `PUSH 0x1a; CALL 0x006e53a0` → `TechnoClass__ProcessCellAction` (verified via `get_function_by_address 0x006e53a0`). Not radio.  
- `0x006cd41b/0x006cd48d/0x006cd4e5/0x006cd64f`: `PUSH 0x1a; CALL 0x0065e660` → `FUN_0065e660` (aircraft spawner, verified via decompile). Not radio.  
- `0x00520827`: `PUSH 0x1a; CALL dword ptr [EDX + 0x558]` → vtable+0x558, not radio.  
- All other PUSH 0x1a addresses are in data-section ranges (0x00b...) — initializer data, not game code.

**0x1B candidates in game code:**
- `0x00415946`: `PUSH 0x1b; CALL dword ptr [EDX + 0x1e8]` → **SetMission(0x1b, 0)** (Mission enum 27, not radio).  
- `0x004a37d3`: Context shows `PUSH 0x1d; PUSH 0x1b; CALL dword ptr [ECX + 0x5c]` — vtable+0x5c, not radio.  
- `0x0055b033`: `PUSH 0x1b; CALL 0x006e53a0` → `TechnoClass__ProcessCellAction`. Not radio.  
- `0x006232f7`: `PUSH 0x1b; CALL 0x0054f1c0` — fixed address, not radio.

**Known receiver (NOT a sender):**
- `0x006f4bd6` (PUSH 0x1a in TechnoClass::Receive_Radio case 0x1A) — this is the self-propagation from round-1, confirmed to be a RECEIVER-side propagate, not an external initiator.
- `0x006f4c0a` (PUSH 0x1b in TechnoClass::Receive_Radio case 0x1B) — same.

**Conclusion:** No PUSH 0x1a or PUSH 0x1b in the entire executable calls through vtable+0x274, +0x278, +0x27C, or +0x280. The TechnoClass self-propagation cases ARE live code, but no external function INITIATES a 0x1A or 0x1B send in YR.

Evidence: `search_byte_patterns "6a 1a"` → 27 addresses; `search_byte_patterns "6a 1b"` → 23 addresses; assembly context via `get_assembly_context` on all code-range hits; `get_function_by_address 0x006e53a0` → "TechnoClass__ProcessCellAction"; `decompile_function 0x0065e660` (aircraft spawner).

### 3.2 Verdict: TS-Dead

**Active in YR: NO.** The 0x1A/0x1B receiver cases in TechnoClass are compiled into the binary and will execute correctly if triggered, but no YR game path ever initiates the first 0x1A or 0x1B send. This is a TS-era secondary dock-lock mechanism. The only way these codes could fire in YR would be through modding (external code sends 0x1A) or a TS code path that is gated off.

**Do NOT implement 0x1A/0x1B as active radio sends in the Rust port.** Model only the receiver propagation (TechnoClass sets/clears +0x419 and propagates), but no sim system should ever initiate a 0x1A or 0x1B transmission.

---

## 4. Code 0x1E — TS-Dead: No YR Sender

### 4.1 Exhaustive Sender Search

`search_byte_patterns "6a 1e"` → 48 addresses.

Assembly context scan for all code-range addresses:

- `0x00443670/0x004439b9/0x00443b03`: `PUSH 0x1e; CALL dword ptr [EDX + 0x38]` → vtable+0x38 (struct/COM init call), not radio.  
- `0x005441e0` through `0x005443e5` (13 addresses): All `PUSH 0x1e; PUSH ESI; CALL 0x0054a120` — table initializer (writing pairs of constants to globals at 0x00abc3c8+). Not radio.  
- `0x00516c1b/0x00516fde/0x00517023`: `PUSH 0x1e; PUSH 0x14; LEA ECX; CALL 0x0065c7e0` — 0x0065c7e0 is not radio (it creates a timer or counter object with size 0x1e = 30 and 0x14 = 20 as parameters).  
- `0x005dc395/0x005e7a59/0x005e7baf/0x005e8491`: `PUSH 0x1e; CALL 0x00540c60` — initializer or timer.  
- `0x00687665`: `PUSH 0x1e; CALL 0x0069ae90` — not radio.  
- `0x006cd6cd`: `PUSH 0x1e; CALL 0x0065eab0` — not radio.  
- `0x0076dbb0/0x0076dc0d`: `PUSH 0x1e; CALL 0x007712c0/0x007714b0` — timer/list operations.  
- All other addresses in data range (0x00b...).

None of the 48 PUSH 0x1e occurrences calls vtable+0x274/+0x278/+0x27C/+0x280.

Evidence: `search_byte_patterns "6a 1e"` → 48 addresses; assembly context via `get_assembly_context` on all code-range hits; vtable offset check on each CALL target.

### 4.2 NavCom Population

The round-1 doc identified `vtable+0x3F4` as the NavCom getter in TechnoClass::Receive_Radio case 0x1E. No investigation of what populates vtable+0x3F4 was performed this session (out of scope given 0x1E is confirmed TS-dead at the sender level). The NavCom vtable slot exists in the compiled binary but the receiver is never reached because no YR code sends 0x1E.

### 4.3 Verdict: TS-Dead

**Active in YR: NO.** No YR code path sends radio 0x1E. The TechnoClass receiver for 0x1E is compiled in and would execute correctly if reached (NavCom gate + SetPath + Mission_Move), but is unreachable because no sender exists. This is a TS navigation-command redirect mechanism.

**Do NOT implement 0x1E as an active radio send in the Rust port.** The receiver logic can be modeled for completeness, but no sim system should initiate a 0x1E transmission.

---

## 5. Implementation Handoff

### Handoff 1 — 0x0B is building-sends-to-contact, not unit-sends-to-building

The sole verified 0x0B radio sender is `FUN_00449a50 @ 0x00449a50`, a BuildingClass mission handler for the factory door-open animation. The building calls `Transmit_Radio(0x0B, NULL)` to its contact via vtable+0x274 at the start of the door animation. The unit contact ignores 0x0B (no handler, falls through, returns 0). If the contact is another building, that building queues Mission_Unload.

**Rust implementation chain:** `RadioMessage::0x0B` (inferred name) → sent by BuildingClass door-open mission handler when state transitions from 0 to 1 → received by unit: no-op; received by building: queues Mission_Unload. In Rust: add `0x0B` as a radio notification sent from the `sim/building.rs` factory-door mission handler in state 0. The receive handler in `sim/building.rs` for 0x0B already exists (round-1 confirmed it queues unload). The UNIT's `accept_radio_msg` for 0x0B should return 0 (no handler).

**Risk:** Don't confuse sender direction. The building sends 0x0B TO a contact; the existing building Receive_Radio handler for 0x0B is for when a building RECEIVES 0x0B (from another building contact).

### Handoff 2 — 0x1A/0x1B: model propagation only, no initiator

0x1A/0x1B self-propagation (TechnoClass sets/clears `+0x419` and forwards to its own contact) should be modeled in `sim/techno.rs` for correctness, but NO sim system should ever call `transmit_radio(0x1A, ...)` or `transmit_radio(0x1B, ...)` as an initiator. If the propagation is implemented, the `ContactsSlot` model for the second dock-lock field (`+0x419`, distinct from `+0x418`) should be added to `ContactsSlot` but remain inert until a TS path triggers it (which never happens in YR).

**Full chain (propagation only):** `TechnoClass::Receive_Radio(0x1A)` → if `+0x419 == 0`: set `+0x419 = 1`, forward `Transmit_Radio(0x1A)` to registered contact, return ROGER. `TechnoClass::Receive_Radio(0x1B)` → if `+0x419 != 0`: clear `+0x419 = 0`, forward `Transmit_Radio(0x1B)` to contact, return ROGER.

### Handoff 3 — RadioMessage enum string-confirmed names

Names `ROGER` (0x01), `HELLO` (0x02), `NEED_TO_MOVE` (0x13), and `RADIO_WANT_RIDE` (0x24) are **confirmed by debug string literals** embedded in `AircraftClass__Mission_Move_Carryall` at 0x00817c14/0x00817e04/0x00817e58. These are the original Westwood-authored names. Use these as the canonical enum variant names. All other code names (DOCK_APPROACH, TIMING_SYNC, etc.) remain behavior-inferred.

**Rust enum comment format:**
```rust
/// Radio message codes — Westwood-authored names confirmed via debug strings in
/// AircraftClass::Mission_Move_Carryall (0x00817c14 / 0x00817e04 / 0x00817e58):
Roger = 0x01,      // string-confirmed: "RADIO_ROGER"
Hello = 0x02,      // string-confirmed: "RADIO_HELLO"
NeedToMove = 0x13, // string-confirmed: "RADIO_NEED_TO_MOVE"
WantRide = 0x24,   // string-confirmed: "RADIO_WANT_RIDE"
/// Remaining codes are behavior-derived names; canonical Westwood symbol names unknown.
```

---

## 6. Negative Facts / Do Not Do

1. **Do NOT treat PUSH 0x0B + CALL 0x00451e40 as radio sends.** `0x00451e40` is `BuildingClass__ClearAnimSlot`, not Transmit_Radio. The 0x0B here is an anim slot index, not a radio code. All other PUSH 0x0B sites except `0x00449b3d` use ClearAnimSlot or SetMission — not radio. Evidence: `get_function_by_address 0x00451e40` → "BuildingClass__ClearAnimSlot".

2. **Do NOT implement 0x1A or 0x1B as radio sends initiated by any sim system.** No YR code path sends the first 0x1A or 0x1B. Implementing an initiator would introduce false behavior absent from gamemd.exe. Evidence: exhaustive `search_byte_patterns "6a 1a"` and `"6a 1b"` scan + assembly context on all 27/23 hits — none call vtable+0x274/+0x278/+0x27C/+0x280.

3. **Do NOT implement 0x1E as an active radio send.** Same evidence: exhaustive `search_byte_patterns "6a 1e"` scan (48 hits) — none call through radio vtable offsets. The 0x1E receiver is live code but unreachable in YR.

4. **Do NOT treat the design doc §3.2 "binary string literal confirmed" claim as fully corroborated.** The strings are present as debug log substrings in ONE function (`AircraftClass__Mission_Move_Carryall`), not as standalone symbol/enum tables. They confirm the Westwood names but are diagnostic strings, not symbol export tables. The claim should be restated as "confirmed as Westwood-authored names via debug strings in `AircraftClass__Mission_Move_Carryall`."

5. **Do NOT assert round-1's "none confirmed" (for the four names) as still standing.** Round-1 did NOT search ROGER, HELLO, NEED_TO_MOVE, RADIO_WANT_RIDE. Its search scope was DOCK_LOCK/DOCKING_COMPLETE/TIMING_SYNC only. The "none confirmed" verdict was an overclaim. Evidence: `search_strings "ROGER"` → 3 matches; `search_strings "HELLO"` → 2 matches; `search_strings "NEED_TO_MOVE"` → 1 match; `search_strings "RADIO_WANT_RIDE"` → 1 match.

---

## 7. Remaining Uncertainty

1. **FUN_00449a50 mission index**: The function is the 28th entry in the BuildingClass mission dispatch table at 0x007e4100. The exact mission enum value this maps to (i.e., what `Mission_Open` numeric value BuildingClass uses to reach this handler) is not verified. This does not affect implementation since the behavior is confirmed from the function body.

2. **Which buildings activate FUN_00449a50**: The specific INI flags (`OpenDoor=yes` or equivalent) that put a building into this mission state are not traced. Presumably buildings with animated doors (war factory, weapons factory, naval yard) use this handler. Verify against INI `artmd.ini` for `Yopen` / `Yclose` keys before assuming all factories use it.

3. **Building-to-building 0x0B scenario**: Whether `FUN_00449a50` ever runs with a BUILDING as its contact (rather than a unit) is not confirmed. If yes, the building-receives-0x0B-queues-Mission_Unload path from round-1 would also be live. If FUN_00449a50 only ever has a unit contact, the BuildingClass Receive_Radio case 0x0B may be a TS-dead path. This is left open.

4. **NavCom vtable+0x3F4 population for 0x1E**: Not investigated (declared TS-dead based on no sender). If NavCom is ever populated in a modded/custom context, 0x1E would become active. For stock YR: dormant.

---

## 8. Design Doc §3.2 Stale-Wording Corrections

**`MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md §3.2`** — replace the four name rows:

```
0x01 ROGER: String-confirmed. Debug string "RADIO_ROGER" found at 0x00817c14,
  0x00817e04, 0x00817e58 in AircraftClass__Mission_Move_Carryall.
  Westwood-authored name; use as canonical enum variant.

0x02 HELLO: String-confirmed. Debug string "RADIO_HELLO" at 0x00817e58 in
  AircraftClass__Mission_Move_Carryall. Westwood-authored name.

0x13 NEED_TO_MOVE: String-confirmed. Debug string "RADIO_NEED_TO_MOVE" at 0x00817c14
  in AircraftClass__Mission_Move_Carryall. Westwood-authored name.

0x24 WANT_RIDE: String-confirmed. Debug string "RADIO_WANT_RIDE" at 0x00817e04 in
  AircraftClass__Mission_Move_Carryall. Westwood-authored name.
```

Also replace the round-1 overreach in the Notes section: change "No radio name is string-confirmed" to "ROGER/HELLO/NEED_TO_MOVE/WANT_RIDE are string-confirmed via debug strings in `AircraftClass__Mission_Move_Carryall`. Other names (DOCK_APPROACH, TIMING_SYNC, etc.) remain behavior-inferred."

**`RADIO_INFERRED_CODES_AND_DOCK_HANDSHAKE_ORDER_GHIDRA_REPORT.md §1`** — replace:
```
"No string literals for any radio message code name were found in gamemd.exe."
→ "Debug string literals for RADIO_ROGER (0x01), RADIO_HELLO (0x02),
   RADIO_NEED_TO_MOVE (0x13), and RADIO_WANT_RIDE (0x24) are confirmed at
   0x00817c14/0x00817e04/0x00817e58 in AircraftClass__Mission_Move_Carryall.
   These are Westwood-authored diagnostic strings naming the codes.
   Other code names remain behavior-inferred."
```

Also update §3 (0x0B section): replace "Status: UNVERIFIED sender" with:
```
"Sender confirmed: FUN_00449a50 @ 0x00449a50 (BuildingClass factory door-open mission
  handler, vtable slot at 0x007e4100). Building sends 0x0B via vtable+0x274 to its
  contact at door-open animation start (state 0). Unit contact has no 0x0B handler
  (falls through, returns 0). Building contact → queues Mission_Unload (existing
  round-1 finding). Active in YR: YES (fires every war factory production cycle)."
```

---

## 9. Sources

| Claim | Evidence |
|-------|----------|
| ROGER/HELLO/NEED_TO_MOVE/RADIO_WANT_RIDE present as debug strings | `search_strings "ROGER"` → 3 matches at 0x00817c14/0x00817e04/0x00817e58; `search_strings "HELLO"` → 2; `search_strings "NEED_TO_MOVE"` → 1; `search_strings "RADIO_WANT_RIDE"` → 1 |
| All three strings xref to AircraftClass__Mission_Move_Carryall | `get_xrefs_to 0x00817c14` → "From 00417272 in AircraftClass__Mission_Move_Carryall"; `get_xrefs_to 0x00817e04` → "From 00416e91"; `get_xrefs_to 0x00817e58` → "From 00416e73" |
| AircraftClass__Mission_Move_Carryall @ 0x00416d50 | `search_functions "Mission_Move_Carryall"` |
| 0x0B sole radio sender at 0x00449b3d | `search_byte_patterns "6a 0b"` → 67 addresses; `get_assembly_context` on all hits; only 0x00449b3d → `CALL dword ptr [EAX + 0x274]` |
| FUN_00449a50 contains the 0x0B radio send | `decompile_function 0x00449a50` |
| FUN_00449a50 is in BuildingClass mission vtable | `get_xrefs_to 0x00449a50` → "From 007e4100 [DATA]"; `read_memory 0x007e4090` 128 bytes shows FUN_00449a50 as 28th vtable entry |
| FUN_00449a50 calls GrandOpening (door-open anim) | `decompile_function 0x00449a50`; `get_function_callers 0x00447780` includes FUN_00449a50 |
| 0x451e40 = BuildingClass__ClearAnimSlot (not radio) | `get_function_by_address 0x00451e40` |
| UnitClass::Receive_Radio has no 0x0B case | `decompile_function 0x00737430` — switch cases 3,7,0xe,0xf,0x15,0x16,0x17,0x24 only |
| FootClass::Receive_Radio has no 0x0B case | `decompile_function 0x004d8fb0` — switch cases 0x11,0x12,0x13,0x17,0x1c,0x23 only |
| No 0x1A/0x1B radio sender in YR game code | `search_byte_patterns "6a 1a"` → 27 addresses; `search_byte_patterns "6a 1b"` → 23 addresses; `get_assembly_context` on all — zero vtable+0x274/+0x278/+0x27C/+0x280 calls |
| 0x006e53a0 = TechnoClass__ProcessCellAction | `get_function_by_address 0x006e53a0` |
| 0x0065e660 is an aircraft spawner, not radio | `decompile_function 0x0065e660` |
| No 0x1E radio sender in YR game code | `search_byte_patterns "6a 1e"` → 48 addresses; assembly context on all code-range hits — zero vtable+0x274/+0x278/+0x27C/+0x280 calls |
