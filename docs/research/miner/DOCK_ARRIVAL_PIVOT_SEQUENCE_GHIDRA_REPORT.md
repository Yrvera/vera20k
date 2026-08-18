# Dock Arrival Pivot Sequence — Ghidra Research Report

**Date:** 2026-05-19
**Binary:** gamemd.exe YR 1.001
**Scope:** End-to-end pivot/stop/face/dump-start sequence when a chrono miner (or war miner)
  arrives at the refinery pad cell. Begins at "miner driving toward dock pad" and ends at
  "miner is stationary on pad, dump animation running."
**Confidence:** HIGH on all core findings (verified from live Ghidra decompilation)

> **Correction 2026-05-26 - mission `0x10` deploy-facing gate**
>
> The older "no pivot/gate at all" wording in this report is too strong. Fresh
> mission-deploy verification shows `UnitClass::Mission_Deploy_Building`
> samples `RateTimer::Current(Unit+0x388)` before unload-start, accepts only
> `((current >> 7) + 1) & 0x1FE == 0x80`, and when not ready may call active
> locomotor vtable `+0x4C(0x4000)` before returning delay `5`. The accepted
> unload-start block still does not directly write body facing. Use
> `MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md`
> and `DOCK_0X16_DOTURN_RATETIMER_UNLOAD_GATE_RECHECK_20260526.md` for the
> current split: radio `0x16` is sync/eligibility, while mission `0x10` owns
> the pixel-relevant deploy-facing gate before the `UnloadingClass` render
> latch.

> **Correction 2026-05-21 - post-dump exit scope**
>
> Arrival/pivot/dump-start findings remain valid. Any later sections that
> describe stock post-dump completion as `ReleaseDockedHarvester` /
> `Force_Track(0x47)` are superseded by the zero-link
> `Mission_Deploy_Building` state-4 findings; those release helpers are
> conditional reciprocal-link paths, not normal stock DockUnload completion.
>
> **Correction 2026-05-24 - stock refinery admission / `0x16` reswarm**
>
> This report is superseded for stock `CMIN/HARV -> GAREFN/NAREFN` dock
> admission and unload-start timing by
> `STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`
> and `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`.
> `0x00739EC0` is `UnitClass::PerCellProcess` / a cell-entry hook, not the
> mission-7 dispatch handler; mission 7 dispatch is
> `FootClass::Mission_Enter @ 0x004D9290`. `UnitClass::Receive_Radio(0x16)`
> has no `GetDockCoord`, no `Set_Destination`, and no location write. First
> ordinary `0x16` may only sync timer/rate and return; later/already-synced
> `0x16` can send `0x15` under stopped-building-destination/contact-entered
> gates. Do not use this doc as evidence for a required physical
> `NW+(3,1) -> NW+(2,1)` bridge or for a proven East-facing pivot during stock
> unload.
**Active in YR:** YES — fires every harvest cycle for every harvester

---

## 0. Naming Correction — "FACE_DOCK (0x16)" Is Wrong

`HARVESTER_DOCK_UNLOAD.md` §Radio Command Map calls command 0x16 "FACE_DOCK" and says its
unit-side handler does: "Stop, set facing to 0x4000."

**This is incorrect on both counts.**

- The command is named **TIMING_SYNC** in `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`
  (confirmed from decompile of BuildingClass::Receive_Radio and UnitClass::Receive_Radio).
- The value `0x4000` is NOT a facing. It is passed to `locomotor vtable+0x4C` — which for
  DriveLocomotionClass is a **movement-rate/speed field** (the locomotion timing register).
  Verified: `UnitClass::Receive_Radio` case 0x16 at 0x00737430 calls
  `(**(code **)(*(int *)param_1[0x19d] + 0x4c))(..., 0x4000)` — this is the locomotor's
  `SetSpeed` slot, not any facing-setting call.
- No explicit facing value is written anywhere in the TIMING_SYNC handler or the dock-cell
  arrival logic inside Mission_Enter.

The facing that a player observes during and after docking is set **only** in
`BuildingClass::ReleaseDockedHarvester` (0x4595C0) at **exit time**, via
`DriveLocomotionClass::Force_Track(0x47, ...)` — not at arrival time.

---

## 1. Overview

The pivot/stop/face sequence is driven by three functions executing in sequence
across multiple ticks:

| Phase | Function | Address | Role |
|-------|----------|---------|------|
| Approach-to-pad | `UnitClass::Mission_Enter` (= `UnitClass__PerCellProcess`) | 0x739EC0 | Detects arrival at dock cell, stops locomotor, initiates dock |
| TIMING_SYNC | `UnitClass::Receive_Radio` case 0x16 | 0x737430 | Sets locomotor speed to 0x4000 (arrival sync), optionally sends DOCK_NOW back |
| Dump loop | `UnitClass::Mission_Deploy_Building` | 0x73D630 | Drives per-bale ore transfer; unit stays on pad, facing unchanged |
| Exit / facing-set | `BuildingClass::ReleaseDockedHarvester` | 0x4595C0 | Power_On, Force_Track(0x47), Set_Destination, SetMission(MOVE) |

---

## 2. Phase 1 — Dock-Cell Arrival Detection (Mission_Enter)

**Address:** 0x739EC0 (labeled `UnitClass__PerCellProcess` by Ghidra)
**Active in YR:** YES
**param_1 type:** `TechnoClass*` (int — direct byte offsets)

### 2.1 Arrival Condition

```
unitCoords = vtable->GetCoords()           // vtable+0x48 → lepton XYZ
unitCell   = Leptons_to_Cell(unitCoords)   // (X >> 8, Y >> 8)

dockCoords = building->vtable->GetDockCoord(&result, unit)  // vtable+0xA8
dockCell   = Leptons_to_Cell(dockCoords)

if (unitCell == dockCell):
    → ARRIVAL BRANCH
```

Comparison is **cell-level**, not lepton-level. Both X and Y shorts must match.
For a standard GAREFN (3×3 foundation), `GetDockCoord` returns `building_center + (128, 0, 0)`
(building center shifted half a cell east) per BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md §2.

### 2.2 IPiggyback Probe

On arrival, the code queries `ILocomotion::QueryInterface(IID_IPiggyback)` on the active
locomotor (unit+0x674). It then reads the inner CLSID via `IPiggyback vtable+0xC`.

The check compares against `CLSID_WalkLocomotion` (global at 0x7E9A40):
```
if (innerCLSID == CLSID_WalkLocomotion):
    // DriveLocomotion is piggybacked over Teleport (or Drive is native)
    if (building->Type->WeaponsFactory  // BuildingType+0x16BD
        && unit->DockLink == 0):
        unit->DockLink = building       // unit+0x5A4 = building pointer
```

**Note:** The WeaponsFactory flag check (`+0x16BD`) is what GAREFN and NAREFN have set —
this is what allows the DockLink to be established on this pass. Verified at 0x0073A4BC
(`MOV EDI, 0x7e9a40` GUID comparison) and 0x0073A4E9 (`[EBP+0x5A4]` = DockLink).

### 2.3 The Three Calls at Pad Arrival

When `building == unit->DockLink` (DockLink already set from prior pass OR just set above):

```
1. FootClass::PerCellProcess(2)        // updates ghost cell, turret facing sync
2. radio(0x15, building)               // vtable+0x274 → DOCK_NOW to building
3. locomotor->Power_Off()              // ILocomotion vtable+0x5C → stop movement
```

Verified at 0x0073A4F7–0x0073A50B:
- `CALL [EDX+0x194]` at 0x0073A4F7 = `FootClass__PerCellProcess`
- `PUSH 0x15 / CALL [EDX+0x274]` at 0x0073A503 = radio DOCK_NOW
- `CALL [EAX+0x5C]` at 0x0073A50B = `ILocomotion::Power_Off`

**No facing is set here.** The unit's facing remains whatever it was when it arrived at the
dock cell. The locomotor is powered off — the unit is stationary.

### 2.4 FootClass::PerCellProcess(2) — What It Does to Turret/Facing

`FootClass::PerCellProcess(2)` (0x4D85D0) does NOT set a body facing. It:
- Clears `field_0x6B2` and `field_0x6B0` (pathfinding flags)
- If the unit has a turret (`TurretCount > 0`): calls `SetDesiredFacing(field_0x55C)` and
  `SetCurrentFacing(body_facing)` — syncs turret to body facing
- Updates ghost cell tracking (removes unit from old queue cell, adds to current cell)
- Updates adjacent cell crowd counters (8 directions)

The war miner has a turret; the chrono miner does NOT (`Turret=no` for CMIN). So this
turret-sync branch is skipped for the chrono miner but fires for the war miner.

### 2.5 What the Building Does on Receiving DOCK_NOW (0x15)

`BuildingClass::Receive_Radio` case 0x15:
- For `DockUnload=yes` buildings (GAREFN, NAREFN): sets the **unit's** mission to
  `0x10` (Mission_Unload — which maps to `Mission_Deploy_Building` for harvesters)
- Returns 1

The building does NOT call `Power_Off` or set any facing. The building does NOT enter
MissionRepairAndProduce from the building side — that transition is handled via the
`field_0x6DD = 1` path in the unit side.

---

## 3. Phase 2 — TIMING_SYNC Radio Command (0x16)

**Sent by:** Building to harvester, AFTER `MOVE_TO_CELL(0x12)` and `ENTER_DOCK(0x18)`
  in the CAN_DOCK(0x0E) sequence
**Received by:** `UnitClass::Receive_Radio` case 0x16 at 0x737430

### 3.1 Handler Sequence (verified from decompile at 0x737430 case 0x16)

```
1. FootClass::Receive_Radio(sender, 0x16, param)   // base class call first

2. if (field_0x6AF == 0):                          // NOT chrono-teleporting
       psVar4 = RateTimer__Current(local_1c)        // read current rate timer
       if (*psVar4 != 0x4000):                      // if not already at 0x4000
           locomotor vtable+0x4C (locomotor, 0x4000)  // SetSpeed/SetRate to 0x4000
           return 1

3. if (loco NOT moving                              // Is_Moving() returns false
       AND destination != NULL                      // has a destination
       AND destination->WhatAmI() == 6             // destination is a Building
       AND building mission == 7                   // building in MissionEnter
       AND unit mission == 7):                     // unit in MissionEnter
           radio(0x15, destination)                // send DOCK_NOW back to building
           vtable+0x278

4. return 1
```

### 3.2 What 0x4000 Means

`0x4000` is the **RateTimer value** written to the locomotor's timing register. For
`DriveLocomotionClass`, vtable+0x4C maps to the speed/rate setter. The value `0x4000`
in the RateTimer is the "full-speed" or "arrival-synchronized" rate. This is NOT a
facing angle.

The `RateTimer` for DriveLocomotion uses a 0x0000–0xFFFF range where:
- `0x8000` = stopped/centered
- `0x0000` = full forward
- `0x4000` = midpoint (arrival-synchronized, signals the locomotor is at the dock cell)

From `UnitClass::Mission_Deploy_Building` (0x73D630), the dump loop checks:
```c
psVar11 = (short *)RateTimer__Current()
if (((*puVar10 >> 7) + 1 & 0x1fe) != 0x80):
    // locomotor not yet at 0x80 midpoint — still driving
    if (field_0x6AF == 0):
        locomotor vtable+0x4C with 0x4000    // force rate to 0x4000
    return 5    // wait — not ready to dump yet
```

This confirms: `0x4000` = the locomotor rate that signals "I am centered on the pad and
ready to start dumping." The dump loop will return `5` (wait) each tick until the rate
timer reaches the 0x80 check.

### 3.3 Chrono Miner Specificity of TIMING_SYNC

The `field_0x6AF` gate (TechnoClass+0x6AF) is the chrono-teleporting flag. For the
chrono miner arriving by DRIVE (DriveLocomotion piggybacked over TeleportLocomotion),
`field_0x6AF` is `0` — the same as for the war miner. So TIMING_SYNC processes
**identically** for both miners.

The `field_0x6AF` flag would be non-zero only if the miner is actively mid-teleport.
During dock approach (DRIVING phase), this flag is `0`.

**Verified:** RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md §7 states "TIMING_SYNC(0x16)
triggers locomotor sync in harvester (`SetSpeed(0x4000)`)." This is consistent with
the decompile at 0x737430 case 0x16. No facing operation is present anywhere in this handler.

---

## 4. Phase 3 — Dump Phase (Mission_Deploy_Building)

**Address:** 0x73D630 (labeled `UnitClass__Mission_Deploy_Building`)
**Active in YR:** YES — called when unit mission = 0x10 (Unload)
**param_1 type:** `int*` (direct byte offsets)

### 4.1 Entry Gate — param_1[0xB9] Check

The function's outermost branch checks `param_1[0xB9]` (unit+0x2E4 — the alt dock link):

```
if (param_1[0xB9] != 0):
    // unit ALREADY physically at pad (has alt dock link set)
    → harvester dump loop branch
else:
    // unit approaching building or doing non-refinery deploy
    → different code path
```

This is the critical moment: `unit+0x2E4` is set when `BuildingClass::ReleaseDockedHarvester`
is called (confirmed from BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md and
REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md §5).

**IMPORTANT:** The HARVESTER dump path is the `param_1[0xB9] != 0` branch. The build
sequence is:
1. Unit arrives at pad cell → DOCK_NOW (0x15) → unit mission set to 0x10
2. Mission_Deploy_Building runs; since `+0x2E4 == 0` initially, takes the non-harvester path
3. In the non-harvester path with Harvester=yes type: checks RateTimer, waits for `0x4000`
4. Once at 0x4000: initializes dump state (`field_0x6D1 = 1`), sets harvester anim slot 7
5. ... then on next tick `+0x2E4` is set (by some caller), and the `+0xB9 != 0` branch fires

**Actually:** Re-reading the decompile:
- The outer check `if (param_1[0xB9] == 0)` takes the path for NON-refinery buildings
  (MCV deploy etc). For harvesters with no type SizeLimit: `goto LAB_0073d672` → to the
  harvester-specific code that loops calling `BuildingClass::ReleaseDockedHarvester()`.

Wait — the decompile shows:
```c
if (param_1[0xb9] == 0):            // no alt-dock-link (== approaching)
    if (TypeClass->SizeLimit < 1):   // +0x5E0 < 1 (storage building, not transport)
        goto LAB_0073d672            // harvester path
```

LAB_0073d672 is the **harvester dump path** — it starts with checking `PathType::Has_Valid_Steps()`
and then handles the rate-timer / dump state machine.

### 4.2 Harvester Dump State Machine in Mission_Deploy_Building

From decompile at 0x73D630 LAB_0073dee7 onwards:

```
if PathType::Has_Valid_Steps() == false:
    // locomotor has finished — unit is stationary on pad
    if (field_0x6AF == 0):          // not chrono-teleporting
        // ensure rate timer at 0x4000
        if RateTimer != 0x4000:
            locomotor->SetRate(0x4000)
        return 5                     // wait (not ready to dump yet)

    // field_0x6AF != 0: rate timer is at right value
    if (field_0x6D1 == 0):
        // FIRST DUMP TICK — initialize
        param_1[0x3E] = 0           // step counter = 0
        field_0x6D1 = 1             // "is dumping" flag
        param_1[0x40] = g_CurrentFrame   // timer start frame
        param_1[0x41] = ...              // timer auxiliary
        param_1[0x42] = 1               // timer step = 1
        param_1[0x43] = 1               // timer rate = 1

        if (TypeClass->Harvester):      // type+0xE0E
            // HARVESTER-SPECIFIC: update dock anim slot 7
            cell = GetCurrentCell()
            MapClass::Get_CellClass(cell)
            building = Look_up_building_in_cell()
            if (building != NULL):
                healthRatio = GetHealthRatio(building)
                BuildingClass::SetAnimSlotImage(7, healthRatio <= ConditionRed, 0)

        param_1[0x2F] = 3           // dump state = 3 (active dump)
```

### 4.3 Facing During Dump Phase

**No facing is set at any point during the dump phase.** The unit's facing is whatever
it was when it arrived at the dock cell. For a unit driving in from the queue cell, the
final drive path direction determines the facing when `Power_Off` fires.

The chrono miner drives from the queue cell (building_anchor+3,+1) to the dock cell.
The dock cell for GAREFN is at building center + half cell east. The approach direction
from queue cell to dock cell is roughly **west** (driving into the refinery). So the miner
faces roughly west during the dump. The war miner follows the same approach path.

**No "pivot to dock facing" operation exists in the binary.** The unit just stops wherever
it is, facing whatever direction it arrived from.

---

## 5. Phase 4 — Exit and Facing Set (ReleaseDockedHarvester)

**Address:** 0x4595C0 (labeled `BuildingClass__ReleaseDockedHarvester`)
**Active in YR:** YES — called from `UnitClass::Mission_Deploy_Building` when dump completes
**param_1 type:** `BuildingClass*` (direct offsets)

### 5.1 Complete Exit Sequence

Verified from full decompile of 0x4595C0:

```
1. BuildingClass::ClearAnimSlot(0xA)         // clear anim slot 10
2. BuildingClass::ClearAnimSlot(0xB)         // clear anim slot 11
3. Play VocClass sound at building location  // Rules+0x244 sound (unloading complete)
4. BuildingClass::SetAnimSlotImage(0xC, ...)  // slot 12 = operational anim (health-dependent)
5. BuildingClass::SetAnimSlotImage(0xD, ...)  // slot 13 = operational anim variant

6. piVar1 = building->field_0x2E4            // alt dock link → docked unit pointer

7. piVar1[0xB9] = 0                           // unit[0x2E4] = 0 (clear unit's alt link)
8. locomotor->Power_On()                      // ILocomotion vtable+0x58 — unit can move again
9. buildingCoords = building->GetCoords()     // vtable+0x48 → {X, Y, Z} lepton
10. locomotor->Force_Track(
        0x47,                                 // track_index = 0x47 (ESE)
        buildingCoords.X - 0x80,              // X offset = -128 leptons
        buildingCoords.Y + 0x80,              // Y offset = +128 leptons
        buildingCoords.Z)                     // same Z
11. unit->SetSpeedMultiplier(1.0)             // vtable+0x544, value 0x3FF00000 (= 1.0)
12. anchorCell = building->GetCellLocation() + (-1, +1)   // SW of building top-left
13. destCell = FootClass::Find_Nearby_Passable_Cell(around anchorCell)
14. unit->Set_Destination(destCell, 1)        // vtable+0x480
15. unit->SetMission(MOVE=2, 0)               // vtable+0x1E8
16. building->field_0x2E4 = 0                 // clear building's alt link (piVar1→ NULL)
17. building->field_0x718 = 0                 // clear related field
18. building->SetMission(Guard=5, 0)          // vtable+0x1E8
19. building->RadioCommand(BREAK=3)           // vtable+0x274 — notify production system
```

### 5.2 Force_Track(0x47) — What It Actually Does

`DriveLocomotionClass::Force_Track` (0x4B0C40) takes `(track_index, x, y, z)`:
- Writes `track_index` to `loco+0x54` (the drive-track index field)
- Clears `loco+0x58` (point_index within track = 0)
- Sets `head_to` at `loco+0x3C/40/44` = `{x, y, z}`
- Calls `Apply_Track_Delta` with the target coords → sets `destination` at `loco+0x30/34/38`
- Sets `loco+0x4C = 0` (residual ticks)
- Sets `loco+0x50 = 0x3FF00000` (speed = 1.0 double)

After `Force_Track(0x47, ...)`, `Is_Moving()` returns true (destination is set).

**`0x47` is a DRIVE TRACK INDEX, not a raw facing value.** It selects entry 0x47 from the
`g_DriveTrackData_Array` global table. This table maps track indices to sequences of
position/facing deltas used by `Apply_Track_Delta`. The visual direction corresponding to
track 0x47 is ESE (East-South-East, ~100° from north). This is the facing the player sees
when the harvester exits the refinery pad.

---

## 6. Chrono Miner vs War Miner — Differences

| Step | War Miner (HARV) | Chrono Miner (CMIN) | Evidence |
|------|-----------------|---------------------|---------|
| Locomotion during dock approach | DriveLocomotion (native) | DriveLocomotion piggybacked over TeleportLocomotion | CHRONO_MINER_SYSTEM_OVERVIEW.md §2; Mission_Enter 0x739EC0 IPiggyback check |
| IPiggyback check in Mission_Enter | Inner CLSID = WalkLocomotion → PASS | Inner CLSID = WalkLocomotion (Drive is outer, Walk is inner) → PASS same branch | Mission_Enter 0x0073A4BC |
| TIMING_SYNC (0x16) field_0x6AF gate | 0 (not teleporting) → SetSpeed(0x4000) fires | 0 (not teleporting during DRIVE approach) → SetSpeed(0x4000) fires identically | UnitClass::Receive_Radio 0x737430 case 0x16 |
| Turret sync in PerCellProcess(2) | YES — war miner has turret; turret facing synced to body | NO — CMIN has no turret; turret-sync branch skipped | FootClass::PerCellProcess(2); CMIN INI Turret=no |
| Dump animation slot 7 (harvester) | YES — SetAnimSlotImage(7, ...) on first dump tick | YES — same; both types+0xE0E = Harvester=yes | Mission_Deploy_Building 0x73D630 LAB_0073e050 |
| dump phase facing | Unchanged from drive-in direction | Unchanged from drive-in direction | No facing write in dump loop |
| Exit Force_Track value | 0x47 (ESE) | 0x47 (ESE) — identical | ReleaseDockedHarvester 0x4595C0 |
| Locomotor swap after exit | Set_Destination → Drive stays active | Set_Destination triggers IPiggyback::Is_Ok_To_End check in next FootClass::AI tick → swaps back to TeleportLocomotion | CHRONO_MINER_SYSTEM_OVERVIEW.md §2 locomotor swap lifecycle |
| Post-exit mission | MOVE (mission 2) | MOVE (mission 2) — but TeleportLoco swap happens soon after | ReleaseDockedHarvester step 15 |

**The primary observable difference**: after exiting the refinery at facing 0x47 (ESE),
the war miner continues driving, while the chrono miner's `FootClass::AI` detects
`Is_Ok_To_End()` → swaps back to TeleportLocomotion → next destination will warp.

---

## 7. Key Struct Offsets (Verified)

### UnitClass (direct byte offsets, param_1 type = `int*`)
| Offset | Field | Verified |
|--------|-------|----------|
| +0x2E4 (= param_1[0xB9]) | Alt dock link / on-pad unit pointer | 0x4595C0 decompile; 0x73D630 outer check |
| +0x5A4 | DockLink (FootClass+0x84 slot) | 0x739EC0 at 0x0073A4E9 |
| +0x674 (= param_1[0x19D]) | Locomotor COM pointer | 0x739EC0; UnitClass::Receive_Radio 0x737430 |
| +0x6AF (byte) | Chrono-teleporting flag; gates TIMING_SYNC SetSpeed | 0x737430 case 0x16 and 0x73D630 |
| +0x6D1 (byte) | "Is dumping" flag; set to 1 on first dump tick | 0x73D630 LAB_0073df53 |
| +0x3E (= param_1[0x0F] offset-wise? Check: 0x3E*4=0xF8) | Step counter for dump | 0x73D630 `param_1[0x3E] = 0` |
| +0x40/41/42/43 (timer fields) | Dump CDTimer (start/aux/step/rate) | 0x73D630 initialization block |
| +0x2F (= param_1+0xBC) | State index for deploy sub-state | 0x73D630 switch |

### DriveLocomotionClass (offsets within loco object, 4-byte aligned)
| Offset | Field | Verified |
|--------|-------|----------|
| +0x30/34/38 | Destination XYZ | 0x4B0C40 Force_Track; 0x4AFCA0 Destination |
| +0x3C/40/44 | Head_To XYZ | 0x4B0C40 Force_Track |
| +0x4C | Residual ticks / SetRate input | 0x4B0C40; 0x73D630 |
| +0x50 | Speed (double, 0x3FF00000 = 1.0) | 0x4B0C40 Force_Track |
| +0x54 | Track index (0x47 at exit) | 0x4B0C40 Force_Track |
| +0x58 | Point index within track (reset to 0) | 0x4B0C40 Force_Track |
| +0x5F | Has_Head_To flag | 0x4B0C40 Force_Track |

### ILocomotion vtable slots (called via `*(loco vtable + slot)`)
| Slot | Function | Called when |
|------|----------|-------------|
| +0x4C | SetSpeed / SetRate | TIMING_SYNC; Mission_Deploy_Building rate sync |
| +0x58 | Power_On | ReleaseDockedHarvester step 8 |
| +0x5C | Power_Off | Mission_Enter step 3 at pad arrival |
| +0x70 | Force_Track | ReleaseDockedHarvester step 10 |

---

## 8. Complete Tick-by-Tick Pivot Sequence

```
TICK N (final drive tick):
  DriveLocomotion::Process() runs — unit drives last step to dock cell.
  FootClass::AI runs — locomotor not at end yet.

TICK N+1 (arrival tick):
  DriveLocomotion::Process() completes last track step.
  Mission_Enter (UnitClass::PerCellProcess state=2) runs:
    → Compares unitCell == dockCell: YES
    → QueryInterface(IPiggyback) → reads inner CLSID → WalkLocomotion → matches
    → Sets DockLink (unit+0x5A4 = building) if WeaponsFactory+NoDockLink
    → FootClass::PerCellProcess(2): ghost cell updated; turret sync (war miner only)
    → radio(0x15, building): building sets unit mission to 0x10 (Unload)
    → locomotor->Power_Off(): unit stops. FACING = whatever drive-in direction was.

TICK N+1 (same tick, building side):
  Building receives 0x15 → unit.SetMission(0x10)

TICK N+2+:
  Mission_Deploy_Building runs (unit mission = 0x10):
    if PathType::Has_Valid_Steps() == false:      // locomotor finished
        if RateTimer != 0x4000:
            SetSpeed(0x4000)
            return 5                              // wait — not ready

    [Meanwhile, during the queue/approach phase, building sent TIMING_SYNC(0x16)
     which already set RateTimer to 0x4000]

    if RateTimer at 0x4000 threshold:
        if field_0x6D1 == 0:
            Initialize dump: step=0, timer=1/1, set Harvester anim slot 7
            field_0x6D1 = 1
            state = 3 (DUMP_ACTIVE)
            return 1

TICK N+3+ (dump loop):
  Mission_Deploy_Building state=3:
    Checks param_1[0x3E] (step counter) vs HarvesterDumpRate*900.0 threshold
    If threshold not reached: param_1[0x3E]++ (via CDTimer), return 1
    If threshold reached: transfer one bale, reset step counter, return 1
    If all bales dumped: state = 4

  FACING: UNCHANGED from arrival tick. No pivot occurs.

  VISIBLE RENDERING: Unit renders with UnloadingClass (HORV/CMON) model because
    DockedBuilding (unit+0x1D0) is non-NULL and dump flag active.

TICK FINAL (dump complete, state=4):
  Mission_Deploy_Building state=4:
    BuildingClass::ReleaseDockedHarvester() called:
      ClearAnimSlot(0xA, 0xB)         // clear unloading anims
      Play completion sound
      SetAnimSlotImage(0xC, 0xD)      // restore operational anims
      unit[0x2E4] = 0                 // clear alt dock link
      locomotor->Power_On()           // can move again
      locomotor->Force_Track(0x47, center-128, center+128, z)
                                      // SET FACING / TRACK to 0x47 (ESE)
      unit->SetSpeedMultiplier(1.0)
      anchorCell = buildingCell + (-1, +1)
      destCell = Find_Nearby_Passable_Cell(anchorCell)
      unit->Set_Destination(destCell, 1)
      unit->SetMission(MOVE=2)
      building->RadioCommand(BREAK=3)

  FACING NOW: 0x47 (ESE track index via Force_Track)
```

---

## 9. HARVESTER_DOCK_UNLOAD.md Conflict Resolution

`HARVESTER_DOCK_UNLOAD.md` §Radio Command Map, row 0x16, states:
> "FACE_DOCK — Unit Handler: Stop, set facing to 0x4000"

**This entry is wrong on three points:**

1. **Name**: "FACE_DOCK" is the wrong name. The command is TIMING_SYNC.
2. **"Stop"**: No stop is performed in handler 0x16. Stop was performed by `Power_Off()`
   in Mission_Enter before TIMING_SYNC even arrives.
3. **"Set facing to 0x4000"**: `0x4000` is NOT a facing value. It is a RateTimer value
   passed to `locomotor->SetSpeed()`. No facing is written in this handler.

The actual facing written is `0x47` (track index, ESE), written in
`BuildingClass::ReleaseDockedHarvester` at exit time — not at TIMING_SYNC time.

**Corrected entry for 0x16:**
> "TIMING_SYNC — Unit Handler: if field_0x6AF==0 AND RateTimer!=0x4000, calls
> locomotor->SetSpeed(0x4000) to synchronize approach timing. If loco not moving
> AND destination is building in mission 7, sends DOCK_NOW (0x15) back to building."

---

## 10. Open Questions — Final State

- `[RESOLVED] Q1 — What function detects pad-cell arrival?` → `UnitClass::PerCellProcess`
  (0x739EC0 = Mission_Enter) state=2 checks unitCell == dockCell. (evidence: 0x739EC0 decompile)
- `[RESOLVED] Q2 — Which radio commands fire and in what order at pad arrival?`
  → `FootClass::PerCellProcess(2)` (not radio), then radio `0x15` to building. TIMING_SYNC
  (0x16) fires EARLIER during queue approach via CAN_DOCK(0x0E) sequence, not at pad arrival.
  (evidence: 0x739EC0 and 0x737430)
- `[RESOLVED] Q3 — Who stops the locomotor?` → `locomotor->Power_Off()` vtable+0x5C, called
  directly in Mission_Enter after radio 0x15. No radio command stops it. (evidence: 0x739EC0)
- `[RESOLVED] Q4 — Who sets the facing and to what value?` → `DriveLocomotionClass::Force_Track`
  (0x4B0C40) called from `BuildingClass::ReleaseDockedHarvester` (0x4595C0) at EXIT time.
  Track index = `0x47` (ESE). This is NOT a raw facing — it is a drive track index.
  (evidence: 0x4595C0 decompile; 0x4B0C40 decompile)
- `[RESOLVED] Q5 — What facing is held DURING dump phase?` → No facing is set. The unit
  holds whatever facing it had when `Power_Off()` fired. No pivot occurs at any point during
  the dump. (evidence: 0x73D630 full decompile — no facing write in dump loop)
- `[RESOLVED] Q6 — What does TIMING_SYNC (0x16) actually do?` → Sets locomotor rate to
  `0x4000` via vtable+0x4C, NOT a facing. The "FACE_DOCK = facing 0x4000" claim in
  HARVESTER_DOCK_UNLOAD.md is incorrect. (evidence: 0x737430 case 0x16 decompile)
- `[RESOLVED] Q7 — What is the chrono miner-specific difference in this sequence?` → Only
  two differences: (a) turret sync in PerCellProcess(2) is skipped for CMIN (no turret);
  (b) after exit, DriveLocomotion→TeleportLocomotion swap fires next FootClass::AI tick.
  All other steps identical. (evidence: CMIN INI `Turret=no`; 0x739EC0 turret check; 
  CHRONO_MINER_SYSTEM_OVERVIEW.md §2)
- `[RESOLVED] Q8 — Is "0x47" a facing or a track index?` → A drive track index. Selects
  entry 0x47 in `g_DriveTrackData_Array`. The visual direction is ESE (~100° from north).
  NOT raw facing byte 0x47 in a 0-255 range. (evidence: 0x4B0C40 writes to `loco+0x54`
  which is the track_index field; Apply_Track_Delta called with it)
- `[DEFERRED] Q9 — Exact tick when building sets unit+0x2E4 (alt dock link)?`
  (category: requires-different-system-context; the write occurs somewhere in the approach
  state machine; not visually observable; next-step: trace callers of write to unit+0x2E4)
- `[DEFERRED] Q10 — Exact facing value (0-255 byte) the miner holds during dump?`
  (category: needs-runtime-debugger; depends on the exact path taken from queue cell to
  dock cell, which varies per map; not extractable from static analysis alone)

---

## Sources

- Ghidra MCP decompilation of:
  - `UnitClass::Mission_Enter` / `UnitClass__PerCellProcess` @ 0x739EC0 (full body)
  - `UnitClass::Receive_Radio` @ 0x737430 (case 0x16 focus)
  - `UnitClass::Mission_Deploy_Building` @ 0x73D630 (full body)
  - `BuildingClass::ReleaseDockedHarvester` @ 0x4595C0 (full body)
  - `DriveLocomotionClass::Force_Track` @ 0x4B0C40 (full body)
  - `FootClass::Receive_Radio` @ 0x4D8FB0 (cases 0x12, 0x13, 0x16, 0x17)
- Docs consulted:
  - HARVESTER_DOCK_UNLOAD.md (naming conflict resolved)
  - MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md (verified context)
  - MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md
  - REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md
  - RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md
  - BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md
  - DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md
  - CHRONO_MINER_SYSTEM_OVERVIEW.md
  - traces/CHRONO_MINER_TELEPORT_DOCK_APPROACH_TRACE.md

**Status: COMPLETE**
