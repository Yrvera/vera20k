# Scatter Trigger Points in Locomotor Processing -- Ghidra Report

## Overview

This report documents every call site where `CellClass::Scatter_Objects` is invoked
from locomotor/unit processing code in gamemd.exe. Scatter is the mechanism by which
a moving unit forces other units out of its path.

## CellClass::Scatter_Objects Signature (0x00481670)

```c
void __thiscall CellClass::Scatter_Objects(
    CellClass* this,        // ECX -- the cell to scatter occupants from
    CoordStruct* coord,     // param_2 -- direction hint passed through to each occupant's Scatter()
    int threat,             // param_3 -- passed through as param_3 to Scatter (typically 1)
    int force,              // param_4 -- if nonzero: force scatter unconditionally
    char on_bridge          // param_5 -- 0=ground occupants (+0xE4), 1=bridge occupants (+0xE8)
);
```

### Internal logic

1. **If `force` is 0**: first pass scans occupants to check if any "should scatter"
   (via `FUN_0040dd70` + `FUN_00750010` = IsFootClass + IsTechnoScatterable). Sets `bVar1=true` if found.
2. Second pass builds an array of up to 10 occupants (filtering by `IsObject` check).
3. For each occupant, calls `vtable+0x174` = `ObjectClass::Scatter(coord, threat, force)` if ANY of:
   - `bVar1` is true (another eligible unit was found in cell)
   - `force` is nonzero
   - `Rules->ScatterEnabled` (+0x17ED) is true
   - The occupant is a Techno with `HasWeaponAbility(3)` (JumpJet?) **OR** its `TypeClass->Size` (+0x24C)
     is >= `Rules->MinScatterSize` (+0x144C)

### Key insight
When `force=1`, the initial eligibility scan is skipped entirely and ALL occupants
get their `Scatter()` method called unconditionally.

---

## Complete Scatter Trigger Table

| # | Function | Address | Can_Enter_Cell Code | Trigger Condition | coord (param_2) | threat (param_3) | force (param_4) | on_bridge (param_5) | After Scatter |
|---|----------|---------|--------------------|--------------------|------------------|-------------------|------------------|----------------------|---------------|
| 1 | DriveLocomotionClass::Process_Movement | 0x4b2dc0 | 6 (BLOCKED_BY_FRIENDLY) | Initial pathfinding: next cell blocked by friendly, close to dest, has valid path, ally in cell, not NoMoveToTarget | NullCoord {0,0,0} | 1 | 1 | bridge_check* | Decrements retry counter (64C), then continues movement attempt |
| 2 | DriveLocomotionClass::Process_Movement | 0x4b327d | 6 (BLOCKED_BY_FRIENDLY) | Re-pathfinding branch: similar to #1 but on the second pathfinding attempt (next path step) | NullCoord {0,0,0} | 1 | 1 | bridge_check* | Sets retry_counter=10 (64C), continues |
| 3 | DriveLocomotionClass::Process_Movement | 0x4b393a | 6 (BLOCKED_BY_FRIENDLY) | Main movement: Can_Enter_Cell returned 6, close to destination (< CloseEnough), not on bridge-occupied cell type 10 | NullCoord {0,0,0} | 1 | 1 | bridge_check* | Jumps to LAB_004b3607: clears head-to coord, stops movement |
| 4 | DriveLocomotionClass::Process_Movement | 0x4b4437 | 6 (BLOCKED_BY_FRIENDLY) | Late movement: secondary Can_Enter_Cell on next-next cell also returned 6, close to dest | NullCoord {0,0,0} | 1 | 1 | bridge_check* | Jumps to LAB_004b41b3: clears path, resets track index |
| 5 | DriveLocomotionClass::Process_Drive_Track | 0x4b1f43 | case 6 in track-step switch | Mid-track: during drive track delta stepping, the arrival cell has a Can_Enter_Cell result of 6 | NullCoord {0,0,0} | 1 | 1 | bridge_check* | Increments track step counter, continues track loop |
| 6 | WalkLocomotionClass::ProcessMovement | 0x75b891 | 6 (BLOCKED_BY_FRIENDLY) | Infantry movement: Can_Enter_Cell returned 6, close to destination, ally blocking | NullCoord {0,0,0} | 1 | 1 | bridge_check* | Returns immediately (infantry stops after scatter) |
| 7 | UnitClass::PerCellProcess | 0x74177a | N/A (not from CEC) | Unit entering cell has IsCrusher + can-crush-occupant, AND param_3=1 (entering), AND cell has sub-cell occupants (byte & 0x1F != 0) | NullCoord {0,0,0} | 1 | 0 | 0 or 1 (see below) |Returns immediately after scatter |

### * Bridge check pattern (identical at all 6 locomotor scatter sites)

All locomotor scatter calls use the same bridge detection logic to compute `on_bridge`:

```
on_bridge = 0;  // default: ground
if (cell->Flags_140 & 0x100) {   // cell has bridge overlay
    on_bridge = 1;
    int height_in_levels = mover->Coords.Z / DriveHeightStep;
    int bridge_level = (signed char)cell->field_11B;
    if (abs(height_in_levels - bridge_level) < 3) {
        on_bridge = 0;  // close to bridge level means ON bridge? No -- reset to ground
    }
}
```

The logic: if the cell has a bridge (`Flags & 0x100`), check whether the mover's Z
divided by DriveHeightStep is within 3 levels of the cell's bridge level (field 0x11B).
If the difference is < 3, the mover is ON the bridge surface, so `on_bridge = 0`
(scatter ground occupants -- the ones that would be in the way at bridge level).
If >= 3, the mover is UNDER the bridge, so `on_bridge = 1` (scatter bridge occupants).

Wait -- re-reading more carefully: when `on_bridge = 0`, Scatter_Objects reads from
`cell+0xE4` (FirstObject = ground layer). When `on_bridge = 1`, it reads from
`cell+0xE8` (BridgeObject = bridge layer). So the bridge flag selects WHICH occupant
list to scatter from.

The condition: if height difference < 3 levels, `on_bridge` stays 0 (default) = scatter
ground layer. If >= 3 levels, `on_bridge` is set to 1 = scatter bridge layer. This means:
- A unit close to bridge height scatters the ground layer (because it IS on the bridge
  traveling alongside ground-layer units... actually this is the "on bridge" case)
- A unit far from bridge height scatters the bridge layer

This needs more investigation to fully resolve the semantics; the key point is that ALL
6 locomotor sites use the exact same bridge detection pattern.

### UnitClass::PerCellProcess bridge handling (site #7)

Different from the locomotor pattern. The bridge flag is determined by `param_3` to
PerCellProcess and a bridge height check:

```c
bool on_bridge = false;
if (cell->Flags_140 & 0x100) {
    if (unit->IsOnBridge == false) {  // +0x8C via IsOnGround check
        // check if going UP onto bridge
        CellClass* next = GetCellClass(unit->GetCoords());
        if (next->field_11B == cell->field_11B + 4) {
            on_bridge = true;  // scatter bridge occupants
        }
    } else {
        on_bridge = true;  // already on bridge
    }
}
```

Then:
- If `on_bridge=true` AND `cell+0x128 & 0x1F != 0`: scatter with `on_bridge=1`
- If `on_bridge=false` AND `cell+0x124 & 0x1F != 0`: scatter with `on_bridge=0`

The `cell+0x124` and `cell+0x128` are sub-cell occupation bitmasks (ground and bridge respectively).

---

## Detailed Analysis Per Call Site

### Site 1: DriveLocomotionClass::Process_Movement @ 0x4b2dc0

**Context**: First pathfinding attempt. The unit has a destination and is computing
its initial path. After `FootClass::Find_Path` succeeds and picks the first direction,
it calls `Can_Enter_Cell`. If the result is 6 (blocked by friendly):

1. Gets the next cell's CellClass pointer
2. Checks if the blocking object is an ally (`HouseClass::Is_Ally`)
3. Checks the mover's TypeClass has `NoMoveToTarget` == false (+0xC94)
4. Computes distance to destination -- if close (< `Rules->CloseEnough` at +0x1718)
   AND path has no valid steps remaining, checks if mover is close in Z to ground
   (within 2 * DriveHeightStep). If close to ground AND cell type != 10 (tiberium):
   **stops movement entirely** instead of scattering.
5. Otherwise: does the bridge check, then calls `Scatter_Objects`.

**After scatter**: Decrements the "scatter retry counter" at FootClass+0x64C. If counter
was already <= 0, clears head-to coord and gives up. Otherwise continues to the
drive track selection code.

### Site 2: DriveLocomotionClass::Process_Movement @ 0x4b327d

**Context**: Secondary pathfinding / retry branch. Similar to Site 1 but reached when
the unit already had a path entry in its queue. The path step direction is read from
the path queue at FootClass+0x5E0. Same ally/distance/bridge checks.

**After scatter**: Sets `FootClass+0x64C = 10` (retry counter = 10 frames), then
falls through to the drive track selection code.

### Site 3: DriveLocomotionClass::Process_Movement @ 0x4b393a

**Context**: Main movement execution. The unit has selected a drive track direction
and called `Can_Enter_Cell` on the target cell. Result code 6 reached inside the
`case 6` handler of the Can_Enter_Cell result switch.

**Flow**: Can_Enter_Cell == 6 -> checks TypeClass->NoMoveToTarget -> if not no-move,
computes distance to destination. If close (< CloseEnough) AND close in Z -> same
"stop if not tiberium" check. Then bridge check -> Scatter_Objects.

**After scatter**: Jumps to LAB_004b3607 which clears the head-to coord and resets
the drive track state. The unit effectively stops and waits for the scattered unit to move.

### Site 4: DriveLocomotionClass::Process_Movement @ 0x4b4437

**Context**: Late/secondary Can_Enter_Cell check. This is in the code path where the
unit is checking the NEXT-NEXT cell (looking ahead two cells). Same CEC==6 pattern.

**After scatter**: Jumps to LAB_004b41b3 which clears the entire path queue
(`FootClass+0x5E0 = -1`), resets drive track index to -1, clears head-to coord.
The unit fully stops and will need to re-pathfind on the next tick.

### Site 5: DriveLocomotionClass::Process_Drive_Track @ 0x4b1f43

**Context**: Inside the drive track stepping loop. While consuming movement budget
and applying dx/dy deltas, the code checks `Can_Enter_Cell` on the cell being entered.
This is in the **case 6** of a switch statement on the CEC result, inside the
per-step `do { ... } while (budget > 7)` loop.

**Assembly** (confirmed):
```asm
push EDX          ; on_bridge (computed from bridge check)
push 0x1          ; force = 1
push 0x1          ; threat = 1
push 0x8a0790     ; coord = &NullCoord
push ESI          ; CellClass ptr (from MapClass::Get_CellClass)
call 0x00565730   ; MapClass::Get_CellClass
mov ECX, EAX      ; this = cell
call CellClass__Scatter_Objects
```

**After scatter**: Increments `track_step_index` (+0x5C), continues the drive track
loop. The unit does NOT stop; it keeps consuming movement budget for this frame.

### Site 6: WalkLocomotionClass::ProcessMovement @ 0x75b891

**Context**: Infantry movement. After computing the next cell from the path queue,
calls `Can_Enter_Cell`. If result == 6:

1. Gets next cell's CellClass
2. Checks if `local_10` (a "first attempt" flag from earlier re-pathfinding logic)
   is nonzero -- if so, clears path and recursively calls ProcessMovement(0) instead
3. Computes distance to destination. If close AND Z-close AND cell type != 10: stops.
4. Otherwise: bridge check -> Scatter_Objects.

**Assembly** (confirmed):
```asm
push EDX          ; on_bridge
push 0x1          ; force = 1
push 0x1          ; threat = 1
push 0xb45be8     ; coord = &NullCoord (Walk variant)
mov ECX, ESI      ; this = CellClass
call CellClass__Scatter_Objects
```

**After scatter**: Returns immediately. The infantry unit stops for this tick.
On the next tick it will re-enter ProcessMovement and re-evaluate.

### Site 7: UnitClass::PerCellProcess @ 0x74177a

**Context**: This is NOT triggered by Can_Enter_Cell at all. It fires when a unit
enters a cell (PerCellProcess is called by the movement system when a unit crosses
a cell boundary). The unit must have `IsCrusher` set (TypeClass+0xD28 is true OR
`HasWeaponAbility(0x11)` is true -- OmniCrusher ability).

**Trigger conditions**:
- `param_3 == 1` (entering cell, not leaving)
- Cell has sub-cell occupants: `cell+0x124 & 0x1F != 0` (ground) or `cell+0x128 & 0x1F != 0` (bridge)

**Assembly** (confirmed):
```asm
push 0x1          ; on_bridge = 1 (bridge variant) or 0x0 (ground variant at 0x741795)
push 0x0          ; force = 0 (!!)
push 0x1          ; threat = 1
push 0xb1cfe8     ; coord = &NullCoord (Unit variant)
mov ECX, ESI      ; this = CellClass
call CellClass__Scatter_Objects
```

**Key difference**: `force = 0`. This means the scatter eligibility check in
Scatter_Objects runs. Only units that pass the size/weapon check actually scatter.
This makes sense for crushers: you want infantry to scatter out of the way, but only
if they're small enough to be scared (large units that can't be crushed won't flee).

**After scatter**: Returns. The actual crushing logic follows separately in the same
function for units that did NOT scatter (iterates occupants, applies crush damage, etc.)

---

## UnitClass::Scatter_Force (0x00738970) -- vtable+0x484

This is a completely separate method from `CellClass::Scatter_Objects`. It is called
**on the mover itself** (not on the cell), and it does NOT call Scatter_Objects.

### When called

Only referenced from the UnitClass vtable at offset 0x484. This is the virtual method
`FootClass::ShouldScatter()` / scatter-force handler. It is called from:

- `DriveLocomotionClass::Process_Movement` at multiple points via `vtable+0x484` --
  this happens when the mover ITSELF needs to give up and scatter (e.g., path blocked,
  destination unreachable). This is the **opposite** of Scatter_Objects: instead of
  telling others to move, the mover tells ITSELF to move elsewhere.

### What it does

1. Checks if unit is deploying, in special mission (0x1C), etc. -- early-outs.
2. If unit is stopped (`Mission == 2 = Guard`) and has no path: picks a scatter
   destination using facing direction, random offset, subcell placement.
3. If unit has Jumpjet/BalloonHover: different handling.
4. If path exists but is blocked: clears target, issues `Enter_Idle_Mode`, assigns
   new mission (5=Move, 0xB=Hunt, 0x10=Retreat, 0xA=EnterTransport depending on context).
5. Handles the special case of units heading to a repair depot that is occupied.

### Key distinction

| Method | Called On | Purpose |
|--------|----------|---------|
| `CellClass::Scatter_Objects` | The cell being entered | Tell OTHER units to get out of the mover's way |
| `UnitClass::Scatter_Force` | The mover itself | Tell the MOVER to give up and move elsewhere |

---

## Summary: All CellClass::Scatter_Objects Call Sites from Locomotors

Every locomotor scatter call:
- Passes `NullCoord` as the direction hint (meaning: scatter in any direction)
- Passes `threat = 1`
- Passes `force = 1` (EXCEPT PerCellProcess which passes `force = 0`)
- Computes `on_bridge` via the standard bridge height check (EXCEPT PerCellProcess which
  uses its own bridge-awareness logic)
- Is triggered specifically by `Can_Enter_Cell` returning **6** (BLOCKED_BY_FRIENDLY)
  (EXCEPT PerCellProcess which checks sub-cell occupation bitmask instead)

The mover's behavior after calling Scatter_Objects varies:
- **Process_Drive_Track**: keeps going (does not stop mid-track)
- **Process_Movement (sites 1-4)**: generally stops or retries pathfinding
- **WalkLocomotionClass**: stops, retries next tick
- **PerCellProcess**: continues to crusher logic for remaining occupants
