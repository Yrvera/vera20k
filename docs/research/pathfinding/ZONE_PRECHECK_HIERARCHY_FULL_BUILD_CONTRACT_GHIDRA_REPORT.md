# Zone Precheck Hierarchy Full-Build Contract - Ghidra Research Report

**Address(es):** `0x00567110`, `0x00581F50`, `0x00581F90`, `0x005824A0`, `0x00582D70`, `0x00584550`, `0x00483C80`, `0x0047D2B0`, `0x0042C1C0`, `0x0042C290`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Batch 1 from `docs/plans/2026-05-24-zone-precheck-production-hierarchy-builder-investigation-plan.md`: the production full-build hierarchy contract, the scanline temp-edge writer, bridge/tube temp-edge inclusion during full/incremental hierarchy builds, the `CellClass::RecalcZoneType` source value consumed by the builder, and the consumer fields `Zone_precheck` requires.  
**Non-Scope:** direct bridge repair/collapse add/remove helpers, retry-local exclusion production, full A* wrapper retry behavior, and `CellClass+0x122` blocker-neighbor count lifecycle. Those remain batches 2-4 of the plan.  
**Confidence:** High for full-build fields/order/edge flags; Medium for exact repeated incremental mutation lifecycle because this slice only spot-checks `0x00584550`; Low for stock-map route outcome because no runtime route trace was collected.  
**Active in YR:** Yes. The build path is reached from map zone init and all-level rebuild, the incremental sibling has live callers, and the graph is consumed by standard `AStar_pathfind_search -> Zone_precheck`.

## 1. Overview

`gamemd.exe` builds a three-level hierarchical zone graph for pathfinding. Full build runs levels `2 -> 1 -> 0`, where level 2 uses 8x8 aligned blocks, level 1 uses 4x4 blocks, and level 0 uses 2x2 blocks. Real zones start at id `1`; id `0` is a sentinel.

The builder contract Rust must reproduce is not just connectivity. It includes per-cell three-level zone ids, per-zone parent/coarser ids, reduced zone type, ordered edge arrays, per-edge low-byte flags, and bridge/tube temp edges appended after scanline-discovered temp edges.

## 2. Class Layout / Key Offsets

| Structure | Offset / stride | Type | Purpose | Evidence |
|-----------|-----------------|------|---------|----------|
| `MapClass+0x68` cell source | 4 bytes per cell | byte reduced type, byte height/level, `u16` persistent source-zone/cluster id | Source reduced zone type, height/level cache, and persistent source-zone id copied into hierarchy cell `+0x06` (corrected 2026-06-01: was only "byte class, byte level"; binary shows `MapClass__UpdateBridgeZonesHelper` clears/fills source `+2`, `ZoneMap__BuildZoneLevel` copies it, and `ZoneMap__FloodFillScanline` compares the copied halfword via `decompile_function 0x0056C510`, `0x00581F90`, `0x005824A0` - STRUCT_FAMILY_CASCADE) | `0x00567110`, `0x0047D2B0`, `0x00483C80`, `0x0056C510`, `0x00581F90`, `0x005824A0` |
| `MapClass+0x70` hierarchy cell data | 10 bytes per cell | `u16 level0`, `u16 level1`, `u16 level2`, `u16 source_zone`, byte height/level, byte spare | Direct per-cell lookup read by `Zone_precheck`; hierarchy flood-fill consumes `+0x06/+0x08` (corrected 2026-06-01: copied source values were underspecified; binary copies source `+2` to cell `+6` and source byte `+1` to cell `+8` via `decompile_function 0x00581F90`, then compares them in `decompile_function 0x005824A0` - STRUCT_FAMILY_CASCADE) | `0x00581F90`, `0x005824A0`, `0x0042C290` |
| level zone-count table | `MapClass+0x74 + level*4` | integer count | Stores next id / count including sentinel | `0x00581F90` |
| temporary edge buckets | `MapClass+0x80 + level*4` | 256 buckets, bucket stride `0x18` | Temporary 12-byte packed-pair edges before final graph emission | `0x00581F90`, `0x005824A0`, `0x00582D70` |
| final graph header | `MapClass+0x8C + level*0x18` and related global view `DAT_0087F878 + level*0x18` | vector header | Owns final zone records | `0x00581F90`, `0x0042C290` |
| final zone record | stride `0x24` | vector + metadata | edge pointer/count, parent, type, representative | `0x00581F90`, `0x0042C290` |
| final zone record `+0x04` | pointer | edge array | Stored-order adjacency scanned by `Zone_precheck` | `0x00581F90`, `0x0042C290` |
| final zone record `+0x10` | count | integer | Number of final 8-byte edge records | `0x00581F90`, `0x0042C290` |
| final zone record `+0x14` | integer | growth quantum | Set to `0x10` for records created here | `0x00581F90`, `0x00584550` |
| final zone record `+0x18` | `u16` | parent/coarser id | Next-coarser parent id; zero at level 2 | `0x00581F90`, `0x00584550`, `0x0042C290` |
| final zone record `+0x1C` | integer | reduced zone type | Passability/cost column consumed by `Zone_precheck` | `0x00581F90`, `0x00483C80`, `0x0042C290` |
| final zone record `+0x20` | integer | representative index | Written as `(x/4) + 0x83 + (y/4)*0x82`; consumer role not fully needed in this slice | `0x00581F90`, `0x00584550` |
| final edge record | stride `8` | `u32 neighbor`, `u32 flag` | `byte(edge+4) != 0` adds `0.001` in `Zone_precheck` | `0x00581F90`, `0x0042C290` |
| temp edge record | stride `12` | packed pair, duplicate copy, flag dword | Third dword low byte copied into both final directed edges | `0x005824A0`, `0x00582D70`, `0x00581F90` |

## 3. Core Logic

### 3.1 Full-build entry order

`FUN_00567110` clears/reallocates `MapClass+0x68` and `+0x70`, initializes source cell data, computes bridge zones, updates persistent bridge zones, then builds hierarchy levels in descending order:

1. `MapClass__InitCellAttributes(0)`
2. `MapClass__ComputeBridgeZones()`
3. `MapClass__UpdateBridgeZonesHelper()`
4. clear final vector for level `2`, call `ZoneMap__BuildZoneLevel(2)`
5. clear final vector for level `1`, call `ZoneMap__BuildZoneLevel(1)`
6. clear final vector for level `0`, call `ZoneMap__BuildZoneLevel(0)`
7. call `FUN_0042C1C0` to refresh pathfinder work arrays

Active in YR: Yes. Evidence: `0x00567110`; wrapper `0x00581F50` repeats the same level loop for all-level rebuilds.

Tiny details that matter:

- The per-map cell count stored at `MapClass+0x6C` is `(map_width + 1 + map_height)^2` as decompiled in `0x00567110`, not simply `width * height`.
- `MapClass+0x68` is initialized as 4-byte records with first byte `7` and second byte `0`; type `7` is the out-of-playfield/sentinel zone type skipped by hierarchy build.
- `MapClass+0x70` allocation is `cell_count * 10`, matching three `u16` level ids, copied source-zone halfword, and copied height/level byte (corrected 2026-06-01: was generic "copied source fields"; binary shows `+6 = *(MapClass+0x68+2)` and `+8 = *(MapClass+0x68+1)` via `decompile_function 0x00581F90` - STRUCT_FAMILY_CASCADE).
- Level build capacities at `MapClass+0xA0`, `+0xB8`, `+0xD0` are initialized from `(width * height * 4) / block_size^2` where block size is `1 << level_number_after_increment`; this is capacity sizing, not a gameplay count.

### 3.2 Per-cell source type and height source

`CellClass__RecalcAttributes @ 0x0047D2B0` calls `CellClass__RecalcZoneType @ 0x00483C80`, then writes:

- `MapClass+0x68[cell].byte0 = CellClass+0x4C` reduced zone type
- `MapClass+0x68[cell].byte1 = CellClass::Level`
- `MapClass+0x70[cell].byte8 = CellClass::Level`
- `MapClass+0x68[cell].u16_at_+2` is not written by `CellClass__RecalcAttributes`; it is reset and filled later by `MapClass__UpdateBridgeZonesHelper`, then consumed by the hierarchy builder (corrected 2026-06-01: source-cluster lifecycle was missing from this subsection; binary shows `+2/+3` zeroed and `MapClass__ZoneFloodFillScanLine` populating persistent source zones via `decompile_function 0x0056C510` - STRUCT_FAMILY_CASCADE).

`CellClass__RecalcZoneType` writes these reduced zone types:

| Type | Meaning in this slice | Evidence |
|------|-----------------------|----------|
| `7` | out of playfield / skipped by builder | `0x00483C80` early return when `MapClass__Is_Cell_In_Playfield` is false |
| `1` | overlay with inherited `ObjectTypeClass::Crushable` at `+0x22D`; also the parent-gate exception in `Zone_precheck` (corrected 2026-06-01: was "road/crate"; binary reads `+0x22D` in `CellClass__RecalcZoneType`, and `ObjectTypeClass__ReadINI` maps `+0x22D` to `Crushable=` via `decompile_function 0x00483C80` and `0x005F92D0` - RTTI_LABEL_DRIFT) | `0x00483C80`; consumer `0x0042C290`; `0x005F92D0` |
| `2` | wall or some overlay/building fallback class | `0x00483C80` |
| `3` | beach (`LandType == 6`) | `0x00483C80` |
| `4` | water (`LandType == 2`) | `0x00483C80` |
| `5` | building/object blocking class | `0x00483C80` |
| `6` | impassable/`IsARock`/zero-speed class (corrected 2026-06-01: was "gate"; binary reads overlay `+0x2B5`, and `OverlayTypeClass__ReadINI` maps `+0x2B5` to `IsARock=` via `decompile_function 0x00483C80` and `0x005FE770` - RTTI_LABEL_DRIFT) | `0x00483C80`, `0x005FE770` |
| `0` | default ground | `0x00483C80` |

Active in YR: Yes. Evidence: `CellClass__RecalcAttributes` writes the exact arrays read by the hierarchy builder. There are TS-era looking branches around `RulesClass+0x664`, but the reduced zone type field is live regardless; this report does not claim those optional branch defaults beyond their presence.

### 3.3 `ZoneMap__BuildZoneLevel` setup

For one level, `0x00581F90`:

1. Clears all 256 temporary edge buckets for the level.
2. Copies persistent source-zone and height/level into the hierarchy cell array and clears the target level id (corrected 2026-06-01: was "source type/height"; binary copies source `+2` to hierarchy `+6` and source `+1` to hierarchy `+8` via `decompile_function 0x00581F90` - STRUCT_FAMILY_CASCADE):
   - `cell.level[level] = 0`
   - `cell.source_zone_or_cluster = source+2`
   - `cell.height = source.byte1`
3. Creates sentinel zone `0`:
   - `zone0.parent = 0`
   - `zone0.type = 7`
4. Sets `next_zone_id = 1`.
5. Computes block size as `1 << (level + 1)`.

Active in YR: Yes. Evidence: `0x00581F90`.

### 3.4 Row-major first-discovery ids inside aligned blocks

The full-build scan walks the full hierarchy cell array linearly. It skips a cell if:

- source type is `7`, or
- the current level id is already nonzero.

For every remaining cell, it calls `ZoneMap__FloodFillScanline @ 0x005824A0` with the current aligned block state and assigns the next real zone id. After flood-fill returns, the builder writes one final zone record and increments the zone id.

Block size:

| Level | Binary formula | Aligned block size |
|-------|----------------|--------------------|
| `2` | `1 << (2 + 1)` | 8x8 |
| `1` | `1 << (1 + 1)` | 4x4 |
| `0` | `1 << (0 + 1)` | 2x2 |

The block start is maintained by row scan state rather than recomputing from every cell, but equivalent Rust behavior for non-negative cells is aligned blocks of the sizes above.

Active in YR: Yes. Evidence: `0x00581F90`, `0x005824A0`.

### 3.5 Flood-fill continuity

`ZoneMap__FloodFillScanline` expands only through cells that match:

- same copied source-zone/cluster halfword at hierarchy cell `+0x06` (from `MapClass+0x68+2`), not the reduced type byte directly (corrected 2026-06-01: was "same reduced source type / cluster field"; binary loads `*(short *)(cell+6)` as the match key in `decompile_function 0x005824A0`, after `decompile_function 0x00581F90` copies source `+2` - STRUCT_FAMILY_CASCADE),
- height delta less than `2` between neighboring cells,
- unassigned at the target hierarchy level,
- inside the current aligned block rectangle when recursing into unassigned cells,
- in playfield for boundary-edge checks.

Horizontal fill first walks left, then right, assigning the same level zone id to every accepted cell. It returns the filled horizontal span length so the outer row-major scan can skip those cells.

Tiny details that matter:

- Height continuity uses absolute byte difference `< 2`, so height difference `0` or `1` connects; `2` does not.
- Horizontal boundary temp edges are zero-flagged.
- Vertical neighbor recursion is the only observed source of nonzero low-byte temp flags in this slice.
- `local_48`/last-neighbor suppression prevents immediately repeated boundary edge emission for one scanline path; it is not a final graph dedup.

Active in YR: Yes. Evidence: `0x005824A0`; xrefs from `0x00581F90` and `0x00584550`.

### 3.6 Zone record writes

For each discovered zone, the full builder writes:

- `record+0x18 = cell.level[level+1]` for levels `0` and `1`;
- `record+0x18 = 0` for level `2`;
- `record+0x1C = reduced source type`;
- `record+0x14 = 0x10`;
- `record+0x20 = (x/4) + 0x83 + (y/4)*0x82`.

The parent is sampled from the first discovered cell that creates the zone. Because level 2 is built first, level 1 zones can parent to level 2 ids, and level 0 zones can parent to level 1 ids.

Active in YR: Yes. Evidence: `0x00581F90`; incremental parent refresh in `0x00584550`; consumer parent gate in `0x0042C290`.

### 3.7 Temporary edge identity and flag semantics

Temporary edges are stored in 256 buckets selected by `(from_zone & 0xF) << 4 | (to_zone & 0xF)`. The stored packed pair is directional/exact, not undirected canonicalized. The helper scans the selected bucket for exact equality before appending.

The temp flag low byte is:

- usually `0`;
- set to `1` in vertical boundary paths when the x coordinate is outside the current block's horizontal range;
- copied into both final directed edges' `edge+4` dword.

This means the final flag is a hierarchy-boundary/tiebreak flag, not a bridge-edge flag.

Active in YR: Yes. Evidence: `0x005824A0`, `0x00581F90`, `0x0042C290`; prior `BRIDGE_ZONE_EDGE_FLAG_WRITER_SEMANTICS_GHIDRA_REPORT.md`.

### 3.8 Bridge/tube full-build injection

After all cells for the level have been scanned and the level zone count is stored, `ZoneMap__BuildZoneLevel` iterates bridge/tube records:

- record count from `MapClass+0x60`,
- record base from `MapClass+0x54`,
- record stride `0x10`,
- active test `record+8 != 0`,
- call `FUN_00582D70(record, level)`.

`FUN_00582D70` computes three connection pairs for bridge/tube connectivity, looks up zone ids from `MapClass+0x70`, checks exact packed-pair duplicates in the same temp-bucket graph, and appends zero-flagged temp entries when needed.

Tiny details that matter:

- Bridge/tube temp insertion happens after scanline temp insertion and before final emission.
- If a scanline temp edge with the same exact packed pair already exists, the bridge/tube helper does not reorder it or rewrite its flag.
- The bridge/tube helper zeroes its flag byte before each append; bridge edges must not get the `0.001` flag solely because they are bridge edges.
- For invalid cell-coordinate lookup, the helper clamps `MapClass__CellCoordToLinearIndex` results into `[0, cell_count - 1]` before reading zone ids.
- Tube branch has null checks for the two tube records and returns early if either lookup is missing.

Active in YR: Yes for maps with active bridge/tube records. Evidence: `0x00581F90`, `0x00582D70`; xrefs at full build and incremental rebuild.

### 3.9 Final graph emission order

After scanline and bridge/tube temporary insertion, full build emits final edges:

1. Iterate temp buckets from offset `0` to `< 0x1800`, stepping `0x18`; this is 256 buckets.
2. For each bucket, walk 12-byte entries in stored insertion order.
3. Decode low halfword and high halfword from the packed pair.
4. Append directed edge on the low-halfword zone record first.
5. Append reverse directed edge on the high-halfword zone record second.
6. Copy the same temp flag byte/dword into both final `edge+4` slots.

There is no final sort and no final dedup pass in the full writer. Ordered adjacency matters because `Zone_precheck` scans final edge arrays linearly and equal-cost paths preserve insertion/heap order.

Active in YR: Yes. Evidence: `0x00581F90`; consumer `0x0042C290`; prior insertion-order report.

### 3.10 Incremental sibling spot-check

`FUN_00584550 @ 0x00584550` is not the first implementation target, but it proves the full-build contract also governs local map changes. It:

- rejects cells outside playfield,
- loops levels `2 -> 1 -> 0`,
- aligns the changed cell to the level's block size,
- collects old zone ids in that block,
- clears target level ids for cells in the block,
- removes final edges touching replaced zones,
- flood-fills replacement zones with `ZoneMap__FloodFillScanline`,
- adds bridge/tube temp edges for active records whose endpoints touch the block,
- emits final edges by the same temp-bucket traversal shape,
- refreshes parent ids across the aligned 8x8 area after all three levels,
- calls `FUN_0042C1C0`.

It has a fallback path that rebuilds all levels with `ZoneMap__BuildZoneLevel` and refreshes arrays. The exact trigger for abnormal capacity/fallback behavior is not required for initial production full-build wiring and belongs to the mutation pass.

Active in YR: Yes. Evidence: `0x00584550`; callers include `OverlayClass__Mark`, `CellClass__DestroyOverlay`, `BuildingClass__Place_OccupyMap`, `TerrainClass__Limbo`, area damage, building sell, and map recalc paths.

### 3.11 Pathfinder array refresh

`FUN_0042C1C0` frees and reallocates three arrays per hierarchy level, zeroing each new allocation. The allocation count for each level comes from the level zone-count/global sizing table. This must run after full hierarchy rebuilds in the binary.

For Rust this is a consumer scratch concern, not a persistent graph field. The important contract is that any production hierarchy rebuild must leave `zone_precheck` scratch/marker structures sized for the new per-level zone counts before the next path query.

Active in YR: Yes. Evidence: `0x0042C1C0`; called from `0x00567110`, `0x00581F50`, and `0x00584550`.

### 3.12 Consumer fields required by `Zone_precheck`

`Zone_precheck @ 0x0042C290` confirms which builder outputs are load-bearing:

- reads per-cell level ids from `DAT_0087F858` (`MapClass+0x70`);
- searches levels in order `2 -> 1 -> 0`;
- reads final graph records from `DAT_0087F878 + level*0x18`;
- scans edge pointer/count in final stored order;
- reads neighbor zone id from `edge+0`;
- tests `byte(edge+4)` and adds `0.001` when nonzero;
- reads neighbor zone parent from `record+0x18` and type from `record+0x1C`;
- at levels `1` and `0`, requires the neighbor's parent to be on the next-coarser selected path unless the neighbor type is `1`;
- checks passability matrix row `MovementZone` by reduced type.

Active in YR: Yes. Evidence: `0x0042C290`; normal A* path calls it via `AStar_pathfind_search` per prior reports.

## 4. INI Keys

No INI key directly controls the hierarchy writer order or edge flag byte. INI and map data still matter as inputs to cell classification, bridge presence, and consumer passability.

| Key / data | Default / values observed | Role in this slice | Active in YR? |
|------------|---------------------------|--------------------|---------------|
| `MovementZone=` | many unit/infantry/aircraft values in `rulesmd.ini`; examples include `Normal`, `Infantry`, `Crusher`, `Water`, `Fly`, `AmphibiousDestroyer` | Selects `Zone_precheck` passability row; not a hierarchy writer-order key | Yes |
| `SpeedType=` | many unit values in `rulesmd.ini` | Terrain speed/cost input upstream; not the direct hierarchy level builder key | Yes |
| `TooBigToFitUnderBridge=` | many naval/large unit overrides | Bridge legality adjacent to pathing; not required for full hierarchy build contract | Yes, out-of-scope for this slice |
| `BridgeRepairHut=` | `CABHUT=yes` in `rulesmd.ini` | Enables bridge repair lifecycle; direct add/remove is batch 2 | Yes, deferred |
| `BridgeStrength=` | `[General] BridgeStrength=1500` in `rulesmd.ini` | Bridge collapse threshold leading to mutation; not a full-build field | Yes, deferred |
| `DestroyableBridges=` | `[General] DestroyableBridges=yes` in `rulesmd.ini` | Allows bridge collapse/mutation scenarios | Yes, deferred |
| `BridgeDestruction=` | map `[SpecialFlags]`, default comment says true; `rulesmd.ini` has `BridgeDestruction=yes` in special flags defaults | Scenario-level bridge destruction gate | Conditional, deferred |
| `TunnelSpeed=` | `[General] TunnelSpeed=1` in `rulesmd.ini` | Low bridge/tube movement adjacent; do not conflate with hierarchy temp-edge injection | Yes, deferred |
| map bridge/tube records | map/terrain derived | Feed `FUN_00582D70` after zone discovery | Conditional on map content |

## 5. Integration Points

| Function / path | Relationship | Evidence | Active in YR? |
|-----------------|--------------|----------|---------------|
| `FUN_00567110` | map zone init; allocates source/hierarchy cell arrays, computes bridge zones, builds levels `2,1,0`, refreshes pathfinder arrays | `0x00567110` | Yes |
| `FUN_00581F50` | all-level hierarchy rebuild wrapper; clears level vectors, builds `2,1,0`, refreshes arrays | `0x00581F50` | Yes |
| `ZoneMap__BuildZoneLevel @ 0x00581F90` | full production builder for one hierarchy level | `0x00581F90`; callers `0x00567110`, `0x00581F50`, `0x00584550` fallback | Yes |
| `ZoneMap__FloodFillScanline @ 0x005824A0` | discovers zones and scanline temp edges | `0x005824A0`; callers `0x00581F90`, `0x00584550` | Yes |
| `FUN_00582D70` | active bridge/tube temp-edge injector into hierarchy temp graph | `0x00582D70`; xrefs from full and incremental build | Yes/conditional on records |
| `FUN_00584550` | local incremental hierarchy rebuild after cell-affecting mutations | `0x00584550`; live callers from overlay/building/terrain/damage paths | Yes |
| `CellClass__RecalcAttributes -> CellClass__RecalcZoneType` | produces source reduced zone type and height copied into `MapClass+0x68/+0x70` | `0x0047D2B0`, `0x00483C80` | Yes |
| `FUN_0042C1C0` | rebuilds pathfinder per-level scratch arrays after graph rebuild | `0x0042C1C0` | Yes |
| `Zone_precheck @ 0x0042C290` | consumer proving load-bearing builder fields | `0x0042C290` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current status | Delta from this report |
|--------------|----------------|------------------------|
| `src/sim/pathfinding/zone_hierarchy.rs` | Defines `ZoneHierarchy`, `ZoneLevelGraph`, `ZoneRecord`, `ZoneEdgeRecord`, and `zone_precheck_flat`; synthetic tests cover parent gate, flags, exclusions, and search behavior. | Consumer foundation exists, but no production builder fills it from map data. |
| `src/sim/pathfinding/zone_map.rs` | `ZoneGrid` stores one `ZoneMap`/`ZoneAdjacency` per `MovementZone` and can store optional `ZoneHierarchy` via `set_hierarchy`. | Storage hook exists; `build_with_terrain` initializes `hierarchies` empty. |
| `src/sim/pathfinding/zone_build.rs` | Builds flat zone maps, extracts flat adjacency, injects bridge adjacency, and builds bridge redirect tables. | Does not build binary 8/4/2 levels, parent ids, temp buckets, final edge flags, or final emission order. |
| `src/sim/pathfinding/zone_incremental.rs` | Rebuilds flat zone maps/adjacency/super-zones for affected areas and bridge records. | Must clear or rebuild hierarchy when used; exact incremental hierarchy parity is deferred. |
| `src/sim/pathfinding/zone_search.rs` | Private path can use hierarchy when optional blocker counts are supplied; public production path falls back without counts. | Hierarchy remains dormant in production until builder and blocker-neighbor counts exist. |
| `src/sim/pathfinding/core.rs` | `BlockerNeighborCounts` and hierarchy-gated cell A* helper exist. | Count producer is absent and belongs to batch 4; do not treat missing counts as zero for production parity. |
| `src/sim/bridge_state/*`, `src/sim/world/bridge_orchestrator.rs` | Runtime bridge damage/repair/collapse rebuilds path grids and state. | Direct bridge hierarchy add/remove strategy remains batch 2. First implementation can conservatively rebuild whole hierarchy with the `ZoneGrid`. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `FUN_00567110` full init order | verified | `0x00567110` decompile | none for full-build order |
| `FUN_00581F50` all-level rebuild wrapper | verified | `0x00581F50` decompile | exact caller frequency not measured |
| `ZoneMap__BuildZoneLevel @ 0x00581F90` | verified | direct decompile; prior `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` | none for fields/order in this slice |
| `ZoneMap__FloodFillScanline @ 0x005824A0` | verified | direct decompile; prior edge flag report | none for builder contract |
| `FUN_00582D70` full-build bridge/tube temp-edge injection | verified | direct decompile | exact high-vs-low record production is batch 2 |
| `FUN_00584550` incremental rebuild sibling | touched-not-exhausted | direct decompile; caller xrefs | exact direct mutation/repeated lifecycle belongs to batch 2 |
| `CellClass__RecalcZoneType @ 0x00483C80` | verified for reduced type source | direct decompile | full optional `RulesClass+0x664` semantics out of scope |
| `CellClass__RecalcAttributes @ 0x0047D2B0` | verified for writes into `+0x68/+0x70` source arrays | direct decompile | complete terrain/LAT/slope behavior out of scope |
| `FUN_0042C1C0` pathfinder array refresh | verified | direct decompile | Rust scratch sizing design remains implementation work |
| `Zone_precheck @ 0x0042C290` consumer contract | verified for fields consumed | direct decompile; prior consumer reports | full retry/A* integration belongs to batch 3 |
| direct `AddBridgeZoneEdges` / `RemoveBridgeZoneEdges` | deferred | existing reports only | batch 2 should produce mutation contract |
| `CellClass+0x122` writers | deferred | plan inventory | batch 4 should produce blocker-count contract |
| stock-map route chain after builder | deferred | no runtime trace | trace after Rust builder/count path exists |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-HFB-001 -- Is this exhaustive-slice or coverage-map? -> exhaustive-slice for batch 1 only; broader 35-function plan remains split.` (evidence: `docs/plans/2026-05-24-zone-precheck-production-hierarchy-builder-investigation-plan.md`)
- `[RESOLVED] OQ-HFB-002 -- Does a newer expected-output file already execute this plan? -> no narrow `ZONE_PRECHECK_HIERARCHY_FULL_BUILD_CONTRACT_GHIDRA_REPORT.md` existed before this pass.` (evidence: filesystem check)
- `[RESOLVED] OQ-HFB-003 -- Which function owns full map hierarchy init? -> `FUN_00567110` allocates arrays, computes bridge zones, builds levels 2,1,0, then refreshes pathfinder arrays.` (evidence: `0x00567110`)
- `[RESOLVED] OQ-HFB-004 -- Is there an all-level rebuild wrapper? -> yes, `FUN_00581F50` clears/builds levels 2,1,0 and calls `FUN_0042C1C0`.` (evidence: `0x00581F50`)
- `[RESOLVED] OQ-HFB-005 -- What are hierarchy block sizes? -> `1 << (level+1)`: 8,4,2 for levels 2,1,0.` (evidence: `0x00581F90`)
- `[RESOLVED] OQ-HFB-006 -- Are real zone ids row-major? -> yes, real ids start at 1 and are assigned by first unassigned non-type-7 cell in row-major scan.` (evidence: `0x00581F90`)
- `[RESOLVED] OQ-HFB-007 -- What source marks cells unbuildable for hierarchy? -> source type `7` from `MapClass+0x68` skips cells; `CellClass__RecalcZoneType` writes this for out-of-playfield.` (evidence: `0x00483C80`, `0x00581F90`)
- `[RESOLVED] OQ-HFB-008 -- What continuity rule does flood-fill use? -> same copied source-zone/cluster halfword and neighbor height byte difference less than 2, inside current block for recursion.` (corrected 2026-06-01: was "same reduced type/cluster"; binary compares hierarchy cell `+6` copied from `MapClass+0x68+2`, not reduced type byte `+0`, via `decompile_function 0x00581F90` and `0x005824A0` - STRUCT_FAMILY_CASCADE)
- `[RESOLVED] OQ-HFB-009 -- Where is parent/coarser id written? -> zone record `+0x18`, from next-level cell id for levels 0/1 and zero for level 2.` (evidence: `0x00581F90`, `0x00584550`)
- `[RESOLVED] OQ-HFB-010 -- What type does the consumer use? -> zone record `+0x1C`, copied from reduced source type, used for passability/cost and type-1 parent-gate exception.` (evidence: `0x00581F90`, `0x0042C290`)
- `[RESOLVED] OQ-HFB-011 -- Are temp edge duplicates undirected canonicalized? -> no, exact packed-pair equality in a bucket is checked before append.` (evidence: `0x005824A0`, `0x00582D70`)
- `[RESOLVED] OQ-HFB-012 -- What does `edge+4` mean in this slice? -> low byte is a hierarchy-boundary/tiebreak flag; nonzero adds `0.001` in `Zone_precheck`.` (evidence: `0x005824A0`, `0x00581F90`, `0x0042C290`)
- `[RESOLVED] OQ-HFB-013 -- Do bridge/tube injected edges set the flag? -> no, `FUN_00582D70` appends zero-flag temp entries.` (evidence: `0x00582D70`)
- `[RESOLVED] OQ-HFB-014 -- Where do bridge/tube temp edges enter full build? -> after scanline discovery and level zone count write, before final temp-bucket emission.` (evidence: `0x00581F90`)
- `[RESOLVED] OQ-HFB-015 -- Is final adjacency sorted? -> no, bucket order then temp insertion order, low-halfword directed append then reverse append.` (evidence: `0x00581F90`)
- `[RESOLVED] OQ-HFB-016 -- Which builder fields does `Zone_precheck` actually consume? -> per-cell ids, parent, type, edge pointer/count, edge flag, and stored edge order.` (evidence: `0x0042C290`)
- `[RESOLVED] OQ-HFB-017 -- Is incremental rebuild live? -> yes, `0x00584550` has live callers from overlay, building, terrain, damage, sell, and map recalc paths.` (evidence: `get_function_callers 0x00584550`)
- `[RESOLVED] OQ-HFB-018 -- Does incremental rebuild share graph semantics? -> yes for aligned block rebuild, temp bridge/tube injection, final bucket emission, parent refresh, and pathfinder array refresh.` (evidence: `0x00584550`)
- `[RESOLVED] OQ-HFB-019 -- Does Rust currently build this hierarchy in production? -> no; storage and consumer exist, but production `ZoneGrid::build_with_terrain` leaves hierarchy empty.` (evidence: `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`)
- `[RESOLVED] OQ-HFB-020 -- Is missing blocker-count producer part of this builder slice? -> no; hierarchy-gated A* needs it before public production use, but writer lifecycle belongs to batch 4.` (evidence: plan Section 10; `src/sim/pathfinding/core.rs`)
- `[DEFERRED] OQ-HFB-021 -- Should Rust use direct incremental add/remove or whole hierarchy rebuild first?` (category: out-of-scope; reason: direct bridge mutation helpers are batch 2; next-step-if-pursued: run bridge/tube mutation contract)
- `[DEFERRED] OQ-HFB-022 -- Exact stock Carville/low-bridge selected zone chain after collapse.` (category: needs-runtime-debugger; reason: requires route/zone logging rather than static builder contract; next-step-if-pursued: trace after builder and counts are wired)
- `[DEFERRED] OQ-HFB-023 -- Full `CellClass+0x122` lifecycle and bridge-layer implications.` (category: out-of-scope; reason: planned as batch 4; next-step-if-pursued: run blocker-neighbor count lifecycle investigation)
- `[DEFERRED] OQ-HFB-024 -- Full optional `RulesClass+0x664` branch semantics in `CellClass__RecalcAttributes`.` (category: requires-different-system-context; reason: this branch is terrain/LAT/fog/slope adjacent and not required for hierarchy builder contract beyond source writes; next-step-if-pursued: investigate cell attribute recomputation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| Full hierarchy has three levels built `2 -> 1 -> 0` with block sizes `8,4,2`; each level has per-cell zone ids and zone records. Active in YR: Yes. | `0x00567110`, `0x00581F50`, `0x00581F90` | missing production builder; test fixtures only | `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_hierarchy.rs` | Build a `ZoneHierarchy` for each bridge-capable relevant movement zone during `ZoneGrid::build_with_terrain`, using binary block sizes and row-major zone id assignment. | A 9x9 map with one continuous ground area creates multiple level-2/1/0 records split by 8/4/2 aligned block boundaries, with ids assigned by first discovery. | Do not derive hierarchy from flat connected components; block boundaries are load-bearing. |
| Zone record parent is next-coarser cell id for levels 0/1 and zero for level 2; `Zone_precheck` uses parent-gated lower-level search except type `1`. Active in YR: Yes. | writer `0x00581F90`; consumer `0x0042C290` | `ZoneRecord.parent` exists but no production values | `zone_hierarchy.rs`, future builder in `zone_build.rs` | Populate parent ids from already-built coarser levels and preserve type-1 exception in consumer. | Fine-level off-corridor edge under an unchosen parent is rejected; type-1 neighbor exception remains allowed. | Do not approximate parent gating with super-zone reachability. |
| Flood-fill groups cells by the same copied persistent source-zone/cluster halfword and height delta `< 2` inside aligned blocks; cross-block contacts become temp edges. Active in YR: Yes. | `0x0056C510`, `0x00581F90`, `0x005824A0` | flat Rust zone flood-fill does not model hierarchy block-bounded grouping or the persistent source-zone match key | `zone_build.rs` | Build/update the `MapClass+0x68+2` equivalent source-zone ids first, then use a hierarchy-specific scanline/block flood-fill or equivalent deterministic traversal that emits the same zones and contacts (corrected 2026-06-01: was "same reduced type"; binary compares copied `+6` source-zone halfword via `decompile_function 0x00581F90` and `0x005824A0` - STRUCT_FAMILY_CASCADE). | Adjacent same-type cells split by source-zone/bridge-zone identity or by a 2x2 level-0 boundary are distinct level-0 zones connected by an edge, not one merged zone. | Do not reuse flat `flood_fill` directly for hierarchy levels without block bounds; do not key hierarchy fill solely on reduced type. |
| Final adjacency order is temp bucket order, temp insertion order, low-halfword directed edge first, reverse second; no final sorting. Active in YR: Yes. | `0x00581F90`, prior writer-order report | current flat adjacency preserves some order but not binary temp buckets; some helper paths sort/dedup for other uses | `zone_build.rs`, `zone_hierarchy.rs` | Store hierarchy edges in append-order vectors with edge metadata. | Equal-cost candidate paths with different insertion order choose the binary's first-written edge. | Do not store parity hierarchy final edges in `BTreeSet` or sorted `ZoneId` order. |
| `byte(edge+4)` is copied from temp edge flag; nonzero adds `0.001` in `Zone_precheck`. Active in YR: Yes. | `0x005824A0`, `0x00581F90`, `0x0042C290` | `ZoneEdgeRecord.flag` exists; no production writer | `zone_hierarchy.rs`, future builder | Set flag low byte to `1` only for verified out-of-block vertical boundary temp edges; otherwise zero. | Two otherwise equal hierarchy routes differ only by flagged edge; unflagged route wins by `0.001`. | Do not treat this as a universal bridge, diagonal, or terrain penalty. |
| Bridge/tube hierarchy temp edges are appended after scanline temp edges and are zero-flagged. Active in YR: Yes/conditional on map records. | `0x00581F90`, `0x00582D70` | flat `inject_bridge_adjacency` exists but not hierarchy temp insertion/flags | `zone_build.rs`, bridge record handoff from `src/sim/bridge_state/*` | During full hierarchy build, inject active bridge/tube record pairs into temp buckets after scanline discovery, preserving first exact-pair entry. | If scanline and bridge insertion produce the same exact pair, scanline order/flag wins; otherwise bridge edge appends with flag zero. | Do not mark all bridge edges as flagged or insert them before scanline edges. |
| Production `Zone_precheck` needs hierarchy plus blocker-neighbor counts before public hierarchy-gated cell A* should be enabled. Active in YR: Yes for consumer; counts batch deferred. | `0x0042C290`; current Rust scan | hierarchy path currently guarded by optional `BlockerNeighborCounts`; no count producer | `zone_search.rs`, `core.rs`, future movement/occupancy count producer | Keep production hierarchy dormant or guarded until real counts are supplied; tests may use synthetic counts. | Public pathing without count producer must not silently over-prune routes around blockers. | Do not treat missing counts as all-zero production data. |
| Initial implementation can conservatively rebuild hierarchy alongside `ZoneGrid` rebuild; exact direct bridge add/remove is a later optimization/contract. Active in YR: binary has direct mutation, but full rebuild is behaviorally safer as a first Rust step if timing is acceptable. | full rebuild `0x00581F50`; incremental/direct mutation deferred | Rust already rebuilds path grid/zone grid after bridge state changes | `zone_map.rs`, `zone_incremental.rs`, world bridge rebuild surfaces | Build hierarchy from current map/bridge state whenever `ZoneGrid` is rebuilt; clear stale hierarchy on mutations until direct mutation contract is implemented. | Bridge collapse triggers rebuilt hierarchy whose bridge/tube temp edges reflect current active records. | Do not implement guessed direct add/remove from this report alone. |

## 10. Negative Facts / Do Not Do

- Do not conflate `0x0056CB90` with this hierarchy scanline builder. The relevant hierarchy helper is `ZoneMap__FloodFillScanline @ 0x005824A0`; `0x0056CB90` is a persistent zone flood-fill path.
- Do not claim `edge+4` means bridge edge. Verified bridge/tube full-build insertion writes zero flags.
- Do not use flat `ZoneMap` connected components as equivalent to the hierarchy. The binary deliberately splits cells by 8/4/2 aligned blocks.
- Do not key hierarchy flood-fill solely on reduced zone type. The binary compares the copied persistent source-zone/cluster halfword at hierarchy cell `+0x06` and uses reduced type separately for skip/type metadata (corrected 2026-06-01: added after verifying `decompile_function 0x0056C510`, `0x00581F90`, `0x005824A0` - STRUCT_FAMILY_CASCADE).
- Do not sort final hierarchy adjacency by `ZoneId`.
- Do not wire public hierarchy-gated A* without production `BlockerNeighborCounts`.
- Do not use direct bridge add/remove helpers from memory; run batch 2 first if direct mutation is desired.

## Sources

- Ghidra decompiled: `FUN_00567110 @ 0x00567110`, `FUN_00581F50 @ 0x00581F50`, `ZoneMap__BuildZoneLevel @ 0x00581F90`, `ZoneMap__FloodFillScanline @ 0x005824A0`, `FUN_00582D70 @ 0x00582D70`, `FUN_00584550 @ 0x00584550`, `MapClass__UpdateBridgeZonesHelper @ 0x0056C510`, `MapClass__ZoneFloodFillScanLine @ 0x0056CB90`, `CellClass__RecalcZoneType @ 0x00483C80`, `CellClass__RecalcAttributes @ 0x0047D2B0`, `ObjectTypeClass__ReadINI @ 0x005F92D0`, `OverlayTypeClass__ReadINI @ 0x005FE770`, `FUN_0042C1C0 @ 0x0042C1C0`, `Zone_precheck @ 0x0042C290`.
- Ghidra xrefs checked: callers of `0x00581F90`, `0x005824A0`, `0x00584550`, `0x00483C80`.
- Plan: `docs/plans/2026-05-24-zone-precheck-production-hierarchy-builder-investigation-plan.md`.
- Prior reports referenced: `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`, `BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`, `BRIDGE_ZONE_EDGE_FLAG_WRITER_SEMANTICS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `LOW_BRIDGE_ZONE_PRECHECK_LANDTYPE10_CONNECTIVITY_GHIDRA_REPORT.md`, `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md`.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust surfaces scanned: `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_incremental.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`, `src/sim/bridge_state/*`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/movement/*`.
