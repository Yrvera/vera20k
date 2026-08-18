# Building Dock & Heal State Machines — Ghidra Research Report

## Critical Correction: Type+0x16C1 is Hospital, NOT Refinery

The branch at `BuildingClass::MissionRepairAndProduce` (0x44B780) gated by
`Type+0x16C1 != 0` is the **Hospital** branch, not the refinery branch.

**BuildingTypeClass flag mapping** (verified from BuildingTypeClass ReadINI at 0x460A5B):

| Offset   | INI Key            | Purpose                                   |
|----------|--------------------|-------------------------------------------|
| +0x16A9  | UnitRepair         | Repair pad (heals vehicles)               |
| +0x16AA  | UnitReload         | Rearm pad                                 |
| +0x16AB  | Bunker             | Bunker (infantry enter)                   |
| +0x16AD  | Grinding           | Grinder (destroys units for money)        |
| +0x16AE  | UnitAbsorb         | Absorbs units                             |
| +0x16AF  | InfantryAbsorb     | Absorbs infantry                          |
| +0x16B3  | DockUnload         | Dock + unload (refinery uses this)        |
| +0x16B9  | ConstructionYard   | CY deploy logic                           |
| +0x16BB  | Refinery           | Refinery flag (storage/money)             |
| +0x16BC  | Weeder             | Weeder building                           |
| +0x16BD  | WeaponsFactory     | War factory                               |
| +0x16C1  | Hospital           | Heals infantry via timer                  |
| +0x16C2  | Armory             | Promotes infantry via timer               |
| +0x16CB  | Helipad            | Helicopter landing pad                    |

**Refineries have `DockUnload=yes` AND `Refinery=yes`.**
The `Refinery` flag is for storage/money logic. The `DockUnload` flag controls
the dock queue system that manages harvester approach/unload/depart.

> **Correction 2026-05-21 - stock refinery link state**
>
> The flag mapping above remains correct. Later focused reports refine the stock
> refinery link state: standard `CMIN/HARV -> GAREFN/NAREFN` DockUnload uses
> radio admission and `TechnoClass+0x418`; it does not use reciprocal
> `unit/building +0x2E4` as the normal refinery dock slot. Reciprocal `+0x2E4`
> is real for Bunker/conditional release paths, but stock ore-refinery unload
> runs the zero-`+0x2E4` `Mission_Deploy_Building` path. See
> `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md` and
> `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM_GHIDRA_REPORT.md`.

Confidence: HIGH — directly verified from string xrefs in BuildingTypeClass ReadINI.

---

## Part 1: Hospital/Armory Timer State Machine (MissionRepairAndProduce)

**Function:** `BuildingClass::MissionRepairAndProduce` at 0x44B780 (833 lines)
**param_1 type:** `BuildingClass *` (direct byte offsets)

### Function Structure Overview

The function is a large if/else chain checking building type flags in priority order:

1. `Type+0x16AB` (Bunker) — calls FUN_00458e50
2. `Type+0x16B9` (ConstructionYard) — GrandOpening + idle anim
3. **`Type+0x16C1` (Hospital)** — heal timer state machine
4. **`Type+0x16C2` (Armory)** — promote timer state machine
5. `Type+0x16A9` (UnitRepair) — repair pad state machine
6. `Type+0x16AA` (UnitReload) — rearm pad logic
7. Fall-through: default dock/undock with IPiggyback locomotion

All three timer-based branches (Hospital, Armory, UnitRepair) share the same
field layout and mechanics but differ in the completion condition and action.

### Hospital Branch (Type+0x16C1)

#### BuildingClass Field Layout

| Offset | Type  | Name               | Init Value | Purpose                                    |
|--------|-------|--------------------|------------|--------------------------------------------|
| +0x0BC | int   | DockState          | 0          | State machine: 0=init, 2=active            |
| +0x2FC | int   | UnloadCounter      | Type+0x684 | Visual swap counter (UnloadingClass)        |
| +0x620 | int   | HealProgress       | 0          | Accumulated progress toward next heal step  |
| +0x624 | byte  | HealTickedThisFrame| 0          | 1 if timer ticked this frame, 0 otherwise  |
| +0x628 | int   | TimerStartFrame    | CurFrame   | CDTimerClass start frame                   |
| +0x62C | int   | TimerUnknown       | ???        | CDTimerClass field (likely high word)       |
| +0x630 | int   | TimerDuration      | 1          | CDTimerClass remaining duration             |
| +0x634 | int   | TimerStep          | 0→1        | Timer duration per tick AND active flag     |
| +0x638 | int   | ProgressIncrement  | 1          | Amount added to HealProgress per tick       |
| +0x6DD | byte  | HasDockedUnit      | 0          | 1 when unit is docked, 0 when idle         |

#### State 0 → Init

When `DockState == 0`:

```
DockState = 2
HasDockedUnit (0x6DD) = 0
HealProgress (0x620) = 0
TimerStep (0x634) = 1
TimerStartFrame (0x628) = CurrentFrame
TimerDuration (0x630) = 1
if (Type.UnloadingClass != -1):
    UnloadCounter-- (clamped to 0)  // triggers visual swap to alt SHP
```

The UnloadingClass swap: `Type+0x684` stores the UnloadingClass image index.
`building+0x2FC` is initialized from this value. When decremented to 0, the
building renders using the alternate (unloading) SHP image. It gets restored
in `BuildingClass::Update` (0x43FB20) when Hospital and Armory flags are both
false (i.e., after the unit departs).

#### State 2 → Active Healing

Each tick while `DockState == 2`:

**Step 1: Timer tick**
```
remaining = CDTimerClass__GetTimeRemaining(building+0x628)
if (remaining == 0 AND TimerStep != 0):
    HealTickedThisFrame = 1
    HealProgress += ProgressIncrement  // +1 per tick
    reset timer: start=CurrentFrame, duration=TimerStep
else:
    HealTickedThisFrame = 0
```

**Step 2: Completion check**
```
threshold = IRepairRate * 900.0
if (HealProgress >= threshold):
    HasDockedUnit = 0
    HealProgress = 0
    result = Transmit_Radio_ToFirst(RADIO_REPAIR=0x1C)  // send heal to unit
    handle result...
```

**The formula:** `IRepairRate * 900.0` converts the INI rate (in minutes) to
game frames. At IRepairRate=0.001, threshold = 0.9, so healing triggers every
1 frame (since progress increments by 1 per frame and 1 >= 0.9).

#### Radio Message 0x1C (RADIO_REPAIR) Handling

When the building sends radio 0x1C to the docked unit, the unit's
`TechnoClass::Receive_Radio` at 0x6F4AB0 case 0x1C:

1. If unit health >= ConditionYellow (rules+0x16F8): return 10 (already healthy)
2. Calculate repair cost from Type->GetRepairCost() and step from Type->GetRepairStep()
3. If owner can afford: spend money, add health
4. Return 1

Back in the building, the return value determines next action:

| Return | Meaning          | Building Action                                        |
|--------|------------------|--------------------------------------------------------|
| 10     | ROGER (healthy)  | Fall through: UndockUnit + SetMission(Guard)           |
| 0x20   | OVER_OUT (32)    | Remove from dock queue only, return                    |
| 0x21   | COMPLETE (33)    | Play EVA + sound if human, then UndockUnit + Guard     |
| other  | unexpected       | return (no action)                                     |

**UndockUnit sequence** (at completion):
```
FUN_00473430(DAT_0089c818)  // remove from dock linked list
vtable+0x100(result)         // clear dock contact
SetMission(5, 0)             // Mission::Guard
```

### Armory Branch (Type+0x16C2)

Identical timer mechanics to Hospital. Differences:

- **Completion condition:** Same formula but with same `rules+0x16F0` (IRepairRate)
  — wait, actually re-checking... The Armory branch uses the SAME `rules+0x16F0`
  (IRepairRate * 900) as Hospital.
- **Completion action:** Instead of sending RADIO_REPAIR, the Armory promotes the
  docked infantry:
  ```
  destination = FootClass__GetDestination(0)
  if (VeterancyStruct__IsRookie(destination)):
      VeterancyStruct__SetVeteran(destination)
  else:
      VeterancyStruct__SetElite(destination)
  ```
  Then UndockUnit + SetMission(Guard).

### UnitRepair (Repair Pad) Branch (Type+0x16A9)

Much more complex, with 3 states (0, 1, 2):

- **State 0:** Init, transitions to state 2
- **State 2:** Timer-based healing with same mechanics
  - **Completion condition:** `URepairRate * 900.0 <= HealProgress`
  - Uses `rules+0x16E8` = **URepairRate** (0.016 default = 14.4 frame threshold)
  - Sends radio 0x13 (RADIO_CAN_ENTER) to check if unit is still there
  - Then sends radio 0x1C (RADIO_REPAIR) to actually heal
  - On completion (unit healthy): transitions to state 1
  - On 0x20 (unit leaving): plays EVA warning, clears anims, goes to state 1
  - On 0x21 (complete): clears anims, sends unit to rally point, goes to state 1
- **State 1:** Unit departing
  - Checks PathType__Has_Valid_Steps() — waits for unit to leave
  - If unit stopped: checks if it needs to force-depart via locomotion
  - Checks distance < 200 to determine arrival/departure
  - Uses IPiggyback locomotion interface for complex movement

---

## Part 2: Refinery Harvester Unload FSM

> **CORRECTION 2026-05-06:** The original Part 2 documented `FUN_006AF6C0` as the
> refinery dock state machine. That function is actually `SlaveManagerClass::AI_Update`
> — it manages slave-miner / slave-infantry behavior, NOT the standard harvester
> unload cycle. The refinery dock FSM lives in `UnitClass::Mission_Deploy_Building`
> (0x73D630), driven from the **harvester unit's** mission processor, not from the
> building. This section is rewritten to reflect the correct entry point.

**Function:** `UnitClass::Mission_Deploy_Building` at 0x73D630
**Called from:** `UnitClass::AI` mission dispatch when `Mission == Unload`
**param_1 type:** the harvester (unit) — direct byte offsets into UnitClass
**Stock refinery link state:** standard `CMIN/HARV -> GAREFN/NAREFN` DockUnload
does **not** use reciprocal `unit/building +0x2E4` as the normal dock slot or
reservation. The live path is radio/contact-driven: `Mission_Harvest` sends
`HELLO(0x02)`, `Mission_Enter` sends `CAN_DOCK(0x0E)`, the building replies
`0x13/0x12` and only emits `0x18/0x16` after the unit is already at the accepted
cell, and `UnitClass::PerCellProcess` sends pad-arrival `0x15` to start unload.
See `BUILDINGCLASS_FIELD_0X2E4_REFINERY_DOCK_GHIDRA_REPORT.md`,
`MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md`, and
`MISSION_ENTER_DOCK_ARRIVED_0X0C_GHIDRA_REPORT.md`.

### State machine — 4 inner cases (0/1/3/4)

The harvester drives the dock cycle itself. There is no separate "queue
manager" iterating over slots. The mission state field (`unit+0xBC`) is the
single source of truth, and the inner switch only ever reaches cases 0, 1, 3,
and 4. (Cases 2/5/6 in the original Part 2 were Tiberian Sun ghosts.)

#### Case 0/1: Admission and arrival
- **Superseded note:** older text here described polling/grabbing
  `BuildingClass+0x2E4`. The 2026-05-21 re-swarm disproves that for standard
  stock refineries.
- The stock inbound chain is staged:
  1. `Mission_Harvest` state 2 sends `HELLO(0x02)` and advances to substate 3
     only on `ROGER(1)`.
  2. Substate 3 queues `Mission_Enter` (`mission 7`).
  3. `Mission_Enter` sends `CAN_DOCK(0x0E)`.
  4. `BuildingClass::Receive_Radio(0x0E)` replies `0x13 -> 0x12`, and only
     sends `0x18 -> 0x16` after `0x12` reports the unit is already at the
     accepted cell.
  5. `UnitClass::PerCellProcess` sends pad-arrival `0x15`; building case
     `0x15` queues sender mission `0x10`, which enters this unload FSM.
- Stock inbound refinery docking does **not** send `0x0C DOCK_ARRIVED`.
- `+0x2E4` remains a real conditional reciprocal link field for bunker/release
  contexts, but not the normal stock ore-refinery admission/unload link.

#### Case 3: Unloading (per-bale pulse)
- Each tick, decrement the unload timer. When it reaches 0:
  1. Pop one bale from the harvester's storage.
  2. Add the bale's credit value to the owner's credits.
  3. If the owner has a Purifier (any building with `OrePurifier=yes`), add
     `value × PurifierBonus` credits **on the same bale** — the bonus is
     per-bale, not per-load. (gamemd applies it inline at deposit time.)
  4. Trigger the refinery's SpecialAnim (slot 10 — `GAREFNOR` for GAREFN)
     one-shot. Reset the frame state if the slot is already playing.
  5. Spawn up to 4 particle systems at `RefinerySmokeOffsetOne..Four` (the
     refinery's vtable+0x468 emitter). Offsets that are zero are skipped —
     hence Allied refineries only show 2 visible smoke puffs even though all
     four offsets are read.
  6. Re-arm the unload timer to `HarvesterDumpRate × 900` frames (default
     0.016 × 900 = 14.4, stored as integer tenths so the fractional cadence
     is preserved exactly across bales).
- When the cargo is empty:
  - Clear the stock unload-active state; do not model this as releasing a
    normal refinery `BuildingClass+0x2E4` reservation.
  - Clear the harvester's UnloadingClass override.
  - Advance `unit+0xBC` to **4 (Departing)**.

#### Case 4: Departing
- Follow the stock zero-link state-4 handoff. Normal stock CMIN/HARV unload
  completion does **not** call `ReleaseDockedHarvester`, does **not** seed
  `Force_Track(0x47)`, and does **not** install a cached queue/exit-cell
  destination.
- Clear the stock unload-active state and any radio/contact bookkeeping used
  for the unload. If a radio contact remains, state 4 may send `BREAK(0x03)`;
  `LEAVE_DOCK(0x19)` is only an indirect conditional
  `TechnoClass::Receive_Radio` cascade when both sides still have `+0x418`
  set.
- Clear `target_ore_cell` and `last_harvest_cell` so SearchOre re-scans
  instead of biasing toward the patch the harvester arrived from.
- Reset `unit+0xBC` to 0 and switch the unit's outer mission back to
  SearchOre/Harvest scheduling.

### Why the Tiberian Sun states are gone

gamemd inherits TS's state-machine scaffolding, but the YR refinery flow
collapses to four states because:
- TS used a multi-slot dock queue (`FUN_006AF6C0` walks a slot array). YR
  stock refineries use radio `Contacts[]` plus the unit-side unload FSM; they
  do not use `BuildingClass+0x2E4` as the normal DockUnload slot.
- TS relied on separate ground-pad rotation stages. YR keeps the visible dock
  pivot, but runs it through the radio `0x18 -> 0x16` entered/face-sync
  handshake and the unit's `Do_Turn(0x4000)`/PrimaryFacing RateTimer before
  the unload FSM begins; this is not a manual 8-bit facing step.
- TS's "post-deposit delay" was a building animation hold. In YR the
  SpecialAnim and particles are emitted per-bale, so there is nothing to wait
  for after the last bale.

If you see references to states 2, 5, or 6 in another doc or in older
research notes, treat them as TS holdovers and verify before implementing.

### Cross-references
- Anim slot 7/10/8 wiring: [REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md](REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md)
- 21-slot building anim system: [BUILDING_ANIM_STATE_MACHINE.md](BUILDING_ANIM_STATE_MACHINE.md)
- Per-bale credit math (Part 3 below) is correct as written.

---

## Part 3: DepositOreFromStorage (0x522D50)

**Function:** `BuildingClass__DepositOreFromStorage` at 0x522D50 (49 lines)
**param_1 type:** `int *` (BuildingClass pointer, `int *` → multiply by 4)

This function empties the building's ore storage and converts it to credits.

```c
void DepositOreFromStorage(BuildingClass* building) {
    bool deposited = false;
    int slot = StorageClass__FindFirstNonEmptySlot();
    
    while (slot != -1) {
        int typeIdx = building->Type;
        int baseValue = typeIdx->field_0x538C;  // ore base value
        
        // Bonus from harvester dump rate?
        if (typeIdx->field_0x1EC == 0 && GameMode != 0):
            baseValue += rules->SomeTable[typeIdx->field_0x184]
        
        float amount = StorageClass__GetAmount(slot);
        float bonus = baseValue * rules->OreGrowthRate(0xF3C) * amount;
        
        float removed = StorageClass__RemoveAmount(amount, slot);
        if (removed > 0.0):
            deposited = true
            HouseClass__Add_Tiberium_Credits(removed, slot)
            if (bonus > 0.0):
                HouseClass__Add_Tiberium_Credits(bonus, slot)
        
        slot = StorageClass__FindFirstNonEmptySlot();
    }
    
    if (deposited):
        building->vtable+0x468()  // update display/stats
}
```

**Key:** This deposits ALL ore at once — it's not a per-bale drip. The entire
storage is emptied in a single call when the harvester reaches the building center.

---

## Part 4: Docking Flow — Radio Communication

### Harvester → Refinery Docking Sequence

From `BuildingClass::Receive_Radio` (0x43C2D0):

1. **RADIO_CAN_I_DOCK (0xE):** Harvester asks to dock
   - For DockUnload (0x16B3): checks `unit->Type+0xE0E` (Harvester flag)
   - If building has space (`field_0x118 == 0`): accept
   - Returns 1 (accepted) with building pointer in param_4

2. **RADIO_DOCKING_ACCEPTED (0x15):**
   - For DockUnload: sets unit mission to 0x10 (Mission::Enter)
   - For Hospital/Armory: sets `building->field_0x6DD=1`,
     building mission to 0x14 (RepairAndProduce), unit mission to 0 (Sleep)

3. **RADIO_CAN_ENTER (0x10):**
   - Checks if building is idle and owned by same player
   - Returns 1 if Refinery, UnitRepair, or Weeder

### Key Difference: Refinery vs Hospital/Armory

| Aspect                | Hospital/Armory                      | Refinery (DockUnload)              |
|-----------------------|--------------------------------------|------------------------------------|
| Building mission      | RepairAndProduce (0x14)              | Does NOT use this mission          |
| Unit mission          | Sleep (0)                            | Enter (0x10)                       |
| Timer mechanism       | field_0x620/0x628/0x634/0x638        | NOT used                           |
| Processing function   | MissionRepairAndProduce (0x44B780)   | Dock queue (0x6AF6C0)              |
| Deposit call          | N/A (sends RADIO_REPAIR instead)     | DepositOreFromStorage (0x522D50)   |
| Deposit granularity   | Per-step (heal one step per cycle)   | Atomic (all ore at once)           |
| Completion signal     | RADIO_REPAIR returns 10              | Unit storage becomes empty         |

---

## Part 5: Visual Swap (UnloadingClass)

### BuildingClass+0x2FC: UnloadCounter

- **Init:** Set from `Type+0x684` (UnloadingClass, -1 if none)
- **Decrement:** In MissionRepairAndProduce state 0, when Hospital or Armory
  begins processing a unit: `UnloadCounter--` (clamped to 0)
- **Restore:** In `BuildingClass::Update` (0x43FB20), when both Hospital and
  Armory are false: `if (UnloadCounter == 0) UnloadCounter = Type+0x684`

When `UnloadCounter` reaches 0, the building renderer uses the alternate SHP
from UnloadingClass instead of the normal Image.

**For refineries:** The UnloadingClass visual swap is NOT controlled by
MissionRepairAndProduce. It would need to be controlled by the dock queue
system or the rendering code directly. Need further investigation.

---

## Part 6: Refinery Smoke Particles

The smoke fields in TechnoTypeClass (read in TechnoTypeClass::ReadINI):
- `RefinerySmokeOffsetOne/Two/Three/Four` — 3D offsets for smoke origins
- `RefinerySmokeFrames` — at BuildingTypeClass+0x156C, frame count for smoke anim
- `RefinerySmokeParticleSystem` — particle system type name

These are spawned in `BuildingClass::Update` (0x43FB20) in the section around
lines 130-170 where it iterates over MuzzleFlash/occupant offsets. The smoke
is spawned when the building is "working" (has docked unit active).

Confidence: MEDIUM — verified the INI read locations but did not trace the exact
runtime smoke spawn code to confirm it's tied to refinery unloading specifically.

---

## Part 7: Edge Cases

### Harvester destroyed during unload
The dock queue (0x6AF6C0) checks `*piVar1` (unit pointer) for null at the top
of the loop. If the unit is destroyed, the pointer becomes null and the slot
transitions to state 6 (cleanup/removal).

### Power goes out
`BuildingClass::Receive_Radio` case 0xE and 0xF both check `HasPower` flag.
If power is lost, radio returns 10 (denied), preventing new docking. Units
already docked continue their current state — the dock queue doesn't check power.

### Building sold during unload
Not directly visible in these functions. Building destruction/sell would trigger
radio 0xC (RADIO_OVER_OUT) to the docked unit, causing it to abort.

---

## RulesClass Field Map (verified)

| Offset   | Type   | INI Key           | Default | Usage                              |
|----------|--------|-------------------|---------|------------------------------------|
| +0x0F3C  | float  | ???               | ???     | Ore value multiplier               |
| +0x0DF8  | int    | MaxDockDistance?   | ???     | Max distance for dock proximity    |
| +0x1528  | double | HarvesterDumpRate  | ???     | NOT used in MissionRepairAndProduce|
| +0x16E8  | double | URepairRate        | 0.016   | Repair pad heal interval (minutes) |
| +0x16F0  | double | IRepairRate        | 0.001   | Hospital heal interval (minutes)   |
| +0x16F8  | double | ConditionYellow    | ???     | Health ratio threshold             |
| +0x1700  | double | ConditionRed       | ???     | Severe damage threshold            |
| +0x1784  | int    | ???               | ???     | Random coord interval for docking  |

---

## Summary of Key Findings

1. **`Type+0x16C1` is Hospital, NOT Refinery.** The user's initial premise was incorrect.

2. **Refineries use the dock queue system** (0x6AF6C0), not MissionRepairAndProduce.
   The `DockUnload=yes` flag triggers this system, separate from `Refinery=yes`.

3. **Ore deposit is atomic** — `DepositOreFromStorage` empties ALL storage at once
   when the harvester reaches the building center. There is no per-bale drip.

4. **HarvesterDumpRate (rules+0x1528) is NOT referenced in MissionRepairAndProduce.**
   It's read from INI but its usage location was not traced in this investigation.

5. **The timer at building+0x620/0x628/0x634/0x638** is used for Hospital (infantry
   heal), Armory (infantry promote), and Repair Pad (vehicle heal) — NOT for refinery
   ore dumping.

6. **The 900.0 constant** at 0x7E27F8 converts INI rate values (in minutes) to game
   frame counts: `rate_in_minutes * 900 = frame_threshold`.
   (900 frames = 1 minute at 15 FPS game speed)
