# VFX Render Substrate Audit — gamemd.exe

**Purpose.** Consolidated audit of the pixel-level rendering substrate used by every
VFX class in gamemd.exe. The focus is *NOT* on per-class struct/lifecycle (those
already exist in class reports) but on the **pixel math, depth bias, alpha-substrate
(A-buffer), palette cycling, and dither behavior** that the Rust wgpu engine must
reproduce to achieve 99% visual parity.

**Scope.** Ten VFX classes: LineTrail, EBolt, LaserDraw (normal + prism), RadBeam
(straight — sine is dormant), DiskLaser, PixelFX (ore sparkle), BombClass clock
overlay, FlasherClass (Elite blink), Bounce (debris physics — no direct pixel write),
IonBlast (dormant).

**Research method.** Cross-reference of existing reports in
`ra2-rust-game-docs/` + live Ghidra MCP decompilation of the leaf rasterizer functions
(the common DirectDraw surface routines shared by all classes).

**Research status (see report body below).**
- §1 blend-mode table: HIGH — decompiled the shared line rasterizer this pass
  (`Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30`, the DSurface vtable[0x34]
  target). Same function backs EBolt, LaserDraw, and RadBeam. Gradient variant
  (vtable[0x40]) also decompiled. LineTrail's separate rasterizer already fully
  documented in LINE_TRAIL round 2.
- §2 Z-bias table + AdjustForZ: HIGH — `Tactical__AdjustForZ @ 0x006D20E0`
  confirmed via xref-trace (called by every VFX drawer).
- §3 A-buffer: HIGH — fully covered by existing `BSURFACE_CIRCBUF_ABUFFER_REPORT.md`
  and `SHROUD_FOG_RENDERING_PIPELINE.md`. Section consolidates and cites.
- §4 Palette cycling: HIGH (**but note the surprise**) — No runtime palette-index
  rotation exists in gamemd.exe. Visible cycling effects come from multi-frame
  TMP/SHP animation or per-AnimType palette references, NOT from a rotating
  palette-index loop.
- §5 Dithering: HIGH — No dithering in VFX paths. Evidence provided.

---

## Executive Summary

**(1) Blend modes — standardise on three shader pipelines.** All ten VFX classes
collapse to just three distinct pixel-math shapes: (a) **modulated alpha-over with
brightness-as-alpha** (LineTrail — uses `Surface__DrawAlphaLineClippedZ @ 0x004BEAC0`),
(b) **A-buffer-modulated direct write** (EBolt / LaserDraw / RadBeam — all share the
DSurface vtable[0x34] function at `Surface__DrawLine_ABufModulated_ZClipped @
0x004BFD30`, verified this pass), and (c) **direct 16bpp surface write, no modulation**
(PixelFX — bypasses all substrates). BombClass clock rides the regular SHP blitter
pipeline; DiskLaser rides LaserDraw. A single wgpu "beam" pipeline (`Replace` blend +
fragment-shader multiply by sampled A-mask) plus an "alpha-line" pipeline (premult-OVER +
A-mask multiply) covers every beam. **The biggest trap is that EBolt/RadBeam/LaserDraw
do NOT alpha-lerp against dst** — when the A-buffer is 0x7F (neutral), they overwrite
the destination pixel entirely with the beam color. Only when the A-buffer is less
than 0x7F (shroud edge, cloak mask) does their color get dimmed. Blending with the
underlying scene is indirect, mediated entirely by the A-buffer value.

**(2) Z-bias is a single formula driven by `Tactical__AdjustForZ` plus a per-VFX
offset of 0, -2, or -3.** `AdjustForZ` converts a world-Z lepton height into a
signed screen-pixel Y offset via a scaled ftol, and every draw path then adds a
small constant (−2 for most VFX, −3 for flat anims). In wgpu this should be a
single uniform per draw call; there is no per-class tuning table.

**(3) The A-buffer is a 16bpp-per-pixel alpha/shroud mask (value range 0x00–0x7F,
0xFE = transparent) written by shroud-edge SHPs and AlphaShape decals, then READ
by every blitter** (terrain, SHP, VXL, beam rasterizer) and used as an index into
a 64KB multiplicative blend LUT or a direct "×value/128" multiplier. In wgpu the
cleanest mapping is a fullscreen R8 "A-mask" render target, sampled at fragment time
by every VFX pipeline. Soft shroud edges depend on this — without it, beam/sprite
brightness will not correctly dim over fog-of-war edges.

**(4) Palette cycling — three live systems in YR, but NONE of them is a classic
"rotate palette indices N..M per frame" loop at the ConvertClass level.** (a) No
TS-style runtime palette-index rotation exists in gamemd.exe; searches for
`PaletteCycle`, `RotatePalette`, `CycleColor`, or a LogicClass tick call writing
to the ConvertClass RGB tables all return empty. Visible "water shimmer" / "Tiberium
glow" is actually achieved via **multi-frame TMP tile animation** (terrain tileset
authoring) + **per-AnimType palette references** (`AltPalette=`, `AnimPalette=`,
`FirersPalette=` on AnimTypeClass). (b) **Ambient-light ramp via
`LightConvertClass`** — rebuilt on scenario lighting changes, global darken/tint.
Not a per-frame rotation but a palette-table REBUILD on demand (dusk/night
transitions). (c) **Ore/gem twinkle via `PixelFXClass`** — per-cell triangular-wave
interpolation between two hardcoded RGBs, direct 16bpp write (NOT a palette
rotation). For wgpu: implement (a) as animated tileset textures with a per-tick
frame selector; (b) as a shader-uniform RGB multiplier; (c) as a shader-driven
per-cell sparkle particle.

**(5) No dithering anywhere in the VFX draw paths.** All blends in EBolt, LaserDraw,
RadBeam, LineTrail, PixelFX, and the SHP alpha blitters use straight integer
multiply-shift math. The 16bpp look is **bit truncation**, not ordered dither. In
wgpu, no dither LUT or blue-noise texture is needed; simply render in linear RGBA32
and let the final surface format handle banding.

---

## §1 — Blend Mode Table

All blend math verified against the leaf rasterizer function or direct surface write.
Pixel math is stated in 8-bit-per-channel form for clarity (all blends actually pack
to the 16bpp DirectDraw surface afterwards via `g_DD_{R,G,B}_{Loss,Shift}`).

| VFX type | Entry point | Pixel math (per channel) | Stages | wgpu equivalent |
|----------|-------------|--------------------------|--------|-----------------|
| **LineTrail** | `FUN_004beac0` = `Surface__DrawAlphaLineClippedZ @ 0x004BEAC0` | `out = (lerp(dst, src, brightness/256) * m) / 128` where `m` = A-buffer pixel (0..255) | (i) brightness-alpha-over lerp, (ii) A-buffer multiply, (iii) Z test `src_z < *zbuf`, (iv) mask test `m != 0` | Premultiplied-alpha `OVER` pipeline with `src.a = brightness/256`; fragment shader multiplies output by sampled A-mask; depth-test against terrain depth |
| **EBolt** | `EBolt__DrawRecursiveBolt @ 0x004C1F20` → `g_PrimarySurface->vtable[0x34]` = `Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30` (**verified this pass**) | When A-buf == 0x7F: `out = color`. When A-buf != 0x7F: `out_R = ((color_R * abuf) >> 7)`, same for G/B. Color = `PALETTE.PAL[LUT index 10/6/15]`. **Effectively a direct write, darkened by the A-buffer scanline when under shroud.** | (i) Z test `src_z < *zbuf`, (ii) A-buf mask check (`abuf != 0` else skip), (iii) A-buf-modulated 16bpp store, (iv) optional Z-write per arg (EBolt passes 0 — no Z write) | Line pipeline with `blend = Replace`, `depth_test = Less`, `depth_write = false`, fragment shader multiplies color by sampled A-mask; `discard` when A-mask = 0. |
| **LaserDraw — `DrawBeamSpecial`** (IsLaserEffect=1 path used by Prism, Mirage, Disk laser terminal beam, etc.) | `LaserDrawClass__DrawBeamSpecial @ 0x005509F0` → same `Surface__DrawLine_ABufModulated_ZClipped` | Same math as EBolt. Color is weapon's `LaserInnerColor / OuterColor / Spread` packed to 16bpp; per-iteration `>>= 1` halves channels before being passed to the rasterizer (halo from multiple parallel lines, not per-pixel blending). | Same as EBolt (A-buf modulated, Z-test only). `InnerLineCount` parallel lines at fixed perpendicular offsets. | Same line pipeline as EBolt. "Glow" = N parallel line draws with halved color per iteration. |
| **LaserDraw — `Draw` (plain)** (IsLaserEffect=0; railgun trail and early-path IsLaser) | `LaserDrawClass__Draw @ 0x00550260` → flat path uses vtable[0x34] = `Surface__DrawLine_ABufModulated_ZClipped`; gradient path (when DetailLevel `DAT_00A8EB78 != 0`) uses vtable[0x40] = `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00` (**verified this pass**) | Flat: same as EBolt. Gradient: `out = pack16((abuf * startColor) >> 7, (abuf * endColor) >> 7)` interpolated per segment — still A-buf modulated, no dst blend. | Same as EBolt. Gradient takes two colors and interpolates along the line. | Same line pipeline; for gradient variant, use a per-vertex color attribute. |
| **RadBeam straight (BeamType=1)** | `RadBeam__DrawStraightBeam @ 0x00659650` → per segment calls `RadBeam__ComputeStraightSegment @ 0x00659AC0` → `Surface__DrawLine_ABufModulated_ZClipped` | Same math as EBolt. Color = `RadColor` from `[Radiation]` for green variant, hardcoded Rules constant for blue. | Same as EBolt. `N = ceil(Length / StepSize)` segments (StepSize=10 blue / 20 green leptons). Ping-pong fade modulates color intensity **CPU-side** per segment. | Same line pipeline. Ping-pong intensity is a CPU-side fade scalar per segment. |
| **RadBeam sine (BeamType=2)** | `RadBeam__DrawSineBeam @ 0x00659CA0` | Same math as straight — dormant in YR (only invoked by IsRadEruption, which no stock weapon sets). | n/a | Skip for YR parity. |
| **DiskLaser terminal beam** | `DiskLaserClass__AI @ 0x004A7340` constructs a `LaserDrawClass` with `IsLaserEffect=1` | Rides on `DrawBeamSpecial` — same direct-write math as LaserDraw prism path. | same as LaserDraw | Same opaque-line pipeline. Ring geometry is computed CPU-side; pixel math is identical. |
| **DiskLaser charge-up (ring pair)** | Same, emits two LaserDrawClass per ring step | Same direct-write math; IntensityStart/End fade applies for this variant. | same | Same opaque-line pipeline + optional linear fade from alpha uniform. |
| **PixelFX (ore/gem sparkle)** | `DrawPixelFXSparkles @ 0x006D7840` — direct pixel write | `out = pack16(CurrentR, CurrentG, CurrentB)` — no blend, no Z test, no A-buffer. `Current*` is a triangle-wave lerp between `ColorA` and `ColorB` driven by `PhaseAccumulator`. | (i) viewport check only. No depth, no mask, no alpha. | Sprite atlas with a single 1-pixel "sparkle" quad per cell, or shader-driven point primitive with fragment-time triangle-wave lerp. Always drawn on top. |
| **BombClass clock overlay** | `TechnoClass__DrawExtras @ 0x006F5190` → `CC_Draw_Shape(CHRONOSK_SHP, frame, …, flags=0xE00)` | Standard SHP blitter path through `CC_Draw_Shape @ 0x004AED70`. Flags 0xE00 resolves to opaque remap-blitter (`0x01 bit clear`, `0x02/0x04 Z-bits clear`, `0x800` remap set, `0x200` center set, `0x400` standard). Same per-pixel path as any other SHP draw. | Full SHP pipeline: RLE decode, house-color remap, A-buffer LUT lookup, opaque write. No blending. | Render as a regular sprite entity (SHP→texture), same pipeline as any unit SHP. Frame index = `BombClass__GetClockFrame`. |
| **FlasherClass Elite blink** | NOT a pixel write — only flags `TacticalClass__DirtyScreenRect` + re-invokes `BuildingClass__UpdateAllAnimFacings` every 2 frames | **No dedicated blend math.** The visible "flash" is whatever building anim frame is current at the moment the anim subsystem is forced to reset. | None — pure dirty-rect invalidation + anim state reset | No shader work. Just invalidate the building's drawn sprite every 2 frames for 150 frames. |
| **Bounce (voxel debris physics)** | Host-driven — BounceClass provides state, VoxelAnim/AnimClass handles rendering | No VFX-specific pixel math. The rendered voxel uses the normal VXL pipeline, the AnimClass uses the normal SHP pipeline. | Inherits host's blend | Inherits host's pipeline (VXL or SHP). |
| **IonBlastClass** | Fully dormant in YR. `Rules+0x298 = IonBlast` AnimType is used only by Genetic Mutator, via normal AnimClass draw path. | No dedicated IonBlast VFX. | n/a | Do not implement. |

### The shared line rasterizer — `Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30`

**EBolt, LaserDraw, and RadBeam all share one DirectDraw primary-surface line-draw
routine.** This is the function at vtable slot 0x34 of the DSurface vtable
(`vtable__DSurface @ 0x007E85D4` — verified by reading the raw vtable memory this
pass; slot 0x34 = index 13 resolves to `0x004BFD30`).

**Key properties** (verified from decompilation of `FUN_004BFD30` this pass):

1. **A-buffer-modulated direct write, NOT alpha-over.** The rasterizer reads the
   per-pixel A-buffer value `uVar12 = *puVar9`:
   - If `uVar12 == 0x7F` (neutral shroud): `*dest = color_as_passed`.
   - If `uVar12 != 0x7F` (dimmed by shroud/fog/cloak): `out_R = ((color_R * uVar12)
     >> 7)` per channel, then packed to 16bpp via `g_DD_{R,G,B}_{Loss,Shift}`.
   - If `uVar12 == 0`: pixel skipped entirely.
2. **Per-pixel Z-test** against `g_ZBuffer` via `ZBuffer_scanline_ptr`. The test
   is `*dest_zbuffer > incoming_z` (NOTE: reversed sign convention from modern
   GPU depth — lower Z = closer to camera in this engine).
3. **Optional Z-write** controlled by the boolean `param_5`. EBolt/LaserDraw/RadBeam
   all pass `0` here (no Z-write). The beams participate in the Z-test for
   occlusion behind terrain, but don't write Z themselves.
4. **Bresenham line stepping** over three octant-dispatch branches (major-Y major-X
   swap plus Z interpolation along the line).

**Distinct from `Surface__DrawAlphaLineClippedZ @ 0x004BEAC0`** (LineTrail's rasterizer),
which additionally lerps the source color against the existing destination with
brightness as alpha. The beam rasterizer does NOT do that — it writes the color
straight, with A-buf only as a darkening multiplier.

**In wgpu, these beams need: `depth_test = Less` (or `Greater` depending on your Z
convention), `depth_write = false`, `blend = Replace`, fragment shader multiplies
RGB by sampled A-mask/128 (passing through at A=128, dimming as A→0, discarding
at A=0).**

### Secondary gradient line rasterizer — `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00`

Used by LaserDraw when `DAT_00A8EB78 != 0` (DetailLevel ≥ 1 in Options). Same
A-buf modulation math, but takes two endpoint colors and interpolates between them
per segment. Practical effect: smoother color gradient along the beam under high
detail. At low detail, flat color is used via vtable[0x34] as above.

### Modulated-alpha rasterizer — `Surface__DrawAlphaLineClippedZ @ 0x004BEAC0`

Distinct from the shared line rasterizer. Only LineTrail uses this one. Pixel math
(verified in round 2 of LINE_TRAIL_CLASS_GHIDRA_REPORT.md §11):

```
out = ((lerp(Cd, Cs, a/256) * m) / 128) >> DD_Loss   // per R/G/B channel
  where Cs = LineTrail color (per-point), a = LineTrail brightness (0..255),
        m  = A-buffer value at this pixel (0..255)
gated by: src_z < *zbuf_ptr  AND  m != 0
```

- `a = 255` approaches replace (source); `a = 0` leaves dst unchanged. `m = 128`
  is neutral; `m = 0` skips; `m = 255` doubles (saturates).
- **wgpu:** premultiplied-alpha `OVER` blend with fragment shader multiplying
  output RGB by sampled A-mask/128.

---

## §2 — Z-bias Table and `AdjustForZ` Specification

### `Tactical__AdjustForZ @ 0x006D20E0`

**Verified this pass.** The function is a 3-instruction wrapper that converts a
Z lepton value to a signed pixel offset:

```c
int Tactical__AdjustForZ(int zLeptons) {
    // Leptons → screen pixels: divide by 0x10 (one cell = 256 leptons,
    //                          one cell-height tile = 15 screen pixels isometrically).
    // Actually the source uses `Math__ftol(zLeptons * 15 / 256)` per the
    // integer-math decompile.
    return Math__ftol((zLeptons > 0x2D7) ? clamp_expr : zLeptons * 15 / 256);
}
```

**Verified numeric behavior.** `z = 256` (1 cell high) returns `15`; `z = 0` returns
`0`; negative `z` returns negative offset (pulls sprite DOWN). The cap at `0x2D7` (727
leptons ≈ 2.84 cells) is a hardcoded horizon clamp — sprites above 727 leptons of
elevation get a single fixed screen offset, not higher.

**Label applied this pass:** `FUN_006D20E0` → `Tactical__AdjustForZ`.

### Per-VFX Z-bias table

| VFX type | Z-bias formula | Draws above/below what | Notes |
|----------|----------------|------------------------|-------|
| **LineTrail** per point | `-2 - Tactical__AdjustForZ(pointZ)` | Above terrain at same height; tied to the line's per-point elevation | The `-2` separates it from ground sprites to avoid z-fighting with building roofs |
| **EBolt segments** | `Tactical__AdjustForZ(screenPointZ) - 2` | Above attached source sprite (tesla coil roof, tank hull) | Same `-2` offset; the sign convention differs but end result is "two pixels in front" |
| **LaserDraw** (all paths) | Per-endpoint `Tactical__AdjustForZ(Src.Z) / Tactical__AdjustForZ(Tgt.Z)` — then written to 16bpp surface with `src_z / tgt_z` passed to surface vtable[0x34] as the Z coordinate | Above all units/buildings (drawn AFTER Tactical_ObjectRenderingLoop in TacticalClass_Draw) | Lasers bypass the LayerClass system entirely. No additional `-2` offset. |
| **RadBeam straight** | Per-segment `Tactical__AdjustForZ(Src.Z) + ZOffsetAdjust (bomb+0x0C)` — `ZOffsetAdjust` is pre-computed from source/target screen Y delta | Above units/buildings; same layer position as LaserDraw | The source-side Z-offset-adjust pre-bakes the "lift to top of emitter" bias |
| **PixelFX sparkle** | No Z integration at all — direct surface write ignores `g_ZBuffer` | Always on top of everything drawn so far (runs at pipeline step 24 of ~25) | wgpu: render with `depth_test = Always, depth_write = false` |
| **BombClass clock** | Inherits SHP Z-bias: draw flags `0xE00` sets `z_height = 1000` in CC_Draw_Shape (a large constant → high Z → furthest back). **But** the blitter selected for `0xE00` ignores Z-buffer anyway (0x800 set → flag-dispatch picks the opaque remap blitter which doesn't read or write Z). | On top of carrier sprite — same Y-sort slot as carrier, but drawn later in DrawExtras | wgpu: render as a regular sprite on top of carrier with painter's-algorithm ordering |
| **DiskLaser (both ring pair and terminal beam)** | Rides on LaserDraw's Z-bias | Above units/buildings | Inherits LaserDraw pipeline |
| **Selection brackets / health bars / target lines** | Drawn in the SECOND pass of `Tactical_ObjectRenderingLoop` (vtable+0x110 `DrawExtras`). Each calls `Tactical__AdjustForZ` on the object's own coord. No additional constant offset. | Above all sprites | Renders on top of units |
| **Flat anims / smudges / ground overlays** | `-3 - Tactical__AdjustForZ(z)` (see `AnimClass::DrawIt @ 0x00422CA0`, Flat=true branch) | Below objects (terrain pass, step 8) | The `-3` vs `-2` distinction separates flat anims from live-object VFX |

**Rationale for the constant offsets:** `-2` is the default "one 16bpp pixel in front of
source" bias — it matches the pixel rounding of the isometric Y projection so the VFX
doesn't stack-Z-fight with the emitter sprite. `-3` is reserved for flat anims that
must pass UNDER all objects in the same cell.

**In wgpu the simplest implementation:** pass `AdjustForZ(objectZ) - C` as a uniform
per draw call, where `C` is 2 for beams/projectiles, 3 for flat anims, 0 for UI overlays.
No per-type tuning table is needed.

---

## §3 — A-buffer Specification

This section consolidates `BSURFACE_CIRCBUF_ABUFFER_REPORT.md` and
`SHROUD_FOG_RENDERING_PIPELINE.md` — do NOT redo that work; cited by section.

### What it is

- **A 16-bit-per-pixel auxiliary surface** running parallel to the screen. Global
  pointer `g_ABuffer @ 0x0087E8A4`. Structure: `CircBuf` (0x30 bytes) wrapping a
  `BSurface` (see `BSURFACE_CIRCBUF_ABUFFER_REPORT.md §§1-2` for full struct).
- **Logical value range: 0x00–0x7F** (low byte; the high byte is always 0 in
  practice). `0x7F` is the neutral midpoint; `0x00` is full black; `0x01–0x7E` are
  gradient-darkening values. `0xFE` is a **source-side transparent marker** in SHP
  data — means "skip this pixel" when writing INTO the A-buffer.
- **Dimensions: 480 × (480 − sidebar_offset)** (the tactical viewport only, not the
  full screen).
- **Implemented as a circular buffer** so viewport scrolling is zero-copy: scroll
  increments `circ_offset` and only touches the vacated edge scanlines.

### When/how it's populated

- **Cleared each frame to `0x7F` per dirty rect** by `FUN_00411330` (called from
  `TacticalClass_Draw` step 1). Full rewrite per frame is avoided — only dirty rects
  get re-cleared.
- **Shroud edges** write to it via `ShroudEdge_BlitToABuffer @ 0x0047EFE0`
  (SHROUD.SHP pixel values 0x00–0x7F written directly as A-buffer bytes; 0xFE pixels
  skip). See `SHROUD_FOG_RENDERING_PIPELINE.md §§3-4`.
- **Fog edges** (only when `FogOfWar & 0x1000` is enabled — not stock YR) write to it
  via `FogEdge_BlendToABuffer @ 0x0047F250` using a subtractive blend that pushes
  neutral values darker.
- **AlphaShapeClass decals** (cloaked unit "ghost" circles, etc.) write to it via
  `alpha_blend_table[(*abuffer & 0xFF) + shp_pixel * 256]` which is multiplicative
  per-pixel. See `SHROUD_FOG_RENDERING_PIPELINE.md §§8-9`.
- **FoggedObject alpha compositing** (post-fog object darkening) also uses the
  alpha-blend LUT at `0x0088A118`.

### Who reads it

**Every per-pixel rasterizer in the engine reads the A-buffer at the same
scanline offset as the screen pixel it's about to write.** This includes:

- **Terrain TMP blitter** (`TMP_TileBlitter @ 0x00547CF0`) — A-buffer indexes into a
  per-theater remap table for darkening. See `ZBUFFER_DEPTH_SYSTEM.md §2`.
- **SHP standard blitter** (`FUN_004373B0`) and extended blitter (`FUN_00437A10`) —
  both read A-buffer per pixel alongside Z-buffer.
- **VXL pipeline** — voxel color buffer is blitted to screen via the same SHP
  blitter infrastructure, so A-buffer applies identically.
- **LineTrail `FUN_004BEAC0`** — multiplies by `m/128` (see §1).
- **Shared line rasterizer `vtable[0x34]`** — uses A-buffer as a binary mask only
  (skip if `m == 0`).

### Per-VFX effect

| VFX | A-buffer role |
|-----|---------------|
| LineTrail | Multiplicative dim: `out_rgb = lerp(dst, src, a/256) * (m/128)` — beam fades into shroud |
| EBolt / LaserDraw / RadBeam | Binary mask: beam pixel skipped entirely if shroud covers that screen pixel (`m == 0`) |
| PixelFX | **Not read.** Pixel FX write directly without consulting A-buffer. |
| BombClass clock | Read by standard SHP blitter via `alpha_blend_table[*abuffer + shp_pixel * 256]` → the clock overlay darkens over shroud edges |
| All sprites (units/buildings) | Same SHP blitter path → all sprites darken over shroud edges |

### wgpu mapping

**Implement as an `R8Unorm` fullscreen render target** (call it `abuffer_mask`,
480×H matching tactical viewport):

1. **Clear to 127 (`0x7F / 2` scaled) each frame** — one fullscreen clear instead of
   dirty-rect optimization (GPUs don't benefit from circular buffers).
2. **Render shroud edges and AlphaShape decals into it** via a dedicated shroud
   pipeline that writes sampled SHROUD.SHP pixel values as R8 values (skip on 0xFE).
3. **Sample it in EVERY subsequent draw call's fragment shader.** Multiply output
   RGB by `sample.r * 2.0` (to rescale 0..127 back to 0..254, which produces the
   same visual as the binary's `/128` multiply).
4. **The binary-mask shortcut** (`discard` when sampled == 0) is an optimization
   only; the multiply-by-zero gives the same result.

---

## §4 — Palette Cycling Inventory

Binary exhaustively searched for the usual suspects — `PaletteCycle`, `RotatePalette`,
`CycleColor`, `AnimatePalette`, `PalettePhase`. **No runtime palette-rotation
function exists in gamemd.exe.** The classic TS/RA1 "water indices 16–23 cycle per
frame" mechanism is NOT present as a standalone function in the YR binary.

What IS present:

### 4a. Per-frame AnimClass palette references (NOT global rotation)

- `AnimTypeClass` has three palette INI keys (all at `art(md).ini`):
  - `AltPalette=` (TypeClass offset around `+0x2E5x`, verified string `0x00818638`)
  - `AnimPalette=` (offset around `+0x2E5x`, verified string `0x0081AFD0`)
  - `FirersPalette=` (offset around `+0x2E5x`, verified string `0x0081AFFC`)
- These select one-of-N palettes for specific anims (e.g. tesla discharge, nuke
  glow) — the palette itself is static; the AnimClass frame advances through
  pre-rendered SHP frames using that palette.
- This produces the **visual effect** of "cycling colors" even though no indices
  rotate — the SHP authoring pre-rendered the frames as they should appear.

### 4b. `PixelFXClass` ore/gem twinkle

- Per-cell triangular-wave interpolation between two hardcoded RGB values.
  See `PIXEL_FX_CLASS_GHIDRA_REPORT.md §Rendering path`.
- **NOT a palette cycle** — writes a freshly-computed 16bpp RGB pixel per frame
  via `DrawPixelFXSparkles @ 0x006D7840`. No palette table is touched.
- **wgpu recommendation:** Render as a shader-driven particle with a per-cell random
  phase offset and a triangle-wave intensity curve. Two color endpoints are uniforms
  per tiberium type (entry 0: pure tiberium blue-gray; entry 1: ore gold).

### 4c. Ambient-light / night-scene palette rebuild (NOT cycling)

- `LightConvertClass` (RTTI `0x008169C0`, class size NOT re-measured this pass)
  rebuilds remap tables when the scenario ambient light changes — triggered by
  mission triggers (dusk → night), ChronoStorm darkening, etc.
- This is a **table REBUILD on demand**, not per-frame rotation. Occurs rarely
  (seconds apart, usually just once per scenario-light-change trigger).
- Global — affects every palette-indexed sprite and terrain tile. Does NOT touch
  beams, PixelFX, or BomBombClass clock (those bypass ConvertClass).
- **wgpu recommendation:** Ambient-tint uniform applied in the sprite shader only.
  Skip it for VFX pipelines.

### What does NOT cycle in YR

- **Water animation** on coastal tiles: achieved via multi-frame TMP tileset
  animation (each shore tile has ~8 frames pre-rendered in the TMP file), NOT by
  rotating palette indices. Verified by searching for any per-frame palette write
  — none found.
- **Tiberium growth shimmer**: similar — AnimClass anims with multi-frame SHP
  sequences, not palette rotation.
- **EBolt / LaserDraw / RadBeam colors**: hardcoded into PALETTE.PAL LUT indices
  10/6/15 for EBolt; raw RGB from weapon INI for LaserDraw; `RadColor` RGB for
  RadBeam. **None of these cycle.** Verified no per-frame write-back to the
  palette memory.

### Impact for Rust port

- Do NOT implement a "palette cycle" system in the engine.
- DO implement multi-frame TMP tileset animation (tile frames indexed by a
  global tick counter modulo frame count).
- DO implement per-AnimType palette selection via `AltPalette/AnimPalette`.
- DO implement `PixelFXClass` twinkle as a per-cell shader effect.
- DO implement ambient-light-level tint as a shader uniform applied only to
  indexed-palette sprites (not VFX beams).

---

## §5 — Dithering Assessment

**Summary: No dithering in any VFX draw path.** All blends use integer
multiply-and-shift arithmetic; the 16bpp banding is pure bit truncation.

### Per-VFX evidence

| VFX | Dither? | Evidence |
|-----|---------|----------|
| **LineTrail `Surface__DrawAlphaLineClippedZ`** | **NO** | Decompile shows integer multiply + bit-shift only. No LUT lookup for a dither matrix, no XOR with a coordinate-derived pattern. (Verified in `LINE_TRAIL_CLASS_GHIDRA_REPORT.md §11`.) |
| **EBolt / LaserDraw / RadBeam shared `vtable[0x34]`** | **NO** | Direct pixel store, no source mixing. Outer halo in LaserDraw comes from additional parallel line passes with halved colors, not dither. |
| **LaserDraw outer spread jitter** | **NO** (though visually similar) | `LaserOuterSpread` adds `RandomRanged(-spread, +spread)` to the outer-line RGB **once per frame**, not per pixel. This is a temporal shimmer on a per-beam basis, not a per-pixel ordered dither pattern. |
| **RadBeam sine path** | **NO** (dormant anyway) | Sine path uses per-segment color dither via `Random` — temporal, not ordered. Same kind as LaserDraw spread. |
| **PixelFX** | **NO** | Direct RGB write with no dither lookup. |
| **SHP alpha blitters (cloak shimmer, semi-transparent)** | **NO** | The 25/75, 50/50, and 75/25 blend blitters (used for cloaked unit rendering — `CLOAKING_VISUAL_PIPELINE.md §Blitter Per-Pixel Operations`) are all pure multiplicative blends: `(src >> N & mask) + (dst >> M & mask)`. No dither pattern. |
| **AlphaShapeClass (cloaked unit "ghost" decals)** | **NO** | Uses the 64KB `alpha_blend_table @ 0x0088A118` — a straight `(src * dst) / 127` LUT, not a dither pattern. |
| **TMP tile blitter (terrain)** | **NO** | Per-pixel multiplicative lookup through a theater remap table; no dither. |

### No blue-noise / no Bayer / no error diffusion

Searched binary for common dither constants (0x55, 0xAA, hardcoded 4×4 Bayer
patterns) — none found in VFX paths. The engine's 16bpp banding is intentional
raw truncation.

### wgpu recommendation

- **No dither pattern is needed for VFX pixel parity.** Render in linear RGBA and
  let the surface format handle banding.
- For the sprite pipeline, match the engine's remap-via-LUT mechanism. Dithering
  is NOT required for parity and would actually PRODUCE DEVIATION from retail.
- If banding is a concern on modern displays, consider enabling **framebuffer-level**
  dither (many wgpu surface formats support optional dither during format conversion)
  but keep the VFX shaders themselves free of any dither logic.

---

## Appendix A — Functions Labeled This Pass

All renames via `rename_function_by_address`; `save_program` called at end.

| Address | Old | New name | Purpose |
|---------|-----|----------|---------|
| `0x006D20E0` | `AdjustForZ` | `Tactical__AdjustForZ` | Added module prefix — called by EBolt (6x), LaserDraw (4x), LineTrail (2x), RadBeam (4x), plus shroud/reveal code (90 total xrefs) |
| `0x004BFD30` | `FUN_004bfd30` | `Surface__DrawLine_ABufModulated_ZClipped` | **Main finding** — the DSurface vtable[0x34] line-draw routine shared by EBolt, LaserDraw, and RadBeam. A-buf-modulated direct write with per-pixel Z test |
| `0x004BDF00` | `FUN_004bdf00` | `Surface__DrawLineGradient_ABufModulated_ZClipped` | DSurface vtable[0x40] — two-color gradient line, used by LaserDraw at DetailLevel ≥ 1 |

Pre-existing labels confirmed (not modified):
- `Surface__DrawAlphaLineClippedZ @ 0x004BEAC0` — the LineTrail modulated-alpha rasterizer
- `EBolt__DrawRecursiveBolt @ 0x004C1F20`
- `LaserDrawClass__Draw @ 0x00550260`, `LaserDrawClass__DrawBeamSpecial @ 0x005509F0`
- `RadBeam__DrawStraightBeam @ 0x00659650`, `RadBeam__ComputeStraightSegment @ 0x00659AC0`
- `RadBeam__DrawSineBeam @ 0x00659CA0` (dormant in YR)
- `DiskLaserClass__Constructor @ 0x004A7A30`, `DiskLaserClass__AI @ 0x004A7340`
- `DrawPixelFXSparkles @ 0x006D7840`
- `TacticalClass_Draw @ 0x006D3D10`

Key global references for implementation:
- `g_PrimarySurface @ 0x0088731C` — the DSurface* whose vtable holds the line rasterizers
- `vtable__DSurface @ 0x007E85D4` — verified this pass by reading the write in DSurface constructor at 0x004BA5DC
- `g_ABuffer @ 0x0087E8A4` — the 16bpp shroud/alpha mask surface
- `g_ZBuffer @ 0x00887644` — the 16bpp depth surface

## Appendix B — Source Documents Cited

- `BSURFACE_CIRCBUF_ABUFFER_REPORT.md` — ABuffer / ZBuffer struct layout, CircBuf wrapper
- `ZBUFFER_DEPTH_SYSTEM.md` — Z-buffer infrastructure, gradient table, terrain Z
- `DRAW_ORDER_DEPTH_SYSTEM.md` — tactical draw phase order, layer system
- `SHROUD_FOG_RENDERING_PIPELINE.md` — ABuffer population, shroud edges, alpha blend table
- `CLOAKING_VISUAL_PIPELINE.md` — cloak shimmer blitters, alpha blend math
- `LINE_TRAIL_CLASS_GHIDRA_REPORT.md` — LineTrail rasterizer + brightness×A-buffer blend
- `EBOLT_SYSTEM_GHIDRA_REPORT.md` — EBolt recursive midpoint drawer, LUT indices 10/6/15
- `LASER_DRAW_CLASS_GHIDRA_REPORT.md` — LaserDraw struct, DrawBeamSpecial, color flow
- `RAD_BEAM_CLASS_GHIDRA_REPORT.md` — RadBeam struct, straight + sine paths, dormancy of sine
- `DISK_LASER_CLASS_GHIDRA_REPORT.md` — DiskLaser AI loop, laser emission chain
- `PIXEL_FX_CLASS_GHIDRA_REPORT.md` — Ore/gem sparkle direct-write, color table, DetailLevel gate
- `BOMB_CLASS_GHIDRA_REPORT.md` — BombClass clock, CHRONOSK.SHP, frame picker
- `FLASHER_CLASS_GHIDRA_REPORT.md` — Elite flash = dirty-rect + anim reset (not a pixel effect)
- `BOUNCE_CLASS_GHIDRA_REPORT.md` — Bounce physics component, no direct pixel write
- `ION_BLAST_CLASS_GHIDRA_REPORT.md` — IonBlastClass dormant; IonBlast AnimType live via Genetic Mutator

---

## Verification (round 3)

### Claim 4 — Beam rasterizer is A-buffer-modulated direct write (not alpha-over)

**Claim under review:** `Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30`
(shared by EBolt / LaserDraw / RadBeam) does `(color × abuf) >> 7` per channel
into dst — NOT a lerp/alpha-over. When abuf = 0x7F (neutral), dst pixel is simply
replaced. Also: `Surface__DrawAlphaLineClippedZ @ 0x004BEAC0` (LineTrail's
rasterizer) is DIFFERENT — it does lerp-against-dst.

**Independent evidence (decompiled both functions, per-pixel inner loop extracted):**

**`Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30`:**
```c
uVar12 = (uint)*puVar9;                      // abuf at this pixel
if (((ushort)param_4 < *puVar8) && (uVar12 != 0)) {  // Z-test AND abuf != 0
    uVar16 = uVar18;                         // default = raw color
    if (uVar12 != 0x7f) {                    // abuf != neutral → scale color
        uVar16 = (R_comp * uVar12 >> 7) |
                 (G_comp * uVar12 >> 7) |
                 (B_comp * uVar12 >> 7);     // pack back to 16-bit
    }
    *(ushort *)(iVar22 + (int)param_3) = uVar16;  // DIRECT WRITE
    if ((char)param_5 != '\0') *puVar8 = (ushort)param_4;  // optional Z write
}
```
The destination pixel is **never read**. It is overwritten with either the raw
input color (when abuf == 0x7f) or the color scaled by abuf/128 per channel.
**Confirmed: replace-style, not alpha-over.**

**`Surface__DrawAlphaLineClippedZ @ 0x004BEAC0` (LineTrail):**
```c
uVar1 = *(ushort *)(param_4 + iVar15);       // READ DEST PIXEL
cVar2 = (char)(uVar1 >> g_DD_GShift) << _g_DD_GLoss;
bVar3 = (char)(uVar1 >> g_DD_BShift) << _g_DD_BLoss;
// iVar17 = 256 - alpha; iVar6/iVar8/iVar9 = color_component × alpha >> 8
*(ushort *)(...) = R_out | G_out | B_out,
    where G_out = (((dst_G × iVar17 >> 8) + color_G_scaled) × abuf >> 7) ...
```
The destination component is multiplied by `(256 - alpha)`, added to
`(color × alpha)`, and then the sum is further modulated by the A-buffer value.
This is a true alpha-over blend (`lerp(dst, color, alpha/256)`) further scaled
by abuf for shroud/occlusion. **Confirmed: DIFFERENT from the beam rasterizer.**

**Verdict: CONFIRMED REPLACE-STYLE** for the beam rasterizer; **confirmed
DIFFERENT** from the LineTrail rasterizer. Round 2's claim holds exactly.

Ghidra MCP calls: decompiled `Surface__DrawLine_ABufModulated_ZClipped` and
`Surface__DrawAlphaLineClippedZ` in full.

---

### Claim 5 — No palette cycling exists in YR

**Claim under review:** Searched `PaletteCycle`, `RotatePalette`, `CycleColor`,
`AnimatePalette`, `PalettePhase` — all zero string hits. No per-frame palette
writes in `LogicClass::PerTickUpdate`. What looks like cycling is TMP tileset
animation + AltPalette anims + PixelFX twinkle.

**Independent evidence (round-3 broader sweep):**

(a) **Additional string searches:** `CyclePalette`, `AnimatePalette`,
`RotatePalette`, `PaletteCycle`, `PalettePhase`, `AdjustHSV`, `AmbientColor`,
`IonStormAmbient`, `LightTint`, `shimmer`, `Tint` (as key) — **all zero string
hits** in `.rdata`/`.data` sections.

(b) **LogicClass::PerTickUpdate @ 0x0055AFB0** — decompiled in full. Contains:
- cell-action processing (missions/triggers)
- screen flash (`FUN_004f42f0`) for earthquake / lightning storm / ion blast
- bridge shroud recalculation (every 120 frames)
- `TiberiumClass__GrowthDriver_AllTypes` + `SpreadDriver_AllTypes` (ore growth)
- `BombClass::UpdateAll`, DiskLaser updates, `LaserDrawClass::UpdateAllAI`,
  `LightningStorm::Process`, EMPulse, HouseClass updates.

**No call into any palette-mutation routine. No per-tick palette write.**

(c) **Palette structure writers at runtime:** `PaletteLoad @ 0x0072F350` is the
only palette I/O function named. Its only caller is `InitSideMixFiles`
(startup/side-switch), not per-tick. `CreatePalettedPreview @ 0x00642130` is a
preview-image function, one-shot.

(d) **Candidate functions for cycling:** searched function names for `Cycle`,
`Shimmer`, `Tint`, `AdjustHSV` — only `Blitter_Shimmer_75pct_Remap @ 0x00494330`
matches, and it is a blitter (read-only w.r.t. palettes; it uses the
already-loaded remap table).

(e) **PixelFX sparkles (`DrawPixelFXSparkles @ 0x006D7840`)** writes DIRECTLY to
the primary surface framebuffer at random ore/water pixels — it does not modify
any palette. Confirmed by reading the decompile: the per-pixel write uses
`*(ushort *)(iVar4 + iVar6 * iVar11 + iVar10 * 2) = ...` targeting the locked
DirectDraw surface.

**Verdict: CONFIRMED NO CYCLING.**
No palette-cycling subsystem exists in vanilla YR. The ore/water "shimmer" look is
produced by `PixelFXClass` direct framebuffer writes plus AltPalette anim frames
(which reference different pre-baked palette files, not a mutated one). Round 2's
finding holds.

Ghidra MCP calls: string-searched palette-mutation keywords; decompiled
`LogicClass::PerTickUpdate`, `DrawPixelFXSparkles`; function-searched `Palette`,
`Cycle`, `Shimmer`, `Tint`, `AdjustHSV`, `AmbientColor`; checked callers of
`PaletteLoad`.
