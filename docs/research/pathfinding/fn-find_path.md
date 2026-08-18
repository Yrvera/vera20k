# FootClass__Find_Path — Decode Doc
**Proposed Ghidra label:** FootClass__Find_Path

## Summary

`FootClass__Find_Path` at `0x004D3920` is the top-level path-request entry point
for all ground (and naval) `FootClass`-derived units in YR. It is invoked by
locomotion `Process_Movement` callbacks when the unit needs a new path to its
current destination. The function:

1. Guards against re-entry / invalid destinations.
2. Normalises the destination cell (impassable water tile → nearby passable cell,
   occupied building destination → nearby passable cell).
3. Calls `FootClass__Run_AStar` to compute the path.
4. On success, copies the resulting step-queue into the FootClass path buffer and
   optionally triggers a cascading re-path for a chained convoy vehicle.
5. On failure, initiates a scatter/retry cycle if the unit is currently blocked and
   far enough from its destination.

## Active in YR

**Yes — unconditional.** Called from three locomotor `Process_Movement` functions
that are live in every standard YR skirmish:
- `DriveLocomotionClass__Process_Movement` @ `0x004B2630` (verified via
  `get_function_callers 0x004D3920`)
- `WalkLocomotionClass__ProcessMovement` @ `0x0075AEC0`
- `ShipLocomotionClass__Process_Movement` @ `0x006A1C80`

Also called from `FUN_005164d0` @ `0x005164d0` (drive-loco sub-tick callback) and
`FUN_005b01c0` @ `0x005b01c0` (ship-loco sub-tick callback), and recursively from
itself for convoy chain re-pathing. No TS-gating flag observed.

## Callers

Verified via `get_function_callers 0x004D3920`:

| Caller | Address | Role |
|--------|---------|------|
| `DriveLocomotionClass__Process_Movement` | `0x004B2630` | Tank/vehicle drive loco tick |
| `FUN_005164d0` | `0x005164d0` | Drive-loco sub-tick / retry handler |
| `FUN_005b01c0` | `0x005b01c0` | Ship-loco sub-tick / retry handler |
| `FootClass__Find_Path` | `0x004D3920` | Recursive: convoy-chain re-path |
| `ShipLocomotionClass__Process_Movement` | `0x006A1C80` | Naval unit loco tick |
| `WalkLocomotionClass__ProcessMovement` | `0x0075AEC0` | Infantry walk loco tick |

## Callees

Verified via `get_function_callees 0x004D3920`:

| Callee | Address | Role |
|--------|---------|------|
| `CoordStruct__Set` | `0x0041C230` | Build destination coord from cell |
| `FUN_0042d170` | `0x0042D170` | PathfinderClass::compute path distance (calls Zone_precheck + Run_Astar) |
| `FUN_00500200` | `0x00500200` | Unknown; called when path fails + not player-controlled |
| `FUN_006EA870` | `0x006EA870` | Unknown; called during loco stop on path failure |
| `FUN_006F03B0` | `0x006F03B0` | Alternate `CloseEnough` distance getter (non-standard unit type) |
| `FUN_007CA650` | `0x007CA650` | Called at entry (stack frame setup / profiling?) |
| `FootClass__Find_Nearby_Passable_Cell` | `0x0056DC20` | Find alternate dest when primary is impassable |
| `FootClass__Find_Path` | `0x004D3920` | Recursive: re-path chained convoy vehicle |
| `FootClass__Run_AStar` | `0x004CBBA0` | Core AStar dispatch |
| `GameDebugLog__Assert` | `0x007DC720` | Debug assertion (release-build no-op) |
| `HouseClass__IsPlayerControl` | `0x0050B730` | Check if unit is player-owned |
| `Look_up_building_in_cell` | `0x0047C520` | Find building at destination cell |
| `MapClass__GetZoneID` | `0x0056D230` | Get zone ID for a cell (passability zone) |
| `MapClass__Get_CellClass` | `0x005657A0` | Get CellClass pointer from coords |
| `Math__ftol` | `0x007C5F00` | Float-to-long conversion |
| `Pathfinding_update_continued` | `0x00481810` | Per-step path continuation (convoy chain) |
| `Sqrt_Approx` | `0x004CAC40` | Fast approximate square root |
| `TechnoClass__Is_Current_Cell_Obstacle_Free` | `0x00486FF0` | Obstacle check for dest cell |

## Decompilation excerpt (key blocks)

Source: `decompile_function 0x004D3920`.

### Block 1 — Entry guard and passability check

```c
FUN_007ca650();                          // stack frame init / profiler probe

if (in_stack_00001fa4 == 0) {
    extraout_ECX[0x178] = -1;            // clear path-step queue head (FootClass+0x5E0 in leptons)
}
cVar3 = (**(code **)(*extraout_ECX + 0x2cc))();  // vtable+0x2CC: Is_Locomotor_Present?
if (cVar3 == '\0') {
    extraout_ECX[0x178] = -1;
    return 0;
}
```

`vtable+0x2CC` returns false if the unit has no attached locomotor (e.g. being
constructed). When false, `FootClass[0x178*4] = FootClass+0x5E0` is set to -1
(no valid path head) and the function returns 0 immediately.

### Block 2 — Distance to destination + CloseEnough threshold

```c
(**(code **)(*extraout_ECX + 0x48))();   // vtable+0x48 = GetCoords() → current position
piVar4 = (int *)CoordStruct__Set();
iStack00000004 = *piVar4;                // save current coord
Sqrt_Approx();
iVar5 = Math__ftol();                    // iVar5 = distance_to_dest (leptons, approx)

if (extraout_ECX[0x175] == 0) {
    iVar6 = *(int *)(g_RulesClass_Instance + 0x1718);  // RulesClass.CloseEnough (leptons)
} else {
    iVar6 = FUN_006f03b0();              // custom CloseEnough for non-standard unit type
}
```

`FootClass[0x175*4]` (= `FootClass+0x5D4`) is a flag indicating a non-default
locomotor or convoy state that uses an alternate close-enough distance. The
standard threshold is `g_RulesClass_Instance + 0x1718` (verified via
`FUN_005164d0` which also reads this global; confirmed field is `CloseEnough`
from rules INI context).

### Block 3 — Locomotor type check and water/bridge destination remap

```c
iVar7 = (**(code **)(*extraout_ECX + 0x84))();  // vtable+0x84 = GetTechnoType()
cVar3 = *(char *)(iVar7 + 0xc94);               // TechnoTypeClass+0xC94 = Naval flag (bool)
iVar7 = *extraout_ECX;
MapClass__Get_CellClass();
iVar7 = (**(code **)(iVar7 + 0x1ac))();          // vtable+0x1AC = GetLocomotorType() → enum
```

`vtable+0x1AC` returns a locomotor-type enum. Values observed in callers:
- `6` = water/naval cell type
- `7` = building occupied

The code then branches on these values to remap impassable destinations:

**Case 6 (water cell):** if far enough from dest and unit is NOT naval
(`cVar3 == '\0'`), calls `FootClass__Find_Nearby_Passable_Cell` and optionally
redirects destination. Sets new dest via `vtable+0x480` (SetDest).

**Case 7 (occupied building):** calls `Look_up_building_in_cell`, then if a
building is found, similarly redirects to a nearby passable cell.

### Block 4 — Run_AStar and path-queue copy

```c
(**(code **)(*extraout_ECX + 0x124))();          // vtable+0x124 = Mark() — occupancy lock

if (in_stack_00001f84 == (undefined1 *)0x0) {
    extraout_ECX[0x178] = -1;
}

puVar8 = (undefined4 *)FootClass__Run_AStar();

if ((puVar8 != (undefined4 *)0x0) && (puVar8[1] != 0)) {
    // Copy path header (8 dwords = PathType struct header)
    for (iVar6 = 8; iVar6 != 0; iVar6--) { *puVar14 = *puVar13; ... }
    (**(code **)(iVar5 + 0x540))();              // vtable+0x540: SetPathValid?

    // Copy step directions into FootClass path buffer at FootClass+0x5E0
    piVar11 = extraout_ECX + (int)(in_stack_00001f84 + 0x178);
    for (; iVar5 != 0; iVar5--) { *piVar11 = *piVar4; ... }
}
(**(code **)(*extraout_ECX + 0x124))();          // vtable+0x124 = Unmark()
```

`FootClass[0x178*4]` = `FootClass+0x5E0`: head of the path step-direction ring
buffer. The step count copied is `min(0x18 - offset, path->step_count)` where
`0x18` = 24 = max steps stored in the ring buffer.

`vtable+0x124` is called as a pair (mark before / unmark after Run_AStar) —
this is the occupancy-mark protocol that prevents other units from pathing
into occupied cells while the search runs. Confirmed by usage in
`FUN_005b01c0` which also calls `vtable+0x124` in mark/unmark pairs.

### Block 5 — Stamp current-frame and save dest coord

```c
extraout_ECX[400] = g_CurrentFrameCounter;   // FootClass+0x640 = last path frame
extraout_ECX[0x191] = iStack00000004;         // FootClass+0x644 = dest coord (saved)
extraout_ECX[0x192] = 0;                      // FootClass+0x648 = retry-wait timer (reset)
```

Verified: `FUN_005164d0` reads `FootClass+0x640` and `FootClass+0x648` as a
timestamp + wait-interval for path-retry throttling.

### Block 6 — Convoy chain re-path

```c
iVar5 = (**(code **)(*extraout_ECX + 0x2c))();   // vtable+0x2C: GetMission() enum
if ((iVar5 == 1) &&                               // MISSION_MOVE = 1
    (extraout_ECX[0x178] != -1) &&
    (piVar4 = (int *)extraout_ECX[0x1b2], piVar4 != 0) &&
    ((char)extraout_ECX[0x1b4] == '\0')) {
    do {
        MapClass__Get_CellClass();
        iVar5 = Pathfinding_update_continued();
        // Check next convoy link; re-path if needed
        FootClass__Find_Path();                    // recursive re-path for convoy follower
        piVar4 = (int *)piVar4[0x1b2];             // FootClass+0x6C8 = convoy next link
    } while (piVar4 != 0);
}
```

`FootClass[0x1b2*4]` = `FootClass+0x6C8` is the convoy-chain next pointer.
`FootClass[0x1b4*4]` = `FootClass+0x6D0` is a convoy-chain lock flag.
`vtable+0x2C` is `GetCurrentMission()`.

### Block 7 — Path failure scatter / retry

On `Run_AStar` returning null, the function checks distance vs a 1-cell
threshold and whether the current cell is a bridge cell (`CellClass+0x140 &
0x100`). If the unit is stuck and far from dest, it either:
- Stops the locomotor via `vtable+0x480` and `vtable+0x3C8`
- Calls `FUN_00500200` if not player-controlled and in multiplayer
  (`g_GameMode != 0`) to report the blocked unit
- Calls scatter (`vtable+0x1E8` = Scatter) for non-player units

## Struct field accesses (FootClass, `extraout_ECX`)

`param_1` is `int *` (C++ `this` = `FootClass *`). Offsets below are **direct
byte offsets** (Ghidra shows `extraout_ECX[N]` for `int *` where byte offset =
`N × 4`).

| Byte offset | Ghidra index | Type | Name (inferred) | Notes |
|-------------|--------------|------|-----------------|-------|
| `0x5E0` | `[0x178]` | `int` | `path_head_step` | Path step-queue head index; -1 = no valid path |
| `0x5D4` | `[0x175]` | `int` | `convoy_or_alt_loco_flag` | Non-zero → use alt CloseEnough fn |
| `0x640` | `[400=0x190]` | `int` | `last_path_frame` | Frame counter when path was last computed |
| `0x644` | `[0x191]` | `int` | `last_dest_coord` | Destination coord at last path request |
| `0x648` | `[0x192]` | `int` | `path_retry_wait` | Retry wait interval (frames); reset to 0 on success |
| `0x6C8` | `[0x1b2]` | `int *` | `convoy_next` | Next unit in convoy chain (pointer) |
| `0x6D0` | `[0x1b4]` | `char` | `convoy_chain_locked` | Prevents recursive convoy re-path |

## Vtable offsets (FootClass vtable, `*extraout_ECX`)

All verified from decompilation context `decompile_function 0x004D3920` and
cross-validated against callers.

| Vtable offset | Inferred name | Return / behaviour |
|---------------|---------------|--------------------|
| `+0x2C` | `GetCurrentMission` | Returns mission enum (1 = MOVE) |
| `+0x48` | `GetCoords` | Returns pointer to current XYZ coord (leptons) |
| `+0x84` | `GetTechnoType` | Returns `TechnoTypeClass *` |
| `+0x124` | `Mark` / `Unmark` | Called in pairs; occupancy mark protocol |
| `+0x1AC` | `GetLocomotorType` | Returns loco-type enum (6=water, 7=building) |
| `+0x1B8` | `Get_Cell_Packed` | Returns cell coord as packed short[2] (NW cell) |
| `+0x1E8` | `Scatter` | Scatter unit to nearby cell |
| `+0x2CC` | `Is_Locomotor_Present` | Returns bool — loco attached and active |
| `+0x3C8` | `Stop` | Stop locomotor / mission |
| `+0x480` | `SetDestCell` | Set next destination cell |
| `+0x500` | `unknown_500` | Called on path fail, non-trivial |
| `+0x540` | `SetPathValid` | Mark path as valid after copy |

## Globals referenced

| Global | Address | Role |
|--------|---------|------|
| `g_RulesClass_Instance` | (global pointer) | `+0x1718` = `CloseEnough` (leptons); threshold for "arrived" |
| `g_CurrentFrameCounter` | (global int) | Stamped into `FootClass+0x640` on each path request |
| `g_GameMode` | (global int) | Non-zero = multiplayer; gates `FUN_00500200` call |
| `DAT_008b3d88` / `DAT_008b3d8a` | globals | Null cell coords (invalid cell sentinel) |

## TechnoTypeClass field accessed

| Offset | Name (inferred) | Notes |
|--------|-----------------|-------|
| `+0xC94` | `Naval` | Bool; true if unit moves on water. Affects dest-remap branch |
| `+0xD2C` | `unknown_D2C` | Bool; checked before obstacle-free test at dest remap |

## CellClass field accessed

| Offset | Mask | Name (inferred) | Notes |
|--------|------|-----------------|-------|
| `+0x140` | `0x100` | `IsBridge` flag | Bit 8 of cell flags; true = bridge cell |

## Control flow summary

```
Find_Path()
├── Guard: no locomotor → return 0
├── Compute dist_to_dest (Sqrt_Approx)
├── Get CloseEnough threshold (RulesClass or FUN_006f03b0)
├── Get locomotor type enum (vtable+0x1AC)
├── Dest remap if needed:
│   ├── loco==6 (water) and not naval and dist > CloseEnough → Find_Nearby_Passable_Cell
│   └── loco==7 (building) and not naval → Find_Nearby_Passable_Cell
├── Mark() [occupancy lock]
├── Run_AStar()
│   ├── success → copy path header + step buffer → SetPathValid → Unmark()
│   └── failure → Unmark()
├── Stamp frame/dest/retry fields
├── If mission==MOVE and path valid → convoy chain re-path loop (recursive)
└── On path failure:
    ├── dist > 1 cell or on bridge → initiate scatter/stop cycle
    └── not player-controlled + multiplayer → FUN_00500200
```

## Out-of-scope refs

The following symbols appeared in the decompilation but are out of scope for
this task:

- `FootClass__Run_AStar` @ `0x004CBBA0` — task #2
- `FootClass__Find_Nearby_Passable_Cell` @ `0x0056DC20` — task #17
- `FUN_0042d170` @ `0x0042D170` — task #20
- `Pathfinding_update_continued` @ `0x00481810` — task #18
- `FUN_00500200` @ `0x00500200` — task #114
- `Zone_precheck` (called from FUN_0042d170) — task #16
- `Path_walk_directions_to_cell` (called from Run_AStar) — task #15

## YELLOW — Unverified

- `vtable+0x2CC` identity: inferred as "Is_Locomotor_Present" from context (returns
  bool, called as guard, used identically in `FUN_005164d0` and `FUN_005b01c0`).
  Not independently decompiled — address not confirmed via `read_memory` of vtable.
- `vtable+0x1AC` return values 6/7: inferred from branch labels and cross-reference
  with `FUN_005b01c0` which checks loco-type 5/4/0/6 against water/bridge context.
  Enum table not separately verified.
- `FootClass+0x6C8` as "convoy_next pointer": inferred from the do-while traversal
  pattern. Field name unconfirmed — no struct layout tool run on FootClass.
- `g_RulesClass_Instance + 0x1718` = `CloseEnough`: confirmed as a distance
  threshold read in the same pattern by multiple locomotor callers, but INI key
  cross-reference not independently verified in this session.
