# Stock Low Bridge Collapse Route Trace - Ghidra Research Report

Target question: Produce a named stock-map-derived acceptance scenario for low bridge collapse route parity.
Non-goals: Do not re-investigate the whole bridge A* system, all low bridge tube internals, or implement Rust changes.
Evidence needed to mark COMPLETE: Stock map fixture with low bridge cells and start/goal cells, verified collapse trigger, verified rebuilt zone/adjacency expectations, and verified chosen route/zone chain from binary runtime evidence.
Stop conditions: Mark PARTIAL if exact chosen runtime route cannot be observed without debugger/replay; still provide the best stock fixture, binary-backed static expectations, and instrumentation plan.

**Address(es):** `0x00429A90`, `0x0042C290`, `0x0057BCF0`, `0x0057C2B0`, `0x0056C510`, `0x00484AB0`, `0x00484F20`
**Investigation Mode:** coverage-map, scoped to a concrete stock-map acceptance fixture
**Claimed Scope:** static map-derived fixture for one stock low-bridge collapse route scenario, plus binary-backed expectations for collapse and pathing order.
**Non-Scope:** full runtime debugger replay, exact numeric rebuilt zone IDs, exhaustive route selection on every stock map, and Rust implementation.
**Confidence:** Medium-High for fixture data and binary mechanics; Low for exact chosen route because no runtime zone-chain capture was available.
**Active in YR:** Conditional. The binary paths are live in standard YR; the fixture triggers them when the selected low bridge is fully collapsed during play.
**Result:** PARTIAL. The report provides a concrete Carville fixture and acceptance/instrumentation plan, but not an observed gamemd route chain.

## 1. Overview

The best stock-map-derived scenario for this slot is loose `Carville.mmx` from `<ra2-install>/`. It is a shipped snow map with twelve low-branch CABHUT no-overlay fallback placements and enough bridge surface to make a low bridge collapse route test meaningful.

The proposed scenario is: use Carville start waypoints `1=(79,50)` and `0=(49,87)`, collapse the low bridge controlled by the CABHUT at `(57,49)` whose static fallback starter is `(60,52):0x11380`, then re-query the route between those start/goal cells. This should become a fixture/instrumentation test, not yet a parity assertion for a specific north/south detour, because the exact post-collapse zone IDs and chosen zone chain were not observed from a live gamemd run.

Active in YR: Conditional. Evidence: Carville retail map data from the installed loose map; CABHUT fallback scan output; binary collapse and pathing functions below.

## 2. Class Layout / Key Offsets

| Structure | Offset / data | Meaning for this fixture | Evidence | Active in YR |
|---|---:|---|---|---|
| `CellClass` | `+0x116` | signed tube index; low bridge predicate requires valid index | `CellClass__IsLowBridgeCell @ 0x00484AB0`, `GetTubeAtCell @ 0x00484F20` | Yes |
| `CellClass` | `+0xEC` | final land type; low bridge predicate requires `10` | `0x00484AB0` | Yes |
| `CellClass` | `+0x44` | overlay/state byte used by low bridge walkers | `0x0057BCF0`, `0x0057C2B0` | Yes |
| `BridgeRecord` | active byte equivalent | all-active zone rebuild includes active bridge records | `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` | Yes |
| Zone adjacency | final neighbor arrays | stored in writer/insertion order, not sorted by zone ID | prior `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE...`; spot-check `0x0042C290` | Yes |
| A* closed lists | ground/bridge arrays | flat low bridges stay ground-layer unless height/flag gate raises bridge layer | `AStar_main_loop @ 0x00429A90` | Yes |

## 3. Stock Fixture Data

### 3.1 Selected map

`Carville.mmx` is a loose installed retail map. Raw map strings show:

| Field | Value | Evidence |
|---|---|---|
| Source | `loose:Carville.mmx` | installed file scan |
| Theater | `SNOW` | raw map `[Map] Theater=SNOW` |
| Map size | `80x86` logical map section; parser grid `166x166` | raw map `[Map] Size=0,0,80,86`; parser probe |
| Local size | `2,6,76,68` | raw map `[Map] LocalSize=2,6,76,68` |
| Parser bridge surface | `G6 heads=150`, `decks=200` | `cargo test --test bridge_pathfinding_g5_g6_fidelity_probe probe_g5_g6_against_retail_maps -- --ignored --nocapture` |

Active in YR: Yes. These are standard retail map payload fields and the parser successfully loads the map.

### 3.2 Start and goal cells

The stock waypoints in Carville decode as `ry * 1000 + rx`:

| Waypoint | Encoded | Cell |
|---:|---:|---|
| `0` | `87049` | `(49,87)` |
| `1` | `50079` | `(79,50)` |
| `2` | `80110` | `(110,80)` |
| `3` | `118083` | `(83,118)` |
| `98` | `78078` | `(78,78)` |

Proposed acceptance start/goal: start at waypoint `1=(79,50)`, goal at waypoint `0=(49,87)`. This pair is intentionally near the northwest low-bridge/CABHUT cluster and is more useful than a synthetic isolated two-shore test because it exercises a real shipped map layout.

Active in YR: Conditional. Waypoints are stock map data; using them as a path query scenario is a test harness choice, not a verified scripted mission action.

### 3.3 Collapse trigger candidate

The prior stock CABHUT fallback scanner still exists at `%TEMP%/bridge_hut_stock_scan_output_named.txt` and lists all twelve Carville CABHUTs as low dispatch, no-overlay fallback. The most compact candidate for the waypoint `1 -> 0` route is:

| Hut | Dispatch | Overlay fast path | Fallback starter | Nearby overlays |
|---|---|---|---|---|
| `(57,49)` | low | false | `(60,52):0x11380` | none |

Other nearby Carville low fallback huts:

| Hut | Starter |
|---|---|
| `(47,60)` | `(50,63):0x11B80` |
| `(64,67)` | `(67,64):0x11B00` |
| `(74,68)` | `(71,65):0x11B80` |

Acceptance fixture should collapse `(57,49)` first, then, if no route delta is observed, repeat with `(47,60)` because both sit between waypoint `1` and waypoint `0`.

Active in YR: Conditional. The fallback branch is live in YR; this exact stock map placement reaches it when C4/hut-death or equivalent collapse dispatch targets that CABHUT.

## 4. Core Binary Logic

### 4.1 Low bridge identity is tube plus land type, not overlay alone

`CellClass__IsLowBridgeCell @ 0x00484AB0` returns true only if `CellClass+0x116` is a valid tube index and `CellClass+0xEC == 10`. `CellClass__GetTubeAtCell @ 0x00484F20` bounds-checks the same tube index and returns `g_TubeArray[index]`; it does not re-check land type.

Active in YR: Yes. These functions are used by standard low bridge/tube and zone paths.

### 4.2 Full low collapse is a second-stage state transition

`MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0` and `MapClass__DestroyBridgeWalker_EW_Low @ 0x0057C2B0` implement low bridge damage as a staged state machine:

| Axis | Healthy main first hit | Final hit / already damaged | Dirty cells | Zone rebuild |
|---|---|---|---|---|
| NS | `0x4A..0x4F -> 0x50` | `0x50..0x52 -> 0x64` | hit cell plus two perpendicular cells | final only |
| EW | `0x53..0x58 -> 0x59` | `0x59..0x5B -> 0x65` | hit cell plus two perpendicular cells | final only |

Assembly spot-check for NS final gate:

- `0x0057C229`, `0x0057C234`, `0x0057C241`: three `CellClass__RecalcAttributes` calls.
- `0x0057C268`: `TEST BL,BL`.
- `0x0057C26A`: branch skips zone update when the final-collapse flag is clear.
- `0x0057C270`: `CALL 0x0056C510` only when final-collapse flag is set.

Active in YR: Yes. These walkers are called by live low bridge destruction dispatch.

### 4.3 Rebuilt zone adjacency order matters

`Zone_precheck @ 0x0042C290` scans adjacency in stored order. Its candidate insert condition is unvisited or strictly lower cost; equal cost does not replace the earlier candidate. Heap bubble/sift comparisons are strict lower-cost comparisons, so equal-cost detours preserve insertion/order effects from the rebuilt graph.

Assembly spot-check:

- `0x0042C6A4..0x0042C6D8`: heap bubble path only moves while parent cost is greater.
- `0x0042C7C5..0x0042C835`: sift-down chooses a child only on strict lower cost.

Active in YR: Yes. `AStar_pathfind_search` uses this precheck for normal unit route queries.

### 4.4 Cell A* adds direction and first-found bias after zone precheck

`AStar_main_loop @ 0x00429A90` uses separate ground and bridge closed/g-cost arrays. Flat low bridge steps do not normally enter the bridge layer because the bridge-layer branch requires bridge flags plus a height delta threshold; low bridge cells at equal height remain ground-layer route cells.

Within the cell loop, directions are expanded `0..8`: compass directions first, then direction `8` tube jump. Compass directions call `AStar_compute_edge_cost` and add the per-direction epsilon table at `0x0081872C`; direction `8` uses tube exit distance and bypasses that helper. Already-closed entries are rejected if their existing cost is within the `1.009` tolerance from `0x007E37C0`, producing a first-found bias.

Active in YR: Yes. Evidence: decompile `0x00429A90`; prior reports and fresh spot-check of the function.

## 5. INI Keys

| Key / source | Value | Effect | Active in YR |
|---|---|---|---|
| `[SpecialFlags] DestroyableBridges` in Carville | `yes` | Confirms this stock map opts into bridge destruction. | Yes |
| `[General] DestroyableBridges` | `yes` in `rules.ini` and `rulesmd.ini` | Default bridge destruction enabled. | Yes |
| `BridgeRepairHut=yes` on `[CABHUT]` | present in `rules.ini` and `rulesmd.ini` | Makes the CABHUT destruction/repair hut paths live for this building type. | Yes |
| `MovementZone=` | per-unit type | Selects movement/passability matrix row used by zone precheck. | Yes |
| Low bridge overlay families | `LOBRDG*`, `LOBRDGE*`, etc. | Visual/state bytes; not sufficient alone for low pathing identity. | Yes |

## 6. Integration Points

The fixture should be exercised in this order:

1. Load loose `Carville.mmx`.
2. Build resolved terrain, bridge runtime state, path grid, and zone graph.
3. Query a ground unit path from `(79,50)` to `(49,87)` before collapse.
4. Collapse the low bridge via the CABHUT/fallback starter `(60,52)` associated with hut `(57,49)`.
5. If the first hit only damages the bridge, apply the second/final hit or explicitly run until the full-collapse state has `zones_dirty`.
6. Rebuild/refresh zones through the same world/orchestrator path used in gameplay.
7. Query the same route again and record:
   - start zone and goal zone at levels `2`, `1`, `0`;
   - adjacency list order for the start corridor;
   - chosen `Zone_precheck` chain;
   - final cell path and whether it uses or avoids the collapsed low span.

The expected binary-backed behavior is symbolic, not numeric: before final collapse, the low bridge record can connect the two sides; after final collapse, that low bridge connection must no longer be the route. If both around-routes are equal, the winning route comes from adjacency insertion order plus `Zone_precheck` strict-cost heap behavior, then A* direction epsilon.

Active in YR: Conditional. The load/path/collapse functions are live; the exact player action sequence is harness-defined.

## 7. Current Rust Implementation Status

| Surface | Current status from source scan | Risk |
|---|---|---|
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | Uses `BinaryHeap<Reverse<(i32, i32, ZoneId)>>` and iterates `adjacency.neighbors_of`. | `ZoneId` still participates in ties, unlike binary insertion-order ties. |
| `src/sim/pathfinding/zone_build.rs` | Local source still has `sort_unstable` / `dedup` around adjacency extraction at scan time. | Sorting loses binary writer order if still present when implementing this fixture. |
| `src/sim/pathfinding/zone_map.rs::are_adjacent` | Local source was scanned for sorted adjacency assumptions. | Binary adjacency lookup should not require sorted lists. |
| `src/sim/bridge_state/walker.rs` | Low first-hit and final-hit tests exist, including `low_direct_first_hit_damages_without_deactivating_zone_record_then_second_hit_collapses`. | Good isolated coverage, but not a stock-map end-to-end route test. |
| `src/sim/world/bridge_orchestrator.rs` | Collapse paths aggregate `zones_dirty` and refresh zones. | Fixture should go through this path, not just call a walker directly. |
| `src/sim/pathfinding/zone_map_tests.rs` | Has `stock_low_bridge_auto_shell_zone_grid_uses_low_records_without_explicit_tubes`. | Good low-record guard, but no Carville-derived post-collapse route assertion. |

No Rust files were modified in this slot.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Carville map selected as fixture | verified | installed `Carville.mmx`; raw map strings; parser probe output | none |
| Carville start/goal cells | verified map data | raw `[Waypoints]`: `1=50079`, `0=87049` | whether this exact route crosses the target low bridge must be observed |
| Carville low CABHUT fallback placement | verified static scan | `%TEMP%/bridge_hut_stock_scan_output_named.txt`, hut `(57,49)`, starter `(60,52):0x11380` | exact runtime collapse span cells require instrumentation |
| Low bridge predicate | verified | decompile `0x00484AB0`, `0x00484F20` | none |
| Low final collapse zone timing | verified | decompile `0x0057BCF0`, `0x0057C2B0`; assembly context `0x0057C229..0x0057C270` | exact Carville span state not captured |
| Zone precheck equal-cost tie behavior | verified | decompile `0x0042C290`; assembly contexts `0x0042C6A4`, `0x0042C7C5` | none for algorithm, but fixture chain not captured |
| A* dual-layer and direction/tolerance behavior | touched-not-exhausted | decompile `0x00429A90`; prior reports | exact fixture route still runtime-dependent |
| Exact rebuilt numeric zone IDs | deferred | no runtime dump | capture from gamemd debugger or Rust instrumentation aligned to binary graph |
| Exact chosen post-collapse zone chain | deferred | no runtime dump | needs debugger/replay or a validated map-corpus route logger |
| Exact final post-collapse cell path | deferred | no runtime dump | needs runtime route query after collapse |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which stock map should anchor the acceptance scenario? -> Use loose Carville.mmx because it has low fallback CABHUT placements, player waypoints, and 200 bridge deck cells.` (evidence: raw map strings; parser probe; fallback scan)
- `[RESOLVED] OQ-02 - What start/goal cells should the fixture use? -> Initial proposal is waypoint 1=(79,50) to waypoint 0=(49,87).` (evidence: raw `[Waypoints]` values)
- `[RESOLVED] OQ-03 - Which bridge/collapse event should be targeted? -> CABHUT at (57,49), low fallback starter (60,52):0x11380; fallback scanner reports no overlay fast path.` (evidence: `%TEMP%/bridge_hut_stock_scan_output_named.txt`)
- `[RESOLVED] OQ-04 - Is this a live low bridge path, not only overlay art? -> Low identity requires tube index plus final land type 10; stock resolved terrain builds bridge/tube facts for Carville.` (evidence: `0x00484AB0`; parser probe)
- `[RESOLVED] OQ-05 - Does first low bridge hit necessarily sever zones? -> No; first hit damages only, final/destroyed-anchor hit calls zone update.` (evidence: `0x0057BCF0`, `0x0057C2B0`, `0x0057C268..0x0057C270`)
- `[RESOLVED] OQ-06 - Does zone tie behavior sort by zone id? -> No; equal-cost candidates preserve insertion/heap order.` (evidence: `0x0042C290`, `0x0042C6A4..0x0042C7C5`)
- `[RESOLVED] OQ-07 - Does flat low bridge route use high bridge closed list? -> No for equal-height low bridge cells; A* has separate lists but the low bridge remains ground-layer unless height/flag gate raises it.` (evidence: `0x00429A90`)
- `[RESOLVED] OQ-08 - Is `DestroyableBridges` enabled for the stock map and defaults? -> Yes in Carville `[SpecialFlags]` and global `rules*.ini`.` (evidence: raw map strings; `ini/rules.ini`; `ini/rulesmd.ini`)
- `[RESOLVED] OQ-09 - What current Rust surfaces would own the fixture? -> Pathfinding zone search/build, bridge walker/orchestrator, and stock map/resolved terrain loading tests.` (evidence: source scan)
- `[DEFERRED] OQ-10 - What exact numeric zone IDs are rebuilt after collapsing Carville starter (60,52)?` (category: needs-runtime-debugger; reason: static map data and decompile do not expose runtime zone labels after mutation; next-step-if-pursued: log zone map arrays immediately before and after final collapse)
- `[DEFERRED] OQ-11 - Which exact post-collapse route wins in gamemd for (79,50)->(49,87)?` (category: needs-runtime-debugger; reason: requires live path query after mutation; next-step-if-pursued: debugger/replay trace or binary-aligned route logger)
- `[DEFERRED] OQ-12 - Is hut (57,49) the best Carville route-delta trigger, or does (47,60) produce a stronger route delta?` (category: requires-different-system-context; reason: needs route probe over multiple Carville huts; next-step-if-pursued: automated Carville-only collapse/path sweep)

Deferred pile is intentional: this slot can name a stock fixture and binary-backed expectations, but cannot claim the exact route without runtime path observations.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Carville has a stock low fallback bridge collapse candidate at CABHUT `(57,49)` with starter `(60,52):0x11380`. | fallback scan output; raw map `[Structures]` | missing fixture | stock map fixture/test loader; `src/sim/world/bridge_orchestrator.rs` | Load Carville-derived cells or a reduced fixture preserving the hut/starter/low bridge topology. | `carville_low_bridge_collapse_repaths_waypoint_1_to_0`: before collapse path from `(79,50)` to `(49,87)` is recorded; after final collapse of `(60,52)` the path no longer uses the collapsed low span. | Do not reduce this to a synthetic bridge-only strip if the goal is stock-map parity. |
| Low bridge full collapse, not first damage, triggers the zone update. | `0x0057BCF0`, `0x0057C2B0`, `0x0057C268..0x0057C270` | partially covered in isolated walker tests | `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs` | Fixture must drive the bridge to final destroyed-anchor state before asserting route rebuild. | Same fixture should assert first hit does not remove connectivity, second/final hit does. | Do not assert route change after a healthy first hit. |
| Equal-cost post-collapse route choice is insertion-order dependent, then A* direction/tolerance dependent. | `0x0042C290`; `0x00429A90`; prior tie-order report | mismatch/unchecked locally: zone ID participates in Rust heap tuple ties; adjacency sorting was found in source scan | `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs` | Preserve adjacency order and make equal-cost zone choices stable by insertion order. | Once Carville route logging exists, assert chosen zone chain exactly, not just reachability. | Do not sort adjacency by zone ID or use zone ID as an implicit equal-cost tiebreaker. |
| Low bridge identity persists as tube/land facts; collapse should remove active connectivity, not delete tube identity. | `0x00484AB0`, `0x00484F20`; low-collapse report | mostly present, needs stock route guard | `src/map/resolved_terrain.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/pathfinding/zone_map_tests.rs` | Keep tube facts while deactivating the collapsed low bridge record. | After collapse, fixture can inspect tube facts remain while active bridge record/connectivity is gone. | Do not clear `tube_index` as the collapse mechanism. |

## 11. Negative Facts / Do Not Do

- Do not claim the exact Carville detour direction yet. Active in YR: Conditional; evidence for algorithm exists, but the route was not observed.
- Do not use `BridgeRepaired`/EVA/radar presentation as the assertion surface for this fixture. Active in YR: Yes, but out of scope for low-collapse route parity.
- Do not treat the CABHUT no-overlay fallback as a custom-map edge case; Carville has twelve stock low fallback placements. Active in YR: Conditional; evidence: fallback scan.
- Do not assert a route change after only the first hit on a healthy low bridge. Active in YR: Yes; evidence: low walkers.
- Do not sort adjacency lists or let `ZoneId` decide equal-cost ties. Active in YR: Yes; evidence: `Zone_precheck` strict-cost behavior.

## 12. Sources

- Ghidra decompiled: `00429A90`, `0042C290`, `0057BCF0`, `0057C2B0`, `0056C510`, `00484AB0`, `00484F20`.
- Ghidra assembly contexts: `0057C229`, `0057C268`, `0057C270`, `0042C6A4`, `0042C7C5`.
- Reports referenced: `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE_GHIDRA_REPORT.md`, `BRIDGE_HUT_FALLBACK_STOCK_USAGE_GHIDRA_REPORT.md`, `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT.md`, `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`.
- Static map/fallback data: installed `Carville.mmx`; `%TEMP%/bridge_hut_stock_scan_output_named.txt`.
- Parser probe: `cargo test --test bridge_pathfinding_g5_g6_fidelity_probe probe_g5_g6_against_retail_maps -- --ignored --nocapture`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`.
- Rust surfaces scanned: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/pathfinding/zone_map_tests.rs`, `src/sim/pathfinding/core_tests.rs`.
