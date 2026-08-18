# AStar Retry Reset / Exclusion Lifetime -- Ghidra Research Report

**Address(es):** `0x0042C900` (`AStar_pathfind_search`), `0x0042A5B0` (`PathfinderClass__Reset`), `0x0042CCD0` (`PathfinderClass__UpdateHierarchicalEdges`), `0x0042CF80` (`PathfinderClass__InvalidateZoneEdge`), `0x0042C290` (`Zone_precheck`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** retry-loop state lifetime in `AStar_pathfind_search`: total attempt budget, reset clear set, retry-local edge-exclusion persistence, marker/path reuse across retry reset, hierarchy flag behavior, and cross-zone precheck abort versus retry.  
**Non-Scope:** full A* neighbor legality, full `FloodFillReachableZones` internals, full edge-producer branch frequency, COM locomotor path behavior, and Rust implementation.  
**Confidence:** High for scoped binary behavior; Medium for Rust delta because this report only scanned current Rust and did not patch or run tests.  
**Active in YR:** Yes. `FootClass__Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search @ 0x0042C900` at `0x004CBC31` with default `-1` pathing arguments on the ordinary foot pathing spine.

## 0. Investigation Contract

Target question: What exact retry-local state survives `PathfinderClass__Reset` between failed hierarchical A* attempts, what does Reset clear, when are retry exclusions cleared or consumed, how are path markers/selected paths reused, what gates hierarchy retries, and when does a cross-zone precheck failure abort instead of retrying?

Non-goals: Do not re-investigate zone graph writer order, `Zone_precheck` tie rules, failed-edge producer edge identity, blocked-destination fallback, COM path mechanics, or broad Rust activation design beyond naming affected surfaces and tests.

Evidence needed to mark COMPLETE: decompile plus assembly context for `0x0042C900` entry, attempt-limit computation, A* failure branch, retry `UpdateHierarchicalEdges -> Reset -> +0x38 reread -> Zone_precheck` ordering, `PathfinderClass__Reset` clear set, and live caller proof from `FootClass__Run_AStar`; prior verified producer/consumer reports may be cited only for edge-exclusion semantics not re-covered here.

Stop conditions: stop after the retry/reset/exclusion lifetime model is proven and Rust handoff tests are concrete. Defer branch-frequency/runtime route captures and unrelated path legality details.

## 1. Overview

`AStar_pathfind_search` initializes search validity and non-exclusion scratch state at entry, clears the three retry-local exclusion vectors once, then attempts cell A*. On a failed hierarchical attempt it appends retry exclusions, calls `PathfinderClass__Reset`, rereads `Pathfinder+0x38`, and reruns `Zone_precheck` only if the search is still valid and the total attempt budget has not been exhausted.

The key lifetime distinction is that `Reset` is not the search-entry clear. It clears pools/heaps and advances marker epochs, but it does not clear edge exclusions, selected hierarchy paths, selected path lengths, or `Pathfinder+0x38`. Those fields intentionally survive the retry reset boundary.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning in this slice | Reset behavior | Active in YR |
|---|---:|---|---|---|
| `PathfinderClass` | `+0x0C` | A* trail pool; counter at pool `+0x180000`. | Reset writes counter zero. | Yes; `0x0042A5B0..0x0042A5B9`. |
| `PathfinderClass` | `+0x10` | A* node pool; counter at pool `+0x100000`. | Reset writes counter zero. | Yes; `0x0042A5BF..0x0042A5C3`. |
| `PathfinderClass` | `+0x14` | A* open heap descriptor. | Reset zeroes live heap entries and count. | Yes; decompile `0x0042A5C9..0x0042A5F9`. |
| `PathfinderClass` | `+0x18/+0x1C` | A* marker arrays for ground/bridge closed/list state. | Normally not cleared; epoch at `+0x28` invalidates old marks. Full clear only on stamp wrap. | Yes; `0x0042A5FB..0x0042A624`. |
| `PathfinderClass` | `+0x28` | current marker epoch/stamp. | Reset increments it; if it wraps to zero, Reset fully clears marker/cost arrays and increments again. | Yes; `0x0042A5FB..0x0042A688`. |
| `PathfinderClass` | `+0x38` | hierarchy/search-valid flag used by retry loop. | Reset does not write it. Entry sets it to `1`; invalidation can clear it. | Yes; entry `0x0042C909`, retry reread `0x0042CC85`. |
| `PathfinderClass` | `+0x3C` | urgency / retry cost mode copied from caller argument. | Reset does not write it. | Yes; `0x0042C927`. |
| `PathfinderClass` | `+0x64` | `Zone_precheck` node pool. | Not cleared as a vector; Zone_precheck overwrites nodes from pool start. | Yes; `0x0042C66E..0x0042C690` in prior consumer report. |
| `PathfinderClass` | `+0x68` | `Zone_precheck` heap descriptor. | Reset zeroes live heap entries and count; Zone_precheck also clears it per level. | Yes; `0x0042A5EA..0x0042A5F9`, `0x0042C300..0x0042C31A`. |
| `PathfinderClass` | `+0x74/+0x8C/+0xA4` | three per-level retry-local edge-exclusion vector objects. | Reset does not clear them; entry clears each once via vtable slot `+0x0C`. | Yes; entry clear `0x0042C912..0x0042C925`, retry reset `0x0042CC80`. |
| `PathfinderClass` | `+0x78/+0x84 + level*0x18` | exclusion data pointer/count consumed by `Zone_precheck`. | Persist across retry reset. | Yes; consumed at `0x0042C620..0x0042C664`; producer reports. |
| `PathfinderClass` | `+0xBC + level*1000` | selected `Zone_precheck` zone path per level. | Reset does not clear; successful `Zone_precheck` overwrites. | Yes; writer `0x0042C887..0x0042C8CE`; update reads before reset at `0x0042CF8D..0x0042D06D`. |
| `PathfinderClass` | `+0xC74 + level*4` | selected path length per level. | Reset does not clear; successful `Zone_precheck` overwrites. | Yes; writer `0x0042C88B`; invalidation read `0x0042CF8D`. |

## 3. Core Logic

### Search entry clear order

Active in YR: Yes. Evidence: live caller `FootClass__Run_AStar @ 0x004CBBA0` reaches call `0x004CBC31`; `AStar_pathfind_search` entry has no TS-only gate before this setup.

At `0x0042C909`, entry writes `Pathfinder+0x38 = 1`. It then calls `PathfinderClass__Reset` at `0x0042C90D`. Only after Reset does it clear the three exclusion vector objects, starting at `Pathfinder+0x74`, step `0x18`, count `3`, by vtable call slot `+0x0C` (`0x0042C912..0x0042C925`). This means exclusion-vector clearing belongs to the outer search entry, not to Reset.

### `PathfinderClass__Reset` clear set

Active in YR: Yes. Evidence: entry call `0x0042C90D` and retry call `0x0042CC80` both target `0x0042A5B0`; decompile plus assembly context `0x0042A5B0..0x0042A6B8`.

`Reset` clears:

- trail pool counter at `*(this+0x0C)+0x180000`;
- A* node pool counter at `*(this+0x10)+0x100000`;
- A* open heap entries/count through descriptor `this+0x14`;
- `Zone_precheck` heap entries/count through descriptor `this+0x68`;
- marker epoch `this+0x28`, with full marker/cost array clear only on epoch wrap.

`Reset` does not clear:

- `Pathfinder+0x38` hierarchy/search-valid flag;
- `Pathfinder+0x3C` urgency;
- per-level exclusion vector objects at `+0x74/+0x8C/+0xA4`;
- selected path arrays at `+0xBC + level*1000`;
- selected path lengths at `+0xC74 + level*4`.

### Attempt budget and retry ordering

Active in YR: Yes. Evidence: `0x004CBBA0 -> 0x0042C900`; attempt-limit assembly `0x0042CB8B..0x0042CBA1`; failure branch `0x0042CC02..0x0042CCC0`.

The retry budget is a total A* call budget, not extra retries. The expression at `0x0042CB8B..0x0042CBA1` computes `5` when the search-limit/max-depth argument is `-1`, otherwise `1`. The failure path increments the completed-attempt counter before the retry update (`0x0042CC71..0x0042CC79`) and compares it with the limit after `Reset` (`0x0042CC91..0x0042CC97`).

Default foot pathing through `FootClass__Run_AStar` passes `-1` at `0x004CBC1E`, so the common path has at most five total `AStar_main_loop @ 0x00429A90` calls. Non-default bounded searches have one total attempt and do not retry after a failed hierarchical A*.

### Retry-local exclusions persist across Reset

Active in YR: Yes. Evidence: entry clears vectors once (`0x0042C912..0x0042C925`); failed retry calls `UpdateHierarchicalEdges @ 0x0042CC79`, then `Reset @ 0x0042CC80`; no clear of vector bases `+0x74/+0x8C/+0xA4` occurs on that path.

After a failed hierarchical `AStar_main_loop`, the function calls `UpdateHierarchicalEdges` before `Reset`. The producer appends sorted undirected zone-edge keys into the same `Pathfinder+0x74 + level*0x18` vector objects consumed by later `Zone_precheck`. Because `Reset` does not clear those vectors, all exclusions appended by prior failed attempts remain visible to the next retry precheck in the same `AStar_pathfind_search` call.

The next outer search call clears them again at entry, so they are search-local, not global map state.

### Marker and path reuse across retry

Active in YR: Yes. Evidence: `Reset` stamp logic `0x0042A5FB..0x0042A688`; `Zone_precheck` writer `0x0042C887..0x0042C8CE`; retry ordering `0x0042CC79 -> 0x0042CC80 -> 0x0042CCB3`.

Marker arrays are reused by epoch. `Reset` normally only increments `+0x28`; old marker entries remain in memory but are invisible because all marker tests compare to the new epoch. On rare wrap to zero, `Reset` fully clears the marker/cost arrays and increments the stamp again.

Selected hierarchy paths are reused as ordinary memory, not cleared. The failed-edge producer reads the previous successful `Zone_precheck` path and path length before `Reset` (`UpdateHierarchicalEdges` is called at `0x0042CC79`, before `Reset` at `0x0042CC80`). If another retry is allowed, the subsequent `Zone_precheck` call at `0x0042CCB3` overwrites selected paths/lengths before the next `AStar_main_loop`. If that retry precheck fails, no next A* runs, so stale selected paths are not consumed by cell A*.

### Hierarchy flag behavior

Active in YR: Conditional. Evidence: entry sets `Pathfinder+0x38 = 1` at `0x0042C909`; local hierarchy-enable byte is set or cleared at `0x0042CAD0..0x0042CB1D`; same-zone failure clears the local byte at `0x0042CB61..0x0042CB86`; retry rereads `Pathfinder+0x38` after producer/reset at `0x0042CC85..0x0042CC93`.

There are two distinct hierarchy flags in this slice:

- `Pathfinder+0x38` is the retry validity flag. Entry sets it to `1`; `InvalidateZoneEdge` can clear it when no actionable path edge exists; `Reset` does not restore it.
- The low byte of the local/decompiled `param_8` is the current attempt's hierarchy-enabled flag. It is initially set only when the binary's ordinary pathing gates allow hierarchical search: relevant type/object checks pass, the mover-side byte at `+0x3D5` is nonzero, a virtual check returns false, and both resolved start and destination coordinates are in the playfield (`0x0042CAD0..0x0042CB1D`). If this local byte is false, failed A* returns without retry.

On retry, the caller copies `Pathfinder+0x38 != 0` back into the local hierarchy byte after `UpdateHierarchicalEdges` and `Reset`. This ordering is load-bearing: invalidation cannot be undone by Reset.

### Precheck failure: same-zone fallback vs cross-zone abort

Active in YR: Yes. Evidence: initial zone compare and branch `0x0042CB22..0x0042CB8B`; retry precheck branch `0x0042CCA1..0x0042CCC0`.

Initial precheck behavior:

- If the MapClass start/destination zone ids match and hierarchy is enabled, the function calls `Zone_precheck` at `0x0042CB58`. If it returns false, the binary logs the hierarchical failure and clears the local hierarchy byte, then still runs one cell A* attempt without hierarchy.
- If the MapClass start/destination zone ids differ and hierarchy is enabled, the function returns `0` immediately at `0x0042CB32..0x0042CB3F`. It does not run cell A* and does not enter the retry producer.
- If hierarchy is already disabled, cross-zone mismatch is not a hard gate; the function can log "A* without HS" and call cell A*.

Retry precheck behavior:

- After a failed hierarchical A* attempt, the binary appends exclusions, resets pools/markers, rereads `+0x38`, checks the total attempt limit, then calls `Zone_precheck` again at `0x0042CCB3` only if hierarchy remains enabled.
- If this retry `Zone_precheck` fails, control returns the current zero result at `0x0042CCC0`; it does not fall back to unrestricted A*.

## 4. INI Keys

No INI key is read directly by the retry/reset lifecycle code in this slice.

| Data | Binary source | Effect | Active in YR |
|---|---|---|---|
| `MovementZone=` | `TechnoTypeClass+0x5B4` when caller passes movement zone `-1` | Selects MapClass zone lookup and `Zone_precheck` matrix row. | Yes; `AStar_pathfind_search` reads via vtable `+0x84`, then `+0x5B4`. |
| caller search-limit/max-depth argument | stack/decompiled `param_6` | `-1` gives five total attempts; any other value gives one total attempt. | Yes; `0x0042CB8B..0x0042CBA1`. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `FootClass__Run_AStar -> AStar_pathfind_search` | ordinary foot pathing reaches this wrapper with default `-1` arguments. | decompile `0x004CBBA0`; assembly call `0x004CBC31`. | Yes. |
| Entry reset/exclusion clear | `+0x38=1`, Reset, then clear three exclusion vectors. | `0x0042C909..0x0042C925`. | Yes. |
| Failed hierarchical A* | calls `UpdateHierarchicalEdges`, then `Reset`, then rereads `+0x38`. | `0x0042CC79..0x0042CC93`. | Yes. |
| Retry precheck | reruns `Zone_precheck` only after producer/reset and only if budget/flag allow. | `0x0042CCA1..0x0042CCB8`. | Yes. |
| Exclusion consumer | `Zone_precheck` scans `+0x78/+0x84 + level*0x18` and skips matching packed undirected edges. | `0x0042C620..0x0042C664`; prior consumer reports. | Yes. |

## 6. Current Rust Implementation Status

| Rust surface | Current status vs this slice |
|---|---|
| `src/sim/pathfinding/zone_hierarchy.rs::ZonePrecheckExclusions` | Has search-local per-level undirected edge-key sets and consumer-side `zone_precheck_flat` support. This matches the consumer lifetime shape but not the producer/reset loop. |
| `src/sim/pathfinding/zone_hierarchy.rs::ZonePrecheckResult` | Retains per-level paths/marked sets from a successful precheck. This is necessary for retry producer parity, but Reset/path reuse ordering is not yet modeled as a production loop. |
| `src/sim/pathfinding/zone_search.rs` hierarchy branch | Runs `zone_precheck_flat` with `ZonePrecheckExclusions::default()` and immediately calls marker-gated A*. It does not loop through failed hierarchical A* attempts with persistent exclusions and `UpdateHierarchicalEdges` producer semantics. |
| `src/sim/pathfinding/zone_search.rs` compatibility corridor branch | Uses five total attempts and edge exclusions, but they come from Rust corridor failure (`exclude_corridor_edges`) rather than the verified binary producer and are not tied to Reset/marker/path lifetimes. |
| `src/sim/pathfinding/core.rs::HierarchyGate` | Models marker-gated cell expansion from `Zone_precheck` output, and correctly requires blocker-neighbor counts before production hierarchy use. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_pathfind_search @ 0x0042C900` entry state | verified | decompile `0x0042C900`; assembly `0x0042C909..0x0042C925` | none |
| attempt budget | verified | `0x0042CB8B..0x0042CBA1`; default caller `0x004CBC1E` | none |
| failed-A* retry ordering | verified | `0x0042CC02..0x0042CCC0`; assembly context `0x0042CC79..0x0042CCB8` | none |
| `PathfinderClass__Reset @ 0x0042A5B0` clear set | verified | decompile and assembly context `0x0042A5B0..0x0042A6B8` | exact heap descriptor struct names remain cosmetic |
| exclusion vector lifetime | verified | entry clear `0x0042C912..0x0042C925`; retry reset `0x0042CC80`; consumer `0x0042C620..0x0042C664` | none |
| marker epoch reuse | verified | `0x0042A5FB..0x0042A688`; prior marker-array consumers | none |
| selected path reuse | verified | writer `0x0042C887..0x0042C8CE`; producer read `0x0042CF8D..0x0042D06D`; reset omission | none for lifecycle |
| initial hierarchy-enable conditions | touched-not-exhausted | `0x0042CAD0..0x0042CB1D` | semantic names of all virtual/type gates are out-of-scope |
| cross-zone vs same-zone precheck failure | verified | `0x0042CB22..0x0042CB8B`; `0x0042CCA1..0x0042CCC0` | none |
| current Rust surfaces | verified for scan | `src/sim/pathfinding/zone_hierarchy.rs`, `zone_search.rs`, `core.rs` | no implementation performed |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-001 -- Is this exhaustive-slice or coverage-map? -> exhaustive-slice for retry/reset/exclusion lifetime only.` (evidence: user target and bounded function set)
- `[RESOLVED] OQ-002 -- Is `AStar_pathfind_search` active in standard YR? -> Yes, ordinary `FootClass__Run_AStar` calls it.` (evidence: `0x004CBBA0`, `0x004CBC31`; Active in YR: Yes)
- `[RESOLVED] OQ-003 -- What is the exact total attempt budget? -> `-1` search-limit gives five total `AStar_main_loop` attempts; non-`-1` gives one.` (evidence: `0x0042CB8B..0x0042CBA1`; Active in YR: Yes)
- `[RESOLVED] OQ-004 -- What does Reset clear? -> pool counters, A* open heap, precheck heap, and marker epoch/full arrays on stamp wrap.` (evidence: `0x0042A5B0..0x0042A6B8`; Active in YR: Yes)
- `[RESOLVED] OQ-005 -- Does Reset clear retry-local exclusions? -> No; only entry clears the three vectors.` (evidence: `0x0042C912..0x0042C925`, `0x0042CC80`; Active in YR: Yes)
- `[RESOLVED] OQ-006 -- Does Reset clear `Pathfinder+0x38`? -> No; entry sets it and invalidation can clear it; caller rereads after Reset.` (evidence: `0x0042C909`, `0x0042CC85`; Active in YR: Yes)
- `[RESOLVED] OQ-007 -- Do selected paths survive Reset? -> Yes as memory; update reads old paths before Reset and next successful precheck overwrites before next A*.` (evidence: `0x0042CC79..0x0042CCB8`, `0x0042CF8D`, `0x0042C887`; Active in YR: Yes)
- `[RESOLVED] OQ-008 -- How are marker arrays reused? -> epoch increment invalidates old marks; full clear only on wrap.` (evidence: `0x0042A5FB..0x0042A688`; Active in YR: Yes)
- `[RESOLVED] OQ-009 -- Does same-zone initial precheck failure abort? -> No; it clears local hierarchy and runs non-hierarchy cell A* once.` (evidence: `0x0042CB42..0x0042CB8B`, `0x0042CC13..0x0042CC19`; Active in YR: Yes)
- `[RESOLVED] OQ-010 -- Does cross-zone initial precheck failure abort? -> Yes when hierarchy local flag is true; no cell A* or retry producer runs.` (evidence: `0x0042CB22..0x0042CB3F`; Active in YR: Yes)
- `[RESOLVED] OQ-011 -- Does retry precheck failure fall back to unrestricted A*? -> No; it returns current zero result.` (evidence: `0x0042CCB3..0x0042CCC0`; Active in YR: Yes)
- `[RESOLVED] OQ-012 -- Are exclusions global graph mutations? -> No; they are Pathfinder-local vectors cleared at next outer search entry.` (evidence: `0x0042C912..0x0042C925`; producer/consumer reports; Active in YR: Yes)
- `[DEFERRED] OQ-013 -- Exact semantic names/defaults of all initial hierarchy-enable virtual/type gates.` (category: out-of-scope; reason: lifecycle model only needs their branch effect; next-step-if-pursued: dedicated caller/type-gate investigation)
- `[DEFERRED] OQ-014 -- Runtime frequency of zero/nonzero retry producer branches on stock maps.` (category: needs-runtime-debugger; reason: static evidence proves lifetime and ordering, not incidence; next-step-if-pursued: instrument failed hierarchy retries)
- `[DEFERRED] OQ-015 -- COM path around `TechnoType+0xD94` and path object behavior.` (category: out-of-scope; reason: separate pathing mode not needed for retry/reset lifetime; next-step-if-pursued: use existing COM path report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Entry clears exclusion vectors once; retry `Reset` does not clear them, so failed-attempt exclusions accumulate until the outer search returns. Active in YR: Yes. | `0x0042C912..0x0042C925`, `0x0042CC79..0x0042CCB8`, `0x0042A5B0` | missing in production hierarchy loop; consumer type exists | `src/sim/pathfinding/zone_search.rs`, `zone_hierarchy.rs::ZonePrecheckExclusions` | Keep one mutable exclusion set for the whole `AStar_pathfind_search` call; clear before first attempt only; pass same set into every retry precheck. | A first failed hierarchical A* excludes edge `1-2`; second retry precheck avoids `1-2`; third retry still avoids `1-2` plus any second failure edge. | Do not allocate/default exclusions inside each retry iteration. Proposed test name: `astar_retry_reset_preserves_search_local_exclusions`. |
| Failed hierarchical A* order is `UpdateHierarchicalEdges -> Reset -> reread +0x38 -> budget check -> Zone_precheck`; Reset does not re-enable hierarchy and next precheck overwrites selected paths before next A*. Active in YR: Yes. | `0x0042CC79..0x0042CCB8`, `0x0042CF8D`, `0x0042C887..0x0042C8CE` | hierarchy branch currently performs no failed-A* retry producer loop | `zone_search.rs`, future retry producer helper, `core.rs::HierarchyGate` caller | Call producer before clearing node/heap scratch; preserve producer's `+0x38` equivalent across reset; rerun precheck before next marker-gated A*. | Producer invalidates search by clearing validity; reset happens; wrapper returns failure without retrying or restoring hierarchy. | Do not call Reset before the producer or treat Reset as setting validity true. Proposed test name: `astar_retry_update_edges_before_reset_and_validity_survives_reset`. |
| Initial same-zone hierarchy failure disables hierarchy and runs one non-hierarchy A*; initial cross-zone hierarchy-enabled failure returns zero before A*; retry precheck failure returns zero with no unrestricted fallback. Active in YR: Yes. | `0x0042CB22..0x0042CB8B`, `0x0042CC13..0x0042CC19`, `0x0042CCB3..0x0042CCC0` | mostly modeled in current reduced/hierarchy branch, but retry activation must preserve the split | `zone_search.rs::find_path_zoned_marker_inner` | Preserve distinct same-zone fallback, cross-zone abort, and retry-precheck abort when adding production retry. | Same-zone incomplete hierarchy fixture still calls cell A*; cross-zone disconnected hierarchy returns `None`; post-failure retry precheck failure returns `None` without non-hierarchy A*. | Do not add a final unrestricted fallback after exhausted/failed hierarchical retries. Proposed test name: `astar_retry_precheck_failure_modes_match_same_zone_cross_zone_and_retry`. |

## 10. Negative Facts / Do Not Do

- Do not clear retry edge exclusions in `Reset`. Active in YR: Yes; evidence entry-only vector clears at `0x0042C912..0x0042C925` and retry `Reset` at `0x0042CC80` has no `+0x74` vector clear.
- Do not treat the default budget as one initial attempt plus five retries. Active in YR: Yes; evidence total limit `5` and counter compare at `0x0042CB8B..0x0042CC97`.
- Do not let `Reset` restore hierarchy validity. Active in YR: Yes; evidence `InvalidateZoneEdge` can clear `+0x38`, caller rereads after Reset at `0x0042CC85`, and Reset decompile has no `+0x38` write.
- Do not run cell A* for hierarchy-enabled cross-zone initial precheck failure. Active in YR: Yes; evidence `0x0042CB32..0x0042CB3F`.
- Do not consume stale selected paths after a retry `Zone_precheck` failure. Active in YR: Yes; evidence failure returns at `0x0042CCC0` before another `AStar_main_loop`.

## 11. Remaining Uncertainty

- The semantic names/default values for every initial hierarchy-enable gate at `0x0042CAD0..0x0042CB1D` remain intentionally unresolved; the branch effect and retry lifecycle are verified.
- Runtime frequency of retry producer branches is not measured here.
- COM path behavior around `TechnoType+0xD94` is outside this report.

## 12. Stale Docs / Follow-up Docs

`docs/research/PATHFINDERCLASS_GHIDRA_REPORT.md`

Replace the "Per-Search Reset" heading and the retry-loop pseudocode wording with:

> `PathfinderClass__Reset @ 0x0042A5B0` is a scratch reset, not the full per-search initializer. It clears pool counters, the A* open heap, the `Zone_precheck` heap, and advances the marker epoch; on epoch wrap it fully clears marker/cost arrays. It does not clear `Pathfinder+0x38`, `+0x3C`, the three edge-exclusion vectors at `+0x74/+0x8C/+0xA4`, or the selected hierarchy paths/lengths at `+0xBC/+0xC74`. `AStar_pathfind_search @ 0x0042C900` clears the exclusion vectors once at outer search entry. On failed hierarchical A*, the retry order is `UpdateHierarchicalEdges -> Reset -> reread +0x38 -> budget check -> Zone_precheck`; exclusions and validity survive Reset. The default budget is five total A* attempts, not five retries after the first.

`docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`

Replace simplified "Reset clears open/closed sets" and "up to 5 attempts" wording with:

> `Reset` advances marker epochs instead of clearing normal marker arrays, except on stamp wrap. The three retry-local edge-exclusion vectors are cleared only by `AStar_pathfind_search` entry and persist across retry Reset calls. Default `-1` search limit means five total `AStar_main_loop` calls; any non-`-1` limit means one total call.

## Sources

- Ghidra decompiled/read this slot: `0x0042C900`, `0x0042A5B0`, `0x0042CCD0`, `0x0042CF80`, `0x0042C290`, `0x004CBBA0`.
- Ghidra assembly contexts/ranges: `0x0042C909..0x0042C925`, `0x0042CAD0..0x0042CB1D`, `0x0042CB22..0x0042CBA1`, `0x0042CC02..0x0042CCC0`, `0x0042A5B0..0x0042A6B8`, `0x004CBC31`.
- Prior reports referenced: `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`.

Status: COMPLETE.
