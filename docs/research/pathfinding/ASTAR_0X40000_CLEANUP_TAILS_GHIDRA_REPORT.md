# AStar 0x40000 Cleanup Tails - Ghidra Research Report

**Address(es):** `0x00429A90` (`AStar_main_loop`), `0x0042C900` (`AStar_pathfind_search`), `0x0042ACF0` (`PathfinderClass::UpdateBridgePassability`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** static control-flow proof for normal `AStar_main_loop` / `AStar_pathfind_search` exits that can surround `CellClass+0x140 & 0x40000` temporary toggles made by `PathfinderClass::UpdateBridgePassability`.
**Non-Scope:** runtime exception/process abort behavior, OS-level termination, thread interruption, full A* cost semantics, full `0x0042ACF0` marker geometry, and global xref census outside the live `FootClass::Run_AStar -> AStar_pathfind_search -> AStar_main_loop` chain.
**Confidence:** High for normal static exits in the scoped functions; Medium for global "no other caller exists" wording because this slot did not have a direct xref-list tool.
**Active in YR:** Yes. `FootClass::Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search`, which calls `AStar_main_loop`; no TS-only gate was found in this slice.

## Target Question

Does every normal/static exit from the live `AStar_main_loop` / `AStar_pathfind_search` path pair temporary `CellClass+0x140 & 0x40000` toggles from `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` with cleanup, or is there a static early/abnormal path that can leave the bit set?

Answer: every normal static `AStar_main_loop` exit after a pre-toggle reaches a cleanup tail. `AStar_pathfind_search` early returns before `AStar_main_loop` do not call the toggler and therefore have nothing to clean up. The only unproven cases are runtime-abnormal interruption cases outside ordinary return control flow.

## Non-Goals

- Do not re-document the full `0x0042ACF0` 5x5/path-queue marker geometry; see `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.
- Do not prove process abort, access violation unwinding, debugger break, or power-loss cleanup.
- Do not claim `0x40000` is a persistent cell/path-grid flag.
- Do not modify Rust implementation.

## Evidence Needed To Mark COMPLETE

- Decompile `AStar_main_loop @ 0x00429A90` and identify every ordinary return tail.
- Disassembly/address-context proof that the pre-search call to `0x0042ACF0` is gated by `Pathfinder+0x3C != 0`.
- Disassembly/address-context proof that success and failure tails both check `Pathfinder+0x3C` and call `0x0042ACF0` before returning.
- Decompile/disassembly of `AStar_pathfind_search @ 0x0042C900` proving its early returns either precede `AStar_main_loop` or consume an already-cleaned `AStar_main_loop` return.
- Verify the only `0x0042ACF0` internal early return after changing `Pathfinder+0x3C` happens on the no-marker-written path.

## Stop Conditions

- Stop at normal static return paths. Runtime exceptions/process aborts are explicitly recorded as uncertainty.
- Stop at the live A* call chain and do not expand into global caller census without a direct xref tool.
- Stop before writing Rust or changing any repo source.

## Core Logic

### 1. Pre-toggle only occurs on the nontrivial A* body path

Active in YR: Yes.

`AStar_main_loop` first rejects null start/destination cells and the already-at-target/same-height case before entering the pre-toggle body. Assembly context:

- `0x00429BF6` and `0x00429C00`: coordinate mismatch branches jump to `0x00429C10`.
- `0x00429C02..0x00429C0A`: compares `Pathfinder+0x30` and `+0x34`; equal jumps to the zero-return tail at `0x0042A451`.
- `0x00429C10..0x00429C1A`: loads `Pathfinder+0x3C`, tests it, and calls `0x0042ACF0` only when nonzero.

Therefore a same-cell/same-height return does not need cleanup because it never toggled.

### 2. Success tail pairs the pre-toggle before returning path result

Active in YR: Yes.

After the search loop converges on a successful path, branch checks at `0x0042A3E8`, `0x0042A3F0`, `0x0042A3F6`, and `0x0042A3FC` send failure-like cases to `0x0042A43E`. The true success path calls reconstruction/smoothing, then checks `Pathfinder+0x3C` and calls cleanup:

- `0x0042A406`: calls `AStar_reconstruct_path`.
- `0x0042A41E`: calls final path optimization helper.
- `0x0042A423..0x0042A42D`: loads/tests `Pathfinder+0x3C`; if nonzero, pushes the foot object and calls `0x0042ACF0`.
- `0x0042A432..0x0042A43B`: moves result to `EAX` and returns `RET 0x18`.

This is the normal success cleanup tail.

### 3. Failure/no-result tail pairs the pre-toggle before returning zero

Active in YR: Yes.

All failure predicates after the loop converge on `0x0042A43E`. That tail:

- `0x0042A442..0x0042A447`: loads/tests `Pathfinder+0x3C`.
- `0x0042A449..0x0042A44C`: if nonzero, pushes the foot object and calls `0x0042ACF0`.
- `0x0042A451..0x0042A45A`: pops registers, zeroes `EAX`, and returns `RET 0x18`.

This covers open-list exhaustion, node-limit hit, depth-limit hit, null current node after the loop, and path-too-short/invalid-result cases that decompile into the shared failure tail.

### 4. `AStar_pathfind_search` does not add unpaired marker exits

Active in YR: Yes.

`AStar_pathfind_search` calls `AStar_main_loop` at `0x0042CC02`. The post-call branch either returns the result or retries after hierarchical edge update:

- `0x0042CC07..0x0042CC0D`: nonzero result jumps to return tail `0x0042CCC4`.
- `0x0042CC13..0x0042CC19`: if hierarchy is disabled after zero result, jumps to the same return tail.
- `0x0042CC79`: failed hierarchical attempt calls `UpdateHierarchicalEdges`, then `Reset`, then may rerun `Zone_precheck`.
- `0x0042CCC0..0x0042CCCB`: retry-precheck failure returns the current zero result.

Because `AStar_main_loop` owns both the pre-toggle and the cleanup tail, `AStar_pathfind_search` receives an already-cleaned result on every ordinary post-`AStar_main_loop` return.

Two `AStar_pathfind_search` early returns are before any cell A* call:

- Cross-zone hierarchy-enabled mismatch returns zero at `0x0042CB36..0x0042CB3F`.
- COM/assert-ish setup in the decompile is before `0x0042CC02`; no `0x0042ACF0` call is in that path in this slice.

Those paths do not leave `0x40000` set because they never entered `AStar_main_loop` and never called the toggler.

### 5. `UpdateBridgePassability` internal early zeroing does not skip cleanup after writes

Active in YR: Yes.

`0x0042ACF0` has an enable-byte early return: `0x0042ACF3..0x0042AD00` tests `Pathfinder+0x03` and returns through `0x0042B069..0x0042B070` if disabled. This writes no marker.

The notable internal tail is:

- `0x0042AEB1..0x0042AEB3`: tests whether any peer path marker was processed; if yes, jumps to the 5x5 phase at `0x0042AFCB`.
- `0x0042AEBD..0x0042AEC1`: if `Pathfinder+0x3C != 1`, jumps to the 5x5 phase.
- `0x0042AECA..0x0042AED5`: only when no peer path was processed and `+0x3C == 1`, writes `Pathfinder+0x3C = 0` and returns.

This path matters because later `AStar_main_loop` cleanup tails are gated by `+0x3C != 0`. Static evidence shows the zeroing path occurs only before the 5x5 write phase and only when no peer path write occurred, so skipping the later cleanup does not strand a marker.

## Integration Points

| Function / path | Role | Active in YR | Evidence |
|---|---|---|---|
| `FootClass::Run_AStar @ 0x004CBBA0` | Live caller into `AStar_pathfind_search` | Yes | decompile calls `AStar_pathfind_search` after path setup |
| `AStar_pathfind_search @ 0x0042C900` | Retry wrapper; does not directly call `0x0042ACF0` | Yes | decompile; A* call at `0x0042CC02` |
| `AStar_main_loop @ 0x00429A90` | Sole verified cleanup owner in this slice | Yes | pre-call `0x00429C1A`; success cleanup `0x0042A42D`; failure cleanup `0x0042A44C` |
| `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` | XOR toggler for temporary `0x40000` marks | Yes, conditional on `+0x03` and caller `+0x3C` | decompile and assembly contexts |
| `PathfinderClass::Reset @ 0x0042A5B0` | Clears A* internal open/closed/vector state; not a `0x40000` cleanup mechanism | Yes | decompile has no `Cell+0x140` writes |

## Current Rust Implementation Status

Read-only scan only:

| Rust area | Status vs finding | Evidence |
|---|---|---|
| `src/sim/pathfinding/core.rs` | `PathGrid` stores static walkability/bridge metadata; no persisted static `0x40000` marker was found | `rg` for `0x40000`, `40000`, `PathGrid`; `PathGrid` definition around line 1094 |
| `src/sim/pathfinding/core.rs` | Current cost shape includes static `CLIFF_COST_MULTIPLIER = 4`, code-2/5/6 entity costs, and bridge/layer logic; no RAII/scoped bridge-approach overlay surface was found | `core.rs` constants and path functions |
| `src/app_sim_tick.rs::rebuild_dynamic_path_grid` | Rebuilds `PathGrid` from terrain, bridges, buildings, walls, and overlays; should not persist `0x40000` cleanup-tail state | `src/app_sim_tick.rs:775..836` |
| `src/sim/pathfinding/zone_*` | Zone grid is persistent connectivity state; no reason to store `0x40000` here | `rg` scan |

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_main_loop` pre-toggle gate | verified | decompile `0x00429A90`; assembly `0x00429C10..0x00429C1A` | none |
| `AStar_main_loop` same-cell/same-height zero return | verified | assembly `0x00429C02..0x00429C0A -> 0x0042A451` | none |
| `AStar_main_loop` success cleanup tail | verified | decompile; assembly `0x0042A423..0x0042A43B` | none |
| `AStar_main_loop` failure cleanup tail | verified | decompile; assembly `0x0042A43E..0x0042A45A` | none |
| `AStar_pathfind_search` pre-A* cross-zone early return | verified | decompile; assembly `0x0042CB36..0x0042CB3F` | none |
| `AStar_pathfind_search` post-A* success/failure/retry exits | verified | decompile; assembly `0x0042CC02..0x0042CCCB` | none |
| `PathfinderClass::UpdateBridgePassability` enable early return | verified | assembly `0x0042ACF3..0x0042AD00`, `0x0042B069..0x0042B070` | none |
| `PathfinderClass::UpdateBridgePassability` `+0x3C=0` early tail | verified | assembly `0x0042AEB1..0x0042AED5` | none |
| Global xref census for every possible caller of `0x0042ACF0` | touched-not-exhausted | prior report says verified caller is `AStar_main_loop`; this slot lacked direct xref-list tool | run direct xref tool if exposed |
| Runtime exception/process abort cleanup | deferred | not statically provable from normal control flow | runtime/debugger or OS-level analysis only |

## Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-1 - Is the investigation mode exhaustive-slice? -> Yes, for normal static cleanup tails in the live A* chain.` (evidence: scope and decompile set `0x00429A90`, `0x0042C900`, `0x0042ACF0`)
- `[RESOLVED] OQ-2 - Does the pre-toggle happen on every A* call? -> No; it happens only for nontrivial coordinate/height mismatch and `Pathfinder+0x3C != 0`.` (evidence: `0x00429BF6..0x00429C1A`)
- `[RESOLVED] OQ-3 - Does the same-cell/same-height return need cleanup? -> No; it jumps to zero return before the pre-toggle.` (evidence: `0x00429C02..0x00429C0A -> 0x0042A451`)
- `[RESOLVED] OQ-4 - Does normal success cleanup run? -> Yes, if `Pathfinder+0x3C` remains nonzero, success tail calls `0x0042ACF0` before returning result.` (evidence: `0x0042A423..0x0042A43B`)
- `[RESOLVED] OQ-5 - Does normal failure cleanup run? -> Yes, if `Pathfinder+0x3C` remains nonzero, failure tail calls `0x0042ACF0` before returning zero.` (evidence: `0x0042A442..0x0042A45A`)
- `[RESOLVED] OQ-6 - Can `+0x3C` become zero after the pre-call and suppress tail cleanup? -> Yes, but only through `0x0042ACF0`'s no-peer/no-write `+0x3C==1` internal early tail.` (evidence: `0x0042AEB1..0x0042AED5`)
- `[RESOLVED] OQ-7 - Does `PathfinderClass::Reset` clean `0x40000`? -> No; it clears A* arrays/vector state and does not write `Cell+0x140`.` (evidence: decompile `0x0042A5B0`)
- `[RESOLVED] OQ-8 - Does `AStar_pathfind_search` directly call the toggler? -> No direct call found in decompile; it calls `AStar_main_loop` at `0x0042CC02`.` (evidence: decompile `0x0042C900`)
- `[RESOLVED] OQ-9 - Can `AStar_pathfind_search` return before A* after a path marker is set? -> No marker is set on the cross-zone early return, because it returns at `0x0042CB36..0x0042CB3F` before `AStar_main_loop @ 0x0042CC02`.` (evidence: assembly contexts)
- `[RESOLVED] OQ-10 - Do retries strand markers between attempts? -> No normal retry path receives zero from `AStar_main_loop` after that loop's failure cleanup tail; retry then calls `UpdateHierarchicalEdges` and `Reset`.` (evidence: `0x0042CC02..0x0042CC80`)
- `[RESOLVED] OQ-11 - Is this live in YR? -> Yes, live foot pathfinding reaches `FootClass::Run_AStar -> AStar_pathfind_search -> AStar_main_loop`.` (evidence: decompile `0x004CBBA0`, `0x0042C900`)
- `[RESOLVED] OQ-12 - What is the Rust-facing surface? -> Any future `0x40000` analogue belongs in per-search A* cost overlay/scope, not `PathGrid` or `ZoneGrid`.` (evidence: cleanup tails and Rust scan)
- `[DEFERRED] OQ-13 - Full global xref census for `0x0042ACF0`.` (category: requires-different-system-context; reason: this slot's Ghidra toolset exposed decompile/assembly context but no xref-list command; next-step-if-pursued: run direct xref listing and compare against `AStar_main_loop` caller claim)
- `[DEFERRED] OQ-14 - Runtime process abort/exception cleanup.` (category: needs-runtime-debugger; reason: normal static return proof cannot prove OS/process-abort restoration; next-step-if-pursued: debugger-inject interruption during active A* marker window)
- `[DEFERRED] OQ-15 - Whether `RateTimer__Current` can change between pre/post calls in pathological reentrant contexts.` (category: requires-different-system-context; reason: normal A* call is synchronous in this static slice; next-step-if-pursued: audit timer mutation/reentrancy globally)

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Normal `AStar_main_loop` exits after a pre-toggle call a matching cleanup tail before returning success or failure. | Pre `0x00429C10..0x00429C1A`; success `0x0042A423..0x0042A43B`; failure `0x0042A442..0x0042A45A`. Active in YR: Yes. | missing scoped overlay model; no persistent marker found | `src/sim/pathfinding/core.rs` future A* cost-overlay surface | Model the `0x40000` analogue as search-scoped state whose lifetime is tied to one A* attempt/call; restore/drop on every ordinary return. | Search with bridge-approach markers succeeds, then an immediate second search over the same `PathGrid` sees no leftover marker. Proposed test name: `astar_bridge_approach_overlay_clears_after_success`. | Do not persist this marker in `PathGrid`, `ZoneGrid`, bridge runtime state, or save data. |
| Normal failure/no-result paths also clean before returning zero, including node-limit/open-list exhaustion style exits. | Failure convergence `0x0042A3E8..0x0042A3FC -> 0x0042A43E`; cleanup call `0x0042A44C`; return `0x0042A45A`. Active in YR: Yes. | unchecked for future overlay failure path | `src/sim/pathfinding/core.rs`, tests around `find_layered_path`/future bridge marker overlay | Ensure overlay cleanup runs on no-path returns and max-node/depth-limit returns. | Impossible route with temporary bridge markers returns `None` and leaves the grid/overlay empty for the next search. Proposed test name: `astar_bridge_approach_overlay_clears_after_no_path`. | Do not rely on only the success path to clear temporary costs. |
| `AStar_pathfind_search` early returns before `AStar_main_loop` never set markers; retry returns consume already-cleaned `AStar_main_loop` results. | Pre-A* return `0x0042CB36..0x0042CB3F`; A* call/post-call `0x0042CC02..0x0042CCCB`. Active in YR: Yes. | none observed for static marker persistence; retry system itself has other known gaps | `src/sim/pathfinding/zone_search.rs` plus any future A* overlay wrapper | Keep marker lifetime local to the cell A* invocation, not the outer zone/retry wrapper. | Cross-zone hierarchy rejection returns before constructing any temporary bridge marker scope; retry after failed A* starts with no stale markers from the failed attempt. Proposed test name: `astar_bridge_approach_overlay_not_created_for_precheck_abort`. | Do not make retry-local zone state own or persist cell-cost markers. |
| `UpdateBridgePassability` can set `Pathfinder+0x3C=0`, but only on a no-peer/no-write path before the 5x5 marker phase. | `0x0042AEB1..0x0042AED5`; 5x5 begins at `0x0042AFCB`. Active in YR: Conditional. | no direct analogue | future marker generator | If implementing the urgency-1 no-peer path, it may skip cleanup only if no marker was actually applied. | No-peer urgency-1 search exits without creating markers and does not require restoration work. Proposed test name: `astar_bridge_approach_urgency1_no_peer_sets_no_overlay`. | Do not infer that `+0x3C==0` after a search means markers were cleaned by `Reset`; the verified reason is no markers were written in that internal path. |

## Negative Facts / Do Not Do

- Do not persist `CellClass+0x140 & 0x40000` in Rust `PathGrid` or `ZoneGrid`. Evidence: normal pre/post lifecycle in `AStar_main_loop` at `0x00429C1A`, `0x0042A42D`, and `0x0042A44C`. Active in YR: Yes.
- Do not use `PathfinderClass::Reset @ 0x0042A5B0` as a cleanup analogue for this bit. It clears A* arrays/vector state and does not write `Cell+0x140`. Active in YR: Yes.
- Do not report `AStar_pathfind_search` cross-zone early return as a leak. It returns at `0x0042CB36..0x0042CB3F` before `AStar_main_loop @ 0x0042CC02`, so no marker was set. Active in YR: Yes.
- Do not treat runtime exception/process abort cleanup as proven. It is outside normal static return flow and remains uncertainty.
- Do not conflate this with `CellClass+0x140 & 0x400`; this report only covers the temporary `0x40000` A* cost marker.

## Remaining Uncertainty

- Runtime interruptions such as access violations, debugger-forced breaks, process aborts, or nonlocal unwinding were not proven and should remain uncertainty.
- A direct global xref census for `0x0042ACF0` was not available through the exposed read-only toolset in this slot. The live A* chain is verified; broader caller uniqueness is inherited from prior report wording, not freshly enumerated here.
- Timer/reentrancy invariance between pre/post calls was not globally audited. Static normal A* control flow is synchronous, and prior work already treated the pre/post XOR lifecycle as canceling.

## Stale Docs / Follow-up Docs

No stale-doc replacement wording is required from this slice. Existing `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` should be extended conceptually from "abnormal cleanup deferred" to:

> Static normal `AStar_main_loop` cleanup tails are paired: after the pre-call at `0x00429C1A`, success calls `PathfinderClass::UpdateBridgePassability` at `0x0042A42D` before returning the path result and failure calls it at `0x0042A44C` before returning zero. `AStar_pathfind_search` early returns before `AStar_main_loop` do not set markers. Runtime exception/process-abort cleanup remains unproven.

## Sources

- Ghidra decompile: `AStar_main_loop @ 0x00429A90`
- Ghidra assembly contexts: `0x00429BF6`, `0x00429C00`, `0x00429C10`, `0x00429C1A`, `0x0042A3E8`, `0x0042A3F0`, `0x0042A3FC`, `0x0042A42D`, `0x0042A43B`, `0x0042A44C`, `0x0042A45A`
- Ghidra decompile: `AStar_pathfind_search @ 0x0042C900`
- Ghidra assembly contexts: `0x0042CB36`, `0x0042CB3F`, `0x0042CC02`, `0x0042CC07`, `0x0042CC13`, `0x0042CC79`, `0x0042CCC0`, `0x0042CCCB`
- Ghidra decompile: `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`
- Ghidra assembly contexts: `0x0042ACF3`, `0x0042AD00`, `0x0042AEB1`, `0x0042AECA`, `0x0042AED5`, `0x0042AFCB`, `0x0042B070`
- Ghidra decompile: `PathfinderClass::Reset @ 0x0042A5B0`
- Ghidra decompile: `FootClass::Run_AStar @ 0x004CBBA0`
- Existing docs read: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`
- Rust scan: `src/sim/pathfinding/core.rs`, `src/app_sim_tick.rs`, `src/sim/pathfinding/zone_*`

Status: COMPLETE for the scoped normal/static cleanup-tail question; PARTIAL for runtime-abnormal interruption proof.
