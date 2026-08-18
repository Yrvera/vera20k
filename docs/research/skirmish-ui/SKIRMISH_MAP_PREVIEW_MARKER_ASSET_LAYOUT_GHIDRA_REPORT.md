# Skirmish Map Preview Marker Asset Layout - Ghidra Report

**Date:** 2026-05-21  
**Address(es):** `0x00640710`, `0x006AE3F0`, `0x00775690`, `0x004A61C0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** live Skirmish/MP preview overlays drawn by `DrawStartPositions`: marker asset, marker and label placement, clipping footprint, and dialog `0x102` control-relative coordinates.  
**Non-Scope:** baked `[PreviewPack]` red marker generation, full preview object lifecycle, full PreviewPack decode, and assigned-player `mmpb.shp` marker logic.  
**Confidence:** High for binary call path, asset name, offsets, count gate, label rectangle, and baked-vs-live separation; Medium for asset dimensions because `18x18` comes from prior local asset probe rather than Ghidra.  
**Active in YR:** Conditional. The paint path is active in standard offline Skirmish when a preview object exists; live `STARTBUT.SHP` overlays require `0 < ScenarioClass+0x113C < 9`.

## 1. Overview

The live numbered start overlay is `STARTBUT.SHP`, frame `0`, drawn by `DrawStartPositions @ 0x00640710` after the preview image has been blitted into dialog child `0x468`. This is a separate layer from small baked red pixels already present inside generated or stored `[PreviewPack]` images.

Active in YR: Yes for the paint function and asset load. Evidence: offline Skirmish `WM_PAINT` at `0x006AE3F0` calls `DrawStartPositions`; string anchor report maps `STARTBUT.SHP @ 0x00836DE4` only to `DrawStartPositions`.

## 2. Dialog And Coordinate Anchor

| Item | Value | Active in YR | Evidence |
|---|---:|---|---|
| Skirmish dialog resource | `0x102` | Yes | `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md` |
| Map preview child | control `0x468` | Yes | `0x00640735..0x00640749` calls `GetDlgItem(hwnd, 0x468)` then `0x00775690` |
| Resource rect | `(429,23,96,69)` DLU | Yes | `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md` |
| Runtime coordinate base | child HWND screen rect converted to main client/backbuffer coordinates | Yes | `FUN_00775690 @ 0x00775690` subtracts `g_hWnd` client screen origin |

Control-relative rule:

```text
child_rect_px = HWND(0x468) window rect converted to main-client pixels
preview_fit_rect = aspect-fit preview surface inside child_rect_px
marker_anchor = preview_fit_rect.origin
              + trunc((HeaderWaypoint - HeaderStart) * 1000 / HeaderSize)
                scaled by preview_fit_rect.size / 1000
```

Active in YR: Yes. Evidence: `0x0064077B..0x00640887` fits/blits the preview; `0x006408F5..0x0064097E` projects each `ScenarioClass+0x1140/+0x1144` pair through `+0x112C..+0x1138`.

Important precision details:

- The fit uses integer `*1000` scale factors and truncating signed divisions, not floating point.
- The child DLU rect is not used directly by `DrawStartPositions`; Win32 has already converted it to a child HWND rectangle.
- The final marker coordinates are relative to the fitted preview image, not automatically the full `0x468` child if aspect-fit letterboxing is present.

Active in YR: Yes. Evidence: `0x00640799..0x00640852` uses `1000`-scale integer math before the preview blit.

## 3. Asset Identity And Separation

| Asset | Role | Active in YR | Evidence |
|---|---|---|---|
| `STARTBUT.SHP` | live numbered available-start overlay | Conditional: drawn only when count is `1..8` | string `0x00836DE4`; load at `0x006408A3..0x006408B2`; draw at `0x006409C9..0x006409D2` |
| `mmpb.shp` | separate assigned-player/house marker path | No for this `DrawStartPositions` slice | string `0x00836DF4` xrefs `FUN_00640A40`, not `0x00640710` |
| baked red `4x4` pixels | generated/stored preview image markers | Conditional: present in generated/saved previews and some stock PreviewPack data | `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md` |

Active in YR: Yes for the separation. Evidence: Ghidra string anchor reports: `STARTBUT.SHP` -> documented `DrawStartPositions`; `mmpb.shp` -> `FUN_00640A40`.

Prior local asset probe result:

- `STARTBUT.SHP`: `ra2md.mix -> localmd.mix`, 360 bytes, `18x18`, 1 frame.
- `mmpb.shp`: `ra2md.mix -> localmd.mix`, `12x12`, 1 frame, not the numbered overlay.

Active in YR: Yes for filenames and call sites; Medium confidence for dimensions. Evidence: `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`.

## 4. Overlay Gate And Draw Order

`DrawStartPositions` order:

1. Validate the dialog rect.
2. Get child `0x468` and convert it to main-client coordinates.
3. Query preview surface dimensions through the preview surface vtable.
4. Aspect-fit and blit the preview surface to `DAT_00887310`.
5. Lazy-load `STARTBUT.SHP`.
6. Read `ScenarioClass+0x113C`.
7. Draw overlays only when `0 < count < 9`.
8. For each marker, draw `STARTBUT.SHP` frame `0`, then draw numeric label `i + 1`.

Active in YR: Yes. Evidence: `0x00640721` validates; `0x00640735..0x00640749` gets child/rect; `0x00640860..0x00640887` blits; `0x0064088A..0x006408B2` lazy-loads; `0x006408D4..0x006408E5` count gate; `0x006409C9..0x00640A15` shape then text.

Loose Dustbowl caveat:

- The loose local `Dustbowl.map` has no `[Header]` and therefore leaves `ScenarioClass+0x113C = -1` on the verified selected-map preview path.
- For that path, `DrawStartPositions` does not draw live `STARTBUT.SHP`; any visible starts come from baked `[PreviewPack]` pixels.

Active in YR: Yes for the loose-map path. Evidence: `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`; guard at `0x006408D4..0x006408E5`.

## 5. Marker And Label Geometry

Let `anchor = (ax, ay)` be the projected start position in fitted-preview coordinates before marker art offsets.

| Output | Top-left | Footprint before generic clipping | Active in YR | Evidence |
|---|---:|---:|---|---|
| `STARTBUT.SHP` frame `0` | `(ax - 9, ay - 6)` | `18x18` from asset metadata | Conditional on count `1..8` and asset pointer non-null | `0x0064098B`, `0x00640999`, `0x006409C9..0x006409D2`; asset report |
| Numeric label | `(ax - 2, ay - 6)` | rectangle args include `8` and `0x19` | Conditional on count `1..8`; label is drawn even if SHP pointer is null | `0x006409DD..0x00640A15` |

Active in YR: Yes for offsets and call order. Evidence: marker draw pushes point `(EDI-9, EBX-6)` before `CC_Draw_Shape`; label draw mutates the unoffset anchor with `ADD EDI,-0x2`, `ADD EBX,-0x6`, increments marker index, then calls `FUN_004A61C0`.

Label numbering:

- Loop index starts at `0`.
- The label index is incremented before the text call, so labels are `1..count`.
- The text format pointer is `DAT_0081B3D0`; `FUN_004A61C0` receives the label number as a vararg and forwards to the font/text helper.

Active in YR: Yes. Evidence: `0x006408EB` zeroes `ESI`; `0x006409E3` increments `ESI`; `0x006409E4` pushes it; `0x00640A15` calls `FUN_004A61C0`.

## 6. Clipping Footprint

Shape clipping:

- The marker candidate footprint is the `18x18` frame at `(ax-9, ay-6)`.
- Before `CC_Draw_Shape`, `DrawStartPositions` asks `DAT_00887310` vtable `+0x78` for the active destination-surface clip/context and passes that to the shape draw path.
- Therefore live `STARTBUT.SHP` pixels are not clipped to the generated preview source or fitted preview image rectangle; they are clipped by the active destination surface. A marker whose native `18x18` footprint crosses the fitted preview edge can still draw the portion that overlaps the destination clip.

Active in YR: Yes. Evidence: `0x006409A7..0x006409B6` calls surface vtable `+0x78`; `0x006409C3..0x006409D2` passes that context to `CC_Draw_Shape`; follow-up `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md` verifies downstream clipping through `CC_Draw_Shape @ 0x004AED70` and `AlphaShapeClass__ClipRect @ 0x00421B60`.

Label clipping:

- The label candidate rectangle starts at `(ax-2, ay-6)` and uses the pushed extents `8` and `0x19`.
- It goes through the same destination surface family before `FUN_004A61C0` renders the number.
- Generic font/surface internals were not drained here, but follow-up marker research verifies the same destination-surface clipping boundary for the start marker family rather than fitted-preview clipping.

Active in YR: Yes for the rectangle and call path; deferred for per-glyph internals. Evidence: `0x006409E4..0x00640A15`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish paint reaches marker draw | verified | `0x006AE3F0` | none |
| Child `0x468` coordinate conversion | verified | `0x00640735..0x00640749`, `0x00775690` | exact DLU-to-pixel size depends on runtime font/window conversion |
| Preview fit and blit before overlays | verified | `0x0064077B..0x00640887` | none for marker layout |
| `STARTBUT.SHP` asset identity | verified | string `0x00836DE4`, load/draw in `0x00640710` | none |
| `STARTBUT.SHP` dimensions | touched-not-exhausted | prior asset probe in `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md` | fresh asset parser rerun if dimensions must be re-certified |
| Marker offset and footprint | verified | `0x0064098B`, `0x00640999`, `0x006409C9..0x006409D2` | none |
| Numeric label placement | verified | `0x006409DD..0x00640A15` | exact font glyph pixels not drained |
| Baked PreviewPack red pixels | verified separation only | `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md` | no PreviewPack decode redo in this slot |
| `mmpb.shp` sibling path | verified separation only | string `0x00836DF4`, xref `FUN_00640A40` | no assigned-player marker investigation here |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What concrete image path draws the live numbered start overlay? `STARTBUT.SHP`, loaded from the string at `0x00836DE4` and drawn in `DrawStartPositions`. Active in YR: Conditional on overlay count. Evidence: `0x006408A3..0x006408B2`, `0x006409C9..0x006409D2`.

[RESOLVED] OQ-2 - Are live overlays the same as baked red PreviewPack markers? No. Live overlays are `STARTBUT.SHP` plus text; baked markers are `4x4` red pixels from `GenerateTerrainPreview`. Active in YR: Yes as separate mechanisms. Evidence: this report plus `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`.

[RESOLVED] OQ-3 - Where does the numeric label draw relative to the marker anchor? Top-left `(ax-2, ay-6)`, after `INC ESI`, with pushed extents `8` and `0x19`. Active in YR: Conditional on overlay count. Evidence: `0x006409DD..0x00640A15`.

[RESOLVED] OQ-4 - Is `mmpb.shp` the numbered start marker? No. Its string anchors to `FUN_00640A40`, not `DrawStartPositions`. Active in YR: No for this numbered overlay slice. Evidence: string anchor reports for `STARTBUT` and `mmpb`.

[DEFERRED] OQ-5 - Exact per-glyph clipping of numeric text at preview edges. Category: out-of-scope. Reason: requires draining the generic font/surface draw internals beyond marker layout; current report verifies the call rectangle and destination context.

## Sources

- Ghidra decompiled/inspected: `DrawStartPositions @ 0x00640710`, `FUN_006AE3F0`, `FUN_00775690`, `FUN_004A61C0`, `FUN_006067A0`.
- Ghidra string anchor reports: `STARTBUT.SHP @ 0x00836DE4`, `mmpb.shp @ 0x00836DF4`.
- Prior docs: `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_MAP_PREVIEW_START_MARKERS_TRACE.md`, `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`, `SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`, `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_DAT_00AC1154_LIFECYCLE_GHIDRA_REPORT.md`.
