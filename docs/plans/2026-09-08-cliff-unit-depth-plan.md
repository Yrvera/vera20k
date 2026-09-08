# Cliff-adjacent unit depth correction

Continue `feature/sprite-zbuffer-depth` at `ef3fc0fa`. The user authorizes this
plan followed by implementation. Preserve the existing terrain/TMP projection,
voxel shading, and the accepted wall/building depth changes.

## Behavior and evidence

Native `gamemd.exe` SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`:

- TechnoType constructor `0x710AF0` stores `ZFudgeCliff=10` at `+0xDC0`
  (`0x711664`). UnitType retains it; INI `0x71541C..0x715437` uses the
  initialized value when the key is absent. Old cliff research claiming a
  zero default and stock dormancy is contradicted by the binary.
- `0x704240` gates on the actual Object `OnBridge` byte, then independently
  probes raw Object cell coordinates plus `(1,1)`, then the first resolved cell's
  stored coordinates plus `(1,1)`. These are `(x+1,y+1)` and `(x+2,y+2)` for
  ordinary cells, but aliases and dummy fallback retain their native behavior.
  A signed height difference >=4 selects
  multiplier 2 for the first probe and 1 for the second; the second overrides
  the first. Runtime initializer `0x49F34A` establishes the direction.
- Foot `0x4DAFC0/0x4DAFF0` composes the type-weighted cliff term through a
  maximum with bridge/column/tunnel terms, then adds its base and `0x704350`
  adjustment. Normal composite `0x73B140` and cached voxel `0x707480` consume
  it. The missing stock cliff term is 10 or 20 native Z units before the
  existing blitter quantisation.

Before this increment, `instances/units.rs::voxel_z_adjust` supplied only elevation
cancellation. SHP foot drawing also omits the term. Terrain already writes
authored depth and the voxel pipeline reads the same attachment. This is a
missing input to the existing mechanism, not disconnected cliff rendering.

## Implementation sequence

1. Finish the bounded native contract before changing callers: exact signed
   arithmetic and invalid-cell lookup; composition with stock-active sibling
   terms; grounded/airborne and aircraft gates in `TechnoClass_DrawSHP`; the
   harvester arm and shadow consumers. Include prerequisites needed for the
   ordinary cliff case, and state any remaining adjacent gaps explicitly.
2. Keep type data in rules. Parse the verified `ZFudgeCliff` default and
   override in `rules/object_type.rs`. Do not add a duplicated, serialized
   per-entity cliff value. Resolve the actual entity type, independently of
   disguise, alternate display type or art. Add required sibling settings after native
   defaults and active consumers are established.
3. Add one shared Foot depth owner, with pure native arithmetic separated
   from the instance helper that gathers current rules/entity/terrain facts.
   Use the live resolved terrain from the runtime view, not the immutable
   load template or an inferred `is_cliff_like` flag. Use `entity.on_bridge`
   for the native gate. Native-index lookup and dummy-level fallback must be
   read-only: `cellclass_projection_view` stamps a shared simulation dummy
   and must not introduce frame-dependent simulation mutations here. Retain
   native resolved/requested coordinates in a local view. Omission of native's
   shared dummy stamp is deliberate: rendering must not change subsequent
   wave/projectile simulation as camera position or frame rate changes.
   Include `0x704350`'s coherent ordinary base: exact `-AdjustForZ(Location.Z)`,
   default -1, and the verified ordered terrain/overlay/tall-TMP alternatives.
   TMP height for the >36 test is header height minus relative extra-Y, not
   the decoded union canvas size. Cache pristine headers for every registered
   tile type at load, including types not initially visited, so live LAT/ramp/
   collapse replacements and restored cells share the same type-owned dimensions.
4. Apply the result to the actual `SpriteInstance.z_adjust`. Compute it once
   per unit and share it between composite and separate hull/turret/barrel
   instances, including slope-transition/cached textures. Keep the existing
   shared composite rectangle. Apply the same verified Foot semantics to
   ordinary infantry and qualifying SHP unit draws. Do not silently change
   building turrets, aircraft or untraced shadow families through a generic
   helper. Existing painter sort bias is not a replacement for fragment depth.
5. Extend the GPU harness to execute the actual voxel shader with indexed
   textures, palette and house ramp. Exercise decoded retail cliff data through
   the production terrain-instance handoff. Avoid widening production APIs
   solely to fit a test. Record which production stages the harness covers.

## Acceptance and validation

- Parser and selector regressions: absent/custom/zero/negative type values;
  signed level difference 3 vs 4; first-only, second-only, both and neither
  probe; actual OnBridge gate; native lookup boundaries; maximum rather than
  sum with competing terms. Compare to native executable evidence where
  feasible; hand calculations are regression expectations, not parity goldens.
- Instance integration: the same current terrain/type result reaches all
  hull/turret/barrel/cached paths and the proven SHP consumers. Airborne and
  non-Foot branches retain their native caller gates. No simulation state
  changes during rendering.
- Actual GPU readback: +0/+10/+20 alter admission at the expected cliff depth
  boundaries; foreground pixels remain visible; transparent terrain/voxel
  pixels do not occlude; voxel depth remains read-only; separate pieces agree
  with a shared composite rectangle. Existing wall/building tests still pass.
- Coordinate Cargo ownership before builds. Run focused library tests, the
  explicit GPU regressions, one final full `cargo test -p vera20k --lib`,
  `cargo clippy -p vera20k --lib`, and a release build. Investigate unchanged
  baseline failures rather than absorbing unrelated fixes.
- A fresh read-only critic reviews implementation, native evidence, actual
  production consumers and results. Resolve confirmed findings.
- Prepare the same cliff-crossing situation for VERA and original YR. The user
  supplies the visual comparison when ready: front, partly hidden, behind,
  reverse movement, tanks and infantry. Keep code/test completion separate
  from this final visual acceptance; the previous screenshots did not cover it.

No publication or unrelated renderer changes are part of this increment.

## Implemented contract and evidence limits

`render/foot_depth.rs` owns selector arithmetic. The app adapter reads live runtime
terrain, actual type (with the proven temporary UnloadingClass exception), facing,
raw height, overlays, bridge/tube state and refinery/radio context. It never stamps
the shared simulation dummy. Body, separate voxel pieces, transition textures and
both unit shadow branches share Foot depth; OREGATH uses the pre-swap type. SHP
keeps its aircraft/GetHeight admission and subsequent -2 adjustment.

Additional native evidence: defaults 10/5/10/0 at `0x711664`; bridge/column/tunnel
selectors `0x703B10`, `0x703CC0`, `0x703E70`, `0x704000`; AdditionalZ `0x704350`;
pristine TMP dimensions `0x547150`; SHP `0x705E00`; shadows `0x707280` and
`0x707480`; OREGATH `0x73D236`; UnloadingClass swap `0x73D2C4`. Contact mission
uses current-or-queued (`0x5B3040`). Refinery adjustment reads the effective type.

The new executable oracle runs original, hash-pinned Foot functions in installed
Unicorn. Root reproduced all 85 stored cases using:

```powershell
$env:VERA20K_GAMEMD_EXE = 'C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe'
python -m tools.render_depth_oracle --check
```

This demonstrates bounded arithmetic parity (cliff thresholds, independent probes,
weighted maximum, signed wrapping, exact Z, facing and TMP dimensions). It does not
execute game construction, SHP admission, active tunnels/overlays, dummy aliases or
whole-game rendering. Those have separate Rust regressions/source evidence.

At the cliff implementation's original delivery, normal movement retained coarse cell-level Z and
semantic altitude, without subcell ramp height. For these poses, SHP admission uses
semantic altitude; exact coordinates use native slope surface subtraction. This
avoids introducing airborne classification for grounded ramp infantry. Missing
subcell ramp pose was recorded as DRIFT and is the subject of the subsequent
authorized continuation below. Flat cliff-adjacent cells do not depend on that gap.

Missing or invalid pristine TMP headers are diagnosed once at load and use an
explicit degraded 30-row fallback. Valid registered types use exact header heights;
unavailable data and invalid IDs have no native-equivalence claim. The existing
bridge painter-order bias remains separate: VXL and SHP depth shaders ignore its
`SpriteInstance.depth`, so it does not duplicate the new fragment Z adjustment.

The earlier wall/cliff comparison map has no cells where the cliff selector is
nonzero: its plateau boundary is invariant along `(1,1)`. Wall/building acceptance
remains valid, but this map alone cannot validate the newly implemented cliff term.
A correctly oriented retail cliff case is required for visual acceptance.

## Comparison fixture prepared

`python tools/render_depth_fixture.py target/depth-comparison/depth-cliff-back-v3.map --cliff-back`
creates a separate case with retail `Cliff28.tem` (tile76, 2x2, sparse slot0;
slots1/2/3 at level4) at (40,46) and (46,40). Foreground plateau is x+y>=87.
Tank (39,45) and GI (45,39) each probe levels [0,0,4], selecting +10. One cell
northwest provides the zero-term control. Native art provenance: `ra2.mix` /
`isotemp.mix`, entry `876BA238`. Independent critic checked geometry and decoded
map contents. Pixel admission still requires rendered comparison.

Generated v3 map SHA-256:
`b68f24e953892258f5d20ebac8d81cf0ce7130feb07dc3d3c7e113586d35bd50`.
Only this mode adds scenario `[General] CliffBackImpassability=0`: stock blocking
would mark the overlap cells Rock and reject Track/Foot starting placement.
The shared map override admits controlled positions in both engines while
preserving ordinary game rules and the native depth coefficients. This is an
occlusion fixture, not evidence of stock pathfinding through those cells.
All three Python fixture regressions pass, including old default/walls-only byte
hashes; the accepted comparison maps were not replaced. No special hidden capture
command is available for a custom map: the current tactical capture contract is
sealed to the stock Fight.MAP radar checkpoint. Use the prepared quickplay map for
the user's eventual cliff comparison instead of changing that unrelated contract.

## Final candidate validation

- Native executable fixtures: `PASS: 85 native Foot Z fixtures reproduce`.
- Focused math/runtime tests: `test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 8587 filtered out; finished in 0.01s`.
- Actual GPU tests: `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 8593 filtered out; finished in 1.24s`.
- Full library: `test result: ok. 8512 passed; 0 failed; 89 ignored; 0 measured; 0 filtered out; finished in 34.45s`.
- Python fixture validation: `Ran 3 tests`, `OK`.

The full suite includes new pristine-TMP header/catalogue tests and the runtime
ramp gate regression. GPU tests run the checked-in WGSL with production instance
layouts and a decoded retail cliff through the terrain instance builder. They
verify depth boundaries, indexed transparency/remap, read-only voxel Z and shared
piece rectangles. Their small offscreen setup does not execute the complete
interactive app draw dispatch or certify native rendered-pixel parity.

Clippy was run on the final tested source and exits 101 on six existing
`clippy::approx_constant` errors: `src/render/vxl_raster.rs` at 414/419/424,
`src/map/rmg/phases/meander.rs` at 49 and `src/map/rmg/phases/river.rs` at 49/50.
All three paths exist and are byte-identical to the starting `51213e58` baseline
(`git diff --quiet 51213e58 -- <three paths>` exits 0). This increment does not
resolve that lint backlog; the lint command is not a pass.

Release build succeeded in 4m48s. Executable `target/release/vera20k.exe` SHA-256:
`d07a4292edd15394eb38070f03d8f2307f86b56f4268130ee580886516a7039f`.
No game was restarted during implementation; the candidate and shared v3 map are
ready for the user's next cliff comparison. The Cargo slot was returned to the
coordinating ownership task after an empty process check.

Fresh critic approved the tested local implementation after the fixture admission
correction. Native zero guard was rechecked: Rules+0x664 zero bypasses all three
cliff-back reclassification blocks in CellClass `0x47D2B0`; INI read
`0x66F1CB..0x66F1E6` stores the explicit General/CliffBackImpassability value.
Foot depth remains enabled. Rust's scenario override ordering is traced through
`process_owner` and `init`; native scenario-map override ordering was not separately
re-established in this increment and remains part of the pending game comparison.

Implementation committed locally as `3aa6c29168f69ac9929a33bc4abac736be772042`.
No push, PR or merge was performed. The System Map annotation adds only the
verified GSI-13.03 Foot owner/consumers; its checker reports zero errors.

## User acceptance and ramp continuation

The user subsequently reported that the cliff case looks good. This records
their visual acceptance of the current fix; no new paired capture accompanied
that message, so it does not establish whole-scene native pixel parity.

The next authorized change is precise ordinary-unit ramp height, followed by a
bridge overlap/entrance check. The existing `Position.exact_z_leptons` is the
authoritative coordinate used by gameplay, persistence and presentation. Height
must be written by simulation at the native movement/placement boundaries,
not reconstructed independently by the renderer. Fresh binary inspection
distinguishes Drive/Ship paid-point height writes from residual XY interpolation
that preserves the previous raw Z; a blanket per-tick surface snap is incorrect.

Implementation samples Drive/Ship paid and terminal coordinates, preserves raw Z
during their residual interpolation, and samples Walk coordinates at its ordinary
commit points. Placement clamps before its occupation update. Regressions cover
bridge flag/object-list ordering, stationary and tube-exit coordinates, independent
airborne owners, and save/load through a residual crossing. A fresh critic reviewed
the implementation and replay attribution. Bridge split-blit admission and complete painter ordering
remain unverified separately; fixing ramp coordinates alone cannot close them.

Legacy runtime inputs with `exact_z_leptons=None` retain that representation
until a real height-write boundary occurs. An idle pass cannot recover the native
historical paid-point sample and must not invent one. Snapshot version 137 rejects
older saves whose cell/track frame and missing paid-height sample cannot satisfy
the new resume invariants; version 136 is already used by another branch. The
existing exact-Z field is retained, without a serialization shape change. The coarse
placement API also retains the explicitly documented negative-terrain authored
Unit input and Infantry subcell-selection limits in the new native height report.

The instrumented candidate passed all 15 new regressions; its only four full-suite
failures were prior replay expectations. Exact-Z-only diagnostics recovered every
old whole-state hash probe. S2's five isolated movement pulses were traced to the
old delayed crossing return, with native loop evidence and independent review.
The report records both the justification and the remaining preexisting early
return on an actual paid crossing. Final uninstrumented validation follows the
reviewed expectation updates; the diagnostic run alone is not that final pass.

The controlled bridge map uses the retail Hills wood span, with a tank and GI at
the entrance, a stationary deck tank and ground GI at the same bridge cell, and
another GI for an attempted under-span crossing. It is staged in the isolated YR
test copy as `depthbridge20260908.map` (Soviet campaign entry), with the prior
campaign configuration backed up. Its SHA-256 is
`d95c7f468931c59cad996677ccc39555e00a9cbd88cf75ece42dece944456ced`.
Original retail files remain unchanged. Runtime bridge acceptance is pending.
The user confirmed that the nearby infantry can walk underneath this bridge in
original YR. This establishes that reference route, without yet certifying its
VERA counterpart or a paired pixel comparison.
The existing Rust retail characterization at
`movement_bridge_retail_tests.rs::infantry_ordered_under_hills_high_bridge_is_currently_refused` records
refusal of the same `(87,71) -> (87,78)` E1 route. Ground/deck cell admission is
therefore a separate movement discrepancy to investigate, not evidence that
the new height writer failed. The stationary ground GI still admits a bridge
drawing check; no successful VERA under-span traversal is claimed here.

## Ramp candidate validation

After removing temporary instrumentation and applying the independently reviewed
replay expectations, the final library run reported:

`test result: ok. 8527 passed; 0 failed; 89 ignored; 0 measured; 0 filtered out; finished in 21.73s`.

The actual candidate test executable also ran the ignored GPU and retail cases
(production code unchanged since that build; later changes were test constants
and provenance comments):

- GPU depth: `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 8608 filtered out; finished in 3.29s`.
- Hills deck crossing, Drive/Walk/Hover: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 8613 filtered out; finished in 8.60s`.
- Hills under-span **refusal characterization**: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8615 filtered out; finished in 0.92s`. This confirms the existing failure to issue a path, not successful traversal.

The headless retail fixture reports missing TWNK1 SHP frame metadata for ambient
ore twinkles; its movement assertions pass. Native height reproduction separately
reports `PASS: 158 native ramp-height fixtures reproduce`. The fresh critic cleared
the implementation, save/load regression and replay attribution, with the report's
explicit cadence, placement and bridge-routing limits retained.

The tested implementation is committed locally as
`5cafdce5a2625f49627e6840db7221fffb16321b`. No publication was performed.
The System Map now records only the reviewed height producer and projection/SHP
consumers; `python -m tools.system_map check` reports zero errors.

Final `cargo clippy -p vera20k --lib` exits 101 on the same six preexisting
`approx_constant` errors in `vxl_raster.rs`, `meander.rs` and `river.rs` described
above (1159 warnings). The three files remain byte-identical to `51213e58`.
Clippy is not a pass; no new deny-level error was introduced in this increment.

The final release build succeeded in 3m47s. Executable SHA-256:
`94011eab51e41b3cba2f60e5633cb86f066f3ee3a9076100418f531f66e6484a`.
It was launched on 2026-09-08 at 19:29 Europe/Berlin with the prepared bridge map
as `RA2_QUICKPLAY`, the primary checkout as working directory, and all other
inherited `RA2_*` overrides removed except `RA2_DIR`. The new executable's window
was responsive after loading (PID 40932), and its title changed to
`RA2 - Depth Comparison - Hills Wood Bridge`, confirming the selected scenario.
The isolated native YR reference remained open. The inherited log level is `warn`,
so the log does not independently identify the map. No pixels were inspected by the agent;
the bridge drawing/entrance comparison remains a user acceptance check.
The Cargo slot was explicitly returned after this task's release build finished;
a separate owner's active build was left untouched.

## Integration for publication

The user authorized publication and merge on 2026-09-08, followed by the separate
bridge work. Integrated `origin/main` at `33f36d79699687afa16e64eef8a6a471910efc7e`
into the existing depth branch at `5f41f2ea`. Main's shared `CellArrival` and entity
construction owners are retained. Walk height still precedes arrival bookkeeping;
paid Drive/Ship height follows it, while residual crossings preserve the prior raw Z.

Snapshot version 138 combines main's v136 Infantry terminal field and this branch's
v137 movement-resume contract. The earlier runnable v137 build lacked that field,
so accepting its saves with the merged layout would be unsafe. Existing version
rejection and terminal restore tests remain active.

Three current replay fingerprints were composed with main's new Infantry terminal
hash fold. Each pre-v136 projection exactly reproduces the premerge depth branch,
and every older projection passes. The initial integration run reached only the
three final current-hash assertions; its 738 other world tests passed. The current
versus pre-v136 hash policies differ only by the `InfantryTerminal` field fold.
An independent critic checked the source, observed values and assertion order.

| Fixture | Pre-v136 / premerge depth branch | Integrated current hash |
| --- | --- | --- |
| Global | `53C317DDE1B051F6` | `71EC0BD6ED9D45CC` |
| Bridge | `964B448BA90BD06A` | `3CCCDF294DDA4D4F` |
| Slice6 | `34D67612D63B1A87` | `BD1A450AFE28594E` |

These are Rust composition/regression checks, not native replay equivalence.
The global RNG pins, bridge route and replay assertions, Slice6 command checks,
and S2 position fingerprint remain unchanged.

Final integrated library run:
`test result: ok. 8535 passed; 0 failed; 88 ignored; 0 measured; 0 filtered out; finished in 26.36s`.
Only two test comments were clarified after compilation; no test logic or production
code changed. Focused checks separately passed 12 ground-height, 85 movement-arrival
and 86 snapshot tests. Actual GPU validation:
`test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 8615 filtered out; finished in 0.80s`.
Both native oracles reproduce all 158 height and 85 Foot-depth cases with the
explicit original executable path. The System Map checker reports zero errors.

The bridge split/composite and under-span walking limits above remain separate;
integration does not close them or claim whole-scene visual parity.

Integration commit: `75ed540b26d251604ab18582fe38c9c9c105b6f9`. A final independent
critic inspected the committed source and actual result logs without finding an
integration blocker. Clippy completes successfully (1154 warnings); current main
already resolved the six earlier deny-level findings. Release build succeeds in
3m51s, with SHA-256
`0adc20903905bde0e72c8aaa98d942bc542b9efd1d91556ac2d6f65e9af8dce7`.
The retail Hills Drive/Walk/Hover deck tests also pass (3 tests); the separate
under-span characterization still confirms refusal (1 test expecting that failure).
