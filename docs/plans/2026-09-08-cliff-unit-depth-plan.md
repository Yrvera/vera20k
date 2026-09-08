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

Preexisting representation limit: normal movement retains coarse cell-level Z and
semantic altitude, without subcell ramp height. For these poses, SHP admission uses
semantic altitude; exact coordinates use native slope surface subtraction. This
avoids introducing airborne classification for grounded ramp infantry. Missing
subcell ramp pose remains DRIFT: ordinary ramp sprite position/depth cannot yet be
claimed identical to YR. Flat cliff-adjacent cells do not depend on that gap.

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
