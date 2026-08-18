# All Callers of ObjectClass::Scatter (vtable+0x174) -- EXCLUDING CellClass::Scatter_Objects

## Overview

The Scatter virtual method (vtable+0x174) is called from many places in gamemd.exe beyond
just `CellClass::Scatter_Objects`. This report catalogs EVERY caller found via byte-pattern
search for `call [reg+0x174]` (FF 90/92/93 74 01 00 00) plus direct function xrefs.

## Method

1. Byte-pattern search for `FF 90 74 01 00 00` (call [eax+0x174]) -- 17 hits
2. Byte-pattern search for `FF 92 74 01 00 00` (call [edx+0x174]) -- 27 hits
3. Byte-pattern search for `FF 93 74 01 00 00` (call [ebx+0x174]) -- 10 hits
4. Byte-pattern search for `FF 91 74 01 00 00` (call [ecx+0x174]) -- 0 hits
5. Cross-referenced with function boundaries to identify containing functions
6. Filtered out false positives (different class vtables where 0x174 is a different method)

## Per-Class Scatter Function Addresses (vtable+0x174)

| Class | Vtable | Scatter Address | Notes |
|-------|--------|----------------|-------|
| UnitClass | 0x7F5C70 | **0x00743A50** | Full implementation with 8-dir cell search |
| InfantryClass | 0x7EB058 | **0x0051D0D0** | More complex, subcell-aware |
| AircraftClass | 0x7E22A4 | **0x0041A590** | Minimal: only scatters if in movement mission |
| BuildingClass | 0x7E3EBC | **0x005F43A0** | No-op stub (`ret 0xC`) -- buildings can't scatter |

---

## CONFIRMED CALLERS (besides CellClass::Scatter_Objects)

### 1. Network Command Handler -- Case 7 "SCATTER" (0x4C6CB0, call at ~0x4C78C5)

**Player scatter hotkey command.** When a player presses the scatter key (X), it sends
network event type 7. The handler:

```c
case 7: // SCATTER command
    FootClass* unit = LookupObject(event->ObjectID);
    if (unit != NULL && unit->IsAlive && !unit->IsDead && !unit->IsInLimbo) {
        unit->field_0x6B2 = 1;  // mark as player-commanded scatter
        unit->vt->Scatter(&DAT_008a39b8, 1, 0);  // threat=1, force=0
    }
```

**Significance**: This is the PLAYER SCATTER COMMAND. Args: `coord=some_global, threat=1, force=0`.
The `field_0x6B2=1` flag distinguishes player-commanded scatter from auto-scatter.

### 2. TechnoClass::ReceiveDamage (0x701900, call at 0x702D0B)

**Damage-triggered scatter.** At the end of TechnoClass::ReceiveDamage, after processing
damage, retaliation, and targeting:

```c
// Near end of ReceiveDamage, after retaliation logic:
if (/* retaliation target found within range */) {
    this->vt->Retaliate(attacker);
    // ...check range, attack...
} else {
    // NO retaliation possible -- try to scatter instead
    // Extensive checks:
    //   - Must be a FootClass (has locomotor)
    //   - Mission timer must not be in scatter-cooldown state
    //   - field_0x418 must be 0 (not deploying?)
    //   - Locomotor must not be busy
    //   - Must have no current target AND no queued target
    //   - Must not be in mission GUARD
    //   - Then: check Rules->PlayerScatter OR vet abilities
    //   - OR: check if type has ScatterOnHit flag (TypeClass+0x29F or +0x2B1)
    if (all_checks_pass) {
        this->vt->Scatter(...);  // at 0x702D0B
    }
}
```

**Significance**: This is the PRIMARY THREAT SCATTER. Units scatter when taking damage
IF they have no target to retaliate against AND meet scatter eligibility requirements.
The checks for `TypeClass+0x29F` and `+0x2B1` likely correspond to veteran/elite
`Scatter` ability flags checked via `Volume__IsNormal` (veteran) and `FUN_00750010` (elite).

### 3. TechnoClass::CloakingTick (0x6FB740, call at 0x6FBC32)

**Decloak scatter.** When a cloaked unit transitions from cloaked (state 3) to decloaking
and the decloaking is "complete" (visual state check returns 1):

```c
// In cloaking state machine, state 1 (uncloaking):
if (What_Am_I() == RTTI_INFANTRY) {
    this->vt->Discover();  // vtable+0xFC
} else {
    // Not infantry: scatter all attackers targeting this unit
    // (iterates TechnoClass array looking for units targeting 'this')
    // Then calls:
    this->vt->Scatter(&g_NullCoord, 1, 0);  // at 0x6FBC32
}
```

**Significance**: When a non-infantry unit finishes uncloaking, it scatters. This makes
cloaked vehicles move after becoming visible, possibly to avoid being sitting ducks.

### 4. FootClass::AI -- Idle Scatter (0x4DA530, call at 0x4DAE59)

**Periodic idle scatter.** Every 64 frames (`(g_CurrentFrameCounter & 0x3F) == 0x3F`),
if a FootClass is idle (no target, not in limbo, mission allows scatter), it scatters:

```c
// At the end of FootClass::AI:
if ((g_CurrentFrameCounter & 0x3F) == 0x3F  // every 64 frames
    && this->NavTarget == NULL               // not heading somewhere
    && this->IsInLimbo == false
    && !CellClass::IsBridge(current_cell)    // cell+0x11C = bridge flag
    && MissionControl->CanScatter()          // mission scatter flag
    && GetHeight() == 0)                     // on ground level
{
    CoordStruct zero = {0, 0, 0};
    this->vt->Scatter(&zero, 1, 0);  // at 0x4DAE59
}
```

**Significance**: This is the IDLE SCATTER that makes units periodically shuffle when
standing around. Runs every ~4.2 seconds (64 frames at 15fps). Only triggers if the
unit's current mission allows scatter (via MissionControlClass Scatter flag).

### 5. FUN_004D82B0 -- FootClass Tick Processing (0x4D82B0, call at 0x4D82FC)

**Post-queued-orders scatter.** Called from InfantryClass::IdleDispatch, UnitClass::Scatter_Force,
and at 0x417782. When `field_0x687` is set (a flag meaning "should scatter next tick"):

```c
void FootClass::ProcessTick(FootClass* this, ...) {
    this->field_0x6B3 = 1;  // mark as processing
    TechnoClass::Tick(this, ...);

    if (this->field_0x687 != 0) {
        this->field_0x687 = 0;
        this->vt->Scatter(&DAT_008b3da8, 1, 0);  // at 0x4D82FC
    }
    // ... queue processing, locomotor updates ...
}
```

**Significance**: Deferred scatter -- some system sets `field_0x687=1` to request scatter
on the next tick rather than immediately. Args: `coord=DAT_008b3da8` (a global coord),
`threat=1, force=0`.

### 6. HouseClass::Update (0x4F8440, call at 0x4F899F)

**House production rally point scatter.** In HouseClass::Update, periodically checks if
there's a unit sitting on the house's primary building rally cell, and scatters it:

```c
// In HouseClass::Update, around 0x4F8934-0x4F89A5:
// Every 15 frames: check if the house's primary building exit cell has a unit on it
CellStruct rally_cell = this->PrimaryBuildingExit;  // HouseClass+0x53E0
if (rally_cell != last_checked_cell) {
    if ((g_CurrentFrameCounter % 15) == 0) {
        CellClass* cell = MapClass::Get_CellClass(rally_cell);
        ObjectClass* obj = CellClass::FindNearestObject(cell, ...);
        if (obj != NULL) {
            if (obj->field_0x14 & 4  /* is deployed? */) {
                // check if already garrisoned
            } else {
                obj->vt->Scatter(&g_NullCoord, 1, 1);  // force=1!  at 0x4F899F
                // Then tries to move obj 5 times via ReceiveDamage-like nudging
            }
        }
    }
}
```

**Significance**: Forces units sitting on a factory exit cell to move away, preventing
production blockage. Note `force=1` -- this scatter cannot be refused.

### 7. FUN_005200b0 -- InfantryClass Garrison/Occupy Timer (0x5200B0, call at 0x520254)

**Garrison-related scatter.** Called from InfantryClass::AI. When a building's "CanOccupy"
flag is set and the infantry has been occupying for more than 0x32 ticks without moving:

```c
// In FUN_005200b0 (called from InfantryClass::AI at 0x51BF0B):
if (type->CanBeOccupied && occupancy_timer > 0x32
    && seq != 0x1B..0x1E  // not crawling
    && !IsInLimbo && !Locomotor->IsBusy())
{
    this->vt->Scatter(&DAT_00a8f200, 1, 0);  // at 0x520254
}
```

**Significance**: Forces infantry to scatter out of a building after a timeout. This
may be related to garrison evacuation or occupy-then-scatter behavior.

### 8. BuildingClass::SellBuilding (0x457DE0, call at 0x45810A)

**Sell building -- eject occupants.** When a building is sold, iterates all occupants
(the garrison array at field_0x688) and scatters them:

```c
// In BuildingClass::SellBuilding:
for (int i = occupant_count - 1; i >= 0; i--) {
    FootClass* occupant = this->Occupants[i];
    if (occupant->CanEnterCell(exit_cell, ...) == PASSABLE) {
        occupant->vt->Scatter(building_exit_coord, ...);  // at 0x45810A (ebx variant)
        if (!dead) {
            occupant->vt->SetMission(MISSION_GUARD, 0);
        }
    } else {
        occupant->vt->Kill();
    }
}
```

**Significance**: Ejects garrisoned infantry when building is sold. Uses the building's
exit coordinate as the scatter direction hint.

### 9. FUN_00458E50 -- BuildingClass Repair Dock/Undeploy Handler (0x458E50, call at 0x458F93)

**Dock state machine -- scatter occupants.** In a building's dock processing (repair depot
or similar), when units arrive and the building needs to clear its foundation cells:

```c
// In FUN_00458E50, case 0 and case 1 of dock state machine:
// Iterates foundation cells looking for objects near the building
ObjectClass* nearby = CellClass::Find_Nearest_Object(cell, ...);
if (nearby != NULL && nearby != docked_unit) {
    nearby->vt->Scatter(building_coord, ...);  // at 0x458F93 (ebx variant)
}
```

**Significance**: Clears other units off a repair depot's foundation so the docked
unit has room. Note: the building calls Scatter on OTHER objects near it, not on itself.

### 10. BuildingClass::Sell (0x449C30, call at 0x44A768)

**Building sell -- scatter adjacent units.** Similar to SellBuilding but in the main
Sell handler:

```c
// In BuildingClass::Sell:
this->vt->Scatter(...);  // scatters occupants during sell process, at 0x44A768
```

### 11. BuildingClass::SpawnSurvivors (0x442D90, calls at 0x442FA1 and 0x443217)

**Building destruction -- scatter survivors.** When a building is destroyed and spawns
survivor units:

```c
// In BuildingClass::SpawnSurvivors:
// After creating survivor infantry at the building's exit point:
survivor->vt->Scatter(exit_coord, ...);  // at 0x442FA1 and 0x443217
```

**Significance**: Makes survivor infantry scatter away from the destroyed building.

### 12. BuildingClass::AddGarrisonOccupant (0x522910, call at 0x522AE6)

**Garrison overflow scatter.** When adding an occupant to a building that handles it
via "SpawnUnitsWithParachute" path (non-garrison building or overflow):

```c
// In BuildingClass::AddGarrisonOccupant:
if (type->CanBeGarrisoned == false) {
    if (type->CanOccupy == false) return;
    SpawnUnitsWithParachute(this);
    this->vt->SetDestination(NULL, 1);
    this->vt->Scatter(infantry_coord, ...);  // scatter the infantry away, at 0x522AE6 (ebx)
}
```

### 13. TacticalClass/BuildingPlacement Handler (0x443C60, call at 0x444F04)

**Building placement -- scatter units off foundation.** When a building is being placed,
iterates the foundation cells and scatters any units that are in the way:

```c
// In TacticalClass handler, case 6 (building placement):
// After verifying placement is valid:
for (each foundation cell) {
    ObjectClass* obj = CellClass::Find_Nearest_Object(cell);
    if (obj != NULL && obj != placing_unit) {
        obj->vt->Scatter(building_exit_coord, ...);  // at 0x444F04
    }
}
```

### 14. FootClass::Receive_Radio (0x4D8FB0, call at 0x4D90C6)

**Radio-triggered scatter.** In FootClass::Receive_Radio, under certain radio messages:

```c
// In FootClass::Receive_Radio:
this->vt->Scatter(&DAT_008a3da8, 1, 0);  // at 0x4D90C6
```

**Significance**: A radio message from another unit triggers scatter behavior.

### 15. BuildingClass::Receive_Radio (0x43C2D0, call at 0x43CAF7)

**Building radio scatter.** The building's Receive_Radio handler scatters an object:

```c
// In BuildingClass::Receive_Radio:
obj->vt->Scatter(...);  // at 0x43CAF7
```

### 16. InfantryClass::UpdateIdleAction (0x51CDB0, call at 0x51CE76)

**Infantry idle action scatter.** During the idle action update, infantry may self-scatter:

```c
// In InfantryClass::UpdateIdleAction:
this->vt->Scatter(...);  // at 0x51CE76
```

### 17. InfantryClass::Mission_Enter (0x5196A0, calls at 0x51A0CE, 0x51A4AF, 0x51A724, etc.)

**Enter-building mission scatter.** Multiple scatter calls in the infantry enter-building
mission state machine. When infantry enters or leaves a building:

```c
// Various states of InfantryClass::Mission_Enter:
this->vt->Scatter(...);  // multiple call sites within the mission
```

### 18. UnitClass::Scatter_Force (0x738970) -- vtable override

**Force scatter with complex logic.** This is NOT the vtable+0x174 Scatter itself but
rather a separate function that calls Scatter internally. It's at vtable+0x4AC or similar,
implementing more complex scatter-like behavior with mission state transitions.

Internal call: `(**(code **)(*param_1 + 0x480))(0, 1)` (SetDestination) rather than 0x174.

### 19. FUN_007359F0 -- UnitClass Tunnel/Tube Movement (0x7359F0, calls at 0x736028 and 0x735F55)

**Tunnel exit scatter.** When a unit exits a tunnel, it scatters nearby units and then
scatters itself:

```c
// In FUN_007359F0 (tunnel exit handler):
// At tube exit point, if cell has occupants:
for (each occupant) {
    if (IsFootClass(occupant) && !locomotor->IsBusy()) {
        occupant->vt->Scatter(&zero_coord, 1, 1);  // force=1, at inner loop
    }
}
// Then the unit itself scatters at its destination:
if (this->NavTarget == destination_cell) {
    this->vt->Scatter(&zero_coord, 1, 1);  // force=1, self scatter at 0x736028
}
```

**Significance**: Units emerging from tunnels force-scatter everything at the exit.

### 20. UnitClass::Mission_Harvest (0x737C90, calls at 0x73831F and 0x73813D)

**Harvester scatter.** During harvest mission, when a harvester needs to clear units:

```c
// In UnitClass::Mission_Harvest:
this->vt->Scatter(...);  // scatter when needing to reposition during harvest
```

### 21. UnitClass::Mission_Enter (0x739EC0, calls at 0x73A5E4, 0x73A7C2, 0x73AADB, 0x73AB66, etc.)

**Enter-transport mission scatter.** Multiple scatter calls in the unit enter-transport
mission state machine.

### 22. UnitClass::Receive_Radio (0x737430, call at 0x737AD0)

**Radio-triggered scatter for units.**

### 23. FUN_006ECF10 -- Cell-Level Scatter All (0x6ECF10)

**Dead code?** Iterates a linked list of objects at param+0x54 (CellClass object list)
and calls Scatter on each:

```c
void FUN_006ecf10(CellClass* cell) {
    for (ObjectClass* obj = cell->FirstObject; obj != NULL; obj = obj->NextInCell) {
        obj->vt->Scatter(&DAT_00b0e968, 1, 0);
    }
    cell->field_0x80 = 1;
}
```

**No xrefs found.** Likely dead/unreferenced code. May be an older version of
CellClass::Scatter_Objects or referenced via an unresolved pointer.

---

## SUMMARY: Branch A vs Branch B Analysis

The question was: **Is threat scatter (Branch B) ever triggered, or only movement scatter
(Branch A via CellClass::Scatter_Objects)?**

### Answer: BOTH branches are active in YR.

**Branch A (Movement Scatter via CellClass::Scatter_Objects):**
- Triggered by locomotor movement when a unit enters an occupied cell
- Documented in SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md

**Branch B (Threat/Direct Scatter -- vtable+0x174 called directly):**

The following callers invoke Scatter DIRECTLY on specific objects, bypassing
CellClass::Scatter_Objects entirely:

| # | Caller | Args | Trigger |
|---|--------|------|---------|
| 1 | **Network Command Case 7** | coord=global, threat=1, force=0 | Player presses scatter key |
| 2 | **TechnoClass::ReceiveDamage** | varies | Taking damage with no retaliation target |
| 3 | **TechnoClass::CloakingTick** | NullCoord, 1, 0 | Non-infantry finishes uncloaking |
| 4 | **FootClass::AI (idle)** | {0,0,0}, 1, 0 | Every 64 frames while idle |
| 5 | **FUN_004D82B0 (deferred)** | global, 1, 0 | Deferred scatter flag set previous tick |
| 6 | **HouseClass::Update** | NullCoord, 1, **1** | Unit blocking factory exit cell |
| 7 | **FUN_005200B0 (garrison timer)** | global, 1, 0 | Infantry occupy timeout |
| 8 | **BuildingClass::SellBuilding** | exit coord | Building sold, eject occupants |
| 9 | **FUN_00458E50 (dock handler)** | building coord | Clear repair depot foundation |
| 10 | **BuildingClass::Sell** | varies | Building sell path |
| 11 | **BuildingClass::SpawnSurvivors** | exit coord | Building destroyed, spawn survivors |
| 12 | **BuildingClass::AddGarrisonOccupant** | infantry coord | Garrison overflow |
| 13 | **Building Placement Handler** | exit coord | Units on placement foundation |
| 14 | **FootClass::Receive_Radio** | global, 1, 0 | Radio message from another unit |
| 15 | **BuildingClass::Receive_Radio** | varies | Building radio message |
| 16 | **InfantryClass::UpdateIdleAction** | varies | Infantry idle state update |
| 17 | **InfantryClass::Mission_Enter** | varies (multiple sites) | Enter-building mission |
| 18 | **FUN_007359F0 (tunnel exit)** | {0,0,0}, 1, **1** | Unit exits tunnel |
| 19 | **UnitClass::Mission_Harvest** | varies | Harvester repositioning |
| 20 | **UnitClass::Mission_Enter** | varies (multiple sites) | Enter-transport mission |
| 21 | **UnitClass::Receive_Radio** | varies | Unit radio message |

### Key Observations

1. **ReceiveDamage scatter is the big one for gameplay** -- this is what makes units
   auto-dodge after being hit. It has extensive eligibility checks including vet abilities,
   PlayerScatter flag, and TypeClass flags at +0x29F and +0x2B1.

2. **Player scatter command (case 7)** uses `force=0`, meaning the per-class Scatter
   implementation CAN refuse it (e.g., if mission doesn't allow scatter). This matches
   the original game behavior where pressing X on a deploying unit does nothing.

3. **Factory exit scatter uses `force=1`** -- HouseClass::Update forces units off factory
   exit cells unconditionally, which is critical for production flow.

4. **Tunnel exit scatter uses `force=1`** -- units emerging from tunnels force everything
   at the exit to scatter.

5. **Idle scatter runs every 64 frames** -- this is the background "shuffling" of idle units,
   gated by the mission's Scatter flag from MissionControlClass.

6. **The `coord` parameter varies by caller**: some pass NullCoord (scatter in any direction),
   some pass {0,0,0}, some pass a specific coordinate (scatter AWAY from that point), and
   some pass global constants whose values need further investigation.

### Global Coordinate Constants Used

All verified to be `{0, 0, 0}` (NullCoord / ZeroCoord):

| Address | Used By | Value (verified) |
|---------|---------|-------------------|
| 0x008a39b8 | Network scatter command (case 7) | `{0, 0, 0}` |
| 0x008b3da8 | FUN_004D82B0 (deferred scatter) | `{0, 0, 0}` |
| 0x00a8f200 | FUN_005200b0 (garrison timer) | `{0, 0, 0}` |
| g_NullCoord variants | Multiple callers | `{0, 0, 0}` |

**Conclusion**: Every direct Scatter caller passes either `{0,0,0}` or a specific
building/exit coordinate. The `{0,0,0}` case means "scatter in a random direction"
(the per-class Scatter handler detects NullCoord and picks a random direction).
When a real coordinate is passed, it's used as a direction hint (scatter AWAY from
that coordinate).
