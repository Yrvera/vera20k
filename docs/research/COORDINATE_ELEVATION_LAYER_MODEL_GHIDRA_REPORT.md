# Coordinate / Elevation / Layer Model - Ghidra Research Report

**Address(es):** `0x006D1F10`, `0x006D20E0`, `0x00480A30`, `0x0047E8A0`, `0x0047EA90`, `0x00487D50`, `0x00429A90`, `0x0042C900`, `0x0073F0A0`  
**Confidence:** High  
**Active in YR:** Yes

## 1. Overview

gamemd.exe uses a **layered 2.5D isometric coordinate model**. The foundation is a 2D cell grid `(cell_x, cell_y)`, with sub-cell precision in **leptons** (`256` leptons per cell) and a separate `Z` component for elevation/altitude. Rendering projects 3D lepton coordinates `(X, Y, Z)` into 2D tactical pixels; pathfinding and occupancy use the same cell grid but split some behavior by height/layer, especially bridges.

This is not a free voxel/3D map. It is a 2D isometric cell plane plus:

- lepton sub-cell precision,
- terrain/object height,
- bridge/ground logical object lists,
- pathfinding state that distinguishes ground-level and bridge-level traversal.

## 2. Key Binary Findings

### 2.1 Tactical projection is 3D lepton coords -> 2D screen pixels

**Function:** `CoordsToClient` at `0x006D1F10`  
**Active in YR:** Yes - central tactical rendering transform.

Decompiled core:

```c
x_term = (X * 0x3c) / 2 + (Y * -0x3c) / 2;
y_term = (X * 0x1e) / 2 + (Y *  0x1e) / 2;
screen_x = (x_term + signed_bias) >> 8;
screen_y = ((y_term + signed_bias) >> 8) - AdjustForZ(Z);
```

Constants:

| Constant | Meaning |
|----------|---------|
| `0x3C` / 60 | full isometric tile width in pixels |
| `0x1E` / 30 | full isometric tile height in pixels |
| `>> 8` | divide lepton-space by 256 leptons per cell |
| `signed_bias = value >> 31 & 0xFF` | signed divide-by-256 floor/adjustment behavior before shift |

For cell-origin leptons `(rx * 256, ry * 256, 0)`:

```text
screen_x = 30 * (rx - ry)
screen_y = 15 * (rx + ry)
```

For cell-center leptons `(rx * 256 + 128, ry * 256 + 128, 0)`:

```text
screen_x = 30 * (rx - ry)
screen_y = 15 * (rx + ry) + 15
```

**Conclusion:** The binary's render-space projection matches an isometric 2D plane with a separate vertical `Z` lift applied to screen Y.

### 2.2 Z is separate elevation, not map Y

**Function:** `Tactical__AdjustForZ` at `0x006D20E0`  
**Active in YR:** Yes - 90 xrefs found from terrain, objects, particles, lines, beams, shroud, and tactical rendering.

The decompiler gives a compact wrapper, but disassembly shows:

```asm
CMP  param_1, 0x2D8   ; (corrected 2026-05-28: was 0x2D7; binary at 006d20e3 shows CMP ECX,0x2D8 — OPERATOR_OR_ORDER_DRIFT)
JL   skip_add1        ; jump if ECX < 0x2D8, i.e. +1 fires when Z >= 0x2D8 (== Z > 0x2D7 for integers)
FILD param_1
FMUL DAT_00B0CD48
FIADD (Z >= 0x2D8 ? 1 : 0)
FADD 0.5
CALL Math__ftol
```

Previously verified multiplier:

```text
screen_z = ftol(Z_leptons * DAT_00B0CD48 + (Z >= 0x2D8 ? 1 : 0) + 0.5)
```
(corrected 2026-05-28: was `Z >= 728`; 728 decimal = 0x2D8, so the value is equivalent; but the threshold constant in binary is 0x2D8 / `CMP ECX,0x2D8 + JL`, not CMP against 0x2D7 — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x006D20E0`)

`CoordsToClient` inlines this same Z-lift computation directly (not a call to `Tactical__AdjustForZ`) and then subtracts the result from projected screen Y. (corrected 2026-05-28: was "CoordsToClient subtracts this value from projected screen Y" with implied call; binary at 006d1faf shows `FMUL [0x00b0cd48]` + `FIADD` + `FADD 0.5` + `CALL ftol` inlined in CoordsToClient; no call to 0x006D20E0 — INFERENCE_HARDENED; verified via `disassemble_function 0x006D1F10`)

**Conclusion:** In gamemd, height is a third coordinate component for projection and physics/range, but visually it becomes an upward screen-Y offset. It is not a second map Y axis.

### 2.3 Cell center coords are cell grid + 128 lepton sub-cell center + ground height

**Function:** `CellClass__Get_Center_Coords` at `0x00480A30`  
**Active in YR:** Yes - CellClass coordinate query.

Decompiled behavior:

```c
coord.x = cell.MapCoord_X * 0x100 + 0x80;
coord.y = cell.MapCoord_Y * 0x100 + 0x80;
coord.z = ground_height_lookup({0x80, 0x80});
```

Constants:

| Constant | Meaning |
|----------|---------|
| `0x100` / 256 | leptons per cell |
| `0x80` / 128 | cell-center sub-cell offset |

**Conclusion:** gamemd stores/uses cell-space plus lepton offsets; the cell center is exactly `(x*256+128, y*256+128)`.

### 2.4 Object coordinates are 3-int lepton coords

**Function:** `ObjectClass__GetCoords` at `0x005F65A0` (corrected 2026-07-12: address was omitted from this doc; verified via `search_functions_enhanced` + `decompile_function 0x005F65A0` — STALE/incomplete citation)  
**Active in YR:** Yes - base object coordinate accessor.

Decompiled behavior:

```c
out.x = this + 0x9C;
out.y = this + 0xA0;
out.z = this + 0xA4;
```

**Offsets:**

| Offset | Purpose |
|--------|---------|
| `ObjectClass+0x9C` | X lepton coordinate |
| `ObjectClass+0xA0` | Y lepton coordinate |
| `ObjectClass+0xA4` | Z lepton coordinate |

**Conclusion:** moving/game objects carry a real 3-component coordinate, but the X/Y components still map back to cells by dividing by 256.

### 2.5 CellClass is a 2D grid, indexed `Y * 512 + X`

**Function:** `AStar_main_loop` at `0x00429A90`  
**Active in YR:** Yes - core A* loop.

Decompiled cell lookup:

```c
dest_cell = *(CellClass**)(g_CellArray_Base + (dest_y * 0x200 + dest_x) * 4);
src_cell  = *(CellClass**)(g_CellArray_Base + (src_y  * 0x200 + src_x ) * 4);
```

Constants:

| Constant | Meaning |
|----------|---------|
| `0x200` / 512 | fixed cell-array row stride |
| `* 4` | CellClass pointer table entries |

**Conclusion:** the map's core storage is a 2D CellClass pointer grid, not a 3D cell volume.

### 2.6 CellClass has separate ground and bridge object lists

**Functions:** `CellClass__AddContent` at `0x0047E8A0`, `CellClass__RemoveContent` at `0x0047EA90`  
**Active in YR:** Yes - object occupancy maintenance.

`CellClass__AddContent` selects the object list using a stack boolean:

```c
if (bridge_layer_arg == 0) {
    list_head = this->FirstObject; // CellClass+0xE4
} else {
    list_head = this->AltObject;   // CellClass+0xE8
}
```

It then inserts the object into either `FirstObject` or `AltObject`. `RemoveContent` uses the same selector to unlink from the matching list.

Offsets:

| Offset | Purpose |
|--------|---------|
| `CellClass+0xE4` | `FirstObject` - ground object list |
| `CellClass+0xE8` | `AltObject` - alternate/bridge object list |
| `ObjectClass+0x30` | linked-list next pointer used by cell content lists |

**Important detail:** buildings (`WhatAmI == 6`) are appended at the tail when the selected list is non-empty; most other objects are prepended.

**Conclusion:** gamemd has one cell coordinate, but two logical object-occupancy lists for ground vs bridge. Our term "layered" is correct for bridge occupancy.

### 2.7 Effective cell height adds +4 for bridge-overlay/elevated state

**Function:** `CellClass__GetEffectiveHeight` at `0x00487D50`  
**Active in YR:** Yes - bridge/elevation helper.

Decompiled behavior:

```c
return (signed char)cell.Level + ((cell.Flags >> 7) & 1) * 4;
```

Offsets and flags:

| Field | Meaning |
|-------|---------|
| `CellClass+0x11B` | terrain level |
| `CellClass+0x140 bit 7 / 0x80` | bridge/elevated overlay height contribution |
| `+4` | bridge effective-height offset in height levels |

**Conclusion:** bridge/elevated height is modeled as a height adjustment on a cell, not as another coordinate axis.

### 2.8 A* pathfinding stores current path height and has separate ground/bridge closed sets

**Functions:** `AStar_pathfind_search` at `0x0042C900`, `AStar_main_loop` at `0x00429A90`  
**Active in YR:** Yes - standard ground pathfinding.

Relevant A* setup:

```c
dest_height = dest_cell.Level;
if (unit is not aircraft && dest_cell.Flags & 0x100) {
    dest_height += 4;
}
this+0x34 = dest_height;

start_height = source_cell.Level;
if (unit is not aircraft && unit.is_on_bridge) {
    start_height += 4;
}
this+0x30 = start_height;
```

Closed/g-cost arrays:

| Pathfinder offset | Purpose |
|-------------------|---------|
| `+0x18` | ground closed/stamp array (corrected 2026-07-12: was labeled "bridge" — swapped with +0x1C. Binary at `00429c3e`/`00429c40` is `CMP start_height(+0x30),srcCell.Level; JLE` — the ELSE/fallthrough path (start_height <= Level, i.e. plain ground height) writes to +0x18/+0x24 at `00429c57`; the neighbor-relax loop's ground-compatible flag (set when neighbor is non-bridge OR height-diff<2) also selects +0x18 at `00429ecf` — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) |
| `+0x1C` | bridge/elevated closed/stamp array (corrected 2026-07-12: was labeled "ground" — swapped with +0x18. Binary at `00429c3e` shows the taken (non-JLE) path — start_height > Level, i.e. Level+4 elevated/on-bridge case — writes to +0x1C/+0x20 at `00429c42`; the neighbor-relax loop's "genuinely different layer" flag (bridge neighbor AND height-diff>=2) selects +0x1C at `00429f04` — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) |
| `+0x20` | bridge/elevated g-cost array (corrected 2026-07-12: was labeled "ground" — swapped with +0x24; paired with the +0x1C stamp array at `00429c4b`/`00429f1b` — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) |
| `+0x24` | ground g-cost array (corrected 2026-07-12: was labeled "bridge" — swapped with +0x20; paired with the +0x18 stamp array at `00429c60`/`00429ee6` — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) |
| `+0x28` | current stamp |
| `+0x30` | current path height |
| `+0x34` | destination path height |

Neighbor logic:

- Gets neighbor cells from 8-direction pointer offsets or direction `8` bridge/tube crossing.
- If neighbor has bridge structural flag `0x100`, compares current path height to neighbor `Level`.
- If height difference is `< 2`, the neighbor can be considered on the ground-compatible side.
- Otherwise it uses the bridge-level closed/g-cost arrays.
- `Can_Enter_Cell` receives the current path height and a bridge/list-selection flag.

**Conclusion:** gamemd pathfinding is not plain 2D. It is cell-grid A* with path height and distinct ground/bridge state. This matches the "layered 2.5D" description.

### 2.9 Unit cell-entry uses bridge height to select the object list

**Function:** `UnitClass__Can_Enter_Cell` at `0x0073F0A0`  
**Active in YR:** Yes - UnitClass passability vtable method.

At entry, if the target cell has bridge structural flag `0x100` and the incoming path height differs from the cell's ground `Level` by at least 2, the function sets an internal bridge-layer flag.

Later it selects which object list to scan:

```c
if (bridge_layer_flag == 0) {
    obj = cell.FirstObject; // +0xE4
} else {
    obj = cell.AltObject;   // +0xE8
}
```

It then iterates the selected list only. Ground units under a bridge and bridge-level units on the bridge are therefore separated by list selection.

**Conclusion:** bridge/ground layering is behaviorally active in normal YR passability, not only rendering.

## 3. Class Layout / Key Offsets

### ObjectClass

| Offset | Type | Purpose | Evidence |
|--------|------|---------|----------|
| `+0x9C` | int | X lepton coord | `ObjectClass__GetCoords` |
| `+0xA0` | int | Y lepton coord | `ObjectClass__GetCoords` |
| `+0xA4` | int | Z lepton coord | `ObjectClass__GetCoords` |
| `+0x30` | ptr | next object in CellClass list | `CellClass__AddContent`, `CellClass__RemoveContent` |

### CellClass

| Offset | Type | Purpose | Evidence |
|--------|------|---------|----------|
| `+0x24` | short | map cell X | `CellClass__Get_Center_Coords`, A* |
| `+0x26` | short | map cell Y | `CellClass__Get_Center_Coords`, A* |
| `+0xE4` | ptr | ground object list (`FirstObject`) | `CellClass__AddContent`, `UnitClass__Can_Enter_Cell` |
| `+0xE8` | ptr | bridge/alt object list (`AltObject`) | `CellClass__AddContent`, `UnitClass__Can_Enter_Cell` |
| `+0x11B` | signed byte | terrain level | `CellClass__GetEffectiveHeight`, A* |
| `+0x116` | short | bridge/tube index | A* direction 8 handling |
| `+0x140` | uint | cell flags | `CellClass__GetEffectiveHeight`, A*, `UnitClass__Can_Enter_Cell` |
| `+0x140 & 0x80` | flag | effective-height +4 | `CellClass__GetEffectiveHeight` |
| `+0x140 & 0x100` | flag | bridge structural/path cell | A*, `UnitClass__Can_Enter_Cell` |

### PathfinderClass

| Offset | Type | Purpose | Evidence |
|--------|------|---------|----------|
| `+0x18` | int* | ground closed/stamp array (corrected 2026-07-12: was "bridge" — swapped with +0x1C; JLE branch `00429c3e`/`00429c40`→`00429c57` and neighbor flag branch `00429ecf` — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) | `AStar_main_loop` |
| `+0x1C` | int* | bridge/elevated closed/stamp array (corrected 2026-07-12: was "ground" — swapped with +0x18; JLE fallthrough `00429c3e`→`00429c42` and neighbor flag branch `00429f04` — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) | `AStar_main_loop` |
| `+0x20` | float* | bridge/elevated g-cost array (corrected 2026-07-12: was "ground" — swapped with +0x24, paired with +0x1C stamp array — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) | `AStar_main_loop` |
| `+0x24` | float* | ground g-cost array (corrected 2026-07-12: was "bridge" — swapped with +0x20, paired with +0x18 stamp array — OPERATOR_OR_ORDER_DRIFT; verified via `disassemble_function 0x00429A90`) | `AStar_main_loop` |
| `+0x28` | int | current stamp value | `AStar_main_loop` |
| `+0x30` | int | current path height | `AStar_main_loop` |
| `+0x34` | int | destination path height | `AStar_main_loop` |
| `+0x3C` | int | bridge/hierarchical path mode flag | `AStar_pathfind_search`, `AStar_main_loop` |

## 4. Core Model

### 4.1 Coordinate spaces in gamemd

| Space | Shape | Unit | Notes |
|-------|-------|------|-------|
| Cell coords | `(cell_x, cell_y)` | cells | stored in CellClass and packed cell structs |
| Lepton coords | `(X, Y, Z)` | leptons | object/world precision; `256` leptons per cell |
| Height levels | `Level`, effective height | small signed levels | terrain/bridge path height; level * roughly 15 screen px |
| Tactical screen coords | `(screen_x, screen_y)` | pixels | produced by `CoordsToClient` |
| Occupancy layer | ground vs bridge list | logical | selected by height/bridge state, not a new coordinate grid |

### 4.2 Recommended terminology

The most accurate short name is:

```text
layered 2.5D isometric cell-space
```

Expanded:

```text
2D isometric CellClass grid + 3D lepton object coordinates + terrain/bridge elevation + logical ground/bridge occupancy layers.
```

### 4.3 What "layered" means and does not mean

It means:

- same `(cell_x, cell_y)` can have ground occupants and bridge occupants,
- passability chooses `FirstObject` or `AltObject`,
- A* tracks ground-level and bridge-level closed/cost state separately,
- bridge surface height is typically `Level + 4`.

It does not mean:

- there are two independent coordinate systems,
- the map is a 3D voxel volume,
- every arbitrary Z can have its own occupancy layer.

## 5. INI Keys / Data Inputs

This foundational model is mostly binary/data-structure driven rather than controlled by one INI switch. Relevant data inputs:

| Data | Source | Effect |
|------|--------|--------|
| TMP tile height/level | map TMP/tile data | fills CellClass height/level fields |
| Bridge overlays / bridge tile flags | map overlay + bridge tile tables | set bridge flags and bridge/tube indices |
| `Height` / `OccupyHeight` | `art.ini` / `artmd.ini` building art | building visual height and occupation/behind-building behavior |
| locomotor type / speed type | `rules.ini` / `rulesmd.ini` object type data | controls which passability and A* rules apply |

## 6. Current Rust Implementation Status

### Matching foundations

| Binary behavior | Rust surface | Status |
|-----------------|--------------|--------|
| cell grid plus sub-cell precision | `Position { rx, ry, sub_x, sub_y, z }` in `src/sim/components.rs` | Conceptually matches |
| 256 leptons per cell | `src/util/lepton.rs` constants/helpers | Matches |
| tile iso projection `30*(rx-ry)-30`, `15*(rx+ry)+15-z*15` | `src/map/terrain.rs::iso_to_screen` | Matches current documented gamemd tile anchor |
| object/cell-center projection | `src/util/lepton.rs::lepton_to_screen` | Matches `CoordsToClient(cell center)` |
| ground/bridge occupancy split | `src/sim/occupancy.rs` stores layer-tagged occupants | Conceptually matches `FirstObject`/`AltObject` |
| ground/bridge path state | `PathCell`, `LayeredPathStep`, `MovementLayer::Bridge` | Conceptually matches A* height/layer split |
| bridge deck height as ground + elevated level | `bridge_deck_level`, `effective_cell_z_for_layer` | Conceptually matches, but exact derivation depends on bridge parsing/runtime state |

### Parity risks to keep visible

| Risk | Why it matters |
|------|----------------|
| Rust `Position.z` is `u8` height level, while gamemd object `Z` is an int lepton coordinate | Need careful conversion: level height, object altitude, projectile Z, and render Z are not always the same unit. |
| Rust occupancy uses one `BTreeMap` with layer-tagged occupants, not separate linked lists | Conceptually fine, but ordering/parity-sensitive object iteration may need gamemd-compatible ordering when behavior depends on first object found. |
| gamemd `AddContent` appends buildings but prepends most other objects | Rust vector/list order may differ and affect targeting/passability edge cases if not normalized. |
| A* in gamemd uses two closed/g-cost arrays keyed by height decision, not only an enum layer | Rust `MovementLayer` is a clean abstraction; verify each transition reproduces gamemd height thresholds. |
| gamemd uses signed/biased divide-by-256 behavior in projection and cell conversion | Rust must match negative/off-map coordinate behavior for particles, projectiles, and edge-of-map visuals. |

## 7. Integration Points

| Function | Role |
|----------|------|
| `CoordsToClient` (`0x006D1F10`) | Converts 3D world/lepton coords to 2D tactical pixels |
| `Tactical__AdjustForZ` (`0x006D20E0`) | Converts Z leptons to screen-Y lift |
| `CellClass__Get_Center_Coords` (`0x00480A30`) | Cell -> center lepton coords with ground Z |
| `ObjectClass__GetCoords` | Object -> 3D lepton coords |
| `CellClass__AddContent` / `RemoveContent` | Maintain ground vs bridge object lists |
| `CellClass__GetEffectiveHeight` | Terrain level plus bridge +4 effective height |
| `AStar_pathfind_search` / `AStar_main_loop` | Height-aware cell pathfinding |
| `UnitClass__Can_Enter_Cell` | Passability and object-list selection |

## 8. Answer to the Original Question

The repo's current conceptual model is correct at the foundation level:

```text
one 2D isometric cell coordinate system,
plus sub-cell lepton precision,
plus a separate Z/elevation value,
plus logical movement/occupancy layers for bridge-vs-ground cases.
```

So "layered 2.5D isometric coordinate system" is an accurate label for gamemd and for the intended Rust model.

The most important nuance: gamemd does have true 3-int object/world coordinates `(X, Y, Z)` in leptons, but the **map topology and CellClass storage are 2D**. Bridge/ground layering is represented by separate object lists and height-aware pathfinding state, not by a general-purpose 3D grid.

## 9. Open Questions

1. **Exact object-list iteration parity in Rust:** Rust's occupancy order should be audited against gamemd's `AddContent` insertion order, especially for passability/target selection when multiple objects share a cell.
2. **Negative coordinate projection/cell conversion:** `CoordsToClient` uses a signed bias before `>> 8`; Rust helpers should be checked for exact parity at negative lepton coordinates.
3. **Bridge deck derivation edge cases:** Effective height `Level + 4` is verified for specific binary helpers, but bridgehead/tube/ramp cells have more specialized path behavior covered in bridge-specific reports.
4. **Air/underground layers:** The foundational proof here covers ground/bridge strongly. Air and underground are locomotor-specific and should be treated as separate investigations when their parity matters.

## Sources

- Ghidra decompiled:
  - `CoordsToClient` at `0x006D1F10`
  - `Tactical__AdjustForZ` at `0x006D20E0`
  - `CellClass__Get_Center_Coords` at `0x00480A30`
  - `CellClass__AddContent` at `0x0047E8A0`
  - `CellClass__RemoveContent` at `0x0047EA90`
  - `CellClass__GetEffectiveHeight` at `0x00487D50`
  - `AStar_pathfind_search` at `0x0042C900`
  - `AStar_main_loop` at `0x00429A90`
  - `UnitClass__Can_Enter_Cell` at `0x0073F0A0`
  - `ObjectClass__GetCoords` at `0x005F65A0` (corrected 2026-07-12: address added)
- Existing reports referenced:
  - `COORDINATE_SYSTEM_GAMEMD.md`
  - `LOCOMOTION_MATH_AND_CONSTANTS.md`
  - `CELLCLASS_STRUCT_GHIDRA_REPORT.md`
  - `PATHFINDING_ASTAR_GHIDRA_REPORT.md`
  - `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`
  - bridge-specific reports in `C:/Users/enok/Documents/ra2-rust-game-docs/`
- Rust files checked:
  - `src/map/terrain.rs`
  - `src/util/lepton.rs`
  - `src/sim/components.rs`
  - `src/sim/occupancy.rs`
  - `src/sim/pathfinding/core.rs`
  - `src/sim/movement/locomotor.rs`
