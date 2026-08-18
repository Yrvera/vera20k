# Bridge Zone Precheck Hierarchy Writer Order -- Ghidra Research Report

**Address(es):** `0x00567110`, `0x00581F90`, `0x005824A0`, `0x00582D70`, `0x00584550`, `0x005851B0`, `0x00584E50`, `0x0042C290`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** writer-side inputs needed by `Zone_precheck`: hierarchy level build order, per-level zone record fields, parent/coarser zone writes, final adjacency emission order, bridge/tube insertion position, direct bridge add/remove order, and current Rust-facing hierarchy/input deltas.  
**Non-Scope:** full consumer loop, full retry algorithm, cell A* route choice, exact stock-map post-collapse path trace, and bridge damage/collapse side effects.  
**Confidence:** High for static writer order and fields; Medium for exact multi-mutation duplicate incidence after repeated collapse/repair cycles; Medium for Rust deltas because no tests were run in this slot.  
**Active in YR:** Yes. The full build path reaches `ZoneMap__BuildZoneLevel` from map zone init, bridge mutate paths call add/remove helpers, and normal pathing consumes the graph through `AStar_pathfind_search -> Zone_precheck`.

## 0. Working Notes Contract

Target question: Verify writer-side inputs for gamemd `Zone_precheck` hierarchy, especially adjacency emission order, three-level record fields, parent/coarser zone fields, and bridge rebuild/collapse order effects.

Non-goals: Do not re-investigate the full `Zone_precheck` consumer loop, A* retry semantics, or cell A* route choice except where consumer reads prove writer fields are live inputs.

Evidence needed to mark COMPLETE: decompile plus assembly/xref evidence for hierarchy build order, final adjacency append order, parent/type/edge fields, bridge/tube insert/remove paths, incremental rebuild order, and current Rust surfaces affected.

Stop conditions: Stop once writer-side order is proved or bounded; record unresolved repeated-mutation/runtime route behavior as Remaining Uncertainty instead of expanding scope.

## 1. Overview

`gamemd.exe` builds a three-level hierarchical zone graph before `Zone_precheck` searches it. The graph is not a sorted `ZoneId` adjacency map. Full build constructs levels `2 -> 1 -> 0`; each level assigns zones by row-major first discovery inside aligned blocks, injects active bridge/tube temporary edges after scanline temporary edges, then emits final directed adjacency arrays by temporary bucket order and insertion order.

This order matters because `Zone_precheck` reads final adjacency arrays linearly. Equal-cost alternatives preserve writer/heap insertion order. Therefore a Rust port that wants exact bridge post-collapse route parity needs the writer order and hierarchy parent fields, not only connectivity.

## 2. Class Layout / Key Offsets

| Structure | Offset / stride | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `MapClass` cell hierarchy ids | `+0x70`, `10` bytes per cell | per-cell zone ids for levels `0..2`; consumer uses `cell*10 + level*2` | `ZoneMap__BuildZoneLevel`, `Zone_precheck` reads `DAT_0087F858 + idx*10 + level*2` | Yes |
| Hierarchy graph headers | `MapClass+0x90`, level stride `0x18` | final graph header family consumed through `DAT_0087F878 + level*0x18` | `0x0042C501`, `0x0042C547` | Yes |
| Zone record | stride `0x24` | final zone record | `ZoneMap__BuildZoneLevel`, `Zone_precheck` | Yes |
| Zone record edge pointer/count | `+0x04`, `+0x10` | final adjacency array and count, scanned in stored order | `0x0042C501..0x0042C540` | Yes |
| Zone record parent | `+0x18` | next-coarser parent zone id; level 2 parent is zero | writer `0x00581F90`; incremental parent refresh in `0x00584550`; consumer `0x0042C554` | Yes |
| Zone record reduced type | `+0x1C` | reduced zone type/cost/passability column | writer `0x00581F90`; consumer `0x0042C55C` | Yes |
| Final edge record | stride `0x08`, `+0` neighbor, `+4` flag dword | low byte of `edge+4` adds `0.001` if nonzero | writer `0x00582402`, `0x0058245B`; consumer `0x0042C540` | Yes |
| Temporary edge bucket entry | stride `0x0C` | packed pair plus third dword flag copied into final `edge+4` | `ZoneMap__FloodFillScanline`; final writer loop | Yes |

## 3. Core Logic

### 3.1 Full hierarchy build order is `2 -> 1 -> 0`

`FUN_00567110` allocates/initializes zone arrays, calls `MapClass__InitCellAttributes`, `MapClass__ComputeBridgeZones`, and `MapClass__UpdateBridgeZonesHelper`, then initializes `EDI = 2` and calls `ZoneMap__BuildZoneLevel` while decrementing `EDI`.

Evidence: decompile of `FUN_00567110`; assembly `0x005671F7` (`MOV EDI,0x2`), `0x0056720D` (`CALL 0x00581F90`), `0x00567212..0x00567218` (`DEC EDI`, loop while nonnegative). Active in YR: Yes; the graph is consumed by normal `Zone_precheck` calls from `AStar_pathfind_search`.

### 3.2 Per-level zone ids are row-major first-discovery ids inside aligned blocks

`ZoneMap__BuildZoneLevel` resets each cell's current-level id, creates sentinel zone `0`, then scans the cell array linearly. Real zone ids start at `1`. The block size is `1 << (level + 1)`, so level 2 uses 8x8, level 1 uses 4x4, and level 0 uses 2x2 aligned blocks. `ZoneMap__FloodFillScanline` stays within the current block/range; cross-block contacts become temporary edges.

Evidence: `ZoneMap__BuildZoneLevel` decompile shows `uStack_68 = 1`, sentinel zone setup, `iStack_2c = 1 << (level + 1)`, linear pointer advance by 10 bytes per cell, and `ZoneMap__FloodFillScanline` when a non-type-7 unassigned cell is encountered. Active in YR: Yes.

### 3.3 Parent/coarser fields are writer-owned and consumer-gated

For each discovered zone, `ZoneMap__BuildZoneLevel` writes `zone+0x18` as the cell's next-coarser zone id for levels 0 and 1, or `0` for level 2. It writes `zone+0x1C` as reduced zone type. `Zone_precheck` loads `zone+0x18` at `0x0042C554` and uses it for lower-level parent-path gating at `0x0042C5F0..0x0042C604`: levels 1/0 require the neighbor parent to be on the next-coarser chosen path unless the neighbor reduced type is `1`.

Evidence: writer decompile plus consumer assembly `0x0042C554`, `0x0042C5F0..0x0042C604`. Active in YR: Yes.

### 3.4 Temporary edge dedup is exact packed-pair dedup, not undirected canonicalization

`ZoneMap__FloodFillScanline` inserts temporary edges as packed pairs and scans the relevant bucket for exact equality before appending. `FUN_00582D70` uses the same exact-pair check for bridge/tube edges before calling append helper `0x0058AF80`. Reversed endpoints are distinct temporary keys; the final writer later emits both directed edges for each temporary entry.

Evidence: flood-fill duplicate loops compare `uVar10 == *puVar11` / `uVar9 == *puVar11` before append; bridge/tube loops compare `local_c == *puVar10` before append at `0x00583046` and `0x005830E0`. Active in YR: Yes.

### 3.5 Final adjacency emission order is bucket/insertion order, not sorted zone order

After scanline and bridge/tube temporary insertion, `ZoneMap__BuildZoneLevel` walks temporary buckets from offset `0` to `< 0x1800`, stepping by `0x18` for 256 buckets. Within each bucket it walks dynamic-vector entries in stored insertion order. For each temporary entry, it appends the low-halfword endpoint's directed edge first, then the high-halfword endpoint's reverse edge. There is no final sort and no final dedup pass in the verified writer.

Evidence: decompile final loop `iStack_54 += 0x18` while `< 0x1800`; assembly `0x00582395..0x00582480` reads temp packed pair/flag, `0x00582402` writes first directed `edge+4`, `0x0058245B` writes reverse `edge+4`, then advances to the next 12-byte temp entry. Active in YR: Yes.

### 3.6 Bridge/tube temp edges are added after scanline temp edges during full build

`ZoneMap__BuildZoneLevel` completes row-major scanline discovery first, stores the current level zone count, then loops active bridge records (`bridge_record+8 != 0`) and calls `FUN_00582D70(record, level)`. Therefore bridge/tube temp entries append after ordinary scanline temp entries unless an exact packed pair already exists, in which case the first entry's position/flag remains.

Evidence: `ZoneMap__BuildZoneLevel` decompile shows the bridge-record loop after the cell scan and before final temp-bucket emission; xrefs show `FUN_00582D70` called from `0x00582358` and `0x00584B2E`. Active in YR: Yes.

### 3.7 Bridge-specific edges are zero-flagged for `edge+4` low-byte purposes

Bridge/tube insertion via `FUN_00582D70` initializes the local flag byte to zero before appending temporary entries. Direct repaired-bridge insertion via `MapClass__AddBridgeZoneEdges` masks six local flag dwords with `& 0xFFFFFF00`, then writes those dwords to final `edge+4`; repaired bridge edges therefore do not get the consumer's `0.001` edge-flag penalty.

Evidence: `FUN_00582D70` decompile (`local_4 = 0` before each bridge/tube temp append); `MapClass__AddBridgeZoneEdges` decompile and assembly `0x00585361` final `edge+4` write from zero-low-byte locals. Active in YR: Yes; `MapClass__ValidateBridgeZones @ 0x0056DB70` calls add at `0x0056DBD6`.

### 3.8 Incremental rebuild uses the same final emission shape

`FUN_00584550` handles local changes around one cell. For each level `2 -> 1 -> 0`, it aligns the affected block, clears current ids, removes old final edges touching replaced zones, flood-fills replacement zones, calls `FUN_00582D70` for active bridge/tube records whose endpoints intersect the block, then emits final edges by the same temp-bucket traversal and low-halfword-first/reverse append shape.

Evidence: `FUN_00584550` decompile; assembly `0x00584B55..0x00584C52`, including first directed append at `0x00584BF2..0x00584BF5` and reverse append at `0x00584C4F..0x00584C52`. Xrefs show many live callers, including damage/overlay/building/terrain paths. Active in YR: Yes.

### 3.9 Bridge add/remove after repair/collapse mutates final arrays directly

`MapClass__RemoveBridgeZoneEdges` removes matching neighbor zone ids from final per-zone adjacency arrays and shifts the remainder left; it does not match on `edge+4`. `MapClass__AddBridgeZoneEdges` appends final bidirectional edges directly for each of three levels, six directed inserts per level, in helper-local order.

Evidence: `MapClass__RemoveBridgeZoneEdges` decompile plus xref from `MapClass__InvalidateBridgeZones @ 0x0056DAE0` through `0x0056DB3B`; `MapClass__AddBridgeZoneEdges` decompile plus xref from `MapClass__ValidateBridgeZones @ 0x0056DB70` through `0x0056DBD6`. Active in YR: Yes.

## 4. INI Keys

No INI key directly controls the hierarchy writer order.

| Key / data | Default / effect | Evidence | Active in YR |
|---|---|---|---|
| `[General] DestroyableBridges=yes` | bridge collapse can invalidate bridge zone edges in standard YR | `rulesmd.ini`; bridge collapse docs | Yes |
| `MovementZone=` | selects passability row used later by `Zone_precheck`; not a writer-order key | `CCINIClass__ReadMovementZone @ 0x00474E40`, stored to type data; consumer docs | Yes |
| Bridge/tube records | binary map/terrain-derived connectivity records feed `FUN_00582D70` and add/remove helpers | `MapClass__ComputeBridgeZones`, prior bridge zone docs | Yes |

## 5. Integration Points

| Producer / consumer | Relationship | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00567110` | full zone initialization builds all three hierarchy levels | `0x005671F7..0x00567218` | Yes |
| `ZoneMap__BuildZoneLevel` | writes per-level zone records and final adjacency | `0x00581F90` | Yes |
| `ZoneMap__FloodFillScanline` | discovers zones and scanline temp edges | `0x005824A0`, xrefs from build/incremental | Yes |
| `FUN_00582D70` | injects active bridge/tube temp edges into the current level | xrefs `0x00582358`, `0x00584B2E` | Yes |
| `FUN_00584550` | incremental writer after local map changes | xrefs from many damage/overlay/building paths | Yes |
| `MapClass__AddBridgeZoneEdges` / `RemoveBridgeZoneEdges` | direct final adjacency mutation for bridge validate/invalidate | xrefs from `0x0056DB70` / `0x0056DAE0` | Yes |
| `Zone_precheck` | reads final adjacency in stored order, parent fields, type, edge flag | `0x0042C501`, `0x0042C540`, `0x0042C554`, `0x0042C5F0` | Yes |

## 6. Current Rust Implementation Status

| Surface | Current status vs verified writer | Evidence |
|---|---|---|
| `src/sim/pathfinding/zone_hierarchy.rs` | reachability-only union-find, explicitly not the YR three-level hierarchy | lines `1..10` |
| `src/sim/pathfinding/zone_map.rs::ZoneGrid` | builds one `ZoneMap`/`ZoneAdjacency` per `MovementZone`, plus connected-component `SuperZoneMap`; no level `2/1/0` graphs or parent fields | lines `180..249` |
| `src/sim/pathfinding/zone_map.rs::ZoneAdjacency` | stores neighbor `Vec`s and can preserve caller-provided order; no edge metadata or per-level parent/type/flag fields | lines `148..177` |
| `src/sim/pathfinding/zone_build.rs::extract_adjacency` | row-major adjacency extraction with first-discovery unique edges; not the binary temp-bucket/final-emission algorithm | lines `591..625`, `727..735` |
| `src/sim/pathfinding/zone_build.rs::build_node_adjacency` | still sorts/dedups an upstream node graph; must not feed parity hierarchy order directly | lines `320..345` |
| `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency` | appends bridge adjacency after base extraction and preserves first discovery, but lacks three-level temp-bucket placement and zero edge-flag metadata | lines `628..666` |
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | current heap tie handling uses stable sequence and no `ZoneId` tie key, but cost is still centroid Manhattan and search is single-level | lines `492..518`, `519..589` |
| `src/sim/pathfinding/zone_search.rs::exclude_corridor_edges` | uses undirected edge exclusions; compatible with retry edge identity but not per-level hierarchy chain production | lines `591..601` |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Full hierarchy level order | verified | `FUN_00567110`; asm `0x005671F7..0x00567218` | none |
| Per-level zone id discovery order | verified | `ZoneMap__BuildZoneLevel` decompile | exact map fixtures not logged |
| Zone record parent/type fields | verified | writer decompile; consumer asm `0x0042C554`, `0x0042C5F0..0x0042C604` | no Rust data structure yet |
| Scanline temp edge insertion/dedup | verified | `ZoneMap__FloodFillScanline` decompile | exact rare duplicate incidence not measured |
| Bridge/tube temp insertion | verified | `FUN_00582D70`; xrefs `0x00582358`, `0x00584B2E` | exact tube coordinate branch not fully re-derived here |
| Final adjacency emission order | verified | asm `0x00582395..0x00582480` | none for static writer order |
| Incremental rebuild emission order | verified | `FUN_00584550`; asm `0x00584B55..0x00584C52` | repeated mutation lifecycle partially bounded |
| Direct bridge add/remove | verified | `MapClass__AddBridgeZoneEdges`, `MapClass__RemoveBridgeZoneEdges`; xrefs from validate/invalidate | duplicate final edge behavior after abnormal repeats not runtime-traced |
| Consumer field reads | touched-not-exhausted | `Zone_precheck` asm `0x0042C501`, `0x0042C540`, `0x0042C554`, `0x0042C5F0` | full consumer loop owned by other reports |
| Current Rust source surfaces | touched-not-exhausted | source scans and codegraph symbol search | no tests run |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is this code active in YR? -> Yes for normal pathing and bridge zone rebuilds.` (evidence: `0x0042C900 -> 0x0042C290`, `0x00581F90`, `0x00584550`; Active in YR: Yes)
- `[RESOLVED] OQ-2 -- Which function builds hierarchy levels? -> `FUN_00567110` calls `ZoneMap__BuildZoneLevel` for levels `2,1,0`.` (evidence: `0x005671F7..0x00567218`; Active in YR: Yes)
- `[RESOLVED] OQ-3 -- Are per-level zone ids sorted by id/geometry? -> No; real ids start at `1` and are assigned by row-major first discovery inside aligned blocks.` (evidence: `ZoneMap__BuildZoneLevel`; Active in YR: Yes)
- `[RESOLVED] OQ-4 -- What is the level block size? -> `1 << (level + 1)`: 8x8, 4x4, 2x2.` (evidence: `ZoneMap__BuildZoneLevel`; Active in YR: Yes)
- `[RESOLVED] OQ-5 -- What zone fields does the writer supply to `Zone_precheck`? -> edge pointer/count, parent at `+0x18`, type at `+0x1C`, and edge flag low byte at `edge+4`.` (evidence: writer decompile; consumer asm; Active in YR: Yes)
- `[RESOLVED] OQ-6 -- Does the parent field gate selected lower-level paths? -> Yes; levels 1/0 consult the next-coarser chosen marker for the neighbor parent except type `1`.` (evidence: `0x0042C554`, `0x0042C5F0..0x0042C604`; Active in YR: Yes)
- `[RESOLVED] OQ-7 -- Are final adjacency arrays sorted? -> No final sort or final dedup pass found; final arrays are append-order arrays.` (evidence: `0x00582395..0x00582480`; Active in YR: Yes)
- `[RESOLVED] OQ-8 -- Does temp dedup canonicalize undirected edges? -> No; it compares exact packed-pair values.` (evidence: `ZoneMap__FloodFillScanline`, `FUN_00582D70`; Active in YR: Yes)
- `[RESOLVED] OQ-9 -- Where do bridge/tube temp edges enter the full build? -> After scanline zone discovery and before final emission.` (evidence: `ZoneMap__BuildZoneLevel`, call at `0x00582358`; Active in YR: Yes)
- `[RESOLVED] OQ-10 -- Are bridge-specific added edges `edge+4` flagged as bridge edges? -> No; bridge helper/direct add paths write low byte zero.` (evidence: `FUN_00582D70`, `MapClass__AddBridgeZoneEdges`, `0x00585361`; Active in YR: Yes)
- `[RESOLVED] OQ-11 -- Does incremental rebuild use the same final emission shape? -> Yes; it emits from temp buckets with the same directed append order.` (evidence: `FUN_00584550`, `0x00584B55..0x00584C52`; Active in YR: Yes)
- `[RESOLVED] OQ-12 -- Are bridge add/remove helpers live? -> Yes; validate calls add, invalidate calls remove, and both validate/invalidate have bridge lifecycle callers.` (evidence: xrefs `0x0056DBD6`, `0x0056DB3B`; Active in YR: Yes)
- `[RESOLVED] OQ-13 -- Can current Rust synthesize this exactly from current one-level data? -> No; it lacks level `2/1/0` zone records, parent fields, edge flags, and temp-bucket emission order.` (evidence: Rust source scan; Active in YR: Rust delta)
- `[RESOLVED] OQ-14 -- Is old claim "Rust tie uses ZoneId" still current? -> No for `find_zone_corridor`; current `ZoneQueueEntry` uses sequence for equal-cost heap ties. Cost/hierarchy are still approximate.` (evidence: `src/sim/pathfinding/zone_search.rs:492..518`; Active in YR: Rust delta)
- `[DEFERRED] OQ-15 -- Exact stock-map route after a low bridge collapse.` (category: needs-runtime-debugger; reason: static writer order is proved but concrete map route needs logged rebuilt zone ids/chain; next-step-if-pursued: runtime trace selected stock map)
- `[DEFERRED] OQ-16 -- Full repeated collapse/repair duplicate lifecycle.` (category: bounded-cost-too-high; reason: direct add/remove order is proved but abnormal repeated mutation duplicate incidence needs a lifecycle trace; next-step-if-pursued: instrument bridge validate/invalidate edge counts)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Hierarchy has three levels built/searched `2 -> 1 -> 0`; each lower level carries parent/coarser fields used to constrain selected paths. Active in YR: Yes. | `0x005671F7..0x00567218`; `0x0042C554`, `0x0042C5F0..0x0042C604` | missing: Rust has one-level adjacency plus reachability components | `src/sim/pathfinding/zone_map.rs`, `zone_hierarchy.rs`, `zone_search.rs` | Add parity-mode hierarchy data with level records, parent ids, selected chain markers, and lower-level parent gate. | Fine-level off-corridor edge under an unchosen parent is pruned, but type-1 neighbor exception remains allowed. Proposed test name: `zone_precheck_parent_gate_prunes_off_corridor_child_edges` | Do not replace parent-gated hierarchy with connected-component reachability. |
| Final adjacency order is temp bucket `0..255`, temp insertion order, low-halfword directed edge before high-halfword reverse; no final sort. Active in YR: Yes. | `0x00582395..0x00582480`; incremental `0x00584B55..0x00584C52` | partial: `ZoneAdjacency` can preserve order, but current builders do not model temp buckets and `build_node_adjacency` still sorts | `src/sim/pathfinding/zone_build.rs`, `zone_map.rs` | Preserve binary writer order for any adjacency feeding exact `Zone_precheck`; avoid sorted/deduped neighbor lists on that surface. | Temp entries that should emit neighbors `[5,2,4]` keep that order through search. Proposed test name: `zone_hierarchy_preserves_temp_bucket_edge_emission_order` | Do not sort by `ZoneId` or use `BTreeSet` as final graph storage for parity hierarchy. |
| Bridge/tube temp edges append after scanline temp edges, exact duplicates keep first position/flag, repaired bridge direct add writes zero-low-byte edge flags. Active in YR: Yes. | `ZoneMap__BuildZoneLevel`, `FUN_00582D70`, `MapClass__AddBridgeZoneEdges`, asm `0x00585361` | partial: Rust injects bridge adjacency after extraction but has no edge flags or exact temp-pair dedup model | `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency`; future hierarchy builder | Keep bridge-derived hierarchy edges zero-flagged and appended after scanline temp edges without reordering an existing exact pair. | Scanline and bridge insertion produce the same exact pair; the scanline order/flag wins, and bridge edge does not add `0.001`. Proposed test name: `zone_hierarchy_bridge_edges_append_after_scanline_and_zero_flag` | Do not label `edge+4` as "bridge edge"; it is a zone-edge tiebreak flag. |

## Negative Facts / Do Not Do

- Do not sort final zone adjacency lists by `ZoneId` for parity `Zone_precheck`. Evidence: final writer appends from temp buckets/insertion order at `0x00582395..0x00582480`; Active in YR: Yes.
- Do not reuse retry `ZoneEdge::new(min,max)` as the hierarchy builder temp-edge key. Evidence: writer temp dedup compares exact packed pairs; retry exclusions are a separate consumer-side undirected edge contract; Active in YR: Yes.
- Do not infer `edge+4` low byte means "bridge edge." Evidence: bridge add and bridge/tube temp writers zero the low byte, while scanline boundary writers can set it; Active in YR: Yes.
- Do not synthesize lower-level search from connected components alone. Evidence: `Zone_precheck` reads parent field and selected coarser-path markers before accepting lower-level neighbors; Active in YR: Yes.
- Do not claim current Rust is still using `ZoneId` as equal-cost heap tie in `find_zone_corridor`; that old delta is stale. Evidence: current `ZoneQueueEntry` uses `sequence`; Active in YR: Rust delta only.

## Remaining Uncertainty

- Exact named-stock-map route after low bridge collapse needs runtime logging of rebuilt zone ids, final adjacency order, selected hierarchy chains, and cell A* path.
- Repeated collapse/repair duplicate behavior is bounded by direct add/remove evidence but not lifecycle-traced for abnormal repeated mutation sequences.
- The human-readable name for `edge+4` remains inferential; "zone-edge tiebreak flag" or "hierarchy-boundary edge flag" is safer than "bridge-edge flag."

## Stale Docs / Follow-up Docs

- `docs/research/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
  - Replace: "`+4` edge flags (low byte = `1 if bridge-edge`)".
  - With: "`+4` edge flag dword; `Zone_precheck` adds `0.001` when `byte(edge+4) != 0`. Verified bridge-specific writers set this low byte to zero; nonzero is a zone-edge tiebreak/hierarchy-boundary flag, not proven bridge-edge semantics."
- `docs/research/ZONE_GRAPH_ADJACENCY_EMISSION_ORDER_GHIDRA_REPORT.md`
  - Replace old Rust-status wording that says `zone_search.rs` equal-cost ties fall through to `ZoneId`.
  - With: "Current Rust `find_zone_corridor` uses stable sequence tie handling, so the older ZoneId-tie delta is stale; remaining deltas are single-level hierarchy, centroid/Manhattan cost, missing parent-gate fields, missing edge flag metadata, and non-binary temp-bucket writer order."

## Sources

- Ghidra decompiled: `FUN_00567110`, `ZoneMap__BuildZoneLevel @ 0x00581F90`, `ZoneMap__FloodFillScanline @ 0x005824A0`, `FUN_00582D70`, `FUN_00584550`, `MapClass__AddBridgeZoneEdges @ 0x005851B0`, `MapClass__RemoveBridgeZoneEdges @ 0x00584E50`, `Zone_precheck @ 0x0042C290`.
- Ghidra assembly contexts: `0x005671F7`, `0x0056720D`, `0x00582395`, `0x00582402`, `0x0058245B`, `0x00583046`, `0x005830E0`, `0x00584B55`, `0x00584BF2`, `0x00584C3C`, `0x00585361`, `0x0042C501`, `0x0042C554`, `0x0042C5F0`.
- Ghidra xrefs: `0x00581F90`, `0x005824A0`, `0x00582D70`, `0x00584550`, `0x005851B0`, `0x00584E50`, `0x0042C290`, `0x0056DB70`, `0x0056DAE0`.
- Existing reports referenced: `ZONE_GRAPH_ADJACENCY_EMISSION_ORDER_GHIDRA_REPORT.md`, `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.
- Rust source scanned: `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_hierarchy.rs`.
