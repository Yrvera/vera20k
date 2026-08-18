# Voxel palette texture sRGB format fix — Design

## Goal

Restore correct gamma round-trip on the voxel rendering pipeline by switching
`palette_tex` and `house_ramp_tex` from `Rgba8Unorm` to `Rgba8UnormSrgb`, so
that .pal-derived display bytes are sRGB-decoded on sample (matching
pre-Phase-1 atlas semantics).

## Architecture Context

### Pre-Phase-1 (working) path

- Voxel atlas: `Rgba8UnormSrgb`. The rasterizer wrote
  `palette.colors[idx]` bytes directly into atlas RGBA — those bytes are
  `.pal`-file display values (sRGB-encoded in display space, since gamemd
  writes them directly to the framebuffer with no gamma math).
- GPU sampler on `Rgba8UnormSrgb`: applies sRGB → linear decode in hardware
  on every `textureSample`.
- Fragment shader: `color.rgb * tint` runs on linear values (correct).
- Surface format: `gpu.surface_format` is chosen with
  `surface_caps.formats.iter().find(|f| f.is_srgb())`
  ([gpu.rs:91-96](../../src/render/gpu.rs#L91-L96)) — on Windows AMD this is
  `Bgra8UnormSrgb`.
- Output stage: linear → sRGB encode in hardware on write.
- Net: the on-screen byte equals the input `.pal` byte.

### Post-Phase-1 (broken) path

- Atlas: `R8Uint` (correct — stores palette indices, not colors).
- New color lookup textures:
  - `palette_tex`: `Rgba8Unorm` ([palette_textures.rs:76](../../src/render/palette_textures.rs#L76))
  - `house_ramp_tex`: `Rgba8Unorm` ([palette_textures.rs:112](../../src/render/palette_textures.rs#L112))
- Both fed with the same raw `.pal` bytes via `build_palette_bytes` /
  `build_house_ramp_bytes` ([palette_textures.rs:243-284](../../src/render/palette_textures.rs#L243-L284)).
- Shader `textureLoad(palette, ...)` on `Rgba8Unorm` returns `byte/255.0`
  treated as **linear** — no sRGB decode.
- Tint multiply happens against a value that is actually sRGB-encoded
  (wrong space).
- Output to sRGB surface: hardware applies sRGB encode anyway.
- Net: one extra gamma encode on the way out. Palette byte 100 (≈0.392
  linear) becomes ≈0.673 ≈ byte 172 on screen. Voxel units render with a
  uniform white wash compared to pre-Phase-1.

### Touched components

- `src/render/palette_textures.rs` — the two texture creations + doc
  comments referencing the format.
- `src/render/sprite_voxel_shader.wgsl` — one binding-comment reference.
- Nothing else. The shader code itself, the atlas pipeline, the FX stub,
  the tint logic, the alpha logic, and `SpriteInstance` are all already
  correct.

## Impact Analysis

- **Blast radius:** voxel sprite pipeline only. SHP path is untouched
  (still on the old `Rgba8UnormSrgb` colored atlas + `batch_shader.wgsl`).
- **Format usage check:** `Rgba8UnormSrgb` requires
  `TEXTURE_BINDING | COPY_DST` usage flags. Both palette textures already
  declare exactly those flags ([palette_textures.rs:77,113](../../src/render/palette_textures.rs#L77)).
  No usage change needed.
- **Bind group layout check:** `TextureSampleType::Float { filterable: false }`
  is already in use ([palette_textures.rs:156,166](../../src/render/palette_textures.rs#L156)).
  Both `Rgba8Unorm` and `Rgba8UnormSrgb` are valid float-sampled formats.
  No bind group / sampler change needed.
- **Existing tests:** `build_house_ramp_row0_mirrors_theater_palette_range`,
  `build_house_ramp_unused_rows_zero`, `build_house_ramp_house_row_populated`
  ([palette_textures.rs:309-348](../../src/render/palette_textures.rs#L309-L348))
  all assert raw bytes in the upload buffer. Texture format is irrelevant to
  buffer contents — only to how the GPU interprets bytes on sample. **Tests
  pass unchanged.**
- **Color-0 invariant test** (commit 6854fc9): unaffected — it tests atlas
  tiles, not the palette texture.
- **Determinism:** Render-side only. No sim/ touch. No state hash impact.
- **Performance:** Hardware sRGB decode is free on every modern GPU
  (dedicated path in the texture sampler).
- **Migration / compatibility:** None. Local change, no on-disk asset or
  save format affected.

## Chosen Approach

**Texture format swap** — change `Rgba8Unorm` → `Rgba8UnormSrgb` for both
palette textures. Shader code unchanged.

Selected over the two alternatives because:

- Matches the pre-Phase-1 pattern exactly (which was verified-correct).
- Free hardware decode; no per-fragment ALU cost.
- Single failure mode (texture format), not two call sites that can drift.
- Preserves 8-bit sRGB quantization curve — better near-black precision
  than 8-bit linear, important because voxel shading produces a lot of
  dark shadow indices.

## Tiny-Detail Ledger

Every item the fix must preserve:

- **Raw `.pal` byte → on-screen RGB identity.** Source: pre-Phase-1
  atlas was `Rgba8UnormSrgb` ([commit 765dfe0:src/render/batch.rs:856](../../src/render/batch.rs))
  + sRGB surface ([gpu.rs:91-96](../../src/render/gpu.rs#L91)). gamemd
  writes palette bytes directly to framebuffer; our final pixel must
  equal that byte. [verified — diagnosis 2026-05-11]
- **Tint multiply in linear space.** `color.rgb * in.tint` at
  [sprite_voxel_shader.wgsl:133](../../src/render/sprite_voxel_shader.wgsl#L133);
  tint range `[0, TOTAL_AMBIENT_CAP=2.0]` ([lighting.rs:23,29](../../src/map/lighting.rs#L23)).
  After the fix, `rgb` enters the multiply already sRGB-decoded (linear),
  preserving the lighting falloff/boost curve. [ini: rulesmd.ini
  ExtraUnitLight=, ExtraAircraftLight=]
- **Byte 0 → `discard`** ([sprite_voxel_shader.wgsl:118](../../src/render/sprite_voxel_shader.wgsl#L118)).
  Unaffected. [verified test: render/vxl color-0 invariant, commit 6854fc9]
- **Bytes [16, 32) → `house_ramp`; rest → `palette`**
  ([sprite_voxel_shader.wgsl:125-131](../../src/render/sprite_voxel_shader.wgsl#L125-L131)).
  Both textures must get the format change in lockstep — they store the
  same kind of data (.pal-derived display bytes) and must decode
  identically. [verified — palette_textures.rs source]
- **`house_ramp` row 0 = theater palette [16, 32) range.**
  ([palette_textures.rs:262-270](../../src/render/palette_textures.rs#L262-L270)).
  No-remap fallback path; sampling row 0 reproduces the raw palette
  colors for remap-range bytes. Format change applies identically.
  [verified test: `build_house_ramp_row0_mirrors_theater_palette_range`]
- **Upload-byte assertion tests** continue to pass — texture format does
  not change the bytes in the COPY_DST upload, only sampler interpretation.
  [verified — wgpu API contract]
- **Texture usage flags** (`TEXTURE_BINDING | COPY_DST`) compatible with
  `Rgba8UnormSrgb`. [verified — wgpu format table]

## Design

### Components

1. `src/render/palette_textures.rs`
   - `palette_tex` creation: `format: wgpu::TextureFormat::Rgba8UnormSrgb`
     (was `Rgba8Unorm`).
   - `house_ramp_tex` creation: `format: wgpu::TextureFormat::Rgba8UnormSrgb`
     (was `Rgba8Unorm`).
   - Update three doc-comment references on the `PaletteSet` struct that
     name the format (lines 40, 43-44 — change "Rgba8Unorm" → "Rgba8UnormSrgb").
   - Update the module-level `//!` header note that mentions `Rgba8Unorm`
     for both textures.

2. `src/render/sprite_voxel_shader.wgsl`
   - Update the comment block listing bind group 2 contents
     ([line 15](../../src/render/sprite_voxel_shader.wgsl#L15)) — change
     "(Rgba8Unorm)" → "(Rgba8UnormSrgb)" in both occurrences.

### Interfaces / Contracts

- Public API of `PaletteSet` (struct fields, `new()`, `rebuild_house_ramps()`)
  is unchanged.
- Bind group layout binding indices, sample types, and sampler kind
  unchanged.
- Bytes uploaded via `build_palette_bytes` / `build_house_ramp_bytes`
  unchanged.
- Net externally observable change: the GPU sampler decodes sRGB on read
  instead of treating bytes as linear.

### Data Flow

`.pal` bytes (sRGB display values)
  → `build_palette_bytes` / `build_house_ramp_bytes` (passes bytes through unchanged)
  → `queue.write_texture` into `Rgba8UnormSrgb` texture
  → shader `textureLoad(palette, ...)` returns **linear** RGB (hardware decoded)
  → multiply by tint (linear)
  → `apply_fx` (linear)
  → write to sRGB surface → hardware sRGB encode on output
  → on-screen pixel == original `.pal` byte (identity restored)

### Error Handling

None. The format swap cannot fail at runtime; if wgpu somehow rejected
`Rgba8UnormSrgb` (it won't — universal support) the device would panic at
texture creation, which is the appropriate behavior for a GPU init failure.

### Testing Strategy

- **Existing unit tests in `palette_textures.rs`** continue to pass
  unchanged (they assert upload bytes, not texture interpretation).
- **Visual regression check:** boot a skirmish map and confirm voxel
  units (Grizzly, Rhino, IFV, Apocalypse, defense turrets) no longer
  show the white wash and match the reference pre-Phase-1 appearance.
- **Color-0 invariant test** (commit 6854fc9) continues to pass —
  it operates on atlas tiles, not palette textures.
- **Optional pixel-level A/B:** screenshot a known voxel pixel on a
  neutral-lighting cell and confirm the on-screen RGB matches the
  source palette byte for that index to within ±1 (allowing for
  rounding). Not required for landing, useful if any drift is
  suspected later.

No new unit tests needed — the bug is in the format choice, not in any
testable Rust logic. A test that round-trips a known palette byte
through the GPU and back would require headless rendering machinery we
don't have set up for this pipeline and isn't worth building for a
two-line fix.

## Architectural Decisions

- **Pattern followed:** "Store sRGB-encoded display bytes in a sRGB
  texture format; let the GPU sampler handle decode in hardware." This
  is the same pattern the SHP path uses (`Rgba8UnormSrgb`) and the
  pre-Phase-1 voxel atlas used. We are restoring the convention, not
  introducing a new one.
- **Pattern rejected (shader-side decode):** Would have created a "manual
  gamma" pattern with no precedent in the codebase and two call sites
  that can drift on future shader edits.
- **Pattern rejected (pre-linearize at upload):** Would have made the
  texture bytes no longer match the `.pal` source bytes, diverging from
  gamemd's mental model, and would have introduced near-black
  precision loss from 8-bit linear quantization.
- **Tech debt introduced:** None.
- **Tech debt resolved:** Implicit assumption in the Phase 1 plan that
  `Rgba8Unorm` was the right format for `.pal`-byte storage is corrected.
  The plan ([2026-05-10-voxel-gpu-remap-fx-plan.md](2026-05-10-voxel-gpu-remap-fx-plan.md))
  specified `Rgba8Unorm` in multiple places without a gamma rationale;
  this design supersedes those format choices.

## Alternatives Considered

- **Approach B — shader-side `srgb_to_linear()` after `textureLoad`.**
  Rejected: per-fragment ALU cost (negligible but non-zero), two call
  sites that can drift on future edits, introduces a manual-gamma
  pattern with no other example in the engine. No upside — FX layer
  doesn't need raw byte access (verified: `apply_fx` operations are all
  RGB-space).
- **Approach C — pre-linearize at upload.** Rejected: 8-bit linear
  quantization has worse precision in dark colors than 8-bit sRGB,
  which would produce visible posterization in voxel shadow regions
  (where palette indices cluster densely). Also surprising semantics —
  `palette_tex` bytes would no longer equal `.pal` bytes.
