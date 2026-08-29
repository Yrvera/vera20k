# PR #170 Merge-Readiness Disparity Scan

Date: 2026-08-29
Scope: PR #170 (`feature/bridge-movement-parity`) only
Compared revisions: `origin/main@0a6e6742` and branch `d2b7d4ac`
Verdict: **FIX_REQUIRED**

## Question

Classify the 163 failures reported by the previous branch-wide `cargo test -p vera20k --lib`
run, close U2-20 and U2-21, and identify the smallest evidence-backed changes needed before
one final branch-wide certification run. This scan does not authorize unrelated parity work or
production rollback.

## Evidence consulted

- Active-retail `TechnoClass` constructor at `0x006F2B90`, especially the raw Scenario draw
  and persistent low-word store at `0x006F3254`.
- Active-retail starting-unit call sites `0x005D7030` and `0x005D70F0`: construct once before
  exact/fallback placement attempts; failure does not refund the constructor draw.
- Active-retail `BuildRiverBridge` at `0x0059E740`: waterfall/river terrain stamping only.
  The verified write set is tile, subtile, slope, level, and scratch state. It does not write
  overlay `Cell+0x44`, data/density `Cell+0x11E`, raw bridge flags, Tube topology, CABHUTs,
  or a construction trace.
- Active-retail low-deck owner `0x0058F2C0` and the conditional Create Random Map/`.SED`
  activation boundary. `TrainBridgeSet` and helpers `0x005A5020`, `0x005A6510`,
  `0x005A82E0`, `0x005A91E0`, and `0x005A1E10` remain excluded as dormant/TS-only.
- `docs/research/bridges/00-system-models/RMG_LOW_BRIDGE_DECK_CABHUT_ACTIVE_RETAIL_CLOSURE_GHIDRA_REPORT.md`
- `docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/contracts/2026-08-28-techno-constructor-scenario-rng-implementation-contract.md`
- `docs/contracts/2026-08-29-rmg-low-bridge-launch-construction-implementation-contract.md`
- Direct branch/source comparison against `origin/main`, plus complete focused reruns of every
  owning module represented in the previous failure list.

The research-index preflight could not open its SQLite database, so this scan used the cited
primary reports and direct source inspection rather than index-derived claims.

## Failure classification

The prior branch-wide run reported 163 failures. At the unchanged branch head, complete focused
reruns of the owning modules reduce that set to eight reproducible failures. The remaining 155
are not reproducible at this head and require no expectation suppression or source change; the
final full-library run is the recurrence check.

| Classification | Count | Current evidence | Required action |
|---|---:|---|---|
| Stale constructor/placement RNG expectations | 4 | All four reproduce in `app::frontend::skirmish::tests`; `origin/main` uses the old placement-first assumption, while the branch now implements the verified constructor-first Scenario draw. | Re-step expected Scenario RNG once per constructed Techno before fallback placement draws. Preserve production order. |
| Generator/simulation ownership violation | 1 | `map::rmg::emit::tests::sim_does_not_reference_the_generator` names `src/sim/scenario_bootstrap.rs`, which consumes RMG-owned construction-trace DTOs. | Move the shared trace DTO to neutral map ownership; keep the dependency guard. |
| Evidence-backed deterministic baseline drift | 3 | Slice-6, bridge-parity, and global-parity tripwires pass until their stored hashes/stream state. Only Scenario state changes in the global stream tuple; that is the stream used by the verified constructor draw. | Rebaseline only the affected constants, after preserving record/replay and per-stage tripwires. |
| Non-reproducible prior failures | 155 | Complete owning-module reruns pass at the same source head. No branch-added mutable global, environment mutation, or test suppressor was found. | No code change. Require the final full-library run to prove non-recurrence. |

### Reproducible test inventory

1. `all_eight_nearoref_mcvs_spawn_on_authored_waypoints_without_fallback_rng`
2. `nearoref_blocked_start_fallback_uses_full_cell_array_clamp`
3. `skirmish_mcv_start_uses_radius_fallback_when_start_cell_blocked`
4. `skirmish_start_unit_blocked_placement_stops_after_twenty_failures`
5. `map::rmg::emit::tests::sim_does_not_reference_the_generator`
6. `sim::world::slice6_retask_tests::replay_hash_stable_through_slice6`
7. `sim::world::bridge_parity_harness_tests::bridge_parity_harness_is_deterministic`
8. `sim::world::global_parity_harness_tests::global_parity_harness_is_deterministic`

### Non-reproducible clusters

Complete focused reruns passed for RMG build, radar lifecycle, AI, combat, turret facing,
miner and outbound drive, movement occupancy, pathfinding zone search, power, production queue,
radar, rocking, sensor lifecycle, spawn manager, lightning storm, paradrop, vision, production
shadow, cloak lifecycle, and the general world tests. The RMG build failures from the prior run
also do not reproduce. These results classify the 155 tests as transient-at-that-run, not as
permission to delete, ignore, weaken, or rebaseline them.

## Verified gaps

### U2-20 — waterfall terrain no-topology boundary

**Status:** open, test-only, compounding.

`src/map/rmg/phases/bridge.rs::build` owns the Rust `BuildRiverBridge @ 0x0059E740`
correspondence. Existing tests either call the lower-level deck stamper or sweep river seeds;
none pins a deterministic successful direct `build` call and its complete negative boundary.
The direct characterization also exposes a material write-set omission: both the waterfall block
stamper and the shore block stamper copy tile/subtile/level but omit the TMP subtile slope that
the native tile-block callee writes.

Required closure:

- Use one fixed successful seed and call `build` directly.
- Prove the allowed tile/subtile/slope/level/scratch terrain change and waterfall appearance.
- Prove overlay, density/data, occupancy, start markers, emitted overlay/data packs, structures,
  explicit tubes, and construction trace remain unchanged.
- Materialize `ResolvedTerrainGrid` and prove zero modeled raw bridge flags/no low deck.
- Repeat from identical inputs and prove identical output and MapGen continuation.
- Restore the two missing `cell.slope = sub.slope` writes; do not change any topology field.

No other production change is indicated by current native/source comparison.

### U2-21 — active/dormant naming boundary

**Status:** open, documentation-only, compounding.

Comments and test names across `src/map/rmg/build.rs`, `pipeline.rs`, `phases/water.rs`,
`island_passes.rs`, `adjacency.rs`, `lake.rs`, `meander.rs`, `river.rs`, `bridge.rs`, and
`water_finalize.rs` still conflate the conditionally active RMG path with dormant behavior or
describe waterfall terrain construction as an active low bridge. The smallest correction is
wording/test-name repair: distinguish waterfall/river terrain `0x0059E740` from the active
low-deck branch `0x0058F2C0`, while retaining established Rust identifiers where renaming would
create churn. No excluded `TrainBridgeSet` surface should be introduced.

## Ranked fix list

| Rank | Fix | Player visibility / trigger frequency | Evidence status |
|---:|---|---|---|
| 1 | Restore TMP slope writes in the two waterfall/shore stampers | Generated river/waterfall maps whenever non-flat TMP subtiles are selected | Verified `0x0059E740` callee write census |
| 2 | Correct constructor-before-placement test expectations | Launches with starting units; ordinary skirmish path | Verified active-retail call order |
| 3 | Rebaseline affected deterministic hashes/Scenario stream | Test certification only, but protects all replayed bridge/launch paths | Verified draw plus passing tripwires |
| 4 | Move construction-trace DTO to neutral map ownership | Architectural; every traced generated-map bootstrap | Direct dependency-guard failure |
| 5 | Add U2-20 negative characterization | Generated river/waterfall maps; guards against future topology pollution | Verified `0x0059E740` write exclusions |
| 6 | Correct U2-21 terminology | Source/reviewer correctness | Verified active/dormant boundary |

## Not gaps

- The constructor Scenario draw is not a regression to remove. The branch behavior matches
  `0x006F3254`; the four launch tests retain pre-fix expected stepping.
- `BuildRiverBridge @ 0x0059E740` must not gain low-overlay, raw-flag, Tube, CABHUT, or trace
  writes to satisfy its name.
- The 155 currently passing tests are not candidates for ignores or bulk baseline edits.
- OpenTS correspondences are navigation leads only and establish none of the conclusions above.

## Implementation boundary

Make only the six ranked fixes, validate each focused owner, obtain fresh read-only criticism,
then run `cargo test -p vera20k --lib` exactly once as final certification. Keep PR #170 draft
unless that command passes. Do not merge.

## Ghidra annotation candidates

None. This scan relies on already recorded active-retail evidence and found no new symbol,
prototype, or field annotation requiring synchronization.
