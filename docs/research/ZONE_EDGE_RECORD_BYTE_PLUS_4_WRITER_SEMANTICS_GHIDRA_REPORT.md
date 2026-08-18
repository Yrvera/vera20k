# Zone Edge Record Byte +4 Writer Semantics -- Ghidra Research Report

**Address(es):** `0x0042C290` (`Zone_precheck`), `0x00581F90` (`ZoneMap__BuildZoneLevel`), `0x005824A0` (`ZoneMap__FloodFillScanline`), `0x005851B0` (`MapClass__AddBridgeZoneEdges`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** writer and reader semantics for the final hierarchical zone-edge record byte at `edge+4`, specifically the byte read by `Zone_precheck` to add `0.001`.  
**Non-Scope:** full `Zone_precheck`, full A* retry behavior, full bridge destruction/repair lifecycle, and Rust implementation changes.  
**Confidence:** High  
**Active in YR:** Yes. `Zone_precheck` is reached from `AStar_pathfind_search @ 0x0042C900` at `0x0042CB58` / `0x0042CCB3` and from `FUN_0042D170` at `0x0042D222`; the final graph is built by map init `FUN_00567110 -> ZoneMap__BuildZoneLevel` and updated by incremental rebuild/bridge validation paths.

## 1. Overview

The final zone graph edge is an 8-byte record: `edge+0` is the neighbor zone id and `edge+4` is a flag dword whose low byte is consumed by `Zone_precheck`. If that low byte is nonzero, `Zone_precheck` adds exactly `0.001` to the candidate zone-edge cost.

The nonzero writer is not the bridge-specific add/repair helper. Nonzero comes from `ZoneMap__FloodFillScanline` temporary connection entries when a vertical adjacent-zone connection crosses outside the current build block's horizontal range; `ZoneMap__BuildZoneLevel` copies that temp flag into both final directed edge records.

## 2. Class Layout / Key Offsets

| Structure | Offset | Type | Verified meaning |
|---|---:|---|---|
| Final zone edge | `+0x00` | `u32` | Neighbor zone id, read by `Zone_precheck` before cost/passability checks. |
| Final zone edge | `+0x04` low byte | `u8` | Edge flag; nonzero selects the `0.001` addend in `Zone_precheck`. |
| Final zone block | `+0x04` | pointer | Final edge array, 8-byte stride. |
| Final zone block | `+0x10` | `i32` | Final edge count scanned by `Zone_precheck`. |
| Temporary connection entry | `+0x00` | `u32` | Packed zone pair. |
| Temporary connection entry | `+0x04` | `u32` | Duplicate packed zone pair used by build emission. |
| Temporary connection entry | `+0x08` low byte | `u8` | Source flag copied to final `edge+4` low byte by full/incremental build. |

## 3. Core Logic

### Reader: `Zone_precheck @ 0x0042C290`

`Zone_precheck` reads the edge flag while scanning a zone block's final edge array:

- `0x0042C53E`: reads `edge+0` neighbor zone id.
- `0x0042C540`: reads `byte(edge+4)` into a stack byte.
- `0x0042C59E..0x0042C5AE`: tests that byte; zero selects `0.0`, nonzero loads `0.001` from `0x007E3818`.
- `0x0042C5BB..0x0042C5D2`: adds target-zone base cost, parent cost, optional slope cost, and the selected flag addend.

Search check: exact byte pattern `8A 50 04 88 54 24 11` occurs at `0x0042C540`, matching the scoped reader site.

### Writer chain: scanline temporary flag -> final `edge+4`

`ZoneMap__FloodFillScanline @ 0x005824A0` inserts temporary 12-byte connection entries. Most insert paths clear the flag low byte with `& 0xFFFFFF00`. Two vertical boundary paths can set it to `1`:

- `0x00582A28..0x00582A3E`: if the x coordinate is left of `block_x_min` or right of `block_x_max`, the temp flag byte is set to `1`; otherwise it is cleared to `0`.
- `0x00582C70..0x00582C86`: symmetric lower/upper-row branch with the same condition.

In pseudocode, the verified condition is:

```text
temp_flag_low_byte = (x < block_x_min || x > block_x_max) ? 1 : 0
```

`ZoneMap__BuildZoneLevel @ 0x00581F90` later emits final bidirectional edges from those temporary entries:

- `0x00582398`: reads the temp flag low byte from temp entry `+8`.
- `0x00582402`: writes the copied flag dword to forward final `edge+4`.
- `0x0058245B`: writes the copied flag dword to reverse final `edge+4`.

`FUN_00584550 @ 0x00584550` is the incremental rebuild sibling. Its decompile shows the same temporary-entry emission pattern and final 8-byte edge writes inside the local rebuild loop; the same `ZoneMap__FloodFillScanline` function supplies the temp flag.

### Bridge/tube writers write zero

`FUN_00582D70` injects active bridge/tube connectivity into the temporary graph during full/incremental zone rebuild. It computes three connection pairs and sets `local_4 = 0` before each temporary insert; helper calls at `0x0058304B`, `0x005830E5`, and `0x00583165` copy that zero flag into temp entry `+8`.

`MapClass__AddBridgeZoneEdges @ 0x005851B0` is the direct repaired/validated-bridge edge adder. It writes final `edge+4` directly, but all six local flag dwords are masked to zero low byte before the three-level loop. Example evidence: first directed insert writes neighbor and flag at `0x0058535E..0x00585361`; analogous direct edge flag writes occur for the other bridge-derived directed inserts. Therefore repaired bridge-added edges do not trigger the `0.001` edge-flag cost.

`MapClass__RemoveBridgeZoneEdges @ 0x00584E50` removes bridge edges by neighbor-zone lookup/removal. It does not use `edge+4` as a match key.

`PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` and `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0` write per-search packed undirected edge exclusions into `Pathfinder+0x78/+0x84` style vectors. They read final graph neighbor ids, but do not read or mutate final `edge+4`.

## 4. INI Keys

No INI key directly controls the `edge+4` flag. `MovementZone=` is still relevant to `Zone_precheck` passability matrix row selection, but it is not a writer or semantic source for this byte.

## 5. Integration Points

| Function/path | Role for `edge+4` | Active in YR |
|---|---|---|
| `FUN_00567110 -> ZoneMap__BuildZoneLevel` | Full map zone graph construction; copies temp flags to final edges. | Yes; map zone initialization path. |
| `FUN_00584550` | Incremental local zone rebuild; reuses `ZoneMap__FloodFillScanline` temp flag semantics. | Yes; reached from terrain/cell edit paths per prior reports and direct `ZoneMap__BuildZoneLevel` xref at `0x00584D79`. |
| `FUN_00582D70` | Bridge/tube temp-edge injector; writes zero temp flags. | Yes; called from full build, incremental rebuild, and `FUN_00582D30`. |
| `MapClass__ValidateBridgeZones -> MapClass__AddBridgeZoneEdges` | Repaired bridge direct final-edge add; writes zero final flags. | Yes; bridge repair/validation path, xref `0x0056DBD6`. |
| `MapClass__InvalidateBridgeZones -> MapClass__RemoveBridgeZoneEdges` | Bridge direct final-edge removal; ignores flag. | Yes; bridge invalidation/destruction path, xref `0x0056DB3B`. |
| `AStar_pathfind_search -> Zone_precheck` | Cost reader; adds `0.001` if byte is nonzero. | Yes; standard pathfinding spine. |
| `FUN_0042D170 -> Zone_precheck` | Alternate cost/reachability caller; same reader. | Yes; `FootClass::Find_Path` blocked-destination helper path per prior reports. |

## 6. Current Rust Implementation Status

Current Rust does not model this exact final edge metadata:

- `src/sim/pathfinding/zone_map.rs` stores `ZoneAdjacency` as `Vec<Vec<ZoneId>>`, with no per-edge flag byte.
- `src/sim/pathfinding/zone_build.rs` injects bridge adjacency into Rust zone graphs, but there is no temporary 12-byte connection entry or final `edge+4` metadata.
- `src/sim/pathfinding/zone_search.rs::find_zone_corridor` uses centroid Manhattan `g+h` and whole-zone exclusions, not binary `Zone_precheck` accumulated reduced-zone cost plus optional edge flag `0.001`.

No Rust files were modified by this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_precheck @ 0x0042C290` `edge+4` read | verified | `0x0042C540`, `0x0042C59E..0x0042C5AE` | none for scoped read |
| Exact `0.001` constant | verified | `0x007E3818` load when flag byte nonzero | none |
| `ZoneMap__FloodFillScanline @ 0x005824A0` nonzero temp writer | verified | `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86` | none for scoped condition |
| `ZoneMap__BuildZoneLevel @ 0x00581F90` final writer | verified | `0x00582398`, `0x00582402`, `0x0058245B` | none |
| `FUN_00584550` incremental rebuild | verified for reuse of same semantics | decompile plus `ZoneMap__FloodFillScanline` xrefs `0x00584925`; final emission loop mirrors build | no broader terrain lifecycle timing claimed |
| `FUN_00582D70` bridge/tube temp injection | verified | decompile, helper calls `0x0058304B`, `0x005830E5`, `0x00583165`; `local_4 = 0` before inserts | none for flag byte |
| `MapClass__AddBridgeZoneEdges @ 0x005851B0` repaired bridge direct writer | verified | `0x0058535E..0x00585361` plus zero-masked flag locals | none for flag byte |
| `MapClass__RemoveBridgeZoneEdges @ 0x00584E50` | verified for ignoring flag | decompile uses neighbor-zone find/remove; no flag-key match | exact duplicate-edge removal ordering remains separate lifecycle concern |
| `PathfinderClass__InvalidateZoneEdge @ 0x0042CF80` | verified non-writer/non-reader of final flag | reads neighbor ids and writes exclusion vectors | none for final `edge+4` |
| Whole-binary exhaustive field-use audit | deferred | scoped static slice plus exact reader pattern search | not needed for requested writer semantics; use debugger watchpoint if future runtime proof is needed |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is the report exhaustive-slice or coverage-map? -> exhaustive-slice for the final zone edge byte at edge+4 and its direct writer/reader semantics.` (evidence: user scope)
- `[RESOLVED] OQ-2 -- What reads final edge+4 for the 0.001 cost? -> Zone_precheck reads byte(edge+4) at 0x0042C540 and selects 0.001 at 0x0042C59E..0x0042C5AE.` (evidence: `0x0042C540`, `0x007E3818`)
- `[RESOLVED] OQ-3 -- What writes nonzero? -> ZoneMap__FloodFillScanline writes temp flag low byte 1 for vertical adjacent-zone edges whose x coordinate lies outside the current build block horizontal range.` (evidence: `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86`)
- `[RESOLVED] OQ-4 -- How does temp flag become final edge+4? -> ZoneMap__BuildZoneLevel reads temp+8 and writes the copied flag to forward/reverse final edge+4.` (evidence: `0x00582398`, `0x00582402`, `0x0058245B`)
- `[RESOLVED] OQ-5 -- Do bridge/tube build helpers set it? -> No; FUN_00582D70 bridge/tube temp inserts set flag low byte 0.` (evidence: `0x0058304B`, `0x005830E5`, `0x00583165` decompile)
- `[RESOLVED] OQ-6 -- Do repaired bridge direct edges set it? -> No; AddBridgeZoneEdges writes zero-masked flag dwords to final edge+4.` (evidence: `0x0058535E..0x00585361`, `0x005851B0` decompile)
- `[RESOLVED] OQ-7 -- Does it mean bridge edge? -> No verified bridge-specific writer sets it nonzero; verified bridge/tube writers set zero. Semantic label is build-boundary tiebreak flag, not bridge-edge.` (evidence: `0x00582D70`, `0x005851B0`, `0x005824A0`)
- `[RESOLVED] OQ-8 -- Does standard YR use the reader? -> Yes, standard pathfinding reaches Zone_precheck from AStar_pathfind_search and FUN_0042D170.` (evidence: `0x0042CB58`, `0x0042CCB3`, `0x0042D222`)
- `[RESOLVED] OQ-9 -- Does standard YR use the writers? -> Yes, full map init calls BuildZoneLevel for levels 2,1,0; incremental and bridge validation paths are also present.` (evidence: `0x00567110`, xrefs `0x00584D79`, `0x0056DBD6`)
- `[RESOLVED] OQ-10 -- Does the retry/invalidation path mutate edge+4? -> No; it writes Pathfinder-local packed edge exclusions, not final graph edge flags.` (evidence: `0x0042CCD0`, `0x0042CF80`)
- `[DEFERRED] OQ-11 -- Whole-binary runtime frequency of nonzero flagged edges on stock maps.` (category: needs-runtime-debugger; reason: static analysis proves live writer/reader semantics, not how often stock maps produce nonzero entries; next-step-if-pursued: set a runtime watchpoint/log around final graph `edge+4` writes during stock map load)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Final zone edges carry a low-byte flag; `Zone_precheck` adds `0.001` only when it is nonzero. | `0x0042C540`, `0x0042C59E..0x0042C5AE` | missing | `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs` | Add an optional per-zone-edge metadata field if/when exact hierarchy parity is implemented; feed it into binary-style accumulated zone cost. | Two otherwise equal hierarchy routes differ only by this flag; unflagged route wins by `0.001`. Proposed test name: `zone_precheck_edge_flag_adds_tiny_tiebreak_cost`. | Do not call this a bridge penalty. |
| Nonzero final flag comes from scanline build-boundary temp entries, not bridge-specific helpers. | `0x00582A28..0x00582A3E`, `0x00582C70..0x00582C86`, `0x00582402`, `0x0058245B` | missing | `src/sim/pathfinding/zone_build.rs` future exact hierarchy builder | Preserve the build-boundary condition if binary-like zone graph emission is added. | A block-boundary vertical adjacent-zone connection emits `flag=1`, while the same in-block connection emits `flag=0`. Proposed test name: `zone_build_marks_out_of_block_vertical_edges_with_tiebreak_flag`. | Do not synthesize the flag from bridge records or terrain names. |
| Bridge/tube graph injection and repaired-bridge direct add write zero flags. | `FUN_00582D70`, `MapClass__AddBridgeZoneEdges @ 0x005851B0`, `0x00585361` | current Rust bridge adjacency has no flag, which is closer than assigning bridge penalty but still lacks exact metadata | `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency`, future repair/incremental adjacency update surface | If edge metadata exists, bridge/tube injected edges should carry `flag=0`. | Repairing a bridge adds adjacency but does not add the `0.001` zone-precheck flag cost. Proposed test name: `repaired_bridge_zone_edges_are_unflagged_for_precheck_cost`. | Do not mark all bridge edges as flagged/nonzero. |

### Stale Docs / Follow-up Docs

- `docs/research/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
  - Replace: "`edge.flags_low_byte = 1 if bridge-edge`", "`bridge edge penalty`", or equivalent wording that equates `edge+4 != 0` with bridge edges.
  - With: "`byte(edge+4) != 0` is a zone graph build-boundary tiebreak flag that adds `0.001` in `Zone_precheck`; verified bridge/tube and repaired-bridge edge writers set this byte to zero."
- `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`
  - Replace any wording that maps `0x007E3818` at this site to diagonal or bridge geometry directly.
  - With: "At the `Zone_precheck` site, `0x007E3818` is selected by `byte(edge+4) != 0`; the verified nonzero writer is the zone scanline build-boundary path, not direct diagonal or bridge geometry."
- `docs/research/BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`
  - Replace open item "Bridge-edge flag low-byte semantic ... likely requires identifying a second writer" with: "Resolved by `ZONE_EDGE_RECORD_BYTE_PLUS_4_WRITER_SEMANTICS_GHIDRA_REPORT.md`: nonzero comes from `ZoneMap__FloodFillScanline` block-boundary temp entries; bridge/tube helpers write zero."
- `docs/research/BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md`
  - Keep its correction but strengthen wording from "known nonzero writer" to "verified nonzero writer for this slice."

## Negative Facts / Do Not Do

- Do not implement `edge+4 != 0` as "bridge edge." Bridge/tube temp injection and repaired-bridge direct add both write zero flags. Evidence: `0x00582D70`, `0x005851B0`.
- Do not add a universal `0.001` cost to bridge adjacency. Repaired bridge edges are unflagged and therefore receive `0.0` at the `Zone_precheck` flag branch. Evidence: `0x00585361`, `0x0042C59E..0x0042C5AE`.
- Do not derive this flag from `MovementZone=` or INI data. No INI key writes it; writer is zone graph construction. Evidence: `0x005824A0`, `0x00581F90`.
- Do not make retry exclusions mutate final graph `edge+4`. Retry invalidation writes Pathfinder-local packed edge exclusions. Evidence: `0x0042CCD0`, `0x0042CF80`.
- Do not use centroid Manhattan/g+h zone search and claim this byte is modeled. The byte participates in binary accumulated edge cost, not Rust's current centroid heuristic. Evidence: `0x0042C5BB..0x0042C5D2`; Rust scan of `zone_search.rs::find_zone_corridor`.

## Remaining Uncertainty

- Runtime frequency of nonzero flagged edges on stock maps was not measured. Static evidence proves the writer condition and live reader path.
- Whole-binary direct field-use audit beyond the pathfinding/zone graph functions was not attempted because the requested slice is the `Zone_precheck` cost byte and its writer semantics.

## Sources

- Ghidra decompiled / checked: `0x0042C290`, `0x00581F90`, `0x005824A0`, `0x00582D70`, `0x00584550`, `0x005851B0`, `0x00584E50`, `0x0042CCD0`, `0x0042CF80`, `0x00567110`.
- Ghidra xrefs: `Zone_precheck` from `0x0042CB58`, `0x0042CCB3`, `0x0042D222`; `ZoneMap__BuildZoneLevel` from `0x00584D79`, `0x00581F6A`, `0x0056720D`; `MapClass__AddBridgeZoneEdges` from `0x0056DBD6`.
- Ghidra byte-pattern checks: `8A 50 04 88 54 24 11` only at `0x0042C540`; final writer patterns include `0x00582402`, `0x0058245B`, and bridge direct write `0x00585361`.
- Prior docs referenced: `BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md`, `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`.
- Rust scan only: `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`.
