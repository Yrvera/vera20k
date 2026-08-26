# FNPC Forward-Side Bridge Projection Design

## Goal

Make `Find_Nearby_Passable_Cell` classify bridge-adjacent candidates with the exact active
`gamemd.exe` forward-side/structural-probe height correction while preserving every other
ring, gate, ordering, and selection behavior.

## Architecture Context

`src/sim/find_nearby_cell.rs` owns deterministic FNPC search. Candidate validation feeds
`collect_candidates`, which calls `is_direct_candidate` per accepted coordinate and retains the
result for collection diagnostics and early-stop. At inspected base revision `eeb2515e`, Rust
reuses that cached value for final pool partition, while native invokes `FUN_006D6410` again
there. Bridge-aware-zone queries
intentionally arm early-stop without the collection projection result while final selection still
re-runs projection.

`ResolvedTerrainCell.bridge_facts.raw_flags` already owns the modeled live
`CellClass+0x140` bits. `ResolvedTerrainGrid::cellclass_bridge_flags_0x1180` is the existing
real-cell/shared-dummy lookup seam; `src/map/bridge_facts.rs` names `0x1000` as
`BRIDGE_FLAG_FORWARD_SIDE` and `0x100` as `BRIDGE_FLAG_STRUCTURAL`, and active
`SetBridgeDirection` stamping preserves both. No new state or writer is needed.

Native `FUN_006D6410 @ 0x006D6410` reads the candidate flag at
`0x006D6473..0x006D6486`; only when `0x1000` is set does it read each projected probe's
`0x100` bit and add four to that probe's signed height delta at
`0x006D64F8..0x006D6513`. The naval FNPC query reaches both the collection and final-pool
uses of this helper. [docs/research/PHASE3_AI_BASE_PLACEMENT_VECTOR_SELECTOR_005060B0_GHIDRA_REPORT.md
§2.2, §9 Handoff B; docs/research/PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md
§8.2, §11.1 item 13]

## Impact Analysis

- Change only `src/sim/find_nearby_cell.rs`: read candidate/probe flags inside
  `is_direct_candidate`, call that corrected classifier again during final pool partition,
  replace its stale UNCHECKED residual comment, and add focused tests.
- Reuse `ResolvedTerrainGrid::cellclass_bridge_flags_0x1180`; do not change bridge facts,
  `PathGrid`, snapshots, hashing, callers, or function signatures.
- Blast radius is every FNPC caller because the helper is shared, but observable change is
  gated by the exact candidate-`0x1000` plus projected-probe-`0x100` combination. Nonbridge
  terrain and structural probes without candidate forward-side remain bit-for-bit unchanged.
- The search consumes no RNG and changes no caller-owned state. Its CellClass flag lookups retain
  native shared-dummy coordinate side effects through the existing accessor. Candidate order,
  ring termination, frame-modulo indexing, and same-tick determinism remain owned by existing code.

## Chosen Approach

Read the candidate's modeled raw bridge flags once before the six-step projection loop. For
each probe, add `BRIDGE_LEVEL_RISE` (`4`) to its signed level only when the cached candidate
has `BRIDGE_FLAG_FORWARD_SIDE` and that probe has `BRIDGE_FLAG_STRUCTURAL`; then feed the
adjusted probe level into the existing relative-rise threshold unchanged. Invoke the same
classifier during collection and again, in stored candidate order, when partitioning the final
direct and indirect pools.

This is the smallest Rust-native translation: it changes the classifier at the exact point
native changes its height delta, uses the existing CellClass flag authority, and restores the
native two-site projection call pattern without changing candidate or selection ordering.

## Player-Experience Detail Ledger

- **MILESTONE-BLOCKING — exact two-bit gate:** probe `0x100` is ignored unless candidate
  `0x1000` is set; otherwise intact bridges can change direct rings and selected cells.
  [Ghidra `0x006D6473..0x006D6516`]
- **MILESTONE-BLOCKING — exact magnitude/order:** add four signed height levels to the probe
  delta before the existing isometric projection comparison, not to the candidate/base height
  and not after classification. [Ghidra `0x006D64F8..0x006D651D`]
- **COMPOUNDING — both semantic uses:** the classifier runs during ordinary collection
  early-stop and again during final direct/indirect pool partition, which can change pool length
  and `frame % pool_len` output. [selector report §2.2; exhaustive report §8.2]
- **COMPOUNDING — preserve bridge-aware asymmetry:** `bridge_aware_zone` still arms early-stop
  on any accepted candidate, while final selection uses projected classification. This slice
  changes only the projection result. [current `collect_candidates`; FNPC caller evidence]
- **COMPOUNDING — flag authority:** use modeled live CellClass flags through the existing
  resolved-terrain/shared-dummy seam; do not infer forward-side from path passability or add a
  writer. [current `bridge_facts.rs`, `resolved_terrain.rs`; exhaustive report §8.2]
- **MILESTONE-BLOCKING — determinism:** retain ring order, duplicate radius-zero seed,
  24-candidate cap, frame-modulo selection, and zero Scenario RNG. [current FNPC tests;
  selector report §2.2]
- **No exactification residual:** all behavior in this narrow correction is verified and its
  required Rust inputs already exist. Naval selection and ordinary base placement remain
  separate mechanisms, not approximations inside this slice.

## Design

### Components

- `is_direct_candidate`: add the exact flag-gated probe-level correction and canonical
  `gamemd-derived` provenance for `FUN_006D6410 @ 0x006D6410`.
- Final pool construction: invoke `is_direct_candidate` once per stored candidate in order,
  matching native's final partition use rather than reusing collection-time classification.
- Test fixtures in the same leaf module: stamp raw flags directly on resolved terrain so each
  load-bearing branch is executable without adding production-only scaffolding.

### Interfaces / Contracts

No public API changes. Missing resolved terrain continues to expose no modeled bridge flags,
matching existing compatibility behavior. The exact active runtime supplies resolved terrain.

### Data Flow

`ResolvedTerrainGrid bridge flags -> collection is_direct_candidate -> per-ring early-stop;
stored candidate order -> final is_direct_candidate -> preferred pool -> existing target/frame
choice`.

### Error Handling

No new failure state. Off-grid/unallocated flag reads use the existing CellClass dummy accessor;
existing level lookup and candidate conversion behavior are unchanged.

### Testing Strategy

Focused executable tests will prove:

1. a forward-side candidate plus structural probe contributes exactly four levels at a known
   projection threshold;
2. the same structural probe is ignored without candidate forward-side;
3. the correction turns the duplicated radius-zero seed indirect, extends collection through
   ring one, and changes the final frame-modulo direct pool in exact ring order;
4. nonbridge candidate classifications, collection, and frame-selected outputs remain unchanged;
5. the complete existing `sim::find_nearby_cell::tests::` module still passes.

## Architectural Decisions

- Keep one classifier implementation, but call it at native's collection and final-partition
  sites; do not duplicate its logic or reuse stale classification across those sites.
- Read CellClass flags from resolved terrain, not `PathGrid`: forward-side is a raw CellClass
  flag, while path passability exposes no equivalent authority.
- Add no persistent state, scheduling, RNG, or cross-layer dependency.
- Tech debt introduced: none.

## Alternatives Considered

1. **Recommended: adjust probe level inside `is_direct_candidate` and invoke it at both native
   sites.** Exact native location and call pattern, one implementation, and existing architecture
   stays intact.
2. **Fold the correction into generic `cell_level`.** Rejected because it would affect FNPC's
   separate seed-height gate and every caller, while native gates it only inside projection.
3. **Keep the current cached final classification.** Rejected: native re-runs the helper, and the
   authoritative CellClass flag accessor retains shared-dummy lookup side effects, so equivalence
   is no longer proven even though ordinary in-grid classification values are identical.

## Autonomous Approval Record

**Review verdict: APPROVE.** Initial review found one P1 issue: the first draft treated cached
collection classification as sufficient for final partition, but the live body/callsites prove a
second native invocation and the raw-flag accessor carries CellClass dummy lookup semantics. The
design now calls one corrected classifier at both sites.

Why approve: every behavior claim is grounded in the two cited exhaustive reports, current Rust,
and a cold decompile/assembly read of `0x006D6410`; all blocking details have executable tests; no
new state, RNG, scheduler, or cross-layer dependency is introduced.

What could still make ordinary skirmish wrong: changing ring order, the bridge-aware early-stop
asymmetry, direct-pool preference, or frame-modulo ordering. The design explicitly leaves those
owners unchanged and reruns the full existing FNPC module tests.

What could create expensive later rework: inventing a second bridge-flag authority or folding
the correction into generic height lookup. Both are rejected; the design consumes the existing
CellClass raw-flag seam at the native projection boundary. No open question or residual remains
for this narrow mechanism. Autonomous approval is recorded under the user's explicit authority.
