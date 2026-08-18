# AStar_create_node — Decode Doc
**Proposed Ghidra label:** AStar_create_node

## Summary

`AStar_create_node` at `0x0042A460` allocates and initialises one A* search node
using two pre-allocated pool arrays stored in the `PathfinderClass` struct. It is
called by `AStar_main_loop` once per neighbor that passes the closed-list guard.

The function writes two distinct data structures:

1. **CellInfo record** (`puVar1`): 12-byte entry in the cell-info pool. Stores the
   CellClass pointer, path height for this node, and parent CellClass pointer.
2. **Node record** (`piVar7`): 16-byte entry in the node pool. Stores the
   CellInfo pointer, accumulated g-cost, heap priority f-cost (g + heuristic),
   and step depth from root.

The heuristic is Euclidean distance from the node's cell to the destination cell,
computed via `Sqrt_Approx`.

Layer (ground vs bridge) is determined at node-creation time from the parent node
height, the neighbor cell's bridge flag, and the neighbor cell's terrain level.

## Active in YR

**Yes.** Sole caller is `AStar_main_loop @ 0x00429A90` (verified via
`get_function_callers 0x0042A460`), which is on the live
`FootClass → Run_AStar → AStar_pathfind_search → AStar_main_loop` chain.

## Callers

Verified via `get_function_callers 0x0042A460`:

| Caller | Address | Role |
|--------|---------|------|
| `AStar_main_loop` | `0x00429A90` | Inner A* loop; creates a node per passable neighbor |

## Callees

Verified via `get_function_callees 0x0042A460`:

| Callee | Address | Role |
|--------|---------|------|
| `Sqrt_Approx` | `0x004CAC40` | Fast approximate Euclidean distance for heuristic |

## Decompilation analysis

Source: `decompile_function 0x0042A460`. Function signature:

```c
int * __thiscall
AStar_create_node(int param_1,      // PathfinderClass *this
                  int *param_2,     // parent node ptr (null for root)
                  int *param_3,     // neighbor cell ptr (as CellClass *)
                  short *param_4,   // destination cell coords [x, y]
                  float param_5)    // step cost (g-delta from parent)
```

### Pool allocation

Two separate pools are allocated inside the PathfinderClass:

```c
// Node pool: stride = 0x10 (16 bytes)
iVar3 = *(int *)(param_1 + 0x10);          // node_pool = Pathfinder+0x10
piVar7 = (int *)(iVar3 + *(int *)(iVar3 + 0x100000) * 0x10);
*(int *)(iVar3 + 0x100000) = ... + 1;      // bump node pool counter

// CellInfo pool: stride = 0x0C (12 bytes)
iVar3 = *(int *)(param_1 + 0x0c);          // cellinfo_pool = Pathfinder+0x0C
iVar4 = *(int *)(iVar3 + 0x180000);        // current cellinfo count
*(int *)(iVar3 + 0x180000) = iVar4 + 1;    // bump cellinfo pool counter
puVar1 = (undefined4 *)(iVar3 + iVar4 * 0xc);  // this entry
```

- `Pathfinder+0x0C`: base of CellInfo pool. Count stored at `pool + 0x180000`.
  Each entry is 12 bytes (3 × `int`).
- `Pathfinder+0x10`: base of node pool. Count stored at `pool + 0x100000`.
  Each entry is 16 bytes (4 × `int`).

### CellInfo record layout (12 bytes, at `puVar1`)

```c
puVar1[0] = param_3;           // CellClass pointer for this cell
puVar1[1] = path_height;       // layer height (computed below)
puVar1[2] = (root ? 0 : *param_2);  // parent CellClass ptr (puVar1[0] of parent)
```

**Path height computation** (layer selection):

```c
if (param_2 == 0) {
    // Root node: inherit current Pathfinder+0x30 height
    puVar1[1] = *(int *)(param_1 + 0x30);
} else {
    puVar1[1] = (int)*(char *)(*param_3 + 0x11b);   // neighbor cell ground level
    if ((*(uint *)(*param_3 + 0x140) & 0x100) != 0) {  // neighbor is bridge cell?
        uVar6 = *(uint *)(*param_2 + 0x140) & 0x100;   // parent is bridge cell?
        if (uVar6 != 0 && parent_height == neighbor_ground_level + 4) {
            puVar1[1] = neighbor_ground_level + 4;  // stay on bridge layer
        } else if (uVar6 == 0 &&
                   abs(neighbor_ground_level - parent_height) <= 1) {
            puVar1[1] = neighbor_ground_level + 4;  // ascend to bridge layer
        }
        // else: stay on ground level (height already set above)
    }
}
```

Summary of bridge-layer rules for `puVar1[1]`:
- Neighbor is NOT a bridge cell → `height = neighbor.level`
- Neighbor IS a bridge cell AND parent is also on bridge AND parent_h == neighbor_level + 4 → `height = neighbor_level + 4` (bridge layer)
- Neighbor IS a bridge cell AND parent is on ground AND `abs(neighbor_level - parent_h) <= 1` → `height = neighbor_level + 4` (ascend to bridge)
- Otherwise → `height = neighbor.level` (ground layer pass-through)

### Node record layout (16 bytes, at `piVar7`)

```c
piVar7[0] = (int)puVar1;       // pointer to this node's CellInfo record
piVar7[1] = (root ? 0 : (int)(param_5 + (float)param_2[1]));
                               // g = parent.g + step_cost  [accumulated g]
piVar7[2] = (int)(float)(heuristic + (float)piVar7[1]);
                               // f = g + h  [heap priority]
piVar7[3] = (root ? 1 : param_2[3] + 1);
                               // depth = parent.depth + 1  [step count from root]
```

**Heuristic computation:**
```c
dx = abs(*(short *)(*param_3 + 0x24) - param_4[0]);  // CellClass+0x24 = cell_x
dy = abs(*(short *)(*param_3 + 0x26) - param_4[1]);  // CellClass+0x26 = cell_y
heuristic = Sqrt_Approx((float)(dx*dx + dy*dy));      // Euclidean distance
piVar7[2] = (int)(heuristic + (float)piVar7[1]);      // f = g + h
```

The heuristic is Euclidean (L2) distance in cell units, using `Sqrt_Approx`
(verified via `get_function_callees 0x0042A460`).

## Node struct layout summary

### Node record (stride = 16 bytes, base at `Pathfinder+0x10` pool)

| Word offset | Byte offset | Type | Name | Notes |
|-------------|-------------|------|------|-------|
| `[0]` | `+0x00` | `int` | `cellinfo_ptr` | Pointer to matching CellInfo record |
| `[1]` | `+0x04` | `float` | `g` | Accumulated path cost from start |
| `[2]` | `+0x08` | `float` | `f` | Heap priority = g + heuristic |
| `[3]` | `+0x0C` | `int` | `depth` | Steps from root (1 for root node itself) |

### CellInfo record (stride = 12 bytes, base at `Pathfinder+0x0C` pool)

| Word offset | Byte offset | Type | Name | Notes |
|-------------|-------------|------|------|-------|
| `[0]` | `+0x00` | `int` | `cell_ptr` | CellClass pointer for this cell |
| `[1]` | `+0x04` | `int` | `path_height` | Layer height for this step (ground or bridge + 4) |
| `[2]` | `+0x08` | `int` | `parent_cell_ptr` | Parent CellClass pointer; 0 for root |

## PathfinderClass fields accessed (param_1)

| Byte offset | Ghidra `[N]` | Name | Notes |
|-------------|--------------|------|-------|
| `+0x0C` | `[3]` | `cellinfo_pool` | Pointer to CellInfo pool base; count at `+0x180000` |
| `+0x10` | `[4]` | `node_pool` | Pointer to node pool base; count at `+0x100000` |
| `+0x30` | `[12]` | `current_path_height` | Used as root node height when `param_2 == null` |

## CellClass fields accessed (via `param_3` / `*param_2`)

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x11B` | `char` | `level` | Terrain height level |
| `+0x140` | `uint` | `cell_flags` | Bit `0x100` = bridge cell |
| `+0x24` | `short` | `cell_x` | X coordinate (for heuristic) |
| `+0x26` | `short` | `cell_y` | Y coordinate (for heuristic) |

## Cross-reference with AStar_main_loop verified fields

From `fn-astar_main_loop.md` (task #4), the main loop reads:
- `piStack_48[1]` = `node.g` → matches `piVar7[1]` here
- `piStack_48[2]` = not directly compared but equals `f` → matches `piVar7[2]`
- `piStack_48[3]` = step depth → matches `piVar7[3]`
- `*piStack_48` = CellInfo pointer → `piVar7[0]` here
- `*puVar1` = CellClass pointer → `puVar1[0]` = `param_3`
- `puVar1[1]` = path height → used as `Pathfinder+0x30` when popped from heap
- `puVar1[2]` = parent cell pointer → used for path reconstruction

## Control flow summary

```
AStar_create_node(pathfinder, parent_node, neighbor_cell, dest_coords, step_cost)
├── Allocate CellInfo slot from Pathfinder+0x0C pool
│   ├── Write cell_ptr = neighbor_cell
│   ├── Compute path_height:
│   │   ├── Root (parent==null): height = Pathfinder+0x30
│   │   ├── Neighbor not bridge: height = neighbor.level
│   │   └── Neighbor is bridge:
│   │       ├── Parent on bridge + parent_h == neighbor_level+4 → height = level+4
│   │       ├── Parent on ground + abs(diff) <= 1 → height = level+4
│   │       └── else → height = neighbor.level (ground)
│   └── Write parent_cell_ptr = (root ? 0 : parent.cellinfo.cell_ptr)
├── Allocate Node slot from Pathfinder+0x10 pool
│   ├── Write cellinfo_ptr → CellInfo just allocated
│   ├── Write g = (root ? 0 : parent.g + step_cost)
│   ├── Compute heuristic = Sqrt_Approx(dx² + dy²)
│   ├── Write f = g + heuristic
│   └── Write depth = (root ? 1 : parent.depth + 1)
└── Return node ptr
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `AStar_main_loop` | `0x00429A90` | task #4 (completed) |
| `Sqrt_Approx` | `0x004CAC40` | math utility |
| `AStar_reconstruct_path` | `0x0042AA90` | task #7 (consumes node.depth and cellinfo.parent_cell_ptr) |

## YELLOW — Unverified

- Pool offset-within-pool arithmetic: `pool + 0x100000` as "node count" and
  `pool + 0x180000` as "cellinfo count": derived from decompilation arithmetic
  but the pool struct header layout was not independently verified via
  `get_struct_layout` or `read_memory`. These could be pool header fields at
  known positions rather than true fixed offsets.
- Root node `piVar7[3] = 1` (not 0): the decompilation `piVar7[3] = 1` for root
  is a direct read of `param_2[3] + 1` where `param_2 == 0` falls through to
  `piVar7[3] = 1`. Confirmed from decompile but not separately re-verified via
  assembly read.
- `puVar1[1] = neighbor_ground_level + 4` bridge-ascend condition:
  `abs(neighbor_level - parent_h) <= 1` — the exact decompiled condition is
  `(uVar6 ^ uVar5) - uVar5 < 2` (i.e., `abs <= 1`). Read directly from
  decompile; not cross-checked via assembly.
