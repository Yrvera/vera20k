# Phase 3: ordered bridge records and their base-zone consumer

Date: 2026-09-10. Branch started from refreshed main `1494261f` after PR321.
Native program: active retail `gamemd.exe`, x86 image base `0x400000`, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Read-only Ghidra `testProsjekt` and original-byte Unicorn comparisons below.

## Outcome and scope

The C6 iterator-consumer hypothesis exposed a concrete production mismatch:
Rust collected high records separately, then invented low spans from automatic
same-cell Tube shells and required structural groups. Native produces one
interleaved scan, requires an increasing native cell ordinal for a Tube record,
and permits explicit Tube records without a damage group. Rust also lost a high
record when the probe immediately beyond its matching far endpoint left Size.
The replacement follows the original record producer and publishes its shared
dummy coordinate effects.

The immediate base-zone consumer was a necessary prerequisite: newly admitted
raw Tube exits must not be interpreted using unsigned coordinates and Rust's
smaller rectangular stride. This increment carries source Size with the derived
record set, implements native signed/clamped zone lookup, reverse record order,
canonical bucket pairs including zone zero, and invalidates navigation on
record-only or geometry-only changes. The source Size is derived rebuild
provenance, never a second independently editable map authority.

This is bounded producer/base-consumer parity evidence, not full bridge,
pathfinding, C6, GSI-04.01 or Phase 3 closure. The separate Tube hierarchy arm,
all remaining CellIterator consumers, full flood-fill parity and broad runtime
bridge lifecycle remain open. No TS-only behavior is included.

## Native behavior established

| Native body | Established behavior and active ownership |
|---|---|
| `CellIterator::Init 0x00578350`, `Next 0x00578290` | Seed `(1,Size.width)` and the width-dependent alternating antidiagonal state. `Next` returns the current cell-table pointer, then advances. Consumers stop on the first null pointer; width-one and internal-null cases cannot be replaced by sorting all allocated cells. |
| `MapClass::ComputeBridgeZones 0x0056D6E0` | Clears Map+0x50's record vector through `0x00588CE0`, then performs one CellIterator scan. Native calls include InitZoneMap `0x005671EB`, full scenario `0x00684FC7`, RMG `0x00599EFD`, invalidation `0x0056DAFC` and validation `0x0056DB8C`. |
| High branch, tables `0x0082A734/0x0082A774/0x0082A7B4` | Concrete and wood high tiles use the first 16 offsets, exact start-subtile/direction/end-subtile tables. High tile classification has priority even when its start subtile rejects. A far match commits after the following probe, including an outside-Size probe, and obtains endpoint B through the actual backward helper. Non-high cells without structural bit `0x100` clear activity. |
| `CellClass::IsTube 0x00484AB0`, `GetTube 0x00484F20` | Tube admission requires signed tube index in the global vector and land dword value 10. Opposite-neighbor tests short circuit east/west before south/north. No structural group, path length, source-kind or span deduplication requirement exists. |
| Ordinal `0x0042B1C0` | Signed endpoint words and wrapping dword arithmetic: `((x-W)-1+y)*W + (((x-y)-1+W)>>1)`. Emit kind 1, active 1 only if the current ordinal is below the referenced Tube exit ordinal. Same-cell automatic shells cannot produce such records. |
| Step `0x00481810`, lookup `0x005657A0` | Neighbor queries use the shared fallback dummy and stamp its packed coordinate on a miss. Retained references observe later writes. High internal holes read its retained structural flag; an outside probe followed by a backstep can stamp it twice. |
| `InitZoneMap 0x00567110` | Instructions `0x0056713E..0x00567151` compute `s=Size.width+Size.height+1`, store `s*s` at Map+0x6C and allocate four bytes per node at Map+0x68. Loop `0x00567172..0x0056717C` initializes class 7 and level 0 for every node. |
| `RebuildZoneConnectivity 0x0056C510`, bridge region `0x0056C6CB..0x0056C7E6` | Reverse active-record traversal, no kind filter. Sign-extend each endpoint word, form wrapping `y*s+x`, clamp the signed result to `[0,s*s-1]`, read the base-zone word. Unequal IDs are sorted min/max and deduplicated within nibble bucket `((min&15)<<4)|(max&15)`. Zone zero is retained. |

For normal initialized maps, native padding row/column W+H retains zone zero:
InitCellAttributes `0x00568BB0` invokes RecalcAttributes `0x0047D2B0` only for real
allocated cells; that body returns immediately for the shared dummy and otherwise
writes only its current native zone node. Admitted Size-diamond coordinates never
reach padding. RebuildZoneConnectivity zeroes all zone words and skips class 7.
The Rust consumer translates the native clamped index back to native coordinates
before projecting into its own smaller backing grid; padding yields zero.
This proof concerns normal initialized geometry, not arbitrary corrupt coordinates.

The dummy no-tile/no-tube defaults come from CellClass constructor `0x0047BBF0`
and in-place Resize reconstruction. IsoMap misses discard tile/subtile payload;
dummy Recalc returns before tile, land or zone work. Existing supporting audit:
[shared dummy final-recalc field evidence](bridges/01-assets-map-load-overlay/OVERLAYPACK_SHARED_DUMMY_FINAL_RECALC_FIELDS_REINVESTIGATION_GHIDRA_REPORT.md).
The new producer reads the modeled retained structural bit and writes coordinates
through the shared lookup; it does not create a private dummy. Native comparison
fixtures start with no tile, no tube and zero land/flags. No claim is made for
externally corrupted saves or an unproved independent dummy tile writer.

Fresh extraction on this date bound all six theater files to the active install's
`ra2md.mix -> localmd.mix` winners (`asset_extract`, `all_mixes=false`). Their
SHA256 values exactly matched the mounted INIs. Declared numeric TilesInSet
totals are 838/964/1081/1175/726/198 respectively; none of the retail high-bridge
windows can approach dummy tile DWORD `65535`. Thus skipping dummy high-tile
classification is valid for these retail theaters, separately from the sentinel's
name. Nonnumeric `TilesInSet=o` in urban-night does not add a declared tile count.
The oracle now supplies exact DWORD `65535`, not signed `-1`.

| Theater file | Fresh retail SHA256 |
|---|---|
| temperatmd.ini | `bf656852ad19636068f2dde6f9ddd1440ecf0e13e6e4de6cd7fb179a3592549d` |
| snowmd.ini | `8e357795fad0d6e3ad383a1044c09a7da0f710790b5992dd987a0a09cf12980f` |
| urbanmd.ini | `d1ac06c81c58d4694009f036d062b10a238caeb19fa8cfac4d96c54342e6cd75` |
| urbannmd.ini | `fe64bd6020a24111ac8e086efe7f193912f0dbff1925dc440118b76b7646f785` |
| desertmd.ini | `51011ed5e335fa966f1e8fec236e86cfc850561b2d2d5e9b9579cd9116e4d2b7` |
| lunarmd.ini | `fcfe324c6c95258052dd3a77d7f7fc903f325ebda87c039136ec9aa8fdb57fea` |

Local extraction receipts and bytes remain under
`.local/bridge-retail-theater-proof/`. This is fresh retail data verification,
not a claim that arbitrary replacement theaters exclude tile 65535.

Explicit Tubes are accepted by the active YR loader `0x007283C0`; record production
is therefore content-conditional active-YR behavior. Automatic Tube shells and
ordinary stock low Road overlays are distinct mechanisms. The earlier retail
coverage census reported no explicit `[Tubes]` in 385 decoded payloads; that
absence does not exclude accepted authored input or convert stock Road into Tube
records. This increment does not claim a new 385-map scan.

## Rust production delivery

`ResolvedTerrainGrid::native_cell_iterator` owns the exact producer traversal.
`bridge_state/record_scan.rs` owns the single high/Tube record pass.
`BridgeRuntimeState::from_resolved_terrain_with_map_size` is the production
factory; the old rectangular factory is now `cfg(test)` only. Initial runtime,
post-map runtime finalization, scenario crate refresh and scenario-start crate
refresh supply their existing raw Map Size authority.

`zone_build::register_bridge_base_edges` implements the immediate original
consumer using source Size. `ZoneGrid` retains that derived geometry and ordered
record identity. `NavigationCaches::rebuild_zones` rebuilds when either changes,
even if the path grid and class grid do not. The wall-damage repair path checks
the same identity and supplies geometry when reconstructing an absent cache.
Group-less Tube records remain active through endpoint activity refresh.

BridgeRuntimeState serializes the geometry receipt alongside its records and
includes present geometry in deterministic hashing. Snapshot schema is 140:
bincode cannot safely infer an omitted field inserted into a prior struct layout.
Version 139 is rejected by the existing preamble rejection check. Synthetic
geometry-absent states retain their prior hash treatment; production factories
always retain Size. Save/restore coverage checks an actual connectivity result.

## Bounded native comparisons

- [Producer harness](../../tools/spatial_oracle/bridge_records.py),
  [83 native cases](../../tools/spatial_oracle/bridge_records.json),
  [provenance](../../tools/spatial_oracle/bridge_records.meta.json).
  Executes the original CRT direction initializer `0x0049F2F0` before any neighbor
  query: its offsets are BSS zeroes in the image. Executes the genuine vector clear,
  supplies adequate external vector capacity at `0x0056D6F7`, then executes the
  complete original scan to return. No code patch or substituted function return.
  Observational hooks capture visit order; record semantic fields and final dummy
  coordinate are read from native memory. Constructor/INI resolution, allocation
  failure and unused stack padding are outside the comparison.
- [Base consumer harness](../../tools/spatial_oracle/bridge_base_edges.py),
  [86 native cases](../../tools/spatial_oracle/bridge_base_edges.json),
  [provenance](../../tools/spatial_oracle/bridge_base_edges.meta.json).
  Reuses all 83 producer outputs plus three consumer fixtures. Executes original
  `0x0056C6CB..0x0056C7E6` with supplied base-zone words and adequate bucket storage.
  This proves the record-to-bucket boundary, not preceding flood fill or subsequent
  native adjacency allocation. Distinct pairs 1/2 and 17/18 collide in bucket 0x12;
  fixtures explicitly distinguish reverse order and retain exact duplicate checks.

Producer coverage includes all 16 high start/end table offsets, rejected subtiles,
both axes and material bases, matching far endpoints at the Size boundary,
missing far endpoints, internal holes, mixed high/Tube scan order, neighbor
short-circuit branches, stale/missing Tube indices, land mismatch, zero/nonzero
path length, increasing/decreasing/equal/extreme signed exits and small widths
including one. Consumer coverage includes signed extremes, native linear aliases,
both clamps, padding zero, equality/inactive skips, reverse collision and duplicate
order. These are saved examples, not an exhaustive proof over every map state.

Rust tests call the actual factory and consumer helpers:
`sim::bridge_state::tests::native_bridge_records_match_original_executable` and
`sim::pathfinding::zone_build::tests::native_bridge_base_edges_match_original_executable`.
`sim::world::tests::navigation_tests::native_bridge_record_geometry_changes_rebuild_and_restore_navigation`
uses two ground regions separated by class 7: adding the raw endpoint `(26,0)`
connects them with native side 17, changing only Size to side 19 removes that
connection, restore preserves the result, and activity-only changes disconnect.
This is Rust lifecycle regression evidence, not a full native scenario capture.

## C6 consumer census and remaining work

Fresh Ghidra xrefs returned 215 call sites for Next and 81 for Init; multiple calls
within one owner and inlined initialization mean these counts are not independent
mechanisms. Native owners include map load/full init/Resize/InitCellAttributes,
ComputeBridgeZones, shroud reveal/reset/restore/blackout/edge-redraw, radar bounds
and colors, MouseClass save, House computer takeover, preview and theater/light
passes, TriggerAction execution, tiberium queue rebuilds and numerous RMG passes.

Current-source checks already found dedicated antidiagonal owners in
`NativeOverlayMapShape::recalc_cells` for overlay passes,
`RmgGrid::native_cells` for RMG, ore growth's `native_rebuild_cells` for queue
seeding, and ordered tile-animation initialization. These are bounded source
findings, not executable certification of every consumer. Native identity's two
Resize generations and dummy construction counts are already owned by
`native_identity::build_noncampaign_fresh_id_prefix` and bootstrap checkpoints;
the historical G3/C3 inventory is not authority for duplicating them.

Remaining C6 audit includes each unproved shroud/radar/preview/house/save/trigger
consumer and order-sensitive effects in the existing specialized owners. Do not
replace generic storage iteration globally. Separate full bridge hierarchy Tube
edges remain unimplemented: this record/base-consumer repair improves immediate
connectivity while leaving long-route hierarchy behavior open. Broader native
zone-fill label/order, padding lifecycle outside initialized maps, runtime bridge
activity semantics, and complete map-constructor/Resize effect coverage also
remain separate unclosed evidence obligations.

## Validation record

Commands run from the task worktree with `VERA20K_GAMEMD_EXE` set to the above
retail executable. Raw local logs are retained under `.local/`.

- Initial producer-focused command:
  `cargo test -p vera20k --lib sim::bridge_state::tests::native_bridge_records_match_original_executable`:
  exit 0, 1 passed, 0 failed, 8706 filtered; 0.04s test time. Log
  `.local/bridge-record-focused.log`.
- Initial combined `cargo test -p vera20k --lib native_bridge_`: exit 0,
  5 passed, 0 failed, 8704 filtered; 0.08s. Log
  `.local/bridge-combined-focused.log`. This preceded the strengthened collision
  and connectivity fixtures; final candidate validation is recorded below.
- Final `python -B -m tools.spatial_oracle.bridge_records --check`: exit 0,
  83 native cases match, no files written; `.local/bridge-record-native-check.log`.
- Final `python -B -m tools.spatial_oracle.bridge_base_edges --check`: exit 0,
  86 native cases match, no files written; `.local/bridge-base-native-check.log`.
- Final `cargo test -p vera20k --lib native_bridge_`: exit 0, 6 passed,
  0 failed, 8703 filtered; 0.07s test time, 58 warnings. Log
  `.local/bridge-combined-focused-v3.log`. This includes all 83/86 native cases,
  stronger connectivity/restore coverage and the schema-140 assertion. The prior
  v2 run exposed only a stale fixture count (83 versus 84), then corrected as the
  corpus expanded; no native output was rebaselined from Rust.
- Final full `cargo test -p vera20k --lib`: exit 0, 8590 passed, 0 failed,
  119 ignored, 16.75s; 58 warnings. Log `.local/bridge-full-lib-final.log`.
  Docs-only refreshed-main integrations changed no source or tool implementation.
- Final `cargo clippy -p vera20k --lib`: exit 0, finished in 38.72s with
  1144 warnings; `.local/bridge-clippy-final.log`. The warning backlog was not
  rebaselined or bulk-edited. Independent source/evidence review passed after
  the confirmed fixes; final publication review reconciles these actual receipts.

The first full suite after docs-only main integration (`1c055245`) exited 101:
8589 passed, 1 failed, 119 ignored, 15.10s (`.local/bridge-full-lib.log`). Its sole
failure was an older test expecting an invented automatic-shell span. Inspection
showed a hand-built five-cell all-GROUND row, not a decoded stock map. The corrected
`automatic_tube_shells_keep_ground_connectivity_without_bridge_records` test
requires no records while retaining Normal/Infantry connectivity and absence of
high-layer redirects. Native no-record evidence comes from the separate original
`automatic_shells` corpus case. This corrects a false fixture contract; it does not
weaken passability assertions or change production code to accommodate a test.
Its focused command
`cargo test -p vera20k --lib sim::pathfinding::zone_map_tests::automatic_tube_shells_keep_ground_connectivity_without_bridge_records`
exited 0: 1 passed, 0 failed, 8708 filtered, 0.00s; log
`.local/bridge-shell-connectivity-focused.log`. Independent read-only review
accepted this exact correction and retained the bounded source/evidence PASS.
