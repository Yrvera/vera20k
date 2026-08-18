# AStar Main Loop Level-0 Marker Gate - Ghidra Research Report

**Address(es):** `0x00429A90` (`AStar_main_loop`), `0x0042C290` (`Zone_precheck`), `0x0042C900` (`AStar_pathfind_search`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** How `AStar_main_loop` consumes `Zone_precheck` level-0 chosen-zone markers when hierarchical A* is enabled, including same-zone/precheck fallback, off-marker `CellClass+0x122` exception, and immediate Rust handoff for replacing one-ring corridor expansion in flat Foundation First pathing.
**Non-Scope:** full `UpdateHierarchicalEdges` retry-edge producer, full A* edge costs, slope cost, stock-map route outcome, full CellClass `+0x122` writer lifecycle beyond existing verified report.
**Confidence:** High for marker-gate control flow; Medium for first-slice sufficiency because exact retry-edge producer remains a separate slot.
**Active in YR:** Yes. `FootClass::Run_AStar -> AStar_pathfind_search -> Zone_precheck/AStar_main_loop` is the live standard pathfinding chain; no TS-only gate was found for this slice.

## 0. Working Notes Required By Swarm Prompt

Target question: Verify exactly how `AStar_main_loop` consumes `Zone_precheck` level-0 chosen-marker/path output when hierarchy is enabled, and decide whether Rust can replace `expand_corridor()` with a marker/path handoff for flat ground pathing in the first Foundation First slice.

Non-goals: Do not re-investigate full edge costs, slope cost, stock route outcome, full retry-edge producer behavior, or broad bridge pathing.

Evidence needed to mark COMPLETE: binary proof that `Zone_precheck` writes level-0 chosen markers/path; binary proof that `AStar_main_loop` reads level-0 marker state and gates neighbor expansion; proof of same-zone/precheck fallback in the wrapper; proof whether `CellClass+0x122` changes strict marker behavior; Rust surface scan showing current `expand_corridor()` mismatch.

Stop conditions: Stop after answering marker consumption and first-slice readiness; defer retry producer and layered bridge entry questions to their own slots.

## 1. Overview

`Zone_precheck` does not hand cell A* a widened zone set. It writes per-level chosen-zone marker arrays and stored chosen chains. When hierarchy remains enabled, `AStar_main_loop` checks a candidate neighbor's level-0 zone against `PathfinderClass+0x40` before calling the mover's `Can_Enter_Cell` predicate.

The marker gate is not simply "zone must be on the chosen path." For the normal/near-height neighbor branch, a candidate outside the marked level-0 path is still allowed when `CellClass+0x122` is nonzero. That byte is the previously verified blocker-neighbor refcount, not fog, not bridge state, and not an amphibious terrain flag.

## 2. Key Offsets And Fields

| Owner | Offset / global | Meaning in this slice | Active in YR |
|---|---:|---|---|
| `PathfinderClass` | `+0x28` | epoch/stamp written into marker arrays | Yes; read/write in `0x0042C38F..0x0042C3B6`, `0x00429E9D..0x00429EA7` |
| `PathfinderClass` | `+0x40` | level-0 chosen-zone marker array consumed by cell A* | Yes; loaded into stack and read around `0x00429E93..0x00429EA7` |
| `PathfinderClass` | `+0xBC + level*1000` | stored chosen zone chain per hierarchy level | Yes; written at `0x0042C887..0x0042C8CE` |
| `PathfinderClass` | `+0xC74 + level*4` | chosen chain length per level | Yes; written at `0x0042C887..0x0042C88B` |
| `DAT_0087F858` | zone-index table | 10-byte cell-zone tuple; first word is level-0 zone id | Yes; `AStar_main_loop` reads first word after `ZoneMap__CellToZoneIndex` |
| `CellClass` | `+0x122` | blocker-neighbor refcount exception for off-marker cells | Yes; read at `0x00429EB1`; writers verified in `CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md` |

## 3. Core Logic

### 3.1 `Zone_precheck` writes the markers cell A* later reads

At each level, `Zone_precheck` computes source/destination zone ids, selects the current level marker array, and immediately stamps both endpoints:

- `0x0042C38F`: loads `Pathfinder+0x4C+level*4` visited array.
- `0x0042C393`: loads `Pathfinder+0x40+level*4` chosen-path marker array.
- `0x0042C39F`: loads epoch from `Pathfinder+0x28`.
- `0x0042C3AA`: stamps source zone in chosen-path marker array.
- `0x0042C3B6`: stamps destination zone in chosen-path marker array.

Active in YR: Yes. `AStar_pathfind_search @ 0x0042C900` calls `Zone_precheck @ 0x0042CB58` and retry calls again at `0x0042CCB3`.

When the Dijkstra result is reconstructed, `Zone_precheck` stamps every chosen zone and stores the path:

- `0x0042C867..0x0042C871`: walks the parent chain and writes epoch into the current level chosen marker array.
- `0x0042C887..0x0042C88B`: writes path count to `Pathfinder+0xC74+level*4`.
- `0x0042C8AA..0x0042C8CE`: writes chosen zone ids into `Pathfinder+0xBC + level*1000`.

Active in YR: Yes. Same live caller chain; these writes feed both `AStar_main_loop` and retry invalidation docs.

### 3.2 `AStar_main_loop` consumes only level-0 marker state for neighbor pruning

For each neighbor candidate:

- `0x00429E85`: calls `ZoneMap__CellToZoneIndex`.
- `0x00429E8A..0x00429E9A`: indexes `DAT_0087F858` and reads the first 16-bit zone id, i.e. level-0.
- `0x00429EA4`: compares `Pathfinder+0x40[level0_zone]` with the current epoch.
- `0x00429EA7`: if marked, jumps to the normal closed-list/cost path.

Active in YR: Yes. This is inside the live `AStar_main_loop` called from `AStar_pathfind_search @ 0x0042CC02`.

This means a first Foundation implementation should hand cell A* the level-0 marker predicate, not a one-ring-expanded zone set. The stored path array is still needed for retry/invalidation and diagnostics, but the direct cell expansion gate uses the marker array.

### 3.3 Off-marker cells are not all rejected: `CellClass+0x122` is an immediate exception

When the candidate's level-0 zone is not marked:

- `0x00429EA9..0x00429EAF`: tests a near-height/normal branch byte; if false, jumps to the alternate list path at `0x00429F04` without the `+0x122` check.
- `0x00429EB1`: reads `byte [neighbor_cell + 0x122]`.
- `0x00429EB7..0x00429EB9`: if nonzero, accepts the off-marker candidate and jumps to the normal path at `0x00429EC7`.
- `0x00429EBB..0x00429EC1`: if `+0x122 == 0` and hierarchy flag `param_7 != 0`, skips the neighbor by jumping to `0x0042A1A1`.
- If hierarchy flag `param_7 == 0`, the same off-marker zero-`+0x122` candidate is accepted.

Active in YR: Yes. `param_7` is passed from `AStar_pathfind_search`'s hierarchy flag at `0x0042CC02`, and the gate has no rules/default feature flag.

`CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md` verifies `+0x122` is a per-cell refcount of blockers in the 8 neighboring cells. Therefore, binary hierarchical A* allows off-marker expansion next to blockers/obstacles, but prunes open off-marker cells. This is the main detail that makes a strict "chosen zones only" replacement incomplete.

### 3.4 Same-zone and precheck-failure wrapper behavior

`AStar_pathfind_search` controls whether hierarchy is enabled when `AStar_main_loop` runs:

- If start and destination zone ids differ and hierarchy is enabled, the wrapper returns `0` before cell A* (`0x0042CB32..0x0042CB3F`).
- If zone ids match, it may call `Zone_precheck`; on precheck failure it logs the hierarchy failure and clears the local hierarchy flag (`0x0042CB42..0x0042CB86`), then still calls cell A*.
- `AStar_main_loop` is called at `0x0042CC02` with the current hierarchy flag.

Active in YR: Yes. This is the standard wrapper around live foot A*.

So same-zone precheck failure is not a strict abort and does not use the marker gate after the flag is cleared. Cross-zone hierarchy failure does not enter cell A* at all.

## 4. Current Rust Implementation Status

Rust currently approximates the binary handoff:

- `src/sim/pathfinding/zone_search.rs::find_path_zoned_marker` uses `find_zone_corridor()`, then calls `expand_corridor()` and passes the widened set to cell A*.
- `src/sim/pathfinding/zone_search.rs::expand_corridor` adds every one-hop neighbor zone to the allowed set.
- `src/sim/pathfinding/core.rs::AStarOptions::corridor` is a simple allowed `BTreeSet<ZoneId>`.
- `src/sim/pathfinding/core.rs::astar_search` filters candidates by `allowed.contains(cell_zone)` after passability/entity checks.

Current Rust does not have:

- a `Zone_precheck` level-0 marker array predicate;
- the `CellClass+0x122` off-marker exception;
- per-cell blocker-neighbor refcount input to A*;
- a distinction between "marked zone", "off-marker but near blocker", and "off-marker open terrain".

## 5. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Cell A* gates neighbor expansion using level-0 chosen-zone markers from `Zone_precheck`, not a free one-ring zone expansion. Active in YR: Yes. | `Zone_precheck` marker writes `0x0042C38F..0x0042C3B6`, path writes `0x0042C887..0x0042C8CE`; `AStar_main_loop` marker read `0x00429E85..0x00429EA7`. | mismatch: Rust passes `expand_corridor()` allowed zones. | `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs::AStarOptions`, future binary-style precheck result surface. | Replace corridor widening with a marker/path handoff predicate for flat hierarchical A*. | A synthetic zone map where the one-ring neighbor contains the geometrically tempting route but is not level-0 marked should be rejected unless an exception applies. Proposed test: `astar_hierarchy_rejects_unmarked_one_ring_zone_without_blocker_exception`. | Do not replace `expand_corridor()` with strict chosen zones only without also implementing the `+0x122` exception. |
| Off-marker normal/near-height candidates are accepted when `CellClass+0x122 != 0`, and rejected only when `+0x122 == 0` while hierarchy flag is true. Active in YR: Yes. | `0x00429EA4..0x00429EC1`; field semantics from `CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`. | missing: no blocker-neighbor refcount surface in path grid/A* options. | `src/sim/pathfinding/core.rs`, `PathGrid` or a search-side `BlockerNeighborCounts` surface, entity/building/wall/terrain occupancy integration later. | Model `+0x122` as a search input or deterministic grid so off-marker cells adjacent to blockers can still be explored in hierarchical mode. | Synthetic off-marker cell with blocker-neighbor count 1 is allowed, while identical off-marker cell with count 0 is pruned. Proposed test: `astar_hierarchy_allows_off_marker_cell_with_blocker_neighbor_count`. | Do not call this fog/shroud, water, bridge edge, or terrain type state. |
| Same-zone `Zone_precheck` failure disables hierarchy and still runs cell A*; cross-zone hierarchy failure aborts before cell A*. Active in YR: Yes. | `0x0042CB22..0x0042CB86`; `AStar_main_loop` call at `0x0042CC02`. | partially present in Rust flat pathing; future precheck replacement must preserve it. | `src/sim/pathfinding/zone_search.rs::find_path_zoned_marker`, future `ZonePrecheckResult` wrapper. | Preserve wrapper-level distinction before marker-gated A*. | Same-zone forced precheck miss runs unrestricted A*; cross-zone forced miss returns no path without invoking cell A*. Proposed test: `zone_precheck_same_zone_failure_clears_hierarchy_before_astar`. | Do not collapse all hierarchy failures into one behavior. |

## 6. Negative Facts / Do Not Do

- Do not implement the first marker-handoff slice as "only zones in the stored path are searchable." Binary hierarchical A* has the `+0x122 != 0` off-marker exception. Active in YR: Yes; evidence `0x00429EB1..0x00429EC1`.
- Do not preserve Rust's one-ring `expand_corridor()` and claim parity. No unconditional one-hop widening was found in the marker gate. Active in YR: No; evidence marker read `0x00429EA4`, no neighbor-ring zone add in `AStar_main_loop`.
- Do not label `CellClass+0x122` as fog/shroud, water, amphibious, bridge, or ore-neighbor state. The corrected report identifies it as blocker-neighbor refcount. Active in YR: Yes.
- Do not put the `+0x122` gate inside `Can_Enter_Cell`; it is in `AStar_main_loop` before the virtual `+0x1AC` call. Active in YR: Yes; evidence `0x00429EB1` precedes call at `0x0042A030`.
- Do not use the stored `+0xBC` path count alone as the A* gate. Cell A* checks the marker array; stored chains are needed for retry/update/debug surfaces.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_precheck` endpoint/path marker writes | verified | `0x0042C38F..0x0042C3B6`, `0x0042C887..0x0042C8CE` | none for this slice |
| `AStar_main_loop` level-0 marker read | verified | `0x00429E85..0x00429EA7` | none for this slice |
| `CellClass+0x122` exception polarity | verified | `0x00429EB1..0x00429EC1`; prior writer report | full writer lifecycle not re-run here |
| same-zone/cross-zone wrapper distinction | verified | `0x0042CB22..0x0042CB86`, `0x0042CC02` | none for this slice |
| exact retry-edge producer after failed A* | deferred | explicit non-scope | slot 2 |
| layered bridge wrapper scope | deferred | explicit non-scope | slot 3 |
| slope cost | deferred | explicit non-scope | existing slope report |
| stock route outcome | deferred | explicit non-scope | runtime trace |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does `Zone_precheck` write a level-0 marker array consumed by cell A*? -> Yes, `Pathfinder+0x40+level*4` is stamped by `Zone_precheck`, and level 0 is consumed by `AStar_main_loop`.` (evidence: `0x0042C393`, `0x0042C3AA..0x0042C3B6`, `0x00429EA4`)
- `[RESOLVED] OQ-2 - Does cell A* consume stored path arrays or marker arrays for candidate gating? -> Candidate gating reads the marker array; stored path arrays are written separately for chain/retry surfaces.` (evidence: `0x00429EA4`, `0x0042C887..0x0042C8CE`)
- `[RESOLVED] OQ-3 - Are off-marker candidates always rejected when hierarchy is enabled? -> No. Normal/near-height off-marker candidates with `CellClass+0x122 != 0` are accepted; zero is skipped only when hierarchy flag is true.` (evidence: `0x00429EB1..0x00429EC1`)
- `[RESOLVED] OQ-4 - What is `CellClass+0x122`? -> Existing verified report identifies it as an 8-neighbor blocker refcount, not fog/water/bridge state.` (evidence: `CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-5 - Does same-zone precheck failure still run cell A*? -> Yes, the wrapper clears hierarchy and continues to `AStar_main_loop`.` (evidence: `0x0042CB42..0x0042CB86`, call `0x0042CC02`)
- `[RESOLVED] OQ-6 - Does cross-zone hierarchy failure enter cell A*? -> No, it returns zero before `AStar_main_loop`.` (evidence: `0x0042CB32..0x0042CB3F`)
- `[RESOLVED] OQ-7 - Is current Rust's `expand_corridor()` a binary behavior? -> No evidence of unconditional one-ring widening; Rust explicitly does this as an approximation.` (evidence: `src/sim/pathfinding/zone_search.rs::expand_corridor`; binary marker gate `0x00429EA4..0x00429EC1`)
- `[DEFERRED] OQ-8 - Which retry edge does `UpdateHierarchicalEdges` produce after a failed marker-gated A*?` (category: out-of-scope; reason: assigned to swarm slot 2; next-step-if-pursued: inspect `0x0042CCD0` producer chain)
- `[DEFERRED] OQ-9 - Does layered bridge pathing need the same first-slice rewrite?` (category: out-of-scope; reason: assigned to swarm slot 3; next-step-if-pursued: inspect layered path entry and Rust wrapper)
- `[DEFERRED] OQ-10 - Exact stock route after low bridge collapse.` (category: needs-runtime-debugger; reason: static marker gate does not determine full map route alone; next-step-if-pursued: runtime route logging)

## 9. Stale Docs / Follow-up Docs

- `PATHFINDING_ASTAR_GHIDRA_REPORT.md`: replace "Fog of war check: `cell+0x122`" with "`CellClass+0x122` blocker-neighbor refcount exception: in hierarchical A*, off-marker normal/near-height candidate cells with `+0x122 == 0` are pruned, while `+0x122 != 0` candidates are allowed. This is not fog/shroud."
- `BRIDGE_ASTAR_PRECHECK_RETRY_INTEGRATION_GHIDRA_REPORT.md`: replace "known `CellClass+0x122` occupancy-adjacent exception from prior reports" with "verified `CellClass+0x122` blocker-neighbor refcount exception: off-marker cells adjacent to blockers are still eligible in hierarchical A*; strict marker-only gating is incomplete."
- `src/sim/pathfinding/zone_search.rs` comments should eventually stop calling the replacement just "corridor" once a marker-result handoff exists; no repo edit was made in this research slot.

## 10. Conclusion

Foundation First is safe to continue only with this refinement: replace `expand_corridor()` with a binary-style marker/path handoff **plus** the `CellClass+0x122` off-marker blocker-neighbor exception. A strict chosen-zone-only A* gate would be a new parity bug. Exact retry-edge production and layered bridge entry can remain assigned to their dedicated slots; they are not blockers for understanding this marker gate.

## Sources

- Ghidra decompile/read-only assembly: `AStar_main_loop @ 0x00429A90`
- Ghidra decompile/read-only assembly: `Zone_precheck @ 0x0042C290`
- Ghidra decompile/read-only assembly: `AStar_pathfind_search @ 0x0042C900`
- `docs/research/CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_ASTAR_PRECHECK_RETRY_INTEGRATION_GHIDRA_REPORT.md`
- Rust scan: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`
