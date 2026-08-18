# Mission_Deploy_Building State Machine — Detailed Ghidra Report

**Function:** `UnitClass::Mission_Deploy_Building` @ `0x0073D630`
**Extent:** `0x0073D630` – `0x0073E5BD` (3966 bytes)
**Date:** 2026-05-19
**Status:** EXTENDS `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` (2026-03-27). Verified from live decompilation. Focuses on harvester-unload state machine, chrono miner branches, caller dispatch, and edge cases.
**Active in YR:** Yes — every harvester and MCV uses this every match.

---

## 1. Mission Dispatch Binding

**Verified from binary:**

- UnitClass vtable base: `0x007F5C74` (confirmed via constructor `*param_1 = &vtable__UnitClass` + GetCLSID at +0x08)
- `Mission_Deploy_Building` resides at vtable offset `+0x238` (= `0x007F5C74 + 0x238 = 0x007F5EAC`; read_memory confirms `0x0073D630` there)
- `MissionClass::Mission_Dispatch` @ `0x005B3060`, case `0xD` (13 = "Stop"): calls `vtable+0x238`
- UnitClass overrides the base `Mission_Stop` stub at vtable+0x238 with `Mission_Deploy_Building`
- **Mission code that routes here: 13 (enum name "Stop", but UnitClass repurposes it as Deploy/Unload)**
- No direct callers found — dispatched exclusively via vtable through `Mission_Dispatch`

**Active in YR:** Yes. Unconditional; fires every tick when CurrentMission == 13.

---

## 2. Top-Level Dispatch: Three Paths

The function keys on two fields first:

```
if (param_1[0xb9] == 0) {          // DockedTo == NULL → NOT DOCKED
    if (UnitTypeClass+0x5E0 < 1)   // Storage == 0
        goto LAB_0073d672          // skip harvester approach
    switch (param_1[0x2f])         // MissionState (byte offset 0xBC)
        case 0: approach init
        case 1: driving in
        case 3: find exit cell / physically enter building
        case 4: set guard, mission complete
} else {                           // DOCKED → trigger post-dump exit
    BuildingClass__ReleaseDockedHarvester()
    fall-through to LAB_0073d672
}

LAB_0073d672:                      // Harvester/MCV/SimpleDeployer branch point
    if (!Harvester && !Weeder)
        MCV or SimpleDeployer path
    else
        Harvester ore-dump state machine
```

**MCV vs Harvester distinction:** checked via `UnitTypeClass+0xE0E` (Harvester) and `+0xE0F` (Weeder). If neither, falls to MCV/SimpleDeployer path keyed on `UnitTypeClass+0x404` (DeploysInto).

---

## 3. MissionState Sub-State Enumeration (param_1[0x2f] = byte offset 0xBC)

### 3A. Undocked Harvester Approach Path (DockedTo == 0, Storage > 0)

| State | Entry Action | Tick Action | Exit Condition | Next State |
|-------|-------------|-------------|----------------|-----------|
| **0** | — | Check locomotor (vtable+0x4: Is_Moving). If moving → return 10. Check cell occupation (cell+0xEC == 2). If blocked → FindNearbyPassableCell + scatter → return 10. Call GetDockDirection (vtable+0x304). | Valid direction found AND position differs from dock coord | **1** (transition written, return 1) |
| **0** alt | — | If no valid dock direction | — | Calls SetMission(Guard=5), remains in dispatch |
| **1** | — | Check `byte 0x6AF` (IsMoving). | IsMoving == 0 (stopped) | **3** (return 1) |
| **1** cont | — | If still moving | — | break → common timer return |
| **3** | — | Check `param_1[0x1b9] < param_1[0x45]` (dump index < dock direction count). If true, dequeue building (FUN_004de710), iterate 8 directions for exit cell, verify passability, place unit. | `param_1[0x1b9] >= param_1[0x45]` (all steps done) | **4** |
| **3** no-cell | — | If no valid exit cell in all 8 directions → requeue building, optionally ForceEject, call building vtable[0x47] (reset) | — | stays in 3 / breaks |
| **4** | SetMission(Guard=5), set byte 0xB8=1 (MissionComplete) | — | — | — (terminal) |

### 3B. Harvester Ore Dump Path (at LAB_0073d672, Harvester || Weeder)

These states use the **same** `param_1[0x2f]` field (0xBC), but reached via the docked continuation.

| State | Entry Action | Tick Action | Exit Condition | Next State |
|-------|-------------|-------------|----------------|-----------|
| **init (0x6D1==0)** | Reset dump timer (`param_1[0x3E]=0`), set `byte 0x6D1=1`, set anim frame counters, call SetAnimSlot(7) for dock-door OPEN (Harvester only) | — | — | **3** (param_1[0x2f] = 3) |
| **3** | — | Per-bale loop each tick (see §4 for timing) | StorageClass empty (FindFirstNonEmptySlot == -1) | **4** (param_1[0x2f] = 4) |
| **3** refinery-gone | — | If building not found → check path, ScanForTargets(3), SetMission(Harvest=10, 1) | — | SetMission(Harvest) |
| **4** (Harvester) | — | Wait for dock door anim finish (building+0x57C != 0). Clear `byte 0x6D1=0`. SetMission(Harvest=10,0). | Dock door anim done | — (normal exit via Harvest) |
| **4** (Weeder) | Immediately clear `byte 0x6D1=0` — NO door wait | SetMission(Harvest=10,0) | Immediate | — |

### 3C. MCV Deploy Path (DeploysInto != NULL)

| State | Action |
|-------|--------|
| **0** | ScanForTargets(3), clear TargetDockBuilding, → state 1 |
| **1** | Wait for locomotor stop, call UnitClass::Deploy(). On deploy success: SetMission(Guard=5) or SetMission(Hunt=0xF) depending on player control and multiplayer flag. If locomotor still moving and CurrentMission valid: QueueMission. → (deploy handled in UnitClass::Deploy @ 0x00739390) |
| **2** | DeployToFire=0: ForceScatter, QueueMission. DeployToFire=1: call Deploy again; if fail and IsSlaveMiner: clear DeployToFire. |

---

## 4. Per-Frame Dump Timing Path (State 3, Harvester)

**Verified from decompile:**

```c
// Dump fires when:
if (*(double *)(g_RulesClass_Instance + 0x1528) * _DAT_007e27f8 <= (double)param_1[0x3e])
```

- `g_RulesClass_Instance + 0x1528` = HarvesterDumpRate (double, default 0.016 min/bale)
- `_DAT_007e27f8` = 900.0 (constant, confirmed at address `0x007E27F8`)
- `param_1[0x3e]` = byte offset 0xF8 = dump tick counter (int)
- Formula: `HarvesterDumpRate * 900.0 = 0.016 * 900.0 = 14.4 frames/bale`
- **The counter (`param_1[0x3e]`) increments once per call.** Since `Mission_Dispatch` calls the handler with timer set to the return value (return 1 = called each frame), the counter increments every frame. The comparison fires at ≥14.4 frames, i.e., bale dump every **14–15 frames** (integer comparison of double >= int).
- After each successful dump, counter is reset to 0: `param_1[0x3e] = 0`.
- On init (byte 0x6D1 transition 0→1): `param_1[0x3e] = 0` explicitly.

**State 3 always returns 1** (call next frame, verified — the only return in state 3 is `return 1` at the bottom of the `if (this_00 != NULL)` block).

---

## 5. Dock Anim Slot Assignments Per State

Verified from decompile (all calls are `BuildingClass__SetAnimSlotImage(slot, isDamaged, ...)`):

| Slot | When Set | Condition | Source |
|------|----------|-----------|--------|
| **7** | Dock-init (byte 0x6D1: 0→1) | Harvester only (`UnitTypeClass+0xE0E` set); building found in adjacent cell | `SetAnimSlotImage(7, isDamaged, 0)` |
| **10** | State 3, per-bale dump fires, if `building->field_0x584 == 0` | Active production anim slot | `SetAnimSlotImage(10, isDamaged, 0, 0)` |
| **8** | State 3→4 transition (storage empty) | Only if `BuildingTypeClass+0x16BB` (Refinery) is set | `SetAnimSlotImage(8, isDamaged, 0, 0)` |
| **10, 11** | Cleared by `BuildingClass__ReleaseDockedHarvester` | At post-dump exit | `ClearAnimSlot(10)`, `ClearAnimSlot(11)` |
| **12, 13** | Set by `BuildingClass__ReleaseDockedHarvester` | Post-dump idle visuals from `Type+0x127C/0x128C` and `+0x12C0/0x12D0` | `CreateAnimForSlot(12/13)` |

Slot 7 = dock OPEN door (approach). Slot 10 = active dump anim. Slot 8 = dock CLOSE door (dump finished). Slots 12/13 = post-release idle visuals.

---

## 6. ReleaseDockedHarvester Transition (State 4 → Harvest)

**Verified from `BuildingClass__ReleaseDockedHarvester` @ `0x004595C0` decompile:**

Called from the DOCKED branch (`param_1[0xb9] != 0`) at `0x0073D672` before falling to `LAB_0073d672`. The function:

1. Clears anim slots 10 and 11 (`BuildingClass__ClearAnimSlot`)
2. Plays VOC at building location if `RulesClass+0x244 != -1`
3. Creates idle anim slots 12 and 13 from `BuildingTypeClass+0x127C/0x128C` and `+0x12C0/0x12D0`
4. Gets `piVar1 = building->field_0x2E4` (DockedUnit pointer)
5. Clears `piVar1[0xB9] = 0` (unit's alt dock link)
6. Calls `piVar1->locomotor->Power_On` (loco slot +0x58)
7. Calls `loco->Force_Track(0x47, exit_X - 0x80, exit_Y + 0x80, exit_Z)` (ESE exit)
8. Calls `piVar1->SetSpeed(1.0)` (vtable+0x544)
9. Finds queue anchor cell = building_cell + (-1, +1), then `FootClass::Find_Nearby_Passable_Cell`
10. Calls `piVar1->Set_Destination(dest_cell, 1)` (vtable+0x480)
11. Calls `piVar1->SetMission(MOVE=2, 0)` (vtable+0x1E8) — overrides any intermediate mission
12. Clears `building->field_0x2E4 = 0` and `field_0x718 = 0`
13. Calls building's `SetMission(Guard=5)` and `ScanForTargets(3)`

**Exit trigger:** `ReleaseDockedHarvester` is called when `param_1[0xb9] != 0` at function entry — i.e., the harvester is still marked docked. The harvester arrives here after state 4 sets `byte 0x6D1 = 0` and then the next call finds DockedTo still set. The callee clears the dock link (`piVar1[0xB9] = 0`, `building->field_0x2E4 = 0`) as part of the release.

---

## 7. Chrono Miner (Teleporter) Branch — NONE

**Finding:** `Mission_Deploy_Building` at `0x0073D630` contains **no check for `UnitTypeClass+0xCD4` (Teleporter flag)** anywhere in its 3966 bytes. The live decompile was fully reviewed — no conditional on Teleporter, no chrono-miner-specific branch.

The Teleporter distinction surfaces inside `BuildingClass__ReleaseDockedHarvester` (step 10 above): when `piVar1->Set_Destination` is called, the locomotor's `IPiggyback::Is_Ok_To_End` check runs inside `Set_Destination`. For a chrono miner (TeleportLocomotor piggy-backed on DriveLocomotor), this check fails because `Force_Track` in step 7 leaves `Is_Moving = true`. The loco falls back to drive mode (calls `loco->Stop_Moving`, then `loco->Set_Destination`). The chrono miner thus DRIVES out rather than teleporting, which is the correct observed behavior.

**Chrono miner behavioral difference:** Driven by the locomotor stack, not by any branch in `Mission_Deploy_Building` itself.

**Active in YR:** Yes — the absence of a branch is itself the finding. Chrono miners use the identical code path.

---

## 8. Edge Cases Verified

### Storage Empty on Entry
If `UnitTypeClass+0x5E0 (Storage) < 1` at top-level entry: `CanPassiveAcquire` and `!IsSimpleDeployer` checked. If both: set destination to own cell + random Harvest approach → `return random(0,2) + 14`. Otherwise → `goto LAB_0073d672` → hits Harvester path → `PathType__Has_Valid_Steps` check → ForceScatter + abort.

### Refinery Destroyed Mid-Dump (State 3, building lookup returns NULL)
```c
if (this_00 == (BuildingClass *)0x0) {
    if (PathType__Has_Valid_Steps()) ScanForTargets(3);
    SetMission(Harvest=10, 1);
    // returns timer from MissionClass__GetMissionTimerEntry
}
```
Harvester gracefully falls back to Harvest mission. No crash.

### Player Issues Order Mid-Dump (slave miner forced-undock check in state 3)
Checked at `LAB_0073e539` (after every dump cycle):
```c
if (param_1[0x169] != 0 && param_1[0x2d] != -1 && param_1[0x2d] != 10) {
    // slave miner with an overriding order → force-close door, set state 4
    if (Refinery) SetAnimSlot(8, ...)
    param_1[0x2f] = 4
    if (field_0x584) ClearAnimSlot(building)
}
```
For normal (non-slave) harvesters: no mid-dump interrupt check — player orders cannot abort the dump in state 3; they queue.

### Dock Door Not Finished (State 4, Harvester)
```c
if (building != NULL && building->Type[0x16bb] (Refinery) && building+0x57c != 0) {
    return 1;  // wait for door anim to complete
}
```
Harvester waits in state 4 until `building+0x57C == 0` (dock door anim pointer cleared). Weeder skips this check entirely.

---

## 5 Load-Bearing Verified Facts

1. **Mission code 13 routes to Mission_Deploy_Building.** UnitClass vtable base = `0x007F5C74` (GetCLSID at +0x08). vtable+0x238 = `0x007F5EAC` contains `0x0073D630`. Confirmed via `read_memory(0x007F5EAC)` and `MissionClass__Mission_Dispatch` case 0xD.

2. **Dump timer: `HarvesterDumpRate * 900.0 <= param_1[0x3E]`.** Verified at address ~`0x0073E1B0` in decompile: `*(double *)(g_RulesClass_Instance + 0x1528) * _DAT_007e27f8 <= (double)param_1[0x3e]`. Result: one bale per 14–15 frames. Counter resets to 0 after each bale.

3. **Dock anim slots: 7=OPEN (init), 10=ACTIVE (per-bale), 8=CLOSE (empty), 12/13=POST-RELEASE.** Slots 10 and 11 cleared in `ReleaseDockedHarvester` @ `0x004595C0`; slots 12/13 created from `BuildingTypeClass+0x127C/0x128C` and `+0x12C0/0x12D0`. Slot 8 gated on `BuildingTypeClass+0x16BB` (Refinery).

4. **ReleaseDockedHarvester called from DOCKED branch (DockedTo != 0), NOT from state 4 directly.** State 4 (Harvester) waits for door-close anim, clears `byte 0x6D1`, then calls `SetMission(Harvest=10)`. The actual dock-link teardown (piVar1[0xB9]=0, building->field_0x2E4=0) happens in `ReleaseDockedHarvester` called at the top of the NEXT invocation when DockedTo is still set. Verified by reading both the state-4 and docked-branch code paths.

5. **No Teleporter branch in Mission_Deploy_Building.** Full decompile reviewed — zero references to `UnitTypeClass+0xCD4`. Chrono miner dock-exit behavior (drive vs teleport) is determined by the locomotor stack inside `Set_Destination` (vtable+0x480), not by any conditional in this function.

---

## Status: COMPLETE

Extends `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` with:
- Confirmed mission dispatch binding (mission 13, vtable+0x238, UnitClass base `0x007F5C74`)
- Complete per-state transition table for all four harvester paths (approach states 0/1/3/4 and dump states init/3/4)
- Confirmed per-frame timer path and 14.4 frames/bale derivation
- Confirmed dock anim slot assignments with slot 12/13 from ReleaseDockedHarvester
- Confirmed ReleaseDockedHarvester exit sequence (10-step verified)
- Confirmed zero Teleporter branches; chrono miner difference is locomotor-side
- All four edge cases verified from decompile
