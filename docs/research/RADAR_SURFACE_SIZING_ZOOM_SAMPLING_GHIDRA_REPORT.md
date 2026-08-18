# Radar Surface Sizing / Zoom Sampling - Ghidra Research Report

Date: 2026-05-27

**Slot:** /re-swarm minimap slot 4  
**Target:** Radar surface sizing and zoom sampling  
**Address(es):** `RadarClass__GenerateTerrainSurface @ 0x006547C0`, `RadarClass__RebuildRadarSurfaces @ 0x00654650`, `RadarClass__ComputeRadarMapBounds @ 0x00654490`, `RadarClass__FillTerrainColors @ 0x00654EA0`, `RadarClass__One_Time @ 0x00652CF0`, `RadarClass__Init_For_House @ 0x00652E90`, `Math__ftol @ 0x007C5F00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** live in-game minimap raw radar-space buffer dimensions, generated secondary surface sizing, zoom factor selection, raw RGB sampling into the secondary terrain surface, and centering/blit geometry for ordinary in-game radar content.  
**Non-Scope:** object-dot priority/visibility, minimap click inverse transform, generic dirty-pipeline caller inventory, radar transition/open-close asset lifecycle, spy-satellite/event pixel shapes, and full TMP/tile color source decoding.  
**Confidence:** High for surface dimensions, branch constants, raw buffer dimensions, weighted area sampling structure, edge weights, color clamp/pack order, and Rust-facing deltas; Medium for exact prose labels on every FPU stack temporary because the decompiler still obscures some stack names, but the assembly pattern resolves the mechanism.  
**Active in YR:** Yes. Evidence: ordinary sidebar path reaches `PowerClass__Draw -> RadarClass__Draw -> RadarClass__Update`; `RadarClass__RebuildRadarSurfaces @ 0x00654650` calls `RadarClass__GenerateTerrainSurface @ 0x006547C0`, and `RadarClass__One_Time @ 0x00652CF0` / `RadarClass__Init_For_House @ 0x00652E90` initialize the live in-game radar fields.

## Working Notes Gate

Target question: What exact generated radar surface dimensions, zoom factor, raw RGB buffer dimensions, and sampling math does gamemd use for the live in-game minimap?  
Non-goals: Do not re-investigate settled aperture/chrome facts, object-dot ordering, click inverse transform, generic dirty callers, or transition assets.  
Evidence needed to mark COMPLETE: Ghidra decompile plus assembly for sizing branches, zoom constants, raw-buffer allocation/write dimensions, sampling loop weights, color clamp/pack, and Rust surface comparison.  
Stop conditions: stop before mutating Ghidra, stop before changing Rust/INI files, stop before broadening into non-live shell/transition radar paths, and stop if exact sampling cannot be proven beyond decompile ambiguity.

## 1. Overview

The live in-game minimap terrain is not a 200x200 texture stretched to the sidebar aperture. `RadarClass` first computes a raw radar-space rectangle from valid cells, allocates a 3-byte RGB buffer of exactly `raw_w * raw_h`, fills it from `CellClass__GetRadarColor`, then creates a 16-bit `BSurface` whose size is aspect-fit into max `140x108`.

`RadarClass__GenerateTerrainSurface` writes the generated secondary surface directly. Its inner loop is a weighted area sampler over the raw RGB buffer: source-pixel edge overlaps are fractional, interior overlaps are `1.0`, accumulated colors are normalized, biased by `+0.5`, converted with `Math__ftol`, clamped to `255`, packed through DirectDraw loss/shift globals, and written as 16-bit pixels.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose | Active in YR |
|---|---:|---|---|
| `RadarClass+0x11F0` | int | sidebar-local minimap aperture base X, normally `16` after `Init_For_House` | Yes; `0x00652E90` |
| `RadarClass+0x11F4` | int | sidebar-local minimap aperture base Y, `49` | Yes; `0x00652CF0` |
| `RadarClass+0x11F8` / `+0x1200` | int | max radar width constants, `140` | Yes; `0x00652CF0` |
| `RadarClass+0x11FC` / `+0x1204` | int | max radar height constants, `108` | Yes; `0x00652CF0` |
| `RadarClass+0x121C` | `DSurface*` | primary/live radar surface cloned from generated secondary size | Yes; `0x00654650` |
| `RadarClass+0x1220` | `BSurface*` | secondary/generated terrain surface, 16-bit pixels | Yes; `0x006547C0` |
| `RadarClass+0x123C` | `byte*` | raw terrain RGB buffer, 3 bytes per raw radar-space pixel | Yes; `0x006547C0` |
| `RadarClass+0x1240` | int | raw RGB buffer stride/width | Yes; `0x006547C0` |
| `RadarClass+0x1244` | int | raw RGB buffer height | Yes; `0x006547C0` |
| `RadarClass+0x1488` | float | zoom factor used by sampling and inverse transforms | Yes; `0x006547C0` |
| `RadarClass+0x149C..0x14A8` | rect | generated surface destination x/y/w/h in sidebar-local radar aperture | Yes; `0x00654650` |
| `RadarClass+0x1490/+0x1498` | int | radar-space iso offsets for cell projection | Yes; `0x00654490`, `0x00654EA0` |

## 3. Core Logic

### 3.1 Fixed live aperture constants

Active in YR: Yes.

`RadarClass__One_Time @ 0x00652CF0` initializes the live in-game radar maximum content dimensions to `140x108` (`0x8C x 0x6C`) and the base content Y to `49`. `RadarClass__Init_For_House @ 0x00652E90` sets `+0x11F0` from the sidebar width branch; both standard branch formulas produce `16` with `g_SIDEBAR_WIDTH_CONST=168`.

Evidence: decompile `0x00652CF0`, decompile `0x00652E90`, prior ordinary in-game path proof in `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`.

### 3.2 Raw radar-space bounds and dimensions

Active in YR: Yes.

`RadarClass__ComputeRadarMapBounds @ 0x00654490` iterates valid cells and expands a raw radar-space rectangle. Each valid cell projects to:

```text
raw_x = RadarClass+0x1490 - cell_y + cell_x
raw_y = cell_y - RadarClass+0x1498 + cell_x
```

The projected cell normally contributes width `2` and height `1`. If `raw_x == -1`, it is corrected to `0` and width becomes `1`; if `raw_x == map_width_iso * 2 - 1`, width also becomes `1`. The rectangle stored at `+0x149C/+0x14A0/+0x14A4/+0x14A8` before rebuild is the source raw rect passed to `GenerateTerrainSurface`.

Evidence: decompile `0x00654490`; assembly edge-correction range `0x006545A0..0x006545D0` shows the `-1` and right-edge width-one cases.

### 3.3 Raw RGB allocation and fill dimensions

Active in YR: Yes.

`RadarClass__GenerateTerrainSurface @ 0x006547C0` allocates `param_3[2] * param_3[3] * 3` bytes for `RadarClass+0x123C` only when the raw buffer pointer is null. It zeroes the allocation, then stores:

```text
RadarClass+0x1240 = param_3[2]  // raw stride / width
RadarClass+0x1244 = param_3[3]  // raw height
```

If the caller requests generation (`param_5 != 0`), it calls `RadarClass__FillTerrainColors @ 0x00654EA0` before sampling. If `param_5 != 0` and the destination dirty/source subrect has nonpositive width or height, `GenerateTerrainSurface` first copies the full source rect into that rect, making a full-surface generation request.

Evidence: decompile `0x006547C0`; allocation/zero/store assembly `0x00654808..0x00654877`; fill call `0x0065487D..0x0065488C`.

### 3.4 Raw RGB writes from cells

Active in YR: Yes.

`RadarClass__FillTerrainColors @ 0x00654EA0` calls `CellClass__GetRadarColor` once per clipped valid cell and writes one or two RGB triples into `+0x123C` at `raw_y * stride + raw_x`. For interior cells it writes left color at the first raw pixel and right color at the adjacent raw pixel. For the left clipped edge (`projected_x == rect.x - 1`) it writes only the right color. For the right clipped edge (`projected_x == rect.x + rect.w - 1`) it writes only the left color.

This matters because the raw buffer is already edge-corrected before zoom sampling; an implementation that averages cell colors into a single per-cell pixel loses the native left/right half-pixel source.

Evidence: decompile `0x00654EA0`; branch stores around `iVar6 == *param_3 - 1` and `iVar6 == param_3[2] - 1 + *param_3`.

### 3.5 Zoom factor and generated surface size

Active in YR: Yes.

When `RadarClass+0x1220` is null, `GenerateTerrainSurface` computes:

```text
candidate_zoom = 140.0f / raw_w
if raw_h * candidate_zoom < 108.0f:
    zoom = candidate_zoom
    generated_w = 140
    generated_h = Math__ftol(raw_h * zoom)
else:
    zoom = 108.0f / raw_h
    generated_w = Math__ftol(raw_w * zoom)
    generated_h = 108
RadarClass+0x1488 = zoom
```

The equality case goes to the height-constrained branch, but produces the same size for exact-fit cases. The dimension conversion uses `Math__ftol @ 0x007C5F00` directly; the call is `FISTP` under the stored FPU control word `DAT_00822D80`, not a Rust `as` cast. No `+0.5` bias is added at the generated-width/generated-height call sites.

Evidence: decompile `0x006547C0`; assembly `0x006548B3..0x00654939` for constants/branch/store; `Math__ftol` assembly `0x007C5F00..0x007C5F3C`.

### 3.6 Native sampling is weighted area integration, not nearest-neighbor

Active in YR: Yes.

After locking the secondary surface (`vtable+0x5C`), the sampling loop maps each generated output pixel to a fractional raw-buffer rectangle. The setup computes raw-per-output steps from raw dimensions and generated dimensions, then iterates the integer raw source pixels overlapping each output pixel.

For each output pixel:

```text
src_y0 = output_y * (raw_h / generated_h)
src_y1 = src_y0 + (raw_h / generated_h)
src_x0 = output_x * (raw_w / generated_w)
src_x1 = src_x0 + (raw_w / generated_w)

for sy in floor(src_y0) .. min(floor(src_y1) + 1, raw_h):
    y_weight =
        if only one source row overlaps: raw_h / generated_h
        else if first source row: (sy + 1) - src_y0
        else if last source row: src_y1 - sy
        else: 1.0
    for sx in floor(src_x0) .. min(floor(src_x1) + 1, raw_w):
        x_weight =
            if only one source column overlaps: raw_w / generated_w
            else if first source column: (sx + 1) - src_x0
            else if last source column: src_x1 - sx
            else: 1.0
        weight = x_weight * y_weight * normalization
        accum.B += raw_byte0 * weight
        accum.G += raw_byte1 * weight
        accum.R += raw_byte2 * weight
```

The loop initializes three accumulators from `0.0f` and uses `1.0f` for fully covered interior source pixels. It loads raw triples at `raw_buffer + (sy * raw_w + sx) * 3`, multiplying each channel by the computed weight before accumulation. The normalization factor is precomputed from the raw/output scale product, so the accumulated values are already averaged before conversion.

Evidence: assembly setup `0x006549BB..0x006549EC`; row/column extent setup `0x00654AF5..0x00654BB5`; y-edge weights `0x00654BE3..0x00654C1E`; x-edge weights `0x00654C49..0x00654C8C`; raw triple loads and weighted accumulations `0x00654C96..0x00654CD2`.

### 3.7 Color conversion, clamp, and packed write order

Active in YR: Yes.

After accumulating a generated pixel, the function converts channels in stack/FPU order with `+0.5` bias before `Math__ftol`, clamps each channel to `0xFF`, and writes a 16-bit surface pixel:

```text
B = min(Math__ftol(accum.B + 0.5), 255)
G = min(Math__ftol(accum.G + 0.5), 255)
R = min(Math__ftol(accum.R + 0.5), 255)

packed =
    ((R >> g_DD_RLoss) << g_DD_RShift) |
    ((G >> g_DD_GLoss) << g_DD_GShift) |
    ((B >> g_DD_BLoss) << g_DD_BShift)
```

The raw buffer byte order is RGB as written by `FillTerrainColors`, but the FPU stack emits the clamp/pack sequence through temporaries that decompile as B, G, then R. The final shifts use `g_DD_RLoss/RShift`, `g_DD_GLoss/GShift`, and `g_DD_BLoss/BShift`, so the output is display-format 16-bit, not RGBA.

Evidence: raw byte loads `0x00654CA0`, `0x00654CAC`, `0x00654CB7`; conversion/clamps `0x00654D29..0x00654D6D`; pack/write `0x00654D72..0x00654DBC`; `Math__ftol @ 0x007C5F00`.

### 3.8 Generated surface centering and primary surface cloning

Active in YR: Yes.

`RadarClass__RebuildRadarSurfaces @ 0x00654650` destroys old secondary/raw/visited buffers, regenerates the secondary surface, reads generated width/height from `+0x1220` vtable `+0x7C/+0x80`, stores them in `+0x14A4/+0x14A8`, then centers smaller generated surfaces inside the `140x108` aperture:

```text
dest_x = RadarClass+0x11F0
if generated_w < 140:
    dest_x += (140 - generated_w) / 2

dest_y = RadarClass+0x11F4
if generated_h < 108:
    dest_y += (108 - generated_h) / 2
```

The half-margin uses signed integer division sequence `CDQ; SUB; SAR 1`; because the guarded value is nonnegative, this is floor division. The primary surface at `+0x121C` is then constructed with the same generated width and height as the secondary surface, not the max aperture size.

Evidence: decompile `0x00654650`; assembly `0x006546E0..0x00654742`; primary clone construction `0x0065474A..0x006547A8`.

## 4. INI Keys

No INI key controls the live generated radar surface maximum dimensions, zoom formula, raw buffer size, or sampling method in this slice. The relevant constants are binary constants/fields (`140.0f @ 0x007F0420`, `108.0f @ 0x007F041C`, `0x8C`, `0x6C`) initialized and consumed by `RadarClass`.

Active in YR: Yes; no TS/YR optional gate was found in the sizing/sampling path. Fog/shroud and object overlays are outside this slot.

## 5. Integration Points

| Point | Status | Evidence | Active in YR |
|---|---|---|---|
| Ordinary sidebar draw reaches radar draw/update | verified by sibling docs | `PowerClass__Draw -> RadarClass__Draw`, `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md` | Yes |
| Scenario/radar initialization sets aperture fields | verified | `0x00652CF0`, `0x00652E90` | Yes |
| Map bounds computed before rebuild | verified | `0x00654490`; `RadarClass__Init` prior-doc path | Yes |
| Surface rebuild calls generation | verified | `0x00654650` calls `0x006547C0` | Yes |
| Generation calls raw color fill when requested | verified | `0x0065487D..0x0065488C` | Yes |
| Generated secondary copied/cloned into primary size | verified | `0x00654650`, `0x0065474A..0x006547A8` | Yes |

## 6. Current Rust Implementation Status

Current Rust does not match the native mechanism:

- `src/render/minimap_helpers.rs:16` hardcodes `MINIMAP_SIZE = 200`; native generated surface is `<=140x108`.
- `src/render/minimap.rs:90..172` allocates a square `200x200` RGBA texture and maps cells directly into it; native allocates raw RGB at raw radar-space bounds, then weighted-samples into a 16-bit `BSurface`.
- `src/render/minimap_helpers.rs:167..198` aspect-fits world/screen extents into `200x200`; native aspect-fits raw radar-space dimensions into `140x108` and stores the zoom at `RadarClass+0x1488`.
- `src/render/minimap.rs:511..532` stretches the whole minimap texture into the caller-provided rectangle; native blits a generated surface already sized to its aspect-fit dimensions, centered in the `140x108` aperture when smaller.
- `src/render/minimap_helpers.rs:133..158` averages TMP left/right colors into one cell pixel before mapping; native preserves one/two raw half-pixels and lets weighted area sampling combine them later.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RadarClass__One_Time` max dimensions | verified | `0x00652CF0` | none |
| `RadarClass__Init_For_House` X base | verified | `0x00652E90` | semantic label of scenario flag remains sibling-doc territory |
| `RadarClass__ComputeRadarMapBounds` raw rectangle and edge width | verified | `0x00654490`, `0x006545A0..0x006545D0` | none for sizing |
| Raw RGB allocation/stride/height | verified | `0x006547C0`, `0x00654808..0x00654877` | none |
| `FillTerrainColors` one/two raw-pixel writes | verified | `0x00654EA0` | exact `CellClass__GetRadarColor` color tree owned by slot 3 / prior report |
| Zoom branch and generated dimensions | verified | `0x006548B3..0x00654939`, `0x007C5F00` | live FPU control-word value not runtime-sampled |
| Weighted sampling loop | verified | `0x006549BB..0x00654CD2` | none for mechanism; variable names are inferred from FPU stack use |
| Color clamp/pack/write | verified | `0x00654D29..0x00654DBC` | exact runtime DD bitfield values deferred to runtime display-mode sampling |
| Centering inside aperture | verified | `0x006546E0..0x00654742` | none |
| Rust comparison | verified by source scan | `src/render/minimap.rs`, `src/render/minimap_helpers.rs` | implementation not changed |

## 8. Open Questions - Final State

- `[RESOLVED] Q1` - Which path creates the live secondary terrain surface? -> `RadarClass__RebuildRadarSurfaces` calls `RadarClass__GenerateTerrainSurface`. (evidence: `0x00654650`, `0x006546D3`)
- `[RESOLVED] Q2` - What is the native max display area? -> `140x108`, initialized as binary constants/fields. (evidence: `0x00652CF0`)
- `[RESOLVED] Q3` - What are raw RGB buffer dimensions? -> `raw_w=param_3[2]`, `raw_h=param_3[3]`, size `raw_w*raw_h*3`, stored at `+0x1240/+0x1244`. (evidence: `0x00654808..0x00654877`)
- `[RESOLVED] Q4` - Does native create a square 200x200 surface? -> No, generated surface is aspect-fit `<=140x108`; no 200 constant appears in this path. (evidence: `0x006548B3..0x00654939`)
- `[RESOLVED] Q5` - How is zoom chosen? -> Width candidate `140/raw_w`; if scaled height is not below `108`, height branch uses `108/raw_h`. (evidence: `0x006548B3..0x00654930`)
- `[RESOLVED] Q6` - What rounding is used for generated dimensions? -> `Math__ftol`/`FISTP` under stored FPU control word, without `+0.5` at dimension call sites. (evidence: `0x006548F7`, `0x0065491A`, `0x007C5F00..0x007C5F3C`)
- `[RESOLVED] Q7` - Is sampling nearest-neighbor? -> No, it is weighted area integration over overlapping raw pixels. (evidence: `0x00654BE3..0x00654CD2`)
- `[RESOLVED] Q8` - Are edge pixels fractionally weighted? -> Yes; first/last source rows/columns use fractional overlap, interior uses `1.0f`. (evidence: `0x00654BE3..0x00654C8C`)
- `[RESOLVED] Q9` - How are accumulated colors converted? -> `+0.5`, `Math__ftol`, clamp to `255`, pack through DD shifts/losses. (evidence: `0x00654D29..0x00654DBC`)
- `[RESOLVED] Q10` - Does native stretch the generated surface to fill `140x108` at blit time? -> No; smaller generated surfaces are centered and blitted at their own width/height. (evidence: `0x006546E0..0x00654742`, sibling content-inset report)
- `[RESOLVED] Q11` - Does raw fill preserve left/right half pixels? -> Yes, interior cells write two adjacent RGB triples, edge cases write one side. (evidence: `0x00654EA0`)
- `[DEFERRED] Q12` - What exact DD RGB565/RGB555 values are active on a user's runtime display? (category: needs-runtime-debugger; reason: globals `g_DD_*Loss/*Shift` are runtime display-format fields; next-step-if-pursued: sample them in a live process)
- `[DEFERRED] Q13` - Are all `CellClass__GetRadarColor` branches fully decoded here? (category: out-of-scope; reason: slot 3 owns color source pipeline; next-step-if-pursued: verify or extend `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`)
- `[DEFERRED] Q14` - Does every dirty subrect caller pass non-empty rects? (category: out-of-scope; reason: generic dirty pipeline is slot 3; next-step-if-pursued: trace `MarkTerrainDirty` callers)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / surface | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `RadarClass__ComputeRadarMapBounds @ 0x00654490` | scenario/map init before rebuild | no surface | raw radar-space bounds | none | Yes | source raw rect |
| 2 | `RadarClass__GenerateTerrainSurface @ 0x006547C0` | called by rebuild; `param_5=1` for full generation | raw RGB `+0x123C` | `raw_w*raw_h*3` | RGB bytes from `CellClass__GetRadarColor` | Yes | raw terrain source |
| 3 | `RadarClass__GenerateTerrainSurface @ 0x006547C0` | secondary surface null or existing | `BSurface +0x1220` | `generated_w<=140`, `generated_h<=108` | weighted RGB -> DD 16-bit | Yes | generated terrain |
| 4 | `RadarClass__RebuildRadarSurfaces @ 0x00654650` | after generated secondary exists | primary `DSurface +0x121C` | clone of generated w/h | 16-bit surface clone | Yes | live display backing |
| 5 | `RadarClass__RebuildRadarSurfaces @ 0x00654650` | generated smaller than aperture | sidebar-local dest fields `+0x149C..+0x14A8` | centered in `(16,49,140,108)` | no stretch | Yes | content placement |

Asset role matrix:

| Asset / surface | Loaded | Drawn | Visible in target | Content | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| raw RGB buffer `+0x123C` | runtime allocated | sampled, not directly drawn | indirectly | Yes | No | No | No | No | `0x006547C0` |
| secondary `BSurface +0x1220` | runtime allocated | read/blitted/restored by radar update paths | indirectly | Yes | No | No | No | No | `0x006547C0`, `0x00654650` |
| primary `DSurface +0x121C` | runtime allocated | blitted to sidebar | Yes | Yes | No | object/fog/event overlays later | No | No | `0x00654650`, prior `0x00656EC0` |
| Rust `200x200` RGBA minimap texture | Rust-only | Rust draws as sprite | Rust-only | Rust content | No | mixed | No | Native path does not use it | `src/render/minimap.rs:90..172` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native generated minimap terrain surface is sized by raw radar-space bounds aspect-fit into max `140x108`; equality goes through height branch but exact-fit result remains same. | `0x006548B3..0x00654939`, `0x00652CF0` | Rust uses square `MINIMAP_SIZE=200` and a later stretch into the UI rect. | `src/render/minimap.rs`, `src/render/minimap_helpers.rs`, `src/app_render/build_instances.rs` | Generate/render the minimap content at native `generated_w/generated_h`, store native zoom, and center it in the `140x108` aperture rather than stretching a square texture. | A raw radar source `raw_w=300, raw_h=180` yields `generated_w=140`, `generated_h=Math__ftol(180*(140/300))`, content blitted at native generated size. Proposed test: `test_minimap_generated_surface_aspect_fits_raw_bounds_into_140x108`. | HIGH; do not keep a 200x200 square intermediate as the parity surface. |
| Native sampling is weighted area integration over raw RGB triples with fractional first/last row/column weights and `1.0` interior weights, then normalized. | `0x00654BE3..0x00654CD2` | Rust maps each cell to a single texture pixel and uses GPU stretch/filtering for final display. | `src/render/minimap.rs`, `src/render/minimap_helpers.rs` | Implement CPU-side native weighted sampling from raw RGB buffer into 16-bit/display-equivalent pixels before any UI blit. | A checkerboard raw buffer downsampled to a smaller generated surface matches native weighted averages at edge and interior output pixels. Proposed test: `test_minimap_zoom_sampling_uses_native_weighted_area_filter`. | HIGH pixel drift; do not use nearest-neighbor, bilinear texture sampling, or final-sprite stretching as a substitute. |
| Raw fill preserves one/two horizontal half-pixels per cell before zoom; clipped left edge writes right color only and clipped right edge writes left color only. | `0x00654EA0`, `0x00654490` | Rust averages `radar_left`/`radar_right` into one RGBA cell pixel before mapping. | `src/render/minimap_helpers.rs::radar_color_for_cell`, `src/render/minimap.rs::new` | Build a raw radar-space RGB buffer with native two-pixel cell footprints and edge handling before zoom sampling. | A cell with distinct left/right radar colors contributes two adjacent raw pixels and downsampled output differs from pre-averaging. Proposed test: `test_minimap_raw_buffer_preserves_cell_left_right_half_pixels`. | MEDIUM-HIGH; do not average TMP left/right radar colors before native sampling. |
| Generated colors are converted with `+0.5`, `Math__ftol`, clamp to `255`, and DD loss/shift packing. | `0x00654D29..0x00654DBC`, `0x007C5F00` | Rust stores RGBA8 and uses float `.round()` in helpers; no display-format packing equivalence surface exists. | `src/render/minimap.rs`, pixel-format abstraction/render upload path | Add a native-pack equivalent or a test-visible packed pixel path for parity, then convert to RGBA only as a final presentation step if needed. | Weighted channel value `n + 0.49` and `n + 0.50` follow the native `+0.5` plus `Math__ftol` route and pack with configured loss/shift globals. Proposed test: `test_minimap_sampled_color_uses_native_bias_clamp_and_dd_pack`. | MEDIUM; do not compare post-GPU RGBA colors without accounting for native 16-bit pack loss. |

## Negative Facts / Do Not Do

- Do not implement live minimap terrain as a `200x200` square parity texture. Native creates a `BSurface` at `generated_w<=140`, `generated_h<=108`. Evidence: `0x006548B3..0x00654939`; Rust mismatch at `src/render/minimap_helpers.rs:16`.
- Do not stretch a generated image to fill `140x108` in the ordinary in-game aperture. Native centers smaller generated surfaces and blits their own dimensions. Evidence: `0x006546E0..0x00654742`.
- Do not use nearest-neighbor, bilinear GPU filtering, or "one cell -> one minimap pixel" as the zoom mechanism. Native uses weighted area integration over raw RGB source pixels. Evidence: `0x00654BE3..0x00654CD2`.
- Do not average `RadarLeft` and `RadarRight` before constructing the raw radar buffer. Native writes left/right into separate adjacent raw RGB triples except at clipped edges. Evidence: `0x00654EA0`.
- Do not treat raw RGB buffer dimensions as map cell width/height or world screen bounds. They are the computed radar-space rectangle dimensions from `ComputeRadarMapBounds`. Evidence: `0x00654490`, `0x006547C0`.
- Do not use Rust `as u8` truncation or RGBA-only comparison as a substitute for native color conversion; native applies `+0.5`, `Math__ftol`, clamps, and 16-bit DD loss/shift packing. Evidence: `0x00654D29..0x00654DBC`.

## Remaining Uncertainty

- Runtime DirectDraw loss/shift globals were not sampled from a live process; the formula and globals are verified, but the active RGB565/RGB555 values remain runtime-display dependent.
- `Math__ftol` uses `FISTP` under `DAT_00822D80`; this report proves call placement and bias/no-bias differences, but did not runtime-sample the stored control word.
- Full `CellClass__GetRadarColor` branch inventory remains delegated to `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`; this report only proves how its RGB triples are buffered and sampled.
- Generic dirty subrect caller coverage remains a separate slot; this report verifies how `GenerateTerrainSurface` handles a nonpositive dirty rect by copying the full source rect.

## Stale Docs / Follow-up Docs

- `docs/research/RADAR_MINIMAP_RENDERING.md`: replace "zoomed_w = round(buffer_width * zoom); zoomed_h = round(buffer_height * zoom)" with "when width-constrained, gamemd sets `generated_w=140` and `generated_h=Math__ftol(raw_h*zoom)`; when height-constrained, it sets `generated_h=108` and `generated_w=Math__ftol(raw_w*zoom)`; equality goes through the height branch. `Math__ftol` is a `FISTP` helper under the stored FPU control word, not a Rust cast."
- `docs/research/RADAR_MINIMAP_RENDERING.md`: refine "Weighted area average" to "weighted area integration over raw RGB triples: first/last overlapping source rows and columns use fractional overlap, interior rows/columns use `1.0f`, samples are multiplied by a precomputed normalization factor, channels are biased by `+0.5`, `Math__ftol` converted, clamped to `255`, and packed into the 16-bit DD surface."
- `docs/research/RADAR_MINIMAP_DEEP_DIVE.md`: replace "actual averaging/accumulation is unclear" in the sampling loop section with "resolved by `RADAR_SURFACE_SIZING_ZOOM_SAMPLING_GHIDRA_REPORT.md`: the loop is a normalized weighted area sampler, with edge-fraction weights at `0x00654BE3..0x00654C8C` and raw RGB weighted accumulations at `0x00654C96..0x00654CD2`."
- `src/render/minimap.rs` comment-level stale wording: replace "The minimap stretches to fill the entire container, matching the original RA2 behavior" with "Native gamemd generates an already aspect-fit radar surface `<=140x108` and centers it inside the aperture; stretching a square texture is a current approximation."

## Sources

- Ghidra read-only decompile: `0x00652CF0`, `0x00652E90`, `0x00654490`, `0x00654650`, `0x006547C0`, `0x00654EA0`, `0x006550C0`, `0x007C5F00`.
- Ghidra read-only assembly contexts: `0x00654808..0x00654877`, `0x006548B3..0x00654939`, `0x006549BB..0x006549EC`, `0x00654AF5..0x00654BB5`, `0x00654BE3..0x00654C8C`, `0x00654C96..0x00654CD2`, `0x00654D29..0x00654DBC`, `0x006546E0..0x00654742`, `0x007C5F00..0x007C5F3C`.
- Existing docs used as navigation/cross-checks: `docs/research/RADAR_MINIMAP_RENDERING.md`, `docs/research/RADAR_MINIMAP_DEEP_DIVE.md`, `docs/research/SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`, `docs/research/MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`.
- Rust scan: `src/render/minimap.rs`, `src/render/minimap_helpers.rs`.

## Status

COMPLETE for the scoped live in-game radar surface sizing, zoom selection, raw RGB dimensions, weighted sampling, centering behavior, and Rust-facing implementation handoff. Remaining uncertainties are runtime DD bitfield sampling, runtime FPU control-word sampling, full color-source branch inventory, and broader dirty caller coverage.
