# Mission_Guard and Mission_AreaGuard — Ghidra Research Report

**Primary addresses:**
- `FootClass::Mission_Guard` @ `0x004D5070`
- `FootClass::Mission_AreaGuard` @ `0x004D6AA0`
- `UnitClass::Mission_Guard_Harvester` @ `0x00740810` (Guard override for vehicles)
- `UnitClass::ScanForTiberium_SlaveMiner` @ `0x00744100` (AreaGuard override for vehicles)
- `AircraftClass::Mission_Guard` @ `0x0041A5C0`
- `BuildingClass::Mission_Guard` @ `0x004496B0`
- `BuildingClass::Mission_AreaGuard` @ `0x00449A40`
- `MissionClass::Mission_Dispatch` @ `0x005B3060` (authoritative enum → vtable slot mapping)
- `MissionControlClass` timer table @ `0x00A8E3A8`
- Mission name pointer table @ `0x00816CAC`

**Confidence:** HIGH (all addresses, offsets, slot mappings verified by decompilation and vtable xrefs)
**Active in YR:** Yes for all handlers. One TS-legacy quirk noted (see §7).

**Revision history:**
- 2026-04-23 initial.
- 2026-04-23 follow-up pass: corrected `GuardRange` / `Sight` / `AirRangeBonus` offsets
  (they were wrong in prior docs, including `TARGET_ACQUISITION_GHIDRA_REPORT.md`);
  resolved `vtable+0x31C` semantics; resolved the close-combat distance constants;
  corrected the AI auto-Sabotage gate from "Assaulter" to `C4=`; identified the
  `BuildingTypeClass` flag cluster at `+0x16A9..+0x16C3`; established that
  `vtable+0x478` ("`ScanForThreats_Simple`") is a universal no-op stub across all
  classes, not a scanner.
- 2026-04-23 final sweep: resolved the three remaining unknown flags —
  `HasStupidGuardMode=` (BuildingType+0x16B5), `CloseRange=` (TechnoType+0x695),
  `DistributedFire=` (TechnoType+0x6B0); also `BridgeRepairHut=` at +0x16B6.
  Identified the four sites that assign `Mission_Area_Guard` (enum 11); it is
  the **AI idle default**, not a player-facing mode, and player-controlled
  units default to `Mission_Guard` instead.

---

## 1. Overview

`Mission_Guard` (enum 5) and `Mission_Area_Guard` (enum 11) are the two "idle behavior"
missions that every non-sleeping unit/building lives in when not under an explicit
player order. The distinction between them is **not** behavior kind but behavior
*scope*: Guard uses the unit's weapon range, Area Guard uses an extended scan radius
and — for vehicles — also folds in ore-seeking / refinery-return logic.

Both are dispatched from `MissionClass::Mission_Dispatch` via the MissionClass
sub-vtable of the object. Classes override the handler they need; those that don't
override it fall through to a stub that simply returns `0x1C2` (450 frames ≈ 30 s at
15 fps) to idle until the next dispatch.

---

## 2. Mission enum and vtable slot mapping

### 2.1 The actual enum (verified from the mission-name pointer table @ `0x00816CAC`)

**CAUTION:** gamemd.exe's mission enum does NOT match YRpp. The binary retains a
Tiberian-Sun-era `Eaten` entry at index 9, which shifts every subsequent mission
by one compared to the "clean" YRpp enum. This is a common trap — when reading
YRpp or ModEnc, always add 1 to mission indices at/after Harvest when cross-
referencing with Ghidra decompiles.

| Enum | Name             | Enum | Name              |
|------|------------------|------|-------------------|
| 0    | Sleep            | 16   | Unload            |
| 1    | Attack           | 17   | Sabotage          |
| 2    | Move             | 18   | Construction      |
| 3    | QMove            | 19   | Selling           |
| 4    | Retreat          | 20   | Repair            |
| 5    | **Guard**        | 21   | Rescue            |
| 6    | Sticky           | 22   | Missile           |
| 7    | Enter            | 23   | Harmless          |
| 8    | Capture          | 24   | Open              |
| 9    | Eaten *(TS)*     | 25   | Patrol            |
| 10   | Harvest          | 26   | Paradrop Approach |
| 11   | **Area Guard**   | 27   | Paradrop Overfly  |
| 12   | Return           | 28   | Wait              |
| 13   | Stop             | 29   | Attack Move       |
| 14   | Ambush           | 30   | Spyplane Approach |
| 15   | Hunt             | 31   | Spyplane Overfly  |

### 2.2 Enum → MissionClass-sub-vtable slot (from `Mission_Dispatch` @ `0x005B3060`)

The dispatcher checks `this->CurrentMission` (byte offset `0xAC`) and calls through
the MissionClass-sub-vtable. Several missions share a slot (e.g. Sticky aliases Guard):

| Case  | Mission             | Slot  |
|-------|---------------------|-------|
| 0     | Sleep               | 0x204 |
| 1     | Attack              | 0x210 |
| 2     | Move                | 0x22C |
| 3     | QMove *(→ default)* | 0x204 |
| 4     | Retreat             | 0x230 |
| **5** | **Guard**           | **0x21C** |
| **6** | **Sticky → Guard**  | **0x21C** |
| 7     | Enter               | 0x240 |
| 8     | Capture             | 0x214 |
| 9     | Eaten               | 0x218 |
| 10    | Harvest             | 0x224 |
| **11**| **Area Guard**      | **0x220** |
| 12    | Return              | 0x234 |
| 13    | Stop                | 0x238 |
| 14    | Ambush              | 0x20C |
| 15    | Hunt                | 0x23C |
| 16    | Unload              | 0x244 |
| 17    | Sabotage            | 0x248 |
| 18    | Construction → Capture | 0x214 |
| 19    | Selling             | 0x24C |
| 20    | Repair              | 0x254 |
| 21    | Rescue              | 0x250 |
| 22    | Missile             | 0x258 |
| 23    | Harmless *(→ Sleep)*| 0x204 |
| 24    | Open                | 0x208 |
| 25    | Patrol              | 0x25C |
| 26    | Paradrop Approach   | 0x260 |
| 27    | Paradrop Overfly    | 0x264 |
| 28    | Wait                | 0x26C |
| 29    | Attack Move         | 0x268 |
| 30    | Spyplane Approach   | 0x270 |
| 31    | Spyplane Overfly    | — (missing — ? re-check)|

### 2.3 Guard / AreaGuard handler per class

| Class           | Guard slot (0x21C)                     | AreaGuard slot (0x220)                                 |
|-----------------|----------------------------------------|--------------------------------------------------------|
| MissionClass    | stub → 450f                            | stub → 450f                                            |
| TechnoClass     | *(inherits MissionClass stub)*         | *(inherits MissionClass stub)*                         |
| **FootClass**   | `Mission_Guard` @ `0x004D5070`         | `Mission_AreaGuard` @ `0x004D6AA0`                     |
| UnitClass       | `Mission_Guard_Harvester` @ `0x00740810` | `ScanForTiberium_SlaveMiner` @ `0x00744100`          |
| InfantryClass   | *(inherits FootClass)*                 | *(inherits FootClass)*                                 |
| AircraftClass   | `Mission_Guard` @ `0x0041A5C0`         | *(inherits MissionClass stub — aircraft never AreaGuard)* |
| **BuildingClass** | `Mission_Guard` @ `0x004496B0`       | `Mission_AreaGuard` @ `0x00449A40` (tail-calls Guard) |

---

## 3. Core logic

### 3.1 `FootClass::Mission_Guard` (`0x004D5070`)

Used by infantry; vehicles layer `Mission_Guard_Harvester` on top (§3.4). Stripped
pseudocode — `param_1` is `int *` (so `param_1[N]` is byte offset `N*4`; `(int)param_1+N`
is direct byte offset):

```pseudo
// Phase 1 — sub-state dispatch (each of these three flags is mutually exclusive
// and wins over everything that follows)
if (byte+0x68F)  return vtable+0x340(this);   // IsReceivingRepair  → RepairAI
if (byte+0x690)  return vtable+0x348(this);   // IsDockingToBuilding → DockingAI
if (byte+0x691)  return vtable+0x34C(this);   // IsWeedingHarvester  → WeedHarvestAI

// Phase 2 — target exists: maintain target, honor DefaultToGuardArea re-anchor
if (this->TarCom /* +0x2B4 */ != 0) {
    type = this->Class_Of();              // vtable+0x84
    if (type->DefaultToGuardArea /* +0x390 */
        && this->Locomotion_Is_Idle())    // vtable+0x1C8
    {
        // Pick a passable cell near home base and set as Set_Destination(cell, 1)
        home_coord = FUN_00703590(this);  // ComputeHomeCellForUnit
        cell = MapClass::Get_CellClass(home_coord);
        this->Set_Destination(cell, 1);   // vtable+0x480
    }
    // else: keep current target, let combat/attack code handle it
}

// Phase 3 — no target: 8-cell garrison scan (only if we have a CanTarget weapon)
else {
    w = this->GetWeapon(1);               // vtable+0x3F8
    if (w == null || !w->Type->CanTarget /* TypeClass+0x158 */) {
        vtable+0x478(this);               // ScanForThreats_Simple  (base stub in FootClass;
                                          //   overridden in TechnoClass/InfantryClass)
    } else {
        for (i = 0; i < 8; i++) {
            cell = Get_Coord() + DirectionOffsets[i];   // 8-neighbour ring
            b    = LookUpBuildingInCell(cell);
            if (b && b->Class->CanBeOccupied /* +0x1575 */
                 && b->Owner() == this->Owner())
            {
                this->Set_ArchiveTarget(b);             // vtable+0x3C8
                this->byte+0x68E = 1;                   // HasFoundAutoTarget
                this->Queue_Mission(Enter=7, false);    // vtable+0x1E8
                goto timer_return;
            }
        }
        vtable+0x478(this);               // no garrison — fall through to simple scan
    }
}

// Phase 4 — AI infantry auto-sabotage (TS-engine holdover, still live in YR)
if (this->What_Am_I() == InfantryClass_RTTI (0xF)
    && !Owner->IsPlayerControl()
    && (this->InfantryType->Assaulter /* InfantryType+0xEC2, via cached ptr +0x6C0 */
        || this->HasWeaponAbility(0xE /* auto-attack structures */))
    && this->CurrentMission != Sabotage (0x11)
    && this->TarCom != 0
    && this->TarCom->What_Am_I() == BuildingClass_RTTI (6))
{
    this->Queue_Mission(Sabotage=0x11, false);
}

// Phase 5 — mission timer return
timer_return:
base = ftol(GetMissionTimerEntry(this)->Rate * 900);    // MissionControl table
if (this->MissionTimerStart /* +0x2EC */ != -1) {
    elapsed = CurrentFrame - MissionTimerStart;
    if (elapsed >= this->MissionTimerDuration /* +0x2F4 */) goto default_return;
    // otherwise hold until timer fires
    return MissionTimerDuration - elapsed;
}
default_return:
// Harvester-like fast re-dispatch when ammo present
if (type->byte+0x6B0 && this->Ammo /* +0x468 */ >= 1) return 0;
return base + RandomRanged(0, 2);
```

### 3.2 `FootClass::Mission_AreaGuard` (`0x004D6AA0`)

The largest mission handler in FootClass (~900 bytes). Implements both idle patrol
*and* full harvester autonomous behavior — an important design fact: **every ground
unit in Area_Guard, including harvesters, flows through this single handler**. That
explains why the function looks "harvester-shaped" — harvesters are the heaviest
consumers, but the same code runs for idle infantry and combat vehicles.

Twelve-phase pipeline (condensed):

1. **Deploy gate.** `if (byte+0x2E4 != 0) Queue_Mission(Guard=5, true); return 1;`
   — a unit that is currently deploying cannot AreaGuard; bounce it to Guard.
2. **Sub-state dispatch** (same three as Guard phase 1 — Repair/Dock/Weed).
3. **AI harvester EnterQueue drain (not player-controlled):** if the unit has
   pending enter-queue entries (`+0x5BC > 0`) with no active NavCom path, and the
   queued destination matches NavCom, issue `Set_Destination(queued, 1)` and call
   `FUN_0045ADD0` (dock approach). If `SelfEnterQueued` (`+0x6B1`) is set, invoke
   deploy helper `FUN_004A7FE0`.
4. **Refinery proximity.** When NavCom target is a Building (`RTTI == 0xB`) owned
   by an ally, look up a passable cell near the refinery and set it as GhostCell
   (`+0x5DC`). Excluded for slave miner infantry (`InfantryType+0xE0E != 0`).
5. **Slave miner re-queue.** Infantry with the slave-miner flag queues
   `Mission_Harvest(10)`, commences, returns `Random(1,...) + 1`.
6. **Ore-collection trigger.** If `+0x2D8 != 0` (SlaveManager pointer),
   `SlaveManagerClass::RecallAllSlaves`.
7. **Idle fallback.** If NavCom and QueuedMission are both absent, set current cell
   as GhostCell so the unit holds position.
8. **AI auto-sabotage.** Same rule as Guard phase 4, returns 1.
9. **Approach-timer computation.**
    - `base = ftol(vtable+0x31C(1) * 900)` (uses "GuardRange" accessor, not weapon).
    - When a NavCom target exists with the "bridge-stuck" bit (`byte+0x14 & 4`),
      clamp `base` to `RulesClass->ConditionRed` (`+0x1724`).
10. **Target distance trim.** If `!HasReachedDock (+0x68D)` and no NavCom path
    active, measure distance to target; if `FUN_005F6360(target) > base`, clear
    target (`Set_ArchiveTarget(0)`) and `Set_Destination(0)`. Essentially: if the
    chosen target moved out of our area-guard tether, give up on it.
11. **Target search.** If no TarCom: try ore scan (`FUN_007091D0` → `FUN_0070F7E0`
    → `vtable+0x39C`), otherwise `ScanForThreats_Simple` (vtable+0x478). If TarCom
    is set, call `vtable+0x53C(0)` (attack-approach helper).
12. **Garrison scan.** Same 8-neighbour CanBeOccupied scan as Guard phase 3.
13. **Timer return.**
    - `base = ftol(MissionTimerEntry->Rate * 900)`.
    - If unit is AircraftClass (`RTTI == 2`), `base *= 2`.
    - `base += Random(1, 5)`.
    - Close-range boost: if TarCom exists AND (this is infantry with
      `InfantryType+0x695` close-combat flag OR primary weapon range `< 0x201`),
      compute distance to target; if `< 0x301` and `>= _DAT_007E9228` (a mid-range
      floor), divide `base` by 6 to re-dispatch quickly while closing.

### 3.3 `AircraftClass::Mission_Guard` (`0x0041A5C0`)

Airborne-specific. Branches on ammo, fuel and ammo-max to manage return-to-base
behavior independent of ground Guard. Key path (paraphrased):

```pseudo
type_flag = this->AircraftType->byte+0xBC;
if (LocomotorIdle() == type_flag) {        // landed / awaiting take-off
    if (this->NavCom != 0) {
        if (this->TarCom) { byte+0x6D4 = 1; Queue_Mission(Move=2); }
        return TimerRate;
    }
    if (this->GetWeapon(0)->Type != 0) {
        byte+0x6D4 = 1;
        vtable+0x484(0, 1);                // request take-off
        return 1;
    }
    // no weapon → return to parking, retry Move
    Set_Destination(current_cell, 1);
    Queue_Mission(Move=2);  return 1;
}

// In the air
if (!SAM_GlobalEngaged && !SAM_AltMode) {
    need_rtb = !HasValidPath() && (Ammo == 0 || Ammo <= AmmoMax/2);
} else if (SAM_AltMode) {
    need_rtb = !HasValidPath() && (Ammo == 0 || Ammo <= AmmoMax/2);
} else {
    need_rtb = Ammo == 0 || !HasValidPath();
}
if (need_rtb) {
    nearest = FUN_...(AmmoMax * 1000 offset);
    if (nearest) {
        Queue_Mission(Enter=7);            // go dock
        Set_Destination(nearest, 1);
        Set_ArchiveTarget(0);
        return 1;
    }
}

// ammo half or less + valid path to an ally building that allows auto-attack
if (this->Ammo < AmmoMax/2 && HasValidPath()) {
    dest = GetDestination(0);
    if (dest->RTTI == 6 /* BuildingClass */
        && dest->BuildingType->byte+0x16AA /* permits auto-strike */)
        return 1;
}

if (TarCom != 0) { Queue_Mission(Attack=1); return 1; }

if (vtable+0x2AC() == 0) return 0x2D;      // can't-engage throttle (45 frames)
if (LocomotorIdle() == 0 && !HasValidPath()) return 0x2D;

if (!Owner->IsPlayerControl()) {
    ally = HouseClass::Find_Nearest_Ally_Building(cell);
    if (ally) { Set_ArchiveTarget(ally); Queue_Mission(Enter=7); }
}

if (Owner->IsPlayerControl() && vtable+0x54(this) == 0)
    return ftol(TimerEntry->Rate * 900) + Random(0,2);

return FootClass::Mission_Guard();        // delegate to ground handler
```

AircraftClass does **not** override `Mission_AreaGuard` — aircraft never enter
AreaGuard in normal YR play (the Ctrl+Alt "area guard" click produces a different
dispatch for air units via `AttackMove`/`Patrol`).

### 3.4 `UnitClass::Mission_Guard_Harvester` (`0x00740810`)

Vehicle override for the Guard slot. Adds three behaviors before delegating:

1. **Slave idle recall.** If we have a SlaveManager pointer (`+0x2D8`) and
   `RulesClass+0x1790 (SlaveMinerKickFrameDelay, default 150)` has elapsed since
   our slaves were last checked, and `ShouldRecallSlaves` returns true, recall and
   return `TimerRate + Random(0,2)`.
2. **Bail-if-no-ore queue-Harvest.** If the UnitType is a harvester
   (`UnitType+0xE0E`) or slave miner (`UnitType+0xE0F`), and the house lacks
   refinery prerequisites, scan owned building types: if none would accept us,
   we Queue_Mission(Harvest=10) so the harvester stops trying to dock at an
   absent refinery.
3. **Unload-on-arrival.** If `UnitType+0xE0F` (slave-miner chassis) and
   `byte+0x6B8` (SelfEnterQueued-equivalent) is set, queue Harvest=10.

Then: `return FootClass::Mission_Guard();` — all the idle/target-scan/garrison
logic comes from the FootClass base.

### 3.5 `UnitClass::ScanForTiberium_SlaveMiner` (`0x00744100`)

Vehicle override for the AreaGuard slot. Simpler than Guard_Harvester — just a
recall gate, then delegate:

```pseudo
if (this->+0x2D8 != 0
    && RulesClass->SlaveMinerKickFrameDelay (+0x1790) + this->+0xC0 < CurrentFrame)
{
    if (SlaveManager::ShouldRecallSlaves()) {
        SlaveManager::RecallAllSlaves();
        return ftol(TimerRate * 900) + Random(0,2);
    }
}
return FootClass::Mission_AreaGuard();
```

### 3.6 `BuildingClass::Mission_Guard` (`0x004496B0`)

**This contradicts prior research notes that called BuildingClass Guard a stub** —
it is a substantive handler for defensive structures. Key behavior:

```pseudo
type = this->Class_Of();
if (type->byte+0xCD5 /* Gattling */) {
    TechnoClass::UpdateGattlingStage();
    this->+0xC4 = 0;                              // reset some stage counter
}

if (vtable+0x2AC(this) == 0) {                    // no weapon?
    if (type->byte+0x16B5) return 100;            // defensive structure throttle
    if (this->+0xBC == 0) {                       // grand-opening state
        BuildingClass::GrandOpening();
        this->+0xBC = 1;
    } else if (this->+0xBC == 1) {
        if (type->byte+0x16A9 || type->byte+0x16AA || type->byte+0x16AB) {
            // hospitals/kennels/etc. eject passengers when they arrive
            foreach passenger in this->+0xE8..+0xE8+count {
                if (passenger->Current_Mission() == Enter=7
                    && distance_to(this, passenger) < 0x40)
                {
                    if (vtable+0x278(0x13)) {     // slot accepts passenger
                        Queue_Mission(Unload=0x14);
                        return 1;
                    }
                }
            }
        }
        if (type->byte+0x16BD) BuildingClass::ClearBibArea();
    }
    if (type->byte+0x16AA) {                      // passive-acquire-capable
        if (HasValidSteps() && vtable+0x274(this) != 1)
            // rarely-taken branch; re-enter Unload
            Queue_Mission(Unload=0x14);
    }
    if (type->byte+0x16A9) {
        return ftol(Rate * 900) + Random(0,2);    // normal idle
    }
    return ftol(Rate * 900) * 3 + Random(0,2);    // 3x idle for inert buildings
}

// Has weapon — defensive structure
this->byte+0x6DD = 1;                             // armed-and-watching
if (type->byte+0x16C3) goto timer;                // IsBaseDefense-like, skip target check
if (type->UnitReload /* +0x16F0 */ != -1) {
    if (HouseClass::CountOwnedInstances(...) != 0) { /* operator present */ }
    else if (type->byte+0x157B && vtable+0x408() < 1) goto timer;  // need operator
}
if (this->TarCom != 0) {
    Queue_Mission(Attack=1);
    MissionClass::Commence();                     // flush immediately
    return 1;
}
timer:
return ftol(Rate * 900) + Random(0,2);
```

Headline: defensive structures with weapons perform automatic target acquisition
during Guard (they don't need AreaGuard). Buildings without weapons handle
passenger/bib/grand-opening housekeeping and nothing more.

### 3.7 `BuildingClass::Mission_AreaGuard` (`0x00449A40`)

Pure thunk:

```
FUN_00449a40:
    JMP  dword ptr [ECX + 0x21C]     // = this->vtable+0x21C (Mission_Guard)
```

Buildings have no distinct AreaGuard — all area-guard logic collapses into their
own Mission_Guard.

---

## 4. Target acquisition — `TechnoClass::Greatest_Threat` (`0x006F8DF0`)

Called from `ScanForThreats_Simple` (see §4.3 — that callee is a no-op; real
acquisition is kicked off elsewhere) and from the Area_Guard timer math. This is
where `GuardRange` actually lives.

**Offset correction from prior docs:** `TARGET_ACQUISITION_GHIDRA_REPORT.md` and
`FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` placed `GuardRange` at TypeClass+0x68C
and `Sight` at +0x5B8. That is **wrong**. Direct disassembly of
`TechnoTypeClass::ReadINI` (@ `0x00712170`; EBP=this throughout) shows:

- `GuardRange=` (INI str @ `0x008444A4`) → TechnoType **`+0x5B8`** (int, leptons)
  — read at `0x007122A4` / stored at `0x007122B8`
- `AirRangeBonus=` (INI str @ `0x00843AD4`) → TechnoType **`+0x68C`** (int, leptons)
  — read at `0x00714794` / stored at `0x007147AE`
- `Sight=` (INI str @ `0x00843D88`) → TechnoType **`+0x5E8`** (int, leptons)
  — stored at `0x007142A7`
- `CanPassiveAquire=` (INI str @ `0x00843C50`, yes, the misspelling is in the INI
  key itself) → TechnoType **`+0xD99`** (bool) — stored at `0x00714480`

### 4.1 `vtable+0x31C` — scan-range accessor (`FUN_00707E60`)

This is the entry point all the scan code goes through. Semantics by argument:

```pseudo
int GetScanRange(int mode) {
    if (mode == -1) return -1;

    if (mode == 0) {                              // "sensor / reveal" mode
        if (Sensor_Is_Disabled())  return 0;      // vtable+0x330
        if (TypeClass->GuardRange (+0x5B8) != 0)  return TypeClass->GuardRange;
        return 0;
    }

    // mode == 1 (Guard scan) or mode == 2 (Paradrop scan)
    leptons = TypeClass->GuardRange (+0x5B8);
    if (leptons == 0) {
        leptons = max(Weapon_Range(0), Weapon_Range(1));   // vtable+0x168
    }
    leptons *= 2;

    if (mode == 2) return clamp(leptons, 0x700, 0x1000);   //  7..16 cells floor
    else           return clamp(leptons, 0,     0x1000);   //  0..16 cells cap
}
```

Key facts to internalize:

- The accessor **doubles** the `GuardRange` INI value. A unit with `GuardRange=5`
  (cells) scans out to **10 cells**, capped at 16 (`0x1000`). This is the
  operative knob.
- When `GuardRange=` is omitted, the scan uses the greater of the unit's two
  weapon ranges, also doubled. So a default unit's scan radius is
  `2 × max(weapon_range)`.
- `AirRangeBonus` (+0x68C) is a **separate** bonus that only contributes inside
  `Greatest_Threat`'s degenerate fallback, not through this accessor. In
  practice most units have `AirRangeBonus=0` and it never shows up.

### 4.2 `TechnoClass::Greatest_Threat` scan radius (cells)

```pseudo
// threat_flags bit 0 = "use weapon range", bit 1 = "use guard range"
if (threat_flags & 1)       scan_leptons = GetScanRange(0);
else if (threat_flags & 2) {
    if (CurrentMission != ParadropApproach (0x19))
        scan_leptons = GetScanRange(1);
    else
        scan_leptons = GetScanRange(2);
}

if (TechnoClass::GetWeaponRange(-1) < 0 && CurrentMission == Guard (5)) {
    scan_leptons = 0x200;                         // weaponless-Guard hard
                                                  //   fallback: 2 cells
}

scan_cells = scan_leptons >> 8;

if (scan_leptons == 0) {                          // degenerate fallback
    weap = max(Weapon_Range(0), Weapon_Range(1));
    scan_cells = (weap >> 8) + 1 + (TypeClass->AirRangeBonus >> 8);
}
```

### 4.3 `vtable+0x478` — "`ScanForThreats_Simple`" is a no-op

Every class (FootClass, AircraftClass, UnitClass, InfantryClass, BuildingClass)
points slot `+0x478` at the same 3-byte stub:

```
0x0041C040:   32 C0        XOR AL, AL
0x0041C042:   C3           RET
```

Xref-verified: the stub is referenced from five vtable slots (0x007E271C,
0x007E4334, 0x007E910C, 0x007F4DD8, 0x007F60E8) — one per class — and nowhere
else. **Calling `vtable+0x478` does nothing and returns 0.** Every place in
`Mission_Guard` / `Mission_AreaGuard` described as "`ScanForThreats_Simple`" in
prior research is actually firing this no-op.

Why does the slot exist, then? It is the Westwood-legacy hook for a base class
whose override was later removed or never written. Automatic target acquisition
in stock YR actually lives in the retaliation path, in the house-AI periodic
scan, and in the target-line drawing logic — **not** in a Guard-time scanner.

### 4.4 Close-combat distance constants (`FootClass::Greatest_Threat_Scan`)

Decoded doubles in the `.rdata`-adjacent constant block:

| Address        | Value (leptons) | Value (cells) | Role                                          |
|----------------|-----------------|---------------|-----------------------------------------------|
| `0x007E9228`   | 281.6           | 1.1           | AreaGuard close-approach floor (timer/6 gate) |
| `0x007E9230`   | 307.2           | 1.2           | Melee ring mid-step                           |
| `0x007E9238`   | 204.8           | 0.8           | Melee spiral inner radius                     |
| `0x007E9240`   | 384.0           | 1.5           | Melee direct-LOS cutoff                       |
| `0x007E9248`   | 332.8           | 1.3           | Close-combat outer ring (0x14C ≈ 332)         |

These are the knobs that give dogs / Terror Drones / Yuri Clones their tight
engage-from-idle behavior. A unit with `TypeClass+0x695` (close-combat flag)
takes the melee-ring path; everything else uses the normal spiral.

### 4.5 INI-key map (correction table)

| TechnoType offset | INI key             | Used by                                        |
|-------------------|---------------------|------------------------------------------------|
| **`+0x5B8`**      | **`GuardRange=`**   | `vtable+0x31C` (the primary scan-radius knob)  |
| `+0x5E8`          | `Sight=`            | shroud-reveal radius / passive detection       |
| **`+0x68C`**      | **`AirRangeBonus=`**| `Greatest_Threat` degenerate fallback bonus    |
| `+0x390`          | `DefaultToGuardArea=` | Guard phase 2 home-anchor                    |
| `+0xD99`          | `CanPassiveAquire=` | gates `FUN_007091D0` (Can-Passive-Acquire)     |

Prior research (`TARGET_ACQUISITION_GHIDRA_REPORT.md` §2, `FOOTCLASS_*` reports)
had `Sight=` / `GuardRange=` at `+0x5B8` / `+0x68C` — **reversed** from reality.
The correction above is verified by `TechnoTypeClass::ReadINI` disassembly.

### 4.2 Mission-sensitive branches inside `FootClass::Greatest_Threat_Scan`
(`0x004D5690`)

```pseudo
if (this->CurrentMission == Area_Guard (0xB) && Owner->IsPlayerControl())
    cVar22 = true;                                // widen acceptance criteria
if (this->CurrentMission == Hunt (0xF) && !Owner->IsPlayerControl())
    fall-through to search;
```

Area_Guard's search loop uses a two-pass walk: first a direct line-of-sight ring
at `_DAT_007E9240` distance, then a spiral outward ring-by-ring up to
`scan_cells`. Unit-type `+0x695` (close-combat / melee marker) shortens the outer
radius to `0x14C` (1.3 cells) — this is how dogs and Terror Drones get their
tight engage-from-idle behavior.

---

## 5. INI keys and their runtime wiring

### 5.1 `[General]` — global knobs

| Key                     | Default | Runtime offset / use                                     |
|-------------------------|---------|----------------------------------------------------------|
| `GuardModeStray`        | 2.0     | How far a follow-Guard unit may stray from its escort    |
| `GuardAreaTargetingDelay` | 36    | Per-tick target-scan spacing while Area_Guard is active  |
| `NormalTargetingDelay`  | 27      | Per-tick target-scan spacing in normal combat            |
| `SlaveMinerKickFrameDelay` | 150 | `RulesClass+0x1790` — gate in Guard_Harvester slave recall|
| `ConditionRed`          | —       | `RulesClass+0x1724` — bridge-stuck timer override in AreaGuard |

### 5.2 Per-object INI (verified against ReadINI disassembly)

**TechnoTypeClass** (all unit/building types share these fields):

| Key                    | Offset         | Type  | Effect                                                 |
|------------------------|----------------|-------|--------------------------------------------------------|
| `GuardRange=`          | TechnoType+0x5B8 | int | Scan-radius basis for Guard/AreaGuard (see §4.1)      |
| `Sight=`               | TechnoType+0x5E8 | int | Shroud-reveal / passive detection radius (leptons)     |
| `AirRangeBonus=`       | TechnoType+0x68C | int | `Greatest_Threat` degenerate-fallback additive (cells) |
| `DefaultToGuardArea=`  | TechnoType+0x390 | bool| Guard phase 2 re-anchor toward home base              |
| `CanPassiveAquire=`    | TechnoType+0xD99 | bool| Master gate for passive target acquisition (§ `FUN_007091D0`) |
| `CanRetaliate=`        | (per-mission `MissionControlClass`, see §5.3) | bool | Fire-back when attacked |
| `Slaved=`              | TechnoType+0xD3E | bool| Unit is a slave (owned by a SlaveManager)             |

**InfantryTypeClass** (verified in `InfantryTypeClass::ReadINI` @ `0x005240A0`,
where ESI = `this`):

| Key             | Offset          | Type | Effect                                                      |
|-----------------|-----------------|------|-------------------------------------------------------------|
| `Assaulter=`    | InfantryType+0xEB5 | bool | Grants weapon-ability 0xE (auto-attack structures)     |
| `C4=`           | InfantryType+0xEC2 | bool | **Gates AI auto-Sabotage branch in Guard/AreaGuard** — Crazy Ivan / Terrorist / Tanya |
| `Agent=`        | InfantryType+0xEC4 | bool | Spy class                                                 |
| `Thief=`        | InfantryType+0xEC0 | bool | (Thief-style infiltration)                                |

**Correction of prior docs:** The AI auto-Sabotage gate in `Mission_Guard` phase
4 and `Mission_AreaGuard` phase 8 — `*(char *)(param_1[0x1b0] + 0xEC2) != 0` —
was claimed to be `Assaulter` in `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`.
It is actually **`C4=`**. `Assaulter=` lives at `+0xEB5` and is OR'd into the
same condition via `HasWeaponAbility(0xE)` (which C4-capable units also have).

**UnitTypeClass** (verified in `UnitTypeClass::ReadINI` @ `0x00747620`, where
EDI = `this`):

| Key             | Offset         | Type | Effect                                              |
|-----------------|----------------|------|-----------------------------------------------------|
| `Harvester=`    | UnitType+0xE0E | bool | **AreaGuard phase 5 Harvest reroute** (was mislabeled "SlaveMiner" in prior docs) |
| `Weeder=`       | UnitType+0xE0F | bool | Tiberian weed gatherer (TS-legacy but still read)  |

The Mission_Guard "fast re-dispatch when Ammo≥1" check (`TechnoType+0x6B0`)
was initially thought to be a harvester-only knob. In fact it is
**`DistributedFire=`** (verified from `TechnoTypeClass::ReadINI` store at
`0x00714864`). Meaning: units with `DistributedFire=yes` (e.g., burst-weapon
platforms designed to spread fire across targets) **bypass the
`Random(0,2)`-frame timer jitter** in Guard phase 5 when they still have
ammo (`ammo ≥ 1`), getting immediate re-dispatch. This keeps them shooting
tightly without the small idle-tick backoff that normally smooths out the
mission scheduler. Examples in `rulesmd.ini`: `DistributedFire=yes` on units
where each shot should seek a fresh target.

**BuildingTypeClass** (verified in `BuildingTypeClass_ReadINI_Water` @
`0x0045FE50`; the misleading Ghidra label is because the function also
includes naval-variant reads). All of these are bool flags (`yes`/`no`):

| INI key           | Offset               | Used in Mission_Guard path?                          |
|-------------------|----------------------|-----------------------------------------------------|
| `UnitRepair=`     | BuildingType+0x16A9  | Yes — triggers passenger-Enter scan                 |
| `UnitReload=`     | BuildingType+0x16AA  | Yes — triggers passenger-Enter scan + passive path  |
| `Bunker=`         | BuildingType+0x16AB  | Yes — triggers passenger-Enter scan                 |
| `Cloning=`        | BuildingType+0x16AC  | —                                                   |
| `Grinding=`       | BuildingType+0x16AD  | —                                                   |
| `UnitAbsorb=`     | BuildingType+0x16AE  | —                                                   |
| `InfantryAbsorb=` | BuildingType+0x16AF  | —                                                   |
| `SecretLab=`      | BuildingType+0x16B0  | —                                                   |
| `DockUnload=`     | BuildingType+0x16B7  | —                                                   |
| `SAM=`            | BuildingType+0x16B8  | —                                                   |
| `ConstructionYard=` | BuildingType+0x16B9 | —                                                 |
| `NukeSilo=`       | BuildingType+0x16BA  | —                                                   |
| `Refinery=`       | BuildingType+0x16BB  | —                                                   |
| `WeaponsFactory=` | BuildingType+0x16BD  | (guarded by `ClearBibArea` flow, not identity)      |
| `LaserFencePost=` | BuildingType+0x16BE  | —                                                   |
| `LaserFence=`     | BuildingType+0x16BF  | —                                                   |
| `FirestormWall=`  | BuildingType+0x16C0  | —                                                   |
| `Hospital=`       | BuildingType+0x16C1  | —                                                   |
| `Armory=`         | BuildingType+0x16C2  | —                                                   |
| `EMPulseCannon=`  | BuildingType+0x16C3  | — (TS-legacy; the "always hunt" behavior in `BuildingClass::Mission_Guard` gates on this flag) |
| `GDIBarracks=`    | BuildingType+0x16E4  | —                                                   |
| `NODBarracks=`    | BuildingType+0x16E5  | —                                                   |
| `YuriBarracks=`   | BuildingType+0x16E6  | —                                                   |
| `HasStupidGuardMode=` | BuildingType+0x16B5 | **Yes — gate for the 100-frame early-return path in BuildingClass::Mission_Guard** (store site `0x00460EB4`) |
| `BridgeRepairHut=` | BuildingType+0x16B6 | — (identity flag for the bridge-repair structure) |

**`HasStupidGuardMode=`** merits a specific note: in all YR stock content it is
set to `false` on every building that sets it at all. Grepping `rulesmd.ini`
turns up only `HasStupidGuardMode=false` entries. The "returns 100 frames
early" branch in `BuildingClass::Mission_Guard` is therefore **effectively dead
code** in a standard YR skirmish. It exists as a hook for a simple
stand-and-shoot AI that TS used on some defensive structures, carried forward
but never enabled.

**Re-analysis of `BuildingClass::Mission_Guard` with real key names:**

```pseudo
// The "if any of these, scan for arriving passengers to Unload" gate
if (type->UnitRepair || type->UnitReload || type->Bunker) {
    for each passenger queued to enter {
        if (distance < 0x40 && Mission==Enter) Queue_Mission(Unload);
    }
}

// The "always-on threat check" path
if (type->EMPulseCannon) goto timer;              // skip operator check entirely
if (type->UnitReload) {                           // reload-capable garrison
    // maintain Unload if the queue is active
}

// Key takeaway: UnitRepair, UnitReload, Bunker are the three flags that make a
// building behave like a passive-garrison dock. Concrete examples in YR:
//   - UnitRepair=yes  : Service Depot
//   - UnitReload=yes  : Naval Yard (for AA ships), airfields-like structures
//   - Bunker=yes      : Bunker (TS-legacy, rarely set in YR content)
```

### 5.3 `MissionControlClass` table (`0x00A8E3A8`, loaded by `Read_INI` @ `0x005B3760`)

Each entry is populated from a `[<MissionName>]` INI section using the string
from the mission-name pointer table (e.g. `[Guard]`, `[Area Guard]`). Layout:

```
MissionControlClass {   // size = 32 bytes, indexed by mission enum
    +0x00  int    MissionID
    +0x04  bool   NoThreat       // "NoThreat="
    +0x05  bool   Zombie         // "Zombie="
    +0x06  bool   Recruitable    // "Recruitable="
    +0x07  bool   Paralyzed      // "Paralyzed="
    +0x08  bool   Retaliate      // "Retaliate="
    +0x09  bool   Scatter        // "Scatter="
    +0x10  double Rate           // "Rate="  — seconds; handler ftol(Rate * 900) → frames
    +0x18  double AARate         // "AARate=" — AA variant; defaults to Rate if 0
}
```

`Rate * 900` yields 15-fps-adjusted frames: at `Rate=0.0167` (≈1/60), the resulting
`~15` frames mean "re-dispatch this mission once per second".

---

## 6. Integration points

**Upstream — who assigns these missions?**
- `MissionClass::Assign_Mission(enum)` (@ `0x005B2FD0`) — default path. One guard:
  a unit currently in `AttackMove` (enum 28 / `0x1C`) **refuses** to be reassigned
  to Guard (5). This is the anti-churn that keeps AttackMove resilient.
- `MissionClass::Queue_Mission(enum, override)` (@ `0x005B35E0`) — queue for next
  Commence.
- Unit creation / rally completion / house AI / map script actions all route through
  these.

**Downstream — who do the handlers call?**
- Core vtable methods used (slot → meaning, roughly):
  - `+0x2C` `What_Am_I` — RTTI (`0xF`=Infantry, `0xB`=Unit, `6`=Building, `2`=Aircraft)
  - `+0x3C` `Owner` (HouseClass*)
  - `+0x54` selection state
  - `+0x84` `Class_Of`
  - `+0x168` weapon range query (`(0)`=primary, `(1)`=secondary)
  - `+0x184` `Current_Mission` (same as reading `+0xAC`)
  - `+0x1B8` `Get_Coord`
  - `+0x1C8` `Locomotion_Is_Idle`
  - `+0x1E8` `Queue_Mission`
  - `+0x274`/`+0x278` state queries
  - `+0x2AC` `Is_Weapon_Equipped`
  - `+0x31C` scan-range accessor (0/1/2 selector)
  - `+0x3C8` `Set_ArchiveTarget`
  - `+0x3F8` `GetWeapon(index)`
  - `+0x478` `ScanForThreats_Simple`  *(stub in FootClass — subclasses override)*
  - `+0x480` `Set_Destination`
  - `+0x484` aircraft take-off request
  - `+0x340` `RepairAI`, `+0x348` `DockingAI`, `+0x34C` `WeedHarvestAI`

**Tick position:**
`MissionClass::Mission_Dispatch` → `ObjectClass::AI` → cell/pathfinding/target
systems → animation update. Dispatch runs before combat each tick; target
acquisition is therefore a pre-combat pass.

### 6.1 Who actually assigns `Mission_Area_Guard` (enum 11)?

Byte-pattern search for `PUSH 0xB` followed by `CALL [vtable + 0x1E8]`
(Queue_Mission) finds four confirmed assignment sites. None of them are
player-command paths — **Area_Guard is the AI idle default, not a player mode**:

| Site address | Function                                | Context                                            |
|--------------|-----------------------------------------|----------------------------------------------------|
| `0x00444D38` | `BuildingClass::ExitObject_Main` (@ `0x00443C60`) | **Newly-produced units start in Area_Guard.** When a factory/barracks/etc. ejects a freshly-built unit, this is the mission queued on it. |
| `0x0044490B` | `BuildingClass::ExitObject_Main` (alt)  | Second exit path in the same function (passenger/multi-exit)|
| `0x004D416A` | `FootClass::Find_Path` (@ `0x004D3920`) | Pathfinding fallback: when a path fails AND the unit is AI-controlled, queue Area_Guard. The symmetric branch for player-controlled units queues `Mission_Guard` instead (also in `Find_Path`). |
| `0x004DDFC2` | `FootClass::Mission_Rescue` (@ `0x004DDF90`) | When a rescue mission completes, the unit transitions to Area_Guard (followed by `Commence()`). |

Player-click paths in `FootClass::ClickedAction_Cell` / `ClickedAction_Object`
also reference the constant `0xB`, but these typically go through
`AttackMove`-style dispatch with different indirection. The Ctrl+Alt-click
"Area Guard" command in the UI is layered above this — it sets a different
flag, not the raw mission enum.

**Important implication:** a freshly-trained unit standing on the War Factory
exit pad is not in `Mission_Guard` — it's in `Mission_Area_Guard`. Its
automatic target-acquisition cadence is therefore governed by
`[General] GuardAreaTargetingDelay=36`, not by `NormalTargetingDelay=27`.
And its scan radius is `GuardRange × 2` (or `weapon × 2` fallback), not the
weapon-range-only scan.

The player-vs-AI split inside `Find_Path` is the tell: when the engine has to
"park" a unit that can't fulfill an order, it parks AI units in Area_Guard
(they'll autonomously hunt / repath) and player units in Guard (they'll
respect the player's intent and stand).

### 6.2 The `[IQ] GuardArea=2` knob

`ini/rulesmd.ini` has `GuardArea=2` under `[IQ]`. This is the AI-difficulty
threshold at which AI-controlled units produced by the house actually enter
Area_Guard rather than Guard. Below this IQ tier, the AI uses the same
player-like Guard behavior, which is less aggressive and doesn't auto-chase
targets. At `GuardArea=2` (Medium difficulty) and above, AI units idle in
Area_Guard and get the wider scan radius plus the harvester/patrol logic
wired into `FootClass::Mission_AreaGuard`.

---

## 7. YR activity check — TS-legacy flags

All code paths described here are **live in Yuri's Revenge**; no `SpecialFlags &
0x1000` gating. Two caveats worth documenting:

1. **`Mission_Eaten` (enum 9)** is retained from Tiberian Sun and maps to vtable
   slot `0x218`. FootClass vtable slot `0x218` is a stub returning 450 frames.
   The mission can still be *assigned* (e.g. by carnivorous creatures swallowing
   a victim), but no standard YR unit or trigger places anything into it. Not a
   Guard/AreaGuard concern, just noted because its presence is what shifts
   Area_Guard from enum 10 (YRpp / ModEnc docs) to **enum 11 (binary reality)**.

2. **AI auto-`Sabotage` branch** (Guard phase 4, AreaGuard phase 8) is active in
   YR — Terrorists/suicide-type infantry driven by the AI will enter
   `Mission_Sabotage` from idle if their current target is a building. Player-
   controlled infantry never trigger this path (the `IsPlayerControl` check
   guards it).

No fog-of-war-style dormant code found in this subsystem.

---

## 8. Current Rust implementation status

Summary of gaps between gamemd and [src/](src/):

| Behavior                                              | Status in Rust today |
|-------------------------------------------------------|----------------------|
| `Guard` command and persistent `OrderIntent::Guard`   | ✓ implemented ([src/sim/command.rs:54](src/sim/command.rs#L54), [src/sim/world/world_commands.rs:984](src/sim/world/world_commands.rs#L984)) |
| `GuardRange=` INI parsing                             | ✓ parsed ([src/rules/object_type.rs:241](src/rules/object_type.rs#L241)) |
| Target acquisition gated by OrderIntent               | ✓ ([src/sim/world/world_orders.rs:22](src/sim/world/world_orders.rs#L22)) |
| Retaliation for idle (no-intent) units                | ✓ ([src/sim/combat/combat_targeting.rs:252](src/sim/combat/combat_targeting.rs#L252)) |
| Aircraft Guard with RTB-on-ammo                       | ✓ ([src/sim/aircraft/mod.rs:57](src/sim/aircraft/mod.rs#L57)) |
| Post-combat anchor return (`OrderIntent::Guard` resume) | ✓ ([src/sim/world/world_orders.rs:58](src/sim/world/world_orders.rs#L58)) |
| **Distinct Area_Guard behavior** (widened scan, different rate) | ✗ — Rust uses a single `Guard` OrderIntent; no separate mission enum |
| **DefaultToGuardArea re-anchor** when target engaged  | ✗ — not implemented ([rules/object_type.rs](src/rules/object_type.rs) does not read `DefaultToGuardArea`) |
| 8-neighbour **garrison auto-enter** scan from idle     | ✗ — not present; units never auto-garrison |
| AI **auto-Sabotage** (Terrorist/Assaulter) from idle  | ✗ — not implemented |
| BuildingClass defensive auto-target (`Mission_Guard` for structures) | ✗ — base defenses currently driven by a different path; verify coverage |
| `GuardAreaTargetingDelay=36` vs `NormalTargetingDelay=27` scan rates | ✗ — single target-scan cadence |
| **Weaponless Guard hard-fallback** `scan_radius = 0x200` (2 cells) | ? — verify in [combat_targeting.rs:188](src/sim/combat/combat_targeting.rs#L188) |
| `[Guard]`/`[Area Guard]` `MissionControlClass` table (Rate/AARate/NoThreat/…) | ✗ — no MissionControl INI parsing |
| Slave-miner `Guard_Harvester` recall-on-idle          | ✗ — no slave-miner support yet |

The architectural gap: gamemd distinguishes **Guard, Area_Guard, Hunt, Patrol,
AttackMove, Sticky** as first-class enum values with different dispatch paths and
per-mission INI knobs. Rust currently collapses all of these into a single
`OrderIntent::Guard` (plus `AttackMove`). Recreating authentic-feeling idle AI
(garrison-rush dogs, auto-sabotage Terrorists, harvester-like repath on Area
Guard) will eventually require a proper mission enum.

---

## 9. Key struct offsets (this = FootClass-derived object)

| Offset | Size | Field                   | Used in                         |
|--------|------|-------------------------|---------------------------------|
| 0x0AC  | 4    | `CurrentMission` (enum) | dispatcher, several checks      |
| 0x0B4  | 4    | `QueuedMission`         | Commence                        |
| 0x2B4  | 4    | `TarCom`                | Guard phase 2, 4; AreaGuard 10,11|
| 0x218  | 4    | `NavCom`                | AreaGuard phase 3,4,7,11        |
| 0x2E4  | 1    | IsDeploying/WarheadBusy | AreaGuard gate                  |
| 0x2EC  | 4    | MissionTimerStartFrame  | Guard phase 5 timer math        |
| 0x2F4  | 4    | MissionTimerDuration    | Guard phase 5 timer math        |
| 0x5BC  | 4    | EnterQueue count        | AreaGuard phase 3               |
| 0x5DC  | 4    | GhostCell (patrol coord)| AreaGuard phases 4,7            |
| 0x68D  | 1    | `HasReachedDock`        | AreaGuard phase 10              |
| 0x68E  | 1    | `HasFoundAutoTarget`    | Guard phase 3 → Attack phase 2  |
| 0x68F  | 1    | `IsReceivingRepair`     | Guard/AreaGuard phase 1         |
| 0x690  | 1    | `IsDockingToBuilding`   | Guard/AreaGuard phase 1         |
| 0x691  | 1    | `IsWeedingHarvester`    | Guard/AreaGuard phase 1         |
| 0x6B1  | 1    | `SelfEnterQueued`       | AreaGuard phase 3               |
| 0x6C0  | 4    | cached InfantryType*    | auto-sabotage check             |
| 0x6C4  | 4    | cached UnitType*        | Guard_Harvester/slave miner     |

**TypeClass offsets (per-object-type) — verified from ReadINI disassembly:**

| Offset  | Kind | Field              | INI key                 | Key uses                               |
|---------|------|--------------------|-------------------------|----------------------------------------|
| +0x158  | bool | CanTarget          | *(Warhead-side; forwarded)* | Guard phase 3 weapon gate          |
| +0x390  | bool | DefaultToGuardArea | `DefaultToGuardArea=`   | Guard phase 2 re-anchor                |
| **+0x5B8** | int  | **GuardRange**    | **`GuardRange=`**       | Primary scan-radius basis (doubled)    |
| +0x5E8  | int  | Sight              | `Sight=`                | Shroud reveal / sensor scan            |
| +0x68C  | int  | AirRangeBonus      | `AirRangeBonus=`        | Degenerate-fallback additive bonus     |
| +0x695  | bool | CloseRange         | `CloseRange=`           | AreaGuard phase 13 close-range divisor; melee spiral |
| +0x6B0  | bool | DistributedFire    | `DistributedFire=`      | Guard phase 5 fast re-dispatch (skip Random jitter)  |
| +0xD3E  | bool | Slaved             | `Slaved=`               | (Slave-linked lifecycle)               |
| +0xD99  | bool | CanPassiveAquire   | `CanPassiveAquire=`     | Can-Passive-Acquire gate               |
| +0xEB5  | bool | Assaulter *(Infantry)* | `Assaulter=`        | Grants auto-structure-attack weapon ability |
| **+0xEC2** | bool | **C4** *(Infantry)* | **`C4=`**            | **AI auto-Sabotage gate** (prev. thought to be Assaulter) |
| +0xEC4  | bool | Agent *(Infantry)* | `Agent=`                | Spy infiltration                       |
| +0xE0E  | bool | Harvester *(Unit)* | `Harvester=`            | AreaGuard phase 5 Harvest reroute (was mislabeled "SlaveMiner") |
| +0xE0F  | bool | Weeder *(Unit)*    | `Weeder=`               | (TS-legacy weed gatherer)              |
| +0x1575 | bool | CanBeOccupied *(Building)* | `CanBeOccupied=` | 8-neighbour garrison scan              |
| +0x16A9 | bool | UnitRepair *(Building)*    | `UnitRepair=`    | BuildingClass::Mission_Guard: arriving-passenger Unload |
| +0x16AA | bool | UnitReload *(Building)*    | `UnitReload=`    | Same as above + Unload-queue maintenance |
| +0x16AB | bool | Bunker *(Building)*        | `Bunker=`        | Same as above                          |
| +0x16B5 | bool | HasStupidGuardMode *(Building)* | `HasStupidGuardMode=` | 100-frame early-return path in BuildingClass::Mission_Guard (dead code — always false in YR) |
| +0x16C3 | bool | EMPulseCannon *(Building)* | `EMPulseCannon=` | "Always hunt" — skip operator check (TS-legacy, unused in YR content) |

---

## 10. Open questions

Resolved in the 2026-04-23 follow-up pass (keeping for history):

- ✅ `vtable+0x31C` — decompiled (`FUN_00707E60`). See §4.1. Result: `GuardRange`
  is doubled; `AirRangeBonus` contributes only in a degenerate fallback.
- ✅ `_DAT_007E9228..+0x20` close-combat constants — decoded to leptons (281.6,
  307.2, 204.8, 384.0, 332.8). See §4.4.
- ✅ `vtable+0x478` — verified as a universal no-op stub (`XOR AL, AL; RET` at
  `0x0041C040`). See §4.3.
- ✅ `FUN_007091D0` — it is `Can_Passive_Acquire()`, a predicate combining
  `CanPassiveAquire=` (TechnoType+0xD99), ammo/target state, operator-present
  checks for buildings, and weapon-equipped check. Returns `true` only when a
  threat scan is permitted right now.
- ✅ `FUN_0070F7E0` — it is `Is_Targeting_Timer_Expired()`. Reads a timer at
  `this+0x180` (start frame) / `this+0x188` (duration) and returns whether the
  cooldown has elapsed.
- ✅ `InfantryType+0xEC2` — it is `C4=`, not `Assaulter=` as prior docs claimed.
  `Assaulter=` is at `+0xEB5`. See §5.2.
- ✅ `UnitType+0xE0E` — it is `Harvester=`, not `SlaveMiner=`. See §5.2.
- ✅ BuildingType flags `+0x16A9..+0x16C3` — mapped. See §5.2.

Resolved in the 2026-04-23 final sweep:

- ✅ `BuildingType+0x16B5` — it is `HasStupidGuardMode=` (store at `0x00460EB4`).
  Noted as dead code in YR since all stock content sets it to `false`.
- ✅ `BuildingType+0x16B6` — it is `BridgeRepairHut=` (same read loop).
- ✅ `TechnoType+0x695` — it is `CloseRange=` (store at `0x00714987`, INI str at
  `0x008439C4`). Marks melee units (dogs, Terror Drones, Yuri Clones).
- ✅ `TechnoType+0x6B0` — it is `DistributedFire=` (store at `0x00714864`, INI
  str at `0x00843A64`). Bypasses the `Random(0,2)`-frame Guard-timer jitter
  when the unit still has ammo.
- ✅ Who assigns `Mission_Area_Guard`: four sites — `BuildingClass::ExitObject_Main`,
  `FootClass::Find_Path` (AI fallback), `FootClass::Mission_Rescue`. See §6.1.
  Player-controlled fallback in the same code paths goes to `Mission_Guard`.
- ✅ `DistributedFire=` is set `yes` on units that spread fire across targets;
  seen on Grizzly-style platforms in `rulesmd.ini`.

Truly still open (not relevant to Guard/AreaGuard fidelity, but noted):

- `BuildingTypeClass::ReadINI` at `0x006F32D0` is still mislabeled — it's a
  90-byte predicate helper, not the full BuildingType reader. The real reader
  is `BuildingTypeClass_ReadINI_Water` at `0x0045FE50` (poorly named because
  the same function also does the land-variant reads — Ghidra's label comes
  from the first case it recognized). A full offset → INI key catalog for
  BuildingType would be useful as a separate reference doc, but is beyond
  the scope of Guard/AreaGuard.
- The five `_DAT_007E9228..48` close-combat doubles are hardcoded constants
  in `.rdata`. No INI-read path writes to them, so they cannot be tweaked
  through mod content — they are fixed engine parameters.
- How the player UI Ctrl+Alt+click "Area Guard" command actually reaches
  `Queue_Mission(0xB)`. The four sites catalogued in §6.1 are the *only*
  direct assignments; the player path in `ClickedAction_*` uses an indirect
  dispatch (`CALL [EBP + 0x378]`) that was not traced this session. This is
  likely going through a higher-level `Assign_Destination` helper that picks
  the mission based on the order context — deferred to a separate investigation.

---

## Sources

**Ghidra decompilations (verified in live gamemd.exe session):**

- `FootClass::Mission_Guard` @ `0x004D5070`
- `FootClass::Mission_AreaGuard` @ `0x004D6AA0`
- `FootClass::Greatest_Threat_Scan` @ `0x004D5690`
- `TechnoClass::Greatest_Threat` @ `0x006F8DF0`
- `UnitClass::Mission_Guard_Harvester` @ `0x00740810`
- `UnitClass::ScanForTiberium_SlaveMiner` @ `0x00744100`
- `AircraftClass::Mission_Guard` @ `0x0041A5C0`
- `BuildingClass::Mission_Guard` @ `0x004496B0`
- `BuildingClass::Mission_AreaGuard` @ `0x00449A40`
- `MissionClass::Mission_Dispatch` @ `0x005B3060` (assembly-level jump table verified)
- `MissionClass::Assign_Mission` @ `0x005B2FD0`
- `MissionClass::Commence` @ `0x005B3570`
- `MissionClass::GetMissionTimerEntry` @ `0x005B3A00`
- `MissionClass::Read_INI` @ `0x005B3760`
- Vtable xref trees for `0x004D5070`, `0x004D6AA0`, `0x004DA2C0`, `0x00740810`,
  `0x00740A90`, `0x00744100`, `0x0041A5C0`, `0x004496B0`, `0x00449A40`
- Mission name pointer table @ `0x00816CAC` (32 entries, ASCII strings)
- MissionClass stub @ `0x005B2E70` (`B8 C2 01 00 00 C3` — `mov eax, 0x1C2; ret`)

**Follow-up decompilations (2026-04-23 verification pass):**

- `FUN_00707E60` — `TechnoClass::GetScanRange` (the `vtable+0x31C` accessor)
- `FUN_007091D0` — `Can_Passive_Acquire()` predicate
- `FUN_0070F7E0` — `Is_Targeting_Timer_Expired()` helper
- `TechnoTypeClass::ReadINI` @ `0x00712170` disassembly (EBP=this; confirmed
  `GuardRange→+0x5B8`, `AirRangeBonus→+0x68C`, `Sight→+0x5E8`,
  `CanPassiveAquire→+0xD99`, `Slaved→+0xD3E`)
- `InfantryTypeClass::ReadINI` @ `0x005240A0` disassembly (ESI=this; confirmed
  `Assaulter→+0xEB5`, `Thief→+0xEC0`, `C4→+0xEC2`, `Agent→+0xEC4`)
- `UnitTypeClass::ReadINI` @ `0x00747620` disassembly (EDI=this; confirmed
  `Harvester→+0xE0E`, `Weeder→+0xE0F`)
- `BuildingTypeClass_ReadINI_Water` @ `0x0045FE50` disassembly (EBP=this;
  mapped the `+0x16A9..+0x16E6` BuildingType flag cluster to named INI keys)
- `XOR AL, AL; RET` stub @ `0x0041C040` — vtable xref confirmed as universal
  no-op for slot `+0x478` across all five concrete classes (FootClass,
  AircraftClass, UnitClass, InfantryClass, BuildingClass)
- Close-combat double constants at `0x007E9228..+0x20` decoded from raw bytes

**Final-sweep decompilations (2026-04-23 third pass):**

- `BuildingTypeClass_ReadINI_Water` @ `0x0045FE50` — deeper read of the flag
  cluster around `0x00460E80..+0x50`; confirmed `HasStupidGuardMode→+0x16B5`
  (store `0x00460EB4`, INI str `0x0081A884`) and `BridgeRepairHut→+0x16B6`
- `TechnoTypeClass::ReadINI` @ `0x00712170` — second-pass reads at
  `0x00714864` (`DistributedFire→+0x6B0`, INI str `0x00843A64`) and
  `0x00714987` (`CloseRange→+0x695`, INI str `0x008439C4`)
- `BuildingClass::ExitObject_Main` @ `0x00443C60` — confirmed `PUSH 0xB` at
  `0x00444D38` and `0x0044490B` (Queue_Mission(Area_Guard) on unit exit)
- `FootClass::Find_Path` @ `0x004D3920` — confirmed `PUSH 0xB` at `0x004D416A`
  (AI fallback on pathfinding failure)
- `FootClass::Mission_Rescue` @ `0x004DDF90` — confirmed `PUSH 0xB` at
  `0x004DDFC2` (rescue completion → Area_Guard)
- Byte-pattern search `6A 0B 8B ?? FF 90 E8 01 00 00` (direct
  `Queue_Mission(0xB, …)`) across `.text` — yielded only the four sites above;
  no other direct Area_Guard assignments exist in the binary.

**Research archive (read, cross-referenced):**
- `MISSIONCLASS_STATE_MACHINE.md`
- `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`
- `FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
- `TARGET_ACQUISITION_GHIDRA_REPORT.md`
- `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`
- `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
- `BUILDINGCLASS_MISSIONS_AND_INI_VERIFICATION.md`

**INI files (checked):**
- `ini/rulesmd.ini` — `[General]` (GuardModeStray, GuardAreaTargetingDelay,
  NormalTargetingDelay), `[IQ]` (GuardArea), per-unit `GuardRange`,
  `DefaultToGuardArea`, `CanPassiveAquire`
- `ini/rules.ini` — base RA2 defaults (same keys)

**Rust source (read for gap analysis, no modifications):**
- [src/sim/command.rs](src/sim/command.rs), [src/sim/components.rs](src/sim/components.rs)
- [src/sim/world/world_commands.rs](src/sim/world/world_commands.rs),
  [src/sim/world/world_orders.rs](src/sim/world/world_orders.rs),
  [src/sim/world/mod.rs](src/sim/world/mod.rs)
- [src/sim/combat/combat_targeting.rs](src/sim/combat/combat_targeting.rs)
- [src/sim/aircraft/mod.rs](src/sim/aircraft/mod.rs)
- [src/rules/object_type.rs](src/rules/object_type.rs)
