# PathfinderClass — Ghidra Research Report

**Primary addresses:** Constructor `0x0042a6d0` (real C++ ctor, CRT static-init at boot), per-map array resize `0x0042ac00`, Reset `0x0042a5b0`, Zone alloc `0x0042c1c0`
**Singleton instance:** `g_PathfinderInstance @ 0x0087e8b8` (fixed static address; NOT embedded in MapClass)
**Confidence:** HIGH — all methods decompiled, struct fields traced through multiple functions
**Active in YR:** YES — core pathfinding infrastructure, used every time any unit pathfinds

## 1. Overview

PathfinderClass is the A* search context **singleton at fixed address `0x0087e8b8`**. It holds all per-search state: memory pools for nodes, open/closed set structures, cost arrays, hierarchical zone corridor data, and configuration flags. There is exactly one instance, reused across all pathfinding operations via stamp-based O(1) clearing.

The struct is approximately **0xC80 bytes** (3200 bytes), plus external heap allocations for the memory pools (~2.75 MB total, excluding the two heap structs and zone node pool). The C++ constructor `PathfinderClass__Constructor` at `0x0042a6d0` runs once at program startup via the CRT static-init thunk at `0x0040afa0` (`MOV ECX,0x87e8b8; CALL 0x0042a6d0`). The per-map array resize at `FUN_0042ac00` runs once per map load. (corrected 2026-06-01: constructor label and ctor xref verified via `decompile_function 0x0042a6d0` and `get_assembly_context 0x0040afa5` - RTTI_LABEL_DRIFT)

**Correction 2026-05-18:** The prior claim that PathfinderClass is "embedded within the MapClass singleton at MapClass+0xEC" was wrong. `MapClass+0xEC` is passed as `param_2` to `FUN_0042ac00` so the resize helper can read `MapClass+0xF4` (MapWidth) and `MapClass+0xF8` (MapHeight) — the helper's `this` pointer is still `0x0087e8b8`. See [PATHFINDERCLASS_COST_MULTIPLIER_GHIDRA_REPORT.md](PATHFINDERCLASS_COST_MULTIPLIER_GHIDRA_REPORT.md) §2 for the full caller-by-caller evidence.

## 2. Class Layout

### Location in Memory

```
PathfinderClass singleton @ 0x0087e8b8  (fixed static address)
  +0x04: cost_multiplier = 1.0f forever (hardcoded by constructor, see §10 Q2 resolution)
  +0x3c: per-pathfind urgency {0,1,2} (NOT bridge_mode — see table below)
  ... (struct body)

MapClass singleton (separately; actual MapClass offsets)
  +0x68: ptr CellZoneData          (4 bytes per cell: zone_type + cluster_id)
  +0x6C: int TotalCellCount        ((width+1+height)²)
  +0x70: ptr PerCellZoneIndex      (10 bytes per cell: 5 zone ID shorts)
  +0xA0: HierZoneLevelInfo[3]      (24 bytes each, zone subdivision sizes)
  +0xBC: ZoneEdgeList[3]           (24 bytes each, at MapClass level)
  +0xEC: resize-parameter view passed as param_2 to FUN_0042ac00; NOT PathfinderClass
         +0x08 from this view = MapWidth  (actual MapClass+0xF4)
         +0x0C from this view = MapHeight (actual MapClass+0xF8)
  +0xF4: int MapWidth
  +0xF8: int MapHeight
  ...
```

**Correction 2026-06-01:** The old table placed `MapWidth`/`MapHeight` at `MapClass+0x08/+0x0C`. The binary caller uses `LEA ECX,[ESI+0xEC]`, pushes that as `param_2`, sets `ECX=0x87e8b8`, then calls `0x0042ac00`; `FUN_0042ac00` reads `param_2+0x08/+0x0C`, so the actual MapClass fields are `+0xF4/+0xF8`. ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT. Evidence: `get_assembly_context 0x005671a5`, `decompile_function 0x0042ac00`, `decompile_function 0x00567110`.

### PathfinderClass Struct Layout (all offsets relative to PathfinderClass base = `0x0087e8b8`)

**param_1 type in all methods:** `int` — all offsets are direct byte offsets.

| Offset | Size | Type | Field | Evidence |
|--------|------|------|-------|----------|
| +0x00 | 1 | byte | unknown_byte_0 | Not yet identified |
| +0x01 | 1 | byte | bridge_aware | Checked in AStar_compute_edge_cost for bridge diagonal costs |
| +0x02 | 1 | byte | unknown_byte_02 | Constructor writes 0; no checked read found in audited PathfinderClass paths |
| +0x03 | 1 | byte | bridge_passability_update_enabled | Constructor writes 1; `PathfinderClass__UpdateBridgePassability` returns immediately if this byte is 0. (corrected 2026-06-01: was folded into padding; binary shows `param_1[3]=1` and read at `0x0042acf0` - OFFSET_RETYPED_WRONG) |
| +0x04 | 4 | float | cost_multiplier | **Hardcoded to 1.0f forever** by the C++ constructor at `0x0042a6d0` (`MOV dword ptr [ESI + 0x4], 0x3f800000`). No INI key, no per-locomotor source, no per-search override. AStar_main_loop multiplies edge cost by this, so the multiplier is a structural no-op. Rust port can omit this field entirely. See [PATHFINDERCLASS_COST_MULTIPLIER_GHIDRA_REPORT.md](PATHFINDERCLASS_COST_MULTIPLIER_GHIDRA_REPORT.md). |
| +0x08 | 1 | byte | unknown_flag_08 | Passed to Can_Enter_Cell vtable call as last param |
| +0x09 | 3 | | (padding) | |
| +0x0C | 4 | ptr | trail_pool | 12-byte trail nodes; counter at base+0x180000; ~131K entries |
| +0x10 | 4 | ptr | search_node_pool | 16-byte search nodes; counter at base+0x100000; ~65K entries |
| +0x14 | 4 | ptr | open_set_heap | Min-heap for A* open set, sorted by f-cost |
| +0x18 | 4 | ptr | closed_ground_stamp | Stamp array for ground-level closed set (map_width² × 4 bytes) |
| +0x1C | 4 | ptr | closed_bridge_stamp | Stamp array for bridge-level closed set (map_width² × 4 bytes) |
| +0x20 | 4 | ptr | g_cost_bridge | g-cost tracking for bridge level (map_width² × 4 bytes) |
| +0x24 | 4 | ptr | g_cost_ground | g-cost tracking for ground level (map_width² × 4 bytes) |
| +0x28 | 4 | int | current_stamp | Incremented each Reset(); used for O(1) closed-set clearing |
| +0x2C | 4 | int | speed_type | SpeedType of searching unit (from TechnoTypeClass+0x67C) |
| +0x30 | 4 | int | source_height | Current/source cell height level |
| +0x34 | 4 | int | dest_height | Destination cell height level |
| +0x38 | 1 | byte | search_valid | 1=search active, 0=invalidated (stops retry loop) |
| +0x39 | 3 | | (padding) | |
| +0x3C | 4 | uint | retry_urgency | Per-pathfind value `{0, 1, 2}`. 0 = first attempt (A* runs blocker-path prediction, code-2 cost = 1.0 or 4.0). 1 = retry-patient (skip prediction, code-2 cost = 4.0). 2 = retry-urgent (skip prediction, code-2 cost = **1000.0**, forces reroute around friendly blocker). Set by each locomotor based on `blocked_delay`. Prior label "bridge_mode" / "destroyer" semantics were wrong. See [PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md](PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md). |
| +0x40 | 4 | ptr | zone_closed_set[0] | Zone-level Dijkstra closed set, level 0 (fine) |
| +0x44 | 4 | ptr | zone_closed_set[1] | Zone-level Dijkstra closed set, level 1 (medium) |
| +0x48 | 4 | ptr | zone_closed_set[2] | Zone-level Dijkstra closed set, level 2 (coarse) |
| +0x4C | 4 | ptr | zone_g_cost[0] | Zone-level g-cost, level 0 |
| +0x50 | 4 | ptr | zone_g_cost[1] | Zone-level g-cost, level 1 |
| +0x54 | 4 | ptr | zone_g_cost[2] | Zone-level g-cost, level 2 |
| +0x58 | 4 | ptr | zone_parent[0] | Zone-level parent chain, level 0 |
| +0x5C | 4 | ptr | zone_parent[1] | Zone-level parent chain, level 1 |
| +0x60 | 4 | ptr | zone_parent[2] | Zone-level parent chain, level 2 |
| +0x64 | 4 | ptr | zone_node_pool | Pre-allocated zone Dijkstra nodes (16 bytes each) |
| +0x68 | 4 | ptr | zone_precheck_heap | Min-heap for zone-level Dijkstra search |
| +0x6C | 4 | int | corridor_index | Current position in hierarchical corridor path |
| +0x70 | 4 | CellStruct | corridor_cell | Current corridor cell (packed short x, short y) |
| +0x74 | 24 | struct | edge_inval_list[0] | Zone edge invalidation list, level 0 |
| +0x8C | 24 | struct | edge_inval_list[1] | Zone edge invalidation list, level 1 |
| +0xA4 | 24 | struct | edge_inval_list[2] | Zone edge invalidation list, level 2 |
| +0xBC | 1000 | ushort[500] | hier_path[0] | Zone corridor path, level 0 (zone IDs) |
| +0x4A4 | 1000 | ushort[500] | hier_path[1] | Zone corridor path, level 1 |
| +0x88C | 1000 | ushort[500] | hier_path[2] | Zone corridor path, level 2 |
| +0xC74 | 4 | int | hier_path_len[0] | Number of zones in corridor, level 0 |
| +0xC78 | 4 | int | hier_path_len[1] | Number of zones in corridor, level 1 |
| +0xC7C | 4 | int | hier_path_len[2] | Number of zones in corridor, level 2 |

**Total struct size: ~0xC80 bytes (3200 bytes)**

### Edge Invalidation List Structure (24 bytes each)

| Offset | Type | Field |
|--------|------|-------|
| +0x00 | ptr | vtable_or_func_ptr (has method at +0x0C for clearing) |
| +0x04 | ptr | data_array (array of packed zone-pair uint32s) |
| +0x08 | int | vector allocation/capacity metadata, not the live count |
| +0x0C | int | vector allocation/growth metadata |
| +0x10 | int | live count / write_index used by exclusion scans |
| +0x14 | int | growth/capacity hint initialized to 10 |

(corrected 2026-06-01: prior row put count at `+0x08`; binary scans invalidation entries using base `+0x10` as count and base `+0x04` as data via `decompile_function 0x0042c290`, while constructor initializes base `+0x14=10` and base `+0x10=0` via `decompile_function 0x0042a6d0` - OFFSET_RETYPED_WRONG)

Each entry in the data array is a packed `uint32`: `(zone_a << 16) | zone_b` where
`zone_a < zone_b` (canonicalized).

---

## 3. Memory Pools

### Trail Node Pool (+0x0C)

Stores parent-chain nodes for path reconstruction.

**Allocation:** Single contiguous block of `0x180004` bytes (1,572,868 bytes ≈ 1.5 MB).

**Node layout (12 bytes each):**

| Offset | Type | Field |
|--------|------|-------|
| +0x00 | ptr | cell_slot_ptr (`CellClass**` slot in CellArray, not a direct CellClass pointer) |
| +0x04 | int | height_level (bridge level = base + 4) |
| +0x08 | ptr | parent_trail_ptr (previous trail node pointer, not a pool index) |

(corrected 2026-06-01: was documented as direct `CellClass`/parent index; `AStar_create_node` stores the CellArray slot pointer at trail+0 and the previous node's trail pointer at trail+8 via `decompile_function 0x0042a460` - OFFSET_RETYPED_WRONG)

**Counter at pool+0x180000:** Tracks next free index. Reset to 0 in PathfinderClass__Reset.
**Max entries:** 0x180000 / 12 = **131,072 trail nodes**.

### Search Node Pool (+0x10)

Stores open-set A* nodes.

**Allocation:** Single contiguous block of `0x100004` bytes (1,048,580 bytes ≈ 1 MB).

**Node layout (16 bytes each):**

| Offset | Type | Field |
|--------|------|-------|
| +0x00 | ptr | trail_node (pointer into trail pool) |
| +0x04 | float | g_cost (accumulated path cost) |
| +0x08 | float | f_cost (g_cost + heuristic) |
| +0x0C | int | depth (step count from start) |

**Counter at pool+0x100000:** Tracks next free index. Reset to 0 in PathfinderClass__Reset.
**Max entries:** 0x100000 / 16 = **65,536 search nodes** (matches MAX_SEARCH_NODES = 0xFFF7 = 65,527).

### Zone Dijkstra Node Pool (+0x64)

Pre-allocated 16-byte nodes for the hierarchical zone search. Shared across all 3 levels.

**Allocation:** The constructor allocates `160000` bytes at `+0x64`, i.e. **10,000 16-byte zone nodes**. (corrected 2026-06-01: prior open question said size/allocation not determined; binary shows `operator_new(160000)` then store to `param_1+100` via `decompile_function 0x0042a6d0` - STALE)

---

## 4. Closed Set: Stamp-Based O(1) Clearing

The closed set uses a **stamp-based** approach to avoid clearing the entire array between
searches. This is critical for performance since the arrays are `map_width²` entries.

**How it works:**
1. `current_stamp` (+0x28) is incremented at the start of each search (in Reset)
2. To mark a cell as visited: `closed_set[cell_index] = current_stamp`
3. To check if visited: `closed_set[cell_index] == current_stamp`
4. Cells from previous searches have old stamp values → automatically "cleared"

**Dual layers:** Ground (`+0x18`) and bridge (`+0x1C`) have separate stamp arrays.
A cell's layer is determined by height:
- `abs(cell_height - search_height) < 2` → ground level
- Otherwise → bridge level (height = base + 4)

**Stamp overflow:** If stamp wraps to 0 (after ~4 billion searches), Reset performs a
full O(n) clear of the two per-cell stamp arrays (`+0x18`, `+0x1C`) and the hierarchical
zone arrays. It does **not** bulk-clear the per-cell g-cost arrays (`+0x20`, `+0x24`);
their entries are only meaningful when the matching stamp array equals `current_stamp`.
(corrected 2026-06-01: was "stamp and cost arrays"; binary clears `+0x18/+0x1C` plus zone arrays only via `decompile_function 0x0042a5b0` - OPERATOR_OR_ORDER_DRIFT)
This is not a rare event confined to 4-billion-search overflow: the constructor
initializes `current_stamp` (+0x28) to `0xffffffff`, so the very first
`PathfinderClass__Reset()` call of the process (`-1 + 1 == 0`) always takes this branch
once, deterministically, near program start. `ResizeMapArrays` (`0x0042ac00`, run on every
map load) does not reset `current_stamp`, so later map loads do not retrigger it; true
32-bit wraparound remains the only other trigger.
(corrected 2026-07-12: was "in practice this never happens during normal gameplay";
binary shows the wrap branch fires unconditionally on the process's first Reset() call
via `decompile_function 0x0042a6d0` and `decompile_function 0x0042a5b0` - INFERENCE_HARDENED)

---

## 5. Lifecycle

### Initialization (map load)

1. **FUN_00567110** (MapClass::InitZoneMap): Called during map loading
   - Allocates cell zone data arrays (MapClass+0x68, +0x70)
   - Calls **FUN_0042ac00** (PathfinderClass per-map array resize) with `this = 0x0087e8b8` and `param_2 = MapClass+0xEC` (corrected 2026-06-01: was "constructor" with `this=MapClass+0xEC`; binary caller sets `ECX=0x87e8b8` before `CALL 0x0042ac00` - GHIDRA_ADDRESS_SHIFT)
     - Computes `map_width = MapWidth + 1 + MapHeight` → stored in global `DAT_0089c2dc`
     - Allocates 4 arrays of `map_width² × 4` bytes: ground stamp, bridge stamp, ground g-cost, bridge g-cost
     - **Sets up dynamic direction offset table** at `0x0089a304`:
       ```
       [0] = -map_width      (N)
       [1] = 1 - map_width   (NE)
       [2] = 1               (E)
       [3] = map_width + 1   (SE)
       [4] = map_width        (S)
       [5] = map_width - 1   (SW)
       [6] = -1              (W)
       [7] = -1 - map_width  (NW)
       ```
       These are used by AStar_main_loop for per-map linear stamp/g-cost indices. Compass neighbor CellArray slot lookup uses the fixed CellArray neighbor-offset table separately. (corrected 2026-06-01: was described as CellArray indexing; binary uses `g_CellNeighborOffsets_8Dir` for CellArray neighbor slots and `0x0089a304` for stamp/g-cost linear indices via `decompile_function 0x00429a90` - MISLEADING)
   - Calls **FUN_0042c1c0** (PathfinderClass::AllocZoneArrays)
     - Allocates 9 zone-level arrays (3 levels × {closed, g_cost, parent}) sized by zone count
   - Computes bridge zones and floods zone map

### Per-Search Reset

**PathfinderClass__Reset (0x42a5b0):**
1. Reset trail pool counter to 0 (`pool+0x180000 = 0`)
2. Reset search node pool counter to 0 (`pool+0x100000 = 0`)
3. Clear open-set heap (set all entries to 0, count to 0)
4. Clear zone-precheck heap (same)
5. Increment `current_stamp` (+0x28) — O(1) closed-set "clearing"
6. If stamp wrapped to 0: perform full O(n) clear of the two stamp arrays plus zone closed/g-cost/parent arrays; per-cell g-cost arrays are not bulk-cleared (corrected 2026-06-01: binary shows no `+0x20/+0x24` clear in the wrap branch via `decompile_function 0x0042a5b0` - OPERATOR_OR_ORDER_DRIFT)

### Per-Search Configuration

**AStar_pathfind_search (0x42c900):**
1. Sets `search_valid` (+0x38) = 1
2. Calls PathfinderClass__Reset()
3. Clears 3 edge invalidation lists (vtable call at +0x0C on each)
4. Sets `retry_urgency` (+0x3C) from caller parameter (see §2 — prior "bridge_mode" label was wrong; verified via `decompile_function 0x0042c900` showing `*(uint *)(param_1 + 0x3c) = param_8`)
5. Looks up source/dest zone IDs via MapClass::GetZoneID
6. Runs Zone_precheck if zones match and bridge-aware
7. Enters retry loop: AStar_main_loop → on failure → UpdateHierarchicalEdges → Reset → retry
8. Max retries: 5 (or 1 if max_iterations specified)

### Zone Reallocation (terrain change)

When terrain changes (overlay destroyed, building placed, etc.):
- **FUN_00581f50** or **FUN_00584550** clear edge lists and call FUN_0042c1c0 to
  reallocate zone arrays sized to new zone counts

---

## 6. Core Logic

### A* Search Flow (AStar_main_loop uses PathfinderClass as `this`)

```
per iteration:
    pop min f-cost node from open_set_heap (+0x14)

    if node.cell == goal AND node.height == dest_height (+0x34):
        → reconstruct path, smooth, return

    for each of 8 compass neighbors, then optional direction-8 tube jump:
        compass dirs 0-7: get neighbor CellArray slot via fixed 8-dir table;
        direction 8: follow Cell+0x116 TubeIndex through g_TubeArray when present

        determine layer (ground vs bridge) from height comparison:
            ground: use closed_ground (+0x18), g_cost_ground (+0x24)
            bridge: use closed_bridge (+0x1C), g_cost_bridge (+0x20)

        if closed_set[cell] == current_stamp (+0x28):
            skip (already visited)

        check zone corridor:
            zone_id must match hier_path[corridor_index] (+0x6C, +0xBE area)
            → keeps search within zone corridor from precheck

        call Can_Enter_Cell (vtable+0x1ac) → returns zone_type (0-7)

        if zone_type < 7:  (passable)
            edge_cost = AStar_compute_edge_cost(zone_type)
            total_g = edge_cost * cost_multiplier (+0x04) + tiebreaker[dir] for dirs 0-7;
            direction 8 uses tube-destination Chebyshev distance instead of 0x0081872c

            create node in search_node_pool (+0x10)
            create trail in trail_pool (+0x0C)
            push to open_set_heap (+0x14)
            mark closed_set[cell] = current_stamp
```

(corrected 2026-06-01: was "dirs 0-7 + bridge=8"; binary shows `iStack_44==8` is a TubeClass jump via `Cell+0x116`/`g_TubeArray`, not a bridge neighbor, and only compass dirs use the `0x0081872c` tie-breaker table via `decompile_function 0x00429a90` - TS_LEGACY_AS_YR)

### Hierarchical Zone Precheck (Zone_precheck at 0x42c290)

Before cell-level A*, runs a zone-level Dijkstra to verify connectivity:

```
for level = 2 downto 0:  (coarse to fine)
    clear zone_precheck_heap (+0x68)

    get source/dest zone IDs from zone map

    if same zone:
        corridor = [that zone]
    else:
        Dijkstra on zone adjacency graph:
            nodes from zone_node_pool (+0x64)
            open set via zone_precheck_heap (+0x68)
            closed via zone_closed_set[level] (+0x40/44/48)
            costs in zone_g_cost[level] (+0x4C/50/54)

            cost per edge = passability_matrix gate (0x0082a594) + edge-type cost table
            (0x007e3794) + accumulated parent cost + optional per-unit slope cost
            (Zone_Estimate_Slope_Cost, only when a technoclass is passed) + a diagonal-
            adjacency bonus; the heap priority is this accumulated cost alone — no
            heuristic-to-goal term is added, so this is uniform-cost (Dijkstra), not A*
            (corrected 2026-07-12: was "heuristic based on zone centroids"; no heuristic
            term found in `decompile_function 0x0042c290` — the value pushed to the heap
            is pure accumulated edge cost - INFERENCE_HARDENED)

        if no path: return false (unreachable)

        reconstruct zone corridor:
            walk parent chain → store zone IDs in hier_path[level]
            store length in hier_path_len[level]
```

The corridor constrains the cell-level A* to only explore cells within the zone corridor,
dramatically reducing search space for long-distance paths.

### Retry Loop (AStar_pathfind_search)

```
for retry = 0 to max_retries:
    result = AStar_main_loop(...)
    if result != 0: return result    (success)

    if not bridge_aware: return 0    (no retry without bridge awareness)

    UpdateHierarchicalEdges:
        for each of 3 levels:
            ZoneMap__FloodFillReachableZones (0x005840c0) flood-fills from corridor_cell
            (+0x70), staying inside cells that share corridor_cell's zone ID, then checks
            whether every same-zone-ID cell in the local search window was actually
            reached by the flood
            if a same-zone-ID cell was NOT reached (the zone-ID grouping no longer
            matches true connectivity around corridor_cell):
                InvalidateZoneEdge: remove broken edge from zone graph
            else:
                walk the zone's adjacency-graph entries and add newly discovered
                neighboring zone IDs to the invalidation/exclusion list
            (corrected 2026-07-12: was "if destination zone unreachable"; `decompile_function
            0x005840c0` shows FloodFillReachableZones takes no destination-cell parameter —
            it only compares flood-reached cells against corridor_cell's own zone ID and
            returns true when a same-zone cell is missed, i.e. a connectivity/zone-ID
            mismatch at the failure point, not a destination-reachability check -
            INFERENCE_HARDENED)

    PathfinderClass__Reset()

    if search_valid (+0x38) == 0: return 0  (invalidation killed search)

    re-run Zone_precheck with updated edges
    if precheck fails: return 0
```

---

## 7. INI Keys

| Key | Section | Default | PathfinderClass interaction |
|-----|---------|---------|---------------------------|
| PathDelay | [General] | 0.01 min | Not directly in PathfinderClass — throttles Find_Path calls |
| CloseEnough | [General] | 2.25 | Not in PathfinderClass — checked before pathfinding |

The PathfinderClass itself has no direct INI configuration. Its behavior is parameterized
by the searching unit's SpeedType, MovementZone, bridge flag, and the zone map state.

---

## 8. Integration Points

### Who creates it:
- **FUN_00567110** (MapClass::InitZoneMap) — during map loading
- **FUN_0042ac00** — allocates per-map stamp/g-cost arrays and direction offsets (PathfinderClass resize helper, not the constructor) (corrected 2026-06-01: current function body frees/reallocates `+0x18/+0x1C/+0x20/+0x24` and reads map dimensions from `param_2`; constructor is `0x0042a6d0` - RTTI_LABEL_DRIFT)
- **FUN_0042c1c0** — allocates zone-level arrays

### Who uses it:
- **AStar_pathfind_search (0x42c900)** — A* orchestrator (configures + runs searches)
- **AStar_main_loop (0x429a90)** — core A* search loop
- **AStar_create_node (0x42a460)** — allocates from pools
- **Zone_precheck (0x42c290)** — hierarchical zone Dijkstra
- **PathfinderClass__EstimateZoneCost (0x42d170)** — zone-corridor distance estimator (used by Find_Path, threat scans)
- **PathfinderClass__Reset (0x42a5b0)** — clears per-search state
- **PathfinderClass__UpdateBridgePassability (0x42acf0)** — toggles 0x40000 flag on cells
- **PathfinderClass__UpdateHierarchicalEdges (0x42ccd0)** — retry mechanism
- **PathfinderClass__InvalidateZoneEdge (0x42cf80)** — removes broken zone edges

### When it runs in tick cycle:
PathfinderClass is used on-demand when units need paths. It's called from:
- `FootClass::Find_Path` (movement orders)
- `FootClass::Mission_Hunt` (attack approach)
- `FootClass::Greatest_Threat_Scan` (threat evaluation)
- Various building/unit placement functions

### Global data dependencies:
- `g_CellArray_Base` — cell pointer array
- `DAT_0089c2dc` — map linear width (set by per-map resize `0x0042ac00`)
- `0x0089a304` — dynamic direction offset table for per-map linear stamp/g-cost indices (set by `0x0042ac00`)
- `0x0087f858` — zone map per-cell data
- `0x0087f878` — zone adjacency graph data (per hierarchical level)
- `0x0082a594` — passability matrix (13×8)
- `0x0081870c` — A* cost table (8 floats by zone type)
- `0x0081872c` — direction tie-breaker offsets for the 8 compass directions; direction 8 tube handling bypasses this table (corrected 2026-06-01: was "9 floats"; binary reads this table only in the non-tube branch via `decompile_function 0x00429a90` - OPERATOR_OR_ORDER_DRIFT)

---

## 9. Current Rust Implementation Status

### What's implemented:
- **A* search with BinaryHeap** (`src/sim/pathfinding/core.rs`): Uses reusable thread-local
  `PathfindWorkspace` buffers (`Vec<i32>` g-cost, `Vec<bool>` closed, `BinaryHeap<Reverse<AStarNode>>` open set).
  It no longer allocates fresh full-map vectors per search, but it refills arrays rather than using gamemd's
  stamp-based closed-set clearing. (corrected 2026-06-01: source has `PATHFIND_WORKSPACE`; binary stamp reset via `decompile_function 0x0042a5b0` - STALE)
- **Zone-aware hierarchical search** (`src/sim/pathfinding/zone_search.rs`): Zone Dijkstra
  corridor with retry logic (`MAX_CORRIDOR_RETRIES=5`) — matches the binary default total attempt cap
  from `AStar_pathfind_search`. (corrected 2026-06-01: was 3; source and `decompile_function 0x0042c900` show 5 - STALE)
- **Layered bridge-aware A*** (`src/sim/pathfinding/core.rs:find_layered_path`): Dual
  ground/bridge state — matches original dual closed sets

### Gaps:
- **No exact persistent PathfinderClass singleton/stamp model**: Rust now reuses a thread-local
  workspace, but does not mirror `current_stamp` O(1) clearing or the exact singleton fields.
  The remaining mismatch is semantics/ordering risk around stamp-gated arrays, not fresh allocation.
  (corrected 2026-06-01: prior allocation-only gap was stale; source `PathfindWorkspace`, binary `current_stamp` via `decompile_function 0x0042a5b0` - STALE)
- **No dynamic direction offset table**: Rust uses hardcoded octile neighbor iteration.
  Original computes offsets from actual map dimensions at load time, stored at 0x0089a304.
- **No `cost_multiplier` field is required for parity if costs are otherwise exact**:
  binary initializes `+0x04` to `1.0f` in the constructor and no audited path overrides it.
  Treating omission as a behavior gap was stale. (corrected 2026-06-01: `decompile_function 0x0042a6d0` and `0x00429a90` - STALE)
- **`retry_urgency` is not `bridge_mode`**: the `+0x3C` field is the code-2 temporary-block
  urgency `{0,1,2}`. Current Rust has an `urgency` option and code-2 multipliers, but exact caller
  ordering still needs parity tests. (corrected 2026-06-01: was "bridge_mode/destroyer"; binary writes `param_8` at `0x0042c900` and reads it in `0x00429830` - RTTI_LABEL_DRIFT)
- **Zone edge invalidation / retry is partial**: Rust has a 5-attempt exclusion retry scaffold,
  but the exact `UpdateHierarchicalEdges` flood-fill producer and `InvalidateZoneEdge` path-neighbor
  exclusion semantics still need parity coverage. (corrected 2026-06-01: prior "not implemented"
  was stale; source `zone_search.rs` has `excluded_edges`/`MAX_CORRIDOR_RETRIES=5`, binary producer via `decompile_function 0x0042ccd0`/`0x0042cf80` - STALE)
- **3 hierarchical zone levels**: The original has 3 hierarchical levels with different
  subdivision granularity. The Rust implementation uses a single level.

---

## 10. Open Questions

1. **Field +0x00 (byte)**: Unknown purpose. Constructor writes 0 to this field
   (`*param_1 = 0;`); no read of it was found in any function decompiled this session
   (constructor, Reset, ResizeMapArrays, UpdateBridgePassability, AStar_pathfind_search,
   AStar_main_loop, AStar_create_node, Zone_precheck, EstimateZoneCost,
   UpdateHierarchicalEdges, AStar_compute_edge_cost). Could be a class ID, initialization
   flag, or always zero. **Confidence: LOW**.
   (corrected 2026-07-12: was "not seen written or checked in any decompiled function";
   binary shows the constructor writes 0 via `decompile_function 0x0042a6d0` - STALE)

2. **Resolved: Field +0x04 (cost_multiplier)**: The constructor writes `0x3f800000`
   (`1.0f`) and AStar multiplies by it, making it a structural no-op in audited YR paths.
   No INI or per-locomotor setter was found in the audited PathfinderClass entry paths.
   (corrected 2026-06-01: was open; verified by `decompile_function 0x0042a6d0`,
   `0x0042c900`, `0x0042a5b0`, and `0x00429a90` - STALE)

3. **Field +0x08 (byte)**: Passed as the last argument to Can_Enter_Cell. Likely a
   search-mode flag or bridge-awareness indicator. Constructor writes 1 unconditionally
   (`param_1[8] = 1;`); no other setter found in the PathfinderClass methods decompiled
   this session. **Confidence: LOW** (purpose still unconfirmed, but the initial value is
   now known).
   (corrected 2026-07-12: was "not set in decompiled PathfinderClass methods"; binary
   shows the constructor writes 1 via `decompile_function 0x0042a6d0` - STALE)

4. **Resolved: Trail pool and search node pool allocation**: Both are allocated by
   `PathfinderClass__Constructor` at `0x0042a6d0`: `operator_new(0x100004)` stored at
   `+0x10` and `operator_new(0x180004)` stored at `+0x0C`. (corrected 2026-06-01:
   was "allocation site not found"; binary shows both allocations via `decompile_function 0x0042a6d0` - STALE)

5. **Resolved: Open set heap allocation**: The constructor allocates heap structs at
   `+0x14` and `+0x68`; the A* heap backing array is `operator_new(0x40004)` with capacity
   `0x10000`, and the zone heap backing array is `operator_new(0x9c44)` with capacity `10000`.
   (corrected 2026-06-01: was "not traced"; verified via `decompile_function 0x0042a6d0` - STALE)

6. **Resolved: Zone node pool (+0x64)**: Size/allocation is `160000` bytes from the
   constructor, matching 10,000 16-byte Dijkstra nodes. (corrected 2026-06-01: was
   "not determined"; verified via `decompile_function 0x0042a6d0` - STALE)

7. **Resolved: `0x0042d170` identity**: Current verified role is
   `PathfinderClass__EstimateZoneCost`. It runs `Zone_precheck`, returns `0x7fffffff`
   on failure, and otherwise computes a Chebyshev/corridor/bridge-adjacent estimate
   without running cell-level A*. (corrected 2026-06-01: was "needs confirmation";
   verified via `decompile_function 0x0042d170` and `get_function_callers 0x0042d170` - STALE)

---

## Sources

### Ghidra addresses decompiled:
- `0x0042a6d0` — PathfinderClass__Constructor (initializes fields, heaps, pools, zone node pool)
- `0x0042ac00` — PathfinderClass per-map array resize (allocates stamp/g-cost arrays, sets direction table)
- `0x0042a5b0` — PathfinderClass__Reset (clear pools, increment stamp)
- `0x0042acf0` — PathfinderClass__UpdateBridgePassability (toggle 0x40000 on enemy paths)
- `0x0042cf80` — PathfinderClass__InvalidateZoneEdge (remove broken zone graph edge)
- `0x0042ccd0` — PathfinderClass__UpdateHierarchicalEdges (flood-fill retry mechanism)
- `0x0042c1c0` — PathfinderClass::AllocZoneArrays (allocate zone-level Dijkstra arrays)
- `0x0042c900` — AStar_pathfind_search (orchestrator with retry loop)
- `0x00429a90` — AStar_main_loop (core A* using PathfinderClass as `this`)
- `0x0042a460` — AStar_create_node (allocates from PathfinderClass pools)
- `0x0042c290` — Zone_precheck (hierarchical zone Dijkstra, 295 lines)
- `0x0042d170` — PathfinderClass__EstimateZoneCost (zone-corridor distance estimator)
- `0x00567110` — MapClass::InitZoneMap (calls PathfinderClass per-map resize)
- `0x004d3920` — FootClass::Find_Path (top-level pathfinding entry)

### Data tables referenced:
- `0x0089c2dc` — map linear width (set by per-map resize)
- `0x0089a304` — dynamic direction offset table (8 ints, set by per-map resize)
- `0x0087f858` — zone map per-cell cluster data
- `0x0087f878` — zone adjacency graph (per hierarchical level × 0x18 bytes)

### Related documents:
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` — A* algorithm, call hierarchy
- `PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT.md` — cost tables, cell utilities
- `ZONE_PASSABILITY_VERIFIED.md` — passability matrix
- `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` — zone map structure
