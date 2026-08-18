# Zone Precheck Slope Contribution Costs - Ghidra Research Report

**Address(es):** `0x0042C290` (`Zone_precheck`), `0x00585F40` (`Zone_Estimate_Slope_Cost`), callers `0x0042C900` and `0x0042D170`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** consumer-side `Zone_precheck` cost formula for target-zone base costs, slope-cost gate/scaling/rounding, edge low-byte tiebreak addend, candidate ordering, and Rust-facing representation in `src/sim/pathfinding/zone_hierarchy.rs` / `zone_search.rs`.
**Non-Scope:** full zone graph writer lifecycle, full `Foot+0x21C` slope-context construction, full cell A* smoothing/reroute slope checks, and runtime route-frequency measurement on stock maps.
**Confidence:** High for the scoped binary cost formula and current Rust delta; Medium for runtime incidence because no live route capture was performed.
**Active in YR:** Yes. `AStar_pathfind_search @ 0x0042C900` calls `Zone_precheck` at `0x0042CB58` and `0x0042CCB3`; `PathfinderClass__EstimateZoneCost @ 0x0042D170` calls it at `0x0042D222`.

## Summary

`Zone_precheck` does not use centroid distance or a destination heuristic. Each accepted graph edge is costed as:

```text
candidate =
    current_node_cost
  + ZoneBaseCost[neighbor_reduced_zone_type]
  + ftol(Zone_Estimate_Slope_Cost(Foot+0x21C, level, current_zone, neighbor_zone) * slope_factor)
  + (edge_low_byte != 0 ? 0.001 : 0.0)
```

The base table at `0x007E3794` is byte-verified as eight `float` values:

```text
zone type: 0  1  2  3  4  5  6  7
cost:      1  0  0  1  1  0  1  1
```

The edge flag addend at `0x007E3818` is exactly `0.001` as a double. The slope gate threshold at `0x007E3810` is exactly `1e-5` as a double. Equal-cost candidates do not replace earlier candidates, and heap movement uses strict lower-cost comparisons, so insertion/adjacency order remains the tie source.

## Verified Binary Findings

### Base cost table

| Item | Value | Evidence | Active in YR |
|---|---:|---|---|
| `ZoneBaseCost[0]` | `1.0f` | `read_memory 0x007E3794 len 32`; load at `0x0042C5BB` | Yes |
| `ZoneBaseCost[1]` | `0.0f` | same dump and load | Yes |
| `ZoneBaseCost[2]` | `0.0f` | same dump and load | Yes |
| `ZoneBaseCost[3]` | `1.0f` | same dump and load | Yes |
| `ZoneBaseCost[4]` | `1.0f` | same dump and load | Yes |
| `ZoneBaseCost[5]` | `0.0f` | same dump and load | Yes |
| `ZoneBaseCost[6]` | `1.0f` | same dump and load | Yes |
| `ZoneBaseCost[7]` | `1.0f` | same dump and load | Yes |

`Zone_precheck` loads the neighbor zone type from the neighbor zone record at `+0x1C` (`0x0042C55C`) and then loads the base cost with `FLD float ptr [EDX*4 + 0x7e3794]` (`0x0042C5BB`). That same reduced zone type is also the `ZonePassabilityMatrix` column.

The table is not a movement-zone row, not raw `LandType`, and not speed-type cost. It is an eight-entry reduced-zone-type edge cost table.

### Edge flag tiebreak penalty

The edge record low byte at `edge+4` is read at `0x0042C540`. If the byte is nonzero, `Zone_precheck` loads the double at `0x007E3818` (`0x0042C5A6`); otherwise it loads the zero double at `0x007E2800` (`0x0042C5AE`). `read_memory 0x007E3818 len 8` decodes to exactly `0.001`.

This value is added as a float contribution after base cost, parent cost, and integer slope contribution:

- `0x0042C5BB`: load `ZoneBaseCost[type]`
- `0x0042C5C2`: add parent node cost from node `+8`
- `0x0042C5C9`: add integer slope contribution with `FIADD`
- `0x0042C5D0`: add edge flag addend from x87 stack
- `0x0042C5D2`: store candidate as `float`

The report keeps the label "edge low-byte tiebreak flag." It is not proven here to mean "bridge edge."

### Slope contribution gate and rounding

If the mover pointer argument is null, slope is disabled. If non-null, `Zone_precheck` calls `FootClass__Get_Slope_Speed_Factor @ 0x004DC760` (`0x0042C2BA`), reads the slope context pointer from `Foot+0x21C` (`0x0042C2BF`), and enables the helper only when the returned factor is strictly greater than the double threshold at `0x007E3810`. `read_memory 0x007E3810 len 8` decodes to exactly `1e-5`.

When enabled, the call sequence is:

- push `neighbor_zone`, `current_zone`, `level`, and `Foot+0x21C`
- call `Zone_Estimate_Slope_Cost @ 0x00585F40` at `0x0042C580`
- `FILD` the helper integer result at `0x0042C589`
- multiply by the stored mover factor at `0x0042C58D`
- call `Math__ftol @ 0x007C5F00` at `0x0042C591`
- add the returned integer via `FIADD` at `0x0042C5C9`

`Zone_Estimate_Slope_Cost` level behavior matches prior report `ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md`: level `0` returns zero; level `1` uses only the neighbor representative and `ctx+0x57E4`; level `2` samples the `ctx+0x59F0` 130-wide grid with direction-specific corner mins and an arithmetic half-sum.

### Candidate ordering and strict replacement

The existing-best check uses a strict lower-cost rule. If a neighbor was already visited this epoch, `0x0042C5DE..0x0042C5EA` compares the existing float best cost against the new candidate and skips on equality. Heap insertion likewise bubbles only when the parent heap node cost is strictly greater than the new cost (`0x0042C6B3..0x0042C6BF`). Equal-cost candidates preserve earlier adjacency/insertion order.

This matters with the verified zero-cost zone types `1`, `2`, and `5`: many paths can tie exactly except for insertion order or the `0.001` edge flag.

### Passability gate order

The candidate cost is computed before the later parent, passability, and exclusion gates. Acceptance then requires:

1. unvisited or strictly lower new cost (`0x0042C5CD..0x0042C5EA`);
2. level `2`, or parent marked in the next-coarser chosen path, or neighbor zone type `1` (`0x0042C5F0..0x0042C604`);
3. `ZonePassabilityMatrix[movementZone][neighbor_zone_type] == 1` (`0x0042C60A..0x0042C612`);
4. no matching sorted undirected edge exclusion in the current level vector (`0x0042C620..0x0042C664`).

The 13x8 matrix at `0x0082A594` is not a cost source here; it is a legal/illegal gate, and only value `1` passes.

## Active YR Status

This cost path is live in standard Yuri's Revenge foot pathfinding:

- `FootClass__Run_AStar @ 0x004CBBA0` reaches `AStar_pathfind_search @ 0x0042C900`.
- `AStar_pathfind_search` calls `Zone_precheck` before cell A* at `0x0042CB58` and after hierarchy-assisted retry at `0x0042CCB3`.
- `PathfinderClass__EstimateZoneCost @ 0x0042D170` also calls `Zone_precheck` at `0x0042D222` and returns `0x7fffffff` on precheck failure.
- No TS/fog/special-mode gate was found inside `Zone_precheck`; ordinary enablement is controlled by the caller's hierarchy flag and per-mover pathing state.

Conditional details:

- Slope contribution is active only for a non-null mover whose factor is greater than `1e-5`.
- The linked-object branch in `FootClass__Get_Slope_Speed_Factor` can force factor `1.0`; otherwise it returns `Foot+0x530`.
- Exact runtime incidence of the slope term changing a route on a stock map requires route logging and is not claimed here.

## Rust Delta

| Rust surface | Current behavior | Delta vs gamemd |
|---|---|---|
| `src/sim/pathfinding/zone_hierarchy.rs::ZONE_BASE_COSTS` | `[1000, 0, 0, 1000, 1000, 0, 1000, 1000]` integer-scaled table | Preserves base table ordering under a `1000x` scale, but not the binary float representation. |
| `zone_hierarchy.rs::search_precheck_level` | `edge_flag_cost = i32::from(edge.flag != 0)` | Relative to the `1000x` base table, this matches `0.001` ordering. It should be documented as a scale encoding, not a literal binary value. |
| `zone_hierarchy.rs::search_precheck_level` | no mover coefficient or slope context input | Missing `ftol(helper * factor)` contribution for levels 1/2 when factor `> 1e-5`. |
| `zone_search.rs::find_zone_corridor` | Manhattan center distance between zone centers | Mismatch for any path still using this legacy corridor: gamemd uses reduced-zone-type base cost + slope + edge flag, no heuristic. |
| `zone_search.rs::find_zone_corridor` | stable insertion-order heap after recent fixes | Directionally compatible for equal-cost tie order, but still uses the wrong cost source. |

The best Rust representation without approximation is to keep a binary-facing float/Dijkstra precheck model for parity code, or to keep an explicit fixed scale where `1.0 == 1000` and `0.001 == 1`. The current `ZONE_BASE_COSTS` plus `edge_flag_cost` already uses that scale consistently for the base/flag parts. That scale cannot represent arbitrary future slope-factor products exactly unless the slope path is also specified in the same fixed-point domain with a proof against gamemd float/`ftol` behavior. Until that proof exists, the exact slope contribution remains missing, not merely scaled.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Base edge cost is `ZoneBaseCost[neighbor reduced zone type]` with float table `{1,0,0,1,1,0,1,1}`. | `read_memory 0x007E3794`; `0x0042C55C`; `0x0042C5BB` | `zone_hierarchy.rs` uses scaled integers; `zone_search.rs` still uses Manhattan centers. | `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs` | Use reduced zone type as the edge cost source for parity precheck; legacy centroid corridor must not be called binary-equivalent. | A graph where a type-1/type-2/type-5 corridor is longer geometrically but lower binary cost than a type-0 corridor chooses the zero-cost corridor. | Do not use centroid distance, raw LandType, or SpeedType as the precheck edge cost. |
| Nonzero edge low byte adds exactly `0.001`; zero adds `0.0`. | `read_memory 0x007E3818`; `0x0042C540`; `0x0042C5A6..0x0042C5D2` | integer `+1` in `zone_hierarchy.rs` is a valid `1000x` ordering encoding for base/flag only. | `src/sim/pathfinding/zone_hierarchy.rs` edge-cost comments/tests | Preserve the flag as a tiebreak addend smaller than a base-cost `1.0`; if fixed scale is retained, document `1.0 == 1000`, `0.001 == 1`. | Two otherwise equal candidate chains differ only by one flagged edge; the unflagged chain wins. | Do not treat the flag as `1.0` in binary-float terms or label it a bridge penalty. |
| Slope term is gated by non-null mover and factor `> 1e-5`, then added as `ftol(helper_result * factor)`. | `0x0042C2BA..0x0042C2BF`; `read_memory 0x007E3810`; `0x0042C580..0x0042C5C9`; `0x004DC760`; `0x00585F40` | missing in Rust precheck; current flat tests are only exact when level 0/no mover/factor <= threshold/no slope context is intended. | future `zone_hierarchy.rs` slope-cost input surface; mover/pathfinder coefficient state | Add slope contribution only under the exact binary gate and with `ftol` truncation after multiplication. | Level-1 synthetic graph where two routes tie on base+flag but neighbor representatives have different `ctx+0x57E4` values; factor `1.0` selects the lower slope route, factor `0.0` preserves insertion-order tie. | Do not reuse runtime `SlopeClimb`/`SlopeDescend`, raw cell height delta, or A* cliff multiplier as this term. |
| Equal-cost replacement and heap movement are strict; equality preserves earlier candidate. | `0x0042C5DE..0x0042C5EA`; `0x0042C6B3..0x0042C6BF`; insertion-order report | mostly fixed in current hierarchy scaffold; legacy corridor must stay stable if retained. | `zone_hierarchy.rs::PrecheckQueueEntry`, `zone_search.rs::ZoneQueueEntry` | Ensure no `ZoneId` tie key enters parity precheck ordering. | Start zone adjacency ordered `[high_id, low_id]`, both equal cost; chosen chain starts with `high_id`. | Do not use `BinaryHeap<(cost, ZoneId)>` or sorted adjacency as a parity substitute. |

## Acceptance Tests

1. `zone_precheck_uses_verified_base_cost_table`
   - Build a synthetic three-level or single-level precheck graph with two candidate chains: a geometrically shorter type-0 path and a geometrically longer type-1/type-2/type-5 path. Expected binary precheck picks the lower accumulated base-cost path, not the shorter centroid path.

2. `zone_precheck_edge_flag_is_point_zero_zero_one_tiebreak`
   - Two chains have identical reduced zone types and no slope contribution. One chain includes a nonzero edge low byte; the other does not. Expected binary precheck chooses the unflagged chain.

3. `zone_precheck_slope_gated_by_factor_threshold`
   - Use a synthetic slope context where one route has nonzero helper cost. With factor exactly `0.0` or `1e-5`, no slope contribution is added. With factor greater than `1e-5`, `ftol(helper * factor)` changes the selected path.

4. `zone_precheck_equal_cost_preserves_adjacency_order`
   - Start adjacency order is `[zone 9, zone 2]`, both with equal candidate cost. Expected chosen chain starts with zone `9`; no zone-id sort participates.

5. `legacy_zone_corridor_not_claimed_binary_equivalent`
   - Guard or document any path using `zone_search.rs::find_zone_corridor` so a test cannot assert binary parity while it still uses centroid Manhattan distance.

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_precheck` base cost table load | verified | `0x0042C55C`, `0x0042C5BB`, `read_memory 0x007E3794` | none |
| Edge low-byte addend | verified | `0x0042C540`, `0x0042C5A6`, `read_memory 0x007E3818` | writer-side meaning of the flag outside scope |
| Slope gate threshold and call consumption | verified | `0x0042C2BA`, `0x0042C2BF`, `0x0042C580`, `0x0042C589`, `read_memory 0x007E3810` | full slope-context writer lifecycle deferred |
| `Zone_Estimate_Slope_Cost` helper formula | touched and anchored | decompile `0x00585F40`; prior parity report | no new writer-side investigation |
| `FootClass__Get_Slope_Speed_Factor` | verified for return source | decompile `0x004DC760` | linked-object flag semantic label outside scope |
| Candidate strict replacement and heap tie behavior | verified | `0x0042C5DE..0x0042C5EA`, `0x0042C6B3..0x0042C6BF` | full graph writer adjacency order outside scope |
| Active caller path | verified | decompile `0x0042C900`, `0x0042D170`; xrefs to `0x0042C290` | runtime frequency not measured |
| Current Rust cost representation | verified for scan | `zone_hierarchy.rs:28..29`, `zone_hierarchy.rs:436..442`, `zone_search.rs:676..681` | no code changes per swarm rule |

## Open Questions - Final State

- `[RESOLVED] OQ-1 - Is this cost path active in standard YR? -> Yes through `AStar_pathfind_search` initial and retry calls, and through `PathfinderClass__EstimateZoneCost`.` (evidence: `0x0042CB58`, `0x0042CCB3`, `0x0042D222`)
- `[RESOLVED] OQ-2 - What are the exact base cost values? -> `{1,0,0,1,1,0,1,1}` as eight 32-bit floats at `0x007E3794`.` (evidence: `read_memory 0x007E3794 len 32`; `0x0042C5BB`)
- `[RESOLVED] OQ-3 - What indexes the base table? -> Neighbor zone record `+0x1C`, the reduced zone type.` (evidence: `0x0042C55C`; `CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-4 - What is the edge flag penalty? -> Nonzero edge low byte adds exactly `0.001`; zero adds `0.0`.` (evidence: `0x0042C540`, `0x0042C5A6`, `0x0042C5AE`, `read_memory 0x007E3818`)
- `[RESOLVED] OQ-5 - What gates slope contribution? -> non-null mover plus `FootClass__Get_Slope_Speed_Factor > 1e-5`.` (evidence: `0x0042C2BA..0x0042C2BF`, `read_memory 0x007E3810`)
- `[RESOLVED] OQ-6 - How is slope rounded? -> helper integer is converted to x87 float, multiplied by factor, then converted by `Math__ftol` before integer add.` (evidence: `0x0042C580..0x0042C591`, `0x0042C5C9`)
- `[RESOLVED] OQ-7 - Does `Zone_precheck` use a destination heuristic? -> No; heap key is accumulated candidate cost only.` (evidence: decompile `0x0042C290`; `0x0042C5BB..0x0042C5D2`)
- `[RESOLVED] OQ-8 - Do equal costs replace earlier candidates? -> No; equality skips replacement and heap bubble uses strict greater-than.` (evidence: `0x0042C5DE..0x0042C5EA`, `0x0042C6B3..0x0042C6BF`)
- `[RESOLVED] OQ-9 - Is the matrix a cost table? -> No, `ZonePassabilityMatrix` is a passability gate and only `1` passes.` (evidence: `0x0042C60A..0x0042C612`; `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-10 - Does current `zone_hierarchy.rs` encode base/flag costs exactly enough? -> It encodes base/flag ordering as `1000x`; this is acceptable only as a documented fixed scale for those terms, not as the literal binary float representation or complete slope parity.` (evidence: `zone_hierarchy.rs:28..29`, `zone_hierarchy.rs:436..442`)
- `[RESOLVED] OQ-11 - Does current `zone_search.rs::find_zone_corridor` match this cost model? -> No; it uses Manhattan center distance.` (evidence: `zone_search.rs:676..681`)
- `[DEFERRED] OQ-12 - Who writes every `Foot+0x21C` slope-context value?` (category: out-of-scope; reason: this slot verifies consumer cost constants and contribution, not context construction; next-step-if-pursued: trace slope-context allocation/fill writers)
- `[DEFERRED] OQ-13 - How often does slope contribution change stock-map routes?` (category: needs-runtime-debugger; reason: requires live route capture/logging; next-step-if-pursued: instrument `0x0042C580` helper return and selected zone chains on hilly maps)
- `[DEFERRED] OQ-14 - Exact graph writer adjacency order for every hierarchy level.` (category: out-of-scope; reason: tie rule is verified but writer order belongs to graph-builder reports; next-step-if-pursued: audit full and incremental zone graph edge emission)

## Remaining Uncertainty

- Full construction and lifetime of the `Foot+0x21C` slope context remains deferred.
- Runtime incidence of slope changing common YR routes is not measured.
- The semantic label for the linked-object flag that makes `FootClass__Get_Slope_Speed_Factor` return `1.0` is not needed for this cost contract and remains outside scope.
- Existing docs already cover most of the cost formula; this report specifically upgrades the base/flag constants and Rust representation guidance with fresh live Ghidra evidence.

## Sources

- Ghidra decompiled/read-only: `0x0042C290`, `0x0042C900`, `0x0042D170`, `0x00585F40`, `0x004DC760`.
- Ghidra assembly contexts: `0x0042C2BA`, `0x0042C580`, `0x0042C589`, `0x0042C5A6`, `0x0042C5AE`, `0x0042C5BB`, `0x0042C5C9`, `0x0042C5D2`, `0x0042C5EA`, `0x0042C6B3`.
- Ghidra memory reads: `0x007E3794` length 64, `0x007E3810` length 16, `0x007E3818` length 8, `0x0082A594` length 64.
- Prior reports: `ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_HIERARCHY_FULL_BUILD_CONTRACT_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`.
