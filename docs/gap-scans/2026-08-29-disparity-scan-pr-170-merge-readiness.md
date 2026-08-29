# PR #170 Merge-Readiness Disparity Scan

Date: 2026-08-29
Scope: PR #170 (`feature/bridge-movement-parity`) only
Compared revisions: `origin/main@0a6e6742` and branch implementation through
`b4faa65c`
Verdict: **MERGE_READY**

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
- Active-retail Object reveal/unlimbo at `0x005F4EC0`, Unit cell admission at
  `0x0073F0A0`, and the Factory production paths at `0x004C9C70`, `0x004CA0E0`,
  `0x004CA5A0`, and `0x004CA1A0`.
- Active-retail paradrop spawner `FUN_0065E660`, edge helper `FUN_004AA440`, and
  criterion-4 outside-playfield acceptance at `0x004AAB3D..0x004AAB4B`, with the
  call-order evidence in `PDPLANE_SPAWNER_EDGE_SILENT_PATH_GHIDRA_REPORT.md` and
  `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`.
- Direct branch/source comparison against `origin/main`, serial reproduction of all owning
  modules represented in the failure list, and focused reruns after each bounded correction.

The research-index preflight could not open its SQLite database, so this scan used the cited
primary reports and direct source inspection rather than index-derived claims.

## Failure classification

The reported 163 failures occurred in two branch-wide stages. The first eight were corrected in
commits `c012136e` and `440a4900`. A subsequent exact full-library run at `440a4900` exposed 155
additional failures. All 152 simulation failures reproduced serially, disproving the initial
parallel-state/interner hypothesis. Their exact partition, plus the three non-simulation
failures, is below. No default lifecycle, production gate, or test scheduler was weakened.

| Classification | Count | Current evidence | Required action |
|---|---:|---|---|
| Stale constructor/placement RNG expectations | 4 | `origin/main` retained the old placement-first assumption; active retail draws in the Techno constructor first. | Closed in `c012136e`; production order preserved. |
| Generator/simulation ownership violation | 1 | Simulation consumed RMG-owned construction-trace DTOs. | Closed in `440a4900` by moving the DTOs to neutral map ownership while retaining the dependency guard. |
| Evidence-backed deterministic baseline drift | 3 | Only the verified constructor draw changed the affected Scenario stream/hash baselines. | Closed in `c012136e`; record/replay and stage tripwires retained. |
| Active-object fixture lifecycle | 93 | Branch-added native `!in_limbo` gates correctly rejected test objects left at `ObjectLifecycle::default()`. | Fixtures now explicitly model revealed/live objects; defaults and production gates are unchanged. |
| Placed-structure fixture marking | 5 | Five of the live structure fixtures also omitted the native placed-cell mark. | Fixtures now set both `in_limbo = false` and `cell_marked = true`. |
| Unit-Unlimbo admission authority | 48 | Fixtures invoked the real constructor/Unlimbo route without valid playfield bounds, terrain speed rows, bridge facts, or an admitted cell. | Fixtures now provide the smallest real map/terrain authority required by `0x0073F0A0`; no admission bypass was added. |
| Paradrop carrier spawn integration | 6 | The old shared rectangular fog-array edge contradicted `FUN_004AA440`, per-carrier call order, and the isometric playfield gate. | Production now uses the exact per-carrier native spawn-edge helper, outside-playfield criterion-4 admission, and verified constructor/helper/Unlimbo RNG order. Stock Open/Rescue mission behavior remains outside this branch-failure repair. |
| Stale deterministic RMG witness | 2 | Seed 4242 stopped witnessing the asserted condition after the verified water-family correction. | The tests now select from fixed `MATRIX_SEEDS` and preserve the same deterministic property. |
| Radar same-cell Unit stacking | 1 | The fixture depended on two Units occupying one admitted cell, which exact Unit CanEnter rejects. | The fixture separates the Units and marks the adjacent observed cell; lifecycle behavior remains under test. |

### Original eight-test inventory

1. `all_eight_nearoref_mcvs_spawn_on_authored_waypoints_without_fallback_rng`
2. `nearoref_blocked_start_fallback_uses_full_cell_array_clamp`
3. `skirmish_mcv_start_uses_radius_fallback_when_start_cell_blocked`
4. `skirmish_start_unit_blocked_placement_stops_after_twenty_failures`
5. `map::rmg::emit::tests::sim_does_not_reference_the_generator`
6. `sim::world::slice6_retask_tests::replay_hash_stable_through_slice6`
7. `sim::world::bridge_parity_harness_tests::bridge_parity_harness_is_deterministic`
8. `sim::world::global_parity_harness_tests::global_parity_harness_is_deterministic`

### Closure validation before final certification

- `cargo test -p vera20k --lib paradrop`: 27 passed, 0 failed.
- `cargo test -p vera20k --lib 'sim::'`: 3,802 passed, 0 failed, 51 ignored.
- `cargo test -p vera20k --lib 'map::rmg::build::tests::'`: 17 passed, 0 failed.
- `cargo test -p vera20k --lib radar_visibility_consumes_live_stock_cloak_and_sensor_lifecycle`:
  1 passed, 0 failed.

Final certification:

- `cargo test -p vera20k --lib`: 7,577 passed, 0 failed, 75 ignored.

## Verified gaps

### U2-20 — waterfall terrain no-topology boundary

**Status:** closed in `85ddabba`, evidence-backed.

`src/map/rmg/phases/bridge.rs::build` owns the Rust `BuildRiverBridge @ 0x0059E740`
correspondence. Before closure, tests either called the lower-level stamper or swept river seeds;
none pinned a deterministic successful direct `build` call and its complete negative boundary.
The direct characterization also exposed a material write-set omission: both the waterfall block
stamper and the shore block stamper copied tile/subtile/level but omitted the TMP subtile slope
that the native tile-block callee writes.

Implemented closure:

- Use one fixed successful seed and call `build` directly.
- Prove the allowed tile/subtile/slope/level/scratch terrain change and waterfall appearance.
- Prove overlay, density/data, occupancy, start markers, emitted overlay/data packs, structures,
  explicit tubes, and construction trace remain unchanged.
- Materialize `ResolvedTerrainGrid` and prove zero modeled raw bridge flags/no low deck.
- Repeat from identical inputs and prove identical output and MapGen continuation.
- Restore the two missing `cell.slope = sub.slope` writes; do not change any topology field.

No other production change is indicated by current native/source comparison.

### U2-21 — active/dormant naming boundary

**Status:** closed in `85ddabba`, documentation-only.

Comments and test names now distinguish waterfall/river terrain `0x0059E740` from the active
low-deck branch `0x0058F2C0`, while retaining established Rust identifiers where renaming would
create churn. No excluded `TrainBridgeSet` surface was introduced.

## Ranked fix list

| Rank | Fix | Player visibility / trigger frequency | Evidence status |
|---:|---|---|---|
| 1 | Restore TMP slope writes in the two waterfall/shore stampers | Generated river/waterfall maps whenever non-flat TMP subtiles are selected | Closed; verified `0x0059E740` callee write census |
| 2 | Correct constructor-before-placement test expectations | Launches with starting units; ordinary skirmish path | Closed; verified active-retail call order |
| 3 | Rebaseline affected deterministic hashes/Scenario stream | Test certification only, but protects all replayed bridge/launch paths | Closed; verified draw plus passing tripwires |
| 4 | Move construction-trace DTO to neutral map ownership | Architectural; every traced generated-map bootstrap | Closed; dependency guard retained |
| 5 | Add U2-20 negative characterization | Generated river/waterfall maps; guards against future topology pollution | Closed; verified `0x0059E740` write exclusions |
| 6 | Correct U2-21 terminology | Source/reviewer correctness | Closed; verified active/dormant boundary |

## Not gaps

- The constructor Scenario draw is not a regression to remove. The branch behavior matches
  `0x006F3254`; the four launch tests retain pre-fix expected stepping.
- `BuildRiverBridge @ 0x0059E740` must not gain low-overlay, raw-flag, Tube, CABHUT, or trace
  writes to satisfy its name.
- The 155 currently passing tests are not candidates for ignores or bulk baseline edits.
- OpenTS correspondences are navigation leads only and establish none of the conclusions above.

## Implementation boundary

All classified corrections and U2 closures are implemented. The final fresh read-only critic
passed, the bounded working tree was committed, and the exact full-library certification is
green. PR #170 can leave draft state for review; this report does not authorize merging it.

## Ghidra annotation candidates

None. This scan relies on already recorded active-retail evidence and found no new symbol,
prototype, or field annotation requiring synchronization.
