# Bridge Dual-Layer A* System Model Synthesis

**Date:** 2026-05-23  
**Scope:** bridge-aware A* layer selection, `Zone_precheck`, temporary bridge passability markers, bridge edge costs, closed-list tolerance, and the current Rust handoff.  
**Non-scope:** full locomotor bridge transitions, low-bridge TubeClass lifecycle except direction-8/tube bypass facts, exact stock Carville post-collapse route without runtime logging.  
**Output type:** model-synthesis with a small conflict map for stale pathfinding wording.  
**Status:** IMPLEMENTATION_SAFE for synthetic zone/cell pathing parity patches; NEEDS_REINVESTIGATE/RUNTIME_TRACE for exact stock Carville route.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| A* uses separate ground and bridge closed/cost arrays. | `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`; spot-check `0x00429ECF`, `0x00429F04`, `0x00429FFB`, `0x0042A00D` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `Zone_precheck` scans adjacency in stored order and equal-cost candidates do not replace earlier candidates. | `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`; spot-check `0x0042C5DE..0x0042C5EA`, `0x0042C6B3..0x0042C6BF` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Zone IDs are not tie-break keys in binary `Zone_precheck`. | `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Rust `find_zone_corridor` currently risks ZoneId tie ordering. | Rust scan: `src/sim/pathfinding/zone_search.rs::find_zone_corridor` uses `BinaryHeap<Reverse<(i32, i32, ZoneId)>>` | confirmed | high | n/a | IMPLEMENTATION_SAFE |
| `Zone_precheck` is accumulated-cost Dijkstra-like search, not centroid A*. | `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`; `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `CellClass+0x140 & 0x40000` multiplies current edge cost by `4.0` after code-2/entity handling and before bridge flank costs. | `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`; spot-check `0x004299AA..0x004299C2` | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| Bridge-layer flank multiplier is `10.0 / 1.0 / 2.0` based on destination orientation `0x800` and flank structural bit `0x100`. | `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| Direction epsilon is caller-side additive after helper return and `Pathfinder+0x04`, not scaled by marker/flank costs. | `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`; spot-check `0x00429F8A..0x00429F9D` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Direction 8/tube expansion bypasses `AStar_compute_edge_cost`, marker x4, bridge flank, and normal epsilon. | `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`; `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| `1.009` is not true reopening; it is an early closed-neighbor skip using stored layer `g` vs `current.g + 1.009`. | `ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`; spot-check `0x00429EEC`, `0x00429FFB`, `0x0042A00D` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Closed nodes are not reinserted on the selected layer once marker equals current epoch. | `ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`; spot-check insertion guards | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `FUN_0042B080` builds an alternate object list. | `PATHFINDER_ALT_OBJECT_LIST_FUN_0042B080_GHIDRA_REPORT.md` | contradicted | high | conditional | DOC_PATCH_READY |
| `FUN_0042B080` scans 5x5 existing `E4/E8` lists and returns first accepted object. | `PATHFINDER_ALT_OBJECT_LIST_FUN_0042B080_GHIDRA_REPORT.md` | confirmed | high | conditional | DESIGN_NEXT |
| Carville waypoint `1=(79,50)` to `0=(49,87)` after CABHUT `(57,49)` collapse is a good stock fixture. | `STOCK_LOW_BRIDGE_COLLAPSE_ROUTE_TRACE_GHIDRA_REPORT.md` | confirmed as fixture | medium | conditional | AUDIT_FIRST |
| Exact Carville post-collapse route/zone IDs are known. | same report | unknown | low | conditional | NEEDS_REINVESTIGATE |

## Current Model

YR A* treats bridge pathing as a dual-layer cell search. Each cell can be closed separately on ground and bridge layers. Layer choice for a neighbor is derived from current path height versus the neighbor cell level/bridge state; a ground closure does not block a bridge closure for the same cell.

Before cell A*, `Zone_precheck @ 0x0042C290` performs a three-level hierarchical graph search over zone adjacency. Its tie behavior is insertion-order sensitive: it scans adjacency arrays in stored order, replaces only on strictly lower cost, and heap movement also uses strict lower-cost comparisons. There is no ZoneId tie key. This matters directly after bridge damage or collapse, where rebuilt adjacency emission order can decide which equal-cost detour wins.

For normal compass directions, `AStar_compute_edge_cost @ 0x00429830` computes the edge cost in this order: base CanEnter code cost, code-2 moving-friendly urgency adjustment, optional `0x40000` marker x4, optional bridge flank multiplier, then caller-side `Pathfinder+0x04` multiply and direction epsilon add. Direction 8/tube expansion bypasses this helper entirely.

The closed-list `1.009` constant is an early skip tolerance, not a reopen rule. If a selected layer/cell is already closed and its stored `g` is less than `current.g + 1.009`, the neighbor is skipped immediately. If not, some legality/blocked-goal work can still run, but later insertion still rejects already-closed current-epoch entries.

`FUN_0042B080` is not a list builder. It is a fallback peer-object finder used by `UpdateBridgePassability` when the direct probe cell list is empty. It scans 25 nearby cells in order, selects each candidate's ground or bridge object list by candidate bridge bit and height gap, then returns the first object whose attached predicate accepts the original probe center/height.

## Implementation-Safe Facts

- Remove ZoneId from equal-cost `Zone_precheck` tie behavior before claiming post-collapse route parity.
- Preserve adjacency insertion order wherever it can feed zone-precheck decisions.
- Model `0x40000` as a temporary search-scoped destination cost multiplier, not static terrain, passability, cliff, or walkability.
- Keep direction epsilon final and additive.
- Direction 8/tube edges must not receive normal compass edge marker/flank/entity-helper costs.
- Add bridge-layer flank cost tests before or with implementation: first flank missing structural bridge = `10.0`, one flank = `1.0`, both flanks = `2.0`.
- Do not implement closed-node true reopen from the `1.009` finding.

## Doc-Patch-Ready Facts

- Any doc saying `FUN_0042B080` creates or fetches an alternate object list should be corrected to "5x5 fallback scan returning first accepted object."
- Any doc saying the `1.009` constant reopens closed nodes should be corrected to "early closed-neighbor skip tolerance; selected-layer closed nodes are not reinserted."
- Any doc implying binary `Zone_precheck` uses ZoneId or sorted neighbor ties should be corrected to insertion-order strict-cost behavior.
- Any doc calling `0x40000` a permanent cliff/ramp/RecalcAttributes flag should be corrected to temporary A* cost marker.

## Stale Or Superseded Claims

- **Superseded:** Rust can approximate `Zone_precheck` with `BinaryHeap<Reverse<(f_cost, g_cost, ZoneId)>>` and sorted adjacency.  
  **Replacement:** binary ties are order-sensitive and do not use ZoneId.

- **Superseded:** `1.009` means slightly better later paths reopen closed nodes.  
  **Replacement:** it only controls an early skip check; insertion still requires marker != current epoch.

- **Superseded:** Direction 8 is just another direction with normal cost decoration.  
  **Replacement:** direction 8 uses tube-jump logic and bypasses the normal edge-cost helper.

## Cross-Doc Conflicts

- Older bridge/pathfinding overview wording that calls the closed arrays "f-cost arrays" is imprecise for the tolerance branch. The latest `ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md` shows the arrays store accumulated `g` for the relevant compare.
- Older "alternate object list" naming conflicts with the new `FUN_0042B080` report. The new report wins for helper semantics, but the concrete `object+0x674` interface name remains unknown.
- Current Rust has already moved some adjacency structures to insertion-order lists, but `zone_search.rs` still contains heuristic/ZoneId tie behavior and one-hop corridor expansion that are not binary `Zone_precheck` semantics.

## Needs Re-Investigation

- Exact stock Carville route after low bridge collapse: use debugger/replay or a binary-aligned route logger for waypoint `1=(79,50)` to `0=(49,87)` after CABHUT `(57,49)` / starter `(60,52):0x11380`.
- Concrete identity of `object+0x674` and vtable `+0xA0` used by `FUN_0042B080`, if exact temporary marker peer modeling becomes an implementation target.
- Full setter/lifecycle for `PathfinderClass+0x01`, if bridge flank multiplier implementation needs to know when the branch is enabled for every live caller.
- Full parity of `Zone_precheck` cost model if implementing more than tie order: zone type base cost, slope cost, `edge+4` `0.001`, parent-level gating, and per-level exclusions.

## Do-Not-Implement Notes

- Do not assert an exact Carville detour direction yet.
- Do not use ZoneId as a stable tie-breaker for equal-cost zone paths.
- Do not convert bridge flank multipliers into hard diagonal blocking; binary penalizes, it does not always reject.
- Do not apply marker/flank costs to direction-8 tube edges.
- Do not treat `0x40000` as static map state.
- Do not implement `1.009` as closed-node reopening.
- Do not merge 25 nearby object lists for `FUN_0042B080`; it returns the first accepted object in scan/list order.

## Rust Handoff

Recommended design input for `/brainstorm bridge A* cost and zone-precheck parity`:

1. Start with `src/sim/pathfinding/zone_search.rs`: remove ZoneId tie influence and add an equal-cost adjacency-order regression. Decide whether to preserve the current heuristic/corridor expansion as a compatibility layer or introduce a binary-style precheck path.
2. Add `src/sim/pathfinding/core.rs` tests for bridge flank multiplier and direction-8 bypass invariants before touching broader movement call sites.
3. Treat closed-list `1.009` as a targeted edge-case patch only if a test proves current immediate closed skip diverges in blocked-goal behavior.
4. Defer `FUN_0042B080` object-footprint implementation unless `UpdateBridgePassability` marker work is in scope.
5. Defer exact Carville route assertions until route logging exists.

## Source Ledger

- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`
- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
- `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`
- `ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md`
- `ZONE_PRECHECK_0042C290_INSERTION_ORDER_GHIDRA_REPORT.md`
- `ASTAR_MAIN_LOOP_00429A90_REOPEN_TOLERANCE_GHIDRA_REPORT.md`
- `PATHFINDER_ALT_OBJECT_LIST_FUN_0042B080_GHIDRA_REPORT.md`
- `STOCK_LOW_BRIDGE_COLLAPSE_ROUTE_TRACE_GHIDRA_REPORT.md`
- `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `.swarm-claims.md` bridge dual-layer A* block, 2026-05-23T09:13+02:00
- `ini/rulesmd.ini`: `DestroyableBridges=yes`, `CABHUT BridgeRepairHut=yes`, live `MovementZone=` data
- `ini/rules.ini`: same base fallbacks
- Rust scan: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/core_tests.rs`, `src/sim/pathfinding/zone_search_tests.rs`
- Fresh Ghidra spot-checks during synthesis:
  - `0x0042C5DE..0x0042C5EA`, `0x0042C6B3..0x0042C6BF`: strict lower-cost zone replacement/heap movement; equality skips.
  - `0x004299AA..0x004299C2`, `0x00429F8A..0x00429F9D`: marker x4 before caller-side epsilon.
  - `0x00429EEC`, `0x00429FFB`, `0x0042A00D`: `1.009` early skip and no selected-layer reinsertion.
