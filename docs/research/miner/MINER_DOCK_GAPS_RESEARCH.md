# Miner Dock System — Gap Analysis Research Report

> **Correction 2026-05-21 - stock DockUnload exit**
>
> Any older statements below that call `ReleaseDockedHarvester` the normal
> post-unload exit are superseded. Stock `CMIN/HARV -> GAREFN/NAREFN`
> DockUnload normally exits through the zero-link `Mission_Deploy_Building`
> state-4 path; `ReleaseDockedHarvester` / `Force_Track(0x47)` are conditional
> reciprocal-link release details.

## Overview

This report covers three gaps in the miner/harvester docking documentation:
1. Locomotor piggyback swap-back after undocking
2. Refinery destruction during dock sequence
3. FUN_00500200 — AI harvester wander-when-queued logic

All findings verified from Ghidra decompilation of gamemd.exe.
Confidence: HIGH on all three gaps (with corrections applied 2026-05-19).

---

## CORRECTIONS — 2026-05-19 (re-swarm chrono miner / refinery docking)

The original 2026-04-25 draft of this report contained four propagated misattributions
that the 2026-05-19 swarm caught and refuted with direct Ghidra evidence. Each section
below is annotated inline where corrections were made; this header summarises them.

1. **`FUN_006AF6C0` is `SlaveManagerClass::AI_Update`, not a refinery dock-queue processor.**
   Verified by `get_function_by_address(0x6AF6C0)` (Ghidra label). The five-int per-slave
   entry layout (state, timer) described in this report's original "Case B" is the
   slave-entry layout, not a refinery queue entry. There is no standalone refinery
   dock-queue processor function. See `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md`.

2. **`BuildingClass::UndockUnit (0x4593A0)` is the interrupt/destruction-time handler ONLY.**
   Three callers verified: `BuildingClass::ReceiveDamage` (death case 4),
   `BuildingClass::Sell`, `TemporalClass::Update`. The original draft's "Sequence Summary"
   step 1 ("UndockUnit sends ILocomotion::Head_To") is wrong for the normal post-unload exit.
   **Superseded 2026-05-21:** normal stock post-unload exit is zero-link `Mission_Deploy_Building` state 4; `ReleaseDockedHarvester (0x4595C0)` is conditional nonzero-link release.
   `UnitClass::Mission_Deploy_Building (0x73D630)`. Both functions share the same eject
   sequence (Stop + Head_To track 0x47, ±0x80 lepton offsets, speed 1.0). See
   `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`.

3. **`BuildingClass::DepositOreFromStorage (0x522D50)` is slave-miner-only.**
   Sole xref: `SlaveManagerClass::AI_Update` state 4. The harvester/chrono-miner ore-to-credits
   conversion runs inline inside `Mission_Deploy_Building` (0x73D630), one slot per
   `HarvesterDumpRate` (Rules+0x1528 = 14.4 frames/bale). See
   `DEPOSITOREFROMSTORAGE_0x522D50_CHRONO_MINER_GHIDRA_REPORT.md`.

4. **Case C — refinery destroyed mid-unload — remaining ore stays on the harvester.**
   The original "any partial ore deposit is lost" wording was self-contradictory and
   imprecise. Verified: ore lives in `StorageClass` on the harvester (`UnitClass+0x33C`),
   not on the building. When the building dies, the harvester drives away with its full
   undumped storage; ore is preserved.

The `unit[0xB9] / building[0xB9]` notation throughout this doc refers to **byte offset
`+0x2E4`** (param_1 typed as `int*`, so `0xB9 × 4 = 0x2E4`). The mutual dock-link
cross-reference pointer (building stores unit ptr, unit stores building ptr) lives at
`+0x2E4` on both classes. See `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md`
for the field-life analysis (set at pad-arrival, NOT at HELLO time).

---

## Gap 1: Locomotor Swap-Back After Undocking

### Background

When a Chrono Miner (Teleporter=yes) docks at a refinery, `TechnoClass::Set_Destination`
(0x741970) creates a new `DriveLocomotionClass` via `CoCreateInstance`, queries `IPiggyback`
from it, and calls `IPiggyback::Begin_Piggyback` to stack DriveLocomotion on top of
TeleportLocomotion. The active locomotor at `FootClass+0x674` becomes DriveLocomotion,
which drives the miner to the dock pad.

### When Does the Swap-Back Happen?

**The swap-back is handled automatically by `FootClass::AI` (0x4DA530), near the end
of the function (around offset +0x970 into the function body).**

It does NOT happen during `BuildingClass::UndockUnit` or `UnitClass::Mission_Harvest`.
Those functions operate on the active locomotor (Drive) without ending the piggyback.

### Exact Mechanism (FootClass::AI, lines ~340-365)

Every tick, `FootClass::AI` runs after `ILocomotion::Process`. Near the end:

```
1. Get active locomotor: piVar4 = FootClass+0x674  (ILocomotion*)
2. QueryInterface(piVar4, IID_IPiggyback) → piVar8  (IPiggyback*)
3. If IPiggyback interface exists:
   a. Call IPiggyback::Is_Ok_To_End() [vtable+0x14]
   b. If returns true:
      - Release current active locomotor: piVar4->Release()
      - Clear FootClass+0x674 = 0
      - Call IPiggyback::End_Piggyback() [vtable+0x10]
        (This extracts the piggybacked TeleportLocomotion and
         stores it back into FootClass+0x674)
```

### IPiggyback COM Vtable Layout (DriveLocomotionClass)

The IPiggyback sub-object vtable pointer lives at DriveLocomotionClass+0x18 (param_1[6]).
RTTI pointer is at vtable[-4]. Actual vtable layout:

| Offset | Method                | Address    |
|--------|-----------------------|------------|
| +0x00  | QueryInterface        | 0x004B4DC0 |
| +0x04  | AddRef                | 0x004B4DD0 |
| +0x08  | Release               | 0x004B4DE0 |
| +0x0C  | Begin_Piggyback       | 0x004AF8E0 |
| +0x10  | End_Piggyback         | 0x004AF930 |
| +0x14  | Is_Ok_To_End          | 0x004AF970 |
| +0x18  | Piggybacker_CLSID     | 0x004AF610 |
| +0x1C  | Is_Piggybacking       | 0x004B4CD0 |

### DriveLocomotionClass::Is_Ok_To_End (0x4AF970)

Returns true when ALL conditions met:
1. `Is_Moving_Now()` returns false (locomotor has stopped)
2. Piggybacked locomotor exists (offset +0x50 != 0)
3. Flag at offset +0x4D is set (enabled flag)
4. Owner unit's `+0x6AD` flag is false (unit is not in limbo/special state)

### DriveLocomotionClass::End_Piggyback (0x4AF930)

Extracts the piggybacked locomotor:
- Copies the ILocomotion pointer from `this+0x50` to the output parameter
- Clears `this+0x50` to null
- Returns S_OK (0) on success, S_FALSE (1) if nothing was piggybacked

### What Triggers the Mission After Swap-Back?

`FootClass::Locomotion_AI` (0x520F40) runs every tick and checks:
1. If `ILocomotion::Is_Moving()` returns true AND `TypeClass+0xD94` (Teleporter=yes):
   - Queries IPiggyback, gets piggybacker CLSID
   - Compares against `CLSID_TeleportLocomotion` (at 0x7E9AC0)
   - If match: checks ammo/ore level (FootClass+0x15E as double)
     - If ore level > threshold → assign Mission 0x18 (Harvest)
     - If ore level <= threshold → assign Mission 0x17 (Guard)
2. If `Is_Moving()` returns false:
   - Checks current mission and reassigns (Guard → Sleep, etc.)

### Sequence Summary

```
1. Mission_Deploy_Building drains the harvester's StorageClass to credits
   (one slot per HarvesterDumpRate tick, see DEPOSITOREFROMSTORAGE doc).
2. When storage empty, Mission_Deploy_Building calls
   BuildingClass::ReleaseDockedHarvester (0x4595C0) — the NORMAL exit path —
   which Stops the loco, calls Head_To(track 0x47, building_center ± 0x80 leptons),
   sets speed to 1.0, clears both +0x2E4 dock-link pointers, sends BREAK(3).
3. DriveLocomotionClass drives the miner away from the pad.
4. DriveLocomotionClass reaches destination, stops.
5. FootClass::AI polls IPiggyback::Is_Ok_To_End → true.
6. FootClass::AI calls End_Piggyback → TeleportLocomotion restored.
7. FootClass::Locomotion_AI detects Teleporter + not moving → assigns Harvest mission.
8. Mission_Harvest state 0 starts the next harvest cycle.
```

**Note (corrected 2026-05-19):** Step 2 above previously read "UndockUnit sends
ILocomotion::Head_To". That was wrong: `UndockUnit (0x4593A0)` is interrupt-only
(fires only when the refinery is destroyed, sold, or temporal-warped while a unit
is on the pad). Superseded 2026-05-21: the normal stock healthy eject is zero-link `Mission_Deploy_Building` state 4; `ReleaseDockedHarvester`
(0x4595C0)`. Both functions share the same `Head_To(0x47, ±0x80, speed=1.0)` sequence,
but `ReleaseDockedHarvester` additionally clears anim slots, plays a VOC, creates
exit anims, and calls `Set_Destination + Set_Mission(MOVE)` on the unit.

### Mission_Harvest State 0 Piggyback Check

In `UnitClass::Mission_Harvest` (0x73E5E0), state 0 also queries IPiggyback:
- If piggybacker CLSID matches TeleportLocomotion AND unit has a NavTarget:
  → Clears destination (`Set_Destination(NULL, 1)`)
- This handles the edge case where the miner still has a stale destination
  from the drive phase

---

## Gap 2: Refinery Destroyed During Dock

### Building Death Path

When a building's health reaches zero, `BuildingClass::ReceiveDamage` (0x442230)
processes the death (switch case 4).

### Case A: Unit Physically on the Pad (field_0x2E4 link)

At death (case 4 in ReceiveDamage), the code explicitly handles the docked unit:

```c
if (field_0x2E4 != 0) {    // dock link to unit on pad
    // Remove docked unit from the "nearby units" vector
    BuildingClass::UndockUnit();  // sends unit away
}
```

`BuildingClass::UndockUnit` (0x4593A0):
- Gets the docked unit from `this+0x2E4`.
- Verifies the unit's active locomotor is `DriveLocomotionClass` (vtable+0x2C == 1);
  otherwise no-ops. Both regular harvester and chrono miner satisfy this during dock
  (chrono miner has DriveLoco piggybacked over TeleportLoco; Drive is active).
- Calls `ILocomotion::Stop()` then `ILocomotion::Head_To(track 0x47, X−0x80, Y+0x80)`.
  The `(-0x80, +0x80)` are hardcoded lepton literals (verified bytes
  `81 EB 80 00 00 00` / `81 C5 80 00 00 00`), NOT a BuildingTypeClass field.
  `0x47` is a hardcoded drive-track index (push `6A 47`), NOT a `TechnoClass::Facing`
  field write — the facing updates as the loco follows the track.
- Calls the speed setter at `unit_vtable+0x544` with IEEE 754 double 1.0
  (= speed multiplier, restores full unit speed).
- Clears both dock links: `unit+0x2E4 = 0` and `building+0x2E4 = 0`. The original draft's
  `unit[0xB9] = 0` / `building[0xB9] = 0` notation reflects int*-indexed decompilation
  (0xB9 × 4 = 0x2E4). The byte offset is `+0x2E4` on both classes — a symmetric
  cross-reference pointer.
- Sends BREAK(3) via vtable+0x274 to notify the production system.

So the docked unit is safely ejected before the building is destroyed.
The unit ends up driving away and will transition via the normal piggyback
swap-back mechanism (Gap 1).

### Case B: Unit in Dock Queue (Approaching)

**CORRECTED 2026-05-19:** The original draft attributed approach-queue management to a
"DockManager-like object processed by FUN_006AF6C0" and described a five-int per-entry
layout (unit ptr at [0], state at [1], cleanup state 6). That entire mechanism does
NOT exist for refineries. `FUN_006AF6C0` is `SlaveManagerClass::AI_Update`; the
"queue entries" with state codes 0–6 and the cleanup-state-6 transition are slaves
managed by a Slave Miner, not harvesters approaching a refinery.

**The actual refinery model:** approach is driven on the **unit side** by
`UnitClass::Mission_Enter` (0x739EC0), which radios `HELLO(2)` to register in the
refinery's `Contacts[]` array (capacity = `BuildingTypeClass+0x1780` `NumberOfDocks`,
default 1 for all stock YR refineries), then sends `CAN_DOCK(0x0E)` to receive the
queue-cell direction from `MOVE_TO_CELL(0x12)`. There is no separate dock-queue
state machine on the refinery side; the unit's mission state drives the whole sequence.

When the building is destroyed:
1. The building broadcasts radio OVER_AND_OUT to all radio contacts.
2. Units in approach receive radio 7 in `UnitClass::Receive_Radio`:
   - Clears destination: `Set_Destination(NULL, 1)`
   - Clears target: `Set_Target(NULL)`
   - Sets mission to Guard: `Set_Mission(0, 0)`
   - Attempts to re-enter the sender: `Transmit_Radio(2, sender)` + `Set_Mission(0x18)`
3. Since the building is dead, the re-enter attempt fails.
4. The unit falls back to Guard/Harvest mission via normal mission logic.

The building's destructor cleans up the `Contacts[]` array. No DockManager exists on
the refinery; the approach was always unit-side state plus radio-link membership.

### Case C: During Unload (Mission_Deploy_Building inline dump)

**CORRECTED 2026-05-19:** Original draft cited `BuildingClass::DepositOreFromStorage
(0x522D50)` and said it is called from "the dock queue processor (state 4→5 transition
in FUN_006AF6C0)". That is wrong on two counts:
- `0x522D50` is slave-miner-only (sole xref: `SlaveManagerClass::AI_Update` state 4).
- There is no refinery-side dock queue processor. The harvester ore-to-credits unload
  runs **inline** inside `UnitClass::Mission_Deploy_Building` (0x73D630), one slot's
  full amount drained per `HarvesterDumpRate` tick (Rules+0x1528 × 900 = 14.4 frames).

The storage being drained is on the **harvester** itself at `UnitClass+0x33C`
(a `StorageClass` = 4 floats indexed by tiberium type 0..3, in credit-value units).
Ore is never transferred to the building before crediting — it goes directly from the
unit's StorageClass to the owner's Balance via `HouseClass::Add_Tiberium_Credits
(0x4F9610)`.

If the building is destroyed mid-deposit:

1. The current tick's `ReceiveDamage` fires and hits death (case 4).
2. The docked unit is forcibly undocked via `UndockUnit()` (interrupt path, see Case A).
3. **Any ore not yet credited remains in the harvester's StorageClass — it is NOT lost.**
   `Mission_Deploy_Building`'s `Look_up_building_in_cell()` returns null on the next
   tick, the dump branch is skipped, and the harvester transitions to Guard. The unit
   carries the remaining ore to the next refinery it docks at.
4. The unit drives away with whatever ore it still holds.

There is no refinery-side dock-queue state machine to "stop processing" — the
inline dump loop simply detects the dead building via the null cell-lookup and bails.

### Orphaned Harvester Final State

In all cases, the orphaned harvester ends up:
- Mission Guard or Harvest (depending on ore level)
- No radio contact (cleared by Over_And_Out)
- No destination (cleared by radio 7 handler or UndockUnit)
- Will attempt to find a new refinery on next Mission_Harvest cycle

---

## Gap 3: FUN_00500200 — AI Harvester Wander Point Generator

### Corrected Understanding

**FUN_00500200 is NOT an "alternative refinery finder."** It is an AI harvester
**random wander point generator** used when a refinery is busy.

### Address: 0x500200
### param_1 type: `undefined4 *` (output cell coordinate)
### param_2 type: `int *` (the unit, FootClass-derived)

### What It Does

1. Reads the unit's ore storage levels via three vtable calls:
   - `vtable+0x2DC` (GetStorage type 2)
   - `vtable+0x2D8` (GetStorage type 1)
   - `vtable+0x2D4` (GetStorage type 0)
2. If total ore > 0: picks random case 1-4 (directional quadrants)
3. If total ore == 0: uses case 0 (any direction)
4. Calls `FUN_00501AC0` (0x501AC0) — the AI random cell picker
5. Calls `FootClass::Find_Nearby_Passable_Cell` to find valid terrain near that point
6. Returns the passable cell coordinate

### FUN_00501AC0 — AI Random Cell Picker (0x501AC0)

This is a helper that generates random coordinates at varying distances:

- Uses `HouseClass+0x5498` (house threat range) clamped to [0x300, 0x800]
- **Case 0:** Random distance [0, range], random direction from a reference point
- **Cases 1-4:** Random distance [range, range*2], biased to different quadrants
  (N, E, S, W roughly) using trigonometric calculations (Cos_lookup, Sin_lookup)
- Falls back recursively with case 0 if the generated cell is invalid

### Call Context in Mission_Enter

Called from `UnitClass::Mission_Enter` (0x739EC0) at approximately offset +0xB80:

```
radio_result = Transmit_Radio(8);  // REQUEST_DOCK
if (radio_result == 0x17) {        // QUEUED (refinery busy)
    if (!IsPlayerControlled) {     // AI only
        cell = FUN_00500200(this);
        if (cell is valid) {
            Set_Mission(MOVE);
            Set_Destination(cell);
            Set_Mission(HARVEST);  // will retry dock later
        }
    } else {
        // Player-controlled: stop and scatter
    }
}
```

### Why AI-Only?

Player-controlled harvesters just stop and wait (or the player can manually
redirect them). AI harvesters need to keep moving to avoid traffic jams at
busy refineries — they wander to a random point and will retry docking later
when Mission_Harvest cycles back to the "find refinery" state.

The actual refinery selection (finding the closest available one) is done by
`Find_Dock_Object` (vtable+0x528), called from Mission_Harvest state 2.

---

## Key Addresses Reference

| Function | Address | Purpose |
|----------|---------|---------|
| FootClass::AI | 0x4DA530 | Per-tick update, includes piggyback swap-back |
| FootClass::Locomotion_AI | 0x520F40 | Per-tick locomotor state, mission reassignment |
| BuildingClass::ReleaseDockedHarvester | 0x4595C0 | **Normal post-unload exit** (called from Mission_Deploy_Building) |
| BuildingClass::UndockUnit | 0x4593A0 | **Interrupt-only eject** (ReceiveDamage case-4 / Sell / TemporalClass::Update) |
| BuildingClass::Receive_Radio | 0x43C2D0 | Refinery radio handler (vtable +0x194); HELLO(2) passes through to RadioClass |
| BuildingClass::ReceiveDamage | 0x442230 | Building damage/death handler |
| BuildingClass::OnDestroyed | 0x445880 | Building death cleanup |
| UnitClass::Mission_Harvest | 0x73E5E0 | Harvest mission state machine (sends HELLO=2 in state 2) |
| UnitClass::Mission_Enter | 0x739EC0 | Enter/dock mission state machine (sends CAN_DOCK=0x0E) |
| UnitClass::Mission_Deploy_Building | 0x73D630 | **Harvester inline ore-dump loop** + caller of ReleaseDockedHarvester |
| UnitClass::Receive_Radio | 0x737430 | Unit radio message handler |
| SlaveManagerClass::AI_Update | 0x6AF6C0 | **Slave-miner state machine** (NOT a refinery dock-queue — corrected 2026-05-19) |
| BuildingClass::DepositOreFromStorage | 0x522D50 | **Slave-miner-only** ore→credits (NOT used by harvester/chrono miner) |
| FUN_00500200 | 0x500200 | AI harvester wander point generator |
| FUN_00501AC0 | 0x501AC0 | AI random cell picker (directional) |
| DriveLocomotionClass::Is_Ok_To_End | 0x4AF970 | Check if piggyback can end |
| DriveLocomotionClass::End_Piggyback | 0x4AF930 | Extract piggybacked locomotor |
| DriveLocomotionClass::Begin_Piggyback | 0x4AF8E0 | Stack a new locomotor |
| TechnoClass::Set_Destination | 0x741970 | Destination setter, piggyback start |
| FootClass::Set_Destination_Internal | 0x4D94B0 | Internal destination + ILocomotion::Head_To |
| RadioClass::Receive_Radio | 0x65A820 | HELLO(2) accept logic (Contacts[] insertion) |
| RadioClass::Broadcast_Radio_ToAll | 0x65ACE0 | Broadcast radio to all contacts |
| HouseClass::Add_Tiberium_Credits | 0x4F9610 | Owner-credit add (no MaxCash cap at deposit) |
