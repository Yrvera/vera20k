# Bridge Zone Edge Flag Writer Semantics -- Ghidra Research Report

**Address(es):** `0x0042C290` (`Zone_precheck` reader), `0x00581F90` (`ZoneMap__BuildZoneLevel` final writer), `0x005824A0` (`ZoneMap__FloodFillScanline` nonzero temp writer), `0x00582D70` (bridge/tube temp-edge injector), `0x005851B0` (`MapClass__AddBridgeZoneEdges` repaired-bridge final writer), `0x00584E50` (`MapClass__RemoveBridgeZoneEdges`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Confirm writer semantics for the final hierarchical zone-edge record byte at `edge+4` consumed by `Zone_precheck`, with emphasis on bridge edge creation, collapse/removal, and repair/re-add paths.  
**Non-Scope:** Full `Zone_precheck` hierarchy/cost behavior, full cell A* retry behavior, exact stock-map route choice after collapse, and Rust code changes.  
**Confidence:** High for the scoped writer/reader semantics; Medium for stock-map frequency of nonzero flags because runtime frequency was not measured.  
**Active in YR:** Yes. `Zone_precheck` is called by standard pathfinding (`AStar_pathfind_search` xrefs `0x0042CB58`, `0x0042CCB3`); zone graph build/update functions are reached from map init and bridge validate/invalidate paths.

## 0. Working Notes

- Target question: Who writes the `edge+4` byte consumed by `Zone_precheck`, what does nonzero mean, and do bridge creation/collapse/repair paths write it nonzero or zero?
- Non-goals: Do not rediscover the full `Zone_precheck` algorithm, all retry behavior, or stock-map route outcomes.
- Evidence needed to mark COMPLETE: Reader site for `edge+4`, nonzero writer site, final copy site, bridge/tube writer sites, bridge removal/repair integration, current Rust metadata status.
- Stop conditions: Stop after proving writer semantics and Rust-facing metadata consequence; put route frequency and broader pathing parity in Remaining Uncertainty.

## 1. Overview

The final hierarchical zone edge is an 8-byte record: `edge+0` is the neighbor zone id and the low byte of `edge+4` is a cost flag. `Zone_precheck` adds `0.001` only when that low byte is nonzero.

The nonzero writer is not bridge-specific. Nonzero is produced by `ZoneMap__FloodFillScanline` while building hierarchy-block boundary connections; `ZoneMap__BuildZoneLevel` copies that temporary flag into both final directed edge records. Verified bridge/tube writers set the flag low byte to zero, and repaired bridge edges added by `MapClass__AddBridgeZoneEdges` also write zero.

## 2. Class Layout / Key Offsets

| Structure | Offset / stride | Meaning | Active in YR |
|---|---:|---|---|
| Final zone edge | stride `8`, `+0x00` | Neighbor zone id read by `Zone_precheck`. | Yes; `Zone_precheck @ 0x0042C290`. |
| Final zone edge | stride `8`, `+0x04` low byte | Flag byte; nonzero selects the `0.001` addend. | Yes; `0x0042C540`, `0x0042C59E..0x0042C5AE`. |
| Final zone block | `+0x04` | Edge array pointer. | Yes; final graph read/written by build and precheck. |
| Final zone block | `+0x10` | Edge count. | Yes; `Zone_precheck` scans this count. |
| Temporary connection entry | stride `12`, `+0x00` | Packed zone pair. | Yes; `ZoneMap__BuildZoneLevel` temp graph. |
| Temporary connection entry | stride `12`, `+0x04` | Duplicate packed zone pair used by final emission. | Yes; final emitter reads this before appending directed edges. |
| Temporary connection entry | stride `12`, `+0x08` low byte | Source flag copied into final `edge+4` low byte. | Yes; copied at final emission. |
| Bridge record | stride `0x10`, `+0x08` | Intact/active byte. | Yes; bridge validate/invalidate toggles it. |
| Bridge record | stride `0x10`, `+0x0C` | Bridge kind; `0` high bridge, `1` low/tube record. | Conditional for some helpers; `FindBridgeRecord` skips nonzero kind, while full build can fold active low/tube records. |

## 3. Core Logic

### 3.1 Reader: `Zone_precheck` consumes `byte(edge+4)`

`Zone_precheck @ 0x0042C290` reads the low byte of final edge `+4` while scanning a zone block's edge array.

Verified behavior:

- `0x0042C53E` reads final `edge+0` neighbor zone id.
- `0x0042C540` reads `byte(edge+4)`.
- `0x0042C59E..0x0042C5AE` tests the byte; zero selects `0.0`, nonzero loads the `0.001` double from `0x007E3818`.
- `0x0042C5BB..0x0042C5D2` adds the selected flag addend to target-zone base cost, parent cost, and optional slope cost.

Active in YR: Yes. Evidence: direct xrefs from `AStar_pathfind_search` at `0x0042CB58` and `0x0042CCB3`, and from `FUN_0042D170` at `0x0042D222`.

### 3.2 Nonzero writer: hierarchy scanline boundary temp entries

`ZoneMap__FloodFillScanline @ 0x005824A0` writes temporary 12-byte connection entries. Most insertion paths clear the flag low byte with `& 0xFFFFFF00`. Two vertical-neighbor boundary branches can set it to `1`:

- `0x00582A28..0x00582A3E`: sets temp flag low byte to `1` if the x coordinate is outside the current hierarchy build block's horizontal range.
- `0x00582C70..0x00582C86`: symmetric branch with the same outside-range condition.

Verified condition:

```text
temp_flag_low_byte = (x < block_x_min || x > block_x_max) ? 1 : 0
```

Active in YR: Yes. Evidence: `ZoneMap__BuildZoneLevel` calls `ZoneMap__FloodFillScanline` during full hierarchy construction (`0x00582187`), and `FUN_00584550` calls it during incremental rebuild (`0x00584925`).

### 3.3 Final writer: temp flag copied to both directed final edges

`ZoneMap__BuildZoneLevel @ 0x00581F90` emits final 8-byte edges from the temporary connection graph.

Verified behavior:

- `0x00582398`: reads the temp flag low byte from temporary entry `+8`.
- `0x00582402`: writes copied flag dword to the first directed final edge `+4`.
- `0x0058245B`: writes copied flag dword to the reverse directed final edge `+4`.

The same final-emission shape exists in the incremental rebuild sibling `FUN_00584550`; prior evidence records the analogous writer loop at `0x00584B55..0x00584C52`.

Active in YR: Yes. Evidence: `ZoneMap__BuildZoneLevel` xrefs from full map init `0x0056720D`, `FUN_00581F50 @ 0x00581F6A`, and incremental rebuild `0x00584D79`.

### 3.4 Bridge/tube full-build injector writes zero temp flags

`FUN_00582D70` injects active bridge/tube connectivity into the temporary graph used by `ZoneMap__BuildZoneLevel`. It computes three connection pairs, performs exact duplicate checks, and inserts temp entries with flag low byte zero.

Verified behavior:

- Decompile initializes the local flag words to zero and sets `local_4 = 0` before each of the three pair insertions.
- Helper-call sites `0x0058304B`, `0x005830E5`, and `0x00583165` insert the bridge/tube temp entries with that zero flag.
- Therefore active bridge/tube edges included during full hierarchy build do not receive the `0.001` `Zone_precheck` flag cost because of being bridge edges.

Active in YR: Yes. Evidence: `FUN_00582D70` xrefs from `ZoneMap__BuildZoneLevel @ 0x00582358`, `FUN_00584550 @ 0x00584B2E`, and `FUN_00582D30 @ 0x00582D57`. Standard maps with bridge/tube records use these paths when records are active.

### 3.5 Bridge repair/revalidation direct adder writes zero final flags

`MapClass__ValidateBridgeZones @ 0x0056DB70` marks matching inactive high-bridge records active and calls `MapClass__AddBridgeZoneEdges @ 0x005851B0`. `AddBridgeZoneEdges` writes final graph edges directly rather than going through the temp graph.

Verified behavior:

- `MapClass__AddBridgeZoneEdges` zero-masks all six local flag dwords before its three-level loop.
- The first final directed insert writes neighbor and flag at `0x0058535E..0x00585361`; the flag operand is the zero-masked local dword.
- The pattern repeats for the other directed inserts in the function.

Active in YR: Yes. Evidence: direct xref from `MapClass__ValidateBridgeZones @ 0x0056DBD6`. This is the bridge repair/validation path.

### 3.6 Bridge collapse/removal ignores the flag

`MapClass__InvalidateBridgeZones @ 0x0056DAE0` calls `MapClass__RemoveBridgeZoneEdges @ 0x00584E50` for matching active high-bridge records, then clears bridge record `+0x08`.

Verified behavior:

- `MapClass__RemoveBridgeZoneEdges` removes bridge final edges by neighbor-zone lookup/removal.
- It does not use `edge+4` as a match key and does not rewrite remaining edge flag bytes.

Active in YR: Yes. Evidence: direct xref from `MapClass__InvalidateBridgeZones @ 0x0056DB3B`; bridge collapse/invalidation paths call this when a high bridge record becomes inactive.

### 3.7 Retry/exclusion helpers are not final `edge+4` writers

`PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0` and `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` write Pathfinder-local per-search edge exclusions. They pack sorted undirected zone pairs into `Pathfinder+0x78/+0x84` style vectors consumed by `Zone_precheck`.

Verified behavior:

- These helpers read final graph neighbor ids but do not mutate the final graph edge array.
- They do not read or write final `edge+4`.

Active in YR: Yes. Evidence: retry path from `AStar_pathfind_search @ 0x0042CC79`; exclusion consumption is separate from `edge+4` cost consumption.

## 4. INI Keys

No INI key directly writes or controls the final `edge+4` flag byte.

| Key / data | Role | Active in YR |
|---|---|---|
| `MovementZone=` | Selects the `Zone_precheck` passability row through existing movement-zone plumbing; it is not a writer or semantic source for `edge+4`. | Yes |
| Bridge/tube map data | Supplies bridge records and tube connectivity; verified bridge/tube writers set this flag to zero. | Yes/Conditional, depending on map content |

## 5. Integration Points

| Function/path | Role for `edge+4` | Active in YR | Evidence |
|---|---|---|---|
| `FUN_00567110 -> ZoneMap__BuildZoneLevel` | Full map zone graph construction; copies temp flags to final edges. | Yes | `ZoneMap__BuildZoneLevel` xref `0x0056720D`. |
| `FUN_00584550` | Incremental local zone rebuild; reuses `ZoneMap__FloodFillScanline` and final emission shape. | Yes | `ZoneMap__FloodFillScanline` xref `0x00584925`, `BuildZoneLevel` xref `0x00584D79`. |
| `ZoneMap__FloodFillScanline` | Only verified nonzero writer in this slice. | Yes | Nonzero branches `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86`. |
| `FUN_00582D70` | Bridge/tube temp-edge injector; writes zero flags. | Yes | Xrefs `0x00582358`, `0x00584B2E`; decompile zero flags. |
| `MapClass__ValidateBridgeZones -> AddBridgeZoneEdges` | Bridge repair direct final-edge add; writes zero flags. | Yes | Xref `0x0056DBD6`; writer `0x0058535E..0x00585361`. |
| `MapClass__InvalidateBridgeZones -> RemoveBridgeZoneEdges` | Bridge collapse/removal path; ignores flag. | Yes | Xref `0x0056DB3B`; remove-by-neighbor decompile. |
| `AStar_pathfind_search -> Zone_precheck` | Reader and cost consumer. | Yes | Xrefs `0x0042CB58`, `0x0042CCB3`. |

## 6. Current Rust Implementation Status

Current Rust does not model final edge metadata:

- `src/sim/pathfinding/zone_map.rs` defines `ZoneAdjacency` as `Vec<Vec<ZoneId>>`, with no per-edge flag byte.
- `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency` appends neighbor IDs only; there is no bridge edge flag field, which avoids the wrong "all bridge edges are flagged" behavior but cannot model the real `0.001` tiebreak flag.
- `src/sim/pathfinding/zone_search.rs::find_zone_corridor` still uses Rust-side zone corridor costs, so even if metadata existed it is not yet consumed in a binary-style accumulated `Zone_precheck` cost.

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_precheck` `edge+4` read and `0.001` addend | verified | `0x0042C540`, `0x0042C59E..0x0042C5AE`, `0x007E3818` | none for scoped reader |
| Nonzero writer condition | verified | `ZoneMap__FloodFillScanline @ 0x005824A0`, `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86` | runtime frequency on stock maps |
| Final temp-to-edge copy | verified | `ZoneMap__BuildZoneLevel @ 0x00581F90`, `0x00582398`, `0x00582402`, `0x0058245B` | none |
| Incremental rebuild equivalent | verified for same semantics | `FUN_00584550`, xrefs `0x00584925`, `0x00584B2E`, `0x00584D79`; prior writer loop `0x00584B55..0x00584C52` | repeated mutation duplicate lifecycle is separate |
| Bridge/tube temp-edge injector | verified | `FUN_00582D70`, helper calls `0x0058304B`, `0x005830E5`, `0x00583165` | none for flag byte |
| Bridge repair final-edge direct writer | verified | `MapClass__AddBridgeZoneEdges @ 0x005851B0`, `0x0058535E..0x00585361`, xref `0x0056DBD6` | none for flag byte |
| Bridge collapse/removal | verified for not using flag | `MapClass__RemoveBridgeZoneEdges @ 0x00584E50`, xref `0x0056DB3B` | exact duplicate-edge lifecycle out of scope |
| Retry exclusion helpers | verified as non-writers | `0x0042CCD0`, `0x0042CF80`; prior retry report | none for final `edge+4` |
| Rust metadata status | verified by scan | `zone_map.rs`, `zone_build.rs`, `zone_search.rs` | no implementation performed |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is the target exhaustive enough for one pass? -> yes, the slice is the final edge +4 byte, not full pathing.` (evidence: user scope and report scope)
- `[RESOLVED] OQ-2 -- What reads `edge+4`? -> `Zone_precheck` reads `byte(edge+4)` and uses it to select `0.001` vs `0.0`.` (evidence: `0x0042C540`, `0x0042C59E..0x0042C5AE`)
- `[RESOLVED] OQ-3 -- Is the reader active in YR? -> yes, standard pathfinding calls `Zone_precheck`.` (evidence: xrefs `0x0042CB58`, `0x0042CCB3`)
- `[RESOLVED] OQ-4 -- Who writes nonzero? -> `ZoneMap__FloodFillScanline` writes temp flag low byte `1` for outside-block horizontal-range vertical boundary contacts.` (evidence: `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86`)
- `[RESOLVED] OQ-5 -- How does temp flag become final `edge+4`? -> `ZoneMap__BuildZoneLevel` copies temp `+8` into both directed final `edge+4` writes.` (evidence: `0x00582398`, `0x00582402`, `0x0058245B`)
- `[RESOLVED] OQ-6 -- Do active bridge/tube full-build edges write nonzero? -> no, `FUN_00582D70` writes zero temp flags.` (evidence: `0x00582D70`, `0x0058304B`, `0x005830E5`, `0x00583165`)
- `[RESOLVED] OQ-7 -- Do repaired bridge direct edges write nonzero? -> no, `AddBridgeZoneEdges` writes zero-masked flag dwords.` (evidence: `0x005851B0`, `0x0058535E..0x00585361`)
- `[RESOLVED] OQ-8 -- Does bridge collapse/removal match by flag? -> no, `RemoveBridgeZoneEdges` removes by neighbor-zone lookup and ignores `edge+4`.` (evidence: `0x00584E50`, xref `0x0056DB3B`)
- `[RESOLVED] OQ-9 -- Is this a bridge-edge flag? -> no verified bridge writer sets it nonzero; best verified name is hierarchy-boundary/tiebreak flag.` (evidence: `0x005824A0`, `0x00582D70`, `0x005851B0`)
- `[RESOLVED] OQ-10 -- Does Rust already store this metadata? -> no, `ZoneAdjacency` stores only neighbor zone IDs.` (evidence: `src/sim/pathfinding/zone_map.rs`)
- `[DEFERRED] OQ-11 -- How often do stock maps produce nonzero final flags?` (category: needs-runtime-debugger; reason: static writer/reader semantics are proven but runtime frequency was not measured; next-step-if-pursued: log final graph edge flags during stock map load and incremental bridge changes)
- `[DEFERRED] OQ-12 -- Exact route change caused by this `0.001` on a named bridge-collapse map.` (category: needs-runtime-debugger; reason: requires route trace, not writer semantics; next-step-if-pursued: capture `Zone_precheck` candidate chain and A* result before/after low bridge collapse)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Final zone edges have an `edge+4` low-byte flag; `Zone_precheck` adds `0.001` only when nonzero. Active in YR: Yes. | `0x0042C540`, `0x0042C59E..0x0042C5AE`, `0x007E3818` | missing | `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs` | When binary-style zone precheck is implemented, store per-edge metadata and add the tiny cost only for flagged edges. | Two equal otherwise routes differ only by this flag; unflagged route wins. Proposed test name: `zone_precheck_edge_flag_adds_tiny_tiebreak_cost`. | Do not apply this as a universal bridge or diagonal penalty. |
| Nonzero flags come from hierarchy scanline build-boundary temp entries and are copied to both final directed edges. Active in YR: Yes. | `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86`, `0x00582402`, `0x0058245B` | missing | `src/sim/pathfinding/zone_build.rs`, future exact hierarchy builder | Preserve the block-boundary writer condition if exact hierarchy graph emission is added. | A block-boundary vertical adjacent-zone contact emits `flag=1`, while an in-block contact emits `flag=0`. Proposed test name: `zone_build_marks_out_of_block_vertical_edges_with_tiebreak_flag`. | Do not derive the flag from terrain names, bridge records, or INI keys. |
| Bridge/tube full-build injection and repaired-bridge direct additions write zero flags; collapse/removal ignores the flag. Active in YR: Yes. | `FUN_00582D70`, `0x0058304B`, `0x005830E5`, `0x00583165`; `MapClass__AddBridgeZoneEdges @ 0x005851B0`; `0x00585361`; `MapClass__RemoveBridgeZoneEdges @ 0x00584E50` | Rust has no field; if added later, bridge edges must default to zero | `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency`, future bridge repair/incremental update surface | Bridge-created and repaired adjacency edges should be valid graph edges but unflagged for the `0.001` precheck cost. | Repairing or rebuilding a bridge adds adjacency with `flag=0`; the path cost does not gain the `0.001` flag solely because the edge is a bridge edge. Proposed test name: `repaired_bridge_zone_edges_are_unflagged_for_precheck_cost`. | Do not mark all bridge edges as flagged/nonzero. |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
  - Replace any wording equivalent to: "`edge+4` is a bridge-edge flag" or "`0.001` is a bridge-edge penalty."
  - With: "`byte(edge+4) != 0` is a hierarchy graph build-boundary/tiebreak flag consumed by `Zone_precheck`; verified bridge/tube and repaired-bridge edge writers set this byte to zero."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`
  - Replace open wording that says the bridge-edge low-byte semantic still needs a second writer.
  - With: "Resolved by `BRIDGE_ZONE_EDGE_FLAG_WRITER_SEMANTICS_GHIDRA_REPORT.md`: nonzero comes from `ZoneMap__FloodFillScanline` hierarchy-boundary temp entries; bridge/tube helpers and repaired-bridge direct adds write zero."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md`
  - Keep the correction, but prefer "verified nonzero writer for this slice" and "hierarchy-boundary/tiebreak flag" over "bridge-edge flag."

## Negative Facts / Do Not Do

- Do not implement `edge+4 != 0` as "bridge edge." Active in YR: Yes. Evidence: bridge/tube temp injection and repaired bridge direct add write zero (`0x00582D70`, `0x005851B0`).
- Do not add `0.001` to every bridge adjacency edge. Active in YR: Yes. Evidence: repaired bridge final writer uses zero flag at `0x00585361`.
- Do not derive this flag from INI keys or `MovementZone=`. Active in YR: Yes. Evidence: no INI writer; writer is zone graph construction (`0x005824A0`, `0x00581F90`).
- Do not let retry exclusions mutate final graph `edge+4`. Active in YR: Yes. Evidence: retry helpers write Pathfinder-local packed edge exclusions (`0x0042CCD0`, `0x0042CF80`).
- Do not treat bridge collapse as a flag rewrite. Active in YR: Yes. Evidence: `RemoveBridgeZoneEdges` removes bridge edges and ignores flag matching (`0x00584E50`).

## Remaining Uncertainty

- Runtime frequency of nonzero flagged final edges on stock maps is not measured.
- Exact route choice impact after a named bridge collapse requires runtime route tracing; static writer semantics alone do not prove a player-visible detour difference.
- Full duplicate-edge lifecycle after repeated collapse/repair cycles is outside this writer-semantics slice.

## Sources

- Ghidra decompiled/checked: `0x0042C290`, `0x00581F90`, `0x005824A0`, `0x00582D70`, `0x00584550`, `0x005851B0`, `0x00584E50`, `0x0056DB70`, `0x0056DAE0`, `0x0042CCD0`, `0x0042CF80`.
- Ghidra xrefs checked: `Zone_precheck` from `0x0042CB58`, `0x0042CCB3`, `0x0042D222`; `ZoneMap__BuildZoneLevel` from `0x0056720D`, `0x00581F6A`, `0x00584D79`; `ZoneMap__FloodFillScanline` from `0x00582187`, `0x00584925`; `FUN_00582D70` from `0x00582358`, `0x00584B2E`; `AddBridgeZoneEdges` from `0x0056DBD6`; `RemoveBridgeZoneEdges` from `0x0056DB3B`.
- Prior docs referenced: `ZONE_EDGE_RECORD_BYTE_PLUS_4_WRITER_SEMANTICS_GHIDRA_REPORT.md`, `BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md`, `ZONE_GRAPH_ADJACENCY_EMISSION_ORDER_GHIDRA_REPORT.md`, `PATHFINDER_ZONE_EDGE_UPDATE_INVALIDATION_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`.
