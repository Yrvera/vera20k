# Core Service Profile — `pathfinding-helpers`

**Service:** Pathfinding helpers (A* + zone connectivity + neighbor expansion)
**Primary doc:** `docs/research/PATHFINDING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (read in full)
**Corroborating corpus:** `docs/research/pathfinding/*` (parity report, ASTAR_*, ZONE_PRECHECK_*, PATHFINDER_*), `docs/research/ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md`, `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`, `docs/research/CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`.
**Authority note:** primary study is Ghidra-verified (function identities, cost tables, slope gate live-verified 2026-06-04). This profile cites its addresses; Ghidra used only to confirm cross-service edges already named in the corpus.

---

## Purpose

The path-search **helper layer** that shapes a unit's route once `FootClass` asks for a path. It is **not** the A* spine alone — it owns the four helper families that turn raw cell/occupancy state into per-edge costs and corridor decisions:

1. **Zone-type classification** — `CellClass::RecalcZoneType` assigns each cell a 0–7 zone-type (`cell+0x4C`), the column index into the zone-passability matrix.
2. **Edge-cost evaluation** — per-neighbor A* cost from the `Can_Enter_Cell` return code (table lookup + moving-friendly "code-2" trajectory prediction + cliff-ramp ×4 + bridge-diagonal flank multipliers + direction tiebreaker + g-cost assembly).
3. **Hierarchical zone corridor** — `Zone_precheck` runs a uniform-cost (no-heuristic) Dijkstra over the zone graph, stamps a chosen-zone marker array, and `AStar_main_loop` gates each neighbor against that marker (with the `cell+0x122` blocker-refcount off-marker exception).
4. **Slope cost + bridge-peer fallback** — `Zone_Estimate_Slope_Cost` (mover-gated zone slope cost) and `FindNearbyBridgePeer` (5×5 first-eligible-object probe fallback for the bridge-passability marker pass).

In one chain: terrain change → `RecalcZoneType` (R1) → path request → `Run_AStar` → `Zone_precheck` (corridor Dijkstra + slope + marker stamp) → `AStar_main_loop` (marker gate → `Can_Enter_Cell` → edge cost → g-cost).

---

## Owns

State/logic this service is the authority for (the *behavior*, not the C++ class tree):

- **Per-cell zone-type value** `cell+0x4C` (0–7) — written by `RecalcZoneType`, read by the edge-cost/marker logic. (The cell storage is substrate-owned; the *classification value* is owned here.)
- **The A* edge-cost formula** — the single per-neighbor cost decision (`AStar_compute_edge_cost` + the g-cost assembly inside `AStar_main_loop`).
- **The hierarchical corridor** — chosen zone chain (`PathfinderClass+0xBC`/count `+0xC74`), the level-0 chosen-zone marker array (`PathfinderClass+0x40`, epoch `+0x28`), and the marker-gate decision per neighbor.
- **The zone slope-cost estimate** (`Zone_Estimate_Slope_Cost`, level 0/1/2) and the mover-gate that folds it (`ftol(raw × slope_factor)`).
- **The bridge-peer 5×5 fallback scan** (`FindNearbyBridgePeer`).
- **Neighbor expansion utilities** — 8-dir get-neighbor (`g_CellNeighborOffsets_8Dir`) and direction-step replay (`Path_walk_directions_to_cell`).

Globals/tables it reads as its private constant substrate (load-bearing, addresses verified in primary doc):

- A* base cost table `0x0081870c` = `{1.0, 1000.0, 1.0, 1.0, 60.0, 20.0, 8.0, 10000.0}`.
- Direction tiebreaker `0x0081872c` = `{.001,.005,.002,.006,.003,.007,.004,.008, 0.0}`.
- Cliff-ramp ×4 `0x007e37bc`; bridge flank `0x007e37b4`=2.0 / `0x007e2ac8`=1.0 / `0x007e37b8`=10.0.
- Cell neighbor offsets `g_CellNeighborOffsets_8Dir 0x007e3774` = `{-512,-511,+1,+513,+512,+511,-1,-513}` (map width 512).
- Zone-corridor base-cost table `0x007e3794` = `{1,0,0,1,1,0,1,1}`; corridor diag step `0x007e3818`=0.001, cardinal `0x007e2800`=0.0; cell-A* per-step `0x007e37c0`≈1.001.
- Slope mover-factor threshold `0x007e3810`; zone graphs `DAT_0087f890`/`DAT_0087f8a8`; zone-index table `DAT_0087f858`.
- Zone-passability matrix `0x0082a594` (MovementZone rows × 8 zone columns; row stride 0x20, i32 `==1`) — read here, but **shared** with zone-build / bridge-zone recompute / best-compatible-zone (see Used-by).

---

## Key functions & globals (addresses)

| Symbol | Address | Role |
|---|---|---|
| `CellClass__RecalcZoneType` | `0x00483c80` | Assign `cell+0x4C` zone type 0–7 (priority cascade) |
| `CellClass::RecalcAttributes` | `0x0047d2b0` | Master cell recalc; calls RecalcZoneType |
| `AStar_compute_edge_cost` | `0x00429830` | Per-neighbor edge cost (table + code-2 walk + cliff/bridge mult) |
| `AStar_main_loop` | `0x00429a90` | A* loop; marker gate + `Can_Enter_Cell` dispatch + g-cost |
| `AStar_create_node` | `0x0042a460` | Node alloc; Euclidean heuristic `Sqrt_Approx(dx²+dy²)` |
| `Sqrt_Approx` | `0x004cac40` | Table-based sqrt for the heuristic |
| `Zone_precheck` | `0x0042c290` | Corridor Dijkstra; marker stamp; slope fold |
| `Zone_Estimate_Slope_Cost` | `0x00585f40` | Level-aware (0/1/2) zone slope estimate (int) |
| `Math__ftol` | `0x007c5f00` | Truncate-toward-zero float→int (slope fold) |
| `AStar_pathfind_search` | `0x0042c900` | Hierarchy wrapper (same/cross-zone dispatch, retry loop) |
| `FootClass::Run_AStar` | `0x004cbba0` | A* wrapper; replays partial path then searches |
| `FootClass__Find_Path` | `0x004d3920` | Top-level pathfinding entry; sole caller of Run_AStar |
| `Pathfinding_update_continued` (get-neighbor) | `0x00481810` | `cell + dir(0–7)` → neighbor `CellClass*` |
| `Path_walk_directions_to_cell` | `0x00429780` | Replay dir-step array → destination cell (dir-8 = tube, TS-legacy) |
| `PathfinderClass::UpdateBridgePassability` | `0x0042acf0` | Writes `0x40000` peer-path marker; sole caller of FindNearbyBridgePeer |
| `PathfinderClass__FindNearbyBridgePeer` (`FUN_0042B080`) | `0x0042b080` | 5×5 first-eligible-object fallback for empty probe list |
| `FootClass__Get_Slope_Speed_Factor` | `0x004dc760` | Returns `Foot+0x530` slope mover factor |

Cost/connectivity globals: `0x0081870c`, `0x0081872c`, `0x007e3794`, `0x007e3818`, `0x007e2800`, `0x007e37c0`, `0x007e37bc`, `0x007e37b4`, `0x007e2ac8`, `0x007e37b8`, `0x007e3774`, `0x007e3810`, `0x0082a594`, `DAT_0087f858`, `DAT_0087f890`, `DAT_0087f8a8`.

---

## Tick / render position

Not a per-tick spine resident — **on-demand, called from the movement phase**. In `World::advance_tick` terms this runs inside **ground movement / air+special movement** when a locomotor needs a route:

- `DriveLocomotionClass::Process_Movement 0x004b2630` / `ShipLocomotionClass::Process_Movement 0x006a1c80` / `WalkLocomotionClass::ProcessMovement 0x0075aec0` → `FootClass__Find_Path 0x004d3920` → `Run_AStar 0x004cbba0` → `AStar_pathfind_search 0x0042c900` → `Zone_precheck` + `AStar_main_loop`.

`RecalcZoneType` (R1) runs **off the tick spine** — it fires reactively on terrain/overlay change via `CellClass::RecalcAttributes` (e.g. bridge destroy/build, wall change). No render-pass involvement (sim-only, no `render/ui/audio/net` dependency).

---

## Depends-on (outgoing edges)

Each edge: target slug — via-symbol — evidence.

- **cell-validation** — `Can_Enter_Cell` (`vtable+0x1AC`; `UnitClass 0x0073f0a0`, `InfantryClass 0x0051bf90`) returns the 0–7 code that `AStar_compute_edge_cost`/`AStar_main_loop` index into the cost table. The marker gate calls it after the level-0 marker check; `iVar17 < 7` expands, `>= 7` rejects. This is the explicit ownership seam (study §6.0, §2d). Evidence: `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md` return-code table; `ASTAR_ENTITY_COST_INTEGRATION` §2.4 (`iVar17` passed as 5th arg).

- **cell-map** — reads cell/zone/occupancy state owned by CellClass/MapClass: zone-type `cell+0x4C`, blocker-refcount `cell+0x122` (R4 off-marker exception), cell flags `cell+0x140` (`0x100` bridge, `0x40000` peer marker, `0x800` flank selector), object-list heads `cell+0xE4/+0xE8` (code-2 walk + R6 scan), level `cell+0x11B`, the zone-passability matrix `0x0082a594`, and zone-index table `DAT_0087f858`. `RecalcZoneType` is itself invoked from `CellClass::RecalcAttributes 0x0047d2b0`. Evidence: study §2c/§2b; `CELL_0x122_CAN_ENTER_CELL_SEMANTIC`.

- **lookup-tables** — consumes the static read-only path-neighbor table family: cost table `0x0081870c`, tiebreaker `0x0081872c`, neighbor offsets `0x007e3774`, corridor base table `0x007e3794`, cliff/bridge multiplier block `0x007e37b0..0x007e37bf`, slope corner/dir tables. Evidence: `docs/research/substrate/tables/PATH_NEIGHBOR_SUBSTRATE_STUDY.md`; study §2b (live bytes verified).

- **bridge-helpers** — bridge-diagonal flank multipliers (1/2/10 selected by flanking-cell bridge bits + `dest+0x140 & 0x800`); the `0x40000` cliff-ramp ×4 trigger is a marker XOR-toggled by `UpdateBridgePassability 0x0042acf0`; `FindNearbyBridgePeer 0x0042b080` is the bridge-passability probe fallback. H4/H5/R6 form one coupled bridge subsystem. Evidence: study H4/H5/§2a; `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`.

- **rules-class** — the slope mover factor `Foot+0x530` is sourced from `TechnoTypeClass+0x2F0` `ThreatAvoidanceCoefficient`, a parsed-INI gameplay tunable (RulesClass-owned type field). Crusher behavior (H3, force code→0) reads `TechnoTypeClass Crusher`. Evidence: study H3/H14, §2c (`Foot+0x530 ← TechnoTypeClass+0x2F0`); `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md`.

- **techno-foot** — reads mover (FootClass) state during search: slope context ptr `Foot+0x21C` (→ `+0x57E4` L1 array, `+0x59F0` L2 grid), slope factor via `FootClass__Get_Slope_Speed_Factor 0x004dc760`, object active/velocity/path-queue fields (`obj+0x14 bit2`, `+0x578` velocity, `+0x5E0` path_queue[0]) in the code-2 prediction walk. (FootClass is also the *caller* — see Used-by; this is the read-side coupling within the same request.) Evidence: study §2c, H2, H14.

- **ini-parsing** *(indirect/transitive)* — `ThreatAvoidanceCoefficient` and `Crusher` reach this service through RulesClass-parsed type data; the literal parse is `CCINIClass`/`INIClass`. Edge owned by rules-class, surfaced via ini-parsing. Evidence: study §10.5 (ReadDouble/`%f` narrowing boundary for the slope factor).

---

## Used-by (incoming edges)

- **techno-foot** — `FootClass__Find_Path 0x004d3920` → `Run_AStar 0x004cbba0` is the sole top-level entry. FootClass requests every path through here. Evidence: study §10.1 (get_function_callers 0x004cbba0); `fn-find_path.md`.

- **frontier-objects (locomotors)** — `DriveLocomotionClass::Process_Movement 0x004b2630`, `ShipLocomotionClass::Process_Movement 0x006a1c80`, `WalkLocomotionClass::ProcessMovement 0x0075aec0` (plus un-named `FUN_005164d0`, `FUN_005b01c0`) call `Find_Path` when a mover needs a route or repaths. (Locomotors are not yet a studied core service; tagged frontier-objects.) Evidence: study §10.1 (get_function_callers 0x004d3920); `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`.

- **cell-map** — `RecalcZoneType` (this service's R1) is invoked by `CellClass::RecalcAttributes 0x0047d2b0` on terrain/overlay change. Reciprocal with the cell-map depends-on edge (cell-map owns *when*; this service owns *what value*). The zone-passability matrix written via R1's classification is also read by `ZoneMap__FindBestCompatibleMovementZone 0x00588a11`, `MapClass__UpdateBridgeZonesHelper 0x0056c997`, `ZoneMap__FloodFillReachableZones 0x005841c9` — matrix consumers in the cell-map/zone-build family. Evidence: study §10.1 (get_xrefs_to 0x0082a594); `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.

- **bridge-helpers** — `UpdateBridgePassability 0x0042acf0` calls `FindNearbyBridgePeer` (R6) and is itself called at A* search entry/exit; the bridge subsystem both feeds and consumes this service. Reciprocal with the bridge-helpers depends-on edge. Evidence: study H4, §2a.

---

## Open / unverified edges

- **Slope-context writer (BLOCKING P0):** who fills `Foot+0x21C → +0x57E4`/`+0x59F0`. No immediate-offset store found; block is populated via computed-base loops. Until resolved, the techno-foot slope-read edge's write side is UNCHECKED. (study §9 BLOCKING; do not flip slope authoritative.)
- **`search_ctx+0x01` bridge-aware flag lifecycle (BLOCKING P0/P7):** gates H5 bridge-flank multipliers; its writer did not surface. The bridge-helpers flank-cost edge is therefore decoded-but-inert.
- **`object+0x674` subobject class / `vtable+0xA0` predicate identity (R6):** UNRESOLVED — model as "footprint accepts point at height." Affects the bridge-helpers depends-on edge's predicate detail only.
- **`g_DirectionOffsets 0x0089f688`:** static read returned all-zeros (runtime-populated or wrong slot). The operative neighbor table is `g_CellNeighborOffsets_8Dir 0x007e3774` (VERIFIED). H16's dx/dy table is UNVERIFIED; the lookup-tables edge stands on `0x007e3774`, not `0x0089f688`.
- **`AStar_pathfind_search` `What_Am_I==0xf` + COM `QueryInterface` branch:** reachability in standard YR skirmish UNCHECKED — flag, do not model (possible TS-legacy / special unit).
- **Tube/tunnel dir-8 (`Path_walk_directions_to_cell`, `cell+0x116` TubeIndex):** TS-legacy, inert in standard YR (`feedback_no_tunnel_subterranean`). Not a live edge.
- **target-scoring / damage-helpers / random-scenario:** no edge found — pathfinding helpers do not read threat scores, run the damage kernel, or consume RNG streams (A* and the corridor Dijkstra are deterministic; ties resolve by epsilon / insertion-order, not RNG).
