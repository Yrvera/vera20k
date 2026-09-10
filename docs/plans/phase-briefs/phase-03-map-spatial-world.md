# Phase 3 brief — Map and spatial world

Rows 37–52 of the [implementation order](../2026-07-30-clean-slate-system-implementation-order.md).
State: **IN PROGRESS; every row remains open**. Baseline: fetched `origin/main`
`8f988ab256b3b38c30bdd3db67fee215c54e25ac`, inspected 2026-09-10.
This is the current investigation frontier, not a completed mechanism census.

## Rows

Registry values below are native evidence / Rust implementation / parity from
[registry.v2.json](../../system-map/registry.v2.json). They are inherited status,
not newly established equivalence. Paths are current representative owners;
references are starting evidence whose applicability must be checked per mechanism.

| Row | GSI | Rust owner(s) | Native anchor or research starting point | Registry | Notes |
|---|---|---|---|---|---|
| 37 | GSI-04.01 | `src/map/cell_index.rs`, `playfield.rs`, `resolved_terrain.rs`; `src/sim/cell_rect.rs` | Get_CellClass `0x005657A0`, world lookup `0x00565730`, IsCellInPlayfield `0x00578460`; [dummy contract](../../research/MAPCLASS_GET_CELLCLASS_FALLBACK_DUMMY_CELL_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Current native comparison exposed reversed ClipRect operands under signed overflow; its repair is the first increment. Constructor/iterator and unmodeled-state candidates require separate proof. |
| 38 | GSI-04.02 | `src/map/theater.rs`, `resolved_terrain.rs` | CalculateLegacyMapTileIndex `0x00544E30`; [translation](../../research/PHASE3_LAST_TILES_IN_SET_COMPATIBILITY_TRANSLATION_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Load, Fill, map-pack and generated-map paths must be checked separately. |
| 39 | GSI-04.03 | `src/util/lepton.rs`; `src/sim/cell_kernel.rs`; `src/map/resolved_terrain.rs` | ComputeGroundHeightAtCoord `0x0047B3A0`; [domain census](../../research/PHASE3_CELL_GROUND_HEIGHT_104_DOMAIN_CONSUMER_CENSUS_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | The CellClass 90-lepton and object/VXL 104-lepton domains are distinct. |
| 40 | GSI-04.04 | `src/sim/cell_kernel.rs`, `overlay_grid.rs`, `pathfinding/terrain_cost.rs` | [RecalcZoneType](../../research/CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Preserve synchronous publication before the next reader. |
| 41 | GSI-04.06 | `src/sim/pathfinding/zone_map.rs`, `zone_build.rs`; `src/sim/world/navigation.rs` | FloodFillReachableZones `0x005840C0`; [flood fill](../../research/MAPCLASS_FLOODFILLREACHABLEZONES_005840C0_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Tube hierarchy registration is explicitly unimplemented in current source; verify its active callers before choosing the repair. |
| 42 | GSI-04.05 | `src/sim/occupancy.rs`, `cell_rect.rs`; `src/sim/world/lifecycle.rs` | AddContent `0x0047E8A0`, RemoveContent `0x0047EA90`; [live list writers](../../research/CELLCLASS_SUBSTRATE_LIVE_OBJECT_LIST_WRITERS_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Cell lists, layer transitions and reservations are distinct authorities. Do not import all AI behavior merely because it reads occupancy. |
| 43 | GSI-04.07 | `src/map/authored_overlay.rs`; `src/sim/overlay_grid.rs` | OverlayClass::Mark `0x005FC570`, DestroyOverlay `0x00480CB0`; [authored boundary](../../research/bridges/01-assets-map-load-overlay/AUTHORED_OVERLAYPACK_INLINE_TRANSACTION_REINVESTIGATION_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Includes mutation order, ownership, shared dummy effects and projection to consumers. |
| 44 | GSI-04.09 | `src/sim/tiberium/mod.rs`, `overlay_grid.rs`; `src/map/authored_overlay.rs` | Reduce_Tiberium `0x00480A80`; [quantity mutation](../../research/CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Identity and quantity are this row; growth/harvesting policy enters only as a proved prerequisite or consumer. |
| 45 | GSI-04.10 | `src/sim/terrain_object.rs`, `terrain_spawn.rs` | [TerrainClass timing](../../research/TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Verify ordinary trees/rocks as well as TIBTRE; spawning is not full destruction/fire coverage. |
| 46 | GSI-04.12 | `src/sim/bridge_state/mod.rs`, `map/bridge_topology.rs`, `world/bridge_orchestrator.rs` | [bridge coverage](../../research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Existing bridge work is substantial but explicitly incomplete. |
| 47 | GSI-04.13 | `src/map/bridge_facts.rs`; `src/sim/movement/movement_bridge.rs` | [bridge coverage](../../research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Distinguish high bridge decks, wood bridges and low bridge tubes. |
| 48 | GSI-04.15 | `src/map/tubes.rs`, `tube_facts.rs`; `src/sim/movement/tube_movement.rs` | ReadTubesINI `0x007283C0`; [bridge coverage](../../research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / DRIFT | Active low-bridge tubes cannot be excluded as TS-only based on their name. |
| 49 | GSI-04.16 | `src/map/waypoints.rs`; `src/map/map_file.rs` | Read_Waypoints `0x0068BDC0`; [map substrate](../../research/CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md) | CONTRACTED / PARTIAL / DRIFT | Signed values and canonical key recovery already landed; audit starts/regions and downstream use separately. |
| 50 | GSI-04.18 | `src/sim/vision/mod.rs`; `src/sim/snapshot.rs` | [shroud reveal](../../research/SHROUD_REVEAL_SYSTEM_GHIDRA_REPORT.md) | ANCHORED / PARTIAL / UNCHECKED | Persisted knowledge, reveal counters and present visibility are distinct. Historical SHROUD_DISPARITIES is not a current gap list. |
| 51 | GSI-04.11 | `src/sim/smudge_grid.rs`, `combat/smudge_dispatch.rs` | [smudge class](../../research/SMUDGE_CLASS_GHIDRA_REPORT.md), [spawn callers](../../research/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md) | CONTRACTED / PARTIAL / UNCHECKED | Placement, caller-specific ore mutation, RNG order and restore require evidence. |
| 52 | GSI-04.20 | `src/map/lighting.rs`; `src/app/presentation/lighting.rs` | ApplyAreaLightConvert `0x00554AF0`; [light source lifecycle](../../research/LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md) | N/A / N/A / N/A (group) | Group status is not exclusion or closure. Separate active ambience, lamp lifecycle and global tint from dormant TS behavior. |

## Loops

The current [topology](../../system-map/topology.v2.json) records representative
connections, not an exhaustive Phase 3 loop inventory:

- `LOOP-005-BUILD-PLACE`: cell passability and occupancy, stages 10–11.
- `LOOP-006-FACTORY-EXIT`: exit occupancy, stage 7.
- `LOOP-008-REVEAL-RADAR`: persisted per-cell knowledge, stage 5.

Trace unlisted map-load, overlay, terrain, bridge and lighting transactions from
their actual entrypoints. A downstream reader alone does not make its entire
owning system a Phase 3 prerequisite.

## Prior work

The 2026-09-10 increment at `258a59ce` repairs LocalSize clipping operand order
and adds original-executable spatial comparisons. Its [evidence report](../../research/PHASE3_MAP_SPATIAL_NATIVE_COMPARISON_20260910.md)
records the failing pre-fix result, corrected production path, independent
review, four passing focused tests, full library results (8,587 passed, zero
failed, 119 ignored), and Clippy exit zero with 1,144 warnings. This is bounded
mechanism evidence; it does not close a row.

The archived **Phase 3 integration goal** task
`01a0529b-815e-7e31-be6d-90511e997891` reports recovery through PRs #166/#167 and
#173–#193 at `5062bcea`. That completed recovery of eligible slices, not Phase 3.
Current source has subsequently changed; use Git and the actual consumers.

[PR #172](https://github.com/Yrvera/vera20k/pull/172) remains an open, conflicting
mixed-history archive at `49f7707f`. Recover individual evidence and coherent
changes only. Its rejected animation shadow/layer and lowercase atlas work,
partial Railgun/AI/trigger/crate/capture work, and Explodes/Temporal research are
not preapproved implementations or automatically required Phase 3 scope.

Current bridge work and remaining transactions are described in the
[bridge plan](../2026-08-28-active-retail-bridge-parity-design.md). The plan's
status is historical; reconcile it against current source before selecting work.

## Corrections

- The new original-instruction corpus exposed reversed ClipRect operands in
  `src/map/playfield.rs`: native clips candidate Size against LocalSize. Ordinary
  intersection symmetry hides the difference; accepted signed-overflow input
  does not. The pre-fix Rust comparison failed, and the first increment repairs
  the production order. See the [comparison report](../../research/PHASE3_MAP_SPATIAL_NATIVE_COMPARISON_20260910.md).

- Old GSI-04.01 gaps G1 dummy reservation reset and G2 IsoMapPack miss stamping
  have production implementations. The old reservation-writer candidate is also
  stale: current lifecycle code implements the Mark/Clear perimeter paths.
- The old copied-dummy target-Z claim is a [documented false positive](../../gap-scans/2026-08-25-disparity-scan-gsi-04-01-dummy-cell-target-z.md).
- CellClass constructor order and anti-diagonal Fill/RNG order are different;
  never fix a hypothetical identity gap by reordering Fill.
- The old multiplayer Resize-prefix hypothesis is partly stale. Current
  `src/sim/native_identity.rs::build_noncampaign_fresh_id_prefix` accounts for
  both Cell/dummy constructor generations, and `scenario_bootstrap` tests their
  identity checkpoints. This does not establish individual Cell identity
  consumers, preview reuse or save/restore equivalence. See the current
  [prefix investigation](../../research/bridges/01-assets-map-load-overlay/FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md).
- Generic storage iteration is not a native-order contract. Authored overlay
  recalculation already has `NativeOverlayMapShape::recalc_cells`, and other
  owners explicitly order their sweeps. Audit each active consumer before
  changing iteration shared by unrelated systems.
- TS-only exclusions require active-binary/data evidence. Neither inherited TS
  code nor a generic registry group is enough to establish applicability.

## Inherited residuals

`src/sim/pathfinding/zone_build.rs::tube_hierarchy_pairs_are_unregistered` records
an ignored test for the tube arm of `0x00582D70`. Its hypothesized trigger is a
long route across a low bridge; the consequence is a detour or no hierarchy route.
Retail incidence and the active caller chain need rechecking; the source marker
is an investigation lead, not a fresh native finding.

The bridge plan retains unresolved construction, restamp, topology and consumer
transactions. Current shroud, smudge, terrain and lighting code has not received
this goal's exhaustive reverse audit. These rows remain open regardless of the
outcome of the first map-lookup comparison.

## Coverage

No phase-wide native differential or completed reverse audit is recorded by this
goal. Existing Rust regression tests and older scoped critic passes do not imply
whole-row equivalence. The [first comparison increment](../../research/PHASE3_MAP_SPATIAL_NATIVE_COMPARISON_20260910.md)
records 2,144 original-executable calls/prefixes for lookup, playfield predicates,
normalization and retained dummy identity. Its report records the executable,
fixtures, endpoint limits, production test consumers and passing validation.
This bounded corpus does not close GSI-04.01. Ignored library tests remain
unexecuted; they are not passing parity evidence.

## Open queue

1. The current iterator investigation has selected bridge endpoint production
   (`ComputeBridgeZones @ 0x0056D6E0`) for repair: one native ordered sweep
   interleaves high-bridge and TubeClass records, and a strict ordinal test
   excludes same-cell automatic tube shells. Current Rust synthesized spans
   for those shells. Native callers include map initialization and runtime
   bridge-zone invalidation/validation. Validation and independent review are
   pending; this does not close the separate hierarchy consumer or all iterator
   consumers.
2. Recheck the remaining constructor identity, shared-dummy field,
   Resize/restore, iterator and consumer-order hypotheses. Implement only proven
   observable differences; retain unresolved candidates explicitly.
3. Reconcile each other row with current production source and active retail
   evidence. Reuse bridge and earlier Phase 3 research without importing their
   historical scope expansions automatically.
4. Run the phase-wide reverse audit only after every in-scope mechanism and
   evidence-backed exclusion is accounted for. Any omission reopens its row.

## Start here

Read the current task checkpoint and verify Git/process state, including whether
the validated clipping increment has merged. Verify refreshed `origin/main`
before selecting the next open mechanism. Each mechanism needs independent
review, the required library tests and Clippy, then PR publication, merge and
verification before another mechanism starts. Preserve a blocked mechanism and
continue independent work when necessary. No phase or row is closed by this brief.
