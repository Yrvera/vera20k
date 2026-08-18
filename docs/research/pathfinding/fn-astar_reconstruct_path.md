# AStar_reconstruct_path — Decode Doc
**Proposed Ghidra label:** AStar_reconstruct_path (already labelled)

## Summary

Path backtracking function at `0x0042AA90`. Called by `AStar_main_loop` (`0x00429a90`)
immediately after the goal node is found. Walks the parent-pointer chain from the goal
node back to the start, converting cell-to-cell deltas into 8-direction bytes (0–7 = S,
SW, W, NW, N, NE, E, SE), writes them into a caller-supplied output buffer from last
to first, appends a 0xFFFFFFFF sentinel, and returns a pointer to a global output struct
containing the first node's X coordinate and the path length. The output buffer and this
struct are then consumed by `Path_smooth_corners` and `Path_optimize_straight_segments`.

**Active in YR: Yes.** Called exclusively by `AStar_main_loop` at `0x00429a90` inside the
`if (1 < piStack_48[3])` guard (path has at least 2 nodes). That function is live in
standard YR skirmish via the locomotor movement chain. Verified via
`get_function_callers 0x0042AA90`.

---

## Signature

```c
undefined * AStar_reconstruct_path(
    int *param_1,   // AStarNode* goal node (head of parent-pointer chain)
    int  param_2    // base address of direction-byte output buffer
)
```

Returns pointer to `DAT_0089a2d8` (a global struct written during backtrack).

---

## Decompilation Excerpt

```c
// verified via decompile_function 0x0042AA90
// One-time static initializer guard
if ((DAT_0089a300 & 1) == 0) {
    DAT_0089a300 |= 1;
    FUN_007c978a(&DAT_0042abf0);  // memset/init out-of-scope runtime utility
}

// Populate global output struct (at DAT_0089a2dc..)
DAT_0089a2dc       = Math__ftol();        // path length (integer)
_DAT_0089a2e0      = param_1[3];          // AStarNode[3] = step count
_DAT_0089a2e8      = 0;
_DAT_0089a2f0      = 0;
_DAT_0089a2f2      = 0;
_DAT_0089a2e4      = param_2;             // output buffer ptr
_DAT_0089a2ec      = &DAT_0089a324;       // direction-array sentinel start

// Walk parent-pointer chain backwards, fill direction bytes
puVar6 = (undefined4 *)*param_1;          // goal node's cell ptr
puVar5 = (undefined4 *)puVar6[2];         // goal node's parent's cell ptr
iVar1  = param_1[3] + -2;                 // write index = (path_len - 2)
local_c = (undefined4 *)(param_2 + iVar1 * 4);  // write ptr (from end of buffer)
local_8 = param_1[3] + -1;                // step count

// Backtracking loop: fills buffer[path_len-2] down to buffer[0]
do {
    if (puVar5 != 0) {
        // Store parent cell index at buffer[i]
        *(undefined4 *)(... + (int)local_c) = puVar5[1];  // cell coord/index of parent node

        // Compute delta between consecutive cells
        // node cell: *(short *)(*cell_ptr + 0x26) = Y, *(short *)(*cell_ptr + 0x24) = X
        uVar8 = Y_next - Y_curr;   // dy = sign-extended short diff
        if (|dy| < 2) {
            dx = X_next - X_curr;
            if (|dx| <= 1) {
                // Direction lookup: index = dy * 3 + dx
                uVar3 = g_DirTable[dy * 3 + dx];  // @ 0x00818760 (base)
            } else goto dir_invalid;
        } else {
dir_invalid:
            uVar3 = 8;  // non-adjacent / invalid direction
        }
        *local_c = uVar3;  // write direction byte
    }
    // Advance both node ptrs along parent chain
    puVar6 = puVar6[2];    // current = parent
    puVar5 = puVar5[2];    // parent = grandparent
    local_c--;             // step backward in buffer
    local_8--;
} while (local_8 != 0);

// Write end-of-path sentinel
*(undefined4 *)(param_2 - 4 + param_1[3] * 4) = 0xffffffff;

// Record first node's X coord in global output struct
_DAT_0089a2d8 = *(undefined4 *)(*puVar6 + 0x24);  // first node X cell coord
if (DAT_0089a2dc == 0) DAT_0089a2dc = 1;           // clamp length to ≥1

return &DAT_0089a2d8;
```

---

## Behavioral Analysis

### Direction encoding: 8 cardinals, 0-based South

The 9-entry lookup table at `0x00818760` (base for `LAB_0081875f_1 + 1`) maps
`index = dy * 3 + dx` → direction byte. Verified via `read_memory 0x00818750` (40 bytes):

| dy | dx | index | raw addr | direction byte | Compass |
|----|----|----|------|-----|----|
| -1 | -1 | -4 | 0x818750 | 3 | NW |
| -1 | 0  | -3 | 0x818754 | 4 | N  |
| -1 | +1 | -2 | 0x818758 | 5 | NE |
| 0  | -1 | -1 | 0x81875C | 2 | W  |
| 0  | 0  |  0 | 0x818760 | 0xFFFFFFFF | same cell (invalid) |
| 0  | +1 | +1 | 0x818764 | 6 | E  |
| +1 | -1 | +2 | 0x818768 | 1 | SW |
| +1 | 0  | +3 | 0x81876C | 0 | S  |
| +1 | +1 | +4 | 0x818770 | 7 | SE |

Frame note: dx/dy are cell-grid deltas (cell-space, +X=East, +Y=South). `dy = Y_next - Y_curr`,
`dx = X_next - X_curr` read from consecutive nodes going **forwards** in the path (source→goal),
but the loop walks **backwards** (goal→source) so node pair access is `puVar5` (goal-side) vs
`puVar6` (source-side).

Direction encoding: 0=S, 1=SW, 2=W, 3=NW, 4=N, 5=NE, 6=E, 7=SE. This is the direction
index used by `Path_walk_directions_to_cell` (confirmed consistent with `g_CellNeighborOffsets_8Dir`
usage in `AStar_main_loop` which iterates iStack_44 from 0 to 7 for 8 neighbors).

If `|dy| >= 2` OR `|dx| > 1` (non-adjacent diagonal — shouldn't happen in a valid A*
path), direction byte is set to **8** (out-of-range, treated as invalid by smoothing).

### Buffer layout
The output buffer (`param_2`) is indexed by step number:
- `buffer[0]` .. `buffer[path_len - 2]` = direction bytes (0–7 or 8 if invalid)
- `buffer[path_len - 1]` = 0xFFFFFFFF sentinel

The buffer is written **backwards** during the loop: the backtracking walk produces
pairs (goal→start) but the buffer is filled from `param_2 + (path_len - 2) * 4` down
to `param_2 + 0`, so that `buffer[0]` holds the direction from the **start** node to
the next node.

### AStarNode struct accesses

Each node pointer (`*puVar6`) points to a CellClass struct (the cell). Key fields accessed:
- `*(short *)(*node + 0x24)` = CellClass X coordinate (cell X, short) — **CellClass+0x24**
- `*(short *)(*node + 0x26)` = CellClass Y coordinate (cell Y, short) — **CellClass+0x26**

Node array layout from `param_1[N]`:
- `param_1[0]` = `*puVar6` = pointer to the goal CellClass
- `param_1[2]` = `puVar6[2]` = pointer to parent node (cell pointer chain)
- `param_1[3]` = path step count (length of the node chain)

These offsets are read directly from the decompile and cross-checked with the
`AStar_main_loop` decompile which accesses the same fields at `*(short *)(*piVar22 + 0x24)`
and `*(short *)(*piVar22 + 0x26)` (verified via `decompile_function 0x00429a90`).

### Global output struct written by this function

| Address | Field | Value written |
|---------|-------|---------------|
| `DAT_0089a2d8` | first node X coord (start cell X) | `*(short *)(*puVar6 + 0x24)` after backtracking to start |
| `DAT_0089a2dc` | path length (integer, ≥1) | `Math__ftol()` result (from A* node count), clamped to ≥1 |
| `_DAT_0089a2e0` | step count from node | `param_1[3]` |
| `_DAT_0089a2e4` | output buffer base | `param_2` |
| `_DAT_0089a2e8` | zeroed | 0 |
| `_DAT_0089a2ec` | direction array sentinel base | `&DAT_0089a324` |
| `_DAT_0089a2f0` | zeroed | 0 |
| `_DAT_0089a2f2` | zeroed | 0 |

The function returns `&DAT_0089a2d8` — a pointer to this global struct. The caller
(`AStar_main_loop`) passes this value directly into `Path_smooth_corners` and
`Path_optimize_straight_segments` as the first argument.

### One-time static initializer
The `if ((DAT_0089a300 & 1) == 0)` guard with `FUN_007c978a(&DAT_0042abf0)` is a
standard C++ lazy-init pattern (thread-unsafe one-time static initializer). `FUN_007c978a`
is an out-of-scope runtime utility (memset/memcpy-class). `DAT_0042abf0` is within the
function body — likely a small static data block zeroed on first call.

### Max path length
No explicit max-path-length check inside `AStar_reconstruct_path`. The function trusts
`param_1[3]` and iterates exactly `param_1[3] - 1` times. The buffer allocation at
`param_5` (passed as `param_2` in the A*_main_loop call) must be large enough for
`param_1[3]` dwords. The actual max is controlled by `param_6` (node expansion limit =
`0xfff7` = 65527 steps by default in `AStar_main_loop`). So in theory up to 65527
direction bytes, but in practice constrained by map size.

---

## Struct Field Accesses

| Access | Object | Byte offset | Interpretation |
|--------|--------|-------------|----------------|
| `*(*puVar6 + 0x24)` | CellClass (short) | 0x24 | Cell X coordinate |
| `*(*puVar6 + 0x26)` | CellClass (short) | 0x26 | Cell Y coordinate |
| `param_1[3]` | AStarNode (int*) | 0x0C (= 3×4) | Path step count |
| `puVar6[2]` | AStarNode (ptr) | 0x08 (= 2×4) | Parent node pointer |
| `puVar5[1]` | AStarNode (ptr) | 0x04 (= 1×4) | Parent cell index/coord |

Note: `param_1` is an `int*` (Ghidra shows `int *`). All `param_1[N]` accesses are
byte offset `N × 4`.

---

## Callers

| Caller | Address | Notes |
|--------|---------|-------|
| `AStar_main_loop` | `0x00429a90` | Sole caller (verified via `get_function_callers 0x0042AA90`). Called when path found and `piStack_48[3] > 1` (path has ≥2 nodes). |

---

## Callees

| Callee | Address | Role |
|--------|---------|------|
| `Math__ftol` | `0x007c5f00` | Convert float path length to integer (out-of-scope utility) |
| `FUN_007c978a` | `0x007c978a` | One-time static init (memset-class, out-of-scope runtime, per manifest) |

All callees verified via `get_function_callees 0x0042AA90`. `Sqrt_Approx` listed in
task description is NOT called by this function — it is called by `AStar_create_node`
or `AStar_compute_edge_cost` instead. Marking as mismatch: no `Sqrt_Approx` call in
the decompile of `0x0042AA90`.

---

## Globals / Enums / INI

| Symbol | Address | Role |
|--------|---------|------|
| `g_DirTable` (inferred name) | `0x00818760` | 9-entry direction lookup: dy*3+dx → direction byte 0–7 |
| `DAT_0089a2d8` | `0x0089a2d8` | Global: first node X coord (returned as struct base) |
| `DAT_0089a2dc` | `0x0089a2dc` | Global: path length (int, ≥1) |
| `DAT_0089a300` | `0x0089a300` | Static init guard byte |

No INI keys read by this function.

---

## Out-of-Scope References

- `Path_smooth_corners` (consumer of output) — in-scope task #13.
- `Path_optimize_straight_segments` (consumer of output) — in-scope task #14.
- `Math__ftol` — out-of-scope runtime utility (per manifest).
- `FUN_007c978a` — out-of-scope runtime utility (per manifest).

---

## Unverified / YELLOW

- **YELLOW: AStarNode struct fields `[0]`, `[2]`, `[3]`.** The offsets 0x00, 0x08, 0x0C
  from the node pointer are used for: `[0]` = CellClass pointer, `[2]` = parent node
  pointer, `[3]` = step count. These are read directly from the decompile and
  cross-checked against `AStar_main_loop` usage, but the AStarNode struct is formally
  decoded in task #26 (pathfinder_class_struct). Mark consistent-with-observed but
  not independently verified.

- **YELLOW: `puVar5[1]` usage.** The value `puVar5[1]` (offset 0x04 from parent's node
  pointer, parent's cell index) is written into the output buffer at `DAT_0089a324 + i*4`.
  This appears to be the cell index rather than a direction byte — it's used as a
  cell reference in `Path_walk_directions_to_cell`. The exact type (cell coord vs flat
  index) is UNCHECKED.

- **YELLOW: `Sqrt_Approx` noted in task description.** Decompile of `0x0042AA90`
  contains no call to `Sqrt_Approx`. The task description may refer to a callee of a
  related function. CHECKED: not present in this function's decompile.
