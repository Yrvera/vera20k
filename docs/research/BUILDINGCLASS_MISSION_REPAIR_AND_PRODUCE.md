---
name: BuildingClass Mission_RepairAndProduce Deep Dive
description: Verified dispatch structure for Mission_RepairAndProduce (0x0044B780) - the 7-mode per-tick operational handler covering ConYard, Hospital, Armory, Repair Depot, Helipad, Bunker, plus the actual multi-state machines inside each mode
type: reference
---

# Mission_RepairAndProduce — Ghidra Research Report

**Address:** `0x0044B780` (BuildingClass::MissionRepairAndProduce, vtable slot 147)
**Size:** 4604 bytes
**Confidence:** HIGH (all 7 modes and their state machines verified from binary)
**Active in YR:** Yes — dispatched from Mission_Dispatch when BuildingClass mission == 20 (REPAIR) / production-active

This function is dispatched whenever a building is in `Mission::REPAIR` or
its idle/operational state. It is NOT just "repair" — it's the per-tick
operational loop for buildings that have any kind of dispatchable tick
behavior: construction yards, hospitals, armories, repair depots, helipads,
bunkers.

## 1. Top-Level Dispatch

The function branches on BuildingTypeClass flags. Each branch has its own
state machine at `BuildingClass+0xBC` (mission sub-state).

| # | Type Flag | Offset | Mode |
|---|---|---|---|
| 1 | `Bunker=` | +0x16AB | Battle Bunker occupant docking |
| 2 | `ConstructionYard=` | +0x16B9 | ConYard construct-idle cycle |
| 3 | `Hospital=` | +0x16C1 | Timed heal |
| 4 | `Armory=` | +0x16C2 | Timed promotion |
| 5 | `UnitRepair=` | +0x16A9 | Repair Depot docking + HP tick |
| 6 | `UnitReload=` | +0x16AA | Helipad dock → refuel/reload cycle |
| 7 | default | — | Return 0xF (no-op timer) |

Default return (nothing matched): `0xF` (15 — suggests a 15-frame re-check
interval for buildings with none of these flags).

---

## 2. Bunker Mode (Type+0x16AB) — 6-State Docking Machine

Delegates to `FUN_00458E50` with the unit pointer at `BuildingClass+0x2E4`
(or via `FootClass::GetDestination`). **Early return** if the docked object
is not `Kind==1` (UnitClass): clears sub-state and queues GUARD.

State stored at `BuildingClass+0x718` (int).

### State 0 — Arrival check
```c
unit_cell = piVar5->vtable[0x1BC](); // GetCellCoord
if (Look_up_building_in_cell(unit_cell) == this &&
    !piVar5[0x19D]->vtable[0x10]()) {
    // Unit at bunker cell AND piggyback not active:
    // iterate exit cells via vtable+0x108 exit list
    // for each cell: CellClass::Find_Nearest_Object, scatter via vtable+0x174
    state = 1;
}
```

### State 1 — Find dock slot + set facing
Iterates exit cells looking for a valid passable one, computes facing from
unit→building delta via `atan2`, stores into `RateTimer`, state=2.

### State 2 — CDTimer wait, anim setup
When `CDTimerClass::Remaining() == 0`:
- Compute animation frame from current facing: `0x43`/`0x44`/`0x45`/`0x46`
  (4 directions)
- Call `piVar5[0x19D]->vtable[0x70]` (piggyback Set_Location)
- Call `piVar5->vtable[0x544]` (set looping/locomotor)
- state = 3

### State 3 — Final arrival check
```c
if (Look_up_building_in_cell == this && !piggyback_active) {
    RateTimer::Set(0x8000);  // half turn
    state = 4;
}
```

### State 4 — Wait for anim frame, then activate anims
When timer expires:
- Create anim slot for `Type+0x11F4/0x1204` (healthy/damaged primary)
- Create anim slot for `Type+0x1238/0x1248` (secondary)
- state = 5

### State 5 — Link unit, notify
- `BuildingClass+0x2E4 = unit` (docked unit pointer)
- `unit[0xB9] = this` (back-pointer)
- `unit[0x85] = -1` (clear some flag)
- Call `unit->vtable[0x150]` (locomotor SetMove?)
- state = 6 (terminal)
- `unit->vtable[0x1E8](5, 1)` (queue mission 5=GUARD)
- If `Rules+0x240 != -1`: `VocClass::PlayAt(Rules+0x240)` — docking sound

**Terminal state 6**: function keeps getting called but does nothing
(unit is garrisoned).

---

## 3. ConstructionYard Mode (Type+0x16B9) — 2-State

State at `BuildingClass+0xBC`.

### State 0 — GrandOpening
- Call `BuildingClass::GrandOpening(2)` (finish build-up)
- Health-dispatch anim slot: `Type+0x1128` (healthy) or `Type+0x1138` (damaged)
- state = 2

### State 2 — Idle monitor
- `PathType::Has_Valid_Steps()` check
- If no valid steps: Queue_Mission(5=GUARD) + ClearAnimSlot, return 1

---

## 4. Hospital Mode (Type+0x16C1) — 2-State Heal Timer

State at `BuildingClass+0xBC`.

### State 0 — Init
```c
state = 2;
+0x6DD = 0;              // construction-complete flag
+0x620 = 0;              // timer accumulator
+0x628 = g_CurrentFrameCounter;
+0x62C = 0;              // CDTimer aux
+0x630 = 1;              // CDTimer rate
+0x634 = 1;              // CDTimer active flag
+0x638 = 1;              // step per tick

if (Type+0x684 != -1) {
    +0x2FC = clamp(+0x2FC - 1, 0, +inf);  // decrement occupant count
}
```

### State 2 — Per-tick timer + heal trigger

```c
if (CDTimerClass::GetTimeRemaining() == 0 && +0x634 != 0) {
    +0x624 = 1;                        // "timer fired" flag
    +0x620 += +0x638;                  // accumulate
    +0x628 = g_CurrentFrameCounter;    // reset CDTimer
    +0x62C = 0;
    +0x630 = +0x634;
} else {
    +0x624 = 0;
}

// TRIGGER when accumulator reaches threshold:
if (Rules+0x16F0 [IRepairRate] * DAT_007E27F8 [= 900.0] <= (double)+0x620) {
    +0x6DD = 0;
    +0x620 = 0;
    msg_result = this->vtable[0x274](0x1C);   // REPAIR_COMPLETE radio
    switch (msg_result) {
        case 10:                       // ROGER, keep going
            break;
        case 0x20:                     // INSUFFICIENT_FUNDS
            this->vtable[0x100](...);  // Exit (eject occupant)
            return 1;
        case 0x21:                     // REPAIR_COMPLETE
            if (player-controlled) {
                CreateRadarEvent(coord);
                VoxClass::PlayEVA(-1);
                VocClass::PlayAt(building.location);
            }
            this->vtable[0x100](...);  // Exit
            this->vtable[0x1E8](5, 0); // Queue_Mission(GUARD)
            break;
    }
}
```

**Heal threshold formula** (verified):
```
threshold = Rules.IRepairRate (Rules+0x16F0) × 900.0
```
900.0 is the constant at `DAT_007E27F8` (same constant master doc
documented). At default rulesmd.ini `IRepairRate=.016` and step=1, the
infantry is healed after `0.016 × 900 = 14.4` timer firings. Timer is
tick-based (1 rate), so ~14-15 frames per heal tick.

---

## 5. Armory Mode (Type+0x16C2) — 2-State Promote Timer

**Identical structure to Hospital** with two differences:

1. The completion branch does **not** call vtable+0x274 (no radio exchange).
   Instead:
   ```c
   occupant = FootClass::GetDestination();
   if (VeterancyStruct::IsRookie(occupant.veterancy)) {
       VeterancyStruct::SetVeteran(occupant.veterancy);
   } else {
       VeterancyStruct::SetElite(occupant.veterancy);
   }
   this->vtable[0x100](...);   // Exit
   this->vtable[0x1E8](5, 0);  // Queue_Mission(GUARD)
   ```

2. Same timer formula: `Rules+0x16F0 × 900.0`. Shared tuning parameter.

**Active in YR:** Yes (Battle Lab? Actually no — Armory is the Yuri unit
promotion building. `YAROCK` / `ARMORY`.)

---

## 6. Repair Depot Mode (Type+0x16A9) — 3-State Machine

This is the **closest thing to the "5-state gate machine"** from master doc
section 10. It's actually 3 states at `+0xBC`, but with complex sub-flow.

State at `BuildingClass+0xBC`.

### State 0 — Piggyback attach (LAB_0044C62A)

Entry point for a fresh repair pad with no docked unit.

```c
if (!PathType::Has_Valid_Steps()) {
    // No unit docking → idle animations
    ClearAnimSlot(8); ClearAnimSlot(11);
    CreateAnimForSlot(Type+0x127C/0x128C);  // idle primary
    CreateAnimForSlot(Type+0x1018/0x1028);  // idle secondary
    Queue_Mission(5=GUARD);
    return 1;
}

// Unit is docking:
unit = FootClass::GetDestination();
unit_locomotor = unit[0x674];
piggyback = LocomotionClass::QueryInterface_IPiggyback(unit_locomotor);

// Compare locomotor to known types:
walk_match = memcmp(piggyback_clsid, &CLSID_WalkLocomotion, 16);
alt_match = memcmp(piggyback_clsid, &DAT_007E9AB0, 16);    // another locomotor CLSID

msg_result = this->vtable[0x274](0x13);   // REQUEST_DOCK radio
if (msg_result == 1) {
    distance = this->vtable[0x4D8](unit);
    if (distance < 100 /*0x64 = local_28*/) {
        state = 1;
        piggyback->vtable[0x8]();    // piggyback Push (attach)
        return 3;
    }
}

// Piggyback cleanup on failure:
if (!FUN_0053A130() && piggyback active) {
    piggyback->vtable[0x58]();   // detach
}
```

### State 1 — Drive-in phase

```c
if (!PathType::Has_Valid_Steps()) {
    // Arrived: switch to anim slots for docked state
    CreateAnimForSlot(Type+0x127C/0x128C);  // arm-out primary
    CreateAnimForSlot(Type+0x1018/0x1028);  // arm-out secondary
    ClearAnimSlot(8); ClearAnimSlot(11);
    if (+0x58C == 0) {
        +0x6DD = 1;
        Queue_Mission(5=GUARD);
    }
    return 1;
}

if (+0x57C == 0) {   // some precondition
    unit = FootClass::GetDestination();
    distance = this->vtable[0x4D8](unit);
    if (distance < 200) {
        // Piggyback is managing unit position
        piggy = unit[0x674];
        if (!piggy->vtable[0x60]()) {     // IsAtDestination?
            if (!piggy->vtable[0x10]()) { // IsActive?
                piggy->vtable[0x5C]();    // Reset
            }
            return 1;
        }
        if (!piggy->vtable[0x60]() &&      // another check
            unit[0x5A4] != 0) {
            FootClass::Stop_Moving();
        }
    }
    
    msg = this->vtable[0x274](0x13);       // REQUEST_DOCK
    if (msg == 1) {                         // ROGER
        unit = FootClass::GetDestination();
        health_ratio = unit.GetHealthRatio();
        gate_threshold = Rules+0x16F8;      // close-enough threshold
        
        if (Rules+0x16F8 <= health_ratio && !unit.Type+0xD24 [NonVehicle]) {
            // Already healthy - check for repair complete via radio
            msg2 = this->vtable[0x274](0x1C);
            if (msg2 == 1 || msg2 == 0x21) {
                // REPAIR_COMPLETE path
                if (health_ratio == 1.0 /*actually = Rules+0x16F8*/) {
                    piggy = unit[0x674];
                    if (!piggy->IsActive()) {
                        piggy->vtable[0x58]();  // detach piggyback
                        // Move unit out:
                        dest = unit[0x19D][0x58]();  // GetExitDestination
                        if (IsPlayerControl) {
                            // Queue_Mission(MOVE) + SetDestination
                        } else {
                            // AI path: Queue_Mission(2=MOVE) directly
                        }
                        unit[0x140] = 0;  // clear locomotor link
                        this->vtable[0x274](3);  // send CONFIRM
                    }
                }
            }
        } else {
            // Start repair animation:
            if (unit[5].IsOnMap && !player-controlled) {
                unit->vtable[0x1A0](1);  // SetRepair
                state = 0;
                +0x6DD = 1;
            } else {
                if (+0x41A) VoxClass::PlayEVA(-1);
                state = 2;
                CreateAnimForSlot(Type+0x11F4/0x1204);  // repair arm
                ClearAnimSlot(3); ClearAnimSlot(18);
                +0x6DD = 0;
                +0x620 = 0;
                +0x628 = g_CurrentFrameCounter;
                +0x634 = 1;  +0x62C = 0;  +0x630 = 1;  // init CDTimer
            }
        }
    }
}
```

### State 2 — Repair tick (HP-per-time)

```c
if (!PathType::Has_Valid_Steps()) {
    // Abort: clear anims and go back to state 1
    ClearAnimSlot(8); ClearAnimSlot(11);
    CreateAnimForSlot(Type+0x127C/0x128C);
    CreateAnimForSlot(Type+0x1018/0x1028);
    state = 1;
    return 1;
}

if (+0x634 == 0) +0x634 = 1;   // restart CDTimer if stopped

// Timer accumulate (same as Hospital):
if (CDTimerClass::GetTimeRemaining() == 0 && +0x634 != 0) {
    +0x624 = 1;
    +0x620 += +0x638;
    // reset timer
} else {
    +0x624 = 0;
}

// Repair tick trigger:
if (Rules+0x16E8 × 1.0 <= (double)+0x620) {
    msg = this->vtable[0x274](0x13);   // REQUEST_DOCK
    if (msg == 1) {                     // ROGER
        +0x6DD = 0;  +0x620 = 0;
        msg2 = this->vtable[0x274](0x1C);  // REPAIR_COMPLETE
        
        if (msg2 != 1) {
            if (msg2 == 0x20) {  // INSUFFICIENT_FUNDS
                if (+0x41A) VoxClass::PlayEVA(-1);
                // clear anims, reset to state 1
                state = 1;
            } else {  // 0x21 REPAIR_COMPLETE
                if (+0x41A) {
                    VoxClass::PlayEVA(-1);
                }
                // Clear anims, restore idle anims, state=1
                // Then eject unit:
                //   - If !player-controlled: Queue_Mission(2=MOVE), SetDestination, vtable[0x274](3)
                //   - If player: stay; player manually drives off
            }
        }
    }
}
```

### Repair tick tuning

**Different** threshold than Hospital:
```
threshold = Rules+0x16E8 × 1.0
```
vs Hospital's `Rules+0x16F0 × 900.0`. Rules+0x16E8 is likely `RepairStep` or
`RepairRate` (unit variant), smaller in magnitude because the multiplier is 1.0
not 900.0.

Each timer fire increments `+0x620` by `+0x638` (which is set to 1 on entry
into state 2). So repair fires every `Rules+0x16E8` ticks.

The per-tick HP amount and cost come from the radio messages — they're
computed at the other end (probably `TechnoClass::EngineerRepair` or
`vtable+0xB0/0xB4` on the unit). This function just drives the timing.

---

## 7. Helipad Mode (Type+0x16AA, UnitReload) — Aircraft Reload Loop

No state machine at `+0xBC`; iterates docked aircraft at `+0xE8` count /
`+0xE4?` array (actually uses FootClass::GetDestination per slot).

```c
bool any_update = false;
for (int i = 0; i < +0xE8; i++) {
    aircraft = FootClass::GetDestination(i);
    if (!aircraft) continue;
    
    // Radio message cycle:
    msg = this->vtable[0x278](0x1D, aircraft);  // REFUEL query
    if (msg == 1) {
        mission = aircraft->vtable[0x184]();
        if (mission == 7) continue;  // already refueling
        
        msg2 = this->vtable[0x278](0x13, aircraft);  // APPROACH
        if (msg2 == 1) {
            any_update = true;
            amission = aircraft->vtable[0x184]();
            if (amission == 0) {
                msg3 = this->vtable[0x278](0x1F, aircraft);  // RESERVE_DOCK
                if (msg3 != 1) {
                    msg4 = this->vtable[0x278](0x1C, aircraft);  // REPAIR_COMPLETE
                    if (msg4 == 1) {
                        aircraft->vtable[0x484](0, 0);  // SetPath(clear)
                        aircraft->vtable[0x1F0](5);      // Queue_Mission(5=GUARD)
                        aircraft->vtable[0x334]();       // release pad
                    }
                }
            } else {
                aircraft->vtable[0x1E8](0, 0);  // Queue_Mission(0=SLEEP)
            }
        }
    }
}

if (any_update) return ftol(timer_value);
else {
    Queue_Mission(5=GUARD);
    return 3;
}
```

### Radio message semantics (observed)

| Msg | Name (inferred) | Purpose |
|---|---|---|
| 0x13 | REQUEST_APPROACH | Request permission to dock |
| 0x1C | REPAIR_COMPLETE | Query "ready to depart" |
| 0x1D | REFUEL_QUERY | "Does unit need fuel?" |
| 0x1F | RESERVE_DOCK | Confirm docking intent |
| 0x20 | INSUFFICIENT_FUNDS | Response: can't continue |
| 0x21 | REPAIR_COMPLETE | Response: finished |
| 10 | ROGER (continue) | Response: OK |
| 3 | CONFIRM (advance state) | Sent on pad release |

---

## 8. Field Layout Used by Mission_RepairAndProduce

All offsets relative to `BuildingClass` (not Type).

| Offset | Size | Purpose |
|---|---|---|
| +0xBC | int | Mission sub-state (0/1/2 for most modes, 0-6 for Bunker at +0x718) |
| +0x2E4 | ptr | Docked unit pointer (Bunker, Repair Depot) |
| +0x2FC | int | Occupant/radio slot counter (decremented on Hospital state 0) |
| +0x41A | bool | "Is player house" indicator (for EVA/sound) |
| +0x620 | int | **Timer accumulator** (heal/repair progress) |
| +0x624 | byte | "Timer fired this tick" flag |
| +0x628 | int | CDTimer start frame |
| +0x62C | int | CDTimer aux |
| +0x630 | int | CDTimer rate |
| +0x634 | int | CDTimer active flag (0=paused) |
| +0x638 | int | **Step amount** per fire (added to +0x620) |
| +0x6DD | byte | Construction-complete flag |
| +0x718 | int | **Bunker sub-state** (0-6, separate from +0xBC) |
| +0x57C | ptr | Anim slot (repair/arm) |
| +0x588 | ptr | Anim slot (repair/arm secondary) |
| +0x58C | ptr | Anim slot |

## 9. Rules Offsets Used

| Offset | Purpose |
|---|---|
| +0x16E8 | Repair Depot HP-per-time threshold |
| +0x16F0 | `IRepairRate` — Hospital/Armory heal rate |
| +0x16F8 | "Close enough" health ratio for repair completion |
| +0x1700 | ConditionYellow health ratio (damaged anim threshold) |
| +0x240 | Bunker docking sound index |
| DAT_007E27F8 | Constant `900.0` multiplier for Hospital/Armory |

---

## 10. What the "5-State Gate Machine" Actually Is

Master doc section 10 referenced a "5-state gate machine" for vehicle exit
from WeaponsFactory. After full investigation, **this is not a single state
machine inside one function**. It's the combination of:

1. **Production completion trigger** in Mission_RepairAndProduce Repair
   Depot path (3-state at +0xBC)
2. **ClearBibArea** (0x00449540) — up to 8 scatter attempts before Unlimbo
3. **ExitObject** Unlimbo + Queue_Mission(MOVE)
4. **LocomotionClass::QueryInterface_IPiggyback** — the actual "gate
   open + drive out" is a locomotor piggyback that temporarily takes over
   the unit's movement
5. **Anim slot state** — `Type+0x16F8 GateStages` drives DrawBody and
   GetCurrentFrame for rendering the gate frames

The "5 phases" described in master doc (init → clear bib → drive out → wait
→ close gate) are conceptual spread across these sub-systems. There is no
single `gate_state` field holding these 5 values.

**The actual verifiable state machines in Mission_RepairAndProduce are:**
- Bunker: **6 states** at +0x718 (0-5, then terminal 6)
- Repair Depot: **3 states** at +0xBC (0-2)
- Hospital/Armory: **2 states** at +0xBC (0, 2)
- ConYard: **2 states** at +0xBC (0, 2)

## 11. Call Graph Summary

**Per-building type dispatch from this function:**

| Mode | Anim slots used | Radio messages used | Vtable calls |
|---|---|---|---|
| Bunker | +0x11F4/1204, +0x1238/1248 | — | 0x70, 0x174, 0x544, 0x150, 0x1E8 |
| ConYard | +0x1128/1138 (healthy/damaged) | — | GrandOpening, 0x1E8 |
| Hospital | (none) | 0x1C → 10/0x20/0x21 | 0x274, 0x100, 0x1E8 |
| Armory | (none) | — | 0x100, 0x1E8, FootClass::GetDestination, VeterancyStruct |
| Repair Depot | +0x127C/128C, +0x1018/1028, +0x11F4/1204 | 0x13, 0x1C, 3 | 0x274, 0x4D8, 0x1A0, many locomotor calls |
| Helipad | (none) | 0x1D, 0x13, 0x1F, 0x1C | 0x278, 0x484, 0x1F0, 0x1E8, 0x334 |

---

## 12. Current Rust Implementation Status

From memory ([project_garrison_system.md](../../../../.claude/projects/c--Users-enok-Documents-ra2-rust-game/memory/project_garrison_system.md)): garrison RE was done but combat
not implemented; similar pattern likely applies here — fields may be parsed
but state machines not wired.

**Needs implementation:**
- Full Mission_RepairAndProduce dispatch (all 6 modes)
- CDTimer with accumulator pattern (+0x620/+0x628/+0x634/+0x638)
- Radio message system (0x13, 0x1C, 0x1D, 0x1F, 0x20, 0x21, etc.) at
  BuildingClass level
- LocomotionClass::IPiggyback interface for Repair Depot drive-in
- Per-anim-slot health-dispatch logic (Type+0xXXX healthy vs damaged)

---

## 13. Open Questions

1. **Rules+0x16E8 exact INI key name** — not traced in this pass. Likely
   `RepairStep=` or `RepairDelay=`. Needs ReadINI trace.
2. **Rules+0x16F8 exact INI key** — "close enough" health ratio threshold.
   Probably `RepairPercent=` or similar.
3. **DAT_007E9AB0 locomotor CLSID** — the "alternate" piggyback-compatible
   locomotor compared in state 0. Not identified; might be
   `JumpjetLocomotion` or `FlyLocomotion`.
4. **+0x57C / +0x588 / +0x58C anim slot roles** — which Anims[i] these
   point to. Need cross-reference with anim slot 21-entry layout.
5. **Helipad radio exchange order** — verified message sequence but the
   semantics of `0x1D REFUEL_QUERY` vs `0x1F RESERVE_DOCK` needs another
   pass; the exact "when does the heli get refuel/reload" trigger isn't
   fully isolated.

---

## Sources

### Ghidra functions decompiled

- `0x0044B780` — BuildingClass::MissionRepairAndProduce (main dispatcher)
- `0x00458E50` — Bunker docking state machine
- `0x00449540` — ClearBibArea (referenced)
- (context) `0x0043D290` — DrawBody (GateStages rendering consumer)
- (context) `0x0043EF90` — GetCurrentFrame (GateStages frame calc)

### Cross-references verified

- `Type+0x16F8 GateStages`: read in DrawBody, GetCurrentFrame,
  MissionRepairAndProduce (repair threshold), BuildingTypeClass::ReadINI
- `Rules+0x16F0`: IRepairRate — referenced by Hospital AND Armory
- `DAT_007E27F8`: constant 900.0 (per master doc)

### Master doc sections corrected / extended

- Section 10 "5-state gate machine" — **not a real state machine**, it's a
  conceptual description spread across ExitObject, Mission_RepairAndProduce,
  ClearBibArea, and LocomotionClass
- Section 17 Mission handlers — expand `Mission_RepairAndProduce` entry
  with the 7-mode dispatch table and actual state machines per mode
- Add new section for **Bunker docking** (6-state machine at +0x718) — not
  documented elsewhere
