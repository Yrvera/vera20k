# TechnoClass Vtable+0x484 — Post-Arrival Mission Dispatch — Ghidra Research Report

**Addresses:**
- TechnoClass vtable base: `0x007F4960`
- Vtable+0x484 entry word: `0x007F4DE4` → bytes `40 9A 70 00` (little-endian `0x00709A40`)
- TechnoClass base implementation: `0x00709A40` (FUN_00709a40)
- UnitClass override: `UnitClass__Scatter_Force` @ `0x00738970` (vtable `0x007F5C70`, entry `0x007F60F4`)
- InfantryClass override: `InfantryClass__IdleDispatch` @ `0x0051CBA0` (vtable `0x007EB058`, entry `0x007EB4DC`)
- FootClass::OnArrival: `0x004D82B0`
- Convoy-dequeue helper: `FUN_004da030` @ `0x004DA030`
- DriveLocomotionClass::Process: `0x004B0500`

**Confidence:** HIGH — vtable bytes read directly from binary; all three implementations decompiled.
**Active in YR:** YES — fires every time a ground unit (UnitClass or InfantryClass) arrives at its
destination cell. Runs multiple times per match.

---

## 1. Overview

TechnoClass vtable slot +0x484 is the **post-arrival mission dispatch** virtual method. It is called
by `DriveLocomotionClass::Process` (and by the Walk locomotor's equivalent) when a ground unit
finishes its move and has a non-empty tether/waypoint queue (`FootClass+0x598 != 0`). The slot is
overridden per class: UnitClass calls it `Scatter_Force`, InfantryClass calls it `IdleDispatch`, the
TechnoClass base implementation at `0x00709A40` is a fallback used only by classes that do not
override it.

**What the slot does (in all implementations):**
1. Calls `FootClass::OnArrival` — which handles the tether-queue dequeue (pops the next queued
   waypoint and issues a new `Set_Destination` call if the queue is non-empty).
2. Clears any temporal-weapon link (base implementation only).
3. Checks EMP state — returns early if the unit is under EMP.
4. Skips if current mission == 0x1C (already in the correct post-arrival mission state).
5. Calls the convoy-dequeue helper `FUN_004da030` — which checks a separate follow/convoy queue
   at `FootClass+0x16C/0x16F`.
6. Queues a new mission via `vtable+0x1E8` (`Queue_Mission`) based on whether the unit has a nav
   target, its current weapon capabilities, the house threat level, etc.

**Name confusion in earlier docs:** Three prior docs labeled this slot inconsistently:
- `TECHNOCLASS_VTABLE_COMPLETE.md`: "GetTarget_484" — WRONG (that function does not get a target).
- `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`: "ScanForTarget" — WRONG.
- `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md`: "Scatter_Force" — PARTIALLY CORRECT (this is the
  UnitClass override's Ghidra label, not the slot's semantic name).

The correct description is **Post-Arrival Mission Dispatch** or **OnArrival virtual**.

---

## 2. Vtable Index Confirmation

**Which class's vtable is indexed?** The owner object — the TechnoClass (specifically a UnitClass
or InfantryClass instance). Drive::Process holds the owner via `piVar2[2]` (DriveLocomotionClass
field at byte offset 0x8, the `ILocomotion.owner` back-pointer). The dispatch is:

```c
// piVar2 = DriveLocomotionClass* (param_1 of DriveLocomotionClass::Process)
// piVar2[2] = owner TechnoClass*
// *(int*)piVar2[2] = owner's vtable pointer
(**(code **)(*(int *)piVar2[2] + 0x484))(0, 1)
```

This is **NOT** through ILocomotion's vtable. It dispatches through the owner's own vtable.

**Binary proof:**
| Vtable | Base address | +0x484 byte offset | Bytes read | Resolved function |
|--------|-------------|---------------------|------------|-------------------|
| TechnoClass | `0x007F4960` | `0x007F4DE4` | `40 9A 70 00` | `0x00709A40` |
| UnitClass   | `0x007F5C70` | `0x007F60F4` | `70 89 73 00` | `0x00738970` = UnitClass__Scatter_Force |
| InfantryClass | `0x007EB058` | `0x007EB4DC` | `A0 CB 51 00` | `0x0051CBA0` = InfantryClass__IdleDispatch |

All three read directly from Ghidra memory in this session.

---

## 3. Call Sites in DriveLocomotionClass::Process (0x004B0500)

The slot is called in **three** code paths, all guarded by `FootClass+0x598 != 0` (the tether queue
count), and all preceded by `FootClass::Stop_Moving()`:

### 3a. Building-cell arrival with tether (lines ~50–70 of Process)
```
// NavTarget RTTI == 0xB (BuildingClass)
// current_cell.X == target_cell.X AND current_cell.Y == target_cell.Y
if (owner->tether_queue_count == 0):          // FootClass+0x598
    owner->vtable+0x480(0, 1)                  // StopMission / Set_Destination(NULL)
    return
// else (tether_queue_count != 0): fall through to LAB_004b0756
```

### 3b. Position arrival when mission==Move (via same LAB_004b0756)
```
// loco.destination != NullCoord AND owner.position == loco.destination
// is_on_track (loco+0x63) == 0
if (owner->tether_queue_count == 0):
    owner->vtable+0x480(0, 1)
    return
// LAB_004b0756: (shared by 3a and 3b)
FootClass__Stop_Moving()
owner->vtable+0x484(0, 1)                      // ← THE SLOT
return
```

### 3c. Path-failure can-still-move==false with tether (lines ~270–290 of Process)
```
// vtable+0x2CC (Can_Still_Move) returned false
// owner->tether_queue_count != 0
FootClass__Stop_Moving()
uVar5 = owner->vtable+0x484(0, 1)             // ← THE SLOT
if (uVar5 != 0): return
goto LAB_004b078c
```

**Key detail:** The two arguments `(0, 1)` passed at each call site map to:
- `param_2 = 0` — used by `FootClass::OnArrival` as a coord/target hint (when 0, no forced
  destination override)
- `param_3 = 1` — passed to FootClass::OnArrival. Newer `NAVCOM_ONARRIVAL_TAIL_HOOKS_GHIDRA_REPORT.md` verifies this is not an EVA/audio path; the `+0x687` branch is a deferred vtable `+0x174` hook that resolves to Scatter for stock Unit/Infantry.

---

## 4. Class Implementations

### 4a. TechnoClass base: `FUN_00709A40` @ `0x00709A40`

Called for any TechnoClass subclass that does NOT override this slot. Used as fallback.

```
// param_1 = TechnoClass this (int*)
1. if (this[0x9d] != 0 && *(this[0x9d] + 0x28) != 0):
       TemporalClass__DetachFromTarget()        // offset 0x9d*4 = 0x274 = temporal ptr

2. uVar1 = vtable+0x4D0()                      // → 0x0070F110 = XOR AL,AL; RET = always false

3. if (uVar1 == false):
       uVar1 = vtable+0x430()                  // → 0x00705D50: calls vtable+0x1D8 (Is_Unloading)
                                                //   returns (NOT Is_Unloading)
       if (uVar1 == true):                      // if NOT currently unloading
           uVar1 = FUN_006385c0()              // see §5 below

4. return uVar1
```

**Critical details:**
- `vtable+0x4D0` at `0x0070F110` is a 3-byte stub (`32 C0 C3` = `XOR AL,AL; RET`). It always
  returns 0/false. The first branch is dead in practice — always falls into the vtable+0x430 check.
- `vtable+0x430` at `0x00705D50`: raw bytes `8B 01 FF 90 D8 01 00 00 84 C0 0F 94 C0 C3` =
  loads vtable from `this`, calls `[vtable+0x1D8]` (Is_Unloading), then SETE AL (return NOT result).
  This is "returns true if the unit is NOT currently unloading".
- Temporal detach happens BEFORE the mission logic, on every call.

### 4b. UnitClass override: `UnitClass__Scatter_Force` @ `0x00738970`

```
1. FootClass__OnArrival(param_2, param_3)       // first call, saves result in uVar2

2. if vtable+0x4AC() != 0: return uVar2         // EMP-disabled: skip mission change
3. if GetMission() == 0x1C: goto END            // already in "arrived" mission: skip

4. if param_1[0xAB] != 0:                       // pending chrono-warp building
       BuildingClass__DeployUnit_ChronoWarp(1)

5. if GetMission() == 2 AND param_1[0x169] == 0:   // Guard mission, no nav target
       get current cell, compute cell center
       if position == cell center:
           vtable+0x274(3)                      // Assign facing = 3?

6. FUN_004da030()                               // convoy/follow dequeue (see §5)

7. Determine new mission iVar8:
   - if nav_target (param_1[0x169]) != 0:       // still has a tether target
       iVar8 = 2 (Guard)
   - else:
       if Deployer-type or building-enter conditions → goto END (no mission change)
       if can_fire conditions met: iVar8 = 0xB (Hunt)
       else: iVar8 = 5 (Guard/Idle)

8. Special convoy-destination check (mission==7 with AircraftTypeClass destination):
       adjust destination to dock approach cell if needed

9. if RTTI not in {0x19 (convoy), 0xB (building), 0x10 (aircraft), 9}:
       Queue_Mission(iVar8, 0)                  // vtable+0x1E8
END:
   return uVar2 & 0xFF
```

**Mission ID references verified (from context):**
- 2 = Guard mission (from Drive::Process: `if (iVar4 == 2) [Guard]` check)
- 5 = Move mission (from Drive::Process: `if (*(int*)(iVar4+0xAC) == 5)` check)
- 0xB = Move/Hunt (from RTTI type 0xB = Building)
- 0x1C = 28 = the "already arrived" or "post-arrival" mission state

### 4c. InfantryClass override: `InfantryClass__IdleDispatch` @ `0x0051CBA0`

Structure is identical to UnitClass__Scatter_Force:
1. `FootClass::OnArrival(param_2, param_3)`
2. EMP check (vtable+0x4AC)
3. Skip if mission == 0x1C
4. `FUN_004da030()` (convoy dequeue)
5. Determine new mission (uses `param_1[0xAD]` — a "spread" flag specific to InfantryClass —
   to choose between mission 1 vs 0xB; also keeps mission 8 and 0x11 if already in those states)
6. If RTTI not 0x19 or 0xB: `Queue_Mission(iVar3, 0)`

**Key difference from UnitClass:** InfantryClass uses `param_1[0xAD]` (spread-formation flag) to
choose mission 1 (Attack? or Formation?) instead of 0xB. It also preserves mission 8 (Capture) and
0x11 (Open) if the unit is already in those missions.

---

## 5. FootClass::OnArrival (0x004D82B0)

Called as the first thing by every implementation of vtable+0x484. This is what actually handles
the tether-queue dequeue. Its logic (decompiled and verified):

```
if (this[0x6B3] != 0): return 0        // re-entry guard: already processing arrival

this[0x6B3] = 1                         // set re-entry guard

FUN_00709a40(param_2, param_3)          // calls TechnoClass base impl directly (not via vtable)
                                         // handles temporal detach + FUN_006385c0

if (this[0x687] != 0):                  // deferred +0x174 hook flag
    this[0x687] = 0
    vtable+0x174(&DAT_008b3da8, 1, 0)   // stock Unit/Infantry resolve to Scatter

// Tether queue handling
if (this[0x166] > 0):                   // tether queue count > 0
    vtable+0x480(*(this[0x163]), 0)     // Set_Destination(next_target, 0)
    dequeue: shift array at this[0x163], decrement this[0x166]
    return 1                            // signal: re-queued next destination

// RTTI + build-center return-to-position logic (if returning from building, use saved exit cell)
// ... various RTTI checks ...

vtable+0x544(0, 0)                      // SetSpeed(0, 0) — stop speed
return 0
```

**Critical detail in FootClass::OnArrival:** There is a **re-entry guard** at offset +0x6B3. If this
byte is set, the function returns 0 immediately. This prevents double-firing if both Drive::Process
and a caller trigger OnArrival in the same tick.

**Also critical:** `FootClass::OnArrival` calls `FUN_00709a40(param_2, param_3)` **directly** (not
through vtable), passing the original args. This means the TechnoClass base implementation at
`0x00709A40` runs as part of every OnArrival call, regardless of which subclass is handling it.

---

## 6. Convoy Dequeue Helper: FUN_004da030 (0x004DA030)

Called after `FootClass::OnArrival` in both UnitClass and InfantryClass implementations.

```
// param_1 = TechnoClass this
if (this[0x169] == 0                     // no current nav target (just arrived)
    AND this[0x16F] > 0                  // convoy queue count > 0
    AND this[0x16C] != 0):              // convoy queue pointer valid
    
    iVar2 = *this[0x16C]                // peek first entry
    vtable+0x480(iVar2, 1)              // Set_Destination(iVar2, 1) — move to next convoy target
    
    // dequeue: shift left
    decrement this[0x16F]
    if this[0x16F] > 0: shift array at this[0x16C]
    
    // loop-back logic (if this[0x6B1] == 1 and conditions met):
    //   re-append the dequeued target to the end of the queue
    //   (circular convoy follow)
```

**Key offsets (FootClass layout):**
| Offset | Field | Purpose |
|--------|-------|---------|
| +0x166 × 4 = +0x598 | tether_queue_count | Count of queued waypoints (checked by Drive::Process) |
| +0x163 × 4 = +0x58C | tether_queue_ptr | Pointer to queued target array |
| +0x16C × 4 = +0x5B0 | convoy_queue_ptr | Pointer to convoy follow targets |
| +0x16F × 4 = +0x5BC | convoy_queue_count | Count of convoy targets |
| +0x6B1 | convoy_loop_flag | If set, convoy is circular (re-appends on dequeue) |
| +0x6B3 | arrival_guard | Re-entry guard for FootClass::OnArrival |
| +0x687 | deferred_arrival_hook_flag | OnArrival clears it and calls vtable `+0x174(&DAT_008B3DA8,1,0)`; stock Unit/Infantry resolve to Scatter, not EVA/audio |

Note: the tether queue (0x598) and convoy queue (0x5BC) are **separate** structures. Drive::Process
guards on the tether queue (0x598 / `param_1[0x166]`). FUN_004DA030 processes the convoy queue
(0x5BC). The two can coexist.

---

## 7. Active in YR: Yes — Trigger Frequency

**Active in YR: YES, unconditionally.**

This slot fires every time a ground vehicle (UnitClass) or infantry (InfantryClass) completes a
move in DriveLocomotionClass, specifically on the arrival paths where `tether_queue_count != 0`.
In normal play, every move order to a unit with nonzero NavQueue triggers it. `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` later found no standard runtime player, TeamClass/AI, or trigger waypoint producer for this queue; the "tether_target"
name from prior docs refers to the NavQueue count, not a physical tether. For single-step
moves (no waypoints), the parallel branch (`vtable+0x480`) fires instead.

For the War Miner: since the miner typically has a simple single-step move (no queued waypoints),
it fires `vtable+0x480` (StopMission) on arrival rather than `vtable+0x484`. The `vtable+0x484`
path fires on the miner only if it is tethered to a multi-stop path, which is uncommon.

No TS-legacy gates involved. No `SpecialFlags` or INI flag controls this path.

---

## 8. Discrepancies with Prior Docs

| Doc | Claim about vtable+0x484 | Status |
|-----|--------------------------|--------|
| `TECHNOCLASS_VTABLE_COMPLETE.md` §289 | "GetTarget_484" | **WRONG** — the function does not get a target |
| `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` §4 table | "ScanForTarget" | **WRONG** |
| `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md` §STOP_AND_SCATTER | "Scatter_Force" | **PARTIALLY CORRECT** — this is the UnitClass override's Ghidra label, not the slot semantic |
| `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` Open Question #3 | "Likely Cancel_Mission, Mark_Cell_Departed, Clear_Destination, or arrival-finalization helper" | **RESOLVED** — it is a post-arrival mission re-assignment dispatcher |

**Recommended fix:** In `TECHNOCLASS_VTABLE_COMPLETE.md`, entry 289 should read
`TechnoClass::PostArrival_MissionDispatch` or `TechnoClass::OnArrival_484` with address `0x00709A40`.
The UnitClass override is `UnitClass__Scatter_Force` at `0x00738970`; InfantryClass override is
`InfantryClass__IdleDispatch` at `0x0051CBA0`. All three are confirmed by binary vtable reads.

---

## 9. Open Questions Log — Final State

- `[RESOLVED] OQ1` — Which class's vtable is indexed in Drive::Process? → The owner TechnoClass
  (UnitClass/InfantryClass), dispatched via `*(int*)owner + 0x484`. Confirmed from decompile of
  Drive::Process and vtable byte reads. Not through ILocomotion.

- `[RESOLVED] OQ2` — What is at TechnoClass vtable+0x484 binary? → `0x00709A40` (FUN_00709a40),
  bytes `40 9A 70 00` at `0x007F4DE4`. Confirmed by binary read.

- `[RESOLVED] OQ3` — What is at UnitClass vtable+0x484? → `UnitClass__Scatter_Force` at `0x00738970`,
  bytes `70 89 73 00` at `0x007F60F4`. Confirmed by binary read.

- `[RESOLVED] OQ4` — What is at InfantryClass vtable+0x484? → `InfantryClass__IdleDispatch` at
  `0x0051CBA0`, bytes `A0 CB 51 00` at `0x007EB4DC`. Confirmed by binary read.

- `[RESOLVED] OQ5` — What is the semantic of the slot? → Post-arrival mission dispatch: calls
  FootClass::OnArrival (tether dequeue), then assigns next mission. Evidence: decompile of all three
  implementations.

- `[RESOLVED] OQ6` — Is vtable+0x4D0 (called in base impl) a real function? → No. It is a 3-byte
  stub `XOR AL,AL; RET` at `0x0070F110`, always returns false. The branch it gates is dead.

- `[RESOLVED] OQ7` — What does vtable+0x430 do? → Calls `Is_Unloading` (vtable+0x1D8) and returns
  logical NOT. Bytes at `0x00705D50` verified. So base impl calls `FUN_006385c0` if unit is NOT
  unloading.

- `[RESOLVED] OQ8` — What is `FUN_006385c0`? → Script/trigger processing function. Reads from a
  global script queue, calls `Queue_Mission(0x1C, 1)` if script permits. It handles the
  TechnoClass-base path for post-arrival mission queueing (only used by classes that don't override
  vtable+0x484 with a full UnitClass/InfantryClass implementation).

- `[RESOLVED] OQ9` — Is FootClass::OnArrival the same as what Drive::Process uses? → Yes. The
  UnitClass and InfantryClass implementations call `FootClass::OnArrival(param_2, param_3)` directly
  as their first action. The base `FUN_00709A40` is called from within OnArrival (directly, not via
  vtable). Evidence: decompile of `FootClass__OnArrival` at `0x004D82B0`.

- `[RESOLVED] OQ10` — Re-entry guard for FootClass::OnArrival? → Yes, at FootClass offset +0x6B3.
  Function returns 0 immediately if this byte is set. Set to 1 on entry, but never cleared in this
  session (cleared elsewhere, presumably at start of next tick). Evidence: decompile of OnArrival.

- `[RESOLVED] OQ11` — Are there TS-legacy gates? → None found. All code paths are live in YR by
  default. No `SpecialFlags` or INI flag controls this slot.

- `[RESOLVED] OQ12` — How many call sites in Drive::Process? → Three paths, two sharing LAB_004b0756
  (building-cell arrival, position arrival); one separate path (path-failure can-still-move==false).
  All three are guarded by `tether_queue_count (FootClass+0x598) != 0`.

- `[DEFERRED] OQ13` — What is the re-entry guard clearing mechanism for FootClass+0x6B3?
  (category: requires-different-system-context; reason: requires tracing the full tick cycle to find
  where +0x6B3 is reset to 0; next-step-if-pursued: search xrefs to writes of byte at +0x6B3
  across all functions)

- `[DEFERRED] OQ14` — What exactly is mission ID 0x1C in the RA2/YR mission enum?
  (category: requires-different-system-context; reason: mission enum is not documented in any
  current doc; next-step-if-pursued: trace `MissionClass::Queue_Mission` callers, find enum
  definition in binary or TS source)

- `[DEFERRED] OQ15` — What does vtable+0x274 do when called with argument 3 in UnitClass__Scatter_Force?
  (category: out-of-scope; reason: this is a facing assignment call in a Guard-mission centering check,
  not part of the arrival dispatch semantic)

---

## 10. Sources

- `gamemd.exe` via Ghidra MCP — all functions decompiled live in this session
- Addresses decompiled: `0x004B0500`, `0x00709A40`, `0x00738970`, `0x0051CBA0`, `0x004D82B0`,
  `0x004DA030`, `0x00637E00`, `0x006385C0`, `0x00705D50` (raw bytes), `0x0070F110` (raw bytes)
- Memory reads: `0x007F4DE4` (TechnoClass vtable+0x484), `0x007F60F4` (UnitClass vtable+0x484),
  `0x007EB4DC` (InfantryClass vtable+0x484), `0x007F5C70` (UnitClass vtable base from constructor
  bytes at `0x00735793`), `0x007EB058` (InfantryClass vtable base from constructor bytes `0x00517D9A`)
- Prior docs consulted (for conflict resolution):
  - `TECHNOCLASS_VTABLE_COMPLETE.md`
  - `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`
  - `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md`
  - `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md`
