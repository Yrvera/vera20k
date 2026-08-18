# PathfinderClass__Reset — Decode Doc
**Proposed Ghidra label:** PathfinderClass__Reset

## Summary

`PathfinderClass__Reset` at `0x0042A5B0` resets all per-search state inside a
`PathfinderClass` instance in preparation for a new A* search. It is called by
`AStar_pathfind_search` before each search attempt (including retry iterations),
and by `FUN_0042D170` (the distance-check helper that also runs A*).

The function performs four operations:

1. **Clear pool counters**: resets node pool and CellInfo pool allocation
   counters to 0.
2. **Clear open heaps**: zeros the node pointer arrays for two heap structures
   (primary open-list heap at `+0x14`, secondary heap at `+0x68`) and resets
   their counts.
3. **Bump search epoch**: increments `Pathfinder+0x28` by 1. This epoch is
   used to stamp closed-list entries — incrementing it effectively clears all
   ground and bridge closed-list markers without zeroing the arrays.
4. **Epoch overflow clear**: if the epoch wraps to zero after the increment, the
   function explicitly zeroes the ground and bridge closed-list arrays and the
   three per-hierarchy-level zone marker arrays, then increments the epoch again
   (so the search starts with epoch = 1, not 0).

**Key invariant**: the closed-list ground/bridge arrays (`+0x18`, `+0x1C`) and
their g-cost arrays (`+0x24`, `+0x20`) are **not** zeroed on every reset. Only
the epoch changes. The arrays retain old values but those values are silently
ignored by the new search because they no longer match the current epoch. Full
zeroing only happens on epoch overflow (every 2^32 resets).

## Active in YR

**Yes.** Called by `AStar_pathfind_search @ 0x0042C900` (verified via
`get_function_callers 0x0042A5B0`) before every search attempt. Also called by
`FUN_0042D170 @ 0x0042D170` which is the distance-estimate helper. Both are on
the live YR pathfinding chain.

## Callers

Verified via `get_function_callers 0x0042A5B0`:

| Caller | Address | Role |
|--------|---------|------|
| `AStar_pathfind_search` | `0x0042C900` | Main search wrapper; calls Reset before each attempt |
| `FUN_0042D170` | `0x0042D170` | Distance estimate (task #20); calls Reset before its A* run |

## Callees

None — all operations are inline (verified via `get_function_callees 0x0042A5B0`).

## Decompilation analysis

Source: `decompile_function 0x0042A5B0`.

### Step 1 — Reset pool allocation counters

```c
*(undefined4 *)(*(int *)(param_1 + 0x0c) + 0x180000) = 0;  // CellInfo pool count = 0
*(undefined4 *)(*(int *)(param_1 + 0x10) + 0x100000) = 0;  // Node pool count = 0
```

`Pathfinder+0x0C` and `Pathfinder+0x10` are pool base pointers (established in
constructor). The "count" fields are stored at large fixed offsets within the pool
memory blocks:
- CellInfo pool counter at `pool + 0x180000`
- Node pool counter at `pool + 0x100000`

Setting these to 0 means the next `AStar_create_node` call reuses memory from the
start of each pool without allocating new memory.

### Step 2 — Clear open heap arrays (primary + secondary)

```c
// Primary open heap at Pathfinder+0x14
piVar6 = *(int **)(param_1 + 0x14);       // heap struct ptr
if (-1 < *piVar6) {                        // heap->count >= 0?
    for (iVar5 = 0; iVar5 <= *piVar6; iVar5++) {
        *(undefined4 *)(piVar6[2] + -4 + iVar5 * 4) = 0;  // zero node ptr entries
    }
}
*piVar6 = 0;                               // heap->count = 0

// Secondary heap at Pathfinder+0x68 (same pattern)
piVar6 = *(int **)(param_1 + 0x68);
... same clear loop ...
*piVar6 = 0;
```

Heap struct layout (inferred from usage):
- `heap[0]` = current count
- `heap[1]` = capacity (not zeroed here)
- `heap[2]` = pointer to node-pointer array (1-indexed, stride 4)
- Node array is cleared from index 0 to count (inclusive: `+(-4 + (i)*4)` with
  `i` starting at 1 is effectively slot [0]..[count]).

The secondary heap at `Pathfinder+0x68` is the retry/fallback candidate heap.

### Step 3 — Increment search epoch

```c
iVar5 = *(int *)(param_1 + 0x28) + 1;
*(int *)(param_1 + 0x28) = iVar5;
```

The epoch at `Pathfinder+0x28` is used to stamp closed-list entries (both ground
`+0x18` and bridge `+0x1C` arrays). After incrementing, old epoch-stamped entries
are effectively invisible to the new search — no array zeroing needed.

### Step 4 — Epoch overflow: full closed-list clear

```c
if (iVar5 == 0) {                          // epoch wrapped to zero?
    // Zero ground closed marker array
    iVar5 = DAT_0089c2dc * DAT_0089c2dc + -1;   // map_width^2 - 1 = last index
    while (-1 < iVar5) {
        *(undefined4 *)(*(int *)(param_1 + 0x18) + 4 + iVar5 * 4) = 0; // ground markers
        *(undefined4 *)(*(int *)(param_1 + 0x1c) + 4 + iVar5 * 4) = 0; // bridge markers
        iVar5--;
    }
    // Zero hierarchy zone marker arrays (3 levels, 3 arrays each)
    piVar6 = (int *)(param_1 + 0x4c);     // start of hier zone arrays
    local_8 = 3;
    do {
        iVar5 = DAT_0087F810[level];       // zone count for this level (runtime data)
        iVar1 = piVar6[-3];                // visited array base
        iVar2 = *piVar6;                   // chosen-path marker array base
        iVar3 = piVar6[3];                 // third array (path cost?)
        // Clear all three per-level zone arrays
        for (puVar4 = end_of_arrays; iVar5 > 0; iVar5--) {
            *(iVar1 + offset) = 0;
            *puVar4 = 0;
            *(iVar3 + offset) = 0;
            puVar4--;
        }
        piVar6++;
        local_8--;
    } while (local_8 != 0);
    // Bump epoch one more time so searches never use epoch=0
    *(int *)(param_1 + 0x28) = *(int *)(param_1 + 0x28) + 1;
}
```

`DAT_0089c2dc` is the map width (512 for a standard map), making
`DAT_0089c2dc * DAT_0089c2dc = 262144 = 0x40000` total cells — the full
closed-list array size (verified: `read_memory 0x0089c2dc` shows 0 at static
load; runtime value is 512).

The epoch-wrap clear ensures closed-list arrays never match a valid epoch value
of 0 by making epoch skip from `0xFFFFFFFF` → clear → `1` (epoch is bumped
twice: once above to reach 0, then once after clearing to reach 1).

## PathfinderClass fields modified

| Byte offset | Ghidra `[N]` | Name | What reset does |
|-------------|--------------|------|-----------------|
| `+0x0C` pool `+0x180000` | — | `cellinfo_pool_count` | Set to 0 |
| `+0x10` pool `+0x100000` | — | `node_pool_count` | Set to 0 |
| `+0x14` → `heap[0]` | — | `primary_heap_count` | Set to 0; heap array entries zeroed |
| `+0x14` → `heap[2]` array | — | `primary_heap_node_array` | Zeroed 0..count entries |
| `+0x28` | `[10]` | `search_epoch` | Incremented by 1 (or 2 on overflow) |
| `+0x68` → `heap[0]` | — | `secondary_heap_count` | Set to 0; heap array entries zeroed |
| `+0x68` → `heap[2]` array | — | `secondary_heap_node_array` | Zeroed 0..count entries |
| `+0x18` (on overflow only) | `[6]` | `ground_closed_marker_array` | All cells zeroed |
| `+0x1C` (on overflow only) | `[7]` | `bridge_closed_marker_array` | All cells zeroed |
| `+0x40` / `+0x4C` region (on overflow only) | — | `hier_zone_marker_arrays[0..2]` | All zone entries zeroed |

## Fields NOT reset (persist across searches)

The following Pathfinder state is explicitly preserved across Reset calls:

- `+0x18` / `+0x1C`: ground/bridge closed-list marker arrays — old values
  persist; old markers become invisible via epoch change.
- `+0x24` / `+0x20`: ground/bridge g-cost arrays — old values persist; only
  consulted if epoch matches, so stale values are safe.
- `+0x04`: `edge_cost_multiplier` — not touched.
- `+0x38`: flag set by `AStar_pathfind_search` before Reset is called.
- `+0x3C`: bridge passability update flag — not touched.
- `+0x40`: level-0 zone marker array — persists (reset only on epoch overflow).
- `+0xBC..+0xC74`: hierarchy chosen-zone chains and lengths — not zeroed.
- Bridge pass state for `UpdateBridgePassability` — not touched.

## Control flow summary

```
PathfinderClass__Reset(pathfinder)
├── cellinfo_pool_count = 0
├── node_pool_count = 0
├── primary_heap: zero node entries [0..count], count = 0
├── secondary_heap: zero node entries [0..count], count = 0
├── epoch = epoch + 1
└── if epoch == 0:
    ├── zero ground_closed[0..map_width²-1]
    ├── zero bridge_closed[0..map_width²-1]
    ├── for each hierarchy level 0..2:
    │   └── zero visited[], chosen[], cost[] arrays [0..zone_count-1]
    └── epoch = epoch + 1   (so epoch = 1, never 0)
```

## Globals referenced

| Global | Address | Value / Role |
|--------|---------|--------------|
| `DAT_0089c2dc` | `0x0089c2dc` | Map width (512); runtime-init, `read_memory` shows 0 at load |
| `DAT_0087F810` | `0x0087F810` | Zone count table per hierarchy level; runtime-populated |

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `AStar_pathfind_search` | `0x0042C900` | task #3 (completed) |
| `FUN_0042D170` | `0x0042D170` | task #20 |
| `AStar_main_loop` | `0x00429A90` | task #4 (completed); reads epoch for closed-list checks |

## YELLOW — Unverified

- `DAT_0089c2dc` = map width 512: consistent with total cell count (`512×512 =
  262144 = 0x40000`), and the multiply `DAT_0089c2dc * DAT_0089c2dc` matches
  the expected closed-list array size. However, `read_memory 0x0089c2dc` returns
  0 (uninitialized at load); the value 512 is inferred from map architecture and
  cross-references in other pathfinding code, not directly read at this address.
- `Pathfinder+0x68` as "secondary heap": the pattern is identical to `+0x14`
  (same pointer → count → array structure); "secondary" is inferred from
  position. Whether this is an alternative candidate heap or something else is
  not independently confirmed.
- `DAT_0087F810` zone count table layout: the loop accesses it relative to
  `piVar6` (which starts at `param_1 + 0x4C`). The 3 levels × 3 arrays
  interpretation is inferred from `local_8 = 3` outer loop and the three
  distinct array base offsets (`iVar1`, `iVar2`, `iVar3`). Full zone table
  layout not separately verified.
