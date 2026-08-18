# Pathfinding — System Synthesis

**System:** `pathfinding` (gamemd.exe core A* + PathfinderClass coordination layer)
**Decode date:** 2026-05-24
**Anchors:** `FootClass__Find_Path @ 0x004D3920`, `AStar_pathfind_search @ 0x0042C900`, `PathfinderClass__Constructor @ 0x0042A6D0`
**Scope:** 34 symbols (33 functions + 1 struct), 0 TS-excluded, 0 ceiling escalations
**Verification:** all 34 per-symbol decodes proofed (28 PROOFED ≥90, 1 PROOFED-YELLOW 95, 0 REJECTED in this run); 84 parity rows in `_parity.md`

---

## Summary

The pathfinding system computes a sequence of cell waypoints from a unit's
current cell to a destination cell, using a global `PathfinderClass` singleton
that holds all per-search state (open/closed lists, node pool, exclusion
vectors, zone path arrays). The flow is:

1. A locomotor (`DriveLocomotionClass::Process_Movement`,
   `WalkLocomotionClass::ProcessMovement`, `ShipLocomotionClass::Process_Movement`)
   asks the unit's `FootClass` for a new path by calling
   `FootClass__Find_Path` (0x004D3920) — the system's entry point.
2. `FootClass__Find_Path` normalizes the destination (water/building remap
   via `Find_Nearby_Passable_Cell`), wraps the search in occupancy mark/unmark
   (vtable+0x124), then calls `FootClass__Run_AStar` (0x004CBBA0).
3. `Run_AStar` walks any prior direction-buffer into a cell waypoint hint
   (`Path_walk_directions_to_cell`), then dispatches to
   `AStar_pathfind_search` (0x0042C900) on the global `PathfinderClass` at
   `0x0087E8B8`.
4. `AStar_pathfind_search` runs zone precheck (`Zone_precheck`, 0x0042C290),
   optionally restricts the search to a zone corridor, then runs
   `AStar_main_loop` (0x00429A90). On failure it invalidates the failed
   zone edge (`UpdateHierarchicalEdges`, `InvalidateZoneEdge`) and retries
   up to 4 (targeted) or 5 (scatter) times.
5. `AStar_main_loop` is a binary min-heap A* with 8-direction expansion
   plus a special direction 8 for tube edges. Each neighbor goes through
   `AStar_compute_edge_cost` (terrain code → multiplier) and
   `AStar_create_node` (heuristic = Euclidean, parent pointer, bridge-layer
   selection). The closed list uses an epoch trick so reset cost is O(1).
6. On success, the discovered path runs through `AStar_reconstruct_path` →
   `Path_smooth_corners` (diagonal-anchor zigzag elimination) →
   `Path_optimize_straight_segments` (long-run collapse via Chebyshev drift
   detection). The final direction-byte buffer is copied into the
   FootClass's 24-step ring at `+0x5E0` and a timestamp is stamped at
   `+0x640..+0x648`.

The system is reusable across every unit type that has a locomotor (Drive,
Walk, Ship, Hover, Jumpjet). It is **not** used by Fly (aircraft pathing is
facing-based, no cell search) and is bypassed entirely by Teleport
(chrono-warp).

---

## Symbol scope

| Kind | Count | Notes |
|---|---|---|
| Functions | 33 | Anchor + 6 PathfinderClass methods + 4 A* core + 4 post-process + 5 helpers + 8 scope-explorer additions + 5 PathfinderHeapVec methods |
| Struct | 1 | `PathfinderClass` (~3200 bytes, NOT defined in Ghidra) |
| Globals | 5 referenced | `g_PathfinderClass_Singleton @ 0x0087E8B8`, `g_DirectionOffsets @ 0x0089F688`, `g_TubeArray @ 0x008B413C`, `g_DirTable @ 0x00818760`, `g_AStar_EdgeCost_BaseTable @ 0x0081870C` |
| Enums | 0 documented | Direction encoding (0..7) is a compass-ordering constant table, not a typedef'd enum |
| Strings | 1 found | `"Warning. A* without HS" @ 0x008187f0` — diagnostic message only |

Excluded from this decode (cited but covered by other systems):
`MapClass__Get_CellClass`, `MapClass__GetZoneID`, `ZoneMap__CellToZoneIndex`,
`ZoneMap__FloodFillReachableZones`, `Zone_Estimate_Slope_Cost`,
`CellRect__CheckPassability`, `TechnoClass__Is_Current_Cell_Obstacle_Free`,
`FootClass__Get_Slope_Speed_Factor`, locomotor `Process_Movement` overrides,
utility helpers (`Math__ftol`, `Sqrt_Approx`, `CoordStruct__Set`,
`RateTimer__Current`), runtime helpers (`operator_new`, `Register_heap_pool`,
`GameDebugLog__Assert`). See `_manifest.yaml`.

TS-legacy mentions surfaced but not implemented in standard YR play:
- `Subterranean` (typo: `Subterannean` in stock INI) — TS movement zone
- Tube traversal (direction 8 + `g_TubeArray`) — TS subterranean carryover

These appear in dispatch tables (e.g., `Path_smooth_corners` direction-8
exclusion) but are inactive in stock YR per project memory.

---

## Control flow

### Top-level dispatch

```
locomotor.Process_Movement (Drive/Walk/Ship/Hover/Jumpjet)
  └─→ FootClass__Find_Path (0x4D3920)                  ← entry, 12 xrefs
        │   • normalize destination (water/building → Find_Nearby_Passable_Cell)
        │   • compute Euclidean dist; check vs CloseEnough (RulesClass+0x1718)
        │   • vtable+0x124 (Mark) — occupancy lock
        ├─→ FootClass__Run_AStar (0x4CBBA0)            ← bridge to A* core
        │     │   • vtable+0x4C → GetCoords (start)
        │     │   • lepton→cell floor: (x + (x>>31&0xFF)) >> 8
        │     │   • Path_walk_directions_to_cell (0x429780)  ← hint from prior buffer
        │     └─→ AStar_pathfind_search (0x42C900)     ← A* orchestrator
        │           ├─→ MapClass__ResolvePathCoord_BridgeAware (bridge snap, both endpoints)
        │           ├─→ Zone_precheck (0x42C290)       ← zone-level reachability
        │           │     │   • zone-level Dijkstra over zone-edge graph
        │           │     │   • slope cost (Zone_Estimate_Slope_Cost + Get_Slope_Speed_Factor)
        │           │     │   • bridge-edge tiebreak (+0.001002 float)
        │           │     │   • MinHeap__SiftDown (0x42DCA0)
        │           │     └─→ records zone path at Pathfinder+0xBC..+0xC73
        │           │
        │           │  ── retry loop (4 targeted / 5 scatter) ──
        │           │
        │           ├─→ AStar_main_loop (0x429A90)     ← per-cell A*
        │           │     │   • binary min-heap open set (Pathfinder+0x68)
        │           │     │   • closed set (Pathfinder+0x10, 65536 cells)
        │           │     │   • iteration cap = 10000 nodes
        │           │     │   • reopen tolerance = 1.009 (DAT_007E37C0)
        │           │     │   • direction-8 = tube edge, no cost fn
        │           │     │   • blocked-goal accept if |height_diff| < 2
        │           │     │   • hierarchy gate: marked_level0 OR blocker_neighbor_count != 0
        │           │     ├─→ AStar_compute_edge_cost (0x429830)    ← per-edge multiplier
        │           │     │     │   • base table @ 0x0081870C: [1, 1000, 1, 1, 60, 20, 8, 10000]
        │           │     │     │   • code 2 (peer unit) urgency: 0=chain×10/1or4, 1=4, 2=1000
        │           │     │     │   • 0x40000 search marker = ×4
        │           │     │     │   • bridge flank: gate Pathfinder+0x01, ×10/×1/×2
        │           │     │     └─→ entity cost + slope cost
        │           │     ├─→ AStar_create_node (0x42A460)          ← heuristic + parent
        │           │     │     │   • h = Sqrt_Approx(dx²+dy²) (Euclidean)
        │           │     │     │   • bridge-layer height: parent_h ± 1 / ± 4 / +4
        │           │     │     │   • dir tiebreak from g_DirEpsilonTable @ 0x0081872C
        │           │     │     └─→ pool alloc from Pathfinder+0x68
        │           │     └─→ AStar_reconstruct_path (0x42AA90)    ← parent backtrace
        │           │           │   • direction byte from g_DirTable @ 0x00818760
        │           │           │   • encoding: 0=S, 1=SW, 2=W, 3=NW, 4=N, 5=NE, 6=E, 7=SE
        │           │           │   • output to DAT_0089A2D8
        │           │
        │           │  ── on failure: invalidate, retry ──
        │           ├─→ PathfinderClass__UpdateHierarchicalEdges (0x42CCD0)
        │           │     │   • soft-split: flood-fill reachable zones, exclude all-neighbor edges
        │           │     │   • hard-split: dispatch to InvalidateZoneEdge
        │           │     └─→ PathfinderClass__InvalidateZoneEdge (0x42CF80)
        │           │           │   • read stored zone path at Pathfinder+0xBC + level*1000
        │           │           │   • append direct edge to exclusion vec at Pathfinder+0x74+level*0x18
        │           │           │   • append common-neighbor edges (early_endpoint, candidate)
        │           │           │   • clear Pathfinder+0x38 if no actionable edge
        │           │
        │           │  ── on success: post-process ──
        │           ├─→ Path_smooth_corners (0x42B210)
        │           │     │   • diagonal-anchor zigzag elimination
        │           │     │   • direction 8 (tube) excluded from smoothing
        │           │     └─→ Path_smooth_single_segment (0x42B420)  ← single shortcut check
        │           │           ├─→ MapClass__Get_Slope_Cost_At_Cell (0x56BCD0)
        │           │           └─→ MapCoord_Step_By_Direction (0x42D490)
        │           └─→ Path_optimize_straight_segments (0x42B7F0)
        │                 │   • 20-step look-ahead window
        │                 │   • Chebyshev drift detection (cum_x, cum_y regression)
        │                 │   • compaction pass: collapse 0xFFFFFFFE entries
        │                 ├─→ Path_Find_Split_Anchor (0x42BCA0)
        │                 └─→ Path_Reroute_Straight_Line (0x42BE20)
        │
        │   • vtable+0x124 (Unmark) — release occupancy lock
        │   • copy ≤24 steps to FootClass+0x5E0 ring buffer
        │   • stamp FootClass+0x640 (last_path_frame), +0x644 (last_dest), +0x648 (retry_wait=0)
        │
        │  ── recursive case: convoy chain ──
        └─→ FootClass__Find_Path (self, recursive)   ← for each convoy follower at FootClass+0x6C8

  side helpers:
  ├─→ FootClass__Find_Nearby_Passable_Cell (0x56DC20)  ← destination remap (47 xrefs)
  ├─→ Pathfinding_update_continued (0x481810)          ← cell-step utility (50 xrefs, NOT a repath)
  └─→ FUN_00500200 (0x500200)                          ← non-player scatter dispatch
```

### PathfinderClass lifecycle (singleton)

```
game startup (0x0040AFA5)
  └─→ PathfinderClass__Constructor (0x42A6D0)
        • singleton at 0x0087E8B8
        • allocates:
          - bridge_closed_list (+0x0C): 1.5MB heap buffer
          - ground_closed_list (+0x10): 1MB heap buffer
          - node_pool_heap (+0x14): 10000-node × 16-byte pool + header
          - zone_node_pool (+0x64): 160000-byte secondary pool
          - open_set_heap (+0x68): 65536-cell min-heap + header
        • zeros 3 PathfinderHeapVec sub-structs at +0x74, +0x8C, +0xA4
          (each: vtable, data_ptr, capacity, init_flag, ownership, count, growth)
        • initializes epoch (+0x28) = 0xFFFFFFFF, increments per Reset

per-search:
  └─→ PathfinderClass__Reset (0x42A5B0)
        • epoch (+0x28) += 1
        • on epoch wrap (0xFFFFFFFF → 0): full sweep clears all closed-set entries
        • old entries persist; epoch mismatch silently invalidates them — O(1) reset

per-tick (called by AStar_pathfind_search):
  └─→ PathfinderClass__UpdateBridgePassability (0x42ACF0)
        • scans peer FootClass instances for queued movement paths
        • XOR-toggles CellClass+0x140 bit 0x40000 on cells along peer paths (×4 cost)
        • 5×5 occupied-cell fallback if no peer paths found
        • gated by Pathfinder+0x03 (enable) and +0x3C (urgency)
        └─→ FUN_0042B080 (FindNearbyBridgePeer) — 5×5 peer scan
```

### PathfinderHeapVec (3 instances at +0x74/+0x8C/+0xA4)

24-byte struct used as per-level exclusion vector for `InvalidateZoneEdge`.
Methods:
- `PathfinderHeapVec__Init` (FUN_0042DC50): zero-cap / external-buf / heap-alloc init
- `PathfinderHeapVec__Clear` (FUN_0042D540): conditional free + zero count/capacity
- `PathfinderHeapVec__Push` (FUN_0042D830): append at data[count], no dedup
- `U16Vec__Constructor` (FUN_0042DD60): companion 10-cap u16 vector used by `UpdateHierarchicalEdges`

---

## State machine — PathfinderClass per-search states

| State | Trigger | Mutations |
|---|---|---|
| `idle` | Game startup | Constructor allocates buffers; `hierarchy_valid` (+0x38) = 1 |
| `precheck-pending` | `AStar_pathfind_search` call | `Reset` invoked; epoch++; closed lists implicitly cleared by epoch trick |
| `precheck-active` | `Zone_precheck` running | min-heap accumulating zone-level Dijkstra nodes; zone path written to `+0xBC + level*1000` |
| `astar-active` | `AStar_main_loop` running | open set + closed set fill; node pool count rises; iteration counter `local_34` increments toward cap 10000 |
| `astar-success` | goal node popped | reconstruct → smooth → optimize → output to `DAT_0089A2D8` |
| `astar-failure` | open set empty OR iter cap | `UpdateHierarchicalEdges` invoked → invalidates failing zone edge → retry loop |
| `hierarchy-degraded` | `InvalidateZoneEdge` finds no actionable edge | `+0x38 = 0` cleared → next retry skips zone precheck, runs flat A* |
| `bridge-passability-marking` | `AStar_pathfind_search` pre-pass | XOR-toggles `CellClass+0x140 & 0x40000` on peer-occupied cells (×4 cost during search; restored after) |

Critical invariants:
- `epoch` (+0x28) wrap is the ONLY time a full closed-list sweep runs; otherwise reset is O(1).
- `hierarchy_valid` (+0x38) clears when zone path is unrecoverable, forces fallback to flat A* on the next retry — this is the graceful-degradation path.
- The 24-step path ring at `FootClass+0x5E0` is the only path output consumed by the locomotor; the in-memory output at `DAT_0089A2D8` is overwritten on every search.

---

## INI surface

The pathfinder reads only a handful of `[General]` constants via the
`RulesClass` instance:

| RulesClass offset | Field (proposed) | Stock YR value | Effect |
|---|---|---|---|
| `+0x1718` | `CloseEnough` | 576 leptons (≈2.25 cells) | Distance threshold for Find_Path to abort early when unit is already close to dest (Euclidean) |
| `+0x768` | `TrackedSpeedCliff` (down) | ? | Speed multiplier for tracked unit descending cliff (consumed indirectly via Get_Slope_Speed_Factor) |
| `+0x770` | `TrackedSpeedCliff` (up) | ? | |
| `+0x778` | `WheeledSpeedCliff` (down) | ? | |
| `+0x780` | `WheeledSpeedCliff` (up) | ? | |
| `+0x1724` | `BridgeStuckOverride` | ? | Bridge-stuck override; flagged YELLOW in `STUCK_DETECTION_SYNTHESIS.md` |

The bulk of pathfinder configuration is hardcoded:
- `MAX_NODES_ITERATION_LIMIT = 10000` (decode: `local_34 != 10000` in
  AStar_main_loop)
- `MAX_PATH_STEPS = 24` (path ring buffer at FootClass+0x5E0)
- `CLOSED_LIST_REOPEN_TOLERANCE = 1.009f` (constant at 0x007E37C0)
- `BRIDGE_FLANK_MULT_NONE = 10.0f`, `_ONE = 1.0f`, `_BOTH = 2.0f` (0x007E37BC ff.)
- `SEARCH_MARKER_COST_MULT = 4.0f` (0x007E37BC)
- `RETRY_LIMIT_TARGETED = 4`, `_SCATTER = 5`
- `DIRECTION_EPSILONS = [0.001, 0.005, 0.002, 0.006, 0.003, 0.007, 0.004, 0.008]` (0x0081872C)
- `OPTIMIZE_WINDOW_STEPS = 20`

No per-unit INI keys are read directly by the pathfinder; per-unit constraints
flow in via the `MovementZone=`, `SpeedType=`, and `Locomotor=` keys parsed by
`TechnoTypeClass` (covered in `MOVEMENT_CLASSIFIERS_REFERENCE.md`).

---

## Observable behaviors

These are the player-visible outputs of the pathfinder. The parity bar
(`CLAUDE.md`) is that each input must produce identical output between
gamemd and the Rust port.

1. **Unit moves from current cell to destination cell** via a sequence of
   8-direction waypoints (the 24-step ring buffer). Visible whenever the
   player issues a move order.
2. **Destination redirect when blocked**: if the destination is water (for
   land units), occupied by a building, or otherwise impassable, the move
   target is silently redirected to the nearest passable cell via
   `Find_Nearby_Passable_Cell`. Visible when player right-clicks an impassable
   cell.
3. **Cross-zone path through gating choke point**: hierarchical search picks
   a "corridor" of zone edges before per-cell A*; visible when a unit takes
   a longer detour around an impassable terrain feature.
4. **Path-failure stop**: when A* exhausts retries (4 or 5 attempts), the
   unit calls `vtable+0x3C8 (Stop)` and remains in place. Visible when
   player commands movement to an unreachable cell.
5. **Path-failure scatter** for non-player units: AI/enemy units that fail
   to path call `vtable+0x1E8 (Scatter)` in multiplayer. Visible as
   enemy AI units shuffling around when they can't reach their target.
6. **Convoy follower re-path**: when the leader of a convoy chain completes
   a path, recursive `Find_Path` calls re-path each follower. Visible in
   convoy scenarios (TS map triggers).
7. **Path smoothing**: cardinal-then-diagonal zigzags are replaced by single
   diagonal steps; long straight runs are collapsed. Visible as cleaner unit
   trajectories vs. raw A* output.
8. **Peer-path bias**: nearby moving units' paths get a ×4 cost penalty
   (0x40000 marker), biasing A* away from cells other units will traverse.
   Visible as units spreading out around bottlenecks.
9. **Repath throttling**: a unit's `last_path_frame` (+0x640) and
   `path_retry_wait` (+0x648) gate how often `Find_Path` may be called.
   Visible only indirectly via the unit's responsiveness to repeated
   move-cancel orders.

---

## Edge cases / known parity hazards

Coordinate conventions and direction encodings per CLAUDE.md "Coordinate
conventions" section:

- **Lepton-to-cell floor** uses sign-correct arithmetic: `(x + (x >> 31 & 0xFF)) >> 8`. Forgetting the floor-correction term flips negative coords by one cell. Rust's `Position` separates rx/sub_x so this is implicit (proven equivalent).
- **Direction byte encoding** (from `g_DirTable @ 0x00818760`): `0=S, 1=SW, 2=W, 3=NW, 4=N, 5=NE, 6=E, 7=SE`. This is clockwise from south. Note: this is different from the facing-byte convention (clockwise from north, 0x00=N, 0x40=E, 0x80=S, 0xC0=W) used by locomotors. Path-direction-byte is NOT a facing byte.
- **Direction 8** = tube edge. Tubes are TS-legacy; in stock YR no live unit emits direction 8 in a real path. The code paths for direction-8 in `Path_smooth_corners` (exclusion), `Path_optimize_straight_segments` (state reset), and `Pathfinding_update_continued` (no-op guard) all exist defensively.
- **Bridge cell coordinate** lives in two layers: ground-level cell at the bridge column AND bridge-deck cell. `AStar_create_node`'s height-step logic chooses which layer to use based on `parent_height`. The transition from ground to bridge requires `abs(neighbor_level - parent_height) <= 1` per gamemd; Rust currently requires exact `diff == 4` plus transition flag (DRIFT row 40 in parity report).
- **Foundation-relative offsets** do NOT apply at the pathfinder layer — the pathfinder operates on raw cell indices. Foundation-relative is for building placement (see `BUILDING_*` docs).
- **Packed cell index 0** is used by gamemd as an "invalid destination" sentinel. Rust treats cell (0,0) as a normal valid cell on small maps, which produces a DRIFT (parity row 17).
- **Iteration cap 10000** is the gamemd value (verified in fn-astar_main_loop.md). Rust's `MAX_SEARCH_NODES = 65527` with a comment claiming "Original engine uses 65,527 (0xFFF7)" is **wrong** — this is the most significant parity bug in the system (parity row 28).

---

## Parity verdict summary

From `_parity.md` (84 rows across 33 functions + 1 struct):

| Verdict | Count | Notes |
|---|---|---|
| `MATCH` | 32 | A* base structure, retry counts, post-process pipeline, dir epsilons, hierarchical zone gate, +6 alternate margin, heap algorithms |
| `INTERNAL-ONLY` (proven) | 12 | Lepton-to-cell floor (algebraic proof), PathfinderClass singleton vs thread-local (functional equivalence), heuristic scale (1000× scaling proof), reconstruction format (cell vs direction-byte equivalent), heap implementation choice |
| `DRIFT` | 32 | Iteration limit 10000 vs 65527; closed-list 1.009 tolerance missing; bridge layer ascend rule; Find_Nearby_Passable_Cell candidate selection + radius + filters; Zone_precheck slope contribution + cost table; Path_optimize drift algorithm (Chebyshev vs cross-product); EstimateZoneCost formula |
| `MISSING` | 8 | Convoy chain re-path; last_path_frame stamps; path-failure scatter (non-player); UpdateBridgePassability peer-marking + 5×5 fallback; InvalidateZoneEdge direct edge + common-neighbor + hierarchy-valid flag; Path_walk_directions_to_cell pre-pass |

### Top parity hazards by player-visibility × frequency

1. **Iteration limit 10000 vs 65527** (DRIFT, parity row 28). Fires on any
   sufficiently-convoluted path. Rust finds routes that gamemd would
   abandon as unreachable — observable as "Rust unit moves where gamemd
   unit stops." Frequency: rare in single-skirmish, but every long path is
   a candidate. **Fix priority: HIGH.**
2. **Find_Nearby_Passable_Cell candidate pool** (DRIFT, parity rows 71-74).
   Five sub-DRIFTs: square vs diamond ring, no randomization, no
   closest-to-target selection, missing passability filters, missing
   height adjustment. Fires on every blocked-destination move command —
   common. **Fix priority: HIGH.**
3. **Close-enough distance metric** (DRIFT, parity row 6). Manhattan vs
   Euclidean for the stop-distance check. Fires on every blocked path with
   a diagonal goal. Visible as units stopping too early/late on diagonal
   approaches. **Fix priority: HIGH.**
4. **UpdateBridgePassability peer-path marking** (MISSING, parity rows
   47-48). Entirely absent in Rust. Affects dense-traffic scenarios:
   units don't bias away from peer paths, leading to more collisions
   and blocking. Frequency: high (any multi-unit movement). **Fix priority: HIGH.**
5. **Bridge layer ascend condition** (DRIFT, parity row 40). gamemd
   `abs(diff) ≤ 1`, Rust `diff == 4 AND transition`. Affects bridge
   approaches from flat terrain. Frequency: any bridge map. **Fix priority: MEDIUM.**
6. **Closed-list 1.009 tolerance** (DRIFT, parity row 29). Rust uses hard
   bool. Affects path tie-breaks in dense cost landscapes. Frequency:
   moderate; visible on paths with multiple near-equal cost routes.
   **Fix priority: MEDIUM.**
7. **InvalidateZoneEdge surface** (MISSING + DRIFT, parity rows 56-61).
   Hierarchy degradation path entirely missing in Rust — units running
   into unreachable destinations exhaust all 5 retries in corridor mode
   where gamemd would degrade to flat A* sooner. **Fix priority: MEDIUM.**
8. **Path_optimize drift detection algorithm** (DRIFT, parity row 53).
   Chebyshev vs cross-product. Different curved paths get straightened.
   Frequency: long paths with mild curves. **Fix priority: LOW** (paths
   are still valid, just different shape).
9. **Convoy chain re-path** (MISSING, parity row 12). No convoy data
   structure in Rust. Frequency: only TS map triggers (not relevant in
   stock skirmish). **Fix priority: LOW.**

---

## Per-symbol doc index

### Entry + bridge
- [fn-find_path.md](fn-find_path.md) — `FootClass__Find_Path @ 0x004D3920`
- [fn-run_astar.md](fn-run_astar.md) — `FootClass__Run_AStar @ 0x004CBBA0`

### A* core
- [fn-astar_pathfind_search.md](fn-astar_pathfind_search.md) — `AStar_pathfind_search @ 0x0042C900`
- [fn-astar_main_loop.md](fn-astar_main_loop.md) — `AStar_main_loop @ 0x00429A90`
- [fn-astar_compute_edge_cost.md](fn-astar_compute_edge_cost.md) — `AStar_compute_edge_cost @ 0x00429830`
- [fn-astar_create_node.md](fn-astar_create_node.md) — `AStar_create_node @ 0x0042A460`
- [fn-astar_reconstruct_path.md](fn-astar_reconstruct_path.md) — `AStar_reconstruct_path @ 0x0042AA90`

### PathfinderClass (singleton)
- [struct-pathfinder_class.md](struct-pathfinder_class.md) — `PathfinderClass` struct (~3200B, 25 fields)
- [fn-pathfinder_constructor.md](fn-pathfinder_constructor.md) — `PathfinderClass__Constructor @ 0x0042A6D0`
- [fn-pathfinder_reset.md](fn-pathfinder_reset.md) — `PathfinderClass__Reset @ 0x0042A5B0`
- [fn-pathfinder_update_bridge_pass.md](fn-pathfinder_update_bridge_pass.md) — `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0`
- [fn-pathfinder_update_hier_edges.md](fn-pathfinder_update_hier_edges.md) — `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`
- [fn-pathfinder_invalidate_zone_edge.md](fn-pathfinder_invalidate_zone_edge.md) — `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80`
- [fn-fn_42b080.md](fn-fn_42b080.md) — `PathfinderClass__FindNearbyBridgePeer @ 0x0042B080` *(proposed rename)*
- [fn-fn_42d170.md](fn-fn_42d170.md) — `PathfinderClass__EstimateZoneCost @ 0x0042D170` *(proposed rename)*

### Zone precheck
- [fn-zone_precheck.md](fn-zone_precheck.md) — `Zone_precheck @ 0x0042C290`
- [fn-minheap_siftdown.md](fn-minheap_siftdown.md) — `MinHeap__SiftDown @ 0x0042DCA0`

### Post-processing
- [fn-path_smooth_corners.md](fn-path_smooth_corners.md) — `Path_smooth_corners @ 0x0042B210`
- [fn-path_smooth_single_segment.md](fn-path_smooth_single_segment.md) — `Path_smooth_single_segment @ 0x0042B420`
- [fn-path_optimize_straight_segments.md](fn-path_optimize_straight_segments.md) — `Path_optimize_straight_segments @ 0x0042B7F0`
- [fn-path_find_split_anchor.md](fn-path_find_split_anchor.md) — `Path_Find_Split_Anchor @ 0x0042BCA0`
- [fn-path_reroute_straight_line.md](fn-path_reroute_straight_line.md) — `Path_Reroute_Straight_Line @ 0x0042BE20`
- [fn-mapclass_get_slope_cost_at_cell.md](fn-mapclass_get_slope_cost_at_cell.md) — `MapClass__Get_Slope_Cost_At_Cell @ 0x0056BCD0`

### Coordinate helpers
- [fn-path_walk_directions_to_cell.md](fn-path_walk_directions_to_cell.md) — `Path_walk_directions_to_cell @ 0x00429780`
- [fn-mapcoord_add.md](fn-mapcoord_add.md) — `MapCoord_Add @ 0x0042D510` *(proposed rename)*
- [fn-mapcoord_set.md](fn-mapcoord_set.md) — `MapCoord_Set @ 0x0042D470` *(proposed rename)*
- [fn-mapcoord_step_by_direction.md](fn-mapcoord_step_by_direction.md) — `MapCoord_Step_By_Direction @ 0x0042D490` *(proposed rename)*

### Destination-snap + misc
- [fn-find_nearby_passable_cell.md](fn-find_nearby_passable_cell.md) — `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`
- [fn-pathfinding_update_continued.md](fn-pathfinding_update_continued.md) — `Pathfinding_update_continued @ 0x00481810` *(misnomer — actually a cell-stepping utility, NOT a repath function)*
- [fn-fn_500200.md](fn-fn_500200.md) — `0x00500200` *(non-player scatter dispatch helper)*

### PathfinderHeapVec (24-byte sub-struct, 3 instances on PathfinderClass)
- [fn-fn_42d540.md](fn-fn_42d540.md) — `PathfinderHeapVec__Clear @ 0x0042D540` *(proposed rename)*
- [fn-fn_42dc50.md](fn-fn_42dc50.md) — `PathfinderHeapVec__Init @ 0x0042DC50` *(proposed rename)*
- [fn-fn_42d830.md](fn-fn_42d830.md) — `PathfinderHeapVec__Push @ 0x0042D830` *(proposed rename)*
- [fn-fn_42dd60.md](fn-fn_42dd60.md) — `U16Vec__Constructor @ 0x0042DD60` *(proposed rename)*

### Parity report
- [_parity.md](_parity.md) — 84 disparity rows: 32 MATCH, 12 INTERNAL-ONLY, 32 DRIFT, 8 MISSING

---

## References

### Existing related research (consult; verified-from-binary in this pass)

- `INDEX_PATHFINDING_LOCOMOTION.md` — top-level index of pathfinding/locomotion research
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` — earlier A* analysis (superseded by per-symbol decodes here)
- `PATHFINDERCLASS_GHIDRA_REPORT.md` — earlier PathfinderClass analysis
- `MOVEMENT_CLASSIFIERS_REFERENCE.md` — MovementZone, SpeedType, ZoneType, LandType enums
- `STUCK_DETECTION_SYNTHESIS.md` — repath triggers + stuck state machine
- `ZONE_PASSABILITY_VERIFIED.md` — 13×8 passability matrix
- `BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md` — dual-layer A* on bridges
- `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` — FootClass-side pathfinding interface

### Out-of-scope dependencies (cited but covered elsewhere)

See `_manifest.yaml` `excluded-out-of-scope` for the full list. Notably:
- `MapClass__Get_CellClass`, `MapClass__GetZoneID`, `ZoneMap__CellToZoneIndex` → map/cell system
- `Zone_Estimate_Slope_Cost` → slope/zone system
- `FootClass__Get_Slope_Speed_Factor` → locomotor system
- `MapClass__ResolvePathCoord_BridgeAware` → bridge system
- Locomotor `Process_Movement` overrides (Drive/Ship/Walk/Hover/Jumpjet) → locomotor docs

---

## Next steps (user decision)

1. **Rank the 9 top parity hazards** above and pick the first to fix. Recommended starting point: parity row 28 (iteration limit 10000 vs 65527) — single-constant fix, high player-visibility, clearly contradicts the Rust comment.
2. **Feed `_system.md` + `_parity.md` to `/brainstorm`** for a parity-closing design spec.
3. **Run `/write-plan`** directly if a specific DRIFT is already scoped (e.g., the iteration cap fix).
4. **`/decode-system pathfinding --resume`** later if scope-explorer Phase 1 surfaces new symbols during implementation (currently scope is at 34/52, with 18 slots of headroom).
5. **Optional**: rerun with `--verify` to dispatch `/verify-doc-swarm` over the 33 per-symbol docs as an extra audit layer (the proofer already covered this; Step 7 is opt-in).
