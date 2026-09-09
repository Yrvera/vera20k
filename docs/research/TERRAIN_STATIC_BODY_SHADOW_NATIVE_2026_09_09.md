# Ordinary static terrain body and shadow

This increment starts from approved palette/voxel commit
`e4cca5ac7b9cfe1415ca91807a9d3300d933766f`. Its acceptance target is ordinary
static TREE01 placement, source ownership, native depth and packed RGB565
shadow, including static overlaps. Animation, death and SpawnsTiberium remain
open branches. No whole-terrain or whole-renderer equivalence is asserted.

## Original behavior

Pinned gamemd.exe SHA256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Terrain DrawIt `0071C1B0` preserves its supplied point at `0071C234..245`.
Ordinary body `0071C2B2..304` uses Cell+34 Convert, signed Cell+10C brightness,
flags 4E00, gradient 2 and Z adjustment -AdjustForZ-12. The subsequent shadow,
gated by initialized byte 00822CF1=1, uses that same point, the frame at
bodyFrame+frameCount/2, flags 4E01, gradient 0, brightness 1000 and
Z adjustment -AdjustForZ-3. There is no ordinary screen-Y minus 3 in this
caller. Centering subtracts integer full-canvas halves and adds the stored
frame X/Y at `004AF002..07C`.

TREE01.tem has a 155 x 155 canvas, body rectangle (61,1,33,78), and shadow
rectangle (79,43,74,36). The retained native capture identifies all 1,470 opaque
body pixels at (360,328), fixing the common point at (376,404); the shadow
starts at (378,370) and has 1,000 stencil pixels. These local retail asset and
capture bytes remain in the parent audit, not in tracked tooling. Source asset
SHA256: `619cb6cba054eaf7c00cb6331eb70a686be25e60fd86573b352df22e22b1956e`.
Body and shadow source masks have zero overlap, so that isolated capture cannot
establish overlapping-destination behavior.

Selector `00490E50` chooses Convert+158 for 4E00 (vtable 007E53A0, leaf
004990E0) and Convert+114 for 4E01 (vtable 007E54B0, leaf 00497390). Both leaves
test signed candidate row Z minus signed shape byte against zero-extended old
u16 Z, strictly less. Accepted pixels store low16(candidate). Body writes its
palette word; shadow writes `(destination >> 1) & mask`. Original mask block
004BAA73..004BAAC1 produces 7BEF for RGB565. Compressed source-zero skips leave
both destinations untouched.

Native clear is distinct from the row seed. Constructor 007BCA08 fills the
backing surface with FFFF, then 007BCA2C initializes the separate DefaultZ row
seed to 8000. Active dirty clear 006D2B60 calls 007BCFB0, which writes FFFF.
Earlier design notes conflated these values. Empty storage must be 65535;
candidates 32768 through 65534 remain admissible, while equal 65535 rejects.

## Executable evidence

[Portable leaf oracle](../../tools/terrain_draw_oracle/leaf.py) executes the
original leaves with prepared synthetic Convert/A/Z/palette buffers. Its
[fixtures](../../tools/terrain_draw_oracle/fixtures/leaf.json) contain 550 signed
candidate/store/repeat cases plus compressed holes. The accompanying packed
color fixture exhausts all 65,536 destination words, SHA256
`5ad832a7d9435b1d61eac2a0cce43d6bd3db84274c9ef277996a184eba1dc53d`.
Candidate -1 stores 65535; an identical repeated shadow darkens again because
-1 is less than 65535. In-range equality rejects. Prepared rows do not execute
the whole caller or surface allocation.

[Portable row oracle](../../tools/terrain_draw_oracle/rows.py) executes original
regions 00437B70..00437CD4 and 00437E82..00437EAE with checked stop boundaries.
Its [fixtures](../../tools/terrain_draw_oracle/fixtures/rows.json) retain 406
clipped initialization/walk cases and 13,854 native row values. They cover
ordinary gradients 0 and 2, nonmultiple heights, top clipping and a negative
prepared adjustment. The captured body at top 328, height 78, adjustment -12
starts at 32324 and ends at 32349; the shadow at top 370, height 36, adjustment
-3 walks 32395 down to 32360. These are prepared zero-origin states.

[Original caller rebasing](../../tools/terrain_draw_oracle/origin.py) executes
005F4B88..005F4CFD before the Terrain virtual call. Six cases cover viewport
Y=0/37 and dirty clip Y=viewport/100/350. The resulting point plus dirty clip
origin always retains framebuffer Y=404. Thus the effective native row top is
framebuffer top minus viewport Y; the dirty origin cancels and must not be
subtracted again. ZBuffer construction copies viewport Y at 007BC991 and the
resize caller supplies it at 004A8A1D..3A. The projected point is an explicit
prepared input; projection itself is outside this oracle.

[Original clear replay](../../tools/terrain_draw_oracle/clear.py) executes
007BCFD7..007BD0D7 with prepared lock/ring/rectangle state. Seven aligned,
unaligned and ring-wrapped cases establish FFFF stores. The aligned-width-one
case writes nothing in that optimized region; the GPU full-frame clear does
not claim equivalence to every native dirty-clear optimization.

## Production ownership

OverlayAtlas retains literal source indices beside RGBA, copied by one shared
placement owner. Ordinary nonanimated, non-SpawnsTiberium terrain with a
drawable extended body and paired second-half shadow is registered separately.
Runtime lookup emits both stored rectangles from one projected cell-centered
point, body first, without the prior FA2 editor adjustment. Unsupported raw or
animated terrain keeps its existing route and remains unresolved.

Tactical native TMP/SHP/VXL fragments now store u16 Z as `word / 65535` in
Depth32Float. Empty depth is 1.0; the row seed remains 32768. Legacy Batch
consumers keep their CPU world/sort scalar and existing Less/LessEqual and
write/read-only policies, including the UI overlay pipeline. Their monotone
affine conversion retains fractional epsilon ordering where float precision
allows. Existing CPU .001/.999 clamps still lose off-map rows; these
compatibility inputs are not claimed exact native u16 producers. TREE rounds
a mixed compatibility destination when reading it. Non-TREE out-of-range
candidate semantics remain unproven: hardware comparison after a wrapped store
is not generally equivalent to the original signed comparison.

`terrain_draw.rs` owns a scissored snapshot and per-piece edit. Live tactical
depth remains the single authority. wgpu 27 forbids partial depth texture
copies (wgpu-core 27.0.3 command/transfer.rs 502..523), so the snapshot is a
render pass sampling live depth while it is not attached. Color uses an
explicit non-sRGB compatible view with TEXTURE_BINDING usage; the swapchain is
never sampled. The MRT scratch formats are R32Uint packed RGB565 and R32Float
native depth, avoiding linear-color quantization. Reusable targets follow both
concrete source handles after resize or the upscale depth swap.

Each individual body/shadow piece gets a fresh snapshot after the previous
piece wrote. The edit shader applies signed comparison before the wrapped
store through a depth-Always pipeline. Normal Ground runs resume between
native edits with their original pipelines and tactical scissor. Scratch uses
absolute framebuffer coordinates; clipping preserves original row origins.
The actual Batch camera upload owns the retained CameraUniform that also
supplies piece bounds. The named viewport origin is in unscaled native rows,
while scissors use target pixels. A frame camera update resets the origin,
then tactical preparation publishes the current viewport Y divided by zoom.

Both current Terrain operations write accepted Z. Vehicle cached-shadow leaf
00496820 instead tests without storing Z and is a separate, unimplemented
consumer; it must not inherit this Terrain policy.

## Validation boundary

`terrain_draw_gpu_tests.rs` uses the production Batch pipeline construction,
camera write, texture-with-indices upload, and actual TerrainDrawRenderer
snapshot/edit methods. Synthetic native goldens isolate color and signed-depth
semantics. The separate terrain_ground_gpu_tests module exercises actual
Ground lowering, pooled instance upload and replay with synthetic atlas pages,
including interleaved TMP, TREE, SHP, VXL and coalesced shadows. These pages do
not claim retail loader equivalence or map-startup coverage. The original
TREE asset/placement comparison and a new application capture remain required.

The six production-owner GPU checks passed: all 65,536 destination words in
both BGRA/RGBA sRGB target formats; all 550 signed/store/repeat cases; all 406
row cases at viewport origins 0/37; empty-depth, zero-stencil and replacement
target handles; 15 fractional-camera/zoom cases; and actual Batch
stamp/bracket/UI near, equal and map-edge policies. The Ground replay check
passed nine successive prefixes, inspecting intermediate color and depth
through TMP, TREE body, SHP read, shadow, SHP write, equal-shadow rejection,
VXL read, shadow and coalesced negative repeated shadows. These tests cover
the previously open synthetic static-overlap, clipping and resource-lifecycle
gates. Thirteen existing actual-shader depth/palette checks also passed.

Literal final check results, preserved in the local versioned handoff packet:

```text
terrain owner GPU: test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 8659 filtered out; finished in 0.79s
Ground replay GPU: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8664 filtered out; finished in 0.24s
depth regression GPU: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 8652 filtered out; finished in 6.38s
full library: test result: ok. 8560 passed; 0 failed; 106 ignored; 0 measured; 0 filtered out; finished in 14.04s
clippy: exit 0; 1144 warnings; Finished dev profile in 1m 35s
System Map: OK: 0 error(s) across 3 file(s)
```

The three correctness GPU runs used the v3 runtime freeze; subsequent source
changes only corrected an obsolete shader-source assertion, clarified comments
and changed the ignored performance probe to one frame per submission. The
final full library and clippy runs cover that v6 source. A first full run had
the obsolete assertion and a missing machine-local `ini/rules.ini`; the
assertion was corrected and the existing original local input copied before
the passing full run. The 106 ignored checks were not all run: only the
explicit GPU and timing filters above/below are claimed. GPU adapter was AMD
Radeon (TM) Graphics, Vulkan, driver 23.19.24.07.

Reproduction uses `cargo test -p vera20k --lib` for the full suite. Run the
ignored `render::terrain_draw_gpu_tests::` filter with
`-- --ignored --skip production_tree_piece_workload_timing`, the
`terrain_ground_gpu_tests::` filter with `-- --ignored`, and the
`render::depth_gpu_tests::` filter with `-- --ignored` for the bounded GPU
checks. The portable native oracles require the pinned original executable
through `VERA20K_GAMEMD_EXE` or `RA2_DIR`; no original executable or retail
asset is distributed here.

## Performance and remaining acceptance

The ignored `production_tree_piece_workload_timing` probe exercises the actual
snapshot/edit owner at 800 x 600 with visible opaque rectangles of the TREE01
stored body/shadow dimensions. It is not the actual sparse TREE01 stencil or
a whole-game workload. Each pair costs four render passes, so 512 pairs submit
2,048 passes in one frame. Two scratch targets and bind groups are reused.

The exploratory measurement was contended: user-owned VERA PID 13812 remained
live at 2026-09-09 14:53:42 UTC. Medians of three one-frame measured samples
after a warmup were:

| Tree pairs | Dispersed GPU ms | Repeated-overlap GPU ms |
| --- | ---: | ---: |
| 1 | 0.06724 | 0.07884 |
| 32 | 1.90384 | 1.42285 |
| 128 | 4.72852 | 4.52312 |
| 512 | 19.64576 | 17.92024 |

At 512 pairs, CPU encoding medians were 1.5538/1.5839 ms respectively.
GPU timestamps include the per-frame clears and every snapshot/edit pass.
CPU timing includes encoder construction and command recording, excluding
finish, submit and wait; pipeline/texture setup is excluded from both. Normal
Ground pass restarts are additional production work absent from this probe.
The one-frame probe passed (`1 passed; 0 failed; 0 ignored; 0 measured; 8665
filtered out; finished in 1.23s`). An earlier three-frame submission probe
failed with an out-of-memory error at queue submission; its failure did not
record the exact pair count. One-frame submissions completed all counts,
which separates that batching failure from a demonstrated per-piece texture
leak. Dense-tree frame cost nevertheless remains a material open performance
gate. An uncontended repeat is required before inferring the correction size.

Independent review supports a bounded correctness capture candidate after
these checks, with dense-tree performance open. Actual application TREE01
capture, including ordinary static overlaps, remains required. Existing
same-source neutral/tinted clear-terrain captures remain regression gates for
the shared depth migration; the new candidate has not yet reproduced them.

The body conversion retains the approved opaque, clear tactical A=127
boundary. Later shroud composition does not prove native non-clear-A behavior.
The exact TREE destination-word operation also cannot correct a destination
already made different by legacy vehicle/bridge alpha shadows or other
unresolved compositing. Animated/death/SpawnsTiberium terrain, unsupported
source formats, non-TREE signed out-of-range depth and the previously stated
compatibility-depth losses remain open. No whole-terrain closure is claimed.
