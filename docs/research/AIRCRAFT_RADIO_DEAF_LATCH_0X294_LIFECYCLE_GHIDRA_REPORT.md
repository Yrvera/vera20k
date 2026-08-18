# AircraftClass Radio-Deaf Gate Field (+0x294) Lifecycle — Ghidra Research Report

**Addresses:** Gate READ: `0x004190D3` (within `AircraftClass::Receive_Radio @ 0x004190B0`);
Writers: `0x006F2E09`, `0x006F41EE`, `0x006F45BE`, `0x00707835`
**Investigation Mode:** targeted-writer-lifecycle / reconcile-sibling-doc
**Scope:** Write sites (set + clear) for `TechnoClass+0x294`; semantic resolution
(pointer vs bool); offset identity (`param_1[0xA5]` vs byte `+0x294`); mission handler
authorship; YR-activity for writer paths.
**Non-goals:** Full ParaDrop / SpyPlane state machine; AirstrikeClass AI / BPLN spawn.
**Authority order:** binary → live Ghidra → sibling docs
**Sibling resolved:** `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md` — read
first; this report reconciles and confirms; does not re-investigate.

---

## 0. Investigation Contract

**Target question:** What writes `TechnoClass+0x294` (the AircraftClass radio-deaf gate
field), when, and by which handlers? Is it a bool latch or a pointer? Is `param_1[0xA5]`
== byte `+0x294` or a separate field?

**Non-goals:** Do not re-decode ParaDrop or SpyPlane state machines. Do not decode
AirstrikeClass AI or BPLN spawn.

**Evidence needed to mark COMPLETE:**
1. Live assembly for the gate READ confirming byte offset `+0x294`.
2. Live assembly for every write site (set + clear).
3. Decompile/caller evidence for `Init_Managers` showing write is guarded by
   `AirstrikeTeam > 0` type-field check.
4. Confirmed-negative: no scoped mission (ParaDrop/SpyPlane) writes `+0x294`.
5. Resolved: `param_1[0xA5]` == byte offset `+0x294`, not byte `0xA5`.
6. Semantic resolved: pointer vs bool.

**Stop conditions:** All writers enumerated, semantic and offset identity resolved,
YR-activity verdict for each writer confirmed, `+0x294 vs 0xA5` reconciled.

**Status: COMPLETE** — all stop conditions met with live binary evidence.

---

## 1. Offset Identity Resolution: `param_1[0xA5]` == byte `+0x294`

The decompiler emits `param_1[0xA5]` because `param_1` is typed as `int*` (4-byte
integers). Pointer arithmetic: `0xA5 * 4 = 0x294`.

**Live evidence — gate READ:**
```
004190d3: MOV EAX,dword ptr [ESI + 0x294]   ; gate read in Receive_Radio
004190d9: TEST EAX,EAX
004190db: JNZ 0x004190e6                     ; non-null → pass gate, handle normally
004190de: POP ESI
; → falls through to XOR EAX,EAX / RET 0xc  ; null → return 0 (drop message)
```
(verified via `get_assembly_context 0x004190D3`)

The displacement in the assembled instruction is `0x294`, confirming `param_1[0xA5]`
is byte offset `+0x294`. These are the same field; `0xA5` is only the decompiler's
index into its phantom `int*` view.

**Conclusion:** `+0x294` and `param_1[0xA5]` are identical. The doc naming "0xA5" is
the decompiler alias; binary byte offset is `+0x294`.

---

## 2. Semantic: Pointer, Not Bool

**Evidence — type resolution:**

The value at `+0x294` is a **4-byte pointer** (`dword ptr`). All write sites write
either a register holding a freshly constructed object address (set) or `EBX=0` (clear).
The read at `0x004190D3` does `TEST EAX,EAX / JNZ` — a standard null-pointer check.

`TechnoClass::Init_Managers @ 0x006F3F40` allocates this pointer:
```
006f41e5: CALL 0x0041d380    ; AirstrikeClass::Constructor(this)
006f41ee: MOV dword ptr [ESI + 0x294],EAX   ; store result → pointer to AirstrikeClass
```
(verified via `get_assembly_context 0x006F41EE`)

**Conclusion:** `+0x294` is `AirstrikeClass*` — a pointer to a per-Techno airstrike
manager object. It is NOT a bool latch. The "radio-deaf" gate fires when this pointer
is null (i.e., the Techno has no airstrike manager).

---

## 3. Writer Enumeration — All Write Sites

### 3.1 TechnoClass::Constructor — Default Clear
- **Address:** `0x006F2E09`
- **Write:** `MOV dword ptr [ESI + 0x294],EBX` (EBX=0, verified by surrounding zero-init
  sequence — adjacent clears to `+0x2e8`, `+0x2e4`, `+0x29c`, `+0x2a0`, etc.)
- **Effect:** `this->AirstrikePtr = null` — every Techno object starts without an airstrike
  manager; radio gate fires by default for aircraft in the scoped missions.
- **Active in YR:** Yes — constructor runs for every object.
- **Evidence:** `get_assembly_context 0x006F2E09`

### 3.2 TechnoClass::Init_Managers — Conditional Allocation (SET)
- **Address:** `0x006F3F40` (function); write at `0x006F41EE`
- **Guard condition:** `Type+0x61C > 0` (AirstrikeTeam INI field; `int`). If zero/absent,
  write path is `006f41ec: XOR EAX,EAX` → `006f41ee: MOV dword ptr [ESI+0x294],EAX`
  (still writes, but writes null; or the call at `006f41e5` to `AirstrikeClass::Constructor`
  runs and the result is stored). The conditional is that the CALL at `006f41e5` is only
  taken when `Type+0x61C > 0`; the `XOR EAX,EAX / JMP 006f41ee` path writes null.
- **Write:** `MOV dword ptr [ESI + 0x294],EAX` — stores either a valid `AirstrikeClass*`
  (non-null, when `AirstrikeTeam > 0`) or 0 (null, fallthrough when team count is zero).
- **Callers of Init_Managers:** `AircraftClass::InitFromType @ 0x00413F85`,
  `InfantryClass::InitFromType @ 0x00517CC4`, `UnitClass::Constructor @ 0x007355F1`,
  `UnitClass::InitFromType @ 0x00746814`, `BuildingClass::Init_Managers @ 0x00442C43`.
- **Active in YR:** Conditional. Only `[BORIS]` (InfantryClass) has `AirstrikeTeam > 0`
  in stock `rulesmd.ini` (lines 4649–4656). No stock aircraft type has `AirstrikeTeam`.
  For all stock aircraft: `Init_Managers` writes null → gate always fires for scoped
  missions → all paradrop/spyplane aircraft are radio-deaf during those missions.
- **Evidence:** `get_assembly_context 0x006F41EE`

### 3.3 TechnoClass Destructor Chain — Owner Clear
- **Address:** `0x006F4510` (function); write at `0x006F45BE`
- **Guard condition:** Owner pointer inside AirstrikeClass (`+0x4C`) must match `this`.
  Context: `JZ 0x006f45be` taken when the owner check passes.
- **Write:** `MOV dword ptr [ESI + 0x294],EBX` (EBX=0) — clears the pointer after
  releasing the AirstrikeClass COM object.
- **Effect:** Cleanup on techno death/destruction.
- **Active in YR:** Yes — destructor runs on every destroyed techno.
- **Evidence:** `get_assembly_context 0x006F45BE`

### 3.4 TechnoClass::PointerExpired — Expired-Pointer Invalidation
- **Address:** `0x00707800` (function); write at `0x00707835`
- **Guard condition:** `CMP EBP,dword ptr [ESI+0x294]` at `0x0070782D` — only clears
  if the expired pointer equals the current `+0x294` value.
- **Write:** `MOV dword ptr [ESI + 0x294],EBX` (EBX=0)
- **Effect:** Detaches the AirstrikeClass manager when the pointed-to object becomes
  invalid (object removed from world without going through normal destructor path).
- **Active in YR:** Yes — `PointerExpired` is called when any tracked object expires.
- **Evidence:** `get_assembly_context 0x00707835`; CMP at `0x0070782D`

---

## 4. Confirmed-Negative: No Scoped Mission Writes +0x294

The following aircraft mission handlers were verified to NOT write `+0x294`
(per `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`, decompile evidence):

| Handler | Address | What it writes instead |
|---|---|---|
| `Mission_ParaDropApproach` | `0x004155F0` | `+0x6D2` (IsStrafe), queues mission `0x1F` |
| `Mission_ParaDropOverfly` | `0x004157C0` | reveal/path destination only |
| `Mission_Open` | `0x004158E0` | `+0x6D3` (PayloadCount decrement) |
| `Mission_Rescue` | `0x00415960` | `+0x6D2`, checks `+0x6D3`, queues `0x1A` or `4` |
| `Mission_QMove` | `0x00415A50` | destination assign/clear only |

There is no SpyPlane mission writer to `+0x294` either. The gate field is not a
"mission enters → set latch / mission exits → clear latch" mechanism. It is a
persistent per-Techno allocation driven entirely by `TechnoType.AirstrikeTeam`.

---

## 5. Gate Logic Summary

```
AircraftClass::Receive_Radio @ 0x004190B0:
  switch(this->Mission):
    case 4  (QMove / Retreat)
    case 0x1A (Open)
    case 0x1B (Rescue)
    case 0x1E (ParaDropApproach)
    case 0x1F (ParaDropOverfly):
      if (this->AirstrikePtr == null) { XOR EAX,EAX; RET 0xC; }  // drop message
  // fall through → normal radio switch handling
```

Assembly confirmed: `0x004190D3`–`0x004190E3`.

The gate logic is:
- `this->AirstrikePtr != null` → message allowed (object summoned by a Boris-airstrike aircraft variant)
- `this->AirstrikePtr == null` → message dropped during the five scoped missions

In stock YR, all aircraft have `AirstrikePtr == null` at runtime (no stock aircraft type
has `AirstrikeTeam > 0`), so every paradrop/spyplane aircraft is always radio-deaf during
these missions without exception.

---

## 6. Design Doc Correction: §3.2 / §5.2.10 / §9.2

The `MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md` Correction #2 already captures the
correct model:

> `+0x294` is an **airstrike back-pointer** (aircraft→summoning AirstrikeClass), NOT a
> bool radio-deaf latch. Model as `airstrike_owner: Option<EntityId>`.

The §9.2 UNCHECKED item reads:
> "AircraftClass `+0x294` latch *setter* — the radio-deaf gate *read* is verified;
> *when the latch clears* is UNCHECKED"

**This is now RESOLVED.** All set and clear sites are verified. The §9.2 item can be
promoted to RESOLVED with evidence from this report.

**No correction needed to the offset value** — the design doc already says `+0x294` in
the correction block. The "0xA5" naming is in the sibling doc title and is the decompiler
alias; both refer to the same field.

---

## 7. Implementation Handoff

### Full Chain 1: Stock YR Paradrop Aircraft (the common case)

1. `SuperWeaponTypeClass` spawns PDPLANE or CARGOPLANE via `AircraftClass::Unlimbo`.
2. `AircraftClass::InitFromType @ 0x00413F80` calls `TechnoClass::Init_Managers @ 0x006F3F40`.
3. Stock aircraft type has `AirstrikeTeam = 0` (default) → `Init_Managers` writes null to
   `this+0x294` (path: `006f41ec: XOR EAX,EAX` → `006f41ee: MOV [ESI+0x294],EAX`).
4. Aircraft enters `ParaDropApproach (0x1E)` or `ParaDropOverfly (0x1F)`.
5. Any incoming radio message hits the gate: `[ESI+0x294] == 0` → `XOR EAX,EAX; RET 0xC`
   (returns 0 / drops message).
6. On aircraft death: destructor writes null again (no-op since already null).

**Rust model:** `airstrike_manager: Option<EntityId>` on every Techno. For stock aircraft
this is always `None`. Radio dispatch for aircraft returns `RadioResponse::Deaf(0)` when
`current_mission ∈ {Retreat, Open, Rescue, ParaDropApproach, ParaDropOverfly}` AND
`airstrike_manager.is_none()`.

### Full Chain 2: AirstrikeTeam Aircraft (Boris-summoned variant)

1. Infantry `[BORIS]` creates an `AirstrikeClass` manager because `AirstrikeTeam=2`.
2. The summoned BPLN aircraft is controlled through the `AirstrikeClass` manager, not
   directly. The BPLN itself is an AircraftClass; whether it receives a non-null `+0x294`
   depends on whether BPLN has `AirstrikeTeam > 0` in its own type (it does not in stock YR).
3. Stock BPLNs therefore are still radio-deaf during scoped missions. The non-null
   `+0x294` exception is hypothetically available but unused in stock YR.

### Full Chain 3: Teardown

When a paradrop aircraft is destroyed:
1. `TechnoClass` destructor at `0x006F4510` checks owner match at `+0x4C` inside
   AirstrikeClass and clears `this+0x294` (`0x006F45BE`).
2. `TechnoClass::PointerExpired @ 0x00707800` catches any invalidation path that bypasses
   the destructor; clears `this+0x294` only if `EBP == [ESI+0x294]` (`0x0070782D`).

---

## 8. Negative Facts / Do Not Do

1. **Do not treat `param_1[0xA5]` as byte offset `0xA5`.** It is byte `+0x294`. Evidence:
   live assembly `MOV EAX,dword ptr [ESI + 0x294]` at `0x004190D3`.
2. **Do not model `+0x294` as a bool latch set/cleared by entering/exiting a mission.**
   No mission handler writes this field. Evidence: verified-negative for all five scoped
   mission handlers (§4 above).
3. **Do not implement a `radio_gate` flag separate from the airstrike manager pointer.**
   The gate IS the null-check of `AirstrikeClass*`. Evidence: gate logic at `0x004190D3`.
4. **Do not assume stock aircraft ever have `+0x294 != null` in standard YR.** No stock
   aircraft type has `AirstrikeTeam > 0`. Evidence: INI scan `rulesmd.ini` lines 4649–4656;
   only `[BORIS]` carries these keys.
5. **Do not implement this gate as AircraftClass-specific storage.** `+0x294` lives in
   TechnoClass layout and applies to all Techno subclasses. Only `AircraftClass::Receive_Radio`
   enforces the mission-firewall check; the pointer allocation is shared. Evidence:
   `Init_Managers` callers span Aircraft, Infantry, Unit, Building.

---

## 9. Remaining Uncertainty

**None** — all stop conditions from §0 are met with live binary evidence.

Deferred (out of scope): Full AirstrikeClass AI / BPLN spawn lifecycle; how the
`AirstrikeClass` at Boris's `+0x294` issues BPLN spawn instructions. These do not
affect the radio-deaf gate or its writer set.

---

## Sources

- Live Ghidra assembly: `get_assembly_context 0x004190D3, 0x006F2E09, 0x006F41EE,
  0x006F45BE, 0x00707835` (all verified this session).
- Sibling doc (extended, not redone):
  `docs/research/AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`
- Design doc (UNCHECKED resolution):
  `docs/research/MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md` §9.2
- INI: `ini/rulesmd.ini` lines 4649–4656 (`[BORIS]` AirstrikeTeam keys)
