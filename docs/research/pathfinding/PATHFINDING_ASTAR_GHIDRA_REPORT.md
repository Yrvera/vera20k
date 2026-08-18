# Pathfinding A* System — Ghidra Decompilation Report

## Overview

This document covers the complete A* pathfinding pipeline in gamemd.exe, from the
high-level `FootClass::Find_Path` entry point down through the core A* search loop,
node expansion, cost functions, path reconstruction, smoothing, and the hierarchical
zone-map precheck system. All addresses reference gamemd.exe.

Confidence: **HIGH** — all functions fully decompiled, control flow traced, data
structures verified from memory dumps.

> **Corrections (2026-04-06, verified against binary):**
> 1. The "road-following" logic at 0x429830 was misidentified. It is actually
>    **friendly-unit-path-prediction** for Can_Enter_Cell code 2 (TemporaryBlock /
>    moving friendly). The 10-cell loop walks the blocking unit's predicted trajectory
>    to determine if the cell will clear.
> 2. The base cost table (`EdgeCostBaseTable` at 0x0081870c) is indexed by
>    **Can_Enter_Cell return codes (0-7)**, NOT by terrain/land type. Semantic labels
>    in the table have been corrected accordingly.
> 3. Can_Enter_Cell return code 2 is "TemporaryBlock (moving friendly)", not "Road".

---

## 1. Call Hierarchy

```
FootClass::Find_Path (0x4d3920)         — Entry point, called from DriveLocomotionClass
  ├─ FootClass::Find_Nearby_Passable_Cell (0x56dc20) — Redirect dest if blocked
  ├─ FootClass::Run_AStar (0x4cbba0)    — Wrapper that invokes the search
  │   ├─ Path_walk_directions_to_cell (0x429780) — Walk path_queue to get start cell
  │   └─ AStar_pathfind_search (0x42c900)        — Orchestrator with retry loop
  │       ├─ PathfinderClass__Reset (0x42a5b0)   — Clear open/closed sets
  │       ├─ Zone_precheck (0x42c290)            — Hierarchical zone-level precheck
  │       ├─ AStar_main_loop (0x429a90)          — THE core A* search
  │       │   ├─ AStar_create_node (0x42a460)    — Allocate + compute f(n) for node
  │       │   ├─ AStar_compute_edge_cost (0x429830) — g-cost for an edge
  │       │   ├─ PathfinderClass__UpdateBridgePassability (0x42acf0)
  │       │   ├─ Can_Enter_Cell (vtable+0x1ac)   — Movement cost for a cell
  │       │   └─ MinHeap__SiftDown (0x42dca0)    — Open set heap maintenance
  │       ├─ AStar_reconstruct_path (0x42aa90)   — Walk parent chain → direction array
  │       ├─ Path_smooth_corners (0x42b210)      — Remove unnecessary zigzags
  │       ├─ Path_optimize_straight_segments (0x42b7f0) — Straighten long runs
  │       └─ PathfinderClass__UpdateHierarchicalEdges (0x42ccd0)
  └─ (copies result into FootClass::path_queue at this+0x5E0)
```

---

## 2. FootClass::Find_Path — 0x4d3920

**Purpose:** Top-level pathfinding entry. Called when a unit needs a new path to its
destination. Handles special cases (aircraft destinations, building entry), then
delegates to the A* search, and writes the result into the 24-entry `path_queue`.

**Key fields on FootClass (this = ECX):**
- `this+0x5E0` (`this[0x178]`): `path_queue[24]` — array of 24 `int` direction entries
  (values 0-7 for compass directions, 8 for bridge crossing, -1 for end sentinel)
- `this+0x640` (`this[0x190]` / `this[400]`): `path_frame` — frame counter when path was last computed
- `this+0x644` (`this[0x191]`): path-related state
- `this+0x648` (`this[0x192]`): path distance
- `this+0x5D4` (`this[0x175]`): `is_aircraft` flag (non-zero for aircraft)
- `this+0x558` (`this[0x156]`): last waypoint cell coordinate
- `this+0x8C` (`this[0x23]`): `is_on_bridge` flag
- `this+0x6C8` (`this[0x1b2]`): pointer to next unit in team/follow chain

**Flow:**
1. Gets destination cell from virtual call at vtable+0x48
2. Computes straight-line distance to destination
3. Determines max search radius: uses `RulesClass+0x1718` (PathDelay rule) for ground
   units, or a flight-specific calculation for aircraft
4. Gets unit's `Can_Enter_Cell` result for destination (vtable+0x1ac)
5. **Special case — result == 6 (occupied by friendly):** If distance > some threshold
   and the unit is not an aircraft, calls `FootClass__Find_Nearby_Passable_Cell` to find
   an alternate unblocked destination nearby. Uses zone map validation.
6. **Special case — result == 7 (building entrance):** Looks up the building in the cell
   and redirects pathfinding to the building's entrance cell.
7. Calls virtual method at vtable+0x124 (likely `Set_Speed` or similar)
8. Calls `FootClass__Run_AStar` (0x4cbba0) to execute the search
9. If the result has entries, copies up to 24 direction entries from the result into
   `path_queue` at `this+0x5E0`
10. If the unit is in a team/follow chain (`this[0x1b2]` linked list), recursively
    pathfinds for followers
11. On failure: tries scatter/alternative movement

**Path queue format:** Each entry is an integer:
- 0-7: compass direction (N=0, NE=1, E=2, SE=3, S=4, SW=5, W=6, NW=7)
- 8: bridge crossing (look up bridge tunnel endpoint)
- -1 (0xFFFFFFFF): end sentinel
- -2 (0xFFFFFFFE): skip marker (used during path smoothing)

---

## 3. AStar_pathfind_search — 0x42c900 (Orchestrator)

**Purpose:** Sets up the pathfinder, runs zone prechecks, and calls the core A* loop
with a retry mechanism.

**Signature:**
```c
int __thiscall AStar_pathfind_search(
    PathfinderClass *this,     // param_1
    CellStruct *source,        // param_2 (short x, short y)
    CellStruct *dest,          // param_3
    FootClass *unit,           // param_4
    int param_5,               // -1 default
    int max_search_depth,      // param_6 (-1 = use default 0xFFF7 = 65527)
    uint zone_type,            // param_7 (zone passability/MovementZone index, 0xFFFFFFFF = auto-detect)
    uint flags                 // param_8
);
```

**Flow:**
1. Calls `PathfinderClass__Reset` (0x42a5b0) to clear open/closed sets
2. Clears 3 priority queues (the PathfinderClass has 3 hierarchical levels)
3. Gets source and destination CellClass pointers
4. Resolves the zone passability index: if `zone_type == -1`, reads from
   `TypeClass+0x5B4`, not the `TypeClass+0x67C` SpeedType field used later by
   cell passability. (corrected 2026-06-01: was "SpeedType"; binary shows
   `AStar_pathfind_search` reading `+0x5B4` for `MapClass__GetZoneID` /
   `Zone_precheck`, while `AStar_main_loop` separately stores `+0x67C` at
   `PathfinderClass+0x2C` via `decompile_function 0x0042C900` and
   `decompile_function 0x00429A90` - ROOT_CAUSE: STRUCT_FAMILY_CASCADE)
5. Gets zone IDs for source and dest cells via `MapClass__GetZoneID`
6. Resolves bridge-aware coordinates via `MapClass__ResolvePathCoord_BridgeAware` (0x583180)
7. **Zone ID comparison:** If source zone == dest zone, attempts the hierarchical
   precheck. If zones differ while hierarchical search is enabled, returns 0
   immediately; if hierarchy is disabled, the cell-level search can still run.
8. `Zone_precheck` (0x42c290): Runs a Dijkstra-style accumulated-cost search at
   each zone level. It uses no destination-distance heuristic. If this initial
   same-zone precheck fails, hierarchy is disabled and the unrestricted cell-level
   A* still runs. (corrected 2026-07-10: was "hierarchical A*" and implied initial
   precheck failure aborted the search; binary accumulates only prior cost, edge
   cost, optional slope cost, and a 0.001 edge tiebreaker in `Zone_precheck`, while
   `AStar_pathfind_search` clears the hierarchy flag and continues via
   `decompile_function 0x0042C290` and `decompile_function 0x0042C900` -
   ROOT_CAUSE: INFERENCE_HARDENED)
9. **Retry loop:** Calls `AStar_main_loop` up to `iStack_14` times:
   - `iStack_14 = 5` if `max_search_depth == -1` (no limit), else `1` (single attempt)
   - On failure: logs "Regular_findpath_failure", updates hierarchical edges via
     `PathfinderClass__UpdateHierarchicalEdges` (0x42ccd0), resets, and retries
   - On zone precheck failure: aborts retry

---

## 4. AStar_main_loop — 0x429a90 (Core A* Search)

**Purpose:** The actual A* search loop. 812 instructions, cyclomatic complexity 113.
This is the most important function in the pathfinding system.

**Signature:**
```c
int __thiscall AStar_main_loop(
    PathfinderClass *this,     // param_1
    CellStruct *source,        // param_2
    CellStruct *dest,          // param_3
    FootClass *unit,           // param_4
    int param_5,               // passed through to reconstruct
    int max_iterations,        // param_6 (default 0xFFF7 = 65527)
    bool hierarchical_ok       // param_7
);
```

### 4.1 Initialization

1. Gets CellClass pointers for source and dest from the global cell array:
   ```c
   cell = *(CellClass**)(g_CellArray_Base + (y * 0x200 + x) * 4)
   ```
   The map is stored as a flat array of CellClass pointers, width = 512 (0x200).

2. **Height level setup:** Reads the cell's height (`cell+0x11B`, a signed char).
   The binary does not use one simple "`unit_on_bridge || cell_bridge`" test:
   destination height adds 4 when the destination cell has flag `0x100` and the
   unit type virtual at `vtable+0x2C` is not code `2`; source height adds 4 when
   `FootClass+0x8C` is set and that same type code is not `2`, with an extra
   `TypeClass+0xC94`/height-delta bridge correction. (corrected 2026-06-01: was
   simplified to one bridge predicate; binary shows the split destination/source
   tests in `AStar_main_loop` via `decompile_function 0x00429A90` -
   ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
   ```
   this+0x34 = destination height level
   this+0x30 = source height level (updated as search progresses)
   ```

3. Stores the unit's SpeedType at `this+0x2C` (from `TypeClass+0x67C`).

4. Creates the start node via `AStar_create_node` and initializes the open set.

5. If bridge handling is enabled (`this+0x3C != 0`), calls
   `PathfinderClass__UpdateBridgePassability` to mark bridge cells in the passability map.

6. Marks the source cell in the height-appropriate closed/g-cost layer. Neighbor
   seeding only runs behind the `TypeClass+0xC94` branch; it is not unconditional.
   (corrected 2026-06-01: was unconditional neighbor seeding; binary shows the
   neighbor-mark loop only after `*(unit->Type()+0xC94) != 0` in
   `AStar_main_loop` via `decompile_function 0x00429A90` -
   ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

### 4.2 Main Loop — Node Expansion

The loop processes nodes from the open set (min-heap sorted by f-cost):

```
do {
    if (max_iterations <= iteration_count) break;

    current_node = open_set.top();  // piStack_48
    current_cell = current_node->cell;

    // Check if we reached the destination
    if (current_cell == dest_cell && current_node->height == dest_height) break;

    best_neighbor = NULL;

    // Expand all 9 neighbors (8 compass + 1 bridge)
    for (direction = 0; direction < 9; direction++) {
        // ... neighbor expansion (see below) ...
    }

    // Select next node from open set
    if (best_neighbor == NULL) {
        // Pop from heap
        next = heap_extract_min(open_set);
    } else {
        // Compare best_neighbor with heap top, use better one
        // ... heap operations ...
    }

    iteration_count++;
} while (current_node != NULL);
```

### 4.3 Neighbor Expansion — 9 Directions

For each direction 0-8:

**Directions 0-7 (compass):**
The neighbor cell is found via a precomputed offset table at `0x007e3774`:
```
Dir 0 (N):  offset -512  (y-1)
Dir 1 (NE): offset -511  (y-1, x+1)
Dir 2 (E):  offset +1    (x+1)
Dir 3 (SE): offset +513  (y+1, x+1)
Dir 4 (S):  offset +512  (y+1)
Dir 5 (SW): offset +511  (y+1, x-1)
Dir 6 (W):  offset -1    (x-1)
Dir 7 (NW): offset -513  (y-1, x-1)
```
These are indices into the CellClass pointer array (map width = 512).

**Direction 8 (bridge tunnel crossing):**
If the current cell has a tube record (`cell+0x116 != -1`), looks up the endpoint
through `g_TubeArray`: endpoint cell =
`*(int*)(g_TubeArray[cell->TubeIndex] + 0x28)`. (corrected 2026-06-01: was
"bridge record table / bridge_index"; binary shows direction 8 using
`g_TubeArray + *(short*)(cell+0x116)*4` in `AStar_main_loop` and
`Path_smooth_corners` via `decompile_function 0x00429A90` and
`decompile_function 0x0042B210` - ROOT_CAUSE: STRUCT_FAMILY_CASCADE)

### 4.4 Passability Check

For each neighbor, the following checks are performed:

1. **Height/layer compatibility (bridge awareness):**
   ```c
   if ((cell_flags & 0x100) == 0 || abs(current_height - neighbor_height) < 2)
       use_ground_layer = true;
   else
       use_ground_layer = false;  // bridge-layer closed/g-cost arrays
   ```
   This branch selects the ground-vs-bridge layer used for closed-set and cost-array
   checks; it is not an immediate `passable=false` reject. Actual rejection is still
   delegated to `Can_Enter_Cell` / locomotor passability. (corrected 2026-06-01: was
   "height gap too large -> passable=false"; binary shows the branch controlling the
   low-byte layer flag before the virtual passability call in `AStar_main_loop` via
   `decompile_function 0x00429A90` - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

2. **Closed set check:** Uses two parallel arrays (one for ground level, one for
   bridge level) indexed by a linearized cell coordinate:
   ```c
   cell_index = cell->MapCoord_Y * DAT_0089c2dc + cell->MapCoord_X;
   ```
   - `this+0x18`: ground-level closed set (stamp array)
   - `this+0x1C`: bridge-level closed set (stamp array)
   - `this+0x24/0x20`: ground/bridge g-cost arrays
   - `this+0x28`: current stamp value (incremented each search to avoid clearing)

   A cell is "in closed set" if `closed_set[cell_index] == current_stamp`.

3. **Fog of war check:** `cell+0x122` — if cell is **shrouded** (`cell+0x122 == 0`) and `param_7` is set,
   the neighbor is skipped (prevents pathfinding through unexplored cells in hierarchical mode).
   (corrected 2026-05-28: was "if cell is unshrouded … skip (allows pathfinding through fog)" — inverted;
   binary `AStar_main_loop` at `0x429a90` shows `if (*(char*)(iVar16+0x122)=='\0') && (param_7!='\0') goto skip` —
   condition fires when shrouded, not unshrouded — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

4. **Can_Enter_Cell virtual call (vtable+0x1AC):**
   ```c
   move_cost = unit->Can_Enter_Cell(neighbor_cell, direction, current_height, current_cell, bridge_flag);
   ```
   (corrected 2026-06-01: was `source_cell`; binary passes `*current_trail_node`,
   the current expanded cell, as the fourth stack argument in `AStar_main_loop` via
   `decompile_function 0x00429A90` - ROOT_CAUSE: PARAM1_TYPE_MISREAD)
   Returns an integer 0-7+:
   - 0: OK/Clear (cell is free)
   - 1: Crushable (civilian/neutral object)
   - 2: TemporaryBlock (moving friendly) — triggers path-prediction logic (see §4.5a)
   - 3: BridgeRamp / ScatterRequired (friendly stationary that can be bumped)
   - 4: FriendlyWall (allied wall overlay)
   - 5: EnemyBlock (enemy unit/building)
   - 6: FriendlyStationary (non-moving allied unit)
   - **7+: IMPASSABLE** — node is NOT expanded

   If the unit type flag at `TypeClass+0xC94` sets `bVar10` and `move_cost < 7`,
   the return code is forced to 0 before edge-cost computation. (corrected
   2026-06-01: was generalized as "aircraft"; binary gates this exact override on
   `*(unit->Type()+0xC94) != 0`, not `FootClass+0x5D4`, via
   `decompile_function 0x00429A90` - ROOT_CAUSE: INFERENCE_HARDENED)

### 4.5 Cost Function

**g-cost computation** — `AStar_compute_edge_cost` at 0x429830:

The edge cost from current to neighbor is:

```
base_cost = EdgeCostBaseTable[move_cost]    // at 0x0081870c
```

The `EdgeCostBaseTable` values (indexed by Can_Enter_Cell return code):
```
[0] = 1.0     OK/Clear — cell is free
[1] = 1000.0  Crushable — civilian/neutral object
[2] = 1.0     TemporaryBlock base (moving friendly) — overridden to 4.0/1000.0 by prediction logic (see §4.5a)
[3] = 1.0     BridgeRamp / ScatterRequired — friendly stationary that can be bumped
[4] = 60.0    FriendlyWall — allied wall overlay
[5] = 20.0    EnemyBlock — enemy unit/building
[6] = 8.0     FriendlyStationary — non-moving allied unit
[7] = 10000.0 Impassable — terrain/height/tunnel block (read for non-tube neighbors before the later `move_cost < 7` expansion gate, but no node is created)
```
 (corrected 2026-06-01: was "never reached"; binary computes
 `AStar_compute_edge_cost` before the `move_cost < 7` node-expansion check in
 `AStar_main_loop` via `decompile_function 0x00429A90` and confirms table value via
 `read_memory 0x0081870C` - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

**Special case — move_cost == 2 (TemporaryBlock / moving friendly):** {#4.5a}
When Can_Enter_Cell returns 2 (a moving friendly unit is temporarily blocking the cell),
the code walks the blocking unit's predicted trajectory forward up to 10 cells only
when `this+0x3C == 0`. If the predictor proves the blocker clears, cost stays 1.0;
otherwise cost becomes 4.0. When `this+0x3C == 1`, the predictor is skipped and cost
is 4.0; when `this+0x3C == 2`, cost becomes 1000.0. (corrected 2026-06-01: was
"prediction jam -> 4.0, urgency 2 -> 1000.0" without the urgency-1 forced-4.0
case; binary shows the 10-hop loop guarded by `*(this+0x3C)==0`, then the 4.0
assignment and urgency-2 override in `AStar_compute_edge_cost` via
`decompile_function 0x00429830` - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

**Temporary marker penalty:**
If destination cell flags contain `0x40000`, the current edge accumulator is
multiplied by 4.0 (`_DAT_007e37bc`). This is the search-scoped bridge-approach /
peer-path marker, not a generic cliff-ramp terrain flag.

**Bridge approach cost (diagonal movement on bridges):**
When `param_4 != 0` (on a bridge) and `this+0x1` is set, a cliff-factor is applied:
- Checks neighbor cells on both sides of the movement diagonal
- If both sides have bridge flags (0x100): multiply by 2.0 (`_DAT_007e37b4`)
- If one side has bridge flag: multiply by 1.0 (`0x007e2ac8` = 1.0, no change)
- If neither side: multiply by 10.0 (`_DAT_007e37b8`)

**Direction tiebreaker:**
A tiny epsilon is added per direction before the g-cost is stored:
```
DirectionEpsilon[9] at 0x0081872c:
  [0] = 0.001,  [1] = 0.005,  [2] = 0.002,  [3] = 0.006
  [4] = 0.003,  [5] = 0.007,  [6] = 0.004,  [7] = 0.008
  [8] = 0.000  (bridge crossing)
```
(corrected 2026-06-01: the 2026-05-28 correction was WRONG; binary stores g-cost and
f-cost as floats with `FSTP float ptr [node+0x4]` and `FSTP float ptr [node+0x8]`, and
heap operations compare `float ptr [node+0x8]`. The direction epsilons survive and can
affect ordering at float granularity. Verified via `disassemble_function 0x0042A460`,
`disassemble_function 0x0042DCA0`, and `read_memory 0x0081872C` -
ROOT_CAUSE: DECOMPILER_TYPE_MISREAD)

**Final g-cost for a node:**
```
g(neighbor) = g(current) + edge_cost * speed_factor + direction_epsilon
```
Where `speed_factor` is stored at `this+0x04` (a float scaling factor from the unit's
speed configuration). g-cost is stored as a float at `node+0x04`, and f-cost is stored
as a float at `node+0x08`. (corrected 2026-06-01: was integer truncation; binary shows
`FADD`/`FSTP float ptr` stores in `AStar_create_node` via
`disassemble_function 0x0042A460` - ROOT_CAUSE: DECOMPILER_TYPE_MISREAD)

### 4.6 h-cost (Heuristic)

Computed in `AStar_create_node` (0x42a460):

```c
dx = abs(neighbor->MapCoord_X - dest->MapCoord_X);
dy = abs(neighbor->MapCoord_Y - dest->MapCoord_Y);
h = sqrt(dx*dx + dy*dy);    // Euclidean distance
f = g + h;
```

The heuristic is **Euclidean distance** — `sqrt(dx^2 + dy^2)` using integer cell
coordinates. Do not infer admissibility/consistency from this alone: normal diagonal
edges do not receive a sqrt(2) movement-cost multiplier in `AStar_compute_edge_cost`,
so Euclidean `h` can exceed the cheapest diagonal-step g-cost path. (corrected
2026-06-01: was "admissible and consistent"; binary shows direction-independent base
edge costs plus small direction epsilons in `AStar_compute_edge_cost` via
`decompile_function 0x00429830` and `read_memory 0x0081870C` -
ROOT_CAUSE: INFERENCE_HARDENED)

For bridge-crossing neighbors (direction 8), the edge-cost increment passed into
`AStar_create_node` is Chebyshev distance between the current cell and tube endpoint;
the heuristic inside `AStar_create_node` is still the Euclidean distance from the new
node to the destination:
```c
g_increment_dir8 = max(abs(current.x - endpoint.x), abs(current.y - endpoint.y));
```
(corrected 2026-06-01: was "direction-8 heuristic uses Chebyshev"; binary shows the
Chebyshev value assigned to `fStack_28` before calling `AStar_create_node`, whose
heuristic remains Euclidean via `Sqrt_Approx`, in `AStar_main_loop` and
`AStar_create_node` via `decompile_function 0x00429A90` and
`decompile_function 0x0042A460` - ROOT_CAUSE: PARAM1_TYPE_MISREAD)

### 4.7 Open Set — Min-Heap

The open set is a binary min-heap stored in `PathfinderClass`:
- `this+0x14`: pointer to heap struct
  - `heap[0]`: count of elements
  - `heap[1]`: capacity
  - `heap[2]`: pointer to array of node pointers
  - `heap[3]`: max pointer (for bounds tracking)
  - `heap[4]`: min pointer

Nodes are sorted by f-cost (stored at `node[2]` as float).

**Heap operations:**
- **Insert (sift-up):** Walk up from new position, swap if parent's f > child's f
- **Extract-min (sift-down):** `MinHeap__SiftDown` at 0x42dca0 — standard binary heap
  sift-down comparing left child, right child, select smallest
- Both operations compare `*(float*)(node_ptr + 8)` which is the f-cost

### 4.8 Closed Set

Uses **stamp-based clearing** — instead of clearing the entire array between searches,
the pathfinder increments a stamp counter (`this+0x28`). A cell is "in the closed set"
if `closed_array[cell_index] == current_stamp`. This makes reset O(1) instead of O(n).

Two parallel closed sets exist:
- Ground level: `this+0x18` (stamp array), `this+0x24` (g-cost array)
- Bridge level: `this+0x1C` (stamp array), `this+0x20` (g-cost array)

If the stamp overflows to 0, a full clear of all arrays is performed (rare).

### 4.9 Node Pool

`PathfinderClass` uses two pre-allocated pools:

**Trail pool** (parent chain nodes) at `this+0x0C`:
- Base array of 12-byte structs: `{ CellClass* cell, int height, TrailNode* parent }`
- Pool counter at base+0x180000 (tracks next free index)
- Max ~0x20000 (131072) trail entries

**Search node pool** at `this+0x10`:
- Base array of 16-byte structs: `{ TrailNode* trail, float g_cost, float f_cost, int depth }`
- Pool counter at base+0x100000 (tracks next free index)
- Max ~0x10000 (65536) search nodes

### 4.10 Search Depth Limit

- Default: `max_iterations = 0xFFF7` (65527) when `param_6 == -1`
- Configurable via the caller
- Hard limit: `iteration_count` tracked as `local_34`; the expansion loop stops when
  `local_34 >= max_iterations` or the open set empties. The value 10000 is not an
  alternate loop break; it is a success-tail rejection check when `local_34 == 10000`.
  (corrected 2026-06-01: was "loop exits at max_iterations or 10000"; binary shows
  the loop guard `param_6 <= local_34` and later `local_34 != 10000` success predicate
  in `AStar_main_loop` via `decompile_function 0x00429A90` -
  ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
- On reaching the limit, the search returns 0 (failure)

### 4.11 Termination and Path Extraction

When the destination is reached (`current_cell == dest_cell && height matches`):

1. `AStar_reconstruct_path` (0x42aa90): Walks the parent chain backwards from the
   destination node to the source, producing a direction array. For each step, computes
   the direction index (0-7) from the cell coordinate delta using a lookup table at
   `0x818760`. Steps that span more than 1 cell in any axis get direction 8 (bridge).

   Output struct at `DAT_0089a2d8`:
   ```
   +0x00: source cell coords (packed short x, short y)   // DAT_0089a2d8
   +0x04: direction_count (int, result of Math__ftol)     // DAT_0089a2dc
   +0x08: path_length (node depth, = param_1[3])         // DAT_0089a2e0
   +0x0C: param_2 (caller-supplied context value)        // DAT_0089a2e4
   +0x10: (zero-init field)                              // DAT_0089a2e8
   +0x14: pointer → direction array                      // DAT_0089a2ec = &DAT_0089a324
   +0x18..+0x4B: (additional fields / padding)
   +0x4C: direction_array[path_length]  // DAT_0089a324
   +...:  height_array[path_length]     // parallel array
   ```
   (corrected 2026-05-28: was "+0x0C: direction_array // at DAT_0089a324" — WRONG offset.
   `DAT_0089a324 - DAT_0089a2d8 = 0x4C`, not 0x0C. `AStar_reconstruct_path` at `0x42aa90`
   shows `_DAT_0089a2ec = &DAT_0089a324` (pointer at +0x14), with the array itself starting
   at +0x4C. — ROOT_CAUSE: OFFSET_RETYPED_WRONG)

2. `Path_smooth_corners` (0x42b210): Post-processes the direction array to smooth
   unnecessary corners. Detects sequences where direction changes by +/-2 direction
   indices (a 90-degree turn on the 8-way compass) and merges them into the
   intermediate diagonal direction. (corrected 2026-06-01: was "45-degree zigzags";
   binary checks `(new_dir - old_dir) & 7` for `2` or `6` in `Path_smooth_corners` via
   `decompile_function 0x0042B210` - ROOT_CAUSE: INFERENCE_HARDENED)

3. `Path_optimize_straight_segments` (0x42b7f0): Further optimizes by detecting long
   straight runs and replacing zigzag approximations with true straight lines where
   the cells are actually passable. Uses `Can_Enter_Cell` to validate shortcuts.
   Marks skipped entries with -2 (0xFFFFFFFE), then compacts the array.

   The `0x13 < iVar13` guard stops the optimization scan after 20 processed input
   entries; it is not a 20-entry output cap. The later compaction loop independently
   traverses the original `direction_count - 1` bound (or stops at `-1`), copies every
   non-`-2` entry, fills the remainder with `-1`, and updates the result count.
   (corrected 2026-07-10: was "Maximum output: 20 entries" / "path is truncated";
   binary keeps `local_28 = param_2[2] - 1` across the guarded optimizer loop and
   uses that original bound in the later compactor via
   `decompile_function 0x0042B7F0` - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

---

## 5. Zone Map System — Hierarchical Precheck

### 5.1 Zone Map Structure

The zone map divides the map into zones — contiguous regions where all cells are
mutually reachable by a given zone passability/MovementZone index. Each cell has a
zone ID stored in:
```
ZoneMap->zone_data[cell_index]    // ushort
```

Zone IDs are looked up via `MapClass__GetZoneID` (0x56d230):
```c
cell_index = (map_border_x + 1 + map_origin_x) * cell_y + cell_x;
zone_data_index = *(ushort*)(zone_grid[cell_index]);
zone_id = *(ushort*)(zone_tables[zone_type] + zone_data_index * 2);
```
(corrected 2026-06-01: was `speed_type`; binary `MapClass__GetZoneID` indexes
`MapClass+0x18 + param_3*4`, and `AStar_pathfind_search` supplies the `TypeClass+0x5B4`
zone field, via `decompile_function 0x0056D230` and `decompile_function 0x0042C900` -
ROOT_CAUSE: STRUCT_FAMILY_CASCADE)

The zone map has 3 hierarchical levels (indexed 0-2), used in `Zone_precheck`.
Level 2 is the coarsest (whole-map connectivity), levels 0-1 are finer subdivisions.

### 5.2 Zone_precheck — 0x42c290

**Purpose:** Before running the expensive cell-level A*, check if the destination is
reachable at the zone level. This is a fast graph search on zone adjacency data.

**Algorithm:** For each of the 3 hierarchical levels (starting from level 2, coarsest):

1. Get source and dest zone IDs at this level
2. If source == dest: trivially connected at this level
3. Otherwise: run a Dijkstra-style accumulated-cost search on the zone adjacency
   graph; no destination heuristic is added to the heap key.
   (corrected 2026-07-10: was "mini A*"; binary computes each queued key from the
   predecessor cost plus zone-edge table cost, optional slope cost, and a 0.001
   edge tiebreaker only via `decompile_function 0x0042C290` -
   ROOT_CAUSE: INFERENCE_HARDENED)
   - Zone adjacency data stored at `DAT_0087f878 + level * 0x18`
   - Each zone has an adjacency list: `{ neighbor_zone_id, is_diagonal, edge_type }`
   - Edge costs use the table at `0x007e3794` indexed by edge_type:
     ```
     [0]=1.0, [1]=0.0, [2]=0.0, [3]=1.0, [4]=1.0, [5]=0.0, [6]=1.0, [7]=1.0
     ```
   - Diagonal edges get an extra `_DAT_007e3818` cost penalty
   - **Passability check:** `g_PassabilityMatrix[zone_type * 8 + edge_type] == 1`
     must hold for the edge to be traversable
     (corrected 2026-06-01: was `speed_type`; binary `Zone_precheck` uses
     `param_4 * 8 + edge_type`, where `param_4` is the `+0x5B4` zone field from
     `AStar_pathfind_search`, via `decompile_function 0x0042C290` -
     ROOT_CAUSE: STRUCT_FAMILY_CASCADE)

4. Uses the same min-heap algorithm as the cell-level search, but a separate zone
   heap stored at `this+0x68` (cell-level heap is `this+0x14`). (corrected 2026-06-01:
   was "same min-heap open set"; binary shows `Zone_precheck` using `this+0x68` and
   `AStar_main_loop` using `this+0x14` via `decompile_function 0x0042C290` and
   `decompile_function 0x00429A90` - ROOT_CAUSE: INFERENCE_HARDENED)

5. On success: stores the zone-level path in `this+0xBC` (up to 500 entries per level)
   and sets `this+0xC74 + level*4` to the path length

6. **Edge invalidation:** When cell-level A* fails, `PathfinderClass__UpdateHierarchicalEdges`
   (0x42ccd0) updates the zone adjacency data — marking edges that proved impassable
   so subsequent zone prechecks reflect the true connectivity.

### 5.3 Zone-Based Flood Fill — 0x5840c0

`ZoneMap__FloodFillReachableZones` is called by
`PathfinderClass__UpdateHierarchicalEdges`, not by `Zone_precheck`. It flood-fills
same-zone cells through 8 compass neighbors while collecting discovered adjacent-zone
IDs. A nonzero return means an unvisited in-playfield cell of the starting zone remains
inside the examined window; the caller invalidates that zone edge. A zero return means
the fill completed and the collected adjacent-zone IDs are supplied to the caller's
edge-update bookkeeping. (corrected 2026-07-10: was described as a `Zone_precheck`
helper returning whether all neighboring zones were reachable; binary returns 1 at the
unvisited same-zone-cell test and `PathfinderClass__UpdateHierarchicalEdges` invalidates
on nonzero, otherwise consumes the collected ID vector, via
`get_xrefs_to 0x005840C0`, `decompile_function 0x005840C0`, and
`decompile_function 0x0042CCD0` - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

---

## 6. SpeedType / LandType / Zone Passability Table Interaction

### 6.1 The Cost Pipeline

```
CellClass::LandType (cell+0xEC)  →  g_SpeedType_LandType_Table[land_type * 9 + speed_type]
                                         ↓
                                    0.0 = impassable
                                    nonzero = passable
                                         ↓
Can_Enter_Cell (vtable+0x1AC)     →  return value 0-7+
                                         ↓
                                    0-6 = passable (with cost)
                                    7+  = impassable
                                         ↓
EdgeCostBaseTable[return_value]   →  base movement cost (float)
                                     ↓
                               × temporary_marker_multiplier (4.0 if Cell+0x140 & 0x40000)
                               × bridge_factor (1.0, 2.0, or 10.0)
                               × speed_factor (unit-specific)
                               + direction_epsilon (0.001-0.008)
                                         ↓
                                    final float g-cost increment
```
(corrected 2026-06-01: was `Cell+0x4C` into `g_PassabilityMatrix[speed_type*8+land_type]`;
binary `UnitClass__Can_Enter_Cell` reads `*(int*)(cell+0xEC)` and compares
`g_SpeedType_LandType_Table[land_type * 9 + *(TypeClass+0x67C)]` to 0.0 via
`decompile_function 0x0073F0A0`; `g_PassabilityMatrix[param_4*8+edge_type]` is the
zone-edge matrix in `Zone_precheck`, not this landtype table - ROOT_CAUSE:
STRUCT_FAMILY_CASCADE)

### 6.2 Can_Enter_Cell (vtable+0x1AC)

This is the critical virtual function that determines passability. For `UnitClass`
at 0x73f0a0 (467 lines), it checks:
- Terrain type vs SpeedType (via `g_SpeedType_LandType_Table[LandType*9 + SpeedType]`,
  not the zone-edge `g_PassabilityMatrix`). (corrected 2026-06-01: binary
  `UnitClass__Can_Enter_Cell` reads `Cell+0xEC` and `TypeClass+0x67C` for this table via
  `decompile_function 0x0073F0A0` - ROOT_CAUSE: STRUCT_FAMILY_CASCADE)
- Occupancy (other units in the cell)
- Building blockage
- Bridge passability and height compatibility
- Cliff accessibility
- Overlay blockers (walls, fences)
- Owner restrictions
- Special cases (garrison, deployed units, etc.)

Return values:
| Value | Meaning                                          | Cost Table Entry |
|-------|--------------------------------------------------|-----------------|
| 0     | OK/Clear — cell is free                          | 1.0             |
| 1     | Crushable — civilian/neutral object              | 1000.0          |
| 2     | TemporaryBlock — moving friendly (see §4.5a)     | 1.0*            |
| 3     | BridgeRamp / ScatterRequired — bumped friendly   | 1.0             |
| 4     | FriendlyWall — allied wall overlay               | 60.0            |
| 5     | EnemyBlock — enemy unit/building                 | 20.0            |
| 6     | FriendlyStationary — non-moving allied unit      | 8.0             |
| 7+    | **IMPASSABLE — not expanded**                    | N/A             |

*Code 2 base cost is 1.0; with `this+0x3C == 0`, the friendly-unit-path-prediction
logic can keep 1.0 if the blocker clears or raise to 4.0 if jammed; `this+0x3C == 1`
forces 4.0 without the predictor; `this+0x3C == 2` forces 1000.0.
(corrected 2026-06-01: was missing the urgency-1 forced-4.0 case; binary shows this
branching in `AStar_compute_edge_cost` via `decompile_function 0x00429830` -
ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

---

## 7. Bridge Handling

### 7.1 Height System

Every cell has a height byte at `cell+0x11B` (signed char, typically 0-12). Bridges
add 4 to the effective height:

```
ground_height = cell->height                    // cell+0x11B
bridge_height = cell->height + 4               // when cell flags & 0x100
```

The pathfinder maintains TWO closed sets (ground and bridge level) and tracks the
current height in each node's trail entry (`trail->height`).

### 7.2 Tube Records Used By Bridge/Tube Direction

Direction-8 data is stored in `g_TubeArray`:
- Each tube record has endpoint coordinates at offset +0x28
- Records are indexed by `cell+0x116` (`TubeIndex`, short, -1 = no tube)
- Direction 8 in the path represents "jump through the tube/bridge endpoint record"
(corrected 2026-06-01: was `BridgeRecordTable` / `bridge_index`; binary uses
`g_TubeArray[cell->TubeIndex] + 0x28` in `AStar_main_loop`,
`Path_smooth_corners`, and `PathfinderClass__UpdateBridgePassability` via
`decompile_function 0x00429A90`, `decompile_function 0x0042B210`, and
`decompile_function 0x0042ACF0` - ROOT_CAUSE: STRUCT_FAMILY_CASCADE)

### 7.3 Bridge Passability Updates

`PathfinderClass__UpdateBridgePassability` (0x42acf0) toggles the `0x40000` flag on
peer path and/or tube-record cells as a temporary A* cost marker. It is cost-only:
the A* cost consumer multiplies by 4.0 when the destination cell has this bit. Normal
success and failure tails in `AStar_main_loop` call the helper again for cleanup;
pre-A* returns in `AStar_pathfind_search` do not set the marker.

### 7.4 Bridge-Aware Coordinate Resolution

`MapClass__ResolvePathCoord_BridgeAware` (0x583180) translates a raw cell coordinate
into the correct bridge-relative coordinate. For cells on bridges, it finds the nearest
bridge endpoint and maps through the bridge/tube endpoint data.

---

## 8. FootClass::Find_Nearby_Passable_Cell — 0x56dc20

Called when the destination cell is blocked. Searches expanding rectangle perimeters
around the target. The radius limit is exclusive: when the clamped limit is 32, the
greatest scanned radius is 31. (corrected 2026-07-10: was "up to 32 cells out";
binary tests the current radius, then increments it and exits when
`limit <= radius` via `decompile_function 0x0056DC20` -
ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

**Checks per candidate cell:**
1. `TechnoClass__IsOnScreen` — cell must be within visible map bounds
2. `CellRect__CheckPassability` (0x56e7c0) — foundation-sized rectangle is passable
3. Height compatibility: `abs(dest_height - candidate_height) < 2`
4. Bridge match check: candidate must match bridge level of destination
5. No impassable cliffs between candidate and destination
6. Occupancy check: `CellRect__CheckOccupancy` (0x586780) — no other units blocking

Fills a buffer of up to 24 (0x18) candidate cells, partitions candidates by whether
the coordinate normalization helper maps back to the same cell, then selects either a
frame-indexed candidate when the selector coordinate is the invalid sentinel or the
candidate closest to the supplied selector coordinate. (corrected 2026-06-01: was
"specific facing" and "matching ground"; binary uses `param_14` as a selector
coordinate/sentinel and the `FUN_006d6410` same-cell partition in
`FootClass__Find_Nearby_Passable_Cell` via `decompile_function 0x0056DC20` -
ROOT_CAUSE: PARAM1_TYPE_MISREAD)

---

## 9. Global Data Structures Summary

| Address     | Name                          | Description                                    |
|-------------|-------------------------------|------------------------------------------------|
| 0x007e3774  | CellNeighborOffsets[8]        | int32 offsets into CellClass* array per direction |
| 0x0081870c  | EdgeCostBaseTable[8]          | float base costs indexed by Can_Enter_Cell return |
| 0x0081872c  | DirectionEpsilon[9]           | float tiebreaker costs per direction (0.001-0.008) |
| 0x007e37b4  | BridgeBothSidesCost           | float 2.0 — bridge with cliff on both sides     |
| 0x007e37b8  | BridgeNoCliffCost             | float 10.0 — bridge with no cliff               |
| 0x007e37bc  | AStar temporary marker multiplier | float 4.0 — cost multiplier when destination CellClass+0x140 has search-scoped bit 0x40000 |
| 0x007e3794  | ZoneEdgeCostTable[8]          | float costs for zone-level edges                 |
| 0x0087f858  | ZoneMapLookupTable            | Zone map index → zone neighbor data              |
| 0x0087f878  | ZoneAdjacencyData[3]          | Per-level zone adjacency graphs (3 levels)       |
| 0x0089c2dc  | MapGridWidth                  | int — map width for cell coordinate linearization |
| 0x0089c2d8  | HeightStepSize                | int — height step conversion factor              |
| 0x008b413c  | g_TubeArray                   | Tube/bridge endpoint record pointer array used by direction 8 |
| 0x0089a2d8  | PathResult                    | Static output buffer for reconstructed path      |
| 0x0089a324  | PathDirectionArray            | Static array of direction entries (in PathResult) |
| g_CellArray_Base | CellPtrArray            | Flat array of CellClass*, 512 wide               |
| g_PassabilityMatrix | ZoneType×EdgeType     | byte[zone_type][8], 1=zone edge passable, 0=blocked |
| g_SpeedType_LandType_Table | LandType×SpeedType | float table indexed as `LandType*9 + SpeedType`; 0.0 blocks UnitClass terrain entry |
| g_DirectionOffsets  | DirDeltaX[8]          | short x-deltas per direction (runtime-filled)    |
| 0x0089f68a  | DirDeltaY[8]                  | short y-deltas per direction (runtime-filled)    |

---

## 10. Functions Labeled in Ghidra

| Address    | Name                                      | Confidence |
|------------|-------------------------------------------|-----------|
| 0x4d3920   | FootClass__Find_Path                      | HIGH (pre-existing) |
| 0x4cbba0   | FootClass__Run_AStar                      | HIGH      |
| 0x42c900   | AStar_pathfind_search                     | HIGH (pre-existing) |
| 0x429a90   | AStar_main_loop                           | HIGH      |
| 0x42a460   | AStar_create_node                         | HIGH      |
| 0x429830   | AStar_compute_edge_cost                   | HIGH      |
| 0x42aa90   | AStar_reconstruct_path                    | HIGH      |
| 0x42b210   | Path_smooth_corners                       | HIGH      |
| 0x42b7f0   | Path_optimize_straight_segments           | HIGH      |
| 0x42b420   | Path_smooth_single_segment                | HIGH      |
| 0x429780   | Path_walk_directions_to_cell              | HIGH      |
| 0x42a5b0   | PathfinderClass__Reset                    | HIGH      |
| 0x42acf0   | PathfinderClass__UpdateBridgePassability  | HIGH (pre-existing) |
| 0x42ccd0   | PathfinderClass__UpdateHierarchicalEdges  | HIGH      |
| 0x42cf80   | PathfinderClass__InvalidateZoneEdge       | HIGH      |
| 0x42dca0   | MinHeap__SiftDown                         | HIGH      |
| 0x42c290   | Zone_precheck                             | HIGH (pre-existing) |
| 0x56dc20   | FootClass__Find_Nearby_Passable_Cell      | HIGH      |
| 0x56d3f0   | ZoneMap__CellToZoneIndex                  | HIGH      |
| 0x5840c0   | ZoneMap__FloodFillReachableZones           | HIGH      |
| 0x56d230   | MapClass__GetZoneID                       | HIGH (pre-existing) |
| 0x583180   | MapClass__ResolvePathCoord_BridgeAware    | HIGH      |
| 0x56e7c0   | CellRect__CheckPassability                | HIGH      |
| 0x586780   | CellRect__CheckOccupancy                  | HIGH      |
| 0x429680   | Pathfinding__ComputeHeightStep            | HIGH (pre-existing) |
| 0x429660   | Pathfinding__InitHeightStep               | HIGH (pre-existing) |

---

## 11. Key Findings for Rust Implementation

1. **Path queue is 24 entries max** — but the actual path from A* can be much longer.
   `Path_optimize_straight_segments` stops its shortcut-analysis scan after 20
   processed entries, then compacts all non-`-2` entries across the original result
   bound; that pass does not itself truncate the result to 20. The later copy into the
   FootClass queue remains independently bounded to 24 entries.
   (corrected 2026-07-10: was "post-processing truncates to ~20"; binary's compactor
   uses the original `direction_count - 1` bound after the 20-entry scan guard via
   `decompile_function 0x0042B7F0` - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

2. **The A* uses TWO height layers** — ground and bridge. The closed set is doubled
   to avoid ground-level paths interfering with bridge-level paths.

3. **Stamp-based closed set** — avoids clearing the closed set array between searches.
   Just increment the stamp. Only clear everything when stamp overflows to 0.

4. **The heuristic is Euclidean** — `sqrt(dx^2 + dy^2)` on cell coordinates. Do not
   claim admissibility/consistency: normal diagonal edges use the same base-cost table
   as cardinal edges plus only the direction epsilon. (corrected 2026-06-01: was
   "admissible and consistent"; binary shows Euclidean `Sqrt_Approx` in
   `AStar_create_node` but no sqrt(2) diagonal edge multiplier in
   `AStar_compute_edge_cost`, via `decompile_function 0x0042A460` and
   `decompile_function 0x00429830` - ROOT_CAUSE: INFERENCE_HARDENED)

5. **65,527 default max iterations** — when caller passes -1, `AStar_main_loop`
   changes it to `0xFFF7`. The loop guard is `local_34 >= max_iterations`; 10000 is
   only a success-tail rejection equality check, not a second loop cap. (corrected
   2026-06-01: was "search will give up after this many" plus 10000 cap wording;
   binary shows this in `AStar_main_loop` via `decompile_function 0x00429A90` -
   ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

6. **Friendly-unit-path-prediction** — when Can_Enter_Cell returns 2 (TemporaryBlock /
   moving friendly), `this+0x3C == 0` runs the 10-hop blocker prediction and can keep
   cost at 1.0 if the blocker clears; otherwise cost is 4.0. `this+0x3C == 1` forces
   4.0 without prediction, and `this+0x3C == 2` forces 1000.0. (corrected
   2026-06-01: was missing the urgency-1 forced-4.0 path; binary shows this in
   `AStar_compute_edge_cost` via `decompile_function 0x00429830` -
   ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

7. **Zone precheck prevents hopeless searches** — the hierarchical zone system does a
   fast connectivity check before committing to cell-level A*. For equal initial zone
   IDs, initial precheck failure disables hierarchy and falls back to unrestricted
   cell-level A*. A cross-zone mismatch while hierarchy is enabled returns 0 immediately,
   and a failed precheck after hierarchical-edge update during the retry loop also ends
   that retry path. (corrected 2026-07-10: was "If zones are disconnected, the search
   is aborted immediately"; binary has distinct same-zone initial fallback,
   cross-zone hard-fail, and retry-time failure branches via
   `decompile_function 0x0042C900` - ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

8. **Direction = 8** — this is a special non-compass tube/bridge endpoint jump using
   `cell+0x116` as `TubeIndex` into `g_TubeArray`, then record `+0x28` as the endpoint.
   (corrected 2026-06-01: was "bridge to the other endpoint"; binary shows
   `g_TubeArray[cell->TubeIndex]+0x28` in `AStar_main_loop` via
   `decompile_function 0x00429A90` - ROOT_CAUSE: STRUCT_FAMILY_CASCADE)

9. **Path smoothing is a two-pass process** — first smooth +/-2 direction-index
   changes (90-degree turns on the 8-way compass) with `Path_smooth_corners`, then
   optimize straight segments by checking if shortcuts are passable
   (`Path_optimize_straight_segments`). (corrected 2026-06-01: was "45-degree
   zigzags"; binary checks direction delta `2`/`6` in `Path_smooth_corners` via
   `decompile_function 0x0042B210` - ROOT_CAUSE: INFERENCE_HARDENED)

10. **The min-heap is a standard binary heap** — nodes sorted by f-cost. No Fibonacci
    heap or other exotic structure. Simple and fast.
