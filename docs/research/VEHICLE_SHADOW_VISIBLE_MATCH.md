# Ordinary vehicle shadow geometry and body masking

This increment targets the visible stationary GTNK/HTNK shadow discrepancy in
the shared retail comparison scenes. It preserves the matched opaque vehicle
body and uses the existing single-pass voxel draw path. Final darkness remains
the existing linear alpha approximation (`0.782`) to packed RGB565 half; this
document does not claim exact destination-word parity.

Native authority is original `gamemd.exe`, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Unit `73B470` finishes body composition before shadow submission `73C5C4`.
`706BD0/707280` submits the hull's frame-zero shadow section through `753F90`,
which projects the four bottom corners. `754510` owns center and six-word crop;
`756860` walks occupied span columns in 8.8 coordinates and writes two adjacent
stencil bytes. Its optional BSurface queries test each byte separately against
the prior 256×256 indexed Unit body. Source zero is transparent in the inner
part copy (`4914C0`). Original `707480 → 437A10 → 496820` tests signed candidate
depth against stored unsigned depth, halves packed destination RGB565, and
leaves depth/A unchanged.

The portable [shadow oracle](../../tools/voxel_oracle/shadow_raster.py) executes
the original loader, startup camera/light, corner submission, crop, raster and
optional BSurface reader. Only immutable file I/O and allocation are supplied.
[Generated fixtures](../../tools/voxel_oracle/shadow_fixtures.py) cover all 32
facings with null and patterned body masks, without embedding retail art.
The retained local retail packet additionally covers 64 unmasked GTNK/HTNK
cases (`shadow-geometry-native.json`, SHA256
`95cf4edbc680484ec51f3058bc0d3b2e5d1cb02bed9c50cf940f972d610966a5`)
and 16 actual native composed-body masks (`shadow-mask-geometry-native.json`,
SHA256 `d701aba1bf989c61963c1846b7575cf98092a0aa62d11aa7bcec474dfebcb424`).
These local files are provenance, not required clean-clone links or bundled art.

Production ownership is split by responsibility:

- `vxl_shadow.rs` prepares the ordinary flat, scale-one, single-section stencil
  and crop using the shared x87 arithmetic helpers. The native startup light X
  is bits `403ffff1`, not rounded `3.0`.
- `unit_atlas.rs` installs native templates only for parsed Drive types without
  ConsideredAircraft. A private legacy companion preserves previous geometry
  when the runtime caller is outside this boundary.
- `unit_shadow_cache.rs` composes the nonzero mask from the actual selected
  body/turret/barrel entries at their production offsets. The first supported
  type/facing request retains the filtered pixels; hits skip composition and
  texture upload. Repacking transfers those pixels into the new atlas while old
  pages remain available to already queued draws.
- Unit presentation admits this path only for active Drive, ordinary Ground,
  stable flat slope and visible uncloaked bodies. It emits the masked shadow
  after the body parts. Ground lowering keeps their piece order and the existing
  read-only depth pipeline; no destination snapshot is added per vehicle.

Validation separates generated native geometry, stock native mask/crop,
production part-mask construction, atlas upload/first-fill/repack GPU checks,
and actual Ground lowering/shader clipping/order checks. Passing synthetic
checks alone does not certify an application scene. The final scene comparison
must retain the existing body/terrain/TREE matches and judge shadow shape and
darkness at normal gameplay size. Stable 20k lookup and draw costs must be
reported separately from cold first-fill cost.

Bounds: global four-family arena purges, type stream resets, full native dirty
visitation/history, slopes/aircraft/other locomotors, relative turret cache
histories, nonzero shadow-section overrides and transient effects are not
closed here. The first eligible VERA draw supplies this bounded cache's mask;
it is not a reproduction of every native first-fill admission. Native cached
and uncached rows differ under bottom clipping; this route retains the cached
full-crop row contract and does not combine the two formulas. Signed negative
depth/wrapped storage cases and exact packed half remain broader renderer
differences, to be prioritized by visible effect under the amended acceptance.

The isolated candidate checks passed: generated 64-case fixture (1 test), stock
64+16 stencils plus 16 production part masks (2 tests), actual atlas
pack/upload/first-fill/hit/relocation with retained old pages (1 GPU test), and
actual Ground lowering/voxel read-only depth/order/left-bottom scissor (1 GPU
test). The atlas check's first version stopped at its relocation precondition;
the added synthetic sprite tied the old height and did not move the shadow.
Increasing that fixture's height exercised relocation and passed without a
runtime correction. All GPU checks used the AMD integrated Vulkan adapter.
Twenty thousand warmed same-key cache calls took 1.0034 ms; this is CPU lookup
cost only, excluding caller key allocation, instance building, cold fill and
GPU draws. It is not whole-game performance evidence.

After these checks, the unchanged caller predicates and companion-key selection
were extracted into small helpers for a direct Drive/temporary-Teleport/Hover/
nonflat/cloak regression. That final test is deliberately unexecuted in this
isolated checkout, awaiting the combined candidate's focused/full checks and
clippy. Application capture and normal-size shadow-darkness acceptance also
remain pending; no final renderer approval is implied.
