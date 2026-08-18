# BuildingClass::Receive_Radio case 0x0E (CAN_DOCK) — Ghidra Report

**Date:** 2026-05-19
**Binary:** gamemd.exe
**Function:** `BuildingClass::Receive_Radio` @ `0x0043C2D0`, vtable slot +0x194
**Scope:** case 0x0E only (CAN_DOCK message — "may I come dock?")
**Confidence:** HIGH on all findings (verified by live decompilation of each callsite)
**Active in YR:** YES — fires every time a harvester tries to dock with a refinery

---

## 1. Summary of the CAN_DOCK(0x0E) Exchange

CAN_DOCK is the second radio message in the harvester→refinery approach handshake,
sent after HELLO(0x02) has already established the radio link (written the harvester
pointer into `Contacts[]`). The building side runs a multi-stage accept/reject filter.
On acceptance it sends back `MOVE_TO_CELL(0x12)` with a cell coord, then
`ENTER_DOCK(0x18)`, then `TIMING_SYNC(0x16)`.

---

## 2. Full Accept/Reject Filter in case 0x0E

```
BuildingClass::Receive_Radio case 0x0E:

Step 0: TechnoClass::Receive_Radio(this, 0x0E, ...)   ← always forwarded first

REJECT 1 — power check:
  if (building->HasPower == false) return 10   ← no-power = NEGATORY

REJECT 2 — UnitRepair already busy (UnitRepair= buildings):
  if (Type[0x16a9] != 0)         ← UnitRepair=yes
    && harvester already in Contacts[]
    && Transmit(0x22, harvester) == 10
  → return 10

REJECT 3 — Bunker deploy check (Bunker= buildings):
  if (Type[0x16ab] != 0)         ← Bunker=yes
    && harvester already in Contacts[]
    && !CanAutoDeployHere(harvester)
  → return 10

Branch: if NOT (Hospital=yes OR Armory=yes):    ← [0x16c1] and [0x16c2]
  --- MAIN REFINERY / DOCK PATH ---

  FREE-SLOT CHECK (FUN_0065adf0 = FindFreeContactSlot):
    Walks Contacts[] for a zero slot or one matching harvester.
    If no free slot found: calls Transmit(0x02/HELLO, harvester) to evict, then re-checks.

  SEND NEED_TO_MOVE(0x13) to harvester:
    iVar10 = Transmit_Radio(0x13, harvester)
    harvester returns ROGER(1) if not pathfinding, else NEGATORY(10).
    If NEGATORY (and stack sentinel == 0): return 1 (silent ignore, not a hard reject).

  On acceptance: write *param_4 = this (building pointer out-param)

  BRANCH — Helipad=yes (0x16cb) path (chrono helipad, NOT a refinery):
    if (Type[0x16cb] != 0):
      Send Transmit_Radio_Impl(0x12, &param_4, harvester)
      If reply != 0x14 (ALREADY_THERE): return 1
      Send Transmit_Radio_ToFirst(0x18)     ← ENTER_DOCK to first contact only
      return 1

  STANDARD REFINERY PATH (DockUnload=yes [0x16b3] OR Weeder=yes [0x16bc]):
    if (Type[0x16b3] != 0 || Type[0x16bc] != 0):
      → Compute queue cell (see §3)
      Transmit_Radio_Impl(0x12, &queue_cell, harvester) → MOVE_TO_CELL
      If reply != 0x14: return 1
      Transmit_Radio(0x18, harvester)               → ENTER_DOCK to harvester
      iVar10 = Transmit_Radio(0x16, harvester)      → TIMING_SYNC
      If iVar10 == 1: return 1
      PlaySound(&DAT_0089c848, 1, 1)                → "ok" sound cue
      return 1

Else branch (Hospital=yes OR Armory=yes):
  FUN_0065adf0 (free-slot check):
    if slot found: send Transmit_Radio_Impl(0x12, &nearest_cell, harvester), return 1
    if no slot: walk contacts, if any has Transmit(0x22)==10, send 0x17 to it, loop
    re-check slot; if still none: return 10
```

---

## 3. Queue-Cell Computation — Case 0x0E

Source code (from decompile):
```c
psVar5 = (short *)(**(code **)(param_1->vtable + 0x1b8))(&stack);
//    vtable+0x1b8 = ObjectClass__Get_Cell_Packed @ 0x0041BEA0
//    returns (X, Y) of building's anchor cell as two packed shorts
uStack_8 = (int *)CONCAT22(psVar5[1] + 1,   // Y += 1
                            *psVar5 + 3);    // X += 3
MapClass__Get_CellClass(&uStack_8);
// → queue cell is sent as *param_4 in Transmit_Radio_Impl(0x12, ...)
```

**The "+3,+1" formula is hardcoded.** It is NOT read from `BuildingTypeClass+0x1618`
(which stores the `QueueingCell=` INI value). `QueueingCell=` is parsed and stored at
`+0x1618` (X) / `+0x161C` (Y) by `BuildingTypeClass_ReadINI_Water` (0x0045FE50), but
the code in case 0x0E does **not** read those fields — it always uses anchor+3, anchor+1.

Verified: `ObjectClass__Get_Cell_Packed` returns `(+0x9C >> 8, +0xA0 >> 8)`, i.e., the
building's world coords converted to cell X and Y. No QueueingCell lookup is present in
this code path.

**Active in YR: YES.** The formula executes for every standard refinery dock.

---

## 4. Does case 0x0E Call BuildingClass::CanDock (0x457CE0)?

**No.** `BuildingClass::CanDock` (0x457CE0) has zero callers from
`BuildingClass::Receive_Radio`. Verified via `get_function_callers(0x457CE0)`:
callers are `FootClass::Find_Nearest_Dock` (0x004DFCB0), `TechnoClass::AI_Update`
(0x006F9E50), `TechnoClass::Evaluate_Candidate` (0x006F7CA0), and three other
unit-side helpers — none of which are inside Receive_Radio.

Case 0x0E has its own inline filter chain (§2). `BuildingClass::CanDock` is a
unit-side scout function used when a harvester is searching for a free refinery,
not when the building decides whether to accept an incoming dock request.

---

## 5. Reply Messages: What the Building Sends Back

| Step | Direction | Message code | Condition |
|------|-----------|-------------|-----------|
| 1 | building→harvester | `NEED_TO_MOVE(0x13)` | always (probe for motion state) |
| 2 | building→harvester | `MOVE_TO_CELL(0x12)` | acceptance; carries queue-cell as `*param_4` |
| 3 | building→harvester | `ENTER_DOCK(0x18)` | after MOVE_TO_CELL reply == 0x14 |
| 4 | building→harvester | `TIMING_SYNC(0x16)` | after ENTER_DOCK |

All four are sent by `Transmit_Radio` (vtable+0x278 = `RadioClass::Transmit_Radio` @
0x0065AAA0) or `Transmit_Radio_Impl` (vtable+0x27c @ 0x0065A970), both of which
forward to the harvester's own `Receive_Radio` via the harvester's vtable+0x194 slot.

The "yes" reply is **MOVE_TO_CELL(0x12)** carrying a `CellClass*` (the queue cell) as
the out-param `*param_4`. The harvester's `FootClass::Receive_Radio` case 0x12 checks
if it is already at that cell: if so it returns 0x14 (ALREADY_THERE), otherwise it
queues a movement command and returns 1 (ROGER). The 0x14 return is the gate that
allows ENTER_DOCK and TIMING_SYNC to be sent.

---

## 6. ENTER_DOCK(0x18) — Who Sends It and What It Does

**Sent by the building** (`Transmit_Radio(0x18, harvester)` at vtable+0x278).
Only sent after `MOVE_TO_CELL` reply == 0x14 (harvester already at queue cell).

`TechnoClass::Receive_Radio` case 0x18 (0x006F4AB0):
- If unit type == vehicle (GetWhat()==2) AND type flag at `+0xCD4`... specifically:
  checks `*(char *)(type + 0xe0d) != 0` — if that flag is set, breaks out (skips).
- If `this->field_0x198 == 0` (not yet in dock-entered state):
  sets `this->field_0x198 = 1`, then propagates `ENTER_DOCK(0x18)` to its own
  first contact via `Transmit_Radio(0x18, ...)`.
- Returns 1 on success.

The harvester does NOT write building field +0x2E4 (on-pad unit pointer) during
ENTER_DOCK. +0x2E4 is written later when the unit physically arrives on the pad.

**Active in YR: YES.**

---

## 7. TIMING_SYNC(0x16) — Who Sends It and What It Does

**Sent by the building** (`Transmit_Radio(0x16, harvester)` at vtable+0x278).
Only sent after ENTER_DOCK in the standard refinery path.

`UnitClass::Receive_Radio` case 0x16 (0x00737430):
- Forwards to `FootClass::Receive_Radio(0x16, ...)` first.
- If not chrono-teleporting (`field_0x6AF == 0`) AND timer not yet at 0x4000:
  sets locomotor timer to 0x4000 via `LocomotorClass::SetSpeed(0x4000)`.
- Then checks: if pathfinder idle AND harvester has destination AND destination is a
  building AND building mission == 7 (dock) AND harvester mission == 7:
  sends `TIMING_SYNC(0x15)` back to the building via `Transmit_Radio(0x15, building)`.

The building's `Receive_Radio` case 0x15 (same function, BuildingClass) sets
`field_0x6DD = 1`, sends anim slot transitions.

**Active in YR: YES.**

---

## 8. Building-Side State Changes When Accepting CAN_DOCK

From the decompile of case 0x0E:
- **No write to +0x2E4** (on-pad unit pointer) during CAN_DOCK. That field is set only
  when the harvester physically arrives on the pad (a separate event).
- **No "incoming unit" field** written during CAN_DOCK.
- **No dock-anim slot change** triggered from case 0x0E directly.
- The only state mutation observable in case 0x0E: the out-param `*param_4` is set to
  `this` (building pointer) or to the computed `CellClass*` for the queue cell.

---

## 9. Chrono Miner / Teleporter Specifics in case 0x0E

There is **no `Teleporter=` (field +0xCD4) branch in case 0x0E** of
`BuildingClass::Receive_Radio`. The building does not distinguish a chrono miner from
a normal harvester at the CAN_DOCK stage.

The Teleporter-specific check does appear in `TechnoClass::Receive_Radio` case 0x18
(`*(char *)(type + 0xe0d)`) and in `TechnoClass::CanAutoDeployHere` (used by the
Bunker=yes REJECT 3 branch for non-refinery buildings).

For a standard refinery (`DockUnload=yes`, not `Helipad=yes`, not `Bunker=yes`),
a chrono miner's CAN_DOCK is processed identically to a normal harvester's.

**Active in YR: YES (no special chrono-miner gate at case 0x0E).**

---

## 10. INI Key to BuildingTypeClass Offset Mapping (verified in ReadINI)

| Offset | INI key | Relevance to case 0x0E |
|--------|---------|----------------------|
| +0x16A9 | `UnitRepair=` | REJECT 2 trigger |
| +0x16AB | `Bunker=` | REJECT 3 trigger |
| +0x16B3 | `DockUnload=` | standard refinery path |
| +0x16BC | `Weeder=` | weed harvester path (same queue formula) |
| +0x16C1 | `Hospital=` | alternate path (no queue cell) |
| +0x16C2 | `Armory=` | alternate path (no queue cell) |
| +0x16CB | `Helipad=` | Helipad path (uses Transmit_ToFirst(0x18)) |
| +0x1618 | `QueueingCell=` X | stored but NOT read in case 0x0E |
| +0x161C | `QueueingCell=` Y | stored but NOT read in case 0x0E |
| +0x1780 | `NumberOfDocks=` | Contacts[] capacity (set at construction time) |

All offsets verified from `BuildingTypeClass_ReadINI_Water` (0x0045FE50) decompile.

---

## 11. Verified Facts Summary

| # | Fact | Evidence |
|---|------|----------|
| 1 | case 0x0E power-check is the first real filter: `HasPower==false → return 10` | Decompile 0x0043C2D0, case 0x0E, first branch |
| 2 | Queue cell = building anchor + (X+3, Y+1), hardcoded, NOT from QueueingCell INI | Decompile 0x0043C2D0; `ObjectClass__Get_Cell_Packed` @ 0x0041BEA0; ReadINI search confirms +0x1618 not referenced |
| 3 | `MOVE_TO_CELL(0x12)` is the "yes" reply; carries `CellClass*` in `*param_4`; gate is harvester reply == 0x14 | Decompile case 0x0E; `FootClass::Receive_Radio` case 0x12 @ 0x004D8FB0 |
| 4 | `BuildingClass::CanDock` (0x457CE0) is NOT called from case 0x0E; it is a unit-side scout function only | `get_function_callers(0x457CE0)` returns no Receive_Radio callers |
| 5 | TIMING_SYNC(0x16) triggers locomotor sync in harvester (`SetSpeed(0x4000)`) and can cascade to TIMING_SYNC(0x15) back to building | `UnitClass::Receive_Radio` case 0x16 @ 0x00737430 |
