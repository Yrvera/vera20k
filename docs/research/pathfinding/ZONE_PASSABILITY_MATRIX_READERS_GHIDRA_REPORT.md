# ZonePassabilityMatrix Readers -- Ghidra Research Report

**Address(es):** `0x0082A594` (data), direct readers `0x0042C290`, `0x0056C510`, `0x005840C0`, `0x005889F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** direct static readers of `ZonePassabilityMatrix`, its dimensions/value semantics, and the movement-zone/speed-type classification contract visible at those readers.  
**Non-Scope:** full pathfinder, full `MovementZone=`/`SpeedType=` parser inventory, exact CellClass flag taxonomy, dynamic zone dirtying/incremental rebuild parity, rectangle validators.  
**Confidence:** High for direct-reader inventory and dimensions; Medium for Rust delta because only relevant surfaces were scanned.  
**Active in YR:** Yes. The readers sit on `FootClass::Find_Path -> FootClass::Run_AStar -> AStar_pathfind_search`, map zone rebuild/bridge update, and team zone-category computation paths with no TS-only gate found in this slice.

## 1. Overview

`ZonePassabilityMatrix` is a 13-row by 8-column table of 32-bit integers at `0x0082A594`. The row index is `MovementZone` (`TechnoTypeClass+0x5B4`), not `SpeedType`; the column index is the reduced `CellClass` zone type (`CellClass+0x4C`), not the 12-value land type or the INI speed table.

Direct readers use only value `1` as passable. Values `2` and `3` both block movement-zone connectivity; value `3` is the row-wide out-of-bounds/sentinel column, not a special pass value.

## 2. Class Layout / Key Offsets

| Offset / address | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `0x0082A594` | `int[13][8]` | Passability matrix | `read_memory(0x0082A594,416)` returned 104 DWORDs; loops end at `0x82A734` in `0x0056C510` and `0x005889F0` | Yes |
| `0x0082A734` | end pointer | First byte after matrix | `0x0056C510` and `0x005889F0` loop while pointer `< 0x82A734` | Yes |
| `TechnoTypeClass+0x5B4` | int | `MovementZone`, direct matrix row | `0x00716079` calls `CCINIClass__ReadMovementZone`; `0x00716081` stores EAX to `[EBP+0x5B4]`; `0x0042C93F` / `0x00584204` read it | Yes |
| `TechnoTypeClass+0x67C` | int | `SpeedType`, separate from matrix row | `0x007121E0` calls `CCINIClass__ReadSpeedType`; `0x007121E5` stores EAX to `[EBP+0x67C]`; no direct matrix reader indexes by this field | Yes |
| `CellClass+0x4C` | int | Reduced zone type column 0..7 | `CellClass__RecalcZoneType @ 0x00483C80` writes literal values 0..7 | Yes |
| `MapClass+0x18` | `ptr[13]` | per-`MovementZone` zone id arrays built from the matrix | `0x0056C510` frees 13 pointers, then writes one array per matrix row | Yes |
| `MapClass+0x68` | cell zone data | 4 bytes/cell; byte0 zone type, bytes2-3 cluster id | `0x0056C510` reads byte0 for matrix columns and bytes2-3 for cluster ids | Yes |

## 3. Core Logic

### Matrix dimensions and values

Binary memory dump at `0x0082A594` is exactly 416 bytes = `13 * 8 * 4`. Parsed as DWORD rows:

| MZ row | Values by zone type 0..7 | Active in YR |
|---:|---|---|
| 0 Normal | `1,2,2,2,2,2,2,3` | Yes |
| 1 Crusher | `1,1,2,2,2,2,2,3` | Yes |
| 2 Destroyer | `1,1,1,2,2,2,2,3` | Yes |
| 3 AmphibiousDestroyer | `1,1,1,1,1,1,2,3` | Yes |
| 4 AmphibiousCrusher | `1,1,2,1,1,2,2,3` | Yes |
| 5 Amphibious | `1,2,2,1,1,2,2,3` | Yes |
| 6 Subterranean | `1,1,1,2,2,2,1,3` | Yes |
| 7 Infantry | `1,2,2,2,2,1,2,3` | Yes |
| 8 InfantryDestroyer | `1,1,1,2,2,1,2,3` | Yes |
| 9 Fly | `1,1,1,1,1,1,1,3` | Yes/Conditional: standard YR content uses `MovementZone=Fly`; whether a given aircraft/jumpjet path reaches these readers depends on locomotor path |
| 10 Water | `2,2,2,2,1,2,2,3` | Yes |
| 11 WaterBeach | `2,2,2,1,1,2,2,3` | Yes |
| 12 CrusherAll | `1,1,1,2,2,2,2,3` | Yes |

`0x0056C510` and `0x005889F0` both stop at `0x82A734`; that end address proves 13 rows. A 12x8 table would end at `0x82A714`, which is not what the binary uses.

### Direct reader inventory

| Reader | What it reads | Semantics | Evidence | Active in YR |
|---|---|---|---|---|
| `Zone_precheck @ 0x0042C290` | `(&g_PassabilityMatrix)[movementZone * 8 + edgeZoneType]` | zone-edge is traversable only if value `== 1`; otherwise skipped | decompile branch at `0x0042C6xx`: `... && ((&g_PassabilityMatrix)[param_4 * 8 + iVar18] == 1)` | Yes. Called by `AStar_pathfind_search @ 0x0042C900`, which is called by `FootClass__Run_AStar @ 0x004CBBA0` |
| `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` | all rows, all clusters' zone types | builds `MapClass+0x18[row]` arrays; initial per-row values are `matrix[row][cluster_type] != 1`; only passable clusters flood into numbered zones | decompile: frees 13 arrays, sets `puStack_3c=&g_PassabilityMatrix`, advances by 8, loops until `<0x82A734` | Yes. Called by initial/zone rebuild and many bridge mutation paths |
| `ZoneMap__FloodFillReachableZones @ 0x005840C0` | `matrix[ownerMovementZone][neighborCellZoneType]` | when exploring local neighbors for updated hierarchy, a neighbor is rejected if cell `Can_Enter_Cell` fails or matrix value is not `1` | decompile: gets `iVar9 = type+0x5B4`, then checks `(&g_PassabilityMatrix)[iVar9 * 8 + *(int *)(cell+0x4C)] != 1` | Yes. Called by `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`, reached after A* corridor failures |
| `ZoneMap__FindBestCompatibleMovementZone @ 0x005889F0` | all 13 candidate rows and two input rows | team compatibility chooser; candidate invalid if it passes (`1`) any column blocked as `2` by either input; score counts columns where all three are `1` | decompile loops `piVar3=&g_PassabilityMatrix` until `<0x82A734`; called by `TeamTypeClass__ComputeZoneCategory @ 0x006F1FA0` | Conditional. Active for TeamTypes/team path categories; not the individual-foot A* row selector |

Static xrefs to the base/inside the matrix found only these four function bodies: `0x0042C290`, `0x0056C510`, `0x005840C0`, `0x005889F0`. No mutating xrefs were found in this slice.

### Column classification

`CellClass__RecalcZoneType @ 0x00483C80` writes `CellClass+0x4C`:

| Column | Binary source condition | Active in YR |
|---:|---|---|
| 0 | default ground | Yes |
| 1 | overlay type flag `+0x22D` | Yes |
| 2 | overlay type flag `+0x2A8`, or some overlay/bridge/building fallbacks | Yes |
| 3 | `LandType == 6` beach | Yes |
| 4 | `LandType == 2` water | Yes |
| 5 | building/object branch | Yes |
| 6 | speed-table zero/threshold, gate, blocking building/overlay cases | Yes |
| 7 | not in playfield | Yes |

This is a reduced zone type, not direct `SpeedType x LandType` and not the 12-value terrain `LandType` enum.

## 4. INI Keys

| Key | Binary field / parser | Effect in this slice | Active in YR |
|---|---|---|---|
| `MovementZone=` | `CCINIClass__ReadMovementZone @ 0x00474E40`; stored to `TechnoTypeClass+0x5B4` at `0x00716081` | direct row index for `MapClass__GetZoneID`, `Zone_precheck`, `ZoneMap__FloodFillReachableZones`, and team compatibility | Yes |
| `SpeedType=` | `CCINIClass__ReadSpeedType @ 0x00476FC0`; stored to `TechnoTypeClass+0x67C` at `0x007121E5` | not a direct matrix row selector in the four direct readers; used by separate terrain speed/cost systems | Yes |

## 5. Integration Points

Individual foot pathfinding:

1. `FootClass__Find_Path @ 0x004D3920` calls `FootClass__Run_AStar @ 0x004CBBA0`.
2. `FootClass__Run_AStar` calls `AStar_pathfind_search @ 0x0042C900`.
3. `AStar_pathfind_search` reads `TechnoTypeClass+0x5B4` when no explicit row override is passed, gets start/dest zone IDs through `MapClass__GetZoneID`, and calls `Zone_precheck @ 0x0042C290`.
4. `Zone_precheck` gates edges by `matrix[movementZone][edgeZoneType] == 1`.
5. If start and destination zones differ and hierarchy is enabled, `AStar_pathfind_search` returns failure before cell A*. If zones match but `Zone_precheck` fails, it logs and continues without hierarchy.

Zone map build/rebuild:

1. `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` groups cell clusters by zone type.
2. It builds 13 `MapClass+0x18[row]` arrays from the matrix. Non-`1` entries seed blocked/non-passable state.
3. It uses the same value equality grouping inside row traversal, so value `2` and value `3` are not passable; value `3` is not merged as ordinary passability.

Team category:

1. `TeamTypeClass__RecomputeAllZoneCategories @ 0x006F2040` calls `TeamTypeClass__ComputeZoneCategory @ 0x006F1FA0`.
2. `TeamTypeClass__ComputeZoneCategory` calls `ZoneMap__FindBestCompatibleMovementZone @ 0x005889F0`.
3. That helper considers all 13 candidate movement-zone rows.

## 6. Current Rust Implementation Status

Scanned surfaces:

| Surface | Status vs binary slice |
|---|---|
| `src/rules/locomotor_type.rs` | `MovementZone` now has 13 variants with explicit row values 0..12; this matches the matrix row contract. `SpeedType` variant order comment says it matches binary, but no `repr(u8)` is present, so raw numeric use should be checked before relying on enum discriminants. |
| `src/sim/pathfinding/passability.rs` | Matrix is `13x8`, but it is remapped to Rust `LandType` columns rather than storing the literal binary `ZoneType` columns. The `zone_layer_for_speed_type()` helper still maps `SpeedType` to matrix rows; this is not a direct binary reader behavior. |
| `src/sim/pathfinding/zone_map.rs` | Builds maps per `MovementZone::all_ground()` and uses `mz.speed_type()` to select cost grids. It excludes `Fly`; binary has a Fly row and builds all 13 row arrays in `0x0056C510`. |
| `src/sim/pathfinding/zone_search.rs` | Reduced precheck is deliberately gated to only some movement zones. Binary `Zone_precheck` accepts any movement-zone row passed by `AStar_pathfind_search`; the current Rust gating is a parity approximation, not the verified matrix contract. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x0082A594` dimensions | verified | `read_memory(416)`, loops to `0x82A734` | none |
| Value semantics `1/2/3` | verified | readers compare `== 1` or initialize blocked as `!= 1` | exact gameplay meaning of value `2` vs `3` beyond sentinel distinction is not expanded |
| Direct static readers | verified | `get_bulk_xrefs` base/offset xrefs; decompiled four bodies | runtime watchpoint not run |
| `Zone_precheck @ 0x0042C290` | verified | decompile; caller `0x0042C900` | full Dijkstra/cost logic out-of-scope |
| `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` | verified for matrix use | decompile matrix loop | full cluster/bridge graph build out-of-scope |
| `ZoneMap__FloodFillReachableZones @ 0x005840C0` | verified for matrix use | decompile row from `type+0x5B4` | full neighbor helper semantics out-of-scope |
| `ZoneMap__FindBestCompatibleMovementZone @ 0x005889F0` | verified | decompile; caller `0x006F1FA0` | exact team member argument source out-of-scope |
| `MovementZone=` row source | verified | `0x00474E40`, `0x00716079..0x00716081` | none for this slice |
| `SpeedType=` separation | verified | `0x00476FC0`, `0x007121E0..0x007121E5`; no direct matrix reader uses `+0x67C` | full speed/land cost pipeline out-of-scope |
| Rust surface scan | touched-not-exhausted | `rg`, Codegraph, selected file reads | no code change or test run per read-only scope |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is the matrix 12x8 or 13x8? -> 13x8 int32, 416 bytes, end `0x82A734`.` (evidence: `read_memory 0x0082A594 len 416`; `0x0056C510`, `0x005889F0`)
- `[RESOLVED] OQ-2 -- Which functions read the matrix directly? -> Four function bodies: `0x0042C290`, `0x0056C510`, `0x005840C0`, `0x005889F0`.` (evidence: `get_bulk_xrefs 0x0082A594...`)
- `[RESOLVED] OQ-3 -- What value is passable? -> only `1`; `2` and `3` block matrix-based zone traversal.` (evidence: `0x0042C290`, `0x005840C0`, `0x0056C510`)
- `[RESOLVED] OQ-4 -- Is row index SpeedType? -> No, direct readers use `MovementZone`/`TechnoType+0x5B4`.` (evidence: `0x00716081`, `0x0042C900`, `0x005840C0`)
- `[RESOLVED] OQ-5 -- Is SpeedType related here? -> SpeedType is parsed to `+0x67C` but is not a direct matrix row selector in this slice.` (evidence: `0x007121E5`; direct reader decompiles)
- `[RESOLVED] OQ-6 -- Is the A* path active in standard YR? -> Yes, `FootClass__Find_Path -> FootClass__Run_AStar -> AStar_pathfind_search -> Zone_precheck`.` (evidence: callers `0x004D3920`, `0x004CBBA0`, `0x0042C900`)
- `[RESOLVED] OQ-7 -- Is the team compatibility reader active? -> Conditional; active when TeamTypes compute zone categories, not individual A*.` (evidence: `0x006F2040 -> 0x006F1FA0 -> 0x005889F0`)
- `[RESOLVED] OQ-8 -- Is Fly represented in the matrix? -> Yes, row 9 exists and blocks column 7 sentinel only.` (evidence: memory row 9; loop covers 13 rows)
- `[DEFERRED] OQ-9 -- Exact writer-side semantics for every `CellClass+0x4C` branch.` (category: `out-of-scope`; reason: parent explicitly scoped out exact cell flag meanings; next-step-if-pursued: use full `0x00483C80`/`0x0047D2B0` writer investigation)
- `[DEFERRED] OQ-10 -- Runtime watchpoint proof for computed indirect readers.` (category: `needs-runtime-debugger`; reason: static Ghidra xrefs were sufficient for this slice; next-step-if-pursued: set read watchpoint on `0x0082A594` during map load/pathfind)
- `[DEFERRED] OQ-11 -- Full Rust behavioral diff for zone rebuild and A* retry logic.` (category: `out-of-scope`; reason: this swarm slot is read-only matrix-reader research; next-step-if-pursued: verify `zone_map.rs`/`zone_search.rs` against `0x0056C510` and `0x0042C290`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Matrix is literal `int[13][8]`; only value `1` passes, value `3` is blocked sentinel | `0x0082A594` dump; `0x0042C290`, `0x005840C0`, `0x0056C510` | partial: Rust stores a remapped `u8[13][8]` by local `LandType` buckets | `src/sim/pathfinding/passability.rs`, zone builder tests | Preserve binary-facing reduced `ZoneType` columns separately from local terrain buckets, or document remap at every use site | A Normal mover cannot cross zone type 1/2/3/4/5/6/7, Fly cannot cross zone type 7, and no `3` entry is treated passable; proposed test `passability_matrix_binary_values_only_one_passes` | Do not collapse `2` and `3` into "maybe passable"; only `1` passes |
| Direct path/zone readers index by `MovementZone`, not `SpeedType` | `0x00716081`, `0x0042C900`, `0x0042C290`, `0x005840C0` | mismatch risk: `zone_layer_for_speed_type()` still exists and `TerrainCostGrid` fallback uses it | `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/terrain_cost.rs`, `src/sim/pathfinding/zone_map.rs` | Keep matrix legality/reachability keyed by `MovementZone`; keep `SpeedType` for speed/cost only unless a separate verified reader says otherwise | Unit with `MovementZone=Water` and `SpeedType=Float` uses Water row for zone reachability; changing SpeedType alone does not change matrix row; proposed test `zone_reachability_uses_movement_zone_not_speed_type` | Do not implement `SpeedType x LandType` as the zone passability matrix |
| Binary rebuild creates per-row zone arrays for all 13 movement-zone rows, including row 9 Fly | `0x0056C510` frees 13 pointers and loops matrix rows until `0x82A734` | mismatch/unchecked: Rust `MovementZone::all_ground()` excludes `Fly` and precheck treats Fly as trivially reachable | `src/rules/locomotor_type.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs` | Decide whether Fly needs a built zone map for standard jumpjet/fly pathing; if retained as shortcut, explicitly preserve row-9 sentinel/OOB blocking where cell legality is checked | Fly-zone mover should not treat sentinel/out-of-playfield column as reachable; proposed test `fly_movement_zone_blocks_sentinel_zone_type` | Do not model Fly as "all cells including OOB"; binary row 9 has final value `3` |

### Negative Facts / Do Not Do

- Do not describe `0x0082A594` as 12x8. Active in YR: Yes; evidence: both `0x0056C510` and `0x005889F0` loop to `0x82A734`, and the 416-byte dump is `13*8*4`.
- Do not use `SpeedType` as the matrix row. Active in YR: Yes; evidence: row source is `MovementZone=` stored to `TechnoType+0x5B4`, while `SpeedType=` stores to `+0x67C` and is not used by the direct matrix readers.
- Do not treat value `3` as special passability. Active in YR: Yes; evidence: `Zone_precheck` and `FloodFillReachableZones` require `==1`; `UpdateBridgeZonesHelper` seeds blocked state as `!=1`.
- Do not treat matrix columns as raw terrain/INI `LandType` or `SpeedType` columns. Active in YR: Yes; evidence: `CellClass__RecalcZoneType @ 0x00483C80` writes reduced zone types 0..7 to `CellClass+0x4C`.
- Do not assume CrusherAll has a unique matrix profile. Active in YR: Yes; evidence: row 12 values match row 2 exactly in the 416-byte dump; behavioral differences are outside this matrix.

### Remaining Uncertainty

- Static Ghidra xrefs found the direct reader set; no runtime watchpoint was run, so unusual computed-pointer reads are not independently observed.
- Exact writer-side meanings for every `CellClass+0x4C` branch remain out of scope for this slot.
- Whether every standard Fly/air locomotor path reaches the matrix readers was not fully traced; row 9 existence and parser activity are verified.

### Stale Docs / Follow-up Docs

- `docs/research/ADDRESS_MAP.md`: replace `0x0082A594 | int[12][8] | Passability matrix (hardcoded)` with `0x0082A594 | int[13][8] / 416 bytes | ZonePassabilityMatrix; rows are MovementZone 0..12, columns are reduced CellClass zone type 0..7, only value 1 passes`.
- `docs/research/ADDRESS_MAP.md`: replace `ZonePassabilityMatrix | 13x8 i32 (1=pass/2=block/3=special)` with `ZonePassabilityMatrix | 13x8 i32 (1=pass; 2=blocked; 3=blocked sentinel/out-of-bounds; only ==1 passes)`.
- `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: replace any wording of `g_PassabilityMatrix[speed_type * 8 + edge_type]` or `SpeedType x LandType` with `g_PassabilityMatrix[movementZone * 8 + reducedZoneType]`, where `movementZone` is `TechnoTypeClass+0x5B4` and `reducedZoneType` is `CellClass+0x4C`.
- `docs/research/ZONE_PASSABILITY_VERIFIED.md`: no dimension replacement needed in the checked current text; its 13x8 and `3`-blocked correction matches this investigation.

## Sources

- Ghidra `read_memory`: `0x0082A594`, length 416.
- Ghidra `get_bulk_xrefs`: `0x0082A594`, `0x0082A598`, `0x0082A59C`, `0x0082A5B4`.
- Ghidra decompiled: `0x0042C290`, `0x0042C900`, `0x004CBBA0`, `0x004D3920`, `0x00483C80`, `0x00474E40`, `0x00476FC0`, `0x0056C510`, `0x0056D100`, `0x0056D230`, `0x005840C0`, `0x005889F0`, `0x006F1FA0`, `0x006F2040`.
- Ghidra assembly context: `0x00716065..0x00716081`, `0x007121D1..0x007121E5`.
- Prior docs checked: `ZONE_PASSABILITY_VERIFIED.md`, `ADDRESS_MAP.md`, `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`, pathfinding/zone related reports via `rg`.
- Rust surfaces scanned: `src/rules/locomotor_type.rs`, `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/terrain_cost.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs`, `src/app_sim_tick.rs`.
