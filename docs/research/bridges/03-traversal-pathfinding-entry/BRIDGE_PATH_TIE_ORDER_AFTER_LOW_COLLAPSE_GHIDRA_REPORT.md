# Bridge Path Tie Order After Low Collapse -- Ghidra Research Report

**Address(es):** `0x00429A90`, `0x00429830`, `0x0042C290`, `0x00581F90`, `0x00584550`, prior low-collapse evidence at `0x0057BCF0`, `0x0057C2B0`  
**Investigation Mode:** exhaustive-slice downgraded to PARTIAL  
**Claimed Scope:** static binary tie/order rules that decide route choice for a concrete flat low-bridge crossing after the low bridge zone connection is collapsed.  
**Non-Scope:** full `TubeClass` lifecycle, high bridge behavior, visual/audio collapse fallout, and live runtime capture from a named retail map.  
**Confidence:** High for binary tie/order mechanics; Medium for Rust deltas; Low for exact stock-map route incidence.  
**Active in YR:** Yes for all cited pathing and zone functions; the unobserved retail-map route choice remains unproven.

## 0. Investigation Contract

Target question: after a standard YR low bridge collapses and zone connectivity is rebuilt, which equal-cost route wins when two around-the-water alternatives are tied or nearly tied?

Non-goals: do not redo low bridge tube deletion, AStar bridge marker semantics, high bridge layer rules, or full `TubeClass` lifecycle.

Evidence needed to mark COMPLETE: a live or fixture-level trace of a named low-bridge map before and after collapse, including start/goal cells, rebuilt zone ids, final adjacency order, `Zone_precheck` chosen chain, and first post-collapse cell A* path. Static binary order rules alone are not enough to name the exact route for an arbitrary stock map.

Stop conditions: stop after verifying binary tie machinery and Rust-facing handoff; mark partial if exact stock-map route cannot be tied to observed rebuilt graph data.

## 1. Overview

The concrete scenario used for this slice is a flat EW low bridge with a ground unit/infantry path from one shore to the opposite shore. Before collapse, active low bridge records connect the two shore zones, low bridge steps stay on the ground layer, and cell A* can route directly across the low bridge. After full low bridge collapse, prior evidence shows the low bridge zone record is deactivated/omitted from all-active bridge-zone connectivity; a new path must route around the blocked crossing if land alternatives exist.

The exact tie rule after collapse is not "shorter by zone id" and not "sorted neighbor order." It is a composition: rebuilt zone adjacency arrays are emitted in writer order; `Zone_precheck` preserves insertion order on equal accumulated costs; then cell A* expands directions `0..8` with per-direction epsilon and a first-found tolerance. Therefore equal/near-equal post-collapse route choice is deterministic but map-order dependent.

## 2. Key Offsets / Ordering Inputs

| Structure | Offset / address | Meaning | Active in YR |
|---|---:|---|---|
| PathfinderClass | `+0x40/+0x44/+0x48` | per-level chosen-zone markers used by cell A* pruning | Yes (`0x00429EA4`, `0x0042C300..0x0042C8F5`) |
| PathfinderClass | `+0x58/+0x5C/+0x60` | per-level best zone cost arrays | Yes (`0x0042C397`, `0x0042C5DE`) |
| PathfinderClass | `+0x68` | zone precheck min-heap descriptor | Yes (`0x0042C693`, `0x0042C740`) |
| PathfinderClass | `+0x78/+0x84 + level*0x18` | per-search undirected edge exclusions for retries | Yes (`0x0042C635..0x0042C664`) |
| Zone record | `+0x04/+0x10` | final adjacency pointer/count, scanned in stored order | Yes (`0x0042C501..0x0042C540`) |
| Edge entry | `+0x00/+0x04` | neighbor zone id and low-byte flag; flag adds `0.001` | Yes (`0x0042C53E..0x0042C5B4`) |
| A* direction table | `0x0081872C` | per-direction epsilon for compass directions | Yes (`0x00429F96`) |
| A* tolerance | `0x007E37C0` | double `1.009` added to current node cost before closed-list rejection | Yes (`0x00429EEC`) |
| Cell flags | `+0x140 & 0x40000` | temporary cost marker, multiplies destination edge cost by 4.0 | Yes (`0x00429830`) |

## 3. Core Logic

### 3.1 Before collapse: low bridge direct route remains ground-layer pathing

Existing trace `PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md` and fresh `AStar_main_loop @ 0x00429A90` decompile agree that flat low bridge cells do not enter the bridge closed list: the layer gate is bridge layer only when `cell+0x140 & 0x100` and `abs(path_height - cell.Level) >= 2`. For low bridges at the same level as ground, `abs(0 - 0) < 2`, so steps remain ground-layer.

Active in YR: Yes. Evidence: decompile `0x00429A90`; prior trace cites `CheckBridgeTraversal` and low-level movement evidence.

### 3.2 Collapse removes the direct low bridge zone connection, not the tie rules

Prior `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT.md` verified that first low bridge damage recalculates cells but does not rebuild bridge-zone connectivity; the destroyed-anchor/full-collapse transition conditionally invalidates/rebuilds zones. Normal low collapse does not delete `TubeClass` records; it deactivates the active low bridge connectivity record.

Active in YR: Yes. Evidence: prior decompile + assembly at `0x0057C229..0x0057C270`, `0x005721D1..0x005721DC`.

### 3.3 Zone graph writer order is the first post-collapse tie source

Full `ZoneMap__BuildZoneLevel @ 0x00581F90` assigns zones by row-major discovery, inserts scanline temp edges first, then active bridge/tube temp edges, then emits final directed adjacency by temp bucket index `0..255`, insertion order inside each bucket, and low-halfword-directed edge before high-halfword reverse. Incremental rebuild sibling `FUN_00584550 @ 0x00584550` uses the same final bucket traversal and directed append shape after rebuilding the changed aligned blocks.

Fresh evidence: decompile `0x00581F90` shows the bridge-record loop after the scanline pass and final loop `iStack_54 += 0x18` until `< 0x1800`; assembly `0x00582395..0x00582480` reads temp packed pair/flag, appends two directed edges, then advances bucket stride `0x18`. Decompile `0x00584550` and assembly `0x00584B55..0x00584C52` show the same incremental emission shape.

Active in YR: Yes. Standard map init and bridge mutation rebuild paths consume these graphs.

### 3.4 `Zone_precheck` preserves equal-cost insertion order

`Zone_precheck @ 0x0042C290` searches levels `2 -> 1 -> 0`. Candidate cost is accumulated current cost plus `ZoneBaseCost[target_zone_type]`, optional slope cost, and optional `0.001` if `byte(edge+4) != 0`. There is no centroid Manhattan heuristic in this function. A neighbor is inserted only if unvisited or strictly lower cost than the stored best; equal cost does not replace an earlier candidate.

Fresh evidence: decompile `0x0042C290`; assembly `0x0042C6A4..0x0042C6D8` bubbles only while parent cost is greater than new cost; `0x0042C7C5..0x0042C835` sift-down chooses children only on strict lower cost. This means equal-cost post-collapse around-routes inherit adjacency writer order rather than zone-id order.

Active in YR: Yes. `AStar_pathfind_search @ 0x0042C900` calls it on normal foot pathing.

### 3.5 Cell A* adds a second deterministic bias after the zone chain

Within `AStar_main_loop`, neighbors are expanded in direction order `0..8`: N, NE, E, SE, S, SW, W, NW, then direction `8` tube jump. For normal compass steps, the call to `AStar_compute_edge_cost @ 0x00429830` is followed by `edge_cost * pathfinder+4 + DirectionEpsilon[dir]` at `0x00429F96`; the direction-8 tube case bypasses this helper and uses Chebyshev distance to the tube exit.

The closed-list near-tie gate is also not exact equality: when a layer/cell is already closed, it compares existing cost against current-node cost plus double `1.009` (`0x00429EEC`). Existing entries within that additive tolerance block reopening. This biases toward the first route found through the chosen zone corridor.

Active in YR: Yes. Evidence: decompile `0x00429A90`, `0x00429830`; assembly `0x00429EEC`, `0x00429F96`.

## 4. INI Keys

| Key / data | Effect | Active in YR |
|---|---|---|
| `[General] DestroyableBridges=yes` | Makes low bridge collapse reachable in stock YR. | Yes (`rulesmd.ini`) |
| `MovementZone=` | Selects `ZonePassabilityMatrix` row and unit pathing behavior; no direct tie-order key. | Yes (`AStar_pathfind_search` reads TechnoType movement data before `Zone_precheck`) |
| Low bridge overlays `LOBRDG*` / `LOBRDGE*` | Provide visible low bridge cells; movement truth is final cell flags/land/tube/zone data, not overlay names alone. | Yes |

## 5. Integration Points

Before collapse: active low bridge record lets zone graph connect across the low bridge; low bridge A* steps remain ground-layer because there is no high-bridge height delta.

After collapse: bridge-zone rebuild removes the low record connection. If start and goal are still connected through two land alternatives, `Zone_precheck` chooses the route whose equal-cost candidate enters the heap first according to rebuilt adjacency order; cell A* then follows the chosen corridor and applies direction epsilon plus first-found tolerance.

What was not observed: no runtime trace was captured for a named retail map after physically collapsing a low bridge and logging the rebuilt zone ids/edge order. Therefore the exact "north detour vs south detour" answer for a specific stock map remains unproven.

## 6. Current Rust Implementation Status

| Surface | Status vs binary slice |
|---|---|
| `src/sim/pathfinding/zone_build.rs` | Current Rust sorts/dedups adjacency in places per prior reports; binary final adjacency order is append order. |
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | Uses `BinaryHeap<Reverse<(f_cost, g_cost, ZoneId)>>` and centroid/Manhattan-style costs; binary `Zone_precheck` uses accumulated zone-type/flag/slope cost and insertion-order ties. |
| `src/sim/pathfinding/core.rs` | Has direction ordering and A* logic, but parity for `DirectionEpsilon[9]`, `1.009` closed-list tolerance, and transient `0x40000` marker remains incomplete or split across prior tasks. |
| `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs` | Low collapse/zone dirty path likely handles connectivity; exact post-collapse equal-route selection is not guarded by a parity test. |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Low collapse invalidates zone connectivity | verified-from-prior | `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT.md`; `0x0057BCF0`, `0x0057C2B0` | no redo |
| Low bridge direct route ground-layer behavior | verified | `0x00429A90`; `PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md` | no named stock map route |
| Zone graph full-build emission order | verified | `0x00581F90`; asm `0x00582395..0x00582480` | none for ordering |
| Zone graph incremental emission order | verified | `0x00584550`; asm `0x00584B55..0x00584C52` | repeated mutation lifecycle not exhausted |
| `Zone_precheck` equal-cost tie behavior | verified | `0x0042C290`; asm `0x0042C6A4..0x0042C6D8`, `0x0042C7C5..0x0042C835` | none for static tie behavior |
| Cell A* direction/near-tie behavior | verified for relevant gates | `0x00429A90`; asm `0x00429EEC`, `0x00429F96` | exact full path on map remains runtime-dependent |
| Concrete stock-map route after collapse | deferred | no runtime/map-corpus log | capture start/goal, zone ids, chosen chain |
| Rust parity status | touched-not-exhausted | codegraph + prior source scans | no tests run, no code changed |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is this code active in YR? -> Yes for normal foot pathing and bridge zone rebuilds.` (evidence: `0x0042C900 -> 0x0042C290`, `0x00581F90`, `0x00584550`; Active in YR: Yes)
- `[RESOLVED] OQ-2 -- Does low bridge direct crossing use bridge-layer A*? -> No for flat low bridges; height delta is below the bridge-layer threshold.` (evidence: `0x00429A90`; Active in YR: Yes)
- `[RESOLVED] OQ-3 -- Does full low collapse sever zone connectivity? -> Yes via prior low-collapse report; first damage does not, destroyed transition does.` (evidence: `0x0057C229..0x0057C270`; Active in YR: Yes)
- `[RESOLVED] OQ-4 -- Are final zone adjacency arrays sorted? -> No; final order is temp bucket/insertion order and directed append order.` (evidence: `0x00582395..0x00582480`; Active in YR: Yes)
- `[RESOLVED] OQ-5 -- Does incremental rebuild preserve the same final emission shape? -> Yes for the scoped final bucket/direct append loop.` (evidence: `0x00584B55..0x00584C52`; Active in YR: Yes)
- `[RESOLVED] OQ-6 -- Does `Zone_precheck` use zone-id tie ordering? -> No; equal costs preserve insertion/heap order.` (evidence: `0x0042C6A4..0x0042C6D8`; Active in YR: Yes)
- `[RESOLVED] OQ-7 -- Does `Zone_precheck` use centroid Manhattan heuristic? -> No; candidate cost is accumulated target zone type, slope, and edge-flag penalty.` (evidence: `0x0042C55C..0x0042C5D2`; Active in YR: Yes)
- `[RESOLVED] OQ-8 -- Does cell A* have an additional direction bias? -> Yes; compass directions add `DirectionEpsilon[dir]`, and direction 8 uses tube Chebyshev cost instead.` (evidence: `0x00429F96`, `0x00429FA3..0x00429FE6`; Active in YR: Yes)
- `[RESOLVED] OQ-9 -- Does near-equal cell A* reopening use exact equality? -> No; existing cost within `current + 1.009` blocks reopening.` (evidence: `0x00429EEC`; Active in YR: Yes)
- `[DEFERRED] OQ-10 -- Which named stock map detour wins after a live low bridge collapse?` (category: needs-runtime-debugger; reason: requires actual rebuilt zone ids/edge arrays and start/goal cells; next-step-if-pursued: capture one retail low-bridge map path before/after collapse with zone chain logs)
- `[DEFERRED] OQ-11 -- How often do equal-cost post-collapse ties occur in stock maps?` (category: needs-runtime-debugger; reason: static evidence proves behavior but not incidence; next-step-if-pursued: map-corpus instrumentation over low bridge maps)
- `[DEFERRED] OQ-12 -- Full repeated incremental mutation duplicate lifecycle.` (category: bounded-cost-too-high; reason: final emitter verified, but many collapse/repair interleavings need a separate lifecycle trace; next-step-if-pursued: trace `FUN_00584550` after collapse+repair cycles)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| After full low collapse, direct low bridge connectivity is gone, and equal around-route choice is inherited from rebuilt zone adjacency emission order. | `LOW_BRIDGE_COLLAPSE_TUBE_ZONES...`; `0x00581F90`; `0x00584550`; `0x0042C290` | mismatch/unchecked: Rust sorts/dedups zone adjacency and uses zone-id tuple heap ties. | `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/world/bridge_orchestrator.rs` | Preserve binary adjacency order and insertion-order ties for parity zone precheck after bridge-zone rebuild. | Collapse a low bridge in a symmetric two-detour fixture; the first post-collapse path uses the detour whose zone edge appears first in binary-style emission order. Proposed test name: `low_bridge_collapse_equal_detour_uses_zone_emission_order`. | Do not sort final adjacency lists or let `ZoneId` decide equal-cost ties. |
| Cell A* applies direction order `0..8`, per-direction epsilon, and `1.009` first-found closed-list tolerance after the zone chain is chosen. | `0x00429A90`; `0x00429EEC`; `0x00429F96`; `0x00429830` | partial/unchecked: current `core.rs` needs focused comparison for epsilon/tolerance/transient marker interaction. | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/core_tests.rs` | Add deterministic cost tie fixtures that distinguish N/NE/E... expansion and first-found tolerance from exact equality. | In a same-zone post-collapse fixture with two equal cell routes, the route with lower direction epsilon/earlier discovered closed-list entry wins. Proposed test name: `astar_equal_cost_after_low_bridge_collapse_uses_direction_epsilon_and_first_found_tolerance`. | Do not model this as arbitrary stable sort or exact-cost reopening. |
| Low bridge collapse does not delete low bridge tube facts; route failure/alternative choice comes from inactive bridge-zone connectivity and cell passability, not missing tube identity. | `LOW_BRIDGE_COLLAPSE_TUBE_ZONES...`; `0x00484AB0`; `0x00484F20` prior | mostly present but guard needed in route fixture. | `src/map/resolved_terrain.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/pathfinding/zone_map_tests.rs` | Keep tube facts stable while removing active low record connection on collapse. | Before collapse, path crosses low bridge; after collapse, tube fact remains but zone precheck/path chooses land detour or fails if none exists. Proposed test name: `low_bridge_collapse_preserves_tubes_but_repaths_around_inactive_zone_record`. | Do not clear `tube_index` as the collapse mechanism. |

### Negative Facts / Do Not Do

- Do not claim a universal north/south/east/west detour order after low bridge collapse; the binary tie is graph-emission/order dependent, not a hardcoded compass preference. Active in YR: Yes; evidence `0x00582395..0x00582480`, `0x0042C6A4..0x0042C6D8`.
- Do not sort zone adjacency lists by `ZoneId` for parity. Active in YR: Yes; final writer appends in temp bucket/insertion order.
- Do not use centroid Manhattan or `g+h` as the binary `Zone_precheck` cost. Active in YR: Yes; evidence `0x0042C55C..0x0042C5D2`.
- Do not treat low collapse as deleting `TubeClass` records or clearing `CellClass+0x116`. Active in YR: Yes; prior low-collapse report found normal collapse invalidates zones instead.
- Do not apply high-bridge diagonal bridge-layer cost logic to flat low bridge direct steps; low bridge crossing remains ground-layer when level delta is zero. Active in YR: Yes; evidence `0x00429A90`.

### Remaining Uncertainty

- Exact route chosen on a named standard YR map after a live low bridge collapse remains unobserved. Static evidence resolves the algorithm, not the concrete stock-map graph instance.
- Runtime frequency of exact equal-cost detours after low collapse is unknown.
- Repeated collapse/repair incremental duplicate lifecycle was not exhausted.

### Stale Docs / Follow-up Docs

- Additive wording for `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT.md` OQ13: `Post-collapse route tie order is governed by zone graph emission order (`ZONE_GRAPH_ADJACENCY_EMISSION_ORDER_GHIDRA_REPORT.md`), `Zone_precheck` insertion-order ties, and cell A* direction epsilon/1.009 first-found tolerance. A named stock-map route still requires runtime or map-corpus tracing.`

## Sources

- Ghidra decompiled: `0x00429A90`, `0x00429830`, `0x0042C290`, `0x00581F90`, `0x00584550`.
- Ghidra assembly contexts: `0x0042C6A4`, `0x0042C7C5`, `0x00582395`, `0x00582479`, `0x00584B55`, `0x00584C3C`, `0x00429EEC`, `0x00429F96`, `0x0042A030`.
- Prior reports: `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT.md`, `ZONE_GRAPH_ADJACENCY_EMISSION_ORDER_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`, `PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md`, `DIRECTION8_TUBE_STEP_REFERENCE_TRACE.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: Codegraph plus `rg` over `src/sim/pathfinding`, `src/sim/bridge_state`, `src/sim/world`.

