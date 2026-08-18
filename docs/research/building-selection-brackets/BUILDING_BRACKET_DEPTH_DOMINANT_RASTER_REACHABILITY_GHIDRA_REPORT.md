# Building Bracket Depth-Dominant Raster Reachability - Ghidra Report

**Report path:** `docs/research/building-selection-brackets/BUILDING_BRACKET_DEPTH_DOMINANT_RASTER_REACHABILITY_GHIDRA_REPORT.md`  
**Target:** building bracket depth-dominant raster reachability  
**Status:** COMPLETE  
**Active in YR:** Yes for the selected-building bracket path; No for depth-dominant raster reachability in normal stock building bracket lines.  
**Mode:** read-only Ghidra/live decompilation plus stock INI data enumeration. No Rust, INI, in-repo docs, or Ghidra state edits.

## Summary

No normal stock selected-building bracket line reaches `Surface::Draw_Line`'s depth-dominant raster loop.

The binary-selected building bracket topology emits axis-aligned box-edge stubs only:

- constant-Z foundation/roof stubs, which have depth delta `0` and are x-dominant after projection;
- vertical-Z stubs, which have screen-X delta `0` and exact screen-Y/depth tie because `DrawLine3D` subtracts `AdjustForZ(z)` from screen Y and also derives surface depth from the same adjusted Z.

`Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30` selects the depth-dominant loop only when depth delta is strictly greater than both screen-X and screen-Y deltas. The vertical selected-building stubs therefore fall through to the y-dominant loop, not the depth-dominant loop.

## Verified Binary Evidence

### 1. Surface branch is strict depth dominance

`Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30` computes:

```text
dx = nonnegative clipped screen-X delta
dy = abs(clipped screen-Y delta)
dz = abs(clipped endpoint depth delta)

if dx < dz && dy < dz:
    use depth-dominant loop
else if dy < dx:
    use x-dominant loop
else:
    use y-dominant loop
```

Evidence: read-only Ghidra decompile of `0x004BFD30`; the depth branch guard is `if ((iVar20 < iVar13) && (iVar21 < iVar13))`, followed by the x branch `else if (iVar21 < iVar20)` and final y branch. This is stricter than "depth ties win".

**Active in YR:** Yes. This is the primary surface vtable `+0x34` line drawer reached by `Tactical::DrawLine3D`.

### 2. DrawLine3D gives vertical bracket lines equal Y and depth deltas

`Tactical__DrawLine3D @ 0x006DBB60` projects each endpoint as:

```text
screen_x = trunc(WorldToScreenSub.x / 256) - Tactical.viewport_x
screen_y = trunc(WorldToScreenSub.y / 256) - AdjustForZ(z) - Tactical.viewport_y
depth    = 0xE - AdjustForZ(z)
```

Evidence: read-only Ghidra decompile of `0x006DBB60`; both endpoint screen-Y calculations call `Tactical__AdjustForZ()`, and the surface depth argument is derived from the same Z-adjusted family before the `g_PrimarySurface->vtable+0x34` call.

For vertical bracket edges, world X/Y are constant and only Z changes. Therefore `abs(screen_y_delta) == abs(depth_delta)` before raster branch selection. Because the surface depth branch requires `dy < dz`, vertical bracket stubs cannot be depth-dominant.

**Active in YR:** Yes. Building bracket helpers and direct stubs call `g_Tactical->vtable+0x60`, bound to this function.

### 3. DrawBracketCorner produces only 25% axis-aligned stubs

`TechnoClass__DrawBracketCorner @ 0x006F5EF0` computes quarter points:

```text
Q1 = trunc_to_zero((3*A + B) / 4)
Q2 = trunc_to_zero((A + 3*B) / 4)
```

Then it draws `A <-> Q1` and `Q2 <-> B`, ordering endpoints by Z. It does not create diagonal 3D lines outside the edge handed to it.

Evidence: read-only Ghidra decompile of `0x006F5EF0`; the three coordinate components are computed with `3*A+B`, signed bias, and `>> 2`, then passed to `g_Tactical->vtable+0x60`.

**Active in YR:** Yes. `DrawBehind @ 0x006F60D0` and `DrawExtras @ 0x006F5190` both call it for selected buildings.

### 4. Selected-building topology has no mixed horizontal+vertical bracket edges

For selected buildings, `DrawBehind @ 0x006F60D0` and `DrawExtras @ 0x006F5190` build an isometric rectangular box from:

```text
origin = object vtable +0x48 coordinate
half_x = Dimension2.x / 2
half_y = Dimension2.y / 2
z_top  = origin.z + Dimension2.z
```

The emitted building bracket submissions per building are:

| Segment class | Count per building | 3D delta | Surface branch |
|---|---:|---|---|
| Foundation/roof stubs | 14 | X or Y changes, Z constant | x-dominant |
| Vertical stubs | 7 | X/Y constant, Z changes | y-dominant by Y/depth tie |
| Depth-dominant stubs | 0 | none | unreachable |

Evidence: read-only Ghidra decompile of `0x006F60D0` confirms five `DrawBracketCorner` calls for back/left edges; `0x006F5190` confirms four more `DrawBracketCorner` calls and three direct `DrawLine3D` single-stub calls. The direct calls also use the same quarter-point construction by `CoordStruct__VecAdd` and `CoordStruct__VecDiv(..., 4)`.

**Active in YR:** Yes. The gates are selected byte `Techno+0x83 != 0`, `WhatAmI()==6`, and the standard object draw/extras path; no TS-only flag gates this bracket geometry.

### 5. Stock data does not introduce a depth-dominant segment

`BuildingTypeClass__Dimension2 @ 0x00464AF0` returns:

```text
out.x = g_FoundationWidthTable[Foundation] << 8
out.y = g_FoundationHeightTable[Foundation] << 8
out.z = Height * g_HeightFactor
```

Evidence: read-only Ghidra decompile of `0x00464AF0`.

Stock data enumeration of merged `rules.ini`/`rulesmd.ini` `[BuildingTypes]` plus merged `art.ini`/`artmd.ini` `Image=` sections found 403 stock building type entries. Realized stock dimensions cover:

```text
1x1, 1x2, 1x3, 1x4,
2x1, 2x2, 2x3, 2x5, 2x6,
3x1, 3x2, 3x3, 3x4, 3x5,
4x2, 4x3, 4x4,
5x3,
6x4
```

The tallest enumerated stock art height in this pass was `Height=22` on `CACHIG05`; the largest realized foundation width was `6x4`; the largest realized foundation height was `2x6`. These values change line lengths, but not the segment class. Constant-Z stubs still have `dz=0`; vertical stubs still have `dy==dz`.

**Active in YR:** Yes for the stock building data consumed by the active parser. No stock foundation/height combination can create a diagonal bracket line because the binary bracket topology never connects corners that differ in both foundation X/Y and Z.

## Inference

Because every normal building bracket segment is either constant-Z or vertical-Z, and because the surface depth loop requires strict `dz > dx && dz > dy`, all standard selected-building bracket lines raster through x-dominant or y-dominant loops. The depth-dominant loop remains real and active in `Surface::Draw_Line`, but not reachable from normal stock building selection brackets.

Viewport clipping does not change the classification for these segment classes: it can shorten a constant-Z or vertical line, but it cannot add Z delta to a constant-Z line, and a vertical line's projected relation remains `depth = screen_y + constant` under the `DrawLine3D` formula.

## Open Questions

- Does any non-building caller of `Tactical::DrawLine3D` in standard YR feed a line where `dz` is strictly greater than both screen axes? Out of scope for this building bracket slot.
- Are there pathological modded building art/parser cases with invalid `Foundation=` strings that visibly collapse bracket dimensions? Out of scope; stock data was sufficient for the reachability answer here.
- Exact pixel outcomes at clipped viewport borders still require the surface clipping report for endpoint inclusion, but not for the depth-dominant reachability classification.

## Sources

- Ghidra decompile: `Surface__DrawLine_ABufModulated_ZClipped @ 0x004BFD30`
- Ghidra decompile: `Tactical__DrawLine3D @ 0x006DBB60`
- Ghidra decompile: `TechnoClass__DrawBracketCorner @ 0x006F5EF0`
- Ghidra decompile: `TechnoClass__DrawBehind @ 0x006F60D0`
- Ghidra decompile: `TechnoClass__DrawExtras @ 0x006F5190`
- Ghidra decompile: `BuildingTypeClass__Dimension2 @ 0x00464AF0`
- Stock data read: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`
