# Zone Graph Adjacency Emission Order -- Ghidra Research Report

**Address(es):** `0x00567110`, `0x00581F90`, `0x005824A0`, `0x00582D70`, `0x00584550`, `0x0042C290`  
**Investigation Mode:** exhaustive-slice  
**Target question:** What writer-side order does gamemd use when building hierarchical zone graph adjacency arrays, and what final edge emission order can affect `Zone_precheck` equal-cost heap/insertion ties?  
**Claimed Scope:** full hierarchy build level order, full-build zone discovery order, temporary adjacency bucket insertion/dedup order, final directed edge append order, bridge/tube insertion position relative to scanline edges, and the equivalent final emission loop in the incremental rebuild sibling.  
**Non-goals:** consumer-side `Zone_precheck` algorithm already covered by `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`; full cell A* behavior; exact runtime frequency of ties; full multi-mutation duplicate lifecycle beyond the verified writer loops; Rust implementation.  
**Confidence:** High for full-build and final-emission ordering; Medium-High for incremental sibling ordering; Medium for player-visible frequency.  
**Active in YR:** Yes. Full map initialization calls `ZoneMap__BuildZoneLevel` for levels `2,1,0` at `0x005671F7..0x00567218`; normal pathfinding reaches `Zone_precheck` from `AStar_pathfind_search` at `0x0042CB58` and `0x0042CCB3`.

## Evidence Needed To Mark COMPLETE

- Decompile plus assembly evidence for full hierarchy build level order.
- Decompile plus assembly evidence for temporary connection graph bucket traversal and final edge append order.
- Decompile plus assembly evidence for temp-edge insertion/dedup ordering and bridge/tube insertion position.
- Decompile plus assembly or xref evidence that the built graph is consumed by live YR `Zone_precheck`.
- Rust-facing handoff naming current ordering mismatches and acceptance scenarios.

## Stop Conditions

- Stop before consumer behavior already covered by the `Zone_precheck` report.
- Stop before broad bridge lifecycle or cell A* retry behavior not needed to answer final adjacency order.
- Stop before Rust edits; this is a research-only slice.
- Stop if ordering claims lack an address range or cited decompile/assembly evidence.

## 1. Overview

The hierarchical zone graph is not emitted in sorted zone-id order. Full build constructs levels in order `2 -> 1 -> 0`; within each level it discovers zones by row-major cell scan and scanline flood fill, stores temporary connection entries in 256 low-nibble buckets, then emits final directed edges by bucket index `0..255` and insertion order within each bucket.

For each temporary connection, the final writer appends the low-halfword endpoint's directed edge first, then the high-halfword endpoint's reverse edge. No final adjacency sort or final dedup pass exists in the verified writer. Because `Zone_precheck` preserves equal-cost heap/insertion order, this writer order is parity-relevant when multiple equal-cost edges compete.

## 2. Key Layout / Ordering Fields

| Structure | Offset / stride | Verified meaning | Active in YR |
|---|---:|---|---|
| Hierarchy level header | stride `0x18` | Per-level graph header; final graph family consumed through `DAT_0087F878 + level*0x18`. | Yes |
| Zone record | stride `0x24` | Zone block; `+0x04` edge pointer, `+0x10` edge count, `+0x18` parent, `+0x1C` reduced zone type. | Yes |
| Final edge record | stride `8` | `+0x00` neighbor zone id, `+0x04` flag dword low byte. | Yes |
| Temporary bucket | stride `0x18`, count `256` | Dynamic vector bucket selected by packed endpoint low nibbles. | Yes |
| Temporary entry | stride `0x0C` | `+0` packed pair, `+4` duplicate packed pair used by final emitter, `+8` flag dword. | Yes |

## 3. Core Logic

### 3.1 Full all-level build order is `2 -> 1 -> 0`

`FUN_00567110` is a full map zone initialization path. It initializes cell attributes, computes bridge zones, runs the per-row zone helper, then calls `ZoneMap__BuildZoneLevel` with `EDI = 2`, decrementing until negative.

Evidence:

- Decompile: `FUN_00567110` calls `MapClass__UpdateBridgeZonesHelper`, then loops `ZoneMap__BuildZoneLevel(iVar6)` with `iVar6 = 2`, `iVar6--`.
- Assembly: `0x005671F7` loads `EDI,0x2`; `0x0056720D` calls `0x00581F90`; `0x00567212..0x00567218` decrements and loops while `EDI >= 0`.
- Active in YR: Yes; this is the map zone init path, and the resulting graph is read by live `Zone_precheck`.

### 3.2 Per-level zone ids are assigned by row-major discovery, not sorted geometry

`ZoneMap__BuildZoneLevel` clears the per-level temporary graph, resets the target zone id slot in every `MapClass+0x70` cell record, creates sentinel zone `0`, and scans the `MapClass+0x70` cell array from start to end. Real zone ids start at `1` and are assigned when the scan reaches the first unassigned non-class-7 cell for that level.

The block size is `1 << (level + 1)`: level `2` uses `8x8`, level `1` uses `4x4`, and level `0` uses `2x2`. Flood-fill stays inside the current aligned block; cross-block contacts become temporary edges.

Evidence:

- Decompile: `ZoneMap__BuildZoneLevel` sets `uStack_68 = 1`, creates zone `0`, computes `iStack_2c = 1 << (level + 1)`, and advances the cell pointer linearly until the end of `MapClass+0x70`.
- Prior assembly in `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`: zone block writes at `0x00582265..0x005822AB`; stored level zone count at `0x00582332`.
- Active in YR: Yes; invoked from `FUN_00567110` full init and from rebuild paths.

### 3.3 Temporary edge buckets preserve first exact packed-pair insertion

Temporary connection entries are bucketed by endpoint low nibbles:

`bucket = ((packed_high16 & 0xF) << 4) | (packed_low16 & 0xF)`

The duplicate check compares the exact packed dword already present in that bucket. It does not canonicalize endpoints first. Therefore a reversed packed pair is a distinct temp key, and an already-present exact pair keeps its first insertion position and first flag byte.

Evidence:

- `ZoneMap__FloodFillScanline` decompile: before appending, it scans the target bucket entry array comparing `uVar10 == *puVar11` / `uVar9 == *puVar11`; append writes temp `+0`, `+4`, and `+8` only when no exact match exists.
- Bridge/tube temp insertion `FUN_00582D70`: same exact-pair scan before helper append; e.g. first pair loop compares `local_c == *puVar10`, then skips append on match.
- Assembly for nonzero flag branch, which also precedes temp insertion: `0x00582A28..0x00582A3E` and `0x00582C70..0x00582C86`.
- Active in YR: Yes; these temp buckets feed final graph emission in the full build and incremental rebuild.

### 3.4 Bridge/tube temp edges are inserted after scanline temp edges

In full `ZoneMap__BuildZoneLevel`, all scanline zones and scanline temp edges are produced first. Only after the full cell scan stores the level zone count does the function iterate active bridge records and call `FUN_00582D70`.

Material ordering consequence: bridge/tube temp entries are appended into the same 256 bucket graph after earlier scanline entries. If an exact packed pair was already present from scanline construction, the bridge/tube insert does not move it, duplicate it, or change its flag.

Evidence:

- Decompile: `ZoneMap__BuildZoneLevel` calls `ZoneMap__FloodFillScanline` during the main scan, stores `*(map+0x74+level*4)`, then loops bridge records and calls `FUN_00582D70` for active records.
- Prior assembly in `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`: bridge active test and call at `0x00582346..0x00582358`.
- `FUN_00582D70` decompile: each bridge/tube packed pair performs exact duplicate scan before append; its flag byte is zero.
- Active in YR: Yes when active bridge/tube records exist; standard maps with bridges use these records.

### 3.5 Final emission order is bucket index, temp insertion order, low-halfword edge first

After scanline and bridge/tube temp insertion, `ZoneMap__BuildZoneLevel` emits final directed edges as follows:

1. Iterate temporary buckets by memory offset `0, 0x18, 0x30, ... < 0x1800`, i.e. bucket index `0..255`.
2. For each bucket, iterate the bucket's temp vector in stored insertion order.
3. Read the duplicate packed pair from temp `+4` and the flag low byte from temp `+8`.
4. Append the first directed final edge from the packed pair's low halfword to the high halfword.
5. Append the reverse directed final edge from the high halfword to the low halfword.
6. Copy the same flag dword to both final directed edge records.

There is no final sort and no final dedup pass in this writer. The per-zone final adjacency array order is therefore the order in which that zone receives appends during global bucket traversal.

Evidence:

- Full-build assembly: `0x00582395` reads temp `+4`; `0x00582398` reads temp `+8` flag byte; `0x005823FF..0x00582402` writes first directed edge; `0x00582458..0x0058245B` writes reverse edge; `0x00582467..0x00582473` advances temp entry and loops; `0x00582479..0x00582480` advances bucket by `0x18` until `0x1800`.
- Decompile: final loop in `ZoneMap__BuildZoneLevel` uses `iStack_54 += 0x18` until `< 0x1800`, and for each entry appends to `zone[low16]`, then `zone[high16]`.
- Active in YR: Yes; this is the graph consumed by normal `Zone_precheck`.

### 3.6 Incremental rebuild uses the same final emission ordering shape

`FUN_00584550` is the local hierarchy rebuild sibling. For each level `2,1,0`, it clears the temporary graph, removes final edges for replaced zones, rebuilds the aligned block, adds active bridge/tube temp edges that touch the block, and emits final edges by the same bucket traversal and first-directed-then-reverse append shape.

Evidence:

- Decompile: `FUN_00584550` uses the same temp graph and final write pattern after local block rebuild.
- Assembly: `0x00584B55..0x00584B6B` initializes final bucket traversal; `0x00584B83..0x00584B88` reads temp packed pair/flag; `0x00584BF2..0x00584BF5` writes first directed final edge; `0x00584C3C..0x00584C52` writes reverse directed final edge.
- Active in YR: Yes per existing `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` caller audit; used by damage, overlay, building, terrain, and anim middle mutation paths. This report does not claim a full multi-mutation ordering lifecycle beyond the verified writer loop.

### 3.7 Why this affects `Zone_precheck` ties

`Zone_precheck` scans each zone record's final edge array in stored order. It inserts candidate nodes into its heap when the new cost is strictly lower than the existing best cost; equal costs do not replace existing entries. Heap bubble/sift comparisons also use strict lower-cost checks. Therefore equal-cost candidate order is inherited from final edge array order.

Evidence:

- Edge scan reads final neighbor and flag at `0x0042C53E..0x0042C540`.
- Prior `Zone_precheck` report verifies strict tie preservation at `0x0042C6A4..0x0042C6D8` and `0x0042C7C5..0x0042C835`.
- Active in YR: Yes through `AStar_pathfind_search` initial and retry precheck calls.

## 4. INI Keys

No INI key directly controls final adjacency array ordering. `MovementZone=` selects the matrix row consumed by `Zone_precheck`, but the writer-side adjacency order is produced by hierarchy build scan order, temp bucket insertion, bridge record order, and final bucket traversal.

## 5. Integration Points

| Point | Ordering role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00567110` | Full graph build order: per-row zones, then hierarchy levels `2,1,0`, then pathfinder scratch refresh. | `0x005671F2`, `0x0056720D`, `0x0056721F` | Yes |
| `ZoneMap__BuildZoneLevel @ 0x00581F90` | Per-level zone discovery, temp buckets, final edge emission. | decompile; final emit `0x00582395..0x00582480` | Yes |
| `ZoneMap__FloodFillScanline @ 0x005824A0` | First source of temp edges in scanline/flood order; exact packed-pair dedup. | decompile; flag branches `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86` | Yes |
| `FUN_00582D70` | Adds bridge/tube temp edges after scanline pass; duplicate exact pair does not move earlier entry. | decompile; helper calls `0x0058304B`, `0x005830E5`, `0x00583165` | Yes |
| `FUN_00584550` | Incremental sibling uses same final bucket traversal shape. | decompile; `0x00584B55..0x00584C52` | Yes |
| `Zone_precheck @ 0x0042C290` | Consumer scans adjacency in stored order; equal costs preserve insertion order. | `0x0042C53E..0x0042C540`; prior tie report | Yes |

## 6. Current Rust Implementation Status

Current Rust does not preserve this writer order:

- `src/sim/pathfinding/zone_map.rs:148` defines `ZoneAdjacency` as sorted neighbor lists.
- `src/sim/pathfinding/zone_build.rs:594` extracts adjacency by row-major ground-zone scan, then calls `sort_unstable()` and `dedup()` on every neighbor list.
- `src/sim/pathfinding/zone_build.rs:735` appends bidirectional adjacency immediately as discovered, then relies on later sorting/dedup.
- `src/sim/pathfinding/zone_search.rs:413` uses `BinaryHeap<Reverse<(f_cost, g_cost, ZoneId)>>`, so equal-cost graph ordering falls through tuple ordering by `ZoneId`, not gamemd's writer/heap insertion order.

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Full all-level build order | verified | `FUN_00567110` decompile; assembly `0x005671F7..0x00567218` | none |
| Full per-level zone discovery order | verified | `ZoneMap__BuildZoneLevel` decompile; prior assembly `0x0058200F..0x00582332` | exact geometry frequency of ties not measured |
| Temp bucket layout and traversal | verified | decompile; final loop `0x00582395..0x00582480`; 0x1800/0x18 buckets | none |
| Temp exact-pair duplicate behavior | verified | `ZoneMap__FloodFillScanline` and `FUN_00582D70` decompile | no whole-binary alternate temp writer census beyond scoped functions |
| Bridge/tube insertion after scanline temp edges | verified | `ZoneMap__BuildZoneLevel` decompile; bridge call after scan count store | none |
| Final directed edge append order | verified | assembly `0x005823FF..0x0058245B` | none |
| Incremental final emission shape | verified | `FUN_00584550` decompile; assembly `0x00584B55..0x00584C52` | full repeated-mutation duplicate lifecycle not exhausted |
| `Zone_precheck` order sensitivity | verified-from-prior plus spot-check | `0x0042C53E..0x0042C540`; prior strict-heap comparisons | consumer internals intentionally not re-covered |
| Rust ordering status | verified | Codegraph nodes for `ZoneAdjacency`, `extract_adjacency`, `find_zone_corridor` | tests not run; no code edits |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- What is the investigation mode? -> exhaustive-slice for writer-side hierarchy adjacency emission order.` (evidence: user scope)
- `[RESOLVED] OQ-2 -- Is the hierarchy build active in standard YR? -> yes, full map init calls `ZoneMap__BuildZoneLevel` for levels `2,1,0`.` (evidence: `0x005671F7..0x00567218`)
- `[RESOLVED] OQ-3 -- Is `Zone_precheck` active and order-sensitive? -> yes; it scans final edge arrays and equal costs preserve insertion/heap order.` (evidence: `0x0042C53E..0x0042C540`; prior report `0x0042C6A4..0x0042C6D8`)
- `[RESOLVED] OQ-4 -- Are final adjacency arrays sorted by zone id? -> no final sort or final dedup pass was found; final arrays are append-order arrays.` (evidence: `0x00582395..0x00582480`)
- `[RESOLVED] OQ-5 -- What is the all-level order? -> level `2`, then `1`, then `0`.` (evidence: `0x005671F7..0x00567218`)
- `[RESOLVED] OQ-6 -- What is the per-level zone discovery order? -> row-major cell scan with scanline flood-fill; real ids begin at 1 after sentinel 0.` (evidence: `ZoneMap__BuildZoneLevel` decompile)
- `[RESOLVED] OQ-7 -- What is temp bucket order? -> 256 buckets traversed by memory offset `0..0x17E8`, stride `0x18`.` (evidence: `0x00582479..0x00582480`)
- `[RESOLVED] OQ-8 -- What is temp entry order inside a bucket? -> dynamic-vector insertion order; exact packed duplicates skip later appends.` (evidence: `ZoneMap__FloodFillScanline` and `FUN_00582D70` decompile)
- `[RESOLVED] OQ-9 -- Is packed-pair dedup undirected? -> no; it compares exact packed dwords, so reversed endpoints are distinct temp keys.` (evidence: temp duplicate loops compare packed value directly)
- `[RESOLVED] OQ-10 -- Where do bridge/tube temp edges enter? -> after scanline/flood-fill temp edges and before final emission.` (evidence: `ZoneMap__BuildZoneLevel` decompile)
- `[RESOLVED] OQ-11 -- Does bridge/tube insertion reorder prior scanline edges? -> no; exact duplicate skips append, and new entries append to bucket tail.` (evidence: `FUN_00582D70` duplicate loops)
- `[RESOLVED] OQ-12 -- What final direction order is emitted per temp entry? -> packed low halfword source first, packed high halfword reverse second.` (evidence: `0x005823FF..0x0058245B`)
- `[RESOLVED] OQ-13 -- Does incremental rebuild use the same final emitter shape? -> yes, same bucket traversal and first/reverse directed writes in `FUN_00584550`.` (evidence: `0x00584B55..0x00584C52`)
- `[RESOLVED] OQ-14 -- Does current Rust preserve this order? -> no; it sorts/dedups neighbor lists and uses tuple heap ordering.` (evidence: `zone_map.rs:148`, `zone_build.rs:594`, `zone_search.rs:413`)
- `[DEFERRED] OQ-15 -- How often do stock maps produce equal-cost ties where this order changes the chosen chain?` (category: needs-runtime-debugger; reason: static writer/consumer order is verified, frequency needs runtime or map-corpus instrumentation; next-step-if-pursued: log equal-cost `Zone_precheck` candidate insertions during stock map loads and path requests)
- `[DEFERRED] OQ-16 -- Full duplicate lifecycle after many incremental edits and bridge repairs/destructions.` (category: bounded-cost-too-high; reason: scoped writer loops are verified, but repeated mutation/removal interleavings need a separate lifecycle trace; next-step-if-pursued: trace `RemoveBridgeZoneEdges` plus `FUN_00584550` on bridge collapse/repair sequences)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Full hierarchy graph builds levels `2 -> 1 -> 0`; each level assigns real zone ids by row-major first-discovery after sentinel zone `0`. Active in YR: Yes. | `0x005671F7..0x00567218`; `ZoneMap__BuildZoneLevel` decompile | missing: Rust has one movement-zone adjacency graph, no 8/4/2 hierarchy levels. | `src/sim/pathfinding/zone_build.rs`, `zone_map.rs`, future hierarchy data. | If exact `Zone_precheck` parity is implemented, build separate level graphs in binary level order and preserve first-discovery zone ids. | Two disconnected blocks discovered in map scan order get zone ids matching first encountered cell, not sorted by centroid or component size. Proposed test name: `zone_hierarchy_assigns_zone_ids_by_scanline_first_discovery`. | Do not compact/sort hierarchy zones after build if those ids feed tie order. |
| Final adjacency arrays are append-order arrays from temp bucket index `0..255`, temp insertion order, and per-entry low-halfword edge before high-halfword reverse. Active in YR: Yes. | `0x00582395..0x00582480`; incremental sibling `0x00584B55..0x00584C52` | mismatch: `extract_adjacency` sorts and dedups every neighbor list. | `src/sim/pathfinding/zone_map.rs::ZoneAdjacency`, `src/sim/pathfinding/zone_build.rs::extract_adjacency`. | Store neighbors in binary emission order for parity mode; if dedup is needed, dedup at temp exact-pair stage, not by sorted neighbor list. | A graph with neighbors discovered as `[5,2,4]` in temp emission order remains `[5,2,4]` for zone search. Proposed test name: `zone_graph_preserves_temp_bucket_neighbor_emission_order`. | Do not call `sort_unstable()` on final adjacency lists used by parity `Zone_precheck`. |
| Temp duplicate suppression is exact packed-pair equality; first exact pair keeps its position and flag, while reversed endpoints are distinct temp keys. Active in YR: Yes. | `ZoneMap__FloodFillScanline` duplicate loops; `FUN_00582D70` duplicate loops | missing: Rust canonical `ZoneEdge`/BTreeSet-style handling collapses undirected pairs in search/retry contexts. | future hierarchy builder temp graph; `src/sim/pathfinding/zone_search.rs` only for retry exclusions. | Separate builder temp-edge ordering from retry exclusion canonicalization: builder temp pairs are oriented/exact, retry exclusions are sorted undirected. | Insert temp pair `A<<16|B`, then `B<<16|A`; builder treats them as distinct temp records unless an exact duplicate already exists. Proposed test name: `zone_build_temp_edges_dedup_exact_packed_pair_not_undirected`. | Do not reuse retry `ZoneEdge::new(min,max)` as the build temp graph key. |
| Bridge/tube temp edges are appended after scanline temp edges and do not reorder an existing exact pair. Active in YR: Yes. | `ZoneMap__BuildZoneLevel` bridge loop after scan; `FUN_00582D70` decompile | partial/mismatch: Rust `inject_bridge_adjacency` runs after extraction but final sorting erases insertion position. | `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency`. | Bridge adjacency should append in record order only after scanline temp edges, and preserve prior exact-pair position if duplicate. | A scanline edge and bridge edge hit the same exact packed pair; final order/flag remains from scanline first insertion. Proposed test name: `zone_build_bridge_temp_edges_do_not_reorder_existing_scanline_pair`. | Do not sort bridge-derived neighbors ahead of scanline-derived neighbors. |
| `Zone_precheck` equal-cost choice inherits final adjacency insertion order, not `ZoneId` ordering. Active in YR: Yes. | `0x0042C53E..0x0042C540`; prior strict heap evidence `0x0042C6A4..0x0042C6D8` | mismatch: `find_zone_corridor` uses `BinaryHeap<Reverse<(f_cost, g_cost, ZoneId)>>`. | `src/sim/pathfinding/zone_search.rs::find_zone_corridor`, future exact `Zone_precheck`. | Use stable insertion sequence or binary heap semantics that do not introduce zone-id tiebreaking. | Two equal-cost candidate neighbors are emitted `[9,3]`; path chooses through `9` first despite `3` being lower id. Proposed test name: `zone_precheck_equal_cost_uses_emission_order_not_zone_id`. | Do not use tuple `(cost, zone_id)` ordering for parity heap ties. |

### Stale Docs / Follow-up Docs

No directly contradictory stale document was found. Suggested additive wording for `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` open question:

> Writer-side order resolved by `ZONE_GRAPH_ADJACENCY_EMISSION_ORDER_GHIDRA_REPORT.md`: final adjacency arrays are emitted by temp bucket index `0..255`, temp insertion order, and low-halfword directed edge before high-halfword reverse; they are not sorted by zone id. Equal-cost `Zone_precheck` ties therefore inherit this writer order.

Suggested additive wording for `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`:

> Final edge emission order is part of the graph contract: the build walks all 256 temporary buckets by increasing bucket index, then each bucket's dynamic-vector entries in insertion order, appending low-halfword-to-high-halfword before the reverse edge. Ports should not sort final adjacency lists if they feed an exact `Zone_precheck` implementation.

## Negative Facts / Do Not Do

- Do not sort final zone adjacency lists by `ZoneId` for parity `Zone_precheck`. Evidence: final writer appends in temp bucket/insertion order at `0x00582395..0x00582480`; Active in YR: Yes.
- Do not use a `BTreeMap`, `BTreeSet`, or tuple heap key as an accidental replacement for gamemd tie order. Evidence: current Rust sorts/dedups adjacency and `Zone_precheck` equal costs preserve insertion order; Active in YR: Yes.
- Do not canonicalize builder temp graph pairs as undirected. Evidence: temp duplicate checks compare exact packed dwords; retry exclusions are the separate undirected/canonicalized structure; Active in YR: Yes.
- Do not insert bridge/tube edges before scanline edges in the hierarchy build. Evidence: `ZoneMap__BuildZoneLevel` calls `FUN_00582D70` after the scanline build and before final emission; Active in YR: Yes.
- Do not let a duplicate bridge/tube temp pair overwrite an earlier scanline temp flag or position. Evidence: `FUN_00582D70` skips append on exact pair match; Active in YR: Yes.

## Remaining Uncertainty

- Runtime frequency and player-visible impact of equal-cost ties from this ordering were not measured.
- Full duplicate/removal behavior after many incremental mutations was not exhausted; this report verifies the writer loops and final emission order, not every long-lived lifecycle interleaving.
- Exact original human-readable names for the temp graph and flag fields remain inferential.

## Sources

- Ghidra decompiled this session: `FUN_00567110`, `ZoneMap__BuildZoneLevel`, `ZoneMap__FloodFillScanline`, `FUN_00582D70`, `FUN_00584550`, `MapClass__AddBridgeZoneEdges`, `Zone_precheck`, `AStar_pathfind_search`, `PathfinderClass__UpdateHierarchicalEdges`, `PathfinderClass__InvalidateZoneEdge`.
- Ghidra assembly contexts this session: `0x0056720D`, `0x00582398`, `0x00582402`, `0x0058245B`, `0x00582A28`, `0x00582C70`, `0x00584B55`, `0x00584B83`, `0x00584BF2`, `0x00584BF5`, `0x00584C3C`, `0x00584C52`, `0x0042C53E`, `0x0042C540`.
- Prior reports referenced: `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `ZONE_EDGE_RECORD_BYTE_PLUS_4_WRITER_SEMANTICS_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`.
- Rust scan via Codegraph: `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`.
