# UnitClass__Mission_Harvest — Decode Doc

**Proposed Ghidra label:** `UnitClass__Mission_Harvest`

## Summary

`UnitClass__Mission_Harvest` (`0x0073E5E0`) is the per-tick mission handler for harvest
units. It drives a 5-state machine stored in the UnitClass substate byte at offset `+0xBC`
(`param_1[0x2f]`, since `param_1` is `int *`; byte offset = `0x2F × 4 = 0xBC`). The five
states are: **0 = SCAN** (search for ore), **1 = HARVEST** (dig at cell), **2 = RETURN**
(navigate to refinery), **3 = DOCK** (enter dock link), **4 = WAIT** (mission handoff after
docking). State 2 contains the critical chrono-vs-normal close/far branching logic that is
the focus of this system.

The function returns the number of ticks to wait before being called again, drawn from
`MissionClass__GetMissionTimerEntry` plus a random 0–2 jitter.

## Active in YR

**Yes — active in standard YR skirmish.** Verified by:
- Vtable slot at `0x007f5e94` contains `0x0073E5E0` (confirmed via `read_memory 0x007f5e8c`,
  16 bytes; `e0e57300` at offset +8 in that read = LE `0x0073E5E0`).
- `UnitClass__Mission_Guard_Harvester` (`0x00740810`) also reads the same `TypeClass+0xCD4`
  Teleporter flag and calls `FootClass__Mission_Guard`, confirming the harvest → guard
  transition is wired into a live call graph.
- `MissionClass__GetMissionTimerEntry` (`0x005B3A00`) has > 40 callers across all
  Mission_* functions, all live in YR (verified via `get_function_callers 0x005B3A00`).

No TS-only gates detected in this function. `SlaveManagerClass__HandleReturnedSlaves` is
called from the early-return path for slave miners (non-chrono path); this is YR-live for
the slave miner unit class, not TS-legacy.

## Decompilation Excerpt

```c
// from decompile_function 0x0073E5E0

int __fastcall UnitClass__Mission_Harvest(int *param_1)
{
  // param_1 is int* — all param_1[N] offsets are N×4 byte offsets from UnitClass base

  iVar8 = param_1[0x1b1];  // UnitClass+0x6C4 = pointer to UnitTypeClass (TypeClass ptr)

  // --- EARLY EXIT: slave miner or no owner house ---
  if (((*(char *)(iVar8 + 0x5ed) == '\0') ||   // TypeClass+0x5ED: IsNaval?
       (*(char *)(iVar8 + 0x5ec) == '\0')) ||   // TypeClass+0x5EC: IsLand?
      (param_1[0xb6] == 0)) {                   // UnitClass+0x2D8: OwnerHouse ptr
    // ... handle no-refinery or slave-manager return ...
    return 1 or 0x1c2;
  }

  // --- MAIN DISPATCH on substate (UnitClass+0xBC) ---
  // param_1[0x2f] = *(int*)((int)param_1 + 0xBC)

  iVar8 = param_1[0x1b1];        // UnitTypeClass ptr
  cVar1 = *(char *)(iVar8 + 0xcd4);  // TypeClass+0xCD4: Teleporter flag

  switch(param_1[0x2f]) {  // UnitClass+0xBC: substate

  case 0: // SCAN — find ore cell and begin harvest or re-enter harvest
    // If locomotor CLSID == CLSID_TeleportLocomotion (0x00818858):
    //   compare CLSID bytes, if match and param_1[0x169]!=0 → SetDestination(0,1)
    //   else → FootClass__Search_For_Tiberium_And_Move
    //       with scan-radius = Rules+0x177C cells
    // If not teleport loco: FootClass__Search_For_Tiberium_Short_And_Move
    ...

  case 1: // HARVEST — tick ore extraction at current cell
    if (param_1[0x3e] < 9) return 1;  // UnitClass+0xF8: wait counter
    cVar1 = UnitClass__Harvest_Ore_Tick();
    if (cVar1 == '\0') {
        // ore depleted at cell — transition to RETURN (substate 2) or search again
        ...
        param_1[0x2f] = 2;
    }
    break;

  case 2: // RETURN — navigate to refinery
    // Find refinery: (*vtable+0x528)(TypeClass+0x3EC_list+0x3EC, 1000, 0, 0)
    piVar3 = FindFirstBuilding(refinery_type, ...);
    if (piVar3 == NULL) goto default;

    if (cVar1 == '\0') {
        // NON-CHRONO path: use HarvesterTooFarDistance (Rules+0xD78)
        dist = distance(miner_GetCoords, refinery_GetCoords);
        if (dist <= Rules[0xD78] * 0x100) goto RADIO_ACCEPT;  // close: send accept radio
        // else: far → find passable cell near refinery QueueingCell
    } else {
        // CHRONO path: use ChronoHarvTooFarDistance (Rules+0xD7C)
        dist = distance(miner_GetCoords, refinery_GetCoords);
        if (dist <= Rules[0xD7C] * 0x100) goto RADIO_ACCEPT;  // close: send accept radio
        // else: far → find passable cell near refinery QueueingCell, or stop
    }

RADIO_ACCEPT:
    // vtable+0x278 = ReceiveRadio/SetDestination, msg=2 (accepted dock)
    iVar8 = (*vtable+0x278)(2, piVar3);
    if (iVar8 == 1) { param_1[0x2f] = 3; }  // → DOCK

    // FAR fallback path:
    // bump g_MapEditorMode+1 (suppresses pathfind zone check)
    // find refinery again with g_MapEditorMode flag
    // if dist > 0x300 or chrono: use QueueingCell (refinery[0x27..0x29] + art+0x1618/0x161C)
    //    → FootClass__Find_Nearby_Passable_Cell → SetDestination
    goto default;

  case 3: // DOCK — issue vtable+0x1E8(7,0) = enter dock
    (*vtable+0x1E8)(7, 0);
    break;

  case 4: // WAIT — mission handoff, look for next refinery or transition
    ...
    (*vtable+0x1E8)(5, 0);  // change mission
  }

  return MissionTimerEntry + Random(0,2);
}
```

## Behavioral Analysis

### State machine overview

| Substate | Name    | UnitClass+0xBC value | Transition condition |
|----------|---------|----------------------|---------------------|
| 0        | SCAN    | 0                    | Ore found → 1; refinery reached if full → 2 |
| 1        | HARVEST | 1                    | Cell depleted / full → 2; more ore → stay |
| 2        | RETURN  | 2                    | Radio accepted → 3; no refinery → stay |
| 3        | DOCK    | 3                    | (Mission_Enter handles actual dock entry) |
| 4        | WAIT    | 4                    | Refinery found → 0; else → change mission |

### State 2 RETURN: chrono close/far split (load-bearing)

This is the core observable decision point. The code path is:

1. Call `FindFirstBuilding` via vtable `+0x528` to find the assigned/nearest refinery.
   - First call: `(TypeClass+0x3EC_list, 1000, 0, **0**)` — normal passability
   - Far-fallback call: `(TypeClass+0x3EC_list, 1000, 0, **1**)` — `g_MapEditorMode`
     flag bypasses zone passability check

2. Compute Euclidean 3D distance between miner's `GetCoords` (vtable `+0x48`) and
   refinery's `GetCoords` (vtable `+0x48`), using `Sqrt_Approx`. Result is in leptons.

3. **Branch on `TypeClass+0xCD4` (Teleporter flag):**

   **Non-chrono (`Teleporter == 0`):**
   - Threshold: `Rules[0xD78] * 0x100` leptons = `HarvesterTooFarDistance × 256` leptons
   - Default `HarvesterTooFarDistance = 5` → 5 × 256 = 1280 leptons = 5 cells
   - If `dist ≤ threshold` → **close path**: send radio msg `0x02` (accept dock) via
     vtable `+0x278`; on success → substate 3
   - If `dist > threshold` → **far path**: use `g_MapEditorMode` trick, find passable
     cell near QueueingCell, issue `SetDestination`

   **Chrono (`Teleporter != 0`):**
   - Threshold: `Rules[0xD7C] * 0x100` leptons = `ChronoHarvTooFarDistance × 256` leptons
   - Default `ChronoHarvTooFarDistance = 50` → 50 × 256 = 12800 leptons = 50 cells
   - Same branching logic as non-chrono, but the 10× larger threshold means the chrono
     miner almost always takes the **close (radio accept)** path
   - Additional check at far fallback: `if (0x300 < dist || cVar1 != '\0')` — if distance
     > `0x300` leptons (3 cells) **or** unit is a chrono miner, always use QueueingCell
     path regardless of distance

4. **Radio accept path (close):**
   - `(*vtable+0x278)(2, piVar3)` — this is `SetDestination(2, refinery_ptr)`
   - Return value `1` → transition to substate 3 (DOCK)
   - `0x02` is the "accepted dock" radio message (see `decode-enum-harvest-substate`)

5. **QueueingCell fallback path (far):**
   - Read refinery `QueueingCell` art offset: `piVar3[0x148]` = UnitTypeClass art ptr,
     `art + 0x1618` = QueueingCell X, `art + 0x161C` = QueueingCell Y (short values)
   - Add to refinery NW cell: `piVar3[0x27]` = X cell, `piVar3[0x28]` = Y cell
   - Convert to leptons: `cell * 0x100 + 0x80` (center of cell)
   - Call `FootClass__Find_Nearby_Passable_Cell` to find walkable cell near QueueingCell
   - Issue `SetDestination` via vtable `+0x480` with resulting cell's `CellClass` pointer

### State 0 SCAN: locomotor CLSID check

In state 0, if the unit does NOT have the teleport locomotor, it falls through to the CLSID
comparison loop (`code_r0x0073e828`). This 4-DWORD GUID compare checks whether the current
locomotor CLSID equals `CLSID_TeleportLocomotion` at `DAT_00818858` (verified: xref from
`0073e7b7` in `get_xrefs_to 0x00818858`). If the miner has swapped to a non-teleport
locomotor (e.g., drive) but was previously a chrono unit, the state-0 scan uses
`FootClass__Search_For_Tiberium_And_Move` with `Rules+0x177C` radius. The `DriveLocomotion`
piggybacker interface is queried first to obtain the locomotor CLSID.

### State 1 HARVEST: wait counter and ore tick

`param_1[0x3e]` (`UnitClass+0xF8`) is a frame counter incremented by the mission timer.
It must reach `≥ 9` before `UnitClass__Harvest_Ore_Tick` is called. This is the wait gate
before ore extraction begins. `UnitClass__Harvest_Ore_Tick` uses `Rules+0x1520`
(`HarvesterLoadRate`) for the per-ore-tick timer reload value (verified via
`RulesClass__ReadGeneral 0x0066D530` grep).

### g_MapEditorMode trick in state 2 far path

The code does `g_MapEditorMode = g_MapEditorMode + 1` before the second `FindFirstBuilding`
call and `g_MapEditorMode = g_MapEditorMode - 1` after. This suppresses the normal zone
passability check inside `Find_First_Building`, allowing the refinery to be found even when
the miner is in a different zone. This is an internal mechanism with no observable output
beyond finding the refinery when zone-separated.

## Struct Field Accesses

All via `param_1` which is `int *` — byte offset = field index × 4.

| Field | Byte offset | Frame | Meaning |
|-------|-------------|-------|---------|
| `param_1[0x1b1]` | `UnitClass+0x6C4` | Direct ptr | UnitTypeClass ptr (TypeClass chain) |
| `param_1[0x2f]` | `UnitClass+0xBC` | Direct byte | Harvest substate (0–4) |
| `param_1[0xb6]` | `UnitClass+0x2D8` | Direct ptr | OwnerHouse ptr |
| `param_1[0x3e]` | `UnitClass+0xF8` | Direct int | Frame wait counter (HARVEST state) |
| `param_1[0x40]` | `UnitClass+0x100` | Direct int | Harvest timer start frame |
| `param_1[0x41]` | `UnitClass+0x104` | Direct int | Harvest Y cell (coord) |
| `param_1[0x42]` | `UnitClass+0x108` | Direct int | Harvest timer duration |
| `param_1[0x43]` | `UnitClass+0x10C` | Direct int | Harvest timer end value |
| `param_1[0x169]` | `UnitClass+0x5A4` | Direct ptr | IsMoving / locomotor-in-motion flag |
| `param_1[0x86]` | `UnitClass+0x218` | Direct ptr | Current target / dock building ptr |
| `param_1[0x87]` | `UnitClass+0x21C` | Direct ptr | Assigned refinery ptr |
| `param_1[0x19d]` | `UnitClass+0x674` | Direct ptr | Locomotion interface ptr |
| `param_1[0xf4]` | `UnitClass+0x3D0` | Direct byte | HasEnteredDock flag (set in state 4) |
| `param_1[0x27]` | `UnitClass+0x9C` | Location (leptons) | X position (leptons, NW-cell frame) |
| `param_1[0x28]` | `UnitClass+0xA0` | Location (leptons) | Y position (leptons, NW-cell frame) |
| `param_1[0x29]` | `UnitClass+0xA4` | Location (leptons) | Z/height position |
| `TypeClass+0xCD4` | `param_1[0x1b1]+0xCD4` | Direct bool | Teleporter flag (chrono miner marker) |
| `TypeClass+0xE0E` | `param_1[0x1b1]+0xE0E` | Direct bool | IsNovaSlave? (slave miner type 1) |
| `TypeClass+0xE0F` | `param_1[0x1b1]+0xE0F` | Direct bool | IsNovaSlave2? (slave miner type 2) |
| `TypeClass+0x3F8` | `param_1[0x1b1]+0x3F8` | Direct int | Count of refinery types list |
| `TypeClass+0x3EC` | `param_1[0x1b1]+0x3EC` | Direct ptr | Ptr to refinery BuildingType list |

Refinery building fields (accessed via `piVar3` = found BuildingClass ptr):
| Field | Byte offset | Meaning |
|-------|-------------|---------|
| `piVar3[0x27]` | `Building+0x9C` | X cell (from Location frame, ÷256) |
| `piVar3[0x28]` | `Building+0xA0` | Y cell |
| `piVar3[0x29]` | `Building+0xA4` | Z height |
| `piVar3[0x148]` | `Building+0x520` | BuildingTypeClass ptr (art data ptr chain) |
| `art+0x1618` | ArtClass+0x1618 | QueueingCell X offset (short, cell-relative to NW) |
| `art+0x161C` | ArtClass+0x161C | QueueingCell Y offset (short, cell-relative to NW) |

## Globals Referenced

| Global | Address | Meaning |
|--------|---------|---------|
| `g_RulesClass_Instance` | (symbol, not raw addr) | Singleton RulesClass ptr |
| `Rules+0xD78` | via `g_RulesClass_Instance` | `HarvesterTooFarDistance` (cells; default 5) — verified `RulesClass__ReadGeneral 0x0066D530` |
| `Rules+0xD7C` | via `g_RulesClass_Instance` | `ChronoHarvTooFarDistance` (cells; default 50) — verified same |
| `Rules+0x1520` | via `g_RulesClass_Instance` | `HarvesterLoadRate` (frames per ore tick) — verified same |
| `Rules+0x1778` | via `g_RulesClass_Instance` | Ore scan radius (cells); used in Search_For_Tiberium_And_Move |
| `Rules+0x177C` | via `g_RulesClass_Instance` | Short ore scan radius (used in state 0 SCAN teleport path) |
| `g_CurrentFrameCounter` | (symbol) | Current game frame |
| `g_MapEditorMode` | (symbol) | Zone passability bypass flag (bumped +1/-1 around far-path FindFirstBuilding) |
| `CLSID_TeleportLocomotion` | `0x00818858` | 16-byte GUID for teleport locomotor — verified via `get_xrefs_to 0x00818858` (xref `0073e7b7` in Mission_Harvest) |
| `DAT_00b1cfb8`/`DAT_00b1cfba` | `0x00B1CFB8` | Invalid/sentinel cell coord (FFFF, FFFF) checked after Find_Nearby_Passable_Cell |

## INI Keys Referenced

| INI Key | INI Section | Rules Offset | Default | Meaning |
|---------|-------------|--------------|---------|---------|
| `HarvesterTooFarDistance` | `[General]` | `+0xD78` | 5 | Max cells for non-chrono miner to use radio-accept path |
| `ChronoHarvTooFarDistance` | `[General]` | `+0xD7C` | 50 | Max cells for chrono miner to use radio-accept path |
| `HarvesterLoadRate` | `[General]` | `+0x1520` | (default) | Frames per harvest tick (wait counter threshold) |

All three verified via `RulesClass__ReadGeneral` (`0x0066D530`) decompilation grep.

## Vtable Slots Used (on UnitClass/FootClass)

| vtable offset | Function | Description |
|---------------|----------|-------------|
| `vtable+0x48` | `GetCoords` | Returns leptons-frame 3D position; used for distance calc |
| `vtable+0x278` | `SetDestination` / receive-radio | Called with msg=2 (accept dock); returns 1 on success |
| `vtable+0x480` | `SetDestination` (cell variant) | Called with CellClass ptr for move target |
| `vtable+0x528` | `FindFirstBuilding` | Find nearest refinery of given type |
| `vtable+0x1e8` | `ChangeMission` | Issue mission change (5=Harvest, 7=Enter, 0xF/0x14/0xA=others) |
| `vtable+0x1bc` | `Stop` (or similar) | Called in Mission_Guard_Harvester when chrono miner docks |
| `vtable+0x2b4` | `GetStorageRatio` | Returns fill ratio as float10; 1.0 = full |
| `vtable+0x338` | `Search_For_Tiberium_And_Move` (virtual) | Used in HARVEST→RETURN for full-storage case |
| `vtable+0x84` | `GetHouse` | Get owning house |

## Out-of-Scope Refs

These symbols are referenced but belong to other decode tasks:

- `UnitClass__Harvest_Ore_Tick` (`0x0073D450`) → task #45
- `FootClass__Search_For_Tiberium_And_Move` (`0x004DCFE0`) → task #46
- `FootClass__Search_For_Tiberium_Short_And_Move` (`0x004DDB90`) → task #47
- `FootClass__Find_Nearby_Passable_Cell` (`0x0056DC20`) → task #4
- `SlaveManagerClass__HandleReturnedSlaves` (`0x006B0DB0`) — out of scope (slave miner)
- `DriveLocomotionClass__Release_Piggybacked_Helper` (`0x004B4D50`) — locomotion scope
- `FUN_00703590` (`0x00703590`) — appears to be a `FindQueueingCell` helper; out of scope
- `HouseClass__CountOwnedInstances` (`0x0049FAE0`) — house management scope
- `TechnoClass__SetGhostCell` (`0x0070C610`) — rendering/ghost scope

## Unverified Claims (YELLOW)

- The exact meaning of `TypeClass+0x5EC` and `TypeClass+0x5ED` as "IsLand"/"IsNaval" is
  inferred from field position and usage context; not directly traced to INI read site.
- `param_1[0xf4]` labeled `HasEnteredDock` — this is inferred from being set to 1 in
  state 4 before mission change; the exact field name in the struct is unknown.
- `param_1[0x1ae]` — brief reference in `Mission_Guard_Harvester`; meaning as a per-tick
  boolean flag is inferred, not traced.
- `Rules+0x1778` vs `Rules+0x177C` as two different scan-radius fields — the exact INI
  key names for these two offsets were not grep-confirmed (only `Rules+0xD78`/`0xD7C`/
  `0x1520` were confirmed via the ReadGeneral decompilation).
- The distance comparison in state 2 uses 3D Euclidean distance. Z axis is included in
  the decompiled sum. Whether Z is always 0 in practice (flat maps) is not verified.
