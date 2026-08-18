# Naval System Research — Ships, Water Movement & Naval Yards

Research notes from reverse-engineering gamemd.exe (Yuri's Revenge) via Ghidra MCP,
cross-referenced with rulesmd.ini / artmd.ini and the Rust engine codebase.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [ShipLocomotionClass](#2-shiplocomotionclass)
3. [ILocomotion Vtable Layout](#3-ilocomotion-vtable-layout)
4. [ShipLocomotionClass Field Layout](#4-shiplocomotionclass-field-layout)
5. [Movement Logic — Process & Pathfinding](#5-movement-logic--process--pathfinding)
6. [Drive Track System](#6-drive-track-system)
7. [Passability Matrix](#7-passability-matrix)
8. [Speed & Terrain Modifiers](#8-speed--terrain-modifiers)
9. [Naval Units (rulesmd.ini)](#9-naval-units-rulesmdini)
10. [Naval Yard Placement — WaterBound](#10-naval-yard-placement--waterbound)
11. [CellClass::IsCellSuitableForBuilding](#11-cellclassiscellsuitableforbuilding)
12. [Naval Yard Properties](#12-naval-yard-properties)
13. [Ship Rendering](#13-ship-rendering)
14. [Submarine & Underwater System](#14-submarine--underwater-system)
15. [Spawner System](#15-spawner-system)
16. [Rust Engine Readiness](#16-rust-engine-readiness)
17. [Implementation Roadmap](#17-implementation-roadmap)
18. [Key Addresses Reference](#18-key-addresses-reference)

---

## 1. Architecture Overview

The single most important finding: **ShipLocomotionClass and DriveLocomotionClass are
sibling classes that share key subroutines and data, but have distinct top-level methods.**

Shared between Ship and Drive:

- The same base class constructor (`FUN_0055a6c0`)
- The same main movement AI subroutine (`FUN_006a1c80`, ~8470 bytes) — called from both Process functions
- The same drive track execution subroutine (`FUN_006a05f0`, ~5737 bytes)
- The same drive track data (**67**-track table at `0x7F2A40` — NOT 72; entries 67–71 don't exist for Ship — 16 base curves at `0x7F2960`)

**Different** between Ship and Drive (distinct vtable entries):

- Process: Ship=`0x69FC10`, Drive=`0x4B0500` (different wrappers, same heavy subroutines)
- Move_To, Stop_Moving, Draw_Matrix, Is_Moving — all have separate implementations
- Only 16 of 32 vtable entries are shared (inherited from LocomotionClass base)

The differentiation between ships and land vehicles happens through **data** and the
per-class Process wrappers (Ship's adds wake animation, water-specific checks):

| Property | Land Vehicle | Ship |
|---|---|---|
| SpeedType | Track / Wheel / Hover | Float |
| MovementZone | Normal / Crusher / etc. | Water |
| Passability row | Land cells passable | Only water cells passable |
| Naval flag | No | Yes |
| Wake animation | No | Every 8th frame on water |

This means the Rust engine does **not** need a separate ship locomotion implementation
from scratch. The existing ground movement state machine + drive track system handles
the heavy lifting. Ship-specific behavior (wake animation, water checks) needs a thin
wrapper in the Process tick, but the core movement/track logic is shared.

---

## 2. ShipLocomotionClass

| Property | Value |
|---|---|
| CLSID | `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` |
| Constructor | `0x69EC50` |
| ILocomotion vtable | `0x7F2D8C` |
| IUnknown vtable | `0x7F2E58` |
| IPiggyback vtable | `0x7F2D68` (corrected 2026-05-29: was "IPersistStream vtable"; binary labels it `ShipLocomotionClass__IPiggyback_vtable` in constructor at `0x69EC50` via `decompile_function 0x69EC50` — RTTI_LABEL_DRIFT) |
| Null-coord sentinel | `DAT_00b077f8`, `DAT_00b077fc`, `DAT_00b07800` |

The constructor initializes the triple-vtable COM pattern (IUnknown + ILocomotion +
IPiggyback), zeroes all coordinate fields to the null-sentinel, sets drive track
index and facing to -1, and sets the "first tick" flag at offset +0x65 to 1.

---

## 3. ILocomotion Vtable Layout

Complete vtable at `0x7F2D8C` (40 entries, Ship = Drive shared implementation):

| Idx | Address | Method | Notes |
|---|---|---|---|
| 0 | `0x6A4300` | QueryInterface | COM |
| 1 | `0x6A4310` | AddRef | COM |
| 2 | `0x6A4320` | Release | COM |
| 3 | `0x55A710` | Link_To_Object | Base class |
| 4 | `0x69F290` | **Is_Moving** | Checks dest/head_to vs null sentinel |
| 5 | `0x69F3A0` | **Destination** | Returns coords at +0x30..+0x38 |
| 6 | `0x69F3D0` | **Head_To_Coord** | Returns head_to or current location |
| 7 | `0x55ABF0` | Can_Enter_Cell | Base class |
| 8 | `0x55ABE0` | Is_To_Have_Shadow | Base class |
| 9 | `0x69F670` | **Draw_Matrix** | Voxel body+turret transform, 1189 bytes |
| 10 | `0x69FB20` | **Shadow_Matrix** | Shadow rendering transform |
| 11 | `0x55ABD0` | Draw_Point | Base class |
| 12 | `0x55A8C0` | Shadow_Point | Base class |
| 13 | `0x55ABC0` | Visual_Character | Base class |
| 14 | `0x6A3EA0` | Z_Adjust | Ship-specific |
| 15 | `0x6A3EB0` | Z_Gradient | Ship-specific |
| 16 | `0x69FC10` | **Process** | Main per-tick update, 1411 bytes |
| 17 | `0x69F450` | **Move_To** | Sets dest, handles bridge Z offset |
| 18 | `0x69F510` | **Stop_Moving** | Clears dest, notifies tethered units |
| 19 | `0x6A05C0` | **Do_Turn** | Delegates to facing helper |
| 20 | `0x69FBE0` | **Unlimbo** | Reads ROT from type, applies facing |
| 21 | `0x55AB90` | Tilt_Pitch_AI | Base class |
| 22 | `0x55A8F0` | Power_On | Base class |
| 23 | `0x55A910` | Power_Off | Base class |
| 24 | `0x55A930` | Is_Powered | Base class |
| 25 | `0x55A940` | Is_Ion_Sensitive | Base class |
| 26 | `0x55AB70` | Push | Base class |
| 27 | `0x55AB80` | Shove | Base class |
| 28 | `0x6A0310` | **Force_Track** | 350 bytes, forced movement path |
| 29 | `0x6A3E50` | **In_Which_Layer** | Layer classification |
| 30 | `0x55AC00` | Force_Immediate_Destination | Base class |
| 31 | `0x69F250` | **Force_New_Slope** | Sets ramp state + timestamp |
| 32 | `0x69F350` | **Is_Moving_Now** | Current motion query |
| 33 | `0x55AD10` | Apparent_Speed | Base class |
| 34 | `0x55ACF0` | Drawing_Code | Base class |
| 35 | `0x55AD00` | Can_Fire | Base class |
| 36 | `0x4B4C60` | Get_Status | Shared |
| 37 | `0x4B4C70` | Acquire_Hunter_Seeker_Target | Shared |
| 38 | `0x4B4C80` | **Is_Surfacing** | Transport cargo proximity check (NOT submarine surfacing) |
| 39 | `0x6A3F00` | **Mark_All_Occupation_Bits** | Cell occupation |

---

## 4. ShipLocomotionClass Field Layout

Offsets relative to the ILocomotion interface pointer (vtable at index 1).

| Offset | Size | Type | Field | Notes |
|---|---|---|---|---|
| +0x00 | 4 | ptr | IUnknown vtable | `0x7F2E58` |
| +0x04 | 4 | ptr | ILocomotion vtable | `0x7F2D8C` |
| +0x08 | 4 | ptr | Owner techno | Linked FootClass* |
| +0x0C | 4 | ptr | Owner techno (2nd) | For COM delegation |
| +0x18 | 4 | ptr | IPersistStream vtable | `0x7F2D68` |
| +0x1C | 4 | int | Ramp/slope value | Current slope ID |
| +0x20 | 4 | int | Ramp timestamp | Frame when ramp started |
| +0x24 | 4 | int | (related to ramp) | |
| +0x28 | 4 | int | Ramp duration current | Remaining ticks |
| +0x2C | 4 | int | Ramp duration total | Total ticks for lerp |
| +0x30 | 12 | XYZ | **Destination** | 3 ints; null = sentinel value |
| +0x3C | 12 | XYZ | **Head-to coords** | Next waypoint |
| +0x48 | 4 | int | (reserved) | |
| +0x4C | 8 | double | **Speed accumulator** | Sub-tick movement budget |
| +0x54 | 4 | int | Track number | Forced track ID |
| +0x58 | 4 | int | **Drive track index** | -1 = no active track |
| +0x5C | 4 | int | **Track step counter** | Current step in track |
| +0x5E | 1 | bool | Scatter flag | |
| +0x5F | 1 | bool | Head-to active | |
| +0x60 | 1 | bool | Reversed flag | Reverse track direction |
| +0x61 | 1 | bool | Moving flag | |
| +0x62 | 1 | bool | (flag) | |
| +0x63 | 1 | bool | **Track active** | |
| +0x64 | 1 | bool | On-bridge transition | |
| +0x65 | 1 | bool | First-tick flag | Initialized to 1 |

---

## 5. Movement Logic — Process & Pathfinding

### Process() — Per-Tick Update (0x69FC10, 1411 bytes)

Called every game tick. Orchestrates the high-level movement state:

1. **Terrain height update** — reads cell height under unit. If changed since last tick,
   starts a 3-tick smooth ramp transition for visual height interpolation.

2. **No active track, no head-to destination:**
   - Check if docked at shipyard (mission 0x0B) — handle dock departure
   - Check if at destination (coords match location) — stop, reacquire target
   - Check scatter flag via `FUN_004c9480` — set scatter state
   - Call `FUN_006a1c80` (main movement AI) to pathfind and start a drive track

3. **Active track:**
   - Call `FUN_006a05f0` (drive track step execution) to advance along the curve
   - On track step failure, fall back to pathfind retry

4. **Wake animation** — every 8th frame (assembly uses `& 0x80000007` which is
   MSVC's signed modulo optimization for `% 8`), if:
   - Unit is alive (techno +0x90 != 0)
   - Unit is not a deployed vessel (techno type +0xd69 == 0)
   - Current mission != 0x23
   - Cell land type == 2 (water, from `cell+0xEC`)
   - `Wake=` is defined in rules (`DAT_008871e0 + 0x94 != 0`) — INI key is `Wake=WAKE1` under `[General]`
   Then spawns a wake animation via `FUN_00421ea0` at unit's location.
   **Note:** Drive Process uses actual IDIV by 10 (wake every 10 frames, not 8).
   Ship Process uses the `% 8` optimization described above.

5. **Speed decay** — if destination is null and unit is idle, gradually zeros speed
   via `vtable+0x544` (SetDesiredSpeed).

### Main Movement AI (FUN_006a1c80, ~8470 bytes)

The top-level per-tick movement handler. Ship-relevant behaviors:

**Path acquisition (no active path):**
- Applies a movement delay timer (fields +0x640, +0x644, +0x648)
- Converts destination to cell coordinates
- Calls `FUN_004d3920` (Pathfind) to compute path
- On pathfinding failure: checks `Can_Crush` / alternate movement, computes distance
  to destination, applies "close enough" threshold from `rules+0x1718` (CloseEnough)

**Cell passability check:**
- Uses `Can_Enter_Cell` (techno vtable +0x1AC) which internally checks the passability
  matrix. For ships (SpeedType=Float, MovementZone=Water), only water cells return passable.

**Can_Enter_Cell return codes:**

| Code | Meaning | Action |
|---|---|---|
| 0 | OK | Proceed with movement |
| 1 | Blocked by unit | Scatter the blocker (`FUN_00483480`) |
| 2 | Wait | Retry next tick |
| 3 | Need redirect | Repath (`FUN_00578ad0`) |
| 4-5 | Occupied | Clear path, retry immediately (recursive) |
| 6 | Cliff edge | Evaluate cliff-fall sequence |
| 7 | Impassable terrain | Stop movement |

**Bridge handling:**
- Checks `cell+0x140 & 0x100` for bridge flag
- Applies `DAT_00b0782c` (bridge Z-offset) when on/off bridge transitions
- Height difference of 3+ levels triggers bridge detection logic
- Ships with `TooBigToFitUnderBridge=true` cannot traverse bridge cells

**Speed computation:**
- Looks up terrain speed modifier: `DAT_0089ea40[SpeedType * 9 + LandType]`
- Values > 1.0 clamped to 1.0; value of 0.0 boosted to 0.5
- Applies slope modifiers: uphill uses `rules+0x768` or `+0x770`, downhill `+0x778` or `+0x780`
- Applies crowd density factor via `FUN_005f5c60`
- If below jam threshold, multiplies by "jam" penalty from `rules+0x1700`

**Drive track selection:**
- Track index = `next_direction + current_direction * 8`
- 64 turn tracks (8 directions × 8 transitions) + 8 straight tracks at `dir * 9`
- Track entry flags at `DAT_007f2a40[index * 12 + 8] & 8` controls special behavior
  (reversing / 3-point-turn)

---

## 6. Drive Track System

Ships use the **identical** drive track curves as tanks. The track data lives in two
tables:

### Track Descriptor Table (0x7F2A40)

**67 entries** (not 72 — Ship lacks entries 67–71 present in Drive), 12 bytes each: `[forward_table_idx, reverse_table_idx, facing, flags, ...]`

### Track Step Arrays (0x7F2960)

16 base curve definitions. Each entry is 16 bytes:
- `+0x00`: pointer to step array
- `+0x04`: (reserved)
- `+0x08`: step count
- `+0x0C`: max step index

Each step is 12 bytes: `[x_delta(4), y_delta(4), z_timing(4)]`

### Per-Tick Track Execution (FUN_006a05f0)

1. **Speed acceleration/deceleration:**
   - Reads max speed from `techno+0x15E` (double)
   - Applies accel rate from type `+0x308`, decel rate from `+0x300`
   - Decel steps from type `+0x678`
   - Clamps to minimums from globals `DAT_007f1308..007f1318`

2. **Track step loop:** consumes 7 sub-ticks per step
   - Reads X/Y deltas from track table
   - Converts to world coords via `FUN_006a3db0` (applies facing rotation)
   - Calls `Set_Coord` or `Move_Towards` on the techno

3. **Cell transition:** when crossing cell boundaries:
   - Marks/unmarks occupation bits (vtable +0x1B4, +0x1CC, +0x124)
   - Handles bridge flag transitions (compares `cell+0x140 & 0x100`)
   - Height comparison between cells for bridge ramp detection
   - Height diff of -4 = descending bridge ramp → sets OnBridge flag

4. **Track end:** when step deltas are both zero:
   - Unit arrived at cell center
   - Sets "at destination" flag (+0x6B6)
   - Triggers next path step or mission reacquisition

---

## 7. Passability Matrix

Hardcoded in gamemd.exe at `0x82A594`. Int32 values: **1 = passable, 2 = blocked, 3 = out-of-map**.

Rows = MovementZone-derived zone index. Columns = LandType (8 types).

```
                  Clear  Road  Rough  Water  Rail  Rock  Wall  OOB
Zone  0 (Foot):     1     2      2      2     2     2     2     3
Zone  1 (Track):    1     1      2      2     2     2     2     3
Zone  2 (Wheel):    1     1      1      2     2     2     2     3
Zone  3 (Hover):    1     1      1      1     1     1     2     3
Zone  4 (Amphib):   1     1      2      1     1     2     2     3
Zone  5 (AmphClf):  1     2      2      1     1     2     2     3
Zone  6 (Float2):   1     1      1      2     2     2     1     3
Zone  7 (FloatR):   1     2      2      2     2     1     2     3
Zone  8 (FltBch):   1     1      1      2     2     1     2     3
Zone  9 (Winged):   1     1      1      1     1     1     1     3
Zone 10 (Ship):     2     2      2      2     1     2     2     3   ← SHIPS
Zone 11 (WtrBch):   2     2      2      1     1     2     2     3   ← WATER+BEACH
```

**Row 10** (ships): only column 4 (Rail/Water) is passable. All land types blocked.
**Row 11** (water+beach): columns 3 (Water) and 4 (Rail) are passable.

---

## 8. Speed & Terrain Modifiers

The runtime speed table at `DAT_0089ea40` is populated from rules.ini terrain sections.
All zeros at binary load time — filled during `[Clear]`, `[Road]`, `[Water]`, etc. parsing.

Layout: `float[12][9]` — 12 LandType rows × 9 SpeedType columns.

| Column | SpeedType | rules.ini key |
|---|---|---|
| 0 | Foot | `Foot=` |
| 1 | Track | `Track=` |
| 2 | Wheel | `Wheel=` |
| 3 | Hover | `Hover=` |
| 4 | (hardcoded 1.0) | Ship internal |
| 5 | Float | `Float=` |
| 6 | Amphibious | `Amphibious=` |
| 7 | FloatBeach | `FloatBeach=` |
| 8 | Buildable | `Buildable=` (byte, not float) |

Row order (LandType enum):

| Index | LandType | String pointer at `0x839D68` |
|---|---|---|
| 0 | Clear | `[Clear]` |
| 1 | Road | `[Road]` |
| 2 | Water | `[Water]` |
| 3 | Rock | `[Rock]` |
| 4 | Wall | `[Wall]` |
| 5 | Tiberium | `[Tiberium]` |
| 6 | Beach | `[Beach]` |
| 7 | Rough | `[Rough]` |
| 8 | Ice | `[Ice]` |
| 9 | Railroad | `[Railroad]` |
| 10 | Tunnel | `[Tunnel]` |
| 11 | Weeds | `[Weeds]` |

Column 8 (Buildable) determines whether buildings can be placed on that terrain.
Water has `Buildable=no` by default. This is what prevents normal buildings on water.

---

## 9. Naval Units (rulesmd.ini)

### Ship Locomotor

All ships use CLSID `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}`.

Common ship properties:
- `Naval=yes` — AI/targeting classification
- `SpeedType=Float` — water terrain speed column
- `MovementZone=Water` — water-only passability
- `Locomotor={2BEA74E1-...}` — ShipLocomotionClass
- `TooBigToFitUnderBridge=true` — most ships (exceptions: DLPH has it commented out, HYD lacks it)
- `SinkingSound=GenLargeWaterDie`
- All ships `Voxel=yes` except Dolphin (SHP sprite)

### Unit Table — True Ship Locomotor Units

All use ShipLocomotionClass CLSID `{2BEA74E1-...}`:

| ID | Name | Side | Underwater | Weapons | Special |
|---|---|---|---|---|---|
| DEST | Destroyer | Allied | No | 155mm, ASWLauncher | Spawns ASW, Sensors |
| AEGIS | Aegis Cruiser | Allied | No | Medusa (AA) | Anti-air specialist |
| DLPH | Dolphin | Allied | Yes | SonicZap | Cloakable, Sensors, Voxel=no |
| CARRIER | Aircraft Carrier | Allied | No | HornetLauncher | Spawns 3× HORNET |
| SUB | Typhoon Sub | Soviet | Yes | SubTorpedo | Sensors |
| HYD | Sea Scorpion | Soviet | No | FlakTrackGun, FlakWeapon | MovementRestrictedTo=Water |
| DRED | Dreadnought | Soviet | No | DredLauncher | Spawns 2× DMISL |
| BSUB | Boomer | Yuri | Yes | BoomerTorpedo, CruiseLauncher | Dual-mode sub, Sensors |
| SQD | Giant Squid | Soviet | Yes | TentacleLash | Organic, Assaulter, Sensors |

### Amphibious Hover Transports (NOT Ship Locomotor)

These have `Naval=yes` but use **HoverLocomotionClass** `{4A582742-...}`,
`SpeedType=Hover`, `MovementZone=Amphibious`. They are NOT true ships:

| ID | Name | Side | Locomotor | Notes |
|---|---|---|---|---|
| LCRF | Landing Craft | Allied | Hover | Amphibious transport, Passengers=12 |
| SAPC | Armored Transport | Soviet | Hover | Amphibious transport, Passengers=12, Crusher=yes |

### NavalTargeting Values

Controls weapon/target priority for naval AI. Units without a `NavalTargeting=` line
use the default value (0). Only units with an explicit value in rulesmd.ini are listed:

| Value | Units |
|---|---|
| 1 | DEST, CDEST |
| 3 | SQD |
| 5 | DLPH, SUB |
| 6 | AEGIS |
| 7 | BSUB |

**Note:** CARRIER, DRED, and VLAD do **not** have NavalTargeting set in rulesmd.ini
(they use the default). LCRF and SAPC also lack it.

### Spawned Sub-Units

| Parent | Spawns | Type | Count |
|---|---|---|---|
| DEST | ASW | Anti-sub weapon | 1 |
| CDEST | ASW | Anti-sub weapon | 1 |
| CARRIER | HORNET | Aircraft | 3 |
| DRED | DMISL | Missile | 2 |
| BSUB | CMISL | Cruise missile | 2 |

### Art Data (artmd.ini)

All ship voxels use `Remapable=yes` for player color.
Key firing offsets:
- DEST: `PrimaryFireFLH=280,0,120`
- DRED: `PrimaryFireFLH=30,43,92`
- BSUB: `PrimaryFireFLH=225,65,0`, `SecondaryFireFLH=0,0,-40`
- DLPH: Exception — `Voxel=no`, `WalkFrames=6`, `FiringFrames=6` (SHP sprite)

---

## 10. Naval Yard Placement — WaterBound

### How WaterBound Works

`WaterBound` is not a separate system — **it repurposes the SpeedType field** on
BuildingTypeClass.

During INI parsing at `0x45FF94`:

```c
// Read WaterBound= bool, default = (current SpeedType == 5)
cVar5 = ReadBool(ini, "WaterBound", *(int*)(buildingType + 0x67C) == 5);
// Store: WaterBound=yes → 5, WaterBound=no → -1
*(uint*)(buildingType + 0x67C) = (-(uint)(cVar5 != '\0') & 6) - 1;
```

- Offset `+0x67C` is the **SpeedType** field (same offset as TechnoTypeClass)
- When `WaterBound=yes` → SpeedType = **5** (Float, the water speed type)
- When `WaterBound=no` → SpeedType = **-1** (no speed type; use Buildable column)

Since buildings don't move, SpeedType is normally irrelevant. WaterBound cleverly
repurposes it: the placement validator uses the **same speed/terrain passability table**
that handles unit movement to check if a cell is valid for a building.

---

## 11. CellClass::IsCellSuitableForBuilding

**Function:** `FUN_0047c620` at `0x47C620`

Called per foundation cell during building placement. This is the key function that
validates whether a specific cell can hold part of a building.

### Parameters

| Param | Type | Description |
|---|---|---|
| param_1 | CellClass* | The cell being validated |
| param_2 | int | SpeedType from BuildingTypeClass+0x67C (-1 or 5) |
| param_3 | BuildingTypeClass* | The building being placed |
| param_4 | int | Owner HouseClass index |

### Logic Flow

**Early occupancy checks (for all buildings):**
1. If map editor mode (`DAT_00a8e7ac != 0`) → always valid
2. If `BuildingType+0x16BF` set (bridge-placeable): check for wall/building overlap
3. If `BuildingType+0x16BE` or `+0x16B7` set (wall/gate): check wall adjacency
4. If `BuildingType+0xE58` != 0 (ToTile reference): check tile set compatibility
5. Otherwise: check for building occupants, wall occupants
6. Check `cell+0x124 & 0x3F` (sub-cell occupation bits) — must be zero

**Overlay compatibility checks:**
- Overlay index `cell+0x44`: pavement (0, 2) allows placement for same-owner buildings
- Overlay 0x1A (railroad?) allows placement for specific building types
- Overlay 0x7E or water-edge overlay allows placement for bridge-placeable buildings

**The critical terrain check at the end (0x47C9CD):**

```c
if (param_2 == -1) {
    // NORMAL BUILDING (WaterBound=no)
    if (no_bridge_flag && no_ramp_flag && no_subcell_flag) {
        if (buildingType->IsNaval == false) {     // +0xCCE
            // Use BUILDABLE column (index 8) of speed table
            return speed_table[cell->LandType * 0x24 + 0x20];  // DAT_0089ea60
        }
        // Naval building without WaterBound: check shore tile range
        if (cell->TileSet >= shore_start && cell->TileSet < shore_start + 14) {
            return 1;
        }
        return 0;
    }
} else {
    // WATERBOUND BUILDING (param_2 = 5 = Float SpeedType)
    // Use FLOAT column (index 5) of speed table
    if (speed_table[param_2 + cell->LandType * 9] != 0.0f) {
        return 1;  // Cell is valid water
    }
}
return 0;
```

### Key Insight

For WaterBound buildings, placement validity is determined by the **same passability
table** used for unit movement:
- `speed_table[Float + Water * 9]` = 1.0 → **passable** (ships and naval yards)
- `speed_table[Float + Clear * 9]` = 0.0 → **blocked** (can't place on land)

For normal buildings:
- `speed_table[Buildable + Clear * 9]` = 1 → **can build on land**
- `speed_table[Buildable + Water * 9]` = 0 → **can't build on water**

This elegant reuse means no special water placement code is needed — just the right
SpeedType value.

### Additional Rejection Conditions

- **Bridge cells** (`cell+0x140 & 0x100`) → rejected
- **Ramp cells** (`cell+0x140 & 0x400`) → rejected
- **Sub-cell occupied** (`cell+0x11C != 0`) → rejected
- **Existing building** at `cell+0xE4` → rejected (with pavement exceptions)

---

## 12. Naval Yard Properties

Three stock naval yards: GAYARD (Allied), NAYARD (Soviet), YAYARD (Yuri).

### INI Properties

| Property | Offset | Value | Purpose |
|---|---|---|---|
| `WaterBound=yes` | +0x67C | 5 (Float) | Placement requires water cells |
| `Naval=yes` | +0xCCE | 1 | Factory classified as naval |
| `Factory=UnitType` | +0xEB8 | 7 | Produces naval units |
| `Adjacent=12` | +0xEB4 | 12 | Max placement distance from ConYard |
| `NumberImpassableRows=3` | ~+0xEF8 | 3 | Blocked rows around dock exit |
| `NumberOfDocks=1` | +0xEFC area | 1 | Single dock slot |
| `WeaponsFactory=yes` | — | true | Shows unit production tab |
| `AmbientSound` | — | `_Amb_WavesLake` | Ambient water sound |

### NumberImpassableRows

Defines how many rows of cells around the dock entrance are marked impassable.
This prevents land units from walking through the water exit area where ships
spawn and depart. All three stock naval yards use a value of 3.

### Build Area (Adjacent)

Naval yards use `Adjacent=12` (vs typical buildings with `Adjacent=1-3`).
This allows placement far from the ConYard, since water may not be adjacent
to the player's base. The check is Manhattan distance, not path-based:
`|yard_x - conyard_x| + |yard_y - conyard_y| <= Adjacent`

### AI Placement

The AI uses `AINavalYardAdjacency=20` (from `[General]` in rulesmd.ini) for an
even more generous placement radius when the AI builds naval yards.
`BuildNavalYard=NAYARD,GAYARD,YAYARD` lists the buildable naval yard types.

### Ship Spawn Location

Naval yards use `TargetCoordOffset` (NOT ExitCoord) to define where produced ships
rally to after construction. Actual values from rulesmd.ini:
- GAYARD / YAYARD: `TargetCoordOffset=300,200,0`
- NAYARD: `TargetCoordOffset=256,256,0`

Ships are **Unlimboed at the building center** (not at an ExitCoord). After spawning,
they move toward the TargetCoordOffset rally point. This is distinct from how land
vehicle factories work (which use ExitCoord for the cell where the unit appears).

---

## 13. Ship Rendering

### Draw_Matrix (0x69F670, 1189 bytes)

Produces the 3x4 transformation matrix for voxel rendering. Shared with Drive.

**Ramp interpolation:**
- When terrain height changes, interpolates a smooth transition over `ramp_duration`
  ticks using ratio `(total - remaining) / total`
- Ratio defaults to 1.0 (from `_DAT_007e1718`) when no ramp is active

**Body tilt / rocking:**
- Reads AngleRotatedSideways from `techno+0x328` (float) and AngleRotatedForwards
  from `techno+0x32C` (float)
- If both near zero (below epsilon `_DAT_007e44e8`) → fast path (body-aligned turret)
- Otherwise: full sin/cos rotation matrix computation via `FUN_004cad00` (sin) /
  `FUN_004cacb0` (cos), composed with body matrix
- Per-frame angular velocities at `+0x330` (RockingSidewaysPerFrame) and `+0x334`
  (RockingForwardsPerFrame) drive the oscillation each tick

**Damage palette fade:**
- Applies palette fade based on health ratio via `FUN_00755a40(palette, ratio)`
- Full health → `FUN_007559b0()` (normal palette)

**Ship-specific visual effects:**
- Body rocking on water comes from the AngleRotatedSideways/Forwards floats on the
  techno object, updated each tick by the RockingPerFrame angular velocities
- Sinking uses WaterlineY (`+0x3CA`) as a screen-space Y clip line, controlled by
  the IsSinking flag (`+0x3CD`)

### Wake Animation

Spawned from Process() every 8th frame when on water (see section 5).
Uses the `Wake=WAKE1` type from `[General]` in rules.ini (stored at `RulesClass+0x94`).

### Underwater Rendering

Submarines use different visual states:
- `Visual_Character` returns different codes for surfaced vs submerged
- Drawing code changes opacity/palette for underwater units
- Dive/surface transitions use CloakState (see Section 14), not `Is_Surfacing`

---

## 14. Submarine & Underwater System

Units with `Underwater=yes` operate submerged by default.

### Relevant Units

| Unit | Underwater | Sensors | Cloakable |
|---|---|---|---|
| DLPH | Yes | Yes | Yes |
| SUB | Yes | Yes | No |
| BSUB | Yes | Yes | No |
| SQD | Yes | Yes | No |

**Note:** DEST also has `Sensors=yes` (surface ship with sub-detection ability).
All five units above plus DEST have Sensors.

### Mechanics (from rulesmd.ini)

- Underwater units are invisible to non-sensor units
- `Sensors=yes` allows detection of submerged units
- Submarines surface to fire weapons (if weapon requires surface)
- `CloakStop=no` on dolphins means they stay cloaked even when stationary

**CloakState system (TechnoClass+0x220):** Submarines use the standard CloakState
field — there is NOT a separate submarine diving/surfacing system. CloakState values:
0=Uncloaked/Surfaced, 1=Cloaking/Diving, 2=Cloaked/Submerged, 3=Uncloaking/Surfacing.
Transitions use `CloakingStages=9` steps at `CloakingSpeed=1` frame each = 9 frames total.

**`Is_Surfacing` (vtable 38 at `0x4B4C80`) is NOT about submarines** — it checks
transport cargo proximity (whether a passenger is near the surface for unloading).
Submarine dive/surface state is entirely handled through the CloakState field above.

### Infantry on Water (Amphibious)

Infantry with `MovementZone=Amphibious` use wet animation sequences when in water:

| Normal Sequence | Water Sequence | Index |
|---|---|---|
| Walk (3) | WetWalk | 0x11 |
| Stand (0/2) | WetStand | 0x10 |
| Idle1 (9) | WetIdle1 | 0x12 |
| Idle2 (10) | WetIdle2 | 0x13 |
| Die (0xB) | WetDie1 | 0x14 |
| Die (0xC) | WetDie2 | 0x15 |
| Fire (4/8) | WetFire | 0x16 |

Remapping triggered when cell land type == 2 or 6 (water/amphibious terrain).

---

## 15. Spawner System

Several naval units spawn sub-units during combat.

### Spawn Manager Pattern

- `SpawnCount=` defines max simultaneous spawns
- `SpawnDelay=` defines respawn interval (in frames)
- `Spawns=` names the spawned unit type
- Parent tracks spawned units, re-creates when destroyed
- Spawned units return to parent for rearming

### Key Projectile Types

- `NavalToGroundSeeker` — special projectile for naval-to-ground targeting (`AG=yes, AN=no`)
- `ASW` spawn by DEST — anti-submarine weapon
- `DMISL` / `CMISL` — missile spawns (DRED spawns DMISL, BSUB spawns CMISL)

### Ghidra Reference

SpawnManagerClass tracked in `DynamicVectorClass<class_SpawnManagerClass*>`.
Each spawn slot has a `SpawnControl` struct tracking state, rearm timer, and
link to the spawned unit entity.

---

## 16. Rust Engine Readiness

### Fully Implemented (no work needed)

| System | File | Notes |
|---|---|---|
| Ship locomotor CLSID | `rules/locomotor_type.rs:62` | `LocomotorKind::Ship` recognized |
| Water passability matrix | `sim/passability.rs:126-129` | Zone 10/11 correct |
| Water zone connectivity | `sim/zone_map.rs:35-48` | `ZoneCategory::Water` |
| Ground movement FSM | `sim/locomotor.rs` | 8-state machine (`GroundMovePhase`), ships use same |
| Drive track system | `sim/drive_track.rs` | 72 (Drive) / 67 (Ship) tracks, pre-computed curves |
| Building production | `sim/production_types.rs` | Factory types, queues |
| Building placement | `sim/production_placement.rs` | Foundation iteration, per-cell |
| Dock infrastructure | `sim/building_dock.rs` | FSM pattern established |
| Aircraft docks | `sim/aircraft_dock.rs` | Multi-slot model |
| Refinery docks | `sim/miner_dock.rs` | Single-slot + spawn pattern |
| Free unit spawning | `sim/production_refinery.rs:105` | Harvester spawn template |
| Cell occupancy | `sim/production_placement.rs:338` | Structure overlap detection |
| Foundation parsing | `rules/object_type.rs:153` | `Foundation=` from art.ini |
| Ship as ground mover | `sim/locomotor.rs:271-280` | `is_ground_mover()` includes Ship |

### Small Changes Needed

| Gap | Location | What to do |
|---|---|---|
| **WaterBound flag** | `rules/object_type.rs` | Add `water_bound: bool`, parse from INI |
| **Naval flag** | `rules/object_type.rs` | Add `naval: bool`, parse from INI |
| **Water cell placement** | `map/resolved_terrain.rs:901` | Currently `build_blocked = is_water`. Need WaterBound-aware check: water cells valid for WaterBound buildings, blocked for normal |
| **cell_placeable()** | `sim/production_placement.rs:321` | Add `water_bound` param; when true, require water; when false, reject water |
| **Build area on water** | `sim/production_placement.rs` | Adjacent check is Manhattan distance — may already work across water |

### Bugs to Fix

| Bug | Location | Details |
|---|---|---|
| **Float/Ship passability conflation** | `sim/passability.rs:143`, `sim/zone_map.rs:73`, `sim/terrain_cost.rs:215` | `SpeedType::Float` maps to zone 9 (passable on all terrain except rock). This is correct for **hover** units but wrong for **ships**. The zone flood-fill for `ZoneCategory::Water` uses `representative_speed_type() → Float → zone 9`, causing water zones to incorrectly include land cells. Ships could pathfind onto land. **Fix:** Zone building for Water should use `zone_layer_for_movement_zone(MovementZone::Water) = 10` (water only), not `zone_layer_for_speed_type(SpeedType::Float) = 9`. The terrain cost fallback in `classify_terrain_cost()` also treats Float same as Hover (land=COST_NORMAL), which needs a ship-specific branch. |

### New Implementation Needed

| System | Effort | Description |
|---|---|---|
| **Naval spawn cell** | Medium | Find water cell adjacent to dock for produced ships |
| **NumberImpassableRows** | Medium | Mark cells around dock exit as ground-impassable |
| **SpawnManager component** | Large | Carrier/Destroyer sub-unit spawning during combat |
| **Submarine state** | Large | Depth/surfacing, visibility to non-sensor units |
| **NavalTargeting** | Medium | Weapon selection based on naval priority values |
| **Wake animation** | Small | Spawn wake every 8th frame on water cells |
| **Ship body rocking** | Small | Pitch/yaw oscillation for visual effect |

---

## 17. Implementation Roadmap

### Phase 1: Naval Yard Placement (small)

1. Parse `WaterBound=yes` into `ObjectType.water_bound: bool`
2. Parse `Naval=yes` into `ObjectType.naval: bool`
3. Modify `cell_placeable()` to accept `water_bound` parameter:
   - When `water_bound=true`: require cell IS water, skip `build_blocked` check for water
   - When `water_bound=false`: reject water cells (current behavior)
4. Parse `NumberImpassableRows=` for naval yard foundation exit area
5. Parse `Adjacent=12` (already parsed)

### Phase 2: Ship Production (medium)

1. Implement `naval_spawn_cell()` — find valid water cell adjacent to naval yard dock
2. Extend production system: when a naval factory completes a unit, spawn on water cell
3. Follow the existing `maybe_spawn_refinery_harvester()` pattern

### Phase 3: Ship Movement (mostly works — bug fix needed)

Ships already work through the ground movement system with Ship locomotor +
Float SpeedType + Water MovementZone. The drive track system handles turning.

**Required fix:** The Float/Ship passability conflation bug (see "Bugs to Fix" above)
must be resolved before ships will be restricted to water. Currently the zone flood-fill
and terrain cost grid incorrectly allow Float units on land.

### Phase 4: Ship Combat (medium-large)

1. Implement `NavalTargeting` weapon selection
2. Implement SpawnManager for Carrier/Destroyer/Dreadnought sub-units
3. Implement submarine surfacing state for underwater units
4. Handle sensor-based detection of submerged units

### Phase 5: Ship Visuals (small)

1. Wake animation spawning (8th-frame check on water cells)
2. Ship body rocking (pitch/yaw modulation)
3. Submarine depth rendering (opacity/palette changes)

---

## 18. Key Addresses Reference

### ShipLocomotionClass

| What | Address |
|---|---|
| Constructor | `0x69EC50` |
| Destructor | `0x69ECF0` |
| ILocomotion vtable | `0x7F2D8C` |
| Process (main tick) | `0x69FC10` |
| Move_To (set destination) | `0x69F450` |
| Stop_Moving | `0x69F510` |
| Do_Turn | `0x6A05C0` |
| Draw_Matrix (rendering) | `0x69F670` |
| Force_Track | `0x6A0310` |
| Unlimbo | `0x69FBE0` |
| Force_New_Slope | `0x69F250` |
| Is_Moving | `0x69F290` |
| Is_Surfacing | `0x4B4C80` |

### DriveLocomotionClass (for comparison)

| What | Address |
|---|---|
| ILocomotion vtable | `0x7E7EB0` |
| Process (main tick) | `0x4B0500` |

### Shared Movement Subroutines (called by both Ship and Drive Process)

| What | Address | Size |
|---|---|---|
| Main movement AI | `FUN_006a1c80` | 8470 bytes |
| Drive track execution | `FUN_006a05f0` | 5737 bytes |
| Drive track step apply | `FUN_006a01a0` | 366 bytes |
| Track coord transform | `FUN_006a3db0` | — |

### Building Placement

| What | Address |
|---|---|
| IsCellSuitableForBuilding | `FUN_0047c620` |
| CanBePlacedAt (foundation) | `FUN_0045ee70` |
| BuildingTypeClass INI reader (WaterBound) | `0x45FF94` area |
| IsFoundationPassable | `FUN_00586780` |

### Data Tables

| What | Address | Notes |
|---|---|---|
| Passability matrix (hardcoded) | `0x82A594` | 12×8 int32, 1=pass 2=block |
| Speed/terrain table (runtime) | `0x89EA40` | 12×9 float, from rules.ini |
| Drive track descriptors | `0x7F2A40` | 72×12 bytes |
| Drive track step arrays | `0x7F2960` | 16×16 bytes (ptrs to steps) |
| Direction offset table | `0x89F688` | 8×4 bytes (cell dx/dy) |
| Sub-cell offset table | `0x89F6D8/DC` | 8×8 bytes |
| Null-coord sentinel | `0xB077F8/FC/00` | 3 globals |
| Bridge Z-offset | `0xB0782C` | int |
| Cell height step | `0xB07838` | int |
| Rules global pointer | `0x8871E0` | RulesClass* |
| Frame counter | `0xA8ED84` | Current game tick |

### TechnoClass / CellClass Field Quick Reference

**TechnoClass (unit):**
- `+0x9C..A4` — Location XYZ (3 ints)
- `+0x15E` — Current speed (double)
- `+0x328` — AngleRotatedSideways (float) — current sideways tilt
- `+0x32C` — AngleRotatedForwards (float) — current forward tilt
- `+0x330` — RockingSidewaysPerFrame (float) — sideways angular velocity
- `+0x334` — RockingForwardsPerFrame (float) — forward angular velocity
- `+0x3CA` — WaterlineY (short) — screen Y clip for sinking
- `+0x3CD` — IsSinking (byte) — master sinking flag
- `+0x3CE` — IsSinking_prev (byte) — edge detection for sound trigger
- `+0x578` — MaxSpeed
- `+0x5E0..63C` — Movement path queue (23 direction entries)
- `+0x640..648` — Movement delay timer
- `+0x68A` — IsMoving flag
- `+0x68B` — OnBridge transition flag
- `+0x8C` — OnBridge state

**TechnoTypeClass (type data):**
- `+0x67C` — SpeedType (5=Float for WaterBound buildings)
- `+0xCCE` — Naval flag
- `+0xC94` — (type capability flag, checked during movement)

**CellClass:**
- `+0x38` — Tile set index
- `+0x44` — Overlay type index (-1 = none)
- `+0x4C` — Terrain type (for metadata)
- `+0xDC` — Speed type bitmask
- `+0xE4` — First occupant object pointer
- `+0xE8` — Alt occupant pointer
- `+0xEC` — **LandType enum** (key for passability lookups). gamemd.exe ordering: Clear=0, Road=1, Water=2, Rock=3, Wall=4, Tiberium=5, Beach=6, Rough=7, Ice=8, Railroad=9, Tunnel=10, Weeds=11. **Note:** Rust engine uses a remapped 8-column ordering (Water=4, not 2) — see Bugs to Fix section.
- `+0x11B` — Cell height level (signed byte)
- `+0x11C` — Sub-cell flag
- `+0x11E` — (cell properties byte)
- `+0x124` — Sub-cell occupation bits (& 0x3F)
- `+0x140` — **Cell flags** (bit 0x100 = bridge, 0x400 = ramp, 0x800 = bridge dir)
