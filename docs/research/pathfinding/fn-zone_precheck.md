# Zone_precheck — Decode Doc
**Proposed Ghidra label:** Zone_precheck

## Summary

`Zone_precheck` at `0x0042C290` is a hierarchical zone-level pre-search that runs
before the cell-level A* main loop. It performs a mini-A* over zone-graph edges
(not individual cells) at hierarchy levels 2, 1, and 0 (outer to inner), confirming
that a zone-level path exists from the source zone to the destination zone. On
success it also caches the zone-chain path (in `Pathfinder+0xBC` / `+0xC74` arrays)
for the cell-level search to use as a zone corridor filter.

If the source and destination are in the same zone at a given level, the function
records that immediately (no search needed for that level). If zones differ, it runs
a zone-level open-list search using the secondary heap (`Pathfinder+0x68`).

The function returns:
- `1` (with low byte = 1) on success (all three hierarchy levels succeeded)
- `0` on failure (zone path unreachable)

## Active in YR

**Yes.** Called by `AStar_pathfind_search @ 0x0042C900` (task #3, completed) before
the cell-level A* loop, and by `FUN_0042D170 @ 0x0042D170` (task #20). Both are on
the live YR pathfinding chain (verified via `get_function_callers 0x0042C290`).

## Callers

Verified via `get_function_callers 0x0042C290`:

| Caller | Address | Role |
|--------|---------|------|
| `AStar_pathfind_search` | `0x0042C900` | Calls zone precheck before cell-level A* |
| `FUN_0042D170` | `0x0042D170` | Distance-estimate helper (task #20) |

## Callees

Verified via `get_function_callees 0x0042C290`:

| Callee | Address | Role |
|--------|---------|------|
| `FootClass__Get_Slope_Speed_Factor` | `0x004DC760` | Returns unit's speed factor (for slope cost) |
| `ZoneMap__CellToZoneIndex` | `0x0056D3F0` | Maps a MapCoord to zone index at current level |
| `Zone_Estimate_Slope_Cost` | `0x00585F40` | Estimates slope traversal cost for a zone edge |
| `MinHeap__SiftDown` | `0x0042DCA0` | Restores heap order after pop (task #19) |
| `Math__ftol` | `0x007C5F00` | Float-to-long conversion for slope cost |

## Decompilation analysis

Source: `decompile_function 0x0042C290`. Ghidra pre-placed comment on this function:
> "Hierarchical zone precheck. When scanning an 8-byte edge entry, tests byte(edge+4);
> if nonzero, adds 0.001 from 0x007E3818 to the candidate zone cost."

### Signature

```c
uint __thiscall
Zone_precheck(int  param_1,    // PathfinderClass *this
              undefined4 param_2,   // source MapCoord
              undefined4 param_3,   // destination MapCoord
              int  param_4,    // SpeedType index (for passability matrix)
              int  param_5)    // FootClass * (for slope speed; 0 = no slope check)
```

### Slope check initialization

```c
if (param_5 == 0) {
    local_30 = 0;        // no slope table
    bVar11 = false;
} else {
    fVar27 = FootClass__Get_Slope_Speed_Factor(param_5);
    local_30 = *(param_5 + 0x21C);   // slope table ptr = FootClass+0x21C
    bVar11 = (fVar27 > _DAT_007e3810);  // enable if speed_factor > 1e-5
}
```

`_DAT_007e3810` = 1e-5 (verified: `read_memory 0x007E3810` → double 1e-5 in prior session).
`local_30` = `FootClass+0x21C` = per-SpeedType slope zone map pointer (same field used
by `Path_Reroute_Straight_Line` and `Path_smooth_single_segment`).

### Outer loop: hierarchy levels 2 → 1 → 0

`local_38` starts at 2 and decrements to 0 (three iterations). The function searches
from the outermost hierarchy level (2) down to the finest (0):

```c
local_38 = 2;
do {
    // --- Reset secondary heap (Pathfinder+0x68) ---
    piVar14 = *(int **)(param_1 + 0x68);
    // clear heap array entries [0..count], reset count to 0
    ...
    *piVar14 = 0;

    // --- Map source and destination cells to zone indices ---
    src_zone  = ZoneMap__CellToZoneIndex(param_2);
    dst_zone  = ZoneMap__CellToZoneIndex(param_3);
    // Read zone indices from zone table: DAT_0087f858 + zone_id * 10 + level * 2
    src_zone_level = *(short *)(DAT_0087f858 + src_zone * 10 + local_38 * 2);
    dst_zone_level = *(short *)(DAT_0087f858 + dst_zone * 10 + local_38 * 2);

    // Stamp source and destination zones as visited (using search epoch)
    visited_array[src_zone_level] = epoch;
    visited_array[dst_zone_level] = epoch;

    if (src_zone_level == dst_zone_level) {
        // Same zone at this level — no search needed
        *(short *)(param_1 + 0xbc + local_38 * 1000) = src_zone_level;
        *(param_1 + 0xc74 + local_38 * 4) = 1;
        // fall through to next level
    } else {
        // Different zones — zone A* needed
        // Push source zone node onto secondary heap
        node = allocate_zone_node(src_zone_level, g_cost=0, f_cost=0, depth=0);
        heap_push(secondary_heap, node);

        // Zone A* loop
        while (heap not empty) {
            current = heap_pop();
            if (current.zone == dst_zone) {
                // Path found — record chain into Pathfinder+0xBC / +0xC74
                record_zone_chain(param_1, current, local_38);
                break;
            }
            // Iterate edges of current zone (from DAT_0087f878 + level * 0x18)
            for each edge at current zone's edge list:
                neighbor_zone = edge.neighbor;
                edge_cost_base = DAT_007e3794[edge.cost_class];   // per cost-class float
                slope_penalty = 0;
                if (bVar11) {
                    slope_penalty = Zone_Estimate_Slope_Cost(
                        local_30, local_38, current.zone, neighbor_zone
                    );
                    slope_penalty = Math__ftol(slope_penalty);
                }
                extra = 0.001 if (edge.byte4 != 0) else 0.0;  // bridge/special edge penalty
                candidate_cost = edge_cost_base + current.g + slope_penalty + extra;

                passable = (&g_PassabilityMatrix)[param_4 * 8 + edge.cost_class] == 1;
                not_visited = (visited_array[neighbor_zone] != epoch)
                              || (candidate_cost < cost_array[neighbor_zone]);
                zone_filter_ok = (local_38 == 2)     // outermost level: no filter
                              || (parent_visited[edge.zone_cross] == epoch)
                              || (edge.cost_class == 1);  // bridge edge: always allowed

                if (passable && not_visited && zone_filter_ok) {
                    // Check invalidated-zone cache (deduplicate)
                    if (not already in invalidated cache) {
                        push to heap;
                        mark neighbor visited;
                        record cost;
                    }
                }
        }
        if (heap empty without finding dst_zone):
            return 0;   // unreachable
    }
    local_38--;
} while (local_38 >= 0);

return 1;   // all three levels succeeded
```

### Zone chain recording

When `dst_zone` is found, the function walks the parent-chain links back from
the destination zone node to the root and writes:
- `Pathfinder+0xBC + level * 1000`: array of zone IDs along the path (as `short`),
  stored from tail to head (reversed walk)
- `Pathfinder+0xC74 + level * 4`: zone chain length + 1

These arrays are the zone corridor used by `AStar_main_loop` to gate which cells
are allowed to be expanded (the level-0 zone marker check at `Pathfinder+0x40`).

### Heap used

The secondary heap at `Pathfinder+0x68` (same struct as the primary heap but
separate slot) is used for zone-level search. It is reset at the start of each
hierarchy level iteration (entries zeroed, count = 0).

### Edge table layout (8 bytes per entry, inferred from decompile)

| Offset | Type | Name | Notes |
|--------|------|------|-------|
| `+0x00` | `uint` | `neighbor_zone` | Neighbor zone index |
| `+0x04` | `uint` | `zone_cross` (high) + flags (low) | byte at +4: if nonzero → +0.001 penalty |
| `+0x18` | `ushort` | zone index into cross table | used for zone_filter_ok check |
| `+0x1C` | `int` | `cost_class` | Index into `DAT_007e3794` edge cost table |

Edge table base: `DAT_0087F878 + level * 0x18` (pointer to zone graph for each level).
Each entry stride: `0x24` bytes (from `uVar16 * 0x24` indexing in decompile).

### Passability gate

```c
(&g_PassabilityMatrix)[param_4 * 8 + edge.cost_class] == 1
```

`g_PassabilityMatrix` is an 8×N matrix indexed by `[SpeedType][edge_cost_class]`.
`param_4` = SpeedType passed from the search. This is the same matrix consulted in
`AStar_main_loop` for cell-level passability.

### Slope cost penalty (0.001 bridge extra)

When `edge.byte4 != 0`, 0.001 is added to the candidate cost:
- Verified via `read_memory 0x007E3818 8` → `fc a9 f1 d2 4d 62 50 3f`
  = IEEE-754 double ≈ 0.001002 (not exactly 0.001 — see YELLOW).

## PathfinderClass fields accessed

| Byte offset | Name | Access |
|-------------|------|--------|
| `+0x28` | `search_epoch` | Stamp for visited arrays |
| `+0x40 + level*4` | `zone_visited_array[level]` | Epoch-stamped visited marker array |
| `+0x44 + level*4` | `zone_parent_visited[level]` | Parent zone visited array |
| `+0x4C + level*4` | `zone_chosen_path[level]` | Chosen-path marker array |
| `+0x58 + level*4` | `zone_cost_array[level]` | g-cost array for zone search |
| `+0x68` | `secondary_heap` | Zone search open list |
| `+0x64` | `node_pool` (inferred via `param_1 + 100`) | Zone node allocation |
| `+0x78 + level*4` | `invalidated_zone_cache` | Dedup cache for zone edges |
| `+0x84 + level*4` | `invalidated_zone_cache_count` | Count of cached entries |
| `+0xBC + level*1000` | `zone_chain[level][]` | Short array: zone IDs along found path |
| `+0xC74 + level*4` | `zone_chain_len[level]` | Length of `zone_chain` (steps + 1) |

## Self-proof (3 claims re-verified)

**Claim 1:** Callers are `AStar_pathfind_search @ 0x0042C900` and `FUN_0042D170 @ 0x0042D170`.
Verified via `get_function_callers 0x0042C290` → exactly those two.

**Claim 2:** 0.001 bridge edge penalty constant is at `0x007E3818`.
Verified via `read_memory 0x007E3818 8` → `fc a9 f1 d2 4d 62 50 3f`
= double ≈ 0.001002 (Ghidra comment says "0.001"; actual value is 0.001002 — see YELLOW).

**Claim 3:** `MinHeap__SiftDown @ 0x0042DCA0` is called during zone-heap pop.
Verified via `get_function_callees 0x0042C290` → result includes
`MinHeap__SiftDown @ 0042dca0`.

## Globals referenced

| Global | Address | Role |
|--------|---------|------|
| `DAT_0087F858` | `0x0087F858` | Zone index table; stride 10 bytes per entry, level offset `level*2` |
| `DAT_0087F878` | `0x0087F878` | Zone edge graph base; stride `level * 0x18` per level |
| `DAT_007E3794` | `0x007E3794` | Float array: edge cost per cost_class |
| `DAT_007E3818` | `0x007E3818` | Bridge-edge extra penalty ≈ 0.001002 |
| `DAT_007E3810` | `0x007E3810` | Slope-check enable gate = 1e-5 |
| `g_PassabilityMatrix` | inline symbol | SpeedType × cost_class passability table |

## Control flow summary

```
Zone_precheck(pathfinder, src_coord, dst_coord, speed_type, foot)
├── Init slope check: bVar11, local_30 = FootClass+0x21C
├── For level in [2, 1, 0]:
│   ├── Reset secondary_heap (Pathfinder+0x68)
│   ├── src_zone = ZoneMap__CellToZoneIndex(src_coord)[level]
│   ├── dst_zone = ZoneMap__CellToZoneIndex(dst_coord)[level]
│   ├── Stamp src_zone and dst_zone as visited
│   ├── If src_zone == dst_zone:
│   │   └── Record trivially; continue to next level
│   └── Else zone-level A*:
│       ├── Push src_zone to heap (g=0)
│       ├── While heap not empty:
│       │   ├── Pop node
│       │   ├── If node == dst_zone → record chain; break
│       │   └── For each edge of node's zone:
│       │       ├── cost = base_cost + g + slope_penalty + bridge_extra
│       │       ├── Check: passable + not_visited + zone_filter_ok + dedup
│       │       └── Push neighbor to heap if all pass
│       └── If no path found → return 0
├── All levels succeeded → return 1
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `AStar_pathfind_search` | `0x0042C900` | task #3 (completed) |
| `FUN_0042D170` | `0x0042D170` | task #20 (pending) |
| `MinHeap__SiftDown` | `0x0042DCA0` | task #19 (pending) |
| `Zone_Estimate_Slope_Cost` | `0x00585F40` | out of pathfinding task list |
| `ZoneMap__CellToZoneIndex` | `0x0056D3F0` | out of pathfinding task list |
| `FootClass__Get_Slope_Speed_Factor` | `0x004DC760` | out of pathfinding task list |

## YELLOW — Unverified

- `0x007E3818` exact value: `read_memory` returns `fc a9 f1 d2 4d 62 50 3f` =
  IEEE-754 double = `0.0010019...`. Ghidra comment says "0.001" — the difference
  is ~0.002% and unlikely to matter in practice, but the exact binary value is
  0.001002 not 0.001.
- `DAT_0087F858` and `DAT_0087F878` layout: entry strides and field offsets are
  inferred from the decompile arithmetic (`iVar15 * 10 + level * 2`, `uVar16 * 0x24`)
  but not independently read via `read_memory` at runtime.
- Zone node pool allocation: the decompile uses `*(int *)(param_1 + 100)` (= `+0x64`)
  as the node pool pointer for zone nodes. This is distinct from the cell-level pool
  at `+0x0C` / `+0x10`. The exact struct of zone nodes (16 bytes: `parent_idx`, `zone_id`,
  `f_cost`, `depth`) is inferred from field access patterns in the decompile, not from
  a separately verified pool struct layout.
- `g_PassabilityMatrix` absolute address: referenced as a symbol in the decompile;
  base address not read via `read_memory` in this session.
