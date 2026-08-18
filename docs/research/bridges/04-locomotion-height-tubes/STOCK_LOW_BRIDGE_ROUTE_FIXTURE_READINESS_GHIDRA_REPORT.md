# Stock Low Bridge Route Fixture Readiness - Ghidra Research Report

Target question: Is existing evidence enough to use a stock low-bridge post-collapse route as an acceptance test?
Non-goals: Do not solve all stock routes, implement route logging, or modify Rust/INI/repo docs.
Evidence needed to mark COMPLETE: Identify the stock fixture status, separate static proof from runtime-only proof, and name the exact gamemd hook/logging strategy needed before route assertions are allowed.
Stop conditions: Stop after fixture readiness is resolved; route/zone IDs and final route direction remain deferred if no live gamemd run is logged.

**Address(es):** `0x0042C900`, `0x0042C290`, `0x00429A90`, `0x0057BCF0`, `0x0057C2B0`, `0x00484AB0`, `0x00484F20`, `0x0056C510`
**Investigation Mode:** coverage-map extension
**Claimed Scope:** Carville low-bridge route fixture readiness only: map/waypoint/collapse-trigger evidence, static binary route constraints, and required runtime logger hooks.
**Non-Scope:** all stock low-bridge routes, full three-level zone-precheck implementation, repeated collapse/repair lifecycle, or Rust code changes.
**Confidence:** High that the fixture is valid but not route-assertion-ready; Medium for exact hook plan until a debugger session validates path-object decoding.
**Active in YR:** Conditional. All cited binary paths are live in standard YR; the fixture exercises them only when a test harness loads Carville, fully collapses the selected low bridge, and issues the path query.
**Result:** COMPLETE for readiness audit; NOT READY for an exact stock-route acceptance assertion.

## 1. Overview

Existing evidence is enough to keep the Carville scenario as the preferred stock fixture, but not enough to assert the exact gamemd detour, zone IDs, or final cell path after collapse.

The usable fixture seed remains: loose `Carville.mmx`, waypoint `1=(79,50)` to waypoint `0=(49,87)`, after fully collapsing the low bridge controlled by CABHUT `(57,49)` with fallback starter `(60,52):0x11380`. The missing evidence is runtime-only: rebuilt zone IDs/edge order, `Zone_precheck` chosen chains, retry exclusions, and final post-smoothing path cells from gamemd.exe.

Active in YR: Conditional. Evidence: retail map data; fallback scan; live pathing/collapse functions listed below.

## 2. Fixture Facts

| Fact | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `Carville.mmx` exists as a retail loose map. | `C:/Users/enok/Documents/Command and Conquer Red Alert II/Carville.mmx` | High | Yes |
| Carville uses `Theater=SNOW`, `Size=0,0,80,86`, `LocalSize=2,6,76,68`. | `rg -a` over `Carville.mmx` lines `599..601` | High | Yes |
| Carville has `DestroyableBridges=yes`. | `Carville.mmx` `[SpecialFlags]`, line `22`; defaults also in `rules.ini` and `rulesmd.ini` | High | Yes |
| Waypoint `1` decodes to `(79,50)` and waypoint `0` to `(49,87)`. | `Carville.mmx` lines `604..605`: `0=87049`, `1=50079` | High | Conditional: map data is live; choosing this route is harness-defined |
| CABHUT `(57,49)` is a low no-overlay fallback dispatch with starter `(60,52):0x11380`. | `%TEMP%/bridge_hut_stock_scan_output_named.txt` line `346` and duplicate line `1439` | High | Conditional: live when this hut is destroyed or equivalent bridge dispatch runs |
| `CABHUT` is a bridge repair hut in stock rules. | `ini/rules.ini:9448/9460`, `ini/rulesmd.ini:16336/16348` | High | Yes |

## 3. Binary Constraints Rechecked

### 3.1 Path search entry and retry integration

`AStar_pathfind_search @ 0x0042C900` is the hook point for the fixture route query. It:

- resolves start/destination cells;
- computes start/destination zone IDs through `MapClass__GetZoneID`;
- resolves bridge-aware path coordinates through `MapClass__ResolvePathCoord_BridgeAware`;
- calls `Zone_precheck @ 0x0042C290` when hierarchical search is enabled;
- calls `AStar_main_loop @ 0x00429A90`;
- on hierarchical A* failure, calls `PathfinderClass__UpdateHierarchicalEdges`, resets, and reruns `Zone_precheck` until attempt limit.

Active in YR: Yes. Evidence: fresh decompile `0x0042C900`.

### 3.2 Zone precheck output is path-chain data, not a guessed detour

`Zone_precheck @ 0x0042C290` runs levels `2 -> 1 -> 0`, reads start/destination zone IDs from `DAT_0087F858`, filters candidates through `g_PassabilityMatrix`, applies optional slope cost, adds `0.001` when edge byte `+4` is nonzero, and writes chosen zone-chain state into Pathfinder fields including per-level chain storage at `this+0xbc + level*1000` and length at `this+0xc74 + level*4`.

Equal-cost heap motion is strict lower-cost only; equality does not replace/bubble ahead. Therefore Carville's exact detour cannot be inferred without the rebuilt adjacency order from that runtime graph.

Active in YR: Yes. Evidence: fresh decompile `0x0042C290`.

### 3.3 Final low collapse, not first damage, is required

`MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0` and `MapClass__DestroyBridgeWalker_EW_Low @ 0x0057C2B0` stage low bridge damage. First healthy-main hit writes damaged overlays (`0x50` for NS, `0x59` for EW). Final damaged-main hit writes destroyed anchors (`0x64` for NS, `0x65` for EW), recalculates cells, and calls `MapClass__UpdateBridgeZonesHelper`.

Active in YR: Yes. Evidence: fresh decompile `0x0057BCF0`, `0x0057C2B0`.

### 3.4 Low bridge identity survives as tube/land data

`CellClass__IsLowBridgeCell @ 0x00484AB0` requires a valid `CellClass+0x116` tube index and `CellClass+0xEC == 10`. `CellClass__GetTubeAtCell @ 0x00484F20` returns `g_TubeArray[index]` when the same index is in range.

Active in YR: Yes. Evidence: fresh decompile `0x00484AB0`, `0x00484F20`.

### 3.5 Cell route still needs final path logging

`AStar_main_loop @ 0x00429A90` expands normal directions `0..7` plus direction `8` tube jump, uses separate ground/bridge closed/cost arrays, adds direction epsilon for normal compass edges, and returns the path after `AStar_reconstruct_path`, `Path_smooth_corners`, and `Path_optimize_straight_segments` in the caller.

Active in YR: Yes. Evidence: fresh decompile `0x00429A90` and caller `0x0042C900`.

## 4. Readiness Decision

The stock fixture is **static-ready** but **not acceptance-ready**.

Safe now:

- use Carville `(79,50) -> (49,87)` and CABHUT `(57,49)` / starter `(60,52):0x11380` as a named runtime trace scenario;
- write synthetic Rust tests for verified mechanics such as adjacency insertion-order ties, undirected edge exclusions, and low final-collapse zone refresh;
- write a stock-map guard that asserts symbolic behavior only: before final collapse a route exists, after final collapse the collapsed low span is not used if an alternative exists.

Not safe yet:

- assert the exact north/south/east/west detour;
- assert exact rebuilt zone IDs;
- assert exact `Zone_precheck` chain;
- assert exact post-smoothing path cells.

## 5. Required Hook / Logging Strategy

Proposed trace name: `gamemd_carville_low_bridge_post_collapse_route_trace`.

Minimum hooks:

| Hook | Log | Why |
|---|---|---|
| `AStar_pathfind_search @ 0x0042C900` entry | start cell, goal cell, movement-zone arg, hierarchical flag, unit type/movement row if accessible | Confirms the fixture query is exactly waypoint `1 -> 0` after collapse. |
| `MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0` / `EW @ 0x0057C2B0` return | input cell, overlay before/after, return flag, whether `UpdateBridgeZonesHelper` ran | Confirms the test hit final collapse, not first damage. |
| `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` after collapse | changed bridge record active state if accessible; affected start/goal level zone IDs from `DAT_0087F858` | Confirms graph was rebuilt for the post-collapse query. |
| `Zone_precheck @ 0x0042C290` entry/return | level `2/1/0` start zone, goal zone, return bool, chain length `this+0xc74+level*4`, chain cells at `this+0xbc+level*1000`, retry exclusion count/edges | Captures the actual zone chain and whether retries changed it. |
| `AStar_pathfind_search @ 0x0042C900` after `AStar_main_loop` and after smoothing/optimization | raw path pointer/length if known, final path cells once path-object decoding is confirmed | Captures the player-visible route acceptance target. |

If path-object decoding is not available in the first pass, log pre-smoothing A* node parent chain around `AStar_reconstruct_path` and treat final path cells as a follow-up trace. Do not substitute Rust's path for gamemd's final path.

## 6. Current Rust Implementation Status

| Surface | Status | Fixture implication |
|---|---|---|
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | Still documents centroid/Manhattan edge cost approximation, though equal-cost queue ties now use sequence order rather than `ZoneId`. | Good synthetic tie tests exist, but not a full gamemd `Zone_precheck`. |
| `src/sim/pathfinding/zone_build.rs` | `neighbors.sort_unstable(); neighbors.dedup();` remains in local adjacency extraction. | Exact binary adjacency order is not guaranteed for stock-route assertions. |
| `src/sim/pathfinding/zone_search_tests.rs` | Has insertion-order and undirected-exclusion synthetic tests. | Useful first-patch guardrails, not a stock Carville route oracle. |
| `src/sim/pathfinding/zone_map_tests.rs` | Has low-bridge zone-grid all-active-record coverage. | Guards low-record inclusion, not exact post-collapse routing. |
| `src/sim/bridge_state/walker.rs` / `src/sim/world/bridge_orchestrator.rs` | Prior work covers staged low collapse and zone-dirty orchestration. | A Carville stock fixture must drive the world/orchestrator path to final collapse before route assertion. |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Carville map presence and waypoints | verified | `Carville.mmx` lines `599..605` | none |
| Carville CABHUT `(57,49)` fallback starter | verified | `%TEMP%/bridge_hut_stock_scan_output_named.txt` lines `346`, `1439` | exact collapsed span cells from live state |
| DestroyableBridges / BridgeRepairHut gates | verified | `Carville.mmx`, `ini/rules*.ini` | none |
| Path search entry/retry hook | verified | `0x0042C900` | runtime hook implementation |
| Zone chain storage/log target | verified | `0x0042C290` | runtime dump and path-object decoding |
| Final low-collapse timing | verified | `0x0057BCF0`, `0x0057C2B0` | live proof that the fixture drives final state |
| Low bridge tube/land identity | verified | `0x00484AB0`, `0x00484F20` | none for readiness |
| Exact rebuilt Carville zone IDs | deferred | no live dump | debugger/logger run |
| Exact Carville post-collapse detour | deferred | no live dump | debugger/logger run |
| Exact final post-smoothing path cells | deferred | no live dump | decode path object or reconstruct node chain |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is there a concrete stock fixture? -> Yes: loose Carville waypoint 1=(79,50) to 0=(49,87), CABHUT (57,49), starter (60,52):0x11380.` (evidence: `Carville.mmx`; `%TEMP%/bridge_hut_stock_scan_output_named.txt`; Active in YR: Conditional)
- `[RESOLVED] OQ-02 - Is bridge destruction enabled? -> Yes for Carville and default rules.` (evidence: `Carville.mmx [SpecialFlags]`, `ini/rules.ini`, `ini/rulesmd.ini`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - Is CABHUT a live bridge hut type? -> Yes, `BridgeRepairHut=yes`.` (evidence: `ini/rules*.ini`; Active in YR: Yes)
- `[RESOLVED] OQ-04 - What hook owns route query/retry? -> `AStar_pathfind_search @ 0x0042C900`.` (evidence: fresh decompile; Active in YR: Yes)
- `[RESOLVED] OQ-05 - Where should zone chains be logged? -> `Zone_precheck @ 0x0042C290`, per-level chain length at `this+0xc74+level*4` and chain storage at `this+0xbc+level*1000`.` (evidence: fresh decompile; Active in YR: Yes)
- `[RESOLVED] OQ-06 - Must the fixture force second/final low collapse? -> Yes; first healthy hit only damages, final hit updates bridge zones.` (evidence: `0x0057BCF0`, `0x0057C2B0`; Active in YR: Yes)
- `[RESOLVED] OQ-07 - Can static docs name the exact detour? -> No; they prove ordering mechanics, not Carville's runtime rebuilt graph instance.` (evidence: `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE_GHIDRA_REPORT.md`; Active in YR: Conditional)
- `[RESOLVED] OQ-08 - Are synthetic Rust tests sufficient for the first zone-precheck patch? -> Yes for binary-style mechanics, no for exact stock route parity.` (evidence: `src/sim/pathfinding/zone_search_tests.rs`; Active in YR: n/a)
- `[DEFERRED] OQ-09 - What exact zone IDs are rebuilt after collapsing starter (60,52)?` (category: needs-runtime-debugger; reason: no live dump exists; next-step-if-pursued: run `gamemd_carville_low_bridge_post_collapse_route_trace`)
- `[DEFERRED] OQ-10 - Which exact detour wins for `(79,50)->(49,87)`?` (category: needs-runtime-debugger; reason: requires runtime zone graph and A* path capture; next-step-if-pursued: log `Zone_precheck` chain and final path cells)
- `[DEFERRED] OQ-11 - Is `(57,49)` stronger than `(47,60)` for route-delta visibility?` (category: requires-different-system-context; reason: requires a small Carville hut sweep; next-step-if-pursued: repeat the same logger for both huts)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Carville fixture seed is valid, but exact route is not observed. | `Carville.mmx`; fallback scan lines `346`, `1439`; prior `STOCK_LOW_BRIDGE_COLLAPSE_ROUTE_TRACE_GHIDRA_REPORT.md` | missing runtime trace, not Rust logic | trace harness / debugger notes; later stock fixture test | Capture gamemd start/goal, collapse state, zone chains, and final path before using Carville as an oracle. | Trace proposal: `gamemd_carville_low_bridge_post_collapse_route_trace` records waypoint `1 -> 0` after final collapse of starter `(60,52)`. | Do not write a Rust stock-route assertion from static docs alone. |
| Final low collapse is required before route rebuild. | `0x0057BCF0`, `0x0057C2B0` | partially covered; fixture must drive world/orchestrator path | `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs` | Ensure the fixture reaches destroyed-anchor state and calls zone refresh before path query. | Test/trace proposal: `carville_low_bridge_fixture_requires_final_collapse_before_route_delta`. | Do not assert route change after first healthy hit. |
| Synthetic zone-precheck parity can proceed without exact Carville route. | `0x0042C290`; `zone_search_tests.rs` source scan | Rust still has centroid/Manhattan approximation and sorted adjacency in some surfaces | `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_build.rs` | Implement binary-style cost/order/retry guardrails using synthetic graphs first. | Test proposal: `zone_precheck_target_zone_type_cost_beats_centroid_distance`; trace later supplies stock oracle. | Do not block all zone-precheck work on Carville, but do not claim stock route parity until trace exists. |

### Stale Docs / Follow-up Docs

No stale doc was found that positively claims the exact Carville detour direction is known. Existing docs already say the exact stock route is runtime-trace blocked.

Optional reinforcement wording for `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md`:

> Carville waypoint `1=(79,50)` to `0=(49,87)` after CABHUT `(57,49)` / starter `(60,52):0x11380` is fixture-ready for runtime tracing only. It must not become an exact route acceptance assertion until gamemd logs the post-collapse rebuilt zone IDs, `Zone_precheck` chain, retry exclusions, and final path cells.

## 10. Negative Facts / Do Not Do

- Do not assert an exact Carville north/south/east/west detour from static docs. Active in YR: Conditional; evidence proves the algorithm, not this runtime graph instance.
- Do not use Carville as a final stock route oracle until `Zone_precheck` chain and final path cells are logged from gamemd. Active in YR: Conditional.
- Do not assert route delta after only the first healthy low-bridge hit. Active in YR: Yes; evidence `0x0057BCF0`, `0x0057C2B0`.
- Do not clear or ignore tube identity as the collapse mechanism. Active in YR: Yes; evidence `0x00484AB0`, `0x00484F20`.
- Do not let synthetic zone-precheck tests claim stock-map route parity. Active in YR: n/a; this is an implementation discipline note.

## 11. Remaining Uncertainty

- Exact rebuilt numeric zone IDs for Carville after final collapse of starter `(60,52)`.
- Exact `Zone_precheck` chain and retry exclusion sequence for waypoint `1 -> 0`.
- Exact post-smoothing path cells returned by gamemd.
- Whether CABHUT `(47,60)` produces a clearer route delta than `(57,49)` for the same waypoint pair.

## Sources

- Fresh Ghidra decompiled: `0x0042C900`, `0x0042C290`, `0x00429A90`, `0x0057BCF0`, `0x0057C2B0`, `0x00484AB0`, `0x00484F20`, `0x0056C510`.
- Docs referenced: `STOCK_LOW_BRIDGE_COLLAPSE_ROUTE_TRACE_GHIDRA_REPORT.md`, `BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md`, `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE_GHIDRA_REPORT.md`, `LOW_BRIDGE_ZONE_PRECHECK_LANDTYPE10_CONNECTIVITY_GHIDRA_REPORT.md`, `BRIDGE_PARITY_GAP_SYSTEM_MODEL_SYNTHESIS.md`.
- Map/data checked: `C:/Users/enok/Documents/Command and Conquer Red Alert II/Carville.mmx`; `%TEMP%/bridge_hut_stock_scan_output_named.txt`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`.
- Rust surfaces scanned: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search_tests.rs`, `src/sim/pathfinding/zone_map_tests.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs`.
