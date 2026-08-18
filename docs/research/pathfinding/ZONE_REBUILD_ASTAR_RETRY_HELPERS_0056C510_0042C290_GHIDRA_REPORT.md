# Zone Rebuild And A* Retry Helpers 0056C510 0042C290 - Ghidra Research Report

**Address(es):** `0x0056C510` (`MapClass::UpdateBridgeZonesHelper`), `0x0042C290` (`Zone_precheck`), retry helper `0x0042CCD0`
**Investigation Mode:** coverage-map
**Claimed Scope:** call-boundary and Rust-facing synthesis for global/per-row zone rebuild, hierarchical precheck consumption, and retry exclusions around `0x0056C510` and `0x0042C290`.
**Non-Scope:** full `CellClass+0x4C` writer internals, full cell-level A*, full adjacency writer tie-order audit, and runtime-frequency measurement.
**Confidence:** Medium overall. The load-bearing binary facts below are high-confidence where cited to existing Ghidra reports, but this slot could not fresh-spot-check Ghidra because no Ghidra MCP namespace was exposed in the session.
**Active in YR:** Yes for the main zone rebuild/precheck/retry paths. Evidence is cited per finding.

## 1. Overview

`0x0056C510` rebuilds persistent map zone state: it tears down existing per-movement-zone arrays, flood-fills cell clusters, adds active bridge record adjacency, and builds `MapClass+0x18[row]` zone-id arrays for all 13 `MovementZone` rows. `0x0042C290` does not rebuild zones; it consumes the hierarchical zone graph and `PathfinderClass` per-search edge-exclusion vectors before or between cell-level A* attempts.

The key Rust implication is that current Rust has two different approximations: persistent `ZoneGrid` rebuilds in `Simulation::rebuild_zone_grid`, and corridor retries in `zone_search.rs` that exclude whole zones. gamemd separates these concerns: global/per-row map rebuilds are not the same operation as retry-local edge exclusions after an A* failure.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| Global | `0x0082A594..0x0082A734` | `int[13][8]` `ZonePassabilityMatrix`; row is `MovementZone`, column is reduced zone type; only value `1` passes. | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` | Yes. Direct readers include `0x0056C510` and `0x0042C290`. |
| `MapClass` | `+0x18[row]` | Per-`MovementZone` zone-id arrays built by `0x0056C510`. | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, section 2 | Yes. Rebuilt by map/bridge/building/passability mutation paths. |
| `MapClass` | `+0x68` | 4-byte-per-cell low-level zone data: class byte plus cluster id. | `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`; `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` | Yes. Source for `0x0056C510` and hierarchy rebuilds. |
| `MapClass` | `+0x70` | 10-byte-per-cell hierarchical zone data: level 0/1/2 ids plus copied cluster/height. | `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` | Yes. `Zone_precheck` reads the absolute singleton as `DAT_0087F858`. |
| Hierarchy graph | `DAT_0087F878 + level*0x18` | Per-level final zone graph consumed by `Zone_precheck`; zone record stride `0x24`, edge stride `8`. | `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`; `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` | Yes. Built at load/rebuild and consumed by normal foot pathfinding. |
| `PathfinderClass` | `+0x78 + level*0x18`, `+0x84 + level*0x18` | Retry-local sorted packed undirected edge exclusions and count. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`; `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md` | Yes. Consumed by `0x0042C290` after A* retry updates. |
| `PathfinderClass` | `+0xBC + level*1000`, `+0xC74 + level*4` | Stored chosen zone path and count from `Zone_precheck`. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` | Yes. Used by retry invalidation at `0x0042CF80`. |

## 3. Core Logic

### Persistent zone rebuild at `0x0056C510`

`MapClass::UpdateBridgeZonesHelper` is a persistent map rebuild helper, not an A* retry helper.

Verified behavior:

1. It clears the 256-bucket adjacency hash table at `MapClass+0x14`, frees 13 `MapClass+0x18[row]` arrays, and clears cluster ids in the `MapClass+0x68` cell-zone cache. Active in YR: Yes; evidence: `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`.
2. It inserts sentinel cluster/type 7 as cluster `0`, then scans all cells. Cells with type 7 or nonzero cluster id are skipped; other cells are filled by `ZoneFloodFillScanLine @ 0x0056CB90`. Active in YR: Yes; evidence: `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`.
3. It walks active bridge records and adds cluster adjacency when endpoint clusters differ. Active in YR: Yes; evidence: `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` and bridge repair/collapse reports citing `0x0056C510`.
4. It builds one zone-id array for each of 13 matrix rows. For each row, passable clusters are those with `ZonePassabilityMatrix[row][cluster_type] == 1`; non-`1` values are blocked. Real zone ids start at `2`, and `zone_ids[row][0] = 0xFFFF` is written for the sentinel. Active in YR: Yes; evidence: `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.

Important boundary: this rebuild is global/persistent map state. It is reached by scenario/load/bridge/building/passability paths. It is not the function called after a hierarchical A* cell-search miss.

### Hierarchical graph build consumed by `0x0042C290`

`Zone_precheck` consumes the three-level hierarchy graph built by the `ZoneMap__BuildZoneLevel` family, not just the 13 per-row `MapClass+0x18` arrays.

Verified behavior:

1. `ZoneMap__BuildZoneLevel @ 0x00581F90` builds levels `2`, `1`, and `0` with block sizes `8`, `4`, and `2`. Active in YR: Yes; evidence: `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`.
2. Each final zone record stores parent id at `+0x18`, reduced class/type at `+0x1C`, and final bidirectional edge records whose low flag byte is copied from temporary graph entries. Active in YR: Yes; evidence: `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`.
3. Rebuilds refresh Pathfinder scratch arrays through `FUN_0042C1C0`; graph rebuild and pathfinder array sizing are coupled. Active in YR: Yes; evidence: `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`.

### `Zone_precheck @ 0x0042C290`

`Zone_precheck` runs the hierarchical precheck and records the chosen zone chain for later retry repair.

Verified behavior:

1. It searches levels in order `2 -> 1 -> 0`; same-zone per level writes a count-one path and continues. Active in YR: Yes; evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.
2. Candidate edge acceptance is ordered: better/unvisited cost, lower-level parent-on-coarser-path gate except type `1`, passability matrix row/column equals `1`, then per-search exclusion scan. Active in YR: Yes; evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.
3. Candidate edge cost is accumulated target-zone-type base cost plus optional slope and optional `0.001` edge-flag tiebreak; no centroid Manhattan heuristic is part of the binary precheck. Active in YR: Yes; evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.
4. Equal-cost heap behavior uses strict lower-cost comparisons; equal costs preserve heap/insertion order, not `ZoneId` tuple ordering. Active in YR: Yes; evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.
5. On failure at any level, `Zone_precheck` returns `0`. `AStar_pathfind_search` treats same-zone and cross-zone initial failures differently, and the retry path can clear hierarchy and fall back. Active in YR: Yes; evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.

### A* retry helpers

`PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0` is the retry helper around this target.

Verified behavior:

1. The live caller is `AStar_pathfind_search @ 0x0042C900` after cell A* fails while hierarchy remains enabled. Active in YR: Yes; evidence: direct call at `0x0042CC79` in `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.
2. `0x0042CCD0` does not rebuild `MapClass` zones or the global hierarchy graph. It appends per-search packed undirected edge exclusions to `PathfinderClass+0x78/+0x84`. Active in YR: Yes; evidence: `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.
3. `0x0042CF80` clears the hierarchy-valid flag when the current path has fewer than two zones or the current zone is absent from the stored path. Active in YR: Yes; evidence: `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.
4. Retry exclusions are consumed by the next `Zone_precheck` call as edge skips, not as zone bans. Active in YR: Yes; evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`.

## 4. INI Keys

No INI key is read directly by `0x0056C510`, `0x0042C290`, or `0x0042CCD0` in the scoped reports.

| Key / data | Binary effect in this slice | Evidence | Active in YR |
|---|---|---|---|
| `MovementZone=` | Parsed into `TechnoTypeClass+0x5B4`; row selector for the 13x8 passability matrix and pathfinding zone reachability. | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; `ini/rulesmd.ini` has standard unit entries. | Yes. |
| `SpeedType=` | Separate movement speed/cost input; not the direct passability-matrix row selector in this slice. | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; Rust scan shows SpeedType still used for cost grids. | Yes, but not as matrix row here. |
| `BridgeDestruction=yes` / `DestroyableBridges=yes` | Makes bridge destruction/repair paths relevant, which call zone update/rebuild helpers in standard content. | `ini/rulesmd.ini`; bridge repair/collapse reports. | Conditional, default standard YR value is enabled in stock rules. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Scenario/load/init | Full hierarchy and per-row zone state are built after cell attributes are available. | `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`; `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` | Yes. |
| Bridge mutation | Bridge destroy/repair paths call `0x0056C510` directly or through dirty/rebuild sequences depending on the mutation. | `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`; `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` | Yes when bridges are present and mutated. |
| Building/overlay/local map changes | Incremental `FUN_00584550` can rebuild affected hierarchy blocks and fall back to all-level rebuild. | `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` | Yes. |
| Normal foot pathfinding | `FootClass__Run_AStar -> AStar_pathfind_search -> Zone_precheck`. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` | Yes. |
| A* retry after hierarchical-assisted cell search miss | `AStar_pathfind_search -> 0x0042CCD0 -> PathfinderClass__Reset -> Zone_precheck` if hierarchy remains enabled. | `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md` | Yes. |
| Nearby-cell fallback / alternate caller | `FUN_0042D170` calls `Zone_precheck` and can return a huge/fail distance when false. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` | Yes; exact frequency deferred. |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta versus binary slice |
|---|---|---|
| `src/sim/world/mod.rs:674` | `Simulation::rebuild_zone_grid` tries incremental update from previous `PathGrid`, then full rebuild. | Binary has distinct persistent rebuild paths and retry-local exclusion paths. Rust persistent rebuild is not a substitute for `0x0042CCD0`. |
| `src/app_sim_tick.rs:775` | `rebuild_dynamic_path_grid` rebuilds PathGrid from terrain/bridges/buildings/walls and then calls `sim.rebuild_zone_grid`. | Conservative persistent refresh, but timing differs from binary local retry repair and may lag until app-level grid rebuild. |
| `src/sim/pathfinding/zone_map.rs:217` | Builds maps for `MovementZone::all_ground()`, not all 13 rows; `can_reach` returns true for `MovementZone::Fly`. | Binary `0x0056C510` builds all 13 matrix rows including Fly row 9. |
| `src/rules/locomotor_type.rs:300` | `all_ground()` excludes Fly and includes 12 movement zones. | Useful design shortcut, but not a literal per-row rebuild match. |
| `src/sim/pathfinding/zone_build.rs:38` | Contains recovered 13x8 movement-class passability rows and terrain-aware node-index build. | Good matrix basis, but no 8/4/2 hierarchy levels, temp graph buckets, or final edge flag writer parity. |
| `src/sim/pathfinding/zone_search.rs:40` | `can_use_reduced_zone_precheck` only enables selected movement zones. | Binary precheck accepts the `MovementZone` row the caller passes; no hard-coded Rust subset was found in the binary reports. |
| `src/sim/pathfinding/zone_search.rs:241` | Corridor retry excludes whole corridor zones and retries up to 5, then falls back to unrestricted A*. | Binary retry appends edge exclusions, not zone bans, and only after hierarchical-assisted cell A* failure. |
| `src/sim/pathfinding/zone_search.rs:381` | Coarse route cost uses Manhattan center distance plus heuristic, with tuple tie ordering. | Binary uses zone-type/slope/edge-flag accumulated cost and strict heap tie behavior. |
| `src/sim/pathfinding/zone_incremental.rs:56` | Terrain-aware dynamic changes force full rebuild. | Conservative relative to current Rust model; not the same as gamemd's localized `FUN_00584550` block-level hierarchy rebuild. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x0056C510` matrix/per-row rebuild contract | verified-from-prior | `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`; `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` | Fresh Ghidra spot-check unavailable in this slot. |
| `0x0056CB90` flood-fill helper | touched-not-exhausted | `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` | Exact scanline quirks not re-audited here. |
| Hierarchy writer `0x00581F90` | verified-from-prior | `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` | Exact final adjacency insertion-order ties remain separate. |
| Incremental hierarchy writer `0x00584550` | touched-not-exhausted | `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` | Runtime fallback condition exact trigger remains low-medium in prior doc. |
| `0x0042C290` precheck consumer | verified-from-prior | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` | Fresh Ghidra spot-check unavailable in this slot. |
| Retry helper `0x0042CCD0` | verified-from-prior | `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md` | Nonzero `FloodFillReachableZones` branch frequency needs runtime data. |
| Rust persistent rebuild scan | verified | Codegraph; `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `zone_map.rs`, `zone_build.rs`, `zone_incremental.rs` | No tests run; no code edited. |
| Rust retry scan | verified | `src/sim/pathfinding/zone_search.rs` | Implementation behavior should be rechecked after current dirty pathfinding work settles. |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is this a fresh exhaustive Ghidra slice? -> No; it is a coverage-map synthesis because Ghidra MCP was not exposed. Binary facts are cited from recent Ghidra reports.` (evidence: session tool availability; cited reports)
- `[RESOLVED] OQ-2 - Is `0x0056C510` active in YR? -> Yes, through scenario/load and bridge/passability mutation paths.` (evidence: `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`; bridge reports)
- `[RESOLVED] OQ-3 - Does `0x0056C510` build all movement rows? -> Yes, all 13 matrix rows, ending at `0x82A734`.` (evidence: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-4 - Does `0x0056C510` use SpeedType as row selector? -> No, rows are MovementZone/matrix rows.` (evidence: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-5 - Does A* retry call `0x0056C510` after a failed corridor? -> No verified direct retry path does so; retry uses `0x0042CCD0` local edge exclusions.` (evidence: `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-6 - Is `0x0042C290` active in YR? -> Yes, through `FootClass__Run_AStar -> AStar_pathfind_search` and alternate `FUN_0042D170`.` (evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-7 - What does `0x0042C290` consume? -> Three-level hierarchy cell ids/graph, movement-zone passability row, and Pathfinder local edge exclusions.` (evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-8 - Are retry exclusions zones or edges? -> Undirected packed edges, not whole zones.` (evidence: `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-9 - Does Rust exclude whole zones? -> Yes, `zone_search.rs` stores `BTreeSet<ZoneId>` and extends it with every corridor zone after failure.` (evidence: `src/sim/pathfinding/zone_search.rs:241-272`)
- `[RESOLVED] OQ-10 - Does Rust rebuild persistent zones after PathGrid changes? -> Yes, `rebuild_dynamic_path_grid` calls `Simulation::rebuild_zone_grid`.` (evidence: `src/app_sim_tick.rs:831-837`; `src/sim/world/mod.rs:674-717`)
- `[RESOLVED] OQ-11 - Does Rust build exact three-level 8/4/2 hierarchy? -> No; current scan shows single per-MovementZone maps, adjacency, and super-zone cache.` (evidence: `src/sim/pathfinding/zone_map.rs:217-320`; `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-12 - Exact adjacency writer order for equal-cost precheck ties.` (category: out-of-scope; reason: requires dedicated hierarchy writer tie-order audit; next-step-if-pursued: audit `0x00581F90` final edge emission order and temp bucket traversal)
- `[DEFERRED] OQ-13 - Runtime frequency of `FUN_0042D170` and nonzero `FloodFillReachableZones` branch.` (category: needs-runtime-debugger; reason: static reachability is verified but frequency is not; next-step-if-pursued: instrument failed A* attempts in bridge-collapse and dense-blocker skirmish cases)
- `[DEFERRED] OQ-14 - Whether exact Fly row zone arrays matter for standard Fly/jumpjet routing.` (category: requires-different-system-context; reason: row 9 is built in the binary, but locomotor call paths were not traced here; next-step-if-pursued: dedicated Fly/jumpjet path-entry audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| A* retry after hierarchical-assisted cell search failure appends Pathfinder-local undirected edge exclusions and reruns precheck; it does not rebuild persistent map zones. | `0x0042CC79 -> 0x0042CCD0`; `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`; `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`. Active in YR: Yes. | mismatch: `zone_search.rs` excludes whole zones and has no verified binary-shaped per-search edge exclusion state. | `src/sim/pathfinding/zone_search.rs` retry/corridor state. | Model retry exclusions as canonical undirected graph edges local to one search; consume them in the zone precheck/corridor search without mutating `ZoneGrid`. | Graph has zones A-B-C and A-D-C; cell A* through edge A-B fails; retry skips only A-B and succeeds through A-D-C. Proposed test name: `zoned_retry_excludes_failed_edge_without_rebuilding_zone_grid`. | Do not call `rebuild_zone_grid` as the analogue of `0x0042CCD0`; it changes persistent state and timing. |
| Binary `Zone_precheck` is a three-level `2 -> 1 -> 0` hierarchy search with lower-level parent-corridor gating, not a single expanded corridor. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`; `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`. Active in YR: Yes. | missing: Rust uses one movement-zone graph plus one-ring expansion before cell A*. | `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`. | Add exact hierarchy data only if/when parity requires binary-like precheck; until then, document current corridor search as approximation. | Fine-level graph has an off-corridor tempting edge whose parent is absent from the level-2 path; binary rejects it unless zone type is `1`. Proposed test name: `zone_precheck_prunes_fine_edges_outside_parent_corridor`. | Do not claim current super-zone reachability is exact `Zone_precheck` parity. |
| `0x0056C510` builds per-row persistent zone ids for all 13 MovementZone rows using matrix value `== 1`; this is independent from retry exclusions. | `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`; `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`. Active in YR: Yes. | partial: Rust has a 13x8 movement-class table but `ZoneGrid::build_with_terrain` iterates `MovementZone::all_ground()` and `can_reach` treats Fly as universally reachable. | `src/rules/locomotor_type.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`. | Keep persistent zone rebuild semantics separate from retry repair; decide explicitly whether Fly row 9 needs a built map or documented bypass. | Build a zone grid and verify row-specific reachability for Water, WaterBeach, CrusherAll, and Fly sentinel/OOB behavior according to the matrix. Proposed test name: `zone_rebuild_preserves_all_movement_zone_row_semantics`. | Do not silently treat `MovementZone::Fly` as "all cells including sentinel/outside" if binary row 9 semantics are required. |
| Binary zone graph edge cost is zone-type/slope/edge-flag accumulated cost; equal ties preserve heap insertion order. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`. Active in YR: Yes. | mismatch: `find_zone_corridor` uses Manhattan center distance plus heuristic and tuple tie ordering. | `src/sim/pathfinding/zone_search.rs::find_zone_corridor`; future hierarchy edge metadata. | If exact precheck is implemented, feed graph search with binary zone-type cost and stable heap order, not centroids. | Two equal Manhattan corridors differ by target zone type; binary chooses the lower base-cost route. Proposed test name: `zone_precheck_uses_zone_type_cost_not_centroid_distance`. | Do not use `(cost, ZoneId)` tuple ordering as a parity-preserving heap replacement. |

### Negative Facts / Do Not Do

- Do not treat `0x0042CCD0` as a MapClass/global zone rebuild. Evidence: retry helper writes Pathfinder-local `+0x78/+0x84` exclusion vectors; Active in YR: Yes.
- Do not exclude whole zones after a failed A* corridor. Evidence: `Zone_precheck` consumes packed undirected edge pairs; Active in YR: Yes.
- Do not conflate `0x0056C510` per-row rebuild with the 8/4/2 hierarchy writer. Evidence: `0x0056C510` builds `MapClass+0x18[row]`; `ZoneMap__BuildZoneLevel` builds `DAT_0087F858`/`DAT_0087F878` hierarchy; Active in YR: Yes.
- Do not use `SpeedType` as the passability-matrix row selector. Evidence: matrix direct-reader report identifies `MovementZone` / `TechnoTypeClass+0x5B4`; Active in YR: Yes.
- Do not use centroid Manhattan route cost or whole-corridor neighbor expansion when claiming binary `Zone_precheck` parity. Evidence: `0x0042C290` cost and gate report; Active in YR: Yes.

### Remaining Uncertainty

- Fresh read-only Ghidra verification was not possible in this slot because no Ghidra MCP tools were exposed. This report relies on recent high-confidence Ghidra reports for binary evidence.
- Exact final adjacency writer order in the hierarchy graph remains out of scope; it can affect equal-cost route ties.
- Runtime frequency of the alternate `FUN_0042D170` path and the nonzero `FloodFillReachableZones` retry branch needs debugger or replay instrumentation.
- Standard Fly/jumpjet path use of binary row 9 was not traced here beyond matrix row existence and parser activity.

### Stale Docs / Follow-up Docs

No new stale-doc replacement wording was found beyond the already listed replacements in:

- `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`
- `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`
- `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`

## Sources

- Prior Ghidra reports: `docs/research/MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`
- Prior Ghidra reports: `docs/research/ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`
- Prior Ghidra reports: `docs/research/ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`
- Prior Ghidra reports: `docs/research/PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`
- Prior Ghidra reports: `docs/research/ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`
- Prior Ghidra reports: `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`
- Rust scanned with Codegraph and line reads: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_incremental.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/rules/locomotor_type.rs`
