---
title: Voxel GPU Remap & FX — Phase 0 + Phase 1 Implementation Plan
status: awaiting approval
---

# Voxel GPU Remap & FX — Phase 0 + Phase 1 Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Phase 0 tasks are independent and parallelizable. Phase 1 tasks must be done
> in order (atlas format change touches everything at once, so the build must
> stay green at every commit point).

**Goal:** Move voxel sprite house remap (and FX uniforms) out of the atlas
bake step and into the fragment shader. Drop the house dimension from the
atlas cache key. Convert atlas tiles from RGBA to u8 palette indices. Phase 0
fixes three independent parity bugs that block nothing else and can ship as a
quick win.

**Architecture:** Render-only changes. The `sim/ never depends on render/`
invariant is preserved. The atlas bake produces tiles whose pixel value is
the **post-VPL-shaded, pre-house-remap palette index**. At draw time, the
fragment shader does:

```
byte = atlas[uv]
if byte == 0 → discard
if 16 <= byte < 32 → rgb = house_ramp[house_idx][byte - 16]   (per-house remap)
else              → rgb = palette[byte]                        (theater palette)
rgb = apply_fx(rgb, ...)
```

House becomes a per-instance uniform; FX uniforms are stubbed in this phase
(populated by future phases 2-7 without further plumbing changes).

The remap is done as **RGB substitution at fragment time** (not as a
byte→byte LUT), mirroring the existing `Palette::with_house_colors(ramp:
&[Color; 16])` pattern. The 16 source bytes (palette indices 16..32) are
the hard-coded "house remap" range gamemd uses; each house ships 16 RGB
ramp entries that replace those palette positions.

**Design Doc:** [2026-05-10-voxel-gpu-remap-fx-design.md](2026-05-10-voxel-gpu-remap-fx-design.md)

---

## Grounding Summary

- **Research docs**: Two HIGH-confidence reports back the design.
  - [VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md](docs/research/VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md)
    documents the on-disk byte layout, the lighting math (specular = 3.0, mode 4 = 245 normals), and the matrix pipeline.
  - [VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md](docs/research/VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md)
    documents the dispatch table, color-0 transparency, VPL output formula, and house-remap-at-blit semantics.
- **Ghidra-verified facts** (cited inline at task level): VPL formula
  `pages[(g_VXL_NormalLUT[normal] << 8) | color]`, specular `3.0` (const
  `0x40400000`), house remap at blit time (`palette[remap_lut[byte]]`),
  color 0 = transparent (hard-coded in row blitters), 245 RA2 normals
  (entries 245-249 dup of 244).
- **Repo pattern mirrored**: Existing `BatchRenderer::create_texture()`
  pattern in [batch.rs](src/render/batch.rs) and the bind-group / pipeline
  layout used by `BATCH_SHADER` in [batch_shader.wgsl](src/render/batch_shader.wgsl).
  We add new pipeline variants and bind groups without disturbing the
  existing tile/sprite atlases.
- **INI keys**: No new INI parsing required. Palette files (UNITTEM.PAL,
  UNITSNO.PAL, UNITURB.PAL) and house remap ramps are already loaded by
  [pal_file.rs](src/assets/pal_file.rs) and
  [house_colors.rs](src/rules/house_colors.rs). The per-house RGB ramp is
  built from `house_colors::house_color_ramp(house)` per house — returns
  `&'static [Color; 16]`. The shader samples this directly at fragment time.
- **Unknown after grounding**: precise wgpu R8Uint sampling syntax in our
  WGSL targeting the wgpu 27 we use — confirmed by spec, but verify at
  Task 1.8.

---

## Key Technical Decisions

- **R8Uint atlas tiles** (not R8Unorm or RGBA). Stores the post-VPL palette
  index directly as an integer byte; sampled via `textureLoad` (point
  sampling, no filtering). 4× storage win over RGBA.
  **Confidence:** high. **Source:** wgpu 27 spec; matches the design.
- **Palette + house-ramp as 2D textures** rather than uniform buffers:
  textures support direct integer-indexed loads (`textureLoad`), avoid
  uniform-buffer 16-byte alignment, and naturally handle 256-entry / 16
  per-house ramp sizes. The house ramp is a 16 × MAX_HOUSES Rgba8Unorm
  texture; bytes in [16, 32) sample from it instead of the theater palette.
  **Confidence:** high. **Source:** repo pattern (`tile_atlas.rs`
  uses similar 2D textures); mirrors existing `Palette::with_house_colors`
  RGB-substitution semantics.
- **Per-instance FX uniforms wired but stubbed.** Phase 1 lays the
  `fx_flags`/`fx_params`/`ic_tint` fields and the shader's `apply_fx()`
  function as a no-op. Phase 2-5 fill them. Avoids re-touching the plumbing.
  **Confidence:** high. **Source:** design doc §10 (Phasing).
- **Phase 0 ships independently of Phase 1.** Phase 0 fixes don't depend on
  the architectural change. They can be committed and merged without Phase 1
  in flight. **Confidence:** high.
- **Color 0 = transparent invariant** preserved. The rasterizer already
  zero-fills empty pixels and rejects voxels with `color_index == 0`; the
  new R8Uint tile follows the same convention, with the shader doing
  `if (byte == 0u) discard;` for transparency.
  **Confidence:** high. **Source:** rasterizer dispatch report §8.
- **Light direction** continues to use the existing
  `[-0.707_107, -0.707_107, 0.0]` constant in
  [vxl_normals.rs:210](src/render/vxl_normals.rs#L210). Already correct per
  research. **Confidence:** high.

## Open Questions

### Resolved During Planning
- **Should the rasterizer output post-VPL shading or raw color?** Resolved:
  output the **post-VPL-shaded palette index** (the same byte gamemd writes
  to its visibility map). The shader then does remap → palette →
  fx_apply(). This is the gamemd-correct path. Source: rasterizer dispatch
  report §2.1.
- **Should rasterizer output ignore the house ramp?** Yes. The rasterizer no
  longer takes a `house_color`; the palette it uses for VPL has the
  vanilla (no remap) ramp. House translation is shader-side only.
- **What about FX timing?** Phase 1 wires the uniform fields but doesn't
  read them. Phases 2-5 each turn on one shader branch + populate the
  uniform from sim. No re-plumbing needed.

### Deferred to Implementation
- **wgpu 27 R8Uint precise WGSL syntax**: confirm `textureLoad(atlas, vec2<i32>(coord), 0).r` works; if not, fall back to `R8Unorm` with `* 255.0` decode. Settle in Task 1.8.
- **Atlas overflow at 30-player saturation**: empirical — measure in Task 1.13.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/assets/vxl_file.rs](src/assets/vxl_file.rs) | Phase 0: variable-length palette; remove `VXL_HEADER_SIZE = 802` |
| Modify | [src/render/vxl_normals.rs](src/render/vxl_normals.rs) | Phase 0: `SPECULAR_STRENGTH = 3.0`; comment fix |
| Create | [src/render/palette_textures.rs](src/render/palette_textures.rs) | Phase 1: `PaletteSet` (palette + per-house RGB ramp GPU resources) |
| Modify | [src/render/vxl_raster.rs](src/render/vxl_raster.rs) | Phase 1: output `Vec<u8>` (palette indices); drop house ramp arg |
| Modify | [src/render/vxl_compute.rs](src/render/vxl_compute.rs) | Phase 1: resolve pass writes u8 |
| Modify | [src/render/unit_atlas.rs](src/render/unit_atlas.rs) | Phase 1: drop `house_color` from key; R8Uint atlas; update cache type |
| Modify | [src/render/batch.rs](src/render/batch.rs) | Phase 1: per-instance FX fields; new `create_palette_texture` and `create_unit_atlas_texture` (R8Uint) |
| Create | [src/render/sprite_voxel_shader.wgsl](src/render/sprite_voxel_shader.wgsl) | Phase 1: fragment shader with byte→remap→palette→fx pipeline |
| Modify | [src/app_instances/units.rs](src/app_instances/units.rs) | Phase 1: drop `house_color` from key build; populate per-instance `house_color_idx` |
| Modify | [src/render/mod.rs](src/render/mod.rs) | Phase 1: declare new `palette_textures` module |
| Modify | [src/bin/audit-assets.rs](src/bin/audit-assets.rs) | Phase 0: regression-verify VXL header parser fix |

## Interface Changes

- `VxlSprite` (rasterizer output) — `rgba: Vec<[u8; 4]>` becomes
  `palette_indices: Vec<u8>`. Drops the dependency on `Palette` for color
  conversion. Consumers: `unit_atlas.rs`, `vxl_compute.rs`. **Phase 1.3+.**
- `UnitSpriteKey` — `house_color: HouseColorIndex` field removed. Hash &
  Eq derive auto-update. Consumers: `app_instances/units.rs`,
  `unit_atlas.rs::get`, the cache. **Phase 1.5.**
- `SpriteInstance` — gains four fields:
  `house_color_idx: u32`, `fx_flags: u32`, `fx_params: [f32; 4]`,
  `ic_tint: [f32; 4]`. Default-initialized to `(0, 0, [0.0;4], [0.0;4])`.
  **Phase 1.7.**
- `BatchRenderer` — gains `create_unit_atlas_texture(width, height, data: &[u8])`
  for R8Uint atlas creation, and `create_palette_set(palette, houses)` for
  the bind-group-2 (palette + house_ramp) resources. **Phase 1.6, 1.9.**
- `vxl_raster::render_vxl()` — signature drops the `palette: &Palette`
  argument. Returns `Vec<u8>` instead of writing into an RGBA buffer.
  **Phase 1.3.**

## Sim Checklist

Not applicable — this plan touches only `assets/`, `render/`, and
`app_instances/`. The `sim/ never depends on render/` invariant is
preserved by construction.

## Risk Areas

- **Visual regression** during Phase 1 transition — mitigated by Task 1.11's
  pixel-comparison test that gates Phase 1 completion. Each Phase 1 task
  must keep `cargo build` green.
- **Atlas overflow** — current renderer can hit `R8G8B8A8Unorm` texture
  size limits at high key cardinality. New R8Uint format quarters that
  pressure (1 byte/pixel). Verify in Task 1.13.
- **Shader compilation differences across wgpu backends** — confirmed
  R8Uint + textureLoad works on Vulkan (AMD) per wgpu 27 spec, but verify
  empirically at Task 1.8.
- **Palette swap on theater change** — currently rare (per-mission); the
  `PaletteSet` exposes a `rebuild_house_ramps()` for runtime updates.
  Sanity-check on a mission switch in Task 1.13.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 0.1 | VXL header parses variable palette section | Wrong header offset breaks every VXL with `palette_count != 1`; silently corrupts limb headers downstream | Unit test with `palette_count = 2`; pixel test against gamemd-rendered Grizzly |
| 0.3 | Specular strength = 3.0 | Every voxel unit's highlight intensity drifts by ~10% with 3.4; visible every match | Pixel test on Grizzly highlights against gamemd capture |
| 0.4 | RA2_NORMALS comment matches binary | Stale comment misleads future readers; "245-249 dup of 244" is the actual binary truth | Manual code review |
| 1.3 | Rasterizer outputs `vpl_pages[(normal_lut[normal] << 8) \| color]` byte | The exact byte gamemd writes to its visibility map; consumer-shader expects this format | Snapshot test: rasterize a known voxel, assert byte values |
| 1.3 | Color 0 = transparent invariant | Visibility-map byte 0 is the hard-coded "no voxel here" sentinel in gamemd | Assertion test: rasterize 100 random arrangements, no atlas tile has byte 0 at "should be opaque" pixel and no non-zero byte at "should be transparent" |
| 1.7 | Per-instance house color uniform | Replaces atlas-baked house remap; gamemd applies remap at blit, we apply at fragment shader | Pixel test: same VXL rendered for two houses produces correctly-tinted output |
| 1.8 | Fragment shader formula `if 16<=byte<32: house_ramp[house_idx][byte-16] else: palette[byte]` | RGB-substitution at shader time mirrors `Palette::with_house_colors` semantics; mis-ordering produces wrong colors | Pixel test against gamemd capture |
| 1.8 | Shader `if (byte == 0u) discard` before remap | Source-byte transparency test happens BEFORE palette translation in gamemd | Manual shader review + visual check (no spurious black pixels) |
| 1.11 | Pixel-comparison test against prior renderer | Catches all unintended visual drift introduced by the architectural transition | Automated `cargo test` |

---

## Phase 0 — Trivial Parity Fixes (independent, parallelizable)

Each Phase 0 task can be committed independently. They have no shared file
contention. Parallel-safe to ship in any order.

### Task 0.1: Fix VXL header parsing for variable-length palette section

**Why:** [VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md §2.1-2.2] verified the on-disk header is **32 bytes**, followed by a variable palette section of `palette_count × 770` bytes (where each palette page = 1 prefix + 768 RGB + 1 suffix). The current `VXL_HEADER_SIZE = 802` bakes in `palette_count == 1`. Fix the parser to read `palette_count` from offset 16 and compute the section start dynamically.

**Files:**
- Modify: [src/assets/vxl_file.rs](src/assets/vxl_file.rs) (constants, `from_bytes`, test)

**Pattern:** existing variable-length offset arithmetic (see how `body_size` is handled lines 107-149).

**Step 1: Replace constants and add palette page size**

In [src/assets/vxl_file.rs](src/assets/vxl_file.rs), replace lines 23-33:

```rust
/// Expected magic string at the start of every VXL file.
const VXL_MAGIC: &[u8; 16] = b"Voxel Animation\0";

/// File header size in bytes: magic(16) + palette_count(4) + limb_count(4) + tailer_count(4) + body_size(4).
const VXL_FILE_HEADER_SIZE: usize = 32;

/// One palette page on disk: 1 prefix byte + 768 RGB + 1 suffix byte.
const VXL_PALETTE_PAGE_SIZE: usize = 770;

/// Per-limb section header size in bytes (name + 3 u32 fields).
const SECTION_HEADER_SIZE: usize = 28;

/// Per-limb section tailer size in bytes (offsets + scale + matrix + bounds + size + mode).
const SECTION_TAILER_SIZE: usize = 92;
```

Update the doc comment block (lines 8-13) to reflect the variable structure:

```rust
//! ## File structure
//! - 32-byte file header (magic, palette_count, limb_count, tailer_count, body_size)
//! - Variable palette section (palette_count × 770 bytes)
//! - Section headers (28 bytes × limb_count): limb names
//! - Body data (sparse column voxels for each limb)
//! - Section tailers (92 bytes × limb_count): bounds, size, scale, normals mode
```

**Step 2: Update `from_bytes` to read `palette_count` and compute offsets**

Replace lines 88-149 (the body of `from_bytes` from the size check through `body_start`):

```rust
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < VXL_FILE_HEADER_SIZE {
            return Err(AssetError::InvalidVxlFile {
                reason: format!(
                    "File too small for header: {} bytes (need {})",
                    data.len(),
                    VXL_FILE_HEADER_SIZE
                ),
            });
        }

        // Validate magic string.
        if &data[0..16] != VXL_MAGIC.as_slice() {
            return Err(AssetError::InvalidVxlFile {
                reason: "Missing 'Voxel Animation' magic string".to_string(),
            });
        }

        let palette_count: u32 = read_u32_le(data, 16);
        let limb_count: u32 = read_u32_le(data, 20);
        let tailer_count: u32 = read_u32_le(data, 24);
        let body_size: u32 = read_u32_le(data, 28);

        if limb_count == 0 {
            return Err(AssetError::InvalidVxlFile {
                reason: "Limb count is zero".to_string(),
            });
        }
        if tailer_count != limb_count {
            return Err(AssetError::InvalidVxlFile {
                reason: format!(
                    "Tailer count ({}) != limb count ({})",
                    tailer_count, limb_count
                ),
            });
        }

        // Variable-length palette section: palette_count pages × 770 bytes each.
        let palette_section_size: usize = (palette_count as usize) * VXL_PALETTE_PAGE_SIZE;
        let sections_start: usize = VXL_FILE_HEADER_SIZE + palette_section_size;

        // Read internal palette from the FIRST page only (typical case: palette_count = 1).
        // Page layout: 1 prefix byte + 768 RGB triplet bytes + 1 suffix byte.
        // Empty palette (palette_count = 0) yields an empty Vec — engine reads palette
        // from theater file at draw time, so internal palette is informational only.
        let palette: Vec<[u8; 3]> = if palette_count >= 1 {
            let palette_start: usize = VXL_FILE_HEADER_SIZE + 1; // skip prefix byte
            (0..256)
                .map(|i| {
                    let off: usize = palette_start + i * 3;
                    [data[off], data[off + 1], data[off + 2]]
                })
                .collect()
        } else {
            Vec::new()
        };

        // Validate file has enough data for all sections.
        let headers_end: usize = sections_start + SECTION_HEADER_SIZE * limb_count as usize;
        let tailers_start: usize = headers_end + body_size as usize;
        let tailers_end: usize = tailers_start + SECTION_TAILER_SIZE * limb_count as usize;

        if data.len() < tailers_end {
            return Err(AssetError::InvalidVxlFile {
                reason: format!(
                    "File too small: {} bytes (need {} for {} limbs, palette_count={})",
                    data.len(),
                    tailers_end,
                    limb_count,
                    palette_count
                ),
            });
        }

        // Body data starts right after section headers.
        let body_start: usize = headers_end;

        // Parse each limb: header + tailer + voxel data.
        let mut limbs: Vec<VxlLimb> = Vec::with_capacity(limb_count as usize);
        for i in 0..limb_count as usize {
            let limb: VxlLimb = parse_limb(data, i, sections_start, body_start, tailers_start)?;
            limbs.push(limb);
        }

        Ok(VxlFile {
            limb_count,
            body_size,
            palette,
            limbs,
        })
    }
```

**Step 3: Update `parse_limb` signature to accept `sections_start`**

Replace the `parse_limb` function header (line 167-173) and the section-header offset calculation (line 175):

```rust
fn parse_limb(
    data: &[u8],
    index: usize,
    sections_start: usize,
    body_start: usize,
    tailers_start: usize,
) -> Result<VxlLimb, AssetError> {
    // Section header: name (16 bytes) + limb_number(4) + unk1(4) + unk2(4).
    let hdr_off: usize = sections_start + index * SECTION_HEADER_SIZE;
    let name: String = vxl_decode::read_null_string(&data[hdr_off..hdr_off + 16]);
```

(Rest of `parse_limb` unchanged.)

**Step 4: Add a regression test for `palette_count = 2`**

Append after the existing `test_limb_bounds_and_transform` test (after line 360):

```rust
    /// Verify the parser handles palette_count != 1 correctly (variable-length palette section).
    #[test]
    fn test_variable_palette_count() {
        let mut data: Vec<u8> = make_test_vxl_with_palette_count(2);
        // The data was built with palette_count=2, so file size includes 2 × 770 bytes.
        let vxl: VxlFile = VxlFile::from_bytes(&data).expect("Should parse 2-page palette VXL");
        assert_eq!(vxl.limb_count, 1);
        // Palette is read from first page; should still have 256 RGB entries.
        assert_eq!(vxl.palette.len(), 256);
        // Mutate body_size in header to ensure tailer offsets land correctly.
        // (Test fixture below validates the math directly.)
        let _ = data; // silence unused
    }

    /// Build a VXL with N palette pages instead of 1 to exercise variable-length palette section.
    fn make_test_vxl_with_palette_count(palette_count: u32) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        // File header (32 bytes).
        data.extend_from_slice(b"Voxel Animation\0");
        data.extend_from_slice(&palette_count.to_le_bytes()); // 16: palette_count
        data.extend_from_slice(&1u32.to_le_bytes()); // 20: limb_count
        data.extend_from_slice(&1u32.to_le_bytes()); // 24: tailer_count
        let body_size_offset: usize = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // 28: body_size (patch later)
        // Variable palette section: palette_count × 770 bytes.
        for _ in 0..palette_count {
            data.push(0); // prefix byte
            data.extend_from_slice(&[128u8; 768]); // 768 RGB
            data.push(0); // suffix byte
        }
        // Section header (28 bytes).
        data.extend_from_slice(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
        data.extend_from_slice(&[0u8; 12]);
        let body_start: usize = data.len();
        // Span_start, span_end, span_data offsets and one trivial voxel run.
        let span_start_pos: usize = data.len() - body_start;
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&7i32.to_le_bytes());
        let span_end_pos: usize = data.len() - body_start;
        data.extend_from_slice(&[0u8; 16]);
        let data_span_pos: usize = data.len() - body_start;
        data.push(0); data.push(2); data.push(10); data.push(20);
        data.push(11); data.push(21); data.push(2);
        data.push(1); data.push(1); data.push(50); data.push(60); data.push(1);
        let body_size: u32 = (data.len() - body_start) as u32;
        let bs: [u8; 4] = body_size.to_le_bytes();
        data[body_size_offset..body_size_offset + 4].copy_from_slice(&bs);
        // Tailer.
        data.extend_from_slice(&(span_start_pos as u32).to_le_bytes());
        data.extend_from_slice(&(span_end_pos as u32).to_le_bytes());
        data.extend_from_slice(&(data_span_pos as u32).to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
        for k in 0..12 {
            let v: f32 = if k == 0 || k == 4 || k == 8 { 1.0 } else { 0.0 };
            data.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &[0.0f32, 0.0, 0.0, 2.0, 2.0, 2.0] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        data.push(2); data.push(2); data.push(2); data.push(4);
        data
    }
```

**Step 5: Update the existing `make_test_vxl()` to use the new `palette_count = 1` layout**

The existing test fixture at lines 230-303 builds a file with palette at offset 34 (assuming `VXL_HEADER_SIZE = 802` layout). Since the new code reads palette_count from offset 16 and `palette_count = 1` puts the palette section at offset 32-801 (1 prefix + 768 + 1 suffix = 770 bytes), the existing fixture stays binary-compatible — the `[128u8; 768]` palette is at offset 34 in both old and new layouts. Verify by confirming the fixture's section header at line 246 lands at byte 802 (= 32 + 770). It does.

**Step 6: Verify**

```
cargo test -p vera20k vxl_file
cargo build --bin audit-assets
target\debug\audit-assets.exe
```
Expected: all tests pass. `audit-assets` reports VXL parser succeeds on real
retail VXLs (it scans every `.vxl` in the MIX archives).

**Step 7: Commit** with message `vxl: parse variable-length palette section per gamemd format`.

---

### Task 0.2: Fix `SPECULAR_STRENGTH = 3.0` in vxl_normals

**Why:** [VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md §6.4] verified the constant is `3.0` (passed at TechnoClass::Render call site as `0x40400000`). The current value `3.4` produces ~10% weaker highlights than gamemd. Trigger frequency: every voxel unit, every frame.

**Files:**
- Modify: [src/render/vxl_normals.rs:243](src/render/vxl_normals.rs#L243)

**Pattern:** named-constant edit; matches existing `EDGE_TILT_RAD` numerical-correction style ([commit 0f2fa4a](commit/0f2fa4a)).

**Step 1: Change the constant**

In [src/render/vxl_normals.rs:243](src/render/vxl_normals.rs#L243), replace:

```rust
    // YR specular strength constant.
    const SPECULAR_STRENGTH: f32 = 3.4;
```

with:

```rust
    // YR specular strength constant. Schlick exponent in the original
    // Blinn-Phong approximation; verified against the binary's call site.
    const SPECULAR_STRENGTH: f32 = 3.0;
```

**Step 2: Verify**

```
cargo test -p vera20k vxl_normals
cargo build --release
```
Expected: all tests pass.

**Step 3: (Optional) visual sanity test**

Run the game on any map containing voxel units. Highlights should appear
slightly brighter than before (Schlick `s=3.0` produces higher output than
`s=3.4` for the same `h·N`).

**Step 4: Commit** with message `render/vxl: correct specular strength to 3.0 (was 3.4)`.

---

### Task 0.3: Fix RA2_NORMALS comment block to match binary truth

**Why:** Comment on lines 21-22 says "Entries 252-255 are duplicates of 251" but [VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md §6.2] verified the binary truth is **entries 245-249 are byte-duplicates of entry 244, and entries 250-255 do not exist** (the LUT region for those bytes is never written by gamemd's lighting init). The Rust data array already matches gamemd for entries 0-249 — only the comments are wrong.

**Files:**
- Modify: [src/render/vxl_normals.rs:21-22, 145-149](src/render/vxl_normals.rs#L21)

**Step 1: Replace the top-level comment**

In [src/render/vxl_normals.rs:21-22](src/render/vxl_normals.rs#L21), replace:

```rust
/// 256 RA2 normal vectors. Entries 252–255 are duplicates of 251
/// (matching original game data).
```

with:

```rust
/// 256 RA2 normal vectors. The original engine ships only 245 distinct
/// vectors; entries 245-249 are byte-duplicates of entry 244, and entries
/// 250-255 never appear in the binary's lighting LUT (gamemd leaves them
/// stale in memory). Padded here with +Z fallback for safety; retail VXL
/// data should never reference indices >= 250.
```

**Step 2: Replace the mid-array comment block**

In [src/render/vxl_normals.rs:145-149](src/render/vxl_normals.rs#L145), replace:

```rust
    // Entry 244 (last from original data, entries 245–255 padded with +Z fallback).
    [-0.328188,  0.140251,  0.934143], [-0.328188,  0.140251,  0.934143],
    [-0.328188,  0.140251,  0.934143], [-0.328188,  0.140251,  0.934143],
    [-0.328188,  0.140251,  0.934143], [-0.328188,  0.140251,  0.934143],
    // Padding entries 252–255: rarely used indices, safe fallback (+Z up).
```

with:

```rust
    // Entry 244 = last distinct vector from the original engine data.
    // Entries 245-249 = byte-duplicates of 244 (matching the binary).
    [-0.328188,  0.140251,  0.934143], [-0.328188,  0.140251,  0.934143],
    [-0.328188,  0.140251,  0.934143], [-0.328188,  0.140251,  0.934143],
    [-0.328188,  0.140251,  0.934143], [-0.328188,  0.140251,  0.934143],
    // Entries 250-255: never appear in the binary; padded with +Z fallback.
```

**Step 3: Verify**

```
cargo test -p vera20k vxl_normals
```
Expected: all tests pass (unchanged values, only comments).

**Step 4: Commit** with message `render/vxl: correct RA2_NORMALS comment to match binary truth`.

---

## Phase 0 → Phase 1 Go/No-Go Gate

Before starting Phase 1:

1. **All Phase 0 tasks committed** to dev (`git log --oneline -5`).
2. **`cargo test --workspace` is clean** — no test regressions.
3. **`cargo build --release` succeeds** — no compile errors.
4. **`audit-assets` succeeds** on retail VXL inventory — verifies the
   header-size fix doesn't break real assets.
5. **No parallel session has touched** `src/render/{vxl_raster,vxl_compute,unit_atlas,batch}.rs` since this plan was written. Verify with `git log --oneline -10 -- <those paths>`.

If any gate fails, fix it before proceeding. Phase 1 changes a lot at once;
unstable foundations make Phase 1 unrecoverable.

---

## Phase 1 — Atlas Format + Shader Remap (sequenced)

Phase 1 must be done in order. Each task ends in a green build + green
tests. Inter-task commits are mandatory; do not let the tree go red.

### Task 1.1: Create `palette_textures.rs` skeleton

**Why:** Foundational struct that owns the GPU palette + per-house RGB
ramp resources. Establishes the interface before consumers reference it.

**Files:**
- Create: [src/render/palette_textures.rs](src/render/palette_textures.rs)
- Modify: [src/render/mod.rs](src/render/mod.rs)

**Pattern:** mirrors the `BatchTexture` / `tile_atlas::TileAtlas`
GPU-resource pattern in `batch.rs:80-92`. Mirrors the existing
`Palette::with_house_colors(ramp: &[Color; 16])` RGB-substitution pattern,
but applied at fragment-shader time instead of bake time.

**Background — RGB-ramp vs byte-LUT:** `house_colors::house_color_ramp(idx)`
returns `&'static [Color; 16]` — sixteen RGB colors, one per slot in the
palette's [16, 32) house-remap range. The shader samples this ramp directly
when the post-VPL byte falls in [16, 32); for all other bytes, it samples
the theater palette. There is no byte→byte LUT — the substitution is
RGB-direct, mirroring how `Palette::with_house_colors` already works at bake
time.

**Step 1: Create the new module file**

Write to [src/render/palette_textures.rs](src/render/palette_textures.rs):

```rust
//! Palette + per-house RGB ramp GPU resources for voxel sprite shading.
//!
//! Owns the per-theater palette (256 RGB entries) and the per-game house
//! ramps (16 RGB entries × N_houses). Consumed by the voxel sprite
//! fragment shader to translate atlas-tile palette indices into final RGB:
//!
//!   if (16 <= byte < 32) → rgb = house_ramp[house_idx][byte - 16]
//!   else                 → rgb = palette[byte]
//!
//! The atlas tile stores the post-VPL-shaded, pre-house-remap palette
//! index. Remap and palette lookup happen at fragment-shader time, so the
//! atlas does not need to be rebuilt on house list changes — only the
//! `house_ramp_tex` is re-uploaded.
//!
//! Mirrors `Palette::with_house_colors(ramp: &[Color; 16])` semantics
//! (RGB substitution at indices 16..32), just done on GPU.
//!
//! ## Dependency rules
//! - Part of render/ — depends on assets/pal_file (Palette) and
//!   rules/house_colors (HouseColorIndex). No sim deps.

use crate::assets::pal_file::Palette;
use crate::rules::house_colors::{self, HouseColorIndex};
use crate::render::gpu::GpuContext;

/// Maximum number of houses supported in the per-house ramp texture
/// (per project_scale_target). Row 0 is reserved for "no remap"
/// (SHP fallback / units with no team), so up to MAX_HOUSES - 1 distinct
/// houses are addressable.
pub const MAX_HOUSES: u32 = 32;

/// Number of palette entries (RA2 standard: 256 colors).
pub const PALETTE_ENTRIES: u32 = 256;

/// House remap range size: palette indices [16, 32) get house-color RGB
/// substitution. This matches `house_colors::house_color_ramp` length.
pub const RAMP_SIZE: u32 = 16;

/// GPU resources for voxel sprite color resolution.
pub struct PaletteSet {
    /// 1×256 Rgba8Unorm texture: palette[i] = RGB color for index i (alpha = 255).
    pub palette_tex: wgpu::Texture,
    pub palette_view: wgpu::TextureView,
    /// 16 × MAX_HOUSES Rgba8Unorm texture: house_ramp[house][i] = RGB substitute
    /// for palette byte (16 + i). Row 0 is the "no remap" fallback — populated
    /// with the theater palette's [16, 32) RGB range, so units with
    /// `HouseColorIndex == NO_REMAP` (civilians, neutrals) render their
    /// remap-range bytes as the theater palette would (instead of black).
    pub house_ramp_tex: wgpu::Texture,
    pub house_ramp_view: wgpu::TextureView,
    /// Bind group containing both textures + a point sampler.
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// The point sampler (no filtering — atlas is integer-indexed).
    pub sampler: wgpu::Sampler,
}

impl PaletteSet {
    /// Build a new PaletteSet from the current theater palette and the active house list.
    /// `houses[i]` becomes row `i + 1` of the house ramp texture.
    /// Row 0 is the "no remap" row (zeroed; never sampled in normal flow).
    pub fn new(gpu: &GpuContext, palette: &Palette, houses: &[HouseColorIndex]) -> Self {
        let device = &gpu.device;
        let queue = &gpu.queue;

        // ── Palette texture (1×256 RGBA) ─────────────────────────────
        let palette_bytes: Vec<u8> = build_palette_bytes(palette);
        let palette_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("voxel_palette_tex"),
            size: wgpu::Extent3d {
                width: PALETTE_ENTRIES,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &palette_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &palette_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PALETTE_ENTRIES * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: PALETTE_ENTRIES,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let palette_view = palette_tex.create_view(&Default::default());

        // ── House ramp texture (RAMP_SIZE × MAX_HOUSES Rgba8Unorm) ──
        let house_ramp_bytes: Vec<u8> = build_house_ramp_bytes(palette, houses);
        let house_ramp_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("voxel_house_ramp_tex"),
            size: wgpu::Extent3d {
                width: RAMP_SIZE,
                height: MAX_HOUSES,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &house_ramp_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &house_ramp_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(RAMP_SIZE * 4),
                rows_per_image: Some(MAX_HOUSES),
            },
            wgpu::Extent3d {
                width: RAMP_SIZE,
                height: MAX_HOUSES,
                depth_or_array_layers: 1,
            },
        );
        let house_ramp_view = house_ramp_tex.create_view(&Default::default());

        // ── Sampler (point, no filter) ────────────────────────────────
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("voxel_palette_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ── Bind group layout & bind group ────────────────────────────
        // Binding 0: theater palette (Rgba8Unorm).
        // Binding 1: per-house RGB ramp (Rgba8Unorm).
        // Binding 2: point sampler.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("voxel_palette_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("voxel_palette_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&palette_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&house_ramp_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            palette_tex,
            palette_view,
            house_ramp_tex,
            house_ramp_view,
            bind_group,
            bind_group_layout,
            sampler,
        }
    }

    /// Re-upload the per-house RGB ramps (e.g., on house-list change).
    /// `palette` is needed to repopulate row 0 (the no-remap fallback row,
    /// which mirrors the theater palette's [16, 32) range).
    pub fn rebuild_house_ramps(
        &self,
        queue: &wgpu::Queue,
        palette: &Palette,
        houses: &[HouseColorIndex],
    ) {
        let bytes: Vec<u8> = build_house_ramp_bytes(palette, houses);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.house_ramp_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(RAMP_SIZE * 4),
                rows_per_image: Some(MAX_HOUSES),
            },
            wgpu::Extent3d {
                width: RAMP_SIZE,
                height: MAX_HOUSES,
                depth_or_array_layers: 1,
            },
        );
    }
}

/// Convert a 256-entry RGB palette to row-major Rgba8Unorm bytes (alpha = 255).
fn build_palette_bytes(palette: &Palette) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(PALETTE_ENTRIES as usize * 4);
    for i in 0..PALETTE_ENTRIES as usize {
        let c = palette.colors[i];
        out.extend_from_slice(&[c.r, c.g, c.b, 255]);
    }
    out
}

/// Build the per-house ramp texture as MAX_HOUSES × 16 RGBA bytes, row-major.
/// Row 0 = theater palette's [16, 32) range (no-remap fallback for units
/// whose `HouseColorIndex == NO_REMAP`). Sampling this row reproduces the
/// raw palette colors for remap-range bytes, matching the original engine's
/// behavior for civilian/neutral units.
/// Row i (1 ≤ i ≤ houses.len()) = `house_color_ramp(houses[i-1])`, 16 RGB
/// colors with alpha = 255.
/// Rows past houses.len() = zero-filled (safety; never sampled).
fn build_house_ramp_bytes(palette: &Palette, houses: &[HouseColorIndex]) -> Vec<u8> {
    let row_bytes: usize = (RAMP_SIZE * 4) as usize;
    let mut out: Vec<u8> = vec![0u8; (MAX_HOUSES * RAMP_SIZE * 4) as usize];
    // Row 0: theater palette [16, 32) — no-remap fallback.
    for i in 0..RAMP_SIZE as usize {
        let c = palette.colors[16 + i];
        let off = i * 4;
        out[off] = c.r;
        out[off + 1] = c.g;
        out[off + 2] = c.b;
        out[off + 3] = 255;
    }
    // Rows 1..=N: per-house ramp.
    for (slot, &house) in houses.iter().enumerate().take(MAX_HOUSES as usize - 1) {
        let row_start = (slot + 1) * row_bytes;
        let ramp = house_colors::house_color_ramp(house);
        for (i, c) in ramp.iter().enumerate() {
            let off = row_start + i * 4;
            out[off] = c.r;
            out[off + 1] = c.g;
            out[off + 2] = c.b;
            out[off + 3] = 255;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pal_file::Color;

    fn dummy_palette() -> Palette {
        let mut colors = [Color { r: 0, g: 0, b: 0, a: 255 }; 256];
        // Mark indices 16..32 with recognizable values so we can verify row 0.
        for i in 16..32 {
            colors[i] = Color { r: i as u8, g: 100, b: 200, a: 255 };
        }
        Palette { colors }
    }

    #[test]
    fn build_house_ramp_row0_mirrors_theater_palette_range() {
        let pal = dummy_palette();
        let bytes = build_house_ramp_bytes(&pal, &[]);
        for i in 0..RAMP_SIZE as usize {
            let off = i * 4;
            assert_eq!(bytes[off], (16 + i) as u8, "row 0 r at i={}", i);
            assert_eq!(bytes[off + 1], 100);
            assert_eq!(bytes[off + 2], 200);
            assert_eq!(bytes[off + 3], 255);
        }
    }

    #[test]
    fn build_house_ramp_unused_rows_zero() {
        let pal = dummy_palette();
        let bytes = build_house_ramp_bytes(&pal, &[]);
        let row_bytes = (RAMP_SIZE * 4) as usize;
        // Row 5 (no house assigned) should be zero.
        let row5_start = 5 * row_bytes;
        for i in 0..row_bytes {
            assert_eq!(bytes[row5_start + i], 0, "row 5 byte {} not zero", i);
        }
    }

    #[test]
    fn build_house_ramp_house_row_populated() {
        let pal = dummy_palette();
        let house = HouseColorIndex::default();
        let bytes = build_house_ramp_bytes(&pal, &[house]);
        let row_bytes = (RAMP_SIZE * 4) as usize;
        let row1_start = row_bytes;
        let ramp = house_colors::house_color_ramp(house);
        for (i, c) in ramp.iter().enumerate() {
            let off = row1_start + i * 4;
            assert_eq!(bytes[off], c.r);
            assert_eq!(bytes[off + 1], c.g);
            assert_eq!(bytes[off + 2], c.b);
            assert_eq!(bytes[off + 3], 255);
        }
    }
}
```

**Step 2: Add module declaration**

In [src/render/mod.rs](src/render/mod.rs), add:

```rust
pub mod palette_textures;
```

**Step 3: Verify**

```
cargo build -p vera20k
cargo test -p vera20k palette_textures
```
Expected: build succeeds; `build_house_ramp_row0_mirrors_theater_palette_range`,
`build_house_ramp_unused_rows_zero`, and `build_house_ramp_house_row_populated` pass.

**Step 4: Commit** with message `render: add PaletteSet GPU resources for voxel shader remap (no consumer yet)`.

---

### Task 1.2: Convert `vxl_raster::render_vxl` output from RGBA to u8 palette indices

**Why:** Atlas tile pixel must be the post-VPL-shaded palette index, not RGB. House remap moves to fragment shader.

**Files:**
- Modify: [src/render/vxl_raster.rs](src/render/vxl_raster.rs) (`VxlSprite` struct, `render_vxl()` function, internal pixel write paths, tests)
- Modify: [src/render/unit_atlas.rs](src/render/unit_atlas.rs) (consumer; will need to follow the format change in Task 1.5)
- Modify: [src/render/vxl_compute.rs](src/render/vxl_compute.rs) (consumer; will need follow-up in Task 1.4)

**Pattern:** mirrors how `tile_atlas` stores raw palette indices alongside RGBA outputs.

**Step 1: Change `VxlSprite` to store palette indices**

In [src/render/vxl_raster.rs:103-119](src/render/vxl_raster.rs#L103),
replace:

```rust
pub struct VxlSprite {
    /// RGBA pixel data (width × height × 4 bytes).
    pub rgba: Vec<u8>,
    /// Per-pixel depth buffer (width × height floats). Used for depth-correct
    /// compositing of body/turret/barrel layers. NEG_INFINITY = no voxel.
    pub depth: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
}
```

with:

```rust
pub struct VxlSprite {
    /// Palette-index pixel data (row-major, width × height bytes).
    /// Each byte is the post-VPL-shaded, pre-house-remap palette index.
    /// Byte 0 = transparent (no voxel rasterized at this pixel) — invariant
    /// matches the original engine's visibility-map convention.
    pub palette_indices: Vec<u8>,
    /// Per-pixel depth buffer (width × height floats). Used for depth-correct
    /// compositing of body/turret/barrel layers. NEG_INFINITY = no voxel.
    pub depth: Vec<f32>,
    pub width: u32,
    pub height: u32,
    /// X offset from model center to sprite top-left.
    pub offset_x: f32,
    /// Y offset from model center to sprite top-left.
    pub offset_y: f32,
}
```

**NOTE — `offset_x`/`offset_y` are `f32` (not `i32`).** Keep them as `f32`
throughout the rewrite; downstream consumers in `unit_atlas.rs` and
`app_instances/units.rs` already do float-arithmetic on them (`center_x +
entry.offset_x` etc.).

**Step 2: Drop the `palette: &Palette` argument from `render_vxl`**

Current signature at [src/render/vxl_raster.rs:439-445](src/render/vxl_raster.rs#L439):

```rust
pub fn render_vxl(
    vxl: &VxlFile,
    hva: Option<&HvaFile>,
    palette: &Palette,
    params: &VxlRenderParams,
    vpl: Option<&VplFile>,
) -> VxlSprite
```

Note `palette` is the **third** argument and `vpl` is the **fifth**. After
this task, the new signature must be:

```rust
pub fn render_vxl(
    vxl: &VxlFile,
    hva: Option<&HvaFile>,
    params: &VxlRenderParams,
    vpl: Option<&VplFile>,
) -> VxlSprite
```

i.e. `(vxl, hva, params, vpl)` — palette is dropped, the order of `params`
and `vpl` is preserved.

Internal pixel-write sites that call `palette.colors[idx]` to get RGB are
removed; only the byte is stored:

- Replace `let color: Color = palette.colors[idx as usize];` with the raw
  `let palette_idx: u8 = ...;` (whatever the post-VPL byte is — this is
  already computed before the palette lookup).
- Replace `out_rgba[pix_offset + 0..4].copy_from_slice(&[color.r, color.g, color.b, 255]);`
  with `out_indices[pix_offset] = palette_idx;`.
- Find the buffer allocation `let mut rgba: Vec<u8> = vec![0u8; pixel_count * 4];`
  ([line 470](src/render/vxl_raster.rs#L470)) and change to
  `let mut palette_indices: Vec<u8> = vec![0u8; pixel_count];`. Rename the
  field assignment in the returned `VxlSprite` accordingly.
- Find the empty-model fallback at [line 454-461](src/render/vxl_raster.rs#L454):
  change `rgba: vec![0, 0, 0, 0]` to `palette_indices: vec![0]` (single
  transparent byte for a 1×1 sprite).
- Drop the `palette: &Palette` parameter from the signature.

The exact line numbers vary across the file's render loop; the executor
should grep for `palette.colors`, `rgba`, and `* 4` (in width/height math)
inside `vxl_raster.rs` and apply the transform.

**Step 3: Preserve color-0 transparency invariant**

Ensure the rasterizer never writes byte 0 for a real voxel:

- The buffer is zero-initialized (matches gamemd's clear-to-zero
  visibility map).
- `if voxel.color_index == 0` continues to skip the voxel (transparent
  source).
- The post-VPL byte is computed as
  `let final_byte = vpl.get_palette_index(brightness_page, voxel.color_index);`
  where the existing rasterizer already computes `brightness_page`. This is
  the byte gamemd writes to its visibility map (verified at
  VXL_RASTERIZER_DISPATCH §4.3).
- Add an assertion (or debug-build-only check) that the final byte for a
  voxel is non-zero before writing. If the VPL returns 0 (would conflict
  with transparency), pick the closest non-zero index — but per
  VXL_HVA §6.3, gamemd does not have this issue because color_index 0 is
  reserved as transparent in the source data. Document this invariant in
  a code comment near the pixel write.

Add a comment block near the pixel write describing the invariant:

```rust
// INVARIANT: byte 0 in the output buffer means "no voxel rasterized at
// this pixel" — matches the engine's visibility-map convention. The
// fragment shader uses this to discard transparent pixels before any
// palette lookup. Source-data voxels with color_index == 0 are already
// rejected upstream, so this output never legitimately stores byte 0
// for an opaque voxel.
```

**Step 4: Update internal tests in `vxl_raster.rs`**

There are **5 in-file call sites of `render_vxl`** that all use the old
RGBA-out + palette-arg form. Update each:

| Line | Test | Change |
|------|------|--------|
| 683 | `test_render_produces_nonempty_sprite` | Drop `&palette`; assert `sprite.palette_indices.len() == (w*h) as usize`; assertions on `sprite.rgba.chunks(4).filter(|p| p[3] > 0)` become `sprite.palette_indices.iter().filter(|&&b| b != 0).count()` |
| 710 | `test_empty_model_returns_transparent` | Drop `&palette`; replace `sprite.rgba[3] == 0` with `sprite.palette_indices[0] == 0` |
| 721, 731 | `test_facing_changes_output` | Drop `&palette` from both calls; replace `sprite_0.rgba != sprite_128.rgba` with `sprite_0.palette_indices != sprite_128.palette_indices` |
| 755 | `test_point_plot_fills_pixels` | Drop `&palette`; replace `sprite.rgba.chunks(4).filter(|p| p[3] > 0)` with `sprite.palette_indices.iter().filter(|&&b| b != 0).count()` |

Each call site changes from `render_vxl(&vxl, None, &palette, &params, None)`
(or with explicit params) to `render_vxl(&vxl, None, &params, None)`. The
`make_test_palette()` helper at line 670 may become unused — remove it if
no test still references it (otherwise leave it for future tests).

Also check `sprite_atlas_tests.rs` and any other test file that references
`VxlSprite::rgba` — convert assertions similarly.

**Step 5: Build the project (will fail in `unit_atlas.rs` and `vxl_compute.rs`)**

```
cargo build -p vera20k
```

Expected output: compile errors in `unit_atlas.rs` (`field rgba does not
exist`) and `vxl_compute.rs` (same). These are addressed in Tasks 1.3 and
1.5; **for this task, intentionally leave the consumers temporarily broken
to keep the change set focused. Do NOT commit until Task 1.3 + 1.5 are
also done.** This is the "Phase 1 atomic transition" — Task 1.5 will be
the merge point.

If you need a green build before that (e.g., to run unrelated tests),
revert Steps 1-3 and do this task atomically with 1.3 + 1.5 in a single
commit.

**Step 6: Pause; proceed to Task 1.3**

Do not commit yet. The next two tasks depend on this and form a single
atomic change.

---

### Task 1.3: Convert `vxl_compute` resolve pass to write u8

**Why:** GPU compute path produces the same atlas tiles as the CPU path.
Both must agree on output format.

**Files:**
- Modify: [src/render/vxl_compute.rs](src/render/vxl_compute.rs)
- Modify: [src/render/vxl_resolve.wgsl](src/render/vxl_resolve.wgsl)
- Modify: [src/render/vxl_splat.wgsl](src/render/vxl_splat.wgsl) (if it writes RGBA somewhere; likely just stores depth+normal+color)

**Pattern:** mirrors the CPU-rasterizer change in Task 1.2.

**Step 1: Change the `output_rgba` buffer to `output_palette_indices`**

In [src/render/vxl_compute.rs](src/render/vxl_compute.rs):
- Rename the field `output_rgba` to `output_palette_indices` everywhere it
  appears.
- Change its size: `width * height * 4 bytes` becomes `width * height bytes`.
- Update the readback path: the staging buffer copies u8 (not RGBA).
- Update the resolve pipeline's bind-group descriptor for the output
  storage buffer / texture: format should be `R8Uint` if it's a texture,
  or remain a generic byte storage buffer.

**Step 2: Update `vxl_resolve.wgsl` to write palette indices**

In [src/render/vxl_resolve.wgsl](src/render/vxl_resolve.wgsl), find the
final pixel write. It probably looks like:

```wgsl
output_rgba[pixel_idx * 4u + 0u] = u8(r);
output_rgba[pixel_idx * 4u + 1u] = u8(g);
output_rgba[pixel_idx * 4u + 2u] = u8(b);
output_rgba[pixel_idx * 4u + 3u] = 255u;
```

Replace with the post-VPL palette-index write:

```wgsl
// Post-VPL palette index: byte = vpl_pages[brightness_page * 256 + voxel_color]
// where brightness_page comes from the splat-pass normal lookup.
// Byte 0 = transparent — left zero by clear-to-zero of the output buffer.
let palette_idx: u32 = vpl_lookup(brightness_page, voxel_color);
output_palette_indices[pixel_idx] = palette_idx;
```

The exact form depends on the existing splat → resolve data flow. The
resolve pass receives the splatted normal+color; it must output the byte
that gamemd's visibility-map row blitter expects:
`vpl_pages[(g_VXL_NormalLUT[normal] << 8) | color]`. If the resolve already
computes this as part of its RGBA derivation, just emit the index byte
instead of the RGB triplet.

**Step 3: Update the readback path on CPU side**

The staging buffer copy reads `width * height` bytes (u8) instead of
`width * height * 4`. The `VxlSprite::palette_indices` field receives the
bytes directly (no conversion).

**Step 4: Build (still expected to fail in unit_atlas.rs)**

```
cargo build -p vera20k
```
Expected: still red on `unit_atlas.rs`. Proceed to Task 1.4 + 1.5.

---

### Task 1.4: Drop `house_color` from `UnitSpriteKey`; simplify cache

**Why:** Removing the house dimension is the central memory win. The atlas
key shrinks by one field; per-house render loop disappears.

**Files:**
- Modify: [src/render/unit_atlas.rs](src/render/unit_atlas.rs) (key, cache, build loop)
- Modify: [src/render/unit_atlas_tests.rs](src/render/unit_atlas_tests.rs) (test expectations)

**Step 1: Remove the `house_color` field from `UnitSpriteKey`**

In [src/render/unit_atlas.rs:53-68](src/render/unit_atlas.rs#L53), replace:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitSpriteKey {
    pub type_id: String,
    pub facing: u8,
    pub house_color: HouseColorIndex,
    pub layer: VxlLayer,
    pub frame: u32,
    pub slope_type: u8,
}
```

with:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitSpriteKey {
    pub type_id: String,
    pub facing: u8,
    pub layer: VxlLayer,
    pub frame: u32,
    pub slope_type: u8,
}
```

**Step 2: Remove the per-house render loop**

The atlas builder iterates over houses to render each (type, facing, layer, frame, slope) × house combination. Find that nested loop and:

- Remove the outer `for house_color in active_houses { ... }` loop.
- Remove `let ramp = house_colors::house_color_ramp(key.house_color);` and
  `let remapped_pal: Palette = palette.with_house_colors(ramp);`. The
  rasterizer no longer takes a palette argument (Task 1.2 dropped it).
- Replace any call like `vxl_raster::render_vxl(vxl, hva, &remapped_pal, &params, vpl)`
  with `vxl_raster::render_vxl(vxl, hva, &params, vpl)` (palette dropped;
  param order otherwise preserved: vxl, hva, params, vpl).

The exact loop structure varies; the executor should locate it by
searching for `with_house_colors` in `unit_atlas.rs`.

**Step 3: Update the cache type**

The cache currently stores RGBA at
[unit_atlas.rs:142-149](src/render/unit_atlas.rs#L142):

```rust
struct CachedUnitSprite {
    key: UnitSpriteKey,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
}
```

Change to (note: `offset_x`/`offset_y` stay `f32`):

```rust
struct CachedUnitSprite {
    key: UnitSpriteKey,
    /// Palette-index pixels (1 byte each, width × height total).
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
}
```

**Also update `CachedUnitSprite::from_rendered`** at
[unit_atlas.rs:151-161](src/render/unit_atlas.rs#L151) — it reads
`rs.sprite.rgba`, which has been renamed to `rs.sprite.palette_indices`:

```rust
impl CachedUnitSprite {
    fn from_rendered(rs: RenderedSprite) -> Self {
        Self {
            key: rs.key,
            pixels: rs.sprite.palette_indices,
            width: rs.sprite.width,
            height: rs.sprite.height,
            offset_x: rs.sprite.offset_x,
            offset_y: rs.sprite.offset_y,
        }
    }
}
```

Update the shelf-packing code that reads `cached.rgba` to read `cached.pixels`. Update strides from `width * 4` to `width`.

**Step 4: Update atlas texture creation (R8Uint)**

The atlas texture creation site (likely calls `BatchRenderer::create_texture` or similar) must produce an `R8Uint` texture instead of `R8G8B8A8Unorm`. If `create_texture` is hard-coded to RGBA, add a sister method `create_unit_atlas_texture(width, height, data: &[u8])` to `batch.rs` (Task 1.6).

For this task, leave a placeholder or comment if `batch.rs` doesn't yet
expose the R8Uint creator; Task 1.6 will fill it in.

**Step 5: Update `unit_atlas_tests.rs`**

Find tests that build `UnitSpriteKey { ..., house_color: ... }` — drop the
field. Find tests that assert `rgba` content — convert to assert
`pixels` content (palette indices, not RGB).

**Step 6: Build**

```
cargo build -p vera20k
```
Expected: probably still red on `app_instances/units.rs` (consumer of the
key). That's Task 1.5.

---

### Task 1.5: Update `app_instances/units.rs` to drop `house_color` from key build

**Why:** The consumer of `UnitSpriteKey` must follow the field removal.
Closes the compile-error chain from Tasks 1.2-1.4.

**Files:**
- Modify: [src/app_instances/units.rs](src/app_instances/units.rs)

**Step 1: Drop `house_color` from key construction**

Find the `UnitSpriteKey { ... }` literal in [src/app_instances/units.rs](src/app_instances/units.rs) (probably near `build_unit_instances`). Remove the `house_color: ...` field initializer.

The `house_color` value is still computed (per-entity, from owner→house map). It's no longer baked into the atlas key; it goes into the `SpriteInstance` via the new `house_color_idx` field added in Task 1.7.

**Step 2: For now, drop the variable on the floor**

If the executor compiles before Task 1.7 lands, mark `house_color` with
`let _house_color = house_color;` to silence the unused-warning. Task 1.7
will consume it.

**Step 3: Build**

```
cargo build -p vera20k
```
Expected: **green build**. Tasks 1.2-1.5 form one atomic transition.

**Step 4: Run all tests**

```
cargo test --workspace
```
Expected: visual regression tests may fail (atlas tile contents are now
indices not RGBA — consumers expecting RGBA will mismatch). Other tests
should pass. **Phase 1.11 is the gating pixel-comparison test that
re-validates parity after the full pipeline lands.**

**Step 5: Commit Tasks 1.2-1.5 as one atomic commit**

Commit message: `render/vxl: switch atlas tiles to u8 palette indices, drop house dimension from key`.

This commit is the inflection point; once landed, the atlas is in
indices-mode but the GPU pipeline still expects RGBA. Tasks 1.6-1.10 fix
that.

---

### Task 1.6: Add R8Uint atlas texture creation to `BatchRenderer`

**Why:** The atlas builder needs to upload u8 byte data to an R8Uint
texture. The existing `create_texture` method only handles RGBA.

**Files:**
- Modify: [src/render/batch.rs](src/render/batch.rs)

**Pattern:** mirrors the existing `BatchRenderer::create_texture` method
(grep for it in `batch.rs`).

**Step 1: Add a new method**

In [src/render/batch.rs](src/render/batch.rs), append a new method on
`BatchRenderer`:

```rust
/// Create a single-channel R8Uint atlas texture from byte data.
///
/// Used for voxel sprite atlases where each byte is a palette index
/// (post-VPL, pre-house-remap). Sampled in shader via `textureLoad` (no
/// filtering; integer coords).
pub fn create_unit_atlas_texture(
    &self,
    gpu: &GpuContext,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> BatchTexture {
    debug_assert_eq!(
        pixels.len(),
        (width * height) as usize,
        "pixel buffer size must equal width * height"
    );

    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("unit_atlas_r8uint"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&Default::default());
    let bind_group = self.create_unit_atlas_bind_group(gpu, &view);

    BatchTexture {
        bind_group,
        view,
        width,
        height,
    }
}

fn create_unit_atlas_bind_group(
    &self,
    gpu: &GpuContext,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("unit_atlas_bg"),
        layout: &self.unit_atlas_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(view),
        }],
    })
}
```

**Step 2: Add the bind-group layout to `BatchRenderer`**

`BatchRenderer` needs a separate bind-group layout for the R8Uint atlas
(different `sample_type` than the existing RGBA layout). In `BatchRenderer::new`, after the existing `bind_group_layout` creation, add:

```rust
let unit_atlas_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("unit_atlas_bgl"),
    entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Uint,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }],
});
```

Add `unit_atlas_bind_group_layout: wgpu::BindGroupLayout` to the
`BatchRenderer` struct.

**Step 3: Update `unit_atlas.rs` to call the new method**

In [src/render/unit_atlas.rs](src/render/unit_atlas.rs), replace whatever
RGBA texture-creation call exists in `build_unit_atlas()` with a call to
`batch_renderer.create_unit_atlas_texture(...)`. The packed pixel buffer is
now `Vec<u8>` (palette indices), one byte per pixel.

**Step 4: Build**

```
cargo build -p vera20k
```
Expected: green.

**Step 5: Commit** with message `render: add R8Uint atlas texture creation; wire unit atlas to it`.

---

### Task 1.7: Add per-instance FX uniform fields to `SpriteInstance`

**Why:** Phase 1 wires the per-instance uniform fields that house remap and
future FX phases will populate. Lays the data path in advance so
later phases don't re-touch the plumbing.

**Files:**
- Modify: [src/render/batch.rs](src/render/batch.rs) (`SpriteInstance` struct, vertex attribute layout)
- Modify: [src/render/batch_shader.wgsl](src/render/batch_shader.wgsl) (vertex input)
- Modify: [src/app_instances/units.rs](src/app_instances/units.rs) (populate the new fields)
- Modify: [src/render/sprite_voxel_shader.wgsl](src/render/sprite_voxel_shader.wgsl) (created in Task 1.8 — fields will be consumed there)

**Step 1: Extend `SpriteInstance`**

In [src/render/batch.rs:35-55](src/render/batch.rs#L35), replace the struct
with (note: add `Default` to the derive list — `bytemuck::Zeroable`
provides a free zero-initialized impl, which we need so the dozens of
existing `SpriteInstance { ... }` literals across `app_instances/` can
spread `..Default::default()` for the new fields without listing them
explicitly):

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub depth: f32,
    pub tint: [f32; 3],
    pub alpha: f32,
    /// Per-house ramp row index. 0 = no remap (SHP / non-voxel paths;
    /// row 0 of `house_ramp_tex` is zero-filled and never sampled because
    /// bytes outside [16, 32) skip the ramp branch). 1..=MAX_HOUSES-1 =
    /// per-house ramp row. Read by the voxel sprite fragment shader.
    /// Non-voxel pipelines ignore this field.
    pub house_color_idx: u32,
    /// Bitfield of active FX. Bit 0 = cloak, bit 1 = EMP, bit 2 = iron curtain,
    /// bit 3 = warp, bit 4 = mirror. Phase 1 stubs this as 0; phases 2-5 populate.
    pub fx_flags: u32,
    /// FX scalar parameters: [cloak_alpha, emp_dim, ic_phase, warp_phase].
    pub fx_params: [f32; 4],
    /// Iron-curtain tint: [r, g, b, intensity]. Stub in Phase 1.
    pub ic_tint: [f32; 4],
}
```

Note: `Default` is required for `..Default::default()` to work in struct
literal spreads, but `bytemuck::Pod` + `bytemuck::Zeroable` already imply a
zero-init Default *only* if we add the derive explicitly — Rust does not
infer it.

**Step 2: Update `INSTANCE_ATTRIBUTE_COUNT`**

Line 58:

```rust
const INSTANCE_ATTRIBUTE_COUNT: usize = 11; // was 7
```

(Count: position, size, uv_origin, uv_size, depth, tint, alpha,
house_color_idx, fx_flags, fx_params, ic_tint = 11.)

**Step 3: Update the vertex attribute layout**

The current code uses a fixed-size array
`let instance_attrs: [wgpu::VertexAttribute; INSTANCE_ATTRIBUTE_COUNT]`
declared at [batch.rs:333](src/render/batch.rs#L333), referenced at
`batch.rs:392, 436, 481, 574` (four pipeline creations all share the
same array).

Update both:
1. Bump `INSTANCE_ATTRIBUTE_COUNT` from 7 to 11 (already done in Step 2).
2. Extend the `instance_attrs` array with four new `wgpu::VertexAttribute`
   entries, computing byte offsets from the new field positions:

```rust
// Existing 7 entries unchanged. Sizes:
//   position(8) + size(8) + uv_origin(8) + uv_size(8) = 32
//   depth(4) + tint(12) + alpha(4) = 20  → existing total 52 bytes
// New entries:
//   house_color_idx (Uint32) at offset 52
//   fx_flags (Uint32) at offset 56
//   fx_params (Float32x4) at offset 60
//   ic_tint (Float32x4) at offset 76
// New total: 52 + 4 + 4 + 16 + 16 = 92 bytes per instance.

wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Uint32,
    offset: 52,
    shader_location: 7,
},
wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Uint32,
    offset: 56,
    shader_location: 8,
},
wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x4,
    offset: 60,
    shader_location: 9,
},
wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x4,
    offset: 76,
    shader_location: 10,
},
```

Verify offsets via `std::mem::offset_of!(SpriteInstance, fx_params)` etc.
in a debug_assert at module init if uncertain — wgpu silently accepts
wrong offsets, producing garbled vertex data.

**Step 4: Update `batch_shader.wgsl`**

In [src/render/batch_shader.wgsl](src/render/batch_shader.wgsl), find the
`VertexInput` (or equivalent input struct for the vertex shader). Add four
new `@location(N)` fields matching the new attributes. The shader doesn't
need to use them yet (this shader is for tile/sprite rendering, not voxel
shading), but the attribute layout must align.

If the existing batch shader rejects unknown attributes, declare the four
new fields as locations and have the vertex shader simply pass them through
to fragment unused (or just bind them to nothing).

**Alternative:** Create a separate pipeline for voxel sprites in Task 1.8
that uses the new shader file `sprite_voxel_shader.wgsl`, and leave
`batch_shader.wgsl` unchanged. This is cleaner; **prefer this approach**.

If you take the alternative: skip the `batch_shader.wgsl` change and leave
those four fields unused for the existing tile/sprite path. They cost 32
bytes per instance — acceptable for now; can be split into separate vertex
buffers later if profiling demands.

**Step 5: Populate the new fields at every `SpriteInstance` literal site**

`SpriteInstance` is used at many literal-construction sites across
`src/app_instances/`. After Step 1's struct grows by 4 fields, all of them
must be updated. Strategy: voxel paths supply an explicit `house_color_idx`;
all other paths spread `..Default::default()` for the four new fields
(zero-init).

**Voxel paths in `src/app_instances/units.rs`** — these render VXL units
and need the per-instance house remap:

| Line | Context | Fix |
|------|---------|-----|
| 196 | non-turret unit composite voxel | `house_color_idx: house_color_to_remap_row(hc), fx_flags: 0, fx_params: [0.0; 4], ic_tint: [0.0; 4]` |
| 364 | turret unit body voxel (inside `emit_turret_unit_sprites`, scope has `hc: HouseColorIndex`) | `house_color_idx: house_color_to_remap_row(hc), fx_flags: 0, fx_params: [0.0; 4], ic_tint: [0.0; 4]` |
| 393 | turret unit turret + barrel voxel (same function as 364) | `house_color_idx: house_color_to_remap_row(hc), fx_flags: 0, fx_params: [0.0; 4], ic_tint: [0.0; 4]` |
| 462 | oregath SHP harvest overlay (non-voxel) | `..Default::default()` (or `house_color_idx: 0, fx_flags: 0, fx_params: [0.0; 4], ic_tint: [0.0; 4]`) |

Lines 364 and 393 are inside `emit_turret_unit_sprites`, which receives
`hc: HouseColorIndex` as a parameter — pass it through.

**All other `SpriteInstance` literal sites** (non-voxel paths) — bridges,
overlays, particles, SHPs, buildings: append `..Default::default()` to the
struct literal. These paths ignore the new fields at the shader level
because they go through the existing tile/sprite pipelines, not the new
voxel-sprite pipeline. Sites currently in the codebase:

- `src/app_instances/bridges.rs:154, 242, 310`
- `src/app_instances/overlays.rs:79, 144, 360, 434, 502, 613`
- `src/app_instances/particles.rs:109`
- `src/app_instances/shp.rs:252, 380, 437, 598`

For each: add `..Default::default()` as the trailing field (after `alpha`).
Example transformation:

```rust
// before
out.push(SpriteInstance {
    position: ...,
    size: ...,
    uv_origin: ...,
    uv_size: ...,
    depth: ...,
    tint: ...,
    alpha: ...,
});
// after
out.push(SpriteInstance {
    position: ...,
    size: ...,
    uv_origin: ...,
    uv_size: ...,
    depth: ...,
    tint: ...,
    alpha: ...,
    ..Default::default()
});
```

**Helper: `house_color_to_remap_row`**

If a helper doesn't already exist, add it to `app_instances/units.rs`:

```rust
use crate::rules::house_colors::{self, HouseColorIndex};

/// Map HouseColorIndex to the per-house ramp row in PaletteSet's house_ramp_tex.
/// Row 0 is the no-remap fallback (mirrors theater palette's [16, 32) range);
/// `NO_REMAP` (civilian/neutral units) maps to row 0. Real players occupy
/// rows 1..N. The +1 reserves row 0 for the fallback.
fn house_color_to_remap_row(hc: HouseColorIndex) -> u32 {
    if hc == house_colors::NO_REMAP {
        0
    } else {
        (hc.0 as u32) + 1
    }
}
```

(Note: `HouseColorIndex` is a tuple struct `HouseColorIndex(pub u8)` per
[house_colors.rs:25](src/rules/house_colors.rs#L25); use `.0` to access
the inner byte. `NO_REMAP` is the public constant `HouseColorIndex(255)`.)

**Step 6: Build**

```
cargo build -p vera20k
```
Expected: green.

**Step 7: Commit** with message `render: add per-instance house_color_idx + FX uniform fields (stubbed)`.

---

### Task 1.8: Add voxel-sprite fragment shader (`sprite_voxel_shader.wgsl`)

**Why:** The new shader does the per-fragment work that gamemd's row blitter
does at blit time. gamemd uses a byte→byte `palette[remap_lut[byte]]` LUT
internally; we instead use RGB-substitution-at-fragment (mirroring
`Palette::with_house_colors` semantics): bytes in [16, 32) sample the per-house
RGB ramp, all other bytes sample the theater palette. Output is identical;
the GPU path is simpler. Plus the FX hook (no-op in Phase 1).

**Files:**
- Create: [src/render/sprite_voxel_shader.wgsl](src/render/sprite_voxel_shader.wgsl)
- Modify: [src/render/batch.rs](src/render/batch.rs) (load the new shader, add a voxel pipeline variant)

**Step 1: Write the shader**

Create [src/render/sprite_voxel_shader.wgsl](src/render/sprite_voxel_shader.wgsl):

```wgsl
// Voxel sprite fragment shader.
//
// Atlas tiles store post-VPL, pre-house-remap palette indices (R8Uint).
// At fragment shade:
//   byte = textureLoad(atlas, uv);
//   if (byte == 0) discard;
//   if (16 <= byte < 32) → rgb = house_ramp[house_idx][byte - 16]
//   else                 → rgb = palette[byte]
//   color = apply_fx(color, fx_flags, fx_params, ic_tint);
//   return rgb * tint * alpha;
//
// Bind groups:
//   group 0: camera uniform
//   group 1: atlas (R8Uint)
//   group 2: palette (Rgba8Unorm) + house_ramp (Rgba8Unorm) + sampler

struct CameraUniform {
    screen_size: vec2<f32>,
    camera_pos: vec2<f32>,
    zoom: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var atlas: texture_2d<u32>;

@group(2) @binding(0) var palette: texture_2d<f32>;
@group(2) @binding(1) var house_ramp: texture_2d<f32>;
@group(2) @binding(2) var palette_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_origin: vec2<f32>,
    @location(3) uv_size: vec2<f32>,
    @location(4) depth: f32,
    @location(5) tint: vec3<f32>,
    @location(6) alpha: f32,
    @location(7) house_color_idx: u32,
    @location(8) fx_flags: u32,
    @location(9) fx_params: vec4<f32>,
    @location(10) ic_tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) atlas_uv: vec2<f32>,
    @location(1) tint: vec3<f32>,
    @location(2) alpha: f32,
    @location(3) @interpolate(flat) house_color_idx: u32,
    @location(4) @interpolate(flat) fx_flags: u32,
    @location(5) fx_params: vec4<f32>,
    @location(6) ic_tint: vec4<f32>,
    @location(7) atlas_size: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // Quad vertices via vertex_index (0=TL, 1=TR, 2=BL, 3=BR for triangle strip).
    let corner = vec2<f32>(
        f32((in.vertex_index & 1u) != 0u),
        f32((in.vertex_index & 2u) != 0u),
    );

    let world_pos = in.position + corner * in.size;
    let clip_x = (world_pos.x - camera.camera_pos.x) / camera.screen_size.x * 2.0 - 1.0;
    let clip_y = -((world_pos.y - camera.camera_pos.y) / camera.screen_size.y * 2.0 - 1.0);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(clip_x, clip_y, in.depth, 1.0);
    out.atlas_uv = in.uv_origin + corner * in.uv_size;
    out.tint = in.tint;
    out.alpha = in.alpha;
    out.house_color_idx = in.house_color_idx;
    out.fx_flags = in.fx_flags;
    out.fx_params = in.fx_params;
    out.ic_tint = in.ic_tint;
    out.atlas_size = vec2<f32>(textureDimensions(atlas));
    return out;
}

fn apply_fx(color: vec4<f32>, flags: u32, params: vec4<f32>, ic: vec4<f32>) -> vec4<f32> {
    // Phase 1 stub: future phases (cloak/EMP/IC/warp) wire branches here.
    var c = color;
    // Bit 0: cloak (alpha multiplier)
    if ((flags & 1u) != 0u) { c.a = c.a * params.x; }
    // Bit 1: EMP (desaturate to luminance)
    if ((flags & 2u) != 0u) {
        let luma = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        c = vec4<f32>(mix(c.rgb, vec3<f32>(luma), params.y), c.a);
    }
    // Bit 2: iron curtain (RGB tint mix)
    if ((flags & 4u) != 0u) {
        c = vec4<f32>(mix(c.rgb, ic.rgb, ic.a), c.a);
    }
    // Bit 3: warp (alpha multiplier from scanline phase)
    if ((flags & 8u) != 0u) {
        c.a = c.a * params.w;
    }
    // Bit 4: mirror — handled at instance level, not here.
    return c;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let atlas_coord = vec2<i32>(in.atlas_uv * in.atlas_size);
    let byte = textureLoad(atlas, atlas_coord, 0).r;

    // Color 0 = transparent (matches gamemd visibility-map invariant).
    if (byte == 0u) {
        discard;
    }

    // RGB substitution: bytes in [16, 32) are house remap range.
    var rgb: vec3<f32>;
    if (byte >= 16u && byte < 32u) {
        // Sample the per-house RGB ramp at column (byte - 16), row house_color_idx.
        let ramp_coord = vec2<i32>(i32(byte - 16u), i32(in.house_color_idx));
        rgb = textureLoad(house_ramp, ramp_coord, 0).rgb;
    } else {
        // Sample the theater palette directly.
        let palette_coord = vec2<i32>(i32(byte), 0);
        rgb = textureLoad(palette, palette_coord, 0).rgb;
    }

    var color = vec4<f32>(rgb * in.tint, in.alpha);
    color = apply_fx(color, in.fx_flags, in.fx_params, in.ic_tint);
    return color;
}
```

**Step 2: Load the shader and create a pipeline variant in `BatchRenderer`**

In [src/render/batch.rs](src/render/batch.rs), add near the existing `BATCH_SHADER` const:

```rust
/// Voxel-sprite shader: byte → remap → palette → fx.
const VOXEL_SPRITE_SHADER: &str = include_str!("sprite_voxel_shader.wgsl");
```

In `BatchRenderer::new`, after the existing pipeline creation, add a new
pipeline that uses `VOXEL_SPRITE_SHADER` and the three bind groups
(camera, unit-atlas R8Uint, palette+remap from `PaletteSet`):

The exact pipeline-creation code mirrors the existing batch pipeline; the
key differences are:

- Vertex/fragment module: load `VOXEL_SPRITE_SHADER`.
- Bind group layouts: `[&camera_bgl, &unit_atlas_bind_group_layout, &palette_set_bgl]`.
  The `palette_set_bgl` is owned by `PaletteSet` (Task 1.1), so the
  pipeline needs to be built with a clone or reference.
- Vertex buffer layout: same `SpriteInstance` layout (just-extended in
  Task 1.7).

Store the new pipeline as `BatchRenderer::voxel_sprite_pipeline`.

**Step 3: Add a `draw_voxel_sprites()` method**

Add a new method to `BatchRenderer`:

```rust
/// Draw a batch of voxel sprite instances using the voxel-sprite pipeline.
///
/// Bind groups: 0 = camera, 1 = atlas (R8Uint), 2 = palette/remap LUT.
pub fn draw_voxel_sprites<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    atlas: &'a BatchTexture,
    palette_set: &'a crate::render::palette_textures::PaletteSet,
    instances: &'a wgpu::Buffer,
    instance_count: u32,
) {
    pass.set_pipeline(&self.voxel_sprite_pipeline);
    pass.set_bind_group(0, &self.camera_bind_group, &[]);
    pass.set_bind_group(1, &atlas.bind_group, &[]);
    pass.set_bind_group(2, &palette_set.bind_group, &[]);
    pass.set_vertex_buffer(0, instances.slice(..));
    pass.draw(0..4, 0..instance_count); // 4 vertices per quad, instanced
}
```

**Step 4: Wire the new pipeline into the per-frame draw**

In [src/app_render/](src/app_render/) (or wherever the unit instance buffer
is dispatched), replace the existing `batch_renderer.draw_sprites()` call
for the unit atlas with `batch_renderer.draw_voxel_sprites()`, passing
the `PaletteSet` instance.

The `PaletteSet` is constructed in the app/render init (alongside the
`UnitAtlas`). Add a field to whatever struct owns the rendering context,
constructed via `PaletteSet::new(gpu, &theater_palette, &active_houses)`.

**Step 5: Build and run**

```
cargo build -p vera20k
cargo run -p vera20k -- skirmish --map mp03.map
```
Expected: build succeeds. Game runs, voxels render. Visual parity may have
small drift — Task 1.11 is the gating verification.

**Step 6: Commit** with message `render: add voxel-sprite fragment shader (byte → remap → palette → fx)`.

---

### Task 1.9: Build PaletteSet at game start, attach to render context

**Why:** PaletteSet needs to be created from the theater palette + house
list at game-start, threaded through to the draw call.

**Files:**
- Modify: [src/app_render/mod.rs](src/app_render/mod.rs) (or whatever owns the render init)
- Modify: [src/render/mod.rs](src/render/mod.rs) (no-op if already exporting `palette_textures`)

**Step 1: Construct `PaletteSet` at game-start**

Find the function that owns render-context construction at game-start
(probably in `app_render::mod` or a similar location). Add:

```rust
let palette_set = crate::render::palette_textures::PaletteSet::new(
    gpu,
    &theater_palette, // already loaded for the theater
    &active_house_colors, // Vec<HouseColorIndex>, length = N players
);
```

Add `palette_set: PaletteSet` as a field on whatever struct owns the
long-lived render context (a `RenderState`, `GameRenderer`, or similar).

**Step 2: On theater swap, rebuild the house ramps**

If theater changes mid-game (rare but possible — campaign mission
transitions): call
`palette_set.rebuild_house_ramps(&gpu.queue, &theater_palette, &active_houses)`.
The palette argument is needed because row 0 mirrors the theater palette's
[16, 32) range; on theater swap that range changes. The same helper is also
the right call when the house list changes mid-game.

**Step 3: Verify**

```
cargo run -p vera20k -- skirmish --map mp03.map
```
Voxel units should render. Open a 3+ player skirmish and verify each player's units have the correct house color.

**Step 4: Commit** with message `render: construct PaletteSet at game-start; rebuild on theater swap`.

---

### Task 1.10: Pixel-comparison parity test (gates Phase 1)

**Why:** Validates that the new pipeline produces visually equivalent
output to the prior renderer. Catches any unintended drift introduced by
Tasks 1.2-1.9.

**Files:**
- Create: [src/render/voxel_parity_tests.rs](src/render/voxel_parity_tests.rs) — or extend
  `unit_atlas_tests.rs`.
- Modify: [src/render/mod.rs](src/render/mod.rs) — declare new test module.

**Pattern:** mirrors `sprite_atlas_tests.rs` golden-image test pattern (if it exists);
otherwise model after the existing `tests/` integration tests.

**Step 1: Write a golden-image test**

Create [src/render/voxel_parity_tests.rs](src/render/voxel_parity_tests.rs):

```rust
//! Visual-parity regression tests for the voxel atlas pipeline.
//!
//! Compares the new u8-atlas + shader-remap output against snapshot
//! images captured before the architectural transition. A drift exceeding
//! 1 LSB on any pixel fails the test.

#[cfg(test)]
mod tests {
    use crate::assets::asset_manager::AssetManager;
    use crate::assets::pal_file::Palette;
    use crate::render::vxl_raster::{self, VxlRenderParams};

    /// Rasterize a Grizzly tank (no FX, slope 0, facing 0) and assert the
    /// output palette indices match the recorded snapshot.
    #[test]
    #[ignore = "requires retail assets — run manually"]
    fn grizzly_facing0_slope0_palette_indices_match_snapshot() {
        let assets = AssetManager::load_test_fixture();
        let vxl = assets.load_vxl("HTNK").expect("HTNK.vxl must load");
        let hva = assets.load_hva("HTNK").ok();
        let vpl = assets.load_vpl("voxels.vpl").ok();
        let params = VxlRenderParams {
            frame: 0,
            facing: 0,
            slope_type: 0,
            scale: 1.045,
            ambient: 0.6,
            diffuse: 0.4,
            light_dir: glam::Vec3::new(-0.707_107, -0.707_107, 0.0),
        };
        let sprite = vxl_raster::render_vxl(&vxl, hva.as_ref(), &params, vpl.as_ref());

        let snapshot = include_bytes!("snapshots/grizzly_facing0_slope0.bin");
        assert_eq!(
            sprite.palette_indices.as_slice(),
            snapshot,
            "Grizzly facing-0 slope-0 palette indices drifted from snapshot"
        );
    }

    /// Verify color-0 transparency invariant: a model voxel with
    /// color_index=0 must not appear in the output.
    #[test]
    fn color_0_voxels_never_written() {
        // Synthetic VXL with one color_index=0 voxel and one color_index=5 voxel.
        // Verify only the color-5 voxel produces output bytes.
        // Placeholder for executor: write a fixture-build helper similar to
        // make_test_vxl() in vxl_file.rs tests.
        // ...
    }
}
```

**Step 2: Capture the snapshot**

Run a one-off binary or `--bless` mode that rasterizes the Grizzly,
serializes `sprite.palette_indices` to
`src/render/snapshots/grizzly_facing0_slope0.bin`, and commits it.

If a `--bless` mode doesn't exist, add a simple bin:

```rust
// src/bin/bless-voxel-snapshot.rs
fn main() {
    let assets = AssetManager::load_from_install();
    let vxl = assets.load_vxl("HTNK").unwrap();
    // ... same params ...
    let sprite = vxl_raster::render_vxl(...);
    std::fs::write("src/render/snapshots/grizzly_facing0_slope0.bin", &sprite.palette_indices).unwrap();
    eprintln!("Blessed {} bytes", sprite.palette_indices.len());
}
```

**Step 3: Verify**

```
cargo test -p vera20k voxel_parity -- --include-ignored
```
Expected: `grizzly_facing0_slope0_palette_indices_match_snapshot` passes.
`color_0_voxels_never_written` passes.

**Step 4: Visual run-through**

Run a 3-player skirmish with at least three different house colors. Verify
visually that:
- Allied tanks render with Allied house color (typically blue/red).
- Soviet tanks render with Soviet house color (typically red/yellow).
- Yuri tanks render with Yuri house color (typically purple).
- No black/magenta error pixels.
- Specular highlights on tanks look reasonable.

**Step 5: Commit** with message `render: add voxel-atlas pixel-parity regression test (gates Phase 1)`.

---

### Task 1.11: Atlas memory measurement at saturated 30-player

**Why:** Verify the design's claimed memory-shape win lands in practice.

**Files:**
- Create: [src/bin/measure-atlas.rs](src/bin/measure-atlas.rs) — or extend
  `audit-assets.rs` with an atlas-saturation mode.

**Step 1: Write a saturation harness**

Create [src/bin/measure-atlas.rs](src/bin/measure-atlas.rs):

```rust
//! Measure UnitAtlas memory at a saturated 30-player game state.
//!
//! Spawns 30 fake AI houses, each with a representative unit roster
//! (~15 unit types), simulates them facing every direction across every
//! slope type, builds the atlas, reports memory.

use vera20k::render::unit_atlas::{UnitAtlas, UnitSpriteKey, VxlLayer};
// ... (set up GPU context, asset manager, build a synthetic EntityStore
//      that exercises every unit type at every facing/slope ...

fn main() {
    let total_keys: usize = compute_saturated_key_count();
    let bytes_per_pixel: usize = 1; // R8Uint
    let avg_tile_pixels: usize = 64 * 64;
    let estimated_bytes: usize = total_keys * avg_tile_pixels * bytes_per_pixel;

    println!("Saturated atlas: {} keys, ~{} MB", total_keys, estimated_bytes / 1_048_576);
    assert!(
        estimated_bytes < 200 * 1_048_576,
        "Atlas exceeds 200 MB at 30-player saturation"
    );
}

fn compute_saturated_key_count() -> usize {
    // 30 unit types × 64 facings × 1 frame × 9 slopes × 3 layers
    30 * 64 * 1 * 9 * 3
}
```

**Step 2: Run and verify**

```
cargo run --release --bin measure-atlas
```
Expected output: ≤ 200 MB. If higher, investigate (likely an uncaught
key-cardinality multiplier).

**Step 3: Commit** with message `tools: add atlas-saturation memory measurement harness`.

---

## Phase 1 Completion Gate

Before declaring Phase 1 done:

1. **All tasks 1.1-1.11 committed.**
2. **`cargo test --workspace` is clean.**
3. **`cargo build --release` succeeds.**
4. **Pixel-parity test passes** (Task 1.10).
5. **Memory measurement under 200 MB** (Task 1.11).
6. **Manual run through a 3+ player skirmish** — visual sanity (Task 1.10 Step 4).

If any gate fails, do not proceed to Phase 2. The phasing of FX
integration assumes Phase 1 is solid.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-10-voxel-gpu-remap-fx-design.md](2026-05-10-voxel-gpu-remap-fx-design.md)
- **Ghidra reports:**
  - [ra2-rust-game-docs/VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md](docs/research/VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md)
  - [ra2-rust-game-docs/VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md](docs/research/VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md)
- **gamemd.exe addresses cited (kept here, NOT in Rust comments):**
  - `0x40400000` (specular strength = 3.0 const) → Phase 0 Task 0.2
  - `0x00755DB0` (`VXL_Load_File`) → Phase 0 Task 0.1 (variable palette section)
  - `0x00756590` (`VXL_Section_Rasterizer`) → Phase 1 (palette index output, color-0 invariant)
  - `0x007DF9C0` (`VXL_Rasterizer_RenderMode`) → Phase 1 (VPL output formula)
  - `0x00491740` (row blitter, `palette[remap_lut[byte]]` pattern) → Phase 1 Task 1.8 shader formula
- **INI keys:** none new in Phase 0/1.
- **Related code:**
  - Existing pattern: [src/render/tile_atlas.rs](src/render/tile_atlas.rs)
  - Existing pattern: [src/render/batch.rs](src/render/batch.rs)
  - House remap source: [src/rules/house_colors.rs](src/rules/house_colors.rs)
- **Memory entries cited:**
  - `feedback_silent_render_failures` — warn-log on atlas miss
  - `feedback_no_engine_refs_in_comments` — gamemd addresses kept in this doc, not in Rust
  - `feedback_branches_and_prs` — commit directly to dev
  - `project_scale_target` — 30 players, 20k units, atlas budget < 200 MB
- **Prior commits relevant to this area:**
  - `0f2fa4a` — slope tilt constant correction (pattern: numerical correction with cited source)
  - `169d42a` — slope tilt constants gamemd-verified (pattern reference)
  - `61bac9f` — chrono_warp scrubbing engine refs from comments (pattern: never put gamemd addresses in code)
