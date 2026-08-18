# GenerateTerrainPreview RandMap Dimensions / Colors - Ghidra Research Report

**Date:** 2026-05-23  
**Target:** `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS`  
**Address(es):** `GenerateTerrainPreview @ 0x00641140`, random-map dialog proc `0x00596300`, random-map generator `0x00598960`, dialog shutdown writer `0x00595BC0`, PCX-style writer `0x007B05C0`, PCX-style loader `0x00641DB0`, preview paint consumer `DrawStartPositions @ 0x00640710`  
**Investigation mode:** exhaustive-slice  
**Claimed scope:** random-map preview surface dimensions, generated-preview pixel/color channel expectations, baked `4x4` red start-marker inclusion and clipping boundary, `RandMap.img` final dimensions, and liveness through the random-map dialog / setup path.  
**Non-scope:** full random terrain-generation formulas, exact per-seed terrain palette choices, lower-level map-cell attribute generation, `.SED` layout, and live `STARTBUT.SHP` overlay clipping.
**Confidence:** High for surface dimension formula, `RandMap.img` dimension round-trip, marker inclusion ordering, fixed marker constants, writer channel class, and active random-map dialog liveness. Medium for screenshot-exact RGB of every generated terrain pixel because that would require draining all `CellClass__GetRadarPixelColor` / overlay / terrain color sources and runtime display pixel-format globals.
**Active in YR:** Yes / Conditional. The path is active in standard YR when the random-map dialog generates a preview and when preview-enabled random-map generation calls `GenerateTerrainPreview`; `RandMap.img` is written only if the random-map dialog has a non-null generated preview surface on shutdown.

## 0. Working Notes Gate

**Target question:** What are the generated random-map preview surface dimensions and pixel/color expectations that Rust must preserve when rendering `RandMap.img` previews?

**Non-goals:** Do not reconstruct full terrain generation formulas, do not redo `.SED` seed/options layout, do not investigate normal map `[PreviewPack]` decode beyond contrast, and do not expand into live `STARTBUT.SHP` overlay clipping.

**Evidence needed to mark COMPLETE:**

- active random-map dialog/generation path to `GenerateTerrainPreview`;
- exact generated surface width/height formula and proof that `RandMap.img` preserves those dimensions;
- terrain pixel color source class and direct-color / palette expectations enough for a decoder/render test;
- whether baked start markers are included in the generated surface before `RandMap.img` write;
- clipping / edge behavior for baked marker pixels at the generated-surface boundary;
- Rust implementation handoff and tests.

**Stop conditions:** Stop once the generated preview image contract is proven. Defer only screenshot-exact per-seed terrain colors or broad generator formulas that would require a separate terrain-generation investigation.

## 1. Overview

`GenerateTerrainPreview @ 0x00641140` creates the runtime preview surface used by the random-map dialog. It scans playable map cells, projects their cell centers into the preview coordinate space, allocates a new `DSurface`, draws one or two packed display-format pixels per playable cell, then paints baked `4x4` red rectangles for valid waypoint indices `0..7`. The random-map dialog shutdown path writes the resulting surface to `RandMap.img`; the writer records the actual source surface width and height, so the image is dynamically dimensioned from the generated map, not from the `0x468` UI control or a stock PreviewPack size.

Active in YR: Yes / Conditional. Evidence: random-map dialog command `0x620` in `0x00596300` calls `FUN_00598960(1, hwnd)` and then `GenerateTerrainPreview`; `0x00598960` itself calls `GenerateTerrainPreview` repeatedly when its preview flag argument is nonzero; dialog shutdown `0x00595BC0` writes `RandMap.img` only when `DAT_00ABE154` and wrapper `+0` exist.

## 2. Key Offsets And Globals

| Item | Purpose | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00ABE154` | random-map dialog preview wrapper; wrapper `+0` is generated preview `DSurface` | `0x00596300` paint/generate path; `0x00595BC0` writer guard | Conditional: random-map dialog |
| `GenerateTerrainPreview @ 0x00641140` | creates/replaces wrapper inner preview surface and paints terrain + baked markers | decompile and assembly around `0x00641170..0x00641898` | Conditional: generated preview |
| `MapClass singleton 0x0087F7E8` | iterated twice for bounds pass and pixel pass | calls at `0x00641161`, `0x006412A0` | Yes when map data exists |
| `DSurface__Constructor @ 0x004BA5A0` | surface allocation with width, height, flags `(1,0)` | call at `0x0064128C`; constructor writes `vtable__DSurface` and DD surface descriptor | Yes |
| `CellClass__GetRadarPixelColor` | base terrain radar/preview color source for each cell | call in pixel pass after `MapClass__Get_CellClass` | Yes |
| `OverlayClass__GetRadarColor` and overlay type RGB fields | overlay color replacement/tint path | overlay branch in `0x00641140` | Conditional on overlay at cell |
| `g_DD_*Loss/*Shift` globals | pack / unpack direct-color RGB into current display pixel format | terrain and marker pack code in `0x00641140`; writer unpack code in `0x007B05C0` | Yes |
| `ScenarioClass+0x632 + index*4` | waypoint table used for baked red markers | marker loop calls `0x0068BD80` and `0x0068BCC0` | Conditional on valid waypoints |
| `RandMap.img @ 0x00829ABC` | runtime preview image filename | string xref in `0x00595BC0`; loader xrefs in setup/chooser reports | Conditional |

## 3. Dimension Contract

### 3.1 Bounds pass uses playable cells only

Active in YR: Yes when `GenerateTerrainPreview` runs. The first pass initializes `min_x = 10000`, `min_y = 10000`, `max_x = 0`, `max_y = 0`, iterates `MapClass` cells, and only considers cells where `MapClass__Is_Cell_In_Playfield(cell, 0)` returns nonzero.

Evidence: `GenerateTerrainPreview` decompile and assembly `0x00641170..0x0064124F`. The playfield predicate is called before coordinate projection; out-of-playfield cells do not affect min/max.

Tiny details:

- Cell centers are used for bounds: `cell_x * 0x100 + 0x80`, `cell_y * 0x100 + 0x80`.
- Projection goes through `FUN_006D62E0`, then integer division by `0x3C` for X and `0x1E` for Y.
- The generated preview origin is the minimum projected cell coordinate from this pass, not `[Preview] Size=` and not the dialog rect.

### 3.2 Surface width and height

Active in YR: Yes when `GenerateTerrainPreview` allocates the preview surface. After the bounds pass, the old inner surface is destroyed if present, then the new `DSurface` is constructed with:

```text
width  = (max_projected_x - min_projected_x) * 2
height =  max_projected_y - min_projected_y
flags  = (1, 0)
```

Evidence: assembly `0x00641260..0x00641295`: `SUB EBP, EBX`, `SUB ESI, ECX`, `SHL EBP, 1`, then pushes `0`, `1`, `ESI`, `EBP` into `DSurface__Constructor @ 0x004BA5A0`. The constructor records width in object slot `+4` and height in slot `+8`.

Tiny details:

- The X dimension is doubled after subtracting projected bounds.
- The Y dimension is not doubled.
- There is no UI-control-size parameter in this allocation.
- A previous inner surface is destroyed before replacement; the new surface is assigned to `*param_1`.

### 3.3 `RandMap.img` preserves generated dimensions

Active in YR: Conditional on dialog shutdown with a generated preview surface. `0x00595BC0` opens `RandMap.img` and calls `0x007B05C0` with `DAT_00ABE154+0` as the source surface. The writer queries the source surface width and height through vtable slots `+0x7C` and `+0x80` and writes:

```text
xmin = 0
ymin = 0
xmax = width - 1
ymax = height - 1
hsize = width
vsize = height
bytes_per_line = width
```

Evidence: `0x00595BC0` writer guard and call; `0x007B05C0` decompile around its header setup.

The loader `0x00641DB0` constructs a temporary `BSurface`, requires nonzero temporary width and height, allocates a destination `DSurface` with those same dimensions, and copies the decoded image into wrapper `+0`.

Evidence: `0x00641DB0` decompile: width/height checks through vtable `+0x7C/+0x80`, destination `DSurface__Constructor(width, height, 1, 0)`, then copy/blit from temporary surface.

Implementation consequence: `RandMap.img` must be decoded with its own file dimensions. It must not be resized at decode time to `0x468`, `80x50`, `138x75`, `144x112`, or any stock map preview size. Paint later aspect-fits the source image into the preview child.

## 4. Pixel / Color Contract

### 4.1 Terrain pixels are packed direct-color surface pixels

Active in YR: Yes when `GenerateTerrainPreview` paints cells. For each playable cell, the function obtains a base radar color through `CellClass__GetRadarPixelColor`, then may replace/tint it through overlay/terrain-object branches. The final color values are packed through `g_DD_*Loss/*Shift` globals before fill calls into the generated `DSurface`.

Evidence: `GenerateTerrainPreview` pixel pass after `0x006412A0`; direct pack operations use `g_DD_RLoss/RShift`, `g_DD_GLoss/GShift`, and `g_DD_BLoss/BShift`. The marker path uses the same pack globals for RGB `(0xF0, 0, 0)`.

Material behavior:

- Base terrain comes from `CellClass__GetRadarPixelColor @ 0x0047BDB0`. **Drained 2026-07-22** (this bullet previously read "this report does not drain its full formula"). The function returns a PACKED PAIR of 16-bit display-format pixels: **high half = left pixel, low half = right pixel**. Three sources, in order:
  1. **Occupier** (skipped when the `ignore_occupier` arg is nonzero): taken only if `CellClass__FindOccupierByRTTI` finds one whose type has `+0xC9A == 0` and `+0x1701 == 0` and whose RTTI vtable slot `+0x68` returns `!= 5`. Colour is the owning house's colour scheme — `g_ColorSchemeArray[house->+0x16054]`, index `scheme->+0x314`, table `scheme->+0x30C`, read as a byte at `+0x174 + idx` when `obj[4] == 1` else a `u16` at `+0x174 + idx*2`. Returns `c<<16 | c`, so **both pixels are identical**.
  2. **Overlay** (`cell->+0x140 & 0x100`): `OverlayClass__GetRadarColor` RGB packed through `g_DD_{R,G,B}{Loss,Shift}`; if that packs to zero it falls back to `g_OverlayTypeClass_Array[24]` fields `+0x2B6/+0x2B7/+0x2B8`. Also returns `c<<16 | c` — **both pixels identical**.
  3. **Terrain** (the normal path, and the ONLY one that can yield two different pixels): tile index `cell->+0x38` (`0xFFFF` means `g_ClearTile`) into `g_IsometricTileTypeClass_Array`, then `IsometricTileTypeClass__GetRadarColorPair @ 0x00549E50` with sub-tile `cell->+0x11A` and slot `cell->+0x11B`. That helper lazily runs `TMP_Loader` when `this->+0xA4 == 0 && this->+0x2F4 != 0`, then returns `*(int*)(this->+0x2A8 + sub_tile*4) + slot*4` — a pointer to a 4-byte record holding two `u16` pixels. The caller returns `CONCAT22(ptr[0], ptr[1])`, i.e. `ptr[0]` is the left/high pixel and `ptr[1]` the right/low pixel. **This is the tile's RadarLeft/RadarRight pair.**

  Evidence: `decompile_function 0x0047BDB0`, `decompile_function 0x00549E50`, caller `decompile_function 0x00641140`.
- Overlay cells go through `OverlayClass__GetRadarColor`; overlay type fields at offsets near `+0x2A9` and `+0x2B6..+0x2B8` choose replacement/tint behavior.
- If either half of the chosen terrain color is zero, the function logs "black pixel" and substitutes packed gray RGB `(0x80,0x80,0x80)`.
- Some branches call `FUN_004BF650` with intensity-like constants such as `0x19`, `0x32`, `0x4B`, or `100`, yielding paired colors for the two preview pixels.

Implementation consequence: the generated image is not a palette-index map. For `RandMap.img`, Rust should preserve RGB/direct-color output from the decoded image instead of trying to reconstruct colors from terrain rules during UI preview rendering.

### 4.2 Two-pixel cell footprint

Active in YR: Yes. The pixel pass computes preview X as `(projected_x / 0x3C - min_x) * 2` and preview Y as `(projected_y / 0x1E - min_y)`. It then issues two adjacent surface fill/write calls: one at the computed X and one at `X + 1`, both on the same Y, using the selected paired colors.

Evidence: `GenerateTerrainPreview` decompile in the terrain pixel loop; after color selection, it calls surface vtable `+0x78` / `+0x88` once with `local_3c[0]` and once after `uStack_7C = iStack_4C + 1`. This matches the earlier `width = projected_delta_x * 2` allocation.

Material behavior:

- The generated source width is doubled because a single playable cell can emit two horizontal preview pixels.
- The two pixels may be identical or may use two values produced by `FUN_004BF650` depending on overlay/terrain branch.
- This is distinct from the later live `STARTBUT.SHP` overlay path.

### 4.3 `RandMap.img` writer channel class

Active in YR: Conditional on `RandMap.img` write. The writer emits a PCX-style header: manufacturer `0x0A`, version `0x05`, encoding `0x01`, bits per pixel `0x08`. It sets the color-plane count to `3` when the source surface reports format value `2`; otherwise it sets it to `1`.

Evidence: `0x007B05C0` decompile: header byte setup and `cStack_34F = (-(format != 2) & 0xFE) + 3`, yielding `3` for format `2` and `1` otherwise.

When using the truecolor/format-2 branch, the writer locks each source row, reads 16-bit packed pixels, and expands them into separate component row buffers using the same DD shift/loss globals before RLE-writing the rows.

Evidence: `0x007B05C0` truecolor branch: source row pointer from vtable `+0x5C`, width loop over vtable `+0x7C`, `ushort` source pixel read, component extraction through `g_DD_RShift/RLoss`, `g_DD_GShift/GLoss`, and `g_DD_BShift/BLoss`, then three row RLE calls.

Implementation consequence: `RandMap.img` requires PCX-style 3-plane direct RGB and must not be assumed to carry a trailing VGA palette or 1-plane indexed pixels.

**Rust status corrected 2026-07-22.** This paragraph previously claimed `src/assets/pcx_file.rs` "only supports 1-plane paletted PCX". That is no longer true: the reader accepts `planes == 3`, decodes plane-per-scanline direct RGB (`decode_direct_rgb_scanlines`), and tolerates a missing trailing palette. Only the *writer* was absent; `encode_direct_rgb` now emits the same form, round-trip tested against the reader.

## 5. Baked Start Markers

### 5.1 Inclusion and ordering

Active in YR: Yes when `GenerateTerrainPreview` runs and waypoints are valid. The baked marker loop runs after the terrain pixel loop and before the generated surface is unlocked/flushed. Therefore the markers are part of the source surface that `0x00595BC0 -> 0x007B05C0` later writes to `RandMap.img`.

Evidence: in `0x00641140`, terrain pixel iteration ends before `0x00641770`; marker loop runs `0x00641770..0x0064188E`; surface flush/unlock follows at `0x00641894..0x00641898`; dialog shutdown writer later writes `DAT_00ABE154+0`.

Implementation consequence: a faithful `RandMap.img` preview should include the baked red marker pixels if the native generated surface had valid waypoints. Rust should not draw these baked `4x4` red rectangles a second time on top of a decoded `RandMap.img`.

### 5.2 Marker constants

Active in YR: Yes. The marker loop tests exactly waypoint indices `0..7`, skips invalid entries without terminating, projects waypoint cell centers, and draws a `4x4` direct-color red rectangle at:

```text
marker_x = ((proj_x / 0x3C) - min_x) * 2 - 1
marker_y =  (proj_y / 0x1E) - min_y - 1
color    = packed RGB(0xF0, 0x00, 0x00)
size     = 4 x 4
```

Evidence: `0x00641770..0x0064188E`; previous marker reports resolve the waypoint predicate (`0x0068BD80`), waypoint copy (`0x0068BCC0`), and projection (`0x006D62E0`).

### 5.3 Clipping boundary for baked marker pixels

Active in YR: Yes when the marker rectangle straddles the generated preview surface. The marker is issued as a rectangle/fill against the generated preview `DSurface` after the surface dimensions are fixed. The code does not expand or reallocate the surface for markers, and `RandMap.img` writer only reads rows `0..height-1` and columns `0..width-1` from that surface.

Evidence: surface allocation occurs once at `0x00641260..0x00641295`; marker rectangle submission occurs later through the generated surface vtable `+0x78` and `+0x10` at `0x00641862..0x0064187A`; writer dimensions are queried from the surface vtable in `0x007B05C0`.

Observable contract:

- A marker wholly inside the generated preview contributes a `4x4` red block.
- A marker partially outside the generated preview can only contribute the intersection with the generated surface; outside pixels cannot appear in `RandMap.img` because the file dimensions and row reads are bounded by the already-allocated surface.
- A marker wholly outside the generated preview contributes no file pixels.

This report does not rename the generic `DSurface` rectangle-fill methods, but the image-level clipping boundary is strong: the source surface dimensions are fixed before markers, and the writer cannot serialize pixels outside those dimensions.

## 6. Active Random-Map UI Path

Active in YR: Conditional and standard for Create Random Map / random-map dialog preview.

1. Dialog command `0x620` disables controls, calls `FUN_00598960(1, hwnd)`, calls `GenerateTerrainPreview`, then re-enables controls and posts/requests paint. Evidence: `0x00596300`.
2. `FUN_00598960` calls `GenerateTerrainPreview` repeatedly when its preview flag is nonzero, followed by `SendMessageA(hwnd, WM_PAINT, 0, 0)`. Evidence: `0x00598960` multiple `(char)param_2 != 0` blocks.
3. Dialog paint checks `DAT_00ABE154`, child `0x468`, and the suppress gate, then calls `DrawStartPositions`. Evidence: `0x00596300` `WM_PAINT` branch.
4. Dialog shutdown `0x00595BC0` writes `RandMap.img` only if `DAT_00ABE154 != 0` and wrapper `+0 != 0`, then destroys the dialog preview wrapper and clears `DAT_00ABE154`.
5. Accepted setup / chooser random branches load `RandMap.img` into `DAT_00AC1154+0` through `0x00641DB0`, as resolved by the sibling loader report.

Implementation consequence: `RandMap.img` represents the last generated dialog preview surface at shutdown, not a passive row-browse preview and not launch terrain data.

## 7. Rust Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Acceptance scenario / proposed test |
|---|---|---|---|---|
| Generated preview dimensions are `(max_x-min_x)*2` by `(max_y-min_y)` from playable-cell projected bounds | `0x00641170..0x00641295` | no random generated-preview model | `src/map/preview.rs`, future random-map preview model | `skirmish_randmap_generated_preview_dimensions_follow_projected_playfield_bounds` |
| `RandMap.img` stores and reloads the generated surface dimensions, not UI-control dimensions | `0x00595BC0`, `0x007B05C0`, `0x00641DB0` | no `RandMap.img` branch | `src/app_skirmish_shell_render.rs`, `src/assets/pcx_file.rs` | `skirmish_randmap_img_preview_preserves_dynamic_dimensions` |
| Runtime image can be PCX-style 3-plane direct RGB when the source surface reports format `2` | `0x007B05C0` header/channel branch and row unpack loop | `PcxFile` supports only 1-plane paletted PCX | `src/assets/pcx_file.rs` or separate native IMG decoder | `skirmish_randmap_img_decodes_three_plane_direct_rgb_pcx` |
| Generated terrain preview includes baked `4x4` red start markers before `RandMap.img` write | `0x00641770..0x00641898`, writer after dialog shutdown | no random preview branch; overlay code is separate | `src/app_skirmish_shell_render.rs`, `src/map/preview.rs` | `skirmish_randmap_img_contains_baked_start_marker_pixels_without_overlay_duplication` |
| Baked markers are clipped only by generated source surface bounds; image dimensions do not expand | allocation `0x00641260..0x00641295`; marker draw `0x00641862..0x0064187A`; writer row bounds `0x007B05C0` | no generated marker image tests | future generated-preview renderer/fixture | `skirmish_randmap_baked_marker_partially_outside_surface_is_source_clipped` |
| Failed/no generated dialog preview should not synthesize `RandMap.img` | writer guard `0x00595C47..0x00595CAC` | random sentinel exists without image lifecycle | `src/ui/skirmish_shell/state.rs`, preview cache | `skirmish_randmap_dialog_writes_img_only_after_generated_preview_exists` |

## 8. Negative Facts / Do Not Do

- Do not hardcode `RandMap.img` dimensions to `80x50`, `138x75`, `144x112`, or the child `0x468` rectangle. Active in YR: No. Evidence: generated surface allocation formula and writer dimension queries.
- Do not treat `RandMap.img` as `[PreviewPack]`. Active in YR: No. Evidence: `RandMap.img` uses `0x007B05C0` / `0x00641DB0`, while normal map previews use the `[PreviewPack]` path.
- Do not assume random-map preview pixels are palette indices. Active in YR: No for the generated `DSurface` / writer path; colors are packed direct-color surface pixels and the writer has a 3-plane direct RGB branch.
- Do not draw live `STARTBUT.SHP` overlays as a substitute for baked generated-preview markers inside `RandMap.img`. Active in YR: No. Evidence: baked markers are written before `RandMap.img`; live overlays are a later paint layer in `DrawStartPositions`.
- Do not duplicate baked markers on top of a decoded `RandMap.img` unless implementing the separate live overlay layer for valid `[Header]` starts. Active in YR: No for the image source itself.
- Do not expand preview image dimensions to fit edge markers. Active in YR: No. Evidence: marker drawing occurs after fixed `DSurface` allocation; writer serializes only that surface's width/height.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes gate | verified | section 0 | none |
| Random-map dialog generate path | verified | `0x00596300` command `0x620` | none for liveness |
| Preview-enabled RMG refresh path | verified | `0x00598960` preview-flag branches | none for liveness |
| Bounds / dimension formula | verified | `0x00641170..0x00641295` | none |
| Terrain pixel color class | verified enough for UI image contract | `0x00641140` pixel pass | exact per-seed terrain colors deferred |
| PCX-style writer dimensions / channels | verified | `0x007B05C0` | exact runtime file sample not captured |
| Loader dimension preservation | verified | `0x00641DB0` | none |
| Baked marker inclusion | verified | `0x00641770..0x00641898`; writer after shutdown | none |
| Baked marker clipping boundary | verified at image/surface bounds level | fixed surface allocation; marker draw after allocation; writer bounded by surface dimensions | generic DSurface fill method name not recovered |
| Current Rust surface scan | verified | `rg` over preview/render/pcx/skirmish files | implementation not performed |

## 10. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is this path active in YR? -> Yes, conditionally through random-map dialog command `0x620`, preview-enabled `0x00598960`, and dialog shutdown writer.` Evidence: `0x00596300`, `0x00598960`, `0x00595BC0`.
- `[RESOLVED] OQ-2 - What dimensions does `GenerateTerrainPreview` allocate? -> `(max_projected_x - min_projected_x) * 2` by `max_projected_y - min_projected_y`, using playable-cell projected bounds.` Evidence: `0x00641170..0x00641295`.
- `[RESOLVED] OQ-3 - Are dimensions tied to the UI control? -> No. The UI control is only used later for paint/aspect fit.` Evidence: allocation has no child/control input; writer queries source surface dimensions.
- `[RESOLVED] OQ-4 - Does `RandMap.img` preserve those dimensions? -> Yes. Writer stores width/height from source vtable; loader constructs a destination surface with decoded width/height.` Evidence: `0x007B05C0`, `0x00641DB0`.
- `[RESOLVED] OQ-5 - Are terrain preview pixels palette indices? -> No for the generated surface contract; colors are packed through DD loss/shift globals, and the writer can emit direct RGB planes.` Evidence: `0x00641140`, `0x007B05C0`.
- `[RESOLVED] OQ-6 - Are baked start markers included in `RandMap.img`? -> Yes when valid waypoints exist, because the marker pass runs before surface flush and before dialog shutdown writes the surface.` Evidence: `0x00641770..0x00641898`, `0x00595BC0`.
- `[RESOLVED] OQ-7 - What marker size/color should tests expect? -> `4x4`, packed RGB `(0xF0,0,0)`, indices `0..7`, invalid entries skipped.` Evidence: `0x00641770..0x0064188E`.
- `[RESOLVED] OQ-8 - What clips baked marker pixels? -> The fixed generated source surface bounds. The marker draw does not resize the surface, and writer reads only that surface's dimensions.` Evidence: `0x00641260..0x00641295`, `0x00641862..0x0064187A`, `0x007B05C0`.
- `[DEFERRED] OQ-9 - Exact screenshot RGB for every generated terrain cell.` Category: out-of-scope. Reason: requires draining terrain/overlay color formula callees and runtime DD format values.
- `[DEFERRED] OQ-10 - Runtime sample bytes for a named generated seed's `RandMap.img`.` Category: needs-runtime-capture. Reason: static binary proves format/dimensions/channel behavior; no live generated file was captured in this slot.
- `[DEFERRED] OQ-11 - Concrete symbol names for DSurface rectangle-fill vtable methods.` Category: not needed for claimed output. Reason: surface-bound clipping contract is proven at the image serialization boundary.

## 11. Remaining Uncertainty

- Exact RGB values for arbitrary generated terrain pixels remain dependent on `CellClass__GetRadarPixelColor`, overlay type radar colors, `FUN_004BF650`, and current display pixel-format globals. This does not block `RandMap.img` rendering because the file should be decoded/rendered as image data.
- A runtime-captured `RandMap.img` for a fixed seed would be useful as a golden fixture, especially to lock down 3-plane RGB row ordering.
- The generic `DSurface` fill method names behind vtable `+0x78/+0x10/+0x88` were not recovered, but the generated-image clipping boundary is still output-determining and verified by fixed surface dimensions plus bounded writer reads.

## 12. Stale Docs / Follow-up Corrections

- `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md` OQ-5 replacement:
  > Baked generated-preview start markers are clipped at the generated preview source surface boundary. `GenerateTerrainPreview` allocates the `DSurface` before marker drawing, marker rectangles are submitted afterward, and `RandMap.img` serializes only that surface's fixed width/height. Exact generic DSurface method names remain separate, but the image cannot grow or wrap marker pixels outside the source surface.

- `SKIRMISH_RANDMAP_IMG_PREVIEW_LOADER_00641DB0_GHIDRA_REPORT.md` uncertainty replacement:
  > Generated preview dimensions are `(max_projected_x - min_projected_x) * 2` by `max_projected_y - min_projected_y` from playable-cell projected bounds. `RandMap.img` preserves those dimensions. The generated surface uses packed direct-color pixels and the writer has a 3-plane direct RGB PCX-style branch when the source surface format reports `2`; Rust's existing 1-plane paletted PCX decoder is not sufficient for this branch.

## 13. Sources

- Ghidra read-only decompile / assembly context: `GenerateTerrainPreview @ 0x00641140`, `0x00641170..0x00641295`, `0x00641770..0x00641898`, `FUN_00596300 @ 0x00596300`, `FUN_00598960 @ 0x00598960`, `FUN_00595BC0 @ 0x00595BC0`, `FUN_007B05C0 @ 0x007B05C0`, `CDFileClass__Constructor / RandMap.img loader @ 0x00641DB0`, `DSurface__Constructor @ 0x004BA5A0`, `DrawStartPositions @ 0x00640710`.
- Prior docs read for duplication/context: `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`, `SKIRMISH_GENERATE_TERRAIN_PREVIEW_BAKED_START_MARKERS_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_IMG_PREVIEW_LOADER_00641DB0_GHIDRA_REPORT.md`, `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `src/app_skirmish_shell_render.rs`, `src/map/preview.rs`, `src/assets/pcx_file.rs`, `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`.
