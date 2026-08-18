# Skirmish Preview STARTBUT Marker Layout Trace - 800x600

**Scenario:** Fresh standard offline YR Skirmish at `800x600`, selected retail map `CrctBrd.yro`, which has `[Header]` start positions and `[Preview]` / `[PreviewPack]` data.
**Scope:** Parent Skirmish preview child `0x468`, fitted PreviewPack image rect, `[Header]` start projection, `STARTBUT.SHP` frame-0 top-lefts, numeric label origins/color source, and clipping boundary for this concrete sample.
**Write constraint:** This trace is the only file written for slot 4.

## Verdict Tally

PASS: 8 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Pipeline

`Skirmish WM_PAINT` -> child `0x468` rect -> selected map PreviewPack texture -> integer aspect-fit -> `[Header]` start projection -> `STARTBUT.SHP` frame `0` -> numeric labels -> destination-surface clipping/screen pixels.

## Concrete Retail Input

Retail installed map evidence: `<ra2-install>/CrctBrd.yro`.

Relevant data:

```text
[Header]
StartX=216
StartY=48
Width=80
Height=81
NumberStartingPoints=4
Waypoint1=224,56
Waypoint2=290,55
Waypoint3=223,123
Waypoint4=289,124

[Preview]
Size=0,0,160,81
[PreviewPack]
...
```

Rust parses the same data from `[Header]` only in `src/app_list_maps.rs:326` and parses four-field `[Preview] Size=` as width/height in `src/map/preview.rs:108`.

## Stage Results

### 1. Active YR Paint Path

gamemd: Standard offline Skirmish `WM_PAINT` calls `DrawStartPositions @ 0x00640710` through `FUN_006AE3F0` when the preview object exists and child `0x468` is not suppressed. Active in YR: yes, conditional on a selected preview object. Evidence: `docs/research/skirmish-ui/SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md:20`.

Rust: `render_skirmish_shell_with_atlas` computes layout, ensures selected preview texture, builds preview instance, marker sprites, and marker labels in `src/app_skirmish_shell_render.rs:390`, `:411`, `:422`, `:443`, `:489`.

Verdict: PASS for active route parity in this selected-map condition.

### 2. Child `0x468` Rect

gamemd: At `800x600`, child `0x468` final rect is `(644,37,144,112)`. Evidence: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md:31-44`.

Rust: `compute_layout(800,600)` expects `layout.map_preview = RectPx::new(644,37,144,112)` in `src/ui/skirmish_shell/layout.rs:885`.

Verdict: PASS.

### 3. PreviewPack Source Dimensions

gamemd: Preview object surface bounds feed aspect-fit through preview vtable, then blit before overlays. Evidence: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md:53-60`.

Rust: `CrctBrd.yro` `[Preview] Size=0,0,160,81` becomes decoded preview dimensions `160x81`; `build_preview_surface_instance` uses those dimensions in `src/app_skirmish_shell_render/preview.rs:267`.

Verdict: PASS for source dimensions and preview-before-overlay ordering.

### 4. Integer Aspect Fit

gamemd formula: integer per-mille truncation, no float rounding. Evidence: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md:69-84`.

Computed for child `(644,37,144,112)` and source `160x81`:

```text
scale_w = 144000 / 160 = 900
scale_h = 112000 / 81 = 1382
scale = 900
fit_w = 160 * 900 / 1000 = 144
fit_h = 81 * 900 / 1000 = 72
fit_x = 644 + 144/2 - (160*900)/2000 = 644
fit_y = 37 + 112/2 - (81*900)/2000 = 57
fitted_preview_rect = (644,57,144,72)
```

Rust: identical integer formula in `src/app_skirmish_shell_render/preview.rs:247`.

Verdict: PASS.

### 5. Marker Anchor Projection

gamemd formula: `x_per_mille = trunc((WaypointX-StartX)*1000/Width)`, `anchor_x = fit_x + trunc(x_per_mille*fit_w/1000)`, equivalent for Y. Evidence: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md:90-98`.

Rust: same formula in `src/app_skirmish_shell_render/preview.rs:52`.

Concrete anchors:

| Label | Per-mille `(x,y)` | Anchor `(x,y)` |
|---:|---:|---:|
| 1 | `(100,98)` | `(658,64)` |
| 2 | `(925,86)` | `(777,63)` |
| 3 | `(87,925)` | `(656,123)` |
| 4 | `(912,938)` | `(775,124)` |

Verdict: PASS.

### 6. `STARTBUT.SHP` Marker Rects

gamemd: `STARTBUT.SHP` frame `0`, top-left `(anchor_x-9, anchor_y-6)`. Evidence: `SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md:85-94`.

Rust: loads optional `STARTBUT.SHP` frame `0` into the shell atlas in `src/render/skirmish_shell_chrome.rs:259`, stores it as `start_marker` in `src/render/skirmish_shell_chrome.rs:392`, and applies `(anchor_x-9, anchor_y-6)` in `src/app_skirmish_shell_render/preview.rs:45`.

Concrete STARTBUT top-lefts:

| Label | Anchor | STARTBUT top-left |
|---:|---:|---:|
| 1 | `(658,64)` | `(649,58)` |
| 2 | `(777,63)` | `(768,57)` |
| 3 | `(656,123)` | `(647,117)` |
| 4 | `(775,124)` | `(766,118)` |

Verdict: PASS.

### 7. Numeric Label Origin And Numbering

gamemd: Label is `i+1`, origin `(anchor_x-2, anchor_y-6)`, after the sprite block. Evidence: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md:104-121`.

Rust: label is `idx + 1`, origin `(anchor_x-2, anchor_y-6)` in `src/app_skirmish_shell_render/text.rs:878` and `:899`.

Concrete label origins:

| Label | Origin |
|---:|---:|
| 1 | `(656,58)` |
| 2 | `(775,57)` |
| 3 | `(654,117)` |
| 4 | `(773,118)` |

Verdict: PASS for origin and numbering. UNCHECKED for final glyph raster bounds and exact post-conversion pixel color; verified docs identify the gamemd color source as `"Yellow"`, while current Rust uses `SHELL_LABEL_TEXT_RGB = [1.0,1.0,0.0]` in `src/app_skirmish_shell_render/text.rs:903`, but this trace did not compute gamemd's display-format converted numeric RGB.

### 8. Clipping Boundary

gamemd: Live STARTBUT overlays are not clipped to the fitted preview rect and are not rejected when anchors fall outside it; clipping is by destination surface through `CC_Draw_Shape` / `AlphaShapeClass__ClipRect`. Evidence: `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md:48-79`.

Rust: `project_preview_start_positions` returns anchors without containment rejection, `push_start_marker_sprites` submits every projected position, and marker draws are issued before any text scissor is set in `src/app_skirmish_shell_render/preview.rs:30`, `src/app_skirmish_shell_render.rs:561`, and `src/app_skirmish_shell_render.rs:589`.

Concrete `CrctBrd.yro` result: all four `18x18` STARTBUT rects are inside the `800x600` destination surface, so the active clip intersection is the full marker rect for both gamemd and Rust. No fitted-preview clipping is applied.

Verdict: PASS for this concrete sample. UNCHECKED for a rendered pixel sample where a marker is partially outside the destination/backbuffer, because this map does not exercise that boundary.

## Failures / Not Implemented

None found in this slot for the concrete `CrctBrd.yro` 800x600 layout and marker positions.

## Adjacent Findings

- Older `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md` lines `164-168` describe Rust mismatches that are stale for the current source: current Rust has integer aspect-fit, corrected label origin/color source, and an outside-fitted-preview projection test.
- Exact font glyph pixels for numeric labels remain outside this slot; this trace only verifies the caller origin, numbering, and color source.
- A backbuffer-edge render test would make destination-surface clipping parity stronger, but `CrctBrd.yro` does not place any start marker at that edge.

## Sources

- `docs/research/skirmish-ui/SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SCENARIO_PREVIEW_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md`
- `<ra2-install>/CrctBrd.yro`
- `src/app_skirmish_shell_render.rs`
- `src/app_skirmish_shell_render/preview.rs`
- `src/app_skirmish_shell_render/text.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/app_list_maps.rs`
- `src/map/preview.rs`
- `src/render/skirmish_shell_chrome.rs`
