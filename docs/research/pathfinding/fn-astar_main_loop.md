# AStar_main_loop — Decode Doc
**Proposed Ghidra label:** AStar_main_loop

## Summary

`AStar_main_loop` at `0x00429A90` is the core cell-level A* search loop inside
the YR pathfinder. It runs the inner expand/relax cycle: it pops the minimum-f
node from a binary min-heap, expands all 8 cardinal/diagonal neighbors plus an
optional tunnel (direction 9 = tube), and updates ground/bridge dual closed-list
arrays with per-cell g-cost values. When the destination node is reached or the
iteration budget is exhausted the function calls `AStar_reconstruct_path` then
`Path_smooth_corners` and `Path_optimize_straight_segments` to return a cleaned
`PathType *`.

Key design points:
- **Dual closed lists**: ground and bridge layers use separate epoch-marker and
  g-cost arrays; a cell closed as ground does not block a bridge-layer visit.
- **Reopen tolerance**: closed cells are not true-reopened. An early skip fires
  when `existing_g < current.g + 1.009`; otherwise legality/cost work continues
  but the insertion guard still refuses creation if the marker equals the current
  epoch.
- **Hierarchical zone gate**: when the hierarchy flag is set, only cells whose
  level-0 zone is marked in `Pathfinder+0x40` are expanded; off-marker cells with
  `CellClass+0x122 != 0` (blocker-neighbor refcount) are excepted.
- **Direction order + epsilon**: directions 0..7 = N, NE, E, SE, S, SW, W, NW
  with additive tie-break epsilons `[0.001, 0.005, 0.002, 0.006, 0.003, 0.007,
  0.004, 0.008]`; direction 8 = tube, no epsilon, Chebyshev cost.

## Active in YR

**Yes — unconditional.** Sole caller is `AStar_pathfind_search @ 0x0042C900`
(verified via `get_function_callers 0x00429A90`), which is on the standard
`FootClass::Run_AStar → AStar_pathfind_search → AStar_main_loop` chain. No
TS-only gate was observed.

## Callers

Verified via `get_function_callers 0x00429A90`:

| Caller | Address | Role |
|--------|---------|------|
| `AStar_pathfind_search` | `0x0042C900` | Hierarchical A* wrapper; passes `param_7` hierarchy flag |

## Callees

Verified via `get_function_callees 0x00429A90`:

| Callee | Address | Role |
|--------|---------|------|
| `AStar_compute_edge_cost` | `0x00429830` | Per-edge passability + cost (locomotor `Can_Enter_Cell`) |
| `AStar_create_node` | `0x0042A460` | Allocate and initialise a node with g + heuristic |
| `AStar_reconstruct_path` | `0x0042AA90` | Walk parent chain to build `PathType` step array |
| `Path_optimize_straight_segments` | `0x0042B7F0` | Post-pass: merge collinear steps |
| `Path_smooth_corners` | `0x0042B210` | Post-pass: cut corners |
| `PathfinderClass__UpdateBridgePassability` | `0x0042ACF0` | Refresh bridge-pass state after path found |
| `RateTimer__Current` | `0x004C93D0` | Read current frame/time for naval layer probe |
| `ZoneMap__CellToZoneIndex` | `0x0056D3F0` | Map cell coords to zone index |

## Decompilation excerpt (key blocks)

Source: `decompile_function 0x00429A90`.

### Block 1 — Setup: CellClass pointer resolution and height initialisation

```c
// Resolve start and dest CellClass pointers
piVar2 = (int *)(g_CellArray_Base + (param_3[1] * 0x200 + (int)*param_3) * 4); // dest cell
puVar3 = (uint *)(g_CellArray_Base + (param_2[1] * 0x200 + (int)*param_2) * 4); // start cell

// Determine start height (layer)
iVar12 = (**(code **)(*param_4 + 0x2c))();  // vtable+0x2C = GetCurrentMission()
if ((iVar12 == 2) || ((*(uint *)(iVar13 + 0x140) & 0x100) == 0)) {
    iVar13 = (int)*(char *)(iVar13 + 0x11b);    // CellClass+0x11B = ground level
} else {
    iVar13 = *(char *)(iVar13 + 0x11b) + 4;     // bridge layer = ground level + 4
}
*(int *)(param_1 + 0x34) = iVar13;             // Pathfinder+0x34 = dest height
```

`param_1` is the `PathfinderClass *`. All Pathfinder offsets are direct byte
offsets (param_1 is `int *` in Ghidra so `[N]` = byte offset N×4).

### Block 2 — Naval same-dest guard and bridge start-layer correction

```c
iVar13 = (**(code **)(*piVar22 + 0x84))();  // vtable+0x84 = GetTechnoType()
if (*(char *)(iVar13 + 0xc94) != '\0') {    // TechnoTypeClass+0xC94 = Naval
    // If naval unit, bridge cell at start, and height diff > 2: bump start height +4
    if (((*(uint *)((int)param_2 + 0x140) & 0x100) != 0) &&
        (abs(piVar22[0x29] / DAT_0089c2d8 - Pathfinder+0x30) > 2)) {
        *(int *)(param_1 + 0x30) = *(int *)(param_1 + 0x30) + 4;
    }
}
```

`DAT_0089c2d8` is a cell-height-to-level scale constant.

### Block 3 — Hierarchical zone gate (level-0 marker check + CellClass+0x122 exception)

For each neighbor cell:

```c
iVar17 = ZoneMap__CellToZoneIndex(psVar1);
uVar14 = (uint)*(short *)(DAT_0087F858 + iVar17 * 10);   // level-0 zone id
if (*(int *)(iVar13 + uVar14 * 4) == iVar17) {            // Pathfinder+0x40 marker?
    // zone is on chosen path → proceed
} else {
    if ((char)param_2 != '\0') {                // near-height / normal branch?
        if ((*(char *)(iVar16 + 0x122) == '\0') && (param_7 != '\0'))
            goto skip_neighbor;                 // off-marker + no blocker-neighbor + hierarchy
        // else: CellClass+0x122 != 0 → off-marker exception, allow
    }
    // bridge-layer candidates: no +0x122 exception
}
```

Verified: `0x00429E85..0x00429EC1` — `ZoneMap__CellToZoneIndex` call at
`0x00429E85`; epoch compare at `0x00429EA4`; `+0x122` read at `0x00429EB1`.

**`CellClass+0x122`** is a blocker-neighbor refcount (count of occupied/blocked
cells in the 8-cell neighborhood), not fog/shroud/water. Off-marker cells
adjacent to blockers still participate in hierarchical A*.

### Block 4 — Dual closed-list tolerance test (ground and bridge)

The function maintains two separate closed-list arrays keyed by search epoch:
- `Pathfinder+0x18`: ground layer epoch-marker array (index = cell linear index)
- `Pathfinder+0x1C`: bridge layer epoch-marker array
- `Pathfinder+0x24`: ground layer stored g-cost array
- `Pathfinder+0x20`: bridge layer stored g-cost array
- `Pathfinder+0x28`: current search epoch

**Ground branch** (verified: assembly `0x00429ECF..0x00429F02`):
```asm
00429ecf: MOV ECX,[ESI+0x18]     ; ground marker array
00429edd: CMP [ECX+EBP],EAX      ; marker == epoch?
00429ee0: JNZ 0x00429f37         ; not closed → skip tolerance
00429ee6: MOV EAX,[ESI+0x24]     ; ground g array
00429ee9: FLD float ptr [EDX+0x4] ; current_node.g  (node+0x04 = g)
00429eec: FADD double ptr [0x007e37c0] ; + 1.009
00429ef2: FLD float ptr [EAX+EBP] ; existing stored g
00429ef5: FCOMPP
00429ef9: FNSTSW AX
00429efc: JNZ 0x0042a1a1         ; existing_g < current_g+1.009 → skip neighbor
```

**Bridge branch** is identical using `+0x1C` / `+0x20` arrays
(verified: `0x00429F04..0x00429F31`).

Layer selection: bridge layer when `CellClass+0x140 & 0x100` and
`abs(Pathfinder+0x30 - neighbor.level) >= 2`; otherwise ground.

### Block 5 — Direction expansion (8 normal + 1 tube)

Loop `iStack_44` runs 0..8 (total 9 iterations):

```c
do {
    if (iStack_44 == 8) {
        // Tube (direction 8): get exit cell from g_TubeArray
        // Cost = Chebyshev distance to tube exit, no epsilon, no AStar_compute_edge_cost
    } else {
        // Normal direction: neighbor = current + g_CellNeighborOffsets_8Dir[iStack_44]
        fStack_28 = (float)(AStar_compute_edge_cost(...) * Pathfinder+0x04
                            + g_DirEpsilonTable[iStack_44]);
    }
    iStack_44++;
} while (iStack_44 < 9);
```

`g_CellNeighborOffsets_8Dir` @ `0x007E3774`: cell-pointer relative offsets for
N, NE, E, SE, S, SW, W, NW on a 512-wide map (verified via `read_memory
0x007E3774 length=32`: `00feffff 01feffff 01000000 01020000 00020000 ff010000
ffffffff fffdffff`).

`g_DirEpsilonTable` @ `0x0081872C` — 9 floats (verified via `read_memory
0x0081872C length=36`):

| Index | Direction | Epsilon |
|-------|-----------|---------|
| 0 | N | 0.001 |
| 1 | NE | 0.005 |
| 2 | E | 0.002 |
| 3 | SE | 0.006 |
| 4 | S | 0.003 |
| 5 | SW | 0.007 |
| 6 | W | 0.004 |
| 7 | NW | 0.008 |
| 8 | tube | 0.0 (unused) |

### Block 6 — Later insertion guard (prevents true reopening)

After legality/cost work, a second marker check prevents inserting an already-closed
cell into the heap:

```asm
; Ground:
00429ffb: MOV EAX,[ESI+0x18]
0042a004: CMP ECX,EAX            ; marker == epoch?
0042a006: JNZ 0x0042a01e         ; not closed → create node OK
0042a008: JMP 0x0042a1a1         ; already closed → no node

; Bridge:
0042a00d: MOV EDX,[ESI+0x1c]
0042a013: CMP EAX,ECX
0042a018: JZ 0x0042a1a1          ; already closed → no node
0042a01e: CALL 0x0042a460        ; AStar_create_node
```

This is **not** true A* reopening: a cell once closed on a layer cannot receive
a new heap node for that layer in the same search epoch.

### Block 7 — In-loop min-heap sift (local candidate comparison)

When a new candidate node is created within a direction pass, it competes with
a local best candidate `piStack_40`. The node with lower `node+0x08` (heap `f =
g + heuristic`) replaces the incumbent in the open heap.

The heap sift-up code at `0x0042A081` and sift-down at `0x0042A29D` / `0x0042A39D`
use strict float `FCOMP`-based comparisons on `node+0x08`.

### Block 8 — Blocked-goal fallback

When `AStar_compute_edge_cost` returns `>= 7` (impassable), if the neighbor is
the destination cell and `abs(Pathfinder+0x30 - Pathfinder+0x34) < 2`:

```c
// 0x0042A17D..0x0042A19B
if ((piVar23 == piVar2) && (!bVar9) &&
    (abs(Pathfinder+0x30 - Pathfinder+0x34) < 2))
    goto LAB_0042a3de;   // accept current node as path end
```

The current node (not the destination) becomes the path terminus when the
destination is blocked but height-compatible.

### Block 9 — Termination and post-processing

```c
if ((local_34 != 10000) &&           // iteration limit
    (piStack_48 != 0) &&              // valid node
    (local_34 != param_6) &&          // caller budget
    (1 < piStack_48[3])) {            // node has > 1 step
    uVar19 = AStar_reconstruct_path(piStack_48, param_5);
    Path_smooth_corners(uVar19, param_4);
    Path_optimize_straight_segments(uVar19, piVar2);
    PathfinderClass__UpdateBridgePassability(piVar2);
    return uVar19;
}
return 0;
```

`local_34` = iteration counter (incremented each outer loop).

## PathfinderClass struct fields (param_1)

`param_1` is `int *` → byte offset = Ghidra `[N] × 4`.

| Byte offset | Ghidra index | Type | Name | Notes |
|-------------|--------------|------|------|-------|
| `0x04` | `[1]` | `float` | `edge_cost_multiplier` | Multiplied with `AStar_compute_edge_cost` result |
| `0x08` | `[2]` | `char` | `search_flag_8` | Passed as arg to `Can_Enter_Cell` |
| `0x18` | `[6]` | `int *` | `ground_closed_marker_array` | Epoch per cell, ground layer |
| `0x1C` | `[7]` | `int *` | `bridge_closed_marker_array` | Epoch per cell, bridge layer |
| `0x20` | `[8]` | `float *` | `bridge_gcost_array` | Accepted g-cost, bridge layer |
| `0x24` | `[9]` | `float *` | `ground_gcost_array` | Accepted g-cost, ground layer |
| `0x28` | `[10]` | `int` | `search_epoch` | Current search epoch; written to closed arrays |
| `0x2C` | `[11]` | `int *` | `open_heap` | Pointer to MinHeap struct |
| `0x30` | `[12]` | `int` | `current_path_height` | Layer height of node being expanded; updated from heap |
| `0x34` | `[13]` | `int` | `dest_height` | Destination layer height |
| `0x3C` | `[15]` | `int` | `update_bridge_pass_flag` | Non-zero → call UpdateBridgePassability |
| `0x40` | `[16]` | `int *` | `level0_zone_marker_array` | Chosen zone epoch array for hierarchy gate |
| `0x6C` | `[27]` | `int` | `chosen_chain_index` | Index into chosen zone chain |
| `0x70` | `[28]` | `int` | `chosen_chain_start` | Start coord of chosen chain |
| `0xBC` | `[0x2F]` | `short[N]` | `hier_chosen_chains[level]` | Per-level chosen zone chains |
| `0xC74` | `[0x31D]` | `int[level]` | `hier_chain_lengths` | Per-level chosen chain lengths |

## CellClass fields accessed

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x11B` | `char` | `level` | Terrain height level (0-based) |
| `+0x116` | `short` | `tube_index` | Tunnel/tube index; -1 = none |
| `+0x122` | `char` | `blocker_neighbor_count` | Count of blocker cells in 8-neighborhood; off-marker exception when > 0 |
| `+0x140` | `uint` | `cell_flags` | Bit `0x100` = bridge cell |
| `+0x24` | `short` | `cell_x` | X coordinate |
| `+0x26` | `short` | `cell_y` | Y coordinate |

## Globals referenced

| Global | Address | Value / Role |
|--------|---------|--------------|
| `g_CellArray_Base` | (global ptr) | Base of cell pointer array; `[y*512 + x]*4` = CellClass ptr |
| `g_CellNeighborOffsets_8Dir` | `0x007E3774` | 8 cell-pointer relative offsets for N..NW (verified `read_memory 0x007E3774`) |
| `g_DirEpsilonTable` | `0x0081872C` | 9 floats: [0.001,0.005,0.002,0.006,0.003,0.007,0.004,0.008,0.0] (verified `read_memory 0x0081872C`) |
| `DAT_007E37C0` | `0x007E37C0` | Double `1.009` = `be9f1a2fdd24f03f` (verified `read_memory 0x007E37C0`) |
| `DAT_0087F858` | `0x0087F858` | Zone index table; 10-byte tuples; first word = level-0 zone id |
| `g_TubeArray` | (global ptr) | Array of tube entry structs; indexed by `CellClass+0x116` |

## Control flow summary

```
AStar_main_loop(pathfinder, start_cell, dest_cell, unit, param5, budget, hier_flag)
├── Null guard: CellClass ptrs valid?
├── Compute dest_height → Pathfinder+0x34
├── Compute start_height → Pathfinder+0x30 (naval bridge correction)
├── Init epoch, create initial node via AStar_create_node
├── Early-exit: start == dest and same height → skip loop
│   └── Optional: UpdateBridgePassability
├── Main expansion loop (local_34 = 0..10000, budget):
│   ├── Pop current node (piStack_48) from open heap
│   ├── For each direction 0..8:
│   │   ├── Get neighbor cell pointer
│   │   ├── Level-0 zone marker gate (hierarchy):
│   │   │   ├── Zone marked → proceed
│   │   │   └── Zone not marked:
│   │   │       ├── CellClass+0x122 != 0 → allow (off-marker exception)
│   │   │       └── CellClass+0x122 == 0 && hier_flag → skip neighbor
│   │   ├── Choose ground/bridge layer
│   │   ├── Closed-list tolerance test:
│   │   │   ├── Not closed → continue
│   │   │   └── Closed: existing_g < current_g + 1.009 → skip neighbor
│   │   │       else continue (but insertion guard still blocks)
│   │   ├── AStar_compute_edge_cost → result < 7 = passable
│   │   │   ├── Passable: compute edge_cost * multiplier + epsilon
│   │   │   │   └── Insertion guard: marker != epoch → AStar_create_node
│   │   │   └── Impassable (≥7): blocked-goal fallback if neighbor == dest
│   │   └── Update local candidate, markers, g-cost array
│   ├── Heap maintenance (sift-up/sift-down on f = node+0x08)
│   └── Update Pathfinder+0x30 from next heap head
├── On termination: piStack_48 valid + > 1 step:
│   ├── AStar_reconstruct_path
│   ├── Path_smooth_corners
│   ├── Path_optimize_straight_segments
│   └── Optional: UpdateBridgePassability
└── Return PathType * or 0
```

## Internal mechanism notes

- `node+0x04` = accumulated g-cost from start (what closed arrays store).
- `node+0x08` = heap priority `f = g + heuristic`.
- `node+0x00` = CellClass pointer for this node.
- Heap structure: `open_heap[0]` = count; `open_heap[1]` = capacity;
  `open_heap[2]` = node pointer array (1-indexed); `open_heap[3]` = max ptr;
  `open_heap[4]` = min ptr. (inferred from heap maintenance code)

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `AStar_compute_edge_cost` | `0x00429830` | task #5 |
| `AStar_create_node` | `0x0042A460` | task #6 |
| `AStar_reconstruct_path` | `0x0042AA90` | task #7 |
| `Path_smooth_corners` | `0x0042B210` | task #13 |
| `Path_optimize_straight_segments` | `0x0042B7F0` | task #14 |
| `PathfinderClass__UpdateBridgePassability` | `0x0042ACF0` | task #10 |
| `AStar_pathfind_search` | `0x0042C900` | task #3 |
| `Zone_precheck` | `0x0042CB58` | task #16 |
| `ZoneMap__CellToZoneIndex` | `0x0056D3F0` | zone-map helper |

## YELLOW — Unverified

- `open_heap` struct layout at `Pathfinder+0x2C`: inferred from sift code; offset
  positions `[0]=count, [1]=capacity, [2]=node_array, [3]=max_ptr, [4]=min_ptr`
  not independently confirmed via `get_struct_layout`.
- `DAT_0087F858` zone tuple format: "first 16-bit word = level-0 zone id" inferred
  from `AStar_main_loop` code; full 10-byte tuple layout not separately verified.
- `Pathfinder+0x04` as `edge_cost_multiplier`: inferred from `fStack_28 =
  AStar_compute_edge_cost(...) * Pathfinder+0x04 + epsilon` usage; field name
  unconfirmed without struct layout tool.
- Naval bridge height correction formula using `DAT_0089c2d8`: constant value
  and exact height encoding unconfirmed — decompile shows the divide but constant
  value not read_memory verified.
