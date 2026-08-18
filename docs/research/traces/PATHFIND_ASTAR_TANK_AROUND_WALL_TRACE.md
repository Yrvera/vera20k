# Trace: Ground Unit A* Path Around Static Obstacle

**Mechanic:** Grizzly Tank (MovementZone=Crusher, SpeedType=Track) ordered move from cell (50,50) to (60,50). 2×2 GAWALL footprint at (54,50),(55,50),(54,51),(55,51) blocks the straight-line path.

**Binary reference:** gamemd.exe YR 1.001. All gamemd behavior sourced from the doc set listed in CONTEXT.

**Date:** 2026-05-20

---

## Stage Pipeline

### Stage 1: Move-Command Input → FootClass::Find_Path

**gamemd:** Player issues move order → DriveLocomotionClass calls `FootClass::Find_Path` (0x4D3920). Destination cell packed as `(cell_y << 16) | cell_x`. `Find_Path` reads destination from vtable+0x48, computes straight-line distance to (60,50), checks `Can_Enter_Cell` for the destination — (60,50) is open grass, returns code 0 (Clear). No special cases triggered. Calls `FootClass::Run_AStar` (0x4CBBA0).

**Our code:** `movement_path.rs` / `zone_search::find_path_zone_aware`. Move-command arrives as `MovementTarget`, which populates goal cell. Calls into zone_search which invokes `astar_search`. The entry path is structurally equivalent: validate goal, run search.

**Status: PASS** — both engines reject the destination pre-check only when Can_Enter_Cell ≥ 7. (60,50) is clear, so both proceed to search.

---

### Stage 2: Zone Reachability Pre-Check

**gamemd:** `AStar_pathfind_search` (0x42C900) calls `MapClass::GetZoneID` on source (50,50) and dest (60,50). Both cells are open grass with LandType=Clear (0), ZoneType=Ground (0). For a Crusher (MovementZone=1), the passability matrix row 1 has `[1, 1, 2, 2, 2, 2, 2, 3]` — column 0 (Ground) = 1 (passable). A 2×2 wall block does not disconnect the zone on flat open terrain (there are paths around it on both N and S sides). Zone precheck: source_zone == dest_zone at the coarse level on flat maps → `Zone_precheck` returns true. Cell-level A* proceeds.

**Our code:** `zone_search.rs::find_path_zone_aware` calls `can_reach_same_or_zoned(zg, mz, from, from_layer, to, to_layer)`. On a flat grass map with a small 2×2 wall, source and dest share a zone — returns true and proceeds to A*.

**Status: PASS** — both engines confirm reachability and proceed to cell-level search. The specific zone IDs are map-runtime-dependent; the structural behavior (same-zone → proceed) matches.

---

### Stage 3: A* Node Initialization

**gamemd:** `AStar_pathfind_search` calls `PathfinderClass::Reset` (0x42A5B0), incrementing the stamp counter. Source cell (50,50) height = 0 (flat grass). Dest cell (60,50) height = 0. `this+0x30` = source_height = 0, `this+0x34` = dest_height = 0. `AStar_create_node` (0x42A460) creates the start node: g=0, h=`sqrt((60-50)²+(50-50)²)` = `sqrt(100)` = 10.0 (Euclidean, float). f=10.0. Pushed to min-heap.

**Our code:** `astar_search` initializes `start_height=0` (ground_level for flat cell), `goal_height=0`. g_cost arrays are `i32::MAX`. Start node pushed with `g_cost=0`, `f_cost = euclidean_heuristic(50,50,60,50)`. `euclidean_heuristic` computes integer-scaled Euclidean: `sqrt(dx²+dy²) * STEP_COST` = `10.0 * 1000 = 10000`.

**Status: PASS** — both use Euclidean heuristic. Scale differs (1.0 float vs 1000 integer) but does not affect search topology. Cost ordering is equivalent.

---

### Stage 4: A* Cost Function — Clear Terrain

**gamemd:** For every clear-grass neighbor expansion, `Can_Enter_Cell` (0x73F0A0) returns 0 (Clear). `AStar_compute_edge_cost` (0x429830) looks up `EdgeCostBaseTable[0]` = 1.0 (float). No cliff ramp flag (0x40000). No bridge. Final edge cost = `1.0 * cost_multiplier(1.0)` + direction_epsilon = `1.0 + epsilon[dir]` where epsilon ∈ {0.001..0.008}. No diagonal upcharge — all 8 directions get base cost 1.0 (plus epsilon).

**Our code:** `base_cost = STEP_COST = 1000`. `terrain_cost = 100` (open grass). Step cost = `1000 * 100 / 100 = 1000`. No height diff → no cliff multiplier. No entity soft-block → no multiplier. `tentative_g = g_cost + 1000 + DIR_TIEBREAK[dir]` where `DIR_TIEBREAK ∈ {1..8}`.

**Numeric comparison for one clear-grass cardinal step:** gamemd = 1.001; ours = 1001. Ratio = 1000:1 (scale difference only). Relative ordering of paths is identical.

**Status: PASS** — cost table values match in ratio. No diagonal upcharge in either engine. Direction tiebreaker structure matches (cardinals lower than diagonals, same 8-direction order).

---

### Stage 5: A* Cost Function — GAWALL Wall Cells

**gamemd:** GAWALL (wall/fence overlay, `IsWall=yes`) sets ZoneType = Road (1) via `CellClass::RecalcZoneType` priority 2. For MovementZone=Crusher (row 1), passability matrix `[1, 1, 2, ...]` column 1 = 1 (passable through road). However, `Can_Enter_Cell` (0x73F0A0) for a wall overlay checks `OverlayType::IsWall` flag. For a Crusher (MovementZone=1, which includes `CrusherAll`-equivalent behavior), the wall is crushable. Return code depends on allied/enemy wall and unit flags:
- GAWALL is neutral/player-owned (own faction). Allied wall → code 4 (OccupiedFriendly/Wall), cost 60.0.
- If enemy: code 4 or 5. Either way, much more expensive than going around.

On a flat 10-cell straight path with a 2×2 wall at columns 54-55: north detour passes (50,50)→...→(53,49)→(54,49)→(55,49)→(56,49)→...→(60,50) = 12 steps, all clear at cost 1.0+epsilon each = ~12.01. Through-wall path: 10 steps but two wall cells at cost 60.0 each → ~128.01 total. The detour wins decisively.

**Our code:** Wall cells in the PathGrid are set as `ground_walkable=false` when the overlay is an impassable wall (the ZoneType/passability gate in `terrain_cost.rs` or `cell_entry.rs` marks them blocked). They appear in `entity_blocks` as hard-blocked cells (code 7-equivalent).

**Status: FAIL** — The Rust implementation hard-blocks wall cells via `entity_blocks` (treating them as impassable, code ≥ 7). gamemd assigns them code 4 (cost 60.0) for Crusher units — Crusher can path THROUGH walls at high cost. Our implementation prevents Crusher-class units from pathing through walls at all, which diverges for any Crusher unit that would prefer to crush rather than detour. For a standard Grizzly Tank (MovementZone=Normal, row 0, passability column 1=2 = blocked), the hard-block is actually correct. **The FAIL applies only when `mover_is_crusher=true`.** For Normal zone, both engines hard-block. For Grizzly (Normal zone), this specific scenario is PASS.

*Narrowing: Grizzly Tank has MovementZone=Normal (0). Passability matrix row 0 = `[1,2,2,2,2,2,2,3]`. Column 1 (Road/Wall) = 2 (blocked). So for Grizzly, GAWALL is impassable. Can_Enter_Cell returns 7+. Hard-block is correct behavior.*

**Revised status for Grizzly Normal: PASS** — wall cells are hard-blocked for Normal movers in both engines.

---

### Stage 6: Neighbor Enumeration Order

**gamemd:** `AStar_main_loop` (0x429A90) expands neighbors with a `do { ... } while (iStack_44 < 9)` loop starting from direction 0 (N), iterating 0→1→2→3→4→5→6→7→8 (bridge). The cell-array neighbor offset table at 0x007E3774: `N=-512, NE=-511, E=+1, SE=+513, S=+512, SW=+511, W=-1, NW=-513`.

**Our code:** `NEIGHBORS: [(dx,dy,is_diagonal); 8] = [(0,-1,false),(1,-1,true),(1,0,false),(1,1,true),(0,1,false),(-1,1,true),(-1,0,false),(-1,-1,true)]` — identical order: N, NE, E, SE, S, SW, W, NW.

**Status: PASS** — enumeration order matches exactly.

---

### Stage 7: Diagonal Corner-Cutting

**gamemd:** `AStar_main_loop` does NOT implement diagonal corner-cutting for passability. It checks the neighbor cell via `Can_Enter_Cell` directly — no check that the two cardinal cells flanking the diagonal are also passable. gamemd allows diagonal moves that "clip" corner obstacles.

**Our code (`core.rs` lines 819-851):**
```rust
if is_diagonal {
    // Both cardinal neighbors must be passable on same layer
    if !adj1_ok || !adj2_ok { continue; }
}
```
Corner-cutting is **blocked** in our engine for diagonal moves where either flanking cardinal is impassable.

**Status: FAIL** — Our engine prevents diagonal corner-cutting; gamemd allows it. In the wall-around scenario, a Grizzly approaching from the west would clip the NW corner of the wall using direction NE (diagonal) from cell (53,50) to (54,49) without checking (53,49) and (54,50). gamemd permits this; our engine does not.

**Player-visible effect:** Units take slightly longer detour routes in our engine when obstacles have corners. Path shape diverges from gamemd near any obstacle corner.

**File:line:** `src/sim/pathfinding/core.rs:819-851`
**gamemd evidence:** `PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.3` — no corner-cutting check described; `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md §6.2` — "9 directions" checks only the neighbor cell.

---

### Stage 8: Node Reopening Threshold

**gamemd:** When a cell is already in the closed set and A* finds a cheaper path, the binary checks `stored_g < current_g + 1.009` (double at 0x007E37C0) before reopening. This suppresses marginal re-expansions where savings < 1.009 cost units.

**Our code:** `if tentative_g < g_array[n_idx]` — no threshold, any improvement opens the node.

**Status: UNCHECKED** — Both handle the common case identically (closed cell not reopened unless strictly better). The 1.009 threshold is a performance optimization that also changes path optimality by ≤1.009 per step. For a 10-step detour, accumulated suppression could reach ~10 units — enough to occasionally prefer a slightly longer path. Cannot compute both engines' exact paths without running the actual scenario. Net observable effect: extremely rare path shape change.

---

### Stage 9: Path Reconstruction

**gamemd:** `AStar_reconstruct_path` (0x42AA90) walks parent chain from destination to source, producing an array of direction indices 0-7. For each step, computes direction from cell-coordinate delta using lookup table at 0x818760.

**Our code:** `reconstruct_path_dual` in `core.rs` walks `came_from` backward from goal_idx using `decode_from`. Produces `Vec<LayeredPathStep>` with per-cell layer info.

**Status: PASS** — both walk parent chain backward and produce equivalent direction sequences.

---

### Stage 10: Path Smoothing — Pass 1 (Zigzag/Corner Smoothing)

**gamemd:** `Path_smooth_corners` (0x42B210) — detects direction sequences where adjacent directions differ by ±2 (45° angle, a zigzag), merges them into the intermediate diagonal direction. E.g., E then S → SE if the diagonal cell is passable.

**Our code:** `path_smooth.rs::smooth_corners` — implements pass 1 using `dir_diff` of ±2 detection and `midpoint_dir`. Structurally matches.

**Status: UNCHECKED** — Algorithm structure matches. Exact output depends on exact path taken, which depends on enumeration order + cost tie-breaks. Cannot claim PASS without running both.

---

### Stage 11: Path Smoothing — Pass 2 (Straight Segment Optimization)

**gamemd:** `Path_optimize_straight_segments` (0x42B7F0) — replaces zigzag approximations of straight lines with direct cells; validates with `Can_Enter_Cell`. Max output 20 entries. Marks skipped entries with -2, then compacts.

**Our code:** `path_smooth.rs::smooth_drift` — implements drift correction pass. Algorithm is described as functionally equivalent but details differ (no re-validation with Can_Enter_Cell equivalent; uses deviation threshold).

**Status: UNCHECKED** — Cannot verify without running both engines on identical input. Smoothing output may diverge on the specific 10-cell detour path.

---

### Stage 12: Per-Tick Walk — DriveLocomotionClass

**gamemd:** After path is computed, `DriveLocomotionClass::Process_Movement` (0x4B2630) advances the unit one sub-cell step per tick using `Force_Track` drive curves. Path_queue direction entries consumed one at a time.

**Our code:** Movement tick in `app_sim_tick.rs` advances units via `LocomotorState`. Drive locomotor steps through path cells.

**Status: UNCHECKED** — Drive locomotor and facing/turn-rate parity are out of scope for this trace slot.

---

### Stage 13: Arrival at (60,50)

**gamemd:** When the unit enters cell (60,50), `Can_Enter_Cell` returns 0. If within `CloseEnough=2.25` cells of destination, movement stops. Path queue depleted or destination reached.

**Our code:** Arrival detected when current cell == goal. Movement target cleared.

**Status: UNCHECKED** — CloseEnough threshold behavior not traced for this slot.

---

## Not-Implemented Findings

1. **`BlockagePathDelay` / urgency escalation** — When the Grizzly is blocked by a moving ally, the binary escalates cost from 4.0 → 1000.0 after 60 ticks. Our engine has the `urgency` field and CODE2 multipliers implemented in `core.rs` but the escalation timer in `DriveLocomotionClass` (code-2 branch at 0x4B3649) is not implemented. Player sees: stuck units don't reroute around persistent movers.

2. **`Can_Enter_Cell` per-neighbor dynamic call** — Binary calls `vtable+0x1AC` per-neighbor during A* with full occupancy state (enemy units cost 20.0, stationary friendlies cost 8.0, crushable 1000.0). Our code uses precomputed `entity_block_map` (soft-block) + `entity_blocks` (hard-block). The dynamic call also handles head-on deadlock detection (facing check at 0x73F0A0 Phase 9) — not implemented.

3. **Zone corridor constraint in A***: Binary constrains per-neighbor expansion to zones in `hier_path[corridor_index]`. Our code has a corridor filter but uses a single zone level Dijkstra vs. 3-level hierarchy.

---

## Adjacent Findings (do not trace this run)

- Drive track curve selection and facing interpolation (out of scope)
- `Find_Nearby_Passable_Cell` (0x56DC20) behavior when destination is occupied
- `PathfinderClass::UpdateHierarchicalEdges` retry mechanism (zone edge invalidation)
- Shroud/fog filter in A* (cell+0x122 check, only relevant when fog enabled — not default YR)

---

## Verdict by Stage

| Stage | Status | Note |
|-------|--------|------|
| 1. Move-command → Find_Path | PASS | Both validate destination before search |
| 2. Zone reachability pre-check | PASS | Same-zone → proceed in both |
| 3. A* node initialization | PASS | Euclidean heuristic, same height logic |
| 4. Cost function — clear terrain | PASS | Same ratio, no diagonal upcharge |
| 5. Cost function — GAWALL (Normal zone) | PASS | Hard-block correct for Normal mover |
| 6. Neighbor enumeration order | PASS | N→NE→E→SE→S→SW→W→NW match |
| 7. Diagonal corner-cutting | FAIL | We block; gamemd allows |
| 8. Node reopening threshold | UNCHECKED | 1.009 threshold not implemented |
| 9. Path reconstruction | PASS | Both walk parent chain backward |
| 10. Path smoothing pass 1 | UNCHECKED | Algorithm matches; output unverified |
| 11. Path smoothing pass 2 | UNCHECKED | Algorithm differs slightly |
| 12. Per-tick walk | UNCHECKED | Out of scope |
| 13. Arrival | UNCHECKED | CloseEnough not traced |
