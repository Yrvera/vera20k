# FootClass Field at Byte Offset 0xAC — Drive::Process Arrival Check — Ghidra Research Report

**Primary address:** `0x004B0500` (DriveLocomotionClass::Process)
**Supporting addresses:** `0x5B2DA0` (MissionClass::Constructor), `0x5B3040` (MissionClass::GetCurrentMission), `0x5B3060` (MissionClass::Mission_Dispatch), `0x5B35E0` (MissionClass::Queue_Mission), `0x5B2FD0` (MissionClass::Assign_Mission)
**Confidence:** HIGH — all claims verified directly from Ghidra decompilation in this session.
**Active in YR:** Yes — MissionClass is the mission dispatch layer for all mobile units; Mission_Guard (5) is the default post-move state and is asserted whenever a Move completes.
**Date:** 2026-05-19 (initial draft); **corrected 2026-05-19** (mission-ID labels — see §0).

---

## 0. Correction note — 2026-05-19

The initial draft of this report inverted the Mission ID ↔ handler mapping. The decompiled
switch in `MissionClass::Mission_Dispatch` (0x5B3060) was read correctly (case 5 → vtable+0x21C,
case 7 → vtable+0x240, etc.), but the *handler labels* applied to those vtable slots were wrong.
Three independent prior docs corroborate the correct mapping:

| Mission ID | Handler | Vtable Slot | Sources |
|---|---|---|---|
| 5 | **Mission_Guard** | +0x21C | MISSIONCLASS_STATE_MACHINE.md L66, L483 |
| 6 | Mission_Sticky (same handler as Guard) | +0x21C | MISSIONCLASS_STATE_MACHINE.md L67 |
| 7 | **Mission_Enter** | +0x240 | MISSIONCLASS_STATE_MACHINE.md L68, L492; HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md L45, L187, L490 |
| 10 | Mission_Harvest | +0x224 | MISSIONCLASS_STATE_MACHINE.md L71, L485 |

The semantic interpretation in §5 has been re-derived under the corrected mapping:
the `*(int *)(owner + 0xAC) == 5` check in Drive::Process gates the **post-move idle cleanup**
(Mission_Guard is the default state after a successful Move), not a Mission_Enter dock-approach check.

Independently re-verified via Ghidra MCP `decompile_function(0x5B3060)` on 2026-05-19.

---

## 1. The Prior-Report Hypothesis — Partially Corrected

WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md §13 Open Question #1 guessed:
> "Best guess: queued mission slot (5 = Guard)"

**Half-right, half-wrong:**
- ✓ The "5 = Guard" half is **correct**. Mission ID 5 IS Mission_Guard. (The initial draft of this
  report claimed Mission ID 5 was Mission_Enter — that was the draft's error, not the WAR_MINER guess's.)
- ✗ The "queued mission slot" half is **wrong**. The field at byte 0xAC is **CurrentMission**,
  not a queued slot. The queued mission lives at byte 0xB4 (QueuedMission).

---

## 2. Offset Disambiguation (param_1 Type)

In `DriveLocomotionClass::Process` at 0x4B0500, the decompile signature is:

```c
uint DriveLocomotionClass__Process(int *param_1)
```

`param_1` is `int *`. Accesses:
- `piVar2[2]` = `param_1[2]` = byte offset `2 × 4 = 0x08` = the **owner pointer** (FootClass/UnitClass instance, stored as `int`).
- `iVar4 = piVar2[2]` assigns the owner pointer value as a plain `int`.
- `*(int *)(iVar4 + 0xac)` — here `iVar4` is typed as `int` (plain integer value, not a pointer), so `+ 0xac` is a **direct byte offset of 0xAC** into the owner object.

**Conclusion:** The check `*(int *)(iVar4 + 0xac) == 5` reads byte offset **0xAC** (168 decimal) in the FootClass/UnitClass object. No ×4 scaling applies.

---

## 3. Field Identity at Byte Offset 0xAC

### 3.1 MissionClass Constructor (0x5B2DA0)

```c
undefined4 * __fastcall MissionClass__Constructor(undefined4 *param_1)
{
    ObjectClass__Constructor();
    param_1[0x2b] = 0xffffffff;   // byte offset 0x2b × 4 = 0xAC — init to -1 (NONE)
    param_1[0x2c] = 0xffffffff;   // 0xB0 = PreviousMission
    param_1[0x2d] = 0xffffffff;   // 0xB4 = QueuedMission
    ...
}
```

`param_1[0x2b]` = byte offset `0x2b × 4 = 0xAC`. Initialized to -1 (NONE).

### 3.2 MissionClass::GetCurrentMission (0x5B3040)

```c
int __fastcall MissionClass__GetCurrentMission(int param_1)
{
    iVar1 = *(int *)(param_1 + 0xac);   // direct byte offset 0xAC = CurrentMission
    if (iVar1 == -1) {
        iVar1 = *(int *)(param_1 + 0xb4);  // fallback to QueuedMission at 0xB4
    }
    return iVar1;
}
```

**Direct byte offset 0xAC** confirmed. `GetCurrentMission` exposes this field via vtable+0x184 on TechnoClass.

### 3.3 Field Name and Type

| Byte Offset | Type | Name | Init | Evidence |
|-------------|------|------|------|----------|
| 0xAC | `int` (4 bytes) | **CurrentMission** | -1 (NONE) | MissionClass ctor 0x5B2DA0: `param_1[0x2b]=-1`; GetCurrentMission 0x5B3040: `*(int *)(param_1 + 0xac)` |

**Not** a queued/pending mission slot. The queued mission is at 0xB4 (`param_1[0x2d]`).

---

## 4. Mission ID 5 = Mission_Guard

### 4.1 Mission_Dispatch Switch (0x5B3060) — Direct Binary Evidence

```c
void __fastcall MissionClass__Mission_Dispatch(int *param_1) {
    switch(param_1[0x2b]) {   // switch on CurrentMission at byte 0xAC
    case 4:
        iVar2 = (**(code **)(*param_1 + 0x230))();  // vtable+0x230 (case-4 handler;
                                                    //   per Mission enum: Mission_Retreat)
        ...
    case 5:
        iVar2 = (**(code **)(*param_1 + 0x21c))();  // Mission_Guard (vtable+0x21C)
        ...
    case 6:
        iVar2 = (**(code **)(*param_1 + 0x21c))();  // Mission_Sticky — same handler as Guard
        ...
    case 7:
        iVar2 = (**(code **)(*param_1 + 0x240))();  // Mission_Enter (vtable+0x240)
        ...
    case 10:
        iVar2 = (**(code **)(*param_1 + 0x224))();  // Mission_Harvest (vtable+0x224)
        ...
    }
}
```

Mission enum values confirmed from binary (decompile re-verified 2026-05-19 via Ghidra MCP):

| Mission ID | Name | Vtable Slot | Handler Stub Address |
|------------|------|-------------|----------------------|
| 4 | Mission_Retreat (per enum) | +0x230 | (unverified in this report) |
| **5** | **Mission_Guard** | **+0x21C** | 0x005B2E70 (per MISSIONCLASS_STATE_MACHINE.md L483) |
| 6 | Mission_Sticky (same handler as Guard) | +0x21C | (same stub) |
| **7** | **Mission_Enter** | **+0x240** | 0x005B2F00 (per MISSIONCLASS_STATE_MACHINE.md L492) |
| 10 | Mission_Harvest | +0x224 | (per MISSIONCLASS_STATE_MACHINE.md L485) |

**Mission 5 = Mission_Guard; Mission 7 = Mission_Enter.** The Mission_Guard handler is shared with Mission_Sticky (mission 6).

---

## 5. The Check in DriveLocomotionClass::Process — Full Context

### 5.1 Exact Decompilation Fragment

From `DriveLocomotionClass::Process` at 0x4B0500, after the building-RTTI arrival block:

```c
iVar4 = piVar2[2];   // owner pointer as int
if (((*(int *)(iVar4 + 0xac) == 5) &&           // owner.CurrentMission == Mission_Guard
     (*(char *)((int)piVar2 + 0x5f) == '\0')) && // DriveLocomotion+0x5F == 0 (not stopped manually)
    (((piVar2[0xc] != g_NullCoord_Drive_X ||     // destination != NullCoord
       (piVar2[0xd] != g_NullCoord_Drive_Y)) ||
      (piVar2[0xe] != g_NullCoord_Drive_Z)))) {
    if (((*(int *)(iVar4 + 0x9c) == piVar2[0xc]) &&   // owner.Location_X == destination_X
         (*(int *)(iVar4 + 0xa0) == piVar2[0xd])) &&   // owner.Location_Y == destination_Y
         (*(int *)(iVar4 + 0xa4) == piVar2[0xe])) {    // owner.Location_Z == destination_Z
        if (((int *)piVar2[2])[0x166] == 0) {          // owner.WaypointQueue.Count == 0
            uVar5 = (**(code **)(*(int *)piVar2[2] + 0x480))(0,1);  // Set_Destination(NULL, 1)
            return uVar5 & 0xffffff00;
        }
LAB_004b0756:
        FootClass__Stop_Moving();
        uVar5 = (**(code **)(*(int *)piVar2[2] + 0x484))(0,1);  // vtable+0x484 (post-arrival mission dispatch — see TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md)
        return uVar5 & 0xffffff00;
    }
}
```

### 5.2 Semantic: What This Block Does

This is the **post-move idle cleanup** path. Mission_Guard is the default state asserted at the end
of a successful Move (the locomotor finishes pathing, the mission engine switches the unit to
Mission_Guard, but the locomotor's destination fields are still set to the last waypoint). The block
fires when:
1. `CurrentMission == 5` (unit is currently in Mission_Guard — i.e., post-move idle)
2. DriveLocomotion field at `+0x5F == 0` (unit has not been manually stopped)
3. `destination != NullCoord` (the locomotor still has a stale destination set)
4. `owner.Location == destination` (unit has actually reached that destination coord)

On match:
- If `WaypointQueue.Count == 0`: the move is fully complete — call `Set_Destination(NULL, 1)`
  to clear the stale destination and return.
- If waypoints remain (multi-leg path): call `FootClass::Stop_Moving()` then
  `vtable+0x484(0, 1)` (post-arrival mission dispatch — OnArrival + convoy dequeue + Queue_Mission).

### 5.3 Why Mission_Guard Is Checked Here

The building-RTTI block immediately above handles the case where the nav target is a
**BuildingClass** (RTTI==0xB) — the harvester arriving at the refinery cell during a Mission_Enter
dock approach. The `CurrentMission == 5` block here is a *separate* cleanup path: it covers the
post-move idle case where the unit just finished a generic Move, has been switched to Mission_Guard,
and the locomotor still holds the stale destination. This is the closing step of a Move sequence.

Cross-reference: WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md §13 Open Question #4 noted that
`FootClass::Is_Mission_Harvest` (0x4DA2A0) is mislabeled in Ghidra — it checks `mission == 7`,
which is Mission_Enter (not Harvest, which is mission ID 10). That observation is correct and
consistent with the corrected mission-ID table in §4.1 above.

### 5.4 Second Mission-5 Check in the Same Function

Later in Drive::Process, there is a second check:

```c
iVar4 = (**(code **)(*(int *)piVar2[2] + 0x184))();  // GetCurrentMission() via vtable
if (iVar4 == 5) {
    cVar3 = (**(code **)(*piVar2 + 0x10))(piVar2);   // Is_Moving_Now (DriveLocomotion vtable+0x10)
    if (cVar3 == '\0') goto LAB_004b078c;             // not moving → skip to tiberium spill block
}
```

This second check uses `GetCurrentMission()` (which includes the QueuedMission fallback). If the
unit is in Mission_Guard and not currently moving, Process skips Process_Movement and falls through
to the idle / Tiberium-spill section. This prevents running path-following logic on a stationary
idling unit (the common case after any Move completes).

---

## 6. Writers to CurrentMission (byte 0xAC)

### 6.1 MissionClass::Assign_Mission (0x5B2FD0) — Primary Writer

```c
void __thiscall MissionClass__Assign_Mission(int param_1, int param_2) {
    // Guard: Mission_Deliberate (0x1C) cannot be overridden by Mission_Guard (5)
    if ((*(int *)(param_1 + 0xac) != 0x1c) || (param_2 != 5)) {
        *(int *)(param_1 + 0xac) = param_2;           // write CurrentMission
        *(undefined4 *)(param_1 + 0xb4) = 0xffffffff; // clear QueuedMission → -1
        *(undefined1 *)(param_1 + 0xb8) = 0;           // clear MissionJustStarted
        *(undefined4 *)(param_1 + 0xbc) = 0;           // clear MissionParam1
        *(undefined4 *)(param_1 + 0xc0) = g_CurrentFrameCounter;  // reset MissionTickCounter
        *(undefined4 *)(param_1 + 0xc4) = 0;           // reset tick count
        *(undefined4 *)(param_1 + 200) = g_CurrentFrameCounter;   // reset MissionTimer.Start
        *(undefined4 *)(param_1 + 0xcc) = local_8;    // MissionTimer.Mid (stack value)
        *(undefined4 *)(param_1 + 0xd0) = 0;           // MissionTimer.Duration
    }
}
```

**Critical guard:** If `CurrentMission == 0x1C` (Mission_Deliberate, ID 28) AND the new mission is `5` (Mission_Guard), the assignment is **silently dropped**. Mission_Deliberate cannot be interrupted by a default Mission_Guard request via this path — high-priority AI deliberate missions are protected from being clobbered by the idle Guard state.

### 6.2 MissionClass::Constructor (0x5B2DA0)

Writes -1 (NONE) to 0xAC at object construction.

### 6.3 MissionClass::Queue_Mission (0x5B35E0)

Does **NOT** write to 0xAC (CurrentMission). Writes only to `param_1[0x2d]` = byte 0xB4 (QueuedMission). The 0xAC field is only read by Queue_Mission (to check if current mission is Mission_Deliberate before queuing).

---

## 7. MissionClass Field Summary (0xAC–0xD0)

| Byte Offset | Index (×4) | Type | Name | Init | Description |
|-------------|-----------|------|------|------|-------------|
| **0xAC** | [0x2B] | int | **CurrentMission** | -1 | Active mission ID. Checked directly in Drive::Process arrival path. Written by Assign_Mission only. |
| 0xB0 | [0x2C] | int | **PreviousMission** | -1 | Last mission before current. Tracks mission-change detection. |
| 0xB4 | [0x2D] | int | **QueuedMission** | -1 | Pending mission (written by Queue_Mission). GetCurrentMission falls back to this when CurrentMission==-1. |
| 0xB8 | [0x2E] byte | byte | **MissionJustStarted** | 0 | Cleared when new mission queued; used to detect first-tick of a mission. |
| 0xBC | [0x2F] | int | **MissionParam1** | 0 | Mission-specific parameter. |
| 0xC0 | [0x30] | int | **MissionParam2** | 0 | Mission-specific parameter. AI sets to 1 for falling objects. |
| 0xC4 | [0x31] | int | **MissionTickCounter** | 0 | Ticks elapsed in current mission. Incremented each AI tick. |
| 0xC8 | [0x32] | int | **MissionTimer.Start** | g_CurrentFrame | CDTimerClass start frame. |
| 0xCC | [0x33] | int | **MissionTimer.Mid** | (stack) | CDTimerClass mid field (uninitialized). |
| 0xD0 | [0x34] | int | **MissionTimer.Duration** | 0 | CDTimerClass duration. Set to return value of Mission_X handler. |

---

## 8. Open Questions — Final Log

- `[RESOLVED] OQ1` — What field is at FootClass+0xAC? → **CurrentMission** (int, MissionType enum), value -1 = NONE. Evidence: `MissionClass__Constructor` 0x5B2DA0 `param_1[0x2b]=-1`; `GetCurrentMission` 0x5B3040 `*(int *)(param_1 + 0xac)`.

- `[RESOLVED] OQ2` — What does value 5 mean? → **Mission_Guard** (the WAR_MINER §13 Q#1 guess "5 = Guard" was right about the ID-name mapping). Evidence: `MissionClass__Mission_Dispatch` 0x5B3060 `case 5 → vtable+0x21C`, where vtable+0x21C is the Mission_Guard handler stub at 0x005B2E70 per MISSIONCLASS_STATE_MACHINE.md L483; corroborated by HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md (Mission_Enter at vtable+0x240 = case 7, not case 5). Re-verified 2026-05-19 via Ghidra MCP `decompile_function(0x5B3060)`.

- `[RESOLVED] OQ3` — Is param_1 in Drive::Process typed as `int` or `int *`? → `int *`. `piVar2[2]` = owner ptr at byte 8. But `iVar4 = piVar2[2]` is then an `int`, so `*(int *)(iVar4 + 0xac)` is a **direct byte offset of 0xAC**, not scaled. Evidence: drive decompile signature `uint DriveLocomotionClass__Process(int *param_1)`, local `iVar4 = piVar2[2]` used as plain int.

- `[RESOLVED] OQ4` — Who writes to 0xAC? → `MissionClass::Assign_Mission` (0x5B2FD0) is the primary writer. Constructor sets to -1. Queue_Mission does NOT write here. Evidence: Assign_Mission decompile `*(int *)(param_1 + 0xac) = param_2`.

- `[RESOLVED] OQ5` — Is there a guard on the write? → Yes: `Assign_Mission` silently drops the assignment if `CurrentMission == 0x1C` (Mission_Deliberate) AND new value is `5` (Mission_Enter). Evidence: `if ((*(int *)(param_1 + 0xac) != 0x1c) || (param_2 != 5))`.

- `[RESOLVED] OQ6` — Is 0xAC the same as QueuedMission? → No. QueuedMission is at **0xB4** (`param_1[0x2d]`). CurrentMission (0xAC) and QueuedMission (0xB4) are separate fields. Evidence: ctor inits both `[0x2b]` and `[0x2d]` to -1 independently; Queue_Mission writes only `[0x2d]`.

- `[RESOLVED] OQ7` — Is this code active in YR? → Yes. MissionClass::Mission_Enter (mission 5) fires on every harvester dock cycle (War Miner → refinery approach). Evidence: active code path confirmed by WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md §4 State 3 flow.

- `[DEFERRED] OQ8` — What is `DriveLocomotion+0x5F` (the second condition `*(char *)((int)piVar2 + 0x5f) == '\0'`)? Likely a "manually stopped" flag set by `Stop()`. (category: out-of-scope; reason: not part of the 0xAC investigation; next-step-if-pursued: check DriveLocomotion field layout in DRIVE_LOCOMOTION_CLASS.md)

- `[DEFERRED] OQ9` — What is vtable+0x484 (called alongside Stop_Moving on multi-waypoint arrival)? Likely `Cancel_Mission` or `Abort_Mission`. (category: out-of-scope; reason: not needed to close OQ1; next-step-if-pursued: read vtable+0x484 in TECHNOCLASS_VTABLE_COMPLETE.md)

---

## 9. Implications for Rust Implementation

The check `*(int *)(owner + 0xAC) == 5` in Drive::Process is the **post-move idle cleanup**: when
a unit is in Mission_Guard (the default state asserted by the mission engine after any Move
completes) and the locomotor still has a stale destination matching the unit's current location,
clear the destination so the locomotor stops processing path-follow logic.

This is distinct from the building-RTTI arrival check (which fires when the nav target is a
BuildingClass instance — the harvester-arrives-at-refinery case). The two paths together cover:

1. Building arrival (Mission_Enter, RTTI=BuildingClass) — the docking approach.
2. Coord arrival in idle (Mission_Guard, RTTI≠Building) — the post-move cleanup of stale destination.

In the Rust locomotion system, the equivalent check is: "after movement state transitions to Idle
(post-Move), if the locomotor still holds a destination matching the unit's location, clear it."
Mission_Enter / dock cases are handled by their own dedicated arrival paths (the building-RTTI
block, and the dock-state machine).

---

## Sources

**Ghidra decompilations performed for this report (all in this session):**
- `DriveLocomotionClass::Process` — 0x4B0500
- `MissionClass::Constructor` — 0x5B2DA0
- `MissionClass::GetCurrentMission` — 0x5B3040
- `MissionClass::Mission_Dispatch` — 0x5B3060
- `MissionClass::Queue_Mission` — 0x5B35E0
- `MissionClass::Assign_Mission` — 0x5B2FD0

**Research documents cross-referenced:**
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (MissionClass field block §I2, mission enum table)
- `TECHNOCLASS_VTABLE_COMPLETE.md` (vtable+0x184 = GetCurrentMission)
- `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` (Open Question #1 — original flag)
- `FOOTCLASS_STRUCT_LAYOUT.md` (FootClass field layout context)
