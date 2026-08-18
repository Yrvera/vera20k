# MapClass__Is_Cell_In_Playfield — decode

## Summary

`MapClass__Is_Cell_In_Playfield` (`0x00578460`) performs a bounds check that determines
whether a given `CellStruct {X, Y}` falls within the active playfield of the isometric
map. It uses **diamond-coordinate arithmetic** (`X+Y` and `X-Y`) consistent with the
isometric grid, checking four inequality conditions derived from several `MapClass`
dimension fields. An optional third parameter (`param3`) enables a height-adjusted edge
correction that looks up a per-cell height value from the cell array.

The function is the gate for all cell-validity checks in the engine: pathfinding,
AI movement, crate spawning, tiberium growth, display input, and more. 72 direct callers
confirmed (41 named + 31 unnamed FUN_*) via live `get_function_callers 0x00578460`
re-verification.

**Verified via `decompile_function 0x00578460`, `disassemble_function 0x00578460`,
`get_function_callers 0x00578460`, `get_function_callees 0x00578460`.**

## Active in YR

YES. 72 direct callers span all major engine subsystems — see full table under "## Callers"
below. Named callers include AStar pathfinding, all locomotors (Drive/Ship/Fly/Walk/Teleport),
aircraft AI, foot-class actions (click, scan, harvest, scatter), display/cursor, HouseClass
AI scans, infantry/unit scatter, zone-map flood fill, radar bounds, technocl
unlimbo, and convoy scripts. All are live in a standard YR skirmish. No TS-only gate detected.

## Address

`0x00578460` in `gamemd.exe`

## Signature (actual)

```c
// __thiscall: ECX = MapClass* (this)
// 2 explicit stdcall args: cellstruct_ptr, param3
// Returns: uint — low byte = 0 (out of playfield) or 1 (in playfield); high bytes from map field
// RET 0x8 = pops 2 × 4-byte args
uint __thiscall MapClass__Is_Cell_In_Playfield(int param_1, short *param_2, char param_3);
```

`param_2` is `short *` — a pointer to `CellStruct {short X, short Y}` where
`param_2[0]` = X and `param_2[1]` = Y (same layout as `MapCoord_Set`/`MapCoord_Add`).
`param_3` is a `char` boolean: `'\0'` = basic check, `!= '\0'` = height-adjusted edges.
`RET 0x8` = pops 2 × 4 bytes (two stdcall args). Verified via `disassemble_function 0x00578460`.

## Parameters

| Name | Type | Location | Meaning |
|------|------|----------|---------|
| `this` | `int` | ECX | MapClass pointer (thiscall) |
| `cellstruct_ptr` | `short *` | `[ESP+4]` | CellStruct: X at [0], Y at [1] (signed shorts) |
| `param3` | `char` | `[ESP+8]` | 0 = basic bounds; non-zero = height-adjusted edges enabled |

## Return Value

`uint` — observed behavior: `1` (low byte = 0x01) if in playfield, `0` (low byte = 0x00)
if out of playfield. The high bytes carry the MapClass+0xf4 field value as a side effect
of the decompiler's return-register tracking — callers observe only AL (the `char`).
Callers from `fn-is-coords-in-playfield.md` confirmed to test only AL as `char`.
Verified via `disassemble_function 0x00578460` — `MOV AL, 0x1` or `XOR AL, AL` before RET.

## Control Flow

Cyclomatic complexity 10, 14 basic blocks, 77 instructions. Single-pass bounds check
with an optional height-correction path.

**Phase 1 — Load and convert cell coordinates:**
```
MOVSX EDX, word[EBX+2]   ; EDX = cell_Y (sign-extend from short)
MOVSX EDI, word[EBX]     ; EDI = cell_X (sign-extend from short)
TEST AL, AL               ; is param3 == 0?
JZ   skip_height_adjust   ; if so, skip phase 2
```

**Phase 2 (param3 != 0) — Height-adjusted edge:**
```
; Compute flat cell array index: Y * 0x200 + X
; Index range check: [0, 0x3FFFF]
; If out of range or cell pointer null: use fallback cell at DAT_00abdc50
; Read [cell+0x11b] (signed byte) → height correction value (iVar4)
; Read [cell+0x11c] (byte flag) → if flag && (X+Y) < edge_threshold: iVar4++
```
The height correction `iVar4` shifts the diamond boundary for elevated terrain.

**Phase 3 — Diamond bounds check (4 conditions, all must be true):**

In isometric space, the playfield is a diamond. The check uses `S = X + Y` (sum)
and `D = X - Y` (difference):

```
Condition 1: (MapClass+0xf4 + MapClass+0x100*2 + iVar4) < S
Condition 2: S <= (MapClass+0xf4 + 2 + (MapClass+0x108 + MapClass+0x100)*2 + iVar4)
Condition 3: D < (MapClass+0x104 + MapClass+0xfc)*2 - MapClass+0xf4
Condition 4: MapClass+0xf4 + MapClass+0xfc*(-2) > D   [i.e., D < f4 - 2*fc]
```

If all 4 hold → `MOV AL, 1; RET`. Else → `XOR AL, AL; RET`.

Verified via `disassemble_function 0x00578460` (full 77-instruction listing).

## MapClass Fields Referenced

| Offset | Access pattern | Role in bounds check |
|---|---|---|
| `+0xf4` | Read (uint) | Left/edge margin — base term in diamond inequalities |
| `+0xfc` | Read (int) | East-west margin — constrains X-Y range |
| `+0x100` | Read (int) | Width-related span — contributes to sum and difference bounds |
| `+0x104` | Read (int) | East boundary component — constrains X-Y upper bound |
| `+0x108` | Read (int) | South boundary component — constrains X+Y upper bound |

These fields encode the playfield diamond dimensions in isometric coordinate space.
`+0xf4` is also used by `MapClass__CellCoordToLinearIndex` (task #4) as a row-stride
component. The exact semantic labels (e.g., "LocalSize" vs "MapSize" per RA2 modding
terminology) are not fully decoded — verified via disassembly but semantic labeling
requires tracing the map load path (out of scope).

## Cell Array Reference (param3 path)

When `param3 != 0`:
- `g_CellArray_Base @ 0x0087f924` — pointer to base of the cell pointer array
- Cell array index: `Y * 0x200 + X` — the cell array has 0x200 (512) cells per row
- Max valid index: `0x3FFFF` = 524287 cells total (512 × 1024 maximum map size)
- Fallback cell: `DAT_00abdc50` — used when index is out of bounds or cell pointer is null;
  `DAT_00abdc74` receives the invalid cell coords for diagnostics
- `[cell + 0x11b]` (signed byte) — height correction for this cell
- `[cell + 0x11c]` (byte) — flag enabling the +1 edge extension

Verified via `disassemble_function 0x00578460`:
`MOV ESI, dword ptr [0x0087f924]` at `0x00578489`;
`MOV EAX, dword ptr [ESI + EAX*0x4]` at `0x0057848f`.

## Callers

72 direct callers via live `get_function_callers 0x00578460` (re-verified at team-lead
audit, replacing an earlier undercount of 30). 41 named + 31 unnamed FUN_*.

### Named callers (41)

| Caller | Address | Subsystem |
|---|---|---|
| `AStar_pathfind_search` | `0x0042c900` | Pathfinding cell validity |
| `AircraftClass__AI` | `0x00414bb0` | Aircraft pathing |
| `AircraftClass__Find_Approach_Cell` | `0x004197c0` | Aircraft landing approach |
| `AircraftClass__Find_Attack_Cell` | `0x00418e20` | Aircraft attack target |
| `AircraftClass__Is_Cell_Free_For_Landing` | `0x00419b00` | Aircraft landing zone |
| `AnimClass__FindAttachTarget` | `0x00425d10` | Anim attachment target |
| `CellClass__CanPlaceTiberium` | `0x004838e0` | Tiberium growth gating |
| `CellClass__RecalcZoneType` | `0x00483c80` | Zone type recalc |
| `CrateSlot__ValidateCellAndCreateOverlay` | `0x004a18f0` | Crate spawn validity |
| `DisplayClass__DetermineAction` | `0x00692610` | Cursor action |
| `DisplayClass__SetCursorFromAction` | `0x004aae90` | Cursor display |
| `DriveLocomotionClass__Process_Movement` | `0x004b2630` | Drive movement validity |
| `FlyLocomotionClass__Emergency_Relocate` | `0x004ccfd0` | Aircraft emergency reposition |
| `FootClass__ClickedAction_Cell` | `0x004d7d50` | Player click on cell |
| `FootClass__ClickedAction_Object` | `0x004d74e0` | Player click on object |
| `FootClass__Greatest_Threat_Scan` | `0x004d5690` | AI threat scan |
| `FootClass__Is_Cell_Harvestable` | `0x004dce80` | Harvester target validity |
| `FootClass__Is_Cell_Weedable` | `0x004dd9f0` | Weed-eater target validity |
| `FootClass__PerCellProcess` | `0x004d85d0` | Per-cell tick processing |
| `FootClass__What_Action_OnCell` | `0x004ddde0` | Action lookup for cell |
| `GenerateTerrainPreview` | `0x00641140` | Terrain preview generation |
| `HouseClass__AI_FindAirTarget` | `0x0050a150` | AI air-target selection |
| `HouseClass__AI_FindBestRallyTarget` | `0x0050cbf0` | AI rally target |
| `HouseClass__AI_FindInfantryTarget` | `0x00509f60` | AI infantry target |
| `HouseClass__AI_ScanBasePerimeter` | `0x005082c0` | AI base perimeter scan |
| `InfantryClass__Scatter` | `0x0051d0d0` | Infantry scatter |
| `MapClass__Can_Reach_Zone` | `0x0056d100` | Zone reachability |
| `MapClass__CellIterator_NextExpanding` | `0x00578710` | Cell iterator |
| `MapClass__IsCoordsInPlayfield` | `0x005785f0` | Lepton→cell wrapper (param3=1) |
| `MapClass__IsRectInPlayfield` | `0x00578390` | Rect bounds (corner check) |
| `MapClass__RecalcCellsAndRebuildZones` | `0x00586990` | Zone rebuild |
| `PathfinderClass__UpdateBridgePassability` | `0x0042acf0` | Bridge passability |
| `RadarClass__ClearBackground` | `0x00655250` | Radar bg clear |
| `RadarClass__ComputeRadarMapBounds` | `0x00654490` | Radar bounds calc |
| `ShipLocomotionClass__Process_Movement` | `0x006a1c80` | Naval movement |
| `TeamClass__Convoy_Script_Move_To_Cell` | `0x006ec7d0` | AI convoy move |
| `TechnoClass__Unlimbo` | `0x006f6ca0` | Object placement |
| `TeleportLocomotionClass__StateMachineTick` | `0x007192f0` | Chrono unit movement |
| `UnitClass__Scatter` | `0x00743a50` | Vehicle scatter |
| `ZoneMap__FloodFillReachableZones` | `0x005840c0` | Zone flood fill |
| `ZoneMap__FloodFillScanline` | `0x005824a0` | Zone scanline |

### Unnamed callers (31 FUN_*)

`FUN_00457020`, `FUN_004aa440`, `FUN_004de1d0`, `FUN_00501ac0`, `FUN_005164d0`,
`FUN_005221d0`, `FUN_00567230`, `FUN_00580bc0`, `FUN_00581140`, `FUN_005835d0`,
`FUN_00583820`, `FUN_00584550`, `FUN_00586e50`, `FUN_00586fc0`, `FUN_0058b820`,
`FUN_0058f2c0`, `FUN_005905d0`, `FUN_00594010`, `FUN_005a28c0`, `FUN_005a6920`,
`FUN_005a8720`, `FUN_00640a40`, `FUN_0064cda0`, `FUN_0065d8e0`, `FUN_00688ed0`,
`FUN_0068ad70`, `FUN_006ec300`, `FUN_006f5090`, `FUN_00700600`, `FUN_00746000`,
`FUN_0074d7c0` — bridge, zone, unit placement, pathfinding helpers.

Also called by `MapClass__IsCoordsInPlayfield @ 0x005785f0` (task #3) which passes
`param3=1` (height-adjusted edges enabled). Verified via `fn-is-coords-in-playfield.md`.

## Callees

None. Leaf function. Verified via `get_function_callees 0x00578460`.

## Globals

| Address | Access | Semantics |
|---|---|---|
| `g_CellArray_Base @ 0x0087f924` | READ (param3 path only) | Base pointer to cell object pointer array |
| `DAT_00abdc50` | READ (fallback cell ptr) | Fallback CellClass for out-of-bounds index |
| `DAT_00abdc74` | WRITE (diagnostics) | Stores invalid cell coords when fallback is used |

## INI Keys

None directly. The MapClass dimension fields (`+0xf4` etc.) are set during map load
from the `[Map]` section (out of scope for this task).

## Enums

None. `param3` is a raw boolean char (0 / non-zero).

## Load-Bearing vs Internal

**Load-bearing:**
- The diamond-coordinate formula (`X+Y`, `X-Y`) is load-bearing — a Cartesian check
  would accept wrong cells at the isometric corners.
- The `param3=1` path (height-adjusted edges) is required for correct behavior when
  called from `MapClass__IsCoordsInPlayfield`. Using `param3=0` gives a tighter
  boundary — wrong for unit unlimbo, aircraft landing, etc.
- Cell index formula `Y * 0x200 + X` — if the map uses a different stride this would
  break the height lookup.
- `g_CellArray_Base` pointer at `0x0087f924` is load-bearing for the param3 path.

**Internal:**
- The fallback cell mechanism (`DAT_00abdc50`) is a defensive artefact.
- The return value format (high bytes carry map field data as Ghidra artefact).

## Out-of-Scope Refs

- `MapClass__IsCoordsInPlayfield @ 0x005785f0` — thin wrapper that converts leptons
  to cells and calls here with `param3=1`; task #3 (completed).
- `MapClass__CellCoordToLinearIndex @ 0x0056d430` — also uses `+0xf4`/`+0xf8`; task #4 (completed).
- `MapClass+0x6c` — total zone count (bounds clamp); different subsystem.
- `g_CellArray_Base`, `CellClass` layout — cell array indexing system; out of scope.
- `DAT_00abdc50` — fallback CellClass object; out of scope.

## SELF-PROOF — 3 random load-bearing claims verified

1. **`RET 0x8`** (2 args × 4 bytes): `disassemble_function 0x00578460` shows both
   return paths end with `RET 0x8` (at `0x0057852b` and `0x00578534`). Verified.
2. **`g_CellArray_Base @ 0x0087f924`**: disassembly at `0x00578489`:
   `MOV ESI, dword ptr [0x0087f924]`. Verified.
3. **`MOV AL, 0x1` for in-playfield path**: disassembly at `0x00578528`:
   `MOV AL, 0x1` then `POP EBX; RET 0x8`. Verified.

## Unverified

- The exact semantic labels for `MapClass+0xf4`, `+0xf8`, `+0xfc`, `+0x100`, `+0x104`,
  `+0x108`. The role as diamond-coordinate boundary terms is verified from disassembly;
  the RA2 modding names (LocalSize, MapSize, etc.) would require tracing the map-load
  path. Marked YELLOW for semantic naming only — the formula is verified.
- `[cell+0x11b]` and `[cell+0x11c]` field semantics within CellClass. Role as height
  correction and height-edge flag is inferred from usage; CellClass layout decode is out of scope.
