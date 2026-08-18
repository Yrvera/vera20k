# Selection Brackets System — Ghidra Research Report

## Overview

Selection brackets in gamemd.exe are drawn differently for buildings vs mobile units.
Buildings use isometric 3D wireframe bracket lines. Units/infantry use SHP sprite bars.

**Confidence: HIGH** for the overall structure and edge topology. The exact coordinate
pairs for DrawExtras edges 3-4 and the 3 direct DrawLine3D calls were confirmed through
assembly tracing of the key math, though some intermediate stack variables were hard to
resolve due to decompiler aliasing.

## Building Selection Brackets

### Visual Result (verified from screenshot)

Buildings display **3 groups of 3 short lines** at the 3 visible corners of the
building's 3D isometric bounding box. Each group looks like a 3D coordinate axis
marker — 3 lines radiating outward along the 3 isometric directions. The groups
appear at the **roof level** of the building:

1. **Top** (back corner of roof, top of screen)
2. **Left** (left corner of roof)
3. **Right** (right corner of roof)

The 4th roof corner (front) and all ground-level bracket marks are hidden behind
the building sprite. Each line is approximately 25% of its edge length (~20-60px
depending on building size).

### 3D Box Edge Topology

A building's bounding box has **12 edges**. All 12 are drawn:
- 5 edges in `DrawBehind` (rendered behind the sprite)
- 4 edges via `DrawBracketCorner` in `DrawExtras` (rendered in front)
- 3 edges via direct `DrawLine3D` in `DrawExtras` (rendered in front)

Each edge has 25% stubs drawn at both endpoints. Stubs at hidden corners are
occluded by the building sprite.

### Edge List

**8 corners of the bounding box** (relative to building center at px, py, pz):
```
Ground: FL(px-hw,py+hh,pz)  FR(px+hw,py+hh,pz)  BL(px-hw,py-hh,pz)  BR(px+hw,py-hh,pz)
Roof:   FLr(px-hw,py+hh,pz+zh) FRr(px+hw,py+hh,pz+zh) BLr(px-hw,py-hh,pz+zh) BRr(px+hw,py-hh,pz+zh)
```
Where hw = foundation_width*128, hh = foundation_height*128, zh = Height*g_HeightFactor.

**DrawBehind (5 edges, drawn behind sprite):**

| # | From | To | Type |
|---|---|---|---|
| 1 | BL ground | BL roof | Back-left vertical |
| 2 | BR ground | BL ground | Back ground |
| 3 | BL ground | FL ground | Left ground |
| 4 | FL roof | BL roof | Left roof |
| 5 | BR roof | BL roof | Back roof |

**DrawExtras — DrawBracketCorner (4 edges, stubs at both ends):**

| # | From | To | Type |
|---|---|---|---|
| 6 | FL ground | FR ground | Front ground |
| 7 | BR ground | FR ground | Right ground |
| 8 | FL roof | FL ground | Front-left vertical |
| 9 | BR roof | BR ground | Back-right vertical |

**DrawExtras — direct DrawLine3D (3 single-stub edges, completing visible corners):**

All 3 edges converge at FR_roof (front-right roof), which is the hidden 4th corner.
Each draws a single stub (25%) at the VISIBLE end, using `(3*corner + FR_roof) / 4`
computed via VecAdd×3 + VecDiv(4). FR_roof is computed fresh in the else block as
`Set(px+hw, py+hh, pz+zh)`. The corner coordinates are reused from the
DrawBracketCorner edges: FL_roof from Edge 3 (`iStack_88/84/80`, not overwritten by
Edge 4), BR_roof from Edge 4 (`local_64/iStack_60/iStack_5c`).

| # | Stub at (visible) | Toward (hidden) | Type |
|---|---|---|---|
| 10 | FL roof | FR roof | **Front roof** stub at LEFT corner |
| 11 | BR roof | FR roof | **Right roof** stub at RIGHT corner |
| 12 | FR ground | FR roof | **FR vertical** stub (hidden behind sprite) |

### 3 Visible Corners

| Corner | Screen Position | 3 Edges Meeting |
|---|---|---|
| **BL roof** (TOP) | (-15, -53-zh_screen) | Back roof + Left roof + BL vertical |
| **FL roof** (LEFT) | (-105, -8-zh_screen) | Left roof + Front roof + FL vertical |
| **BR roof** (RIGHT) | (+105, +8-zh_screen) | Back roof + Right roof + BR vertical |

Screen positions shown for ConYard (4×4 foundation). The front-right roof corner
(FRr) is hidden behind the building sprite.

### Key Functions

| Address | Name | Purpose |
|---|---|---|
| `0x006f5ef0` | `DrawBracketCorner` | Draws 25% stubs at both ends of one edge |
| `0x006f5190` | `DrawExtras` | 4 DrawBracketCorner + 3 DrawLine3D for front/right edges |
| `0x006f60d0` | `DrawBehind` | 5 DrawBracketCorner for back/left edges |
| `0x006dbb60` | `DrawLine3D` | Projects 3D coords to screen, draws line via Surface::Draw_Line |
| `0x006d1eb0` | `WorldToScreen` | screen_x = 30*(wx-wy), screen_y = 15*(wx+wy) (sub-pixel) |
| `0x006ce240` | `VecAdd` | result = A + B (3D vector addition) |
| `0x00710700` | `VecDiv` | result = A / n (3D vector division) |
| `0x00464af0` | `Dimension2` | Returns {fw*256, fh*256, Height*HeightFactor} |

### DrawBracketCorner — Stub Calculation

```c
void DrawBracketCorner(CoordStruct *P1, CoordStruct *P2, color) {
    // Quarter-point near P1: (3*P1 + P2) / 4
    mid1 = (P1*3 + P2) / 4;
    // Quarter-point near P2: (P1 + 3*P2) / 4
    mid2 = (P1 + P2*3) / 4;
    // Draw short stub at P1 end (25% of edge)
    DrawLine3D(P1, mid1, color);
    // Draw short stub at P2 end (25% of edge)
    DrawLine3D(mid2, P2, color);
}
```

The direct `DrawLine3D` calls in `DrawExtras` use the same quarter-point formula
but computed manually via `VecAdd` chains and `VecDiv(vec, 4)`.

### Dimension Source

`BuildingTypeClass::Dimension2` at `0x00464af0`:
```c
result.x = FoundationWidthTable[foundation_id] << 8;   // width in leptons
result.y = FoundationHeightTable[foundation_id] << 8;   // height in leptons
result.z = Height * g_HeightFactor;                      // z extent in leptons
```

- Foundation tables at `0x008192b8` (width) and `0x00819310` (height):

| ID | W×H | ID | W×H | ID | W×H |
|---|---|---|---|---|---|
| 0 | 1×1 | 7 | 3×5 | 14 | 1×5 |
| 1 | 2×1 | 8 | 4×2 | 15 | 2×6 |
| 2 | 1×2 | 9 | 3×3 | 16 | 2×5 |
| 3 | 2×2 | 10 | 1×3 | 17 | 5×3 |
| 4 | 2×3 | 11 | 3×1 | **18** | **4×4** |
| 5 | 3×2 | 12 | 4×3 | 19 | 3×4 |
| 6 | 3×3 | 13 | 1×4 | 20 | 6×4 |
- `g_HeightFactor` at `0x0089ddb8` — computed at runtime from camera tilt angle
  via trigonometric lookup (sin table at `0x0085d0a4`, 4096 entries per full circle,
  conversion factor `4096/(2π)` at `0x007e8970`). The computation chain:
  ```
  _0089c8c0 = (π/180) * 60.0     (60° in radians)
  _0089c898 = (π/180) * 90.0     (90° in radians)
  _0089c8b8 = sqrt(2 * atan2(256.0, 2.0))
  g_HeightFactor = ftol(sin(30°) * _0089c8b8 * 0.5)
  ```
- `Height=` is read from art.ini (via Image= redirect), default 2
- **Practical equivalence: 1 Height unit = 1 terrain height level = 15 screen pixels**
  This is verified by: `g_HeightFactor * g_AdjustForZ_Mult ≈ 15.0`

### AdjustForZ — Z to Screen Y

`AdjustForZ` at `0x006d20e0` converts Z leptons to screen pixels:
```c
int AdjustForZ(int z_leptons) {
    int round_adj = (z_leptons >= 728) ? 1 : 0;    // rounding for tall objects
    return ftol(z_leptons * g_AdjustForZ_Mult + round_adj + 0.5);
}
```

`g_AdjustForZ_Mult` at `0x00b0cd48` is computed in `FUN_006d1ba8`:
```c
g_AdjustForZ_Mult = cos(camera_angle) * 60.0 / viewport_scale;
```

### Screen Projection

`WorldToScreen` at `0x006d1eb0`:
```c
screen_x_sub = 30 * (world_x - world_y);   // sub-pixel (>>8 for pixels)
screen_y_sub = 15 * (world_x + world_y);   // sub-pixel (>>8 for pixels)
```
Then `DrawLine3D` applies: `screen_y -= AdjustForZ(z)` for height offset.

For ConYard GACNST (**4×4 foundation**, Height=4, hw=hh=512 leptons):
- Diamond width on screen: 240px (symmetric, 4 cells × 60px)
- Diamond height on screen: 120px (symmetric, 4 cells × 30px)
- Z offset: 4 × 15 = 60px upward
- Ground stub length: ~30px (25% of 120px edge)
- Vertical stub length: ~15px (25% of 60px Z extent)
- Roof stub length: ~30px (25% of 120px roof edge)

### Line Rendering

- **Width**: 1 pixel (standard `Surface::Draw_Line` at `vtable+0x34`)
- **Color**: palette entry `0x0F` (white) for normal buildings; palette
  entry `0x0C` only when `ObjectClass::GetHeight` (`vtable+0x1C8`,
  `0x005F5F40`) returns `< -4`.
  `GetHeight` computes above-ground height:
  `Location.Z - CellClass::GetGroundHeight(Location) - (OnBridge ? BridgeHeight : 0)`.
  Ordinary placed buildings return `0`, so they draw white. The dim-color path is
  a real binary branch, but it requires a negative-height state such as an
  inconsistent bridge/height condition; it is not driven by
  `PixelSelectionBracketDelta`.
- The palette is at `g_PaletteData` (`0x0087f6c4`), color conversion table at
  offset `+0x174`. Supports both 8-bit and 16-bit surface formats.
- `DrawLine3D` passes `0xE - AdjustForZ(z)` as a 5th parameter to `Surface::Draw_Line`,
  controlling depth ordering for overlapping lines.

### Z-Sorting in DrawBracketCorner

Each DrawBracketCorner call draws two line segments. Before drawing each line,
the Z values of the two endpoints are compared:
- If `P.z > quarterPoint.z`: draw from quarterPoint to P (lower Z first)
- If `P.z <= quarterPoint.z`: draw from P to quarterPoint

This ensures correct painter's-algorithm depth ordering when bracket edges cross
different Z levels (e.g., vertical edges going from ground to roof).

### Building Anchor Position (GetCoords at vtable+0x48)

`BuildingClass::GetCoords` at `0x00447AC0` computes the **foundation center**:
```c
center_x = raw_x + (foundation_w - 1) * 128;   // shift to center
center_y = raw_y + (foundation_h - 1) * 128;
center_z = raw_z;
```
For a 4×4 building, the center is shifted +384 leptons (1.5 cells) from the
raw corner position in both X and Y. **Brackets are centered on this point.**
The bib (if any) is NOT included — `GetFoundationHeight(0)` passes `param_2=0`.

### PixelSelectionBracketDelta

INI key at TechnoTypeClass offset `0x3E0`.
- For **buildings**: not used for line-bracket geometry or line color. Building
  line color uses `ObjectClass::GetHeight` through `vtable+0x1C8`.
- For **units/infantry/vehicles/aircraft**: read directly by
  `TechnoClass::DrawHealthBar` (`0x006F64A0`) to shift the PIPBRD/pip Y
  position vertically.

## Unit/Infantry Selection — SHP Sprites (TechnoClass__DrawHealthBar at 0x006F64A0)

Units and infantry do NOT use line-drawn brackets. They use `PIPBRD.SHP` sprites
as selection background + `PIPS.SHP` for health indicators.

### Vehicles (RTTI != 6 and != 0xF)
```c
if (selected) {
    CC_Draw_Shape(PIPBRD.SHP, frame=0, x+1, y + PixelSelectionBracketDelta - 26);
}
// Health pips: max 17 slots, 2px spacing
for (i = 0; i < health_pip_count; i++) {
    CC_Draw_Shape(PIPS.SHP, pip_frame, x - 15 + i*2, y + delta - 25);
}
```

### Infantry (RTTI == 0xF)
```c
if (selected) {
    CC_Draw_Shape(PIPBRD.SHP, frame=1, x+11, y + PixelSelectionBracketDelta - 25);
}
// Health pips: max 8 slots, 2px spacing
for (i = 0; i < health_pip_count; i++) {
    CC_Draw_Shape(PIPS.SHP, pip_frame, x - 5 + i*2, y + delta - 24);
}
```

### Buildings (RTTI == 6)

Buildings do NOT use PIPBRD.SHP. Health pips are drawn in an **isometric diagonal
arrangement** along the left foundation edge:

1. Get foundation dimensions via `Dimension2` → `{w*256, h*256, Height*HF}`
2. Compute half-dims: `half = (w*128, h*128, Height*HF/2)`
3. Project 3 offset points through `CoordsToClient`:
   - `(-half_w, +half_h, zh)` → left corner with height
   - `(-half_w, -half_h, zh)` → back corner with height
   - `(-half_w, +half_h, 0)` → left corner ground
4. Pip count = `(screen_y_left - screen_y_back) / 2` = half the edge height
5. Health ratio determines filled vs empty pips
6. Each pip is drawn with **(-4, +2) pixel spacing** — following the exact isometric
   tile edge slope (60:30 = 4:2 per pip)

```c
// Filled pips (green=1, yellow=2, red=4)
for (i = 0; i < filled; i++) {
    x = cam_x + start_x + 3 + total*4 - i*4;
    y = cam_y + start_y + (total*-2 + 4) - i*(-2);
    CC_Draw_Shape(PIPS.SHP, pip_frame, x, y, ...);
}
// Empty pips (frame 0)
for (i = filled; i < total; i++) {
    CC_Draw_Shape(PIPS.SHP, 0, x, y, ...);  // same spacing
}
```

The pips run along the **left edge** of the foundation diamond, from the
back-left (top of screen) toward the front-left.

### SHP Globals

| Address | Name |
|---|---|
| `0x00AC1478` | `PIPBRD.SHP` pointer (frame 0=vehicle, frame 1=infantry) |
| `0x00AC147C` | `PIPS.SHP` pointer (health pip sprites) |
| `0x00AC1480` | `PIPS2.SHP` pointer (secondary pips) |
| `0x00AC1484` | `TALKBUBL.SHP` pointer (speech bubble) |

### PIPS.SHP Frame Map

| Frame | Use |
|---|---|
| 0 | Empty pip (building unfilled) |
| 1 | Green building pip |
| 2 | Yellow building pip |
| 4 | Red building pip |
| 16 (0x10) | Green unit/infantry pip |
| 17 (0x11) | Yellow unit/infantry pip |
| 18 (0x12) | Red unit/infantry pip |

### Health Thresholds (from rules.ini)

- `ConditionYellow` at RulesClass+0x1700 (double) — below this ratio → yellow pips
- `ConditionRed` at RulesClass+0x1708 (double) — below this ratio → red pips

## Guard Conditions

Brackets only draw when ALL of these are true:
1. `field_0x3CD == 0` — object is NOT in limbo/warp state (DrawExtras entry guard)
2. `field_0x83 != 0` — object IS selected (the `IsSelected` byte)
3. RTTI == 6 — object is a building (bracket lines only for buildings)
4. RTTI != 0xF — not infantry (buildings are never infantry, but this is checked)

If any fails, no bracket lines are drawn. Health pips (vtable+0x448) have a
separate check: `field_0x1B > 0` (health > 0) AND (allied OR `ShowEnemyPips`
rule at RulesClass+0x17E6).

## Single-Stub Drawer (FUN_006f6030 at 0x006F6030)

Used by the infantry bracket path. Draws ONE stub (25%) at the P1 end:
```c
void DrawSingleStub(CoordStruct *P1, CoordStruct *P2, int color) {
    quarter = (3*P1 + P2) / 4;
    if (quarter.z < P1.z)
        DrawLine3D(P1, quarter, color, 0);
    else
        DrawLine3D(quarter, P1, color, 0);
}
```

The building else block does the same computation manually via VecAdd×3 + VecDiv(4)
for the 3 direct DrawLine3D calls. The formula `(3*P1 + P2) / 4` is equivalent to
`P1 + (P2-P1)/4` — a point 25% from P1 toward P2.

## Render Pipeline — Draw Phase Ordering

The engine uses a **multi-phase render dispatch** at `0x005B2950`. Each object is drawn
in ~20 sequential phases via a switch/case that calls different vtable slots. The
bracket-related phases are:

| Phase | vtable | Function | Purpose |
|-------|--------|----------|---------|
| ... | ... | (earlier phases) | Shadow, turret, overlays, main sprite |
| **0x12** | **+0x244** | **DrawBehind** | Back bracket edges (behind sprite) |
| **0x13** | **+0x248** | **DrawExtras** | Front bracket edges + health pips (in front) |
| 0x14 | +0x24C | ? | Post-draw effects |
| ... | ... | (later phases) | Additional overlays |

**Updated 2026-05-21 interleaving note:** `BUILDING_BRACKET_MULTI_OBJECT_INTERLEAVING_GHIDRA_REPORT.md`
refines this simplified phase wording. In the standard non-foot Techno branch, the
first tactical object pass calls `DrawBehind`, then `DrawExtras`, then `vtable+0x104`
with flag `1`. After all first-pass display layers finish, a later second pass calls
`DrawExtras` again for visible techno objects. The final front bracket submission is
therefore effectively phase-batched by the second `DrawExtras` pass, not just a simple
per-object `DrawBehind -> body -> DrawExtras` sequence.

The player-visible model remains: back/left edges are intended to be occluded by
building artwork, while the later `DrawExtras` front/right edges and health pips draw
in front. Multi-building overlap cases depend on the refined two-call `DrawExtras`
ordering above.

## Current Rust Status

As of the 2026-05-20 bracket swarm, Rust has a dormant bracket builder in
`src/app_selection_brackets.rs`, but visible building bracket generation is
disabled in `src/app_render/build_instances.rs`: the call to
`build_selection_bracket_instances` is commented out and the bracket instance
vector is forced to `Vec::new()`. Therefore current Rust does not render these
building line brackets even though much of the topology code exists.

## Key Addresses Summary

| Address | Name |
|---|---|
| `0x006f5190` | `TechnoClass::DrawExtras` — main bracket + pip drawing |
| `0x006f60d0` | `TechnoClass::DrawBehind` — back bracket edges |
| `0x006f5ef0` | `TechnoClass::DrawBracketCorner` — one edge with 25% stubs |
| `0x006f64a0` | `TechnoClass::DrawHealthBar` (vtable+0x44C) — pips for all types |
| `0x006dbb60` | `Tactical::DrawLine3D` (vtable+0x60) — 3D projected line |
| `0x006d1eb0` | World-to-screen projection |
| `0x006ce240` | Vector addition |
| `0x00710700` | Vector division |
| `0x00464af0` | `BuildingTypeClass::Dimension2` — foundation + height dims |
| `0x0089ddb8` | `g_HeightFactor` — runtime camera-angle height scaling |
| `0x00AC147C` | `pips.shp` pointer — health pip sprites |
| `0x00AC1478` | `pipbrd.shp` pointer — pip bar background sprites |
