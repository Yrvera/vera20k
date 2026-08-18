# Foundation Center — Open Investigation

## Problem

gamemd.exe positions building sprites at the **foundation center** via `GetCoords`
(vtable+0x48), but our engine positions them at the **NW cell center**. When we tried
adding the foundation center offset to sprites, buildings looked wrong (shifted too
far south). When we removed it, buildings look correct on tiles but health bars are
~5px too high.

The health bar formula `sy - 11 - Height*15` is algebraically exact from the gamemd
decompilation, but we use `-6` instead of `-11` as an empirical correction for the
building sprite anchor mismatch.

## What gamemd does (verified via Ghidra)

### Building sprite rendering
- Tactical loop (0x6D8DB0) calls `GetCoords` (vtable+0x48) for screen position
- `BuildingClass::GetCoords` (0x447AC0) returns **foundation center**:
  ```c
  result.X = Location.X + (foundWidth - 1) * 128;
  result.Y = Location.Y + (foundHeight - 1) * 128;
  result.Z = Location.Z;
  ```
- CoordsToClient projects this to screen, then DrawSHP applies `-canvas_h/2 + frame_y`
- The SHP canvas is designed for this anchor point

### Building health bar rendering
- DrawExtras (0x6F5190) also uses `GetCoords` for bracket coordinates
- DrawHealthBar (0x6F64A0) receives pLocation from the rendering chain
- Dimension2 offsets + foundation center cancel, giving: `pip0.Y = pLoc.Y - 11 - H*15`

### Foundation center offset in screen pixels
```
fc_dx = 15 * (foundWidth - foundHeight)    // 0 for square foundations
fc_dy = 7.5 * (foundWidth + foundHeight) - 15
```

| Foundation | fc_dy |
|------------|-------|
| 1x1 | 0 |
| 2x2 | 15 |
| 3x2 | 22.5 |
| 3x3 | 30 |
| 4x3 | 37.5 |
| 4x4 | 45 |

## What our engine does

- Building sprite: `final_y = sy + offset_y` (NW cell center, no foundation offset)
- Health bar: `start_y = sy - 6 - Height*15` (-6 instead of gamemd's -11)
- The -6 is an empirical correction: gamemd's -11 assumes foundation center anchor,
  but our sprites use NW cell center, so we compensate with +5

## The mystery

When we add fc_dy to building sprites (matching gamemd's foundation center):
- Buildings shift too far south and look WRONG on tiles
- Larger buildings (4x4) shift more than smaller ones (2x2)
- The user confirmed buildings look correct WITHOUT the offset

This contradicts the Ghidra analysis showing gamemd uses foundation center. Possible
explanations:

1. **SHP canvas design assumption**: Our SHP compositing produces a canvas where the
   building image is positioned for NW cell center anchoring, not foundation center.
   Maybe gamemd's DrawSHP handles frame offsets differently than our compositing.

2. **DrawBody (198, 446) anchor system**: gamemd's BuildingClass::DrawBody uses anchor
   constants (198, 446) with CellToPixel subtraction. These are for clipping bounds,
   NOT draw position (verified). But maybe they interact with the rendering in ways
   we don't replicate.

3. **Band splitter (FUN_0043D030)**: gamemd splits building rendering into two depth
   bands. This could shift the effective draw position through clipping rect
   interactions.

4. **The tactical loop position may differ from GetCoords**: The general object loop
   at 0x6D8DB0 uses GetCoords for SOME objects but might use a different function
   for buildings specifically. The building-specific pass at layer 2 uses
   GetRenderCoords (vtable+0xAC = raw Location = NW cell center).

## What needs further investigation

- [ ] Trace the EXACT function call chain from tactical loop → DrawIt → DrawBody →
      DrawSHP for a specific building, logging the Y value at each step
- [ ] Compare our SHP frame compositing (blit at fx,fy into canvas, then -canvas_h/2)
      against gamemd's DrawSHP (centering then +frame_y) for a specific building SHP
      with known frame data
- [ ] Check if the band splitter modifies the effective draw position
- [ ] Verify whether DrawIt receives GetCoords or GetRenderCoords position for buildings
- [ ] Run gamemd.exe and screenshot a building at known cell coordinates, then compute
      the expected pixel position and compare

## Current state

The -6 constant (instead of algebraically correct -11) works well visually. The 5px
difference is consistent across all buildings and is a minor artifact of the sprite
anchor mismatch. This is acceptable for now but should be resolved when the
foundation center rendering is properly understood.

## Relevant code

- `src/app_ui_overlays.rs` — Health bar formula: `sy - 6 - H*15`
- `src/app_instances/shp.rs` — Building sprite: `sy + offset_y` (no fc offset)
- `src/render/selection_overlay.rs` — Ghost placement (no fc offset)
- `src/app_instances/overlays.rs` — Fog snapshots (no fc offset)

## Relevant gamemd addresses

| Address | Function |
|---------|----------|
| 0x447AC0 | BuildingClass::GetCoords (foundation center) |
| 0x41BE00 | ObjectClass::GetRenderCoords (raw Location) |
| 0x43D290 | BuildingClass::DrawBody |
| 0x43D030 | Band splitter (splits building into 2 depth bands) |
| 0x705E00 | TechnoClass::DrawSHP |
| 0x4AED70 | Low-level DrawSHP (canvas centering) |
| 0x6D8DB0 | Tactical object rendering loop |
| 0x6F5190 | DrawExtras (calls DrawHealthBar) |
| 0x6F64A0 | DrawHealthBar |
