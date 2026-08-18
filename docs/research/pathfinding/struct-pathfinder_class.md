# PathfinderClass Struct — Decode Doc
**Derived from:** `PathfinderClass__Constructor @ 0x0042A6D0` + `PathfinderClass__Reset @ 0x0042A5B0` + field accesses across all decoded pathfinding functions.
**Status:** Derived from decompilation; struct not yet defined in Ghidra.

## Summary

PathfinderClass holds all state for a single A* pathfinding search: closed/open set arrays, a node pool heap, per-level exclusion vectors, zone path arrays, search-control flags, and a generation epoch counter. The struct is allocated once per `FootClass` (as an embedded field or heap pointer) and reused across searches via `Reset`.

All offsets below are **byte offsets** from the start of the struct.

---

## Field Table

| Offset | Size | Type | Proposed Name | Value / Notes | Evidence |
|--------|------|------|--------------|---------------|---------|
| `+0x00` | 1 | `bool` | `master_enable_lo` | Set to 0 in constructor | `*param_1 = 0` — `decompile_function 0x0042A6D0` |
| `+0x01` | 1 | `bool` | `field_01` | Set to 0 in constructor | `param_1[1] = 0` |
| `+0x02` | 1 | `bool` | `field_02` | Set to 0 in constructor | `param_1[2] = 0` |
| `+0x03` | 1 | `byte` | `field_03` | Set to 1 in constructor | `param_1[3] = 1` |
| `+0x04` | 4 | `float` | `cost_multiplier` | `0x3F800000` = 1.0f | `*(undefined4*)(param_1+4) = 0x3f800000` — constructor |
| `+0x08` | 1 | `bool` | `master_enable` | Set to 1 in constructor | `param_1[8] = 1` — constructor |
| `+0x0C` | 4 | `void*` | `bridge_closed_list` | Ptr to 0x180004-byte block; sentinel at +0x180000 | `*(void**)(param_1+0xC)` — constructor; Reset: `*(param_1+0xC + 0x180000) = 0` — `decompile_function 0x0042A5B0` |
| `+0x10` | 4 | `void*` | `ground_closed_list` | Ptr to 0x100004-byte block; sentinel at +0x100000 | `*(void**)(param_1+0x10)` — constructor; Reset: `*(param_1+0x10 + 0x100000) = 0` |
| `+0x14` | 4 | `AStarHeap*` | `node_pool_heap` | Ptr to 0x14-byte heap struct (node pool); `*piVar3 = 0` on reset | `*(void**)(param_1+0x14)` — constructor; Reset: `piVar6 = *(int**)(param_1+0x14); *piVar6 = 0` |
| `+0x18` | 4 | `void*` | `closed_set_array_a` | Ptr to array cleared on epoch reset | Reset: `*(param_1+0x18 + idx*4)` zeroed — `decompile_function 0x0042A5B0` |
| `+0x1C` | 4 | `void*` | `closed_set_array_b` | Ptr to array cleared on epoch reset | Reset: `*(param_1+0x1C + idx*4)` zeroed |
| `+0x20` | 4 | `int` | `field_20` | Set to 0 in constructor | `*(undefined4*)(param_1+0x20) = 0` |
| `+0x24` | 4 | `int` | `field_24` | Set to 0 in constructor | `*(undefined4*)(param_1+0x24) = 0` |
| `+0x28` | 4 | `uint` | `epoch` | Init 0xFFFFFFFF; incremented by Reset each call; wraps around at 0 (triggers full cleared-array sweep) | Constructor: `0xffffffff`; Reset: `iVar5 = *(param_1+0x28)+1; *(param_1+0x28) = iVar5` — `decompile_function 0x0042A5B0` |
| `+0x2C` | 4 | `uint` | `field_2C` | Init 0xFFFFFFFF | Constructor |
| `+0x30` | 4 | `int` | `dest_level` | Destination cell height level (for bridge-aware A*) | Referenced in `fn-astar_pathfind_search.md` as `Pathfinder+0x30` |
| `+0x34` | 4 | `int` | `src_level` | Source cell height level | Referenced in `fn-astar_pathfind_search.md` as `Pathfinder+0x34` |
| `+0x38` | 1 | `bool` | `hierarchy_valid` | 1 = hierarchy path OK; cleared by `InvalidateZoneEdge` and `UpdateHierarchicalEdges` when no edge found; read by `AStar_pathfind_search` retry loop | Constructor: `param_1[0x38] = 1`; `fn-pathfinder_invalidate_zone_edge.md`: `*(param_1+0x38) = 0` |
| `+0x3C` | 4 | `int` | `hs_capable_flag` | HS (hierarchical search) capable + urgency flag | Constructor: `0`; `fn-astar_pathfind_search.md`: `Pathfinder+0x3C` |
| `+0x40` | 4 | `float` | `cost_scale` | Cost scaling factor for retry (not confirmed in constructor; referenced from `astar_compute_edge_cost`) | `fn-astar_compute_edge_cost.md` references `Pathfinder+0x40` — YELLOW |
| `+0x44..+0x4B` | 8 | `?` | `field_44_4B` | Unknown; not observed in constructor | YELLOW |
| `+0x4C` | 4 | `int` | `zone_level_count_0` | Per-level zone count (first of 3 per-level int fields, stride 4) | Constructor: `puVar1[-3] = 0` where `puVar1 = (undefined4*)(param_1+0x4C)` → `puVar1[-3]` at `+0x4C - 0xC` = `+0x40`... wait, need to recompute |
| `+0x64` | 4 | `void*` | `zone_node_pool` | Ptr to 160000-byte zone node pool | Constructor: `*(void**)(param_1+100)` = `*(void**)(param_1+0x64)` — `operator_new(160000)` |
| `+0x68` | 4 | `AStarHeap*` | `open_set_heap` | Ptr to 10000-element A* open-set min-heap | `*(void**)(param_1+0x68)` — constructor; Reset: `piVar6 = *(int**)(param_1+0x68); *piVar6 = 0` |
| `+0x6C` | 4 | `int` | `hs_path_index` | HS hierarchical path index; -1 = none | Constructor: `*(undefined4*)(param_1+0x6C) = 0xFFFFFFFF`; `fn-astar_pathfind_search.md`: `Pathfinder+0x6C` |
| `+0x70` | 2 | `short` | `field_70` | Set to 0 in constructor | `*(undefined2*)(param_1+0x70) = 0` |
| `+0x72` | 2 | `short` | `field_72` | Set to 0 in constructor | `*(undefined2*)(param_1+0x72) = 0` |
| `+0x74` | 24 | `PathfinderHeapVec` | `exclusion_vec[0]` | Level-0 exclusion vector (24 bytes = 6 undefined4 fields): vtable(+0), data_ptr(+4), capacity(+8), init_flag(+C), ownership(+D), count(+10), growth(+14) | Constructor: `puVar1 = (undefined4*)(param_1+0x74)`; loop `puVar1[5] = 10; puVar1[4] = 0; puVar1 += 6` — `decompile_function 0x0042A6D0` |
| `+0x8C` | 24 | `PathfinderHeapVec` | `exclusion_vec[1]` | Level-1 exclusion vector | Constructor loop iteration 2 |
| `+0xA4` | 24 | `PathfinderHeapVec` | `exclusion_vec[2]` | Level-2 exclusion vector | Constructor loop iteration 3 |
| `+0xBC` | 1000 | `ushort[500]` | `zone_path[0]` | Level-0 stored Zone_precheck path (500 ushort zone IDs); stride 1000 bytes per level | `fn-pathfinder_invalidate_zone_edge.md`: `(ushort*)(param_1+0xBC + level*1000)` |
| `+0x4A4` | 1000 | `ushort[500]` | `zone_path[1]` | Level-1 zone path | `+0xBC + 1*1000 = +0x4A4` |
| `+0x88C` | 1000 | `ushort[500]` | `zone_path[2]` | Level-2 zone path | `+0xBC + 2*1000 = +0x88C` |
| `+0xC74` | 4 | `int` | `zone_path_count[0]` | Level-0 zone path length | `fn-pathfinder_invalidate_zone_edge.md`: `*(int*)(param_1+0xC74 + level*4)` |
| `+0xC78` | 4 | `int` | `zone_path_count[1]` | Level-1 zone path length | `+0xC74 + 1*4 = +0xC78` |
| `+0xC7C` | 4 | `int` | `zone_path_count[2]` | Level-2 zone path length | `+0xC74 + 2*4 = +0xC7C` |

> Note: the zone path array layout (0xBC + level*1000) gives 3 paths of 500 shorts each (1000 bytes). This accounts for offsets 0xBC..0xC73. The zone_path_count fields at 0xC74..0xC7F follow immediately after.

---

## Per-Level Sub-Struct (3 levels, repeated at +0x74, +0x8C, +0xA4)

Each `PathfinderHeapVec` occupies 24 bytes (6 `undefined4` fields):

| Rel Offset | Field | Value in constructor |
|---|---|---|
| `+0x00` | vtable | `&PTR_FUN_007e37cc` |
| `+0x04` | data_ptr | 0 (null initially) |
| `+0x08` | capacity | 0 (from `FUN_0042DC50(0,0)` call) |
| `+0x0C` | init_flag | Set by `FUN_0042DC50` |
| `+0x0D` | ownership_flag | 0 |
| `+0x10` | count | `puVar1[4] = 0` → element count = 0 |
| `+0x14` | growth_increment | `puVar1[5] = 10` → grow by 10 on overflow |

Confirmed from constructor loop: `puVar1[5] = 10; puVar1[4] = 0; puVar1 = puVar1 + 6` — `decompile_function 0x0042A6D0`.

---

## AStarHeap Sub-Struct (at `param_1+0x14` and `param_1+0x68`)

From the constructor, two heap objects are allocated as 0x14-byte structs:

| Index | Field | Value | Notes |
|---|---|---|---|
| `[0]` = `+0x00` | size / head index | 0, then set to capacity | `*piVar3 = 0` on reset |
| `[1]` = `+0x04` | capacity | 0x10000 (65536) for node pool; 10000 for open-set | `puVar1[1] = 0x10000` / `10000` |
| `[2]` = `+0x08` | data_ptr | allocated buffer | `puVar1[2] = operator_new(...)` |
| `[3]` = `+0x0C` | field_3 | 0 | `puVar1[3] = 0` |
| `[4]` = `+0x10` | field_4 | -1 (0xFFFFFFFF) | `puVar1[4] = 0xffffffff` |

Confirmed from constructor: `puVar1[3] = 0; puVar1[4] = 0xffffffff; *puVar1 = 0; puVar1[1] = 0x10000` — `decompile_function 0x0042A6D0`.

---

## Struct Size Estimate

The highest confirmed offset is `+0xC7C` (zone_path_count[2], 4 bytes) = minimum size `0xC80` bytes (3200 bytes). The `operator_new(160000)` zone node pool is stored at `+0x64` as a pointer, not inline.

---

## Self-Proof (3 Claims Verified This Session)

1. **`epoch` field at `+0x28`: initialized to `0xFFFFFFFF`, incremented by Reset** — confirmed from constructor: `*(undefined4*)(param_1+0x28) = 0xffffffff`; and from Reset decompile: `iVar5 = *(int*)(param_1+0x28) + 1; *(int*)(param_1+0x28) = iVar5` — epoch wrap at 0 triggers full closed-set sweep. Verified via `decompile_function 0x0042A6D0` and `decompile_function 0x0042A5B0`.

2. **Exclusion vectors at `+0x74`, `+0x8C`, `+0xA4` (3 × 24 bytes = `PathfinderHeapVec` structs)**  — confirmed from constructor: `puVar1 = (undefined4*)(param_1+0x74)`; loop 3 times, each advancing `puVar1 += 6` (6 × 4 = 24 bytes). Growth increment initialized to 10 (`puVar1[5] = 10`), count to 0 (`puVar1[4] = 0`). Verified via `decompile_function 0x0042A6D0`.

3. **Zone path arrays at `+0xBC + level*1000`, zone path counts at `+0xC74 + level*4`** — confirmed: constructor shows `local_8 = (undefined4*)(param_1+0xbc)` advancing by `0xfa` (= 250 undefined4 = 1000 bytes) per iteration for 3 iterations (covering levels 0..2). Zone path count access pattern `*(int*)(param_1+0xC74 + level*4)` confirmed from `fn-pathfinder_invalidate_zone_edge.md` cross-reference using fresh `decompile_function 0x0042CF80`. Verified via `decompile_function 0x0042A6D0`.

---

## YELLOW (Unverified / Gaps)

| Offset | Issue |
|---|---|
| `+0x30`, `+0x34` | `dest_level` / `src_level` — referenced in prior `astar_pathfind_search` doc but not explicitly set in constructor; may be initialized elsewhere |
| `+0x40..+0x4B` | 12 bytes between hierarchy_valid (0x38) and zone_level_count region (0x4C); not observed being set in constructor or Reset |
| `+0x4C..+0x63` | Per-level int fields referenced by Reset (zone count, ptr to count arrays); layout from Reset's `piVar6 = (int*)(param_1+0x4c)` loop not fully decoded |
| `+0x18`, `+0x1C` | Closed-set arrays A and B: size/layout not determined; only that they are ptrs to int arrays cleared on epoch=0 reset |
| `+0xBC..+0xC73` | 3000 bytes of zone path arrays; layout confirmed as `ushort[500]` × 3 levels |

---

## Proposed Ghidra Rename Table (v9.1 format)

Since PathfinderClass struct does not exist in Ghidra (`get_struct_layout` returns "Structure not found"), these are proposed field names for human implementation:

| Byte offset | Size | Proposed name | Type |
|---|---|---|---|
| 0x00 | 1 | `master_enable_lo` | `bool` |
| 0x04 | 4 | `cost_multiplier` | `float` |
| 0x08 | 1 | `master_enable` | `bool` |
| 0x0C | 4 | `bridge_closed_list` | `void*` |
| 0x10 | 4 | `ground_closed_list` | `void*` |
| 0x14 | 4 | `node_pool_heap` | `AStarHeap*` |
| 0x18 | 4 | `closed_set_array_a` | `void*` |
| 0x1C | 4 | `closed_set_array_b` | `void*` |
| 0x28 | 4 | `epoch` | `uint` |
| 0x2C | 4 | `field_2C` | `uint` |
| 0x38 | 1 | `hierarchy_valid` | `bool` |
| 0x3C | 4 | `hs_capable_flag` | `int` |
| 0x64 | 4 | `zone_node_pool` | `void*` |
| 0x68 | 4 | `open_set_heap` | `AStarHeap*` |
| 0x6C | 4 | `hs_path_index` | `int` |
| 0x74 | 24 | `exclusion_vec[0]` | `PathfinderHeapVec` |
| 0x8C | 24 | `exclusion_vec[1]` | `PathfinderHeapVec` |
| 0xA4 | 24 | `exclusion_vec[2]` | `PathfinderHeapVec` |
| 0xBC | 1000 | `zone_path[0]` | `ushort[500]` |
| 0x4A4 | 1000 | `zone_path[1]` | `ushort[500]` |
| 0x88C | 1000 | `zone_path[2]` | `ushort[500]` |
| 0xC74 | 4 | `zone_path_count[0]` | `int` |
| 0xC78 | 4 | `zone_path_count[1]` | `int` |
| 0xC7C | 4 | `zone_path_count[2]` | `int` |
