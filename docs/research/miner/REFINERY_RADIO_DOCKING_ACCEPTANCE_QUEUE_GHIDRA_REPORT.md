# Refinery RADIO_DOCKING(HELLO=2) Acceptance and Queue Admission — Ghidra Report

**Date:** 2026-05-19
**Binary:** gamemd.exe
**Confidence:** HIGH on all core findings (verified by live decompilation)
**Active in YR:** YES — core harvester/refinery docking handshake

---

## 0. Scope Correction: "RADIO_DOCKING=2" Is HELLO(0x02)

`CHRONO_MINER_SYSTEM_OVERVIEW.md §4` writes `RadioClass::Transmit_Radio(RADIO_DOCKING=2, dock)`.
This is **HELLO (message code 0x02)**, not a specialized "DOCKING" message. Verified by:
- `UnitClass::Mission_Harvest` state 2 decompile (address `0x73E5E0`, label `LAB_0073ee51`):
  `(**(code **)(*param_1 + 0x278))(2, piVar3)` — the miner calls `Transmit_Radio(2, refinery)`.
- `RadioClass::Receive_Radio` (0x0065A820): the only case-2 handler is `HELLO`.
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md §5` table: code 0x02 = `HELLO` (establish radio link).

There is no `RADIO_DOCKING` enum value distinct from `HELLO`. The overview doc's label is
a semantic alias for the same wire value.

---

## 1. Address of BuildingClass::Receive_Radio

| Symbol | Address | Vtable slot | Vtable offset |
|--------|---------|------------|---------------|
| `BuildingClass::Receive_Radio` | `0x0043C2D0` | slot 101 | **+0x194** |
| `BuildingClass` vtable base | `0x007E3EBC` | — | — |

**The assignment brief's "likely vtable+0x274" is wrong.** Vtable+0x274 is
`RadioClass::Transmit_Radio_ToFirst`. The vtable-DATA xref to `0x0043C2D0` is at `0x007E4050`.
`0x007E4050 − 0x007E3EBC = 0x194`. Verified by reading 4 bytes at `0x007E4050`:
`D0 C2 43 00` (little-endian `0x0043C2D0`). Confirmed.

---

## 2. HELLO(2) Path in BuildingClass::Receive_Radio

`BuildingClass::Receive_Radio` (full decompile at `0x0043C2D0`) has **no case 2** in its
switch. Message 2 falls through the entire switch body to the tail:

```c
iVar10 = TechnoClass__Receive_Radio(param_2, param_3, param_4);
return iVar10;
```

`TechnoClass::Receive_Radio` (0x006F4AB0) also has **no case 2** — it falls through to:

```c
uVar6 = RadioClass__Receive_Radio(param_2, param_3, param_4);
return uVar6;
```

`RadioClass::Receive_Radio` (0x0065A820) handles case 2 as `HELLO`:

```
HELLO(2) accept conditions (ALL must pass):
  1. ObjectClass_field_0x6C != 0              — building is alive/on-map
  2. HouseClass::Is_Ally_ByObject(this, sender) — sender is allied to building
  3. If (this->AbstractFlags & 1): sender must also pass ally check (bidirectional)
  4. Sender not already in Contacts[]         — idempotent (returns ROGER if already present)
  5. At least one NULL slot in Contacts[]     — capacity check

Accept: write sender pointer into first NULL Contacts[] slot. Return 1 (ROGER).
Reject: any condition fails → return 10 (NEGATORY).
```

**There is no BuildingClass-level accept/reject logic for HELLO(2).** The refinery does
not inspect its queue depth, mission state, building type, power state, or unit type before
accepting a HELLO. All of those checks live in `CAN_DOCK (0x0E)`, not here.

---

## 3. Accept/Reject Conditions for HELLO(2)

(a) **Reject — enemy sender:** `Is_Ally_ByObject` returns false → NEGATORY(10).

(b) **Reject — no free Contacts slot:** `Contacts.Capacity` is set from
   `BuildingTypeClass+0x1780` (`NumberOfDocks`, minimum 1) by `RadioClass::Set_Contact_Count`
   called from `BuildingClass::Constructor` (0x0043B740, confirmed). A standard refinery has
   `NumberOfDocks=1` → capacity 1. If one harvester is already in Contacts[], a second HELLO
   is rejected with NEGATORY(10). This is the "queue full" rejection at the radio level.
   It does **not** evict the refinery's current contact; slot-0 eviction is a
   sender-side outgoing `Transmit_Radio_Impl(HELLO)` behavior on the sender's
   own `Contacts[]`.

(c) **Reject — building dead/off-map:** `ObjectClass_field_0x6C == 0` → NEGATORY silently
   (the outer `else if` is not entered; falls through to `ObjectClass::Receive_Radio`).

(d) **Idempotent re-HELLO:** If sender already in Contacts[], returns ROGER(1) immediately
   without re-inserting. This is important — Mission_Enter can safely re-send HELLO
   without corrupting the roster.

**No power check, no construction-state check, no type check at HELLO time.** Those are
deferred to CAN_DOCK(0x0E) (BuildingClass::Receive_Radio case 0xE), which is a separate
exchange that happens later in the approach sequence.

---

## 4. Fields Written on BuildingClass When HELLO(2) Accepted

Only one write: `Contacts[i] = sender` in the RadioClass-level `Contacts[]` array.

| Field | Byte offset | Write | Cleared by |
|-------|-------------|-------|------------|
| `Contacts[i]` (RadioClass) | `+0xE4` (ptr to array) + `i×4` | sender pointer | BREAK(0x03) → nulls that slot |

**No dock-queue entry is created. No +0x2E4 field is written. No mission-state field is
updated.** HELLO only establishes the radio link (Contacts roster). The 2026-05-21
re-swarm further corrected the old "later +0x2E4 dock-pad link" interpretation:
standard stock `CMIN/HARV -> GAREFN/NAREFN` DockUnload does not establish a
reciprocal `unit/building +0x2E4` link. See
`BUILDINGCLASS_FIELD_0X2E4_REFINERY_DOCK_GHIDRA_REPORT.md`.

**Building does NOT send a reply ROGER/NEGATIVE radio message back.** The return code 1
(ROGER) flows back as the **return value of Transmit_Radio**, synchronously on the caller's
stack. No reply radio message is transmitted.

---

## 5. Building Field +0x2E4 — Semantic

`BuildingClass::UndockUnit` (0x4593A0) decompile:

```c
void BuildingClass__UndockUnit(int *param_1) {
    int *piVar1 = param_1[0xb9];   // param_1 is int* → byte offset = 0xb9*4 = 0x2E4
    if (piVar1 != NULL) {
        // ILocomotion::Stop, ILocomotion::Head_To (exit facing 0x47 SE)
        piVar1[0xb9] = 0;          // unit[0x2E4] = 0 (unit's back-link cleared)
        param_1[0xb9] = 0;         // building[0x2E4] = 0 (building's forward-link cleared)
        Transmit_Radio_ToFirst(3); // BREAK
    }
}
```

**Corrected 2026-05-21:** `BuildingClass+0x2E4` is a conditional reciprocal link
field, not the normal stock refinery DockUnload slot. It is verified for
bunker/cleanup/release contexts and is read/cleared by helpers such as
`UndockUnit`/`ReleaseDockedHarvester` when the reciprocal link is already nonzero.
It is NOT a queue entry and is NOT set at HELLO(2) acceptance time. For standard
stock `CMIN/HARV -> GAREFN/NAREFN`, later reports found the unload FSM normally
runs with `unit+0x2E4 == 0`.

From `BuildingClass::ReceiveDamage` (referenced in MINER_DOCK_GAPS_RESEARCH.md §"Case A"):

```c
if (field_0x2E4 != 0) {
    BuildingClass::UndockUnit();  // eject unit on pad before building dies
}
```

This confirms only that `+0x2E4` can be populated before conditional
cleanup/release paths. It does **not** prove the stock refinery path writes the
field when a harvester reaches the pad. The newer report
`BUILDINGCLASS_FIELD_0X2E4_REFINERY_DOCK_GHIDRA_REPORT.md` found that stock
refinery case `0x0E`, case `0x15`, and zero-link `Mission_Deploy_Building` do
not create a reciprocal `+0x2E4` link.

---

## 6. Unit+0x2E4 (unit[0xB9]) vs Building+0x2E4 (building[0xB9])

Both `UnitClass` and `BuildingClass` can use offset **+0x2E4** for a conditional
reciprocal link:

| Object | Field | Meaning |
|--------|-------|---------|
| `BuildingClass+0x2E4` | `building[0xb9]` | Conditional pointer to linked unit; not normal stock refinery DockUnload |
| `UnitClass+0x2E4` | `unit[0xb9]` | Conditional pointer back to linked building; normally zero in stock refinery unload |

This is a **symmetric cross-link** when a context actually creates it. Both ends
are cleared together in `UndockUnit`/conditional release cleanup. Do not infer
that stock refinery DockUnload creates this link.

Confidence for "coincident layout vs mixin": **MEDIUM** — the offsets are the same by
inspection of `UndockUnit` decompilation. Whether this is intentional or coincidental cannot
be determined without BuildingClass and UnitClass full layout analysis.

---

## 7. How HELLO(2) Relates to the Queue and FUN_006AF6C0

### Summary

HELLO(2) does **not** insert into the dock queue. It only adds the sender to `Contacts[]`.

The refinery dock queue is managed by what the docs call the "dock queue processor". The
function at `0x006AF6C0` labeled `FUN_006AF6C0` in the docs decompiles as
`SlaveManagerClass::AI_Update` (per this session's decompilation). The actual dock queue
processor for regular refineries may be a different function. This is a scope boundary
per the assignment brief; the queue state machine is owned by slot-1.

What CAN be confirmed from the HELLO(2) path:

- **HELLO accepted → Contacts[i] = sender.** The miner is now a radio partner.
- **Next tick or same tick:** the miner enters `Mission_Enter` and sends
  `CAN_DOCK(0x0E)`.
- **CAN_DOCK accepted:** building sends `0x13 -> 0x12`; only after `0x12`
  returns already-there does the building send `0x18 -> 0x16`.
- **Pad arrival:** stock inbound refinery docking does **not** send
  `0x0C DOCK_ARRIVED`. `UnitClass::PerCellProcess` sends `0x15`; building case
  `0x15` queues sender mission `0x10` to start the unload FSM.
- **No stock +0x2E4 queue link:** the older dock-queue-state-machine paragraph
  above is superseded. Stock refinery DockUnload does not set
  `building[0x2E4] = unit` or call `UndockUnit` as the normal completion path.

HELLO(2) is the **handshake admission step**, not the queue admission step. Queue
admission happens inside CAN_DOCK(0xE) when `Contacts.Contains(sender)` is true and
`MOVE_TO_CELL(0x12)` is successfully sent.

---

## 8. Chrono-Miner-Specific Handling

**There is no Teleporter=yes branch in any of the three Receive_Radio levels for HELLO(2).**

The refinery's radio stack (`BuildingClass::Receive_Radio` → `TechnoClass::Receive_Radio`
→ `RadioClass::Receive_Radio`) is completely blind to the sender's locomotor type.
`Teleporter=yes` is a flag on the sender's `TechnoTypeClass` (`+0xCD4`), never read during
radio processing. The chrono miner and a regular harvester are treated identically by all
three HELLO handlers.

Any chrono-specific behavior (piggyback swap, warp-in vs drive-in) occurs on the **miner
side** (locomotor layer, `FootClass::AI`, `Set_Destination` path), not on the building side.

---

## 9. Verified Facts Summary

| # | Fact | Evidence | Confidence |
|---|------|----------|-----------|
| 1 | `BuildingClass::Receive_Radio` is at `0x0043C2D0`, vtable slot +0x194, NOT +0x274 | vtable xref `0x007E4050` → `D0 C2 43 00`; `0x007E4050 - 0x007E3EBC = 0x194` | HIGH |
| 2 | HELLO(2) is handled entirely in `RadioClass::Receive_Radio` (0x0065A820); Building and Techno layers pass it through | Full decompile of all three layers; no case 2 in Building or Techno switches | HIGH |
| 3 | HELLO(2) accept conditions: ally check + free Contacts slot (capacity = NumberOfDocks, default 1 per refinery) | `RadioClass::Receive_Radio` decompile + `BuildingClass::Constructor` calling `Set_Contact_Count(Type+0x1780)` | HIGH |
| 4 | `BuildingClass+0x2E4` (`building[0xb9]`) is a **conditional reciprocal link field**, not the normal stock refinery DockUnload slot; stock refinery `0x0E`/`0x15`/zero-link unload do not write it | `BUILDINGCLASS_FIELD_0X2E4_REFINERY_DOCK_GHIDRA_REPORT.md`; `UndockUnit` only proves cleanup of an already-populated link | HIGH |
| 5 | Chrono miner (Teleporter=yes) receives identical treatment to regular harvester in all HELLO(2) handling; no Teleporter branch exists in the refinery radio stack | Full decompile of BuildingClass, TechnoClass, RadioClass Receive_Radio — no TypeClass flag reads in HELLO path | HIGH |

---

## 10. Addresses Reference

| Symbol | Address |
|--------|---------|
| `BuildingClass::Receive_Radio` | `0x0043C2D0` |
| `BuildingClass` vtable base | `0x007E3EBC` |
| `TechnoClass::Receive_Radio` | `0x006F4AB0` |
| `RadioClass::Receive_Radio` | `0x0065A820` |
| `RadioClass::Set_Contact_Count` | `0x0065AE60` |
| `BuildingClass::Constructor` | `0x0043B740` |
| `BuildingClass::UndockUnit` | `0x4593A0` |
| `BuildingClass::CanDock` | `0x457CE0` |
| `UnitClass::Mission_Harvest` (state 2 HELLO send) | `0x73E5E0`, label `LAB_0073ee51` |

---

## Status: COMPLETE

All 8 scope questions answered with binary evidence. No invented offsets or addresses.
