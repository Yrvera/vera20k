# BuildingClass::MissionRepairAndProduce Deep Dive

**Address:** `0x0044B780` (833 decompiled lines)
**Date:** 2026-03-23
**Confidence:** HIGH (verified from binary via Ghidra MCP)

## Overview

This function is the main mission handler for buildings in the "Repair and Produce" mission
(mission type 0x1C / 28). It dispatches to 6 sub-systems based on `BuildingTypeClass` boolean
flags. The function uses a state machine via `field_0xBC` (MissionState: 0=init, 1=active,
2=processing).

### BuildingTypeClass Flag Offsets (verified from ReadINI at 0x00460905)

| Offset  | INI Key           | Purpose                     |
|---------|-------------------|-----------------------------|
| 0x16A9  | `UnitRepair`      | Service Depot               |
| 0x16AA  | `UnitReload`      | Reload Pad (ammo)           |
| 0x16AB  | `Bunker`          | Battle Bunker               |
| 0x16B9  | `ConstructionYard`| MCV/Construction Yard       |
| 0x16C1  | `Hospital`        | Infantry healing             |
| 0x16C2  | `Armory`          | Infantry veterancy promotion |

### Dispatch Order (checked first to last)

1. **Bunker** (0x16AB) - checked first, calls `FUN_00458E50`
2. **ConstructionYard** (0x16B9)
3. **Hospital** (0x16C1)
4. **Armory** (0x16C2)
5. **UnitRepair** (0x16A9) - Service Depot
6. **UnitReload** (0x16AA) - Reload Pad
7. If none match: returns `0x0F` (MISSION_SLEEP)

---

## 1. UnitRepair (Service Depot) Path - `Type[0x16A9]`

### State Machine

- **State 0 (Init):** Transitions to state 2. Sets up repair anim based on health ratio vs
  `ConditionYellow`. Initializes the repair accumulator:
  - `field_0x620` = 0 (accumulator reset)
  - `field_0x624` = step-happened flag
  - `field_0x628` = `g_CurrentFrameCounter` (CDTimer start)
  - `field_0x634` = 1 (timer rate)
  - `field_0x630` = 1

- **State 1 (Unit Approaching):** Guides unit to center of service depot.
  - Checks `PathType::Has_Valid_Steps()` - if path is clear, continue.
  - Checks `field_0x57C` and `field_0x588` (production/factory flags) for anim management.
  - Uses `BuildingClass::DistanceToObject` (vtable+0x4D8, `0x00447E00`) to check if unit is
    within 200 leptons of the dock position.
  - Locomotion management at `field_0x674`:
    - vtable+0x60: `ILocomotion::Is_Moving()`
    - vtable+0x10: `ILocomotion::Is_Moving_Now()`
    - vtable+0x5C: `ILocomotion::Stop_Moving()`
    - vtable+0x58: `ILocomotion::Force_Track()`
  - Uses `BuildingClass::GetDockCellForObject` (vtable+0x4D4, `0x0044EFB0`) to compute the
    target cell for the unit, factoring in the building's foundation offset at `field_0x218`.
  - Sets ghost cell via `TechnoClass::SetGhostCell` (writes to `TechnoClass+0x218`)
  - Radio command 0x13 ("are you still there?") to check link.

- **State 2 (Repairing):** The core repair loop.
  - CDTimer at 0x628 controls tick rate. When timer expires AND `field_0x634 != 0`:
    - `field_0x620 += field_0x638` (accumulate repair step)
    - Reset timer.
  - **Threshold check:** `Rules.URepairRate * 900.0 <= field_0x620`
    - `900.0` = constant at `0x007E27F8` (15 fps * 60 = ticks per minute)
    - When threshold reached: send radio 0x13, then 0x1C to the docked unit.

### Radio Command Protocol (Service Depot)

| Command | Name                   | Meaning                                          |
|---------|------------------------|--------------------------------------------------|
| 0x13    | RADIO_NEED_TO_MOVE     | "Are you still linked?" Response 1=yes            |
| 0x1C    | RADIO_REPAIR_ONE_STEP  | "Repair yourself one step." Handled in TechnoClass |
| 0x1F    | RADIO_RELOAD_AMMO      | "Reload one ammo." Increments ammo counter        |
| 0x20    | RADIO_CANT_AFFORD      | Response: owner can't afford repair cost          |
| 0x21    | RADIO_REPAIR_DONE      | Response: unit fully repaired                     |

### Radio 0x1C Handler (TechnoClass::Receive_Radio at `0x006F4AB0`)

This is the actual repair logic:

```
1. Check health ratio >= Rules.RepairPercent (Rules+0x16F8) -> return RADIO_NEGATIVE (already full)
2. Get repair cost:  TypeClass->GetCost() via vtable+0xB0
3. Get repair step:  TypeClass->GetRepairStep() via vtable+0xB4
   - If step < 2, clamp to 1
4. Check owner funds:  HouseClass->Available_Money() >= cost
   - If can't afford: return 0x20 (RADIO_CANT_AFFORD)
5. Spend money:  HouseClass::SpendMoney(cost) at 0x004F9790
6. Add health:   Health += step, EstimatedHealth += step
7. Update visual warp effects if present (field at ObjectClass+0x310)
8. If health >= RepairPercent threshold:
   - Clamp health to TypeClass.Strength (max HP at Type+0xA0)
   - Return 0x21 (RADIO_REPAIR_DONE)
9. Otherwise return 1 (RADIO_ROGER, continue repairing)
```

### Service Depot Response Handling

When radio 0x1C returns:
- **0x20 (Can't Afford):** If player-controlled, play EVA "Unit Repaired" (misleading name?).
  Clear anim slots 8 and 11. Set up idle/repair anims. Transition to state 1.
- **0x21 (Repair Done):** Same as above but additionally:
  - If player-controlled: create radar event + play EVA.
  - Spawn `AnimClass` at building location (repair completion anim from `Rules+0x240`).
  - Clear anim slots, set up idle anims. Transition to state 1.
  - If unit has `field_0x86` (occupying cell?) and `FUN_0050b730` returns false:
    - Set mission to 2 (MISSION_MOVE), set destination, clear ghost cell.
    - Send radio command 3 (RADIO_OVER_AND_OUT).
- **Default / Continue:** Reset accumulator, send 0x1C again next cycle.

### Ghost Cell / Destination Logic

The unit is guided to the service depot center via:
1. Building computes dock cell: `GetDockCellForObject(unit)` (vtable+0x4D4)
2. If building has `field_0x218` (foundation helper), adjusts cell with lepton->cell conversion
3. Compares current cell to target: if different, sends unit to destination via
   `Set_Destination(MapClass::Get_CellClass(target), 1)` and `Queue_Mission(MOVE)`
4. Ghost cell at `TechnoClass+0x218` tracks the reserved cell
5. `piVar11[0x140] = 0` clears the unit's path destination after arrival

---

## 2. Hospital Path - `Type[0x16C1]`

### State Machine

- **State 0 (Init):** Transitions to state 2. Same accumulator init as service depot.
  - `field_0x6DD` = 0 (repair-in-progress flag)
  - `field_0x620` = 0 (accumulator)
  - Decrements `field_0x2FC` if `Type+0x684` (InitialAmmo?) != -1
    - Clamped to 0: `field_0x2FC = max(0, field_0x2FC - 1)`

- **State 2 (Healing):** CDTimer-based accumulator, same pattern as service depot.
  - Threshold: `Rules.IRepairRate * 900.0 <= field_0x620`
  - When threshold reached: send radio 0x1C to docked infantry.

### Accumulator Details

- **Rules+0x16F0 = `IRepairRate`** (double, read from [General] section)
- **Threshold:** `IRepairRate * 900.0` (same 900.0 constant at 0x007E27F8)
- The 900.0 multiplier converts the rate from "per minute" to "per tick accumulation"

### Radio Protocol

Same radio 0x1C as service depot. The `TechnoClass::Receive_Radio` handler at 0x006F4AB0
performs the same repair logic (check funds, spend money, add health).

Radio 0x1C responses:
- **0x20 (Can't Afford):**
  - Sends `FUN_00473430()` result to vtable+0x100 (detach from dock queue)
  - Returns immediately (unit stays but no further healing)
- **0x21 (Repair Done):**
  - If player-controlled: create radar event + play EVA
  - Spawn completion anim at building coordinates
  - Sends detach, then `Queue_Mission(GUARD, 0)` to self
- **Neither 0x20 nor 0x21:** Same detach + guard behavior

### No Sound or Anim on Heal Tick

The hospital does NOT spawn an anim or play a sound per heal tick. It only spawns an anim
on completion (radio 0x21) if the building is player-controlled.

---

## 3. Armory Path - `Type[0x16C2]`

### State Machine

Identical structure to Hospital (shares the same accumulator pattern).

- **State 0 (Init):** Same as hospital. Transition to state 2.
  - Unconditionally decrements `field_0x2FC` (no `-1` check like hospital)

- **State 2 (Promoting):**
  - Threshold: `Rules.IRepairRate * 900.0 <= field_0x620` (SAME as Hospital!)
  - When threshold reached: promotes the docked infantry unit.

### Veterancy Promotion Logic

```
1. Get destination unit (FootClass::GetDestination)
2. VeterancyStruct::IsRookie() at 0x0074FFF0
   - Checks if experience float < 0.0
   - Returns true = rookie
3. If NOT rookie (already veteran):
   - VeterancyStruct::SetElite(1) at 0x007500B0
   - Sets experience to 2.0 (elite threshold)
4. If rookie:
   - VeterancyStruct::SetVeteran(1) at 0x00750090
   - Sets experience to 1.0 (veteran threshold)
5. Detach unit from dock, Queue_Mission(GUARD)
```

**Key finding:** The Armory promotes by exactly ONE rank. Rookie -> Veteran, Veteran -> Elite.
It does NOT double-promote.

### Vtable Functions Identified

| Address    | Name                         | What it does                      |
|------------|------------------------------|-----------------------------------|
| 0x0074FFF0 | `VeterancyStruct::IsRookie`  | Returns true if XP < 0.0          |
| 0x007500B0 | `VeterancyStruct::SetElite`  | Sets XP = 2.0 (elite)             |
| 0x00750090 | `VeterancyStruct::SetVeteran`| Sets XP = 1.0 (veteran)           |

---

## 4. UnitReload Path - `Type[0x16AA]`

### No State Machine

Unlike the other paths, UnitReload does NOT use `field_0xBC` states. Instead it:

1. Loops through all dock slots: `for (i = 0; i < field_0xE8; i++)`
   - `field_0xE8` = number of occupied docks
2. For each docked unit (`FootClass::GetDestination(i)`):

### Per-Unit Radio Protocol

```
1. Send radio 0x1D to unit: "Request reload"
   Response 1 (ROGER):
     - Check unit->GetType()->field_0xA0 (Strength/MaxAmmo) vs unit[0x1B] (CurrentAmmo?)
     - If equal: already full, goto EJECT
     EJECT:
       - unit->Scatter_Force(1, 0)    [vtable+0x484]
       - unit->Set_Mission(GUARD)     [vtable+0x1F0]
       - unit->ReloadAmmo()           [vtable+0x334]

   Response != 1:
     - Check unit->GetCurrentMission() [vtable+0x184]
     - If mission == 7 (MISSION_ENTER): skip (unit still entering)
     - Otherwise: send radio 0x13 ("are you linked?")
       Response 1 (ROGER):
         bHasUnit = true
         - Check GetCurrentMission() == 0 (MISSION_NONE):
           - Send radio 0x1F to unit: "Reload one ammo"
             - If response != 1: send 0x1C, if also != 1: goto EJECT
         - If mission != 0:
           - Queue_Mission(MISSION_NONE, 0) on unit
```

### Radio 0x1F Handler (TechnoClass::Receive_Radio)

```
field_0x4C = current ammo count (in ObjectClass field layout)
Type+0x684 = max ammo (from TypeClass)
If current ammo == max ammo: return RADIO_NEGATIVE (10)
Otherwise: ammo++ and return 1 (ROGER)
```

### Timer

If any unit was being reloaded (`bHasUnit == true`): returns timer value from
`MissionClass::GetMissionTimerEntry` which reads the mission timer table at `0x00A8E3A8`.
If no units present: `Queue_Mission(GUARD, 0)`, return 3.

---

## 5. ConstructionYard Path - `Type[0x16B9]`

### State Machine

- **State 0 (Init):**
  - `Queue_Mission(MISSION_MOVE, false)` via `FUN_00447780()`
  - Checks health ratio vs `ConditionYellow` (Rules+0x1700)
  - Creates appropriate anim (idle/damaged) at Type+0x1128 or Type+0x1138
  - Transitions to state 2

- **State 2 (Validating):**
  - Calls `PathType::Has_Valid_Steps()`
  - If path is invalid: `Queue_Mission(GUARD, 0)`, clear anims, return 1
  - If valid: return 1 (keep waiting)

### Purpose

The CY in this mission simply validates that the building's path/placement is valid after
construction completes. If the path becomes invalid (something blocks deploy), it transitions
to GUARD mission. This is the MCV undeploy validation path.

---

## 6. Bunker Path - `Type[0x16AB]`

### FUN_00458E50 (Bunker State Machine) at `0x00458E50`

This is a 6-state state machine stored in `field_0x718`:

- **State 0 (Init):** Get docked unit from `field_0x2E4` or `FootClass::GetDestination(0)`.
  Verify it's type 1 (infantry via vtable+0x2C). Check locomotion is stopped. Scan all
  foundation cells for nearby objects using `CellClass::Find_Nearest_Object()` with radius
  0x80. Push away any blocking units. Transition to state 1.

- **State 1 (Scan & Orient):** Scan foundation cells again. If a unit is found that has
  `AbstractFlags & 4` and `field_0x169 == 0` (not tethered), transition to state 1 (keep
  scanning). If no blocking units found: calculate facing angle from unit to building using
  `atan2`, set a `RateTimer`, transition to state 2.

- **State 2 (Wait for Turn):** Wait for `CDTimerClass::Remaining()` == 0. Then compute
  facing direction (8 cardinal directions from `(angle >> 7) + 1 & 0x1FE`). Get building
  coords. Command unit locomotion: `ILocomotion::Move_To(coords)` (vtable+0x70).
  Set unit speed to max: vtable+0x544 with `0x3FF00000`. Transition to state 3.

- **State 3 (Enter Building):** Check unit is at building cell. Verify locomotion stopped.
  Set `RateTimer(0x8000)`. Transition to state 4.

- **State 4 (Wall Animation):** Wait for timer. Create wall-down anims based on health ratio.
  Type+0x11F4/0x1204 (active anim by health), Type+0x1238/0x1248 (second anim set).
  Transition to state 5.

- **State 5 (Dock Complete):** Tether unit to building:
  - `field_0x2E4 = unit` (building's docked unit ref)
  - `unit[0xB9] = building` (unit's owner building ref)
  - `unit[0x85] = -1` (mark as garrisoned?)
  - `unit->vtable+0x150` (some cleanup)
  - Transition to state 6
  - `Queue_Mission(GUARD, 1)` on docked unit
  - Spawn anim from `Rules+0x240` at building location

- **Error/Exit:** If unit not found or wrong type: `field_0x718 = 0`,
  `Queue_Mission(GUARD, 0)`.

---

## 7. INI Key to RulesClass Offset Mapping

All verified from `RulesClass::ReadGeneral` at `0x0066D530`:

| INI Key          | Rules Offset | Type   | Used By           | Read Function    |
|------------------|-------------|--------|-------------------|------------------|
| `RepairStep`     | +0x16CC     | int    | Building repair   | ReadInt          |
| `RepairPercent`  | +0x16D0     | double | Unit repair limit | ReadDouble       |
| `IRepairStep`    | +0x16D8     | int    | Infantry repair   | ReadInt          |
| `RepairRate`     | +0x16E0     | double | Building repair   | ReadDouble       |
| `URepairRate`    | +0x16E8     | double | Service Depot     | ReadDouble       |
| `IRepairRate`    | +0x16F0     | double | Hospital/Armory   | ReadDouble       |
| (no INI key)     | +0x16F8     | double | Unit repair cap (service depot) | Constructor default |
| `ConditionYellow`| +0x1700     | double | Damage threshold  | ReadDouble (AudioVisual) |
| `ReloadRate`     | +0x1508     | double | Ammo reload       | ReadDouble       |

**Notes:**
- There is NO `URepairStep` INI key in the binary. The unit repair step is computed from
  the unit's TypeClass via vtable+0xB4 (`GetRepairStep`), NOT from a global Rules value.
- `Rules+0x16F8` is used as the unit repair completion threshold in
  `TechnoClass::Receive_Radio` case 0x1C. It is NOT set by any INI key in ReadGeneral.
  It must use its constructor default, likely 1.0 (100% health). The service depot repairs
  units until `GetHealthRatio() >= Rules+0x16F8`. This is distinct from `RepairPercent`
  at +0x16D0, which controls building self-repair percentage.

### String Addresses

| String         | Address    |
|----------------|------------|
| "RepairRate"   | 0x0083BDD0 |
| "RepairStep"   | 0x0083BDE8 |
| "IRepairRate"  | 0x0083BDB8 |
| "IRepairStep"  | 0x0083BDDC |
| "URepairRate"  | 0x0083BDC4 |
| "RepairPercent"| 0x0083BDF4 |
| "ReloadRate"   | 0x0083BE6C |

---

## 8. Common Repair Accumulator Pattern

All three repair-type buildings (ServiceDepot, Hospital, Armory) share the same accumulator
pattern using BuildingClass fields:

| Field    | Purpose                                    |
|----------|--------------------------------------------|
| 0x620    | Repair accumulator (int, starts at 0)      |
| 0x624    | Step-happened flag (byte)                  |
| 0x628    | CDTimer start frame                        |
| 0x62C    | CDTimer param 2                            |
| 0x630    | CDTimer rate                               |
| 0x634    | Rate active flag (1=ticking)               |
| 0x638    | Step increment per timer tick               |
| 0x6DD    | Repair-in-progress flag                    |

### Accumulator Formula

```
Each timer tick:
  if (CDTimer expired AND field_0x634 != 0):
    field_0x620 += field_0x638
    reset timer

Threshold check:
  if (RulesRate * 900.0 <= field_0x620):
    fire repair/heal/promote action
    reset field_0x620 = 0

Where RulesRate is:
  Service Depot: Rules+0x16E8 (URepairRate)
  Hospital:      Rules+0x16F0 (IRepairRate)
  Armory:        Rules+0x16F0 (IRepairRate)  -- SAME as Hospital!
```

The constant **900.0** at `0x007E27F8` converts the rate from minutes to game ticks
(15 frames/sec * 60 sec = 900 ticks/minute).

---

## 9. Vtable Offset Summary

### BuildingClass Vtable (base: `0x007E3EBC`)

| Offset | Address    | Function                                  |
|--------|------------|-------------------------------------------|
| 0x084  | 0x006F3270 | TechnoClass::GetTechnoType                |
| 0x100  | 0x00443C60 | BuildingClass::Detach (mislabeled in Ghidra) |
| 0x184  | -          | MissionClass::GetCurrentMission           |
| 0x1B8  | 0x0041BEA0 | BuildingClass::GetCell                    |
| 0x1E8  | 0x005B35E0 | MissionClass::Queue_Mission               |
| 0x274  | 0x0065ACB0 | RadioClass::Transmit_Radio_ToFirst        |
| 0x278  | 0x0065AAA0 | RadioClass::Transmit_Radio                |
| 0x27C  | 0x0065A970 | RadioClass::Transmit_Radio_Impl           |
| 0x4D4  | 0x0044EFB0 | BuildingClass::GetDockCellForObject       |
| 0x4D8  | 0x00447E00 | BuildingClass::DistanceToObject           |

### UnitClass Vtable (base: `0x007F5C70`)

| Offset | Address    | Function                                  |
|--------|------------|-------------------------------------------|
| 0x084  | 0x006F3270 | TechnoClass::GetTechnoType                |
| 0x184  | 0x005B3040 | MissionClass::GetCurrentMission           |
| 0x1E8  | 0x005B35E0 | MissionClass::Queue_Mission               |
| 0x1F0  | 0x005B2FD0 | MissionClass::Set_Mission (force)         |
| 0x334  | 0x004DE580 | TechnoClass::UpdateReloadAnim             |
| 0x480  | 0x00741970 | TechnoClass::Set_Destination              |
| 0x484  | 0x00738970 | UnitClass::Scatter_Force                  |

---

## 10. Ghidra Labels Applied This Session

| Address    | New Name                                |
|------------|-----------------------------------------|
| 0x0074FFF0 | VeterancyStruct__IsRookie               |
| 0x007500B0 | VeterancyStruct__SetElite               |
| 0x00750090 | VeterancyStruct__SetVeteran             |
| 0x00447E00 | BuildingClass__DistanceToObject         |
| 0x0044EFB0 | BuildingClass__GetDockCellForObject     |
| 0x005B3A00 | MissionClass__GetMissionTimerEntry      |

### Functions Created

| Address    | Note                                    |
|------------|-----------------------------------------|
| 0x00447E00 | Was undefined, created function         |

---

## 11. Key Helper Functions

| Address    | Name                           | Purpose                          |
|------------|--------------------------------|----------------------------------|
| 0x00473430 | (unnamed)                      | Pop first from dock linked list  |
| 0x0050B730 | FUN_0050B730                   | Check if unit is player-owned in current game mode |
| 0x0053A130 | FUN_0053A130                   | Always returns 0 (stub/disabled feature) |
| 0x004F9790 | HouseClass::SpendMoney         | Deducts funds (credits first, then ore) |
| 0x005B3A00 | MissionClass::GetMissionTimerEntry | Returns timer table entry at 0x00A8E3A8 |

---

## 12. Mission Constants Referenced

| Value | Mission Name     |
|-------|------------------|
| 0     | MISSION_NONE     |
| 2     | MISSION_MOVE     |
| 5     | MISSION_GUARD    |
| 7     | MISSION_ENTER    |
| 0xC   | MISSION_RETREAT  |
| 0x10  | MISSION_UNLOAD   |
| 0x12  | MISSION_SELLING  |
| 0x13  | MISSION_REPAIR   |
| 0x14  | MISSION_MISSILE  |
| 0x1C  | MISSION_REPAIR_PRODUCE |

## 13. Radio Command Summary

| Code | Name (inferred)         | Sender      | Handler                    | Return Values            |
|------|------------------------|-------------|----------------------------|--------------------------|
| 0x13 | RADIO_NEED_TO_MOVE     | Building    | FootClass::Receive_Radio   | 1=linked, 10=busy        |
| 0x1C | RADIO_REPAIR_ONE_STEP  | Building    | TechnoClass::Receive_Radio | 1=ok, 0x20=broke, 0x21=done |
| 0x1D | RADIO_REQUEST_RELOAD   | Building    | Handled in UnitReload path | 1=accepted               |
| 0x1F | RADIO_RELOAD_AMMO      | Building    | TechnoClass::Receive_Radio | 1=reloaded, 10=full      |
| 0x20 | RADIO_CANT_AFFORD      | Unit->Bldg  | Response code from 0x1C    | (not a command, a reply) |
| 0x21 | RADIO_REPAIR_COMPLETE  | Unit->Bldg  | Response code from 0x1C    | (not a command, a reply) |
